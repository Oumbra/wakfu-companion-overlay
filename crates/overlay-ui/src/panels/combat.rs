//! Panneau "Dégâts du combat" — extrait de `main.rs::render` (L2), enrichi du portrait de classe
//! de chaque allié (roster déclaré par l'utilisateur, sinon `breed` du combat — voir
//! `overlay_engine::session` pour la cascade, `crate::portraits::PortraitAtlas` pour le rendu).
//!
//! **Refonte 2026-09-01** (retour utilisateur en test réel, capture d'écran à l'appui) : la liste
//! n'affichait jusqu'ici que les alliés, mélangés en dur avec un nom coloré et une barre de
//! progression qui donnaient l'impression d'un bandeau sombre derrière chaque ligne. Remplacée par
//! une liste verticale de PORTRAITS (le nom n'apparaît qu'au survol — tooltip egui standard) + une
//! barre de dégâts par ligne. Switch Alliés/Ennemis au-dessus (`CombatSide`) — l'ennemi n'a pas de
//! portrait de classe résolvable (breed pas déterministe côté ennemi, voir `class_breed.rs`) donc
//! n'était jusqu'ici jamais affiché du tout ; il l'est maintenant avec un portrait générique
//! (`UiIcons::unknown_entity_image`). Ligne "Combat #N — gagné/perdu" retirée (retour utilisateur :
//! n'apporte rien).
//!
//! **Refonte 2026-09-03** (demande utilisateur, redesign complet) :
//! - Camp Alliés : fond décoratif par nombre d'alliés (`crate::panels::combat_frame::CombatFrame`,
//!   médaillons "totem" contenant les portraits de classe, voir sa doc de module) au-delà de 6
//!   alliés (aucun template ne va plus loin), les alliés excédentaires continuent dans le style
//!   "plat" ci-dessous plutôt que de faire échouer l'affichage.
//! - Camp Ennemis, et alliés excédentaires : liste "plate" (portrait + barre par ligne).
//! - Portrait grisé (`FighterDamage::is_ko`, voir `overlay_engine::session`) pour tout combattant
//!   déjà mis KO au moins une fois ce combat — vrai niveau de gris précalculé pour les portraits de
//!   classe (`PortraitAtlas`), simple tint pour les icônes de repli/distantes (`grey_tint_if_ko`,
//!   voir sa doc — pas de version grisée précalculée pour celles-ci).
//!
//! **Refonte 2026-09-04** (retour utilisateur, second redesign) — DÉCOUPLAGE portraits/barres :
//! - Les portraits (cadre ET liste "plate") restent dans l'ordre STABLE de `FightSnapshot::
//!   fighters` (voir sa doc) — ils ne sont PLUS triés par dégâts, pour ne plus changer de position
//!   d'une frame à l'autre. Chaque portrait porte désormais son pourcentage de dégâts en
//!   incrustation (bas-droite, voir `paint_portrait_percent`) plutôt qu'une barre associée par sa
//!   position.
//! - Les barres, elles, restent triées par dégâts décroissant, mais forment maintenant une colonne
//!   INDÉPENDANTE à droite des portraits (voir `show` ci-dessous) — nom au-dessus de sa barre
//!   (`damage_bar_group`), groupe compact, sans lien de position avec un portrait précis. Seuls les
//!   combattants ayant infligé au moins 1 dégât y figurent (retour utilisateur : des barres à 0%
//!   pour tout le monde n'apportait rien, ne faisait que polluer l'overlay).
//! - Un portrait qui n'a plus "sa" barre juste à côté redevient difficile à identifier : infobulle
//!   au survol (nom) rétablie sur TOUS les portraits, cadre inclus (elle n'existait jusqu'ici que
//!   sur la liste "plate").
//! - Total de dégâts du camp affiché : replacé en tête de la colonne des BARRES (plus au-dessus de
//!   tout le panneau) — demande utilisateur explicite, et mis en valeur (couleur d'accent, taille
//!   plus grande) plutôt que du texte de statut ordinaire.

use overlay_engine::{CatalogIndex, FightSnapshot, FighterDamage};

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
/// Écart vertical entre deux portraits de la liste "plate", et entre deux groupes nom+barre de la
/// colonne de droite (même rythme pour les deux colonnes, demande utilisateur explicite).
const ROW_GAP: f32 = 6.0;
/// Écart horizontal entre la colonne des portraits (cadre ou liste plate) et celle des barres.
const COLUMN_GAP: f32 = 12.0;

// Charte reprise telle quelle du thème sombre par défaut du dépôt web (`styles.css` `:root`, voir
// `.icon-switch`/`.icon-switch-highlight`) — pas de palette propre à l'overlay pour ce composant.
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x00, 0xd2, 0xff);
const TINT_MEDIUM: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 31);
const TINT_STRONG: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 46);

/// Hauteur d'une barre de dégâts — réduite par rapport à l'ancien design (24 px) : le nom n'est
/// plus peint DANS la barre (voir `damage_bar_group`), qui peut donc redevenir un simple ruban
/// fin, plus proche de la maquette fournie par l'utilisateur (2026-09-02) que l'ancienne version
/// épaissie pour loger le texte.
const BAR_HEIGHT: f32 = 14.0;
/// Largeur maximale d'une barre — demande utilisateur explicite (« la largeur de la barre est
/// peut-être un peu trop grande ») : sans plafond, une barre s'étire sur toute la largeur
/// disponible de la colonne de droite, ce qui peut être bien plus large que nécessaire pour rester
/// lisible. Purement un point de départ pour itérer avec l'utilisateur, pas une mesure issue d'une
/// maquette.
const BAR_MAX_WIDTH: f32 = 150.0;
/// Écart entre le nom et sa barre, DANS un groupe (voir `damage_bar_group`) — volontairement plus
/// petit que `ROW_GAP` (qui sépare deux groupes ENTRE eux) : c'est cette différence de rythme qui
/// donne à l'œil la lecture "un nom + une barre = un groupe", demande utilisateur explicite.
const GROUP_NAME_BAR_GAP: f32 = 2.0;

// Couleurs de la barre de dégâts — mesurées sur la maquette fournie par l'utilisateur (capture
// d'écran jointe à la demande de redesign, 2026-09-03).
const BAR_BORDER: egui::Color32 = egui::Color32::from_rgb(30, 31, 37);
const BAR_TRACK: egui::Color32 = egui::Color32::from_rgb(22, 23, 28);
/// Base du remplissage (bas de la barre) — voir `BAR_FILL_HIGHLIGHT` pour le reflet du haut.
const BAR_FILL_BASE: egui::Color32 = egui::Color32::from_rgb(7, 121, 130);
/// Reflet plus clair peint sur le tiers supérieur du remplissage — effet "verre/glacis" mesuré sur
/// la maquette (bande nettement plus claire que `BAR_FILL_BASE` juste sous le bord supérieur).
/// Réutilisée aussi comme couleur du total (voir `show`) : accent le plus prononcé de la palette de
/// ce panneau, cohérent avec les barres plutôt qu'une couleur ajoutée sans lien.
const BAR_FILL_HIGHLIGHT: egui::Color32 = egui::Color32::from_rgb(13, 190, 190);
/// Ligne de biseau (haut et bas de la piste, qu'elle soit remplie ou non à cet endroit) — un blanc
/// translucide fin, mesuré comme un éclaircissement d'environ +40 sur la piste sombre.
const BAR_BEVEL: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 40);
const BAR_TEXT: egui::Color32 = egui::Color32::from_rgb(235, 240, 245);
/// Copie du texte peinte 1px en dessous/à droite avant le texte principal (pseudo-contour) — voir
/// `paint_shadowed_text` : nécessaire pour tout texte qui flotte nu par-dessus le jeu (nom
/// au-dessus d'une barre, pourcentage sur un portrait), le fond derrière étant arbitraire.
const BAR_TEXT_SHADOW: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(0, 0, 0, 200);

const TOTAL_FONT_SIZE: f32 = 16.0;
const TOTAL_GAP: f32 = 6.0;

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
            // Ordre STABLE (pas trié par dégâts, voir doc de module et `FightSnapshot::fighters`)
            // — c'est l'ordre des PORTRAITS, cadre et liste plate confondus.
            let fighters: Vec<&FighterDamage> = fight
                .fighters
                .iter()
                .filter(|f| f.is_ally == (*side == CombatSide::Allies))
                .collect();

            if fighters.is_empty() {
                ui.weak(match side {
                    CombatSide::Allies => "Aucun allié pour l'instant.",
                    CombatSide::Enemies => "Aucun ennemi pour l'instant.",
                });
                return;
            }

            // Barres : liste SÉPARÉE, triée par dégâts décroissant, uniquement les combattants
            // ayant infligé au moins 1 dégât (voir doc de module) — dénominateurs calculés sur
            // cette liste (`max_damage`) et sur TOUS les combattants affichés (`total_damage`, y
            // compris ceux à 0 dégât : ils comptent pour 0 dans la somme, le résultat est
            // identique, mais c'est bien le total du camp affiché qui a du sens ici).
            let mut bars: Vec<&FighterDamage> =
                fighters.iter().copied().filter(|f| f.total_damage > 0).collect();
            bars.sort_by_key(|f| std::cmp::Reverse(f.total_damage));

            let max_damage = bars.first().map_or(1, |f| f.total_damage).max(1);
            let total_damage = fighters.iter().map(|f| f.total_damage).sum::<i64>().max(1);

            let (framed, flat_portraits): (&[&FighterDamage], &[&FighterDamage]) =
                if *side == CombatSide::Allies {
                    fighters.split_at(fighters.len().min(MAX_FRAME_SLOTS))
                } else {
                    (&[], &fighters)
                };

            ui.horizontal_top(|ui| {
                // Colonne de gauche : portraits (cadre pour les 6 premiers alliés dans l'ordre
                // stable, liste plate sinon) — voir doc de module.
                ui.vertical(|ui| {
                    if !framed.is_empty() {
                        frame.show(ui, portraits, icons, framed, total_damage);
                    }
                    if !flat_portraits.is_empty() {
                        if !framed.is_empty() {
                            ui.add_space(ROW_GAP);
                        }
                        for (i, fighter) in flat_portraits.iter().enumerate() {
                            if i > 0 {
                                ui.add_space(ROW_GAP);
                            }
                            paint_flat_portrait(
                                ui,
                                portraits,
                                icons,
                                catalog,
                                remote_icons,
                                remote_icon_textures,
                                fighter,
                                total_damage,
                            );
                        }
                    }
                });

                ui.add_space(COLUMN_GAP);

                // Colonne de droite : total, puis un groupe nom+barre compact par combattant
                // ayant infligé des dégâts, trié par dégâts décroissant — totalement indépendante
                // du rythme vertical de la colonne des portraits (demande utilisateur explicite :
                // « il ne faut pas que les groupes soient alignés au portrait »).
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Total : {total_damage}"))
                            .strong()
                            .size(TOTAL_FONT_SIZE)
                            .color(BAR_FILL_HIGHLIGHT),
                    );
                    ui.add_space(TOTAL_GAP);
                    for (i, fighter) in bars.iter().enumerate() {
                        if i > 0 {
                            ui.add_space(ROW_GAP);
                        }
                        damage_bar_group(ui, &fighter.name, fighter.total_damage, max_damage);
                    }
                });
            });
        }
    }
}

/// Portrait d'une ligne de la liste "plate" (ennemis, ou alliés au-delà de `MAX_FRAME_SLOTS`) —
/// portrait de classe pour un allié classifié, sinon (ennemi, ou allié pas encore classifié)
/// portrait RÉEL du monstre si le catalogue le résout par nom, repli générique tant qu'il n'a pas
/// fini de télécharger ou si le nom n'est pas reconnu. Infobulle (nom) et pourcentage de dégâts
/// (bas-droite, si non nul) systématiques — voir doc de module.
#[allow(clippy::too_many_arguments)]
fn paint_flat_portrait(
    ui: &mut egui::Ui,
    portraits: &PortraitAtlas,
    icons: &UiIcons,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    fighter: &FighterDamage,
    total_damage: i64,
) {
    let class_portrait = fighter
        .class_name
        .as_deref()
        .and_then(|class_name| portraits.image(class_name, fighter.gender, fighter.is_ko));
    let remote_monster_texture = class_portrait.is_none().then(|| {
        catalog
            .find_monster_icon(&fighter.name, None)
            .and_then(|icon_ref| remote_icon_textures.resolve(ui.ctx(), remote_icons, &icon_ref))
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
                // Pas de version grisée précalculée pour une icône distante (voir doc de module) :
                // simple tint, approximation acceptée.
                .tint(grey_tint_if_ko(fighter.is_ko)),
        ),
        (None, None) => ui.add(
            icons
                .unknown_entity_image()
                .tint(grey_tint_if_ko(fighter.is_ko)),
        ),
    };
    let rect = response.rect;
    response.on_hover_text(fighter.name.as_str());
    if fighter.total_damage > 0 {
        paint_portrait_percent(ui, rect, fighter.total_damage, total_damage);
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

/// Pourcentage de dégâts d'un combattant par rapport au total du camp affiché, incrusté en
/// bas-droite de `rect` (portrait de classe, monstre, ou repli générique — appelé aussi bien par
/// `panels::combat_frame::CombatFrame::show` que par `paint_flat_portrait` ci-dessus) — demande
/// utilisateur explicite : pas en plein centre du portrait, légèrement décalé bas-droite, pour
/// garder deux niveaux de lecture distincts (le portrait reste reconnaissable, le pourcentage vient
/// en incrustation plutôt qu'en écraser le centre).
///
/// `rect` est le carré ENGLOBANT du portrait, pas le disque visible qu'il dessine (un cercle
/// inscrit dans ce carré, voir `portraits.rs`) — ancrer le texte pile sur le coin `right_bottom()`
/// du carré le fait déborder hors du disque (le coin d'un carré est toujours hors du cercle qui y
/// est inscrit). `rect.shrink(PERCENT_INSET)` ramène l'ancrage à un point qui reste dans le disque.
pub(crate) fn paint_portrait_percent(ui: &egui::Ui, rect: egui::Rect, damage: i64, total_damage: i64) {
    const PERCENT_INSET: f32 = 8.0;
    let percent = ((damage as f64 / total_damage as f64) * 100.0).round() as i64;
    let text = format!("{percent}%");
    let pos = rect.shrink(PERCENT_INSET).right_bottom();
    paint_shadowed_text(
        ui,
        pos,
        egui::Align2::RIGHT_BOTTOM,
        &text,
        egui::FontId::proportional(10.0),
        BAR_TEXT,
    );
}

/// Un "groupe" nom + barre de la colonne de droite — nom peint au-dessus (`paint_shadowed_text`,
/// pour rester lisible par-dessus le jeu comme le reste de l'overlay), barre juste en dessous,
/// quasiment collée (`GROUP_NAME_BAR_GAP`) pour que l'œil lise la paire comme un seul bloc — demande
/// utilisateur explicite : « il faut qu'on voit que c'est un groupe », et que l'ensemble reste
/// compact (pas d'aération façon liste plate).
fn damage_bar_group(ui: &mut egui::Ui, name: &str, damage: i64, max_damage: i64) {
    let bar_width = ui.available_width().min(BAR_MAX_WIDTH);
    let name_font = egui::FontId::proportional(11.0);
    let name_height = name_font.size + 2.0;
    let (name_rect, _) =
        ui.allocate_exact_size(egui::vec2(bar_width, name_height), egui::Sense::hover());
    paint_shadowed_text(
        ui,
        name_rect.left_center(),
        egui::Align2::LEFT_CENTER,
        name,
        name_font,
        BAR_TEXT,
    );
    ui.add_space(GROUP_NAME_BAR_GAP);
    let (bar_rect, _) =
        ui.allocate_exact_size(egui::vec2(bar_width, BAR_HEIGHT), egui::Sense::hover());
    damage_bar(ui, bar_rect, damage, max_damage);
}

/// Piste + remplissage d'une barre de dégâts, peinte dans `rect` (déjà alloué par l'appelant, voir
/// `damage_bar_group`) : fond sombre biseauté, rempli d'un dégradé sarcelle/turquoise dans une
/// proportion `damage / max_damage` (référence = plus gros dégât de la colonne, PAS le total :
/// c'est ce qui donne sa longueur visuelle à chaque barre). Ni nom ni pourcentage ici depuis la
/// refonte 2026-09-04 (voir doc de module) : le nom est peint par l'appelant au-dessus de `rect`,
/// le pourcentage sur le portrait correspondant (`paint_portrait_percent`). Couleurs : voir
/// `BAR_BORDER`/`BAR_TRACK`/`BAR_FILL_BASE`/`BAR_FILL_HIGHLIGHT`/`BAR_BEVEL`, mesurées sur la
/// maquette fournie par l'utilisateur.
fn damage_bar(ui: &mut egui::Ui, rect: egui::Rect, damage: i64, max_damage: i64) {
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
}

/// Peint `text` en deux passes (copie décalée de 1px en ombre, puis le texte plein) — nécessaire
/// pour tout texte qui flotte nu par-dessus le jeu (nom au-dessus d'une barre, pourcentage sur un
/// portrait) : contrairement à l'ancien texte peint DANS la barre (fond toujours sombre connu),
/// ici le fond est le jeu lui-même, de couleur arbitraire — voir doc de module.
fn paint_shadowed_text(
    ui: &egui::Ui,
    pos: egui::Pos2,
    align: egui::Align2,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
) {
    let painter = ui.painter();
    painter.text(
        pos + egui::vec2(1.0, 1.0),
        align,
        text,
        font.clone(),
        BAR_TEXT_SHADOW,
    );
    painter.text(pos, align, text, font, color);
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
