# Langue

Toutes les réponses, explications et descriptions d'étapes communiquées à l'utilisateur sont
rédigées **en français**, même si le code, les dépendances ou la documentation technique sont en
anglais.

# Branche de travail : `dev` — règle impérative

**Tout travail de session Claude sur ce dépôt est commité et poussé sur la branche `dev`**, quelle
que soit la session : locale, cloud (Claude Code on the web, tâche GitHub, agent distant),
déclenchée par un trigger, ou reprise.

Début de session : `git fetch origin`, puis `git checkout -B dev origin/dev` (si `dev` n'existe
nulle part, la créer depuis l'état courant : `git checkout -b dev` et `git push -u origin dev`), et
`bash scripts/install-hooks.sh` (`core.hooksPath` est une config **locale**, à reposer dans tout
conteneur neuf).

En session cloud, trois consignes propres au conteneur éphémère :

- La branche auto-générée par l'outil (ex. `claude/mon-sujet-ab12cd`) ne prime pas sur cette
  convention : basculer sur `dev` avant de pousser (au besoin `git cherry-pick` les commits déjà
  faits sur la branche auto-générée).
- Si `git commit` échoue avec `fatal: cannot exec '/tmp/code-sign'` / `failed to write commit
  object` (signature gérée par l'environnement, parfois indisponible), committer avec
  `git commit --no-gpg-sign` — décision explicite de l'utilisateur (2026-09-08). Ne **jamais**
  modifier `~/.gitconfig`/`commit.gpgsign` : c'est un contournement par commit, pas une politique.
  Une session locale n'est pas concernée ; si la signature y échoue, diagnostiquer avant de
  contourner.
- **Ce qui n'est pas commité et poussé sur `dev` est perdu** à la fin de la session : pousser avant
  de conclure, même pour un travail intermédiaire.
- Un conteneur neuf n'est pas prêt pour `cargo` : `bash patches/setup-vendor.sh` (vendor
  `wgpu-hal`), `apt-get install libasound2-dev pkg-config mesa-vulkan-drivers`, et les hooks git
  ci-dessus. Rien ne le fait à votre place.

## Interdits

- **Ne jamais pousser sur `master`/`main`.** Les fusions de `dev` vers la branche principale sont
  faites **par le mainteneur**, jamais par une session Claude.
- Ne pas créer de branche supplémentaire (`feature/*`, `claude/*`…) pour y laisser du travail :
  tout converge sur `dev`.
- Pas de `push --force` sur `dev` (branche partagée entre sessions et avec le mainteneur).
- Pas d'ouverture de pull request sans demande explicite de l'utilisateur.

# Commits

- Format [Conventional Commits](https://www.conventionalcommits.org/) : `feat:`, `fix:`, `docs:`,
  `refactor:`, `chore:`, `test:` — comme sur `Oumbra/wakfu-companion`.
- Ligne de sujet courte (< 50 caractères), en français.
- Un commit = un changement cohérent ; ne pas mélanger documentation et code applicatif.
- Ne pas ajouter d'attribution IA (pas de "Co-Authored-By: Claude")

## Choisir le préfixe : « qu'est-ce qui change pour l'utilisateur du binaire livré ? »

Le préfixe n'est pas un jugement sur l'importance du travail : il pilote la **montée de version**
(hook `post-commit`, voir plus bas) et, à terme, ce qu'annoncent les notes de Release. La question
à se poser est donc : *si cet exe était livré demain, l'utilisateur verrait-il quelque chose de
nouveau (`feat:`), de corrigé (`fix:`), ou rien du tout ?* Quand la réponse est « rien », c'est un
type **sans bump** :

| Changement | Préfixe | Pourquoi |
| --- | --- | --- |
| Nouvelle fonctionnalité, nouveau panneau, nouvel onglet, nouvelle option visible | `feat:` | l'utilisateur voit quelque chose de nouveau |
| Comportement corrigé, régression, valeur par défaut qui change (ex. base URL de l'API) | `fix:` | l'utilisateur voit quelque chose de corrigé |
| Réorganisation interne sans effet visible, renommage, découpage de module | `refactor:` | le binaire change, patch, sans nouveauté visible |
| Captures de référence, tests, golden files | `test:` | patch, par prudence (le harnais protège le binaire) |
| Fichier d'outillage ou de configuration : **clé publique de signature**, `.gitignore`, hooks, scripts de dev, `rust-toolchain.toml` | `chore:` | rien ne change dans l'exe livré |
| Workflow GitHub Actions (`.github/**`), actions composites, `scripts/ci-local.sh` | `ci:` | idem |
| `Cargo.toml` hors dépendances applicatives : profil de compilation, métadonnées | `build:` | idem — une dépendance qui change le comportement est un `feat:`/`fix:` |
| Documentation, plans, `CLAUDE.md`, `.claude/rules/**`, README | `docs:` | idem |

# Numéro de version — automatique, jamais à la main

La version vit **uniquement** dans `[workspace.package] version` (`Cargo.toml` racine) et le hook
`post-commit` l'incrémente d'après le type du commit (`feat:` → minor, `fix:`/`refactor:`/`test:` →
patch, `docs:`/`chore:`/`ci:`/`build:` → rien). **Ne jamais éditer ce champ, ni `Cargo.lock`.**
Détail (niveaux, `--dry-run`, rebase, gel pour les captures) : `.claude/rules/versioning.md`.

# CI — ne jamais pousser du rouge

Trois garde-fous, à utiliser systématiquement :

1. **Toolchain épinglée** (`rust-toolchain.toml`) — ne jamais la contourner (`cargo +stable …`) ;
   monter Rust est un commit explicite.
2. **`bash scripts/ci-local.sh`** (`--lint` pour format + clippy seuls) **avant tout push touchant
   du code**, y compris en session cloud.
3. **Hook `pre-push`** (`bash scripts/install-hooks.sh`).

Historique, symétrie `ci.yml`/`ci-local.sh`, coût et symptômes : `.claude/rules/ci.md`.

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

# Où est documenté quoi (règles chargées à la demande)

Les retours d'expérience détaillés (incidents réels, décisions datées, pièges) vivent dans
`.claude/rules/*.md`. Chaque règle porte un frontmatter `paths:` : elle est chargée automatiquement
dès qu'un fichier correspondant est lu, et reste chargée pour la session. Avant de travailler sur
un de ces sujets sans avoir encore ouvert un fichier concerné (ex. création d'un fichier neuf),
**lire la règle explicitement**. Un nouvel apprentissage se documente dans la règle de son sujet,
jamais ici — ce fichier ne garde que ce qui vaut pour toute tâche.

| Quand on touche à… | Lire / enrichir |
| --- | --- |
| `Cargo.toml`, `Cargo.lock`, hooks git, `bump-version.sh`, `build_info` | `.claude/rules/versioning.md` |
| `.github/**`, `scripts/ci-local.sh`, `rust-toolchain.toml`, un CI rouge | `.claude/rules/ci.md` |
| `overlay-ui`, `overlay-testkit`, `assets/**`, captures PNG, régénération des références | `.claude/rules/captures.md` |
| `overlay-sync`, `release.yml`, mise à jour automatique, `preview.*`, clés de signature | `.claude/rules/release.md` |
| panneau Combat (`panels/combat*.rs`), écart voulu avec le site | `.claude/rules/combat-panel.md` |
| détourer un asset, relever une interface, construire un composant egui | skills `design-asset`, `ui-blueprint`, `ui-component` |
