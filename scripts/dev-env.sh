#!/usr/bin/env bash
# Relance un script du dépôt LÀ OÙ IL PEUT RÉELLEMENT COMPILER — à sourcer, pas à exécuter.
#
# Sur une machine de développement ordinaire (Windows, Linux avec Rust installé, runner du CI),
# `dev_env_reexec` ne fait RIEN : la chaîne de compilation est déjà là.
#
# Sur SteamOS (Steam Deck), elle traverse les deux frontières qui séparent un terminal quelconque
# d'un environnement capable de compiler (voir `scripts/setup-steamdeck.sh`) :
#   1. le bac à sable Flatpak (terminal intégré de VS Code Flatpak…) → l'hôte, via `flatpak-spawn` ;
#   2. l'hôte, dont le rootfs en lecture seule n'a ni compilateur ni en-têtes → le conteneur
#      `distrobox`, qui partage HOME, écran, GPU et audio.
#
# Sans cela, `scripts/ci-local.sh` (et le hook `pre-push` qui l'appelle) échouerait sur un
# « cargo: commande introuvable » qui RESSEMBLE à un écart de lints et pousse à `--no-verify` —
# exactement l'habitude que ce garde-fou existe pour empêcher (voir CLAUDE.md § CI).
#
# Usage, au tout début du script appelant :
#   . "$(dirname "$0")/dev-env.sh"      # ou ../../scripts/dev-env.sh selon l'emplacement
#   dev_env_reexec "$0" "$@"

DEV_ENV_CONTAINER="${CONTAINER:-wakfu-overlay-dev}"

# `/run/.containerenv` : marqueur podman, posé dans tout conteneur distrobox.
dev_env_in_container() { [ -f /run/.containerenv ] || [ -f /.dockerenv ]; }

# Vrai seulement si on peut RÉELLEMENT compiler ici : cargo ET un linker C. `rustup` s'installe
# dans le HOME, donc `~/.cargo/bin/cargo` devient visible depuis l'hôte dès que le conteneur l'a
# installé — le voir ne dit rien de la possibilité de compiler. Sans `cc`, le build échouerait à
# l'édition de liens, après plusieurs minutes de compilation pour rien.
dev_env_can_build() {
  { command -v cargo >/dev/null 2>&1 || [ -x "$HOME/.cargo/bin/cargo" ]; } \
    && command -v cc >/dev/null 2>&1
}

# `scripts/setup-steamdeck.sh` installe rustup avec `--no-modify-path` (il n'a rien à faire dans le
# profil de l'utilisateur, ni sur l'hôte ni dans le conteneur) : `~/.cargo/bin` n'est donc dans
# AUCUN PATH, y compris celui d'un shell de login du conteneur. On l'ajoute ici, pour tous les
# scripts qui passent par ce module.
dev_env_use_cargo_home() {
  if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
}

dev_env_reexec() {
  local script="$1"
  shift

  dev_env_use_cargo_home

  # Windows (Git Bash / MSYS) : le linker est celui de MSVC, pas `cc` — le test ci-dessus n'y veut
  # rien dire, et ni Flatpak ni distrobox n'y existent. On ne touche à rien.
  case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*) return 0 ;;
  esac

  if [ -f /.flatpak-info ] && command -v flatpak-spawn >/dev/null 2>&1 &&
    ! dev_env_in_container; then
    # `flatpak-spawn` propage l'environnement de l'appelant, `$DISPLAY` VIDE compris (une
    # application Flatpak sans permission X11 ne le voit pas) : sans ce `--env`, un overlay relancé
    # sur l'hôte se croirait sans session graphique. `:0` est le display d'un bureau standard.
    echo "Bac à sable Flatpak détecté — relance sur l'hôte…"
    exec flatpak-spawn --host --env=DISPLAY="${DISPLAY:-:0}" bash "$script" "$@"
  fi

  dev_env_can_build && return 0

  if dev_env_in_container; then
    echo "Chaîne de compilation incomplète DANS le conteneur (cargo ou cc) —" >&2
    echo "relancer : bash scripts/setup-steamdeck.sh" >&2
    exit 1
  fi
  if ! command -v distrobox >/dev/null 2>&1; then
    echo "Ni chaîne de compilation ni distrobox : installer Rust + un compilateur C," >&2
    echo "ou sur SteamOS lancer scripts/setup-steamdeck.sh." >&2
    exit 1
  fi
  if ! distrobox list 2>/dev/null | awk 'NR>1 {print $3}' | grep -qx "$DEV_ENV_CONTAINER"; then
    echo "Conteneur « $DEV_ENV_CONTAINER » absent — lancer : bash scripts/setup-steamdeck.sh" >&2
    exit 1
  fi

  # Chemins ABSOLUS : le conteneur démarre dans un répertoire courant qui n'est pas forcément
  # celui d'ici, un chemin relatif n'y désignerait plus le même fichier.
  local script_dir script_abs repo_root
  script_dir="$(cd "$(dirname "$script")" && pwd)"
  script_abs="$script_dir/$(basename "$script")"
  repo_root="$(git -C "$script_dir" rev-parse --show-toplevel 2>/dev/null || echo "$script_dir")"

  echo "Pas de chaîne de compilation sur l'hôte — relance dans le conteneur « $DEV_ENV_CONTAINER »…"
  exec distrobox enter --name "$DEV_ENV_CONTAINER" -- bash -lc \
    "cd '$repo_root' && bash '$script_abs' $(printf '%q ' "$@")"
}
