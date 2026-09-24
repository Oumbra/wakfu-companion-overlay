---
paths:
  - "crates/overlay-sync/**"
  - ".github/workflows/release.yml"
  - ".github/workflows/release-pr.yml"
  - ".claude/skills/release/**"
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

Deux chemins mènent `dev` sur `main`, donc à une Release : la fusion à la main par le mainteneur
(push sur `main`, qui déclenche `release.yml`), ou le skill `release` (2026-09-24) — branche
`release-AAAA-MM-JJ` créée depuis `main` avec `dev` fusionnée dedans, puis `release-pr.yml` ouvre
la PR, **appelle `ci.yml` en workflow réutilisable** (pas de `paths-ignore`, pas de dépendance à la
version de la branche par défaut comme avec `workflow_run`), fusionne par le GITHUB_TOKEN et lance
`release.yml` par `workflow_dispatch` (une fusion par ce jeton ne déclenche pas `push`). Si `main`
reçoit un jour une règle de protection avec checks requis, ceux de ce chemin s'appellent
`ci / <job>`.

**Un binaire de Release vise toujours la prod** (`overlay_sync::client::DEFAULT_BASE_URL`) ; le
déploiement dev (`claude-dev.wakfu-companion.com`) n'est utilisé qu'en local via
`crates/overlay-ui/preview.{ps1,sh}` (`WAKFU_COMPANION_API_URL`).

La clé privée de signature ne vit **que** dans les secrets GitHub Actions — jamais dans le dépôt,
jamais dans une session Claude. Depuis l'audit de sécurité du 2026-09-23, le job `publish` de
`release.yml` est rattaché à l'**environnement GitHub `release`** : `MINISIGN_SECRET_KEY` et
`MINISIGN_PASSWORD` doivent être des secrets de cet environnement (pas du dépôt), et l'environnement
restreint à la branche `main` — sinon n'importe quel workflow poussé sur `dev` (session Claude
comprise) peut lire la clé qui signe les mises à jour de tous les utilisateurs. Toute action tierce
s'épingle par SHA de commit, avec la version en commentaire (`uses: owner/action@<sha> # vX.Y.Z`).
Dans le job `publish`, `xtask` est **compilé dans une étape sans secret** puis seul le binaire
compilé (`xtask/target/release/xtask dist`) s'exécute avec `MINISIGN_*` : un `cargo run` dans
l'étape signée exposait la clé à tous les `build.rs`/proc-macros des dépendances de xtask. Ne pas
réintroduire de `cargo run`/`cargo build` dans une étape qui reçoit un secret ; les checkouts
des jobs de compilation et de publication se font en `persist-credentials: false` (idem `regen-captures.yml`, où seule l'étape de push
reçoit le jeton). Dependabot (`.github/dependabot.yml`) propose les montées d'actions et de
crates sur `dev`, cooldown 7 jours, jamais fusionnées automatiquement.

La clé publique (`wakfu-overlay.pub`) est un fichier d'outillage
tant qu'aucun code ne l'embarque : son ajout était un `chore:`, pas un `feat:` (cas vécu dans
`.claude/rules/versioning.md`).
