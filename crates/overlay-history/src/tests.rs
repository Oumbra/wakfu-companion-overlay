//! Tests de l'écriture et des recalculs sur des événements construits à la main. Le rejeu d'un vrai
//! `wakfu.log` dans le moteur est dans `tests/rejeu_log.rs`.

use overlay_engine::history::{
    FightLootPayload, FightParticipantPayload, FightPayload, FightSide, HistoryEventKind,
    HistoryPayload, PactExtractionItemPayload, PactExtractionPayload, PurchasePayload,
    TradeDirection, TradeItemPayload, TradePayload,
};
use overlay_engine::{CatalogIndex, DungeonIndex, SyncEvent};

use super::*;

fn catalog() -> CatalogIndex {
    CatalogIndex::from_json(&serde_json::json!({
        "items": [[10, "Laine de Bouftou", 1, 1, 0, 3]],
        "monsters": [
            [1, "Bouftou", 1, 10, 0, 0, 0],
            [2, "Bouftou Royal", 2, 10, 1, 0, 0],
            [3, "Bouftou Noir", 3, 10, 0, 0, 0],
        ],
    }))
}

fn dungeons() -> DungeonIndex {
    DungeonIndex::from_json(&serde_json::json!([
        {"id": 100, "fr": "Donjon Bouftou", "bossMonsterId": [2], "monsterFamilyId": [10],
         "type": "THREE_ROOMS", "hasPreBossArchi": false},
    ]))
}

fn enemy(name: &str, monster_id: i64, instance_index: i64) -> FightParticipantPayload {
    FightParticipantPayload {
        side: FightSide::Enemy,
        name: name.to_string(),
        monster_id: Some(monster_id),
        instance_index,
        class_name: None,
        damage: 100,
        defeated: true,
        fled: false,
        spells: Vec::new(),
        heal: 0,
        armor: 0,
        heal_spells: Vec::new(),
        armor_spells: Vec::new(),
        xp_gained: 0,
    }
}

fn ally() -> FightParticipantPayload {
    FightParticipantPayload {
        side: FightSide::Ally,
        name: "Anonyme-Iop1".to_string(),
        monster_id: None,
        instance_index: 0,
        class_name: Some("Iop".to_string()),
        damage: 500,
        defeated: false,
        fled: false,
        spells: Vec::new(),
        heal: 0,
        armor: 0,
        heal_spells: Vec::new(),
        armor_spells: Vec::new(),
        xp_gained: 1200,
    }
}

/// Un combat gagné commencé à `minute` minutes après une heure fixe, contre `enemies`.
fn fight(signature: &str, minute: i64, enemies: &[(&str, i64)]) -> SyncEvent {
    let mut participants = vec![ally()];
    participants.extend(
        enemies
            .iter()
            .enumerate()
            .map(|(i, (name, id))| enemy(name, *id, i as i64)),
    );
    SyncEvent {
        kind: HistoryEventKind::Fight,
        signature: signature.to_string(),
        payload: HistoryPayload::Fight(FightPayload {
            fight_id: Some(minute),
            started_at: format!("2026-11-27T10:{minute:02}:00.000Z"),
            duration_ms: Some(60_000),
            won: true,
            turns: 3,
            total_damage: 500,
            xp_gained: 1200,
            kamas_gained: Some(40),
            game_server: Some("pandora".to_string()),
            dungeon_id: None,
            dungeon_run_signature: None,
            challenges_passed: 1,
            challenges_failed: 0,
            participants,
            loot: vec![FightLootPayload {
                item_id: Some(10),
                item_name: None,
                quantity: 2,
            }],
        }),
    }
}

fn store() -> HistoryStore {
    HistoryStore::open_in_memory().unwrap()
}

fn record(store: &mut HistoryStore, events: &[SyncEvent]) -> RecordReport {
    let (catalog, dungeons) = (catalog(), dungeons());
    store
        .record(
            events,
            Referentiels {
                catalog: &catalog,
                dungeons: &dungeons,
            },
        )
        .unwrap()
}

fn fight_row(
    store: &HistoryStore,
    signature: &str,
) -> (Option<i64>, Option<String>, Option<String>) {
    store
        .connection()
        .query_row(
            "SELECT dungeon_id, dungeon_run_key, fight_type FROM fights WHERE signature = ?1",
            [signature],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap()
}

#[test]
fn reecrire_le_meme_lot_n_ajoute_rien() {
    let mut store = store();
    let events = vec![
        fight("f1", 1, &[("Bouftou", 1), ("Bouftou", 1)]),
        SyncEvent {
            kind: HistoryEventKind::Purchase,
            signature: "p1".to_string(),
            payload: HistoryPayload::Purchase(PurchasePayload {
                item_id: Some(10),
                item_name: None,
                quantity: 5,
                total_cost: 1500,
                occurred_at: "2026-11-27T11:00:00.000Z".to_string(),
                game_server: None,
            }),
        },
    ];
    let first = record(&mut store, &events);
    assert_eq!(first.fights_inserted, 1);
    assert_eq!(first.purchases_inserted, 1);
    let counts = store.counts().unwrap();
    for _ in 0..9 {
        let again = record(&mut store, &events);
        assert_eq!(again.fights_inserted, 0);
        assert_eq!(again.purchases_inserted, 0);
    }
    assert_eq!(store.counts().unwrap(), counts);
    // Deux Bouftous = deux lignes, plus l'allié.
    assert_eq!(counts.fight_participants, 3);
    assert_eq!(counts.fight_loot, 1);
}

#[test]
fn une_salle_ecrite_avant_son_boss_est_rattachee_au_run() {
    let mut store = store();
    // Deux salles gagnées, dans deux lots séparés (deux lancements de l'overlay)…
    record(&mut store, &[fight("salle1", 1, &[("Bouftou", 1)])]);
    record(&mut store, &[fight("salle2", 5, &[("Bouftou Noir", 3)])]);
    assert_eq!(fight_row(&store, "salle1").0, None);
    assert_eq!(
        fight_row(&store, "salle1").2.as_deref(),
        Some("FAMILY_10"),
        "une salle seule est d'abord un combat de famille"
    );
    // … puis le boss : les deux salles rejoignent son run, clé = signature du boss.
    let report = record(&mut store, &[fight("boss", 9, &[("Bouftou Royal", 2)])]);
    assert_eq!(report.dungeon_attached, 3);
    for signature in ["salle1", "salle2", "boss"] {
        let (dungeon_id, key, _) = fight_row(&store, signature);
        assert_eq!(dungeon_id, Some(100), "{signature}");
        assert_eq!(key.as_deref(), Some("boss"), "{signature}");
    }
    assert_eq!(
        fight_row(&store, "salle1").2.as_deref(),
        Some("DUNGEON_ROOM")
    );
    assert_eq!(
        fight_row(&store, "boss").2.as_deref(),
        Some("DUNGEON_THREE_ROOMS")
    );
}

#[test]
fn un_rattachement_n_est_jamais_efface_par_un_renvoi_sans_donjon() {
    let mut store = store();
    let mut with_dungeon = fight("boss", 9, &[("Bouftou Royal", 2)]);
    if let HistoryPayload::Fight(payload) = &mut with_dungeon.payload {
        payload.dungeon_id = Some(100);
        payload.dungeon_run_signature = Some("boss".to_string());
    }
    record(&mut store, &[with_dungeon]);
    record(&mut store, &[fight("boss", 9, &[("Bouftou Royal", 2)])]);
    assert_eq!(fight_row(&store, "boss").0, Some(100));
}

#[test]
fn les_objets_d_un_echange_sont_numerotes_par_direction() {
    let mut store = store();
    let item = |direction, name: &str| TradeItemPayload {
        direction,
        item_id: None,
        item_name: Some(name.to_string()),
        quantity: 1,
    };
    let trade = SyncEvent {
        kind: HistoryEventKind::Trade,
        signature: "t1".to_string(),
        payload: HistoryPayload::Trade(TradePayload {
            peer_name: "Anonyme-42".to_string(),
            self_name: "Anonyme-Iop1".to_string(),
            occurred_at: "2026-11-27T12:00:00.000Z".to_string(),
            kamas_acquired: 100,
            kamas_given: 0,
            game_server: None,
            items: vec![
                item(TradeDirection::Given, "A"),
                item(TradeDirection::Acquired, "B"),
                item(TradeDirection::Given, "C"),
            ],
        }),
    };
    record(&mut store, std::slice::from_ref(&trade));
    record(&mut store, &[trade]);
    let rows: Vec<(String, i64, String)> = store
        .connection()
        .prepare("SELECT direction, line_index, item_name FROM trade_items ORDER BY direction, line_index")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        rows,
        vec![
            ("acquired".to_string(), 0, "B".to_string()),
            ("given".to_string(), 0, "A".to_string()),
            ("given".to_string(), 1, "C".to_string()),
        ]
    );
}

#[test]
fn une_extraction_de_pacte_s_ecrit_une_fois() {
    let mut store = store();
    let pact = SyncEvent {
        kind: HistoryEventKind::Pact,
        signature: "pacte".to_string(),
        payload: HistoryPayload::PactExtraction(PactExtractionPayload {
            occurred_at: "2026-11-27T13:00:00.000Z".to_string(),
            game_server: Some("ogrest".to_string()),
            items: vec![
                PactExtractionItemPayload {
                    item_id: None,
                    item_name: Some("Eclat".to_string()),
                    quantity: 48,
                },
                PactExtractionItemPayload {
                    item_id: None,
                    item_name: Some("Pierre".to_string()),
                    quantity: 2,
                },
            ],
        }),
    };
    assert_eq!(
        record(&mut store, std::slice::from_ref(&pact)).pacts_inserted,
        1
    );
    assert_eq!(record(&mut store, &[pact]).pacts_inserted, 0);
    let counts = store.counts().unwrap();
    assert_eq!(
        (counts.pact_extractions, counts.pact_extraction_items),
        (1, 2)
    );
}

#[test]
fn un_horodatage_illisible_est_ecarte_sans_bloquer_le_lot() {
    let mut store = store();
    let mut bad = fight("mauvais", 1, &[("Bouftou", 1)]);
    if let HistoryPayload::Fight(payload) = &mut bad.payload {
        payload.started_at = "hier".to_string();
    }
    let report = record(&mut store, &[bad, fight("bon", 2, &[("Bouftou", 1)])]);
    assert_eq!((report.skipped, report.fights_inserted), (1, 1));
}

#[test]
fn la_base_se_rouvre_sans_rien_perdre() {
    let dir = std::env::temp_dir().join(format!(
        "overlay-history-reouverture-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join(FILE_NAME);
    {
        let mut store = HistoryStore::open(&path).unwrap();
        record(&mut store, &[fight("f1", 1, &[("Bouftou", 1)])]);
    }
    let store = HistoryStore::open(&path).unwrap();
    assert_eq!(store.counts().unwrap().fights, 1);
    let version: i64 = store
        .connection()
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION);
    drop(store);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn une_base_plus_recente_est_refusee() {
    let dir = std::env::temp_dir().join(format!("overlay-history-recente-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(FILE_NAME);
    {
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
    }
    assert!(matches!(
        HistoryStore::open(&path),
        Err(HistoryError::NewerSchema { .. })
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

mod import;
mod stats;

#[test]
fn un_combat_relu_porte_ses_combattants_et_son_butin() {
    let mut store = store();
    record(
        &mut store,
        &[fight("f1", 1, &[("Bouftou", 1), ("Bouftou", 1)])],
    );
    let fights = store.fights().unwrap();
    assert_eq!(fights.len(), 1);
    let fight = &fights[0];
    assert_eq!(fight.participants.len(), 3);
    assert_eq!(
        fight.participants.iter().filter(|p| p.is_ally).count(),
        1,
        "un allié, deux Bouftous"
    );
    assert_eq!(fight.loot.len(), 1);
}

#[test]
fn data_version_bouge_quand_une_autre_connexion_ecrit() {
    let dir = std::env::temp_dir().join(format!(
        "overlay-history-data-version-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join(FILE_NAME);
    let mut overlay = HistoryStore::open(&path).unwrap();
    let window = HistoryStore::open(&path).unwrap();
    let before = window.data_version().unwrap();
    assert_eq!(window.data_version().unwrap(), before, "rien n'a changé");
    record(&mut overlay, &[fight("f1", 1, &[("Bouftou", 1)])]);
    assert_ne!(window.data_version().unwrap(), before);
    drop((overlay, window));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn les_sorts_d_un_combattant_se_lisent_a_la_demande() {
    let mut store = store();
    let mut event = fight("f1", 1, &[("Bouftou", 1)]);
    if let HistoryPayload::Fight(payload) = &mut event.payload {
        payload.participants[0].spells = vec![
            overlay_engine::history::FightSpellPayload {
                spell: "Épée divine".to_string(),
                total: 120,
                by_element: [("Feu".to_string(), 120)].into(),
                by_turn: vec![
                    overlay_engine::history::FightSpellTurnPayload {
                        turn: 2,
                        total: 70,
                        by_element: [("Feu".to_string(), 70)].into(),
                    },
                    overlay_engine::history::FightSpellTurnPayload {
                        turn: 1,
                        total: 50,
                        by_element: [("Feu".to_string(), 50)].into(),
                    },
                ],
            },
            overlay_engine::history::FightSpellPayload {
                spell: "Super Iop Punch".to_string(),
                total: 380,
                by_element: [("Feu".to_string(), 300), ("Terre".to_string(), 80)].into(),
                by_turn: Vec::new(),
            },
        ];
    }
    record(&mut store, &[event]);
    let fight = &store.fights().unwrap()[0];
    let ally = fight.participants.iter().find(|p| p.is_ally).unwrap();
    let spells = store
        .participant_spells(
            fight.id,
            &ally.name,
            ally.instance_index,
            read::SpellColumn::Damage,
        )
        .unwrap();
    let names: Vec<&str> = spells.iter().map(|s| s.spell.as_str()).collect();
    assert_eq!(
        names,
        vec!["Super Iop Punch", "Épée divine"],
        "du plus gros au plus petit"
    );
    assert_eq!(spells[0].by_element.len(), 2);
    assert!(spells[0].by_turn.is_empty(), "sans détail par tour");
    let turns: Vec<(i64, i64)> = spells[1]
        .by_turn
        .iter()
        .map(|t| (t.turn, t.total))
        .collect();
    assert_eq!(turns, vec![(1, 50), (2, 70)], "par tour croissant");
    let all = store
        .fight_spells(fight.id, read::SpellColumn::Damage)
        .unwrap();
    assert_eq!(all.len(), 2, "l'allié et le Bouftou");
    assert!(store
        .participant_spells(
            fight.id,
            &ally.name,
            ally.instance_index,
            read::SpellColumn::Heal
        )
        .unwrap()
        .is_empty());
    assert!(store
        .participant_spells(fight.id, "Personne", 0, read::SpellColumn::Damage)
        .unwrap()
        .is_empty());
}
