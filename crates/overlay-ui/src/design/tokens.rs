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
use overlay_engine::ChatChannel;

/// **L'or du jeu**, celui de son texte — `#f4d89e`.
///
/// La teinte la plus répandue de l'interface, et elle est **mesurée trois fois indépendamment** :
/// une valeur saisie dans un champ (`input-search.png`, `input-number.png`,
/// `large-input-number.png` donnent tous le même pic), le libellé d'un onglet inactif, et celui
/// d'une case cochée. C'est un fait du design system, pas une valeur propre à un composant — d'où
/// ce jeton, que les trois précités citent au lieu de le recopier.
///
/// **À ne pas confondre avec [`BUTTON_TEXT_ON_GOLD`]** : celui-ci est de l'or POUR du texte, celui
/// -là du texte SUR de l'or (un brun très sombre).
pub const TEXT_GOLD: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9E);

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
pub const INPUT_TEXT: Color32 = TEXT_GOLD;

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

/// Hauteur native de la **barre de recherche** du jeu — `InputSize::Search`.
///
/// Mesurée sur `empty-input-search.png` et `input-search.png` (341 × 32, boîte du champ y 2..29
/// inclus, soit 28 px), deux assets d'accord au pixel. Ce n'est PAS le champ de 25 px de l'onglet
/// Commandes : la barre de recherche est plus haute, et son texte garde la même encre — un « R »
/// de 12 px (y 11..22) dans 28, là où le champ standard loge un « A » de 12 px dans 25. Retour
/// utilisateur du 2026-09-12 (« l'input est un tout petit peu trop petit en hauteur, ou alors
/// c'est la police qui est trop grande ») : c'est la hauteur qui manquait, pas la police qui
/// débordait.
pub const INPUT_SEARCH_HEIGHT: f32 = 28.0;

/// Teinte de la loupe d'une barre de recherche — **pas** le kaki du texte indicatif.
///
/// Pic `#a69064` (166, 144, 100) sur les deux assets de barre de recherche, identique champ vide
/// ou rempli : une teinte plus chaude et plus claire que `INPUT_PLACEHOLDER` (`#83775b`), que la
/// loupe portait jusqu'au 2026-09-12 (« n'est pas colorée comme sur la maquette »).
pub const INPUT_ICON: Color32 = Color32::from_rgb(0xA6, 0x90, 0x64);

/// Teinte de la croix d'effacement d'un champ, au repos — pic `#675d46` (103, 93, 70) sur
/// `input-search.png`, plus sourde que le texte indicatif : elle ne rivalise pas avec la valeur.
pub const INPUT_CLEAR_ICON: Color32 = Color32::from_rgb(0x67, 0x5D, 0x46);

/// Teinte de la croix d'effacement survolée — **inventée** (aucune capture d'une croix survolée) :
/// la teinte de la loupe, pour rester dans la même famille sans introduire une couleur de plus.
pub const INPUT_CLEAR_ICON_HOVERED: Color32 = INPUT_ICON;

/// Côté de la boîte de la croix d'effacement, en fraction de la hauteur du champ — encre
/// 11 × 12 px (x 320..330, y 10..21) sur les 28 de `input-search.png`.
pub const INPUT_CLEAR_ICON_RATIO: f32 = 12.0 / 28.0;

/// Retrait entre le bord extérieur DROIT du champ et la croix, en fraction de sa hauteur — l'encre
/// finit à x=330 pour un bord extérieur à x=338 inclus : 9 px sur 28.
pub const INPUT_CLEAR_INSET_RATIO: f32 = 9.0 / 28.0;

/// Écart réservé entre la fin du texte et la croix — la même gouttière que devant la loupe, faute
/// de valeur longue sur les captures (« aa » ne l'atteint pas).
pub const INPUT_CLEAR_GAP_RATIO: f32 = INPUT_LEADING_ICON_GAP_RATIO;

/// Côté de l'encre d'une icône d'ornement posée DANS un champ, en fraction de la hauteur du champ.
///
/// **Mesuré sur `empty-input-search.png`** (341 × 32, boîte du champ y 2..29, soit 28 px de haut) :
/// la loupe y occupe x 9..21 / y 10..22, soit 13 × 13 px. 13/28 ≈ 0,464.
///
/// Un ratio et non une valeur absolue : à `InputSize::Search` (28 px, la hauteur de cette même
/// capture) les trois ratios d'ornement redonnent exactement 7, 13 et 8 px.
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
pub const TAB_LABEL_IDLE: Color32 = TEXT_GOLD;

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
// Switch à deux cases — mesuré le 2026-09-16 sur les deux captures du sélecteur de genre du jeu
// (`switch-first-slot-active.png` / `switch-second-slot-active.png`, 88 × 44 une fois détourées et
// générifiées) et sur le relevé `ui-blueprint` qui en a été tiré. Détail dans
// `design::components::switch`.

/// Hauteur de la **capture** du switch — **44px**, celle des deux textures du jeu. Comme pour un
/// onglet, la hauteur est celle de la texture, jamais déduite de la largeur.
///
/// Ce n'est plus la hauteur à laquelle le composant se peint : depuis le 2026-09-16 (demande
/// utilisateur, « passer les boutons du composant switch en 36 par 36 de manière générique »),
/// toute case est servie à [`SWITCH_SLOT_SIZE`], et ce jeton ne sert qu'à en déduire le rapport
/// de réduction (`36 / 44`) appliqué aux marges du 9-slice, au séparateur, au liseré et aux
/// glyphes — voir `design::components::switch`. `Switch::scale` multiplie ce rapport ;
/// `Switch::height` impose la hauteur seule.
pub const SWITCH_HEIGHT: f32 = 44.0;

/// Largeur d'une case dans la **capture** — **43px**, la moitié des 88px du switch du jeu une
/// fois le séparateur déduit : liseré 2 + case active 40 + séparateur 2 + case inactive 42 +
/// liseré 2. Même statut que [`SWITCH_HEIGHT`] : une mesure du jeu, pas la largeur servie — celle-ci
/// est [`SWITCH_SLOT_SIZE`], et `Switch::width` l'étire.
///
/// 43 et non 40 ou 42 : la case active du jeu fait 40 de remplissage, l'inactive 42, et chacune
/// porte un liseré de 2 à son extrémité. Donner la même largeur aux deux laisse le 9-slice
/// absorber un pixel de chaque côté. La capture « première case active » mesurait 87px (case
/// inactive à 41) contre 88 pour l'autre : écart de rendu du jeu, égalisé à 88 au traitement de
/// l'asset.
pub const SWITCH_SLOT_WIDTH: f32 = 43.0;

/// Côté d'une case de switch, **dans les deux variantes** — **36px**, [`ICON_BUTTON_SIZE`].
///
/// **Décision utilisateur du 2026-09-16** : « passer les boutons du composant switch en 36 par 36
/// de manière générique ». Jusque-là, seule la variante premier plan était à 36 (son socle EST un
/// bouton icône) ; la variante cadre se peignait aux 43 × 44 de sa capture, et les deux
/// contrôles ne tombaient pas sur la même grille dès qu'ils cohabitaient. Une case est désormais
/// un carré de 36, quelle que soit sa matière : le cadre du jeu y est ramené par un 9-slice à
/// l'échelle `36 / 44` (`DesignSystem::paint_scaled`) — coins, liseré et biseaux réduits ensemble,
/// comme le jeu réduit son interface —, et non par un corps comprimé entre des marges figées.
///
/// Un switch à `n` cases fait donc `36 × n` plus ses gouttières : `+ 2 × (n − 1)` en cadre (74
/// pour deux, 112 pour trois), `− 2 × (n − 1)` en premier plan, dont les socles se chevauchent
/// ([`SWITCH_FIRST_PLAN_OVERLAP`] : 70 et 104).
pub const SWITCH_SLOT_SIZE: f32 = ICON_BUTTON_SIZE;

/// Largeur du séparateur entre les deux cases — **2px**, colonnes x 42–43 de la capture.
pub const SWITCH_SEPARATOR_WIDTH: f32 = 2.0;

/// Épaisseur du liseré haut et bas — **2px**. Le séparateur ne court qu'entre ces deux liserés
/// (y 2..42), comme celui d'une barre d'onglets.
pub const SWITCH_BORDER_Y: f32 = 2.0;

/// Liseré du cadre — **`#221f24`**, `uispec.py palette` sur les colonnes x 0–1 (79 % à
/// `#221f24`, le reste à un niveau près). Peint dans la gouttière, où aucune texture de case ne
/// le porte : sans lui, le cadre serait entaillé de deux encoches au droit du séparateur.
pub const SWITCH_BORDER: Color32 = Color32::from_rgb(0x22, 0x1F, 0x24);

/// Séparateur entre les deux cases — **`#312d2d`**, 100 % de la palette des colonnes x 42–43
/// entre les deux liserés. Un aplat, pas un dégradé : contrairement au trait entre deux onglets,
/// la colonne est constante à deux niveaux près de y=4 à y=39 (les extrémités reçoivent le
/// biseau de la case active, un pixel plus clair).
pub const SWITCH_SEPARATOR: Color32 = Color32::from_rgb(0x31, 0x2D, 0x2D);

/// Glyphe de la case ACTIVE — **`#f4d89f`**, couleur dominante des pixels pleins du ♂ et du ♀
/// sur leurs captures respectives. C'est, à un niveau près, l'or de tout le design system
/// ([`TEXT_GOLD`], [`ICON_TINT_HOVER`]).
pub const SWITCH_ICON_ACTIVE: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9F);

/// Glyphe de la case INACTIVE — **`#a9a5a2`**, gris clair légèrement chaud, couleur dominante des
/// pixels pleins du ♀ inactif (capture 1) et du ♂ inactif (capture 2). Ce n'est pas
/// [`ICON_TINT`] (`#c5cbcc`, gris froid du premier plan) : le jeu grise le glyphe non
/// sélectionné plus franchement.
pub const SWITCH_ICON_INACTIVE: Color32 = Color32::from_rgb(0xA9, 0xA5, 0xA2);

/// Atténuation d'un glyphe **en couleurs** (`DsIcon::native_color`) là où un glyphe blanc serait
/// teinté [`SWITCH_ICON_INACTIVE`] : multiplicateur 150/255 (≈ 59 %) sur ses trois canaux.
///
/// **Estimation, pas une mesure** : le jeu n'a pas de switch à glyphes en couleurs dans les
/// interfaces relevées. 150 est le rapport de luminance entre [`SWITCH_ICON_INACTIVE`] et
/// [`SWITCH_ICON_ACTIVE`] (`#a9a5a2` sur `#f4d89f`, ≈ 0,68) arrondi vers le bas pour que
/// l'atténuation se lise aussi sur un glyphe déjà sombre (la dague des dégâts). Validé sur le
/// rendu du panneau Combat (2026-09-16), à remplacer par une mesure si le jeu en fournit une.
pub const ICON_NATIVE_DIM: Color32 = Color32::from_rgb(150, 150, 150);

/// Côté du carré englobant d'un glyphe de switch — **16px**.
///
/// **Mesuré, avec une nuance.** Les deux glyphes du jeu sont peints à leur taille native dans
/// leur case de 40px : ♂ 14 × 14 et ♀ 10 × 16, tous deux centrés (relevé `ui-blueprint`,
/// centres à 22,22 et 66,22 pour des cases centrées en 22 et 65). 16 est la plus grande de ces
/// deux encres. Le composant ne **grossit** donc pas un glyphe qui tient dans ce carré — le ♂
/// reste à 14 — et ne réduit que ceux qui le dépassent. Un étalon commun (comme
/// [`ICON_BUTTON_CONTENT`]) donnerait un ♂ à 16, deux pixels plus large que dans le jeu.
pub const SWITCH_ICON_SIZE: f32 = 16.0;

/// Côté natif d'une case de la variante [`SwitchVariant::FirstPlan`](crate::design::SwitchVariant)
/// — **[`ICON_BUTTON_SIZE`], 36px**, parce qu'une case y EST un socle de bouton icône
/// (`button-icon-first-plan.png`, 36 × 36). Lui donner une référence propre ferait diverger deux
/// contrôles qui partagent leur texture, et l'écart se verrait dès qu'ils cohabitent dans une même
/// barre de premier plan. Depuis le 2026-09-16, c'est aussi le côté d'une case de la variante
/// cadre ([`SWITCH_SLOT_SIZE`]) : les deux variantes partagent leur grille.
pub const SWITCH_FIRST_PLAN_SIZE: f32 = SWITCH_SLOT_SIZE;

/// Liseré de la case CHOISIE en variante premier plan — **`#126068`, la teinte la plus vive de
/// `button-icon-first-plan-hover.png`** (relevé de sa palette, 2026-09-16 : 10 px de la texture).
///
/// Il devient le signal de la case choisie depuis que le survol allume l'icône ET le socle : sans
/// lui, une case survolée et la case choisie sont identiques. Ni or, ni gris — **une couleur de la
/// texture elle-même**, choisie sur rendu comparatif des six teintes de son corps (artefact du
/// 2026-09-16, décision utilisateur).
///
/// Le socle sélectionné est un **dégradé**, clair en haut (`#428087`) et sombre en bas
/// (`#18383e`) : une teinte prise dans sa moitié basse (`#164047`, essayée) se dissout dans le
/// haut de la case. `#126068` tient sur toute la hauteur.
///
/// **Il remplace la bordure du socle, il ne s'y ajoute pas** : même épaisseur
/// ([`SWITCH_FIRST_PLAN_RIM_WIDTH`]), même arrondi ([`SWITCH_FIRST_PLAN_RIM_ROUNDING`]), posé SUR
/// elle (retrait nul). Peint après toutes les cases, il recouvre aussi les bordures des voisines
/// qui chevauchent la case choisie.
pub const SWITCH_FIRST_PLAN_RIM: Color32 = Color32::from_rgb(0x12, 0x60, 0x68);

/// Épaisseur du liseré de la case choisie — **2 px, celle de la bordure de la texture**, mesurée :
/// sur `button-icon-first-plan.png`, les colonnes x 0–1 et les lignes y 0–1 sont à `#141519`, le
/// corps commence en 2. Un liseré d'un pixel laisserait la moitié de la bordure noire visible.
pub const SWITCH_FIRST_PLAN_RIM_WIDTH: f32 = 2.0;

/// Opacité du liseré de la case choisie — **85 % (216/255)**, choisie sur rendu comparatif
/// (100 / 85 / 70 %) : elle retire au trait ce qu'il a de tranchant sans l'effacer. Décision
/// utilisateur du 2026-09-16.
pub const SWITCH_FIRST_PLAN_RIM_ALPHA: u8 = 216;

/// Opacité du halo d'un pixel peint juste à l'intérieur du liseré, de la même couleur —
/// **35 % (90/255)**, choisie sur rendu comparatif (0 / 25 / 35 / 50 / 70 %).
///
/// Il adoucit la marche entre le liseré et le corps du socle. Au-delà de 50 %, il cesse d'être un
/// halo : il se lit comme un liseré de trois pixels.
pub const SWITCH_FIRST_PLAN_HALO_ALPHA: u8 = 90;

/// Épaisseur du halo : **un pixel**, pas plus — demande utilisateur explicite.
pub const SWITCH_FIRST_PLAN_HALO_WIDTH: f32 = 1.0;

/// Arrondi du halo : celui du liseré moins son épaisseur, donc quasi nul ; 1 garde un coin adouci
/// plutôt qu'un angle droit sous l'arrondi du liseré.
pub const SWITCH_FIRST_PLAN_HALO_ROUNDING: u8 = 1;

/// Arrondi du liseré de la case choisie — **2, celui des coins de la texture**, mesuré sur le
/// coin haut-gauche de `button-icon-first-plan.png` : l'alpha y vaut 24/255 en (0,0), 32 et 42 sur
/// ses deux voisins, puis 236 en (0,2). Un liseré à angle droit dépasserait du socle dans les
/// quatre coins.
pub const SWITCH_FIRST_PLAN_RIM_ROUNDING: u8 = 2;

/// Chevauchement de deux socles voisins en variante premier plan (2026-09-16).
///
/// Chaque socle porte un liseré de 2 px : deux socles côte à côte en alignent quatre, plus la
/// gouttière du fond, soit 6 px de sombre entre deux glyphes (retour utilisateur : « ça fait comme
/// s'il y avait une bordure de 2 pixels », là où le jeu n'en montre qu'une, fine). Les faire se
/// chevaucher superpose les deux liserés en un seul.
pub const SWITCH_FIRST_PLAN_OVERLAP: f32 = 2.0;

/// Trait peint sur la jointure de deux socles qui se chevauchent. Un gris sombre
/// mais franchement plus clair que le fond du socle (`#141519`) : superposés, les deux liserés
/// noirs ne se distinguent plus de lui.
pub const SWITCH_FIRST_PLAN_SEAM: Color32 = Color32::from_rgb(0x4A, 0x4E, 0x54);

/// Glyphe de la case CHOISIE en variante premier plan — **blanc pur**, donc la couleur vraie du
/// fichier.
///
/// C'est la distinction qui sépare la case choisie de la case survolée, qui partagent leur socle
/// (`button-icon-first-plan-hover.png`) : même règle que la barre d'onglets, où l'actif et le
/// survolé partagent leur fond et ne se distinguent que par la couleur de leur libellé (voir
/// [`TAB_LABEL_ACTIVE`]). Le couple gris clair → or du contexte premier plan ([`ICON_TINT`] /
/// [`ICON_TINT_HOVER`]) tient les deux autres états, il ne pouvait pas arbitrer celui-ci.
pub const SWITCH_FIRST_PLAN_ICON_ACTIVE: Color32 = TAB_LABEL_ACTIVE;

/// Corps du libellé d'une case **sans pictogramme** — 17px, le corps de tous les libellés du jeu
/// ([`TAB_FONT_SIZE`]).
///
/// **Inventé, faute de référence** : le jeu n'a pas de switch à libellé texte dans les captures
/// relevées, seulement le sélecteur de genre à pictogrammes. La valeur reprend le corps d'onglet,
/// la case la plus proche par sa hauteur (44px, la même). À mesurer dès qu'une capture existera.
pub const SWITCH_FONT_SIZE: f32 = TAB_FONT_SIZE;

// ---------------------------------------------------------------------------------------------
// Case à cocher — mesurée sur `releve-section-options.json` (nœuds `cb1` à `cb3`) et sur les deux
// assets, qui font exactement la taille relevée. Détail dans `design::components::checkbox`.
// ---------------------------------------------------------------------------------------------

/// Côté de la case — 20px, `cb1` en `[36, 173, 56, 193]`, confirmé par les deux assets 20 × 20.
pub const CHECKBOX_SIZE: f32 = 20.0;

/// Écart entre la case et son libellé — la case finit à x=56, le libellé commence à x=62.
pub const CHECKBOX_LABEL_GAP: f32 = 6.0;

/// Écart vertical entre deux lignes d'option consécutives — **11px**.
///
/// Le relevé donne le PAS, pas l'écart : « Rythme : 31 px entre deux lignes d'option
/// consécutives », vérifié au pixel sur `interface-options-jeu.png` (haut de la première case
/// y=175, haut de la seconde y=206) et sur les onze lignes de `interface-options-chat.png`, dont
/// les pas relevés sont 30/31/31/31/31/33/…/31/31. Une ligne étant haute de
/// [`CHECKBOX_SIZE`], il reste **31 − 20 = 11px** de blanc entre deux cases.
///
/// **C'est un jeton de MISE EN PAGE, pas du composant** : `design::checkbox` alloue la hauteur
/// d'une ligne et rien de plus (voir sa doc de module, « affaire de la mise en page, pas du
/// composant »), et `design::panel` met `item_spacing.y` à zéro pour que chaque écart soit posé
/// explicitement. C'est ce qui a manqué jusqu'au 2026-09-14 : deux cases empilées se suivaient à
/// 20px au lieu de 31 — retour utilisateur, capture du jeu à l'appui, « un gros écart entre les
/// deux ». L'écart n'a jamais été un choix, seulement une valeur que personne n'avait posée.
///
/// Se pose entre deux lignes, **jamais après la dernière** : le blanc qui suit un bloc est le
/// `SECTION_GAP` de `panels::options_modal` (17px, le relevé encore), et les deux s'ajouteraient.
pub const CHECKBOX_ROW_GAP: f32 = 11.0;

/// Corps du libellé d'une case — **15, pas 17**. Le jeton `libellé d'option` de
/// `releve-modale-options.json` donne une encre de 10px, plus petite que les 13px d'un libellé de
/// bouton : une ligne d'option n'est pas un contrôle, elle se lit en continu.
pub const CHECKBOX_FONT_SIZE: f32 = 15.0;

/// Libellé d'une case DÉCOCHÉE — blanc.
pub const CHECKBOX_LABEL_OFF: Color32 = Color32::WHITE;

/// Libellé d'une case COCHÉE — l'or du jeu. **Le libellé porte l'état autant que la case** : le jeu
/// double toujours son signal, comme la barre d'onglets le fait avec le sien.
pub const CHECKBOX_LABEL_ON: Color32 = TEXT_GOLD;

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

/// Rayon du socle, et des DEUX COINS HAUTS de la liste — 2, comme tout le reste de l'interface.
pub const SELECT_RADIUS: u8 = 2;

/// Rayon des **deux coins bas de la liste dépliée** — 4, presque le double du reste.
///
/// Ce n'est pas une fantaisie : `select-simple-opened.png` le montre au pixel. Les quatre
/// dernières lignes de la liste rentrent de 3, 2, 1 puis 0 px — fond à x=7 en y=151, x=8 en 152,
/// x=9 en 153, bord noir à x=10 en 154 — et symétriquement à droite. Le haut, lui, ne rentre que
/// de 2 px : il est collé au socle, il n'a pas à s'en détacher. C'est le bas, qui flotte au-dessus
/// du contenu, que le jeu adoucit.
pub const SELECT_LIST_BOTTOM_RADIUS: u8 = 4;

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

// -------------------------------------------------------------------------------------------
// Autocomplétion — `design::autocomplete`
// -------------------------------------------------------------------------------------------
//
// Le composant **reprend les jetons de décor de `select` déplié** (fond de liste, bord,
// surbrillance, filet de tête) : c'est la même liste du jeu, il n'y a pas de second relevé à
// faire. Les jetons ci-dessous couvrent ce que `select` n'a pas — la bande de filtres, et la
// rangée entière, qui est celle du web et non la cadence de 28 px du jeu (voir
// `AUTOCOMPLETE_ROW_HEIGHT`).
//
// Provenance : `shared/wakfu-autocomplete/wakfu-autocomplete.component.css` du dépôt
// `Oumbra/wakfu-companion`, relevé le 2026-09-11. Ce sont des valeurs de la version WEB, portées
// telles quelles faute de capture du jeu montrant une autocomplétion — le jeu n'en a pas. Ce n'est
// donc PAS une mesure sur asset : c'est un portage assumé, et c'est dit ici plutôt que laissé à
// deviner.

/// Hauteur de la bande de filtres : bouton 26 + 2 × 6 de marge (`padding: 6px`).
pub const AUTOCOMPLETE_FILTER_BAR_HEIGHT: f32 = 38.0;
/// Côté d'un bouton de filtre — `.wakfu-autocomplete-category-btn`, 26 × 26.
pub const AUTOCOMPLETE_FILTER_BUTTON: f32 = 26.0;
/// Marge intérieure d'un bouton de filtre : l'icône occupe 20 des 26 (`padding: 3px`).
pub const AUTOCOMPLETE_FILTER_ICON_PAD: f32 = 3.0;
/// Écart entre deux boutons de filtre (`gap: 4px`).
pub const AUTOCOMPLETE_FILTER_GAP: f32 = 4.0;
/// Rayon d'angle d'un bouton de filtre (`border-radius: 4px`).
pub const AUTOCOMPLETE_FILTER_RADIUS: u8 = 4;
/// Opacité d'un filtre au repos (`opacity: 0.6`), appliquée en alpha de teinte.
pub const AUTOCOMPLETE_FILTER_IDLE_ALPHA: u8 = 153;
/// Marge gauche de la bande de filtres — `.wakfu-autocomplete-categories { padding: 6px }`.
pub const AUTOCOMPLETE_FILTER_BAR_PAD: f32 = 6.0;

// La rangée : `.wakfu-autocomplete-item` et ses enfants, relevés le 2026-09-12 au soir après le
// retour « les images sont beaucoup plus collées que sur le web, ça manque de ce côté aéré ». La
// rangée suivait jusque-là la cadence du select du jeu (28 px, écarts de 6) ; elle reprend
// désormais la géométrie du web **valeur pour valeur**, dans l'ordre où l'œil la parcourt :
//
// ```text
// 0        10   24     30            60        70                        →  fin − 10
// │ marge  │gem │  6   │ colonne 30  │  gap 10 │ nom …                     │ marge │
//                        (image 24, centrée)
// ```
//
// Pas de mise à l'échelle : le corps du nom (15 px contre 13,1 px sur le web) est déjà celui de
// l'overlay, et la gemme, elle, est demandée en 14 × 14 « comme sur le web ». Un rapport unique
// n'aurait donc pu satisfaire les deux — les cotes sont portées telles quelles.

/// Hauteur d'une rangée — `.wakfu-autocomplete-item { height: 35px }`. Pas
/// [`SELECT_ROW_HEIGHT`] (28) : la liste dépliée du jeu n'a ni gemme ni image, la rangée du web
/// est faite pour les loger avec de l'air autour.
pub const AUTOCOMPLETE_ROW_HEIGHT: f32 = 35.0;
/// Marge gauche et droite d'une rangée — `.wakfu-autocomplete-item-main { padding: 0 10px }`.
pub const AUTOCOMPLETE_ROW_PADDING_X: f32 = 10.0;
/// Côté de la boîte de la gemme de rareté — `.wakfu-autocomplete-item-rarity`, 14 × 14 en
/// `object-fit: contain` : une image 13 × 20 y entre donc en 9,1 × 14, limitée par la hauteur.
/// Posée à `left: 10px`, soit au ras de la marge.
pub const AUTOCOMPLETE_GEM_BOX: f32 = 14.0;
/// Retrait de la colonne d'image depuis la marge gauche — `.wakfu-autocomplete-item-icon
/// { margin-left: 20px }` : la colonne commence à 30 px du bord de la rangée, six pixels après la
/// boîte de la gemme.
pub const AUTOCOMPLETE_IMAGE_COLUMN_OFFSET: f32 = 20.0;
/// Largeur de la colonne d'image — `.wakfu-autocomplete-item-icon { width: 30px }`. L'image y est
/// centrée ; la colonne garde sa place quand l'image n'est pas encore arrivée du CDN.
pub const AUTOCOMPLETE_IMAGE_COLUMN: f32 = 30.0;
/// Côté de l'image d'objet — `<app-item-icon [size]="24">`.
pub const AUTOCOMPLETE_IMAGE_SIZE: f32 = 24.0;
/// Écart entre la colonne d'image et le nom — `.wakfu-autocomplete-item-main { gap: 10px }`.
pub const AUTOCOMPLETE_ROW_GAP: f32 = 10.0;
/// Écart entre le champ et le panneau déplié.
pub const AUTOCOMPLETE_PANEL_GAP: f32 = 2.0;
/// Marge intérieure du panneau, sur les quatre côtés.
pub const AUTOCOMPLETE_PANEL_PAD: f32 = 2.0;
/// Hauteur de la bande « Aucun résultat dans cette catégorie ».
pub const AUTOCOMPLETE_EMPTY_HEIGHT: f32 = 34.0;
/// Nombre de rangées visibles avant que la liste ne défile — `max-height: 175px` côté web, soit
/// cinq rangées.
pub const AUTOCOMPLETE_MAX_VISIBLE_ROWS: usize = 5;
/// Largeur de la barre de défilement du panneau au repos — `::-webkit-scrollbar { width: 8px }`
/// du dépôt web (`styles.css`), la barre que l'utilisateur a sous les yeux sur le site. Le jeu n'a
/// pas d'autocomplétion, donc pas de barre à relever pour ce panneau : c'est le web qui fait
/// référence ici, comme pour tout ce composant. Plus large que les 6 px de [`SCROLLBAR_WIDTH`],
/// mesurés sur la fenêtre Options du jeu (retour du 2026-09-12 : « un tout petit peu plus large,
/// à l'image de l'autocomplétion sur le web »).
pub const AUTOCOMPLETE_SCROLLBAR_WIDTH: f32 = 8.0;
/// Ce que le rail déborde de la poignée, **de chaque côté** — un pixel à gauche et à droite, un
/// au-dessus et au-dessous quand elle est en butée. Le rail fait donc
/// `AUTOCOMPLETE_SCROLLBAR_WIDTH + 2` de large, et la poignée y est centrée. Retour du
/// 2026-09-12, fin de nuit : une poignée de la couleur de la liste posée sur un rail exactement
/// de sa largeur ne se distingue plus de la liste en butée — « histoire de bien voir le rail et
/// la poignée à l'intérieur ».
pub const AUTOCOMPLETE_SCROLLBAR_TRACK_INSET: f32 = 1.0;
/// Rayon de la poignée — `border-radius: 4px` du web, en cohérence avec sa largeur de 8. Le rail,
/// plus large d'un pixel de chaque côté, prend le rayon de plus pour rester concentrique.
pub const AUTOCOMPLETE_SCROLLBAR_RADIUS: u8 = 4;
/// Teinte de la poignée **au repos** — **le fond de la liste elle-même**, [`SELECT_LIST_FILL`].
/// Sur son rail plus sombre ([`AUTOCOMPLETE_SCROLLBAR_TRACK`]), la poignée se lit comme un
/// morceau de liste qui glisse dans une gouttière. Retour du 2026-09-12, nuit : « par défaut la
/// barre doit avoir la couleur du fond du résultat de l'autocomplétion, pas la couleur grise » —
/// ce gris était [`HEADING_TEXT`], gardé un temps parce que l'utilisateur l'avait sous les yeux
/// et avait dit « conserver les couleurs de base » ; c'est lui qui « perturbait ».
pub const AUTOCOMPLETE_SCROLLBAR_THUMB: Color32 = SELECT_LIST_FILL;
/// Teinte de la poignée **sous le pointeur et pendant le glissement** — [`SELECT_ROW_HIGHLIGHT`],
/// le fond d'une rangée survolée. Retour du 2026-09-12 au soir : plus d'élargissement (la barre
/// garde ses 8 px), c'est la teinte qui dit « tu peux agir dessus », « la même couleur que sur les
/// éléments ».
pub const AUTOCOMPLETE_SCROLLBAR_THUMB_HOVERED: Color32 = SELECT_ROW_HIGHLIGHT;
/// Fond du rail, **plus sombre que la liste** pour qu'on lise où la barre passe — même retour.
/// Le web fait son rail (`--scrollbar-track`, `#1a1a1a`) à 68 % de la surface qui le porte
/// (`--surface-raised`, `#262626`) ; c'est ce rapport, appliqué au brun de [`SELECT_LIST_FILL`]
/// (`#675d46`), qui donne cette valeur.
pub const AUTOCOMPLETE_SCROLLBAR_TRACK: Color32 = Color32::from_rgb(0x47, 0x3F, 0x30);

/// Côté de la zone cliquable d'une action de rangée — voir [`AutocompleteEntry::action`].
pub const AUTOCOMPLETE_ACTION_BOX: f32 = 22.0;

/// Encre du glyphe d'une action de rangée, dans cette boîte.
pub const AUTOCOMPLETE_ACTION_GLYPH: f32 = 14.0;

/// Glyphe d'une action de rangée AU REPOS — **la teinte du rail de la barre de défilement de ce
/// même panneau** ([`AUTOCOMPLETE_SCROLLBAR_TRACK`]).
///
/// Demande explicite du 2026-09-13, et elle a une logique : au repos, cette action est un **affordant
/// discret**, du même ordre qu'un rail de défilement — présent, disponible, mais qui ne réclame pas
/// l'attention dans une liste où l'œil cherche des noms. Elle ne prend sa couleur pleine que sous
/// le pointeur.
pub const AUTOCOMPLETE_ACTION_IDLE: Color32 = AUTOCOMPLETE_SCROLLBAR_TRACK;

/// Glyphe d'une action de rangée SOUS LE POINTEUR — l'or du jeu, la couleur d'état de ce design
/// system.
pub const AUTOCOMPLETE_ACTION_HOVERED: Color32 = TEXT_GOLD;
/// Longueur minimale de la poignée — **choix** : à 115 résultats pour cinq rangées visibles, le
/// minimum d'egui (12 px) donnait un point plutôt qu'une poignée.
pub const AUTOCOMPLETE_SCROLLBAR_MIN_HANDLE: f32 = 24.0;
/// Longueur minimale de la requête avant toute recherche — `MIN_QUERY_LENGTH`
/// (`wakfu-search.service.ts`), comptée sur la requête NORMALISÉE, pas sur la frappe brute.
pub const AUTOCOMPLETE_MIN_QUERY_LEN: usize = 3;
/// Corps du nom d'une entrée.
pub const AUTOCOMPLETE_ENTRY_FONT_SIZE: f32 = 15.0;
/// Corps de la mention de droite (« déjà suivi »), et du message de catégorie vide.
pub const AUTOCOMPLETE_MENTION_FONT_SIZE: f32 = 13.0;
/// Marge droite de la mention.
pub const AUTOCOMPLETE_MENTION_MARGIN: f32 = 10.0;

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

/// Teinte du glyphe d'un bouton icône posé **dans un panneau** (socle kaki `button-icon.png`) —
/// **blanc pur, dans les DEUX états**.
///
/// Retour utilisateur 2026-09-14, deux captures du jeu à l'appui (le bouton « Jouer le son
/// d'alerte », au repos et survolé) : sur ce socle-là, le jeu peint un glyphe blanc et ne le change
/// pas au survol — c'est **la texture du socle** qui s'éclaircit, et elle seule. Le couple
/// [`ICON_TINT`] / [`ICON_TINT_HOVER`] (gris clair → or) reste celui du contexte premier plan, où
/// il a été mesuré : `menu-button-icon-first-plan.png` ne montre que des socles bleus, elle
/// n'arbitrait donc rien pour le socle de panneau, qui avait hérité de ces teintes par défaut.
pub const PANEL_ICON_TINT: Color32 = Color32::WHITE;

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

/// Corps du numéro de version peint à GAUCHE de la bannière (`design::window(..).version(true)`).
///
/// Même police, même blanc et même ombre portée que le titre, à un seul réglage près : la taille.
/// **13 contre les 21 du titre** — choisi par l'utilisateur sur planche comparative (13/15/17 px
/// rendus à l'échelle, `crates/overlay-testkit/examples/banniere-version.rs`, 2026-09-14). Le
/// numéro n'est pas une information qu'on cherche : il doit se trouver quand on le cherche, pas
/// tenir tête au titre centré.
///
/// Son ancrage n'a pas de constante propre : il reprend [`WINDOW_CLOSE_MARGIN`] telle quelle, pour
/// que le numéro à gauche et la croix à droite soient à la MÊME distance de leur bord (demande
/// explicite de l'utilisateur). Une seconde constante de même valeur les aurait laissés diverger au
/// premier réglage de l'une des deux.
pub const WINDOW_VERSION_FONT_SIZE: f32 = 13.0;

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

// Bouton de fermeture de la bannière — tout mesuré sur `window-close.png` et
// `window-close-hover.png` (deux recadrages 48 × 48 du bouton du jeu, échelle 1, captures
// utilisateur du 2026-09-13), par `tools/design-system/build_window_close.py`. Le bouton est un
// carré arrondi TRANSLUCIDE posé sur la bannière — les hachures se voient au travers —, ce
// qu'aucune texture ne peut porter : il est peint, avec ces valeurs.

/// Côté du bouton de fermeture — 32 px, comme le pas numérique et non comme le bouton icône (36).
/// Mesuré identique au repos et au survol : le survol ne grossit pas le carré.
pub const WINDOW_CLOSE_SIZE: f32 = 32.0;

/// Marge entre le carré et le bord droit de la fenêtre — 12 px, ce qui le centre aussi
/// verticalement dans la bannière de 56 (12 + 32 + 12). Le jeu n'a qu'une cote pour les deux.
pub const WINDOW_CLOSE_MARGIN: f32 = 12.0;

/// Rayon des angles du carré — l'antialiasing du survol rentre de 4, 2 puis 1 px sur ses trois
/// premières rangées, ce qu'un rayon de 5 reproduit.
pub const WINDOW_CLOSE_RADIUS: u8 = 5;

/// Voile du carré au repos — noir à **0,168** : intérieur mesuré à 83,2 % de la bannière nue
/// autour. À peine visible, et c'est voulu : le jeu ne signale le bouton qu'au survol.
pub const WINDOW_CLOSE_FILL: Color32 = Color32::from_black_alpha(43);

/// Liseré d'1 px au repos, PAR-DESSUS le voile — noir à 0,115, pour que le bord cumulé tombe aux
/// 73,6 % mesurés (1 − 0,832 × (1 − 0,115) = 0,264). Absent au survol, où le bord (64,2 %) ne se
/// distingue pas de l'intérieur.
pub const WINDOW_CLOSE_RIM: Color32 = Color32::from_black_alpha(29);

/// Voile du carré au survol — noir à **0,366** : intérieur à 63,4 % de la bannière nue.
pub const WINDOW_CLOSE_FILL_HOVER: Color32 = Color32::from_black_alpha(93);

/// Encre de la croix — 12 px pour un socle de 32, frange d'antialiasing comprise (le cœur à
/// seuil 150 fait 10). C'est la taille du FICHIER `icon-close-window.png`, extrait de ces mêmes
/// captures : le rapport est donc exact, pas une normalisation.
pub const WINDOW_CLOSE_ICON_CONTENT: f32 = 12.0;

/// Teinte de la croix — `#F4D89F` dans les DEUX états, le pic RGB relevé au repos comme au survol.
/// Même or que [`ICON_TINT_HOVER`] et [`STEPPER_ICON_TINT`] : le survol de ce bouton ne se lit pas
/// sur son glyphe, seulement sur son voile.
pub const WINDOW_CLOSE_ICON_TINT: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9F);

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

/// Écart entre un titre de section et la première ligne qui le suit — **13px depuis le
/// 2026-09-14**, contre 7 auparavant.
///
/// Ce qui se relève n'est pas cet écart mais la **cote d'ensemble** : « 30 px entre le haut d'un
/// titre de section et le haut de la première ligne qui le suit ». Vérifiée au pixel sur les sept
/// sections des cinq onglets du jeu où un titre est immédiatement suivi d'un contrôle — 30, 30,
/// 30, 30, 30, 28, 32, l'écart tenant à la rondeur de la capitale initiale. Un titre réservant
/// [`HEADING_INK_HEIGHT`] (16px), il reste **13px** sous lui, et non 7 : à 7, l'overlay mesurait
/// 24px là où le jeu en donne 30 — « le titre est très collé à la suite », retour utilisateur.
///
/// Les deux candidates 13 et 14 ne se départageaient pas au calcul (le bord peint d'une case
/// commence un pixel sous le rect qui la porte) : rendues toutes les deux par le vrai moteur puis
/// remesurées avec la méthode appliquée au jeu, 13 donne 30px et 16px du bas de l'encre au haut de
/// la case, les deux cotes du jeu exactement ; 14 donne 31 et 17.
///
/// **Vaut pour TOUS les écrans** depuis le 2026-09-14 (2) : cinq d'entre eux posaient jusque-là
/// leur propre écart — une demi-`SECTION_GAP` dans les onglets Suivi, Alertes, Chat et Raccourcis,
/// un littéral de 8 dans la fenêtre de recette — et le même titre s'ouvrait sur quatre valeurs
/// différentes selon l'endroit. Retirés sur demande de l'utilisateur : « les espacements doivent
/// être génériques ». Seul `Heading::preview_trailing_gap` en change encore, et uniquement dans les
/// planches de simulation qui servent à mesurer une valeur candidate.
pub const HEADING_TO_ROW: f32 = 13.0;

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
/// menu/combobox ailleurs), un seul réglage couvre donc TOUS les tooltips de l'appli. Les deux
/// enveloppes maison (`combat::show_tooltip_above`, `watchlist::show_tooltip_left`) ont été
/// remplacées par `components::tooltip` le 2026-09-11, et les derniers `on_hover_text` ponctuels le
/// 2026-09-13 : il n'existe plus **aucun** chemin d'infobulle qui contourne le composant.
///
/// **Alpha remonté de 209 à 240 le 2026-09-13**, sur retour utilisateur (« le fond est un peu trop
/// translucide [...] un tout petit peu moins »), et c'est un écart ASSUMÉ à la mesure ci-dessus,
/// pas une correction de celle-ci : `a≈0,82` reste ce que la capture du jeu donne. Ce qu'elle ne
/// dit pas, c'est sur QUOI l'infobulle se pose ici. Le jeu la pose sur son propre décor ; cet
/// overlay la pose aussi par-dessus ses propres écrans, où ce qui passe au travers est du TEXTE —
/// et un titre de section lu en filigrane derrière une infobulle est précisément ce que la planche
/// `options_suivi_infobulle_tuile` montrait encore à 232.
///
/// Valeur choisie sur ce résidu, pas à l'œil. La composition est linéaire, donc ce qui reste
/// visible d'un texte clair (luminance ~230) sur une zone vide (~30) vaut `(1−a)·200` :
///
/// | alpha | opacité | écart résiduel |
/// | --- | --- | --- |
/// | 209 | 82 % | 36 niveaux — le texte derrière se lit |
/// | 232 | 91 % | 18 niveaux — il se devine encore |
/// | **240** | **94 %** | **12 niveaux** — il ne se distingue plus du fond |
/// | 250 | 98 % | 4 niveaux — indiscernable, mais l'infobulle n'est plus translucide |
///
/// Le même retour demandait dans la même phrase un texte « plus blanc » : c'était l'autre moitié du
/// même problème de contraste, et elle se règle sans toucher à ce jeton (voir `TOOLTIP_TEXT` — la
/// moitié des composants la manquaient faute de passer par le composant d'infobulle).
pub const TOOLTIP_BG_FILL: Color32 = Color32::from_rgba_unmultiplied_const(0x15, 0x17, 0x1C, 240);

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

// ---------------------------------------------------------------------------------------------
// Le contenu flottant — `tokens::OVERLAY_ACCENT`
// ---------------------------------------------------------------------------------------------

/// Accent du **contenu qui flotte par-dessus le jeu** — le cyan `#00d2ff`.
///
/// **Décision utilisateur du 2026-09-10**, et le seul point où l'overlay a une contrainte que le
/// jeu n'a pas : les panneaux Combat et Suivi prennent les formes et la typographie du jeu, mais
/// **gardent leur accent cyan**. Ce n'est pas un compromis mou. Ces panneaux se lisent *par-dessus*
/// le jeu, sur un fond arbitraire et mouvant ; `#00d2ff` n'existe nulle part dans l'interface
/// Wakfu, et c'est précisément ce qui l'empêche de s'y confondre. Une jauge de dégâts or posée sur
/// un décor or se cherche.
///
/// La valeur vient du thème sombre du dépôt web (`styles.css`, `:root --accent`) — elle était
/// jusqu'ici recopiée en deux constantes locales, `panels::combat::ACCENT` et
/// `panels::watchlist::ACCENT`, identiques au pixel et sans lien déclaré entre elles. Deux copies
/// d'une même décision dérivent à la première retouche.
///
/// **À ne pas confondre avec les jetons du jeu.** Ce qui vit *dans* une fenêtre — un bouton, un
/// champ, un onglet — prend les teintes mesurées du client. `OVERLAY_ACCENT` est réservé à ce qui
/// n'a pas de fenêtre : les jauges, les compteurs et les bandeaux qui se posent sur l'écran de jeu.
pub const OVERLAY_ACCENT: Color32 = Color32::from_rgb(0x00, 0xD2, 0xFF);

// ---------------------------------------------------------------------------------------------
// Emplacement d'objet — `design::item_slot`
//
// Ces valeurs viennent de `panels::watchlist`, où elles décrivaient la tuile du bandeau Suivi, et
// avaient été affinées par plusieurs retours utilisateur **sur les cotes du portage web**.
//
// **Le carré et ses coins sont passés aux cotes du client le 2026-09-12** : `docs/design-tokens.json`
// (analyse pixel de captures réelles) donne `item_slot_square` 63-64 et `item_slot_border` 2, contre
// 58 et un rayon de 10 hérités du CSS. C'est la vague 3 du lot 5 — « les formes migrent au langage
// du jeu ».
//
// Ce qui N'A PAS suivi, et pourquoi : `item_slot_gap` (2 px dans le jeu) décrit la densité d'une
// grille d'inventaire, pas celle du bandeau Suivi de l'overlay, dont l'espacement de 12 px vient
// d'un réglage utilisateur et n'appartient de toute façon pas au composant — un emplacement ne
// connaît pas son voisin, c'est l'appelant qui espace.
// ---------------------------------------------------------------------------------------------

/// Côté d'un emplacement d'objet — **64 px, la cote du client**.
///
/// `docs/design-tokens.json` relève `item_slot_square` entre 63 et 64 px : une fourchette, parce
/// que la mesure pixel d'un bord adouci n'a pas de frontière nette. 64 est retenu par une
/// coïncidence qui n'en est pas une : le liseré des textures `Border-*.webp` occupe 1/30ᵉ de leur
/// canevas (17 px sur les 512 qu'elles faisaient alors), soit **2,1 px une fois rendu à 64** — la
/// valeur d'`item_slot_border`
/// relevée sur les mêmes captures. À 58 px (la cote du portage web, `.kpi` de
/// `tracker-strip.component.css`, en place jusqu'au 2026-09-12) il en faisait 1,9.
pub const ITEM_SLOT_SIZE: f32 = 64.0;

/// Rayon des coins — 2 px.
///
/// Le relevé range les emplacements dans `corner_style_inputs_lists`
/// (« square_or_near_square »), loin des 10 px hérités du CSS : dans le jeu, une case d'inventaire
/// est un carré. 2 plutôt que 0 parce que le contour extérieur des textures de rareté est lui-même
/// légèrement arrondi (rayon ≈ 1/16ᵉ du canevas, soit ≈ 4 px à 64) — un fond parfaitement
/// rectangulaire pointerait hors de ses coins.
pub const ITEM_SLOT_ROUNDING: f32 = 2.0;

/// Marge intérieure des textures `Border-*.webp`, en fraction de leur canevas.
///
/// **Mesurée** : la fenêtre où l'icône se peint commence à 13 px et finit à 114 px sur le canevas
/// de 128, à l'identique sur les sept fichiers. `1 - 2 × ce ratio` donne donc la fraction du côté
/// occupée par cette fenêtre, soit ≈ 0,797.
///
/// Le rapport est **inchangé depuis la réduction des textures** du 2026-09-12 : 13/128 vaut
/// exactement 52/512, la valeur relevée sur le canevas d'origine. Une fraction, pas une cote —
/// c'est ce qui a permis de changer l'échelle des assets sans toucher à ce jeton.
pub const ITEM_SLOT_BORDER_INNER_RATIO: f32 = 13.0 / 128.0;

/// Fraction de la fenêtre intérieure qu'occupe réellement l'icône — 0,96, et pas 1.
///
/// Les captures du jeu montrent toujours une petite marge entre l'icône et son cadre, jamais un
/// remplissage au pixel. Relevé de 0,9 à 0,96 sur retour utilisateur (« les objets doivent être
/// plus gros ») une fois la bordure repeinte SOUS l'icône — plus aucun risque qu'elle recouvre un
/// débord.
pub const ITEM_SLOT_ICON_FILL: f32 = 0.96;

/// Fraction du carré qu'occupe l'icône d'un emplacement **sans rareté** — ≈ 0,517.
///
/// Une **fraction**, pas une cote : elle est née d'un rapport mesuré (30 px sur 58) et suit depuis
/// le côté qu'on donne à l'emplacement — 33 px sur les 64 d'aujourd'hui. Le carré a changé le
/// 2026-09-12 en passant aux cotes du client, ce rapport non : c'est lui que le retour utilisateur
/// avait réglé, pas les 30 px.
///
/// **Un cadre simple ne mange pas de marge**, contrairement à une bordure de rareté dont la fenêtre
/// intérieure impose la sienne. Rien ne contraint donc l'icône, et il a fallu une cote propre :
/// `app-item-icon [size]="30"` du template web, sur une tuile de 58.
///
/// Découvert en migrant `watchlist::entry_tile` : la première version du composant faisait occuper
/// tout le carré à l'icône d'un cadre simple, et le snapshot du décompte l'a montré — un monstre
/// deux fois trop gros dans sa tuile. Une bordure de rareté masquait le défaut sur les objets,
/// c'est l'ennemi qui l'a révélé.
pub const ITEM_SLOT_PLAIN_ICON_FILL: f32 = 30.0 / 58.0;

/// Épaisseur du trait d'un emplacement **sans rareté** — 2 px, celui des tuiles d'ennemi.
pub const ITEM_SLOT_PLAIN_STROKE: f32 = 2.0;

/// Marge entre le bord d'un emplacement et son liseré, en fraction du côté — 4/128, soit 2 px à 64.
///
/// **Mesurée sur les textures de rareté**, à l'identique sur les sept : sur la ligne médiane du
/// canevas de 128, l'alpha reste sous 40 jusqu'à x = 3 puis saute à 193 en x = 4. Le liseré de ces
/// textures n'est donc pas collé au bord — il flotte à 2 px une fois rendu à 64, et le fond de
/// l'emplacement dépasse tout autour.
///
/// C'est ce que [`ITEM_SLOT_BORDER_CORNER_RATIO`] et ce jeton servent à retrouver : **tout ce qui
/// se pose sur cette bordure** — le trait d'un cadre simple, le liseré de sélection d'un panneau —
/// doit viser le même anneau, sinon il se voit décalé de ces 2 px. Retour utilisateur du
/// 2026-09-13, sur la sélection multiple du bandeau : « il faut vraiment que ça se superpose ».
pub const ITEM_SLOT_BORDER_INSET_RATIO: f32 = 4.0 / 128.0;

/// Rayon des coins du liseré, en fraction du côté — 6/128, soit ≈ 3 px à 64.
///
/// **Mesuré sur le même canevas** : le bord extérieur du liseré part de (10, 4) et rejoint (4, 9),
/// un quart de cercle de rayon 6 autour de (10, 10). Rapporté au bord de l'emplacement, cela fait
/// ≈ 5 px — d'où les « ≈ 4 px » estimés à l'œil dans la doc d'[`ITEM_SLOT_ROUNDING`], qui visait le
/// contour extérieur de la texture et non le liseré lui-même.
///
/// Le cadre simple utilisait 2 px et le liseré de sélection du Suivi 4 : ni l'un ni l'autre ne
/// tombait sur l'arc de la texture, ce qui se voyait aux coins dès qu'un objet et un monstre
/// étaient côte à côte.
pub const ITEM_SLOT_BORDER_CORNER_RATIO: f32 = 6.0 / 128.0;

/// Liseré d'un emplacement **sélectionné** — l'or du design system, la couleur d'état de cette
/// interface.
pub const ITEM_SLOT_SELECTED_BORDER: Color32 = TEXT_GOLD;

/// Liseré d'un emplacement sélectionné **en vue d'une suppression** — le rouge du bouton
/// « Annuler », seul rouge que le design system ait mesuré (voir [`INFO_ALERT`]).
///
/// Demande utilisateur du 2026-09-13 : une sélection multiple n'est pas toujours destructive, et
/// les deux ne doivent pas se ressembler. L'or dit « retenu », le rouge dit « retenu pour être
/// détruit » — même géométrie, même case, seule la couleur change.
pub const ITEM_SLOT_SELECTED_BORDER_DANGER: Color32 = INFO_ALERT;

/// Retrait de la case à cocher depuis le coin d'un emplacement en mode sélection — 7 px.
///
/// Le liseré occupe les pixels 2 à 4 (voir [`ITEM_SLOT_BORDER_INSET_RATIO`]) : 4 collait la case
/// contre lui, 5 la posait au contact. **Deux retours utilisateur pour arriver ici** — « décaler la
/// checkbox d'un pixel » le 2026-09-13, puis « d'au moins 2 px » sur la planche suivante. 7 laisse
/// donc trois pixels de fond entre le liseré et la case, sur les deux axes : ne décaler qu'en
/// abscisse poserait la case de travers dans son coin, à 3 px du bord gauche et 1 px du bord haut.
pub const ITEM_SLOT_SELECTION_INSET: f32 = 7.0;

/// En dessous de ce côté, un emplacement ne peut plus rien montrer — et le dit une fois au journal
/// (clause 4 du contrat de composant).
///
/// **Seuil choisi, pas mesuré**, et la raison est arithmétique : un liseré de
/// [`ITEM_SLOT_PLAIN_STROKE`] mange 2 px de chaque côté, et une bordure de rareté réserve déjà
/// 2 × [`ITEM_SLOT_BORDER_INNER_RATIO`] de son carré. À 8 px, il reste 4 px au milieu, soit deux pixels d'icône une fois la
/// marge de [`ITEM_SLOT_ICON_FILL`] retirée. Personne ne demande sciemment un emplacement de cette
/// taille : c'est le signe d'une largeur calculée qui est tombée à rien, exactement ce qu'un
/// journal doit rattraper.
pub const ITEM_SLOT_MIN_SIZE: f32 = 4.0 * ITEM_SLOT_PLAIN_STROKE;

/// Fond d'un emplacement — `#1e1e1e`, repris du portage web (`--surface`).
///
/// **Peint dans `item_slot::border_ring`, pas sur le carré de l'emplacement** : posé sur le carré
/// entier, il dépassait des quatre coins des textures de rareté — qui posent leur liseré en retrait
/// de 2 px, arrondi à 3 — et dessinait un carré sombre derrière une bordure arrondie (retour
/// utilisateur du 2026-09-13). Sous une rareté, il n'est donc plus visible du tout, la texture le
/// couvrant entièrement ; sur un emplacement sans rareté, c'est toujours le seul fond, et il monte
/// jusque sous le trait gris.
pub const ITEM_SLOT_BACKGROUND: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);

/// Trait d'un emplacement sans rareté — `#4d4d4d` (`--border-strong` du web).
pub const ITEM_SLOT_PLAIN_BORDER: Color32 = Color32::from_rgb(0x4D, 0x4D, 0x4D);

/// Corps du compteur — 14 px.
///
/// **Trois retours pour y arriver** : 11 px était « pas du tout lisible », 15 px « beaucoup trop
/// élevé » une fois comparé en jeu, 13 px un palier intermédiaire, 14 px la valeur retenue après
/// test en conditions réelles.
pub const ITEM_SLOT_COUNT_FONT_SIZE: f32 = 14.0;

/// Corps de la fraction cible — 10 px, plus petite que le nombre courant (« on la mettrait en onze
/// ou en dix »).
pub const ITEM_SLOT_TARGET_FONT_SIZE: f32 = 10.0;

/// Marge du compteur au bord droit — 6 px, assez pour rester lisible par-dessus le liseré d'une
/// bordure de rareté sans empiéter dessus.
pub const ITEM_SLOT_COUNT_INSET_RIGHT: f32 = 6.0;

/// Marge du compteur au bord bas — 4 px. **Distincte de la marge droite** depuis le retour du
/// 2026-09-06 (« décale d'un pixel vers la gauche et descends-le d'un pixel vers le bas ») : les
/// deux n'ont plus de raison d'être égales.
pub const ITEM_SLOT_COUNT_INSET_BOTTOM: f32 = 4.0;

/// De combien le nombre courant remonte au-dessus de la fraction cible — 12 px.
pub const ITEM_SLOT_TARGET_LINE_OFFSET: f32 = 12.0;

/// Couleur d'un compteur simple — blanc.
pub const ITEM_SLOT_COUNT_TEXT: Color32 = Color32::WHITE;

/// Couleur du nombre **courant** d'une fraction — l'or des kamas (`--kama-color` du web).
pub const ITEM_SLOT_COUNT_CURRENT: Color32 = Color32::from_rgb(0xFF, 0xD7, 0x00);

/// Couleur de la fraction cible — `#b0b0b0`, éclairci le 2026-09-06 par rapport au gris sourd
/// qu'elle portait avant.
pub const ITEM_SLOT_TARGET_TEXT: Color32 = Color32::from_rgb(0xB0, 0xB0, 0xB0);

/// Côté du **glyphe de mode** (cible du décompte, drapeau de l'objectif — voir
/// `item_slot::SlotGlyph`) — 8 px, un peu moins que les 9 px de la maquette validée : « un tout
/// petit peu réduit pour qu'il ne déborde pas sur la bordure » (2026-09-17).
pub const ITEM_SLOT_GLYPH_SIZE: f32 = 8.0;

/// Retrait du glyphe depuis le coin **haut-gauche**, sur les deux axes — 8 px, un de plus que la
/// case à cocher ([`ITEM_SLOT_SELECTION_INSET`]), qui occupe ce même coin en mode sélection.
///
/// Le liseré finit au pixel 4 (voir [`ITEM_SLOT_BORDER_INSET_RATIO`]), et son antialiasing en
/// mord un cinquième. Contrairement à la case, le glyphe est **cerné de noir** d'un pixel tout
/// autour : à 7, sa forme n'avait qu'un pixel de fond entre son cerne et le liseré, et se lisait
/// collée. À 8, le cerne se pose là où commence la case (7) et la forme garde trois pixels de
/// fond depuis le liseré. Retour utilisateur du 2026-09-17 : « en haut à gauche, avec deux à
/// trois pixels d'écart de la bordure, en haut et sur le côté, pour qu'il ne soit pas collé » —
/// le coin bas-gauche de la première version, sur la ligne de base de la fraction, se lisait
/// comme un morceau du compteur.
pub const ITEM_SLOT_GLYPH_INSET: f32 = ITEM_SLOT_SELECTION_INSET + 1.0;

// ---------------------------------------------------------------------------------------------
// Jauge — `design::meter`
//
// Ces valeurs viennent de `panels::combat`, où elles décrivaient la barre de dégâts. Elles ont été
// **mesurées pixel par pixel** sur une maquette fournie par l'utilisateur (capture du 2026-09-02),
// après une première tentative approximée à l'œil qui « dénotait du jeu ». Elles remontent ici
// telles quelles le 2026-09-11 : ce commit déplace du code.
// ---------------------------------------------------------------------------------------------

/// Hauteur d'une jauge — 16 px, mesurée sur la maquette. Une itération l'avait portée à 18 sans
/// nécessité (« elle est plus haute que celle que je t'ai fournie »), revenue à la mesure.
pub const METER_HEIGHT: f32 = 16.0;

/// Arrondi des coins — 4 px, et **pas `hauteur / 2`**.
///
/// Une première itération en avait fait un stade complet : « le border radius est beaucoup trop
/// rond dans ce que tu as produit », capture de comparaison à l'appui. La maquette n'a qu'un
/// arrondi léger.
pub const METER_ROUNDING: f32 = 4.0;

/// Épaisseur de **chacune** des deux bordures concentriques — 2 px sur une jauge de 16.
pub const METER_BORDER_WIDTH: f32 = 2.0;

/// Bordure extérieure — gris moyen `(72,72,74)`, **pas noir**.
///
/// Contre-intuitif : l'utilisateur la décrivait comme sombre, la mesure dit l'inverse. C'est la
/// bordure *intérieure* qui est presque noire.
pub const METER_OUTER_BORDER: Color32 = Color32::from_rgb(72, 72, 74);

/// Bordure intérieure — `(34,35,39)`, presque noire. C'est elle qui donne l'effet de double
/// bordure.
pub const METER_INNER_BORDER: Color32 = Color32::from_rgb(34, 35, 39);

/// Piste vide — `(22,23,27)`.
pub const METER_TRACK: Color32 = Color32::from_rgb(22, 23, 27);

/// Curseur de fin de remplissage — `(191,191,191)`.
///
/// Le petit trait clair vertical à l'extrémité du remplissage, décrit comme « une petite barre
/// blanche pour dire c'est ici que je suis ». Il manquait entièrement à la première tentative.
/// Couleur **fixe** : c'est un repère de position, pas une donnée à lire.
pub const METER_END_CAP: Color32 = Color32::from_rgb(191, 191, 191);

/// Largeur du curseur de fin — 2 px.
pub const METER_END_CAP_WIDTH: f32 = 2.0;

/// Reflet du haut du remplissage — `#0dbebe`, une **couleur explicite**.
///
/// Demandée telle quelle (retour du 2026-09-05) plutôt que dérivée du remplissage par
/// éclaircissement : l'utilisateur veut ce ton précis, pas « n'importe quel bleu-vert plus clair ».
/// C'est aussi pourquoi il ne suit pas la teinte de remplissage quand celle-ci varie.
pub const METER_HIGHLIGHT: Color32 = Color32::from_rgb(0x0D, 0xBE, 0xBE);

/// Fraction de la hauteur du remplissage qu'occupe le reflet — le tiers supérieur, 0,35.
pub const METER_HIGHLIGHT_RATIO: f32 = 0.35;

/// Teinte de remplissage par défaut — `#077982`, un sarcelle plus sombre que
/// [`OVERLAY_ACCENT`].
///
/// **Délibérément distincte de l'accent**, après un aller-retour : les deux ont été fusionnées au
/// 6e retour (le magenta `#ff02ff` du 5e ayant été rejeté), puis re-séparées au 8e. Le pourcentage
/// sur le portrait, lui, est revenu à l'accent au 9e — barre et pourcentage n'ont donc plus la
/// même couleur, et c'est voulu.
pub const METER_FILL: Color32 = Color32::from_rgb(0x07, 0x79, 0x82);

// ---------------------------------------------------------------------------------------------
// Le reste du vocabulaire du contenu flottant — `OVERLAY_*`
//
// Remontés des panneaux le 2026-09-12. Ces valeurs viennent du thème sombre du dépôt web
// (`styles.css`, `:root`) : l'overlay est un portage, et ce sont ses surfaces à lui, celles qui
// n'ont pas de fenêtre du jeu autour. Ne pas les confondre avec les jetons mesurés sur le client —
// un bouton, un champ, un onglet prennent les teintes du jeu, pas celles-ci.
//
// **Trois d'entre elles existaient en double**, recopiées dans `panels::combat` ET
// `panels::watchlist` sans lien déclaré : le fond translucide (sous deux noms différents,
// `LEADER_PANEL_FILL` et `PANEL_BACKDROP_FILL`) et les deux teintes de survol. C'est exactement le
// motif qui avait déjà été corrigé pour l'accent — deux copies d'une même décision dérivent à la
// première retouche.
// ---------------------------------------------------------------------------------------------

/// Fond translucide d'un bandeau posé sur l'écran de jeu — noir bleuté, alpha 150.
///
/// Approximation d'un bandeau du client (les captures de référence n'ont pas de canal alpha
/// exploitable) : assez opaque pour détacher le contenu du décor sans devenir un pavé plein. La
/// planche fournie par l'utilisateur le 2026-09-06 la montre derrière la bande de boutons de
/// premier plan du jeu, visible dans l'écart entre deux boutons adjacents.
///
/// **Trois consommateurs, un seul jeton** depuis le 2026-09-12 : la ligne leader du panneau
/// Combat, son bloc de sorts, et le fond du carré de contrôle du Suivi. Les deux premiers la
/// nommaient `LEADER_PANEL_FILL`, le troisième `PANEL_BACKDROP_FILL` — la doc de celui-ci disait
/// déjà « même teinte que `combat::LEADER_PANEL_FILL` », ce qui est un doublon qui s'annonce.
pub const OVERLAY_BACKDROP: Color32 = Color32::from_rgba_unmultiplied_const(10, 12, 16, 150);

/// `--tint-medium` du thème web — voile blanc à 31/255, fond d'un bouton discret au repos.
///
/// Était en double : bouton de fermeture d'un toast (Suivi) et surface du panneau Combat.
pub const OVERLAY_TINT_MEDIUM: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 31);

/// `--tint-strong` du thème web — le même voile à 46/255, pour le survol.
pub const OVERLAY_TINT_STRONG: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 46);

/// `--surface-well` du thème web — fond creusé, celui du badge de compteur d'une tuile de suivi.
pub const OVERLAY_SURFACE_WELL: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);

/// `--surface-raised` du thème web — fond surélevé, celui de la carte d'un toast de ramassage.
///
/// Le web y pose un dégradé à deux arrêts **identiques** : un aplat suffit.
pub const OVERLAY_SURFACE_RAISED: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);

/// `--border-strong` du thème web — bordure d'un badge de compteur, et bordure « monstre ».
pub const OVERLAY_BORDER_STRONG: Color32 = Color32::from_rgb(0x4D, 0x4D, 0x4D);

/// `--text-bright` du thème web — le nom d'un objet ou d'un monstre dans un toast.
pub const OVERLAY_TEXT_BRIGHT: Color32 = Color32::from_rgb(0xF2, 0xF2, 0xF2);

/// `--text-muted` du thème web — la cible grisée d'un décompte, le texte d'une tuile « + » / « − ».
pub const OVERLAY_TEXT_MUTED: Color32 = Color32::from_rgb(0x88, 0x88, 0x88);

/// Texte d'une barre de dégâts — `#ebf0f5`, **et ce n'est pas [`OVERLAY_TEXT_BRIGHT`]**.
///
/// Les deux sont des blancs cassés voisins, d'origines différentes : celui-ci est mesuré pixel par
/// pixel sur la maquette de barre fournie par l'utilisateur (capture du 2026-09-02), l'autre vient
/// du `:root` web. Les fusionner sur leur ressemblance effacerait la provenance de l'un des deux —
/// et la mesure perdrait contre la recopie.
pub const OVERLAY_TEXT: Color32 = Color32::from_rgb(235, 240, 245);

// ---------------------------------------------------------------------------------------------
// Portrait — `design::portrait`
// ---------------------------------------------------------------------------------------------

/// Teinte d'un portrait de combattant **KO** — un gris moyen.
///
/// **C'est une approximation, et elle n'est pas toujours utilisée.** Une teinte egui *multiplie* :
/// elle assombrit uniformément sans désaturer, là où un vrai niveau de gris désature. Les portraits
/// de classe ont leur version grise **précalculée** dans l'atlas et n'en ont donc pas besoin ; cette
/// teinte ne sert qu'aux portraits sans équivalent gris — une icône de monstre téléchargée, le
/// repli générique. C'est à l'appelant de savoir dans quel cas il est, lui seul a la texture.
pub const PORTRAIT_KO_TINT: Color32 = Color32::from_gray(130);

/// Corps du pourcentage incrusté sur un portrait — 12 px.
///
/// Agrandi une fois sur retour : « ça a l'air compliqué à lire, il en manque un ou deux pixels ».
pub const PORTRAIT_PERCENT_FONT_SIZE: f32 = 12.0;

/// De combien le pourcentage déborde **vers l'extérieur** du coin bas-droit — 2 px à droite, 1 en
/// bas.
///
/// Vers l'extérieur et non vers l'intérieur : « comme si on traçait un carré autour du rond et
/// qu'on plaçait le pourcentage tout en bas à droite », puis « encore un peu plus sur la droite
/// pour qu'il mange un peu moins sur le portrait ». Sur un portrait rond, ce coin du carré
/// englobant est de toute façon hors du disque.
pub const PORTRAIT_PERCENT_OFFSET: egui::Vec2 = egui::Vec2::new(2.0, 1.0);

// ---------------------------------------------------------------------------------------------
// Tableau — `design::table`
//
// Relevé du 2026-09-12 sur les trois tableaux de l'Hôtel de Vente
// (`docs/design-system/hdv-table.json`) : `interface-hdv-historique.png`,
// `interface-hdv-vente-list.png`, `interface-hdv-achat.png`. Les trois sont VIDES (« 0 Objet ») —
// tout ce qui touche au contenu d'une cellule est donc choisi, et le dit.
// ---------------------------------------------------------------------------------------------

/// Hauteur d'une ligne — **60 px exactement**, et c'est l'invariant le plus solide du relevé.
///
/// Mesuré par les transitions du zébrage sur les trois captures, à trois origines différentes
/// (200, 260, 320… / 316, 376… / 352, 412…) : le pas est le même partout. C'est aussi, à la
/// coïncidence près, la hauteur d'un en-tête de repliable ([`COLLAPSE_HEADER_HEIGHT`]) — les deux
/// restent deux jetons, rien ne dit que le jeu les tienne liés.
pub const TABLE_ROW_HEIGHT: f32 = 60.0;

/// Rembourrage au-dessus de l'encre de l'en-tête — 14 px (haut du tableau y=165, encre y=179).
pub const TABLE_HEADER_PAD_TOP: f32 = 14.0;

/// Hauteur d'ENCRE réservée par l'en-tête, et non la hauteur de sa galley — 14 px (y 179..193).
///
/// Même raisonnement que [`HEADING_INK_HEIGHT`] : réserver la galley entière ajouterait sous
/// l'en-tête une bande vide que le relevé ne montre pas.
pub const TABLE_HEADER_INK: f32 = 14.0;

/// Écart entre l'encre de l'en-tête et la première ligne — 7 px, identique sur les trois captures.
///
/// **Sa propre valeur, plus un alias de [`HEADING_TO_ROW`]** (2026-09-14). Les deux mesures
/// tombaient sur 7 px, l'une sur le HDV et l'autre sur la fenêtre Options, et le jeton était
/// partagé pour ne pas laisser dériver un doublon. La seconde s'est révélée fausse — l'écart sous
/// un titre de section vaut 13 px, voir [`HEADING_TO_ROW`] — et l'alias aurait emporté l'en-tête de
/// tableau avec elle, alors que ses trois captures disent toujours 7. Deux mesures qui coïncident
/// ne sont pas une mesure commune.
pub const TABLE_HEADER_GAP: f32 = 7.0;

/// Couleur des libellés d'en-tête — **le gris des titres de section**, [`HEADING_TEXT`].
///
/// Relevé : `#b9babb` sur les pixels pleins des six libellés, contre `#b8b9ba` pour le titre de
/// section. Un canal d'écart : c'est la même teinte, pas une seconde à déclarer.
pub const TABLE_HEADER_TEXT: Color32 = HEADING_TEXT;

/// Corps d'un libellé d'en-tête — 17 px.
///
/// 14 px d'encre de capitale, et le rapport d'encre d'egui pour cette police (0,805, mesuré lors
/// du chantier du bouton) donne 14/0,805 = 17,4. **Linéale et non serif** : les libellés d'en-tête
/// sont vérifiés sans empattement sur la capture agrandie ×4, contrairement aux titres de section.
pub const TABLE_HEADER_FONT_SIZE: f32 = 17.0;

/// Zébrage d'une ligne sur deux — un blanc à 12/255, soit **un éclaircissement, pas une teinte**.
///
/// Le tableau n'a **pas de fond propre** : le décor du jeu se lit à travers, et le zébrage
/// l'éclaircit. Deux constantes opaques seraient donc fausses dès que ce décor change, ce qui est
/// le cas permanent d'un overlay posé sur un jeu en mouvement.
///
/// La valeur vient d'un alpha **déduit colonne par colonne** — `(claire − nue) / (1 − nue/255)` —
/// sur quinze colonnes réparties de x=60 à x=1180 : médiane **12,3**, valeurs de 10,1 à 16,2. La
/// dispersion est celle du décor, pas de la mesure : deux lignes voisines ne couvrent pas le même
/// morceau de fond. Une première estimation sur deux colonnes seulement donnait 13,5 — trois
/// points de plus suffisaient à la déplacer, quinze la stabilisent.
pub const TABLE_ROW_STRIPE: Color32 = Color32::from_rgba_premultiplied(12, 12, 12, 12);

/// Rembourrage horizontal d'une cellule — **choisi, pas mesuré**.
///
/// Les trois tableaux relevés sont vides : aucune valeur n'y est alignée sur quoi que ce soit.
/// 10 px reprend [`SELECT_PADDING_X`], le rembourrage de la liste déroulante, faute de mieux —
/// à corriger dès qu'une capture d'un tableau peuplé existe.
pub const TABLE_CELL_PAD_X: f32 = SELECT_PADDING_X;

/// Hauteur du corps quand le tableau n'a rien à montrer — **choisie**.
///
/// Le jeu ne montre aucun état vide : ses tableaux à « 0 Objet » sont simplement… vides, sans
/// message. Deux hauteurs de ligne donnent au message la place de respirer sans faire un trou.
pub const TABLE_EMPTY_HEIGHT: f32 = 2.0 * TABLE_ROW_HEIGHT;

/// Couleur du message d'un tableau vide — le gris des libellés désactivés.
pub const TABLE_EMPTY_TEXT: Color32 = TEXT_DISABLED;

/// Corps du message d'un tableau vide — celui des libellés d'en-tête.
pub const TABLE_EMPTY_FONT_SIZE: f32 = TABLE_HEADER_FONT_SIZE;

/// Hauteur du corps pendant un chargement — **choisie**, assez haute pour porter le rouage.
pub const TABLE_LOADING_HEIGHT: f32 = 3.0 * TABLE_ROW_HEIGHT;

// ---------------------------------------------------------------------------------------------
// Pagination — `design::pagination`
//
// Relevé du 2026-09-12 (`docs/design-system/hdv-table.json`, nœud `pager`), puis re-mesuré à ×5 en
// écrivant le composant : ce que le relevé décrivait comme « deux flèches de 7 px » sont deux
// BOUTONS ICÔNE de 36 px portant chacun un triangle.
// ---------------------------------------------------------------------------------------------

/// Doré du libellé **et du numéro courant** — la teinte des onglets inactifs, [`TAB_LABEL_IDLE`].
///
/// Mesuré (244, 216, 158) sur les pixels pleins des deux, soit `#f4d89e` au canal près. Le relevé
/// annonçait le numéro courant en blanc : c'est faux, il est doré comme le mot « Page ». Seuls la
/// barre oblique et le total sont blancs — *ce qui bouge est en or, ce qui borne est en blanc*.
pub const PAGINATION_LABEL: Color32 = TAB_LABEL_IDLE;

/// Blanc du « / total » — blanc pur mesuré (255, 255, 255).
pub const PAGINATION_TOTAL: Color32 = Color32::WHITE;

/// Corps du libellé — **19 px, réglé au rendu et non par le calcul**.
///
/// La mesure de départ est la hauteur de CAPITALE et elle seule : le « P » de « Page » fait 13 px
/// dans le jeu (y 915..927). Mesurer « Page » en entier donnerait 17 px — le jambage du « g » —
/// pour un corps faux d'un tiers ; c'est l'erreur qui avait déjà coûté un aller-retour sur le titre
/// de repliable.
///
/// Le rapport d'encre habituel (0,805) donnait 13/0,805 ≈ 16. **Au rendu, 16 ne produit que 11 px
/// d'encre** ; 19 en produit 13, exactement comme le jeu. Vérification complète du segment
/// « Page 0 / 0 » à ce corps : 41 px pour « Page » contre 42 dans le jeu, 83 px pour le libellé
/// entier contre 82. Le rapport d'encre de cette police n'est donc pas celui du chantier du bouton,
/// et une constante posée par le calcul seul aurait été 45 % trop petite.
pub const PAGINATION_FONT_SIZE: f32 = 19.0;

/// Écart entre la fin de l'encre du libellé et le premier socle — 7 px (1174 → 1181).
pub const PAGINATION_TEXT_GAP: f32 = 7.0;

/// Gouttière entre les deux flèches — 4 px (fin du premier socle 1216, début du second 1221).
///
/// Les deux socles font 36 px chacun, d'où un pas de 40 px entre leurs centres : c'est ce que le
/// relevé avait lu comme « 39 px entre les deux débuts de flèche », à un pixel de détourage près.
pub const PAGINATION_ARROW_GAP: f32 = 4.0;

// ---------------------------------------------------------------------------------------------
// `design::confirm_dialog` — la boîte de confirmation du jeu.
//
// Mesures relevées sur `assets/design-system/interfaces/interface-confirm-box.png` (449 × 209),
// la boîte du client qui pose exactement la même forme de question (« Êtes-vous sûr(e) de vouloir
// supprimer ce build ? »). Ces valeurs sont arrivées avec la maquette de la page Alertes
// (2026-09-11), y ont vécu en constantes locales, et remontent ici le 2026-09-12 quand la boîte a
// gagné un second appelant — la garde de fermeture de la fenêtre Options.
// ---------------------------------------------------------------------------------------------

// Les sept jetons qui suivent décrivaient la boîte de confirmation **peinte à la main**, avant que
// `design::confirm_dialog` ne passe aux textures détourées (2026-09-15). Plus rien ne les peint :
// le corps, la crête et le filet de pied viennent maintenant de `confirm-box-{body,crest,foot}.png`,
// qui portent leur remplissage, leur liseré et leur arrondi dans leurs propres pixels.
//
// Ils sont conservés le temps que la comparaison avant/après soit validée — le skill `ui-component`
// interdit de retirer les constantes de l'ancien rendu avant ce feu vert. À supprimer ensuite.

/// Fond du corps — **`#585955`**, un gris CLAIR.
///
/// Histogramme de x 40..410 / y 60..110 sur la capture : `#585955` dominant, puis `#595a56` et
/// `#5a5b5c`, tous à un niveau les uns des autres. À l'opposé du kaki d'une liste déroulante, que
/// lui donnait une version antérieure de la maquette.
pub const CONFIRM_FILL: Color32 = Color32::from_rgb(0x58, 0x59, 0x55);

/// Bord du corps — le noir de bord commun au jeu, celui de [`SELECT_LIST_BORDER`].
pub const CONFIRM_BORDER: Color32 = SELECT_LIST_BORDER;

/// Épaisseur de ce bord — 2 px.
pub const CONFIRM_BORDER_WIDTH: f32 = 2.0;

/// Rayon des coins du corps — 4 px.
pub const CONFIRM_RADIUS: u8 = 4;

/// Médaillon en crête, qui déborde le haut du corps — l'or du jeu ([`TAB_LABEL_IDLE`]), comme sur
/// la capture où le disque est doré et son point d'interrogation sombre.
pub const CONFIRM_CREST: Color32 = TAB_LABEL_IDLE;

/// Rayon du médaillon — 20 px.
pub const CONFIRM_CREST_RADIUS: f32 = 20.0;

/// Côté du glyphe inscrit dans le médaillon — 14 px.
pub const CONFIRM_CREST_GLYPH: f32 = 14.0;

/// Largeur du corps — **420 px mesurés** (bords à x=13 et x=433 sur une capture de 449).
pub const CONFIRM_WIDTH: f32 = 420.0;

/// Hauteur du corps — **148 px mesurés** (y = 45..193 sur la capture).
///
/// C'était 120 px jusqu'au 2026-09-15, annoncé comme un « choix de mise en page, pas une mesure »
/// au motif que les questions posées ici tiennent sur une ligne. Le relevé `ui-blueprint` a donné
/// 148, et la texture du corps fait exactement cette hauteur : la garder évite d'étirer une bande
/// médiane pour rien.
pub const CONFIRM_HEIGHT: f32 = 148.0;

/// Largeur de la crête — 250 px, taille native de `confirm-box-crest.png`.
///
/// **Mesure, et volontairement figée** : sur la capture le bandeau doré s'arrête à 250 px quand le
/// corps en fait 420. Une seule capture ne dit pas si le jeu l'allongerait sur une boîte plus
/// large ; le figer est le choix retenu (décision utilisateur, 2026-09-15), cohérent avec les
/// embouts de bouton qui ne s'étirent pas davantage.
pub const CONFIRM_CREST_WIDTH: f32 = 250.0;

/// Hauteur de la crête — 57 px, taille native de la texture.
pub const CONFIRM_CREST_HEIGHT: f32 = 57.0;

/// De combien la crête descend **dans** le corps — 21 px.
///
/// La pointe basse du losange s'arrête à y = 53 sur la capture et son cerne sombre à 55, quand le
/// corps commence à y = 36 (repère de l'image détourée). La crête se peint donc APRÈS le corps, et
/// ne déborde au-dessus de lui que de `CONFIRM_CREST_HEIGHT - CONFIRM_CREST_OVERLAP` = 36 px.
pub const CONFIRM_CREST_OVERLAP: f32 = 21.0;

/// Largeur du filet de pied — 98 px, taille native de `confirm-box-foot.png`.
pub const CONFIRM_FOOT_WIDTH: f32 = 98.0;

/// Hauteur de ce filet — 9 px, dont la totalité déborde sous le corps.
pub const CONFIRM_FOOT_HEIGHT: f32 = 9.0;

/// Centre du bloc de la question, depuis le haut du corps — 42 px.
///
/// Mesuré : le bloc occupe y = 58..98 sur la capture détourée, soit 22..62 sous le bord du corps,
/// dont le milieu est à 42. C'est ce point que vise un `Align2::CENTER_CENTER`.
pub const CONFIRM_QUESTION_TOP: f32 = 42.0;

/// Corps de la question — 18 px.
///
/// **Ce jeton valait 15, sur une provenance fausse.** Elle invoquait « une hauteur de capitale
/// mesurée de 12 px » et le ratio d'egui de 0,805 ; une hauteur de capitale se lit sur un seul
/// glyphe, dépend de l'accent qui le surmonte et du seuil qui l'isole, et elle avait été relevée
/// sur le rendu à la main d'avant le détourage — pas sur la capture. Retour utilisateur du
/// 2026-09-15 : « un poil petite ».
///
/// La mesure qui le remplace ne dépend d'aucune conversion encre → corps, parce qu'elle compare
/// deux rendus de la **même chaîne** : les deux lignes du jeu sont rejouées dans chaque police
/// candidate par `cargo run -p overlay-testkit --example police-confirmation`, puis mesurées comme
/// la capture l'a été.
///
/// | | « Êtes-vous sûr(e) de vouloir supprimer ce » | « build ? » |
/// | --- | --- | --- |
/// | jeu | 325 px | 51 px |
/// | Light 15 (l'ancien corps) | 264 px | 41 px |
/// | **Light 18** | **317 px** | **50 px** |
/// | Light 19 | 334 px | 53 px |
///
/// Le corps 15 rendait la question **17 % trop courte**. Dix-huit l'amène à 2 % du jeu ; 19 la
/// dépasse de 3 % et gonfle la hauteur d'encre de la seconde ligne à 15 px pour 13 mesurés.
pub const CONFIRM_FONT_SIZE: f32 = 18.0;

/// Couleur de l'encre de la question — **gris clair, pas blanc**.
///
/// Mesurée au cœur des lettres de la capture (érosion 2×2, 463 px) : R 208,5 G 208,8 B 208,4. Le
/// composant demandait `Color32::WHITE`, ce qui rendait la question à 220 au cœur et 213 sur ses
/// franges, contre 208 et 197 dans le jeu — plus claire et plus dense, donc plus accrocheuse juste
/// sous l'or du médaillon.
///
/// **Elle est rigoureusement neutre**, et c'est la réponse à l'impression de jaune rapportée le
/// 2026-09-15 : mesurés, le cœur, les franges d'antialiasing, le halo et le fond sont neutres à
/// ±0,5 d'écart rouge − bleu **des deux côtés**. Le seul élément chaud de la boîte est la crête
/// dorée posée au-dessus de la question (+8,2), et elle est conforme au jeu.
pub const CONFIRM_INK: Color32 = Color32::from_rgb(0xD0, 0xD1, 0xD0);

/// Largeur où la question passe à la ligne — 344 px, la largeur utile du corps.
///
/// `CONFIRM_WIDTH - 40 - 36`, les deux paddings mesurés sur la capture. Le jeu coupe bien dans cet
/// intervalle : sa première ligne fait 325 px, et « build » (51 px) n'y tiendrait pas.
///
/// **Sans ce retour à la ligne, le passage au corps 18 déborderait.** La plus longue des trois
/// questions posées par `panels::options_modal` — « Fermer l'overlay et installer la version X ? »
/// — demande environ 356 px à ce corps, pour 344 disponibles.
pub const CONFIRM_TEXT_WIDTH: f32 = 344.0;

/// Interligne de la question — 23 px, mesuré de centre à centre entre les deux lignes de la
/// capture. C'est plus que l'interligne naturel d'Ubuntu au corps 18 (≈ 21 px), d'où un réglage
/// explicite plutôt que la valeur par défaut d'`egui`.
pub const CONFIRM_LINE_HEIGHT: f32 = 23.0;

/// Marge latérale de la rangée de boutons — 38 px de chaque côté.
///
/// **Centrage strict, là où la capture ne l'est pas** : elle donne 40 px à gauche et 36 à droite,
/// un écart de 2 px qui tient au rendu du texte et non à une règle de mise en page — la deuxième
/// ligne de la question est décalée du même côté et de la même quantité. 38 = (420 - 344) / 2,
/// où 344 est la largeur du groupe.
pub const CONFIRM_BUTTONS_INSET: f32 = 38.0;

/// Hauteur de cette rangée, depuis le haut du corps — **82 px mesurés** (y = 118 sur la capture
/// détourée, dont le corps commence à 36).
pub const CONFIRM_BUTTONS_TOP: f32 = 82.0;

/// Hauteur des deux boutons — 36 px mesurés, soit exactement [`ButtonSize::Compact`].
///
/// [`ButtonSize::Compact`]: crate::design::ButtonSize::Compact
pub const CONFIRM_BUTTON_HEIGHT: f32 = 36.0;

/// Largeur de chacun — **168 px mesurés**, et la même pour les deux.
///
/// Le relevé de référence du skill `ui-blueprint` les donnait à 166 et 159 px, en attribuant
/// l'écart au rendu du libellé : c'était une mesure prise sur le remplissage, liseré exclu.
pub const CONFIRM_BUTTON_WIDTH: f32 = 168.0;

/// Gouttière entre les deux — **8 px mesurés** (x = 208..216 sur la capture détourée).
pub const CONFIRM_BUTTON_GAP: f32 = 8.0;

// ---------------------------------------------------------------------------------------------
// `design::scrim` — voile modal.
// ---------------------------------------------------------------------------------------------

/// Opacité du voile posé sur ce qu'une fenêtre modale interrompt.
///
/// **Le voile n'est pas une teinte, c'est une information** : tant que la fenêtre est ouverte, ce
/// qu'il couvre est inerte. C'est pourquoi le composant le peint sur le rectangle que l'appelant
/// lui donne — la FENÊTRE entière, pied de page compris — et non sur son seul panneau.
///
/// Né `CONFIRM_SCRIM_ALPHA` avec la boîte de confirmation (2026-09-12), estimé à l'œil sur la
/// capture `interface-confirm-box.png` du jeu (pas de mesure : le voile y couvre une scène dont on
/// ne connaît pas la luminosité d'origine). Renommé le 2026-09-17 quand le voile est devenu un
/// composant à part entière, partagé par quatre appelants.
pub const SCRIM_ALPHA: u8 = 0x88;

// ---------------------------------------------------------------------------------------------
// `design::label` — libellé d'une ligne, élidé.
// ---------------------------------------------------------------------------------------------

/// Corps par défaut d'un libellé — 13 px, celui des noms d'objet sous un emplacement.
///
/// **Choix de mise en page, pas une mesure du jeu** : le client n'écrit jamais le nom d'un objet
/// sous sa case d'inventaire, il n'y a donc aucune capture à relever pour ce cas précis. 13 px est
/// le corps qui laisse un nom d'objet courant tenir dans une tuile de 118 px.
pub const LABEL_FONT_SIZE: f32 = 13.0;

/// Couleur par défaut d'un libellé — blanc, comme tout texte de corps du jeu.
pub const LABEL_TEXT: Color32 = Color32::WHITE;

// -------------------------------------------------------------------------------------------
// Tuile à légende — `design::legend_tile` (2026-09-13)
//
// Le cadre est **celui des champs de saisie** : bordure chaude de 2 px et fond sombre, la
// signature « cadre » du design system, mesurée sur `input-text-width-placeholder.png` (voir
// `INPUT_BORDER`/`INPUT_FILL`). Les cotes propres à la tuile (hauteur, corps de la légende, retrait
// de la légende) sont des **choix de maquette validés par l'utilisateur le 2026-09-13**
// (`crates/overlay-testkit/examples/chat-mockups.rs`, planche « recherches en tuiles »), pas des
// mesures du jeu : le client n'a pas de cadre à légende, c'est un idiome de formulaire emprunté.
// -------------------------------------------------------------------------------------------

/// Bordure de la tuile — la bordure des champs de saisie, mesurée.
pub const LEGEND_TILE_BORDER: Color32 = INPUT_BORDER;
/// Épaisseur de la bordure — celle des champs, 2 px.
pub const LEGEND_TILE_BORDER_WIDTH: f32 = INPUT_BORDER_WIDTH;
/// Fond de la tuile — le fond des champs de saisie, mesuré. Pas d'arrondi : les cadres du jeu ont
/// les coins droits (`docs/design-system.md` §4).
pub const LEGEND_TILE_FILL: Color32 = INPUT_FILL;
/// Hauteur par défaut de la tuile, légende exclue — choix de maquette : une légende sur la
/// bordure, un mot au centre, de l'air.
pub const LEGEND_TILE_HEIGHT: f32 = 54.0;
/// Corps de la légende — petit, c'est une étiquette, pas le contenu. Choix de maquette.
pub const LEGEND_TILE_LEGEND_FONT_SIZE: f32 = 11.0;
/// Couleur de la légende — le gris des titres de section du jeu (`#b8b9ba`, mesuré sur quatre
/// captures, voir `HEADING_TEXT`).
pub const LEGEND_TILE_LEGEND_TEXT: Color32 = HEADING_TEXT;
/// Retrait de la légende depuis le bord gauche de la tuile. Choix de maquette.
pub const LEGEND_TILE_LEGEND_INSET: f32 = 10.0;
/// Respiration entre la légende et la bordure qui s'interrompt de chaque côté. Choix de maquette.
pub const LEGEND_TILE_LEGEND_GAP: f32 = 4.0;
/// Corps du contenu — celui du corps de texte des onglets (`panels::alerts_tab::BODY_FONT_SIZE`).
pub const LEGEND_TILE_FONT_SIZE: f32 = 15.0;
/// Couleur du contenu — blanc, comme tout texte de corps du jeu.
pub const LEGEND_TILE_TEXT: Color32 = Color32::WHITE;
/// Marge horizontale du contenu, de chaque côté : ce qui reste est la largeur utile avant ellipse.
pub const LEGEND_TILE_PAD_X: f32 = 10.0;
/// Voile d'une tuile survolée — le même noir à 40 % que les tuiles d'Alertes et du Suivi
/// (`panels::alerts_tab::TILE_HOVER_SCRIM`).
pub const LEGEND_TILE_HOVER_SCRIM: Color32 = Color32::from_black_alpha(0x66);
/// Retrait de la case à cocher du mode sélection depuis le coin **haut-droit du cadre** — le coin
/// de la croix de retrait, qu'elle remplace (2026-09-16).
///
/// 6 px, et non les 7 d'[`ITEM_SLOT_SELECTION_INSET`] : l'emplacement d'objet doit loger sa case à
/// l'intérieur d'un liseré de rareté qui occupe les pixels 2 à 4 du bord, une tuile à légende n'a
/// qu'un trait d'un pixel. La case y tombe à la même distance visible du cadre.
pub const LEGEND_TILE_SELECTION_INSET: f32 = 6.0;

// -------------------------------------------------------------------------------------------
// Couleurs des canaux de chat
//
// Source : la légende des canaux du client Wakfu (douze lignes « /l - Proximité », « /w - Privé »,
// …, chaque libellé écrit dans la couleur de son canal), capture fournie par l'utilisateur le
// 2026-09-14. Cinq valeurs sont **celles qu'il a données** le même jour (Proximité, Groupe,
// Guilde, Communauté, Recrutement) ; Commerce est un relevé à l'œil sur la capture, non versée au
// dépôt. Seuls les six canaux que le moteur reconnaît (`overlay_engine::CHAT_CHANNELS`) sont
// portés — Privé, Information, Politique, Camp JcJ et Discussion ANKAMA n'existent pas côté parser.
// -------------------------------------------------------------------------------------------

/// « /l - Proximité » — gris clair.
pub const CHAT_CHANNEL_PROXIMITE: Color32 = Color32::from_rgb(0xc1, 0xbf, 0xb8);
/// « /p - Groupe » — violet profond.
pub const CHAT_CHANNEL_GROUPE: Color32 = Color32::from_rgb(0x6a, 0x35, 0x8b);
/// « /g - Guilde » — brun cuivré.
pub const CHAT_CHANNEL_GUILDE: Color32 = Color32::from_rgb(0x94, 0x63, 0x47);
/// « /r - Recrutement (FR) » — rouge framboise.
pub const CHAT_CHANNEL_RECRUTEMENT: Color32 = Color32::from_rgb(0xc3, 0x01, 0x4e);
/// « /m - Commerce » — orange.
pub const CHAT_CHANNEL_COMMERCE: Color32 = Color32::from_rgb(0xe9, 0x96, 0x2f);
/// « /c - Communauté (FR) » — bleu roi.
pub const CHAT_CHANNEL_COMMUNAUTE: Color32 = Color32::from_rgb(0x34, 0x5a, 0xb1);

/// La couleur d'un canal, telle que le client l'écrit dans sa légende.
pub fn chat_channel_color(channel: ChatChannel) -> Color32 {
    match channel {
        ChatChannel::Proximite => CHAT_CHANNEL_PROXIMITE,
        ChatChannel::Groupe => CHAT_CHANNEL_GROUPE,
        ChatChannel::Guilde => CHAT_CHANNEL_GUILDE,
        ChatChannel::Recrutement => CHAT_CHANNEL_RECRUTEMENT,
        ChatChannel::Commerce => CHAT_CHANNEL_COMMERCE,
        ChatChannel::Communaute => CHAT_CHANNEL_COMMUNAUTE,
    }
}
