---
name: release
description: Mettre en production l'overlay Wakfu Companion — crée la branche `release-AAAA-MM-JJ` depuis `main`, y fusionne `dev` en résolvant les conflits, vérifie, pousse ; le workflow `release-pr.yml` ouvre alors la PR vers `main`, rejoue tout le CI, fusionne quand il est vert et lance `release.yml` (binaires Windows/Linux, manifeste signé, Release GitHub que l'overlay installe seul chez les utilisateurs). À utiliser quand l'utilisateur demande une mise en prod / release / publication d'une version. Fonctionne en local comme en session cloud (seul `git` est nécessaire, pas `gh`).
---

# Mise en production (branche de release)

On ne pousse jamais sur `main`. Le skill prépare une branche de release **déjà fusionnable sans
conflit** ; tout le reste (PR, CI complet, fusion, publication, suppression de la branche) est fait
par `.github/workflows/release-pr.yml`, puis `.github/workflows/release.yml`. Seul `git` est
nécessaire — pas de `gh`, pas de clé API.

**Ce qui arrive sur `main` part chez tous les utilisateurs** : `release.yml` compile les binaires,
signe `latest.json` avec la clé `minisign`, et l'overlay de chacun se met à jour tout seul. Ne
lancer ce skill que **sur demande explicite de l'utilisateur**, jamais de sa propre initiative ni
pour « tester ».

Cette procédure est l'**exception explicite** à deux règles de `CLAUDE.md` : « tout converge sur
`dev` » (la branche de release est créée, poussée, puis supprimée par le workflow) et « les fusions
de `dev` vers `main` sont faites par le mainteneur » (c'est lui qui la déclenche en demandant la
release).

## 1. Préconditions

```bash
git status --porcelain            # doit être vide : sinon s'arrêter et demander à l'utilisateur
git fetch origin main dev --tags
git rev-list --count origin/main..origin/dev
```

- `0` ⇒ rien à livrer : le dire à l'utilisateur et s'arrêter.
- Prévisualiser les conflits sans rien toucher : `git merge-tree --write-tree --name-only
  origin/main origin/dev` (1re ligne = arbre résultat ; les lignes suivantes, s'il y en a, =
  fichiers en conflit).
- Version qui sera publiée : la première ligne `version = "x.y.z"` de `Cargo.toml` sur `dev`
  (`git show origin/dev:Cargo.toml | grep -m1 -E '^version = '`). Si le tag `vx.y.z` existe déjà
  (`git ls-remote --tags origin refs/tags/vx.y.z`), `release.yml` **ne publiera rien** : aucun
  `feat:`/`fix:`/`refactor:`/`test:` depuis la dernière Release (docs, CI, outillage seulement).
  Le signaler à l'utilisateur et lui demander s'il veut quand même ramener `main` au niveau de
  `dev` (fusion sans nouvelle Release) ; ne jamais « corriger » en éditant la version à la main.
- `origin/dev` doit avoir un CI vert sur son dernier commit (onglet Actions, workflow `CI`) : une
  release ne sert pas à découvrir un rouge. Si ce n'est pas vérifiable depuis la session, le
  mentionner dans le rapport — le job `ci` de `release-pr.yml` le rejouera de toute façon.

## 2. Créer la branche

Nom : `release-$(date +%F)` (ex. `release-2026-09-24`). Si elle existe déjà sur `origin`
(`git ls-remote --heads origin <nom>`), suffixer `-2`, `-3`... Format imposé : `release-pr.yml`
ignore toute branche qui ne respecte pas `^release-[0-9]{4}-[0-9]{2}-[0-9]{2}(-[0-9]+)?$`.

```bash
git switch -c release-AAAA-MM-JJ origin/main
SKIP_VERSION_BUMP=1 git merge --no-ff --no-commit origin/dev
```

`--no-ff` : toujours un vrai commit de merge (c'est son message qui porte le rapport).
`SKIP_VERSION_BUMP=1` : aucun bump sur une release (le hook `post-commit` s'abstient déjà sur un
commit de fusion, la variable est une ceinture de plus — voir `.claude/rules/versioning.md`) ; la
version livrée est celle de `dev`.

En session cloud, si `git commit` échoue sur `/tmp/code-sign`, ajouter `--no-gpg-sign` (règle de
`CLAUDE.md`) à **chaque** commit de cette procédure.

## 3. Résoudre les conflits (s'il y en a)

`dev` est la source de vérité ; `main` ne reçoit normalement rien d'autre que des fusions de `dev`.
Un conflit vient donc d'un correctif poussé directement sur `main` par le mainteneur ou d'une
réécriture d'historique.

- Lire chaque conflit (`git diff`, `git log --oneline origin/dev..origin/main -- <fichier>` pour
  savoir ce que `main` a apporté) et **préserver l'intention des deux côtés**. Ne pas prendre
  `--theirs`/`--ours` en bloc sans avoir compris le conflit.
- `main` porte des copies de commits de `dev` (autres SHA, mêmes sujets) : comparer
  `git rev-parse origin/main^{tree}` aux arbres des ancêtres de `dev` ; si l'un est identique, la
  bonne résolution est de prendre intégralement `dev`
  (`git restore --source=origin/dev --staged --worktree :/`, qui retire aussi les fichiers absents
  de `dev`).
- `Cargo.toml` (`[workspace.package] version`) / `Cargo.lock` : garder la version de `dev`, sauf si
  `main` porte une version supérieure (correctif direct) — dans ce cas s'arrêter et la signaler à
  l'utilisateur : publier une version inférieure à la dernière Release casserait la mise à jour
  automatique. Ne jamais écrire de numéro à la main.
- Captures de référence (`crates/overlay-testkit/tests/snapshots/*.png`, binaires, non
  fusionnables) : prendre celles de `dev`, qui correspondent à son code ; ne jamais les régénérer
  hors de l'environnement du CI (`.claude/rules/captures.md`).
- `wakfu-overlay.pub` en conflit : **s'arrêter et demander**. Une clé publique qui ne correspond plus
  au secret de l'environnement `release` fait échouer la publication, et une mauvaise clé livrée
  empêche toute mise à jour future chez les utilisateurs (`.claude/rules/release.md`).
- Workflows (`.github/**`) : garder les deux intentions, puis relire la règle
  `.claude/rules/ci.md` (symétrie `ci.yml` / `scripts/ci-local.sh`).
- Choix ambigu (logique métier, deux correctifs concurrents) : **demander à l'utilisateur** plutôt
  que trancher seul.
- Ensuite, vérifier localement (obligatoire dès qu'il y a eu un conflit dans du code ; le CI
  rejouera de toute façon) — conteneur neuf : préparer d'abord l'environnement comme indiqué dans
  `CLAUDE.md` (`bash patches/setup-vendor.sh`, `apt-get install libasound2-dev pkg-config
  mesa-vulkan-drivers`, `bash scripts/install-hooks.sh`) :

```bash
bash scripts/ci-local.sh
```

Sans conflit, ces vérifications locales sont facultatives : le job `ci` de `release-pr.yml` joue
tout `ci.yml` (Windows compris) avant toute fusion.

## 4. Commit de merge = rapport

Le **corps** du message devient la description de la PR (job `open-pr`) : le rédiger pour
l'utilisateur, en prose normale. Les notes de la Release GitHub, elles, sont générées par
`release.yml` à partir des sujets `feat:`/`fix:` — inutile de les recopier ici.

```bash
SKIP_VERSION_BUMP=1 git commit -F - <<'EOF'
Merge dev into main (release AAAA-MM-JJ)

## Version publiée
vx.y.z (dernière Release : vX.Y.Z)
<ou « aucune nouvelle Release : vx.y.z existe déjà, fusion de mise à niveau seulement »>

## Contenu livré
<git log --oneline --no-merges origin/main..origin/dev, regroupé par type : feat / fix / autres>

## Conflits
Aucun.
<ou, par fichier : origine du conflit (commit côté main), résolution retenue et pourquoi>

## Vérifications locales
<commandes lancées et résultat, ou « aucune (pas de conflit), CI complet de release-pr.yml »>
EOF
```

Le commit de merge doit être le **dernier** commit de la branche (c'est le message du commit de tête
du push qui est lu). Une correction nécessaire après coup : `SKIP_VERSION_BUMP=1 git commit
--amend`, pas un commit de plus. Un bug trouvé dans le code livré se corrige sur `dev` (commit
`fix:` normal, avec son bump), puis on recommence la release.

## 5. Pousser

```bash
git push -u origin release-AAAA-MM-JJ
```

Le hook `pre-push` rejoue `ci-local.sh --lint` : s'il échoue, le problème est dans `dev` (ou dans
la résolution) — le corriger, jamais `--no-verify`.

Si le push est refusé (proxy git d'une session cloud, classificateur auto-mode « Production
Deploy »...), ne pas contourner ni renommer la branche : donner la commande de push exacte à
l'utilisateur et signaler le refus.

## 6. Ramener la résolution dans `dev` (seulement si nécessaire)

```bash
git diff --quiet origin/dev HEAD && echo identique
```

- Arbre identique à `dev` ⇒ rien à faire.
- Sinon (résolution de conflit ou correctif de `main` absent de `dev`) : fusionner la branche de
  release dans `dev` pour que la prochaine release ne rejoue pas le même conflit (fusion simple, pas
  de `push --force` sur `dev`) :

```bash
git switch dev && git pull --ff-only origin dev
SKIP_VERSION_BUMP=1 git merge --no-ff release-AAAA-MM-JJ -m "chore: report de la release AAAA-MM-JJ dans dev"
git push origin dev
```

## 7. Rapport à l'utilisateur

- Nom de la branche poussée et lien :
  `https://github.com/Oumbra/wakfu-companion-overlay/pulls?q=is%3Apr+head%3A<branche>`.
- Version publiée (ou absence de nouvelle Release), résumé du contenu livré, conflits et
  résolutions, vérifications.
- Suite automatique : PR ⇒ CI complet (`ci.yml` appelé par `release-pr.yml`) ⇒ fusion ⇒
  `release.yml` (binaires, `latest.json` signé, Release `vx.y.z`) ⇒ suppression de la branche.
  Suivi : `https://github.com/Oumbra/wakfu-companion-overlay/actions`, résultat :
  `https://github.com/Oumbra/wakfu-companion-overlay/releases`.
- Revenir sur `dev` en local (`git switch dev`) et supprimer la branche locale de release
  (`git branch -D release-AAAA-MM-JJ`).

## Si le workflow ne fusionne pas

- Job `ci` rouge : corriger sur `dev` (ou amender le merge si l'erreur vient de la résolution),
  repousser. Captures en écart : les régénérer sur `dev` avec le workflow « Régénérer les
  captures », jamais sur la branche de release.
- PR non créée : vérifier le réglage « Allow GitHub Actions to create and approve pull requests »
  (Settings → Actions → General).
- Fusion refusée par une règle de protection de `main` qui exige des checks nommés : les checks de
  ce run s'appellent `ci / fmt`, `ci / test-linux`... ; les ajouter à la règle (réglage du dépôt, à
  faire par le mainteneur), ou fusionner à la main.
- `main` a bougé entre-temps et la PR est en conflit : recommencer la release avec un suffixe `-2`.
- Fusion faite mais pas de Release : `release.yml` a vu le tag déjà existant (version inchangée,
  voir §1), ou le job `publish` a échoué (secrets `MINISIGN_*` de l'environnement `release`, voir
  `.claude/rules/release.md`). Relancer à la main : Actions → Release → « Run workflow » sur `main`.
- En dernier recours, la PR peut toujours être fusionnée à la main sur GitHub (« Create a merge
  commit ») : le push qui en résulte, fait au nom de l'utilisateur, déclenche `release.yml`
  normalement (supprimer ensuite la branche de release).
