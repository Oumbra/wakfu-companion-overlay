//! Vue **Échanges** — port de `features/trades/` (site) : recherche (personnage, objet ou date),
//! tri récent / ancien, groupes par jour avec le nombre d'échanges ; chaque échange garde sa
//! propre carte — personnages, kamas, objets acquis et cédés côte à côte.
//!
//! Le filtre ([`matches`]) est séparé du rendu : c'est lui que les tests comparent au site.

use egui::{Align2, Color32, Rect, Sense, Ui};
use overlay_engine::roster::normalize_wakfu_name;
use overlay_engine::CatalogIndex;
use overlay_history::read::{ItemLine, TradeRow};
use overlay_history::HistoryStore;

use super::items::{self, CompactRow, DayTail, ListState};
use super::{format, HistoryContext};
use crate::design::{self, text, tokens};
use crate::remote_icons::RemoteIconTextures;

pub type EchangesState = ListState<TradeRow>;

/// `.trade-entry` : `padding: 10px 15px`.
const PAD_Y: f32 = 10.0;
const PAD_X: f32 = 15.0;
/// `.trade-header` et son `margin-bottom: 8px`.
const HEADER_HEIGHT: f32 = 20.0;
const BLOCK_GAP: f32 = 8.0;
/// `.trade-kamas-row` : `padding: 5px 8px`.
const KAMAS_HEIGHT: f32 = 26.0;
/// `.trade-section` : `padding: 8px`, libellé et son `margin-bottom: 6px`, `gap: 10px` entre les
/// deux colonnes.
const SECTION_PAD: f32 = 8.0;
const SECTION_LABEL: f32 = 14.0;
const SECTION_LABEL_GAP: f32 = 6.0;
const COLUMN_GAP: f32 = 10.0;
/// `.trade-item-row` : `padding: 3px 6px`, `gap: 6px`, `margin-bottom: 2px`.
const ITEM_ROW: CompactRow = CompactRow {
    height: 26.0,
    pad_x: 6.0,
    gap: 6.0,
    font_size: 13.0,
};
const ITEM_GAP: f32 = 2.0;
/// En dessous de cette largeur, acquis et cédés s'empilent (`@media (max-width: 480px)`).
const STACK_BELOW: f32 = 480.0;
/// Fond en creux (`--surface-well`) des blocs kamas et objets.
const WELL: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 90);
const ENTRY_BORDER: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 70);

/// Un échange retenu par la recherche — même règle que `TradesComponent.groups` : le personnage
/// en face, la date, ou un objet acquis ou cédé.
pub fn matches(trade: &TradeRow, query: &str, catalog: &CatalogIndex) -> bool {
    if query.is_empty() {
        return true;
    }
    normalize_wakfu_name(&trade.peer_name).contains(query)
        || normalize_wakfu_name(&format::medium_date(format::local_day(trade.occurred_at)))
            .contains(query)
        || trade
            .acquired
            .iter()
            .chain(&trade.given)
            .any(|line| normalize_wakfu_name(&items::item_name(line, catalog)).contains(query))
}

/// Hauteur d'une colonne d'objets (libellé compris).
fn section_height(lines: &[ItemLine]) -> f32 {
    let rows = lines.len() as f32;
    SECTION_PAD * 2.0
        + SECTION_LABEL
        + SECTION_LABEL_GAP
        + rows * ITEM_ROW.height
        + (rows - 1.0).max(0.0) * ITEM_GAP
}

/// Les colonnes d'objets non vides : (libellé, couleur, lignes).
fn sections(trade: &TradeRow) -> Vec<(&'static str, Color32, &[ItemLine])> {
    [
        ("ACQUIS", items::GAIN, trade.acquired.as_slice()),
        ("CÉDÉS", items::LOSS, trade.given.as_slice()),
    ]
    .into_iter()
    .filter(|(_, _, lines)| !lines.is_empty())
    .collect()
}

/// Hauteur d'une carte d'échange à cette largeur.
fn entry_height(trade: &TradeRow, width: f32) -> f32 {
    let mut height = PAD_Y * 2.0 + HEADER_HEIGHT + BLOCK_GAP;
    if trade.kamas_acquired > 0 || trade.kamas_given > 0 {
        height += KAMAS_HEIGHT + BLOCK_GAP;
    }
    let sections = sections(trade);
    if !sections.is_empty() {
        let heights = sections.iter().map(|(_, _, lines)| section_height(lines));
        if width < STACK_BELOW {
            height += heights.clone().sum::<f32>() + COLUMN_GAP * (sections.len() - 1) as f32;
        } else {
            height += heights.fold(0.0, f32::max);
        }
    }
    height
}

pub fn show(
    ui: &mut Ui,
    zones: &design::PanelZones,
    state: &mut EchangesState,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    state.load(ctx.store, HistoryStore::trades, "échanges");
    items::list_header(
        ui,
        zones,
        &mut state.query,
        &mut state.order,
        "Rechercher un objet, une date ou un personnage...",
        "historique.echanges",
    );
    let rows = state.rows.as_deref().unwrap_or_default();
    let times: Vec<i64> = rows.iter().map(|row| row.occurred_at).collect();
    let shown = state.pager.visible(&times, false);
    let query = normalize_wakfu_name(state.query.trim());
    let groups = items::group_by_day(
        rows[..shown]
            .iter()
            .filter(|trade| matches(trade, &query, ctx.catalog)),
        |trade| trade.occurred_at,
        state.order,
    );
    let pager = &mut state.pager;
    let collapsed_days = &mut state.collapsed;
    zones.scroll_area(ui, "historique-echanges", |ui, width| {
        if groups.is_empty() {
            ui.add(design::label("Aucun échange enregistré.").color(tokens::TEXT_DISABLED));
        }
        for (day, trades) in &groups {
            let collapsed = collapsed_days.contains(day);
            if items::day_header(
                ui,
                width,
                &format::relative_day(*day, ctx.today),
                DayTail::Count(trades.len()),
                !collapsed,
            ) {
                EchangesState::toggle(collapsed_days, *day);
            }
            if collapsed {
                continue;
            }
            for (index, trade) in trades.iter().enumerate() {
                trade_entry(ui, width, trade, index, ctx, textures);
            }
            ui.add_space(6.0);
        }
        pager.footer(ui, width, &times, shown, "historique.echanges");
    });
}

/// `⇄` : une flèche vers la droite au-dessus d'une flèche vers la gauche, centrées sur `center`.
fn paint_swap_arrows(painter: &egui::Painter, center: egui::Pos2, color: Color32) {
    let stroke = egui::Stroke::new(1.4, color);
    let (half, gap, head) = (6.0, 3.0, 3.0);
    for (y, dir) in [(center.y - gap, 1.0), (center.y + gap, -1.0)] {
        let tip = egui::pos2(center.x + half * dir, y);
        painter.line_segment([egui::pos2(center.x - half * dir, y), tip], stroke);
        painter.line_segment(
            [tip, egui::pos2(tip.x - head * dir, y - head * dir)],
            stroke,
        );
    }
}

fn trade_entry(
    ui: &mut Ui,
    width: f32,
    trade: &TradeRow,
    index: usize,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(width, entry_height(trade, width)),
        Sense::hover(),
    );
    if !ui.is_rect_visible(rect) {
        return;
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
    let mut y = inner.top();

    // En-tête : horloge (date et heure au survol), « moi ⇄ lui ».
    let header_center = y + HEADER_HEIGHT / 2.0;
    items::entry_clock(
        ui,
        egui::pos2(inner.left() + 8.0, header_center),
        trade.occurred_at,
        ui.id().with(("echange-horloge", trade.id)),
    );
    let names_rect = Rect::from_min_max(
        egui::pos2(inner.left() + 26.0, y),
        egui::pos2(inner.right(), y + HEADER_HEIGHT),
    );
    // « moi ⇄ lui » : la police du design system n'a pas la flèche, elle est dessinée.
    let names = painter.with_clip_rect(names_rect);
    let name_font = text::label_strong_font(ui.ctx(), items::FONT_SIZE);
    let own = names.text(
        names_rect.left_center(),
        Align2::LEFT_CENTER,
        &trade.self_name,
        name_font.clone(),
        Color32::WHITE,
    );
    let arrows = egui::pos2(own.right() + 12.0, header_center);
    paint_swap_arrows(&names, arrows, Color32::WHITE);
    names.text(
        egui::pos2(arrows.x + 12.0, header_center),
        Align2::LEFT_CENTER,
        &trade.peer_name,
        name_font,
        Color32::WHITE,
    );
    y += HEADER_HEIGHT + BLOCK_GAP;

    // Kamas : « KAMAS  +12 345 ₭  -678 ₭ ».
    if trade.kamas_acquired > 0 || trade.kamas_given > 0 {
        let row = Rect::from_min_size(
            egui::pos2(inner.left(), y),
            egui::vec2(inner.width(), KAMAS_HEIGHT),
        );
        painter.rect_filled(row, 6.0, WELL);
        let label = painter.text(
            row.left_center() + egui::vec2(8.0, 0.0),
            Align2::LEFT_CENTER,
            "KAMAS",
            text::label_strong_font(ui.ctx(), 11.0),
            tokens::OVERLAY_TEXT_MUTED,
        );
        let font = text::label_strong_font(ui.ctx(), 13.0);
        let mut x = label.right() + 8.0;
        for (amount, sign, color) in [
            (trade.kamas_acquired, '+', items::GAIN),
            (trade.kamas_given, '-', items::LOSS),
        ] {
            if amount <= 0 {
                continue;
            }
            let value = painter.text(
                egui::pos2(x, row.center().y),
                Align2::LEFT_CENTER,
                format!("{sign}{}", format::kamas(amount)),
                font.clone(),
                color,
            );
            x = value.right() + 8.0;
        }
        y += KAMAS_HEIGHT + BLOCK_GAP;
    }

    // Objets acquis et cédés, côte à côte (empilés si la fenêtre est étroite).
    let sections = sections(trade);
    if sections.is_empty() {
        return;
    }
    let stacked = width < STACK_BELOW;
    let columns = if stacked { 1 } else { sections.len() };
    let column_width = (inner.width() - COLUMN_GAP * (columns - 1) as f32) / columns as f32;
    let mut x = inner.left();
    for (label, color, lines) in sections {
        let height = section_height(lines);
        let section = Rect::from_min_size(egui::pos2(x, y), egui::vec2(column_width, height));
        painter.rect_filled(section, 6.0, WELL);
        let content = section.shrink(SECTION_PAD);
        painter.text(
            egui::pos2(content.left(), content.top() + SECTION_LABEL / 2.0),
            Align2::LEFT_CENTER,
            label,
            text::label_strong_font(ui.ctx(), 11.0),
            color,
        );
        let mut row_top = content.top() + SECTION_LABEL + SECTION_LABEL_GAP;
        for line in lines {
            let row = Rect::from_min_size(
                egui::pos2(content.left(), row_top),
                egui::vec2(content.width(), ITEM_ROW.height),
            );
            items::paint_compact_item(ui, &painter, row, line, ITEM_ROW, ctx, textures);
            row_top += ITEM_ROW.height + ITEM_GAP;
        }
        if stacked {
            y += height + COLUMN_GAP;
        } else {
            x += column_width + COLUMN_GAP;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(name: &str, quantity: i64) -> ItemLine {
        ItemLine {
            item_id: None,
            item_name: Some(name.to_string()),
            quantity,
        }
    }

    fn trade(acquired: Vec<ItemLine>, given: Vec<ItemLine>, kamas: i64) -> TradeRow {
        TradeRow {
            id: 1,
            peer_name: "Élodïe".to_string(),
            self_name: "Moi".to_string(),
            occurred_at: 1_795_780_800_000,
            kamas_acquired: kamas,
            kamas_given: 0,
            game_server: None,
            acquired,
            given,
        }
    }

    #[test]
    fn la_recherche_porte_sur_le_personnage_et_les_objets() {
        let catalog = CatalogIndex::default();
        let row = trade(vec![line("Plume de Tofu", 3)], vec![line("Laine", 1)], 0);
        assert!(matches(&row, "", &catalog));
        assert!(matches(&row, &normalize_wakfu_name("elodie"), &catalog));
        assert!(matches(&row, &normalize_wakfu_name("LAINE"), &catalog));
        assert!(matches(&row, &normalize_wakfu_name("27 nov"), &catalog));
        assert!(!matches(&row, &normalize_wakfu_name("Moi"), &catalog));
    }

    #[test]
    fn une_carte_grandit_avec_sa_plus_longue_colonne() {
        let empty = trade(Vec::new(), Vec::new(), 0);
        let bare = PAD_Y * 2.0 + HEADER_HEIGHT + BLOCK_GAP;
        assert_eq!(entry_height(&empty, 800.0), bare);
        let with_kamas = trade(Vec::new(), Vec::new(), 500);
        assert_eq!(
            entry_height(&with_kamas, 800.0),
            bare + KAMAS_HEIGHT + BLOCK_GAP
        );
        let full = trade(
            vec![line("A", 1), line("B", 2), line("C", 3)],
            vec![line("D", 1)],
            0,
        );
        let three = section_height(&full.acquired);
        assert_eq!(entry_height(&full, 800.0), bare + three);
        // Étroite : les colonnes s'empilent.
        let one = section_height(&full.given);
        assert_eq!(entry_height(&full, 400.0), bare + three + COLUMN_GAP + one);
    }
}
