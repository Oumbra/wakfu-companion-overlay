//! Thread Engine (§3 du plan) — voir la doc de `lib.rs` pour pourquoi ce module est partagé entre
//! les deux binaires (`main.rs` Windows, `bin/wakfu-companion-overlay-x11.rs` Linux) : rien ici ne
//! dépend de l'OS, seuls les threads compte lié (Auth/Sync/Catalogue, lot L4-L5) restent
//! Windows-only pour l'instant — le binaire Linux tourne pour l'instant uniquement en mode invité
//! (`catalog`/ `dungeons` vides, `sync_tx` vers un canal dont rien ne lit jamais le receveur).
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
    CatalogIndex, DungeonIndex, Engine, SessionSnapshot, WatchlistAlertReason, WatchlistEntry,
    WatchlistKind,
};
use overlay_sync::AccountSettings;
use serde_json::Value;
use winit::event_loop::EventLoopProxy;

use crate::alert_sound;
use crate::panels;
use crate::panels::chat_tab::ChatToastSettings;
use crate::panels::feature_switch::FeatureToggles;
use crate::panels::notifications::{AlertMutes, ToastClose};
use crate::panels::suivi_tab::CountdownToastSettings;
use crate::panels::watchlist::{WatchlistToast, WatchlistToastReason};
use crate::render_content::UserEvent;

/// Message transmis au thread Engine sur le même canal que les réglages de compte récupérés
/// (`AccountSettings`) — `Disconnect` (déconnexion volontaire) n'est PAS juste une absence de
/// réglages : il doit activement effacer le roster/suivi déjà appliqués (repli `breed`, Suivi
/// vidé), ce qu'un simple silence sur le canal ne ferait jamais.
// `AccountSettings` pèse ~280 octets là où la deuxième variante en fait 32 : clippy propose de le
// boxer. Ce canal porte des GESTES D'UTILISATEUR — une connexion, un réglage changé dans la modale
// — quelques messages par session, jamais une boucle chaude. Un `Box` y échangerait 250 octets sur
// une poignée d'envois contre une allocation et une indirection à chaque lecture, et surtout contre
// la lisibilité de l'appelant. L'écart est assumé, pas ignoré.
#[allow(clippy::large_enum_variant)]
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
    /// Profil d'alerte validé depuis l'onglet « Alertes » de la fenêtre Options (2026-09-12) —
    /// liste d'objets à son activé ET réglages du toast.
    ///
    /// **Distinct d'`ApplySettings`**, qui porte tout ce qui descend du compte : celui-ci est ce
    /// que l'utilisateur vient de régler LUI-MÊME, appliqué tout de suite pour que la prochaine
    /// alerte obéisse sans attendre un aller-retour réseau. L'écriture au compte, elle, part en
    /// parallèle côté hôte.
    SetAlertProfile(overlay_engine::AlertProfile),
    /// Définitions suivies validées depuis l'onglet « Suivi » de la fenêtre Options (2026-09-13) —
    /// nom, genre, mode et cible de chaque entrée.
    ///
    /// **Les compteurs n'y sont pas**, et c'est délibéré : le brouillon a été pris à l'ouverture de
    /// la fenêtre, un objet ramassé depuis y serait resté à sa valeur d'alors. Le moteur garde donc
    /// les siens — voir `WatchlistState::apply_definitions`. Comme pour `SetAlertProfile`, la
    /// réplication au compte part en parallèle côté hôte, par le même chemin que les compteurs
    /// (`SyncCommand::SyncWatchlist`).
    SetWatchlistDefinitions {
        definitions: Vec<WatchlistEntry>,
        /// Les entrées que l'édition a RETIRÉES — vide pour un geste qui ne peut rien recréer
        /// (retrait groupé ou déplacement depuis le bandeau : la liste part telle qu'elle doit
        /// être, dans la foulée du geste).
        ///
        /// Une entrée qui y figure ET revient dans `definitions` a été supprimée puis recréée
        /// pendant la même édition : son compteur ne la suit pas (voir
        /// `WatchlistState::apply_definitions`).
        retirees: Vec<WatchlistEntry>,
    },
    /// Recherches de chat validées depuis l'onglet « Chat » (2026-09-13) — même principe que
    /// `SetAlertProfile` : appliquées tout de suite, l'écriture au compte (`chatFilters`) part en
    /// parallèle côté hôte.
    SetChatFilters(Vec<overlay_engine::ChatFilter>),
    /// Réglages de la carte d'alerte de chat (durée, fermeture manuelle) — locaux à la machine
    /// (voir `config::OverlayConfig`), envoyés au démarrage puis à chaque validation de l'onglet.
    SetChatToast(ChatToastSettings),
    /// **Le roster validé depuis l'onglet « Personnages »** (2026-09-16) — même principe que
    /// `SetChatFilters` : appliqué tout de suite, l'écriture au compte (clé `roster`) part en
    /// parallèle côté hôte.
    ///
    /// Sans cette commande, un personnage déclaré ne serait reconnu qu'au prochain `ApplySettings`
    /// — c'est-à-dire, en pratique, à la prochaine ouverture de la fenêtre Options : le combat en
    /// cours continuerait de le classer sur son seul `breed`.
    SetRoster(overlay_engine::Roster),
    /// Réglages de la carte de **décompte arrivé à zéro** (durée, fermeture manuelle) — locaux à
    /// la machine comme [`Self::SetChatToast`], envoyés au démarrage puis à chaque validation de
    /// la fenêtre Options (section « Suivi » de l'onglet « Paramètres », 2026-09-16).
    ///
    /// Avant cette commande, la carte du décompte prenait la durée du **profil d'alertes de
    /// ramassage** descendu du compte : les deux alertes partageaient un seul réglage.
    SetCountdownToast(CountdownToastSettings),
    /// **Les trois interrupteurs de fonctionnalité** — cases « Activer le suivi » / « Activer les
    /// alertes » / « Activer la recherche » (`panels::feature_switch`, 2026-09-15). Locaux à la
    /// machine comme `SetChatToast`, envoyés au démarrage puis à chaque validation de la fenêtre
    /// Options.
    ///
    /// **Ce qu'ils coupent ici, et ce qu'ils ne coupent pas** : ce thread cesse de jouer le son et
    /// d'afficher la carte des alertes concernées. Il continue en revanche à les DRAINER du moteur
    /// (voir la boucle d'ingestion) et le moteur continue à compter, à suivre et à synchroniser —
    /// une fonctionnalité coupée met son bruit en sourdine, elle ne fait perdre ni un ramassage ni
    /// un compteur. Recocher la case retrouve donc tout en l'état, sans rattrapage ni relecture du
    /// log ; en échange, les alertes survenues pendant la coupure sont perdues, ce qui est
    /// exactement ce qu'on demande en coupant.
    SetFeatures(FeatureToggles),
    /// **Les deux sourdines** — cases « Couper le son des notifications » des onglets « Suivi » et
    /// « Chat » (`panels::notifications`, 2026-09-15). Locales à la machine comme `SetFeatures`,
    /// envoyées au démarrage puis à chaque validation de la fenêtre Options.
    ///
    /// **Ce qu'elles coupent, et ce qu'elles ne coupent pas** : ce thread cesse de JOUER LE SON de
    /// l'alerte concernée, et continue d'afficher sa carte par-dessus le jeu. C'est là toute la
    /// différence avec [`Self::SetFeatures`], qui coupe les deux canaux à la fois.
    SetAlertMutes(AlertMutes),
}

/// L'instant où une carte réglée LOCALEMENT doit disparaître — le pendant de [`toast_deadline`]
/// pour les réglages qui ne descendent pas du compte : la carte de chat
/// (`panels::chat_tab::ChatToastSettings`) et celle du décompte à zéro
/// (`panels::suivi_tab::CountdownToastSettings`).
///
/// Générique sur [`ToastClose`], le trait que la fenêtre Options utilise déjà pour peindre les
/// trois lignes « Fermeture automatique » : les deux types bornent leur durée eux-mêmes, il n'y a
/// rien à revalider ici.
fn local_toast_deadline<T: ToastClose>(
    created_at: std::time::Instant,
    settings: &T,
) -> Option<std::time::Instant> {
    if settings.manual_close() {
        return None;
    }
    Some(created_at + std::time::Duration::from_secs_f32(settings.duration_seconds()))
}

/// L'instant où un toast doit disparaître, d'après le profil d'alerte — `None` quand le compte a
/// demandé une fermeture manuelle.
///
/// La durée est déjà bornée par `AlertProfile` (0,5 à 30 s) : rien à revalider ici.
fn toast_deadline(
    created_at: std::time::Instant,
    profile: &overlay_engine::AlertProfile,
) -> Option<std::time::Instant> {
    if profile.manual_close {
        return None;
    }
    Some(created_at + std::time::Duration::from_secs_f32(profile.duration_seconds))
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
/// Le profil d'alerte du compte et l'objet `profile` BRUT dont il est tiré, partagés entre le
/// thread Engine (qui les publie) et l'hôte (qui ouvre la fenêtre Options).
///
/// `None` tant qu'aucun compte n'a répondu, et de nouveau `None` après une déconnexion. Le JSON
/// brut voyage avec le profil parce qu'il faut repartir de LUI pour réécrire la clé `profile` sans
/// effacer le pseudo et l'avatar — voir `overlay_engine::AlertProfile::patch_value`.
pub type SharedAlertProfile = Arc<ArcSwap<Option<(overlay_engine::AlertProfile, Option<Value>)>>>;

/// Les recherches de chat du compte, partagées de la même façon que [`SharedAlertProfile`] :
/// `None` tant qu'aucun compte n'a répondu (et après une déconnexion), `Some` sinon — même vide.
/// L'hôte les lit à l'ouverture de la fenêtre Options, pour en faire le brouillon de l'onglet
/// « Chat ».
pub type SharedChatFilters = Arc<ArcSwap<Option<Vec<overlay_engine::ChatFilter>>>>;
/// Le roster du compte, publié par le thread Engine comme les filtres de chat — la surveillance
/// de tour (`turn_watch`) y lit les personnages du compte de chaque fenêtre (titulaire + héros).
/// `None` sans compte lié.
pub type SharedRoster = Arc<ArcSwap<Option<overlay_engine::RosterIndex>>>;
/// **Le même roster, sous sa forme éditable** — ce que l'onglet « Personnages » prend en brouillon
/// à l'ouverture de la fenêtre Options (2026-09-16). Publié par le même `ApplySettings` que
/// [`SharedRoster`], donc jamais désynchronisé de lui ; distinct parce que l'index de lecture jette
/// l'identité des comptes, que l'écriture ne peut pas se permettre de perdre (voir
/// `overlay_engine::roster`, doc de module).
pub type SharedRosterDraft = Arc<ArcSwap<Option<overlay_engine::Roster>>>;

pub struct EngineHandles {
    pub snapshot: Arc<ArcSwap<SessionSnapshot>>,
    pub watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
    pub watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
    pub catalog: Arc<ArcSwap<CatalogIndex>>,
    /// Référentiel des donjons (L5, §7.1 du plan) — voir le thread Catalogue. Contrairement au
    /// catalogue, aucun panneau ne le consomme directement : seul l'Engine s'en sert, pour la
    /// synchro serveur.
    pub dungeons: Arc<ArcSwap<DungeonIndex>>,
    /// Publié par ce thread, qui est celui qui reçoit `ApplySettings` — et republié à chaque
    /// `Disconnect`. L'hôte le lit à l'ouverture de la fenêtre Options, pour en faire le brouillon
    /// de l'onglet « Alertes ».
    pub alert_profile: SharedAlertProfile,
    /// Publié par ce thread comme `alert_profile`, pour l'onglet « Chat ».
    pub chat_filters: SharedChatFilters,
    /// Publié par ce thread comme `alert_profile`, pour la surveillance de tour.
    pub roster: SharedRoster,
    /// Publié par ce thread comme `roster`, pour l'onglet « Personnages ».
    pub roster_draft: SharedRosterDraft,
    /// Avancement du démarrage (voir `crate::startup`) : ce thread y marque la fin du rattrapage
    /// initial de `wakfu.log` — au premier silence du watcher (200 ms sans lot), ou au premier lot
    /// qui n'est plus étiqueté `is_initial_load`. Le watcher pousse les lots du rattrapage d'un
    /// trait (voir `overlay_ingest::watcher::run`), un silence en marque donc la fin — et un
    /// fichier vide ou absent ne bloque rien.
    pub startup: Arc<crate::startup::StartupProgress>,
}

pub fn spawn_engine_thread(
    log_path: PathBuf,
    handles: EngineHandles,
    proxy: EventLoopProxy<UserEvent>,
    settings_rx: mpsc::Receiver<EngineCommand>,
    sync_tx: mpsc::Sender<SyncCommand>,
) {
    let EngineHandles {
        startup,
        snapshot,
        watchlist,
        watchlist_toast,
        alert_profile: alert_profile_out,
        catalog,
        dungeons,
        chat_filters: chat_filters_out,
        roster: roster_out,
        roster_draft: roster_draft_out,
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
            // Voir `EngineHandles::startup` — posé une seule fois, jamais remis à `false` (un
            // changement de fichier en cours de session n'est pas un démarrage).
            let mut log_replayed = false;
            // Les réglages du toast (durée, fermeture manuelle) — le repli du profil par défaut
            // tant qu'aucun compte n'a répondu. C'est ici qu'ils vivent parce que c'est ici que
            // `hide_at` se calcule, au moment où l'alerte naît.
            let mut alert_profile = overlay_engine::AlertProfile::default();
            // Même rôle pour la carte de chat — envoyés par l'hôte dès le démarrage
            // (`SetChatToast`, depuis la config locale).
            let mut chat_toast = ChatToastSettings::default();
            // Même rôle pour la carte du décompte à zéro — jusqu'au 2026-09-16, elle prenait la
            // durée d'`alert_profile` ci-dessus, c'est-à-dire celle des alertes de ramassage.
            let mut countdown_toast = CountdownToastSettings::default();
            // Les trois interrupteurs — tout actif tant que l'hôte n'a rien dit (voir
            // `FeatureToggles::default`), comme pour une installation neuve.
            let mut features = FeatureToggles::default();
            // Les deux sourdines — aucune tant que l'hôte n'a rien dit (`AlertMutes::default`),
            // c'est-à-dire tout sonne, comme pour une installation neuve.
            let mut mutes = AlertMutes::default();
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
                            roster_out.store(Arc::new(Some(settings.roster.clone())));
                            roster_draft_out.store(Arc::new(Some(settings.roster_draft)));
                            engine.set_roster(Some(settings.roster));
                            engine.set_watchlist_entries(settings.watchlist);
                            alert_profile = settings.alerts;
                            engine.set_sound_items(alert_profile.sound_items.clone());
                            alert_profile_out.store(Arc::new(Some((
                                alert_profile.clone(),
                                settings.profile_raw,
                            ))));
                            engine.set_chat_filters(settings.chat_filters.clone());
                            chat_filters_out.store(Arc::new(Some(settings.chat_filters)));
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
                            roster_out.store(Arc::new(None));
                            roster_draft_out.store(Arc::new(None));
                            engine.set_roster(None);
                            engine.set_watchlist_entries(Vec::new());
                            engine.set_sound_items(Vec::new());
                            alert_profile = overlay_engine::AlertProfile::default();
                            // Plus de compte : l'onglet « Alertes » doit repasser à « aucun compte
                            // lié », pas garder la liste du compte qu'on vient de quitter.
                            alert_profile_out.store(Arc::new(None));
                            engine.set_chat_filters(Vec::new());
                            chat_filters_out.store(Arc::new(None));
                        }
                        EngineCommand::SetAlertProfile(profile) => {
                            tracing::info!(
                                sound_item_count = profile.sound_items.len(),
                                duration_seconds = profile.duration_seconds,
                                manual_close = profile.manual_close,
                                "[options] profil d'alerte appliqué depuis la fenêtre Options"
                            );
                            engine.set_sound_items(profile.sound_items.clone());
                            // Le brouillon validé devient la référence : rouvrir la fenêtre doit
                            // repartir de ce qu'on vient de régler, pas de ce que le compte
                            // renvoyait avant. Le JSON brut du compte est conservé tel quel — la
                            // prochaine réécriture doit toujours repartir de LUI.
                            let raw = alert_profile_out
                                .load()
                                .as_ref()
                                .as_ref()
                                .and_then(|(_, raw)| raw.clone());
                            alert_profile_out.store(Arc::new(Some((profile.clone(), raw))));
                            alert_profile = profile;
                        }
                        EngineCommand::SetChatFilters(filters) => {
                            tracing::info!(
                                filter_count = filters.len(),
                                "[options] recherches de chat appliquées depuis la fenêtre Options"
                            );
                            engine.set_chat_filters(filters.clone());
                            chat_filters_out.store(Arc::new(Some(filters)));
                        }
                        EngineCommand::SetRoster(roster) => {
                            let account_count = roster.accounts.len();
                            let character_count: usize =
                                roster.accounts.iter().map(|a| a.characters.len()).sum();
                            tracing::info!(
                                account_count,
                                character_count,
                                "[options] roster appliqué depuis la fenêtre Options"
                            );
                            // L'index de lecture se REFAIT depuis le roster édité : les deux
                            // publications viennent de la même valeur, elles ne peuvent pas
                            // diverger (voir `SharedRosterDraft`).
                            let index = overlay_engine::RosterIndex::from_settings_json(
                                &serde_json::json!({ "roster": roster.patch_value() }),
                            );
                            roster_out.store(Arc::new(Some(index.clone())));
                            engine.set_roster(Some(index));
                            roster_draft_out.store(Arc::new(Some(roster)));
                        }
                        EngineCommand::SetFeatures(toggles) => {
                            tracing::info!(
                                suivi = toggles.suivi,
                                alertes = toggles.alerts,
                                recherche = toggles.chat,
                                "[options] fonctionnalités actives"
                            );
                            features = toggles;
                        }
                        EngineCommand::SetAlertMutes(coupures) => {
                            tracing::info!(
                                suivi = coupures.suivi,
                                chat = coupures.chat,
                                "[options] sons d'alerte coupés"
                            );
                            mutes = coupures;
                        }
                        EngineCommand::SetChatToast(settings) => {
                            tracing::info!(
                                duration_seconds = settings.duration_seconds,
                                manual_close = settings.manual_close,
                                "[options] réglages de la carte de chat appliqués"
                            );
                            chat_toast = settings;
                        }
                        EngineCommand::SetCountdownToast(settings) => {
                            tracing::info!(
                                duration_seconds = settings.duration_seconds,
                                manual_close = settings.manual_close,
                                "[options] réglages de la carte de décompte appliqués"
                            );
                            countdown_toast = settings;
                        }
                        EngineCommand::SetWatchlistDefinitions {
                            definitions,
                            retirees,
                        } => {
                            tracing::info!(
                                entry_count = definitions.len(),
                                removed_count = retirees.len(),
                                "[options] liste de suivi appliquée depuis la fenêtre Options"
                            );
                            // Les compteurs vivants sont gardés par le moteur, jamais repris du
                            // brouillon — sauf ceux des entrées retirées pendant l'édition, voir
                            // la doc de la commande.
                            engine.set_watchlist_definitions(definitions, &retirees);
                        }
                        EngineCommand::ChangeLogPath(new_path) => {
                            tracing::info!(
                                "[options] nouveau fichier de log : {} (ancien thread watcher \
                                 abandonné, session oubliée, rattrapage complet du nouveau fichier)",
                                new_path.display()
                            );
                            // Le nouveau fichier est relu depuis sa première ligne : la session
                            // repart de zéro AVANT, sinon chaque combat qu'il contient s'ajouterait
                            // à ce que l'ancien fichier avait déjà construit — et s'il s'agit du
                            // même fichier sous un autre chemin, se dupliquerait ligne à ligne
                            // (voir `Engine::forget_session`).
                            engine.forget_session();
                            snapshot.store(Arc::new(engine.snapshot()));
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
                        if !batch.is_initial_load && !log_replayed {
                            log_replayed = true;
                            startup.mark_log_replayed();
                            let _ = proxy.send_event(UserEvent::StartupProgress);
                        }
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
                        // **Toujours drainé, même Suivi coupé** : laisser les alertes
                        // s'accumuler dans le moteur les ferait toutes sortir d'un coup à la
                        // réactivation, des heures après le ramassage qui les a produites. Voir
                        // `EngineCommand::SetFeatures` — l'interrupteur met en sourdine, il ne met
                        // pas en file d'attente.
                        for alert in engine.drain_watchlist_alerts() {
                            if !features.suivi {
                                continue;
                            }
                            tracing::info!(
                                name = %alert.name,
                                reason = ?alert.reason,
                                "alerte de suivi (décompte à 0 ou objectif atteint)"
                            );
                            // **La carte s'affiche quoi qu'il arrive** : la sourdine ne coupe que
                            // le son (voir `EngineCommand::SetAlertMutes`).
                            if !mutes.suivi {
                                alert_sound::play_countdown_alert();
                            }
                            let created_at = std::time::Instant::now();
                            watchlist_toast.store(Arc::new(Some(WatchlistToast {
                                name: alert.name,
                                kind: alert.kind,
                                reason: match alert.reason {
                                    WatchlistAlertReason::Countdown => {
                                        WatchlistToastReason::Countdown
                                    }
                                    WatchlistAlertReason::Goal => WatchlistToastReason::Goal,
                                },
                                catalog_id: alert.catalog_id,
                                created_at,
                                confetti: panels::watchlist::build_confetti(),
                                hide_at: local_toast_deadline(created_at, &countdown_toast),
                            })));
                        }
                        // Ramassage d'un objet à son activé (compte) — INDÉPENDANT de la
                        // watchlist ci-dessus, même mécanisme de toast (un seul emplacement
                        // affiché à la fois : le plus récent des deux écrase l'autre, jamais de
                        // file d'attente — acceptable, ces alertes sont rares et ≤ 5 s chacune).
                        for alert in engine.drain_loot_alerts() {
                            // Même règle que ci-dessus : drainé quoi qu'il arrive, silencieux
                            // quand la fonctionnalité est coupée.
                            if !features.alerts {
                                continue;
                            }
                            tracing::info!(
                                name = %alert.name,
                                quantity = alert.quantity,
                                sound = alert.sound_enabled,
                                "alerte de ramassage"
                            );
                            // **La carte s'affiche quoi qu'il arrive**, comme pour le Suivi et
                            // le Chat ci-dessus/ci-dessous : un objet en mode silencieux
                            // (`panels::alerts_tab`) ne coupe que le son — voir
                            // `overlay_engine::LootAlert::sound_enabled`.
                            if alert.sound_enabled {
                                alert_sound::play_loot_alert();
                            }
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
                                hide_at: toast_deadline(created_at, &alert_profile),
                            })));
                        }
                        // Message de chat correspondant à une recherche (compte, onglet « Chat »)
                        // — INDÉPENDANT des deux alertes ci-dessus, même emplacement de toast.
                        // **Un seul son par lot** (règle du web, `ChatPanelComponent`) et la
                        // carte du DERNIER message trouvé : un lot en rafale ne doit pas
                        // empiler cinq sons.
                        // Drainé même recherche coupée (voir les deux blocs ci-dessus), puis
                        // jeté : c'est `features.chat` qui décide si le lot fait du bruit.
                        let chat_alerts = engine.drain_chat_alerts();
                        let chat_alerts = if features.chat {
                            chat_alerts
                        } else {
                            Vec::new()
                        };
                        for alert in &chat_alerts {
                            tracing::info!(
                                author = %alert.author,
                                word = %alert.filter.text,
                                channel = overlay_engine::channel_label(alert.channel),
                                "alerte de chat (recherche trouvée)"
                            );
                        }
                        if let Some(alert) = chat_alerts.into_iter().last() {
                            // Même règle que pour le décompte : muette, l'alerte garde sa carte.
                            if !mutes.chat {
                                alert_sound::play_chat_alert();
                            }
                            let created_at = std::time::Instant::now();
                            watchlist_toast.store(Arc::new(Some(WatchlistToast {
                                name: alert.author.clone(),
                                kind: WatchlistKind::Item,
                                reason: WatchlistToastReason::Chat {
                                    channel: alert.channel,
                                    word: alert.filter.text,
                                    author: alert.author,
                                    message: alert.message,
                                },
                                catalog_id: None,
                                created_at,
                                // Pas de confettis : ce n'est pas une célébration.
                                confetti: Vec::new(),
                                hide_at: local_toast_deadline(created_at, &chat_toast),
                            })));
                        }
                        let _ = proxy.send_event(UserEvent::NewSnapshot);
                    }
                    Ok(Err(err)) => tracing::warn!(%err, "erreur de lecture de wakfu.log"),
                    Err(RecvTimeoutError::Timeout) => {
                        if !log_replayed {
                            log_replayed = true;
                            tracing::info!("rattrapage initial de wakfu.log terminé");
                            startup.mark_log_replayed();
                            let _ = proxy.send_event(UserEvent::StartupProgress);
                        }
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break, // watcher arrêté (process en fin de vie)
                }
            }
        })
        .expect("échec de création du thread Engine");
}
