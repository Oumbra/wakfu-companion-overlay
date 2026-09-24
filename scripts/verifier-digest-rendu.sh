#!/usr/bin/env bash
# Vérifie que l'IMAGE DE BASE du rendu est la même PARTOUT où elle est écrite.
#
# Le digest de `ubuntu:24.04@sha256:…` est recopié dans trois fichiers, et il n'existe aucun moyen
# de le factoriser : la clé `container.image` d'un workflow GitHub n'accepte ni variable de dépôt
# ni sortie d'étape, et un `ARG` de Dockerfile ne se lit pas depuis un YAML. La duplication est donc
# subie, pas choisie — ce script est ce qui l'empêche de dériver.
#
# CE QU'IL PROTÈGE, très précisément. Les références de `crates/overlay-testkit/tests/snapshots/`
# sont comparées pixel à pixel, et le rendu vient de lavapipe, donc de la version de Mesa, donc de
# l'image de base. Le Dockerfile le dit déjà : « les deux valeurs doivent bouger ensemble — un
# digest changé d'un seul côté fait diverger le rendu local de celui qui décide du gate ». Jusqu'ici
# rien ne le vérifiait : on l'aurait découvert par un gate rouge sur des captures régénérées « dans
# l'environnement du CI » qui n'en était plus un. Avec un troisième fichier
# (`regen-captures.yml`, qui RÉÉCRIT les références), le risque n'était plus théorique.
#
# Lancé par : le job `fmt` de `.github/workflows/ci.yml` et `scripts/ci-local.sh` (toutes plates-
# formes — c'est du texte, il n'y a rien à compiler).
#
#   bash scripts/verifier-digest-rendu.sh
set -uo pipefail
cd "$(dirname "$0")/.."

# Tout fichier ajouté ici doit contenir au moins une référence à l'image de base ; l'absence est
# une erreur, pas un silence (un fichier renommé sortirait sinon du périmètre sans bruit).
FICHIERS=(
  .github/workflows/ci.yml
  .github/workflows/regen-captures.yml
  .github/ci-image/Dockerfile
)

MOTIF='ubuntu:24\.04@sha256:[0-9a-f]{64}'

erreurs=0
reference=""
fichier_reference=""

for fichier in "${FICHIERS[@]}"; do
  if [ ! -f "$fichier" ]; then
    echo "ERREUR : $fichier introuvable — la liste de ce script est périmée." >&2
    erreurs=$((erreurs + 1))
    continue
  fi

  # `sort -u` : un même fichier peut légitimement citer l'image plusieurs fois (le digest apparaît
  # dans la doc d'en-tête du Dockerfile comme dans son `ARG`). Ce qui compte est qu'il n'en cite
  # jamais DEUX différents.
  mapfile -t trouves < <(grep -oE "$MOTIF" "$fichier" | sort -u)

  if [ "${#trouves[@]}" -eq 0 ]; then
    echo "ERREUR : $fichier ne mentionne aucune image de base ($MOTIF)." >&2
    erreurs=$((erreurs + 1))
    continue
  fi
  if [ "${#trouves[@]}" -gt 1 ]; then
    echo "ERREUR : $fichier mentionne ${#trouves[@]} images de base différentes :" >&2
    printf '  %s\n' "${trouves[@]}" >&2
    erreurs=$((erreurs + 1))
    continue
  fi

  if [ -z "$reference" ]; then
    reference="${trouves[0]}"
    fichier_reference="$fichier"
  elif [ "${trouves[0]}" != "$reference" ]; then
    echo "ERREUR : image de base divergente." >&2
    echo "  $fichier_reference : $reference" >&2
    echo "  $fichier : ${trouves[0]}" >&2
    erreurs=$((erreurs + 1))
  fi
done

if [ "$erreurs" -ne 0 ]; then
  cat >&2 <<'MSG'

Le rendu des captures dépend de cette image. Changer le digest est un geste EXPLICITE, dans son
propre commit, dans TOUS les fichiers à la fois — et il faut régénérer les références dans le
nouvel environnement (workflow « Régénérer les captures », ou
`UPDATE_SNAPSHOTS=1 bash scripts/ci-local.sh --captures-conteneur`).
Voir l'en-tête de scripts/setup-render-env.sh.
MSG
  exit 1
fi

echo "image de base du rendu, identique dans ${#FICHIERS[@]} fichiers : $reference"
