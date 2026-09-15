#!/usr/bin/env bash
# Incrémente la version SemVer du produit d'après le TYPE Conventional Commits du commit qui vient
# d'être créé, puis AMENDE ce commit pour y inclure le changement.
#
# Lancé par le hook `post-commit` (`.githooks/post-commit`, activé par `scripts/install-hooks.sh`) ;
# se lance aussi à la main pour vérifier ce qu'il ferait :
#
#   bash scripts/bump-version.sh --dry-run
#
# ## Où vit la version
#
# `[workspace.package] version` du `Cargo.toml` racine, UNE seule fois : toutes les crates y
# renvoient (`version.workspace = true`), et Cargo l'embarque dans le binaire (`CARGO_PKG_VERSION`,
# voir `crates/overlay-ui/src/build_info.rs`). Ce script tient à jour, en plus du manifeste, les
# DEUX fichiers de verrouillage qui recopient ces numéros — `Cargo.lock` et `xtask/Cargo.lock`
# (xtask dépend de `overlay-engine`/`overlay-ingest` par chemin). Les laisser dériver ferait échouer
# tout `cargo build --locked`, et rendrait bruyant chaque `cargo build` ordinaire, qui les
# réécrirait dans le dos du commit suivant.
#
# ## Correspondance type → niveau de bump
#
#   BREAKING CHANGE (`type!:` ou footer `BREAKING CHANGE:`)  → major   (X.0.0)  — prioritaire
#   feat                                                      → minor   (x.Y.0)
#   fix, style, perf, refactor, test                          → patch   (x.y.Z)
#   docs, chore, ci, build                                    → aucun bump
#   message non conforme (merge, revert, sujet libre)         → aucun bump
#
# Le partage retenu (décision de l'utilisateur, 2026-09-14) : tout ce qui touche le binaire livré
# fait avancer le numéro, ce qui n'y change rien le laisse en place. Un `style:` reformate bien du
# code compilé — d'où son patch ; un `docs:` ne sort jamais du dépôt.
#
# ## Gardes
#
# - `SKIP_VERSION_BUMP=1 git commit …` : échappatoire ponctuelle (voir README).
# - `WAKFU_VERSION_BUMP_AMEND=1` : posée par ce script sur l'amend qu'il lance lui-même. Sans elle,
#   cet amend — qui EST un commit — relancerait `post-commit`, donc ce script, en boucle.
# - **Opération git en cours** (rebase, merge, cherry-pick, bisect) : aucun bump. Amender au milieu
#   d'un rebase rejoue un bump sur un commit qui en porte déjà un (bug vécu sur le dépôt web :
#   1.1.0 → 1.2.0 en rejouant un commit déjà versionné), et amender un commit de merge réécrit un
#   historique que personne n'a demandé à toucher.
# - **Commit de fusion** (deux parents ou plus) : aucun bump, même raison.
set -uo pipefail

cd "$(dirname "$0")/.."

DRY_RUN=0
case "${1:-}" in
  --dry-run) DRY_RUN=1 ;;
  '') ;;
  *) echo "usage : bash scripts/bump-version.sh [--dry-run]" >&2; exit 2 ;;
esac

log() { printf '\033[1m[version]\033[0m %s\n' "$*"; }

# ── Gardes ───────────────────────────────────────────────────────────────────────────────────────
if [ "${WAKFU_VERSION_BUMP_AMEND:-}" = "1" ]; then
  exit 0  # amend lancé par ce script : le bump est déjà dedans.
fi
if [ "${SKIP_VERSION_BUMP:-}" = "1" ]; then
  log "SKIP_VERSION_BUMP=1 — bump ignoré."
  exit 0
fi

git_dir="$(git rev-parse --git-dir 2>/dev/null)" || exit 0
for marqueur in rebase-merge rebase-apply MERGE_HEAD CHERRY_PICK_HEAD REVERT_HEAD BISECT_LOG; do
  if [ -e "$git_dir/$marqueur" ]; then
    log "opération git en cours ($marqueur) — bump ignoré."
    exit 0
  fi
done

# `rev-list --parents -n 1` liste le commit puis ses parents : plus de deux champs = fusion.
if [ "$(git rev-list --parents -n 1 HEAD 2>/dev/null | wc -w)" -gt 2 ]; then
  log "commit de fusion — bump ignoré."
  exit 0
fi

# ── Niveau de bump ───────────────────────────────────────────────────────────────────────────────
message="$(git log -1 --pretty=%B HEAD)"
# Première ligne non vide et non commentée : un éditeur de message peut préfixer des lignes `#`.
sujet="$(printf '%s\n' "$message" | grep -v '^[[:space:]]*#' | grep -m1 '[^[:space:]]' || true)"

niveau=""
if printf '%s\n' "$message" | grep -qE '^BREAKING[ -]CHANGE:' \
   || printf '%s\n' "$sujet" | grep -qE '^[a-zA-Z]+(\([^)]+\))?!:'; then
  niveau=major
elif printf '%s\n' "$sujet" | grep -qE '^[a-zA-Z]+(\([^)]+\))?:[[:space:]]'; then
  type="$(printf '%s\n' "$sujet" | sed -E 's/^([a-zA-Z]+).*/\1/' | tr '[:upper:]' '[:lower:]')"
  case "$type" in
    feat)                          niveau=minor ;;
    fix|style|perf|refactor|test)  niveau=patch ;;
    *)                             niveau="" ;;   # docs, chore, ci, build, type inconnu
  esac
fi

if [ -z "$niveau" ]; then
  exit 0  # silencieux : c'est le cas le plus fréquent (docs/chore), pas un incident.
fi

# ── Calcul ───────────────────────────────────────────────────────────────────────────────────────
actuelle="$(sed -nE 's/^version = "([0-9]+\.[0-9]+\.[0-9]+)"$/\1/p' Cargo.toml | head -1)"
if [ -z "$actuelle" ]; then
  echo "[version] version introuvable dans Cargo.toml ([workspace.package]) — bump abandonné." >&2
  exit 1
fi

IFS=. read -r major minor patch <<< "$actuelle"
case "$niveau" in
  major) major=$((major + 1)); minor=0; patch=0 ;;
  minor) minor=$((minor + 1)); patch=0 ;;
  patch) patch=$((patch + 1)) ;;
esac
nouvelle="$major.$minor.$patch"

if [ "$DRY_RUN" = "1" ]; then
  log "$actuelle → $nouvelle ($niveau, sujet : ${sujet:-—}) — essai à blanc, rien écrit."
  exit 0
fi

# ── Écriture ─────────────────────────────────────────────────────────────────────────────────────
# Manifeste racine : la PREMIÈRE ligne `version = "x.y.z"` du fichier est celle de
# `[workspace.package]` (le seul autre numéro du fichier est la version PATCHÉE de wgpu-hal, qui
# vit dans un chemin, pas dans un champ `version`). `0,/…/` borne la substitution à cette
# occurrence — sans lui, `sed` réécrirait toute ligne de même forme ajoutée plus tard.
sed -i -E "0,/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"$/s//version = \"$nouvelle\"/" Cargo.toml

# Fichiers de verrouillage : chaque crate du workspace y a un bloc `[[package]]` dont le champ
# `version` recopie celui du manifeste. On ne réécrit QUE les blocs des crates locales — jamais
# ceux des dépendances externes, dont les numéros n'ont rien à voir avec le nôtre.
membres="$(sed -nE 's/^name = "(overlay-[a-z-]+)"$/\1/p' crates/*/Cargo.toml | sort -u | tr '\n' ' ')"
maj_lock() {
  local lock="$1"
  [ -f "$lock" ] || return 0
  awk -v membres="$membres" -v v="$nouvelle" '
    BEGIN { split(membres, liste, " "); for (i in liste) est_local[liste[i]] = 1 }
    /^name = "/ {
      nom = $0; sub(/^name = "/, "", nom); sub(/"$/, "", nom)
      attend = (nom in est_local)
    }
    attend && /^version = "/ { print "version = \"" v "\""; attend = 0; next }
    { print }
  ' "$lock" > "$lock.tmp" && mv "$lock.tmp" "$lock"
  git add "$lock"
}
maj_lock Cargo.lock
maj_lock xtask/Cargo.lock
git add Cargo.toml

# ── Amend ────────────────────────────────────────────────────────────────────────────────────────
# `--no-verify` : les hooks de VALIDATION (pre-commit/commit-msg) ont déjà tourné pour ce commit,
# les rejouer ne vérifierait rien de neuf. `post-commit`, lui, se relance quand même (git l'exécute
# hors du périmètre de `--no-verify`) — c'est `WAKFU_VERSION_BUMP_AMEND` qui l'arrête.
# stderr de la PREMIÈRE tentative mis de côté : quand la signature est indisponible, git y écrit
# un « fatal: cannot exec … / failed to write commit object » qui n'est pas un incident — le repli
# juste en dessous règle le cas. Ne l'afficher que si les DEUX tentatives échouent, sinon le hook
# crie à l'erreur sur un commit qui s'est parfaitement bien passé.
erreur_signature="$(mktemp)"
if ! WAKFU_VERSION_BUMP_AMEND=1 git commit --amend --no-edit --no-verify --quiet 2> "$erreur_signature"; then
  # Repli documenté par CLAUDE.md : en session cloud, le programme de signature (`gpg.ssh.program`,
  # script temporaire) est parfois absent et `git commit` échoue sur `failed to write commit
  # object`. Committer sans signature vaut mieux que laisser le dépôt avec un bump indexé mais
  # jamais committé, que le commit SUIVANT emporterait à tort.
  if ! WAKFU_VERSION_BUMP_AMEND=1 git commit --amend --no-edit --no-verify --no-gpg-sign --quiet; then
    cat "$erreur_signature" >&2
    rm -f "$erreur_signature"
    echo "[version] amend impossible — bump annulé (fichiers restaurés)." >&2
    git checkout -- Cargo.toml Cargo.lock xtask/Cargo.lock 2>/dev/null
    exit 1
  fi
  log "commit amendé SANS signature (programme de signature indisponible)."
fi
rm -f "$erreur_signature"

log "$actuelle → $nouvelle ($niveau)"
