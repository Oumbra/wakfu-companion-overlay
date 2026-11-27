//! **Lecture des listes** de la fenêtre d'historique (phase 3) : achats, échanges, pactes.
//!
//! Ces trois historiques restent petits (quelques centaines de lignes par an) : ils sont lus en
//! entier, du plus récent au plus ancien, et la vue filtre, groupe et trie en mémoire — comme le
//! site, qui recevait tout l'historique avant d'en faire ses groupes par jour. Les combats, eux,
//! auront leur lecture propre.
//!
//! **Égalité d'horodatage** (même milliseconde, cas réel : une ligne d'achat lue deux fois par un
//! ancien parseur) : départagée par `id` **croissant**. Le site triait par son id décroissant, et
//! l'import écrit les pages du plus récent au plus ancien — l'ordre des id locaux est donc
//! l'inverse de celui du serveur, et l'ordre affiché reste celui du site.

use rusqlite::params;

use crate::{HistoryError, HistoryStore};

/// Un achat à l'Hôtel de Vente, ou une récupération de kamas
/// ([`crate::stats::HDV_KAMAS_SALE_ITEM`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseRow {
    pub id: i64,
    pub item_id: Option<i64>,
    pub item_name: Option<String>,
    pub quantity: i64,
    pub total_cost: i64,
    /// ms UTC.
    pub occurred_at: i64,
    pub game_server: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLine {
    pub item_id: Option<i64>,
    pub item_name: Option<String>,
    pub quantity: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeRow {
    pub id: i64,
    pub peer_name: String,
    pub self_name: String,
    pub occurred_at: i64,
    pub kamas_acquired: i64,
    pub kamas_given: i64,
    pub game_server: Option<String>,
    pub acquired: Vec<ItemLine>,
    pub given: Vec<ItemLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PactRow {
    pub id: i64,
    pub occurred_at: i64,
    pub game_server: Option<String>,
    pub items: Vec<ItemLine>,
}

impl HistoryStore {
    /// Tous les achats, du plus récent au plus ancien.
    pub fn purchases(&self) -> Result<Vec<PurchaseRow>, HistoryError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, item_id, item_name, quantity, total_cost, occurred_at, game_server
             FROM purchases ORDER BY occurred_at DESC, id",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(PurchaseRow {
                    id: row.get(0)?,
                    item_id: row.get(1)?,
                    item_name: row.get(2)?,
                    quantity: row.get(3)?,
                    total_cost: row.get(4)?,
                    occurred_at: row.get(5)?,
                    game_server: row.get(6)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// Tous les échanges, du plus récent au plus ancien, avec leurs objets dans l'ordre du log.
    pub fn trades(&self) -> Result<Vec<TradeRow>, HistoryError> {
        let mut trades: Vec<TradeRow> = self
            .conn
            .prepare_cached(
                "SELECT id, peer_name, self_name, occurred_at, kamas_acquired, kamas_given,
                    game_server
                 FROM trades ORDER BY occurred_at DESC, id",
            )?
            .query_map([], |row| {
                Ok(TradeRow {
                    id: row.get(0)?,
                    peer_name: row.get(1)?,
                    self_name: row.get(2)?,
                    occurred_at: row.get(3)?,
                    kamas_acquired: row.get(4)?,
                    kamas_given: row.get(5)?,
                    game_server: row.get(6)?,
                    acquired: Vec::new(),
                    given: Vec::new(),
                })
            })?
            .collect::<Result<_, _>>()?;
        let index: std::collections::HashMap<i64, usize> = trades
            .iter()
            .enumerate()
            .map(|(position, trade)| (trade.id, position))
            .collect();
        let mut stmt = self.conn.prepare_cached(
            "SELECT trade_id, direction, item_id, item_name, quantity FROM trade_items
             ORDER BY trade_id, direction, line_index",
        )?;
        let mut rows = stmt.query(params![])?;
        while let Some(row) = rows.next()? {
            let Some(&position) = index.get(&row.get::<_, i64>(0)?) else {
                continue;
            };
            let line = ItemLine {
                item_id: row.get(2)?,
                item_name: row.get(3)?,
                quantity: row.get(4)?,
            };
            let trade = &mut trades[position];
            if row.get::<_, String>(1)? == "acquired" {
                trade.acquired.push(line);
            } else {
                trade.given.push(line);
            }
        }
        Ok(trades)
    }

    /// Toutes les extractions de pacte, de la plus récente à la plus ancienne.
    pub fn pacts(&self) -> Result<Vec<PactRow>, HistoryError> {
        let mut pacts: Vec<PactRow> = self
            .conn
            .prepare_cached(
                "SELECT id, occurred_at, game_server FROM pact_extractions
                 ORDER BY occurred_at DESC, id",
            )?
            .query_map([], |row| {
                Ok(PactRow {
                    id: row.get(0)?,
                    occurred_at: row.get(1)?,
                    game_server: row.get(2)?,
                    items: Vec::new(),
                })
            })?
            .collect::<Result<_, _>>()?;
        let index: std::collections::HashMap<i64, usize> = pacts
            .iter()
            .enumerate()
            .map(|(position, pact)| (pact.id, position))
            .collect();
        let mut stmt = self.conn.prepare_cached(
            "SELECT extraction_id, item_id, item_name, quantity FROM pact_extraction_items
             ORDER BY extraction_id, line_index",
        )?;
        let mut rows = stmt.query(params![])?;
        while let Some(row) = rows.next()? {
            if let Some(&position) = index.get(&row.get::<_, i64>(0)?) {
                pacts[position].items.push(ItemLine {
                    item_id: row.get(1)?,
                    item_name: row.get(2)?,
                    quantity: row.get(3)?,
                });
            }
        }
        Ok(pacts)
    }
}
