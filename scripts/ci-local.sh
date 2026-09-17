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
# machine du dev. Tout le reste est identique ; pour les captures, voir
# `avertir_si_mesa_different` plus bas.
#
# Usage :
#   bash scripts/ci-local.sh                       # tout : format, clippy, tests
#   bash scripts/ci-local.sh --lint                # format + clippy seulement (rapide)
#   bash scripts/ci-local.sh --captures-conteneur  # captures dans l'environnement de rendu du CI
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

step "fmt — workspace"                cargo fmt --all -- --check
step "fmt — xtask"                    cargo fmt --manifest-path xtask/Cargo.toml -- --check

step "clippy — crates métier"         cargo clippy -p overlay-engine -p overlay-ingest -p overlay-sync -p overlay-platform -p overlay-app --all-targets -- -D warnings
if [ "$PLATFORM" = linux ]; then
  step "clippy — overlay-ui (lib + x11)" cargo clippy -p overlay-ui --lib --bin wakfu-companion-overlay-x11 -- -D warnings
  step "clippy — overlay-testkit"        cargo clippy -p overlay-testkit --all-targets -- -D warnings
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

# Étape « captures » hors conteneur : bloquante si et seulement si le rendu est prouvé épinglé.
etape_captures_locale() {
  if rendu_est_epingle; then
    step "test — overlay-testkit (captures, rendu épinglé)" \
      cargo test --no-fail-fast -p overlay-testkit
    return
  fi
  local attendu
  attendu="$(sed -n 's/^MESA_ATTENDU="\(.*\)"$/\1/p' "$SCRIPT_DIR/setup-render-env.sh")"
  printf '\n\033[1m▶ test — overlay-testkit (captures — INFORMATIF, rendu non épinglé)\033[0m\n'
  printf '\033[33m  Cette machine n\x27a pas le rendu du CI (Mesa %s, posé par setup-render-env.sh).\033[0m\n' "$attendu"
  printf '  Un écart ci-dessous peut ne venir que de là, et n\x27est donc PAS compté en échec.\n'
  printf '  Pour un verdict qui vaut — et pour RÉGÉNÉRER des références :\n'
  printf '    bash scripts/ci-local.sh --captures-conteneur\n'
  CAPTURES_NON_VERIFIEES=1
  cargo test --no-fail-fast -p overlay-testkit \
    || printf '\033[33m! captures en écart — informatif, rendu non épinglé\033[0m\n'
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
  step "test — crates métier"         cargo test --no-fail-fast -p overlay-engine -p overlay-ingest -p overlay-sync -p overlay-platform -p overlay-app
  if [ "$PLATFORM" = linux ]; then
    step "test — overlay-ui (lib)"     cargo test --no-fail-fast -p overlay-ui --lib
    step "build — binaire Linux/X11"   cargo build -p overlay-ui --bin wakfu-companion-overlay-x11
    # GATE du CI depuis le 2026-09-13 (§17.1 du plan). Bloquant ici seulement si le rendu de cette
    # machine est prouvé être celui du CI — voir le bloc « Captures » plus haut.
    if [ "$CAPTURES_CONTENEUR" -eq 1 ]; then
      etape_captures_conteneur
    else
      etape_captures_locale
    fi
  else
    step "build — workspace (Windows)" cargo build --workspace
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
