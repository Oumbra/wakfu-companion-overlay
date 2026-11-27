//! **Fenêtre d'historique** (2026-11-27, `docs/plan-historique-local.md` §7) : l'historique local
//! (`overlay_history`) en six onglets — Combats, Récap, Achats, Échanges, Pactes, À propos —, dans
//! une fenêtre application normale. Remplace les pages d'historique du site.
//!
//! Ce module ne connaît ni `winit` ni le GPU : [`show`] peint dans le `Ui` qu'on lui donne, ce qui
//! permet au harnais de rendu (`overlay-testkit`) de le dessiner hors écran sur une vraie base.

pub mod achats;
pub mod format;

use chrono::NaiveDate;
use egui::Ui;
use overlay_engine::CatalogIndex;
use overlay_history::HistoryStore;

use crate::design::{self, DsIcon};

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
    }
}

/// Ce dont la fenêtre a besoin pour se peindre.
pub struct HistoryContext<'a> {
    /// `None` : base absente ou illisible — les vues se montrent vides.
    pub store: Option<&'a HistoryStore>,
    pub catalog: &'a CatalogIndex,
    /// Jour local courant (« Aujourd'hui », « Hier »…), fourni par l'hôte pour que les captures
    /// soient reproductibles.
    pub today: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HistoryAction {
    #[default]
    None,
    Close,
}

pub fn show(
    ui: &mut Ui,
    state: &mut HistoryWindowState,
    ctx: &HistoryContext<'_>,
) -> HistoryAction {
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
        HistoryTab::Achats => achats::show(
            ui,
            zones,
            &mut state.achats,
            ctx.store,
            ctx.catalog,
            ctx.today,
        ),
        other => {
            ui.add(
                design::label(format!("{} — à venir", other.label()))
                    .color(design::tokens::TEXT_DISABLED),
            );
        }
    });
    if chrome.close {
        HistoryAction::Close
    } else {
        HistoryAction::None
    }
}
