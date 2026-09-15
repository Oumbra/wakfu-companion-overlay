//! **L'interrupteur d'une fonctionnalité** — la case « Activer … » qui ouvre les onglets « Suivi »,
//! « Alertes » et « Chat » de la fenêtre Options, et qui commande tout ce qui la suit dans son
//! onglet (2026-09-15).
//!
//! Demande utilisateur : « permettre de désactiver les features Suivi, Alertes, Chat, via une
//! option tout en haut, après le titre […] activée par défaut ; lorsqu'elle est désactivée, tout
//! le contenu devient grisé et désactivé, impossible d'interagir avec ».
//!
//! ## Trois onglets, un seul endroit
//!
//! Les trois cases sont rigoureusement la même chose : même place (sous le titre et sa phrase,
//! avant le premier réglage), même aération, même façon de griser la suite. Trois copies de ces
//! quelques lignes auraient divergé au premier ajustement — c'est ce module qui les tient
//! ensemble, comme `panels::tile_reorder` tient le glisser-déposer partagé par le Suivi et le
//! bandeau.
//!
//! ## Griser, c'est `Ui::disable`, pas un voile peint
//!
//! [`egui::Ui::disable`] fait les deux moitiés du travail d'un seul geste : il retire
//! l'interaction (plus un clic, plus un survol, plus une infobulle) **et** multiplie l'opacité du
//! `Painter` par `Visuals::disabled_alpha` (50 % chez egui), dont tous les enfants créés ensuite
//! héritent. C'est ce qui permet de griser un onglet entier sans toucher à son code : la case
//! appelle `disable` sur le `Ui` de l'onglet, et tout ce qui est peint après — y compris les
//! tuiles peintes à la main au `Painter`, y compris la zone de défilement et ses sous-`Ui`, qui
//! héritent tous du `Painter` de leur parent — est estompé et inerte.
//!
//! Deux chemins écartés : un voile peint par-dessus aurait grisé l'apparence en laissant les clics
//! passer (l'inverse exact de la demande), et un `enabled(false)` posé sur chaque composant aurait
//! demandé de retoucher les trois onglets widget par widget, en en oubliant à chaque ajout.
//!
//! La case elle-même, et le titre et la phrase qui la précèdent, restent évidemment vifs : c'est
//! par eux qu'on rallume la fonctionnalité.

use crate::design;

/// **Les trois interrupteurs, ensemble** — Suivi, Alertes, Recherche de chat.
///
/// Un seul type plutôt que trois `bool` baladeurs, pour deux raisons : ils voyagent toujours
/// ensemble (config persistée → fenêtre Options → hôte → thread Engine), et surtout leur défaut
/// est `true`. Trois champs `bool` dans une structure `#[derive(Default)]` vaudraient `false`,
/// c'est-à-dire « tout coupé » — le contraire de ce que doit faire une version neuve. Ici le
/// défaut est écrit une fois, et tout ce qui dérive `Default` au-dessus l'hérite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureToggles {
    /// Le bandeau de suivi et l'alerte de décompte à zéro.
    pub suivi: bool,
    /// Le son et la carte au ramassage d'un objet à son activé.
    pub alerts: bool,
    /// Le son et la carte quand un message du chat correspond à une recherche.
    pub chat: bool,
}

impl Default for FeatureToggles {
    /// **Tout est actif** : voir la doc du type — c'est l'état d'une installation neuve, et celui
    /// d'un `config.toml` écrit avant l'existence de ces réglages.
    fn default() -> Self {
        Self {
            suivi: true,
            alerts: true,
            chat: true,
        }
    }
}

/// Aération entre la case d'activation et le premier bloc de l'onglet.
///
/// **Le même écart que celui qui suit un titre de section** (18 px, voir `SECTION_GAP` dans les
/// trois onglets) : la case ouvre son onglet comme un titre ouvre une section, et un écart plus
/// serré la collerait au premier réglage — précisément celui qu'elle commande, donc celui dont
/// elle doit se distinguer.
const GAP_AFTER: f32 = 18.0;

/// Peint la case « Activer … » et **grise tout ce qui sera peint ensuite dans `ui`** quand la
/// fonctionnalité est coupée.
///
/// À appeler juste après le titre de l'onglet et sa phrase de description, avant le premier
/// réglage — voir la doc de module. `enabled` est un brouillon comme le reste de la fenêtre : la
/// bascule ne prend effet qu'à « Valider » (voir `panels::options_modal::OptionsCommit`), son
/// retour n'a donc rien à déclencher ici et n'est pas rendu.
pub fn show(ui: &mut egui::Ui, enabled: &mut bool, label: &str, tooltip: &str, log_name: &str) {
    ui.add(
        design::checkbox(enabled, label)
            .tooltip(tooltip)
            .log_name(log_name),
    );
    ui.add_space(GAP_AFTER);
    if !*enabled {
        // Voir la doc de module : retire l'interaction ET pose le fondu, pour ce `Ui` et pour tous
        // les enfants qu'il créera — c'est-à-dire pour tout le reste de l'onglet.
        ui.disable();
    }
}
