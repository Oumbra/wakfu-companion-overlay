//! Prépare le dossier `cdn/` : les images dont l'overlay a besoin et que wakassets n'a pas
//! (`docs/plan-historique-local.md` §5.3).
//!
//! Pour chaque objet, monstre, donjon, famille et sort du référentiel, l'image est cherchée dans
//! le clone local de wakassets ; absente, elle est extraite de `gui.jar` (TGA), convertie en PNG
//! et écrite sous le même chemin que wakassets l'aurait servie (`items/{gfxId}.png`…). L'overlay
//! essaie wakassets puis ce CDN, sans rien savoir de plus.
//!
//! Les sous-dossiers gérés sont vidés avant d'être réécrits : `cdn/` reflète exactement ce qui
//! manque à wakassets à l'instant du lancement.

use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::catalogue;
use crate::lire_json;

/// Une image à fournir : où wakassets la servirait, où la trouver dans le jeu.
struct Besoin {
    /// Chemin relatif au CDN, identique à celui de wakassets (`items/123.png`).
    chemin: String,
    /// Autres chemins wakassets qui suffisent (un monstre peut vivre dans deux dossiers).
    alternatives: Vec<String>,
    /// Entrée de `gui.jar` qui contient l'image.
    entree_jar: String,
    /// Libellé lisible pour le bilan.
    libelle: String,
}

/// Dossiers du CDN gérés par cet outil — vidés puis réécrits à chaque lancement.
const DOSSIERS: [&str; 6] = [
    "items",
    "monsterIllustrations",
    "bossIllustrations",
    "monstersfamily",
    "spells",
    "timePointBonus",
];

pub fn run(
    repository: &Path,
    wakassets: &Path,
    gui_jar: &Path,
    sortie: &Path,
) -> Result<(), String> {
    let besoins = besoins(repository)?;

    let fichier =
        File::open(gui_jar).map_err(|err| format!("ouverture de {} : {err}", gui_jar.display()))?;
    let mut jar =
        zip::ZipArchive::new(fichier).map_err(|err| format!("lecture de l'archive : {err}"))?;
    let entrees: HashSet<String> = jar.file_names().map(str::to_string).collect();

    for dossier in DOSSIERS {
        let chemin = sortie.join(dossier);
        if chemin.exists() {
            std::fs::remove_dir_all(&chemin)
                .map_err(|err| format!("nettoyage de {} : {err}", chemin.display()))?;
        }
    }

    let mut ecrits: BTreeMap<String, usize> = BTreeMap::new();
    let mut introuvables = Vec::new();
    let mut octets = 0usize;
    let mut deja_faits = HashSet::new();
    for besoin in &besoins {
        let present = std::iter::once(&besoin.chemin)
            .chain(&besoin.alternatives)
            .any(|chemin| wakassets.join(chemin).is_file());
        if present || !deja_faits.insert(besoin.chemin.clone()) {
            continue;
        }
        if !entrees.contains(&besoin.entree_jar) {
            introuvables.push(format!("{} ({})", besoin.libelle, besoin.entree_jar));
            continue;
        }
        let mut brut = Vec::new();
        jar.by_name(&besoin.entree_jar)
            .map_err(|err| format!("{} : {err}", besoin.entree_jar))?
            .read_to_end(&mut brut)
            .map_err(|err| format!("{} : {err}", besoin.entree_jar))?;
        let image = image::load_from_memory_with_format(&brut, image::ImageFormat::Tga)
            .map_err(|err| format!("décodage de {} : {err}", besoin.entree_jar))?;
        let destination = sortie.join(&besoin.chemin);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("{} : {err}", parent.display()))?;
        }
        image
            .to_rgba8()
            .save_with_format(&destination, image::ImageFormat::Png)
            .map_err(|err| format!("écriture de {} : {err}", destination.display()))?;
        octets += std::fs::metadata(&destination)
            .map(|m| m.len() as usize)
            .unwrap_or(0);
        let dossier = besoin.chemin.split('/').next().unwrap_or("").to_string();
        *ecrits.entry(dossier).or_default() += 1;
    }

    let total: usize = ecrits.values().sum();
    println!(
        "{total} images écrites dans {} ({} Ko) :",
        sortie.display(),
        octets / 1024
    );
    for (dossier, nombre) in &ecrits {
        println!("  {dossier:<22} {nombre}");
    }
    if !introuvables.is_empty() {
        println!(
            "{} images absentes de wakassets ET du jeu (icône générique dans l'overlay) :",
            introuvables.len()
        );
        for libelle in &introuvables {
            println!("  {libelle}");
        }
    }
    Ok(())
}

fn besoins(repository: &Path) -> Result<Vec<Besoin>, String> {
    let mut besoins = Vec::new();

    for objet in catalogue::objets(repository)? {
        besoins.push(Besoin {
            chemin: format!("items/{}.png", objet.gfx_id),
            alternatives: Vec::new(),
            entree_jar: format!("icons/items/64/{}.tga", objet.gfx_id),
            libelle: format!("objet {} « {} »", objet.id, objet.fr),
        });
    }

    // Un monstre est servi par `monsters/` ou `monsterIllustrations/` côté wakassets ; le jeu n'a
    // que le second.
    for monstre in lire_json(&repository.join("monsters.json"))?
        .as_array()
        .into_iter()
        .flatten()
    {
        let gfx = monstre["gfxId"]
            .as_str()
            .map_or_else(|| monstre["gfxId"].to_string(), str::to_string);
        besoins.push(Besoin {
            chemin: format!("monsterIllustrations/{gfx}.png"),
            alternatives: vec![format!("monsters/{gfx}.png")],
            entree_jar: format!("icons/monsterIllustrations/{gfx}.tga"),
            libelle: format!(
                "monstre {} « {} »",
                monstre["id"],
                monstre["fr"].as_str().unwrap_or("")
            ),
        });
    }

    for donjon in lire_json(&repository.join("dungeons.json"))?
        .as_array()
        .into_iter()
        .flatten()
    {
        let id = &donjon["id"];
        besoins.push(Besoin {
            chemin: format!("bossIllustrations/{id}.png"),
            alternatives: Vec::new(),
            entree_jar: format!("icons/bossIllustrations/{id}.tga"),
            libelle: format!("donjon {id} « {} »", donjon["fr"].as_str().unwrap_or("")),
        });
    }

    for famille in lire_json(&repository.join("monster-families.json"))?
        .as_array()
        .into_iter()
        .flatten()
    {
        let Some(chemin) = famille["picture"]
            .as_str()
            .and_then(|url| url.split("/wakassets/").nth(1))
        else {
            continue;
        };
        let numero = chemin
            .trim_start_matches("monstersfamily/")
            .trim_end_matches(".png");
        besoins.push(Besoin {
            chemin: chemin.to_string(),
            alternatives: Vec::new(),
            entree_jar: format!("icons/monstersFamily/{numero}.tga"),
            libelle: format!(
                "famille {} « {} »",
                famille["id"],
                famille["fr"].as_str().unwrap_or("")
            ),
        });
    }

    // Sorts embarqués par l'overlay (`assets/spells.json`, `assets/monster-spells.json`) — relus
    // depuis le dépôt courant : l'outil se lance à sa racine.
    for referentiel in ["assets/spells.json", "assets/monster-spells.json"] {
        let Ok(sorts) = lire_json(Path::new(referentiel)) else {
            eprintln!(
                "{referentiel} introuvable — sorts ignorés (lancer l'outil à la racine du dépôt)"
            );
            continue;
        };
        for proprietaire in sorts.as_array().into_iter().flatten() {
            for sort in proprietaire["spells"].as_array().into_iter().flatten() {
                let Some(chemin) = sort["picture"].as_str() else {
                    continue;
                };
                let Some((dossier, fichier)) = chemin.split_once('/') else {
                    continue;
                };
                besoins.push(Besoin {
                    chemin: chemin.to_string(),
                    alternatives: Vec::new(),
                    // Le jeu range les sorts par taille (`icons/spells/64/`), pas les bonus PA/PM.
                    entree_jar: match dossier {
                        "spells" => {
                            format!("icons/spells/64/{}.tga", fichier.trim_end_matches(".png"))
                        }
                        _ => format!("icons/{dossier}/{}.tga", fichier.trim_end_matches(".png")),
                    },
                    libelle: format!(
                        "sort {} « {} »",
                        sort["id"],
                        sort["fr"].as_str().unwrap_or("")
                    ),
                });
            }
        }
    }

    Ok(besoins)
}
