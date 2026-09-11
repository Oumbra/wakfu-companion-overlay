//! **Manifeste des textures du design system** : la table qui relie un nom logique
//! (`DsTexture::ButtonPrimary`) à un fichier de `assets/design-system/` et à son découpage 9-slice.
//!
//! Ajouter une texture au design system = ajouter **une variante d'énumération et une ligne de
//! `spec`**, rien d'autre. C'est volontairement le seul endroit du crate où un chemin d'asset est
//! écrit : un composant ne connaît que des `DsTexture`, jamais un `include_bytes!`.
//!
//! **Les fichiers sont référencés directement dans `assets/design-system/`**, à la racine du dépôt,
//! pas recopiés dans `crates/overlay-ui/assets/` comme les icônes historiques (`ui_icons`). Ces
//! dernières venaient d'ailleurs (dossier `Pictures` de l'utilisateur) et n'existaient nulle part
//! dans le dépôt ; `assets/design-system/` est au contraire **la** source du design system, tenue à
//! jour par le skill `design-asset`. Une copie dans le crate se serait désynchronisée au premier
//! retraitement d'asset. Le crate n'est pas publié (`publish = false` dans le workspace) : un
//! `include_bytes!` qui sort du dossier du crate ne pose donc aucun problème d'empaquetage.
//!
//! Les marges 9-slice sont **mesurées**, pas choisies (`component.py insets`) — et ce sont les
//! **hachures qui les dimensionnent**, pas les coins : voir `button_slice`.

use crate::design::nine_slice::{Fill, Insets, NineSlice};
use crate::design::tokens;

/// Découpage d'une texture de bouton texte : `cap` pixels figés à gauche et à droite, 6px en haut
/// et en bas, tout le reste étiré.
///
/// - **`cap` = étendue du décor**, pas le rayon des coins. Retour utilisateur 2026-09-09 : « il faut
///   que les hachures soient présentes uniquement sur les côtés [...] et que sur le fond central, ce
///   soit sans hachure ». Les croisillons diagonaux ne sont pas une texture de fond mais un
///   **embout** : `component.py insets` les trouve cantonnés aux 43–51 premiers pixels de chaque
///   extrémité sur les cinq textures 200×52/169×52, et aux 29–30 premiers sur les deux textures
///   338×36 — au-delà, l'écart au dégradé retombe au niveau du bruit (1,0 à 1,6 contre un pic de
///   3,6 à 7,5). Les marges retenues (52 et 32) arrondissent la plus grande mesure de chaque famille
///   vers le haut : une marge trop courte laisse un bout de croisillon dans la bande médiane, où il
///   serait étiré sur toute la longueur du bouton.
/// - **6px en haut et en bas** : `max(rayon, liseré) + 2` de sécurité, mesuré identique sur les sept
///   fichiers (arrondi 3–4px, liseré 2px). Le décor ne déborde pas verticalement, rien n'oblige à
///   figer davantage — et moins on fige, mieux le dégradé suit la hauteur.
/// - **`Fill::Stretch` sur les deux axes** : la bande médiane est désormais un dégradé lisse dans
///   les deux directions, il n'y a plus rien de périodique à répéter.
const fn button_slice(cap: f32) -> NineSlice {
    NineSlice::new(
        Insets {
            left: cap,
            top: 6.0,
            right: cap,
            bottom: 6.0,
        },
        Fill::Stretch,
        Fill::Stretch,
    )
}

/// Boutons de fenêtre (textures 200×52 et 169×52) — décor mesuré jusqu'à 51px des bords.
pub const BUTTON_SLICE: NineSlice = button_slice(52.0);

/// Boutons de pied de page de modale (textures 338×36) — décor mesuré jusqu'à 34px des bords
/// (29-30px sur les deux textures rouges, 31-34px sur les deux textures or, arrondi à 36 pour
/// couvrir toute la famille). Un embout de 52px y couvrirait près du tiers d'un bouton pourtant
/// conçu pour être long.
pub const BUTTON_SLICE_COMPACT: NineSlice = button_slice(36.0);

/// Découpage d'une **icône** : aucune marge figée, mise à l'échelle uniforme sur les deux axes.
///
/// Ce n'est pas un 9-slice dégénéré par paresse — c'est ce qu'une icône demande. Figer des coins
/// sur un glyphe de 27 px le déformerait dès qu'on le peint à 12, et il n'y a rien de périodique à
/// répéter. Passer par `DesignSystem::paint` malgré tout garde un seul chemin de peinture pour
/// toutes les textures du manifeste.
pub const ICON_SLICE: NineSlice = NineSlice::new(Insets::same(0.0), Fill::Stretch, Fill::Stretch);

/// Découpage d'un onglet : **4px figés sur les quatre côtés**, tout le reste étiré.
///
/// Pas de long embout ici, contrairement aux boutons : un onglet n'a pas de hachures d'extrémité,
/// juste un bord sombre de 2px doublé d'un liseré. 4px = ce bord plus deux pixels de sécurité, la
/// même règle que le haut et le bas d'un bouton.
///
/// `Fill::Stretch` sur les deux axes. En Y, la question ne se pose pas : un onglet est toujours
/// peint à sa hauteur native de 44px. En X, le décor du corps est un motif très sourd, dont
/// l'étirement se voit moins qu'un raccord de tuilage au milieu d'un onglet.
pub const TAB_SLICE: NineSlice = NineSlice::new(Insets::same(4.0), Fill::Stretch, Fill::Stretch);

/// Découpage d'un onglet d'**extrémité de barre** : **8px figés du côté arrondi**, 4 des trois
/// autres.
///
/// L'arrondi appartient aux deux bouts de la barre, pas à chaque onglet — et il est porté par
/// l'alpha de la texture, comme celui d'un bouton, parce qu'un `Mesh` egui ne sait pas découper un
/// coin. 8 = les 2px de bord + les 4px d'escalier d'alpha + 2 de sécurité, la même règle que
/// [`SELECT_SLICE`]. Le côté intérieur reste à 4 : il est droit, comme celui d'un onglet du milieu.
const fn tab_end_slice(left: f32, right: f32) -> NineSlice {
    NineSlice::new(
        Insets {
            left,
            top: 4.0,
            right,
            bottom: 4.0,
        },
        Fill::Stretch,
        Fill::Stretch,
    )
}

/// Premier onglet de la barre — arrondi à gauche.
pub const TAB_SLICE_FIRST: NineSlice = tab_end_slice(8.0, 4.0);

/// Dernier onglet de la barre — arrondi à droite.
pub const TAB_SLICE_LAST: NineSlice = tab_end_slice(4.0, 8.0);

/// Découpage d'une case à cocher : **5px figés sur les quatre côtés**.
///
/// Mesuré : 2px de fond sombre puis 2px de cadre, plus un pixel de sécurité. Une case est toujours
/// peinte à sa taille native de 20px — ce découpage n'existe donc que pour honorer la règle « toute
/// taille est valide » sans déformer le cadre si un panneau en demandait une autre.
pub const CHECKBOX_SLICE: NineSlice =
    NineSlice::new(Insets::same(5.0), Fill::Stretch, Fill::Stretch);

/// Découpage du socle d'une liste déroulante : **8px figés à gauche et à droite, 6 en haut et en
/// bas**.
///
/// 8 en horizontal : les deux pixels de bord, plus le coin arrondi, plus une marge. C'est aussi
/// exactement la largeur des colonnes que la génération de `select-face.png` a conservées telles
/// quelles — au-delà, la texture est une seule colonne propre répétée, un étirement n'y perd donc
/// rien. 6 en vertical : le bord, le liseré et deux pixels de sécurité, comme sur un bouton.
pub const SELECT_SLICE: NineSlice = NineSlice::new(
    Insets {
        left: 8.0,
        top: 6.0,
        right: 8.0,
        bottom: 6.0,
    },
    Fill::Stretch,
    Fill::Stretch,
);

/// Découpage du socle d'un bouton icône : **6px figés sur les quatre côtés**, comme le haut et le
/// bas d'un bouton texte.
///
/// Un socle carré de 36px n'a pas d'embout décoratif à préserver — juste un bord et un liseré. En
/// pratique il est presque toujours peint à sa taille native, ce découpage n'existe donc que pour
/// honorer la règle « toute taille est valide ».
pub const ICON_BUTTON_SLICE: NineSlice =
    NineSlice::new(Insets::same(6.0), Fill::Stretch, Fill::Stretch);

/// Découpage du corps de modale (`modal-body.png`, 720 × 505) : **125px figés à gauche, 195 à
/// droite, 110 en haut, 180 en bas**.
///
/// Quatre valeurs distinctes, contrairement à tout le reste du manifeste, parce que le décor de
/// cette texture est franchement asymétrique — et c'est encore lui qui dimensionne, comme sur un
/// bouton (`button_slice`). Les hachures de la fenêtre Options ne sont pas une trame de fond mais
/// un **cadre** : denses dans les angles et le long des bords, absentes de la bande centrale.
/// Chaque marge est la plus petite qui contienne 90 % du décor de son côté, arrondie au multiple
/// de 5 supérieur (`tools/design-system/build_modal_body.py`, sortie `marges du décor`) ; le bas en
/// demande plus que le haut parce que le pourtour des deux boutons de pied de page y concentre les
/// croisillons les plus marqués.
///
/// Les deux totaux (320 en X, 290 en Y) laissent une vraie bande médiane à la taille où la modale
/// est peinte aujourd'hui (560 × 380) : c'est elle qui absorbe le changement de dimensions, sans
/// toucher au décor.
///
/// `Fill::Stretch` sur les deux axes : cette bande médiane est un fond quasi uni — moins de deux
/// niveaux d'écart d'un bord à l'autre — il n'y a rien de périodique à répéter.
/// Cadre d'un bloc repliable (`collapse-frame.png`, 722 × 28) — **marges figées sur le décor
/// d'angle**, comme pour un bouton.
///
/// 20 px à gauche et à droite : l'étendue du motif en équerre qui marque les quatre angles, mesurée
/// sur le profil du coin haut-gauche. 13 px en haut et en bas : la hauteur des deux bandes
/// prélevées sur la capture, liseré compris.
///
/// `Stretch` sur les deux axes — entre les angles, le cadre n'est qu'un liseré d'un pixel sur un
/// fond uni, il n'y a rien de périodique à répéter.
pub const COLLAPSE_FRAME_SLICE: NineSlice = NineSlice::new(
    Insets {
        left: 20.0,
        top: 13.0,
        right: 20.0,
        bottom: 13.0,
    },
    Fill::Stretch,
    Fill::Stretch,
);

/// Socle d'un bouton de pas (`button-stepper.png`, 32 × 32) — **aucun décor à figer**.
///
/// Mesuré (`component.py insets`) : rayon d'angle 3, aucun liseré, bruit de décor 0,05 sur 255 —
/// c'est un aplat, contrairement aux socles de bouton icône qui portent un liseré kaki. Six pixels
/// de marge, soit deux fois le rayon, suffisent donc à figer les angles ; vérifié sur une planche
/// 9-slice à 24 × 24, 32 × 32, 48 × 32 et 64 × 32 avant d'écrire la moindre ligne de Rust.
pub const STEPPER_SLICE: NineSlice =
    NineSlice::new(Insets::same(6.0), Fill::Stretch, Fill::Stretch);

/// Bannière de la modale Options (`modal-header.png`, 720 × 56) — **aucune marge figée**.
///
/// Son motif de losanges est réparti sur toute la largeur : ce ne sont pas des embouts d'extrémité
/// comme sur un bouton, il n'y a donc rien à figer. Étirer la texture entière est ce que le rendu
/// fait depuis l'origine, et la comparaison au jeu l'a validé — le rayon de coin natif (≈ 12 px sur
/// 720) reste imperceptible à l'échelle où la bannière est peinte.
///
/// Ce découpage n'est de toute façon pas emprunté aujourd'hui : `panels::options_modal` peint la
/// bannière par `egui::Image` pour lui appliquer un arrondi de coins HAUTS, ce qu'un `Mesh`
/// 9-slice ne sait pas faire. Il est déclaré pour que la texture entre au manifeste comme les
/// autres, et il sera juste le jour où quelqu'un l'empruntera.
///
/// Même valeur qu'[`ICON_SLICE`] à ce jour, et ce n'est pas un oubli : les deux disent des choses
/// différentes — un glyphe n'a rien à figer parce qu'il n'a pas de décor, cette bannière parce que
/// le sien est réparti. Le jour où l'un des deux gagne une marge mesurée, l'autre ne doit pas
/// bouger avec lui.
pub const BANNER_SLICE: NineSlice = NineSlice::new(Insets::same(0.0), Fill::Stretch, Fill::Stretch);

pub const MODAL_BODY_SLICE: NineSlice = NineSlice::new(
    Insets {
        left: 125.0,
        top: 110.0,
        right: 195.0,
        bottom: 180.0,
    },
    Fill::Stretch,
    Fill::Stretch,
);

/// Découpage du panneau de contenu (`modal-section.png`, 688 × 375) : **105px figés à gauche,
/// 175 à droite, 50 en haut, 120 en bas**.
///
/// Même règle que [`MODAL_BODY_SLICE`], appliquée au même décor : le cadre de hachures de la
/// fenêtre traverse le panneau et se concentre dans ses quatre angles. Les marges sont les plus
/// petites qui contiennent 90 % du décor de chaque côté
/// (`tools/design-system/build_modal_section.py`). Elles sont plus courtes que celles du corps,
/// parce que le panneau ne voit qu'une partie du cadre — celle qui tombe dans son rectangle.
///
/// Totaux 280 en X et 170 en Y : le panneau est peint à 520 × 253 dans la modale actuelle, la
/// bande médiane existe donc sur les deux axes. Et c'est un fond encore plus plat que celui du
/// corps — moins de deux niveaux d'un bord à l'autre — donc rien à répéter, `Fill::Stretch`.
pub const MODAL_SECTION_SLICE: NineSlice = NineSlice::new(
    Insets {
        left: 105.0,
        top: 50.0,
        right: 175.0,
        bottom: 120.0,
    },
    Fill::Stretch,
    Fill::Stretch,
);

/// Une texture du design system, désignée par son rôle et non par son chemin.
///
/// Les variantes `*Hover` sont des **fichiers distincts capturés dans le jeu**, pas un
/// éclaircissement calculé : un `tint` egui ne peut que multiplier la couleur d'origine, jamais
/// l'éclaircir au-delà (même limite déjà documentée dans `ui_icons::load_texture_recolored_pair`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DsTexture {
    ButtonPrimary,
    ButtonPrimaryHover,
    /// Or en 338×36 — **même variante que `ButtonPrimary`, autre hauteur native**. Les deux
    /// existent parce que le décor d'extrémité n'a pas la même largeur selon la hauteur du bouton
    /// capturé (52px sur le bouton de fenêtre HDV, 34px sur celui de pied de page) : rendre le
    /// bouton de pied de page avec la texture 200×52 lui donnerait un embout une fois et demie
    /// trop large, ce qui se voit immédiatement à côté du bouton rouge voisin. Voir
    /// `components::button::ButtonVariant::texture` pour la règle de choix.
    ButtonPrimaryCompact,
    ButtonPrimaryCompactHover,
    ButtonSecondary,
    ButtonSecondaryHover,
    ButtonDanger,
    ButtonDangerHover,
    /// Bouton grisé — **une seule texture pour les trois variantes**. §5.1 du design-system :
    /// l'état désactivé est une désaturation complète en niveaux de gris, « aucune trace de la
    /// teinte or » ; la capture disponible est celle du bouton primaire désactivé, et rien
    /// n'indique que le jeu en ait une par variante. Réutilisée telle quelle plutôt que
    /// d'inventer deux fichiers.
    ButtonDisabled,
    /// Pastille « i » du bloc d'information (`icons/icon-info.png`, 27 × 28).
    ///
    /// **La seule icône du manifeste, et la seule texture qui n'y soit pas un 9-slice** : un
    /// glyphe n'a ni embout ni bande médiane, ses marges figées sont donc nulles et il est
    /// simplement mis à l'échelle. Blanche dans le fichier, elle prend sa couleur par teinte —
    /// c'est ce qui permet au ton `Alert` de `design::info_text` d'exister sans second fichier.
    IconInfo,
    /// Onglet ACTIF — et survolé, les deux partageant exactement le même fond (voir
    /// `design::components::tabs`). Découpé du premier segment de
    /// `tabs-with-first-tab-active.png`, coin gauche redressé : ce coin arrondi appartient à
    /// l'extrémité de la BARRE, pas à l'onglet, et les segments du milieu sont bien droits.
    TabActive,
    /// Onglet inactif — et désactivé, dont seul le libellé change. Découpé du segment du MILIEU du
    /// même fichier, le seul dont les deux coins sont droits.
    TabInactive,
    /// Onglet actif en PREMIÈRE position — les deux coins gauches arrondis.
    ///
    /// Découpé du premier segment de `tabs-with-first-tab-active.png` **sans redresser son coin**,
    /// contrairement à [`DsTexture::TabActive`] : c'est la même capture, gardée telle quelle. Un
    /// seul pixel opaque isolé a été retiré de l'angle bas-gauche, résidu du détourage d'origine.
    TabActiveFirst,
    /// Onglet actif en DERNIÈRE position — miroir horizontal de [`DsTexture::TabActiveFirst`].
    ///
    /// Le jeu n'a pas de capture d'onglet actif en fin de barre. Le miroir est légitime ici : le
    /// corps d'un onglet est un dégradé **vertical**, ses deux bords sont identiques, et c'est déjà
    /// la technique qui avait servi à redresser le coin de `tab-active.png`.
    TabActiveLast,
    /// Onglet inactif en DERNIÈRE position — les deux coins droits arrondis. Découpé du **dernier**
    /// segment de la même capture, le seul onglet inactif dont un côté soit arrondi.
    TabInactiveLast,
    /// Onglet inactif en PREMIÈRE position — miroir horizontal de [`DsTexture::TabInactiveLast`].
    TabInactiveFirst,
    /// Case à cocher COCHÉE (`checkbox-true.png`, 20 × 20) — cadre doré vif, carré blanc plein.
    CheckboxChecked,
    /// Case à cocher DÉCOCHÉE (`checkbox-false.png`, 20 × 20) — cadre kaki sombre, intérieur noir.
    CheckboxUnchecked,
    /// Socle d'une liste déroulante (`select-face.png`, 220 × 36) — dégradé, liseré haut, ombre
    /// basse et bords, **libellé et chevron retirés** : ce sont le composant et le manifeste qui
    /// les remettent, l'un depuis l'appelant, l'autre depuis `IconChevronDown`.
    SelectFace,
    /// Chevron `⌄` d'une liste déroulante (`icons/icon-chevron-down.png`, 14 × 8) — la taille à
    /// laquelle le jeu le peint, au pixel près.
    IconChevronDown,
    /// Socle d'un bouton icône posé DANS un panneau (`button-icon.png`, 36 × 36).
    ButtonIcon,
    ButtonIconHover,
    /// Socle d'un bouton icône posé par-dessus le jeu (`button-icon-first-plan.png`, 36 × 36) —
    /// octet pour octet le `button-background.png` que `ui_icons` charge déjà de son côté.
    ButtonIconFirstPlan,
    ButtonIconFirstPlanHover,
    /// Socle grisé — **une seule texture pour les deux contextes**, le jeu n'en ayant capturé
    /// qu'une (même parti pris que `ButtonDisabled` pour le bouton texte).
    ButtonIconDisabled,
    /// Glyphes des boutons icône de l'overlay. La table ne porte que les icônes réellement
    /// utilisées : `assets/design-system/icons/` en compte plus de trente, toutes chargées sur le
    /// GPU dès le premier composant peint (voir `DesignSystem::load`), et il n'y a aucune raison de
    /// payer celles que personne n'affiche.
    IconOption,
    IconExternalLink,
    IconPlus,
    IconMinus,
    /// Loupe d'un champ de recherche (`icons/icon-search.png`, 24 × 24) — le jeu la pose À
    /// L'INTÉRIEUR du champ, collée au bord gauche, jamais sur un socle de bouton
    /// (`interface-hdv-achat.png` x 27..39, `interface-personnage-equiement.png`). Elle n'a donc
    /// pas de `icon_content_size` : sa taille est celle que lui donne le champ qui la porte.
    IconSearch,
    /// Croix de fermeture/retrait (`icons/icon-close.png`, 13 × 14) — le « × » de « Retirer tous
    /// les filtres » et le bouton de fermeture d'une fenêtre.
    IconClose,
    /// Corbeille (`icons/icon-delete.png`, 12 × 14) — la suppression d'un élément d'une liste, sur
    /// socle de bouton icône (`interface-personnage-equiement.png`, barre d'outils du build).
    IconDelete,
    /// Point d'interrogation (`icons/icon-help.png`, 12 × 12) — le bouton d'aide en tête de
    /// fenêtre (`interface-personnage-equiement.png`, coin haut-droit).
    IconHelp,
    /// Coche (`icons/icon-tick.png`, 12 × 9) — le marqueur « actif » du jeu, à côté d'un libellé
    /// (« ✓ Actif ») plutôt que sur un socle : pas de `icon_content_size` pour la même raison que
    /// [`DsTexture::IconSearch`].
    IconTick,
    /// Flèche de réinitialisation (`icons/icon-undo.png`, 14 × 12) — le bouton « rétablir les
    /// valeurs par défaut » de la fenêtre Options (`interface-options-son.png`, coin haut-droit).
    IconUndo,
    /// Haut-parleur (`icons/icon-volume.png`, 26 × 22) — glyphe de volume actif, détouré le
    /// 2026-09-11 (skill `design-asset`) depuis une capture du jeu sans socle porteur : le glyphe
    /// est directement posé sur le décor, contrairement aux icônes prélevées sur un bouton.
    ///
    /// **Pas encore d'appelant** : aucun réglage de son n'est câblé dans l'overlay à ce jour —
    /// `alert_sound` ne fait que jouer les sons, jamais les couper. Entrée au manifeste comme
    /// [`DsTexture::ModalHeader`] avant sa bannière : préparée pour le futur bouton muet/actif de
    /// la fenêtre Options (`interface-options-son.png`), posée sur un socle de bouton icône —
    /// d'où `icon_content_size`, comme les glyphes de la même famille.
    IconVolume,
    /// Haut-parleur barré (`icons/icon-volume-mute.png`, 26 × 26) — pendant coupé de
    /// [`DsTexture::IconVolume`], même provenance et même statut (pas encore d'appelant).
    IconVolumeMute,
    /// Œil ouvert (`icons/icon-eye.png`, 16 × 14) — glyphe SOMBRE, détouré le 2026-09-11 (skill
    /// `design-asset`, `--polarity dark --keep center --floor 45`) depuis un crop du jeu sans
    /// socle porteur, contrairement aux icônes prélevées sur un bouton.
    ///
    /// **Pas encore d'appelant** : aucun bascule affiché/masqué n'existe dans l'overlay à ce jour.
    /// Entrée au manifeste comme [`DsTexture::IconVolume`] avant son futur bouton — préparée pour
    /// un toggle de visibilité (candidat naturel : masquer une entrée du panneau Suivi, ou un champ
    /// de jeton dans la fenêtre Options). Posée sur un socle de bouton icône, d'où
    /// `icon_content_size`.
    IconEye,
    /// Œil barré (`icons/icon-eye-off.png`, 16 × 14) — pendant coupé de [`DsTexture::IconEye`],
    /// même provenance et même statut (pas encore d'appelant).
    IconEyeOff,
    /// Corps de la modale Options (`modal-body.png`, 720 × 505) — le fond SOUS la bannière,
    /// découpé des six captures de la fenêtre Options du jeu puis débarrassé de son contenu
    /// (`tools/design-system/build_modal_body.py`, §9 ter du design-system).
    ///
    /// **Les deux angles inférieurs sont portés par l'alpha de la texture** (arrondi de rayon 12,
    /// le même que les angles hauts de `modal-header.png`), comme pour un bouton : un `Mesh` egui
    /// ne sait pas découper un coin.
    ///
    /// Remplace l'aplat `MODAL_BG` qui peignait ce fond jusqu'ici. La translucidité de la fenêtre,
    /// elle, ne vient plus de la couleur mais de la teinte passée à la peinture — voir
    /// `panels::options_modal`.
    ModalBody,
    /// Panneau de contenu de la modale Options (`modal-section.png`, 688 × 375) — l'encadré qui
    /// tient les sections, découpé des mêmes six captures que [`DsTexture::ModalBody`]
    /// (`tools/design-system/build_modal_section.py`, §9 quater du design-system).
    ///
    /// **Ce n'est pas une surface à part** : c'est le fond de la fenêtre assombri de huit à neuf
    /// niveaux (`#1E2126` → `#15191C`), que le décor de la fenêtre traverse — les hachures s'y
    /// retrouvent, aux quatre angles. La texture porte ce fond, son liseré de 2px et l'arrondi de
    /// rayon 6 de ses angles, celui-ci dans son alpha comme pour un bouton.
    ///
    /// Remplace l'aplat `SECTION_BG` + `rect_stroke` qui le peignaient jusqu'ici.
    ModalSection,
    /// Bannière de la modale Options (`modal-header.png`, 720 × 56) — le bandeau turquoise qui
    /// porte le titre de la fenêtre, au-dessus de [`DsTexture::ModalBody`], avec lequel elle se
    /// juxtapose sans recouvrement (elle s'arrête où l'autre commence).
    ///
    /// **Ses deux angles hauts sont portés par l'alpha de la texture** (rayon 12), comme les angles
    /// bas de `ModalBody`.
    ///
    /// Entrée au manifeste le 2026-09-10 (lot 0.1 de `docs/plan-composants-ui.md`). Le fichier
    /// existait jusque-là **en double** — `crates/overlay-ui/assets/ui/options/modal-header.png`
    /// en était une copie octet pour octet, chargée par un `include_bytes!` local. C'était la
    /// dernière raison d'exister de `OptionsModalAssets`, de sa fonction de décodage et du champ
    /// `options_assets` que `RenderContent` promenait jusqu'au panneau.
    ModalHeader,
    /// Socle d'un bouton de pas (`button-stepper.png`, 32 × 32) — le carré sombre qui porte le
    /// « − » et le « + » d'un pas numérique (`design::stepper`).
    ///
    /// **Ce n'est pas le socle d'un bouton icône** : le jeu en a deux familles distinctes, et
    /// celle-ci est un aplat gris-bleu sans liseré (`#2b2d33`), plus discrète, faite pour être
    /// encastrée à côté d'un champ — là où `button-icon.png` porte un liseré kaki et vit dans un
    /// panneau.
    ///
    /// **Générifiée le 2026-09-10** depuis `button-moins.png`, qui portait son glyphe incrusté —
    /// exactement l'« asset par libellé » que le skill `ui-component` interdit. Reconstruction par
    /// diffusion (`dsimg.py genericize --method diffusion`) : résidu maximal de 4/255 dans la zone
    /// du glyphe, imperceptible. `button-plus.png` en diffère de 70 pixels, tous dans son glyphe :
    /// c'est bien un seul socle pour les deux boutons.
    ///
    /// Les glyphes, eux, viennent du manifeste ([`DsTexture::IconPlus`], [`DsTexture::IconMinus`])
    /// et sont posés par `design::icon_button` — un socle, deux glyphes, jamais deux textures.
    ButtonStepper,
    /// Cadre d'un bloc repliable (`collapse-frame.png`, 722 × 28) — le décor de
    /// [`design::collapsible`](crate::design::collapsible), ses quatre angles et son liseré.
    ///
    /// **Assemblée le 2026-09-11 à partir de `collapse-block-opened.png`**, faute d'asset détouré :
    /// treize lignes prélevées en haut du cadre (au-dessus de son en-tête), deux lignes de fond uni
    /// prises entre deux blocs de contenu, treize lignes prélevées en bas (sous le dernier texte).
    /// Les bandes sont ensuite nettoyées — hors des angles et du liseré, tout est ramené au fond uni
    /// `#1f2227`.
    ///
    /// Ce nettoyage n'est pas cosmétique : **le cadre du jeu est translucide**, et la capture avait
    /// donc figé le décor du jeu vu au travers. Le garder aurait collé un fragment de paysage dans
    /// tout bloc repliable de l'overlay.
    ///
    /// **Ce qui n'est PAS reproduit** : cette translucidité elle-même. Il faudrait deux captures du
    /// même cadre sur deux fonds différents pour en déduire l'alpha, comme
    /// `tools/design-system/build_modal_body.py` le fait pour la modale. Le cadre est donc opaque —
    /// écart assumé, et le seul de cet asset.
    CollapseFrame,
}

/// Description statique d'une texture : nom de cache egui, octets PNG embarqués, découpage.
pub struct DsTextureSpec {
    pub name: &'static str,
    pub bytes: &'static [u8],
    pub slice: NineSlice,
}

macro_rules! ds_asset {
    ($file:literal) => {
        include_bytes!(concat!("../../../../assets/design-system/", $file))
    };
}

impl DsTexture {
    /// Toutes les textures du manifeste — l'ordre fixe l'index de stockage dans
    /// `super::DesignSystem`, il n'a pas d'autre sens.
    pub const ALL: &'static [DsTexture] = &[
        DsTexture::ButtonPrimary,
        DsTexture::ButtonPrimaryHover,
        DsTexture::ButtonPrimaryCompact,
        DsTexture::ButtonPrimaryCompactHover,
        DsTexture::ButtonSecondary,
        DsTexture::ButtonSecondaryHover,
        DsTexture::ButtonDanger,
        DsTexture::ButtonDangerHover,
        DsTexture::ButtonDisabled,
        DsTexture::IconInfo,
        DsTexture::TabActive,
        DsTexture::TabInactive,
        DsTexture::TabActiveFirst,
        DsTexture::TabActiveLast,
        DsTexture::TabInactiveFirst,
        DsTexture::TabInactiveLast,
        DsTexture::CheckboxChecked,
        DsTexture::CheckboxUnchecked,
        DsTexture::SelectFace,
        DsTexture::IconChevronDown,
        DsTexture::ButtonIcon,
        DsTexture::ButtonIconHover,
        DsTexture::ButtonIconFirstPlan,
        DsTexture::ButtonIconFirstPlanHover,
        DsTexture::ButtonIconDisabled,
        DsTexture::IconOption,
        DsTexture::IconExternalLink,
        DsTexture::IconPlus,
        DsTexture::IconMinus,
        DsTexture::IconSearch,
        DsTexture::IconClose,
        DsTexture::IconDelete,
        DsTexture::IconHelp,
        DsTexture::IconTick,
        DsTexture::IconUndo,
        DsTexture::IconVolume,
        DsTexture::IconVolumeMute,
        DsTexture::IconEye,
        DsTexture::IconEyeOff,
        DsTexture::ModalBody,
        DsTexture::ModalSection,
        DsTexture::ModalHeader,
        DsTexture::ButtonStepper,
        DsTexture::CollapseFrame,
    ];

    pub(crate) fn index(self) -> usize {
        DsTexture::ALL
            .iter()
            .position(|t| *t == self)
            .expect("toute variante de DsTexture est listée dans ALL")
    }

    /// Plus grande dimension d'encre à laquelle cette texture est peinte quand elle sert d'icône
    /// sur un socle de [`tokens::ICON_BUTTON_SIZE`] — `None` pour tout ce qui n'est pas une icône
    /// de bouton, ou dont la taille est déjà celle du composant qui la porte.
    ///
    /// Les glyphes de `assets/design-system/icons/` sont détourés au pixel près : leur fichier fait
    /// exactement la taille de leur encre, qui varie d'un glyphe à l'autre (13 pour le lien
    /// externe, 16 pour le rouage). Peints tels quels, ils donneraient trois hauteurs d'encre
    /// différentes dans une même barre — le jeu, lui, les cale tous sur une grille commune. Cette
    /// grille est [`tokens::ICON_BUTTON_CONTENT`], mesurée sur le jeu ; sa doc porte la mesure.
    ///
    /// C'est le manifeste qui la porte, et pas l'appelant : la taille d'encre d'un glyphe est une
    /// propriété de l'asset, au même titre que son découpage 9-slice.
    pub fn icon_content_size(self) -> Option<f32> {
        match self {
            DsTexture::IconOption
            | DsTexture::IconExternalLink
            | DsTexture::IconPlus
            | DsTexture::IconMinus
            | DsTexture::IconClose
            | DsTexture::IconDelete
            | DsTexture::IconHelp
            | DsTexture::IconUndo
            | DsTexture::IconVolume
            | DsTexture::IconVolumeMute
            | DsTexture::IconEye
            | DsTexture::IconEyeOff => Some(tokens::ICON_BUTTON_CONTENT),
            _ => None,
        }
    }

    pub fn spec(self) -> DsTextureSpec {
        match self {
            DsTexture::ButtonPrimary => DsTextureSpec {
                name: "ds-button-primary",
                bytes: ds_asset!("button-primary.png"),
                slice: BUTTON_SLICE,
            },
            DsTexture::ButtonPrimaryHover => DsTextureSpec {
                name: "ds-button-primary-hover",
                bytes: ds_asset!("button-primary-hover.png"),
                slice: BUTTON_SLICE,
            },
            DsTexture::ButtonPrimaryCompact => DsTextureSpec {
                name: "ds-button-primary-compact",
                bytes: ds_asset!("button-primary-compact.png"),
                slice: BUTTON_SLICE_COMPACT,
            },
            DsTexture::ButtonPrimaryCompactHover => DsTextureSpec {
                name: "ds-button-primary-compact-hover",
                bytes: ds_asset!("button-primary-compact-hover.png"),
                slice: BUTTON_SLICE_COMPACT,
            },
            DsTexture::ButtonSecondary => DsTextureSpec {
                name: "ds-button-secondary",
                bytes: ds_asset!("button-secondary.png"),
                slice: BUTTON_SLICE,
            },
            DsTexture::ButtonSecondaryHover => DsTextureSpec {
                name: "ds-button-secondary-hover",
                bytes: ds_asset!("button-secondary-hover.png"),
                slice: BUTTON_SLICE,
            },
            DsTexture::ButtonDanger => DsTextureSpec {
                name: "ds-button-danger",
                bytes: ds_asset!("button-danger.png"),
                slice: BUTTON_SLICE_COMPACT,
            },
            DsTexture::ButtonDangerHover => DsTextureSpec {
                name: "ds-button-danger-hover",
                bytes: ds_asset!("button-danger-hover.png"),
                slice: BUTTON_SLICE_COMPACT,
            },
            DsTexture::ButtonDisabled => DsTextureSpec {
                name: "ds-button-disabled",
                bytes: ds_asset!("button-disabled.png"),
                slice: BUTTON_SLICE,
            },
            DsTexture::IconInfo => DsTextureSpec {
                name: "ds-icon-info",
                bytes: ds_asset!("icons/icon-info.png"),
                slice: ICON_SLICE,
            },
            DsTexture::TabActive => DsTextureSpec {
                name: "ds-tab-active",
                bytes: ds_asset!("tab-active.png"),
                slice: TAB_SLICE,
            },
            DsTexture::TabInactive => DsTextureSpec {
                name: "ds-tab-inactive",
                bytes: ds_asset!("tab-inactive.png"),
                slice: TAB_SLICE,
            },
            DsTexture::TabActiveFirst => DsTextureSpec {
                name: "ds-tab-active-first",
                bytes: ds_asset!("tab-active-first.png"),
                slice: TAB_SLICE_FIRST,
            },
            DsTexture::TabActiveLast => DsTextureSpec {
                name: "ds-tab-active-last",
                bytes: ds_asset!("tab-active-last.png"),
                slice: TAB_SLICE_LAST,
            },
            DsTexture::TabInactiveFirst => DsTextureSpec {
                name: "ds-tab-inactive-first",
                bytes: ds_asset!("tab-inactive-first.png"),
                slice: TAB_SLICE_FIRST,
            },
            DsTexture::TabInactiveLast => DsTextureSpec {
                name: "ds-tab-inactive-last",
                bytes: ds_asset!("tab-inactive-last.png"),
                slice: TAB_SLICE_LAST,
            },
            DsTexture::CheckboxChecked => DsTextureSpec {
                name: "ds-checkbox-checked",
                bytes: ds_asset!("checkbox-true.png"),
                slice: CHECKBOX_SLICE,
            },
            DsTexture::CheckboxUnchecked => DsTextureSpec {
                name: "ds-checkbox-unchecked",
                bytes: ds_asset!("checkbox-false.png"),
                slice: CHECKBOX_SLICE,
            },
            DsTexture::SelectFace => DsTextureSpec {
                name: "ds-select-face",
                bytes: ds_asset!("select-face.png"),
                slice: SELECT_SLICE,
            },
            DsTexture::IconChevronDown => DsTextureSpec {
                name: "ds-icon-chevron-down",
                bytes: ds_asset!("icons/icon-chevron-down.png"),
                slice: ICON_SLICE,
            },
            DsTexture::ButtonIcon => DsTextureSpec {
                name: "ds-button-icon",
                bytes: ds_asset!("button-icon.png"),
                slice: ICON_BUTTON_SLICE,
            },
            DsTexture::ButtonIconHover => DsTextureSpec {
                name: "ds-button-icon-hover",
                bytes: ds_asset!("button-icon-hover.png"),
                slice: ICON_BUTTON_SLICE,
            },
            DsTexture::ButtonIconFirstPlan => DsTextureSpec {
                name: "ds-button-icon-first-plan",
                bytes: ds_asset!("button-icon-first-plan.png"),
                slice: ICON_BUTTON_SLICE,
            },
            DsTexture::ButtonIconFirstPlanHover => DsTextureSpec {
                name: "ds-button-icon-first-plan-hover",
                bytes: ds_asset!("button-icon-first-plan-hover.png"),
                slice: ICON_BUTTON_SLICE,
            },
            DsTexture::ButtonIconDisabled => DsTextureSpec {
                name: "ds-button-icon-disabled",
                bytes: ds_asset!("button-icon-disabled.png"),
                slice: ICON_BUTTON_SLICE,
            },
            DsTexture::IconOption => DsTextureSpec {
                name: "ds-icon-option",
                bytes: ds_asset!("icons/icon-option.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconExternalLink => DsTextureSpec {
                name: "ds-icon-external-link",
                bytes: ds_asset!("icons/icon-external-link.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconPlus => DsTextureSpec {
                name: "ds-icon-plus",
                bytes: ds_asset!("icons/icon-plus.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconMinus => DsTextureSpec {
                name: "ds-icon-minus",
                bytes: ds_asset!("icons/icon-minus.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconSearch => DsTextureSpec {
                name: "ds-icon-search",
                bytes: ds_asset!("icons/icon-search.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconClose => DsTextureSpec {
                name: "ds-icon-close",
                bytes: ds_asset!("icons/icon-close.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconDelete => DsTextureSpec {
                name: "ds-icon-delete",
                bytes: ds_asset!("icons/icon-delete.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconHelp => DsTextureSpec {
                name: "ds-icon-help",
                bytes: ds_asset!("icons/icon-help.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconTick => DsTextureSpec {
                name: "ds-icon-tick",
                bytes: ds_asset!("icons/icon-tick.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconUndo => DsTextureSpec {
                name: "ds-icon-undo",
                bytes: ds_asset!("icons/icon-undo.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconVolume => DsTextureSpec {
                name: "ds-icon-volume",
                bytes: ds_asset!("icons/icon-volume.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconVolumeMute => DsTextureSpec {
                name: "ds-icon-volume-mute",
                bytes: ds_asset!("icons/icon-volume-mute.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconEye => DsTextureSpec {
                name: "ds-icon-eye",
                bytes: ds_asset!("icons/icon-eye.png"),
                slice: ICON_SLICE,
            },
            DsTexture::IconEyeOff => DsTextureSpec {
                name: "ds-icon-eye-off",
                bytes: ds_asset!("icons/icon-eye-off.png"),
                slice: ICON_SLICE,
            },
            DsTexture::ModalBody => DsTextureSpec {
                name: "ds-modal-body",
                bytes: ds_asset!("modal-body.png"),
                slice: MODAL_BODY_SLICE,
            },
            DsTexture::ModalSection => DsTextureSpec {
                name: "ds-modal-section",
                bytes: ds_asset!("modal-section.png"),
                slice: MODAL_SECTION_SLICE,
            },
            DsTexture::ModalHeader => DsTextureSpec {
                name: "ds-modal-header",
                bytes: ds_asset!("modal-header.png"),
                slice: BANNER_SLICE,
            },
            DsTexture::ButtonStepper => DsTextureSpec {
                name: "ds-button-stepper",
                bytes: ds_asset!("button-stepper.png"),
                slice: STEPPER_SLICE,
            },
            DsTexture::CollapseFrame => DsTextureSpec {
                name: "ds-collapse-frame",
                bytes: ds_asset!("collapse-frame.png"),
                slice: COLLAPSE_FRAME_SLICE,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seuil d'opacité au-delà duquel un pixel compte comme de l'encre — le même que celui du skill
    /// `design-asset`, pour que les mesures se comparent d'un outil à l'autre.
    const ALPHA_THRESHOLD: u8 = 10;

    /// Boîte englobante des pixels opaques : `(largeur, hauteur)`.
    fn ink_bbox(img: &image::RgbaImage) -> (u32, u32) {
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0_u32, 0_u32);
        let mut vu = false;
        for (x, y, pixel) in img.enumerate_pixels() {
            if pixel.0[3] > ALPHA_THRESHOLD {
                vu = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
        assert!(vu, "texture entièrement transparente");
        (max_x - min_x + 1, max_y - min_y + 1)
    }

    fn decode(bytes: &[u8]) -> image::RgbaImage {
        image::load_from_memory(bytes)
            .expect("texture du manifeste décodable")
            .to_rgba8()
    }

    /// **La mesure qui a débloqué la migration, transformée en garde-fou.**
    ///
    /// `tokens::ICON_BUTTON_CONTENT` vaut 18 parce que le jeu cale les icônes de cette famille sur
    /// une grille commune — mesuré sur les huit icônes de `menu-button-icon-first-plan.png`. Tant
    /// que cette valeur n'était qu'un paragraphe de documentation, rien n'empêchait de la changer
    /// « à l'œil ». Ce test la rattache à la capture : retraiter l'asset ou poser un autre étalon
    /// le fait tomber.
    ///
    /// Découpage de la capture : socles de 36 × 36, cadence verticale de 38 (2 px de gouttière),
    /// premier socle en (4, 5). L'échelle 1 est vérifiée séparément — `button-icon-first-plan.png`
    /// s'y recale avec un écart moyen de 2,3/255, contre 4,5 et plus dès 35 ou 37.
    #[test]
    fn l_etalon_d_icone_est_celui_mesure_sur_le_jeu() {
        const SOCLE: u32 = 36;
        const CADENCE: u32 = 38;
        const ORIGINE: (u32, u32) = (4, 5);
        /// Seuil de luminance séparant l'encre claire de l'icône du socle sombre — le résultat ne
        /// bouge pas entre 120 et 180, ce n'est donc pas un réglage critique.
        const SEUIL_LUMINANCE: u32 = 140;

        let barre = decode(ds_asset!("menu-button-icon-first-plan.png"));
        let mut mesures = Vec::new();
        for i in 0..8 {
            let (x0, y0) = (ORIGINE.0, ORIGINE.1 + CADENCE * i);
            let socle = image::imageops::crop_imm(&barre, x0, y0, SOCLE, SOCLE).to_image();
            let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0_u32, 0_u32);
            for (x, y, pixel) in socle.enumerate_pixels() {
                let luminance =
                    (pixel.0[0] as u32 + pixel.0[1] as u32 + pixel.0[2] as u32).div_ceil(3);
                if luminance > SEUIL_LUMINANCE {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
            mesures.push((max_x - min_x + 1).max(max_y - min_y + 1));
        }
        mesures.sort_unstable();

        let mediane = mesures[mesures.len() / 2] as f32;
        assert_eq!(
            mediane,
            tokens::ICON_BUTTON_CONTENT,
            "médiane mesurée {mediane}, étalon {} — mesures : {mesures:?}",
            tokens::ICON_BUTTON_CONTENT,
        );
        let (min, max) = (mesures[0] as f32, mesures[mesures.len() - 1] as f32);
        assert!(
            (16.0..=20.0).contains(&min) && (16.0..=20.0).contains(&max),
            "l'encre du jeu sort de la plage 16–20 : {mesures:?}",
        );
    }

    /// Les glyphes d'icône du manifeste sont **détourés au pixel près** : leur canevas est
    /// exactement leur encre.
    ///
    /// C'est l'hypothèse sur laquelle repose `icon_draw_size` : il ramène la plus grande dimension
    /// du FICHIER à l'étalon. Une marge transparente autour d'un glyphe rétrécirait donc son encre
    /// en silence, d'autant plus que la marge est large — précisément le défaut qu'`ui_icons`
    /// corrigeait à la volée avant la migration du 2026-09-10, ses fichiers sources laissant des
    /// canevas de 18 et 22 px pour des encres de 13 et 16. Ce test l'attrape au retraitement de
    /// l'asset, pas au retour utilisateur.
    ///
    /// **Un pixel de tolérance** sur chaque axe : la frange d'antialiasing d'un détourage peut
    /// tomber sous [`ALPHA_THRESHOLD`] sur la dernière rangée — c'est le cas de
    /// `icon-external-link.png`, dont la dernière ligne plafonne à un alpha de 6. Ce n'est pas une
    /// marge, et ce test vise les marges (plusieurs pixels), pas la frange.
    #[test]
    fn les_glyphes_d_icone_sont_detoures_au_pixel_pres() {
        /// Écart admis entre le canevas et l'encre, par axe — voir la doc de la fonction.
        const TOLERANCE: u32 = 1;
        // **Toutes les textures découpées en `ICON_SLICE`**, et non les seules qui portent un
        // `icon_content_size` : ce filtre-là laissait échapper les glyphes qui ne vivent pas sur
        // un socle (`IconSearch` dans un champ, `IconTick` à côté d'un libellé, `IconChevronDown`
        // sur une liste déroulante) — précisément ceux dont une marge transparente passerait
        // inaperçue, faute d'étalon pour les recadrer. Réserve de la revue du 2026-09-10.
        for texture in DsTexture::ALL.iter().copied() {
            let spec = texture.spec();
            if spec.slice.insets != Insets::same(0.0) {
                continue;
            }
            let img = decode(spec.bytes);
            let encre = ink_bbox(&img);
            let (canevas_x, canevas_y) = img.dimensions();
            assert!(
                canevas_x - encre.0 <= TOLERANCE && canevas_y - encre.1 <= TOLERANCE,
                "{} : canevas {:?}, encre {encre:?} — marge transparente à retirer",
                spec.name,
                img.dimensions(),
            );
        }
    }

    /// Les cinq socles de bouton icône font la taille native que le composant suppose.
    #[test]
    fn les_socles_de_bouton_icone_font_la_taille_native() {
        for texture in [
            DsTexture::ButtonIcon,
            DsTexture::ButtonIconHover,
            DsTexture::ButtonIconFirstPlan,
            DsTexture::ButtonIconFirstPlanHover,
            DsTexture::ButtonIconDisabled,
        ] {
            let spec = texture.spec();
            let img = decode(spec.bytes);
            let attendu = tokens::ICON_BUTTON_SIZE as u32;
            assert_eq!(
                img.dimensions(),
                (attendu, attendu),
                "{} : {:?} au lieu de {attendu} × {attendu}",
                spec.name,
                img.dimensions(),
            );
        }
    }
}
