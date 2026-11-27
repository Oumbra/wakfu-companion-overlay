//! Import de l'historique en ligne (plan §8) : pages du site, reconnaissance par clé du site dans
//! les deux sens, curseurs.

use overlay_engine::history::HistoryEventKind;
use rusqlite::Connection;

use super::{fight, fight_row, record, store};
use crate::import::{import_all, ImportError, ImportKind};
use crate::{client_key, schema, HistoryStore, FILE_NAME, IMPORTED_SIGNATURE_PREFIX};

const UID: &str = "8f0c2d4e-compte";

fn key(kind: &str, signature: &str) -> String {
    client_key(UID, kind, signature)
}

/// Une entrée de `GET /api/v1/history/fights`, telle que le site la rend.
fn server_fight(client_key: &str, minute: i64, dungeon: Option<(i64, &str)>) -> serde_json::Value {
    serde_json::json!({
        "clientKey": client_key,
        "startedAt": format!("2026-11-20T10:{minute:02}:00.000Z"),
        "durationMs": 60000,
        "won": true,
        "turns": 3,
        "totalDamage": 500,
        "xpGained": 1200,
        "kamasGained": 40,
        "gameServer": "pandora",
        "dungeonId": dungeon.map(|(id, _)| id),
        "dungeonRunKey": dungeon.map(|(_, run)| run),
        "challengesPassed": 1,
        "challengesFailed": 0,
        "fightType": dungeon.map(|_| "DUNGEON_THREE_ROOMS"),
        "participants": [
            {"side": "ally", "name": "Anonyme-Iop1", "monsterId": null, "instanceIndex": 0,
             "className": "Iop", "damage": 500, "defeated": false, "fled": false,
             "spells": [{"spell": "Épée", "total": 500, "byElement": {"Air": 500}}],
             "heal": 0, "armor": 0, "healSpells": [], "armorSpells": [], "xpGained": 1200},
            {"side": "enemy", "name": "Bouftou", "monsterId": 1, "instanceIndex": 0,
             "className": null, "damage": 100, "defeated": true, "fled": false,
             "spells": [], "heal": 0, "armor": 0, "healSpells": [], "armorSpells": [],
             "xpGained": 0},
        ],
        "loot": [{"lineIndex": 0, "itemId": 10, "itemName": null, "quantity": 2}],
    })
}

fn page(entries: Vec<serde_json::Value>, next: Option<&str>) -> serde_json::Value {
    serde_json::json!({"entries": entries, "nextBefore": next, "maxBatch": 500})
}

fn signatures(store: &HistoryStore) -> Vec<String> {
    store
        .connection()
        .prepare("SELECT signature FROM fights ORDER BY started_at")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn fight_key(store: &HistoryStore) -> String {
    store
        .connection()
        .query_row("SELECT client_key FROM fights", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn la_cle_du_site_est_celle_de_la_file_d_envoi() {
    assert_eq!(
        client_key(UID, "fight", "10:01:00,000|3|won"),
        overlay_sync::client_key(UID, HistoryEventKind::Fight, "10:01:00,000|3|won")
    );
    assert_eq!(
        client_key(UID, "pact", "x"),
        overlay_sync::client_key(UID, HistoryEventKind::Pact, "x")
    );
}

#[test]
fn reimporter_une_page_n_ajoute_rien() {
    let mut store = store();
    let body = page(
        vec![server_fight("k1", 1, None), server_fight("k2", 2, None)],
        Some("2026-11-20T10:02:00.000Z~7"),
    );
    let (report, next) = store.import_page(ImportKind::Fights, &body).unwrap();
    assert_eq!((report.inserted, report.present), (2, 0));
    assert_eq!(next.as_deref(), Some("2026-11-20T10:02:00.000Z~7"));
    let (again, _) = store.import_page(ImportKind::Fights, &body).unwrap();
    assert_eq!((again.inserted, again.present), (0, 2));
    let counts = store.counts().unwrap();
    assert_eq!(
        (counts.fights, counts.fight_participants, counts.fight_loot),
        (2, 4, 2)
    );
    let spells: String = store
        .connection()
        .query_row(
            "SELECT spells FROM fight_participants WHERE name = 'Anonyme-Iop1' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(spells.contains("byElement"), "{spells}");
}

#[test]
fn un_combat_deja_lu_dans_le_log_n_est_pas_reimporte() {
    let mut store = store();
    // Le log a déjà produit `f1` avant que le compte soit connu…
    record(&mut store, &[fight("f1", 1, &[("Bouftou", 1)])]);
    // … le compte arrive : la ligne reçoit sa clé du site.
    let account = store.set_account_uid(UID).unwrap();
    assert_eq!((account.keyed, account.merged), (1, 0));
    // L'import rend ce même combat (même clé) et un autre.
    let body = page(
        vec![
            server_fight(&key("fight", "f1"), 1, Some((100, "migrated:1"))),
            server_fight(&key("fight", "f-ancien"), 2, None),
        ],
        None,
    );
    let (report, _) = store.import_page(ImportKind::Fights, &body).unwrap();
    assert_eq!((report.inserted, report.present), (1, 1));
    assert_eq!(store.counts().unwrap().fights, 2);
    // Le rattachement que le site connaissait complète la ligne locale.
    let (dungeon, run, fight_type) = fight_row(&store, "f1");
    assert_eq!(dungeon, Some(100));
    assert_eq!(run.as_deref(), Some("migrated:1"));
    assert_eq!(fight_type.as_deref(), Some("DUNGEON_THREE_ROOMS"));
}

#[test]
fn un_combat_importe_puis_relu_dans_le_log_prend_sa_vraie_signature() {
    let mut store = store();
    store.set_account_uid(UID).unwrap();
    let body = page(vec![server_fight(&key("fight", "f1"), 1, None)], None);
    store.import_page(ImportKind::Fights, &body).unwrap();
    assert!(signatures(&store)[0].starts_with(IMPORTED_SIGNATURE_PREFIX));
    let report = record(&mut store, &[fight("f1", 1, &[("Bouftou", 1)])]);
    assert_eq!(report.fights_inserted, 0, "adopté, pas doublé");
    assert_eq!(signatures(&store), vec!["f1".to_string()]);
    // Puis le log relu à l'identique : toujours rien de neuf.
    let again = record(&mut store, &[fight("f1", 1, &[("Bouftou", 1)])]);
    assert_eq!(again.fights_inserted, 0);
    assert_eq!(store.counts().unwrap().fights, 1);
    assert_eq!(fight_key(&store), key("fight", "f1"));
}

#[test]
fn un_doublon_importe_avant_que_le_compte_soit_connu_est_fondu() {
    let mut store = store();
    record(&mut store, &[fight("f1", 1, &[("Bouftou", 1)])]);
    // Import sans compte connu : la ligne locale n'a pas de clé, le combat est doublé…
    let body = page(
        vec![server_fight(&key("fight", "f1"), 1, Some((100, "run")))],
        None,
    );
    store.import_page(ImportKind::Fights, &body).unwrap();
    assert_eq!(store.counts().unwrap().fights, 2);
    // … jusqu'à ce que le compte soit posé : la copie importée est fondue dans la ligne locale.
    let account = store.set_account_uid(UID).unwrap();
    assert_eq!(account.merged, 1);
    assert_eq!(signatures(&store), vec!["f1".to_string()]);
    assert_eq!(fight_row(&store, "f1").0, Some(100));
    assert_eq!(
        store.counts().unwrap().fight_participants,
        2,
        "les participants de la copie partent avec elle"
    );
}

#[test]
fn achats_echanges_et_pactes_s_importent_avec_leurs_objets() {
    let mut store = store();
    let purchases = page(
        vec![serde_json::json!({
            "clientKey": "a1", "itemId": 10, "itemName": null, "quantity": 5,
            "totalCost": 1500, "occurredAt": "2026-11-20T11:00:00.000Z", "gameServer": null,
        })],
        None,
    );
    let trades = page(
        vec![serde_json::json!({
            "clientKey": "e1", "peerName": "Anonyme-42", "selfName": "Anonyme-Iop1",
            "occurredAt": "2026-11-20T12:00:00.000Z", "kamasAcquired": 100, "kamasGiven": 0,
            "gameServer": "pandora",
            "acquired": [{"itemId": null, "itemName": "B", "quantity": 1}],
            "given": [{"itemId": null, "itemName": "A", "quantity": 1},
                      {"itemId": 10, "itemName": null, "quantity": 3}],
        })],
        None,
    );
    let pacts = page(
        vec![serde_json::json!({
            "clientKey": "p1", "occurredAt": "2026-11-20T13:00:00.000Z", "gameServer": "ogrest",
            "items": [{"itemId": null, "itemName": "Eclat", "quantity": 48}],
        })],
        None,
    );
    for _ in 0..2 {
        store
            .import_page(ImportKind::Purchases, &purchases)
            .unwrap();
        store.import_page(ImportKind::Trades, &trades).unwrap();
        store.import_page(ImportKind::Pacts, &pacts).unwrap();
    }
    let counts = store.counts().unwrap();
    assert_eq!(
        (
            counts.purchases,
            counts.trades,
            counts.trade_items,
            counts.pact_extractions,
            counts.pact_extraction_items
        ),
        (1, 1, 3, 1, 1)
    );
}

#[test]
fn une_entree_illisible_est_ecartee_sans_bloquer_la_page() {
    let mut store = store();
    let mut bad = server_fight("k1", 1, None);
    bad["startedAt"] = serde_json::json!("hier");
    let mut incomplete = server_fight("k2", 2, None);
    incomplete.as_object_mut().unwrap().remove("won");
    let body = page(vec![bad, incomplete, server_fight("k3", 3, None)], None);
    let (report, _) = store.import_page(ImportKind::Fights, &body).unwrap();
    assert_eq!((report.inserted, report.skipped), (1, 2));
}

#[test]
fn un_combat_importe_sans_type_est_classe_a_la_prochaine_ecriture() {
    let mut store = store();
    let body = page(vec![server_fight("k1", 1, None)], None);
    store.import_page(ImportKind::Fights, &body).unwrap();
    let type_of_k1 = |store: &HistoryStore| -> Option<String> {
        store
            .connection()
            .query_row(
                "SELECT fight_type FROM fights WHERE client_key = 'k1'",
                [],
                |row| row.get(0),
            )
            .unwrap()
    };
    assert_eq!(type_of_k1(&store), None);
    record(&mut store, &[fight("f1", 30, &[("Bouftou", 1)])]);
    assert_eq!(type_of_k1(&store).as_deref(), Some("FAMILY_10"));
}

#[derive(Debug, thiserror::Error)]
#[error("panne")]
struct Panne;

#[test]
fn l_import_suit_les_curseurs_de_chaque_type() {
    let dir = std::env::temp_dir().join(format!("overlay-history-import-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join(FILE_NAME);
    let mut calls = Vec::new();
    let mut pages_seen = 0;
    let total = import_all(
        || HistoryStore::open(&path),
        |kind, before| {
            calls.push((kind, before.map(str::to_string)));
            Ok::<_, Panne>(match (kind, before) {
                (ImportKind::Fights, None) => page(vec![server_fight("k1", 1, None)], Some("c1")),
                (ImportKind::Fights, Some("c1")) => {
                    page(vec![server_fight("k2", 2, None)], Some("c2"))
                }
                // Curseur qui ne bouge plus : l'import s'arrête au lieu de boucler.
                (ImportKind::Fights, Some("c2")) => page(Vec::new(), Some("c2")),
                _ => page(Vec::new(), None),
            })
        },
        |_| pages_seen += 1,
    )
    .unwrap();
    assert_eq!(total.inserted, 2);
    assert_eq!(pages_seen, 6, "3 pages de combats + 1 par autre type");
    assert_eq!(calls[1], (ImportKind::Fights, Some("c1".to_string())));
    assert_eq!(calls[3].0, ImportKind::Purchases);

    let mut store = HistoryStore::open(&path).unwrap();
    assert!(!store.import_done(UID).unwrap());
    store.mark_import_done(UID, 1).unwrap();
    assert!(store.import_done(UID).unwrap());
    assert!(!store.import_done("autre").unwrap());
    drop(store);

    // Une panne réseau remonte telle quelle ; ce qui est écrit reste.
    let failed = import_all(
        || HistoryStore::open(&path),
        |_, _| Err::<serde_json::Value, _>(Panne),
        |_| {},
    );
    assert!(matches!(failed, Err(ImportError::Fetch(Panne))));
    assert_eq!(
        HistoryStore::open(&path).unwrap().counts().unwrap().fights,
        2
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn une_base_de_la_version_1_passe_a_la_version_2_sans_rien_perdre() {
    let dir = std::env::temp_dir().join(format!("overlay-history-v1-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(FILE_NAME);
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(schema::MIGRATIONS[0]).unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute(
            "INSERT INTO fights (signature, started_at, won) VALUES ('ancien', 0, 1)",
            [],
        )
        .unwrap();
    }
    let mut store = HistoryStore::open(&path).unwrap();
    assert_eq!(store.counts().unwrap().fights, 1);
    assert_eq!(store.set_account_uid(UID).unwrap().keyed, 1);
    assert_eq!(fight_key(&store), key("fight", "ancien"));
    drop(store);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn changer_de_compte_recalcule_les_cles_locales() {
    let mut store = store();
    record(&mut store, &[fight("f1", 1, &[("Bouftou", 1)])]);
    store.set_account_uid(UID).unwrap();
    assert_eq!(
        store.set_account_uid(UID).unwrap().keyed,
        0,
        "rien à refaire"
    );
    assert_eq!(store.set_account_uid("autre").unwrap().keyed, 1);
    assert_eq!(fight_key(&store), client_key("autre", "fight", "f1"));
}
