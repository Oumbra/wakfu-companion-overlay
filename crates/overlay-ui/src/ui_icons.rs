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
//! **Retour utilisateur suivant** : le doré d'origine détonnait à côté du blanc de l'icône lien
//! externe dans la même barre d'outils — recolorée en `#fbfbfb` (RGB des pixels opaques remplacé,
//! alpha inchangé, donc silhouette et anticrénelage identiques au fichier fourni) pour reprendre
//! exactement la couleur de remplissage de `external-link-icon.png`.

const ALLIES_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/header-allies.png");
const ENEMIES_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/header-enemies.png");
const UNKNOWN_ENTITY_BYTES: &[u8] = include_bytes!("../assets/ui/unknown-entity.png");
const BUTTON_BACKGROUND_BYTES: &[u8] = include_bytes!("../assets/ui/button-background.png");
const BUTTON_BACKGROUND_HOVER_BYTES: &[u8] =
    include_bytes!("../assets/ui/button-background-hover.png");
const EXTERNAL_LINK_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/external-link-icon.png");
const OPTIONS_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/options-icon.png");

pub struct UiIcons {
    allies: egui::TextureHandle,
    enemies: egui::TextureHandle,
    unknown_entity: egui::TextureHandle,
    button_background: egui::TextureHandle,
    button_background_hover: egui::TextureHandle,
    external_link_icon: egui::TextureHandle,
    options_icon: egui::TextureHandle,
}

impl UiIcons {
    pub fn load(ctx: &egui::Context) -> Self {
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
