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

## Coût du CI (dépôt privé — minutes GitHub Actions comptées)

Le dépôt est **privé** : chaque run consomme le quota Actions du compte, les jobs `windows-latest`
étant facturés **au double** des jobs Linux. Un quota épuisé ne se voit pas comme une erreur de
build — **tous les jobs échouent en quelques secondes, sans log ni runner assigné** (c'est ce qui
est arrivé à partir du 2026-09-09). Devant ce symptôme, vérifier la facturation du compte avant de
chercher une régression dans le code.

Le workflow limite déjà la dépense : cache `cargo`/`vendor`, `concurrency` (une rafale de commits
n'exécute que le dernier run), et `paths-ignore` sur `docs/**`, `**/*.md` et `.claude/**`. Ne pas
étendre `paths-ignore` à `assets/**` : ces fichiers sont embarqués par `include_bytes!`
(`design/assets.rs`, `design/fonts.rs`, `ui_icons.rs`…) et peuvent casser la compilation comme les
snapshots d'`overlay-testkit`.

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

# Rendus visuels — toujours en artefact

L'utilisateur travaille en mode terminal : une image affichée inline (`Read` sur un PNG) **ne
s'affiche pas** dans son client, contrairement à l'aperçu visuel qu'en a Claude. Toute vérification
visuelle doit donc être publiée comme **artefact** (page HTML, image(s) intégrée(s) en base64),
jamais seulement montrée inline — sans quoi le résultat reste invisible pour lui.

Concerné en particulier : les snapshots produits par le harnais de rendu offscreen
`crates/overlay-testkit` (`egui_kittest`, `tests/snapshots/*.png`, §17.1 du plan) — après tout
changement visuel dans `overlay-ui`, publier un artefact reprenant la capture obtenue (page entière
et/ou détail recadré si utile), pas uniquement une ligne de terminal disant que le test passe.
