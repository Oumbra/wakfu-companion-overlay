//! Fenêtre des groupes de dégâts du panneau Combat — bornée à six groupes, défilante au-delà
//! (13 sept. 2026, demande utilisateur, maquette « Barres en brèche » validée en artefact
//! interactif, choix arrêtés en révision 2).
//!
//! **Le problème** : en breach, 40 à 60 monstres frappent ; un groupe nom + dégâts + barre par
//! combattant ayant infligé des dégâts faisait grandir la colonne sans limite, jusqu'à sortir de
//! la fenêtre — le bloc « ligne de sorts » avec elle. Le cadre des portraits, lui, était déjà
//! borné depuis le 2026-09-07 (`combat_frame_scroll` : le plus grand gabarit comme fenêtre fixe).
//!
//! **La règle** : le plus grand gabarit du cadre (six emplacements, `combat_frame::MAX_FRAME_SLOTS`)
//! reste la mesure de tout le panneau. Les groupes continuent de s'accumuler, mais dans un
//! conteneur INVISIBLE — ni bordure, ni fond, rien ne le trahit — haut de six groupes exactement
//! (`MAX_VISIBLE_GROUPS`), qui défile. En dessous de six groupes, la fenêtre se resserre sur son
//! contenu : jamais de vide sous le dernier groupe, et aucun pixel ne bouge par rapport à la liste
//! d'avant (les captures alliées de référence sont inchangées). Le plafond compte les GROUPES —
//! les combattants ayant frappé, triés par dégâts décroissants par l'appelant — pas les monstres :
//! quatorze monstres dont cinq ont frappé donnent cinq groupes et aucun ascenseur, alors que le
//! cadre des portraits, lui, défile déjà. La ligne leader (au-dessus) et le bloc « ligne de
//! sorts » (au-dessous) restent hors de la fenêtre, toujours visibles.
//!
//! **Ascenseur** : le même que celui du cadre, au pixel (`combat_scrollbar`, extrait pour
//! l'occasion) — à GAUCHE de la fenêtre, dans l'air qui sépare le cadre des barres, en face de
//! celui du cadre qui colle à son bord gauche (choix utilisateur, révision 2 : les deux ascenseurs
//! se font face, chacun collé à sa colonne, la largeur de la colonne des barres ne bouge pas).
//! Toujours visible dès qu'il y a plus de six groupes ; il n'existe donc jamais dans un combat
//! ordinaire. La molette agit au survol de la fenêtre et de l'ascenseur, en pixels.
//!
//! **Fondu aux bords** (choix utilisateur) : le conteneur étant invisible, un groupe coupé net
//! serait le seul indice qu'il continue. Les `FADE_HEIGHT` premiers et derniers pixels de la
//! fenêtre fondent vers le transparent, UNIQUEMENT du côté où du contenu est masqué — rien en
//! haut tant qu'on est au début, rien en bas une fois au bout. egui ne sait pas masquer en alpha
//! ce qui est déjà peint, et le fond du panneau est transparent (le jeu est derrière) : le fondu
//! est donc appliqué ÉLÉMENT par élément — la ligne nom + dégâts et la barre d'un groupe reçoivent
//! chacune une opacité égale à la moyenne du masque de fondu sur leur partie visible
//! (`fade_alpha`). Continu au défilement, mais par tranches de 13 et 16 px plutôt que par pixel ;
//! écart assumé avec la maquette, invisible à l'usage.
//!
//! **État de défilement** : décalage en PIXELS, persisté via `egui::Context::data_mut` sous
//! `scroll_offset_id` — même mécanisme et mêmes raisons que le cadre (voir sa doc de module) ;
//! commun aux deux camps affichés, comme celui du cadre. Culling : un groupe entièrement hors de
//! la fenêtre n'est pas peint.

use overlay_engine::FighterDamage;

use super::combat::{damage_group_height, paint_damage_bar_group, BAR_MAX_WIDTH, ROW_GAP};
use super::combat_scrollbar::{DrawnScrollbar, SCROLLBAR_HIT_WIDTH, SCROLLBAR_WIDTH};

/// Plafond de groupes visibles — le plus grand gabarit du cadre (`combat_frame::MAX_FRAME_SLOTS`),
/// réutilisé comme mesure : voir doc de module.
pub const MAX_VISIBLE_GROUPS: usize = super::combat_frame::MAX_FRAME_SLOTS;
/// Hauteur du fondu à chaque bord de la fenêtre, du côté où du contenu est masqué.
pub const FADE_HEIGHT: f32 = 8.0;
/// Air entre le bord droit du liseré de la poignée et le bord gauche des barres.
const SCROLLBAR_GAP: f32 = 4.0;
/// Marge horizontale laissée au clip de la fenêtre de part et d'autre des barres — le contour de
/// texte (`design::text::OUTLINE_FULL`, 1 px) doit pouvoir déborder comme avant.
const CLIP_SLACK_X: f32 = 2.0;
/// Décalage du bord gauche de la poignée par rapport à la fenêtre : liseré extérieur (1 px) +
/// poignée + liseré + air. Soit 10 px, la poignée occupant `−10..−5` (liseré : `−11..−4`).
const THUMB_OFFSET: f32 = SCROLLBAR_WIDTH + 1.0 + SCROLLBAR_GAP;

/// Clé du décalage courant, en pixels (0 = premier groupe aligné sur le haut de la fenêtre).
pub fn scroll_offset_id() -> egui::Id {
    egui::Id::new("combat-bars-scroll-offset")
}

/// Voir la doc de module.
pub struct DamageBars;

impl DamageBars {
    /// Peint les groupes de `bars` (déjà filtrés aux dégâts > 0 et triés par l'appelant) dans une
    /// fenêtre haute d'au plus `MAX_VISIBLE_GROUPS` groupes, défilante au-delà. `total_damage`
    /// est la référence du remplissage des barres (total du camp affiché, voir `combat::show`).
    /// Ne peint rien pour une liste vide.
    pub fn show(ui: &mut egui::Ui, bars: &[&FighterDamage], total_damage: i64) {
        let n = bars.len();
        if n == 0 {
            return;
        }
        // Rythme vertical STRICTEMENT identique à la liste d'avant (un `allocate_exact_size` par
        // ligne, `item_spacing` d'egui entre deux allocations, `ROW_GAP` ajouté entre deux
        // groupes) — c'est ce qui garde les captures alliées de référence au pixel près.
        let spacing = ui.spacing().item_spacing.y;
        let group_height = damage_group_height(ui);
        let pitch = group_height + spacing + ROW_GAP;
        let content_height = group_height * n as f32 + (pitch - group_height) * (n - 1) as f32;
        let visible = n.min(MAX_VISIBLE_GROUPS);
        let window_height =
            group_height * visible as f32 + (pitch - group_height) * (visible - 1) as f32;
        let width = ui.available_width().min(BAR_MAX_WIDTH);
        let (window, _) =
            ui.allocate_exact_size(egui::vec2(width, window_height), egui::Sense::hover());

        let max_offset = (content_height - window_height).max(0.0);
        let scrolls = max_offset > 0.0;
        let scroll_id = scroll_offset_id();
        let mut offset = if scrolls {
            ui.ctx()
                .data_mut(|d| *d.get_temp_mut_or_insert_with(scroll_id, || 0.0_f32))
        } else {
            0.0
        };
        if scrolls {
            // Molette au survol de la fenêtre OU de l'ascenseur (zone de saisie comprise) — en
            // pixels, comme un `ScrollArea` classique.
            let wheel_rect = egui::Rect::from_min_max(
                egui::pos2(window.min.x - SCROLLBAR_HIT_WIDTH, window.min.y),
                window.max,
            );
            let hovering = ui
                .interact(wheel_rect, ui.id().with("bars-hover"), egui::Sense::hover())
                .hovered();
            if hovering {
                offset -= ui.input(|i| i.smooth_scroll_delta.y);
            }
        }
        offset = offset.clamp(0.0, max_offset);

        // Fondu du côté où du contenu est masqué seulement (voir doc de module).
        let fade_top = scrolls && offset > 0.5;
        let fade_bottom = scrolls && offset < max_offset - 0.5;
        let fade_alpha = |rect: egui::Rect| fade_alpha(rect, window, fade_top, fade_bottom);

        // Groupes, peints dans la fenêtre (clip) à leur position défilée, dans un `Ui` enfant
        // créé par `new_child` — PAS `ui.scope` : un scope réalloue dans le parent le rectangle
        // utilisé par l'enfant, et chaque sous-scope d'opacité y laissait un `item_spacing`
        // fantôme qui décalait le bloc de sorts de 6 px (capture de référence à l'appui).
        // `new_child` n'alloue rien : la fenêtre a déjà été allouée ci-dessus.
        {
            // Le clip ne borne que VERTICALEMENT : le contour de 1 px des textes (nom à gauche,
            // dégâts à droite, `text::OUTLINE_FULL`) déborde d'un pixel de chaque côté de la
            // colonne, comme dans l'ancienne liste — le rogner changeait les captures de
            // référence. Rien à masquer horizontalement : l'ascenseur est à 4 px de là.
            let clip = window.expand2(egui::vec2(CLIP_SLACK_X, 0.0));
            let mut clipped = ui.new_child(egui::UiBuilder::new().max_rect(window));
            clipped.shrink_clip_rect(clip);
            let ui = &mut clipped;
            for (i, fighter) in bars.iter().enumerate() {
                let top = window.min.y - offset + pitch * i as f32;
                let group_rect = egui::Rect::from_min_size(
                    egui::pos2(window.min.x, top),
                    egui::vec2(width, group_height),
                );
                if group_rect.max.y < window.min.y || group_rect.min.y > window.max.y {
                    continue;
                }
                paint_damage_bar_group(
                    ui,
                    group_rect,
                    &fighter.name,
                    fighter.total_damage,
                    total_damage,
                    &fade_alpha,
                );
            }
        }

        if scrolls {
            let scrollbar = DrawnScrollbar {
                band: window,
                hit_left: window.min.x - SCROLLBAR_HIT_WIDTH,
                thumb_left: window.min.x - THUMB_OFFSET,
                visible_frac: window_height / content_height,
            };
            let position = scrollbar.show(ui, ui.id().with("bars-scroll-hit"), offset / max_offset);
            offset = (position * max_offset).clamp(0.0, max_offset);
            ui.ctx().data_mut(|d| d.insert_temp(scroll_id, offset));
        }
    }
}

/// Opacité d'un élément (ligne de texte ou barre) : moyenne, sur sa partie visible dans `window`,
/// du masque de fondu — 0 au bord, 1 à `FADE_HEIGHT` du bord, de chaque côté actif. 1 pour un
/// élément entièrement dans la zone pleine. Échantillonnée (huit points) plutôt qu'intégrée :
/// le masque est linéaire par morceaux, l'écart est négligeable à cette échelle.
fn fade_alpha(rect: egui::Rect, window: egui::Rect, fade_top: bool, fade_bottom: bool) -> f32 {
    if !fade_top && !fade_bottom {
        return 1.0;
    }
    let y0 = rect.min.y.max(window.min.y);
    let y1 = rect.max.y.min(window.max.y);
    if y1 <= y0 {
        return 0.0;
    }
    const SAMPLES: usize = 8;
    let mut sum = 0.0;
    for s in 0..SAMPLES {
        let y = y0 + (y1 - y0) * (s as f32 + 0.5) / SAMPLES as f32;
        let mut m = 1.0_f32;
        if fade_top {
            m *= ((y - window.min.y) / FADE_HEIGHT).clamp(0.0, 1.0);
        }
        if fade_bottom {
            m *= ((window.max.y - y) / FADE_HEIGHT).clamp(0.0, 1.0);
        }
        sum += m;
    }
    sum / SAMPLES as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(0.0, 100.0), egui::vec2(190.0, 212.0))
    }

    #[test]
    fn sans_fondu_actif_l_opacite_est_pleine() {
        let r = egui::Rect::from_min_size(egui::pos2(0.0, 100.0), egui::vec2(190.0, 13.0));
        assert_eq!(fade_alpha(r, window(), false, false), 1.0);
    }

    #[test]
    fn un_element_colle_au_bord_haut_fondu_est_a_moitie_transparent_environ() {
        // 13 px de texte à partir du bord : 8 px de rampe puis 5 px pleins → moyenne ≈ 0.69.
        let r = egui::Rect::from_min_size(egui::pos2(0.0, 100.0), egui::vec2(190.0, 13.0));
        let a = fade_alpha(r, window(), true, false);
        assert!((0.6..0.8).contains(&a), "{a}");
    }

    #[test]
    fn un_element_dans_la_zone_pleine_garde_toute_son_opacite() {
        let r = egui::Rect::from_min_size(egui::pos2(0.0, 150.0), egui::vec2(190.0, 16.0));
        assert_eq!(fade_alpha(r, window(), true, true), 1.0);
    }

    #[test]
    fn un_element_hors_fenetre_est_invisible() {
        let r = egui::Rect::from_min_size(egui::pos2(0.0, 80.0), egui::vec2(190.0, 16.0));
        assert_eq!(fade_alpha(r, window(), true, true), 0.0);
    }

    #[test]
    fn le_bas_ne_fond_que_si_demande() {
        let r = egui::Rect::from_min_size(egui::pos2(0.0, 296.0), egui::vec2(190.0, 16.0));
        assert_eq!(fade_alpha(r, window(), true, false), 1.0);
        // 16 px de barre collés au bord bas : 8 px pleins puis 8 px de rampe → moyenne 0,75.
        let a = fade_alpha(r, window(), true, true);
        assert!((0.7..0.8).contains(&a), "{a}");
    }
}
