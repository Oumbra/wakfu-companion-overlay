#!/usr/bin/env bash
# Prépare un environnement de compilation Rust utilisable sur SteamOS (Steam Deck) — à lancer UNE
# fois, depuis un terminal de l'hôte (Konsole en mode Bureau).
#
# Pourquoi un conteneur : le rootfs de SteamOS est en LECTURE SEULE et son contenu est écrasé à
# chaque mise à jour du système. Il n'y a ni `cargo`, ni `gcc`, ni en-têtes de développement
# (X11, ALSA, Vulkan) — et `steamos-readonly disable` + `pacman -S` ne tiendrait pas d'une mise à
# jour à l'autre. `distrobox` (déjà fourni par SteamOS, au-dessus de `podman`) crée un conteneur
# Arch qui PARTAGE le HOME, l'affichage, le GPU et l'audio de l'hôte : le dépôt s'y compile et
# l'overlay s'y lance en voyant les vraies fenêtres du jeu, sans rien installer dans le système.
#
# Une fois ce script passé, tout se fait avec `crates/overlay-ui/preview.sh` (qui entre tout seul
# dans le conteneur).
#
# Usage :
#   bash scripts/setup-steamdeck.sh              # crée/complète le conteneur, installe Rust
#   CONTAINER=autre-nom bash scripts/setup-steamdeck.sh
set -euo pipefail

CONTAINER="${CONTAINER:-wakfu-overlay-dev}"
IMAGE="${IMAGE:-docker.io/library/archlinux:latest}"

# Ce script pilote `podman` : il doit tourner sur l'HÔTE, pas dans le bac à sable d'une application
# Flatpak (le terminal intégré de VS Code Flatpak, par exemple, n'y a pas accès directement).
if [ -f /.flatpak-info ] && ! command -v podman >/dev/null 2>&1; then
  echo "Ce script doit être lancé depuis un terminal de l'hôte (Konsole), pas depuis un Flatpak." >&2
  echo "Astuce : flatpak-spawn --host bash scripts/setup-steamdeck.sh" >&2
  exit 1
fi

if ! command -v distrobox >/dev/null 2>&1; then
  echo "distrobox est introuvable — attendu sur SteamOS (/usr/bin/distrobox)." >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

printf '\n\033[1m▶ Conteneur %s (%s)\033[0m\n' "$CONTAINER" "$IMAGE"
if distrobox list 2>/dev/null | awk 'NR>1 {print $3}' | grep -qx "$CONTAINER"; then
  echo "déjà créé — réutilisé."
else
  distrobox create --yes --name "$CONTAINER" --image "$IMAGE"
fi

# `--` : tout ce qui suit est exécuté DANS le conteneur.
#
# Paquets : `base-devel` (gcc, ld, make) pour le linker de Rust ; les en-têtes X11 exigées par
# winit/x11rb (`overlay-ui-x11`) ; `alsa-lib` pour `rodio` (sons d'alerte) ; `vulkan-icd-loader` +
# `mesa` + `vulkan-radeon` pour wgpu sur l'APU AMD du Deck ; `curl`/`patch`/`tar` pour
# `patches/setup-vendor.sh` ; `git` pour les dépendances de `cargo`.
printf '\n\033[1m▶ Dépendances système dans le conteneur\033[0m\n'
distrobox enter --name "$CONTAINER" -- bash -lc '
  set -euo pipefail
  sudo pacman -Sy --needed --noconfirm \
    base-devel pkgconf git curl patch \
    libx11 libxcb libxkbcommon libxkbcommon-x11 libxcursor libxrandr libxi \
    alsa-lib vulkan-icd-loader vulkan-radeon mesa
'

# rustup s'installe dans le HOME (partagé avec l'hôte) : il survit donc à la recréation du
# conteneur, et `rust-toolchain.toml` du dépôt sélectionne tout seul la version épinglée par le CI
# (voir CLAUDE.md § « CI »). Ne jamais forcer une autre version ici.
printf '\n\033[1m▶ Rust (rustup dans ~/.cargo, partagé avec l’hôte)\033[0m\n'
distrobox enter --name "$CONTAINER" -- bash -lc "
  set -euo pipefail
  if ! command -v rustup >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --default-toolchain none
  fi
  export PATH=\"\$HOME/.cargo/bin:\$PATH\"
  cd '$REPO_ROOT'
  rustup show           # installe la toolchain de rust-toolchain.toml si besoin
  cargo --version
"

printf '\n\033[32m✓ Environnement prêt.\033[0m\n'
printf 'Prévisualiser l’overlay :  bash crates/overlay-ui/preview.sh\n'
printf 'Entrer dans le conteneur :  distrobox enter %s\n' "$CONTAINER"
