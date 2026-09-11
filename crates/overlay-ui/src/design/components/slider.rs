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
//! ## Les graduations, et pourquoi elles ne sont pas systématiques
//!
//! Un curseur gradué annonce **où la poignée peut s'immobiliser** ; c'est ce que
//! [`Slider::steps`] déclare, et sans lui le curseur est continu et nu. La distinction n'est pas
//! une commodité d'API : elle est mesurée. Le curseur d'échelle d'interface
//! (`interfaces/interface-options-interface.png`) porte **26 graduations**, celui du volume
//! (`interface-options-son.png`) **aucune** — pas un pixel clair le long de sa rainure.
//!
//! Et les graduations tombent bien sur les arrêts de la poignée : les trois libellés de la capture
//! (« 67 % », « 100 % », « 233 % ») sont centrés à x = 114, 199,5 et 539,5, pour des graduations à
//! 111, 196 et 536 — la première, la sixième et la vingt-sixième. Le disque, lui, est exactement
//! sur la sixième.
//!
//! **Une graduation ne traverse pas la rainure.** Elle la coupe : 3 px visibles au-dessus, 3 en
//! dessous — 2 de débord et **1 seul** de morsure sur un liseré qui en fait 2 — et entre les deux,
//! les pixels d'une colonne graduée sont identiques à ceux d'une colonne nue. D'où deux segments
//! peints, jamais un trait. Mordre le liseré entier donnerait 4 px de segment et 4 px de rainure
//! nue au lieu de 3 et 6 : assez pour que le repère se lise comme un trait presque continu.
//!
//! **Un écart assumé avec la capture** : dans le jeu, les graduations s'arrêtent à 37 px des bords
//! de la rainure (elles courent de x=111 à 536 pour une rainure de 74 à 573), soit 28 px de plus
//! que le rayon de la poignée. Ce composant ne reproduit pas ce retrait : les graduations occupent
//! toute la course du centre. Une seule capture d'un curseur gradué ne permet pas de savoir si ces
//! 37 px sont une valeur absolue ou une fraction de la largeur, et les extrapoler ferait un retrait
//! de 74 px sur une rainure de 120. Trancher demande une seconde capture, à une autre largeur.
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

/// Ramène une fraction sur le cran le plus proche, pour `steps` valeurs sélectionnables.
///
/// Fonction libre et testée pour la même raison que [`track_travel`] : une erreur d'unité ici — le
/// nombre de crans au lieu du nombre d'intervalles — décale toutes les graduations sauf la
/// première, d'un peu plus à chaque cran, et la dernière tombe alors *avant* le bout de la course.
/// Avec 26 crans il y a **25** intervalles.
///
/// Moins de deux crans rend la fraction inchangée : il n'y a rien à quantifier, et surtout pas de
/// division par zéro.
pub fn snap_to_step(fraction: f32, steps: usize) -> f32 {
    if steps < 2 {
        return fraction;
    }
    let intervals = (steps - 1) as f32;
    (fraction * intervals).round() / intervals
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
        steps: None,
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
    steps: Option<usize>,
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

    /// Nombre de **valeurs sélectionnables**, graduations comprises. Sans cet appel, le curseur est
    /// continu et n'affiche aucune graduation.
    ///
    /// C'est la distinction que fait le jeu, et elle est mesurée : le curseur d'échelle
    /// d'interface porte 26 graduations, celui du volume **aucune**. Un curseur gradué annonce ses
    /// arrêts ; un curseur continu n'en a pas à annoncer.
    ///
    /// La valeur est quantifiée sur ces crans — la poignée ne s'immobilise que sur une graduation,
    /// jamais entre deux. Moins de deux crans n'a pas de sens (il n'y aurait rien à choisir) et est
    /// traité comme un curseur continu, avec une ligne dans le journal.
    pub fn steps(mut self, steps: usize) -> Self {
        self.steps = Some(steps);
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

        // Moins de deux crans : il n'y aurait rien à choisir. Traité comme un curseur continu
        // plutôt que comme une erreur — mais dit une fois, parce que c'est presque sûrement un
        // `steps` calculé qui est tombé à zéro.
        let steps = self.steps.filter(|&n| {
            if n >= 2 {
                return true;
            }
            let warned_id = response.id.with("ds-slider-steps");
            let already = ui.ctx().data_mut(|d| {
                let seen = d.get_temp::<bool>(warned_id).unwrap_or(false);
                d.insert_temp(warned_id, true);
                seen
            });
            if !already {
                tracing::warn!(
                    component = "slider",
                    name = self.log_name.as_deref().unwrap_or("slider"),
                    crans = n,
                    "moins de deux crans, le curseur reste continu et sans graduation"
                );
            }
            false
        });

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
                let x = match steps {
                    Some(steps) => snap_to_step(x, steps),
                    None => x,
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
            // La rainure, centrée dans la hauteur que la poignée impose — et **calée sur la grille
            // de pixels**. Le `Ui` qui nous alloue peut tomber sur un demi-pixel (les espacements
            // d'egui sont des `f32`), et un creux de 8 px posé à y,5 s'étale alors sur 9 lignes,
            // débord des graduations compris : mesuré, cela donnait des repères de 4 px là où le
            // jeu en montre 3, et 5 px de rainure nue au lieu de 6.
            let track_top = (rect.center().y - tokens::SLIDER_TRACK_HEIGHT / 2.0).round();
            let track = egui::Rect::from_min_size(
                egui::pos2(rect.left(), track_top),
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

            // Les graduations, peintes APRÈS la rainure et JAMAIS dans son intérieur : sur la
            // capture, les pixels d'une colonne graduée sont identiques à ceux d'une colonne nue
            // entre les deux liserés. Chaque graduation est donc deux segments, pas un trait.
            if let Some(steps) = steps {
                let over = tokens::SLIDER_TICK_OVERHANG;
                let half = tokens::SLIDER_TICK_WIDTH / 2.0;
                let intervals = (steps - 1) as f32;
                for i in 0..steps {
                    // Les graduations marquent **les arrêts de la poignée**, donc la course de son
                    // centre — c'est ce qui leur donne leur sens : l'utilisateur voit où elle peut
                    // s'immobiliser. Vérifié sur la capture, où le disque est exactement sur la
                    // sixième.
                    let x = rect.left() + radius + (i as f32 / intervals) * travel;
                    for band in [
                        egui::Rect::from_min_max(
                            egui::pos2(x - half, track.top() - over),
                            egui::pos2(x + half, track.top() + tokens::SLIDER_TICK_BITE),
                        ),
                        egui::Rect::from_min_max(
                            egui::pos2(x - half, track.bottom() - tokens::SLIDER_TICK_BITE),
                            egui::pos2(x + half, track.bottom() + over),
                        ),
                    ] {
                        painter.rect_filled(band, 0.0, tokens::SLIDER_TICK);
                    }
                }
            }

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
    fn la_quantification_compte_les_intervalles_pas_les_crans() {
        // 26 crans, donc 25 intervalles : la dernière graduation tombe à 1,0 exactement. Compter
        // 26 intervalles la ferait tomber à 25/26 = 0,96, avant le bout de la course.
        assert_eq!(snap_to_step(1.0, 26), 1.0);
        assert_eq!(snap_to_step(0.0, 26), 0.0);
        assert!((snap_to_step(0.5, 3) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn la_quantification_prend_le_cran_le_plus_proche() {
        // 3 crans : 0, 0,5, 1. Juste au-dessus du quart, on bascule sur le cran du milieu.
        assert_eq!(snap_to_step(0.24, 3), 0.0);
        assert!((snap_to_step(0.26, 3) - 0.5).abs() < 1e-6);
        assert_eq!(snap_to_step(0.99, 3), 1.0);
    }

    #[test]
    fn moins_de_deux_crans_ne_quantifie_rien_et_ne_divise_pas_par_zero() {
        for steps in [0, 1] {
            assert_eq!(snap_to_step(0.37, steps), 0.37);
        }
    }

    #[test]
    fn une_graduation_fait_trois_pixels_et_en_laisse_six() {
        // Les deux cotes du jeu, verrouillées ensemble parce que c'est leur RAPPORT qui fait lire
        // deux repères plutôt qu'un trait presque continu : segment visible de 3 px (2 de débord
        // plus 1 de morsure), et 6 px de rainure nue entre les deux.
        let visible = tokens::SLIDER_TICK_OVERHANG + tokens::SLIDER_TICK_BITE;
        let nue = tokens::SLIDER_TRACK_HEIGHT - tokens::SLIDER_TICK_BITE * 2.0;
        assert_eq!(visible, 3.0, "segment visible");
        assert_eq!(nue, 6.0, "rainure nue entre les deux segments");
        assert!(
            tokens::SLIDER_TICK_BITE < tokens::SLIDER_TRACK_EDGE,
            "la graduation mord le liseré, elle ne le remplace pas",
        );
    }

    #[test]
    fn les_graduations_tiennent_dans_la_hauteur_du_composant() {
        assert!(
            tokens::SLIDER_TRACK_HEIGHT + tokens::SLIDER_TICK_OVERHANG * 2.0
                <= tokens::SLIDER_HANDLE_SIZE,
            "c'est la poignée qui fixe la hauteur, les graduations doivent y tenir",
        );
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
