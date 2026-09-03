# `overlay-engine` — L2 : frontière métier

`LineBatch` (overlay-ingest) → `LogEntry` typés (QuickJS + `LogParser` vendu depuis
`wakfu-companion`) → `SessionSnapshot` (agrégation Rust) — voir
[`docs/plan-architecture.md`](../../docs/plan-architecture.md) §2, §9, §12.

## Contenu

| Module | Rôle |
| --- | --- |
| `engine-js/` | `LogParser` + `LogEntry` **vendus tels quels** depuis `Oumbra/wakfu-companion` (voir `engine-js/VENDORED_FROM.txt` — même commit que le spike S2), bundlés en IIFE ES2020 (`esbuild`, `dist/engine.bundle.js` **committé**, ~30 Ko : pas besoin de Node.js pour `cargo build`, seulement pour retoucher le TS vendu). |
| `quickjs_engine.rs` | `LogParserEngine` : charge le bundle dans `rquickjs`, expose `parse_lines()`/`reset()`/`set_catalog()`. Aucune logique métier ici, seulement le pont QuickJS↔Rust — y compris dans l'autre sens : `hostIsKnownMonsterName`, une fonction NATIVE posée sur les globals AVANT évaluation du bundle, lue par `entry.ts` (voir plus bas). |
| `model.rs` | Miroir Rust exact (`serde`) de `engine-js/src/log-entry.model.ts` — la forme de chaque `LogEntry`, jamais sa sémantique (qui reste dans le TS vendu). |
| `session.rs` | `Engine` (gère la sémantique `is_initial_load`/reset, §5.3) + `SessionState` : agrégation **en Rust**, pas vendue — dégâts par combattant du combat en cours, récap de session (kamas, XP, combats, butin), nombre de tours (`turn_count`, miroir de `registerFightTurn`). Volontairement minimal, voir le commentaire de module pour ce qui manque (`StatsStoreService`, §14 point 3). Inclut un portage complet de `InitiativeSeat`/`resolveNextActor` (`stats-store.service.ts`) pour distinguer plusieurs combattants qui partagent EXACTEMENT le même nom (pack du même monstre) — voir plus bas. |
| `dungeon_run.rs` | Port complet de `findDungeonForEnemies`/`groupDungeonRuns` (`fight-image.util.ts`/`dungeon-run-grouping.util.ts`) : résolution du donjon d'un combat (boss simple, brèche, brèche ultime) et regroupement des salles d'un run multi-combats — câblé via `session::SessionState::resolve_dungeon_assignment`, mais borné à l'historique de la session locale en cours (comme côté web) : renforcé côté serveur pour le cas cross-session/cross-client, voir `history.rs::FightPayload::dungeon_id`. |
| `class_breed.rs` | Port de `wakfu-class-breed-ids.data.ts` (breed → classe, uniquement déterministe pour un allié confirmé) + `class-portraits.data.ts::CLASS_PORTRAIT_ORDER` (ligne d'une classe dans la planche de portraits, utilisée par `overlay-ui`). |
| `roster.rs` | `RosterIndex` : lecture seule du roster de personnages déclaré par l'utilisateur (`GET /api/v1/settings`, clé `roster` — voir `overlay-sync`), lookup O(1) par nom normalisé (`normalize_wakfu_name`, port de `wakfu-name.util.ts`). Priorité sur `breed` dans `session.rs::resolve_ally_class`, comme `EntityClassifierService.getDetectedClass` côté web. |
| `watchlist.rs` | `WatchlistState` : comptage du Suivi (§9 du plan) **porté en Rust plutôt que vendu** (§14 point 3 — logique isolée, sans rapport avec les heuristiques kamas/HDV qui motivent le choix B du §2) — définitions lues en lecture seule depuis le compte, compteurs incrémentés et persistés localement. Détecte aussi les alertes de décompte à 0 (`WatchlistAlert`, drainées par `Engine::drain_watchlist_alerts`). |
| `catalog.rs` | `CatalogIndex` : lot L3 réduit au strict nécessaire pour résoudre l'icône réelle (`IconRef`) d'un objet/monstre suivi — lookup O(1) par id (prioritaire) ou nom normalisé, depuis l'index compact `GET /api/v1/catalog/` (tuples positionnels, voir `overlay-sync`). Pas encore le reste du lot (recettes, familles de monstres, donjons, repli hors-ligne embarqué). |

## Pourquoi vendre `LogParser` plutôt que le réimplémenter

Décision structurante n°1 du plan (§2) : une seule implémentation de la logique métier au monde,
celle du dépôt web. Validée par le spike S2 (`spikes/s2-engine-quickjs/`, ~28 000 lignes/s) avant
d'en faire un vrai crate ici.

## Tests

```
cargo test -p overlay-engine
```

Sur le **vrai** `wakfu.log` vendu depuis `wakfu-companion` (`tests/wakfu.log`, 10 975 lignes, même
fichier que S2) — pas un fichier synthétique : vérifie un récap plausible (combats, XP, dégâts non
nuls), l'invariant « un rattrapage découpé en plusieurs `LineBatch` (à cause de
`MAX_BATCH_LINES`) donne le même résultat qu'un lot unique », et qu'un lot en direct après
rattrapage ne réinitialise pas l'état.

## Combattants homonymes dans un même combat (`InitiativeSeat`)

Retour utilisateur 2026-09-02 : « il n'y a que quatre monstres qui sont toujours affichés, pas
plus » sur des combats en affichant visiblement plus — cause racine : le log ne relie JAMAIS une
ligne de dégâts/soin à un `fighterId` précis (seule la ligne de jointure `[_FL_]` le porte), et
Wakfu autorise plusieurs ennemis du même pack à partager EXACTEMENT le même nom affiché. La version
initiale de `upsert_fighter` dédupliquait par nom exact, fusionnant silencieusement ces instances.

Corrigé par un portage complet de l'heuristique du web (`InitiativeSeat`, `resolveNextActor`,
`registerFightTurn` — `stats-store.service.ts`) : chaque combat suit une file de « sièges »
d'initiative, mise à jour à chaque ligne « X lance le sort Y » (`SpellCast`) — la ligne suivante de
dégâts/soin est attribuée au dernier siège résolu pour ce nom. Adapté en Rust avec une différence
assumée : à la place du web (qui perd silencieusement les dégâts d'un nom ambigu sans `SpellCast`
préalable), le repli ici retombe sur la première instance jointe de ce nom — jamais de perte
silencieuse de dégâts. Ambiguïté résiduelle documentée et non résolue (comme côté web) : deux
instances homonymes dont les tours se suivent sans aucun autre acteur entre les deux fusionnent sur
le même siège.

## Invocations exclues du récap (`summonedBy`)

Retour utilisateur 2026-09-02 (captures d'écran à l'appui) : l'invocation d'un allié (mécanisme,
totem) s'affichait à tort côté ennemis — `wakfu.log` logue TOUJOURS `isControlledByAI=true` pour
une invocation, quel que soit le camp réel de son invocateur. `session.rs` suit maintenant
`FighterJoinedEntry::summoned_by` (déjà résolu par le TS vendu, `log-parser.ts::parseFighterJoin`) :
une entrée dont `summoned_by` est renseigné n'obtient jamais de ligne dans le récap, ni ses dégâts
« bruts » (non réattribués à l'invocateur par le parser) ne créditent qui que ce soit — miroir du
`return` anticipé de `registerFighterJoin`/du filtre `summonNames` (`stats-store.service.ts`).

Protégé contre le cas inverse (un vrai monstre qui se révèle EXACTEMENT comme une invocation aux
yeux du parser — mimique, brèche — sans jamais avoir été annoncé par une ligne « X: Invoque ... »)
par `hostIsKnownMonsterName` : une fonction Rust posée sur les globals QuickJS AVANT l'évaluation du
bundle (`quickjs_engine.rs::LogParserEngine::new`), lue par `entry.ts` et transmise à `LogParser`
(`isKnownMonsterName`, un prédicat optionnel que `LogParser` accepte déjà — vendu tel quel, zéro
ligne modifiée), qui consulte `overlay_engine::CatalogIndex` en temps réel (`Engine::set_catalog`,
relayé par `overlay-ui` à chaque rechargement du catalogue) — miroir exact de `StatsStoreService`
(`isKnownMonsterName: (name) => this.catalog.isKnownWakfuMonsterName(name)`), adapté à
`CatalogIndex` plutôt que `CatalogService`.
