//! **Référentiel embarqué** (2026-11-27, `docs/plan-historique-local.md` §4) — objets, monstres,
//! recettes, donjons et familles de monstres, en français, compilés dans le binaire.
//!
//! Remplace les appels à `GET /api/v1/catalog/`, `/items/{id}`, `/dungeons` et
//! `/monster-families` : l'overlay n'a plus besoin ni de réseau ni de compte pour nommer un objet,
//! retrouver son icône ou développer une recette. Le référentiel suit les releases ; on le
//! régénère avec `tools/referentiel` (voir sa doc) à chaque mise à jour du dépôt web.
//!
//! Le fichier fait ~1 Mo de JSON, une entrée par ligne (diffs lisibles). Il est lu une seule fois,
//! au démarrage, sur un thread de fond : [`load`] construit les trois index d'un coup, puis l'arbre
//! JSON intermédiaire est libéré.

use crate::catalog::CatalogIndex;
use crate::dungeon::DungeonIndex;
use crate::monster_family::MonsterFamilyIndex;

const REFERENTIEL_JSON: &str = include_str!("../../../assets/referentiel.json");

/// Les index construits depuis le référentiel embarqué.
pub struct Referentiel {
    pub catalog: CatalogIndex,
    pub dungeons: DungeonIndex,
    pub monster_families: MonsterFamilyIndex,
}

/// Construit les index du référentiel embarqué. Un fichier illisible (impossible en pratique : il
/// est validé par les tests) donne des index vides plutôt qu'une panique — l'overlay retombe alors
/// sur ses icônes génériques.
pub fn load() -> Referentiel {
    let data: serde_json::Value = serde_json::from_str(REFERENTIEL_JSON).unwrap_or_else(|err| {
        tracing::error!(%err, "référentiel embarqué illisible");
        serde_json::Value::Null
    });
    let empty = serde_json::Value::Array(Vec::new());
    Referentiel {
        catalog: CatalogIndex::from_json(&data),
        dungeons: DungeonIndex::from_json(data.get("dungeons").unwrap_or(&empty)),
        monster_families: MonsterFamilyIndex::from_json(
            data.get("monsterFamilies").unwrap_or(&empty),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{IconKind, MonsterClassification, WakfuItemCategory, WakfuRarity};

    /// Le référentiel embarqué se lit en entier : un fichier tronqué ou mal régénéré casserait
    /// silencieusement toutes les icônes et recettes.
    #[test]
    fn le_referentiel_embarque_se_charge_en_entier() {
        let referentiel = load();
        let catalog = &referentiel.catalog;
        assert!(catalog.search_items("bouftou", 3, usize::MAX).len() > 10);
        assert!(!catalog.search_monsters("bouftou", 3, usize::MAX).is_empty());
        assert!(!referentiel.dungeons.is_empty());
        assert!(!referentiel.monster_families.is_empty());
    }

    /// Ancrages réels du référentiel — les mêmes valeurs que servait l'API du site.
    #[test]
    fn ancrages_reels_du_referentiel() {
        let catalog = load().catalog;
        // Bois de Pommier : ressource commune, icône 2011718.
        let icon = catalog
            .find_item_icon("Bois de Pommier", Some(1718))
            .unwrap();
        assert_eq!(icon.kind, IconKind::Item);
        assert_eq!(icon.gfx_id, "2011718");
        assert_eq!(
            catalog.find_item_rarity("bois de pommier", None),
            WakfuRarity::Common
        );
        assert_eq!(
            catalog.find_item_category("bois de pommier", None),
            WakfuItemCategory::Harvests
        );
        // Bouftou : monstre de la famille 1, sans classement particulier.
        assert_eq!(catalog.find_monster_id("Bouftou"), Some(1));
        assert_eq!(catalog.find_monster_family_id("bouftou", None), Some(1));
        assert_eq!(
            catalog.find_monster_classification("bouftou", None),
            MonsterClassification::None
        );
        // Larventura : donjon 65, boss 2157.
        let dungeons = load().dungeons;
        assert_eq!(dungeons.find_by_id(65).unwrap().fr, "Larventura");
        assert_eq!(dungeons.find_by_boss_monster_id(2157).unwrap().id, 65);
    }

    /// Les recettes désignent des ingrédients que le catalogue sait nommer — sinon la fenêtre
    /// « Objets de la recette » perdrait des lignes. Le référentiel source en contient quelques-uns
    /// d'inconnus (36 sur 33 388 au 2026-11-27, objets retirés du jeu) : ils sont omis, comme sur
    /// le site ; le test garde un œil sur leur proportion.
    #[test]
    fn les_recettes_ont_des_ingredients_connus() {
        let catalog = load().catalog;
        let craftables: Vec<_> = catalog
            .search_items("epee", 3, usize::MAX)
            .into_iter()
            .filter(|item| catalog.find_item_has_recipe(&item.name, Some(item.id)))
            .collect();
        assert!(!craftables.is_empty());
        let (mut connus, mut inconnus) = (0, 0);
        for item in &craftables {
            let recipe = catalog.recipe(item.id);
            assert!(!recipe.is_empty(), "{} n'a pas d'ingrédients", item.name);
            for (ingredient, quantity) in recipe {
                assert!(*quantity > 0);
                match catalog.item_name(*ingredient) {
                    Some(_) => connus += 1,
                    None => inconnus += 1,
                }
            }
            assert!(!crate::recipe::resolve_recipe(item.id, &catalog).is_empty());
        }
        assert!(
            inconnus * 100 < connus,
            "{inconnus} ingrédients inconnus pour {connus} connus"
        );
    }
}
