//! Les images `wakassets` dont les maquettes ont besoin — gemmes de rareté et icônes de catégorie.
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
use overlay_engine::{IconRef, WakfuItemCategory, WakfuRarity};

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

// -------------------------------------------------------------------------------------------
// Les filtres par catégorie de l'autocomplétion
// -------------------------------------------------------------------------------------------

/// Un bouton de la bande de filtres — miroir de `WakfuAutocompleteFilterButton`
/// (`wakfu-autocomplete.component.ts`).
///
/// **L'ordre de cette énumération est celui de la bande** : « Tout » d'abord, puis les huit
/// catégories d'objet dans l'ordre de `WAKFU_ITEM_CATEGORIES`, et « Monstres » en dernier (le web
/// l'ajoute après les catégories, et seulement en domaine `both`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CategoryFilter {
    All,
    Equipment,
    Resources,
    Sublimations,
    Harvests,
    HavenBag,
    Cosmetics,
    Craft,
    Misc,
    /// **Domaine `both` seulement** — la page Alertes est en domaine `item` et ne l'affiche
    /// jamais. `allow` plutôt qu'une suppression : ce module est compilé dans DEUX exemples, et
    /// chacun n'en utilise qu'une partie ; la planche des composants, elle, montre bien ce filtre
    /// dans sa bande « les dix filtres possibles ».
    #[allow(dead_code)]
    Enemy,
}

impl CategoryFilter {
    /// Les huit catégories d'OBJET, dans l'ordre du web. Ni « Tout » ni « Monstres » : le premier
    /// n'est pas une catégorie, le second n'existe qu'en domaine `both` — et la page Alertes est en
    /// domaine `item` (voir `profile-page.component.html`, `domain="item"`).
    pub const ITEM_CATEGORIES: [CategoryFilter; 8] = [
        CategoryFilter::Equipment,
        CategoryFilter::Resources,
        CategoryFilter::Sublimations,
        CategoryFilter::Harvests,
        CategoryFilter::HavenBag,
        CategoryFilter::Cosmetics,
        CategoryFilter::Craft,
        CategoryFilter::Misc,
    ];

    /// La référence d'icône du filtre — **construite par le moteur**, jamais par une table
    /// recopiée ici.
    ///
    /// C'est le même principe que pour les gemmes : la planche choisit son fichier par le `gfx_id`
    /// que produira l'URL au runtime, donc une planche fausse voudrait dire que le code de
    /// production l'est aussi. « Tout » et « Monstres » ont leurs propres constructeurs côté
    /// moteur — ce ne sont pas des catégories d'objet.
    pub fn icon_ref(self) -> IconRef {
        match self {
            CategoryFilter::All => IconRef::for_all_categories(),
            CategoryFilter::Enemy => IconRef::for_monster_category(),
            CategoryFilter::Equipment => IconRef::for_item_category(WakfuItemCategory::Equipment),
            CategoryFilter::Resources => IconRef::for_item_category(WakfuItemCategory::Resources),
            CategoryFilter::Sublimations => {
                IconRef::for_item_category(WakfuItemCategory::Sublimations)
            }
            CategoryFilter::Harvests => IconRef::for_item_category(WakfuItemCategory::Harvests),
            CategoryFilter::HavenBag => IconRef::for_item_category(WakfuItemCategory::HavenBag),
            CategoryFilter::Cosmetics => IconRef::for_item_category(WakfuItemCategory::Cosmetics),
            CategoryFilter::Craft => IconRef::for_item_category(WakfuItemCategory::Craft),
            CategoryFilter::Misc => IconRef::for_item_category(WakfuItemCategory::Misc),
        }
    }

    /// Libellé d'infobulle — les clés `itemCategory.*` et `wakfuAutocomplete.allCategories` de
    /// `translations.ts`, en français.
    ///
    /// `allow(dead_code)` pour la même raison que [`CategoryFilter::Enemy`] : seule la planche des
    /// composants légende ses filtres, la maquette de la page ne montre que la bande.
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            CategoryFilter::All => "Tout",
            CategoryFilter::Equipment => "Equipements",
            CategoryFilter::Resources => "Ressources",
            CategoryFilter::Sublimations => "Sublimations",
            CategoryFilter::Harvests => "Récoltes",
            CategoryFilter::HavenBag => "Havre-Sac",
            CategoryFilter::Cosmetics => "Cosmétiques",
            CategoryFilter::Craft => "Craft",
            CategoryFilter::Misc => "Divers",
            CategoryFilter::Enemy => "Monstres",
        }
    }
}

/// Les dix icônes de catégorie, chargées une fois.
pub struct CategoryIcons {
    par_numero: HashMap<String, TextureHandle>,
}

impl CategoryIcons {
    pub fn load(ctx: &Context) -> Self {
        const FIXTURES: &[(&str, &[u8])] = &[
            ("-1", include_bytes!("../../fixtures/item-types/-1.png")),
            ("109", include_bytes!("../../fixtures/item-types/109.png")),
            ("226", include_bytes!("../../fixtures/item-types/226.png")),
            ("602", include_bytes!("../../fixtures/item-types/602.png")),
            ("237", include_bytes!("../../fixtures/item-types/237.png")),
            ("295", include_bytes!("../../fixtures/item-types/295.png")),
            ("525", include_bytes!("../../fixtures/item-types/525.png")),
            ("761", include_bytes!("../../fixtures/item-types/761.png")),
            ("385", include_bytes!("../../fixtures/item-types/385.png")),
            ("282", include_bytes!("../../fixtures/item-types/282.png")),
        ];
        let par_numero = FIXTURES
            .iter()
            .map(|(numero, bytes)| {
                let image = image::load_from_memory(bytes)
                    .expect("icône de catégorie décodable")
                    .to_rgba8();
                let taille = [image.width() as usize, image.height() as usize];
                let texture = ctx.load_texture(
                    format!("maquette.categorie-{numero}"),
                    egui::ColorImage::from_rgba_unmultiplied(taille, image.as_raw()),
                    TextureOptions::LINEAR,
                );
                ((*numero).to_string(), texture)
            })
            .collect();
        Self { par_numero }
    }

    /// Peint l'icône d'un filtre dans `rect`, avec l'opacité que le web donne à son état.
    ///
    /// `actif` : pleine opacité. Sinon 0,6, comme `.wakfu-autocomplete-category-btn` au repos —
    /// c'est cette différence d'encre, pas seulement le cadre, qui fait ressortir le filtre en
    /// cours.
    pub fn paint(&self, ui: &egui::Ui, rect: Rect, filtre: CategoryFilter, actif: bool) {
        let Some(texture) = self.par_numero.get(&filtre.icon_ref().gfx_id) else {
            return;
        };
        let teinte = if actif {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_white_alpha(153)
        };
        egui::Image::new(texture).tint(teinte).paint_at(ui, rect);
    }
}
