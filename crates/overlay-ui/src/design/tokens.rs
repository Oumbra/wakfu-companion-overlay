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
/// **Mesuré par balayage, dans egui.** Une planche temporaire (`overlay-testkit`, test
/// `etalonnage_du_libelle`, supprimé une fois la valeur figée) a rendu « Annuler » et « Valider »
/// sur leur texture réelle en 338×36 — la taille exacte des captures du jeu qui portent encore ces
/// libellés gravés — pour trente couples corps × graisse, chaque cellule étant ensuite comparée au
/// pixel avec `large-button-cancel.png` / `large-button-validate.png`.
///
/// Le critère retenu est la **boîte d'encre** du mot (hauteur et largeur des pixels à plus de 50 %
/// d'opacité), pas sa masse : le halo de graisse ajoute beaucoup de pixels faiblement opaques, que
/// l'œil ne compte pas et qu'une somme d'alphas surévalue.
///
/// | corps | « Annuler » | « Valider » |
/// | --- | --- | --- |
/// | référence (jeu) | 13 × 63 | 13 × 59 |
/// | 16px | 12 × 56 | 12 × 50 |
/// | **17px** | **13 × 60** | **13 × 53** |
/// | 18px | 14 × 63 | 14 × 56 |
///
/// 17px est le seul corps qui retrouve exactement la hauteur d'encre de 13px sur les deux mots.
/// Le mot reste 3 à 6px plus étroit que dans le jeu : la police du jeu est un peu plus large à
/// hauteur égale, écart qu'aucun corps ne résout sans casser la hauteur — il disparaîtra avec la
/// police de substitution prévue au §6.4 du design-system.
///
/// La mesure précédente (`13 / 0,805 / 36`, soit 16,1px) déduisait le corps d'un rapport
/// hauteur-de-capitale supposé ; ce balayage le mesure directement, d'où l'écart d'un pixel.
/// À revalider si la police de l'overlay change — la valeur dépend de la police, pas du design.
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

/// Graisse synthétique d'un libellé de composant : l'opacité du halo d'un pixel peint autour du
/// texte — voir `design::text::weighted` pour le procédé et pourquoi il n'y a pas d'alternative.
///
/// **Choisi à l'œil sur la planche de balayage** décrite dans `BUTTON_FONT_SIZE_RATIO`, et
/// assumé comme tel : la mesure ne pouvait pas trancher, parce que le jeu lui-même n'est pas
/// cohérent. Ses deux libellés de pied de page n'ont pas la même graisse — le « Valider » sombre
/// sur or est nettement plus gras que le « Annuler » blanc sur rouge (masse d'encre mesurée :
/// 411 contre 310, à hauteur d'encre identique), l'asymétrie classique du texte sombre sur fond
/// clair, probablement accentuée par une ombre incrustée. Aucune valeur unique ne peut coller aux
/// deux à la fois.
///
/// 0,2 place le rendu entre les deux : « Annuler » retrouve exactement la densité du jeu,
/// « Valider » reste un peu plus léger que sa référence. 0,4 rattrapait « Valider » mais empâtait
/// « Annuler ». Retour utilisateur à l'origine du réglage : rendre la police « un petit peu plus
/// grasse » en attendant un vrai fichier de police.
pub const TEXT_WEIGHT: f32 = 0.2;
