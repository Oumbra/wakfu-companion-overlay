//! **Textes du design system** — la police d'un texte de composant, et son cerne.
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

use egui::{Color32, Context, FontFamily, FontId, Pos2, Vec2};

/// Police de la famille nommée `nom`, au corps demandé, **avec repli** sur la proportionnelle par
/// défaut si la famille n'est pas encore liée — voir [`label_font`] pour le pourquoi de ce repli.
fn famille(ctx: &Context, nom: &str, size: f32) -> FontId {
    let family = FontFamily::Name(nom.into());
    if ctx.fonts(|fonts| fonts.families().contains(&family)) {
        FontId::new(size, family)
    } else {
        FontId::proportional(size)
    }
}

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
    famille(ctx, super::fonts::LABEL, size)
}

/// Police d'un titre (bannière de modale, titre de section), au corps demandé — la serif grasse du
/// jeu, voir [`super::fonts`]. Même repli que [`label_font`].
pub fn title_font(ctx: &Context, size: f32) -> FontId {
    famille(ctx, super::fonts::TITLE, size)
}

/// Couleur du cerne, quel que soit le jeu de décalages : **noir plein**. C'est le procédé du jeu
/// lui-même pour ses incrustations, et la mesure de son titre de modale le confirme — le texte est
/// blanc pur (cœur à 254,5) et son ombre quasi noire (luminance 8), il n'y a donc pas de biseau
/// coloré à reproduire.
const OUTLINE: Color32 = Color32::BLACK;

/// **Contour complet** — les huit voisins immédiats. À réserver au texte qui flotte nu par-dessus
/// le jeu (dégâts, noms, compteurs de suivi) : le fond y est arbitraire, une ombre d'un seul côté
/// devient illisible dès que ce fond est clair de ce côté-là.
pub const OUTLINE_FULL: &[Vec2] = &[
    Vec2::new(-1.0, -1.0),
    Vec2::new(0.0, -1.0),
    Vec2::new(1.0, -1.0),
    Vec2::new(-1.0, 0.0),
    Vec2::new(1.0, 0.0),
    Vec2::new(-1.0, 1.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(1.0, 1.0),
];

/// **Quart de contour** — les trois voisins bas-droite, soit une ombre portée : le jeu éclaire son
/// titre de modale depuis le haut-gauche (masse sombre mesurée à +2,56 px en x et +1,78 px en y sur
/// la capture de référence). À réserver au texte posé sur un fond **connu**, où l'on peut se
/// permettre de ne cerner qu'un côté ; c'est plus léger et plus proche du jeu qu'un contour complet,
/// qui empâte le mot.
///
/// Une ombre d'un seul pixel unique (un seul décalage diagonal) a été essayée et écartée : à
/// l'échelle réelle elle disparaît. Trois décalages, c'est le minimum qui se voit sans épaissir.
pub const SHADOW_BOTTOM_RIGHT: &[Vec2] = &[
    Vec2::new(1.0, 0.0),
    Vec2::new(0.0, 1.0),
    Vec2::new(1.0, 1.0),
];

/// Peint `text` cerné de noir, en repeignant une copie décalée par entrée de `offsets` avant le
/// texte plein — passer [`OUTLINE_FULL`] ou [`SHADOW_BOTTOM_RIGHT`] selon que le fond est arbitraire
/// ou connu (voir la doc de chacun).
///
/// Les décalages sont en pixels **entiers** : `egui` arrondit de toute façon la position d'un texte
/// au pixel (voir la doc de module), un décalage fractionnaire serait sans effet.
pub fn paint_outlined_text(
    ui: &egui::Ui,
    pos: Pos2,
    align: egui::Align2,
    text: &str,
    font: FontId,
    color: Color32,
    offsets: &[Vec2],
) {
    let painter = ui.painter();
    for offset in offsets {
        painter.text(pos + *offset, align, text, font.clone(), OUTLINE);
    }
    painter.text(pos, align, text, font, color);
}
