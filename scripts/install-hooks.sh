#!/usr/bin/env bash
# Active les hooks git versionnés du dépôt (`.githooks/`) — à lancer UNE FOIS après un clone.
#
#   bash scripts/install-hooks.sh
#
# Utilise `core.hooksPath` plutôt qu'une copie dans `.git/hooks/` : le hook reste alors un fichier
# versionné, corrigé pour tout le monde en même temps que le reste du dépôt, sans réinstallation.
#
# Pour désactiver : `git config --unset core.hooksPath`.
set -euo pipefail
cd "$(dirname "$0")/.."

git config core.hooksPath .githooks

printf 'Hooks activés (core.hooksPath = .githooks).\n\n'
printf '  pre-push : rejoue les lints du CI (cargo fmt --check + clippy -D warnings)\n'
printf '             et refuse le push en cas d écart. Contournement : git push --no-verify\n'
