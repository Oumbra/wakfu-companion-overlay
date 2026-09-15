//! **Réordonnancement de tuiles au glisser-déposer** — la mécanique partagée par les deux écrans
//! qui montrent la liste de suivi : l'onglet « Suivi » de la fenêtre Options
//! ([`crate::panels::suivi_tab`]) et le bandeau in-game ([`crate::panels::watchlist`]).
//!
//! Le geste vient du site (`TrackerStripComponent`, dépôt web), et il y est écrit deux fois — une
//! par vue. Ici il n'est écrit qu'une fois : deux copies, ce serait deux listes qui finiraient par
//! ne plus se réordonner pareil, alors qu'elles réordonnent *la même liste*.
//!
//! ## Ce que ce module peint, et pourquoi
//!
//! Le navigateur fabrique un fantôme natif pendant un glissement ; egui ne peint rien du tout.
//! Sans ces trois marques, le geste serait invisible :
//!
//! | Marque | Ce qu'elle dit |
//! | --- | --- |
//! | Voile sur la tuile prise | l'entrée n'est plus là — le fantôme en est le seul exemplaire |
//! | Fantôme sous le pointeur | ce qu'on déplace, tenu **par où on l'a pris** |
//! | Liseré or sur la tuile visée | la place que l'entrée prendra en se posant |
//!
//! Et une quatrième, qui n'est pas peinte : la **croix fléchée du jeu**
//! ([`egui::CursorIcon::Move`], traduite en bitmap par [`crate::cursor`]), au survol comme pendant
//! tout le geste. C'est le seul signe qu'une tuile se déplace — ces écrans n'ont ni poignée ni
//! libellé pour l'annoncer.
//!
//! ## Ce qu'il ne fait pas
//!
//! Il ne touche à aucune liste. [`handle`] rend ce que le geste a produit, l'appelant décide quoi
//! en faire : l'onglet « Suivi » réordonne son brouillon, le bandeau — qui est en lecture seule sur
//! les entrées (voir `overlay_engine::watchlist`) — demande à l'hôte d'écrire. [`reorder`] est là
//! pour que les deux le fassent au même rang.

use egui::{Rect, Vec2};

use crate::design::{self, SlotFrame};

/// Voile posé sur la place d'ORIGINE d'une tuile en cours de déplacement.
const DRAG_SOURCE_SCRIM: egui::Color32 = egui::Color32::from_black_alpha(0xAA);

/// Liseré posé sur la tuile SURVOLÉE — l'or des sélections, le ton du jeu pour « ceci est visé ».
const DROP_MARKER: egui::Color32 = design::tokens::ITEM_SLOT_SELECTED_BORDER;

/// Opacité du fantôme : assez pour dire « en vol », assez pour rester lisible.
const DRAG_GHOST_OPACITY: f32 = 0.9;

/// Ce qu'une tuile emporte en vol — son rang dans la liste, et rien d'autre : la liste ne bouge pas
/// tant que le pointeur n'est pas relâché, un rang suffit donc à retrouver l'entrée.
///
/// Type nommé plutôt qu'un `usize` nu : [`egui::DragAndDrop`] indexe sa charge utile **par type**,
/// et un `usize` anonyme serait attrapé par n'importe quel autre glisser-déposer de l'application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DragIndex(usize);

/// La tuile qu'on vient de peindre, telle que le fantôme doit la redessiner.
pub struct Tile {
    /// Rang dans la liste affichée.
    pub index: usize,
    /// Icône déjà résolue — la même que celle de la tuile, sinon le fantôme montrerait autre chose.
    pub icon: egui::TextureId,
    /// Cadre de l'emplacement : c'est lui qui dit quelle entrée est en vol (rareté, ou neutre).
    pub frame: SlotFrame,
    /// Côté du carré.
    pub size: f32,
}

/// Ce que le geste dit à la tuile qui vient d'être peinte.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Gesture {
    /// Rang de la tuile en vol — **n'importe laquelle de la liste**, pas seulement celle-ci. Un
    /// écran s'en sert pour retirer ce qui n'a plus de sens pendant un déplacement : la tuile
    /// survolée est une destination, pas une cible de clic.
    pub dragged: Option<usize>,
    /// Rang de la tuile qui vient d'être lâchée SUR celle-ci, à la frame du dépôt.
    pub dropped: Option<usize>,
}

impl Gesture {
    /// Un déplacement est-il en cours, ici ou ailleurs dans la liste ?
    pub fn in_flight(&self) -> bool {
        self.dragged.is_some()
    }
}

/// Branche le glisser-déposer sur une tuile déjà peinte et déjà allouée en
/// [`egui::Sense::click_and_drag`] (ou `drag`) — voir [`Gesture`] pour ce qu'elle en apprend.
///
/// À appeler pour CHAQUE tuile réordonnable, après l'avoir peinte : c'est là que le fantôme se pose
/// au-dessus d'elle, et la barre d'insertion sur elle.
pub fn handle(ui: &egui::Ui, response: &egui::Response, tuile: Tile) -> Gesture {
    // Pose la charge utile à l'instant où le glissement démarre — sans effet les autres frames.
    response.dnd_set_drag_payload(DragIndex(tuile.index));

    // Lu APRÈS la pose : la tuile qui vient de partir se voit voilée dès la première frame du
    // geste, sans attendre la suivante.
    let Some(depuis) = egui::DragAndDrop::payload::<DragIndex>(ui.ctx()).map(|charge| charge.0)
    else {
        // Hors geste, la croix fléchée dit ce que la tuile permet — aucun autre signe ne l'annonce.
        //
        // **`enabled()` en plus de `contains_pointer()`** (2026-09-15) : `contains_pointer` ne dit
        // que la géométrie, il reste vrai sur un `Ui` désactivé. Sans ce garde, une tuile d'un
        // onglet dont la fonctionnalité est coupée (`panels::feature_switch`) annonçait encore un
        // déplacement qu'aucun glissement n'aurait exécuté — un curseur qui ment.
        if response.contains_pointer() && response.enabled() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Move);
        }
        return Gesture::default();
    };

    // **La croix pendant tout le geste, sur toute la liste.** Le pointeur peut sortir des tuiles
    // (gouttières, bord de la bande) sans que le geste s'interrompe, et le curseur ne doit pas
    // clignoter d'une forme à l'autre au passage. Posée explicitement plutôt que laissée au
    // `Grabbing` que `DragAndDrop` met par défaut en fin de frame : c'est la croix qui a été
    // demandée, et c'est elle que `crate::cursor` traduit en bitmap du jeu.
    ui.ctx().set_cursor_icon(egui::CursorIcon::Move);

    let rect = response.rect;
    if depuis == tuile.index {
        // La place d'origine s'efface derrière le fantôme, maintenant seul exemplaire de l'entrée.
        ui.painter()
            .rect_filled(rect, design::tokens::ITEM_SLOT_ROUNDING, DRAG_SOURCE_SCRIM);
        ghost(ui, response.id, &tuile, rect);
        return Gesture {
            dragged: Some(depuis),
            dropped: None,
        };
    }

    if response.dnd_hover_payload::<DragIndex>().is_some() {
        marker(ui, rect);
    }
    Gesture {
        dragged: Some(depuis),
        dropped: response
            .dnd_release_payload::<DragIndex>()
            .map(|charge| charge.0),
    }
}

/// **Le fantôme qui suit le pointeur** — l'emplacement redessiné dans sa propre couche au-dessus de
/// tout, tenu par le point de saisie.
///
/// Le web réduit le sien à l'icône sur un carré neutre parce que sa tuile réelle porte des éléments
/// flottants (badge, croix) que la capture d'écran du navigateur emportait ; ici rien n'est
/// capturé, l'emplacement est repeint — il garde donc son cadre, qui dit quelle entrée est en vol.
fn ghost(ui: &egui::Ui, id: egui::Id, tuile: &Tile, rect: Rect) {
    let Some(pointeur) = ui.ctx().pointer_interact_pos() else {
        return;
    };
    // **Tenu par où on l'a pris**, et non centré sur le pointeur : la tuile garde sous le doigt le
    // point exact où l'appui a commencé, donc elle suit la souris au pixel près. Centré, le fantôme
    // recouvrait presque exactement la tuile visée — la barre d'insertion et la destination
    // disparaissaient dessous, au moment précis où on les regarde.
    let prise = ui
        .ctx()
        .input(|i| i.pointer.press_origin())
        .map(|origine| origine - rect.min)
        .unwrap_or_else(|| Vec2::splat(tuile.size / 2.0));
    egui::Area::new(id.with("fantome"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pointeur - prise)
        // Une couche qui prendrait le survol volerait aux tuiles la détection de la destination :
        // le fantôme est sous le pointeur en permanence, il masquerait tout.
        .interactable(false)
        .show(ui.ctx(), |ui| {
            ui.set_opacity(DRAG_GHOST_OPACITY);
            ui.add(
                design::item_slot()
                    .size(tuile.size)
                    .frame(tuile.frame)
                    .icon(tuile.icon),
            );
        });
}

/// **Le liseré de la tuile visée** — « ta tuile viendra ici ».
///
/// Posé sur le MÊME anneau que le cadre de l'emplacement (`design::item_slot_border_ring`), comme
/// le liseré de sélection : il remplace visuellement la bordure de rareté le temps du survol, il ne
/// s'ajoute pas à côté d'elle.
///
/// **Une barre latérale d'abord, et pourquoi elle est partie** (13/09/2026, sur la première planche
/// du bandeau) : la version initiale peignait un trait sur le bord vers lequel l'entrée allait —
/// à droite en descendant, à gauche en remontant. Deux défauts, l'un fatal :
///
/// 1. **Rognée dans la bande in-game.** La fenêtre Suivi est dimensionnée à son contenu
///    (`panels::watchlist::content_width`) : la dernière tuile touche son bord droit, et le trait
///    qui s'y posait tombait hors de la zone de défilement — invisible précisément là où on visait.
/// 2. **Redondante.** Dans les deux sens, l'entrée déplacée prend la PLACE de la tuile visée (celle
///    -ci recule ou avance d'un rang, voir [`reorder`]) : le côté n'ajoutait qu'une nuance de plus
///    à lire, là où le liseré dit la chose directement.
fn marker(ui: &egui::Ui, rect: Rect) {
    let (anneau, rayon) = design::item_slot_border_ring(rect);
    ui.painter().rect_stroke(
        anneau,
        rayon,
        egui::Stroke::new(design::tokens::ITEM_SLOT_PLAIN_STROKE, DROP_MARKER),
        egui::StrokeKind::Inside,
    );
}

/// Déplace l'élément de rang `depuis` au rang `vers` — **miroir exact de
/// `StatsStoreService.reorderWatchlist`** (dépôt web) : retrait, puis insertion au rang `vers` de
/// la liste DÉJÀ amputée.
///
/// Cette nuance est le comportement lui-même, pas un détail d'implémentation : déplacer une entrée
/// vers le bas la pose **après** la tuile visée (celle-ci a reculé d'un rang entre-temps), vers le
/// haut **à sa place**. C'est ce que la barre d'insertion annonce, et ce que le site fait déjà —
/// deux listes réordonnées différemment des deux côtés se contrediraient à la première synchro.
pub fn reorder<T>(items: &mut Vec<T>, depuis: usize, vers: usize) {
    if depuis == vers || depuis >= items.len() || vers >= items.len() {
        return;
    }
    let deplace = items.remove(depuis);
    items.insert(vers, deplace);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le déplacement suit le web au rang près.** Descendre une entrée la pose APRÈS la tuile
    /// visée, la remonter la pose à SA place.
    #[test]
    fn le_deplacement_reproduit_le_reordonnancement_du_web() {
        let mut liste = vec!["A", "B", "C", "D"];
        reorder(&mut liste, 0, 2);
        assert_eq!(liste, ["B", "C", "A", "D"]);
        reorder(&mut liste, 3, 1);
        assert_eq!(liste, ["B", "D", "C", "A"]);
    }

    /// Un rang hors liste ou identique ne touche à rien — les grilles ne devraient jamais en
    /// produire, mais une liste réordonnée « presque » serait un désordre silencieux, pas une
    /// erreur visible.
    #[test]
    fn un_deplacement_impossible_laisse_la_liste_intacte() {
        let mut liste = vec!["A", "B"];
        reorder(&mut liste, 1, 1);
        reorder(&mut liste, 0, 9);
        reorder(&mut liste, 9, 0);
        assert_eq!(liste, ["A", "B"]);
    }
}
