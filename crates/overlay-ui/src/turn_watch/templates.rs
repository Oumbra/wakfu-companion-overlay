//! Persistance des gabarits de nom appris — un PNG 8 bits par personnage, dans le dossier de
//! données de l'overlay (`turn-templates/`, même racine que `logs/` et `catalog_cache` depuis le
//! 2026-09-19, voir `overlay_engine::app_dirs`).
//!
//! Un gabarit vaut pour un client à une échelle d'interface donnée : il est indexé par le nom du
//! personnage tel que le titre de fenêtre le donne. Changer l'échelle d'interface du jeu rendra
//! les gabarits obsolètes — ils cesseront simplement de reconnaître, et le prochain combat en
//! réapprendra de nouveaux. Effacer le dossier fait la même chose à la main.
//!
//! Best-effort comme `config` : un échec d'écriture est journalisé, jamais fatal — l'overlay
//! réapprendra au prochain lancement.

use std::collections::HashMap;
use std::path::PathBuf;

use super::vision::Glyph;

/// Dossier de données de l'overlay — demandé à `config` plutôt que reconstruit ici : une seule
/// racine pour tout le dépôt (`overlay_engine::app_dirs`, constat C13 de `docs/analyse-rgpd.md`).
pub fn data_dir() -> Option<PathBuf> {
    crate::config::project_dirs().map(|d| d.data_dir().to_path_buf())
}

/// Dossier des gabarits appris — public pour que `crate::local_data` puisse l'effacer (un PNG du
/// nom du personnage rendu à l'écran et le nom en clair, constat C8).
pub fn dir() -> Option<PathBuf> {
    data_dir().map(|d| d.join("turn-templates"))
}

/// Nom de fichier sûr pour un nom de personnage (les noms Wakfu peuvent porter des espaces et des
/// signes ; on garde lettres, chiffres, tiret et souligné).
fn file_stem(character: &str) -> String {
    character
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Tous les gabarits du disque — un fichier illisible est ignoré et journalisé.
pub fn load_all() -> HashMap<String, Glyph> {
    let mut out = HashMap::new();
    let Some(dir) = dir() else {
        return out;
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "png") {
            continue;
        }
        // Le nom réel est dans le fichier compagnon `.name` (le nom de fichier est assaini) ;
        // à défaut, le nom de fichier lui-même.
        let name = std::fs::read_to_string(path.with_extension("name"))
            .ok()
            .map(|s| s.trim().to_string())
            .or_else(|| path.file_stem().map(|s| s.to_string_lossy().to_string()));
        let Some(name) = name else { continue };
        match std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|b| Glyph::from_png(&b).map_err(|e| e.to_string()))
        {
            Ok(glyph) => {
                // Nom de personnage en `debug` seulement (constat C6 de `docs/analyse-rgpd.md`)
                // : ce qui se diagnostique ici, c'est le NOMBRE de gabarits relus et leur taille.
                tracing::info!("[tour] gabarit chargé ({}x{})", glyph.w, glyph.h);
                tracing::debug!(character = %name, "[tour] gabarit chargé");
                out.insert(name, glyph);
            }
            Err(err) => tracing::warn!(
                "[tour] gabarit illisible {} : {err}",
                overlay_ingest::privacy::redact_path(&path)
            ),
        }
    }
    out
}

pub fn save(character: &str, glyph: &Glyph) {
    let Some(dir) = dir() else {
        return;
    };
    let result = (|| -> Result<(), String> {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let stem = file_stem(character);
        let png = glyph.to_png().map_err(|e| e.to_string())?;
        std::fs::write(dir.join(format!("{stem}.png")), png).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(format!("{stem}.name")), character).map_err(|e| e.to_string())
    })();
    match result {
        Ok(()) => {
            tracing::info!(
                "[tour] gabarit enregistré → {}",
                overlay_ingest::privacy::redact_path(&dir)
            );
            tracing::debug!(%character, "[tour] gabarit enregistré");
        }
        Err(err) => {
            tracing::warn!("[tour] gabarit non enregistré : {err}");
            tracing::debug!(%character, "[tour] gabarit non enregistré");
        }
    }
}
