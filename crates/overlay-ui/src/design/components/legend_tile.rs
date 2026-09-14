//! **Tuile à légende** du design system — un cadre au style des champs du jeu, un texte au centre,
//! et une **légende posée sur la bordure haute**, qui s'interrompt sous elle comme celle d'un cadre
//! de formulaire. Composant **feuille** (§1 du contrat).
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! let response = ui.add(
//!     design::legend_tile("Commerce", "gelano")
//!         .width(155.0)
//!         .tooltip("Retirer")
//!         .log_name("chat.recherche"),
//! );
//! ```
//!
//! ## À quoi elle sert
//!
//! À poser côte à côte des entrées qui ont **deux informations de poids inégal** : la légende dit
//! la catégorie (un canal de chat), le contenu dit la chose (le mot recherché). Née pour l'onglet
//! Chat (maquettes du 2026-09-13, planche « recherches en tuiles », retenue par l'utilisateur contre
//! la liste en lignes : deux fois plus d'entrées visibles d'un coup, et le canal lisible sans
//! couleur).
//!
//! ## Ce que la tuile fait, et ne fait pas
//!
//! - Elle **réserve elle-même la place de sa légende** au-dessus du cadre : un appelant qui l'empile
//!   n'a pas à savoir de combien la légende déborde. `height` est la hauteur du CADRE ; la boîte
//!   allouée le dépasse de la moitié de la légende.
//! - Le contenu est **élidé sur une ligne**, jamais rogné en silence, et l'infobulle de l'appelant
//!   (`tooltip`) est la seule : un contenu coupé se lit dans l'infobulle que l'appelant pose, avec
//!   le mot entier s'il le souhaite.
//! - Survolée, elle se **voile** comme les tuiles d'Alertes et du Suivi. La croix de retrait, elle,
//!   reste au panneau (`panels::alerts_tab::alert_item`) : une `Response` ne porte qu'un clic, et
//!   la croix est un second clic sur une zone à part.
//! - Cliquable : `Sense::click()`, comme toute tuile du jeu ; c'est l'appelant qui décide de ce
//!   que le clic fait.

use egui::{Color32, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};

use crate::design::{text, tokens};

/// État de rendu, forcé par [`LegendTile::preview_state`] — galerie et captures seulement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendTileState {
    Idle,
    Hovered,
    Disabled,
}

/// De quel côté de la bordure haute la légende se pose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendSide {
    Left,
    Right,
}

/// Construit une tuile à légende.
pub fn legend_tile(legend: impl Into<String>, text: impl Into<String>) -> LegendTile {
    LegendTile {
        legend: legend.into(),
        text: text.into(),
        width: None,
        height: tokens::LEGEND_TILE_HEIGHT,
        enabled: true,
        legend_color: None,
        tooltip: None,
        log_name: None,
        forced_state: None,
    }
}

/// Voir [`legend_tile`].
pub struct LegendTile {
    legend: String,
    text: String,
    width: Option<f32>,
    height: f32,
    enabled: bool,
    legend_color: Option<Color32>,
    tooltip: Option<String>,
    log_name: Option<String>,
    forced_state: Option<LegendTileState>,
}

impl LegendTile {
    /// Largeur imposée. Par défaut, toute la largeur disponible.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Hauteur du **cadre** (la légende s'y ajoute au-dessus, voir la doc de module). Par défaut
    /// `tokens::LEGEND_TILE_HEIGHT`.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Couleur de la légende — celle du canal de chat (`tokens::chat_channel_color`), par exemple.
    /// Par défaut le gris des titres de section (`LEGEND_TILE_LEGEND_TEXT`).
    pub fn legend_color(mut self, color: Color32) -> Self {
        self.legend_color = Some(color);
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Force l'état peint — **galerie et captures uniquement** : en rendu offscreen, aucun pointeur
    /// ne survole quoi que ce soit.
    pub fn preview_state(mut self, state: LegendTileState) -> Self {
        self.forced_state = Some(state);
        self
    }

    /// Peint **le cadre seul** — fond, bordure interrompue sous la légende, légende — pour un
    /// appelant qui compose lui-même son contenu dedans (la carte d'alerte de chat du Suivi,
    /// `panels::watchlist`). `frame` est le cadre ; la légende déborde au-dessus de
    /// `legend_overshoot`. `tint` s'applique à chaque couleur (grisé, fondu d'entrée…).
    pub fn paint_frame(
        painter: &egui::Painter,
        ctx: &egui::Context,
        frame: Rect,
        legend: &str,
        legend_color: Color32,
        side: LegendSide,
        tint: &dyn Fn(Color32) -> Color32,
    ) {
        painter.rect_filled(frame, 0.0, tint(tokens::LEGEND_TILE_FILL));

        let legend = painter.layout_no_wrap(
            legend.to_owned(),
            text::label_font(ctx, tokens::LEGEND_TILE_LEGEND_FONT_SIZE),
            tint(legend_color),
        );
        let inset = tokens::LEGEND_TILE_LEGEND_INSET;
        let legend_width = legend.size().x.min(frame.width() - inset * 2.0);
        let legend_left = match side {
            LegendSide::Left => frame.left() + inset,
            LegendSide::Right => frame.right() - inset - legend_width,
        };
        let legend_right = legend_left + legend_width;
        let stroke = Stroke::new(
            tokens::LEGEND_TILE_BORDER_WIDTH,
            tint(tokens::LEGEND_TILE_BORDER),
        );
        let half = tokens::LEGEND_TILE_BORDER_WIDTH / 2.0;
        let (top, bottom) = (frame.top() + half, frame.bottom() - half);
        let (left, right) = (frame.left() + half, frame.right() - half);
        let gap = tokens::LEGEND_TILE_LEGEND_GAP;
        painter.line_segment(
            [egui::pos2(left, top), egui::pos2(legend_left - gap, top)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(legend_right + gap, top), egui::pos2(right, top)],
            stroke,
        );
        painter.line_segment([egui::pos2(left, top), egui::pos2(left, bottom)], stroke);
        painter.line_segment([egui::pos2(right, top), egui::pos2(right, bottom)], stroke);
        painter.line_segment(
            [egui::pos2(left, bottom), egui::pos2(right, bottom)],
            stroke,
        );
        painter.galley(
            egui::pos2(legend_left, frame.top() + half - legend.size().y / 2.0),
            legend,
            Color32::WHITE,
        );
    }

    /// De combien la légende dépasse au-dessus du cadre — la moitié de sa hauteur de ligne.
    pub fn legend_overshoot(ui: &Ui) -> f32 {
        let line = ui
            .painter()
            .layout_no_wrap(
                "Ag".to_owned(),
                text::label_font(ui.ctx(), tokens::LEGEND_TILE_LEGEND_FONT_SIZE),
                Color32::WHITE,
            )
            .size()
            .y;
        (line / 2.0).ceil()
    }

    /// Hauteur totale allouée par une tuile de cadre `height` — pour un appelant qui réserve.
    pub fn allocated_height(ui: &Ui, height: f32) -> f32 {
        height + Self::legend_overshoot(ui)
    }
}

impl Widget for LegendTile {
    fn ui(self, ui: &mut Ui) -> Response {
        let name = self.log_name.clone().unwrap_or_else(|| self.text.clone());
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let overshoot = Self::legend_overshoot(ui);

        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (allocated, response) =
            ui.allocate_exact_size(Vec2::new(width, self.height + overshoot), sense);
        let frame = Rect::from_min_max(
            egui::pos2(allocated.left(), allocated.top() + overshoot),
            allocated.max,
        );

        // Même condition que tous les composants (§2 du contrat) : l'appui retire le survol.
        // `contains_pointer` et non `hovered` : le panneau pose une croix PAR-DESSUS la tuile, et
        // dès que le pointeur l'atteint egui lui donne le survol — le voile disparaîtrait à
        // l'instant où l'on vise. Piège déjà payé par les tuiles d'Alertes et du Suivi.
        let state = self.forced_state.unwrap_or(if !self.enabled {
            LegendTileState::Disabled
        } else if response.contains_pointer() && !ui.input(|i| i.pointer.any_down()) {
            LegendTileState::Hovered
        } else {
            LegendTileState::Idle
        });

        if ui.is_rect_visible(allocated) {
            let painter = ui
                .painter()
                .with_clip_rect(allocated.intersect(ui.clip_rect()));
            // Désactivée : tout s'efface de l'opacité de `DISABLED_DIM`, comme le socle d'un bouton
            // icône — un multiplicateur d'alpha, pas une seconde couleur.
            let dim = |color: Color32| {
                if state == LegendTileState::Disabled {
                    color.gamma_multiply(f32::from(tokens::DISABLED_DIM.a()) / 255.0)
                } else {
                    color
                }
            };

            Self::paint_frame(
                &painter,
                ui.ctx(),
                frame,
                &self.legend,
                self.legend_color.unwrap_or(tokens::LEGEND_TILE_LEGEND_TEXT),
                LegendSide::Left,
                &dim,
            );
            let half = tokens::LEGEND_TILE_BORDER_WIDTH / 2.0;

            // Le contenu, centré, élidé s'il déborde — jamais rogné en silence.
            let mut job = egui::text::LayoutJob::simple(
                self.text.clone(),
                text::label_strong_font(ui.ctx(), tokens::LEGEND_TILE_FONT_SIZE),
                dim(tokens::LEGEND_TILE_TEXT),
                width - tokens::LEGEND_TILE_PAD_X * 2.0,
            );
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            job.wrap.overflow_character = Some('…');
            let content = painter.layout_job(job);
            if content.elided {
                // Une seule fois par instance : un contenu coupé est un événement de mise en
                // page, pas un flux à 60 Hz (§4 du contrat).
                let key = response.id.with("legend-tile-elided");
                let already = ui.data(|d| d.get_temp::<bool>(key).unwrap_or(false));
                if !already {
                    ui.data_mut(|d| d.insert_temp(key, true));
                    tracing::warn!(
                        component = "legend_tile",
                        name,
                        width,
                        "contenu élidé : la tuile est trop étroite pour son texte"
                    );
                }
            }
            let size = content.size();
            painter.galley(
                egui::pos2(
                    frame.center().x - size.x / 2.0,
                    frame.center().y - size.y / 2.0 + half,
                ),
                content,
                Color32::WHITE,
            );

            if state == LegendTileState::Hovered {
                painter.rect_filled(
                    frame.shrink(tokens::LEGEND_TILE_BORDER_WIDTH),
                    0.0,
                    tokens::LEGEND_TILE_HOVER_SCRIM,
                );
            }
        }

        if response.clicked() {
            tracing::debug!(component = "legend_tile", name, "clic");
        }
        let response = if self.enabled {
            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            response
        };
        if let Some(tooltip) = &self.tooltip {
            crate::design::tooltip(&response).text(tooltip.as_str());
        }
        response
    }
}
