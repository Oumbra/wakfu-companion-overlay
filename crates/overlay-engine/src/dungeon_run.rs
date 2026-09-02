//! Regroupement de combats de donjon multi-salles (L5, §7.1 du plan) — port fidèle de
//! `dungeon-run-grouping.util.ts` + la partie `findDungeonForEnemies` de `fight-image.util.ts`
//! (dépôt web local `../wakfu-companion`, consulté directement pour cette implémentation plutôt
//! que redevinée) : reconnaît qu'une salle de donjon et le combat de boss qui la suit forment un
//! seul et même "run", pour que TOUS les combats d'un run partagent le même `dungeonId`/
//! `dungeonRunKey` — voir `history.rs::FightPayload::dungeon_run_signature` pour le format final.
//!
//! Volontairement laissé de côté jusqu'ici (§2 du plan) : cette heuristique fait partie des ~3800
//! lignes de logique métier que le §2 réserve au moteur TS partagé pour éviter tout risque de
//! divergence avec le web — mais demande explicite du mainteneur (2026-09-02, suite) de la porter
//! quand même, le dépôt web étant disponible en local pour vérifier chaque formule plutôt que la
//! redeviner. Les vecteurs de test de `group_dungeon_runs` sont recalculés à la main depuis les cas
//! documentés côté web (`dungeon-run-grouping.util.spec.ts`) pour limiter ce risque de divergence.
//!
//! **Ce qui n'est PAS porté** : la brèche/brèche ultime restent résolues par
//! `find_dungeon_for_enemies` (celui-ci EST complet, 3 priorités) — c'est le regroupement
//! MULTI-COMBATS (`group_dungeon_runs`) qui ne s'applique qu'aux donjons à salles
//! (`TWO_ROOMS`/`THREE_ROOMS`/`FOUR_ROOMS`, résolus par boss uniquement) : une brèche est toujours
//! un donjon à un seul combat (`room_count() == 1`), jamais un candidat au regroupement.

use crate::catalog::{CatalogIndex, MonsterClassification};
use crate::dungeon::{DungeonEntry, DungeonIndex};

/// Plus de familles de monstre distinctes que ce seuil, sans aucun boss présent : une horde
/// hétérogène plutôt qu'un donjon/archi/dominant — miroir de `DISTINCT_FAMILY_THRESHOLD`
/// (`fight-image.util.ts`).
const DISTINCT_FAMILY_THRESHOLD: usize = 4;

/// Résout le donjon (classique OU brèche/brèche ultime) dont `enemy_names` sont les ennemis d'UN
/// SEUL combat — miroir de `findDungeonForEnemies` (`fight-image.util.ts`), les 3 priorités :
/// 0. Plusieurs boss d'IDS DISTINCTS présents simultanément → la brèche ULTIME dont les boss
///    couvrent exactement cet ensemble (`DungeonIndex::find_ultimate_breach_by_boss_monsters`).
/// 1. Un boss présent (seul, ou plusieurs mais aucune brèche ultime ne correspond) → le donjon
///    classique dont il est le boss attitré (`DungeonIndex::find_by_boss_monster_id`), `None` si
///    son id n'est référencé par aucun donjon.
/// 2. Aucun boss du tout, mais plus de `DISTINCT_FAMILY_THRESHOLD` familles de monstre DISTINCTES
///    parmi les ennemis résolus par le catalogue → la brèche SIMPLE dont la composition en
///    familles couvre celles observées (`DungeonIndex::find_breach_by_monster_families`).
///
/// Un nom d'ennemi non résolu par le catalogue est simplement ignoré à chaque étape (aucune
/// information à en tirer), jamais une erreur — même politique que le reste de `catalog.rs`.
pub fn find_dungeon_for_enemies<'a>(
    catalog: &CatalogIndex,
    dungeons: &'a DungeonIndex,
    enemy_names: &[String],
) -> Option<&'a DungeonEntry> {
    // Résolution catalogue une seule fois par nom — réutilisée par les 3 priorités.
    let resolved: Vec<(i64, MonsterClassification, Option<i64>)> = enemy_names
        .iter()
        .filter_map(|name| {
            let id = catalog.find_monster_id(name)?;
            Some((
                id,
                catalog.find_monster_classification(name, Some(id)),
                catalog.find_monster_family_id(name, Some(id)),
            ))
        })
        .collect();

    // Priorité 0 : ids de boss DISTINCTS (pas le nombre brut d'entrées `isBoss`, voir la doc web —
    // une resynchronisation en cours de combat peut réémettre plusieurs fois le même boss).
    let mut distinct_boss_ids: Vec<i64> = resolved
        .iter()
        .filter(|(_, classification, _)| *classification == MonsterClassification::Boss)
        .map(|(id, ..)| *id)
        .collect();
    distinct_boss_ids.sort_unstable();
    distinct_boss_ids.dedup();
    if distinct_boss_ids.len() > 1 {
        if let Some(dungeon) = dungeons.find_ultimate_breach_by_boss_monsters(&distinct_boss_ids) {
            return Some(dungeon);
        }
    }

    // Priorité 1 : un boss (dans l'ordre d'apparition — le premier donjon trouvé suffit, deux boss
    // distincts référençant deux donjons différents dans le même combat n'existent pas en pratique
    // en dehors du cas brèche ultime déjà traité ci-dessus).
    for id in &distinct_boss_ids {
        if let Some(dungeon) = dungeons.find_by_boss_monster_id(*id) {
            return Some(dungeon);
        }
    }

    // Priorité 2 : horde hétérogène, seulement si AUCUN boss du tout (miroir exact du web :
    // `bossEntries.length === 0`, pas `distinctBossIds.length === 0` — deux instances du même boss
    // ne doivent pas se qualifier ici, mais ce cas est déjà couvert par la priorité 1 ci-dessus qui
    // aurait renvoyé plus tôt).
    if distinct_boss_ids.is_empty() {
        let mut distinct_families: Vec<Option<i64>> =
            resolved.iter().map(|(_, _, family)| *family).collect();
        distinct_families.sort_unstable();
        distinct_families.dedup();
        if distinct_families.len() > DISTINCT_FAMILY_THRESHOLD {
            let family_ids: Vec<i64> = distinct_families.into_iter().flatten().collect();
            if let Some(dungeon) = dungeons.find_breach_by_monster_families(&family_ids) {
                return Some(dungeon);
            }
        }
    }

    None
}

/// Clé de comparaison de composition d'ennemis — miroir de `enemyCompositionKey`
/// (`dungeon-run-grouping.util.ts`) : ensemble d'espèces DISTINCTES (pas les comptes), triées pour
/// être insensible à l'ordre d'apparition. Noms bruts, PAS normalisés (`normalize_wakfu_name`) —
/// même choix que le web, qui compare les noms du log tels quels ici.
pub fn enemy_composition_key(enemy_names: &[String]) -> String {
    let mut distinct: Vec<&str> = enemy_names.iter().map(String::as_str).collect();
    distinct.sort_unstable();
    distinct.dedup();
    distinct.join("|")
}

/// Un combat, vu du seul regroupement de donjon — équivalent minimal de `DungeonGroupableFight`.
#[derive(Debug, Clone, Copy)]
pub struct GroupableFight {
    pub id: i64,
    pub won: bool,
}

/// Résultat du regroupement pour un combat — miroir de `DungeonHistoryEntry` (`dungeon-run-
/// grouping.util.ts`), réduit aux seuls champs utiles ici (pas de variante affichage/tri).
#[derive(Debug)]
pub enum DungeonHistoryEntry<'a> {
    Single(i64),
    DungeonRun {
        dungeon: &'a DungeonEntry,
        /// Ids de combat du run, du plus RÉCENT au plus ANCIEN (représentant en premier) — même
        /// ordre que `records` en entrée.
        fight_ids: Vec<i64>,
    },
}

/// Regroupe les combats d'un même donjon (salles + tentatives de boss) — port direct de
/// `groupDungeonRuns` (voir sa doc côté web, non redupliquée ligne à ligne ici : seule la
/// traduction Rust est commentée). `records` DOIT être trié du plus récent au plus ancien (même
/// convention que `HistoryArchiveService.mergedFights`) : c'est ce qui permet de scanner vers
/// l'arrière dans le temps sans retri.
///
/// `find_dungeon`/`has_archi_enemy`/`room_composition_key` sont injectées plutôt qu'un accès
/// direct à `SessionState` : garde cette fonction pure et testable sans construire un `Engine`
/// complet — même principe que `find_dungeon_for_enemies` ci-dessus.
pub fn group_dungeon_runs<'a>(
    records: &[GroupableFight],
    find_dungeon: impl Fn(i64) -> Option<&'a DungeonEntry>,
    has_archi_enemy: impl Fn(i64) -> bool,
    room_composition_key: impl Fn(i64) -> String,
) -> Vec<DungeonHistoryEntry<'a>> {
    let mut entries = Vec::new();
    let mut consumed = vec![false; records.len()];

    let mut i = 0;
    while i < records.len() {
        if consumed[i] {
            i += 1;
            continue;
        }

        let Some(dungeon) = find_dungeon(records[i].id) else {
            entries.push(DungeonHistoryEntry::Single(records[i].id));
            consumed[i] = true;
            i += 1;
            continue;
        };

        // Donjon à un seul combat (étape 0) : jamais regroupé, même en cas de tentatives répétées.
        if dungeon.room_count() == 1 {
            entries.push(DungeonHistoryEntry::Single(records[i].id));
            consumed[i] = true;
            i += 1;
            continue;
        }

        // `records[i]` fait partie du cluster quel que soit son résultat (tentative la plus
        // RÉCENTE contre ce boss). Les tentatives plus anciennes ne rejoignent que tant qu'elles
        // sont des DÉFAITES contre ce même boss (étape 1) — une victoire plus ancienne appartient
        // déjà à un run précédent distinct.
        let mut j = i + 1;
        while j < records.len() {
            match find_dungeon(records[j].id) {
                Some(candidate) if candidate.id == dungeon.id && !records[j].won => j += 1,
                _ => break,
            }
        }

        // Créneau archimonstre optionnel (étape 2) : consommé seulement s'il est réellement
        // présent dans CE combat précis, jamais sur la seule foi de `has_pre_boss_archi`.
        if dungeon.has_pre_boss_archi
            && j < records.len()
            && find_dungeon(records[j].id).is_none()
            && has_archi_enemy(records[j].id)
        {
            j += 1;
        }

        // Salles précédentes (étape 3) : on avance combat par combat. Une VICTOIRE compte pour une
        // salle (sauf si `room_slots` est déjà atteint : salle en trop = run antérieur distinct,
        // on s'arrête sans la consommer). Une DÉFAITE n'est ramassée comme tentative ratée de la
        // salle EN COURS que si sa composition d'ennemis correspond EXACTEMENT à celle de la
        // victoire qui la referme — sinon c'est un combat sans rapport, la fenêtre s'arrête ici.
        // Un combat de boss (ce donjon ou un autre) interrompt toujours ce ramassage.
        let room_slots = dungeon.room_count() - 1;
        let mut rooms_found = 0;
        let mut current_room_key: Option<String> = None;
        while j < records.len() {
            if find_dungeon(records[j].id).is_some() {
                break;
            }
            if records[j].won {
                if rooms_found >= room_slots {
                    break;
                }
                current_room_key = Some(room_composition_key(records[j].id));
                rooms_found += 1;
                j += 1;
                continue;
            }
            if current_room_key.as_deref() == Some(&room_composition_key(records[j].id)) {
                j += 1;
                continue;
            }
            break;
        }

        for slot in consumed.iter_mut().take(j).skip(i) {
            *slot = true;
        }

        let span = &records[i..j];
        if span.len() <= 1 {
            entries.push(DungeonHistoryEntry::Single(records[i].id));
        } else {
            entries.push(DungeonHistoryEntry::DungeonRun {
                dungeon,
                fight_ids: span.iter().map(|f| f.id).collect(),
            });
        }
        i += 1;
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dungeons_fixture() -> DungeonIndex {
        DungeonIndex::from_json(&serde_json::json!([
            { "id": 65, "fr": "Larventura", "en": "x", "es": "x", "pt": "x",
              "bossMonsterId": [200], "monsterFamilyId": [], "type": "TWO_ROOMS",
              "hasPreBossArchi": false },
            { "id": 70, "fr": "Kokokolantha", "en": "x", "es": "x", "pt": "x",
              "bossMonsterId": [300], "monsterFamilyId": [], "type": "TWO_ROOMS",
              "hasPreBossArchi": true },
            { "id": 12, "fr": "Arcade", "en": "x", "es": "x", "pt": "x",
              "bossMonsterId": [400], "monsterFamilyId": [], "type": "ULTIMATE_BOSS" },
        ]))
    }

    fn catalog_fixture() -> CatalogIndex {
        CatalogIndex::from_compact_json(&serde_json::json!({
            "monsters": [
                [200, "Boss Larventura", "x", "x", "x", "1", -1, 1, 0, 0],
                [201, "Salle Larventura", "x", "x", "x", "2", -1, 0, 0, 0],
                [300, "Boss Koko", "x", "x", "x", "3", -1, 1, 0, 0],
                [301, "Archi Koko", "x", "x", "x", "4", -1, 0, 1, 0],
                [400, "Boss Ultime", "x", "x", "x", "5", -1, 1, 0, 0],
            ],
        }))
    }

    /// Assemble les fermetures `find_dungeon`/`has_archi_enemy`/`room_composition_key` attendues
    /// par `group_dungeon_runs`, à partir d'une table `fight_id -> noms d'ennemis` — pratique pour
    /// ces tests, où chaque combat est décrit par ses seuls ennemis.
    #[allow(clippy::type_complexity)] // triplet de fermetures, factoriser un alias n'aiderait pas la lisibilité ici
    fn closures<'a>(
        catalog: &'a CatalogIndex,
        dungeons: &'a DungeonIndex,
        enemies_by_fight: &'a std::collections::HashMap<i64, Vec<String>>,
    ) -> (
        impl Fn(i64) -> Option<&'a DungeonEntry> + 'a,
        impl Fn(i64) -> bool + 'a,
        impl Fn(i64) -> String + 'a,
    ) {
        let find_dungeon = move |id: i64| {
            enemies_by_fight
                .get(&id)
                .and_then(|names| find_dungeon_for_enemies(catalog, dungeons, names))
        };
        let has_archi_enemy = move |id: i64| {
            enemies_by_fight
                .get(&id)
                .is_some_and(|names| names.iter().any(|n| catalog.find_monster_is_archi(n, None)))
        };
        let room_composition_key = move |id: i64| {
            enemies_by_fight
                .get(&id)
                .map(|names| enemy_composition_key(names))
                .unwrap_or_default()
        };
        (find_dungeon, has_archi_enemy, room_composition_key)
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn une_salle_suivie_de_son_boss_forme_un_seul_run() {
        let catalog = catalog_fixture();
        let dungeons = dungeons_fixture();
        // Plus récent en premier : boss (id 2) puis salle (id 1).
        let enemies = std::collections::HashMap::from([
            (2, names(&["Boss Larventura"])),
            (1, names(&["Salle Larventura"])),
        ]);
        let (find_dungeon, has_archi, room_key) = closures(&catalog, &dungeons, &enemies);
        let records = [
            GroupableFight { id: 2, won: true },
            GroupableFight { id: 1, won: true },
        ];

        let entries = group_dungeon_runs(&records, find_dungeon, has_archi, room_key);

        assert_eq!(entries.len(), 1);
        match &entries[0] {
            DungeonHistoryEntry::DungeonRun { dungeon, fight_ids } => {
                assert_eq!(dungeon.id, 65);
                assert_eq!(fight_ids, &[2, 1]);
            }
            other => panic!("run attendu, obtenu {other:?}"),
        }
    }

    #[test]
    fn plusieurs_defaites_contre_le_boss_rejoignent_le_meme_run() {
        let catalog = catalog_fixture();
        let dungeons = dungeons_fixture();
        // Plus récent en premier : victoire (3), défaite (2), salle (1).
        let enemies = std::collections::HashMap::from([
            (3, names(&["Boss Larventura"])),
            (2, names(&["Boss Larventura"])),
            (1, names(&["Salle Larventura"])),
        ]);
        let (find_dungeon, has_archi, room_key) = closures(&catalog, &dungeons, &enemies);
        let records = [
            GroupableFight { id: 3, won: true },
            GroupableFight { id: 2, won: false },
            GroupableFight { id: 1, won: true },
        ];

        let entries = group_dungeon_runs(&records, find_dungeon, has_archi, room_key);

        assert_eq!(entries.len(), 1);
        match &entries[0] {
            DungeonHistoryEntry::DungeonRun { fight_ids, .. } => {
                assert_eq!(fight_ids, &[3, 2, 1]);
            }
            other => panic!("run attendu, obtenu {other:?}"),
        }
    }

    #[test]
    fn une_victoire_plus_ancienne_contre_le_meme_boss_appartient_a_un_autre_run() {
        let catalog = catalog_fixture();
        let dungeons = dungeons_fixture();
        // Deux clears complets et distincts du même donjon : [boss2, salle2] puis [boss1, salle1].
        let enemies = std::collections::HashMap::from([
            (4, names(&["Boss Larventura"])),
            (3, names(&["Salle Larventura"])),
            (2, names(&["Boss Larventura"])),
            (1, names(&["Salle Larventura"])),
        ]);
        let (find_dungeon, has_archi, room_key) = closures(&catalog, &dungeons, &enemies);
        let records = [
            GroupableFight { id: 4, won: true },
            GroupableFight { id: 3, won: true },
            GroupableFight { id: 2, won: true },
            GroupableFight { id: 1, won: true },
        ];

        let entries = group_dungeon_runs(&records, find_dungeon, has_archi, room_key);

        assert_eq!(entries.len(), 2, "deux runs distincts, jamais fusionnés");
        for entry in &entries {
            match entry {
                DungeonHistoryEntry::DungeonRun { fight_ids, .. } => {
                    assert_eq!(fight_ids.len(), 2);
                }
                other => panic!("run attendu, obtenu {other:?}"),
            }
        }
    }

    #[test]
    fn archimonstre_pre_boss_rejoint_le_run_seulement_sil_est_reellement_present() {
        let catalog = catalog_fixture();
        let dungeons = dungeons_fixture();
        // Kokokolantha (id 70) : `hasPreBossArchi = true`. Boss (2) précédé d'un VRAI combat
        // d'archimonstre (1).
        let enemies = std::collections::HashMap::from([
            (2, names(&["Boss Koko"])),
            (1, names(&["Archi Koko"])),
        ]);
        let (find_dungeon, has_archi, room_key) = closures(&catalog, &dungeons, &enemies);
        let records = [
            GroupableFight { id: 2, won: true },
            GroupableFight { id: 1, won: true },
        ];

        let entries = group_dungeon_runs(&records, find_dungeon, has_archi, room_key);

        match &entries[0] {
            DungeonHistoryEntry::DungeonRun { fight_ids, .. } => {
                assert_eq!(fight_ids, &[2, 1], "l'archi pré-boss rejoint bien le run")
            }
            other => panic!("run attendu, obtenu {other:?}"),
        }
    }

    /// Bug réel corrigé côté web (voir la doc de `group_dungeon_runs`) : l'ancienne version
    /// rattachait un combat supplémentaire dès que `hasPreBossArchi` était vrai, MÊME sans
    /// archimonstre dedans — consommant alors 1 combat de trop (boss + faux-archi + vraie salle)
    /// au lieu de `1 + room_slots`. Ici, Kokokolantha (`TWO_ROOMS`, `room_slots = 1`) : boss (3),
    /// un combat normal SANS archimonstre (2), puis une vraie salle plus ancienne encore (1).
    #[test]
    fn archimonstre_pre_boss_absent_ne_consomme_jamais_un_combat_de_plus() {
        let catalog = catalog_fixture();
        let dungeons = dungeons_fixture();
        let enemies = std::collections::HashMap::from([
            (3, names(&["Boss Koko"])),
            (2, names(&["Salle Larventura"])), // pas d'archi ici
            (1, names(&["Salle Larventura"])),
        ]);
        let (find_dungeon, has_archi, room_key) = closures(&catalog, &dungeons, &enemies);
        let records = [
            GroupableFight { id: 3, won: true },
            GroupableFight { id: 2, won: true },
            GroupableFight { id: 1, won: true },
        ];

        let entries = group_dungeon_runs(&records, find_dungeon, has_archi, room_key);

        // `room_slots = 1` : le combat 2 le remplit (voie "salle", pas "archi pré-boss" puisque
        // sans archimonstre) — le combat 1 reste HORS de ce run, jamais un 3ᵉ combat rattaché à
        // tort. Sans le correctif, les 3 auraient été regroupés.
        match &entries[0] {
            DungeonHistoryEntry::DungeonRun { fight_ids, .. } => {
                assert_eq!(
                    fight_ids,
                    &[3, 2],
                    "pas de combat en trop sans archimonstre réel"
                );
            }
            other => panic!("run attendu, obtenu {other:?}"),
        }
        assert!(
            matches!(entries.get(1), Some(DungeonHistoryEntry::Single(1))),
            "le combat 1, non consommé, redevient une entrée isolée à ce passage"
        );
    }

    #[test]
    fn donjon_a_un_seul_combat_nest_jamais_regroupe() {
        let catalog = catalog_fixture();
        let dungeons = dungeons_fixture();
        let enemies = std::collections::HashMap::from([
            (2, names(&["Boss Ultime"])),
            (1, names(&["Boss Ultime"])), // tentative précédente, même boss ultime
        ]);
        let (find_dungeon, has_archi, room_key) = closures(&catalog, &dungeons, &enemies);
        let records = [
            GroupableFight { id: 2, won: true },
            GroupableFight { id: 1, won: false },
        ];

        let entries = group_dungeon_runs(&records, find_dungeon, has_archi, room_key);

        assert_eq!(entries.len(), 2);
        assert!(entries
            .iter()
            .all(|e| matches!(e, DungeonHistoryEntry::Single(_))));
    }

    #[test]
    fn combat_hors_donjon_reste_isole() {
        let catalog = catalog_fixture();
        let dungeons = dungeons_fixture();
        let enemies = std::collections::HashMap::from([(1, names(&["Mouton"]))]);
        let (find_dungeon, has_archi, room_key) = closures(&catalog, &dungeons, &enemies);
        let records = [GroupableFight { id: 1, won: true }];

        let entries = group_dungeon_runs(&records, find_dungeon, has_archi, room_key);

        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0], DungeonHistoryEntry::Single(1)));
    }

    #[test]
    fn salle_sans_boss_pas_encore_connu_reste_isolee_pour_linstant() {
        let catalog = catalog_fixture();
        let dungeons = dungeons_fixture();
        // Seule la salle est connue de l'historique pour l'instant (le boss n'a pas encore été
        // combattu) : `findDungeon` ne trouve rien pour elle (pas de boss dans ses ennemis).
        let enemies = std::collections::HashMap::from([(1, names(&["Salle Larventura"]))]);
        let (find_dungeon, has_archi, room_key) = closures(&catalog, &dungeons, &enemies);
        let records = [GroupableFight { id: 1, won: true }];

        let entries = group_dungeon_runs(&records, find_dungeon, has_archi, room_key);

        assert!(matches!(entries[0], DungeonHistoryEntry::Single(1)));
    }

    #[test]
    fn enemy_composition_key_ignore_ordre_et_doublons() {
        assert_eq!(
            enemy_composition_key(&names(&["B", "A", "B"])),
            enemy_composition_key(&names(&["A", "B"]))
        );
        assert_ne!(
            enemy_composition_key(&names(&["A", "B"])),
            enemy_composition_key(&names(&["A", "C"]))
        );
    }

    #[test]
    fn find_dungeon_for_enemies_priorite_1_boss_seul() {
        let catalog = catalog_fixture();
        let dungeons = dungeons_fixture();
        let dungeon =
            find_dungeon_for_enemies(&catalog, &dungeons, &names(&["Boss Larventura"])).unwrap();
        assert_eq!(dungeon.id, 65);
    }

    #[test]
    fn find_dungeon_for_enemies_priorite_0_breche_ultime_avant_priorite_1() {
        let catalog = CatalogIndex::from_compact_json(&serde_json::json!({
            "monsters": [
                [1, "BossA", "x", "x", "x", "1", -1, 1, 0, 0],
                [2, "BossB", "x", "x", "x", "2", -1, 1, 0, 0],
            ],
        }));
        let dungeons = DungeonIndex::from_json(&serde_json::json!([
            { "id": 900, "fr": "Ultime", "en": "x", "es": "x", "pt": "x",
              "bossMonsterId": [1, 2], "monsterFamilyId": [], "type": "ULTIMATE_BREACH" },
            // Un donjon classique qui référence AUSSI le boss 1 : sans la priorité 0, ce donjon
            // serait trouvé en premier (bug réel documenté côté web, cas "Phacochemar").
            { "id": 10, "fr": "Classique", "en": "x", "es": "x", "pt": "x",
              "bossMonsterId": [1], "monsterFamilyId": [], "type": "TWO_ROOMS" },
        ]));
        let dungeon =
            find_dungeon_for_enemies(&catalog, &dungeons, &names(&["BossA", "BossB"])).unwrap();
        assert_eq!(
            dungeon.id, 900,
            "brèche ultime prioritaire sur le donjon classique partagé"
        );
    }

    #[test]
    fn find_dungeon_for_enemies_priorite_2_horde_heterogene() {
        let catalog = CatalogIndex::from_compact_json(&serde_json::json!({
            "monsters": [
                [1, "M1", "x", "x", "x", "1", 10, 0, 0, 0],
                [2, "M2", "x", "x", "x", "2", 20, 0, 0, 0],
                [3, "M3", "x", "x", "x", "3", 30, 0, 0, 0],
                [4, "M4", "x", "x", "x", "4", 40, 0, 0, 0],
                [5, "M5", "x", "x", "x", "5", 50, 0, 0, 0],
            ],
        }));
        let dungeons = DungeonIndex::from_json(&serde_json::json!([
            { "id": 50, "fr": "Brèche", "en": "x", "es": "x", "pt": "x",
              "bossMonsterId": [], "monsterFamilyId": [10, 20, 30, 40, 50], "type": "BREACH" },
        ]));
        let dungeon =
            find_dungeon_for_enemies(&catalog, &dungeons, &names(&["M1", "M2", "M3", "M4", "M5"]))
                .unwrap();
        assert_eq!(dungeon.id, 50);
    }

    #[test]
    fn find_dungeon_for_enemies_horde_sous_le_seuil_ne_declenche_pas_la_priorite_2() {
        let catalog = CatalogIndex::from_compact_json(&serde_json::json!({
            "monsters": [
                [1, "M1", "x", "x", "x", "1", 10, 0, 0, 0],
                [2, "M2", "x", "x", "x", "2", 20, 0, 0, 0],
            ],
        }));
        let dungeons = DungeonIndex::from_json(&serde_json::json!([
            { "id": 50, "fr": "Brèche", "en": "x", "es": "x", "pt": "x",
              "bossMonsterId": [], "monsterFamilyId": [10, 20], "type": "BREACH" },
        ]));
        assert!(find_dungeon_for_enemies(&catalog, &dungeons, &names(&["M1", "M2"])).is_none());
    }

    #[test]
    fn find_dungeon_for_enemies_aucun_ennemi_resolu_renvoie_none() {
        let catalog = catalog_fixture();
        let dungeons = dungeons_fixture();
        assert!(find_dungeon_for_enemies(&catalog, &dungeons, &names(&["Inconnu"])).is_none());
    }
}
