//! **Fenêtre d'historique** (2026-11-27, `docs/plan-historique-local.md` §7) : l'historique local
//! (`overlay_history`) en six onglets — Combats, Récap, Achats, Échanges, Pactes, À propos —, dans
//! une fenêtre application normale. Remplace les pages d'historique du site.
//!
//! Ce module ne connaît ni `winit` ni le GPU : [`show`] peint dans le `Ui` qu'on lui donne, ce qui
//! permet au harnais de rendu (`overlay-testkit`) de le dessiner hors écran sur une vraie base.

pub mod achats;
pub mod calendar;
pub mod combats;
pub mod echanges;
pub mod format;
pub mod items;
pub mod pactes;
pub mod pager;
pub mod recap;

use chrono::NaiveDate;
use egui::{Sense, Ui};
use overlay_engine::{CatalogIndex, DungeonIndex};
use overlay_history::HistoryStore;

use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};

use crate::design::{self, DsIcon};

/// Titre système de la fenêtre (barre des tâches, Alt+Tab) — aussi ce que cherche une seconde
/// ouverture pour ramener la première au premier plan (`single_instance::history_guard`).
pub const WINDOW_TITLE: &str = "Wakfu Companion — Historique";

/// Fond opaque sous le décor — le ton sombre de la texture du corps.
const WINDOW_UNDERLAY: egui::Color32 = egui::Color32::from_rgb(0x15, 0x17, 0x1C);

/// Côté de l'icône Kamas qui suit un montant.
pub(super) const KAMAS_ICON_SIZE: f32 = 14.0;

/// Peint un glyphe du design system centré sur `center`, à `size` de côté (rapport conservé).
pub(crate) fn paint_icon(
    ui: &Ui,
    painter: &egui::Painter,
    center: egui::Pos2,
    icon: DsIcon,
    size: f32,
    tint: egui::Color32,
) {
    let ds = design::DesignSystem::get(ui.ctx());
    let drawn = design::components::icon_button::glyph_fit(ds.icon_native_size(icon), size);
    ds.paint_icon(
        painter,
        egui::Rect::from_center_size(center, drawn),
        icon,
        tint,
    );
}

/// Le chevron d'une ligne repliable centré sur `center` : `⌄` dépliée, `›` repliée — le même trait
/// tourné d'un quart de tour, partout dans la fenêtre (retour utilisateur du 2026-11-27).
pub(crate) fn paint_caret(
    ui: &Ui,
    painter: &egui::Painter,
    center: egui::Pos2,
    open: bool,
    tint: egui::Color32,
) {
    let icon = if open {
        DsIcon::ChevronDown
    } else {
        DsIcon::ChevronRight
    };
    paint_icon(ui, painter, center, icon, CARET_SIZE, tint);
}

/// Une **pastille** (`.fight-group-count`, `.dungeon-run-meta` du site) : texte clair sur fond
/// arrondi, bord droit en `right_center`. Rend son rectangle.
pub(crate) fn paint_pill(
    ui: &Ui,
    painter: &egui::Painter,
    right_center: egui::Pos2,
    label: &str,
) -> egui::Rect {
    let galley = painter.layout_no_wrap(
        label.to_string(),
        design::text::label_strong_font(ui.ctx(), PILL_TEXT),
        PILL_COLOR,
    );
    let size = galley.size() + egui::vec2(2.0 * PILL_PAD_X, 2.0 * PILL_PAD_Y);
    let rect = egui::Rect::from_min_size(
        egui::pos2(right_center.x - size.x, right_center.y - size.y / 2.0),
        size,
    );
    painter.rect_filled(rect, size.y / 2.0, PILL_FILL);
    painter.galley(
        rect.min + egui::vec2(PILL_PAD_X, PILL_PAD_Y),
        galley,
        PILL_COLOR,
    );
    rect
}

/// Pastille : corps (0,7 rem à l'échelle), marges (`padding: 1px 8px`), texte `--text-color`
/// sur `--surface-raised`.
const PILL_TEXT: f32 = 11.5;
const PILL_PAD_X: f32 = 8.0;
const PILL_PAD_Y: f32 = 2.0;
const PILL_COLOR: egui::Color32 = egui::Color32::from_rgb(0xE0, 0xE0, 0xE0);
const PILL_FILL: egui::Color32 = egui::Color32::from_rgb(0x26, 0x26, 0x26);

/// Côté du chevron d'une ligne repliable.
const CARET_SIZE: f32 = 12.0;

/// Hauteur commune des contrôles d'une ligne d'outils : un switch, un champ de recherche, un
/// bouton icône — le socle de premier plan fait 36 (`.icon-switch` et `app-loot-search` du site
/// partagent de même leurs 32 px).
pub(crate) const CONTROL_HEIGHT: f32 = 36.0;
/// Corps des libellés de switch (`.icon-switch-option` du site, 0,68 à 0,72 rem en gras, à
/// l'échelle de la fenêtre).
pub(crate) const SWITCH_TEXT: f32 = 12.0;
/// Côté d'un pictogramme posé à côté d'un libellé de switch (`.icon-switch-option svg`, 13 px) —
/// et du glyphe des boutons icônes voisins.
pub(crate) const SWITCH_ICON: f32 = 13.0;
/// Corps du texte d'un champ de recherche (`.search-input`, 0,78 à 0,82 rem).
const SEARCH_TEXT: f32 = 14.0;
/// Côté de la loupe d'un champ de recherche.
const SEARCH_ICON: f32 = 15.0;

/// **Le champ de recherche de la fenêtre** (`app-loot-search`, `.search-input` du site) : loupe
/// dans le champ, verre à gauche ; texte indicatif gris ; à la hauteur des switches voisins.
/// L'appelant ajoute largeur, croix d'effacement et nom de journal.
pub(crate) fn search_field<'a>(query: &'a mut String, placeholder: &str) -> design::Input<'a> {
    design::input(query)
        .placeholder(placeholder)
        .leading_icon(DsIcon::Search)
        .leading_icon_mirrored(false)
        .leading_icon_side(SEARCH_ICON)
        .box_height(CONTROL_HEIGHT)
        .text_size(SEARCH_TEXT)
        .placeholder_color(design::tokens::OVERLAY_TEXT_MUTED)
}

/// **Le switch de la fenêtre** : le socle bleu de premier plan (celui de Dégâts / Armure / Soins)
/// et les libellés au corps du site. Les menus, eux, sont des onglets (`design::tabs`).
pub(crate) fn switch<T: PartialEq + Copy>(value: &mut T) -> design::Switch<'_, T> {
    design::switch(value)
        .variant(design::SwitchVariant::FirstPlan)
        .text_size(SWITCH_TEXT)
        .beside_icon_size(SWITCH_ICON)
}

/// Un montant en kamas aligné à droite sur `right` : le nombre, puis l'icône Kamas — la police du
/// design system n'a pas le symbole ₭. Rend le bord gauche du texte.
pub(crate) fn paint_amount(
    ui: &Ui,
    painter: &egui::Painter,
    right: egui::Pos2,
    amount: &str,
    font: egui::FontId,
    color: egui::Color32,
) -> f32 {
    paint_icon(
        ui,
        painter,
        egui::pos2(right.x - KAMAS_ICON_SIZE / 2.0, right.y),
        DsIcon::Kamas,
        KAMAS_ICON_SIZE,
        egui::Color32::WHITE,
    );
    let rect = painter.text(
        egui::pos2(right.x - KAMAS_ICON_SIZE - 4.0, right.y),
        egui::Align2::RIGHT_CENTER,
        amount,
        font,
        color,
    );
    rect.left()
}

/// Un montant en kamas qui commence en `left` : le nombre, puis l'icône Kamas. Rend le bord droit
/// de l'icône.
pub(crate) fn paint_amount_left(
    ui: &Ui,
    painter: &egui::Painter,
    left: egui::Pos2,
    amount: &str,
    font: egui::FontId,
    color: egui::Color32,
) -> f32 {
    let rect = painter.text(left, egui::Align2::LEFT_CENTER, amount, font, color);
    let icon_center = egui::pos2(rect.right() + 4.0 + KAMAS_ICON_SIZE / 2.0, left.y);
    paint_icon(
        ui,
        painter,
        icon_center,
        DsIcon::Kamas,
        KAMAS_ICON_SIZE,
        egui::Color32::WHITE,
    );
    icon_center.x + KAMAS_ICON_SIZE / 2.0
}

/// Noms des familles de monstres (`monsterFamilies` du référentiel embarqué), publiés une fois
/// par le thread Référentiel (`background::spawn_referentiel_thread`). Un `OnceLock` plutôt qu'un
/// `ArcSwap` de plus à faire passer par les deux hôtes : l'index ne change jamais après son
/// chargement, et seule la fenêtre d'historique le lit.
static MONSTER_FAMILIES: std::sync::OnceLock<overlay_engine::MonsterFamilyIndex> =
    std::sync::OnceLock::new();

/// Publie l'index des familles — sans effet après le premier appel.
pub fn publish_monster_families(index: overlay_engine::MonsterFamilyIndex) {
    let _ = MONSTER_FAMILIES.set(index);
}

/// Nom français d'une famille de monstres ; `None` avant le chargement du référentiel, ou pour un
/// id inconnu.
pub(crate) fn family_name(id: i64) -> Option<&'static str> {
    MONSTER_FAMILIES
        .get()?
        .find_by_id(id)
        .map(|family| family.fr.as_str())
}

/// Les onglets, dans l'ordre de la barre (demande de l'utilisateur, 2026-11-27 : le Récap
/// d'abord). Pas d'onglet « À propos » : il existe déjà dans les Options de l'overlay, et les
/// pages légales sont au menu de l'icône de notification — la fenêtre d'historique ne le redit pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryTab {
    #[default]
    Recap,
    Combats,
    Achats,
    Echanges,
    Pactes,
}

impl HistoryTab {
    pub const ALL: [HistoryTab; 5] = [
        HistoryTab::Recap,
        HistoryTab::Combats,
        HistoryTab::Achats,
        HistoryTab::Echanges,
        HistoryTab::Pactes,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HistoryTab::Combats => "Combats",
            HistoryTab::Recap => "Récap",
            HistoryTab::Achats => "Achats",
            HistoryTab::Echanges => "Échanges",
            HistoryTab::Pactes => "Pactes",
        }
    }

    /// Nom d'onglet passé en paramètre d'ouverture (`--historique=achats`, raccourcis) —
    /// insensible à la casse et aux accents.
    pub fn parse(value: &str) -> Option<Self> {
        let value = overlay_engine::roster::normalize_wakfu_name(value);
        Self::ALL
            .into_iter()
            .find(|tab| overlay_engine::roster::normalize_wakfu_name(tab.label()) == value)
    }
}

/// Tout l'état de la fenêtre : onglet courant et état de chaque vue.
#[derive(Default)]
pub struct HistoryWindowState {
    pub tab: HistoryTab,
    pub achats: achats::AchatsState,
    pub combats: combats::CombatsState,
    pub recap: recap::RecapState,
    pub echanges: echanges::EchangesState,
    pub pactes: pactes::PactesState,
}

impl HistoryWindowState {
    pub fn new(tab: HistoryTab) -> Self {
        Self {
            tab,
            ..Default::default()
        }
    }

    /// La base a changé (nouveaux événements, import) : chaque vue relira ses données.
    pub fn invalidate(&mut self) {
        self.achats.rows = None;
        self.combats.list.rows = None;
        self.combats.detail.spells.clear();
        self.combats.detail.fight_spells.clear();
        self.recap.invalidate();
        self.echanges.rows = None;
        self.pactes.rows = None;
    }
}

/// Ce dont la fenêtre a besoin pour se peindre.
pub struct HistoryContext<'a> {
    /// `None` : base absente ou illisible — les vues se montrent vides.
    pub store: Option<&'a HistoryStore>,
    pub catalog: &'a CatalogIndex,
    /// Référentiel des donjons (noms des runs de la vue Combats).
    pub dungeons: &'a DungeonIndex,
    /// Portraits de classe des alliés (vue Combats) ; `None` : icônes de monstre seulement.
    pub portraits: Option<&'a crate::portraits::PortraitAtlas>,
    /// Icônes d'objets (wakassets, puis le CDN de l'overlay), partagées par le processus.
    pub remote_icons: &'a RemoteIconStore,
    /// Jour local courant (« Aujourd'hui », « Hier »…), fourni par l'hôte pour que les captures
    /// soient reproductibles.
    pub today: NaiveDate,
}

/// Ce que la fenêtre demande à son hôte après une frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HistoryOutcome {
    /// La croix de la bannière.
    pub close: bool,
    /// Bouton pressé sur la bannière : l'hôte lance `Window::drag_window` (pas de décorations
    /// système, comme la fenêtre de connexion).
    pub drag_window: bool,
    /// Bouton pressé sur un bord : l'hôte lance `Window::drag_resize_window` dans ce sens.
    pub resize: Option<egui::ResizeDirection>,
}

/// Ce que l'hôte garde pour la fenêtre ouverte : son état et la base, ouverte avec elle et
/// refermée avec elle (un fichier SQLite ouvert ne peut pas être supprimé sous Windows).
pub struct HistoryHost {
    pub state: HistoryWindowState,
    pub store: Option<HistoryStore>,
    /// Référentiel des donjons (noms des runs), publié par le thread Référentiel.
    pub dungeons: std::sync::Arc<arc_swap::ArcSwap<DungeonIndex>>,
    /// Dernier `data_version` vu, et quand : voir [`Self::refresh_if_changed`].
    seen_version: Option<i64>,
    checked_at: Option<std::time::Instant>,
}

/// Délai entre deux regards sur la base, fenêtre ouverte.
pub const REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

impl HistoryHost {
    /// Ouvre la base de l'overlay (`HistoryStore::default_path`) ; une base absente ou illisible
    /// laisse la fenêtre s'ouvrir, vide.
    pub fn open(
        tab: HistoryTab,
        dungeons: std::sync::Arc<arc_swap::ArcSwap<DungeonIndex>>,
    ) -> Self {
        Self::open_path(tab, HistoryStore::default_path(), dungeons)
    }

    /// Sur une autre base que celle de l'overlay — la copie de l'essai automatique
    /// (`crate::autotest`).
    pub fn open_path(
        tab: HistoryTab,
        path: Result<std::path::PathBuf, overlay_history::HistoryError>,
        dungeons: std::sync::Arc<arc_swap::ArcSwap<DungeonIndex>>,
    ) -> Self {
        let store = path
            .and_then(|path| HistoryStore::open(&path))
            .map_err(|err| tracing::warn!(%err, "historique : base inaccessible"))
            .ok();
        let seen_version = store.as_ref().and_then(|store| store.data_version().ok());
        Self {
            state: HistoryWindowState::new(tab),
            store,
            dungeons,
            seen_version,
            checked_at: None,
        }
    }

    /// **Rafraîchissement** : l'overlay continue d'écrire dans la base pendant que la fenêtre est
    /// ouverte (un combat qui se termine, un achat). Au plus toutes les [`REFRESH_INTERVAL`], la
    /// fenêtre regarde `data_version` ; s'il a bougé, chaque vue relit ses données — la recherche,
    /// les groupes repliés et le nombre de jours chargés sont conservés. Vrai si elle a relu.
    pub fn refresh_if_changed(&mut self) -> bool {
        let now = std::time::Instant::now();
        if self
            .checked_at
            .is_some_and(|at| now.duration_since(at) < REFRESH_INTERVAL)
        {
            return false;
        }
        self.checked_at = Some(now);
        let Some(version) = self
            .store
            .as_ref()
            .and_then(|store| store.data_version().ok())
        else {
            return false;
        };
        if self.seen_version.replace(version) == Some(version) {
            return false;
        }
        tracing::info!("[historique] la base a changé : les vues relisent leurs données.");
        self.state.invalidate();
        true
    }
}

/// Épaisseur de la bande de redimensionnement le long des bords.
const RESIZE_BAND: f32 = 6.0;

/// Bords et coins de redimensionnement — la fenêtre n'a pas de décorations système.
fn resize_edges(ui: &mut Ui, outcome: &mut HistoryOutcome) {
    use egui::{CursorIcon, ResizeDirection as Dir};
    let rect = ui.max_rect();
    let band = RESIZE_BAND;
    let corner = band * 2.0;
    let zones = [
        (
            Dir::NorthWest,
            egui::Rect::from_min_size(rect.min, egui::vec2(corner, corner)),
            CursorIcon::ResizeNorthWest,
        ),
        (
            Dir::NorthEast,
            egui::Rect::from_min_size(
                egui::pos2(rect.right() - corner, rect.top()),
                egui::vec2(corner, corner),
            ),
            CursorIcon::ResizeNorthEast,
        ),
        (
            Dir::SouthWest,
            egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.bottom() - corner),
                egui::vec2(corner, corner),
            ),
            CursorIcon::ResizeSouthWest,
        ),
        (
            Dir::SouthEast,
            egui::Rect::from_min_size(
                rect.max - egui::vec2(corner, corner),
                egui::vec2(corner, corner),
            ),
            CursorIcon::ResizeSouthEast,
        ),
        (
            Dir::North,
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.top() + band)),
            CursorIcon::ResizeVertical,
        ),
        (
            Dir::South,
            egui::Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - band), rect.max),
            CursorIcon::ResizeVertical,
        ),
        (
            Dir::West,
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.left() + band, rect.bottom())),
            CursorIcon::ResizeHorizontal,
        ),
        (
            Dir::East,
            egui::Rect::from_min_max(egui::pos2(rect.right() - band, rect.top()), rect.max),
            CursorIcon::ResizeHorizontal,
        ),
    ];
    for (index, (direction, zone, cursor)) in zones.into_iter().enumerate() {
        let response = ui.interact(
            zone,
            ui.id().with(("historique-bord", index)),
            Sense::drag(),
        );
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(cursor);
        }
        if response.drag_started() && outcome.resize.is_none() {
            outcome.resize = Some(direction);
        }
    }
}

/// Une icône distante peinte dans `rect` ; rien tant qu'elle n'est pas arrivée (le thread des
/// icônes réveille la fenêtre quand elle l'est).
pub(crate) fn paint_remote_icon(
    ui: &Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: &overlay_engine::IconRef,
    store: &RemoteIconStore,
    textures: &mut RemoteIconTextures,
) {
    if let Some(texture) = textures.resolve(ui.ctx(), store, icon) {
        // Proportions gardées, image centrée (`object-fit: contain` du site) : une illustration
        // de donjon est en paysage, une icône de monstre carrée.
        let size = texture.size_vec2();
        let scale = (rect.width() / size.x).min(rect.height() / size.y);
        let drawn = egui::Rect::from_center_size(rect.center(), size * scale);
        painter.image(
            texture.id(),
            drawn,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
}

/// Comme [`paint_remote_icon`], mais l'image posée **en bas** de `rect` (tuiles de donjon du
/// Récap : `align-items: flex-end`, le personnage pose au sol).
pub(crate) fn paint_remote_icon_bottom(
    ui: &Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: &overlay_engine::IconRef,
    store: &RemoteIconStore,
    textures: &mut RemoteIconTextures,
) {
    if let Some(texture) = textures.resolve(ui.ctx(), store, icon) {
        let size = texture.size_vec2();
        let scale = (rect.width() / size.x).min(rect.height() / size.y);
        let drawn = size * scale;
        let min = egui::pos2(rect.center().x - drawn.x / 2.0, rect.bottom() - drawn.y);
        painter.image(
            texture.id(),
            egui::Rect::from_min_size(min, drawn),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
}

/// La vignette d'une famille de monstres ; `None` avant le chargement du référentiel, pour un id
/// inconnu ou une famille sans image.
pub(crate) fn family_picture(id: i64) -> Option<overlay_engine::IconRef> {
    MONSTER_FAMILIES.get()?.find_by_id(id)?.picture.clone()
}

pub fn show(
    ui: &mut Ui,
    state: &mut HistoryWindowState,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) -> HistoryOutcome {
    let mut outcome = HistoryOutcome::default();
    let window_rect = ui.max_rect();
    // Fenêtre application, pas modale posée sur le jeu : le corps du design system est teinté à
    // 92 % d'opacité (`WINDOW_BODY_TINT`), et le bureau transparaissait dessous — texte du
    // terminal lisible entre les onglets (constat sur la vraie fenêtre, 2026-11-27). Un fond
    // opaque, aux coins de la fenêtre, le bouche.
    ui.painter()
        .rect_filled(window_rect, design::tokens::WINDOW_RADIUS, WINDOW_UNDERLAY);
    let chrome = design::window("Historique")
        .close_button(true)
        .version(true)
        .log_name("historique")
        .show(ui);
    let mut tabs = design::tabs(&mut state.tab).log_name("historique-onglets");
    for tab in HistoryTab::ALL {
        tabs = tabs.entry(tab, tab.label());
    }
    chrome.tabs(ui, tabs);
    design::panel().show(ui, chrome.content, |ui, zones| match state.tab {
        HistoryTab::Combats => combats::show(ui, zones, &mut state.combats, ctx, textures),
        HistoryTab::Recap => recap::show(ui, zones, &mut state.recap, ctx, textures),
        HistoryTab::Achats => achats::show(ui, zones, &mut state.achats, ctx, textures),
        HistoryTab::Echanges => echanges::show(ui, zones, &mut state.echanges, ctx, textures),
        HistoryTab::Pactes => pactes::show(ui, zones, &mut state.pactes, ctx, textures),
    });
    outcome.close = chrome.close;
    // Poignée de déplacement : la bannière, croix exceptée (elle garde ses clics).
    let close_reserve =
        design::tokens::WINDOW_CLOSE_SIZE + 2.0 * design::tokens::WINDOW_CLOSE_MARGIN;
    let banner = egui::Rect::from_min_max(
        window_rect.min,
        egui::pos2(
            window_rect.right() - close_reserve,
            window_rect.top() + design::tokens::WINDOW_BANNER_HEIGHT,
        ),
    );
    if ui
        .interact(banner, ui.id().with("historique-banniere"), Sense::drag())
        .drag_started()
    {
        outcome.drag_window = true;
    }
    resize_edges(ui, &mut outcome);
    outcome
}
