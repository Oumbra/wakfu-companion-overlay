#!/usr/bin/env bash
# Dépendances système du RENDU, à versions ÉPINGLÉES — §17.3 du plan d'architecture (« un seul
# script d'installation idempotent versionné, versions documentées et pinnées — jamais d'apt-get ad
# hoc au fil d'une session »).
#
# POURQUOI l'épinglage : les captures d'`overlay-testkit` sont comparées pixel à pixel à des
# références versionnées. Le rendu vient de lavapipe, le rastériseur logiciel Vulkan de Mesa, et sa
# sortie change d'une version de Mesa (ou du LLVM qu'il embarque) à l'autre. Tant que le CI faisait
# `apt-get install mesa-vulkan-drivers` sur `ubuntu-latest`, il prenait ce qui était publié LE JOUR
# MÊME : une mise à jour de Mesa aurait rougi tout le CI sur du code que personne n'a touché. C'est
# l'incident dont `rust-toolchain.toml` est né (CI rouge du 2026-09-01 au 2026-09-10 sur une
# toolchain flottante), transposé au rendu — inacceptable depuis que la comparaison est un GATE.
#
# COMMENT : APT pointe vers un INSTANTANÉ DATÉ de l'archive Ubuntu (`snapshot.ubuntu.com`, service
# officiel et permanent) au lieu de l'archive vivante. Épingler seulement la version du paquet
# (`mesa-vulkan-drivers=25.2.8-...`) ne suffirait PAS : une version supplantée finit par DISPARAÎTRE
# de `archive.ubuntu.com`, et l'installation échouerait en 404 quelques semaines plus tard —
# constaté dans ce dépôt sur `libasound2-dev` le 2026-09-13.
#
# METTRE À JOUR LE RENDU est un geste EXPLICITE, dans son propre commit, exactement comme monter la
# version de Rust : changer `SNAPSHOT` ci-dessous, puis régénérer les références avec
# `UPDATE_SNAPSHOTS=1 cargo test -p overlay-testkit` DANS CET ENVIRONNEMENT (voir
# `.github/ci-image/Dockerfile`, qui le reproduit en local), et committer les deux ensemble. Jamais
# une dérive silencieuse.
#
# Utilisé par : le job `test-linux` de `.github/workflows/ci.yml` (dans son conteneur épinglé) et
# `.github/ci-image/Dockerfile` (même environnement, reproductible sur un poste de dev).
set -euo pipefail

# Instantané de l'archive Ubuntu 24.04 (noble). Porte `mesa-vulkan-drivers 25.2.8-0ubuntu0.24.04.2`,
# la version sous laquelle les références de `crates/overlay-testkit/tests/snapshots/` ont été
# produites et vérifiées.
SNAPSHOT="${WAKFU_APT_SNAPSHOT:-20260913T000000Z}"

# Version attendue, vérifiée APRÈS installation : si l'instantané cessait un jour de la servir, un
# échec explicite vaut mieux qu'un CI qui rougit ensuite sur 57 captures sans dire pourquoi.
MESA_ATTENDU="25.2.8-0ubuntu0.24.04.2"

# `sudo` seulement s'il existe ET qu'on n'est pas déjà root : root dans un conteneur (le cas du CI),
# sudo sur un poste de dev.
if [ "$(id -u)" -eq 0 ]; then SUDO=""; else SUDO="sudo"; fi

# `snapshot.ubuntu.com` impose TLS (une source `http` est redirigée en 301). Une image Ubuntu nue
# n'a pas `ca-certificates` : pointer APT vers l'instantané d'emblée échouerait sur un « certificate
# issuer is unknown » avant d'avoir pu installer le paquet qui règle le problème. Ce paquet-là (et
# lui seul) vient donc des sources par défaut. Il ne participe à aucun rendu.
$SUDO apt-get update
$SUDO env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates

# Format deb822 d'Ubuntu 24.04, réécrit d'un bloc vers l'instantané. Les trois suites d'où viennent
# Mesa et ses dépendances — dont `libllvm`, que lavapipe embarque et qui pèse autant que Mesa sur le
# rendu.
printf '%s\n' \
  'Types: deb' \
  "URIs: https://snapshot.ubuntu.com/ubuntu/${SNAPSHOT}" \
  'Suites: noble noble-updates noble-security' \
  'Components: main universe' \
  'Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg' \
  | $SUDO tee /etc/apt/sources.list.d/ubuntu.sources > /dev/null

$SUDO apt-get update
# `patch`, `curl`, `xz-utils` : requis par `patches/setup-vendor.sh` (vendor wgpu-hal).
# `libasound2-dev`/`pkg-config` : `rodio` → `alsa-sys` (voir §17.1 du plan).
# `mesa-vulkan-drivers` : lavapipe, le rendu lui-même.
$SUDO env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  build-essential \
  curl \
  git \
  libasound2-dev \
  mesa-vulkan-drivers \
  patch \
  pkg-config \
  xz-utils

MESA_INSTALLE="$(dpkg-query -W -f='${Version}' mesa-vulkan-drivers)"
echo "mesa-vulkan-drivers=${MESA_INSTALLE} (instantané ${SNAPSHOT})"
if [ "$MESA_INSTALLE" != "$MESA_ATTENDU" ]; then
  echo "ERREUR : Mesa ${MESA_INSTALLE} installé, ${MESA_ATTENDU} attendu." >&2
  echo "Les références de tests/snapshots/ ont été produites sous ${MESA_ATTENDU} ; tout autre" >&2
  echo "rendu les ferait échouer. Corriger SNAPSHOT/MESA_ATTENDU ensemble, et régénérer les" >&2
  echo "références dans le nouvel environnement (voir l'en-tête de ce script)." >&2
  exit 1
fi
