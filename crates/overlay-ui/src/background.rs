//! **Threads de fond partagés par les deux binaires** (2026-09-14) : compte (`spawn_auth_thread`),
//! file de synchro (`spawn_sync_thread`), catalogue (`spawn_catalog_thread`) et référentiel de
//! donjons (`spawn_dungeon_thread`). Ils vivaient dans `main.rs` (Windows) depuis les lots L3-L5 ;
//! rien n'y dépend de l'OS — `std::thread`, `mpsc`, `ArcSwap`, `overlay_sync` et un
//! `EventLoopProxy` — et le binaire Linux (`bin/overlay-ui-x11.rs`) en a besoin depuis que la
//! fenêtre de connexion lui est portée (§9.1 undecies du plan) : sortis ici tels quels, doc
//! comprise, plutôt que dupliqués.
//!
//! Chacun signale au [`crate::startup::StartupProgress`] partagé la fin de son chargement initial,
//! ce qui fait tomber l'écran de chargement de la fenêtre de connexion (voir `panels::login`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use arc_swap::ArcSwap;
use overlay_engine::{CatalogIndex, DungeonIndex, WatchlistEntry};
use winit::event_loop::EventLoopProxy;

use crate::engine_thread::{EngineCommand, SyncCommand};
use crate::render_content::{AuthCommand, AuthFailure, AuthStatus, UserEvent};
use crate::startup::StartupProgress;

/// Thread Auth (lot L4, §7.2 du plan) : résout l'accès au compte sur un thread dédié — jamais le
/// thread Engine ni le main thread. Jamais fatal : sans jeton stocké, l'overlay affiche sa fenêtre
/// de connexion et attend (plus de mode invité depuis le 2026-09-14, voir `spawn_auth_thread`).
///
/// Thread Catalogue (lot L3, §7.4 du plan — réduit pour l'instant à la résolution d'icônes, voir
/// `overlay_engine::catalog`) : résout les icônes réelles d'objets/monstres affichées par le
/// panneau Suivi (retour utilisateur 2026-09-02, capture d'écran à l'appui : icône générique
/// partout, tuiles impossibles à distinguer). Offline-first (mêmes principes que
/// `CatalogService.initialize()` côté web) : le cache disque
/// (`overlay_sync::catalog_cache`) est chargé et publié IMMÉDIATEMENT s'il existe, sans attendre
/// le réseau — le rafraîchissement qui suit ne republie que si `GET /api/v1/catalog/version`
/// (`indexHash`) a changé depuis le cache, jamais pour rien. Le MÊME `Arc<ArcSwap<CatalogIndex>>`
/// est aussi relayé au thread Engine (voir `spawn_engine_thread`, `Engine::set_catalog`) : sert
/// cette fois de garde-fou `hostIsKnownMonsterName` (`quickjs_engine.rs`) contre un vrai monstre
/// qui se révèle (mimique, brèche) confondu à tort avec une invocation.
///
/// Tout premier lancement SANS cache disque ET SANS réseau : repli sur le catalogue embarqué
/// (`overlay_sync::catalog_cache::embedded_fallback`, §7.4 du plan) — l'overlay reste utilisable
/// plutôt que de rester sur un `CatalogIndex::default()` vide. `catalog_stale` (lu par `render`,
/// icône « 📦⚠ » de la zone Combat) est mis à `true` dans ce seul cas — jamais réinitialisé à
/// `false` explicitement ailleurs dans ce thread : sa valeur initiale (posée par `main`) est déjà
/// `false`, et les autres branches de cette fonction ne s'exécutent qu'une fois par lancement, donc
/// aucune ne peut suivre une mise à `true` pour la corriger a posteriori.
pub fn spawn_catalog_thread(
    catalog: Arc<ArcSwap<CatalogIndex>>,
    catalog_stale: Arc<AtomicBool>,
    startup: Arc<StartupProgress>,
    proxy: EventLoopProxy<UserEvent>,
) {
    thread::Builder::new()
        .name("overlay-catalog".into())
        .spawn(move || {
            load_catalog(&catalog, &catalog_stale, &proxy);
            // Quel que soit le chemin pris ci-dessus (cache, réseau, repli, échec) : le
            // catalogue a fini de se charger, l'écran de chargement peut le décompter.
            startup.mark_catalog();
            let _ = proxy.send_event(UserEvent::StartupProgress);
        })
        .expect("échec de création du thread Catalogue");
}

fn load_catalog(
    catalog: &Arc<ArcSwap<CatalogIndex>>,
    catalog_stale: &Arc<AtomicBool>,
    proxy: &EventLoopProxy<UserEvent>,
) {
    let mut cached_hash = None;
    if let Some((hash, index)) = overlay_sync::catalog_cache::load() {
        catalog.store(Arc::new(CatalogIndex::from_compact_json(&index)));
        let _ = proxy.send_event(UserEvent::NewSnapshot);
        cached_hash = Some(hash);
    }

    let latest_hash = match overlay_sync::client::fetch_catalog_version() {
        Ok(hash) => hash,
        Err(err) => {
            // Repli hors-ligne EMBARQUÉ (§7.4 du plan, `catalog_cache::embedded_fallback`)
            // — uniquement si `cached_hash` est vide : un cache disque déjà chargé
            // ci-dessus reste toujours préférable (référentiel plus récent que le
            // placeholder embarqué), le réseau injoignable n'y change rien.
            if cached_hash.is_none() {
                tracing::warn!(
                    %err,
                    "catalogue injoignable ET aucun cache local — repli sur le catalogue embarqué (daté)"
                );
                catalog.store(Arc::new(CatalogIndex::from_compact_json(
                    &overlay_sync::catalog_cache::embedded_fallback(),
                )));
                catalog_stale.store(true, Ordering::Relaxed);
                let _ = proxy.send_event(UserEvent::NewSnapshot);
            } else {
                tracing::warn!(%err, "version du catalogue injoignable, repli sur le cache local");
            }
            return;
        }
    };
    if cached_hash.as_deref() == Some(latest_hash.as_str()) {
        tracing::info!("catalogue déjà à jour (cache local)");
        return;
    }
    match overlay_sync::client::fetch_catalog_index() {
        Ok(index) => {
            catalog.store(Arc::new(CatalogIndex::from_compact_json(&index)));
            let _ = proxy.send_event(UserEvent::NewSnapshot);
            if let Err(err) = overlay_sync::catalog_cache::save(&latest_hash, &index) {
                tracing::warn!(
                    %err,
                    "échec de mise en cache du catalogue (retéléchargé au prochain lancement)"
                );
            }
        }
        Err(err) => tracing::warn!(
            %err,
            "téléchargement du catalogue impossible, repli sur le cache local"
        ),
    }
}

/// Thread Donjons (L5, §7.1 du plan — assignation `dungeonId`/`dungeonRunKey`) : miroir simplifié
/// de `spawn_catalog_thread` ci-dessus — `GET /api/v1/dungeons` n'expose pas d'endpoint `/version`
/// séparé (voir `reference_data_cache.rs`), donc pas de comparaison de hash possible : le cache
/// disque est chargé immédiatement (l'Engine reste utilisable pour la synchro dès le démarrage même
/// hors ligne), puis la version réseau REMPLACE inconditionnellement l'index en mémoire dès qu'elle
/// arrive, sans jamais bloquer ce thread ni les autres. Volume négligeable (~150 lignes, voir la
/// doc de tête de `dungeon.rs`) : pas de repli embarqué comme pour le catalogue (~1,8 Mo) — un
/// référentiel de donjons manquant laisse simplement `dungeonId` à `None`, jamais un blocage.
pub fn spawn_dungeon_thread(
    dungeons: Arc<ArcSwap<DungeonIndex>>,
    startup: Arc<StartupProgress>,
    proxy: EventLoopProxy<UserEvent>,
) {
    thread::Builder::new()
        .name("overlay-dungeons".into())
        .spawn(move || {
            load_dungeons(&dungeons, &proxy);
            startup.mark_dungeons();
            let _ = proxy.send_event(UserEvent::StartupProgress);
        })
        .expect("échec de création du thread Donjons");
}

fn load_dungeons(dungeons: &Arc<ArcSwap<DungeonIndex>>, proxy: &EventLoopProxy<UserEvent>) {
    if let Some(rows) = overlay_sync::reference_data_cache::load(
        overlay_sync::reference_data_cache::ReferenceData::Dungeons,
    ) {
        dungeons.store(Arc::new(DungeonIndex::from_json(&rows)));
        let _ = proxy.send_event(UserEvent::NewSnapshot);
    }
    match overlay_sync::client::fetch_dungeons() {
        Ok(rows) => {
            dungeons.store(Arc::new(DungeonIndex::from_json(&rows)));
            let _ = proxy.send_event(UserEvent::NewSnapshot);
            if let Err(err) = overlay_sync::reference_data_cache::save(
                overlay_sync::reference_data_cache::ReferenceData::Dungeons,
                &rows,
            ) {
                tracing::warn!(
                    %err,
                    "échec de mise en cache du référentiel de donjons (retéléchargé au prochain lancement)"
                );
            }
        }
        Err(err) => tracing::warn!(
            %err,
            "téléchargement du référentiel de donjons impossible, repli sur le cache local (dungeonId non résolu si aucun cache)"
        ),
    }
}

/// Thread Sync (lot L5, §7.3 du plan) : possède la file SQLite persistante (`overlay_sync::
/// SyncQueue`) — écriture (`enqueue`) ET envoi réseau (`flush_once`) vivent entièrement ici, jamais
/// sur le thread Engine (voir `spawn_engine_thread`, qui ne fait que relayer des `SyncEvent` déjà
/// sérialisés sans jamais attendre dessus) ni sur le main thread. Best-effort à l'ouverture de la
/// file elle-même (dossier de données/SQLite indisponible) : le thread se termine simplement,
/// l'overlay continue sans synchro plutôt que de planter — même philosophie que
/// `spawn_catalog_thread`/`spawn_auth_thread`.
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
/// **Compteurs de Suivi (`SyncCommand::SyncWatchlist`, §14 point 3 du plan, chantier fermé le
/// 2026-09-07)** — même thread, mécanisme VOLONTAIREMENT séparé de la file `SyncQueue` ci-dessus :
/// pas de file SQLite ni de `client_key` idempotent (la valeur ENTIÈRE remplace la clé côté
/// serveur, voir `functions/api/v1/settings.ts::onRequestPatch`, « dernier écrivain gagne »), donc
/// rien à dédupliquer — seul le DERNIER instantané reçu compte, porté en mémoire
/// (`pending_watchlist`) plutôt qu'en base. Débounce de `WATCHLIST_DEBOUNCE` avant le premier
/// essai (miroir de `WRITE_DEBOUNCE_MS` côté web, `RemoteUserDataRepository`) : un compteur qui
/// s'incrémente à chaque kill d'un combat ne doit pas déclencher une requête par kill. Un nouvel
/// instantané reçu PENDANT l'attente (débounce ou backoff) remplace le précédent et relance un
/// débounce complet — rien n'est perdu (`WatchlistState` garde de toute façon le fichier local
/// comme vérité, voir sa doc), seul le nombre de requêtes est réduit.
pub fn spawn_sync_thread(command_rx: mpsc::Receiver<SyncCommand>) {
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
            // Dernier instantané de Suivi reçu et pas encore répliqué avec succès (voir la doc de
            // `spawn_sync_thread` ci-dessus) — `None` tant qu'aucun `SyncCommand::SyncWatchlist`
            // n'est arrivé, ou après un envoi réussi.
            let mut pending_watchlist: Option<Vec<WatchlistEntry>> = None;
            // Instant à partir duquel retenter l'envoi watchlist (fin du débounce, ou du backoff
            // après un échec) — distinct du calcul `wait` de l'historique ci-dessous : les deux
            // mécanismes n'ont ni la même cause de délai ni le même état.
            let mut watchlist_ready_at: Option<std::time::Instant> = None;
            let mut watchlist_consecutive_failures: u32 = 0;
            // Pas de compte connu au démarrage : n'attend qu'une commande, ne sonde jamais pour
            // rien (même philosophie que `settings_rx.try_recv()` côté thread Engine).
            let mut wait = std::time::Duration::from_secs(3600);
            loop {
                match command_rx.recv_timeout(wait) {
                    Ok(SyncCommand::Activate { uid, token }) => {
                        tracing::info!("file de synchro activée (compte connecté, lot L5)");
                        account = Some((uid, token));
                    }
                    Ok(SyncCommand::Deactivate) => {
                        tracing::info!(
                            "file de synchro désactivée (compte déconnecté) — contenu déjà en file conservé sur disque"
                        );
                        account = None;
                    }
                    Ok(SyncCommand::Enqueue(events)) => {
                        for event in &events {
                            if let Err(err) = queue.enqueue(event) {
                                tracing::warn!(
                                    %err,
                                    kind = event.kind.as_str(),
                                    "échec d'enfilage d'un événement d'historique (SQLite)"
                                );
                            }
                        }
                    }
                    Ok(SyncCommand::SyncWatchlist(entries)) => {
                        pending_watchlist = Some(entries);
                        watchlist_consecutive_failures = 0;
                        watchlist_ready_at = Some(std::time::Instant::now() + WATCHLIST_DEBOUNCE);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {} // réessai programmé (backoff) — retombe sur le flush ci-dessous
                    Err(mpsc::RecvTimeoutError::Disconnected) => break, // App fermée
                }

                let history_wait = match &account {
                    None => std::time::Duration::from_secs(3600),
                    Some((uid, token)) => {
                        match queue.flush_once(uid, |path, body| {
                            overlay_sync::post_json_authenticated(token, path, body)
                        }) {
                            Ok(overlay_sync::FlushOutcome::Idle | overlay_sync::FlushOutcome::Synced) => {
                                std::time::Duration::from_secs(3600)
                            }
                            // **Correctif du 2026-09-03** : cet échec n'était auparavant tracé
                            // nulle part — un blocage persistant (401/429/réseau/5xx) restait
                            // invisible, backoff après backoff, jusqu'à 5 min entre essais.
                            Ok(overlay_sync::FlushOutcome::Retry(reason)) => {
                                tracing::warn!(
                                    reason = %reason,
                                    consecutive_failures = queue.consecutive_failures(),
                                    "échec d'envoi de l'historique — nouvel essai après un backoff"
                                );
                                backoff_delay(queue.consecutive_failures())
                            }
                            Err(err) => {
                                tracing::warn!(%err, "erreur de file de synchro (SQLite)");
                                std::time::Duration::from_secs(60)
                            }
                        }
                    }
                };

                let watchlist_wait = flush_watchlist_once(
                    &account,
                    &mut pending_watchlist,
                    &mut watchlist_ready_at,
                    &mut watchlist_consecutive_failures,
                );

                wait = history_wait.min(watchlist_wait);
            }
        })
        .expect("échec de création du thread Sync");
}

/// Débounce avant le premier essai d'un envoi watchlist — miroir de `WRITE_DEBOUNCE_MS`
/// (`remote-user-data.repository.ts`) côté web.
const WATCHLIST_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(1_500);

/// Tente, si le moment est venu, de répliquer le dernier instantané de Suivi reçu — voir la doc de
/// `spawn_sync_thread` pour le mécanisme complet (débounce + backoff, sans file persistante).
/// Renvoie le délai avant le PROCHAIN passage utile pour CE mécanisme (à combiner par l'appelant
/// avec celui de l'historique, voir `wait`) : 1 h tant que rien n'est en attente ou qu'aucun
/// compte n'est connu (réveillé immédiatement par la prochaine commande), le temps restant avant
/// `ready_at` pendant un débounce/backoff en cours, ou le nouveau backoff après un échec.
fn flush_watchlist_once(
    account: &Option<(String, String)>,
    pending: &mut Option<Vec<WatchlistEntry>>,
    ready_at: &mut Option<std::time::Instant>,
    consecutive_failures: &mut u32,
) -> std::time::Duration {
    let Some(entries) = pending.as_ref() else {
        return std::time::Duration::from_secs(3600);
    };
    let Some((_, token)) = account else {
        return std::time::Duration::from_secs(3600); // pas de compte : rien à tenter avant `Activate`
    };
    let now = std::time::Instant::now();
    if let Some(at) = ready_at {
        if now < *at {
            return *at - now;
        }
    }

    match overlay_sync::patch_watchlist(token, entries) {
        Ok(_) => {
            *pending = None;
            *ready_at = None;
            *consecutive_failures = 0;
            std::time::Duration::from_secs(3600)
        }
        Err(err) => {
            *consecutive_failures += 1;
            tracing::warn!(
                %err,
                consecutive_failures = *consecutive_failures,
                "échec de synchro des compteurs de Suivi — nouvel essai après un backoff"
            );
            let delay = backoff_delay(*consecutive_failures);
            *ready_at = Some(now + delay);
            delay
        }
    }
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
                            overlay_sync::token_store::clear_token();
                            let _ = settings_tx.send(EngineCommand::Disconnect);
                            let _ = sync_tx.send(SyncCommand::Deactivate);
                            connected = false;
                            status.store(Arc::new(AuthStatus::Disconnected { failure: None }));
                            let _ = proxy.send_event(UserEvent::AuthStatusChanged);
                            tracing::info!(
                                "[compte] déconnecté — jeton effacé, retour à la fenêtre de connexion."
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
            tracing::info!(
                "Ouvre {} et confirme le code : {}",
                handle.verification_url,
                handle.pairing_code
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
