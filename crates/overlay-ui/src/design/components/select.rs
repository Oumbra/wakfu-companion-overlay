//! **Liste déroulante** du design system Wakfu — le second débloqueur de contenu : tout réglage à
//! choix multiple en dépend.
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! // Choix simple : la valeur vit chez l'appelant.
//! ui.add(
//!     design::select(&mut state.theme)
//!         .option(Theme::Sombre, "Sombre")
//!         .option(Theme::Clair, "Clair")
//!         .width(560.0),
//! );
//!
//! // Choix multiple : une case par entrée, et un résumé sur le socle.
//! ui.add(
//!     design::select_multi(&mut state.raretes)
//!         .option(Rarete::Commun, "Commun")
//!         .option(Rarete::Rare, "Rare")
//!         .summary("Toutes"),
//! );
//! ```
//!
//! ## Le seul composant qui peint hors de son rectangle
//!
//! Déplié, il ouvre une liste **par-dessus le contenu qui le suit**. Elle est peinte dans une
//! `egui::Area` au premier plan, pas dans le `Ui` courant : sans ça, elle serait recouverte par le
//! widget suivant, et la place qu'elle occupe décalerait la mise en page à chaque ouverture.
//!
//! L'état ouvert/fermé vit dans la mémoire d'egui, indexé sur l'id du widget — pas chez l'appelant.
//! Ce n'est pas de l'état applicatif (le contrat de composant en interdit) mais de l'état
//! d'interaction, du même ordre que « ce widget a le focus » : il ne survit pas à la fermeture de la
//! fenêtre et personne d'autre n'a de raison de le lire. `egui::ComboBox` procède de même.
//!
//! ## Ce qui a été mesuré
//!
//! Sur `select-simple.png` (le socle, colonne x=60) et `select-simple-opened.png` (la liste,
//! colonne x=100), recoupés avec le nœud `select-theme` de
//! [`releve-options-interface.json`](../../../../../docs/design-system/releve-options-interface.json).
//!
//! | Grandeur | Valeur | Détail |
//! | --- | --- | --- |
//! | Hauteur du socle | **36 px, bord compris** | bord 2 + liseré 2 + dégradé 28 + ombre 2 + bord 2 |
//! | Rayon | 2 | comme tout le reste de l'interface |
//! | Chevron | **14 × 8 px**, à 8 px du bord droit | exactement la taille native d'`icons/icon-chevron-down.png` |
//! | Retrait du libellé de socle | 10 px | le texte commence à x=17 pour un socle à x=7 |
//! | Hauteur d'une entrée | **28 px** | surbrillance en y 71..98, entrée suivante à 99 |
//! | Retrait du texte d'une entrée | 12 px | « Tous » commence à x=19 pour une liste à x=7 |
//! | Fond de liste | `#675d46` | uniforme, aucun dégradé |
//! | Fond d'une entrée mise en avant | `#a58e63` | voir l'ambiguïté ci-dessous |
//!
//! > **Le piège de la hauteur**, énoncé par le relevé : « le chiffre de 32 px qu'on lit en mesurant
//! > le remplissage est trompeur — il exclut les 2 px de bord haut et bas. » Vérifié au pixel : le
//! > socle occupe y 5..40 inclus, soit 36.
//!
//! > **Il n'existe pas de largeur de liste unique.** Le relevé mesure 210, 208, 560 et 650 px selon
//! > le contrôle : « la largeur est décidée contrôle par contrôle ». Le composant n'impose donc
//! > aucune largeur par défaut autre que la place disponible, comme `design::input`.
//!
//! ## Ce qui n'a PAS de référence, et est donc inventé
//!
//! - **La distinction entre « survolée » et « valeur courante ».** Une seule capture montre une
//!   entrée sur fond clair (`#a58e63`), et elle est à la fois la valeur du socle et, très
//!   probablement, celle que la souris survolait. Les deux lectures sont possibles et rien ne les
//!   départage. Le composant applique donc ce même fond aux deux — c'est le seul fond de mise en
//!   avant relevé, et le jeu n'en a pas montré de second.
//! - **Le gabarit propre au mode multiple.** `select-multiple.png` donne une entrée de 26 px là où
//!   `select-simple-opened.png` en donne 28 ; les deux captures ne sont pas à la même échelle
//!   d'interface. Le composant applique **les cotes du simple** dans les deux modes, et ajoute la
//!   case à cocher du design system à l'axe du texte.
//! - **L'état désactivé** : socle et libellé teintés de `TEXT_DISABLED`, comme partout ailleurs, et
//!   la liste ne s'ouvre pas.

use egui::{Align2, Response, Sense, Ui, Vec2, Widget};

use crate::design::{assets::DsTexture, icons::DsIcon, text, tokens, DesignSystem};

/// État visuel du socle. `Hovered` est identique à `Idle` faute de capture d'un socle survolé —
/// même parti pris que `design::input`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectState {
    Idle,
    Hovered,
    Disabled,
}

/// Ce sur quoi le composant écrit : une valeur, ou un ensemble de valeurs.
enum Selection<'a, T> {
    Single(&'a mut T),
    Multiple(&'a mut Vec<T>),
}

/// Construit une liste déroulante à choix simple sur `selected`.
pub fn select<T: PartialEq + Clone>(selected: &mut T) -> Select<'_, T> {
    Select::new(Selection::Single(selected))
}

/// Construit une liste déroulante à choix multiple sur `selected` — l'ordre du `Vec` est celui des
/// clics, pas celui des options.
pub fn select_multi<T: PartialEq + Clone>(selected: &mut Vec<T>) -> Select<'_, T> {
    Select::new(Selection::Multiple(selected))
}

pub struct Select<'a, T> {
    selection: Selection<'a, T>,
    options: Vec<(T, String)>,
    width: Option<f32>,
    placeholder: Option<String>,
    summary: Option<String>,
    enabled: bool,
    log_name: Option<String>,
    forced_state: Option<SelectState>,
    /// Force la liste ouverte — voir [`Select::preview_open`].
    forced_open: Option<bool>,
    /// Force l'entrée peinte en survol — voir [`Select::preview_hovered`].
    forced_hover: Option<usize>,
}

impl<'a, T: PartialEq + Clone> Select<'a, T> {
    fn new(selection: Selection<'a, T>) -> Self {
        Self {
            selection,
            options: Vec::new(),
            width: None,
            placeholder: None,
            summary: None,
            enabled: true,
            log_name: None,
            forced_state: None,
            forced_open: None,
            forced_hover: None,
        }
    }

    /// Ajoute une entrée à la liste. L'ordre d'appel est l'ordre d'affichage.
    pub fn option(mut self, value: T, label: impl Into<String>) -> Self {
        self.options.push((value, label.into()));
        self
    }

    /// Largeur imposée. **Sans elle, le socle prend toute la largeur disponible** — le relevé est
    /// formel, « la largeur est décidée contrôle par contrôle », il n'y a pas de largeur de liste
    /// à laquelle se raccrocher.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Libellé du socle quand la valeur courante ne correspond à aucune option (choix simple).
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Libellé du socle en choix multiple — c'est ainsi que le jeu écrit « Toutes ». Sans lui, le
    /// socle affiche le nombre d'entrées retenues.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Nom d'instance pour la journalisation (défaut : `"select"`).
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Force l'état peint du socle — galerie et captures uniquement.
    pub fn preview_state(mut self, state: SelectState) -> Self {
        self.forced_state = Some(state);
        self
    }

    /// Force la liste ouverte ou fermée — galerie et captures uniquement : en rendu offscreen,
    /// personne ne clique sur le socle pour la déplier.
    pub fn preview_open(mut self, open: bool) -> Self {
        self.forced_open = Some(open);
        self
    }

    /// Force une entrée peinte en survol — galerie et captures uniquement.
    pub fn preview_hovered(mut self, index: usize) -> Self {
        self.forced_hover = Some(index);
        self
    }

    /// Alias ergonomique d'`ui.add(...)`, lisible quand le composant porte une liste d'options
    /// chaînées.
    pub fn show(self, ui: &mut Ui) -> Response {
        ui.add(self)
    }

    /// Ce qu'affiche le socle.
    fn face_label(&self) -> String {
        match &self.selection {
            Selection::Single(value) => self
                .options
                .iter()
                .find(|(candidate, _)| candidate == *value)
                .map(|(_, label)| label.clone())
                .or_else(|| self.placeholder.clone())
                .unwrap_or_default(),
            Selection::Multiple(values) => {
                self.summary.clone().unwrap_or_else(|| match values.len() {
                    0 => "Aucune".to_owned(),
                    1 => "1 sélectionnée".to_owned(),
                    n => format!("{n} sélectionnées"),
                })
            }
        }
    }

    fn is_checked(&self, value: &T) -> bool {
        match &self.selection {
            Selection::Single(current) => *current == value,
            Selection::Multiple(values) => values.contains(value),
        }
    }

    /// Applique un clic sur l'entrée `index`. Rend `true` si la liste doit se refermer — un choix
    /// simple ferme, un choix multiple reste ouvert pour qu'on puisse en cocher plusieurs.
    fn apply_click(&mut self, index: usize) -> bool {
        let value = self.options[index].0.clone();
        match &mut self.selection {
            Selection::Single(current) => {
                **current = value;
                true
            }
            Selection::Multiple(values) => {
                if let Some(position) = values.iter().position(|v| *v == value) {
                    values.remove(position);
                } else {
                    values.push(value);
                }
                false
            }
        }
    }

    fn is_multiple(&self) -> bool {
        matches!(self.selection, Selection::Multiple(_))
    }
}

impl<T: PartialEq + Clone> Widget for Select<'_, T> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let (rect, mut response) = ui.allocate_exact_size(
            Vec2::new(width, tokens::SELECT_HEIGHT),
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let name = self.log_name.clone().unwrap_or_else(|| "select".to_owned());

        // Ouvert/fermé : mémoire egui, indexée sur l'id du widget — voir la doc de module.
        let open_id = response.id.with("ds-select-open");
        let mut open = self
            .forced_open
            .unwrap_or_else(|| ui.data(|d| d.get_temp::<bool>(open_id).unwrap_or(false)));
        if response.clicked() && self.enabled {
            open = !open;
            tracing::debug!(component = "select", name, ouvert = open, "socle cliqué");
        }

        let state = self.forced_state.unwrap_or({
            if !self.enabled {
                SelectState::Disabled
            } else if response.hovered() {
                SelectState::Hovered
            } else {
                SelectState::Idle
            }
        });
        let tint = match state {
            SelectState::Idle | SelectState::Hovered => egui::Color32::WHITE,
            SelectState::Disabled => tokens::TEXT_DISABLED,
        };
        let text_color = match state {
            SelectState::Disabled => tokens::TEXT_DISABLED,
            _ => tokens::SELECT_TEXT,
        };
        let font = text::label_font(ui.ctx(), tokens::SELECT_FONT_SIZE);
        let design = DesignSystem::get(ui.ctx());

        if ui.is_rect_visible(rect) {
            design.paint(ui.painter(), rect, DsTexture::SelectFace, tint);
            let chevron = egui::Rect::from_min_size(
                egui::pos2(
                    rect.right() - tokens::SELECT_CHEVRON_MARGIN - tokens::SELECT_CHEVRON_SIZE.x,
                    (rect.center().y - tokens::SELECT_CHEVRON_SIZE.y / 2.0).round(),
                ),
                tokens::SELECT_CHEVRON_SIZE,
            );
            design.paint_icon(ui.painter(), chevron, DsIcon::ChevronDown, text_color);
            // Le libellé s'arrête AVANT le chevron : sans cet écrêtage, une valeur longue passerait
            // dessous et le contrôle n'aurait plus l'air d'un contrôle.
            let label_clip = egui::Rect::from_min_max(
                egui::pos2(rect.left() + tokens::SELECT_PADDING_X, rect.top()),
                egui::pos2(chevron.left() - tokens::SELECT_PADDING_X, rect.bottom()),
            );
            ui.painter()
                .with_clip_rect(label_clip.intersect(ui.clip_rect()))
                .text(
                    egui::pos2(rect.left() + tokens::SELECT_PADDING_X, rect.center().y),
                    Align2::LEFT_CENTER,
                    self.face_label(),
                    font.clone(),
                    text_color,
                );
        }

        if open && self.enabled && !self.options.is_empty() {
            let list_height = tokens::SELECT_ROW_HEIGHT * self.options.len() as f32;
            let list_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.bottom()),
                Vec2::new(width, list_height),
            );
            // `Area` au premier plan : la liste sort du flux, donc ni le widget suivant ne la
            // recouvre, ni son ouverture ne décale la mise en page. `Order::Foreground` la place
            // au-dessus du panneau sans passer devant les infobulles.
            let area = egui::Area::new(response.id.with("ds-select-popup"))
                .order(egui::Order::Foreground)
                .fixed_pos(list_rect.min)
                .constrain(false)
                .show(ui.ctx(), |ui| {
                    ui.set_min_size(list_rect.size());
                    let painter = ui.painter();
                    painter.rect_filled(list_rect, tokens::SELECT_RADIUS, tokens::SELECT_LIST_FILL);
                    // Liseré clair d'un pixel en haut de la liste : c'est ce qui la détache du
                    // socle, dont l'ombre basse est de la même famille.
                    painter.rect_filled(
                        egui::Rect::from_min_size(list_rect.min, Vec2::new(width, 1.0)),
                        0,
                        tokens::SELECT_LIST_TOP_LINE,
                    );
                    painter.rect_stroke(
                        list_rect,
                        tokens::SELECT_RADIUS,
                        egui::Stroke::new(2.0, tokens::SELECT_LIST_BORDER),
                        egui::StrokeKind::Inside,
                    );

                    let mut clicked = None;
                    for (index, (_, label)) in self.options.iter().enumerate() {
                        let row = egui::Rect::from_min_size(
                            egui::pos2(
                                list_rect.left(),
                                list_rect.top() + tokens::SELECT_ROW_HEIGHT * index as f32,
                            ),
                            Vec2::new(width, tokens::SELECT_ROW_HEIGHT),
                        );
                        let row_response = ui.interact(
                            row,
                            response.id.with(("ds-select-row", index)),
                            Sense::click(),
                        );
                        let checked = self.is_checked(&self.options[index].0);
                        // Survolée et « valeur courante » partagent le même fond : c'est le seul
                        // fond de mise en avant relevé, et rien ne dit qu'il en existe un second.
                        let highlighted = self.forced_hover == Some(index)
                            || row_response.hovered()
                            || (!self.is_multiple() && checked);
                        if highlighted {
                            ui.painter()
                                .rect_filled(row, 0, tokens::SELECT_ROW_HIGHLIGHT);
                        }
                        let mut text_x = row.left() + tokens::SELECT_ROW_PADDING_X;
                        if self.is_multiple() {
                            let box_rect = egui::Rect::from_min_size(
                                egui::pos2(
                                    text_x,
                                    (row.center().y - tokens::CHECKBOX_SIZE / 2.0).round(),
                                ),
                                Vec2::splat(tokens::CHECKBOX_SIZE),
                            );
                            let texture = if checked {
                                DsTexture::CheckboxChecked
                            } else {
                                DsTexture::CheckboxUnchecked
                            };
                            design.paint(ui.painter(), box_rect, texture, egui::Color32::WHITE);
                            text_x = box_rect.right() + tokens::CHECKBOX_LABEL_GAP;
                        }
                        ui.painter()
                            .with_clip_rect(row.intersect(ui.clip_rect()))
                            .text(
                                egui::pos2(text_x, row.center().y),
                                Align2::LEFT_CENTER,
                                label,
                                font.clone(),
                                tokens::SELECT_TEXT,
                            );
                        if row_response.clicked() {
                            clicked = Some(index);
                        }
                    }
                    clicked
                });

            if let Some(index) = area.inner {
                let label = self.options[index].1.clone();
                if self.apply_click(index) {
                    open = false;
                }
                response.mark_changed();
                tracing::debug!(component = "select", name, entree = label, "entrée choisie");
            } else if area.response.clicked_elsewhere() && response.clicked_elsewhere() {
                // Clic à côté : on referme, comme n'importe quelle liste déroulante. Les deux
                // conditions sont nécessaires — sans la seconde, le clic qui vient d'ouvrir la
                // liste la refermerait dans la même frame.
                open = false;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                open = false;
            }
        }

        if self.forced_open.is_none() {
            ui.data_mut(|d| d.insert_temp(open_id, open));
        }
        if self.enabled {
            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            response
        }
    }
}
