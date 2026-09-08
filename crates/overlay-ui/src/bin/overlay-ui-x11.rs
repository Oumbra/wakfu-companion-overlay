//! `overlay-ui-x11` — binaire Linux/X11 (docs/plan-architecture.md §17.2, Niveau 2 : critère de
//! sortie de S3, point « multi-fenêtres réel côté `overlay-ui` »). Réutilise de la LIB tout ce qui
//! ne dépend pas de l'OS (voir la doc de `lib.rs`) : `frame::render`, `engine_thread::
//! spawn_engine_thread`, `game_window` n'est PAS réutilisé ici — voir plus bas pourquoi ce binaire
//! parle directement à `overlay_platform::linux::x11` — `panels`, `render_content`, `logging`.
//! Fenêtrage réel via winit natif X11 (`WindowLevel`, `set_cursor_hittest`,
//! `with_x11_window_type`) — mêmes primitives déjà validées par `spikes/s3-window-linux/`, voir
//! son README pour la preuve programmatique sous Xvfb (Shape, stacking, focus).
//!
//! **Portée volontairement réduite par rapport à `main.rs` (Windows), documentée honnêtement
//! (§17.2 « État ») — mode INVITÉ uniquement pour cette première version :**
//! - Pas de compte lié / synchro serveur (lots L4-L5) : aucun thread Auth/Sync/Catalogue/Donjons.
//!   `catalog`/`dungeons` restent à leurs valeurs par défaut (vides), `auth_status` reste
//!   `Connected` fixe (aucune icône de relance d'appairage n'est jamais affichée),
//!   `auth_command_tx` est un `NoopAuthSink` (rien n'écoute de toute façon).
//! - Pas de hotkey rafraîchissement/déconnexion (`Ctrl+Shift+R`/`Ctrl+Alt+D`) : sans thread
//!   Auth/Catalogue à redemander, ils n'auraient aucun effet ici. Seuls bascule (`HOTKEY_LABEL`)
//!   et sortie (`QUIT_HOTKEY_LABEL`) sont câblés.
//! - Icônes réelles d'objets/monstres : `RemoteIconStore::empty()` (pas de thread réseau, voir sa
//!   doc) — le panneau Suivi retombe sur l'icône générique, comme en mode invité côté Windows.
//!
//! Ce que ce binaire couvre RÉELLEMENT (pas un stub, pas un panneau de diagnostic comme le spike
//! S3) : ingestion + moteur réels sur un vrai `wakfu.log`, panneaux Combat/Suivi réels
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
        "overlay-ui-x11 est réservé à Linux/X11 (voir docs/plan-architecture.md §17.2) — \
         utilisez le binaire overlay-ui sous Windows."
    );
    std::process::exit(1);
}

#[cfg(not(target_os = "windows"))]
mod linux_main {
    use std::collections::HashMap;
    use std::env;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;

    use arc_swap::ArcSwap;
    use egui_wgpu::wgpu;
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};
    use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
    use overlay_engine::{CatalogIndex, DungeonIndex, SessionSnapshot, WatchlistEntry};
    use overlay_ingest::discovery;
    use overlay_platform::linux::topmost::{self, TopmostAction, TopmostState};
    use overlay_platform::linux::x11::{GameRect, GameWindowTracker};
    use overlay_ui::config;
    use overlay_ui::engine_thread::{
        spawn_engine_thread, EngineCommand, EngineHandles, SyncCommand,
    };
    use overlay_ui::frame::{render, GpuState};
    use overlay_ui::logging;
    use overlay_ui::panels;
    use overlay_ui::panels::combat::CombatSide;
    use overlay_ui::panels::combat_frame::CombatFrame;
    use overlay_ui::panels::options_modal::{self, OptionsModalAction, OptionsModalState};
    use overlay_ui::panels::watchlist::WatchlistToast;
    use overlay_ui::portraits::PortraitAtlas;
    use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
    use overlay_ui::render_content;
    use overlay_ui::render_content::{
        AuthStatus, NoopAuthSink, OverlayKind, RenderContent, UserEvent,
    };
    use overlay_ui::ui_icons::UiIcons;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::application::ApplicationHandler;
    use winit::dpi::PhysicalPosition;
    use winit::event::{ElementState, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::keyboard::{KeyCode, PhysicalKey};
    use winit::platform::x11::{EventLoopBuilderExtX11, WindowAttributesExtX11, WindowType};
    use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

    const HOTKEY_LABEL: &str = "Ctrl+Shift+W";
    const QUIT_HOTKEY_LABEL: &str = "Ctrl+Shift+Q";
    /// Modale Options (2026-09-08, §9 du plan) — même raccourci que Windows (`main.rs::
    /// OPTIONS_HOTKEY_LABEL`), câblé ICI contrairement à Rafraîchissement/Déconnexion (voir la doc
    /// de module) : cette fonctionnalité n'a rien à voir avec le compte lié, absent en mode invité.
    const OPTIONS_HOTKEY_LABEL: &str = "Ctrl+Shift+O";
    /// Même cadence que Windows (§6.5 du plan) : 20 Hz pour l'ancrage/topmost/hotkey.
    const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
    // Hauteur élargie de `render_content::COMBAT_TOP_MARGIN` (2026-09-06, retour utilisateur :
    // tooltips du switch Alliés/Ennemis affichées en dessous faute de place au-dessus, même
    // correctif que `main.rs::WINDOW_SIZE`) — voir sa doc.
    const WINDOW_SIZE: (f64, f64) = (420.0, 480.0 + render_content::COMBAT_TOP_MARGIN as f64);
    const WATCHLIST_HEIGHT: f64 = 132.0;
    const WATCHLIST_INNER_MARGIN: f64 = 12.0;
    const WATCHLIST_WIDTH_FRACTION: f64 = 0.5;
    const WATCHLIST_MAX_CEILING: f64 = 1000.0;
    const GAME_EDGE_MARGIN_PX: i32 = 12;
    const GAME_TOP_MARGIN_PX: i32 = 28;

    /// Même calcul que `main.rs::watchlist_target_width` (voir sa doc pour le détail) — dupliqué
    /// plutôt que partagé : petite fonction pure, coût de duplication largement inférieur au coût
    /// d'une abstraction supplémentaire pour un si petit nombre de lignes.
    fn watchlist_target_width(entry_count: usize, toast_active: bool, game_width_px: i32) -> f64 {
        let ceiling = (game_width_px as f64 * WATCHLIST_WIDTH_FRACTION).min(WATCHLIST_MAX_CEILING);
        let tiles = panels::watchlist::content_width(entry_count) as f64;
        let toast = if toast_active {
            panels::watchlist::TOAST_LAYER_WIDTH as f64
        } else {
            0.0
        };
        let content = tiles.max(toast) + WATCHLIST_INNER_MARGIN;
        content.min(ceiling).max(WATCHLIST_INNER_MARGIN)
    }

    fn watchlist_target_height(toast_active: bool) -> f64 {
        WATCHLIST_HEIGHT
            + if toast_active {
                panels::watchlist::TOAST_AREA_HEIGHT as f64
            } else {
                0.0
            }
    }

    struct OverlayWindow {
        window: Arc<Window>,
        gpu: GpuState,
        kind: OverlayKind,
        portraits: PortraitAtlas,
        combat_frame: CombatFrame,
        icons: UiIcons,
        remote_icon_textures: RemoteIconTextures,
        combat_side: CombatSide,
        /// État de la modale Options (2026-09-08) — `Some` UNIQUEMENT pour `kind ==
        /// OverlayKind::Options`, voir `App::open_options_modal`.
        options_state: Option<OptionsModalState>,
        /// XID X11 de la fenêtre de jeu — équivalent du `hwnd` côté Windows (voir
        /// `overlay_platform::linux::x11::GameWindowInfo`). `0` (aucune XID valide) pour
        /// `OverlayKind::Options` : cette fenêtre n'est pas rattachée à une fenêtre de jeu précise
        /// — voir `App::sync_windows`/`sync_topmost`, qui l'excluent explicitement de toute
        /// logique basée sur ce champ pour cette raison.
        game_window: u32,
        game_rect: GameRect,
        character_name: String,
        last_position: Option<PhysicalPosition<i32>>,
        last_watchlist_width: Option<f64>,
        last_watchlist_height: Option<f64>,
        /// État topmost/délai de grâce — voir `overlay_platform::linux::topmost` (7 tests
        /// unitaires, déjà couvert avant ce binaire).
        topmost_state: TopmostState,
        next_redraw_at: Option<std::time::Instant>,
    }

    struct App {
        windows: HashMap<WindowId, OverlayWindow>,
        #[allow(dead_code)] // jamais relu : sa seule raison d'être est de rester en vie
        hotkey_manager: GlobalHotKeyManager,
        hotkey_events: &'static global_hotkey::GlobalHotKeyEventReceiver,
        toggle_hotkey_id: u32,
        quit_hotkey_id: u32,
        options_hotkey_id: u32,
        interactive: bool,
        snapshot: Arc<ArcSwap<SessionSnapshot>>,
        watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
        watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
        catalog: Arc<ArcSwap<CatalogIndex>>,
        remote_icons: RemoteIconStore,
        auth_status: AuthStatus,
        auth_command_tx: NoopAuthSink,
        /// Voir la doc de `AppState::settings_tx` — permet à `validate_and_apply_log_path`
        /// d'envoyer `EngineCommand::ChangeLogPath` sans redémarrer tout le binaire.
        settings_tx: mpsc::Sender<EngineCommand>,
        log_path: PathBuf,
        game_window: GameWindowTracker,
        banner_printed: bool,
        /// Dialogue de fichier natif (`rfd`) en cours, le cas échéant — voir
        /// `App::start_file_dialog`. Un seul à la fois (une seule modale Options peut être ouverte,
        /// voir `open_options_modal`), sondé sans bloquer à chaque `about_to_wait` (même motif que
        /// `hotkey_events`).
        pending_dialog: Option<mpsc::Receiver<Option<PathBuf>>>,
    }

    struct AppState {
        log_path: PathBuf,
        snapshot: Arc<ArcSwap<SessionSnapshot>>,
        watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
        watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
        catalog: Arc<ArcSwap<CatalogIndex>>,
        remote_icons: RemoteIconStore,
        game_window: GameWindowTracker,
        /// Canal vers le thread Engine (2026-09-08, §9 du plan) — voir
        /// `engine_thread::EngineCommand::ChangeLogPath`. Contrairement au reste de ce binaire
        /// (mode invité fixe), CE canal est bien câblé côté thread Engine (`spawn_engine_thread`
        /// l'accepte déjà, quel que soit le binaire) : seule sa moitié `settings_rx` était jusqu'ici
        /// abandonnée sans jamais recevoir de commande, ce lot lui en donne enfin une à traiter.
        settings_tx: mpsc::Sender<EngineCommand>,
    }

    impl App {
        fn new(state: AppState) -> Self {
            let AppState {
                log_path,
                snapshot,
                watchlist,
                watchlist_toast,
                catalog,
                remote_icons,
                game_window,
                settings_tx,
            } = state;

            let hotkey_manager = GlobalHotKeyManager::new().expect("création GlobalHotKeyManager");
            let toggle_hotkey =
                HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyW);
            let quit_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyQ);
            let options_hotkey =
                HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyO);
            hotkey_manager
                .register(toggle_hotkey)
                .expect("enregistrement du hotkey global");
            hotkey_manager
                .register(quit_hotkey)
                .expect("enregistrement du hotkey de sortie");
            hotkey_manager
                .register(options_hotkey)
                .expect("enregistrement du hotkey Options");

            Self {
                windows: HashMap::new(),
                hotkey_manager,
                hotkey_events: GlobalHotKeyEvent::receiver(),
                toggle_hotkey_id: toggle_hotkey.id(),
                quit_hotkey_id: quit_hotkey.id(),
                options_hotkey_id: options_hotkey.id(),
                interactive: true,
                snapshot,
                watchlist,
                watchlist_toast,
                catalog,
                remote_icons,
                // Fixe : aucun thread Auth ici (mode invité, voir la doc de module) — jamais
                // `Disconnected`, donc jamais d'icône de relance d'appairage affichée.
                auth_status: AuthStatus::Connected,
                auth_command_tx: NoopAuthSink,
                settings_tx,
                log_path,
                game_window,
                banner_printed: false,
                pending_dialog: None,
            }
        }

        /// Même politique que `main.rs::App::sync_windows` (voir sa doc) : converge `self.windows`
        /// vers l'état actuel des fenêtres de jeu trouvées.
        fn sync_windows(&mut self, event_loop: &ActiveEventLoop) {
            let found = self.game_window.scan();

            self.windows.retain(|_, overlay| {
                // La modale Options (2026-09-08) n'est PAS rattachée à une fenêtre de jeu précise
                // (voir la doc de `OverlayWindow::game_window`) — jamais retirée par ce scan, sa
                // durée de vie est pilotée exclusivement par l'utilisateur (Annuler/Valider), voir
                // `window_event`.
                if overlay.kind == OverlayKind::Options {
                    return true;
                }
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
                for kind in [OverlayKind::Combat, OverlayKind::Watchlist] {
                    if let Some(existing) = self
                        .windows
                        .values_mut()
                        .find(|w| w.game_window == info.window && w.kind == kind)
                    {
                        Self::reposition(existing, info.rect);
                        continue;
                    }
                    let overlay = Self::create_overlay_window(
                        event_loop,
                        kind,
                        info.window,
                        character_name.clone(),
                        info.rect,
                        self.interactive,
                    );
                    tracing::info!(
                        "[fenêtre de jeu] {character_name} trouvée — overlay {kind:?} créé."
                    );
                    overlay.window.request_redraw();
                    self.windows.insert(overlay.window.id(), overlay);
                }
            }
        }

        /// Même ancrage que Windows (voir `main.rs::App::anchor_position`) : Combat au bord gauche
        /// centré verticalement, Suivi au bord haut centré horizontalement.
        fn anchor_position(
            kind: OverlayKind,
            rect: GameRect,
            overlay_width: i32,
            overlay_height: i32,
        ) -> PhysicalPosition<i32> {
            match kind {
                OverlayKind::Combat => PhysicalPosition::new(
                    rect.left + GAME_EDGE_MARGIN_PX,
                    rect.top + (rect.height - overlay_height) / 2,
                ),
                OverlayKind::Watchlist => PhysicalPosition::new(
                    rect.left + (rect.width - overlay_width) / 2,
                    rect.client_top + GAME_TOP_MARGIN_PX,
                ),
                // Centrée sur les DEUX axes (2026-09-08, §9 du plan) — « au centre de l'écran de
                // l'utilisateur au niveau du jeu », contrairement à Combat/Suivi qui restent
                // ancrés sur un bord.
                OverlayKind::Options => PhysicalPosition::new(
                    rect.left + (rect.width - overlay_width) / 2,
                    rect.top + (rect.height - overlay_height) / 2,
                ),
            }
        }

        fn create_overlay_window(
            event_loop: &ActiveEventLoop,
            kind: OverlayKind,
            game_window: u32,
            character_name: String,
            rect: GameRect,
            interactive: bool,
        ) -> OverlayWindow {
            let size = match kind {
                OverlayKind::Combat => WINDOW_SIZE,
                OverlayKind::Watchlist => (
                    watchlist_target_width(0, false, rect.width),
                    watchlist_target_height(false),
                ),
                OverlayKind::Options => (
                    options_modal::WINDOW_SIZE.0 as f64,
                    options_modal::WINDOW_SIZE.1 as f64,
                ),
            };
            let title_suffix = match kind {
                OverlayKind::Combat => "Combat",
                OverlayKind::Watchlist => "Suivi",
                OverlayKind::Options => "Options",
            };
            let attrs = WindowAttributes::default()
                .with_title(format!(
                    "wakfu-companion-overlay — {character_name} — {title_suffix}"
                ))
                .with_inner_size(winit::dpi::LogicalSize::new(size.0, size.1))
                .with_transparent(true)
                .with_decorations(false)
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_resizable(false)
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

            let outer = window.outer_size();
            let position =
                Self::anchor_position(kind, rect, outer.width as i32, outer.height as i32);
            window.set_outer_position(position);

            OverlayWindow {
                window,
                gpu,
                kind,
                portraits,
                combat_frame,
                icons,
                remote_icon_textures: RemoteIconTextures::default(),
                combat_side: CombatSide::default(),
                // Renseigné juste après par l'appelant (`open_options_modal`) pour `kind ==
                // Options` — `None` ici pour Combat/Suivi, jamais consulté (voir
                // `RenderContent::options`).
                options_state: (kind == OverlayKind::Options).then(OptionsModalState::default),
                game_window,
                game_rect: rect,
                character_name,
                last_position: Some(position),
                last_watchlist_width: (kind == OverlayKind::Watchlist).then_some(size.0),
                last_watchlist_height: (kind == OverlayKind::Watchlist).then_some(size.1),
                // `WindowLevel::AlwaysOnTop` déjà appliqué ci-dessus à la création — voir la doc
                // de `topmost::decide` pour la suite de la politique.
                topmost_state: TopmostState::Above {
                    pending_demote_since: None,
                },
                next_redraw_at: None,
            }
        }

        fn reposition(overlay: &mut OverlayWindow, rect: GameRect) {
            overlay.game_rect = rect;
            let outer = overlay.window.outer_size();
            let desired =
                Self::anchor_position(overlay.kind, rect, outer.width as i32, outer.height as i32);
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

        fn toggle_interactive(&mut self) {
            self.interactive = !self.interactive;
            for overlay in self.windows.values() {
                if let Err(err) = overlay.window.set_cursor_hittest(self.interactive) {
                    tracing::warn!("set_cursor_hittest a échoué : {err}");
                }
                overlay.window.request_redraw();
            }
            tracing::info!(
                ">>> Bascule ({HOTKEY_LABEL}) : mode = {}",
                if self.interactive {
                    "INTERACTIF"
                } else {
                    "CLIC-TRAVERSANT"
                }
            );
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
                let this_relevant = active == Some(overlay.game_window)
                    || active == Some(Self::xid_of(&overlay.window));
                if this_relevant && !relevant_game_windows.contains(&overlay.game_window) {
                    relevant_game_windows.push(overlay.game_window);
                }
            }

            for overlay in self.windows.values_mut() {
                // La modale Options (2026-09-08) reste `AlwaysOnTop` tout du long, posé une seule
                // fois à sa création (voir `create_overlay_window`) — jamais concernée par le
                // suivi de focus PAR PERSONNAGE ci-dessus (voir la doc de
                // `OverlayWindow::game_window`), sans quoi elle serait démotée après le délai de
                // grâce faute de `game_window` correspondant à une vraie fenêtre de jeu.
                if overlay.kind == OverlayKind::Options {
                    continue;
                }
                let relevant = relevant_game_windows.contains(&overlay.game_window);
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

        /// Ouvre la modale Options (2026-09-08, §9 du plan) — bouton "Options" du carré de
        /// contrôle (`anchor_rect` = `game_rect` de la fenêtre Suivi cliquée) ou raccourci global
        /// `OPTIONS_HOTKEY_LABEL` (`anchor_rect` = celui de la première fenêtre de jeu connue, s'il
        /// y en a une). Sans effet si une modale est déjà ouverte (une seule à la fois, comme un
        /// vrai dialogue modal) — pas de file d'attente, l'utilisateur referme/valide l'existante
        /// avant d'en rouvrir une.
        fn open_options_modal(
            &mut self,
            event_loop: &ActiveEventLoop,
            anchor_rect: Option<GameRect>,
        ) {
            if self
                .windows
                .values()
                .any(|w| w.kind == OverlayKind::Options)
            {
                return;
            }
            let rect = anchor_rect
                .or_else(|| self.windows.values().next().map(|w| w.game_rect))
                .unwrap_or(GameRect {
                    left: 0,
                    top: 0,
                    width: 1280,
                    height: 720,
                    client_top: 0,
                });
            // Toujours interactive (jamais clic-traversant), quel que soit `self.interactive` —
            // c'est une modale qui doit capter le clavier/la souris pour éditer le chemin, pas un
            // overlay passif d'information comme Combat/Suivi.
            let mut overlay = Self::create_overlay_window(
                event_loop,
                OverlayKind::Options,
                0,
                "Options".to_string(),
                rect,
                true,
            );
            overlay.options_state = Some(OptionsModalState {
                path_input: self.log_path.display().to_string(),
                error: None,
            });
            overlay.window.request_redraw();
            self.windows.insert(overlay.window.id(), overlay);
            tracing::info!("[options] modale ouverte.");
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
                    // `App::validate_and_apply_log_path`/`discovery::validate_log_path`, jamais
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
        fn validate_and_apply_log_path(&mut self, options_window_id: WindowId, raw: String) {
            let candidate = PathBuf::from(raw.trim());
            match discovery::validate_log_path(&candidate) {
                Ok(()) => {
                    tracing::info!(
                        "[options] nouveau fichier de log validé : {}",
                        candidate.display()
                    );
                    self.log_path = candidate.clone();
                    config::save(&config::OverlayConfig {
                        log_path: Some(candidate.clone()),
                    });
                    let _ = self
                        .settings_tx
                        .send(EngineCommand::ChangeLogPath(candidate));
                    self.windows.remove(&options_window_id);
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
            self.sync_windows(event_loop);
            if !self.banner_printed {
                tracing::info!("=== wakfu-companion-overlay (Linux/X11, §17.2 du plan) ===");
                tracing::info!("Suivi de {}", self.log_path.display());
                tracing::info!(
                    "Mode invité uniquement (pas de compte lié, voir la doc de ce binaire). \
                     {HOTKEY_LABEL} pour basculer interactif / clic-traversant. \
                     {QUIT_HOTKEY_LABEL} ou Ctrl+C (dans ce terminal) pour quitter."
                );
                self.banner_printed = true;
            }
        }

        fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
            match event {
                UserEvent::NewSnapshot | UserEvent::AuthStatusChanged => {
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
                OpenOptions(GameRect),
                CloseOptions,
                BrowseOptions,
                ValidateOptions(String),
            }
            let mut post_redraw = PostRedraw::None;

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
                    logging::log_session_end("fermeture de fenêtre");
                    event_loop.exit();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    // Les fenêtres overlay ne demandent jamais le focus clavier en pratique (elles
                    // restent `AlwaysOnTop` sans jamais voler l'entrée au jeu), mais Échap reste
                    // câblé par prudence si jamais l'une d'elles l'obtenait malgré tout — même
                    // filet que Windows (`main.rs`, jamais atteint non plus en pratique).
                    if event.state == ElementState::Pressed
                        && event.physical_key == PhysicalKey::Code(KeyCode::Escape)
                    {
                        event_loop.exit();
                    }
                }
                WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                    let max_dim = overlay.gpu.device.limits().max_texture_dimension_2d;
                    overlay.gpu.config.width = size.width.min(max_dim);
                    overlay.gpu.config.height = size.height.min(max_dim);
                    overlay
                        .gpu
                        .surface
                        .configure(&overlay.gpu.device, &overlay.gpu.config);
                }
                WindowEvent::RedrawRequested => {
                    let snapshot = self.snapshot.load();
                    let fight = snapshot.fight_for_character(&overlay.character_name);
                    let watchlist = self.watchlist.load_full();
                    let watchlist_toast = self.watchlist_toast.load_full();
                    let watchlist_toast = watchlist_toast.as_ref().as_ref();
                    let now = std::time::Instant::now();

                    if overlay.kind == OverlayKind::Watchlist {
                        let toast_active = watchlist_toast.is_some();
                        let target_width = watchlist_target_width(
                            watchlist.len(),
                            toast_active,
                            overlay.game_rect.width,
                        );
                        let target_height = watchlist_target_height(toast_active);
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
                    // La modale Options force sa propre interactivité (voir
                    // `App::open_options_modal`) — jamais assujettie à `self.interactive` (mode
                    // clic-traversant global de Combat/Suivi), sans quoi elle deviendrait
                    // elle-même traversable si l'utilisateur avait basculé ce mode juste avant.
                    let interactive = overlay.kind == OverlayKind::Options || self.interactive;
                    let this_game_rect = overlay.game_rect;
                    let (repaint_delay, outcome) = render(
                        &mut overlay.gpu,
                        &overlay.window,
                        RenderContent {
                            kind: overlay.kind,
                            fight,
                            portraits: &overlay.portraits,
                            combat_frame: &overlay.combat_frame,
                            icons: &overlay.icons,
                            combat_side: &mut overlay.combat_side,
                            watchlist: &watchlist,
                            watchlist_toast,
                            catalog: &catalog,
                            catalog_stale: false,
                            remote_icons: &self.remote_icons,
                            remote_icon_textures: &mut overlay.remote_icon_textures,
                            auth_status: &self.auth_status,
                            auth_command_tx: &self.auth_command_tx,
                            interactive,
                            now,
                            options: overlay.options_state.as_mut(),
                        },
                    );
                    if outcome.close_toast {
                        self.watchlist_toast.store(Arc::new(None));
                    }
                    if outcome.open_options {
                        post_redraw = PostRedraw::OpenOptions(this_game_rect);
                    }
                    match outcome.options_action {
                        OptionsModalAction::None => {}
                        OptionsModalAction::Cancel => post_redraw = PostRedraw::CloseOptions,
                        OptionsModalAction::Browse => post_redraw = PostRedraw::BrowseOptions,
                        OptionsModalAction::Validate(raw) => {
                            post_redraw = PostRedraw::ValidateOptions(raw)
                        }
                    }
                    overlay.next_redraw_at = (repaint_delay < std::time::Duration::from_secs(3600))
                        .then(|| std::time::Instant::now() + repaint_delay);
                }
                _ => {}
            }

            match post_redraw {
                PostRedraw::None => {}
                PostRedraw::OpenOptions(rect) => self.open_options_modal(event_loop, Some(rect)),
                PostRedraw::CloseOptions => {
                    self.windows.remove(&id);
                    tracing::info!("[options] modale fermée (Annuler).");
                }
                PostRedraw::BrowseOptions => self.start_file_dialog(),
                PostRedraw::ValidateOptions(raw) => self.validate_and_apply_log_path(id, raw),
            }
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            // Bug réel trouvé côté X11 (spike S3, 2026-09-03, voir son README « Bugs réels
            // trouvés » n°3) : `global-hotkey` (XGrabKey) remonte DEUX événements par pression
            // (`Pressed` ET `Released`), contrairement à `WM_HOTKEY` sous Windows. Filtré sur
            // `Pressed` uniquement dès l'écriture de ce binaire — jamais reproduit ici comme un
            // "nouveau" bug, le correctif est appliqué avant même le premier lancement.
            while let Ok(event) = self.hotkey_events.try_recv() {
                if event.state != global_hotkey::HotKeyState::Pressed {
                    continue;
                }
                if event.id == self.toggle_hotkey_id {
                    self.toggle_interactive();
                } else if event.id == self.quit_hotkey_id {
                    logging::log_session_end(QUIT_HOTKEY_LABEL);
                    event_loop.exit();
                } else if event.id == self.options_hotkey_id {
                    tracing::info!(">>> Options ({OPTIONS_HOTKEY_LABEL})");
                    self.open_options_modal(event_loop, None);
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
                            // Même garde que "Valider" (voir `validate_and_apply_log_path`) :
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

            self.sync_windows(event_loop);
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
                label: Some("overlay-ui-x11-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
                ..Default::default()
            })
            .await
            .expect("création du device");

        let size = window.inner_size();
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
            width: size.width.max(1),
            height: size.height.max(1),
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
    fn resolve_path(config: &config::OverlayConfig) -> PathBuf {
        let cli_arg = env::args().nth(1).map(PathBuf::from);
        match config::resolve_log_path(cli_arg, config) {
            Some(path) => path,
            None => {
                tracing::error!("wakfu.log introuvable aux emplacements connus. Chemins essayés :");
                for candidate in discovery::candidate_paths() {
                    tracing::error!("  - {}", candidate.display());
                }
                tracing::error!(
                    "Précisez le chemin explicitement : cargo run -p overlay-ui --bin overlay-ui-x11 -- <chemin>"
                );
                logging::log_session_end("échec de démarrage (wakfu.log introuvable)");
                std::process::exit(1);
            }
        }
    }

    pub fn run() {
        let log_dir = logging::init();
        logging::install_ctrlc_handler();
        if let Some(dir) = &log_dir {
            tracing::info!("journal de session : {}", dir.display());
        }

        let saved_config = config::load();
        let log_path = resolve_path(&saved_config);
        let snapshot = Arc::new(ArcSwap::from_pointee(SessionSnapshot::default()));
        let watchlist = Arc::new(ArcSwap::from_pointee(Vec::<WatchlistEntry>::new()));
        let watchlist_toast = Arc::new(ArcSwap::from_pointee(None::<WatchlistToast>));
        let catalog = Arc::new(ArcSwap::from_pointee(CatalogIndex::default()));
        let dungeons = Arc::new(ArcSwap::from_pointee(DungeonIndex::default()));

        let game_window = GameWindowTracker::connect()
            .expect("connexion X11 pour la découverte de fenêtres ($DISPLAY défini ?)");

        let event_loop = EventLoop::<UserEvent>::with_user_event()
            .with_x11()
            .build()
            .expect("création de l'event loop");
        let proxy = event_loop.create_proxy();
        // Mode invité fixe (voir la doc de module) pour `ApplySettings`/`Disconnect` : rien ne les
        // envoie jamais ici (aucun thread Auth). `ChangeLogPath` (2026-09-08, §9 du plan) est en
        // revanche bien émis par ce binaire — voir `App::validate_and_apply_log_path` — d'où
        // `settings_tx` conservé (plus de `_`) plutôt qu'abandonné comme avant ce lot.
        let (settings_tx, settings_rx) = mpsc::channel();
        let (sync_tx, _sync_rx) = mpsc::channel::<SyncCommand>();
        let remote_icons = RemoteIconStore::empty();
        spawn_engine_thread(
            log_path.clone(),
            EngineHandles {
                snapshot: Arc::clone(&snapshot),
                watchlist: Arc::clone(&watchlist),
                watchlist_toast: Arc::clone(&watchlist_toast),
                catalog: Arc::clone(&catalog),
                dungeons,
            },
            proxy,
            settings_rx,
            sync_tx,
        );

        event_loop.set_control_flow(ControlFlow::Wait);
        let mut app = App::new(AppState {
            log_path,
            snapshot,
            watchlist,
            watchlist_toast,
            catalog,
            remote_icons,
            game_window,
            settings_tx,
        });
        event_loop.run_app(&mut app).expect("boucle d'événements");
    }
}
