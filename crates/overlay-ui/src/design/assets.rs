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
//! Les marges 9-slice sont **mesurées**, pas choisies :
//! `component.py insets assets/design-system/button-*.png` rend pour les sept textures de bouton un
//! arrondi de 3–4px et un liseré de 2px, d'où un minimum géométrique de 6px — la valeur retenue,
//! identique sur les sept (voir `BUTTON_SLICE`).

use crate::design::nine_slice::{Fill, Insets, NineSlice};

/// Découpage commun à **toutes** les textures de bouton texte du design system.
///
/// - `insets` 6px : `max(rayon, liseré) + 2` de sécurité, mesuré identique sur les sept fichiers
///   (`button-{primary,secondary,danger}[-hover].png` et `button-disabled.png` : arrondi 3–4px,
///   liseré 2px). Une valeur unique plutôt qu'une par fichier — les textures viennent du même
///   composant du jeu, une divergence de 1px serait du bruit de mesure, pas une intention de design.
/// - `fill_x = Tile` : les hachures diagonales sont horizontalement périodiques ; les étirer
///   transforme les croisillons en traînées dès qu'un bouton dépasse sa largeur native (planche de
///   contrôle du skill `ui-component`, comparaison 500×48 étiré / répété).
/// - `fill_y = Stretch` : le dégradé vertical (clair en haut, sombre en bas) DOIT suivre la hauteur
///   du bouton ; le répéter empilerait deux dégradés.
pub const BUTTON_SLICE: NineSlice = NineSlice::new(Insets::same(6.0), Fill::Tile, Fill::Stretch);

/// Une texture du design system, désignée par son rôle et non par son chemin.
///
/// Les variantes `*Hover` sont des **fichiers distincts capturés dans le jeu**, pas un
/// éclaircissement calculé : un `tint` egui ne peut que multiplier la couleur d'origine, jamais
/// l'éclaircir au-delà (même limite déjà documentée dans `ui_icons::load_texture_recolored_pair`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DsTexture {
    ButtonPrimary,
    ButtonPrimaryHover,
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
        DsTexture::ButtonSecondary,
        DsTexture::ButtonSecondaryHover,
        DsTexture::ButtonDanger,
        DsTexture::ButtonDangerHover,
        DsTexture::ButtonDisabled,
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
                slice: BUTTON_SLICE,
            },
            DsTexture::ButtonDangerHover => DsTextureSpec {
                name: "ds-button-danger-hover",
                bytes: ds_asset!("button-danger-hover.png"),
                slice: BUTTON_SLICE,
            },
            DsTexture::ButtonDisabled => DsTextureSpec {
                name: "ds-button-disabled",
                bytes: ds_asset!("button-disabled.png"),
                slice: BUTTON_SLICE,
            },
        }
    }
}
