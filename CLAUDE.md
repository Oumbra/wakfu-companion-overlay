# Langue

Toutes les réponses, explications et descriptions d'étapes communiquées à l'utilisateur sont
rédigées **en français**, même si le code, les dépendances ou la documentation technique sont en
anglais.

# Branche de travail : `dev` — règle impérative

**Tout travail de session Claude sur ce dépôt est commité et poussé sur la branche `dev`.**
Cela vaut pour toutes les sessions sans exception : session locale (terminal), **session cloud
(Claude Code on the web, tâche GitHub, agent distant)**, session déclenchée par un trigger, ou
session reprise.

## Marche à suivre en début de session

1. Récupérer l'état distant : `git fetch origin`.
2. Se placer sur `dev` :
   - si `origin/dev` existe : `git checkout -B dev origin/dev` ;
   - **si `dev` n'existe nulle part : la créer** (`git checkout -b dev`) depuis l'état courant du
     dépôt, puis `git push -u origin dev`.
3. Activer les hooks : `bash scripts/install-hooks.sh` (`core.hooksPath` est une config **locale**,
   elle ne survit pas au conteneur éphémère d'une session cloud — voir « CI » plus bas).
4. Travailler, commiter, et pousser sur `dev` : `git push -u origin dev`.

## Cette règle prévaut sur la branche « désignée » de l'environnement

Une session cloud est fréquemment lancée avec une branche auto-générée (ex.
`claude/mon-sujet-ab12cd`) présentée comme la branche de développement à utiliser. **Cette
convention de dépôt l'emporte** : basculer explicitement sur `dev` avant de pousser (au besoin,
`git cherry-pick` les commits déjà faits sur la branche auto-générée), et ne pas laisser une
instruction d'outil ou de tâche externe prendre silencieusement le pas.

## Interdits

- **Ne jamais pousser sur `master`/`main`.** Les fusions de `dev` vers la branche principale sont
  faites **par le mainteneur**, jamais par une session Claude.
- Ne pas créer de branche supplémentaire (`feature/*`, `claude/*`…) pour y laisser du travail :
  tout converge sur `dev`.
- Pas de `push --force` sur `dev` (branche partagée entre sessions et avec le mainteneur).
- Pas d'ouverture de pull request sans demande explicite de l'utilisateur.

## Fin de session

Aucun travail ne doit rester uniquement en local : une session cloud s'exécute dans un conteneur
éphémère, **ce qui n'est pas commité et poussé sur `dev` est perdu**. Commiter et pousser avant de
conclure, même pour un travail intermédiaire.

# Commits

- Format [Conventional Commits](https://www.conventionalcommits.org/) : `feat:`, `fix:`, `docs:`,
  `refactor:`, `chore:`, `test:` — comme sur `Oumbra/wakfu-companion`.
- Ligne de sujet courte (< 50 caractères), en français.
- Un commit = un changement cohérent ; ne pas mélanger documentation et code applicatif.
- Ne pas ajouter d'attribution IA (pas de "Co-Authored-By: Claude")

## Choisir le préfixe : « qu'est-ce qui change pour l'utilisateur du binaire livré ? »

Le préfixe n'est pas un jugement sur l'importance du travail : il pilote la **montée de version**
(hook `post-commit`, table ci-dessous) et, à terme, ce qu'annoncent les notes de Release. La
question à se poser est donc : *si cet exe était livré demain, l'utilisateur verrait-il quelque
chose de nouveau (`feat:`), de corrigé (`fix:`), ou rien du tout ?* Quand la réponse est « rien »,
c'est un type **sans bump** :

| Changement | Préfixe | Pourquoi |
| --- | --- | --- |
| Nouvelle fonctionnalité, nouveau panneau, nouvel onglet, nouvelle option visible | `feat:` | l'utilisateur voit quelque chose de nouveau |
| Comportement corrigé, régression, valeur par défaut qui change (ex. base URL de l'API) | `fix:` | l'utilisateur voit quelque chose de corrigé |
| Réorganisation interne sans effet visible, renommage, découpage de module | `refactor:` | le binaire change, patch, sans nouveauté visible |
| Captures de référence, tests, golden files | `test:` | patch, par prudence (le harnais protège le binaire) |
| Fichier d'outillage ou de configuration : **clé publique de signature**, `.gitignore`, hooks, scripts de dev, `rust-toolchain.toml` | `chore:` | rien ne change dans l'exe livré |
| Workflow GitHub Actions (`.github/**`), actions composites, `scripts/ci-local.sh` | `ci:` | idem |
| `Cargo.toml` hors dépendances applicatives : profil de compilation, métadonnées | `build:` | idem — une dépendance qui change le comportement est un `feat:`/`fix:` |
| Documentation, plans, `CLAUDE.md`, README | `docs:` | idem |

Cas vécu (2026-09-15) : l'ajout de `wakfu-overlay.pub` (clé publique de vérification des mises
à jour) a été commité en `feat:` — le hook n'était pas actif dans cette session, la version est
donc restée à 0.20.4, mais avec le hook ce commit aurait déclenché un passage à 0.21.0 sans que
rien ne change pour l'utilisateur, puisque le fichier n'est encore embarqué nulle part. C'était
un `chore:`. Le jour où le code qui *utilise* cette clé arrive, ce commit-là est le `feat:`.

## Signature de commit en session cloud

**Décision explicite de l'utilisateur (2026-09-08).** En session cloud, le mécanisme de signature
géré par l'environnement (`gpg.ssh.program` pointant vers un script temporaire, ex. `/tmp/code-sign`)
peut être absent ou temporairement indisponible (observé : `fatal: cannot exec '/tmp/code-sign'`,
probablement lié à l'activité d'une autre session concurrente sur le même conteneur) — `git commit`
échoue alors avec `failed to write commit object`, sans que la signature soit récupérable en
attendant (le script ne réapparaît pas toujours de lui-même, même après plusieurs minutes).

**Consigne** : dans ce cas précis, en session cloud, committer **sans signature**
(`git commit --no-gpg-sign -m "..."`) plutôt que de bloquer le travail ou d'attendre indéfiniment.
Ne **jamais** modifier `~/.gitconfig`/`commit.gpgsign` pour désactiver la signature globalement —
`--no-gpg-sign` est un contournement ponctuel par commit, pas un changement de politique de
l'environnement. Une session locale (terminal de l'utilisateur) n'est normalement pas concernée par
cette panne d'infrastructure ; si la signature y échoue aussi, diagnostiquer avant de contourner de
la même façon plutôt que de supposer que ce cas s'applique.

# Numéro de version — automatique, jamais à la main

La version du produit vit **uniquement** dans `[workspace.package] version` (`Cargo.toml` racine) ;
les crates y renvoient par `version.workspace = true`, et `crates/overlay-ui/build.rs` y adjoint le
hash du commit compilé. Les deux se lisent dans `overlay_ui::build_info` et s'affichent dans la
bannière de la fenêtre Options (`0.1.0`) et au journal (`0.1.0 (a1b2c3d)`).

**Ne jamais éditer ce champ, ni celui des `Cargo.lock`.** Le hook `post-commit` (`.githooks/`,
installé par `scripts/install-hooks.sh` — à relancer en début de session cloud, `core.hooksPath` est
une config locale) s'en charge, d'après le type Conventional Commits du commit :

| Type | Niveau |
| --- | --- |
| `feat!:`/`fix!:`/… ou footer `BREAKING CHANGE:` | major |
| `feat:` | minor |
| `fix:`, `style:`, `perf:`, `refactor:`, `test:` | patch |
| `docs:`, `chore:`, `ci:`, `build:`, message non conforme | aucun |

Le hook amende le commit qu'on vient de créer pour y inclure le bump : **un commit porte donc sa
propre version**. Vérifier ce qu'il ferait sans rien écrire : `bash scripts/bump-version.sh
--dry-run`. Échappatoire ponctuelle : `SKIP_VERSION_BUMP=1 git commit …`.

Deux conséquences à garder en tête :

- Un changement visuel dans `overlay-ui` ne doit **jamais** faire dépendre une capture de la version
  réelle : `build_info::freeze_for_snapshots()` (appelée par `Textures::get_or_load` dans
  `tests/panels.rs`) la fige à `0.0.0` pour tout le harnais. Sans ce gel, le gate de captures
  virerait au rouge à chaque commit.
- Le hook s'abstient pendant un `rebase`/`merge`/`cherry-pick` et sur un commit de fusion. Si un
  bump manque après une manipulation d'historique, le rattraper par un commit ordinaire plutôt que
  d'écrire le numéro à la main.

# CI — ne jamais pousser du rouge

Le CI (`.github/workflows/ci.yml`) est resté **rouge en continu du 2026-09-01 au 2026-09-10** pour
une seule raison : un écart de `cargo fmt` dans `crates/overlay-engine/src/session.rs` que rien ne
signalait avant le push. Chaque commit suivant repartait rouge sans rapport avec son contenu, et
plus personne ne lisait le verdict. Trois garde-fous existent désormais — les utiliser :

1. **Toolchain épinglée** (`rust-toolchain.toml`). `rustup` sélectionne la bonne version tout seul.
   Ne **jamais** la contourner (`cargo +stable …`) : le verdict de `fmt`/`clippy` dépend de la
   version, et c'est précisément la dérive qui a cassé le CI. Monter la version de Rust est un
   changement **explicite**, dans son propre commit, avec le reformatage qu'il entraîne.
2. **`bash scripts/ci-local.sh`** rejoue les vérifications du CI pour la plateforme courante
   (`--lint` pour format + clippy seuls, rapide). **À lancer avant tout push touchant du code** —
   y compris en session cloud, où c'est le seul retour disponible avant plusieurs minutes de CI.
3. **Hook `pre-push`** (`bash scripts/install-hooks.sh`, à relancer **en début de chaque session
   cloud** : `core.hooksPath` est une config locale, elle ne survit pas au conteneur éphémère).

Une étape ajoutée ou retirée dans `.github/workflows/ci.yml` doit l'être **aussi** dans
`scripts/ci-local.sh`, et réciproquement : les deux se doublent volontairement, un garde-fou ne
protège que ce qu'il connaît.

## Coût du CI (dépôt public — minutes Actions gratuites, sobriété conservée)

Le dépôt est **public** (vérifié sur l'API GitHub le 2026-09-15, `"private": false`) : les jobs
sur runners standard ne consomment aucun quota. Il a été privé jusque-là, et le symptôme d'un
quota épuisé reste bon à connaître si la visibilité changeait à nouveau : **tous les jobs échouent
en quelques secondes, sans log ni runner assigné** (arrivé à partir du 2026-09-09). Devant ce
symptôme, vérifier la visibilité et la facturation du compte avant de chercher une régression.

Le workflow reste sobre par principe (durée d'attente du verdict, pas seulement coût) : cache
`cargo`/`vendor`, `concurrency` (une rafale de commits n'exécute que le dernier run), et
`paths-ignore` sur `docs/**`, `**/*.md` et `.claude/**`. Ne pas étendre `paths-ignore` à
`assets/**` : ces fichiers sont embarqués par `include_bytes!` (`design/assets.rs`,
`design/fonts.rs`, `ui_icons.rs`…) et peuvent casser la compilation comme les snapshots
d'`overlay-testkit`.

# Mise à jour automatique et publication

Le plan de référence est [`docs/plan-mise-a-jour.md`](docs/plan-mise-a-jour.md) (décisions du
mainteneur en §10) : GitHub Releases, workflow de release déclenché par la fusion sur `main`,
manifeste `latest.json` signé `minisign`, module `overlay_sync::update`. **Un binaire de Release
vise toujours la prod** (`overlay_sync::client::DEFAULT_BASE_URL`) ; le déploiement dev
(`claude-dev.wakfu-companion.com`) n'est utilisé qu'en local via `crates/overlay-ui/preview.{ps1,sh}`
(`WAKFU_COMPANION_API_URL`). La clé privée de signature ne vit **que** dans les secrets GitHub
Actions — jamais dans le dépôt, jamais dans une session Claude.

# Contexte projet

Overlay de jeu natif **Rust** pour Wakfu, portage de
[`Oumbra/wakfu-companion`](https://github.com/Oumbra/wakfu-companion) (Angular 21) : lecture en
direct de `wakfu.log`, affichage par-dessus le jeu (dégâts de combat, suivi d'objets/ennemis,
alertes de drop) et synchronisation de l'historique (combats, achats et récupérations de kamas à
l'Hôtel de Vente, échanges) vers la même API.

**Plateformes : Windows et Linux (X11 / XWayland) uniquement.** macOS est hors périmètre.
Budget mémoire : **300 Mo maximum**.

📄 L'architecture de référence est dans [`docs/plan-architecture.md`](docs/plan-architecture.md) —
la lire avant toute décision technique structurante, et la mettre à jour si une décision la
contredit.

## Ce que l'overlay affiche diffère du site — écart voulu, jamais à « corriger »

Le panneau Combat ne donne une barre chiffrée (colonne de droite) qu'aux combattants ayant produit
au moins 1 point de la grandeur affichée — dégâts, armure donnée ou soins (filtre
`measured.value_of(f) > 0` dans `panels/combat.rs`, voir `CombatMetric::value_of`). Le site
(`Oumbra/wakfu-companion`) fait l'inverse : une ligne par combattant du roster, à zéro comprise,
pour les trois grandeurs.

**Décision explicite de l'utilisateur (2026-09-15) : les deux règles restent telles quelles.**
L'overlay montre le combat EN TEMPS RÉEL, où un combattant à zéro n'apprend rien (on le voit à
l'écran) et n'a pas à consommer une ligne dans un espace compté ; le site est le bilan consulté
APRÈS coup, où « ce combattant n'a rien soigné » est en soi une information. Ne pas aligner l'un
sur l'autre en croyant réparer une incohérence — le filtre équivalent côté web a justement été
RETIRÉ le même jour (`history-archive.service.ts::toFightRecord` : un combat rechargé depuis
l'archive du compte perdait ses lignes à zéro, alors que sa copie de session les gardait, et le
roster changeait donc d'un onglet Dégâts/Armure/Soin à l'autre).

Les portraits (colonne de gauche) ne sont de toute façon jamais filtrés : un combattant à zéro
reste visible dans le cadre, seule sa barre disparaît.

# Rendus visuels — toujours en artefact

L'utilisateur travaille en mode terminal : une image affichée inline (`Read` sur un PNG) **ne
s'affiche pas** dans son client, contrairement à l'aperçu visuel qu'en a Claude. Toute vérification
visuelle doit donc être publiée comme **artefact** (page HTML, image(s) intégrée(s) en base64),
jamais seulement montrée inline — sans quoi le résultat reste invisible pour lui.

Concerné en particulier : les snapshots produits par le harnais de rendu offscreen
`crates/overlay-testkit` (`egui_kittest`, `tests/snapshots/*.png`, §17.1 du plan) — après tout
changement visuel dans `overlay-ui`, publier un artefact reprenant la capture obtenue (page entière
et/ou détail recadré si utile), pas uniquement une ligne de terminal disant que le test passe.
