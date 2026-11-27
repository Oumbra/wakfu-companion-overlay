//! **Agrégats d'une période** — port de `functions/api/v1/history/stats.ts` (site), qui alimentait
//! la carte Récap : totaux de combats, kamas, XP par personnage, butin, regroupements par donjon et
//! par famille, pactes (`docs/plan-historique-local.md` §6.3).
//!
//! Le serveur joignait sa table `monsters` ; la base locale n'en a pas — le référentiel embarqué
//! (`CatalogIndex`) en tient lieu. Les combats de la période sont donc lus avec leurs ennemis et
//! agrégés **en Rust** (quelques milliers de lignes pour une année de jeu intensif), achats,
//! échanges et pactes en SQL. Mêmes règles que le serveur :
//!
//! 1. un combat dont un ennemi (connu du référentiel) est de la famille 161, « Extra Incarnam »,
//!    n'entre dans **aucun** total de combat ;
//! 2. par donjon, `won`/`lost`/`dungeon_runs` ne comptent que les combats contenant un boss (une
//!    victoire de boss = un donjon fait) ; combats, kamas, XP et butin comptent toutes les salles ;
//! 3. hors donjon, chaque combat est rangé sous sa famille représentative (boss > archimonstre >
//!    dominant > plus gros dégâts) ; un combat sans ennemi connu du référentiel n'est dans aucune ;
//! 4. ventes à l'Hôtel de Vente et achats séparés par la sentinelle [`HDV_KAMAS_SALE_ITEM`].

use std::collections::HashMap;

use overlay_engine::{CatalogIndex, MonsterClassification};
use rusqlite::params;

use crate::derive::{representative_family, Enemy};
use crate::{HistoryError, HistoryStore};

/// Famille exclue de tous les totaux — miroir d'`EXCLUDED_FAMILY_ID` (site).
pub const EXCLUDED_FAMILY_ID: i64 = 161;

/// Nom d'objet d'une récupération de kamas à l'Hôtel de Vente — miroir de `HDV_KAMAS_SALE_ITEM`.
pub const HDV_KAMAS_SALE_ITEM: &str = "__hdv_kamas_sale__";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTotal {
    pub item_id: Option<i64>,
    pub item_name: Option<String>,
    pub quantity: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XpTotal {
    pub name: String,
    pub amount: i64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CombatTotals {
    pub won: i64,
    pub lost: i64,
    pub challenges_passed: i64,
    pub challenges_failed: i64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct KamasTotals {
    pub from_combat: i64,
    pub from_hdv_sales: i64,
    pub spent_on_purchases: i64,
    pub trades_acquired: i64,
    pub trades_given: i64,
    pub trade_count: i64,
}

/// Totaux d'un donjon ou d'une famille — `GroupTotalsRow` du site.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GroupTotals {
    pub fights: i64,
    /// Victoires de boss (donjons) ; toujours 0 pour une famille.
    pub dungeon_runs: i64,
    /// Donjon : issue des combats de boss seulement. Famille : combats bruts.
    pub won: i64,
    pub lost: i64,
    pub kamas_gained: i64,
    pub xp_gained: i64,
    /// Par XP décroissante.
    pub xp_by_character: Vec<XpTotal>,
    pub loot: Vec<ItemTotal>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PeriodStats {
    pub combats: CombatTotals,
    pub kamas: KamasTotals,
    /// Par XP décroissante.
    pub xp_by_character: Vec<XpTotal>,
    pub loot: Vec<ItemTotal>,
    /// Par nombre de combats décroissant.
    pub dungeons: Vec<(i64, GroupTotals)>,
    /// Par nombre de combats décroissant ; `None` = représentant sans famille.
    pub families: Vec<(Option<i64>, GroupTotals)>,
    pub pacts: Vec<ItemTotal>,
}

/// Un combat de la période, avec ce que les règles ci-dessus en retiennent.
struct FightRow {
    won: bool,
    challenges_passed: i64,
    challenges_failed: i64,
    kamas: i64,
    xp: i64,
    dungeon_id: Option<i64>,
    enemies: Vec<Enemy>,
}

/// Où un combat est rangé : son donjon, ou sa famille représentative hors donjon.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Group {
    Dungeon(i64),
    Family(Option<i64>),
}

type ItemKey = (Option<i64>, Option<String>);

/// Cumul d'objets par `(item_id, item_name)`, dans l'ordre de première apparition.
#[derive(Default)]
struct ItemSums {
    order: Vec<ItemKey>,
    quantities: HashMap<ItemKey, i64>,
}

impl ItemSums {
    fn add(&mut self, key: ItemKey, quantity: i64) {
        match self.quantities.get_mut(&key) {
            Some(total) => *total += quantity,
            None => {
                self.order.push(key.clone());
                self.quantities.insert(key, quantity);
            }
        }
    }

    fn into_vec(mut self) -> Vec<ItemTotal> {
        self.order
            .into_iter()
            .map(|key| {
                let quantity = self.quantities.remove(&key).unwrap_or(0);
                ItemTotal {
                    item_id: key.0,
                    item_name: key.1,
                    quantity,
                }
            })
            .collect()
    }
}

/// Cumul d'XP par personnage, rendu par XP décroissante (puis nom, pour un ordre stable).
#[derive(Default)]
struct XpSums(HashMap<String, i64>);

impl XpSums {
    fn add(&mut self, name: &str, amount: i64) {
        *self.0.entry(name.to_string()).or_insert(0) += amount;
    }

    fn into_vec(self) -> Vec<XpTotal> {
        let mut out: Vec<XpTotal> = self
            .0
            .into_iter()
            .map(|(name, amount)| XpTotal { name, amount })
            .collect();
        out.sort_by(|a, b| b.amount.cmp(&a.amount).then_with(|| a.name.cmp(&b.name)));
        out
    }
}

#[derive(Default)]
struct GroupSums {
    totals: GroupTotals,
    xp: XpSums,
    loot: ItemSums,
}

impl HistoryStore {
    /// Agrégats de `[since_ms, until_ms[` (ms UTC) — voir la doc du module.
    pub fn period_stats(
        &self,
        since_ms: i64,
        until_ms: i64,
        catalog: &CatalogIndex,
    ) -> Result<PeriodStats, HistoryError> {
        let conn = &self.conn;
        let mut fights: HashMap<i64, FightRow> = HashMap::new();
        let mut order: Vec<i64> = Vec::new();
        {
            let mut stmt = conn.prepare_cached(
                "SELECT id, won, challenges_passed, challenges_failed, kamas_gained, xp_gained,
                    dungeon_id
                 FROM fights WHERE started_at >= ?1 AND started_at < ?2
                 ORDER BY started_at, id",
            )?;
            let mut rows = stmt.query(params![since_ms, until_ms])?;
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                order.push(id);
                fights.insert(
                    id,
                    FightRow {
                        won: row.get(1)?,
                        challenges_passed: row.get(2)?,
                        challenges_failed: row.get(3)?,
                        kamas: row.get::<_, Option<i64>>(4)?.unwrap_or(0),
                        xp: row.get(5)?,
                        dungeon_id: row.get(6)?,
                        enemies: Vec::new(),
                    },
                );
            }
        }
        {
            let mut stmt = conn.prepare_cached(
                "SELECT p.fight_id, p.name, p.monster_id, p.damage
                 FROM fight_participants p JOIN fights f ON f.id = p.fight_id
                 WHERE f.started_at >= ?1 AND f.started_at < ?2 AND p.side = 'enemy'
                 ORDER BY p.fight_id, p.instance_index, p.name",
            )?;
            let mut rows = stmt.query(params![since_ms, until_ms])?;
            while let Some(row) = rows.next()? {
                if let Some(fight) = fights.get_mut(&row.get(0)?) {
                    fight.enemies.push(Enemy {
                        name: row.get(1)?,
                        monster_id: row.get(2)?,
                        damage: row.get(3)?,
                    });
                }
            }
        }

        let known = |enemy: &Enemy| enemy.monster_id.filter(|id| catalog.has_monster_id(*id));
        // Règle 1 : combats exclus. Puis le groupe de chacun des autres.
        let mut group_of: HashMap<i64, Option<Group>> = HashMap::new();
        for (&id, fight) in &fights {
            let excluded = fight.enemies.iter().any(|enemy| {
                known(enemy).is_some_and(|monster| {
                    catalog.find_monster_family_id(&enemy.name, Some(monster))
                        == Some(EXCLUDED_FAMILY_ID)
                })
            });
            if excluded {
                continue;
            }
            let group = match fight.dungeon_id {
                Some(dungeon) => Some(Group::Dungeon(dungeon)),
                None => representative_family(&fight.enemies, catalog).map(Group::Family),
            };
            group_of.insert(id, group);
        }

        let mut stats = PeriodStats::default();
        let mut groups: HashMap<Group, GroupSums> = HashMap::new();
        let mut group_order: Vec<Group> = Vec::new();
        for id in &order {
            let (Some(group), fight) = (group_of.get(id), &fights[id]) else {
                continue;
            };
            stats.combats.won += i64::from(fight.won);
            stats.combats.lost += i64::from(!fight.won);
            stats.combats.challenges_passed += fight.challenges_passed;
            stats.combats.challenges_failed += fight.challenges_failed;
            stats.kamas.from_combat += fight.kamas;
            let Some(group) = group else { continue };
            let sums = groups.entry(*group).or_insert_with(|| {
                group_order.push(*group);
                GroupSums::default()
            });
            sums.totals.fights += 1;
            sums.totals.kamas_gained += fight.kamas;
            sums.totals.xp_gained += fight.xp;
            match group {
                // Règle 2 : l'issue d'un donjon est celle de ses combats de boss.
                Group::Dungeon(_) => {
                    let has_boss = fight.enemies.iter().any(|enemy| {
                        known(enemy).is_some_and(|monster| {
                            catalog.find_monster_classification(&enemy.name, Some(monster))
                                == MonsterClassification::Boss
                        })
                    });
                    if has_boss {
                        sums.totals.won += i64::from(fight.won);
                        sums.totals.dungeon_runs += i64::from(fight.won);
                        sums.totals.lost += i64::from(!fight.won);
                    }
                }
                Group::Family(_) => {
                    sums.totals.won += i64::from(fight.won);
                    sums.totals.lost += i64::from(!fight.won);
                }
            }
        }

        // XP par personnage (alliés) et butin : global, et par groupe.
        let mut xp = XpSums::default();
        {
            let mut stmt = conn.prepare_cached(
                "SELECT p.fight_id, p.name, p.xp_gained
                 FROM fight_participants p JOIN fights f ON f.id = p.fight_id
                 WHERE f.started_at >= ?1 AND f.started_at < ?2 AND p.side = 'ally'",
            )?;
            let mut rows = stmt.query(params![since_ms, until_ms])?;
            while let Some(row) = rows.next()? {
                let Some(group) = group_of.get(&row.get::<_, i64>(0)?) else {
                    continue;
                };
                let (name, amount): (String, i64) = (row.get(1)?, row.get(2)?);
                xp.add(&name, amount);
                if let Some(sums) = group.and_then(|group| groups.get_mut(&group)) {
                    sums.xp.add(&name, amount);
                }
            }
        }
        let mut loot = ItemSums::default();
        {
            let mut stmt = conn.prepare_cached(
                "SELECT l.fight_id, l.item_id, l.item_name, l.quantity
                 FROM fight_loot l JOIN fights f ON f.id = l.fight_id
                 WHERE f.started_at >= ?1 AND f.started_at < ?2
                 ORDER BY f.started_at, l.fight_id, l.line_index",
            )?;
            let mut rows = stmt.query(params![since_ms, until_ms])?;
            while let Some(row) = rows.next()? {
                let Some(group) = group_of.get(&row.get::<_, i64>(0)?) else {
                    continue;
                };
                let key: ItemKey = (row.get(1)?, row.get(2)?);
                let quantity: i64 = row.get(3)?;
                if let Some(sums) = group.and_then(|group| groups.get_mut(&group)) {
                    sums.loot.add(key.clone(), quantity);
                }
                loot.add(key, quantity);
            }
        }
        stats.xp_by_character = xp.into_vec();
        stats.loot = loot.into_vec();

        for group in group_order {
            let Some(sums) = groups.remove(&group) else {
                continue;
            };
            let mut totals = sums.totals;
            totals.xp_by_character = sums.xp.into_vec();
            totals.loot = sums.loot.into_vec();
            match group {
                Group::Dungeon(id) => stats.dungeons.push((id, totals)),
                Group::Family(id) => stats.families.push((id, totals)),
            }
        }
        // `ORDER BY count(*) DESC` côté serveur ; à égalité, ordre de première apparition.
        stats
            .dungeons
            .sort_by_key(|group| std::cmp::Reverse(group.1.fights));
        stats
            .families
            .sort_by_key(|group| std::cmp::Reverse(group.1.fights));

        // Règle 4 : achats et ventes à l'HDV.
        (stats.kamas.spent_on_purchases, stats.kamas.from_hdv_sales) = conn.query_row(
            "SELECT
                COALESCE(SUM(CASE WHEN item_name IS NOT ?3 THEN total_cost END), 0),
                COALESCE(SUM(CASE WHEN item_name = ?3 THEN total_cost END), 0)
             FROM purchases WHERE occurred_at >= ?1 AND occurred_at < ?2",
            params![since_ms, until_ms, HDV_KAMAS_SALE_ITEM],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        (
            stats.kamas.trades_acquired,
            stats.kamas.trades_given,
            stats.kamas.trade_count,
        ) = conn.query_row(
            "SELECT COALESCE(SUM(kamas_acquired), 0), COALESCE(SUM(kamas_given), 0), COUNT(*)
             FROM trades WHERE occurred_at >= ?1 AND occurred_at < ?2",
            params![since_ms, until_ms],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let mut pacts = ItemSums::default();
        {
            let mut stmt = conn.prepare_cached(
                "SELECT i.item_id, i.item_name, i.quantity
                 FROM pact_extraction_items i
                 JOIN pact_extractions e ON e.id = i.extraction_id
                 WHERE e.occurred_at >= ?1 AND e.occurred_at < ?2
                 ORDER BY e.occurred_at, i.extraction_id, i.line_index",
            )?;
            let mut rows = stmt.query(params![since_ms, until_ms])?;
            while let Some(row) = rows.next()? {
                pacts.add((row.get(0)?, row.get(1)?), row.get(2)?);
            }
        }
        stats.pacts = pacts.into_vec();
        Ok(stats)
    }
}
