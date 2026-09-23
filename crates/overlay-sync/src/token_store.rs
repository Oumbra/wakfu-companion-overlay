//! Stockage du jeton natif — trousseau OS (`keyring`, Credential Manager sous Windows, Secret
//! Service sous Linux) en priorité, repli fichier `0600` explicite et signalé (jamais silencieux,
//! `docs/plan-architecture.md` §7.2) quand aucun trousseau n'est disponible (WM minimalistes sous
//! Linux sans Secret Service).
//!
//! **Un jeton par déploiement** (2026-09-21). Un jeton natif n'est valable que pour le
//! déploiement qui l'a émis (sa ligne de session vit dans SA base) ; un poste peut faire tourner un
//! exe de release (prod) et un exe de preview (dev) — voir `build.rs`. Jusque-là ils partageaient
//! le même emplacement et s'écrasaient mutuellement, d'où le 401 en boucle décrit dans `build.rs`.
//! L'emplacement est désormais dérivé de `client::base_url()` ([`slot`]) : la prod garde les noms
//! historiques (`native-session`, aucune migration pour les sessions existantes), tout autre
//! déploiement suffixe l'hôte (`native-session@claude-dev.wakfu-companion.com`). C'est aussi ce
//! qui permet à `gen-catalog-fallback` de retrouver le jeton du déploiement qu'il vise.

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
/// Hôte du déploiement dont l'emplacement n'est pas suffixé — la prod, pour ne pas invalider les
/// sessions appairées avant le 2026-09-21 (voir la doc de tête).
const UNSUFFIXED_HOST: &str = "wakfu-companion.com";

/// Nom d'emplacement (compte du trousseau, base des fichiers de repli) pour le déploiement visé —
/// voir la doc de tête.
fn slot() -> String {
    slot_for(&crate::client::base_url())
}

fn slot_for(base_url: &str) -> String {
    let host = base_url
        .trim_end_matches('/')
        .split("://")
        .nth(1)
        .unwrap_or(base_url)
        .to_ascii_lowercase();
    if host == UNSUFFIXED_HOST {
        return ACCOUNT.to_string();
    }
    // `localhost:8788` (`wrangler pages dev`) : `:` interdit dans un nom de fichier Windows.
    let host: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{ACCOUNT}@{host}")
}

fn entry() -> Result<keyring::Entry, SyncError> {
    keyring::Entry::new(APP_NAME, &slot()).map_err(|err| SyncError::TokenStore(err.to_string()))
}

fn token_file_path() -> PathBuf {
    data_file(&format!("{}.token", slot()))
}

/// **Quand le jeton courant a été émis** — secondes Unix, à côté du fichier de repli. Ce n'est pas
/// un secret (une date), et le trousseau n'a pas de place pour une seconde valeur : un fichier
/// suffit. Écrit par [`save_token`], lu par [`token_age`], retiré par [`clear_token`].
fn issued_at_file_path() -> PathBuf {
    data_file(&format!("{}.issued-at", slot()))
}

fn data_dir() -> Option<PathBuf> {
    overlay_engine::app_dirs::project_dirs(APP_NAME).map(|dirs| dirs.data_dir().to_path_buf())
}

fn data_file(name: &str) -> PathBuf {
    data_dir()
        .map(|dir| dir.join(name))
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
/// relisible par une `Entry` construite séparément. Cause élucidée par l'audit du 2026-09-23 :
/// `keyring` était compilé sans aucune feature de backend, donc sur son magasin `mock` (en
/// mémoire, propre à chaque `Entry`). Les backends sont désormais déclarés (`Cargo.toml`), mais
/// la vérification reste : un Secret Service absent ou verrouillé sous Linux doit toujours
/// retomber sur le fichier de repli, jamais sur un jeton silencieusement perdu (§7.2 du plan,
/// "jamais silencieux").
///
/// Une fois le jeton dans le trousseau, un fichier de repli laissé par une sauvegarde antérieure
/// est supprimé : il contiendrait un jeton en clair, périmé, et l'avis des Options resterait
/// affiché à tort.
pub fn save_token(token: &str) -> Result<(), SyncError> {
    record_issued_now();
    if verify_keyring_write(token) {
        remove_token_file();
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
    if let Some(token) = entry().ok().and_then(|e| e.get_password().ok()) {
        return Some(token);
    }
    let token = load_token_file()?;
    // Jeton resté dans le fichier de repli (sauvegardé avant que le trousseau ne soit réellement
    // compilé, ou pendant une indisponibilité passagère) : migré dès que le trousseau répond.
    if verify_keyring_write(&token) {
        remove_token_file();
        tracing::info!("jeton natif migré du fichier de repli vers le trousseau OS");
    }
    Some(token)
}

fn remove_token_file() {
    let path = token_file_path();
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            tracing::warn!(path = %path.display(), %err, "fichier de repli du jeton non supprimé")
        }
    }
}

/// Efface le jeton **du déploiement courant** des deux emplacements possibles, et sa date
/// d'émission — best-effort (une absence n'est pas une erreur). C'est le geste de la déconnexion ;
/// l'effacement complet passe par [`clear_all_tokens`].
pub fn clear_token() {
    clear_slot(&slot());
}

/// Efface le jeton de **tous les déploiements** connus sur ce poste — pour « Supprimer les données
/// locales » (`overlay_ui::local_data`, constat C5 de `docs/analyse-rgpd.md`), qui promet l'état
/// d'une installation neuve. Depuis le 2026-09-21 chaque déploiement a son emplacement
/// ([`slot`]) : les fichiers de repli partent avec la racine de dossiers, mais une entrée du
/// trousseau posée par un autre exe (preview contre dev, release contre prod) survivrait à
/// [`clear_token`], qui ne vise que l'emplacement de l'exe qui l'appelle.
///
/// Les emplacements visés : la prod et le déploiement dev (les deux seuls que `build.rs` fige),
/// celui de l'exe courant (une surcharge `WAKFU_COMPANION_API_URL`), et tout emplacement dont un
/// fichier `<slot>.token` ou `<slot>.issued-at` subsiste dans le dossier de données — `save_token`
/// date chaque émission sur disque, trousseau ou pas, donc un déploiement appairé depuis le
/// 2026-09-19 y laisse toujours sa trace. Le trousseau, lui, ne s'énumère pas par préfixe.
pub fn clear_all_tokens() {
    let mut slots: std::collections::BTreeSet<String> = KNOWN_HOSTS
        .iter()
        .map(|host| slot_for(&format!("https://{host}")))
        .collect();
    slots.insert(slot());
    if let Some(dir) = data_dir() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            slots.extend(
                entries
                    .flatten()
                    .filter_map(|entry| slot_from_file_name(&entry.file_name().to_string_lossy())),
            );
        }
    }
    for slot in slots {
        clear_slot(&slot);
    }
}

/// Hôtes des déploiements figés par `build.rs` — les seuls qu'un exe livré ou de preview vise
/// sans surcharge à l'exécution.
const KNOWN_HOSTS: &[&str] = &["wakfu-companion.com", "claude-dev.wakfu-companion.com"];

/// L'emplacement dont `name` est un fichier (`<slot>.token`, `<slot>.issued-at`) — `None` pour
/// tout autre fichier du dossier de données.
fn slot_from_file_name(name: &str) -> Option<String> {
    let stem = name
        .strip_suffix(".token")
        .or_else(|| name.strip_suffix(".issued-at"))?;
    (stem == ACCOUNT || stem.starts_with(&format!("{ACCOUNT}@"))).then(|| stem.to_string())
}

fn clear_slot(slot: &str) {
    if let Ok(e) = keyring::Entry::new(APP_NAME, slot) {
        let _ = e.delete_credential();
    }
    let _ = std::fs::remove_file(data_file(&format!("{slot}.token")));
    let _ = std::fs::remove_file(data_file(&format!("{slot}.issued-at")));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_emplacement_par_deploiement_la_prod_gardant_le_nom_historique() {
        assert_eq!(slot_for("https://wakfu-companion.com"), "native-session");
        assert_eq!(slot_for("https://WAKFU-companion.com/"), "native-session");
        assert_eq!(
            slot_for("https://claude-dev.wakfu-companion.com"),
            "native-session@claude-dev.wakfu-companion.com"
        );
        assert_eq!(
            slot_for("http://localhost:8788"),
            "native-session@localhost_8788"
        );
    }

    /// Les emplacements se retrouvent d'après leurs fichiers ; le reste du dossier de données
    /// (file de synchro, caches) n'en est pas un.
    #[test]
    fn un_emplacement_se_retrouve_d_apres_ses_fichiers() {
        assert_eq!(
            slot_from_file_name("native-session.token").as_deref(),
            Some("native-session")
        );
        assert_eq!(
            slot_from_file_name("native-session@localhost_8788.issued-at").as_deref(),
            Some("native-session@localhost_8788")
        );
        assert_eq!(slot_from_file_name("native-session"), None);
        assert_eq!(slot_from_file_name("sync-queue.sqlite3"), None);
        assert_eq!(slot_from_file_name("other.token"), None);
    }

    /// Garde-fou de non-régression : trouvé en session (2026-09-01) en testant contre un vrai
    /// déploiement — `keyring::Entry::set_password` peut renvoyer `Ok` sans que l'écriture soit
    /// relisible par une `Entry` FRAÎCHE (cause trouvée le 2026-09-23 : aucun backend compilé,
    /// magasin `mock` ; reste possible avec un Secret Service absent ou verrouillé). `save_token`/`load_token`
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
