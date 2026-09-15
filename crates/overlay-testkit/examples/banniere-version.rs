//! **Planche de choix du numéro de version en bannière** (2026-09-14), pour publication en Artifact.
//!
//! La demande : afficher la version du produit à GAUCHE de la bannière de la modale Options, à la
//! même distance du bord que la croix de fermeture à droite (`tokens::WINDOW_CLOSE_MARGIN`, 12 px),
//! centrée verticalement comme elle, dans la même serif blanche à ombre portée que le titre — mais
//! plus petite. Reste à trancher : **de combien** plus petite.
//!
//! ```bash
//! cargo run -p overlay-testkit --example banniere-version
//! # écrit target/rendus/banniere-version-*.png
//! ```
//!
//! Deux sorties :
//! - `banniere-version-tailles.png` — trois bandeaux à l'échelle 1:1 (13, 15 et 17 px de corps)
//!   pour comparer à l'œil, titre et croix compris. Les deux tailles NON retenues sont peintes ici
//!   à la main : `design::window` ne rend que celle du manifeste
//!   (`tokens::WINDOW_VERSION_FONT_SIZE`), et ajouter un réglage de taille au composant pour une
//!   planche de comparaison aurait laissé au design system une option dont personne n'a besoin.
//! - `banniere-version-fenetre.png` — la VRAIE fenêtre du design system, taille en vigueur, pour
//!   juger l'ensemble plutôt que le bandeau seul.
//!
//! `overlay_ui::style::apply` est appelé comme dans tout harnais du dépôt (voir la doc de
//! `examples/modale-alpha.rs` : sans lui, les polices du design system ne sont pas installées et le
//! rendu publié ne montre pas le produit).

use egui::{Align2, Color32, Pos2, Rect, Vec2};
use egui_kittest::Harness;
use overlay_ui::design::{
    self, assets::DsTexture, icons::DsIcon, text, tokens, DesignSystem, IconContext,
};

/// Largeur native de `modal-header.png`, donc de la fenêtre Options.
const LARGEUR: f32 = 720.0;

/// Les corps comparés. 21 px est celui du titre : le numéro doit rester en dessous sans devenir
/// illisible par-dessus le grain de la bannière.
const TAILLES: [f32; 3] = [13.0, 15.0, 17.0];

fn dossier_sortie() -> std::path::PathBuf {
    let dir = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(target) => std::path::PathBuf::from(target),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"),
    }
    .join("rendus");
    std::fs::create_dir_all(&dir).expect("création de target/rendus");
    dir
}

/// Peint UN bandeau : la texture du jeu, le titre centré, le numéro à gauche au corps demandé.
///
/// Reproduit volontairement `design::components::window` (mêmes tokens, même ancrage, même ombre) —
/// voir la doc de module pour la raison. Toute divergence se verrait immédiatement en comparant
/// avec la seconde sortie, qui passe bien par le composant.
fn bandeau(ui: &mut egui::Ui, rect: Rect, taille_version: f32, version: &str) {
    let ds = DesignSystem::get(ui.ctx());
    egui::Image::new(ds.texture(DsTexture::ModalHeader))
        .corner_radius(egui::CornerRadius {
            nw: tokens::WINDOW_RADIUS,
            ne: tokens::WINDOW_RADIUS,
            sw: 0,
            se: 0,
        })
        .paint_at(ui, rect);

    text::paint_outlined_text(
        ui,
        rect.center(),
        Align2::CENTER_CENTER,
        "Options",
        text::title_font(ui.ctx(), tokens::WINDOW_TITLE_FONT_SIZE),
        tokens::WINDOW_TITLE_TEXT,
        text::SHADOW_BOTTOM_RIGHT,
    );

    text::paint_outlined_text(
        ui,
        Pos2::new(rect.left() + tokens::WINDOW_CLOSE_MARGIN, rect.center().y),
        Align2::LEFT_CENTER,
        version,
        text::title_font(ui.ctx(), taille_version),
        tokens::WINDOW_TITLE_TEXT,
        text::SHADOW_BOTTOM_RIGHT,
    );

    // La VRAIE croix du design system, à sa place réelle — c'est elle qui donne la cote que le
    // numéro reprend de l'autre côté : une approximation peinte ici ne prouverait pas la symétrie.
    let croix = Rect::from_center_size(
        Pos2::new(
            rect.right() - tokens::WINDOW_CLOSE_MARGIN - tokens::WINDOW_CLOSE_SIZE / 2.0,
            rect.center().y,
        ),
        Vec2::splat(tokens::WINDOW_CLOSE_SIZE),
    );
    ui.put(
        croix,
        design::icon_button(DsIcon::CloseWindow)
            .context(IconContext::Banner)
            .size(croix.width())
            .log_name("planche.fermer"),
    );
}

/// Une ligne de la planche : le corps à comparer, le numéro à peindre, la légende sous le bandeau.
fn lignes() -> Vec<(f32, String, String)> {
    let mut v: Vec<(f32, String, String)> = TAILLES
        .iter()
        .map(|t| {
            (
                *t,
                overlay_ui::build_info::BANNER_LABEL.to_string(),
                format!("{t:.0} px"),
            )
        })
        .collect();
    // Pire cas de longueur, au corps proposé : deux chiffres à chaque composante. Le numéro pousse
    // vers le centre, jamais vers la croix — reste-t-il à distance respectable du titre ?
    v.push((
        tokens::WINDOW_VERSION_FONT_SIZE,
        "10.12.4".to_string(),
        format!(
            "{:.0} px — numéro long (pire cas)",
            tokens::WINDOW_VERSION_FONT_SIZE
        ),
    ));
    v
}

fn planche_des_tailles() -> image::RgbaImage {
    let hauteur = tokens::WINDOW_BANNER_HEIGHT;
    let ecart = 26.0;
    let lignes = lignes();
    let nombre = lignes.len();
    let mut harness = Harness::new_ui(move |ui| {
        overlay_ui::style::apply(&ui.ctx().clone());
        for (i, (taille, version, legende)) in lignes.iter().enumerate() {
            let haut = i as f32 * (hauteur + ecart);
            let rect = Rect::from_min_size(
                ui.max_rect().min + Vec2::new(0.0, haut),
                Vec2::new(LARGEUR, hauteur),
            );
            bandeau(ui, rect, *taille, version);
            // Cote lisible sur l'image elle-même : la planche doit rester interprétable une fois
            // sortie de ce fichier.
            ui.painter().text(
                Pos2::new(rect.left(), rect.bottom() + 3.0),
                Align2::LEFT_TOP,
                legende,
                egui::FontId::proportional(12.0),
                Color32::from_gray(150),
            );
        }
    });
    harness.set_size(Vec2::new(LARGEUR, nombre as f32 * (hauteur + ecart) + 6.0));
    harness.run();
    harness.render().expect("rendu de la planche")
}

fn fenetre_reelle() -> image::RgbaImage {
    let mut harness = Harness::new_ui(move |ui| {
        overlay_ui::style::apply(&ui.ctx().clone());
        let chrome = design::window("Options")
            .footer("Annuler", "Valider")
            .close_button(true)
            .version(true)
            .log_name("planche.version")
            .show(ui);
        design::panel().show(ui, chrome.content, |ui, _panel| {
            ui.add(design::heading("Fichier"));
        });
    });
    harness.set_size(Vec2::new(LARGEUR, 260.0));
    harness.run();
    harness.render().expect("rendu de la fenêtre")
}

fn main() {
    let dir = dossier_sortie();
    for (image, nom) in [
        (planche_des_tailles(), "banniere-version-tailles.png"),
        (fenetre_reelle(), "banniere-version-fenetre.png"),
    ] {
        let chemin = dir.join(nom);
        image.save(&chemin).expect("écriture du PNG");
        println!("écrit : {}", chemin.display());
    }
}
