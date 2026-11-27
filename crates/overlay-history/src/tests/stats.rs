//! Agrégats d'une période (`stats`, port de `stats.ts`) : les quatre règles du serveur.

use overlay_engine::history::{HistoryEventKind, HistoryPayload, PurchasePayload};
use overlay_engine::{CatalogIndex, SyncEvent};

use super::{dungeons, fight, record, store};
use crate::stats::HDV_KAMAS_SALE_ITEM;

/// Le catalogue des tests de base, plus un monstre d'Incarnam (famille exclue).
fn catalog() -> CatalogIndex {
    CatalogIndex::from_json(&serde_json::json!({
        "items": [[10, "Laine de Bouftou", 1, 1, 0, 3]],
        "monsters": [
            [1, "Bouftou", 1, 10, 0, 0, 0],
            [2, "Bouftou Royal", 2, 10, 1, 0, 0],
            [3, "Bouftou Noir", 3, 10, 0, 0, 0],
            [4, "Sac à patates", 4, 161, 0, 0, 0],
        ],
    }))
}

fn purchase(signature: &str, item_name: Option<&str>, total_cost: i64) -> SyncEvent {
    SyncEvent {
        kind: HistoryEventKind::Purchase,
        signature: signature.to_string(),
        payload: HistoryPayload::Purchase(PurchasePayload {
            item_id: None,
            item_name: item_name.map(str::to_string),
            quantity: 1,
            total_cost,
            occurred_at: "2026-11-27T11:00:00.000Z".to_string(),
            game_server: None,
        }),
    }
}

/// Toute la journée du 27 novembre 2026 (UTC).
const DAY: (i64, i64) = (1_795_737_600_000, 1_795_824_000_000);

#[test]
fn les_totaux_suivent_les_regles_du_site() {
    let mut store = store();
    record(
        &mut store,
        &[
            // Un run : deux salles, puis le boss (rattachement recalculé localement).
            fight("salle1", 1, &[("Bouftou", 1)]),
            fight("salle2", 5, &[("Bouftou Noir", 3)]),
            fight("boss", 9, &[("Bouftou Royal", 2)]),
            // Hors donjon : un combat de famille 10…
            fight("famille", 20, &[("Bouftou", 1)]),
            // … et un combat d'Incarnam, exclu de tout.
            fight("incarnam", 30, &[("Sac à patates", 4), ("Bouftou", 1)]),
            purchase("achat", Some("Laine"), 300),
            purchase("vente", Some(HDV_KAMAS_SALE_ITEM), 5_000),
        ],
    );
    let catalog = catalog();
    let stats = store.period_stats(DAY.0, DAY.1, &catalog).unwrap();

    // Règle 1 : quatre combats comptés sur cinq.
    assert_eq!((stats.combats.won, stats.combats.lost), (4, 0));
    assert_eq!(stats.kamas.from_combat, 4 * 40);
    assert_eq!(stats.xp_by_character.len(), 1);
    assert_eq!(stats.xp_by_character[0].amount, 4 * 1200);
    assert_eq!(stats.loot.len(), 1);
    assert_eq!(stats.loot[0].quantity, 4 * 2);

    // Règle 2 : le donjon compte trois combats, mais un seul run (le boss).
    assert_eq!(stats.dungeons.len(), 1);
    let (dungeon_id, run) = &stats.dungeons[0];
    assert_eq!(*dungeon_id, 100);
    assert_eq!(
        (run.fights, run.dungeon_runs, run.won, run.lost),
        (3, 1, 1, 0)
    );
    assert_eq!(run.xp_gained, 3 * 1200);

    // Règle 3 : le combat hors donjon sous sa famille.
    assert_eq!(stats.families.len(), 1);
    assert_eq!(stats.families[0].0, Some(10));
    assert_eq!(stats.families[0].1.fights, 1);

    // Règle 4 : achats et ventes séparés.
    assert_eq!(
        (stats.kamas.spent_on_purchases, stats.kamas.from_hdv_sales),
        (300, 5_000)
    );

    // Hors de la période : rien.
    let empty = store.period_stats(0, DAY.0, &catalog).unwrap();
    assert_eq!(empty, Default::default());
    let _ = dungeons();
}
