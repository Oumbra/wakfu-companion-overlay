//! Sons d'alerte (§9 du plan, « Alertes de drop ») — mêmes fichiers que le web
//! (`public/assets/sounds/{countdown-de76d898,alert-9d3d06ed}.mp3`, « fourni par l'utilisateur »
//! selon leur propre commentaire de source), embarqués ici pour ne dépendre d'aucun téléchargement
//! au premier lancement. Miroir réduit d'`AlertSoundService` (`alert-sound.service.ts`) : le web a
//! trois sons distincts (ramassage avec son activé, filtre de chat, décompte à 0) — seuls les deux
//! premiers l'ont été d'abord (voir `overlay_engine::watchlist::WatchlistAlert` et
//! `overlay_engine::profile::LootAlert`), le troisième — la recherche de chat,
//! `public/assets/sounds/chat-filter-c13da61f.mp3` côté web — depuis le 2026-09-13 (voir
//! `overlay_engine::chat_alert::ChatAlert`).

use std::sync::atomic::{AtomicU8, Ordering};

const COUNTDOWN_SOUND_BYTES: &[u8] = include_bytes!("../assets/sounds/countdown.mp3");
const LOOT_SOUND_BYTES: &[u8] = include_bytes!("../assets/sounds/loot.mp3");
const CHAT_SOUND_BYTES: &[u8] = include_bytes!("../assets/sounds/chat-filter.mp3");
/// Son de la notification de tour (§9.1 decies) — le toast Windows est silencieux, c'est ce
/// fichier qu'on entend. Ce n'est plus la copie provisoire du décompte des débuts : le fichier a
/// été remplacé (asset maintenu par l'utilisateur, comme `spells.json`) ; sa provenance reste à
/// documenter dans `assets/sounds/README.md`.
const TURN_SOUND_BYTES: &[u8] = include_bytes!("../assets/sounds/turn.mp3");

/// **Volume d'un son, en pour cent** — de [`VOLUME_MIN`] à [`VOLUME_MAX`].
///
/// Pas de zéro : un son à 0 % est un son coupé, et couper le son a déjà sa case (« Couper le son
/// des notifications », `panels::notifications`). Deux façons de dire la même chose se
/// contrediraient au premier aller-retour — une sourdine levée sur un volume nul resterait muette.
pub const VOLUME_MIN: u8 = 1;
pub const VOLUME_MAX: u8 = 100;
/// Le volume d'un son jamais réglé — plein, pour que rien ne change à la mise à jour : c'est le
/// volume auquel chaque son était joué avant que le réglage existe.
pub const VOLUME_DEFAULT: u8 = VOLUME_MAX;

/// Ramène un volume dans `VOLUME_MIN..=VOLUME_MAX` — un `config.toml` retouché à la main peut
/// porter `0` ou `250`.
pub fn clamp_volume(volume: u8) -> u8 {
    volume.clamp(VOLUME_MIN, VOLUME_MAX)
}

/// **Les volumes, un par section de « Paramètres »** (demande utilisateur du 2026-09-30).
///
/// Un par son plutôt qu'un seul pour tout l'overlay : les quatre fichiers n'ont pas le même niveau
/// d'enregistrement, et tout le monde n'est pas sensible de la même façon à un son aigu ou grave.
/// Même logique de voyage que `panels::notifications::AlertMutes` : un seul type, qui va de la
/// config persistée à la fenêtre Options et revient à l'hôte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertVolumes {
    /// La notification de tour — section « Combat » ([`play_turn_alert`]).
    pub turn: u8,
    /// Le décompte arrivé à zéro — section « Suivi » ([`play_countdown_alert`]).
    pub suivi: u8,
    /// Le ramassage d'un objet à alerte — section « Alertes » ([`play_loot_alert`]).
    pub alertes: u8,
    /// Le message trouvé — section « Chat » ([`play_chat_alert`]).
    pub chat: u8,
}

impl Default for AlertVolumes {
    fn default() -> Self {
        Self {
            turn: VOLUME_DEFAULT,
            suivi: VOLUME_DEFAULT,
            alertes: VOLUME_DEFAULT,
            chat: VOLUME_DEFAULT,
        }
    }
}

impl AlertVolumes {
    /// Les quatre volumes ramenés dans leurs bornes — voir [`clamp_volume`].
    pub fn clamped(self) -> Self {
        Self {
            turn: clamp_volume(self.turn),
            suivi: clamp_volume(self.suivi),
            alertes: clamp_volume(self.alertes),
            chat: clamp_volume(self.chat),
        }
    }
}

/// Les sons que l'overlay sait jouer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    Countdown,
    Loot,
    Chat,
    Turn,
}

impl Sound {
    fn label(self) -> &'static str {
        match self {
            Sound::Countdown => "décompte",
            Sound::Loot => "ramassage",
            Sound::Chat => "chat",
            Sound::Turn => "tour",
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            Sound::Countdown => COUNTDOWN_SOUND_BYTES,
            Sound::Loot => LOOT_SOUND_BYTES,
            Sound::Chat => CHAT_SOUND_BYTES,
            Sound::Turn => TURN_SOUND_BYTES,
        }
    }

    fn slot(self) -> &'static AtomicU8 {
        match self {
            Sound::Countdown => &VOLUME_SUIVI,
            Sound::Loot => &VOLUME_ALERTES,
            Sound::Chat => &VOLUME_CHAT,
            Sound::Turn => &VOLUME_TURN,
        }
    }
}

// **Les volumes en vigueur, au niveau du module** plutôt que portés par chaque appel : les sons
// partent de deux threads (l'hôte pour le tour et les essais, le thread Engine pour les alertes),
// et faire descendre les volumes jusqu'au second demanderait une commande de plus
// (`EngineCommand`) pour une valeur qui n'y décide de rien — le moteur ne choisit que SI un son
// part, jamais à quel volume. L'hôte les pose au démarrage et à chaque « Valider »
// ([`set_volumes`]).
static VOLUME_TURN: AtomicU8 = AtomicU8::new(VOLUME_DEFAULT);
static VOLUME_SUIVI: AtomicU8 = AtomicU8::new(VOLUME_DEFAULT);
static VOLUME_ALERTES: AtomicU8 = AtomicU8::new(VOLUME_DEFAULT);
static VOLUME_CHAT: AtomicU8 = AtomicU8::new(VOLUME_DEFAULT);

/// Pose les volumes en vigueur — au démarrage (lus de la config) et à chaque « Valider » de la
/// fenêtre Options. Les sons déjà en cours de lecture gardent leur volume.
pub fn set_volumes(volumes: AlertVolumes) {
    let volumes = volumes.clamped();
    VOLUME_TURN.store(volumes.turn, Ordering::Relaxed);
    VOLUME_SUIVI.store(volumes.suivi, Ordering::Relaxed);
    VOLUME_ALERTES.store(volumes.alertes, Ordering::Relaxed);
    VOLUME_CHAT.store(volumes.chat, Ordering::Relaxed);
}

/// Gain appliqué au flux pour un volume en pour cent — **le carré de la fraction**, pas la
/// fraction elle-même.
///
/// L'oreille entend l'intensité de façon logarithmique : un gain linéaire tasse toute la variation
/// audible dans le bas du curseur, et les trois quarts de sa course ne changent presque rien. Le
/// carré (−6 dB à 71 %, −12 dB à 50 %, −40 dB à 10 %) répartit mieux l'effet sur la course, sans
/// jamais atteindre le silence : à 1 % il reste −80 dB.
pub fn gain(volume: u8) -> f32 {
    let fraction = f32::from(clamp_volume(volume)) / f32::from(VOLUME_MAX);
    fraction * fraction
}

/// Joue `sound` à `volume` — le bouton d'essai de la fenêtre Options, qui fait entendre le volume
/// du BROUILLON avant qu'il soit validé. Voir `play_alert` pour les garanties.
pub fn play(sound: Sound, volume: u8) {
    play_alert(sound.label(), sound.bytes(), gain(volume));
}

/// Joue `sound` au volume en vigueur ([`set_volumes`]).
fn play_current(sound: Sound) {
    play(sound, sound.slot().load(Ordering::Relaxed));
}

/// Joue le son de décompte à 0 — voir `play_alert` pour les garanties (thread dédié, best-effort).
pub fn play_countdown_alert() {
    play_current(Sound::Countdown);
}

/// Joue le son de ramassage (objet à son activé au compte, voir `overlay_engine::profile`) — voir
/// `play_alert` pour les garanties (thread dédié, best-effort).
pub fn play_loot_alert() {
    play_current(Sound::Loot);
}

/// Joue le son de recherche de chat (un message correspond à une recherche de l'onglet « Chat »,
/// voir `overlay_engine::chat_alert`) — le fichier du web, tel quel. Voir `play_alert` pour les
/// garanties (thread dédié, best-effort).
pub fn play_chat_alert() {
    play_current(Sound::Chat);
}

/// Joue le son de la notification de tour — voir `TURN_SOUND_BYTES` et `play_alert`.
pub fn play_turn_alert() {
    play_current(Sound::Turn);
}

/// Joue `bytes` sur un thread dédié et JAMAIS bloquant pour l'appelant (voir
/// `main.rs::spawn_engine_thread`, qui appelle ceci depuis le thread Engine — bloquer là
/// retarderait l'ingestion du log). Un thread par lecture plutôt qu'un flux audio persistant :
/// les alertes sont rares, le coût d'ouvrir/fermer le périphérique audio à chaque fois est
/// négligeable en pratique et évite de garder une ressource système ouverte en permanence pour un
/// besoin aussi occasionnel (budget mémoire §8 du plan). Best-effort : un périphérique audio
/// indisponible (ou l'échec du décodage) laisse l'alerte visuelle (toast) fonctionner seule,
/// jamais une raison de faire échouer quoi que ce soit d'autre — même philosophie que
/// `AlertSoundService.play` côté web (`catch` silencieux). `label` ne sert qu'au message
/// d'avertissement en cas d'échec, pour distinguer laquelle des alertes est en cause. `gain` est
/// déjà passé par [`gain`] : c'est le facteur d'amplitude appliqué au flux.
fn play_alert(label: &'static str, bytes: &'static [u8], gain: f32) {
    std::thread::Builder::new()
        .name("overlay-alert-sound".into())
        .spawn(move || {
            if let Err(err) = play_blocking(bytes, gain) {
                tracing::warn!(%err, "lecture du son d'alerte de {label} impossible");
            }
        })
        .expect("échec de création du thread de lecture audio");
}

fn play_blocking(bytes: &'static [u8], gain: f32) -> Result<(), Box<dyn std::error::Error>> {
    let stream_handle = rodio::OutputStreamBuilder::open_default_stream()?;
    let sink = rodio::Sink::connect_new(stream_handle.mixer());
    sink.set_volume(gain);
    let cursor = std::io::Cursor::new(bytes);
    let source = rodio::Decoder::new(cursor)?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_volume_hors_bornes_est_ramene_dedans() {
        assert_eq!(clamp_volume(0), VOLUME_MIN);
        assert_eq!(clamp_volume(250), VOLUME_MAX);
        assert_eq!(clamp_volume(42), 42);
    }

    #[test]
    fn le_gain_est_plein_a_100_et_jamais_nul() {
        assert_eq!(gain(VOLUME_MAX), 1.0);
        assert!((gain(50) - 0.25).abs() < 1e-6);
        assert!(gain(VOLUME_MIN) > 0.0, "1 % n'est pas le silence");
        // Un 0 venu d'ailleurs ne coupe pas le son en douce : il est ramené à 1 %.
        assert_eq!(gain(0), gain(VOLUME_MIN));
    }

    #[test]
    fn le_gain_croit_avec_le_volume() {
        for v in VOLUME_MIN..VOLUME_MAX {
            assert!(
                gain(v) < gain(v + 1),
                "le gain doit croître entre {v} et {}",
                v + 1
            );
        }
    }

    #[test]
    fn les_volumes_par_defaut_sont_pleins() {
        let v = AlertVolumes::default();
        assert_eq!([v.turn, v.suivi, v.alertes, v.chat], [VOLUME_MAX; 4]);
    }
}
