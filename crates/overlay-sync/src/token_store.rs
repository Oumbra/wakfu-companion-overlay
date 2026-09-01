//! Stockage du jeton natif — trousseau OS (`keyring`, Credential Manager sous Windows, Secret
//! Service sous Linux) en priorité, repli fichier `0600` explicite et signalé (jamais silencieux,
//! `docs/plan-architecture.md` §7.2) quand aucun trousseau n'est disponible (WM minimalistes sous
//! Linux sans Secret Service).

use std::path::PathBuf;

use crate::SyncError;

const SERVICE: &str = "wakfu-companion-overlay";
const ACCOUNT: &str = "native-session";

fn entry() -> Result<keyring::Entry, SyncError> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(|err| SyncError::TokenStore(err.to_string()))
}

fn token_file_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "wakfu-companion-overlay")
        .map(|dirs| dirs.data_dir().join("native-session.token"))
        .unwrap_or_else(|| PathBuf::from("native-session.token"))
}

fn save_token_file(token: &str) -> Result<(), SyncError> {
    let path = token_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| SyncError::TokenStore(err.to_string()))?;
    }
    std::fs::write(&path, token).map_err(|err| SyncError::TokenStore(err.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|err| SyncError::TokenStore(err.to_string()))?;
    }
    tracing::warn!(
        path = %path.display(),
        "trousseau OS indisponible — jeton natif stocké en clair sur disque (voir §7.2 du plan)"
    );
    Ok(())
}

fn load_token_file() -> Option<String> {
    std::fs::read_to_string(token_file_path())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Sauvegarde le jeton natif — trousseau OS, repli fichier si indisponible (tracing::warn! posé
/// dans `save_token_file`, jamais silencieux).
pub fn save_token(token: &str) -> Result<(), SyncError> {
    match entry().and_then(|e| {
        e.set_password(token)
            .map_err(|err| SyncError::TokenStore(err.to_string()))
    }) {
        Ok(()) => Ok(()),
        Err(err) => {
            tracing::warn!(%err, "échec d'écriture dans le trousseau OS, repli fichier");
            save_token_file(token)
        }
    }
}

/// `None` si aucun jeton n'a jamais été enregistré (ou trousseau ET fichier absents/illisibles) —
/// jamais une erreur : l'overlay doit démarrer sans compte lié (mode invité), voir `overlay-app`.
pub fn load_token() -> Option<String> {
    entry()
        .ok()
        .and_then(|e| e.get_password().ok())
        .or_else(load_token_file)
}

/// Efface le jeton des deux emplacements possibles — best-effort (une absence des deux côtés
/// n'est pas une erreur).
pub fn clear_token() {
    if let Ok(e) = entry() {
        let _ = e.delete_credential();
    }
    let _ = std::fs::remove_file(token_file_path());
}
