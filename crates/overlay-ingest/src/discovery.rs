//! Découverte du chemin de `wakfu.log` — voir docs/plan-architecture.md §5.1.
//!
//! Seul le premier chemin Windows est **vérifié** — sur une installation réelle (2026-08-31), pas
//! seulement déduit d'une ligne de log. Tous les autres sont des hypothèses documentées comme
//! telles dans le plan — l'appelant ne doit jamais traiter une découverte manquée comme une erreur
//! fatale : c'est le rôle du sélecteur de fichier manuel côté UI (hors périmètre de ce crate).

use std::path::Path;
use std::path::PathBuf;

/// Chemins candidats, dans l'ordre de préférence du plan. Ne vérifie rien sur le disque — voir
/// [`discover`] pour la résolution effective.
pub fn candidate_paths() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        windows_candidates()
    }
    #[cfg(not(windows))]
    {
        unix_candidates()
    }
}

/// Premier chemin candidat qui existe réellement sur le disque, ou `None` si aucun ne convient.
pub fn discover() -> Option<PathBuf> {
    candidate_paths().into_iter().find(|p| p.is_file())
}

#[cfg(windows)]
fn windows_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    // 1. Vérifié sur machine réelle (2026-08-31) : le plan v2 initial (§1) citait
    // `%APPDATA%\zaap\gamesLogs\wakfu\wakfu.log` sans le sous-dossier `logs\`, déduit à tort d'une
    // ligne de log mentionnant un *autre* fichier (`.../wakfu/config`) — corrigé ici après
    // constat direct sur une installation réelle du client.
    if let Some(appdata) = std::env::var_os("APPDATA") {
        out.push(PathBuf::from(appdata).join("zaap/gamesLogs/wakfu/logs/wakfu.log"));
    }
    // 2. Hypothèse non confirmée (installation via un autre lanceur Ankama) — sous-dossier
    // `logs\` aligné sur le n°1 par cohérence, mais pas vérifié pour cet emplacement précis.
    if let Some(localappdata) = std::env::var_os("LOCALAPPDATA") {
        out.push(PathBuf::from(localappdata).join("Ankama/zaap/gamesLogs/wakfu/logs/wakfu.log"));
    }
    out
}

#[cfg(not(windows))]
fn unix_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let home = std::env::var_os("HOME").map(PathBuf::from);

    // 1. Zaap natif Linux — hypothèse non confirmée (§5.1 : « à confirmer sur machine réelle »).
    if let Some(xdg_config) = std::env::var_os("XDG_CONFIG_HOME") {
        out.push(PathBuf::from(xdg_config).join("zaap/gamesLogs/wakfu/wakfu.log"));
    }
    if let Some(home) = &home {
        out.push(home.join(".config/zaap/gamesLogs/wakfu/wakfu.log"));
    }

    if let Some(home) = &home {
        // 2. Steam/Proton : le préfixe est un AppID numérique imprévisible à l'avance — on énumère
        // les répertoires réellement présents plutôt que de deviner un identifiant.
        extend_with_prefix_children(
            &mut out,
            &home.join(".steam/steam/steamapps/compatdata"),
            "pfx/drive_c/users/steamuser/AppData/Roaming/zaap/gamesLogs/wakfu/wakfu.log",
        );

        // 3. Wine générique : même logique, le nom d'utilisateur simulé n'est pas prévisible.
        extend_with_prefix_children(
            &mut out,
            &home.join(".wine/drive_c/users"),
            "AppData/Roaming/zaap/gamesLogs/wakfu/wakfu.log",
        );
    }
    out
}

/// Ajoute `<enfant>/<suffix>` pour chaque sous-répertoire de `parent` — sert à couvrir les
/// préfixes Steam/Wine dont le nom exact n'est pas connu à l'avance. Silencieux si `parent`
/// n'existe pas (cas très courant : la plupart des machines n'ont ni Steam ni Wine installés).
#[cfg(not(windows))]
fn extend_with_prefix_children(out: &mut Vec<PathBuf>, parent: &Path, suffix: &str) {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            out.push(entry.path().join(suffix));
        }
    }
}

/// Nom de fichier attendu pour le journal du client Wakfu (§5.1 du plan) — garde-fou du sélecteur
/// de fichier manuel de l'UI (`overlay-ui::panels::options_modal`) : l'utilisateur doit pouvoir
/// pointer vers N'IMPORTE QUEL dossier (installation ailleurs que l'emplacement par défaut, bêta,
/// jeu de test…) mais jamais vers un fichier qui ne soit pas RÉELLEMENT `wakfu.log` — sélectionner
/// un fichier quelconque casserait silencieusement le parsing (§1 du plan : motifs en français
/// attendus dans un format de log précis).
pub const LOG_FILE_NAME: &str = "wakfu.log";

/// `true` si `path` se termine par [`LOG_FILE_NAME`] — comparaison insensible à la casse
/// (Windows ne distingue pas la casse des noms de fichiers ; rien ne coûte à rester tolérant
/// ailleurs aussi plutôt que de piéger un utilisateur dont l'explorateur affiche `Wakfu.log`).
pub fn is_valid_log_filename(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(LOG_FILE_NAME))
}

/// Erreur de validation d'un chemin choisi manuellement par l'utilisateur (voir
/// [`is_valid_log_filename`]) — message déjà en français, directement affichable tel quel dans la
/// modale Options (`overlay-ui::panels::options_modal`), pas seulement destiné aux journaux.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogPathError {
    /// Le fichier ne s'appelle pas `wakfu.log` (n'importe quelle casse).
    WrongFileName,
    /// Le nom convient, mais rien n'existe à ce chemin (faute de frappe, fichier déplacé entre la
    /// saisie et la validation…).
    NotFound,
}

impl LogPathError {
    pub fn message(&self) -> &'static str {
        match self {
            LogPathError::WrongFileName => "Le fichier sélectionné doit s'appeler wakfu.log.",
            LogPathError::NotFound => "Fichier introuvable à ce chemin.",
        }
    }
}

/// Valide un chemin choisi manuellement (champ texte ou explorateur de fichiers de l'UI) avant de
/// le retenir comme nouveau `wakfu.log` à suivre — double garde : bon nom de fichier ET fichier
/// réellement présent sur le disque. Utilisée aussi bien après un `rfd::FileDialog::pick_file`
/// (dont le filtre d'extension seul ne garantit pas le nom exact) qu'après une saisie manuelle du
/// champ texte.
pub fn validate_log_path(path: &Path) -> Result<(), LogPathError> {
    if !is_valid_log_filename(path) {
        return Err(LogPathError::WrongFileName);
    }
    if !path.is_file() {
        return Err(LogPathError::NotFound);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepte_wakfu_log_quelle_que_soit_la_casse() {
        // Chemins au format de la plateforme de COMPILATION du test (`std::path::Path` interprète
        // les séparateurs selon `cfg(windows)`, pas selon un OS cible arbitraire) — un chemin
        // Windows littéral (`C:\...`) analysé ici sous Unix ne serait qu'un seul composant sans
        // séparateur reconnu, ce n'est pas ce que ce test veut couvrir.
        assert!(is_valid_log_filename(
            &PathBuf::from("beta").join("wakfu.log")
        ));
        assert!(is_valid_log_filename(
            &PathBuf::from("beta").join("Wakfu.LOG")
        ));
        assert!(is_valid_log_filename(Path::new("WAKFU.LOG")));
    }

    #[test]
    fn refuse_un_autre_nom() {
        assert!(!is_valid_log_filename(Path::new("/tmp/wakfu.log.0")));
        assert!(!is_valid_log_filename(Path::new("/tmp/notes.txt")));
        assert!(!is_valid_log_filename(Path::new("/tmp/")));
    }

    #[test]
    fn validate_refuse_mauvais_nom_avant_meme_de_toucher_le_disque() {
        assert_eq!(
            validate_log_path(Path::new("/chemin/inexistant/notes.txt")),
            Err(LogPathError::WrongFileName)
        );
    }

    #[test]
    fn validate_refuse_fichier_absent() {
        let dir = std::env::temp_dir().join(format!(
            "wakfu-overlay-test-{}-{}",
            std::process::id(),
            line!()
        ));
        let missing = dir.join("wakfu.log");
        assert_eq!(validate_log_path(&missing), Err(LogPathError::NotFound));
    }

    #[test]
    fn validate_accepte_un_wakfu_log_reel() {
        let dir = std::env::temp_dir().join(format!(
            "wakfu-overlay-test-ok-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("wakfu.log");
        std::fs::write(&path, "contenu").unwrap();
        assert_eq!(validate_log_path(&path), Ok(()));
        let cursor = std::fs::remove_dir_all(&dir);
        let _ = cursor;
    }
}
