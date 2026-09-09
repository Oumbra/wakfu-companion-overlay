//! **Libellés du design system** — la peinture du texte, mutualisée entre les composants.
//!
//! Un seul service pour l'instant, mais il concerne tous les composants qui portent un libellé
//! (bouton, onglet, en-tête de section, plus tard le panneau Combat) : la **graisse synthétique**.
//!
//! Pourquoi synthétique : egui n'embarque qu'une seule graisse de police proportionnelle
//! (`Ubuntu-Light`, du crate `epaint_default_fonts`), et son API de mise en forme n'expose aucun
//! réglage de graisse — il n'y a donc rien à *demander*, ni « bold » ni « semibold ». Or les
//! libellés gravés dans les captures du jeu sont nettement plus gras que cette Light : le
//! constater a été le point 3 du relevé de la modale Options (« la police du libellé »), et le
//! retour utilisateur a été de la rendre « un petit peu plus grasse » en attendant un vrai fichier
//! de police (chantier séparé, §6.4 du design-system).
//!
//! Comment : la même galley est repeinte huit fois autour de sa position, à **un pixel exactement**,
//! avec une opacité réduite ; puis une dernière fois au centre, à pleine opacité. Le trait
//! s'épaissit d'une fraction de pixel visuelle, réglée par cette opacité.
//!
//! Pourquoi l'opacité et pas le rayon — c'est la seule voie praticable : egui arrondit la position
//! d'un texte au pixel entier (`Options::round_text_to_pixels`, actif par défaut, c'est ce qui rend
//! le texte net). Un anneau à 0,3px de rayon est donc rigoureusement identique à un anneau à 0 —
//! mesuré : les corps 17px aux graisses 0,15 / 0,3 / 0,45 rendaient **exactement** la même image.
//! Le rayon d'un pixel est le plus petit pas possible, et il est déjà trop gros pour une encre de
//! 13px ; c'est l'opacité du halo qui redonne le réglage fin.
//!
//! Conséquence sur l'appelant : la galley doit être mise en page avec `Color32::PLACEHOLDER` pour
//! que chaque passe puisse imposer sa propre couleur (convention egui). `weighted` s'en charge à
//! l'appel, mais une galley mise en page avec une couleur en dur ignorerait le halo.
//!
//! Ce que ça ne fait **pas** : dessiner une vraie graisse. Les pleins et les déliés s'épaississent
//! de la même quantité, là où une Bold dessinée redistribue les contrastes. À l'échelle d'un
//! libellé de bouton (13px d'encre) la différence ne se voit pas ; sur un titre de 30px, elle se
//! verrait — c'est la limite à garder en tête avant de réutiliser ce module pour du gros texte.

use std::sync::Arc;

use egui::{Color32, Galley, Painter, Pos2, Vec2};

/// Décalages du halo, en pixels — les huit voisins immédiats. Entiers par construction : voir la
/// doc de module, un décalage fractionnaire serait arrondi et n'aurait aucun effet.
const HALO: [(f32, f32); 8] = [
    (-1.0, -1.0),
    (0.0, -1.0),
    (1.0, -1.0),
    (-1.0, 0.0),
    (1.0, 0.0),
    (-1.0, 1.0),
    (0.0, 1.0),
    (1.0, 1.0),
];

/// Peint `galley` en `pos` avec une graisse synthétique.
///
/// `weight` est l'**opacité du halo**, de 0 (aucune graisse ajoutée : exactement
/// `Painter::galley`, sans surcoût) à 1 (halo opaque, soit un plein pixel d'épaississement de
/// chaque côté). Le coût maximal est de neuf draw calls pour un libellé — négligeable devant le
/// nombre de libellés affichés simultanément par l'overlay, et payé uniquement par les composants
/// du design system.
///
/// `galley` doit avoir été mise en page avec `Color32::PLACEHOLDER` (voir la doc de module).
pub fn weighted(painter: &Painter, pos: Pos2, galley: Arc<Galley>, color: Color32, weight: f32) {
    let weight = weight.clamp(0.0, 1.0);
    if weight > 0.0 {
        let halo = color.gamma_multiply(weight);
        for (dx, dy) in HALO {
            painter.galley(pos + Vec2::new(dx, dy), galley.clone(), halo);
        }
    }
    painter.galley(pos, galley, color);
}

/// Graisse d'un libellé de composant — voir `tokens::TEXT_WEIGHT`.
///
/// Fonction plutôt que lecture directe du jeton : le jour où la graisse dépendra du corps (un
/// titre de 40px n'a pas besoin du même halo qu'un libellé de 16px), seul ce corps de fonction
/// change.
pub fn label_weight() -> f32 {
    crate::design::tokens::TEXT_WEIGHT
}
