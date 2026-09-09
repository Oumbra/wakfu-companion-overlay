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

/// Largeur minimale d'un bouton, en fraction de sa hauteur — garde-fou de proportion pour un
/// libellé très court (« OK »), pour qu'il ne devienne pas un carré. Réglage d'ergonomie, pas une
/// mesure.
pub const BUTTON_MIN_ASPECT: f32 = 2.5;
