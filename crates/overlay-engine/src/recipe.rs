//! **Les ingrédients d'une recette**, résolus récursivement — ce que la fenêtre « Objets de la
//! recette » de l'onglet « Suivi » affiche, et ce qu'elle ajoute au suivi quand on la valide.
//!
//! Miroir de `CatalogService.resolveRecipeIngredients` (dépôt web, `core/api/catalog.service.ts`).
//! Les ingrédients viennent du référentiel embarqué ([`CatalogIndex::recipe`], 2026-11-27) :
//! plus d'aller-retour vers `GET /api/v1/items/{id}`, la résolution est instantanée et marche hors
//! ligne. Seuls les ingrédients qui ONT une recette sont développés à leur tour.
//!
//! ## La garde anti-cycle
//!
//! Le référentiel Ankama contient des objets qui sont ingrédients d'eux-mêmes, directement ou par
//! une chaîne plus longue (cas réel, documenté côté web). Un ingrédient dont l'id est déjà un
//! **ancêtre** de la chaîne en cours est résolu avec `has_recipe: false` : sa ligne s'affiche
//! normalement, seule sa propre recette n'est pas développée davantage.

use std::collections::HashSet;

use crate::catalog::{CatalogIndex, WakfuRarity};

/// Un ingrédient résolu — miroir de `CatalogResolvedIngredient` côté web.
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeIngredient {
    /// Nom fr, tel qu'il s'écrit — celui que le log emploie, donc celui qu'il faut stocker.
    pub name: String,
    /// **L'id Ankama EXACT**, jamais une résolution par nom : un ingrédient peut partager son nom
    /// avec une autre variante de rareté du même objet, et retomber sur la mauvaise rendrait le
    /// suivi faux.
    pub id: i64,
    pub rarity: WakfuRarity,
    /// Quantité nécessaire pour **une** unité de l'objet parent.
    pub quantity: i64,
    /// A-t-il lui-même une recette développable ? `false` sur un cycle, voir la doc de module.
    pub has_recipe: bool,
    /// Sa propre recette, résolue — vide si `has_recipe` est faux.
    pub children: Vec<RecipeIngredient>,
}

/// Résout les ingrédients de l'objet `id`, récursivement, depuis le référentiel.
///
/// Un ingrédient que le catalogue ne sait pas nommer (id absent du référentiel) est
/// **silencieusement omis**, comme côté web : sans nom, il n'y a rien à suivre.
pub fn resolve_recipe(id: i64, catalog: &CatalogIndex) -> Vec<RecipeIngredient> {
    let mut ancetres = HashSet::from([id]);
    resolve_level(id, catalog, &mut ancetres)
}

fn resolve_level(
    id: i64,
    catalog: &CatalogIndex,
    ancetres: &mut HashSet<i64>,
) -> Vec<RecipeIngredient> {
    let mut resolus = Vec::new();
    for &(ingredient_id, quantity) in catalog.recipe(id) {
        // Sans nom, rien à suivre — la ligne est omise, jamais rendue avec un libellé vide.
        let Some(name) = catalog.item_name(ingredient_id) else {
            continue;
        };
        let cycle = ancetres.contains(&ingredient_id);
        let has_recipe = catalog.find_item_has_recipe(name, Some(ingredient_id)) && !cycle;
        let children = if has_recipe {
            ancetres.insert(ingredient_id);
            let enfants = resolve_level(ingredient_id, catalog, ancetres);
            ancetres.remove(&ingredient_id);
            enfants
        } else {
            Vec::new()
        };

        resolus.push(RecipeIngredient {
            name: name.to_string(),
            id: ingredient_id,
            rarity: catalog.find_item_rarity(name, Some(ingredient_id)),
            quantity,
            has_recipe,
            children,
        });
    }
    resolus
}

/// **Aplatit un arbre d'ingrédients en ce qu'il faut réellement suivre**, quantités multipliées.
///
/// `nested` dit quelles lignes l'utilisateur a dépliées : une ligne dépliée est **remplacée par ses
/// propres ingrédients** plutôt que suivie elle-même (« suivre les ingrédients de cet objet plutôt
/// que l'objet lui-même », `tracker.recipeNestTooltip`). Le chemin d'une ligne est sa position dans
/// l'arbre, `"0"`, `"2.1"` — la même convention que le web.
///
/// Les quantités se multiplient en descendant : deux Coiffes qui demandent deux Cuirs de cinq Peaux
/// chacun font vingt Peaux.
pub fn flatten_for_tracking(
    ingredients: &[RecipeIngredient],
    multiplier: i64,
    nested: &HashSet<String>,
) -> Vec<(String, i64, i64)> {
    let mut sortie = Vec::new();
    collect(ingredients, multiplier, nested, "", &mut sortie);
    sortie
}

fn collect(
    ingredients: &[RecipeIngredient],
    multiplier: i64,
    nested: &HashSet<String>,
    prefixe: &str,
    sortie: &mut Vec<(String, i64, i64)>,
) {
    for (rang, ingredient) in ingredients.iter().enumerate() {
        let chemin = if prefixe.is_empty() {
            rang.to_string()
        } else {
            format!("{prefixe}.{rang}")
        };
        let quantite = ingredient.quantity * multiplier;
        if ingredient.has_recipe && nested.contains(&chemin) {
            // Dépliée : on suit ses ingrédients, pas elle.
            collect(&ingredient.children, quantite, nested, &chemin, sortie);
        } else {
            sortie.push((ingredient.name.clone(), ingredient.id, quantite));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un référentiel minimal : `10` (avec recette) → `20` (avec recette) → `30`.
    fn catalogue(recipes: serde_json::Value) -> CatalogIndex {
        CatalogIndex::from_json(&serde_json::json!({
            "items": [
                [10, "Coiffe", 1, 1, 1, 0],
                [20, "Cuir", 2, 0, 1, 1],
                [30, "Peau", 3, 0, 0, 1],
            ],
            "recipes": recipes,
        }))
    }

    #[test]
    fn la_recette_se_resout_sur_deux_niveaux() {
        let catalog = catalogue(serde_json::json!([[10, [[20, 2]]], [20, [[30, 5]]]]));
        let arbre = resolve_recipe(10, &catalog);
        assert_eq!(arbre.len(), 1);
        assert_eq!(arbre[0].name, "Cuir");
        assert_eq!(arbre[0].quantity, 2);
        assert!(arbre[0].has_recipe);
        assert_eq!(arbre[0].children.len(), 1);
        assert_eq!(arbre[0].children[0].name, "Peau");
        assert_eq!(arbre[0].children[0].quantity, 5);
        assert!(!arbre[0].children[0].has_recipe);
    }

    #[test]
    fn un_cycle_ne_boucle_pas_a_l_infini() {
        // **Cas réel du référentiel Ankama** : un objet ingrédient de lui-même. Sans la garde, la
        // résolution ne rendrait jamais la main.
        let catalog = catalogue(serde_json::json!([[10, [[20, 1]]], [20, [[20, 1]]]]));
        let arbre = resolve_recipe(10, &catalog);
        // La ligne s'affiche, mais sa propre recette n'est pas développée.
        assert_eq!(arbre[0].children.len(), 1);
        assert!(!arbre[0].children[0].has_recipe);
        assert!(arbre[0].children[0].children.is_empty());
    }

    #[test]
    fn un_ingredient_sans_nom_est_omis() {
        let catalog = catalogue(serde_json::json!([[10, [[99, 1], [30, 3]]]]));
        let arbre = resolve_recipe(10, &catalog);
        assert_eq!(arbre.len(), 1);
        assert_eq!(arbre[0].name, "Peau");
    }

    #[test]
    fn un_objet_sans_recette_ne_rend_rien() {
        let catalog = catalogue(serde_json::json!([]));
        assert!(resolve_recipe(30, &catalog).is_empty());
    }

    #[test]
    fn une_ligne_depliee_est_remplacee_par_ses_ingredients() {
        let cuir = RecipeIngredient {
            name: "Cuir".into(),
            id: 20,
            rarity: WakfuRarity::Common,
            quantity: 2,
            has_recipe: true,
            children: vec![RecipeIngredient {
                name: "Peau".into(),
                id: 30,
                rarity: WakfuRarity::Common,
                quantity: 5,
                has_recipe: false,
                children: Vec::new(),
            }],
        };
        // Repliée : on suit le Cuir, deux par Coiffe, deux Coiffes.
        let replie = flatten_for_tracking(std::slice::from_ref(&cuir), 2, &HashSet::new());
        assert_eq!(replie, vec![("Cuir".to_string(), 20, 4)]);
        // Dépliée : on suit les Peaux — 5 × 2 × 2 = 20, et le Cuir n'est plus suivi.
        let deplie = flatten_for_tracking(&[cuir], 2, &HashSet::from(["0".to_string()]));
        assert_eq!(deplie, vec![("Peau".to_string(), 30, 20)]);
    }
}
