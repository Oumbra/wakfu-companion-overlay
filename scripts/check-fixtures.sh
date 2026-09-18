#!/usr/bin/env bash
# Refuse une fixture `wakfu.log` qui contiendrait des données réelles.
#
#   bash scripts/check-fixtures.sh
#
# Le dépôt a versionné pendant deux semaines, puis PUBLIÉ le 2026-09-15, un vrai journal du client
# Wakfu : jeton de session, IP locale, nom de compte Windows, 6 personnages avec leurs identifiants
# numériques, 119 pseudonymes de joueurs tiers et l'intégralité de leurs messages de chat (constat
# C1 de `docs/analyse-rgpd.md`). Il est arrivé par un `git add` ordinaire pendant le spike S2, à une
# époque où le dépôt était privé, et RIEN n'a alerté — ni à l'ajout, ni au passage en public.
#
# Ce script est ce qui manquait. Il est purement textuel (quelques millisecondes, vrai partout), et
# il est branché aux DEUX bouts, comme le demande le CLAUDE.md : le hook `pre-commit` l'appelle
# avant chaque commit, et `scripts/ci-local.sh` comme `.github/workflows/ci.yml` en font une étape.
# Le hook seul ne suffirait pas : `core.hooksPath` est une configuration LOCALE, elle ne survit pas
# au conteneur éphémère d'une session cloud. C'est l'étape de CI qui fait foi.
#
# Ce qu'il sait voir, et rien de plus : les cinq marqueurs qui ont réellement été trouvés dans le
# fichier. Un garde-fou ne protège que ce qu'il connaît — quand une catégorie de donnée nouvelle
# apparaît dans un journal, elle s'ajoute ici, pas ailleurs.
set -uo pipefail
cd "$(dirname "$0")/.."

# Les deux copies (octet pour octet identiques) de la fixture. Toute autre `wakfu.log` est ignorée
# par `.gitignore` et n'a donc pas à être inspectée ici.
FIXTURES=(
  crates/overlay-engine/tests/wakfu.log
  spikes/s2-engine-quickjs/tests/wakfu.log
)

ECHECS=0

signaler() {
  printf '\033[31m✗ %s\033[0m\n  %s\n' "$1" "$2" >&2
  ECHECS=$((ECHECS + 1))
}

# `grep -c` sur un motif absent renvoie 0 ET un statut 1 : `|| true` évite que `set -o pipefail`
# fasse passer une absence de correspondance — le cas NOMINAL — pour une erreur.
compter() { grep -cE "$1" "$2" 2> /dev/null || true; }

for f in "${FIXTURES[@]}"; do
  [ -f "$f" ] || { signaler "$f" "fixture attendue et absente"; continue; }

  # 1. Jeton de session du client de jeu. Le seul secret qu'un wakfu.log contienne.
  n=$(grep -E 'Authentication token received from dispatch server : [0-9a-fA-F-]{36}' "$f" 2> /dev/null \
      | grep -cvE ': 0{8}-0{4}-0{4}-0{4}-0{12}' || true)
  [ "$n" -gt 0 ] && signaler "$f : $n jeton(s) d'authentification" \
    "attendu « 00000000-0000-0000-0000-000000000000 »"

  # 2. Nom de compte Windows. Le chemin du journal le porte à chaque ligne de texture chargée.
  n=$(grep -oE 'C:[\\/]Users[\\/][^\\/ ]+' "$f" 2> /dev/null | grep -cv 'anonymous' || true)
  [ "$n" -gt 0 ] && signaler "$f : $n chemin(s) C:\\Users\\<nom>" \
    "attendu « anonymous » — le vrai nom de compte n'a rien à faire ici"

  # 3. Adresse IP privée. La 192.0.2.0/24 (RFC 5737) est réservée à la documentation, non routable.
  n=$(compter '(^|[^0-9.])(10\.|192\.168\.|172\.(1[6-9]|2[0-9]|3[01])\.)[0-9]{1,3}\.[0-9]{1,3}' "$f")
  [ "$n" -gt 0 ] && signaler "$f : $n ligne(s) avec une IP privée" \
    "attendu une adresse de la plage de documentation 192.0.2.0/24 (RFC 5737)"

  # 4. Auteur de chat non pseudonymisé. Six canaux publics, un pseudonyme de tiers par ligne.
  n=$(grep -oE '\) - \[(Proximité|Guilde|Commerce|Groupe|Équipe|Recrutement[^]]*|Communauté[^]]*)\] [^:]+ : ' "$f" 2> /dev/null \
      | grep -cvE '\] Anonyme-[0-9]{3} : ' || true)
  [ "$n" -gt 0 ] && signaler "$f : $n message(s) de chat à auteur réel" \
    "attendu « Anonyme-NNN » — et un contenu de message généré, jamais recopié"

  # 5. Combattant humain non pseudonymisé. `isControlledByAI=false` ⇒ un vrai compte de joueur,
  #    avec son identifiant numérique entre crochets sur la même ligne.
  n=$(grep -oE '\[_FL_\] fightId=[0-9]+ [^[]+ breed : [0-9]+ \[-?[0-9]+\] isControlledByAI=false' "$f" 2> /dev/null \
      | grep -cvE ' Anonyme-[A-Za-zÀ-ÿ]+[0-9]+ breed ' || true)
  [ "$n" -gt 0 ] && signaler "$f : $n entrée(s) en combat d'un joueur réel" \
    "attendu « Anonyme-<Classe><N> » — voir l'en-tête de tests/session_real_log.rs"
done

if [ "$ECHECS" -gt 0 ]; then
  cat >&2 <<'MSG'

╭──────────────────────────────────────────────────────────────────────╮
│ Des données réelles dans une fixture wakfu.log.                      │
│                                                                      │
│ Le dépôt est PUBLIC : un journal brut y publie un jeton de session,  │
│ une IP, un nom de compte Windows et les pseudonymes et messages de   │
│ tous les joueurs croisés ce jour-là.                                 │
│                                                                      │
│ Repseudonymiser avant de committer — voir docs/analyse-rgpd.md §3.1  │
│ et l'en-tête de crates/overlay-engine/tests/session_real_log.rs.     │
╰──────────────────────────────────────────────────────────────────────╯
MSG
  exit 1
fi

printf 'Fixtures wakfu.log : pseudonymisées.\n'
