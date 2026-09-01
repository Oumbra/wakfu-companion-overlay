# `overlay-engine` — L2 : frontière métier

`LineBatch` (overlay-ingest) → `LogEntry` typés (QuickJS + `LogParser` vendu depuis
`wakfu-companion`) → `SessionSnapshot` (agrégation Rust) — voir
[`docs/plan-architecture.md`](../../docs/plan-architecture.md) §2, §9, §12.

## Contenu

| Module | Rôle |
| --- | --- |
| `engine-js/` | `LogParser` + `LogEntry` **vendus tels quels** depuis `Oumbra/wakfu-companion` (voir `engine-js/VENDORED_FROM.txt` — même commit que le spike S2), bundlés en IIFE ES2020 (`esbuild`, `dist/engine.bundle.js` **committé**, ~30 Ko : pas besoin de Node.js pour `cargo build`, seulement pour retoucher le TS vendu). |
| `quickjs_engine.rs` | `LogParserEngine` : charge le bundle dans `rquickjs`, expose `parse_lines()`/`reset()`. Aucune logique métier ici, seulement le pont QuickJS↔Rust. |
| `model.rs` | Miroir Rust exact (`serde`) de `engine-js/src/log-entry.model.ts` — la forme de chaque `LogEntry`, jamais sa sémantique (qui reste dans le TS vendu). |
| `session.rs` | `Engine` (gère la sémantique `is_initial_load`/reset, §5.3) + `SessionState` : agrégation **en Rust**, pas vendue — dégâts par combattant du combat en cours, récap de session (kamas, XP, combats, butin). Volontairement minimal, voir le commentaire de module pour ce qui manque (`StatsStoreService`, §14 point 3). |
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

## Simplification assumée (à ne pas oublier)

La classification allié/ennemi utilise `FighterJoinedEntry::is_controlled_by_ai` directement, sans
suivre `summonedBy` (héritage du camp d'une invocation) — voir le commentaire en tête de
`session.rs`. Une invocation alliée s'affichera donc comme ennemie. Corriger ça correctement
reviendrait à dupliquer une heuristique de `StatsStoreService`, contraire à la raison d'être du
choix QuickJS (§2) — à ne pas « corriger » ici sans y réfléchir à deux fois.
