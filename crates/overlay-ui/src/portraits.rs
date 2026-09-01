//! Portraits de classe (planche `class-avatars-sheet.png`, copie de
//! `class-avatars-sheet-ade44514.png` du dépôt web — voir `assets/avatars/`, même planche que
//! `ClassPortraitComponent`/`class-portraits.data.ts` : 2 colonnes [féminin, masculin] × 18
//! lignes). Décodée une seule fois au démarrage (`include_bytes!`, jamais de lecture disque au
//! runtime — même logique que le repli catalogue prévu en L3) et chargée en texture GPU via egui.

use overlay_engine::{class_breed, Gender};

const SHEET_BYTES: &[u8] = include_bytes!("../assets/avatars/class-avatars-sheet.png");
const SPRITE_COLS: f32 = 2.0;
const SPRITE_ROWS: f32 = 18.0;

/// Taille (carrée, points egui) à laquelle un portrait est dessiné dans le panneau Combat.
/// 22px d'origine confirmé illisible en test réel (2026-09-01, retour utilisateur avec capture
/// d'écran) — passé à 40px. Le réglage visuel fin (mise en page de la ligne combattant, largeur de
/// fenêtre) reste à faire, voir panels/combat.rs — ce n'est qu'une taille lisible, pas la version
/// définitive.
pub const PORTRAIT_SIZE: f32 = 40.0;

pub struct PortraitAtlas {
    texture: egui::TextureHandle,
}

impl PortraitAtlas {
    /// Décode l'asset embarqué et le charge en texture — appelé une seule fois, dès qu'un
    /// `egui::Context` existe (voir `main.rs::resumed`). Panique si l'asset embarqué est corrompu :
    /// c'est un bug de build, pas un cas runtime à tolérer (contrairement à un flux réseau).
    pub fn load(ctx: &egui::Context) -> Self {
        let decoded = image::load_from_memory(SHEET_BYTES)
            .expect("planche de portraits embarquée invalide — asset corrompu au build")
            .to_rgba8();
        let (width, height) = decoded.dimensions();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [width as usize, height as usize],
            decoded.as_raw(),
        );
        let texture =
            ctx.load_texture("class-portraits", color_image, egui::TextureOptions::LINEAR);
        Self { texture }
    }

    /// Widget prêt à `ui.add(...)` pour la classe/sexe donnés — `None` si `class_name` ne
    /// correspond à aucune classe connue (voir `class_breed::class_portrait_row`) : l'appelant ne
    /// dessine alors rien plutôt qu'un portrait trompeur (repli 1ère case côté web, volontairement
    /// pas reproduit ici).
    pub fn image(&self, class_name: &str, gender: Gender) -> Option<egui::Image<'_>> {
        let row = class_breed::class_portrait_row(class_name)? as f32;
        let col = if gender == Gender::F { 0.0 } else { 1.0 };
        let uv = egui::Rect::from_min_size(
            egui::pos2(col / SPRITE_COLS, row / SPRITE_ROWS),
            egui::vec2(1.0 / SPRITE_COLS, 1.0 / SPRITE_ROWS),
        );
        Some(
            egui::Image::new(&self.texture)
                .uv(uv)
                .fit_to_exact_size(egui::vec2(PORTRAIT_SIZE, PORTRAIT_SIZE))
                // Sans ça, `ImageSize::calc_size` (egui 0.36.1, widgets/image.rs) applique quand
                // même `scale_to_fit` avec le ratio d'aspect de la planche ENTIÈRE (200×1800 px,
                // pas le rectangle après `.uv(...)`, qu'egui ignore pour ce calcul) : un carré
                // 40×40 demandé devenait ~4×40 (bug réel constaté en session, 2026-09-01, capture
                // d'écran à l'appui — un fin trait vertical coloré, pas un portrait).
                .maintain_aspect_ratio(false),
        )
    }
}
