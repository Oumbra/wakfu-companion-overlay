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

/// Côté du bouton, en points logiques — la **hauteur des boutons Menu/Boutique du jeu** : relevée
/// sur la capture annotée de `recap_placement::DEFAULT_OFFSET`, ils occupent y = 32 à 71 dans la
/// zone cliente, soit 40 px. Le socle `FirstPlan` (36 px natifs) est mis à l'échelle, glyphe
/// compris (voir `design::icon_button`).
pub const BUTTON_SIZE: f32 = 40.0;

/// Décalage du bouton, en pixels physiques depuis le coin haut-gauche de la zone cliente du jeu
/// (`GameRect::left`, `GameRect::client_top`).
///
/// - En ordonnée, 32 : le haut des boutons du jeu (24 px de fausse barre de titre, 8 px de vide,
///   même relevé que `recap_placement::DEFAULT_OFFSET`).
/// - En abscisse, 212 : le bouton Boutique finit à x = 210 (le bloc Récap, 206 px posés à x = 4,
///   « finit au bord droit du bouton Boutique »), plus [`GAME_BUTTON_GAP`].
pub const DEFAULT_OFFSET: (i32, i32) = (210 + GAME_BUTTON_GAP, 32);

/// **Écart entre deux boutons de premier plan du jeu** (demande utilisateur 2026-09-28 : « le
/// même espacement entre ce nouveau bouton et ceux du jeu »).
///
/// Mesuré sur `assets/design-system/menu-button-icon-first-plan.png`, capture à l'échelle 1 de la
/// colonne de boutons de premier plan du jeu : le socle `button-icon-first-plan.png` (36 px) s'y
/// recale tous les 38 px, huit fois de suite (y = 5, 43, 81 … 271), soit 2 px de vide entre deux
/// socles. La rangée Menu … Boutique n'a pas de capture à elle ; elle est de la même famille.
pub const GAME_BUTTON_GAP: i32 = 2;

/// Nom du bouton dans le journal (`design::icon_button::log_name`).
const LOG_NAME: &str = "clic-traversant.bascule";

/// Peint le bouton dans toute la fenêtre et renvoie `true` à la frame où il est cliqué — c'est
/// l'hôte qui bascule le mode (`App::toggle_interactive`), jamais le panneau.
///
/// `interactive` est le mode **global** de l'overlay, pas celui de cette fenêtre, qui l'est
/// toujours : il choisit seulement le glyphe.
pub fn show(ui: &mut egui::Ui, interactive: bool) -> bool {
    let glyph = if interactive {
        DsIcon::Eye
    } else {
        DsIcon::EyeOff
    };
    let rect = egui::Rect::from_min_size(ui.max_rect().min, egui::Vec2::splat(BUTTON_SIZE));
    ui.put(
        rect,
        design::icon_button(glyph)
            .context(IconContext::FirstPlan)
            .size(BUTTON_SIZE)
            .log_name(LOG_NAME),
    )
    .clicked()
}
