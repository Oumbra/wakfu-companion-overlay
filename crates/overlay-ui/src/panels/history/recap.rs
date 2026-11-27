//! Vue **Récap** — port de `features/session-recap/` (site), mode « cumulé » : une période (jour,
//! mois ou année, bornée dans le calendrier local), parcourue par précédent / suivant, et ses
//! agrégats (`HistoryStore::period_stats`) — bande d'indicateurs (kamas nets, XP, combats gagnés –
//! perdus, challenges réussis – échoués), détail des kamas, XP par personnage, butin et pactes.
//!
//! **Pas encore porté** : la granularité « session », le sélecteur calendrier, et les modes « par
//! donjon et famille » et « par type de donjon ».

use chrono::{Datelike, Local, NaiveDate, TimeZone};
use egui::{Align2, Color32, Rect, Sense, Ui};
use overlay_history::read::ItemLine;
use overlay_history::stats::{ItemTotal, PeriodStats};

use super::items::{self, CompactRow};
use super::{format, HistoryContext};
use crate::design::{self, text, tokens, DsIcon};
use crate::remote_icons::RemoteIconTextures;

/// La granularité de la période.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Granularity {
    #[default]
    Day,
    Month,
    Year,
}

const MONTHS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

/// Le premier jour de la période qui contient `day`.
pub fn period_start(granularity: Granularity, day: NaiveDate) -> NaiveDate {
    match granularity {
        Granularity::Day => day,
        Granularity::Month => day.with_day(1).unwrap_or(day),
        Granularity::Year => NaiveDate::from_ymd_opt(day.year(), 1, 1).unwrap_or(day),
    }
}

/// Le premier jour de la période suivante (`step` = 1) ou précédente (`step` = -1).
pub fn shift(granularity: Granularity, start: NaiveDate, step: i32) -> NaiveDate {
    match granularity {
        Granularity::Day => start + chrono::Duration::days(step as i64),
        Granularity::Month => {
            let months = start.year() * 12 + start.month0() as i32 + step;
            NaiveDate::from_ymd_opt(months.div_euclid(12), months.rem_euclid(12) as u32 + 1, 1)
                .unwrap_or(start)
        }
        Granularity::Year => NaiveDate::from_ymd_opt(start.year() + step, 1, 1).unwrap_or(start),
    }
}

/// Minuit local d'un jour, en ms UTC — sans piège d'heure d'été (le premier instant valide).
fn local_midnight_ms(day: NaiveDate) -> i64 {
    let midnight = day.and_hms_opt(0, 0, 0).unwrap_or_default();
    Local
        .from_local_datetime(&midnight)
        .earliest()
        .map_or(0, |moment| moment.timestamp_millis())
}

/// `[début, fin[` de la période, en ms UTC.
pub fn bounds(granularity: Granularity, start: NaiveDate) -> (i64, i64) {
    (
        local_midnight_ms(start),
        local_midnight_ms(shift(granularity, start, 1)),
    )
}

/// Le libellé de la période : « Aujourd'hui », « novembre 2026 », « 2026 ».
pub fn label(granularity: Granularity, start: NaiveDate, today: NaiveDate) -> String {
    match granularity {
        Granularity::Day => format::relative_day(start, today),
        Granularity::Month => format!("{} {}", MONTHS[start.month0() as usize], start.year()),
        Granularity::Year => start.year().to_string(),
    }
}

/// Kamas nets de la période : gains (combats, ventes, échanges reçus) moins dépenses (achats,
/// échanges donnés).
pub fn net_kamas(stats: &PeriodStats) -> i64 {
    let k = &stats.kamas;
    k.from_combat + k.from_hdv_sales + k.trades_acquired - k.spent_on_purchases - k.trades_given
}

/// État de la vue.
#[derive(Default)]
pub struct RecapState {
    pub granularity: Granularity,
    /// Premier jour de la période affichée ; `None` : celle d'aujourd'hui.
    pub start: Option<NaiveDate>,
    /// Les agrégats de la période affichée, et ce qui les a produits.
    cache: Option<(Granularity, NaiveDate, bool, PeriodStats)>,
}

impl RecapState {
    /// La base a changé : les agrégats seront recalculés.
    pub fn invalidate(&mut self) {
        self.cache = None;
    }
}

const NAV_WIDTH: f32 = 36.0;
const GRANULARITY_WIDTH: f32 = 270.0;
const BAND_HEIGHT: f32 = 64.0;
const ITEM_ROW: CompactRow = CompactRow {
    height: 26.0,
    pad_x: 6.0,
    gap: 6.0,
    font_size: 13.0,
};

pub fn show(
    ui: &mut Ui,
    zones: &design::PanelZones,
    state: &mut RecapState,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let current = period_start(state.granularity, ctx.today);
    let mut start = state
        .start
        .map_or(current, |start| period_start(state.granularity, start));

    // Granularité, puis précédent / libellé / suivant.
    ui.horizontal(|ui| {
        let before = state.granularity;
        ui.add(
            design::switch(&mut state.granularity)
                .slot(Granularity::Day, "Jour")
                .slot(Granularity::Month, "Mois")
                .slot(Granularity::Year, "Année")
                .width(GRANULARITY_WIDTH)
                .log_name("historique.recap.granularite"),
        );
        if state.granularity != before {
            start = period_start(state.granularity, start);
        }
        ui.add_space(16.0);
        if ui
            .add(
                design::icon_button(DsIcon::TriangleRight)
                    .size(NAV_WIDTH)
                    .tooltip("Période précédente")
                    .log_name("historique.recap.precedente"),
            )
            .clicked()
        {
            start = shift(state.granularity, start, -1);
        }
        let label_width = zones.inner.width() - GRANULARITY_WIDTH - 2.0 * NAV_WIDTH - 60.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(label_width, NAV_WIDTH), Sense::hover());
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            label(state.granularity, start, ctx.today),
            text::label_strong_font(ui.ctx(), items::FONT_SIZE + 2.0),
            tokens::TEXT_GOLD,
        );
        // Pas de période future.
        let next_enabled = start < current;
        if ui
            .add(
                design::icon_button(DsIcon::TriangleRight)
                    .mirrored(true)
                    .size(NAV_WIDTH)
                    .enabled(next_enabled)
                    .tooltip("Période suivante")
                    .log_name("historique.recap.suivante"),
            )
            .clicked()
            && next_enabled
        {
            start = shift(state.granularity, start, 1);
        }
    });
    ui.add_space(8.0);
    state.start = (start != current).then_some(start);

    // Agrégats : recalculés quand la période change, ou quand le référentiel arrive (il range les
    // combats par famille et écarte ceux d'Incarnam).
    let catalog_ready = !ctx.catalog.is_empty();
    let stale = state.cache.as_ref().is_none_or(|(g, s, ready, _)| {
        (*g, *s, *ready) != (state.granularity, start, catalog_ready)
    });
    if stale {
        let (since, until) = bounds(state.granularity, start);
        let stats = match ctx
            .store
            .map(|store| store.period_stats(since, until, ctx.catalog))
        {
            Some(Ok(stats)) => stats,
            Some(Err(err)) => {
                tracing::warn!(%err, "historique : récap illisible");
                PeriodStats::default()
            }
            None => PeriodStats::default(),
        };
        state.cache = Some((state.granularity, start, catalog_ready, stats));
    }
    let Some((_, _, _, stats)) = &state.cache else {
        return;
    };

    zones.scroll_area(ui, "historique-recap", |ui, width| {
        band(ui, width, stats);
        ui.add_space(12.0);
        kamas_detail(ui, width, stats);
        ui.add_space(12.0);
        section_title(ui, width, "Expérience", "");
        if stats.xp_by_character.iter().all(|xp| xp.amount <= 0) {
            ui.add(
                design::label("Aucune XP enregistrée pour le moment.").color(tokens::TEXT_DISABLED),
            );
        }
        // Un personnage présent sans XP gagnée n'apprend rien.
        for xp in stats.xp_by_character.iter().filter(|xp| xp.amount > 0) {
            let (row, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), Sense::hover());
            let painter = ui.painter_at(row);
            painter.text(
                egui::pos2(row.left() + 12.0, row.center().y),
                Align2::LEFT_CENTER,
                &xp.name,
                text::label_font(ui.ctx(), 13.0),
                Color32::WHITE,
            );
            painter.text(
                egui::pos2(row.right() - 12.0, row.center().y),
                Align2::RIGHT_CENTER,
                format!("+{}", format::thousands(xp.amount)),
                text::label_strong_font(ui.ctx(), 13.0),
                tokens::OVERLAY_ACCENT,
            );
        }
        for (title, list) in [("Butin", &stats.loot), ("Pacte", &stats.pacts)] {
            if list.is_empty() {
                continue;
            }
            ui.add_space(12.0);
            section_title(ui, width, title, &format!("{} objet(s)", list.len()));
            item_list(ui, width, list, ctx, textures);
        }
    });
}

/// Une case de la bande : glyphe blanc, libellé, puis la valeur en couleur.
struct Cell<'a> {
    icon: DsIcon,
    label: &'a str,
    parts: Vec<(String, Color32)>,
}

fn band(ui: &mut Ui, width: f32, stats: &PeriodStats) {
    let c = &stats.combats;
    let net = net_kamas(stats);
    let xp: i64 = stats.xp_by_character.iter().map(|xp| xp.amount).sum();
    let cells = [
        Cell {
            icon: DsIcon::Kamas,
            label: "Kamas",
            parts: vec![(
                format::thousands(net),
                if net < 0 {
                    items::LOSS
                } else {
                    tokens::TEXT_GOLD
                },
            )],
        },
        Cell {
            icon: DsIcon::Xp,
            label: "Expérience",
            parts: vec![(format::thousands(xp), tokens::OVERLAY_ACCENT)],
        },
        Cell {
            icon: DsIcon::Sword,
            label: "Combats",
            parts: vec![
                (format::thousands(c.won), items::GAIN),
                (" – ".to_string(), tokens::OVERLAY_TEXT_MUTED),
                (format::thousands(c.lost), items::LOSS),
            ],
        },
        Cell {
            icon: DsIcon::Trophy,
            label: "Challenges",
            parts: vec![
                (format::thousands(c.challenges_passed), items::GAIN),
                (" – ".to_string(), tokens::OVERLAY_TEXT_MUTED),
                (format::thousands(c.challenges_failed), items::LOSS),
            ],
        },
    ];
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, BAND_HEIGHT), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, Color32::from_rgba_premultiplied(0, 0, 0, 80));
    let cell_width = width / cells.len() as f32;
    for (index, cell) in cells.iter().enumerate() {
        let area = Rect::from_min_size(
            egui::pos2(rect.left() + index as f32 * cell_width, rect.top()),
            egui::vec2(cell_width, rect.height()),
        );
        if index > 0 {
            painter.vline(
                area.left(),
                area.y_range().shrink(12.0),
                egui::Stroke::new(1.0, Color32::from_white_alpha(20)),
            );
        }
        painter.text(
            egui::pos2(area.center().x, area.top() + 18.0),
            Align2::CENTER_CENTER,
            cell.label.to_uppercase(),
            text::label_strong_font(ui.ctx(), 11.0),
            tokens::OVERLAY_TEXT_MUTED,
        );
        let font = text::label_strong_font(ui.ctx(), 17.0);
        let galleys: Vec<_> = cell
            .parts
            .iter()
            .map(|(value, color)| painter.layout_no_wrap(value.clone(), font.clone(), *color))
            .collect();
        let value_width: f32 = galleys.iter().map(|g| g.size().x).sum();
        let icon_side = 16.0;
        let total = icon_side + 8.0 + value_width;
        let y = area.top() + 42.0;
        let mut x = area.center().x - total / 2.0;
        super::paint_icon(
            ui,
            &painter,
            egui::pos2(x + icon_side / 2.0, y),
            cell.icon,
            icon_side,
            Color32::WHITE,
        );
        x += icon_side + 8.0;
        for galley in galleys {
            let size = galley.size();
            painter.galley(egui::pos2(x, y - size.y / 2.0), galley, Color32::WHITE);
            x += size.x;
        }
    }
}

/// Le détail des kamas : une ligne par source, signée.
fn kamas_detail(ui: &mut Ui, width: f32, stats: &PeriodStats) {
    let k = &stats.kamas;
    section_title(ui, width, "Kamas", "");
    for (label, amount, sign) in [
        ("Combats", k.from_combat, 1),
        ("Hôtel de vente", k.from_hdv_sales, 1),
        ("Achats", k.spent_on_purchases, -1),
        ("Échanges reçus", k.trades_acquired, 1),
        ("Échanges donnés", k.trades_given, -1),
    ] {
        let (row, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), Sense::hover());
        let painter = ui.painter_at(row);
        painter.text(
            egui::pos2(row.left() + 12.0, row.center().y),
            Align2::LEFT_CENTER,
            label,
            text::label_font(ui.ctx(), 13.0),
            Color32::WHITE,
        );
        let (text, color) = match (amount, sign) {
            (0, _) => ("0".to_string(), tokens::OVERLAY_TEXT_MUTED),
            (amount, 1) => (format!("+{}", format::thousands(amount)), items::GAIN),
            (amount, _) => (format!("-{}", format::thousands(amount)), items::LOSS),
        };
        super::paint_amount(
            ui,
            &painter,
            egui::pos2(row.right() - 12.0, row.center().y),
            &text,
            text::label_strong_font(ui.ctx(), 13.0),
            color,
        );
    }
}

fn section_title(ui: &mut Ui, width: f32, title: &str, tail: &str) {
    let (row, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), Sense::hover());
    let painter = ui.painter_at(row);
    let title = painter.text(
        egui::pos2(row.left() + 4.0, row.center().y),
        Align2::LEFT_CENTER,
        title,
        text::label_strong_font(ui.ctx(), 14.0),
        tokens::TEXT_GOLD,
    );
    painter.text(
        egui::pos2(title.right() + 10.0, row.center().y),
        Align2::LEFT_CENTER,
        tail,
        text::label_font(ui.ctx(), 13.0),
        tokens::OVERLAY_TEXT_MUTED,
    );
}

/// Une liste d'objets cumulés, par quantité décroissante.
fn item_list(
    ui: &mut Ui,
    width: f32,
    list: &[ItemTotal],
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let mut lines: Vec<ItemLine> = list
        .iter()
        .map(|item| ItemLine {
            item_id: item.item_id,
            item_name: item.item_name.clone(),
            quantity: item.quantity,
        })
        .collect();
    lines.sort_by_key(|line| std::cmp::Reverse(line.quantity));
    for line in &lines {
        let (row, _) =
            ui.allocate_exact_size(egui::vec2(width, ITEM_ROW.height + 2.0), Sense::hover());
        if !ui.is_rect_visible(row) {
            continue;
        }
        let row = Rect::from_min_size(
            row.min + egui::vec2(0.0, 1.0),
            egui::vec2(width, ITEM_ROW.height),
        );
        items::paint_compact_item(ui, ui.painter(), row, line, ITEM_ROW, ctx, textures);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn periodes_du_calendrier_local() {
        let today = day(2026, 11, 27);
        assert_eq!(period_start(Granularity::Month, today), day(2026, 11, 1));
        assert_eq!(period_start(Granularity::Year, today), day(2026, 1, 1));
        assert_eq!(
            shift(Granularity::Month, day(2026, 1, 1), -1),
            day(2025, 12, 1)
        );
        assert_eq!(
            shift(Granularity::Month, day(2026, 12, 1), 1),
            day(2027, 1, 1)
        );
        assert_eq!(
            shift(Granularity::Day, day(2026, 3, 1), -1),
            day(2026, 2, 28)
        );
        assert_eq!(label(Granularity::Day, today, today), "Aujourd'hui");
        assert_eq!(
            label(Granularity::Month, day(2026, 8, 1), today),
            "août 2026"
        );
        assert_eq!(label(Granularity::Year, day(2025, 1, 1), today), "2025");
        let (since, until) = bounds(Granularity::Day, today);
        // 23, 24 ou 25 heures selon l'heure d'été.
        assert!((23 * 3_600_000..=25 * 3_600_000).contains(&(until - since)));
    }
}
