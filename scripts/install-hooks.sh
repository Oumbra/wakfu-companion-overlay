#!/usr/bin/env bash
# Active les hooks git versionnés du dépôt (`.githooks/`) — à lancer UNE FOIS après un clone.
#
#   bash scripts/install-hooks.sh
#
# Utilise `core.hooksPath` plutôt qu'une copie dans `.git/hooks/` : le hook reste alors un fichier
# versionné, corrigé pour tout le monde en même temps que le reste du dépôt, sans réinstallation.
#
# Trois hooks sont posés : `pre-commit` (données réelles dans une fixture), `pre-push` (lints du
# CI) et `post-commit` (version du produit) — voir leurs fichiers respectifs dans `.githooks/`.
#
# Pour désactiver : `git config --unset core.hooksPath`.
set -euo pipefail
cd "$(dirname "$0")/.."

git config core.hooksPath .githooks

printf 'Hooks activés (core.hooksPath = .githooks).\n\n'
printf '  pre-commit  : refuse une fixture wakfu.log qui contiendrait des données réelles\n'
printf '                (jeton, IP, nom de compte, pseudonymes de tiers). Contournement :\n'
printf '                git commit --no-verify\n'
printf '  pre-push    : rejoue les lints du CI (cargo fmt --check + clippy -D warnings)\n'
printf '                et refuse le push en cas d écart. Contournement : git push --no-verify\n'
printf '  post-commit : incrémente la version du produit selon le type du commit\n'
printf '                (feat -> minor, fix/style/perf/refactor/test -> patch, ! -> major).\n'
printf '                Contournement : SKIP_VERSION_BUMP=1 git commit ...\n'
