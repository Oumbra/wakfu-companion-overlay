//! `wakfu-companion-overlay-x11` — binaire Linux/X11 (docs/plan-architecture.md §17.2, Niveau 2 :
//! critère de sortie de S3, point « multi-fenêtres réel côté `overlay-ui` »). Réutilise de la LIB
//! tout ce qui ne dépend pas de l'OS (voir la doc de `lib.rs`) : `frame::render`, `engine_thread::
//! spawn_engine_thread`, `game_window` n'est PAS réutilisé ici — voir plus bas pourquoi ce binaire
//! parle directement à `overlay_platform::linux::x11` — `panels`, `render_content`, `logging`.
//! Fenêtrage réel via winit natif X11 (`WindowLevel`, `set_cursor_hittest`, `with_x11_window_type`)
//! — mêmes primitives déjà validées par `spikes/s3-window-linux/`, voir son README pour la preuve
//! programmatique sous Xvfb (Shape, stacking, focus).
//!
//! **Compte, synchro et catalogue depuis le 2026-09-14** : les threads de fond sont partagés avec
//! Windows (`overlay_ui::background` — Auth, Sync, Catalogue, Donjons) et la fenêtre de connexion
//! (`OverlayKind::Login`, `panels::login`, §9.1 undecies du plan) est câblée ici comme là-bas :
//! écran de chargement au lancement, compte obligatoire (plus de mode invité), overlays de jeu
//! seulement compte lié, retour à la fenêtre de connexion à la déconnexion
//! (`App::sync_session_windows`). Le fenêtrage OS de cette fenêtre est dupliqué (comme tout le
//! fenêtrage de ce fichier, voir `lib.rs`) : fenêtre X11 ordinaire (pas `Utility`), donc présente
//! dans la barre des tâches, centrée sur l'écran principal, icône de fenêtre = logo du site.
//!
//! **Ce qui reste propre à Windows, documenté honnêtement (§17.2 « État ») :**
//! - Pas d'icône de zone de notification : `tray-icon` tire GTK/libappindicator sous Linux et,
//!   sous GNOME, une telle icône dépend d'une extension. « Quitter » passe par le raccourci
//!   global, la fermeture de la fenêtre de connexion (`CloseRequested`) ou Ctrl+C ; « Déconnecter »
//!   par la section « Compte » de la fenêtre Options.
//! - Pas de hotkey rafraîchissement/détails (`ShortcutAction::LINUX_SUPPORTED` seulement :
//!   bascule, sortie, Options, sélection multiple du bandeau, invitation/suivi multicompte) —
//!   les autres restent personnalisables et persistées, simplement inertes ici.
//!
//! Ce que ce binaire couvre RÉELLEMENT (pas un stub, pas un panneau de diagnostic comme le spike
//! S3) : ingestion + moteur réels sur un vrai `wakfu.log`, icônes réelles d'objets/monstres
//! (`RemoteIconStore::spawn`), panneaux Combat/Suivi réels
//! (`paint_content`, la même fonction que Windows et qu'`overlay-testkit`), une fenêtre overlay
//! PAR fenêtre de jeu trouvée créée/détruite dynamiquement (`sync_windows`, même politique que
//! `main.rs::App::sync_windows`), ancrage identique (bord gauche pour Combat, bord haut pour
//! Suivi), topmost focus-aware avec délai de grâce (`overlay_platform::linux::topmost::decide`,
//! déjà testé unitairement) et click-through réel (`Window::set_cursor_hittest`, extension Shape
//! X11 sous le capot — voir `spikes/s3-window-linux/README.md` §"Click-through").

#[cfg(not(target_os = "windows"))]
fn main() {
    linux_main::run();
}

#[cfg(target_os = "windows")]
fn main() {
    eprintln!(
        "wakfu-companion-overlay-x11 est réservé à Linux/X11 (voir docs/plan-architecture.md \
         §17.2) — utilisez le binaire wakfu-companion-overlay sous Windows."
    );
    std::process::exit(1);
}

#[cfg(not(target_os = "windows"))]
mod linux_main {
    use std::collections::HashMap;
    use std::env;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;

    use arc_swap::ArcSwap;
    use egui_wgpu::wgpu;
    use global_hotkey::GlobalHotKeyEvent;
    use overlay_engine::{CatalogIndex, DungeonIndex, SessionSnapshot, WatchlistEntry};
    use overlay_ingest::discovery;
    use overlay_platform::linux::topmost::{self, TopmostAction, TopmostState};
    use overlay_platform::linux::x11::{GameRect, GameWindowTracker};
    use overlay_sync::update::{apply as update_apply, UpdateStatus};
    use overlay_ui::avatars::AvatarAtlas;
    use overlay_ui::background::{
        spawn_auth_thread, spawn_catalog_thread, spawn_dungeon_thread, spawn_sync_thread,
        spawn_update_thread, UpdateCommand,
    };
    use overlay_ui::build_info;
    use overlay_ui::chat_command::{self, ChatCommand};
    use overlay_ui::combat_placement;
    use overlay_ui::config;
    use overlay_ui::engine_thread::{
        spawn_engine_thread, EngineCommand, EngineHandles, SharedAlertProfile, SharedChatFilters,
        SharedRosterDraft, WatchlistCompleted,
    };
    use overlay_ui::frame::{render, GpuState};
    use overlay_ui::game_servers::GameServers;
    use overlay_ui::logging;
    use overlay_ui::panels;
    use overlay_ui::panels::alerts_tab;
    use overlay_ui::panels::chat_tab;
    use overlay_ui::panels::combat::{CombatMetric, CombatSide};
    use overlay_ui::panels::combat_frame::CombatFrame;
    use overlay_ui::panels::feature_switch::FeatureToggles;
    use overlay_ui::panels::login::{self, LoginState};
    use overlay_ui::panels::notifications::AlertMutes;
    use overlay_ui::panels::options_modal::{self, OptionsModalAction, OptionsModalState};
    use overlay_ui::panels::personnages_tab::{PersonnagesAvailability, PersonnagesTabState};
    use overlay_ui::panels::suivi_tab;
    use overlay_ui::panels::watchlist::WatchlistToast;
    use overlay_ui::portraits::PortraitAtlas;
    use overlay_ui::recap_placement;
    use overlay_ui::recap_session::{self, RecapSession};
    use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
    use overlay_ui::render_content;
    use overlay_ui::render_content::{
        AuthCommand, AuthStatus, OverlayKind, RenderContent, ResetTarget, UserEvent,
    };
    use overlay_ui::shortcuts::{ShortcutAction, ShortcutBindings, ShortcutRegistry};
    use overlay_ui::startup::StartupProgress;
    use overlay_ui::ui_icons::{self, UiIcons};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::application::ApplicationHandler;
    use winit::dpi::PhysicalPosition;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::platform::x11::{EventLoopBuilderExtX11, WindowAttributesExtX11, WindowType};
    use winit::window::{Icon, Window, WindowAttributes, WindowId, WindowLevel};

    // Combinaisons : voir `overlay_ui::shortcuts` (personnalisables depuis le 2026-09-13, onglet
    // « Raccourcis » de la fenêtre Options). Ce binaire n'enregistre que les actions de
    // `ShortcutAction::LINUX_SUPPORTED` — bascule, quitter, Options et la sélection multiple du
    // bandeau ; les autres n'ont pas de câblage ici (voir la doc de module).
    /// Même cadence que Windows (§6.5 du plan) : 20 Hz pour l'ancrage/topmost/hotkey.
    /// Écart entre deux images pendant qu'une tuile du Suivi célèbre son aboutissement — 60 Hz,
    /// voir `main.rs`. Demandé le temps de la célébration seulement, jamais en continu.
    const COMPLETION_FRAME: std::time::Duration = std::time::Duration::from_millis(16);

    const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
    // Hauteur élargie de `render_content::COMBAT_TOP_MARGIN` (2026-09-06, retour utilisateur :
    // tooltips du switch Alliés/Ennemis affichées en dessous faute de place au-dessus, même
    // correctif que `main.rs::WINDOW_SIZE`) — voir sa doc.
    const WINDOW_SIZE: (f64, f64) = (420.0, 480.0 + render_content::COMBAT_TOP_MARGIN as f64);
    // Voir `main.rs::WATCHLIST_HEIGHT` — même réserve sous la bande pour ses infobulles.
    const WATCHLIST_HEIGHT: f64 = 92.0 + render_content::WATCHLIST_TOOLTIP_RESERVE as f64;
    const WATCHLIST_INNER_MARGIN: f64 = 12.0;
    const WATCHLIST_WIDTH_FRACTION: f64 = 0.5;
    const WATCHLIST_MAX_CEILING: f64 = 1000.0;
    const GAME_TOP_MARGIN_PX: i32 = 28;
    // `GAME_EDGE_MARGIN_PX` (12 px ici, 0 sous Windows depuis le 2026-09-04) a disparu le
    // 2026-09-17 avec l'ancrage du panneau Combat, parti dans `overlay_ui::combat_placement` :
    // la marge y est nulle des deux côtés, comme l'utilisateur l'avait demandé pour Windows —
    // ce binaire ne l'avait jamais suivi.
    // L'ancrage du bloc Récap et le décalage que l'utilisateur lui donne à la souris
    // (2026-09-17) vivent dans `overlay_ui::recap_placement`, partagé avec `main.rs` :
    // `DEFAULT_OFFSET` a remplacé les constantes `GAME_RECAP_*_MARGIN_PX` qui étaient ici, et
    // porte tout leur relevé sur capture.

    /// Même calcul que `main.rs::watchlist_target_width` (voir sa doc pour le détail) — dupliqué
    /// plutôt que partagé : petite fonction pure, coût de duplication largement inférieur au coût
    /// d'une abstraction supplémentaire pour un si petit nombre de lignes.
    fn watchlist_target_width(
        entry_count: usize,
        tracking_enabled: bool,
        toast_active: bool,
        game_width_px: i32,
    ) -> f64 {
        let ceiling = (game_width_px as f64 * WATCHLIST_WIDTH_FRACTION).min(WATCHLIST_MAX_CEILING);
        let tiles = panels::watchlist::content_width(entry_count, tracking_enabled) as f64;
        let toast = if toast_active {
            panels::watchlist::TOAST_LAYER_WIDTH as f64
        } else {
            0.0
        };
        let content = tiles.max(toast) + WATCHLIST_INNER_MARGIN;
        content.min(ceiling).max(WATCHLIST_INNER_MARGIN)
    }

    fn watchlist_target_height(toast_active: bool, select_open: bool) -> f64 {
        WATCHLIST_HEIGHT
            + if toast_active {
                panels::watchlist::TOAST_AREA_HEIGHT as f64
            } else {
                0.0
            }
            // La bande du bouton de suppression groupée, le temps de la sélection multiple — la
            // fenêtre se rétracte en quittant le mode (2026-09-13).
            + if select_open {
                panels::watchlist::SELECTION_BAR_HEIGHT as f64
            } else {
                0.0
            }
    }

    /// Voir `main.rs::format_alert_duration` — dupliqué, comme le reste du fenêtrage.
    fn format_alert_duration(seconds: f32) -> String {
        if seconds.fract().abs() < f32::EPSILON {
            format!("{}", seconds as i64)
        } else {
            format!("{seconds:.1}").replace('.', ",")
        }
    }

    struct OverlayWindow {
        window: Arc<Window>,
        gpu: GpuState,
        kind: OverlayKind,
        portraits: PortraitAtlas,
        combat_frame: CombatFrame,
        icons: UiIcons,
        /// Les bustes de classe de l'onglet « Personnages » — **`Some` seulement pour la fenêtre
        /// Options**, la seule qui les affiche (voir `overlay_ui::avatars`, doc de module) : 36 PNG
        /// décodés et 1,8 Mo de textures par fenêtre de jeu seraient payés pour rien.
        avatars: Option<AvatarAtlas>,
        remote_icon_textures: RemoteIconTextures,
        combat_side: CombatSide,
        /// Grandeur mesurée par le panneau Combat — dégâts, armure donnée ou soins (voir
        /// `panels::combat::CombatMetric`). Un état par fenêtre, comme `combat_side` : deux
        /// personnages peuvent regarder deux grandeurs différentes du même combat.
        combat_metric: CombatMetric,
        /// État de la modale Options (2026-09-08) — `Some` UNIQUEMENT pour `kind ==
        /// OverlayKind::Options`, voir `App::open_options_modal`.
        options_state: Option<OptionsModalState>,
        /// État de la fenêtre de connexion — `Some` UNIQUEMENT pour `kind == OverlayKind::Login`,
        /// voir `App::create_login_window` (même règle qu'`options_state`).
        login_state: Option<LoginState>,
        /// Dernière hauteur demandée pour la fenêtre de connexion — voir `main.rs`.
        last_login_height: Option<f32>,
        /// XID X11 de la fenêtre de jeu — équivalent du `hwnd` côté Windows (voir
        /// `overlay_platform::linux::x11::GameWindowInfo`).
        ///
        /// **Renseigné pour la modale Options aussi depuis le 2026-09-12** : elle est rattachée à
        /// la fenêtre depuis laquelle on l'a ouverte, comme n'importe quel overlay. Elle portait
        /// `0` jusque-là, ce qui obligeait `sync_windows`/`sync_topmost` à l'exclure de toute leur
        /// logique — et la laissait au-dessus de TOUT, y compris quand l'utilisateur était passé
        /// sur une autre application. Reste `0` pour la fenêtre de connexion et pour la modale
        /// ouverte sans aucun client à l'écran (2026-09-17, voir `is_detached`).
        game_window: u32,
        game_rect: GameRect,
        character_name: String,
        last_position: Option<PhysicalPosition<i32>>,
        last_watchlist_width: Option<f64>,
        last_watchlist_height: Option<f64>,
        /// Voir `main.rs::OverlayWindow::last_recap_height` — la hauteur que le bloc Récap a
        /// mesurée à sa dernière frame.
        last_recap_height: Option<f32>,
        /// La fenêtre est-elle actuellement affichée ? — toujours `true` sauf pour une fenêtre
        /// `Combat` hors combat quand l'option est décochée (le défaut), voir
        /// `App::sync_panel_visibility`. Même rôle que dans `main.rs` : éviter un `set_visible`
        /// par tick alors que rien n'a changé.
        visible: bool,
        /// État topmost/délai de grâce — voir `overlay_platform::linux::topmost` (7 tests
        /// unitaires, déjà couvert avant ce binaire).
        topmost_state: TopmostState,
        next_redraw_at: Option<std::time::Instant>,
    }

    impl OverlayWindow {
        /// Voir `main.rs::OverlayWindow::is_detached` : aucune fenêtre de jeu derrière
        /// (`game_window == 0`) — la fenêtre de connexion, et la fenêtre Options ouverte sans
        /// client Wakfu à l'écran (2026-09-17). Laissée en place par `sync_windows`, jamais
        /// rétrogradée par `sync_topmost`.
        fn is_detached(&self) -> bool {
            self.game_window == 0
        }
    }

    struct App {
        windows: HashMap<WindowId, OverlayWindow>,
        /// Voir `overlay_ui::shortcuts::ShortcutRegistry` (partagé avec `main.rs`) — combinaisons
        /// effectives, enregistrement auprès de l'OS et table `id -> action`.
        hotkeys: ShortcutRegistry,
        hotkey_events: &'static global_hotkey::GlobalHotKeyEventReceiver,
        interactive: bool,
        snapshot: Arc<ArcSwap<SessionSnapshot>>,
        watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
        watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
        /// Sélection multiple du bandeau — ici et pas dans `OverlayWindow` : elle se pilote aussi
        /// au raccourci global, qui arrive par la boucle d'événements sans savoir quelle fenêtre
        /// existe. Il n'y a de toute façon qu'un bandeau Suivi à la fois.
        watchlist_selection: panels::watchlist::WatchlistSelection,
        /// **Les célébrations de complétion en cours** (2026-09-17) — alimentées par
        /// `completions_rx`, avancées par `tick_watchlist_completions`, lues par le rendu du bandeau.
        watchlist_completions: panels::watchlist::WatchlistCompletions,
        /// Par où les complétions arrivent du thread Engine — voir
        /// `engine_thread::WatchlistCompleted`.
        completions_rx: mpsc::Receiver<WatchlistCompleted>,
        /// Voir `main.rs::App::watchlist_reset_pending` — l'entrée dont la réinitialisation attend
        /// confirmation (2026-09-18).
        watchlist_reset_pending: Option<WatchlistEntry>,
        catalog: Arc<ArcSwap<CatalogIndex>>,
        /// Voir `main.rs::App::catalog_stale` — indicateur « catalogue daté » de la zone Combat.
        catalog_stale: Arc<AtomicBool>,
        remote_icons: RemoteIconStore,
        /// Publié par le thread Auth (`overlay_ui::background::spawn_auth_thread`) — pilote la
        /// fenêtre de connexion et l'existence même des overlays de jeu.
        auth_status: Arc<ArcSwap<AuthStatus>>,
        auth_command_tx: mpsc::Sender<AuthCommand>,
        /// Avancement des chargements initiaux — voir `overlay_ui::startup`.
        startup: Arc<StartupProgress>,
        /// État de la mise à jour automatique — voir `main.rs::App::update_status`.
        update_status: Arc<ArcSwap<UpdateStatus>>,
        update_command_tx: mpsc::Sender<UpdateCommand>,
        /// Voir `main.rs::App::auto_update`.
        auto_update: bool,
        /// Voir `main.rs::App::verbose_log`.
        verbose_log: bool,
        /// Profil d'alerte et recherches de chat du compte — voir `main.rs::App::alert_profile`.
        alert_profile: SharedAlertProfile,
        chat_filters: SharedChatFilters,
        /// Le roster du compte sous sa forme éditable, pour l'onglet « Personnages » — voir
        /// `engine_thread::SharedRosterDraft`.
        roster_draft: SharedRosterDraft,
        /// Les serveurs de jeu proposés par le sélecteur du même onglet — voir
        /// `background::spawn_game_servers_thread`.
        game_servers: Arc<ArcSwap<GameServers>>,
        /// Voir la doc de `AppState::settings_tx` — permet à `validate_and_commit_options`
        /// d'envoyer `EngineCommand::ChangeLogPath` sans redémarrer tout le binaire.
        settings_tx: mpsc::Sender<EngineCommand>,
        log_path: PathBuf,
        /// Le panneau Combat reste-t-il affiché en dehors des combats ? — réglage LOCAL persisté
        /// (`config::OverlayConfig::combat_always_visible`), même politique que `main.rs` : lu au
        /// démarrage, remplacé à la validation de la fenêtre Options, `false` par défaut.
        combat_always_visible: bool,
        /// Le panneau Combat est-il posé à DROITE de la fenêtre de jeu ? — réglage LOCAL persisté
        /// (`config::OverlayConfig::combat_on_right`), même politique que `main.rs` : ancrage de la
        /// fenêtre (`anchor_position`) et miroir de son contenu (`overlay_ui::mirror`).
        combat_on_right: bool,
        /// Voir `main.rs::App::combat_position_y` — la hauteur du panneau Combat (2026-09-17).
        combat_position_y: Option<i32>,
        /// Voir `main.rs::App::combat_locked` — le cadenas du panneau Combat.
        combat_locked: bool,
        /// Le glissement du panneau en cours, s'il y en a un — voir `CombatDragState`.
        combat_drag: Option<CombatDragState>,
        /// Prévenir par une notification du système qu'un personnage doit jouer ? — réglage LOCAL
        /// persisté (`config::OverlayConfig::turn_notification`), même politique que
        /// `combat_always_visible`.
        turn_notification: bool,
        /// Voir `AppState::turn_notification_muted`.
        turn_notification_muted: bool,
        /// Les trois interrupteurs de fonctionnalité — voir `main.rs::App::features`, même
        /// politique et mêmes deux effets (bandeau sans tuile, alertes en sourdine).
        features: FeatureToggles,
        /// Les deux sourdines — voir `main.rs::App::alert_mutes`, même politique et même effet
        /// unique : le thread Engine ne joue plus le son de l'alerte concernée, sa carte reste.
        alert_mutes: AlertMutes,
        /// Réglages de la carte d'alerte de chat en vigueur — voir `main.rs::App::chat_toast`.
        chat_toast: chat_tab::ChatToastSettings,
        /// Réglages de la carte de décompte à zéro en vigueur — voir
        /// `main.rs::App::countdown_toast`.
        countdown_toast: suivi_tab::CountdownToastSettings,
        /// **Ce que devient un suivi complété** EN VIGUEUR (retrait, animation) — même provenance et
        /// même politique que `countdown_toast`. Lu par `about_to_wait` à chaque complétion reçue du
        /// thread Engine : c'est lui qui décide s'il y a une célébration à jouer et un retrait à
        /// envoyer (voir `panels::suivi_tab::CompletionSettings`).
        completion: suivi_tab::CompletionSettings,
        game_window: GameWindowTracker,
        /// Le scan précédent trouvait au moins une fenêtre de jeu — voir
        /// `main.rs::App::game_was_present`.
        game_was_present: bool,
        /// La session du Récap — voir `main.rs::App::recap_session` (2026-09-17).
        recap_session: RecapSession,
        /// Où l'utilisateur a posé la bande Récap — voir `main.rs::App::recap_position`
        /// (2026-09-17) : décalage du bloc depuis le coin de la zone cliente du jeu, `None` tant
        /// qu'il ne l'a pas déplacée, une seule position pour toutes les fenêtres de jeu.
        recap_position: Option<(i32, i32)>,
        /// Voir `main.rs::App::recap_locked` — le cadenas de la bande Récap (2026-09-17).
        recap_locked: bool,
        /// Le glissement de la bande en cours, s'il y en a un — voir `RecapDragState`.
        recap_drag: Option<RecapDragState>,
        banner_printed: bool,
        /// Dialogue de fichier natif (`rfd`) en cours, le cas échéant — voir
        /// `App::start_file_dialog`. Un seul à la fois (une seule modale Options peut être ouverte,
        /// voir `open_options_modal`), sondé sans bloquer à chaque `about_to_wait` (même motif que
        /// `hotkey_events`).
        pending_dialog: Option<mpsc::Receiver<Option<PathBuf>>>,
        /// Résolution des ingrédients d'une recette en vol — voir `main.rs::App::pending_recipe`.
        pending_recipe: Option<(
            WindowId,
            mpsc::Receiver<Vec<overlay_engine::RecipeIngredient>>,
        )>,
        /// Écran de mise à jour ouvert à la demande — voir `main.rs::App::manual_update`, même
        /// rôle et même effet sur `sync_session_windows`.
        ///
        /// **Aucun déclencheur sous Linux aujourd'hui** : cet écran s'ouvre par l'entrée « Mise à
        /// jour » du menu de la zone de notification, et cet hôte n'en a pas (voir la doc de
        /// module). La mécanique est portée quand même — les deux hôtes ne divergent pas, et le
        /// jour où un accès Linux existe (raccourci, bouton d'un panneau), il n'y a qu'à poser ce
        /// drapeau.
        manual_update: bool,
    }

    /// Ce qu'il faut savoir pour poser une fenêtre `Combat` — voir
    /// `main.rs::CombatAnchor`, même type et même rôle : traduire le rectangle de fenêtre de jeu
    /// de CETTE plateforme dans le vocabulaire d'`overlay_ui::combat_placement`, qui fait le
    /// calcul (bord vertical, hauteur, bornage, aimantation) pour les deux hôtes.
    #[derive(Debug, Clone, Copy)]
    struct CombatAnchor {
        on_right: bool,
        offset: Option<i32>,
        scale: f64,
    }

    impl Default for CombatAnchor {
        fn default() -> Self {
            Self {
                on_right: false,
                offset: None,
                scale: 1.0,
            }
        }
    }

    impl CombatAnchor {
        fn new(on_right: bool, offset: Option<i32>, scale: f64) -> Self {
            Self {
                on_right,
                offset,
                scale,
            }
        }

        /// `rect.top`/`rect.height` — la fenêtre de jeu ENTIÈRE, le repère sur lequel ce panneau
        /// est centré depuis l'origine (voir `main.rs::CombatAnchor::geometry`).
        fn geometry(
            self,
            rect: GameRect,
            overlay_width: i32,
            overlay_height: i32,
        ) -> (combat_placement::ClientArea, combat_placement::Panel) {
            (
                combat_placement::ClientArea {
                    left: rect.left,
                    top: rect.top,
                    width: rect.width,
                    height: rect.height,
                },
                combat_placement::Panel::new(overlay_width, overlay_height, self.scale),
            )
        }
    }

    /// Un glissement du panneau Combat en cours — voir `main.rs::CombatDragState`, même rôle et
    /// même raison d'être en une seule dimension.
    #[derive(Debug, Clone, Copy)]
    struct CombatDragState {
        window: WindowId,
        grab_y: i32,
    }

    /// `main.rs::RecapAnchor`, même type et même rôle : traduire le rectangle de fenêtre de jeu
    /// de CETTE plateforme dans le vocabulaire d'`overlay_ui::recap_placement`, qui fait le
    /// calcul pour les deux.
    #[derive(Debug, Clone, Copy)]
    struct RecapAnchor {
        offset: Option<(i32, i32)>,
        scale: f64,
    }

    impl Default for RecapAnchor {
        fn default() -> Self {
            Self {
                offset: None,
                scale: 1.0,
            }
        }
    }

    impl RecapAnchor {
        fn new(offset: Option<(i32, i32)>, scale: f64) -> Self {
            Self { offset, scale }
        }

        fn geometry(
            self,
            rect: GameRect,
            overlay_width: i32,
            overlay_height: i32,
        ) -> (recap_placement::ClientArea, recap_placement::Band) {
            (
                recap_placement::ClientArea {
                    left: rect.left,
                    top: rect.client_top,
                    width: rect.width,
                    height: rect.top + rect.height - rect.client_top,
                },
                recap_placement::Band::new(overlay_width, overlay_height, self.scale),
            )
        }
    }

    /// Un glissement de la bande Récap en cours — voir `main.rs::RecapDragState`, même rôle et
    /// même raison : le geste se raconte en positions, jamais en écarts cumulés.
    #[derive(Debug, Clone, Copy)]
    struct RecapDragState {
        /// La fenêtre `Recap` saisie.
        window: WindowId,
        /// Position de saisie DANS la fenêtre, en pixels physiques.
        grab: (i32, i32),
    }

    struct AppState {
        log_path: PathBuf,
        /// Voir `App::combat_always_visible` — lu de la config au démarrage (`run`).
        combat_always_visible: bool,
        /// Voir `App::combat_on_right` — même provenance.
        combat_on_right: bool,
        /// Voir `App::combat_position_y` — relue de la config au démarrage.
        combat_position_y: Option<i32>,
        /// Voir `App::combat_locked` — relu de la config au démarrage.
        combat_locked: bool,
        /// Voir `App::turn_notification` — même provenance.
        turn_notification: bool,
        /// Le son de la notification de tour coupé (`config::OverlayConfig::
        /// turn_notification_muted`) — persisté, sans effet tant que la notification elle-même
        /// n'existe pas sous X11.
        turn_notification_muted: bool,
        /// Voir `App::features` — lus de la config au démarrage (`run`).
        features: FeatureToggles,
        /// Voir `App::alert_mutes` — lues de la config au démarrage (`run`).
        alert_mutes: AlertMutes,
        /// Réglages de la carte d'alerte de chat en vigueur — voir `main.rs::App::chat_toast`.
        chat_toast: chat_tab::ChatToastSettings,
        /// Réglages de la carte de décompte à zéro en vigueur — voir
        /// `main.rs::App::countdown_toast`.
        countdown_toast: suivi_tab::CountdownToastSettings,
        /// Voir `App::completion` — lus de la config au démarrage.
        completion: suivi_tab::CompletionSettings,
        /// Voir `App::completions_rx` — le canal créé par `run`, avant le thread Engine.
        completions_rx: mpsc::Receiver<WatchlistCompleted>,
        /// La session du Récap relue du disque — voir `main.rs::AppState::recap_session`.
        recap_session: RecapSession,
        /// La position de la bande Récap relue de la config — voir `App::recap_position`.
        recap_position: Option<(i32, i32)>,
        /// Le verrou de la bande Récap relu de la config — voir `App::recap_locked`.
        recap_locked: bool,
        /// Raccourcis EFFECTIFS au démarrage — défauts, ou personnalisation lue de `config.toml`.
        /// Même provenance que `combat_always_visible`.
        shortcuts: ShortcutBindings,
        snapshot: Arc<ArcSwap<SessionSnapshot>>,
        watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
        watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
        catalog: Arc<ArcSwap<CatalogIndex>>,
        catalog_stale: Arc<AtomicBool>,
        remote_icons: RemoteIconStore,
        auth_status: Arc<ArcSwap<AuthStatus>>,
        auth_command_tx: mpsc::Sender<AuthCommand>,
        startup: Arc<StartupProgress>,
        update_status: Arc<ArcSwap<UpdateStatus>>,
        update_command_tx: mpsc::Sender<UpdateCommand>,
        auto_update: bool,
        /// Voir `main.rs::App::verbose_log`.
        verbose_log: bool,
        alert_profile: SharedAlertProfile,
        chat_filters: SharedChatFilters,
        roster_draft: SharedRosterDraft,
        game_servers: Arc<ArcSwap<GameServers>>,
        game_window: GameWindowTracker,
        /// Canal vers le thread Engine (2026-09-08, §9 du plan) — voir
        /// `engine_thread::EngineCommand::ChangeLogPath`.
        settings_tx: mpsc::Sender<EngineCommand>,
    }

    impl App {
        fn new(state: AppState) -> Self {
            let AppState {
                log_path,
                combat_always_visible,
                combat_on_right,
                combat_position_y,
                combat_locked,
                turn_notification,
                turn_notification_muted,
                features,
                alert_mutes,
                chat_toast,
                countdown_toast,
                completion,
                completions_rx,
                recap_session,
                recap_position,
                recap_locked,
                shortcuts,
                snapshot,
                watchlist,
                watchlist_toast,
                catalog,
                catalog_stale,
                remote_icons,
                auth_status,
                auth_command_tx,
                startup,
                update_status,
                update_command_tx,
                auto_update,
                verbose_log,
                alert_profile,
                chat_filters,
                roster_draft,
                game_servers,
                game_window,
                settings_tx,
            } = state;

            let hotkeys = ShortcutRegistry::new(&ShortcutAction::LINUX_SUPPORTED, shortcuts);

            Self {
                windows: HashMap::new(),
                hotkeys,
                hotkey_events: GlobalHotKeyEvent::receiver(),
                watchlist_selection: panels::watchlist::WatchlistSelection::default(),
                watchlist_completions: Default::default(),
                completions_rx,
                watchlist_reset_pending: None,
                interactive: true,
                snapshot,
                watchlist,
                watchlist_toast,
                catalog,
                catalog_stale,
                remote_icons,
                auth_status,
                auth_command_tx,
                startup,
                update_status,
                update_command_tx,
                auto_update,
                verbose_log,
                alert_profile,
                chat_filters,
                roster_draft,
                game_servers,
                settings_tx,
                log_path,
                combat_always_visible,
                combat_on_right,
                combat_position_y,
                combat_locked,
                turn_notification,
                turn_notification_muted,
                features,
                alert_mutes,
                chat_toast,
                countdown_toast,
                completion,
                game_window,
                game_was_present: false,
                recap_session,
                recap_position,
                recap_locked,
                recap_drag: None,
                combat_drag: None,
                banner_printed: false,
                pending_dialog: None,
                pending_recipe: None,
                manual_update: false,
            }
        }

        /// **Installe la mise à jour prête et relance** — appelé à chaque tick d'`about_to_wait`, AVANT
        /// tout le reste. Le thread de mise à jour s'arrête à `ReadyToInstall` (exe vérifié sur le
        /// disque, voir `background::spawn_update_thread`) : le remplacement de l'exe courant et la
        /// relance doivent précéder `event_loop.exit()`, que seul ce thread peut appeler. À ce moment,
        /// `StartupProgress::is_update_blocking` est levé depuis le début du téléchargement : seule la
        /// fenêtre de connexion existe (voir `sync_session_windows`), aucun overlay de jeu n'est
        /// interrompu. Un échec de remplacement (antivirus, dossier protégé) repasse en `Failed` et
        /// rend la main : l'overlay continue avec sa version, ou reste sur « Mise à jour requise » si
        /// elle était obligatoire.
        fn install_update_if_ready(&mut self, event_loop: &ActiveEventLoop) {
            let status = self.update_status.load();
            let UpdateStatus::ReadyToInstall {
                version,
                staged,
                mandatory,
            } = &**status
            else {
                return;
            };
            tracing::info!(
                "[mise à jour] installation de {} depuis {} — l'overlay va se relancer.",
                version,
                staged.display()
            );
            self.update_status.store(Arc::new(UpdateStatus::Installing {
                version: version.clone(),
            }));
            match update_apply::install_and_relaunch(staged, build_info::VERSION) {
                Ok(()) => {
                    logging::log_session_end(&format!(
                        "mise à jour {} → {version}",
                        build_info::VERSION
                    ));
                    event_loop.exit();
                }
                Err(err) => {
                    tracing::warn!("[mise à jour] installation impossible : {err}");
                    self.update_status.store(Arc::new(UpdateStatus::Failed {
                        headline: "Installation impossible".to_string(),
                        detail: err.to_string(),
                        mandatory: *mandatory,
                    }));
                    self.startup.set_update_blocking(*mandatory);
                    if !*mandatory {
                        self.startup.mark_update_resolved();
                    }
                    self.request_all_redraw();
                }
            }
        }

        /// « Mettre à jour vers X » confirmé dans la fenêtre Options (`OptionsModalAction::InstallUpdate`)
        /// : la fenêtre se ferme, le démarrage repasse en « mise à jour en cours » (ce qui ferme les
        /// overlays de jeu et ramène la fenêtre de connexion sur son écran de chargement au prochain
        /// tick, voir `sync_session_windows`), et le thread de mise à jour télécharge. L'installation
        /// suit dans `install_update_if_ready`.
        /// Redessine toutes les fenêtres — un état partagé vient de changer hors d'un événement.
        fn request_all_redraw(&self) {
            for overlay in self.windows.values() {
                overlay.window.request_redraw();
            }
        }

        fn request_update_install(&mut self, options_window_id: WindowId) {
            tracing::info!(">>> Mise à jour demandée (fenêtre Options).");
            self.startup.set_update_blocking(true);
            let _ = self.update_command_tx.send(UpdateCommand::Download);
            self.close_options_modal(options_window_id, "Mise à jour");
        }

        /// Voir `main.rs::App::session_ready` : chargements initiaux terminés ET compte lié.
        fn session_ready(&self) -> bool {
            self.startup.is_complete() && self.auth_status.load().is_connected()
        }

        /// Voir `main.rs::App::sync_session_windows` — même logique, mêmes trois situations
        /// (chargement, compte lié, compte non lié), sans icône de zone de notification à
        /// tenir à jour.
        fn sync_session_windows(&mut self, event_loop: &ActiveEventLoop) {
            let auth = self.auth_status.load();
            let loading = !self.startup.is_complete() || matches!(**auth, AuthStatus::Connecting);
            let connected = !loading && auth.is_connected();
            let has_login = self.windows.values().any(|w| w.kind == OverlayKind::Login);
            if connected {
                // Seule exception à « compte lié = pas de fenêtre de connexion » : l'écran de
                // mise à jour demandé à la main (voir `App::manual_update`).
                if self.manual_update {
                    if !has_login {
                        self.create_login_window(event_loop);
                    }
                } else if has_login {
                    self.windows.retain(|_, w| w.kind != OverlayKind::Login);
                    tracing::info!(
                        "[connexion] compte lié et chargements terminés — fenêtre de connexion fermée, overlays de jeu activés."
                    );
                }
            } else {
                let had_options = self
                    .windows
                    .values()
                    .any(|w| w.kind == OverlayKind::Options);
                let before = self.windows.len();
                self.windows.retain(|_, w| w.kind == OverlayKind::Login);
                if self.windows.len() != before {
                    if had_options {
                        let _ = self.hotkeys.resume();
                    }
                    tracing::info!(
                        "[connexion] aucun compte lié — {} fenêtre(s) de jeu fermée(s), retour à la fenêtre de connexion.",
                        before - self.windows.len()
                    );
                }
                if !has_login {
                    self.create_login_window(event_loop);
                }
            }
            let manual_update = self.manual_update;
            for overlay in self.windows.values_mut() {
                if let Some(state) = overlay.login_state.as_mut() {
                    if state.loading != loading {
                        state.loading = loading;
                        overlay.window.request_redraw();
                        if !loading {
                            tracing::info!(
                                "[connexion] chargements terminés — écran de connexion."
                            );
                        }
                    }
                    if state.manual_update != manual_update {
                        state.manual_update = manual_update;
                        overlay.window.request_redraw();
                    }
                }
            }
        }

        /// Voir `main.rs::App::close_manual_update_window` — « Fermer » / « Plus tard » de l'écran
        /// de mise à jour.
        fn close_manual_update_window(&mut self, event_loop: &ActiveEventLoop) {
            if !self.manual_update {
                return;
            }
            tracing::info!("[mise à jour] écran de mise à jour refermé.");
            self.manual_update = false;
            self.sync_session_windows(event_loop);
        }

        /// Voir `main.rs::App::create_login_window` — fenêtre X11 ORDINAIRE (pas de type
        /// `Utility`, contrairement aux overlays) : barre des tâches, bascule de fenêtres, focus,
        /// z-order normal, centrée sur l'écran principal, sans décorations (la carte peint son
        /// bord et se déplace par sa bannière). Icône de fenêtre = logo du site.
        fn create_login_window(&mut self, event_loop: &ActiveEventLoop) {
            let (rgba, width, height) = ui_icons::app_logo_rgba();
            let icon = match Icon::from_rgba(rgba, width, height) {
                Ok(icon) => Some(icon),
                Err(err) => {
                    tracing::warn!("[connexion] icône de fenêtre refusée : {err}");
                    None
                }
            };
            let attrs = WindowAttributes::default()
                .with_title("Wakfu Companion Overlay")
                .with_inner_size(winit::dpi::LogicalSize::new(
                    login::WINDOW_WIDTH as f64,
                    login::INITIAL_HEIGHT as f64,
                ))
                .with_transparent(true)
                .with_decorations(false)
                .with_window_level(WindowLevel::Normal)
                .with_resizable(false)
                .with_visible(true)
                .with_window_icon(icon);
            let window = event_loop
                .create_window(attrs)
                .expect("création de la fenêtre de connexion");
            let window = Arc::new(window);
            if let Err(err) = window.set_cursor_hittest(true) {
                tracing::warn!("set_cursor_hittest a échoué à la création : {err}");
            }

            let gpu = pollster::block_on(init_gpu(Arc::clone(&window)));
            let portraits = PortraitAtlas::load(&gpu.egui_ctx);
            let combat_frame = CombatFrame::load(&gpu.egui_ctx);
            let icons = UiIcons::load(&gpu.egui_ctx);

            Self::center_on_primary_monitor(event_loop, &window);
            window.focus_window();

            let overlay = OverlayWindow {
                window,
                gpu,
                kind: OverlayKind::Login,
                portraits,
                combat_frame,
                icons,
                avatars: None,
                remote_icon_textures: RemoteIconTextures::default(),
                combat_side: CombatSide::default(),
                combat_metric: CombatMetric::default(),
                options_state: None,
                login_state: Some(LoginState::new(std::time::Instant::now())),
                last_login_height: Some(login::INITIAL_HEIGHT),
                game_window: 0,
                game_rect: GameRect {
                    left: 0,
                    top: 0,
                    width: 0,
                    height: 0,
                    client_top: 0,
                },
                character_name: "Connexion".to_string(),
                last_position: None,
                last_watchlist_width: None,
                last_watchlist_height: None,
                last_recap_height: None,
                visible: true,
                // Jamais promue : `sync_topmost` l'ignore, elle reste en z-order normal.
                topmost_state: TopmostState::Normal,
                next_redraw_at: None,
            };
            overlay.window.request_redraw();
            tracing::info!("[connexion] fenêtre de connexion ouverte.");
            self.windows.insert(overlay.window.id(), overlay);
        }

        /// Voir `main.rs::App::center_on_primary_monitor`.
        fn center_on_primary_monitor(event_loop: &ActiveEventLoop, window: &Window) {
            let monitor = event_loop
                .primary_monitor()
                .or_else(|| event_loop.available_monitors().next());
            let Some(monitor) = monitor else {
                return;
            };
            let origin = monitor.position();
            let screen = monitor.size();
            let outer = window.outer_size();
            window.set_outer_position(PhysicalPosition::new(
                origin.x + (screen.width as i32 - outer.width as i32) / 2,
                origin.y + (screen.height as i32 - outer.height as i32) / 2,
            ));
        }

        /// Même politique que `main.rs::App::sync_windows` (voir sa doc) : converge `self.windows`
        /// vers l'état actuel des fenêtres de jeu trouvées — **compte lié seulement**.
        fn sync_windows(&mut self, event_loop: &ActiveEventLoop) {
            if !self.session_ready() {
                return;
            }
            let found = self.game_window.scan();
            // Une fenêtre `Combat` créée hors combat naît masquée quand l'option est décochée —
            // voir `sync_panel_visibility`. Lu ici une fois pour toute la passe.
            let snapshot = self.snapshot.load();

            self.windows.retain(|_, overlay| {
                if overlay.kind == OverlayKind::Login || overlay.is_detached() {
                    return true; // n'appartient à aucune fenêtre de jeu
                }
                // **La modale Options suit la même règle depuis le 2026-09-12** : rattachée à la
                // fenêtre de jeu depuis laquelle on l'a ouverte, elle s'en va avec elle. Un écran
                // de réglages qui survivrait au client qu'il configure n'aurait plus de raison
                // d'être à l'écran. Seule exception, ci-dessus : la modale ouverte SANS client à
                // l'écran (`is_detached`), qui n'a aucune fenêtre de jeu derrière laquelle
                // disparaître (voir `main.rs::App::sync_windows`).
                let still_here = found
                    .iter()
                    .any(|(_, info)| info.window == overlay.game_window);
                if !still_here {
                    tracing::info!(
                        "[fenêtre de jeu] {} fermée — son overlay est retiré.",
                        overlay.character_name
                    );
                }
                still_here
            });

            for (character_name, info) in &found {
                for kind in [
                    OverlayKind::Combat,
                    OverlayKind::Watchlist,
                    OverlayKind::Recap,
                ] {
                    if let Some(existing) = self
                        .windows
                        .values_mut()
                        .find(|w| w.game_window == info.window && w.kind == kind)
                    {
                        Self::reposition(
                            existing,
                            info.rect,
                            self.combat_on_right,
                            self.combat_position_y,
                            self.recap_position,
                        );
                        continue;
                    }
                    let visible = match kind {
                        OverlayKind::Combat => panels::combat::should_show(
                            &snapshot,
                            character_name,
                            self.combat_always_visible,
                            self.features.combat,
                        ),
                        // La bande Récap ne dépend que de sa case — voir `main.rs`, même règle.
                        OverlayKind::Recap => self.features.recap,
                        _ => true,
                    };
                    let overlay = Self::create_overlay_window(
                        event_loop,
                        kind,
                        info.window,
                        character_name.clone(),
                        info.rect,
                        self.interactive,
                        visible,
                        self.combat_on_right,
                        self.combat_position_y,
                        self.recap_position,
                    );
                    tracing::info!(
                        "[fenêtre de jeu] {character_name} trouvée — overlay {kind:?} créé."
                    );
                    overlay.window.request_redraw();
                    self.windows.insert(overlay.window.id(), overlay);
                }
            }
            // La session du Récap suit ce balayage, et la fermeture du dernier client part au
            // moteur — voir `main.rs::App::sync_windows`.
            let game_present = !found.is_empty();
            self.recap_session.observe(
                game_present,
                &snapshot.totals,
                std::time::SystemTime::now(),
            );
            if self.game_was_present && !game_present {
                let _ = self.settings_tx.send(EngineCommand::GameClosed);
            }
            self.game_was_present = game_present;
        }

        /// Affiche ou masque les fenêtres dont la présence est CONDITIONNELLE — `Combat` (combat
        /// en cours pour SON personnage) et `Recap` (sa case à cocher, 2026-09-16) — même
        /// politique que `main.rs::App::sync_panel_visibility` (masquer plutôt que détruire), les
        /// règles elles-mêmes étant `panels::combat::should_show` et `features.recap`.
        ///
        /// Pas de recréation de surface ici, contrairement à Windows : ce chemin corrige un défaut
        /// propre à DirectComposition (voir `main.rs::sync_topmost`), sans équivalent sous X11.
        fn sync_panel_visibility(&mut self) {
            let snapshot = self.snapshot.load();
            let always = self.combat_always_visible;
            // Voir `main.rs::App::sync_panel_visibility` : même rôle, le détail des combats
            // coupé masque toutes les fenêtres Combat.
            let enabled = self.features.combat;
            let recap_enabled = self.features.recap;
            for overlay in self.windows.values_mut() {
                let wanted = match overlay.kind {
                    OverlayKind::Combat => panels::combat::should_show(
                        &snapshot,
                        &overlay.character_name,
                        always,
                        enabled,
                    ),
                    OverlayKind::Recap => recap_enabled,
                    _ => continue,
                };
                if wanted == overlay.visible {
                    continue;
                }
                overlay.window.set_visible(wanted);
                overlay.visible = wanted;
                if wanted {
                    overlay.window.request_redraw();
                }
                tracing::info!(
                    "[{}] {} — panneau {}",
                    if overlay.kind == OverlayKind::Recap {
                        "recap"
                    } else {
                        "combat"
                    },
                    overlay.character_name,
                    if wanted { "affiché" } else { "masqué" }
                );
            }
        }

        /// Même ancrage que Windows (voir `main.rs::App::anchor_position`) : Combat au bord
        /// gauche — ou DROIT si `combat_on_right` (2026-09-17) — centré verticalement, Suivi au
        /// bord haut centré horizontalement.
        fn anchor_position(
            kind: OverlayKind,
            rect: GameRect,
            overlay_width: i32,
            overlay_height: i32,
            combat: CombatAnchor,
            recap: RecapAnchor,
        ) -> PhysicalPosition<i32> {
            match kind {
                // Combat : bord vertical au choix, centré verticalement tant qu'il n'a pas été
                // déplacé, à sa hauteur ensuite (2026-09-17) — tout le calcul est dans
                // `overlay_ui::combat_placement`, voir `main.rs::App::anchor_position`.
                OverlayKind::Combat => {
                    let (client, panel) = combat.geometry(rect, overlay_width, overlay_height);
                    let (x, y) = combat_placement::window_position(
                        combat.offset,
                        combat.on_right,
                        client,
                        panel,
                    );
                    PhysicalPosition::new(x, y)
                }
                OverlayKind::Watchlist => PhysicalPosition::new(
                    rect.left + (rect.width - overlay_width) / 2,
                    rect.client_top + GAME_TOP_MARGIN_PX,
                ),
                // Récap : sous les boutons du jeu tant que l'utilisateur ne l'a pas déplacée,
                // là où il l'a posée ensuite — tout le calcul est dans
                // `overlay_ui::recap_placement`, voir `main.rs::App::anchor_position`.
                OverlayKind::Recap => {
                    let (client, band) = recap.geometry(rect, overlay_width, overlay_height);
                    let (x, y) = recap_placement::window_position(recap.offset, client, band);
                    PhysicalPosition::new(x, y)
                }
                // La confirmation de remise à zéro couvre la fenêtre de jeu entière — voir
                // `main.rs::App::anchor_position`.
                OverlayKind::ResetConfirm(_) => PhysicalPosition::new(rect.left, rect.top),
                // Rattachée à une fenêtre de jeu, la fenêtre Options la couvre entière et voile
                // tout sauf la modale, centrée par le rendu (2026-09-17) — voir
                // `main.rs::App::anchor_position`. Détachée, ce bras n'est pas lu.
                OverlayKind::Options => PhysicalPosition::new(rect.left, rect.top),
                // Jamais créée par ce binaire (voir la doc de module) — exhaustivité seulement.
                OverlayKind::Login => PhysicalPosition::new(0, 0),
            }
        }

        // Neuf paramètres depuis que le panneau Combat se pose à droite et que la bande Récap se
        // déplace (2026-09-17) : ce sont les caractéristiques d'UNE fenêtre à créer, toutes
        // distinctes et toutes obligatoires — voir `main.rs`, même remarque.
        #[allow(clippy::too_many_arguments)]
        fn create_overlay_window(
            event_loop: &ActiveEventLoop,
            kind: OverlayKind,
            game_window: u32,
            character_name: String,
            rect: GameRect,
            interactive: bool,
            visible: bool,
            // `combat_on_right` : voir `anchor_position`.
            combat_on_right: bool,
            // À quelle hauteur poser le panneau Combat (`App::combat_position_y`, 2026-09-17).
            combat_offset: Option<i32>,
            // Où poser la bande Récap (`App::recap_position`) — sans objet pour les autres zones.
            recap_offset: Option<(i32, i32)>,
        ) -> OverlayWindow {
            let size = match kind {
                OverlayKind::Combat => WINDOW_SIZE,
                OverlayKind::Watchlist => (
                    // Suivi supposé actif à la création — voir `main.rs`, même repli.
                    watchlist_target_width(0, true, false, rect.width),
                    watchlist_target_height(false, false),
                ),
                // Rattachée à une fenêtre de jeu : SA taille, pour que le voile la couvre en
                // entier (2026-09-17, voir `RenderContent::veiled` et `main.rs`). Détachée :
                // celle de la modale seule, sans voile.
                OverlayKind::Options if game_window != 0 => (rect.width as f64, rect.height as f64),
                OverlayKind::Options => (
                    options_modal::WINDOW_SIZE.0 as f64,
                    options_modal::WINDOW_SIZE.1 as f64,
                ),
                // Récap : largeur fixe, hauteur pilotée par le contenu dès la première frame —
                // voir `main.rs`, même mécanique.
                OverlayKind::Recap => (
                    panels::recap::WIDTH as f64,
                    panels::recap::HEIGHT as f64
                        + render_content::RECAP_TOOLTIP_RESERVE as f64
                        + render_content::RECAP_ACTIONS_RESERVE as f64,
                ),
                // La taille de la fenêtre de jeu, pour que le voile la couvre — voir `main.rs`.
                OverlayKind::ResetConfirm(_) => (rect.width as f64, rect.height as f64),
                // Jamais créée par ce binaire (voir la doc de module) — exhaustivité seulement.
                OverlayKind::Login => (
                    panels::login::WINDOW_WIDTH as f64,
                    panels::login::INITIAL_HEIGHT as f64,
                ),
            };
            // Une fenêtre qui couvre le jeu se mesure en pixels PHYSIQUES, comme le rectangle
            // dont elle vient et la position qu'on lui pose — voir `main.rs`, même raison.
            let covers_game = matches!(kind, OverlayKind::ResetConfirm(_))
                || (kind == OverlayKind::Options && game_window != 0);
            let inner_size: winit::dpi::Size = if covers_game {
                winit::dpi::PhysicalSize::new(size.0, size.1).into()
            } else {
                winit::dpi::LogicalSize::new(size.0, size.1).into()
            };
            let title_suffix = match kind {
                OverlayKind::Combat => "Combat",
                OverlayKind::Watchlist => "Suivi",
                OverlayKind::Recap => "Recap",
                OverlayKind::ResetConfirm(_) => "Confirmation",
                OverlayKind::Options => "Options",
                OverlayKind::Login => "Connexion",
            };
            let attrs = WindowAttributes::default()
                .with_title(format!(
                    "wakfu-companion-overlay — {character_name} — {title_suffix}"
                ))
                .with_inner_size(inner_size)
                .with_transparent(true)
                .with_decorations(false)
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_resizable(false)
                // Une fenêtre `Combat` peut naître MASQUÉE (aucun combat en cours, option
                // décochée — voir `sync_panel_visibility`) : demandé dès les attributs plutôt que
                // par un `set_visible(false)` juste après, qui la ferait clignoter à l'écran.
                .with_visible(visible)
                // `_NET_WM_WINDOW_TYPE_UTILITY` (§6.4 du plan) — absent du taskbar/alt-tab, comme
                // `with_skip_taskbar` côté Windows (non applicable ici, propriété EWMH distincte).
                .with_x11_window_type(vec![WindowType::Utility]);

            let window = event_loop
                .create_window(attrs)
                .expect("création de la fenêtre overlay");
            let window = Arc::new(window);

            if let Err(err) = window.set_cursor_hittest(interactive) {
                tracing::warn!("set_cursor_hittest a échoué à la création : {err}");
            }

            let gpu = pollster::block_on(init_gpu(Arc::clone(&window)));
            let portraits = PortraitAtlas::load(&gpu.egui_ctx);
            let combat_frame = CombatFrame::load(&gpu.egui_ctx);
            let icons = UiIcons::load(&gpu.egui_ctx);
            // Payés seulement là où ils servent — voir le champ `avatars` d'`OverlayWindow`.
            let avatars = (kind == OverlayKind::Options).then(|| AvatarAtlas::load(&gpu.egui_ctx));

            let outer = window.outer_size();
            let position = Self::anchor_position(
                kind,
                rect,
                outer.width as i32,
                outer.height as i32,
                CombatAnchor::new(combat_on_right, combat_offset, window.scale_factor()),
                RecapAnchor::new(recap_offset, window.scale_factor()),
            );
            window.set_outer_position(position);

            OverlayWindow {
                window,
                gpu,
                kind,
                portraits,
                combat_frame,
                icons,
                avatars,
                remote_icon_textures: RemoteIconTextures::default(),
                combat_side: CombatSide::default(),
                combat_metric: CombatMetric::default(),
                // Renseigné juste après par l'appelant (`open_options_modal`) pour `kind ==
                // Options` — `None` ici pour Combat/Suivi, jamais consulté (voir
                // `RenderContent::options`).
                options_state: (kind == OverlayKind::Options).then(OptionsModalState::default),
                login_state: None,
                last_login_height: None,
                game_window,
                game_rect: rect,
                character_name,
                last_position: Some(position),
                last_watchlist_width: (kind == OverlayKind::Watchlist).then_some(size.0),
                last_watchlist_height: (kind == OverlayKind::Watchlist).then_some(size.1),
                last_recap_height: (kind == OverlayKind::Recap).then_some(panels::recap::HEIGHT),
                visible,
                // `WindowLevel::AlwaysOnTop` déjà appliqué ci-dessus à la création — voir la doc
                // de `topmost::decide` pour la suite de la politique.
                topmost_state: TopmostState::Above {
                    pending_demote_since: None,
                },
                next_redraw_at: None,
            }
        }

        fn reposition(
            overlay: &mut OverlayWindow,
            rect: GameRect,
            combat_on_right: bool,
            combat_offset: Option<i32>,
            recap_offset: Option<(i32, i32)>,
        ) {
            overlay.game_rect = rect;
            let outer = overlay.window.outer_size();
            let scale = overlay.window.scale_factor();
            let desired = Self::anchor_position(
                overlay.kind,
                rect,
                outer.width as i32,
                outer.height as i32,
                CombatAnchor::new(combat_on_right, combat_offset, scale),
                RecapAnchor::new(recap_offset, scale),
            );
            if overlay.last_position != Some(desired) {
                overlay.window.set_outer_position(desired);
                overlay.last_position = Some(desired);
            }
        }

        fn xid_of(window: &Window) -> u32 {
            match window.window_handle().expect("handle de fenêtre").as_raw() {
                // winit (rwh_06, X11) renvoie toujours `Xlib`, jamais `Xcb` — Xlib et XCB parlent
                // au MÊME serveur X et partagent le même espace d'identifiants (XID), voir
                // `spikes/s3-window-linux/src/main.rs` pour la vérification faite dans son code
                // source.
                RawWindowHandle::Xlib(handle) => handle.window as u32,
                other => panic!("handle de fenêtre inattendu sous X11 : {other:?}"),
            }
        }

        /// Redessine la ou les fenêtres du bandeau Suivi — appelé après une bascule venue du
        /// clavier, que rien d'autre ne signale au moteur de rendu.
        fn request_watchlist_redraw(&self) {
            for overlay in self.windows.values() {
                if overlay.kind == OverlayKind::Watchlist {
                    overlay.window.request_redraw();
                }
            }
        }

        fn toggle_interactive(&mut self) {
            self.interactive = !self.interactive;
            for overlay in self.windows.values() {
                // La fenêtre de connexion n'est pas un overlay : toujours interactive.
                if overlay.kind == OverlayKind::Login {
                    continue;
                }
                if let Err(err) = overlay.window.set_cursor_hittest(self.interactive) {
                    tracing::warn!("set_cursor_hittest a échoué : {err}");
                }
                overlay.window.request_redraw();
            }
            tracing::info!(
                ">>> Bascule ({}) : mode = {}",
                self.hotkeys.bindings().label(ShortcutAction::Toggle),
                if self.interactive {
                    "INTERACTIF"
                } else {
                    "CLIC-TRAVERSANT"
                }
            );
        }

        /// Réponse en privé depuis la carte d'alerte de chat — voir `main.rs::whisper_from_toast` :
        /// la fenêtre de jeu active, ou la première trouvée.
        fn whisper_from_toast(&mut self, author: &str) {
            let active = self.game_window.active_window().unwrap_or(0);
            let windows: Vec<u32> = self
                .game_window
                .scan()
                .into_iter()
                .map(|(_, info)| info.window)
                .collect();
            let target = windows
                .iter()
                .copied()
                .find(|key| *key == active)
                .or_else(|| windows.first().copied())
                .map(|xid| xid as usize);
            // Le nom du destinataire est celui d'un TIERS (constat C6 de `docs/analyse-rgpd.md`) :
            // l'action se journalise, pas la personne visée — le nom part en `debug`.
            tracing::info!(">>> Répondre en privé à l'auteur de l'alerte de chat.");
            tracing::debug!(author, ">>> Répondre en privé");
            chat_command::send_whisper(author, target);
        }

        /// `ShortcutAction::InvitePartner` / `ShortcutAction::FollowPartner` : tape `/i "<nom>"` ou
        /// `/fol "<nom>"` dans le chat de la fenêtre de jeu active, en visant le personnage de
        /// l'AUTRE fenêtre — même code que Windows (`chat_command`, dont l'`imp` Linux passe par
        /// XTEST, voir `overlay_platform::linux::keyboard`), seule la façon de nommer la fenêtre
        /// active change (`_NET_ACTIVE_WINDOW` plutôt que `GetForegroundWindow`).
        ///
        /// **Le scan d'ici est dans l'ordre de `_NET_CLIENT_LIST`, c'est-à-dire l'ordre de
        /// CRÉATION** des fenêtres, là où `EnumWindows` (Windows) donne l'ordre de profondeur :
        /// au-delà de deux clients, le partenaire désigné n'est donc pas forcément le même sur les
        /// deux OS. Stable et prévisible dans les deux cas (voir `chat_command`, doc de module),
        /// et sans objet pour le cas visé par la demande — deux clients, un seul autre personnage.
        fn send_partner_command(&mut self, command: ChatCommand) {
            let label = self.hotkeys.bindings().label(match command {
                ChatCommand::Invite => ShortcutAction::InvitePartner,
                ChatCommand::Follow => ShortcutAction::FollowPartner,
            });
            // `None` (aucune fenêtre active connue du WM) ne peut désigner aucune fenêtre de jeu :
            // `partner_character` répondra `NoGameFocused`, exactement comme il se doit.
            let active = self.game_window.active_window().unwrap_or(0);
            let windows: Vec<(String, u32)> = self
                .game_window
                .scan()
                .into_iter()
                .map(|(character, info)| (character, info.window))
                .collect();
            match chat_command::partner_character(&windows, &active) {
                Ok(partner) => {
                    tracing::info!(">>> {} ({label}) : {partner}", command.label());
                    chat_command::send(command, partner);
                }
                Err(err) => tracing::info!(">>> {} ({label}) : {}", command.label(), err.message()),
            }
        }

        /// Même politique focus-aware que Windows (`main.rs::App::sync_topmost`), portée sur
        /// `overlay_platform::linux::topmost::decide` (délai de grâce déjà testé unitairement,
        /// jamais réécrit ici) plutôt que `SetWindowPos`/`GetForegroundWindow` — `_NET_ACTIVE_
        /// WINDOW` (`GameWindowTracker::active_window`) est l'équivalent EWMH de
        /// `GetForegroundWindow`.
        fn sync_topmost(&mut self) {
            let active = self.game_window.active_window();
            let now = std::time::Instant::now();

            // Même correctif que `main.rs::App::sync_topmost` (2026-09-06/07) — voir sa doc :
            // le focus sur N'IMPORTE LEQUEL des overlays d'un personnage (ou le jeu lui-même)
            // rend TOUS les overlays de CE personnage relevant, pas seulement celui cliqué.
            let mut relevant_game_windows: Vec<u32> = Vec::new();
            for overlay in self.windows.values() {
                if overlay.kind == OverlayKind::Login || overlay.is_detached() {
                    continue;
                }
                let this_relevant = active == Some(overlay.game_window)
                    || active == Some(Self::xid_of(&overlay.window));
                if this_relevant && !relevant_game_windows.contains(&overlay.game_window) {
                    relevant_game_windows.push(overlay.game_window);
                }
            }

            for overlay in self.windows.values_mut() {
                // La fenêtre de connexion est une fenêtre ordinaire, jamais promue (2026-09-14).
                if overlay.kind == OverlayKind::Login {
                    continue;
                }
                // **La modale Options participe à ce calcul comme les autres depuis le
                // 2026-09-12** — voir `main.rs::App::sync_topmost` pour le détail : rattachée à
                // une vraie fenêtre de jeu, elle n'a plus besoin d'exception. **Sauf ouverte sans
                // aucun client à l'écran** (`is_detached`, 2026-09-17) : aucun premier plan à
                // suivre, elle reste toujours pertinente — jamais repassée `Normal`.
                let relevant =
                    overlay.is_detached() || relevant_game_windows.contains(&overlay.game_window);
                let (next_state, action) = topmost::decide(overlay.topmost_state, relevant, now);
                overlay.topmost_state = next_state;
                // Même diagnostic que `main.rs::App::sync_topmost` (2026-09-06) : journalise les
                // vraies transitions topmost, pas le sondage — voir sa doc.
                match action {
                    TopmostAction::None => {}
                    TopmostAction::SetAbove => {
                        tracing::info!(
                            "[topmost] {} ({:?}) -> AlwaysOnTop",
                            overlay.character_name,
                            overlay.kind
                        );
                        overlay.window.set_window_level(WindowLevel::AlwaysOnTop);
                        // Même correctif que `main.rs::App::sync_topmost` (2026-09-06) — voir sa
                        // doc : un redessin peut avoir échoué silencieusement (occlusion) pendant
                        // que cette fenêtre était `Normal`, rien ne le rattrapait à la repromotion.
                        overlay.window.request_redraw();
                    }
                    TopmostAction::SetNormal => {
                        tracing::info!(
                            "[topmost] {} ({:?}) -> Normal",
                            overlay.character_name,
                            overlay.kind
                        );
                        overlay.window.set_window_level(WindowLevel::Normal)
                    }
                }
            }
        }

        /// Ouvre la modale Options (2026-09-08, §9 du plan), **rattachée à une fenêtre de jeu**.
        ///
        /// `anchor` est la fenêtre depuis laquelle elle est demandée : le bouton « Options » du
        /// carré de contrôle la donne directement. Le raccourci global n'en a pas — il prend alors
        /// la fenêtre de jeu ACTIVE (`_NET_ACTIVE_WINDOW`), et à défaut le premier overlay connu.
        /// **Aucune fenêtre de jeu du tout** (2026-09-17) : la modale s'ouvre quand même,
        /// détachée — voir `main.rs::App::open_options_modal`.
        ///
        /// Sans effet si une modale est déjà ouverte (une seule à la fois, comme un vrai dialogue
        /// modal) — pas de file d'attente, l'utilisateur referme/valide l'existante avant d'en
        /// rouvrir une.
        ///
        /// `initial_tab` est l'onglet sur lequel la fenêtre s'ouvre — **`Paramètres` pour le
        /// bouton « Options » du bandeau de suivi** (demande utilisateur 2026-09-13 : ce bouton
        /// donne accès au chemin de `wakfu.log`, pas à la composition de la liste suivie, qui a
        /// son propre accès dans ce même bandeau), le défaut d'[`options_modal::OptionsTab`] pour
        /// le raccourci global, qui n'a pas de bandeau particulier à privilégier.
        fn open_options_modal(
            &mut self,
            event_loop: &ActiveEventLoop,
            anchor: Option<(u32, GameRect)>,
            initial_tab: options_modal::OptionsTab,
        ) {
            if self
                .windows
                .values()
                .any(|w| w.kind == OverlayKind::Options)
            {
                return;
            }
            // Compte non lié : la fenêtre de connexion est la seule interface (voir `main.rs`).
            if !self.session_ready() {
                tracing::info!(
                    "[options] aucun compte lié — la fenêtre Options n'est accessible qu'une fois connecté."
                );
                return;
            }
            // Sans ancre explicite (raccourci global), la fenêtre de jeu ACTIVE est la bonne
            // réponse : c'est celle que l'utilisateur regarde au moment où il appuie. Repli sur le
            // premier overlay connu si l'active n'est pas un client Wakfu.
            let anchor = anchor.or_else(|| {
                let active = self.game_window.active_window();
                self.windows
                    .values()
                    .filter(|w| !w.is_detached())
                    .find(|w| Some(w.game_window) == active)
                    .or_else(|| self.windows.values().find(|w| !w.is_detached()))
                    .map(|w| (w.game_window, w.game_rect))
            });
            // **Aucune fenêtre de jeu à l'écran ⇒ la modale s'ouvre quand même, seule**
            // (2026-09-17) — voir `main.rs::App::open_options_modal` : détachée
            // (`OverlayWindow::is_detached`), centrée sur l'écran principal, jamais refermée par
            // `sync_windows`, ni rattachée à un client lancé entre-temps.
            let detached = anchor.is_none();
            let (game_window, rect) = anchor.unwrap_or((
                0,
                GameRect {
                    left: 0,
                    top: 0,
                    width: 0,
                    height: 0,
                    client_top: 0,
                },
            ));
            // Toujours interactive (jamais clic-traversant), quel que soit `self.interactive` —
            // c'est une modale qui doit capter le clavier/la souris pour éditer le chemin, pas un
            // overlay passif d'information comme Combat/Suivi.
            let mut overlay = Self::create_overlay_window(
                event_loop,
                OverlayKind::Options,
                game_window,
                "Options".to_string(),
                rect,
                true,
                // Une fenêtre de réglages qu'on vient d'ouvrir est visible, toujours : seul
                // `Combat` peut naître masqué (voir `sync_panel_visibility`).
                true,
                self.combat_on_right,
                // Sans objet : cette fenêtre-ci n'est ni le panneau Combat ni la bande Récap.
                None,
                None,
            );
            if detached {
                // Pas de jeu sur lequel s'ancrer : au centre de l'écran principal, et le focus
                // tout de suite — voir `main.rs::App::open_options_modal`.
                Self::center_on_primary_monitor(event_loop, &overlay.window);
                overlay.last_position = None;
                tracing::info!(
                    "[options] aucune fenêtre de jeu à l'écran — fenêtre Options ouverte seule, centrée sur l'écran principal."
                );
            }
            // **Le focus clavier, rattachée ou non** (2026-09-17) — même correctif que
            // `main.rs::App::open_options_modal` (voir sa doc) : sans lui, Échap et Entrée
            // n'atteignaient pas la modale ouverte depuis un bandeau.
            overlay.window.focus_window();
            // **Brouillons pris sur le compte** — même logique que `main.rs::open_options_modal`
            // (voir sa doc) : alertes, chat et suivi sont des COPIES de l'état du compte, renvoyées
            // seulement à « Valider » ; et le compte est relu à l'ouverture, sur un thread.
            let alerts_snapshot = self.alert_profile.load();
            let (alerts_draft, alerts_availability) = match alerts_snapshot.as_ref() {
                Some((profile, _)) => {
                    (Some(profile.clone()), alerts_tab::AlertsAvailability::Ready)
                }
                None if self.auth_status.load().is_connected() => {
                    (None, alerts_tab::AlertsAvailability::Loading)
                }
                None => (None, alerts_tab::AlertsAvailability::NoAccount),
            };
            let chat_snapshot = self.chat_filters.load();
            let (chat_draft, chat_availability) = match chat_snapshot.as_ref() {
                Some(filters) => (
                    Some(chat_tab::ChatDraft {
                        filters: filters.clone(),
                        toast: self.chat_toast,
                    }),
                    chat_tab::ChatAvailability::Ready,
                ),
                None if self.auth_status.load().is_connected() => {
                    (None, chat_tab::ChatAvailability::Loading)
                }
                None => (None, chat_tab::ChatAvailability::NoAccount),
            };
            let suivi_snapshot = self.watchlist.load();
            let (suivi_draft, suivi_availability) =
                if suivi_snapshot.is_empty() && !self.auth_status.load().is_connected() {
                    (None, suivi_tab::SuiviAvailability::Loading)
                } else {
                    (
                        Some(suivi_snapshot.as_ref().clone()),
                        suivi_tab::SuiviAvailability::Ready,
                    )
                };
            let settings_tx = self.settings_tx.clone();
            thread::spawn(move || {
                let Some(token) = overlay_sync::token_store::load_token() else {
                    return;
                };
                match overlay_sync::client::fetch_settings(&token) {
                    Ok(settings) => {
                        tracing::info!(
                            entry_count = settings.watchlist.len(),
                            "[options] réglages relus à l'ouverture de la fenêtre"
                        );
                        let _ = settings_tx.send(EngineCommand::ApplySettings(settings));
                    }
                    Err(err) => {
                        tracing::warn!(%err, "[options] relecture des réglages impossible")
                    }
                }
            });

            // **Le brouillon du roster**, même principe que les alertes et le chat : une copie
            // de ce que le compte porte, prise ici, modifiée librement, renvoyée à « Valider ».
            let roster_snapshot = self.roster_draft.load();
            let (personnages_draft, personnages_availability) = match roster_snapshot.as_ref() {
                Some(roster) => (Some(roster.clone()), PersonnagesAvailability::Ready),
                None => (None, PersonnagesAvailability::Loading),
            };
            // Lu une seule fois par ouverture, jamais à chaque frame : c'est un accès au registre
            // (Windows) ou au disque (Linux), et la case est un brouillon comme les autres — elle ne
            // doit surtout pas se remettre d'aplomb toute seule pendant qu'on la regarde.
            let autostart_actif = overlay_ui::autostart::is_enabled();

            overlay.options_state = Some(OptionsModalState {
                path_input: self.log_path.display().to_string(),
                error: None,
                // Voir la doc de `open_options_modal` : le bouton « Options » du bandeau de
                // suivi demande `Parametres`, le raccourci global le défaut d'`OptionsTab`.
                tab: initial_tab,
                combat_always_visible: self.combat_always_visible,
                combat_on_right: self.combat_on_right,
                turn_notification: self.turn_notification,
                turn_notification_muted: self.turn_notification_muted,
                // Les cases « Activer … » s'ouvrent sur l'état réel — voir `main.rs`. Les deux
                // cases « Couper le son des notifications » aussi.
                features: self.features,
                mutes: self.alert_mutes,
                // La fermeture de la carte de décompte (2026-09-16) — réglage local, la ligne
                // s'ouvre directement sur sa valeur, voir `main.rs`.
                countdown_toast: self.countdown_toast,
                completion: self.completion,
                // La reprise de la session du Récap (2026-09-17), même principe.
                recap_resume: self.recap_session.resume_settings(),
                shortcuts: self.hotkeys.bindings().clone(),
                raccourcis: Default::default(),
                account_connected: self.auth_status.load().is_connected(),
                pending_disconnect: false,
                personnages: PersonnagesTabState {
                    // Le compte affiché à l'ouverture est le principal, celui que tout roster a.
                    account: personnages_draft
                        .as_ref()
                        .map(|roster| roster.default_index())
                        .unwrap_or(0),
                    ..Default::default()
                },
                // La section « Mise à jour » lit l'état publié par le thread de mise à jour ; rafraîchi
                // avant chaque rendu (voir `redraw`), posé ici pour la première frame.
                update: (**self.update_status.load()).clone(),
                auto_update: self.auto_update,
                verbose_log: self.verbose_log,
                // **Le démarrage avec l'ordinateur se lit dans le SYSTÈME**, pas dans un champ
                // de l'hôte : c'est le seul réglage de cette fenêtre qu'un autre programme peut
                // avoir changé entre deux ouvertures (réglages du bureau, fichier supprimé à la
                // main). Voir `autostart`, doc de module.
                start_with_os: autostart_actif,
                pending_install: None,
                pending_quit: false,
                pending_restart: false,
                alerts: alerts_tab::AlertsTabState {
                    duration_input: alerts_draft
                        .as_ref()
                        .map(|p| format_alert_duration(p.duration_seconds))
                        .unwrap_or_default(),
                    ..Default::default()
                },
                chat: chat_tab::ChatTabState {
                    duration_input: format_alert_duration(self.chat_toast.duration_seconds),
                    ..Default::default()
                },
                initial: options_modal::OptionsInitial {
                    path: self.log_path.display().to_string(),
                    alerts: alerts_draft.clone(),
                    suivi: suivi_draft.clone(),
                    chat: chat_draft.clone(),
                    personnages: personnages_draft.clone(),
                    combat_always_visible: self.combat_always_visible,
                    combat_on_right: self.combat_on_right,
                    turn_notification: self.turn_notification,
                    turn_notification_muted: self.turn_notification_muted,
                    features: self.features,
                    mutes: self.alert_mutes,
                    countdown_toast: self.countdown_toast,
                    completion: self.completion,
                    recap_resume: self.recap_session.resume_settings(),
                    shortcuts: self.hotkeys.bindings().clone(),
                    auto_update: self.auto_update,
                    verbose_log: self.verbose_log,
                    start_with_os: autostart_actif,
                },
                pending_close: false,
                alerts_draft,
                alerts_availability,
                chat_draft,
                chat_availability,
                personnages_draft,
                personnages_availability,
                suivi: suivi_tab::SuiviTabState {
                    duration_input: format_alert_duration(self.countdown_toast.duration_seconds),
                    ..Default::default()
                },
                suivi_draft,
                suivi_availability,
            });
            overlay.window.request_redraw();
            self.windows.insert(overlay.window.id(), overlay);
            // Voir `main.rs::open_options_modal` : XGrabKey intercepterait sinon la frappe que
            // l'onglet « Raccourcis » essaie justement de capturer.
            self.hotkeys.suspend();
            tracing::info!("[options] modale ouverte — raccourcis globaux suspendus.");
        }

        /// Voir `main.rs::commit_chat`.
        fn commit_chat(&mut self, options_window_id: WindowId) -> bool {
            let Some(draft) = self
                .windows
                .get(&options_window_id)
                .and_then(|overlay| overlay.options_state.as_ref())
                .and_then(|state| state.chat_draft.clone())
            else {
                return false;
            };
            let toast_changed = draft.toast != self.chat_toast;
            if toast_changed {
                self.chat_toast = draft.toast;
                let _ = self
                    .settings_tx
                    .send(EngineCommand::SetChatToast(draft.toast));
            }
            let reference = self.chat_filters.load();
            if reference.as_ref().as_ref() == Some(&draft.filters) {
                return toast_changed;
            }
            let _ = self
                .settings_tx
                .send(EngineCommand::SetChatFilters(draft.filters.clone()));
            let filters = draft.filters;
            thread::spawn(move || {
                match overlay_sync::token_store::load_token() {
                Some(token) => match overlay_sync::client::patch_chat_filters(&token, &filters) {
                    Ok(_) => {
                        tracing::info!("[options] recherches de chat enregistrées sur le compte.")
                    }
                    Err(err) => tracing::warn!(
                        %err,
                        "[options] échec de l'enregistrement des recherches de chat"
                    ),
                },
                None => tracing::info!(
                    "[options] recherches de chat appliquées localement — aucun compte lié, rien n'est enregistré."
                ),
            }
            });
            toast_changed
        }

        /// Voir `main.rs::commit_alerts`.
        /// Voir `main.rs::commit_personnages`.
        fn commit_personnages(&mut self, options_window_id: WindowId) {
            let Some(roster) = self
                .windows
                .get(&options_window_id)
                .and_then(|overlay| overlay.options_state.as_ref())
                .and_then(|state| state.personnages_draft.clone())
            else {
                return;
            };
            let reference = self.roster_draft.load();
            if reference.as_ref().as_ref() == Some(&roster) {
                return;
            }
            // Appliqué localement d'abord : le combat en cours doit reconnaître le personnage
            // déclaré sans attendre le réseau.
            let _ = self
                .settings_tx
                .send(EngineCommand::SetRoster(roster.clone()));
            thread::spawn(move || {
                match overlay_sync::token_store::load_token() {
                    Some(token) => match overlay_sync::client::patch_roster(&token, &roster) {
                        Ok(_) => {
                            tracing::info!("[options] roster enregistré sur le compte.")
                        }
                        Err(err) => {
                            tracing::warn!(%err, "[options] échec de l'enregistrement du roster")
                        }
                    },
                    None => tracing::info!(
                        "[options] roster appliqué localement — aucun compte lié, rien n'est enregistré."
                    ),
                }
            });
        }

        fn commit_alerts(&mut self, options_window_id: WindowId) {
            let Some(draft) = self
                .windows
                .get(&options_window_id)
                .and_then(|overlay| overlay.options_state.as_ref())
                .and_then(|state| state.alerts_draft.clone())
            else {
                return;
            };
            let connu = self.alert_profile.load();
            let (reference, raw) = match connu.as_ref() {
                Some((profile, raw)) => (Some(profile.clone()), raw.clone()),
                None => (None, None),
            };
            if reference.as_ref() == Some(&draft) {
                return;
            }
            let _ = self
                .settings_tx
                .send(EngineCommand::SetAlertProfile(draft.clone()));
            let profile_value = draft.patch_value(raw.as_ref());
            thread::spawn(move || {
                match overlay_sync::token_store::load_token() {
                Some(token) => match overlay_sync::client::patch_profile(&token, &profile_value) {
                    Ok(_) => tracing::info!("[options] alertes enregistrées sur le compte."),
                    Err(err) => {
                        tracing::warn!(%err, "[options] échec de l'enregistrement des alertes")
                    }
                },
                None => tracing::info!(
                    "[options] alertes appliquées localement — aucun compte lié, rien n'est enregistré."
                ),
            }
            });
        }

        /// Voir `main.rs::commit_suivi`.
        fn commit_suivi(&mut self, options_window_id: WindowId) {
            let Some(state) = self
                .windows
                .get(&options_window_id)
                .and_then(|overlay| overlay.options_state.as_ref())
            else {
                return;
            };
            let Some(draft) = state.suivi_draft.clone() else {
                return;
            };
            let retirees = state.suivi.retirees.clone();
            if state.initial.suivi.as_ref() == Some(&draft) && retirees.is_empty() {
                return;
            }
            tracing::info!(
                entry_count = draft.len(),
                removed_count = retirees.len(),
                "[options] liste de suivi validée"
            );
            let _ = self
                .settings_tx
                .send(EngineCommand::SetWatchlistDefinitions {
                    definitions: draft,
                    retirees,
                });
        }

        /// Voir `main.rs::start_recipe_resolution` — sur un thread, jamais sur la boucle winit.
        fn start_recipe_resolution(&mut self, options_window_id: WindowId, item_id: i64) {
            let (tx, rx) = mpsc::channel();
            self.pending_recipe = Some((options_window_id, rx));
            let catalog = Arc::clone(&self.catalog);
            thread::Builder::new()
                .name("overlay-ui-recipe".into())
                .spawn(move || {
                    let index = catalog.load();
                    let mut fetch = |id: i64| overlay_sync::client::fetch_item_detail(id).ok();
                    let ingredients = overlay_engine::resolve_recipe(item_id, &index, &mut fetch);
                    tracing::info!(
                        item_id,
                        ingredient_count = ingredients.len(),
                        "[options] recette résolue"
                    );
                    let _ = tx.send(ingredients);
                })
                .ok();
        }

        /// Voir `main.rs::open_reset_confirm` — la confirmation de remise à zéro du Récap,
        /// par-dessus la fenêtre de jeu d'où le glyphe a été cliqué, une seule à la fois.
        fn open_reset_confirm(
            &mut self,
            event_loop: &ActiveEventLoop,
            game_window: u32,
            rect: GameRect,
            target: ResetTarget,
        ) {
            if self
                .windows
                .values()
                .any(|w| matches!(w.kind, OverlayKind::ResetConfirm(_)))
            {
                return;
            }
            let overlay = Self::create_overlay_window(
                event_loop,
                OverlayKind::ResetConfirm(target),
                game_window,
                "Recap".to_string(),
                rect,
                true,
                true,
                self.combat_on_right,
                // La confirmation couvre la fenêtre de jeu entière — elle ne suit ni le panneau
                // Combat ni la bande Récap.
                None,
                None,
            );
            overlay.window.request_redraw();
            self.windows.insert(overlay.window.id(), overlay);
            match target {
                ResetTarget::RecapSession => {
                    tracing::info!("[session] confirmation de remise à zéro ouverte.")
                }
                ResetTarget::RecapPosition => {
                    tracing::info!("[recap] confirmation de replacement ouverte.")
                }
                ResetTarget::CombatPosition => {
                    tracing::info!("[combat] confirmation de replacement ouverte.")
                }
                ResetTarget::WatchlistCounter => {
                    tracing::info!(
                        name = self
                            .watchlist_reset_pending
                            .as_ref()
                            .map(|e| e.name.as_str()),
                        "[suivi] confirmation de réinitialisation du compteur ouverte."
                    )
                }
            }
        }

        /// Voir `main.rs::open_watchlist_reset_confirm`.
        fn open_watchlist_reset_confirm(
            &mut self,
            event_loop: &ActiveEventLoop,
            game_window: u32,
            rect: GameRect,
            entry: WatchlistEntry,
        ) {
            self.watchlist_reset_pending = Some(entry);
            self.open_reset_confirm(event_loop, game_window, rect, ResetTarget::WatchlistCounter);
        }

        /// Voir `main.rs::toggle_combat_lock`.
        fn toggle_combat_lock(&mut self) {
            self.combat_locked = !self.combat_locked;
            self.persist_config();
            tracing::info!(
                "[combat] panneau {}.",
                if self.combat_locked {
                    "verrouillé"
                } else {
                    "déverrouillé"
                }
            );
        }

        /// Voir `main.rs::toggle_recap_lock`.
        fn toggle_recap_lock(&mut self) {
            self.recap_locked = !self.recap_locked;
            self.persist_config();
            tracing::info!(
                "[recap] bande {}.",
                if self.recap_locked {
                    "verrouillée"
                } else {
                    "déverrouillée"
                }
            );
        }

        /// Voir `main.rs::answer_reset_confirm`.
        fn answer_reset_confirm(
            &mut self,
            confirm_window_id: WindowId,
            target: ResetTarget,
            confirmed: bool,
        ) {
            self.windows.remove(&confirm_window_id);
            match (target, confirmed) {
                (ResetTarget::RecapSession, true) => {
                    let snapshot = self.snapshot.load();
                    self.recap_session
                        .reset(&snapshot.totals, std::time::SystemTime::now());
                }
                (ResetTarget::RecapSession, false) => {
                    tracing::info!("[session] remise à zéro annulée.")
                }
                (ResetTarget::RecapPosition, true) => {
                    self.recap_position = None;
                    self.persist_config();
                    self.reposition_recap();
                    tracing::info!("[recap] bande revenue à son emplacement d'origine.");
                }
                (ResetTarget::RecapPosition, false) => {
                    tracing::info!("[recap] replacement annulé.")
                }
                (ResetTarget::CombatPosition, true) => {
                    self.combat_position_y = None;
                    self.persist_config();
                    self.reposition_combat();
                    tracing::info!("[combat] panneau revenu à sa hauteur d'origine.");
                }
                (ResetTarget::CombatPosition, false) => {
                    tracing::info!("[combat] replacement annulé.")
                }
                (ResetTarget::WatchlistCounter, true) => {
                    match self.watchlist_reset_pending.take() {
                        Some(entry) => {
                            tracing::info!(
                                name = %entry.name,
                                "[suivi] réinitialisation du compteur confirmée."
                            );
                            let _ = self.settings_tx.send(EngineCommand::ResetWatchlistCounter {
                                name: entry.name,
                                kind: entry.kind,
                            });
                        }
                        None => tracing::warn!(
                            "[suivi] réinitialisation confirmée sans entrée retenue, rien fait."
                        ),
                    }
                }
                (ResetTarget::WatchlistCounter, false) => {
                    self.watchlist_reset_pending = None;
                    tracing::info!("[suivi] réinitialisation du compteur annulée.")
                }
            }
        }

        /// Voir `main.rs::close_options_modal` — unique point de fermeture, rend les raccourcis
        /// globaux au système.
        fn close_options_modal(&mut self, options_window_id: WindowId, raison: &str) {
            self.windows.remove(&options_window_id);
            let refuses = self.hotkeys.resume();
            if !refuses.is_empty() {
                tracing::warn!(
                    "[options] {} raccourci(s) refusé(s) par le système — voir les lignes [raccourcis] ci-dessus.",
                    refuses.len()
                );
            }
            tracing::info!("[options] fenêtre fermée ({raison}).");
        }

        /// **Les suivis qui viennent d'aboutir** — reçus du thread Engine, célébrés, puis retirés
        /// (2026-09-17).
        ///
        /// Appelée à chaque tick de la boucle d'événements, **pas au rendu**, et c'est tout l'intérêt :
        /// décision utilisateur, « le retrait est une conséquence du seuil, pas de l'animation ». Une
        /// fenêtre Suivi masquée, un Suivi coupé, un joueur qui regardait ailleurs : l'entrée aboutie
        /// part quand même, et part au compte.
        ///
        /// Le retrait emprunte le chemin de la suppression groupée du bandeau, sans rien y ajouter :
        /// `SetWatchlistDefinitions` avec la liste amputée → `WatchlistState::apply_definitions` →
        /// `drain_watchlist_sync` → `SyncCommand::SyncWatchlist` → `PATCH /api/v1/settings`. Le moteur
        /// garde les compteurs des entrées restantes, et un échec réseau est retenté par le thread
        /// Sync — le retrait ne se perd pas dans une coupure.
        fn tick_watchlist_completions(&mut self) {
            let now = std::time::Instant::now();
            while let Ok(completed) = self.completions_rx.try_recv() {
                tracing::info!(
                    name = %completed.name,
                    remove = self.completion.remove,
                    animate = self.completion.animates(),
                    "[suivi] entrée complétée"
                );
                self.watchlist_completions.push(
                    completed.key,
                    now,
                    self.completion.removal_delay_seconds(),
                    self.completion.remove,
                );
            }

            // **Redessiner le bandeau tant qu'une tuile célèbre.** La boucle est réactive (§6.1) :
            // sans cette demande, l'animation n'avancerait qu'au tick de 50 ms et au gré des lots du
            // moteur — soit des à-coups visibles sur une couronne qui tourne. Le rythme est rendu à la
            // bande dès la dernière image : trois secondes et demie de 60 Hz, pas une de plus.
            if self.watchlist_completions.is_animating(now) {
                let prochaine = now + COMPLETION_FRAME;
                for overlay in self.windows.values_mut() {
                    if matches!(overlay.kind, OverlayKind::Watchlist) {
                        overlay.next_redraw_at = Some(match overlay.next_redraw_at {
                            Some(deja) => deja.min(prochaine),
                            None => prochaine,
                        });
                    }
                }
            }

            let a_retirer = self.watchlist_completions.drain_due(now);
            if a_retirer.is_empty() {
                return;
            }
            // **La liste de référence est celle du moteur**, relue à l'instant du retrait et non à
            // celui du franchissement : entre les deux, le joueur a pu ajouter une entrée depuis la
            // fenêtre Options ou le site. Repartir d'une copie prise 3,5 s plus tôt la ferait
            // disparaître.
            let definitions: Vec<WatchlistEntry> = self
                .watchlist
                .load()
                .iter()
                .filter(|entry| {
                    !a_retirer.contains(&panels::suivi_tab::key_of(&entry.name, entry.catalog_id))
                })
                .cloned()
                .collect();
            // Rien à écrire si aucune des clés ne correspond plus à une entrée vivante : elles ont pu
            // être retirées entre-temps depuis la fenêtre Options ou le site. Réécrire la clé pour
            // rien repousserait son horodatage, et le « dernier écrivain gagne » du serveur ferait
            // perdre une modification faite ailleurs (même précaution que `commit_suivi`).
            if definitions.len() == self.watchlist.load().len() {
                return;
            }
            tracing::info!(
                retirees = a_retirer.len(),
                restantes = definitions.len(),
                "[suivi] entrées complétées retirées, réplication au compte en route"
            );
            let _ = self
                .settings_tx
                .send(EngineCommand::SetWatchlistDefinitions {
                    definitions,
                    // Rien à oublier en plus : l'entrée complétée quitte la liste sans y revenir,
                    // exactement comme un retrait depuis le bandeau. Le jour où elle est recréée,
                    // c'est une entrée neuve, qui repart de la valeur de son mode.
                    retirees: Vec::new(),
                });
        }

        /// Lance l'explorateur de fichiers natif (`rfd`) sur un thread dédié — bloquant côté OS,
        /// ne doit JAMAIS geler la boucle winit (même raison que tous les threads réseau de ce
        /// dépôt, voir §7.3 du plan pour la justification appliquée à `overlay-sync`). Le résultat
        /// (chemin choisi, ou `None` si l'utilisateur a annulé le dialogue) revient par
        /// `self.pending_dialog`, sondé à chaque `about_to_wait`.
        fn start_file_dialog(&mut self) {
            let (tx, rx) = mpsc::channel();
            self.pending_dialog = Some(rx);
            thread::Builder::new()
                .name("overlay-ui-file-dialog".into())
                .spawn(move || {
                    // Filtre par EXTENSION uniquement (`rfd` ne sait pas filtrer par nom de fichier
                    // exact) — le garde-fou du NOM exact (`wakfu.log`) est appliqué après coup par
                    // `App::validate_and_commit_options`/`discovery::validate_log_path`, jamais
                    // sauté même si l'utilisateur choisit un `.log` mal nommé dans le dialogue.
                    let picked = rfd::FileDialog::new()
                        .set_title("Sélectionner le fichier wakfu.log")
                        .add_filter("wakfu.log", &["log"])
                        .pick_file();
                    let _ = tx.send(picked);
                })
                .expect("échec de création du thread de dialogue de fichier");
        }

        /// Valide `raw` (contenu du champ texte au moment du clic sur "Valider", ou chemin choisi
        /// par le dialogue natif) via `discovery::validate_log_path` — voir §5.1 du plan : « il ne
        /// peut sélectionner qu'un fichier wakfu.log [...] des guards pour éviter de sélectionner
        /// n'importe quoi ». Sur succès : persiste (`config::save`), recharge l'Engine À CHAUD
        /// (`EngineCommand::ChangeLogPath`, jamais de redémarrage du binaire) et ferme la modale —
        /// « lorsqu'il valide [...] c'est ce nouveau fichier qui est lu de manière continue ». Sur
        /// échec : la modale RESTE ouverte, le message d'erreur est écrit dans son état pour le
        /// prochain redessin (voir `OptionsModalState::error`) — rien n'est pris en compte tant que
        /// la validation n'a pas réussi.
        /// Recolle chaque bande Récap sur sa fenêtre de jeu — voir
        /// `main.rs::App::reposition_recap`, même rôle : le replacement de la bande doit se
        /// voir dans la même passe que le geste qui l'a demandé.
        fn reposition_recap(&mut self) {
            self.reposition_kind(OverlayKind::Recap);
        }

        /// Recolle chaque panneau Combat sur sa fenêtre de jeu — voir
        /// `main.rs::App::reposition_combat`.
        fn reposition_combat(&mut self) {
            self.reposition_kind(OverlayKind::Combat);
        }

        /// Recolle toutes les fenêtres d'une zone, fenêtres de jeu inchangées.
        fn reposition_kind(&mut self, kind: OverlayKind) {
            let recap_position = self.recap_position;
            let combat_on_right = self.combat_on_right;
            let combat_position_y = self.combat_position_y;
            for overlay in self.windows.values_mut() {
                if overlay.kind == kind {
                    let rect = overlay.game_rect;
                    Self::reposition(
                        overlay,
                        rect,
                        combat_on_right,
                        combat_position_y,
                        recap_position,
                    );
                }
            }
        }

        /// Écrit `config.toml` en entier depuis l'état courant — voir
        /// `main.rs::App::persist_config`, même contenu et même raison d'être le seul endroit qui
        /// sache ce que la config doit porter.
        fn persist_config(&self) {
            let mut saved = config::OverlayConfig {
                log_path: Some(self.log_path.clone()),
                combat_always_visible: self.combat_always_visible,
                combat_on_right: self.combat_on_right,
                turn_notification: self.turn_notification,
                turn_notification_muted: self.turn_notification_muted,
                auto_update: self.auto_update,
                verbose_log: self.verbose_log,
                // Jalon posé au démarrage — voir `main.rs`.
                autostart_initialized: true,
                ..Default::default()
            };
            saved.set_shortcuts(self.hotkeys.bindings());
            saved.set_chat_toast(self.chat_toast);
            saved.set_countdown_toast(self.countdown_toast);
            saved.set_completion(self.completion);
            saved.set_recap_resume(self.recap_session.resume_settings());
            saved.set_recap_position(self.recap_position);
            saved.recap_locked = self.recap_locked;
            saved.combat_position_y = self.combat_position_y;
            saved.combat_locked = self.combat_locked;
            saved.set_features(self.features);
            saved.set_alert_mutes(self.alert_mutes);
            config::save(&saved);
        }

        fn validate_and_commit_options(
            &mut self,
            options_window_id: WindowId,
            commit: options_modal::OptionsCommit,
        ) {
            let candidate = PathBuf::from(commit.path.trim());
            match discovery::validate_log_path(&candidate) {
                Ok(()) => {
                    // Rechargé seulement si le chemin a CHANGÉ — même garde que `main.rs` : un
                    // `ChangeLogPath` à chemin identique rejouait tout le fichier dans une session
                    // qui gardait son état, et dupliquait le combat en cours (2026-09-12).
                    let path_changed = candidate != self.log_path;
                    if path_changed {
                        tracing::info!(
                            "[options] nouveau fichier de log validé : {}",
                            candidate.display()
                        );
                        self.log_path = candidate.clone();
                        let _ = self
                            .settings_tx
                            .send(EngineCommand::ChangeLogPath(candidate.clone()));
                    } else {
                        tracing::info!("[options] chemin de log inchangé, moteur non touché.");
                    }
                    let combat_changed = commit.combat_always_visible != self.combat_always_visible;
                    if combat_changed {
                        self.combat_always_visible = commit.combat_always_visible;
                        tracing::info!(
                            "[options] panneau de combat en dehors des combats : {}",
                            if self.combat_always_visible {
                                "affiché"
                            } else {
                                "masqué"
                            }
                        );
                    }
                    // Le côté du panneau (2026-09-17) — voir `main.rs`, même mécanique : le
                    // balayage des fenêtres de jeu recolle l'overlay à son nouvel ancrage, et le
                    // rendu lit le nouveau côté à la frame suivante.
                    let side_changed = commit.combat_on_right != self.combat_on_right;
                    if side_changed {
                        self.combat_on_right = commit.combat_on_right;
                        tracing::info!(
                            "[options] panneau de combat : {}",
                            if self.combat_on_right {
                                "à droite"
                            } else {
                                "à gauche"
                            }
                        );
                    }
                    let turn_changed = commit.turn_notification != self.turn_notification;
                    if turn_changed {
                        self.turn_notification = commit.turn_notification;
                        tracing::info!(
                            "[options] notification de tour : {}",
                            if self.turn_notification {
                                "activée"
                            } else {
                                "désactivée"
                            }
                        );
                        // La lecture du widget passe par une capture de fenêtre sans focus, tranchée
                        // sous Windows (spike S4) et pas encore sous X11 (`XCompositeNameWindowPixmap`,
                        // §14 point 7 du plan) : le réglage est persisté, il ne fait rien ici.
                        if self.turn_notification {
                            tracing::warn!(
                                "[tour] la notification de tour n'est pas encore disponible sous                                  Linux — réglage enregistré, sans effet pour l'instant."
                            );
                        }
                    }
                    let turn_muted_changed =
                        commit.turn_notification_muted != self.turn_notification_muted;
                    if turn_muted_changed {
                        self.turn_notification_muted = commit.turn_notification_muted;
                    }
                    // Les trois interrupteurs (2026-09-15) — voir `main.rs` : le thread Engine
                    // est prévenu dès qu'ils bougent, le bandeau lit `self.features` au rendu.
                    let features_changed = commit.features != self.features;
                    if features_changed {
                        self.features = commit.features;
                        tracing::info!(
                            suivi = self.features.suivi,
                            alertes = self.features.alerts,
                            recherche = self.features.chat,
                            combat = self.features.combat,
                            sorts = self.features.spells,
                            recap = self.features.recap,
                            recap_duree = self.features.recap_cells.duration,
                            recap_combats = self.features.recap_cells.fights,
                            recap_challenges = self.features.recap_cells.challenges,
                            "[options] fonctionnalités actives mises à jour"
                        );
                        let _ = self
                            .settings_tx
                            .send(EngineCommand::SetFeatures(self.features));
                    }
                    // Les deux sourdines (2026-09-15) — voir `main.rs` : seul le thread Engine
                    // a besoin de savoir quels sons se taisent, une sourdine ne se voit pas.
                    let mutes_changed = commit.mutes != self.alert_mutes;
                    if mutes_changed {
                        self.alert_mutes = commit.mutes;
                        tracing::info!(
                            suivi = self.alert_mutes.suivi,
                            chat = self.alert_mutes.chat,
                            "[options] sons d'alerte coupés mis à jour"
                        );
                        let _ = self
                            .settings_tx
                            .send(EngineCommand::SetAlertMutes(self.alert_mutes));
                    }
                    // La fermeture de la carte de décompte (2026-09-16) — voir `main.rs` : c'est
                    // le thread Engine qui pose `hide_at` quand l'alerte naît.
                    let countdown_toast_changed = commit.countdown_toast != self.countdown_toast;
                    if countdown_toast_changed {
                        self.countdown_toast = commit.countdown_toast;
                        tracing::info!(
                            duration_seconds = self.countdown_toast.duration_seconds,
                            manual_close = self.countdown_toast.manual_close,
                            "[options] fermeture de la carte de décompte mise à jour"
                        );
                        let _ = self
                            .settings_tx
                            .send(EngineCommand::SetCountdownToast(self.countdown_toast));
                    }
                    // **Les deux réglages de complétion (2026-09-17)** — ils ne partent PAS au
                    // thread Engine, contrairement à la fermeture ci-dessus : le moteur ne sait rien
                    // de la célébration ni du retrait, c'est l'hôte qui les décide à réception de la
                    // complétion (voir `about_to_wait`).
                    if commit.completion != self.completion {
                        self.completion = commit.completion;
                        tracing::info!(
                            remove = self.completion.remove,
                            animate = self.completion.animates(),
                            "[options] complétion d'un suivi mise à jour"
                        );
                    }
                    // La reprise de la session du Récap (2026-09-17) — voir `main.rs`.
                    let recap_resume_changed =
                        commit.recap_resume != self.recap_session.resume_settings();
                    if recap_resume_changed {
                        self.recap_session.set_resume_settings(commit.recap_resume);
                        tracing::info!(
                            enabled = commit.recap_resume.enabled,
                            minutes = commit.recap_resume.minutes,
                            "[options] reprise de la session du récap mise à jour"
                        );
                    }
                    // Raccourcis (2026-09-13) — voir `main.rs` : `apply` pendant la suspension ne
                    // touche pas encore l'OS, c'est `close_options_modal` qui enregistre.
                    let shortcuts_changed = commit.shortcuts != *self.hotkeys.bindings();
                    if shortcuts_changed {
                        tracing::info!("[options] raccourcis personnalisés mis à jour.");
                        self.hotkeys.apply(commit.shortcuts);
                    }
                    // **Le démarrage avec l'ordinateur (2026-09-16)** — le seul réglage de cette fenêtre
                    // qui ne passe NI par la config NI par le thread Engine : il s'inscrit dans le système
                    // (voir `autostart`, doc de module). `apply` compare à l'état réel avant d'écrire, et
                    // n'échoue jamais bruyamment — un refus du système laisse simplement la case revenir
                    // sur son état réel à la prochaine ouverture.
                    overlay_ui::autostart::apply(commit.start_with_os);
                    // **La mise à jour automatique (2026-09-15)** — persistée, effective au prochain lancement :
                    // c'est là que la première commande au thread de mise à jour se décide.
                    let auto_update_changed = commit.auto_update != self.auto_update;
                    if auto_update_changed {
                        self.auto_update = commit.auto_update;
                        tracing::info!(
                            "[options] mise à jour automatique au démarrage : {}",
                            if self.auto_update {
                                "activée"
                            } else {
                                "désactivée"
                            }
                        );
                    }
                    // **Le journal détaillé (2026-09-18, constat C6)** — appliqué À CHAUD, voir
                    // `main.rs` pour le motif.
                    let verbose_log_changed = commit.verbose_log != self.verbose_log;
                    if verbose_log_changed {
                        self.verbose_log = commit.verbose_log;
                        logging::set_verbose(self.verbose_log);
                    }
                    // La config est réécrite EN ENTIER, et seulement si l'un des réglages a bougé —
                    // même raison que `main.rs` : le fichier est réécrit d'un bloc.
                    let chat_toast_changed = self.commit_chat(options_window_id);
                    self.commit_personnages(options_window_id);
                    if path_changed
                        || combat_changed
                        || side_changed
                        || turn_changed
                        || turn_muted_changed
                        || shortcuts_changed
                        || chat_toast_changed
                        || features_changed
                        || mutes_changed
                        || countdown_toast_changed
                        || recap_resume_changed
                        || auto_update_changed
                        || verbose_log_changed
                    {
                        // **`persist_config` et rien d'autre** (2026-09-17) : cette liste de
                        // champs vivait ici en double de celle de `persist_config`, et elle avait
                        // déjà divergé — la position ET le verrou de la bande Récap y manquaient,
                        // si bien que valider cette fenêtre sous Linux effaçait du disque une
                        // bande qu'on venait de déplacer. C'est exactement le risque que la doc de
                        // `main.rs::App::persist_config` annonce (« deux copies de cette liste
                        // auraient divergé au premier réglage ajouté »), et la hauteur du panneau
                        // Combat en aurait été la troisième victime. `self.log_path` porte déjà le
                        // chemin validé (voir plus haut), il n'y a donc rien à passer.
                        self.persist_config();
                    }
                    // « Valider » commit TOUS les onglets — voir `main.rs`.
                    self.commit_alerts(options_window_id);
                    self.commit_suivi(options_window_id);
                    self.close_options_modal(options_window_id, "Valider");
                    // Sans cet appel, cocher la case ne se verrait qu'au prochain tick
                    // d'`about_to_wait` — même raison que `main.rs`.
                    self.sync_panel_visibility();
                }
                Err(err) => {
                    tracing::info!("[options] chemin refusé : {}", err.message());
                    if let Some(overlay) = self.windows.get_mut(&options_window_id) {
                        if let Some(state) = &mut overlay.options_state {
                            state.error = Some(err.message().to_string());
                        }
                        overlay.window.request_redraw();
                    }
                }
            }
        }
    }

    impl ApplicationHandler<UserEvent> for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            self.sync_session_windows(event_loop);
            self.sync_windows(event_loop);
            if !self.banner_printed {
                tracing::info!("=== wakfu-companion-overlay (Linux/X11, §17.2 du plan) ===");
                // Chemin EXPURGÉ du nom d'utilisateur du système (constat C6 de
                // `docs/analyse-rgpd.md`, `overlay_ingest::privacy`) : la forme du chemin — Steam,
                // Wine, installation native, dossier déplacé — reste entière, c'est elle qu'on lit
                // ici ; le chemin réel part en `debug` (« Journal détaillé »).
                tracing::info!(
                    "Suivi de {}",
                    overlay_ingest::privacy::redact_path(&self.log_path)
                );
                tracing::debug!(path = %self.log_path.display(), "chemin de journal suivi");
                // Libellés LUS dans les raccourcis effectifs (personnalisables) : une bannière
                // qui annoncerait les défauts à qui les a changés serait un contresens.
                let bindings = self.hotkeys.bindings();
                tracing::info!(
                    "Un compte est obligatoire : la fenêtre de connexion reste seule à l'écran \
                     tant qu'aucun n'est lié. \
                     {} pour basculer interactif / clic-traversant. \
                     {} pour la fenêtre Options, dont l'onglet « Raccourcis » et le bouton \
                     « Fermer l'overlay » (ou Ctrl+C dans ce terminal) pour quitter.",
                    bindings.label(ShortcutAction::Toggle),
                    bindings.label(ShortcutAction::Options),
                );
                self.banner_printed = true;
            }
        }

        fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
            match event {
                UserEvent::NewSnapshot
                | UserEvent::AuthStatusChanged
                | UserEvent::StartupProgress
                | UserEvent::UpdateProgress => {
                    for overlay in self.windows.values() {
                        overlay.window.request_redraw();
                    }
                }
            }
        }

        fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
            // Renseigné éventuellement PAR l'emprunt de `overlay` ci-dessous (voir la fin de cette
            // fonction) — regroupé plutôt que de multiples `bool`/`Option` séparés, pour un seul
            // `match` final au lieu de plusieurs `if` empilés. Ne fait AUCUN appel `&mut self`
            // avant la fin de cette fonction : `overlay` (obtenu juste après) emprunte
            // `self.windows` pour toute la durée de son dernier usage (NLL), un appel `&mut self`
            // plus tôt romprait la compilation.
            enum PostRedraw {
                None,
                /// Fenêtre de jeu **depuis laquelle** la modale est demandée — son XID et son
                /// rectangle. Voir `App::open_options_modal`. Le troisième champ est l'onglet à
                /// ouvrir — "+" demande « Suivi », "Options" demande « Paramètres » (voir
                /// `render_content::RenderOutcome::open_watchlist`/`open_options`).
                OpenOptions(u32, GameRect, options_modal::OptionsTab),
                CloseOptions,
                /// Glyphe de remise à zéro du Récap cliqué — voir `main.rs`.
                OpenResetConfirm(u32, GameRect, ResetTarget),
                /// La confirmation a répondu — voir `main.rs`.
                AnswerResetConfirm(ResetTarget, bool),
                /// Bouton de réinitialisation d'une tuile du bandeau cliqué — voir `main.rs`.
                OpenWatchlistResetConfirm(u32, GameRect, WatchlistEntry),
                /// Le cadenas de la bande Récap vient d'être cliqué (voir `toggle_recap_lock`).
                ToggleRecapLock,
                /// Le cadenas du panneau Combat vient d'être cliqué (voir `toggle_combat_lock`).
                ToggleCombatLock,
                /// Carte d'alerte de chat cliquée — voir `main.rs`.
                Whisper(String),
                BrowseOptions,
                /// Ce que « Valider » emporte de l'onglet « Paramètres » — voir
                /// `options_modal::OptionsCommit`.
                ValidateOptions(options_modal::OptionsCommit),
                /// Bouton « Déconnecter » de la section « Compte », après confirmation.
                DisconnectAccount,
                /// Résoudre les ingrédients de cet objet pour la fenêtre de recette.
                ResolveRecipe(i64),
                /// Section « Mise à jour » de la fenêtre Options — voir `main.rs`.
                CheckUpdate,
                InstallUpdate,
                /// « Réessayer » de l'écran « Mise à jour requise ».
                RetryUpdate,
                /// « Fermer l'overlay », après confirmation — voir `main.rs`.
                Quit,
                /// « Redémarrer », après confirmation — voir `main.rs`.
                Restart,
                /// « Mettre à jour maintenant » de l'écran de mise à jour manuelle — voir
                /// `main.rs`.
                StartManualUpdate,
                /// « Fermer » / « Plus tard » de l'écran de mise à jour manuelle.
                CloseManualUpdate,
            }
            let mut post_redraw = PostRedraw::None;
            // La bande Récap vient d'être reposée : la config est réécrite une fois le geste
            // fini — voir `main.rs::App::redraw`, même drapeau et même raison (une position
            // perdue ne se rattrape pas, elle ne doit pas dépendre d'un `PostRedraw` qu'une
            // autre action écraserait).
            let mut persist_recap_position = false;
            // Même mécanique pour la hauteur du panneau Combat (2026-09-17).
            let mut persist_combat_position = false;

            let Some(overlay) = self.windows.get_mut(&id) else {
                return;
            };

            let response = overlay
                .gpu
                .egui_winit
                .on_window_event(&overlay.window, &event);
            if response.repaint {
                overlay.window.request_redraw();
            }

            match event {
                WindowEvent::CloseRequested => {
                    // **La croix de la fenêtre Options ferme la MODALE, pas l'overlay.** Elle quittait
                    // tout jusqu'au 2026-09-12 : cette fenêtre est la seule focalisable (§9.1 du plan),
                    // donc la seule dont la croix est réellement atteignable à la souris, et elle
                    // tuait la session entière. Avec des modifications en attente, elle passe par la
                    // même garde que « Annuler » et Échap — les trois gestes ferment la même chose.
                    if overlay.kind == OverlayKind::Options {
                        match overlay.options_state.as_mut() {
                            Some(state) if state.is_dirty() => {
                                state.pending_close = true;
                                overlay.window.request_redraw();
                            }
                            _ => post_redraw = PostRedraw::CloseOptions,
                        }
                    } else if let OverlayKind::ResetConfirm(target) = overlay.kind {
                        // Fermer la question, c'est répondre « Non ».
                        post_redraw = PostRedraw::AnswerResetConfirm(target, false);
                    } else if overlay
                        .login_state
                        .as_ref()
                        .is_some_and(|state| state.manual_update)
                    {
                        // La croix de l'écran de mise à jour ferme CET écran, pas l'overlay —
                        // voir `main.rs`.
                        post_redraw = PostRedraw::CloseManualUpdate;
                    } else {
                        logging::log_session_end("fermeture de fenêtre");
                        event_loop.exit();
                    }
                }
                // **Aucun filet « Échap quitte l'overlay »** — retiré le 2026-09-17 en même
                // temps que son jumeau de `main.rs`, pour la même raison et avec le même
                // raisonnement (voir le commentaire là-bas) : les fenêtres overlay PEUVENT
                // recevoir le focus clavier, et une touche nue qui arrête le programme fermait
                // la session d'un geste aussi ordinaire que « cliquer Options puis taper Échap ».
                // Échap appartient aux panneaux qui le lisent ; les sorties propres sont le
                // bouton « Fermer l'overlay », la zone de notification, la fermeture de fenêtre
                // (`CloseRequested` ci-dessus) et Ctrl+C.
                WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                    let max_dim = overlay.gpu.device.limits().max_texture_dimension_2d;
                    overlay.gpu.config.width = size.width.min(max_dim);
                    overlay.gpu.config.height = size.height.min(max_dim);
                    overlay
                        .gpu
                        .surface
                        .configure(&overlay.gpu.device, &overlay.gpu.config);
                }
                // Fenêtre `Combat` masquée hors combat (voir `sync_panel_visibility`) : rien à
                // peindre ni à présenter. C'est `sync_panel_visibility` qui redemande un
                // redessin en la faisant réapparaître.
                WindowEvent::RedrawRequested if !overlay.visible => {}
                WindowEvent::RedrawRequested => {
                    let snapshot = self.snapshot.load();
                    let fight = snapshot.fight_for_character(&overlay.character_name);
                    let watchlist_all = self.watchlist.load_full();
                    // Suivi coupé : le bandeau n'affiche plus aucune tuile, le carré de contrôle
                    // reste — voir `main.rs`, même raisonnement et mêmes conséquences.
                    let watchlist: &[WatchlistEntry] = if self.features.suivi {
                        &watchlist_all
                    } else {
                        &[]
                    };
                    let watchlist_toast = self.watchlist_toast.load_full();
                    let watchlist_toast = watchlist_toast.as_ref().as_ref();
                    let now = std::time::Instant::now();

                    if overlay.kind == OverlayKind::Watchlist {
                        let toast_active = watchlist_toast.is_some();
                        let target_width = watchlist_target_width(
                            watchlist.len(),
                            self.features.suivi,
                            toast_active,
                            overlay.game_rect.width,
                        );
                        let target_height = watchlist_target_height(
                            toast_active,
                            self.watchlist_selection.is_open(),
                        );
                        if overlay.last_watchlist_width != Some(target_width)
                            || overlay.last_watchlist_height != Some(target_height)
                        {
                            // Sur X11, contrairement à Windows (voir `main.rs::
                            // reconfigure_surface`), `request_inner_size` renvoie `None` (résolu
                            // de façon asynchrone) : le vrai `WindowEvent::Resized` qui suit
                            // reconfigure la surface lui-même, voir la branche ci-dessus.
                            let _ =
                                overlay
                                    .window
                                    .request_inner_size(winit::dpi::LogicalSize::new(
                                        target_width,
                                        target_height,
                                    ));
                            overlay.last_watchlist_width = Some(target_width);
                            overlay.last_watchlist_height = Some(target_height);
                        }
                    }

                    let catalog = self.catalog.load();
                    let game_servers = self.game_servers.load();
                    let auth_status = self.auth_status.load();
                    // La modale Options force sa propre interactivité (voir
                    // `App::open_options_modal`) — jamais assujettie à `self.interactive` (mode
                    // clic-traversant global de Combat/Suivi), sans quoi elle deviendrait
                    // elle-même traversable si l'utilisateur avait basculé ce mode juste avant.
                    let interactive = matches!(
                        overlay.kind,
                        OverlayKind::Options | OverlayKind::Login | OverlayKind::ResetConfirm(_)
                    ) || self.interactive;
                    let this_game_rect = overlay.game_rect;
                    let this_game_window = overlay.game_window;
                    let overlay_kind = overlay.kind;
                    // Ce que la bande Récap sait d'elle-même par l'hôte (2026-09-17) — voir
                    // `main.rs`, même calcul.
                    let recap_chrome = panels::recap::RecapChrome {
                        locked: self.recap_locked,
                        moved: self.recap_position.is_some(),
                        actions_below: overlay.kind == OverlayKind::Recap && {
                            let outer = overlay.window.outer_size();
                            let (client, band) =
                                RecapAnchor::new(None, overlay.window.scale_factor()).geometry(
                                    overlay.game_rect,
                                    outer.width as i32,
                                    outer.height as i32,
                                );
                            recap_placement::actions_below(self.recap_position, client, band)
                        },
                    };
                    // Le chrome du panneau Combat — voir `main.rs::App::redraw`.
                    let combat_chrome = panels::combat::CombatChrome {
                        locked: self.combat_locked,
                        moved: self.combat_position_y.is_some(),
                    };
                    // La vue de la session du Récap — voir `main.rs`.
                    let recap_view = panels::recap::RecapView {
                        totals: self.recap_session.totals(&snapshot.totals),
                        uptime: self.recap_session.uptime(),
                        started_at: self.recap_session.started_at_local(),
                        resumed: self.recap_session.resumed(),
                    };
                    // L'état de la mise à jour, copié dans la fenêtre qui l'affiche AVANT chaque rendu
                    // (jamais figé à l'ouverture, voir `OptionsModalState::update`).
                    let update_status = self.update_status.load();
                    if let Some(state) = overlay.login_state.as_mut() {
                        if state.update != **update_status {
                            state.update = (**update_status).clone();
                        }
                    }
                    if let Some(state) = overlay.options_state.as_mut() {
                        state.update = (**update_status).clone();
                    }
                    // Voir `RenderContent::veiled` : la fenêtre Options rattachée à un client
                    // couvre sa fenêtre de jeu et la voile ; détachée, elle est à la taille de
                    // la modale.
                    let veiled = overlay.kind == OverlayKind::Options && !overlay.is_detached();
                    let (repaint_delay, outcome) = render(
                        &mut overlay.gpu,
                        &overlay.window,
                        event_loop,
                        RenderContent {
                            kind: overlay.kind,
                            fight,
                            portraits: &overlay.portraits,
                            combat_frame: &overlay.combat_frame,
                            icons: &overlay.icons,
                            avatars: overlay.avatars.as_ref(),
                            game_servers: &game_servers,
                            combat_side: &mut overlay.combat_side,
                            combat_metric: &mut overlay.combat_metric,
                            watchlist,
                            watchlist_enabled: self.features.suivi,
                            spells_enabled: self.features.spells_visible(),
                            combat_on_right: self.combat_on_right,
                            combat_chrome,
                            watchlist_selection: &mut self.watchlist_selection,
                            watchlist_completions: &self.watchlist_completions,
                            watchlist_reset: self.watchlist_reset_pending.as_ref(),
                            watchlist_toast,
                            catalog: &catalog,
                            catalog_stale: self.catalog_stale.load(Ordering::Relaxed),
                            remote_icons: &self.remote_icons,
                            remote_icon_textures: &mut overlay.remote_icon_textures,
                            auth_status: &auth_status,
                            auth_command_tx: &self.auth_command_tx,
                            interactive,
                            shortcuts: self.hotkeys.bindings(),
                            now,
                            recap_cells: self.features.recap_cells,
                            recap: &recap_view,
                            recap_chrome,
                            options: overlay.options_state.as_mut(),
                            veiled,
                            login: overlay.login_state.as_mut(),
                        },
                    );
                    // Bloc Récap : retaillé à la hauteur qu'il vient de mesurer — voir
                    // `main.rs`, même mécanique. Sous X11, `request_inner_size` est asynchrone :
                    // le `Resized` qui suit reconfigure la surface lui-même.
                    if overlay.kind == OverlayKind::Recap {
                        if let Some(height) = outcome.recap_height {
                            // Jamais pendant qu'on la tient — voir `main.rs`, même raison.
                            if overlay.last_recap_height != Some(height)
                                && self.recap_drag.is_none()
                            {
                                let _ = overlay.window.request_inner_size(
                                    winit::dpi::LogicalSize::new(
                                        panels::recap::WIDTH as f64,
                                        height as f64
                                            + render_content::RECAP_TOOLTIP_RESERVE as f64
                                            + render_content::RECAP_ACTIONS_RESERVE as f64,
                                    ),
                                );
                                overlay.last_recap_height = Some(height);
                                overlay.window.request_redraw();
                            }
                        }
                        // **La bande saisie à la souris** (2026-09-17) — voir
                        // `main.rs::App::redraw`, même calcul au pixel près : la position visée
                        // vaut « le curseur à l'écran, moins le point par lequel on tient la
                        // bande », sans rien accumuler d'une frame à l'autre.
                        let scale = overlay.window.scale_factor();
                        let outer = overlay.window.outer_size();
                        let (client, band) = RecapAnchor::new(None, scale).geometry(
                            overlay.game_rect,
                            outer.width as i32,
                            outer.height as i32,
                        );
                        let physical = |pos: egui::Pos2| {
                            (
                                (pos.x as f64 * scale).round() as i32,
                                (pos.y as f64 * scale).round() as i32,
                            )
                        };
                        let mut place = |position: Option<(i32, i32)>| {
                            let (x, y) = recap_placement::window_position(position, client, band);
                            let posed = PhysicalPosition::new(x, y);
                            if overlay.last_position != Some(posed) {
                                overlay.window.set_outer_position(posed);
                                overlay.last_position = Some(posed);
                            }
                        };
                        match outcome.recap_drag {
                            panels::recap::RecapDrag::None => {}
                            panels::recap::RecapDrag::Started(pos) => {
                                self.recap_drag = Some(RecapDragState {
                                    window: id,
                                    grab: physical(pos),
                                });
                            }
                            // Le curseur vient du serveur X11 (`QueryPointer` sur la racine), et
                            // non d'egui, dont la position est mesurée depuis le coin de cette
                            // fenêtre-ci : s'en servir pour la déplacer reboucle et la fait
                            // vibrer (voir `recap_placement::drag_offset`). Requête illisible :
                            // la bande reste où elle est.
                            panels::recap::RecapDrag::Moved => {
                                if let Some(drag) = self.recap_drag.filter(|drag| drag.window == id)
                                {
                                    if let Some(cursor) = self.game_window.cursor_position() {
                                        let offset = recap_placement::drag_offset(
                                            cursor, drag.grab, client, band,
                                        );
                                        if self.recap_position != Some(offset) {
                                            self.recap_position = Some(offset);
                                            place(Some(offset));
                                        }
                                    }
                                }
                            }
                            panels::recap::RecapDrag::Released => {
                                if self.recap_drag.is_some_and(|drag| drag.window == id) {
                                    self.recap_drag = None;
                                    self.recap_position =
                                        self.recap_position.and_then(recap_placement::snap);
                                    place(self.recap_position);
                                    persist_recap_position = true;
                                    match self.recap_position {
                                        Some((x, y)) => {
                                            tracing::info!("[recap] bande posée en {x} / {y}.")
                                        }
                                        None => tracing::info!(
                                            "[recap] bande revenue à son emplacement d'origine."
                                        ),
                                    }
                                }
                            }
                        }
                    }
                    // **Le panneau Combat saisi par sa poignée latérale** (2026-09-17) — voir
                    // `main.rs::App::redraw`, même calcul au pixel près, en une seule dimension :
                    // seule la hauteur bouge.
                    if overlay.kind == OverlayKind::Combat {
                        // Le côté est copié AVANT la fermeture qui pose la fenêtre : elle ne
                        // doit rien garder de `self`, dont d'autres champs changent juste après
                        // (la hauteur, l'état du glissement).
                        let on_right = self.combat_on_right;
                        let scale = overlay.window.scale_factor();
                        let outer = overlay.window.outer_size();
                        let (client, panel) = CombatAnchor::new(on_right, None, scale).geometry(
                            overlay.game_rect,
                            outer.width as i32,
                            outer.height as i32,
                        );
                        let mut place = |offset: Option<i32>| {
                            let (x, y) =
                                combat_placement::window_position(offset, on_right, client, panel);
                            let posed = PhysicalPosition::new(x, y);
                            if overlay.last_position != Some(posed) {
                                overlay.window.set_outer_position(posed);
                                overlay.last_position = Some(posed);
                            }
                        };
                        match outcome.combat_drag {
                            panels::drag::PanelDrag::None => {}
                            panels::drag::PanelDrag::Started(pos) => {
                                self.combat_drag = Some(CombatDragState {
                                    window: id,
                                    grab_y: (pos.y as f64 * scale).round() as i32,
                                });
                            }
                            panels::drag::PanelDrag::Moved => {
                                if let Some(drag) =
                                    self.combat_drag.filter(|drag| drag.window == id)
                                {
                                    if let Some((_, cursor_y)) = self.game_window.cursor_position()
                                    {
                                        let offset = combat_placement::drag_offset(
                                            cursor_y,
                                            drag.grab_y,
                                            client,
                                            panel,
                                        );
                                        if self.combat_position_y != Some(offset) {
                                            self.combat_position_y = Some(offset);
                                            place(Some(offset));
                                        }
                                    }
                                }
                            }
                            panels::drag::PanelDrag::Released => {
                                if self.combat_drag.is_some_and(|drag| drag.window == id) {
                                    self.combat_drag = None;
                                    self.combat_position_y =
                                        self.combat_position_y.and_then(|offset| {
                                            combat_placement::snap(offset, client, panel)
                                        });
                                    place(self.combat_position_y);
                                    persist_combat_position = true;
                                    match self.combat_position_y {
                                        Some(y) => tracing::info!(
                                            "[combat] panneau posé à la hauteur {y}."
                                        ),
                                        None => tracing::info!(
                                            "[combat] panneau revenu à sa hauteur d'origine."
                                        ),
                                    }
                                }
                            }
                        }
                    }
                    // Fenêtre de connexion : retaillée à la hauteur de la carte et recentrée —
                    // voir `main.rs`. Sous X11, `request_inner_size` est asynchrone (le
                    // `Resized` qui suit reconfigure la surface, voir plus haut) : le décalage de
                    // recentrage se calcule donc sur les hauteurs LOGIQUES demandées, pas sur
                    // `outer_size` qui n'a pas encore bougé.
                    if overlay.kind == OverlayKind::Login {
                        if let Some(height) = outcome.login_height {
                            if overlay.last_login_height != Some(height) {
                                let previous = overlay.last_login_height.unwrap_or(height);
                                let _ = overlay.window.request_inner_size(
                                    winit::dpi::LogicalSize::new(
                                        login::WINDOW_WIDTH as f64,
                                        height as f64,
                                    ),
                                );
                                if let Ok(position) = overlay.window.outer_position() {
                                    let shift =
                                        ((height - previous) as f64 * overlay.window.scale_factor()
                                            / 2.0)
                                            .round() as i32;
                                    overlay.window.set_outer_position(PhysicalPosition::new(
                                        position.x,
                                        position.y - shift,
                                    ));
                                }
                                overlay.last_login_height = Some(height);
                                overlay.window.request_redraw();
                            }
                        }
                        if outcome.drag_window {
                            if let Err(err) = overlay.window.drag_window() {
                                tracing::warn!(
                                    "[connexion] déplacement de la fenêtre refusé : {err}"
                                );
                            }
                        }
                        if outcome.retry_update {
                            post_redraw = PostRedraw::RetryUpdate;
                        }
                        // Écran de mise à jour manuelle — voir `main.rs`.
                        if outcome.check_update {
                            post_redraw = PostRedraw::CheckUpdate;
                        }
                        if outcome.install_update {
                            post_redraw = PostRedraw::StartManualUpdate;
                        }
                        if outcome.close_update {
                            post_redraw = PostRedraw::CloseManualUpdate;
                        }
                    }
                    if outcome.close_toast {
                        self.watchlist_toast.store(Arc::new(None));
                    }
                    // Carte d'alerte de chat cliquée — voir `main.rs`.
                    if let Some(author) = outcome.whisper_to {
                        post_redraw = PostRedraw::Whisper(author);
                    }
                    // Suppression groupée demandée depuis le bandeau : même chemin que la
                    // validation de l'onglet « Suivi » — seules les DÉFINITIONS partent, le moteur
                    // garde ses compteurs et réplique au compte de lui-même (voir
                    // `EngineCommand::SetWatchlistDefinitions`). L'`ArcSwap` local n'est pas touché
                    // ici : c'est le moteur qui republie la liste, compteurs vivants compris.
                    if let Some(edition) = outcome.watchlist_edit {
                        tracing::info!(
                            entry_count = edition.definitions.len(),
                            geste = edition.reason.label(),
                            "[bandeau] définitions modifiées"
                        );
                        let _ = self
                            .settings_tx
                            .send(EngineCommand::SetWatchlistDefinitions {
                                definitions: edition.definitions,
                                // Le bandeau n'a pas de brouillon : une entrée qu'il retire disparaît de la
                                // liste dans la foulée, son compteur part avec elle (rien à oublier en plus),
                                // et il ne peut en recréer aucune.
                                retirees: Vec::new(),
                            });
                    }
                    if outcome.open_watchlist {
                        post_redraw = PostRedraw::OpenOptions(
                            this_game_window,
                            this_game_rect,
                            options_modal::OptionsTab::Suivi,
                        );
                    }
                    if outcome.open_options {
                        post_redraw = PostRedraw::OpenOptions(
                            this_game_window,
                            this_game_rect,
                            options_modal::OptionsTab::Parametres,
                        );
                    }
                    // L'ouverture de page est faite ICI, par l'hôte, jamais par le panneau qui l'a
                    // demandée : voir `RenderOutcome::open_url`. `open::that` est best-effort, comme
                    // partout ailleurs — un navigateur qui ne s'ouvre pas ne fait rien planter.
                    if let Some(url) = &outcome.open_url {
                        let _ = open::that(url);
                    }
                    // Réinitialisation du compteur d'une tuile (2026-09-18) — voir `main.rs`.
                    if let Some(entry) = outcome.watchlist_reset_requested {
                        post_redraw = PostRedraw::OpenWatchlistResetConfirm(
                            this_game_window,
                            this_game_rect,
                            entry,
                        );
                    }
                    // Remise à zéro du Récap (2026-09-17) — voir `main.rs`.
                    if outcome.recap_reset_requested {
                        post_redraw = PostRedraw::OpenResetConfirm(
                            this_game_window,
                            this_game_rect,
                            ResetTarget::RecapSession,
                        );
                    }
                    // Le glyphe de replacement de la rangée d'actions (2026-09-17) — voir
                    // `main.rs`.
                    if outcome.recap_restore_requested {
                        post_redraw = PostRedraw::OpenResetConfirm(
                            this_game_window,
                            this_game_rect,
                            ResetTarget::RecapPosition,
                        );
                    }
                    // Le cadenas : bascule immédiate et persistée, sans confirmation — rien ne
                    // se perd, et le glyphe montre aussitôt l'état obtenu.
                    if outcome.recap_toggle_lock {
                        post_redraw = PostRedraw::ToggleRecapLock;
                    }
                    // Les deux mêmes commandes pour le panneau Combat (2026-09-17).
                    if outcome.combat_restore_requested {
                        post_redraw = PostRedraw::OpenResetConfirm(
                            this_game_window,
                            this_game_rect,
                            ResetTarget::CombatPosition,
                        );
                    }
                    if outcome.combat_toggle_lock {
                        post_redraw = PostRedraw::ToggleCombatLock;
                    }
                    if let OverlayKind::ResetConfirm(target) = overlay_kind {
                        match outcome.reset_choice {
                            overlay_ui::design::ConfirmChoice::Pending => {}
                            overlay_ui::design::ConfirmChoice::Yes => {
                                post_redraw = PostRedraw::AnswerResetConfirm(target, true)
                            }
                            overlay_ui::design::ConfirmChoice::No => {
                                post_redraw = PostRedraw::AnswerResetConfirm(target, false)
                            }
                        }
                    }
                    match outcome.options_action {
                        OptionsModalAction::None => {}
                        OptionsModalAction::Cancel => post_redraw = PostRedraw::CloseOptions,
                        OptionsModalAction::Browse => post_redraw = PostRedraw::BrowseOptions,
                        OptionsModalAction::Validate(commit) => {
                            post_redraw = PostRedraw::ValidateOptions(commit)
                        }
                        // Les sons d'essai se jouent par le même chemin qu'en jeu — c'est tout
                        // l'intérêt du bouton : entendre ce qu'on entendra.
                        OptionsModalAction::TestAlertSound => {
                            overlay_ui::alert_sound::play_loot_alert()
                        }
                        OptionsModalAction::TestChatSound => {
                            overlay_ui::alert_sound::play_chat_alert()
                        }
                        OptionsModalAction::TestCountdownSound => {
                            overlay_ui::alert_sound::play_countdown_alert()
                        }
                        OptionsModalAction::TestTurnSound => {
                            overlay_ui::alert_sound::play_turn_alert()
                        }
                        OptionsModalAction::Disconnect => {
                            post_redraw = PostRedraw::DisconnectAccount
                        }
                        OptionsModalAction::ResolveRecipe(id) => {
                            post_redraw = PostRedraw::ResolveRecipe(id)
                        }
                        OptionsModalAction::CheckUpdate => post_redraw = PostRedraw::CheckUpdate,
                        // Déjà traduite en `outcome.open_url` par `render_content` (voir la
                        // variante) ; ne parvient jamais ici.
                        OptionsModalAction::OpenUrl(_) => {}
                        OptionsModalAction::InstallUpdate => {
                            post_redraw = PostRedraw::InstallUpdate
                        }
                        OptionsModalAction::Quit => post_redraw = PostRedraw::Quit,
                        OptionsModalAction::Restart => post_redraw = PostRedraw::Restart,
                    }
                    overlay.next_redraw_at = (repaint_delay < std::time::Duration::from_secs(3600))
                        .then(|| std::time::Instant::now() + repaint_delay);
                }
                _ => {}
            }

            // Après la dernière ligne qui touche `overlay` — voir `main.rs`.
            if persist_recap_position || persist_combat_position {
                self.persist_config();
            }

            match post_redraw {
                PostRedraw::None => {}
                PostRedraw::OpenOptions(window, rect, tab) => {
                    self.open_options_modal(event_loop, Some((window, rect)), tab)
                }
                PostRedraw::CloseOptions => self.close_options_modal(id, "Annuler"),
                PostRedraw::OpenResetConfirm(window, rect, target) => {
                    self.open_reset_confirm(event_loop, window, rect, target)
                }
                PostRedraw::AnswerResetConfirm(target, confirmed) => {
                    self.answer_reset_confirm(id, target, confirmed)
                }
                PostRedraw::OpenWatchlistResetConfirm(window, rect, entry) => {
                    self.open_watchlist_reset_confirm(event_loop, window, rect, entry)
                }
                PostRedraw::ToggleRecapLock => self.toggle_recap_lock(),
                PostRedraw::ToggleCombatLock => self.toggle_combat_lock(),
                PostRedraw::Whisper(author) => self.whisper_from_toast(&author),
                PostRedraw::BrowseOptions => self.start_file_dialog(),
                PostRedraw::ValidateOptions(commit) => self.validate_and_commit_options(id, commit),
                // Voir `main.rs` : la commande part au thread Auth, qui efface le jeton ; l'hôte
                // verra `Disconnected` au prochain tick et reviendra à la fenêtre de connexion.
                PostRedraw::DisconnectAccount => {
                    let _ = self.auth_command_tx.send(AuthCommand::Disconnect);
                    tracing::info!(">>> Déconnexion du compte demandée (fenêtre Options).");
                    self.close_options_modal(id, "Déconnexion");
                }
                PostRedraw::ResolveRecipe(item_id) => self.start_recipe_resolution(id, item_id),
                PostRedraw::CheckUpdate => {
                    tracing::info!(">>> Recherche de mise à jour (fenêtre Options).");
                    let _ = self.update_command_tx.send(UpdateCommand::Check {
                        install_if_available: false,
                    });
                }
                PostRedraw::InstallUpdate => self.request_update_install(id),
                PostRedraw::RetryUpdate => {
                    let _ = self.update_command_tx.send(UpdateCommand::Check {
                        install_if_available: true,
                    });
                }
                // Même sortie que l'entrée « Quitter » de la zone de notification — voir `main.rs`.
                PostRedraw::Quit => {
                    logging::log_session_end("Fermer l'overlay (fenêtre Options)");
                    event_loop.exit();
                }
                // Même sortie, un process neuf en plus — voir `main.rs` et `restart::relaunch`.
                PostRedraw::Restart => match overlay_ui::restart::relaunch() {
                    Ok(()) => {
                        logging::log_session_end("Redémarrer l'overlay (fenêtre Options)");
                        event_loop.exit();
                    }
                    Err(err) => {
                        tracing::error!("[redémarrage] impossible de relancer l'overlay : {err}");
                    }
                },
                // Voir `main.rs` : même chemin que « Mettre à jour vers X » de la fenêtre
                // Options, sans fenêtre Options à refermer.
                PostRedraw::StartManualUpdate => {
                    tracing::info!(">>> Mise à jour demandée (écran de mise à jour).");
                    self.startup.set_update_blocking(true);
                    let _ = self.update_command_tx.send(UpdateCommand::Download);
                }
                PostRedraw::CloseManualUpdate => self.close_manual_update_window(event_loop),
            }
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            // Une mise à jour prête s'installe avant tout le reste — voir `main.rs`.
            self.install_update_if_ready(event_loop);
            if event_loop.exiting() {
                return;
            }
            // **Les suivis qui viennent d'aboutir**, avant tout le reste du tick : leur retrait
            // ne dépend ni d'une fenêtre visible ni d'un rendu (voir
            // `tick_watchlist_completions`).
            self.tick_watchlist_completions();
            // Bug réel trouvé côté X11 (spike S3, 2026-09-03, voir son README « Bugs réels
            // trouvés » n°3) : `global-hotkey` (XGrabKey) remonte DEUX événements par pression
            // (`Pressed` ET `Released`), contrairement à `WM_HOTKEY` sous Windows. Filtré sur
            // `Pressed` uniquement dès l'écriture de ce binaire — jamais reproduit ici comme un
            // "nouveau" bug, le correctif est appliqué avant même le premier lancement.
            while let Ok(event) = self.hotkey_events.try_recv() {
                if event.state != global_hotkey::HotKeyState::Pressed {
                    continue;
                }
                let Some(action) = self.hotkeys.action_for(event.id) else {
                    continue;
                };
                match action {
                    ShortcutAction::Toggle => self.toggle_interactive(),
                    ShortcutAction::WatchlistRemove => {
                        tracing::info!(
                            ">>> Supprimer ({})",
                            self.hotkeys
                                .bindings()
                                .label(ShortcutAction::WatchlistRemove)
                        );
                        self.watchlist_selection.toggle_mode();
                        // La fenêtre change de hauteur avec le mode, et rien d'autre ne la
                        // redessine : sans ce réveil, le raccourci n'aurait d'effet qu'au prochain
                        // événement venu d'ailleurs.
                        self.request_watchlist_redraw();
                    }
                    ShortcutAction::Options => {
                        tracing::info!(
                            ">>> Options ({})",
                            self.hotkeys.bindings().label(ShortcutAction::Options)
                        );
                        // Même destination que le bouton "Options" qu'il double — voir la doc de
                        // `PostRedraw::OpenOptions` (2026-09-13).
                        self.open_options_modal(
                            event_loop,
                            None,
                            options_modal::OptionsTab::Parametres,
                        );
                    }
                    ShortcutAction::InvitePartner => {
                        self.send_partner_command(ChatCommand::Invite);
                    }
                    ShortcutAction::FollowPartner => {
                        self.send_partner_command(ChatCommand::Follow);
                    }
                    // Jamais enregistrées ici — voir `ShortcutAction::LINUX_SUPPORTED`.
                    autre => tracing::debug!(
                        ">>> Action {autre:?} sans câblage dans le binaire Linux — ignorée."
                    ),
                }
            }

            // Résultat du dialogue de fichier natif (`App::start_file_dialog`), le cas échéant —
            // sondé sans bloquer, comme `hotkey_events` ci-dessus. `try_recv` retourne `Empty`
            // tant que l'utilisateur n'a pas fini d'interagir avec le dialogue OS (peut prendre
            // plusieurs secondes) : `self.pending_dialog` n'est vidé QUE sur une réponse effective
            // (`Ok`) ou un thread mort (`Disconnected`, dialogue en échec) — jamais sur `Empty`,
            // qui doit re-sonder au prochain tour.
            if let Some(rx) = &self.pending_dialog {
                match rx.try_recv() {
                    Ok(picked) => {
                        self.pending_dialog = None;
                        if let Some(path) = picked {
                            // Même garde que "Valider" (voir `validate_and_commit_options`) :
                            // `rfd` ne filtre QUE par extension, un `.log` mal nommé doit être
                            // refusé exactement pareil qu'une saisie manuelle invalide, jamais
                            // silencieusement accepté parce qu'il vient du dialogue natif — le
                            // champ affiche quand même le chemin choisi (l'utilisateur voit ce
                            // qu'il a sélectionné) accompagné du message d'erreur, plutôt que de
                            // l'ignorer en silence.
                            let validation = discovery::validate_log_path(&path);
                            if let Some(overlay) = self
                                .windows
                                .values_mut()
                                .find(|w| w.kind == OverlayKind::Options)
                            {
                                if let Some(state) = &mut overlay.options_state {
                                    state.path_input = path.display().to_string();
                                    state.error = validation.err().map(|e| e.message().to_string());
                                }
                                overlay.window.request_redraw();
                            }
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => self.pending_dialog = None,
                }
            }

            // Ingrédients d'une recette, même sondage non bloquant — voir `main.rs`.
            if let Some((window_id, rx)) = &self.pending_recipe {
                let window_id = *window_id;
                match rx.try_recv() {
                    Ok(ingredients) => {
                        self.pending_recipe = None;
                        if let Some(overlay) = self.windows.get_mut(&window_id) {
                            if let Some(state) = &mut overlay.options_state {
                                if let Some(dialogue) = state.suivi.recipe.as_mut() {
                                    dialogue.ingredients = Some(ingredients);
                                }
                            }
                            overlay.window.request_redraw();
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => self.pending_recipe = None,
                }
            }

            // Fenêtre de connexion ou overlays de jeu, jamais les deux — voir `main.rs`.
            self.sync_session_windows(event_loop);
            self.sync_windows(event_loop);
            // Apparition/disparition automatique du panneau Combat — entre les deux, même ordre
            // que `main.rs` : `sync_windows` vient peut-être de créer la fenêtre, `sync_topmost`
            // doit voir son état final.
            self.sync_panel_visibility();
            self.sync_topmost();

            let now = std::time::Instant::now();
            let mut next_wake = now + POLL_INTERVAL;
            for overlay in self.windows.values_mut() {
                if let Some(due) = overlay.next_redraw_at {
                    if due <= now {
                        overlay.next_redraw_at = None;
                        overlay.window.request_redraw();
                    } else {
                        next_wake = next_wake.min(due);
                    }
                }
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(next_wake));
        }
    }

    /// Backend GPU logiciel (lavapipe/llvmpipe) — voir `spikes/s3-window-linux/README.md`
    /// §"Prérequis n°1" pour la validation complète sous Xvfb. `VULKAN | GL` plutôt qu'un seul
    /// backend forcé : sur une machine avec un vrai GPU, `wgpu` choisit le backend natif
    /// disponible ; sous Xvfb (aucun GPU réel), seul l'ICD logiciel installé (lavapipe le plus
    /// souvent) répond. Contrairement à `main.rs::init_gpu` (Windows, DX12/DirectComposition), ce
    /// choix de backend N'EST PAS partageable entre les deux binaires (voir la doc de `lib.rs`).
    async fn init_gpu(window: Arc<Window>) -> GpuState {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("création de la surface X11");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .expect(
                "aucun adaptateur Vulkan/GL compatible (voir spikes/s3-window-linux/README.md)",
            );
        tracing::info!("Adaptateur GPU : {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("wakfu-companion-overlay-x11-device"),
                required_features: wgpu::Features::empty(),
                // Plancher `webgl2` (l'overlay ne demande rien de plus), mais avec les limites
                // de RÉSOLUTION de l'adaptateur : la swapchain de la confirmation de remise à
                // zéro couvre la fenêtre de jeu ENTIÈRE (`OverlayKind::ResetConfirm`), et
                // `downlevel_webgl2_defaults()` plafonne une texture 2D à 2048 px — un jeu en
                // 2560×1392 faisait donc paniquer `Surface::configure` au clic sur le glyphe
                // (2026-09-17). L'idiome est celui que wgpu documente sur `using_resolution`.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
                ..Default::default()
            })
            .await
            .expect("création du device");

        let size = window.inner_size();
        // Même clamp défensif qu'à la reconfiguration (voir `reconfigure_surface` côté
        // Windows, `WindowEvent::Resized` côté Linux) : une erreur wgpu est FATALE par défaut,
        // une fenêtre plus grande que ce que le GPU accepte ne doit jamais tuer l'overlay.
        let max_dim = device.limits().max_texture_dimension_2d;
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.clamp(1, max_dim),
            height: size.height.clamp(1, max_dim),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);

        let egui_ctx = egui::Context::default();
        // Style partagé avec le binaire Windows ET le harnais de rendu offscreen
        // (`overlay_ui::style`, voir sa doc) — remplace un réglage local qui avait déjà divergé de
        // celui de `main.rs` (curseur "main" et design system tooltip absents ici avant ce
        // refactor, 2026-09-06).
        overlay_ui::style::apply(&egui_ctx);
        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer =
            egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());

        GpuState {
            // Conservée pour la même raison que côté Windows (voir la doc de
            // `GpuState::instance`) même si `recreate_surface` n'est pas appelée ici — X11/EWMH
            // n'a pas de `Commit` DirectComposition séparé à rejouer, voir sa doc.
            instance,
            surface,
            device,
            queue,
            config,
            egui_ctx,
            egui_winit,
            egui_renderer,
            occluded_since: None,
        }
    }

    /// Ordre de priorité complet (voir `config::resolve_log_path`) : argument CLI > chemin
    /// sauvegardé par une validation précédente de la modale Options (2026-09-08) > découverte
    /// automatique. Seule la découverte automatique peut échouer complètement (aucun argument, rien
    /// en config, aucun chemin connu du système) — dans ce cas SEULEMENT, ce binaire refuse encore
    /// de démarrer sans chemin explicite : un futur lot pourrait démarrer quand même et laisser la
    /// modale Options seule responsable de fixer un premier chemin, mais ce n'est pas encore câblé
    /// ainsi (l'Engine a besoin d'un chemin dès `spawn_engine_thread`, voir `run`).
    fn resolve_path(config: &config::OverlayConfig, cli_arg: Option<PathBuf>) -> PathBuf {
        match config::resolve_log_path(cli_arg, config) {
            Some(path) => path,
            None => {
                tracing::error!("wakfu.log introuvable aux emplacements connus. Chemins essayés :");
                for candidate in discovery::candidate_paths() {
                    tracing::error!("  - {}", overlay_ingest::privacy::redact_path(&candidate));
                }
                tracing::error!(
                    "Précisez le chemin explicitement : cargo run -p overlay-ui \
                     --bin wakfu-companion-overlay-x11 -- <chemin>"
                );
                logging::log_session_end("échec de démarrage (wakfu.log introuvable)");
                std::process::exit(1);
            }
        }
    }

    pub fn run() {
        let log_dir = logging::init();
        logging::install_ctrlc_handler();
        logging::install_panic_hook();
        if let Some(dir) = &log_dir {
            tracing::info!(
                "journal de session : {}",
                overlay_ingest::privacy::redact_path(dir)
            );
            tracing::debug!(dir = %dir.display(), "dossier du journal de session");
        }

        // Référentiels de sorts (classes et monstres) construits DÈS LE DÉMARRAGE, jamais au premier
        // sort affiché en combat — décision utilisateur du 13 sept. 2026 : deux fichiers de quelques
        // centaines de Ko, très loin du budget mémoire, et aucun à-coup en jeu.
        let (class_spells, monster_spells) = overlay_engine::preload_spell_indexes();
        tracing::info!("référentiels de sorts chargés : {class_spells} sorts de classe, {monster_spells} sorts de monstres");

        let mut saved_config = config::load();
        // **Journal détaillé**, dès que la config est lue (constat C6 de `docs/analyse-rgpd.md`) :
        // `logging::init` démarre toujours au niveau ordinaire — c'est le défaut voulu, rien de
        // personnel dans le fichier tant que rien n'a été demandé — et c'est ici, et seulement si
        // la case est cochée, que les `debug!` s'ouvrent.
        logging::set_verbose(saved_config.verbose_log);
        // Actif par défaut, une seule fois par installation — voir `autostart`, doc de module.
        overlay_ui::autostart::enable_by_default_once(&mut saved_config);

        // `--updated-from X` (relance après une mise à jour, voir `overlay_sync::update::apply`)

        // n'est pas un chemin de log : les arguments sont triés avant de résoudre le fichier.

        let cli = update_apply::parse_args(env::args().skip(1));

        if let Some(from) = &cli.updated_from {
            tracing::info!(
                "[mise à jour] mis à jour {from} → {} — nettoyage du dossier de mise à jour.",
                build_info::VERSION
            );

            update_apply::cleanup(&overlay_sync::update::updates_dir());
        }

        let log_path = resolve_path(&saved_config, cli.log_path);
        let snapshot = Arc::new(ArcSwap::from_pointee(SessionSnapshot::default()));
        let watchlist = Arc::new(ArcSwap::from_pointee(Vec::<WatchlistEntry>::new()));
        let watchlist_toast = Arc::new(ArcSwap::from_pointee(None::<WatchlistToast>));
        let catalog = Arc::new(ArcSwap::from_pointee(CatalogIndex::default()));
        let catalog_stale = Arc::new(AtomicBool::new(false));
        let dungeons = Arc::new(ArcSwap::from_pointee(DungeonIndex::default()));
        let alert_profile: SharedAlertProfile = Arc::new(ArcSwap::from_pointee(None));
        let chat_filters: SharedChatFilters = Arc::new(ArcSwap::from_pointee(None));
        let roster_draft: SharedRosterDraft = Arc::new(ArcSwap::from_pointee(None));
        // Les serveurs de jeu ne sont pas un jalon de démarrage : voir la doc du thread.
        let game_servers: Arc<ArcSwap<GameServers>> =
            Arc::new(ArcSwap::from_pointee(GameServers::default()));
        overlay_ui::background::spawn_game_servers_thread(Arc::clone(&game_servers));
        // Publié par le thread Engine pour la surveillance de tour — sans effet sous X11 pour
        // l'instant (voir `turn_notification`), mais le thread l'attend.
        let roster: overlay_ui::engine_thread::SharedRoster = Arc::new(ArcSwap::from_pointee(None));
        let auth_status = Arc::new(ArcSwap::from_pointee(AuthStatus::Connecting));
        let startup = Arc::new(StartupProgress::new());
        let update_status = Arc::new(ArcSwap::from_pointee(UpdateStatus::Idle));

        let game_window = GameWindowTracker::connect()
            .expect("connexion X11 pour la découverte de fenêtres ($DISPLAY défini ?)");

        let event_loop = EventLoop::<UserEvent>::with_user_event()
            .with_x11()
            .build()
            .expect("création de l'event loop");
        let proxy = event_loop.create_proxy();
        // Les mêmes threads de fond que Windows (`overlay_ui::background`, 2026-09-14) : compte,
        // synchro, catalogue, donjons, icônes réseau.
        let (settings_tx, settings_rx) = mpsc::channel();
        let (auth_command_tx, auth_command_rx) = mpsc::channel();
        let (sync_tx, sync_rx) = mpsc::channel();
        spawn_sync_thread(sync_rx);
        spawn_auth_thread(
            settings_tx.clone(),
            sync_tx.clone(),
            Arc::clone(&auth_status),
            auth_command_rx,
            proxy.clone(),
        );
        spawn_catalog_thread(
            Arc::clone(&catalog),
            Arc::clone(&catalog_stale),
            Arc::clone(&startup),
            proxy.clone(),
        );
        spawn_dungeon_thread(Arc::clone(&dungeons), Arc::clone(&startup), proxy.clone());
        // Mise à jour automatique (2026-09-15, `docs/plan-mise-a-jour.md` §7) : la vérification part
        // tout de suite, derrière l'écran de chargement ; avec « Installer automatiquement » coché,
        // une version plus récente est installée avant d'ouvrir le moindre overlay de jeu.
        let (update_command_tx, update_command_rx) = mpsc::channel();
        spawn_update_thread(
            Arc::clone(&update_status),
            Arc::clone(&startup),
            update_command_rx,
            proxy.clone(),
        );
        let _ = update_command_tx.send(UpdateCommand::Check {
            install_if_available: saved_config.auto_update,
        });
        let remote_icons = RemoteIconStore::spawn(proxy.clone());
        // **Le canal des complétions** (2026-09-17) — voir `engine_thread::WatchlistCompleted` sur
        // pourquoi un canal et pas un `ArcSwap` : une complétion perdue est une entrée jamais retirée.
        let (completions_tx, completions_rx) = mpsc::channel();
        spawn_engine_thread(
            log_path.clone(),
            EngineHandles {
                snapshot: Arc::clone(&snapshot),
                watchlist: Arc::clone(&watchlist),
                watchlist_toast: Arc::clone(&watchlist_toast),
                completions: completions_tx,
                alert_profile: Arc::clone(&alert_profile),
                chat_filters: Arc::clone(&chat_filters),
                roster,
                roster_draft: Arc::clone(&roster_draft),
                catalog: Arc::clone(&catalog),
                dungeons,
                startup: Arc::clone(&startup),
            },
            proxy,
            settings_rx,
            sync_tx,
        );
        // Les réglages de la carte de chat sont locaux : le moteur les reçoit d'ici. Les
        // interrupteurs de fonctionnalité aussi — voir `main.rs` pour pourquoi cet envoi ne peut
        // pas attendre la première validation de la fenêtre Options.
        let _ = settings_tx.send(EngineCommand::SetChatToast(saved_config.chat_toast()));
        let _ = settings_tx.send(EngineCommand::SetCountdownToast(
            saved_config.countdown_toast(),
        ));
        let _ = settings_tx.send(EngineCommand::SetFeatures(saved_config.features()));
        // Les sourdines de même — voir `main.rs`.
        let _ = settings_tx.send(EngineCommand::SetAlertMutes(saved_config.alert_mutes()));

        event_loop.set_control_flow(ControlFlow::Wait);
        let mut app = App::new(AppState {
            log_path,
            combat_always_visible: saved_config.combat_always_visible,
            combat_on_right: saved_config.combat_on_right,
            combat_position_y: saved_config.combat_position_y,
            combat_locked: saved_config.combat_locked,
            turn_notification: saved_config.turn_notification,
            turn_notification_muted: saved_config.turn_notification_muted,
            features: saved_config.features(),
            alert_mutes: saved_config.alert_mutes(),
            chat_toast: saved_config.chat_toast(),
            countdown_toast: saved_config.countdown_toast(),
            completion: saved_config.completion(),
            completions_rx,
            // Voir `main.rs` : à côté des combats en cours.
            recap_session: RecapSession::load(
                overlay_engine::fight_store::default_store_dir().join(recap_session::FILE_NAME),
                saved_config.recap_resume(),
                std::time::SystemTime::now(),
            ),
            recap_position: saved_config.recap_position(),
            recap_locked: saved_config.recap_locked,
            shortcuts: saved_config.shortcuts(),
            snapshot,
            watchlist,
            watchlist_toast,
            catalog,
            catalog_stale,
            remote_icons,
            auth_status,
            auth_command_tx,
            startup,
            update_status,
            update_command_tx,
            auto_update: saved_config.auto_update,
            verbose_log: saved_config.verbose_log,
            alert_profile,
            chat_filters,
            roster_draft,
            game_servers,
            game_window,
            settings_tx,
        });
        event_loop.run_app(&mut app).expect("boucle d'événements");
    }
}
