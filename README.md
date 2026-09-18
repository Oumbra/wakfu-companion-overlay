# wakfu-companion-overlay

Overlay de jeu natif (Rust) pour [Wakfu](https://www.wakfu.com/), portage de
[`Oumbra/wakfu-companion`](https://github.com/Oumbra/wakfu-companion) : lecture en direct de
`wakfu.log`, affichage par-dessus le jeu des dégâts de combat, du suivi d'objets/ennemis et des
alertes de drop, avec synchronisation de l'historique (combats, achats et récupérations de kamas à
l'Hôtel de Vente, échanges) vers le même compte que l'application web.

**Plateformes visées : Windows et Linux (X11 / XWayland).**

📄 **[Plan d'architecture technique](docs/plan-architecture.md)** — stack, modèle de threads,
ingestion du log, rendu et click-through par OS, synchronisation serveur, budget mémoire,
feuille de route et revue d'experts.

## Installation

Cette section s'adresse à qui veut simplement **utiliser** l'overlay. Pour compiler le projet
soi-même, voir [« Mise en route (développement) »](#mise-en-route-développement) plus bas.

Les binaires sont publiés sur la page **[Releases](https://github.com/Oumbra/wakfu-companion-overlay/releases/latest)**
du dépôt, un par plateforme, compressés en gzip simple (pas une archive `.zip`/`.tar` — un seul
fichier, à décompresser tel quel) :

- `wakfu-companion-overlay-{version}-windows-x86_64.exe.gz`
- `wakfu-companion-overlay-{version}-linux-x86_64.gz`

Une fois installée, l'overlay se met à jour **seul** au démarrage suivant (`docs/plan-mise-a-jour.md`) :
cette étape manuelle ne se refait qu'à la toute première installation.

### Windows

1. Télécharger `wakfu-companion-overlay-{version}-windows-x86_64.exe.gz` depuis la Release.
2. Le décompresser en `.exe`. L'Explorateur Windows n'ouvre pas les `.gz` nativement ; deux façons
   de s'en sortir sans rien installer de nouveau :
   - **7-Zip** (souvent déjà présent) : clic droit sur le fichier → *7-Zip → Extraire ici*.
   - **PowerShell**, sans outil externe :
     ```powershell
     $in  = [IO.File]::OpenRead("wakfu-companion-overlay-{version}-windows-x86_64.exe.gz")
     $out = [IO.File]::Create("wakfu-companion-overlay.exe")
     $gz  = New-Object IO.Compression.GZipStream($in, [IO.Compression.CompressionMode]::Decompress)
     $gz.CopyTo($out)
     $gz.Dispose(); $out.Dispose(); $in.Dispose()
     ```
3. Lancer `wakfu-companion-overlay.exe`. Aucune dépendance à installer : DirectComposition et
   DirectX 12 (backend `dx12` de wgpu) font partie de Windows 10/11.

### Linux (X11 / XWayland)

L'overlay vise **X11**, natif ou via **XWayland** — c'est le cas de toute session de bureau
courante (GNOME, KDE, XFCE…), même sous Wayland natif, tant que XWayland est installé (il l'est par
défaut sur la quasi-totalité des distributions grand public).

1. Télécharger `wakfu-companion-overlay-{version}-linux-x86_64.gz` depuis la Release.
2. Décompresser et rendre exécutable :
   ```bash
   gunzip wakfu-companion-overlay-{version}-linux-x86_64.gz
   mv wakfu-companion-overlay-{version}-linux-x86_64 wakfu-companion-overlay
   chmod +x wakfu-companion-overlay
   ```
3. Lancer : `./wakfu-companion-overlay`.

Le binaire lie dynamiquement quelques bibliothèques système, quasiment toujours déjà présentes sur
un poste de jeu (session X11 + son + carte graphique), mais listées ici pour les images minimales
(conteneur, serveur, installation "core") :

| Bibliothèque | Rôle | Debian / Ubuntu | Fedora | Arch / SteamOS | openSUSE |
| --- | --- | --- | --- | --- | --- |
| Vulkan (chargeur + pilote) | rendu wgpu — **aucun repli OpenGL**, un pilote Vulkan fonctionnel est obligatoire | `libvulkan1` + `mesa-vulkan-drivers` (ou pilote proprio NVIDIA) | `vulkan-loader` + `mesa-vulkan-drivers` | `vulkan-icd-loader` + `vulkan-radeon`/`vulkan-intel`/`nvidia-utils` selon le GPU | `libvulkan1` + `Mesa-vulkan-device-select` |
| ALSA | lecture des sons d'alerte (`rodio`/`cpal`) | `libasound2` (`libasound2t64` sur les versions récentes) | `alsa-lib` | `alsa-lib` | `libasound2` |
| X11 / xkbcommon | fenêtre, clic-traversant, raccourcis (`winit`, `x11rb`) | `libx11-6`, `libxkbcommon-x11-0` | `libX11`, `libxkbcommon-x11` | `libx11`, `libxkbcommon-x11` | `libX11-6`, `libxkbcommon-x11-0` |

Sur un poste qui fait déjà tourner des jeux (pilote GPU installé, session graphique standard), ces
paquets sont déjà en place — aucune manipulation n'est en général nécessaire.

**Steam Deck (SteamOS)** : lancer le binaire depuis le **mode Bureau** (double-clic ou terminal) ;
SteamOS fournit déjà XWayland, ALSA et le pilote Vulkan RADV, sans rien à installer. Le mode Jeu ne
lance pas d'exécutable arbitraire hors de Steam — passer par le mode Bureau, éventuellement en
ajoutant le binaire comme jeu non-Steam pour un accès rapide.

**Compatibilité glibc** : le binaire est compilé sur Ubuntu (dernière image `ubuntu-latest` du CI).
Sur une distribution **beaucoup plus ancienne** que sa glibc, le lancement peut échouer avec une
erreur du type `version 'GLIBC_2.xx' not found` — dans ce cas, mettre à jour la distribution ou
compiler depuis les sources (section suivante) plutôt que chercher un correctif ponctuel.

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
bash scripts/ci-local.sh          # tout : format, clippy, tests, captures
bash scripts/ci-local.sh --lint   # format + clippy seulement (rapide)
```

### Captures de référence

Le CI compare 153 captures d'interface sous un rendu Linux **figé** (conteneur épinglé par digest +
Mesa épinglé). Une référence régénérée ailleurs — sous Windows, sous un autre Mesa — fait rougir le
gate à coup sûr. Deux chemins, selon la machine :

```bash
# Linux + Docker : on se place dans l'environnement du CI
bash scripts/ci-local.sh --captures-conteneur
UPDATE_SNAPSHOTS=1 bash scripts/ci-local.sh --captures-conteneur   # régénérer
```

Sans Docker (poste Windows, session cloud) : **GitHub → Actions → « Régénérer les captures (rendu
du CI) » → Run workflow**. Il régénère dans le conteneur du CI, publie l'avant/diff/après en
artefact — **à relire, c'est là qu'on attrape une régression promue en référence** — et pousse le
commit.

Un `bash scripts/ci-local.sh` ordinaire joue les captures sur toute plateforme et signale les écarts
trop grands pour du bruit de rastérisation (seuil réglable par `WAKFU_SEUIL_BRUIT_CAPTURES`).

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

Plan de référence : [`docs/plan-mise-a-jour.md`](docs/plan-mise-a-jour.md). Une Release se publie
**en fusionnant `dev` dans `main`** : `.github/workflows/release.yml` lit la version du
`Cargo.toml`, s'arrête si le tag `v{version}` existe déjà, compile les deux binaires en release
(Windows `wakfu-companion-overlay.exe`, Linux `wakfu-companion-overlay-x11`), puis
`cargo xtask dist` les compresse en gzip, écrit le manifeste `latest.json` (SHA-256 des assets et
des binaires installés), le signe avec la clé privée `minisign` des secrets du dépôt et revérifie
la signature avec [`wakfu-overlay.pub`](wakfu-overlay.pub) avant de créer la Release. Le
différentiel n'est pour l'instant que **mesuré** (rapport dans le résumé du job) ; il sera publié
quand la mesure le justifiera.

Un binaire de Release vise toujours l'API de prod ; un binaire compilé avec tout autre profil
(`preview`, debug) vise le déploiement dev — le choix est figé à la compilation
(`crates/overlay-sync/build.rs`), `WAKFU_COMPANION_API_URL` le surcharge à l'exécution.

## Crates (`crates/`)

| Crate | Lot | Contenu |
| --- | --- | --- |
| [`overlay-ingest`](crates/overlay-ingest/) | L1 ✅ | Suivi de `wakfu.log` : découverte de chemin, lecture incrémentale, rotation/troncature. `cargo test -p overlay-ingest`. |
| [`overlay-app`](crates/overlay-app/) | L1 ✅ | Binaire minimal : branche `overlay-ingest` sur la console pour l'observer sur un vrai `wakfu.log`. `cargo run -p overlay-app`. |
| [`overlay-engine`](crates/overlay-engine/) | L2 ✅ | QuickJS + `LogParser` vendu depuis `wakfu-companion` → `LogEntry` → `SessionSnapshot` (agrégation Rust). `cargo test -p overlay-engine`. |
| [`overlay-ui`](crates/overlay-ui/) | L2 ✅ | Premier overlay réel : fenêtre S1 + panneaux Dégâts du combat/Récap de session, ancré sur la fenêtre du jeu, sur un vrai `wakfu.log`. `.\preview.ps1` (Windows) ou `preview.sh` (Linux) depuis le dossier du crate. |
| [`overlay-platform`](crates/overlay-platform/) | L2 ✅ | Primitives spécifiques à l'OS pour `overlay-ui` : découverte de la fenêtre du jeu, décision topmost, clic-traversant. |
| [`overlay-sync`](crates/overlay-sync/) | L3–L5 ✅ | Réseau : appairage/auth native, catalogue (fetch + cache disque + repli embarqué), file de synchro SQLite idempotente vers l'API. |
| [`overlay-testkit`](crates/overlay-testkit/) | L7 | Harnais de rendu offscreen des panneaux `overlay-ui` (153 captures de référence, voir « Captures de référence » plus haut). |

## Spikes (`spikes/`)

| Spike | État | Prévisualisation |
| --- | --- | --- |
| [`s1-window-windows`](spikes/s1-window-windows/) | ✅ validé | `.\preview.ps1` depuis le dossier du spike |
| [`s2-engine-quickjs`](spikes/s2-engine-quickjs/) | ✅ validé | voir son README |
| [`s3-window-linux`](spikes/s3-window-linux/) | ✅ validé | `harness.sh` depuis le dossier du spike (Xvfb) |
| [`s4-capture-hors-focus`](spikes/s4-capture-hors-focus/) | ✅ validé | voir son README (Windows) |

## Licence

[MIT](LICENSE).
