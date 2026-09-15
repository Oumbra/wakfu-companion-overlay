//! Référentiel des familles de monstres (lot L3, §7.4 du plan — même volet « reste » que
//! `dungeon.rs`, voir sa doc de tête). Miroir réduit de `CatalogMonsterFamilyEntry`
//! (`fight-image.util.ts` côté `wakfu-companion`) : résout le nom localisé d'une famille à partir
//! de son id (`catalog::CatalogIndex::find_monster_family_id`) — sert au même futur consommateur
//! que `dungeon.rs` (panneau conscient du donjon/de la famille, pas encore construit).
//!
//! Source : `GET /api/v1/monster-families` (voir `overlay_sync::client::fetch_monster_families`),
//! réponse `Vec<{id, fr, en, es, pt, pictureUrl: string|null}>` (voir `functions/api/v1/
//! monster-families.ts`) — même remarque que `dungeon.rs` : un tableau d'objets complets, pas des
//! tuples compacts (volume négligeable, ~150 lignes).

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct RawMonsterFamilyRow {
    id: i64,
    fr: String,
    en: String,
    es: String,
    pt: String,
}

/// Une famille de monstre résolue — mêmes remarques que `DungeonEntry` (4 locales toutes
/// disponibles, pas de sélection ici faute d'i18n overlay). `picture_url` (référentiel curé à la
/// main côté serveur, pas garanti pour toute famille — voir `server/db/schema.ts`) volontairement
/// absent : aucun consommateur overlay n'affiche encore d'illustration de famille.
#[derive(Debug, Clone, PartialEq)]
pub struct MonsterFamilyEntry {
    pub id: i64,
    pub fr: String,
    pub en: String,
    pub es: String,
    pub pt: String,
}

/// Index en RAM des familles de monstres — O(1) par id, même politique de tolérance aux lignes mal
/// formées que `catalog::CatalogIndex`/`dungeon::DungeonIndex`.
#[derive(Default)]
pub struct MonsterFamilyIndex {
    by_id: HashMap<i64, MonsterFamilyEntry>,
}

impl MonsterFamilyIndex {
    pub fn from_json(data: &serde_json::Value) -> Self {
        let mut index = Self::default();
        let rows: Vec<RawMonsterFamilyRow> =
            serde_json::from_value(data.clone()).unwrap_or_default();
        for row in rows {
            index.by_id.insert(
                row.id,
                MonsterFamilyEntry {
                    id: row.id,
                    fr: row.fr,
                    en: row.en,
                    es: row.es,
                    pt: row.pt,
                },
            );
        }
        index
    }

    pub fn find_by_id(&self, id: i64) -> Option<&MonsterFamilyEntry> {
        self.by_id.get(&id)
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> serde_json::Value {
        serde_json::json!([
            { "id": 42, "fr": "Bworks", "en": "Bworks", "es": "Bworks", "pt": "Bworks", "pictureUrl": null },
            { "id": 3, "fr": "Boss Ultimes", "en": "Ultimate Bosses", "es": "Jefes Últimos", "pt": "Chefes Finais", "pictureUrl": null },
        ])
    }

    #[test]
    fn resout_une_famille_par_id() {
        let index = MonsterFamilyIndex::from_json(&sample());
        assert_eq!(index.find_by_id(42).unwrap().fr, "Bworks");
    }

    #[test]
    fn famille_inconnue_renvoie_none() {
        let index = MonsterFamilyIndex::from_json(&sample());
        assert!(index.find_by_id(999999).is_none());
    }

    #[test]
    fn referentiel_absent_ou_vide_ne_plante_pas() {
        let index = MonsterFamilyIndex::from_json(&serde_json::json!([]));
        assert!(index.is_empty());
    }
}
