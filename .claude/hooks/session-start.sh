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

echo "session-start : environnement prêt."
