//! Rendu hors écran de la **fenêtre d'historique** sur une vraie base — un PNG par onglet, pour
//! comparer avec le site et repérer les régressions sur ses propres données.
//!
//! ```text
//! cargo run -p overlay-testkit --example historique -- [--db <history.sqlite3>] [--onglet achats]
//!     [--recherche <texte>] [--taille 1100x820]
//! ```
//!
//! Sans `--db`, la base de l'overlay sur ce poste (`HistoryStore::default_path`), ouverte en
//! lecture seulement. Les PNG vont dans `target/rendus/historique-<onglet>.png` — hors dépôt : une
//! vraie base contient des noms de personnages.
use egui_kittest::Harness;
use overlay_history::HistoryStore;
use overlay_ui::panels::history::{self, HistoryContext, HistoryTab, HistoryWindowState};

fn dossier_sortie() -> std::path::PathBuf {
    let dir = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(target) => std::path::PathBuf::from(target),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"),
    }
    .join("rendus");
    std::fs::create_dir_all(&dir).expect("création de target/rendus");
    dir
}

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn main() {
    overlay_ui::build_info::freeze_for_snapshots();
    let path = arg("--db")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| HistoryStore::default_path().expect("dossier de données"));
    let (width, height) = arg("--taille")
        .and_then(|t| {
            let (w, h) = t.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or((1100.0f32, 820.0f32));
    let tabs: Vec<HistoryTab> = match arg("--onglet") {
        Some(name) => vec![HistoryTab::parse(&name).expect("onglet inconnu")],
        None => HistoryTab::ALL.to_vec(),
    };
    let query = arg("--recherche").unwrap_or_default();
    // Copie de travail : la base de l'overlay peut être ouverte par lui en même temps, et ce rendu
    // ne doit jamais y écrire (même pas une migration).
    let copy =
        std::env::temp_dir().join(format!("historique-rendu-{}.sqlite3", std::process::id()));
    std::fs::copy(&path, &copy).expect("copie de la base");
    let store = HistoryStore::open(&copy).expect("ouverture de la base");
    let referentiel = overlay_engine::referentiel::load();
    let today = chrono::Local::now().date_naive();

    // `--texte` : le modèle des vues, une ligne par élément affiché — ce qu'on compare au texte
    // des pages du site (voir `docs/plan-historique-local.md`, vérifications de la phase 3).
    if std::env::args().any(|a| a == "--texte") {
        let rows = store.purchases().expect("achats");
        for group in overlay_ui::panels::history::achats::groups(
            &rows,
            &query,
            Default::default(),
            &referentiel.catalog,
        ) {
            println!(
                "J|{}|{}",
                overlay_ui::panels::history::format::relative_day(group.day, today),
                group.total_cost
            );
            for row in group.rows {
                println!(
                    "L|{}|{}|{}{}",
                    row.name,
                    if row.is_hdv_sale { 0 } else { row.quantity },
                    if row.is_hdv_sale { "+" } else { "" },
                    row.total_cost
                );
            }
        }
        return;
    }

    for tab in tabs {
        let mut state = HistoryWindowState::new(tab);
        state.achats.query = query.clone();
        // Sans réseau : les icônes d'objets restent vides ici (l'essai automatique de la vraie
        // fenêtre, `--autotest`, les montre).
        let remote_icons = overlay_ui::remote_icons::RemoteIconStore::empty();
        let mut textures = overlay_ui::remote_icons::RemoteIconTextures::default();
        let ctx = HistoryContext {
            store: Some(&store),
            catalog: &referentiel.catalog,
            remote_icons: &remote_icons,
            today,
        };
        let started = std::time::Instant::now();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(width, height))
            .build_ui(|ui| {
                overlay_ui::style::apply(ui.ctx());
                let _ = history::show(ui, &mut state, &ctx, &mut textures);
            });
        harness.run();
        let image = harness.render().expect("rendu de la fenêtre");
        let name = overlay_engine::roster::normalize_wakfu_name(tab.label()).replace(' ', "-");
        let out = dossier_sortie().join(format!("historique-{name}.png"));
        image.save(&out).expect("écriture du PNG");
        println!("{} ({:?})", out.display(), started.elapsed());
    }
    drop(store);
    let _ = std::fs::remove_file(&copy);
}
