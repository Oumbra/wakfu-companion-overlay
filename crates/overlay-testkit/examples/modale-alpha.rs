//! **Rendu de la modale Options avec son vrai canal alpha**, pour publication en Artifact.
//!
//! Le harnais `egui_kittest` peint son propre panneau opaque avant la fermeture d'interface : un
//! rendu direct sort donc toujours à alpha 255, et l'on ne peut pas montrer ce que la fenêtre
//! laisse voir du jeu derrière elle. Cet outil rend deux fois la MÊME interface, sur fond noir puis
//! sur fond blanc, et résout la composition pixel par pixel :
//!
//! ```text
//! I_noir  = C·a              I_blanc = C·a + 255·(1 − a)
//! a = 1 − (I_blanc − I_noir) / 255       C = I_noir / a
//! ```
//!
//! Le résultat est un PNG RGBA à poser sur n'importe quel fond — c'est la seule façon honnête de
//! juger la translucidité : un damier peint DANS la capture est figé, et la modale l'écrase à 8 %.
//!
//! ```bash
//! cargo run -p overlay-testkit --example modale-alpha
//! # écrit target/rendus/modale-options-alpha.png
//! ```
//!
//! **`overlay_ui::style::apply` est appelé ici, comme dans tout harnais du dépôt.** Un outil de
//! rendu écrit à la main l'avait omis le 2026-09-10 : les polices du design system n'étaient pas
//! installées, le titre de section est sorti dans la proportionnelle par défaut d'`egui`, et
//! l'aperçu publié montrait un titre aux hampes tronquées qui n'existait nulle part dans le
//! produit. `design::text::famille` porte désormais une assertion contre cet oubli — cet exemple
//! existe pour qu'il n'y ait plus à réécrire le harnais du tout.

use egui_kittest::Harness;
use overlay_engine::CatalogIndex;
use overlay_ui::panels::combat::CombatSide;
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::panels::options_modal::{OptionsModalState, OptionsTab};
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content::{
    paint_content, AuthStatus, NoopAuthSink, OverlayKind, RenderContent,
};
use overlay_ui::ui_icons::UiIcons;

/// Chemin de sortie — sous `target/`, jamais commité (§17.4 du plan : pas de blob d'image au
/// dépôt en dehors des captures de non-régression).
fn sortie() -> std::path::PathBuf {
    let dir = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(target) => std::path::PathBuf::from(target),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"),
    }
    .join("rendus");
    std::fs::create_dir_all(&dir).expect("création de target/rendus");
    dir.join("modale-options-alpha.png")
}

/// Rend la modale sur `fond`, un aplat opaque posé sous toute l'interface.
fn rendu_sur(fond: egui::Color32) -> image::RgbaImage {
    let mut portraits: Option<PortraitAtlas> = None;
    let mut frame: Option<CombatFrame> = None;
    let mut icons: Option<UiIcons> = None;
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let now = std::time::Instant::now();
    let mut options_state = OptionsModalState {
        path_input: "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log".to_string(),
        error: None,
        // L'onglet du chemin de log, explicitement : la fenêtre s'ouvre sur « Alertes » depuis
        // le 2026-09-12, et c'est le champ de chemin que ce test regarde.
        tab: OptionsTab::Parametres,
        ..Default::default()
    };

    let mut harness = Harness::new_ui(move |ui| {
        let ctx = ui.ctx().clone();
        // MÊME style que les binaires et que `tests/panels.rs` — voir la doc de module.
        overlay_ui::style::apply(&ctx);
        // Curseur de saisie figé : le champ prend le focus dès la première frame, et un curseur
        // clignotant ferait dépendre les deux rendus du nombre de passes.
        ui.style_mut().visuals.text_cursor.blink = false;
        // Peint SOUS l'interface et au-delà du `max_rect` du harnais : les angles arrondis de la
        // fenêtre tombent hors de ce rectangle, et sans fond connu dessous leur alpha serait
        // indéterminé.
        ctx.layer_painter(egui::LayerId::background()).rect_filled(
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(4000.0, 4000.0)),
            0,
            fond,
        );
        let p = portraits.get_or_insert_with(|| PortraitAtlas::load(&ctx));
        let f = frame.get_or_insert_with(|| CombatFrame::load(&ctx));
        let i = icons.get_or_insert_with(|| UiIcons::load(&ctx));
        paint_content(
            ui,
            RenderContent {
                kind: OverlayKind::Options,
                fight: None,
                portraits: p,
                combat_frame: f,
                icons: i,
                combat_side: &mut combat_side,
                watchlist: &[],
                watchlist_toast: None,
                catalog: &catalog,
                catalog_stale: false,
                remote_icons: &remote_icon_store,
                remote_icon_textures: &mut remote_icon_textures,
                auth_status: &auth_status,
                auth_command_tx: &auth_sink,
                interactive: true,
                now,
                options: Some(&mut options_state),
            },
        );
    });
    harness.run();
    harness
        .render()
        .expect("rendu offscreen — voir doc de module")
}

fn main() {
    let noir = rendu_sur(egui::Color32::BLACK);
    let blanc = rendu_sur(egui::Color32::WHITE);
    assert_eq!(
        noir.dimensions(),
        blanc.dimensions(),
        "deux rendus, une taille"
    );

    let (w, h) = noir.dimensions();
    let mut out = image::RgbaImage::new(w, h);
    for (dst, (n, b)) in out.pixels_mut().zip(noir.pixels().zip(blanc.pixels())) {
        // L'écart entre les deux fonds ne subsiste qu'à hauteur de ce que la fenêtre laisse passer.
        let ecart: f32 = (0..3)
            .map(|c| f32::from(b[c]) - f32::from(n[c]))
            .sum::<f32>()
            / 3.0;
        let a = (1.0 - ecart / 255.0).clamp(0.0, 1.0);
        let couleur = |c: usize| {
            if a > 0.004 {
                (f32::from(n[c]) / a).clamp(0.0, 255.0) as u8
            } else {
                0
            }
        };
        *dst = image::Rgba([
            couleur(0),
            couleur(1),
            couleur(2),
            (a * 255.0).round() as u8,
        ]);
    }

    let chemin = sortie();
    out.save(&chemin).expect("écriture du rendu");
    println!("écrit {} ({w} × {h})", chemin.display());
}
