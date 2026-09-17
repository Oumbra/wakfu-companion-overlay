#!/usr/bin/env bash
# Build + lance overlay-ui pour prévisualiser l'overlay à l'instant T sur un vrai wakfu.log.
# Équivalent Linux de `preview.ps1` (Windows / PowerShell), même contrat :
#
#   1. Prépare vendor/wgpu-hal-30.0.1 (patch DirectComposition, voir patches/setup-vendor.sh) s'il
#      est absent : `[patch.crates-io]` du Cargo.toml racine s'applique aussi sous Linux, sans ce
#      dossier AUCUNE commande cargo touchant overlay-ui ne démarre.
#   2. `cargo run -p overlay-ui --bin wakfu-companion-overlay-x11 --profile preview` : compile puis
#      lance directement les fenêtres overlay, câblées sur overlay-ingest + overlay-engine. Le profil
#      `preview` (Cargo.toml racine) optimise comme `release` mais sans LTO fat ni
#      `codegen-units = 1`, qui ne servent qu'au binaire livré et coûtaient 5 à 10 min par relance.
#
# Sur SteamOS (Steam Deck), le système n'a ni cargo ni compilateur et son rootfs est en lecture
# seule : le script se relance alors TOUT SEUL dans le conteneur préparé par
# `scripts/setup-steamdeck.sh` (distrobox, HOME/écran/GPU partagés). Rien à faire de plus que
# lancer ce script depuis un terminal normal.
#
# Les fenêtres sont transparentes, toujours au-dessus, sans bordure, et n'apparaissent que si une
# fenêtre de jeu Wakfu est ouverte (elles s'ancrent dessus). Ctrl+Shift+W bascule interactif /
# clic-traversant (hotkey global), Ctrl+Shift+Q ou Ctrl+C quitte — combinaisons par défaut,
# modifiables dans l'onglet Raccourcis des Options. Ce binaire ne câble que quatre des neuf actions
# (bascule, quitter, Options, sélection multiple) — voir la doc de tête de
# src/bin/wakfu-companion-overlay-x11.rs.
#
# Usage :
#   bash crates/overlay-ui/preview.sh                  # découverte automatique du wakfu.log
#   bash crates/overlay-ui/preview.sh <chemin>         # rejouer un wakfu.log précis
#   bash crates/overlay-ui/preview.sh --debug          # build debug, sans optimisation
#   bash crates/overlay-ui/preview.sh --release        # vrai profil release (LTO fat), long
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CONTAINER="${CONTAINER:-wakfu-overlay-dev}"

PROFILE=preview
LOG_PATH=""
for arg in "$@"; do
  case "$arg" in
    --debug) PROFILE=dev ;;
    --release) PROFILE=release ;;
    --help|-h) sed -n '2,30p' "$0"; exit 0 ;;
    -*) echo "option inconnue : $arg (voir --help)" >&2; exit 2 ;;
    *)  LOG_PATH="$arg" ;;
  esac
done

# Relance le script là où il peut réellement compiler (bac à sable Flatpak → hôte → conteneur
# distrobox sur SteamOS) ; sans effet sur une machine qui a déjà Rust et un compilateur C.
. "$REPO_ROOT/scripts/dev-env.sh"
dev_env_reexec "$REPO_ROOT/crates/overlay-ui/preview.sh" "$@"

export PATH="$HOME/.cargo/bin:$PATH"

cd "$REPO_ROOT"

if [ ! -d vendor/wgpu-hal-30.0.1 ]; then
  printf '\033[33mvendor/ absent — préparation via patches/setup-vendor.sh (patch DirectComposition)…\033[0m\n'
  bash patches/setup-vendor.sh
fi

# Le binaire parle X11 (XWayland compris) : sans $DISPLAY, `GameWindowTracker::connect()` panique
# avec un message bien moins clair que celui-ci.
if [ -z "${DISPLAY:-}" ]; then
  echo "\$DISPLAY est vide : ce binaire a besoin d'un serveur X11 (XWayland sous KDE Wayland)." >&2
  echo "Lancer ce script depuis une session graphique de bureau (mode Bureau sur Steam Deck)." >&2
  exit 1
fi

# Deux binaires coexistent dans le crate (overlay-ui pour Windows, wakfu-companion-overlay-x11 pour
# Linux/X11) — cargo ne peut pas choisir seul ; ici on est forcément sous Linux.
printf '\n\033[36m=== overlay-ui — prévisualisation (wakfu-companion-overlay-x11) ===\033[0m\n'
printf '\033[36mCtrl+Shift+W = bascule interactif / clic-traversant  |  Ctrl+Shift+Q ou Ctrl+C = quitter\033[0m\n'
printf '\033[36mLes fenêtres overlay ne s’affichent qu’au-dessus d’une fenêtre de jeu Wakfu ouverte.\033[0m\n\n'

# L'origine de l'API est figée par le PROFIL de compilation (`crates/overlay-sync/build.rs`) :
# `preview`/debug → déploiement dev, `release` → prod, sans rien poser ici — un exe de preview
# lancé hors de ce script parle donc aussi à dev. Une valeur déjà posée dans l'environnement (ex.
# `wrangler pages dev` local) surcharge ce défaut, le binaire trace l'origine retenue au démarrage.
if [ -n "${WAKFU_COMPANION_API_URL:-}" ]; then
  printf '\033[90mAPI : %s (surcharge WAKFU_COMPANION_API_URL)\033[0m\n\n' "$WAKFU_COMPANION_API_URL"
elif [ "$PROFILE" = "release" ]; then
  printf '\033[90mAPI : prod (profil release)\033[0m\n\n'
else
  printf '\033[90mAPI : déploiement dev (profil %s)\033[0m\n\n' "$PROFILE"
fi

CARGO_ARGS=(run -p overlay-ui --bin wakfu-companion-overlay-x11 --profile "$PROFILE")
[ -n "$LOG_PATH" ] && CARGO_ARGS+=(-- "$LOG_PATH")

exec cargo "${CARGO_ARGS[@]}"
