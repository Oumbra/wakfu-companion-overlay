//! Mesure jetable : coût d'écriture et taille disque d'un gros historique local.
//!
//! `cargo run --release -p overlay-history --example bench-historique [combats]`
//!
//! Simule un joueur intensif : N combats (8 participants, 6 lignes de butin, ventilation par sort),
//! écrits par lots de 50 comme au fil d'une session, puis un « relancement » qui réécrit tout
//! (rattrapage du log) et doit ne rien ajouter.
use std::time::Instant;

use overlay_engine::history::{
    FightLootPayload, FightParticipantPayload, FightPayload, FightSide, FightSpellPayload,
    HistoryEventKind, HistoryPayload,
};
use overlay_engine::log_time::format_iso_utc;
use overlay_engine::SyncEvent;
use overlay_history::{HistoryStore, Referentiels};

fn fight(index: i64) -> SyncEvent {
    let spells = vec![
        FightSpellPayload {
            spell: "Épée Céleste".to_string(),
            total: 12_000,
            by_element: [("Air".to_string(), 12_000)].into_iter().collect(),
            by_turn: Vec::new(),
        };
        4
    ];
    let participants = (0..8)
        .map(|seat| FightParticipantPayload {
            side: if seat < 4 {
                FightSide::Ally
            } else {
                FightSide::Enemy
            },
            name: if seat < 4 {
                format!("Personnage{seat}")
            } else {
                "Bouftou".to_string()
            },
            monster_id: (seat >= 4).then_some(1),
            instance_index: seat,
            class_name: None,
            damage: 10_000 + seat,
            defeated: seat >= 4,
            fled: false,
            spells: spells.clone(),
            heal: 0,
            armor: 0,
            heal_spells: Vec::new(),
            armor_spells: Vec::new(),
            xp_gained: 1_000,
        })
        .collect();
    let started = 1_780_000_000_000 + index * 180_000;
    SyncEvent {
        kind: HistoryEventKind::Fight,
        signature: format!("{started}|{index}|won|bench"),
        payload: HistoryPayload::Fight(FightPayload {
            fight_id: Some(index),
            started_at: format_iso_utc(started),
            duration_ms: Some(120_000),
            won: true,
            turns: 6,
            total_damage: 40_000,
            xp_gained: 4_000,
            kamas_gained: Some(250),
            game_server: Some("pandora".to_string()),
            dungeon_id: None,
            dungeon_run_signature: None,
            challenges_passed: 1,
            challenges_failed: 0,
            participants,
            loot: (0..6)
                .map(|line| FightLootPayload {
                    item_id: Some(1718 + line),
                    item_name: None,
                    quantity: 3,
                })
                .collect(),
        }),
    }
}

fn main() {
    let total: i64 = std::env::args()
        .nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(20_000);
    let referentiel = overlay_engine::referentiel::load();
    let refs = Referentiels {
        catalog: &referentiel.catalog,
        dungeons: &referentiel.dungeons,
    };
    let dir = std::env::temp_dir().join(format!("bench-historique-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join(overlay_history::FILE_NAME);

    let events: Vec<SyncEvent> = (0..total).map(fight).collect();
    let started = Instant::now();
    for lot in events.chunks(50) {
        // Une ouverture par lot, comme le thread Historique.
        let mut store = HistoryStore::open(&path).unwrap();
        store.record(lot, refs).unwrap();
    }
    let first = started.elapsed();
    println!(
        "{total} combats écrits par lots de 50 : {first:?} ({:.2} ms par lot)",
        first.as_secs_f64() * 1000.0 / (total as f64 / 50.0)
    );

    let started = Instant::now();
    let mut store = HistoryStore::open(&path).unwrap();
    let report = store.record(&events, refs).unwrap();
    println!(
        "relancement (tout réécrit en une transaction) : {:?}, {} nouveaux",
        started.elapsed(),
        report.fights_inserted
    );

    let started = Instant::now();
    let page: Vec<i64> = store
        .connection()
        .prepare("SELECT id FROM fights ORDER BY started_at DESC, id DESC LIMIT 50")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    println!(
        "page de 50 combats : {:?} ({} lignes)",
        started.elapsed(),
        page.len()
    );

    let started = Instant::now();
    let xp: i64 = store
        .connection()
        .query_row(
            "SELECT SUM(xp_gained) FROM fights WHERE started_at >= ?1",
            [1_780_000_000_000i64 + (total / 2) * 180_000],
            |row| row.get(0),
        )
        .unwrap();
    println!(
        "agrégat XP sur la moitié de l'historique : {:?} ({xp})",
        started.elapsed()
    );

    drop(store);
    let size: u64 = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter_map(|entry| entry.metadata().ok())
        .map(|meta| meta.len())
        .sum();
    println!(
        "taille sur disque : {:.1} Mo ({:.1} Ko par combat)",
        size as f64 / 1_048_576.0,
        size as f64 / 1024.0 / total as f64
    );
    let _ = std::fs::remove_dir_all(&dir);
}
