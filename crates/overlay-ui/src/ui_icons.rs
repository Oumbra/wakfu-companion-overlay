//! Icônes d'interface embarquées — ce qui reste de textures chargées à la main dans ce crate.
//!
//! **Deux** familles, toutes hors du design system :
//!
//! - **Switch Alliés/Ennemis** du panneau Combat (`header-allies.png`/`header-enemies.png`) : les
//!   mêmes fichiers que le dépôt web, hash de cache retiré du nom (inutile sans navigateur).
//! - **Portrait de repli pour un ennemi** (`unknown-entity.png`) : un ennemi n'a jamais de classe
//!   résolue (`breed` non déterministe côté ennemi, voir `overlay_engine::class_breed`), donc
//!   jamais de portrait de `class-avatars-sheet.png` — voir `portraits.rs`.
//!
//! Chargées une fois par fenêtre overlay, même logique que `PortraitAtlas`.
//!
//! **Retrait 2026-09-11 (fin de la migration `design::item_slot`).** Ce module portait aussi les
//! sept emplacements d'objet par rareté (`assets/items/Border-<RARETÉ>.webp`), chargés ici ET au
//! manifeste depuis que le composant existe — **7,3 Mo décodés payés deux fois**, ce que son
//! commit signalait à résorber dès que les deux maquettes du testkit seraient migrées. Elles le
//! sont : plus personne n'appelait `item_border`. Les fichiers, eux, ne bougent pas — le manifeste
//! les référence toujours dans le crate (`ds_crate_asset!`).
//!
//! **Retrait 2026-09-10 (migration des boutons icône).** Ce module portait aussi le socle et les
//! quatre glyphes des boutons icône de l'overlay, plus la machinerie qui allait avec : deux copies
//! recolorées par glyphe (`#c5cbcc` au repos, `#f4d89f` survolé) et `normalize_icon_content`, qui
//! recadrait chaque glyphe sur sa boîte d'encre pour compenser les marges transparentes inégales
//! des fichiers sources. Tout cela vit désormais dans le design system : les teintes sont des
//! jetons (`design::tokens::ICON_TINT`/`ICON_TINT_HOVER`, appliqués au moment de peindre plutôt
//! qu'en dupliquant les textures), le recadrage est fait en amont par le skill `design-asset` et
//! l'étalon de taille est au manifeste (`DsTexture::icon_content_size`). Les six PNG
//! correspondants ont été supprimés d'`assets/ui/` — deux d'entre eux étaient d'ailleurs, octet
//! pour octet, ceux d'`assets/design-system/`.

const ALLIES_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/header-allies.png");
const ENEMIES_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/header-enemies.png");
const UNKNOWN_ENTITY_BYTES: &[u8] = include_bytes!("../assets/ui/unknown-entity.png");

pub struct UiIcons {
    allies: egui::TextureHandle,
    enemies: egui::TextureHandle,
    unknown_entity: egui::TextureHandle,
}

impl UiIcons {
    pub fn load(ctx: &egui::Context) -> Self {
        Self {
            allies: load_texture(ctx, "icon-header-allies", ALLIES_ICON_BYTES),
            enemies: load_texture(ctx, "icon-header-enemies", ENEMIES_ICON_BYTES),
            unknown_entity: load_texture(ctx, "icon-unknown-entity", UNKNOWN_ENTITY_BYTES),
        }
    }

    pub fn allies(&self) -> &egui::TextureHandle {
        &self.allies
    }

    pub fn enemies(&self) -> &egui::TextureHandle {
        &self.enemies
    }

    /// Portrait de repli pour un ennemi (jamais de classe, voir doc de module) — dessiné à la même
    /// taille que les vrais portraits de classe (`portraits::PORTRAIT_SIZE`) pour un alignement
    /// identique dans la liste, même si l'asset source est beaucoup plus petit (upscale visible,
    /// acceptable en attendant un vrai jeu d'illustrations d'ennemis — retour utilisateur
    /// 2026-09-01 : « on affinera graphiquement plus tard »).
    pub fn unknown_entity_image(&self) -> egui::Image<'_> {
        egui::Image::new(&self.unknown_entity)
            .fit_to_exact_size(egui::vec2(
                crate::portraits::PORTRAIT_SIZE,
                crate::portraits::PORTRAIT_SIZE,
            ))
            .maintain_aspect_ratio(false)
    }

    /// Texture brute du repli générique — pour un appelant qui a besoin d'une taille différente de
    /// `PORTRAIT_SIZE` (voir `unknown_entity_image`), ex. les tuiles du panneau Suivi
    /// (`panels::watchlist`), plus petites que les portraits du panneau Combat.
    pub fn unknown_entity_texture(&self) -> &egui::TextureHandle {
        &self.unknown_entity
    }
}

fn load_texture(ctx: &egui::Context, name: &'static str, bytes: &[u8]) -> egui::TextureHandle {
    let decoded = decode(bytes);
    load_rgba(ctx, name, decoded.dimensions(), decoded.as_raw())
}

fn decode(bytes: &[u8]) -> image::RgbaImage {
    image::load_from_memory(bytes)
        .expect("icône UI embarquée invalide — asset corrompu au build")
        .to_rgba8()
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
