//! Cache disque des référentiels donjons/familles de monstres (§7.4 du plan, volet « reste » du
//! lot L3) — même principe que `catalog_cache.rs` (fichier JSON sous le dossier de données de
//! l'app, chargé immédiatement au démarrage avant tout accès réseau), en plus simple : ni
//! `GET /api/v1/dungeons` ni `GET /api/v1/monster-families` n'exposent d'endpoint `/version`
//! séparé (contrairement au catalogue objets/monstres, voir `functions/api/v1/catalog/version.ts`
//! côté `wakfu-companion`) — ces deux référentiels sont rechargés inconditionnellement en tâche de
//! fond à chaque lancement plutôt que comparés à une empreinte, volume négligeable oblige (~150
//! lignes chacun, voir la doc de tête de `dungeon.rs`/`monster_family.rs`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::SyncError;

#[cfg(not(test))]
const APP_NAME: &str = "wakfu-companion-overlay";
#[cfg(test)]
const APP_NAME: &str = "wakfu-companion-overlay-test";

/// Quel référentiel — détermine uniquement le nom du fichier de cache (voir `file_path`), les deux
/// suivent exactement la même mécanique de lecture/écriture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceData {
    Dungeons,
    MonsterFamilies,
}

impl ReferenceData {
    fn file_name(self) -> &'static str {
        match self {
            ReferenceData::Dungeons => "dungeons-cache.json",
            ReferenceData::MonsterFamilies => "monster-families-cache.json",
        }
    }
}

fn cache_file_path(which: ReferenceData) -> PathBuf {
    directories::ProjectDirs::from("", "", APP_NAME)
        .map(|dirs| dirs.data_dir().join(which.file_name()))
        .unwrap_or_else(|| PathBuf::from(which.file_name()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedReferenceData {
    /// Réponse brute du endpoint (`Vec<{...}>`) — stockée telle quelle, même raison qu'`index` dans
    /// `catalog_cache::CachedCatalog` : `DungeonIndex::from_json`/`MonsterFamilyIndex::from_json`
    /// restent l'unique point de construction de l'index.
    rows: serde_json::Value,
}

/// Charge le cache local — `None` au premier lancement ou si absent/corrompu, jamais une erreur
/// (même politique que `catalog_cache::load`).
pub fn load(which: ReferenceData) -> Option<serde_json::Value> {
    let content = std::fs::read_to_string(cache_file_path(which)).ok()?;
    let cached: CachedReferenceData = serde_json::from_str(&content).ok()?;
    Some(cached.rows)
}

/// Écrit le cache local — best-effort, jamais fatal pour l'appelant (voir `catalog_cache::save`).
pub fn save(which: ReferenceData, rows: &serde_json::Value) -> Result<(), SyncError> {
    let path = cache_file_path(which);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| SyncError::TokenStore(err.to_string()))?;
    }
    let json = serde_json::to_string(&CachedReferenceData { rows: rows.clone() })
        .map_err(|err| SyncError::Json(err.to_string()))?;
    std::fs::write(&path, json).map_err(|err| SyncError::TokenStore(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Même précaution que `catalog_cache` : exécuté sur son propre thread, `APP_NAME` distinct en
    /// test — jamais le vrai cache de l'utilisateur.
    #[test]
    fn sauvegarde_puis_lecture_coherentes_pour_chaque_referentiel() {
        let handle = std::thread::Builder::new()
            .name("reference-data-cache-test".into())
            .spawn(|| {
                for which in [ReferenceData::Dungeons, ReferenceData::MonsterFamilies] {
                    let rows = serde_json::json!([{ "id": 1, "fr": "Test" }]);
                    save(which, &rows).expect("save ne doit pas échouer");
                    assert_eq!(load(which), Some(rows));
                    let _ = std::fs::remove_file(cache_file_path(which));
                }
            })
            .unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn absence_de_cache_ne_plante_pas() {
        let _ = std::fs::remove_file(cache_file_path(ReferenceData::Dungeons));
        assert!(load(ReferenceData::Dungeons).is_none());
    }
}
