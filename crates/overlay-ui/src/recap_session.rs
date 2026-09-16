//! **La session du Récap** — ce que la bande `panels::recap` appelle « la session » : son chrono,
//! ses quatre compteurs, et la règle qui décide quand ils continuent et quand ils repartent de
//! zéro (2026-09-17, demande utilisateur).
//!
//! ## La règle, en une phrase
//!
//! **La session, c'est ce que l'overlay a vu du jeu.** Le chrono avance tant qu'au moins une
//! fenêtre de jeu est à l'écran — le signal qui écrit déjà « [fenêtre de jeu] X trouvée / fermée »
//! au journal (`game_window::scan`, un balayage par tick de l'hôte) — et les quatre compteurs
//! (kamas, XP, combats, challenges) suivent la même vie que lui. Une fenêtre de jeu qui disparaît
//! met la session **en pause** ; une qui revient la **reprend** si la pause n'a pas dépassé la
//! tolérance réglée par l'utilisateur ([`ResumeSettings`], section « Recap » des Paramètres), et
//! la **remplace** sinon, tout à zéro.
//!
//! Avant ce module, le chrono partait au lancement du processus et ne s'arrêtait jamais
//! (`App::started_at`), et les compteurs couvraient tout le `wakfu.log` relu au démarrage — un
//! écart noté comme « chantier à part » dans le plan (§9.1 octodecies). Retour utilisateur du
//! 2026-09-16 tard : « j'aimerais que ce soit correspondant au temps où l'overlay est visible par
//! l'utilisateur [...] si l'utilisateur ferme sa fenêtre et la réouvre dans un laps de temps de
//! quelques minutes, on ne va pas réinitialiser ce compteur, on va ajouter la session à la session
//! passée ».
//!
//! ## Ce que « ce que l'overlay a vu » implique — décisions du 2026-09-17
//!
//! - **Une seule session, même en multi-compte** : deux fenêtres de jeu ne comptent pas double,
//!   et les totaux du moteur sont de toute façon globaux. Décision explicite de l'utilisateur.
//! - **Overlay éteint, jeu ouvert** : ni le temps ni les gains de ce trou n'entrent dans la
//!   session — l'overlay ne les a pas vus. L'alternative (dater chaque ligne du log et compter
//!   tout ce qui suit le début de la session) aurait fait deux règles, une pour le temps et une
//!   pour les gains ; écartée par l'utilisateur : « ce que l'overlay a vu ».
//! - **Les compteurs sont une DIFFÉRENCE**, pas une seconde comptabilité : le moteur garde ses
//!   totaux de fichier (`overlay_engine::SessionTotals`, ce dont l'historique a besoin), et ce
//!   module retient un **point de référence** — les totaux du moteur au dernier pliage — pour
//!   afficher `totaux de session + (moteur − référence)` ([`RecapSession::totals`]). Une rotation
//!   de `wakfu.log` en cours de route ne gêne pas : les totaux du moteur ne redescendent jamais
//!   (`Engine::state_initialized`).
//! - **Machine en veille, jeu ouvert** : entre deux ticks, l'horloge murale saute. Un saut de plus
//!   de [`TICK_GAP`] n'est pas du jeu : il est traité exactement comme une pause, avec la même
//!   règle de reprise — le pendant de la coupure à cinq minutes de silence que fait le web
//!   (`StatsStoreService.accumulateSessionDuration`).
//!
//! ## Ce qui est écrit, et où
//!
//! Un JSON ([`FILE_NAME`]) à côté des combats en cours (`overlay_engine::fight_store`, même
//! dossier `data/`), écrit à chaque pause, à chaque remise à zéro, et toutes les [`SAVE_INTERVAL`]
//! pendant le jeu — un crash ne coûte alors qu'une demi-minute. Au démarrage, l'hôte le relit et
//! la première fenêtre de jeu trouvée passe par la règle de reprise avec le « dernier instant
//! actif » qu'il porte. Fichier absent ou illisible : session neuve, sans bruit — même tolérance
//! que `fight_store`.
//!
//! Le module est **pur** hors de `load`/`save` : l'hôte lui donne l'heure et l'état des fenêtres
//! ([`RecapSession::observe`]), il rend sa décision ([`Transition`]) — c'est ce qui le rend
//! testable sans fenêtre ni horloge.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use overlay_engine::SessionTotals;
use serde::{Deserialize, Serialize};

use crate::panels::recap::format_duration;

/// Tolérance de pause par défaut, en minutes — décision utilisateur du 2026-09-17 (« 60 ») :
/// un repas sans perdre la soirée.
pub const DEFAULT_RESUME_MINUTES: i64 = 60;
/// Borne basse du pas numérique des Paramètres. En dessous, la reprise ne servirait qu'à
/// survivre au redémarrage du client.
pub const MIN_RESUME_MINUTES: i64 = 5;
/// Borne haute : une journée. Au-delà, ce n'est plus une pause, c'est une autre session.
pub const MAX_RESUME_MINUTES: i64 = 24 * 60;
/// Incrément du pas numérique — des minutes rondes, pas de 61.
pub const RESUME_STEP_MINUTES: i64 = 5;

/// Saut d'horloge entre deux ticks au-delà duquel le temps écoulé n'est PAS compté comme du jeu
/// (machine en veille, boucle d'événements bloquée) et vaut une pause — voir la doc de module.
/// Deux minutes : très au-dessus d'un tick (l'hôte balaie les fenêtres à chaque `about_to_wait`),
/// très en dessous de la plus petite tolérance réglable ([`MIN_RESUME_MINUTES`]).
pub const TICK_GAP: Duration = Duration::from_secs(2 * 60);

/// Cadence d'écriture du fichier pendant le jeu — voir la doc de module.
const SAVE_INTERVAL: Duration = Duration::from_secs(30);

/// Nom du fichier, dans le dossier de `overlay_engine::fight_store::default_store_dir`.
pub const FILE_NAME: &str = "recap-session.json";

/// Le réglage « Reprendre la session après une pause de moins de … min » de la section « Recap »
/// des Paramètres — persisté dans la config LOCALE (`config::OverlayConfig::recap_resume`), pas au
/// compte : il n'a pas d'équivalent web, et le serveur n'accepte que des clés connues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResumeSettings {
    /// Case cochée. Décochée, chaque retour dans le jeu repart de zéro.
    pub enabled: bool,
    /// La tolérance, en minutes — toujours dans [`MIN_RESUME_MINUTES`]..=[`MAX_RESUME_MINUTES`]
    /// (voir [`Self::set_minutes`]).
    pub minutes: i64,
}

impl Default for ResumeSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            minutes: DEFAULT_RESUME_MINUTES,
        }
    }
}

impl ResumeSettings {
    /// Pose une tolérance, bornée — une config écrite à la main hors bornes revient dedans.
    pub fn set_minutes(&mut self, minutes: i64) {
        self.minutes = minutes.clamp(MIN_RESUME_MINUTES, MAX_RESUME_MINUTES);
    }

    /// La pause maximale qui permet de reprendre — `None` quand la case est décochée.
    pub fn tolerance(&self) -> Option<Duration> {
        self.enabled
            .then(|| Duration::from_secs(self.minutes.max(0) as u64 * 60))
    }
}

/// Ce que [`RecapSession::observe`] vient de décider — rendu pour les tests et le journal ; l'hôte
/// n'a rien à en faire, la session s'est déjà mise à jour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// Toute première fenêtre de jeu d'une session qui n'en avait jamais vu.
    Started,
    /// Une fenêtre de jeu est revenue après une pause tolérée : chrono et compteurs continuent.
    Resumed { pause: Duration },
    /// Une fenêtre de jeu est revenue après une pause trop longue (ou la reprise est désactivée) :
    /// tout est reparti de zéro.
    Restarted { pause: Duration },
    /// Plus aucune fenêtre de jeu : le chrono s'arrête, tout est conservé.
    Paused,
}

/// La forme sur disque — voir la doc de module. Des millisecondes depuis l'époque Unix pour les
/// instants, comme `FightSnapshot::started_at_ms`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Persisted {
    started_at_ms: i64,
    last_seen_at_ms: i64,
    active_secs: u64,
    totals: SessionTotals,
    resumed: u32,
}

/// La session — voir la doc de module.
#[derive(Debug)]
pub struct RecapSession {
    /// `None` pour une session en mémoire seule (tests).
    path: Option<PathBuf>,
    resume: ResumeSettings,
    /// Début de la session courante — ce que l'infobulle de la durée affiche.
    started_at: SystemTime,
    /// Dernier instant où une fenêtre de jeu était présente ; `None` tant que la session n'en a
    /// jamais vu. C'est contre lui que la pause se mesure au retour d'une fenêtre.
    last_seen_at: Option<SystemTime>,
    /// Le chrono : temps cumulé avec une fenêtre de jeu présente.
    active: Duration,
    /// Les compteurs de la session, PLIÉS au dernier pliage — voir [`Self::totals`].
    totals: SessionTotals,
    /// Les totaux du moteur au dernier pliage ; `None` avant la première observation.
    baseline: Option<SessionTotals>,
    /// Nombre de reprises de la session courante.
    resumed: u32,
    /// `Some` tant qu'une fenêtre de jeu est présente : l'instant du tick précédent.
    last_tick: Option<SystemTime>,
    last_save: Option<Instant>,
    /// Dernier état du moteur reçu — pour plier à la fermeture sans le redemander.
    last_engine: SessionTotals,
    dirty: bool,
}

impl RecapSession {
    /// Relit la session persistée à `path` (voir la doc de module) — session neuve si le fichier
    /// manque ou ne se lit pas.
    pub fn load(path: PathBuf, resume: ResumeSettings, now: SystemTime) -> Self {
        let persisted = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Persisted>(&bytes) {
                Ok(persisted) => Some(persisted),
                Err(err) => {
                    tracing::warn!(
                        "[session] {} illisible ({err}) — session neuve.",
                        path.display()
                    );
                    None
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => {
                tracing::warn!(
                    "[session] {} inaccessible ({err}) — session neuve.",
                    path.display()
                );
                None
            }
        };
        let mut session = Self::in_memory(resume, now);
        session.path = Some(path);
        if let Some(persisted) = persisted {
            session.started_at = from_ms(persisted.started_at_ms).unwrap_or(now);
            session.last_seen_at = from_ms(persisted.last_seen_at_ms);
            session.active = Duration::from_secs(persisted.active_secs);
            session.totals = persisted.totals;
            session.resumed = persisted.resumed;
            tracing::info!(
                "[session] relue — {} de jeu, dernière fenêtre de jeu vue à {}.",
                format_duration(session.active),
                session
                    .last_seen_at
                    .map(local_hhmm)
                    .unwrap_or_else(|| "jamais".to_string())
            );
        }
        session
    }

    /// Une session neuve, jamais écrite — pour les tests, et le repli de [`Self::load`].
    pub fn in_memory(resume: ResumeSettings, now: SystemTime) -> Self {
        Self {
            path: None,
            resume,
            started_at: now,
            last_seen_at: None,
            active: Duration::ZERO,
            totals: SessionTotals::default(),
            baseline: None,
            resumed: 0,
            last_tick: None,
            last_save: None,
            last_engine: SessionTotals::default(),
            dirty: false,
        }
    }

    /// Le réglage de reprise en vigueur — ce sur quoi la fenêtre Options s'ouvre.
    pub fn resume_settings(&self) -> ResumeSettings {
        self.resume
    }

    /// Pose le réglage de reprise — pris en compte à la prochaine pause.
    pub fn set_resume_settings(&mut self, resume: ResumeSettings) {
        self.resume = resume;
    }

    /// Un tick de l'hôte : `game_present` dit si au moins une fenêtre de jeu est à l'écran,
    /// `engine` porte les totaux courants du moteur, `now` l'horloge murale. Rend ce qui vient
    /// d'être décidé, s'il s'est décidé quelque chose — voir la doc de module pour la règle.
    pub fn observe(
        &mut self,
        game_present: bool,
        engine: &SessionTotals,
        now: SystemTime,
    ) -> Option<Transition> {
        self.last_engine = *engine;
        // Première observation : les totaux du moteur d'AVANT sont ceux du fichier relu, pas
        // ceux de la session — la référence les met de côté.
        if self.baseline.is_none() {
            self.baseline = Some(*engine);
        }
        if !game_present {
            if self.last_tick.take().is_none() {
                return None;
            }
            tracing::info!(
                "[session] en pause à {} — reprise possible {}.",
                format_duration(self.active),
                match self.resume.tolerance() {
                    Some(tolerance) => format!("jusqu'à {}", local_hhmm(now + tolerance)),
                    None => "désactivée".to_string(),
                }
            );
            self.save(engine);
            return Some(Transition::Paused);
        }
        let transition = match self.last_tick {
            None => Some(self.arrive(engine, now)),
            Some(previous) => {
                let delta = now.duration_since(previous).unwrap_or_default();
                if delta > TICK_GAP {
                    // Veille ou boucle bloquée : ce temps n'est pas du jeu (voir `TICK_GAP`).
                    Some(self.arrive(engine, now))
                } else {
                    self.active += delta;
                    None
                }
            }
        };
        self.last_tick = Some(now);
        self.last_seen_at = Some(now);
        self.dirty = true;
        if self
            .last_save
            .is_none_or(|at| at.elapsed() >= SAVE_INTERVAL)
        {
            self.save(engine);
        }
        transition
    }

    /// Une fenêtre de jeu vient d'apparaître (ou l'horloge vient de sauter) : reprendre ou
    /// repartir, selon la pause mesurée contre le dernier instant actif.
    fn arrive(&mut self, engine: &SessionTotals, now: SystemTime) -> Transition {
        let Some(seen) = self.last_seen_at else {
            self.started_at = now;
            tracing::info!("[session] début à {}.", local_hhmm(now));
            return Transition::Started;
        };
        let pause = now.duration_since(seen).unwrap_or_default();
        match self.resume.tolerance() {
            Some(tolerance) if pause <= tolerance => {
                self.resumed += 1;
                tracing::info!(
                    "[session] reprise — pause de {} (tolérance {}), {} et {} combat(s) conservés.",
                    format_pause(pause),
                    format_pause(tolerance),
                    format_duration(self.active),
                    self.totals(engine).fights_won + self.totals(engine).fights_lost
                );
                Transition::Resumed { pause }
            }
            Some(tolerance) => {
                tracing::info!(
                    "[session] nouvelle session — pause de {} (tolérance {}).",
                    format_pause(pause),
                    format_pause(tolerance)
                );
                self.start_fresh(engine, now);
                Transition::Restarted { pause }
            }
            None => {
                tracing::info!(
                    "[session] nouvelle session — pause de {}, la reprise est désactivée.",
                    format_pause(pause)
                );
                self.start_fresh(engine, now);
                Transition::Restarted { pause }
            }
        }
    }

    fn start_fresh(&mut self, engine: &SessionTotals, now: SystemTime) {
        self.started_at = now;
        self.last_seen_at = Some(now);
        self.active = Duration::ZERO;
        self.totals = SessionTotals::default();
        self.baseline = Some(*engine);
        self.resumed = 0;
        self.dirty = true;
    }

    /// Le bouton du bloc, une fois confirmé : tout à zéro, la session repart de cet instant.
    pub fn reset(&mut self, engine: &SessionTotals, now: SystemTime) {
        let before = self.totals(engine);
        tracing::info!(
            "[session] remise à zéro demandée — {}, {} combat(s) et {} XP effacés.",
            format_duration(self.active),
            before.fights_won + before.fights_lost,
            before.xp_gained
        );
        self.start_fresh(engine, now);
        self.save(engine);
    }

    /// Les compteurs de la session : ceux pliés, plus ce que le moteur a gagné depuis le pliage.
    /// Jamais négatif par champ — un moteur qui redescendrait sous la référence (ne devrait pas
    /// arriver, voir la doc de module) ne ferait pas reculer la session.
    pub fn totals(&self, engine: &SessionTotals) -> SessionTotals {
        let base = self.baseline.unwrap_or(*engine);
        let gained = |session: i64, now: i64, then: i64| session + (now - then).max(0);
        SessionTotals {
            kamas_gained: gained(
                self.totals.kamas_gained,
                engine.kamas_gained,
                base.kamas_gained,
            ),
            kamas_lost: gained(self.totals.kamas_lost, engine.kamas_lost, base.kamas_lost),
            xp_gained: gained(self.totals.xp_gained, engine.xp_gained, base.xp_gained),
            loot_count: gained(self.totals.loot_count, engine.loot_count, base.loot_count),
            fights_won: gained(self.totals.fights_won, engine.fights_won, base.fights_won),
            fights_lost: gained(
                self.totals.fights_lost,
                engine.fights_lost,
                base.fights_lost,
            ),
            challenges_passed: gained(
                self.totals.challenges_passed,
                engine.challenges_passed,
                base.challenges_passed,
            ),
            challenges_failed: gained(
                self.totals.challenges_failed,
                engine.challenges_failed,
                base.challenges_failed,
            ),
        }
    }

    /// Le chrono.
    pub fn uptime(&self) -> Duration {
        self.active
    }

    /// Heure locale `HH:MM` du début de la session — « Session depuis 20:12 ».
    pub fn started_at_local(&self) -> String {
        local_hhmm(self.started_at)
    }

    /// Nombre de reprises de la session courante.
    pub fn resumed(&self) -> u32 {
        self.resumed
    }

    /// Plie les compteurs sur les totaux courants du moteur, puis écrit le fichier s'il y en a un
    /// — best-effort, comme `fight_store::save_fight` : une écriture échouée ne doit rien casser.
    fn save(&mut self, engine: &SessionTotals) {
        self.totals = self.totals(engine);
        self.baseline = Some(*engine);
        self.last_save = Some(Instant::now());
        self.dirty = false;
        let Some(path) = &self.path else {
            return;
        };
        let persisted = Persisted {
            started_at_ms: to_ms(self.started_at),
            last_seen_at_ms: self.last_seen_at.map(to_ms).unwrap_or(0),
            active_secs: self.active.as_secs(),
            totals: self.totals,
            resumed: self.resumed,
        };
        if let Err(err) = write_json(path, &persisted) {
            tracing::warn!(
                "[session] écriture de {} impossible : {err}",
                path.display()
            );
        }
    }
}

impl Drop for RecapSession {
    /// L'overlay qui s'arrête proprement écrit ce qu'il a vu depuis la dernière écriture — sans
    /// ce pli, un « Quitter » juste après une reprise perdrait jusqu'à trente secondes.
    fn drop(&mut self) {
        if self.dirty && self.path.is_some() {
            let engine = self.last_engine;
            self.save(&engine);
        }
    }
}

fn write_json(path: &Path, persisted: &Persisted) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(persisted).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}

fn to_ms(at: SystemTime) -> i64 {
    at.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn from_ms(ms: i64) -> Option<SystemTime> {
    (ms > 0).then(|| UNIX_EPOCH + Duration::from_millis(ms as u64))
}

/// `HH:MM` en heure locale — la même délégation à l'OS que `overlay_engine::log_time`.
fn local_hhmm(at: SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(at)
        .format("%H:%M")
        .to_string()
}

/// Une pause lisible au journal : « < 1 min », « 40 min », « 3 h 25 ».
fn format_pause(pause: Duration) -> String {
    let minutes = pause.as_secs() / 60;
    match minutes {
        0 => "< 1 min".to_string(),
        m if m < 60 => format!("{m} min"),
        m => format!("{} h {:02}", m / 60, m % 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: SystemTime = UNIX_EPOCH;

    fn at(secs: u64) -> SystemTime {
        T0 + Duration::from_secs(secs)
    }

    fn min(m: u64) -> Duration {
        Duration::from_secs(m * 60)
    }

    fn engine(xp: i64, fights_won: i64) -> SessionTotals {
        SessionTotals {
            xp_gained: xp,
            fights_won,
            ..Default::default()
        }
    }

    /// Joue `ticks` secondes de jeu, une observation par seconde, moteur inchangé.
    fn play(session: &mut RecapSession, from: u64, secs: u64, totals: &SessionTotals) -> u64 {
        for s in 1..=secs {
            session.observe(true, totals, at(from + s));
        }
        from + secs
    }

    /// Le tout premier tick avec une fenêtre de jeu démarre la session à cet instant — et le
    /// chrono n'avance qu'à partir du tick SUIVANT (rien ne s'est encore écoulé).
    #[test]
    fn la_premiere_fenetre_demarre_la_session() {
        let mut session = RecapSession::in_memory(ResumeSettings::default(), at(0));
        assert_eq!(
            session.observe(true, &engine(0, 0), at(10)),
            Some(Transition::Started)
        );
        assert_eq!(session.uptime(), Duration::ZERO);
        play(&mut session, 10, 5, &engine(0, 0));
        assert_eq!(session.uptime(), Duration::from_secs(5));
    }

    /// Sans fenêtre de jeu, rien ne bouge : ni transition, ni chrono.
    #[test]
    fn sans_fenetre_rien_ne_demarre() {
        let mut session = RecapSession::in_memory(ResumeSettings::default(), at(0));
        assert_eq!(session.observe(false, &engine(0, 0), at(10)), None);
        assert_eq!(session.observe(false, &engine(0, 0), at(20)), None);
        assert_eq!(session.uptime(), Duration::ZERO);
    }

    /// Le scénario de la demande : 20 min de jeu, 40 min de pause, tolérance 60 min — la session
    /// reprend avec ses 20 min, et compte une reprise.
    #[test]
    fn une_pause_toleree_reprend_la_session() {
        let mut session = RecapSession::in_memory(ResumeSettings::default(), at(0));
        session.observe(true, &engine(0, 0), at(0));
        let t = play(&mut session, 0, 20 * 60, &engine(0, 0));
        assert_eq!(
            session.observe(false, &engine(0, 0), at(t + 1)),
            Some(Transition::Paused)
        );
        let back = t + 40 * 60;
        assert_eq!(
            session.observe(true, &engine(0, 0), at(back)),
            Some(Transition::Resumed { pause: min(40) })
        );
        assert_eq!(session.uptime(), min(20));
        assert_eq!(session.resumed(), 1);
        play(&mut session, back, 60, &engine(0, 0));
        assert_eq!(session.uptime(), min(21));
    }

    /// Le second scénario : 3 h de pause pour 60 min de tolérance — tout repart de zéro, et le
    /// début de la session est l'instant du retour.
    #[test]
    fn une_pause_trop_longue_remplace_la_session() {
        let mut session = RecapSession::in_memory(ResumeSettings::default(), at(0));
        session.observe(true, &engine(0, 0), at(0));
        let t = play(&mut session, 0, 20 * 60, &engine(0, 5));
        session.observe(false, &engine(0, 5), at(t + 1));
        let back = t + 3 * 3600;
        assert_eq!(
            session.observe(true, &engine(0, 5), at(back)),
            Some(Transition::Restarted { pause: min(180) })
        );
        assert_eq!(session.uptime(), Duration::ZERO);
        assert_eq!(session.resumed(), 0);
        assert_eq!(session.totals(&engine(0, 5)).fights_won, 0);
        assert_eq!(session.started_at, at(back));
    }

    /// Case décochée : même une pause d'une minute remplace la session.
    #[test]
    fn la_reprise_desactivee_repart_toujours_de_zero() {
        let resume = ResumeSettings {
            enabled: false,
            minutes: 60,
        };
        let mut session = RecapSession::in_memory(resume, at(0));
        session.observe(true, &engine(0, 0), at(0));
        let t = play(&mut session, 0, 600, &engine(0, 0));
        session.observe(false, &engine(0, 0), at(t + 1));
        assert_eq!(
            session.observe(true, &engine(0, 0), at(t + 60)),
            Some(Transition::Restarted { pause: min(1) })
        );
        assert_eq!(session.uptime(), Duration::ZERO);
    }

    /// La pause se mesure à la seconde près contre la tolérance : 60 min tout juste reprend,
    /// une seconde de plus remplace.
    #[test]
    fn la_tolerance_est_inclusive() {
        for (extra, expect_resume) in [(0u64, true), (1, false)] {
            let mut session = RecapSession::in_memory(ResumeSettings::default(), at(0));
            session.observe(true, &engine(0, 0), at(0));
            session.observe(false, &engine(0, 0), at(1));
            let back = 3600 + extra;
            let transition = session.observe(true, &engine(0, 0), at(back)).unwrap();
            assert_eq!(
                matches!(transition, Transition::Resumed { .. }),
                expect_resume,
                "pause de 60 min + {extra} s"
            );
        }
    }

    /// Les compteurs de la session sont une différence avec le point de référence : ce que le
    /// moteur portait AVANT la première observation (le fichier relu) n'en fait pas partie.
    #[test]
    fn les_compteurs_ignorent_ce_qui_precede_la_session() {
        let mut session = RecapSession::in_memory(ResumeSettings::default(), at(0));
        session.observe(true, &engine(20_000_000_000, 40), at(0));
        assert_eq!(
            session.totals(&engine(20_000_000_000, 40)),
            SessionTotals::default()
        );
        session.observe(true, &engine(20_000_012_345, 41), at(1));
        let totals = session.totals(&engine(20_000_012_345, 41));
        assert_eq!(totals.xp_gained, 12_345);
        assert_eq!(totals.fights_won, 1);
    }

    /// Le pliage (à chaque écriture) ne change rien à ce qui s'affiche : les compteurs sont les
    /// mêmes juste avant et juste après.
    #[test]
    fn le_pliage_est_invisible() {
        let mut session = RecapSession::in_memory(ResumeSettings::default(), at(0));
        session.observe(true, &engine(100, 0), at(0));
        session.observe(true, &engine(150, 1), at(1));
        let before = session.totals(&engine(150, 1));
        session.save(&engine(150, 1));
        assert_eq!(session.totals(&engine(150, 1)), before);
        session.observe(true, &engine(160, 1), at(2));
        assert_eq!(session.totals(&engine(160, 1)).xp_gained, 60);
    }

    /// La remise à zéro efface chrono, compteurs et reprises, et le début devient l'instant du
    /// clic — ce que le moteur gagne ENSUITE compte à nouveau.
    #[test]
    fn la_remise_a_zero_repart_de_maintenant() {
        let mut session = RecapSession::in_memory(ResumeSettings::default(), at(0));
        session.observe(true, &engine(0, 0), at(0));
        let t = play(&mut session, 0, 300, &engine(500, 3));
        session.reset(&engine(500, 3), at(t));
        assert_eq!(session.uptime(), Duration::ZERO);
        assert_eq!(session.totals(&engine(500, 3)), SessionTotals::default());
        assert_eq!(session.started_at, at(t));
        play(&mut session, t, 10, &engine(520, 4));
        let totals = session.totals(&engine(520, 4));
        assert_eq!((totals.xp_gained, totals.fights_won), (20, 1));
        assert_eq!(session.uptime(), Duration::from_secs(10));
    }

    /// Un saut d'horloge de plus de `TICK_GAP` entre deux ticks (machine en veille, jeu ouvert)
    /// n'est pas compté comme du jeu — et passe par la règle de reprise.
    #[test]
    fn la_veille_ne_compte_pas_comme_du_jeu() {
        let mut session = RecapSession::in_memory(ResumeSettings::default(), at(0));
        session.observe(true, &engine(0, 0), at(0));
        let t = play(&mut session, 0, 60, &engine(0, 0));
        // Dix minutes sans tick : tolérées (60 min), la session reprend sans les compter.
        assert_eq!(
            session.observe(true, &engine(0, 0), at(t + 600)),
            Some(Transition::Resumed { pause: min(10) })
        );
        assert_eq!(session.uptime(), Duration::from_secs(60));
        // Deux heures sans tick : trop long, la session est remplacée.
        assert_eq!(
            session.observe(true, &engine(0, 0), at(t + 600 + 7200)),
            Some(Transition::Restarted { pause: min(120) })
        );
        assert_eq!(session.uptime(), Duration::ZERO);
    }

    /// Ce qui est écrit se relit : chrono, compteurs, reprises et dernier instant actif — et une
    /// session relue reprend (ou non) exactement comme une session restée en mémoire.
    #[test]
    fn la_session_survit_a_un_redemarrage() {
        let dir = std::env::temp_dir().join(format!(
            "wakfu-companion-overlay-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join(FILE_NAME);
        {
            let mut session = RecapSession::load(path.clone(), ResumeSettings::default(), at(0));
            session.observe(true, &engine(1_000, 10), at(0));
            let t = play(&mut session, 0, 20 * 60, &engine(1_500, 12));
            session.observe(false, &engine(1_500, 12), at(t + 1));
        }
        let mut session = RecapSession::load(path.clone(), ResumeSettings::default(), at(0));
        assert_eq!(session.uptime(), min(20));
        assert_eq!(session.totals(&engine(0, 0)).xp_gained, 500);
        assert_eq!(session.totals(&engine(0, 0)).fights_won, 2);
        // Après une rotation de `wakfu.log`, le moteur repart de zéro : la référence se pose sur
        // ses nouveaux totaux à la première observation, la session continue.
        // La pause se mesure depuis le dernier tick ACTIF (20 min), pas depuis celui qui l'a
        // constatée une seconde plus tard.
        let back = 20 * 60 + 30 * 60;
        assert_eq!(
            session.observe(true, &engine(0, 0), at(back)),
            Some(Transition::Resumed { pause: min(30) })
        );
        session.observe(true, &engine(70, 1), at(back + 1));
        let totals = session.totals(&engine(70, 1));
        assert_eq!((totals.xp_gained, totals.fights_won), (570, 3));
        assert_eq!(session.resumed(), 1);
        drop(session);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Un fichier absent ou corrompu donne une session neuve, sans paniquer.
    #[test]
    fn un_fichier_absent_ou_corrompu_donne_une_session_neuve() {
        let dir = std::env::temp_dir().join(format!(
            "wakfu-companion-overlay-test-corrompu-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(FILE_NAME);
        let session = RecapSession::load(path.clone(), ResumeSettings::default(), at(5));
        assert_eq!(session.last_seen_at, None);
        std::fs::write(&path, b"{ pas du json").unwrap();
        let session = RecapSession::load(path, ResumeSettings::default(), at(5));
        assert_eq!(session.last_seen_at, None);
        assert_eq!(session.started_at, at(5));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn les_minutes_de_reprise_sont_bornees() {
        let mut resume = ResumeSettings::default();
        resume.set_minutes(0);
        assert_eq!(resume.minutes, MIN_RESUME_MINUTES);
        resume.set_minutes(10_000);
        assert_eq!(resume.minutes, MAX_RESUME_MINUTES);
        resume.set_minutes(90);
        assert_eq!(resume.tolerance(), Some(min(90)));
        resume.enabled = false;
        assert_eq!(resume.tolerance(), None);
    }

    #[test]
    fn une_pause_se_lit_en_minutes_puis_en_heures() {
        assert_eq!(format_pause(Duration::from_secs(30)), "< 1 min");
        assert_eq!(format_pause(min(40)), "40 min");
        assert_eq!(format_pause(min(205)), "3 h 25");
    }
}
