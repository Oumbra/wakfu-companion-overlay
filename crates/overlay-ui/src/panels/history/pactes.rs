//! Vue **Pactes** — port de `features/pact/` (site) : recherche (objet ou date), tri récent /
//! ancien, groupes par jour avec le nombre d'extractions ; chaque extraction garde sa propre
//! carte — horloge, nombre d'objets, puis les objets obtenus.

use egui::{Align2, Color32, Rect, Sense, Ui};
use overlay_engine::roster::normalize_wakfu_name;
use overlay_engine::CatalogIndex;
use overlay_history::read::PactRow;
use overlay_history::HistoryStore;

use super::items::{self, CompactRow, DayTail, ListState};
use super::{format, HistoryContext};
use crate::design::{self, text, tokens};
use crate::remote_icons::RemoteIconTextures;

pub type PactesState = ListState<PactRow>;

/// `.pact-entry` : `padding: 10px 15px`.
const PAD_Y: f32 = 10.0;
const PAD_X: f32 = 15.0;
/// `.pact-header` et son `margin-bottom: 8px`.
const HEADER_HEIGHT: f32 = 20.0;
const HEADER_GAP: f32 = 8.0;
/// `.pact-item-row` : `padding: 4px 8px`, `gap: 8px`, `0.82rem` ; `gap: 2px` entre les lignes.
const ITEM_ROW: CompactRow = CompactRow {
    height: 28.0,
    pad_x: 8.0,
    gap: 8.0,
    font_size: 13.5,
};
const ITEM_GAP: f32 = 2.0;
const ENTRY_BORDER: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 70);

/// Une extraction retenue par la recherche — `PactComponent.groups` : la date ou un objet.
pub fn matches(pact: &PactRow, query: &str, catalog: &CatalogIndex) -> bool {
    query.is_empty()
        || normalize_wakfu_name(&format::medium_date(format::local_day(pact.occurred_at)))
            .contains(query)
        || pact
            .items
            .iter()
            .any(|line| normalize_wakfu_name(&items::item_name(line, catalog)).contains(query))
}

/// Hauteur d'une extraction ; repliée, sa seule ligne d'en-tête.
fn entry_height(pact: &PactRow, open: bool) -> f32 {
    if !open {
        return PAD_Y * 2.0 + HEADER_HEIGHT;
    }
    let rows = pact.items.len() as f32;
    PAD_Y * 2.0
        + HEADER_HEIGHT
        + HEADER_GAP
        + rows * ITEM_ROW.height
        + (rows - 1.0).max(0.0) * ITEM_GAP
}

pub fn show(
    ui: &mut Ui,
    zones: &design::PanelZones,
    state: &mut PactesState,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    state.load(ctx.store, HistoryStore::pacts, "pactes");
    items::list_header(
        ui,
        zones,
        &mut state.query,
        &mut state.order,
        "Rechercher un objet ou une date...",
        "historique.pactes",
    );
    let rows = state.rows.as_deref().unwrap_or_default();
    let times: Vec<i64> = rows.iter().map(|row| row.occurred_at).collect();
    let shown = state.pager.visible(&times, false);
    let query = normalize_wakfu_name(state.query.trim());
    let groups = items::group_by_day(
        rows[..shown]
            .iter()
            .filter(|pact| matches(pact, &query, ctx.catalog)),
        |pact| pact.occurred_at,
        state.order,
    );
    let pager = &mut state.pager;
    let collapsed_days = &mut state.collapsed;
    let collapsed_entries = &mut state.collapsed_entries;
    let footer = pager.footer(ui, zones, &times, shown, "historique.pactes");
    let zones = footer.as_ref().unwrap_or(zones);
    zones.scroll_area(ui, "historique-pactes", |ui, width| {
        if groups.is_empty() {
            ui.add(
                design::label("Aucune extraction de pacte enregistrée.")
                    .color(tokens::TEXT_DISABLED),
            );
        }
        for (day, pacts) in &groups {
            let collapsed = collapsed_days.contains(day);
            if items::day_header(
                ui,
                width,
                &format::relative_day(*day, ctx.today),
                DayTail::Count(pacts.len()),
                !collapsed,
            ) {
                PactesState::toggle(collapsed_days, *day);
            }
            if collapsed {
                continue;
            }
            for (index, pact) in pacts.iter().enumerate() {
                let open = !collapsed_entries.contains(&pact.id);
                if pact_entry(ui, width, pact, index, open, ctx, textures)
                    && !collapsed_entries.remove(&pact.id)
                {
                    collapsed_entries.insert(pact.id);
                }
            }
            ui.add_space(6.0);
        }
    });
}

/// Une extraction ; rend `true` quand sa ligne d'en-tête est cliquée (replier / déplier).
#[allow(clippy::too_many_arguments)]
fn pact_entry(
    ui: &mut Ui,
    width: f32,
    pact: &PactRow,
    index: usize,
    open: bool,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) -> bool {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, entry_height(pact, open)), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return false;
    }
    let painter = ui.painter_at(rect);
    if index % 2 == 1 {
        painter.rect_filled(rect, 0.0, tokens::TABLE_ROW_STRIPE);
    }
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        egui::Stroke::new(1.0, ENTRY_BORDER),
    );
    let inner = rect.shrink2(egui::vec2(PAD_X, PAD_Y));
    let header_center = inner.top() + HEADER_HEIGHT / 2.0;
    let clicked = items::entry_toggle(
        ui,
        Rect::from_min_size(inner.min, egui::vec2(inner.width(), HEADER_HEIGHT)),
        open,
        ui.id().with(("pacte-repli", pact.id)),
    );
    items::entry_clock(
        ui,
        egui::pos2(inner.left() + 8.0, header_center),
        pact.occurred_at,
        ui.id().with(("pacte-horloge", pact.id)),
    );
    painter.text(
        egui::pos2(inner.left() + 26.0, header_center),
        Align2::LEFT_CENTER,
        format!("{} OBJET(S)", format::thousands(pact.items.len() as i64)),
        text::label_strong_font(ui.ctx(), 12.0),
        tokens::OVERLAY_TEXT_MUTED,
    );
    if !open {
        return clicked;
    }
    let mut row_top = inner.top() + HEADER_HEIGHT + HEADER_GAP;
    for line in &pact.items {
        let row = Rect::from_min_size(
            egui::pos2(inner.left(), row_top),
            egui::vec2(inner.width(), ITEM_ROW.height),
        );
        items::paint_compact_item(ui, &painter, row, line, ITEM_ROW, ctx, textures);
        row_top += ITEM_ROW.height + ITEM_GAP;
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;
    use overlay_history::read::ItemLine;

    #[test]
    fn recherche_par_objet_ou_date() {
        let catalog = CatalogIndex::default();
        let pact = PactRow {
            id: 1,
            occurred_at: 1_795_780_800_000,
            game_server: None,
            items: vec![ItemLine {
                item_id: None,
                item_name: Some("Éclat de Pacte".to_string()),
                quantity: 2,
            }],
        };
        assert!(matches(&pact, &normalize_wakfu_name("eclat"), &catalog));
        assert!(matches(&pact, &normalize_wakfu_name("27 nov."), &catalog));
        assert!(!matches(&pact, &normalize_wakfu_name("laine"), &catalog));
        assert_eq!(
            entry_height(&pact, true),
            PAD_Y * 2.0 + HEADER_HEIGHT + HEADER_GAP + ITEM_ROW.height
        );
        assert_eq!(entry_height(&pact, false), PAD_Y * 2.0 + HEADER_HEIGHT);
    }
}
