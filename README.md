# wakfu-companion-overlay

Overlay de jeu natif (Rust) pour [Wakfu](https://www.wakfu.com/), portage de
[`Oumbra/wakfu-companion`](https://github.com/Oumbra/wakfu-companion) : lecture en direct de
`wakfu.log`, affichage par-dessus le jeu des dégâts de combat, du suivi d'objets/ennemis et des
alertes de drop, avec synchronisation de l'historique (combats, achats et récupérations de kamas à
l'Hôtel de Vente, échanges) vers le même compte que l'application web.

**Plateformes visées : Windows et Linux (X11 / XWayland).** macOS est hors périmètre.

État : spikes de validation technique (`spikes/`) terminés, premier lot applicatif réel en cours
(`crates/`, workspace Cargo à la racine).

📄 **[Plan d'architecture technique](docs/plan-architecture.md)** — stack, modèle de threads,
ingestion du log, rendu et click-through par OS, synchronisation serveur, budget mémoire,
feuille de route et revue d'experts.

## Crates (`crates/`)

| Crate | Lot | Contenu |
| --- | --- | --- |
| [`overlay-ingest`](crates/overlay-ingest/) | L1 ✅ | Suivi de `wakfu.log` : découverte de chemin, lecture incrémentale, rotation/troncature. `cargo test -p overlay-ingest`. |
| [`overlay-app`](crates/overlay-app/) | L1 (câblage) | Binaire minimal : branche `overlay-ingest` sur la console pour l'observer sur un vrai `wakfu.log`. `cargo run -p overlay-app`. |

## Spikes (`spikes/`)

| Spike | État | Prévisualisation |
| --- | --- | --- |
| [`s1-window-windows`](spikes/s1-window-windows/) | ✅ validé | `.\preview.ps1` depuis le dossier du spike |
| [`s2-engine-quickjs`](spikes/s2-engine-quickjs/) | ✅ validé | voir son README |
