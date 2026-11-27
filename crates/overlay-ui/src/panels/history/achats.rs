//! Vue **Achats** — port de `features/purchases/` (site) : recherche (objet ou date), tri récent /
//! ancien, groupes par jour avec le total dépensé ; dans un jour, achats cumulés par objet, la
//! récupération de kamas à l'Hôtel de Vente épinglée en tête.
//!
//! Le modèle ([`groups`]) est séparé du rendu : c'est lui que les tests comparent au site.

use std::collections::{BTreeSet, HashMap};

use chrono::NaiveDate;
use egui::{Align2, Color32, Rect, Sense, Ui};
use overlay_engine::roster::normalize_wakfu_name;
use overlay_engine::CatalogIndex;
use overlay_history::read::PurchaseRow;
use overlay_history::stats::HDV_KAMAS_SALE_ITEM;

use super::format;
use crate::design::{self, text, tokens, DsIcon};

/// Libellé d'une récupération de kamas (`purchases.hdvSource`).
pub const HDV_LABEL: &str = "Hôtel de vente";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// Plus récent en premier (défaut du site).
    #[default]
    Newest,
    Oldest,
}

/// Une ligne d'un jour : un objet, cumulé sur la journée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayRow {
    /// Nom affiché (catalogue par id, sinon nom du log ; [`HDV_LABEL`] pour une vente).
    pub name: String,
    pub item_id: Option<i64>,
    pub is_hdv_sale: bool,
    pub quantity: i64,
    pub total_cost: i64,
    last_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayGroup {
    pub day: NaiveDate,
    /// Dépense du jour — ventes à l'HDV exclues.
    pub total_cost: i64,
    pub rows: Vec<DayRow>,
}

fn display_name(row: &PurchaseRow, catalog: &CatalogIndex) -> String {
    if row.item_name.as_deref() == Some(HDV_KAMAS_SALE_ITEM) {
        return HDV_LABEL.to_string();
    }
    row.item_id
        .and_then(|id| catalog.item_name(id))
        .map(str::to_string)
        .or_else(|| row.item_name.clone())
        .unwrap_or_else(|| "Objet inconnu".to_string())
}

/// Les groupes affichés — même filtre, mêmes cumuls, même ordre que `PurchasesComponent.groups`.
pub fn groups(
    rows: &[PurchaseRow],
    query: &str,
    order: SortOrder,
    catalog: &CatalogIndex,
) -> Vec<DayGroup> {
    let query = normalize_wakfu_name(query);
    let mut by_day: Vec<(NaiveDate, Vec<(&PurchaseRow, String)>)> = Vec::new();
    for row in rows {
        let name = display_name(row, catalog);
        let day = format::local_day(row.occurred_at);
        if !query.is_empty()
            && !normalize_wakfu_name(&name).contains(&query)
            && !normalize_wakfu_name(&format::medium_date(day)).contains(&query)
        {
            continue;
        }
        match by_day.iter_mut().find(|(d, _)| *d == day) {
            Some((_, list)) => list.push((row, name)),
            None => by_day.push((day, vec![(row, name)])),
        }
    }
    let mut groups: Vec<DayGroup> = by_day
        .into_iter()
        .map(|(day, records)| {
            let mut rows: Vec<DayRow> = Vec::new();
            let mut index: HashMap<(String, Option<i64>), usize> = HashMap::new();
            let mut total_cost = 0;
            for (record, name) in records {
                let is_hdv_sale = record.item_name.as_deref() == Some(HDV_KAMAS_SALE_ITEM);
                if !is_hdv_sale {
                    total_cost += record.total_cost;
                }
                let key = (name.clone(), record.item_id);
                match index.get(&key) {
                    Some(&position) => {
                        let row = &mut rows[position];
                        row.quantity += record.quantity;
                        row.total_cost += record.total_cost;
                        row.last_ms = row.last_ms.max(record.occurred_at);
                    }
                    None => {
                        index.insert(key, rows.len());
                        rows.push(DayRow {
                            name,
                            item_id: record.item_id,
                            is_hdv_sale,
                            quantity: record.quantity,
                            total_cost: record.total_cost,
                            last_ms: record.occurred_at,
                        });
                    }
                }
            }
            rows.sort_by(|a, b| {
                b.is_hdv_sale.cmp(&a.is_hdv_sale).then(match order {
                    SortOrder::Newest => b.last_ms.cmp(&a.last_ms),
                    SortOrder::Oldest => a.last_ms.cmp(&b.last_ms),
                })
            });
            DayGroup {
                day,
                total_cost,
                rows,
            }
        })
        .collect();
    groups.sort_by(|a, b| {
        let a_time = a.rows.first().map_or(0, |row| row.last_ms);
        let b_time = b.rows.first().map_or(0, |row| row.last_ms);
        match order {
            SortOrder::Newest => b_time.cmp(&a_time),
            SortOrder::Oldest => a_time.cmp(&b_time),
        }
    });
    groups
}

/// État de la vue : la saisie, le tri, les jours repliés, et les lignes lues dans la base.
#[derive(Default)]
pub struct AchatsState {
    pub query: String,
    pub order: SortOrder,
    pub collapsed: BTreeSet<NaiveDate>,
    /// `None` : pas encore lu (ou à relire).
    pub rows: Option<Vec<PurchaseRow>>,
}

const ROW_HEIGHT: f32 = 26.0;
const SORT_SWITCH_WIDTH: f32 = 180.0;
const DAY_HEADER_HEIGHT: f32 = 30.0;
const FONT_SIZE: f32 = 14.0;
const HDV_GAIN: Color32 = Color32::from_rgb(0x8B, 0xD3, 0x6B);

/// Rendu dans la zone de contenu d'un panneau.
pub fn show(
    ui: &mut Ui,
    zones: &design::PanelZones,
    state: &mut AchatsState,
    store: Option<&overlay_history::HistoryStore>,
    catalog: &CatalogIndex,
    today: NaiveDate,
) {
    if state.rows.is_none() {
        state.rows = Some(match store.map(|store| store.purchases()) {
            Some(Ok(rows)) => rows,
            Some(Err(err)) => {
                tracing::warn!(%err, "historique : achats illisibles");
                Vec::new()
            }
            None => Vec::new(),
        });
    }
    ui.horizontal(|ui| {
        ui.add(
            design::input(&mut state.query)
                .placeholder("Rechercher un objet ou une date...")
                .clearable(true)
                .width(zones.inner.width() - SORT_SWITCH_WIDTH - 12.0)
                .log_name("historique.achats.recherche"),
        );
        ui.add(
            design::switch(&mut state.order)
                .slot(SortOrder::Newest, "Récent")
                .slot(SortOrder::Oldest, "Ancien")
                .width(SORT_SWITCH_WIDTH)
                .log_name("historique.achats.tri"),
        );
    });
    ui.add_space(8.0);
    let groups = groups(
        state.rows.as_deref().unwrap_or_default(),
        &state.query,
        state.order,
        catalog,
    );
    zones.scroll_area(ui, "historique-achats", |ui, width| {
        if groups.is_empty() {
            ui.add(design::label("Aucun achat enregistré.").color(tokens::TEXT_DISABLED));
            return;
        }
        for group in &groups {
            let collapsed = state.collapsed.contains(&group.day);
            if day_header(
                ui,
                width,
                &format::relative_day(group.day, today),
                &format::thousands(group.total_cost),
                !collapsed,
            ) {
                if collapsed {
                    state.collapsed.remove(&group.day);
                } else {
                    state.collapsed.insert(group.day);
                }
            }
            if collapsed {
                continue;
            }
            for (index, row) in group.rows.iter().enumerate() {
                purchase_row(ui, width, row, index);
            }
            ui.add_space(6.0);
        }
    });
}

/// En-tête de jour cliquable : date à gauche, total et chevron à droite. Rend `true` au clic.
fn day_header(ui: &mut Ui, width: f32, day: &str, total: &str, open: bool) -> bool {
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
    super::paint_amount(
        ui,
        &painter,
        egui::pos2(rect.right() - 34.0, rect.center().y),
        total,
        font,
        tokens::TEXT_GOLD,
    );
    super::paint_icon(
        ui,
        &painter,
        egui::pos2(rect.right() - 14.0, rect.center().y),
        if open {
            DsIcon::ChevronDown
        } else {
            DsIcon::TriangleRight
        },
        12.0,
        tokens::TEXT_GOLD,
    );
    response.clicked()
}

fn purchase_row(ui: &mut Ui, width: f32, row: &DayRow, index: usize) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, ROW_HEIGHT), Sense::hover());
    let painter = ui.painter_at(rect);
    if index % 2 == 1 {
        painter.rect_filled(rect, 0.0, tokens::TABLE_ROW_STRIPE);
    }
    let font = text::label_font(ui.ctx(), FONT_SIZE);
    let cost_right = rect.right() - 10.0;
    let qty_right = cost_right - 130.0;
    let name_rect = Rect::from_min_max(
        rect.min + egui::vec2(10.0, 0.0),
        egui::pos2(qty_right - 70.0, rect.max.y),
    );
    let (name_color, cost) = if row.is_hdv_sale {
        (HDV_GAIN, format!("+{}", format::thousands(row.total_cost)))
    } else {
        (Color32::WHITE, format::thousands(row.total_cost))
    };
    painter.with_clip_rect(name_rect).text(
        name_rect.left_center(),
        Align2::LEFT_CENTER,
        &row.name,
        font.clone(),
        name_color,
    );
    if !row.is_hdv_sale {
        painter.text(
            egui::pos2(qty_right, rect.center().y),
            Align2::RIGHT_CENTER,
            format!("×{}", format::thousands(row.quantity)),
            font.clone(),
            tokens::OVERLAY_TEXT_MUTED,
        );
    }
    super::paint_amount(
        ui,
        &painter,
        egui::pos2(cost_right, rect.center().y),
        &cost,
        font,
        if row.is_hdv_sale {
            HDV_GAIN
        } else {
            tokens::TEXT_GOLD
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: i64, name: Option<&str>, item_id: Option<i64>, cost: i64, at: i64) -> PurchaseRow {
        PurchaseRow {
            id,
            item_id,
            item_name: name.map(str::to_string),
            quantity: 1,
            total_cost: cost,
            occurred_at: at,
            game_server: None,
        }
    }

    #[test]
    fn un_jour_cumule_par_objet_et_epingle_la_vente_hdv() {
        let noon = 1_795_780_800_000; // 2026-11-27 12:00 UTC
        let rows = vec![
            row(1, Some("Laine"), None, 100, noon + 3_000),
            row(2, Some(HDV_KAMAS_SALE_ITEM), None, 5_000, noon + 2_000),
            row(3, Some("Laine"), None, 50, noon + 1_000),
            row(4, Some("Plume"), None, 10, noon),
        ];
        let groups = groups(&rows, "", SortOrder::Newest, &CatalogIndex::default());
        assert_eq!(groups.len(), 1);
        let day = &groups[0];
        assert_eq!(day.total_cost, 160, "la vente HDV n'est pas une dépense");
        let names: Vec<&str> = day.rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, vec![HDV_LABEL, "Laine", "Plume"]);
        assert_eq!((day.rows[1].quantity, day.rows[1].total_cost), (2, 150));
        // Recherche sans accents ni casse.
        let found = super::groups(&rows, "HOTEL", SortOrder::Newest, &CatalogIndex::default());
        assert_eq!(found[0].rows.len(), 1);
    }
}
