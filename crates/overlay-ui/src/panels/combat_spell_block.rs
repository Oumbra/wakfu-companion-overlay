//! Bloc « ligne de sorts » du panneau Combat — sous le dernier groupe de dégâts, les sorts lancés
//! par un allié pendant son dernier tour, en icônes numérotées. L'allié se choisit **en cliquant
//! son portrait dans le cadre à médaillons** (`combat_frame`), où deux marques dorées disent qui
//! est qui. Spécification validée en artefact avec l'utilisateur (« Ligne de sorts du combat »,
//! révision 4c, 11-12 sept. 2026), puis **refonte « Sélection par le cadre »** le 12 sept. au soir
//! (proposition en artefact, révision 2, décidée) : la rangée d'onglets-portraits de 22 px du
//! premier rendu, jugée trop petite et coûteuse en hauteur, a été retirée au profit des portraits
//! de 48 px déjà présents dans le cadre.
//!
//! ## Ce que le bloc montre
//!
//! - **Sorts** : `FighterDamage::last_turn_casts` de l'allié sélectionné (remis à zéro par le
//!   moteur à chaque nouveau tour, voir `overlay_engine::session`), cinq icônes de 32 px par
//!   rangée, retour à la ligne (jamais de défilement latéral, décision explicite), badge d'index
//!   en haut à gauche, critique = liseré doré + coin plié haut-droit. Survol : liseré `ACCENT` et
//!   infobulle `design::tooltip` au-dessus, une seule ligne : le nom du sort en blanc, suivi de
//!   « · Critique » en doré s'il y a lieu (pas de nom de lanceur : le liseré du cadre le dit).
//! - **Icône** : nom normalisé + classe du lanceur → `overlay_engine::SpellIndex` (référentiel
//!   `assets/spells.json`, maintenu par l'utilisateur) → `RemoteIconStore`/`RemoteIconTextures`,
//!   même circuit que les portraits de monstres. Sans correspondance : pavé « ? », nom brut en
//!   infobulle, un `tracing::warn!` par nom et par session (voir `warn_unknown_spell_once`).
//!   Entrée connue mais sans image dans le fichier : tuile sombre sans « ? », nom en infobulle.
//! - **Vue Alliés seulement** (décidé le 12 sept.) : en vue Ennemis, ni bloc ni marques — les
//!   portraits alliés n'y sont pas, rien n'indiquerait de qui sont les sorts.
//! - **Rien avant le premier sort allié** : ni bloc, ni espace réservé, ni marque.
//!
//! ## Les deux marques sur le cadre (voir [`SpellSelection`], [`paint_marks`])
//!
//! - **Le liseré** (`RING_COLOR`, 2 px au bord du portrait, sur le rebord du médaillon) entoure
//!   l'allié **dont on lit les sorts**.
//! - **Le point** (`DOT_RADIUS`, en haut à gauche du portrait, à l'opposé du pourcentage de
//!   dégâts, « comme une notification ») marque **le dernier allié à avoir lancé un sort**
//!   (`FightSnapshot::last_ally_caster`) — toujours à jour, qu'un ennemi ait joué depuis ou non.
//! - Par défaut les deux sont sur le même allié : c'est le **suivi automatique**, le liseré suit le
//!   point. Un clic sur un autre allié **épingle** ce dernier : le liseré reste sur lui pendant que
//!   le point continue de suivre les lanceurs. Un clic sur l'allié qui porte le point, ou un second
//!   clic sur l'allié épinglé, rend la main au suivi automatique. Un allié qui n'a rien lancé n'est
//!   pas cliquable et ne porte jamais de marque. L'épingle est un état local de la fenêtre
//!   (`egui::Context::data_mut`), clé par `fight_id` : un nouveau combat repart sans épingle.
//! - Les marques apparaissent et changent de médaillon par un fondu de `MARK_FADE` — pas de
//!   glissade entre des médaillons espacés de 52 px.
//!
//! ## Géométrie du bloc (repère du bloc, origine en haut à gauche)
//!
//! Largeur `BLOCK_WIDTH` (190, celle des barres), fond `LEADER_PANEL_FILL` arrondi 6, marge
//! intérieure 4. Icônes de 32 px + 5 px d'écart (voir `SPELL_GAP`), cinq par rangée. Hauteur :
//! `4 + 32·r + 5·(r − 1) + 4` = 40 / 77 / 114 px pour `r` rangées. `BLOCK_GAP` (10 px) au-dessus.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use overlay_engine::{FightSnapshot, FighterDamage, SpellIndex};

use crate::design::{self, text, tokens};
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};

use crate::design::tokens::OVERLAY_ACCENT as ACCENT;

use super::combat::{BAR_MAX_WIDTH, LEADER_PANEL_FILL, LEADER_PANEL_ROUNDING};

/// Air VISIBLE entre la barre du dernier groupe de dégâts et le haut du bloc — 18 px dans la
/// spécification d'origine, ramené à 10 px le 12 sept. (révision 2 de « Sélection par le cadre »),
/// le même écart que `combat::TOTAL_GAP` entre le total et le premier groupe, pour l'homogénéité
/// des blocs. L'appelant (`combat::show`) en retranche l'`item_spacing` vertical d'egui pour que
/// ce soit bien l'écart à l'écran.
pub const BLOCK_GAP: f32 = 10.0;
/// Largeur du bloc — celle de la colonne des barres, il s'aligne dessus.
pub const BLOCK_WIDTH: f32 = BAR_MAX_WIDTH;
const PADDING: f32 = 4.0;
#[cfg(test)]
const INNER_WIDTH: f32 = BLOCK_WIDTH - 2.0 * PADDING;

pub const SPELL_SIZE: f32 = 32.0;
/// Écart entre deux icônes — 5 px, pas les 3 px de la maquette : un critique porte deux liserés
/// HORS de son icône (1 px sombre + 1 px doré), soit 2 px de chaque côté ; à 3 px, deux critiques
/// voisins se touchaient (capture utilisateur du 12 sept., trois critiques d'affilée sans aucun
/// jour entre eux). À 5 px il reste 1 px d'air entre deux liserés dorés, et cinq icônes tiennent
/// toujours dans la largeur utile (5 × 32 + 4 × 5 = 180 ≤ 182).
const SPELL_GAP: f32 = 5.0;
pub const SPELLS_PER_ROW: usize = 5;
const SPELL_ROUNDING: f32 = 3.0;
/// Cadre sombre de 1 px autour de chaque icône (`#0e1115`, le fond des champs du design system).
const SPELL_FRAME: egui::Color32 = tokens::INPUT_FILL;
/// Fond du pavé « ? » (sort absent du référentiel) et de l'icône pas encore téléchargée.
const SPELL_PLACEHOLDER_FILL: egui::Color32 = egui::Color32::from_rgb(0x1E, 0x22, 0x28);
/// Doré du design system (`#f4d89e`, le même que `tokens::INPUT_TEXT`) — liseré et coin plié d'un
/// critique, « · Critique » de l'infobulle, « ? » du pavé de repli, et les deux marques du cadre.
const GOLD: egui::Color32 = egui::Color32::from_rgb(0xF4, 0xD8, 0x9E);
const CRIT_CORNER: f32 = 9.0;

/// Retrait du badge depuis le coin haut-gauche de l'icône : 1 px de chaque côté (retours
/// utilisateur du 12 sept., en deux fois : d'abord le haut — à 2 px, le décalage sautait aux yeux
/// sur un critique, dont les deux liserés épaississent le bord — puis la gauche, jugée encore
/// trop rentrée sur l'agrandissement ×3).
const BADGE_INSET_X: f32 = 1.0;
const BADGE_INSET_Y: f32 = 1.0;
const BADGE_MIN_WIDTH: f32 = 12.0;
const BADGE_HEIGHT: f32 = 11.0;
const BADGE_ROUNDING: f32 = 2.0;
const BADGE_PADDING_X: f32 = 2.0;
/// Fond noir à 62 %.
const BADGE_FILL: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(0, 0, 0, 158);
const BADGE_FONT_SIZE: f32 = 9.0;

/// Liseré de sélection sur le portrait du cadre — doré (décidé le 12 sept., révision 2, contre le
/// violet Stasis et le cyan de l'overlay), 2 px posés au bord extérieur du portrait de 48 px
/// (rayon 24 → 26), sur le rebord du médaillon.
pub const RING_COLOR: egui::Color32 = GOLD;
pub const RING_WIDTH: f32 = 2.0;
/// Point du dernier lanceur — 6 px de diamètre, doré, cerné de 1 px sombre, centré sur le bord du
/// portrait en haut à gauche (à 45°), à l'opposé du pourcentage de dégâts en bas à droite.
pub const DOT_RADIUS: f32 = 3.0;
const DOT_OUTLINE: egui::Color32 = SPELL_FRAME;
/// Fondu d'apparition/déplacement des deux marques.
pub const MARK_FADE: f32 = 0.15;

/// État local du bloc, par fenêtre et par combat — voir doc de module.
#[derive(Clone, Copy, Default)]
struct SpellBlockState {
    /// Index (dans `FightSnapshot::fighters`) de l'allié épinglé par un clic, `None` en suivi
    /// automatique.
    pinned: Option<usize>,
}

fn state_id(fight: &FightSnapshot) -> egui::Id {
    egui::Id::new(("combat-spell-block", fight.fight_id))
}

fn load_state(ctx: &egui::Context, fight: &FightSnapshot) -> SpellBlockState {
    ctx.data_mut(|d| d.get_temp(state_id(fight)))
        .unwrap_or_default()
}

fn store_state(ctx: &egui::Context, fight: &FightSnapshot, state: SpellBlockState) {
    ctx.data_mut(|d| d.insert_temp(state_id(fight), state));
}

/// Un allié ayant lancé au moins un sort ce combat — le seul genre de combattant que le bloc
/// affiche, que l'on peut cliquer, ou qui porte une marque.
fn is_casting_ally(fight: &FightSnapshot, idx: usize) -> bool {
    fight
        .fighters
        .get(idx)
        .is_some_and(|f| f.is_ally && !f.last_turn_casts.is_empty())
}

/// Le bloc a-t-il quelque chose à montrer — au moins un allié ayant lancé un sort ce combat.
pub fn has_casting_ally(fight: &FightSnapshot) -> bool {
    (0..fight.fighters.len()).any(|i| is_casting_ally(fight, i))
}

/// Qui porte quelle marque — calculé une fois par frame par `combat::show` (vue Alliés seulement),
/// partagé entre le cadre (marques, clics) et le bloc (sorts). Les deux index pointent dans
/// `FightSnapshot::fighters`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellSelection {
    /// L'allié dont on lit les sorts — le liseré.
    pub selected: usize,
    /// Le dernier allié à avoir lancé un sort — le point.
    pub last_caster: usize,
}

/// Sélection courante pour ce combat — `None` tant qu'aucun allié n'a lancé de sort (rien n'est
/// alors affiché, ni bloc ni marque). Une épingle devenue invalide (combattant disparu) est levée.
pub fn selection(ctx: &egui::Context, fight: &FightSnapshot) -> Option<SpellSelection> {
    let last_caster = fight
        .last_ally_caster
        .filter(|&i| is_casting_ally(fight, i))
        .or_else(|| (0..fight.fighters.len()).find(|&i| is_casting_ally(fight, i)))?;
    let mut state = load_state(ctx, fight);
    if state.pinned.is_some_and(|i| !is_casting_ally(fight, i)) {
        state.pinned = None;
        store_state(ctx, fight, state);
    }
    Some(SpellSelection {
        selected: state.pinned.unwrap_or(last_caster),
        last_caster,
    })
}

/// Clic sur le portrait du combattant `idx` (index dans `FightSnapshot::fighters`) — applique les
/// règles de l'épingle (voir doc de module) : clic sur le porteur du point ou sur l'allié déjà
/// épinglé → suivi automatique ; clic sur un autre allié ayant lancé un sort → épingle. Sans effet
/// pour un combattant qui n'a rien lancé.
pub fn on_portrait_clicked(ctx: &egui::Context, fight: &FightSnapshot, idx: usize) {
    let Some(current) = selection(ctx, fight) else {
        return;
    };
    if !is_casting_ally(fight, idx) {
        return;
    }
    let mut state = load_state(ctx, fight);
    state.pinned = if idx == current.last_caster || state.pinned == Some(idx) {
        None
    } else {
        Some(idx)
    };
    store_state(ctx, fight, state);
}

/// Peint les marques sur un portrait rond du cadre (`portrait_rect` : le carré englobant du
/// portrait) — `ring` pour le liseré, `dot` pour le point, chacune en fondu (`MARK_FADE`, animé
/// par `id`). À appeler APRÈS le portrait et AVANT le pourcentage de dégâts, qui doit rester
/// lisible par-dessus le liseré.
pub fn paint_marks(ui: &egui::Ui, portrait_rect: egui::Rect, id: egui::Id, ring: bool, dot: bool) {
    let ctx = ui.ctx();
    let ring_alpha = ctx.animate_bool_with_time(id.with("spell-ring"), ring, MARK_FADE);
    let dot_alpha = ctx.animate_bool_with_time(id.with("spell-dot"), dot, MARK_FADE);
    let painter = ui.painter();
    let radius = portrait_rect.width() / 2.0;
    if ring_alpha > 0.0 {
        painter.circle_stroke(
            portrait_rect.center(),
            radius + RING_WIDTH / 2.0,
            egui::Stroke::new(RING_WIDTH, RING_COLOR.gamma_multiply(ring_alpha)),
        );
    }
    if dot_alpha > 0.0 {
        // Sur le bord du portrait, à 45° en haut à gauche — l'opposé exact du pourcentage.
        let offset = radius * std::f32::consts::FRAC_1_SQRT_2;
        let center = portrait_rect.center() - egui::vec2(offset, offset);
        painter.circle(
            center,
            DOT_RADIUS,
            GOLD.gamma_multiply(dot_alpha),
            egui::Stroke::new(1.0, DOT_OUTLINE.gamma_multiply(dot_alpha)),
        );
    }
}

/// Hauteur du bloc pour `rows` rangées de sorts (au moins une) — voir doc de module.
fn block_height(rows: usize) -> f32 {
    let rows = rows.max(1) as f32;
    PADDING + SPELL_SIZE * rows + SPELL_GAP * (rows - 1.0) + PADDING
}

/// Peint le bloc à la position courante de `ui` (colonne des barres, après le dernier groupe et
/// `BLOCK_GAP`) pour l'allié `selection.selected`.
pub fn show(
    ui: &mut egui::Ui,
    fight: &FightSnapshot,
    selection: SpellSelection,
    spells: &SpellIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
) {
    let Some(selected_ally) = fight.fighters.get(selection.selected) else {
        return;
    };
    let casts = &selected_ally.last_turn_casts;
    let rows = casts.len().div_ceil(SPELLS_PER_ROW);

    let (block, _) = ui.allocate_exact_size(
        egui::vec2(BLOCK_WIDTH, block_height(rows)),
        egui::Sense::hover(),
    );
    let painter = ui.painter().clone();
    painter.rect_filled(block, LEADER_PANEL_ROUNDING, LEADER_PANEL_FILL);

    for (i, cast) in casts.iter().enumerate() {
        let row = i / SPELLS_PER_ROW;
        let col = i % SPELLS_PER_ROW;
        let rect = egui::Rect::from_min_size(
            block.min
                + egui::vec2(
                    PADDING + col as f32 * (SPELL_SIZE + SPELL_GAP),
                    PADDING + row as f32 * (SPELL_SIZE + SPELL_GAP),
                ),
            egui::Vec2::splat(SPELL_SIZE),
        );
        let response = ui.interact(
            rect,
            ui.id().with(("spell-block-spell", selection.selected, i)),
            egui::Sense::hover(),
        );

        let entry = spells.find(&cast.spell, selected_ally.class_name.as_deref());
        let display_name = entry.map_or(cast.spell.as_str(), |e| e.name.as_str());
        // Sort inconnu du référentiel → pavé « ? » ; connu mais sans image (`icon: None`) ou pas
        // encore téléchargé → tuile sombre sans « ? », le nom reste en infobulle.
        let texture = entry
            .and_then(|e| e.icon.as_ref())
            .and_then(|icon| remote_icon_textures.resolve(ui.ctx(), remote_icons, icon));
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

        let critical = cast.critical;
        let display_name = display_name.to_string();
        design::tooltip(&response).show(|ui| {
            ui.set_max_width(ui.spacing().tooltip_width);
            // Une seule ligne, deux couleurs : le nom en blanc, « · Critique » en doré derrière.
            let font = egui::TextStyle::Body.resolve(ui.style());
            let mut job = egui::text::LayoutJob::default();
            job.append(
                &display_name,
                0.0,
                egui::TextFormat::simple(font.clone(), tokens::TOOLTIP_TEXT),
            );
            if critical {
                job.append(" · Critique", 0.0, egui::TextFormat::simple(font, GOLD));
            }
            ui.label(job);
        });
    }
}

/// Badge d'index en haut à gauche de l'icône (1 px du bord gauche et du haut — voir
/// `BADGE_INSET_X`/`BADGE_INSET_Y`) : pastille noire à 62 %, rayon 2, 12 × 11 minimum (s'élargit
/// pour deux chiffres), chiffre blanc 9 px semi-gras sans contour.
fn paint_index_badge(ui: &egui::Ui, painter: &egui::Painter, icon: egui::Rect, index: usize) {
    let galley = painter.layout_no_wrap(
        index.to_string(),
        text::label_strong_font(ui.ctx(), BADGE_FONT_SIZE),
        egui::Color32::WHITE,
    );
    let width = (galley.size().x + 2.0 * BADGE_PADDING_X).max(BADGE_MIN_WIDTH);
    let badge = egui::Rect::from_min_size(
        icon.min + egui::vec2(BADGE_INSET_X, BADGE_INSET_Y),
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

/// Un `FighterDamage` est-il cette entrée de `fight.fighters` — les tranches passées au cadre
/// (`framed`, liste plate) sont des références dans ce même tableau, l'identité de pointeur suffit.
pub fn fighter_index(fight: &FightSnapshot, fighter: &FighterDamage) -> Option<usize> {
    fight.fighters.iter().position(|f| std::ptr::eq(f, fighter))
}

#[cfg(test)]
mod tests {
    use super::*;
    use overlay_engine::{Gender, SpellCastRecord};

    fn ally(name: &str, casts: usize) -> FighterDamage {
        FighterDamage {
            name: name.to_string(),
            is_ally: true,
            total_damage: 0,
            total_heal: 0,
            class_name: None,
            gender: Gender::M,
            xp_gained: 0,
            spells: Default::default(),
            is_ko: false,
            last_turn_casts: (0..casts)
                .map(|i| SpellCastRecord {
                    spell: format!("Sort {i}"),
                    critical: false,
                })
                .collect(),
            last_turn: 1,
        }
    }

    fn fight(fighters: Vec<FighterDamage>, last_ally_caster: Option<usize>) -> FightSnapshot {
        FightSnapshot {
            fight_id: 1,
            ongoing: true,
            result: None,
            fighters,
            started_at_ms: 0,
            last_ally_caster,
        }
    }

    #[test]
    fn hauteur_du_bloc_par_rangee() {
        assert_eq!(block_height(0), 40.0);
        assert_eq!(block_height(1), 40.0);
        assert_eq!(block_height(2), 77.0);
        assert_eq!(block_height(3), 114.0);
    }

    /// Cinq icônes et leurs liserés de critique (2 px hors de l'icône de chaque côté) tiennent dans
    /// la largeur utile, et deux critiques voisins gardent un jour entre eux — voir `SPELL_GAP`.
    #[test]
    fn cinq_icones_tiennent_et_deux_critiques_voisins_ne_se_touchent_pas() {
        let row_width =
            SPELLS_PER_ROW as f32 * SPELL_SIZE + (SPELLS_PER_ROW as f32 - 1.0) * SPELL_GAP;
        assert!(row_width <= INNER_WIDTH, "{row_width} > {INNER_WIDTH}");
        assert!(
            SPELL_GAP > 2.0 * 2.0,
            "deux liserés de 2 px se toucheraient"
        );
    }

    #[test]
    fn sans_sort_allie_aucune_selection() {
        let ctx = egui::Context::default();
        let f = fight(vec![ally("A", 0), ally("B", 0)], None);
        assert_eq!(selection(&ctx, &f), None);
    }

    /// Suivi automatique : liseré et point sur le dernier lanceur ; épingle sur un autre allié :
    /// le liseré reste, le point suit ; clic sur le porteur du point : retour au suivi.
    #[test]
    fn epingle_et_retour_au_suivi_automatique() {
        let ctx = egui::Context::default();
        let mut f = fight(vec![ally("A", 2), ally("B", 3), ally("C", 0)], Some(0));
        assert_eq!(
            selection(&ctx, &f),
            Some(SpellSelection {
                selected: 0,
                last_caster: 0
            })
        );

        on_portrait_clicked(&ctx, &f, 1); // épingle B
        assert_eq!(
            selection(&ctx, &f),
            Some(SpellSelection {
                selected: 1,
                last_caster: 0
            })
        );

        f.last_ally_caster = Some(0); // A rejoue : le point reste sur A, le liseré sur B
        assert_eq!(selection(&ctx, &f).unwrap().selected, 1);

        on_portrait_clicked(&ctx, &f, 2); // C n'a rien lancé : sans effet
        assert_eq!(selection(&ctx, &f).unwrap().selected, 1);

        on_portrait_clicked(&ctx, &f, 0); // clic sur le porteur du point : suivi automatique
        assert_eq!(
            selection(&ctx, &f),
            Some(SpellSelection {
                selected: 0,
                last_caster: 0
            })
        );

        on_portrait_clicked(&ctx, &f, 1); // épingle B…
        on_portrait_clicked(&ctx, &f, 1); // …second clic : suivi automatique
        assert_eq!(selection(&ctx, &f).unwrap().selected, 0);
    }

    /// L'épingle suit l'INDEX du combattant : le liseré reste sur lui même quand le point passe
    /// à un autre allié.
    #[test]
    fn le_point_suit_le_dernier_lanceur_sans_deplacer_l_epingle() {
        let ctx = egui::Context::default();
        let mut f = fight(vec![ally("A", 2), ally("B", 3)], Some(0));
        on_portrait_clicked(&ctx, &f, 1);
        f.last_ally_caster = Some(1); // B rejoue : point et liseré tous deux sur B
        assert_eq!(
            selection(&ctx, &f),
            Some(SpellSelection {
                selected: 1,
                last_caster: 1
            })
        );
        f.last_ally_caster = Some(0);
        assert_eq!(
            selection(&ctx, &f),
            Some(SpellSelection {
                selected: 1,
                last_caster: 0
            })
        );
    }
}
