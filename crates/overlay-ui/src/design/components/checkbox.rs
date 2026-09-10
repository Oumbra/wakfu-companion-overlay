//! **Case à cocher** du design system Wakfu — un réglage booléen : une case, un libellé, et le fait
//! que les deux portent l'état.
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! // La valeur vit chez l'appelant : le composant la bascule, l'appelant la relit.
//! if ui.add(design::checkbox(&mut state.alertes_sonores, "Alertes sonores")).changed() {
//!     // `Response::changed()` — la frame où la case vient d'être basculée.
//! }
//! ```
//!
//! ## Le jeu double toujours son signal
//!
//! **La couleur du libellé porte l'état en plus de la case** : blanc décoché, **doré `#f4d89e`
//! coché**. C'est le même principe que la barre d'onglets, où le libellé distingue l'actif du
//! survolé — le jeu ne se repose jamais sur un seul signal visuel. Un portage qui ne changerait que
//! la case perdrait la moitié de l'information.
//!
//! ## Ce qui a été mesuré
//!
//! Source : [`releve-section-options.json`](../../../../../docs/design-system/releve-section-options.json),
//! nœuds `cb1` à `cb3` et leurs libellés.
//!
//! | Grandeur | Valeur | Détail |
//! | --- | --- | --- |
//! | Case | **20 × 20 px** | `cb1` en `[36, 173, 56, 193]`, et les deux assets font exactement 20 × 20 |
//! | Rayon | **0** | **le seul élément carré de l'interface** — tout le reste est à 2, sauf le champ de saisie à 4 |
//! | Écart case → libellé | **6 px** | la case finit à x=56, le libellé commence à x=62 |
//! | Corps du libellé | **15 px** (encre 10) | jeton `libellé d'option` de `releve-modale-options.json` — plus petit que les 17 px d'un libellé de bouton |
//! | Couleur du libellé | **blanc décoché, doré coché** | voir ci-dessus |
//! | Rythme | 31 px entre deux lignes de case | affaire de la mise en page, pas du composant |
//!
//! ## Ce qui n'a PAS de référence, et est donc inventé
//!
//! Les deux mêmes états que `design::input`, et pour la même raison — aucune capture n'en existe :
//!
//! - **Survolé** : le composant ne change **rien** au survol, seulement le curseur de la souris
//!   (`CursorIcon::PointingHand`). Inventer un éclaircissement serait inventer du design.
//! - **Désactivé** : la case est teintée de `TEXT_DISABLED` et le libellé aussi, par cohérence avec
//!   le bouton désactivé. À remplacer par une mesure dès qu'une capture existe.

use egui::{Align2, Response, Sense, Ui, Vec2, Widget};

use crate::design::{assets::DsTexture, text, tokens, DesignSystem};

/// État visuel d'une case — les trois du contrat, `Hovered` étant délibérément identique à `Idle`
/// faute de référence (doc de module).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckboxState {
    Idle,
    Hovered,
    Disabled,
}

/// Construit une case à cocher sur `checked`. Point d'entrée unique — voir la doc de module.
pub fn checkbox<'a>(checked: &'a mut bool, label: impl Into<String>) -> Checkbox<'a> {
    Checkbox::new(checked, label)
}

pub struct Checkbox<'a> {
    checked: &'a mut bool,
    label: String,
    enabled: bool,
    tooltip: Option<String>,
    log_name: Option<String>,
    forced_state: Option<CheckboxState>,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, label: impl Into<String>) -> Self {
        Self {
            checked,
            label: label.into(),
            enabled: true,
            tooltip: None,
            log_name: None,
            forced_state: None,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Nom d'instance pour la journalisation (défaut : le libellé).
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Force l'état peint, **sans passer par l'interaction** — réservé à la galerie de contrôle et
    /// aux captures, où aucun pointeur ne survole quoi que ce soit.
    pub fn preview_state(mut self, state: CheckboxState) -> Self {
        self.forced_state = Some(state);
        self
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let font = text::label_font(ui.ctx(), tokens::CHECKBOX_FONT_SIZE);
        let galley = ui.fonts_mut(|f| {
            f.layout_no_wrap(self.label.clone(), font.clone(), tokens::CHECKBOX_LABEL_OFF)
        });
        // La ligne est aussi haute que le plus haut des deux : la case (20) ou son libellé. Le jeu
        // n'a que des libellés d'une ligne à ce corps, donc c'est toujours la case en pratique —
        // mais un libellé au corps relevé de demain n'aurait pas à être un cas particulier.
        let height = tokens::CHECKBOX_SIZE.max(galley.size().y);
        let width = tokens::CHECKBOX_SIZE + tokens::CHECKBOX_LABEL_GAP + galley.size().x;

        // Toute la ligne est cliquable, case ET libellé : c'est ce que fait le jeu, et ce à quoi
        // s'attend n'importe qui — viser un carré de 20px à la souris est une punition.
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, mut response) = ui.allocate_exact_size(Vec2::new(width, height), sense);

        let state = self.forced_state.unwrap_or({
            if !self.enabled {
                CheckboxState::Disabled
            } else if response.hovered() {
                CheckboxState::Hovered
            } else {
                CheckboxState::Idle
            }
        });

        if response.clicked() {
            *self.checked = !*self.checked;
            response.mark_changed();
            tracing::debug!(
                component = "checkbox",
                name = self.log_name.as_deref().unwrap_or(&self.label),
                coche = *self.checked,
                "bascule"
            );
        }

        if ui.is_rect_visible(rect) {
            let box_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.center().y - tokens::CHECKBOX_SIZE / 2.0),
                Vec2::splat(tokens::CHECKBOX_SIZE),
            );
            let texture = if *self.checked {
                DsTexture::CheckboxChecked
            } else {
                DsTexture::CheckboxUnchecked
            };
            // Teinte neutre sauf désactivé : un `tint` egui multiplie, il ne peut que ternir — ce
            // qui est exactement l'effet voulu pour un contrôle grisé, et rien de plus.
            let tint = match state {
                CheckboxState::Idle | CheckboxState::Hovered => egui::Color32::WHITE,
                CheckboxState::Disabled => tokens::TEXT_DISABLED,
            };
            DesignSystem::get(ui.ctx()).paint(ui.painter(), box_rect, texture, tint);

            let label_color = match state {
                CheckboxState::Disabled => tokens::TEXT_DISABLED,
                // Le libellé porte l'état AUTANT que la case — voir la doc de module.
                _ if *self.checked => tokens::CHECKBOX_LABEL_ON,
                _ => tokens::CHECKBOX_LABEL_OFF,
            };
            ui.painter()
                .with_clip_rect(rect.intersect(ui.clip_rect()))
                .text(
                    egui::pos2(
                        box_rect.right() + tokens::CHECKBOX_LABEL_GAP,
                        rect.center().y,
                    ),
                    Align2::LEFT_CENTER,
                    &self.label,
                    font,
                    label_color,
                );
        }

        let response = if self.enabled {
            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            response
        };
        match self.tooltip {
            Some(tooltip) => response.on_hover_text(tooltip),
            None => response,
        }
    }
}
