//! **Les notifications d'une fonctionnalité** — couper le son, et régler la fermeture automatique
//! de la carte, pour le Suivi, les Alertes et le Chat.
//!
//! Ces blocs vivent depuis le 2026-09-15 dans l'onglet « Paramètres », en **sections dédiées
//! posées après « Combat »** (demande utilisateur) — plus dans chacun des trois onglets qu'ils
//! concernent. Ce module les peint tous les trois ; il ne joue aucun son et n'écrit rien : il
//! modifie des brouillons, comme tout le reste de cette fenêtre (§17.3 bis du plan).
//!
//! **Plus de bouton d'essai** (2026-09-16) : la ligne « Tester le son des notifications » que
//! chaque section ouvrait a été retirée à la demande de l'utilisateur, et avec elle les actions
//! `TestAlertSound`/`TestChatSound`/`TestCountdownSound` que la fenêtre renvoyait à l'hôte. Les
//! sons eux-mêmes ne bougent pas (`alert_sound`), ils ne se déclenchent plus qu'en jeu.
//!
//! ## Pourquoi tout regrouper dans « Paramètres »
//!
//! La section « Combat » portait déjà, depuis le 2026-09-14, une notification et sa sourdine
//! (« Me prévenir quand un de mes personnages doit jouer », « Couper le son des notifications »).
//! Les trois autres fonctionnalités réglaient les leurs chacune dans son onglet, avec des
//! formulations qui avaient divergé — « Tester le son de l'alerte » d'un côté, « Fermeture
//! automatique » de l'autre. Les quatre sections de « Paramètres » disent maintenant la même
//! chose de la même façon, fonctionnalité par fonctionnalité : ce que l'overlay fait ENTENDRE et
//! ce qu'il fait VOIR se règle en un seul endroit, et les onglets « Suivi », « Alertes » et
//! « Chat » ne gardent que ce qu'ils listent — objets suivis, objets à alerte, recherches.
//!
//! ## Couper le son, ce n'est pas couper la fonctionnalité
//!
//! Cochée, la sourdine fait taire l'alerte mais **la carte s'affiche toujours** par-dessus le jeu
//! — c'est ce qui la distingue de la case « Activer … » qui ouvre l'onglet correspondant
//! ([`crate::panels::feature_switch`]) et qui, elle, coupe les deux canaux. Une fonctionnalité
//! éteinte grise sa section ici : ni son à essayer, ni carte à fermer.
//!
//! ## Pas de fond de ligne sous la fermeture automatique
//!
//! Le couple case + durée était posé sur un pavé arrondi plus sombre (`#26282b`, l'idiome des
//! lignes d'aptitude du jeu), hérité de la maquette d'« Alertes » où il tranchait sur le reste de
//! l'onglet. Retiré le 2026-09-15 à la demande de l'utilisateur : dans une section de
//! « Paramètres », ce pavé faisait de la durée le seul réglage encadré de la fenêtre, alors que
//! c'est une ligne d'option comme les autres.

use egui::{Color32, RichText, Vec2};
use overlay_engine::AlertProfile;

use crate::design::{self, InputSize};

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

/// Gris des unités — `#b8b9ba`, le gris unique du jeu (voir
/// `panels::options_modal::SECTION_TITLE_TEXT`).
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);

/// Hauteur d'une ligne de cette section — celle mesurée sur `interface-options-commandes.png`,
/// partagée par les trois (ou quatre) lignes qu'une section peut porter.
const ROW_HEIGHT: f32 = 39.0;

const BODY_FONT_SIZE: f32 = 15.0;

/// Largeur du champ de durée — celle de la maquette d'« Alertes », assez pour « 30 » comme pour
/// « 0,75 ».
const DURATION_FIELD_WIDTH: f32 = 52.0;

/// Écart entre un libellé et le contrôle posé à sa droite — la valeur du relevé de section
/// (`panels::options_modal::FIELD_TO_BROWSE_GAP` en pose la sœur à 10).
const CONTROL_GAP: f32 = 12.0;

/// Le libellé de la sourdine — le même que celui de la section « Combat », écrit une seule fois
/// ici et repris là-bas. **« des notifications », pas « de l'alerte »** (2026-09-15) : les quatre
/// sections de « Paramètres » parlent des mêmes objets, elles les nomment pareil.
pub const MUTE_LABEL: &str = "Couper le son des notifications";

/// Le libellé de la fermeture automatique — « des notifications » pour la même raison que
/// [`MUTE_LABEL`] : « Fermeture automatique » seul ne disait pas de quoi.
pub const AUTO_CLOSE_LABEL: &str = "Fermeture automatique des notifications";

/// Le libellé de la fermeture automatique **du Suivi** — « de décompte » en plus (2026-09-16).
///
/// La section « Suivi » est la seule à porter DEUX notifications de nature différente : la carte
/// d'un compteur arrivé à zéro, réglée par cette ligne, et — pour qui suit aussi des objets à
/// alerte — celle d'un ramassage, qui se règle dans « Alertes ». Le libellé générique des deux
/// autres sections laisserait croire que la ligne commande tout ce que le Suivi affiche ; il dit
/// donc ici exactement quelle carte se ferme.
pub const COUNTDOWN_AUTO_CLOSE_LABEL: &str = "Fermeture automatique des notifications de décompte";

/// **Ce qu'une carte d'alerte sait de sa fermeture** — le peu dont ce module a besoin, pour ne pas
/// dupliquer le bornage de la durée.
///
/// Deux types le portent et ne sont pas parents : [`AlertProfile`] (les alertes de ramassage, qui
/// vivent sur le COMPTE) et [`crate::panels::chat_tab::ChatToastSettings`] (la carte de chat, qui
/// vit dans la config LOCALE). Chacun borne déjà sa durée à sa façon ; ce trait lui délègue plutôt
/// que de refaire un `clamp` ici, qui aurait divergé au premier ajustement.
pub trait ToastClose {
    /// La carte ne se ferme qu'à la main.
    fn manual_close(&self) -> bool;
    fn set_manual_close(&mut self, manual: bool);
    /// Durée d'affichage en secondes, ignorée quand la fermeture est manuelle.
    fn duration_seconds(&self) -> f32;
    /// Pose une durée — **c'est l'implémentation qui la borne**.
    fn set_duration(&mut self, seconds: f32);
}

impl ToastClose for AlertProfile {
    fn manual_close(&self) -> bool {
        self.manual_close
    }
    fn set_manual_close(&mut self, manual: bool) {
        self.manual_close = manual;
    }
    fn duration_seconds(&self) -> f32 {
        self.duration_seconds
    }
    fn set_duration(&mut self, seconds: f32) {
        AlertProfile::set_duration(self, seconds);
    }
}

/// La fermeture automatique d'une section — la carte que la fonctionnalité pose par-dessus le
/// jeu, et le délai au bout duquel elle s'efface.
///
/// **Le Suivi en a une depuis le 2026-09-16** : la doc de ce module l'a longtemps dit absente,
/// « son alerte est un son et un bandeau permanent, pas un toast ». C'était faux pour le décompte
/// arrivé à zéro, qui affiche bel et bien une carte
/// (`panels::watchlist::WatchlistToastReason::Countdown`) — elle empruntait simplement la durée du
/// profil d'alertes de ramassage, faute de réglage à elle.
pub struct AutoClose<'a> {
    /// **Le brouillon est-il descendu du compte ?** Sinon la ligne se peint grisée plutôt que de
    /// disparaître : une ligne qui apparaît quelques centaines de millisecondes après l'ouverture
    /// déplacerait tout ce qui la suit sous le pointeur.
    pub available: bool,
    /// Les réglages de fermeture eux-mêmes — brouillon, comme le reste de la fenêtre.
    pub settings: &'a mut dyn ToastClose,
    /// Le libellé de la case — [`AUTO_CLOSE_LABEL`] pour les sections qui n'affichent qu'une
    /// sorte de carte, [`COUNTDOWN_AUTO_CLOSE_LABEL`] pour le Suivi (voir sa doc).
    pub label: &'a str,
    /// La durée **telle que tapée** — une chaîne, pas un nombre.
    ///
    /// **Bornée à la perte de focus, jamais à la frappe.** Une version antérieure la bornait à
    /// chaque frame, ce qui rendait le champ inutilisable : taper `0.75` donnait `0` → borné à
    /// `0.5` sous les doigts, puis `0.5.` → non parsable → `3.5`. Toute saisie décimale passe par
    /// un état transitoire non parsable ; l'écraser avant qu'elle soit finie interdit d'écrire la
    /// valeur voulue.
    pub input: &'a mut String,
}

/// Ce qu'une section de notifications règle — voir [`section`].
pub struct Section<'a> {
    /// Nomme la fonctionnalité dans le journal d'interaction (`"suivi"`, `"alertes"`, `"chat"`) :
    /// tous les contrôles en dérivent leur `log_name`, pour qu'une trace dise toujours de quelle
    /// section vient le clic.
    pub log_prefix: &'a str,
    /// La fonctionnalité est-elle allumée (`panels::feature_switch`) ? Toute la section est grisée
    /// sinon — il n'y a ni son à couper ni carte à fermer quand rien ne se déclenche.
    pub enabled: bool,
    /// La sourdine, pour les fonctionnalités qui en ont une (Suivi et Chat — voir [`AlertMutes`]).
    pub muted: Option<&'a mut bool>,
    /// La fermeture automatique de la carte, pour les fonctionnalités qui en affichent une —
    /// le Suivi, les Alertes et le Chat (toutes, depuis le 2026-09-16 ; voir
    /// [`COUNTDOWN_AUTO_CLOSE_LABEL`]).
    pub auto_close: Option<AutoClose<'a>>,
}

/// Peint le contenu d'une section de notifications — **sans son titre**, que l'appelant pose
/// (`design::heading`) comme il pose celui de « Combat ».
///
/// Ne renvoie rien : tout ce que la section règle est un brouillon, que « Valider » emporte.
pub fn section(ui: &mut egui::Ui, width: f32, spec: Section<'_>) {
    let Section {
        log_prefix,
        enabled,
        muted,
        auto_close,
    } = spec;
    ui.scope(|ui| {
        // Voir `panels::feature_switch` : `disable` retire l'interaction ET pose le fondu, pour ce
        // `Ui` et pour tous les enfants qu'il créera — donc pour toute la section, sans que chaque
        // ligne ait à s'en occuper. Le titre, lui, reste vif : il est posé hors de ce scope.
        if !enabled {
            ui.disable();
        }
        if let Some(muted) = muted {
            mute_row(ui, width, log_prefix, muted);
        }
        if let Some(auto_close) = auto_close {
            close_row(ui, width, log_prefix, auto_close);
        }
    });
}

/// « Couper le son des notifications ».
fn mute_row(ui: &mut egui::Ui, width: f32, log_prefix: &str, muted: &mut bool) {
    let row = ui.allocate_space(Vec2::new(width, ROW_HEIGHT)).1;
    let mut cell = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row)
            .id_salt((log_prefix, "sourdine")),
    );
    cell.horizontal_centered(|ui| {
        ui.add(
            design::checkbox(muted, MUTE_LABEL)
                .tooltip(
                    "Coché, l'alerte ne joue plus aucun son — la carte, elle, continue de \
                     s'afficher par-dessus le jeu.",
                )
                .log_name(format!("{log_prefix}.sans-son")),
        );
    });
}

/// « Fermeture automatique des notifications » et sa durée — **sans fond de ligne**, voir la doc
/// de module.
fn close_row(ui: &mut egui::Ui, width: f32, log_prefix: &str, auto_close: AutoClose<'_>) {
    let AutoClose {
        available,
        label,
        settings,
        input,
    } = auto_close;

    let row = ui.allocate_space(Vec2::new(width, ROW_HEIGHT)).1;
    // **Un `id_salt` NOMMÉ, jamais l'identifiant automatique** — chaque section nomme sa rangée,
    // pour que l'identité du champ de durée ne dépende pas du compteur du `Ui` parent. La raison
    // complète, et le symptôme qui l'a révélée, sont dans `design::components::input` : un champ
    // qui hérite de l'état d'un autre se peint VIDE.
    let mut cell = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row)
            .id_salt((log_prefix, "fermeture")),
    );
    if !available {
        cell.disable();
    }
    // La case dit « fermeture AUTOMATIQUE », les réglages stockent son contraire (`manual_close`,
    // le nom du champ web). La négation vit ici, au plus près de la case, plutôt que dans le
    // moteur où elle rendrait le miroir du web illisible.
    let mut auto = !settings.manual_close();
    cell.horizontal_centered(|ui| {
        if ui
            .add(
                design::checkbox(&mut auto, label)
                    .tooltip(
                        "Décoché, la carte reste à l'écran jusqu'à ce que vous la fermiez \
                         vous-même.",
                    )
                    .log_name(format!("{log_prefix}.auto")),
            )
            .clicked()
        {
            settings.set_manual_close(!auto);
        }
        ui.add_space(CONTROL_GAP);
        // **Le champ suit la case** : décochée, la fermeture est manuelle, il n'y a plus de délai
        // et la valeur n'a plus d'effet — le champ est grisé et non modifiable.
        let response = ui.add(
            design::input(input)
                .size(InputSize::Standard)
                .width(DURATION_FIELD_WIDTH)
                .enabled(auto)
                .log_name(format!("{log_prefix}.duree")),
        );
        // La borne se pose à la PERTE DE FOCUS, pas à la frappe — voir `AutoClose::input`. C'est
        // le moment où la saisie est finie, et le seul où corriger « 0 » en « 0,5 » n'empêche pas
        // d'écrire « 0,75 ».
        if response.lost_focus() {
            settings.set_duration(parse_duration(input, settings.duration_seconds()));
            *input = format_duration(settings.duration_seconds());
        }
        ui.label(RichText::new("sec.").color(SUBDUED).size(BODY_FONT_SIZE));
    });
}

/// Lit une durée tapée — virgule décimale comprise.
///
/// **La virgule est le séparateur décimal d'un clavier français**, et ce champ est rempli en jeu,
/// au pavé numérique. La refuser renverrait la valeur de repli sur une saisie parfaitement
/// légitime.
///
/// Une saisie vide ou illisible garde la valeur en place plutôt que de retomber sur le défaut :
/// vider un champ par mégarde ne doit pas réécrire un réglage.
pub fn parse_duration(raw: &str, actuelle: f32) -> f32 {
    raw.trim()
        .replace(',', ".")
        .parse::<f32>()
        .unwrap_or(actuelle)
}

/// Écrit une durée dans le champ — sans décimale inutile (« 4 » plutôt que « 4.0 »), et avec la
/// virgule française qu'on vient d'accepter en entrée.
pub fn format_duration(seconds: f32) -> String {
    if seconds.fract().abs() < f32::EPSILON {
        format!("{}", seconds as i64)
    } else {
        format!("{seconds:.1}").replace('.', ",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_duree_se_tape_a_la_virgule_comme_au_point() {
        assert_eq!(parse_duration("2,5", 3.5), 2.5);
        assert_eq!(parse_duration("2.5", 3.5), 2.5);
        assert_eq!(parse_duration(" 4 ", 3.5), 4.0);
    }

    #[test]
    fn une_saisie_vide_garde_la_valeur_en_place() {
        assert_eq!(parse_duration("", 3.5), 3.5);
        assert_eq!(parse_duration("abc", 3.5), 3.5);
    }

    #[test]
    fn une_duree_entiere_s_ecrit_sans_decimale() {
        assert_eq!(format_duration(4.0), "4");
        assert_eq!(format_duration(3.5), "3,5");
    }

    #[test]
    fn le_profil_d_alerte_borne_la_duree_qu_on_lui_pose() {
        let mut profile = AlertProfile::default();
        ToastClose::set_duration(&mut profile, 0.1);
        assert_eq!(profile.duration_seconds(), 0.5);
        ToastClose::set_duration(&mut profile, 120.0);
        assert_eq!(profile.duration_seconds(), 30.0);
        profile.set_manual_close(true);
        assert!(profile.manual_close());
    }
}
