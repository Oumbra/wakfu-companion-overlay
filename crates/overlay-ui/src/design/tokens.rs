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

/// Bord d'un champ dont la valeur a été refusée — **le même rouge que [`INFO_ALERT`]**, et c'est
/// délibérément un alias plutôt qu'une seconde valeur.
///
/// Un champ en erreur et le message qui l'accompagne sont un seul signal, en deux endroits : les
/// voir dans deux rouges voisins mais distincts se lirait comme deux alertes différentes. Le jour
/// où le jeu fournira une capture d'un champ refusé, c'est cette constante qui prendra la mesure —
/// et elle se détachera alors d'`INFO_ALERT` d'elle-même.
pub const INPUT_BORDER_ERROR: Color32 = INFO_ALERT;

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

/// Côté de l'encre d'une icône d'ornement posée DANS un champ, en fraction de la hauteur du champ.
///
/// **Mesuré sur `empty-input-search.png`** (341 × 32, boîte du champ y 2..29, soit 28 px de haut) :
/// la loupe y occupe x 9..21 / y 10..22, soit 13 × 13 px. 13/28 ≈ 0,464.
///
/// Un ratio et non une valeur absolue, pour la même raison que [`INPUT_FONT_SIZE_RATIO`] : cette
/// capture est à une échelle d'interface un peu plus grande que les 25 px de `InputSize::Standard`.
pub const INPUT_LEADING_ICON_RATIO: f32 = 13.0 / 28.0;

/// Retrait entre le bord extérieur du champ et l'icône d'ornement, en fraction de sa hauteur —
/// 7 px sur les 28 de `empty-input-search.png`.
pub const INPUT_LEADING_ICON_INSET_RATIO: f32 = 7.0 / 28.0;

/// Écart entre l'icône d'ornement et le premier glyphe du texte, en fraction de la hauteur du
/// champ — la loupe finit à x=21, le texte commence à x=30, soit 8 px sur 28.
///
/// **C'est cette gouttière qui fait que le texte ne passe pas sous l'icône** : la version « six
/// espaces dans le texte indicatif » qui a précédé ce jeton ne décalait que le texte indicatif, et
/// laissait une valeur SAISIE démarrer sous la loupe.
pub const INPUT_LEADING_ICON_GAP_RATIO: f32 = 8.0 / 28.0;

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

/// Haut du trait entre deux onglets — **`#837d70`**.
///
/// Le trait n'est pas d'une seule couleur : c'est un **dégradé vertical**, clair en haut, sombre en
/// bas. Ce qui règle une incohérence qui traînait — trois valeurs avaient été relevées pour ce même
/// trait (`#837d70` au relevé, `#6d6657` sur l'asset détouré, `#595140` sur la capture de la
/// modale), et aucune n'était fausse : ce sont **trois points du même dégradé**, échantillonnés à
/// trois hauteurs. Le profil complet est colonne x=261 de `tabs-with-first-tab-active.png`.
pub const TAB_SEPARATOR_TOP: Color32 = Color32::from_rgb(0x83, 0x7D, 0x70);

/// Bas du trait entre deux onglets — **`#595140`**, le même kaki que le bord d'un champ de saisie.
///
/// Voir [`TAB_SEPARATOR_TOP`] pour le dégradé dont c'est l'autre extrémité.
pub const TAB_SEPARATOR_BOTTOM: Color32 = Color32::from_rgb(0x59, 0x51, 0x40);

/// Début de la rampe du dégradé, en fraction de la hauteur du trait.
///
/// Le dégradé n'est pas linéaire de bout en bout : **11 px de clair constant**, puis 19 px de rampe,
/// puis **10 px de sombre constant** (mesurés sur les 40 px du trait). 11/40 = 0,275.
pub const TAB_SEPARATOR_RAMP_START: f32 = 0.275;

/// Fin de la rampe du dégradé, en fraction de la hauteur du trait — 30/40, voir
/// [`TAB_SEPARATOR_RAMP_START`].
pub const TAB_SEPARATOR_RAMP_END: f32 = 0.75;

/// Bord sombre qui cerne toute la barre — **`#1a1d1f`**.
///
/// Moyenne des 774 pixels opaques de la ligne y=0 de `tabs-with-first-tab-active.png`, retrouvée à
/// l'identique sur les trois autres côtés. Il vient de la texture de chaque onglet, sauf **dans les
/// gouttières**, que le composant peint lui-même : sans quoi le fond du panneau y traverse la barre
/// de part en part, et le cerne se retrouve entaillé de deux encoches au droit de chaque
/// séparateur.
pub const TAB_BORDER: Color32 = Color32::from_rgb(0x1A, 0x1D, 0x1F);

/// Épaisseur du bord sombre qui cerne un onglet en haut et en bas.
///
/// **Le trait séparateur s'arrête dessus** : il court de y=2 à y=41 sur les 44 px de la barre, soit
/// 40 px, jamais sur toute la hauteur. Le peindre de bord à bord le fait dépasser de deux pixels
/// au-dessus et en dessous des corps d'onglets — un défaut qui saute aux yeux à côté du jeu.
pub const TAB_BORDER_Y: f32 = 2.0;

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

/// Largeur d'un onglet **à pictogramme**, quand la barre ne s'étire pas — 66 px.
///
/// **Mesurée** sur `assets/design-system/icon-tabs.png` (268 × 44) : les crêtes de séparation y
/// tombent à x=65-66, 133-134 et 201-202, soit un pas de 68 px dont 2 de gouttière. Un onglet à
/// pictogramme est donc plus étroit qu'un onglet texte ([`TAB_MIN_WIDTH`], 77) — il n'a pas de mot
/// à contenir.
pub const TAB_ICON_WIDTH: f32 = 66.0;

/// Côté de l'encre d'un pictogramme d'onglet, **en fraction de la hauteur de la barre**.
///
/// **Dérivé, pas mesuré**, et il faut le dire : `icon-tabs.png` est un gabarit vide — le jeu n'y a
/// laissé aucun pictogramme à mesurer. La valeur reprend le rapport du bouton icône
/// ([`ICON_BUTTON_CONTENT`] sur [`ICON_BUTTON_SIZE`], soit 0,5), seul rapport glyphe/socle que le
/// design system ait mesuré. Sur une barre de 44 px cela donne 22 px d'encre. À remplacer par une
/// mesure dès qu'une capture d'onglets à pictogrammes existera.
pub const TAB_ICON_RATIO: f32 = ICON_BUTTON_CONTENT / ICON_BUTTON_SIZE;

/// Facteur d'assombrissement du **cerne** d'un libellé d'onglet — `0,205`.
///
/// Le libellé du jeu est cerné sur 1px dans les huit directions, et ce cerne n'est **pas noir** : sa
/// couleur est celle du libellé lui-même, très assombrie. Deux couleurs de texte le confirment sur
/// la même capture, ce qui exclut les deux autres explications possibles :
///
/// | Onglet | Libellé | Cerne mesuré | Rapport |
/// | --- | --- | --- | --- |
/// | inactif | `#f4d89e` doré | `#312c21` | 0,201 / 0,204 / 0,209 |
/// | actif | `#ffffff` blanc | `#353534` gris neutre | 0,208 / 0,208 / 0,204 |
///
/// Un cerne **noir translucide** donnerait une couleur proportionnelle au *fond* — or le fond de
/// l'onglet actif est un kaki chaud `#605844` et son cerne est gris neutre. Et un cerne noir
/// **opaque** donnerait du noir. C'est donc bien le libellé qui est repeint assombri.
///
/// Valeurs prises au 10ᵉ centile des pixels du cerne, le cœur du trait : la médiane est tirée vers
/// le haut par l'anticrénelage du glyphe, qui occupe les mêmes pixels.
///
/// **L'état désactivé n'a pas de mesure** — aucune capture d'onglet grisé n'existe. Son libellé est
/// cerné par la même règle, faute de mieux.
pub const TAB_LABEL_OUTLINE_FACTOR: f32 = 0.205;

/// Corps du libellé d'onglet — 17px, la même encre de 13px que tous les libellés du jeu.
pub const TAB_FONT_SIZE: f32 = 17.0;

// ---------------------------------------------------------------------------------------------
// Case à cocher — mesurée sur `releve-section-options.json` (nœuds `cb1` à `cb3`) et sur les deux
// assets, qui font exactement la taille relevée. Détail dans `design::components::checkbox`.
// ---------------------------------------------------------------------------------------------

/// Côté de la case — 20px, `cb1` en `[36, 173, 56, 193]`, confirmé par les deux assets 20 × 20.
pub const CHECKBOX_SIZE: f32 = 20.0;

/// Écart entre la case et son libellé — la case finit à x=56, le libellé commence à x=62.
pub const CHECKBOX_LABEL_GAP: f32 = 6.0;

/// Corps du libellé d'une case — **15, pas 17**. Le jeton `libellé d'option` de
/// `releve-modale-options.json` donne une encre de 10px, plus petite que les 13px d'un libellé de
/// bouton : une ligne d'option n'est pas un contrôle, elle se lit en continu.
pub const CHECKBOX_FONT_SIZE: f32 = 15.0;

/// Libellé d'une case DÉCOCHÉE — blanc.
pub const CHECKBOX_LABEL_OFF: Color32 = Color32::WHITE;

/// Libellé d'une case COCHÉE — l'or du jeu. **Le libellé porte l'état autant que la case** : le jeu
/// double toujours son signal, comme la barre d'onglets le fait avec le sien.
pub const CHECKBOX_LABEL_ON: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9E);

// ---------------------------------------------------------------------------------------------
// Liste déroulante — mesurée sur `select-simple.png` (socle, colonne x=60) et
// `select-simple-opened.png` (liste, colonne x=100), recoupées avec le nœud `select-theme` de
// `releve-options-interface.json`. Détail dans `design::components::select`.
// ---------------------------------------------------------------------------------------------

/// Hauteur du socle — **36px, BORD COMPRIS**.
///
/// Piège du relevé : « le chiffre de 32px qu'on lit en mesurant le remplissage est trompeur — il
/// exclut les 2px de bord haut et bas. » Vérifié au pixel sur l'asset : bord 2 + liseré 2 +
/// dégradé 28 + ombre 2 + bord 2.
pub const SELECT_HEIGHT: f32 = 36.0;

/// Rayon du socle et de la liste — 2, comme tout le reste de l'interface.
pub const SELECT_RADIUS: u8 = 2;

/// Retrait du libellé de socle — le texte commence à x=17 pour un socle dont le bord est à x=7.
pub const SELECT_PADDING_X: f32 = 10.0;

/// Marge entre le chevron et le bord droit du socle.
pub const SELECT_CHEVRON_MARGIN: f32 = 8.0;

/// Taille du chevron — **exactement la taille native d'`icons/icon-chevron-down.png`**. Ce n'est pas
/// une coïncidence : l'icône a été détourée de cette capture, et le chevron y mesure 14 × 8 (x
/// 201..214, y 19..26), centré sur le socle.
pub const SELECT_CHEVRON_SIZE: egui::Vec2 = egui::vec2(14.0, 8.0);

/// Hauteur d'une entrée de la liste dépliée — surbrillance en y 71..98, entrée suivante à y=99.
///
/// `select-multiple.png` donne 26 pour la même chose ; les deux captures ne sont pas à la même
/// échelle d'interface. C'est la mesure du select **simple** qui fait foi, celui dont ce composant
/// reproduit toutes les autres cotes.
pub const SELECT_ROW_HEIGHT: f32 = 28.0;

/// Retrait du texte d'une entrée — « Tous » commence à x=19 pour une liste dont le bord est à x=7.
/// Deux pixels de plus que sur le socle, et c'est bien ce que la capture montre.
pub const SELECT_ROW_PADDING_X: f32 = 12.0;

/// Fond de la liste dépliée — uniforme, aucun dégradé (contrairement au socle).
pub const SELECT_LIST_FILL: Color32 = Color32::from_rgb(0x67, 0x5D, 0x46);

/// Fond d'une entrée mise en avant.
///
/// **Survolée ou valeur courante, indistinctement** : une seule capture montre une entrée sur fond
/// clair, et elle est à la fois la valeur du socle et, très probablement, celle que la souris
/// survolait. Rien ne départage les deux lectures — voir `design::components::select`.
pub const SELECT_ROW_HIGHLIGHT: Color32 = Color32::from_rgb(0xA5, 0x8E, 0x63);

/// Liseré clair d'un pixel en haut de la liste — ce qui la détache du socle, dont l'ombre basse
/// appartient à la même famille chromatique.
pub const SELECT_LIST_TOP_LINE: Color32 = Color32::from_rgb(0x7E, 0x75, 0x62);

/// Bord de la liste — le même noir que celui du socle.
pub const SELECT_LIST_BORDER: Color32 = Color32::from_rgb(0x0E, 0x10, 0x15);

/// Texte du socle comme des entrées — blanc. Le texte d'une entrée mise en avant ne change pas :
/// c'est son fond qui porte le signal.
pub const SELECT_TEXT: Color32 = Color32::WHITE;

/// Corps du texte d'une liste — 17px, la même encre de 13px que tous les libellés de contrôle.
pub const SELECT_FONT_SIZE: f32 = 17.0;

// ---------------------------------------------------------------------------------------------
// Barre de défilement — mesurée sur `releve-modale-options.json` (nœuds `scrollbar-thumb` et
// `panel`) et sur les deux assets, ligne y=150. Détail dans `design::components::scroll_area`.
// ---------------------------------------------------------------------------------------------

/// Largeur de la poignée — 6px, et rien d'autre : **il n'y a pas de rail**. Relevé : « le fond du
/// panneau tient lieu de gouttière ». Confirmé par les deux assets, larges de 14 dont 6 de poignée
/// (x 5..10) et 4 de fond de chaque côté.
pub const SCROLLBAR_WIDTH: f32 = 6.0;

/// Rayon de la poignée.
pub const SCROLLBAR_RADIUS: u8 = 3;

/// Poignée au repos.
///
/// `#515356`, la valeur du relevé de la modale. `scrollbar-inactive.png` donne `#5e5f62` et §5.9 du
/// design-system `#5c5e61` — trois mesures à quelques valeurs près, et c'est celle de la fenêtre
/// qu'on reproduit qui l'emporte, comme pour le séparateur d'onglets.
pub const SCROLLBAR_THUMB: Color32 = Color32::from_rgb(0x51, 0x53, 0x56);

/// Poignée survolée ou tirée — l'or de `scrollbar-active.png` (`#c1ad83`, mesuré à la ligne y=150).
/// §5.9 le donne à `#c0ac83`, à une valeur près.
pub const SCROLLBAR_THUMB_ACTIVE: Color32 = Color32::from_rgb(0xC1, 0xAD, 0x83);

/// Marge entre le contenu et la poignée — 679 → 685 sur la modale du jeu.
pub const SCROLLBAR_CONTENT_MARGIN: f32 = 6.0;

/// Marge entre la poignée et le bord du panneau — 691 → 705.
pub const SCROLLBAR_OUTER_MARGIN: f32 = 14.0;

// ---------------------------------------------------------------------------------------------
// Bouton icône — teintes mesurées sur `menu-button-icon-first-plan.png`, taille native des cinq
// socles. Détail dans `design::components::icon_button`.
// ---------------------------------------------------------------------------------------------

/// Côté d'un bouton icône — **36px, la taille native des cinq socles** (`button-icon.png`,
/// `button-icon-first-plan.png`, leurs `-hover` et `button-icon-disabled.png`, toutes 36 × 36).
/// C'est aussi la taille à laquelle le jeu pose son bouton de réinitialisation.
pub const ICON_BUTTON_SIZE: f32 = 36.0;

/// Teinte d'une icône au repos — `#c5cbcc`, mesurée sur `menu-button-icon-first-plan.png`, pas
/// devinée. Les icônes du design system étant blanc pur avec alpha, une simple teinte suffit à les
/// reproduire — là où `ui_icons` en charge deux copies recolorées au chargement.
pub const ICON_TINT: Color32 = Color32::from_rgb(0xC5, 0xCB, 0xCC);

/// Côté par défaut d'un glyphe posé seul — 16 px.
///
/// **Choisi, pas mesuré**, et il faut le dire : le jeu ne peint aucun glyphe hors socle dans les
/// captures relevées, il n'y a donc pas de taille de référence à reprendre. 16 est la taille d'un
/// glyphe accolé à une ligne de texte de corps 17, celle des libellés du design system — assez
/// grand pour se lire, assez petit pour ne pas dépasser de la ligne. À remplacer par une mesure
/// dès qu'une capture en montrera un.
pub const ICON_SIZE: f32 = 16.0;

/// Teinte d'une icône survolée — `#f4d89f`, valeur donnée par l'utilisateur (2026-09-06).
pub const ICON_TINT_HOVER: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9F);

/// Assombrissement d'un bouton icône désactivé **en contexte premier plan**, où le socle grisé du
/// jeu ne convient pas (voir `IconContext::disabled`) — un multiplicateur d'alpha appliqué au socle
/// de repos, ~43 % d'opacité.
///
/// Valeur reprise telle quelle du prédécesseur maison de ce composant, qui la portait depuis le
/// 2026-09-06 : assez marquée pour lire « désactivé » sur le fond translucide commun d'une barre de
/// premier plan, sans devenir illisible. Elle n'a jamais été reprochée ; la migration vers le
/// design system n'était pas le moment de la rejuger.
pub const DISABLED_DIM: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 110);

/// Teinte d'une icône désactivée en contexte premier plan — [`ICON_TINT`] à l'opacité de
/// [`DISABLED_DIM`], pour que l'icône s'efface exactement autant que son socle.
pub const ICON_TINT_DISABLED: Color32 =
    Color32::from_rgba_unmultiplied_const(0xC5, 0xCB, 0xCC, 110);

/// Plus grande dimension d'encre d'une icône posée sur un socle de [`ICON_BUTTON_SIZE`] — **18px,
/// mesuré sur `menu-button-icon-first-plan.png`** (2026-09-10), pas estimé.
///
/// Le jeu cale les icônes de cette famille sur une grille commune : bbox opaque de 16 à 20px sur
/// les huit icônes de cette barre, **médiane 18**, toutes centrées au pixel près sur l'axe du
/// socle (seuil de luminance 140, résultat invariant de 120 à 180). La capture est bien à l'échelle
/// 1 : `button-icon-first-plan.png` s'y recale à 36 × 36 avec un écart moyen de 2,3/255 sur les
/// pixels de socle, contre 4,5 et plus dès 35 ou 37.
///
/// Sans cet étalon, une icône serait peinte à sa taille de fichier — 13, 14 ou 16px selon le glyphe
/// détouré, donc trois hauteurs d'encre différentes dans une même barre, toutes en dessous du jeu.
/// `ui_icons` normalisait déjà à 16, et à 18 pour le seul rouage après un retour utilisateur
/// « encore trop petite » (2026-09-06) : la mesure dit que ce 18-là valait pour les quatre. Il ne
/// reste rien de cette machinerie depuis la migration du 2026-09-10 — le recadrage est fait en
/// amont par le skill `design-asset`, l'étalon est ici.
///
/// Appliqué par le manifeste (`DsTexture::icon_content_size`), jamais par l'appelant : c'est une
/// propriété de l'asset.
pub const ICON_BUTTON_CONTENT: f32 = 18.0;

// ---------------------------------------------------------------------------------------------
// Fenêtre — `design::window`, `design::panel`, `design::heading`
//
// Toutes ces valeurs viennent du relevé de la fenêtre Options du jeu
// (`docs/design-system/releve-modale-options.json`, six captures d'`interface-options-*.png` au
// chrome identique), affiné par `dsimg.py analyze` puis validé avec l'utilisateur sur une
// simulation HTML avant portage. Elles vivaient dans `panels::options_modal` jusqu'au 2026-09-10
// (lot 1 de `docs/plan-composants-ui.md`) — leur provenance est reprise telle quelle, c'est elle
// qui interdit de les « ajuster à l'œil ».
// ---------------------------------------------------------------------------------------------

/// Rayon d'arrondi de la fenêtre entière — mesuré au pixel sur `modal-header.png`.
///
/// Ne s'applique en pratique qu'aux angles HAUTS : les angles bas sont portés par l'alpha de
/// `modal-body.png`, un `Mesh` egui ne sachant de toute façon pas découper un coin.
pub const WINDOW_RADIUS: u8 = 12;

/// Hauteur de la bannière de titre — la hauteur native de `modal-header.png` (720 × 56).
pub const WINDOW_BANNER_HEIGHT: f32 = 56.0;

/// Corps du titre de bannière, en serif grasse ([`text::title_font`](super::text::title_font)).
pub const WINDOW_TITLE_FONT_SIZE: f32 = 21.0;

/// Couleur du titre de bannière — blanc pur, contre le gris des titres de section.
pub const WINDOW_TITLE_TEXT: Color32 = Color32::WHITE;

/// Marge gauche et droite du contenu, hors bannière et pied de page.
pub const WINDOW_PAD_SIDE: f32 = 20.0;

/// Écart bannière → premier élément de contenu.
pub const WINDOW_PAD_TOP: f32 = 14.0;

/// Marge résiduelle sous le pied de page — la bande sombre mesurée sous « Annuler »/« Valider ».
pub const WINDOW_PAD_BOTTOM: f32 = 12.0;

/// Écart barre d'onglets → panneau de contenu.
pub const WINDOW_TAB_GAP: f32 = 14.0;

/// Écart panneau de contenu → pied de page.
pub const WINDOW_FOOTER_GAP: f32 = 14.0;

/// Gouttière entre les deux boutons du pied de page — **11 px, la valeur du relevé telle quelle**.
///
/// Elle a valu 12 : une mise à l'échelle des 15 px lus à l'œil sur `interface-options-jeu.png`.
/// Le relevé, lui, les cote directement.
pub const WINDOW_FOOTER_GUTTER: f32 = 11.0;

/// Hauteur des boutons du pied de page — **la hauteur native de leur texture** (338 × 36).
///
/// Jamais déduite de la largeur : une première version écrivait `largeur * (36 / 338)` et
/// affichait des boutons de 27 px là où le jeu en met 36, le corps du libellé suivant la hauteur.
pub const WINDOW_FOOTER_BUTTON_HEIGHT: f32 = 36.0;

/// Translucidité du corps de la fenêtre — 235/255, la valeur qu'avait l'aplat qui le peignait
/// avant que la texture du jeu ne le remplace. La couleur, elle, vient de la texture.
pub const WINDOW_BODY_TINT: Color32 = Color32::from_rgba_premultiplied(235, 235, 235, 235);

/// Translucidité du panneau de contenu — **230/255**.
///
/// Sa couleur, son liseré et l'arrondi de ses angles ne se règlent plus ici : ils sont dans
/// `DsTexture::ModalSection`, découpée des captures du jeu (§9 quater du design-system). Un blanc
/// à alpha réduit atténue sans changer la teinte — le mécanisme de teinte de
/// `design::nine_slice::paint`, déjà celui de l'état désactivé des boutons.
///
/// L'alpha 230 est une **déviation assumée** : la fenêtre entière est légèrement translucide, et
/// un panneau opaque à l'intérieur d'elle se verrait comme une tache. En pratique l'écart ne se
/// voit pas — les deux alphas se multiplient, et `modale_options_sur_damier` mesure 0,7 de
/// contraste résiduel sous le panneau contre 9,7 dans les marges.
///
/// **Attention au constructeur** : `from_rgba_premultiplied` attend des composantes déjà
/// multipliées par l'alpha. L'ancien `PANEL_FILL` lui passait les valeurs relevées telles quelles,
/// et peignait donc le panneau 2,3 niveaux trop clair — 24,5 au rendu contre 22,1 attendus. Une
/// teinte blanche n'a pas ce piège : ses trois composantes valent son alpha.
pub const PANEL_TINT: Color32 = Color32::from_rgba_premultiplied(230, 230, 230, 230);

/// Axe des TITRES de section, depuis le bord du panneau.
pub const PANEL_PAD_TITLE_X: f32 = 12.0;

/// Axe des CONTRÔLES, depuis le bord du panneau.
///
/// L'écart avec l'axe des titres — 7 px — est le seul signal de niveau du jeu : une section n'a ni
/// fond, ni bordure, ni filet, seul son titre est en retrait.
pub const PANEL_PAD_CONTROL_X: f32 = 19.0;

/// Rembourrage haut du panneau — **19, la valeur du relevé telle quelle**.
///
/// Elle a longtemps valu 13, corrigée à la main des 6 px que la police réserve au-dessus de
/// l'encre d'un titre. Cette correction n'a plus lieu d'être depuis que le titre réserve sa
/// hauteur d'ENCRE et non celle de sa galley — voir [`HEADING_INK_HEIGHT`].
pub const PANEL_PAD_TOP: f32 = 19.0;

/// Corps d'un titre de section — **le même que le titre de fenêtre**, même serif.
///
/// La hiérarchie entre les deux niveaux passe par la COULEUR, pas par le corps. Une première
/// valeur (18) venait d'un rapport calculé de travers : 17 px d'encre mesurés sur un mot sans
/// jambage comparés à 21 px sur un mot qui en a un. C'est la ligne de base qui est stable, pas la
/// boîte — comparer l'encre de deux mots différents pour en déduire un rapport donne un faux.
pub const HEADING_FONT_SIZE: f32 = WINDOW_TITLE_FONT_SIZE;

/// Couleur d'un titre de section — un gris franc, **pas le blanc du titre de fenêtre**.
pub const HEADING_TEXT: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);

/// Hauteur d'ENCRE réservée par un titre de section, et non la hauteur de sa galley.
///
/// Une galley est plus haute que son encre des deux côtés : la police y réserve la place des
/// accents de capitale au-dessus et des jambages en dessous, que la plupart des titres n'utilisent
/// ni l'un ni l'autre. Réserver la galley entière ajoutait ~14 px invisibles sous le titre, sur
/// sept relevés.
pub const HEADING_INK_HEIGHT: f32 = 16.0;

/// Décalage du haut de la galley au haut de l'encre — voir [`HEADING_INK_HEIGHT`].
pub const HEADING_INK_TOP: f32 = 6.0;

/// Écart entre un titre de section et la première ligne qui le suit.
pub const HEADING_TO_ROW: f32 = 7.0;

// ---------------------------------------------------------------------------------------------
// Pas numérique — `design::stepper`
//
// Mesuré le 2026-09-10 sur les DEUX captures du jeu, `input-number.png` (104 × 28) et
// `large-input-number.png` (192 × 34), segmentées par la couleur de bordure du champ. Les deux
// donnent les mêmes rapports, ce qui est la seule raison de les écrire en rapports plutôt qu'en
// pixels : le pas du jeu existe à deux échelles, et il se met à l'échelle sans se déformer.
// ---------------------------------------------------------------------------------------------

/// Côté natif du socle d'un bouton de pas — **32 px**, mesuré deux fois plutôt qu'une.
///
/// Sur l'asset isolé (`button-moins.png`, 34 × 34) : un pixel transparent tout autour, socle utile
/// 32 × 32. Sur la capture composée (`large-input-number.png`, 34 de haut) : le socle court de y=1
/// à y=32, et les deux boutons y sont symétriques (x 1..32 et x 159..190). Le fichier fait donc 34,
/// le bouton 32 — et c'est le bouton qui est l'étalon.
///
/// **La confusion des deux a produit un défaut visible** (corrigé le 2026-09-10) : la galerie
/// rendait ses pas à `size(34.0)`, la hauteur du FICHIER, ce qui donnait des boutons deux pixels
/// trop hauts et creusait l'écart avec leur champ.
///
/// **Écart assumé avec `design-tokens.json`**, qui cote `stepper_button_square` à 30 : cette
/// valeur-là a été lue à l'œil sur une capture d'écran en 2026-09-05, celle-ci est mesurée sur
/// l'asset ET sur la capture composée. La mesure l'emporte.
pub const STEPPER_SIZE: f32 = 32.0;

/// Gouttière entre un bouton de pas et le champ, **en rapport de la taille du socle**.
///
/// **10 px pour un socle de 32**, mesuré sur `large-input-number.png` : le découpage complet y est
/// 1 + 32 + 10 + 106 + 10 + 32 + 1 = 192, le bouton allant de x=1 à x=32 et son symétrique de
/// x=159 à x=190, la bordure du champ de x=43 à x=148.
///
/// **La petite capture donne 7 pour un socle de 24** (0,292 contre 0,3125) — un pixel d'écart une
/// fois remise à l'échelle, ce qui passerait. Mais elle diverge franchement sur un autre point :
/// son glyphe fait **12 px comme celui de la grande**, alors que son socle est d'un quart plus
/// petit. Les deux captures ne sont donc PAS le même composant à deux échelles — le jeu règle son
/// interface de 67 % à 233 % (`interface-options-interface.png`), et elles ont vraisemblablement
/// été prises à deux réglages différents, ce qui ne met pas tout à l'échelle de la même façon.
///
/// Tout est donc calé sur la **grande capture**, la seule dont le socle (32) coïncide avec l'asset
/// isolé `button-moins.png`. La petite reste une vérification de cohérence, pas une source.
///
/// Une première version donnait 0,267 : elle divisait par la hauteur du FICHIER (34) au lieu de
/// celle du socle (32), et par une hauteur de bouton fausse sur la petite capture.
pub const STEPPER_GUTTER_RATIO: f32 = 10.0 / 32.0;

/// Plus grande dimension d'encre d'un glyphe de pas, **en rapport de la taille du socle**.
///
/// Mesuré : l'encre du « + » incrusté fait 12 × 12 et celle du « − » 12 × 2, dans un socle de 32
/// (`large-input-number.png`). La petite capture donne la même encre de 12 dans un socle de 24 —
/// voir [`STEPPER_GUTTER_RATIO`], qui explique pourquoi elle n'est pas retenue comme source.
///
/// **Ce n'est pas la grille des boutons icône** ([`ICON_BUTTON_CONTENT`], 18 pour un socle de 36,
/// soit 0,5). Le jeu a deux grilles distinctes pour ses deux familles de socles, et un même glyphe
/// — le « + » — s'y peint à 18 dans une barre de premier plan et à 12 dans un pas. La taille
/// d'encre est donc une propriété du **couple (asset, contexte)**, pas de l'asset seul : c'est
/// pourquoi `IconContext` la porte pour ce contexte-là, plutôt que le manifeste.
pub const STEPPER_ICON_RATIO: f32 = 12.0 / 32.0;

/// Teinte des glyphes d'un pas — **l'or du jeu, au repos comme au survol**.
///
/// Mesuré au pixel sur `large-input-number.png` : le « − » y est uniformément `#f4d89f`. C'est un
/// troisième comportement, distinct des deux autres contextes de bouton icône, qui peignent leur
/// glyphe en gris clair ([`ICON_TINT`]) et ne passent à l'or qu'au survol ([`ICON_TINT_HOVER`]).
///
/// Même valeur qu'`ICON_TINT_HOVER` à ce jour, et ce n'est pas un oubli : les deux disent des
/// choses différentes — l'un est un état, l'autre une couleur de repos. Le jour où une capture
/// survolée d'un pas existera, seul celui-ci bougera.
pub const STEPPER_ICON_TINT: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9F);

/// Hauteur du champ central d'un pas, **en rapport de la hauteur des boutons**.
///
/// **1,0 — le champ fait exactement la hauteur de ses boutons.** C'est une décision utilisateur
/// (2026-09-10) et un **écart assumé avec le jeu**, dit ici plutôt que masqué : mesuré sur
/// `large-input-number.png`, le jeu met un socle de 32 px (y=1..32) et un champ de 26 px (bordure
/// kaki y=4..29), soit un rapport de 0,81. Un champ plus court que ses boutons a été jugé
/// « pas très joli » ; l'uniformité l'emporte ici sur la fidélité.
///
/// Le jeton reste parce que le rapport reste une question ouverte : si une capture d'une autre
/// interface montre un jour un pas mieux proportionné, c'est cette valeur-là qui bougera, en un
/// seul endroit.
pub const STEPPER_FIELD_HEIGHT_RATIO: f32 = 1.0;

// ---------------------------------------------------------------------------------------------
// Bloc repliable — `design::collapsible`
//
// Relevé de référence : `docs/design-system/collapse-block.json`, établi par le mainteneur le
// 2026-09-11 sur les captures détourées `collapse-block-closed.png` (732 × 60) et
// `collapse-block-opened.png` (732 × 210). Les deux concordent au pixel sur l'en-tête : même bande
// à 7 px du bord haut, seule la hauteur totale change.
//
// **Ce relevé remplace celui du même jour établi ici même**, qui portait sur des captures non
// détourées : il donnait 18 px d'encre de titre au lieu de 14, une icône de 35 px au lieu de 33, et
// des marges de 16/20 au lieu de 17/23. Les écarts venaient de l'ombre portée et du fond de jeu
// comptés comme de l'encre. C'est le relevé outillé qui fait foi.
// ---------------------------------------------------------------------------------------------

/// Hauteur de l'en-tête, **et donc hauteur du bloc entier quand il est fermé** — 60 px.
///
/// C'est la mesure la moins ambiguë du relevé : à l'état fermé, le cadre ne contient QUE son
/// en-tête, et la capture détourée fait exactement 732 × 60. L'icône, le titre et le chevron y sont
/// centrés sur y=30, soit le milieu exact de cette hauteur.
pub const COLLAPSE_HEADER_HEIGHT: f32 = 60.0;

/// Marge intérieure gauche et droite — 17 px du bord du cadre au contenu.
///
/// Le relevé donne 17 à gauche (l'icône et les intitulés de section y commencent) et 15 à droite.
/// **Les deux px d'écart ne sont pas repris** : ils portent sur le bord droit du recadrage, là où
/// rien du contenu ne vient buter, et une asymétrie de deux pixels qu'aucun élément ne matérialise
/// coûterait un second jeton pour rien. Le chevron, lui, a bien sa propre marge — voir
/// [`COLLAPSE_CHEVRON_PAD_X`].
pub const COLLAPSE_PAD_X: f32 = 17.0;

/// Marge intérieure basse — 23 px sous le dernier élément de contenu.
///
/// Plus généreuse que la marge latérale, comme partout dans le jeu.
pub const COLLAPSE_PAD_BOTTOM: f32 = 23.0;

/// Marge droite **du chevron** — 13 px, contre 17 pour le reste du contenu.
///
/// Mesurée sur le relevé : le glyphe s'arrête à x=719 d'un cadre large de 732. Ce n'est pas du
/// bruit de détourage mais un écart de quatre pixels sur un élément dont les deux bords sont nets,
/// et le rendre symétrique au contenu décalerait visiblement le chevron vers l'intérieur.
pub const COLLAPSE_CHEVRON_PAD_X: f32 = 13.0;

/// Corps du titre d'en-tête, en serif grasse.
///
/// Le relevé mesure 14 px d'encre et en déduit ≈19 px de corps. Le rapport d'encre d'egui pour
/// cette police (0,805, mesuré lors du chantier du bouton) donne 14/0,805 = 17,4 — mais c'est le
/// rapport d'une **capitale**, or « Description » et « Le Village » portent des jambages. 18 est le
/// corps qui redonne 14 px d'encre sur le gabarit réel, vérifié au rendu.
pub const COLLAPSE_TITLE_FONT_SIZE: f32 = 18.0;

/// Couleur du titre d'en-tête — `#fefefe`, le quasi-blanc du relevé.
pub const COLLAPSE_TITLE_TEXT: Color32 = Color32::from_rgb(0xFE, 0xFE, 0xFE);

/// Teinte du chevron — `#a69064`, un doré olive.
///
/// **Ce n'est pas [`ICON_TINT`]** (`#c5cbcc`, un gris froid), et le relevé insiste sur ce point :
/// l'icône de contenu du bloc tire vers le crème doré (`#dad6c7`), le chevron vers l'olive. Deux
/// accents distincts dans un même en-tête, pas un seul jeton partagé.
pub const COLLAPSE_CHEVRON_TINT: Color32 = Color32::from_rgb(0xA6, 0x90, 0x64);

/// Côté de l'icône d'en-tête, **en rapport de la hauteur de l'en-tête**.
///
/// Mesuré : 33 × 32 px dans un en-tête de 60, soit un carré de 32 à la tolérance du détourage.
/// L'icône est optionnelle et vient de l'appelant — c'est une icône de contenu (une quête, un
/// lieu), pas un glyphe du design system.
pub const COLLAPSE_ICON_RATIO: f32 = 32.0 / 60.0;

/// Gouttière entre l'icône et le titre — 10 px, entre la fin de l'icône (x=50) et le début du titre
/// (x=60).
///
/// Le titre n'est donc **pas** aligné sur la marge intérieure de 17 px quand il n'y a pas d'icône :
/// il l'est seulement quand l'icône l'a poussé. Voir `design::collapsible`.
pub const COLLAPSE_ICON_GAP: f32 = 10.0;

// ---------------------------------------------------------------------------------------------
// Rouage de chargement — `design::loader`
// ---------------------------------------------------------------------------------------------

/// Côté du rouage à sa taille native (1×), en pixels : **mesuré** sur l'enregistrement du client
/// du 2026-09-11 — 118 px d'encre (rayon extérieur 58,5) plus 3 px de marge transparente de chaque
/// côté, marge conservée pour que le lissage des bords ne soit pas coupé. C'est aussi la cellule de
/// `loader-sheet.png` et la borne haute de [`LOADER_MIN_SIZE`] : au-delà, le bitmap serait flouté.
pub const LOADER_NATIVE_SIZE: f32 = 124.0;

/// Plus petit côté admis — **décision utilisateur** (2026-09-11) : « le minimum ce serait
/// quarante-huit sur quarante-huit ». Cohérent avec la géométrie : à 48 px le pas d'une dent vaut
/// ≈ 9 px de circonférence, à 32 px ≈ 6 px et les seize dents se confondent.
pub const LOADER_MIN_SIZE: f32 = 48.0;

/// Paliers nommés de `LoaderSize`. `Small` est le plancher ; `Medium` et `Large` sont **choisis,
/// pas mesurés** (le jeu n'affiche ce rouage qu'à une taille) : ils découpent l'intervalle 48–124
/// en marches d'environ 25 px, assez espacées pour se distinguer à l'œil.
pub const LOADER_SIZE_SMALL: f32 = LOADER_MIN_SIZE;
pub const LOADER_SIZE_MEDIUM: f32 = 72.0;
pub const LOADER_SIZE_LARGE: f32 = 96.0;

/// Cadence de l'animation — **mesurée** : 4 images nouvelles sur 5 enregistrées à 30 i/s.
pub const LOADER_FPS: f64 = 24.0;

/// Longueur de la boucle — **mesurée** : le rouage avance d'une dent toutes les 8 images, mais la
/// tête ne revient à sa position que toutes les 16 (l'image 386 de l'enregistrement est identique
/// à la 409, pas à la 399).
pub const LOADER_FRAMES: usize = 16;

// ---------------------------------------------------------------------------------------------
// Filet de séparation — `design::separator`
//
// **Mesuré au pixel** le 2026-09-11 sur `assets/design-system/collapse-block-opened.png` et
// `collapse-block-opened-hover.png` (les captures *source*, non générifiées : les génériques ont
// justement été nettoyés de leur contenu, filet compris). Le filet y apparaît deux fois, aux lignes
// y=82..85 et y=152..155, identiques au pixel près dans les deux occurrences et dans les deux
// textures.
//
// Deux écarts avec `docs/design-system/collapse-block.json`, tranchés en faveur de la mesure :
//
// - la note de `sep-1` donne la ligne claire à `#323436` — la texture donne `#323335`. L'écart est
//   d'un niveau sur le vert et deux sur le bleu ; `#323436` est la teinte du *fond survolé*, qui a
//   vraisemblablement été relevée à sa place.
// - la palette du relevé déclare `border-bevel-dark: #28292b` — or `#28292b` est exactement le fond
//   au repos, et une ligne sombre de cette teinte serait invisible. La texture donne `#222325`,
//   valeur que la note de `sep-1` portait déjà : c'est la palette qui est fautive, pas la note.
// ---------------------------------------------------------------------------------------------

/// Hauteur totale du filet — 4 px, soit deux lignes de [`SEPARATOR_BEVEL_HEIGHT`].
pub const SEPARATOR_HEIGHT: f32 = 4.0;

/// Hauteur de **chacune** des deux lignes du bevel — 2 px.
///
/// Deux lignes plates empilées, pas un dégradé : les quatre lignes de pixels mesurées ne portent
/// que deux teintes, sans valeur intermédiaire.
pub const SEPARATOR_BEVEL_HEIGHT: f32 = 2.0;

/// Ligne claire, au repos — `#323335`, dix niveaux au-dessus du fond.
pub const SEPARATOR_LIGHT: Color32 = Color32::from_rgb(0x32, 0x33, 0x35);

/// Ligne sombre, au repos — `#222325`, six niveaux sous le fond.
pub const SEPARATOR_DARK: Color32 = Color32::from_rgb(0x22, 0x23, 0x25);

/// Ligne claire quand la surface porteuse est survolée — `#3c3e3f`.
///
/// Le jeu **recalcule le bevel** avec le fond au lieu de le laisser en place : voir
/// [`SEPARATOR_SURFACE_HOVER`] et la doc de `design::separator`.
pub const SEPARATOR_LIGHT_HOVER: Color32 = Color32::from_rgb(0x3C, 0x3E, 0x3F);

/// Ligne sombre quand la surface porteuse est survolée — `#2b2c2e`.
pub const SEPARATOR_DARK_HOVER: Color32 = Color32::from_rgb(0x2B, 0x2C, 0x2E);

/// Fond sur lequel le filet de repos a été mesuré — `#28292b`, la surface du bloc repliable.
///
/// Aucun composant ne peint cette couleur (le repliable pose une texture), mais c'est la référence
/// qui donne son sens aux deux teintes du bevel : elles ne valent que *relativement* à elle. Un
/// test de `design::separator` s'en sert pour vérifier que la claire est bien au-dessus et la
/// sombre en dessous.
pub const SEPARATOR_SURFACE_REST: Color32 = Color32::from_rgb(0x28, 0x29, 0x2B);

/// Fond sur lequel le filet survolé a été mesuré — `#323436`.
pub const SEPARATOR_SURFACE_HOVER: Color32 = Color32::from_rgb(0x32, 0x34, 0x36);

/// Écart réservé **sous** le filet, avant le corps de la section — 12 px.
///
/// Mesuré entre le bas du filet (y=86) et le haut de l'encre du corps (y=98). L'écart *au-dessus*
/// du filet vaut 10 px (bas de l'encre de l'intitulé, y=72) et appartient à l'élément précédent —
/// même répartition que [`HEADING_TO_ROW`], pour que deux composants voisins ne doublent pas
/// l'espace à leur jonction.
pub const SEPARATOR_GAP_BELOW: f32 = 12.0;

// ---------------------------------------------------------------------------------------------
// Curseur de réglage — `design::slider`
//
// **Mesuré au pixel** le 2026-09-11 sur les deux curseurs de volume de
// `assets/design-system/interfaces/interface-options-son.png` (« Musique », rainure y 335..342 ;
// « Sons - Ambiance », y 430..437). Les deux concordent sur toutes les cotes.
//
// **Les deux curseurs de la capture sont au minimum.** Rien n'y montre donc ce que devient la
// portion parcourue de la rainure — voir `design::slider`, qui ne la remplit pas.
// ---------------------------------------------------------------------------------------------

/// Hauteur de la rainure — 8 px, dont 2 de liseré en haut et 2 en bas.
pub const SLIDER_TRACK_HEIGHT: f32 = 8.0;

/// Épaisseur du liseré de la rainure, en haut comme en bas — 2 px.
pub const SLIDER_TRACK_EDGE: f32 = 2.0;

/// Assombrissement du **liseré** de la rainure, en noir translucide.
///
/// La rainure n'a pas de couleur propre : c'est un **creux**, et c'est une mesure, pas un choix de
/// rendu. Sur deux fonds différents (`#131b16` sous « Musique », `#151c17` sous « Ambiance ») le
/// rapport au fond reste le même — 0,735 sur les trois canaux — alors que les valeurs absolues
/// diffèrent de deux niveaux. Un noir à 27 % reproduit ce rapport ; deux constantes opaques
/// seraient fausses dès que le panneau qui porte le curseur change de teinte.
pub const SLIDER_TRACK_EDGE_SHADE: Color32 = Color32::from_black_alpha(68);

/// Assombrissement de l'**intérieur** de la rainure — noir à 15 %, rapport 0,853 mesuré.
pub const SLIDER_TRACK_SHADE: Color32 = Color32::from_black_alpha(38);

/// Côté de la poignée — 18 px, la taille native de `slider-handle.png`.
///
/// C'est **elle** qui fixe la hauteur du composant : la poignée déborde la rainure de 5 px de
/// chaque côté, et un curseur haut de 8 px lui couperait le disque.
pub const SLIDER_HANDLE_SIZE: f32 = 18.0;

/// Largeur relevée de la rainure — 200 px (x 75..274).
///
/// **Ce n'est pas un gabarit**, seulement l'ordre de grandeur du jeu : la largeur d'un curseur
/// vient de sa mise en page, comme celle d'un champ de saisie. Conservée pour qu'une galerie ou un
/// panneau puisse retrouver les proportions de la capture.
pub const SLIDER_TRACK_WIDTH_REF: f32 = 200.0;

/// Gouttière entre un libellé d'extrémité et la rainure — 12 px à gauche (« Min » finit à x=65, la
/// rainure commence à 75), 10 px à droite (elle finit à 274, « Max » commence à 284).
///
/// Les libellés **n'appartiennent pas au composant** : « Min »/« Max » conviennent à un volume,
/// pas à une échelle d'interface qui dirait « 50 % »/« 200 % ». C'est l'appelant qui les pose, et
/// cette cote lui dit à quelle distance.
pub const SLIDER_LABEL_GAP: f32 = 12.0;

/// Couleur d'une graduation — `#e2ddd7`, un blanc cassé très légèrement chaud.
///
/// **Mesurée** sur le curseur d'échelle d'interface
/// (`interfaces/interface-options-interface.png`, y=182), identique sur les 26 graduations.
pub const SLIDER_TICK: Color32 = Color32::from_rgb(0xE2, 0xDD, 0xD7);

/// Largeur d'une graduation — 1 px, mesurée sur 26 graduations dont 26 font exactement un pixel.
pub const SLIDER_TICK_WIDTH: f32 = 1.0;

/// De combien une graduation **dépasse** la rainure, en haut comme en bas — 2 px.
///
/// Mesuré : la graduation occupe y 181..183 au-dessus d'une rainure qui court de 183 à 190, et
/// y 190..192 en dessous. Elle **recouvre donc le premier pixel du liseré** et déborde de deux, ce
/// qui lui donne 3 px visibles de chaque côté. Elle ne traverse jamais l'intérieur de la rainure :
/// au milieu, les pixels d'une colonne graduée sont identiques à ceux d'une colonne nue.
pub const SLIDER_TICK_OVERHANG: f32 = 2.0;

/// De combien une graduation **mord sur le liseré** de la rainure — 1 px, soit la moitié du liseré.
///
/// Mesuré : le liseré haut occupe y 183..184 et la graduation s'arrête à 183. Mordre les deux px
/// donnerait un segment de 4 px là où le jeu en montre 3, et ne laisserait que 4 px de rainure nue
/// entre les deux segments au lieu de 6 — assez pour que la marque se lise comme un trait presque
/// continu plutôt que comme deux repères. C'est le défaut qu'a rattrapé la comparaison au jeu, la
/// seconde fois sur ce composant.
pub const SLIDER_TICK_BITE: f32 = 1.0;

// ---------------------------------------------------------------------------------------------
// Infobulle — `design::tooltip`
//
// Ces quatre valeurs étaient dans `panels::tooltip`, module né du démembrement de l'ancien
// `panels::icon_button`. Elles sont remontées ici le 2026-09-11 avec le composant : un jeton mesuré
// n'a pas à vivre dans un panneau, et `style::apply` les lit désormais d'ici.
// ---------------------------------------------------------------------------------------------

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
pub const TOOLTIP_BG_FILL: Color32 = Color32::from_rgba_unmultiplied_const(0x15, 0x17, 0x1C, 209);

/// Couleur du texte d'un tooltip du design system — blanc pur, mesuré par échantillonnage pixel sur
/// les quatre premières captures de référence (pics à `(255,255,255)`) : même démarche déjà
/// appliquée à `panels::watchlist::TEXT_COLOR` (« vérifié par échantillonnage pixel [...] exactement », voir
/// sa doc) — pas le gris `--text-color` par défaut d'egui pour un label non-interactif
/// (`Visuals::widgets.noninteractive.fg_stroke`), dont ce tooltip ne s'inspire pas. Opaque : c'est
/// `TOOLTIP_BG_FILL` qui porte la transparence du tooltip, pas son texte.
pub const TOOLTIP_TEXT: Color32 = Color32::WHITE;

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
