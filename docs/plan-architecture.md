# `wakfu-companion-overlay` — Plan d'architecture technique (v2)

> Overlay de jeu natif Rust pour Wakfu, portage de la logique de
> [`Oumbra/wakfu-companion`](https://github.com/Oumbra/wakfu-companion) (Angular 21).
> Ce document remplace le plan v1 : contraintes révisées (RAM ≤ 300 Mo, **Windows + Linux
> uniquement**, macOS hors périmètre) et périmètre élargi à la **synchronisation serveur**.

---

## 0. Ce qui change par rapport au plan v1

| Point | v1 | v2 (ce document) |
| --- | --- | --- |
| Budget RAM | 15–30 Mo (irréaliste avec wgpu+egui) | **300 Mo max**, cible nominale 150–200 Mo |
| OS | Windows / Linux / macOS | **Windows + Linux**. macOS retiré (supprime le main-thread NSApp, les permissions Accessibility, la notarisation) |
| Fichier lu | `wakfu-chat.log` | **`wakfu.log`** (nom réel, enveloppe log Java du client) |
| Données | « extraire des JSON du dépôt web » | Le catalogue est en **Postgres/Neon derrière une API** — consommation via `GET /api/v1/catalog/` |
| Serveur | absent du plan | **Synchronisation historique combats / achats HDV / kamas HDV / échanges**, idempotente, sur l'API existante |

---

## 1. Faits établis (vérifiés dans le dépôt web, pas supposés)

| Fait | Source |
| --- | --- |
| Le fichier lu est `wakfu.log`, encapsulé dans le log technique Java : `LEVEL HH:MM:SS,mmm [thread] (classe:ligne) - contenu`. Seul `INFO` est traité. | `src/app/core/services/log-parser.ts` (`HEADER_RE`) |
| Le log ne contient **aucune date**, seulement `HH:MM:SS,mmm`. | idem |
| Encodage **UTF-8** (vérifié sur `tests/wakfu.log`, 10 975 lignes). | `file -i tests/wakfu.log` |
| Chemin Windows : `%APPDATA%\zaap\gamesLogs\wakfu\logs\wakfu.log`. | Constat direct sur une installation réelle du client (2026-08-31) — corrige la première version de ce document, qui omettait le sous-dossier `logs\` (déduit à tort d'une ligne de log mentionnant un *autre* fichier, `.../wakfu/config`, dans `tests/wakfu.log` du dépôt web) |
| Les motifs de parsing sont **en français** (`Vous avez gagné … kamas`, `Vous avez ramassé …`). Le client de jeu doit être en FR. | `log-parser.ts` |
| Volume de logique métier à porter : `log-parser.ts` **1 053 l.**, `stats-store.service.ts` **2 755 l.**, `history-sync.service.ts` 435 l., `sync-queue.service.ts` 314 l., `history-archive.service.ts` 789 l. | `wc -l` |
| API : `/api/v1/history/{fights,purchases,trades}` en `POST` (ingestion idempotente par lots) et `GET` (pagination curseur). | `functions/api/v1/history/*.ts` |
| Idempotence : `client_key = sha256(uid \| kind \| signature)` contre `UNIQUE (user_id, client_key)` + `INSERT … ON CONFLICT DO NOTHING`. | `src/app/core/sync/client-key.util.ts`, `functions/api/v1/history/fights.ts` |
| Lots : `MAX_HISTORY_BATCH = 100` côté serveur, `SYNC_BATCH_SIZE = 50` côté client, payload ≤ 1 Mio. | `server/history/parse.ts`, `sync-queue.service.ts` |
| Auth : cookie de session **opaque, HttpOnly, SameSite=Lax** (`wc_session`) + CSRF double-submit `X-CSRF-Token` où `csrf = sha256(session_token + ":csrf")`. | `server/auth/cookies.ts`, `server/auth/flow.ts` |
| Catalogue : index compact gzip **≈ 349 Ko** (≈ 1,15 Mo brut) — 11 032 objets + 851 monstres, 4 langues, format tuple. | `server/README.md`, `server/catalog/compact-index.ts` |
| L'app web est **offline-first** : le mode invité n'envoie rien, la connexion de compte est optionnelle. | `README.md`, `sync-queue.service.ts` |

> ⚠️ **Non vérifiable depuis ici** : le chemin du log sous Linux (Zaap natif vs Steam/Proton). À
> confirmer sur machine réelle — voir §5.1, la détection est conçue pour tolérer l'incertitude.

---

## 2. Décision structurante n°1 — où vit la logique métier

C'est **la** décision du projet. Elle conditionne tout le reste.

### Le vrai risque : la parité de `client_key`

L'idempotence serveur repose sur une signature de contenu calculée côté client, par ex. :

```ts
`${time}|${fightId}|${result}|${participants.map(p => `${normalize(p.name)}#${p.instanceIndex}`).sort().join(',')}`
```

Un `.sort()` JavaScript (ordre par unités de code UTF-16), une normalisation de nom, un arrondi,
un `null` vs `0` : **le moindre écart entre l'implémentation Rust et l'implémentation TS produit une
`client_key` différente pour le même combat** — donc une ligne dupliquée en base pour l'utilisateur
qui utilise à la fois le web et l'overlay. C'est un bug silencieux, permanent, et non réparable par
un rejeu (les deux clés coexistent).

À cela s'ajoutent ~3 800 lignes d'heuristiques durement acquises et documentées comme telles
(corrélation invocation → jointure avec fenêtre temporelle, rapprochement perte de kamas → achat
marchand/HDV, `pendingHdvKamaGain` pour la récupération de kamas à l'Hôtel de Vente, regroupement de
runs de donjon, gating `isInitialLoad`).

### Options

| | A — Port Rust intégral | B — Réutilisation du TS embarqué (QuickJS) | C — Hybride |
| --- | --- | --- | --- |
| Effort v1 | ~4 000–6 000 l. Rust + harnais de parité | ~800 l. Rust (hôte) + bundle headless côté web | B puis migration progressive |
| Risque de divergence | **Élevé et permanent** (2 implémentations à maintenir) | **Nul par construction** | Faible, borné |
| Perf | Optimale | À valider (cf. spike S1) | Optimale sur le chemin chaud |
| RAM | ~5–15 Mo | ~20–50 Mo | ~20–40 Mo |

### ✅ Recommandation : **B pour la v1, derrière une frontière qui permet C plus tard**

Le budget de 300 Mo rend QuickJS (`rquickjs`) parfaitement abordable : moteur ~1 Mo de binaire,
empreinte mémoire dominée par l'état métier lui-même. On embarque **le code TS existant** — parser,
store, signatures — compilé en un bundle ESM unique, exécuté dans un runtime QuickJS sur le thread
Engine. Rust garde ce qu'il fait mieux : IO, réseau, file persistante, fenêtrage, rendu.

Conséquence : **une seule implémentation de la logique métier au monde**, celle du dépôt web, qui
reste la référence. Une correction de parsing profite instantanément aux deux clients.

Le code métier est isolé derrière un trait :

```rust
pub trait EngineBackend {
    fn ingest(&mut self, batch: LineBatch) -> EngineOutput; // snapshot UI + événements à synchroniser
    fn reset_session(&mut self);
    fn snapshot(&self) -> Arc<UiSnapshot>;
}
```

`QuickJsEngine` en v1 ; un futur `NativeEngine` (port Rust) peut le remplacer module par module,
validé par le même harnais de parité (§2.2). La décision n'est jamais irréversible.

### 2.1 Travail requis côté `wakfu-companion` (dépôt web)

Le code métier y est aujourd'hui couplé à Angular (`@Injectable`, `inject()`, `signal()`). Il faut
une **cible de build headless**, sans DOM ni DI :

1. `src/engine/headless.ts` — point d'entrée exportant `createEngine()`, `ingestLines()`,
   `takeSnapshot()`, `drainSyncEvents()`.
2. Alias de build `@angular/core` → `src/engine/angular-signal-shim.ts` (~100 l. : `signal`,
   `computed`, `effect`, `inject`, `Injectable`) — les services du cœur n'utilisent rien d'autre
   d'Angular.
3. Les E/S sont injectées par l'hôte : pas de `fetch`, pas d'IndexedDB, pas de `crypto.subtle`
   dans le bundle (le hachage `client_key` et le réseau restent Rust).
4. ~~`npm run build:engine` → `dist/engine/wakfu-engine.mjs`, publié comme asset de GitHub Release.~~
   **Caduc (2026-09-15)** : le bundle est vendu dans `crates/overlay-engine/engine-js/` et diverge
   du dépôt web ; il voyage avec le binaire, mis à jour par le mécanisme de
   [`plan-mise-a-jour.md`](plan-mise-a-jour.md).

C'est un lot de travail **dans le dépôt web**, à planifier là-bas — pas un détail d'intégration.

### 2.2 Harnais de parité (obligatoire, quelle que soit l'option)

`tests/wakfu.log` (10 975 lignes réelles) + `tests/logs/fr/` deviennent des **golden files** :
un script Node produit `expected-snapshot.json` + `expected-sync-events.json` ; un test Rust
d'intégration rejoue le même fichier et compare octet à octet, `client_key` comprises.
Ce test tourne en CI des deux côtés. C'est le seul filet qui garantit qu'un utilisateur web+overlay
ne voit jamais de doublon.

---

## 3. Architecture d'exécution

```
                        ┌──────────────────────────────────────────────┐
   wakfu.log  ──tail──▶ │ Thread IO / Watcher                          │
   (Java client)        │  notify + PollWatcher de repli               │
                        │  offset, rotation, lignes partielles, UTF-8  │
                        └───────────────┬──────────────────────────────┘
                                        │ crossbeam::channel<LineBatch>  (batchs de ≤ 2 000 lignes)
                                        ▼
                        ┌──────────────────────────────────────────────┐
                        │ Thread ENGINE (mono-thread, propriétaire     │
                        │ de l'état)                                   │
                        │  QuickJsEngine ⟵ wakfu-engine.mjs            │
                        │  → UiSnapshot (ArcSwap)                      │
                        │  → SyncEvent (fight / purchase / trade)      │
                        └────┬──────────────────────────────┬──────────┘
             ArcSwap<Snapshot>│                             │ channel<SyncEvent>
                              ▼                             ▼
        ┌─────────────────────────────────┐   ┌──────────────────────────────────┐
        │ MAIN THREAD — winit + egui/wgpu │   │ Thread SYNC (std::thread bloquant)│
        │  rendu réactif, hit-test,       │   │  file SQLite persistante          │
        │  hotkey global, sondage curseur │   │  POST /api/v1/history/*           │
        └─────────────────────────────────┘   │  auth (keyring), catalogue        │
                    ▲  EventLoopProxy          └──────────────────────────────────┘
                    └── réveil sur nouvel état / fin de sync
```

**Règles :**

- L'état métier n'est **jamais** partagé : il appartient au thread Engine. L'UI ne lit qu'un
  `Arc<UiSnapshot>` immuable publié par `ArcSwap` — zéro verrou sur le chemin de rendu.
- Le thread IO ne parse rien : il découpe des lignes complètes et les envoie par lots.
- Le thread Sync ne connaît que des payloads déjà sérialisés — il ne rappelle jamais l'Engine.
- Le main thread ne fait **aucun IO bloquant**. Point final.
- Pas de thread dédié au sondage de la souris : c'est un `ControlFlow::WaitUntil` de 16 ms sur le
  main thread quand l'overlay est visible (voir §6.3).

---

## 4. Arborescence

```
wakfu-companion-overlay/
├── Cargo.toml                     # workspace
├── crates/
│   ├── overlay-app/               # binaire : câblage, config, cycle de vie
│   │   └── src/{main.rs,config.rs,paths.rs,hotkey.rs}
│   ├── overlay-ingest/            # tail de wakfu.log
│   │   └── src/{watcher.rs,tailer.rs,rotation.rs,discovery.rs}
│   ├── overlay-engine/            # frontière métier
│   │   └── src/{lib.rs,backend.rs,quickjs.rs,model.rs,snapshot.rs}
│   ├── overlay-sync/              # API + file persistante + auth
│   │   └── src/{client.rs,pairing.rs,token_store.rs,queue.rs}  # auth native (L4) fait ; catalogue (L3) fait ; file d'envoi (L5) fait — voir son statut au §12
│   ├── overlay-ui/                # egui : design system, panneaux, i18n
│   │   ├── src/design/            # composants réutilisables (§9.2) : assets.rs, nine_slice.rs, text.rs,
│   │   │                          #   tokens.rs, components/{button.rs,…}
│   │   └── src/{app.rs,panels/{damage.rs,tracker.rs,alerts.rs,recap.rs,status.rs},theme.rs}
│   └── overlay-platform/          # tout le code spécifique OS
│       └── src/{lib.rs,windows/{layered.rs,dcomp.rs,cursor.rs},linux/{x11.rs,cursor.rs}}
├── assets/
│   ├── engine/wakfu-engine.mjs    # bundle headless (fallback include_str!)
│   ├── catalog/catalog-index.json.gz  # repli hors-ligne
│   └── sounds/, fonts/
├── tests/
│   ├── parity/                    # golden files partagés avec le dépôt web
│   └── logs/                      # échantillons réels
├── xtask/                         # build, packaging, mesure RSS
└── docs/plan-architecture.md      # ce document
```

Dépendances principales : `winit`, `wgpu`, `egui`/`egui-wgpu`/`egui-winit`, `notify`,
`rquickjs`, `serde`/`serde_json`, `ureq` (rustls) — voir §7.3 pour pourquoi PAS `reqwest`/`tokio`,
`rusqlite` (bundled), `keyring`, `global-hotkey`, `sha2`, `arc-swap`, `crossbeam-channel`,
`directories`, `tracing`, `chrono` (feature `clock` seulement — voir §5.4, conversion de fuseau
LOCAL sans base de fuseaux IANA embarquée). Sur Windows : `windows` (Win32 + DirectComposition). Sur
Linux : `x11rb`.

---

## 5. Ingestion de `wakfu.log`

### 5.1 Découverte du fichier

Ordre d'essai, premier existant retenu, **toujours surchargeable** par la config et par un sélecteur
de fichier dans l'UI :

**Windows**
1. `%APPDATA%\zaap\gamesLogs\wakfu\logs\wakfu.log` *(vérifié sur machine réelle, 2026-08-31)*
2. `%LOCALAPPDATA%\Ankama\zaap\gamesLogs\wakfu\logs\wakfu.log` *(à confirmer — sous-dossier `logs\` aligné par cohérence sur le n°1, pas vérifié pour cet emplacement)*

**Linux**
1. `$XDG_CONFIG_HOME/zaap/gamesLogs/wakfu/wakfu.log` puis `~/.config/zaap/gamesLogs/wakfu/wakfu.log` *(à confirmer)*
2. Préfixe Proton/Steam : `~/.steam/steam/steamapps/compatdata/*/pfx/drive_c/users/steamuser/AppData/Roaming/zaap/gamesLogs/wakfu/wakfu.log`
3. Préfixes Wine génériques : `~/.wine/drive_c/users/*/AppData/Roaming/zaap/gamesLogs/wakfu/`

L'échec de détection n'est **pas** une erreur fatale : l'overlay démarre et affiche un sélecteur.

### 5.2 Suivi (tail)

- `notify::RecommendedWatcher` sur le **répertoire** (pas le fichier : la rotation détruit l'inode
  surveillé), + `PollWatcher` à 1 s en repli et sur montage réseau.
- Debounce 100 ms : le client Java écrit par rafales.
- Lecture depuis `offset`, découpe sur `\n`, **conservation du reliquat** (ligne partielle) jusqu'au
  prochain événement — jamais de parsing d'une demi-ligne.
- Décodage UTF-8 strict avec remplacement (`String::from_utf8_lossy`) : une ligne corrompue ne doit
  jamais tuer l'ingestion.
- **Détection de rotation/troncature** : si `len < offset` → repartir de 0 ; comparer aussi
  l'identité du fichier (Linux `st_dev`/`st_ino`, Windows `FILE_ID_INFO` via
  `GetFileInformationByHandleEx`) pour repérer un remplacement à taille croissante. Implémenté et
  testé dans `crates/overlay-ingest/` (L1). Le répertoire de logs réel observé (§5.1) confirme une
  rotation par **renommage** (`wakfu.log` → `wakfu.log.0` → `.1` → `.2`, un nouveau `wakfu.log`
  recréé au même chemin) plutôt qu'une troncature en place — exactement le cas que la comparaison
  d'identité est conçue pour couvrir.
- Ouverture en lecture seule ; `std::fs` sous Windows demande déjà
  `FILE_SHARE_READ|WRITE|DELETE` — le client Java garde donc son handle sans conflit.

> **Bug réel trouvé (2026-09-04, en construisant la CI de base, §17.3) — corrigé le même jour.**
> `FileIdentity` (dev+ino) suppose qu'un `rm` suivi d'un `create` au même chemin obtient un inode
> DIFFÉRENT de celui libéré — pas garanti par POSIX/ext4 en général, et effectivement mis en défaut
> ici (reproduit à coup sûr sur cet `ext4` réel, pas un `tmpfs` exotique, mais démontré NON
> déterministe : disparaît selon quelle autre activité fichier a eu lieu juste avant dans le même
> process — dépend donc de l'état de l'allocateur d'inodes, pas de notre code). Comme le §5.1
> confirme que la rotation réelle se fait par **renommage** (`wakfu.log` → `wakfu.log.0`, nouveau
> `wakfu.log` recréé, où l'ancien inode reste occupé par le fichier renommé), l'impact en
> conditions réelles était probablement faible — mais l'hypothèse posée par `FileIdentity` seule
> restait fausse dans l'absolu. **Correctif** : `Tailer` compare désormais AUSSI, à chaque `poll()`,
> les `IDENTITY_PREFIX_LEN` (64) premiers octets du fichier à ceux mémorisés au tour précédent (voir
> `Tailer::identity_prefix`) — un fichier de log n'écrase jamais un octet déjà écrit, donc un
> préfixe qui change (sur la longueur commune) prouve un remplacement, MÊME quand `FileIdentity`
> prétend le contraire. Testé 20/20 exécutions vertes en forçant délibérément la réutilisation
> d'inode (isolé du reste de la suite, condition qui la déclenche systématiquement). L'ancien test
> `rotation::tests::remplacement_identite_differente` (`assert_ne!` sur l'identité seule) est
> retiré : il testait une garantie OS fausse, pas notre code — sa couverture réelle vit maintenant
> dans `tailer::tests::rotation_nouveau_fichier_meme_chemin_relit_depuis_zero`, qui passe
> indépendamment de ce que dit `FileIdentity`. Les deux tests précédemment exclus de la CI
> (`--skip`) y sont donc revenus normalement.

### 5.3 Sémantique `isInitialLoad` (parité obligatoire)

Toute (re)connexion relit le fichier **depuis le début**. Le premier lot porte `is_initial_load =
true`, ce qui déclenche côté Engine `resetSessionState()` (historique/kamas/XP reconstruits) mais
**jamais** l'incrément des compteurs persistants (watchlist). C'est le principe d'architecture n°2
du `CLAUDE.md` du dépôt web : le reproduire à l'identique, sinon les compteurs de suivi regonflent
à chaque relance de l'overlay.

**Écart assumé, portage seulement (2026-09-02, retour utilisateur, vidéo à l'appui) :**
`resetSessionState()` n'est en réalité rejoué qu'au **tout premier** rattrapage de l'Engine, jamais
à une rotation `wakfu.log` ultérieure survenant EN COURS DE SESSION (`Engine::state_initialized`,
`overlay-engine/src/session.rs`) — un fichier rotaté ne rejoue PAS l'historique déjà lu côté client
Wakfu (vérifié en conditions réelles), donc vider `state` à ce moment-là effaçait un combat encore
actif en jeu (allés/ennemis disparaissaient du panneau Combat pile à la rotation). Le PARSER, lui,
est toujours réinitialisé à chaque rattrapage, rotation comprise (contexte transitoire QuickJS à
resynchroniser avec la position de lecture) — seul `state` (combats/totaux Rust) survit désormais
au-delà du tout premier. Le dépôt web n'a pas ce problème dans les mêmes proportions (page
généralement rechargée entière avant qu'une reconnexion ne survienne) ; ce n'est donc pas un écart
de parité fonctionnelle voulu, seulement une conséquence du fait que l'overlay, contrairement à un
onglet de navigateur, reste ouvert en continu pendant des heures de jeu et traverse donc bien plus
souvent une vraie rotation `wakfu.log` mid-session.

### 5.4 Date et heure

Le log ne porte que `HH:MM:SS,mmm`. L'Engine reconstitue la date du jour de lecture (comportement
web actuel). Deux conséquences déjà traitées côté web, à ne pas casser :
- les **signatures** n'utilisent que l'heure brute (relire demain le même fichier reste idempotent) ;
- passage de minuit : détecter le recul de l'horloge (`HH:MM:SS` inférieur au précédent) et
  incrémenter le jour côté hôte plutôt que d'empiler les événements sur une seule date.

**Fuseau horaire des instants synchronisés (`startedAt`/`occurredAt`, correctif du 2026-09-03,
retour utilisateur en conditions réelles)** : le log Wakfu n'écrit que l'heure LOCALE de la machine
qui fait tourner le client de jeu — exactement comme le `new Date(year, month, day, ...)` du web est
implicitement local au fuseau du NAVIGATEUR. `overlay_engine::log_time::LogDateTracker::
full_timestamp_ms` traitait initialement cette date civile comme un instant **UTC** au lieu de la
convertir depuis le fuseau LOCAL de la machine — décalage constant, égal au fuseau de la machine,
sur les seuls champs `startedAt`/`occurredAt` envoyés au serveur (jamais sur la signature, qui
n'utilise que l'heure brute, donc sans impact sur l'idempotence). Repéré en usage réel : un
événement survenu à 22h54 heure locale (CEST, UTC+2) apparaissait daté du LENDEMAIN sur le site
(l'instant, réinterprété à tort comme "22h54 UTC", tombe après minuit une fois reconverti). Corrigé
via `chrono::Local` (feature `clock` uniquement) : **pas `chrono-tz`**, aucune base de fuseaux IANA
embarquée — la résolution de fuseau est déléguée entièrement à l'OS (API Windows / `/etc/localtime`
sous Linux), donc pas la dépendance lourde qu'un premier jet de ce document écartait à tort pour ce
besoin (elle visait la résolution d'un fuseau NOMMÉ arbitraire, jamais nécessaire ici : seul le
fuseau COURANT de la machine locale compte, exactement ce que fournit `Local` sans base de données).

### 5.5 Performance cible

**Critère non négociable, celui-là mesurable indépendamment du moteur retenu** : l'UI reste à
60 fps pendant l'ingestion initiale d'un gros fichier — jamais de gel, avec une barre de
progression. C'est exactement la régression vécue côté web (~10 s de gel, sans spinner, corrigée
le 2026-08-30) ; le modèle de threading (§3) la rend structurellement impossible ici, l'Engine
tournant sur un thread séparé du rendu.

Le débit d'ingestion lui-même (« 80 000 lignes en combien de temps ») est un critère secondaire,
d'expérience utilisateur (délai avant historique complet disponible), pas de fluidité : voir le
spike S2 (`spikes/s2-engine-quickjs/README.md`), qui mesure ~28 000 lignes/s avec QuickJS sur le
vrai `LogParser` — sous la cible initiale de ~40 000 lignes/s, mais sans jamais bloquer le rendu.
Prérequis qui en découle pour L1 : publier des `UiSnapshot` intermédiaires pendant `isInitialLoad`
(pas seulement à la fin), pour que la barre de progression avance réellement.

---

## 6. Fenêtrage, rendu, click-through

### 6.1 Choix de rendu

`winit` + `wgpu` + `egui` en **mode réactif** :

- `ControlFlow::Wait` par défaut ; réveil par `EventLoopProxy` quand l'Engine publie un snapshot.
- Pendant un combat, l'Engine ne publie qu'à **10 Hz** (les compteurs de dégâts n'ont pas besoin de
  60 Hz) ; l'interaction souris repasse à 60 Hz.
- Overlay masqué (hotkey) → aucune frame rendue, aucun sondage curseur.

C'est ce qui rend l'outil « léger » en CPU/GPU/batterie, bien plus que le choix du langage.

### 6.2 Windows — validé par S1 (`spikes/s1-window-windows/`)

Une fenêtre à transparence par pixel + toujours au-dessus + traversable, avec un swapchain moderne,
ne s'obtient pas naïvement, mais le chemin est **plus simple que prévu ici** : pas besoin de piloter
`IDCompositionDevice`/`Target`/`Visual` à la main, `wgpu-hal` le fait déjà en interne. Chemin
confirmé par le spike :

1. Fenêtre `winit` : `with_transparent(true)`, `with_decorations(false)`,
   `with_window_level(AlwaysOnTop)`, et extensions Windows `with_skip_taskbar(true)`,
   `with_no_redirection_bitmap(true)`.
2. Styles étendus complémentaires, posés à la main sur le HWND (non exposés par `winit`) :
   `WS_EX_NOACTIVATE` (ne vole jamais le focus au jeu), `WS_EX_TOOLWINDOW`.
3. Composition **DirectComposition**, pilotée entièrement par `wgpu-hal` : backend forcé DX12,
   `Dx12BackendOptions { presentation_system: Dx12SwapchainKind::DxgiFromVisual, .. }` à la création
   de l'instance `wgpu`. `wgpu-hal` crée et gère lui-même `IDCompositionDevice`/`Target`/`Visual` en
   interne dès la configuration de la surface (`DCompState::get_or_init`) — nul besoin de code
   DirectComposition manuel côté `overlay-platform`. `CompositeAlphaMode::PreMultiplied`
   obligatoire (`PostMultiplied` est rejeté par `CreateSwapChainForComposition` sur ce pilote), ce
   qui correspond de toute façon au blending qu'`egui_wgpu::Renderer` produit déjà.
4. Deux bugs réels de `wgpu-hal` 30.0.1 pour la cible composition (`DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING`
   posé inconditionnellement, `SwapEffect` codé en dur à `FLIP_DISCARD` au lieu de
   `FLIP_SEQUENTIAL`) nécessitent un patch vendored tant qu'ils ne sont pas corrigés en amont — voir
   `spikes/s1-window-windows/README.md` (§"Découverte n°2") et le patch associé. À réévaluer pour
   `overlay-platform` : soit une PR amont sur `gfx-rs/wgpu` d'ici là, soit reconduire le même patch.
5. Clamper `config.width`/`height` à `device.limits().max_texture_dimension_2d` avant chaque
   `Surface::configure()` : un `WindowEvent::Resized` incohérent et transitoire a été observé au
   tout premier redimensionnement (cause racine non élucidée, sans impact une fois clampé) — pattern
   à reprendre systématiquement, indépendamment de sa cause.
6. Repli documenté si une configuration matérielle future s'avère incompatible : fond opaque + mode
   « fenêtre compagnon accolée » plutôt qu'overlay transparent — non nécessaire sur le matériel testé
   (RTX 3080 Ti, driver 32.0.16.1062, Windows 11).
7. **Le rendu est piloté par notre boucle, pas par `WM_PAINT` (2026-09-12).** Sur ces fenêtres
   (composition, `with_no_redirection_bitmap`), `Window::request_redraw()` — qui repose sur
   `RedrawWindow(RDW_INTERNALPAINT)` — ne produit pas de `RedrawRequested` de façon fiable. Mesuré
   sur la modale Options avec un journal instrumenté et une frappe pilotée par `SendInput` : les
   touches arrivaient dans `window_event`, le redessin était demandé, et la frame suivante ne venait
   que de la réaffirmation topmost périodique (`SetWindowPos`, 2 s) — la saisie s'appliquait par
   paquets de deux secondes. Depuis, `App::redraw` rend une frame directement depuis
   `about_to_wait` pour toute fenêtre dont `next_redraw_at` est échu ; les événements et le thread
   Engine ne font que poser cette échéance, et `RedrawRequested` n'est plus qu'un déclencheur parmi
   d'autres. Latence mesurée après correctif : moins de dix millisecondes entre la touche et sa
   frame. Le mode réactif de §6.1 est intact — rien ne tourne à 60 Hz sans raison.

Détail complet (bugs, découvertes, repro isolé, capture d'écran de validation) dans
`spikes/s1-window-windows/README.md` — à relire avant d'implémenter `overlay-platform::windows`,
ce document ne le répète pas.

### 6.3 Click-through et retour de la souris

`window.set_cursor_hittest(false)` rend la fenêtre traversable — mais elle ne reçoit alors **plus
aucun événement souris**, donc elle ne peut pas savoir que le curseur la survole. Deux mécanismes,
volontairement redondants :

1. **Hotkey global (mécanisme principal)** — `global-hotkey`, par défaut `Ctrl+Shift+W` : bascule
   interactif/traversable. Déterministe, fonctionne partout, aucun sondage.
2. **Survol (confort)** — quand l'overlay est visible, sondage de la position curseur à 60 Hz sur le
   main thread (`GetCursorPos` / `XQueryPointer`), comparaison aux rectangles interactifs du dernier
   snapshot, et bascule du hit-test. Coût mesuré négligeable ; désactivable dans la config.

En mode interactif, l'overlay ne prend jamais le focus clavier tant qu'un champ de saisie n'est pas
explicitement cliqué (`WS_EX_NOACTIVATE` côté Windows, `_NET_WM_STATE_ABOVE` + pas de
`input_focus` côté X11) — sinon le jeu perd ses raccourcis.

### 6.3 bis Curseur du jeu à la place du curseur système (2026-09-13)

Quand le pointeur survole un overlay interactif, c'est le **curseur de Wakfu** qui s'affiche, pas
la flèche de l'OS — même logique que les boutons et infobulles du design system : l'overlay doit
passer pour une partie du jeu. Les quatre bitmaps (`assets/cursor/`, isolés pixel par pixel depuis
des enregistrements d'écran, voir leur `README.md`) sont embarqués par `overlay-ui::cursor` ; le
point chaud de la flèche est déduit de l'image (première ligne opaque), les fichiers peuvent donc
être re-détourés sans toucher au code — celui de la croix fléchée et de l'I-beam est leur
**centre**, une croix symétrique ne pointant nulle part et un I-beam visant sa hampe.

Comportement calqué sur le jeu, mesuré à 30 i/s : flèche d'egui (`CursorIcon::Default`) → bitmap
de repos, fixe ; main (`PointingHand`, tout ce qui se clique) → clignotement éclair 533 ms / repos
533 ms, l'éclair en premier dès l'entrée en survol, bascule franche sans fondu ; croix fléchée
(tout curseur de déplacement ou de saisie : `Move`, posé par les tuiles de suivi — dans la fenêtre
Options comme dans le bandeau in-game, voir `panels::tile_reorder` ; `Grab`/`Grabbing`, qu'egui pose
lui-même au survol d'un glissable et tant qu'une charge est en vol) → bitmap de déplacement,
**fixe** (le jeu ne le fait pas clignoter, et n'a qu'une seule croix pour tous ces gestes) ;
I-beam (`Text`, posé par `design::input` au survol d'un champ de saisie) → bitmap texte, **fixe**
lui aussi (2026-09-14) ; tout autre curseur (`ResizeHorizontal`, `VerticalText`…) → curseur système
inchangé.

Mécanique : egui 0.36 porte nativement un curseur bitmap (`Context::set_cursor_image` →
`PlatformOutput::cursor_image`), qu'`egui-winit` applique en `winit::window::CustomCursor` à
condition de recevoir l'event loop (`handle_platform_output_with_event_loop`, d'où le paramètre
ajouté à `frame::render`). Le choix de l'image est fait à la fin de `render_content::paint_content`,
donc dans le code partagé par les deux binaires **et** par le harnais `overlay-testkit`, qui le
vérifie en lisant `FullOutput::platform_output.cursor_image`. Le mode réactif de §6.1 est respecté :
en mode main, chaque frame demande le redessin pile pour la prochaine bascule
(`request_repaint_after`) ; ailleurs, rien n'est redemandé. Limite connue : `CustomCursor` est en
pixels physiques, le curseur n'est pas agrandi avec l'échelle d'affichage du système.

### 6.4 Linux

- **X11 : plateforme supportée.** `_NET_WM_STATE_ABOVE`, `_NET_WM_WINDOW_TYPE_UTILITY`, transparence
  via visuel 32 bits ARGB (**nécessite un compositeur actif** : picom, KWin, Mutter — sinon repli
  opaque automatique), passthrough via région d'entrée XShape (ce que fait `set_cursor_hittest`).
- **Wayland natif : non supporté, et c'est assumé.** Deux blocages de conception, pas des bugs :
  aucun positionnement absolu par le client, et pas d'équivalent portable au hit-test. L'overlay
  force le backend X11 (`WINIT_UNIX_BACKEND=x11`) et tourne donc sous **XWayland** dans une session
  Wayland — configuration à valider sur le terrain avec le client Wakfu (lui-même Java/JOGL, donc
  très probablement XWayland également).
- Jeu en **fenêtré sans bordure** exigé (documenté à l'utilisateur), comme sous Windows.
- Multi-écran/HiDPI : suivre le `scale_factor` winit ; ancrage automatique de l'overlay sur la
  fenêtre de jeu (voir §6.5), pas de repositionnement manuel (retiré, décision du mainteneur,
  voir §9).

### 6.5 Ancrage sur la fenêtre de jeu — une fenêtre overlay par fenêtre de jeu

**Multi-compte (2026-09-01, retour utilisateur en test réel)** : `wakfu.log` est **partagé et
entrelacé** par toutes les instances du client lancées sous le même compte Windows (contrairement à
Dofus, qui écrit un fichier par instance — vérifié sur le disque). Un seul overlay ancré sur une
fenêtre trouvée « au hasard » affichait donc le combat d'un **autre** personnage que celui de la
fenêtre sur laquelle il était collé. Correction : **une fenêtre overlay par fenêtre de jeu
trouvée**, créée/détruite dynamiquement au gré des clients qui se lancent/se ferment, chacune
affichant le combat de SON personnage (`overlay_engine::SessionSnapshot::fight_for_character`,
résolu depuis le nom extrait du titre de fenêtre — voir §2 du plan overlay-engine et
`crates/overlay-ui/src/main.rs::App::sync_windows`). Chaque fenêtre overlay se cale au bord gauche
de SA fenêtre de jeu, verticalement centrée dessus, et suit tout déplacement/redimensionnement
(`crates/overlay-ui/src/game_window.rs`) :

- **Identification par titre, pas par process.** Le titre de la fenêtre de jeu est
  `"<Nom du personnage> - WAKFU"` — variable, mais le suffixe `" - WAKFU"` est constant, ET c'est
  lui qui donne le nom de personnage servant au rapprochement fenêtre↔combat. Vérifié en
  conditions réelles : le client tourne sous un process `java`/`javaw` générique (Wakfu est Java,
  voir §6.4), donc filtrer par nom d'exécutable est trop large pour être fiable — seul le titre
  discrimine correctement.
- **Windows** : `EnumWindows` + `GetWindowTextW` pour trouver **toutes** les fenêtres de jeu (pas
  la première seulement) à chaque scan — pas de cache de `HWND` unique, un `EnumWindows` complet
  reste négligeable même répété à 20 Hz. Rectangle via `DWMWA_EXTENDED_FRAME_BOUNDS` (bord
  réellement visible, pas la marge de redimensionnement invisible que `GetWindowRect` inclut sur
  Windows 10/11), repli sur `GetWindowRect` si l'appel DWM échoue.
- **Sondage à 20 Hz** (même tick que le sondage hotkey, §6.3) plutôt qu'un événement : il n'existe
  pas d'API portable pour être notifié du déplacement d'une fenêtre qui n'est pas la nôtre sans un
  hook global (`SetWinEventHook`) — jugé disproportionné pour ce besoin. Coût mesuré négligeable ;
  repositionnement (`set_outer_position`) uniquement si la position cible a changé, pas à chaque
  tick. Même tick pour la création/destruction dynamique des fenêtres overlay (diff par `HWND`
  entre deux scans).
- **Focus-aware topmost** : chaque overlay reste au-dessus tant qu'une fenêtre de jeu (n'importe
  laquelle, pas nécessairement la sienne) ou un overlay a le focus (`GetForegroundWindow` comparé
  aux `HWND` connus), sinon repli en z-order normal via `SetWindowPos(HWND_NOTOPMOST, ...)` — ne
  recouvre plus une application quelconque devenue active (explorateur de fichiers, navigateur…),
  retour utilisateur du 2026-09-01. Politique volontairement simplifiée (pas de logique « seulement
  l'overlay du personnage actif »).
  **Délai de grâce avant repli (2026-09-02, retour utilisateur, vidéo à l'appui)** : la démotion en
  `HWND_NOTOPMOST` était jusqu'ici IMMÉDIATE dès qu'un seul tick (~50 ms, cadence de sondage §6.5)
  voyait `GetForegroundWindow()` cesser de désigner la fenêtre de jeu — l'overlay Combat
  disparaissait alors « un coup sur deux » en changeant de fenêtre, alors que le Suivi du MÊME
  personnage restait visible au même instant bien que les deux passent par exactement le même code
  (`App::sync_topmost`, vérifié identique pour les deux) : un aléa d'ordonnancement Windows d'un
  seul tick entre les deux `SetWindowPos` suffisait à les faire diverger visuellement. Un overlay
  qui vient de perdre `relevant` n'est désormais démoté qu'après `TOPMOST_DEMOTE_GRACE` (1,5 s)
  écoulée EN CONTINU sans redevenir pertinent — la réaffirmation en topmost, elle, reste immédiate
  dès que le focus revient, avant l'échéance. Ne revient pas sur le principe du 2026-09-01 (repli
  toujours appliqué au bout du délai), absorbe seulement les aléas de timing d'un tick.
- **Fenêtres à durée de vie dynamique** : le motif `Box::leak`/`&'static Window` du mono-fenêtre
  d'origine ne tient plus dès qu'une fenêtre doit pouvoir être détruite (client fermé) —
  `Arc<Window>` à la place (`wgpu::Instance::create_surface` l'accepte directement, donnant un
  `Surface<'static>` sans fuite : motif standard wgpu+winit pour ce cas).
- **Récap de session encore global** : chaque overlay affiche pour l'instant les mêmes totaux
  cumulés (kamas/XP/combats), pas ventilés par personnage — limitation connue, kamas n'a pas de
  champ personnage dans le log (XP si, mais nécessiterait un vrai second lot de travail).
- **X11 (à faire, S3 différé)** : équivalent par `_NET_WM_NAME` (ou `WM_NAME`) + comparaison de
  suffixe, `_NET_CLIENT_LIST` pour l'énumération, `XGetWindowProperty`/`_NET_FRAME_EXTENTS` pour le
  rectangle visible — devra couvrir le multi-fenêtre dès le départ, pas en repli après coup.

---

## 7. Synchronisation serveur

### 7.1 Ce qui est envoyé (parité web stricte)

| Événement | Endpoint | Contenu clé |
| --- | --- | --- |
| Combat terminé | `POST /api/v1/history/fights` | `clientKey`, `startedAt`, `durationMs`, `won`, `turns`, `totalDamage`, `xpGained`, `kamasGained`, `gameServer`, `dungeonId`, `dungeonRunKey`, `challengesPassed/Failed`, `participants[]` (side, name, monsterId, instanceIndex, damage, defeated, fled, spells[], xpGained), `loot[]` |
| Achat marchand/HDV | `POST /api/v1/history/purchases` | `clientKey`, `itemId`\|`itemName`, `quantity`, `totalCost`, `occurredAt`, `gameServer` |
| **Récupération de kamas HDV** | `POST /api/v1/history/purchases` | même endpoint, `itemName = "__hdv_kamas_sale__"` (constante `HDV_KAMAS_SALE_ITEM`) — c'est ainsi que le web l'enregistre |
| Échange joueur | `POST /api/v1/history/trades` | `clientKey`, `peerName`, `selfName`, `kamasAcquired/Given`, `items[]` |

`clientKey = sha256_hex("{uid}|{kind}|{signature}")`, signature produite **par l'Engine** (donc par
le code TS partagé) — l'hôte Rust ne fait que le hachage, comme `SyncQueueService` le fait côté web.

### 7.2 Authentification native — le seul vrai manque côté serveur

L'API s'authentifie par **cookie de session HttpOnly posé par un flux OAuth navigateur**. Un client
natif ne peut pas récupérer ce cookie. C'est le seul point qui **exige une évolution du dépôt web**.

**Solution recommandée — appairage par code (device pairing), aucune reconfiguration OAuth :**

1. Overlay → `POST /api/v1/auth/native/pair` → `{ pairingCode, verificationUrl, pollToken }`.
2. Overlay ouvre `verificationUrl` dans le navigateur par défaut. L'utilisateur s'y connecte par le
   flux Discord/Google **existant, inchangé** (les URI de redirection déclarées chez les
   fournisseurs ne bougent pas — c'est tout l'intérêt face à une redirection loopback).
3. La page web appelle `POST /api/v1/auth/native/claim` (même origine, cookie + CSRF classiques)
   avec le `pairingCode` affiché par l'overlay ; le serveur émet un **jeton de session natif**
   (même table `sessions`, marqué `client=native`, révocable dans « Sessions actives »).
4. Overlay récupère le jeton par `POST /api/v1/auth/native/poll` (expiration 10 min, un seul usage).

**Coût côté serveur** : 3 handlers + 1 page de saisie de code. Et une modification de 3 lignes dans
`functions/api/_auth.ts` pour accepter `Authorization: Bearer <token>` en plus du cookie —
avec un porteur explicite, **le contrôle CSRF n'a plus lieu d'être** (il protège d'un cookie envoyé
automatiquement par un navigateur, ce qui n'existe pas ici).

> Solution de repli sans aucune modification serveur, si l'évolution ci-dessus n'est pas retenue :
> l'overlay peut envoyer `Cookie: wc_session=<token>` et calculer lui-même
> `X-CSRF-Token = sha256("<token>:csrf")` — les endpoints existants l'acceptent tels quels. Reste
> le problème de l'obtention initiale du jeton, qui exige de toute façon un point d'entrée natif.
> **Ne jamais** aller lire le cookie dans la base du navigateur : fragile, intrusif, indéfendable.

**Stockage du jeton** : trousseau OS via `keyring` (Credential Manager sous Windows, Secret Service
sous Linux). Repli explicite et signalé à l'utilisateur : fichier `0600` sous
`$XDG_DATA_HOME/wakfu-overlay/` quand aucun Secret Service n'est disponible (WM minimalistes).

### 7.3 File d'envoi (miroir Rust de `SyncQueueService`) — ✅ fait (lot L5, 2026-09-02)

Table SQLite `sync_queue(id TEXT PRIMARY KEY, kind, payload_json, signature, queued_at, attempts)`
— `id = "{kind}:{signature}"`, donc dédoublonnage naturel, comme en IndexedDB.
**Simplification retenue par rapport à la table esquissée initialement ici** : pas de colonne
`next_attempt_at` — un seul backoff GLOBAL pour toute la file (`SyncQueue::consecutive_failures`,
en mémoire, jamais persisté), pas un par ligne : c'est exactement ce que fait déjà
`SyncQueueService.consecutiveFailures` côté web (un seul compteur d'instance, pas un champ par
entrée IndexedDB) — reproduire une planification par ligne aurait été plus fidèle au schéma
esquissé mais moins fidèle au COMPORTEMENT réel qu'il miroir. Pas de plafond de taille de payload
appliqué côté client non plus (le serveur, lui, borne déjà tout ce qui compte — `MAX_HISTORY_BATCH`,
tailles de champs — voir `server/history/parse.ts`) : un payload construit depuis
`overlay_engine::history` ne peut de toute façon pas dépasser 1 Mio en pratique (pas de champ non
borné), ce n'était pas un vrai garde-fou à porter ici.

Paramètres **identiques au web** (`overlay_sync::queue`) : lots de **50** (`SYNC_BATCH_SIZE`),
backoff **15 s → 5 min** (doublement, `backoff_delay` côté `overlay-ui`), abandon d'une entrée
après **10 tentatives non réseau** (`MAX_ATTEMPTS`) — seul un rejet HTTP 4xx (hors 401/429) compte
comme tentative, exactement comme `permanent` côté web.

**Écart assumé : pas de vrai debounce de 2 s** (`FLUSH_DEBOUNCE_MS` côté web) — chaque
`SyncCommand::Enqueue` reçu par le thread Sync (`overlay-ui::spawn_sync_thread`) déclenche une
tentative d'envoi immédiate de TOUT ce qui est en file à cet instant (pas seulement ce qui vient
d'arriver). Jamais incorrect (rien n'est perdu, l'idempotence tient toujours), seulement plus
d'appels réseau qu'un vrai regroupement dans le cas d'une rafale de petits lots rapprochés — non
mesuré comme gênant en pratique, à revisiter si ça devient un problème réel.

**`reqwest`/`tokio` (esquissés au §4 dans une itération précédente de ce document) abandonnés au
profit de `ureq` bloquant sur un `std::thread` dédié** : toutes les autres briques réseau de
l'overlay (auth, catalogue, référentiels) sont déjà des threads `std::thread` bloquants avec `ureq`
(voir `spawn_auth_thread`/`spawn_catalog_thread`) — introduire un runtime `tokio` pour la seule
file d'envoi aurait ajouté une dépendance lourde et un DEUXIÈME modèle de concurrence dans le même
binaire, pour un gain nul (le thread Sync ne fait jamais qu'une poignée de requêtes HTTP
séquentielles par passage, jamais de parallélisme à en tirer). Décision cohérente avec §13 (revue
d'architecture) : simplicité et homogénéité du modèle de threads plutôt qu'une optimisation sans
bénéfice mesurable.

Propriétés tenues : jamais bloquant pour l'Engine (`Engine::drain_sync_events` ne fait qu'accumuler
en mémoire, `overlay-ui::spawn_engine_thread` relaie par canal sans jamais attendre le thread Sync),
survit à un crash/une coupure réseau/un redémarrage (la file SQLite est écrite AVANT tout envoi),
rejeu sans conséquence grâce à l'idempotence (`SyncQueue::enqueue`, testé par rejeu 10x — voir
`crates/overlay-sync/src/queue.rs::tests`, critère de sortie du §12).

**Mode invité par défaut** : sans compte connecté (`uid` jamais résolu, voir plus bas), le thread
Sync n'envoie jamais rien — `SyncCommand::Enqueue` continue d'ÉCRIRE en file (persistance locale,
comme le web écrit quand même en IndexedDB en mode invité) mais `flush_once` n'est jamais appelé
sans `uid` connu. **Rien ne quitte la machine** tant qu'aucun compte n'est lié.

**`uid` (clé de `client_key = sha256(uid|kind|signature)`) résolu via `GET /api/v1/auth/me`**
(`overlay_sync::client::fetch_account_id`, nouveau — `AuthService.uid` côté web vient du cookie de
session, indisponible pour un client natif) : appelé une fois après chaque connexion réussie
(`attempt_connect` → `activate_sync_queue`), jamais par événement. Best-effort, comme le reste de
l'auth native : un échec laisse la file simplement inactive cette session (roster/watchlist restent
pleinement fonctionnels), sans jamais faire échouer toute la connexion pour ce besoin annexe.

**Génération des événements (`overlay_engine::history`, `crates/overlay-engine/src/session.rs`)**
— parité complète avec `StatsStoreService` sur tous les champs de `FightPayload`/`PurchasePayload`/
`TradePayload`, en s'appuyant directement sur le dépôt web local (`../wakfu-companion`, présent sur
ce poste de dev) pour retrouver les formules et raisonnements exacts plutôt que de les redeviner
(voir la doc de tête de `history.rs`/`dungeon_run.rs` pour le détail complet) :
- **Fait (2026-09-02)** : signatures, détection d'achat marchand/HDV, échanges, combats
  (participants, dégâts, `defeated`/`fled`), reconstruction de date calendaire réelle, ventilation
  des dégâts par sort/élément (dégâts uniquement, pas les soins), `xpGained` par participant ET
  total du combat (miroir de `registerFightXp`/`isRosterMember` — un nom qui n'a pas rejoint CE
  combat ne crédite ni le participant ni le total), résolution `monsterId`/`itemId` via le
  catalogue (`itemId`/`itemName` mutuellement exclusifs comme `HistorySyncService.itemPayload`),
  `gameServer` (déduit du dernier personnage du roster reconnu dans le log —
  `RosterAccount::game_server` + `Engine::current_game_server`/`notice_character`, miroir de
  `GameServerService`), récupération de kamas HDV sans achat adjacent (`HDV_KAMAS_SALE_ITEM`,
  corrélée à un `TradeCompleted` proche dans les deux sens, miroir de `considerHdvKamaGain`/
  `resolvePendingHdvKamaGain`/`flushPendingHdvKamaGain`).
- **Fait (2026-09-02, suite — regroupement de donjon multi-salles)** : `overlay_engine::
  dungeon_run` porte désormais `find_dungeon_for_enemies` (3 priorités : brèche ultime par boss
  distincts, donjon classique par boss, brèche simple par familles — miroir complet de
  `findDungeonForEnemies`, `fight-image.util.ts`) et `group_dungeon_runs` (port direct de
  `groupDungeonRuns`/`dungeon-run-grouping.util.ts` : cluster de tentatives contre un même boss,
  créneau archimonstre pré-boss optionnel, salles précédentes bornées par `roomSlots` avec
  vérification de composition — tous les cas documentés côté web, y compris les bugs déjà corrigés
  là-bas, sont couverts par des tests dédiés). `dungeon.rs` porte `WakfuDungeonType`/
  `has_pre_boss_archi` (mêmes champs que `GET /api/v1/dungeons`, jusque-là ignorés) et les
  recherches de brèche (`find_ultimate_breach_by_boss_monsters`/`find_breach_by_monster_families`).
  `SessionState::resolve_dungeon_assignment` (`session.rs`) regroupe TOUS les combats non `ongoing`
  de la session à chaque `CombatEnd`, et **renvoie aussi les SIBLINGS** d'un run tout juste complété
  (salles déjà envoyées sans rattachement) avec le même `dungeonId` et la même graine de run — miroir
  de `HistorySyncService.recordFight`, boucle `assignment.siblings` (nécessite de conserver l'heure
  de fin propre à chaque combat, voir `FightWorking::ended_at_time`/`ended_at_ms`, pour pouvoir
  rebâtir fidèlement un combat déjà terminé longtemps après coup).
- **Fait (2026-09-02, suite — branchement `overlay-ui`)** : `dungeonId`/`dungeonRunKey` sont
  désormais réellement alimentés en production, pas seulement prêts côté `overlay-engine` — nouveau
  thread `spawn_dungeon_thread` (`overlay-ui/src/main.rs`, miroir simplifié de
  `spawn_catalog_thread` : pas d'endpoint `/version` pour ce référentiel, cache disque chargé puis
  toujours remplacé par la version réseau) alimente un `Arc<ArcSwap<DungeonIndex>>` relayé à
  `Engine::set_dungeons` par le thread Engine, même mécanique de comparaison par pointeur que pour
  le catalogue. `spawn_engine_thread` regroupe ses `Arc<ArcSwap<_>>` dans un nouveau
  `struct EngineHandles` (5 paramètres au lieu de 8+1) — corrige au passage un dépassement
  `clippy::too_many_arguments` pré-existant plutôt que l'aggraver.
- **Reste, volontairement hors périmètre** : `turns` (nombre de tours) reste à `0`, aucun panneau
  n'en affiche le besoin et aucune formule ne le nécessite (jamais dans une signature).
- **Vérifié** : 133 tests (`overlay-engine` 114 + `overlay-sync` 19, dont une quarantaine nouveaux
  sur l'ensemble de ces lots : XP, ventilation sort/élément, résolution monsterId/itemId,
  gameServer, kamas HDV, hachage dungeonRunKey, `find_dungeon_for_enemies` 3 priorités,
  `group_dungeon_runs` — clusters de tentatives, archimonstre pré-boss réel/absent, salles bornées,
  deux runs successifs jamais fusionnés — et bout-en-bout via `SessionState::apply`, siblings
  compris), `clippy -D warnings`/`fmt --check` propres sur `overlay-engine`/`overlay-sync`/
  `overlay-ui`, `cargo build --workspace` propre. Les 2 erreurs clippy pré-existantes d'`overlay-ui`
  (fonction à 8 arguments, fermeture redondante), confirmées présentes sur l'état du dépôt avant ce
  lot (`git stash`), sont corrigées au passage — le workspace entier est maintenant propre sous
  `clippy -D warnings`.

### 7.4 Synchro des compteurs de Suivi (watchlist) — ✅ fait (2026-09-07)

Mécanisme **VOLONTAIREMENT distinct** de la file d'envoi du §7.3 (`SyncQueue`) — voir §14 point 3
pour la décision d'origine (compteurs locaux à l'overlay en v1) et son retour utilisateur de
fermeture (un Suivi jamais visible sur le site, la réplication manquait entièrement). Pas de file
SQLite ni de `client_key` idempotent ici : la clé `"watchlist"` de `PATCH /api/v1/settings` est
remplacée EN BLOC par le serveur (« dernier écrivain gagne » par horodatage, voir
`functions/api/v1/settings.ts::onRequestPatch` côté dépôt web) — rien à dédoublonner, seul le
DERNIER instantané compte.

- **Détection du changement** : `WatchlistState::drain_pending_sync`/`Engine::
  drain_watchlist_sync` (motif « drain », comme `drain_sync_events`) renvoient un instantané complet
  des entrées suivies dès qu'un compteur a changé (`apply`, à chaque `ingest_batch`) OU qu'un
  rattrapage est nécessaire (`merge_config` : le compte répond avec un `count` en retard sur le
  local, ex. un envoi resté en échec avant une fermeture de l'overlay).
- **Transport** : `overlay_sync::client::patch_watchlist(token, entries)` → `PATCH
  /api/v1/settings` avec `{ entries: [{ key: "watchlist", value: entries, updatedAt }] }` (lot d'une
  seule entrée = écriture par clé, pas de route `/settings/{key}` séparée côté serveur). Le porteur
  `Authorization: Bearer` accepté sans CSRF, même exception que pour l'historique (§7.2/`_auth.ts::
  requireCsrf`).
- **Débounce + backoff, sans persistance disque dédiée** : le thread Sync (`overlay-ui::
  spawn_sync_thread`, même thread que §7.3, mécanisme séparé) garde le DERNIER instantané reçu en
  mémoire (`pending_watchlist`), attend `WATCHLIST_DEBOUNCE` (1,5 s, miroir de `WRITE_DEBOUNCE_MS`,
  `RemoteUserDataRepository` côté web) avant le premier essai — un compteur qui s'incrémente à
  chaque kill d'un combat ne doit pas déclencher une requête par kill. Un nouvel instantané reçu
  PENDANT l'attente (débounce ou backoff) remplace le précédent et relance un débounce complet.
  Échec réseau : backoff 15 s→5 min (`backoff_delay`, même formule que §7.3), retenté indéfiniment
  (pas d'abandon après N tentatives comme `SyncQueue` : il n'y a qu'UNE valeur courante à répliquer,
  jamais une file qui grossit — rien à perdre en continuant de réessayer). Rien n'est jamais perdu
  côté overlay dans l'intervalle : `watchlist-counts.json` (fichier local, voir `watchlist.rs`)
  reste la source de vérité immédiate, la réplication réseau est un aval, jamais la seule copie.
- **Mode invité / rattrapage à la connexion** : sans compte connecté, `pending_watchlist` reste en
  attente (aucune requête tentée) jusqu'à `SyncCommand::Activate` — cohérent avec le §7.3 (« rien ne
  quitte la machine tant qu'aucun compte n'est lié »).
- **Divergence assumée pendant la fenêtre de debounce/backoff** : un overlay et le web utilisés en
  parallèle sur le même personnage peuvent afficher un `count` différent tant que la réplication
  n'a pas abouti — exactement le même compromis que `RemoteUserDataRepository` côté web (jamais
  instantané là-bas non plus).
- **Vérifié** : tests unitaires `overlay-engine::watchlist` (marquage `dirty`/drain, rattrapage
  `merge_config` selon que le compte est en retard ou déjà à jour, format de l'entrée `PATCH`) ;
  `cargo build`/`test`/`clippy -D warnings`/`fmt --check` propres sur `overlay-engine`/
  `overlay-sync` (natif Linux) et `overlay-ui` (cross-compilé `x86_64-pc-windows-gnu`, seule cible
  réellement visée par ce crate).

### 7.5 Catalogue

- `GET /api/v1/catalog/version` au démarrage → si changement, `GET /api/v1/catalog/` (≈ 349 Ko gzip).
- Cache disque dans `$XDG_CACHE_HOME` / `%LOCALAPPDATA%`, validé par `ETag`/version.
- Repli hors-ligne : `assets/catalog/catalog-index.json.gz` embarqué (`include_bytes!`), utilisé si
  aucun cache et pas de réseau — l'overlay reste utilisable, avec un bandeau « catalogue daté ».
- Index en RAM **O(1) par nom normalisé** (`HashMap<String, SmallVec<Entry>>`), construit une seule
  fois. Le piège vécu côté web (balayage O(catalogue) par ligne de butin, ~10 s de gel) ne doit pas
  être réintroduit : c'est un test de non-régression, pas une bonne intention.
- `GET /api/v1/game-servers`, `GET /api/v1/dungeons` : même schéma cache + repli.

---

## 8. Budget mémoire (cible 300 Mo)

| Poste | Estimation | Note |
| --- | --- | --- |
| Runtime graphique (winit + wgpu + pilote DX12/Vulkan) | 70–120 Mo | Poste dominant, largement hors de notre contrôle |
| egui (atlas de police, buffers) | 10–20 Mo | |
| QuickJS + bundle moteur | 5–15 Mo | Runtime + code |
| État métier (session de jeu typique) | 20–60 Mo | Historique, combats, chat |
| Catalogue en RAM (11 k objets, 4 langues + index) | 8–15 Mo | |
| File SQLite + client HTTP + divers | 5–10 Mo | |
| **Total nominal** | **120–240 Mo** | Marge confortable sous 300 Mo |

**Garde-fous, pas des vœux :**
- `xtask mem-budget` mesure le RSS après un scénario scripté (ingestion de 80 k lignes + 10 min
  d'affichage) et **échoue la CI au-dessus de 300 Mo**.
- Plafonds internes : chat borné à N messages, historique de session borné, purge des combats
  archivés déjà synchronisés (l'historique long terme vit côté serveur, pas en RAM).

---

## 9. Contenu de l'overlay

| Panneau | Contenu | Comportement |
| --- | --- | --- |
| **Dégâts de combat** | dégâts par allié/ennemi en temps réel, détail par sort et par élément, onglets multi-combats (multi-compte) | Ouvert automatiquement à l'entrée en combat, replié à la fin (configurable) |
| **Suivi (watchlist)** | compteurs objets/ennemis, mode incrémental ou décompte avec alerte | Persistant entre sessions — **jamais incrémenté pendant `isInitialLoad`** |
| **Alertes de drop** | toast (carte + confettis, miroir visuel de `loot-alert.component` du dépôt web) + son, sur ramassage à son activé (défaut ou ajouté au compte) ET sur décompte de suivi à 0 | Son configurable par objet (parité web) ; toast ≤ 5 s (minuterie fixe) OU fermé plus tôt par clic (carte ou croix) — les deux cohabitent, pas un réglage exclusif comme `ProfileService.alertManualClose` côté web |
| **Récap de session** (2026-09-16, §9.1 octodecies) | XP, kamas nets, combats gagnés − perdus, challenges réussis − échoués, durée de la session | Bande compacte en haut à gauche, sous les boutons du jeu ; `OverlayKind::Recap`, une par fenêtre de jeu ; masquée par la case « Activer le récap de session » (section « Recap » des Paramètres) |
| **État de synchro** | `idle`/`pending`/`syncing`/`error`, nombre en attente, dernière synchro | Discret ; l'erreur réseau ne doit jamais masquer le jeu |
| **Modale Options** (2026-09-08) | Onglets « Suivi », « Alertes » (§9.1 ter), « Raccourcis » (§9.1 quinquies) et « Paramètres » (chemin de `wakfu.log`, §5.1 — plus l'affichage du panneau Combat hors combat) | Ouverte par le bouton "Options" du carré de contrôle Suivi ou son raccourci (`Ctrl+Shift+O` par défaut, personnalisable) ; fenêtre OS dédiée, centrée sur la fenêtre de jeu, chrome du design system (bannière turquoise, ligne d'onglets, pied de page Annuler/Valider) ; rien n'est pris en compte avant "Valider" |

Raccourcis globaux : bascule interactif/traversable, afficher/masquer, panneau suivant.
Chaque panneau reste ancré automatiquement sur sa fenêtre de jeu (§6.5), avec opacité réglable.
Pas de repositionnement manuel des panneaux, pas de redimensionnement manuel ni de thème
configurable (retiré, décision du mainteneur, voir ci-dessous) — un overlay n'a pas vocation à
proposer une disposition personnalisable comme le ferait un site web.

> **Décision du mainteneur (2026-09-02) : pas de thème configurable, ni de mode daltonien.**
> Contrairement à l'app web, un overlay n'est pas un site : proposer un choix de thème/palette y
> introduirait une logique de personnalisation qui n'a pas sa place ici — l'objectif est de coller
> au design du jeu pour sortir le moins possible l'utilisateur de son immersion, pas de lui offrir
> un réglage supplémentaire. Palette fixe, telle que déjà codée en dur dans `panels::combat`/
> `panels::watchlist`. Retire ce point de la feuille de route (§12, L2) — le plan v1 de ce document
> (mode daltonien "repris du web") est explicitement abandonné, pas seulement reporté.
>
> **Même décision, disposition persistée par écran** : la poignée de glissement « ⠿ » et la
> persistance du décalage par écran (`layout_store`), un temps implémentées, sont retirées pour la
> même raison — laisser l'utilisateur repositionner et mémoriser la disposition de chaque panneau
> est une logique de personnalisation de site web, pas d'overlay. L'ancrage automatique
> (`App::anchor_position`, §6.5) reste la seule source de position.

### 9.1 Modale Options (2026-09-08)

Feuille de route de finalisation : [`plan-modale-options.md`](plan-modale-options.md)
(2026-09-10) — dix étapes, dont sept créent un composant réutilisable de `overlay_ui::design`.

Premier écran de réglages de l'overlay — voir `docs/design-system.md` §9 pour le chrome (mesuré sur
`assets/design-system/interfaces/interface-options-*.png`) et `crates/overlay-ui/src/panels/
options_modal.rs`. Un seul réglage en v1, conforme à §5.1 : le chemin de `wakfu.log` doit être
« toujours surchargeable par la config et par un sélecteur de fichier dans l'UI » — le fichier est
figé par la découverte automatique jusqu'ici, ce qui bloque un utilisateur dont l'installation vit
ailleurs (bêta, tests). Choix retenus :

- **Fenêtre OS dédiée** (`OverlayKind::Options`), pas un panneau dans une fenêtre existante — créée
  à la demande (bouton "Options" du carré de contrôle Suivi ou `Ctrl+Shift+O`), détruite à la
  fermeture (Annuler/Valider). Centrée sur la fenêtre de jeu (nouveau cas dans
  `App::anchor_position`), toujours `AlwaysOnTop`/`HWND_TOPMOST` sans jamais suivre le focus (exclue
  explicitement de `sync_windows`/`sync_topmost`, contrairement à Combat/Suivi) — une modale
  ponctuelle n'a pas besoin de la même politique de repli qu'un panneau d'information permanent.
  Seule fenêtre overlay qui accepte le focus clavier (`WS_EX_NOACTIVATE` omis côté Windows) : il
  faut pouvoir taper dans le champ de chemin.
- **Garde-fou de nom de fichier** (`overlay_ingest::discovery::validate_log_path`) : le fichier
  choisi (dialogue natif `rfd`, dont le filtre ne couvre que l'extension, ou saisie manuelle) doit
  s'appeler `wakfu.log` (insensible à la casse) ET exister sur le disque — sinon la modale reste
  ouverte avec un message d'erreur, rien n'est appliqué.
- **Rien n'est pris en compte avant "Valider"** — un brouillon local (`OptionsModalState`) porte la
  saisie ; Annuler l'abandonne sans effet.
- **Persistance** (`overlay_ui::config`, TOML via `directories::ProjectDirs`) + **rechargement à
  chaud** (`engine_thread::EngineCommand::ChangeLogPath`, respawn du watcher sur le nouveau chemin
  sans redémarrer l'overlay ni recréer l'`Engine` — roster/watchlist déjà appliqués sont conservés).
  Priorité de résolution au démarrage : argument CLI > config sauvegardée > découverte automatique.

**Révision visuelle 2026-09-09** — voir `docs/design-system.md` §9 bis pour le détail des mesures :
chrome porté sur les VRAIES textures du jeu (`crates/overlay-ui/assets/ui/options/`, chargées par
`panels::options_modal::OptionsModalAssets::load`, une fois par fenêtre OS comme
`panels::combat_frame::CombatFrame`) plutôt que des formes peintes à la main — coins ARRONDIS (pas
chanfreinés) pour la fenêtre elle-même et son encadré interne, menu à trois entrées (Alertes/
Personnages/Paramètres — « Alertes » câblé depuis le 2026-09-12, voir §9.1 ter ; « Personnages »
reste un stub), et un nouveau module `panels::nine_slice`
(étirement 9-slice générique, coins/bordure à taille native) pour agrandir le bouton "Sélectionner
le fichier" sans aplatir son chanfrein — méthode validée au préalable via une simulation HTML/CSS
avant portage, plutôt que d'itérer directement en Rust/egui.

### 9.1 ter Onglet « Alertes » de la fenêtre Options (2026-09-12)

Retour utilisateur après un test en jeu : « la partie alerte n'est pas accessible ». La **mécanique**
d'alerte existait depuis le 2026-09-02 (son + toast au ramassage d'un objet à son activé) mais
l'**écran de réglage** manquait — la liste ne se modifiait que depuis le site, et l'entrée de menu
était affichée désactivée. `panels::alerts_tab` la porte désormais, en appliquant la maquette
validée (`crates/overlay-testkit/examples/alertes-mockups.rs`, trois versions, la première refusée
par une revue à trois experts) à de vraies données.

- **Contenu** : titre et description, champ d'ajout par `design::autocomplete`
  sur le catalogue, et la grille de tuiles. Une tuile porte **deux informations qui ne se gênent
  pas** : bordure et pictogramme disent l'état du SON, emplacement de rareté et nom disent ce
  qu'est l'OBJET. Cliquer bascule le son ; la croix de retrait n'existe **pas** sur les dix objets
  de `DEFAULT_SOUND_ITEM_NAMES`, que le web refuse structurellement de supprimer.
- **Retrait confirmé** dans la boîte centrée du jeu (pas la popover du web), au voile couvrant la
  fenêtre ENTIÈRE — c'est lui qui dit que le pied de page est inerte. Bouton de confirmation **or,
  jamais rouge** : le rouge est réservé au « Annuler » pleine largeur.
- **Transactionnel**, comme le reste de la fenêtre : l'onglet travaille sur un brouillon
  d'`AlertProfile`, et « Valider » commit **tous les onglets à la fois** — le pied de page est
  partagé, un bouton dont l'effet dépendrait de l'onglet affiché serait imprévisible. Un chemin de
  log refusé n'écrit donc rien, alertes comprises.
- **Écriture au compte** : `PATCH /api/v1/settings` sur la clé `profile`, reconstruite **à partir
  de l'objet brut reçu au `GET`** (`AlertProfile::patch_value`) — le serveur remplace la valeur
  entière de la clé, et cette clé porte aussi le pseudo, l'avatar et le mode d'affichage des
  personnages, que l'overlay n'affiche nulle part. Écriture sautée si le brouillon n'a pas changé :
  l'arbitrage est « dernier écrivain gagne », une écriture inutile écraserait une modification
  faite depuis le site.
- **La durée réglée pilote vraiment le toast** : `WatchlistToast::hide_at` est passé à
  `Option<Instant>`, `None` valant « ne se ferme qu'à la main ». Régler une durée sans effet aurait
  été pire que pas de réglage.
- **Fenêtre agrandie à 760 × 810** (contre 560 × 436), ce que la maquette demande pour cinq tuiles
  par rangée et trois rangées visibles. Le rapport d'aspect 720:561 de la vraie fenêtre du jeu est
  abandonné au passage : arbitrage assumé au profit du contenu.
- **Trois captures** au testkit : liste, fermeture manuelle, confirmation de retrait.

**Garde de fermeture** (ajoutée le jour même) : « Annuler », Échap **et la croix de la fenêtre OS**
demandent confirmation tant que `OptionsModalState::is_dirty()` — chemin de log retouché, ou
brouillon d'alertes différent de la référence figée à l'ouverture. Le **changement d'onglet**
n'intercepte rien : le brouillon lui survit, et demander confirmation à chaque aller-retour rendrait
la fenêtre inutilisable. Un seul dialogue à la fois : une confirmation de retrait déjà ouverte fait
attendre la garde, deux voiles empilés étant deux fois plus sombres et leurs deux « Échap » se
marchant dessus.

Au passage, un défaut corrigé : **la croix de la fenêtre Options quittait l'overlay entier**
(`WindowEvent::CloseRequested` → `event_loop.exit()`). Cette fenêtre est la seule focalisable, donc
la seule dont la croix est réellement atteignable à la souris — elle ferme maintenant la modale, et
elle seule.

La boîte de confirmation a été **remontée au design system** à cette occasion (`design::confirm_dialog`,
voir le catalogue des composants) : deux appelants, donc sa place n'est plus dans un panneau.
L'extraction est à pixel constant.

### Deux défauts corrigés après un test en jeu (2026-09-12)

**1. La modale restait au-dessus de tout.** Elle naissait avec un `game_hwnd` nul — « pas rattachée
à une fenêtre de jeu précise » — ce qui obligeait `sync_windows` et `sync_topmost` à l'exclure
explicitement de toute leur logique. Conséquence : elle restait `HWND_TOPMOST` en permanence, y
compris quand l'utilisateur passait sur un navigateur, sur une autre application, ou sur un **second
client Wakfu** en multi-compte. Retour utilisateur : « à la différence des overlays Suivi et Combat,
la modale reste en premier plan, ce qui n'est pas bon ».

Elle est désormais **rattachée à la fenêtre de jeu depuis laquelle on l'a ouverte**, comme n'importe
quel overlay : le bouton « Options » du carré de contrôle passe le `HWND` de sa fenêtre, le raccourci
global prend celle au premier plan (à défaut le premier overlay connu). Les deux exemptions sont
supprimées — elle suit le premier plan de SON personnage, disparaît quand on regarde ailleurs, et
s'en va avec le client qu'elle configure. Même traitement côté X11 (`game_window`,
`_NET_ACTIVE_WINDOW`).

**2. Les noms d'objet étaient illisibles dans la grille.** Trois tuiles affichaient « Plan "Epée
de » à l'identique. La cause n'était pas la mise en forme du texte mais `Ui::put`, qui **avance le
curseur du parent** : la tuile suivante démarrait 27 px trop tôt et son fond opaque effaçait la fin
du nom de la précédente (voir le catalogue des composants, « Piège d'appelant »). Le défaut était
déjà dans la maquette validée et le portage l'a repris tel quel.

Corrigé — et c'était bien là tout le défaut. Le nom est passé **un moment** sur deux lignes pour
distinguer quatre libellés proches, puis **revenu sur une seule le jour même, sur décision de
l'utilisateur** : la tuile porte déjà l'icône de l'objet, et c'est elle qui lève l'ambiguïté, bien
avant le texte. « Ton problème de nom, c'est un faux problème puisque l'utilisateur voit des images
en plus des noms. » La tuile garde donc ses 88 px et la fenêtre ses 810.

Le nom passe par **`design::label`** (nouveau composant) : une ligne, ellipse au bout, et
l'infobulle qui rend le nom entier quand — et seulement quand — il est coupé.

Reste hors périmètre de ce lot : l'onglet « Personnages ».

### 9.1 bis Ligne de sorts du combat (2026-09-12)

Sous le dernier groupe de dégâts du panneau Combat, un bloc montre **les sorts lancés par un
combattant du camp affiché pendant son dernier tour** (un allié en vue Alliés, un ennemi en vue
Ennemis — vue Ennemis ajoutée le 13 sept., voir ci-dessous), en icônes numérotées dans l'ordre du log — spécification
validée en artefact avec l'utilisateur en quatre révisions (11-12 sept.), cotes reprises telles
quelles dans `overlay-ui::panels::combat_spell_block`. Décisions fermes :

- **Position** : dans le flux de la colonne des barres, **10 px** sous la barre du dernier groupe
  (18 px à l'origine, ramené le 12 sept. au soir — le même écart, `TOTAL_GAP`, sépare désormais le
  total du premier groupe, « pour l'homogénéité entre les blocs »), jamais en position fixe sous
  le gabarit ; **chaque vue montre le bloc de son camp** (vue Alliés seulement du 12 au 13 sept.).
  Bloc de 40 / 77 / 114 px pour une,
  deux ou trois rangées ; pire cas (six groupes, trois rangées) : 388 px, sous le bas du gabarit 6
  (437) — pas de changement de taille de fenêtre. Rien n'est affiché avant le premier sort allié.
- **Sélection par le cadre** (refonte du 12 sept. au soir, proposition en artefact en deux
  révisions, décidée) : la rangée d'onglets-portraits de 22 px du premier rendu, jugée trop petite
  et coûteuse en hauteur, est retirée. On choisit l'allié en **cliquant son portrait dans le cadre
  à médaillons** ; deux marques dorées (`#f4d89e`, décidé contre le violet Stasis et le cyan) : un
  **liseré** de 2 px au bord du portrait = l'allié dont on lit les sorts ; un **point** de 6 px en
  haut à gauche du portrait (à l'opposé du pourcentage, « comme une notification ») = le dernier
  allié à avoir lancé un sort, toujours à jour. Par défaut les deux sont sur le même allié (suivi
  automatique) ; clic sur un autre allié = épingle (le liseré reste, le point continue de suivre) ;
  clic sur le porteur du point ou second clic sur l'épinglé = retour au suivi. Un allié sans sort
  n'est ni cliquable ni marqué. Fondu de 150 ms, pas de glissade.
- **Remise à zéro à chaque nouveau tour** de l'allié, pas d'historique ; **pas de défilement
  latéral** : retour à la ligne tous les cinq sorts (32 px, 5 px d'écart — 3 px faisaient se
  toucher deux critiques voisins). Badge d'index **en haut à gauche** (1 px du bord gauche et du
  haut), critique = liseré doré + coin plié, infobulle sur une ligne « Nom · Critique » sans nom
  de lanceur.
- **Données** : `FighterDamage::last_turn_casts` / `FightSnapshot::last_ally_caster`
  (`overlay-engine::session`, alimentés dans `apply` sur `SpellCast` via le signal « nouveau tour »
  de `register_fight_turn`, persistés par `fight_store`). **Icônes** : référentiel
  `assets/spells.json` (maintenu à la main par l'utilisateur, embarqué, `overlay_engine::spells::
  SpellIndex`, clé nom normalisé + classe du lanceur — « Rafale »/« Poursuite » existent chez deux
  classes, plus une pseudo-classe `common` de `breedId` −2 pour les sorts communs à tous, résolus
  par le nom seul), `IconKind::Spell` sur le même circuit `RemoteIconStore` que les monstres ;
  sort absent du référentiel → pavé « ? » et un avertissement par nom ; entrée sans `picture` →
  tuile sombre sans « ? ». Référentiel complété par l'utilisateur le 12 sept. (462 entrées,
  mécaniques de classe comprises) : plus aucun « ? » sur le log de parité.
- **Testkit** : `tests/combat_spell_block.rs`, rejeu réel + fixtures PNG injectées par
  `RemoteIconStore::preload` (jamais le réseau), interactions par nœuds d'accessibilité.

**Vue Ennemis (2026-09-13)** — proposition « Sorts ennemis du combat » en artefact, revue par un
expert (validée avec réserves, toutes intégrées), décidée :

- **Mêmes règles, même géométrie, mêmes marques** que côté allié ; le composant
  (`combat_spell_block`) ne connaît plus « l'allié » mais **le camp affiché** (`SpellSelection::
  is_ally`). Épingle mémorisée par combat **et par camp** (`("combat-spell-block", fight_id,
  is_ally)`) : basculer le switch retrouve chaque camp comme on l'a laissé. Point = dernier lanceur
  du camp (`FightSnapshot::last_enemy_caster`, pendant de `last_ally_caster`).
- **Icône** : nouveau référentiel `assets/monster-spells.json` (maintenu à la main, embarqué,
  `overlay_engine::spells::MonsterSpellIndex`), clé nom normalisé + **`breed` de la ligne de
  jointure** (`FighterDamage::breed`, nouveau, `#[serde(default)]`) — c'est l'identifiant du monstre
  (`Grokoko breed : 4728` ↔ `breedId: 4728`), 69 noms ayant des images différentes selon le
  monstre (« Coup d'Koko » : `spells/3.png` chez le Grokoko, `spells/4.png` chez le Kokoko).
  Repli nom seul DANS ce fichier (combat restauré d'avant le champ `breed`), jamais vers le
  référentiel de classe. Les deux index partagent une table générique (`SpellTable<K>`) et l'UI
  passe par `overlay_engine::resolve_cast(fighter, spell)`, seul à savoir quel fichier répond.
  Sort absent → « ? » + un avertissement nommant le fichier à compléter. Les deux index sont
  construits **au démarrage de l'overlay** (`preload_spell_indexes`), jamais au premier sort.
- **Ennemis nombreux** (cadre à défilement, > 6) : marques et clics sur les portraits visibles
  seulement ; **pas de défilement automatique** vers le dernier lanceur (le défilement reste à
  l'utilisateur), le bloc montre ses sorts quoi qu'il en soit. Identifiant egui des médaillons
  désormais par emplacement (deux Grokoko partageaient le même `Id` : clic attribué aux deux).
- **Deux corrections d'attribution (2026-09-13, retour utilisateur : « deux sorts du même nom
  rapprochés, un seul affiché »)**, toutes deux dans le parseur vendu (`engine-js/`, consignées
  dans `VENDORED_FROM.txt` — l'overlay et le web n'ont pas les mêmes fonctionnalités, le parseur
  diverge normalement, décision utilisateur du 13 sept.) :
  - **Relancers avalés par la déduplication multi-compte** : `isDuplicate` jetait tout
    événement identique à moins de 1 s ; un vrai relancer du même sort (Croc-en-jambe deux fois
    à 724 ms, chacun avec ses dégâts) disparaissait. Mesuré sur le log de parité en séparant les
    combats observés par un ou deux clients (jointures `[_FL_]` dupliquées) : les copies d'un
    second client sont toutes à moins de 452 ms, les relancers réels tous à plus de 724 ms.
    Fenêtre dédiée aux `spell-cast` : **600 ms** (`SPELL_CAST_DEDUPE_WINDOW_MS`), 18 lancers
    récupérés sur le log de parité. Les dégâts et soins gardent la fenêtre de 1 s — même
    mécanisme, même risque théorique, non mesuré, à reprendre séparément.
  - **Homonymes de part et d'autre d'un tour allié muet** : « N secondes reportées pour le tour
    suivant. » est émis à la fin du tour de chaque personnage du joueur, même passé sans sort
    (nouvel événement `turn-ended` / `LogEntry::TurnEnded`, rattaché au combat courant). Le
    moteur y désarme le raccourci « même acteur = même tour » (`FightWorking::end_own_turn`) :
    « Grokoko, fin de tour allié, Grokoko » est désormais résolu par la file d'initiative — l'autre
    Grokoko, ou le même avec un nouveau tour s'il n'y en a qu'un. Ne couvre pas les alliés d'un
    autre joueur (aucune ligne pour eux) ni un tour où tout le temps a été consommé.
- **Limites connues, acceptées** : deux homonymes consécutifs sans AUCUNE ligne entre eux (allié
  intercalé KO, dont le tour est sauté par le jeu) restent fusionnés sur un siège (test
  `deux_homonymes_consecutifs_sont_fusionnes_sur_un_siege`) — indécidable depuis le log, qui ne
  porte l'identifiant d'instance que sur la jointure ; le nom du lanceur reste absent de l'infobulle même quand le liseré est
  hors de la bande visible (suggestion de la revue, non retenue en révision 1 pour garder les
  règles strictement identiques aux deux camps — à reconsidérer sur retour en jeu). Alliés au-delà
  de six (liste plate) : toujours ni marque ni clic — asymétrie assumée avec les ennemis > 6.
- **Testkit** : captures `combat_spell_block_ennemis_{suivi_auto,epingle,survol}` (fixtures
  `3.png`/`4.png`), les quatre captures alliées inchangées ; l'aller-retour Alliés → Ennemis →
  Alliés recompare la capture alliée d'origine.

Alliés au-delà du sixième (liste plate, cas rare) : pas encore de marque ni de clic sur leur
portrait — à ajouter si le cas se présente. Option « n'afficher que mes personnages » (roster ∩
alliés) : plus tard.

### 9.1 quater Barres de dégâts en breach (2026-09-13)

Maquette « Barres en brèche » validée en artefact interactif (deux révisions, choix arrêtés par
l'utilisateur), implémentée dans `panels::combat_bars` (voir sa doc de module) :

- **Plafond** : la colonne des barres ne dépasse jamais six groupes — le plus grand gabarit du
  cadre (`MAX_FRAME_SLOTS`) reste la mesure de tout le panneau. Au-delà, les groupes s'accumulent
  dans un conteneur invisible (ni bordure ni fond) haut de six groupes exactement, qui défile.
  En dessous, la fenêtre se resserre sur son contenu : rythme vertical identique à l'ancienne
  liste, les captures alliées de référence sont inchangées au pixel. Le plafond compte les
  **groupes** (combattants à dégâts > 0), pas les monstres.
- **Hors défilement** : ligne leader au-dessus, bloc « ligne de sorts » au-dessous, toujours
  visibles.
- **Ascenseur** : celui du cadre à défilement, extrait dans `panels::combat_scrollbar` et partagé
  (5 px, `#998a6c`, liseré noir extérieur, coins à 2 px, 20 px minimum, zone de saisie 14 px) —
  à **gauche** de la fenêtre, dans l'air entre le cadre et les barres, en face de celui du cadre ;
  toujours visible dès qu'il y a plus de six groupes. Molette au survol de la fenêtre et de
  l'ascenseur, décalage en pixels persisté comme celui du cadre (`scroll_offset_id`).
- **Fondu de 8 px** aux bords, uniquement du côté où du contenu est masqué. egui ne masquant pas en
  alpha ce qui est déjà peint, et le fond étant transparent, le fondu est appliqué élément par
  élément (ligne nom + dégâts, puis barre : opacité = moyenne du masque sur la partie visible) —
  continu au défilement, par tranches de 13 et 16 px plutôt que par pixel. Écart assumé avec la
  maquette.
- **Testkit** : `combat_breche_6_ennemis` (ni ascenseur ni fondu), `combat_breche_14_ennemis`,
  `..._defile`, `..._fin` — panneau complet peint via `paint_content` sur un `FightSnapshot`
  construit à la main (même exception que `tests/combat_frame_scroll.rs`).

### 9.1 quinquies Onglet « Raccourcis » de la fenêtre Options (2026-09-13)

Demande utilisateur : « ajouter un onglet "Raccourcis", **avant paramètre**, pour permettre à
l'utilisateur de personnaliser les raccourcis de l'overlay », sur le modèle de l'onglet
« Commandes » du jeu (`assets/design-system/interfaces/interface-options-commandes.png`).
Jusque-là, les neuf combinaisons d'alors étaient des constantes de `main.rs` — changeables seulement en
recompilant. `overlay_ui::shortcuts` en devient la **source unique** (liste des actions,
combinaisons par défaut, lecture/écriture de la config, enregistrement auprès de l'OS partagé par
les deux binaires) et `panels::raccourcis_tab` l'écran qui les édite.

- **Ce que la référence donne, et ce qui en est repris** : champ « Rechercher » en tête, groupes
  (« Overlay », « Suivi », « Combat », « Compte » — `ShortcutAction::section`), lignes
  `libellé → champ`, bouton de réinitialisation. Deux écarts assumés : le nom de groupe est porté
  par l'**en-tête du tableau** (`design::table`) et non par un `design::heading`, qui peint son
  libellé 7 px à gauche de son rectangle et se faisait rogner par l'écrêtage de la liste défilante ;
  et la réinitialisation est un **bouton texte** au bout de la ligne de recherche, la bannière
  portant déjà sa croix de fermeture.
- **Défauts inchangés** : `Ctrl+Shift+W`/`R`/`Q`/`O`/`D`/`A`/`S`/`E` et `Ctrl+Alt+D`, exactement les
  anciennes constantes — une mise à jour ne change rien sous les doigts de qui n'a rien
  personnalisé. Leur POURQUOI (pas de touche de fonction nue, pas d'Échap, etc.) est conservé dans
  la doc de chaque variante de `ShortcutAction`.
- **Au moins un modificateur** (`Shortcut::is_valid`) : ces raccourcis sont GLOBAUX
  (`RegisterHotKey`/XGrabKey), une touche nue serait volée à Wakfu lui-même. **Une exception depuis
  le 2026-09-13** : les touches de fonction nues (F1-F12), qui ne s'écrivent pas et que les
  raccourcis multicompte demandent telles quelles — voir §9.1 sexies.
- **Doublon refusé avant validation** (`ShortcutBindings::conflict`) : l'OS rejetterait le second
  enregistrement (même `HotKey::id`). Signalé dès la frappe, et re-vérifié par
  `OptionsModalState::validate` quel que soit le geste qui valide.
- **Raccourcis suspendus tant que la fenêtre est ouverte** (`ShortcutRegistry::suspend`, rendus au
  `close_options_modal`) : sans cela, l'OS avalerait la frappe que l'utilisateur essaie justement
  d'assigner — à commencer par la combinaison qui vient d'ouvrir la fenêtre.
- **Échec d'enregistrement non fatal** : une combinaison déjà prise par une autre application est
  journalisée et laisse l'action sans raccourci pour la session — plus de `expect` qui ferait
  tomber l'overlay, risque devenu réel dès lors que l'utilisateur choisit lui-même les touches.
- **Transactionnel**, comme le reste de la fenêtre : brouillon de `ShortcutBindings`, protégé par
  la garde de fermeture (`is_dirty`), appliqué et persisté seulement à « Valider », avec le chemin
  de log et le reste — la table `[shortcuts]` du même `config.toml`, tolérante (clé inconnue
  ignorée, combinaison illisible remplacée par le défaut, table absente = tous les défauts).
- **Libellés propagés jusqu'aux infobulles** (`RenderContent::shortcuts`) : les boutons du carré de
  contrôle et le switch Alliés/Ennemis affichent la combinaison RÉELLE, plus une chaîne recopiée.
- **Portée Linux** : `bin/overlay-ui-x11.rs` n'enregistre que `ShortcutAction::LINUX_SUPPORTED`
  (bascule, quitter, Options, sélection multiple, **plus les deux actions multicompte** depuis le
  2026-09-13 — §9.1 sexies) faute de câblage pour les autres — celles-ci restent éditables et
  persistées, un même `config.toml` servant aux deux OS.

**Section « Compte » de l'onglet « Paramètres »** (même jour) : titre, bloc d'information disant que
l'overlay ne fonctionne qu'avec un compte connecté et que se déconnecter ramène à l'écran de
connexion, puis un bouton « Déconnecter ». Trois décisions :

- **Ce bouton n'est pas un brouillon**, contrairement à tout le reste de la fenêtre : il agit tout
  de suite, et « Annuler » ne le rattraperait pas. C'est ce qui justifie la **confirmation** qu'il
  ouvre (`design::confirm_dialog`) là où l'onglet « Alertes » a pu retirer la sienne — son retrait
  d'objet, lui, restait annulable jusqu'à « Valider ».
- **Bouton secondaire (kaki), pas `Danger`** : le rouge de cette fenêtre est celui du « Annuler »
  plein-largeur du pied de page, que le design system réserve à ce pattern. C'est la confirmation
  qui porte l'avertissement, pas la couleur.
- **Désactivé sans compte lié** (`OptionsModalState::account_connected`, posé par l'hôte qui seul
  connaît `AuthStatus`) — toujours le cas du binaire Linux, en mode invité fixe.

### 9.1 sexies Raccourcis multicompte : inviter / suivre l'autre personnage (2026-09-13)

Demande utilisateur : « en multicompte, on souhaite généralement suivre son second compte et
j'aimerais rendre facile ces interactions [...] utiliser le nom du personnage de la fenêtre qui
n'est PAS la fenêtre en focus, pour pouvoir directement appliquer les raccourcis textuels du jeu
[...] `/i "<nom>"` pour inviter, `/fol "<nom>"` pour suivre [...] ajouter les raccourcis **F1**
(inviter) et **F2** (suivre) ».

Deux nouvelles actions (`ShortcutAction::InvitePartner`/`FollowPartner`, section « Multicompte » de
l'onglet « Raccourcis »), un module `overlay_ui::chat_command` pour la commande elle-même, et un
module `overlay_platform::linux::keyboard` pour la frappe synthétique côté X11.

- **Le nom du personnage ne se saisit nulle part** : il est déjà dans le titre de la fenêtre de jeu
  (`"<Nom> - WAKFU"`, §6.5), que l'overlay scrute en continu pour s'ancrer. C'est ce qui rend le
  geste « gratuit » — aucun réglage, aucune liste de comptes à tenir à jour.
- **La cible est la fenêtre qui n'a PAS le focus** (`chat_command::partner_character`, fonction pure
  testée) ; la frappe, elle, part dans la fenêtre qui l'a — celle où le joueur écrit son chat.
  L'overlay n'active ni ne déplace jamais aucune fenêtre (ses propres fenêtres portent
  `WS_EX_NOACTIVATE`) : la frappe synthétique suit simplement le focus clavier réel.
- **Rien n'est envoyé si le premier plan n'est pas une fenêtre de jeu**
  (`PartnerError::NoGameFocused`) : F1 dans un navigateur ne doit pas y écrire `/i "..."`. **Le
  revers est assumé** : la touche reste confisquée à l'application au premier plan tant que
  l'overlay tourne (un raccourci global est un `RegisterHotKey`/XGrabKey, il n'y a pas de « laisser
  passer »). C'est le seul coût réel de cette fonctionnalité, et il est le prix des touches nues que
  la demande réclame.
- **Touches de fonction nues autorisées** (`Shortcut::is_valid`, exception à la règle du
  modificateur, §9.1 quinquies) : le geste doit être aussi immédiat que les raccourcis du jeu qu'il
  imite — un `Ctrl+Shift+…` à trois doigts en plein combat raterait l'intention. L'exception est
  limitée à F1-F12 : une lettre nue serait volée à Wakfu dès la première ligne de chat écrite.
- **La séquence tapée** : `Entrée` (ouvre la saisie du chat), la ligne, `Entrée` (envoie), avec des
  délais entre les frappes (140 ms après l'ouverture, 12 ms entre caractères). Le client Wakfu est
  en Java et échantillonne le clavier par image : une rafale envoyée d'un bloc lui ferait perdre des
  caractères, ou les ferait interpréter comme des raccourcis de jeu avant que le chat ait le focus.
  Le tout sur un **thread dédié** (~300 ms) : le tenir dans la boucle d'événements figerait
  l'overlay à chaque appui.
- **Le nom est cité et vérifié avant de l'être** (`PartnerError::UnsafeName`) : les noms Wakfu
  peuvent contenir une espace (« Sagittarius Caecus »), que le jeu couperait au premier mot sans les
  guillemets ; et un titre de fenêtre est une donnée EXTERNE — un guillemet ou un retour à la ligne
  qui s'y glisserait sortirait de la citation et ferait taper une seconde commande.
- **Deux implémentations, une seule logique** : `SendInput` + `KEYEVENTF_UNICODE` sous Windows (la
  frappe porte le caractère, pas une touche physique — indépendante du layout) ; **XTEST** sous X11
  (`overlay_platform::linux::keyboard`), avec traduction caractère → keysym → keycode dans le layout
  COURANT de l'utilisateur, niveau Maj compris, et emprunt temporaire d'un keycode libre à la
  `xdotool` pour un caractère absent du layout — le layout est restauré même si la frappe échoue.
  `XSendEvent` n'était pas une option : son drapeau `send_event` est ignoré par AWT, donc par le
  client Wakfu.
- **Trois clients ou plus** : le partenaire est la première fenêtre de jeu sans focus dans l'ordre du
  scan — ordre de profondeur sous Windows (`EnumWindows`), ordre de création sous X11
  (`_NET_CLIENT_LIST`). Sans ambiguïté à deux clients, le cas visé ; au-delà, un choix arbitraire
  mais stable, jamais une commande envoyée au hasard.
- **Limite connue** : un joueur qui aurait rebindé la touche de chat dans Wakfu (autre qu'`Entrée`)
  verrait la séquence échouer sans trace côté jeu — à rendre configurable le jour où le cas se
  présente.

### 9.1 septies Bandeau « Suivi » : boutons fixes, bande défilante, barre du jeu (2026-09-13)

Quatre demandes d'un même retour utilisateur, capture d'un bandeau volontairement surchargé à
l'appui (« j'ai fait en sorte d'avoir énormément d'objets suivis pour faire afficher la
scrollbar ») :

- **Les boutons ne défilent plus.** Le carré « + / − / Détails / Options » était le premier enfant
  de la zone défilante : défiler la bande l'emmenait hors de l'écran avec les tuiles. Il est
  maintenant peint DEHORS, dans la rangée qui porte cette zone — « les boutons sont fixes, il
  devrait y avoir un conteneur qui affiche les éléments suivis, un peu comme le scroll pour les
  ennemis » (§9.1 quater, où seuls les portraits défilent dans un cadre immobile).
- **L'overlay démarre au premier pixel des boutons.** La réserve d'infobulle de 48 px à gauche du
  carré (`CONTROL_TOOLTIP_RESERVE`) et les 6 px de marge interne gauche sont tombées : « il y a
  cinquante pixels à gauche des boutons, alors que le groupe de boutons c'est le démarrage de
  l'overlay ». Contrepartie admise dans la même demande : les infobulles du carré ne sont plus
  CENTRÉES sur leur bouton mais rabattues sur le bord de la fenêtre — une popup ne peut pas se
  peindre hors de la fenêtre qui la contient. La réserve DROITE reste, elle : sans elle, la fenêtre
  d'un bandeau vide (116 px) serait plus étroite que la moindre infobulle.
- **Moins d'air en haut.** `WATCHLIST_TOP_MARGIN` 36 → 28 px et `WATCHLIST_HEIGHT` 132 → 92 px :
  les deux constantes réservaient de la place qui ne servait plus (l'espace du toast, compté deux
  fois ; la bande dégagée sous les tuiles pour une barre flottante qui n'existe plus). Le reste de
  l'écart visible en jeu vient de l'ancrage de la fenêtre (`GAME_TOP_MARGIN_PX`, 28 px sous le bord
  haut du client), calé sur les boutons du jeu et laissé tel quel.
- **La barre de défilement est celle du jeu.** `ScrollStyle::thin()` d'egui (flottante, fine au
  repos, qui GROSSIT au survol, à moitié translucide) est remplacée par `design::scroll_area`
  couchée à l'horizontale (§9.2) : 6 px d'épaisseur constante, pas de rail, `#515356` au repos,
  `#c1ad83` au survol et au glissé. Demande mot pour mot : « je ne veux pas que le scroll
  s'agrandisse […] utiliser le scroll qui est déjà utilisé pour la modale dans l'onglet
  Raccourcis […] gris quand l'utilisateur n'a pas sa souris dessus et doré quand il passe sa souris
  dessus, c'est mieux dans l'ADN du jeu ». Seule la marge extérieure diffère du relevé (2 px au lieu
  de 14 : le bandeau n'a pas de bord de panneau à respecter).

**Testkit** : `panneau_suivi_bande_defilante_boutons_fixes` — bande au repos, poignée survolée
(dorée, MÊME épaisseur), et bande réellement défilée à la poignée pendant que le carré ne bouge pas.
Au passage, les harnais du bandeau demandaient 16 px de moins que la vraie fenêtre (`egui_kittest`
ajoute 8 px de marge sur les quatre côtés) — sans effet tant que la fenêtre était large devant son
contenu, mais la barre, peinte SOUS les tuiles, tombait hors du cadre et n'apparaissait sur aucune
capture.

### 9.1 octies Bandeau « Suivi » : toutes les infobulles en dessous, la bande remonte (2026-09-13)

Retour du même jour, au soir, sur le résultat de §9.1 septies — deux captures, dont un bandeau sans
aucun suivi : « que la bande de suivi ne soit pas autant décalée par rapport au haut de la fenêtre
du jeu, c'est très dérangeant visuellement, et encore plus lorsqu'il n'y a pas du tout de suivi —
il y a quatre boutons qui flottent dans le vide, c'est très perturbant ».

Les 28 px de `WATCHLIST_TOP_MARGIN` ne servaient qu'à loger les infobulles de « + »/« − » et des
tuiles, ouvertes vers le haut depuis le matin même. Le remède est venu avec la demande :
« intervertir les choses [...] le tooltip d'Ajouter se comporterait comme celui de Détails, il
s'afficherait en bas du bouton ; pareil pour Options et Supprimer [...] en collant la bande plus
haut et en laissant juste l'espace nécessaire — on gagnerait la moitié, peut-être plus, du vide ».

- **Tout s'ouvre en dessous** — les quatre boutons du carré ; une tuile, elle, ouvre son nom à
  l'écart du composant, voir la barre plus bas — et la réserve change de côté : `WATCHLIST_TOP_MARGIN` devient
  `WATCHLIST_TOOLTIP_RESERVE`, même valeur mais laissée en hauteur de fenêtre SOUS le contenu, là
  où elle poussait le contenu vers le bas. La bande touche le bord haut de sa fenêtre ; ce qui
  l'éloigne encore du client n'est plus que l'ancrage (`GAME_TOP_MARGIN_PX`, 28 px, calé sur les
  boutons d'interface du jeu). **62 px de vide → 28.** La hauteur de fenêtre, elle, ne bouge pas :
  ce qui était réservé au-dessus l'est maintenant en dessous.
- **Un côté ne suffit plus à éviter le recouvrement** : sous « + », il y a « Détails ». Les quatre
  infobulles du carré s'accrochent donc au carré ENTIER (`design::Tooltip::anchor`, §9.2) et
  s'ouvrent sous sa dernière ligne ; celle d'une tuile s'accroche à la bande jusqu'au bas de sa
  zone défilante, barre de défilement comprise — ouverte sous la seule tuile, elle masquerait la
  barre qu'on vient d'y mettre (§9.1 septies). En rangée (bandeau vide), rien à protéger : chaque
  infobulle reste centrée sur son bouton, ce qui dit lequel elle décrit.
- **La barre de défilement passe AU-DESSUS des tuiles** (`design::ScrollArea::bar_before`, §9.2).
  Première version : elle restait dessous, l'infobulle d'une tuile s'ancrant alors au pied de la
  zone défilante pour ne pas la masquer. Retour immédiat, et il porte sur les deux : « je veux que
  la part de scroll soit au-dessus de la ligne de suivi, pas en dessous [...] il faut laisser un ou
  deux pixels au-dessus de la barre de scroll seulement », et les noms d'objets étaient
  « extrêmement loin, pas dans les conventions du composant ». La barre en tête libère le bas :
  l'infobulle d'une tuile revient à `TOOLTIP_GAP` (5 px) sous elle. La marge interne HAUTE de la
  fenêtre tombe à 0 dans le même mouvement — les 2 px au-dessus de la barre appartiennent à la
  bande (`STRIP_SCROLLBAR_OUTER_MARGIN`), et quand la bande tient entière il n'y a pas de barre
  du tout, donc rien à dégager : les boutons touchent alors le bord haut de leur fenêtre.
- `CONTROL_TOOLTIP_RESERVE` remesurée une dernière fois : 88 px, à DROITE seulement. Une infobulle
  ancrée sur le carré (60 px) déborde plus loin que centrée sur un bouton ; deux entrées suffisent
  à la fournir, une seule coûte 12 px de largeur de fenêtre transparente.

**Dernier ajustement du même jour — le carré de contrôle fait une CASE.** « Le carré de boutons
est mis au même niveau en haut qu'un item slot, sauf qu'un item slot est plus haut que le carré :
soit qu'il soit aligné au milieu de manière verticale, soit qu'on augmente la taille des quatre
boutons pour que le carré, avec le fond de section en plus, fasse la hauteur d'un item slot. » Les
cotes : tuile 64 px, carré 60 (`4 × 3` de marge + `24 × 2` de bouton), 4 px d'écart tous en bas.
Les deux propositions ont été rendues à l'échelle (`overlay-testkit/examples/
bandeau-carre-hauteur.rs`), et **la seconde retenue** : `CONTROL_BUTTON_SIZE` passe de 24 à 26 px,
`control_row_height` vaut alors exactement 64 — hauts alignés, bas alignés, la trame d'une grille
d'inventaire. Le centrage de la première (`control_row_centering`) reste en place et devient neutre
de lui-même : il vaut la moitié de ce qui manque au carré, donc zéro. Écrit comme un calcul et non
comme une constante, il tiendra aussi si la cote des boutons rebouge.

**Testkit** : `panneau_suivi_toutes_les_infobulles_sous_la_bande` capture les cinq survols (quatre
boutons + une tuile). Au passage, les quatre survols de
`panneau_suivi_vide_boutons_en_ligne_infobulles_dessous` visaient 70 px trop à droite depuis le
retrait de la réserve gauche (§9.1 septies) : deux captures montraient l'infobulle d'« Options »,
deux n'en montraient aucune, et aucun test n'échouait. Un nom de fichier n'est pas une assertion —
seule la lecture de la capture publiée en artefact l'est.

### 9.1 nonies Onglet « Chat » : recherches, son et carte de réponse (2026-09-14)

Portage du panneau Chat du dépôt web, **recadré par l'utilisateur** après une première maquette
qui reportait le panneau entier (fil de messages, cases de canaux) : le jeu affiche déjà son chat
et filtre déjà ses canaux, l'overlay n'a rien à en remontrer. Ce qui manque au joueur, c'est
d'**être prévenu** quand un message correspond à un critère qu'il a posé. Maquettes :
`crates/overlay-testkit/examples/chat-mockups.rs` (v3, tuiles retenues contre la liste en lignes).

- **Moteur** (`overlay_engine::chat_alert`) : une recherche = un mot (trimé, minuscules) et une
  portée (un canal, ou tous). Règle du web reprise telle quelle : `contains` sur le texte OU
  l'auteur, en minuscules, sans expression régulière ni mot entier. `LogEntry::Chat`, parsé depuis
  L2 mais jeté jusqu'ici, est confronté aux recherches dans `Engine::apply_entry`, **sous le même
  gating `is_initial_load` que le ramassage** : un message déjà dans le fichier à l'ouverture ne
  sonne pas. File `drain_chat_alerts`, séparée des deux autres.
- **Compte** : clé `chatFilters` de `/api/v1/settings`, déjà dans la liste blanche du serveur, lue
  au `GET` (`AccountSettings::chat_filters`) et réécrite en entier au `PATCH`
  (`client::patch_chat_filters`) au **format du web** — `[{ text, channel }]`, `channel` valant
  `global` ou la clé d'un canal, anciens éléments texte relus comme des recherches globales.
- **Onglet** (`panels::chat_tab`, entre Alertes et Personnages) : formulaire canal → mot →
  « Ajouter » (Entrée ajoute aussi ;
  vide et doublon refusés avec une phrase), grille de tuiles à légende
  (`design::legend_tile`, §9.2) quatre par rangée, croix au survol. **Sans couleur de canal** : les
  thèmes du jeu. Transactionnel comme les autres onglets (`OptionsModalState::chat_draft`,
  `is_dirty`, `main.rs::commit_chat`).
- **La durée de la carte est locale** (`config::OverlayConfig::chat_alert_*`), par exception au
  principe « ce qui appartient au joueur passe par le compte » : ce réglage n'a pas d'équivalent
  web et le serveur n'accepte que des clés connues. Le jour où il en porte une, il y migre.
- **Son et carte** : le son du web (`chat-filter-c13da61f.mp3`, `alert_sound::play_chat_alert`),
  **une fois par lot** et pour le dernier message trouvé ; carte `WatchlistToastReason::Chat`
  dans le bandeau Suivi — **le gabarit des tuiles de recherche** (`design::LegendTile::paint_frame`,
  révision du 2026-09-14) : le canal en légende sur la bordure haute, **à droite, en 16 px, sans
  fond ni ombre portée** (sa moitié haute déborde du cadre par-dessus le jeu ; à ce corps et en
  couleur elle s'y lit, décision utilisateur), dans la couleur que le client lui donne (`tokens::chat_channel_color`) ; dedans « Recherche : « mot » » en gris italique au corps du message, l'auteur en doré
  puis le message complet (retour à la ligne à 420 px) ; **largeur fixe** (`CHAT_CARD_WIDTH`),
  hauteur au texte. Sans confettis. À droite, la bulle de message (`DsIcon::Message`, icône
  fournie par l'utilisateur, blanche puis cyan sur halo au survol) prépare la réponse en privé ;
  cliquer la carte elle-même la ferme, comme les autres cartes (inversion du 2026-09-14).
- **Réponse en privé** : un clic sur la bulle de la carte la ferme et prépare `/w "<auteur>" ` dans le jeu
  (`chat_command::send_whisper`, espace final, **sans Entrée final** : le joueur tape son
  message). Séquence sur un thread : rendre le focus à la fenêtre de jeu au premier plan ou à la
  première trouvée (`SetForegroundWindow` / `_NET_ACTIVE_WINDOW`, nouveau
  `overlay_platform::linux::x11::GameWindowTracker::activate`), un délai, Entrée, la ligne. Comme
  la fermeture au clic des autres cartes, cela demande le mode interactif de l'overlay.
- **Vérifications** : 7 tests unitaires + 2 d'intégration côté moteur, tests de brouillon, de
  config et de garde de fermeture côté UI, quatre captures (`options_chat_*`,
  `watchlist_avec_carte_de_chat` et `_courte` — produites par le vrai moteur, la seconde bulle survolée), planche
  `design_gallery_legend_tile`. Les 28 captures de la fenêtre Options ont été régénérées : la
  barre d'onglets compte une entrée de plus.

### 9.1 decies Notification de tour (2026-09-14) — spécification, décisions acquises, reste à valider

**La demande, dans les mots de l'utilisateur** : « avertir l'utilisateur qu'un de ses personnages
a démarré son tour et qu'il doit jouer », par une **notification du système**, uniquement quand la
fenêtre de jeu concernée n'est pas celle qu'il a sous les yeux. Réglée par une case à cocher — la
section « Combat » de l'onglet Paramètres, livrée ce jour (`config::OverlayConfig::
turn_notification`), décochée par défaut.

#### L'exigence qui structure tout : ça marche dans tous les cas, ou ça ne sert à rien

Reformulation de l'utilisateur le 2026-09-14, qui **remplace** la lecture initiale (« multicompte,
même combat ») :

> Dès lors que la fenêtre n'est pas en focus, elle doit surveiller à qui c'est de jouer. […] Peu
> importe si je suis en multicompte, en monocompte, dans le même combat ou pas dans le même combat.

Quatre situations, toutes à couvrir :

| | Fenêtres Wakfu | Au premier plan | Ce qu'il faut lire |
| --- | --- | --- | --- |
| A | une | un navigateur | la fenêtre de jeu, **en arrière-plan** |
| B | deux, même combat | une fenêtre de jeu | celle du premier plan suffit |
| C | deux, combats séparés | une fenêtre de jeu | **l'autre fenêtre**, en arrière-plan |
| D | deux | un navigateur | **les deux**, en arrière-plan |

Trois cas sur quatre exigent de lire une fenêtre **qui n'a pas le focus**. C'est donc la brique
centrale, pas un cas limite — à l'inverse de la lecture retenue le matin même, qui n'aurait couvert
que B.

Le cas B reste utile comme **raccourci** : quand toutes les fenêtres partagent un combat (ce que le
moteur sait dire, voir plus bas), une seule lecture renseigne tout le monde.

#### Ce que le log donne, et où il s'arrête (vérifié, pas supposé)

Dépouillement de `crates/overlay-engine/tests/wakfu.log` (10 975 lignes, 133 fins de tour) :

- **Aucun début de tour n'est jamais logué.** Ni dans `[Information (combat)]`, ni dans `[FIGHT]`,
  `[FIGHT_STATE]`, `[FIGHT_REFACTOR]` ou `[_FL_]`.
- Le seul marqueur de tour est une **fin** : « N secondes reportées pour le tour suivant. »
  (`TURN_ENDED_RE` → `LogEntry::TurnEnded`, §5 et `engine-js/src/log-parser.ts`). Elle est émise
  pour **chaque personnage du joueur**, toutes instances confondues (le log est partagé, §6.5),
  **même pour un tour passé sans agir** — observé à 20:35:00 sur un tour sans le moindre sort.
  Elle **ne porte aucun nom**.
- Un tour de monstre n'émet rien. Or les camps alternent : c'est presque toujours un monstre qui
  précède le personnage suivant du joueur.
- Aucune occurrence de « 0 seconde reportée » sur 133 fins : un tour mené jusqu'au bout du chrono
  n'émet probablement pas la ligne. À confirmer en conditions réelles.

Conséquence : la file d'initiative (`FightWorking::initiative_seats`) sait dire **qui** vient après
qui, mais seulement une fois construite par observation — elle est **aveugle au premier tour**, et
le premier tour est précisément le moment où un personnage qui joue en dernier doit être annoncé.
Le log seul ne peut donc pas tenir l'exigence.

#### Ce que le widget « Fin du tour » du jeu donne (analyse vidéo, 2026-09-14)

Vidéo fournie par l'utilisateur (90 s, 30 i/s, recadrée sur le widget), dépouillée image par image :

- **Deux états.** Au repos, le widget affiche le **nom du combattant dont c'est le tour**, le bouton
  « Fin du tour » et le chrono du tour. Au survol d'un combattant, il bascule en carte de stats
  (niveau, PV, PA/PM/PO, portrait) et le nom devient celui du **survolé** — état à écarter, et
  facile à écarter : le panneau droit passe d'un beige uniforme `rgb(209, 190, 133)` à un portrait
  sombre.
- **Le bouton ne distingue rien.** À t = 3,6 s (« Pugio Letalis ») et t = 6,0 s (« Oumbra »), tous
  deux au repos, le panneau doré est rigoureusement identique. Pas de variante grisée ou
  désactivée selon que le combattant actif est ou non le personnage de cette fenêtre. **Toute piste
  fondée sur l'apparence du bouton est close.**
- Le chrono est celui du tour en cours et **saute vers le haut** à chaque changement de tour
  (63 s → 105 s entre les deux images ci-dessus) : c'est un détecteur de frontière de tour
  indépendant du nom.
- La boussole de gauche indique l'**orientation** du personnage, pas le tour.
- La bande du nom est un cas favorable : **fond noir uni** (aucun décor derrière), texte blanc,
  serif gras, aligné à droite sur une ligne de base fixe.
- **Position à l'écran, établie par une seconde vidéo** (2026-09-14, plein écran non recadré, deux
  fenêtres côte à côte) : le widget est ancré au **coin bas-droit** de la fenêtre de jeu, jamais en
  haut. Deux widgets distincts partagent cet emplacement selon la phase : en phase de placement
  (« Tour 0 »), une carte simplifiée « Prêt / N s » qui égrène les personnages au fil de leurs
  confirmations (changements de nom toutes les 200-400 ms, à ne pas confondre avec un vrai
  changement de tour) ; le combat engagé (« Tour 1 » et suivants), le widget riche décrit
  ci-dessus.

**Le seul discriminant est donc le nom**, et il faut le lire.

#### Le rendu ne se fige pas hors focus — vérifié, plus seulement espéré

**La question qui conditionnait toute la chaîne visuelle** : Wakfu suspend-il son rendu et sa
logique de jeu quand sa fenêtre perd le focus — comme le font, par économie, beaucoup
d'applications Java ? Si oui, aucune API de capture, aussi bonne soit-elle, ne verrait plus qu'une
image figée. Ce n'était pas acquis, et c'était le principal risque du point dur ci-dessous.

Réponse mesurée sur la même vidéo (85,6 s, 5118 × 1438, deux fenêtres partageant le même combat) :
**non.** Deux segments, focus inversé entre les deux, chacun comparant le chrono de la fenêtre
active à celui de la fenêtre sans focus :

- 44,0 s → 49,6 s, focus à gauche : chrono à droite (sans focus) 54 → 53 → 52 → 51 → 50 → 49 s.
- 63,6 s → 68,8 s, focus inversé (à droite) : chrono à gauche (sans focus) 54 → 53 → 52 → 51 → 50 s.

Dans les deux sens, le chrono de la fenêtre sans focus décompte **exactement au même rythme** que
celui de la fenêtre active, à la seconde près. Le client ne suspend ni son rendu ni sa logique de
jeu en arrière-plan.

**Bonus qui simplifie le chemin critique** : dans les deux segments, c'est systématiquement la
fenêtre SANS focus qui affiche le bouton doré (état « repos »), jamais la carte de stats (état
« survol »). Logique : l'état survol dépend de la position de la souris, qui ne peut pas se
trouver dans une fenêtre qu'on ne regarde pas. **Le cas qui nous intéresse — lire une fenêtre en
arrière-plan — retombe donc quasi toujours dans l'état le plus simple à lire**, nom net sans
portrait ni stats en travers ; l'état survol reste un repli à gérer, pas le chemin principal.

Reste hors du périmètre de cette vidéo, donc encore à vérifier : cette capture est une capture
d'écran **complète** (ce que Windows compose à l'affichage), elle ne prouve pas qu'une capture
**programmatique** (Windows Graphics Capture, XComposite) obtient le même résultat — c'est
justement l'objet du spike ci-dessous. Les deux fenêtres sont restées visibles côte à côte tout du
long, jamais l'une entièrement masquée par l'autre ni par une troisième application : le cas D
(focus sur un navigateur, aucune fenêtre Wakfu visible) n'a pas été couvert.

#### Lire le nom sans embarquer d'OCR

La question n'est pas « quel texte ? » mais « **est-ce l'un de mes N personnages, et lequel ?** » —
N vaut 2 à 8, et les noms sont connus d'avance : le roster les donne, et les lignes `[_FL_]`
donnent en plus la liste fermée des combattants du combat en cours. Comparaison de gabarits, donc,
pas de reconnaissance de texte.

Et le gabarit **s'apprend à l'exécution** plutôt que de se deviner : quand la fenêtre du personnage
P est lisible et que le log vient de dire que P a lancé un sort, la bande affichée à cet instant
*est* le gabarit de P — dans la police, la taille et l'échelle d'interface exactes du client, sans
calibration à la charge de l'utilisateur. Mis en cache à côté de `config.toml`, indexé par
personnage et par taille de fenêtre.

#### Le point dur, à valider par un spike avant tout engagement

**Capturer une fenêtre de jeu qui n'a pas le focus** — et, cas D, qui peut être entièrement occultée
par une autre application. Wakfu est Java/JOGL (§6.4), le pire cas pour les API de capture. Le
risque que le CLIENT lui-même se fige en arrière-plan est levé (ci-dessus, vérifié par vidéo) ; ce
qui reste à établir est que l'API de capture, elle, obtient bien ce contenu :

- **Windows — ✅ tranché par le spike S4** (`spikes/s4-capture-hors-focus/`, 2026-09-14, deux
  passages sur la machine du mainteneur, deux clients en combat). `PrintWindow` +
  `PW_RENDERFULLCONTENT` **et** Windows Graphics Capture voient toutes deux un contenu **vivant**
  d'une fenêtre recouverte à 100 % — par l'autre client comme par une application tierce
  maximisée avec aucune fenêtre Wakfu au premier plan (le cas D) : chrono 96 → 95 → 94 → 93 → 92
  → 91 s, dans les deux fenêtres, par les deux méthodes. La réputation de `PrintWindow` sur OpenGL
  ne s'est pas vérifiée sur ce client. **Retenue pour la v1 : `PrintWindow`** — synchrone, à la
  demande, aucune session par fenêtre, aucun liseré, image de l'instant même ; et `GetDIBits`
  sait ne copier que les dernières lignes, là où vit le widget (2 Mo par lecture au lieu de 14).
  WGC reste le repli documenté si `PrintWindow` échouait sur une autre configuration graphique.
  Une fenêtre **minimisée** est hors d'atteinte des deux, proprement : `PrintWindow` renvoie faux,
  WGC ne reçoit plus d'image — et `IsIconic` le dit avant d'essayer. Cas non couvert, annoncé
  tel quel.
- **Linux / X11 — reste à valider** : `XCompositeRedirectWindow` + `XCompositeNameWindowPixmap`.
  Le compositeur actif est déjà une **condition d'exploitation** de l'overlay (§6.4, transparence
  par visuel ARGB), la dépendance n'est donc pas nouvelle. `XGetImage` sur une fenêtre occluse ne
  convient pas. À faire sur une machine Linux (SteamOS chez le mainteneur), même protocole que S4.

La chaîne visuelle peut donc s'engager **sous Windows**. Sous Linux, tant que le pendant de S4
n'a pas conclu, le repli reste la file d'initiative seule, avec le premier tour assumé comme angle
mort — et l'utilisateur prévenu que l'exigence n'y est pas tenue.

#### Ce que le moteur sait déjà, et qui sert tel quel

- `SessionSnapshot::fight_for_character` rattache un combat à un personnage : c'est lui qui dit si
  deux fenêtres partagent un combat (cas B) ou non (cas C), sans rien ajouter.
- `RosterIndex::find` dit si un nom est un personnage du joueur.
- `GameWindowTracker::scan` donne toutes les fenêtres de jeu avec leur personnage (titre
  `"<Nom> - WAKFU"`, §6.5) et leur rectangle.
- `GetForegroundWindow` / `_NET_ACTIVE_WINDOW` (déjà lus par `sync_topmost` et
  `send_partner_command`) donnent la fenêtre au premier plan — y compris « aucune fenêtre Wakfu »,
  qui est le cas A/D et non une erreur.

#### Décidé

- **Canal : notification du système**, pas un toast dans l'overlay — un toast s'afficherait sur une
  fenêtre que l'utilisateur ne regarde pas (décision de l'utilisateur, 2026-09-14).
- **Emplacement du réglage** : section « Combat » de l'onglet Paramètres (décision de
  l'utilisateur, 2026-09-14). Cette section **absorbe l'ancienne « Affichage »** le même jour, sur
  décision de l'utilisateur : celle-ci ne portait qu'une case, « Afficher le panneau de combat en
  dehors des combats », qui parlait déjà du panneau de COMBAT — deux sections voisines sur le même
  sujet en auraient fait une de trop. Ordre des deux cases : l'affichage permanent d'abord, la
  notification ensuite, du plus passif au plus intrusif.
- **Réglage local**, jamais au compte : une notification système dépend de la machine (démon de
  notifications, écrans, appairage téléphone), pas du joueur.
- **Une notification par tour et par personnage** au maximum ; rien quand la fenêtre concernée est
  déjà au premier plan.

#### Ouvert (voir aussi §14)

- **Windows, identité de l'émetteur** : sans `AppUserModelID` enregistré (un raccourci dans le menu
  Démarrer), le toast s'affiche au nom de PowerShell. L'overlay n'a pas d'installeur (§11).
  **Reporté explicitement par l'utilisateur le 2026-09-14** — à trancher avec l'installeur.
- **Faisabilité de la capture hors focus** sur les deux plateformes : spike préalable, ci-dessus.
- **Position du widget dans la fenêtre** : inconnue (la vidéo est recadrée). Détection par recherche
  du panneau beige uniforme, ou relevé sur une capture plein écran — en attente.

### 9.1 undecies Fenêtre de connexion, compte obligatoire, icône de zone de notification (2026-09-14)

Demande utilisateur : « une fenêtre de connexion au démarrage quand aucun jeton n'est détecté »,
première interface de l'overlay, et **plus aucun mode invité, jamais** — un compte Wakfu Companion
est obligatoire. Maquette validée en quatre passes (direction « Fidèle au web », artefact de
session), portée telle quelle dans `panels::login`.

**Ce qui existe maintenant :**

- **Écran de chargement d'abord** (demande utilisateur, même jour) : la toute première image de
  l'overlay est cette carte avec le rouage du jeu (`design::loader`, 72 px) centré dans le corps —
  logo, titre, séparateur, version, rien d'autre. Elle masque tout le démarrage : catalogue,
  référentiel de donjons, rattrapage de `wakfu.log` par le moteur (fin détectée au premier
  silence de 200 ms du watcher), vérification du jeton stocké et `GET /api/v1/settings`. Les
  trois premiers posent chacun un drapeau sur `overlay_ui::startup::StartupProgress` (partagé
  avec les threads, `UserEvent::StartupProgress` réveille l'hôte) ; le compte est
  `AuthStatus::Connecting`. Garde-fou de 45 s, puis l'écran suivant quoi qu'il arrive. Compte lié
  ⇒ la fenêtre cède la place aux overlays ; sinon ⇒ « Vous n'êtes pas connecté ». Même hauteur
  que cet écran-là : le passage ne fait pas bouger la fenêtre.
- **Plancher d'affichage de 5 s** (`startup::MIN_DISPLAY`, demande utilisateur du 2026-09-15) :
  tout en cache et un jeton qui répond du premier coup, et l'écran ne durait qu'une poignée
  d'images — un clignotement au lancement, pas un écran de chargement. `is_complete` reste donc
  faux tant que l'écran n'a pas tenu cinq secondes, avant même le garde-fou ; l'hôte, qui ne
  connaît que lui, garde la fenêtre de connexion sans rien savoir de ce délai. Le plancher court
  depuis l'**apparition** de l'écran et repart quand `set_update_blocking(true)` le rouvre sur une
  session déjà ouverte (« Mettre à jour » des Options), sinon une mise à jour qui échoue d'emblée
  — dossier d'installation non inscriptible, manifeste injoignable — rendrait la main aussi vite
  qu'elle l'a prise.
- **`OverlayKind::Login` / `panels::login`** — une carte de 400 px sur fond noir quasi opaque
  (`rgba(8,10,14,.90)` ; le `.78` du web laissait voir le bureau, demande du 2026-09-16) :
  logo du site (`assets/ui/logo-purple.png`, copie de `public/logo-purple.png` du dépôt web),
  titre « WAKFU COMPANION » en accent cyan `#00d2ff` suivi d'« OVERLAY » en italique gris, badge
  *beta* en haut à droite, séparateur gravé, corps aligné à gauche, version en pied en bas à
  droite, au style exact du badge *beta* (Ubuntu 10 px, italique simulé, même gris — demande
  utilisateur du même jour, elle était d'abord à gauche en PT Serif). Un **anneau lumineux tourne en permanence** autour de la
  carte (`Mesh` à couleurs par sommet, ~30 images/s) : gris translucide au repos, cyan pendant
  l'appairage, rouge en erreur. Trois états calqués sur `AuthStatus` — *non connecté* (« Se
  connecter »), *appairage* (code en grand, compte à rebours « expire dans mm:ss », « Copier le
  code », « Rouvrir la page », lien « Annuler l'appairage »), *erreur* (« Connexion impossible »,
  titre court de l'échec, détail technique, « Réessayer ») ; `Connecting` réutilise l'écran
  d'accueil avec « Connexion… » à la place du bouton. **Palette du site, pas du jeu** : c'est la
  porte d'entrée du compte, pas un overlay — seule exception assumée aux textures 9-slice du design
  system (§9.2). Italique simulé par cisaillement des sommets (aucune fonte italique embarquée),
  interlettrage peint glyphe par glyphe (egui n'en a pas).
- **Une fenêtre logicielle classique, pas un overlay** (`main.rs::App::create_login_window`) :
  barre des tâches et bascule de fenêtres, focalisable, z-order normal (jamais `HWND_TOPMOST`,
  exclue de `sync_topmost`), centrée sur l'écran principal (`center_on_primary_monitor`, repli
  premier écran) « comme Discord au lancement », sans décorations OS — elle se déplace par sa
  bannière (`LoginOutcome::drag_window` → `Window::drag_window`). Retaillée à chaque changement
  d'état à la hauteur que la carte a réellement occupée (`LoginOutcome::content_height`), en
  gardant son centre. Le logo est son icône de fenêtre et de barre des tâches. Échap n'y quitte
  pas l'application ; Alt+F4, la croix de barre des tâches et le menu de zone de notification, si.
- **Cycle de vie piloté par le démarrage et `AuthStatus`** (`App::sync_session_windows`, avant
  `sync_windows` à chaque tick) : chargement en cours ⇒ la fenêtre de connexion seule, sur son
  rouage ; compte non lié ⇒ la fenêtre de connexion est la SEULE fenêtre (tout `Combat`/
  `Watchlist`/`Options` est fermé, raccourcis rendus au système si une modale Options tombait) ;
  compte lié et tout chargé (`App::session_ready`) ⇒ elle disparaît et `sync_windows` crée enfin
  les overlays de jeu. « Déconnecter » (fenêtre Options, section « Compte », ou menu de zone de
  notification) efface le jeton, ferme tous les overlays et ramène à cette fenêtre. Au
  relancement, elle reste sur son rouage tant que le jeton n'est pas validé.
- **Threads de fond partagés** (`overlay_ui::background`, 2026-09-14) : Auth, Sync, Catalogue et
  Donjons vivaient dans `main.rs` ; rien n'y dépend de l'OS, ils sont sortis dans la lib pour
  servir aussi le binaire Linux.
- **Thread Auth sans appairage spontané** (`background::spawn_auth_thread`/`attempt_connect`) : au démarrage,
  seul un jeton stocké est essayé ; sans jeton, l'état neutre `Disconnected { failure: None }` est
  publié et le navigateur ne s'ouvre que sur « Se connecter » (`AuthCommand::Retry`). Un jeton
  **refusé** (401/403) est effacé ; un jeton **injoignable** (réseau) est conservé — un lancement
  hors ligne n'efface plus une session valide (c'était le cas jusqu'ici, à tort). Les échecs
  portent un `AuthFailure { headline, detail }` pour l'écran rouge. `pair_and_wait` accepte une
  fermeture d'annulation consultée toutes les 250 ms (`AuthCommand::CancelPairing`, ou
  `Disconnect` pendant un appairage) et expose `expires_at` pour le compte à rebours.
- **Icône de zone de notification** (`tray-icon` 0.25, dépendance Windows seulement — ses
  fonctionnalités par défaut tirent GTK sous Linux) posée au premier `resumed`
  (`App::install_tray`) : menu **Options / Déconnecter / Quitter** validé par l'utilisateur,
  « Options » et « Déconnecter » grisés tant qu'aucun compte n'est lié (`sync_tray_menu`), clics
  sondés dans `about_to_wait` comme les raccourcis. « Déconnecter » agit sans confirmation (un menu
  contextuel n'en ouvre pas). **« Mise à jour » ajoutée après « Options » (2026-09-15, demande de
  l'utilisateur)** : lance la recherche de mise à jour (`UpdateCommand::Check`, même commande que
  le bouton de la fenêtre Options — §8.3 de `docs/plan-mise-a-jour.md`), toujours active puisqu'une
  recherche ne dépend pas du compte ; le résultat se lit dans la section « Mise à jour » de la
  fenêtre Options. C'est le seul accès à l'overlay quand ni jeu ni fenêtre de connexion
  ne sont à l'écran, et le seul moyen de quitter proprement une fois connecté.
- **L'ancienne carte d'appairage de la zone Combat** (code, icône de relance 🔌, « Connexion… »)
  est retirée : la zone Combat n'existe plus que compte lié.
- **Captures** (`tests/panels.rs`, `login_*.png`, un harnais par test — `egui_kittest` exige
  qu'un test n'en ait qu'un) : les quatre états, animation figée (`LoginState::animate = false`),
  chacune prise à la hauteur exacte que `main.rs` donne à la fenêtre OS, vérifiée par assertion.

**Linux (`bin/overlay-ui-x11.rs`, porté le même jour) :** mêmes threads de fond
(`overlay_ui::background`), même fenêtre de connexion (fenêtre X11 ordinaire, pas `Utility` —
barre des tâches, focus, centrée sur l'écran principal, icône = logo), même cycle de vie, fenêtre
Options désormais pleinement câblée (compte, alertes, chat, suivi, recettes) et icônes réseau
(`RemoteIconStore::spawn`). Le binaire n'est plus « invité fixe ». Reste propre à Windows :
l'icône de zone de notification (`tray-icon` tire GTK/libappindicator sous Linux, et GNOME exige
une extension) — « Quitter » y passe par le raccourci global ou la fermeture de la fenêtre de
connexion, « Déconnecter » par la fenêtre Options.

**Captures** : `login_chargement`, `login_non_connecte`, `login_appairage`, `login_erreur`.

**Non fait, volontairement :** pas de bouton de fermeture dans la carte (fidèle à la maquette) ;
pas de mémorisation de la position de la fenêtre ; « Réessayer » et « Se connecter » sont la même
commande (`Retry` reprend le jeton stocké s'il existe, sinon appaire).

### 9.1 duodecies Interrupteurs de fonctionnalité : Suivi, Alertes, Recherche (2026-09-15)

Demande utilisateur : « permettre de désactiver les features Suivi, Alertes, Chat, via une option
tout en haut, après le titre — "Activer le suivi", "Activer la surveillance du drop", "Activer la recherche" ;
activée par défaut ; lorsqu'elle est désactivée, tout le contenu devient grisé et désactivé,
impossible d'interagir avec ».

**Ce qui existe maintenant :**

- **Une case en tête de chaque onglet**, sous le titre et sa phrase de description, avant le
  premier réglage — `panels::feature_switch`, partagé par les trois onglets pour que leur place,
  leur aération et leur façon de griser ne divergent pas.
- **Griser, c'est `egui::Ui::disable`** appelé sur le `Ui` de l'onglet juste après la case : il
  retire l'interaction et multiplie l'opacité du `Painter`, dont héritent tous les enfants — y
  compris les tuiles peintes à la main et le contenu des zones de défilement. Le titre, la phrase
  et la case restent vifs : c'est par eux qu'on rallume.
- **Un brouillon comme le reste de la fenêtre** (§5.1) : la bascule n'a d'effet qu'à « Valider »,
  la garde de fermeture s'ouvre tant qu'elle n'est pas validée.
- **Persistées en LOCAL** (`config::OverlayConfig::{suivi,alerts,chat}_enabled`, `features()` /
  `set_features()`) et non au compte, comme `combat_always_visible` : ce qu'on accepte de voir
  par-dessus son jeu dépend de la machine. `#[serde(default = "actif")]` et non le
  `#[serde(default)]` de leurs voisines — un `bool` non renseigné vaudrait `false`, ce qui
  couperait les trois fonctionnalités chez tous ceux qui les utilisent déjà.

**Ce qu'une fonctionnalité coupée cesse de faire, et ce qu'elle continue de faire :**

| Coupée | Ce qui s'arrête | Ce qui continue |
| --- | --- | --- |
| Suivi | Le bandeau in-game n'affiche plus aucune tuile, ni les boutons « + » et « − » (2026-09-15, voir ci-dessous) ; l'alerte de décompte à zéro ne sonne plus | Le moteur compte, suit et synchronise au compte |
| Alertes | Le ramassage d'un objet à son activé ne joue plus rien et n'affiche plus de carte | La liste d'objets reste au compte |
| Recherche | Aucun message du chat ne fait plus sonner l'overlay ni n'affiche de carte | Les recherches restent au compte |

**Bandeau du Suivi coupé : deux boutons, pas quatre (2026-09-15)** — retour utilisateur : « lorsque
le suivi est désactivé, il faut retirer les boutons "+" et "-" du bandeau ». Le carré de contrôle
restait entier alors que « + » n'a plus rien à ajouter et que « − » n'était que grisé faute de
liste. Ces deux boutons ne sont désormais plus peints du tout, la rangée se referme sur
« Détails » et « Options », et la fenêtre Suivi rétrécit d'autant
(`panels::watchlist::ControlLayout::RowTrackingOff` / `content_width`). « Options » reste, lui :
c'est le seul accès à la fenêtre de réglages depuis le jeu, donc le seul moyen de rallumer le
Suivi. L'état de la case descend jusqu'au panneau par `render_content::RenderContent::
watchlist_enabled` — une liste vide ne suffit pas à le déduire, un bandeau vide Suivi ACTIF garde
ses quatre boutons. Capture : `watchlist_suivi_coupe_rangee`.

Le thread Engine reçoit les trois drapeaux par `EngineCommand::SetFeatures` (au démarrage depuis la
config, puis à chaque validation) et **continue de drainer** les alertes qu'il ne joue pas : les
laisser s'accumuler les ferait toutes sortir d'un coup à la réactivation, des heures après le
ramassage qui les a produites. Recocher une case retrouve donc la liste et les compteurs tels
quels, sans rattrapage ni relecture du log ; en échange, ce qui est survenu pendant la coupure est
perdu — ce qui est exactement ce qu'on demande en coupant.

**Captures** : `options_suivi_desactive`, `options_alertes_desactive`, `options_chat_desactive`, et
le test `options_suivi_coupe_la_grille_ne_repond_plus` pour la moitié « impossible d'interagir »,
qu'aucune image ne peut montrer.

**Effet de bord corrigé au passage** : `panels::tile_reorder` posait la croix fléchée sur toute
tuile survolée, `Response::contains_pointer()` ne disant que la géométrie — une tuile d'un onglet
coupé annonçait donc encore un déplacement qu'aucun glissement n'aurait exécuté. Le curseur est
désormais conditionné à `Response::enabled()`.

### 9.1 terdecies Couper le son d'une alerte : Suivi et Chat (2026-09-15)

Demande utilisateur : « ajouter une option dans Suivi et Chat permettant de couper le son des
notifications, juste en dessous de la ligne "Tester le son de l'alerte" ».

**Ce qui existe maintenant :**

- **Une case « Couper le son des notifications »** sous la ligne d'essai des onglets « Suivi » et
  « Chat ». Cochée, l'alerte **ne joue plus aucun son et affiche toujours sa carte** par-dessus le
  jeu : c'est le demi-pas qui manquait entre « tout actif » et l'interrupteur de fonctionnalité
  (§9.1 duodecies), qui, lui, coupe les deux canaux.
- **Le bouton d'essai est grisé quand le son est coupé** — proposer d'écouter ce qu'on vient de
  faire taire serait une promesse que le jeu ne tiendra pas. Même règle que le champ de durée grisé
  sous une « Fermeture automatique » décochée.
- **La ligne d'essai est devenue un composant** (`panels::notifications`, `panels::sound_row`
  jusqu'au 2026-09-15) : elle était écrite trois fois à l'identique dans les onglets Suivi, Alertes
  et Chat, et c'est elle qui porte désormais la case. Même motif que `panels::feature_switch`.
- **Pas de case dans « Alertes »**, et ce n'est pas un oubli : le son d'un ramassage s'y coupe déjà
  objet par objet, à la tuile — une sourdine globale ferait double emploi avec un réglage plus fin.
- **Un brouillon comme le reste de la fenêtre** (§5.1) : la bascule n'a d'effet qu'à « Valider », et
  la garde de fermeture s'ouvre tant qu'elle n'est pas validée.
- **Persistées en LOCAL** (`config::OverlayConfig::{suivi,chat}_alert_muted`, `alert_mutes()` /
  `set_alert_mutes()`), comme les interrupteurs et pour la même raison — un son bienvenu au casque
  ne l'est pas forcément sur la machine du salon. `#[serde(default)]` suffit ici, contrairement aux
  interrupteurs : le défaut d'une sourdine est d'être LEVÉE, c'est-à-dire `false`.

Le thread Engine reçoit les deux sourdines par `EngineCommand::SetAlertMutes` (au démarrage depuis
la config, puis à chaque validation) et se contente de ne pas appeler `alert_sound::play_*` : le
toast, lui, est publié comme avant. C'est toute la différence avec `SetFeatures`, qui saute
l'alerte entière.

**Captures** : `options_parametres_son_coupe` — les deux cases cochées, les deux boutons d'essai
grisés, et le reste des sections resté vif. (Deux planches d'onglet, `options_suivi_son_coupe` et
`options_chat_son_coupe`, jusqu'au regroupement du §9.1 quaterdecies.)

### 9.1 quaterdecies Les notifications se règlent dans « Paramètres » (2026-09-15)

Demande utilisateur, en quatre points : uniformiser « Tester le son de l'alerte » en **« Tester le
son des notifications »** et « Fermeture automatique » en **« Fermeture automatique des
notifications »**, sortir le couple case + durée de sa ligne à fond arrondi, et **déplacer les
trois blocs (essai du son, sourdine, fermeture automatique) de tous les onglets dans des sections
dédiées, après la section « Combat » de l'onglet « Paramètres »**.

**Ce qui existe maintenant :**

- **Quatre sections par fonctionnalité** dans « Paramètres » : « Combat » (inchangée, §9.1 decies),
  puis « Suivi », « Alertes » et « Chat », dans l'ordre du menu d'onglets. Chacune porte ce que sa
  fonctionnalité fait entendre et voir — et rien n'a été uniformisé de force : les Alertes n'ont
  pas de sourdine globale (le son d'un ramassage se coupe déjà objet par objet, à la tuile, §9.1
  terdecies). Le Suivi n'avait pas de fermeture automatique à cette date, au motif que « son alerte
  est un son et un bandeau permanent, pas une carte à fermer » — c'était faux, et §9.1 sexdecies
  le corrige.
- **`panels::notifications`** (l'ancien `panels::sound_row`) peint ces sections. Il porte aussi le
  trait `ToastClose`, que `AlertProfile` et `chat_tab::ChatToastSettings` implémentent : le bornage
  de la durée reste chez eux, le peintre n'en refait pas un à lui.
- **Les trois onglets ne gardent que ce qu'ils listent** — objets suivis, objets à alerte,
  recherches. `AlertsTabAction` et `ChatTabAction` ont disparu avec le bouton d'essai, seule
  intention que ces écrans produisaient ; `SuiviTabAction` ne garde que `ResolveRecipe`.
- **Une fonctionnalité éteinte grise sa section** (`panels::feature_switch`) : ni son à essayer, ni
  carte à fermer quand rien ne se déclenche. Un brouillon d'alertes ou de chat pas encore descendu
  du compte grise sa seule ligne de fermeture, plutôt que de la faire apparaître en cours de route.
- **Plus de fond de ligne sous la fermeture automatique** : le pavé arrondi `#26282b` venait de la
  maquette d'« Alertes », où il tranchait sur le reste de l'onglet ; dans une section de
  « Paramètres » il faisait de la durée le seul réglage encadré de la fenêtre.
- **L'onglet « Paramètres » défile** (`design::PanelZones::scroll_area`, zone
  « options-parametres ») : il porte sept sections, et « Mise à jour » tombait hors de la fenêtre
  sans que rien ne le dise. Agrandir la fenêtre n'était pas une option — elle est posée par-dessus
  un jeu. Corrigé au passage : la zone défilable rognait le retrait des titres de section, qui
  rendaient « ichier », « ombat », « uivi ».

**Captures** : `options_parametres_infobulle_test_son`, `options_parametres_son_coupe`,
`options_parametres_fermeture_manuelle`, et `options_parametres_compte` /
`options_parametres_mise_a_jour` après défilement.

### 9.1 tredecies Mise à jour automatique (2026-09-15)

Plan, décisions et détail dans [`plan-mise-a-jour.md`](plan-mise-a-jour.md) (§7.1 pour ce qui est
construit). Ce qui change dans l'overlay : une quatrième étape de démarrage (« vérification de
mise à jour », `startup.rs`), un thread de fond de plus (`background::spawn_update_thread`,
`overlay_sync::update`), l'écran de chargement qui montre sous son rouage le téléchargement
(ligne d'état, jauge, compteur) ou « Mise à jour requise » quand une version minimale n'a pas pu
s'installer, et une section « Mise à jour » dans Options › Paramètres (ligne d'information, case
« Installer automatiquement… », bouton unique « Recherche de mise à jour » / « Mettre à jour vers
X » / « Réessayer », confirmation avant installation). L'installation remplace l'exe en cours
d'exécution (`self-replace`) et relance avec `--updated-from`, toujours derrière l'écran de
chargement, jamais pendant une session. Captures : `login_telechargement`,
`login_version_disponible`, `login_mise_a_jour_requise`, `options_parametres_mise_a_jour`.

**À retravailler** (retour du mainteneur) : la mise en forme de l'écran de chargement, dans une
itération dédiée — le mécanisme est en place, pas son dessin.

### 9.1 quaterdecies Interrupteurs du panneau Combat : détail des combats, suivi des sorts (2026-09-15)

Demande utilisateur : « ajouter une option d'activation de l'overlay combat dans la section
"Combat" de l'onglet "Paramètres" avec le libellé "Activer le détail des combats", active par
défaut », et « ajouter une option d'activation de l'aperçu des sorts […] avec le libellé "Activer
le suivi des sorts", active par défaut ; cette option est dépendante de l'option "Activer le détail
des combats" ».

**Ce qui existe maintenant :**

- **Deux cases en tête de la section « Combat »**, avant les réglages qu'elles commandent — même
  place qu'un interrupteur d'onglet (§9.1 duodecies). « Activer le suivi des sorts » est en
  retrait sous « Activer le détail des combats », à l'aplomb de son libellé : la même géométrie
  que la sourdine sous la notification de tour (§9.1 decies).
- **Le même véhicule que les trois cases d'onglet** :
  `panels::feature_switch::FeatureToggles::{combat, spells}`, brouillon jusqu'à « Valider »,
  persisté en LOCAL (`config::OverlayConfig::{combat,spells}_enabled`, `#[serde(default =
  "actif")]`). Pas d'appel à `feature_switch::show` en revanche : il n'y a pas d'onglet « Combat »
  à griser, seulement deux cases ordinaires.
- **Une dépendance qui grise sans écraser** : le détail des combats coupé grise « Activer le suivi
  des sorts » ET « Afficher le panneau de combat en dehors des combats » — un réglage d'
  encombrement ne peut pas rallumer un panneau que son interrupteur éteint. Les deux cases gardent
  leur valeur : `FeatureToggles::spells_visible()` combine les deux au moment de peindre, et qui
  rallume l'interrupteur retrouve ses réglages tels quels.

| Coupée | Ce qui s'arrête | Ce qui continue |
| --- | --- | --- |
| Détail des combats | Aucune fenêtre Combat n'est montrée, combat en cours compris (`panels::combat::should_show`, appliquée par les deux hôtes) | Le moteur mesure les combats et synchronise l'historique au compte |
| Suivi des sorts | Le bloc « ligne de sorts », les deux marques sur les médaillons et l'épinglage au clic (`render_content::RenderContent::spells_enabled` → `panels::combat::show`) | Portraits, barres, switches et infobulles du panneau |

Ces deux drapeaux **ne concernent pas le thread Engine** : ils voyagent dans le même
`EngineCommand::SetFeatures` que les trois autres, mais rien ne les y lit — le détail des combats
est une affaire de fenêtre OS (`sync_combat_visibility`, appelée dès la validation pour que le
geste et son effet soient dans la même passe), le suivi des sorts une affaire de rendu.

**Captures** : `options_parametres_combat_coupe` (les deux cases grisées, toujours cochées),
`combat_spell_block_coupe` (le panneau sans sa ligne de sorts ni ses marques, à comparer à
`combat_spell_block_suivi_auto`), et le test
`options_parametres_la_case_des_sorts_suit_le_detail_des_combats` pour la moitié « la case grisée
ne répond plus », qu'aucune image ne montre.

### 9.1 quindecies La suppression multiple gagne « Alertes » et « Chat » (2026-09-16)

Demande utilisateur : « ajouter le système de la suppression multiple, comme dans l'onglet "Suivi",
dans les onglets "Alertes" et "Chat" ».

Le geste existait depuis le 2026-09-13, mais **au Suivi seulement** : un bouton corbeille qui ouvre
un mode, une case à cocher sur chaque tuile, un bouton rouge dont le libellé dit ce qu'il retire.
Les deux autres onglets composent pourtant la même sorte de liste, et n'offraient qu'un retrait
tuile par tuile — vider une liste de quinze recherches demandait quinze survols.

**Ce qui existe maintenant :**

- **Un module partagé**, `panels::bulk_select`, où la mécanique vit une seule fois : l'en-tête
  (titre à gauche, corbeille et bouton groupé ancrés à droite), la bascule du mode, l'oubli des
  coches en quittant, et le libellé `bulk_label` que le bandeau in-game partageait déjà avec le
  Suivi. Il ne touche à aucune liste : il rend une intention (`BulkRequest::{All, Keys}`) et
  l'appelant, seul à savoir ce qu'est une entrée, l'applique à la sienne. L'onglet Suivi y a migré
  du même coup — trois copies du même en-tête auraient divergé au premier ajustement.
- **Les quatre règles du Suivi tenues partout** : la sélection est un MODE et non une case
  permanente ; sélection vide = « Supprimer tout » (la règle du web : aucune coche se lit « aucune
  exclusion ») ; aucune commande quand il n'y a rien à retirer ; quitter le mode oublie les coches.
  Et le ton reste **destructif** (`SelectionTone::Danger`) : cocher ici ne mène qu'au retrait, l'or
  promettrait un choix qui n'existe pas.
- **Ce que chaque onglet ajoute, et lui seul :**

| Onglet | Ce qui lui est propre |
| --- | --- |
| Suivi | inchangé — le glisser-déposer de réordonnancement reste coupé dans le mode |
| Alertes | **les dix objets par défaut ne se cochent pas** : ils ne se retirent pas (`SoundItemEntry::is_default`, refus structurel du web), donc ni case, ni clic — leur infobulle le dit plutôt que de laisser le clic ne rien faire. Le bouton disparaît quand la liste n'a plus qu'eux, et « Supprimer tout » les conserve. La phrase sous le titre change avec le mode : le clic ne bascule plus le son, il coche |
| Chat | toutes les recherches se retirent ; la case se pose au coin **haut-droit** de la tuile — le haut-gauche porte la légende du canal — c'est-à-dire au coin de la croix qu'elle remplace |

- **Un composant enrichi** : `design::legend_tile` sait porter une sélection
  (`LegendTile::selection`/`selection_tone`), comme `design::item_slot` depuis le 2026-09-13. Case
  ET liseré appartiennent au composant, jamais au panneau — la case n'est pas un widget et ne prend
  aucun geste, c'est le clic de la tuile qui coche. Le liseré n'est pas un trait ajouté par-dessus :
  c'est **la bordure du cadre repeinte** au ton de la sélection, sans quoi un rectangle plein
  traverserait la légende, qui interrompt justement la bordure haute.

**Captures** : `options_alertes_selection` (les trois objets du joueur cochables, dont deux cochés,
et les dix objets par défaut sans case), `options_chat_selection` (case au coin haut-droit, croix
disparue, bordures rouges), et la rangée « Sélection multiple » de
`design_gallery_legend_tile`. `options_suivi_selection` reste inchangée : la migration vers le
module partagé ne devait rien déplacer, et c'est cette image qui le prouve.

### 9.1 sexdecies Fermeture automatique des notifications de décompte (2026-09-16)

Demande utilisateur : « ajouter une option de gestion du temps d'affichage des notifications de
décompte dans la section "Suivi" de l'onglet "Paramètres", à l'image de celles des sections
"Alertes" et "Chat" […] le libellé changera légèrement pour devenir "Fermeture automatique des
notifications de décompte" ».

**Ce que §9.1 quaterdecies avait manqué** : la section « Suivi » était la seule sans ligne de
fermeture, parce que son alerte passait pour « un son et un bandeau permanent, pas une carte à
fermer ». Le décompte arrivé à zéro affiche pourtant bien une carte par-dessus le jeu
(`panels::watchlist::WatchlistToastReason::Countdown`) — elle empruntait simplement la durée du
**profil d'alertes de ramassage** descendu du compte (`AlertProfile`), et n'était donc réglable
que depuis la section d'à côté, pour les deux alertes à la fois.

**Ce qui existe maintenant :**

- **Une ligne de plus dans la section « Suivi »**, sous sa sourdine : case, champ de durée, unité
  « sec. » — la ligne des deux autres sections, aux mêmes règles (case décochée = fermeture
  manuelle, champ grisé ; durée bornée à 0,5–30 s à la PERTE DE FOCUS, jamais à la frappe ;
  virgule décimale acceptée). Seul le libellé change :
  `notifications::COUNTDOWN_AUTO_CLOSE_LABEL`, parce que le Suivi est la seule fonctionnalité dont
  deux cartes de nature différente peuvent s'afficher (son décompte ici, un ramassage réglé dans
  « Alertes »).
- **Un réglage propre, `suivi_tab::CountdownToastSettings`** — le pendant de
  `chat_tab::ChatToastSettings`, qui implémente le même trait `notifications::ToastClose` et borne
  sa durée lui-même. Brouillon jusqu'à « Valider » (`OptionsModalState::countdown_toast`,
  `OptionsCommit::countdown_toast`), **persisté en LOCAL**
  (`config::OverlayConfig::{countdown_alert_duration_seconds, countdown_alert_manual_close}`,
  `countdown_toast()` / `set_countdown_toast()`) : ce réglage n'a pas d'équivalent web et le
  serveur n'accepte que des clés connues, même exception que la carte de chat.
- **Sa ligne n'est jamais grisée par une attente** (`AutoClose::available: true`) : un réglage
  local n'a aucun brouillon de compte à attendre, contrairement à ceux des Alertes et du Chat.
- **Le thread Engine le reçoit par `EngineCommand::SetCountdownToast`** (au démarrage depuis la
  config, puis à chaque validation) et s'en sert pour poser le `hide_at` de la carte au moment où
  l'alerte naît. `chat_toast_deadline` devient `local_toast_deadline`, générique sur `ToastClose` :
  les deux réglages locaux calculent leur échéance par la même fonction, `toast_deadline` restant
  celle du profil de compte.

**Défaut commun : fermeture automatique cochée, 5 s** (demande du même jour : « toutes les options
de gestion de fermeture doivent être actives par défaut et les valeurs numériques à 5 par
défaut »). La case l'était déjà — `manual_close: false` dans les trois `Default` — mais la durée
reprenait le `DEFAULT_ALERT_DURATION_SECONDS` du web, 3,5 s. La constante passe à **5,0** dans
`overlay-engine::profile` et les trois lignes en héritent (profil de compte sans
`alertDurationSeconds`, `ChatToastSettings::default`, `CountdownToastSettings::default`). Le web
garde ses 3,5 s ; un compte ou une config qui porte déjà une durée n'est pas touché. Les captures
de « Paramètres », qui posent leur durée explicitement, ne bougent pas.

**Un bug de rendu trouvé en chemin, et corrigé** (`design::components::input`) : la zone d'édition
d'un champ prenait l'identifiant AUTOMATIQUE de son `Ui` parent. Un widget que le défilement sort
du champ visible ne consomme pas les mêmes identifiants, et ceux de tous les champs suivants se
décalent d'une frame à l'autre — un champ héritait alors de l'état mémorisé d'un autre, dont son
défilement horizontal (`TextEditState::text_offset`), et **se peignait vide alors que sa valeur
était bien là**. Le nouveau champ du Suivi l'a révélé (fenêtre défilée au point de sortir le champ
« Fichier », long et focalisé). La zone d'édition porte désormais un `id_salt` NOMMÉ, dérivé du
`log_name` du champ, et les rangées de `panels::notifications` nomment la leur de la même façon.

**Captures** : `options_parametres_son_coupe` (la ligne vive, durée à 3,5 s),
`options_parametres_fermeture_manuelle` (les TROIS champs grisés, valeur conservée),
`options_parametres_compte` et `options_parametres_mise_a_jour` (la même ligne, après défilement —
les deux planches qui montraient le champ vide).

### 9.1 septdecies Le bouton d'essai revient, sur la ligne de sourdine (2026-09-16)

Demande utilisateur : « ajouter le bouton de test du son des notifications sur la ligne de coupure
du son des notifications. Pour la section "Alertes", ajouter une ligne "Tester le son des
notifications" avec le bouton pour écouter le son » — et, dans l'onglet « Alertes », « déplacer
l'icône mute en bas à droite des item_slot ».

**Contexte** : la ligne « Tester le son des notifications » que chaque section de « Paramètres »
ouvrait (§9.1 quaterdecies) avait été retirée le matin même, à la demande de l'utilisateur, avec
les actions `Test*Sound` de la fenêtre. Elle revient sous une autre forme, plus courte d'une ligne
par section.

**Ce qui existe maintenant :**

- **Le haut-parleur d'essai à droite de la case « Couper le son des notifications »**, sur la
  même ligne — sections « Combat » (son de tour), « Suivi » (décompte) et « Chat » (recherche).
  C'est le même son qu'on coupe et qu'on essaie, il n'occupe plus deux lignes. **Grisé quand le
  son ne viendrait pas** (sourdine cochée, ou notification de tour décochée pour Combat), et
  l'infobulle le dit — la règle de §9.1 quaterdecies, inchangée.
- **Les Alertes gardent une ligne à elles**, « Tester le son des notifications »
  (`notifications::TEST_LABEL`) : elles n'ont pas de sourdine globale (§9.1 terdecies), le bouton
  n'a aucune ligne où se poser. Il y est toujours vif.
- **`notifications::section` renvoie de nouveau le clic**, et `notifications::test_sound_button`
  est public pour la section « Combat », qui peint sa sourdine de tour elle-même
  (`panels::options_modal`). Sa ligne prend la hauteur des lignes de section
  (`notifications::ROW_HEIGHT`, 39 px) : dans un simple `horizontal`, la case restait calée en
  haut d'un bouton plus grand qu'elle. Les hôtes (`main.rs`, `overlay-ui-x11.rs`) rejouent les
  quatre actions `TestAlertSound` / `TestChatSound` / `TestCountdownSound` / `TestTurnSound` par
  `alert_sound`, le chemin exact du jeu.
- **Le pictogramme « son coupé » d'une tuile d'alerte est en bas à droite** (`alerts_tab`,
  `Corner::BottomRight`), et non plus en haut à gauche : ce coin est celui de la case du mode
  sélection (`item_slot`), qui recouvrait le haut-parleur d'une tuile coupée dès qu'on entrait
  dans le mode. La croix garde le haut droit ; même retrait de 8 px.

**Captures** : `options_parametres_son_coupe` (Suivi coupé → bouton grisé, ligne d'essai des
Alertes), `options_parametres_combat_coupe` (tour cochée, son coupé), `options_alertes_liste` et
`options_alertes_selection` (la tuile coupée, badge en bas à droite).

### 9.1 octodecies Récap de session (2026-09-16)

Demande utilisateur : « ajouter un overlay en haut à gauche, en dessous des boutons du jeu,
présentant le récap de la session : XP gagné, kamas gagné, combats (gagné − perdu), challenges
(réussi − échoué), durée de la session », avec « la section "Recap", après la section "Combat",
dans l'onglet "Paramètres" » et « une option pour activer l'affichage de cet overlay, active par
défaut ». Le même jour, la section « Recap » est **remontée en tête de l'onglet** (« déplace la
section Recap en premier ») et « Démarrage » descend après « Fichier », parmi les réglages qu'on
pose une fois — l'ordre courant est celui commenté en tête de `OptionsTab::Parametres` dans
`panels/options_modal.rs`.

**Ce que c'est** : une quatrième zone d'overlay ancrée sur le jeu (`OverlayKind::Recap`,
`panels::recap`), à côté de Combat, Suivi et Options. Une bande d'une seule ligne, cinq cases
séparées par un filet, chacune un glyphe du design system et un chiffre, sur le fond translucide
déjà employé par le carré de contrôle du Suivi (`tokens::OVERLAY_BACKDROP`).

C'est la « bande coup d'œil » du web (`session-recap.component.html`, `.recap-bandeau`) et rien de
plus : le site déplie sous elle l'XP par personnage, la ventilation des kamas, le butin et les
accordéons par donjon — de la consultation APRÈS coup, pas du temps réel par-dessus un jeu.

**La durée est celle de l'overlay, jamais celle du fichier** — décision explicite de
l'utilisateur : « c'est par rapport à la durée d'uptime de l'overlay [...] plutôt que de se baser
sur le fichier ». Le web fait l'inverse (`StatsStoreService.accumulateSessionDuration` : somme des
écarts entre lignes horodatées, coupée au-delà de cinq minutes de silence), et ça ne convient pas
ici : un `wakfu.log` porte plusieurs sessions de jeu et l'overlay le relit en entier à son
démarrage, la durée annoncerait donc du temps de jeu d'avant-hier. Le chrono part au lancement du
processus (`App::started_at`) et avance tant qu'il tourne.

**Écart assumé, à connaître avant de croire à un bug** : les quatre autres chiffres, eux, couvrent
tout le fichier relu (`overlay_engine::SessionTotals`, alimenté par le rattrapage initial comme par
les lignes lues en direct). Un overlay lancé au milieu d'une partie affiche l'XP de toute la partie
en face d'une durée qui démarre à zéro. Les aligner demanderait de trancher ce qu'est « la
session » côté moteur : un autre chantier, et une décision qui n'a pas été prise.

**Ce que le moteur a gagné** : `SessionTotals::challenges_passed`/`challenges_failed`, les
challenges de TOUTE la session — `FightSnapshot` ne comptait que ceux d'un combat, et
`MAX_TRACKED_FIGHTS` purge les plus anciens, un total recalculé à la volée diminuerait donc en
cours de session (même raison que `fights_won`/`fights_lost`). Comptés même quand le parser n'a pas
résolu de `fightId`, miroir exact du web.

**Les détails qui ont demandé un arbitrage :**

- **Ancrage** : bord gauche, `client_top + GAME_RECAP_TOP_MARGIN_PX` (70 px = les 28 px de fausse
  barre de titre déjà mesurés, plus 36 px de bouton du jeu — `tokens::ICON_BUTTON_SIZE` —, plus
  6 px). Seule valeur de cette famille dérivée d'une mesure du design system plutôt que relevée sur
  une capture : à corriger sur retour d'écran.
- **Largeur pilotée par le contenu** : la bande mesure ce qu'elle occupe et le renvoie
  (`RenderOutcome::recap_width`), l'hôte y ajuste la fenêtre OS — même raison que le Suivi, une
  fenêtre plus large que sa bande capte les clics sur du vide en mode interactif.
- **Infobulles en dessous** (`TooltipSide::Below`, `RECAP_TOOLTIP_RESERVE`) : la bande est collée en
  haut, il n'y a rien au-dessus d'elle — la règle de §9.1 octies, à l'identique.
- **Le chrono se redessine tout seul** : `request_repaint_after(1 s)` depuis le panneau. Cette
  architecture ne rend une frame que lorsque quelque chose change (§6.1) ; la durée, elle, change
  sans que rien d'autre ne bouge.
- **Pas de symbole « ₭ »** derrière les kamas, contrairement au web : Ubuntu, la police embarquée,
  ne couvre pas U+20AD — la première version affichait un « ? », vu sur la capture du harnais. Le
  glyphe Kamas à gauche dit déjà de quelle monnaie il s'agit.
- **Aucune icône inventée** : `Xp`, `Kamas`, `MetricDamage` (les combats), `Trophy` (les
  challenges) et `Calendar` (la durée — la seule notion de temps du registre `DsIcon`, le jeu n'a
  pas de cadran).
- **La case vit avec les autres interrupteurs** (`FeatureToggles::recap`, §9.1 duodecies), persistée
  en local (`config::OverlayConfig::recap_enabled`, `true` par défaut, y compris pour un
  `config.toml` écrit avant ce champ). Décochée, la fenêtre est **masquée, jamais détruite** —
  `App::sync_panel_visibility`, qui porte désormais Combat ET Récap, même politique.

**Capture** : `recap_apres_rejeu_reel` (totaux du vrai rejeu, durée fixée à 1 h 23 min 45 s — elle
n'existe pas dans le fichier par construction).

### 9.1 novodecies Démarrage actif par défaut, bouton « Fermer l'overlay » (2026-09-16)

Deux demandes utilisateur du même jour, toutes deux dans l'onglet « Paramètres ».

**« Option de démarrage active par défaut. »** La case « Lancer l'overlay au démarrage de
l'ordinateur » (section « Démarrage », `overlay_ui::autostart`) lit et écrit l'état RÉEL du système
(clé `Run` sous Windows, `.desktop` sous Linux), jamais une copie en config — et le système ne
distingue pas « jamais inscrit » de « retiré exprès ». Un défaut ne peut donc pas se rejouer à
chaque lancement, sous peine de réinscrire ce que l'utilisateur vient de décocher. Il s'applique
**une seule fois par installation** : `autostart::enable_by_default_once`, appelée par les deux
hôtes juste après `config::load`, inscrit l'overlay puis lève le jalon
`config::OverlayConfig::autostart_initialized` (sauvegardé dans la foulée — la seule écriture de
`config.toml` hors « Valider »). Une config écrite avant ce champ vaut « jamais fait » : les
installations existantes sont inscrites, une fois, à leur premier lancement de cette version. Les
hôtes réécrivent le jalon à `true` à chaque sauvegarde (ils rebâtissent la config depuis leurs
champs). Un échec d'inscription lève le jalon quand même : la case reste là pour réessayer.

**« Un bouton pour fermer l'overlay à la toute fin de l'onglet, secondaire, centré, avec une
confirmation. »** Sous la section « Compte », sans section propre (ce n'est pas un réglage, c'est
la sortie) : `design::button` en `Secondary` — et non `Danger` comme « Se déconnecter » juste
au-dessus, parce que fermer ne détruit rien (compte appairé, réglages validés conservés) —, centré
comme lui parce que ce sont les deux seules actions de la fenêtre qui échappent à « Annuler ». La
confirmation (`OptionsModalState::pending_quit`, « Fermer l'overlay ? ») est la quatrième boîte
exclusive de la fenêtre ; « Oui » remonte `OptionsModalAction::Quit`, et l'hôte sort par le chemin
du raccourci « Quitter » et de la zone de notification (`logging::log_session_end` puis
`event_loop.exit()`, §11 — la borne de fin de session dit « Fermer l'overlay (fenêtre Options) »).

**Captures** : `options_parametres_fermer_overlay` (bas de l'onglet) ; `options_parametres_compte`
et `options_parametres_mise_a_jour` bougent avec la hauteur de l'onglet. L'aide de défilement des
tests (`defile_les_parametres`) retire désormais le pointeur AVANT les frames de repos : un bouton
centré passant sous lui ouvrait son infobulle, dont l'animation empêchait `Harness::run` de se
poser.

### 9.2 Design system — composants réutilisables (2026-09-09)

`crates/overlay-ui/src/design/` — couche introduite sur demande explicite de l'utilisateur, dont le
constat est le point de départ : « à chaque fois que je demande un bouton, il faut que j'explique
c'est tel composant, c'est tel label, il faut qu'il fasse telle taille ». Deux symptômes concrets
dans le dépôt avant cette couche :

- `panels::options_modal` peint ses trois boutons à la main — sept constantes de couleur et un
  appel `chamfer` chacun ; rien n'est réutilisable, rien ne se corrige en un seul endroit ;
- `assets/design-system/large-button-cancel.png` / `large-button-validate.png` sont **la même
  texture que `button-danger.png` / la texture primaire**, redécoupées à la taille du pied de page
  avec le libellé encore incrusté : un asset par taille **et** par libellé.

Décisions :

- **Une texture générique par variante, toutes les tailles au rendu** — peinture **9-slice**
  (`design::nine_slice`) : coins, liseré et **décor d'extrémité** figés, bandes médianes étendues.
  Les marges figées se dimensionnent sur l'étendue du décor (mesurée, ~50px sur un bouton 200×52),
  pas sur le rayon des coins : les hachures diagonales du jeu sont des embouts, pas une texture de
  fond, et une marge trop courte les laisse s'étirer sur toute la longueur du bouton (retour
  utilisateur 2026-09-09).
- **Manifeste unique** (`design::assets`) : nom logique → fichier + découpage. Seul endroit du crate
  où un chemin d'asset est écrit ; les fichiers sont référencés **directement dans
  `assets/design-system/`** (source tenue par le skill `design-asset`), pas recopiés dans le crate.
- **Chargement paresseux mémorisé par `egui::Context`** (`DesignSystem::get`) : aucun câblage dans
  `render_content`/`main.rs`, et rien n'est téléversé sur le GPU tant qu'aucun composant n'est
  utilisé (§8, budget mémoire).
- **Contrat de composant** uniforme — API paramétrable sans texture en argument, `impl
  egui::Widget`, trois états (repos/survolé/désactivé, l'appui retirant l'apparence survolée comme
  pour les boutons icône), écrêtage du contenu, journalisation à l'action et avertissement unique
  sur défaut de géométrie. Détail :
  `.claude/skills/ui-component/references/contrat-composant.md`.
- **Galerie de non-régression** (`crates/overlay-testkit/tests/design_gallery.rs`, §17.1) : toutes
  les variantes et tous les états sur une capture unique, à publier en Artifact.

**Doublon résorbé (2026-09-09)** : `panels::nine_slice` — seconde implémentation du 9-slice
arrivée le même jour avec la refonte visuelle de la modale Options (marge unique sur les quatre
côtés, étirement seul, pas de répétition) — est supprimé, ainsi que les quatre PNG de
`crates/overlay-ui/assets/ui/options/` que la modale chargeait (`footer-cancel.png`,
`footer-validate.png`, `browse-button[-hover].png`), tous des copies octet pour octet d'assets déjà
déclarés au manifeste `design::assets`. Les trois boutons de la modale sont maintenant des appels à
`design::button`.

Trois décisions prises à cette occasion :

- **Plusieurs textures par variante, choisies par la hauteur.** Le jeu capture le même bouton or à
  deux hauteurs, et son embout décoratif n'y a pas la même largeur (52px sur la texture 200×52,
  34px sur la 338×36). Le composant retient donc la texture dont la hauteur native est la plus
  proche de la hauteur demandée. `button-primary-compact[-hover].png` a été générifiée depuis
  `large-button-validate[-hover].png` par le skill `design-asset`.
- **Gouttière de pied de page** : 12pt entre « Annuler » et « Valider », l'échelle des 15px mesurés
  sur `interface-options-jeu.png` (boutons en x 18..351 et 367..700 sur 720). La première version
  les collait l'un à l'autre.
- **Police des libellés** (`design::fonts`, `design::text`) : egui n'embarque qu'une Ubuntu Light
  et n'expose aucun réglage de graisse, mais `FontDefinitions` accepte n'importe quel fichier que
  nous embarquons. `assets/fonts/Ubuntu-Medium.ttf` (Ubuntu Font Licence, licence jointe) est
  enregistrée comme famille nommée `ds-label` par `style::apply` ; seuls les libellés du design
  system l'utilisent, la proportionnelle par défaut reste celle du reste de l'interface. Corps et
  police choisis par un balayage rendu dans egui puis comparé au pixel aux libellés gravés du jeu
  (voir `tokens::BUTTON_FONT_SIZE_RATIO` et la doc de `design::fonts`).

  La première version fabriquait une graisse **synthétique** (halo d'un pixel à opacité réduite),
  retirée le 2026-09-09 après-midi : un halo est un contour, pas une graisse, et rend le libellé
  flou. En reste une leçon à ne pas redécouvrir : **egui arrondit la position d'un texte au pixel
  entier** (`Options::round_text_to_pixels`), aucun effet visuel ne se règle par un déplacement
  fractionnaire de texte.
- **Police des titres** (`design::fonts`, `design::text::title_font`) : le jeu n'a **pas une seule
  police**. Ses libellés de bouton sont dans une linéale, ses titres dans une **serif grasse**.
  `assets/fonts/PTSerif-Bold.ttf` (SIL OFL, licence jointe) est enregistrée comme famille nommée
  `ds-title` — meilleure correspondance libre parmi vingt-six serifs comparées aux deux titres du
  jeu. Corps 21 sur la bannière de modale, 18 en titre de section.
- **Cerne d'un texte** (`design::text::paint_outlined_text`) : la liste des décalages est un
  **paramètre**, avec deux jeux nommés, et le choix dépend du fond. `OUTLINE_FULL` (huit voisins)
  pour le texte qui flotte nu par-dessus le jeu — fond arbitraire, une ombre d'un seul côté y
  devient illisible dès que le fond est clair de ce côté-là ; `SHADOW_BOTTOM_RIGHT` (trois voisins)
  pour un texte posé sur un fond connu, ce que fait le jeu pour ses titres, éclairés depuis le
  haut-gauche. La fonction était une fonction privée de `panels::combat` avec ses huit décalages en
  dur ; elle a remonté dans le design system le 2026-09-09.
- **Une « section » n'a ni fond, ni bordure, ni filet** (`docs/design-system/releve-section-options.json`) :
  dans le jeu, le seul signal de regroupement est l'espacement, et le seul signal de niveau est le
  retrait de 7px du titre par rapport à ses lignes. Ce qui a un fond, un bord et un rayon, c'est le
  **panneau de contenu** qui contient les sections (`#15181c`, bord 2px `#131518`, rayon **2**).
  `panels::options_modal` appelait « section » ce qui est un panneau, et lui avait donné un rayon de
  18 lu à l'œil — un rayon qui n'existe nulle part dans cette interface.
- **Un titre de section est gris (`#b8b9ba`), au corps du titre de fenêtre** — la hiérarchie entre
  les deux niveaux passe par la couleur, pas par le corps. Attention en mesurant : la hauteur
  d'encre d'un titre varie avec le mot (25px pour « Échelle de l'interface (100%) », accent de
  capitale et parenthèses descendantes ; 22px pour « Thème d'interface personnalisé », jambage seul ;
  14px pour « Combat », rien du tout). **C'est la ligne de base qui est stable, pas la boîte** —
  comparer l'encre de deux mots différents pour en déduire un rapport de corps donne un résultat
  faux, ce qui est arrivé une fois.
- **Deux graisses de libellé de bouton**, mesurées sur `interface-options-interface.png` : le pied
  de page de modale est plus gras que le contenu, à hauteur d'encre identique. Voir
  `design::fonts` et `ButtonVariant::label_strong`.
- **Hauteur d'un bouton = hauteur de sa texture**, jamais déduite de sa largeur. La modale Options
  écrivait `footer_button_width * (36.0 / 338.0)`, à l'inverse de ce à quoi sert un 9-slice, et
  affichait des boutons de 27px là où le jeu en met 36 — le corps du libellé suivant la hauteur, il
  y perdait 3px d'encre sur 13. Voir `panels::options_modal::FOOTER_BUTTON_HEIGHT`.

**Deux décisions structurantes prises le 2026-09-10**, après un diagnostic complet de la couche :

- **Le contrat de composant a deux familles.** Une *feuille* implémente `egui::Widget` ; un
  *conteneur* — celui qui encadre du contenu fourni par l'appelant — expose un `show` générique sur
  le retour de ce contenu, parce que `fn ui(self, ui) -> Response` n'a de place ni pour ce contenu
  ni pour ce qu'il rend. egui a tranché de la même façon : aucun de ses conteneurs n'implémente
  `Widget`. `design::scroll_area`, jusque-là documenté comme « seul écart au contrat », en devient
  le premier cas nominal. Voir `.claude/skills/ui-component/references/contrat-composant.md` §1 bis.
- **Le vocabulaire visuel des panneaux flottants est fixé.** Combat et Suivi prennent les *formes*
  et la *typographie* du jeu, et **gardent leur accent cyan** (`#00d2ff`), promu en jeton nommé
  `tokens::OVERLAY_ACCENT`. Motif : un overlay se lit par-dessus le jeu, sur un fond arbitraire ;
  le cyan n'existe nulle part dans l'interface Wakfu, ce qui est exactement ce qui l'empêche de s'y
  confondre. Conséquence pour §9.2 : **un seul jeu de composants**, dont la teinte vient d'un jeton
  (`OVERLAY_ACCENT` pour ce qui flotte, les jetons du jeu pour ce qui vit dans une fenêtre) — jamais
  d'un paramètre de thème par composant.

Catalogue et état d'avancement : [`docs/design-system-composants.md`](design-system-composants.md).
Feuille de route de la suite (couche conteneur, ménage, composants de données) :
[`docs/plan-composants-ui.md`](plan-composants-ui.md).
Composants livrés : le bouton texte, puis le champ de saisie (`design::input`, 2026-09-10 — hauteur
native 25px, valeur en or `#f4d89e` et non en blanc, texte indicatif peint à la main parce qu'egui
impose sa propre couleur à un `hint_text`). `panels::icon_button::paint_icon_button`, hors contrat
(quatre `TextureHandle` en paramètres), **a été migré et supprimé le 2026-09-10** : les quatre
boutons du carré de contrôle du Suivi passent par `design::icon_button`, la taille d'encre des
glyphes est au manifeste (`tokens::ICON_BUTTON_CONTENT`, 18px pour un socle de 36, mesuré sur
`menu-button-icon-first-plan.png`), et le module résiduel — une infobulle et un fond de barre — a
été renommé `panels::tooltip`.

---

## 10. Sécurité, vie privée, conformité

- **Contrainte de conception non négociable** : lecture d'un fichier texte produit par le jeu, et
  rien d'autre. Jamais de lecture mémoire, jamais d'injection DLL/hook, jamais de capture d'écran,
  jamais d'automatisation d'entrées. C'est la posture déjà tenue par l'app web ; l'overlay ne
  l'élargit pas.
- Aucune donnée ne sort en mode invité (§7.3).
- Le contenu du log est **hostile par nature** (messages de chat écrits par des tiers) : tout texte
  affiché est traité comme donnée, jamais interprété ; longueurs bornées ; parsing sans
  récursion non bornée.
- Mises à jour : binaire et bundle moteur signés (`minisign`/ed25519), signature **vérifiée avant
  exécution ou chargement**. Un asset de Release non vérifié n'est jamais chargé — un moteur JS
  téléchargé est du code exécutable, pas de la donnée.
- Le jeton de session ne transite jamais en clair sur disque hors trousseau (§7.2), n'apparaît
  jamais dans les logs de l'overlay.

---

## 11. Distribution

- **Windows** : binaire + installeur NSIS/MSI. Signature Authenticode fortement recommandée (sans
  elle, SmartScreen effraie chaque nouvel utilisateur) — coût à budgéter, mais l'app reste
  installable sans.
- **Linux** : AppImage (cible principale, aucune dépendance système à gérer) + `.deb`. Dépendances
  runtime documentées : Vulkan/Mesa, X11, `libsecret` (optionnel, cf. repli §7.2).
- **Numéro de version : une seule source, incrémentée automatiquement** (décision du 2026-09-14).
  `[workspace.package] version` du `Cargo.toml` racine est la version du PRODUIT ; toutes les crates
  y renvoient (`version.workspace = true`), Cargo l'embarque dans le binaire, et
  `crates/overlay-ui/build.rs` y ajoute le hash du commit compilé (`overlay_ui::build_info`). Le
  hook `post-commit` (`scripts/bump-version.sh`) l'incrémente d'après le type Conventional Commits
  du commit — `feat:` → minor, `fix:`/`style:`/`perf:`/`refactor:`/`test:` → patch, `!`/`BREAKING
  CHANGE:` → major, `docs:`/`chore:`/`ci:`/`build:` → rien — puis AMENDE ce commit, de sorte que le
  bump ne vive jamais séparément du changement qui l'a causé. Ce que la distribution en attend : un
  asset de Release et un rapport de bug désignent tous deux un état exact du dépôt, sans qu'il
  faille penser à relever un numéro à la main. Modèle repris du dépôt web (`tools/bump-version-from-
  commit.mjs`), avec deux gardes que celui-ci n'a pas : aucun bump pendant un `rebase`/`merge`/
  `cherry-pick` ni sur un commit de fusion (rejouer un commit déjà versionné le bumpait une seconde
  fois — bug vécu côté web).
  - **Conséquence sur le gate de captures** (§17.1) : la bannière de la fenêtre Options peint ce
    numéro, donc chaque bump périmerait toutes ses captures. `overlay_ui::build_info::
    freeze_for_snapshots` le fige à `0.0.0` pour le harnais, appelée depuis le point de passage
    obligé de `tests/panels.rs` (`Textures::get_or_load`) plutôt que test par test.
- **Mise à jour automatique** : plan dédié dans [`plan-mise-a-jour.md`](plan-mise-a-jour.md)
  (2026-09-15, décisions du mainteneur en §10) — GitHub Releases, workflow de release sur la
  fusion `dev` → `main`, manifeste `latest.json` signé `minisign`, module `overlay_sync::update`
  (`ureq` + `self-replace` + `minisign-verify`), installation au démarrage derrière l'écran de
  chargement, différentiel après mesure. **Un binaire de Release vise toujours la prod**
  (`DEFAULT_BASE_URL`), le domaine dev n'est plus utilisé qu'en local par les scripts de preview.
  - ~~Le bundle moteur peut être mis à jour **sans** nouvelle version du binaire (asset versionné +
    signature)~~ — **retiré le 2026-09-15** (décision 7 du plan de mise à jour) : le bundle est
    vendu et diverge volontairement du dépôt web (`engine-js/VENDORED_FROM.txt`, 2026-09-13), et
    l'exe se met à jour tout seul ; un second canal signé pour 31 Ko n'a plus d'objet.
- CI GitHub Actions : build Windows + Linux, tests de parité, budget mémoire, lint `clippy -D warnings`.
- **Version de Rust épinglée** par `rust-toolchain.toml` (décision du 2026-09-10), et non plus
  suivie sur `stable`. Motif : `cargo fmt --check` et `clippy -D warnings` sont des gates dont le
  verdict dépend de la version de l'outil — tant que le CI suivait `stable`, la sortie d'une
  nouvelle version de Rust suffisait à le faire rougir sur du code inchangé, sans qu'un poste de
  dev à une autre version puisse le reproduire (c'est ce qui a tenu le CI rouge du 2026-09-01 au
  2026-09-10, sur un seul écart de format dans `session.rs`). Monter Rust devient un changement
  explicite, dans son propre commit. Plancher : `rust-version = 1.95` déclaré par egui 0.36.1.
- **Garde-fou local** : `scripts/ci-local.sh` rejoue les vérifications du CI pour la plateforme
  courante, et le hook `pre-push` de `.githooks/` (activé par `scripts/install-hooks.sh`) le lance
  en mode `--lint` avant chaque push. Le script double volontairement le workflow — les deux listes
  d'étapes doivent être maintenues ensemble.
- **Dépôt public depuis le 2026-09-15 au plus tard** (vérifié sur l'API GitHub) : minutes Actions
  gratuites sur les runners standard. Il a été privé (jobs Windows facturés au double), et le
  workflow garde la sobriété acquise à cette époque — durée d'attente du verdict autant que coût :
  cache `~/.cargo`/`target` (`Swatinem/rust-cache`) et le `vendor/wgpu-hal` patché, annule les runs
  obsolètes d'une même branche (`concurrency`), et ignore les pushs purement documentaires
  (`paths-ignore` : `docs/**`, `**/*.md`, `.claude/**` — jamais `assets/**`, embarqué par
  `include_bytes!`). Symptôme d'un quota épuisé, à ne pas confondre avec une régression : tous les
  jobs échouent en quelques secondes, sans log ni runner assigné.

---

## 12. Feuille de route

| Lot | Contenu | Critère de sortie |
| --- | --- | --- |
| **S1 — Spike rendu Windows** ✅ fait | Fenêtre transparente + always-on-top + click-through + DirectComposition + wgpu | Voir `spikes/s1-window-windows/README.md` : panneau egui semi-transparent confirmé par capture d'écran par-dessus une autre fenêtre, hotkey global de bascule confirmé sans focus, RSS ~87 Mo — **verdict : chemin DirectComposition via `wgpu-hal` (`DxgiFromVisual`) validé**, plus simple que prévu (§6.2 mis à jour) |
| **S2 — Spike moteur** ✅ fait | Bundle headless TS + QuickJS, ingestion de `tests/wakfu.log` | Voir `spikes/s2-engine-quickjs/README.md` : correction confirmée (rejeu identique), débit ~28 000 l/s (sous la cible initiale de ~40 000 l/s, ×1,4), critère de fluidité UI reformulé en §5.5 — **verdict : choix QuickJS maintenu** |
| **S3 — Spike X11** ✅ fait | Équivalent S1 sous X11 + XWayland, **développé conjointement avec son harnais de test Xvfb** (voir §17.2) — implémentation et harnais en TDD, pas l'un après l'autre | **Spike validé (2026-09-03), critère de sortie atteint le 2026-09-04** (sauf point 5, question ouverte par nature — voir §17.2 « État »). Voir `spikes/s3-window-linux/README.md` pour le détail complet du spike : les trois prérequis de faisabilité vérifiés dans l'ordre — rendu logiciel Vulkan/lavapipe sous Xvfb confirmé avec du vrai code `wgpu`, WM EWMH (`openbox`) nécessaire et suffisant, scénario sans compositeur (repli opaque) validé comme cas par défaut et scénario avec compositeur en secondaire. Six bugs réels trouvés et corrigés dans le spike (dépendance système manquante, panique `TexturesDelta` en debug, double événement `global-hotkey` sous X11, piège de titre `xterm` dynamique, piège regex `xdotool search`, `[workspace]` manquant préexistant sur S1/S2 aussi). **Critère de sortie enrichi (2026-09-03, revue à 3 experts, §17.6) — rempli aux points 1/2/3/4 le 2026-09-04, voir §17.2 « État »** : le CODE **migré** vers `crates/overlay-platform/src/linux/` (7 tests unitaires topmost verts) ; le multi-fenêtres réel **câblé** — nouveau binaire `crates/overlay-ui/src/bin/overlay-ui-x11.rs` (mode invité), validé sous Xvfb avec preuve visuelle (2 fenêtres Combat/Suivi réelles) ; le HARNAIS **extrait** vers `xtask visual-check` (Rust, garde RAII, assertions par le protocole X11) — toutes les assertions passent sur le VRAI binaire (ancrage, click-through 1→0→1, topmost/délai de grâce/réaffirmation), un second bug réel trouvé au passage (`xdotool search --name` incapable de matcher un tiret cadratin UTF-8 dans son pattern, contourné) |
| **L7 — Outillage de test visuel (Niveau 1)** | Rendu offscreen déterministe des panneaux `overlay-ui` (crate `overlay-testkit`), voir §17.1 | Un changement de panneau (combat/watchlist) produit un diff visuel détecté automatiquement, sans Xvfb ni GPU physique, exécutable dans une session Claude cloud headless — critère détaillé en §17.6 |
| **L1 — Ingestion** ✅ fait | tail, rotation, découverte de chemin, `isInitialLoad` | Voir `crates/overlay-ingest/` : rejeu, ligne partielle, troncature et rotation (suppression + recréation) couverts par des tests synchrones sur `Tailer::poll` ; watcher temps réel (`notify` + repli) vérifié séparément (`tests/watcher_smoke.rs`, manuel). Réserve trouvée puis corrigée le 2026-09-04 (§5.2) : la détection de rotation combine désormais l'identité de fichier ET un second signal (préfixe de contenu), qui ne dépend pas de l'hypothèse fausse « inode jamais réutilisé ». |
| **L2 — UI** 🟡 en cours | dégâts, suivi, alertes, récap | Utilisable en jeu une soirée sans redémarrage — **fait** : `crates/overlay-engine/` (QuickJS + `LogParser` vendu → `LogEntry` → `SessionSnapshot`, + `watchlist.rs` — comptage du Suivi et alertes de décompte portés en Rust, voir §14 point 3) et `crates/overlay-ui/` (fenêtre S1, deux fenêtres overlay indépendantes Combat/Suivi — bande de tuiles, alertes son+toast sur décompte à 0), validés sur un vrai `wakfu.log`. **Fait (2026-09-02, suite)** : Alertes de drop version « ramassage avec son activé » — `overlay_engine::profile` lit `data.profile.soundItems` (`GET /api/v1/settings`), indépendant de la watchlist ; `Engine::drain_loot_alerts` déclenche toast + son (`alert_sound::play_loot_alert`, fichier mp3 identique au web) pour tout objet ramassé dont le son est activé au compte (objets par défaut `DEFAULT_SOUND_ITEM_NAMES` ou ajoutés par l'utilisateur, mêmes règles), suivi ou non — miroir de `registerLoot`/`ProfileService.findEnabledSoundItem`. **Fait (2026-09-02, refonte visuelle)** : le toast (`panels::watchlist::toast_card`) reproduit la carte du dépôt web (`loot-alert.component`) — icône réelle, titre/bordure `--accent`, nom (+ quantité), confettis tombants (dispersion tirée une fois par déclenchement, animée en continu tant que le toast est affiché), fermeture au clic sur la carte OU sur une croix EN PLUS de la minuterie fixe (les deux cohabitent, contrairement au réglage exclusif `ProfileService.alertManualClose` côté web, pas encore porté) ; toast affiché même watchlist vide (ramassage à son activé indépendant de la watchlist) ; fenêtre Suivi élargie/agrandie dynamiquement le temps qu'un toast est affiché (`watchlist_target_width`/`_height`), comme pour le nombre d'entrées. **Fait (2026-09-07, retour utilisateur : un Suivi jamais visible sur le site)** : les COMPTEURS du Suivi sont désormais répliqués vers le compte (`PATCH /api/v1/settings`, débounce + backoff, voir §14 point 3 pour le détail complet) — jusqu'ici locaux à l'overlay uniquement. **Reste** : indicateur visuel d'état de synchro dans l'UI (idle/pending/syncing/error, §9 — le mécanisme réseau existe désormais pour l'historique ET le Suivi, seul l'affichage manque). **Retiré (2026-09-02, décision du mainteneur, voir §9)** : thème configurable/mode daltonien ; disposition persistée par écran (poignée de glissement, `layout_store`, un temps implémentée puis retirée pour la même raison — un overlay n'est pas un site, pas de personnalisation de disposition) — palette fixe et ancrage automatique seul assumés |
| **L3 — Catalogue** ✅ fait | fetch, cache, repli embarqué, index O(1) | Résolution d'objet identique au web sur les golden files — **fait (2026-09-02, retour utilisateur)** : `overlay_engine::catalog` (index O(1) par id/nom, depuis `GET /api/v1/catalog/`) + `overlay-sync` (fetch + cache disque `catalog_cache.rs`, offline-first) + `overlay-ui::remote_icons` (résolution/téléchargement/cache d'icônes réelles `wakassets` pour le panneau Suivi, vérifié en direct contre le déploiement dev). **Fait (2026-09-02, suite)** : `catalog::find_item_has_recipe` (drapeau recette) et `catalog::find_monster_classification`/`find_monster_family_id` (boss/archimonstre/dominant, priorité `MonsterClassification`, miroir de `resolveFightTypeClassification`) ; `dungeon.rs`/`monster_family.rs` — deux nouveaux index O(1) (par id, + réciproque boss→donjon) construits depuis `GET /api/v1/dungeons`/`GET /api/v1/monster-families` (`overlay_sync::client::fetch_dungeons`/`fetch_monster_families`, cache disque `reference_data_cache.rs`, sans endpoint `/version` dédié côté serveur donc toujours rechargés en tâche de fond) ; repli hors-ligne embarqué (`overlay_sync::catalog_cache::embedded_fallback`, `assets/catalog/catalog-index.json.gz` via `include_bytes!`, décompression `flate2`) branché dans `spawn_catalog_thread` (utilisé seulement si aucun cache disque ET réseau injoignable) ; golden files de non-régression (`crates/overlay-engine/tests/golden/*.json` + `tests/catalog_golden.rs`, cohérence croisée catalogue/donjons/familles). **Clôturé (2026-09-02, poste de dev avec accès réseau réel)** : les lots précédents avaient été développés dans un sandbox sans accès à Neon/`*.pages.dev` ni à `overlay-ui` (Windows-only, non buildable là-bas) — ce n'est plus le cas ici, les trois points bloquants ont donc été levés pour de vrai plutôt que redocumentés comme limite : (1) `claude-dev.wakfu-companion.com` confirmé joignable (`catalog/`, `catalog/version`, `dungeons`, `monster-families` en 200) ; (2) repli embarqué **régénéré depuis ce déploiement réel** via `cargo run -p overlay-sync --bin gen-catalog-fallback` — catalogue complet (~1,8 Mo bruts / ~489 Ko gzip), n'est plus un placeholder ; (3) petit indicateur « 📦⚠ catalogue daté » ajouté dans la zone Combat de `overlay-ui` (`catalog_stale: Arc<AtomicBool>`, posé par `spawn_catalog_thread` uniquement quand le repli embarqué est utilisé, tooltip explicatif) — remplace le `tracing::warn!` jusque-là invisible en jeu. `cargo build`/`test`/`clippy -D warnings`/`fmt --check` **propres sur les 5 crates du workspace, `overlay-ui` compris** (précédemment non vérifiable en sandbox). **Volontairement reporté, pas un blocage de clôture** : brancher `DungeonIndex`/`MonsterFamilyIndex` dans `overlay-ui` — aucun panneau §9 n'en a besoin aujourd'hui (`LogEntry` n'a pas de `dungeonId` par combat, voir `model.rs`), prévu pour un futur panneau Combat conscient du donjon, pas une régression de ce lot. |
| **L4 — Auth native** ✅ fait | endpoints d'appairage (dépôt web) + trousseau | Connexion Discord/Google depuis l'overlay, session révocable — **fait** : 3 endpoints serveur (`/api/v1/auth/native/{pair,claim,poll}`, table `native_pairings`, `Authorization: Bearer` accepté par `_auth.ts`), page web `/pair`, crate `overlay-sync` (pairing bloquant + `keyring`/repli fichier + `GET /settings`), roster appliqué à `overlay-engine::session` (priorité sur `breed`), portraits de classe affichés dans le panneau Combat (`overlay-ui`). **Fait (2026-09-02, suite)** : révocation/déconnexion — raccourci global `Ctrl+Alt+D` (`App::disconnect_account`), commande `AuthCommand::Disconnect` traitée par le thread Auth (`spawn_auth_thread`, restructuré pour rester vivant après une connexion réussie plutôt que de se terminer, condition requise pour pouvoir déconnecter PUIS reconnecter sans redémarrer l'overlay) — efface le jeton (trousseau + repli fichier) et notifie le thread Engine (`EngineCommand::Disconnect`) qui repasse en mode invité (roster `None` → repli `breed`, Suivi vidé ; compteurs locaux déjà persistés conservés pour une reconnexion ultérieure). Vérification bout en bout **partiellement levée** (poste de dev avec accès réseau réel, comme pour L3) : `claude-dev.wakfu-companion.com` confirmé joignable sur les trois routes d'appairage natif (`pair` → 200 avec code+URL réels, `poll` → `pending`/`expired` conformes au format attendu par `pairing.rs`, `settings` sans/avec jeton invalide → 401 comme attendu) ; la complétion réelle d'un appairage (connexion Discord/Google dans un navigateur) reste non automatisable depuis ici et n'a donc pas été rejouée. **Clôturé (2026-09-02, suite — UI de pairing)** : le code d'appairage n'était visible qu'en console — nouvel état `AuthStatus::PairingStarted { pairing_code, verification_url }` (publié par `attempt_connect` dès que le code est obtenu, avant le premier sondage), affiché directement dans la fenêtre overlay (zone Combat) sous forme d'une carte compacte — code en grand (lisible/tapable), bouton copier (`egui::Context::copy_text`), bouton pour rouvrir la page de vérification (`open::that`, utile si l'ouverture automatique du navigateur a échoué ou si l'onglet a été fermé par erreur). **Refondu (2026-09-14, §9.1 undecies)** : la carte de la zone Combat et le raccourci de déconnexion ont laissé place à une **fenêtre de connexion** dédiée (`OverlayKind::Login`, `panels::login`), seule interface tant qu'aucun compte n'est lié — plus de mode invité, appairage lancé sur « Se connecter » seulement, annulable, déconnexion depuis la fenêtre Options ou l'icône de zone de notification. **Reste** : la complétion réelle d'un appairage (connexion Discord/Google) reste non automatisable depuis ce sandbox (limite d'environnement, pas une lacune du code), comme documenté ci-dessus |
| **L5 — Synchro** ✅ fait | file SQLite, lots, backoff, idempotence | Rejeu 10× du même log ⇒ **aucun** doublon en base, y compris en alternant web et overlay — **fait (2026-09-02)** : `overlay_engine::history` (signatures + payloads fight/purchase/trade) + `overlay_engine::log_time` (dates réelles depuis `LogDateAnchor`) + `overlay_sync::queue::SyncQueue` (file SQLite idempotente, lots de 50, backoff 15s→5min, abandon après 10 tentatives non réseau — testé par rejeu 10x, critère de sortie ci-contre) + câblage `overlay-ui` (thread Sync dédié, activé/désactivé avec le compte). **Complété (2026-09-02, suite, depuis le dépôt web local)** : ventilation par sort/élément, xpGained par participant ET total du combat, résolution monsterId/itemId par catalogue, gameServer (déduit du roster), récupération de kamas HDV sans achat adjacent. **Clôturé (2026-09-02, suite — demande explicite du mainteneur de lever les deux limites restantes)** : regroupement de donjon multi-salles complet (`overlay_engine::dungeon_run`, port direct de `dungeon-run-grouping.util.ts` + `findDungeonForEnemies` 3 priorités, siblings renvoyés avec le rattachement une fois le run complété — voir §7.3) ET branchement réel `overlay-ui` (`spawn_dungeon_thread`, `GET /api/v1/dungeons` → `Engine::set_dungeons`) : `dungeonId`/`dungeonRunKey` sont désormais alimentés en production, plus seulement prêts côté moteur. **Reste** : rien — voir §7.3 pour le seul renoncement volontaire restant (`turns`, jamais utilisé) |
| **L6 — Packaging** 🟡 en cours | AppImage, installeur, mise à jour signée | Installation propre sur une machine vierge Windows et Linux — **mise à jour signée : plan et phase 0 faits le 2026-09-15** ([`plan-mise-a-jour.md`](plan-mise-a-jour.md) : profil release, API prod par défaut, procédure des clés) ; installeur et AppImage restent à faire |

S1/S2/S3 étaient prévus **bloquants** (ils peuvent remettre en cause la stack). **Décision du
mainteneur (2026-08-31) : S3 est reporté** faute de machine Linux disponible pour l'instant — S1 et
S2 ont validé la stack sur Windows, suffisant pour démarrer L1. S3 reste à faire **avant** tout
travail spécifique à `overlay-platform::linux` (§6.4) ou toute distribution Linux (§11) : le risque
qu'il couvre (Wayland/X11, XWayland avec le vrai client Wakfu) ne disparaît pas, il est seulement
découplé du reste de la feuille de route.

**Mise à jour (2026-09-03)** : une session Claude cloud (Linux, sans machine Windows ni matériel
Linux physique) a pu lever une bonne partie du blocage « aucune machine Linux disponible » —
Xvfb + rendu logiciel (lavapipe) suffisent à valider tout ce qui ne dépend pas d'un vrai compositeur
matériel. Le spike lui-même est validé (voir `spikes/s3-window-linux/README.md` et la ligne S3
ci-dessus) ; XWayland avec le vrai client Wakfu, en revanche, reste hors de portée d'un
environnement cloud headless (pas de session Wayland réelle à faire tourner) et devra être vérifié
sur une vraie machine Linux le moment venu, comme documenté au §17.2 (limite assumée).

**Mise à jour (2026-09-13)** : cette vérification « vraie machine Linux » est désormais possible —
le mainteneur dispose d'un **Steam Deck (SteamOS, KDE Wayland + XWayland)** avec le client Wakfu
natif (Zaap). Deux conséquences outillées dans le dépôt plutôt que laissées à la manœuvre manuelle :
`crates/overlay-ui/preview.sh` (équivalent bash de `preview.ps1`, lance `overlay-ui-x11`) et
`scripts/setup-steamdeck.sh` (conteneur `distrobox` Arch partageant HOME/écran/GPU/audio, seul moyen
durable de compiler sous SteamOS dont le rootfs est en lecture seule et réécrit à chaque mise à
jour ; `preview.sh` y entre tout seul). Premier constat de cette vérification sur matériel réel : la
découverte de `wakfu.log` sous Linux (§5.1) pointait vers `~/.config/zaap/gamesLogs/wakfu/wakfu.log`
alors que le client natif écrit dans le sous-dossier `logs/`, comme sous Windows — corrigé dans
`overlay-ingest::discovery` (le chemin sans `logs/` reste candidat, en repli).

---

## 13. Revue des trois experts

### Expert 1 — Architecte Rust temps réel  ✅ Validé sous réserve

Le modèle « un seul propriétaire de l'état + snapshot immuable publié par `ArcSwap` » est la bonne
réponse : il supprime la classe entière des bugs de verrou entre parsing et rendu, et rend le rendu
réactif trivial à implémenter. Le découpage IO / Engine / UI / Sync est justifié par des raisons de
latence réelles, pas par goût du parallélisme.

*Réserves formulées, intégrées au plan :*
1. Le budget de 300 Mo est tenable mais **seulement s'il est mesuré en CI** — une cible non mesurée
   dérive toujours. → `xtask mem-budget`, §8.
2. QuickJS mono-thread est un point de sérialisation : l'ingestion initiale de 80 000 lignes doit
   être **découpée en lots** et publier des snapshots intermédiaires, sinon l'UI paraît gelée même
   si elle ne l'est pas techniquement. → §5.5.
3. La frontière `EngineBackend` doit exister **dès le premier commit**, même avec un seul
   implémenteur. Ajoutée après coup, elle ne serait jamais posée au bon endroit.

*Point de désaccord assumé* : je porterais volontiers le parser en Rust natif à terme (perf, absence
de dépendance JS). Mais pas en v1, et pas sans le harnais de parité : le risque de doublons en base
via une `client_key` divergente est un coût utilisateur bien supérieur au gain de quelques
millisecondes.

### Expert 2 — Plateforme Windows  ✅ Validé sous réserve

Le chemin `WS_EX_LAYERED` + `WS_EX_NOREDIRECTIONBITMAP` + DirectComposition + surface wgpu sur
visual est le bon, et c'est le seul qui tienne avec un swapchain moderne. `WS_EX_NOACTIVATE` est
tout aussi important que la transparence : un overlay qui vole le focus au client de jeu est
inutilisable, et c'est l'erreur la plus fréquente sur ce type d'outil.

*Réserves :*
1. **S1 avant tout le reste.** Tant que ce spike n'a pas tourné sur au moins deux GPU (un NVIDIA, un
   Intel/AMD intégré), la faisabilité n'est pas démontrée. Prévoir le repli « fenêtre compagnon
   accolée » comme un vrai chemin produit, pas comme un aveu d'échec.
2. Le sondage de curseur à 60 Hz est acceptable (`GetCursorPos` est bon marché), mais l'ordre des
   opérations compte : basculer le hit-test **puis** rendre, jamais l'inverse, sinon un clic
   arrive à la fois au jeu et à l'overlay pendant une frame.
3. Wakfu en « plein écran exclusif » ne recevra jamais d'overlay correct : le mode fenêtré sans
   bordure doit être vérifié au démarrage et signalé à l'utilisateur, pas supposé.

### Expert 3 — Plateforme Linux  ✅ Validé sous réserve

Le choix d'annoncer **X11 supporté / Wayland natif non supporté** est le bon appel, et surtout la
seule position honnête : sur Wayland, l'absence de positionnement client et de hit-test n'est pas un
manque de winit, c'est le protocole. Prétendre le contraire produirait des rapports de bug
insolubles.

*Réserves :*
1. Le chemin du log Linux (§5.1) est **une hypothèse non vérifiée**. La détection multi-chemins et
   le sélecteur manuel doivent être livrés dès L1, pas ajoutés après le premier retour utilisateur.
   Un joueur sous Steam/Proton a un préfixe Wine imprévisible.
2. La transparence X11 exige un **compositeur actif** — courant sur GNOME/KDE, absent sur i3/dwm nus.
   Le repli opaque doit être automatique et silencieux, jamais une fenêtre noire inexpliquée.
3. `libsecret` est absent de beaucoup d'installations minimalistes : le repli fichier `0600` du
   jeton (§7.2) est indispensable, et doit être **annoncé** à l'utilisateur, pas silencieux.
4. Valider XWayland tôt (S3) avec le vrai client Wakfu : le comportement « always-on-top » d'une
   fenêtre XWayland dépend du compositeur, et c'est le point le plus susceptible de décevoir.

### Point de convergence des trois

Les trois experts convergent sur un même arbitrage : **la réutilisation du code métier TS n'est pas
un raccourci, c'est la décision qui protège la donnée utilisateur.** Le coût d'un moteur JS embarqué
(quelques dizaines de Mo, un spike de perf) est faible ; le coût d'une divergence de `client_key`
entre deux clients qui écrivent dans la même base est permanent et invisible.

---

## 14. Décisions ouvertes (à trancher par le mainteneur)

1. ~~**Moteur** : QuickJS embarqué ou port Rust intégral ?~~ **Tranché par S2 (voir
   `spikes/s2-engine-quickjs/`) : QuickJS embarqué, confirmé.** Correction validée sur
   `LogParser` (rejeu identique après `reset()`), débit ~28 000 lignes/s — sous la cible initiale
   mais sans conséquence grâce au threading (§5.5, reformulé). Reste à valider sur
   `StatsStoreService` (bien plus gros, couplé Angular) lors de l'extraction réelle côté
   `wakfu-companion` (point 3 ci-dessous).
2. ~~**Auth native** : appairage par code (recommandé) ou redirection loopback ?~~ **Tranché :
   appairage par code, implémenté (voir L4 ci-dessus)** — ne touche à aucune configuration OAuth
   existante, conforme à la recommandation initiale de ce document.
3. **Extraction du moteur headless** : lot à planifier **dans `wakfu-companion`** — qui le fait,
   quand, et sur quelle branche ? **Reste ouvert pour le récap de session complet et les
   heuristiques kamas/HDV** (`StatsStoreService`, 2 755 l.). **Tranché différemment pour le Suivi
   (watchlist), 2026-09-01** : la logique de comptage réelle (`registerLoot`/`registerDefeat`/
   `incrementWatched`) est une trentaine de lignes isolées, sans heuristique de corrélation
   comparable à celles qui motivaient la décision B du §2 — portée directement en Rust
   (`overlay-engine::watchlist`) plutôt que d'attendre l'extraction complète, décision utilisateur
   assumée comme une dérogation ciblée à la décision B, pas une remise en cause. La LISTE des
   entrées suivies reste lue en lecture seule depuis le compte (`GET /api/v1/settings`, clé
   `"watchlist"`, comme le roster) ; les COMPTEURS, eux, sont incrémentés et persistés **localement
   à l'overlay en premier lieu**, PUIS répliqués vers le compte. **Fermé le 2026-09-07** (retour
   utilisateur : un Suivi jamais visible sur le site) : la réplication des compteurs manquait
   entièrement — `overlay-sync` n'écrivait jamais sur `PATCH /api/v1/settings`, alors que ce dernier
   accepte déjà le porteur `Authorization: Bearer` sans CSRF (voir `functions/api/_auth.ts::
   requireCsrf` côté dépôt web, même exception que pour l'historique — §7.2). Fait :
   `WatchlistState::drain_pending_sync`/`Engine::drain_watchlist_sync` (miroir du motif « drain »
   de `drain_sync_events`) relaient un instantané complet des entrées suivies dès qu'un compteur
   change ou qu'un rattrapage est nécessaire (`merge_config`, compte en retard sur le local) ;
   `overlay_sync::client::patch_watchlist` envoie `PATCH /api/v1/settings` (`{ entries: [{ key:
   "watchlist", value: <liste complète>, updatedAt }] }`, « dernier écrivain gagne » côté serveur,
   voir `functions/api/v1/settings.ts::onRequestPatch`) ; le thread Sync (`overlay-ui::
   spawn_sync_thread`) porte un débounce de 1,5 s (miroir de `WRITE_DEBOUNCE_MS`,
   `RemoteUserDataRepository` côté web) puis un backoff 15 s→5 min en cas d'échec, sans file
   SQLite ni `client_key` (mécanisme volontairement distinct de `SyncQueue` — la valeur ENTIÈRE
   remplace la clé côté serveur, rien à dédupliquer, voir la doc de `spawn_sync_thread`).
4. **Signature Authenticode** Windows : budget accepté ou distribution non signée assumée en v1 ?
5. **Langue du client de jeu** : le parser actuel est FR uniquement. L'overlay hérite de cette
   limite — la documenter, ou élargir le parser côté web (qui bénéficierait aux deux) ?
6. **Notification de tour, émetteur Windows** (2026-09-14, voir §9.1 decies) : un toast WinRT sans
   `AppUserModelID` enregistré s'affiche au nom de PowerShell, et l'overlay n'a pas d'installeur
   (§11). Trois issues : l'assumer en l'état, faire poser le raccourci du menu Démarrer par
   l'overlay lui-même au premier lancement, ou ne livrer que Linux en attendant.
   **Reporté explicitement par l'utilisateur le 2026-09-14** — à reprendre avec l'installeur, dont
   ce point devient une exigence.
7. ~~**Notification de tour, capture d'une fenêtre sans focus**~~ (2026-09-14, voir §9.1 decies) :
   **tranché sous Windows par le spike S4** (`spikes/s4-capture-hors-focus/`) — `PrintWindow`
   comme Windows Graphics Capture voient un contenu vivant d'une fenêtre recouverte à 100 %, cas D
   compris (application tierce maximisée, aucune fenêtre Wakfu au premier plan) ; `PrintWindow`
   retenue pour la v1, WGC en repli documenté ; minimisée = hors d'atteinte, annoncé tel quel.
   **Reste ouvert : Linux / X11** (`XCompositeNameWindowPixmap`), même protocole, sur une machine
   Linux.

---

## 15. Journalisation (`overlay-ui`, 2026-09-02)

Objectif : que chaque exécution soit rejouable/analysable après coup (dégâts, alertes, appairage,
fenêtrage) sans dépendre d'un copier-coller du terminal — le fichier EST la source de vérité,
la console n'en est qu'un miroir.

- **Un seul système de journalisation** : `tracing` partout (`overlay-ingest`/`overlay-engine`
  l'utilisaient déjà). Avant ce lot, `overlay-ui` faisait cohabiter `tracing::info!/warn!` (sans
  aucun subscriber installé — donc **invisibles**, bug silencieux) et `env_logger` (façade `log`,
  console uniquement) : les deux sont partis, remplacés par un unique
  `tracing_subscriber::registry()` (voir `crates/overlay-ui/src/logging.rs`). Les dépendances
  externes qui utilisent encore la façade `log` (wgpu, winit) sont pontées automatiquement vers ce
  même subscriber par `tracing-subscriber` (feature `tracing-log`, activée par défaut sur `.init()`
  — aucun code de pont à écrire).
- **Deux sorties, un seul contenu** : une couche console (`fmt::layer()`, ANSI) et une couche
  fichier (`fmt::layer().with_ansi(false)`), toutes deux sous le même `EnvFilter` — ce qui apparaît
  dans le terminal est exactement ce qui est écrit sur disque (horodatage, champs structurés,
  thread, ligne compris).
- **Emplacement** : `<dossier de données de l'appli>/logs/` (même racine que
  `catalog_cache`/`token_store`/`watchlist`, résolue par `directories::ProjectDirs` — sous Windows
  `%APPDATA%\wakfu-companion-overlay\logs\`), fichier `overlay-ui.<AAAA-MM-JJ>.log`.
- **Rotation** : quotidienne (`tracing_appender::rolling::Rotation::DAILY`), 14 fichiers conservés
  (`max_log_files`) — pas de croissance illimitée sur un poste laissé tel quel.
- **Écriture synchrone** (pas de `tracing_appender::non_blocking`) : volume de lignes faible (pas
  un chemin chaud), et ça garantit qu'aucune ligne n'est perdue si le process s'arrête
  brutalement — notamment Ctrl+C dans le terminal, un des deux moyens de sortie documentés dans la
  bannière de démarrage (l'autre étant le hotkey Quitter).
- **Horodatage** : UTC ISO-8601 microseconde (`fmt::time::SystemTime`, timer par défaut de
  `tracing-subscriber`, zéro dépendance supplémentaire) — UTC plutôt qu'heure locale pour un tri
  lexical fiable et aucune ambiguïté de fuseau/heure d'été.
- **Niveau** : `info` sur le code de l'appli, `warn` sur `wgpu_hal`/`wgpu_core`/`naga` (bruyants),
  réglable sans recompiler via `RUST_LOG` (même convention que `overlay-app`).
- **Bornes de session** : chaque lancement journalise `=== session démarrée ===` (PID, version,
  hash de commit — voir §11, OS)
  en tout premier dans `main()`, et `=== session terminée ===` (même PID, + la raison) à CHAQUE
  point de sortie — fermeture de fenêtre, hotkey Quitter, Ctrl+C (`logging::install_ctrlc_handler`,
  sans quoi ce chemin de sortie n'aurait jamais de borne de fin exploitable), échec de démarrage
  (`wakfu.log` introuvable). Absence de ligne de fin avant la prochaine ligne de début = sortie
  anormale (crash).
- Le jeton de compte ne transite **jamais** dans ces logs (§10) — seuls des messages de statut
  (succès/échec d'appairage, de sauvegarde, de récupération des réglages) y apparaissent.
- `overlay-app` (harnais L1, pas l'overlay final) garde un `tracing_subscriber::fmt` console
  uniquement — pas de fichier, pas de rotation : pas l'usage visé par ce lot.

---

## 16. Persistance du combat en cours (`overlay-engine::fight_store`, 2026-09-02)

Complète `Engine::state_initialized` (§5.3) : ce champ protège déjà un combat actif d'une rotation
de `wakfu.log` survenant **pendant que l'overlay tourne**, mais rien ne protégeait une rotation
survenue **pendant que l'overlay est arrêté** — au redémarrage, `Engine::new()` repartait d'un état
vide, et le nouveau `wakfu.log` ne rejoue jamais l'historique déjà lu (même constat que §5.3) : un
combat toujours en cours en jeu réapparaissait à zéro (dégâts perdus) le temps qu'une nouvelle ligne
survienne.

- **Un fichier JSON par combat encore `ongoing`**, `fight-{fight_id}.json`, sous
  `%APPDATA%/wakfu-companion-overlay/data/` (`fight_store::default_store_dir`, même racine
  `directories::ProjectDirs` que `watchlist`/`catalog_cache`/`logs`). Contenu : `FightSnapshot` tel
  qu'affiché par l'UI (liste des combattants alliés/ennemis, dégâts/soins cumulés) — pas l'état
  interne d'attribution par siège d'initiative (`FightWorking`, propre à un process), qui repart
  neuf après restauration (écart assumé, voir la doc de `fight_store.rs` pour le raisonnement).
- **Écriture** : après chaque lot (`Engine::ingest_batch`) qui touche un combat encore en cours —
  pas ligne par ligne, un combat encaisse potentiellement des dizaines de lignes par lot.
- **Suppression** : dès que le combat se termine (`CombatEnd`) ou est purgé de la mémoire
  (`MAX_TRACKED_FIGHTS`, voir `session.rs`) — il n'y a alors plus rien à restaurer.
- **Restauration** : `Engine::new()`/`with_stores` recharge tout fichier restant au démarrage
  (forcément un combat qui n'a jamais reçu sa ligne de fin) et le réinjecte dans `SessionState`
  avant tout premier lot ingéré. `state_initialized` démarre à `true` dès qu'au moins un combat est
  restauré, pour que le tout premier lot (marqué `is_initial_load`, rattrapage ou reprise après
  rotation à froid) ne vide pas cet état comme un vrai premier lancement le ferait.
- **Nettoyage des fichiers orphelins** : un fichier de plus de 24 h (crash sans `CombatEnd`, ou
  overlay resté éteint longtemps) est supprimé sans être restauré, plutôt que de réafficher
  indéfiniment un combat quitté depuis longtemps.
- **Piège de test déjà documenté** (voir `Engine::with_watchlist_store`, `watchlist.rs`) : un test d'intégration
  (`tests/*.rs`, sans `cfg(test)` actif pour `overlay-engine`) qui appellerait `Engine::new()` ou
  `with_watchlist_store()` seul écrirait dans le VRAI dossier de combats de production dès qu'il
  laisse un combat `ongoing` en fin de test. `Engine::with_stores(watchlist_path, fight_store_dir)`
  expose les deux chemins explicitement — utilisé par `tests/session_real_log.rs` (plusieurs
  combats laissés `ongoing` en fin de test) et `tests/watchlist_boss_sans_ligne_ko.rs`.

---

## 17. Harnais de test visuel et comportemental (rendu, multi-fenêtres, interactions)

**Contrainte de départ, propre à ce projet** : le développement se fait par des sessions Claude
Code, y compris des sessions **cloud Linux headless** — sans GPU garanti, sans display interactif,
sans le jeu Wakfu installé, sans machine Windows. Sans un système dédié, tout ce qui touche au
rendu egui, à l'ancrage multi-fenêtres ou au click-through ne peut être vérifié qu'en décrivant le
code, jamais en le montrant. Ce chapitre décrit le système retenu pour lever cette limite,
**challengé par trois relectures Rust indépendantes jusqu'à accord unanime** (§17.6) : rendu
wgpu/egui, systèmes X11/fenêtrage, testing/architecture CI.

Deux niveaux de test, délibérément séparés (ils ne couvrent pas la même chose et n'ont pas la même
maturité), plus un mécanisme de restitution humaine.

### 17.1 Niveau 1 — Rendu offscreen déterministe

Objectif : visualiser tout changement des panneaux egui (`crates/overlay-ui/src/panels/*.rs`) sans
fenêtre système, sans GPU physique, exécutable dans une session Claude cloud comme dans une CI
standard.

- **Nouveau crate `crates/overlay-testkit`** (lib + tests). Jamais dans le graphe de dépendances du
  binaire livré — vérifié en CI par `cargo tree -p overlay-app` (aucune occurrence attendue).
- **Frontière à extraire de `crates/overlay-ui/src/main.rs::render()`** : une fonction pure
  `build_ui(ctx: &egui::Context, content: RenderContent<'_>) -> egui::FullOutput`, qui ne contient
  que ce qui est aujourd'hui dans la fermeture `ctx.run_ui(...)`. `render()` garde tout ce qui
  touche `egui_winit`/`Window`/`wgpu::Surface`. `RenderContent` doit devenir constructible sans
  canaux `mpsc` réels (champs `Option`, ou petit trait `AuthCommandSink` mocké en test).
  `RemoteIconStore` (icônes `wakassets` réseau, `remote_icons.rs`) est construit à la main dans le
  harnais (vide ou préchargé) — jamais alimenté par le thread réseau réel, pour ne dépendre d'aucun
  accès réseau pendant un test « offscreen ».
- **Horloge injectable, condition de non-flakiness dès le premier panneau testé** :
  `panels::watchlist::toast_card`/`is_active` lisent aujourd'hui `std::time::Instant::now()` en
  interne (fondu d'entrée du toast, chute des confettis, expiration `hide_at`) — indépendamment de
  tout ce que contient le `WatchlistToast` passé en entrée. Sans correctif, deux exécutions du même
  test à deux instants réels différents affichent une carte à une phase d'animation différente : ce
  n'est pas du bruit d'anticrénelage absorbable par un seuil de diff, c'est une différence
  structurelle de contenu. Un instant explicite (`now: std::time::Instant` propagé dans la chaîne
  d'appel, ou un `trait Clock` minimal) doit être fait transiter jusqu'à ces deux points, dans le
  **même chantier** que l'extraction de `build_ui`, pas en suivi séparé. Le champ `confetti:
  Vec<ConfettiPiece>` lui-même n'est pas concerné (déjà résolu une fois par
  `main.rs::spawn_engine_thread`, jamais régénéré au rendu) : le harnais le construit à valeurs
  fixes sans jamais rappeler `build_confetti()`.
- **Snapshot testing via `egui_kittest`** (dev-dependency, version alignée sur `egui`/`egui-wgpu`
  déjà utilisées par `overlay-ui`, feature `wgpu` + `snapshot`) plutôt qu'un harnais offscreen fait
  maison — le crate encapsule déjà la sélection d'un adaptateur logiciel, la convention
  `tests/snapshots/`, et la comparaison à seuil de tolérance ; réinventer cette mécanique serait un
  coût de maintenance pur.
- **Driver logiciel** : `mesa-vulkan-drivers` (lavapipe) comme prérequis documenté, installé par un
  script idempotent unique (modèle `patches/setup-vendor.sh`), version pinnée. Filet si Vulkan
  indisponible localement : feature `gles` ajoutée à `wgpu` **et** l'instance du testkit construite
  avec `Backends::VULKAN | Backends::GL` explicitement (pas seulement le flag Cargo) — sans quoi
  l'énumération d'adaptateurs ne retombe pas automatiquement sur llvmpipe/GL.
- **Comparaison à seuil, jamais pixel-exact** : `assert_eq!` sur des octets PNG est écarté d'emblée
  (dérive connue de rastérisation entre versions de Mesa). Image de référence versionnée dans le
  dépôt, version de Mesa figée dans l'image CI, flux explicite de mise à jour des références,
  ~~revu en PR dédiée — jamais glissé dans une PR fonctionnelle~~ → **commit dédié + capture
  publiée** (amendé le 2026-09-13, voir ci-dessous).
- **Fixtures : jamais de `UiSnapshot`/`SessionSnapshot` construit à la main.** Il doit systématiquement
  être dérivé du rejeu du vrai `crates/overlay-engine/tests/wakfu.log` à travers le vrai
  `EngineBackend` — pour rester attelé au harnais de parité existant (§2.2) et casser visiblement le
  harnais si la forme du snapshot change, plutôt que de rendre silencieusement des données obsolètes.
- **Gouvernance CI** : démarre en job **informatif** (artefact publié, non bloquant), promu en gate
  seulement après une période de rodage sans flake constaté sur les runners réels. Pour la partie
  qui doit bloquer, préférer des assertions structurelles (layout, présence/absence de panneau,
  contenu textuel) au diff pixel brut.

**Promu en GATE le 2026-09-13** (décision utilisateur explicite), avec l'environnement de rendu figé
que la règle « version de Mesa figée dans l'image CI » ci-dessus réclamait depuis le début et qui
n'avait jamais été mis en place.

- **Ce qui a forcé la décision** : l'étape informative ne rodait rien du tout. `cargo test` s'arrête
  au premier BINAIRE de test en échec, et `tests/design_gallery.rs` passe avant `tests/panels.rs`
  dans l'ordre alphabétique — deux captures périmées du premier ont caché **vingt-et-une** captures
  périmées du second pendant une semaine, sans qu'aucun journal du CI n'ait montré une seule ligne
  de `panels.rs`. Un job non bloquant qui n'exécute pas ce qu'il prétend surveiller ne rode rien ;
  `--no-fail-fast` (ajouté le même jour sur les quatre `cargo test` du workflow) est ce qui rend le
  signal complet.
- **Rendu figé, à deux verrous** : le job `test-linux` tourne dans un conteneur `ubuntu:24.04`
  épinglé **par digest** (un tag désigne une image différente à chaque point release), et
  `scripts/setup-render-env.sh` fait pointer APT vers un **instantané daté** de l'archive Ubuntu
  (`snapshot.ubuntu.com`) plutôt que vers l'archive vivante. Épingler seulement la version du paquet
  ne suffisait pas : une version supplantée finit par disparaître de l'archive, et l'installation
  échouerait en 404 quelques semaines plus tard — constaté dans ce dépôt sur `libasound2-dev` le
  jour même. Le script échoue explicitement si le Mesa obtenu n'est pas celui attendu, plutôt que de
  laisser le gate rougir ensuite sur 57 captures sans dire pourquoi.
- **Vérifié avant promotion, pas supposé** : les 57 tests passent dans cet environnement figé contre
  les références **telles qu'elles sont versionnées**, sans aucune régénération. C'était la condition
  — promouvoir d'abord et régénérer ensuite aurait rendu le gate rouge dès son premier run.
- **Mettre à jour le rendu est un geste explicite**, dans son propre commit, exactement comme monter
  la version de Rust (`rust-toolchain.toml`, même histoire) : changer l'instantané, régénérer les
  références DANS cet environnement (`.github/ci-image/Dockerfile` le reproduit sur un poste de dev),
  committer les deux ensemble.
- **Ce que la promotion coûte** : tout changement visuel voulu doit désormais porter ses références
  régénérées, sinon le CI bloque.

**Règle de régénération, amendée le 2026-09-13** (décision utilisateur) : « revu en PR dédiée »
devient **commit dédié + capture publiée**.

- **Ce que la règle protège, et qui ne change pas** : régénérer une référence est une AFFIRMATION
  — « ce nouveau rendu est correct » — et c'est la seule que git ne sait pas montrer, un PNG modifié
  s'affichant `Bin 693015 -> 698936 bytes`. Le mode de défaillance visé est précis : on change du
  code d'interface, douze captures rougissent, `UPDATE_SNAPSHOTS=1`, vert — et si l'une des douze
  était une vraie régression (libellé rogné, infobulle du mauvais côté), elle vient de devenir la
  référence, définitivement et sans que personne l'ait vue. La règle existe pour que ce geste ne
  soit jamais un réflexe.
- **Pourquoi « PR dédiée » ne s'appliquait pas** : ce dépôt n'ouvre pas de PR (voir `CLAUDE.md` —
  tout converge sur `dev` par commits directs). La règle avait été écrite pour un flux qui n'est pas
  le sien, et n'a donc jamais été tenue une seule fois.
- **Ce qui la remplace, à intention identique** : (1) un **commit dédié** ne portant que les `.png`,
  visible dans `git log` comme un acte distinct du changement de code ; (2) la **capture publiée en
  artefact**, déjà imposée par `CLAUDE.md` pour tout changement visuel — le seul support qui rende
  la bénédiction relisible, puisque le diff ne le peut pas.
- **Et le CI publie désormais de quoi la relire** : sur un gate rouge, `*.new.png` (le rendu obtenu)
  et `*.diff.png` (les pixels en écart) sont publiés en artefact. Sans eux, le journal ne donne
  qu'un nombre de pixels, et il fallait reproduire l'environnement du CI en local rien que pour
  savoir CE QUI avait changé.
**Captures hors CI — `scripts/ci-local.sh`, tranché le 2026-09-13** (décision utilisateur) : le
script tourne sur la machine du dev, qui n'a aucune raison d'avoir le rendu du CI. Trois situations,
trois comportements.

- **Environnement prouvé épinglé → étape bloquante**, comme au CI. La preuve est le marqueur
  `/etc/wakfu-render-pinned`, posé par `setup-render-env.sh` en fin de course. **Un marqueur et non
  un `dpkg-query` refait par l'appelant** : `dpkg` n'existe pas sous Arch — le conteneur de
  développement du Steam Deck (`setup-steamdeck.sh`) en est un — où l'interrogation échouerait
  silencieusement et ferait passer un environnement NON épinglé pour épinglé.
- **Environnement quelconque → étape informative**, jamais comptée en échec, et le récapitulatif
  final dit explicitement que les captures n'ont pas été vérifiées. Un verdict rendu sous un autre
  Mesa ne vaut rien, et un « tout est vert » qui rougit à tort est pire que pas de verdict : c'est
  la mécanique qui a fait que plus personne ne lisait le CI pendant la semaine rouge de septembre.
  Le cas est réel et non théorique — sur le Steam Deck, le conteneur de dev est sous Arch, en Mesa
  roulant, et sans `vulkan-swrast` il ne rendrait même pas avec le même pilote.
- **`--captures-conteneur` → on se place dans l'environnement du CI**, donc bloquante. Le script
  construit l'image de `.github/ci-image/Dockerfile` et y rejoue les captures ; c'est aussi le seul
  chemin légitime pour les RÉGÉNÉRER (`UPDATE_SNAPSHOTS=1 bash scripts/ci-local.sh
  --captures-conteneur`).

**État (2026-09-04) : Niveau 1 implémenté et validé de bout en bout, portée volontairement
réduite pour l'instant.**

- `overlay-ui` converti en crate **lib + bin** : `src/lib.rs` expose `panels`/`portraits`/
  `remote_icons`/`ui_icons`/`render_content` ; `main.rs` (le binaire, Windows-only, `windows::`
  importé sans `cfg`) consomme ces items depuis la lib au lieu de les définir localement. La LIB
  seule compile nativement sous Linux (vérifié : `cargo check -p overlay-ui --lib` sans cible
  croisée), condition nécessaire pour qu'`overlay-testkit` tourne dans une session Claude cloud.
  Dépendance système supplémentaire découverte au passage : `rodio` (son, module `alert_sound`
  resté privé au binaire) tire `alsa-sys`, qui a besoin de `libasound2-dev` installé pour compiler
  ne serait-ce que la lib — sans lien avec le rendu, mais Cargo compile toutes les dépendances du
  paquet quelle que soit la target demandée.
- `render_content::build_ui` scindée en deux : `build_ui` (fenêtrage-adjacent, reconstruit un
  `RenderContent` frais à chaque appel de sa fermeture — `ctx.run_ui` exige `FnMut`, déplacer
  l'agrégat capturé ne typerait qu'en `FnOnce`) et **`paint_content(ui, RenderContent) -> bool`**,
  qui porte toute la logique de peinture et ne dépend que d'`egui::Ui` — directement appelable par
  `egui_kittest::Harness::new_ui`, qui fournit déjà son propre `&mut egui::Ui`.
- `AuthCommandSink` (trait, implémenté par `mpsc::Sender<AuthCommand>` en prod, `NoopAuthSink` en
  test) et `RemoteIconStore::empty()` (store sans thread réseau) rendent `RenderContent`
  constructible sans aucun état de production — exactement la réserve de la revue à trois experts.
- **Crate `crates/overlay-testkit`** (dépend de la LIB `overlay-ui`, jamais de son binaire) :
  `tests/panels.rs` rejoue le vrai `crates/overlay-engine/tests/wakfu.log` via `Tailer` + `Engine`
  (même mécanique que le harnais de parité, §2.2) puis appelle `paint_content` avec le
  `SessionSnapshot` réellement obtenu — jamais un littéral fait main. `egui_kittest` (feature
  `wgpu`+`snapshot`) choisit lui-même un adaptateur logiciel ; `mesa-vulkan-drivers` (lavapipe,
  déjà nécessaire au spike S3, §17.2) est le seul prérequis système.
- **Résultat obtenu, avec preuve visuelle** : le panneau Combat rendu depuis un vrai combat trouvé
  dans le rejeu (`Anonyme-Ouginak1`, 38 733 dégâts, portraits et barres de progression réels) — image
  envoyée à l'utilisateur en session, deux tests verts (`cargo test -p overlay-testkit`), images de
  référence versionnées dans `crates/overlay-testkit/tests/snapshots/`.
- **Portée actuellement couverte** : panneau Combat sur le dernier combat encore suivi en fin de
  rejeu, panneau Suivi vide (rejeu sans configuration de watchlist), panneau Suivi avec entrées
  réelles ET toast de ramassage actif. **Mise en défaut RÉELLE constatée le 2026-09-04** (pas plus
  une hypothèse) : un snapshot régénéré et committé sur une machine (changement légitime des
  templates du cadre Combat) a fait échouer le test sur cette session, écart visuel confirmé minime
  (quelques lignes de séparation décalées de sub-pixels — rendu logiciel lavapipe, pas une
  régression), corrigé dans l'urgence en régénérant le snapshot depuis CET environnement, PUIS
  traité à la racine le même jour (voir juste en dessous). Point vérifié dans cette session :
  `cargo tree -p overlay-app | grep testkit` ne remonte rien — `overlay-testkit` n'entre jamais dans
  le graphe du binaire livré.
- **Clôturé (2026-09-04, suite)** — les trois manques listés ci-dessus au moment de l'écriture de
  ce paragraphe sont désormais traités :
  - **Seuil de tolérance explicite** : `crates/overlay-testkit/kittest.toml` (`max_failed_pixels =
    32`, `threshold` laissé à sa valeur par défaut 0.6) — valeur choisie comme la marge minimale
    qui absorbe l'écart réellement observé ci-dessus (quelques dizaines de pixels de bordure) sans
    s'approcher des centaines/milliers de pixels qu'affecterait un changement visuel réel sur une
    image 800×600 ; voir les commentaires du fichier pour le raisonnement complet et la règle de
    révision (à la hausse seulement si un futur écart légitime la dépasse, jamais préventivement).
  - **Scénario toast de ramassage + entrées watchlist réelles** :
    `panneau_suivi_avec_toast_de_ramassage_ne_panique_pas` (`tests/panels.rs`) — la LISTE des
    entrées watchlist est une configuration explicite (`Engine::set_watchlist_entries`, ce
    qu'alimenterait un compte lié via `GET /api/v1/settings`, un simple transport HTTP autour du
    même appel), mais le FRANCHISSEMENT à 0 qui construit le toast provient du vrai rejeu : « Bottes
    Lantha », ramassée 15 fois dans le vrai `wakfu.log` de test dont la première dès la ligne 537,
    configurée en cible de décompte à 1 pour un déclenchement déterministe dès le premier
    ramassage. Le `WatchlistToast` est construit avec la même logique que
    `engine_thread::spawn_engine_thread` en production.
  - **Reste, seul point non couvert** : scénario Combat ABSENT — nécessite un log de test dédié
    avec un combat encore `ongoing` à sa toute fin (le `wakfu.log` de parité existant n'a pas cet
    état), non bloquant pour clore ce lot.
  - **Gouvernance CI** : le job « informatif » (§11/§17.3) reste **non-gate** — la mise en défaut
    RÉELLE ci-dessus, aussi minime soit-elle une fois le seuil corrigé, est précisément le signal
    qu'une période de rodage sans flake constaté (§17.3) n'a pas encore commencé pour de bon ;
    repartir de zéro sur ce compteur à partir de ce commit plutôt que de considérer un incident déjà
    corrigé comme suffisant serait prématuré. **Caduc depuis le 2026-09-13** : l'étape est un gate,
    et le rodage attendu ici n'aurait de toute façon jamais eu lieu — elle n'exécutait pas
    `tests/panels.rs`. Voir « Promu en GATE » au §17.1.

### 17.2 Niveau 2 — Comportemental multi-fenêtres/click-through (X11, sous Xvfb)

**Ce n'est pas un harnais de non-régression sur du code existant** : à ce jour, aucun code
d'ancrage/multi-fenêtre/click-through X11 n'existe dans le dépôt (S3 non commencé — toute la logique
de `game_window.rs`/`main.rs` est `cfg(target_os = "windows")`). C'est donc un **spike S3
« implémentation + harnais conjoint »** : `overlay-platform::linux/x11.rs` s'écrit EN MÊME TEMPS
que son harnais Xvfb, le second servant de TDD au premier — voir le critère de sortie enrichi au
§12.

**Prérequis à valider EN PREMIER, dans cet ordre, avant tout investissement supplémentaire**
(goulots d'étranglement de faisabilité binaire, pas des nuances à traiter en cours de route) :

1. `overlay-ui` produit-il une frame sous Xvfb sans GPU réel ? → valider `mesa-vulkan-drivers`/
   lavapipe isolément, en tout premier (mêmes paquets qu'au §17.1).
2. Un **gestionnaire de fenêtres EWMH minimal** tourne-t-il dans le Xvfb (`openbox` recommandé) ?
   Xvfb seul ne fournit aucun WM : sans lui, poser `_NET_WM_STATE_ABOVE` ou lire l'ordre
   d'empilement n'a aucun sens observable — un compositeur (`picom`) n'est pas un substitut, il gère
   la composition, pas le focus/la pile de fenêtres. Extension **XTEST** activée dans le même Xvfb
   (au même rang que RENDER/COMPOSITE) : sans elle, tout le pilotage `xdotool` (clics, focus, hotkey
   simulé) échoue.
3. Le **scénario sans compositeur est le cas par défaut du harnais** (repli opaque automatique et
   silencieux à vérifier — jamais une fenêtre noire inexpliquée, cf. réserve Expert 3 du §13) ; le
   cas ARGB 32 bits + `picom --backend xrender` (le seul backend utilisable sans GPU réel) est un
   scénario secondaire, avec vérification explicite du visuel choisi (`xdpyinfo`) plutôt qu'une
   déduction depuis le rendu final.

Architecture, une fois ces prérequis validés :

- **Fenêtre factice** : `xterm -T "<Nom> - WAKFU"` piloté par `xdotool` — pas de binaire GUI ad hoc,
  effort superflu tant que la détection reste par suffixe de titre (§6.5). Le suffixe
  `" - WAKFU"` utilisé par le harnais **réutilise la constante réelle** du code de détection,
  jamais une chaîne recopiée à la main. **Scénario multi-fenêtres nommé explicitement** dans le plan
  de test (2+ `xterm` à titres différents, création/fermeture asynchrone) : c'est le cœur du
  problème métier du 2026-09-01 (une overlay par fenêtre, jamais de mélange de personnages), pas un
  cas secondaire sous-entendu.
- **Vérification programmatique, jamais seulement « regarder la capture d'écran »** :
  - hit-test/click-through : extension **Shape** via `x11rb` (`shape::get_rectangles`, kind
    `Input`) — réponse binaire instantanée, pas de délai ni de course ;
  - réception réelle du clic : la fenêtre factice journalise les `ButtonPress` reçus (présents si
    click-through actif, absents avec timeout court sinon) ;
  - focus : `xdotool getwindowfocus`/`getactivewindow` avant/après clic (vérifie l'exigence
    « pas d'`input_focus` en mode passthrough », §6.3) ;
  - topmost/stacking : `_NET_CLIENT_LIST_STACKING` (exposé par le WM EWMH) comparé avant/après le
    délai de grâce.
- **Horloge injectable côté X11 aussi** : `sync_topmost`/`sync_windows` s'appuient directement sur
  `std::time::Instant::now()` (`TOPMOST_DEMOTE_GRACE` 1,5 s, sondage 20 Hz) sans abstraction
  aujourd'hui. Extraire cette logique derrière une horloge substituable pour la couvrir par des
  tests **unitaires, synchrones, sans Xvfb**, sur le modèle de `overlay-ingest::Tailer::poll` (§12,
  L1). Réserver Xvfb à un très petit nombre de tests d'intégration tolérants en délai, qui vérifient
  que le vrai code X11 appelle bien ces primitives — pas à couvrir la matrice de timing.
- **Artefact vidéo/image (`ffmpeg -f x11grab`, `xwd`/`import`) réservé exclusivement à la
  restitution humaine** (§17.4) — jamais utilisé comme oracle de test automatique. Deux mécanismes
  distincts : les assertions protocolaires ci-dessus sont la source de vérité CI, la capture sert la
  revue humaine.
- **Cycle de vie des process orchestrés** (Xvfb, openbox, picom, xterm, ffmpeg) : un garde RAII qui
  les termine proprement même en cas de panique/échec d'assertion en cours de scénario — sinon un
  test précédent en échec laisse un display corrompu qui fait échouer le suivant sans rapport avec
  le bug réel.
- **Placement** : sous-commandes `xtask visual-check` pour l'orchestration (peu de logique Rust,
  des appels système enchaînés) — jamais `spikes/` pour cette partie durable (sémantique du dépôt :
  « harnais jetables, jamais résolus avec le workspace principal », `Cargo.toml` racine). Le spike
  S3 lui-même démarre bien dans `spikes/s3-window-linux/` (même régime que S1/S2, implémentation et
  harnais encore instables et co-écrits en TDD) — voir le critère de sortie enrichi au §12, qui
  impose la migration du code vers `crates/overlay-platform/` et du harnais vers `xtask` avant
  clôture du lot : rien ne doit rester enterré dans un dossier jetable.
- **Isolation workspace** : à trancher explicitement au moment d'écrire `xtask` — membre du
  workspace principal ou crate autonome à `Cargo.lock` séparé (même logique que `spikes/`), pour ne
  pas faire peser `x11rb`/les dépendances de pilotage sur la résolution de dépendances de
  production.
- **Gouvernance** : jamais un gate CI par défaut (`push`/`pull_request`) — strictement à la demande
  (`workflow_dispatch` ou déclenché par une session Claude). Politique de quarantaine automatique
  (désactivation + ticket) au premier flake répété dans une fenêtre glissante, jamais un correctif
  par `sleep`/retry.
- **Limite assumée et documentée explicitement** : ce dispositif couvre X11 natif, **pas XWayland**
  (pas de compositeur Wayland réel dans Xvfb) — une partie seulement du périmètre Linux annoncé
  comme supporté (§6.4). Le chemin Windows natif (DirectComposition, Win32) reste hors périmètre,
  testé manuellement en local (`spikes/s1-window-windows/preview.ps1` + capture humaine), comme
  aujourd'hui.

**État (2026-09-04, mise à jour) : critère de sortie de S3 rempli aux points 1/2/4, validé sous
Xvfb avec preuve visuelle. Points 3 (`xtask visual-check`) et 5 (`override_redirect`) restent.**

- **Point 1 du critère de sortie (migration `discovery.rs`/`topmost.rs`) : fait.** Nouvelle crate de
  production `crates/overlay-platform` (membre du workspace, `x11rb` en dépendance uniquement sous
  `cfg(not(target_os = "windows"))` — jamais tirée côté Windows) : `linux::x11` (ex-`discovery.rs`)
  et `linux::topmost` (ex-`topmost.rs`), code inchangé, seul le module hôte change. Les 7 tests
  unitaires de `topmost::decide` tournent nativement (`cargo test -p overlay-platform`, aucun Xvfb
  requis, voir sa doc).
- **Point 2 (multi-fenêtres réel côté `overlay-ui`) : fait, y compris le rendu — nouveau binaire
  `crates/overlay-ui/src/bin/overlay-ui-x11.rs`.** Plutôt que de dupliquer les 2 000+ lignes de
  `main.rs` (fortement couplées à des appels Win32 bruts — `SetWindowPos`, `WS_EX_NOACTIVATE`/
  `WS_EX_TOOLWINDOW`, `GetForegroundWindow`, non partageables), tout ce qui NE dépendait d'AUCUNE
  API Windows en a d'abord été extrait vers la LIB (`overlay-ui/src/{frame,engine_thread,
  alert_sound,logging}.rs`, voir la doc de `lib.rs`) : `frame::render` (boucle de peinture par
  frame, générique wgpu/egui), `engine_thread::spawn_engine_thread` (thread d'ingestion, aucun
  appel Windows), `alert_sound` (rodio pur), `logging`. `main.rs` consomme désormais ces mêmes
  items depuis la lib au lieu de les définir localement — vérifié sans régression (`cargo check`/
  `clippy -D warnings`/`fmt --check` propres sur la cible Windows croisée, `cargo test -p
  overlay-testkit` toujours vert). Seul `init_gpu` (choix de backend GPU, DX12/DirectComposition
  vs Vulkan/GL) reste dupliqué entre les deux binaires — pas du code partageable, le choix diffère
  fondamentalement d'un OS à l'autre (§6.2).

  Le nouveau binaire Linux (mode **invité uniquement** — pas de compte lié/synchro serveur L4-L5,
  voir sa doc de module pour le détail exact de ce qui est omis) réutilise ces modules partagés
  et parle directement à `overlay_platform::linux::{x11,topmost}` (pas via le wrapper cross-OS
  `overlay_ui::game_window`, qui ne sert qu'à `main.rs`) : ancrage identique à Windows (même
  formule, `GAME_EDGE_MARGIN_PX`/`GAME_TOP_MARGIN_PX`), topmost focus-aware via
  `topmost::decide` + `Window::set_window_level` (déjà testé unitairement), click-through via
  `Window::set_cursor_hittest` (extension Shape X11 sous le capot, déjà validée par le spike),
  une fenêtre overlay PAR fenêtre de jeu trouvée créée/détruite dynamiquement (`sync_windows`,
  même politique que `main.rs::App::sync_windows`), panneaux Combat/Suivi RÉELS (`paint_content`,
  la même fonction que Windows et qu'`overlay-testkit`, pas un panneau de diagnostic).

  **Validé sous Xvfb+openbox, programmatiquement (`xdotool`), avec preuve visuelle** — même
  méthode que le spike S3 : ingestion réelle du vrai `crates/overlay-engine/tests/wakfu.log`,
  adaptateur GPU logiciel confirmé (`llvmpipe`), DEUX fenêtres overlay créées pour la fenêtre de
  jeu factice trouvée (`xdotool search --name wakfu-companion-overlay` → 2 résultats, Combat ET
  Suivi), positions cohérentes avec la formule d'ancrage (vérifié par calcul contre la géométrie
  réelle de la xterm factice), bascule interactif/clic-traversant testée 3 fois de suite
  (`xdotool key ctrl+alt+w`) → exactement 3 lignes de log, **jamais de double-bascule** (le bug
  #3 du spike, déjà corrigé dès l'écriture de ce binaire — voir point 4). Capture d'écran envoyée
  à l'utilisateur en session : panneau Combat avec son vrai contenu (« Aucun combat pour
  l'instant », switch Alliés/Ennemis), fond opaque en repli faute de compositeur (comportement
  documenté, pas un bug). Chevauchement visuel Combat/Suivi sur cette capture : artefact de la
  PETITE taille de la xterm de test (484×316 px, bien plus petite qu'un vrai client Wakfu), pas
  un bug d'ancrage — les deux zones ne se recouvriraient pas sur une vraie fenêtre de jeu.
- **Point 3 (extraire `harness.sh`/`probe.rs` vers `xtask visual-check`) : fait.** Nouvelle
  sous-commande `cargo run --manifest-path xtask/Cargo.toml -- visual-check` — orchestre
  Xvfb+openbox+xterm factice+**le vrai `overlay-ui-x11`** (compilé au vol) en Rust (garde RAII
  `ProcessGuard` plutôt qu'un `trap` bash), assertions par le protocole X11 lui-même (`x11rb` :
  Shape pour le click-through, `_NET_CLIENT_LIST_STACKING` pour le topmost/délai de grâce),
  jamais une déduction visuelle. Toutes les assertions passent (ancrage, click-through 1→0→1 sur
  deux bascules exactement, topmost focus-aware, démotion après délai de grâce, réaffirmation
  immédiate) — capture finale envoyée à l'utilisateur en session. **Bug réel trouvé en
  l'écrivant** : `xdotool search --name` échoue systématiquement dès que le pattern contient le
  tiret cadratin `—` (U+2014) présent dans les titres de fenêtre réels — pas une histoire de
  locale du process appelant (testé explicitement en `C.UTF-8`, même échec), plutôt une limite de
  la regex POSIX étendue sous-jacente de `libxdo` face à l'UTF-8 multi-octets ; contourné en ne
  passant jamais ce caractère à `xdotool` (motif ASCII sûr côté `search`, comparaison exacte
  faite ensuite en Rust sur le titre déjà récupéré par `getwindowname`) — voir la doc de
  `find_window_by_title`. Jamais appelée par `.github/workflows/ci.yml` (gouvernance §17.2 :
  « jamais un gate CI par défaut » pour ce harnais précis), strictement à la demande.
- **Point 4 (bug `global-hotkey` double-événement) : résolu des DEUX côtés.** Déjà corrigé côté
  Windows depuis le 2026-09-02 ; le nouveau binaire Linux filtre sur `HotKeyState::Pressed` dès
  son écriture (jamais réintroduit), vérifié sous Xvfb ci-dessus (3 appuis, 3 bascules, jamais 6).
- **Point 5 (scénario `override_redirect`) : question ouverte, non retranchée.** Toujours jamais
  rencontrée (aucun test dessus) ; à rouvrir si le vrai client Wakfu s'avère un jour se comporter
  différemment d'un `xterm` géré par un WM classique.

### 17.3 Dépendances et discipline

Même discipline que celle déjà tenue pour `wgpu-hal` (patch vendored plutôt que dépendance non
maîtrisée) et pour `ureq`/`tokio` (refus d'un second modèle de concurrence sans gain mesurable,
§7.3) :

- **Dépendances système** (Xvfb, openbox, picom, `xdotool`, `ffmpeg`, `mesa-vulkan-drivers`) : un
  seul script d'installation idempotent versionné (modèle `patches/setup-vendor.sh`), versions
  documentées et pinnées — jamais d'`apt-get` ad hoc au fil d'une session.
- **Dépendances Rust nouvelles** (`egui_kittest`, `x11rb` pour les assertions du harnais, une
  éventuelle lib de diff d'image) : en `[dev-dependencies]` d'`overlay-testkit` ou du harnais
  Niveau 2 uniquement — jamais dans `overlay-ui`/`overlay-app`, chacune justifiée par une ligne
  écrite (modèle §7.3 pour `ureq` vs `tokio`).
- Aucune de ces dépendances ne remonte dans `[patch.crates-io]` du `Cargo.toml` racine — réservé au
  patch DirectComposition de production.
- **Séquencement** : la CI de base décrite au §8/§11 (build Windows+Linux, parité, budget mémoire,
  clippy) est documentée mais absente du disque à ce jour — à écrire d'abord, avant de greffer les
  Niveaux 1/2 dessus, pour disposer d'un signal de référence stable.

### 17.3 bis Un panneau ne produit aucun effet hors de l'écran

**Règle**, établie le 2026-09-09 après un incident : un panneau peint et **rend compte**, il
n'exécute jamais lui-même un effet observable hors de la fenêtre — ouvrir une page, écrire un
fichier, émettre une requête. Il remonte l'intention à l'hôte (`RenderOutcome`, `WatchlistOutcome`,
`OptionsModalAction`), et l'hôte agit.

Ce n'est pas une préférence de style. Le harnais du Niveau 1 **clique réellement** sur les boutons
(`Harness::hover_at`/`drag_at`/`drop_at`) : un panneau qui produit un effet le produit donc à chaque
`cargo test`. Le bouton « Détails » du panneau Suivi appelait `open::that(base_url())` directement —
chaque exécution de la suite ouvrait le navigateur de la personne qui la lançait sur le déploiement
dev, plusieurs fois dans la journée avant que l'utilisateur ne le signale.

Faire remonter l'intention rend les panneaux inertes **par construction** : aucun test n'a à se
souvenir de neutraliser quoi que ce soit, et un nouveau panneau ne peut pas réintroduire le problème
sans que cela se voie dans sa signature. Le seul point du binaire qui appelle `open::that` est
l'hôte (`main.rs`, `bin/overlay-ui-x11.rs`) — c'est vérifiable d'un `grep`.

### 17.4 Restitution humaine

- Publication d'un artefact (page HTML avant/après pour le Niveau 1, courte vidéo pour le Niveau 2)
  déclenchée **mécaniquement** par un diff visuel réellement détecté par le harnais — jamais à
  chaque commit, jamais au jugement libre de Claude. Granularité par unité de revue (session/tâche),
  pas par micro-commit. Exclusion explicite des changements internes sans effet observable à l'écran
  (refactor, synchro, données).
- Jamais de blob vidéo/image commité dans le dépôt — publié en artefact CI à rétention limitée ou en
  Artifact Claude.

### 17.5 Documentation

Ce chapitre est la documentation unique du système — pas de fichier séparé, pour ne pas ajouter une
deuxième source à la dérive documentaire déjà connue entre §4 et le disque réel. Toute évolution du
harnais se met à jour ici, avec la même exigence de dater et sourcer les faits que le reste du plan.

### 17.6 Revue à trois experts Rust (2026-09-03)

Système challengé par trois relectures indépendantes (rendu wgpu/egui, systèmes X11/fenêtrage,
testing/architecture CI), sur le modèle du §13, jusqu'à accord unanime — deux tours ont été
nécessaires :

- **Tour 1** : rendu → ✅ OK sous réserve (adopter `egui_kittest`, extraire `build_ui`, mocker
  `RemoteIconStore`, diff à seuil). Systèmes/X11 → ❌ PAS OK — le Niveau 2 prétendait valider un
  code X11 qui n'existe pas encore, sans WM EWMH ni validation préalable du rendu logiciel.
  Testing/CI → ✅ OK sous réserve (placement `overlay-testkit`/`xtask` plutôt que `spikes/`,
  gouvernance CI progressive, dérivation systématique du snapshot depuis un rejeu réel).
- **Tour 2** (proposition amendée : Niveau 2 reformulé en spike S3 conjoint avec prérequis validés
  en premier) : les trois passent à ✅ OK, avec deux réserves supplémentaires précises — horloge
  injectable requise dès le premier panneau testé (rendu) et contradiction à lever entre
  « rien dans `spikes/` » et « Niveau 2 = spike S3 » (testing/CI).
- **Tour 3** (critère de sortie de S3 enrichi, §12, pour garantir l'extraction du harnais hors de
  `spikes/`) : les trois confirment **✅ OK**, sans réserve bloquante restante.

Toutes les réserves des trois tours sont intégrées dans le texte des §17.1/§17.2/§17.3 ci-dessus,
pas seulement listées ici — ce paragraphe n'est qu'un journal de la délibération, pas une liste
d'actions à part.
