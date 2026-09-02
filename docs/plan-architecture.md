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
        │ MAIN THREAD — winit + egui/wgpu │   │ Thread SYNC (tokio current_thread)│
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
│   │   └── src/{client.rs,pairing.rs,token_store.rs}  # auth native (L4) fait ; queue.rs/catalog.rs/payload.rs restent à faire (L3/L5)
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
`rquickjs`, `serde`/`serde_json`, `reqwest` (rustls), `tokio` (rt current_thread),
`rusqlite` (bundled), `keyring`, `global-hotkey`, `sha2`, `arc-swap`, `crossbeam-channel`,
`directories`, `tracing`. Sur Windows : `windows` (Win32 + DirectComposition). Sur Linux : `x11rb`.

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

### 5.3 Sémantique `isInitialLoad` (parité obligatoire)

Toute (re)connexion relit le fichier **depuis le début**. Le premier lot porte `is_initial_load =
true`, ce qui déclenche côté Engine `resetSessionState()` (historique/kamas/XP reconstruits) mais
**jamais** l'incrément des compteurs persistants (watchlist). C'est le principe d'architecture n°2
du `CLAUDE.md` du dépôt web : le reproduire à l'identique, sinon les compteurs de suivi regonflent
à chaque relance de l'overlay.

### 5.4 Date et heure

Le log ne porte que `HH:MM:SS,mmm`. L'Engine reconstitue la date du jour de lecture (comportement
web actuel). Deux conséquences déjà traitées côté web, à ne pas casser :
- les **signatures** n'utilisent que l'heure brute (relire demain le même fichier reste idempotent) ;
- passage de minuit : détecter le recul de l'horloge (`HH:MM:SS` inférieur au précédent) et
  incrémenter le jour côté hôte plutôt que d'empiler les événements sur une seule date.

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
- Multi-écran/HiDPI : suivre le `scale_factor` winit ; ancrage de l'overlay par écran + décalage,
  persistés par identifiant d'écran.

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

### 7.3 File d'envoi (miroir Rust de `SyncQueueService`)

Table SQLite `sync_queue(id TEXT PRIMARY KEY, kind, payload_json, signature, queued_at, attempts,
next_attempt_at)` — `id = "{kind}:{signature}"`, donc dédoublonnage naturel, comme en IndexedDB.

Paramètres **identiques au web**, pour ne pas surprendre le serveur : lots de **50**, debounce
**2 s**, backoff **15 s → 5 min** (doublement), abandon d'une entrée après **10 tentatives non
réseau**, payload ≤ 1 Mio.

Propriétés à tenir : jamais bloquant pour l'UI (l'enfilage est une écriture mémoire, la persistance
et le hachage viennent après), survit à un crash / une coupure réseau / un redémarrage, et rejeu
sans conséquence grâce à l'idempotence.

**Mode invité par défaut** : sans compte connecté, la file n'est pas active et **rien ne quitte la
machine**. C'est la définition du mode invité côté web ; l'overlay ne doit pas l'affaiblir.

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
| **Alertes de drop** | toast + son quand un objet suivi tombe | Son configurable par objet (parité web) ; toast ≤ 5 s, non bloquant, jamais interactif |
| **Récap de session** | kamas (combat / ventes HDV / échanges), XP, combats gagnés/perdus | Compact, toujours visible |
| **État de synchro** | `idle`/`pending`/`syncing`/`error`, nombre en attente, dernière synchro | Discret ; l'erreur réseau ne doit jamais masquer le jeu |

Raccourcis globaux : bascule interactif/traversable, afficher/masquer, panneau suivant.
Chaque panneau est déplaçable, redimensionnable, avec opacité réglable ; disposition persistée par
écran. Thème sombre par défaut, **mode daltonien** repris du web.

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
| **S3 — Spike X11** (2 j) | Équivalent S1 sous X11 + XWayland | **Reporté** (aucune machine Linux disponible pour l'instant) — S1/S2 suffisent à valider la stack sur Windows ; à faire avant tout travail spécifique Linux |
| **L1 — Ingestion** ✅ fait | tail, rotation, découverte de chemin, `isInitialLoad` | Voir `crates/overlay-ingest/` : rejeu, ligne partielle, troncature et rotation (suppression + recréation) couverts par des tests synchrones sur `Tailer::poll` ; watcher temps réel (`notify` + repli) vérifié séparément (`tests/watcher_smoke.rs`, manuel) |
| **L2 — UI** 🟡 en cours | dégâts, suivi, alertes, récap, disposition persistée | Utilisable en jeu une soirée sans redémarrage — **fait** : `crates/overlay-engine/` (QuickJS + `LogParser` vendu → `LogEntry` → `SessionSnapshot`, + `watchlist.rs` — comptage du Suivi et alertes de décompte portés en Rust, voir §14 point 3) et `crates/overlay-ui/` (fenêtre S1, deux fenêtres overlay indépendantes Combat/Suivi — bande de tuiles, alertes son+toast sur décompte à 0), validés sur un vrai `wakfu.log`. **Reste** : Alertes de drop version « ramassage avec son activé » (dépend des réglages `profile` du compte, pas encore lus), État de synchro, disposition persistée par écran, thème configurable |
| **L3 — Catalogue** 🟡 en cours | fetch, cache, repli embarqué, index O(1) | Résolution d'objet identique au web sur les golden files — **fait (2026-09-02, retour utilisateur)** : `overlay_engine::catalog` (index O(1) par id/nom, depuis `GET /api/v1/catalog/`) + `overlay-sync` (fetch + cache disque `catalog_cache.rs`, offline-first) + `overlay-ui::remote_icons` (résolution/téléchargement/cache d'icônes réelles `wakassets` pour le panneau Suivi, vérifié en direct contre le déploiement dev). **Fait (2026-09-02, suite)** : `catalog::find_item_has_recipe` (drapeau recette) et `catalog::find_monster_classification`/`find_monster_family_id` (boss/archimonstre/dominant, priorité `MonsterClassification`, miroir de `resolveFightTypeClassification`) ; `dungeon.rs`/`monster_family.rs` — deux nouveaux index O(1) (par id, + réciproque boss→donjon) construits depuis `GET /api/v1/dungeons`/`GET /api/v1/monster-families` (`overlay_sync::client::fetch_dungeons`/`fetch_monster_families`, cache disque `reference_data_cache.rs`, sans endpoint `/version` dédié côté serveur donc toujours rechargés en tâche de fond) ; repli hors-ligne embarqué (`overlay_sync::catalog_cache::embedded_fallback`, `assets/catalog/catalog-index.json.gz` via `include_bytes!`, décompression `flate2`) branché dans `spawn_catalog_thread` (utilisé seulement si aucun cache disque ET réseau injoignable) ; golden files de non-régression (`crates/overlay-engine/tests/golden/*.json` + `tests/catalog_golden.rs`, cohérence croisée catalogue/donjons/familles). ⚠️ L'asset embarqué est un PLACEHOLDER réduit (2 entrées) — ce sandbox de dev n'a jamais eu accès à Neon/`*.pages.dev` (même limite que tous les lots serveur précédents, voir `server/README.md` côté web) : **à régénérer depuis un vrai déploiement avant release** via `cargo run -p overlay-sync --bin gen-catalog-fallback`. Vérifié dans ce sandbox : `cargo test`/`cargo clippy -D warnings`/`cargo fmt --check` propres sur `overlay-engine`/`overlay-sync` ; `overlay-ui` (qui consommerait `dungeon`/`monster_family`, pas encore fait — voir Reste) reste non buildable ici même en `cargo check` — code Windows-only (`windows` crate, `GameWindowInfo.hwnd`) sans repli Linux tant que S3 est reporté, limite préexistante sans rapport avec ce lot. **Reste** : brancher `DungeonIndex`/`MonsterFamilyIndex` dans `overlay-ui` (aucun panneau §9 n'en a encore besoin — `LogEntry` n'a d'ailleurs pas de `dungeonId` par combat, voir `model.rs` — prévu pour un futur panneau Combat conscient du donjon) ; régénérer le repli embarqué depuis un vrai déploiement ; bandeau UI « catalogue daté » (actuellement un simple `tracing::warn!`) |
| **L4 — Auth native** 🟡 en cours | endpoints d'appairage (dépôt web) + trousseau | Connexion Discord/Google depuis l'overlay, session révocable — **fait** : 3 endpoints serveur (`/api/v1/auth/native/{pair,claim,poll}`, table `native_pairings`, `Authorization: Bearer` accepté par `_auth.ts`), page web `/pair`, crate `overlay-sync` (pairing bloquant + `keyring`/repli fichier + `GET /settings`), roster appliqué à `overlay-engine::session` (priorité sur `breed`), portraits de classe affichés dans le panneau Combat (`overlay-ui`). **Reste** : UI de pairing dans la fenêtre overlay (console-only pour l'instant), révocation/déconnexion côté overlay, vérification bout en bout contre un vrai déploiement (non joignable depuis un sandbox de dev, voir `crates/overlay-sync/README.md`) |
| **L5 — Synchro** | file SQLite, lots, backoff, idempotence | Rejeu 10× du même log ⇒ **aucun** doublon en base, y compris en alternant web et overlay |
| **L6 — Packaging** | AppImage, installeur, mise à jour signée | Installation propre sur une machine vierge Windows et Linux |

S1/S2/S3 étaient prévus **bloquants** (ils peuvent remettre en cause la stack). **Décision du
mainteneur (2026-08-31) : S3 est reporté** faute de machine Linux disponible pour l'instant — S1 et
S2 ont validé la stack sur Windows, suffisant pour démarrer L1. S3 reste à faire **avant** tout
travail spécifique à `overlay-platform::linux` (§6.4) ou toute distribution Linux (§11) : le risque
qu'il couvre (Wayland/X11, XWayland avec le vrai client Wakfu) ne disparaît pas, il est seulement
découplé du reste de la feuille de route.

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
