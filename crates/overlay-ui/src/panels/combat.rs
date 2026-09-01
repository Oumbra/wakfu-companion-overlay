//! Panneau "Dégâts du combat" — extrait de `main.rs::render` (L2), enrichi du portrait de classe
//! de chaque allié (roster déclaré par l'utilisateur, sinon `breed` du combat — voir
//! `overlay_engine::session` pour la cascade, `crate::portraits::PortraitAtlas` pour le rendu).
//!
//! **Refonte 2026-09-01** (retour utilisateur en test réel, capture d'écran à l'appui) : la liste
//! n'affichait jusqu'ici que les alliés, mélangés en dur avec un nom coloré et une barre de
//! progression qui donnaient l'impression d'un bandeau sombre derrière chaque ligne. Remplacée par
//! une liste verticale de PORTRAITS (le nom n'apparaît qu'au survol — tooltip egui standard) + une
//! barre de dégâts par ligne (voir `damage_bar` : fond sombre, remplissage bleu proportionnel au
//! max de dégâts du camp affiché — même bleu que le switch — et le chiffre peint DANS la barre,
//! seul moyen de rester lisible par-dessus le jeu quel que soit le décor derrière, contrairement
//! au chiffre flottant nu de la toute première version). Switch Alliés/Ennemis au-dessus
//! (`CombatSide`) — l'ennemi n'a pas de portrait de classe résolvable (breed pas déterministe côté
//! ennemi, voir `class_breed.rs`) donc n'était jusqu'ici jamais affiché du tout ; il l'est
//! maintenant avec un portrait générique (`UiIcons::unknown_entity_image`). Ligne "Combat #N —
//! gagné/perdu" retirée (retour utilisateur : n'apporte rien).

use overlay_engine::{CatalogIndex, FightSnapshot};

use crate::portraits::PortraitAtlas;
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
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

/// Hauteur de la barre de dégâts d'une ligne — plus fine que le portrait (`PORTRAIT_SIZE`),
/// centrée verticalement à côté de lui (alignement par défaut de `ui.horizontal`).
const BAR_HEIGHT: f32 = 18.0;
/// Fond de la barre : opaque et TRÈS sombre plutôt qu'un simple gris — c'est lui qui garantit que
/// le chiffre reste lisible par-dessus n'importe quel décor de jeu, quelle que soit la proportion
/// remplie (voir `damage_bar`). Même famille que le fond de l'ancien panneau (`18, 20, 28`), en
/// plus sombre et plus opaque : ce fond-ci est volontaire (demande explicite d'une barre), à ne
/// pas confondre avec le fond de panneau plein retiré par ailleurs.
const BAR_BG: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(8, 10, 16, 235);
/// Remplissage proportionnel : le même bleu que le switch (`ACCENT`), mais semi-transparent plutôt
/// qu'opaque — demande explicite ("une couleur sombre et par-dessus le même bleu que le switch").
/// Un `ACCENT` opaque plein rendrait le chiffre illisible une fois dessus (bleu clair = mauvais
/// contraste avec un texte clair) ; en semi-transparent sur `BAR_BG`, le mélange reste assez sombre
/// pour que le texte clair de `BAR_TEXT` garde un contraste correct sur toute la largeur de la
/// barre, remplie ou non.
const BAR_FILL: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(0x00, 0xd2, 0xff, 110);
const BAR_TEXT: egui::Color32 = egui::Color32::from_rgb(235, 240, 245);
/// Copie du texte peinte 1px en dessous/à droite avant le texte principal (pseudo-contour) — filet
/// de sécurité supplémentaire aux endroits où le remplissage bleu est le plus clair.
const BAR_TEXT_SHADOW: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(0, 0, 0, 200);

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    fight: Option<&FightSnapshot>,
    portraits: &PortraitAtlas,
    icons: &UiIcons,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    side: &mut CombatSide,
) {
    // Pas de titre "Dégâts du combat" (retour utilisateur 2026-09-01 : n'apporte rien, retiré) —
    // le switch Alliés/Ennemis en tête suffit à situer ce que montre la liste.
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

            // Sert de référence 100% à la barre de chaque ligne (voir `damage_bar`) — celui qui
            // inflige le plus de dégâts DANS LE CAMP AFFICHÉ, pas tous combattants confondus :
            // sinon la barre du camp le plus faible resterait quasi vide en permanence.
            let max_damage = fighters.first().map_or(1, |f| f.total_damage).max(1);

            for (i, fighter) in fighters.iter().enumerate() {
                if i > 0 {
                    ui.add_space(ROW_GAP);
                }
                ui.horizontal(|ui| {
                    // Portrait de classe pour un allié classifié. Sinon (ennemi, ou allié pas
                    // encore classifié) : portrait RÉEL du monstre si le catalogue le résout par
                    // nom (retour utilisateur 2026-09-02 — le catalogue existe désormais, voir
                    // `panels::watchlist` pour le même mécanisme), repli générique tant qu'il n'a
                    // pas fini de télécharger ou si le nom n'est pas reconnu. Aucun fond derrière
                    // l'image : uniquement le portrait, comme demandé.
                    let class_portrait = fighter
                        .class_name
                        .as_deref()
                        .and_then(|class_name| portraits.image(class_name, fighter.gender));
                    let remote_monster_texture = class_portrait.is_none().then(|| {
                        catalog
                            .find_monster_icon(&fighter.name, None)
                            .and_then(|icon_ref| {
                                remote_icon_textures.resolve(ui.ctx(), remote_icons, &icon_ref)
                            })
                    });
                    let response = match (class_portrait, remote_monster_texture.flatten()) {
                        (Some(image), _) => ui.add(image),
                        (None, Some(texture)) => ui.add(
                            egui::Image::new(&texture)
                                .fit_to_exact_size(egui::vec2(
                                    crate::portraits::PORTRAIT_SIZE,
                                    crate::portraits::PORTRAIT_SIZE,
                                ))
                                .maintain_aspect_ratio(false),
                        ),
                        (None, None) => ui.add(icons.unknown_entity_image()),
                    };
                    response.on_hover_text(fighter.name.as_str());

                    damage_bar(ui, ui.available_width(), fighter.total_damage, max_damage);
                });
            }
        }
    }
}

/// Barre de dégâts d'une ligne : fond sombre pleine largeur (garantit la lisibilité du chiffre
/// quel que soit le décor de jeu derrière), rempli d'une proportion `damage / max_damage` dans le
/// bleu du switch (`BAR_FILL`), chiffre peint DANS la barre (pas à côté, en clair : la barre EST
/// le fond du chiffre — demande utilisateur explicite). Centrée verticalement à côté du portrait
/// par le layout par défaut de `ui.horizontal` (`Align::Center`).
fn damage_bar(ui: &mut egui::Ui, width: f32, damage: i64, max_damage: i64) {
    let width = width.max(1.0);
    let (rect, _response) =
        ui.allocate_exact_size(egui::vec2(width, BAR_HEIGHT), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, BAR_BG);

    let ratio = (damage as f32 / max_damage as f32).clamp(0.0, 1.0);
    if ratio > 0.0 {
        let fill_rect =
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * ratio, rect.height()));
        painter.rect_filled(fill_rect, 4.0, BAR_FILL);
    }

    let text = damage.to_string();
    let font = egui::FontId::monospace(12.0);
    let text_pos = rect.right_center() - egui::vec2(6.0, 0.0);
    painter.text(
        text_pos + egui::vec2(1.0, 1.0),
        egui::Align2::RIGHT_CENTER,
        &text,
        font.clone(),
        BAR_TEXT_SHADOW,
    );
    painter.text(text_pos, egui::Align2::RIGHT_CENTER, &text, font, BAR_TEXT);
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
