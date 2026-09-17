//! **Onglet « À propos » de la fenêtre Options** — ce qui concerne le programme lui-même, et non
//! ce qu'il affiche : sa mise à jour, son redémarrage, son arrêt.
//!
//! Créé le 2026-09-18, en **dernière** position du menu (demande utilisateur : « ajouter l'onglet
//! "À propos" en dernier et y déplacer la section Mise à jour, les boutons "Redémarrer" et "Fermer
//! l'overlay" »). Les trois vivaient jusque-là au pied de l'onglet « Paramètres », après « Compte »
//! — un onglet qui avait grossi jusqu'à huit sections défilantes, où les sorties du programme se
//! trouvaient sous les réglages de notifications, à un défilement de distance. Ni la mise à jour
//! ni les sorties ne sont des réglages : elles ne passent pas par « Valider », à l'exception de la
//! case d'installation automatique, brouillon comme les autres (voir
//! `panels::options_modal::OptionsModalState::auto_update`).
//!
//! La version courante n'est **pas** rappelée ici : la bannière de la fenêtre la porte déjà, à
//! gauche (`design::window`, `.version(true)`), et c'est le seul endroit où elle a à l'être.
//!
//! ## Ce que l'onglet ne fait pas lui-même
//!
//! Il ne confirme rien et n'agit sur rien : il **remonte une intention** ([`AProposTabAction`]) à
//! la fenêtre Options, qui ouvre la confirmation qui convient (installation, redémarrage, arrêt —
//! toutes en `design::confirm_dialog` sur la fenêtre entière) et, sur « Oui », passe l'action à
//! l'hôte. Les trois s'excluent par construction chez l'appelant ; l'onglet n'en produit qu'une
//! par frame.

use overlay_sync::update::{self, UpdateStatus};

use crate::design::{self, ButtonSize, ButtonVariant, PanelZones};

/// Hauteur d'une ligne de contrôle — la même que dans `panels::options_modal` (36 px, relevé).
const ROW_HEIGHT: f32 = 36.0;
/// Écart entre la ligne d'information et la case qui la suit — voir
/// `panels::options_modal::INFO_GAP`.
const INFO_GAP: f32 = 9.0;
/// Air entre la fin d'une section et ce qui suit — voir `panels::options_modal::SECTION_GAP`.
const SECTION_GAP: f32 = 17.0;

/// Ce que l'onglet reçoit de la fenêtre Options.
pub struct AProposTabContext<'a> {
    /// Où en est la mise à jour automatique — copié par l'hôte avant chaque rendu, jamais figé à
    /// l'ouverture (voir `OptionsModalState::update`).
    pub update: &'a UpdateStatus,
    /// La case « Installer automatiquement les mises à jour au démarrage » — un brouillon, que
    /// « Valider » écrit et qu'« Annuler » abandonne.
    pub auto_update: &'a mut bool,
}

/// L'intention que l'onglet remonte à la fenêtre Options, au plus une par frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AProposTabAction {
    None,
    /// « Recherche de mise à jour » / « Réessayer » : lire la dernière version publiée, sans rien
    /// installer.
    CheckUpdate,
    /// « Mettre à jour vers X » : demander l'installation de cette version — à confirmer, elle
    /// ferme l'overlay.
    Install(String),
    /// « Redémarrer l'overlay » — à confirmer.
    Restart,
    /// « Fermer l'overlay » — à confirmer.
    Quit,
}

pub fn show(
    ui: &mut egui::Ui,
    panel: &PanelZones,
    ctx: &mut AProposTabContext<'_>,
) -> AProposTabAction {
    let mut action = AProposTabAction::None;
    let inner_width = panel.inner.width();

    // **Section « Mise à jour »** (2026-09-15, `docs/plan-mise-a-jour.md` §8.2, décisions du
    // mainteneur) : une ligne d'information (dernière vérification, version disponible et son
    // poids), la case d'installation automatique, et UN bouton dont le libellé suit l'état :
    // « Recherche de mise à jour » → « Recherche… » → « Mettre à jour vers X » / « Réessayer ».
    // Pas de bouton « Notes de version » pour l'instant (aucune note n'est rédigée aujourd'hui).
    //
    // « Mettre à jour » n'est PAS un brouillon : il ferme l'overlay de jeu le temps de
    // l'installation — d'où sa confirmation, chez l'appelant. « Recherche », lui, ne touche à rien.
    ui.add(design::heading("Mise à jour"));
    let (info, tone) = update_info_line(ctx.update, std::time::Instant::now());
    ui.add(
        design::info_text(info)
            .tone(tone)
            .width(inner_width)
            .log_name("options-mise-a-jour-info"),
    );
    ui.add_space(INFO_GAP);
    ui.add(
        design::checkbox(
            ctx.auto_update,
            "Installer automatiquement les mises à jour au démarrage",
        )
        .tooltip(
            "Au lancement, une version plus récente est téléchargée et installée avant \
             d'ouvrir l'overlay. Décochée, elle est seulement signalée ici.",
        )
        .log_name("options-mise-a-jour-auto"),
    );
    ui.add_space(design::tokens::CHECKBOX_ROW_GAP);
    let button_spec = update_button(ctx.update);
    let update_button = design::button(button_spec.label)
        .variant(button_spec.variant)
        .size(ButtonSize::Height(ROW_HEIGHT))
        .enabled(button_spec.enabled)
        .tooltip(button_spec.tooltip)
        .log_name("options-mise-a-jour-bouton");
    let update_size = update_button.desired_size(ui);
    let row = ui.allocate_space(egui::vec2(inner_width, ROW_HEIGHT)).1;
    if ui
        .put(
            egui::Rect::from_center_size(row.center(), update_size),
            update_button,
        )
        .clicked()
    {
        action = match ctx.update {
            UpdateStatus::Available { version, .. } => AProposTabAction::Install(version.clone()),
            _ => AProposTabAction::CheckUpdate,
        };
    }

    // **« Redémarrer l'overlay » et « Fermer l'overlay »** (2026-09-16 pour la sortie, 2026-09-17
    // pour le redémarrage, ici depuis le 2026-09-18), sans section : ce ne sont pas des réglages,
    // ce sont les sorties. Jusqu'au 2026-09-16, quitter demandait le raccourci « Quitter » ou
    // l'icône de la zone de notification — deux chemins qu'un joueur qui a la fenêtre Options sous
    // les yeux ne voit pas ; et relancer demandait de faire les deux à la suite, à la main.
    //
    // **Secondaires et centrés** (demande utilisateur) : centrés comme « Se déconnecter » dans
    // l'onglet « Paramètres », parce que ce sont des actions qui échappent à « Annuler » ;
    // secondaires et non `Danger`, parce que ni l'une ni l'autre ne détruit quoi que ce soit — le
    // compte reste appairé, les réglages validés restent écrits, relancer retrouve tout. Les
    // confirmations, elles, restent : un clic en plein combat coupe le détail des dégâts sans
    // retour, et le brouillon de la fenêtre part avec.
    //
    // **« Redémarrer l'overlay », en toutes lettres** (2026-09-18, en même temps que le
    // déplacement) : il disait « Redémarrer » tant qu'il était collé à « Fermer l'overlay », qui
    // lui prêtait son complément. Les deux libellés se répondent maintenant mot pour mot.
    //
    // **La paire est centrée, pas chaque bouton** : les deux largeurs naturelles et la gouttière
    // du pied de page (`WINDOW_FOOTER_GUTTER`, la seule gouttière bouton-à-bouton relevée dans le
    // jeu) forment un bloc, centré d'un seul tenant sur la colonne — centrer chacun dans une
    // moitié les éloignerait l'un de l'autre au gré de la largeur de la fenêtre, et
    // « Redémarrer l'overlay » ne se lirait plus comme la variante de son voisin. Il est à GAUCHE :
    // on lit l'action la moins définitive en premier.
    ui.add_space(SECTION_GAP);
    let restart = design::button("Redémarrer l'overlay")
        .variant(ButtonVariant::Secondary)
        .size(ButtonSize::Height(ROW_HEIGHT))
        .tooltip(
            "Arrêter puis relancer l'overlay. Utile après avoir changé de fichier de \
             journal ou quand l'affichage ne suit plus le jeu.",
        )
        .log_name("options-redemarrer-overlay");
    let quit = design::button("Fermer l'overlay")
        .variant(ButtonVariant::Secondary)
        .size(ButtonSize::Height(ROW_HEIGHT))
        .tooltip(
            "Arrêter l'overlay. Le compte reste appairé et les réglages validés sont \
             conservés pour la prochaine fois.",
        )
        .log_name("options-fermer-overlay");
    let restart_size = restart.desired_size(ui);
    let quit_size = quit.desired_size(ui);
    let gutter = design::tokens::WINDOW_FOOTER_GUTTER;
    let row = ui.allocate_space(egui::vec2(inner_width, ROW_HEIGHT)).1;
    let paire_gauche = row.center().x - (restart_size.x + gutter + quit_size.x) / 2.0;
    let restart_rect = egui::Rect::from_min_size(
        egui::pos2(paire_gauche, row.center().y - restart_size.y / 2.0),
        restart_size,
    );
    let quit_rect = egui::Rect::from_min_size(
        egui::pos2(
            restart_rect.right() + gutter,
            row.center().y - quit_size.y / 2.0,
        ),
        quit_size,
    );
    if ui.put(restart_rect, restart).clicked() {
        action = AProposTabAction::Restart;
    }
    if ui.put(quit_rect, quit).clicked() {
        action = AProposTabAction::Quit;
    }

    action
}

/// La ligne d'information de la section « Mise à jour » et son ton — une fonction libre, pour
/// que ses formulations soient testées sans peindre.
pub fn update_info_line(
    status: &UpdateStatus,
    now: std::time::Instant,
) -> (String, design::InfoTone) {
    let since = |at: std::time::Instant| {
        let secs = now.saturating_duration_since(at).as_secs();
        if secs < 60 {
            "à l'instant".to_string()
        } else if secs < 3600 {
            format!("il y a {} min", secs / 60)
        } else {
            format!("il y a {} h", secs / 3600)
        }
    };
    match status {
        UpdateStatus::Idle => (
            "Aucune vérification depuis le lancement.".to_string(),
            design::InfoTone::Info,
        ),
        UpdateStatus::Checking => ("Recherche en cours…".to_string(), design::InfoTone::Info),
        UpdateStatus::UpToDate { checked_at } => (
            format!(
                "Dernière vérification {} · vous êtes à jour.",
                since(*checked_at)
            ),
            design::InfoTone::Info,
        ),
        UpdateStatus::Available {
            version,
            download_size,
            mandatory,
            checked_at,
            ..
        } => (
            format!(
                "Dernière vérification {} · version {version} disponible · {}{}",
                since(*checked_at),
                update::human_size(*download_size),
                if *mandatory { " · obligatoire" } else { "" }
            ),
            design::InfoTone::Info,
        ),
        UpdateStatus::Downloading {
            version,
            received,
            total,
        } => (
            format!(
                "Téléchargement de la version {version} : {} / {}",
                update::human_size(*received),
                update::human_size(*total)
            ),
            design::InfoTone::Info,
        ),
        UpdateStatus::Verifying { version } => (
            format!("Vérification de la version {version}…"),
            design::InfoTone::Info,
        ),
        UpdateStatus::ReadyToInstall { version, .. } | UpdateStatus::Installing { version } => (
            format!("Installation de la version {version}…"),
            design::InfoTone::Info,
        ),
        UpdateStatus::Unavailable { reason, checked_at } => (
            format!(
                "Dernière vérification {} · impossible ({reason}).",
                since(*checked_at)
            ),
            design::InfoTone::Alert,
        ),
        UpdateStatus::Failed {
            headline, detail, ..
        } => (
            format!("Mise à jour impossible : {headline} ({detail})"),
            design::InfoTone::Alert,
        ),
    }
}

/// Le bouton unique de la section « Mise à jour », selon l'état.
pub struct UpdateButtonSpec {
    pub label: String,
    pub variant: ButtonVariant,
    pub enabled: bool,
    pub tooltip: &'static str,
}

pub fn update_button(status: &UpdateStatus) -> UpdateButtonSpec {
    match status {
        UpdateStatus::Available { version, .. } => UpdateButtonSpec {
            label: format!("Mettre à jour vers {version}"),
            variant: ButtonVariant::Primary,
            enabled: true,
            tooltip: "Ferme l'overlay, installe la nouvelle version et le relance",
        },
        UpdateStatus::Checking => UpdateButtonSpec {
            label: "Recherche…".to_string(),
            variant: ButtonVariant::Secondary,
            enabled: false,
            tooltip: "Lecture de la dernière version publiée",
        },
        UpdateStatus::Downloading { .. }
        | UpdateStatus::Verifying { .. }
        | UpdateStatus::ReadyToInstall { .. }
        | UpdateStatus::Installing { .. } => UpdateButtonSpec {
            label: "Mise à jour en cours…".to_string(),
            variant: ButtonVariant::Secondary,
            enabled: false,
            tooltip: "L'overlay se relancera une fois la version installée",
        },
        UpdateStatus::Failed { .. } => UpdateButtonSpec {
            label: "Réessayer".to_string(),
            variant: ButtonVariant::Secondary,
            enabled: true,
            tooltip: "Rechercher à nouveau la dernière version publiée",
        },
        UpdateStatus::Idle | UpdateStatus::UpToDate { .. } | UpdateStatus::Unavailable { .. } => {
            UpdateButtonSpec {
                label: "Recherche de mise à jour".to_string(),
                variant: ButtonVariant::Secondary,
                enabled: true,
                tooltip: "Lire la dernière version publiée, sans rien installer",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ligne_et_bouton_de_la_section_mise_a_jour_suivent_l_etat() {
        use overlay_sync::update::UpdateStatus;
        let now = std::time::Instant::now();
        let (ligne, _) = update_info_line(
            &UpdateStatus::UpToDate { checked_at: now },
            now + std::time::Duration::from_secs(185),
        );
        assert_eq!(
            ligne,
            "Dernière vérification il y a 3 min · vous êtes à jour."
        );
        assert_eq!(
            update_button(&UpdateStatus::Idle).label,
            "Recherche de mise à jour"
        );
        let disponible = UpdateStatus::Available {
            version: "0.21.0".into(),
            download_size: 3_100_000,
            mandatory: false,
            notes_url: None,
            checked_at: now,
        };
        let (ligne, tone) = update_info_line(&disponible, now);
        assert_eq!(
            ligne,
            "Dernière vérification à l'instant · version 0.21.0 disponible · 3,1 Mo"
        );
        assert_eq!(tone, design::InfoTone::Info);
        let bouton = update_button(&disponible);
        assert_eq!(bouton.label, "Mettre à jour vers 0.21.0");
        assert!(bouton.enabled);
        assert_eq!(bouton.variant, ButtonVariant::Primary);
        assert!(!update_button(&UpdateStatus::Checking).enabled);
        let (_, tone) = update_info_line(
            &UpdateStatus::Unavailable {
                reason: "hors ligne".into(),
                checked_at: now,
            },
            now,
        );
        assert_eq!(tone, design::InfoTone::Alert);
    }
}
