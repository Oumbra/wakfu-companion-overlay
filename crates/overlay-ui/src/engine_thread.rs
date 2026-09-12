//! Thread Engine (§3 du plan) — voir la doc de `lib.rs` pour pourquoi ce module est partagé entre
//! les deux binaires (`main.rs` Windows, `bin/overlay-ui-x11.rs` Linux) : rien ici ne dépend de
//! l'OS, seuls les threads compte lié (Auth/Sync/Catalogue, lot L4-L5) restent Windows-only pour
//! l'instant — le binaire Linux tourne pour l'instant uniquement en mode invité (`catalog`/
//! `dungeons` vides, `sync_tx` vers un canal dont rien ne lit jamais le receveur).
//!
//! Lit `wakfu.log` en continu, alimente `overlay-engine`, publie chaque nouveau `SessionSnapshot`
//! par `ArcSwap` et réveille le thread principal. Ne rappelle jamais l'UI directement — l'UI ne
//! lit que la dernière valeur publiée (§3 : « zéro verrou sur le chemin de rendu »). Un seul
//! thread/`Engine` pour toutes les fenêtres overlay : `wakfu.log` est partagé par tous les clients,
//! `SessionSnapshot::fights` porte déjà tous les combats simultanés.

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use arc_swap::ArcSwap;
use crossbeam_channel::RecvTimeoutError;
use overlay_engine::{
    CatalogIndex, DungeonIndex, Engine, SessionSnapshot, WatchlistEntry, WatchlistKind,
};
use overlay_sync::AccountSettings;
use winit::event_loop::EventLoopProxy;

use crate::alert_sound;
use crate::panels;
use crate::panels::watchlist::{WatchlistToast, WatchlistToastReason};
use crate::render_content::UserEvent;

/// Message transmis au thread Engine sur le même canal que les réglages de compte récupérés
/// (`AccountSettings`) — `Disconnect` (déconnexion volontaire) n'est PAS juste une absence de
/// réglages : il doit activement effacer le roster/suivi déjà appliqués (repli `breed`, Suivi
/// vidé), ce qu'un simple silence sur le canal ne ferait jamais.
pub enum EngineCommand {
    ApplySettings(AccountSettings),
    Disconnect,
    /// Nouveau chemin de `wakfu.log` à suivre, choisi par l'utilisateur via la modale Options
    /// (2026-09-08, §5.1/§9 du plan) — voir `panels::options_modal`. Traité en respawnant SEULEMENT
    /// le watcher (`overlay_ingest::watcher::spawn`) sur le nouveau chemin, jamais en recréant
    /// l'`Engine` : le rattrapage `is_initial_load=true` d'un tailer flambant neuf resynchronise
    /// déjà le PARSER (§5.3 du plan — vrai à CHAQUE rattrapage, rotation ou changement de chemin,
    /// pas seulement au premier), et conserver le roster/watchlist/sound_items déjà appliqués évite
    /// de perdre la configuration du compte lié pour un simple changement d'emplacement de fichier.
    /// `state` (combats/totaux Rust) suit alors la même règle que pour une rotation classique —
    /// écart déjà assumé et documenté au §5.3, pas une régression propre à ce nouveau cas.
    ChangeLogPath(PathBuf),
}

/// Message transmis au thread Sync (lot L5, §7.3 du plan) — `Activate`/`Deactivate` suivent
/// exactement `AuthCommand::Retry`/`Disconnect` (compte lié ⇒ file active, mode invité ⇒ rien ne
/// quitte la machine) ; `Enqueue` relaie les événements produits par `Engine::drain_sync_events`
/// après chaque lot ingéré — jamais bloquant pour ce dernier, l'écriture SQLite et l'envoi réseau
/// se font entièrement sur le thread Sync. `SyncWatchlist` relaie de même
/// `Engine::drain_watchlist_sync` (compteurs de Suivi, §14 point 3 du plan, chantier fermé le
/// 2026-09-07) — MÊME thread, mais mécanisme distinct de `Enqueue`/`SyncQueue` : pas de file
/// SQLite ni de `client_key` idempotent, un simple `PATCH /api/v1/settings` « dernier écrivain
/// gagne », débounce et backoff gérés côté thread Sync (voir `spawn_sync_thread`).
pub enum SyncCommand {
    Activate { uid: String, token: String },
    Deactivate,
    Enqueue(Vec<overlay_engine::SyncEvent>),
    SyncWatchlist(Vec<WatchlistEntry>),
}

/// Regroupe les `Arc<ArcSwap<_>>` partagés avec le reste de l'app que le thread Engine consomme —
/// factorisé pour que `spawn_engine_thread` reste sous la limite `clippy::too_many_arguments`.
pub struct EngineHandles {
    pub snapshot: Arc<ArcSwap<SessionSnapshot>>,
    pub watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
    pub watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
    pub catalog: Arc<ArcSwap<CatalogIndex>>,
    /// Référentiel des donjons (L5, §7.1 du plan) — voir le thread Catalogue. Contrairement au
    /// catalogue, aucun panneau ne le consomme directement : seul l'Engine s'en sert, pour la
    /// synchro serveur.
    pub dungeons: Arc<ArcSwap<DungeonIndex>>,
}

pub fn spawn_engine_thread(
    log_path: PathBuf,
    handles: EngineHandles,
    proxy: EventLoopProxy<UserEvent>,
    settings_rx: mpsc::Receiver<EngineCommand>,
    sync_tx: mpsc::Sender<SyncCommand>,
) {
    let EngineHandles {
        snapshot,
        watchlist,
        watchlist_toast,
        catalog,
        dungeons,
    } = handles;
    thread::Builder::new()
        .name("overlay-engine".into())
        .spawn(move || {
            let mut engine = match Engine::new() {
                Ok(engine) => engine,
                Err(err) => {
                    tracing::error!("[erreur fatale] création de l'Engine QuickJS : {err}");
                    return;
                }
            };
            // Dernier catalogue/référentiel de donjons déjà transmis à l'Engine (voir
            // `Engine::set_catalog`/`set_dungeons`) — comparés par pointeur à chaque tick pour ne
            // relayer qu'un VRAI changement. Pour le catalogue, protège
            // `hostIsKnownMonsterName` (voir `quickjs_engine.rs`) contre un vrai monstre qui se
            // révèle (mimique, brèche) confondu à tort avec une invocation.
            let mut last_seen_catalog: Option<Arc<CatalogIndex>> = None;
            let mut last_seen_dungeons: Option<Arc<DungeonIndex>> = None;
            // `mut` depuis `EngineCommand::ChangeLogPath` (2026-09-08, voir sa doc) : un nouveau
            // chemin respawne un tailer flambant neuf sur CE `rx`, remplaçant l'ancien récepteur —
            // l'ancien thread watcher se termine de lui-même dès que son émetteur est abandonné ici.
            let mut rx = overlay_ingest::watcher::spawn(&log_path);
            loop {
                // Non bloquant : n'attend jamais activement les réglages de compte, seulement les
                // lignes de log (voir recv_timeout plus bas) — un compte jamais lié ne doit pas
                // retarder l'ingestion d'un seul milliseconde.
                while let Ok(command) = settings_rx.try_recv() {
                    match command {
                        EngineCommand::ApplySettings(settings) => {
                            let entry_count = settings.watchlist.len();
                            let sound_item_count = settings.alerts.sound_items.len();
                            tracing::info!(
                                entry_count,
                                sound_item_count,
                                "réglages de compte appliqués à l'Engine (lot L4)"
                            );
                            engine.set_roster(Some(settings.roster));
                            engine.set_watchlist_entries(settings.watchlist);
                            engine.set_sound_items(settings.alerts.sound_items);
                        }
                        // Déconnexion volontaire : repli mode invité — plus de roster connu
                        // (classification retombe sur `breed`), Suivi vidé (la LISTE suivie est
                        // lue depuis le compte ; sans compte, il n'y a plus de liste à afficher),
                        // plus aucun son de ramassage activé (même raison). Les compteurs déjà
                        // persistés sur disque ne sont pas effacés : une reconnexion ultérieure au
                        // MÊME compte les retrouve (`merge_config`).
                        EngineCommand::Disconnect => {
                            tracing::info!(
                                "compte déconnecté — Engine repasse en mode invité (repli `breed`, Suivi vidé)"
                            );
                            engine.set_roster(None);
                            engine.set_watchlist_entries(Vec::new());
                            engine.set_sound_items(Vec::new());
                        }
                        EngineCommand::ChangeLogPath(new_path) => {
                            tracing::info!(
                                "[options] nouveau fichier de log : {} (ancien thread watcher \
                                 abandonné, rattrapage complet du nouveau fichier)",
                                new_path.display()
                            );
                            rx = overlay_ingest::watcher::spawn(&new_path);
                        }
                    }
                    watchlist.store(Arc::new(engine.watchlist_entries().to_vec()));
                    // Rattrapage éventuel (voir `WatchlistState::merge_config`) : un compte qui
                    // répond avec un `count` en retard sur le local doit être resynchronisé —
                    // jamais après `Disconnect` (watchlist vidée, rien à rattraper), voir
                    // `Engine::drain_watchlist_sync`.
                    if let Some(entries) = engine.drain_watchlist_sync() {
                        let _ = sync_tx.send(SyncCommand::SyncWatchlist(entries));
                    }
                    let _ = proxy.send_event(UserEvent::NewSnapshot);
                }
                let current_catalog = catalog.load_full();
                let already_seen = last_seen_catalog
                    .as_ref()
                    .is_some_and(|seen| Arc::ptr_eq(seen, &current_catalog));
                if !already_seen {
                    engine.set_catalog(Arc::clone(&current_catalog));
                    last_seen_catalog = Some(current_catalog);
                }
                let current_dungeons = dungeons.load_full();
                let dungeons_already_seen = last_seen_dungeons
                    .as_ref()
                    .is_some_and(|seen| Arc::ptr_eq(seen, &current_dungeons));
                if !dungeons_already_seen {
                    engine.set_dungeons(Arc::clone(&current_dungeons));
                    last_seen_dungeons = Some(current_dungeons);
                }
                match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                    Ok(Ok(batch)) => {
                        if let Err(err) = engine.ingest_batch(&batch) {
                            tracing::warn!(%err, "échec d'ingestion d'un lot, ligne(s) ignorée(s)");
                            continue;
                        }
                        snapshot.store(Arc::new(engine.snapshot()));
                        watchlist.store(Arc::new(engine.watchlist_entries().to_vec()));
                        // L5, §7.1 du plan : relayé tel quel au thread Sync, qu'un compte soit lié
                        // ou non — `SyncCommand::Enqueue` en mode invité est simplement ignoré par
                        // ce dernier (aucune écriture réseau), jamais retenu ici. Ne bloque jamais
                        // ce thread : l'écriture SQLite/l'envoi vivent ailleurs.
                        let sync_events = engine.drain_sync_events();
                        if !sync_events.is_empty() {
                            let _ = sync_tx.send(SyncCommand::Enqueue(sync_events));
                        }
                        // Compteurs de Suivi (§14 point 3 du plan, chantier fermé le 2026-09-07) —
                        // même relais sans blocage, mécanisme distinct de `Enqueue` ci-dessus (voir
                        // la doc de `SyncCommand::SyncWatchlist`).
                        if let Some(entries) = engine.drain_watchlist_sync() {
                            let _ = sync_tx.send(SyncCommand::SyncWatchlist(entries));
                        }
                        for alert in engine.drain_watchlist_alerts() {
                            tracing::info!(name = %alert.name, "alerte de suivi (décompte à 0)");
                            alert_sound::play_countdown_alert();
                            let created_at = std::time::Instant::now();
                            watchlist_toast.store(Arc::new(Some(WatchlistToast {
                                name: alert.name,
                                kind: alert.kind,
                                reason: WatchlistToastReason::Countdown,
                                catalog_id: alert.catalog_id,
                                created_at,
                                confetti: panels::watchlist::build_confetti(),
                                hide_at: created_at + panels::watchlist::TOAST_DURATION,
                            })));
                        }
                        // Ramassage d'un objet à son activé (compte) — INDÉPENDANT de la
                        // watchlist ci-dessus, même mécanisme de toast (un seul emplacement
                        // affiché à la fois : le plus récent des deux écrase l'autre, jamais de
                        // file d'attente — acceptable, ces alertes sont rares et ≤ 5 s chacune).
                        for alert in engine.drain_loot_alerts() {
                            tracing::info!(
                                name = %alert.name,
                                quantity = alert.quantity,
                                "alerte de ramassage (son activé)"
                            );
                            alert_sound::play_loot_alert();
                            let created_at = std::time::Instant::now();
                            watchlist_toast.store(Arc::new(Some(WatchlistToast {
                                name: alert.name,
                                kind: WatchlistKind::Item,
                                reason: WatchlistToastReason::Loot {
                                    quantity: alert.quantity,
                                },
                                catalog_id: alert.catalog_id,
                                created_at,
                                confetti: panels::watchlist::build_confetti(),
                                hide_at: created_at + panels::watchlist::TOAST_DURATION,
                            })));
                        }
                        let _ = proxy.send_event(UserEvent::NewSnapshot);
                    }
                    Ok(Err(err)) => tracing::warn!(%err, "erreur de lecture de wakfu.log"),
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break, // watcher arrêté (process en fin de vie)
                }
            }
        })
        .expect("échec de création du thread Engine");
}
