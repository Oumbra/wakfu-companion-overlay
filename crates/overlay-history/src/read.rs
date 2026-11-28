//! **Lecture des listes** de la fenêtre d'historique (phase 3) : achats, échanges, pactes.
//!
//! Ces trois historiques restent petits (quelques centaines de lignes par an) : ils sont lus en
//! entier, du plus récent au plus ancien, et la vue filtre, groupe et trie en mémoire — comme le
//! site, qui recevait tout l'historique avant d'en faire ses groupes par jour. Les combats sont lus
//! de même, sans leurs ventilations par sort (le JSON le plus lourd de la base) : quelques milliers
//! de combats et leurs participants tiennent en quelques Mo.
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

/// Un combattant d'un combat — une ligne par instance (trois Bouftous = trois lignes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantRow {
    pub name: String,
    /// Rang de l'instance parmi les combattants du même nom (clé avec `name`).
    pub instance_index: i64,
    pub is_ally: bool,
    pub monster_id: Option<i64>,
    pub class_name: Option<String>,
    pub damage: i64,
    pub heal: i64,
    pub armor: i64,
    pub defeated: bool,
    pub fled: bool,
    pub xp_gained: i64,
}

/// La ventilation lue par [`HistoryStore::participant_spells`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpellColumn {
    Damage,
    Heal,
    Armor,
}

/// Un sort d'un combattant : son total et sa part par élément (`Feu`, `Eau`…), dans l'ordre du
/// JSON enregistré.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLine {
    pub spell: String,
    pub total: i64,
    pub by_element: Vec<(String, i64)>,
    /// Le détail tour par tour (`byTurn`), par tour croissant — vide pour un combat enregistré
    /// avant ce détail ou importé du site, qui ne l'avait pas.
    pub by_turn: Vec<SpellTurn>,
}

/// Un tour d'un sort.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellTurn {
    pub turn: i64,
    pub total: i64,
    pub by_element: Vec<(String, i64)>,
}

/// Un combat, ses combattants (dans l'ordre d'écriture) et son butin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FightRow {
    pub id: i64,
    /// ms UTC.
    pub started_at: i64,
    pub duration_ms: Option<i64>,
    pub won: bool,
    pub turns: i64,
    pub xp_gained: i64,
    pub kamas_gained: Option<i64>,
    pub dungeon_id: Option<i64>,
    /// Partagée par les combats d'un même run de donjon (`derive`).
    pub dungeon_run_key: Option<String>,
    pub fight_type: Option<String>,
    pub participants: Vec<ParticipantRow>,
    pub loot: Vec<ItemLine>,
}

impl HistoryStore {
    /// Tous les combats, du plus récent au plus ancien, avec leurs combattants et leur butin.
    pub fn fights(&self) -> Result<Vec<FightRow>, HistoryError> {
        let mut fights: Vec<FightRow> = self
            .conn
            .prepare_cached(
                "SELECT id, started_at, duration_ms, won, turns, xp_gained, kamas_gained,
                    dungeon_id, dungeon_run_key, fight_type
                 FROM fights ORDER BY started_at DESC, id",
            )?
            .query_map([], |row| {
                Ok(FightRow {
                    id: row.get(0)?,
                    started_at: row.get(1)?,
                    duration_ms: row.get(2)?,
                    won: row.get(3)?,
                    turns: row.get(4)?,
                    xp_gained: row.get(5)?,
                    kamas_gained: row.get(6)?,
                    dungeon_id: row.get(7)?,
                    dungeon_run_key: row.get(8)?,
                    fight_type: row.get(9)?,
                    participants: Vec::new(),
                    loot: Vec::new(),
                })
            })?
            .collect::<Result<_, _>>()?;
        let index: std::collections::HashMap<i64, usize> = fights
            .iter()
            .enumerate()
            .map(|(position, fight)| (fight.id, position))
            .collect();
        let mut stmt = self.conn.prepare_cached(
            "SELECT fight_id, name, side, monster_id, class_name, damage, heal, armor, defeated,
                fled, xp_gained, instance_index
             FROM fight_participants ORDER BY fight_id, rowid",
        )?;
        let mut rows = stmt.query(params![])?;
        while let Some(row) = rows.next()? {
            if let Some(&position) = index.get(&row.get::<_, i64>(0)?) {
                fights[position].participants.push(ParticipantRow {
                    name: row.get(1)?,
                    is_ally: row.get::<_, String>(2)? == "ally",
                    monster_id: row.get(3)?,
                    class_name: row.get(4)?,
                    damage: row.get(5)?,
                    heal: row.get(6)?,
                    armor: row.get(7)?,
                    defeated: row.get(8)?,
                    fled: row.get(9)?,
                    xp_gained: row.get(10)?,
                    instance_index: row.get(11)?,
                });
            }
        }
        let mut stmt = self.conn.prepare_cached(
            "SELECT fight_id, item_id, item_name, quantity FROM fight_loot
             WHERE quantity > 0
             ORDER BY fight_id, line_index",
        )?;
        let mut rows = stmt.query(params![])?;
        while let Some(row) = rows.next()? {
            if let Some(&position) = index.get(&row.get::<_, i64>(0)?) {
                fights[position].loot.push(ItemLine {
                    item_id: row.get(1)?,
                    item_name: row.get(2)?,
                    quantity: row.get(3)?,
                });
            }
        }
        Ok(fights)
    }

    /// Les sorts d'un combattant, du plus gros total au plus petit — lus à la demande, quand la
    /// vue Combats déplie sa ligne : [`Self::fights`] ne charge pas ces JSON. Un JSON illisible
    /// donne une liste vide.
    pub fn participant_spells(
        &self,
        fight_id: i64,
        name: &str,
        instance_index: i64,
        column: SpellColumn,
    ) -> Result<Vec<SpellLine>, HistoryError> {
        let sql = match column {
            SpellColumn::Damage => {
                "SELECT spells FROM fight_participants
                 WHERE fight_id = ?1 AND name = ?2 AND instance_index = ?3"
            }
            SpellColumn::Heal => {
                "SELECT heal_spells FROM fight_participants
                 WHERE fight_id = ?1 AND name = ?2 AND instance_index = ?3"
            }
            SpellColumn::Armor => {
                "SELECT armor_spells FROM fight_participants
                 WHERE fight_id = ?1 AND name = ?2 AND instance_index = ?3"
            }
        };
        let json: Option<String> = self
            .conn
            .prepare_cached(sql)?
            .query_row(params![fight_id, name, instance_index], |row| row.get(0))
            .map(Some)
            .or_else(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                err => Err(err),
            })?;
        Ok(json.map(|json| parse_spells(&json)).unwrap_or_default())
    }

    /// Les sorts de **tous** les combattants d'un combat, pour une grandeur : `(nom, rang de
    /// l'instance, sorts)` — ce dont la vue « Tour » a besoin pour recalculer chaque ligne.
    pub fn fight_spells(
        &self,
        fight_id: i64,
        column: SpellColumn,
    ) -> Result<Vec<(String, i64, Vec<SpellLine>)>, HistoryError> {
        let sql = match column {
            SpellColumn::Damage => {
                "SELECT name, instance_index, spells FROM fight_participants WHERE fight_id = ?1"
            }
            SpellColumn::Heal => {
                "SELECT name, instance_index, heal_spells FROM fight_participants
                 WHERE fight_id = ?1"
            }
            SpellColumn::Armor => {
                "SELECT name, instance_index, armor_spells FROM fight_participants
                 WHERE fight_id = ?1"
            }
        };
        let mut stmt = self.conn.prepare_cached(sql)?;
        let rows = stmt
            .query_map(params![fight_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows
            .into_iter()
            .map(|(name, instance, json)| (name, instance, parse_spells(&json)))
            .collect())
    }

    /// La classe de chaque personnage allié déjà vu (`nom → classe`, la plus récente l'emporte) —
    /// le portrait d'une ligne d'expérience du Récap, qui ne connaît que des noms
    /// (`app-entity-icon` du site résout de même un nom en classe).
    pub fn ally_classes(&self) -> Result<std::collections::HashMap<String, String>, HistoryError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT name, class_name FROM fight_participants
             WHERE side = 'ally' AND class_name IS NOT NULL ORDER BY rowid",
        )?;
        let rows = stmt
            .query_map(params![], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows.into_iter().collect())
    }

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
             WHERE quantity > 0
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

/// `[{"spell": "Flèche explosive", "total": 29616, "byElement": {"Feu": 29616}}, …]` — le format
/// du site, enregistré tel quel. Triés du plus gros total au plus petit.
fn parse_spells(json: &str) -> Vec<SpellLine> {
    let number = |value: &serde_json::Value| {
        value
            .as_i64()
            .or_else(|| value.as_f64().map(|v| v.round() as i64))
    };
    let Ok(serde_json::Value::Array(spells)) = serde_json::from_str(json) else {
        return Vec::new();
    };
    let elements = |value: Option<&serde_json::Value>| -> Vec<(String, i64)> {
        value
            .and_then(|v| v.as_object())
            .map(|elements| {
                elements
                    .iter()
                    .filter_map(|(element, value)| Some((element.clone(), number(value)?)))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut lines: Vec<SpellLine> = spells
        .iter()
        .filter_map(|spell| {
            let mut by_turn: Vec<SpellTurn> = spell
                .get("byTurn")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter_map(|turn| {
                    Some(SpellTurn {
                        turn: turn.get("turn").and_then(number)?,
                        total: turn.get("total").and_then(number).unwrap_or(0),
                        by_element: elements(turn.get("byElement")),
                    })
                })
                .collect();
            by_turn.sort_by_key(|turn| turn.turn);
            Some(SpellLine {
                spell: spell.get("spell")?.as_str()?.to_string(),
                total: spell.get("total").and_then(number).unwrap_or(0),
                by_element: elements(spell.get("byElement")),
                by_turn,
            })
        })
        .collect();
    lines.sort_by_key(|line| std::cmp::Reverse(line.total));
    lines
}
