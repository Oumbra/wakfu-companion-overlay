//! **Curseur de réglage** du design system Wakfu — la rainure et son disque, celui des volumes de
//! l'onglet Son. Composant **feuille** (§1 du contrat).
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! // La valeur vit chez l'appelant, comme pour un champ de saisie.
//! if ui.add(design::slider(&mut state.volume)).changed() {
//!     audio.set_volume(state.volume);
//! }
//!
//! // Une plage autre que 0..=1, et les libellés d'extrémité, qui appartiennent à l'appelant.
//! ui.horizontal(|ui| {
//!     ui.label("50 %");
//!     ui.add_space(tokens::SLIDER_LABEL_GAP);
//!     ui.add(design::slider(&mut state.echelle).range(0.5..=2.0).width(200.0));
//! });
//! ```
//!
//! ## La rainure est un creux, pas une barre
//!
//! Elle n'a **pas de couleur propre** : elle assombrit le fond qui la porte. C'est une mesure et
//! non un choix de rendu — sur les deux curseurs de la capture, posés sur des fonds qui diffèrent
//! de deux niveaux, le rapport au fond est le même (0,735 sur le liseré, 0,853 à l'intérieur, sur
//! les trois canaux). Deux constantes opaques auraient reproduit la capture et rien d'autre : elles
//! seraient fausses dès qu'un panneau change de teinte. Voir [`tokens::SLIDER_TRACK_EDGE_SHADE`].
//!
//! ## Ce que la capture ne dit pas, et qu'on n'invente donc pas
//!
//! **Les deux curseurs du jeu sont au minimum.** Rien ne montre ce que devient la portion
//! parcourue de la rainure : ce composant **ne la remplit pas**. Peindre un remplissage doré, si
//! naturel que ce soit dans d'autres interfaces, serait inventer du design — exactement ce que
//! [`design::input`](super::input) refuse de faire pour son état survolé, et pour la même raison.
//! La poignée porte déjà la valeur.
//!
//! Faute de référence également, et signalés comme tels :
//!
//! - **Survolé** : rien ne change. Seul le curseur de la souris passe à
//!   [`egui::CursorIcon::ResizeHorizontal`] — un comportement de plateforme, pas une décision
//!   esthétique.
//! - **Désactivé** : la poignée est teintée en [`tokens::TEXT_DISABLED`], par cohérence avec le
//!   bouton et le champ désactivés.
//!
//! ## Pourquoi la poignée est un asset
//!
//! Ses trois teintes sont plates — pas le moindre dégradé — et pourtant elle n'est pas peinte en
//! code : son relief est **directionnel**. Le liseré extérieur est clair en haut à gauche, mi-ton
//! en bas à droite, séparé du cœur par un sillon sombre. Reproduire ça demanderait deux arcs
//! partiels pour dix-huit pixels. Voir [`DsTexture::SliderHandle`].
//!
//! ## La géométrie, et le seul piège qu'elle contient
//!
//! La poignée fait 18 px et la rainure 8 : c'est **la poignée qui fixe la hauteur** du composant,
//! sans quoi son disque serait coupé. Et son centre parcourt la rainure **en retrait de son propre
//! rayon** à chaque extrémité — à valeur nulle, le disque affleure le bord gauche de la rainure au
//! lieu de le dépasser. La course utile vaut donc `largeur − 18`, ce qui rend une largeur inférieure
//! à 18 px dégénérée : [`track_travel`] rend alors 0 et la poignée reste centrée.

use std::ops::RangeInclusive;

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::design::{tokens, DesignSystem, DsTexture};

/// Course utile du **centre** de la poignée, pour une rainure de cette largeur.
///
/// Fonction libre et testée parce que c'est le seul calcul du composant qui puisse se tromper en
/// silence : une course égale à la largeur pleine ferait sortir le disque de la rainure aux deux
/// bouts, d'un demi-diamètre, et le défaut ne se verrait qu'aux extrémités — là où on regarde le
/// moins.
pub fn track_travel(width: f32) -> f32 {
    (width - tokens::SLIDER_HANDLE_SIZE).max(0.0)
}

/// État visuel d'un curseur — les trois états du design system, `Hovered` étant identique à `Idle`
/// faute de référence (doc de module).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderState {
    Idle,
    Hovered,
    Disabled,
}

/// Construit un curseur sur `value`, dans la plage `0.0..=1.0`.
pub fn slider(value: &mut f32) -> Slider<'_> {
    Slider {
        value,
        range: 0.0..=1.0,
        width: None,
        enabled: true,
        tooltip: None,
        log_name: None,
        forced_state: None,
        forced_fraction: None,
    }
}

/// Voir [`slider`].
pub struct Slider<'a> {
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    width: Option<f32>,
    enabled: bool,
    tooltip: Option<String>,
    log_name: Option<String>,
    forced_state: Option<SliderState>,
    forced_fraction: Option<f32>,
}

impl<'a> Slider<'a> {
    /// Plage de la valeur. Par défaut `0.0..=1.0`. Une plage vide ou inversée est traitée comme une
    /// plage nulle : la poignée reste à gauche plutôt que de produire un `NaN`.
    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        self.range = range;
        self
    }

    /// Largeur imposée. Par défaut, toute la largeur disponible — comme un champ de saisie, et pour
    /// la même raison : un curseur n'a pas de contenu qui puisse dicter sa largeur.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Nom d'instance pour le journal. Par défaut `"slider"`.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Force l'état peint — **galerie et captures uniquement** : en rendu offscreen aucun pointeur
    /// ne survole quoi que ce soit.
    pub fn preview_state(mut self, state: SliderState) -> Self {
        self.forced_state = Some(state);
        self
    }

    /// Force la position de la poignée, en fraction de la course — **galerie et captures
    /// uniquement**. Montrer plusieurs positions sur une planche demanderait autrement autant de
    /// valeurs mutables que de curseurs, pour une image.
    pub fn preview_fraction(mut self, fraction: f32) -> Self {
        self.forced_fraction = Some(fraction.clamp(0.0, 1.0));
        self
    }

    /// Fraction de la course que représente la valeur courante, dans `0.0..=1.0`.
    fn fraction(&self) -> f32 {
        if let Some(forced) = self.forced_fraction {
            return forced;
        }
        let (lo, hi) = (*self.range.start(), *self.range.end());
        // Une plage nulle ou inversée : pas de division, pas de `NaN` qui se propagerait jusqu'à la
        // position peinte — la poignée reste à gauche, et la valeur ne bouge pas non plus.
        if hi <= lo {
            return 0.0;
        }
        ((*self.value - lo) / (hi - lo)).clamp(0.0, 1.0)
    }
}

impl Widget for Slider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let height = tokens::SLIDER_HANDLE_SIZE;
        let sense = if self.enabled {
            Sense::click_and_drag()
        } else {
            // Un curseur désactivé garde `hover` pour son infobulle, qui explique souvent
            // *pourquoi* il est grisé, mais perd le glisser : `dragged()` ne peut alors
            // structurellement pas être vrai, plutôt que d'être filtré après coup.
            Sense::hover()
        };
        let (rect, mut response) = ui.allocate_exact_size(Vec2::new(width.max(0.0), height), sense);

        let travel = track_travel(rect.width());
        let radius = tokens::SLIDER_HANDLE_SIZE / 2.0;

        // Interaction — lue AVANT la peinture, pour que la frame où l'on saisit montre déjà la
        // nouvelle position plutôt que l'ancienne.
        let mut fraction = self.fraction();
        if self.enabled && (response.dragged() || response.clicked()) {
            if let Some(pos) = response.interact_pointer_pos() {
                // Le pointeur désigne le CENTRE de la poignée, pas son bord : on retire le rayon
                // avant de ramener dans la course. Sans ça, saisir le disque en son milieu le
                // ferait sauter d'un rayon vers la droite au premier mouvement.
                let x = if travel > 0.0 {
                    ((pos.x - rect.left() - radius) / travel).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                if (x - fraction).abs() > f32::EPSILON {
                    fraction = x;
                    let (lo, hi) = (*self.range.start(), *self.range.end());
                    if hi > lo {
                        *self.value = lo + x * (hi - lo);
                        response.mark_changed();
                        tracing::debug!(
                            component = "slider",
                            name = self.log_name.as_deref().unwrap_or("slider"),
                            valeur = *self.value,
                            "déplacement"
                        );
                    }
                }
            }
        }

        let state = self.forced_state.unwrap_or({
            if !self.enabled {
                SliderState::Disabled
            } else if response.hovered() {
                SliderState::Hovered
            } else {
                SliderState::Idle
            }
        });

        if state != SliderState::Disabled && response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }

        if ui.is_rect_visible(rect) {
            // La rainure, centrée dans la hauteur que la poignée impose.
            let track = egui::Rect::from_center_size(
                rect.center(),
                Vec2::new(rect.width(), tokens::SLIDER_TRACK_HEIGHT),
            );
            let painter = ui.painter();
            // **Trois bandes disjointes, et surtout pas deux superposées.** La première version
            // peignait le liseré sur toute la hauteur puis l'intérieur par-dessus : les deux
            // alphas se composent (0,733 × 0,851 = 0,624), et l'intérieur sortait bien plus sombre
            // que le liseré au lieu d'être plus clair — un creux à l'envers. La comparaison au jeu
            // l'a montré, aucune relecture ne l'aurait fait.
            let edge = tokens::SLIDER_TRACK_EDGE;
            let inner = track.shrink2(Vec2::new(0.0, edge));
            for band in [
                egui::Rect::from_min_max(
                    track.left_top(),
                    egui::pos2(track.right(), track.top() + edge),
                ),
                egui::Rect::from_min_max(
                    egui::pos2(track.left(), track.bottom() - edge),
                    track.right_bottom(),
                ),
            ] {
                painter.rect_filled(band, 0.0, tokens::SLIDER_TRACK_EDGE_SHADE);
            }
            painter.rect_filled(inner, 0.0, tokens::SLIDER_TRACK_SHADE);

            let center_x = rect.left() + radius + fraction * travel;
            let handle = egui::Rect::from_center_size(
                egui::pos2(center_x, rect.center().y),
                Vec2::splat(tokens::SLIDER_HANDLE_SIZE),
            );
            let tint = match state {
                SliderState::Idle | SliderState::Hovered => egui::Color32::WHITE,
                SliderState::Disabled => tokens::TEXT_DISABLED,
            };
            DesignSystem::get(ui.ctx()).paint(painter, handle, DsTexture::SliderHandle, tint);
        }

        match self.tooltip {
            Some(tooltip) => response.on_hover_text(tooltip),
            None => response,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_course_retire_le_diametre_de_la_poignee() {
        assert_eq!(track_travel(200.0), 200.0 - 18.0);
        assert_eq!(track_travel(tokens::SLIDER_HANDLE_SIZE), 0.0);
    }

    #[test]
    fn une_rainure_plus_etroite_que_la_poignee_ne_rend_pas_une_course_negative() {
        // Une course négative inverserait le sens du curseur : la poignée reculerait quand la
        // valeur monte. Elle reste immobile, et l'anomalie se voit sur la galerie.
        assert_eq!(track_travel(4.0), 0.0);
        assert_eq!(track_travel(0.0), 0.0);
    }

    #[test]
    fn la_poignee_tient_dans_la_rainure_aux_deux_extremites() {
        // La propriété que `track_travel` existe pour garantir, vérifiée sur la largeur du jeu.
        let width = tokens::SLIDER_TRACK_WIDTH_REF;
        let radius = tokens::SLIDER_HANDLE_SIZE / 2.0;
        let travel = track_travel(width);
        for fraction in [0.0, 0.5, 1.0] {
            let center = radius + fraction * travel;
            assert!(
                center - radius >= -0.01 && center + radius <= width + 0.01,
                "à {fraction}, le disque sort de la rainure",
            );
        }
    }

    #[test]
    fn le_lisere_creuse_plus_que_l_interieur() {
        // Le liseré creuse plus que l'intérieur — c'est ce qui fait lire un creux plutôt qu'une
        // bande plate. Inverser les deux jetons par étourderie donnerait une bosse.
        assert!(
            tokens::SLIDER_TRACK_EDGE_SHADE.a() > tokens::SLIDER_TRACK_SHADE.a(),
            "le liseré doit assombrir plus que l'intérieur",
        );
    }

    #[test]
    fn les_deux_liseres_tiennent_dans_la_hauteur_de_la_rainure() {
        assert!(
            tokens::SLIDER_TRACK_EDGE * 2.0 < tokens::SLIDER_TRACK_HEIGHT,
            "il ne resterait rien de l'intérieur",
        );
    }
}
