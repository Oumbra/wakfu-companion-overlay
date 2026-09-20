# Langue

Toutes les réponses, explications et descriptions d'étapes communiquées à l'utilisateur sont
rédigées **en français**, même si le code, les dépendances ou la documentation technique sont en
anglais.

# Commandes shell : passer par `rtk` (obligatoire)

[`rtk`](https://github.com/rtk-ai/rtk) (`rtk --help` pour la liste complète) est un proxy CLI qui condense la sortie des commandes avant qu'elle n'atteigne le contexte — même signal, beaucoup moins de tokens. **Toute commande que `rtk` sait filtrer doit être préfixée par `rtk`**, jamais lancée en natif. Le hook global `PreToolUse` (`rtk hook claude`) réécrit déjà la plupart des appels simples ; le préfixe explicite reste la règle pour ne pas dépendre de ce filet (commandes composées, pipes, `cd ... &&`, scripts) et pour que la commande réellement exécutée soit celle affichée.

| Besoin | Écrire | Pas |
| --- | --- | --- |
| Git (usage premier) | `rtk git status` / `diff` / `log` / `show` / `add` / `commit` / `checkout` / `push` / `pull` / `branch` / `fetch` / `stash` / `worktree` (accepte `-C`, `-c`, `--no-pager`) | `git ...` |
| Recherche de contenu | `rtk grep ...`, `rtk rg ...`, `rtk ast-grep ...` | `grep`/`rg` natifs |
| Fichiers et arborescence | `rtk find ...`, `rtk ls ...`, `rtk tree ...`, `rtk wc ...` | `find`/`ls`/`tree`/`wc` |
| Lecture d'un fichier en shell | `rtk read <fichier>` | `cat`/`head`/`sed -n` |
| npm / npx / outils du projet | `rtk npm run build`, `rtk npm start`, `rtk npx ...`, `rtk tsc`, `rtk lint`, `rtk prettier`, `rtk playwright ...`, `rtk pip ...` | appels natifs |
| GitHub, HTTP, JSON | `rtk gh ...`, `rtk curl ...`, `rtk json ...`, `rtk diff ...` | `gh`/`curl`/`jq`/`diff` |
| Commande sans filtre dédié | `rtk err <cmd>` (erreurs/avertissements seulement), `rtk test <cmd>` (échecs seulement), `rtk summary <cmd>` (résumé heuristique) | sortie brute |

- Enchaîner plusieurs commandes liées dans un seul appel (`rtk git add -A && rtk git commit -m "..."`) plutôt que multiplier les tours.
- Traiter la sortie condensée comme le résultat complet. Une sortie tronquée indique son propre chemin de récupération (`rtk recall <hash>`). Repasser en `rtk proxy <cmd>` (sortie brute, usage tracé) **uniquement** si le résultat est inutilisable : vide alors qu'une sortie était attendue, contredisant le code de sortie, ou illisible.
- `rtk run <cmd>` (aucun filtre, aucun suivi) est réservé aux commandes qui cassent sous filtre — à justifier dans le message.
- Les outils dédiés (`Read`, `Grep`, `Glob`, `Edit`) restent préférables au shell quand ils suffisent ; la règle ci-dessus vaut dès qu'on passe par Bash/PowerShell.
- **Mode dégradé** : si `rtk` est absent (`command -v rtk` échoue — typiquement une session cloud dont l'environnement n'a pas de setup script, ou la machine d'un autre contributeur), le signaler **une seule fois** puis utiliser les commandes natives sans réessayer le préfixe. Ne pas tenter d'installer `rtk` soi-même : en session cloud il s'installe via le setup script de l'environnement (UI claude.ai/code, résultat mis en cache ~7 jours ; le script officiel `install.sh` peut échouer en 403 sur les release assets GitHub — repli `cargo install --git https://github.com/rtk-ai/rtk --locked`), en local c'est un choix de l'utilisateur. Le hook `PreToolUse` de `.claude/settings.json` est déjà protégé et devient un no-op sans `rtk`.

# Style de réponse : Caveman (plugin `caveman@caveman`)

Le plugin [caveman](https://github.com/JuliusBrussee/caveman) compresse la **prose** des réponses
(articles, remplissage, narration des appels d'outils) sans toucher au code, aux commandes, aux
chemins ni aux messages d'erreur exacts. Il complète rtk : rtk réduit ce que Claude *lit*, Caveman
ce que Claude *écrit*.

- Installation : déclaré dans `.claude/settings.json` (`extraKnownMarketplaces` + `enabledPlugins`),
  Claude Code le propose à l'ouverture du dépôt ; à la main :
  `claude plugin marketplace add JuliusBrussee/caveman && claude plugin install caveman@caveman`.
  En session cloud, c'est le setup script de l'environnement qui l'installe.
- Niveau par défaut : `.caveman.json` à la racine (`defaultMode`, ici `full`). Changer en session :
  `/caveman lite|full|ultra|off` (`/caveman-help` rappelle les niveaux) ; `CAVEMAN_DEFAULT_MODE`
  en variable d'environnement prime sur le fichier.
- La règle « répondre en français » prime : Caveman compresse le style, pas la langue.
- Hors périmètre (règle du plugin) : commits, docs, issues, fichiers mémoire et tout texte persistant
  restent en prose normale ; les avertissements de sécurité et les confirmations d'actions
  irréversibles aussi.
- **Mode dégradé** : si le plugin est absent (le hook `SessionStart` le signale), appliquer les mêmes
  règles à la main — phrases courtes, sans remplissage ni narration d'outils, termes techniques
  exacts — sans tenter d'installer le plugin soi-même.
- Ne pas installer le CLI/proxy `@caveman-ai/cli` (`caveman claude`, `caveman setup`) : il redirige
  tout le trafic API vers un proxy local et double rtk sur les sorties de commandes.

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
