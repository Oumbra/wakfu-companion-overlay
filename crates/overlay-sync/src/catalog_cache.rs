//! Cache disque du catalogue objets/monstres (§7.4 du plan) — évite de retélécharger ~350 Ko
//! gzip à chaque lancement quand le contenu n'a pas changé (voir `client::fetch_catalog_version`,
//! comparé à l'empreinte du cache avant tout retéléchargement). Même famille que
//! `token_store.rs` : un fichier JSON sous le dossier de données de l'app
//! (`directories::ProjectDirs`), jamais une base de données pour un besoin aussi simple.
//!
//! Repli hors-ligne EMBARQUÉ (`assets/catalog/catalog-index.json.gz`, `embedded_fallback` —
//! §7.4 du plan) : utilisé UNIQUEMENT quand ni le cache disque ci-dessus ni le réseau ne sont
//! disponibles au démarrage (voir `overlay-ui::main::spawn_catalog_thread`) — l'overlay reste
//! utilisable (icônes réelles pour les quelques entrées embarquées, icône générique sinon) plutôt
//! que de retomber sur un catalogue totalement vide au tout premier lancement hors ligne.
//!
//! ⚠️ Le fichier embarqué est un PLACEHOLDER volontairement réduit (voir sa fixture de test
//! ci-dessous pour le contenu exact) : ce sandbox de développement ne peut atteindre ni Neon ni
//! `*.pages.dev` (même limite documentée partout ailleurs dans ce dépôt et dans `wakfu-companion`,
//! voir `server/README.md` côté web) et ne peut donc pas produire le vrai catalogue complet
//! (~11 700 objets / ~850 monstres). **À régénérer depuis un vrai déploiement avant toute release**
//! via `cargo run -p overlay-sync --bin gen-catalog-fallback` (voir ce binaire) — jamais à la main.

use std::io::Read;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::SyncError;

const EMBEDDED_FALLBACK_GZ: &[u8] = include_bytes!("../assets/catalog/catalog-index.json.gz");

// Même précaution de nommage qu'ailleurs dans ce crate (`token_store.rs`) — un test ne doit
// jamais lire/écraser le VRAI cache de l'utilisateur.
#[cfg(not(test))]
const APP_NAME: &str = "wakfu-companion-overlay";
#[cfg(test)]
const APP_NAME: &str = "wakfu-companion-overlay-test";

fn cache_file_path() -> PathBuf {
    directories::ProjectDirs::from("", "", APP_NAME)
        .map(|dirs| dirs.data_dir().join("catalog-cache.json"))
        .unwrap_or_else(|| PathBuf::from("catalog-cache.json"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedCatalog {
    index_hash: String,
    /// Réponse brute de `GET /api/v1/catalog/` (`{ items: [...], monsters: [...] }`) — stockée
    /// telle quelle plutôt que déjà indexée : `overlay_engine::CatalogIndex::from_compact_json`
    /// reste l'unique point de construction de l'index, pas de logique de parsing dupliquée ici.
    index: serde_json::Value,
}

/// Charge le cache local s'il existe — `None` au premier lancement ou si le fichier est
/// corrompu/illisible, jamais une erreur (voir doc de module : un catalogue absent est un état
/// normal, pas une panne).
pub fn load() -> Option<(String, serde_json::Value)> {
    let content = std::fs::read_to_string(cache_file_path()).ok()?;
    let cached: CachedCatalog = serde_json::from_str(&content).ok()?;
    Some((cached.index_hash, cached.index))
}

/// Écrit le cache local — best-effort (voir `tracing::warn!` en cas d'échec) : un catalogue non
/// mis en cache retélécharge simplement au prochain lancement, jamais une raison de faire
/// échouer quoi que ce soit d'autre.
pub fn save(index_hash: &str, index: &serde_json::Value) -> Result<(), SyncError> {
    let path = cache_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| SyncError::TokenStore(err.to_string()))?;
    }
    let json = serde_json::to_string(&CachedCatalog {
        index_hash: index_hash.to_string(),
        index: index.clone(),
    })
    .map_err(|err| SyncError::Json(err.to_string()))?;
    std::fs::write(&path, json).map_err(|err| SyncError::TokenStore(err.to_string()))
}

/// Décompresse le repli embarqué (`assets/catalog/catalog-index.json.gz`, voir doc de module) —
/// n'échoue jamais en pratique (fichier connu à la compilation, vérifié par
/// `decompresse_sans_planter` ci-dessous) mais retombe quand même sur un catalogue vide plutôt que
/// de paniquer si le fichier venait à être corrompu par une future édition manuelle malheureuse,
/// même politique de tolérance que `load`/le reste de ce module.
pub fn embedded_fallback() -> serde_json::Value {
    let mut decoder = flate2::read::GzDecoder::new(EMBEDDED_FALLBACK_GZ);
    let mut json = String::new();
    if decoder.read_to_string(&mut json).is_err() {
        tracing::warn!("repli hors-ligne embarqué illisible (asset corrompu ?)");
        return serde_json::json!({ "items": [], "monsters": [] });
    }
    serde_json::from_str(&json).unwrap_or_else(|err| {
        tracing::warn!(%err, "repli hors-ligne embarqué mal formé (asset corrompu ?)");
        serde_json::json!({ "items": [], "monsters": [] })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cache_file_path()` n'est pas paramétrable depuis les tests (contrairement à
    /// `WatchlistState`, voir sa doc) : ce module n'a qu'un seul fichier de cache par machine, pas
    /// un par `Engine` — `APP_NAME` distinct en test (voir ci-dessus) suffit à ne jamais toucher
    /// le vrai cache de l'utilisateur, exécuté sur son propre thread pour rester représentatif
    /// (même précaution que `token_store.rs`).
    #[test]
    fn sauvegarde_puis_lecture_coherentes() {
        let handle = std::thread::Builder::new()
            .name("catalog-cache-test".into())
            .spawn(|| {
                let index = serde_json::json!({ "items": [], "monsters": [] });
                save("hash-de-test", &index).expect("save ne doit pas échouer");
                let (loaded_hash, loaded_index) = load().expect("cache attendu après save");
                assert_eq!(loaded_hash, "hash-de-test");
                assert_eq!(loaded_index, index);
                let _ = std::fs::remove_file(cache_file_path());
            })
            .unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn absence_de_cache_ne_plante_pas() {
        let _ = std::fs::remove_file(cache_file_path());
        assert!(load().is_none());
    }

    /// Non-régression du repli embarqué : vérifie que l'asset compilé dans le binaire se
    /// décompresse en un JSON exploitable par `overlay_engine::CatalogIndex::from_compact_json`
    /// (pas seulement « ne panique pas ») — un asset corrompu/mal régénéré (voir
    /// `gen-catalog-fallback`) serait détecté ici plutôt qu'au premier lancement hors ligne d'un
    /// utilisateur.
    #[test]
    fn le_repli_embarque_se_decompresse_en_catalogue_exploitable() {
        let index = embedded_fallback();
        let catalog = overlay_engine::CatalogIndex::from_compact_json(&index);
        assert!(!catalog.is_empty());
    }
}
