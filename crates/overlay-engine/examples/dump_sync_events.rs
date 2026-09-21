//! Rejoue un `wakfu.log` dans l'`Engine` et liste les événements d'historique qu'il produirait
//! (`SyncEvent` : fight / purchase / trade / pact), un par ligne — l'outil qui a servi à vérifier
//! les trois flux hors combat sur un vrai fichier (2026-09-21, §7.1 du plan).
//!
//! ```text
//! cargo run -p overlay-engine --example dump_sync_events -- <wakfu.log> [settings.json]
//! ```
//!
//! `settings.json` (facultatif) : la réponse de `GET /api/v1/settings` (ou juste `{ "roster": … }`)
//! — sans roster, un échange entre deux personnages du compte sort deux fois et `gameServer` reste
//! `null`, exactement ce que produisait le rattrapage avant les réglages (voir `spawn_engine_thread`).
//! Sans catalogue : `itemId` reste `null`, `itemName` est renseigné.

use overlay_engine::{Engine, HistoryPayload};
use overlay_ingest::Tailer;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage : dump_sync_events <wakfu.log> [settings.json]");
    // Jamais `Engine::new()` : il écrirait dans les vrais dossiers de production.
    let dir = std::env::temp_dir().join(format!("wakfu-overlay-dump-{}", std::process::id()));
    let mut engine =
        Engine::with_stores(dir.join("watchlist-counts.json"), dir.join("fights")).unwrap();
    if let Some(settings) = std::env::args().nth(2) {
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(settings).unwrap()).unwrap();
        engine.set_roster(Some(overlay_engine::RosterIndex::from_settings_json(&json)));
    }
    let mut tailer = Tailer::new(&path);
    let mut entries = 0usize;
    loop {
        let batches = tailer.poll().unwrap();
        if batches.is_empty() {
            break;
        }
        for batch in &batches {
            entries += engine.ingest_batch(batch).unwrap().len();
            for event in engine.drain_sync_events() {
                let payload = match &event.payload {
                    HistoryPayload::Fight(fight) => serde_json::json!({
                        "startedAt": fight.started_at,
                        "won": fight.won,
                        "kamasGained": fight.kamas_gained,
                        "gameServer": fight.game_server,
                        "loot": fight.loot,
                    }),
                    HistoryPayload::Purchase(purchase) => serde_json::to_value(purchase).unwrap(),
                    HistoryPayload::Trade(trade) => serde_json::to_value(trade).unwrap(),
                    HistoryPayload::PactExtraction(pact) => serde_json::to_value(pact).unwrap(),
                };
                println!("{} | {} | {payload}", event.kind.as_str(), event.signature);
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    eprintln!("entrées parsées : {entries}");
}
