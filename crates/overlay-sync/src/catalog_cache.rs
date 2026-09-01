//! Cache disque du catalogue objets/monstres (§7.4 du plan) — évite de retélécharger ~350 Ko
//! gzip à chaque lancement quand le contenu n'a pas changé (voir `client::fetch_catalog_version`,
//! comparé à l'empreinte du cache avant tout retéléchargement). Même famille que
//! `token_store.rs` : un fichier JSON sous le dossier de données de l'app
//! (`directories::ProjectDirs`), jamais une base de données pour un besoin aussi simple.
//!
//! Repli hors-ligne EMBARQUÉ (`assets/catalog/…`, prévu au plan) volontairement absent de cette
//! itération : sans cache disque ET sans réseau au tout premier lancement, le catalogue reste
//! simplement vide — les tuiles du panneau Suivi retombent sur l'icône générique, jamais une
//! erreur bloquante (voir `overlay_engine::CatalogIndex::is_empty`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::SyncError;

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
}
