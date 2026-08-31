#!/usr/bin/env bash
# Recrée vendor/wgpu-hal-30.0.1/ (source pristine + patch DirectComposition appliqué) — ce
# dossier n'est PAS commité tel quel (il duplique tout un crate tiers) : seul le patch l'est.
# À lancer depuis n'importe où ; regénère toujours from scratch (idempotent).
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="30.0.1"
DEST="vendor/wgpu-hal-${VERSION}"

rm -rf "$DEST"
mkdir -p vendor
# crates.io rejette (403) toute requête sans User-Agent identifiable.
curl -sSL -A "wakfu-companion-overlay-spike-s1 (github.com/Oumbra/wakfu-companion-overlay)" \
  "https://crates.io/api/v1/crates/wgpu-hal/${VERSION}/download" \
  -o "/tmp/wgpu-hal-${VERSION}.crate"
tar -xzf "/tmp/wgpu-hal-${VERSION}.crate" -C vendor
patch -p1 -d "$DEST" < patches/wgpu-hal-30.0.1-directcomposition.patch

echo "OK : $DEST prêt (patch DirectComposition appliqué)."
