//! Persistance de la configuration utilisateur de l'overlay — §5.1 du plan d'architecture : le
//! chemin de `wakfu.log` doit être « toujours surchargeable par la config et par un sélecteur de
//! fichier dans l'UI ». Seul ce champ est persisté pour l'instant (voir [`OverlayConfig`]) : un
//! seul fichier TOML pour toute future extension plutôt qu'un fichier par réglage.
//!
//! **Priorité de résolution du chemin au démarrage** (voir `resolve_log_path`, partagé par
//! `main.rs` et `bin/overlay-ui-x11.rs`) : argument CLI explicite > chemin sauvegardé ici (choisi
//! via la modale Options, `panels::options_modal`) > découverte automatique
//! (`overlay_ingest::discovery::discover`).
//!
//! Emplacement du fichier : `directories::ProjectDirs` (déjà une dépendance de ce crate, utilisée
//! par ailleurs pour le repli du jeton de compte natif — voir §7.2 du plan) —
//! `%APPDATA%\Oumbra\wakfu-companion-overlay\config\config.toml` sous Windows,
//! `$XDG_CONFIG_HOME/wakfu-companion-overlay/config.toml` (ou `~/.config/...`) sous Linux.
//! Chargement/sauvegarde **best-effort** : un échec (droits insuffisants, disque plein, fichier
//! corrompu) n'est jamais fatal, seulement journalisé — un utilisateur dans ce cas retombe
//! simplement sur la découverte automatique à chaque lancement, comme avant l'existence de ce
//! module.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    /// Chemin explicite de `wakfu.log`, choisi par l'utilisateur via la modale Options — voir
    /// doc de module pour l'ordre de priorité au démarrage et
    /// `engine_thread::EngineCommand::ChangeLogPath` pour le rechargement à chaud (sans
    /// redémarrer l'overlay) quand ce réglage change en cours de session.
    pub log_path: Option<PathBuf>,
}

fn project_dirs() -> Option<directories::ProjectDirs> {
    // Mêmes qualifieurs que le reste du dépôt (organisation GitHub `Oumbra`, voir
    // `overlay_sync::token_store` pour le même motif appliqué au jeton de compte natif).
    directories::ProjectDirs::from("com", "Oumbra", "wakfu-companion-overlay")
}

fn config_file() -> Option<PathBuf> {
    project_dirs().map(|dirs| dirs.config_dir().join("config.toml"))
}

/// Charge la config persistée — `OverlayConfig::default()` (donc `log_path: None`) au tout premier
/// lancement (fichier absent) ou si sa lecture/son parsing échoue : jamais fatal, voir doc de
/// module.
pub fn load() -> OverlayConfig {
    let Some(path) = config_file() else {
        tracing::warn!(
            "[config] répertoire de configuration introuvable — réglages non persistés cette session."
        );
        return OverlayConfig::default();
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return OverlayConfig::default(),
        Err(err) => {
            tracing::warn!(
                "[config] échec de lecture de {} ({err}) — réglages par défaut.",
                path.display()
            );
            return OverlayConfig::default();
        }
    };
    match toml::from_str(&raw) {
        Ok(config) => config,
        Err(err) => {
            tracing::warn!(
                "[config] échec de lecture de {} ({err}) — réglages par défaut.",
                path.display()
            );
            OverlayConfig::default()
        }
    }
}

/// Sauvegarde `config` sur disque — best-effort (voir doc de module), crée le répertoire parent
/// si besoin. Appelée UNIQUEMENT à la validation de la modale Options (jamais à chaque frame) :
/// voir `panels::options_modal::OptionsModalAction::Validate`.
pub fn save(config: &OverlayConfig) {
    let Some(path) = config_file() else {
        tracing::warn!(
            "[config] répertoire de configuration introuvable — réglages non sauvegardés."
        );
        return;
    };
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::warn!(
                "[config] échec de création de {} ({err}) — réglages non sauvegardés.",
                parent.display()
            );
            return;
        }
    }
    let raw = match toml::to_string_pretty(config) {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[config] échec de sérialisation ({err}) — réglages non sauvegardés.");
            return;
        }
    };
    if let Err(err) = std::fs::write(&path, raw) {
        tracing::warn!(
            "[config] échec d'écriture de {} ({err}) — réglages non sauvegardés.",
            path.display()
        );
    }
}

/// Résout le chemin de `wakfu.log` à utiliser au démarrage — voir doc de module pour l'ordre de
/// priorité. Partagée par `main.rs` (Windows) et `bin/overlay-ui-x11.rs` (Linux), qui appellent
/// chacun `env::args().nth(1)` pour l'argument CLI (rien d'OS-spécifique là-dedans, mais
/// `std::env::args` reste appelé au point d'entrée de chaque binaire plutôt qu'ici, pour ne pas
/// faire dépendre ce module de la façon dont chaque binaire construit ses arguments).
///
/// L'échec de toute résolution (aucun argument, rien en config, découverte automatique
/// infructueuse) n'est PAS traité ici — voir la doc de `overlay_ingest::discovery::discover` :
/// c'est à l'appelant de décider (message d'erreur + sortie côté CLI headless historique, futur
/// sélecteur de fichier de la modale Options côté UI).
pub fn resolve_log_path(cli_arg: Option<PathBuf>, config: &OverlayConfig) -> Option<PathBuf> {
    cli_arg
        .or_else(|| config.log_path.clone())
        .or_else(overlay_ingest::discovery::discover)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_priorise_argument_cli() {
        let config = OverlayConfig {
            log_path: Some(PathBuf::from("/config/wakfu.log")),
        };
        let resolved = resolve_log_path(Some(PathBuf::from("/cli/wakfu.log")), &config);
        assert_eq!(resolved, Some(PathBuf::from("/cli/wakfu.log")));
    }

    #[test]
    fn resolve_retombe_sur_la_config_sans_argument_cli() {
        let config = OverlayConfig {
            log_path: Some(PathBuf::from("/config/wakfu.log")),
        };
        let resolved = resolve_log_path(None, &config);
        assert_eq!(resolved, Some(PathBuf::from("/config/wakfu.log")));
    }
}
