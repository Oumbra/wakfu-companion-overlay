//! Découverte du chemin de `wakfu.log` — voir docs/plan-architecture.md §5.1.
//!
//! Seul le premier chemin Windows est **vérifié** (présent dans `tests/wakfu.log` du dépôt web).
//! Tous les autres sont des hypothèses documentées comme telles dans le plan — l'appelant ne doit
//! jamais traiter une découverte manquée comme une erreur fatale : c'est le rôle du sélecteur de
//! fichier manuel côté UI (hors périmètre de ce crate).

#[cfg(not(windows))]
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
    // 1. Vérifié sur machine réelle (tests/wakfu.log du dépôt web).
    if let Some(appdata) = std::env::var_os("APPDATA") {
        out.push(PathBuf::from(appdata).join("zaap/gamesLogs/wakfu/wakfu.log"));
    }
    // 2. Hypothèse non confirmée (installation via un autre lanceur Ankama).
    if let Some(localappdata) = std::env::var_os("LOCALAPPDATA") {
        out.push(PathBuf::from(localappdata).join("Ankama/zaap/gamesLogs/wakfu/wakfu.log"));
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
