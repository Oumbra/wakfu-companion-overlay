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
#   bash scripts/ci-local.sh            # tout : format, clippy, tests
#   bash scripts/ci-local.sh --lint     # format + clippy seulement (rapide)
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
case "${1:-}" in
  --lint) LINT_ONLY=1 ;;
  '') ;;
  *) echo "usage : bash scripts/ci-local.sh [--lint]" >&2; exit 2 ;;
esac

# Windows (Git Bash / MSYS) ne compile pas les mêmes cibles que Linux : `overlay-ui::main` importe
# `windows::` sans `cfg` (Windows uniquement), et `overlay-ui-x11` vise X11 (Linux uniquement).
# Même découpage que le CI, qui a un job par plateforme pour cette raison.
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
  step "clippy — overlay-ui (lib + x11)" cargo clippy -p overlay-ui --lib --bin overlay-ui-x11 -- -D warnings
  step "clippy — overlay-testkit"        cargo clippy -p overlay-testkit --all-targets -- -D warnings
fi
step "clippy — xtask"                 cargo clippy --manifest-path xtask/Cargo.toml --all-targets -- -D warnings

# Les captures d'`overlay-testkit` sont un GATE du CI depuis le 2026-09-13 (§17.1 du plan), et
# elles y sont comparées sous un rendu FIGÉ : Mesa épinglé par `scripts/setup-render-env.sh`, dans
# le conteneur épinglé du job `test-linux`. Ce script-ci, lui, tourne sur la machine du dev, avec le
# Mesa que cette machine a. Les deux coïncident souvent — pas toujours.
#
# D'où un avertissement plutôt qu'un échec : un écart de Mesa ici ne dit RIEN de ce que fera le CI,
# et laisser un dev régénérer des références sous son propre Mesa est exactement ce qu'il ne faut
# pas faire — elles rougiraient le CI pour tout le monde. On le prévient, on ne le bloque pas.
avertir_si_mesa_different() {
  command -v dpkg-query > /dev/null 2>&1 || return 0   # ni Debian ni Ubuntu : rien à comparer
  local attendu installe
  attendu="$(sed -n 's/^MESA_ATTENDU="\(.*\)"$/\1/p' "$SCRIPT_DIR/setup-render-env.sh")"
  installe="$(dpkg-query -W -f='${Version}' mesa-vulkan-drivers 2>/dev/null || true)"
  [ -n "$attendu" ] && [ -n "$installe" ] || return 0
  [ "$attendu" = "$installe" ] && return 0
  printf '\n\033[33m! Mesa local %s, le CI compare les captures sous %s.\033[0m\n' "$installe" "$attendu"
  printf '  Un écart de capture ci-dessous peut ne venir que de là. Pour trancher — et pour\n'
  printf '  RÉGÉNÉRER des références — passer par l\x27environnement de rendu du CI :\n'
  printf '    docker build -t wakfu-ci-render -f .github/ci-image/Dockerfile .\n'
  printf '    docker run --rm -v "$PWD:$PWD" -w "$PWD" wakfu-ci-render \\\n'
  printf '      cargo test --no-fail-fast -p overlay-testkit\n'
}

if [ "$LINT_ONLY" -eq 0 ]; then
  step "test — crates métier"         cargo test --no-fail-fast -p overlay-engine -p overlay-ingest -p overlay-sync -p overlay-platform -p overlay-app
  if [ "$PLATFORM" = linux ]; then
    step "test — overlay-ui (lib)"    cargo test --no-fail-fast -p overlay-ui --lib
    step "build — overlay-ui-x11"     cargo build -p overlay-ui --bin overlay-ui-x11
    # GATE du CI depuis le 2026-09-13 (§17.1 du plan) : compté comme un échec ici aussi, comme
    # toutes les autres étapes — un écart de capture bloque désormais le CI pour de vrai.
    avertir_si_mesa_different
    step "test — overlay-testkit (captures)" cargo test --no-fail-fast -p overlay-testkit
  else
    step "build — workspace (Windows)" cargo build --workspace
  fi
fi

printf '\n────────────────────────────────────────\n'
if [ ${#FAILED[@]} -eq 0 ]; then
  printf '\033[32mTout est vert.\033[0m Le CI devrait passer sur cette plateforme (%s).\n' "$PLATFORM"
  exit 0
fi
printf '\033[31m%d étape(s) en échec :\033[0m\n' "${#FAILED[@]}"
printf '  - %s\n' "${FAILED[@]}"
printf "\nAstuce : \`cargo fmt --all\` (sans \`-- --check\`) corrige tout seul les écarts de format.\n"
exit 1
