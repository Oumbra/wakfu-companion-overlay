//! Portraits de classe — un fichier PNG par classe et par sexe (`assets/class-profile/
//! {classe}-{f|m}.png`, 48×48, cercle inscrit avec les 4 coins transparents), copie de
//! `wakfu-companion-overlay/assets/class-profile/` (voir `docs/plan-architecture.md` : ces mêmes
//! fichiers servent aussi de source à `crate::combat_frame`, qui compose les portraits DANS les
//! médaillons du cadre décoratif — voir sa doc de module).
//!
//! **Refonte 2026-09-03** (demande utilisateur, redesign du panneau Combat) : remplace la planche
//! unique `class-avatars-sheet.png` (2 colonnes × 18 lignes, un seul rectangle UV par classe/sexe)
//! par 36 textures indépendantes — nécessaire pour `crate::combat_frame`, qui a besoin de coller
//! un portrait NATIF (48×48, non re-échantillonné) dans chaque médaillon d'un cadre déjà
//! redimensionné à l'identique (voir `assets/_test/README.md` du dépôt pour la validation de cet
//! ajustement) ; un rectangle UV sur la planche partagée aurait imposé le filtrage `LINEAR` de
//! toute la planche aux bords de CHAQUE portrait, visible en zoom. Chaque portrait est chargé deux
//! fois (couleur + niveaux de gris précalculé, voir `to_grayscale`) : un combattant KO (`FighterDamage::
//! is_ko`, voir `overlay_engine::session`) est affiché grisé dans la liste comme dans le cadre —
//! demande explicite. Le gris est calculé une fois au chargement (luminance Rec. 601, alpha
//! préservé), jamais par un `tint()` egui à l'affichage : un tint multiplie la couleur d'origine
//! sans désaturer, il assombrit un portrait coloré sans jamais le rendre gris (essayé, rejeté).

use std::collections::HashMap;

use overlay_engine::{class_breed::CLASS_PORTRAIT_ORDER, Gender};

/// Taille (carrée, points egui) à laquelle un portrait est dessiné dans la liste "plate" du
/// panneau Combat (repli sans cadre : ennemis, ou alliés au-delà des 6 médaillons du cadre — voir
/// `crate::combat_frame::MAX_FRAME_SLOTS`). Le cadre, lui, dessine ses portraits à leur taille
/// NATIVE (48×48, voir `crate::combat_frame`) : ne pas confondre les deux tailles.
pub const PORTRAIT_SIZE: f32 = 40.0;

/// Taille native des fichiers `class-profile/*.png` — voir `crate::combat_frame::SLOT_DIAMETER`,
/// mesuré pour correspondre exactement à cette taille une fois le cadre redimensionné.
pub const NATIVE_PORTRAIT_SIZE: f32 = 48.0;

/// `include_bytes!` exige un chemin littéral au moment de la compilation : impossible de dériver
/// dynamiquement cette liste depuis `CLASS_PORTRAIT_ORDER` (voir `overlay_engine::class_breed`,
/// même 18 classes, même orthographe) malgré la redondance — un test de ce module vérifie que les
/// deux restent synchronisées plutôt que de risquer une classe oubliée en silence.
macro_rules! class_profile_asset {
    ($class:literal) => {
        (
            $class,
            include_bytes!(concat!("../assets/class-profile/", $class, "-f.png")) as &[u8],
            include_bytes!(concat!("../assets/class-profile/", $class, "-m.png")) as &[u8],
        )
    };
}

const CLASS_PROFILE_ASSETS: &[(&str, &[u8], &[u8])] = &[
    class_profile_asset!("cra"),
    class_profile_asset!("ecaflip"),
    class_profile_asset!("sacrier"),
    class_profile_asset!("zobal"),
    class_profile_asset!("sram"),
    class_profile_asset!("ouginak"),
    class_profile_asset!("iop"),
    class_profile_asset!("eniripsa"),
    class_profile_asset!("feca"),
    class_profile_asset!("sadida"),
    class_profile_asset!("enutrof"),
    class_profile_asset!("eliotrope"),
    class_profile_asset!("foggernaut"),
    class_profile_asset!("huppermage"),
    class_profile_asset!("xelor"),
    class_profile_asset!("osamodas"),
    class_profile_asset!("pandawa"),
    class_profile_asset!("rogue"),
];

/// Une texture par sexe — `HashMap<&'static str, GenderPair>` plutôt que `HashMap<(&'static str,
/// Gender), _>` : une clé-tuple `(&'static str, Gender)` interrogée avec un `class_name: &'a str`
/// de durée de vie plus courte ne trouve pas d'impl `Borrow` adéquate (les tuples n'ont pas
/// l'équivalent de `String: Borrow<str>`) — contourné en sortant `Gender` de la clé.
struct GenderPair {
    f: egui::TextureHandle,
    m: egui::TextureHandle,
}

impl GenderPair {
    fn get(&self, gender: Gender) -> &egui::TextureHandle {
        match gender {
            Gender::F => &self.f,
            Gender::M => &self.m,
        }
    }
}

pub struct PortraitAtlas {
    color: HashMap<&'static str, GenderPair>,
    /// Version grisée précalculée de chaque portrait — voir la doc de module pour pourquoi ce
    /// n'est pas un simple `tint()` à l'affichage.
    grey: HashMap<&'static str, GenderPair>,
}

impl PortraitAtlas {
    /// Décode et charge les 36 assets embarqués (18 classes × 2 sexes), une texture couleur et une
    /// texture grisée par portrait — appelé une seule fois, dès qu'un `egui::Context` existe (voir
    /// `main.rs::resumed`). Panique si un asset embarqué est corrompu : c'est un bug de build, pas
    /// un cas runtime à tolérer (contrairement à un flux réseau, voir `RemoteIconStore`).
    pub fn load(ctx: &egui::Context) -> Self {
        let mut color = HashMap::with_capacity(CLASS_PROFILE_ASSETS.len());
        let mut grey = HashMap::with_capacity(CLASS_PROFILE_ASSETS.len());
        for (class_name, f_bytes, m_bytes) in CLASS_PROFILE_ASSETS {
            let (color_f, grey_f) = load_portrait(ctx, class_name, Gender::F, f_bytes);
            let (color_m, grey_m) = load_portrait(ctx, class_name, Gender::M, m_bytes);
            color.insert(
                *class_name,
                GenderPair {
                    f: color_f,
                    m: color_m,
                },
            );
            grey.insert(
                *class_name,
                GenderPair {
                    f: grey_f,
                    m: grey_m,
                },
            );
        }
        Self { color, grey }
    }

    /// Texture brute pour la classe/sexe donnés (couleur, ou grisée si `ko`) — `None` si
    /// `class_name` ne correspond à aucune classe connue (voir `CLASS_PORTRAIT_ORDER` :
    /// l'appelant ne dessine alors rien plutôt qu'un portrait trompeur, même choix que l'ancienne
    /// planche partagée). Utilisé directement par `crate::combat_frame`, qui a besoin de la
    /// texture pour la peindre à une taille et une position précises (`Painter::image`), pas d'un
    /// widget `egui::Image` centré par le layout comme `image` ci-dessous.
    pub fn texture(
        &self,
        class_name: &str,
        gender: Gender,
        ko: bool,
    ) -> Option<&egui::TextureHandle> {
        if !CLASS_PORTRAIT_ORDER.contains(&class_name) {
            return None;
        }
        let map = if ko { &self.grey } else { &self.color };
        map.get(class_name).map(|pair| pair.get(gender))
    }

    /// Widget prêt à `ui.add(...)` pour la liste "plate" (voir `PORTRAIT_SIZE`) — `None` même cas
    /// que `texture`.
    pub fn image(&self, class_name: &str, gender: Gender, ko: bool) -> Option<egui::Image<'_>> {
        let texture = self.texture(class_name, gender, ko)?;
        Some(
            egui::Image::new(texture)
                .fit_to_exact_size(egui::vec2(PORTRAIT_SIZE, PORTRAIT_SIZE))
                .maintain_aspect_ratio(false),
        )
    }
}

/// Décode `bytes` et charge ses deux textures (couleur + grisée) — factorisé entre les deux sexes
/// de `PortraitAtlas::load`.
fn load_portrait(
    ctx: &egui::Context,
    class_name: &str,
    gender: Gender,
    bytes: &[u8],
) -> (egui::TextureHandle, egui::TextureHandle) {
    let decoded = image::load_from_memory(bytes)
        .expect("portrait de classe embarqué invalide — asset corrompu au build")
        .to_rgba8();
    let name = format!("class-portrait-{class_name}-{gender:?}");
    let color = load_rgba(ctx, &name, decoded.dimensions(), decoded.as_raw());
    let greyscale = to_grayscale(&decoded);
    let grey = load_rgba(
        ctx,
        &format!("{name}-gris"),
        greyscale.dimensions(),
        greyscale.as_raw(),
    );
    (color, grey)
}

fn load_rgba(
    ctx: &egui::Context,
    name: &str,
    (width, height): (u32, u32),
    raw: &[u8],
) -> egui::TextureHandle {
    let color_image =
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], raw);
    ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR)
}

/// Luminance Rec. 601 (mêmes coefficients que `image::imageops::grayscale`, appliqués ici à la
/// main pour garder le canal alpha d'origine — `grayscale` de la crate `image` renvoie une
/// `GrayImage` sans alpha, qui perdrait la transparence des 4 coins du portrait).
fn to_grayscale(decoded: &image::RgbaImage) -> image::RgbaImage {
    let mut out = decoded.clone();
    for pixel in out.pixels_mut() {
        let [r, g, b, _a] = pixel.0;
        let luma = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32)
            .round()
            .clamp(0.0, 255.0) as u8;
        pixel.0[0] = luma;
        pixel.0[1] = luma;
        pixel.0[2] = luma;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Garde-fou contre une classe ajoutée d'un côté (`class_breed::CLASS_PORTRAIT_ORDER`) et
    /// oubliée de l'autre (`CLASS_PROFILE_ASSETS`, qui ne peut pas être dérivée automatiquement —
    /// voir sa doc). Compare aussi l'ORDRE : sans intérêt fonctionnel ici (les deux sont indexées
    /// par nom, pas par position), mais un ordre qui diverge est le signe le plus probable d'un
    /// copier-coller fautif lors d'un futur ajout de classe.
    #[test]
    fn class_profile_assets_synchronise_avec_class_portrait_order() {
        let asset_names: Vec<&str> = CLASS_PROFILE_ASSETS
            .iter()
            .map(|(name, ..)| *name)
            .collect();
        assert_eq!(asset_names, CLASS_PORTRAIT_ORDER);
    }

    #[test]
    fn to_grayscale_preserve_alpha_et_neutralise_la_teinte() {
        let mut img = image::RgbaImage::new(1, 1);
        img.put_pixel(0, 0, image::Rgba([200, 40, 40, 137]));
        let grey = to_grayscale(&img);
        let px = grey.get_pixel(0, 0);
        assert_eq!(px.0[0], px.0[1]);
        assert_eq!(px.0[1], px.0[2]);
        assert_eq!(px.0[3], 137, "alpha d'origine préservé");
    }
}
