//! **Avancement du démarrage** (2026-09-14, §9.1 undecies du plan) — ce que la fenêtre de
//! connexion attend derrière son écran de chargement avant de montrer quoi que ce soit d'autre.
//!
//! Demande utilisateur : « cette fenêtre est là pour temporiser l'utilisateur lors du démarrage
//! de l'overlay, mais aussi pour la vérification du jeton et la récupération des informations si
//! un jeton est trouvé [...] tout ça doit être masqué par cette fenêtre avant d'afficher "vous
//! n'êtes pas connecté" ». Quatre chargements de fond y contribuent, un drapeau chacun, posé par
//! le thread concerné quand son travail INITIAL est fini (succès ou repli, peu importe) :
//!
//! - **catalogue** (`background::spawn_catalog_thread`) — cache disque, réseau ou repli embarqué ;
//! - **référentiel de donjons** (`background::spawn_dungeon_thread`) — même logique ;
//! - **rattrapage de `wakfu.log`** (`engine_thread::spawn_engine_thread`) — la relecture complète
//!   du fichier par le moteur, jusqu'au premier silence du watcher ;
//! - **vérification de mise à jour** (`background::spawn_update_thread`, 2026-09-15,
//!   `docs/plan-mise-a-jour.md` §8.1) — le manifeste de la dernière Release lu, ou déclaré
//!   injoignable (cinq secondes au plus) : dans les deux cas l'étape est *résolue*.
//!
//! Le compte, lui, n'a pas de drapeau ici : c'est `AuthStatus::Connecting` qui dit qu'il est en
//! cours (validation du jeton stocké, puis `GET /api/v1/settings`), et l'hôte le combine à
//! [`StartupProgress::is_complete`] pour décider de l'écran (voir `sync_session_windows` dans
//! chaque binaire).
//!
//! **Garde-fou** : au-delà de [`STARTUP_TIMEOUT`], l'écran de chargement tombe quoi qu'il en soit
//! — un réseau qui ne répond pas fait déjà attendre les délais d'`ureq`, mais un thread qui ne
//! marquerait jamais sa fin (bug) ne doit pas condamner l'utilisateur à un rouage éternel.
//!
//! **Une exception au garde-fou : une mise à jour en cours** ([`StartupProgress::set_update_blocking`]).
//! Un téléchargement de quinze mégaoctets sur une connexion lente dépasse quarante-cinq secondes
//! sans que rien ne soit cassé ; le couper à mi-course pour ouvrir les overlays reviendrait à
//! installer la mise à jour au milieu d'une session — ce que le plan interdit. Tant que ce
//! drapeau est levé, l'écran reste, quelle que soit l'horloge ; le thread de mise à jour a son
//! propre plafond (délais de réception par bloc, voir `overlay_sync::update::download`). Le même
//! drapeau sert à REVENIR sur l'écran de chargement depuis la fenêtre Options (« Mettre à jour »)
//! une fois la session ouverte.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Voir la doc de module.
pub const STARTUP_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug)]
pub struct StartupProgress {
    catalog: AtomicBool,
    dungeons: AtomicBool,
    log_replayed: AtomicBool,
    update_resolved: AtomicBool,
    update_blocking: AtomicBool,
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
            update_resolved: AtomicBool::new(false),
            update_blocking: AtomicBool::new(false),
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

    /// La vérification de mise à jour a rendu son verdict (à jour, disponible sans installation
    /// automatique, ou injoignable) : rien ne retient plus le démarrage de ce côté-là.
    pub fn mark_update_resolved(&self) {
        self.update_resolved.store(true, Ordering::Release);
    }

    /// Une mise à jour se télécharge ou s'installe (`true`), ou ne le fait plus (`false`) — voir la
    /// doc de module : tant que c'est levé, l'écran de chargement reste, garde-fou compris.
    pub fn set_update_blocking(&self, blocking: bool) {
        self.update_blocking.store(blocking, Ordering::Release);
    }

    pub fn is_update_blocking(&self) -> bool {
        self.update_blocking.load(Ordering::Acquire)
    }

    /// Tout est chargé — ou le délai de garde est dépassé — et aucune mise à jour n'est en cours.
    pub fn is_complete(&self) -> bool {
        if self.is_update_blocking() {
            return false;
        }
        (self.catalog.load(Ordering::Acquire)
            && self.dungeons.load(Ordering::Acquire)
            && self.log_replayed.load(Ordering::Acquire)
            && self.update_resolved.load(Ordering::Acquire))
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
        if !self.update_resolved.load(Ordering::Acquire) {
            pending.push("vérification de mise à jour");
        }
        if self.is_update_blocking() {
            pending.push("mise à jour en cours");
        }
        pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complet_seulement_quand_les_quatre_drapeaux_sont_poses() {
        let progress = StartupProgress::new();
        assert!(!progress.is_complete());
        progress.mark_catalog();
        progress.mark_dungeons();
        assert!(!progress.is_complete());
        assert_eq!(
            progress.pending(),
            vec!["rattrapage de wakfu.log", "vérification de mise à jour"]
        );
        progress.mark_log_replayed();
        assert!(!progress.is_complete());
        progress.mark_update_resolved();
        assert!(progress.is_complete());
        assert!(progress.pending().is_empty());
    }

    #[test]
    fn une_mise_a_jour_en_cours_retient_l_ecran_meme_complet() {
        let progress = StartupProgress::new();
        progress.mark_catalog();
        progress.mark_dungeons();
        progress.mark_log_replayed();
        progress.mark_update_resolved();
        assert!(progress.is_complete());
        progress.set_update_blocking(true);
        assert!(!progress.is_complete());
        assert_eq!(progress.pending(), vec!["mise à jour en cours"]);
        progress.set_update_blocking(false);
        assert!(progress.is_complete());
    }
}
