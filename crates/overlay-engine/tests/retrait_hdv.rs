//! Retour utilisateur (2026-10-05) : récupérer à l'Hôtel de vente un objet qu'on avait soi-même mis
//! en vente faisait monter son compteur de Suivi, comme s'il avait été obtenu. Ce retrait ne compte
//! plus, sauf si la case « Activer la prise en compte des invendus de l'hôtel de vente » est cochée
//! (`Engine::set_track_hdv_retrievals`, `false` par défaut). Un ACHAT, lui, compte toujours.
//!
//! Lignes reprises du `wakfu.log` fourni avec le retour (« Pierre d'entourage » retirée à 11:54:04,
//! « Saccawotte » achetée à 11:34:43), en lot EN DIRECT (`is_initial_load: false`) : le rattrapage
//! initial ne compte jamais rien, voir `Engine::apply_entry`.

use overlay_engine::{Engine, WatchlistEntry, WatchlistKind, WatchlistMode};
use overlay_ingest::LineBatch;

/// Chemins temporaires dédiés — jamais `Engine::new()` dans un test d'intégration, voir
/// `watchlist_boss_sans_ligne_ko.rs`.
fn engine(test: &str) -> Engine {
    let store_dir = std::env::temp_dir().join(format!(
        "wakfu-overlay-engine-test-{}-{}-{}",
        module_path!().replace("::", "_"),
        test,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&store_dir);
    let mut engine = Engine::with_stores(
        store_dir.join("watchlist-counts.json"),
        store_dir.join("fights"),
    )
    .expect("création de l'Engine");
    engine.set_watchlist_entries(
        [
            "Pierre d'entourage",
            "Saccawotte",
            "Sac en Toile Usée",
            "Blé",
        ]
        .into_iter()
        .map(|name| WatchlistEntry {
            name: name.to_string(),
            kind: WatchlistKind::Item,
            mode: WatchlistMode::Up,
            count: 0,
            countdown_target: 0,
            catalog_id: None,
        })
        .collect(),
    );
    engine
}

fn jeu(time: &str, message: &str) -> String {
    format!(" INFO {time} [AWT-EventQueue-0] (aNZ:174) - [Information (jeu)] {message}")
}

fn ouverture_hdv(time: &str) -> String {
    format!(
        " INFO {time} [AWT-EventQueue-0] (bmc:41) - Lancement de l'occupation MARKET sur la board [bCM id=31570]{{Point3 : (-21, -92, 3)}}"
    )
}

fn fermeture_hdv(time: &str) -> String {
    format!(
        " INFO {time} [AWT-EventQueue-0] (bmc:77) - On arrête l'occupation MARKET sur la board [bCM id=31570]{{Point3 : (-21, -92, 3)}}"
    )
}

fn ingerer(engine: &mut Engine, lines: Vec<String>) {
    engine
        .ingest_batch(&LineBatch {
            lines,
            is_initial_load: false,
        })
        .expect("ingestion du lot");
}

fn compte(engine: &Engine, name: &str) -> i64 {
    engine
        .watchlist_entries()
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.count)
        .expect("entrée suivie")
}

/// Le scénario du fichier : un achat (perte de kamas juste avant le ramassage), un sac ouvert hors
/// de l'HDV, puis le retrait de la Pierre d'entourage mise en vente.
fn scenario() -> Vec<String> {
    vec![
        ouverture_hdv("11:34:40,245"),
        jeu("11:34:43,174", "Vous avez perdu 74 958 kamas."),
        jeu("11:34:43,174", "Vous avez ramassé 3x Saccawotte ."),
        fermeture_hdv("11:34:43,498"),
        jeu("11:35:12,140", "Vous avez ramassé 1x Sac en Toile Usée ."),
        ouverture_hdv("11:53:57,816"),
        jeu("11:54:04,453", "Vous avez ramassé 1x Pierre d'entourage ."),
        fermeture_hdv("11:54:50,327"),
    ]
}

#[test]
fn par_defaut_un_retrait_hdv_ne_compte_pas_mais_un_achat_si() {
    let mut engine = engine("defaut");
    ingerer(&mut engine, scenario());

    assert_eq!(compte(&engine, "Saccawotte"), 3, "un achat compte toujours");
    assert_eq!(
        compte(&engine, "Sac en Toile Usée"),
        1,
        "un ramassage hors HDV compte"
    );
    assert_eq!(
        compte(&engine, "Pierre d'entourage"),
        0,
        "l'objet récupéré de ses propres ventes ne compte pas"
    );
}

#[test]
fn l_option_cochee_compte_aussi_les_retraits_hdv() {
    let mut engine = engine("option");
    engine.set_track_hdv_retrievals(true);
    ingerer(&mut engine, scenario());

    assert_eq!(compte(&engine, "Saccawotte"), 3);
    assert_eq!(compte(&engine, "Pierre d'entourage"), 1);
}

#[test]
fn une_session_hdv_jamais_refermee_s_eteint_apres_une_minute() {
    // Comme l'ouverture de 20:33:04 de `tests/wakfu.log` : aucune ligne de fermeture. Au-delà de
    // `MARKET_IDLE_MS` sans activité marchande, un ramassage n'est plus un retrait.
    let mut engine = engine("inactive");
    ingerer(
        &mut engine,
        vec![
            ouverture_hdv("10:00:00,000"),
            jeu("10:00:30,000", "Vous avez ramassé 1x Pierre d'entourage ."),
            jeu("10:01:31,000", "Vous avez ramassé 2x Blé ."),
        ],
    );

    assert_eq!(compte(&engine, "Pierre d'entourage"), 0, "dans la minute");
    assert_eq!(
        compte(&engine, "Blé"),
        2,
        "une minute d'inactivité plus tard"
    );
}

#[test]
fn le_butin_d_un_combat_en_cours_n_est_jamais_un_retrait() {
    // Fenêtre HDV ouverte pendant qu'un combat tourne (multicompte) : le butin porte un `fightId`,
    // il reste compté.
    let mut engine = engine("combat");
    ingerer(
        &mut engine,
        vec![
            " INFO 10:00:00,000 [AWT-EventQueue-0] (a:1) - [_FL_] fightId=1 Oumbra breed : 4 [1] isControlledByAI=false obstacleId : -1 join the fight at {P}".to_string(),
            " INFO 10:00:00,001 [AWT-EventQueue-0] (a:1) - [_FL_] fightId=1 Blop breed : 4777 [-1] isControlledByAI=true obstacleId : -1 join the fight at {P}".to_string(),
            ouverture_hdv("10:00:05,000"),
            jeu("10:00:10,000", "Vous avez ramassé 2x Blé ."),
        ],
    );

    assert_eq!(compte(&engine, "Blé"), 2);
}
