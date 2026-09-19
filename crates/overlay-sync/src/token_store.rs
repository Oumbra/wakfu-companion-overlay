//! Stockage du jeton natif — trousseau OS (`keyring`, Credential Manager sous Windows, Secret
//! Service sous Linux) en priorité, repli fichier `0600` explicite et signalé (jamais silencieux,
//! `docs/plan-architecture.md` §7.2) quand aucun trousseau n'est disponible (WM minimalistes sous
//! Linux sans Secret Service).

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::SyncError;

// Nom d'app DISTINCT en test (`#[cfg(test)]`) — un bug réel a été trouvé en session (2026-09-01) :
// le test de non-régression ci-dessous appelait `clear_token()` sur le VRAI jeton natif de
// l'utilisateur (même trousseau/même fichier que la production), effaçant silencieusement une
// session déjà appairée à chaque `cargo test`. Jamais de recouvrement possible entre l'app réelle
// et ses propres tests.
#[cfg(not(test))]
const APP_NAME: &str = "wakfu-companion-overlay";
#[cfg(test)]
const APP_NAME: &str = "wakfu-companion-overlay-test";

const ACCOUNT: &str = "native-session";

fn entry() -> Result<keyring::Entry, SyncError> {
    keyring::Entry::new(APP_NAME, ACCOUNT).map_err(|err| SyncError::TokenStore(err.to_string()))
}

fn token_file_path() -> PathBuf {
    data_file("native-session.token")
}

/// **Quand le jeton courant a été émis** — secondes Unix, à côté du fichier de repli. Ce n'est pas
/// un secret (une date), et le trousseau n'a pas de place pour une seconde valeur : un fichier
/// suffit. Écrit par [`save_token`], lu par [`token_age`], retiré par [`clear_token`].
fn issued_at_file_path() -> PathBuf {
    data_file("native-session.issued-at")
}

fn data_file(name: &str) -> PathBuf {
    overlay_engine::app_dirs::project_dirs(APP_NAME)
        .map(|dirs| dirs.data_dir().join(name))
        .unwrap_or_else(|| PathBuf::from(name))
}

/// Depuis combien de temps le jeton courant a été émis — `None` si la date n'a jamais été
/// enregistrée (jeton sauvegardé par une version antérieure au 2026-09-19) ou n'est plus lisible.
/// C'est ce que `background::attempt_connect` compare au seuil de rotation ; un âge inconnu vaut
/// « à renouveler », ce qui pose la date au passage.
pub fn token_age() -> Option<Duration> {
    let secs: u64 = std::fs::read_to_string(issued_at_file_path())
        .ok()?
        .trim()
        .parse()
        .ok()?;
    SystemTime::now()
        .duration_since(UNIX_EPOCH + Duration::from_secs(secs))
        .ok()
}

fn record_issued_now() {
    let path = issued_at_file_path();
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let written = path
        .parent()
        .map(std::fs::create_dir_all)
        .unwrap_or(Ok(()))
        .and_then(|_| std::fs::write(&path, secs.to_string()));
    if let Err(err) = written {
        // Pas bloquant : sans date, le prochain lancement renouvellera le jeton une fois de plus.
        tracing::warn!(path = %path.display(), %err, "date d'émission du jeton non enregistrée");
    }
}

/// Le fichier de repli **existe** : le trousseau a manqué au moins une fois et le jeton est sur
/// disque en clair. C'est ce que la fenêtre Options (section « Compte ») dit à l'utilisateur
/// (constat C7 de `docs/analyse-rgpd.md`, 2026-09-19) — le `warn!` du journal ne suffit pas, il ne
/// le lit pas. `clear_token` (déconnexion, effacement) retire le fichier, et l'avis avec lui.
pub fn token_file_in_use() -> bool {
    token_file_path().is_file()
}

/// Où vit le fichier de repli — pour le dire à l'utilisateur à côté de [`token_file_in_use`].
pub fn token_file_location() -> PathBuf {
    token_file_path()
}

fn save_token_file(token: &str) -> Result<(), SyncError> {
    save_token_file_at(&token_file_path(), token)
}

fn save_token_file_at(path: &std::path::Path, token: &str) -> Result<(), SyncError> {
    use std::io::Write as _;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| SyncError::TokenStore(err.to_string()))?;
    }
    // **Le mode restrictif se pose à la création, pas après** (constat C7, 2026-09-19) : un
    // `fs::write` puis `set_permissions` laissait le fichier lisible par tout utilisateur de la
    // machine (umask courante, `0644`) le temps de l'écriture. `mode(0o600)` ne vaut qu'à la
    // création — un fichier déjà là garde ses bits, d'où le `set_permissions` conservé derrière.
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|err| SyncError::TokenStore(err.to_string()))?;
    file.write_all(token.as_bytes())
        .map_err(|err| SyncError::TokenStore(err.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|err| SyncError::TokenStore(err.to_string()))?;
    }
    // Sous Windows, `%APPDATA%` est déjà réservé au compte utilisateur par l'ACL héritée du
    // profil ; pas de DPAPI pour l'instant (choix du 2026-09-19 : signaler d'abord, chiffrer
    // ensuite si le cas se présente réellement).
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

/// Sauvegarde le jeton natif — trousseau OS, repli fichier si indisponible — et **date son
/// émission** ([`token_age`]) : un jeton sauvegardé est un jeton que le serveur vient d'émettre,
/// à l'appairage comme à la rotation (`client::rotate_native_session`).
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
    record_issued_now();
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
/// jamais une erreur : l'overlay doit pouvoir démarrer sans compte lié et afficher sa fenêtre de connexion.
pub fn load_token() -> Option<String> {
    entry()
        .ok()
        .and_then(|e| e.get_password().ok())
        .or_else(load_token_file)
}

/// Efface le jeton des deux emplacements possibles, et sa date d'émission — best-effort (une
/// absence n'est pas une erreur).
pub fn clear_token() {
    if let Ok(e) = entry() {
        let _ = e.delete_credential();
    }
    let _ = std::fs::remove_file(token_file_path());
    let _ = std::fs::remove_file(issued_at_file_path());
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
                // Daté à la sauvegarde : c'est ce que la rotation au démarrage consulte.
                let age = token_age().expect("date d'émission enregistrée avec le jeton");
                assert!(age < Duration::from_secs(60), "{age:?}");

                clear_token();
                assert!(
                    load_token().is_none(),
                    "clear_token doit effacer les deux emplacements"
                );
                assert!(token_age().is_none(), "la date part avec le jeton");
            })
            .unwrap();
        handle.join().unwrap();
    }

    /// Le fichier de repli naît en `0600` — et le reste quand il préexistait plus permissif.
    /// Dans un dossier à part : le test ci-dessus peut lui aussi passer par le fichier de repli
    /// (trousseau absent en CI), les deux ne doivent pas se marcher dessus.
    #[cfg(unix)]
    #[test]
    fn le_fichier_de_repli_est_prive_des_sa_creation() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("token-store-mode-{}", std::process::id()));
        let path = dir.join("native-session.token");
        let _ = std::fs::remove_dir_all(&dir);
        save_token_file_at(&path, "jeton-de-test-mode").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        save_token_file_at(&path, "jeton-de-test-mode-2").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "un fichier préexistant est remis en 0600");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "jeton-de-test-mode-2",
            "tronqué puis réécrit, pas complété"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
