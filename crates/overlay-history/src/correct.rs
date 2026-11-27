//! **Corrections manuelles** d'un objet : quand le référentiel porte plusieurs objets du même nom
//! (raretés différentes, « Larme d'Ogrest »…), la résolution par nom seul ne sait pas lequel le
//! log désignait, et l'utilisateur corrige à la main depuis la fenêtre d'historique — comme le
//! clic droit du site (`ItemPickerService`, `PurchaseReassignService`).
//!
//! Une ligne corrigée porte `corrected = 1` : relire le même `wakfu.log` ne l'écrase plus (voir
//! `write`). Une ligne de butin vidée par une correction reste en base à quantité nulle plutôt que
//! d'être supprimée — sinon la relecture du log la recréerait — et les lectures l'ignorent.

use rusqlite::{params, OptionalExtension};

use crate::{HistoryError, HistoryStore};

/// Les lignes de butin créées par une correction partielle prennent leur numéro à partir d'ici :
/// jamais celui d'une ligne que le log pourrait réécrire.
const SPLIT_LINE_BASE: i64 = 1_000_000;

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
        if item_id == Some(new_id) || quantity <= 0 {
            return Ok(0);
        }
        let tx = self.conn.unchecked_transaction()?;
        let lines: Vec<(i64, i64)> = tx
            .prepare(
                "SELECT line_index, quantity FROM fight_loot
                 WHERE fight_id = ?1 AND item_id IS ?2 AND item_name IS ?3 AND quantity > 0
                 ORDER BY line_index",
            )?
            .query_map(params![fight_id, item_id, item_name], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<Result<_, _>>()?;
        let mut left = quantity;
        for (line_index, line_quantity) in lines {
            if left == 0 {
                break;
            }
            let taken = line_quantity.min(left);
            tx.execute(
                "UPDATE fight_loot SET quantity = quantity - ?3, corrected = 1
                 WHERE fight_id = ?1 AND line_index = ?2",
                params![fight_id, line_index, taken],
            )?;
            left -= taken;
        }
        let moved = quantity - left;
        if moved > 0 {
            let target: Option<i64> = tx
                .query_row(
                    "SELECT line_index FROM fight_loot
                     WHERE fight_id = ?1 AND item_id = ?2 AND item_name IS ?3
                     ORDER BY line_index LIMIT 1",
                    params![fight_id, new_id, item_name],
                    |row| row.get(0),
                )
                .optional()?;
            match target {
                Some(line_index) => {
                    tx.execute(
                        "UPDATE fight_loot SET quantity = quantity + ?3, corrected = 1
                         WHERE fight_id = ?1 AND line_index = ?2",
                        params![fight_id, line_index, moved],
                    )?;
                }
                None => {
                    let line_index: i64 = tx.query_row(
                        "SELECT MAX(?2, COALESCE(MAX(line_index), 0) + 1) FROM fight_loot
                         WHERE fight_id = ?1",
                        params![fight_id, SPLIT_LINE_BASE],
                        |row| row.get(0),
                    )?;
                    tx.execute(
                        "INSERT INTO fight_loot
                            (fight_id, line_index, item_id, item_name, quantity, corrected)
                         VALUES (?1, ?2, ?3, ?4, ?5, 1)",
                        params![fight_id, line_index, new_id, item_name, moved],
                    )?;
                }
            }
        }
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
