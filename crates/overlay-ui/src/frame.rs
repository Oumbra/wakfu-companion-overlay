//! État GPU par fenêtre et boucle de peinture par frame — voir la doc de `lib.rs` pour pourquoi ce
//! module (contrairement à `alert_sound`/`game_window`/`engine_thread`) est partagé entre les DEUX
//! binaires (`main.rs` Windows, `bin/overlay-ui-x11.rs` Linux) : rien ici ne dépend de l'OS, la
//! seule partie qui en dépendrait (le choix du backend GPU, `init_gpu`) reste volontairement
//! dupliquée dans chaque binaire plutôt que d'être artificiellement partagée ici.

use egui_wgpu::wgpu;

use crate::render_content::{build_ui, RenderContent, RenderOutcome};

/// État GPU/egui d'UNE fenêtre overlay — un par fenêtre, jamais partagé (voir la doc
/// d'`OverlayWindow` dans chaque binaire).
pub struct GpuState {
    /// Conservée (pas seulement une variable locale d'`init_gpu`) pour permettre de RECRÉER
    /// `surface` plus tard — voir `recreate_surface` et sa doc : un simple `surface.configure()`
    /// répété sur la MÊME `Surface` ne suffit pas à rejouer `IDCompositionVisual::SetContent` +
    /// `IDCompositionDevice::Commit()` (chemin `ResizeBuffers` de wgpu-hal, qui les saute
    /// délibérément — voir `vendor/wgpu-hal-30.0.1/src/dx12/mod.rs`), seule une `Surface`
    /// fraîchement créée le fait.
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub egui_ctx: egui::Context,
    pub egui_winit: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
    /// Instant d'ENTRÉE dans l'état `Occluded`/`Timeout` (voir `render`), `None` tant que la
    /// dernière tentative a réussi — sert uniquement à journaliser en `info!` l'ENTRÉE et la SORTIE
    /// de cet état (une fois chacune, jamais à chaque essai de 150 ms) : diagnostic 2026-09-06
    /// (retour utilisateur, instabilité perçue entre les deux overlays malgré le correctif de
    /// boucle de réessai) — sans cette trace, impossible de savoir depuis un simple journal si
    /// l'occlusion se produit effectivement pendant un test donné, combien de temps elle dure, et
    /// si elle coïncide avec les bascules topmost (`main.rs::sync_topmost`) plutôt que de devoir
    /// ré-analyser une vidéo image par image à chaque fois.
    pub occluded_since: Option<std::time::Instant>,
}

/// Fenêtrage/GPU autour de `render_content::build_ui` (voir sa doc) : prend l'entrée egui de la
/// fenêtre réelle, construit l'interface, puis tessèle/uploade/présente sur la vraie
/// `wgpu::Surface`. Renvoie le délai de redessin demandé par egui pour CETTE fenêtre
/// (`ViewportOutput::repaint_delay`, ex. le délai d'apparition d'une tooltip) — voir le champ
/// `next_redraw_at` de chaque binaire pour pourquoi l'appelant doit impérativement en tenir
/// compte, cette architecture n'ayant pas de boucle de rendu continue — ainsi que le
/// [`RenderOutcome`] produit par cette frame (fermeture du toast, ouverture/action de la modale
/// Options — voir sa doc et celle de `build_ui`).
pub fn render(
    gpu: &mut GpuState,
    window: &winit::window::Window,
    content: RenderContent<'_>,
) -> (std::time::Duration, RenderOutcome) {
    // Copiés hors de `content` (tous deux `Copy`) AVANT qu'il ne soit déplacé dans `build_ui` —
    // réutilisés juste en dessous pour le calcul du délai de redessin, sur la MÊME référence de
    // temps que celle vue par le contenu peint (voir la doc de `RenderContent::now`).
    let now = content.now;
    let watchlist_toast = content.watchlist_toast;

    let raw_input = gpu.egui_winit.take_egui_input(window);
    let (mut full_output, outcome) = build_ui(&gpu.egui_ctx, raw_input, content);

    // Repli `Duration::MAX` ("pas de redessin demandé") si jamais le viewport racine n'a pas
    // d'entrée — ne devrait pas arriver en pratique (une seule fenêtre racine par `egui::Context`,
    // jamais de sous-viewport ici).
    let mut repaint_delay = full_output
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .map_or(std::time::Duration::MAX, |viewport| viewport.repaint_delay);
    // Le toast d'alerte anime des confettis en continu (voir `panels::watchlist::toast_card`) et
    // disparaît de lui-même après `WatchlistToast::hide_at` (voir sa doc) — sans ceci, rien ne
    // redéclencherait de redessin pendant l'animation NI à l'expiration, dans cette architecture
    // sans boucle de rendu continue (§6.1). S'arrête de lui-même dès que `hide_at` est dépassé (le
    // toast n'est alors plus reçu par `panels::watchlist::show`, voir son filtre) — jamais de
    // minuterie à annuler explicitement, seulement des redessins qui cessent d'être redemandés.
    if let Some(toast) = watchlist_toast {
        // Un toast à fermeture manuelle (`hide_at: None`) n'a pas d'échéance à programmer, mais
        // ses confettis tombent quand même : le redessin reste demandé, sans borne de fin.
        if toast.hide_at.is_none_or(|hide_at| hide_at > now) {
            const CONFETTI_FRAME_INTERVAL: std::time::Duration =
                std::time::Duration::from_millis(33); // ~30 images/s, largement suffisant à cette échelle
            repaint_delay = repaint_delay
                .min(
                    toast
                        .hide_at
                        .map_or(std::time::Duration::MAX, |hide_at| hide_at - now),
                )
                .min(CONFETTI_FRAME_INTERVAL);
        }
    }

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
        wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
            // Constaté côté Windows (DXGI renvoie couramment `Occluded` pour une fenêtre
            // `AlwaysOnTop` qui vient de recevoir un changement de style étendu sans qu'aucune
            // entrée utilisateur ne lui soit adressée, voir `main.rs`) : sur `DXGI_STATUS_OCCLUDED`
            // (guide officiel), arrêter de dessiner MAIS continuer à sonder périodiquement pour
            // détecter la fin de l'occlusion — cette frame-ci est perdue.
            //
            // **Correctif 2026-09-06** (retour utilisateur : un overlay « gèle » ou perd le survol
            // dès que l'autre reprend le premier plan) : l'ancien code rappelait `request_redraw()`
            // ICI MÊME, sans aucun délai. Résultat : tant que la fenêtre restait occluse — ce qui
            // arrive précisément à chaque bascule topmost entre les DEUX overlays (le changement de
            // style qui déclenche `Occluded`, voir ci-dessus) — ce redessin immédiat en reprovoquait
            // aussitôt un autre, en boucle SERRÉE, sur la boucle winit MONO-THREAD partagée par
            // TOUTES les fenêtres overlay (une seule `App`/`run_app`, voir `main.rs`). Cette boucle
            // de réessai monopolisait le thread au lieu de rendre la main à `about_to_wait`/
            // `ControlFlow::WaitUntil`, empêchant les événements (survol, clic) de l'AUTRE fenêtre
            // d'être traités tant qu'elle durait — exactement le gel observé. On borne désormais le
            // délai avant nouvelle tentative à un sondage périodique court plutôt qu'un redessin
            // instantané : imperceptible pour l'utilisateur une fois l'occlusion levée, mais laisse
            // enfin la boucle d'événements respirer entre deux essais (§6.1 : pas de boucle de rendu
            // continue — ce correctif restaure ce principe, que l'ancien code violait justement pour
            // une fenêtre occluse). Le mécanisme de délai existe déjà (`next_redraw_at` côté
            // appelant, voir sa doc) : inutile de forcer `request_redraw()` nous-mêmes, il suffit de
            // renvoyer un délai borné. Comportement générique wgpu, pas spécifique à un backend :
            // traité pareil quel que soit l'OS qui exécute ce code.
            const OCCLUDED_RETRY_INTERVAL: std::time::Duration =
                std::time::Duration::from_millis(150);
            // `info!` une seule fois à l'ENTRÉE dans l'occlusion (jamais à chaque essai de 150 ms,
            // qui spammerait le journal pour rien) — voir la doc de `GpuState::occluded_since`.
            if gpu.occluded_since.is_none() {
                gpu.occluded_since = Some(now);
                tracing::info!(
                    "[occlusion] fenêtre « {} » occluse/timeout — sondage toutes les {}ms",
                    window.title(),
                    OCCLUDED_RETRY_INTERVAL.as_millis()
                );
            }
            return (repaint_delay.min(OCCLUDED_RETRY_INTERVAL), outcome);
        }
        wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
            gpu.surface.configure(&gpu.device, &gpu.config);
            // Cette frame-ci (celle qui portait le changement — opacité, taille, n'importe quel
            // contenu) est purement et simplement PERDUE, jamais présentée. Sans redemander
            // explicitement un redessin ici, plus rien ne le fait tant qu'un événement SANS
            // RAPPORT ne survient par ailleurs (§6.1 : pas de boucle de rendu continue).
            // `request_redraw` ici force une nouvelle tentative dès le prochain tour de la boucle
            // d'événements, sur la surface qui vient d'être reconfigurée juste au-dessus.
            window.request_redraw();
            return (repaint_delay, outcome);
        }
        wgpu::CurrentSurfaceTexture::Validation => {
            // PAS de `request_redraw` ici, contrairement à Outdated/Lost ci-dessus : cette
            // branche ne reconfigure rien, donc rien ne garantit qu'une nouvelle tentative
            // réussirait mieux que celle-ci — redemander sans arrêt un redessin qui échouerait à
            // nouveau à chaque tick reviendrait à la boucle de rendu continue que cette
            // architecture évite justement (§6.1). Se contente de journaliser.
            tracing::warn!("get_current_texture: erreur de validation");
            return (repaint_delay, outcome);
        }
    };
    // Sortie de l'occlusion (voir son entrée ci-dessus) : cette frame-ci a réussi
    // `get_current_texture()`, donc l'occlusion — si elle avait commencé — vient de se terminer.
    if let Some(since) = gpu.occluded_since.take() {
        tracing::info!(
            "[occlusion] fenêtre « {} » de nouveau visible après {:?}",
            window.title(),
            since.elapsed()
        );
    }
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
    (repaint_delay, outcome)
}

/// Recrée `gpu.surface` de zéro à partir de `gpu.instance` (même `device`/`queue` conservés,
/// aucun besoin de re-négocier un adaptateur) et la reconfigure aussitôt — voir la doc de
/// `GpuState::instance` pour la raison : un simple `gpu.surface.configure()` répété sur la MÊME
/// `Surface` ne suffit PAS à rejouer `IDCompositionVisual::SetContent` +
/// `IDCompositionDevice::Commit()` sous Windows (chemin `ResizeBuffers` de wgpu-hal, qui les
/// saute délibérément une fois la surface déjà configurée une première fois).
///
/// **Diagnostic 2026-09-06** (retour utilisateur, plusieurs sessions) : une fenêtre restée
/// `HWND_NOTOPMOST` un moment, puis repromue `HWND_TOPMOST`, pouvait continuer à présenter des
/// frames "avec succès" (`get_current_texture()` ne renvoyait jamais `Occluded`/`Timeout` — voir
/// `GpuState::occluded_since`, jamais atteint dans ces sessions) sans jamais réellement réapparaître
/// à l'écran, jusqu'à une interaction quelconque de l'utilisateur dessus. Hypothèse la plus
/// probable trouvée en lisant `vendor/wgpu-hal-30.0.1/src/dx12/mod.rs` : DWM peut ne pas avoir
/// (re)composé le visual DirectComposition de cette fenêtre pendant sa période `NOTOPMOST`, et
/// rien ne force `SetContent`/`Commit` à être rejoués depuis la toute première configuration —
/// recréer la `Surface` à chaque repromotion (voir `main.rs::App::sync_topmost`) force ce chemin.
///
/// Réservé à cet appel PONCTUEL (à la repromotion, pas à chaque frame ni à chaque
/// redimensionnement — `reconfigure_surface`/`surface.configure()` restent largement suffisants
/// et bien moins coûteux pour ces deux cas) : recréer une `Surface` a un coût réel (nouvelle
/// négociation DXGI), pas justifié pour un événement qui se produit au plus quelques fois par
/// minute. Spécifique à `main.rs` (Windows/DirectComposition) : `overlay-ui-x11.rs` ne l'appelle
/// pas, X11/EWMH n'ayant pas cette limitation de composition (pas de `Commit` séparé à rejouer).
pub fn recreate_surface(gpu: &mut GpuState, window: &std::sync::Arc<winit::window::Window>) {
    match gpu.instance.create_surface(std::sync::Arc::clone(window)) {
        Ok(surface) => {
            gpu.surface = surface;
            gpu.surface.configure(&gpu.device, &gpu.config);
        }
        Err(err) => {
            // Jamais fatal : la fenêtre continue avec son ANCIENNE surface (toujours valide,
            // juste peut-être pas recomposée par DWM) plutôt que de planter tout l'overlay pour
            // un correctif qui reste, par nature, best-effort.
            tracing::warn!("recreate_surface : échec de recréation de la surface : {err}");
        }
    }
}
