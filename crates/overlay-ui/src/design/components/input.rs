//! **Champ de saisie** du design system Wakfu — un champ de formulaire : une valeur qu'on lit et
//! qu'on écrit, un texte indicatif quand elle est vide, une taille.
//!
//! ```ignore
//! use overlay_ui::design::{self, InputSize};
//!
//! // La valeur vit chez l'appelant : le composant l'écrit, l'appelant la relit.
//! ui.add(design::input(&mut state.chemin).placeholder("Chemin vers wakfu.log"));
//!
//! if ui.add(design::input(&mut state.recherche).width(240.0)).changed() {
//!     // `Response::changed()` — la frame où la valeur vient d'être modifiée.
//! }
//! ```
//!
//! **Vide = le texte indicatif s'affiche.** Il n'y a pas de troisième état : une valeur est une
//! `String`, et une `String` vide *est* l'absence de valeur pour un champ texte. Envelopper la
//! valeur dans une `Option` n'ajouterait aucune information et obligerait chaque appelant à
//! choisir entre `None` et `Some("")` sans savoir lequel veut dire quoi.
//!
//! ## Ce qui a été mesuré
//!
//! Sur la barre de recherche de l'onglet Commandes
//! (`assets/design-system/interfaces/interface-options-commandes.png`, champ en x 30..500,
//! y 138..163), recoupée avec les assets isolés via
//! `.claude/skills/design-asset/scripts/dsimg.py analyze` :
//!
//! | Grandeur | Valeur | Source |
//! | --- | --- | --- |
//! | Hauteur native | **25 px** | capture (y 138..163) ET `large-input-text-width-placeholder.png` (composant 258 × 25) — deux sources indépendantes qui tombent d'accord |
//! | Bord | **2 px `#595140`** | capture (y 138-139 et 161-162, x 30-31 et 498-499) ; même couleur sur les quatre assets de champ |
//! | Rayon | **4** | `dsimg.py analyze`, ajustement parfait (IoU 1,0) sur deux assets |
//! | Fond | **`#0e1115`** | capture — bien plus sombre que le panneau qui le porte |
//! | Retrait du texte | **6 px** depuis le bord extérieur | premier glyphe de « Rechercher » à x=36 pour un champ à x=30 |
//! | Corps | **17 px** (encre 13) | même encre que les libellés de bouton, donc même corps |
//!
//! **Une valeur saisie est OR (`#f4d89e`), pas blanche.** C'est le constat le plus contre-intuitif
//! du relevé, et il tient sur trois assets indépendants : `input-search.png` (« aa »),
//! `input-number.png` et `large-input-number.png` donnent tous le même pic `#f4d89e`. Le texte
//! indicatif, lui, est un kaki éteint (`#83775b`) — la même famille chromatique que le bord, en
//! plus sourd. Un champ dont la valeur serait blanche ne ressemblerait pas au jeu.
//!
//! ## Le champ fait 25 px là où un bouton en fait 36
//!
//! Ce n'est pas une incohérence à corriger : le jeu compose réellement des lignes où un champ est
//! plus bas que le bouton d'à côté. La règle du design system s'applique telle quelle — **la
//! hauteur d'un composant est celle de sa référence**, et c'est à la mise en page de centrer le
//! plus petit sur la ligne.
//!
//! ## Ce qui n'a PAS de référence, et est donc inventé
//!
//! Deux états, signalés comme tels plutôt que présentés comme mesurés :
//!
//! - **Survolé** : aucune capture d'un champ survolé. Le composant ne change donc **rien** au
//!   survol — inventer un éclaircissement serait inventer du design. Seul le curseur de la souris
//!   change (`CursorIcon::Text`), ce qui est un comportement de plateforme, pas une décision
//!   esthétique.
//! - **Désactivé** : aucune capture non plus. Le bord et le texte passent à `TEXT_DISABLED`, par
//!   cohérence avec le bouton désactivé. À remplacer par une mesure dès qu'une capture existe.

use egui::{Align2, Response, Sense, Ui, Vec2, Widget};

use crate::design::{text, tokens};

/// Gabarit de hauteur. `Standard` est la **hauteur native** relevée dans le jeu, pas un palier
/// inventé — voir la doc de module.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputSize {
    /// 25 px — la hauteur de tous les champs de saisie relevés.
    Standard,
    /// Hauteur libre. Le corps de police et le retrait du texte suivent
    /// (`tokens::INPUT_FONT_SIZE_RATIO`), donc un champ de hauteur arbitraire reste proportionné.
    Height(f32),
}

impl InputSize {
    /// `const` pour qu'un panneau puisse en dériver une constante de mise en page — voir
    /// `panels::options_modal::FIELD_HEIGHT`.
    pub const fn height(self) -> f32 {
        match self {
            InputSize::Standard => 25.0,
            InputSize::Height(h) => h,
        }
    }
}

/// État visuel d'un champ — mêmes trois états que le bouton (voir `components`), à ceci près que
/// `Hovered` est visuellement identique à `Idle` faute de référence (doc de module).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputState {
    Idle,
    Hovered,
    Disabled,
}

/// Construit un champ de saisie sur `text`. Point d'entrée unique — voir la doc de module.
pub fn input(text: &mut String) -> Input<'_> {
    Input::new(text)
}

pub struct Input<'a> {
    text: &'a mut String,
    placeholder: Option<String>,
    size: InputSize,
    width: Option<f32>,
    enabled: bool,
    tooltip: Option<String>,
    log_name: Option<String>,
    forced_state: Option<InputState>,
    request_focus: bool,
    leading_icon: Option<crate::design::DsTexture>,
}

impl<'a> Input<'a> {
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: None,
            size: InputSize::Standard,
            width: None,
            enabled: true,
            tooltip: None,
            log_name: None,
            forced_state: None,
            request_focus: false,
            leading_icon: None,
        }
    }

    /// Pose une icône **à l'intérieur** du champ, collée au bord gauche — la loupe d'une barre de
    /// recherche, telle que le jeu la place partout (`interface-hdv-achat.png` x 27..39,
    /// `interface-personnage-equiement.png` x 768..780, `interface-options-commandes.png`
    /// x 36..48). Jamais un bouton icône posé à côté du champ : le jeu ne fait pas ça.
    ///
    /// **Le composant, et lui seul, réserve la gouttière** (`tokens::INPUT_LEADING_ICON_*`) en
    /// avançant le bord gauche du texte — donc pour le texte indicatif ET pour la valeur saisie, et
    /// l'icône est peinte dans le `clip_rect` du champ. C'est la différence avec le pis-aller qui a
    /// précédé ce paramètre : des espaces de tête dans le texte indicatif, qui ne décalaient rien
    /// d'autre et laissaient une valeur saisie démarrer sous l'icône.
    ///
    /// L'icône prend `INPUT_PLACEHOLDER` au repos et `TEXT_DISABLED` désactivée. Elle ne change pas
    /// avec la présence d'une valeur : mesuré sur `empty-input-search.png` et `input-search.png`,
    /// dont le profil de la colonne de la loupe est rigoureusement identique.
    pub fn leading_icon(mut self, icon: crate::design::DsTexture) -> Self {
        self.leading_icon = Some(icon);
        self
    }

    /// Texte affiché tant que la valeur est vide.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    /// Largeur imposée. **Sans elle, le champ prend toute la largeur disponible** — c'est l'inverse
    /// du bouton, qui se cale sur son libellé, et c'est voulu : un champ de saisie n'a pas de
    /// contenu au moment où on le place, sa largeur ne peut venir que de la mise en page.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Nom d'instance pour la journalisation (défaut : le texte indicatif, puis `"input"`). À
    /// renseigner dès qu'un formulaire porte plusieurs champs, sans quoi les lignes de
    /// `overlay-ui.<date>.log` sont indiscernables.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Demande le focus clavier POUR CETTE FRAME. À n'appeler qu'une fois — sinon le champ
    /// reprendrait le focus à chaque frame et l'utilisateur ne pourrait plus le quitter.
    ///
    /// C'est bien à l'appelant de décider *quand* (voir `panels::options_modal`, qui le fait à la
    /// première frame de la modale) : le composant n'a aucun moyen de savoir si le formulaire vient
    /// de s'ouvrir ou s'il est affiché depuis dix minutes.
    pub fn request_focus(mut self, request: bool) -> Self {
        self.request_focus = request;
        self
    }

    /// Force l'état peint, **sans passer par l'interaction** — réservé à la galerie de contrôle et
    /// aux captures de non-régression, où aucun pointeur ne survole quoi que ce soit. Même rôle que
    /// `Button::preview_state`.
    pub fn preview_state(mut self, state: InputState) -> Self {
        self.forced_state = Some(state);
        self
    }

    /// Taille que le champ occupera, sans le dessiner. `None` en largeur si elle n'est pas imposée :
    /// elle dépend alors de la place disponible, que seul `ui` connaît au moment du rendu.
    pub fn desired_size(&self) -> (Option<f32>, f32) {
        (self.width, self.size.height())
    }
}

impl Widget for Input<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let height = self.size.height();
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let font = text::label_font(ui.ctx(), height * tokens::INPUT_FONT_SIZE_RATIO);
        let pad_x = height * tokens::INPUT_PADDING_X_RATIO;

        let (rect, frame_response) =
            ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());

        let state = self.forced_state.unwrap_or({
            if !self.enabled {
                InputState::Disabled
            } else if frame_response.hovered() {
                InputState::Hovered
            } else {
                InputState::Idle
            }
        });
        let (border, value_color) = match state {
            // `Hovered` est délibérément identique à `Idle` — voir la doc de module.
            InputState::Idle | InputState::Hovered => (tokens::INPUT_BORDER, tokens::INPUT_TEXT),
            InputState::Disabled => (tokens::TEXT_DISABLED, tokens::TEXT_DISABLED),
        };

        if ui.is_rect_visible(rect) {
            ui.painter()
                .rect_filled(rect, tokens::INPUT_RADIUS, tokens::INPUT_FILL);
            ui.painter().rect_stroke(
                rect,
                tokens::INPUT_RADIUS,
                egui::Stroke::new(tokens::INPUT_BORDER_WIDTH, border),
                egui::StrokeKind::Inside,
            );
        }

        // Ornement de gauche — peint AVANT le texte, et surtout : la place qu'il occupe est
        // retirée de celle du texte juste après. Voir `Input::leading_icon`.
        let leading = self.leading_icon.map(|icon| {
            let side = height * tokens::INPUT_LEADING_ICON_RATIO;
            let inset = height * tokens::INPUT_LEADING_ICON_INSET_RATIO;
            let gap = height * tokens::INPUT_LEADING_ICON_GAP_RATIO;
            // `glyph_fit` et non `Vec2::splat` : le paramètre accepte n'importe quel
            // `DsTexture`, et un carré déformerait tout glyphe qui n'en est pas un. La règle est
            // celle du design system, partagée avec le bouton icône — il ne doit y en avoir
            // qu'une.
            let native = crate::design::DesignSystem::get(ui.ctx()).native_size(icon);
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.left() + inset + side / 2.0, rect.center().y),
                super::icon_button::glyph_fit(native, side),
            );
            if ui.is_rect_visible(rect) {
                let tint = match state {
                    InputState::Disabled => tokens::TEXT_DISABLED,
                    _ => tokens::INPUT_PLACEHOLDER,
                };
                crate::design::DesignSystem::get(ui.ctx()).paint(
                    &ui.painter().with_clip_rect(rect.intersect(ui.clip_rect())),
                    icon_rect,
                    icon,
                    tint,
                );
            }
            // Ce que le texte perd à gauche : le retrait, l'icône, la gouttière — moins le
            // rembourrage que le champ lui donnait déjà.
            (inset + side + gap - pad_x).max(0.0)
        });
        let leading_room = leading.unwrap_or(0.0);

        // Une seule ligne de texte, centrée verticalement dans le champ. Le rectangle est calculé
        // ici plutôt que laissé à egui : `TextEdit` prend la hauteur d'une ligne et se pose en haut
        // de l'espace qu'on lui donne, ce qui collerait le texte au bord supérieur.
        let row_height = ui.fonts_mut(|f| f.row_height(&font));
        let text_rect = egui::Rect::from_center_size(
            egui::pos2(rect.center().x + leading_room / 2.0, rect.center().y),
            Vec2::new((width - 2.0 * pad_x - leading_room).max(0.0), row_height),
        );

        let empty = self.text.is_empty();
        let enabled = self.enabled;
        let edit = egui::TextEdit::singleline(self.text)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .font(font.clone())
            .text_color(value_color)
            .desired_width(text_rect.width());
        let edit_response = ui
            .scope_builder(egui::UiBuilder::new().max_rect(text_rect), |ui| {
                ui.add_enabled(enabled, edit)
            })
            .inner;
        if self.request_focus {
            edit_response.request_focus();
        }

        // Le texte indicatif est peint À LA MAIN plutôt que confié à `TextEdit::hint_text` :
        // egui écrase la couleur d'un `hint_text` par `Visuals::weak_text_color()`
        // (`text_edit/builder.rs`, « hint_text.map_texts(…) »), il est donc impossible de lui
        // imposer le kaki éteint du jeu par cette voie.
        if empty {
            if let Some(placeholder) = &self.placeholder {
                ui.painter()
                    .with_clip_rect(text_rect.intersect(ui.clip_rect()))
                    .text(
                        text_rect.left_center(),
                        Align2::LEFT_CENTER,
                        placeholder,
                        font.clone(),
                        tokens::INPUT_PLACEHOLDER,
                    );
            }
        }

        let name = self
            .log_name
            .as_deref()
            .or(self.placeholder.as_deref())
            .unwrap_or("input");

        // Champ trop étroit pour son propre texte indicatif : c'est un défaut de mise en page (la
        // valeur saisie, elle, défile normalement, ce n'est pas un défaut). Signalé UNE fois par
        // instance, comme le débordement d'un libellé de bouton.
        if let Some(placeholder) = &self.placeholder {
            let needed = ui
                .painter()
                .layout_no_wrap(placeholder.clone(), font, egui::Color32::PLACEHOLDER)
                .size()
                .x;
            if needed > text_rect.width() + 0.5 {
                let warned_id = frame_response.id.with("ds-input-overflow");
                let already = ui.data_mut(|d| {
                    let seen = d.get_temp::<bool>(warned_id).unwrap_or(false);
                    d.insert_temp(warned_id, true);
                    seen
                });
                if !already {
                    tracing::warn!(
                        component = "input",
                        name,
                        largeur = text_rect.width(),
                        requise = needed,
                        "texte indicatif écrêté : le champ est plus étroit que son contenu"
                    );
                }
            }
        }

        if edit_response.changed() {
            tracing::debug!(component = "input", name, "valeur modifiée");
        }

        let response = frame_response.union(edit_response);
        let response = if enabled {
            response.on_hover_cursor(egui::CursorIcon::Text)
        } else {
            response
        };
        match self.tooltip {
            Some(tooltip) => response.on_hover_text(tooltip),
            None => response,
        }
    }
}
