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
//! voir. L'état se lit sur le glyphe.
//!
//! ## Le glyphe dit ce que le clic FERA (2026-09-28)
//!
//! Retour utilisateur : « quand l'overlay est visible, il faut mettre l'œil barré pour signifier
//! à l'utilisateur que quand il clique, il va cacher ; et inversement ». Overlay interactif :
//! œil barré, « Masquer l'overlay » ; overlay en clic-traversant : œil ouvert, « Afficher
//! l'overlay » — le raccourci entre parenthèses, comme sur les boutons du bandeau Suivi.
//!
//! ## Le fond de section du jeu (2026-09-28)
//!
//! Le cadeau, à côté de Boutique, est posé dans un cadre sombre — « un fond un peu en mode
//! section, comme les boutons plus, moins [...] 2 pixels de large et de hauteur ». Le bouton le
//! reprend : `DsTexture::ButtonIconFirstPlanSection` (40 × 42), et le socle à sa taille native
//! (36 px) posé dedans à (2, 2). La fenêtre fait la taille de la section.
//!
//! ## Une infobulle, dans une fenêtre qui ne s'agrandit qu'au survol
//!
//! Une infobulle egui ne s'affiche qu'à l'intérieur de sa fenêtre OS. Lui en faire la place en
//! permanence agrandirait une fenêtre qui capte TOUJOURS les clics — une bande de ~250 × 40 px
//! sous les boutons du jeu où le clic n'atteindrait plus jamais le jeu, même en clic-traversant.
//! L'hôte l'agrandit donc **le temps du survol seulement** (`click_through_placement::
//! tip_window`) : le bouton ne bouge pas d'un pixel à l'écran, il se peint au décalage que
//! l'hôte lui donne ([`ClickThroughTip`]), et l'infobulle s'ouvre dans la réserve. Dès que la
//! souris quitte le bouton, la fenêtre se rétracte.
//!
//! L'infobulle se lit **centrée sur le bouton**, dessous par défaut, au-dessus quand il est posé
//! trop bas, et glissée juste assez pour ne jamais sortir du jeu quand il est contre un bord
//! (demande utilisateur 2026-09-28, voir `click_through_placement::tip_window` et
//! `design::Tooltip::slide`). Et même agrandie, la fenêtre ne capte le clic que sur le bouton :
//! la réserve et l'infobulle le laissent passer au jeu (`crate::hit_region`).

use crate::design::{self, DsIcon, DsTexture, IconContext};
use crate::panels::drag::PanelDrag;
use crate::shortcuts::{ShortcutAction, ShortcutBindings};

/// Taille de la fenêtre au repos, en points logiques : le **fond de section** du jeu
/// (`DsTexture::ButtonIconFirstPlanSection`, 40 × 42) — 2 px de cadre sur les côtés et en haut,
/// 4 en bas, autour du socle de 36 px.
///
/// Mesuré sur la capture du jeu du 2026-09-28 (cadeau à côté de Boutique) : le cadre du cadeau
/// occupe y = 31 à 72 de la zone cliente, sur ~40 px de large. Il remplace le socle seul mis à
/// l'échelle 40 × 40 d'avant, qui n'avait pas de cadre.
pub const SECTION_SIZE: egui::Vec2 = egui::vec2(40.0, 42.0);

/// Décalage du socle dans sa section — le cadre du jeu, 2 px en haut et à gauche.
const SOCKET_INSET: f32 = 2.0;

/// Côté du socle, sa taille native : le glyphe y garde les cotes du jeu (voir
/// `design::icon_button`).
const SOCKET_SIZE: f32 = crate::design::tokens::ICON_BUTTON_SIZE;

/// **Place que l'hôte ajoute à la fenêtre au survol**, en points logiques, pour y ouvrir
/// l'infobulle : sa largeur totale (bouton compris) et la hauteur ajoutée au-dessus ou en dessous.
///
/// Mesurée sur le libellé le plus long, « Afficher l'overlay (Ctrl+Shift+W) » : ~210 px de large
/// et 33 px de haut cadre compris, plus `design::tokens::TOOLTIP_GAP` (5 px) sous le cadre de la
/// section. Arrondi à 240 × 40 pour qu'un raccourci personnalisé un peu plus long tienne encore.
///
/// 36 px de haut jusqu'au 2026-09-28 : l'infobulle, ancrée alors sur le socle caché dans le cadre,
/// y tenait tout juste — et paraissait collée au bouton. Ancrée sur le cadre, elle en demande 38.
pub const TIP_RESERVE: egui::Vec2 = egui::vec2(240.0, 40.0);

/// **Où est le bouton dans sa fenêtre, et de quel côté son infobulle s'ouvre** — ce que l'hôte
/// décide au survol (voir la doc de module). Au repos, le bouton est au coin de sa fenêtre et
/// rien n'est ouvert : c'est le défaut.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ClickThroughTip {
    /// Décalage du coin haut-gauche de la section dans la fenêtre, en points logiques — non nul
    /// quand la fenêtre s'est étendue à gauche ou au-dessus du bouton.
    pub button_origin: egui::Vec2,
    /// L'infobulle s'ouvre AU-DESSUS du bouton — quand il est posé trop bas dans le jeu pour
    /// qu'elle tienne dessous. En dessous sinon, sous la rangée de boutons du jeu.
    pub above: bool,
}

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

/// Peint le bouton et remonte le clic ou le glissement de la frame.
///
/// `interactive` est le mode **global** de l'overlay, pas celui de cette fenêtre, qui l'est
/// toujours : il choisit le glyphe et l'infobulle.
///
/// **Cliquer ou déplacer, un seul geste de départ** (2026-09-28, « permets à l'utilisateur de
/// placer ce bouton où il le souhaite ») : le bouton est à la fois cliquable et saisissable
/// (`Sense::click_and_drag`). egui tranche au relâchement : un appui relâché sans bouger est un
/// clic, un appui suivi d'un mouvement au-delà de son seuil devient un glissement, et ne bascule
/// rien. Pas de poignée ni de cadenas comme la bande Récap : le bouton n'a pas la place d'en
/// porter, et un clic qui bouge d'un pixel reste un clic. Le curseur « main fermée » pendant le
/// geste dit qu'on déplace ; au repos, le bouton garde l'apparence d'un bouton.
pub fn show(
    ui: &mut egui::Ui,
    interactive: bool,
    shortcuts: &ShortcutBindings,
    tip: ClickThroughTip,
) -> ClickThroughOutcome {
    let (glyph, label) = if interactive {
        (DsIcon::EyeOff, "Masquer l'overlay")
    } else {
        (DsIcon::Eye, "Afficher l'overlay")
    };
    let section = egui::Rect::from_min_size(ui.max_rect().min + tip.button_origin, SECTION_SIZE);
    design::DesignSystem::get(ui.ctx()).paint(
        ui.painter(),
        section,
        DsTexture::ButtonIconFirstPlanSection,
        egui::Color32::WHITE,
    );
    let socket = egui::Rect::from_min_size(
        section.min + egui::Vec2::splat(SOCKET_INSET),
        egui::Vec2::splat(SOCKET_SIZE),
    );
    let response = ui
        .put(
            socket,
            design::icon_button(glyph)
                .context(IconContext::FirstPlan)
                .size(SOCKET_SIZE)
                .log_name(LOG_NAME),
        )
        .interact(egui::Sense::click_and_drag());
    let drag = if response.drag_started() {
        // Le point de saisie depuis le coin de la FENÊTRE au repos, c'est-à-dire de la section :
        // c'est elle que l'hôte place (`click_through_placement::drag_offset`).
        response
            .interact_pointer_pos()
            .map_or(PanelDrag::None, |pos| {
                PanelDrag::Started((pos - section.min).to_pos2())
            })
    } else if response.drag_stopped() {
        PanelDrag::Released
    } else if response.dragged() {
        PanelDrag::Moved
    } else {
        PanelDrag::None
    };
    if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else {
        // Ancrée sur la SECTION, pas sur le socle : l'écart du design system se compte depuis le
        // cadre visible — depuis le socle, caché dedans, l'infobulle paraissait collée.
        design::tooltip(&response)
            .anchor(section)
            .slide()
            .side(if tip.above {
                design::TooltipSide::Above
            } else {
                design::TooltipSide::Below
            })
            .text(format!(
                "{label} ({})",
                shortcuts.label(ShortcutAction::Toggle)
            ));
    }
    ClickThroughOutcome {
        toggle: response.clicked(),
        drag,
    }
}
