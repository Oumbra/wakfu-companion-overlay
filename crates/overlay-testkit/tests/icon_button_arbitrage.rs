//! **Banc d'arbitrage du bouton icône** — la planche qui manquait pour décider si le carré de
//! contrôle du panneau Suivi peut passer de `panels::icon_button::paint_icon_button` à
//! `design::icon_button`.
//!
//! `design::icon_button` existe, il est complet et testé, mais **aucun des quatre boutons icône de
//! l'overlay ne l'utilise** : la doc du composant explique que leurs icônes diffèrent (celles
//! d'`ui_icons` sont normalisées au chargement, celles du design system sont les glyphes détourés
//! bruts) et qu'« aucune capture de référence ne permet d'arbitrer laquelle des deux normalisations
//! est la bonne ».
//!
//! Cette capture-là existe pourtant : `assets/design-system/menu-button-icon-first-plan.png`. Elle
//! est déjà la source des deux teintes d'icône du composant, mais personne n'y avait mesuré la
//! **taille d'encre**. Mesuré ici (seuil de luminance 140, invariant de 120 à 180) : les huit
//! icônes de cette barre ont une bbox opaque de **16 à 20 px pour un socle de 36** — médiane 18,
//! moyenne 17,75 — et toutes sont centrées au pixel près sur l'axe du socle. Le socle de la capture
//! est bien à l'échelle 1 : `button-icon-first-plan.png` s'y recale à 36 × 36 avec un écart moyen
//! de 2,3/255 sur les pixels de socle, contre 4,5 et plus dès 35 ou 37.
//!
//! De quoi juger les trois candidats, plutôt que de continuer à repousser :
//!
//! | Variante | Icônes | Taille du socle |
//! | --- | --- | --- |
//! | A — aujourd'hui | `ui_icons`, normalisées à 16 (18 pour le rouage) | 24 |
//! | B — design system tel quel | glyphes détourés bruts (13, 16, 14, 14) | 24 |
//! | C — design system + `icon_content` | ramenées à 18, l'étalon mesuré sur le jeu | 24 |
//! | D — design system tel quel, socle natif | glyphes bruts | 36 |
//!
//! D n'est pas une proposition d'interface : c'est le témoin qui montre ce que la réduction à 24
//! coûte au trait le plus fin (le « − » du design system fait 2 px de haut, soit 1,3 px une fois
//! réduit).
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use egui::{Color32, RichText, Sense, Vec2};
use egui_kittest::Harness;
use overlay_ui::design::{self, DsTexture, IconButtonState, IconContext};
use overlay_ui::panels::icon_button::{paint_icon_button, PANEL_BACKDROP_FILL};
use overlay_ui::ui_icons::UiIcons;

/// Fond de la planche — le fond de panneau du jeu, comme la galerie du design system : les socles
/// sont détourés, ils se jugent sur le fond sur lequel ils vivront.
const PAGE_FILL: Color32 = Color32::from_rgb(0x18, 0x18, 0x20);
const HEADING: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9F);
const CAPTION: Color32 = Color32::from_rgb(0x9A, 0xA0, 0xA6);

/// Côté d'un bouton du carré de contrôle du Suivi (`watchlist::CONTROL_BUTTON_SIZE`, privé) et
/// gouttière entre deux boutons (`CONTROL_BUTTON_GAP`) — recopiés ici parce que le banc reproduit
/// ce carré à l'identique ; ils ne servent qu'à cette planche.
const CONTROL_BUTTON_SIZE: f32 = 24.0;
const CONTROL_BUTTON_GAP: f32 = 4.0;
/// Taille native du socle (`design::tokens::ICON_BUTTON_SIZE`) — la variante témoin D.
const NATIVE_SIZE: f32 = 36.0;
/// Étalon mesuré sur `menu-button-icon-first-plan.png` (voir doc de module) : médiane de la plus
/// grande dimension de bbox des huit icônes, pour un socle de 36.
const GAME_ICON_CONTENT: f32 = 18.0;

/// L'état peint dans une case de la planche. Aucun pointeur ne survole quoi que ce soit en rendu
/// offscreen : les trois états se posent, ils ne s'obtiennent pas.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    Idle,
    Hovered,
    Disabled,
}

#[test]
fn arbitrage_du_bouton_icone() {
    let mut icons: Option<UiIcons> = None;
    let mut harness = Harness::builder()
        .with_size(Vec2::new(620.0, 700.0))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            let ctx = ui.ctx().clone();
            let icons = icons.get_or_insert_with(|| UiIcons::load(&ctx));
            egui::Frame::NONE
                .fill(PAGE_FILL)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    board(ui, icons);
                });
        });

    harness.run();
    harness.snapshot("icon_button_arbitrage");
}

fn board(ui: &mut egui::Ui, icons: &UiIcons) {
    ui.spacing_mut().item_spacing = Vec2::new(18.0, 8.0);

    row(
        ui,
        "A — aujourd'hui : panels::icon_button",
        "icônes ui_icons normalisées au chargement (contenu 16, 18 pour le rouage) · socle 24",
        |ui, cell| legacy_square(ui, icons, CONTROL_BUTTON_SIZE, cell),
    );
    row(
        ui,
        "B — design::icon_button tel quel",
        "glyphes détourés bruts : 13, 16, 14, 14 · socle 24",
        |ui, cell| ds_square(ui, CONTROL_BUTTON_SIZE, cell, None),
    );
    row(
        ui,
        "C — design::icon_button + icon_content(18)",
        "toutes les icônes à l'étalon mesuré sur le jeu · socle 24",
        |ui, cell| ds_square(ui, CONTROL_BUTTON_SIZE, cell, Some(GAME_ICON_CONTENT)),
    );
    row(
        ui,
        "D — témoin : B au socle natif",
        "mêmes glyphes bruts, socle 36 — ce que la réduction à 24 coûte au trait du « − »",
        |ui, cell| ds_square(ui, NATIVE_SIZE, cell, None),
    );
}

/// Une variante : son titre, sa légende, puis le carré de contrôle du Suivi dans les trois états.
fn row(ui: &mut egui::Ui, title: &str, caption: &str, mut square: impl FnMut(&mut egui::Ui, Cell)) {
    ui.add_space(10.0);
    ui.label(RichText::new(title).color(HEADING).size(15.0).strong());
    ui.label(RichText::new(caption).color(CAPTION).size(12.0));
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        for (cell, label) in [
            (Cell::Idle, "repos"),
            (Cell::Hovered, "survolé"),
            (Cell::Disabled, "désactivé"),
        ] {
            ui.vertical(|ui| {
                square(ui, cell);
                ui.label(RichText::new(label).color(CAPTION).size(11.0));
            });
        }
    });
}

/// Le fond translucide du carré de contrôle (`watchlist::control_button_row`) et les quatre
/// positions de bouton, dans l'ordre de la production : `[+] [−]` puis `[🔗] [⚙]`.
fn square_rects(ui: &mut egui::Ui, size: f32) -> [egui::Rect; 4] {
    let side = size * 2.0 + CONTROL_BUTTON_GAP * 3.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
    ui.painter().rect_filled(
        rect,
        overlay_ui::panels::icon_button::PANEL_BACKDROP_ROUNDING,
        PANEL_BACKDROP_FILL,
    );
    let origin = rect.min + Vec2::splat(CONTROL_BUTTON_GAP);
    let step = size + CONTROL_BUTTON_GAP;
    [(0.0, 0.0), (step, 0.0), (0.0, step), (step, step)]
        .map(|(dx, dy)| egui::Rect::from_min_size(origin + Vec2::new(dx, dy), Vec2::splat(size)))
}

/// Variante A — le code de production, appelé tel quel.
///
/// L'état survolé ne peut pas se forcer : `paint_icon_button` le déduit de `Response::hovered()`,
/// et aucun pointeur n'existe en rendu offscreen. On lui passe donc les textures survolées **aux
/// deux emplacements** (repos et survol) : l'apparence peinte est exactement celle du survol, et la
/// mise à l'échelle comme le centrage restent ceux de la production — jamais une copie de sa
/// logique, qui aurait pu diverger sans qu'on le voie.
fn legacy_square(ui: &mut egui::Ui, icons: &UiIcons, size: f32, cell: Cell) {
    let rects = square_rects(ui, size);
    let glyphs = [
        ("plus", icons.icon_plus(), icons.icon_plus_hover()),
        ("minus", icons.icon_minus(), icons.icon_minus_hover()),
        (
            "link",
            icons.external_link_icon(),
            icons.external_link_icon_hover(),
        ),
        ("options", icons.options_icon(), icons.options_icon_hover()),
    ];
    for (rect, (name, idle, hovered)) in rects.into_iter().zip(glyphs) {
        let (icon, icon_hover) = match cell {
            Cell::Hovered => (hovered, hovered),
            _ => (idle, idle),
        };
        let (bg, bg_hover) = match cell {
            Cell::Hovered => (
                icons.button_background_hover(),
                icons.button_background_hover(),
            ),
            _ => (icons.button_background(), icons.button_background()),
        };
        paint_icon_button(
            ui,
            rect,
            &format!("arbitrage-legacy-{name}-{}", rect.min.x + rect.min.y),
            Sense::hover(),
            cell != Cell::Disabled,
            bg,
            bg_hover,
            icon,
            icon_hover,
        );
    }
}

/// Variantes B, C et D — le composant, avec ou sans normalisation d'icône.
fn ds_square(ui: &mut egui::Ui, size: f32, cell: Cell, content: Option<f32>) {
    let rects = square_rects(ui, size);
    let glyphs = [
        DsTexture::IconPlus,
        DsTexture::IconMinus,
        DsTexture::IconExternalLink,
        DsTexture::IconOption,
    ];
    let state = match cell {
        Cell::Idle => IconButtonState::Idle,
        Cell::Hovered => IconButtonState::Hovered,
        Cell::Disabled => IconButtonState::Disabled,
    };
    for (rect, glyph) in rects.into_iter().zip(glyphs) {
        let mut button = design::icon_button(glyph)
            .context(IconContext::FirstPlan)
            .size(size)
            .preview_state(state);
        if let Some(content) = content {
            button = button.icon_content(content);
        }
        ui.put(rect, button);
    }
}

/// **Détail sans texte**, pour l'agrandissement publié en Artifact : les trois candidats côte à
/// côte, un état par ligne, à la taille exacte de la production (socle 24). Un écart
/// d'un pixel sur un trait de « − » ne se juge pas sur une planche annotée à l'échelle 1 — cette
/// capture-ci est faite pour être agrandie au plus proche voisin, sans que du texte anticrénelé
/// vienne brouiller la lecture.
#[test]
fn arbitrage_du_bouton_icone_detail() {
    let mut icons: Option<UiIcons> = None;
    let mut harness = Harness::builder()
        .with_size(Vec2::new(216.0, 206.0))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            let ctx = ui.ctx().clone();
            let icons = icons.get_or_insert_with(|| UiIcons::load(&ctx));
            egui::Frame::NONE
                .fill(PAGE_FILL)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    ui.spacing_mut().item_spacing = Vec2::splat(8.0);
                    for cell in [Cell::Idle, Cell::Hovered, Cell::Disabled] {
                        ui.horizontal(|ui| {
                            legacy_square(ui, icons, CONTROL_BUTTON_SIZE, cell);
                            ds_square(ui, CONTROL_BUTTON_SIZE, cell, None);
                            ds_square(ui, CONTROL_BUTTON_SIZE, cell, Some(GAME_ICON_CONTENT));
                        });
                    }
                });
        });

    harness.run();
    harness.snapshot("icon_button_arbitrage_detail");
}
