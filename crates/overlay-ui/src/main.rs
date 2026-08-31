//! `overlay-ui` (docs/plan-architecture.md §6 et §9, lot L2) — premier vrai overlay : fenêtre
//! transparente Windows (DirectComposition, voir spike S1) rendant deux panneaux réels alimentés
//! par `overlay-ingest` + `overlay-engine` sur un vrai `wakfu.log` : dégâts du combat en cours,
//! récap de session. Le fenêtrage/rendu reprend telle quelle l'approche validée par S1
//! (`spikes/s1-window-windows/README.md`) — mêmes bugs déjà corrigés (patch `wgpu-hal`, alpha
//! prémultiplié, clamp de redimensionnement), pas réinventée ici.
//!
//! Volontairement incomplet par rapport à §9 du plan : pas encore de panneau Suivi/Alertes/État de
//! synchro (ceux-là dépendent soit de `StatsStoreService` non porté — §14 point 3 — soit de la
//! synchro serveur, L4/L5), pas de disposition persistée par écran, pas de thème configurable. Ce
//! sont les deux panneaux atteignables avec `overlay-engine` tel qu'il existe aujourd'hui.

use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use arc_swap::ArcSwap;
use egui_wgpu::wgpu;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use overlay_engine::{Engine, SessionSnapshot};
use overlay_ingest::discovery;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

#[cfg(target_os = "windows")]
use winit::platform::windows::WindowAttributesExtWindows;

const HOTKEY_LABEL: &str = "Ctrl+Alt+W";
const WINDOW_SIZE: (f64, f64) = (360.0, 480.0);

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

struct App {
    window: Option<&'static Window>,
    gpu: Option<GpuState>,
    #[allow(dead_code)] // jamais relu : sa seule raison d'être est de rester en vie (voir S1)
    hotkey_manager: GlobalHotKeyManager,
    hotkey_events: &'static global_hotkey::GlobalHotKeyEventReceiver,
    interactive: bool,
    snapshot: Arc<ArcSwap<SessionSnapshot>>,
    log_path: PathBuf,
}

impl App {
    fn new(log_path: PathBuf, snapshot: Arc<ArcSwap<SessionSnapshot>>) -> Self {
        let hotkey_manager = GlobalHotKeyManager::new().expect("création GlobalHotKeyManager");
        let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW);
        hotkey_manager
            .register(hotkey)
            .expect("enregistrement du hotkey global");

        Self {
            window: None,
            gpu: None,
            hotkey_manager,
            hotkey_events: GlobalHotKeyEvent::receiver(),
            interactive: true,
            snapshot,
            log_path,
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
        if let Some(window) = self.window {
            if let Err(err) = window.set_cursor_hittest(self.interactive) {
                eprintln!("set_cursor_hittest a échoué : {err}");
            }
            window.request_redraw();
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
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("wakfu-companion-overlay")
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
            .expect("création de la fenêtre");
        // Fuite volontaire : durée de vie 'static nécessaire pour wgpu::Surface<'static> — une
        // seule fenêtre, jamais recréée, jamais libérée avant la fin du process (voir S1).
        let window: &'static Window = Box::leak(Box::new(window));

        Self::apply_extended_styles(Self::hwnd_of(window));

        let gpu = pollster::block_on(init_gpu(window));
        self.window = Some(window);
        self.gpu = Some(gpu);

        println!("=== wakfu-companion-overlay (L2, overlay-ui) ===");
        println!("Suivi de {}", self.log_path.display());
        println!("{HOTKEY_LABEL} pour basculer interactif / clic-traversant. Échap/Ctrl+C pour quitter.\n");

        window.request_redraw();
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::NewSnapshot => {
                if let Some(window) = self.window {
                    window.request_redraw();
                }
            }
        }
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
                // Clamp défensif (voir S1, README.md §"Bug de redimensionnement") : un resize
                // excessif ne doit jamais faire planter l'overlay, quelle qu'en soit la cause.
                let max_dim = gpu.device.limits().max_texture_dimension_2d;
                gpu.config.width = size.width.min(max_dim);
                gpu.config.height = size.height.min(max_dim);
                gpu.surface.configure(&gpu.device, &gpu.config);
            }
            WindowEvent::RedrawRequested => {
                render(gpu, window, self.interactive, &self.snapshot.load())
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Hotkey global : thread OS dédié, sondé ici sans bloquer (voir S1).
        if self.hotkey_events.try_recv().is_ok() {
            self.toggle_interactive();
        }
        // Réactif (§6.1) : on attend soit un événement fenêtre, soit un `UserEvent::NewSnapshot`
        // du thread Engine, jamais de boucle 60 Hz forcée. Le sondage hotkey ci-dessus impose
        // quand même un réveil périodique court, sans quoi le hotkey ne serait vu qu'au prochain
        // événement fenêtre.
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(50),
        ));
    }
}

async fn init_gpu(window: &'static Window) -> GpuState {
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

    let surface = instance
        .create_surface(window)
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

fn render(gpu: &mut GpuState, window: &Window, interactive: bool, snapshot: &SessionSnapshot) {
    let raw_input = gpu.egui_winit.take_egui_input(window);
    let full_output = gpu.egui_ctx.run_ui(raw_input, |ui| {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(18, 20, 28, 215)),
            )
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.heading("wakfu-companion-overlay");
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

                ui.strong("Dégâts du combat");
                match &snapshot.current_fight {
                    None => {
                        ui.weak("Aucun combat pour l'instant.");
                    }
                    Some(fight) => {
                        let mut fighters = fight.fighters.clone();
                        fighters.sort_by_key(|f| std::cmp::Reverse(f.total_damage));
                        let max_damage =
                            fighters.first().map(|f| f.total_damage).unwrap_or(0).max(1);
                        for fighter in &fighters {
                            ui.horizontal(|ui| {
                                let color = if fighter.is_ally {
                                    egui::Color32::from_rgb(110, 200, 140)
                                } else {
                                    egui::Color32::from_rgb(210, 100, 100)
                                };
                                ui.colored_label(color, &fighter.name);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.monospace(fighter.total_damage.to_string());
                                    },
                                );
                            });
                            let ratio = fighter.total_damage as f32 / max_damage as f32;
                            ui.add(
                                egui::ProgressBar::new(ratio)
                                    .desired_height(4.0)
                                    .show_percentage()
                                    .text(""),
                            );
                        }
                        let status = match fight.result {
                            None => "en cours".to_string(),
                            Some(overlay_engine::FightResult::Won) => "gagné".to_string(),
                            Some(overlay_engine::FightResult::Lost) => "perdu".to_string(),
                        };
                        ui.small(format!("Combat #{} — {status}", fight.fight_id));
                    }
                }

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

    for id in &full_output.textures_delta.free {
        gpu.egui_renderer.free_texture(id);
    }

    gpu.queue.submit(Some(encoder.finish()));
    gpu.queue.present(output_frame);
}

/// Thread Engine (§3 du plan) : lit `wakfu.log` en continu, alimente `overlay-engine`, publie
/// chaque nouveau `SessionSnapshot` par `ArcSwap` et réveille le main thread. Ne rappelle jamais
/// l'UI directement — l'UI ne lit que la dernière valeur publiée (§3 : « zéro verrou sur le
/// chemin de rendu »).
fn spawn_engine_thread(
    log_path: PathBuf,
    snapshot: Arc<ArcSwap<SessionSnapshot>>,
    proxy: EventLoopProxy<UserEvent>,
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
            for result in rx {
                match result {
                    Ok(batch) => {
                        if let Err(err) = engine.ingest_batch(&batch) {
                            tracing::warn!(%err, "échec d'ingestion d'un lot, ligne(s) ignorée(s)");
                            continue;
                        }
                        snapshot.store(Arc::new(engine.snapshot()));
                        let _ = proxy.send_event(UserEvent::NewSnapshot);
                    }
                    Err(err) => tracing::warn!(%err, "erreur de lecture de wakfu.log"),
                }
            }
        })
        .expect("échec de création du thread Engine");
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
    spawn_engine_thread(log_path.clone(), Arc::clone(&snapshot), proxy);

    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(log_path, snapshot);
    event_loop.run_app(&mut app).expect("boucle d'événements");
}
