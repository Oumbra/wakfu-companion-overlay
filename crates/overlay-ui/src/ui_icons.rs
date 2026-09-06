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
//! - `item_border_*` (`assets/items/Border-<RARETÉ>.webp`, copiés depuis la racine du dépôt, voir
//!   `docs/design-system.md` §2.4/§7) : les 7 textures d'emplacement d'objet du jeu, une par
//!   rareté — remplacent le dégradé+bordure dessinés à la main pour les tuiles ITEM du bandeau
//!   Suivi (`panels::watchlist::entry_tile`). `WakfuRarity::Old` (jamais résolue au runtime, voir
//!   sa doc) retombe sur la texture `Common`, aucun asset dédié n'existe pour cette rareté.
//!
//! **Refonte 2026-09-06 (design system boutons icône)** : l'utilisateur a fourni une planche de
//! référence (`menu-button-icon-first-plan.png`, menu d'icônes de premier plan du jeu) ainsi que le
//! socle qu'elle utilise, déjà détouré en deux états — `button-background.png`/
//! `button-background-hover.png` REMPLACENT les fichiers du même nom (12e retour, ci-dessus) par
//! cette nouvelle paire, plus proche du jeu que la première tentative. Conséquences :
//! - `external_link_icon`/`options_icon` ne sont plus recolorées une fois en `#fbfbfb` (12e retour)
//!   mais chargées en DEUX variantes (`_hover` incluse), recolorées à la volée en `#c5cbcc` au repos
//!   et `#f4d89f` survolée (voir `recolor`) — ces deux teintes sont celles mesurées par
//!   échantillonnage pixel sur la planche de référence, pas des valeurs devinées. `paint_icon_button`
//!   (`panels::combat`) est du même coup extrait en composant PARTAGÉ (`panels::icon_button`, voir sa
//!   doc), pour que ces deux boutons ET les deux boutons "+"/"−" du bandeau Suivi (ci-dessous)
//!   utilisent EXACTEMENT le même socle et la même teinte.
//! - `watchlist-add.png`/`watchlist-remove.png` (fond+glyphe déjà intégrés, ajoutés au retour
//!   précédent) sont RETIRÉS, remplacés par `icon-plus.png`/`icon-minus.png` (`icon-plus.png`/
//!   `icon-moins.png` fournis par l'utilisateur — glyphe seul, à fond transparent, PAS de socle
//!   intégré) : `panels::watchlist::control_button_row` compose désormais ces glyphes avec le MÊME
//!   `button_background`/`button_background_hover` que les boutons Combat, recolorés avec la MÊME
//!   paire `#c5cbcc`/`#f4d89f` — demande explicite : « appliques ce système aux quatre boutons ».
//!   `brighten`/`WATCHLIST_BUTTON_HOVER_BRIGHTEN` (éclaircissement approximatif de l'ancien glyphe
//!   intégré, ci-dessous) n'ont donc plus lieu d'être, retirés au passage.
//!
//! **Correctif 2026-09-06 (icônes visiblement plus petites en Combat)** : retour utilisateur,
//! capture des deux groupes de boutons à l'appui — l'icône "lien externe" (et dans une moindre
//! mesure "Options") paraissait nettement plus petite que "+"/"−" du panneau Suivi sur le MÊME
//! socle. `load_texture_recolored_pair` normalise désormais chaque icône (`normalize_icon_content`,
//! voir sa doc) avant recolorage : la marge transparente propre à chaque fichier source
//! (`external-link-icon.png`/`options-icon.png` en laissent, `icon-plus.png`/`icon-minus.png` non)
//! ne fausse plus la mise à l'échelle de `paint_icon_button`, qui reste par ailleurs inchangée.
//!
//! **Correctif 2026-09-06 (second passage, Options encore trop petite)** : la normalisation
//! ci-dessus alignait déjà la bbox d'Options (16×15) sur celle de "+"/"−" (16×16), mais retour
//! utilisateur après capture : toujours perçue comme trop petite — un rouage à traits fins reste
//! visuellement plus discret qu'un "+"/"−" plein à bbox strictement égale. `OPTIONS_ICON_CONTENT_
//! REFERENCE` (18px, +12,5 % par rapport à `ICON_CONTENT_REFERENCE`) lui est désormais dédiée,
//! `normalize_icon_content` prenant la référence cible en paramètre plutôt qu'une constante unique.

use overlay_engine::WakfuRarity;

const ALLIES_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/header-allies.png");
const ENEMIES_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/header-enemies.png");
const UNKNOWN_ENTITY_BYTES: &[u8] = include_bytes!("../assets/ui/unknown-entity.png");
const BUTTON_BACKGROUND_BYTES: &[u8] = include_bytes!("../assets/ui/button-background.png");
const BUTTON_BACKGROUND_HOVER_BYTES: &[u8] =
    include_bytes!("../assets/ui/button-background-hover.png");
const EXTERNAL_LINK_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/external-link-icon.png");
const OPTIONS_ICON_BYTES: &[u8] = include_bytes!("../assets/ui/options-icon.png");
const ICON_PLUS_BYTES: &[u8] = include_bytes!("../assets/ui/icon-plus.png");
const ICON_MINUS_BYTES: &[u8] = include_bytes!("../assets/ui/icon-minus.png");

const ITEM_BORDER_COMMON_BYTES: &[u8] = include_bytes!("../assets/items/Border-COMMON.webp");
const ITEM_BORDER_RARE_BYTES: &[u8] = include_bytes!("../assets/items/Border-RARE.webp");
const ITEM_BORDER_MYTHICAL_BYTES: &[u8] = include_bytes!("../assets/items/Border-MYTHICAL.webp");
const ITEM_BORDER_LEGENDARY_BYTES: &[u8] = include_bytes!("../assets/items/Border-LEGENDARY.webp");
const ITEM_BORDER_MEMORY_BYTES: &[u8] = include_bytes!("../assets/items/Border-MEMORY.webp");
const ITEM_BORDER_EPIC_BYTES: &[u8] = include_bytes!("../assets/items/Border-EPIC.webp");
const ITEM_BORDER_RELIC_BYTES: &[u8] = include_bytes!("../assets/items/Border-RELIC.webp");

/// Teinte des icônes posées sur `button_background` au repos (`external_link_icon`, `options_icon`,
/// `icon_plus`, `icon_minus`) — mesurée par échantillonnage pixel sur la planche de référence
/// (`menu-button-icon-first-plan.png`, fournie par l'utilisateur), pas devinée : `#c5cbcc`. Voir
/// `recolor` — remplace le canal RGB de chaque pixel opaque, alpha (donc silhouette et
/// anticrénelage) inchangé.
const ICON_COLOR: [u8; 3] = [0xc5, 0xcb, 0xcc];
/// Même rôle que `ICON_COLOR`, état survolé — `#f4d89f`, valeur donnée par l'utilisateur (pas
/// mesurée sur la planche de référence, qui ne montre aucun bouton survolé).
const ICON_COLOR_HOVER: [u8; 3] = [0xf4, 0xd8, 0x9f];

/// Taille de référence (plus grande dimension du CONTENU opaque, pas du canevas) à laquelle
/// `normalize_icon_content` recale "lien externe"/"+"/"−" — voir sa doc. Étalon choisi : la taille
/// native de `icon-plus.png`/`icon-minus.png` (16×16, fournis par l'utilisateur), déjà jugée
/// correcte visuellement, plutôt qu'une valeur arbitraire.
const ICON_CONTENT_REFERENCE: f32 = 16.0;

/// Référence DÉDIÉE à l'icône "Options" — retour utilisateur 2026-09-06 (second passage, après la
/// normalisation générale ci-dessus) : encore perçue comme trop petite alors que sa bbox (16×15,
/// voir doc de `normalize_icon_content`) est DÉJÀ alignée sur `ICON_CONTENT_REFERENCE` — un rouage
/// à traits fins reste visuellement plus discret qu'un "+"/"−" plein à bbox strictement égale (une
/// bbox identique ne garantit pas la même DENSITÉ d'encre). Ajustement délibérément propre à cette
/// icône (+12,5 %, 16 → 18px) plutôt qu'une nouvelle mesure générale qui aurait aussi fait grossir
/// "+"/"−"/lien externe, déjà jugés corrects.
const OPTIONS_ICON_CONTENT_REFERENCE: f32 = 18.0;

/// Rogne `img` à la bbox de ses pixels opaques (seuil `ALPHA_THRESHOLD`) puis le remet à l'échelle
/// (aspect ratio conservé) pour que la plus grande dimension de cette bbox atteigne `target` — voir
/// `ICON_CONTENT_REFERENCE`/`OPTIONS_ICON_CONTENT_REFERENCE`. Retour utilisateur 2026-09-06
/// (comparaison de deux captures des boutons Combat et Suivi) : l'icône "lien externe" paraissait
/// nettement plus petite que "+"/"−" une fois posée sur le MÊME socle, alors que `paint_icon_button`
/// les met toutes à l'échelle dans le même ratio (voir sa doc). Cause mesurée par bbox opaque, pas
/// par impression : les canevas `external-link-icon.png` (18×18) et `options-icon.png` (22×22)
/// laissent une marge transparente autour du glyphe (bbox réelle 13×13 et 16×15), alors que
/// `icon-plus.png`/`icon-minus.png` (16×16) occupent tout leur canevas (bbox 16×16 et 16×6) —
/// `paint_icon_button` met à l'échelle le CANEVAS entier, marge invisible comprise, donc un glyphe
/// entouré de plus de marge ressort plus petit à socle égal. Rogner puis recaler sur un étalon
/// commun élimine cette marge cachée sans toucher au reste du pipeline (recolorage,
/// `paint_icon_button`) : "+"/"−" (bbox déjà ≈16px) en ressortent quasi inchangés (facteur proche de
/// 1.0), le lien externe (bbox 13px) est réellement agrandi (facteur ≈1.23) — et Options utilise sa
/// propre référence, plus grande (voir `OPTIONS_ICON_CONTENT_REFERENCE`).
fn normalize_icon_content(img: &image::RgbaImage, target: f32) -> image::RgbaImage {
    const ALPHA_THRESHOLD: u8 = 10;
    let (width, height) = img.dimensions();
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x: i64 = -1;
    let mut max_y: i64 = -1;
    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel.0[3] > ALPHA_THRESHOLD {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x as i64);
            max_y = max_y.max(y as i64);
        }
    }
    if max_x < 0 {
        // Aucun pixel opaque (fichier corrompu/vide) — rien à rogner, on repart du canevas tel quel
        // plutôt que de paniquer sur une bbox négative.
        return img.clone();
    }
    let bbox_w = (max_x as u32 + 1).saturating_sub(min_x).max(1);
    let bbox_h = (max_y as u32 + 1).saturating_sub(min_y).max(1);
    let cropped = image::imageops::crop_imm(img, min_x, min_y, bbox_w, bbox_h).to_image();

    let content_size = bbox_w.max(bbox_h) as f32;
    let scale = target / content_size;
    if (scale - 1.0).abs() < 0.01 {
        return cropped;
    }
    let new_w = (bbox_w as f32 * scale).round().max(1.0) as u32;
    let new_h = (bbox_h as f32 * scale).round().max(1.0) as u32;
    image::imageops::resize(&cropped, new_w, new_h, image::imageops::FilterType::CatmullRom)
}

pub struct UiIcons {
    allies: egui::TextureHandle,
    enemies: egui::TextureHandle,
    unknown_entity: egui::TextureHandle,
    button_background: egui::TextureHandle,
    button_background_hover: egui::TextureHandle,
    external_link_icon: egui::TextureHandle,
    external_link_icon_hover: egui::TextureHandle,
    options_icon: egui::TextureHandle,
    options_icon_hover: egui::TextureHandle,
    icon_plus: egui::TextureHandle,
    icon_plus_hover: egui::TextureHandle,
    icon_minus: egui::TextureHandle,
    icon_minus_hover: egui::TextureHandle,
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
        let (external_link_icon, external_link_icon_hover) = load_texture_recolored_pair(
            ctx,
            "icon-external-link",
            EXTERNAL_LINK_ICON_BYTES,
            ICON_CONTENT_REFERENCE,
        );
        let (options_icon, options_icon_hover) = load_texture_recolored_pair(
            ctx,
            "icon-options",
            OPTIONS_ICON_BYTES,
            OPTIONS_ICON_CONTENT_REFERENCE,
        );
        let (icon_plus, icon_plus_hover) = load_texture_recolored_pair(
            ctx,
            "icon-plus",
            ICON_PLUS_BYTES,
            ICON_CONTENT_REFERENCE,
        );
        let (icon_minus, icon_minus_hover) = load_texture_recolored_pair(
            ctx,
            "icon-minus",
            ICON_MINUS_BYTES,
            ICON_CONTENT_REFERENCE,
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
            external_link_icon,
            external_link_icon_hover,
            options_icon,
            options_icon_hover,
            icon_plus,
            icon_plus_hover,
            icon_minus,
            icon_minus_hover,
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

    /// Socle d'un bouton icône du design system, état au repos — voir doc de module et
    /// `panels::icon_button::paint_icon_button`. Partagé par les quatre boutons icône de l'overlay
    /// (lien externe, Options, "+"/"−" du Suivi).
    pub fn button_background(&self) -> &egui::TextureHandle {
        &self.button_background
    }

    /// Socle d'un bouton icône du design system, état survolé — voir `button_background`.
    pub fn button_background_hover(&self) -> &egui::TextureHandle {
        &self.button_background_hover
    }

    /// Icône "lien externe" à fond transparent recolorée en `ICON_COLOR`, à centrer sur
    /// `button_background` — voir doc de module et `panels::icon_button::paint_icon_button`.
    pub fn external_link_icon(&self) -> &egui::TextureHandle {
        &self.external_link_icon
    }

    /// Même icône, recolorée en `ICON_COLOR_HOVER` — à centrer sur `button_background_hover`.
    pub fn external_link_icon_hover(&self) -> &egui::TextureHandle {
        &self.external_link_icon_hover
    }

    /// Icône "Options" à fond transparent recolorée en `ICON_COLOR`, à centrer sur
    /// `button_background` — voir doc de module et `panels::combat::bottom_toolbar`.
    pub fn options_icon(&self) -> &egui::TextureHandle {
        &self.options_icon
    }

    /// Même icône, recolorée en `ICON_COLOR_HOVER` — à centrer sur `button_background_hover`.
    pub fn options_icon_hover(&self) -> &egui::TextureHandle {
        &self.options_icon_hover
    }

    /// Glyphe "+" à fond transparent recoloré en `ICON_COLOR` (`icon-plus.png`, fourni par
    /// l'utilisateur) — à composer avec `button_background`, voir
    /// `panels::watchlist::control_button_row`.
    pub fn icon_plus(&self) -> &egui::TextureHandle {
        &self.icon_plus
    }

    /// Même glyphe, recoloré en `ICON_COLOR_HOVER` — à composer avec `button_background_hover`.
    pub fn icon_plus_hover(&self) -> &egui::TextureHandle {
        &self.icon_plus_hover
    }

    /// Glyphe "−" à fond transparent recoloré en `ICON_COLOR` (`icon-minus.png`, `icon-moins.png`
    /// fourni par l'utilisateur) — voir `icon_plus`.
    pub fn icon_minus(&self) -> &egui::TextureHandle {
        &self.icon_minus
    }

    /// Même glyphe, recoloré en `ICON_COLOR_HOVER` — voir `icon_plus_hover`.
    pub fn icon_minus_hover(&self) -> &egui::TextureHandle {
        &self.icon_minus_hover
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

/// Charge `bytes` (glyphe à fond transparent, une seule teinte d'origine — voir doc de module) en
/// DEUX textures : recolorée en `ICON_COLOR` (repos) puis en `ICON_COLOR_HOVER` (survolée), voir
/// `recolor`. Utilisé pour les quatre icônes du design system boutons (lien externe, Options, "+",
/// "−") — toutes partagent la même paire de teintes plutôt qu'un `tint()` egui dynamique (qui ne
/// peut que MULTIPLIER la couleur d'origine, jamais l'éclaircir au-delà — même limite déjà
/// rencontrée pour le grisé KO, voir `portraits::to_grayscale`).
fn load_texture_recolored_pair(
    ctx: &egui::Context,
    name: &'static str,
    bytes: &[u8],
    content_reference: f32,
) -> (egui::TextureHandle, egui::TextureHandle) {
    let decoded = normalize_icon_content(&decode(bytes), content_reference);
    let normal = recolor(&decoded, ICON_COLOR);
    let hovered = recolor(&decoded, ICON_COLOR_HOVER);
    (
        load_rgba(ctx, name, normal.dimensions(), normal.as_raw()),
        load_rgba(
            ctx,
            &format!("{name}-hover"),
            hovered.dimensions(),
            hovered.as_raw(),
        ),
    )
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

/// Remplace le canal RGB de CHAQUE pixel de `decoded` par `rgb` (alpha inchangé) — même principe
/// que le recolorage ponctuel de `options-icon.png` en `#fbfbfb` (voir doc de module, 12e retour),
/// généralisé ici en fonction réutilisable : un remplacement pur plutôt qu'un mélange (contrairement
/// à l'ancien `brighten`), la silhouette et son anticrénelage (portés par l'alpha, pas la couleur)
/// restent identiques au fichier fourni par l'utilisateur.
fn recolor(decoded: &image::RgbaImage, rgb: [u8; 3]) -> image::RgbaImage {
    let mut out = decoded.clone();
    for pixel in out.pixels_mut() {
        let a = pixel.0[3];
        pixel.0 = [rgb[0], rgb[1], rgb[2], a];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recolor_remplace_rgb_alpha_preserve() {
        let mut img = image::RgbaImage::new(1, 1);
        img.put_pixel(0, 0, image::Rgba([100, 100, 100, 137]));
        let out = recolor(&img, [0xc5, 0xcb, 0xcc]);
        let px = out.get_pixel(0, 0);
        assert_eq!(px.0, [0xc5, 0xcb, 0xcc, 137]);
    }

    /// Un glyphe déjà à la taille de référence (comme `icon-plus.png`, 16×16 plein cadre) ne doit
    /// quasiment pas bouger — seule la marge transparente autour d'un glyphe plus petit doit être
    /// éliminée (voir `normalize_icon_content`).
    #[test]
    fn normalize_icon_content_glyphe_deja_a_la_reference_inchange() {
        let mut img = image::RgbaImage::new(16, 16);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([255, 255, 255, 255]);
        }
        let out = normalize_icon_content(&img, ICON_CONTENT_REFERENCE);
        assert_eq!(out.dimensions(), (16, 16));
    }

    /// Un glyphe entouré d'une marge transparente (bbox plus petite que son canevas, comme
    /// `external-link-icon.png`) doit être rogné PUIS agrandi jusqu'à la référence, pas laissé à sa
    /// taille de bbox — sans quoi il resterait plus petit que les glyphes déjà pleins cadre une fois
    /// posé sur le même socle (retour utilisateur, voir doc de la fonction).
    #[test]
    fn normalize_icon_content_rogne_puis_agrandit_a_la_reference() {
        let mut img = image::RgbaImage::new(18, 18);
        for y in 3..16 {
            for x in 3..16 {
                img.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }
        let out = normalize_icon_content(&img, ICON_CONTENT_REFERENCE);
        // Bbox opaque = 13×13 (indices 3..=15) ; agrandie pour que sa plus grande dimension
        // atteigne ICON_CONTENT_REFERENCE (16px), donc 16×16 ici (carré).
        assert_eq!(out.dimensions(), (16, 16));
    }

    /// L'icône Options utilise sa PROPRE référence, plus grande (voir
    /// `OPTIONS_ICON_CONTENT_REFERENCE`) — un glyphe déjà plein cadre à 16×16 doit donc quand même
    /// être agrandi jusqu'à 18×18 avec cette référence, contrairement au cas générique ci-dessus.
    #[test]
    fn normalize_icon_content_options_utilise_sa_propre_reference() {
        let mut img = image::RgbaImage::new(16, 16);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([255, 255, 255, 255]);
        }
        let out = normalize_icon_content(&img, OPTIONS_ICON_CONTENT_REFERENCE);
        assert_eq!(out.dimensions(), (18, 18));
    }
}
