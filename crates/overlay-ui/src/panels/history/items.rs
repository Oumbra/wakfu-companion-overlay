//! Briques communes aux listes de l'historique (Achats, Échanges, Pactes) : barre de recherche et
//! de tri (`app-history-list-header` du site), en-tête de jour repliable, regroupement par jour,
//! et ligne d'objet avec son icône et le fond de sa rareté.

use std::collections::BTreeSet;

use chrono::NaiveDate;
use egui::{Align2, Color32, Rect, Sense, Ui};
use overlay_engine::CatalogIndex;
use overlay_history::read::ItemLine;
use overlay_history::{HistoryError, HistoryStore};

use super::{format, HistoryContext};
use crate::design::{self, text, tokens, DsIcon};
use crate::rarity_bridge::rarity_color;
use crate::remote_icons::RemoteIconTextures;

/// Taille de police des lignes de liste.
pub(crate) const FONT_SIZE: f32 = 14.0;
const SORT_SWITCH_WIDTH: f32 = 72.0;
pub(super) const DAY_HEADER_HEIGHT: f32 = 30.0;

/// `--btn-active-green` du site : kamas et objets acquis.
pub(crate) const GAIN: Color32 = Color32::from_rgb(0x2E, 0xCC, 0x71);
/// `--btn-active-red` du site : kamas et objets cédés.
pub(crate) const LOSS: Color32 = Color32::from_rgb(0xE7, 0x4C, 0x3C);
/// `--accent` du site : nombre d'entrées d'un jour.
pub(super) const ACCENT: Color32 = tokens::OVERLAY_ACCENT;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// Plus récent en premier (défaut du site).
    #[default]
    Newest,
    Oldest,
}

impl SortOrder {
    /// Compare deux instants dans l'ordre choisi.
    pub fn cmp_times(self, a: i64, b: i64) -> std::cmp::Ordering {
        match self {
            SortOrder::Newest => b.cmp(&a),
            SortOrder::Oldest => a.cmp(&b),
        }
    }
}

/// Ce qu'une liste garde entre deux frames : la saisie, le tri, les jours repliés, les lignes lues
/// dans la base et le chargement progressif.
pub struct ListState<T> {
    pub query: String,
    pub order: SortOrder,
    pub collapsed: BTreeSet<NaiveDate>,
    /// `None` : pas encore lu (ou à relire).
    pub rows: Option<Vec<T>>,
    pub pager: super::pager::Pager,
}

impl<T> Default for ListState<T> {
    fn default() -> Self {
        Self {
            query: String::new(),
            order: SortOrder::default(),
            collapsed: BTreeSet::new(),
            rows: None,
            pager: super::pager::Pager::default(),
        }
    }
}

impl<T> ListState<T> {
    /// Lit les lignes dans la base au premier affichage (ou après [`Self::rows`] remis à `None`).
    /// Une base absente ou illisible donne une liste vide.
    pub fn load(
        &mut self,
        store: Option<&HistoryStore>,
        read: impl FnOnce(&HistoryStore) -> Result<Vec<T>, HistoryError>,
        what: &str,
    ) {
        if self.rows.is_some() {
            return;
        }
        self.rows = Some(match store.map(read) {
            Some(Ok(rows)) => rows,
            Some(Err(err)) => {
                tracing::warn!(%err, what, "historique : liste illisible");
                Vec::new()
            }
            None => Vec::new(),
        });
    }

    /// Replie ou déplie un jour.
    pub fn toggle(collapsed: &mut BTreeSet<NaiveDate>, day: NaiveDate) {
        if !collapsed.remove(&day) {
            collapsed.insert(day);
        }
    }
}

/// La barre au-dessus d'une liste (`history-list-header` du site) : champ de recherche à loupe, puis
/// la bascule de tri en deux flèches (haut = plus récent en premier, bas = plus ancien).
pub fn list_header(
    ui: &mut Ui,
    zones: &design::PanelZones,
    query: &mut String,
    order: &mut SortOrder,
    placeholder: &str,
    log_name: &str,
) {
    ui.horizontal(|ui| {
        ui.add(
            design::input(query)
                .placeholder(placeholder)
                .leading_icon(design::DsIcon::Search)
                .width(zones.inner.width() - SORT_SWITCH_WIDTH - 12.0)
                .log_name(format!("{log_name}.recherche")),
        );
        ui.add(
            design::switch(order)
                .slot(SortOrder::Newest, "Plus récent en premier")
                .icon(design::DsIcon::ArrowUp)
                .slot(SortOrder::Oldest, "Plus ancien en premier")
                .icon(design::DsIcon::ArrowDown)
                .width(SORT_SWITCH_WIDTH)
                .log_name(format!("{log_name}.tri")),
        );
    });
    ui.add_space(8.0);
}

/// Ce qu'un en-tête de jour montre à droite.
pub enum DayTail<'a> {
    /// Total dépensé (Achats).
    Kamas(&'a str),
    /// Nombre d'entrées (Échanges, Pactes).
    Count(usize),
    /// Un libellé (« 3 combats »).
    Text(&'a str),
}

/// En-tête de jour cliquable : date à gauche, total ou nombre et chevron à droite. Rend `true`
/// au clic.
pub fn day_header(ui: &mut Ui, width: f32, day: &str, tail: DayTail<'_>, open: bool) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, DAY_HEADER_HEIGHT), Sense::click());
    let painter = ui.painter_at(rect);
    let fill = if response.hovered() {
        Color32::from_rgba_unmultiplied(255, 255, 255, 18)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 10)
    };
    painter.rect_filled(rect, 4.0, fill);
    let font = text::label_strong_font(ui.ctx(), FONT_SIZE + 1.0);
    painter.text(
        rect.left_center() + egui::vec2(10.0, 0.0),
        Align2::LEFT_CENTER,
        day,
        font.clone(),
        tokens::TEXT_GOLD,
    );
    let tail_right = egui::pos2(rect.right() - 34.0, rect.center().y);
    match tail {
        DayTail::Kamas(total) => {
            super::paint_amount(ui, &painter, tail_right, total, font, tokens::TEXT_GOLD);
        }
        DayTail::Count(count) => {
            painter.text(
                tail_right,
                Align2::RIGHT_CENTER,
                format::thousands(count as i64),
                font,
                ACCENT,
            );
        }
        DayTail::Text(label) => {
            painter.text(tail_right, Align2::RIGHT_CENTER, label, font, ACCENT);
        }
    }
    super::paint_caret(
        ui,
        &painter,
        egui::pos2(rect.right() - 14.0, rect.center().y),
        open,
        tokens::TEXT_GOLD,
    );
    response.clicked()
}

/// Les entrées regroupées par jour local, dans l'ordre choisi (jours et entrées d'un jour) —
/// `groups` des composants Échanges et Pacte du site.
pub fn group_by_day<'a, T>(
    records: impl IntoIterator<Item = &'a T>,
    time: impl Fn(&T) -> i64,
    order: SortOrder,
) -> Vec<(NaiveDate, Vec<&'a T>)> {
    let mut by_day: Vec<(NaiveDate, Vec<&T>)> = Vec::new();
    for record in records {
        let day = format::local_day(time(record));
        match by_day.iter_mut().find(|(d, _)| *d == day) {
            Some((_, list)) => list.push(record),
            None => by_day.push((day, vec![record])),
        }
    }
    for (_, list) in &mut by_day {
        list.sort_by(|a, b| order.cmp_times(time(a), time(b)));
    }
    by_day.sort_by(|(_, a), (_, b)| {
        let a_time = a.first().map_or(0, |record| time(record));
        let b_time = b.first().map_or(0, |record| time(record));
        order.cmp_times(a_time, b_time)
    });
    by_day
}

/// Le nom affiché d'un objet : celui du référentiel par identifiant, sinon celui du log.
pub fn item_name(line: &ItemLine, catalog: &CatalogIndex) -> String {
    line.item_id
        .and_then(|id| catalog.item_name(id))
        .map(str::to_string)
        .or_else(|| line.item_name.clone())
        .unwrap_or_else(|| "Objet inconnu".to_string())
}

/// Le fond de rareté d'une ligne d'objet.
#[derive(Debug, Clone, Copy)]
pub struct RarityStyle {
    /// Largeur du liseré gauche.
    pub edge: f32,
    /// Opacité de la couleur au départ du dégradé.
    pub strength: f32,
    /// Fin du dégradé, en part de la largeur.
    pub fade: f32,
    /// Filet haut qui s'efface avec le dégradé (`.purchase-row`).
    pub top_line: bool,
}

/// `.purchase-row[class*='rarity-']` : liseré 3 px, 35 % jusqu'à 70 %, filet haut.
pub const PURCHASE_RARITY: RarityStyle = RarityStyle {
    edge: 3.0,
    strength: 0.35,
    fade: 0.7,
    top_line: true,
};

/// `.trade-item-row` / `.pact-item-row` : liseré 2 px, 30 % jusqu'à 75 %.
pub const COMPACT_RARITY: RarityStyle = RarityStyle {
    edge: 2.0,
    strength: 0.30,
    fade: 0.75,
    top_line: false,
};

/// Liseré gauche de la couleur de rareté et dégradé de cette couleur vers le fond.
pub fn paint_rarity_background(
    painter: &egui::Painter,
    rect: Rect,
    rarity: Color32,
    style: RarityStyle,
) {
    let fade_x = rect.left() + rect.width() * style.fade;
    let horizontal_fade = |top: f32, bottom: f32, from: Color32| {
        let mut mesh = egui::Mesh::default();
        let clear = Color32::TRANSPARENT;
        for (x, color) in [(rect.left(), from), (fade_x, clear)] {
            mesh.colored_vertex(egui::pos2(x, top), color);
            mesh.colored_vertex(egui::pos2(x, bottom), color);
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(1, 3, 2);
        mesh
    };
    painter.add(horizontal_fade(
        rect.top(),
        rect.bottom(),
        rarity.gamma_multiply(style.strength),
    ));
    if style.top_line {
        painter.add(horizontal_fade(rect.top(), rect.top() + 1.0, rarity));
    }
    painter.rect_filled(
        Rect::from_min_max(
            rect.min,
            egui::pos2(rect.left() + style.edge, rect.bottom()),
        ),
        0.0,
        rarity,
    );
}

/// Dimensions d'une ligne d'objet compacte (échanges, pactes).
#[derive(Debug, Clone, Copy)]
pub struct CompactRow {
    pub height: f32,
    pub pad_x: f32,
    pub gap: f32,
    pub font_size: f32,
}

/// Côté de l'icône d'une ligne compacte (`app-item-icon [size]="20"`).
const COMPACT_ICON: f32 = 20.0;

/// Une ligne d'objet compacte dans `rect` : fond de rareté, icône, nom, `×quantité` à droite.
pub fn paint_compact_item(
    ui: &Ui,
    painter: &egui::Painter,
    rect: Rect,
    line: &ItemLine,
    dims: CompactRow,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let name = item_name(line, ctx.catalog);
    paint_rarity_background(
        painter,
        rect,
        rarity_color(ctx.catalog.find_item_rarity(&name, line.item_id)),
        COMPACT_RARITY,
    );
    let icon_rect = Rect::from_center_size(
        egui::pos2(
            rect.left() + dims.pad_x + COMPACT_ICON / 2.0,
            rect.center().y,
        ),
        egui::vec2(COMPACT_ICON, COMPACT_ICON),
    );
    if let Some(icon) = ctx.catalog.find_item_icon(&name, line.item_id) {
        super::paint_remote_icon(ui, painter, icon_rect, &icon, ctx.remote_icons, textures);
    }
    let qty_font = text::label_strong_font(ui.ctx(), dims.font_size - 1.0);
    let qty = painter.text(
        egui::pos2(rect.right() - dims.pad_x, rect.center().y),
        Align2::RIGHT_CENTER,
        format!("×{}", format::thousands(line.quantity)),
        qty_font,
        Color32::WHITE,
    );
    let name_rect = Rect::from_min_max(
        egui::pos2(icon_rect.right() + dims.gap, rect.top()),
        egui::pos2(qty.left() - dims.gap, rect.bottom()),
    );
    painter.with_clip_rect(name_rect.intersect(rect)).text(
        name_rect.left_center(),
        Align2::LEFT_CENTER,
        name,
        text::label_font(ui.ctx(), dims.font_size),
        Color32::WHITE,
    );
}

/// L'horloge d'une entrée, avec la date et l'heure complètes au survol (`formatDateTime`).
pub fn entry_clock(ui: &Ui, center: egui::Pos2, occurred_at: i64, id: egui::Id) {
    let rect = Rect::from_center_size(center, egui::vec2(16.0, 16.0));
    super::paint_icon(
        ui,
        ui.painter(),
        center,
        DsIcon::Clock,
        15.0,
        tokens::OVERLAY_TEXT_MUTED,
    );
    ui.interact(rect, id, Sense::hover())
        .on_hover_text(format::date_time(occurred_at));
}
