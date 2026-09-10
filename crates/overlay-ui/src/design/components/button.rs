//! **Bouton texte** du design system Wakfu — le composant de référence : un libellé, une variante
//! d'intention, une taille, un état. Rien d'autre n'est à fournir, et surtout **aucune texture** :
//!
//! ```ignore
//! use overlay_ui::design::{self, ButtonSize, ButtonVariant};
//!
//! if ui.add(design::button("Valider").variant(ButtonVariant::Primary)).clicked() {
//!     // …
//! }
//! ui.add(
//!     design::button("Annuler")
//!         .variant(ButtonVariant::Danger)
//!         .size(ButtonSize::Compact)
//!         .width(338.0),
//! );
//! ```
//!
//! Ce que remplace ce composant : les boutons du pied de page de `panels::options_modal`, peints à
//! la main (dégradé `CANCEL_TOP`/`CANCEL_BOTTOM` + `chamfer`, un jeu de sept constantes par
//! bouton), et les assets taillés sur mesure `assets/design-system/large-button-cancel.png` /
//! `large-button-validate.png` — un PNG par taille ET par libellé. Ici : trois textures génériques
//! (plus leurs variantes survolées), peintes en 9-slice à la taille demandée, libellé rendu par
//! egui.
//!
//! **La variante porte aussi la graisse du libellé** — voir `ButtonVariant::label_strong` : le jeu
//! écrit ses boutons de pied de page plus gras que ceux posés dans un contenu, à hauteur d'encre
//! identique. Rien à fournir de plus à l'appel.
//!
//! **Variante ≠ couleur, variante = intention.** `Primary` (or) pour l'action qui valide, `Danger`
//! (rouge) pour celle qui annule/détruit dans un pied de page de modale, `Secondary` (gris-brun)
//! pour tout le reste. C'est la nuance que §5.2 du design-system signale explicitement : le rouge
//! est réservé au pattern « bouton pleine largeur du pied de page », un « Annuler » de simple boîte
//! de dialogue reste kaki.

use egui::{Color32, Response, Sense, Ui, Vec2, Widget};

use crate::design::{assets::DsTexture, text, tokens, DesignSystem};

/// Intention d'un bouton — détermine sa paire de textures et sa couleur de libellé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Or : action qui valide/paye (« Valider », « Mettre en vente »).
    Primary,
    /// Gris-brun : action neutre, le cas par défaut.
    Secondary,
    /// Rouge : annulation/destruction en pied de page de modale (§5.2 du design-system).
    Danger,
}

impl ButtonVariant {
    /// Textures disponibles pour cette variante : `(hauteur native, repos, survol)`.
    ///
    /// Une variante peut en avoir plusieurs parce que le jeu a capturé le même bouton à deux
    /// hauteurs — et que **l'embout décoratif n'y a pas la même largeur** (52px sur les textures
    /// de 52px de haut, 34px sur celles de 36px). Ce n'est donc pas une redondance qu'un
    /// étirement pourrait absorber : rendre un bouton de pied de page avec la texture de fenêtre
    /// lui donne un embout une fois et demie trop large, visible dès qu'un bouton d'une autre
    /// variante est posé à côté.
    fn textures(self) -> &'static [(f32, DsTexture, DsTexture)] {
        match self {
            ButtonVariant::Primary => &[
                (
                    52.0,
                    DsTexture::ButtonPrimary,
                    DsTexture::ButtonPrimaryHover,
                ),
                (
                    36.0,
                    DsTexture::ButtonPrimaryCompact,
                    DsTexture::ButtonPrimaryCompactHover,
                ),
            ],
            ButtonVariant::Secondary => &[(
                52.0,
                DsTexture::ButtonSecondary,
                DsTexture::ButtonSecondaryHover,
            )],
            ButtonVariant::Danger => {
                &[(36.0, DsTexture::ButtonDanger, DsTexture::ButtonDangerHover)]
            }
        }
    }

    /// Texture retenue pour un bouton rendu à `height` : celle dont la **hauteur native est la plus
    /// proche**. C'est à la fois celle qui subira le moins d'étirement vertical du dégradé et celle
    /// dont l'embout a la bonne largeur pour ce gabarit. Une seule candidate ⇒ elle est prise quelle
    /// que soit la hauteur, l'étirement restant préférable à un embout inventé.
    fn texture(self, height: f32, hovered: bool) -> DsTexture {
        let (_, idle, over) = self
            .textures()
            .iter()
            .min_by(|a, b| (a.0 - height).abs().total_cmp(&(b.0 - height).abs()))
            .expect("chaque variante déclare au moins une texture");
        if hovered {
            *over
        } else {
            *idle
        }
    }

    /// La variante porte-t-elle un libellé **appuyé** (`design::fonts::LABEL_STRONG`) ?
    ///
    /// Le jeu écrit ses boutons dans deux graisses, mesurées sur l'onglet Interface de la modale
    /// Options — voir `design::fonts` : ses deux boutons de pied de page sont plus gras que les
    /// quatre boutons posés dans le contenu, à hauteur d'encre identique.
    ///
    /// La graisse suit donc la **variante**, sans paramètre supplémentaire : `Primary` et `Danger`
    /// sont précisément les deux intentions du pied de page de modale (voir `ButtonVariant`, et §5.2
    /// du design-system qui réserve le rouge à ce seul pattern), `Secondary` est « tout le reste »,
    /// c'est-à-dire le contenu.
    ///
    /// **Corrélation observée, pas loi générale** : la seule capture qui montre les deux familles
    /// côte à côte est celle-là. Si un `Primary` apparaît un jour DANS un panneau de contenu et
    /// qu'il s'y révèle maigre, c'est ici qu'il faudra couper — probablement en ajoutant un réglage
    /// explicite plutôt qu'en changeant la règle sous les appelants existants.
    fn label_strong(self) -> bool {
        match self {
            ButtonVariant::Primary | ButtonVariant::Danger => true,
            ButtonVariant::Secondary => false,
        }
    }

    fn text_color(self) -> Color32 {
        match self {
            ButtonVariant::Primary => tokens::BUTTON_TEXT_ON_GOLD,
            ButtonVariant::Secondary => tokens::BUTTON_TEXT_ON_KHAKI,
            ButtonVariant::Danger => tokens::BUTTON_TEXT_ON_DANGER,
        }
    }
}

/// Gabarit de hauteur. Les deux valeurs nommées sont les **hauteurs natives** des textures du jeu,
/// pas des paliers inventés : `button-primary.png`/`button-secondary.png` font 52px de haut (fenêtre
/// HDV), `button-danger.png` 36px (pied de page de la modale Options). Un bouton rendu à sa hauteur
/// native ne subit aucun étirement vertical du dégradé.
///
/// La largeur, elle, n'est jamais un gabarit : elle vient du libellé (voir `Button::width`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonSize {
    /// 52px — hauteur native des boutons de fenêtre (HDV).
    Standard,
    /// 36px — hauteur native des boutons de pied de page de modale.
    Compact,
    /// Hauteur libre. Le corps de police et les marges suivent (voir `tokens::
    /// BUTTON_FONT_SIZE_RATIO`), donc un bouton de hauteur arbitraire reste proportionné.
    Height(f32),
}

impl ButtonSize {
    pub fn height(self) -> f32 {
        match self {
            ButtonSize::Standard => 52.0,
            ButtonSize::Compact => 36.0,
            ButtonSize::Height(h) => h,
        }
    }
}

/// État visuel d'un bouton.
///
/// **Il n'y a délibérément pas d'état « pressé » distinct** (décision utilisateur 2026-09-09) :
/// dans le jeu, appuyer fait *disparaître* l'apparence survolée — le bouton retombe au repos tant
/// que le bouton de souris est enfoncé, et repasse en survol au relâchement. C'est exactement la
/// règle déjà appliquée aux boutons icône (correctif 2026-09-08, alors dans le prédécesseur maison
/// de `design::icon_button`), reprise ici pour que les deux familles se comportent à l'identique.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonState {
    Idle,
    Hovered,
    Disabled,
}

/// Atténuation appliquée à la texture désactivée. Même principe que
/// [`crate::design::tokens::DISABLED_DIM`], en plus léger : un multiplicateur d'alpha, pour que
/// l'état désactivé se lise aussi sur une capture statique, en plus de la texture grise et du
/// libellé gris.
const DISABLED_TINT: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 190);

/// Construit un bouton du design system. Point d'entrée unique — voir la doc de module.
pub fn button(text: impl Into<String>) -> Button {
    Button::new(text)
}

pub struct Button {
    text: String,
    variant: ButtonVariant,
    size: ButtonSize,
    width: Option<f32>,
    min_width: Option<f32>,
    enabled: bool,
    tooltip: Option<String>,
    log_name: Option<String>,
    forced_state: Option<ButtonState>,
}

impl Button {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: ButtonVariant::Secondary,
            size: ButtonSize::Standard,
            width: None,
            min_width: None,
            enabled: true,
            tooltip: None,
            log_name: None,
            forced_state: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Largeur imposée. Sans elle, la largeur vaut `libellé + 2 × marge`, avec un plancher de
    /// proportion (`tokens::BUTTON_MIN_ASPECT`) pour qu'un libellé très court ne donne pas un carré.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Plancher de largeur — utile pour aligner une rangée de boutons de libellés inégaux sans
    /// figer leur largeur.
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = Some(min_width);
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

    /// Nom d'instance pour la journalisation (défaut : le libellé). À renseigner quand plusieurs
    /// boutons partagent un libellé (« Valider » dans deux modales) : c'est ce nom qui distingue
    /// les lignes dans `overlay-ui.<date>.log`.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Force l'état peint, **sans passer par l'interaction**. Réservé aux planches de contrôle et
    /// aux captures de non-régression (`overlay-testkit`), où aucun pointeur ne survole quoi que ce
    /// soit : c'est le seul moyen de montrer les trois états côte à côte sur une même image. En
    /// usage normal, ne pas l'appeler — l'état vient de la `Response`.
    pub fn preview_state(mut self, state: ButtonState) -> Self {
        self.forced_state = Some(state);
        self
    }

    /// Taille que le bouton occupera, sans le dessiner — pour une mise en page qui doit réserver la
    /// place avant (pied de page à deux boutons alignés, par exemple).
    pub fn desired_size(&self, ui: &Ui) -> Vec2 {
        let height = self.size.height();
        let text_width = self.layout(ui, height).size().x;
        Vec2::new(self.resolve_width(height, text_width), height)
    }

    fn font_size(height: f32) -> f32 {
        height * tokens::BUTTON_FONT_SIZE_RATIO
    }

    /// Mise en page du libellé, dans la police des libellés du design system (voir `design::fonts`)
    /// et non dans la proportionnelle par défaut d'egui — à la graisse de la variante, voir
    /// `ButtonVariant::label_strong`. La couleur reste `Color32::PLACEHOLDER` : `layout` sert aussi
    /// à `desired_size`, où l'état du bouton — donc la couleur du texte — n'est pas encore connu ;
    /// `Painter::galley` la résout à la peinture.
    fn layout(&self, ui: &Ui, height: f32) -> std::sync::Arc<egui::Galley> {
        let size = Self::font_size(height);
        let font = if self.variant.label_strong() {
            text::label_strong_font(ui.ctx(), size)
        } else {
            text::label_font(ui.ctx(), size)
        };
        ui.painter()
            .layout_no_wrap(self.text.clone(), font, Color32::PLACEHOLDER)
    }

    fn resolve_width(&self, height: f32, text_width: f32) -> f32 {
        let natural = text_width + 2.0 * height * tokens::BUTTON_PADDING_X_RATIO;
        let floor = height * tokens::BUTTON_MIN_ASPECT;
        self.width
            .unwrap_or_else(|| natural.max(floor))
            .max(self.min_width.unwrap_or(0.0))
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let ds = DesignSystem::get(ui.ctx());
        let height = self.size.height();

        let state_for_color = self.forced_state.unwrap_or(if self.enabled {
            ButtonState::Idle
        } else {
            ButtonState::Disabled
        });
        let text_color = if state_for_color == ButtonState::Disabled {
            tokens::TEXT_DISABLED
        } else {
            self.variant.text_color()
        };
        let galley = self.layout(ui, height);
        let width = self.resolve_width(height, galley.size().x);

        // Désactivé, le bouton n'est PAS inerte : il garde `Sense::hover()` pour que son infobulle
        // — souvent la seule explication de pourquoi il est grisé — reste consultable. Retirer le
        // `Sense::click()` plutôt que de filtrer `clicked()` après coup garantit qu'un appelant qui
        // écrit `if ui.add(...).clicked()` sans se soucier de l'état ne peut pas déclencher
        // l'action.
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), sense);
        let response = response.on_hover_cursor(if self.enabled {
            egui::CursorIcon::PointingHand
        } else {
            egui::CursorIcon::Default
        });

        let state = self.forced_state.unwrap_or_else(|| {
            if !self.enabled {
                ButtonState::Disabled
            } else if response.hovered() && !ui.input(|i| i.pointer.any_down()) {
                // Voir la doc de `ButtonState` : l'appui retire l'apparence survolée.
                ButtonState::Hovered
            } else {
                ButtonState::Idle
            }
        });

        if ui.is_rect_visible(rect) {
            let (texture, tint) = match state {
                ButtonState::Idle => (self.variant.texture(height, false), Color32::WHITE),
                ButtonState::Hovered => (self.variant.texture(height, true), Color32::WHITE),
                ButtonState::Disabled => (DsTexture::ButtonDisabled, DISABLED_TINT),
            };
            ds.paint(ui.painter(), rect, texture, tint);

            // Le libellé est écrêté au bouton : un texte trop long déborderait sinon sur le
            // panneau voisin, et le défaut passerait pour un bug de mise en page.
            let text_pos = rect.center() - galley.size() * 0.5;
            ui.painter()
                .with_clip_rect(rect.intersect(ui.clip_rect()))
                .galley(text_pos, galley.clone(), text_color);
        }

        let name = self.log_name.as_deref().unwrap_or(self.text.as_str());

        // Débordement : signalé UNE fois par instance (mémorisé sur l'id du widget), pas à chaque
        // frame — c'est un défaut de mise en page, pas un événement.
        let needed = galley.size().x + 2.0 * height * tokens::BUTTON_PADDING_X_RATIO;
        if needed > width + 0.5 {
            let warned_id = response.id.with("ds-button-overflow");
            let already = ui.data_mut(|d| {
                let seen = d.get_temp::<bool>(warned_id).unwrap_or(false);
                d.insert_temp(warned_id, true);
                seen
            });
            if !already {
                tracing::warn!(
                    component = "button",
                    name,
                    largeur = width,
                    requise = needed,
                    "libellé écrêté : le bouton est plus étroit que son contenu"
                );
            }
        }

        if response.clicked() {
            tracing::debug!(component = "button", name, variant = ?self.variant, "clic");
        }

        match self.tooltip {
            Some(tooltip) => response.on_hover_text(tooltip),
            None => response,
        }
    }
}
