//! Régression (retour utilisateur, 2026-09-17) : dans l'onglet « Alertes », un objet passé en mode
//! silencieux ne produisait plus ni toast ni confettis — le moteur ne l'alertait pas du tout,
//! miroir du web où « Son désactivé » coupe tout. Dans l'overlay, la sourdine ne coupe QUE le son
//! (voir `overlay_engine::LootAlert`) : l'alerte doit sortir, marquée `sound_enabled: false`.
//!
//! Ligne construite à la main au format réel du log (voir
//! `tests/watchlist_boss_sans_ligne_ko.log`), en lot EN DIRECT (`is_initial_load: false`) — le
//! rattrapage initial reste muet, comme pour le décompte et le chat.

use overlay_engine::{Engine, SoundItemEntry};
use overlay_ingest::LineBatch;

fn engine() -> Engine {
    let store_dir = std::env::temp_dir().join(format!(
        "wakfu-overlay-engine-test-{}-{}",
        module_path!().replace("::", "_"),
        std::process::id()
    ));
    Engine::with_stores(
        store_dir.join("watchlist-counts.json"),
        store_dir.join("fights"),
    )
    .expect("création de l'Engine")
}

fn ramassage(time: &str, quantite: i64, objet: &str) -> String {
    format!(
        " INFO {time} [AWT-EventQueue-0] (aPV:174) - [Information (jeu)] Vous avez ramassé {quantite}x {objet} ."
    )
}

fn entree(name: &str, enabled: bool) -> SoundItemEntry {
    SoundItemEntry {
        name: name.to_string(),
        enabled,
        is_default: false,
        catalog_id: None,
    }
}

#[test]
fn un_objet_silencieux_alerte_sans_son_et_un_objet_inconnu_n_alerte_pas() {
    let mut engine = engine();
    engine.set_sound_items(vec![
        entree("Cape d'El Pochito", true),
        entree("Pierre d'aventure", false),
    ]);

    engine
        .ingest_batch(&LineBatch {
            lines: vec![
                ramassage("22:43:56,340", 1, "Cape d'El Pochito"),
                ramassage("22:43:56,341", 3, "Pierre d'aventure"),
                ramassage("22:43:56,342", 1, "Larve Bleue"),
            ],
            is_initial_load: false,
        })
        .expect("ingestion du lot");

    let alerts = engine.drain_loot_alerts();
    let resume: Vec<(&str, i64, bool)> = alerts
        .iter()
        .map(|a| (a.name.as_str(), a.quantity, a.sound_enabled))
        .collect();
    assert_eq!(
        resume,
        vec![
            ("Cape d'El Pochito", 1, true),
            ("Pierre d'aventure", 3, false),
        ],
        "l'objet silencieux alerte (carte) sans son ; l'objet hors liste n'alerte pas"
    );
    assert!(
        engine.drain_loot_alerts().is_empty(),
        "drainé une fois, la file est vide"
    );
}

#[test]
fn le_rattrapage_initial_n_alerte_jamais_meme_son_active() {
    let mut engine = engine();
    engine.set_sound_items(vec![entree("Cape d'El Pochito", true)]);
    engine
        .ingest_batch(&LineBatch {
            lines: vec![ramassage("22:43:56,340", 1, "Cape d'El Pochito")],
            is_initial_load: true,
        })
        .expect("ingestion du rattrapage");
    assert!(engine.drain_loot_alerts().is_empty());
}
