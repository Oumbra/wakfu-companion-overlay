//! Bloc « ligne de sorts » du panneau Combat — sous le dernier groupe de dégâts, les sorts lancés
//! par un allié pendant son dernier tour, en icônes numérotées, avec une rangée d'onglets-portraits
//! pour choisir l'allié. Spécification validée en artefact avec l'utilisateur (quatre révisions,
//! 11-12 sept. 2026, « Ligne de sorts du combat », révision 4c) ; les cotes ci-dessous en sont la
//! transcription littérale, ne pas les « arrondir » sans rouvrir la décision.
//!
//! ## Ce que le bloc montre
//!
//! - **Onglets** : tous les alliés du combat, dans l'ordre stable de `FightSnapshot::fighters`,
//!   sur SIX emplacements fixes calculés sur le cas à six alliés (sept écarts égaux, premier et
//!   dernier portrait à la même distance des bords, quel que soit le nombre d'alliés). Un allié qui
//!   n'a encore rien lancé a son onglet estompé, non cliquable, à sa place définitive. Au-delà de
//!   six alliés (jamais vu en pratique, même limite que le cadre à médaillons), les suivants n'ont
//!   pas d'onglet — question encore ouverte dans l'artefact, tranchée provisoirement ainsi.
//! - **Sélection automatique** : l'indicateur suit le dernier allié à avoir lancé un sort
//!   (`FightSnapshot::last_ally_caster`). **Clic** sur un onglet : épingle cet allié, ses sorts
//!   restent affichés pendant que les autres jouent ; second clic sur l'onglet épinglé : retour au
//!   suivi automatique. L'épingle est un état local de la fenêtre (`egui::Context::data_mut`), clé
//!   par `fight_id` : un nouveau combat repart sans épingle.
//! - **Indicateur** : 2 px `ACCENT` posé sur un séparateur discret, qui GLISSE jusqu'à l'onglet
//!   choisi en 250 ms (même geste que `.icon-switch-highlight` du site web, courbe CSS `ease`) —
//!   voir `Slide`. À l'ouverture du bloc il apparaît directement sous l'allié sélectionné.
//! - **Sorts** : `FighterDamage::last_turn_casts` de l'allié sélectionné (remis à zéro par le
//!   moteur à chaque nouveau tour, voir `overlay_engine::session`), cinq icônes de 32 px par
//!   rangée, retour à la ligne (jamais de défilement latéral, décision explicite), badge d'index
//!   en haut à gauche (décidé le 12 septembre), critique = liseré doré + coin plié haut-droit.
//!   Survol : liseré `ACCENT` et infobulle `design::tooltip` au-dessus (nom du sort, puis lanceur
//!   et « Critique » s'il y a lieu).
//! - **Icône** : nom normalisé + classe du lanceur → `overlay_engine::SpellIndex` (référentiel
//!   `assets/spells.json`, maintenu par l'utilisateur) → `RemoteIconStore`/`RemoteIconTextures`,
//!   même circuit que les portraits de monstres. Sans correspondance : pavé « ? », nom brut en
//!   infobulle, un `tracing::warn!` par nom et par session (voir `warn_unknown_spell_once`).
//! - **Ennemis** : jamais dans le bloc, mais le bloc reste visible sur la vue Ennemis (retenu
//!   dans l'artefact) — il porte sur les alliés du combat, pas sur le camp affiché au-dessus.
//!
//! ## Géométrie (repère du bloc, origine en haut à gauche)
//!
//! Largeur `BLOCK_WIDTH` (190, celle des barres), fond `LEADER_PANEL_FILL` arrondi 6, marge
//! intérieure 4. Onglets 22 px à `y = 4`, emplacement `i` à `x = 4 + 7,14 + i × 29,14`.
//! Séparateur 1 px blanc à 13 % à `y = 30`, du bord gauche du premier emplacement au bord droit du
//! sixième. Indicateur 22 × 2 à `y = 29`. Première rangée de sorts à `y = 39`, 32 px + 3 px
//! d'écart, cinq par rangée. Hauteur : `39 + 32·r + 3·(r − 1) + 4` = 75 / 110 / 145 px pour `r`
//! rangées (au moins une, même vide).

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use overlay_engine::{FightSnapshot, FighterDamage, SpellIndex};

use crate::design::{self, text, tokens};
use crate::portraits::PortraitAtlas;
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::ui_icons::UiIcons;

use super::combat::{ACCENT, BAR_MAX_WIDTH, LEADER_PANEL_FILL, LEADER_PANEL_ROUNDING};

/// Air VISIBLE entre la barre du dernier groupe de dégâts et le haut du bloc — « le double de
/// l'écart entre le switch et le premier groupe » (demande utilisateur, 2 × 9 px visibles sous le
/// switch). L'appelant (`combat::show`) en retranche l'`item_spacing` vertical d'egui pour que ce
/// soit bien l'écart à l'écran.
pub const BLOCK_GAP: f32 = 18.0;
/// Largeur du bloc — celle de la colonne des barres, il s'aligne dessus.
pub const BLOCK_WIDTH: f32 = BAR_MAX_WIDTH;
const PADDING: f32 = 4.0;
const INNER_WIDTH: f32 = BLOCK_WIDTH - 2.0 * PADDING;

/// Emplacements d'onglet, FIXES quel que soit le nombre d'alliés (voir doc de module).
pub const TAB_SLOTS: usize = 6;
pub const TAB_SIZE: f32 = 22.0;
/// Sept écarts égaux sur la largeur utile : `(182 − 6 × 22) / 7` ≈ 7,14 px.
const TAB_GAP: f32 = (INNER_WIDTH - TAB_SLOTS as f32 * TAB_SIZE) / (TAB_SLOTS as f32 + 1.0);
const TAB_TOP: f32 = 4.0;
/// Opacité d'un onglet non sélectionné (55 %), et d'un allié qui n'a encore rien lancé (28 %).
const TAB_ALPHA_IDLE: u8 = 140;
const TAB_ALPHA_SILENT: u8 = 71;

/// Séparateur : 1 px, blanc à 13 %, `y = 30` (centre du trait à 30,5).
const SEPARATOR_Y: f32 = 30.5;
const SEPARATOR_COLOR: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 33);
const INDICATOR_TOP: f32 = 29.0;
const INDICATOR_HEIGHT: f32 = 2.0;
/// Durée de la glissade de l'indicateur — `transition: left .25s ease` du site.
const SLIDE_DURATION: f64 = 0.25;

const ROWS_TOP: f32 = 39.0;
pub const SPELL_SIZE: f32 = 32.0;
const SPELL_GAP: f32 = 3.0;
pub const SPELLS_PER_ROW: usize = 5;
const SPELL_ROUNDING: f32 = 3.0;
const BOTTOM_PADDING: f32 = 4.0;
/// Cadre sombre de 1 px autour de chaque icône (`#0e1115`, le fond des champs du design system).
const SPELL_FRAME: egui::Color32 = tokens::INPUT_FILL;
/// Fond du pavé « ? » (sort absent du référentiel) et de l'icône pas encore téléchargée.
const SPELL_PLACEHOLDER_FILL: egui::Color32 = egui::Color32::from_rgb(0x1E, 0x22, 0x28);
/// Doré du design system (`#f4d89e`, le même que `tokens::INPUT_TEXT`) — liseré et coin plié d'un
/// critique, seconde ligne de l'infobulle, « ? » du pavé de repli.
const GOLD: egui::Color32 = egui::Color32::from_rgb(0xF4, 0xD8, 0x9E);
const CRIT_CORNER: f32 = 9.0;

const BADGE_INSET: f32 = 2.0;
const BADGE_MIN_WIDTH: f32 = 12.0;
const BADGE_HEIGHT: f32 = 11.0;
const BADGE_ROUNDING: f32 = 2.0;
const BADGE_PADDING_X: f32 = 2.0;
/// Fond noir à 62 %.
const BADGE_FILL: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(0, 0, 0, 158);
const BADGE_FONT_SIZE: f32 = 9.0;

/// État local du bloc, par fenêtre et par combat — voir doc de module.
#[derive(Clone, Copy, Default)]
struct SpellBlockState {
    /// Index (dans `FightSnapshot::fighters`) de l'allié épinglé par un clic, `None` en suivi
    /// automatique.
    pinned: Option<usize>,
    slide: Option<Slide>,
}

/// Glissade en cours (ou terminée) de l'indicateur — position en x du bord gauche, repère du
/// bloc. `from`/`to` en points, `started_at` en secondes egui (`InputState::time`).
#[derive(Clone, Copy)]
struct Slide {
    from: f32,
    to: f32,
    started_at: f64,
}

impl Slide {
    fn position(&self, now: f64) -> f32 {
        let t = ((now - self.started_at) / SLIDE_DURATION).clamp(0.0, 1.0) as f32;
        self.from + (self.to - self.from) * css_ease(t)
    }

    fn done(&self, now: f64) -> bool {
        now - self.started_at >= SLIDE_DURATION
    }
}

/// Courbe CSS `ease` = `cubic-bezier(0.25, 0.1, 0.25, 1.0)` — celle de `.icon-switch-highlight`
/// côté web. Résolue par quelques itérations de Newton sur la composante x (largement suffisant
/// pour une animation de 250 ms à 60 images/s), pas une approximation « smoothstep » qui n'aurait
/// pas le départ un peu plus vif de `ease`.
fn css_ease(t: f32) -> f32 {
    const X1: f32 = 0.25;
    const Y1: f32 = 0.1;
    const X2: f32 = 0.25;
    const Y2: f32 = 1.0;
    let bezier = |p1: f32, p2: f32, s: f32| {
        let c = 3.0 * p1;
        let b = 3.0 * (p2 - p1) - c;
        let a = 1.0 - c - b;
        ((a * s + b) * s + c) * s
    };
    let derivative = |p1: f32, p2: f32, s: f32| {
        let c = 3.0 * p1;
        let b = 3.0 * (p2 - p1) - c;
        let a = 1.0 - c - b;
        (3.0 * a * s + 2.0 * b) * s + c
    };
    let mut s = t;
    for _ in 0..6 {
        let x = bezier(X1, X2, s) - t;
        let dx = derivative(X1, X2, s);
        if dx.abs() < 1e-6 {
            break;
        }
        s -= x / dx;
    }
    bezier(Y1, Y2, s.clamp(0.0, 1.0))
}

/// Bord gauche de l'emplacement d'onglet `slot` (0 à 5), repère du bloc.
fn tab_x(slot: usize) -> f32 {
    PADDING + TAB_GAP + slot as f32 * (TAB_SIZE + TAB_GAP)
}

/// Hauteur du bloc pour `rows` rangées de sorts (au moins une, même vide) — voir doc de module.
fn block_height(rows: usize) -> f32 {
    let rows = rows.max(1) as f32;
    ROWS_TOP + SPELL_SIZE * rows + SPELL_GAP * (rows - 1.0) + BOTTOM_PADDING
}

/// Peint le bloc à la position courante de `ui` (colonne des barres, après le dernier groupe et
/// `BLOCK_GAP`). Ne fait rien si le combat n'a aucun allié — l'appelant vérifie déjà, par
/// cohérence avec « le bloc n'est peint que s'il y a un combat et au moins un allié ».
pub fn show(
    ui: &mut egui::Ui,
    fight: &FightSnapshot,
    portraits: &PortraitAtlas,
    icons: &UiIcons,
    spells: &SpellIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
) {
    let allies: Vec<(usize, &FighterDamage)> = fight
        .fighters
        .iter()
        .enumerate()
        .filter(|(_, f)| f.is_ally)
        .take(TAB_SLOTS)
        .collect();
    if allies.is_empty() {
        return;
    }
    let is_tab = |idx: usize| allies.iter().any(|(i, _)| *i == idx);
    let has_casts = |idx: usize| !fight.fighters[idx].last_turn_casts.is_empty();

    let state_id = egui::Id::new(("combat-spell-block", fight.fight_id));
    let mut state: SpellBlockState = ui
        .ctx()
        .data_mut(|d| d.get_temp(state_id))
        .unwrap_or_default();

    // Les onglets réagissent AVANT l'allocation du bloc (leur position ne dépend que de l'origine
    // du bloc, connue d'avance) : un clic change l'allié sélectionné, donc le nombre de rangées,
    // donc la hauteur à allouer — dans la même frame, sans une frame de décalage où le fond aurait
    // la mauvaise hauteur.
    let origin = ui.cursor().min;
    let mut tab_responses = Vec::with_capacity(allies.len());
    for (slot, (idx, ally)) in allies.iter().enumerate() {
        let rect = egui::Rect::from_min_size(
            origin + egui::vec2(tab_x(slot), TAB_TOP),
            egui::Vec2::splat(TAB_SIZE),
        );
        let clickable = has_casts(*idx);
        let sense = if clickable {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let response = ui.interact(rect, ui.id().with(("spell-block-tab", *idx)), sense);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, clickable, ally.name.as_str())
        });
        if clickable && response.clicked() {
            state.pinned = if state.pinned == Some(*idx) {
                None
            } else {
                Some(*idx)
            };
        }
        tab_responses.push((rect, response));
    }

    // Sélection : épingle valide, sinon dernier lanceur allié, sinon premier allié ayant lancé
    // quelque chose, sinon le premier allié (bloc vide, une rangée).
    let pinned = state.pinned.filter(|&i| is_tab(i) && has_casts(i));
    state.pinned = pinned;
    let selected = pinned
        .or(fight.last_ally_caster.filter(|&i| is_tab(i)))
        .or_else(|| allies.iter().map(|(i, _)| *i).find(|&i| has_casts(i)))
        .unwrap_or(allies[0].0);
    let selected_slot = allies.iter().position(|(i, _)| *i == selected).unwrap_or(0);
    let selected_ally = &fight.fighters[selected];
    let casts = &selected_ally.last_turn_casts;
    let rows = casts.len().div_ceil(SPELLS_PER_ROW);

    let (block, _) = ui.allocate_exact_size(
        egui::vec2(BLOCK_WIDTH, block_height(rows)),
        egui::Sense::hover(),
    );
    let painter = ui.painter().clone();
    painter.rect_filled(block, LEADER_PANEL_ROUNDING, LEADER_PANEL_FILL);

    // Séparateur, du bord gauche du 1er emplacement au bord droit du 6e — toujours les six, même
    // avec un seul allié (les emplacements ne dépendent pas du nombre d'alliés).
    painter.hline(
        (block.left() + tab_x(0))..=(block.left() + tab_x(TAB_SLOTS - 1) + TAB_SIZE),
        block.top() + SEPARATOR_Y,
        egui::Stroke::new(1.0, SEPARATOR_COLOR),
    );

    // Indicateur : glisse vers l'emplacement sélectionné (voir `Slide`), directement en place à
    // la première apparition.
    let now = ui.input(|i| i.time);
    let target = tab_x(selected_slot);
    let indicator_x = match state.slide {
        None => {
            state.slide = Some(Slide {
                from: target,
                to: target,
                started_at: now - SLIDE_DURATION,
            });
            target
        }
        Some(slide) if slide.to != target => {
            let from = slide.position(now);
            state.slide = Some(Slide {
                from,
                to: target,
                started_at: now,
            });
            from
        }
        Some(slide) => slide.position(now),
    };
    if state.slide.is_some_and(|s| !s.done(now)) {
        ui.ctx().request_repaint();
    }
    painter.rect_filled(
        egui::Rect::from_min_size(
            block.min + egui::vec2(indicator_x, INDICATOR_TOP),
            egui::vec2(TAB_SIZE, INDICATOR_HEIGHT),
        ),
        0.0,
        ACCENT,
    );

    // Onglets-portraits, par-dessus le séparateur.
    for ((rect, response), (idx, ally)) in tab_responses.iter().zip(&allies) {
        let alpha = if *idx == selected {
            255
        } else if has_casts(*idx) {
            TAB_ALPHA_IDLE
        } else {
            TAB_ALPHA_SILENT
        };
        let texture = ally
            .class_name
            .as_deref()
            .and_then(|class| portraits.texture(class, ally.gender, ally.is_ko))
            .unwrap_or_else(|| icons.unknown_entity_texture());
        egui::Image::new(texture)
            .fit_to_exact_size(rect.size())
            .maintain_aspect_ratio(false)
            .corner_radius((TAB_SIZE / 2.0) as u8)
            .tint(egui::Color32::from_white_alpha(alpha))
            .paint_at(ui, *rect);
        painter.circle_stroke(
            rect.center(),
            TAB_SIZE / 2.0,
            egui::Stroke::new(1.0, egui::Color32::from_black_alpha(alpha)),
        );
        design::tooltip(response).text(ally.name.as_str());
        if has_casts(*idx) {
            response
                .clone()
                .on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    }

    // Rangées de sorts.
    for (i, cast) in casts.iter().enumerate() {
        let row = i / SPELLS_PER_ROW;
        let col = i % SPELLS_PER_ROW;
        let rect = egui::Rect::from_min_size(
            block.min
                + egui::vec2(
                    PADDING + col as f32 * (SPELL_SIZE + SPELL_GAP),
                    ROWS_TOP + row as f32 * (SPELL_SIZE + SPELL_GAP),
                ),
            egui::Vec2::splat(SPELL_SIZE),
        );
        let response = ui.interact(
            rect,
            ui.id().with(("spell-block-spell", selected, i)),
            egui::Sense::hover(),
        );

        let entry = spells.find(&cast.spell, selected_ally.class_name.as_deref());
        let display_name = entry.map_or(cast.spell.as_str(), |e| e.name.as_str());
        let texture =
            entry.and_then(|e| remote_icon_textures.resolve(ui.ctx(), remote_icons, &e.icon));
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Image,
                true,
                format!("Sort {} : {display_name}", i + 1),
            )
        });

        match texture {
            Some(texture) => {
                egui::Image::new(&texture)
                    .fit_to_exact_size(rect.size())
                    .maintain_aspect_ratio(false)
                    .corner_radius(SPELL_ROUNDING as u8)
                    .paint_at(ui, rect);
            }
            None => {
                painter.rect_filled(rect, SPELL_ROUNDING, SPELL_PLACEHOLDER_FILL);
                if entry.is_none() {
                    warn_unknown_spell_once(&cast.spell, selected_ally.class_name.as_deref());
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "?",
                        text::label_strong_font(ui.ctx(), 16.0),
                        GOLD,
                    );
                }
            }
        }
        painter.rect_stroke(
            rect,
            SPELL_ROUNDING,
            egui::Stroke::new(1.0, SPELL_FRAME),
            egui::StrokeKind::Outside,
        );
        if cast.critical {
            painter.rect_stroke(
                rect.expand(1.0),
                SPELL_ROUNDING + 1.0,
                egui::Stroke::new(1.0, GOLD),
                egui::StrokeKind::Outside,
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(rect.right() - CRIT_CORNER, rect.top()),
                    egui::pos2(rect.right(), rect.top()),
                    egui::pos2(rect.right(), rect.top() + CRIT_CORNER),
                ],
                GOLD,
                egui::Stroke::NONE,
            ));
        }
        if response.hovered() {
            painter.rect_stroke(
                rect,
                SPELL_ROUNDING,
                egui::Stroke::new(1.0, ACCENT),
                egui::StrokeKind::Outside,
            );
        }

        paint_index_badge(ui, &painter, rect, i + 1);

        let caster = selected_ally.name.clone();
        let critical = cast.critical;
        let display_name = display_name.to_string();
        design::tooltip(&response).show(|ui| {
            ui.set_max_width(ui.spacing().tooltip_width);
            ui.label(egui::RichText::new(display_name).color(tokens::TOOLTIP_TEXT));
            let detail = if critical {
                format!("{caster} · Critique")
            } else {
                caster
            };
            ui.label(egui::RichText::new(detail).color(GOLD));
        });
    }

    ui.ctx().data_mut(|d| d.insert_temp(state_id, state));
}

/// Badge d'index en haut à gauche de l'icône, 2 px du bord : pastille noire à 62 %, rayon 2,
/// 12 × 11 minimum (s'élargit pour deux chiffres), chiffre blanc 9 px semi-gras sans contour.
fn paint_index_badge(ui: &egui::Ui, painter: &egui::Painter, icon: egui::Rect, index: usize) {
    let galley = painter.layout_no_wrap(
        index.to_string(),
        text::label_strong_font(ui.ctx(), BADGE_FONT_SIZE),
        egui::Color32::WHITE,
    );
    let width = (galley.size().x + 2.0 * BADGE_PADDING_X).max(BADGE_MIN_WIDTH);
    let badge = egui::Rect::from_min_size(
        icon.min + egui::Vec2::splat(BADGE_INSET),
        egui::vec2(width, BADGE_HEIGHT),
    );
    painter.rect_filled(badge, BADGE_ROUNDING, BADGE_FILL);
    painter.galley(
        badge.center() - galley.size() / 2.0,
        galley,
        egui::Color32::WHITE,
    );
}

/// Un avertissement par nom de sort et par session — le référentiel est maintenu à la main, c'est
/// ce message qui dit à l'utilisateur quoi y ajouter, sans inonder le journal à chaque frame.
fn warn_unknown_spell_once(spell: &str, class_name: Option<&str>) {
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let mut warned = WARNED.get_or_init(Default::default).lock().unwrap();
    if warned.insert(spell.to_string()) {
        tracing::warn!(
            spell,
            class = class_name.unwrap_or("?"),
            "sort absent du référentiel assets/spells.json, pavé « ? » affiché"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_emplacements_symetriques_sur_la_largeur_utile() {
        let first_left = tab_x(0) - PADDING;
        let last_right = INNER_WIDTH - (tab_x(TAB_SLOTS - 1) - PADDING + TAB_SIZE);
        assert!((first_left - last_right).abs() < 1e-4);
        assert!((first_left - TAB_GAP).abs() < 1e-4);
        assert!((TAB_GAP - 7.142857).abs() < 1e-3);
    }

    #[test]
    fn hauteur_du_bloc_par_rangee() {
        assert_eq!(block_height(0), 75.0);
        assert_eq!(block_height(1), 75.0);
        assert_eq!(block_height(2), 110.0);
        assert_eq!(block_height(3), 145.0);
    }

    #[test]
    fn la_courbe_ease_est_monotone_et_bornee() {
        assert!(css_ease(0.0).abs() < 1e-4);
        assert!((css_ease(1.0) - 1.0).abs() < 1e-4);
        let mut previous = 0.0;
        for step in 1..=20 {
            let value = css_ease(step as f32 / 20.0);
            assert!(value >= previous - 1e-5, "non monotone à {step}");
            previous = value;
        }
        // `ease` est en avance sur la droite en milieu de course (départ vif).
        assert!(css_ease(0.5) > 0.7);
    }

    #[test]
    fn la_glissade_part_de_from_et_finit_sur_to() {
        let slide = Slide {
            from: 10.0,
            to: 40.0,
            started_at: 100.0,
        };
        assert_eq!(slide.position(100.0), 10.0);
        assert_eq!(slide.position(100.0 + SLIDE_DURATION), 40.0);
        assert_eq!(slide.position(200.0), 40.0);
        assert!(!slide.done(100.1));
        assert!(slide.done(100.0 + SLIDE_DURATION));
    }
}
