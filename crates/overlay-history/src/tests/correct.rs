//! Corrections manuelles d'un objet (`correct`) : butin d'un combat, achats.

use super::*;

/// Le butin du combat `f1` : `(item_id, quantité)` des lignes non vides, dans l'ordre.
fn loot(store: &HistoryStore) -> Vec<(Option<i64>, i64)> {
    store.fights().unwrap()[0]
        .loot
        .iter()
        .map(|line| (line.item_id, line.quantity))
        .collect()
}

fn fight_id(store: &HistoryStore) -> i64 {
    store.fights().unwrap()[0].id
}

#[test]
fn corriger_tout_le_lot_change_l_objet_de_la_ligne() {
    let mut store = store();
    record(&mut store, &[fight("f1", 1, &[("Bouftou", 1)])]);
    let id = fight_id(&store);
    assert_eq!(
        store
            .reassign_fight_loot(id, Some(10), None, 2, 11)
            .unwrap(),
        2
    );
    assert_eq!(loot(&store), vec![(Some(11), 2)]);
}

#[test]
fn corriger_une_partie_du_lot_ajoute_une_ligne() {
    let mut store = store();
    record(&mut store, &[fight("f1", 1, &[("Bouftou", 1)])]);
    let id = fight_id(&store);
    assert_eq!(
        store
            .reassign_fight_loot(id, Some(10), None, 1, 11)
            .unwrap(),
        1
    );
    assert_eq!(loot(&store), vec![(Some(10), 1), (Some(11), 1)]);
    // Le reste rejoint la ligne déjà corrigée au lieu d'en créer une troisième.
    assert_eq!(
        store
            .reassign_fight_loot(id, Some(10), None, 5, 11)
            .unwrap(),
        1
    );
    assert_eq!(loot(&store), vec![(Some(11), 2)]);
}

#[test]
fn relire_le_log_n_efface_pas_une_correction() {
    let mut store = store();
    let events = [fight("f1", 1, &[("Bouftou", 1)])];
    record(&mut store, &events);
    let id = fight_id(&store);
    store
        .reassign_fight_loot(id, Some(10), None, 1, 11)
        .unwrap();
    record(&mut store, &events);
    assert_eq!(
        loot(&store),
        vec![(Some(10), 1), (Some(11), 1)],
        "la ligne corrigée garde sa quantité, la ligne vidée ne revient pas"
    );
}

#[test]
fn corriger_un_achat_resiste_a_la_relecture() {
    let mut store = store();
    let purchase = SyncEvent {
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
    };
    record(&mut store, std::slice::from_ref(&purchase));
    let id = store.purchases().unwrap()[0].id;
    assert_eq!(store.reassign_purchases(&[id], 11).unwrap(), 1);
    assert_eq!(store.reassign_purchases(&[id], 11).unwrap(), 0, "déjà fait");
    record(&mut store, &[purchase]);
    assert_eq!(store.purchases().unwrap()[0].item_id, Some(11));
}
