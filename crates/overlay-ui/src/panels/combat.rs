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
//!
//! **Refonte 2026-09-03** (demande utilisateur, redesign complet) :
//! - Camp Alliés : fond décoratif par nombre d'alliés (`crate::panels::combat_frame::CombatFrame`,
//!   médaillons "totem" contenant les portraits de classe, voir sa doc de module) au-delà de 6
//!   alliés (aucun template ne va plus loin), les alliés excédentaires continuent dans le style
//!   "plat" ci-dessous plutôt que de faire échouer l'affichage.
//! - Camp Ennemis, et alliés excédentaires : liste "plate" inchangée dans son principe (portrait +
//!   barre par ligne), mais `damage_bar` peint désormais le NOM du combattant et son POURCENTAGE
//!   d'équivalence par rapport au total de dégâts du camp affiché (voir sa doc) — plus seulement
//!   le chiffre brut.
//! - Total de dégâts du camp affiché, affiché avant la liste (`ui.label` dédié).
//! - Portrait grisé (`FighterDamage::is_ko`, voir `overlay_engine::session`) pour tout combattant
//!   déjà mis KO au moins une fois ce combat — vrai niveau de gris précalculé pour les portraits de
//!   classe (`PortraitAtlas`), simple tint pour les icônes de repli/distantes (`grey_tint_if_ko`,
//!   voir sa doc — pas de version grisée précalculée pour celles-ci).

use overlay_engine::{CatalogIndex, FightSnapshot};

use crate::portraits::PortraitAtlas;
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::ui_icons::UiIcons;

use super::combat_frame::{CombatFrame, MAX_FRAME_SLOTS};

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
/// Écart vertical entre deux portraits de la liste "plate" (demande utilisateur explicite).
const ROW_GAP: f32 = 6.0;

// Charte reprise telle quelle du thème sombre par défaut du dépôt web (`styles.css` `:root`, voir
// `.icon-switch`/`.icon-switch-highlight`) — pas de palette propre à l'overlay pour ce composant.
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x00, 0xd2, 0xff);
const TINT_MEDIUM: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 31);
const TINT_STRONG: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 46);

/// Hauteur d'une barre de dégâts dans la liste "plate" (ennemis, ou alliés au-delà de
/// `MAX_FRAME_SLOTS`) — voir `combat_frame::BAR_ROW_HEIGHT` pour la variante dessinée à côté d'un
/// médaillon du cadre, légèrement plus haute pour rester proportionnée à son diamètre.
const BAR_HEIGHT: f32 = 24.0;

/// Couleurs de la barre de dégâts — mesurées sur la maquette fournie par l'utilisateur (capture
/// d'écran jointe à la demande de redesign, 2026-09-03) plutôt que reprises de `ACCENT` (le bleu du
/// switch, qui habillait l'ancienne barre) : la maquette est un dégradé sarcelle/turquoise distinct,
/// pas une déclinaison de `ACCENT`.
const BAR_BORDER: egui::Color32 = egui::Color32::from_rgb(30, 31, 37);
const BAR_TRACK: egui::Color32 = egui::Color32::from_rgb(22, 23, 28);
/// Base du remplissage (bas de la barre) — voir `BAR_FILL_HIGHLIGHT` pour le reflet du haut.
const BAR_FILL_BASE: egui::Color32 = egui::Color32::from_rgb(7, 121, 130);
/// Reflet plus clair peint sur le tiers supérieur du remplissage — effet "verre/glacis" mesuré sur
/// la maquette (bande nettement plus claire que `BAR_FILL_BASE` juste sous le bord supérieur).
const BAR_FILL_HIGHLIGHT: egui::Color32 = egui::Color32::from_rgb(13, 190, 190);
/// Ligne de biseau (haut et bas de la piste, qu'elle soit remplie ou non à cet endroit) — un blanc
/// translucide fin, mesuré comme un éclaircissement d'environ +40 sur la piste sombre.
const BAR_BEVEL: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 40);
const BAR_TEXT: egui::Color32 = egui::Color32::from_rgb(235, 240, 245);
/// Copie du texte peinte 1px en dessous/à droite avant le texte principal (pseudo-contour) — filet
/// de sécurité supplémentaire aux endroits où le remplissage clair est le plus proche du texte.
const BAR_TEXT_SHADOW: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(0, 0, 0, 200);

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    fight: Option<&FightSnapshot>,
    portraits: &PortraitAtlas,
    frame: &CombatFrame,
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
                return;
            }

            // Dénominateurs des deux barres, calculés UNE FOIS sur tout le camp affiché — voir
            // `damage_bar` : `max_damage` pilote le remplissage (proportionnel au plus gros dégât
            // du camp), `total_damage` pilote le pourcentage affiché (part de chacun dans le total
            // du camp). Partagés entre le cadre et le repli "plat" pour qu'un allié affiché dans
            // l'un ou l'autre selon sa position (voir `MAX_FRAME_SLOTS`) reste cohérent avec ses
            // voisins.
            let max_damage = fighters.first().map_or(1, |f| f.total_damage).max(1);
            let total_damage = fighters.iter().map(|f| f.total_damage).sum::<i64>().max(1);

            // Total de dégâts du camp affiché — demande utilisateur explicite, affiché AVANT la
            // liste des barres (pas dans la liste elle-même).
            ui.label(
                egui::RichText::new(format!("Total : {total_damage}"))
                    .strong()
                    .size(13.0),
            );
            ui.add_space(4.0);

            let (framed, flat): (&[&_], &[&_]) = if *side == CombatSide::Allies {
                fighters.split_at(fighters.len().min(MAX_FRAME_SLOTS))
            } else {
                (&[], &fighters)
            };

            if !framed.is_empty() {
                frame.show(ui, portraits, icons, framed, max_damage, total_damage);
                if !flat.is_empty() {
                    ui.add_space(ROW_GAP);
                }
            }

            for (i, fighter) in flat.iter().enumerate() {
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
                    let class_portrait = fighter.class_name.as_deref().and_then(|class_name| {
                        portraits.image(class_name, fighter.gender, fighter.is_ko)
                    });
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
                                .maintain_aspect_ratio(false)
                                // Pas de version grisée précalculée pour une icône distante (voir
                                // doc de module) : simple tint, approximation acceptée.
                                .tint(grey_tint_if_ko(fighter.is_ko)),
                        ),
                        (None, None) => ui.add(
                            icons
                                .unknown_entity_image()
                                .tint(grey_tint_if_ko(fighter.is_ko)),
                        ),
                    };
                    response.on_hover_text(fighter.name.as_str());

                    let bar_rect = ui
                        .allocate_exact_size(
                            egui::vec2(ui.available_width(), BAR_HEIGHT),
                            egui::Sense::hover(),
                        )
                        .0;
                    damage_bar(
                        ui,
                        bar_rect,
                        &fighter.name,
                        fighter.total_damage,
                        max_damage,
                        total_damage,
                    );
                });
            }
        }
    }
}

/// Tint à appliquer à une icône de repli/distante pour approximer un grisé KO — voir la doc de
/// module pour pourquoi ce n'est PAS utilisé sur les portraits de classe (`PortraitAtlas` en
/// précalcule une vraie version en niveaux de gris). Un tint multiplie chaque canal de couleur par
/// le tint : un gris moyen assombrit uniformément sans désaturer la teinte d'origine — moins bon
/// qu'un vrai niveau de gris, mais suffisant pour un repli rarement affiché (allié pas encore
/// classifié avec un cadre visible, ou une icône réseau qui n'a de toute façon pas d'équivalent
/// niveaux de gris préchargé).
pub(crate) fn grey_tint_if_ko(is_ko: bool) -> egui::Color32 {
    if is_ko {
        egui::Color32::from_gray(130)
    } else {
        egui::Color32::WHITE
    }
}

/// Barre de dégâts d'une ligne, peinte dans `rect` (déjà alloué par l'appelant — voir
/// `panels::combat_frame::CombatFrame::show` pour la variante "cadre", et la boucle ci-dessus pour
/// la variante "plate") : fond sombre biseauté, rempli d'un dégradé sarcelle/turquoise dans une
/// proportion `damage / max_damage` (référence = plus gros dégât du camp affiché, PAS le total :
/// c'est ce qui donne sa longueur visuelle à chaque barre), nom à gauche et pourcentage
/// `damage / total_damage` centré — deux dénominateurs volontairement distincts, voir la doc de
/// `show` ci-dessus. Couleurs : voir `BAR_BORDER`/`BAR_TRACK`/`BAR_FILL_BASE`/`BAR_FILL_HIGHLIGHT`/
/// `BAR_BEVEL`, mesurées sur la maquette fournie par l'utilisateur.
pub(crate) fn damage_bar(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    name: &str,
    damage: i64,
    max_damage: i64,
    total_damage: i64,
) {
    let painter = ui.painter().with_clip_rect(rect);
    let rounding = (rect.height() * 0.3).min(8.0);
    painter.rect_filled(rect, rounding, BAR_BORDER);

    const BORDER_WIDTH: f32 = 2.0;
    let track_rect = rect.shrink(BORDER_WIDTH);
    let inner_rounding = (rounding - BORDER_WIDTH).max(0.0);
    painter.rect_filled(track_rect, inner_rounding, BAR_TRACK);

    let ratio = (damage as f32 / max_damage as f32).clamp(0.0, 1.0);
    if ratio > 0.0 {
        let fill_rect = egui::Rect::from_min_size(
            track_rect.min,
            egui::vec2(track_rect.width() * ratio, track_rect.height()),
        );
        // Coins non arrondis (voir doc de fonction) : négligeable, seul le bord droit — jamais
        // visible au repos puisqu'il tombe presque toujours à l'intérieur de la piste arrondie.
        painter.rect_filled(fill_rect, 0.0, BAR_FILL_BASE);
        let highlight_rect = egui::Rect::from_min_size(
            fill_rect.min,
            egui::vec2(fill_rect.width(), fill_rect.height() * 0.4),
        );
        painter.rect_filled(highlight_rect, 0.0, BAR_FILL_HIGHLIGHT);
    }

    // Biseau haut/bas — fin liseré clair, présent qu'il y ait remplissage ou non à cet endroit
    // (voir doc de module).
    let top_bevel = egui::Rect::from_min_size(
        track_rect.min + egui::vec2(0.0, 1.0),
        egui::vec2(track_rect.width(), 1.0),
    );
    let bottom_bevel = egui::Rect::from_min_size(
        track_rect.left_bottom() + egui::vec2(0.0, -2.0),
        egui::vec2(track_rect.width(), 1.0),
    );
    painter.rect_filled(top_bevel, 0.0, BAR_BEVEL);
    painter.rect_filled(bottom_bevel, 0.0, BAR_BEVEL);

    let name_font = egui::FontId::proportional(12.0);
    let name_pos = rect.left_center() + egui::vec2(8.0, 0.0);
    painter.text(
        name_pos + egui::vec2(1.0, 1.0),
        egui::Align2::LEFT_CENTER,
        name,
        name_font.clone(),
        BAR_TEXT_SHADOW,
    );
    painter.text(
        name_pos,
        egui::Align2::LEFT_CENTER,
        name,
        name_font,
        BAR_TEXT,
    );

    let percent = ((damage as f64 / total_damage as f64) * 100.0).round() as i64;
    let percent_text = format!("{percent}%");
    let percent_font = egui::FontId::proportional(12.0);
    let percent_pos = rect.center();
    painter.text(
        percent_pos + egui::vec2(1.0, 1.0),
        egui::Align2::CENTER_CENTER,
        &percent_text,
        percent_font.clone(),
        BAR_TEXT_SHADOW,
    );
    painter.text(
        percent_pos,
        egui::Align2::CENTER_CENTER,
        &percent_text,
        percent_font,
        BAR_TEXT,
    );
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
