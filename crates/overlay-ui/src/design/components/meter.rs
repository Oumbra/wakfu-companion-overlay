//! **Jauge** du design system — la barre de progression du panneau Combat : deux bordures
//! concentriques, une piste, un remplissage, son reflet et son curseur de fin. Composant **feuille**
//! (§1 du contrat).
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! ui.add(design::meter(0.42).width(190.0));
//! ui.add(design::meter(ratio).fill(couleur_calculée).width(190.0));
//! ```
//!
//! ## Six couches, et l'ordre ne suffit pas
//!
//! Contrairement à [`design::item_slot`](super::item_slot), dont le piège était l'ordre de
//! peinture, celui de la jauge est **géométrique** : chaque couche se déduit de la précédente par
//! un `shrink`, et son arrondi décroît d'autant. Écrire les rayons à la main donne une jauge dont
//! les coins ne sont pas concentriques — visible sur un arrondi de 4 px, où chaque pixel compte.
//!
//! | Couche | Rectangle | Arrondi |
//! | --- | --- | --- |
//! | bordure extérieure | le rectangle donné | 4 |
//! | bordure intérieure | `shrink(2)` | 2 |
//! | piste | `shrink(2)` encore | 0 |
//! | remplissage | fraction de la piste | voir ci-dessous |
//! | reflet | tiers supérieur du remplissage | coins hauts seulement |
//! | curseur de fin | 2 px à l'extrémité | 1 |
//!
//! ## L'arrondi conditionnel du remplissage
//!
//! **Les coins droits du remplissage ne s'arrondissent que s'il atteint le bout de la piste.**
//! Sinon son bord droit tombe au milieu de la piste, et un coin arrondi y serait faux : il
//! suggérerait un bord qui n'existe pas. C'est [`fill_corners`], une fonction libre que son test
//! verrouille — le défaut est invisible tant qu'on ne regarde pas une jauge à moitié pleine.
//!
//! Le **curseur de fin** disparaît pour la même raison à 100 % : il se confondrait avec le bord
//! droit de la piste sans rien apporter.
//!
//! ## La teinte du remplissage vient de l'appelant, le reflet non
//!
//! [`Meter::fill`] est un paramètre parce que le panneau Combat fait varier la couleur de sa barre
//! selon la part de dégâts. Le **reflet**, lui, est fixe ([`tokens::METER_HIGHLIGHT`]) : demandé
//! comme un ton précis (`#0dbebe`) plutôt que comme une dérivation du remplissage, après le retrait
//! d'un éclaircissement automatique qui ne donnait pas ce ton-là.

use egui::{CornerRadius, Response, Sense, Ui, Vec2, Widget};

use crate::design::tokens;

/// Seuil au-delà duquel une jauge est considérée pleine.
///
/// Pas `>= 1.0` : une fraction calculée en `f32` peut tomber à 0,9999998 pour un rapport qui vaut
/// exactement un, et la jauge afficherait alors un curseur de fin collé au bord droit avec deux
/// coins carrés — le défaut se verrait précisément sur le cas le plus fréquent, celui du premier
/// combattant du classement.
pub const FULL_THRESHOLD: f32 = 0.999;

/// Arrondis des quatre coins du remplissage.
///
/// Fonction libre et testée : c'est le seul calcul de ce composant qui puisse se tromper en
/// silence, et son erreur ne se voit que sur une jauge partiellement remplie.
pub fn fill_corners(full: bool, radius: u8) -> CornerRadius {
    CornerRadius {
        nw: radius,
        sw: radius,
        // Les coins DROITS suivent la piste seulement si le remplissage la touche.
        ne: if full { radius } else { 0 },
        se: if full { radius } else { 0 },
    }
}

/// Construit une jauge remplie à `fraction` (0 à 1, bornée).
pub fn meter(fraction: f32) -> Meter {
    Meter {
        fraction: fraction.clamp(0.0, 1.0),
        fill: tokens::METER_FILL,
        width: None,
        height: tokens::METER_HEIGHT,
    }
}

/// Voir [`meter`].
pub struct Meter {
    fraction: f32,
    fill: egui::Color32,
    width: Option<f32>,
    height: f32,
}

impl Meter {
    /// Teinte du remplissage. Par défaut [`tokens::METER_FILL`].
    pub fn fill(mut self, fill: egui::Color32) -> Self {
        self.fill = fill;
        self
    }

    /// Largeur imposée. Par défaut, toute la largeur disponible.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Hauteur. Par défaut [`tokens::METER_HEIGHT`], la cote de la maquette.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl Widget for Meter {
    fn ui(self, ui: &mut Ui) -> Response {
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width.max(0.0), self.height), Sense::hover());
        if ui.is_rect_visible(rect) {
            paint(ui, rect, self.fraction, self.fill);
        }
        response
    }
}

/// Peint les six couches dans `rect`.
///
/// Séparée de `Widget::ui` pour qu'un appelant qui pose déjà sa géométrie — une ligne de tableau,
/// une cellule de grille — puisse peindre une jauge sans passer par l'allocation d'egui.
pub fn paint(ui: &Ui, rect: egui::Rect, fraction: f32, fill: egui::Color32) {
    let painter = ui.painter().with_clip_rect(rect);
    let border = tokens::METER_BORDER_WIDTH;

    // Trois rectangles concentriques, dont les arrondis décroissent avec les marges : c'est ce qui
    // garde les coins parallèles. Les calculer indépendamment donne des bords qui divergent.
    painter.rect_filled(rect, tokens::METER_ROUNDING, tokens::METER_OUTER_BORDER);
    let inner = rect.shrink(border);
    let inner_rounding = (tokens::METER_ROUNDING - border).max(0.0);
    painter.rect_filled(inner, inner_rounding, tokens::METER_INNER_BORDER);
    let track = inner.shrink(border);
    let track_rounding = (inner_rounding - border).max(0.0);
    painter.rect_filled(track, track_rounding, tokens::METER_TRACK);

    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= 0.0 {
        return;
    }
    let full = fraction >= FULL_THRESHOLD;
    let corners = fill_corners(full, track_rounding as u8);
    let fill_rect = egui::Rect::from_min_size(
        track.min,
        Vec2::new(track.width() * fraction, track.height()),
    );
    painter.rect_filled(fill_rect, corners, fill);

    // Le reflet ne couvre que le haut du remplissage : ses coins bas restent carrés, sinon il
    // dessinerait un bord au milieu du remplissage.
    painter.rect_filled(
        egui::Rect::from_min_size(
            fill_rect.min,
            Vec2::new(
                fill_rect.width(),
                fill_rect.height() * tokens::METER_HIGHLIGHT_RATIO,
            ),
        ),
        CornerRadius {
            nw: corners.nw,
            ne: corners.ne,
            sw: 0,
            se: 0,
        },
        tokens::METER_HIGHLIGHT,
    );

    if !full {
        painter.rect_filled(
            egui::Rect::from_center_size(
                egui::pos2(fill_rect.max.x, track.center().y),
                Vec2::new(tokens::METER_END_CAP_WIDTH, track.height()),
            ),
            1.0,
            tokens::METER_END_CAP,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_remplissage_partiel_garde_ses_coins_droits_carres() {
        // **Le défaut que ce test verrouille** : un coin arrondi à droite d'un remplissage qui
        // s'arrête au milieu suggère un bord qui n'existe pas. Invisible sur une jauge pleine ou
        // vide — c'est-à-dire dans les deux cas qu'on regarde en premier.
        let c = fill_corners(false, 4);
        assert_eq!((c.nw, c.sw), (4, 4), "les coins gauches suivent la piste");
        assert_eq!((c.ne, c.se), (0, 0), "les coins droits restent carrés");
    }

    #[test]
    fn un_remplissage_complet_arrondit_les_quatre() {
        let c = fill_corners(true, 4);
        assert_eq!((c.nw, c.sw, c.ne, c.se), (4, 4, 4, 4));
    }

    #[test]
    fn le_seuil_de_pleine_tolere_l_arrondi_du_flottant() {
        // Un rapport qui vaut exactement un peut sortir à 0,9999998 en `f32`. Sans tolérance, la
        // jauge du premier combattant — le cas le plus fréquent — afficherait un curseur de fin
        // collé au bord et deux coins carrés.
        assert!(0.9999998_f32 >= FULL_THRESHOLD);
        assert!(0.99_f32 < FULL_THRESHOLD, "99 % n'est pas plein");
    }

    #[test]
    fn les_arrondis_decroissent_avec_les_marges() {
        // La propriété qui garde les trois rectangles concentriques : chaque couche perd en rayon
        // exactement ce qu'elle perd en marge. Les fixer indépendamment fait diverger les bords.
        let border = tokens::METER_BORDER_WIDTH;
        let inner = (tokens::METER_ROUNDING - border).max(0.0);
        let track = (inner - border).max(0.0);
        assert!(tokens::METER_ROUNDING > inner && inner >= track);
        assert!(track >= 0.0, "un rayon négatif inverserait la courbure");
    }

    #[test]
    fn une_jauge_plus_fine_que_ses_bordures_ne_produit_pas_de_rayon_negatif() {
        // Deux bordures de 2 px sur un arrondi de 4 : la piste tombe pile à 0. Une jauge plus
        // arrondie ou moins bordée ne doit pas passer sous zéro.
        for rounding in [0.0_f32, 1.0, 4.0, 12.0] {
            let inner = (rounding - tokens::METER_BORDER_WIDTH).max(0.0);
            let track = (inner - tokens::METER_BORDER_WIDTH).max(0.0);
            assert!(track >= 0.0, "rayon négatif pour un arrondi de {rounding}");
        }
    }
}
