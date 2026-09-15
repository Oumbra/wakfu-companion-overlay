//! Alerte de chat sur des lots EN DIRECT (`is_initial_load: false`), et silence pendant le
//! rattrapage initial — le même gating que le ramassage et le décompte (voir `Engine::ingest_batch`).
//!
//! Lignes construites à la main au format réel du log (voir `tests/wakfu.log`, lignes
//! `[Commerce] … : …`) plutôt que rejouées depuis la fixture : le bundle dédoublonne un message
//! identique revu dans la seconde qui suit (voir `DEDUPE_WINDOW_MS` côté TS), une même ligne
//! rejouée deux fois ne produirait qu'une entrée.

use overlay_engine::{ChatChannel, ChatFilter, ChatFilterScope, Engine};
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

fn ligne(time: &str, canal: &str, auteur: &str, message: &str) -> String {
    format!(" INFO {time} [AWT-EventQueue-0] (aPV:174) - [{canal}] {auteur} : {message}")
}

#[test]
fn un_message_qui_correspond_declenche_une_alerte_en_direct_mais_pas_au_rattrapage() {
    let mut engine = engine();
    engine.set_chat_filters(vec![
        ChatFilter::new(
            ChatFilterScope::Channel(ChatChannel::Commerce),
            "Force Vitale",
        )
        .unwrap(),
        ChatFilter::new(ChatFilterScope::All, "kralamoure").unwrap(),
    ]);

    // Rattrapage : le message est déjà dans le fichier, il a déjà été lu — silence.
    engine
        .ingest_batch(&LineBatch {
            lines: vec![ligne(
                "20:34:50,180",
                "Commerce",
                "Anonyme-042",
                "vends Force Vitale II, mp si intéressé",
            )],
            is_initial_load: true,
        })
        .expect("ingestion du rattrapage");
    assert!(
        engine.drain_chat_alerts().is_empty(),
        "pas d'alerte pendant le rattrapage"
    );

    // En direct : trois messages, deux correspondent (texte sur Commerce, auteur sur tout canal),
    // le troisième est sur le mauvais canal pour sa recherche.
    engine
        .ingest_batch(&LineBatch {
            lines: vec![
                ligne(
                    "20:40:00,000",
                    "Commerce",
                    "Marchand",
                    "vends force vitale II pas cher",
                ),
                ligne(
                    "20:40:01,000",
                    "Guilde",
                    "Marchand",
                    "vends force vitale II pas cher",
                ),
                ligne(
                    "20:40:02,000",
                    "Proximité",
                    "Kralamoure-Fan",
                    "quelqu'un pour le boss ?",
                ),
            ],
            is_initial_load: false,
        })
        .expect("ingestion du direct");
    let alerts = engine.drain_chat_alerts();
    assert_eq!(alerts.len(), 2, "{alerts:?}");
    assert_eq!(alerts[0].channel, ChatChannel::Commerce);
    assert_eq!(alerts[0].author, "Marchand");
    assert_eq!(alerts[0].message, "vends force vitale II pas cher");
    assert_eq!(alerts[0].filter.text, "force vitale");
    assert_eq!(alerts[1].author, "Kralamoure-Fan");
    assert_eq!(alerts[1].filter.scope, ChatFilterScope::All);
    // Vidée par le drain.
    assert!(engine.drain_chat_alerts().is_empty());
}

#[test]
fn sans_recherche_aucun_message_ne_sonne() {
    let mut engine = engine();
    engine
        .ingest_batch(&LineBatch {
            lines: vec![ligne("20:40:00,000", "Commerce", "Marchand", "vends tout")],
            is_initial_load: false,
        })
        .expect("ingestion");
    assert!(engine.drain_chat_alerts().is_empty());
}
