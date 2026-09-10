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
///
/// **Le repli est légitime à la première passe, et à elle seule.** Au-delà, il ne signale plus une
/// police en cours de chargement mais un contexte qui n'est jamais passé par [`crate::style::apply`]
/// — et le rendu part alors dans la police par défaut d'`egui` **sans rien dire**. C'est arrivé le
/// 2026-09-10 : un harnais de rendu écrit à la main pour produire un aperçu avait omis cet appel,
/// et le titre de section est sorti dans une autre police, hampes tronquées. Rien dans la capture
/// ne disait pourquoi ; il a fallu comparer avec le snapshot de non-régression pour le comprendre.
///
/// L'assertion ci-dessous ne coûte rien en release (le repli continue d'y protéger l'utilisateur
/// d'un `panic` d'`epaint`) et fait échouer immédiatement, en debug, tout harnais qui oublierait
/// l'appel — tests et outils de rendu compris.
fn famille(ctx: &Context, nom: &str, size: f32) -> FontId {
    let family = FontFamily::Name(nom.into());
    if ctx.fonts(|fonts| fonts.families().contains(&family)) {
        FontId::new(size, family)
    } else {
        debug_assert!(
            ctx.cumulative_pass_nr() == 0,
            "la famille « {nom} » du design system n'est pas liée à la passe {} : ce contexte \
             egui n'est pas passé par `overlay_ui::style::apply`, qui installe les polices. Le \
             rendu retomberait silencieusement sur la proportionnelle par défaut d'egui. \
             Corriger l'appelant (harnais de test, outil de rendu, binaire) plutôt que cette \
             assertion.",
            ctx.cumulative_pass_nr()
        );
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

/// Police d'un libellé APPUYÉ, au corps demandé — la graisse que le jeu réserve à ses boutons de
/// pied de page de modale. Voir [`super::fonts`] pour la mesure qui distingue les deux graisses.
pub fn label_strong_font(ctx: &Context, size: f32) -> FontId {
    famille(ctx, super::fonts::LABEL_STRONG, size)
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

/// Assombrit une couleur **en conservant sa teinte et son opacité** — ce n'est donc pas
/// `Color32::gamma_multiply`, qui entamerait aussi l'alpha et rendrait le cerne translucide.
///
/// Sert à dériver la couleur d'un cerne de celle de son texte, là où le jeu ne cerne pas de noir.
pub fn dimmed(color: Color32, factor: f32) -> Color32 {
    let dim = |c: u8| (c as f32 * factor).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(dim(color.r()), dim(color.g()), dim(color.b()), color.a())
}

/// Peint une galley cernée d'une couleur **choisie**, sur un painter donné (donc écrêtable).
///
/// Deux différences avec [`paint_outlined_text`], et les deux comptent :
///
/// - **le cerne n'est pas noir mais paramétrable.** Le jeu ne cerne pas de la même façon partout :
///   son titre de modale est cerné de noir, mais le libellé d'un onglet est cerné d'une version
///   très assombrie de **sa propre couleur** (voir `tokens::TAB_LABEL_OUTLINE_FACTOR`), ce qui le
///   détache sans le salir ;
/// - **la galley est mise en page une fois** pour les neuf passes, au lieu d'une fois par passe.
///
/// La galley doit avoir été mise en page avec [`Color32::PLACEHOLDER`] : c'est le marqueur d'`egui`
/// pour « la couleur est donnée au moment de peindre », sans quoi les neuf passes sortiraient toutes
/// de la couleur figée à la mise en page.
///
/// **Un cerne n'est pas une graisse** — la mise en garde de la doc de module tient toujours. La
/// différence tient à la couleur : huit copies de la *même* couleur empâtent le mot, huit copies
/// nettement plus sombres le détourent. C'est ce second geste qui est mesuré dans le jeu, pas le
/// premier.
pub fn paint_outlined_galley(
    painter: &egui::Painter,
    pos: Pos2,
    galley: &std::sync::Arc<egui::Galley>,
    color: Color32,
    outline: Color32,
    offsets: &[Vec2],
) {
    for offset in offsets {
        painter.galley(pos + *offset, galley.clone(), outline);
    }
    painter.galley(pos, galley.clone(), color);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fait tourner `passes` passes d'interface sur un contexte neuf et rend le `FontId` que
    /// [`title_font`] a donné à la DERNIÈRE — sans GPU : `Context::run` suffit à faire avancer le
    /// compteur de passes, ce qui est tout ce dont le repli dépend.
    fn font_a_la_passe(passes: u32, applique_le_style: bool) -> FontId {
        let ctx = Context::default();
        let mut dernier = FontId::proportional(1.0);
        for _ in 0..passes {
            let mut sortie = ctx.run_ui(egui::RawInput::default(), |ctx| {
                if applique_le_style {
                    crate::style::apply(ctx);
                }
                dernier = title_font(ctx, 18.0);
            });
            // Personne ne peint ici : `epaint` refuse qu'on jette un `TexturesDelta` non traité,
            // et l'atlas de glyphes en produit un dès que `set_fonts` le reconstruit.
            sortie.textures_delta.clear();
        }
        dernier
    }

    #[test]
    fn premiere_passe_sans_polices_retombe_sans_broncher() {
        // `Context::set_fonts` ne prend effet qu'à la passe suivante : un composant peint
        // légitimement avant que la police n'existe, et le repli est là pour ça.
        assert_eq!(
            font_a_la_passe(1, true).family,
            FontFamily::Proportional,
            "la première passe doit encore retomber sur la proportionnelle"
        );
    }

    #[test]
    fn passe_suivante_prend_la_police_du_design_system() {
        assert_eq!(
            font_a_la_passe(2, true).family,
            FontFamily::Name(super::super::fonts::TITLE.into()),
            "dès la deuxième passe, la serif des titres doit être liée"
        );
    }

    /// Le garde-fou : un contexte qui ne passe jamais par `style::apply` rendrait tous ses textes
    /// dans la police par défaut d'egui **sans rien dire**. C'est la panne qui a produit un aperçu
    /// trompeur le 2026-09-10 (voir la doc de [`famille`]).
    #[test]
    #[should_panic(expected = "n'est pas passé par `overlay_ui::style::apply`")]
    #[cfg(debug_assertions)]
    fn sans_style_apply_le_repli_devient_une_erreur() {
        font_a_la_passe(2, false);
    }
}
