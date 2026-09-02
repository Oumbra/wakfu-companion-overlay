//! Décalage manuel persisté par panneau ET par écran (lot L2, §6.4/§9 du plan : « ancrage de
//! l'overlay par écran + décalage, persistés par identifiant d'écran ») — chaque panneau reste
//! ancré automatiquement sur SA fenêtre de jeu (voir `main.rs::App::anchor_position`), mais
//! l'utilisateur peut affiner cet ancrage en le faisant glisser (poignée « ⠿ », voir `main.rs::
//! render`) ; c'est ce DÉCALAGE qui est mémorisé ici, jamais une position absolue (qui n'aurait
//! aucun sens dès que la fenêtre de jeu bouge à son tour) — voir `main.rs::OverlayWindow::
//! manual_offset`.
//!
//! Stockage disque plat (`serde_json`), même convention que `overlay_engine::watchlist` /
//! `overlay_sync::token_store` (`directories::ProjectDirs`) : un seul petit fichier, jamais de
//! base de données pour un besoin aussi simple. Clé `"<écran>:<panneau>"` plutôt que deux niveaux
//! imbriqués — suffisant pour ce volume (au plus quelques écrans × 2 panneaux) et trivial à
//! inspecter/éditer à la main en cas de besoin.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::OverlayKind;

// Nom d'app DISTINCT en test (même motif que `overlay_sync::token_store`, précaution tirée d'un
// bug réel trouvé en session sur ce dernier, 2026-09-01) : sans ça, `cargo test` écrirait/effacerait
// directement `layout.json` du VRAI dossier de données de l'utilisateur qui lance les tests sur sa
// propre machine — jamais de recouvrement possible entre l'app réelle et ses propres tests.
#[cfg(not(test))]
const APP_NAME: &str = "wakfu-companion-overlay";
#[cfg(test)]
const APP_NAME: &str = "wakfu-companion-overlay-test";

/// Décalage (pixels physiques) appliqué EN PLUS de l'ancrage automatique — voir
/// `main.rs::App::anchor_position`. `(0, 0)` (`Default`) : aucun réglage manuel, comportement
/// identique à avant ce lot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PanelOffset {
    pub dx: i32,
    pub dy: i32,
}

impl OverlayKind {
    fn storage_key(self) -> &'static str {
        match self {
            OverlayKind::Combat => "combat",
            OverlayKind::Watchlist => "watchlist",
        }
    }
}

fn store_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", APP_NAME).map(|dirs| dirs.data_dir().join("layout.json"))
}

fn key(screen_id: &str, kind: OverlayKind) -> String {
    format!("{screen_id}:{}", kind.storage_key())
}

/// Fichier absent/illisible/vide → carte vide, jamais une erreur : un décalage jamais enregistré
/// n'est pas différent d'un décalage nul (voir `load_offset`).
fn load_all(path: &Path) -> HashMap<String, PanelOffset> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// `PanelOffset::default()` si rien n'a jamais été enregistré pour cet écran+panneau, ou si le
/// dossier de données/le fichier est indisponible — jamais une erreur (confort de mise en page,
/// pas une condition de démarrage).
pub fn load_offset(screen_id: &str, kind: OverlayKind) -> PanelOffset {
    let Some(path) = store_path() else {
        return PanelOffset::default();
    };
    load_all(&path)
        .get(&key(screen_id, kind))
        .copied()
        .unwrap_or_default()
}

/// Enregistre le décalage de CET écran+panneau — lecture-modification-écriture de tout le fichier
/// (petit, peu fréquent : seulement à la fin d'un glissement, voir `main.rs`), jamais un verrou ni
/// une base pour un besoin aussi simple. Best-effort : une écriture échouée n'est qu'un
/// `tracing::warn!`, jamais fatale (le décalage reste appliqué en mémoire pour cette session, il
/// sera juste redemandé au prochain lancement).
pub fn save_offset(screen_id: &str, kind: OverlayKind, offset: PanelOffset) {
    let Some(path) = store_path() else {
        tracing::warn!("dossier de données introuvable — décalage de panneau non persisté");
        return;
    };
    let mut all = load_all(&path);
    all.insert(key(screen_id, kind), offset);
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::warn!("création du dossier de disposition impossible ({err})");
            return;
        }
    }
    match serde_json::to_string_pretty(&all) {
        Ok(json) => {
            if let Err(err) = std::fs::write(&path, json) {
                tracing::warn!("écriture de la disposition impossible ({err})");
            }
        }
        Err(err) => tracing::warn!("sérialisation de la disposition impossible ({err})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Les deux tests partagent le même fichier (`APP_NAME` distinct de la vraie app, voir
    // ci-dessus) — un `Mutex` les sérialise pour qu'ils ne s'écrasent pas l'un l'autre (déjà
    // single-thread par défaut côté `cargo test`, mais explicite plutôt que de compter sur ce
    // détail implicite).
    static GUARD: Mutex<()> = Mutex::new(());

    fn clean_slate() -> PathBuf {
        let path = store_path().expect("dossier de données résolvable en test");
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn absence_denregistrement_renvoie_un_decalage_nul() {
        let _lock = GUARD.lock().unwrap();
        clean_slate();
        assert_eq!(
            load_offset("écran-de-test-absent", OverlayKind::Combat),
            PanelOffset::default()
        );
    }

    #[test]
    fn sauvegarde_puis_lecture_coherentes_par_ecran_et_par_panneau() {
        let _lock = GUARD.lock().unwrap();
        clean_slate();

        let screen = "écran-de-test-layout";
        save_offset(screen, OverlayKind::Combat, PanelOffset { dx: 12, dy: -7 });
        save_offset(
            screen,
            OverlayKind::Watchlist,
            PanelOffset { dx: -3, dy: 40 },
        );

        assert_eq!(
            load_offset(screen, OverlayKind::Combat),
            PanelOffset { dx: 12, dy: -7 }
        );
        assert_eq!(
            load_offset(screen, OverlayKind::Watchlist),
            PanelOffset { dx: -3, dy: 40 }
        );
        // Un écran différent ne doit jamais lire le décalage d'un autre — sinon deux moniteurs de
        // résolution différente hériteraient l'un du réglage de l'autre.
        assert_eq!(
            load_offset("écran-de-test-layout-2", OverlayKind::Combat),
            PanelOffset::default()
        );
    }
}
