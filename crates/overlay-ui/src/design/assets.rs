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
        DsTexture::CheckboxChecked,
        DsTexture::CheckboxUnchecked,
        DsTexture::SelectFace,
        DsTexture::IconChevronDown,
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
        }
    }
}
