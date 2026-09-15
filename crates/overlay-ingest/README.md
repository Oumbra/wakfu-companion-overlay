# `overlay-ingest` — L1 : suivi de `wakfu.log`

Premier crate applicatif réel du projet (voir [`docs/plan-architecture.md`](../../docs/plan-architecture.md)
§4 et §12, lot **L1**). Thread IO du diagramme d'exécution §3 : ne parse rien, découpe des lignes
complètes et les publie par lots (`LineBatch`).

## Contenu

| Module | Rôle |
| --- | --- |
| `discovery` | Chemins candidats de `wakfu.log` par OS (§5.1) et résolution du premier existant. |
| `rotation` | `FileIdentity` : identité de fichier indépendante du chemin (`dev`/`ino` sous Unix, `FileId` 128 bits sous Windows), pour distinguer une rotation d'une simple croissance. |
| `tailer` | `Tailer::poll()` : lecture incrémentale, gestion du reliquat de ligne partielle, détection rotation/troncature, sémantique `is_initial_load` (§5.3). **Pur et synchrone** — aucune E/S déclenchée par un événement, testable sans dépendre du système de fichiers réel. |
| `watcher` | `watcher::spawn()` : fil réel au-dessus de `Tailer` — `notify` sur le répertoire parent + repli par sondage à 1 s, débounce 100 ms (§5.2). |

## Pourquoi `Tailer` et `watcher` sont séparés

`Tailer::poll()` ne fait qu'une chose observable : étant donné l'état du fichier sur disque *au
moment de l'appel*, que faut-il publier ? Tous les scénarios délicats (rejeu, rotation,
troncature, ligne partielle, gros volume à découper) se testent donc en pilotant `poll()` à la
main sur de vrais fichiers temporaires (`tests/tailer.rs`), sans dépendre d'événements `notify`
réels — non déterministes et lents en CI.

`watcher` ne fait que décider *quand* appeler `poll()` (sur événement filesystem ou à intervalle
fixe). Sa correction dépend surtout de celle de `notify` elle-même ; il est volontairement exclu
des tests par défaut (`tests/watcher_smoke.rs`, marqué `#[ignore]`) et vérifié à la main :

```
cargo test -p overlay-ingest --test watcher_smoke -- --ignored --nocapture
```

## Tests

```
cargo test -p overlay-ingest
```

Couvre le critère de sortie de L1 (§12) : rejeu simple, ligne partielle, fin de ligne CRLF,
troncature en place, rotation (suppression + recréation), fichier absent, découpage en lots
bornés à `MAX_BATCH_LINES`, et robustesse à un encodage invalide.
