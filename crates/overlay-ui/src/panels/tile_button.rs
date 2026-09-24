//! **Le bouton d'une tuile** — un glyphe révélé au survol, sur le disque qui dit « ceci est un
//! bouton » ou nu, partagé par les écrans qui posent une action sur une tuile.
//!
//! Né dans `panels::personnages_tab` (2026-09-16) pour le crayon de modification d'une carte de
//! héros, et extrait ici le 2026-09-18 quand le bandeau in-game a voulu le même bouton, au même
//! endroit, pour réinitialiser le compteur d'une tuile suivie (`panels::watchlist::entry_tile`) :
//! « avec le même design que l'icône bouton "modifier" ». Deux copies de cette géométrie auraient
//! divergé au premier ajustement — le socle a déjà changé de fond une fois.
//!
//! **Le socle n'est pas pour tous** (décision du 2026-09-16) : il porte le bouton posé au CENTRE
//! d'une tuile, où rien d'autre ne signalerait qu'on peut cliquer. La croix de retrait des tuiles
//! d'Alertes, de Chat et de Personnages garde le glyphe nu — c'est l'idiome de l'application, et
//! l'entourer d'un disque l'aurait mise en avant alors qu'elle est le geste qu'on ne veut PAS faire
//! par mégarde. D'où [`disc_button`] et [`glyph_button`], et non un booléen.
//!
//! **L'infobulle reste à l'appelant** : chaque écran a sa règle de placement (ancrée sur la tuile
//! dans l'onglet Personnages, en dessous du bandeau in-game), et deux infobulles ne doivent jamais
//! s'ouvrir ensemble — c'est l'appelant qui sait ce qu'il y a d'autre à côté.

use egui::{Color32, Pos2, Rect, Vec2};

use crate::design::{self, DsIcon};

/// Côté du glyphe d'un badge révélé au survol.
pub(crate) const BADGE: f32 = 14.0;
/// Diamètre du socle rond qui porte le bouton central d'une tuile.
pub(crate) const BADGE_DISC: f32 = 26.0;
/// Fond du socle — assez opaque pour détacher le glyphe de ce qu'il recouvre (un buste, une icône
/// d'objet), assez sombre pour rester du jeu.
const BADGE_DISC_FILL: Color32 = Color32::from_black_alpha(0xB4);
/// Trait du socle au repos — le cadre éclairci d'une tuile survolée
/// (`personnages_tab::TILE_BORDER_HOVER`), pour que le disque appartienne à la tuile qu'il
/// surmonte. Au survol du bouton lui-même, l'or (`tokens::TEXT_GOLD`), comme son glyphe.
pub(crate) const HOVER_BORDER: Color32 = Color32::from_rgb(0x9A, 0x8C, 0x6E);

/// Un glyphe sur son disque, cliquable — le bouton central d'une tuile. Voir la doc de module.
///
/// Main sous le pointeur : la tuile en dessous n'en affiche pas (elle se déplace, ou se coche),
/// c'est donc ici que le geste se signale.
pub(crate) fn disc_button(
    ui: &mut egui::Ui,
    center: Pos2,
    icon: DsIcon,
    id: egui::Id,
) -> egui::Response {
    button(ui, center, icon, true, id)
}

/// Un glyphe nu, cliquable — la croix de retrait d'une tuile. Voir la doc de module.
pub(crate) fn glyph_button(
    ui: &mut egui::Ui,
    center: Pos2,
    icon: DsIcon,
    id: egui::Id,
) -> egui::Response {
    button(ui, center, icon, false, id)
}

fn button(
    ui: &mut egui::Ui,
    center: Pos2,
    icon: DsIcon,
    socle: bool,
    id: egui::Id,
) -> egui::Response {
    let disc = Rect::from_center_size(center, Vec2::splat(BADGE_DISC));
    let response = ui
        .interact(disc, id, egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let survol = response.contains_pointer();
    if socle {
        let painter = ui.painter();
        painter.circle_filled(center, BADGE_DISC / 2.0, BADGE_DISC_FILL);
        painter.circle_stroke(
            center,
            BADGE_DISC / 2.0,
            egui::Stroke::new(
                1.0,
                if survol {
                    design::tokens::TEXT_GOLD
                } else {
                    HOVER_BORDER
                },
            ),
        );
    }
    paint_glyph(
        ui,
        center,
        icon,
        BADGE,
        if survol {
            design::tokens::TEXT_GOLD
        } else {
            design::tokens::ICON_TINT
        },
    );
    response
}

/// Un glyphe du design system peint à l'échelle demandée, sans socle — le « + » de la tuile
/// d'ajout de l'onglet Personnages, ou le glyphe d'un bouton de tuile.
pub(crate) fn paint_glyph(ui: &egui::Ui, center: Pos2, icon: DsIcon, side: f32, tint: Color32) {
    let ds = design::DesignSystem::get(ui.ctx());
    let native = ds.icon_native_size(icon);
    let fit = native.x.max(native.y);
    ds.paint_icon(
        ui.painter(),
        Rect::from_center_size(center, native * (side / fit)),
        icon,
        tint,
    );
}
