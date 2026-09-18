//! **La racine unique des dossiers de l'overlay** — le seul endroit du dépôt qui appelle
//! `directories::ProjectDirs::from` (un test ci-dessous y veille).
//!
//! Jusqu'au 2026-09-19, deux triplets de qualifieurs coexistaient (constat C13 de
//! `docs/analyse-rgpd.md`) : `("", "", "wakfu-companion-overlay")` pour les journaux, les données
//! d'`overlay-engine` et tout `overlay-sync`, et `("com", "Oumbra", "wakfu-companion-overlay")` pour
//! `config.toml` et les gabarits de tour. Sous Linux les deux retombent sur le même dossier XDG ;
//! sous **Windows non** — `%APPDATA%\wakfu-companion-overlay\…` d'un côté,
//! `%APPDATA%\Oumbra\wakfu-companion-overlay\…` de l'autre —, et l'overlay laissait donc deux
//! dossiers derrière lui, que l'effacement complet devait connaître tous les deux. Décision du
//! mainteneur (2026-09-19) : **la racine sans qualifieur**, `wakfu-companion-overlay` seul ; l'autre
//! n'existe plus que pour être migrée puis supprimée au démarrage ([`legacy_project_dirs`],
//! `overlay_ui::config::migrate_legacy_root`).
//!
//! Chaque crate garde son propre `APP_NAME` avec le suffixe `-test` sous `cfg(test)` — `cfg(test)`
//! est par crate, une constante partagée ici ne le verrait pas depuis les tests des autres : un
//! test qui écrit ou efface ne doit jamais atteindre le dossier réel de la personne qui lance la
//! suite (bug vécu le 2026-09-01, `overlay_sync::token_store`).

use directories::ProjectDirs;

/// La racine de l'overlay pour `app_name` : `%APPDATA%\<app_name>\{config,data}` sous Windows,
/// `~/.config/<app_name>` et `~/.local/share/<app_name>` sous Linux.
pub fn project_dirs(app_name: &str) -> Option<ProjectDirs> {
    ProjectDirs::from("", "", app_name)
}

/// L'ancienne racine de `config.toml` et des gabarits de tour (`("com", "Oumbra", …)`, jusqu'au
/// 2026-09-19) — **uniquement** pour la migrer et l'effacer ; rien ne doit plus y écrire. Sous
/// Linux elle se confond avec [`project_dirs`], et il n'y a rien à migrer.
pub fn legacy_project_dirs(app_name: &str) -> Option<ProjectDirs> {
    ProjectDirs::from("com", "Oumbra", app_name)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// Personne d'autre n'appelle `ProjectDirs::from` : c'est ainsi que C13 ne se reproduit pas.
    /// Parcourt `crates/*/src/**/*.rs` depuis le manifeste de cette crate.
    #[test]
    fn un_seul_endroit_construit_les_racines() {
        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut offenders = Vec::new();
        for entry in std::fs::read_dir(&crates).unwrap() {
            let src = entry.unwrap().path().join("src");
            if src.is_dir() {
                walk(&src, &mut offenders);
            }
        }
        assert!(
            offenders.is_empty(),
            "`ProjectDirs::from` hors de `overlay_engine::app_dirs` : {offenders:?}"
        );
    }

    fn walk(dir: &Path, offenders: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, offenders);
            } else if path.extension().is_some_and(|e| e == "rs")
                && path.file_name().is_some_and(|n| n != "app_dirs.rs")
            {
                let text = std::fs::read_to_string(&path).unwrap();
                if text.contains("ProjectDirs::from(") {
                    offenders.push(path.display().to_string());
                }
            }
        }
    }
}
