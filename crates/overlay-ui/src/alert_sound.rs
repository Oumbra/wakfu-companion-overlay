//! Sons d'alerte (§9 du plan, « Alertes de drop ») — mêmes fichiers que le web
//! (`public/assets/sounds/{countdown-de76d898,alert-9d3d06ed}.mp3`, « fourni par l'utilisateur »
//! selon leur propre commentaire de source), embarqués ici pour ne dépendre d'aucun téléchargement
//! au premier lancement. Miroir réduit d'`AlertSoundService` (`alert-sound.service.ts`) : le web a
//! trois sons distincts (ramassage avec son activé, filtre de chat, décompte à 0) — seuls les deux
//! premiers sont câblés côté overlay (voir `overlay_engine::watchlist::WatchlistAlert` et
//! `overlay_engine::profile::LootAlert`), le filtre de chat restant hors périmètre (pas de panneau
//! chat côté overlay pour l'instant).

const COUNTDOWN_SOUND_BYTES: &[u8] = include_bytes!("../assets/sounds/countdown.mp3");
const LOOT_SOUND_BYTES: &[u8] = include_bytes!("../assets/sounds/loot.mp3");

/// Joue le son de décompte à 0 — voir `play_alert` pour les garanties (thread dédié, best-effort).
pub fn play_countdown_alert() {
    play_alert("décompte", COUNTDOWN_SOUND_BYTES);
}

/// Joue le son de ramassage (objet à son activé au compte, voir `overlay_engine::profile`) — voir
/// `play_alert` pour les garanties (thread dédié, best-effort).
pub fn play_loot_alert() {
    play_alert("ramassage", LOOT_SOUND_BYTES);
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
/// d'avertissement en cas d'échec, pour distinguer laquelle des deux alertes est en cause.
fn play_alert(label: &'static str, bytes: &'static [u8]) {
    std::thread::Builder::new()
        .name("overlay-alert-sound".into())
        .spawn(move || {
            if let Err(err) = play_blocking(bytes) {
                tracing::warn!(%err, "lecture du son d'alerte de {label} impossible");
            }
        })
        .expect("échec de création du thread de lecture audio");
}

fn play_blocking(bytes: &'static [u8]) -> Result<(), Box<dyn std::error::Error>> {
    let stream_handle = rodio::OutputStreamBuilder::open_default_stream()?;
    let sink = rodio::Sink::connect_new(stream_handle.mixer());
    let cursor = std::io::Cursor::new(bytes);
    let source = rodio::Decoder::new(cursor)?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}
