#!/usr/bin/env bash
# Recrée vendor/wgpu-hal-30.0.1/ (source pristine + patch DirectComposition appliqué) — ce
# dossier n'est PAS commité tel quel (il duplique tout un crate tiers) : seul le patch l'est.
# À lancer depuis n'importe où ; regénère toujours from scratch (idempotent).
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="30.0.1"
# Même empreinte que `patches/setup-vendor.sh` à la racine (SHA-256 publiée par l'index crates.io,
# `cksum` de wgpu-hal 30.0.1 — https://index.crates.io/wg/pu/wgpu-hal) : jamais de code compilé
# issu d'un téléchargement non vérifié, spike compris.
SHA256="b6b7fb58561a792bc237628ba0792e332de418fefe145f13b5ed8201e6d52f58"
DEST="vendor/wgpu-hal-${VERSION}"

CRATE="$(mktemp)"
trap 'rm -f "$CRATE"' EXIT
# crates.io rejette (403) toute requête sans User-Agent identifiable.
curl -sSfL -A "wakfu-companion-overlay-spike-s1 (github.com/Oumbra/wakfu-companion-overlay)" \
  "https://crates.io/api/v1/crates/wgpu-hal/${VERSION}/download" \
  -o "$CRATE"
echo "${SHA256}  ${CRATE}" | sha256sum -c --quiet - || {
  echo "ERREUR : empreinte SHA-256 inattendue pour wgpu-hal ${VERSION} — archive refusée." >&2
  exit 1
}

rm -rf "$DEST"
mkdir -p vendor
tar -xzf "$CRATE" -C vendor
patch -p1 -d "$DEST" < patches/wgpu-hal-30.0.1-directcomposition.patch

echo "OK : $DEST prêt (patch DirectComposition appliqué)."
