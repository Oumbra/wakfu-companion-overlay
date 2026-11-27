//! **Sélecteur de période** du Récap — port de `shared/period-picker/` (site) : le bouton calendrier
//! à côté de précédent / suivant ouvre une grille pour rejoindre d'un clic une période éloignée.
//!
//! Une page par granularité : un **mois** de jours (lundi en premier) pour le jour, une **année** de
//! douze mois pour le mois, une **décennie** pour l'année ; ‹ › tournent la page. Une case hors de
//! l'historique (avant le 1ᵉʳ août 2026) ou dans le futur est grisée, la période affichée est
//! surlignée. Le modèle ([`cells`], [`page_label`], [`turn_page`]) est séparé du rendu.

use chrono::{Datelike, NaiveDate};
use egui::{Align2, Color32, Rect, Sense, Ui};

use super::recap::{period_start, shift, Granularity};
use crate::design::{text, tokens};

/// Une case de la grille : la période qu'elle désigne (son premier jour) et son libellé ; `None`
/// pour une case de remplissage avant le premier jour ou après le dernier d'un mois.
pub type Cell = Option<(NaiveDate, String)>;

const MONTHS_SHORT: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];
const MONTHS: [&str; 12] = [
    "Janvier",
    "Février",
    "Mars",
    "Avril",
    "Mai",
    "Juin",
    "Juillet",
    "Août",
    "Septembre",
    "Octobre",
    "Novembre",
    "Décembre",
];
/// Lundi en premier, comme la grille du site.
const WEEKDAYS: [&str; 7] = ["lu", "ma", "me", "je", "ve", "sa", "di"];

/// Le premier jour de la page qui contient `day` : son mois (jour), son année (mois), sa décennie
/// (année).
pub fn page_start(granularity: Granularity, day: NaiveDate) -> NaiveDate {
    match granularity {
        Granularity::Day => day.with_day(1).unwrap_or(day),
        Granularity::Month => NaiveDate::from_ymd_opt(day.year(), 1, 1).unwrap_or(day),
        Granularity::Year => {
            NaiveDate::from_ymd_opt(day.year().div_euclid(10) * 10, 1, 1).unwrap_or(day)
        }
    }
}

/// La page `step` pages après celle de `page` (‹ : -1, › : +1).
pub fn turn_page(granularity: Granularity, page: NaiveDate, step: i32) -> NaiveDate {
    let page = page_start(granularity, page);
    match granularity {
        Granularity::Day => shift(Granularity::Month, page, step),
        Granularity::Month => shift(Granularity::Year, page, step),
        Granularity::Year => shift(Granularity::Year, page, step * 10),
    }
}

/// L'en-tête de la page : « Novembre 2026 », « 2026 », « 2020–2029 ».
pub fn page_label(granularity: Granularity, page: NaiveDate) -> String {
    let page = page_start(granularity, page);
    match granularity {
        Granularity::Day => format!("{} {}", MONTHS[page.month0() as usize], page.year()),
        Granularity::Month => page.year().to_string(),
        Granularity::Year => format!("{}–{}", page.year(), page.year() + 9),
    }
}

/// Les cases de la page, dans l'ordre de la grille.
pub fn cells(granularity: Granularity, page: NaiveDate) -> Vec<Cell> {
    let page = page_start(granularity, page);
    match granularity {
        Granularity::Day => {
            let mut cells: Vec<Cell> = Vec::new();
            for _ in 0..page.weekday().num_days_from_monday() {
                cells.push(None);
            }
            let mut day = page;
            while day.month() == page.month() {
                cells.push(Some((day, day.day().to_string())));
                day = day.succ_opt().unwrap_or(day);
                if day == page {
                    break;
                }
            }
            while !cells.len().is_multiple_of(7) {
                cells.push(None);
            }
            cells
        }
        Granularity::Month => (1..=12)
            .filter_map(|month| NaiveDate::from_ymd_opt(page.year(), month, 1))
            .map(|start| Some((start, MONTHS_SHORT[start.month0() as usize].to_string())))
            .collect(),
        Granularity::Year => (0..10)
            .filter_map(|offset| NaiveDate::from_ymd_opt(page.year() + offset, 1, 1))
            .map(|start| Some((start, start.year().to_string())))
            .collect(),
    }
}

/// Colonnes de la grille (`period-picker-grid-*`).
fn columns(granularity: Granularity) -> usize {
    match granularity {
        Granularity::Day => 7,
        Granularity::Month => 4,
        Granularity::Year => 5,
    }
}

/// Largeur du sélecteur (240 px sur le site, à l'échelle de la fenêtre).
pub const WIDTH: f32 = 266.0;
const PAD: f32 = 10.0;
const GAP: f32 = 3.0;
const HEADER: f32 = 28.0;
const WEEKDAY_ROW: f32 = 18.0;
const WIDE_CELL: f32 = 36.0;
const FILL: Color32 = Color32::from_rgb(0x20, 0x24, 0x2A);

/// Ce que l'appelant passe au sélecteur.
pub struct Bounds {
    /// La période affichée (surlignée).
    pub selected: NaiveDate,
    /// La première période permise (le début de l'historique).
    pub floor: NaiveDate,
    /// La dernière (la période en cours).
    pub current: NaiveDate,
}

/// Le sélecteur ouvert sous `anchor`, sur la page `page` (tournée sur place par ‹ ›). Rend la
/// période choisie à la frame du clic ; `close` passe à `true` sur un clic à côté ou Échap.
pub fn show(
    ui: &Ui,
    anchor: Rect,
    granularity: Granularity,
    page: &mut NaiveDate,
    bounds: &Bounds,
    close: &mut bool,
) -> Option<NaiveDate> {
    let cols = columns(granularity);
    let grid = cells(granularity, *page);
    let cell_width = (WIDTH - 2.0 * PAD - GAP * (cols - 1) as f32) / cols as f32;
    let cell_height = if granularity == Granularity::Day {
        cell_width
    } else {
        WIDE_CELL
    };
    let rows = grid.len().div_ceil(cols);
    let weekdays = if granularity == Granularity::Day {
        WEEKDAY_ROW
    } else {
        0.0
    };
    let height = 2.0 * PAD
        + HEADER
        + 8.0
        + weekdays
        + rows as f32 * cell_height
        + (rows.saturating_sub(1)) as f32 * GAP;
    let origin = egui::pos2(anchor.left(), anchor.bottom() + 6.0);
    let frame = Rect::from_min_size(origin, egui::vec2(WIDTH, height));
    let id = egui::Id::new("historique-recap-calendrier");
    let area = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(frame.min)
        .constrain(true)
        .show(ui.ctx(), |ui| {
            ui.set_min_size(frame.size());
            let frame = Rect::from_min_size(ui.min_rect().min, frame.size());
            let painter = ui.painter();
            painter.rect_filled(frame.expand(2.0), 10.0, Color32::from_black_alpha(90));
            painter.rect_filled(frame, 8.0, FILL);
            painter.rect_stroke(
                frame,
                8.0,
                egui::Stroke::new(1.0, tokens::OVERLAY_ACCENT),
                egui::StrokeKind::Inside,
            );
            let inner = frame.shrink(PAD);
            // En-tête : ‹ libellé ›, « › » grisé quand la page suivante est entièrement future.
            let header = Rect::from_min_size(inner.min, egui::vec2(inner.width(), HEADER));
            let next_page = turn_page(granularity, *page, 1);
            let next_enabled = period_start(granularity, next_page) <= bounds.current;
            let nav = |ui: &Ui, rect: Rect, glyph: &str, enabled: bool, salt: &str| -> bool {
                let response = ui.interact(rect, id.with(salt), Sense::click());
                let hovered = enabled && response.hovered();
                ui.painter().rect_filled(
                    rect,
                    4.0,
                    if hovered {
                        tokens::OVERLAY_ACCENT
                    } else {
                        Color32::from_white_alpha(14)
                    },
                );
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    glyph,
                    text::label_strong_font(ui.ctx(), 16.0),
                    if !enabled {
                        tokens::TEXT_DISABLED
                    } else if hovered {
                        Color32::BLACK
                    } else {
                        Color32::WHITE
                    },
                );
                enabled && response.clicked()
            };
            let side = egui::vec2(HEADER - 2.0, HEADER - 2.0);
            if nav(
                ui,
                Rect::from_min_size(header.min + egui::vec2(0.0, 1.0), side),
                "‹",
                true,
                "page-precedente",
            ) {
                *page = turn_page(granularity, *page, -1);
            }
            if nav(
                ui,
                Rect::from_min_size(
                    egui::pos2(header.right() - side.x, header.top() + 1.0),
                    side,
                ),
                "›",
                next_enabled,
                "page-suivante",
            ) {
                *page = next_page;
            }
            ui.painter().text(
                header.center(),
                Align2::CENTER_CENTER,
                page_label(granularity, *page),
                text::label_strong_font(ui.ctx(), 14.0),
                tokens::OVERLAY_ACCENT,
            );
            let mut top = header.bottom() + 8.0;
            if granularity == Granularity::Day {
                for (index, day) in WEEKDAYS.iter().enumerate() {
                    let x = inner.left() + index as f32 * (cell_width + GAP) + cell_width / 2.0;
                    ui.painter().text(
                        egui::pos2(x, top + WEEKDAY_ROW / 2.0),
                        Align2::CENTER_CENTER,
                        *day,
                        text::label_font(ui.ctx(), 11.0),
                        tokens::OVERLAY_TEXT_MUTED,
                    );
                }
                top += WEEKDAY_ROW;
            }
            let mut picked = None;
            for (index, cell) in grid.iter().enumerate() {
                let Some((start, label)) = cell else {
                    continue;
                };
                let (row, col) = (index / cols, index % cols);
                let rect = Rect::from_min_size(
                    egui::pos2(
                        inner.left() + col as f32 * (cell_width + GAP),
                        top + row as f32 * (cell_height + GAP),
                    ),
                    egui::vec2(cell_width, cell_height),
                );
                let enabled = *start >= bounds.floor && *start <= bounds.current;
                let selected = *start == bounds.selected;
                let response = ui.interact(rect, id.with(("case", index)), Sense::click());
                let response = if enabled {
                    response.on_hover_cursor(egui::CursorIcon::PointingHand)
                } else {
                    response
                };
                if selected {
                    ui.painter().rect_filled(rect, 4.0, tokens::OVERLAY_ACCENT);
                } else if enabled && response.hovered() {
                    ui.painter()
                        .rect_filled(rect, 4.0, Color32::from_white_alpha(18));
                }
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    label,
                    if selected {
                        text::label_strong_font(ui.ctx(), 13.0)
                    } else {
                        text::label_font(ui.ctx(), 13.0)
                    },
                    if selected {
                        Color32::BLACK
                    } else if enabled {
                        Color32::WHITE
                    } else {
                        tokens::TEXT_DISABLED
                    },
                );
                if enabled && response.clicked() {
                    picked = Some(*start);
                }
            }
            picked
        });
    if area.inner.is_some()
        || area.response.clicked_elsewhere()
        || ui.input(|i| i.key_pressed(egui::Key::Escape))
    {
        *close = true;
    }
    area.inner
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn un_mois_commence_un_lundi_et_finit_sur_une_semaine_pleine() {
        // Novembre 2026 commence un dimanche : six cases vides avant le 1ᵉʳ.
        let grid = cells(Granularity::Day, date(2026, 11, 18));
        assert_eq!(grid.iter().take_while(|cell| cell.is_none()).count(), 6);
        assert_eq!(grid[6], Some((date(2026, 11, 1), "1".to_string())));
        assert_eq!(grid.iter().flatten().count(), 30);
        assert_eq!(grid.len() % 7, 0);
        assert_eq!(
            page_label(Granularity::Day, date(2026, 11, 18)),
            "Novembre 2026"
        );
    }

    #[test]
    fn mois_et_annees_par_page() {
        let months = cells(Granularity::Month, date(2026, 11, 18));
        assert_eq!(months.len(), 12);
        assert_eq!(months[7], Some((date(2026, 8, 1), "août".to_string())));
        let years = cells(Granularity::Year, date(2026, 11, 18));
        assert_eq!(
            years.first(),
            Some(&Some((date(2020, 1, 1), "2020".to_string())))
        );
        assert_eq!(page_label(Granularity::Year, date(2026, 3, 1)), "2020–2029");
    }

    #[test]
    fn tourner_la_page() {
        assert_eq!(
            turn_page(Granularity::Day, date(2026, 1, 15), -1),
            date(2025, 12, 1)
        );
        assert_eq!(
            turn_page(Granularity::Month, date(2026, 5, 1), 1),
            date(2027, 1, 1)
        );
        assert_eq!(
            turn_page(Granularity::Year, date(2026, 1, 1), -1),
            date(2010, 1, 1)
        );
    }
}
