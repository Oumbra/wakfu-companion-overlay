#!/usr/bin/env bash
# Hook PreToolUse (matcher Bash) : réécrit la commande en `rtk <commande>` pour en condenser la
# sortie (60–90 % de jetons en moins sur git, cargo, ls, grep… d'après RTK). `rtk hook claude`
# lit le JSON de l'appel sur stdin et répond, s'il connaît la commande, un `updatedInput` ; il se
# tait sur une commande inconnue ou déjà préfixée — vérifié le 2026-09-18 avec rtk 0.48.0.
#
# Réservé à la session cloud : sur le poste du mainteneur, RTK est déjà branché en global par
# `rtk init -g` (~/.claude/settings.json), et doubler le hook n'apporterait rien. Et si le binaire
# manque (installation échouée dans session-start.sh), ne rien répondre : Claude Code garde alors
# la commande d'origine, ce qui vaut toujours mieux qu'une réécriture vers un `rtk` introuvable.
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

if ! command -v rtk >/dev/null 2>&1; then
  exit 0
fi

exec rtk hook claude
