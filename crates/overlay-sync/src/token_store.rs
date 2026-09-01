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

/// Sauvegarde le jeton natif — trousseau OS, repli fichier si indisponible.
///
/// ⚠️ Écrit PUIS relit immédiatement avant de faire confiance au trousseau (`verify_keyring_write`)
/// — un `keyring::Entry::set_password` peut renvoyer `Ok` sans que l'écriture soit réellement
/// relisible par une `Entry` construite séparément (observé en session : reproductible même en
/// mono-processus, deux `Entry` indépendantes créées à la suite — cause exacte non élucidée,
/// probablement un trousseau système restreint/virtualisé dans un environnement d'exécution
/// contraint). Sans cette vérification, un jeton silencieusement non persisté ferait recommencer
/// l'appairage à **chaque** lancement sans que rien ne le signale — exactement ce que §7.2 du plan
/// interdit ("jamais silencieux").
pub fn save_token(token: &str) -> Result<(), SyncError> {
    if verify_keyring_write(token) {
        return Ok(());
    }
    tracing::warn!("trousseau OS indisponible ou écriture non relisible — repli fichier");
    save_token_file(token)
}

/// Écrit dans le trousseau puis relit via une `Entry` FRAÎCHE (jamais celle qui a écrit) pour
/// détecter une écriture qui « réussit » sans être réellement persistée — voir `save_token`.
fn verify_keyring_write(token: &str) -> bool {
    let Ok(write_entry) = entry() else {
        return false;
    };
    if write_entry.set_password(token).is_err() {
        return false;
    }
    matches!(entry().and_then(|e| e.get_password().map_err(|err| SyncError::TokenStore(err.to_string()))), Ok(readback) if readback == token)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Garde-fou de non-régression : trouvé en session (2026-09-01) en testant contre un vrai
    /// déploiement — `keyring::Entry::set_password` peut renvoyer `Ok` sans que l'écriture soit
    /// relisible par une `Entry` FRAÎCHE (reproductible même en mono-processus, deux `Entry`
    /// indépendantes créées à la suite ; cause exacte non élucidée, probablement un trousseau
    /// système restreint/virtualisé selon l'environnement d'exécution). `save_token`/`load_token`
    /// doivent rester cohérents entre eux quel que soit le backend réellement utilisé derrière —
    /// exécuté depuis un thread dédié (comme le vrai `spawn_auth_thread` d'`overlay-ui`, jamais le
    /// thread de test lui-même) pour rester fidèle au contexte réel.
    #[test]
    fn sauvegarde_puis_lecture_du_jeton_coherentes_meme_si_le_trousseau_ment() {
        let handle = std::thread::Builder::new()
            .name("token-store-test".into())
            .spawn(|| {
                clear_token(); // état propre, au cas où un run précédent aurait laissé une trace
                assert!(load_token().is_none());

                save_token("jeton-de-test-123")
                    .expect("save_token ne doit jamais échouer totalement (repli fichier)");
                assert_eq!(load_token().as_deref(), Some("jeton-de-test-123"));

                clear_token();
                assert!(
                    load_token().is_none(),
                    "clear_token doit effacer les deux emplacements"
                );
            })
            .unwrap();
        handle.join().unwrap();
    }
}
