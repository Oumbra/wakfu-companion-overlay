//! Génère `assets/referentiel.json`, le référentiel que l'overlay embarque (`overlay_engine::
//! referentiel`) — objets, monstres, recettes, donjons, familles de monstres, **en français
//! seulement**.
//!
//! Reprend à l'identique les règles de `server/import/import-catalog.ts` et
//! `server/catalog/compact-index.ts` du dépôt web, qui alimentaient jusqu'ici `GET /api/v1/
//! catalog/` : exclusion de la rareté « old », catégorie large déduite de la sous-catégorie,
//! dédoublonnage des objets identiques (même nom fr, rareté et gfxId), tri par id. Le résultat doit
//! donc résoudre chaque objet exactement comme le site.
//!
//! Format (une entrée par ligne, pour que la mise à jour du référentiel reste lisible en diff) :
//!
//! ```text
//! items    : [id, fr, gfxId, raritySortOrder, hasRecipe(0|1), categorySortOrder]
//! monsters : [id, fr, gfxId, family(-1 si aucune), isBoss(0|1), isArchi(0|1), isDominant(0|1)]
//! recipes  : [itemId, [[ingredientId, quantité], …]]
//! dungeons : { id, fr, level, bracket, type, bossMonsterId[], monsterFamilyId[],
//!              hasPreBossArchi, picture }
//! monsterFamilies : { id, fr, picture | null }
//! ```

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

use serde_json::{json, Value};

use crate::lire_json;

/// Miroir de `RARITY_SORT_ORDER` (`compact-index.ts`).
fn rarete_ordre(rarete: &str) -> i64 {
    match rarete {
        "old" => 0,
        "common" => 1,
        "rare" => 2,
        "mythical" => 3,
        "legendary" => 4,
        "memory" => 5,
        "epic" => 6,
        "relic" => 7,
        _ => 1,
    }
}

/// Miroir de `normalizeRarity` (`import-catalog.ts`).
fn normaliser_rarete(brute: &str) -> &'static str {
    match brute {
        "old" => "old",
        "common" => "common",
        "rare" => "rare",
        "mythical" => "mythical",
        "legendary" => "legendary",
        "memory" => "memory",
        "epic" => "epic",
        "relic" => "relic",
        "Qualité commune" => "old",
        autre => {
            eprintln!("rareté « {autre} » invalide, repli sur « common »");
            "common"
        }
    }
}

/// Miroir d'`ITEM_SUBCATEGORY_CATALOG` + `resolveBroadCategory` (`import-catalog.ts`), encodé
/// directement en `CATEGORY_SORT_ORDER` (`compact-index.ts`) : equipment 0, resources 1,
/// harvests 3, havenBag 4, cosmetics 5, craft 6, misc 7 (sublimations 2 n'est attribuée par aucune
/// sous-catégorie, comme côté web).
fn categorie_ordre(sous_categorie: Option<i64>, a_recette: bool) -> i64 {
    match sous_categorie {
        Some(1..=19) => 0,
        Some(20..=24) => 4,
        Some(25..=31) => 3,
        Some(32..=38) => 1,
        Some(39..=45) => 5,
        _ if a_recette => 6,
        _ => 7,
    }
}

/// Un objet conservé dans le référentiel — partagé avec `images.rs`, qui a besoin des mêmes
/// objets que le catalogue (pas des doublons écartés).
pub struct Objet {
    pub id: i64,
    pub fr: String,
    pub gfx_id: i64,
    rarete: &'static str,
    a_recette: bool,
    categorie: i64,
}

/// Les objets du référentiel, filtrés et dédoublonnés comme `import-catalog.ts`, triés par id.
pub fn objets(repository: &Path) -> Result<Vec<Objet>, String> {
    let bruts = lire_json(&repository.join("items.json"))?;
    let recettes = lire_json(&repository.join("recipes.json"))?;

    let mut references = HashSet::new();
    for recette in recettes.as_array().into_iter().flatten() {
        if let Some(id) = recette["itemId"].as_i64() {
            references.insert(id);
        }
        for ingredient in recette["recipe"].as_array().into_iter().flatten() {
            if let Some(id) = ingredient["itemId"].as_i64() {
                references.insert(id);
            }
        }
    }

    let mut avec_doublons = Vec::new();
    for brut in bruts.as_array().into_iter().flatten() {
        let rarete = normaliser_rarete(brut["rarity"].as_str().unwrap_or(""));
        if rarete == "old" {
            continue;
        }
        // Les objets sans id Ankama sont exclus de l'index compact (`buildCompactIndex`).
        let Some(id) = brut["id"].as_i64() else {
            continue;
        };
        let gfx_id = match &brut["gfxId"] {
            Value::String(texte) => texte.parse().unwrap_or(0),
            autre => autre.as_i64().unwrap_or(0),
        };
        let a_recette = brut["hasRecipe"].as_bool() == Some(true);
        avec_doublons.push(Objet {
            id,
            fr: brut["fr"].as_str().unwrap_or("").to_string(),
            gfx_id,
            rarete,
            a_recette,
            categorie: categorie_ordre(brut["category"].as_i64(), a_recette),
        });
    }

    // `dedupeItemRows` : même fr/rareté/gfxId ⇒ un seul gardé, celui référencé par une recette
    // s'il y en a un, sinon l'id le plus bas. L'ordre d'origine des groupes est conservé.
    let mut groupes: Vec<Vec<Objet>> = Vec::new();
    let mut position: HashMap<(String, &'static str, i64), usize> = HashMap::new();
    for objet in avec_doublons {
        let cle = (objet.fr.clone(), objet.rarete, objet.gfx_id);
        match position.get(&cle) {
            Some(&rang) => groupes[rang].push(objet),
            None => {
                position.insert(cle, groupes.len());
                groupes.push(vec![objet]);
            }
        }
    }
    let mut ecartes = 0;
    let mut objets: Vec<Objet> = Vec::new();
    for mut groupe in groupes {
        ecartes += groupe.len() - 1;
        groupe.sort_by_key(|objet| objet.id);
        let gagnant = groupe
            .iter()
            .position(|objet| references.contains(&objet.id))
            .unwrap_or(0);
        objets.push(groupe.swap_remove(gagnant));
    }
    if ecartes > 0 {
        println!("{ecartes} doublons d'objets écartés (même fr/rareté/gfxId)");
    }
    // Tri stable par id, comme `.sort((a, b) => a.ankamaId - b.ankamaId)`.
    objets.sort_by_key(|objet| objet.id);
    Ok(objets)
}

pub fn run(repository: &Path, sortie: &Path) -> Result<(), String> {
    let objets = objets(repository)?;
    let monstres = lire_json(&repository.join("monsters.json"))?;
    let recettes = lire_json(&repository.join("recipes.json"))?;
    let donjons = lire_json(&repository.join("dungeons.json"))?;
    let familles = lire_json(&repository.join("monster-families.json"))?;

    let mut texte = String::from("{\n\"items\": [\n");
    let lignes: Vec<String> = objets
        .iter()
        .map(|objet| {
            json!([
                objet.id,
                objet.fr,
                objet.gfx_id,
                rarete_ordre(objet.rarete),
                i64::from(objet.a_recette),
                objet.categorie
            ])
            .to_string()
        })
        .collect();
    texte.push_str(&lignes.join(",\n"));

    // Monstres : tous, triés par id (`buildCompactIndex`).
    let mut monstres: Vec<&Value> = monstres.as_array().into_iter().flatten().collect();
    monstres.sort_by_key(|monstre| monstre["id"].as_i64().unwrap_or(0));
    let lignes: Vec<String> = monstres
        .iter()
        .map(|monstre| {
            json!([
                monstre["id"],
                monstre["fr"],
                monstre["gfxId"]
                    .as_str()
                    .map_or_else(|| monstre["gfxId"].to_string(), str::to_string),
                monstre["family"].as_i64().unwrap_or(-1),
                i64::from(monstre["isBoss"].as_bool() == Some(true)),
                i64::from(monstre["isArchi"].as_bool() == Some(true)),
                i64::from(monstre["isDominant"].as_bool() == Some(true))
            ])
            .to_string()
        })
        .collect();
    texte.push_str("\n],\n\"monsters\": [\n");
    texte.push_str(&lignes.join(",\n"));

    // Recettes : celles des objets conservés qui portent le drapeau `hasRecipe` — c'est la
    // condition de `GET /api/v1/items/{id}` côté web. Plusieurs entrées pour un même objet se
    // concatènent, comme les lignes de `item_recipes`.
    let avec_recette: HashSet<i64> = objets
        .iter()
        .filter(|objet| objet.a_recette)
        .map(|objet| objet.id)
        .collect();
    let mut par_objet: Vec<(i64, Vec<Value>)> = Vec::new();
    let mut rang_objet: HashMap<i64, usize> = HashMap::new();
    for recette in recettes.as_array().into_iter().flatten() {
        let Some(id) = recette["itemId"].as_i64() else {
            continue;
        };
        if !avec_recette.contains(&id) {
            continue;
        }
        let ingredients: Vec<Value> = recette["recipe"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|ingredient| {
                Some(json!([
                    ingredient["itemId"].as_i64()?,
                    ingredient["quantity"].as_i64().unwrap_or(1)
                ]))
            })
            .collect();
        let rang = *rang_objet.entry(id).or_insert_with(|| {
            par_objet.push((id, Vec::new()));
            par_objet.len() - 1
        });
        par_objet[rang].1.extend(ingredients);
    }
    par_objet.sort_by_key(|(id, _)| *id);
    let lignes: Vec<String> = par_objet
        .iter()
        .map(|(id, ingredients)| json!([id, ingredients]).to_string())
        .collect();
    texte.push_str("\n],\n\"recipes\": [\n");
    texte.push_str(&lignes.join(",\n"));

    let mut donjons: Vec<&Value> = donjons.as_array().into_iter().flatten().collect();
    donjons.sort_by_key(|donjon| donjon["id"].as_i64().unwrap_or(0));
    let lignes: Vec<String> = donjons
        .iter()
        .map(|donjon| {
            json!({
                "id": donjon["id"],
                "fr": donjon["fr"],
                "level": donjon["level"],
                "bracket": donjon["bracket"],
                "type": donjon["type"],
                "bossMonsterId": liste_ids(&donjon["bossMonsterId"]),
                "monsterFamilyId": liste_ids(&donjon["monsterFamilyId"]),
                "hasPreBossArchi": donjon["has_pre_boss_archi"].as_bool() == Some(true),
                "picture": format!("bossIllustrations/{}.png", donjon["id"]),
            })
            .to_string()
        })
        .collect();
    texte.push_str("\n],\n\"dungeons\": [\n");
    texte.push_str(&lignes.join(",\n"));

    let mut familles: Vec<&Value> = familles.as_array().into_iter().flatten().collect();
    familles.sort_by_key(|famille| famille["id"].as_i64().unwrap_or(0));
    let lignes: Vec<String> = familles
        .iter()
        .map(|famille| {
            json!({
                "id": famille["id"],
                "fr": famille["fr"],
                "picture": famille["picture"].as_str().and_then(chemin_wakassets),
            })
            .to_string()
        })
        .collect();
    texte.push_str("\n],\n\"monsterFamilies\": [\n");
    texte.push_str(&lignes.join(",\n"));
    texte.push_str("\n]\n}\n");

    // Le fichier doit rester du JSON valide : relu avant d'être écrit.
    serde_json::from_str::<Value>(&texte)
        .map_err(|err| format!("JSON produit invalide : {err}"))?;
    std::fs::write(sortie, &texte)
        .map_err(|err| format!("écriture de {} : {err}", sortie.display()))?;

    let mut bilan = String::new();
    let _ = write!(
        bilan,
        "{} : {} objets, {} monstres, {} recettes, {} donjons, {} familles, {} octets",
        sortie.display(),
        objets.len(),
        monstres.len(),
        par_objet.len(),
        donjons.len(),
        familles.len(),
        texte.len()
    );
    println!("{bilan}");
    Ok(())
}

/// `null | entier | tableau` (curation manuelle) ⇒ toujours un tableau, comme `toIdArray`.
fn liste_ids(valeur: &Value) -> Value {
    match valeur {
        Value::Array(_) => valeur.clone(),
        Value::Number(_) => json!([valeur]),
        _ => json!([]),
    }
}

/// `https://vertylo.github.io/wakassets/monstersfamily/68.png` ⇒ `monstersfamily/68.png` : le
/// référentiel ne garde que le chemin, l'overlay choisit lui-même où le télécharger.
fn chemin_wakassets(url: &str) -> Option<String> {
    url.split("/wakassets/").nth(1).map(str::to_string)
}
