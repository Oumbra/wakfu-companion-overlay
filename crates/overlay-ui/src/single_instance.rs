//! **Une seule instance de l'overlay par session utilisateur** (2026-09-23).
//!
//! Deux overlays lancés en même temps (double-clic sur l'exe, raccourci plus démarrage avec
//! l'ordinateur) partageaient le même jeton et la même file d'envoi SQLite
//! (`overlay_sync::queue`) : chacun envoyait les mêmes lots, et chacun relisait `wakfu.log` pour
//! mettre en file les mêmes combats. Le serveur absorbait les doublons (`clientKey`), mais au prix
//! de requêtes doublées contre la limite de débit du compte, et de deux jeux de fenêtres par-dessus
//! le jeu.
//!
//! **Mécanisme : un verrou exclusif sur un fichier**, `std::fs::File::try_lock` (stable depuis Rust
//! 1.89 : `flock` sous Linux, `LockFileEx` sous Windows) — aucune dépendance ajoutée. Le système
//! lève le verrou à la mort du processus, plantage compris : pas de fichier « PID » périmé à
//! nettoyer. Le PID du détenteur est écrit dans le fichier, pour le journal seulement.
//!
//! **Où** : dans un dossier propre à l'utilisateur et hors de la racine de données, que
//! « Supprimer les données locales » efface pendant que l'overlay tourne
//! (`local_data::Scope::Everything`) — `%TEMP%` sous Windows (profil de l'utilisateur),
//! `$XDG_RUNTIME_DIR` sous Linux (`/run/user/<uid>`, `0700`), repli sur le dossier de données
//! quand ce dernier n'existe pas. Un seul verrou par utilisateur, quel que soit le déploiement
//! visé : un exe de preview et un exe de release partagent aussi la file d'envoi.
//!
//! **Relance** (bouton « Redémarrer », installation d'une mise à jour) : le nouveau processus est
//! lancé AVANT que l'ancien ne sorte (`restart`, `overlay_sync::update::apply`). Il porte
//! [`RELAUNCH_ENV`] et attend alors le verrou jusqu'à [`RELAUNCH_WAIT`] ; un lancement ordinaire
//! ne l'attend pas.
//!
//! **Seconde instance** : message à l'utilisateur (boîte de dialogue sous Windows, où l'exe n'a
//! pas de console ; sortie d'erreur sous Linux) et sortie propre. Pas de mise au premier plan de
//! l'instance existante : l'overlay n'a pas de fenêtre principale à qui la donner.

use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// Même précaution que partout ailleurs (`overlay_engine::app_dirs`) : un test ne doit jamais
// prendre le verrou de l'overlay réel de la personne qui lance la suite.
#[cfg(not(test))]
const APP_NAME: &str = "wakfu-companion-overlay";
#[cfg(test)]
const APP_NAME: &str = "wakfu-companion-overlay-test";

/// Variable d'environnement posée sur le processus relancé par l'overlay lui-même (voir la doc de
/// module). Sa valeur n'importe pas. Définie côté `overlay_sync`, qui relance après une mise à
/// jour sans dépendre de ce crate.
pub use overlay_sync::update::apply::RELAUNCH_ENV;

/// Attente maximale du verrou pour un processus relancé : le temps que l'ancien ferme ses
/// fenêtres et sorte de sa boucle d'événements.
pub const RELAUNCH_WAIT: Duration = Duration::from_secs(30);

const POLL: Duration = Duration::from_millis(100);

/// Verrou détenu — à garder vivant jusqu'à la fin de `main` : le relâcher (drop) lève le verrou.
#[derive(Debug)]
pub struct InstanceLock {
    _file: File,
    path: PathBuf,
}

impl InstanceLock {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Pourquoi le verrou n'a pas été obtenu.
#[derive(Debug)]
pub enum AcquireError {
    /// Une autre instance le détient. `pid` : celui qu'elle a écrit, quand le fichier se lit
    /// (sous Windows, la plage verrouillée n'est pas lisible — `None`).
    AlreadyRunning { pid: Option<u32> },
    /// Le fichier de verrou n'a pas pu être ouvert ou verrouillé (dossier absent, droits,
    /// système de fichiers sans verrou) — l'appelant démarre quand même, un verrou impossible
    /// ne doit pas empêcher d'utiliser l'overlay.
    Io(std::io::Error),
}

/// Emplacement du verrou pour l'utilisateur courant — voir la doc de module.
pub fn default_lock_path() -> Option<PathBuf> {
    let file_name = format!("{APP_NAME}.lock");
    #[cfg(windows)]
    {
        Some(std::env::temp_dir().join(file_name))
    }
    #[cfg(not(windows))]
    {
        if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|dir| runtime_dir_is_private(dir))
        {
            return Some(dir.join(file_name));
        }
        overlay_engine::app_dirs::project_dirs(APP_NAME)
            .map(|dirs| dirs.data_dir().join("instance.lock"))
    }
}

/// `$XDG_RUNTIME_DIR` n'est retenu que s'il est **à nous et privé** (audit de sécurité du
/// 2026-09-23, O10) : chemin absolu, vrai dossier (pas un lien symbolique), propriété de
/// l'utilisateur effectif, mode exactement `0700` — ce que la spécification XDG impose et que
/// `systemd-logind` fournit. Une variable héritée d'un autre compte (`sudo -E`, `su` sans `-`) ou
/// pointant vers un dossier partagé laisserait un tiers poser ou bloquer le verrou ; on retombe
/// alors sur le dossier de données.
#[cfg(not(windows))]
fn runtime_dir_is_private(dir: &Path) -> bool {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
    if !dir.is_absolute() {
        return false;
    }
    let Ok(meta) = std::fs::symlink_metadata(dir) else {
        return false;
    };
    // SAFETY: `geteuid` ne lit que l'identité du processus, sans effet de bord ni échec possible.
    let euid = unsafe { libc::geteuid() };
    meta.is_dir() && meta.uid() == euid && meta.permissions().mode() & 0o777 == 0o700
}

/// Combien attendre le verrou : [`RELAUNCH_WAIT`] si ce processus a été relancé par l'overlay
/// lui-même, sinon rien. La variable est retirée au passage : un processus que CELUI-CI lancerait
/// plus tard sans la poser ne doit pas en hériter.
pub fn wait_for_this_launch() -> Duration {
    let relaunched = std::env::var_os(RELAUNCH_ENV).is_some();
    if relaunched {
        // Appelé en tête de `main`, avant tout autre thread.
        std::env::remove_var(RELAUNCH_ENV);
        RELAUNCH_WAIT
    } else {
        Duration::ZERO
    }
}

/// Prend le verrou `path`, en l'attendant jusqu'à `wait` s'il est détenu.
pub fn acquire_at(path: &Path, wait: Duration) -> Result<InstanceLock, AcquireError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AcquireError::Io)?;
    }
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create(true)
        // Jamais tronqué à l'ouverture : c'est peut-être le fichier d'une instance en cours.
        .truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        // Jamais à travers un lien symbolique (O10) : un lien planté à la place du verrou ferait
        // écrire notre PID dans le fichier qu'il désigne. Fichier créé privé (`0600`).
        options.custom_flags(libc::O_NOFOLLOW).mode(0o600);
    }
    let mut file = options.open(path).map_err(AcquireError::Io)?;
    let deadline = Instant::now() + wait;
    loop {
        match file.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err(AcquireError::AlreadyRunning {
                        pid: read_pid(&mut file),
                    });
                }
                std::thread::sleep(POLL);
            }
            Err(std::fs::TryLockError::Error(err)) => return Err(AcquireError::Io(err)),
        }
    }
    // Diagnostic seulement : un échec d'écriture ne remet pas le verrou en cause.
    let _ = file
        .set_len(0)
        .and_then(|_| file.seek(SeekFrom::Start(0)))
        .and_then(|_| file.write_all(std::process::id().to_string().as_bytes()))
        .and_then(|_| file.flush());
    Ok(InstanceLock {
        _file: file,
        path: path.to_path_buf(),
    })
}

fn read_pid(file: &mut File) -> Option<u32> {
    let mut content = String::new();
    file.seek(SeekFrom::Start(0)).ok()?;
    file.read_to_string(&mut content).ok()?;
    content.trim().parse().ok()
}

/// Le message montré à la seconde instance.
pub const ALREADY_RUNNING_MESSAGE: &str = "Wakfu Companion Overlay est déjà lancé.\n\n\
     Son icône se trouve dans la zone de notification. Pour le relancer, quittez-le d'abord \
     (ou utilisez « Redémarrer » dans ses options).";

/// Prévient l'utilisateur qu'une instance tourne déjà — boîte de dialogue sous Windows (l'exe n'a
/// pas de console), sortie d'erreur sous Linux.
pub fn notify_already_running() {
    eprintln!("{ALREADY_RUNNING_MESSAGE}");
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::UI::WindowsAndMessaging::{
            MessageBoxW, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND,
        };
        let wide = |s: &str| {
            s.encode_utf16()
                .chain(std::iter::once(0))
                .collect::<Vec<u16>>()
        };
        let text = wide(ALREADY_RUNNING_MESSAGE);
        let title = wide("Wakfu Companion Overlay");
        // SAFETY: deux chaînes UTF-16 terminées par un zéro, vivantes pendant l'appel ; aucune
        // fenêtre propriétaire (`None`).
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                PCWSTR(title.as_ptr()),
                MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND,
            );
        }
    }
}

/// Verdict du garde de démarrage — voir [`guard`].
#[derive(Debug)]
pub enum Startup {
    /// Démarrer. Le verrou (quand il a pu être posé) est à garder vivant jusqu'à la fin de `main`.
    Proceed(Option<InstanceLock>),
    /// Une autre instance tourne : l'utilisateur est prévenu, l'appelant sort sans rien
    /// initialiser.
    Exit,
}

/// **Le garde de démarrage des deux hôtes.** Un verrou impossible à poser (voir
/// [`AcquireError::Io`]) laisse démarrer, avec un avertissement sur la sortie d'erreur (le journal
/// n'est pas encore ouvert à ce stade, et ne doit pas l'être : une seconde instance ne doit pas
/// toucher aux fichiers de la première).
pub fn guard() -> Startup {
    let Some(path) = default_lock_path() else {
        return Startup::Proceed(None);
    };
    match acquire_at(&path, wait_for_this_launch()) {
        Ok(lock) => Startup::Proceed(Some(lock)),
        Err(AcquireError::AlreadyRunning { pid }) => {
            if let Some(pid) = pid {
                eprintln!("instance déjà en cours : pid {pid} ({})", path.display());
            }
            notify_already_running();
            Startup::Exit
        }
        Err(AcquireError::Io(err)) => {
            eprintln!(
                "verrou d'instance unique indisponible ({}) : {err} — démarrage sans verrou",
                path.display()
            );
            Startup::Proceed(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_lock(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("single-instance-{}-{name}", std::process::id()))
            .join("instance.lock")
    }

    /// O10 : `$XDG_RUNTIME_DIR` n'est retenu que privé (`0700`, à nous, pas un lien).
    #[cfg(unix)]
    #[test]
    fn dossier_d_execution_retenu_seulement_prive() {
        use std::os::unix::fs::PermissionsExt as _;
        let base = std::env::temp_dir().join(format!("single-instance-{}-xdg", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let private = base.join("prive");
        std::fs::create_dir_all(&private).unwrap();
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(runtime_dir_is_private(&private));

        let shared = base.join("partage");
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(!runtime_dir_is_private(&shared));

        let link = base.join("lien");
        std::os::unix::fs::symlink(&private, &link).unwrap();
        assert!(!runtime_dir_is_private(&link), "lien symbolique refusé");
        assert!(!runtime_dir_is_private(Path::new("relatif")));
        assert!(!runtime_dir_is_private(&base.join("absent")));
        let _ = std::fs::remove_dir_all(&base);
    }

    /// O10 : le fichier de verrou ne s'ouvre jamais à travers un lien symbolique.
    #[cfg(unix)]
    #[test]
    fn verrou_refuse_un_lien_symbolique() {
        let path = temp_lock("lien");
        let dir = path.parent().unwrap().to_path_buf();
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("cible");
        std::fs::write(&target, "ne pas toucher").unwrap();
        std::os::unix::fs::symlink(&target, &path).unwrap();
        assert!(matches!(
            acquire_at(&path, Duration::ZERO),
            Err(AcquireError::Io(_))
        ));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "ne pas toucher");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Le second verrou échoue tant que le premier est détenu, et réussit dès qu'il est relâché —
    /// `flock`/`LockFileEx` valent entre deux descripteurs du même processus, ce qui permet de
    /// tester sans lancer de second processus.
    #[test]
    fn un_seul_detenteur_a_la_fois() {
        let path = temp_lock("exclusif");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());

        let first = acquire_at(&path, Duration::ZERO).expect("premier verrou");
        assert_eq!(first.path(), path.as_path());
        match acquire_at(&path, Duration::ZERO) {
            Err(AcquireError::AlreadyRunning { pid }) => {
                // Lisible sous Linux ; sous Windows la plage verrouillée ne se lit pas.
                if cfg!(unix) {
                    assert_eq!(pid, Some(std::process::id()));
                }
            }
            other => panic!("attendu AlreadyRunning, obtenu {other:?}"),
        }
        drop(first);
        let again = acquire_at(&path, Duration::ZERO).expect("verrou relâché au drop");
        drop(again);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// Un processus relancé attend que l'ancien relâche le verrou, au lieu d'abandonner.
    #[test]
    fn une_relance_attend_la_fin_de_l_ancienne_instance() {
        let path = temp_lock("relance");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());

        let first = acquire_at(&path, Duration::ZERO).unwrap();
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            drop(first);
        });
        let started = Instant::now();
        let second = acquire_at(&path, Duration::from_secs(10)).expect("obtenu après relâche");
        assert!(started.elapsed() >= Duration::from_millis(250));
        releaser.join().unwrap();
        drop(second);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    /// Un fichier de verrou laissé par une instance morte (plantage) n'empêche rien : c'est le
    /// verrou du système qui compte, pas l'existence du fichier.
    #[test]
    fn un_fichier_orphelin_ne_bloque_pas() {
        let path = temp_lock("orphelin");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "999999").unwrap();

        let lock = acquire_at(&path, Duration::ZERO).expect("fichier orphelin repris");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap_or_default().trim(),
            std::process::id().to_string()
        );
        drop(lock);

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn le_verrou_vit_hors_de_la_racine_de_donnees_quand_c_est_possible() {
        let path = default_lock_path().expect("un emplacement existe");
        assert!(path.to_string_lossy().contains(APP_NAME), "{path:?}");
    }
}
