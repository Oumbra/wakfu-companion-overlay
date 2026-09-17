//! **Le glisser-déposer d'un overlay, tel que l'hôte le reçoit** (2026-09-17) — la bande Récap
//! d'abord (`panels::recap`), le panneau Combat ensuite (`panels::combat`), par le même type.
//!
//! Un panneau ne déplace jamais sa propre fenêtre : il n'en connaît ni la position à l'écran, ni
//! la fenêtre de jeu qui la borne, et de toute façon aucun panneau ne parle à l'OS (voir la doc de
//! `render_content`, qui explique pourquoi `build_ui`/`paint_content` restent pures). Il remonte
//! donc une intention, et l'hôte pose la fenêtre.
//!
//! **Une seule position circule ici : le point de saisie** ([`PanelDrag::Started`]), pris une fois
//! pour toutes au début du geste. Le suivi, lui, ne passe pas par les panneaux : l'hôte lit le
//! curseur à l'ÉCHELLE DE L'ÉCRAN et pose la fenêtre à `curseur − point de saisie` (voir
//! `recap_placement::drag_offset` et `combat_placement::drag_offset`).

/// Ce qu'un overlay déplaçable remonte à son hôte pour la frame en cours.
///
/// **[`Self::Moved`] ne porte aucune position**, et ce n'est pas un oubli. La première version de
/// ce type (matin du 2026-09-17, bande Récap) remontait la position du curseur DANS la fenêtre, à
/// charge pour l'hôte d'en déduire le déplacement : c'est une boucle, puisque bouger la fenêtre
/// change cette position sans que la souris bouge — la bande en vibrait au point d'être impossible
/// à poser (retour d'écran du jour, vidéo à l'appui, diagnostic complet dans
/// `recap_placement::drag_offset`). Le type interdit désormais de la refaire : la coordonnée qui
/// rebouclait n'existe plus.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum PanelDrag {
    /// Rien cette frame — le cas de l'immense majorité d'entre elles.
    #[default]
    None,
    /// Le bouton vient d'être enfoncé sur la poignée : la position de saisie DANS la fenêtre, en
    /// points logiques, celle que l'hôte garde jusqu'au relâchement.
    Started(egui::Pos2),
    /// Le glissement se poursuit — le signal suffit, l'hôte sait où est le curseur.
    Moved,
    /// Bouton relâché — l'hôte aimante l'overlay à sa place d'origine s'il en est proche, et
    /// persiste la position. C'est la seule étape qui écrit sur le disque.
    Released,
}

/// Lit le geste d'une poignée déjà déclarée à egui — la réponse d'un `Sense::drag()` traduite en
/// [`PanelDrag`], et le curseur qui dit que ça s'attrape.
///
/// **Le même code pour les deux overlays** : tout le fond de la bande Récap est une poignée, le
/// panneau Combat n'en offre qu'une lisière sur son bord extérieur, mais ce qu'on en fait ensuite
/// est identique au pixel près — et une divergence ici se paierait en vibration d'un côté
/// seulement, le plus difficile des bugs à voir.
///
/// `Grab`/`Grabbing` pendant que la poignée est survolée puis tenue : c'est le seul indice, à
/// l'écran, qu'un overlay sans décoration OS se déplace.
pub fn from_response(ui: &egui::Ui, response: &egui::Response) -> PanelDrag {
    if response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    if response.drag_started() {
        // La position d'appui, et non le centre de la poignée : c'est ce point-là qui doit rester
        // sous le pointeur pendant tout le geste.
        if let Some(pos) = response.interact_pointer_pos() {
            return PanelDrag::Started(pos);
        }
    }
    if response.drag_stopped() {
        return PanelDrag::Released;
    }
    if response.dragged() {
        return PanelDrag::Moved;
    }
    PanelDrag::None
}
