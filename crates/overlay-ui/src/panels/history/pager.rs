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

use egui::{Rect, Sense, Ui};

use super::format;
use crate::design::{self, text, tokens, DsIcon};

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
}

impl Default for Pager {
    fn default() -> Self {
        Self { target: PAGE_SIZE }
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

    /// Le pied de liste (`history-load-more` du site), **épinglé en bas du panneau**, hors de la
    /// zone défilable — retour utilisateur du 2026-11-27 : il ne suit pas les résultats, il reste au
    /// pied. Seulement s'il reste des entrées à montrer : rend alors les zones réduites au-dessus
    /// du pied, où l'appelant pose sa zone défilable.
    pub fn footer(
        &mut self,
        ui: &mut Ui,
        zones: &design::PanelZones,
        times: &[i64],
        shown: usize,
        log_name: &str,
    ) -> Option<design::PanelZones> {
        if shown >= times.len() {
            return None;
        }
        let (reduced, band) = zones.footer(FOOTER_HEIGHT, FOOTER_MARGIN);
        // Filet haut (`border-top` du site), puis le contrôle scindé au centre.
        ui.painter()
            .with_clip_rect(band.expand(FOOTER_MARGIN))
            .hline(
                band.x_range(),
                band.top() - FOOTER_MARGIN / 2.0,
                egui::Stroke::new(1.0, egui::Color32::from_white_alpha(14)),
            );
        let total = LOAD_WIDTH + CARET_WIDTH;
        let main = Rect::from_min_size(
            egui::pos2(band.center().x - total / 2.0, band.top()),
            egui::vec2(LOAD_WIDTH, band.height()),
        );
        let caret = Rect::from_min_size(
            egui::pos2(main.right(), band.top()),
            egui::vec2(CARET_WIDTH, band.height()),
        );
        // Les deux moitiés se touchent sans arrondi entre elles (`border-radius: 4px 0 0 4px` et
        // `0 4px 4px 0` du site) : chaque bouton est peint plus large, du côté de la jointure, et
        // écrêté à sa moitié — ses coins arrondis de ce côté tombent hors champ.
        let mut main_ui = ui.new_child(egui::UiBuilder::new().max_rect(band));
        main_ui.set_clip_rect(main);
        if main_ui
            .put(
                Rect::from_min_size(main.min, main.size() + egui::vec2(GLUE, 0.0)),
                design::button("Charger plus")
                    .size(design::ButtonSize::Height(band.height()))
                    .width(LOAD_WIDTH + GLUE)
                    .log_name(format!("{log_name}.charger-plus")),
            )
            .clicked()
        {
            self.load_more(shown);
        }
        let mut caret_ui = ui.new_child(egui::UiBuilder::new().max_rect(band));
        caret_ui.set_clip_rect(caret);
        let caret_response = caret_ui
            .put(
                Rect::from_min_max(caret.min - egui::vec2(GLUE, 0.0), caret.max),
                design::button("")
                    .size(design::ButtonSize::Height(band.height()))
                    .width(CARET_WIDTH + GLUE)
                    .log_name(format!("{log_name}.portee")),
            )
            .on_hover_text("Choisir la portée du chargement");
        // Fin filet doré entre les deux, et le chevron à la couleur du libellé.
        caret_ui.painter().vline(
            caret.left() + 0.5,
            caret.y_range().shrink(5.0),
            egui::Stroke::new(1.0, tokens::TEXT_GOLD.gamma_multiply(0.8)),
        );
        super::paint_icon(
            &caret_ui,
            caret_ui.painter(),
            caret.center(),
            DsIcon::ChevronDown,
            11.0,
            tokens::BUTTON_TEXT_ON_KHAKI,
        );
        let child = caret_ui;
        if let Some(span) = span_menu(&child, &caret_response, log_name) {
            let now_ms = chrono::Utc::now().timestamp_millis();
            self.load_span(span, times, now_ms);
        }
        Some(reduced)
    }
}

/// Hauteur du pied de liste et sa marge au bord du panneau.
const FOOTER_HEIGHT: f32 = 32.0;
const FOOTER_MARGIN: f32 = 8.0;
/// La partie « Charger plus » du contrôle scindé, et celle du chevron accolée à sa droite.
const LOAD_WIDTH: f32 = 150.0;
const CARET_WIDTH: f32 = 30.0;
/// Ce que chaque moitié déborde sous l'autre, le temps d'y cacher ses coins arrondis.
const GLUE: f32 = 8.0;
/// Largeur du menu des portées.
const MENU_WIDTH: f32 = 140.0;

/// Le menu des portées (`load-more-scope-menu` du site), ouvert au-dessus du chevron : rend la
/// portée choisie à la frame du clic.
fn span_menu(ui: &Ui, caret: &egui::Response, log_name: &str) -> Option<LoadSpan> {
    let open_id = caret.id.with("portee-ouverte");
    let mut open = ui.data(|d| d.get_temp::<bool>(open_id).unwrap_or(false));
    if caret.clicked() {
        open = !open;
    }
    let mut chosen = None;
    if open {
        let height = tokens::SELECT_ROW_HEIGHT * LoadSpan::ALL.len() as f32;
        let menu = Rect::from_min_size(
            egui::pos2(
                caret.rect.right() - MENU_WIDTH,
                caret.rect.top() - 4.0 - height,
            ),
            egui::vec2(MENU_WIDTH, height),
        );
        let area = egui::Area::new(open_id.with("menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(menu.min)
            .constrain(false)
            .show(ui.ctx(), |ui| {
                ui.set_min_size(menu.size());
                ui.painter()
                    .rect_filled(menu, 4.0, tokens::SELECT_LIST_FILL);
                let font = text::label_font(ui.ctx(), 14.0);
                let mut picked = None;
                for (index, span) in LoadSpan::ALL.into_iter().enumerate() {
                    let row = Rect::from_min_size(
                        egui::pos2(
                            menu.left(),
                            menu.top() + index as f32 * tokens::SELECT_ROW_HEIGHT,
                        ),
                        egui::vec2(MENU_WIDTH, tokens::SELECT_ROW_HEIGHT),
                    );
                    let response = ui
                        .interact(row, open_id.with(("portee", index)), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if response.hovered() {
                        ui.painter()
                            .rect_filled(row, 4.0, tokens::SELECT_ROW_HIGHLIGHT);
                    }
                    ui.painter().text(
                        egui::pos2(row.left() + tokens::SELECT_ROW_PADDING_X, row.center().y),
                        egui::Align2::LEFT_CENTER,
                        span.label(),
                        font.clone(),
                        tokens::SELECT_TEXT,
                    );
                    if response.clicked() {
                        picked = Some(span);
                    }
                }
                ui.painter().rect_stroke(
                    menu,
                    4.0,
                    egui::Stroke::new(2.0, tokens::SELECT_LIST_BORDER),
                    egui::StrokeKind::Inside,
                );
                picked
            });
        if let Some(span) = area.inner {
            tracing::info!(">>> clic {log_name}.portee {}", span.label());
            chosen = Some(span);
            open = false;
        } else if area.response.clicked_elsewhere() && caret.clicked_elsewhere() {
            open = false;
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            open = false;
        }
    }
    ui.data_mut(|d| d.insert_temp(open_id, open));
    chosen
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
