//! Référentiel des donjons (lot L3, §7.4 du plan — volet « reste » : recettes/familles de
//! monstres/donjons, voir `catalog.rs` en tête de module). Miroir réduit de
//! `CatalogService.findWakfuDungeonEntryById`/`findWakfuDungeonByBossMonsterId` (`catalog.service.ts`) :
//! seuls l'id, les 4 noms localisés et les deux tableaux de rattachement (boss/famille) intéressent
//! un consommateur overlay pour l'instant — `level`/`bracket`/`type`/`pictureUrl`/`hasPreBossArchi`
//! restent hors de ce module tant qu'aucun panneau (§9 du plan, PAS encore de panneau « donjon »)
//! n'en a besoin, plutôt que de les porter par anticipation sans usage réel à vérifier.
//!
//! Pas encore consommé par `overlay-ui`/`overlay-engine::session` : aucun combat (`LogEntry::
//! CombatEnd`) ne porte de `dungeonId` côté parseur pour l'instant (voir `model.rs`) — cette brique
//! prépare la résolution de nom qu'un futur panneau « Dégâts de combat conscient du donjon »
//! consommera, même principe que `catalog::MonsterClassification` (construit avant son
//! consommateur). Source : `GET /api/v1/dungeons` (voir `overlay_sync::client::fetch_dungeons`),
//! réponse `Vec<{id, fr, en, es, pt, bossMonsterId: number[], monsterFamilyId: number[], ...}>`
//! (voir `functions/api/v1/dungeons.ts` côté `wakfu-companion`) — un tableau d'OBJETS complets,
//! PAS des tuples compacts comme `catalog::CatalogIndex` (volume négligeable, ~151 lignes, pas
//! besoin de compacité ici — voir la doc de `dungeons.ts`).

use std::collections::HashMap;

use serde::Deserialize;

/// Une ligne de `GET /api/v1/dungeons` — seuls les champs consommés ici sont déclarés ; `serde`
/// ignore silencieusement les autres clés du JSON (`level`, `bracket`, `type`, `pictureUrl`,
/// `wakassetsAvailable`, `hasPreBossArchi`), pas de désérialisation positionnelle stricte à
/// respecter contrairement à `catalog::RawItemRow`/`RawMonsterRow`.
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
}

/// Index en RAM des donjons — O(1) par id ET par id de monstre-boss (miroir de
/// `findWakfuDungeonByBossMonsterId`, qui balaie côté web faute d'un tel index ; construit ici en
/// une seule passe pour ne pas réintroduire le piège de balayage documenté sur `catalog.rs`).
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
                "type": "DUNGEON",
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
    fn referentiel_absent_ou_vide_ne_plante_pas() {
        let index = DungeonIndex::from_json(&serde_json::json!([]));
        assert!(index.is_empty());
        assert!(index.find_by_id(65).is_none());
        assert!(index.find_by_boss_monster_id(24875).is_none());
    }
}
