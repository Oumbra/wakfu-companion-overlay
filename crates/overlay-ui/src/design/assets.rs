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
    ];

    pub(crate) fn index(self) -> usize {
        DsTexture::ALL
            .iter()
            .position(|t| *t == self)
            .expect("toute variante de DsTexture est listée dans ALL")
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
        }
    }
}
