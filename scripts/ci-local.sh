#!/usr/bin/env bash
# Rejoue EN LOCAL les vérifications du CI (.github/workflows/ci.yml) avant de pousser.
#
# Pourquoi ce script existe : le CI est resté rouge en continu pendant plus d'une semaine sur un
# seul `cargo fmt --check` (crates/overlay-engine/src/session.rs), parce que rien ne le signalait
# avant le push — chaque commit suivant repartait rouge sans rapport avec son contenu. Le lancer
# avant `git push` (ou laisser le hook `pre-push` le faire, voir `scripts/install-hooks.sh`)
# transforme cet aller-retour de plusieurs minutes en une vérification locale.
#
# Complément indispensable : `rust-toolchain.toml` épingle la version de Rust, donc le `rustfmt` et
# le `clippy` employés ici sont EXACTEMENT ceux du CI. Sans cet épinglage, un verdict local vert ne
# garantirait rien.
#
# ⚠ SOURCE DE VÉRITÉ PARTAGÉE : les commandes ci-dessous doublent celles de
# `.github/workflows/ci.yml`. Toute étape ajoutée/retirée là-bas doit l'être ici aussi.
#
# UNE différence assumée, et une seule : le CI exécute ses tests dans un CONTENEUR ÉPINGLÉ au rendu
# figé (`container:` du job `test-linux` + `scripts/setup-render-env.sh`), ce script tourne sur la
# machine du dev. Tout le reste est identique ; pour les captures, voir `etape_captures_locale` et
# le bloc « Seuil de bruit de rastérisation » plus bas.
#
# Usage :
#   bash scripts/ci-local.sh                       # tout : format, clippy, tests
#   bash scripts/ci-local.sh --lint                # format + clippy seulement (rapide)
#   bash scripts/ci-local.sh --captures-conteneur  # captures dans l'environnement de rendu du CI
#                                                  # (Linux + Docker ; refusé sous MSYS, voir plus bas)
#
# Pour RÉGÉNÉRER des références sans Docker — le cas d'un poste Windows comme d'une session cloud :
# GitHub → Actions → « Régénérer les captures (rendu du CI) » (`.github/workflows/regen-captures.yml`).
#
# Le script ne s'arrête PAS à la première erreur : il déroule tout et récapitule à la fin, pour
# corriger l'ensemble en une passe plutôt qu'un aller-retour par étape. `--no-fail-fast` étend ce
# principe À L'INTÉRIEUR d'une étape : sans lui, `cargo test` s'arrête au premier BINAIRE de test en
# échec et n'exécute pas les suivants — c'est ainsi que deux captures périmées de
# `tests/design_gallery.rs` (qui passe avant `tests/panels.rs` dans l'ordre alphabétique) en ont
# caché vingt-et-une de `tests/panels.rs` pendant une semaine, sans qu'aucun journal n'en montre la
# moindre ligne.
set -uo pipefail
# Chemin ABSOLU capturé AVANT le `cd` : après lui, `dirname "$0"` ne désigne plus le dossier des
# scripts (`bash ci-local.sh` depuis `scripts/` donnerait `.`, c'est-à-dire la racine du dépôt).
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

# Sur SteamOS, ni l'hôte ni un bac à sable Flatpak ne savent compiler : on se relance d'abord dans
# le conteneur de développement (voir scripts/dev-env.sh). Sans cela, chaque étape échouerait sur
# un « cargo: commande introuvable » que le hook pre-push présenterait comme un écart de lints.
. "$SCRIPT_DIR/dev-env.sh"
dev_env_reexec "$SCRIPT_DIR/ci-local.sh" "$@"

LINT_ONLY=0
CAPTURES_CONTENEUR=0
case "${1:-}" in
  --lint) LINT_ONLY=1 ;;
  --captures-conteneur) CAPTURES_CONTENEUR=1 ;;
  '') ;;
  *) echo "usage : bash scripts/ci-local.sh [--lint | --captures-conteneur]" >&2; exit 2 ;;
esac

# Windows (Git Bash / MSYS) ne compile pas les mêmes cibles que Linux : `overlay-ui::main` importe
# `windows::` sans `cfg` (Windows uniquement), et `wakfu-companion-overlay-x11` vise X11 (Linux
# uniquement). Même découpage que le CI, qui a un job par plateforme pour cette raison.
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) PLATFORM=windows ;;
  *)                    PLATFORM=linux ;;
esac

# `--captures-conteneur` sous MSYS : REFUSÉ, bruyamment. Jusqu'au 2026-09-18 l'option était
# silencieusement IGNORÉE ici — le bloc qui l'honore vivait à l'intérieur d'un `if PLATFORM = linux`,
# si bien que `UPDATE_SNAPSHOTS=1 bash scripts/ci-local.sh --captures-conteneur` lançait un
# `cargo build --workspace`, affichait « Tout est vert » et ne régénérait pas une seule référence.
# C'est la commande que la doc du Dockerfile recommande, et c'est la seule voie de régénération que
# le dépôt documentait : sur un poste Windows, elle ne pouvait pas marcher.
#
# Le montage « même chemin absolu » dont dépend ce mode (empreintes cargo, réutilisation de
# `target/`) n'a pas d'équivalent Docker Desktop : l'hôte est en `D:\…`, le conteneur en `/…`. Plutôt
# qu'un second chemin de code non testé, on renvoie vers le workflow, qui régénère dans l'
# environnement du CI lui-même.
if [ "$CAPTURES_CONTENEUR" -eq 1 ] && [ "$PLATFORM" = windows ]; then
  cat >&2 <<'MSG'
--captures-conteneur n'est pas disponible sous Windows (Git Bash/MSYS).

Ce mode monte le dépôt au MÊME chemin absolu que l'hôte pour réutiliser target/ ; un chemin
Windows (D:\...) n'a pas d'équivalent dans un conteneur Linux.

Pour régénérer les références sous le rendu du CI :
  GitHub → Actions → « Régénérer les captures (rendu du CI) » → Run workflow (sur dev).
  Le workflow réécrit les références, publie l'avant/diff/après en artefact, et pousse.

Pour un verdict local indicatif, lancer ce script sans option : les captures y sont informatives,
et les écarts trop grands pour du bruit de rastérisation y sont signalés.
MSG
  exit 2
fi

FAILED=()
step() {
  local label="$1"; shift
  printf '\n\033[1m▶ %s\033[0m\n' "$label"
  if "$@"; then
    printf '\033[32m✓ %s\033[0m\n' "$label"
  else
    printf '\033[31m✗ %s\033[0m\n' "$label"
    FAILED+=("$label")
  fi
}

# `[patch.crates-io]` du Cargo.toml racine : sans ce dossier, toute commande cargo touchant
# `overlay-ui` échoue sur un chemin introuvable. Regénéré seulement s'il manque (le script amont
# retélécharge le crate depuis crates.io à chaque appel).
if [ ! -d vendor/wgpu-hal-30.0.1 ]; then
  printf '\n\033[1m▶ vendor wgpu-hal patché (absent, reconstruction)\033[0m\n'
  bash patches/setup-vendor.sh || { echo "échec de patches/setup-vendor.sh" >&2; exit 1; }
fi

# Purement textuel : aucune compilation, quelques millisecondes, et vrai partout. Double le job
# `fmt` du CI. Voir l'en-tête du script appelé.
step "digest de l'image de rendu"     bash "$SCRIPT_DIR/verifier-digest-rendu.sh"

# Même nature : purement textuel, quelques millisecondes. Double l'étape homonyme du CI, et le hook
# `pre-commit` — le dépôt est public, une fixture `wakfu.log` brute y publierait un jeton de
# session, une IP, un nom de compte Windows et les messages de tous les joueurs croisés ce jour-là.
step "fixtures sans données réelles"  bash "$SCRIPT_DIR/check-fixtures.sh"

step "fmt — workspace"                cargo fmt --all -- --check
step "fmt — xtask"                    cargo fmt --manifest-path xtask/Cargo.toml -- --check

step "clippy — crates métier"         cargo clippy -p overlay-engine -p overlay-ingest -p overlay-sync -p overlay-platform --all-targets -- -D warnings
if [ "$PLATFORM" = linux ]; then
  step "clippy — overlay-ui (lib + x11)" cargo clippy -p overlay-ui --lib --bin wakfu-companion-overlay-x11 -- -D warnings
  step "clippy — overlay-testkit"        cargo clippy -p overlay-testkit --all-targets -- -D warnings
else
  # **La moitié Windows d'`overlay-ui` n'était vérifiée par AUCUN lint local** (corrigé le
  # 2026-09-18). `--lint` ne compilait ici que les crates métier : ni la lib (dont tout le
  # `cfg(windows)` : `turn_watch/capture.rs`, `turn_watch/notify.rs`), ni surtout `main.rs`, les
  # 4 000 lignes du binaire livré. Le hook `pre-push` appelant ce mode, un `main.rs` qui ne compile
  # plus passait le push sans un mot — vécu du 2026-09-17 : trois runs `build-windows` rouges
  # d'affilée sur deux champs déclarés dans `App` au lieu d'`AppState` (commits ff133d0/669f934,
  # réparés par 1af5aec). Le CI le voyait ; rien avant lui.
  #
  # Même portée que le CI, où le job `build-windows` lance désormais la MÊME commande.
  step "clippy — overlay-ui (lib + binaire Windows)" cargo clippy -p overlay-ui --lib --bin wakfu-companion-overlay -- -D warnings
fi
step "clippy — xtask"                 cargo clippy --manifest-path xtask/Cargo.toml --all-targets -- -D warnings

# ── Captures : trois situations, trois comportements ────────────────────────────────────────────
#
# Les captures d'`overlay-testkit` sont un GATE du CI depuis le 2026-09-13 (§17.1 du plan), et le
# CI les compare sous un rendu FIGÉ : conteneur épinglé par digest + Mesa épinglé par
# `scripts/setup-render-env.sh`. Ce script-ci tourne sur la machine du dev, qui n'a aucune raison
# d'avoir le même Mesa — le conteneur de développement du Steam Deck est sous Arch, en Mesa roulant,
# et sans `vulkan-swrast` il ne rendrait même pas avec le même pilote.
#
# Un verdict rendu là-dessus ne vaut rien, et un « tout est vert » qui rougit à tort est pire que
# pas de verdict du tout : c'est exactement ce qui a fait que plus personne ne lisait le CI pendant
# la semaine rouge de septembre. D'où :
#
#   environnement prouvé épinglé  → étape BLOQUANTE, comme au CI
#   environnement quelconque      → étape INFORMATIVE, jamais comptée en échec
#   `--captures-conteneur`        → on se place dans l'environnement du CI, donc BLOQUANTE
#
# La preuve est le marqueur posé par `setup-render-env.sh` en fin de course (voir sa doc) : sa
# présence signifie que ce script est allé au bout sur cette machine. Jamais un `dpkg-query` refait
# ici — `dpkg` n'existe pas sous Arch, l'interrogation y échouerait sans bruit et ferait passer un
# environnement non épinglé pour épinglé.
MARQUEUR_RENDU="/etc/wakfu-render-pinned"

# Posé par `etape_captures_locale` quand elle rend un verdict INFORMATIF : le récapitulatif final
# doit alors dire ce qu'il n'a pas vérifié, plutôt que de promettre un CI vert qu'il ne sait pas
# prédire.
CAPTURES_NON_VERIFIEES=0

rendu_est_epingle() {
  local attendu marque
  [ -r "$MARQUEUR_RENDU" ] || return 1
  attendu="$(sed -n 's/^MESA_ATTENDU="\(.*\)"$/\1/p' "$SCRIPT_DIR/setup-render-env.sh")"
  marque="$(sed -n 's/^mesa-vulkan-drivers=//p' "$MARQUEUR_RENDU")"
  [ -n "$attendu" ] && [ "$attendu" = "$marque" ]
}

# ── Seuil de bruit de rastérisation ─────────────────────────────────────────────────────────────
#
# « Le rendu local n'est pas celui du CI, donc on ne peut rien conclure » était vrai, et coûteux :
# cette étape ne disait RIEN, et six runs de suite sont partis rouges avec des références périmées
# que la machine du dev voyait parfaitement. Ce qu'on ne peut pas conclure, c'est qu'un écart de
# quelques dizaines de pixels soit une régression. Un écart de VINGT MILLE pixels, lui, n'est pas de
# l'anticrénelage : c'est un onglet en plus dans la barre, une ligne retirée, un bouton déplacé.
#
# Mesure faite sur ce dépôt le 2026-09-18 (Windows, 153 références, 97 en écart) : 32 écarts entre
# 48 et 224 px, et 65 entre 552 et 97 201 px — dont 50 captures `options_*`, exactement les panneaux
# qu'avaient changés les quatre commits visuels partis sans leurs références. **Rien entre 224 et
# 552** : les deux populations ne se touchent pas, et le seuil tombe dans le vide qui les sépare.
#
# Le seuil sépare les deux. Il est HEURISTIQUE et le dit ; une machine au rendu plus bruyant le
# relève (`WAKFU_SEUIL_BRUIT_CAPTURES=800 bash scripts/ci-local.sh`), et le CI reste seul juge.
SEUIL_BRUIT_CAPTURES="${WAKFU_SEUIL_BRUIT_CAPTURES:-300}"

# Étape « captures » hors conteneur. Bloquante si le rendu est prouvé épinglé (verdict du CI) ;
# sinon bloquante sur les seuls écarts trop grands pour du bruit, informative sur le reste.
etape_captures_locale() {
  if rendu_est_epingle; then
    step "test — overlay-testkit (captures, rendu épinglé)" \
      cargo test --no-fail-fast -p overlay-testkit
    return
  fi

  local attendu journal code ecarts
  attendu="$(sed -n 's/^MESA_ATTENDU="\(.*\)"$/\1/p' "$SCRIPT_DIR/setup-render-env.sh")"
  journal="$(mktemp)"

  printf '\n\033[1m▶ test — overlay-testkit (captures, rendu local non épinglé)\033[0m\n'
  printf '  Cette machine n\x27a pas le rendu du CI (Mesa %s, posé par setup-render-env.sh) :\n' "$attendu"
  printf '  un petit écart y est normal. Seuls ceux de plus de %s px sont comptés en échec.\n' "$SEUIL_BRUIT_CAPTURES"

  cargo test --no-fail-fast -p overlay-testkit > "$journal" 2>&1
  code=$?

  # « 'nom' Image did not match snapshot. Diff: N » — le seul format d'échec de COMPARAISON
  # d'`egui_kittest`. Tout autre échec (panique, adaptateur Vulkan introuvable, compilation) ne
  # produit pas cette ligne, et c'est ainsi qu'on les distingue.
  #
  # Deux tris, et jamais `sort -rn -u` en un seul : avec `-n`, `-u` déduplique sur la CLÉ de
  # comparaison — le nombre — et non sur la ligne. Il écrasait donc toutes les captures partageant
  # le même nombre de pixels en écart, ce qui n'a rien d'un cas tordu : un onglet ajouté décale les
  # mêmes pixels dans quinze captures du même panneau. Mesuré ici, la différence était de 52
  # références annoncées pour 65 réelles. Le `sort -u` lexical d'abord (dédoublonnage honnête, une
  # même capture pouvant échouer dans deux binaires de test), le tri numérique ensuite.
  ecarts="$(grep -oE "'[A-Za-z0-9_]+' Image did not match snapshot\. Diff: [0-9]+" "$journal" \
            | sed -E "s/'([A-Za-z0-9_]+)'.*Diff: ([0-9]+)/\2 \1/" \
            | sort -u | sort -k1,1nr)"

  if [ "$code" -ne 0 ] && [ -z "$ecarts" ]; then
    printf '\033[31m✗ captures — échec qui n\x27est pas une comparaison d\x27image\033[0m\n'
    printf '  (panique, compilation, adaptateur de rendu introuvable…) — 40 dernières lignes :\n'
    tail -40 "$journal"
    printf '  Journal complet : %s\n' "$journal"
    FAILED+=("captures — échec hors comparaison d'image")
    return
  fi

  local bruit=0
  local perimees=()
  while read -r diff nom; do
    [ -n "${nom:-}" ] || continue
    if [ "$diff" -le "$SEUIL_BRUIT_CAPTURES" ]; then
      bruit=$((bruit + 1))
    else
      perimees+=("$(printf '%7s px  %s' "$diff" "$nom")")
    fi
  done <<< "$ecarts"

  if [ "$bruit" -gt 0 ]; then
    printf '  %d écart(s) au niveau du bruit de rastérisation (≤ %s px) — ignoré(s).\n' \
      "$bruit" "$SEUIL_BRUIT_CAPTURES"
  fi

  if [ ${#perimees[@]} -eq 0 ]; then
    printf '\033[32m✓ captures — aucune référence manifestement périmée\033[0m\n'
    # Le verdict reste NON CONCLUANT : seul le rendu épinglé décide. On sait juste qu'aucun écart
    # n'est trop grand pour s'expliquer par la machine.
    CAPTURES_NON_VERIFIEES=1
    rm -f "$journal"
    return
  fi

  printf '\033[31m✗ captures — %d référence(s) probablement PÉRIMÉE(S)\033[0m\n' "${#perimees[@]}"
  printf '%s\n' "${perimees[@]}" | head -25
  if [ ${#perimees[@]} -gt 25 ]; then
    printf '  … et %d autre(s).\n' "$((${#perimees[@]} - 25))"
  fi
  printf '\n  Un changement visuel voulu doit porter ses références régénérées (§17.1 du plan) :\n'
  printf '    GitHub → Actions → « Régénérer les captures (rendu du CI) » → Run workflow\n'
  printf '  Régénérer ICI ne servirait à rien : le rendu de cette machine n\x27est pas celui du gate.\n'
  printf '  Journal complet : %s\n' "$journal"
  FAILED+=("captures — ${#perimees[@]} référence(s) probablement périmée(s)")
}

# `--captures-conteneur` : construit l'environnement de rendu du CI et y rejoue les captures.
#
# Trois précautions, toutes apprises à la construction de ce dispositif :
#
# - **Le dépôt est monté AU MÊME CHEMIN qu'à l'extérieur**, et `$HOME` aussi : les empreintes de
#   cargo contiennent des chemins absolus, donc `target/`, la toolchain et le registre déjà
#   présents sont réutilisés tels quels au lieu de tout recompiler et retélécharger.
# - **`--user`** : sans lui, le conteneur écrit dans `target/` et `~/.cargo` en tant que root, et le
#   dev retrouve chez lui des fichiers qu'il ne peut plus supprimer.
# - **`UPDATE_SNAPSHOTS` n'est transmis que s'il est NON VIDE** : `egui_kittest` fait un `env::var`
#   puis compare la valeur à une liste fermée, et PANIQUE sur tout ce qu'il ne connaît pas — une
#   chaîne vide comprise. Le transmettre systématiquement casserait chaque exécution.
etape_captures_conteneur() {
  if ! command -v docker > /dev/null 2>&1; then
    printf '\033[31m✗ --captures-conteneur : docker introuvable.\033[0m\n' >&2
    printf '  Sans lui, lancer le script sans option : les captures y seront informatives.\n' >&2
    FAILED+=("captures — docker absent")
    return
  fi
  step "image de rendu du CI" \
    docker build -q -t wakfu-ci-render -f .github/ci-image/Dockerfile .

  local passe_update=()
  if [ -n "${UPDATE_SNAPSHOTS:-}" ]; then
    passe_update=(-e "UPDATE_SNAPSHOTS=$UPDATE_SNAPSHOTS")
  fi

  step "test — overlay-testkit (captures, conteneur du CI)" \
    docker run --rm \
      --user "$(id -u):$(id -g)" \
      -v "$PWD:$PWD" -w "$PWD" \
      -v "$HOME:$HOME" -e "HOME=$HOME" \
      -e "RUSTUP_HOME=$HOME/.rustup" -e "CARGO_HOME=$HOME/.cargo" \
      -e "PATH=$HOME/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
      "${passe_update[@]}" \
      wakfu-ci-render cargo test --no-fail-fast -p overlay-testkit
}


if [ "$LINT_ONLY" -eq 0 ]; then
  step "test — crates métier"         cargo test --no-fail-fast -p overlay-engine -p overlay-ingest -p overlay-sync -p overlay-platform
  if [ "$PLATFORM" = linux ]; then
    step "test — overlay-ui (lib)"     cargo test --no-fail-fast -p overlay-ui --lib
    step "build — binaire Linux/X11"   cargo build -p overlay-ui --bin wakfu-companion-overlay-x11
  else
    step "build — workspace (Windows)" cargo build --workspace
  fi

  # GATE du CI depuis le 2026-09-13 (§17.1 du plan) — voir le bloc « Captures » plus haut pour ce
  # que ce script est en droit d'en conclure selon l'environnement.
  #
  # **Hors de la branche par plateforme depuis le 2026-09-18.** `overlay-testkit` compile et rend
  # sous Windows comme sous Linux (il ne dépend que de la LIB `overlay-ui`, jamais de son binaire) :
  # le laisser à l'intérieur du `if PLATFORM = linux` privait le seul poste de développement du
  # dépôt du seul signal qui l'aurait prévenu — et rendait `--captures-conteneur` inopérant sans
  # rien dire.
  if [ "$CAPTURES_CONTENEUR" -eq 1 ]; then
    etape_captures_conteneur
  else
    etape_captures_locale
  fi
fi

printf '\n────────────────────────────────────────\n'
if [ ${#FAILED[@]} -eq 0 ]; then
  printf '\033[32mTout est vert.\033[0m Le CI devrait passer sur cette plateforme (%s).\n' "$PLATFORM"
  if [ "$CAPTURES_NON_VERIFIEES" -eq 1 ]; then
    printf '\033[33mSauf les captures : rendu non épinglé ici, verdict non concluant.\033[0m\n'
    printf 'Les vérifier pour de bon : bash scripts/ci-local.sh --captures-conteneur\n'
  fi
  exit 0
fi
printf '\033[31m%d étape(s) en échec :\033[0m\n' "${#FAILED[@]}"
printf '  - %s\n' "${FAILED[@]}"
printf "\nAstuce : \`cargo fmt --all\` (sans \`-- --check\`) corrige tout seul les écarts de format.\n"
exit 1
