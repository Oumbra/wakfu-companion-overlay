//! Spike S1 (docs/plan-architecture.md §12) — question posée :
//!
//!   « Une fenêtre transparente, toujours au-dessus, traversable par la souris, rendue avec
//!   wgpu+egui, est-elle atteignable sous Windows sans piège de composition ? »
//!
//! Toutes les API utilisées ici (`Dx12SwapchainKind::DxgiFromVisual` en particulier) ont été
//! vérifiées dans le CODE SOURCE réel de wgpu-hal 30.0.1 (pas dans sa doc docs.rs, incomplète
//! sur les items spécifiques à Windows car docs.rs documente par défaut une cible Linux) — voir
//! README.md de ce dossier pour le détail de cette découverte, qui simplifie largement le §6.2
//! du plan (pas besoin de piloter IDCompositionDevice/Target/Visual à la main : wgpu-hal le fait
//! déjà en interne dès qu'on lui demande `DxgiFromVisual`).
//!
//! Ce binaire n'est PAS l'ébauche du futur `overlay-platform`/`overlay-ui` : c'est un harnais de
//! validation, jetable, limité à Windows.

use std::num::NonZeroIsize;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use egui_wgpu::wgpu;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dxgi::{
    DXGIGetDebugInterface1, DXGI_DEBUG_ALL, DXGI_INFO_QUEUE_MESSAGE, IDXGIInfoQueue,
};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

#[cfg(target_os = "windows")]
use winit::platform::windows::WindowAttributesExtWindows;

/// Rappel du hotkey affiché à l'écran et dans la console.
const HOTKEY_LABEL: &str = "Ctrl+Alt+W";

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
    /// Jamais lu après `new()` : sa seule raison d'être ici est de rester en vie pour toute la
    /// durée du process — le laisser tomber désenregistrerait le hotkey global.
    #[allow(dead_code)]
    hotkey_manager: GlobalHotKeyManager,
    hotkey_events: &'static global_hotkey::GlobalHotKeyEventReceiver,
    /// `true` = fenêtre interactive (capture les clics), `false` = clic-traversant.
    interactive: bool,
    started_at: Instant,
    last_rss_print: Instant,
    frame_count: u64,
}

impl App {
    fn new() -> Self {
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
            started_at: Instant::now(),
            last_rss_print: Instant::now(),
            frame_count: 0,
        }
    }

    /// `WS_EX_NOACTIVATE` : la fenêtre ne vole jamais le focus au jeu, même cliquée en mode
    /// interactif. `WS_EX_TOOLWINDOW` : absente du alt-tab et de la barre des tâches. Ni l'un ni
    /// l'autre n'est exposé par `winit` — on passe par le HWND brut (§6.2 du plan).
    fn apply_extended_styles(hwnd: HWND) {
        unsafe {
            let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let new_style = current | (WS_EX_NOACTIVATE.0 as isize) | (WS_EX_TOOLWINDOW.0 as isize);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
        }
    }

    fn hwnd_of(window: &Window) -> HWND {
        match window.window_handle().expect("handle de fenêtre").as_raw() {
            RawWindowHandle::Win32(handle) => {
                let raw: NonZeroIsize = handle.hwnd;
                HWND(raw.get() as *mut _)
            }
            other => panic!("handle de fenêtre inattendu sur Windows : {other:?}"),
        }
    }

    fn print_rss(&mut self) {
        if self.last_rss_print.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.last_rss_print = Instant::now();
        unsafe {
            let mut counters = PROCESS_MEMORY_COUNTERS::default();
            let ok = GetProcessMemoryInfo(
                GetCurrentProcess(),
                &mut counters,
                std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            );
            if ok.is_ok() {
                let rss_mb = counters.WorkingSetSize as f64 / (1024.0 * 1024.0);
                println!(
                    "[t+{:>5.1}s] RSS = {:>6.1} Mo  |  mode = {}  |  frames = {}",
                    self.started_at.elapsed().as_secs_f64(),
                    rss_mb,
                    if self.interactive { "INTERACTIF (clics captes)" } else { "CLIC-TRAVERSANT" },
                    self.frame_count
                );
            }
        }
    }

    fn toggle_interactive(&mut self) {
        self.interactive = !self.interactive;
        if let Some(window) = self.window {
            // `set_cursor_hittest(false)` = la fenêtre laisse passer tous les événements
            // souris (clic-traversant) — voir §6.3 du plan.
            if let Err(err) = window.set_cursor_hittest(self.interactive) {
                eprintln!("set_cursor_hittest a échoué : {err}");
            }
        }
        println!(
            "\n>>> Bascule ({}) : mode = {}\n",
            HOTKEY_LABEL,
            if self.interactive { "INTERACTIF" } else { "CLIC-TRAVERSANT" }
        );
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("Spike S1 — wakfu-companion-overlay")
            .with_inner_size(winit::dpi::LogicalSize::new(420.0, 220.0))
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false);
        #[cfg(target_os = "windows")]
        let attrs = attrs.with_skip_taskbar(true).with_no_redirection_bitmap(true);

        let window = event_loop
            .create_window(attrs)
            .expect("création de la fenêtre");
        // Fuite volontaire : durée de vie 'static nécessaire pour wgpu::Surface<'static> dans ce
        // spike jetable (une seule fenêtre, jamais recréée, jamais libérée avant la fin du process).
        let window: &'static Window = Box::leak(Box::new(window));

        let hwnd = Self::hwnd_of(window);
        Self::apply_extended_styles(hwnd);

        let gpu = pollster::block_on(Self::init_gpu(window));
        self.window = Some(window);
        self.gpu = Some(gpu);

        // Etat initial : interactif (on voit tout de suite qu'on peut cliquer dedans), à basculer
        // avec le hotkey.
        println!("=== Spike S1 — fenêtre transparente Windows (DirectComposition via wgpu) ===");
        println!("Fenêtre créée. {HOTKEY_LABEL} pour basculer interactif / clic-traversant.");
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
                gpu.config.width = size.width;
                gpu.config.height = size.height;
                gpu.surface.configure(&gpu.device, &gpu.config);
            }
            WindowEvent::RedrawRequested => {
                self.render();
                self.print_rss();
                // Rendu réactif (§6.1 du plan) : pas de boucle 60 Hz forcée dans ce spike, on
                // republie juste une frame après l'autre pour observer le comportement en continu.
                if let Some(window) = self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Le hotkey global arrive hors du thread d'événements winit (thread dédié côté OS) —
        // on le sonde ici, sans bloquer, à chaque tour de boucle.
        if self.hotkey_events.try_recv().is_ok() {
            self.toggle_interactive();
        }
        event_loop.set_control_flow(ControlFlow::Poll);
    }
}

impl App {
    async fn init_gpu(window: &'static Window) -> GpuState {
        // Backend forcé à DX12 : c'est le seul qui sache utiliser `Dx12SwapchainKind::DxgiFromVisual`
        // (Vulkan/GL n'ont pas d'équivalent DirectComposition dans wgpu-hal) — voir README.md.
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
                label: Some("s1-spike-device"),
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
            // DXGI_ALPHA_MODE_STRAIGHT (= wgpu::CompositeAlphaMode::PostMultiplied) est REJETÉ
            // par CreateSwapChainForComposition sur ce pilote (RTX 3080 Ti, 32.0.16.1062,
            // Windows 11) — confirmé par repro Win32/DXGI brut, hors wgpu-hal (voir
            // src/raw_repro.rs et README.md). DXGI_ALPHA_MODE_PREMULTIPLIED, lui, réussit — et
            // correspond de toute façon à ce qu'`egui_wgpu::Renderer` produit déjà en interne
            // (blending alpha prémultiplié), donc pas de conversion de couleur nécessaire.
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
        let egui_renderer = egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());

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

    fn render(&mut self) {
        let interactive = self.interactive;
        self.frame_count += 1;
        let Some(window) = self.window else { return };
        let Some(gpu) = self.gpu.as_mut() else { return };

        let raw_input = gpu.egui_winit.take_egui_input(window);
        let full_output = gpu.egui_ctx.run_ui(raw_input, |ui| {
            egui::CentralPanel::default()
                .frame(egui::Frame::default().fill(egui::Color32::from_rgba_unmultiplied(20, 24, 34, 200)))
                .show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.heading("wakfu-companion-overlay — spike S1");
                    ui.add_space(6.0);
                    ui.label(format!(
                        "Mode actuel : {}",
                        if interactive { "INTERACTIF (clics captés)" } else { "CLIC-TRAVERSANT" }
                    ));
                    ui.label(format!("Bascule : {HOTKEY_LABEL}  (fonctionne même sans focus)"));
                    ui.add_space(6.0);
                    if ui.button("Un bouton pour vérifier le clic").clicked() {
                        println!(">>> Bouton egui cliqué — la capture de clic fonctionne.");
                    }
                });
        });
        gpu.egui_winit
            .handle_platform_output(window, full_output.platform_output);

        let paint_jobs = gpu
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        // `get_current_texture()` (wgpu 30) renvoie directement un enum, plus un `Result` — voir
        // README.md, API vérifiée dans le code source (docs.rs ne documente pas fidèlement les
        // versions très récentes).
        let output_frame = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return;
            }
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
            // `set` regroupe désormais les deltas par id (`SmallVec<[ImageDelta; 1]>`, en
            // pratique un seul élément la plupart du temps) — appliquer chacun dans l'ordre.
            for delta in deltas {
                gpu.egui_renderer
                    .update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("s1-encoder") });
        gpu.egui_renderer
            .update_buffers(&gpu.device, &gpu.queue, &mut encoder, &paint_jobs, &screen_descriptor);

        {
            // Fond totalement transparent (alpha 0) : seul ce que dessine egui doit apparaître —
            // c'est le test décisif de la composition DirectComposition.
            let mut render_pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("s1-egui-pass"),
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
            gpu.egui_renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            gpu.egui_renderer.free_texture(id);
        }

        gpu.queue.submit(Some(encoder.finish()));
        // `present()` a migré de `SurfaceTexture` vers `Queue` dans cette version — voir README.md.
        gpu.queue.present(output_frame);
    }
}

/// DIAGNOSTIC TEMPORAIRE (spike S1) : `wgpu-hal` ne consulte que la couche de debug D3D12
/// (nécessite la fonctionnalité facultative Windows « Outils graphiques », absente ici et non
/// installable sans élévation dans cet environnement). `IDXGIInfoQueue` (dxgidebug.dll) est une
/// source indépendante, généralement disponible sans cette fonctionnalité, et porte le message
/// de validation précis derrière l'HRESULT générique DXGI_ERROR_INVALID_CALL. Installé comme
/// panic hook : le panic de wgpu-core survient plusieurs lignes de log après l'échec DXGI réel,
/// mais la file de messages DXGI n'est vidée par rien entre-temps.
fn install_dxgi_debug_panic_hook() {
    let info_queue: Option<IDXGIInfoQueue> = unsafe { DXGIGetDebugInterface1(0) }.ok();
    if info_queue.is_none() {
        eprintln!(
            "(DIAG) IDXGIInfoQueue indisponible (DXGIGetDebugInterface1 a échoué) — la \
             fonctionnalité facultative « Outils graphiques » Windows n'est probablement pas \
             installée."
        );
    }
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(iq) = &info_queue {
            unsafe {
                let count = iq.GetNumStoredMessages(DXGI_DEBUG_ALL);
                eprintln!("=== (DIAG) IDXGIInfoQueue : {count} message(s) stocké(s) ===");
                for i in 0..count {
                    let mut len: usize = 0;
                    if iq.GetMessage(DXGI_DEBUG_ALL, i, None, &mut len).is_ok() && len > 0 {
                        let mut buf = vec![0u8; len];
                        let msg_ptr = buf.as_mut_ptr().cast::<DXGI_INFO_QUEUE_MESSAGE>();
                        if iq.GetMessage(DXGI_DEBUG_ALL, i, Some(msg_ptr), &mut len).is_ok() {
                            let msg = &*msg_ptr;
                            let desc_len = msg.DescriptionByteLength.saturating_sub(1);
                            let desc = std::slice::from_raw_parts(msg.pDescription, desc_len);
                            eprintln!(
                                "(DIAG) [{:?}/{:?}] id={} : {}",
                                msg.Category,
                                msg.Severity,
                                msg.ID,
                                String::from_utf8_lossy(desc)
                            );
                        }
                    }
                }
            }
        }
        default_hook(info);
    }));
}

fn main() {
    // Sans logger initialisé, wgpu-core avale silencieusement le VRAI message d'erreur (HRESULT
    // DirectComposition, etc.) derrière `ConfigureSurfaceError::InvalidSurface` — voir README.md.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,wgpu_hal=info"))
        .init();
    install_dxgi_debug_panic_hook();

    let event_loop = EventLoop::new().expect("création de l'event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("boucle d'événements");
}

// Réserve pour un futur usage si le spike évolue (canal explicite plutôt que sondage) — pas
// utilisé pour l'instant, `about_to_wait` sonde directement `GlobalHotKeyEvent::receiver()`.
#[allow(dead_code)]
fn _unused_channel() -> Receiver<()> {
    mpsc::channel().1
}
