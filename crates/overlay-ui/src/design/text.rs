//! **Libellés du design system** — le corps et la police d'un texte de composant.
//!
//! Un composant ne construit jamais son `FontId` lui-même : il passe par [`label_font`]. Le jour où
//! la police des libellés change (voir [`super::fonts`]), un seul corps de fonction est à toucher,
//! et aucun composant ne cite ni un nom de famille ni un nom de fichier.
//!
//! **Historique — la graisse synthétique a été retirée le 2026-09-09.** Ce module peignait
//! auparavant chaque galley neuf fois : une passe centrale à pleine opacité et huit voisins
//! immédiats à opacité réduite, pour épaissir le trait faute de graisse disponible dans `egui`. La
//! planche de comparaison a tranché sans appel — *un halo est un contour, pas une graisse* : il
//! épaissit le mot en dégradant son contraste, le libellé devient flou là où celui du jeu est net.
//! Un vrai fichier de police (`assets/fonts/Ubuntu-Medium.ttf`) fait le travail correctement, pour
//! neuf fois moins de draw calls. Le raisonnement complet est dans [`super::fonts`].
//!
//! Une conséquence de cet épisode mérite de survivre à la rustine, parce qu'elle se redécouvre
//! douloureusement : **`egui` arrondit la position d'un texte au pixel entier**
//! (`Options::round_text_to_pixels`, actif par défaut — c'est ce qui rend le texte net). Tout
//! décalage sous le pixel appliqué à une galley est donc sans effet ; mesuré à l'époque, les corps
//! 17px aux graisses 0,15 / 0,3 / 0,45 rendaient **exactement** la même image. Aucun effet visuel
//! ne peut être réglé par un déplacement fractionnaire de texte.

use egui::{Context, FontFamily, FontId};

/// Police d'un libellé de composant, au corps demandé.
///
/// **Retombe sur la proportionnelle par défaut si la famille n'est pas encore liée**, plutôt que de
/// laisser `epaint` paniquer (« FontFamily::Name(…) is not bound to any fonts »). Ce n'est pas de la
/// prudence gratuite, c'est le cas normal de la toute première passe : `Context::set_fonts` ne prend effet
/// qu'à la passe suivante, or [`crate::style::apply`] est appelé *depuis* la fermeture d'interface
/// dans les harnais de test (`overlay-testkit`) — à la toute première passe, un composant peint
/// donc légitimement avant que la police ne soit disponible. Les passes suivantes l'ont, et une
/// capture de non-régression est prise après stabilisation.
///
/// Le coût est une petite allocation par libellé (`Fonts::families` rend un `Vec` de trois ou
/// quatre entrées) — négligeable au regard du nombre de libellés affichés simultanément, et payé
/// uniquement par les composants du design system.
pub fn label_font(ctx: &Context, size: f32) -> FontId {
    let family = FontFamily::Name(super::fonts::LABEL.into());
    if ctx.fonts(|fonts| fonts.families().contains(&family)) {
        FontId::new(size, family)
    } else {
        FontId::proportional(size)
    }
}
