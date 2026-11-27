//! Critère de sortie de la phase 1 (`docs/plan-historique-local.md` §12) : rejouer dix fois le même
//! `wakfu.log` — dix lancements de l'overlay — n'écrit **aucun doublon**, et la base raconte la même
//! session que le moteur (mêmes combats, mêmes victoires, même XP, mêmes kamas).
//!
//! Le log est la fixture pseudonymisée d'`overlay-engine` (une vraie session de jeu, 10 975 lignes).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

use overlay_engine::session::SessionSnapshot;
use overlay_engine::{CatalogIndex, DungeonIndex, Engine, HistoryPayload, SyncEvent};
use overlay_history::{Counts, HistoryStore, Referentiels};
use overlay_ingest::Tailer;

const WAKFU_LOG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../overlay-engine/tests/wakfu.log"
);

static NEXT_ID: AtomicU32 = AtomicU32::new(0);

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "overlay-history-{name}-{}-{}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Un lancement de l'overlay : un moteur neuf relit tout le log et rend ses événements, lot par lot
/// comme le thread moteur les relaie, avec le total de session qu'il affiche.
fn one_launch(
    catalog: &Arc<CatalogIndex>,
    dungeons: &Arc<DungeonIndex>,
) -> (Vec<Vec<SyncEvent>>, SessionSnapshot) {
    let dir = temp_dir("moteur");
    let mut engine = Engine::with_stores(dir.join("watchlist-counts.json"), dir.join("fights"))
        .expect("création de l'Engine");
    engine.set_catalog(Arc::clone(catalog));
    engine.set_dungeons(Arc::clone(dungeons));
    let mut tailer = Tailer::new(WAKFU_LOG);
    let mut lots = Vec::new();
    loop {
        let batches = tailer.poll().expect("poll() du tailer");
        if batches.is_empty() {
            break;
        }
        for batch in &batches {
            engine.ingest_batch(batch).expect("ingestion d'un lot");
            let events = engine.drain_sync_events();
            if !events.is_empty() {
                lots.push(events);
            }
        }
    }
    let snapshot = engine.snapshot();
    let _ = std::fs::remove_dir_all(&dir);
    (lots, snapshot)
}

fn sum(store: &HistoryStore, sql: &str) -> i64 {
    store
        .connection()
        .query_row(sql, [], |row| row.get::<_, Option<i64>>(0))
        .unwrap()
        .unwrap_or(0)
}

#[test]
fn dix_lancements_sur_le_meme_log_n_ecrivent_aucun_doublon() {
    let referentiel = overlay_engine::referentiel::load();
    let catalog = Arc::new(referentiel.catalog);
    let dungeons = Arc::new(referentiel.dungeons);
    let refs = Referentiels {
        catalog: &catalog,
        dungeons: &dungeons,
    };
    let dir = temp_dir("base");
    let path = dir.join(overlay_history::FILE_NAME);

    let mut after_first: Option<Counts> = None;
    let mut snapshot = None;
    let mut last_lots = Vec::new();
    for launch in 1..=10 {
        let (lots, session) = one_launch(&catalog, &dungeons);
        let mut store = HistoryStore::open(&path).unwrap();
        let started = Instant::now();
        let mut inserted = 0;
        for lot in &lots {
            let report = store.record(lot, refs).unwrap();
            assert_eq!(report.skipped, 0, "aucun événement ne doit être écarté");
            inserted += report.fights_inserted
                + report.purchases_inserted
                + report.trades_inserted
                + report.pacts_inserted;
        }
        let counts = store.counts().unwrap();
        eprintln!(
            "lancement {launch} : {} lots écrits en {:?}, {inserted} lignes neuves, {counts:?}",
            lots.len(),
            started.elapsed()
        );
        match &after_first {
            None => {
                assert!(
                    counts.fights > 10,
                    "le log contient des dizaines de combats"
                );
                after_first = Some(counts);
            }
            Some(first) => {
                assert_eq!(inserted, 0, "lancement {launch} : doublon écrit");
                assert_eq!(&counts, first, "lancement {launch} : volumes changés");
            }
        }
        snapshot = Some(session);
        last_lots = lots;
    }

    // La base raconte la même session que le moteur.
    let store = HistoryStore::open(&path).unwrap();
    let totals = &snapshot.unwrap().totals;
    assert_eq!(
        sum(&store, "SELECT COUNT(*) FROM fights WHERE won = 1"),
        totals.fights_won,
        "combats gagnés"
    );
    assert_eq!(
        sum(&store, "SELECT COUNT(*) FROM fights WHERE won = 0"),
        totals.fights_lost,
        "combats perdus"
    );
    // XP et kamas des COMBATS, tels que le moteur les a émis (dernière version de chaque combat).
    // Pas `totals.xp_gained` : le total de session compte aussi l'XP gagnée hors combat (métiers),
    // que le site ne stockait pas davantage.
    let mut per_fight: std::collections::HashMap<&str, (i64, i64)> = Default::default();
    for event in last_lots.iter().flatten() {
        if let HistoryPayload::Fight(fight) = &event.payload {
            per_fight.insert(
                &event.signature,
                (fight.xp_gained, fight.kamas_gained.unwrap_or(0)),
            );
        }
    }
    let (xp, kamas) = per_fight
        .values()
        .fold((0, 0), |(xp, kamas), (x, k)| (xp + x, kamas + k));
    assert!(xp > 0 && xp <= totals.xp_gained);
    assert_eq!(sum(&store, "SELECT SUM(xp_gained) FROM fights"), xp, "XP");
    assert_eq!(
        sum(&store, "SELECT SUM(kamas_gained) FROM fights"),
        kamas,
        "kamas"
    );
    // Chaque combat a un type ; les combats de donjon ont leur run.
    assert_eq!(
        sum(
            &store,
            "SELECT COUNT(*) FROM fights WHERE fight_type IS NULL"
        ),
        0,
        "combat sans type"
    );
    assert_eq!(
        sum(
            &store,
            "SELECT COUNT(*) FROM fights WHERE dungeon_id IS NOT NULL AND dungeon_run_key IS NULL"
        ),
        0,
        "combat de donjon sans run"
    );
    drop(store);
    let _ = std::fs::remove_dir_all(&dir);
}
