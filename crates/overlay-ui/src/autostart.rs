//! **Démarrage de l'overlay avec l'ordinateur** — case « Lancer l'overlay au démarrage de
//! l'ordinateur » de la section « Démarrage » de l'onglet « Paramètres » (demande utilisateur du
//! 2026-09-16).
//!
//! ## Ce réglage ne vit PAS dans `config.toml`
//!
//! C'est le seul réglage de la fenêtre Options dont l'état réel appartient au SYSTÈME : sous
//! Windows, une valeur dans `HKCU\…\CurrentVersion\Run`, que le Gestionnaire des tâches (onglet
//! « Applications de démarrage ») laisse désactiver ; sous Linux, un fichier `.desktop` dans
//! `~/.config/autostart`, que les réglages du bureau montrent et suppriment. Le dupliquer dans la
//! config créerait deux vérités qui divergeraient au premier geste fait ailleurs — la case
//! afficherait « oui » alors que rien ne démarrerait.
//!
//! La fenêtre Options lit donc l'état réel à son ouverture ([`is_enabled`]) et le repose à
//! « Valider » ([`apply`]), comme elle le fait déjà du reste : brouillon en attendant, rien
//! d'écrit si la case n'a pas bougé.
//!
//! ## Actif par défaut — une fois, au premier lancement
//!
//! Demande utilisateur du 2026-09-16 : « option de démarrage active par défaut ». Le système ne
//! sachant pas distinguer « jamais inscrit » de « retiré exprès », le défaut ne peut pas se
//! rejouer à chaque lancement : il s'applique **une seule fois**, au premier lancement qui trouve
//! le jalon `config::OverlayConfig::autostart_initialized` à `false`
//! ([`enable_by_default_once`]), après quoi seule la case de la fenêtre Options — ou le système
//! lui-même — change l'inscription. C'est la seule entorse à « rien dans `config.toml` », et elle
//! ne duplique pas l'état : elle dit seulement que le défaut a été posé.
//!
//! ## Ce qui est écrit, et où
//!
//! | Plateforme | Emplacement | Contenu |
//! | --- | --- | --- |
//! | Windows | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, valeur [`ENTRY_NAME`] | le chemin de l'exécutable courant, entre guillemets |
//! | Linux | `$XDG_CONFIG_HOME/autostart/wakfu-companion-overlay.desktop` (`~/.config/…` par défaut) | une entrée `Type=Application` vers l'exécutable courant |
//!
//! **`HKCU` et jamais `HKLM`** : l'overlay s'installe pour un utilisateur, et `HKLM` demanderait
//! une élévation que rien d'autre dans ce programme ne demande.
//!
//! **Le chemin écrit est celui de l'exécutable qui coche la case** (`std::env::current_exe`).
//! Déplacer l'overlay à la main après coup laisse donc une entrée morte : la remettre en place
//! demande de décocher puis recocher. Une mise à jour, elle, remplace le binaire **sur place**
//! (voir `docs/plan-mise-a-jour.md`) et ne casse rien.
//!
//! ## Best-effort, comme `config`
//!
//! Aucune de ces opérations n'est vitale : un échec (droits, disque plein, stratégie de groupe)
//! est journalisé et rien de plus — l'overlay démarre et fonctionne exactement pareil. La case
//! rouvrira sur l'état réel, c'est-à-dire sur l'échec, ce qui est la seule façon honnête de le
//! dire sans inventer une bannière d'erreur pour un réglage à une case.

/// Nom de l'entrée — la valeur sous `Run` (Windows), le nom de fichier `.desktop` (Linux, en
/// minuscules). Il sert d'identité : le changer perdrait de vue ce qu'une version précédente a
/// écrit.
pub const ENTRY_NAME: &str = "WakfuCompanionOverlay";

/// Ce que le gestionnaire de démarrage du bureau affiche (Linux) — le même libellé que le centre
/// de notifications Windows (`turn_watch::notify`).
const DISPLAY_NAME: &str = "Wakfu Companion Overlay";

/// L'overlay est-il inscrit au démarrage de la session ? Lit l'état RÉEL du système, jamais un
/// réglage mémorisé — voir la doc de module.
pub fn is_enabled() -> bool {
    imp::is_enabled()
}

/// **Pose le défaut « actif », une fois par installation** — voir la doc de module. Appelée par
/// les hôtes au démarrage, juste après `config::load` et avant toute fenêtre : si le jalon
/// `autostart_initialized` est levé, ne fait rien ; sinon inscrit l'overlay ([`apply`]), lève le
/// jalon et le sauvegarde. Un échec d'inscription (droits, stratégie de groupe) lève le jalon
/// quand même : le défaut a été proposé, la case de la fenêtre Options reste là pour réessayer —
/// le rejouer à chaque lancement ne ferait que remplir le journal du même refus.
pub fn enable_by_default_once(config: &mut crate::config::OverlayConfig) {
    if config.autostart_initialized {
        return;
    }
    tracing::info!(
        "[démarrage] premier lancement : inscription au démarrage de l'ordinateur par défaut."
    );
    apply(true);
    config.autostart_initialized = true;
    crate::config::save(config);
}

/// Inscrit ou retire l'overlay du démarrage de la session. Best-effort : n'écrit que si l'état
/// demandé diffère de l'état réel, journalise et renonce en cas d'échec.
pub fn apply(enabled: bool) {
    if imp::is_enabled() == enabled {
        return;
    }
    match imp::set_enabled(enabled) {
        Ok(()) => tracing::info!(
            "[démarrage] overlay {} au démarrage de l'ordinateur.",
            if enabled { "inscrit" } else { "retiré" }
        ),
        Err(err) => tracing::warn!(
            "[démarrage] inscription au démarrage de l'ordinateur impossible ({}) : {err}",
            if enabled {
                "activation"
            } else {
                "désactivation"
            }
        ),
    }
}

#[cfg(windows)]
mod imp {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    /// La clé de démarrage de l'utilisateur courant — voir la doc de module pour le choix de
    /// `HKCU`.
    const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Ouvre la clé `Run` en lecture/écriture. `RegCreateKeyExW` et non `RegOpenKeyExW` : la clé
    /// existe toujours, et c'est l'appel dont `turn_watch::notify` a déjà éprouvé la signature.
    fn open_run_key(access: windows::Win32::System::Registry::REG_SAM_FLAGS) -> Option<HKEY> {
        let subkey = wide(RUN_KEY);
        let mut key = HKEY::default();
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                access,
                None,
                &mut key,
                None,
            )
        };
        if status.is_err() {
            tracing::warn!("[démarrage] clé {RUN_KEY} inaccessible : {status:?}");
            return None;
        }
        Some(key)
    }

    pub(super) fn is_enabled() -> bool {
        let Some(key) = open_run_key(KEY_READ) else {
            return false;
        };
        let name = wide(super::ENTRY_NAME);
        // `lpData` et `lpcbData` nuls : on ne veut pas la valeur, seulement savoir si elle existe.
        let status =
            unsafe { RegQueryValueExW(key, PCWSTR(name.as_ptr()), None, None, None, None) };
        unsafe {
            let _ = RegCloseKey(key);
        }
        status.is_ok()
    }

    pub(super) fn set_enabled(enabled: bool) -> Result<(), String> {
        let Some(key) = open_run_key(KEY_READ | KEY_WRITE) else {
            return Err(format!("clé {RUN_KEY} inaccessible"));
        };
        let name = wide(super::ENTRY_NAME);
        let result = if enabled {
            match std::env::current_exe() {
                Ok(exe) => {
                    // Les guillemets sont ce qui distingue un chemin à espaces (« Program
                    // Files ») de plusieurs arguments, pour le shell qui lira cette valeur.
                    let value = wide(&format!("\"{}\"", exe.display()));
                    let bytes = unsafe {
                        std::slice::from_raw_parts(value.as_ptr() as *const u8, value.len() * 2)
                    };
                    let status = unsafe {
                        RegSetValueExW(key, PCWSTR(name.as_ptr()), None, REG_SZ, Some(bytes))
                    };
                    if status.is_err() {
                        Err(format!("écriture refusée : {status:?}"))
                    } else {
                        Ok(())
                    }
                }
                Err(err) => Err(format!("exécutable courant introuvable : {err}")),
            }
        } else {
            let status = unsafe { RegDeleteValueW(key, PCWSTR(name.as_ptr())) };
            if status.is_err() {
                Err(format!("suppression refusée : {status:?}"))
            } else {
                Ok(())
            }
        };
        unsafe {
            let _ = RegCloseKey(key);
        }
        result
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::{Path, PathBuf};

    /// Le dossier des entrées de démarrage de session, selon la spécification XDG
    /// « Desktop Application Autostart » : `$XDG_CONFIG_HOME/autostart`, `~/.config/autostart`
    /// à défaut. `directories::BaseDirs` applique déjà cette règle, et c'est la dépendance qui
    /// sert au reste de la config (voir `crate::config`).
    fn autostart_dir() -> Option<PathBuf> {
        directories::BaseDirs::new().map(|dirs| dirs.config_dir().join("autostart"))
    }

    pub(super) fn is_enabled() -> bool {
        autostart_dir().is_some_and(|dir| enabled_in(&dir))
    }

    pub(super) fn set_enabled(enabled: bool) -> Result<(), String> {
        let dir =
            autostart_dir().ok_or_else(|| "dossier de configuration introuvable".to_owned())?;
        let exe = std::env::current_exe()
            .map_err(|err| format!("exécutable courant introuvable : {err}"))?;
        set_enabled_in(&dir, &exe, enabled).map_err(|err| err.to_string())
    }

    /// Le fichier d'entrée dans `dir` — un seul, nommé d'après le dépôt.
    fn entry_file(dir: &Path) -> PathBuf {
        dir.join("wakfu-companion-overlay.desktop")
    }

    /// Même chose que [`is_enabled`], mais sur un dossier donné : c'est ce que les tests pilotent.
    fn enabled_in(dir: &Path) -> bool {
        entry_file(dir).is_file()
    }

    /// Même chose que [`set_enabled`], mais sur un dossier et un exécutable donnés.
    fn set_enabled_in(dir: &Path, exe: &Path, enabled: bool) -> std::io::Result<()> {
        let file = entry_file(dir);
        if enabled {
            std::fs::create_dir_all(dir)?;
            std::fs::write(&file, entry_contents(exe))
        } else {
            match std::fs::remove_file(&file) {
                // Déjà absent : c'est l'état demandé, pas une erreur.
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
                autre => autre,
            }
        }
    }

    /// Le contenu du fichier `.desktop`.
    ///
    /// `X-GNOME-Autostart-enabled` n'est pas standard mais GNOME le lit : sans lui, désactiver
    /// l'entrée depuis les réglages du bureau puis la réactiver depuis ici laisserait GNOME sur
    /// son `false` mémorisé. Les autres bureaux l'ignorent.
    ///
    /// **`Exec` entre guillemets** : la spécification les prévoit pour un chemin qui contient une
    /// espace, et un chemin d'exécutable sans espace les tolère sans rien y perdre.
    fn entry_contents(exe: &Path) -> String {
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Version=1.0\n\
             Name={name}\n\
             Comment=Overlay de jeu pour Wakfu\n\
             Exec=\"{exe}\"\n\
             Terminal=false\n\
             X-GNOME-Autostart-enabled=true\n",
            name = super::DISPLAY_NAME,
            exe = exe.display(),
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Un dossier temporaire à soi — pas de dépendance de test pour trois lignes, et le nom
        /// porte l'identifiant du processus pour que deux exécutions parallèles ne se marchent
        /// pas dessus.
        fn tmp_dir(nom: &str) -> PathBuf {
            let dir = std::env::temp_dir()
                .join(format!("overlay-autostart-{nom}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            dir
        }

        /// Le cycle complet, tel que « Valider » l'enchaîne : rien, puis inscrit, puis retiré.
        #[test]
        fn le_fichier_d_entree_apparait_et_disparait_avec_le_reglage() {
            let dir = tmp_dir("cycle");
            let exe = Path::new("/opt/wakfu companion/overlay-ui-x11");
            assert!(!enabled_in(&dir), "rien n'est inscrit au départ");

            set_enabled_in(&dir, exe, true).expect("inscription");
            assert!(enabled_in(&dir), "le fichier .desktop doit être là");

            set_enabled_in(&dir, exe, false).expect("retrait");
            assert!(!enabled_in(&dir), "le fichier .desktop doit avoir disparu");

            // Retirer deux fois de suite n'est pas une erreur : l'utilisateur a pu supprimer
            // l'entrée depuis les réglages de son bureau entre-temps.
            set_enabled_in(&dir, exe, false).expect("retrait idempotent");
            let _ = std::fs::remove_dir_all(&dir);
        }

        /// Ce que le bureau lira — et surtout le chemin à espaces entre guillemets, sans quoi
        /// `Exec` serait compris comme une commande suivie d'un argument.
        #[test]
        fn l_entree_pointe_l_executable_entre_guillemets() {
            let contenu = entry_contents(Path::new("/opt/wakfu companion/overlay-ui-x11"));
            assert!(
                contenu.contains("Exec=\"/opt/wakfu companion/overlay-ui-x11\"\n"),
                "Exec mal formé :\n{contenu}"
            );
            assert!(contenu.starts_with("[Desktop Entry]\n"));
            assert!(contenu.contains("Type=Application\n"));
        }
    }
}
