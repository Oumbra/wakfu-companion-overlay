//! **Bouton œil de la bascule interactif / clic-traversant** (2026-09-28, demande utilisateur) —
//! le pendant souris du raccourci `ShortcutAction::Toggle`, posé en haut à gauche de la fenêtre
//! de jeu, juste après le bouton Boutique (le cadeau) de la rangée de boutons du client.
//!
//! Demande, mot pour mot ou presque : « un bouton icône un peu après le bouton cadeau, en haut à
//! gauche de la fenêtre du jeu, avec un œil ouvert ; quand l'utilisateur clique dessus, ça rend le
//! reste de l'overlay clic-traversant mais pas ce bouton ; l'icône devient un œil barré, et quand
//! il reclique, ça revient — l'équivalent du raccourci qui bascule l'interactivité ».
//!
//! ## Une fenêtre OS à lui, toujours interactive
//!
//! Le clic-traversant se règle fenêtre OS par fenêtre OS (`Window::set_cursor_hittest`) : un
//! bouton peint DANS un autre overlay deviendrait traversant avec lui, et ne pourrait plus jamais
//! ramener l'overlay en mode interactif. Il vit donc dans sa propre fenêtre
//! (`OverlayKind::ClickThrough`, une par fenêtre de jeu, comme le Récap), que les deux hôtes
//! excluent de la bascule — la même exception que la fenêtre de connexion.
//!
//! Pour la même raison, il ne prend **pas** l'opacité réduite du mode clic-traversant
//! (`render_content::CLICK_THROUGH_OPACITY`) : c'est le seul élément encore cliquable, il doit se
//! voir. L'état se lit sur le glyphe — œil ouvert en interactif, œil barré en clic-traversant.
//!
//! ## Pas d'infobulle
//!
//! Une infobulle egui ne s'affiche qu'à l'intérieur de sa fenêtre OS. Lui en faire la place
//! agrandirait une fenêtre qui capte TOUJOURS les clics : une bande de ~250 × 40 px sous les
//! boutons du jeu où le clic n'atteindrait plus jamais le jeu, même en clic-traversant. La
//! fenêtre fait donc exactement la taille du bouton.

use crate::design::{self, DsIcon, IconContext};
use crate::panels::drag::PanelDrag;

/// Côté du bouton, en points logiques — la **hauteur des boutons Menu/Boutique du jeu** : relevée
/// sur la capture annotée de `recap_placement::DEFAULT_OFFSET`, ils occupent y = 32 à 71 dans la
/// zone cliente, soit 40 px. Le socle `FirstPlan` (36 px natifs) est mis à l'échelle, glyphe
/// compris (voir `design::icon_button`).
pub const BUTTON_SIZE: f32 = 40.0;

/// Nom du bouton dans le journal (`design::icon_button::log_name`).
const LOG_NAME: &str = "clic-traversant.bascule";

/// Ce que le bouton remonte à son hôte pour la frame en cours.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ClickThroughOutcome {
    /// Le bouton vient d'être cliqué : l'hôte bascule le mode (`App::toggle_interactive`).
    pub toggle: bool,
    /// Le geste de déplacement, s'il y en a un — l'hôte pose la fenêtre, jamais le panneau (voir
    /// `panels::drag`, `click_through_placement`).
    pub drag: PanelDrag,
}

/// Peint le bouton dans toute la fenêtre et remonte le clic ou le glissement de la frame.
///
/// `interactive` est le mode **global** de l'overlay, pas celui de cette fenêtre, qui l'est
/// toujours : il choisit seulement le glyphe.
///
/// **Cliquer ou déplacer, un seul geste de départ** (2026-09-28, « permets à l'utilisateur de
/// placer ce bouton où il le souhaite ») : le bouton est à la fois cliquable et saisissable
/// (`Sense::click_and_drag`). egui tranche au relâchement : un appui relâché sans bouger est un
/// clic, un appui suivi d'un mouvement au-delà de son seuil devient un glissement, et ne bascule
/// rien. Pas de poignée ni de cadenas comme la bande Récap : le bouton n'a pas la place d'en
/// porter, et un clic qui bouge d'un pixel reste un clic. Le curseur « main fermée » pendant le
/// geste dit qu'on déplace ; au repos, le bouton garde l'apparence d'un bouton.
pub fn show(ui: &mut egui::Ui, interactive: bool) -> ClickThroughOutcome {
    let glyph = if interactive {
        DsIcon::Eye
    } else {
        DsIcon::EyeOff
    };
    let rect = egui::Rect::from_min_size(ui.max_rect().min, egui::Vec2::splat(BUTTON_SIZE));
    let response = ui
        .put(
            rect,
            design::icon_button(glyph)
                .context(IconContext::FirstPlan)
                .size(BUTTON_SIZE)
                .log_name(LOG_NAME),
        )
        .interact(egui::Sense::click_and_drag());
    let drag = if response.drag_started() {
        response
            .interact_pointer_pos()
            .map_or(PanelDrag::None, PanelDrag::Started)
    } else if response.drag_stopped() {
        PanelDrag::Released
    } else if response.dragged() {
        PanelDrag::Moved
    } else {
        PanelDrag::None
    };
    if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }
    ClickThroughOutcome {
        toggle: response.clicked(),
        drag,
    }
}
