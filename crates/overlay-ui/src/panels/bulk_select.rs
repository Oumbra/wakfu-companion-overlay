//! **La sélection multiple d'une liste de tuiles** — l'en-tête qui l'ouvre, le bouton qui
//! supprime, et le libellé qui dit ce que ce bouton va faire.
//!
//! Écrit pour l'onglet « Suivi » le 2026-09-13, puis **partagé** avec « Alertes » et « Chat » le
//! 2026-09-16 à la demande de l'utilisateur (« ajouter le système de la suppression multiple,
//! comme dans l'onglet Suivi »). Trois écrans qui composent une liste, trois fois le même geste :
//! une copie par onglet, c'est trois libellés qui divergent au premier ajustement, et déjà le
//! bandeau in-game (`panels::watchlist`) partageait [`bulk_label`] avec le Suivi pour cette
//! raison.
//!
//! ## Les règles que ce module tient
//!
//! 1. **La sélection est un MODE**, pas une case permanente : un bouton corbeille l'ouvre, le même
//!    la referme, et le mode se lit sur ce bouton comme un onglet actif. Hors du mode, aucune case
//!    à cocher n'occupe les tuiles.
//! 2. **Sélection vide = « Supprimer tout »** (aucune exclusion cochée), la règle du web — voir
//!    [`bulk_label`]. Tout cocher à la main donne le même libellé, par cohérence.
//! 3. **Rien à commander quand il n'y a rien à supprimer** : le bouton disparaît au lieu de rester
//!    grisé quand la liste est vide (ou, pour les Alertes, quand elle n'a que des objets par
//!    défaut, qui ne se retirent pas). Le geste n'a pas d'objet — un bouton offert sur du vide se
//!    lit comme un bug (retour du 2026-09-16).
//! 4. **Quitter le mode oublie les coches** : rouvrir la sélection repart d'une liste vierge, donc
//!    de « Supprimer tout ». Une sélection qui survivrait au repli du mode agirait la fois
//!    suivante sans être visible.
//!
//! Ce module ne touche à **aucune liste** : il rend ce que l'utilisateur vient de demander
//! ([`BulkRequest`]) et l'appelant, seul à savoir ce qu'est une entrée, l'applique à la sienne.

use egui::{Rect, Vec2};

use crate::design::{self, ButtonSize, ButtonVariant, DsIcon, IconContext};

/// Hauteur de la ligne d'en-tête — **le côté du bouton icône, pas une valeur ronde**.
///
/// Elle valait 34 px au Suivi alors que le bouton en mesure 36 : centré dans une ligne plus courte
/// que lui, il débordait d'un pixel en haut ET en bas, et ce débordement du bas mangeait la
/// gouttière qui le séparait de la première tuile (relevé par l'utilisateur le 2026-09-14). La
/// ligne fait donc la taille de son plus haut occupant.
pub const HEADER_HEIGHT: f32 = design::tokens::ICON_BUTTON_SIZE;

/// Gouttière entre l'en-tête et ce qu'il coiffe.
///
/// Le titre, plus court que le bouton, garde l'air que lui donne sa ligne ; le bouton, lui, n'a que
/// cette gouttière.
pub const HEADER_GAP: f32 = 6.0;

/// Écart à poser sous l'en-tête avant un **paragraphe**, pour que le titre garde exactement l'air
/// que `design::heading` lui laisse quand il est peint seul.
///
/// Un titre ajouté à la suite réserve son encre ([`design::tokens::HEADING_INK_HEIGHT`]) puis
/// [`design::tokens::HEADING_TO_ROW`] sous elle. Ici il est **centré dans une ligne plus haute que
/// lui** (le bouton icône la commande) : la moitié de la différence lui tient déjà lieu d'écart, et
/// il ne manque que le reste. Calculé, jamais écrit à la main — un littéral se décalerait en
/// silence le jour où l'un des trois jetons bouge.
pub const HEADER_TO_PARAGRAPH: f32 =
    design::tokens::HEADING_TO_ROW - (HEADER_HEIGHT - design::tokens::HEADING_INK_HEIGHT) / 2.0;

/// Écart entre le bouton corbeille et le bouton de suppression groupée.
const COMMAND_GAP: f32 = 8.0;

/// Largeur minimale du bouton de suppression groupée — assez pour que « Supprimer tout » et
/// « Supprimer (3) » ne fassent pas sauter la ligne d'une frame à l'autre.
const BULK_MIN_WIDTH: f32 = 150.0;

/// Hauteur du bouton de suppression groupée — celle du bandeau in-game, dont il est le jumeau.
const BULK_HEIGHT: f32 = 28.0;

/// Ce que l'utilisateur vient de demander depuis l'en-tête.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BulkRequest {
    /// Rien cette frame.
    None,
    /// **Tout retirer** — sélection vide, donc aucune exclusion cochée (voir [`bulk_label`]).
    /// L'appelant retire tout ce qui est retirable, pas forcément toute la liste : aux Alertes,
    /// les objets par défaut restent.
    All,
    /// Retirer ces clés-là, et elles seules.
    Keys(Vec<String>),
}

/// L'état d'une sélection multiple, et les deux champs que les trois onglets en gardent.
///
/// Emprunté en mutable par [`show`] : les onglets le portent dans leur propre état (qui survit au
/// changement d'onglet) sans que ce module ait à connaître leur type.
pub struct BulkSelection<'a> {
    /// Le mode est-il ouvert ?
    pub mode: &'a mut bool,
    /// Les clés cochées — leur forme appartient à l'appelant, ce module ne fait que les comparer.
    pub keys: &'a mut Vec<String>,
}

/// Ce qu'il faut pour peindre l'en-tête d'une liste sélectionnable.
pub struct BulkHeader<'a> {
    /// Le titre de la liste, à gauche — `design::heading`, comme partout ailleurs.
    pub title: &'a str,
    /// **Combien d'entrées le bouton « Supprimer tout » retirerait** : zéro fait disparaître les
    /// commandes (règle 3). Ce n'est pas forcément la longueur de la liste — voir [`BulkRequest`].
    pub removable: usize,
    /// Infobulle du bouton de suppression groupée — chaque onglet dit ce qu'il retire.
    pub bulk_tooltip: &'a str,
    /// Préfixe des noms de journal : `"alertes"` donne `alertes.selection` et
    /// `alertes.supprimer-groupe`.
    pub log_prefix: &'a str,
    /// **Les commandes sont-elles offertes ?** `false` pendant un chargement, ou quand l'écran
    /// n'a pas de liste à montrer. Le titre, lui, est peint dans tous les cas.
    pub enabled: bool,
}

/// Peint la ligne d'en-tête et rend ce que l'utilisateur a demandé.
///
/// **N'ajoute aucune gouttière sous elle** : l'appelant pose la sienne ([`HEADER_GAP`] avant une
/// grille de tuiles, l'écart d'un titre avant un paragraphe), parce que ce qui suit n'est pas le
/// même d'un onglet à l'autre.
pub fn show(
    ui: &mut egui::Ui,
    width: f32,
    header: BulkHeader<'_>,
    selection: BulkSelection<'_>,
) -> BulkRequest {
    let row = ui.allocate_space(Vec2::new(width, HEADER_HEIGHT)).1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));

    let mut bascule = false;
    let mut supprimer = false;
    let select_mode = *selection.mode;
    let cochees = selection.keys.len();
    let removable = header.removable;

    cell.horizontal_centered(|ui| {
        ui.add(design::heading(header.title));
        // Règle 3 : rien à commander quand il n'y a rien à retirer.
        if !header.enabled || removable == 0 {
            return;
        }
        commands(
            ui,
            row,
            &header,
            select_mode,
            cochees,
            removable,
            &mut bascule,
            &mut supprimer,
        );
    });

    if supprimer {
        let demande = if selection.keys.is_empty() {
            BulkRequest::All
        } else {
            BulkRequest::Keys(selection.keys.clone())
        };
        *selection.mode = false;
        selection.keys.clear();
        return demande;
    }
    if bascule {
        // Règle 4 : le mode se referme sur une sélection vierge, dans les deux sens.
        *selection.mode = !*selection.mode;
        selection.keys.clear();
    }
    BulkRequest::None
}

/// Les deux boutons, ancrés au bord droit de la ligne.
#[allow(clippy::too_many_arguments)]
fn commands(
    ui: &mut egui::Ui,
    row: Rect,
    header: &BulkHeader<'_>,
    select_mode: bool,
    cochees: usize,
    removable: usize,
    bascule: &mut bool,
    supprimer: &mut bool,
) {
    let mut droite = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    let mut bouton = design::icon_button(DsIcon::Delete)
        .context(IconContext::Panel)
        .tooltip(if select_mode {
            "Quitter la sélection"
        } else {
            "Suppression multiple"
        })
        .log_name(format!("{}.selection", header.log_prefix));
    if select_mode {
        // Le mode ouvert se lit sur le bouton lui-même, comme un onglet actif.
        bouton = bouton.preview_state(design::IconButtonState::Hovered);
    }
    if droite.add(bouton).clicked() {
        *bascule = true;
    }
    if !select_mode {
        return;
    }
    droite.add_space(COMMAND_GAP);
    if droite
        .add(
            // **Rouge**, comme le « Annuler » du pied de page — décision explicite de
            // l'utilisateur le 2026-09-13, qui prévaut sur la règle « le rouge est réservé au pied
            // de fenêtre » que l'onglet Alertes avait posée.
            design::button(bulk_label(cochees, removable))
                .variant(ButtonVariant::Danger)
                .size(ButtonSize::Height(BULK_HEIGHT))
                .min_width(BULK_MIN_WIDTH)
                .tooltip(header.bulk_tooltip)
                .log_name(format!("{}.supprimer-groupe", header.log_prefix)),
        )
        .clicked()
    {
        *supprimer = true;
    }
}

/// Le libellé du bouton de suppression groupée — **la règle du web**, reprise telle quelle.
///
/// Aucune tuile cochée se lit « aucune exclusion » et non « rien à faire » : le bouton porte alors
/// « Supprimer tout » et agit sur la liste entière. Tout cocher à la main donne le même libellé,
/// par cohérence — même résultat, deux chemins pour y arriver.
///
/// **Partagé par les trois onglets et le bandeau in-game** (`panels::watchlist`) : le bouton y est
/// le même, jusqu'à sa variante et sa hauteur.
pub fn bulk_label(selected: usize, total: usize) -> String {
    if selected == 0 || selected == total {
        "Supprimer tout".to_string()
    } else {
        format!("Supprimer ({selected})")
    }
}

/// Coche ou décoche une clé — le geste d'une tuile en mode sélection.
pub fn toggle(keys: &mut Vec<String>, key: &str) {
    if let Some(pos) = keys.iter().position(|k| k == key) {
        keys.remove(pos);
    } else {
        keys.push(key.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libelle_du_bouton_groupe() {
        // Sélection vide : aucune exclusion, donc tout.
        assert_eq!(bulk_label(0, 12), "Supprimer tout");
        // Tout coché : même résultat, même libellé.
        assert_eq!(bulk_label(12, 12), "Supprimer tout");
        assert_eq!(bulk_label(3, 12), "Supprimer (3)");
    }

    #[test]
    fn cocher_puis_recocher_retire_la_cle() {
        let mut keys = Vec::new();
        toggle(&mut keys, "Bouftou::42");
        assert_eq!(keys, vec!["Bouftou::42".to_string()]);
        toggle(&mut keys, "Larve::");
        toggle(&mut keys, "Bouftou::42");
        assert_eq!(keys, vec!["Larve::".to_string()]);
    }
}
