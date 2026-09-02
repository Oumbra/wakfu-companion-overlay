//! Référentiel des donjons (lot L3, §7.4 du plan — volet « reste » : recettes/familles de
//! monstres/donjons, voir `catalog.rs` en tête de module). Miroir réduit de
//! `CatalogService.findWakfuDungeonEntryById`/`findWakfuDungeonByBossMonsterId`/
//! `findWakfuBreachByMonsterFamilies`/`findWakfuUltimateBreachByBossMonsters` (`catalog.service.ts`) :
//! `level`/`bracket`/`pictureUrl` restent hors de ce module tant qu'aucun panneau (§9 du plan, PAS
//! encore de panneau « donjon ») n'en a besoin — `type`/`hasPreBossArchi`, en revanche, sont
//! désormais portés (2026-09-02) : nécessaires à `dungeon_run.rs` pour le regroupement de combats
//! (L5, §7.1).
//!
//! Consommé par `overlay-engine::session` (L5, §7.1 — assignation `dungeonId`/`dungeonRunKey`),
//! **pas câblé côté `overlay-ui` par défaut de données** : voir `Engine::set_dungeons`/
//! `spawn_dungeon_thread` (`overlay-ui/src/main.rs`) pour le chargement réel (`GET
//! /api/v1/dungeons`, voir `overlay_sync::client::fetch_dungeons`), réponse `Vec<{id, fr, en, es,
//! pt, bossMonsterId: number[], monsterFamilyId: number[], type, hasPreBossArchi, ...}>` (voir
//! `functions/api/v1/dungeons.ts` côté `wakfu-companion`) — un tableau d'OBJETS complets, PAS des
//! tuples compacts comme `catalog::CatalogIndex` (volume négligeable, ~151 lignes, pas besoin de
//! compacité ici — voir la doc de `dungeons.ts`).

use std::collections::HashMap;

use serde::Deserialize;

/// Catégorie d'un donjon — miroir de `WakfuDungeonType` (`catalog.service.ts`, dépôt web).
/// `TwoRooms`/`ThreeRooms`/`FourRooms` portent le nombre de salles précédant le boss (voir
/// `room_count`, `dungeon_run.rs`) ; `Breach`/`UltimateBreach` remplacent les anciens booléens
/// `isBreach`/`isUltimateBreach` ; `ThreePlayers`/`UltimateBoss`/`Arcade` désignent des donjons à
/// un seul combat (aucune salle à rattacher).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WakfuDungeonType {
    TwoRooms,
    ThreeRooms,
    FourRooms,
    ThreePlayers,
    UltimateBoss,
    Breach,
    UltimateBreach,
    Arcade,
}

impl WakfuDungeonType {
    /// Nombre de salles précédant le boss (boss compris) pour un clear "propre" — miroir de
    /// `ROOM_COUNT_BY_TYPE`/`dungeonRoomCount` (`dungeon-run-grouping.util.ts`). Seuls les trois
    /// types "à salles" ont plus d'un combat ; tous les autres désignent un donjon à un seul combat.
    pub fn room_count(self) -> usize {
        match self {
            WakfuDungeonType::TwoRooms => 2,
            WakfuDungeonType::ThreeRooms => 3,
            WakfuDungeonType::FourRooms => 4,
            WakfuDungeonType::ThreePlayers
            | WakfuDungeonType::UltimateBoss
            | WakfuDungeonType::Breach
            | WakfuDungeonType::UltimateBreach
            | WakfuDungeonType::Arcade => 1,
        }
    }
}

/// Une ligne de `GET /api/v1/dungeons` — seuls les champs consommés ici sont déclarés ; `serde`
/// ignore silencieusement les autres clés du JSON (`level`, `bracket`, `pictureUrl`,
/// `wakassetsAvailable`), pas de désérialisation positionnelle stricte à respecter contrairement à
/// `catalog::RawItemRow`/`RawMonsterRow`.
#[derive(Debug, Clone, Deserialize)]
struct RawDungeonRow {
    id: i64,
    fr: String,
    en: String,
    es: String,
    pt: String,
    #[serde(rename = "bossMonsterId", default)]
    boss_monster_id: Vec<i64>,
    #[serde(rename = "monsterFamilyId", default)]
    monster_family_id: Vec<i64>,
    #[serde(rename = "type")]
    dungeon_type: WakfuDungeonType,
    #[serde(rename = "hasPreBossArchi", default)]
    has_pre_boss_archi: bool,
}

/// Un donjon résolu — les 4 noms restent tous les 4 disponibles (pas de sélection de locale ici :
/// l'overlay n'a pour l'instant aucun système d'i18n, contrairement au web — voir `roster.rs`/
/// `watchlist.rs`, qui n'en ont pas non plus) ; un futur consommateur choisit le champ voulu.
#[derive(Debug, Clone, PartialEq)]
pub struct DungeonEntry {
    pub id: i64,
    pub fr: String,
    pub en: String,
    pub es: String,
    pub pt: String,
    /// Toujours un tableau, jamais absent — miroir de `dungeons.bossMonsterId` (`server/db/
    /// schema.ts`, normalisé côté serveur), vide pour un type de donjon sans boss (BREACH, ARCADE).
    pub boss_monster_ids: Vec<i64>,
    pub monster_family_ids: Vec<i64>,
    pub dungeon_type: WakfuDungeonType,
    /// Vrai pour les rares donjons avec un combat d'archimonstre supplémentaire avant le boss (ex.
    /// Kokokolantha) — voir `dungeon_run.rs::group_dungeon_runs`.
    pub has_pre_boss_archi: bool,
}

impl DungeonEntry {
    /// Miroir de `dungeonRoomCount` — voir `WakfuDungeonType::room_count`.
    pub fn room_count(&self) -> usize {
        self.dungeon_type.room_count()
    }
}

/// Index en RAM des donjons — O(1) par id ET par id de monstre-boss (miroir de
/// `findWakfuDungeonByBossMonsterId`, qui balaie côté web faute d'un tel index ; construit ici en
/// une seule passe pour ne pas réintroduire le piège de balayage documenté sur `catalog.rs`). Les
/// recherches de brèche (`find_ultimate_breach_by_boss_monsters`/`find_breach_by_monster_families`)
/// restent, elles, un balayage linéaire — miroir exact du web (`this.dungeons.find(...)`), le
/// volume (~150 entrées) rendant un index dédié inutile pour un besoin qui ne survient qu'une fois
/// par fin de combat, jamais par ligne de log.
#[derive(Default)]
pub struct DungeonIndex {
    by_id: HashMap<i64, DungeonEntry>,
    dungeon_id_by_boss_monster_id: HashMap<i64, i64>,
}

impl DungeonIndex {
    /// Construit l'index depuis la réponse brute de `GET /api/v1/dungeons` (un tableau JSON
    /// d'objets). Une ligne mal formée est silencieusement ignorée (même politique que
    /// `catalog::CatalogIndex::from_compact_json`) — un référentiel de donjons partiel reste
    /// préférable à aucun référentiel du tout.
    pub fn from_json(data: &serde_json::Value) -> Self {
        let mut index = Self::default();
        let rows: Vec<RawDungeonRow> = serde_json::from_value(data.clone()).unwrap_or_default();
        for row in rows {
            for boss_id in &row.boss_monster_id {
                index.dungeon_id_by_boss_monster_id.insert(*boss_id, row.id);
            }
            index.by_id.insert(
                row.id,
                DungeonEntry {
                    id: row.id,
                    fr: row.fr,
                    en: row.en,
                    es: row.es,
                    pt: row.pt,
                    boss_monster_ids: row.boss_monster_id,
                    monster_family_ids: row.monster_family_id,
                    dungeon_type: row.dungeon_type,
                    has_pre_boss_archi: row.has_pre_boss_archi,
                },
            );
        }
        index
    }

    pub fn find_by_id(&self, id: i64) -> Option<&DungeonEntry> {
        self.by_id.get(&id)
    }

    /// Résout le donjon d'un combat de boss à partir de l'id du monstre-boss rencontré — miroir de
    /// `findWakfuDungeonByBossMonsterId`. `None` si ce monstre n'est le boss d'aucun donjon connu
    /// (immense majorité des monstres, un donjon n'a qu'un ou deux boss).
    pub fn find_by_boss_monster_id(&self, boss_monster_id: i64) -> Option<&DungeonEntry> {
        self.dungeon_id_by_boss_monster_id
            .get(&boss_monster_id)
            .and_then(|dungeon_id| self.by_id.get(dungeon_id))
    }

    /// Brèche ULTIME dont les boss couvrent EXACTEMENT les ids donnés — miroir de
    /// `findWakfuUltimateBreachByBossMonsters`. `None` si `boss_ids` est vide ou si aucune brèche
    /// ultime connue ne couvre entièrement cet ensemble.
    pub fn find_ultimate_breach_by_boss_monsters(&self, boss_ids: &[i64]) -> Option<&DungeonEntry> {
        if boss_ids.is_empty() {
            return None;
        }
        self.by_id.values().find(|dungeon| {
            dungeon.dungeon_type == WakfuDungeonType::UltimateBreach
                && boss_ids
                    .iter()
                    .all(|id| dungeon.boss_monster_ids.contains(id))
        })
    }

    /// Brèche SIMPLE dont les familles couvrent EXACTEMENT les ids donnés — miroir de
    /// `findWakfuBreachByMonsterFamilies`. Même règle de correspondance : TOUTES les familles
    /// observées doivent figurer dans le référentiel d'une même brèche.
    pub fn find_breach_by_monster_families(&self, family_ids: &[i64]) -> Option<&DungeonEntry> {
        if family_ids.is_empty() {
            return None;
        }
        self.by_id.values().find(|dungeon| {
            dungeon.dungeon_type == WakfuDungeonType::Breach
                && family_ids
                    .iter()
                    .all(|id| dungeon.monster_family_ids.contains(id))
        })
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Id 65 = "Larventura" (référentiel réel, déjà utilisé comme donjon de vérification côté web —
    // voir CLAUDE.md de wakfu-companion, carte Récap) : ancrage réel plutôt qu'un id inventé, même
    // si `bossMonsterId`/`monsterFamilyId` ci-dessous restent des valeurs de fixture (aucun accès
    // réseau à la vraie base Neon depuis ce sandbox pour les vérifier — voir `server/README.md`).
    fn sample() -> serde_json::Value {
        serde_json::json!([
            {
                "id": 65,
                "fr": "Larventura",
                "en": "Larventura",
                "es": "Larventura",
                "pt": "Larventura",
                "level": 155,
                "bracket": 3,
                "bossMonsterId": [24875],
                "monsterFamilyId": [42],
                "pictureUrl": "https://example.invalid/larventura.png",
                "wakassetsAvailable": true,
                "type": "TWO_ROOMS",
                "hasPreBossArchi": false,
            },
            {
                "id": 12,
                "fr": "Sans boss",
                "en": "No boss",
                "es": "Sin jefe",
                "pt": "Sem chefe",
                "bossMonsterId": [],
                "monsterFamilyId": [],
                "type": "ARCADE",
            },
            {
                "id": 900,
                "fr": "Brèche de test",
                "en": "Test breach",
                "es": "x",
                "pt": "x",
                "bossMonsterId": [],
                "monsterFamilyId": [42, 43],
                "type": "BREACH",
            },
            {
                "id": 901,
                "fr": "Brèche ultime de test",
                "en": "Ultimate test breach",
                "es": "x",
                "pt": "x",
                "bossMonsterId": [111, 222],
                "monsterFamilyId": [],
                "type": "ULTIMATE_BREACH",
            },
        ])
    }

    #[test]
    fn resout_un_donjon_par_id() {
        let index = DungeonIndex::from_json(&sample());
        let dungeon = index.find_by_id(65).unwrap();
        assert_eq!(dungeon.fr, "Larventura");
        assert_eq!(dungeon.boss_monster_ids, vec![24875]);
    }

    #[test]
    fn resout_un_donjon_par_id_de_monstre_boss() {
        let index = DungeonIndex::from_json(&sample());
        let dungeon = index.find_by_boss_monster_id(24875).unwrap();
        assert_eq!(dungeon.id, 65);
    }

    #[test]
    fn monstre_qui_nest_boss_daucun_donjon_renvoie_none() {
        let index = DungeonIndex::from_json(&sample());
        assert!(index.find_by_boss_monster_id(999999).is_none());
    }

    #[test]
    fn donjon_sans_boss_ni_famille_reste_resolu_par_id() {
        let index = DungeonIndex::from_json(&sample());
        let dungeon = index.find_by_id(12).unwrap();
        assert!(dungeon.boss_monster_ids.is_empty());
        assert!(dungeon.monster_family_ids.is_empty());
    }

    #[test]
    fn room_count_reflete_le_type_du_donjon() {
        let index = DungeonIndex::from_json(&sample());
        assert_eq!(index.find_by_id(65).unwrap().room_count(), 2); // TWO_ROOMS
        assert_eq!(index.find_by_id(12).unwrap().room_count(), 1); // ARCADE
        assert_eq!(index.find_by_id(900).unwrap().room_count(), 1); // BREACH
    }

    #[test]
    fn resout_une_breche_simple_par_familles_couvertes() {
        let index = DungeonIndex::from_json(&sample());
        let breach = index.find_breach_by_monster_families(&[42, 43]).unwrap();
        assert_eq!(breach.id, 900);
        // Sous-ensemble des familles couvertes : match toujours valide (TOUTES doivent figurer
        // dans le référentiel, pas l'inverse).
        assert_eq!(
            index.find_breach_by_monster_families(&[42]).unwrap().id,
            900
        );
    }

    #[test]
    fn aucune_breche_simple_si_une_famille_nest_couverte_par_aucune() {
        let index = DungeonIndex::from_json(&sample());
        assert!(index.find_breach_by_monster_families(&[42, 999]).is_none());
        assert!(index.find_breach_by_monster_families(&[]).is_none());
    }

    #[test]
    fn resout_une_breche_ultime_par_boss_couverts() {
        let index = DungeonIndex::from_json(&sample());
        let breach = index
            .find_ultimate_breach_by_boss_monsters(&[111, 222])
            .unwrap();
        assert_eq!(breach.id, 901);
    }

    #[test]
    fn aucune_breche_ultime_si_un_boss_nest_couvert_par_aucune() {
        let index = DungeonIndex::from_json(&sample());
        assert!(index
            .find_ultimate_breach_by_boss_monsters(&[111, 999])
            .is_none());
        assert!(index.find_ultimate_breach_by_boss_monsters(&[]).is_none());
    }

    #[test]
    fn referentiel_absent_ou_vide_ne_plante_pas() {
        let index = DungeonIndex::from_json(&serde_json::json!([]));
        assert!(index.is_empty());
        assert!(index.find_by_id(65).is_none());
        assert!(index.find_by_boss_monster_id(24875).is_none());
    }
}
