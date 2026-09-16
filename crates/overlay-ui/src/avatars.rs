//! Bustes de classe — un PNG par classe et par sexe (`crates/overlay-ui/assets/avatars/{classe}-{f|m}.png`,
//! 80×80, détourés sur fond transparent), ce que l'onglet « Personnages » de la fenêtre Options
//! peint dans ses tuiles et dans sa grille de choix de classe.
//!
//! **À ne pas confondre avec [`crate::portraits`]** : ce sont deux jeux d'images différents pour
//! deux usages différents — 48×48 en médaillon rond pour le panneau Combat là-bas, 80×80 détourés
//! en pied ici. Ils partagent en revanche leur mécanique, et le module voisin en porte la doc :
//! chaque buste est chargé DEUX fois, en couleur et en gris précalculé (`portraits::to_grayscale`,
//! luminance Rec. 601, alpha préservé), parce qu'un `tint()` egui multiplie la couleur sans
//! désaturer — il assombrit un buste coloré, il ne le rend jamais gris. C'est ce qui permet la
//! grille de classes « tout gris, coloré sous le curseur » de la modale de personnage.
//!
//! **Chargé à la demande, jamais au démarrage** (voir `panels::personnages_tab`) : ces 36 textures
//! ne servent qu'à une fenêtre de réglages, et les payer au lancement retarderait l'overlay pour
//! un écran que l'utilisateur n'ouvrira peut-être pas de la session. 1,8 Mo de textures une fois
//! les deux teintes chargées (36 × 2 × 80 × 80 × 4 octets), sur un budget de 300 Mo.

use std::collections::HashMap;

use overlay_engine::Gender;

use crate::portraits::{load_rgba, to_grayscale};

/// Côté natif des fichiers, **auquel ils sont toujours peints** : la leçon de `portraits.rs`,
/// refondu en textures indépendantes pour cette seule raison — un buste rééchantillonné perd son
/// détourage aux bords.
pub const AVATAR_SIZE: f32 = 80.0;

/// `include_bytes!` exige un chemin littéral : même contrainte et même garde-fou que
/// `portraits::CLASS_PROFILE_ASSETS` (un test compare la liste à `CLASS_PORTRAIT_ORDER`).
macro_rules! avatar_asset {
    ($class:literal) => {
        (
            $class,
            include_bytes!(concat!("../assets/avatars/", $class, "-f.png")) as &[u8],
            include_bytes!(concat!("../assets/avatars/", $class, "-m.png")) as &[u8],
        )
    };
}

const AVATAR_ASSETS: &[(&str, &[u8], &[u8])] = &[
    avatar_asset!("cra"),
    avatar_asset!("ecaflip"),
    avatar_asset!("sacrier"),
    avatar_asset!("zobal"),
    avatar_asset!("sram"),
    avatar_asset!("ouginak"),
    avatar_asset!("iop"),
    avatar_asset!("eniripsa"),
    avatar_asset!("feca"),
    avatar_asset!("sadida"),
    avatar_asset!("enutrof"),
    avatar_asset!("eliotrope"),
    avatar_asset!("foggernaut"),
    avatar_asset!("huppermage"),
    avatar_asset!("xelor"),
    avatar_asset!("osamodas"),
    avatar_asset!("pandawa"),
    avatar_asset!("rogue"),
];

/// Une texture par sexe — `HashMap<&'static str, _>` plutôt qu'une clé-tuple, pour la raison
/// détaillée dans `portraits::GenderPair` (pas d'impl `Borrow` pour un tuple contenant un `&str`).
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

pub struct AvatarAtlas {
    color: HashMap<&'static str, GenderPair>,
    grey: HashMap<&'static str, GenderPair>,
}

impl AvatarAtlas {
    /// Décode et charge les 72 textures (18 classes × 2 sexes × 2 teintes). Panique si un asset
    /// embarqué est corrompu : c'est un bug de build, pas un cas runtime à tolérer — même
    /// arbitrage que `portraits::PortraitAtlas::load`.
    pub fn load(ctx: &egui::Context) -> Self {
        let mut color = HashMap::with_capacity(AVATAR_ASSETS.len());
        let mut grey = HashMap::with_capacity(AVATAR_ASSETS.len());
        for (class_name, f_bytes, m_bytes) in AVATAR_ASSETS {
            let (color_f, grey_f) = load_avatar(ctx, class_name, Gender::F, f_bytes);
            let (color_m, grey_m) = load_avatar(ctx, class_name, Gender::M, m_bytes);
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

    /// Texture pour la classe/sexe donnés — `None` si `class_name` n'est pas une classe connue
    /// (voir `CLASS_PORTRAIT_ORDER`). L'appelant peint alors le portrait générique d'`UiIcons`
    /// plutôt qu'un trou : un personnage déclaré depuis le site avec une classe inconnue de cette
    /// version de l'overlay doit rester visible dans sa liste.
    pub fn texture(
        &self,
        class_name: &str,
        gender: Gender,
        grey: bool,
    ) -> Option<&egui::TextureHandle> {
        let map = if grey { &self.grey } else { &self.color };
        map.get(class_name).map(|pair| pair.get(gender))
    }
}

fn load_avatar(
    ctx: &egui::Context,
    class_name: &str,
    gender: Gender,
    bytes: &[u8],
) -> (egui::TextureHandle, egui::TextureHandle) {
    let decoded = image::load_from_memory(bytes)
        .expect("buste de classe embarqué invalide — asset corrompu au build")
        .to_rgba8();
    let name = format!("class-avatar-{class_name}-{gender:?}");
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

#[cfg(test)]
mod tests {
    use super::*;
    use overlay_engine::class_breed::CLASS_PORTRAIT_ORDER;

    /// Même garde-fou que `portraits` : une classe ajoutée d'un côté et oubliée ici sortirait une
    /// tuile sans buste, sans que rien ne le signale au build.
    #[test]
    fn les_bustes_couvrent_toutes_les_classes() {
        let noms: Vec<&str> = AVATAR_ASSETS.iter().map(|(name, ..)| *name).collect();
        assert_eq!(noms, CLASS_PORTRAIT_ORDER);
    }
}
