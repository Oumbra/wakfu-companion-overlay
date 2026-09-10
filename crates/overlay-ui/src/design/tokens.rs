//! Jetons du design system — couleurs et métriques **mesurées**, partagées par les composants.
//!
//! Miroir Rust de `docs/design-tokens.json` (extrait par analyse pixel de captures réelles du
//! client, voir `docs/design-system.md` §7 pour les sources). Seuls les jetons effectivement
//! consommés par un composant sont repris ici : un jeton sans usage est du bruit, il reste dans le
//! JSON jusqu'à ce qu'un composant en ait besoin.
//!
//! Règle de rédaction, identique au reste du crate : **toute constante dit d'où vient sa valeur**
//! (mesure pixel, capture de référence, retour utilisateur). Une valeur devinée qui n'annonce pas
//! qu'elle est devinée est le pire cas — impossible à distinguer plus tard d'une mesure.

use egui::Color32;

/// Texte d'un bouton primaire (or) — `accent_warm.button_text_on_gold` de `design-tokens.json`,
/// mesuré sur les captures HDV : brun très sombre, jamais du noir pur.
pub const BUTTON_TEXT_ON_GOLD: Color32 = Color32::from_rgb(0x3A, 0x35, 0x23);

/// Texte d'un bouton secondaire (gris-brun) — §5.2 du design-system : « le texte blanc domine
/// visuellement ».
pub const BUTTON_TEXT_ON_KHAKI: Color32 = Color32::WHITE;

/// Texte d'un bouton danger (rouge) — `accent_danger.text` de `design-tokens.json`.
pub const BUTTON_TEXT_ON_DANGER: Color32 = Color32::WHITE;

/// Texte d'un bouton désactivé — `neutrals.text_disabled`. Va avec la texture
/// `DsTexture::ButtonDisabled` (désaturation complète, §5.1 du design-system).
pub const TEXT_DISABLED: Color32 = Color32::from_rgb(0x8A, 0x8A, 0x8A);

/// Corps de police d'un libellé de bouton, en fraction de la hauteur du bouton.
///
/// **Mesuré par balayage, dans egui**, sur les textures réelles 338×36 — la taille exacte des
/// captures du jeu qui portent encore ces libellés gravés (`large-button-cancel.png`,
/// `large-button-validate.png`). Le critère est la **boîte d'encre** du mot : hauteur et largeur
/// des pixels à plus de 50 % d'opacité, pas la masse d'encre, qu'une somme d'alphas surévalue.
///
/// | corps | « Annuler » | « Valider » |
/// | --- | --- | --- |
/// | référence (jeu) | 13 × 62 | 13 × 59 |
/// | 16px | 12 × 56 | 12 × 50 |
/// | **17px** | **13 × 63** | **13 × 57** |
/// | 19px | 15 × 68 | 15 × 61 |
///
/// 17px est le seul corps qui retrouve la hauteur d'encre de 13px sur les deux mots. Confirmé une
/// seconde fois le 2026-09-09 après-midi, avec la police Medium (`design::fonts`) : les largeurs
/// tombent alors à 1 et 2px de la référence, contre 2 et 6px avec l'ancienne Light.
///
/// **Ce ratio suppose que le bouton est à sa hauteur native.** Il l'a longtemps été à tort dans la
/// modale Options, qui déduisait sa hauteur du rapport d'aspect de la texture : à 27px au lieu de
/// 36, le libellé tombait à 12,6px de corps et 10px d'encre. Le symptôme se lisait comme « la
/// police est trop petite », il fallait lire « le bouton est trop court » — agrandir le corps
/// l'aurait masqué (19px donne 15px d'encre, deux de trop). Voir
/// `panels::options_modal::FOOTER_BUTTON_HEIGHT`.
///
/// À revalider si la police des libellés change — la valeur dépend de la police, pas du design.
pub const BUTTON_FONT_SIZE_RATIO: f32 = 17.0 / 36.0;

/// Marge horizontale entre le bord du bouton et son libellé, en fraction de la hauteur du bouton.
/// **Estimation, pas une mesure** : les deux boutons du pied de page de la modale Options sont
/// pleine largeur, leur libellé y est centré — la capture ne dit donc rien du padding minimal.
/// Valeur calée sur l'ordre de grandeur donné par §4 du design-system (« padding horizontal
/// généreux, ~30-40px de chaque côté » sur des boutons de ~64px de haut, soit ≈ 0,55×hauteur).
pub const BUTTON_PADDING_X_RATIO: f32 = 0.55;

// ---------------------------------------------------------------------------------------------
// Champ de saisie — mesuré sur la barre de recherche de l'onglet Commandes
// (`interface-options-commandes.png`, champ en x 30..500, y 138..163) et recoupé avec les assets
// isolés (`dsimg.py analyze`). Détail des sources dans `design::components::input`.
// ---------------------------------------------------------------------------------------------

/// Fond d'un champ — nettement plus sombre que le panneau qui le porte.
pub const INPUT_FILL: Color32 = Color32::from_rgb(0x0E, 0x11, 0x15);

/// Bord d'un champ — le kaki chaud systématique de tous les champs du jeu
/// (`accent_warm.input_border` de `design-tokens.json`, retrouvé à l'identique sur les quatre
/// assets de champ).
pub const INPUT_BORDER: Color32 = Color32::from_rgb(0x59, 0x51, 0x40);

/// Épaisseur du bord, en pixels — mesurée, pas supposée : le bord occupe deux lignes pleines
/// (y 138-139 et 161-162) et deux colonnes pleines (x 30-31 et 498-499).
pub const INPUT_BORDER_WIDTH: f32 = 2.0;

/// Rayon d'angle — `dsimg.py analyze` sur `large-input-text-width-placeholder.png` et
/// `input-search.png`, ajustement parfait (IoU 1,0) dans les deux cas. C'est le seul composant du
/// jeu à ne pas être à 2.
pub const INPUT_RADIUS: u8 = 4;

/// Couleur d'une valeur SAISIE — **or, pas blanc**. Le constat le plus contre-intuitif du relevé,
/// et il tient sur trois assets indépendants : `input-search.png`, `input-number.png` et
/// `large-input-number.png` donnent tous le même pic `#f4d89e`. Un champ à valeur blanche ne
/// ressemble pas au jeu.
pub const INPUT_TEXT: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9E);

/// Couleur du texte indicatif — un kaki éteint, la même famille chromatique que le bord en plus
/// sourd (pic `#83775b` sur « Rechercher », `#8a7d60` sur « Min »).
pub const INPUT_PLACEHOLDER: Color32 = Color32::from_rgb(0x83, 0x77, 0x5B);

/// Corps de police du texte d'un champ, en fraction de sa hauteur. Le texte d'un champ a la même
/// hauteur d'encre (13px) qu'un libellé de bouton, donc le même corps de 17px — mais sur un champ
/// de 25px et non un bouton de 36, d'où un ratio différent.
pub const INPUT_FONT_SIZE_RATIO: f32 = 17.0 / 25.0;

/// Retrait horizontal du texte depuis le bord du champ, en fraction de sa hauteur. Mesuré : le
/// premier glyphe de « Rechercher » commence à x=36 pour un champ dont le bord est à x=30, soit
/// 6px sur 25 de hauteur.
pub const INPUT_PADDING_X_RATIO: f32 = 6.0 / 25.0;

/// Largeur minimale d'un bouton, en fraction de sa hauteur — garde-fou de proportion pour un
/// libellé très court (« OK »), pour qu'il ne devienne pas un carré. Réglage d'ergonomie, pas une
/// mesure.
pub const BUTTON_MIN_ASPECT: f32 = 2.5;

// ---------------------------------------------------------------------------------------------
// Texte d'information — mesuré sur le bloc en bas de l'onglet Interface
// (`releve-options-interface.json`, nœud `info`), le seul de toute la fenêtre Options du jeu.
// Détail des sources dans `design::components::info_text`.
// ---------------------------------------------------------------------------------------------

/// Couleur du texte d'information — **blanc pur**, pas un gris. Note du relevé : « l'impression de
/// gris vient du fond et de l'absence de graisse, pas de la couleur ».
pub const INFO_TEXT: Color32 = Color32::WHITE;

/// Couleur de la pastille — `info-dot` de `releve-options-interface.json`, mesurée sur le disque
/// doré du bloc d'information.
pub const INFO_DOT: Color32 = Color32::from_rgb(0xA6, 0x90, 0x64);

/// Couleur du ton **alerte** — pastille et texte à la fois.
///
/// **La composition est une extension assumée**, le jeu n'ayant aucune variante d'alerte relevée
/// (voir `design::components::info_text`). La couleur, elle, ne l'est pas : c'est
/// `accent_danger.top` de `design-tokens.json`, le haut du bouton « Annuler » — le seul rouge que
/// le design system ait mesuré. Un rouge choisi à l'œil (l'ancien `#e06055` peint à la main dans
/// `panels::options_modal`) n'appartenait à aucune capture.
pub const INFO_ALERT: Color32 = Color32::from_rgb(0xC9, 0x52, 0x4A);

/// Côté de la pastille, en pixels — boîte `[38, 438, 50, 450]` du relevé.
pub const INFO_DOT_SIZE: f32 = 12.0;

/// Gouttière entre la pastille et le texte — la pastille finit à x=50, le texte commence à x=57.
pub const INFO_DOT_GAP: f32 = 7.0;

/// Interligne, en pixels : haut d'encre à haut d'encre (y=436 puis y=459). **Une mesure, pas la
/// hauteur naturelle de la police** — egui empilerait les lignes plus serré.
pub const INFO_LINE_HEIGHT: f32 = 23.0;

/// Corps du texte d'information — le même que celui d'un libellé de bouton, pour la même encre de
/// 13 px. Une valeur absolue et non un ratio : ce bloc n'a pas de hauteur propre dont il pourrait
/// dériver, sa hauteur est celle de son contenu.
pub const INFO_FONT_SIZE: f32 = 17.0;

// ---------------------------------------------------------------------------------------------
// Barre d'onglets — mesurée sur `releve-modale-options.json` (nœuds `tabbar` et `tab-*`) et
// recoupée au pixel sur `interface-options-video.png`, ligne y=110. Détail des sources dans
// `design::components::tabs`.
// ---------------------------------------------------------------------------------------------

/// Hauteur native d'un onglet — `[72, 116]` dans une bande `[56, 124]`.
///
/// **La hauteur d'un composant est celle de sa texture**, jamais déduite de sa largeur : la modale
/// écrasait cette texture de 44 px à 31, ce qui aplatissait le décor et le libellé avec.
pub const TAB_HEIGHT: f32 = 44.0;

/// Libellé de l'onglet ACTIF — blanc pur. C'est la seule chose qui le distingue d'un onglet
/// survolé, dont le fond est identique (voir `design::components::tabs`).
pub const TAB_LABEL_ACTIVE: Color32 = Color32::WHITE;

/// Libellé d'un onglet inactif **et** survolé — l'or du jeu.
pub const TAB_LABEL_IDLE: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9E);

/// Trait clair entre deux onglets.
///
/// **`#595140`, la valeur de la capture de la modale** (x 100-101 sur `interface-options-video.png`,
/// ligne 110) — le même kaki que le bord d'un champ de saisie. Deux autres valeurs traînent pour ce
/// même trait : `#837d70` dans le relevé et `#6d6657` dans l'asset détouré. La capture de la fenêtre
/// qu'on reproduit l'emporte sur les deux.
pub const TAB_SEPARATOR: Color32 = Color32::from_rgb(0x59, 0x51, 0x40);

/// Largeur du trait séparateur, et donc gouttière entre deux onglets : chacun porte déjà ses deux
/// bords sombres de 2px, ce qui donne les 6px que le relevé mesure entre deux remplissages.
pub const TAB_SEPARATOR_WIDTH: f32 = 2.0;

/// Marge horizontale entre le bord d'un onglet et son libellé.
///
/// **La seule valeur stable des six onglets relevés** : 15px sur « Interface » (encre 73, onglet
/// 103) et 16 sur « Commandes » (encre 100, onglet 133). Les quatre libellés courts ont un padding
/// bien plus large (jusqu'à 35px sur « Chat »), sans règle retrouvable — voir
/// `design::components::tabs`.
pub const TAB_PADDING_X: f32 = 16.0;

/// Largeur plancher d'un onglet — le plus petit onglet relevé (« Jeu », 77px). Garde-fou de
/// proportion pour un libellé court, comme `BUTTON_MIN_ASPECT` pour un bouton. Réglage, pas mesure.
pub const TAB_MIN_WIDTH: f32 = 77.0;

/// Corps du libellé d'onglet — 17px, la même encre de 13px que tous les libellés du jeu.
pub const TAB_FONT_SIZE: f32 = 17.0;
