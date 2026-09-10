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
//! ## La taille d'encre vient du manifeste
//!
//! Les glyphes de `assets/design-system/icons/` sont détourés au pixel près : leur fichier fait
//! exactement la taille de leur encre, et celle-ci varie d'un glyphe à l'autre (13 pour le lien
//! externe, 16 pour le rouage). Peints tels quels, ils donneraient trois hauteurs d'encre
//! différentes dans une même barre.
//!
//! Le jeu, lui, les cale sur une grille commune. Mesuré le 2026-09-10 sur
//! `menu-button-icon-first-plan.png` — déjà la source des deux teintes ci-dessus, mais personne n'y
//! avait mesuré la taille d'encre : **bbox de 16 à 20 px pour un socle de 36**, médiane 18, toutes
//! centrées au pixel près sur l'axe du socle (seuil de luminance 140, résultat invariant de 120 à
//! 180). La capture est bien à l'échelle 1 : `button-icon-first-plan.png` s'y recale à 36 × 36 avec
//! un écart moyen de 2,3/255 sur les pixels de socle, contre 4,5 et plus dès 35 ou 37.
//!
//! C'est [`tokens::ICON_BUTTON_CONTENT`], appliqué par `DsTexture::icon_content_size` — donc par
//! **le manifeste, jamais par l'appelant** : la taille d'encre est une propriété de l'asset.
//! `icon_draw_size` en bas de ce fichier fait le calcul, et ses tests l'éprouvent sans GPU.
//!
//! Cette doc a longtemps dit qu'« aucune capture de référence ne permet d'arbitrer laquelle des
//! deux normalisations est la bonne », et c'est pour cette raison que les quatre boutons icône de
//! l'overlay ne passaient pas encore par ce composant. La mesure ci-dessus a tranché : le principe
//! de normalisation d'`ui_icons` était le bon, et son étalon de 18 — réservé au seul rouage après un
//! retour « encore trop petite » — valait pour les quatre.
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

    /// Socle désactivé, sa teinte, et celle de l'icône — **pas la même mécanique selon le
    /// contexte**, et c'est mesuré, pas arbitraire.
    ///
    /// Le jeu n'a capturé qu'une texture de socle grisé, `button-icon-disabled.png`, et elle
    /// appartient au contexte `Panel` : sa luminance moyenne est de 60, contre 81 pour
    /// `button-icon.png`, le socle actif du même contexte. Un bouton désactivé y est donc plus
    /// sombre que ses voisins, ce qu'on attend.
    ///
    /// Posée sur une barre de premier plan, cette même texture s'inverse : 60 contre 41 pour
    /// `button-icon-first-plan.png`. Le bouton désactivé devient le plus lumineux du carré et
    /// attire l'œil avant les boutons actifs — constaté sur la capture de migration du carré de
    /// contrôle du Suivi (2026-09-10), qui est exactement ce cas.
    ///
    /// En `FirstPlan`, on garde donc le socle de repos et on l'assombrit, comme le faisait
    /// `panels::icon_button` faute d'asset dédié. Ce n'est pas un repli : c'est la seule des deux
    /// mécaniques qui dit « désactivé » sur ce fond-là.
    fn disabled(self) -> (DsTexture, egui::Color32, egui::Color32) {
        match self {
            IconContext::FirstPlan => (
                DsTexture::ButtonIconFirstPlan,
                tokens::DISABLED_DIM,
                tokens::ICON_TINT_DISABLED,
            ),
            IconContext::Panel => (
                DsTexture::ButtonIconDisabled,
                egui::Color32::WHITE,
                tokens::TEXT_DISABLED,
            ),
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
            let (background, background_tint, icon_tint) = match state {
                IconButtonState::Idle => (
                    self.context.background(false),
                    egui::Color32::WHITE,
                    tokens::ICON_TINT,
                ),
                IconButtonState::Hovered => (
                    self.context.background(true),
                    egui::Color32::WHITE,
                    tokens::ICON_TINT_HOVER,
                ),
                IconButtonState::Disabled => self.context.disabled(),
            };
            design.paint(ui.painter(), rect, background, background_tint);

            let icon_size = icon_draw_size(
                design.native_size(self.icon),
                self.icon.icon_content_size(),
                rect.width(),
            );
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

/// Taille à laquelle peindre l'icône sur un bouton de côté `button_size`.
///
/// Le socle et l'icône partagent le **même facteur d'échelle**, dérivé de la largeur du socle : une
/// icône reste proportionnée à son bouton à n'importe quelle taille.
///
/// `content` — la taille d'encre du manifeste (`DsTexture::icon_content_size`) — ramène en plus la
/// plus grande dimension du glyphe à l'étalon du jeu **avant** cette mise à l'échelle. Sans elle,
/// une icône est peinte à sa taille de fichier, qui varie d'un glyphe détouré à l'autre.
///
/// Fonction libre plutôt que corps de `Widget::ui` : c'est le seul calcul du composant qui peut se
/// tromper en silence, et il s'éprouve sans GPU (voir les tests en bas de ce fichier).
fn icon_draw_size(native: Vec2, content: Option<f32>, button_size: f32) -> Vec2 {
    let scale = button_size / tokens::ICON_BUTTON_SIZE;
    match content {
        // Rapport commun aux deux axes : une icône normalisée garde ses proportions.
        Some(target) => native * (target / native.x.max(native.y)) * scale,
        None => native * scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tolérance de comparaison — ces tailles finissent en coordonnées de peinture flottantes, pas
    /// en pixels entiers ; un centième suffit à attraper une erreur de formule.
    const EPS: f32 = 0.01;

    /// Les quatre glyphes du carré de contrôle du Suivi, à leur taille de fichier (détourés au
    /// pixel près par le skill `design-asset`, donc canevas = encre). Recopiés ici plutôt que lus
    /// par `DesignSystem` : ce test ne doit dépendre d'aucun contexte egui ni d'aucun GPU.
    const GLYPHES: [(&str, Vec2); 4] = [
        ("icon-plus", Vec2::new(14.0, 14.0)),
        ("icon-minus", Vec2::new(14.0, 2.0)),
        ("icon-external-link", Vec2::new(13.0, 13.0)),
        ("icon-option", Vec2::new(16.0, 15.0)),
    ];

    #[test]
    fn une_icone_normalisee_atteint_l_etalon_du_jeu() {
        for (nom, native) in GLYPHES {
            let peinte = icon_draw_size(native, Some(tokens::ICON_BUTTON_CONTENT), 24.0);
            let attendu = tokens::ICON_BUTTON_CONTENT * 24.0 / tokens::ICON_BUTTON_SIZE;
            assert!(
                (peinte.x.max(peinte.y) - attendu).abs() < EPS,
                "{nom} : plus grande dimension {} au lieu de {attendu}",
                peinte.x.max(peinte.y),
            );
        }
    }

    #[test]
    fn une_icone_normalisee_garde_ses_proportions() {
        for (nom, native) in GLYPHES {
            let peinte = icon_draw_size(native, Some(tokens::ICON_BUTTON_CONTENT), 24.0);
            assert!(
                (peinte.x / peinte.y - native.x / native.y).abs() < EPS,
                "{nom} : rapport d'aspect {} au lieu de {}",
                peinte.x / peinte.y,
                native.x / native.y,
            );
        }
    }

    /// Le « − » est le cas qui se serait cassé en silence : 2 px de haut à l'origine, il ne survit
    /// à la normalisation que si le facteur s'applique aux DEUX axes. Une normalisation qui
    /// n'agirait que sur la plus grande dimension en ferait une barre.
    #[test]
    fn le_trait_du_moins_reste_un_trait() {
        let peinte = icon_draw_size(Vec2::new(14.0, 2.0), Some(18.0), 24.0);
        assert!((peinte.x - 12.0).abs() < EPS, "largeur {}", peinte.x);
        assert!(
            (peinte.y - 12.0 * 2.0 / 14.0).abs() < EPS,
            "hauteur {}",
            peinte.y
        );
    }

    /// Sans étalon, on retombe exactement sur la taille de fichier mise à l'échelle du socle — le
    /// comportement de toutes les textures qui ne sont pas des icônes de bouton.
    #[test]
    fn sans_etalon_l_icone_garde_sa_taille_de_fichier() {
        for (nom, native) in GLYPHES {
            let peinte = icon_draw_size(native, None, 24.0);
            let attendu = native * (24.0 / tokens::ICON_BUTTON_SIZE);
            assert!(
                (peinte - attendu).length() < EPS,
                "{nom} : {peinte:?} au lieu de {attendu:?}",
            );
        }
    }

    /// À la taille native du socle, l'étalon est la taille peinte, sans conversion.
    #[test]
    fn au_socle_natif_l_etalon_est_la_taille_peinte() {
        let peinte = icon_draw_size(
            Vec2::new(13.0, 13.0),
            Some(tokens::ICON_BUTTON_CONTENT),
            tokens::ICON_BUTTON_SIZE,
        );
        assert!((peinte.x - tokens::ICON_BUTTON_CONTENT).abs() < EPS);
        assert!((peinte.y - tokens::ICON_BUTTON_CONTENT).abs() < EPS);
    }
}
