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

    use arc_swap::ArcSwap;
    use egui_wgpu::wgpu;
    use global_hotkey::hotkey::{Code, HotKey, Modifiers};
    use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
    use overlay_engine::{CatalogIndex, DungeonIndex, SessionSnapshot, WatchlistEntry};
    use overlay_ingest::discovery;
    use overlay_platform::linux::topmost::{self, TopmostAction, TopmostState};
    use overlay_platform::linux::x11::{GameRect, GameWindowTracker};
    use overlay_ui::engine_thread::{spawn_engine_thread, EngineHandles, SyncCommand};
    use overlay_ui::frame::{render, GpuState};
    use overlay_ui::logging;
    use overlay_ui::panels;
    use overlay_ui::panels::combat::CombatSide;
    use overlay_ui::panels::combat_frame::CombatFrame;
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
        /// XID X11 de la fenêtre de jeu — équivalent du `hwnd` côté Windows (voir
        /// `overlay_platform::linux::x11::GameWindowInfo`).
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
        interactive: bool,
        snapshot: Arc<ArcSwap<SessionSnapshot>>,
        watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
        watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
        catalog: Arc<ArcSwap<CatalogIndex>>,
        remote_icons: RemoteIconStore,
        auth_status: AuthStatus,
        auth_command_tx: NoopAuthSink,
        log_path: PathBuf,
        game_window: GameWindowTracker,
        banner_printed: bool,
    }

    struct AppState {
        log_path: PathBuf,
        snapshot: Arc<ArcSwap<SessionSnapshot>>,
        watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
        watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
        catalog: Arc<ArcSwap<CatalogIndex>>,
        remote_icons: RemoteIconStore,
        game_window: GameWindowTracker,
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
            } = state;

            let hotkey_manager = GlobalHotKeyManager::new().expect("création GlobalHotKeyManager");
            let toggle_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyW);
            let quit_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyQ);
            hotkey_manager
                .register(toggle_hotkey)
                .expect("enregistrement du hotkey global");
            hotkey_manager
                .register(quit_hotkey)
                .expect("enregistrement du hotkey de sortie");

            Self {
                windows: HashMap::new(),
                hotkey_manager,
                hotkey_events: GlobalHotKeyEvent::receiver(),
                toggle_hotkey_id: toggle_hotkey.id(),
                quit_hotkey_id: quit_hotkey.id(),
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
                log_path,
                game_window,
                banner_printed: false,
            }
        }

        /// Même politique que `main.rs::App::sync_windows` (voir sa doc) : converge `self.windows`
        /// vers l'état actuel des fenêtres de jeu trouvées.
        fn sync_windows(&mut self, event_loop: &ActiveEventLoop) {
            let found = self.game_window.scan();

            self.windows.retain(|_, overlay| {
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
            };
            let title_suffix = match kind {
                OverlayKind::Combat => "Combat",
                OverlayKind::Watchlist => "Suivi",
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

            for overlay in self.windows.values_mut() {
                let our_xid = Self::xid_of(&overlay.window);
                let relevant = active == Some(overlay.game_window) || active == Some(our_xid);
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
                        overlay.window.set_window_level(WindowLevel::AlwaysOnTop)
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
                    let (repaint_delay, close_toast) = render(
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
                            interactive: self.interactive,
                            now,
                        },
                    );
                    if close_toast {
                        self.watchlist_toast.store(Arc::new(None));
                    }
                    overlay.next_redraw_at = (repaint_delay < std::time::Duration::from_secs(3600))
                        .then(|| std::time::Instant::now() + repaint_delay);
                }
                _ => {}
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

    fn resolve_path() -> PathBuf {
        if let Some(arg) = env::args().nth(1) {
            return PathBuf::from(arg);
        }
        match discovery::discover() {
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

        let log_path = resolve_path();
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
        // Mode invité fixe (voir la doc de module) : personne n'écoute jamais ce canal, mais
        // `spawn_engine_thread` (partagé avec Windows) en a besoin pour relayer les événements de
        // synchro qu'`Engine::drain_sync_events` produirait — jamais consommés ici, `sync_rx` est
        // juste droppé (le `send` de `spawn_engine_thread` échoue alors silencieusement, jamais
        // fatal, voir sa doc).
        let (_settings_tx, settings_rx) = mpsc::channel();
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
        });
        event_loop.run_app(&mut app).expect("boucle d'événements");
    }
}
