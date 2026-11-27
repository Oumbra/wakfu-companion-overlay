//! **Chargement progressif** des listes de l'historique — port du « Charger plus » du site
//! (`history.component.html`, `HistoryArchiveService.loadMore` / `loadMoreForSpan`).
//!
//! Le site recevait l'historique du serveur par pages de 50 ; l'overlay le lit dans sa base, mais
//! n'en montre pas plus d'un coup : une liste commence aux [`PAGE_SIZE`] entrées les plus
//! récentes, « Charger plus » en ajoute autant, et le menu à côté charge d'un geste tout ce qui
//! tient dans une période — 1 semaine, 1 mois, 1 an — comptée depuis maintenant, comme sur le
//! site (`LOAD_MORE_SPAN_MS` : 7, 30 et 365 jours, bornes volontairement approximatives).
//!
//! Les achats se chargent **par jour complet** (`loadMorePurchasesUntilDayComplete`) : le dernier
//! jour montré n'est jamais coupé, sans quoi son total serait faux.

use egui::Ui;

use super::format;
use crate::design;

/// Entrées ajoutées par « Charger plus » (`PAGE_SIZE` du site).
pub const PAGE_SIZE: usize = 50;

/// Les portées du menu (`LoadMoreSpan`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadSpan {
    Week,
    Month,
    Year,
}

impl LoadSpan {
    pub const ALL: [LoadSpan; 3] = [LoadSpan::Week, LoadSpan::Month, LoadSpan::Year];

    /// `history.source.loadWeek` / `loadMonth` / `loadYear`.
    pub fn label(self) -> &'static str {
        match self {
            LoadSpan::Week => "1 semaine",
            LoadSpan::Month => "1 mois",
            LoadSpan::Year => "1 an",
        }
    }

    /// `LOAD_MORE_SPAN_MS`.
    pub fn millis(self) -> i64 {
        const DAY: i64 = 24 * 60 * 60 * 1000;
        match self {
            LoadSpan::Week => 7 * DAY,
            LoadSpan::Month => 30 * DAY,
            LoadSpan::Year => 365 * DAY,
        }
    }
}

/// Combien d'entrées d'une liste sont montrées.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pager {
    /// Au moins ce nombre d'entrées (avant complétion du dernier jour).
    target: usize,
    /// Le choix du menu, remis à `None` aussitôt appliqué.
    span_choice: Option<LoadSpan>,
}

impl Default for Pager {
    fn default() -> Self {
        Self {
            target: PAGE_SIZE,
            span_choice: None,
        }
    }
}

impl Pager {
    /// Nombre d'entrées montrées parmi `times` (ms UTC, de la plus récente à la plus ancienne).
    /// `whole_days` : le dernier jour montré est complété (achats).
    pub fn visible(&self, times: &[i64], whole_days: bool) -> usize {
        let mut shown = self.target.min(times.len());
        if whole_days && shown > 0 {
            let last_day = format::local_day(times[shown - 1]);
            while shown < times.len() && format::local_day(times[shown]) == last_day {
                shown += 1;
            }
        }
        shown
    }

    /// « Charger plus » : une page de plus après ce qui est montré.
    pub fn load_more(&mut self, shown: usize) {
        self.target = shown + PAGE_SIZE;
    }

    /// Le menu : tout ce qui est plus récent que `now_ms - span`, au moins.
    pub fn load_span(&mut self, span: LoadSpan, times: &[i64], now_ms: i64) {
        let from = now_ms - span.millis();
        let covered = times.iter().take_while(|&&at| at >= from).count();
        // Une entrée de plus que la période : celle qui prouve qu'elle est couverte, comme la
        // boucle du site, qui s'arrête sur la première page dont la plus ancienne la dépasse.
        self.target = self.target.max((covered + 1).min(times.len()));
    }

    /// Le pied de liste — « Charger plus » et le menu des portées —, seulement s'il reste des
    /// entrées à montrer. Rend `true` si ce qui est montré a changé.
    pub fn footer(
        &mut self,
        ui: &mut Ui,
        width: f32,
        times: &[i64],
        shown: usize,
        log_name: &str,
    ) -> bool {
        if shown >= times.len() {
            return false;
        }
        const SPAN_WIDTH: f32 = 160.0;
        const GAP: f32 = 8.0;
        let mut changed = false;
        ui.add_space(8.0);
        ui.allocate_ui_with_layout(
            egui::vec2(width, 36.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let button_width = 150.0;
                ui.add_space(((width - button_width - GAP - SPAN_WIDTH) / 2.0).max(0.0));
                if ui
                    .add(
                        design::button("Charger plus")
                            .size(design::ButtonSize::Compact)
                            .width(button_width)
                            .log_name(format!("{log_name}.charger-plus")),
                    )
                    .clicked()
                {
                    self.load_more(shown);
                    changed = true;
                }
                ui.add_space(GAP);
                let mut choice = self.span_choice;
                let mut select = design::select(&mut choice)
                    .placeholder("Période…")
                    .width(SPAN_WIDTH)
                    .log_name(format!("{log_name}.portee"));
                for span in LoadSpan::ALL {
                    select = select.option(Some(span), span.label());
                }
                ui.add(select)
                    .on_hover_text("Choisir la portée du chargement");
                if let Some(span) = choice {
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    self.load_span(span, times, now_ms);
                    changed = true;
                }
                self.span_choice = None;
            },
        );
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 60 * 60 * 1000;

    #[test]
    fn une_page_puis_une_de_plus() {
        let times: Vec<i64> = (0..120)
            .map(|i| 1_000_000_000_000 - i * 30 * HOUR)
            .collect();
        let mut pager = Pager::default();
        assert_eq!(pager.visible(&times, false), PAGE_SIZE);
        pager.load_more(PAGE_SIZE);
        assert_eq!(pager.visible(&times, false), 2 * PAGE_SIZE);
        pager.load_more(2 * PAGE_SIZE);
        assert_eq!(pager.visible(&times, false), 120);
    }

    #[test]
    fn les_achats_vont_jusqu_au_bout_du_dernier_jour() {
        // 60 achats le même jour, à une minute d'écart : la première page en montre 50, le jour
        // complet en montre 60.
        let noon = 1_795_780_800_000; // 2026-11-27 12:00 UTC
        let times: Vec<i64> = (0..60).map(|i| noon - i * 60_000).collect();
        let pager = Pager::default();
        assert_eq!(pager.visible(&times, false), 50);
        assert_eq!(pager.visible(&times, true), 60);
    }

    #[test]
    fn une_periode_couvre_tout_ce_qui_est_plus_recent() {
        let now = 1_795_780_800_000;
        // Une entrée toutes les 2 heures sur 20 jours : 85 tiennent dans la semaine, et la
        // suivante prouve qu'elle est couverte.
        let times: Vec<i64> = (0..240).map(|i| now - i * 2 * HOUR).collect();
        let mut pager = Pager::default();
        pager.load_span(LoadSpan::Week, &times, now);
        assert_eq!(pager.visible(&times, false), 86);
        // Une période plus courte que ce qui est déjà montré ne retire rien.
        pager.load_more(86);
        pager.load_span(LoadSpan::Week, &times, now);
        assert_eq!(pager.visible(&times, false), 136);
    }
}
