//! Icônes d'interface embarquées — switch Alliés/Ennemis du panneau Combat (mêmes fichiers PNG
//! que le dépôt web, `public/assets/ui/header-allies-*.png`/`header-enemies-*.png` — hash de
//! cache retiré du nom de fichier, inutile ici puisqu'il n'y a pas de navigateur), portrait de
//! repli pour un ennemi (`unknown-entity-*.png`) : un ennemi n'a jamais de classe résolue (`breed`
//! pas déterministe côté ennemi, voir `overlay_engine::class_breed`), donc jamais de portrait de
//! la planche `class-avatars-sheet.png` — voir `portraits.rs` ; et le socle + l'icône du bouton
//! "lien externe" du panneau Combat (`panels::combat::show_leader_row`, composés par
//! `panels::combat::paint_icon_button`) — **fournis directement par l'utilisateur** (pas extraits
//! ni redessinés, contrairement à une première tentative rejetée qui avait isolé la texture au
//! pixel près depuis une capture d'écran) : `button-background.png` est le socle générique d'un
//! bouton icône du jeu au repos, `button-background-hover.png` le même socle éclairci pour l'état
//! survolé (ajouté après coup, retour utilisateur explicite), `external-link-icon.png` l'icône à
//! fond transparent à centrer dessus. Chargées une fois par fenêtre overlay, même logique que
//! `PortraitAtlas`.
//!
//! **Ajout 2026-09-05** : `options-icon.png` (`nut.png` fourni par l'utilisateur — un écrou/rouage
//! 22×22 à fond transparent, à l'origine doré) est l'icône du nouveau bouton "Options" de la barre
//! d'outils en bas du panneau Combat (`panels::combat::bottom_toolbar`), composée avec le MÊME
//! socle `button_background`/`button_background_hover` que le bouton lien externe —
//! `paint_icon_button` reste un composant générique (voir sa doc). N'ouvre encore aucun panneau :
//! réservé à une future page de réglages.
//!
//! **Refonte 2026-09-05 (12e retour, voir `panels::combat`)** : le doré d'origine détonnait à côté
//! du blanc de l'icône lien externe dans la même barre d'outils — recolorée en `#fbfbfb` (RGB des
//! pixels opaques remplacé, alpha inchangé, donc silhouette et anticrénelage identiques au fichier
//! fourni) pour reprendre exactement la couleur de remplissage de `external-link-icon.png`.
//!
//! **Ajout 2026-09-06** (refonte du panneau Suivi, voir `panels::watchlist`) :
//! - `watchlist-add.png`/`watchlist-remove.png` (`button-plus.png`/`button-moins.png` fournis par
//!   l'utilisateur — deux icônes 34×34 à fond arrondi sombre déjà intégré, PAS un fond+glyphe
//!   séparés comme `button_background`/`external_link_icon`) remplacent les deux tuiles "+"/"−"
//!   dessinées à la main (bordure pointillée, glyphe ASCII) du bandeau Suivi — demande explicite :
//!   réutiliser des icônes que l'utilisateur reconnaît déjà dans l'interface du jeu, et gagner de
//!   la place (34px de large empilés verticalement contre 2×58px+écart côte à côte auparavant).
//!   Contrairement à `button_background`, l'utilisateur n'a fourni qu'UNE SEULE variante de chacune
//!   (pas de second fichier "survolé" séparé) — l'état survolé (« s'éclaircit ») est donc généré
//!   ici au chargement plutôt qu'attendu comme un second asset, voir `brighten`.
//! - `item_border_*` (`assets/items/Border-<RARETÉ>.webp`, copiés depuis la racine du dépôt, voir
//!   `docs/design-system.md` §2.4/§7) : les 7 textures d'emplacement d'objet du jeu, une par
//!   rareté — remplacent le dégradé+bordure dessinés à la main pour les tuiles ITEM du bandeau
//!   Suivi (`panels::watchlist::entry_tile`). `WakfuRarity::Old` (jamais résolue au runtime, voir
//!   sa doc) retombe sur la texture `Common`, aucun asset dédié n'existe pour cette rareté.

use overlay_engine::WakfuRarity;

const ALLIES_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/header-allies.png");
const ENEMIES_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/header-enemies.png");
const UNKNOWN_ENTITY_BYTES: &[u8] = include_bytes!("../assets/ui/unknown-entity.png");
const BUTTON_BACKGROUND_BYTES: &[u8] = include_bytes!("../assets/ui/button-background.png");
const BUTTON_BACKGROUND_HOVER_BYTES: &[u8] =
    include_bytes!("../assets/ui/button-background-hover.png");
const EXTERNAL_LINK_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/external-link-icon.png");
const OPTIONS_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/options-icon.png");
const WATCHLIST_ADD_BYTES: &[u8] = include_bytes!("../assets/ui/watchlist-add.png");
const WATCHLIST_REMOVE_BYTES: &[u8] = include_bytes!("../assets/ui/watchlist-remove.png");

const ITEM_BORDER_COMMON_BYTES: &[u8] = include_bytes!("../assets/items/Border-COMMON.webp");
const ITEM_BORDER_RARE_BYTES: &[u8] = include_bytes!("../assets/items/Border-RARE.webp");
const ITEM_BORDER_MYTHICAL_BYTES: &[u8] = include_bytes!("../assets/items/Border-MYTHICAL.webp");
const ITEM_BORDER_LEGENDARY_BYTES: &[u8] = include_bytes!("../assets/items/Border-LEGENDARY.webp");
const ITEM_BORDER_MEMORY_BYTES: &[u8] = include_bytes!("../assets/items/Border-MEMORY.webp");
const ITEM_BORDER_EPIC_BYTES: &[u8] = include_bytes!("../assets/items/Border-EPIC.webp");
const ITEM_BORDER_RELIC_BYTES: &[u8] = include_bytes!("../assets/items/Border-RELIC.webp");

/// Facteur d'éclaircissement appliqué à `watchlist_add`/`watchlist_remove` pour produire leur
/// variante survolée (voir `brighten`) — calibré par échantillonnage pixel sur la paire
/// `button_background`/`button_background_hover` déjà fournie par l'utilisateur pour le bouton
/// lien externe (centre de l'asset : `(112,104,85)` au repos → `(133,123,98)` survolé, soit un
/// mélange vers le blanc d'environ 12 à 15 % selon le canal) : reprend le MÊME degré
/// d'éclaircissement plutôt qu'une valeur arbitraire, pour rester cohérent avec le seul autre
/// exemple de bouton survolé déjà validé par l'utilisateur dans cette UI.
const WATCHLIST_BUTTON_HOVER_BRIGHTEN: f32 = 0.15;

pub struct UiIcons {
    allies: egui::TextureHandle,
    enemies: egui::TextureHandle,
    unknown_entity: egui::TextureHandle,
    button_background: egui::TextureHandle,
    button_background_hover: egui::TextureHandle,
    external_link_icon: egui::TextureHandle,
    options_icon: egui::TextureHandle,
    watchlist_add: egui::TextureHandle,
    watchlist_add_hover: egui::TextureHandle,
    watchlist_remove: egui::TextureHandle,
    watchlist_remove_hover: egui::TextureHandle,
    item_border_common: egui::TextureHandle,
    item_border_rare: egui::TextureHandle,
    item_border_mythical: egui::TextureHandle,
    item_border_legendary: egui::TextureHandle,
    item_border_memory: egui::TextureHandle,
    item_border_epic: egui::TextureHandle,
    item_border_relic: egui::TextureHandle,
}

impl UiIcons {
    pub fn load(ctx: &egui::Context) -> Self {
        let (watchlist_add, watchlist_add_hover) = load_texture_with_hover(
            ctx,
            "icon-watchlist-add",
            WATCHLIST_ADD_BYTES,
            WATCHLIST_BUTTON_HOVER_BRIGHTEN,
        );
        let (watchlist_remove, watchlist_remove_hover) = load_texture_with_hover(
            ctx,
            "icon-watchlist-remove",
            WATCHLIST_REMOVE_BYTES,
            WATCHLIST_BUTTON_HOVER_BRIGHTEN,
        );
        Self {
            allies: load_texture(ctx, "icon-header-allies", ALLIES_ICON_BYTES),
            enemies: load_texture(ctx, "icon-header-enemies", ENEMIES_ICON_BYTES),
            unknown_entity: load_texture(ctx, "icon-unknown-entity", UNKNOWN_ENTITY_BYTES),
            button_background: load_texture(ctx, "icon-button-background", BUTTON_BACKGROUND_BYTES),
            button_background_hover: load_texture(
                ctx,
                "icon-button-background-hover",
                BUTTON_BACKGROUND_HOVER_BYTES,
            ),
            external_link_icon: load_texture(ctx, "icon-external-link", EXTERNAL_LINK_ICON_BYTES),
            options_icon: load_texture(ctx, "icon-options", OPTIONS_ICON_BYTES),
            watchlist_add,
            watchlist_add_hover,
            watchlist_remove,
            watchlist_remove_hover,
            item_border_common: load_texture(ctx, "item-border-common", ITEM_BORDER_COMMON_BYTES),
            item_border_rare: load_texture(ctx, "item-border-rare", ITEM_BORDER_RARE_BYTES),
            item_border_mythical: load_texture(
                ctx,
                "item-border-mythical",
                ITEM_BORDER_MYTHICAL_BYTES,
            ),
            item_border_legendary: load_texture(
                ctx,
                "item-border-legendary",
                ITEM_BORDER_LEGENDARY_BYTES,
            ),
            item_border_memory: load_texture(ctx, "item-border-memory", ITEM_BORDER_MEMORY_BYTES),
            item_border_epic: load_texture(ctx, "item-border-epic", ITEM_BORDER_EPIC_BYTES),
            item_border_relic: load_texture(ctx, "item-border-relic", ITEM_BORDER_RELIC_BYTES),
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

    /// Socle d'un bouton icône du jeu, état au repos — voir doc de module et
    /// `panels::combat::paint_icon_button`.
    pub fn button_background(&self) -> &egui::TextureHandle {
        &self.button_background
    }

    /// Socle d'un bouton icône du jeu, état survolé (éclairci) — voir doc de module et
    /// `panels::combat::paint_icon_button`.
    pub fn button_background_hover(&self) -> &egui::TextureHandle {
        &self.button_background_hover
    }

    /// Icône "lien externe" à fond transparent, à centrer sur `button_background` — voir doc de
    /// module et `panels::combat::paint_icon_button`.
    pub fn external_link_icon(&self) -> &egui::TextureHandle {
        &self.external_link_icon
    }

    /// Icône "Options" à fond transparent, à centrer sur `button_background` — voir doc de module
    /// et `panels::combat::bottom_toolbar`.
    pub fn options_icon(&self) -> &egui::TextureHandle {
        &self.options_icon
    }

    /// Bouton "+" du bandeau Suivi, état au repos (fond arrondi + glyphe déjà intégrés à l'asset,
    /// voir doc de module) — `panels::watchlist::control_button`.
    pub fn watchlist_add(&self) -> &egui::TextureHandle {
        &self.watchlist_add
    }

    /// Même bouton, état survolé (éclairci, voir `brighten`).
    pub fn watchlist_add_hover(&self) -> &egui::TextureHandle {
        &self.watchlist_add_hover
    }

    /// Bouton "−" du bandeau Suivi, état au repos — voir `watchlist_add`.
    pub fn watchlist_remove(&self) -> &egui::TextureHandle {
        &self.watchlist_remove
    }

    /// Même bouton, état survolé (éclairci, voir `brighten`).
    pub fn watchlist_remove_hover(&self) -> &egui::TextureHandle {
        &self.watchlist_remove_hover
    }

    /// Texture d'emplacement d'objet du jeu pour `rarity` — voir doc de module et
    /// `panels::watchlist::entry_tile`. `WakfuRarity::Old` retombe sur `Common` (aucun asset dédié,
    /// jamais résolue au runtime de toute façon, voir sa doc).
    pub fn item_border(&self, rarity: WakfuRarity) -> &egui::TextureHandle {
        match rarity {
            WakfuRarity::Old | WakfuRarity::Common => &self.item_border_common,
            WakfuRarity::Rare => &self.item_border_rare,
            WakfuRarity::Mythical => &self.item_border_mythical,
            WakfuRarity::Legendary => &self.item_border_legendary,
            WakfuRarity::Memory => &self.item_border_memory,
            WakfuRarity::Epic => &self.item_border_epic,
            WakfuRarity::Relic => &self.item_border_relic,
        }
    }
}

fn load_texture(ctx: &egui::Context, name: &'static str, bytes: &[u8]) -> egui::TextureHandle {
    let decoded = decode(bytes);
    load_rgba(ctx, name, decoded.dimensions(), decoded.as_raw())
}

/// Charge `bytes` en DEUX textures : la couleur d'origine, puis une variante éclaircie de
/// `brighten_factor` (voir `brighten`) pour l'état survolé — voir doc de module (boutons "+"/"−"
/// du bandeau Suivi, un seul fichier fourni par l'utilisateur).
fn load_texture_with_hover(
    ctx: &egui::Context,
    name: &'static str,
    bytes: &[u8],
    brighten_factor: f32,
) -> (egui::TextureHandle, egui::TextureHandle) {
    let decoded = decode(bytes);
    let normal = load_rgba(ctx, name, decoded.dimensions(), decoded.as_raw());
    let brightened = brighten(&decoded, brighten_factor);
    let hover = load_rgba(
        ctx,
        &format!("{name}-hover"),
        brightened.dimensions(),
        brightened.as_raw(),
    );
    (normal, hover)
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

/// Éclaircit `decoded` en mélangeant chaque canal de couleur vers le blanc dans la proportion
/// `factor` (0.0 = inchangé, 1.0 = blanc plein), alpha préservé — un `tint()` egui à l'affichage ne
/// peut que MULTIPLIER la couleur d'origine (donc l'assombrir ou la laisser inchangée, jamais
/// l'éclaircir au-delà de l'original — même limite déjà rencontrée et contournée pour le grisé KO,
/// voir `portraits::to_grayscale`), d'où un précalcul au chargement plutôt qu'un tint dynamique.
fn brighten(decoded: &image::RgbaImage, factor: f32) -> image::RgbaImage {
    let mut out = decoded.clone();
    for pixel in out.pixels_mut() {
        let [r, g, b, a] = pixel.0;
        let toward_white = |c: u8| (c as f32 + (255.0 - c as f32) * factor).round() as u8;
        pixel.0 = [toward_white(r), toward_white(g), toward_white(b), a];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brighten_melange_vers_le_blanc_alpha_preserve() {
        let mut img = image::RgbaImage::new(1, 1);
        img.put_pixel(0, 0, image::Rgba([100, 100, 100, 137]));
        let out = brighten(&img, 0.5);
        let px = out.get_pixel(0, 0);
        assert_eq!(px.0, [178, 178, 178, 137]);
    }
}
