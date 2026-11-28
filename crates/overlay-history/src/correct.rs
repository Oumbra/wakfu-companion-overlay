//! **Corrections manuelles** d'un objet : quand le référentiel porte plusieurs objets du même nom
//! (raretés différentes, « Larme d'Ogrest »…), la résolution par nom seul ne sait pas lequel le
//! log désignait, et l'utilisateur corrige à la main depuis la fenêtre d'historique — comme le
//! clic droit du site (`ItemPickerService`, `PurchaseReassignService`).
//!
//! Une ligne corrigée porte `corrected = 1` : relire le même `wakfu.log` ne l'écrase plus (voir
//! `write`). Une ligne d'objets vidée par une correction reste en base à quantité nulle plutôt que
//! d'être supprimée — sinon la relecture du log la recréerait — et les lectures l'ignorent.

use rusqlite::types::ToSql;
use rusqlite::{params, params_from_iter, OptionalExtension, Transaction};

use crate::{HistoryError, HistoryStore};

/// Les lignes créées par une correction partielle prennent leur numéro à partir d'ici : jamais
/// celui d'une ligne que le log pourrait réécrire.
const SPLIT_LINE_BASE: i64 = 1_000_000;

/// Sens d'un objet d'échange (`trade_items.direction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeSide {
    Acquired,
    Given,
}

impl TradeSide {
    fn as_sql(self) -> &'static str {
        match self {
            TradeSide::Acquired => "acquired",
            TradeSide::Given => "given",
        }
    }
}

impl HistoryStore {
    /// Passe `quantity` exemplaires de l'objet (`item_id`, `item_name`) du butin du combat
    /// `fight_id` sur l'objet `new_id` — tout le lot ou une partie (« 2 des 5 étaient l'autre
    /// variante »). Ils rejoignent la ligne de `new_id` si le combat en a déjà une. Rend le nombre
    /// d'exemplaires effectivement déplacés.
    pub fn reassign_fight_loot(
        &self,
        fight_id: i64,
        item_id: Option<i64>,
        item_name: Option<&str>,
        quantity: i64,
        new_id: i64,
    ) -> Result<i64, HistoryError> {
        let tx = self.conn.unchecked_transaction()?;
        let moved = reassign_lines(
            &tx,
            "fight_loot",
            &[("fight_id", &fight_id)],
            (item_id, item_name),
            quantity,
            new_id,
        )?;
        tx.commit()?;
        Ok(moved)
    }

    /// Même correction pour un objet d'un échange, dans son sens (acquis ou cédé) — le site passe
    /// par le même menu (`TradesComponent`).
    pub fn reassign_trade_item(
        &self,
        trade_id: i64,
        direction: TradeSide,
        item_id: Option<i64>,
        item_name: Option<&str>,
        quantity: i64,
        new_id: i64,
    ) -> Result<i64, HistoryError> {
        let tx = self.conn.unchecked_transaction()?;
        let moved = reassign_lines(
            &tx,
            "trade_items",
            &[("trade_id", &trade_id), ("direction", &direction.as_sql())],
            (item_id, item_name),
            quantity,
            new_id,
        )?;
        tx.commit()?;
        Ok(moved)
    }

    /// Passe les achats `ids` sur l'objet `new_id`, chacun en bloc avec sa quantité et son coût
    /// (la modale du site coche les lignes d'achat à réattribuer). Rend le nombre d'achats
    /// modifiés.
    pub fn reassign_purchases(&self, ids: &[i64], new_id: i64) -> Result<usize, HistoryError> {
        let tx = self.conn.unchecked_transaction()?;
        let mut changed = 0;
        {
            let mut update = tx.prepare(
                "UPDATE purchases SET item_id = ?2, corrected = 1
                 WHERE id = ?1 AND item_id IS NOT ?2",
            )?;
            for id in ids {
                changed += update.execute(params![id, new_id])?;
            }
        }
        tx.commit()?;
        Ok(changed)
    }
}

/// Le cœur commun : dans `table`, parmi les lignes de `owner` (colonnes = valeurs) qui portent
/// `item`, retire `quantity` exemplaires dans l'ordre des lignes et les ajoute à la ligne de
/// `new_id` (créée au besoin). Rend le nombre d'exemplaires déplacés.
fn reassign_lines(
    tx: &Transaction<'_>,
    table: &str,
    owner: &[(&str, &dyn ToSql)],
    item: (Option<i64>, Option<&str>),
    quantity: i64,
    new_id: i64,
) -> Result<i64, HistoryError> {
    let (item_id, item_name) = item;
    if item_id == Some(new_id) || quantity <= 0 {
        return Ok(0);
    }
    // `col1 = ?1 AND col2 = ?2 …`, puis les paramètres propres à chaque requête à la suite.
    let scope = owner
        .iter()
        .enumerate()
        .map(|(index, (column, _))| format!("{column} = ?{}", index + 1))
        .collect::<Vec<_>>()
        .join(" AND ");
    let next = owner.len() + 1;

    let lines: Vec<(i64, i64)> = tx
        .prepare(&format!(
            "SELECT line_index, quantity FROM {table}
             WHERE {scope} AND item_id IS ?{next} AND item_name IS ?{} AND quantity > 0
             ORDER BY line_index",
            next + 1
        ))?
        .query_map(
            params_from_iter(bind(owner, &[&item_id, &item_name])),
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?
        .collect::<Result<_, _>>()?;
    let mut left = quantity;
    for (line_index, line_quantity) in lines {
        if left == 0 {
            break;
        }
        let taken = line_quantity.min(left);
        tx.execute(
            &format!(
                "UPDATE {table} SET quantity = quantity - ?{}, corrected = 1
                 WHERE {scope} AND line_index = ?{next}",
                next + 1
            ),
            params_from_iter(bind(owner, &[&line_index, &taken])),
        )?;
        left -= taken;
    }
    let moved = quantity - left;
    if moved == 0 {
        return Ok(0);
    }
    let target: Option<i64> = tx
        .query_row(
            &format!(
                "SELECT line_index FROM {table}
                 WHERE {scope} AND item_id = ?{next} AND item_name IS ?{}
                 ORDER BY line_index LIMIT 1",
                next + 1
            ),
            params_from_iter(bind(owner, &[&new_id, &item_name])),
            |row| row.get(0),
        )
        .optional()?;
    match target {
        Some(line_index) => {
            tx.execute(
                &format!(
                    "UPDATE {table} SET quantity = quantity + ?{}, corrected = 1
                     WHERE {scope} AND line_index = ?{next}",
                    next + 1
                ),
                params_from_iter(bind(owner, &[&line_index, &moved])),
            )?;
        }
        None => {
            let line_index: i64 = tx.query_row(
                &format!(
                    "SELECT MAX(?{next}, COALESCE(MAX(line_index), 0) + 1) FROM {table}
                     WHERE {scope}"
                ),
                params_from_iter(bind(owner, &[&SPLIT_LINE_BASE])),
                |row| row.get(0),
            )?;
            let columns = owner
                .iter()
                .map(|(column, _)| *column)
                .collect::<Vec<_>>()
                .join(", ");
            let placeholders = (1..=owner.len() + 4)
                .map(|index| format!("?{index}"))
                .collect::<Vec<_>>()
                .join(", ");
            tx.execute(
                &format!(
                    "INSERT INTO {table}
                        ({columns}, line_index, item_id, item_name, quantity, corrected)
                     VALUES ({placeholders}, 1)"
                ),
                params_from_iter(bind(owner, &[&line_index, &new_id, &item_name, &moved])),
            )?;
        }
    }
    Ok(moved)
}

/// Les valeurs de `owner`, puis `extra` : les paramètres d'une requête de [`reassign_lines`].
fn bind<'a>(owner: &[(&str, &'a dyn ToSql)], extra: &[&'a dyn ToSql]) -> Vec<&'a dyn ToSql> {
    owner
        .iter()
        .map(|(_, value)| *value)
        .chain(extra.iter().copied())
        .collect()
}
