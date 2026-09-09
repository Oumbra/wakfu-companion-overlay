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
