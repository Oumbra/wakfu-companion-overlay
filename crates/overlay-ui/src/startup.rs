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
//!
//! **Plancher d'affichage** ([`MIN_DISPLAY`], demande utilisateur du 2026-09-15). Quand tout est
//! déjà en cache et que le jeton répond du premier coup, les quatre drapeaux tombent en quelques
//! centaines de millisecondes : la fenêtre de connexion s'ouvre, montre son rouage le temps d'une
//! poignée d'images et disparaît — un **clignotement**, pas un écran de chargement. L'écran reste
//! donc affiché au moins cinq secondes, quoi qu'il arrive : `is_complete` ment tant que ce délai
//! n'est pas écoulé, et l'hôte, qui ne connaît que lui, garde la fenêtre de connexion.
//!
//! Le plancher court depuis l'**apparition** de l'écran, pas depuis le lancement : il repart
//! quand [`StartupProgress::set_update_blocking`] le fait revenir sur une session déjà ouverte
//! (« Mettre à jour » dans les Options), sans quoi une mise à jour qui échoue au premier coup
//! d'oeil — dossier d'installation non inscriptible, manifeste injoignable — rendrait la main
//! aussi vite qu'elle l'a prise.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Voir la doc de module.
pub const STARTUP_TIMEOUT: Duration = Duration::from_secs(45);

/// Durée minimale d'affichage de l'écran de chargement — voir la doc de module.
pub const MIN_DISPLAY: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct StartupProgress {
    catalog: AtomicBool,
    dungeons: AtomicBool,
    log_replayed: AtomicBool,
    update_resolved: AtomicBool,
    update_blocking: AtomicBool,
    /// Échéance du plancher d'affichage, en millisecondes depuis `started_at` — voir la doc de
    /// module. Un entier relatif plutôt qu'un `Instant` : l'`AtomicU64` garde le type partageable
    /// sans verrou, comme ses voisins.
    min_display_until_ms: AtomicU64,
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
            min_display_until_ms: AtomicU64::new(MIN_DISPLAY.as_millis() as u64),
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
    ///
    /// Lever le drapeau alors que le démarrage était complet, c'est **rouvrir** l'écran de
    /// chargement sur une session déjà ouverte : le plancher d'affichage repart de zéro. Le lever
    /// pendant le démarrage ne le touche pas — l'écran est là depuis le début, son plancher court
    /// déjà.
    pub fn set_update_blocking(&self, blocking: bool) {
        if blocking && !self.is_update_blocking() && self.is_complete() {
            self.restart_min_display();
        }
        self.update_blocking.store(blocking, Ordering::Release);
    }

    /// Fait repartir le plancher d'affichage maintenant — voir [`set_update_blocking`].
    ///
    /// [`set_update_blocking`]: Self::set_update_blocking
    fn restart_min_display(&self) {
        let until = self.started_at.elapsed() + MIN_DISPLAY;
        self.min_display_until_ms
            .store(until.as_millis() as u64, Ordering::Release);
    }

    /// L'écran de chargement a-t-il été montré assez longtemps pour pouvoir tomber ?
    fn min_display_elapsed(&self) -> bool {
        self.started_at.elapsed().as_millis() as u64
            >= self.min_display_until_ms.load(Ordering::Acquire)
    }

    pub fn is_update_blocking(&self) -> bool {
        self.update_blocking.load(Ordering::Acquire)
    }

    /// Tout est chargé — ou le délai de garde est dépassé — aucune mise à jour n'est en cours, et
    /// l'écran de chargement a tenu son plancher d'affichage (voir la doc de module).
    pub fn is_complete(&self) -> bool {
        if self.is_update_blocking() {
            return false;
        }
        // AVANT le garde-fou : cinq secondes ne condamnent personne, et un `STARTUP_TIMEOUT` déjà
        // dépassé au retour sur l'écran (session ouverte depuis plus de quarante-cinq secondes)
        // le rendrait sinon complet à l'instant même où il réapparaît.
        if !self.min_display_elapsed() {
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
        if !self.min_display_elapsed() {
            pending.push("plancher d'affichage");
        }
        pending
    }

    /// Consomme le plancher d'affichage — **tests seulement**, pour vérifier la logique des
    /// drapeaux sans attendre cinq secondes.
    #[cfg(test)]
    fn skip_min_display(&self) {
        self.min_display_until_ms.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complet_seulement_quand_les_quatre_drapeaux_sont_poses() {
        let progress = StartupProgress::new();
        progress.skip_min_display();
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
        progress.skip_min_display();
        assert!(progress.is_complete());
        progress.set_update_blocking(true);
        assert!(!progress.is_complete());
        assert_eq!(
            progress.pending(),
            vec!["mise à jour en cours", "plancher d'affichage"]
        );
        progress.set_update_blocking(false);
        progress.skip_min_display();
        assert!(progress.is_complete());
    }

    /// Le clignotement : tout répond du premier coup, et l'écran tomberait dans la seconde sans
    /// le plancher — voir la doc de module.
    #[test]
    fn le_plancher_d_affichage_retient_l_ecran_meme_tout_charge() {
        let progress = StartupProgress::new();
        progress.mark_catalog();
        progress.mark_dungeons();
        progress.mark_log_replayed();
        progress.mark_update_resolved();
        assert!(!progress.is_complete());
        assert_eq!(progress.pending(), vec!["plancher d'affichage"]);
        progress.skip_min_display();
        assert!(progress.is_complete());
    }

    /// « Mettre à jour » depuis les Options rouvre l'écran : son plancher repart, sinon un échec
    /// immédiat le refermerait aussitôt.
    #[test]
    fn rouvrir_l_ecran_pour_une_mise_a_jour_fait_repartir_le_plancher() {
        let progress = StartupProgress::new();
        progress.mark_catalog();
        progress.mark_dungeons();
        progress.mark_log_replayed();
        progress.mark_update_resolved();
        progress.skip_min_display();
        assert!(progress.is_complete());

        progress.set_update_blocking(true);
        progress.set_update_blocking(false);
        assert!(!progress.is_complete());
        assert_eq!(progress.pending(), vec!["plancher d'affichage"]);
    }

    /// Pendant le démarrage, en revanche, l'écran est déjà là : lever le drapeau ne repousse pas
    /// une échéance qui court depuis le lancement.
    #[test]
    fn une_mise_a_jour_pendant_le_demarrage_ne_repousse_pas_le_plancher() {
        let progress = StartupProgress::new();
        // Échéance consommée pour que le test lise un « repoussé ou non » sans ambiguïté : seul un
        // redémarrage la ramènerait au-dessus de zéro.
        progress.skip_min_display();
        progress.set_update_blocking(true);
        assert_eq!(progress.min_display_until_ms.load(Ordering::Acquire), 0);

        progress.set_update_blocking(false);
        progress.mark_catalog();
        progress.mark_dungeons();
        progress.mark_log_replayed();
        progress.mark_update_resolved();
        assert!(progress.is_complete());
    }
}
