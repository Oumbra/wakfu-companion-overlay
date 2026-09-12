//! **Design system de l'overlay** — la couche qui transforme les captures préparées de
//! `assets/design-system/` en composants egui paramétrables, réutilisables et testés.
//!
//! Motivation (retour utilisateur 2026-09-09) : jusqu'ici chaque panneau redessinait ses propres
//! boutons à la main (`panels::options_modal` : sept constantes de couleur et un `chamfer` par
//! bouton), et chaque nouvelle taille de bouton demandait un **nouvel asset** découpé à la main,
//! libellé compris (`assets/design-system/large-button-cancel.png`, `large-button-validate.png`).
//! Résultat : refaire à chaque fois un travail déjà fait, et aucun endroit où corriger un défaut
//! une seule fois pour toute l'application.
//!
//! Répartition des rôles, à respecter :
//!
//! | Couche | Rôle | Ne fait PAS |
//! | --- | --- | --- |
//! | `design::assets` | manifeste : nom logique → fichier + découpage 9-slice | de la peinture |
//! | `design::nine_slice` | peindre une texture à n'importe quelle taille | connaître un composant |
//! | `design::fonts` | enregistrer les polices embarquées auprès d'egui | mettre en page |
//! | `design::text` | le corps et la police d'un libellé | choisir quoi écrire |
//! | `design::tokens` | couleurs et métriques mesurées, partagées | de la mise en page |
//! | `design::components` | un composant = une API paramétrable + son comportement | de la logique métier |
//! | `panels::*` | mise en page, état applicatif | dessiner un bouton à la main |
//!
//! Un panneau qui a besoin d'un bouton écrit `ui.add(design::button("Valider"))`. S'il lui faut
//! quelque chose qu'aucun composant ne sait faire, la réponse est **d'étendre le composant**, pas
//! de repeindre à côté — c'est exactement ce que le skill `.claude/skills/ui-component/` outille.
//!
//! Chargement des textures : paresseux et mémorisé par `egui::Context` (`DesignSystem::get`). Aucun
//! câblage à faire dans `render_content`/`main.rs`, et rien n'est téléversé sur le GPU tant qu'aucun
//! composant n'est utilisé — le budget mémoire (300 Mo, §8 du plan) ne paie que ce qui est affiché.

pub mod assets;
pub mod components;
pub mod fonts;
pub mod icons;
pub mod nine_slice;
pub mod text;
pub mod tokens;

use std::sync::Arc;

pub use assets::DsTexture;
pub use components::button::{button, Button, ButtonSize, ButtonState, ButtonVariant};
pub use components::checkbox::{checkbox, Checkbox, CheckboxState};
pub use icons::DsIcon;
// Les trois fonctions de géométrie du repliable sont préfixées à la réexportation : `design::
// closed_height` ne dirait pas de quoi, et le jour où un second conteneur en aura une, le nom nu
// serait déjà pris.
pub use components::autocomplete::{
    autocomplete, Autocomplete, AutocompleteEntry, AutocompleteFilter, AutocompleteOutcome,
};
pub use components::collapsible::{
    closed_height as collapsible_closed_height, collapsible,
    content_width as collapsible_content_width, open_height as collapsible_open_height,
    Collapsible,
};
pub use components::heading::{heading, Heading};
pub use components::icon::{icon, Icon};
pub use components::icon_button::{icon_button, IconButton, IconButtonState, IconContext};
pub use components::info_text::{info_text, InfoText, InfoTone};
pub use components::input::{input, Input, InputSize, InputState};
pub use components::item_slot::{
    item_slot, paint_order as item_slot_paint_order, ItemRarity, ItemSlot, SlotCount, SlotFrame,
    SlotLayer,
};
pub use components::loader::{loader, Loader, LoaderSize};
pub use components::meter::{
    fill_corners as meter_fill_corners, meter, paint as paint_meter, Meter,
};
pub use components::pagination::{
    arrows_enabled as pagination_arrows_enabled, pagination, Pagination, PaginationOutcome,
    PaginationStep,
};
pub use components::panel::{panel, Panel, PanelZones};
pub use components::portrait::{
    paint as paint_portrait, paint_percent as paint_portrait_percent,
    percent_of as portrait_percent, portrait, Portrait, PortraitShape,
};
pub use components::scroll_area::{scroll_area, ScrollArea};
pub use components::select::{select, select_multi, Select, SelectState};
pub use components::separator::{separator, Separator};
pub use components::slider::{
    slider, snap_to_step as slider_snap_to_step, track_travel as slider_track_travel, Slider,
    SliderState,
};
pub use components::stepper::{stepper, Stepper};
pub use components::table::{
    column_spans as table_column_spans, is_striped as table_is_striped, table, Table, TableAlign,
    TableBody, TableColumn, TableRow, TableWidth,
};
pub use components::tabs::{tabs, TabState, Tabs};
pub use components::tooltip::{tooltip, Tooltip, TooltipSide};
pub use components::window::{window, FooterClick, Window, WindowChrome};

/// Identifiant de mémorisation dans `egui::Context` — voir `DesignSystem::get`.
const MEMO_ID: &str = "wakfu-overlay-design-system";

struct Inner {
    /// Indexé par `DsTexture::index()` — un `Vec` plutôt qu'une `HashMap` : le manifeste est une
    /// table statique de taille connue, une table de hachage n'apporterait qu'un coût de hachage
    /// à chaque frame.
    textures: Vec<egui::TextureHandle>,
    /// Indexé par `DsIcon::index()`. **Deux tables et non une**, parce que ce sont deux registres :
    /// un fond se peint en 9-slice, un glyphe se met à l'échelle. Voir `design::icons`.
    icons: Vec<egui::TextureHandle>,
}

/// Poignée sur les textures du design system. `Clone` bon marché (un `Arc`) : les composants la
/// récupèrent par `DesignSystem::get(ui.ctx())` à chaque frame sans surcoût mesurable.
#[derive(Clone)]
pub struct DesignSystem {
    inner: Arc<Inner>,
}

impl DesignSystem {
    /// Récupère l'instance associée à `ctx`, en la chargeant au premier appel.
    ///
    /// La mémorisation passe par `Context::data` (données temporaires, jamais sérialisées). Le test
    /// « présent ? » et le chargement sont **séparés volontairement** : `Context::data_mut` prend le
    /// verrou d'écriture du contexte, et `Context::load_texture` le prend aussi — les imbriquer
    /// interbloquerait. Charger hors du verrou puis insérer coûte, dans le pire cas (deux appels
    /// concurrents au tout premier frame), un double chargement inoffensif.
    pub fn get(ctx: &egui::Context) -> Self {
        let id = egui::Id::new(MEMO_ID);
        if let Some(existing) = ctx.data(|d| d.get_temp::<DesignSystem>(id)) {
            return existing;
        }
        let loaded = Self::load(ctx);
        ctx.data_mut(|d| d.insert_temp(id, loaded.clone()));
        loaded
    }

    /// Téléverse toutes les textures du manifeste. Appelé par `get` ; public pour un harnais qui
    /// voudrait maîtriser le moment du chargement.
    pub fn load(ctx: &egui::Context) -> Self {
        let textures = DsTexture::ALL
            .iter()
            .map(|texture| {
                let spec = texture.spec();
                upload(ctx, spec.name, spec.bytes)
            })
            .collect();
        let icons = DsIcon::ALL
            .iter()
            .map(|icon| {
                let spec = icon.spec();
                upload(ctx, spec.name, spec.bytes)
            })
            .collect();
        tracing::debug!(
            textures = DsTexture::ALL.len(),
            icones = DsIcon::ALL.len(),
            "design system chargé (textures téléversées)"
        );
        Self {
            inner: Arc::new(Inner { textures, icons }),
        }
    }

    /// Poignée sur le glyphe `icon`.
    pub fn icon(&self, icon: DsIcon) -> &egui::TextureHandle {
        &self.inner.icons[icon.index()]
    }

    /// Taille native d'un glyphe, en pixels — celle à laquelle il a été détouré.
    pub fn icon_native_size(&self, icon: DsIcon) -> egui::Vec2 {
        self.icon(icon).size_vec2()
    }

    /// Peint un glyphe dans `rect`, à l'échelle uniforme.
    ///
    /// Pas de 9-slice ici, et c'est tout l'intérêt d'avoir deux registres : figer des coins sur un
    /// glyphe de 27 px le déformerait dès qu'on le peint à 12, et il n'y a rien de périodique à
    /// répéter. C'est à l'appelant de lui donner un rectangle au bon rapport — voir
    /// `components::icon_button::glyph_fit`, le seul endroit qui calcule ce rapport.
    pub fn paint_icon(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        icon: DsIcon,
        tint: egui::Color32,
    ) {
        self.paint_icon_flipped(painter, rect, icon, tint, false);
    }

    /// Peint un glyphe, éventuellement **retourné horizontalement**.
    ///
    /// Le miroir existe parce que le jeu réutilise un même glyphe dans ses deux sens : les deux
    /// flèches de sa pagination sont le même triangle, l'une étant le reflet de l'autre. Le
    /// retournement se fait dans les coordonnées de texture — aucun second fichier, donc aucun
    /// doublon à retraiter le jour où le glyphe change.
    ///
    /// À n'employer que sur un glyphe **symétrique verticalement** : le miroir d'un « 2 » reste un
    /// miroir, pas un caractère.
    pub fn paint_icon_flipped(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        icon: DsIcon,
        tint: egui::Color32,
        mirrored: bool,
    ) {
        let (left, right) = if mirrored { (1.0, 0.0) } else { (0.0, 1.0) };
        let mut mesh = egui::Mesh::with_texture(self.icon(icon).id());
        mesh.add_rect_with_uv(
            rect,
            egui::Rect::from_min_max(egui::pos2(left, 0.0), egui::pos2(right, 1.0)),
            tint,
        );
        painter.add(egui::Shape::mesh(mesh));
    }

    pub fn texture(&self, texture: DsTexture) -> &egui::TextureHandle {
        &self.inner.textures[texture.index()]
    }

    /// Taille native de la texture, en pixels — la taille à laquelle elle a été capturée dans le
    /// jeu, donc la référence des gabarits (voir `components::button::ButtonSize`).
    pub fn native_size(&self, texture: DsTexture) -> egui::Vec2 {
        self.texture(texture).size_vec2()
    }

    /// Peint une texture du manifeste dans `rect`, en 9-slice, avec le découpage déclaré pour elle.
    /// C'est le seul chemin que doivent emprunter les composants — jamais `egui::Image::paint_at`,
    /// qui déformerait les coins.
    /// Pose la texture dans un emplacement **réservé plus tôt** par `Painter::add`.
    ///
    /// Un conteneur dont le cadre enveloppe un contenu de hauteur inconnue ne peut pas peindre son
    /// décor au moment où il le rencontre : il le poserait par-dessus son propre contenu. L'idiome
    /// egui est de réserver un emplacement, de mesurer, puis d'y écrire — c'est ce que fait
    /// `design::collapsible`.
    pub fn paint_to_slot(
        &self,
        painter: &egui::Painter,
        slot: egui::layers::ShapeIdx,
        rect: egui::Rect,
        texture: DsTexture,
        tint: egui::Color32,
    ) {
        painter.set(
            slot,
            nine_slice::shape(rect, self.texture(texture), &texture.spec().slice, tint),
        );
    }

    /// Peint la texture **retournée verticalement**.
    ///
    /// Sert aux glyphes dont le jeu n'a qu'une orientation : le chevron d'un bloc repliable pointe
    /// vers le bas fermé et vers le haut ouvert, et le manifeste n'en porte qu'un. En fabriquer un
    /// second serait un asset par état, ce que le contrat interdit au même titre qu'un asset par
    /// taille.
    pub fn paint_flipped_y(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        icon: DsIcon,
        tint: egui::Color32,
    ) {
        let mut mesh = egui::Mesh::with_texture(self.icon(icon).id());
        // UV inversées sur l'axe vertical : v0 en bas, v1 en haut.
        mesh.add_rect_with_uv(
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 1.0), egui::pos2(1.0, 0.0)),
            tint,
        );
        painter.add(egui::Shape::mesh(mesh));
    }

    /// Peint une **région** de la texture (`uv`, en coordonnées `0..1`) dans `rect`, à l'échelle
    /// uniforme.
    ///
    /// C'est le chemin d'une planche d'images (`DsTexture::LoaderSheet`) : le manifeste porte une
    /// texture par rôle, et une animation est un rôle — seize variantes `LoaderFrame00…15` seraient
    /// seize lignes pour dire la même chose. Le 9-slice ne s'applique pas : une cellule de planche
    /// n'a ni coin ni liseré à figer, elle se réduit comme un glyphe.
    pub fn paint_region(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        texture: DsTexture,
        uv: egui::Rect,
        tint: egui::Color32,
    ) {
        let mut mesh = egui::Mesh::with_texture(self.texture(texture).id());
        mesh.add_rect_with_uv(rect, uv, tint);
        painter.add(egui::Shape::mesh(mesh));
    }

    pub fn paint(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        texture: DsTexture,
        tint: egui::Color32,
    ) {
        nine_slice::paint(
            painter,
            rect,
            self.texture(texture),
            &texture.spec().slice,
            tint,
        );
    }
}

/// Décode un PNG embarqué et le téléverse sur le GPU sous le nom de cache `name`.
///
/// Partagée par les deux registres : le décodage ne dépend pas de ce que la texture représente.
fn upload(ctx: &egui::Context, name: &'static str, bytes: &'static [u8]) -> egui::TextureHandle {
    let decoded = image::load_from_memory(bytes)
        .expect("texture du design system invalide — asset corrompu au build")
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        decoded.as_raw(),
    );
    ctx.load_texture(name, image, egui::TextureOptions::LINEAR)
}
