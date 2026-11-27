//! **Threads de fond partagés par les deux binaires** (2026-09-14) : compte (`spawn_auth_thread`),
//! file de synchro (`spawn_sync_thread`), référentiel embarqué (`spawn_referentiel_thread`),
//! historique local (`spawn_history_thread`). Ils
//! vivaient
//! dans `main.rs` (Windows) depuis les lots L3-L5 ; rien n'y dépend de l'OS — `std::thread`,
//! `mpsc`, `ArcSwap`, `overlay_sync` et un `EventLoopProxy` — et le binaire Linux
//! (`bin/wakfu-companion-overlay-x11.rs`) en a besoin depuis que la fenêtre de connexion lui est
//! portée (§9.1 undecies du plan) : sortis ici tels quels, doc comprise, plutôt que dupliqués.
//!
//! Chacun signale au [`crate::startup::StartupProgress`] partagé la fin de son chargement initial,
//! ce qui fait tomber l'écran de chargement de la fenêtre de connexion (voir `panels::login`).

use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use arc_swap::ArcSwap;
use overlay_engine::{CatalogIndex, DungeonIndex, WatchlistEntry};

use winit::event_loop::EventLoopProxy;

use crate::engine_thread::{EngineCommand, SyncCommand};
use crate::render_content::{AuthCommand, AuthFailure, AuthStatus, UserEvent};
use crate::startup::StartupProgress;

/// Thread Référentiel (2026-11-27, `docs/plan-historique-local.md` §4) : construit les index du
/// référentiel **embarqué** (`overlay_engine::referentiel`) — objets, monstres, recettes, donjons —
/// puis les publie. Hors du main thread : c'est ~1 Mo de JSON à analyser au lancement.
///
/// Remplace les anciens threads Catalogue et Donjons, qui interrogeaient l'API du site
/// (`GET /api/v1/catalog/*`, `/dungeons`) derrière un cache disque et un repli embarqué daté :
/// le référentiel suit désormais les releases, il n'y a plus ni réseau, ni cache, ni repli, ni
/// session à attendre.
///
/// Le MÊME `Arc<ArcSwap<CatalogIndex>>` est relayé au thread Engine (voir `spawn_engine_thread`,
/// `Engine::set_catalog`) : il y sert de garde-fou `hostIsKnownMonsterName` (`quickjs_engine.rs`)
/// contre un vrai monstre qui se révèle (mimique, brèche) confondu à tort avec une invocation.
pub fn spawn_referentiel_thread(
    catalog: Arc<ArcSwap<CatalogIndex>>,
    dungeons: Arc<ArcSwap<DungeonIndex>>,
    startup: Arc<StartupProgress>,
    proxy: EventLoopProxy<UserEvent>,
) {
    thread::Builder::new()
        .name("overlay-referentiel".into())
        .spawn(move || {
            let started = Instant::now();
            let referentiel = overlay_engine::referentiel::load();
            tracing::info!(
                elapsed_ms = started.elapsed().as_millis() as u64,
                "référentiel embarqué chargé"
            );
            catalog.store(Arc::new(referentiel.catalog));
            dungeons.store(Arc::new(referentiel.dungeons));
            startup.mark_catalog();
            startup.mark_dungeons();
            let _ = proxy.send_event(UserEvent::NewSnapshot);
            let _ = proxy.send_event(UserEvent::StartupProgress);
            crate::local_data::remove_obsolete_caches();
        })
        .expect("échec de création du thread Référentiel");
}

/// Thread Historique (2026-11-27, `docs/plan-historique-local.md` §6.2) : écrit dans
/// `data/history.sqlite3` (`overlay_history`) chaque lot d'événements que le thread moteur lui
/// relaie — combats, achats, échanges, pactes —, puis rattache les salles de donjon à leur run et
/// classe les combats. Idempotent : le rattrapage de tout `wakfu.log` à chaque lancement réécrit
/// les mêmes événements sans rien ajouter.
///
/// Rend l'émetteur à confier au thread moteur (`EngineHandles::history`).
///
/// **La base n'est ouverte que le temps d'écrire un lot**, puis refermée : un fichier SQLite
/// ouvert ne peut pas être supprimé sous Windows, et « Supprimer les données locales »
/// (`local_data::purge`) doit pouvoir emporter l'historique avec le reste du dossier. Les lots
/// arrivent au rythme des fins de combat : l'ouverture (quelques millisecondes) ne pèse rien, et
/// les lots en attente sont fusionnés en une seule écriture.
///
/// Les recalculs ont besoin du référentiel : le premier lot (le rattrapage du log, au lancement)
/// attend qu'il soit chargé (`spawn_referentiel_thread`, ~40 ms), cinq secondes au plus.
///
/// Best-effort, comme le reste des threads de fond : un échec d'écriture est journalisé, l'overlay
/// continue ; le prochain lancement réécrira ce que le log contient encore.
pub fn spawn_history_thread(
    catalog: Arc<ArcSwap<CatalogIndex>>,
    dungeons: Arc<ArcSwap<DungeonIndex>>,
) -> mpsc::Sender<Vec<overlay_engine::SyncEvent>> {
    let (tx, rx) = mpsc::channel::<Vec<overlay_engine::SyncEvent>>();
    thread::Builder::new()
        .name("overlay-history".into())
        .spawn(move || {
            let path = match overlay_history::HistoryStore::default_path() {
                Ok(path) => path,
                Err(err) => {
                    tracing::warn!(%err, "historique local indisponible");
                    return;
                }
            };
            while let Ok(first) = rx.recv() {
                let mut events = first;
                while let Ok(more) = rx.try_recv() {
                    events.extend(more);
                }
                wait_for_referentiel(&catalog);
                let catalog = catalog.load();
                let dungeons = dungeons.load();
                let started = Instant::now();
                let result = overlay_history::HistoryStore::open(&path).and_then(|mut store| {
                    store.record(
                        &events,
                        overlay_history::Referentiels {
                            catalog: &catalog,
                            dungeons: &dungeons,
                        },
                    )
                });
                match result {
                    Ok(report) => tracing::debug!(
                        evenements = events.len(),
                        combats = report.fights_inserted,
                        achats = report.purchases_inserted,
                        echanges = report.trades_inserted,
                        pactes = report.pacts_inserted,
                        rattaches = report.dungeon_attached,
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "historique local écrit"
                    ),
                    Err(err) => tracing::warn!(%err, "écriture de l'historique local impossible"),
                }
            }
        })
        .expect("échec de création du thread Historique");
    tx
}

/// Attend que le thread Référentiel ait publié le catalogue — voir `spawn_history_thread`.
fn wait_for_referentiel(catalog: &ArcSwap<CatalogIndex>) {
    let deadline = Instant::now() + std::time::Duration::from_secs(5);
    while catalog.load().is_empty() && Instant::now() < deadline {
        thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// Thread Sync (lot L5, §7.3 du plan) : possède la file SQLite persistante (`overlay_sync::
/// SyncQueue`) — écriture (`enqueue`) ET envoi réseau (`flush_once`) vivent entièrement ici, jamais
/// sur le thread Engine (voir `spawn_engine_thread`, qui ne fait que relayer des `SyncEvent` déjà
/// sérialisés sans jamais attendre dessus) ni sur le main thread. Best-effort à l'ouverture de la
/// file elle-même (dossier de données/SQLite indisponible) : le thread se termine simplement,
/// l'overlay continue sans synchro plutôt que de planter — même philosophie que
/// `spawn_auth_thread`.
///
/// **Simplification assumée par rapport à `SyncQueueService` côté web** : pas de debounce explicite
/// de 2 s avant un envoi (`FLUSH_DEBOUNCE_MS`) — chaque `SyncCommand::Enqueue` tente un
/// `flush_once` immédiatement après écriture. Les entrées ne sont de toute façon jamais perdues
/// (persistées avant tout envoi), un lot ingéré produit rarement plus d'une poignée d'événements à
/// la fois (contrairement au rattrapage initial d'un `wakfu.log` entier, qui arrive lui aussi par
/// lots ≤ 2 000 lignes, voir `overlay-ingest`), et `flush_once` traite de toute façon TOUT ce qui
/// est en file au moment de l'appel (pas seulement le dernier lot reçu) — la seule perte réelle est
/// un envoi réseau de plus qu'avec un vrai debounce, jamais un comportement incorrect.
///
/// Le délai avant le PROCHAIN passage (`wait`, argument de `recv_timeout`) encode à la fois
/// l'inactivité (aucun compte connu : 1 h, réveillé immédiatement par la prochaine commande) et le
/// backoff après un échec réessayable (`FlushOutcome::Retry`, voir `backoff_delay` — 15 s à 5 min,
/// doublé à chaque échec consécutif, miroir de `RETRY_BASE_DELAY_MS`/`RETRY_MAX_DELAY_MS` côté web).
///
/// **Le backoff tient même quand une commande réveille le thread** (audit de sécurité du
/// 2026-09-23, O3) : `next_history_attempt` retient l'instant du prochain envoi autorisé après un
/// échec. Un `Enqueue` ou un `SyncWatchlist` reçu avant cet instant met en file mais n'envoie rien
/// — sans quoi chaque combat terminé relançait un `flush_once` immédiat, `Retry-After` d'un 429
/// compris, et l'overlay martelait un serveur déjà en difficulté. Seul `Activate` (nouvelle
/// session) lève l'attente.
///
/// **Compteurs de Suivi (`SyncCommand::SyncWatchlist`, §14 point 3 du plan, chantier fermé le
/// 2026-09-07)** — même thread, mécanisme VOLONTAIREMENT séparé de la file `SyncQueue` ci-dessus :
/// pas de file SQLite ni de `client_key` idempotent (la valeur ENTIÈRE remplace la clé côté
/// serveur, voir `functions/api/v1/settings.ts::onRequestPatch`, « dernier écrivain gagne »), donc
/// rien à dédupliquer — seul le DERNIER instantané reçu compte, porté en mémoire
/// (`pending_watchlist`) plutôt qu'en base. Débounce de `WATCHLIST_DEBOUNCE` avant le premier
/// essai (miroir de `WRITE_DEBOUNCE_MS` côté web, `RemoteUserDataRepository`) : un compteur qui
/// s'incrémente à chaque kill d'un combat ne doit pas déclencher une requête par kill. Un nouvel
/// instantané reçu PENDANT l'attente (débounce ou backoff) remplace le précédent et relance un
/// débounce complet — sans jamais avancer un backoff en cours (O3) : l'essai suivant a lieu au plus
/// tard des deux instants. Rien n'est perdu (`WatchlistState` garde de toute façon le fichier local
/// comme vérité, voir sa doc), seul le nombre de requêtes est réduit.
///
/// **Session refusée en cours de route** (401, 2026-09-23) : historique ou Suivi, le thread cesse
/// aussitôt d'envoyer (`account = None` — ce qui arrive ensuite patiente en mémoire, comme au
/// démarrage) et signale `AuthCommand::SessionExpired` au thread Auth par `auth_tx`, qui efface le
/// jeton et ramène la fenêtre de connexion. La file SQLite, elle, est **conservée** pour la
/// reconnexion ; `SyncQueue::claim_owner` la vide si c'est un autre compte qui se reconnecte.
pub fn spawn_sync_thread(
    command_rx: mpsc::Receiver<SyncCommand>,
    auth_tx: mpsc::Sender<AuthCommand>,
) {
    thread::Builder::new()
        .name("overlay-sync".into())
        .spawn(move || {
            let mut queue = match overlay_sync::SyncQueue::default_store_path() {
                Some(path) => match overlay_sync::SyncQueue::open(&path) {
                    Ok(queue) => queue,
                    Err(err) => {
                        tracing::warn!(
                            %err,
                            "file de synchro (SQLite) indisponible — historique non envoyé au compte cette session"
                        );
                        return;
                    }
                },
                None => {
                    tracing::warn!(
                        "impossible de résoudre le dossier de données de l'overlay — file de synchro désactivée"
                    );
                    return;
                }
            };

            // `uid` (pour `client_key`) ET `token` (pour authentifier l'envoi, voir
            // `SyncCommand::Activate`) — toujours mis à jour ensemble.
            let mut account: Option<(String, String)> = None;
            // **Ce qui arrive SANS compte reste en mémoire** (2026-09-18) : rien n'est écrit dans
            // `sync-queue.sqlite3` tant qu'aucun compte ne peut le recevoir. Le cas ordinaire est
            // le démarrage — le rattrapage de `wakfu.log` produit ses événements pendant que le
            // thread Auth résout encore l'`uid` — et ce tampon les fait patienter jusqu'à
            // `Activate`, qui les verse dans la file. L'autre cas est le mode invité, où ils
            // seraient sinon restés sur disque sans destinataire, avec les pseudonymes des autres
            // combattants (`docs/analyse-rgpd.md` §3.3). Borné par `HELD_EVENTS_CAP` : au-delà,
            // le plus ancien cède la place — perte sans conséquence, le rattrapage du prochain
            // lancement rejoue ce que `wakfu.log` contient encore.
            let mut held: std::collections::VecDeque<overlay_engine::SyncEvent> =
                std::collections::VecDeque::new();
            // Dernier instantané de Suivi reçu et pas encore répliqué avec succès (voir la doc de
            // `spawn_sync_thread` ci-dessus) — `None` tant qu'aucun `SyncCommand::SyncWatchlist`
            // n'est arrivé, ou après un envoi réussi.
            let mut pending_watchlist: Option<Vec<WatchlistEntry>> = None;
            // Instant à partir duquel retenter l'envoi watchlist (fin du débounce, ou du backoff
            // après un échec) — distinct du calcul `wait` de l'historique ci-dessous : les deux
            // mécanismes n'ont ni la même cause de délai ni le même état.
            let mut watchlist_ready_at: Option<std::time::Instant> = None;
            let mut watchlist_consecutive_failures: u32 = 0;
            // Prochain envoi d'historique autorisé après un échec réessayable (voir la doc
            // ci-dessus, O3) — `None` : rien n'empêche d'envoyer.
            let mut next_history_attempt: Option<std::time::Instant> = None;
            // Pas de compte connu au démarrage : n'attend qu'une commande, ne sonde jamais pour
            // rien (même philosophie que `settings_rx.try_recv()` côté thread Engine).
            let mut wait = std::time::Duration::from_secs(3600);
            loop {
                match command_rx.recv_timeout(wait) {
                    Ok(SyncCommand::Activate { uid, token }) => {
                        tracing::info!(
                            held = held.len(),
                            "file de synchro activée (compte connecté, lot L5)"
                        );
                        // Un autre compte que celui que la file servait (reconnexion après une
                        // session expirée) : ce qui attendait ne lui appartient pas.
                        match queue.claim_owner(&uid) {
                            Ok(0) => {}
                            Ok(dropped) => tracing::info!(
                                dropped,
                                "file de synchro vidée — elle appartenait à un autre compte"
                            ),
                            Err(err) => tracing::warn!(%err, "propriétaire de la file de synchro illisible"),
                        }
                        // Nouvelle session : un type suspendu (403) a droit à un nouvel essai, et
                        // l'envoi n'attend plus le backoff de la session précédente.
                        queue.unblock_all();
                        next_history_attempt = None;
                        account = Some((uid, token));
                        // Ce qui attendait depuis plus d'un mois n'a plus de sens — voir
                        // `MAX_PENDING_AGE` ; ensuite seulement, ce que le démarrage a retenu.
                        match queue.prune_older_than(overlay_sync::MAX_PENDING_AGE) {
                            Ok(0) | Err(_) => {}
                            Ok(removed) => tracing::info!(
                                removed,
                                "entrées d'historique abandonnées — en attente depuis plus d'un mois"
                            ),
                        }
                        enqueue_all(&queue, held.drain(..));
                    }
                    Ok(SyncCommand::Deactivate) => {
                        // Rien de ce qui attendait n'a plus de destinataire — ni en base, ni en
                        // mémoire (voir `held` et `SyncQueue::clear`).
                        let dropped = queue.clear().unwrap_or(0) + held.len();
                        held.clear();
                        tracing::info!(
                            dropped,
                            "file de synchro désactivée (compte déconnecté) — contenu en attente effacé"
                        );
                        account = None;
                    }
                    Ok(SyncCommand::Enqueue(events)) => {
                        if account.is_some() {
                            enqueue_all(&queue, events);
                        } else {
                            for event in events {
                                if held.len() >= HELD_EVENTS_CAP {
                                    held.pop_front();
                                }
                                held.push_back(event);
                            }
                        }
                    }
                    Ok(SyncCommand::SyncWatchlist(entries)) => {
                        pending_watchlist = Some(entries);
                        // Débounce relancé, mais jamais avant la fin d'un backoff en cours (O3) ;
                        // le compteur d'échecs ne repart de zéro qu'après un envoi réussi.
                        let debounced = std::time::Instant::now() + WATCHLIST_DEBOUNCE;
                        watchlist_ready_at = Some(match watchlist_ready_at {
                            Some(at) if watchlist_consecutive_failures > 0 => at.max(debounced),
                            _ => debounced,
                        });
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {} // réessai programmé (backoff) — retombe sur le flush ci-dessous
                    Err(mpsc::RecvTimeoutError::Disconnected) => break, // App fermée
                }

                let backoff = history_backoff_remaining(next_history_attempt, std::time::Instant::now());
                let history_wait = match (&account, backoff) {
                    (None, _) => std::time::Duration::from_secs(3600),
                    // Backoff en cours : les commandes reçues sont en file, rien n'est envoyé.
                    (Some(_), Some(remaining)) => remaining,
                    (Some((uid, token)), None) => {
                        next_history_attempt = None;
                        match queue.flush_once(uid, |path, body| {
                            overlay_sync::post_json_authenticated(token, path, body)
                        }) {
                            Ok(overlay_sync::FlushOutcome::Idle | overlay_sync::FlushOutcome::Synced) => {
                                std::time::Duration::from_secs(3600)
                            }
                            // **Correctif du 2026-09-03** : cet échec n'était auparavant tracé
                            // nulle part — un blocage persistant (429/réseau/5xx) restait
                            // invisible, backoff après backoff, jusqu'à 5 min entre essais.
                            // Un 429 impose son propre délai (`Retry-After`, plafonné à 1 h).
                            Ok(overlay_sync::FlushOutcome::Retry { reason, after }) => {
                                let delay = retry_delay(after, queue.consecutive_failures());
                                tracing::warn!(
                                    reason = %reason,
                                    consecutive_failures = queue.consecutive_failures(),
                                    delay_s = delay.as_secs(),
                                    "échec d'envoi de l'historique — nouvel essai après un délai"
                                );
                                next_history_attempt = Some(std::time::Instant::now() + delay);
                                delay
                            }
                            Ok(overlay_sync::FlushOutcome::Unauthorized(reason)) => {
                                session_refused(&mut account, &auth_tx, &reason);
                                std::time::Duration::from_secs(3600)
                            }
                            Err(err) => {
                                tracing::warn!(%err, "erreur de file de synchro (SQLite)");
                                let delay = std::time::Duration::from_secs(60);
                                next_history_attempt = Some(std::time::Instant::now() + delay);
                                delay
                            }
                        }
                    }
                };

                let watchlist_wait = match flush_watchlist_once(
                    &account,
                    &mut pending_watchlist,
                    &mut watchlist_ready_at,
                    &mut watchlist_consecutive_failures,
                ) {
                    WatchlistFlush::Wait(delay) => delay,
                    WatchlistFlush::Unauthorized(reason) => {
                        session_refused(&mut account, &auth_tx, &reason);
                        std::time::Duration::from_secs(3600)
                    }
                };

                wait = history_wait.min(watchlist_wait);
            }
        })
        .expect("échec de création du thread Sync");
}

/// **401 en cours de session** — voir la doc de `spawn_sync_thread` : plus rien ne part avec ce
/// jeton, le thread Auth est prévenu. Une seule fois par session : `account` repasse à `None`, et
/// seul un nouvel `Activate` le rétablit.
fn session_refused(
    account: &mut Option<(String, String)>,
    auth_tx: &mpsc::Sender<AuthCommand>,
    reason: &str,
) {
    if account.take().is_none() {
        return;
    }
    tracing::warn!(
        reason,
        "[compte] jeton refusé par le serveur (401) en cours de session — synchronisation suspendue, reconnexion proposée ; la file d'envoi est conservée"
    );
    let _ = auth_tx.send(AuthCommand::SessionExpired);
}

/// Temps restant avant le prochain envoi d'historique autorisé — `None` quand rien n'empêche
/// d'envoyer maintenant (aucun échec en cours, ou backoff écoulé). Voir `next_history_attempt`
/// dans `spawn_sync_thread` (O3).
fn history_backoff_remaining(
    next_attempt: Option<std::time::Instant>,
    now: std::time::Instant,
) -> Option<std::time::Duration> {
    next_attempt.filter(|at| *at > now).map(|at| at - now)
}

/// Délai avant le prochain passage après un échec réessayable : celui qu'impose le serveur
/// (`Retry-After` d'un 429), sinon le backoff ordinaire.
fn retry_delay(
    after: Option<std::time::Duration>,
    consecutive_failures: u32,
) -> std::time::Duration {
    after.unwrap_or_else(|| backoff_delay(consecutive_failures))
}

/// Nombre maximal d'événements d'historique retenus en mémoire sans compte connecté (voir `held`
/// dans `spawn_sync_thread`). Un `wakfu.log` d'une journée chargée en produit quelques centaines ;
/// cinq mille couvrent largement le rattrapage initial, pour quelques Mo au pire.
const HELD_EVENTS_CAP: usize = 5_000;

/// Écrit chaque événement dans la file SQLite — l'échec d'un seul n'arrête pas les autres, et se
/// contente d'un avertissement au journal (best-effort, comme l'ouverture de la file).
fn enqueue_all(
    queue: &overlay_sync::SyncQueue,
    events: impl IntoIterator<Item = overlay_engine::SyncEvent>,
) {
    for event in events {
        if let Err(err) = queue.enqueue(&event) {
            tracing::warn!(
                %err,
                kind = event.kind.as_str(),
                "échec d'enfilage d'un événement d'historique (SQLite)"
            );
        }
    }
}

/// Débounce avant le premier essai d'un envoi watchlist — miroir de `WRITE_DEBOUNCE_MS`
/// (`remote-user-data.repository.ts`) côté web.
const WATCHLIST_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(1_500);

/// Tente, si le moment est venu, de répliquer le dernier instantané de Suivi reçu — voir la doc de
/// `spawn_sync_thread` pour le mécanisme complet (débounce + backoff, sans file persistante).
/// Renvoie le délai avant le PROCHAIN passage utile pour CE mécanisme (à combiner par l'appelant
/// avec celui de l'historique, voir `wait`) : 1 h tant que rien n'est en attente ou qu'aucun
/// compte n'est connu (réveillé immédiatement par la prochaine commande), le temps restant avant
/// `ready_at` pendant un débounce/backoff en cours, ou le nouveau backoff après un échec — ou
/// `Unauthorized` sur un 401 (l'instantané reste en attente pour la reconnexion).
fn flush_watchlist_once(
    account: &Option<(String, String)>,
    pending: &mut Option<Vec<WatchlistEntry>>,
    ready_at: &mut Option<std::time::Instant>,
    consecutive_failures: &mut u32,
) -> WatchlistFlush {
    let Some(entries) = pending.as_ref() else {
        return WatchlistFlush::Wait(std::time::Duration::from_secs(3600));
    };
    let Some((_, token)) = account else {
        // pas de compte : rien à tenter avant `Activate`
        return WatchlistFlush::Wait(std::time::Duration::from_secs(3600));
    };
    let now = std::time::Instant::now();
    if let Some(at) = ready_at {
        if now < *at {
            return WatchlistFlush::Wait(*at - now);
        }
    }

    match overlay_sync::patch_watchlist(token, entries) {
        Ok(_) => {
            *pending = None;
            *ready_at = None;
            *consecutive_failures = 0;
            WatchlistFlush::Wait(std::time::Duration::from_secs(3600))
        }
        Err(err @ overlay_sync::SyncError::Http { status: 401, .. }) => {
            WatchlistFlush::Unauthorized(err.to_string())
        }
        Err(err) => {
            *consecutive_failures += 1;
            tracing::warn!(
                %err,
                consecutive_failures = *consecutive_failures,
                "échec de synchro des compteurs de Suivi — nouvel essai après un backoff"
            );
            let retry_after = match &err {
                overlay_sync::SyncError::Http { retry_after, .. } => *retry_after,
                _ => None,
            };
            let delay = retry_delay(retry_after, *consecutive_failures);
            *ready_at = Some(now + delay);
            WatchlistFlush::Wait(delay)
        }
    }
}

/// Ce que `flush_watchlist_once` demande au thread Sync.
enum WatchlistFlush {
    /// Prochain passage utile dans ce délai.
    Wait(std::time::Duration),
    /// 401 — voir `session_refused`.
    Unauthorized(String),
}

/// Miroir de `RETRY_BASE_DELAY_MS`/`RETRY_MAX_DELAY_MS`/le calcul de `scheduleRetry`
/// (`sync-queue.service.ts`) — 15 s doublés à chaque échec consécutif, plafonné à 5 min.
fn backoff_delay(consecutive_failures: u32) -> std::time::Duration {
    const BASE_MS: u64 = 15_000;
    const MAX_MS: u64 = 5 * 60_000;
    let exponent = consecutive_failures.saturating_sub(1).min(20); // évite un débordement de décalage
    let delay_ms = BASE_MS.saturating_mul(1u64 << exponent).min(MAX_MS);
    std::time::Duration::from_millis(delay_ms)
}

/// **Boucle de retentative** (2026-09-01, retour utilisateur : appairage en échec — 405 côté
/// serveur — sans aucun moyen de retenter sans relancer tout le logiciel) : une tentative échouée
/// publie `AuthStatus::Disconnected` plutôt que de laisser le thread mourir — la fenêtre de
/// connexion (`panels::login`) en déduit son écran, et ses boutons poussent `AuthCommand::Retry`
/// dans `command_rx` pour reprendre cette boucle.
///
/// **Plus d'appairage spontané (2026-09-14, §9.1 undecies du plan).** Au démarrage, seul un jeton
/// déjà stocké est essayé ; sans jeton (ou jeton refusé), le thread publie l'état neutre
/// `Disconnected { failure: None }` et attend — le navigateur ne s'ouvre que sur « Se
/// connecter », jamais tout seul au lancement. C'est `pair_if_needed`, vrai seulement après un
/// `Retry` explicite. Un échec réseau sur un jeton stocké le CONSERVE (voir `attempt_connect`) :
/// un lancement hors ligne ne doit pas effacer une session valide.
///
/// **Déconnexion volontaire** (2026-09-02, §14 point 3 du plan) : une connexion réussie ne fait
/// PAS terminer le thread — il reste à l'écoute de `command_rx` pour traiter un
/// `AuthCommand::Disconnect` (bouton « Déconnecter » de la fenêtre Options ou de l'icône de zone
/// de notification) tant que le compte reste lié. Un `Disconnect` efface le jeton
/// (`token_store::clear_token`), notifie le thread Engine (`EngineCommand::Disconnect`) et la
/// file de synchro, republie l'état neutre, puis attend un `Retry`. Chaque commande hors de son
/// état pertinent est silencieusement ignorée.
///
/// **Annulation d'un appairage** : pendant `pair_and_wait`, le thread ne lit pas `command_rx`
/// ici mais par la fermeture `should_cancel` — `CancelPairing` (lien de la fenêtre) ou
/// `Disconnect` (menu de zone de notification) y valent abandon, retour à l'état neutre.
pub fn spawn_auth_thread(
    settings_tx: mpsc::Sender<EngineCommand>,
    sync_tx: mpsc::Sender<SyncCommand>,
    status: Arc<ArcSwap<AuthStatus>>,
    command_rx: mpsc::Receiver<AuthCommand>,
    proxy: EventLoopProxy<UserEvent>,
) {
    thread::Builder::new()
        .name("overlay-auth".into())
        .spawn(move || {
            let mut pair_if_needed = false;
            loop {
                status.store(Arc::new(AuthStatus::Connecting));
                let _ = proxy.send_event(UserEvent::AuthStatusChanged);

                let result = attempt_connect(
                    pair_if_needed,
                    &settings_tx,
                    &sync_tx,
                    &status,
                    &command_rx,
                    &proxy,
                );

                let mut connected = result.is_ok();
                status.store(Arc::new(match result {
                    Ok(()) => AuthStatus::Connected,
                    Err(AttemptEnd::Idle) => AuthStatus::Disconnected { failure: None },
                    Err(AttemptEnd::Failed(failure)) => AuthStatus::Disconnected {
                        failure: Some(failure),
                    },
                }));
                let _ = proxy.send_event(UserEvent::AuthStatusChanged);

                // Attend la commande qui justifie de reprendre la boucle externe : `Retry`
                // seulement si PAS déjà connecté, jamais de nouvelle tentative automatique.
                loop {
                    match command_rx.recv() {
                        Ok(AuthCommand::Retry) if !connected => {
                            pair_if_needed = true;
                            break;
                        }
                        Ok(AuthCommand::Disconnect) if connected => {
                            // **Le serveur d'abord, le disque ensuite** (2026-09-18, constat C5) :
                            // c'est le jeton qui désigne la session à effacer côté serveur, et
                            // `clear_token` le fait disparaître. L'ordre inverse laisserait une
                            // session valide en base sans plus aucun moyen de la nommer.
                            crate::local_data::revoke_server_session();
                            overlay_sync::token_store::clear_token();
                            let _ = settings_tx.send(EngineCommand::Disconnect);
                            let _ = sync_tx.send(SyncCommand::Deactivate);
                            connected = false;
                            status.store(Arc::new(AuthStatus::Disconnected { failure: None }));
                            let _ = proxy.send_event(UserEvent::AuthStatusChanged);
                            tracing::info!(
                                "[compte] déconnecté — jeton effacé, retour à la fenêtre de connexion."
                            );
                            // **Les données locales à tiers partent avec le compte** (2026-09-18,
                            // constat C5 de `docs/analyse-rgpd.md`) : combats en cours, récap
                            // de session, compteurs de Suivi, gabarits de tour, et le contenu des
                            // journaux. APRÈS les deux commandes ci-dessus, jamais avant : le
                            // moteur oublie sa session sur `EngineCommand::Disconnect`
                            // (`forget_session`) et le thread de synchro vide sa file sur
                            // `Deactivate` — purger d'abord laisserait chacun réécrire ce qu'il
                            // tient encore en mémoire. Le récap, lui, appartient à l'hôte, qui
                            // l'efface en voyant le statut changer
                            // (`recap_session::RecapSession::purge`).
                            let _ = crate::local_data::purge(crate::local_data::Scope::OnDisconnect);
                        }
                        Ok(AuthCommand::SessionExpired) if connected => {
                            // **Jeton refusé en cours de session** (401 vu par le thread Sync,
                            // 2026-09-23) : même issue que le 401 au démarrage (`attempt_connect`)
                            // — jeton effacé, état neutre, la fenêtre de connexion revient sur
                            // « Se connecter » et un clic relance l'appairage (`pair_if_needed`).
                            // Rien d'autre n'est effacé : pas de révocation serveur (le jeton n'y
                            // est déjà plus valide), ni purge des données locales — ce n'est pas
                            // l'utilisateur qui a demandé à partir. La file d'envoi attend la
                            // reconnexion (voir `spawn_sync_thread`).
                            overlay_sync::token_store::clear_token();
                            connected = false;
                            status.store(Arc::new(AuthStatus::Disconnected { failure: None }));
                            let _ = proxy.send_event(UserEvent::AuthStatusChanged);
                            tracing::warn!(
                                "[compte] session refusée par le serveur (révoquée ou expirée) — jeton effacé, reconnexion proposée."
                            );
                        }
                        Ok(_) => continue, // commande sans effet dans l'état courant — ignorée
                        Err(_) => return,  // App fermée (canal fermé avec l'émetteur) — rien à faire.
                    }
                }
            }
        })
        .expect("échec de création du thread Auth");
}

/// Comment une tentative de connexion s'est terminée sans compte lié — voir `attempt_connect`.
enum AttemptEnd {
    /// Rien à reprocher à personne : pas de jeton et pas de « Se connecter » encore, jeton refusé
    /// par le serveur (session révoquée), ou appairage annulé. La fenêtre de connexion montre son
    /// écran d'accueil.
    Idle,
    /// La tentative a échoué pour une raison à montrer (serveur injoignable, appairage expiré…).
    Failed(AuthFailure),
}

/// Titre court + détail technique d'un échec, pour la fenêtre de connexion — `step` dit ce que
/// l'overlay faisait (« la validation de la session », « l'appairage »…). Jamais le jeton dans le
/// détail (§10 du plan) : `SyncError` ne le porte pas.
fn auth_failure(err: &overlay_sync::SyncError, step: &str) -> AuthFailure {
    use overlay_sync::SyncError;
    let headline = match err {
        SyncError::Network(_) => "Serveur injoignable",
        SyncError::Http { .. } => "Réponse inattendue du serveur",
        SyncError::Json(_) => "Réponse illisible du serveur",
        SyncError::PairingExpired => "Appairage expiré",
        SyncError::PairingCancelled => "Appairage annulé",
        SyncError::TokenStore(_) => "Stockage de la session impossible",
    };
    AuthFailure {
        headline: headline.to_string(),
        detail: format!("{step} — {err}"),
    }
}

/// Âge au-delà duquel un jeton natif stocké est **renouvelé au démarrage** (voir
/// [`rotate_token_if_due`]) — 7 jours, contre 30 de validité glissante : un jeton copié n'ouvre
/// donc plus rien une semaine plus tard, sans que l'utilisateur ait rien à faire, et un poste
/// lancé chaque jour n'appelle la route qu'une fois par semaine.
const TOKEN_ROTATION_AGE: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 3600);

/// **Renouvelle le jeton s'il a plus de [`TOKEN_ROTATION_AGE`]** — ou si son âge est inconnu
/// (sauvegardé avant que la date ne soit tenue, 2026-09-19) — et rend le jeton à utiliser pour la
/// suite de la session (`POST /api/v1/auth/native/session`, constat C5 de `docs/analyse-rgpd.md`
/// §3.5 ; côté serveur, `server/auth/pairing.ts::rotateNativeSession`).
///
/// Appelé **entre** la validation du jeton stocké (`GET /api/v1/settings` réussi) et l'activation
/// de la file d'envoi — c'est le seul endroit du processus qui garde un jeton en mémoire
/// (`SyncCommand::Activate`), et il reçoit ainsi le nouveau d'emblée ; tout le reste (`local_data`,
/// validations de la fenêtre Options) relit le trousseau à chaque appel. La grâce de 5 min laissée
/// à l'ancien jeton n'a donc rien à couvrir ici que d'éventuelles requêtes déjà parties.
///
/// **Le nouveau jeton est persisté AVANT d'être utilisé**, et un échec n'est jamais une
/// déconnexion : la route absente (déploiement pas encore à jour), un 401 ou une panne réseau
/// laissent l'ancien jeton en place, qui vient d'être accepté — ce n'est pas lui qui est en cause.
/// Si le serveur a répondu mais que la sauvegarde échoue, le nouveau jeton sert quand même pour
/// cette session (l'ancien meurt dans 5 min) ; il sera redemandé au prochain lancement, comme
/// après n'importe quel échec de `save_token`.
fn rotate_token_if_due(token: String) -> String {
    match overlay_sync::token_store::token_age() {
        Some(age) if age < TOKEN_ROTATION_AGE => {
            tracing::debug!(
                age_days = age.as_secs() / 86_400,
                "[compte] jeton natif récent, pas de rotation"
            );
            return token;
        }
        Some(age) => tracing::info!(
            age_days = age.as_secs() / 86_400,
            "[compte] jeton natif de plus de 7 jours — renouvellement."
        ),
        None => {
            tracing::info!("[compte] date d'émission du jeton natif inconnue — renouvellement.")
        }
    }
    match overlay_sync::client::rotate_native_session(&token) {
        Ok(rotated) => {
            if let Err(err) = overlay_sync::token_store::save_token(&rotated.token) {
                tracing::warn!(
                    "[compte] nouveau jeton natif non sauvegardé ({err}) — utilisé pour cette session, sera redemandé au prochain lancement."
                );
            }
            tracing::info!(
                expires_at = %rotated.expires_at,
                previous_valid_until = %rotated.previous_token_valid_until,
                "[compte] jeton natif renouvelé — l'ancien n'est plus accepté que quelques minutes."
            );
            rotated.token
        }
        Err(err) => {
            tracing::warn!(
                "[compte] renouvellement du jeton natif impossible ({err}) — jeton actuel conservé, prochaine tentative au prochain lancement."
            );
            token
        }
    }
}

/// Une tentative complète de connexion au compte : jeton déjà stocké et encore valide, sinon —
/// et seulement si `pair_if_needed` — nouvel appairage. `Ok(())` si les réglages de compte
/// (roster + watchlist) ont bien été récupérés et transmis (`settings_tx`) ; sinon `AttemptEnd`
/// dit si c'est un échec à afficher ou un simple état neutre (voir sa doc).
///
/// **Le jeton n'est effacé que sur refus du serveur** (401/403) : un serveur injoignable garde la
/// session pour la prochaine tentative (« Réessayer », ou le prochain lancement). `status`/`proxy`
/// servent uniquement à publier `AuthStatus::PairingStarted` dès que le code est connu ;
/// `spawn_auth_thread` publie lui-même `Connecting`/`Connected`/`Disconnected` autour de l'appel.
fn attempt_connect(
    pair_if_needed: bool,
    settings_tx: &mpsc::Sender<EngineCommand>,
    sync_tx: &mpsc::Sender<SyncCommand>,
    status: &Arc<ArcSwap<AuthStatus>>,
    command_rx: &mpsc::Receiver<AuthCommand>,
    proxy: &EventLoopProxy<UserEvent>,
) -> Result<(), AttemptEnd> {
    use overlay_sync::SyncError;

    if let Some(token) = overlay_sync::token_store::load_token() {
        match overlay_sync::client::fetch_settings(&token) {
            Ok(settings) => {
                // Nombre d'entrées loggé (pas seulement « récupéré ») — diagnostic ajouté après un
                // retour utilisateur 2026-09-02 (Suivi resté vide au tout premier lancement) : sans
                // ça, impossible de savoir si le problème vient d'une réponse déjà vide ou d'une
                // course entre son application et le premier redessin du Suivi (voir
                // `force_refresh`).
                tracing::info!(
                    "[compte] réglages récupérés depuis le jeton natif déjà connu ({} entrée(s) de suivi).",
                    settings.watchlist.len()
                );
                let _ = settings_tx.send(EngineCommand::ApplySettings(settings));
                // Jeton validé, file pas encore activée : le moment de le renouveler (voir
                // `rotate_token_if_due`) — la file reçoit celui qui vaut pour la session.
                let token = rotate_token_if_due(token);
                activate_sync_queue(&token, sync_tx);
                return Ok(());
            }
            Err(SyncError::Http {
                status: 401 | 403, ..
            }) => {
                tracing::warn!(
                    "[compte] jeton natif refusé par le serveur — session révoquée ou expirée, nouvel appairage nécessaire."
                );
                overlay_sync::token_store::clear_token();
                if !pair_if_needed {
                    return Err(AttemptEnd::Idle);
                }
            }
            Err(err) => {
                tracing::warn!(
                    "[compte] validation du jeton natif impossible ({err}) — jeton conservé, nouvelle tentative sur « Réessayer »."
                );
                return Err(AttemptEnd::Failed(auth_failure(
                    &err,
                    "GET /api/v1/settings",
                )));
            }
        }
    } else if !pair_if_needed {
        tracing::info!(
            "[compte] aucun jeton natif stocké — en attente de « Se connecter » dans la fenêtre de connexion."
        );
        return Err(AttemptEnd::Idle);
    }

    let should_cancel = || {
        matches!(
            command_rx.try_recv(),
            Ok(AuthCommand::CancelPairing | AuthCommand::Disconnect)
        )
    };
    let token = match overlay_sync::pair_and_wait(
        |handle| {
            tracing::info!("=== Connexion du compte ===");
            // **Le code d'appairage n'est PAS journalisé** (constat C6 de `docs/analyse-rgpd.md`) :
            // c'est un secret à usage unique, exploitable par quiconque lit le fichier pendant sa
            // fenêtre de validité — et le journal vit 14 jours, en clair, sur un poste partagé le
            // cas échéant. Il n'avait de toute façon pas à y être pour être utilisable : il
            // s'affiche dans la fenêtre de connexion (`AuthStatus::PairingStarted`, juste en
            // dessous), qui est le seul endroit où on le lit vraiment. Ce qui reste ici — l'URL de
            // vérification, sans paramètre — suffit au diagnostic (« le navigateur s'ouvre-t-il ?
            // sur quelle origine ? »).
            tracing::info!(
                "Confirme l'appairage sur {} — le code est affiché dans la fenêtre de connexion.",
                handle.verification_url
            );
            // Voir `AuthStatus::PairingStarted` : c'est ce qui rend le code visible dans la
            // fenêtre de connexion, pas seulement dans ces logs.
            status.store(Arc::new(AuthStatus::PairingStarted {
                pairing_code: handle.pairing_code.clone(),
                verification_url: handle.verification_url.clone(),
                expires_at: handle.expires_at,
            }));
            let _ = proxy.send_event(UserEvent::AuthStatusChanged);
        },
        should_cancel,
    ) {
        Ok(token) => token,
        Err(SyncError::PairingCancelled) => {
            tracing::info!("[compte] appairage annulé — retour à la fenêtre de connexion.");
            return Err(AttemptEnd::Idle);
        }
        Err(err) => {
            // Le détail précise la ROUTE en cause : « pair » a échoué avant même qu'un code
            // n'existe (aucun navigateur ne s'est ouvert — retour utilisateur 2026-09-01 : « il
            // devrait ouvrir le navigateur... rien ne se passe »), ou « poll » après coup.
            tracing::warn!("[compte] appairage non complété ({err}).");
            let step = if matches!(err, SyncError::PairingExpired) {
                "POST /api/v1/auth/native/poll"
            } else {
                "POST /api/v1/auth/native/pair"
            };
            return Err(AttemptEnd::Failed(auth_failure(&err, step)));
        }
    };

    if let Err(err) = overlay_sync::token_store::save_token(&token) {
        // `warn!` : un jeton non sauvegardé fait silencieusement recommencer l'appairage à chaque
        // lancement, ça DOIT être vu — jamais le jeton lui-même dans ce message (§10 du plan).
        tracing::warn!(
            "[compte] échec de sauvegarde du jeton natif ({err}) — sera redemandé au prochain lancement."
        );
    }
    match overlay_sync::client::fetch_settings(&token) {
        Ok(settings) => {
            tracing::info!(
                "[compte] connecté — réglages récupérés ({} entrée(s) de suivi).",
                settings.watchlist.len()
            );
            let _ = settings_tx.send(EngineCommand::ApplySettings(settings));
            activate_sync_queue(&token, sync_tx);
            Ok(())
        }
        Err(err) => {
            tracing::warn!("[compte] échec de récupération des réglages après appairage ({err}).");
            Err(AttemptEnd::Failed(auth_failure(
                &err,
                "GET /api/v1/settings après appairage",
            )))
        }
    }
}

/// Résout l'`uid` (`GET /api/v1/auth/me`, voir `overlay_sync::client::fetch_account_id`) et active
/// la file de synchro (L5, §7.1/§7.3 du plan) — appelé après CHAQUE connexion réussie
/// (`attempt_connect`, jeton déjà connu ou tout juste obtenu par appairage). Best-effort et jamais
/// fatal, comme le reste de cette fonction : un échec ici laisse simplement la file inactive cette
/// session (roster/watchlist restent pleinement fonctionnels, seul l'historique ne remonte pas au
/// compte) plutôt que de faire échouer toute la connexion pour un besoin annexe.
fn activate_sync_queue(token: &str, sync_tx: &mpsc::Sender<SyncCommand>) {
    match overlay_sync::client::fetch_account_id(token) {
        Ok(uid) => {
            spawn_history_import(uid.clone(), token.to_string());
            let _ = sync_tx.send(SyncCommand::Activate {
                uid,
                token: token.to_string(),
            });
        }
        Err(err) => {
            tracing::warn!(
                %err,
                "[compte] identifiant introuvable (/api/v1/auth/me) — historique non synchronisé cette session."
            );
        }
    }
}

/// Un seul import à la fois : une reconnexion pendant un import en cours n'en relance pas un second.
static HISTORY_IMPORT_RUNNING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Essais par page avant d'abandonner l'import (le suivant reprendra à la prochaine connexion).
const HISTORY_IMPORT_ATTEMPTS: u32 = 3;

/// Import de l'historique en ligne (2026-11-27, `docs/plan-historique-local.md` §8) : dès que le
/// compte est connu, tout ce que le site a reçu (`GET /api/v1/history/*`) est recopié dans la base
/// locale, **une fois par compte** — l'historique doit survivre à l'arrêt du serveur.
///
/// 1. `set_account_uid` d'abord : les lignes déjà écrites depuis le log reçoivent leur clé du site,
///    l'import les reconnaît au lieu de les doubler (`overlay_history::account`).
/// 2. Les quatre historiques, page par page ; la base n'est ouverte que le temps d'écrire une page
///    (même règle que [`spawn_history_thread`]), et le thread Historique peut écrire entre deux.
/// 3. Import complet : noté dans la base (`mark_import_done`), jamais refait pour ce compte. Un
///    échec (réseau, serveur) n'est pas noté : la prochaine connexion recommence, sans doublon.
///
/// Thread jetable, best-effort comme ses voisins : journalisé, jamais bloquant.
fn spawn_history_import(uid: String, token: String) {
    use std::sync::atomic::Ordering;
    if HISTORY_IMPORT_RUNNING.swap(true, Ordering::AcqRel) {
        return;
    }
    let spawned = thread::Builder::new()
        .name("overlay-history-import".into())
        .spawn(move || {
            run_history_import(&uid, &token);
            HISTORY_IMPORT_RUNNING.store(false, Ordering::Release);
        });
    if spawned.is_err() {
        HISTORY_IMPORT_RUNNING.store(false, Ordering::Release);
    }
}

fn run_history_import(uid: &str, token: &str) {
    use overlay_history::{HistoryStore, ImportKind};
    let path = match HistoryStore::default_path() {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!(%err, "[import] historique local indisponible");
            return;
        }
    };
    let prepared = HistoryStore::open(&path).and_then(|mut store| {
        let account = store.set_account_uid(uid)?;
        Ok((account, store.import_done(uid)?))
    });
    match prepared {
        Ok((account, true)) => {
            tracing::debug!(
                cles = account.keyed,
                fondus = account.merged,
                "[import] déjà fait pour ce compte"
            );
            return;
        }
        Ok((account, false)) => {
            tracing::info!(
                cles = account.keyed,
                fondus = account.merged,
                "[import] historique en ligne : début"
            );
        }
        Err(err) => {
            tracing::warn!(%err, "[import] base d'historique inaccessible");
            return;
        }
    }
    let started = Instant::now();
    let fetch = |kind: ImportKind, before: Option<&str>| {
        let mut attempt = 1;
        loop {
            match overlay_sync::fetch_history_page(
                token,
                kind.path(),
                overlay_history::import::PAGE_SIZE,
                before,
            ) {
                Ok(body) => return Ok(body),
                // Un refus (jeton révoqué, route absente) ne passera pas mieux au deuxième essai.
                Err(err @ overlay_sync::SyncError::Http { status, .. })
                    if status != 429 && status < 500 =>
                {
                    return Err(err)
                }
                Err(err) if attempt >= HISTORY_IMPORT_ATTEMPTS => return Err(err),
                Err(err) => {
                    tracing::debug!(%err, attempt, "[import] page en échec, nouvel essai");
                    thread::sleep(std::time::Duration::from_secs(2 * u64::from(attempt)));
                    attempt += 1;
                }
            }
        }
    };
    let result = overlay_history::import_all(
        || HistoryStore::open(&path),
        fetch,
        |progress| {
            tracing::debug!(
                type_ = progress.kind.label(),
                pages = progress.pages,
                ajoutes = progress.report.inserted,
                deja_la = progress.report.present,
                "[import] page écrite"
            );
        },
    );
    match result {
        Ok(total) => {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_millis() as i64)
                .unwrap_or(0);
            if let Err(err) =
                HistoryStore::open(&path).and_then(|mut store| store.mark_import_done(uid, now_ms))
            {
                tracing::warn!(%err, "[import] fin d'import non notée (sera refait, sans doublon)");
            }
            tracing::info!(
                ajoutes = total.inserted,
                deja_la = total.present,
                ecartes = total.skipped,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "[import] historique en ligne recopié dans la base locale"
            );
        }
        Err(err) => tracing::warn!(%err, "[import] interrompu — repris à la prochaine connexion"),
    }
}

// ── Mise à jour automatique (2026-09-15, docs/plan-mise-a-jour.md §7) ──────────────────────────

/// Ce que l'hôte demande au thread de mise à jour — voir [`spawn_update_thread`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateCommand {
    /// Lire le manifeste et rendre un verdict. `install_if_available` : télécharger et mettre en
    /// place aussitôt si une version plus récente existe (démarrage avec « Installer
    /// automatiquement » coché, ou « Réessayer » d'une mise à jour obligatoire) ; sinon se
    /// contenter de la signaler (bouton « Recherche de mise à jour »).
    Check { install_if_available: bool },
    /// Télécharger et mettre en place la version signalée disponible par la dernière
    /// vérification — bouton « Mettre à jour vers X » de la fenêtre Options. Sans verdict
    /// « disponible » en mémoire, revient à `Check { install_if_available: true }`.
    Download,
}

/// Deux vérifications manuelles ne se suivent pas à moins de trente secondes (anti-rafale sur le
/// bouton) ; une demande d'installation, elle, passe toujours.
const MANUAL_CHECK_COOLDOWN: std::time::Duration = std::time::Duration::from_secs(30);

/// Le thread de mise à jour — `std::thread` bloquant comme ses voisins, un seul modèle de
/// concurrence dans le binaire (§7.3 du plan d'architecture). Il ne fait que ce que
/// `overlay_sync::update` sait faire, dans l'ordre du §7 du plan de mise à jour, et publie chaque
/// pas dans `status` (réveil de l'hôte par `UserEvent::UpdateProgress`) :
///
/// 1. `Check` : `Checking` → `UpToDate` / `Available` / `Unavailable`. L'étape de démarrage est
///    résolue (`StartupProgress::mark_update_resolved`) sauf si l'installation suit.
/// 2. `Download` (ou `Check` avec installation) : `Downloading { received, total }` par bloc →
///    `Verifying` → `ReadyToInstall { staged }`. Pendant tout ce temps
///    `StartupProgress::set_update_blocking(true)` : l'écran de chargement reste, garde-fou
///    compris, et la fenêtre Options a rendu la main.
/// 3. C'est l'**hôte** qui installe (`App::install_update_if_ready`, à chaque tick) : le
///    remplacement de l'exe et la relance doivent précéder `event_loop.exit()`, que seul le thread
///    principal peut appeler. Le thread, lui, a fini son travail à `ReadyToInstall`.
///
/// Un échec publie `Failed { headline, detail, mandatory }` ; si la mise à jour n'était pas
/// obligatoire, le démarrage est débloqué et résolu (l'overlay repart avec sa version, on
/// réessaiera au prochain lancement) ; si elle l'était, l'écran reste sur « Mise à jour requise »
/// jusqu'à un `Check { install_if_available: true }` (« Réessayer »).
///
/// **Le dossier de l'exe est sondé avant tout téléchargement** (`check_writable_install_dir`,
/// §12 du plan) : un exe posé dans un dossier protégé donne un échec explicite tout de suite,
/// jamais un téléchargement de quinze mégaoctets suivi d'un « accès refusé ».
pub fn spawn_update_thread(
    status: Arc<ArcSwap<overlay_sync::update::UpdateStatus>>,
    startup: Arc<StartupProgress>,
    command_rx: mpsc::Receiver<UpdateCommand>,
    proxy: EventLoopProxy<UserEvent>,
) {
    use overlay_sync::update::{self, UpdateStatus, Verdict};

    thread::Builder::new()
        .name("overlay-update".into())
        .spawn(move || {
            let publish = |next: UpdateStatus| {
                status.store(Arc::new(next));
                let _ = proxy.send_event(UserEvent::UpdateProgress);
            };
            let resolve_startup = || {
                startup.mark_update_resolved();
                let _ = proxy.send_event(UserEvent::StartupProgress);
            };
            // Verdict « disponible » de la dernière vérification, gardé pour un `Download`.
            let mut available: Option<(update::Manifest, update::Asset, bool)> = None;
            let mut last_manual_check: Option<std::time::Instant> = None;

            while let Ok(command) = command_rx.recv() {
                if status.load().is_busy() {
                    continue; // une opération à la fois — commande ignorée
                }
                let (need_check, install) = match command {
                    UpdateCommand::Check {
                        install_if_available,
                    } => (true, install_if_available),
                    UpdateCommand::Download => (available.is_none(), true),
                };
                if need_check {
                    if !install {
                        if let Some(at) = last_manual_check {
                            if at.elapsed() < MANUAL_CHECK_COOLDOWN {
                                tracing::info!(
                                    "[mise à jour] vérification demandée trop tôt après la précédente — ignorée."
                                );
                                continue;
                            }
                        }
                        last_manual_check = Some(std::time::Instant::now());
                    }
                    publish(UpdateStatus::Checking);
                    tracing::info!(
                        "[mise à jour] lecture de {}",
                        update::manifest_url(update::MANIFEST_NAME)
                    );
                    match update::manifest::check(crate::build_info::VERSION) {
                        Ok(Verdict::UpToDate) => {
                            tracing::info!(
                                "[mise à jour] {} est la dernière version publiée.",
                                crate::build_info::VERSION
                            );
                            available = None;
                            publish(UpdateStatus::UpToDate {
                                checked_at: std::time::Instant::now(),
                            });
                            resolve_startup();
                            continue;
                        }
                        Ok(Verdict::Available {
                            manifest,
                            asset,
                            mandatory,
                        }) => {
                            tracing::info!(
                                "[mise à jour] version {} disponible ({} à télécharger{})",
                                manifest.version,
                                update::human_size(asset.size),
                                if mandatory { ", obligatoire" } else { "" }
                            );
                            publish(UpdateStatus::Available {
                                version: manifest.version.clone(),
                                download_size: asset.size,
                                mandatory,
                                notes_url: manifest.notes_url.clone(),
                                checked_at: std::time::Instant::now(),
                            });
                            available = Some((*manifest, asset, mandatory));
                            if !(install || mandatory) {
                                resolve_startup();
                                continue;
                            }
                        }
                        Err(err) => {
                            tracing::warn!(
                                "[mise à jour] vérification impossible : {err} — l'overlay continue avec sa version."
                            );
                            publish(UpdateStatus::Unavailable {
                                reason: err.to_string(),
                                checked_at: std::time::Instant::now(),
                            });
                            resolve_startup();
                            continue;
                        }
                    }
                }
                if let Some(entry) = available.clone() {
                    download_and_stage(&publish, &startup, entry);
                }
            }
        })
        .expect("échec de création du thread de mise à jour");
}

/// Téléchargement, vérification et mise en place d'une version — voir [`spawn_update_thread`].
fn download_and_stage(
    publish: &dyn Fn(overlay_sync::update::UpdateStatus),
    startup: &StartupProgress,
    (manifest, asset, mandatory): (
        overlay_sync::update::Manifest,
        overlay_sync::update::Asset,
        bool,
    ),
) {
    use overlay_sync::update::{self, UpdateStatus};

    let version = manifest.version.clone();
    let fail = |err: &update::UpdateError| {
        tracing::warn!("[mise à jour] échec vers {version} : {err}");
        publish(UpdateStatus::Failed {
            headline: update::headline(err).to_string(),
            detail: err.to_string(),
            mandatory,
        });
        // Non obligatoire : l'overlay démarre (ou continue) avec sa version. Obligatoire : l'écran
        // reste bloqué sur « Mise à jour requise » jusqu'à « Réessayer ».
        startup.set_update_blocking(mandatory);
        if !mandatory {
            startup.mark_update_resolved();
        }
    };

    startup.set_update_blocking(true);
    if let Err(reason) = update::apply::check_writable_install_dir() {
        fail(&update::UpdateError::Io(reason));
        return;
    }
    let dir = update::updates_dir();
    let dest = dir.join(&asset.name);
    let url = update::asset_url(&version, &asset.name);
    tracing::info!(
        "[mise à jour] téléchargement de {url} ({})",
        update::human_size(asset.size)
    );
    publish(UpdateStatus::Downloading {
        version: version.clone(),
        received: 0,
        total: asset.size,
    });
    // Un état publié par bloc de 64 Kio ferait redessiner la fenêtre à chaque bloc : au plus dix
    // publications par seconde, et toujours la dernière.
    let mut last_publish = std::time::Instant::now();
    let mut on_progress = |received: u64, total: u64| {
        if received == total || last_publish.elapsed() >= std::time::Duration::from_millis(100) {
            last_publish = std::time::Instant::now();
            publish(UpdateStatus::Downloading {
                version: version.clone(),
                received,
                total,
            });
        }
    };
    if let Err(err) =
        update::download::fetch_to_file(&url, &dest, asset.size, &asset.sha256, &mut on_progress)
    {
        fail(&err);
        return;
    }
    publish(UpdateStatus::Verifying {
        version: version.clone(),
    });
    match update::apply::stage(
        &dest,
        &version,
        asset.installed.size,
        &asset.installed.sha256,
        &dir,
    ) {
        Ok(staged) => {
            let _ = std::fs::remove_file(&dest);
            tracing::info!(
                "[mise à jour] {} vérifiée et prête : {}",
                version,
                staged.display()
            );
            publish(UpdateStatus::ReadyToInstall {
                version,
                staged,
                mandatory,
            });
        }
        Err(err) => fail(&err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Un 429 impose son délai ; sans lui, backoff ordinaire (15 s doublés, plafond 5 min).
    #[test]
    fn le_delai_du_serveur_prime_sur_le_backoff() {
        assert_eq!(
            retry_delay(Some(Duration::from_secs(42)), 1),
            Duration::from_secs(42)
        );
        assert_eq!(retry_delay(None, 1), Duration::from_secs(15));
        assert_eq!(retry_delay(None, 3), Duration::from_secs(60));
        assert_eq!(retry_delay(None, 30), Duration::from_secs(300));
    }

    /// O3 : tant que le backoff court, aucun envoi — une commande reçue entre-temps ne l'abrège
    /// pas ; une fois l'instant passé (ou sans échec), l'envoi est permis.
    #[test]
    fn le_backoff_d_historique_retient_l_envoi_jusqu_a_son_terme() {
        let now = std::time::Instant::now();
        assert_eq!(history_backoff_remaining(None, now), None);
        assert_eq!(
            history_backoff_remaining(Some(now + Duration::from_secs(30)), now),
            Some(Duration::from_secs(30))
        );
        assert_eq!(history_backoff_remaining(Some(now), now), None);
        assert_eq!(
            history_backoff_remaining(Some(now), now + Duration::from_secs(1)),
            None
        );
    }

    /// Un 401 prévient le thread Auth UNE fois, et coupe l'envoi : le compte est oublié jusqu'au
    /// prochain `Activate`.
    #[test]
    fn un_401_previent_le_thread_auth_une_seule_fois() {
        let (tx, rx) = mpsc::channel();
        let mut account = Some(("uid".to_string(), "jeton".to_string()));
        session_refused(&mut account, &tx, "HTTP 401");
        session_refused(&mut account, &tx, "HTTP 401");
        assert!(account.is_none());
        assert_eq!(rx.try_recv(), Ok(AuthCommand::SessionExpired));
        assert!(rx.try_recv().is_err(), "pas de second signal");
    }
}
