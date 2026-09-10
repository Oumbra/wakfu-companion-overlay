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
# Usage :
#   bash scripts/ci-local.sh            # tout : format, clippy, tests
#   bash scripts/ci-local.sh --lint     # format + clippy seulement (rapide)
#
# Le script ne s'arrête PAS à la première erreur : il déroule tout et récapitule à la fin, pour
# corriger l'ensemble en une passe plutôt qu'un aller-retour par étape.
set -uo pipefail
cd "$(dirname "$0")/.."

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

if [ "$LINT_ONLY" -eq 0 ]; then
  step "test — crates métier"         cargo test -p overlay-engine -p overlay-ingest -p overlay-sync -p overlay-platform -p overlay-app
  if [ "$PLATFORM" = linux ]; then
    step "test — overlay-ui (lib)"    cargo test -p overlay-ui --lib
    step "build — overlay-ui-x11"     cargo build -p overlay-ui --bin overlay-ui-x11
    # Miroir du `continue-on-error: true` du CI (§17.3 du plan : snapshots pas encore promus en
    # gate) — exécuté pour information, jamais compté comme un échec.
    printf '\n\033[1m▶ test — overlay-testkit (informatif, non bloquant)\033[0m\n'
    cargo test -p overlay-testkit || printf '\033[33m! snapshots en écart — informatif, non bloquant (§17.3)\033[0m\n'
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
