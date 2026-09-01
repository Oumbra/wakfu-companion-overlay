//! `overlay-ui` (docs/plan-architecture.md §6 et §9, lot L2) — overlay natif : fenêtres
//! transparentes Windows (DirectComposition, voir spike S1) rendant deux panneaux réels alimentés
//! par `overlay-ingest` + `overlay-engine` sur un vrai `wakfu.log` : dégâts du combat en cours,
//! récap de session. Le fenêtrage/rendu reprend telle quelle l'approche validée par S1
//! (`spikes/s1-window-windows/README.md`) — mêmes bugs déjà corrigés (patch `wgpu-hal`, alpha
//! prémultiplié, clamp de redimensionnement), pas réinventée ici.
//!
//! **Multi-fenêtre** (2026-09-01, retour utilisateur en test réel multi-compte) : `wakfu.log` est
//! partagé et entrelacé par toutes les instances du client (contrairement à Dofus, un fichier par
//! instance) — un seul `Engine`/thread suffit à le suivre (voir `spawn_engine_thread`), mais il
//! faut désormais **une fenêtre overlay par fenêtre de jeu trouvée**, chacune affichant le combat
//! de SON personnage (`overlay_engine::SessionSnapshot::fight_for_character`, le nom venant du
//! titre de fenêtre `"<Personnage> - WAKFU"`). Les fenêtres sont créées/détruites dynamiquement à
//! chaque tick (`App::sync_windows`) au gré des clients qui se lancent/se ferment — voir
//! `OverlayWindow` et le commentaire sur `Arc<Window>` (remplace la fuite `'static` d'origine,
//! plus tenable dès que des fenêtres doivent pouvoir être détruites).
//!
//! Volontairement incomplet par rapport à §9 du plan : pas encore de panneau Suivi/Alertes/État de
//! synchro (ceux-là dépendent soit de `StatsStoreService` non porté — §14 point 3 — soit de la
//! synchro serveur, L5), pas de disposition persistée par écran, pas de thème configurable. Le
//! récap de session reste également **global** (identique sur toutes les fenêtres, pas ventilé par
//! personnage — limitation connue, voir le plan) : ce sont les deux panneaux atteignables avec
//! `overlay-engine` tel qu'il existe aujourd'hui.

mod game_window;
mod panels;
mod portraits;

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use arc_swap::ArcSwap;
use crossbeam_channel::RecvTimeoutError;
use egui_wgpu::wgpu;
use game_window::{GameRect, GameWindowTracker};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use overlay_engine::{Engine, FightSnapshot, RosterIndex, SessionSnapshot};
use overlay_ingest::discovery;
use portraits::PortraitAtlas;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE,
    HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

#[cfg(target_os = "windows")]
use winit::platform::windows::WindowAttributesExtWindows;

const HOTKEY_LABEL: &str = "Ctrl+Alt+W";
// Largeur élargie 360 -> 420 (2026-09-01) pour laisser la place au portrait de classe (40px,
// voir portraits.rs) sans écraser le nom/les dégâts — réglage fin de la mise en page toujours à
// faire.
const WINDOW_SIZE: (f64, f64) = (420.0, 480.0);
/// Marge, en pixels physiques, entre le bord gauche visible de la fenêtre de jeu et le bord
/// gauche de l'overlay — « collé à quelques pixels près » (demande utilisateur). À ajuster après
/// avoir vu le rendu en pratique.
const GAME_EDGE_MARGIN_PX: i32 = 12;

/// Émis par le thread Engine (§3 du plan) quand un nouveau `SessionSnapshot` est disponible —
/// réveille le main thread, en `ControlFlow::Wait` le reste du temps (§6.1 : pas de boucle 60 Hz
/// forcée, l'overlay ne consomme rien tant que rien ne change).
enum UserEvent {
    NewSnapshot,
}

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    egui_ctx: egui::Context,
    egui_winit: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

/// Une fenêtre overlay, ancrée sur UNE fenêtre de jeu précise — un personnage, un combat. Créée et
/// détruite dynamiquement par `App::sync_windows` au gré des clients qui se lancent/se ferment.
struct OverlayWindow {
    /// `Arc`, pas `&'static` : contrairement à la version mono-fenêtre d'origine (`Box::leak`),
    /// une fenêtre doit pouvoir être réellement détruite quand son client de jeu ferme — l'`Arc`
    /// est le motif standard wgpu+winit pour des fenêtres à durée de vie dynamique
    /// (`wgpu::Instance::create_surface` accepte `Arc<Window>`, qui garde la fenêtre en vie aussi
    /// longtemps que la `Surface`, donnant un `Surface<'static>` sans fuite).
    window: Arc<Window>,
    gpu: GpuState,
    /// Chargée par fenêtre (chacune a son propre `egui::Context`) — léger surcoût de
    /// décodage/upload par fenêtre, négligeable pour le nombre de comptes réaliste.
    portraits: PortraitAtlas,
    game_hwnd: HWND,
    character_name: String,
    /// Dernière position appliquée — évite de rappeler `set_outer_position` à chaque tick (50 ms)
    /// quand la fenêtre de jeu n'a pas bougé.
    last_position: Option<PhysicalPosition<i32>>,
    /// État `HWND_TOPMOST`/`HWND_NOTOPMOST` déjà appliqué — évite un `SetWindowPos` par tick pour
    /// rien (voir `App::sync_topmost`).
    is_topmost: bool,
}

struct App {
    windows: HashMap<WindowId, OverlayWindow>,
    #[allow(dead_code)] // jamais relu : sa seule raison d'être est de rester en vie (voir S1)
    hotkey_manager: GlobalHotKeyManager,
    hotkey_events: &'static global_hotkey::GlobalHotKeyEventReceiver,
    interactive: bool,
    snapshot: Arc<ArcSwap<SessionSnapshot>>,
    log_path: PathBuf,
    game_window: GameWindowTracker,
    /// N'affiche la bannière de démarrage qu'une fois — `resumed()` peut être rappelé par winit
    /// (perte/reprise de focus applicatif), `sync_windows` doit rester idempotent mais pas cette
    /// bannière.
    banner_printed: bool,
}

impl App {
    fn new(log_path: PathBuf, snapshot: Arc<ArcSwap<SessionSnapshot>>) -> Self {
        let hotkey_manager = GlobalHotKeyManager::new().expect("création GlobalHotKeyManager");
        let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW);
        hotkey_manager
            .register(hotkey)
            .expect("enregistrement du hotkey global");

        Self {
            windows: HashMap::new(),
            hotkey_manager,
            hotkey_events: GlobalHotKeyEvent::receiver(),
            interactive: true,
            snapshot,
            log_path,
            game_window: GameWindowTracker::new(),
            banner_printed: false,
        }
    }

    /// Scanne les fenêtres de jeu actuellement ouvertes et fait converger `self.windows` vers cet
    /// état : retire les overlays dont le client a fermé, crée un overlay pour chaque nouvelle
    /// fenêtre de jeu trouvée, repositionne les autres. Appelé au premier `resumed()` et à chaque
    /// tick d'`about_to_wait` (comme l'ancien `track_game_window` mono-fenêtre) — idempotent,
    /// rappelable sans risque.
    fn sync_windows(&mut self, event_loop: &ActiveEventLoop) {
        let found = self.game_window.scan();

        self.windows.retain(|_, overlay| {
            let still_here = found.iter().any(|(_, info)| info.hwnd == overlay.game_hwnd);
            if !still_here {
                println!(
                    "[fenêtre de jeu] {} fermée — son overlay est retiré.",
                    overlay.character_name
                );
            }
            still_here
        });

        for (character_name, info) in &found {
            if let Some(existing) = self.windows.values_mut().find(|w| w.game_hwnd == info.hwnd) {
                Self::reposition(existing, info.rect);
                continue;
            }
            let overlay = Self::create_overlay_window(
                event_loop,
                info.hwnd,
                character_name.clone(),
                info.rect,
                self.interactive,
            );
            println!("[fenêtre de jeu] {character_name} trouvée — overlay créé.");
            self.windows.insert(overlay.window.id(), overlay);
        }
    }

    fn create_overlay_window(
        event_loop: &ActiveEventLoop,
        game_hwnd: HWND,
        character_name: String,
        rect: GameRect,
        interactive: bool,
    ) -> OverlayWindow {
        let attrs = WindowAttributes::default()
            .with_title(format!("wakfu-companion-overlay — {character_name}"))
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_SIZE.0, WINDOW_SIZE.1))
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false);
        #[cfg(target_os = "windows")]
        let attrs = attrs
            .with_skip_taskbar(true)
            .with_no_redirection_bitmap(true);

        let window = event_loop
            .create_window(attrs)
            .expect("création de la fenêtre overlay");
        let window = Arc::new(window);

        let hwnd = Self::hwnd_of(&window);
        Self::apply_extended_styles(hwnd);
        if let Err(err) = window.set_cursor_hittest(interactive) {
            eprintln!("set_cursor_hittest a échoué à la création : {err}");
        }

        let gpu = pollster::block_on(init_gpu(Arc::clone(&window)));
        let portraits = PortraitAtlas::load(&gpu.egui_ctx);

        let overlay_height = window.outer_size().height as i32;
        let position = PhysicalPosition::new(
            rect.left + GAME_EDGE_MARGIN_PX,
            rect.top + (rect.height - overlay_height) / 2,
        );
        window.set_outer_position(position);

        OverlayWindow {
            window,
            gpu,
            portraits,
            game_hwnd,
            character_name,
            last_position: Some(position),
            is_topmost: true, // WindowLevel::AlwaysOnTop déjà appliqué ci-dessus à la création
        }
    }

    /// Recolle une fenêtre overlay au bord gauche de sa fenêtre de jeu, verticalement centrée
    /// (demande utilisateur) ; n'appelle `set_outer_position` que si la position cible a changé,
    /// pour ne pas spammer le compositeur DWM 20×/s pour rien.
    fn reposition(overlay: &mut OverlayWindow, rect: GameRect) {
        let overlay_height = overlay.window.outer_size().height as i32;
        let desired = PhysicalPosition::new(
            rect.left + GAME_EDGE_MARGIN_PX,
            rect.top + (rect.height - overlay_height) / 2,
        );
        if overlay.last_position != Some(desired) {
            overlay.window.set_outer_position(desired);
            overlay.last_position = Some(desired);
        }
    }

    /// `WS_EX_NOACTIVATE`/`WS_EX_TOOLWINDOW`, non exposés par `winit` — voir S1 (§6.2 du plan).
    fn apply_extended_styles(hwnd: HWND) {
        unsafe {
            let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let new_style = current | (WS_EX_NOACTIVATE.0 as isize) | (WS_EX_TOOLWINDOW.0 as isize);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
        }
    }

    fn hwnd_of(window: &Window) -> HWND {
        match window.window_handle().expect("handle de fenêtre").as_raw() {
            RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as *mut _),
            other => panic!("handle de fenêtre inattendu sur Windows : {other:?}"),
        }
    }

    fn toggle_interactive(&mut self) {
        self.interactive = !self.interactive;
        for overlay in self.windows.values() {
            if let Err(err) = overlay.window.set_cursor_hittest(self.interactive) {
                eprintln!("set_cursor_hittest a échoué : {err}");
            }
            overlay.window.request_redraw();
        }
        println!(
            ">>> Bascule ({HOTKEY_LABEL}) : mode = {}",
            if self.interactive {
                "INTERACTIF"
            } else {
                "CLIC-TRAVERSANT"
            }
        );
    }

    /// Au-dessus tant qu'une fenêtre pertinente (n'importe laquelle des fenêtres de jeu suivies,
    /// OU n'importe lequel des overlays lui-même) a le focus ; sinon repli en z-order normal, pour
    /// ne plus recouvrir une application quelconque devenue active (retour utilisateur,
    /// 2026-09-01 : "l'overlay ne doit pas s'afficher par-dessus l'explorateur de fichiers"). Un
    /// seul `GetForegroundWindow()` par tick, comparé aux `HWND` déjà connus — coût négligeable.
    ///
    /// Politique volontairement simplifiée : TOUTE fenêtre de jeu Wakfu (pas seulement celle du
    /// personnage actif) remet TOUS les overlays au premier plan, pas de logique par-personnage —
    /// à affiner si un besoin réel l'impose en usage.
    fn sync_topmost(&mut self) {
        let foreground = unsafe { GetForegroundWindow() };
        let relevant = self
            .windows
            .values()
            .any(|w| w.game_hwnd == foreground || Self::hwnd_of(&w.window) == foreground);

        for overlay in self.windows.values_mut() {
            if overlay.is_topmost == relevant {
                continue;
            }
            let insert_after = if relevant {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            };
            let hwnd = Self::hwnd_of(&overlay.window);
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    Some(insert_after),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            overlay.is_topmost = relevant;
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.sync_windows(event_loop);
        if !self.banner_printed {
            println!("=== wakfu-companion-overlay (L2, overlay-ui) ===");
            println!("Suivi de {}", self.log_path.display());
            println!(
                "{HOTKEY_LABEL} pour basculer interactif / clic-traversant. Échap/Ctrl+C pour quitter.\n"
            );
            self.banner_printed = true;
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::NewSnapshot => {
                for overlay in self.windows.values() {
                    overlay.window.request_redraw();
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(overlay) = self.windows.get_mut(&id) else {
            return; // événement d'une fenêtre déjà retirée (client fermé entre-temps) — ignoré
        };

        let response = overlay
            .gpu
            .egui_winit
            .on_window_event(&overlay.window, &event);
        if response.repaint {
            overlay.window.request_redraw();
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && event.physical_key == PhysicalKey::Code(KeyCode::Escape)
                {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                // Clamp défensif (voir S1, README.md §"Bug de redimensionnement") : un resize
                // excessif ne doit jamais faire planter l'overlay, quelle qu'en soit la cause.
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
                render(
                    &mut overlay.gpu,
                    &overlay.window,
                    self.interactive,
                    &snapshot,
                    fight,
                    &overlay.character_name,
                    &overlay.portraits,
                );
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Hotkey global : thread OS dédié, sondé ici sans bloquer (voir S1).
        if self.hotkey_events.try_recv().is_ok() {
            self.toggle_interactive();
        }
        // Découverte/suivi des fenêtres de jeu : même sondage périodique que le hotkey (pas d'API
        // Win32 pour être notifié d'un déplacement/redimensionnement/apparition d'une fenêtre qui
        // n'est pas la nôtre sans un hook global — un sondage à 20 Hz est largement assez réactif
        // ici et reste négligeable en coût, voir game_window.rs).
        self.sync_windows(event_loop);
        self.sync_topmost();
        // Réactif (§6.1) : on attend soit un événement fenêtre, soit un `UserEvent::NewSnapshot`
        // du thread Engine, jamais de boucle 60 Hz forcée. Le sondage hotkey/fenêtre de jeu
        // ci-dessus impose quand même un réveil périodique court, sans quoi ni l'un ni l'autre ne
        // seraient vus qu'au prochain événement fenêtre.
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(50),
        ));
    }
}

async fn init_gpu(window: Arc<Window>) -> GpuState {
    // Voir spikes/s1-window-windows/README.md pour le détail complet de ce qui suit — bugs
    // wgpu-hal corrigés par le patch vendored, choix d'alpha prémultiplié, etc.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        backend_options: wgpu::BackendOptions {
            dx12: wgpu::Dx12BackendOptions {
                presentation_system: wgpu::Dx12SwapchainKind::DxgiFromVisual,
                ..Default::default()
            },
            ..Default::default()
        },
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });

    // `Arc<Window>` donne un `Surface<'static>` sans fuite (la surface garde l'`Arc` en interne,
    // la fenêtre reste vivante aussi longtemps qu'elle) — voir la doc d'`OverlayWindow::window`.
    let surface = instance
        .create_surface(Arc::clone(&window))
        .expect("création de la surface (DirectComposition visual)");

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            ..Default::default()
        })
        .await
        .expect("aucun adaptateur DX12 compatible");
    println!("Adaptateur GPU : {:?}", adapter.get_info());

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("overlay-ui-device"),
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
    }
}

fn render(
    gpu: &mut GpuState,
    window: &Window,
    interactive: bool,
    snapshot: &SessionSnapshot,
    fight: Option<&FightSnapshot>,
    character_name: &str,
    portraits: &PortraitAtlas,
) {
    let raw_input = gpu.egui_winit.take_egui_input(window);
    let mut full_output = gpu.egui_ctx.run_ui(raw_input, |ui| {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(18, 20, 28, 215)),
            )
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    // Repère visuel du rapprochement fenêtre↔personnage (multi-compte,
                    // 2026-09-01) : le nom affiché DOIT correspondre à la fenêtre de jeu sur
                    // laquelle cet overlay est collé — vérifiable d'un coup d'œil en test réel.
                    ui.heading(character_name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.small(if interactive {
                            "interactif"
                        } else {
                            "clic-traversant"
                        });
                    });
                });
                ui.small(format!("{HOTKEY_LABEL} pour basculer"));
                ui.separator();

                panels::combat::show(ui, fight, portraits);

                ui.add_space(8.0);
                ui.separator();
                ui.strong("Récap de session");
                let t = &snapshot.totals;
                egui::Grid::new("recap-grid")
                    .num_columns(2)
                    .spacing([12.0, 2.0])
                    .show(ui, |ui| {
                        ui.label("Kamas");
                        ui.monospace(format!("{:+}", t.kamas_gained - t.kamas_lost));
                        ui.end_row();
                        ui.label("XP gagnée");
                        ui.monospace(t.xp_gained.to_string());
                        ui.end_row();
                        ui.label("Combats");
                        ui.monospace(format!(
                            "{} gagnés / {} perdus",
                            t.fights_won, t.fights_lost
                        ));
                        ui.end_row();
                        ui.label("Butin ramassé");
                        ui.monospace(t.loot_count.to_string());
                        ui.end_row();
                    });

                if let Some(last) = snapshot.recent_loot.last() {
                    ui.small(format!(
                        "Dernier butin : {} ×{} ({})",
                        last.item, last.quantity, last.time
                    ));
                }
            });
    });
    gpu.egui_winit
        .handle_platform_output(window, full_output.platform_output);

    let paint_jobs = gpu
        .egui_ctx
        .tessellate(full_output.shapes, full_output.pixels_per_point);

    // Traité inconditionnellement, *avant* toute sortie anticipée ci-dessous : `textures_delta`
    // doit être appliqué (set) puis libéré (free), et explicitement vidé (`clear`) ensuite, quel
    // que soit le sort de cette frame — `TexturesDelta` panique (`debug_assert!`, donc invisible
    // en `--release`, ce qui l'a caché jusqu'ici) si ses collections ne sont pas vides à sa
    // destruction, et les itérer par référence ne les vide pas.
    for (id, deltas) in &full_output.textures_delta.set {
        for delta in deltas {
            gpu.egui_renderer
                .update_texture(&gpu.device, &gpu.queue, *id, delta);
        }
    }
    for id in &full_output.textures_delta.free {
        gpu.egui_renderer.free_texture(id);
    }
    full_output.textures_delta.clear();

    let output_frame = match gpu.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(frame) => frame,
        wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
        wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return,
        wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
            gpu.surface.configure(&gpu.device, &gpu.config);
            return;
        }
        wgpu::CurrentSurfaceTexture::Validation => {
            eprintln!("get_current_texture: erreur de validation");
            return;
        }
    };
    let view = output_frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let screen_descriptor = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [gpu.config.width, gpu.config.height],
        pixels_per_point: full_output.pixels_per_point,
    };

    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("overlay-ui-encoder"),
        });
    gpu.egui_renderer.update_buffers(
        &gpu.device,
        &gpu.queue,
        &mut encoder,
        &paint_jobs,
        &screen_descriptor,
    );

    {
        let mut render_pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("overlay-ui-egui-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                ..Default::default()
            })
            .forget_lifetime();
        gpu.egui_renderer
            .render(&mut render_pass, &paint_jobs, &screen_descriptor);
    }

    gpu.queue.submit(Some(encoder.finish()));
    gpu.queue.present(output_frame);
}

/// Thread Engine (§3 du plan) : lit `wakfu.log` en continu, alimente `overlay-engine`, publie
/// chaque nouveau `SessionSnapshot` par `ArcSwap` et réveille le main thread. Ne rappelle jamais
/// l'UI directement — l'UI ne lit que la dernière valeur publiée (§3 : « zéro verrou sur le
/// chemin de rendu »). Un seul thread/`Engine` pour toutes les fenêtres overlay (voir doc de
/// module) : `wakfu.log` est partagé par tous les clients, `SessionSnapshot::fights` porte déjà
/// tous les combats simultanés.
///
/// Reçoit aussi (lot L4) le roster récupéré par le thread Auth (`spawn_auth_thread`) via
/// `roster_rx` — appliqué à `Engine` de façon non bloquante entre deux lots, jamais en attendant
/// activement dessus (voir la sélection `recv_timeout` ci-dessous, seule façon de sonder les DEUX
/// canaux — lignes de log et roster — sans thread de sondage dédié).
fn spawn_engine_thread(
    log_path: PathBuf,
    snapshot: Arc<ArcSwap<SessionSnapshot>>,
    proxy: EventLoopProxy<UserEvent>,
    roster_rx: mpsc::Receiver<RosterIndex>,
) {
    thread::Builder::new()
        .name("overlay-engine".into())
        .spawn(move || {
            let mut engine = match Engine::new() {
                Ok(engine) => engine,
                Err(err) => {
                    eprintln!("[erreur fatale] création de l'Engine QuickJS : {err}");
                    return;
                }
            };
            let rx = overlay_ingest::watcher::spawn(&log_path);
            loop {
                // Non bloquant : n'attend jamais activement le roster, seulement les lignes de
                // log (voir recv_timeout plus bas) — un roster qui n'arrive jamais (pas de
                // compte lié) ne doit pas retarder l'ingestion d'un seul milliseconde.
                while let Ok(roster) = roster_rx.try_recv() {
                    tracing::info!("roster appliqué à l'Engine (lot L4)");
                    engine.set_roster(Some(roster));
                }
                match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                    Ok(Ok(batch)) => {
                        if let Err(err) = engine.ingest_batch(&batch) {
                            tracing::warn!(%err, "échec d'ingestion d'un lot, ligne(s) ignorée(s)");
                            continue;
                        }
                        snapshot.store(Arc::new(engine.snapshot()));
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

/// Thread Auth (lot L4, §7.2 du plan) : résout l'accès au compte AVANT de bloquer sur quoi que ce
/// soit d'autre — jamais le thread Engine ni le main thread. Best-effort et jamais fatal : sans
/// jeton stocké ni appairage complété, l'overlay continue simplement en mode invité (repli
/// `breed` déjà géré par `overlay-engine::session`), exactement comme le mode invité du web.
///
/// Pas encore d'UI de pairing dans la fenêtre overlay elle-même (hors périmètre de cette
/// itération, voir le panneau "État de synchro" du plan §9, toujours à construire) : le code
/// d'appairage est affiché en console.
fn spawn_auth_thread(roster_tx: mpsc::Sender<RosterIndex>) {
    thread::Builder::new()
        .name("overlay-auth".into())
        .spawn(move || {
            if let Some(token) = overlay_sync::token_store::load_token() {
                match overlay_sync::client::fetch_roster(&token) {
                    Ok(roster) => {
                        println!("[compte] roster récupéré depuis le jeton natif déjà connu.");
                        let _ = roster_tx.send(roster);
                        return;
                    }
                    Err(err) => {
                        println!("[compte] jeton natif invalide/expiré ({err}) — nouvel appairage nécessaire.");
                        overlay_sync::token_store::clear_token();
                    }
                }
            }

            let token = match overlay_sync::pair_and_wait(|handle| {
                println!("\n=== Connexion du compte (optionnelle) ===");
                println!(
                    "Ouvre {} et entre le code : {}",
                    handle.verification_url, handle.pairing_code
                );
                println!(
                    "(l'overlay fonctionne aussi sans compte lié — repli sur la classe détectée automatiquement)\n"
                );
            }) {
                Ok(token) => token,
                Err(err) => {
                    println!("[compte] appairage non complété ({err}) — l'overlay continue sans roster.");
                    return;
                }
            };

            if let Err(err) = overlay_sync::token_store::save_token(&token) {
                // Volontairement `eprintln!`, pas seulement `tracing::warn!` (invisible par
                // défaut ici, voir main() — aucun subscriber `tracing` installé, seulement
                // `env_logger` pour la façade `log`) : un jeton non sauvegardé fait
                // silencieusement recommencer l'appairage à chaque lancement, ça DOIT être vu.
                eprintln!("[compte] échec de sauvegarde du jeton natif ({err}) — sera redemandé au prochain lancement.");
            }
            match overlay_sync::client::fetch_roster(&token) {
                Ok(roster) => {
                    println!("[compte] connecté — roster récupéré.");
                    let _ = roster_tx.send(roster);
                }
                Err(err) => println!("[compte] échec de récupération du roster après appairage ({err})."),
            }
        })
        .expect("échec de création du thread Auth");
}

fn resolve_path() -> PathBuf {
    if let Some(arg) = env::args().nth(1) {
        return PathBuf::from(arg);
    }
    match discovery::discover() {
        Some(path) => path,
        None => {
            eprintln!("wakfu.log introuvable aux emplacements connus. Chemins essayés :");
            for candidate in discovery::candidate_paths() {
                eprintln!("  - {}", candidate.display());
            }
            eprintln!("\nPrécisez le chemin explicitement : cargo run -p overlay-ui -- <chemin>");
            std::process::exit(1);
        }
    }
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,wgpu_hal=info"),
    )
    .init();

    let log_path = resolve_path();
    let snapshot = Arc::new(ArcSwap::from_pointee(SessionSnapshot::default()));

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("création de l'event loop");
    let proxy = event_loop.create_proxy();
    let (roster_tx, roster_rx) = mpsc::channel();
    spawn_auth_thread(roster_tx);
    spawn_engine_thread(log_path.clone(), Arc::clone(&snapshot), proxy, roster_rx);

    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(log_path, snapshot);
    event_loop.run_app(&mut app).expect("boucle d'événements");
}
