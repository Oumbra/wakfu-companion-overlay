//! Spike S3 (docs/plan-architecture.md §12, §17.2) — question posée :
//!
//!   « Une fenêtre transparente, toujours au-dessus, traversable par la souris, ancrée sur une
//!   fenêtre de jeu trouvée par titre, rendue avec wgpu+egui sous un rendu logiciel (lavapipe), est-
//!   elle atteignable sous X11 sans piège de composition ? »
//!
//! Comme S1 (Windows), ce binaire n'est PAS l'ébauche figée de `overlay-platform::linux::x11` —
//! c'est un harnais de validation. Contrairement à S1, ses deux modules de logique (`discovery`,
//! `topmost`, dans `s3_window_linux`, voir `lib.rs`) SONT en revanche écrits comme un portage
//! direct, migrable tel quel (voir leur doc) : le spike ici ne sert qu'à les CÂBLER à une vraie
//! fenêtre winit et à les valider en conditions réelles (Xvfb, voir README.md).
//!
//! **Simplifications assumées par rapport à la production Windows (`crates/overlay-ui`)**,
//! documentées en détail dans README.md :
//! - Une seule fenêtre overlay, ancrée sur la PREMIÈRE fenêtre de jeu trouvée — pas encore le jeu
//!   d'ensemble dynamique multi-fenêtres de `main.rs::sync_windows` (§6.5 du plan).
//! - Panneau unique de diagnostic (personnage trouvé, mode, état topmost) — pas les vrais panneaux
//!   Combat/Suivi.
//! - `Box::leak` (durée de vie `'static`, comme S1) : acceptable pour un spike à une seule fenêtre,
//!   pas pour la production (voir la doc d'`OverlayWindow::window` dans `overlay-ui`).

use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use egui_wgpu::wgpu;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use s3_window_linux::{discovery, topmost};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::x11::{EventLoopBuilderExtX11, WindowAttributesExtX11, WindowType};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

/// Rappel du hotkey affiché à l'écran et dans la console — même combinaison que le reste du projet
/// (`overlay-ui::HOTKEY_LABEL`).
const HOTKEY_LABEL: &str = "Ctrl+Alt+W";
/// Même cadence que le sondage Windows (§6.5 du plan : 20 Hz pour l'ancrage/topmost).
const POLL_INTERVAL: Duration = Duration::from_millis(50);
/// Même marge que `overlay_ui::GAME_EDGE_MARGIN_PX` (ancrage du panneau Combat : bord gauche,
/// centré verticalement).
const GAME_EDGE_MARGIN_PX: i32 = 12;

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    egui_ctx: egui::Context,
    egui_winit: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

struct App {
    window: Option<&'static Window>,
    gpu: Option<GpuState>,
    /// XID Xlib/XCB de NOTRE fenêtre (même espace de nommage, voir README.md) — comparé à
    /// `_NET_ACTIVE_WINDOW` pour `relevant` (voir `poll`).
    our_xid: Option<u32>,
    #[allow(dead_code)] // jamais relu : sa seule raison d'être est de rester en vie (voir S1)
    hotkey_manager: GlobalHotKeyManager,
    hotkey_events: &'static global_hotkey::GlobalHotKeyEventReceiver,
    interactive: bool,
    tracker: discovery::GameWindowTracker,
    topmost_state: topmost::TopmostState,
    last_position: Option<PhysicalPosition<i32>>,
    last_poll: Instant,
    /// Personnage de la première fenêtre de jeu trouvée au dernier `poll()` — `None` si aucune
    /// fenêtre `TITLE_SUFFIX` n'est visible (affiché dans le panneau, voir `render`).
    found_character: Option<String>,
}

impl App {
    fn new(tracker: discovery::GameWindowTracker) -> Self {
        let hotkey_manager = GlobalHotKeyManager::new().expect("création GlobalHotKeyManager");
        let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW);
        hotkey_manager
            .register(hotkey)
            .expect("enregistrement du hotkey global");

        Self {
            window: None,
            gpu: None,
            our_xid: None,
            hotkey_manager,
            hotkey_events: GlobalHotKeyEvent::receiver(),
            interactive: true,
            tracker,
            topmost_state: topmost::TopmostState::Above {
                pending_demote_since: None,
            },
            last_position: None,
            last_poll: Instant::now() - POLL_INTERVAL, // sonde dès le premier `about_to_wait`
            found_character: None,
        }
    }

    fn xid_of(window: &Window) -> u32 {
        match window.window_handle().expect("handle de fenêtre").as_raw() {
            // winit (rwh_06, X11) renvoie toujours `Xlib`, jamais `Xcb` — vérifié dans le code
            // source de winit 0.30.13 (`raw_window_handle_rwh_06`, voir README.md). Xlib et XCB
            // parlent au MÊME serveur X et partagent le même espace d'identifiants (XID) : ce
            // `u32` est directement comparable à un `x11rb::protocol::xproto::Window`.
            RawWindowHandle::Xlib(handle) => handle.window as u32,
            other => panic!("handle de fenêtre inattendu sous X11 : {other:?}"),
        }
    }

    fn toggle_interactive(&mut self) {
        self.interactive = !self.interactive;
        if let Some(window) = self.window {
            // `set_cursor_hittest(false)` pose une région d'entrée XShape VIDE — voir README.md
            // pour la vérification (le code source de winit confirme ce mécanisme, voir §17.2 du
            // plan) et `probe.rs` pour la vérification programmatique côté harnais.
            if let Err(err) = window.set_cursor_hittest(self.interactive) {
                eprintln!("set_cursor_hittest a échoué : {err}");
            }
            window.request_redraw();
        }
        println!(
            "\n>>> Bascule ({HOTKEY_LABEL}) : mode = {}\n",
            if self.interactive {
                "INTERACTIF"
            } else {
                "CLIC-TRAVERSANT"
            }
        );
    }

    /// Ancrage + topmost — même cadence et même logique que `overlay_ui::main::App::sync_windows`/
    /// `sync_topmost` (§6.5 du plan), mais pour UNE seule fenêtre overlay ancrée sur la PREMIÈRE
    /// fenêtre de jeu trouvée (voir la doc de module pour la simplification assumée).
    fn poll(&mut self) {
        let found = self.tracker.scan();
        let first = found.into_iter().next();
        self.found_character = first.as_ref().map(|(name, _)| name.clone());

        if let (Some(window), Some((_, info))) = (self.window, &first) {
            let outer = window.outer_size();
            let desired = PhysicalPosition::new(
                info.rect.left + GAME_EDGE_MARGIN_PX,
                info.rect.top + (info.rect.height - outer.height as i32) / 2,
            );
            if self.last_position != Some(desired) {
                window.set_outer_position(desired);
                self.last_position = Some(desired);
            }
        }

        let active = self.tracker.active_window();
        let relevant = match (active, self.our_xid, first.as_ref()) {
            (Some(active), Some(our_xid), _) if active == our_xid => true,
            (Some(active), _, Some((_, info))) => active == info.window,
            _ => false,
        };
        let (next_state, action) = topmost::decide(self.topmost_state, relevant, Instant::now());
        self.topmost_state = next_state;
        match action {
            topmost::TopmostAction::None => {}
            topmost::TopmostAction::SetAbove => {
                if let Some(window) = self.window {
                    window.set_window_level(WindowLevel::AlwaysOnTop);
                }
            }
            topmost::TopmostAction::SetNormal => {
                if let Some(window) = self.window {
                    window.set_window_level(WindowLevel::Normal);
                }
            }
        }

        if let Some(window) = self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("wakfu-companion-overlay — Spike S3")
            .with_inner_size(winit::dpi::LogicalSize::new(360.0, 200.0))
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false)
            // `_NET_WM_WINDOW_TYPE_UTILITY` (§6.4 du plan) — absent du taskbar/alt-tab, comme
            // `with_skip_taskbar` côté Windows.
            .with_x11_window_type(vec![WindowType::Utility]);

        let window = event_loop
            .create_window(attrs)
            .expect("création de la fenêtre overlay");
        // Fuite volontaire, comme S1 : une seule fenêtre, jamais recréée (voir doc de module).
        let window: &'static Window = Box::leak(Box::new(window));
        self.our_xid = Some(Self::xid_of(window));

        let gpu = pollster::block_on(init_gpu(window));
        self.window = Some(window);
        self.gpu = Some(gpu);

        println!("=== Spike S3 — fenêtre transparente X11 (lavapipe via wgpu) ===");
        println!("XID de l'overlay : {:#x}", self.our_xid.unwrap());
        println!("{HOTKEY_LABEL} pour basculer interactif / clic-traversant.");
        println!("Ctrl+C dans cette console pour arrêter.\n");

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = self.window else { return };
        if window.id() != id {
            return;
        }
        let Some(gpu) = self.gpu.as_mut() else { return };

        let response = gpu.egui_winit.on_window_event(window, &event);
        if response.repaint {
            window.request_redraw();
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
                let max_dim = gpu.device.limits().max_texture_dimension_2d;
                gpu.config.width = size.width.min(max_dim);
                gpu.config.height = size.height.min(max_dim);
                gpu.surface.configure(&gpu.device, &gpu.config);
            }
            WindowEvent::RedrawRequested => self.render(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // **Bug réel trouvé sous X11 (2026-09-03)** : contrairement à `WM_HOTKEY` sous Windows (un
        // seul message par activation), `global-hotkey` sur X11 (XGrabKey) remonte DEUX
        // événements par pression — `HotKeyState::Pressed` ET `HotKeyState::Released` — sur le
        // même canal. Un simple `try_recv().is_ok()` (ignorant l'état) faisait donc basculer
        // interactif/traversant DEUX FOIS par appui, annulant l'effet visible. Filtrer sur
        // `Pressed` uniquement ; `while let` pour drainer tout ce qui s'est accumulé depuis le
        // tour précédent, pas seulement un événement par tick.
        while let Ok(event) = self.hotkey_events.try_recv() {
            if event.state == global_hotkey::HotKeyState::Pressed {
                self.toggle_interactive();
            }
        }
        if self.last_poll.elapsed() >= POLL_INTERVAL {
            self.last_poll = Instant::now();
            self.poll();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.last_poll + POLL_INTERVAL));
    }
}

impl App {
    fn render(&mut self) {
        let interactive = self.interactive;
        let found_character = self.found_character.clone();
        let topmost_label = match self.topmost_state {
            topmost::TopmostState::Above {
                pending_demote_since: None,
            } => "AU-DESSUS",
            topmost::TopmostState::Above {
                pending_demote_since: Some(_),
            } => "AU-DESSUS (repli en attente)",
            topmost::TopmostState::Normal => "NORMAL (replié)",
        };
        let Some(window) = self.window else { return };
        let Some(gpu) = self.gpu.as_mut() else { return };

        let raw_input = gpu.egui_winit.take_egui_input(window);
        let mut full_output = gpu.egui_ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgba_unmultiplied(20, 24, 34, 200)),
                )
                .show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.heading("wakfu-companion-overlay — spike S3 (X11)");
                    ui.add_space(6.0);
                    ui.label(format!(
                        "Fenêtre de jeu : {}",
                        found_character.as_deref().unwrap_or("(aucune trouvée)")
                    ));
                    ui.label(format!(
                        "Mode : {}",
                        if interactive {
                            "INTERACTIF (clics captés)"
                        } else {
                            "CLIC-TRAVERSANT"
                        }
                    ));
                    ui.label(format!("Topmost : {topmost_label}"));
                    ui.label(format!("Bascule : {HOTKEY_LABEL}"));
                });
        });
        gpu.egui_winit
            .handle_platform_output(window, full_output.platform_output);

        let paint_jobs = gpu
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

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

        for (id, deltas) in &full_output.textures_delta.set {
            for delta in deltas {
                gpu.egui_renderer
                    .update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("s3-encoder"),
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
                    label: Some("s3-egui-pass"),
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

        for id in &full_output.textures_delta.free {
            gpu.egui_renderer.free_texture(id);
        }
        // Explicite, requis depuis egui 0.36 (constaté ici : sans lui, `TexturesDelta` PANIQUE à
        // sa destruction — `debug_assert!` invisible en `--release`, ce qui a caché ce piège côté
        // `overlay-ui`/S1 jusqu'à ce que `overlay-ui::render` l'ajoute explicitement, voir sa doc.
        // Reproduit ici en conditions réelles : ce spike, lancé en `cargo build` DEBUG sous Xvfb,
        // paniquait bien à la première frame sans cette ligne.
        full_output.textures_delta.clear();

        gpu.queue.submit(Some(encoder.finish()));
        gpu.queue.present(output_frame);
    }
}

async fn init_gpu(window: &'static Window) -> GpuState {
    // `VULKAN | GL` (voir Cargo.toml) : sous Xvfb, seul lavapipe (Vulkan) ou llvmpipe (GL) sont
    // disponibles selon l'ICD/driver installé — laisser wgpu énumérer les deux plutôt que de
    // forcer un seul backend, voir README.md pour ce qui a été réellement trouvé sur cette
    // machine.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });

    let surface = instance
        .create_surface(window)
        .expect("création de la surface X11");

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            ..Default::default()
        })
        .await
        .expect("aucun adaptateur Vulkan/GL compatible (voir README.md — prérequis lavapipe)");
    println!("Adaptateur GPU : {:?}", adapter.get_info());

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("s3-spike-device"),
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
        window,
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

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,wgpu_hal=info"),
    )
    .init();

    let tracker = discovery::GameWindowTracker::connect()
        .expect("connexion X11 pour la découverte de fenêtres (DISPLAY défini ?)");

    let event_loop: EventLoop<()> = EventLoopBuilder::default()
        .with_x11()
        .build()
        .expect("création de l'event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(tracker);
    event_loop.run_app(&mut app).expect("boucle d'événements");
}

// Réserve pour un futur usage (voir S1) — pas utilisé pour l'instant, `about_to_wait` sonde
// directement `GlobalHotKeyEvent::receiver()`.
#[allow(dead_code)]
fn _unused_channel() -> Receiver<()> {
    mpsc::channel().1
}
