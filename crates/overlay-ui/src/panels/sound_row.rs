//! **La ligne du son d'une alerte** — « Tester le son de l'alerte » et, pour le Suivi et le Chat,
//! la case « Couper le son des notifications » posée juste dessous (2026-09-15).
//!
//! Demande utilisateur : « ajouter une option dans Suivi et Chat permettant de couper le son des
//! notifications, juste en dessous de la ligne "Tester le son de l'alerte" ».
//!
//! ## Trois onglets, une seule ligne
//!
//! La ligne d'essai existait à l'identique dans « Alertes », « Suivi » et « Chat » — trois copies
//! des mêmes quinze lignes, que leurs docs respectives décrivaient déjà comme « la même que dans
//! les deux autres ». Elle vit ici depuis que la case l'accompagne, pour la même raison que
//! [`crate::panels::feature_switch`] tient les trois cases « Activer … » : deux copies d'un même
//! réglage divergent au premier ajustement.
//!
//! Chaque onglet fait entendre le son qu'il commande — décompte à zéro pour le Suivi, ramassage
//! pour les Alertes, message trouvé pour le Chat : ce module ne joue rien lui-même, il renvoie le
//! clic et c'est l'hôte qui a le périphérique audio (§17.3 bis du plan).
//!
//! ## Couper le son, ce n'est pas couper la fonctionnalité
//!
//! La case est **sous** la ligne d'essai et **au-dessus** du premier réglage de l'onglet, parce
//! qu'elle parle du même sujet qu'elle : ce qu'on entend. Cochée, l'alerte ne fait plus de bruit
//! mais **la carte s'affiche toujours** par-dessus le jeu — c'est ce qui la distingue de la case
//! « Activer … » du haut de l'onglet, qui, elle, coupe les deux canaux
//! ([`crate::panels::feature_switch`]).
//!
//! **Le bouton d'essai est grisé quand le son est coupé** : proposer d'écouter ce qu'on vient de
//! faire taire serait une promesse que le jeu ne tiendra pas. C'est la même règle que le champ de
//! durée grisé sous une « Fermeture automatique » décochée (`panels::alerts_tab`), et que la case
//! de son grisée sous une notification de tour décochée (`panels::options_modal`) : un réglage qui
//! n'a plus d'effet se voit avant le clic.

use egui::{Color32, RichText, Vec2};

use crate::design::{self, DsIcon, IconContext};

/// **Les sourdines, ensemble** — Suivi et Chat.
///
/// Un seul type plutôt que deux `bool` baladeurs, pour la même raison que
/// [`crate::panels::feature_switch::FeatureToggles`] : ils voyagent toujours ensemble (config
/// persistée → fenêtre Options → hôte → thread Engine), et les confondre en route couperait le son
/// de l'autre fonctionnalité.
///
/// **Alertes n'en a pas**, et ce n'est pas un oubli : le son d'un ramassage se coupe déjà objet par
/// objet, à la tuile (`panels::alerts_tab`) — une sourdine globale y ferait double emploi avec un
/// réglage plus fin.
///
/// `Default` vaut « rien n'est coupé » : c'est bien `false` pour les deux, le dérivé convient donc
/// ici, contrairement à `FeatureToggles` dont le défaut est `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AlertMutes {
    /// L'alerte de décompte arrivé à zéro (`alert_sound::play_countdown_alert`) est muette.
    pub suivi: bool,
    /// L'alerte de message trouvé (`alert_sound::play_chat_alert`) est muette.
    pub chat: bool,
}

/// Texte courant — blanc, comme tout texte de corps du jeu.
const TEXT: Color32 = Color32::WHITE;

/// Hauteur d'une ligne simple, sans fond — celle mesurée sur `interface-options-commandes.png`,
/// partagée par les trois onglets.
const ROW_HEIGHT: f32 = 39.0;

const BODY_FONT_SIZE: f32 = 15.0;

/// Peint la ligne « Tester le son de l'alerte » et, quand `muted` est fourni, la case « Couper le
/// son des notifications » juste dessous. Renvoie `true` la frame où le bouton d'essai est cliqué.
///
/// `log_prefix` nomme l'onglet dans le journal d'interaction (`"suivi"`, `"alertes"`, `"chat"`) :
/// les deux contrôles en dérivent leur `log_name`, pour qu'une trace dise toujours de quel écran
/// vient le clic.
///
/// `muted` est un brouillon comme le reste de la fenêtre Options — la bascule ne prend effet qu'à
/// « Valider » (voir `panels::options_modal::OptionsCommit`), son retour n'a donc rien à
/// déclencher ici et n'est pas rendu.
pub fn show(ui: &mut egui::Ui, width: f32, log_prefix: &str, muted: Option<&mut bool>) -> bool {
    let coupe = muted.as_ref().map(|muted| **muted).unwrap_or(false);
    let row = ui.allocate_space(Vec2::new(width, ROW_HEIGHT)).1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    let mut clicked = false;
    cell.horizontal_centered(|ui| {
        ui.label(
            RichText::new("Tester le son de l'alerte")
                .color(TEXT)
                .size(BODY_FONT_SIZE),
        );
        ui.add_space(12.0);
        clicked = ui
            .add(
                design::icon_button(DsIcon::Volume)
                    .context(IconContext::Panel)
                    // Voir la doc de module : grisé quand le son est coupé, et l'infobulle le dit
                    // plutôt que de promettre un son qui ne viendrait pas.
                    .enabled(!coupe)
                    .tooltip(if coupe {
                        "Le son est coupé pour cette alerte."
                    } else {
                        "Jouer le son d'alerte"
                    })
                    .log_name(format!("{log_prefix}.tester")),
            )
            .clicked();
    });

    if let Some(muted) = muted {
        let row = ui.allocate_space(Vec2::new(width, ROW_HEIGHT)).1;
        let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
        cell.horizontal_centered(|ui| {
            ui.add(
                design::checkbox(muted, "Couper le son des notifications")
                    .tooltip(
                        "Coché, l'alerte ne joue plus aucun son — la carte, elle, continue de \
                         s'afficher par-dessus le jeu.",
                    )
                    .log_name(format!("{log_prefix}.sans-son")),
            );
        });
    }

    clicked
}
