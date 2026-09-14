//! **Avancement du démarrage** (2026-09-14, §9.1 undecies du plan) — ce que la fenêtre de
//! connexion attend derrière son écran de chargement avant de montrer quoi que ce soit d'autre.
//!
//! Demande utilisateur : « cette fenêtre est là pour temporiser l'utilisateur lors du démarrage
//! de l'overlay, mais aussi pour la vérification du jeton et la récupération des informations si
//! un jeton est trouvé [...] tout ça doit être masqué par cette fenêtre avant d'afficher "vous
//! n'êtes pas connecté" ». Trois chargements de fond y contribuent, un drapeau chacun, posé par le
//! thread concerné quand son travail INITIAL est fini (succès ou repli, peu importe) :
//!
//! - **catalogue** (`background::spawn_catalog_thread`) — cache disque, réseau ou repli embarqué ;
//! - **référentiel de donjons** (`background::spawn_dungeon_thread`) — même logique ;
//! - **rattrapage de `wakfu.log`** (`engine_thread::spawn_engine_thread`) — la relecture complète
//!   du fichier par le moteur, jusqu'au premier silence du watcher.
//!
//! Le compte, lui, n'a pas de drapeau ici : c'est `AuthStatus::Connecting` qui dit qu'il est en
//! cours (validation du jeton stocké, puis `GET /api/v1/settings`), et l'hôte le combine à
//! [`StartupProgress::is_complete`] pour décider de l'écran (voir `sync_session_windows` dans
//! chaque binaire).
//!
//! **Garde-fou** : au-delà de [`STARTUP_TIMEOUT`], l'écran de chargement tombe quoi qu'il en soit
//! — un réseau qui ne répond pas fait déjà attendre les délais d'`ureq`, mais un thread qui ne
//! marquerait jamais sa fin (bug) ne doit pas condamner l'utilisateur à un rouage éternel.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Voir la doc de module.
pub const STARTUP_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug)]
pub struct StartupProgress {
    catalog: AtomicBool,
    dungeons: AtomicBool,
    log_replayed: AtomicBool,
    started_at: Instant,
}

impl Default for StartupProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl StartupProgress {
    pub fn new() -> Self {
        Self {
            catalog: AtomicBool::new(false),
            dungeons: AtomicBool::new(false),
            log_replayed: AtomicBool::new(false),
            started_at: Instant::now(),
        }
    }

    pub fn mark_catalog(&self) {
        self.catalog.store(true, Ordering::Release);
    }

    pub fn mark_dungeons(&self) {
        self.dungeons.store(true, Ordering::Release);
    }

    pub fn mark_log_replayed(&self) {
        self.log_replayed.store(true, Ordering::Release);
    }

    /// Tout est chargé — ou le délai de garde est dépassé.
    pub fn is_complete(&self) -> bool {
        (self.catalog.load(Ordering::Acquire)
            && self.dungeons.load(Ordering::Acquire)
            && self.log_replayed.load(Ordering::Acquire))
            || self.started_at.elapsed() >= STARTUP_TIMEOUT
    }

    /// Ce qui manque encore, pour le journal — vide une fois complet.
    pub fn pending(&self) -> Vec<&'static str> {
        let mut pending = Vec::new();
        if !self.catalog.load(Ordering::Acquire) {
            pending.push("catalogue");
        }
        if !self.dungeons.load(Ordering::Acquire) {
            pending.push("donjons");
        }
        if !self.log_replayed.load(Ordering::Acquire) {
            pending.push("rattrapage de wakfu.log");
        }
        pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complet_seulement_quand_les_trois_drapeaux_sont_poses() {
        let progress = StartupProgress::new();
        assert!(!progress.is_complete());
        progress.mark_catalog();
        progress.mark_dungeons();
        assert!(!progress.is_complete());
        assert_eq!(progress.pending(), vec!["rattrapage de wakfu.log"]);
        progress.mark_log_replayed();
        assert!(progress.is_complete());
        assert!(progress.pending().is_empty());
    }
}
