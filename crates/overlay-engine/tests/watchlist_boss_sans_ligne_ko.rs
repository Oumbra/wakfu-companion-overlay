//! Régression réelle (retour utilisateur, 2026-09-01) : sur un vrai combat de donjon (extrait
//! authentique du `wakfu.log` de l'utilisateur, voir `watchlist_boss_sans_ligne_ko.log`), AUCUN
//! des 4 ennemis (dont le boss "El Pochito") ne produit de ligne "est KO !"/"est hors-combat !"
//! avant la fin du combat — seul le butin ramassé juste après confirme la victoire. Sans le filet
//! de rattrapage de `SessionState::apply` (cas `CombatEnd`, voir sa doc), aucun des 4 n'était
//! jamais crédité dans le Suivi malgré une victoire confirmée. Pas un test synthétique : mêmes
//! lignes que le vrai log, seulement élaguées des lignes de dégâts/effets sans rapport avec ce
//! qui est vérifié ici.

use std::fs;

use overlay_engine::{Engine, WatchlistEntry, WatchlistKind, WatchlistMode};
use overlay_ingest::LineBatch;

const LOG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/watchlist_boss_sans_ligne_ko.log"
);

fn watched(name: &str, kind: WatchlistKind) -> WatchlistEntry {
    WatchlistEntry {
        name: name.to_string(),
        kind,
        mode: WatchlistMode::Up,
        count: 0,
        countdown_target: 0,
        catalog_id: None,
    }
}

#[test]
fn tous_les_ennemis_dun_combat_gagne_sont_credites_meme_sans_ligne_de_defaite_dediee() {
    // JAMAIS `Engine::new()`/`Engine::with_watchlist_store()` seul ici — voir la doc d'`Engine::
    // with_stores` : un test d'intégration (ce fichier) n'a pas `cfg(test)` actif pour
    // `overlay-engine`, donc écrirait dans le VRAI fichier de compteurs de production (bug réel
    // vécu en session, 2026-09-01 — compteurs d'objets de l'utilisateur perdus) ET, depuis
    // `fight_store`, dans le VRAI dossier de combats de production. Chemins temporaires dédiés,
    // nettoyés en fin de test.
    let store_dir = std::env::temp_dir().join(format!(
        "wakfu-overlay-engine-test-{}-{}",
        module_path!().replace("::", "_"),
        std::process::id()
    ));
    let store_path = store_dir.join("watchlist-counts.json");
    let fight_store_dir = store_dir.join("fights");
    let mut engine =
        Engine::with_stores(store_path, fight_store_dir).expect("création de l'Engine");
    engine.set_watchlist_entries(vec![
        watched("El Pochito", WatchlistKind::Enemy),
        watched("Rey Mystroolrio", WatchlistKind::Enemy),
        watched("The Undertroolker", WatchlistKind::Enemy),
        watched("Troolk Hoogan", WatchlistKind::Enemy),
        // Contrôle négatif : jamais rejoint ce combat, ne doit JAMAIS être crédité.
        watched("Un Monstre Absent", WatchlistKind::Enemy),
    ]);

    // `LineBatch` construit à la main plutôt que via `Tailer` (voir `session_real_log.rs`) : ce
    // test vérifie le comptage d'un lot EN DIRECT (`is_initial_load: false`), pas le rattrapage —
    // `Tailer::poll()` sur un fichier lu depuis zéro marquerait tout comme rattrapage, ce que
    // `Engine::ingest_batch` gate volontairement pour la watchlist (voir sa doc).
    let content = fs::read_to_string(LOG).expect("lecture de la fixture");
    let lines: Vec<String> = content.lines().map(str::to_string).collect();
    engine
        .ingest_batch(&LineBatch {
            lines,
            is_initial_load: false,
        })
        .expect("ingestion du combat");

    let counts: std::collections::HashMap<&str, i64> = engine
        .watchlist_entries()
        .iter()
        .map(|e| (e.name.as_str(), e.count))
        .collect();

    assert_eq!(counts["El Pochito"], 1, "le boss doit être crédité");
    assert_eq!(counts["Rey Mystroolrio"], 1);
    assert_eq!(counts["The Undertroolker"], 1);
    assert_eq!(counts["Troolk Hoogan"], 1);
    assert_eq!(
        counts["Un Monstre Absent"], 0,
        "jamais rejoint ce combat, ne doit rien recevoir"
    );

    let _ = fs::remove_dir_all(&store_dir);
}
