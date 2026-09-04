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
4. `npm run build:engine` → `dist/engine/wakfu-engine.mjs`, publié comme asset de GitHub Release.

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
│   ├── overlay-ui/                # egui : panneaux, thème, i18n
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

Détail complet (bugs, découvertes, repro isolé, capture d'écran de validation) dans
`spikes/s1-window-windows/README.md` — à relire avant d'implémenter `overlay-platform::windows`,
ce document ne le répète pas.

### 6.3 Click-through et retour de la souris

`window.set_cursor_hittest(false)` rend la fenêtre traversable — mais elle ne reçoit alors **plus
aucun événement souris**, donc elle ne peut pas savoir que le curseur la survole. Deux mécanismes,
volontairement redondants :

1. **Hotkey global (mécanisme principal)** — `global-hotkey`, par défaut `Ctrl+Alt+W` : bascule
   interactif/traversable. Déterministe, fonctionne partout, aucun sondage.
2. **Survol (confort)** — quand l'overlay est visible, sondage de la position curseur à 60 Hz sur le
   main thread (`GetCursorPos` / `XQueryPointer`), comparaison aux rectangles interactifs du dernier
   snapshot, et bascule du hit-test. Coût mesuré négligeable ; désactivable dans la config.

En mode interactif, l'overlay ne prend jamais le focus clavier tant qu'un champ de saisie n'est pas
explicitement cliqué (`WS_EX_NOACTIVATE` côté Windows, `_NET_WM_STATE_ABOVE` + pas de
`input_focus` côté X11) — sinon le jeu perd ses raccourcis.

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

### 7.4 Catalogue

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
| **Récap de session** | kamas (combat / ventes HDV / échanges), XP, combats gagnés/perdus | Compact, toujours visible |
| **État de synchro** | `idle`/`pending`/`syncing`/`error`, nombre en attente, dernière synchro | Discret ; l'erreur réseau ne doit jamais masquer le jeu |

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
- Mise à jour : vérification `GET` de la dernière Release au démarrage, téléchargement en tâche de
  fond, application au prochain lancement. Le bundle moteur peut être mis à jour **sans** nouvelle
  version du binaire (asset versionné + signature) — c'est ce qui permet de suivre une correction de
  parsing du dépôt web sans republier l'overlay.
- CI GitHub Actions : build Windows + Linux, tests de parité, budget mémoire, lint `clippy -D warnings`.

---

## 12. Feuille de route

| Lot | Contenu | Critère de sortie |
| --- | --- | --- |
| **S1 — Spike rendu Windows** ✅ fait | Fenêtre transparente + always-on-top + click-through + DirectComposition + wgpu | Voir `spikes/s1-window-windows/README.md` : panneau egui semi-transparent confirmé par capture d'écran par-dessus une autre fenêtre, hotkey global de bascule confirmé sans focus, RSS ~87 Mo — **verdict : chemin DirectComposition via `wgpu-hal` (`DxgiFromVisual`) validé**, plus simple que prévu (§6.2 mis à jour) |
| **S2 — Spike moteur** ✅ fait | Bundle headless TS + QuickJS, ingestion de `tests/wakfu.log` | Voir `spikes/s2-engine-quickjs/README.md` : correction confirmée (rejeu identique), débit ~28 000 l/s (sous la cible initiale de ~40 000 l/s, ×1,4), critère de fluidité UI reformulé en §5.5 — **verdict : choix QuickJS maintenu** |
| **S3 — Spike X11** 🟡 en cours | Équivalent S1 sous X11 + XWayland, **développé conjointement avec son harnais de test Xvfb** (voir §17.2) — implémentation et harnais en TDD, pas l'un après l'autre | **Spike validé (2026-09-03)**, critère de sortie PAS ENCORE atteint. Voir `spikes/s3-window-linux/README.md` pour le détail complet : les trois prérequis de faisabilité (§17.2) vérifiés dans l'ordre — rendu logiciel Vulkan/lavapipe sous Xvfb confirmé avec du vrai code `wgpu` (pas seulement `vulkaninfo`), WM EWMH (`openbox`) nécessaire et suffisant, scénario sans compositeur (repli opaque) validé comme cas par défaut et scénario avec compositeur (`picom --backend xrender`, transparence ARGB32 réelle) en secondaire. Ancrage dynamique par titre, click-through (extension Shape) et focus-aware topmost (délai de grâce 1,5 s, horloge injectable dès l'écriture — `src/topmost.rs`, 7 tests unitaires sans Xvfb) validés **programmatiquement** par `harness.sh` (jamais une capture d'écran comme oracle), reproductible en une commande, guard de nettoyage systématique. Six bugs réels trouvés et corrigés en cours de route (dépendance système manquante, panique `TexturesDelta` en debug, double événement `global-hotkey` sous X11, piège de titre `xterm` dynamique, piège regex `xdotool search`, `[workspace]` manquant préexistant sur S1/S2 aussi) — voir le README pour le détail de chacun. **Critère de sortie enrichi (2026-09-03, revue à 3 experts, §17.6) — rempli aux points 1/2/4 le 2026-09-04, voir §17.2 « État »** : le CODE (`discovery.rs`/`topmost.rs`) **migré** vers `crates/overlay-platform/src/linux/` (7 tests unitaires topmost verts) ; le multi-fenêtres réel côté `overlay-ui` **câblé** — nouveau binaire `crates/overlay-ui/src/bin/overlay-ui-x11.rs` (mode invité), validé sous Xvfb avec preuve visuelle (2 fenêtres Combat/Suivi réelles, ancrage/topmost/click-through/hotkey tous vérifiés programmatiquement) ; le HARNAIS (`harness.sh`/`probe.rs`) **reste** à extraire vers `xtask visual-check` — plus justifié par « rien à piloter » maintenant qu'un vrai binaire existe, simplement pas encore fait |
| **L7 — Outillage de test visuel (Niveau 1)** | Rendu offscreen déterministe des panneaux `overlay-ui` (crate `overlay-testkit`), voir §17.1 | Un changement de panneau (combat/watchlist) produit un diff visuel détecté automatiquement, sans Xvfb ni GPU physique, exécutable dans une session Claude cloud headless — critère détaillé en §17.6 |
| **L1 — Ingestion** ✅ fait | tail, rotation, découverte de chemin, `isInitialLoad` | Voir `crates/overlay-ingest/` : rejeu, ligne partielle, troncature et rotation (suppression + recréation) couverts par des tests synchrones sur `Tailer::poll` ; watcher temps réel (`notify` + repli) vérifié séparément (`tests/watcher_smoke.rs`, manuel). Réserve trouvée puis corrigée le 2026-09-04 (§5.2) : la détection de rotation combine désormais l'identité de fichier ET un second signal (préfixe de contenu), qui ne dépend pas de l'hypothèse fausse « inode jamais réutilisé ». |
| **L2 — UI** 🟡 en cours | dégâts, suivi, alertes, récap | Utilisable en jeu une soirée sans redémarrage — **fait** : `crates/overlay-engine/` (QuickJS + `LogParser` vendu → `LogEntry` → `SessionSnapshot`, + `watchlist.rs` — comptage du Suivi et alertes de décompte portés en Rust, voir §14 point 3) et `crates/overlay-ui/` (fenêtre S1, deux fenêtres overlay indépendantes Combat/Suivi — bande de tuiles, alertes son+toast sur décompte à 0), validés sur un vrai `wakfu.log`. **Fait (2026-09-02, suite)** : Alertes de drop version « ramassage avec son activé » — `overlay_engine::profile` lit `data.profile.soundItems` (`GET /api/v1/settings`), indépendant de la watchlist ; `Engine::drain_loot_alerts` déclenche toast + son (`alert_sound::play_loot_alert`, fichier mp3 identique au web) pour tout objet ramassé dont le son est activé au compte (objets par défaut `DEFAULT_SOUND_ITEM_NAMES` ou ajoutés par l'utilisateur, mêmes règles), suivi ou non — miroir de `registerLoot`/`ProfileService.findEnabledSoundItem`. **Fait (2026-09-02, refonte visuelle)** : le toast (`panels::watchlist::toast_card`) reproduit la carte du dépôt web (`loot-alert.component`) — icône réelle, titre/bordure `--accent`, nom (+ quantité), confettis tombants (dispersion tirée une fois par déclenchement, animée en continu tant que le toast est affiché), fermeture au clic sur la carte OU sur une croix EN PLUS de la minuterie fixe (les deux cohabitent, contrairement au réglage exclusif `ProfileService.alertManualClose` côté web, pas encore porté) ; toast affiché même watchlist vide (ramassage à son activé indépendant de la watchlist) ; fenêtre Suivi élargie/agrandie dynamiquement le temps qu'un toast est affiché (`watchlist_target_width`/`_height`), comme pour le nombre d'entrées. **Reste** : État de synchro (dépend de L5). **Retiré (2026-09-02, décision du mainteneur, voir §9)** : thème configurable/mode daltonien ; disposition persistée par écran (poignée de glissement, `layout_store`, un temps implémentée puis retirée pour la même raison — un overlay n'est pas un site, pas de personnalisation de disposition) — palette fixe et ancrage automatique seul assumés |
| **L3 — Catalogue** ✅ fait | fetch, cache, repli embarqué, index O(1) | Résolution d'objet identique au web sur les golden files — **fait (2026-09-02, retour utilisateur)** : `overlay_engine::catalog` (index O(1) par id/nom, depuis `GET /api/v1/catalog/`) + `overlay-sync` (fetch + cache disque `catalog_cache.rs`, offline-first) + `overlay-ui::remote_icons` (résolution/téléchargement/cache d'icônes réelles `wakassets` pour le panneau Suivi, vérifié en direct contre le déploiement dev). **Fait (2026-09-02, suite)** : `catalog::find_item_has_recipe` (drapeau recette) et `catalog::find_monster_classification`/`find_monster_family_id` (boss/archimonstre/dominant, priorité `MonsterClassification`, miroir de `resolveFightTypeClassification`) ; `dungeon.rs`/`monster_family.rs` — deux nouveaux index O(1) (par id, + réciproque boss→donjon) construits depuis `GET /api/v1/dungeons`/`GET /api/v1/monster-families` (`overlay_sync::client::fetch_dungeons`/`fetch_monster_families`, cache disque `reference_data_cache.rs`, sans endpoint `/version` dédié côté serveur donc toujours rechargés en tâche de fond) ; repli hors-ligne embarqué (`overlay_sync::catalog_cache::embedded_fallback`, `assets/catalog/catalog-index.json.gz` via `include_bytes!`, décompression `flate2`) branché dans `spawn_catalog_thread` (utilisé seulement si aucun cache disque ET réseau injoignable) ; golden files de non-régression (`crates/overlay-engine/tests/golden/*.json` + `tests/catalog_golden.rs`, cohérence croisée catalogue/donjons/familles). **Clôturé (2026-09-02, poste de dev avec accès réseau réel)** : les lots précédents avaient été développés dans un sandbox sans accès à Neon/`*.pages.dev` ni à `overlay-ui` (Windows-only, non buildable là-bas) — ce n'est plus le cas ici, les trois points bloquants ont donc été levés pour de vrai plutôt que redocumentés comme limite : (1) `claude-dev.wakfu-companion.com` confirmé joignable (`catalog/`, `catalog/version`, `dungeons`, `monster-families` en 200) ; (2) repli embarqué **régénéré depuis ce déploiement réel** via `cargo run -p overlay-sync --bin gen-catalog-fallback` — catalogue complet (~1,8 Mo bruts / ~489 Ko gzip), n'est plus un placeholder ; (3) petit indicateur « 📦⚠ catalogue daté » ajouté dans la zone Combat de `overlay-ui` (`catalog_stale: Arc<AtomicBool>`, posé par `spawn_catalog_thread` uniquement quand le repli embarqué est utilisé, tooltip explicatif) — remplace le `tracing::warn!` jusque-là invisible en jeu. `cargo build`/`test`/`clippy -D warnings`/`fmt --check` **propres sur les 5 crates du workspace, `overlay-ui` compris** (précédemment non vérifiable en sandbox). **Volontairement reporté, pas un blocage de clôture** : brancher `DungeonIndex`/`MonsterFamilyIndex` dans `overlay-ui` — aucun panneau §9 n'en a besoin aujourd'hui (`LogEntry` n'a pas de `dungeonId` par combat, voir `model.rs`), prévu pour un futur panneau Combat conscient du donjon, pas une régression de ce lot. |
| **L4 — Auth native** ✅ fait | endpoints d'appairage (dépôt web) + trousseau | Connexion Discord/Google depuis l'overlay, session révocable — **fait** : 3 endpoints serveur (`/api/v1/auth/native/{pair,claim,poll}`, table `native_pairings`, `Authorization: Bearer` accepté par `_auth.ts`), page web `/pair`, crate `overlay-sync` (pairing bloquant + `keyring`/repli fichier + `GET /settings`), roster appliqué à `overlay-engine::session` (priorité sur `breed`), portraits de classe affichés dans le panneau Combat (`overlay-ui`). **Fait (2026-09-02, suite)** : révocation/déconnexion — raccourci global `Ctrl+Alt+D` (`App::disconnect_account`), commande `AuthCommand::Disconnect` traitée par le thread Auth (`spawn_auth_thread`, restructuré pour rester vivant après une connexion réussie plutôt que de se terminer, condition requise pour pouvoir déconnecter PUIS reconnecter sans redémarrer l'overlay) — efface le jeton (trousseau + repli fichier) et notifie le thread Engine (`EngineCommand::Disconnect`) qui repasse en mode invité (roster `None` → repli `breed`, Suivi vidé ; compteurs locaux déjà persistés conservés pour une reconnexion ultérieure). Vérification bout en bout **partiellement levée** (poste de dev avec accès réseau réel, comme pour L3) : `claude-dev.wakfu-companion.com` confirmé joignable sur les trois routes d'appairage natif (`pair` → 200 avec code+URL réels, `poll` → `pending`/`expired` conformes au format attendu par `pairing.rs`, `settings` sans/avec jeton invalide → 401 comme attendu) ; la complétion réelle d'un appairage (connexion Discord/Google dans un navigateur) reste non automatisable depuis ici et n'a donc pas été rejouée. **Clôturé (2026-09-02, suite — UI de pairing)** : le code d'appairage n'était visible qu'en console — nouvel état `AuthStatus::PairingStarted { pairing_code, verification_url }` (publié par `attempt_connect` dès que le code est obtenu, avant le premier sondage), affiché directement dans la fenêtre overlay (zone Combat) sous forme d'une carte compacte — code en grand (lisible/tapable), bouton copier (`egui::Context::copy_text`), bouton pour rouvrir la page de vérification (`open::that`, utile si l'ouverture automatique du navigateur a échoué ou si l'onglet a été fermé par erreur). **Reste** : rien — la complétion réelle d'un appairage (connexion Discord/Google) reste non automatisable depuis ce sandbox (limite d'environnement, pas une lacune du code), comme documenté ci-dessus |
| **L5 — Synchro** ✅ fait | file SQLite, lots, backoff, idempotence | Rejeu 10× du même log ⇒ **aucun** doublon en base, y compris en alternant web et overlay — **fait (2026-09-02)** : `overlay_engine::history` (signatures + payloads fight/purchase/trade) + `overlay_engine::log_time` (dates réelles depuis `LogDateAnchor`) + `overlay_sync::queue::SyncQueue` (file SQLite idempotente, lots de 50, backoff 15s→5min, abandon après 10 tentatives non réseau — testé par rejeu 10x, critère de sortie ci-contre) + câblage `overlay-ui` (thread Sync dédié, activé/désactivé avec le compte). **Complété (2026-09-02, suite, depuis le dépôt web local)** : ventilation par sort/élément, xpGained par participant ET total du combat, résolution monsterId/itemId par catalogue, gameServer (déduit du roster), récupération de kamas HDV sans achat adjacent. **Clôturé (2026-09-02, suite — demande explicite du mainteneur de lever les deux limites restantes)** : regroupement de donjon multi-salles complet (`overlay_engine::dungeon_run`, port direct de `dungeon-run-grouping.util.ts` + `findDungeonForEnemies` 3 priorités, siblings renvoyés avec le rattachement une fois le run complété — voir §7.3) ET branchement réel `overlay-ui` (`spawn_dungeon_thread`, `GET /api/v1/dungeons` → `Engine::set_dungeons`) : `dungeonId`/`dungeonRunKey` sont désormais alimentés en production, plus seulement prêts côté moteur. **Reste** : rien — voir §7.3 pour le seul renoncement volontaire restant (`turns`, jamais utilisé) |
| **L6 — Packaging** | AppImage, installeur, mise à jour signée | Installation propre sur une machine vierge Windows et Linux |

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
   `"watchlist"`, comme le roster) ; les COMPTEURS, eux, sont **locaux à l'overlay** (pas de
   `PATCH /api/v1/settings` pour l'instant — synchroniser les compteurs eux-mêmes reste un
   chantier futur, plus proche de L5).
4. **Signature Authenticode** Windows : budget accepté ou distribution non signée assumée en v1 ?
5. **Langue du client de jeu** : le parser actuel est FR uniquement. L'overlay hérite de cette
   limite — la documenter, ou élargir le parser côté web (qui bénéficierait aux deux) ?

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
- **Bornes de session** : chaque lancement journalise `=== session démarrée ===` (PID, version, OS)
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
  revu en PR dédiée — jamais glissé dans une PR fonctionnelle.
- **Fixtures : jamais de `UiSnapshot`/`SessionSnapshot` construit à la main.** Il doit systématiquement
  être dérivé du rejeu du vrai `crates/overlay-engine/tests/wakfu.log` à travers le vrai
  `EngineBackend` — pour rester attelé au harnais de parité existant (§2.2) et casser visiblement le
  harnais si la forme du snapshot change, plutôt que de rendre silencieusement des données obsolètes.
- **Gouvernance CI** : démarre en job **informatif** (artefact publié, non bloquant), promu en gate
  seulement après une période de rodage sans flake constaté sur les runners réels. Pour la partie
  qui doit bloquer, préférer des assertions structurelles (layout, présence/absence de panneau,
  contenu textuel) au diff pixel brut.

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
  dans le rejeu (`Erz-Wouaf`, 38 733 dégâts, portraits et barres de progression réels) — image
  envoyée à l'utilisateur en session, deux tests verts (`cargo test -p overlay-testkit`), images de
  référence versionnées dans `crates/overlay-testkit/tests/snapshots/`.
- **Portée actuellement couverte, volontairement limitée** : seuls les états qu'un rejeu simple
  (sans compte lié) produit réellement — panneau Combat sur le dernier combat encore suivi en fin
  de rejeu, panneau Suivi vide. **Reste, avant de considérer ce lot clos** : scénario avec toast de
  ramassage actif (nécessite l'horloge injectable de `panels::watchlist`, déjà en place depuis le
  chantier précédent, mais pas encore exercée par un test testkit), entrées watchlist réelles
  (nécessite un compte lié ou des réglages de test), diff à seuil de tolérance explicite plutôt que
  la comparaison stricte par défaut d'`egui_kittest` (jamais mise en défaut jusqu'ici — pas encore
  éprouvée sur un changement mineur de version Mesa), gouvernance CI (le job « informatif » lui-même
  n'existe pas encore, voir §17.3 sur l'absence de CI de base). Point vérifié dans cette session :
  `cargo tree -p overlay-app | grep testkit` ne remonte rien — `overlay-testkit` n'entre jamais dans
  le graphe du binaire livré.

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
- **Point 3 (extraire `harness.sh`/`probe.rs` vers `xtask visual-check`) : reste à faire.** Un vrai
  binaire existe désormais (voir point 2) — ce report n'a plus la même justification qu'avant
  (« rien à piloter »), mais n'a pas encore été fait dans cette session.
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
