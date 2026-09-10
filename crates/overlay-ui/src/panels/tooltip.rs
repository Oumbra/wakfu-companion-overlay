//! **Infobulle du design system** — le fond, la couleur de texte, l'écart et la marge mesurés sur
//! les captures du jeu, plus le rendu du contenu.
//!
//! Ce module s'appelait `panels::icon_button` : il portait aussi `paint_icon_button`, l'ancêtre
//! maison de `design::icon_button`, retiré le 2026-09-10 quand les quatre boutons du carré de
//! contrôle du Suivi ont migré vers le composant. Ce qui restait n'était plus qu'une infobulle,
//! d'où le nom.
//!
//! Les valeurs ci-dessous ne sont pas posées par un composant mais par le **thème** global
//! (`style::apply`) : `window_fill`/`window_stroke`/`popup_shadow`/`menu_corner_radius`/
//! `menu_margin` ne servent qu'aux infobulles dans cette interface (aucune `egui::Window` ni menu
//! ailleurs), un seul réglage couvre donc toutes les infobulles — les deux enveloppes maison
//! (`combat::show_tooltip_above`, `watchlist::show_tooltip_left`/`_right`) comme les
//! `on_hover_text` ponctuels.

/// Fond d'un tooltip du design system — retour utilisateur 2026-09-06, quatre captures de référence
/// du menu d'icônes de premier plan du jeu à l'appui ("Politique (N)", "Champs de bataille (B)",
/// "Guilde (G)", "Héros et compagnons (K)") : `#15171c` UNIFORME sur les quatre, mesuré par
/// échantillonnage pixel, sans dégradé ni ombre portée visible — contrairement au fond et à l'ombre
/// PAR DÉFAUT d'egui (`Visuals::window_fill`/`popup_shadow` du thème sombre, gris `#1b1b1b` avec un
/// flou de 8px) que l'utilisateur juge justement pas assez nets (« ça rend flou ») une fois comparés
/// à cette référence.
///
/// **Correctif 2026-09-06 (second passage)** : ces quatre premières captures avaient toutes un fond
/// de jeu SOMBRE derrière le tooltip, qui masquait une éventuelle transparence — un fond opaque et
/// un fond translucide à ~80 % sur un fond noir sont visuellement indiscernables. Une cinquième
/// capture ("Répertoire (C)"), cette fois sur un fond de jeu très clair (vert-jaune), révèle la
/// vraie teinte : le tooltip s'y lit gris-olive, pas `#15171c` pur — signe d'un mélange alpha. Alpha
/// résolu par les deux canaux R/G (le fond de cette capture est proche de B=0, peu discriminant) en
/// supposant `#15171c` comme teinte pleine (déjà mesurée ci-dessus) : `blended = a·fg + (1-a)·bg`
/// donne `a≈0,82` sur les deux canaux, cohérent avec l'ombre légèrement transparente que
/// l'utilisateur décrit (« pas noir opaque [...] une opacité à soixante, soixante-dix pour cent à
/// tester ») — mesure retenue plutôt que l'estimation à l'œil, mais dans le même sens (loin de
/// l'opaque posé au premier passage). Alpha stocké directement dans ce `Color32` (RGBA, pas RGB) :
/// appliqué GLOBALEMENT (`egui::Visuals::window_fill`, voir `main.rs`/`style.rs`) plutôt que
/// redéfini à chaque tooltip — `window_fill`/`window_stroke`/`popup_shadow`/`menu_corner_radius`/
/// `menu_margin` ne servent QUE de socle aux tooltips dans cette UI (aucun `egui::Window` ni
/// menu/combobox ailleurs), un seul réglage couvre donc TOUS les tooltips de l'appli — les deux
/// enveloppes maison (`combat::show_tooltip_above`, `watchlist::show_tooltip_left`) ET les
/// `on_hover_text` ponctuels (`render_content`, `watchlist::toast_card`).
pub const TOOLTIP_BG_FILL: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(0x15, 0x17, 0x1c, 209);

/// Couleur du texte d'un tooltip du design system — blanc pur, mesuré par échantillonnage pixel sur
/// les quatre premières captures de référence (pics à `(255,255,255)`) : même démarche déjà
/// appliquée à `watchlist::TEXT_COLOR` (« vérifié par échantillonnage pixel [...] exactement », voir
/// sa doc) — pas le gris `--text-color` par défaut d'egui pour un label non-interactif
/// (`Visuals::widgets.noninteractive.fg_stroke`), dont ce tooltip ne s'inspire pas. Opaque : c'est
/// `TOOLTIP_BG_FILL` qui porte la transparence du tooltip, pas son texte.
pub const TOOLTIP_TEXT_COLOR: egui::Color32 = egui::Color32::WHITE;

/// Écart entre l'élément survolé et son tooltip (`Tooltip::gap`, défaut egui 4px) — mesuré par
/// échantillonnage pixel sur les quatre premières captures de référence (bord de la tuile d'icône au
/// bord du tooltip) : plancher constant de 5px, cohérent sur les quatre (les lectures ponctuelles
/// plus hautes viennent de l'arrondi des coins des deux éléments, pas de l'écart réel entre leurs
/// côtés droits).
pub const TOOLTIP_GAP: f32 = 5.0;

/// Marge interne d'un tooltip du design system (`egui::Style::spacing::menu_margin`, voir
/// `style.rs`) — retour utilisateur 2026-09-06 (second passage) : le remplacement du thème par
/// défaut d'egui (`Margin::same(6)`) gardait sa marge uniforme de 6px, perçue trop étroite comparée
/// au jeu, LATÉRALEMENT et surtout en haut/bas. Mesurée par bbox de texte sur la capture "Politique
/// (N)" (seule des cinq où le tooltip entier — pas juste un bord — tient dans le cadre de la
/// capture) : gauche/droite 8px, haut 12px, bas 8px — descendue à 7/10/8 ici pour coller à
/// l'estimation donnée par l'utilisateur (« au moins sept de chaque côté [...] dix en haut [...]
/// sept ou huit en bas »), les deux mesures s'accordant à 1-2px près.
pub const TOOLTIP_MARGIN: egui::Margin = egui::Margin {
    left: 7,
    right: 7,
    top: 10,
    bottom: 8,
};

/// Peint le contenu d'un tooltip du design system (largeur max + couleur, voir `TOOLTIP_TEXT_COLOR`)
/// — partagé par `combat::show_tooltip_above` et `watchlist::show_tooltip_left`, qui ne diffèrent
/// que par l'alignement du popup lui-même (au-dessus vs à gauche, voir leur doc respective).
pub(crate) fn paint_tooltip_label(ui: &mut egui::Ui, text: &str) {
    ui.set_max_width(ui.spacing().tooltip_width);
    ui.label(egui::RichText::new(text).color(TOOLTIP_TEXT_COLOR));
}
