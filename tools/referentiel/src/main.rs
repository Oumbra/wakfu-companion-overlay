//! Outil du mainteneur — voir `docs/plan-historique-local.md` §4 (référentiel embarqué) et §5
//! (images du CDN de l'overlay).
//!
//! **Jamais livré, jamais lancé par la CI ni par la release** : on le lance à la main, sur le poste
//! du mainteneur, à chaque mise à jour du référentiel ou du jeu, et on commite son résultat. En
//! particulier, l'overlay lui-même ne lit jamais aucun fichier du client de jeu (décision du
//! mainteneur du 2026-11-27) : c'est le rôle de cet outil, en amont, de préparer ce que le CDN
//! sert ensuite.
//!
//! ```text
//! cargo run --release --manifest-path tools/referentiel/Cargo.toml -- catalogue \
//!     --repository ../wakfu-companion/repository --sortie assets/referentiel.json
//! cargo run --release --manifest-path tools/referentiel/Cargo.toml -- images \
//!     --repository ../wakfu-companion/repository --wakassets ../wakassets \
//!     --gui-jar "D:/Games/Ankama/Wakfu/contents/gui_jar/gui.jar" --sortie cdn
//! ```

mod catalogue;
mod images;

use std::collections::HashMap;
use std::path::PathBuf;

const USAGE: &str = "usage : referentiel catalogue --repository <dossier> --sortie <fichier>\n\
                     \x20      referentiel images --repository <dossier> --wakassets <dossier> \
                     --gui-jar <fichier> --sortie <dossier>";

fn main() {
    let mut args = std::env::args().skip(1);
    let commande = args.next();
    let options = lire_options(args);
    let resultat = match commande.as_deref() {
        Some("catalogue") => catalogue::run(
            &chemin(&options, "--repository"),
            &chemin(&options, "--sortie"),
        ),
        Some("images") => images::run(
            &chemin(&options, "--repository"),
            &chemin(&options, "--wakassets"),
            &chemin(&options, "--gui-jar"),
            &chemin(&options, "--sortie"),
        ),
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    };
    if let Err(err) = resultat {
        eprintln!("erreur : {err}");
        std::process::exit(1);
    }
}

fn lire_options(args: impl Iterator<Item = String>) -> HashMap<String, String> {
    let args: Vec<String> = args.collect();
    args.chunks(2)
        .filter_map(|paire| match paire {
            [cle, valeur] => Some((cle.clone(), valeur.clone())),
            _ => None,
        })
        .collect()
}

fn chemin(options: &HashMap<String, String>, cle: &str) -> PathBuf {
    match options.get(cle) {
        Some(valeur) => PathBuf::from(valeur),
        None => {
            eprintln!("option manquante : {cle}\n{USAGE}");
            std::process::exit(2);
        }
    }
}

/// Lit un fichier JSON du référentiel.
pub fn lire_json(chemin: &std::path::Path) -> Result<serde_json::Value, String> {
    let texte = std::fs::read_to_string(chemin)
        .map_err(|err| format!("lecture de {} : {err}", chemin.display()))?;
    serde_json::from_str(&texte)
        .map_err(|err| format!("JSON invalide dans {} : {err}", chemin.display()))
}
