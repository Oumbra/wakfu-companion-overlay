//! **Les ingrédients d'une recette**, résolus récursivement — ce que la fenêtre « Objets de la
//! recette » de l'onglet « Suivi » affiche, et ce qu'elle ajoute au suivi quand on la valide.
//!
//! Miroir de `CatalogService.resolveRecipeIngredients` (dépôt web, `core/api/catalog.service.ts`),
//! avec les deux mêmes contraintes :
//!
//! 1. **Un aller-retour réseau par NIVEAU de recette.** L'index compact du catalogue ne porte que
//!    le drapeau `has_recipe` ([`CatalogIndex::find_item_has_recipe`]) — jamais les ingrédients,
//!    qui alourdiraient un index chargé au démarrage pour un besoin qui ne concerne qu'un
//!    dialogue. Le détail vient de `GET /api/v1/items/{id}`.
//! 2. **Seuls les ingrédients qui ONT une recette déclenchent un appel de plus.** Le drapeau se lit
//!    dans l'index, en mémoire : l'arbre n'est jamais exploré à l'aveugle.
//!
//! ## Le réseau n'entre pas ici
//!
//! [`resolve_recipe`] reçoit un **résolveur** — une closure qui rend le JSON d'un objet — plutôt
//! que d'appeler `overlay_sync` elle-même. Deux raisons : `overlay-engine` ne dépend pas du
//! transport (§3 du plan), et une résolution récursive avec garde anti-cycle se teste beaucoup
//! mieux sur une table en mémoire que sur un serveur.
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

/// Résout les ingrédients de l'objet `id`, récursivement.
///
/// `fetch` rend le JSON de `GET /api/v1/items/{id}` — `None` quand l'objet est injoignable, auquel
/// cas ses ingrédients sont simplement absents plutôt qu'une erreur : une recette partiellement
/// résolue reste plus utile qu'un dialogue vide.
///
/// Un ingrédient dont le nom n'a pas pu être résolu côté serveur (`name: null`, id absent de la
/// table) est **silencieusement omis**, comme côté web : sans nom, il n'y a rien à suivre.
pub fn resolve_recipe(
    id: i64,
    catalog: &CatalogIndex,
    fetch: &mut dyn FnMut(i64) -> Option<serde_json::Value>,
) -> Vec<RecipeIngredient> {
    let mut ancetres = HashSet::from([id]);
    resolve_level(id, catalog, fetch, &mut ancetres)
}

fn resolve_level(
    id: i64,
    catalog: &CatalogIndex,
    fetch: &mut dyn FnMut(i64) -> Option<serde_json::Value>,
    ancetres: &mut HashSet<i64>,
) -> Vec<RecipeIngredient> {
    let Some(detail) = fetch(id) else {
        return Vec::new();
    };
    let Some(lignes) = detail.get("recipe").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    let mut resolus = Vec::new();
    for ligne in lignes {
        // Sans nom, rien à suivre — la ligne est omise, jamais rendue avec un libellé vide.
        let Some(name) = ligne.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(ingredient_id) = ligne.get("itemId").and_then(|v| v.as_i64()) else {
            continue;
        };
        let quantity = ligne.get("quantity").and_then(|v| v.as_i64()).unwrap_or(1);

        let cycle = ancetres.contains(&ingredient_id);
        let has_recipe = catalog.find_item_has_recipe(name, Some(ingredient_id)) && !cycle;
        let children = if has_recipe {
            ancetres.insert(ingredient_id);
            let enfants = resolve_level(ingredient_id, catalog, fetch, ancetres);
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
    fn catalogue() -> CatalogIndex {
        CatalogIndex::from_compact_json(&serde_json::json!({
            "items": [
                [10, "Coiffe", "Hat", "Gorro", "Chapeu", 1, 1, 1, 0],
                [20, "Cuir", "Leather", "Cuero", "Couro", 2, 0, 1, 1],
                [30, "Peau", "Hide", "Piel", "Pele", 3, 0, 0, 1],
            ],
            "monsters": []
        }))
    }

    fn detail(id: i64, recipe: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "id": id, "recipe": recipe })
    }

    #[test]
    fn la_recette_se_resout_sur_deux_niveaux() {
        let catalog = catalogue();
        let mut fetch = |id: i64| match id {
            10 => Some(detail(
                10,
                serde_json::json!([{ "itemId": 20, "name": "Cuir", "quantity": 2 }]),
            )),
            20 => Some(detail(
                20,
                serde_json::json!([{ "itemId": 30, "name": "Peau", "quantity": 5 }]),
            )),
            _ => None,
        };
        let arbre = resolve_recipe(10, &catalog, &mut fetch);
        assert_eq!(arbre.len(), 1);
        assert_eq!(arbre[0].name, "Cuir");
        assert!(arbre[0].has_recipe);
        assert_eq!(arbre[0].children.len(), 1);
        assert_eq!(arbre[0].children[0].name, "Peau");
        // La feuille n'a pas de recette : aucun appel de plus n'a été tenté pour elle.
        assert!(!arbre[0].children[0].has_recipe);
    }

    #[test]
    fn un_cycle_ne_boucle_pas_a_l_infini() {
        // **Cas réel du référentiel Ankama** : un objet ingrédient de lui-même. Sans la garde, la
        // résolution ne rendrait jamais la main.
        let catalog = catalogue();
        let mut fetch = |id: i64| match id {
            10 => Some(detail(
                10,
                serde_json::json!([{ "itemId": 20, "name": "Cuir", "quantity": 1 }]),
            )),
            20 => Some(detail(
                20,
                serde_json::json!([{ "itemId": 20, "name": "Cuir", "quantity": 1 }]),
            )),
            _ => None,
        };
        let arbre = resolve_recipe(10, &catalog, &mut fetch);
        // La ligne s'affiche, mais sa propre recette n'est pas développée.
        assert_eq!(arbre[0].children.len(), 1);
        assert!(!arbre[0].children[0].has_recipe);
        assert!(arbre[0].children[0].children.is_empty());
    }

    #[test]
    fn un_ingredient_sans_nom_est_omis() {
        let catalog = catalogue();
        let mut fetch = |_: i64| {
            Some(detail(
                10,
                serde_json::json!([
                    { "itemId": 99, "name": null, "quantity": 1 },
                    { "itemId": 30, "name": "Peau", "quantity": 3 },
                ]),
            ))
        };
        let arbre = resolve_recipe(10, &catalog, &mut fetch);
        assert_eq!(arbre.len(), 1);
        assert_eq!(arbre[0].name, "Peau");
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
