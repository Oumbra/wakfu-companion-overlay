//! Modale "Options" du design system (`docs/design-system.md` §9, mesuré le 2026-09-08 sur
//! `assets/design-system/interfaces/interface-options-*.png`) — voir §5.1 du plan d'architecture :
//! premier (et pour l'instant seul) réglage exposé, le chemin de `wakfu.log` à suivre. Ouverte par
//! le bouton "Options" du carré de contrôle (`panels::watchlist::control_button_row`) ou le
//! raccourci global `Ctrl+Shift+O` (voir `main.rs`/`bin/overlay-ui-x11.rs`).
//!
//! Chrome fidèle à la référence réelle : bannière turquoise dégradée (coins SUPÉRIEURS
//! chanfreinés), corps anthracite, pied de page plein-largeur scindé Annuler (rouge, nouveau token
//! `accent_danger`)/Valider (or, coins INFÉRIEURS chanfreinés) — voir `panels::chamfer` pour le
//! dessin des polygones. Pas de ligne d'onglets (Jeu/Vidéo/Interface/Son/Commandes/Chat existent
//! dans le jeu réel mais n'ont encore aucun équivalent overlay, voir doc §9 du design-system) ni de
//! croix de fermeture (Annuler/Valider en tiennent lieu, comme la référence réelle).
//!
//! **Pas de validation filesystem ICI** : cette fonction ne fait que peindre et renvoyer l'INTENTION
//! de l'utilisateur (`OptionsModalAction`) — c'est l'appelant (`main.rs`/`bin/overlay-ui-x11.rs`,
//! qui seuls savent comment déclencher un dialogue de fichier natif et parler au thread Engine) qui
//! valide via `overlay_ingest::discovery::validate_log_path` et alimente [`OptionsModalState::error`]
//! en retour pour le prochain redessin.

use crate::panels::chamfer::{self, ChamferCorners};

/// Taille de la fenêtre OS dédiée à cette modale (voir `main.rs::create_overlay_window`, nouveau
/// cas `OverlayKind::Options`) — assez large pour le champ de chemin + le bouton "Sélectionner le
/// fichier" côte à côte sans compresser le texte, assez haute pour bannière + un seul réglage +
/// pied de page sans vide excessif.
pub const WINDOW_SIZE: (f32, f32) = (560.0, 230.0);

const BANNER_HEIGHT: f32 = 54.0;
const FOOTER_HEIGHT: f32 = 46.0;
const CONTENT_MARGIN: f32 = 24.0;
const CHAMFER: f32 = 10.0;

// Voir docs/design-tokens.json (`banner_teal`, révisé 2026-09-08 — §9 du design-system.md).
const BANNER_TOP: egui::Color32 = egui::Color32::from_rgb(0x1A, 0x6E, 0x80);
const BANNER_BOTTOM: egui::Color32 = egui::Color32::from_rgb(0x17, 0x63, 0x72);
const BANNER_BORDER: egui::Color32 = egui::Color32::from_rgb(0x10, 0x12, 0x15);
const TITLE_TEXT: egui::Color32 = egui::Color32::WHITE;

// `panel_fill` (docs/design-tokens.json::neutrals).
const BODY_FILL: egui::Color32 = egui::Color32::from_rgb(0x18, 0x18, 0x20);
const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(0xF0, 0xF0, 0xF0);

// `accent_danger` (docs/design-tokens.json, nouveau 2026-09-08).
const CANCEL_TOP: egui::Color32 = egui::Color32::from_rgb(0xC9, 0x52, 0x4A);
const CANCEL_BOTTOM: egui::Color32 = egui::Color32::from_rgb(0xAF, 0x43, 0x3C);
const CANCEL_BORDER: egui::Color32 = egui::Color32::from_rgb(0x0F, 0x11, 0x14);
const CANCEL_TEXT: egui::Color32 = egui::Color32::WHITE;

// `accent_warm.gold_bright_*`/`button_text_on_gold` (docs/design-tokens.json).
const VALIDATE_TOP: egui::Color32 = egui::Color32::from_rgb(0xEA, 0xD8, 0x93);
const VALIDATE_BOTTOM: egui::Color32 = egui::Color32::from_rgb(0xE0, 0xC3, 0x75);
const VALIDATE_BORDER: egui::Color32 = egui::Color32::from_rgb(0x0E, 0x10, 0x13);
const VALIDATE_TEXT: egui::Color32 = egui::Color32::from_rgb(0x3A, 0x35, 0x23);

// Bouton secondaire kaki (§5.2 du design-system) — "Sélectionner le fichier".
const BROWSE_TOP: egui::Color32 = egui::Color32::from_rgb(0x84, 0x78, 0x5E);
const BROWSE_BOTTOM: egui::Color32 = egui::Color32::from_rgb(0x60, 0x58, 0x48);
const BROWSE_BORDER: egui::Color32 = egui::Color32::from_rgb(0x10, 0x10, 0x10);
const BROWSE_TEXT: egui::Color32 = egui::Color32::WHITE;

// Champ de saisie (§5.4 du design-system) — bordure chaude systématique de tous les inputs.
const FIELD_FILL: egui::Color32 = egui::Color32::from_rgb(0x1C, 0x1E, 0x23);
const FIELD_BORDER: egui::Color32 = egui::Color32::from_rgb(0x59, 0x51, 0x40);

const ERROR_TEXT: egui::Color32 = egui::Color32::from_rgb(0xE0, 0x60, 0x55);

/// État mutable de la modale, propriété de la fenêtre OS qui l'affiche (voir
/// `main.rs`/`bin/overlay-ui-x11.rs`, nouveau champ `OverlayWindow` réservé au cas
/// `OverlayKind::Options`) — persiste d'une frame à l'autre, contrairement à [`OptionsModalAction`]
/// qui ne vit que le temps d'un `show`.
#[derive(Debug, Default, Clone)]
pub struct OptionsModalState {
    /// Contenu ACTUEL du champ texte — initialisé au chemin actif au moment de l'ouverture de la
    /// modale (voir l'appelant), modifié librement par la frappe ou par [`OptionsModalAction::Browse`]
    /// une fois le dialogue natif résolu. Rien n'est pris en compte tant que
    /// [`OptionsModalAction::Validate`] n'a pas été renvoyé ET jugé valide par l'appelant (§5.1 du
    /// plan : « tant que l'utilisateur n'a pas validé son choix, rien n'est pris en compte »).
    pub path_input: String,
    /// Message d'erreur de la DERNIÈRE tentative de validation (`Browse` sur un fichier mal nommé,
    /// ou clic sur "Valider" avec un chemin invalide) — `None` tant qu'aucune tentative n'a encore
    /// échoué. Vidé par l'appelant dès qu'une nouvelle tentative commence.
    pub error: Option<String>,
}

/// Ce que l'utilisateur vient de demander CETTE frame — `None` la plupart du temps (aucun bouton
/// cliqué). Voir doc de module : ne porte aucune garantie de validité, c'est à l'appelant de
/// vérifier avant d'agir.
#[derive(Debug, Default)]
pub enum OptionsModalAction {
    #[default]
    None,
    Cancel,
    /// Ouvrir l'explorateur de fichiers natif — l'appelant seul sait le faire (`rfd`, sur un thread
    /// dédié pour ne jamais geler le rendu, voir sa doc dans `main.rs`).
    Browse,
    /// Chemin brut tel que tapé/affiché dans le champ au moment du clic — PAS encore un `PathBuf`
    /// validé, voir doc de module.
    Validate(String),
}

/// Peint la modale dans TOUT le rectangle disponible de `ui` (fenêtre OS dédiée, voir doc de
/// module) et renvoie l'action déclenchée par cette frame, le cas échéant.
pub fn show(ui: &mut egui::Ui, state: &mut OptionsModalState) -> OptionsModalAction {
    let mut action = OptionsModalAction::None;
    let rect = ui.max_rect();
    let painter = ui.painter();

    painter.rect_filled(rect, 0.0, BODY_FILL);

    let banner_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), BANNER_HEIGHT));
    chamfer::chamfered_rect(
        painter,
        banner_rect,
        CHAMFER,
        ChamferCorners::TOP,
        BANNER_TOP,
        BANNER_BOTTOM,
        Some(BANNER_BORDER),
    );
    painter.text(
        banner_rect.center(),
        egui::Align2::CENTER_CENTER,
        "Options",
        egui::FontId::proportional(20.0),
        TITLE_TEXT,
    );

    let footer_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.bottom() - FOOTER_HEIGHT),
        egui::vec2(rect.width(), FOOTER_HEIGHT),
    );
    let half = footer_rect.width() / 2.0;
    let cancel_rect = egui::Rect::from_min_size(footer_rect.min, egui::vec2(half, FOOTER_HEIGHT));
    let validate_rect = egui::Rect::from_min_size(
        footer_rect.min + egui::vec2(half, 0.0),
        egui::vec2(footer_rect.width() - half, FOOTER_HEIGHT),
    );
    chamfer::chamfered_rect(
        painter,
        cancel_rect,
        CHAMFER,
        ChamferCorners::BOTTOM_LEFT,
        CANCEL_TOP,
        CANCEL_BOTTOM,
        Some(CANCEL_BORDER),
    );
    chamfer::chamfered_rect(
        painter,
        validate_rect,
        CHAMFER,
        ChamferCorners::BOTTOM_RIGHT,
        VALIDATE_TOP,
        VALIDATE_BOTTOM,
        Some(VALIDATE_BORDER),
    );
    painter.text(
        cancel_rect.center(),
        egui::Align2::CENTER_CENTER,
        "Annuler",
        egui::FontId::proportional(16.0),
        CANCEL_TEXT,
    );
    painter.text(
        validate_rect.center(),
        egui::Align2::CENTER_CENTER,
        "Valider",
        egui::FontId::proportional(16.0),
        VALIDATE_TEXT,
    );
    let cancel_response = ui
        .interact(
            cancel_rect,
            ui.id().with("options-cancel"),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let validate_response = ui
        .interact(
            validate_rect,
            ui.id().with("options-validate"),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if cancel_response.clicked() {
        action = OptionsModalAction::Cancel;
    }
    if validate_response.clicked() {
        action = OptionsModalAction::Validate(state.path_input.clone());
    }

    // Corps : label + champ de chemin + bouton "Sélectionner le fichier" (§5.1 du plan — renommé
    // depuis "Ouvrir", voir la référence `interface-options-interface.png`, boutons "Ouvrir le
    // dossier"/"Ouvrir le thème" du même style secondaire kaki).
    let content_rect = egui::Rect::from_min_max(
        rect.min + egui::vec2(CONTENT_MARGIN, BANNER_HEIGHT + CONTENT_MARGIN),
        egui::pos2(rect.right() - CONTENT_MARGIN, footer_rect.top() - 12.0),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
        ui.label(
            egui::RichText::new("Fichier wakfu.log")
                .color(TEXT_PRIMARY)
                .size(15.0)
                .strong(),
        );
        ui.add_space(8.0);

        const BROWSE_BUTTON_WIDTH: f32 = 190.0;
        const FIELD_HEIGHT: f32 = 34.0;
        ui.horizontal(|ui| {
            let field_width = (content_rect.width() - BROWSE_BUTTON_WIDTH - 10.0).max(80.0);
            let field_rect = ui.allocate_space(egui::vec2(field_width, FIELD_HEIGHT)).1;
            ui.painter().rect_filled(field_rect, 0.0, FIELD_FILL);
            ui.painter().rect_stroke(
                field_rect,
                0.0,
                egui::Stroke::new(1.0, FIELD_BORDER),
                egui::StrokeKind::Inside,
            );
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(field_rect.shrink(6.0)),
                |ui| {
                    ui.centered_and_justified(|ui| {
                        let edit = egui::TextEdit::singleline(&mut state.path_input)
                            .frame(egui::Frame::NONE)
                            .text_color(TEXT_PRIMARY)
                            .hint_text("Chemin vers wakfu.log");
                        ui.add(edit);
                    });
                },
            );

            ui.add_space(10.0);

            let browse_rect = ui
                .allocate_space(egui::vec2(BROWSE_BUTTON_WIDTH, FIELD_HEIGHT))
                .1;
            chamfer::chamfered_rect(
                ui.painter(),
                browse_rect,
                6.0,
                ChamferCorners::default(),
                BROWSE_TOP,
                BROWSE_BOTTOM,
                Some(BROWSE_BORDER),
            );
            ui.painter().text(
                browse_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Sélectionner le fichier",
                egui::FontId::proportional(13.0),
                BROWSE_TEXT,
            );
            let browse_response = ui
                .interact(
                    browse_rect,
                    ui.id().with("options-browse"),
                    egui::Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if browse_response.clicked() {
                action = OptionsModalAction::Browse;
            }
        });

        if let Some(err) = &state.error {
            ui.add_space(10.0);
            ui.label(egui::RichText::new(err).color(ERROR_TEXT).size(13.0));
        }
    });

    action
}
