//! Icônes d'interface embarquées — switch Alliés/Ennemis du panneau Combat (mêmes fichiers PNG
//! que le dépôt web, `public/assets/ui/header-allies-*.png`/`header-enemies-*.png` — hash de
//! cache retiré du nom de fichier, inutile ici puisqu'il n'y a pas de navigateur) et portrait de
//! repli pour un ennemi (`unknown-entity-*.png`) : un ennemi n'a jamais de classe résolue (`breed`
//! pas déterministe côté ennemi, voir `overlay_engine::class_breed`), donc jamais de portrait de
//! la planche `class-avatars-sheet.png` — voir `portraits.rs`. Chargées une fois par fenêtre
//! overlay, même logique que `PortraitAtlas`.

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
}

fn load_texture(ctx: &egui::Context, name: &'static str, bytes: &[u8]) -> egui::TextureHandle {
    let decoded = image::load_from_memory(bytes)
        .expect("icône UI embarquée invalide — asset corrompu au build")
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        decoded.as_raw(),
    );
    ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR)
}
