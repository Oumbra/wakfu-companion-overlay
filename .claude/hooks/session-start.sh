#!/usr/bin/env bash
# Prépare un conteneur de session cloud pour que `cargo` fonctionne sans manipulation manuelle.
#
# Trois choses manquent à un conteneur neuf, et chacune donne une erreur qui n'a rien à voir avec
# sa cause (voir CLAUDE.md, « CI » et « Marche à suivre en début de session ») :
#
#   1. vendor/wgpu-hal-30.0.1   absent  → « Unable to update /…/vendor/wgpu-hal-30.0.1 »
#   2. libasound2-dev           absent  → « failed to run custom build command for alsa-sys »
#   3. core.hooksPath           non posé → le pre-push du dépôt ne s'exécute pas, et un écart de
#                                          `cargo fmt` part en CI sans que rien ne l'ait signalé
#
# mesa-vulkan-drivers s'y ajoute : `overlay-testkit` rend hors écran sur un adaptateur logiciel
# (lavapipe), sans lequel toute la suite de snapshots échoue à l'initialisation de wgpu.
#
# RTK (https://github.com/rtk-ai/rtk) s'y ajoute aussi : le hook PreToolUse déclaré dans
# `.claude/settings.json` (`rtk-hook.sh`) réécrit chaque commande Bash en `rtk <commande>` pour en
# condenser la sortie.
# Sur le poste du mainteneur, RTK est installé et branché en global (`rtk init -g`) ; le conteneur
# éphémère, lui, repart sans binaire, et une commande réécrite sans `rtk` dans le PATH échouerait
# toutes en « command not found ». D'où l'installation ici, dans /usr/local/bin (toujours dans le
# PATH, à la différence de ~/.local/bin que choisit le script par défaut). Voie principale : le
# script officiel (binaire précompilé). Repli si celui-ci échoue : `cargo run -p xtask --
# setup-tools`, qui installe RTK (et tout futur outil Cargo global du dépôt) via `cargo install` —
# la même commande, réutilisable telle quelle sur un poste de dev ou une autre machine.
#
# Ne fait rien hors session cloud : un poste de développeur a déjà ses paquets système, et ce
# script ne doit pas décider à sa place d'en installer.
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "${CLAUDE_PROJECT_DIR:-$(dirname "$0")/../..}"

# 1. Paquets système — seulement ceux qui manquent, et sans reconstruire l'index si tout est là.
missing=()
for pkg in libasound2-dev pkg-config mesa-vulkan-drivers; do
  dpkg -s "$pkg" >/dev/null 2>&1 || missing+=("$pkg")
done
if [ ${#missing[@]} -gt 0 ]; then
  echo "session-start : installation de ${missing[*]}"
  apt-get update -qq
  apt-get install -y -qq --no-install-recommends "${missing[@]}"
fi

# 2. Source vendue et patchée de wgpu-hal (DirectComposition, voir Cargo.toml racine).
#    `setup-vendor.sh` retélécharge à chaque appel : ne le relancer que si le dossier manque.
if [ ! -f vendor/wgpu-hal-30.0.1/Cargo.toml ]; then
  echo "session-start : reconstruction de vendor/wgpu-hal-30.0.1"
  bash patches/setup-vendor.sh
fi

# 3. Hooks git versionnés — `core.hooksPath` est une config LOCALE, elle ne survit pas au
#    conteneur éphémère.
bash scripts/install-hooks.sh >/dev/null

# 4. RTK — binaire Linux précompilé, version épinglée (même esprit que rust-toolchain.toml : une
#    montée de version est un changement explicite, pas une dérive d'un conteneur à l'autre).
#    Le script officiel vérifie la somme SHA-256 de l'archive contre checksums.txt de la Release —
#    à préférer à `xtask setup-tools` (voir plus bas) quand c'est disponible : instantané, et la
#    version installée correspond exactement à celle du poste du mainteneur.
#    Un échec ici n'est pas bloquant : `rtk-hook.sh` laisse passer les commandes telles quelles
#    quand le binaire manque, la session tourne juste sans condensation.
RTK_VERSION_PINNED="v0.49.0"
if ! command -v rtk >/dev/null 2>&1; then
  echo "session-start : installation de rtk ${RTK_VERSION_PINNED}"
  if ! curl -fsSL https://raw.githubusercontent.com/rtk-ai/rtk/refs/heads/master/install.sh |
      RTK_INSTALL_DIR=/usr/local/bin RTK_VERSION="${RTK_VERSION_PINNED}" sh >/dev/null; then
    echo "session-start : installation du binaire précompilé échouée, repli sur \`xtask setup-tools\` (cargo install)."
    if ! cargo run --quiet --manifest-path xtask/Cargo.toml -- setup-tools; then
      echo "session-start : xtask setup-tools a aussi échoué, sortie des commandes non condensée."
    fi
  fi
fi

echo "session-start : environnement prêt."
if command -v rtk >/dev/null 2>&1; then
  # Repris de ~/.claude/RTK.md (posé par `rtk init -g`), que le conteneur n'a pas : la sortie du
  # hook SessionStart est versée dans le contexte, c'est le seul endroit où le dire.
  cat <<RTK
RTK actif ($(rtk --version)) : la sortie des commandes Bash est condensée pour économiser des jetons,
tout signal utile est conservé. La traiter comme le résultat complet. Un résultat tronqué indique
lui-même comment récupérer le reste. Ne relancer une commande via \`rtk proxy <cmd>\` que si son
résultat est inutilisable : vide alors qu'une sortie était attendue, contradictoire avec son code
de retour, ou illisible.
RTK
fi
