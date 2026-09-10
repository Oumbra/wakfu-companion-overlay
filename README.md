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

Avant de pousser (le hook `pre-push` le fait pour les lints) :

```bash
bash scripts/ci-local.sh          # tout : format, clippy, tests
bash scripts/ci-local.sh --lint   # format + clippy seulement (rapide)
```

## Crates (`crates/`)

| Crate | Lot | Contenu |
| --- | --- | --- |
| [`overlay-ingest`](crates/overlay-ingest/) | L1 ✅ | Suivi de `wakfu.log` : découverte de chemin, lecture incrémentale, rotation/troncature. `cargo test -p overlay-ingest`. |
| [`overlay-app`](crates/overlay-app/) | L1 (câblage) | Binaire minimal : branche `overlay-ingest` sur la console pour l'observer sur un vrai `wakfu.log`. `cargo run -p overlay-app`. |
| [`overlay-engine`](crates/overlay-engine/) | L2 🟡 | QuickJS + `LogParser` vendu depuis `wakfu-companion` → `LogEntry` → `SessionSnapshot` (agrégation Rust). `cargo test -p overlay-engine`. |
| [`overlay-ui`](crates/overlay-ui/) | L2 🟡 | Premier overlay réel : fenêtre S1 + panneaux Dégâts du combat/Récap de session, ancré sur la fenêtre du jeu, sur un vrai `wakfu.log`. `.\preview.ps1` depuis le dossier du crate. |

## Spikes (`spikes/`)

| Spike | État | Prévisualisation |
| --- | --- | --- |
| [`s1-window-windows`](spikes/s1-window-windows/) | ✅ validé | `.\preview.ps1` depuis le dossier du spike |
| [`s2-engine-quickjs`](spikes/s2-engine-quickjs/) | ✅ validé | voir son README |
