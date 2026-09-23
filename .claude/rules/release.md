---
paths:
  - "crates/overlay-sync/**"
  - ".github/workflows/release.yml"
  - "docs/plan-mise-a-jour.md"
  - "wakfu-overlay.pub"
  - "crates/overlay-ui/preview.*"
---

# release

Portée : mise à jour automatique de l'overlay, publication d'une Release, signature du manifeste,
base URL de l'API selon le profil. Tout nouvel apprentissage sur ce sujet se documente ici, jamais
dans `CLAUDE.md`.

## Mise à jour automatique et publication

Le plan de référence est [`docs/plan-mise-a-jour.md`](../../docs/plan-mise-a-jour.md) (décisions
du mainteneur en §10) : GitHub Releases, workflow de release déclenché par la fusion sur `main`,
manifeste `latest.json` signé `minisign`, module `overlay_sync::update`.

**Un binaire de Release vise toujours la prod** (`overlay_sync::client::DEFAULT_BASE_URL`) ; le
déploiement dev (`claude-dev.wakfu-companion.com`) n'est utilisé qu'en local via
`crates/overlay-ui/preview.{ps1,sh}` (`WAKFU_COMPANION_API_URL`).

La clé privée de signature ne vit **que** dans les secrets GitHub Actions — jamais dans le dépôt,
jamais dans une session Claude. Depuis l'audit de sécurité du 2026-09-23, le job `publish` de
`release.yml` est rattaché à l'**environnement GitHub `release`** : `MINISIGN_SECRET_KEY` et
`MINISIGN_PASSWORD` doivent être des secrets de cet environnement (pas du dépôt), et l'environnement
restreint à la branche `main` — sinon n'importe quel workflow poussé sur `dev` (session Claude
comprise) peut lire la clé qui signe les mises à jour de tous les utilisateurs. Toute action tierce
s'épingle par SHA de commit, avec la version en commentaire (`uses: owner/action@<sha> # vX.Y.Z`). La clé publique (`wakfu-overlay.pub`) est un fichier d'outillage
tant qu'aucun code ne l'embarque : son ajout était un `chore:`, pas un `feat:` (cas vécu dans
`.claude/rules/versioning.md`).
