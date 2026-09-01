//! Panneau "Dégâts du combat" — extrait de `main.rs::render` (L2), enrichi du portrait de classe
//! de chaque allié (roster déclaré par l'utilisateur, sinon `breed` du combat — voir
//! `overlay_engine::session` pour la cascade, `crate::portraits::PortraitAtlas` pour le rendu).
//!
//! **Refonte 2026-09-01** (retour utilisateur en test réel) : la liste n'affichait jusqu'ici que
//! les alliés, mélangés en dur avec un nom coloré et une barre de progression qui donnaient
//! l'impression d'un bandeau sombre derrière chaque ligne. Remplacé par une liste verticale de
//! PORTRAITS SEULS (le nom n'apparaît plus qu'au survol, en dessous du portrait — tooltip egui
//! standard), les dégâts alignés en face, un écart vertical entre chaque ligne, et un switch
//! Alliés/Ennemis au-dessus (`CombatSide`) — l'ennemi n'a pas de portrait de classe résolvable
//! (breed pas déterministe côté ennemi, voir `class_breed.rs`) donc n'était jusqu'ici jamais
//! affiché du tout ; il l'est maintenant avec un portrait générique (`UiIcons::unknown_entity_image`).

use overlay_engine::{FightResult, FightSnapshot};

use crate::portraits::PortraitAtlas;
use crate::ui_icons::UiIcons;

/// Camp actuellement affiché dans la liste verticale de portraits, piloté par le switch
/// (`side_switch`). Un état par fenêtre overlay (donc par personnage) — voir `OverlayWindow`
/// dans `main.rs`, pas un état global : rien n'empêche de vouloir regarder les ennemis d'un
/// personnage pendant que la fenêtre d'un autre reste sur ses alliés.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CombatSide {
    /// Vue par défaut (demande utilisateur explicite) : c'est aussi la seule où le portrait
    /// apporte une vraie information tant que les ennemis n'ont pas leur propre illustration
    /// (repli générique pour l'instant, voir `UiIcons`).
    #[default]
    Allies,
    Enemies,
}

const SWITCH_HEIGHT: f32 = 26.0;
const SWITCH_OPTION_WIDTH: f32 = 30.0;
const SWITCH_ICON_SIZE: f32 = 15.0;
/// Écart vertical entre deux portraits de la liste (demande utilisateur explicite).
const ROW_GAP: f32 = 6.0;

// Charte reprise telle quelle du thème sombre par défaut du dépôt web (`styles.css` `:root`, voir
// `.icon-switch`/`.icon-switch-highlight`) — pas de palette propre à l'overlay pour ce composant.
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x00, 0xd2, 0xff);
const TINT_MEDIUM: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 31);
const TINT_STRONG: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 46);

pub fn show(
    ui: &mut egui::Ui,
    fight: Option<&FightSnapshot>,
    portraits: &PortraitAtlas,
    icons: &UiIcons,
    side: &mut CombatSide,
) {
    ui.strong("Dégâts du combat");
    side_switch(ui, side, icons);
    ui.add_space(6.0);
    match fight {
        None => {
            ui.weak("Aucun combat pour l'instant.");
        }
        Some(fight) => {
            let mut fighters: Vec<_> = fight
                .fighters
                .iter()
                .filter(|f| f.is_ally == (*side == CombatSide::Allies))
                .collect();
            fighters.sort_by_key(|f| std::cmp::Reverse(f.total_damage));

            if fighters.is_empty() {
                ui.weak(match side {
                    CombatSide::Allies => "Aucun allié pour l'instant.",
                    CombatSide::Enemies => "Aucun ennemi pour l'instant.",
                });
            }

            for (i, fighter) in fighters.iter().enumerate() {
                if i > 0 {
                    ui.add_space(ROW_GAP);
                }
                ui.horizontal(|ui| {
                    // Portrait de classe pour un allié classifié ; repli générique sinon (ennemi,
                    // ou allié pas encore classifié — même image que pour un ennemi, faute de
                    // mieux tant qu'aucune classe n'est connue). Aucun fond derrière l'image :
                    // uniquement le portrait, comme demandé.
                    let class_portrait = fighter
                        .class_name
                        .as_deref()
                        .and_then(|class_name| portraits.image(class_name, fighter.gender));
                    let response = match class_portrait {
                        Some(image) => ui.add(image),
                        None => ui.add(icons.unknown_entity_image()),
                    };
                    response.on_hover_text(fighter.name.as_str());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.monospace(fighter.total_damage.to_string());
                    });
                });
            }

            let status = match fight.result {
                None => "en cours".to_string(),
                Some(FightResult::Won) => "gagné".to_string(),
                Some(FightResult::Lost) => "perdu".to_string(),
            };
            ui.add_space(4.0);
            ui.small(format!("Combat #{} — {status}", fight.fight_id));
        }
    }
}

/// Switch à deux icônes (alliés/ennemis) avec fond glissant — même mécanique que `.icon-switch` du
/// dépôt web (`styles.css`), portée en dessin egui direct (peintre + zones cliquables) puisqu'il
/// n'y a pas de CSS ici pour l'obtenir gratuitement.
fn side_switch(ui: &mut egui::Ui, side: &mut CombatSide, icons: &UiIcons) {
    let option_size = egui::vec2(SWITCH_OPTION_WIDTH, SWITCH_HEIGHT);
    let (rect, _response) = ui.allocate_exact_size(
        egui::vec2(option_size.x * 2.0, option_size.y),
        egui::Sense::hover(),
    );
    let allies_rect = egui::Rect::from_min_size(rect.min, option_size);
    let enemies_rect =
        egui::Rect::from_min_size(rect.min + egui::vec2(option_size.x, 0.0), option_size);

    let painter = ui.painter();
    painter.rect_filled(rect, 5.0, TINT_MEDIUM);
    painter.rect_stroke(
        rect,
        5.0,
        egui::Stroke::new(1.0, TINT_STRONG),
        egui::StrokeKind::Inside,
    );
    let highlight_rect = if *side == CombatSide::Allies {
        allies_rect
    } else {
        enemies_rect
    };
    painter.rect_filled(highlight_rect.shrink(1.0), 4.0, ACCENT);

    draw_centered_icon(ui, allies_rect, icons.allies());
    draw_centered_icon(ui, enemies_rect, icons.enemies());

    let allies_response = ui
        .interact(
            allies_rect,
            ui.id().with("combat-side-allies"),
            egui::Sense::click(),
        )
        .on_hover_text("Alliés");
    let enemies_response = ui
        .interact(
            enemies_rect,
            ui.id().with("combat-side-enemies"),
            egui::Sense::click(),
        )
        .on_hover_text("Ennemis");
    if allies_response.clicked() {
        *side = CombatSide::Allies;
    }
    if enemies_response.clicked() {
        *side = CombatSide::Enemies;
    }
}

fn draw_centered_icon(ui: &egui::Ui, rect: egui::Rect, texture: &egui::TextureHandle) {
    let icon_rect = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(SWITCH_ICON_SIZE, SWITCH_ICON_SIZE),
    );
    egui::Image::new(texture).paint_at(ui, icon_rect);
}
