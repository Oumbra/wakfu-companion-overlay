# wakfu-companion-overlay

Overlay de jeu natif (Rust) pour [Wakfu](https://www.wakfu.com/), portage de
[`Oumbra/wakfu-companion`](https://github.com/Oumbra/wakfu-companion) : lecture en direct de
`wakfu.log`, affichage par-dessus le jeu des dégâts de combat, du suivi d'objets/ennemis et des
alertes de drop, avec synchronisation de l'historique (combats, achats et récupérations de kamas à
l'Hôtel de Vente, échanges) vers le même compte que l'application web.

**Plateformes visées : Windows et Linux (X11 / XWayland).** macOS est hors périmètre.

État : spikes de validation technique (`spikes/`) terminés ; L1 (ingestion) fait, L2 (UI) en
cours — deux panneaux réels (dégâts du combat, récap de session) tournent déjà sur un vrai
`wakfu.log` (`crates/`, workspace Cargo à la racine).

📄 **[Plan d'architecture technique](docs/plan-architecture.md)** — stack, modèle de threads,
ingestion du log, rendu et click-through par OS, synchronisation serveur, budget mémoire,
feuille de route et revue d'experts.

## Mise en route (développement)

Après un clone, dans l'ordre :

```bash
bash patches/setup-vendor.sh    # vendor/wgpu-hal patché, requis par [patch.crates-io] (voir Cargo.toml)
bash scripts/install-hooks.sh   # hook pre-push : rejoue les lints du CI avant chaque push
```

La version de Rust est **épinglée** par [`rust-toolchain.toml`](rust-toolchain.toml) : `rustup`
installe et sélectionne la bonne toolchain tout seul dès la première commande `cargo` lancée depuis
le dépôt. C'est ce qui garantit que `cargo fmt` et `clippy` rendent ici le même verdict qu'en CI —
sans cet épinglage, la sortie d'une nouvelle version de Rust suffit à faire rougir le CI sur du
code que personne n'a touché.

Sur **SteamOS (Steam Deck)**, le système n'a ni `cargo` ni compilateur et son rootfs est en lecture
seule : préparer d'abord l'environnement une fois pour toutes, depuis un terminal du mode Bureau.

```bash
bash scripts/setup-steamdeck.sh   # conteneur distrobox (Arch) + rustup, HOME/écran/GPU partagés
```

Ensuite, rien ne change : `ci-local.sh`, le hook `pre-push` et `crates/overlay-ui/preview.sh`
entrent tout seuls dans ce conteneur quand la machine ne sait pas compiler (voir
[`scripts/dev-env.sh`](scripts/dev-env.sh)) — y compris depuis le terminal d'un éditeur Flatpak.

Avant de pousser (le hook `pre-push` le fait pour les lints) :

```bash
bash scripts/ci-local.sh          # tout : format, clippy, tests
bash scripts/ci-local.sh --lint   # format + clippy seulement (rapide)
```

## Numéro de version

La version du produit vit à **un seul endroit**, `[workspace.package] version` du `Cargo.toml`
racine : toutes les crates y renvoient (`version.workspace = true`) et Cargo l'embarque dans le
binaire. `crates/overlay-ui/build.rs` y ajoute le **hash du commit** compilé. Les deux se lisent
dans `overlay_ui::build_info`, et se voient à deux endroits : la bannière de la fenêtre Options
(`0.1.0`) et la ligne `=== session démarrée ===` du journal (`0.1.0 (a1b2c3d)`).

**Elle s'incrémente toute seule, ne pas l'éditer à la main.** Le hook `post-commit`
(`scripts/bump-version.sh`) lit le type [Conventional Commits](https://www.conventionalcommits.org/)
du commit qui vient d'être créé, calcule le niveau, met à jour le manifeste et les deux
`Cargo.lock`, puis **amende ce même commit** — le bump voyage donc avec le changement qui l'a causé,
jamais dans un commit séparé.

| Type de commit | Niveau | Exemple |
| --- | --- | --- |
| `feat!:`, `fix!:`, … ou footer `BREAKING CHANGE:` | **major** | `0.4.2` → `1.0.0` |
| `feat:` | **minor** | `0.4.2` → `0.5.0` |
| `fix:`, `style:`, `perf:`, `refactor:`, `test:` | **patch** | `0.4.2` → `0.4.3` |
| `docs:`, `chore:`, `ci:`, `build:`, message non conforme | aucun | `0.4.2` inchangé |

Ce qui touche le binaire livré fait avancer le numéro ; ce qui n'en sort jamais le laisse en place.

```bash
bash scripts/bump-version.sh --dry-run   # ce que ferait le hook sur le commit courant
SKIP_VERSION_BUMP=1 git commit -m "…"    # committer sans bump (échappatoire ponctuelle)
```

Le hook s'abstient de lui-même pendant un `rebase`/`merge`/`cherry-pick` et sur un commit de
fusion : rejouer un commit déjà versionné le bumperait une seconde fois.

## Publication (Release GitHub) et mise à jour automatique

Plan de référence : [`docs/plan-mise-a-jour.md`](docs/plan-mise-a-jour.md). Une Release se
publie **en fusionnant `dev` dans `main`** : `.github/workflows/release.yml` lit la version du
`Cargo.toml`, s'arrête si le tag `v{version}` existe déjà, compile les deux binaires en release
(Windows `wakfu-companion-overlay.exe`, Linux `overlay-ui-x11`), puis `cargo xtask dist` les
compresse en gzip,
écrit le manifeste `latest.json` (SHA-256 des assets et des binaires installés), le signe avec la
clé privée `minisign` des secrets du dépôt et revérifie la signature avec
[`wakfu-overlay.pub`](wakfu-overlay.pub) avant de créer la Release. Le différentiel n'est pour
l'instant que **mesuré** (rapport dans le résumé du job) ; il sera publié quand la mesure le
justifiera.

Un binaire de Release vise toujours l'API de prod ; un binaire compilé avec tout autre profil
(`preview`, debug) vise le déploiement dev — le choix est figé à la compilation
(`crates/overlay-sync/build.rs`), `WAKFU_COMPANION_API_URL` le surcharge à l'exécution.

## Crates (`crates/`)

| Crate | Lot | Contenu |
| --- | --- | --- |
| [`overlay-ingest`](crates/overlay-ingest/) | L1 ✅ | Suivi de `wakfu.log` : découverte de chemin, lecture incrémentale, rotation/troncature. `cargo test -p overlay-ingest`. |
| [`overlay-app`](crates/overlay-app/) | L1 (câblage) | Binaire minimal : branche `overlay-ingest` sur la console pour l'observer sur un vrai `wakfu.log`. `cargo run -p overlay-app`. |
| [`overlay-engine`](crates/overlay-engine/) | L2 🟡 | QuickJS + `LogParser` vendu depuis `wakfu-companion` → `LogEntry` → `SessionSnapshot` (agrégation Rust). `cargo test -p overlay-engine`. |
| [`overlay-ui`](crates/overlay-ui/) | L2 🟡 | Premier overlay réel : fenêtre S1 + panneaux Dégâts du combat/Récap de session, ancré sur la fenêtre du jeu, sur un vrai `wakfu.log`. `.\preview.ps1` (Windows) ou `preview.sh` (Linux) depuis le dossier du crate. |

## Spikes (`spikes/`)

| Spike | État | Prévisualisation |
| --- | --- | --- |
| [`s1-window-windows`](spikes/s1-window-windows/) | ✅ validé | `.\preview.ps1` depuis le dossier du spike |
| [`s2-engine-quickjs`](spikes/s2-engine-quickjs/) | ✅ validé | voir son README |
