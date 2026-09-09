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
/// **Calé sur une comparaison pixel avec le jeu**, pas sur une convention typographique. Démarche
/// (rejouable — c'est l'étape « comparer au jeu » du skill `ui-component`) :
///
/// 1. Hauteur d'encre du libellé « Annuler » sur `assets/design-system/large-button-cancel.png`
///    (bouton de 36px capturé dans le jeu, libellé encore incrusté) : **13px**, mesurée comme la
///    boîte des pixels quasi blancs (seuil 200).
/// 2. Même mesure sur le rendu du composant dans `design_gallery.png` à corps 23,6px : **19px**.
///    La police proportionnelle par défaut d'egui rend donc une hauteur de capitale de
///    19/23,6 ≈ 0,805 em — la convention « 0,72 » du skill `ui-blueprint` vaut pour la police du
///    jeu, pas pour celle-ci, d'où l'écart de 45 % constaté au premier passage (libellé
///    visiblement plus gros que sur la capture).
/// 3. D'où `corps = 13 / 0,805` pour un bouton de 36px, ramené à une fraction de la hauteur.
///
/// Le rapport est appliqué à **toutes** les hauteurs plutôt que figé par gabarit : un bouton plus
/// haut porte un libellé proportionnellement plus grand, ce que montrent les captures du jeu.
/// À revalider si la police de l'overlay change (§6.4 du design-system : une police display de
/// substitution est prévue) — la valeur dépend de la police, pas seulement du design.
pub const BUTTON_FONT_SIZE_RATIO: f32 = 13.0 / 0.805 / 36.0;

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
