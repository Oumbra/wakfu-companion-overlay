//! **Avant / après de la migration du carré de contrôle du Suivi** — les quatre boutons icône
//! passent de `panels::icon_button::paint_icon_button` à `design::icon_button`.
//!
//! Ce fichier a d'abord été le banc d'arbitrage qui a débloqué la migration. La question qu'il
//! posait — laquelle des deux façons de dimensionner une icône est la bonne — est tranchée : la
//! mesure est dans `tokens::ICON_BUTTON_CONTENT`, la décision utilisateur date du 2026-09-10
//! (« rendu C et le « − » à 1,7 px me convient totalement »). Il ne reste ici que ce qui sert à
//! juger le changement de rendu, **le temps que l'ancien chemin existe encore** : ces deux captures
//! disparaissent avec `paint_icon_button`.
//!
//! Ce qui bouge, et pourquoi :
//!
//! | | Avant | Après |
//! | --- | --- | --- |
//! | icônes | `ui_icons`, normalisées à 16 (18 pour le rouage) | manifeste, normalisées à 18 |
//! | « − » | barre pleine de 4 px | trait de 1,7 px, comme le jeu |
//! | socle désactivé | socle de repos assombri à 43 % | `button-icon-disabled.png`, le socle grisé du jeu |
//! | socles repos/survol | `assets/ui/button-background{,-hover}.png` | les mêmes fichiers, octet pour octet |
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
/// gouttière entre deux boutons (`CONTROL_BUTTON_GAP`) — recopiés ici parce que ces planches
/// reproduisent ce carré à l'identique ; ils ne servent qu'à elles.
const CONTROL_BUTTON_SIZE: f32 = 24.0;
const CONTROL_BUTTON_GAP: f32 = 4.0;

/// L'état peint dans une case. Aucun pointeur ne survole quoi que ce soit en rendu offscreen : les
/// trois états se posent, ils ne s'obtiennent pas.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    Idle,
    Hovered,
    Disabled,
}

#[test]
fn avant_apres_du_carre_de_controle() {
    let mut icons: Option<UiIcons> = None;
    let mut harness = Harness::builder()
        .with_size(Vec2::new(560.0, 340.0))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            let ctx = ui.ctx().clone();
            let icons = icons.get_or_insert_with(|| UiIcons::load(&ctx));
            egui::Frame::NONE
                .fill(PAGE_FILL)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    ui.spacing_mut().item_spacing = Vec2::new(18.0, 8.0);
                    row(
                        ui,
                        "Avant — panels::icon_button",
                        "icônes ui_icons normalisées à 16 (18 pour le rouage) · socle de repos assombri",
                        |ui, cell| legacy_square(ui, icons, cell),
                    );
                    row(
                        ui,
                        "Après — design::icon_button",
                        "icônes du manifeste normalisées à 18 · socle grisé du jeu",
                        ds_square,
                    );
                });
        });

    harness.run();
    harness.snapshot("icon_button_migration");
}

/// **Détail sans texte**, pour l'agrandissement publié en Artifact : avant en haut, après en bas,
/// à la taille exacte de la production. Un écart d'un pixel sur un trait de « − » ne se juge pas
/// sur une planche annotée à l'échelle 1 — cette capture-ci est faite pour être agrandie au plus
/// proche voisin, sans que du texte anticrénelé vienne brouiller la lecture.
#[test]
fn avant_apres_du_carre_de_controle_detail() {
    let mut icons: Option<UiIcons> = None;
    let mut harness = Harness::builder()
        .with_size(Vec2::new(216.0, 152.0))
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
                    for square in [
                        &mut (|ui: &mut egui::Ui, cell| legacy_square(ui, icons, cell))
                            as &mut dyn FnMut(&mut egui::Ui, Cell),
                        &mut ds_square,
                    ] {
                        ui.horizontal(|ui| {
                            for cell in [Cell::Idle, Cell::Hovered, Cell::Disabled] {
                                square(ui, cell);
                            }
                        });
                    }
                });
        });

    harness.run();
    harness.snapshot("icon_button_migration_detail");
}

/// Une ligne : son titre, sa légende, puis le carré de contrôle dans les trois états.
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
/// positions de bouton, dans l'ordre de la production : `[+] [−]` puis `[Détails] [Options]`.
fn square_rects(ui: &mut egui::Ui) -> [egui::Rect; 4] {
    let side = CONTROL_BUTTON_SIZE * 2.0 + CONTROL_BUTTON_GAP * 3.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
    ui.painter().rect_filled(
        rect,
        overlay_ui::panels::icon_button::PANEL_BACKDROP_ROUNDING,
        PANEL_BACKDROP_FILL,
    );
    let origin = rect.min + Vec2::splat(CONTROL_BUTTON_GAP);
    let step = CONTROL_BUTTON_SIZE + CONTROL_BUTTON_GAP;
    [(0.0, 0.0), (step, 0.0), (0.0, step), (step, step)].map(|(dx, dy)| {
        egui::Rect::from_min_size(origin + Vec2::new(dx, dy), Vec2::splat(CONTROL_BUTTON_SIZE))
    })
}

/// **Avant** — le code sortant, appelé tel quel.
///
/// L'état survolé ne peut pas se forcer : `paint_icon_button` le déduit de `Response::hovered()`,
/// et aucun pointeur n'existe en rendu offscreen. On lui passe donc les textures survolées **aux
/// deux emplacements** (repos et survol) : l'apparence peinte est exactement celle du survol, et la
/// mise à l'échelle comme le centrage restent ceux de la production — jamais une copie de sa
/// logique, qui aurait pu diverger sans qu'on le voie.
fn legacy_square(ui: &mut egui::Ui, icons: &UiIcons, cell: Cell) {
    let rects = square_rects(ui);
    let glyphs = [
        ("plus", icons.icon_plus(), icons.icon_plus_hover()),
        ("minus", icons.icon_minus(), icons.icon_minus_hover()),
        (
            "details",
            icons.external_link_icon(),
            icons.external_link_icon_hover(),
        ),
        ("options", icons.options_icon(), icons.options_icon_hover()),
    ];
    for (rect, (name, idle, hovered)) in rects.into_iter().zip(glyphs) {
        let icon = match cell {
            Cell::Hovered => hovered,
            _ => idle,
        };
        let background = match cell {
            Cell::Hovered => icons.button_background_hover(),
            _ => icons.button_background(),
        };
        paint_icon_button(
            ui,
            rect,
            &format!("migration-avant-{name}-{}", rect.min.x + rect.min.y),
            Sense::hover(),
            cell != Cell::Disabled,
            background,
            background,
            icon,
            icon,
        );
    }
}

/// **Après** — le composant, tel que le panneau l'appellera.
fn ds_square(ui: &mut egui::Ui, cell: Cell) {
    let rects = square_rects(ui);
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
        ui.put(
            rect,
            design::icon_button(glyph)
                .context(IconContext::FirstPlan)
                .size(CONTROL_BUTTON_SIZE)
                .preview_state(state),
        );
    }
}
