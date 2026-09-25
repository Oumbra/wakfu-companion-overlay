//! **Onglet « Raccourcis » de la fenêtre Options** — l'écran qui personnalise les raccourcis
//! clavier globaux de l'overlay.
//!
//! Il n'existait pas jusqu'au 2026-09-13 : les combinaisons étaient des constantes de `main.rs`
//! (`HOTKEY_LABEL`, `DETAILS_HOTKEY_LABEL`…), changeables seulement en recompilant. Demande
//! utilisateur : « ajouter un onglet "Raccourcis", avant paramètre, pour permettre à l'utilisateur
//! de personnaliser les raccourcis de l'overlay », en prenant pour modèle l'onglet « Commandes »
//! du jeu (`assets/design-system/interfaces/interface-options-commandes.png`).
//!
//! ## Ce que la référence du jeu donne, et ce qui en est repris
//!
//! Trois éléments, dans cet ordre : un **champ « Rechercher »** pleine largeur en tête d'écran, des
//! **en-têtes de groupe** (« Barres de raccourcis » là-bas ; « Overlay », « Suivi », « Combat »
//! ici, voir `shortcuts::ShortcutAction::section`), et des **lignes
//! `libellé à gauche → champ à droite`**. Le bouton de réinitialisation que le jeu pose en haut à
//! droite est repris au bout de la ligne de recherche : la bannière de CETTE fenêtre porte déjà sa
//! croix de fermeture (2026-09-13), y ajouter un second bouton icône donnerait deux gestes très
//! différents à deux pixels l'un de l'autre.
//!
//! **Un tableau du design system, pas des lignes peintes à la main** (`design::table`) : les
//! groupes deviennent des tableaux successifs coiffés d'un `design::heading`. Le jeu, lui, n'a
//! qu'une seule longue liste — mais huit actions réparties en trois familles se lisent mieux
//! groupées, et c'est le composant qui apporte le zébrage, la barre de défilement du jeu et
//! l'écrêtage des cellules.
//!
//! ## Transactionnel, comme le reste de la fenêtre
//!
//! Rien n'est enregistré auprès de l'OS tant que « Valider » n'a pas été cliqué (§5.1 du plan) :
//! cet onglet travaille sur un **brouillon** de [`ShortcutBindings`] que l'appelant lui prête (voir
//! `panels::options_modal::OptionsModalState::shortcuts`), et c'est l'hôte qui l'applique
//! (`shortcuts::ShortcutRegistry::apply`). « Annuler » l'abandonne, la garde de fermeture
//! (`OptionsModalState::is_dirty`) le protège comme les autres brouillons.
//!
//! ## La saisie d'une combinaison
//!
//! Clic sur une case → [`RaccourcisTabState::capturing`], et la prochaine frappe est capturée
//! (`Shortcut::from_egui`). Deux garde-fous portés par `crate::shortcuts`, rappelés ici parce
//! qu'ils décident de ce que l'écran affiche : une combinaison **sans modificateur** est refusée
//! sauf sur une **touche de fonction** (ces raccourcis sont globaux — une lettre nue serait volée à
//! Wakfu lui-même, une touche de fonction ne s'écrit pas ; c'est l'exception qui porte les
//! raccourcis multicompte, F1/F2 par défaut), et un **doublon** est signalé immédiatement plutôt
//! qu'au moment de valider (l'OS refuserait le second enregistrement).
//!
//! ## La case « Activer les raccourcis multicompte » (2026-09-25)
//!
//! Posée en tête du groupe « Multicompte », au-dessus de son tableau, comme « Activer le suivi »
//! en tête de l'onglet Suivi : une seule case pour les deux raccourcis (demande utilisateur). Elle
//! travaille sur le même brouillon que les combinaisons (`ShortcutBindings::multiaccount_enabled`),
//! donc passe par « Valider » et « Réinitialiser » comme elles. Décochée, les combinaisons restent
//! modifiables — elles resserviront à la réactivation — mais ne sont plus enregistrées.
//!
//! **Les raccourcis globaux sont suspendus tant que la fenêtre Options est ouverte**
//! (`main.rs::open_options_modal`) : sans cela, l'OS avalerait la frappe que l'utilisateur essaie
//! justement d'assigner — à commencer par la combinaison qui vient d'ouvrir cette fenêtre.

use egui::{Color32, RichText};

use crate::design::{self, DsIcon, InputSize, TableAlign, TableBody, TableColumn};
use crate::shortcuts::{Shortcut, ShortcutAction, ShortcutBindings};

/// Texte courant — blanc, comme tout texte de corps du jeu (même jeton que `panels::alerts_tab`).
const TEXT: Color32 = Color32::WHITE;
/// Gris du jeu (`#b8b9ba`) — libellés secondaires, voir `panels::alerts_tab::SUBDUED`.
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);
/// Corps du texte courant — **15 px, comme les onglets « Alertes » et « Suivi »**.
///
/// 12 px à la première version, et c'était trop petit (retour utilisateur du 2026-09-13 : la
/// police des libellés d'action doit être « un tout petit plus grande ») : cet onglet était le seul
/// de la fenêtre à ne pas être au corps commun, ce qui se voyait d'un onglet à l'autre.
const BODY_FONT_SIZE: f32 = 15.0;

/// Air entre deux blocs — même valeur relevée que les autres onglets
/// (`panels::options_modal::SECTION_GAP`, 17 px : le seul signal de regroupement du jeu).
const SECTION_GAP: f32 = 17.0;

/// Largeur de la colonne « Combinaison ».
///
/// **Fixe, et calée sur le plus long libellé affichable** (`Ctrl+Alt+Shift+PageSuiv`, 23
/// caractères) plus les deux marges du champ : une colonne élastique ferait sauter la largeur des
/// cases d'une ligne à l'autre selon la longueur du libellé d'action, alors que ces cases forment
/// une colonne que l'œil suit de haut en bas.
///
/// 176 -> 212 px après la première capture : le texte d'attente y était écrêté (« Tapez la
/// combinais »), et une combinaison à trois modificateurs l'aurait été aussi. Le champ ne peut pas
/// défiler ici — il est en lecture seule, personne n'y met le curseur pour voir la fin.
const SHORTCUT_COLUMN: f32 = 212.0;
/// Gouttière entre le champ de recherche et le bouton « Réinitialiser ».
const ROW_GAP: f32 = 10.0;
/// Hauteur de la ligne « Rechercher + Réinitialiser » — celle du bouton, le champ gardant sa
/// hauteur native et se centrant dessus (même composition que la ligne « chemin + Parcourir » de
/// l'onglet « Paramètres », voir `panels::options_modal`).
const SEARCH_ROW_HEIGHT: f32 = 36.0;
/// Hauteur d'une ligne de tableau — **39 px, le pas relevé sur la référence de cet onglet même**
/// (`interface-options-commandes.png`, voir `panels::alerts_tab::ROW_HEIGHT` qui cite la même
/// mesure). 34 px à la première version, une valeur choisie ; le passage du corps de texte à 15 px
/// demandait de toute façon plus d'air.
const ROW_HEIGHT: f32 = 39.0;

/// Ce que l'onglet garde entre deux frames. **Pas les raccourcis** : ceux-ci sont le brouillon que
/// l'appelant prête à [`show`], comme le profil d'alertes de `panels::alerts_tab`.
#[derive(Debug, Default, Clone)]
pub struct RaccourcisTabState {
    /// Filtre du champ « Rechercher » — comparé en minuscules au libellé de l'action, à sa section
    /// ET à la combinaison affichée (chercher « Ctrl+Shift+W » doit retrouver sa ligne).
    pub search: String,
    /// L'action dont la case attend une frappe, le cas échéant. Un second clic sur la même case,
    /// Échap, ou un changement d'onglet l'abandonne.
    pub capturing: Option<ShortcutAction>,
    /// Message de la dernière frappe refusée (touche nue autre qu'une touche de fonction, ou sans équivalent
    /// système) ou du dernier doublon détecté — `None` quand tout va bien.
    pub error: Option<String>,
}

/// Ce que [`show`] a produit cette frame. L'onglet n'agit jamais sur l'OS lui-même (voir doc de
/// module) : seul le brouillon change, et l'hôte l'applique à la validation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RaccourcisTabAction {
    #[default]
    None,
}

/// Peint l'onglet dans le panneau de section de la fenêtre Options.
///
/// `bindings` est le **brouillon** (voir doc de module) : modifié en place au fil des frappes,
/// jamais appliqué ici.
pub fn show(
    ui: &mut egui::Ui,
    panel: &design::PanelZones,
    state: &mut RaccourcisTabState,
    bindings: &mut ShortcutBindings,
) -> RaccourcisTabAction {
    let width = panel.inner.width();

    // La frappe est lue AVANT de peindre les lignes : la case concernée affiche ainsi la nouvelle
    // combinaison dès cette frame, et non à la suivante.
    capture_pending_key(ui, state, bindings);

    ui.add(design::heading("Raccourcis"));
    paragraph(
        ui,
        "Cliquez sur une combinaison pour la changer, puis tapez la nouvelle. Ces raccourcis \
         fonctionnent même quand le jeu a le focus : Ctrl, Alt ou Shift est donc requis, sauf sur \
         une touche de fonction (F1 à F12) — toute autre touche seule serait prise à Wakfu.",
    );
    ui.add_space(SECTION_GAP);

    search_row(ui, state, bindings, width);
    ui.add_space(SECTION_GAP);

    // Le message de la dernière frappe refusée vit AU-DESSUS de la liste, pas sous la ligne
    // fautive : les lignes sont dans des tableaux de hauteur fixe, y glisser une ligne de texte
    // déplacerait toutes les suivantes à chaque frappe refusée — et posé sous une liste défilante,
    // il serait hors écran au moment précis où il compte.
    if let Some(error) = &state.error {
        ui.add(
            design::info_text(error)
                .tone(design::InfoTone::Alert)
                .width(width)
                .log_name("raccourcis.erreur"),
        );
        ui.add_space(SECTION_GAP);
    }

    // **Les groupes défilent, le champ de recherche et le message restent.** Dix raccourcis en
    // quatre groupes dépassent la hauteur du panneau ; filtrer ou lire un refus depuis le bas de
    // la liste exigerait sinon de remonter.
    let needle = state.search.trim().to_lowercase();
    panel.scroll_area(ui, "raccourcis.liste", |ui, content_width| {
        let mut vu = 0usize;
        for section in SECTIONS {
            let actions: Vec<ShortcutAction> = ShortcutAction::ALL
                .into_iter()
                .filter(|action| action.section() == section)
                .filter(|action| matches_search(*action, bindings.get(*action), &needle))
                .collect();
            if actions.is_empty() {
                // Un en-tête de groupe sans ligne en dessous se lirait comme « ce groupe est
                // vide », alors que c'est la recherche qui l'a vidé.
                continue;
            }
            if vu > 0 {
                ui.add_space(SECTION_GAP);
            }
            vu += actions.len();
            if section == MULTIACCOUNT_SECTION {
                multiaccount_toggle(ui, bindings);
                ui.add_space(design::tokens::CHECKBOX_ROW_GAP);
            }
            shortcut_table(ui, state, bindings, &actions, content_width, section);
        }

        if vu == 0 {
            ui.add(
                design::info_text("Aucun raccourci ne correspond à cette recherche.")
                    .tone(design::InfoTone::Info)
                    .width(content_width)
                    .log_name("raccourcis.sans-resultat"),
            );
        }
    });

    RaccourcisTabAction::None
}

/// Les sections, dans l'ordre d'affichage — celui de `ShortcutAction::ALL`, dont les actions d'une
/// même section sont contiguës.
///
/// « Compte » a disparu le 2026-09-13 avec son unique action (la déconnexion, devenue un bouton de
/// l'onglet « Paramètres ») — un groupe vide ne s'affiche pas, mais le laisser ici ferait croire
/// qu'il reste quelque chose à y ranger. Le test `toutes_les_sections_sont_listees` garantit
/// l'inverse : aucune action ne peut se retrouver sans groupe.
const SECTIONS: [&str; 4] = ["Overlay", "Suivi", "Combat", MULTIACCOUNT_SECTION];

/// Le groupe qui porte la case d'activation des raccourcis multicompte — voir doc de module.
const MULTIACCOUNT_SECTION: &str = "Multicompte";

/// La case « Activer les raccourcis multicompte », en tête de son groupe.
fn multiaccount_toggle(ui: &mut egui::Ui, bindings: &mut ShortcutBindings) {
    ui.add(
        design::checkbox(
            bindings.multiaccount_enabled_mut(),
            "Activer les raccourcis multicompte",
        )
        .tooltip(
            "Inviter et Suivre tapent une commande dans le chat du jeu à votre place. \
             Décochée, leurs touches ne sont plus prises au jeu ni aux autres applications.",
        )
        .log_name("raccourcis.multicompte-actif"),
    );
}

/// Message affiché quand la frappe capturée n'est pas assignable — `pub` pour que le harnais de
/// capture (`overlay-testkit`, `tests/panels.rs::options_raccourcis`) le REPRENNE plutôt que de le
/// recopier : une capture qui montre un message que le code n'affiche plus ne vérifie rien.
pub const MESSAGE_COMBINAISON_REFUSEE: &str =
    "Combinaison refusée : ajoutez Ctrl, Alt ou Shift, ou utilisez une touche de fonction seule (F1 à F12).";

fn paragraph(ui: &mut egui::Ui, text: &str) {
    // Un paragraphe, pas un `design::info_text` : celui-ci porte une pastille et un fond, réservés
    // à ce qui doit arrêter l'œil (même arbitrage que `panels::alerts_tab::paragraph`).
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .color(SUBDUED)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE))
                .line_height(Some(BODY_FONT_SIZE * 1.5)),
        )
        .wrap_mode(egui::TextWrapMode::Wrap),
    );
}

/// Champ « Rechercher » + bouton « Réinitialiser », sur une seule ligne.
///
/// **Le bouton prend sa largeur NATURELLE** (libellé + marges du design system) et le champ occupe
/// tout le reste — jamais l'inverse : une largeur figée pour le bouton écrête son libellé dès que
/// la police ou le mot changent, ce qui s'est vu (« Réinitialise ») à la première capture.
fn search_row(
    ui: &mut egui::Ui,
    state: &mut RaccourcisTabState,
    bindings: &mut ShortcutBindings,
    width: f32,
) {
    let reset = design::button("Réinitialiser")
        .size(design::ButtonSize::Height(SEARCH_ROW_HEIGHT))
        .log_name("raccourcis.reinitialiser");
    let reset_width = reset.desired_size(ui).x;
    let row = ui.allocate_space(egui::vec2(width, SEARCH_ROW_HEIGHT)).1;
    // Plancher à zéro : si la ligne devenait plus étroite que le bouton, un rectangle de largeur
    // négative serait inversé par egui et peint n'importe où (même garde que `options_modal`).
    let field_width = (row.width() - reset_width - ROW_GAP).max(0.0);
    let field_rect = egui::Rect::from_center_size(
        egui::pos2(row.left() + field_width / 2.0, row.center().y),
        egui::vec2(field_width, InputSize::Standard.height()),
    );
    ui.put(
        field_rect,
        design::input(&mut state.search)
            .size(InputSize::Standard)
            .width(field_width)
            .leading_icon(DsIcon::Search)
            .clearable(true)
            .placeholder("Rechercher")
            .log_name("raccourcis.recherche"),
    );
    let reset_rect = egui::Rect::from_min_size(
        egui::pos2(row.right() - reset_width, row.top()),
        egui::vec2(reset_width, SEARCH_ROW_HEIGHT),
    );
    {
        if ui.put(reset_rect, reset).clicked() {
            // Ne touche QUE les raccourcis, jamais les autres onglets : c'est la portée du bouton
            // ↺ du jeu, qui vit dans l'écran qu'il réinitialise. Et comme tout le reste de cette
            // fenêtre, ça ne sort pas du brouillon tant que « Valider » n'est pas cliqué — un clic
            // malheureux se rattrape par « Annuler ».
            *bindings = ShortcutBindings::default();
            state.capturing = None;
            state.error = None;
            tracing::debug!("[raccourcis] brouillon réinitialisé aux combinaisons par défaut.");
        }
    }
}

/// Un tableau par section : colonne élastique **intitulée du nom de la section**, colonne
/// « Combinaison » fixe.
///
/// **Le groupe est porté par l'en-tête du tableau, et non par un `design::heading` au-dessus** :
/// celui-ci peint son libellé 7 px à GAUCHE du rectangle qu'il alloue (le retrait de section du
/// jeu, voir son composant), ce que l'écrêtage d'une zone défilante coupe — première lettre
/// rognée, constaté à la capture. L'en-tête du tableau dit la même chose, à sa place, et fait
/// disparaître au passage un « Action » répété quatre fois.
fn shortcut_table(
    ui: &mut egui::Ui,
    state: &mut RaccourcisTabState,
    bindings: &mut ShortcutBindings,
    actions: &[ShortcutAction],
    width: f32,
    section: &str,
) {
    design::table()
        .column(TableColumn::flex(section, 1.0))
        .column(TableColumn::fixed("Combinaison", SHORTCUT_COLUMN).align(TableAlign::End))
        .body(TableBody::Rows(actions.len()))
        .row_height(ROW_HEIGHT)
        .width(width)
        .log_name(format!("raccourcis.{}", section.to_lowercase()))
        .show(ui, |row| {
            let Some(action) = actions.get(row.index()).copied() else {
                return;
            };
            row.cell(|ui| {
                ui.label(
                    RichText::new(action.label())
                        .color(TEXT)
                        .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
                );
            });
            row.cell(|ui| shortcut_cell(ui, state, bindings, action));
        });
}

/// La case d'une ligne : le libellé de la combinaison dans un champ en LECTURE SEULE, cliquable.
///
/// **Un champ et non un bouton** : c'est ce que montre la référence du jeu, et c'est juste — la
/// case porte une valeur qu'on remplace, pas une action qu'on déclenche. Le clic est capté par une
/// zone posée sur le rectangle du champ, `design::input` n'étant pas cliquable en lecture seule
/// (voir sa doc : `interactive(false)` retire la saisie ET le focus).
fn shortcut_cell(
    ui: &mut egui::Ui,
    state: &mut RaccourcisTabState,
    bindings: &ShortcutBindings,
    action: ShortcutAction,
) {
    let capturing = state.capturing == Some(action);
    // Le champ affiche ce qu'il attend quand il écoute ; `design::input` peint sa valeur, jamais un
    // texte indicatif sur une valeur non vide, d'où la bascule ici plutôt qu'un `placeholder`.
    let mut display = if capturing {
        "En attente…".to_string()
    } else {
        bindings.label(action)
    };
    let response = ui.add(
        design::input(&mut display)
            .size(InputSize::Standard)
            .width(SHORTCUT_COLUMN - 2.0 * design::tokens::TABLE_CELL_PAD_X)
            .read_only(true)
            // Le bord rouge signale la case EN ÉCOUTE aussi bien qu'une valeur refusée : dans les
            // deux cas, c'est là que l'utilisateur doit regarder, et le design system n'a pas
            // d'autre état de bord à proposer (voir `design::components::input::InputState`).
            .error(capturing)
            .tooltip(if capturing {
                "Tapez la nouvelle combinaison (Échap pour annuler)".to_string()
            } else {
                format!("Changer le raccourci de « {} »", action.label())
            })
            .log_name(format!("raccourcis.{}", action.key())),
    );
    let click = ui
        .interact(
            response.rect,
            response.id.with("raccourcis-case"),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if click.clicked() {
        // Recliquer la case en écoute l'abandonne — sinon, une case ouverte par erreur ne se
        // referme qu'en tapant quelque chose ou en trouvant la touche Échap.
        state.capturing = (!capturing).then_some(action);
        state.error = None;
    }
}

/// Consomme la première frappe de la frame quand une case attend une combinaison.
///
/// Échap annule. Une combinaison refusée par `Shortcut::from_egui` (aucun modificateur, ou touche
/// sans équivalent `global_hotkey`) laisse la case EN ÉCOUTE avec un message : l'utilisateur
/// retape, il n'a pas à recliquer.
fn capture_pending_key(
    ui: &mut egui::Ui,
    state: &mut RaccourcisTabState,
    bindings: &mut ShortcutBindings,
) {
    let Some(action) = state.capturing else {
        return;
    };
    let pressed = ui.input(|input| {
        input.events.iter().find_map(|event| match event {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => Some((*key, *modifiers)),
            _ => None,
        })
    });
    let Some((key, modifiers)) = pressed else {
        return;
    };
    if key == egui::Key::Escape {
        state.capturing = None;
        state.error = None;
        return;
    }
    match Shortcut::from_egui(key, modifiers) {
        Some(shortcut) => {
            bindings.set(action, shortcut);
            state.capturing = None;
            // Doublon signalé TOUT DE SUITE, et pas seulement au clic sur « Valider » :
            // l'utilisateur corrige la case qu'il vient de saisir, pas une autre retrouvée cinq
            // lignes plus haut. Seuls les conflits qui impliquent CETTE action sont montrés ici —
            // un conflit préexistant entre deux autres lignes ne se déclenche pas sur sa frappe.
            state.error = bindings
                .conflict()
                .filter(|(first, second)| *first == action || *second == action)
                .map(|(first, second)| conflict_message(first, second, shortcut));
        }
        None => {
            state.error = Some(MESSAGE_COMBINAISON_REFUSEE.to_string());
        }
    }
}

/// Message de doublon — partagé avec `panels::options_modal`, qui refait la vérification au moment
/// de valider (la frappe n'est pas le seul chemin : « Réinitialiser » et le brouillon initial
/// peuvent aussi, en théorie, porter un conflit).
pub fn conflict_message(
    first: ShortcutAction,
    second: ShortcutAction,
    shortcut: Shortcut,
) -> String {
    format!(
        "« {} » et « {} » utilisent la même combinaison ({}). Changez-en une avant de valider.",
        first.label(),
        second.label(),
        shortcut.label()
    )
}

/// Filtre du champ « Rechercher » — libellé, section ET combinaison affichée (chercher
/// « Ctrl+Shift+W », ou juste « W », doit retrouver la ligne correspondante).
fn matches_search(action: ShortcutAction, shortcut: Shortcut, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    action.label().to_lowercase().contains(needle)
        || action.section().to_lowercase().contains(needle)
        || shortcut.label().to_lowercase().contains(needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Toutes les sections déclarées par `ShortcutAction` doivent apparaître dans [`SECTIONS`] —
    /// une section oubliée ici ferait DISPARAÎTRE ses lignes de l'écran, sans erreur ni trace.
    #[test]
    fn toutes_les_sections_sont_listees() {
        for action in ShortcutAction::ALL {
            assert!(
                SECTIONS.contains(&action.section()),
                "section « {} » absente de SECTIONS",
                action.section()
            );
        }
    }

    #[test]
    fn recherche_sur_libelle_section_et_combinaison() {
        let bindings = ShortcutBindings::default();
        let options = ShortcutAction::Options;
        assert!(matches_search(options, bindings.get(options), ""));
        assert!(matches_search(options, bindings.get(options), "ouvrir"));
        assert!(matches_search(options, bindings.get(options), "overlay"));
        assert!(matches_search(
            options,
            bindings.get(options),
            "ctrl+shift+o"
        ));
        assert!(!matches_search(options, bindings.get(options), "combat"));
    }
}
