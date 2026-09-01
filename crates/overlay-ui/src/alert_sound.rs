//! Son d'alerte de décompte à 0 (§9 du plan, « Alertes de drop ») — même fichier que le web
//! (`public/assets/sounds/countdown-de76d898.mp3`, « fourni par l'utilisateur » selon son propre
//! commentaire de source), embarqué ici pour ne dépendre d'aucun téléchargement au premier
//! lancement. Miroir volontairement réduit d'`AlertSoundService` (`alert-sound.service.ts`) : le
//! web a trois sons distincts (ramassage avec son activé, filtre de chat, décompte à 0) — seul le
//! troisième est câblé côté overlay pour cette itération (voir la doc de
//! `overlay_engine::watchlist::WatchlistAlert`), les deux autres dépendant de réglages `profile`
//! du compte pas encore lus ici.

const COUNTDOWN_SOUND_BYTES: &[u8] = include_bytes!("../assets/sounds/countdown.mp3");

/// Joue le son de décompte, sur un thread dédié et JAMAIS bloquant pour l'appelant (voir
/// `main.rs::spawn_engine_thread`, qui appelle ceci depuis le thread Engine — bloquer là
/// retarderait l'ingestion du log). Un thread par lecture plutôt qu'un flux audio persistant :
/// les alertes sont rares (un décompte de suivi qui atteint 0), le coût d'ouvrir/fermer le
/// périphérique audio à chaque fois est négligeable en pratique et évite de garder une ressource
/// système ouverte en permanence pour un besoin aussi occasionnel (budget mémoire §8 du plan).
/// Best-effort : un périphérique audio indisponible (ou l'échec du décodage) laisse l'alerte
/// visuelle (toast) fonctionner seule, jamais une raison de faire échouer quoi que ce soit
/// d'autre — même philosophie que `AlertSoundService.play` côté web (`catch` silencieux).
pub fn play_countdown_alert() {
    std::thread::Builder::new()
        .name("overlay-alert-sound".into())
        .spawn(|| {
            if let Err(err) = play_blocking() {
                tracing::warn!(%err, "lecture du son d'alerte de décompte impossible");
            }
        })
        .expect("échec de création du thread de lecture audio");
}

fn play_blocking() -> Result<(), Box<dyn std::error::Error>> {
    let stream_handle = rodio::OutputStreamBuilder::open_default_stream()?;
    let sink = rodio::Sink::connect_new(stream_handle.mixer());
    let cursor = std::io::Cursor::new(COUNTDOWN_SOUND_BYTES);
    let source = rodio::Decoder::new(cursor)?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}
