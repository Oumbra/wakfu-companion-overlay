//! Les huit gemmes de rareté du jeu, pour les maquettes.
//!
//! **Ce ne sont pas des assets du design system.** Ce sont des *fixtures du harnais* : au runtime,
//! l'overlay les télécharge du CDN `wakassets` par `RemoteIconStore`, exactement comme une icône
//! d'objet — voir [`overlay_engine::IconKind::Rarity`]. Le harnais, lui, n'a pas de réseau, et une
//! planche qui montre huit trous ne juge rien. D'où ces huit PNG de 13 × 20 sous
//! `crates/overlay-testkit/fixtures/rarities/`, dépouillés de leurs métadonnées (18 Ko d'XMP et de
//! profil ICC chacun à la source, ~650 octets une fois réduits aux seuls IHDR/IDAT/IEND).
//!
//! **Ne jamais les déplacer sous `assets/design-system/`** : ils y deviendraient des textures
//! embarquées dans le binaire, ce que la décision « les gemmes sont distantes » écarte précisément.
//!
//! Le fichier est choisi par `IconRef::for_rarity(rarité).gfx_id`, pas par une seconde table écrite
//! ici : c'est la MÊME correspondance rareté → numéro que celle qui construira l'URL au runtime,
//! donc une planche fausse voudrait dire que le code de production l'est aussi.

use std::collections::HashMap;

use egui::{Context, Rect, TextureHandle, TextureOptions, Vec2};
use overlay_engine::{IconRef, WakfuRarity};

/// Taille native d'une gemme — mesurée sur les fichiers du CDN, identique pour les huit.
pub const GEM_NATIVE: Vec2 = Vec2::new(13.0, 20.0);

/// Côté de la boîte où la gemme se pose, dans une rangée de suggestion. **14, comme le web**
/// (`.wakfu-autocomplete-item-rarity`, 14 × 14 en `object-fit: contain`) : une image 13 × 20 y
/// entre donc en 9,1 × 14, limitée par la hauteur. Jamais un `Vec2::splat` sur la boîte, qui
/// écraserait la gemme en carré.
pub const GEM_BOX: f32 = 14.0;

/// Les huit gemmes, chargées une fois.
pub struct RarityGems {
    par_numero: HashMap<String, TextureHandle>,
}

impl RarityGems {
    pub fn load(ctx: &Context) -> Self {
        const FIXTURES: &[(&str, &[u8])] = &[
            ("0", include_bytes!("../../fixtures/rarities/0.png")),
            ("1", include_bytes!("../../fixtures/rarities/1.png")),
            ("2", include_bytes!("../../fixtures/rarities/2.png")),
            ("3", include_bytes!("../../fixtures/rarities/3.png")),
            ("4", include_bytes!("../../fixtures/rarities/4.png")),
            ("5", include_bytes!("../../fixtures/rarities/5.png")),
            ("6", include_bytes!("../../fixtures/rarities/6.png")),
            ("7", include_bytes!("../../fixtures/rarities/7.png")),
        ];
        let par_numero = FIXTURES
            .iter()
            .map(|(numero, bytes)| {
                let image = image::load_from_memory(bytes)
                    .expect("gemme de rareté décodable")
                    .to_rgba8();
                let taille = [image.width() as usize, image.height() as usize];
                let texture = ctx.load_texture(
                    format!("maquette.gemme-{numero}"),
                    egui::ColorImage::from_rgba_unmultiplied(taille, image.as_raw()),
                    TextureOptions::LINEAR,
                );
                ((*numero).to_string(), texture)
            })
            .collect();
        Self { par_numero }
    }

    /// Peint la gemme d'une rareté, centrée dans `box_rect` et **à son rapport natif**.
    pub fn paint(&self, ui: &egui::Ui, box_rect: Rect, rarity: WakfuRarity) {
        let Some(texture) = self.par_numero.get(&IconRef::for_rarity(rarity).gfx_id) else {
            return;
        };
        let taille = overlay_ui::design::components::icon_button::glyph_fit(
            GEM_NATIVE,
            box_rect.width().min(box_rect.height()),
        );
        egui::Image::new(texture).paint_at(ui, Rect::from_center_size(box_rect.center(), taille));
    }
}
