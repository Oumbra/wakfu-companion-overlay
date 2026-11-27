//! Écriture idempotente des événements du moteur — port de `server/history/ingest.ts` (site),
//! sans ses garde-fous anti-abus (quota, sièges écrivables, pseudonymes retirés) qui n'ont pas de
//! sens pour une base qui ne reçoit que ce que l'overlay du poste produit lui-même.
//!
//! Mêmes règles de mise à jour que le serveur, parce qu'elles répondent au même log relu :
//! - un combat est **immuable** une fois écrit, sauf son rattachement de donjon (`dungeon_id`,
//!   `dungeon_run_key`) : la salle d'un run est produite avant le combat de boss qui révèle le
//!   donjon, puis renvoyée par le moteur une fois le run identifié. Une valeur reçue remplace,
//!   une absence n'efface jamais (`COALESCE`) ;
//! - les participants et le butin d'un combat sont réécrits à chaque envoi (mêmes valeurs pour un
//!   log relu à l'identique) ;
//! - achats, échanges et pactes : seule l'identification d'objet peut changer après coup.

use overlay_engine::history::{
    FightPayload, FightSide, HistoryPayload, PactExtractionPayload, PurchasePayload,
    TradeDirection, TradePayload,
};
use overlay_engine::log_time::parse_iso_utc_ms;
use overlay_engine::SyncEvent;
use rusqlite::{params, OptionalExtension, Transaction};

use crate::account::{adopt_imported, client_key};

/// Ce qu'un lot a changé — compté pour le journal et les tests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RecordReport {
    pub fights_inserted: usize,
    pub purchases_inserted: usize,
    pub trades_inserted: usize,
    pub pacts_inserted: usize,
    /// Événements écartés : horodatage illisible (ne devrait jamais arriver, le moteur l'écrit
    /// lui-même au format attendu).
    pub skipped: usize,
    /// Combats rattachés à un donjon par le recalcul des runs (`derive`).
    pub dungeon_attached: usize,
}

/// Écrit un lot et rend les ids des combats touchés (nouveaux comme déjà connus) : ce sont eux
/// dont le rattachement de donjon et le type sont recalculés ensuite.
///
/// `uid` : le compte du site, s'il est connu (`account`) — chaque ligne reçoit alors sa
/// `client_key`, et une ligne importée équivalente est adoptée plutôt que doublée.
pub(crate) fn record_events(
    tx: &Transaction<'_>,
    events: &[SyncEvent],
    uid: Option<&str>,
    report: &mut RecordReport,
) -> rusqlite::Result<Vec<i64>> {
    let mut touched_fights = Vec::new();
    for event in events {
        match &event.payload {
            HistoryPayload::Fight(fight) => {
                match write_fight(tx, &event.signature, fight, uid, report)? {
                    Some(id) => touched_fights.push(id),
                    None => report.skipped += 1,
                }
            }
            HistoryPayload::Purchase(purchase) => {
                if !write_purchase(tx, &event.signature, purchase, uid, report)? {
                    report.skipped += 1;
                }
            }
            HistoryPayload::Trade(trade) => {
                if !write_trade(tx, &event.signature, trade, uid, report)? {
                    report.skipped += 1;
                }
            }
            HistoryPayload::PactExtraction(pact) => {
                if !write_pact(tx, &event.signature, pact, uid, report)? {
                    report.skipped += 1;
                }
            }
        }
    }
    touched_fights.sort_unstable();
    touched_fights.dedup();
    Ok(touched_fights)
}

/// La `client_key` de l'événement, après adoption d'une éventuelle ligne importée équivalente.
fn account_key(
    tx: &Transaction<'_>,
    uid: Option<&str>,
    table: &str,
    kind: &str,
    signature: &str,
) -> rusqlite::Result<Option<String>> {
    match uid {
        Some(uid) => adopt_imported(tx, table, &client_key(uid, kind, signature), signature),
        None => Ok(None),
    }
}

fn timestamp(raw: &str) -> Option<i64> {
    let parsed = parse_iso_utc_ms(raw);
    if parsed.is_none() {
        tracing::warn!(%raw, "historique local : horodatage illisible, événement écarté");
    }
    parsed
}

fn write_fight(
    tx: &Transaction<'_>,
    signature: &str,
    fight: &FightPayload,
    uid: Option<&str>,
    report: &mut RecordReport,
) -> rusqlite::Result<Option<i64>> {
    let Some(started_at) = timestamp(&fight.started_at) else {
        return Ok(None);
    };
    let key = account_key(tx, uid, "fights", "fight", signature)?;
    let existing: Option<i64> = tx
        .query_row(
            "SELECT id FROM fights WHERE signature = ?1",
            [signature],
            |row| row.get(0),
        )
        .optional()?;
    let fight_id = match existing {
        Some(id) => {
            tx.execute(
                "UPDATE fights
                 SET dungeon_id = COALESCE(?2, dungeon_id),
                     dungeon_run_key = COALESCE(?3, dungeon_run_key),
                     client_key = COALESCE(client_key, ?4)
                 WHERE id = ?1",
                params![id, fight.dungeon_id, fight.dungeon_run_signature, key],
            )?;
            id
        }
        None => {
            tx.execute(
                "INSERT INTO fights (signature, fight_log_id, started_at, duration_ms, won, turns,
                    total_damage, xp_gained, kamas_gained, game_server, dungeon_id,
                    dungeon_run_key, challenges_passed, challenges_failed, client_key)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    signature,
                    fight.fight_id,
                    started_at,
                    fight.duration_ms,
                    fight.won,
                    fight.turns,
                    fight.total_damage,
                    fight.xp_gained,
                    fight.kamas_gained,
                    fight.game_server,
                    fight.dungeon_id,
                    fight.dungeon_run_signature,
                    fight.challenges_passed,
                    fight.challenges_failed,
                    key,
                ],
            )?;
            report.fights_inserted += 1;
            tx.last_insert_rowid()
        }
    };

    let mut participant = tx.prepare_cached(
        "INSERT INTO fight_participants (fight_id, name, instance_index, side, monster_id,
            class_name, damage, defeated, fled, spells, heal, armor, heal_spells, armor_spells,
            xp_gained)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT (fight_id, name, instance_index) DO UPDATE SET
            side = excluded.side, monster_id = excluded.monster_id,
            class_name = excluded.class_name, damage = excluded.damage,
            defeated = excluded.defeated, fled = excluded.fled, spells = excluded.spells,
            heal = excluded.heal, armor = excluded.armor, heal_spells = excluded.heal_spells,
            armor_spells = excluded.armor_spells, xp_gained = excluded.xp_gained",
    )?;
    for p in &fight.participants {
        let side = match p.side {
            FightSide::Ally => "ally",
            FightSide::Enemy => "enemy",
        };
        participant.execute(params![
            fight_id,
            p.name,
            p.instance_index,
            side,
            p.monster_id,
            p.class_name,
            p.damage,
            p.defeated,
            p.fled,
            to_json(&p.spells),
            p.heal,
            p.armor,
            to_json(&p.heal_spells),
            to_json(&p.armor_spells),
            p.xp_gained,
        ])?;
    }

    let mut loot = tx.prepare_cached(
        "INSERT INTO fight_loot (fight_id, line_index, item_id, item_name, quantity)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (fight_id, line_index) DO UPDATE SET
            item_id = excluded.item_id, item_name = excluded.item_name,
            quantity = excluded.quantity",
    )?;
    for (line_index, row) in fight.loot.iter().enumerate() {
        loot.execute(params![
            fight_id,
            line_index as i64,
            row.item_id,
            row.item_name,
            row.quantity
        ])?;
    }
    Ok(Some(fight_id))
}

fn to_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_string())
}

fn write_purchase(
    tx: &Transaction<'_>,
    signature: &str,
    purchase: &PurchasePayload,
    uid: Option<&str>,
    report: &mut RecordReport,
) -> rusqlite::Result<bool> {
    let Some(occurred_at) = timestamp(&purchase.occurred_at) else {
        return Ok(false);
    };
    let key = account_key(tx, uid, "purchases", "purchase", signature)?;
    let updated = tx.execute(
        "UPDATE purchases SET item_id = ?2, item_name = ?3,
            client_key = COALESCE(client_key, ?4)
         WHERE signature = ?1",
        params![signature, purchase.item_id, purchase.item_name, key],
    )?;
    if updated == 0 {
        tx.execute(
            "INSERT INTO purchases (signature, item_id, item_name, quantity, total_cost,
                occurred_at, game_server, client_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                signature,
                purchase.item_id,
                purchase.item_name,
                purchase.quantity,
                purchase.total_cost,
                occurred_at,
                purchase.game_server,
                key,
            ],
        )?;
        report.purchases_inserted += 1;
    }
    Ok(true)
}

fn write_trade(
    tx: &Transaction<'_>,
    signature: &str,
    trade: &TradePayload,
    uid: Option<&str>,
    report: &mut RecordReport,
) -> rusqlite::Result<bool> {
    let Some(occurred_at) = timestamp(&trade.occurred_at) else {
        return Ok(false);
    };
    let key = account_key(tx, uid, "trades", "trade", signature)?;
    let inserted = tx.execute(
        "INSERT INTO trades (signature, peer_name, self_name, occurred_at, kamas_acquired,
            kamas_given, game_server, client_key)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT (signature) DO NOTHING",
        params![
            signature,
            trade.peer_name,
            trade.self_name,
            occurred_at,
            trade.kamas_acquired,
            trade.kamas_given,
            trade.game_server,
            key,
        ],
    )?;
    report.trades_inserted += inserted;
    let trade_id: i64 = tx.query_row(
        "SELECT id FROM trades WHERE signature = ?1",
        [signature],
        |row| row.get(0),
    )?;
    let mut item = tx.prepare_cached(
        "INSERT INTO trade_items (trade_id, direction, line_index, item_id, item_name, quantity)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (trade_id, direction, line_index) DO UPDATE SET
            item_id = excluded.item_id, item_name = excluded.item_name",
    )?;
    let (mut acquired, mut given) = (0i64, 0i64);
    for row in &trade.items {
        let (direction, counter) = match row.direction {
            TradeDirection::Acquired => ("acquired", &mut acquired),
            TradeDirection::Given => ("given", &mut given),
        };
        item.execute(params![
            trade_id,
            direction,
            *counter,
            row.item_id,
            row.item_name,
            row.quantity
        ])?;
        *counter += 1;
    }
    Ok(true)
}

fn write_pact(
    tx: &Transaction<'_>,
    signature: &str,
    pact: &PactExtractionPayload,
    uid: Option<&str>,
    report: &mut RecordReport,
) -> rusqlite::Result<bool> {
    let Some(occurred_at) = timestamp(&pact.occurred_at) else {
        return Ok(false);
    };
    let key = account_key(tx, uid, "pact_extractions", "pact", signature)?;
    let inserted = tx.execute(
        "INSERT INTO pact_extractions (signature, occurred_at, game_server, client_key)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (signature) DO NOTHING",
        params![signature, occurred_at, pact.game_server, key],
    )?;
    report.pacts_inserted += inserted;
    let extraction_id: i64 = tx.query_row(
        "SELECT id FROM pact_extractions WHERE signature = ?1",
        [signature],
        |row| row.get(0),
    )?;
    let mut item = tx.prepare_cached(
        "INSERT INTO pact_extraction_items (extraction_id, line_index, item_id, item_name,
            quantity)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (extraction_id, line_index) DO UPDATE SET
            item_id = excluded.item_id, item_name = excluded.item_name",
    )?;
    for (line_index, row) in pact.items.iter().enumerate() {
        item.execute(params![
            extraction_id,
            line_index as i64,
            row.item_id,
            row.item_name,
            row.quantity
        ])?;
    }
    Ok(true)
}
