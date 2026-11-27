//! **Fenêtre d'historique** (2026-11-27, `docs/plan-historique-local.md` §7) : l'historique local
//! (`overlay_history`) en six onglets — Combats, Récap, Achats, Échanges, Pactes, À propos —, dans
//! une fenêtre application normale. Remplace les pages d'historique du site.
//!
//! Ce module ne connaît ni `winit` ni le GPU : [`show`] peint dans le `Ui` qu'on lui donne, ce qui
//! permet au harnais de rendu (`overlay-testkit`) de le dessiner hors écran sur une vraie base.

pub mod achats;
pub mod echanges;
pub mod format;
pub mod items;
pub mod pactes;
pub mod pager;

use chrono::NaiveDate;
use egui::{Sense, Ui};
use overlay_engine::CatalogIndex;
use overlay_history::HistoryStore;

use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};

use crate::design::{self, DsIcon};

/// Titre système de la fenêtre (barre des tâches, Alt+Tab) — aussi ce que cherche une seconde
/// ouverture pour ramener la première au premier plan (`single_instance::history_guard`).
pub const WINDOW_TITLE: &str = "Wakfu Companion — Historique";

/// Fond opaque sous le décor — le ton sombre de la texture du corps.
const WINDOW_UNDERLAY: egui::Color32 = egui::Color32::from_rgb(0x15, 0x17, 0x1C);

/// Côté de l'icône Kamas qui suit un montant.
const KAMAS_ICON_SIZE: f32 = 14.0;

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

/// Les onglets, dans l'ordre de la barre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryTab {
    #[default]
    Combats,
    Recap,
    Achats,
    Echanges,
    Pactes,
    APropos,
}

impl HistoryTab {
    pub const ALL: [HistoryTab; 6] = [
        HistoryTab::Combats,
        HistoryTab::Recap,
        HistoryTab::Achats,
        HistoryTab::Echanges,
        HistoryTab::Pactes,
        HistoryTab::APropos,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HistoryTab::Combats => "Combats",
            HistoryTab::Recap => "Récap",
            HistoryTab::Achats => "Achats",
            HistoryTab::Echanges => "Échanges",
            HistoryTab::Pactes => "Pactes",
            HistoryTab::APropos => "À propos",
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
        self.echanges.rows = None;
        self.pactes.rows = None;
    }
}

/// Ce dont la fenêtre a besoin pour se peindre.
pub struct HistoryContext<'a> {
    /// `None` : base absente ou illisible — les vues se montrent vides.
    pub store: Option<&'a HistoryStore>,
    pub catalog: &'a CatalogIndex,
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
}

impl HistoryHost {
    /// Ouvre la base de l'overlay (`HistoryStore::default_path`) ; une base absente ou illisible
    /// laisse la fenêtre s'ouvrir, vide.
    pub fn open(tab: HistoryTab) -> Self {
        Self::open_path(tab, HistoryStore::default_path())
    }

    /// Sur une autre base que celle de l'overlay — la copie de l'essai automatique
    /// (`crate::autotest`).
    pub fn open_path(
        tab: HistoryTab,
        path: Result<std::path::PathBuf, overlay_history::HistoryError>,
    ) -> Self {
        let store = path
            .and_then(|path| HistoryStore::open(&path))
            .map_err(|err| tracing::warn!(%err, "historique : base inaccessible"))
            .ok();
        Self {
            state: HistoryWindowState::new(tab),
            store,
        }
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
        painter.image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
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
        HistoryTab::Achats => achats::show(ui, zones, &mut state.achats, ctx, textures),
        HistoryTab::Echanges => echanges::show(ui, zones, &mut state.echanges, ctx, textures),
        HistoryTab::Pactes => pactes::show(ui, zones, &mut state.pactes, ctx, textures),
        other => {
            ui.add(
                design::label(format!("{} — à venir", other.label()))
                    .color(design::tokens::TEXT_DISABLED),
            );
        }
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
