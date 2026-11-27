//! Ce que le serveur du site recalculait après chaque envoi, porté ici et appliqué après chaque lot :
//!
//! 1. **Rattachement aux runs de donjon** (`server/history/dungeon-run.ts`) : le moteur ne voit que
//!    la session en cours ; une salle dont le boss a été battu lors d'un lancement précédent de
//!    l'overlay (ou d'une salle écrite avant que le moteur ne connaisse son run) n'est rattachée que
//!    par ce recalcul, qui voit l'historique local.
//! 2. **Type de combat** (`server/history/fight-type.ts`) : classement matérialisé qui alimente les
//!    regroupements du récap (`DUNGEON_*`, brèches, salle, famille, événement).
//!
//! Les règles sont celles du site, à l'identique, pour que l'historique importé (phase 2) et
//! l'historique écrit ici donnent les mêmes regroupements.

use std::collections::HashMap;

use overlay_engine::catalog::MonsterClassification;
use overlay_engine::dungeon_run::{
    enemy_composition_key, find_dungeon_for_enemies, group_dungeon_runs, DungeonHistoryEntry,
    GroupableFight,
};
use overlay_engine::{CatalogIndex, DungeonEntry, DungeonIndex, WakfuDungeonType};
use rusqlite::{params, Transaction};

/// Combats chargés en arrière avant de regrouper — même marge que le serveur (`LOOKBACK_FIGHTS`) :
/// bien au-delà du plus long run réel (quatre salles, archimonstre, tentatives de boss répétées).
const LOOKBACK_FIGHTS: i64 = 100;

/// Un combat vu du regroupement.
struct RunRow {
    id: i64,
    started_at: i64,
    won: bool,
    dungeon_id: Option<i64>,
    dungeon_run_key: Option<String>,
    signature: String,
    enemies: Vec<String>,
}

fn enemy_names(tx: &Transaction<'_>, fight_id: i64) -> rusqlite::Result<Vec<String>> {
    let mut statement = tx.prepare_cached(
        "SELECT name FROM fight_participants
         WHERE fight_id = ?1 AND side = 'enemy'
         ORDER BY instance_index, name",
    )?;
    let names = statement
        .query_map([fight_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(names)
}

fn load_run_row(tx: &Transaction<'_>, id: i64) -> rusqlite::Result<RunRow> {
    let mut row = tx.query_row(
        "SELECT id, started_at, won, dungeon_id, dungeon_run_key, signature
         FROM fights WHERE id = ?1",
        [id],
        |row| {
            Ok(RunRow {
                id: row.get(0)?,
                started_at: row.get(1)?,
                won: row.get(2)?,
                dungeon_id: row.get(3)?,
                dungeon_run_key: row.get(4)?,
                signature: row.get(5)?,
                enemies: Vec::new(),
            })
        },
    )?;
    row.enemies = enemy_names(tx, id)?;
    Ok(row)
}

/// Recalcule le rattachement de donjon pour les combats `touched` d'un lot — miroir de
/// `recomputeDungeonRunsForBatch`. Rend les ids des combats rattachés, y compris ceux de la
/// fenêtre d'historique hors du lot (salles écrites plus tôt, rattachées maintenant que leur boss
/// arrive) : leur type de combat doit être recalculé avec le lot.
///
/// **Filtre d'abord** : seul un lot dont un combat révèle LUI-MÊME un donjon (boss ou brèche) peut
/// avoir des salles à rattacher ; sinon, rien n'est relu.
pub(crate) fn recompute_dungeon_runs(
    tx: &Transaction<'_>,
    touched: &[i64],
    catalog: &CatalogIndex,
    dungeons: &DungeonIndex,
) -> rusqlite::Result<Vec<i64>> {
    if touched.is_empty() || dungeons.is_empty() {
        return Ok(Vec::new());
    }
    let mut rows: HashMap<i64, RunRow> = HashMap::new();
    for &id in touched {
        rows.insert(id, load_run_row(tx, id)?);
    }
    let reveals_dungeon = rows
        .values()
        .any(|row| find_dungeon_for_enemies(catalog, dungeons, &row.enemies).is_some());
    if !reveals_dungeon {
        return Ok(Vec::new());
    }

    let lookback: Vec<i64> = {
        let mut statement =
            tx.prepare_cached("SELECT id FROM fights ORDER BY started_at DESC, id DESC LIMIT ?1")?;
        let ids = statement
            .query_map([LOOKBACK_FIGHTS], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<i64>>>()?;
        ids
    };
    for id in lookback {
        if let std::collections::hash_map::Entry::Vacant(slot) = rows.entry(id) {
            slot.insert(load_run_row(tx, id)?);
        }
    }

    // Du plus récent au plus ancien, comme l'attend `group_dungeon_runs`.
    let mut ordered: Vec<&RunRow> = rows.values().collect();
    ordered.sort_by(|a, b| {
        b.started_at
            .cmp(&a.started_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    let detected: HashMap<i64, Option<&DungeonEntry>> = ordered
        .iter()
        .map(|row| {
            (
                row.id,
                find_dungeon_for_enemies(catalog, dungeons, &row.enemies),
            )
        })
        .collect();
    let records: Vec<GroupableFight> = ordered
        .iter()
        .map(|row| GroupableFight {
            id: row.id,
            won: row.won,
        })
        .collect();
    let entries = group_dungeon_runs(
        &records,
        |id| detected.get(&id).copied().flatten(),
        |id| {
            rows.get(&id).is_some_and(|row| {
                row.enemies
                    .iter()
                    .any(|name| catalog.find_monster_is_archi(name, None))
            })
        },
        |id| {
            rows.get(&id)
                .map(|row| enemy_composition_key(&row.enemies))
                .unwrap_or_default()
        },
    );

    // Mêmes décisions que `resolveUpdatesForUser`, à une différence près : faute de clé existante,
    // la clé d'un run est la signature de son combat représentatif (celle que le moteur aurait
    // donnée), là où le serveur fabriquait `migrated:<id>`.
    let mut updates: Vec<(i64, i64, String)> = Vec::new();
    for entry in entries {
        match entry {
            DungeonHistoryEntry::Single(id) => {
                let row = &rows[&id];
                if let (Some(dungeon), None) = (detected[&id], row.dungeon_id) {
                    let key = row
                        .dungeon_run_key
                        .clone()
                        .unwrap_or_else(|| row.signature.clone());
                    updates.push((id, dungeon.id, key));
                }
            }
            DungeonHistoryEntry::DungeonRun { dungeon, fight_ids } => {
                let key = fight_ids
                    .iter()
                    .find_map(|id| rows[id].dungeon_run_key.clone())
                    .unwrap_or_else(|| rows[&fight_ids[0]].signature.clone());
                for id in fight_ids {
                    if rows[&id].dungeon_id.is_none() {
                        updates.push((id, dungeon.id, key.clone()));
                    }
                }
            }
        }
    }

    let mut attached = Vec::new();
    let mut statement = tx.prepare_cached(
        "UPDATE fights SET dungeon_id = ?2, dungeon_run_key = ?3
         WHERE id = ?1 AND dungeon_id IS NULL",
    )?;
    for (id, dungeon_id, key) in updates {
        if statement.execute(params![id, dungeon_id, key])? > 0 {
            attached.push(id);
        }
    }
    Ok(attached)
}

/// Un ennemi vu du type de combat.
struct Enemy {
    name: String,
    monster_id: Option<i64>,
    damage: i64,
}

/// Type de combat (`fights.fight_type`) — miroir des trois requêtes de `fight-type.ts`. `None` :
/// la valeur actuelle reste en place (donjon `ARCADE` ou absent du référentiel, ennemis portant un
/// id que le référentiel ne connaît plus) — le serveur ne l'écrivait pas non plus.
fn fight_type(
    dungeon_id: Option<i64>,
    enemies: &[Enemy],
    catalog: &CatalogIndex,
    dungeons: &DungeonIndex,
) -> Option<String> {
    // Seuls les ennemis dont l'id est au référentiel comptent (jointure `monsters` côté serveur).
    let known: Vec<(&Enemy, i64)> = enemies
        .iter()
        .filter_map(|enemy| {
            enemy
                .monster_id
                .filter(|id| catalog.has_monster_id(*id))
                .map(|id| (enemy, id))
        })
        .collect();
    let classification =
        |enemy: &Enemy, id: i64| catalog.find_monster_classification(&enemy.name, Some(id));

    if let Some(dungeon_id) = dungeon_id {
        let dungeon = dungeons.find_by_id(dungeon_id)?;
        let rooms = |code: &str| {
            let has_boss = known
                .iter()
                .any(|(enemy, id)| classification(enemy, *id) == MonsterClassification::Boss);
            Some(if has_boss {
                format!("DUNGEON_{code}")
            } else {
                "DUNGEON_ROOM".to_string()
            })
        };
        return match dungeon.dungeon_type {
            WakfuDungeonType::Breach => Some("BREACH".to_string()),
            WakfuDungeonType::UltimateBreach => Some("ULTIMATE_BREACH".to_string()),
            WakfuDungeonType::ThreePlayers => Some("DUNGEON_THREE_PLAYERS".to_string()),
            WakfuDungeonType::UltimateBoss => Some("DUNGEON_ULTIMATE_BOSS".to_string()),
            WakfuDungeonType::TwoRooms => rooms("TWO_ROOMS"),
            WakfuDungeonType::ThreeRooms => rooms("THREE_ROOMS"),
            WakfuDungeonType::FourRooms => rooms("FOUR_ROOMS"),
            WakfuDungeonType::Arcade => None,
        };
    }

    if enemies.iter().all(|enemy| enemy.monster_id.is_none()) {
        return Some("EVENT".to_string());
    }
    // Famille représentative : boss > archimonstre > dominant > autre, puis plus gros dégâts.
    // `min_by` sur l'ordre inversé : à égalité parfaite, le premier dans l'ordre de lecture gagne,
    // stable d'un calcul à l'autre.
    let (enemy, id) = known.iter().min_by(|(a, a_id), (b, b_id)| {
        classification(b, *b_id)
            .cmp(&classification(a, *a_id))
            .then_with(|| b.damage.cmp(&a.damage))
    })?;
    Some(
        match catalog.find_monster_family_id(&enemy.name, Some(*id)) {
            Some(family) => format!("FAMILY_{family}"),
            None => "FAMILY_NONE".to_string(),
        },
    )
}

/// Recalcule et écrit le type des combats `ids` — entièrement, à chaque fois (idempotent).
pub(crate) fn recompute_fight_types(
    tx: &Transaction<'_>,
    ids: &[i64],
    catalog: &CatalogIndex,
    dungeons: &DungeonIndex,
) -> rusqlite::Result<()> {
    let mut enemies_of = tx.prepare_cached(
        "SELECT name, monster_id, damage FROM fight_participants
         WHERE fight_id = ?1 AND side = 'enemy'
         ORDER BY instance_index, name",
    )?;
    let mut dungeon_of = tx.prepare_cached("SELECT dungeon_id FROM fights WHERE id = ?1")?;
    let mut write = tx.prepare_cached("UPDATE fights SET fight_type = ?2 WHERE id = ?1")?;
    for &id in ids {
        let dungeon_id: Option<i64> = dungeon_of.query_row([id], |row| row.get(0))?;
        let enemies = enemies_of
            .query_map([id], |row| {
                Ok(Enemy {
                    name: row.get(0)?,
                    monster_id: row.get(1)?,
                    damage: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if let Some(value) = fight_type(dungeon_id, &enemies, catalog, dungeons) {
            write.execute(params![id, value])?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> CatalogIndex {
        // [id, nom, gfxId, famille, boss, archi, dominant]
        CatalogIndex::from_json(&serde_json::json!({
            "items": [],
            "monsters": [
                [1, "Bouftou", 1, 10, 0, 0, 0],
                [2, "Bouftou Royal", 2, 10, 1, 0, 0],
                [3, "Boufcoul", 3, 11, 0, 1, 0],
                [4, "Chef de Guerre", 4, 12, 0, 0, 1],
                [5, "Sans Famille", 5, -1, 0, 0, 0],
            ],
        }))
    }

    fn dungeons() -> DungeonIndex {
        DungeonIndex::from_json(&serde_json::json!([
            {"id": 100, "fr": "Donjon Bouftou", "bossMonsterId": [2], "monsterFamilyId": [10],
             "type": "TWO_ROOMS", "hasPreBossArchi": false},
            {"id": 200, "fr": "Brèche", "bossMonsterId": [], "monsterFamilyId": [],
             "type": "BREACH", "hasPreBossArchi": false},
            {"id": 300, "fr": "Arcade", "bossMonsterId": [], "monsterFamilyId": [],
             "type": "ARCADE", "hasPreBossArchi": false},
        ]))
    }

    fn enemy(name: &str, monster_id: Option<i64>, damage: i64) -> Enemy {
        Enemy {
            name: name.to_string(),
            monster_id,
            damage,
        }
    }

    fn typed(dungeon_id: Option<i64>, enemies: &[Enemy]) -> Option<String> {
        fight_type(dungeon_id, enemies, &catalog(), &dungeons())
    }

    #[test]
    fn un_combat_de_boss_de_donjon_a_salles_prend_le_type_du_donjon() {
        let enemies = [
            enemy("Bouftou Royal", Some(2), 10),
            enemy("Bouftou", Some(1), 50),
        ];
        assert_eq!(
            typed(Some(100), &enemies).as_deref(),
            Some("DUNGEON_TWO_ROOMS")
        );
    }

    #[test]
    fn une_salle_sans_boss_est_une_salle() {
        let enemies = [enemy("Bouftou", Some(1), 50)];
        assert_eq!(typed(Some(100), &enemies).as_deref(), Some("DUNGEON_ROOM"));
    }

    #[test]
    fn une_breche_est_une_breche() {
        assert_eq!(typed(Some(200), &[]).as_deref(), Some("BREACH"));
    }

    #[test]
    fn arcade_et_donjon_inconnu_laissent_la_valeur_en_place() {
        assert_eq!(typed(Some(300), &[]), None);
        assert_eq!(typed(Some(999), &[]), None);
    }

    #[test]
    fn hors_donjon_la_famille_suit_boss_archi_dominant_puis_degats() {
        // L'archimonstre l'emporte sur le dominant et sur un monstre ordinaire plus frappé.
        let enemies = [
            enemy("Bouftou", Some(1), 999),
            enemy("Chef de Guerre", Some(4), 10),
            enemy("Boufcoul", Some(3), 5),
        ];
        assert_eq!(typed(None, &enemies).as_deref(), Some("FAMILY_11"));
        // Sans classement, le plus gros dégât.
        let enemies = [
            enemy("Sans Famille", Some(5), 1),
            enemy("Bouftou", Some(1), 80),
        ];
        assert_eq!(typed(None, &enemies).as_deref(), Some("FAMILY_10"));
        let enemies = [enemy("Sans Famille", Some(5), 1)];
        assert_eq!(typed(None, &enemies).as_deref(), Some("FAMILY_NONE"));
    }

    #[test]
    fn aucun_ennemi_identifie_est_un_evenement() {
        let enemies = [enemy("Inconnu", None, 10)];
        assert_eq!(typed(None, &enemies).as_deref(), Some("EVENT"));
        assert_eq!(typed(None, &[]).as_deref(), Some("EVENT"));
    }

    #[test]
    fn un_id_que_le_referentiel_ne_connait_plus_ne_classe_rien() {
        let enemies = [enemy("Disparu", Some(77), 10)];
        assert_eq!(typed(None, &enemies), None);
    }
}
