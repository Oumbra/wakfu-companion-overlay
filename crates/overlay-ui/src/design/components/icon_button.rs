//! **Bouton icône** du design system Wakfu — un socle carré et un glyphe centré dessus.
//!
//! ```ignore
//! use overlay_ui::design::{self, DsTexture, IconContext};
//!
//! if ui
//!     .add(design::icon_button(DsTexture::IconOption).context(IconContext::FirstPlan))
//!     .clicked()
//! {
//!     // l'appelant décide de l'action — jamais le composant
//! }
//! ```
//!
//! ## Ce que ce composant reprend, et ce qu'il corrige
//!
//! Une implémentation existait déjà — `panels::icon_button::paint_icon_button` — et elle
//! fonctionne. Elle ne respecte simplement pas le contrat de composant : elle prend **quatre
//! `egui::TextureHandle` en paramètres**, ce qui oblige chaque appelant à connaître et à câbler les
//! textures. C'est exactement le symptôme qui a motivé la création du design system.
//!
//! Ici, l'appelant nomme une **intention** (`IconContext::FirstPlan`, `DsTexture::IconPlus`) et le
//! manifeste résout les fichiers.
//!
//! Ce qui est repris tel quel, parce que c'est mesuré ou déjà arbitré :
//!
//! - **Le socle et l'icône partagent le même facteur d'échelle**, dérivé de la largeur du socle. Une
//!   icône reste donc proportionnée à son bouton quelle que soit la taille demandée.
//! - **Un appui de souris retire l'apparence survolée**, elle revient au relâchement — la règle
//!   commune à toute l'interface (`response.hovered() && !pointer.any_down()`), corrigée une fois en
//!   2026-09-08 parce que deux familles de boutons s'en écartaient.
//! - **Les deux teintes d'icône** : `#c5cbcc` au repos, `#f4d89f` au survol, mesurées sur
//!   `menu-button-icon-first-plan.png`. Les icônes du design system étant blanc pur avec alpha, une
//!   simple teinte les reproduit — là où `ui_icons` en charge deux copies recolorées.
//!
//! ## Deux contextes de socle
//!
//! | Contexte | Socle | Où |
//! | --- | --- | --- |
//! | `FirstPlan` | `button-icon-first-plan.png` | barre de premier plan, par-dessus le jeu |
//! | `Panel` | `button-icon.png` | à l'intérieur d'un panneau |
//!
//! Les deux font **36 × 36**, la taille native. `button-icon-disabled.png` est partagée par les deux
//! contextes — le jeu n'a qu'une capture de socle grisé, comme pour le bouton texte.
//!
//! ## Pas encore utilisé en production, et où en est la migration
//!
//! Les quatre boutons icône de l'overlay — « + », « − », Détails et Options, **tous les quatre dans
//! le carré de contrôle du Suivi** depuis la refonte 2026-09-08 (`watchlist::control_button_row`) —
//! **continuent de passer par `panels::icon_button`**. Leur socle est pourtant déjà celui du design
//! system : `crates/overlay-ui/assets/ui/button-background.png` est octet pour octet
//! `assets/design-system/button-icon-first-plan.png`, et de même pour la variante survolée.
//!
//! Ce sont leurs **icônes** qui diffèrent : celles d'`ui_icons` sont recadrées et normalisées à une
//! taille de référence (`ui_icons::normalize_icon_content`), les nôtres sont les glyphes détourés
//! bruts.
//!
//! Cette doc a longtemps dit qu'« aucune capture de référence ne permet d'arbitrer laquelle des
//! deux normalisations est la bonne ». **C'est faux depuis le 2026-09-10** :
//! `assets/design-system/menu-button-icon-first-plan.png` — déjà la source des deux teintes
//! d'icône ci-dessus — porte huit icônes de la même famille, et personne n'y avait mesuré la taille
//! d'encre. Mesuré (seuil de luminance 140, résultat invariant de 120 à 180) : **bbox de 16 à 20 px
//! pour un socle de 36**, médiane 18, toutes centrées au pixel près sur l'axe du socle ; le socle de
//! cette capture est bien à l'échelle 1 (`button-icon-first-plan.png` s'y recale à 36 × 36 avec un
//! écart moyen de 2,3/255, contre 4,5 et plus dès 35 ou 37). Autrement dit **le principe de
//! normalisation d'`ui_icons` est le bon, et son étalon de 18 — celui du rouage — est la bonne
//! valeur pour toutes les icônes**, pas seulement pour le rouage.
//!
//! Les candidats sont peints côte à côte par `overlay-testkit/tests/icon_button_arbitrage.rs`
//! (`icon_button_arbitrage.png`), et [`IconButton::icon_content`] porte la normalisation le temps
//! de l'arbitrage. La migration elle-même attend un choix de l'utilisateur : elle change le rendu
//! d'un panneau visible en permanence et fait bouger neuf captures de non-régression.
//!
//! Le cas que le plan appelle « réinitialisation » — le bouton 36 × 36 que le jeu pose à droite de
//! la barre d'onglets et à droite d'un réglage isolé — attend qu'un réglage réinitialisable existe.
//! Note du relevé à ne pas « corriger » le jour venu : dans la barre d'onglets, **son axe est 9 px
//! plus bas que celui des onglets**.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::design::{assets::DsTexture, tokens, DesignSystem};

/// Sur quoi le bouton est posé — c'est ce qui choisit son socle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IconContext {
    /// Par-dessus le jeu, dans une barre de premier plan.
    #[default]
    FirstPlan,
    /// À l'intérieur d'un panneau.
    Panel,
}

impl IconContext {
    fn background(self, hovered: bool) -> DsTexture {
        match (self, hovered) {
            (IconContext::FirstPlan, false) => DsTexture::ButtonIconFirstPlan,
            (IconContext::FirstPlan, true) => DsTexture::ButtonIconFirstPlanHover,
            (IconContext::Panel, false) => DsTexture::ButtonIcon,
            (IconContext::Panel, true) => DsTexture::ButtonIconHover,
        }
    }
}

/// État visuel — les trois du contrat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconButtonState {
    Idle,
    Hovered,
    Disabled,
}

/// Construit un bouton icône. `icon` est une texture du manifeste, jamais un fichier.
pub fn icon_button(icon: DsTexture) -> IconButton {
    IconButton::new(icon)
}

pub struct IconButton {
    icon: DsTexture,
    context: IconContext,
    size: f32,
    enabled: bool,
    tooltip: Option<String>,
    log_name: Option<String>,
    forced_state: Option<IconButtonState>,
    icon_content: Option<f32>,
}

impl IconButton {
    pub fn new(icon: DsTexture) -> Self {
        Self {
            icon,
            context: IconContext::default(),
            size: tokens::ICON_BUTTON_SIZE,
            enabled: true,
            tooltip: None,
            log_name: None,
            forced_state: None,
            icon_content: None,
        }
    }

    pub fn context(mut self, context: IconContext) -> Self {
        self.context = context;
        self
    }

    /// Côté du bouton. **36 px par défaut, la taille native des cinq socles** — toute autre valeur
    /// met le socle ET l'icône à l'échelle dans le même rapport.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// **Banc d'arbitrage uniquement (2026-09-10), pas encore un choix figé.** Ramène la plus
    /// grande dimension de l'icône à `px`, exprimé dans le repère du socle natif (36) — le rapport
    /// est ensuite mis à l'échelle avec le bouton, comme la taille native l'est déjà.
    ///
    /// Sans ce réglage, une icône est peinte à sa taille de fichier : les glyphes du design system
    /// étant détourés au pixel près, deux glyphes voisins n'ont alors pas la même hauteur d'encre
    /// (13 pour le lien externe, 16 pour le rouage) là où le jeu, lui, les cale sur une grille
    /// commune (mesuré : 16 à 20 px de bbox pour un socle de 36 sur les huit icônes de
    /// `menu-button-icon-first-plan.png`, médiane 18). C'est le seul écart de rendu entre ce
    /// composant et `panels::icon_button`, qui normalise la même chose au chargement
    /// (`ui_icons::normalize_icon_content`).
    ///
    /// Si cette normalisation est retenue, sa place définitive est le **manifeste** (une taille de
    /// contenu par texture d'icône), pas l'appelant : c'est une propriété de l'asset.
    pub fn icon_content(mut self, px: f32) -> Self {
        self.icon_content = Some(px);
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

    /// Nom d'instance pour la journalisation (défaut : `"icon-button"`). À renseigner dès qu'une
    /// barre en porte plusieurs, sans quoi les lignes du journal sont indiscernables.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Force l'état peint — galerie et captures uniquement.
    pub fn preview_state(mut self, state: IconButtonState) -> Self {
        self.forced_state = Some(state);
        self
    }
}

impl Widget for IconButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::splat(self.size),
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        // Même condition que `design::button` et `panels::icon_button` : un appui de souris retire
        // l'apparence survolée partout dans l'overlay.
        let pointer_down = ui.input(|i| i.pointer.any_down());
        let state = self.forced_state.unwrap_or({
            if !self.enabled {
                IconButtonState::Disabled
            } else if response.hovered() && !pointer_down {
                IconButtonState::Hovered
            } else {
                IconButtonState::Idle
            }
        });

        if ui.is_rect_visible(rect) {
            let design = DesignSystem::get(ui.ctx());
            let (background, icon_tint) = match state {
                IconButtonState::Idle => (self.context.background(false), tokens::ICON_TINT),
                IconButtonState::Hovered => {
                    (self.context.background(true), tokens::ICON_TINT_HOVER)
                }
                // Une seule texture de socle grisé pour les deux contextes : le jeu n'en a capturé
                // qu'une, comme pour le bouton texte.
                IconButtonState::Disabled => (DsTexture::ButtonIconDisabled, tokens::TEXT_DISABLED),
            };
            design.paint(ui.painter(), rect, background, egui::Color32::WHITE);

            // Le socle et l'icône partagent le MÊME facteur d'échelle, dérivé de la largeur du
            // socle : l'icône reste proportionnée à son bouton à n'importe quelle taille.
            let scale = rect.width() / tokens::ICON_BUTTON_SIZE;
            let native = design.native_size(self.icon);
            let icon_size = match self.icon_content {
                // Rapport commun aux deux axes : une icône normalisée garde ses proportions.
                Some(target) => native * (target / native.x.max(native.y)) * scale,
                None => native * scale,
            };
            let icon_rect = egui::Rect::from_center_size(rect.center(), icon_size);
            design.paint(ui.painter(), icon_rect, self.icon, icon_tint);
        }

        if response.clicked() {
            tracing::debug!(
                component = "icon_button",
                name = self.log_name.as_deref().unwrap_or("icon-button"),
                "clic"
            );
        }

        let response = if self.enabled {
            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            response.on_hover_cursor(egui::CursorIcon::Default)
        };
        match self.tooltip {
            Some(tooltip) => response.on_hover_text(tooltip),
            None => response,
        }
    }
}
