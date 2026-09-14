//! **Fenêtre de connexion** (2026-09-14, §9.1 undecies du plan) — la première interface de
//! l'overlay, et la seule tant qu'aucun compte n'est lié.
//!
//! Elle reproduit la maquette validée par l'utilisateur (direction « Fidèle au web », révisée en
//! quatre passes) : une carte de 400 px sur fond noir translucide, le logo du site, le titre
//! « WAKFU COMPANION » dans l'accent cyan du site suivi d'« OVERLAY » en italique gris, le badge
//! « beta » en haut à droite, un séparateur gravé (ligne sombre puis claire), un corps aligné à
//! gauche, et le numéro de version en pied, au style de la bannière de la fenêtre Options. Un
//! **anneau lumineux tourne en permanence** autour de la carte — gris translucide au repos, cyan
//! pendant l'appairage, rouge en erreur : la fenêtre est toujours vivante.
//!
//! Trois états, calqués sur [`AuthStatus`] :
//!
//! - **non connecté** (`Disconnected { failure: None }`, et `Connecting` avec « Connexion… » à la
//!   place du bouton) : « Vous n'êtes pas connecté » + « Se connecter » ;
//! - **appairage** (`PairingStarted`) : le code en grand, le compte à rebours, « Copier le code »,
//!   « Rouvrir la page », et le lien « Annuler l'appairage » ;
//! - **erreur** (`Disconnected { failure: Some(_) }`) : « Connexion impossible », le titre court de
//!   l'échec, le détail technique et « Réessayer ».
//!
//! **Palette du site, pas du jeu.** C'est un choix délibéré et validé : cette fenêtre n'est pas un
//! overlay posé sur le jeu mais une fenêtre logicielle classique, la porte d'entrée du compte
//! Wakfu Companion — elle reprend donc l'en-tête du site (`app-header.component`, accent
//! `#00d2ff`, badge *beta*, logo `logo-purple.png`) et ses boutons (`.btn-primary`), pas les
//! textures 9-slice de `crate::design`. Seule la version du pied reprend le style du jeu, comme
//! la bannière de la fenêtre Options.
//!
//! **Pas de mode invité.** Cette fenêtre n'a aucun bouton pour la contourner, et n'en aura jamais :
//! un compte est obligatoire (décision utilisateur, 2026-09-13/14).
//!
//! Comme tout panneau de ce crate, elle est **pure** : elle n'ouvre aucune page elle-même
//! (`LoginOutcome::open_url`), ne déplace pas sa fenêtre (`LoginOutcome::drag_window`) et ne
//! connaît pas sa fenêtre OS — l'hôte (`main.rs`) fait tout cela, et le harnais de captures peut
//! la rejouer sans effet de bord (§17.3 bis du plan).

use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::text::LayoutJob;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use crate::build_info;
use crate::design::text;
use crate::render_content::{AuthCommand, AuthCommandSink, AuthStatus};
use crate::ui_icons::UiIcons;

/// Largeur de la carte, et donc de la fenêtre OS (maquette : 400 px).
pub const WINDOW_WIDTH: f32 = 400.0;
/// Hauteur de départ de la fenêtre OS — celle de l'état « non connecté », le premier affiché. La
/// hauteur réelle est mesurée à chaque frame ([`LoginOutcome::content_height`]) et l'hôte ajuste
/// la fenêtre dès la première : cette valeur ne sert qu'à ne pas naître à une taille absurde.
pub const INITIAL_HEIGHT: f32 = 385.0;

// ── Palette (dépôt web : `styles.css`, `app-header.component.css`) ─────────────────────────────
const CARD_FILL: Color32 = Color32::from_rgba_premultiplied(6, 8, 11, 199); // rgba(8,10,14,.78)
const CARD_BORDER: Color32 = Color32::from_rgb(0x2a, 0x30, 0x38);
const CARD_BORDER_PAIRING: Color32 = Color32::from_rgba_premultiplied(0, 46, 56, 56); // cyan .22
const CARD_BORDER_ERROR: Color32 = Color32::from_rgba_premultiplied(81, 29, 26, 89); // rouge .35
const ACCENT: Color32 = Color32::from_rgb(0x00, 0xd2, 0xff);
const ACCENT_HOVER: Color32 = Color32::from_rgb(0x38, 0xdc, 0xff);
const ON_ACCENT: Color32 = Color32::from_rgb(0x04, 0x22, 0x2a);
const TEXT: Color32 = Color32::from_rgb(0xe8, 0xed, 0xf2);
const TEXT_MUTED: Color32 = Color32::from_rgb(0x9a, 0xa4, 0xb0);
const TEXT_DIM: Color32 = Color32::from_rgb(0x8a, 0x94, 0xa0);
const TITLE_OVERLAY: Color32 = Color32::from_rgb(0x6b, 0x75, 0x80);
const BETA: Color32 = Color32::from_rgb(0xb4, 0xbc, 0xc5);
const RULE_DARK: Color32 = Color32::from_rgb(0x0a, 0x0d, 0x11);
const RULE_LIGHT: Color32 = Color32::from_rgb(0x31, 0x3a, 0x45);
const SECONDARY_BORDER: Color32 = Color32::from_rgb(0x2a, 0x30, 0x38);
const SECONDARY_BORDER_HOVER: Color32 = Color32::from_rgb(0x3b, 0x44, 0x4f);
const SECONDARY_FILL_HOVER: Color32 = Color32::from_rgba_premultiplied(12, 12, 12, 13); // .05
const CODE_BORDER: Color32 = Color32::from_rgba_premultiplied(0, 59, 71, 71); // cyan .28
const CODE_FILL: Color32 = Color32::from_rgba_premultiplied(0, 11, 13, 13); // cyan .05
const ERROR_TEXT: Color32 = Color32::from_rgb(0xff, 0x8a, 0x83);
const ERROR_DOT: Color32 = Color32::from_rgb(0xe5, 0x53, 0x4b);
const ERROR_DOT_HALO: Color32 = Color32::from_rgba_premultiplied(41, 15, 13, 46); // .18
const DETAIL_FILL: Color32 = Color32::from_rgba_premultiplied(8, 8, 8, 8); // blanc .03
const VERSION: Color32 = BETA;

// ── Anneau animé ───────────────────────────────────────────────────────────────────────────────
/// Un tour complet en 3 s (`animation: v-web-ring 3s linear infinite`).
const RING_PERIOD: Duration = Duration::from_secs(3);
/// Épaisseur du liseré lumineux (`padding: 1.5px` du pseudo-élément).
const RING_WIDTH: f32 = 1.5;
/// Cadence de redessin de l'animation — ~30 images/s, comme les confettis du toast de suivi.
const ANIMATION_FRAME: Duration = Duration::from_millis(33);
/// Période de la pulsation des trois points d'attente (`v-web-pulse 1.4s`).
const DOTS_PERIOD: Duration = Duration::from_millis(1400);

// ── Gabarit (maquette v4, valeurs en px logiques) ──────────────────────────────────────────────
const CARD_RADIUS: f32 = 10.0;
const HEAD_PAD_TOP: f32 = 26.0;
const HEAD_PAD_SIDE: f32 = 18.0;
const LOGO_SIZE: f32 = 56.0;
const HEAD_GAP: f32 = 10.0;
const BETA_HEIGHT: f32 = 10.0;
const TITLE_SIZE: f32 = 16.0;
const TITLE_LINE: f32 = 20.0;
const TITLE_SPACING: f32 = 1.0;
const RULE_MARGIN_TOP: f32 = 14.0;
const BODY_PAD_TOP: f32 = 18.0;
const BODY_PAD_SIDE: f32 = 24.0;
const BODY_PAD_BOTTOM: f32 = 22.0;
const H2_SIZE: f32 = 17.0;
const H2_LINE: f32 = 21.0;
const H2_GAP: f32 = 8.0;
const P_SIZE: f32 = 13.0;
const P_LINE: f32 = 20.0;
const P_GAP: f32 = 6.0;
const ACTIONS_MARGIN_TOP: f32 = 22.0;
const ACTIONS_GAP: f32 = 10.0;
const BUTTON_HEIGHT: f32 = 40.0;
const BUTTON_RADIUS: f32 = 6.0;
const BUTTON_FONT: f32 = 14.0;
const LINK_MARGIN_TOP: f32 = 14.0;
const LINK_SIZE: f32 = 12.0;
const CODE_MARGIN_TOP: f32 = 18.0;
const CODE_PAD_TOP: f32 = 16.0;
const CODE_PAD_BOTTOM: f32 = 14.0;
const CODE_VALUE_SIZE: f32 = 30.0;
const CODE_VALUE_SPACING: f32 = 4.0;
const CODE_LABEL_GAP: f32 = 8.0;
const CODE_LABEL_SIZE: f32 = 11.0;
const WAIT_MARGIN_TOP: f32 = 14.0;
const WAIT_SIZE: f32 = 12.0;
const WAIT_DOT: f32 = 5.0;
const WAIT_DOT_GAP: f32 = 4.0;
const WAIT_TEXT_GAP: f32 = 10.0;
const STATUS_SIZE: f32 = 11.0;
const STATUS_DOT: f32 = 7.0;
const STATUS_GAP: f32 = 7.0;
const STATUS_MARGIN_BOTTOM: f32 = 10.0;
const DETAIL_MARGIN_TOP: f32 = 12.0;
const DETAIL_PAD_X: f32 = 10.0;
const DETAIL_PAD_Y: f32 = 8.0;
const DETAIL_SIZE: f32 = 11.0;
const FOOT_HEIGHT: f32 = 30.0;
const FOOT_PAD_SIDE: f32 = 12.0;
const VERSION_SIZE: f32 = 11.0;
/// Inclinaison de l'italique simulé (voir [`paint_italic`]) — proche des 12° d'une vraie italique.
const ITALIC_SHEAR: f32 = 0.2;

/// État par fenêtre de connexion — l'horloge de l'animation. Créé par l'hôte avec la fenêtre.
#[derive(Debug, Clone)]
pub struct LoginState {
    /// Origine de la phase de l'anneau et des points d'attente : `now - started_at` module la
    /// période. Un harnais qui passe `now == started_at` obtient toujours la même image.
    pub started_at: Instant,
    /// `false` fige l'animation (aucun redessin réclamé) — réservé aux captures de non-régression,
    /// dont le harnais exige qu'une frame finisse par ne plus rien demander.
    pub animate: bool,
}

impl LoginState {
    pub fn new(started_at: Instant) -> Self {
        Self {
            started_at,
            animate: true,
        }
    }
}

/// Ce que la fenêtre demande à son hôte — voir la doc de module.
#[derive(Debug, Default)]
pub struct LoginOutcome {
    /// « Rouvrir la page » : l'URL de vérification à ouvrir dans le navigateur.
    pub open_url: Option<String>,
    /// Appui sur l'en-tête : la fenêtre, sans décorations OS, veut être déplacée à la souris.
    pub drag_window: bool,
    /// Hauteur exacte que la carte vient d'occuper — l'hôte y ajuste la fenêtre OS (chaque état
    /// a la sienne : la carte d'appairage est plus haute que l'écran d'accueil).
    pub content_height: f32,
}

/// Peint la fenêtre de connexion dans tout `ui` et rend ce qu'elle demande à l'hôte.
pub fn show(
    ui: &mut egui::Ui,
    state: &mut LoginState,
    icons: &UiIcons,
    auth_status: &AuthStatus,
    auth_command_tx: &dyn AuthCommandSink,
    now: Instant,
) -> LoginOutcome {
    let mut outcome = LoginOutcome::default();
    let elapsed = now.saturating_duration_since(state.started_at);
    let card = ui.max_rect();
    let ctx = ui.ctx().clone();

    // Fond et liseré de la carte — sur TOUTE la fenêtre : c'est l'hôte qui la taille à la carte.
    let (border, ring, ring_soft) = match auth_status {
        AuthStatus::PairingStarted { .. } => (
            CARD_BORDER_PAIRING,
            ACCENT,
            Color32::from_rgba_premultiplied(0, 32, 38, 38),
        ),
        AuthStatus::Disconnected { failure: Some(_) } => (
            CARD_BORDER_ERROR,
            Color32::from_rgb(0xff, 0x5f, 0x57),
            Color32::from_rgba_premultiplied(41, 15, 13, 46),
        ),
        _ => (
            CARD_BORDER,
            Color32::from_rgba_premultiplied(140, 140, 140, 140),
            Color32::from_rgba_premultiplied(20, 20, 20, 20),
        ),
    };
    ui.painter().rect_filled(card, CARD_RADIUS, CARD_FILL);
    ui.painter().rect_stroke(
        card.shrink(0.5),
        CARD_RADIUS,
        Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    let phase = if state.animate {
        (elapsed.as_secs_f32() / RING_PERIOD.as_secs_f32()).fract()
    } else {
        0.0
    };
    paint_ring(ui, card, phase, ring, ring_soft);
    if state.animate {
        ctx.request_repaint_after(ANIMATION_FRAME);
    }

    // ── En-tête : logo, titre, badge, séparateur ────────────────────────────────────────────
    let mut y = card.top() + HEAD_PAD_TOP;
    let center_x = card.center().x;
    let logo_rect = Rect::from_center_size(
        Pos2::new(center_x, y + LOGO_SIZE / 2.0),
        Vec2::splat(LOGO_SIZE),
    );
    egui::Image::new(icons.logo())
        .fit_to_exact_size(Vec2::splat(LOGO_SIZE))
        .paint_at(ui, logo_rect);
    y += LOGO_SIZE + HEAD_GAP;

    // Le bloc de marque réserve 10 px au-dessus du titre pour le badge (« top: -10px »).
    let beta_top = y;
    y += BETA_HEIGHT + HEAD_GAP;
    let title_font = text::label_strong_font(&ctx, TITLE_SIZE);
    let overlay_font = text::label_font(&ctx, TITLE_SIZE);
    let title_main = "WAKFU COMPANION ";
    let title_overlay = "OVERLAY";
    let main_width = spaced_width(ui, title_main, &title_font, TITLE_SPACING);
    let overlay_width = spaced_width(ui, title_overlay, &overlay_font, TITLE_SPACING);
    let title_width = main_width + overlay_width;
    let title_left = center_x - title_width / 2.0;
    let title_center_y = y + TITLE_LINE / 2.0;
    paint_spaced(
        ui,
        Pos2::new(title_left, title_center_y),
        title_main,
        &title_font,
        ACCENT,
        TITLE_SPACING,
    );
    paint_spaced_italic(
        ui,
        Pos2::new(title_left + main_width, title_center_y),
        title_overlay,
        &overlay_font,
        TITLE_OVERLAY,
        TITLE_SPACING,
    );
    // Badge « beta » — calé sur le bord droit du titre, italique.
    let beta_font = text::label_font(&ctx, BETA_HEIGHT);
    let beta_galley = ui.fonts_mut(|f| f.layout_no_wrap("beta".to_owned(), beta_font, BETA));
    paint_italic(
        ui,
        Pos2::new(
            title_left + title_width - beta_galley.rect.width(),
            beta_top + (BETA_HEIGHT - beta_galley.rect.height()) / 2.0,
        ),
        beta_galley,
        BETA,
    );
    y += TITLE_LINE;

    // Poignée de déplacement : tout l'en-tête, du haut de la carte au séparateur.
    let head_rect = Rect::from_min_max(
        Pos2::new(card.left(), card.top()),
        Pos2::new(card.right(), y + HEAD_GAP + RULE_MARGIN_TOP),
    );
    let head = ui.interact(head_rect, ui.id().with("login-head"), Sense::drag());
    if head.drag_started() {
        outcome.drag_window = true;
    }

    // Séparateur gravé : 1 px sombre puis 1 px clair, sur la largeur du contenu.
    y += HEAD_GAP + RULE_MARGIN_TOP;
    let rule_left = card.left() + HEAD_PAD_SIDE;
    let rule_right = card.right() - HEAD_PAD_SIDE;
    ui.painter().rect_filled(
        Rect::from_min_max(Pos2::new(rule_left, y), Pos2::new(rule_right, y + 1.0)),
        0.0,
        RULE_DARK,
    );
    ui.painter().rect_filled(
        Rect::from_min_max(
            Pos2::new(rule_left, y + 1.0),
            Pos2::new(rule_right, y + 2.0),
        ),
        0.0,
        RULE_LIGHT,
    );
    y += 2.0;

    // ── Corps ───────────────────────────────────────────────────────────────────────────────
    y += BODY_PAD_TOP;
    let body_left = card.left() + BODY_PAD_SIDE;
    let body_width = card.width() - 2.0 * BODY_PAD_SIDE;
    let h2_font = text::label_strong_font(&ctx, H2_SIZE);
    let p_font = text::label_font(&ctx, P_SIZE);

    match auth_status {
        AuthStatus::Disconnected { failure: None }
        | AuthStatus::Connecting
        | AuthStatus::Connected => {
            y = paint_paragraph(
                ui,
                Pos2::new(body_left, y),
                body_width,
                "Vous n'êtes pas connecté",
                &h2_font,
                TEXT,
                H2_LINE,
            ) + H2_GAP;
            y = paint_paragraph(
                ui,
                Pos2::new(body_left, y),
                body_width,
                "Associez ce poste à votre compte Wakfu Companion pour activer le suivi des \
                 combats, du butin et de l'historique.",
                &p_font,
                TEXT_MUTED,
                P_LINE,
            ) + P_GAP;
            y = paint_paragraph(
                ui,
                Pos2::new(body_left, y),
                body_width,
                "La connexion se fait sur le site, dans votre navigateur.",
                &p_font,
                TEXT_MUTED,
                P_LINE,
            );
            y += ACTIONS_MARGIN_TOP;
            let button_rect = Rect::from_min_size(
                Pos2::new(body_left, y),
                Vec2::new(body_width, BUTTON_HEIGHT),
            );
            if matches!(auth_status, AuthStatus::Connecting) {
                // Une tentative est en cours (jeton stocké en cours de validation, ou code
                // d'appairage demandé) : le bouton cède la place à un état d'attente.
                paint_wait_row(
                    ui,
                    Pos2::new(body_left, y + (BUTTON_HEIGHT - WAIT_SIZE) / 2.0),
                    "Connexion…",
                    None,
                    elapsed,
                    state.animate,
                );
            } else if button(
                ui,
                button_rect,
                "Se connecter",
                ButtonKind::Primary,
                "login-se-connecter",
            ) {
                tracing::info!("[connexion] « Se connecter » — appairage demandé.");
                auth_command_tx.send(AuthCommand::Retry);
            }
            y += BUTTON_HEIGHT;
        }
        AuthStatus::PairingStarted {
            pairing_code,
            verification_url,
            expires_at,
        } => {
            y = paint_paragraph(
                ui,
                Pos2::new(body_left, y),
                body_width,
                "Confirmez ce code sur le site",
                &h2_font,
                TEXT,
                H2_LINE,
            ) + H2_GAP;
            y = paint_paragraph(
                ui,
                Pos2::new(body_left, y),
                body_width,
                "La page de connexion s'est ouverte dans votre navigateur. Connectez-vous avec \
                 Discord ou Google si nécessaire, puis confirmez le code ci-dessous.",
                &p_font,
                TEXT_MUTED,
                P_LINE,
            );
            // Encadré du code.
            y += CODE_MARGIN_TOP;
            let code_font = FontId::monospace(CODE_VALUE_SIZE);
            let label_font = text::label_font(&ctx, CODE_LABEL_SIZE);
            let label_galley = ui.fonts_mut(|f| {
                f.layout_no_wrap("CODE D'APPAIRAGE".to_owned(), label_font.clone(), ACCENT)
            });
            let code_height = CODE_PAD_TOP
                + CODE_VALUE_SIZE
                + CODE_LABEL_GAP
                + label_galley.rect.height()
                + CODE_PAD_BOTTOM;
            let code_rect =
                Rect::from_min_size(Pos2::new(body_left, y), Vec2::new(body_width, code_height));
            ui.painter()
                .rect_filled(code_rect, BUTTON_RADIUS, CODE_FILL);
            ui.painter().rect_stroke(
                code_rect.shrink(0.5),
                BUTTON_RADIUS,
                Stroke::new(1.0, CODE_BORDER),
                egui::StrokeKind::Inside,
            );
            let code_width = spaced_width(ui, pairing_code, &code_font, CODE_VALUE_SPACING);
            paint_spaced(
                ui,
                Pos2::new(
                    center_x - code_width / 2.0,
                    y + CODE_PAD_TOP + CODE_VALUE_SIZE / 2.0,
                ),
                pairing_code,
                &code_font,
                TEXT,
                CODE_VALUE_SPACING,
            );
            let label_width = spaced_width(ui, "CODE D'APPAIRAGE", &label_font, 0.4);
            paint_spaced(
                ui,
                Pos2::new(
                    center_x - label_width / 2.0,
                    y + CODE_PAD_TOP
                        + CODE_VALUE_SIZE
                        + CODE_LABEL_GAP
                        + label_galley.rect.height() / 2.0,
                ),
                "CODE D'APPAIRAGE",
                &label_font,
                ACCENT,
                0.4,
            );
            y += code_height;
            // Ligne d'attente : points, libellé, compte à rebours.
            y += WAIT_MARGIN_TOP;
            let remaining = expires_at.saturating_duration_since(now);
            let timer = format!(
                "expire dans {:02}:{:02}",
                remaining.as_secs() / 60,
                remaining.as_secs() % 60
            );
            let row_height = paint_wait_row(
                ui,
                Pos2::new(body_left, y),
                "En attente de confirmation",
                Some((&timer, body_left + body_width)),
                elapsed,
                state.animate,
            );
            y += row_height;
            // Deux boutons côte à côte.
            y += ACTIONS_MARGIN_TOP;
            let half = (body_width - ACTIONS_GAP) / 2.0;
            let copy_rect =
                Rect::from_min_size(Pos2::new(body_left, y), Vec2::new(half, BUTTON_HEIGHT));
            let reopen_rect = Rect::from_min_size(
                Pos2::new(body_left + half + ACTIONS_GAP, y),
                Vec2::new(half, BUTTON_HEIGHT),
            );
            if button(
                ui,
                copy_rect,
                "Copier le code",
                ButtonKind::Primary,
                "login-copier",
            ) {
                tracing::info!("[connexion] code d'appairage copié.");
                ctx.copy_text(pairing_code.clone());
            }
            if button(
                ui,
                reopen_rect,
                "Rouvrir la page",
                ButtonKind::Secondary,
                "login-rouvrir",
            ) {
                tracing::info!(
                    "[connexion] « Rouvrir la page » — page de vérification redemandée."
                );
                outcome.open_url = Some(verification_url.clone());
            }
            y += BUTTON_HEIGHT;
            // Lien centré.
            y += LINK_MARGIN_TOP;
            let link_height = link(
                ui,
                Pos2::new(center_x, y),
                "Annuler l'appairage",
                "login-annuler",
                || {
                    tracing::info!("[connexion] appairage annulé par l'utilisateur.");
                    auth_command_tx.send(AuthCommand::CancelPairing);
                },
            );
            y += link_height;
        }
        AuthStatus::Disconnected {
            failure: Some(failure),
        } => {
            // Statut : point rouge auréolé + libellé en capitales.
            let status_font = text::label_strong_font(&ctx, STATUS_SIZE);
            let status_galley = ui.fonts_mut(|f| {
                f.layout_no_wrap("CONNEXION IMPOSSIBLE".to_owned(), status_font, ERROR_TEXT)
            });
            let status_center_y = y + status_galley.rect.height() / 2.0;
            ui.painter().circle_filled(
                Pos2::new(body_left + STATUS_DOT / 2.0, status_center_y),
                STATUS_DOT / 2.0 + 3.0,
                ERROR_DOT_HALO,
            );
            ui.painter().circle_filled(
                Pos2::new(body_left + STATUS_DOT / 2.0, status_center_y),
                STATUS_DOT / 2.0,
                ERROR_DOT,
            );
            ui.painter().galley(
                Pos2::new(body_left + STATUS_DOT + STATUS_GAP, y),
                status_galley.clone(),
                ERROR_TEXT,
            );
            y += status_galley.rect.height() + STATUS_MARGIN_BOTTOM;
            y = paint_paragraph(
                ui,
                Pos2::new(body_left, y),
                body_width,
                &failure.headline,
                &h2_font,
                TEXT,
                H2_LINE,
            ) + H2_GAP;
            y = paint_paragraph(
                ui,
                Pos2::new(body_left, y),
                body_width,
                "L'overlay n'a pas pu se connecter au compte : le serveur Wakfu Companion n'a \
                 pas répondu comme attendu. Vérifiez votre connexion internet, puis réessayez.",
                &p_font,
                TEXT_MUTED,
                P_LINE,
            );
            // Détail technique, sur une ligne, tronqué au besoin.
            y += DETAIL_MARGIN_TOP;
            let detail_font = FontId::monospace(DETAIL_SIZE);
            let detail_galley = ui.fonts_mut(|f| {
                let mut job = LayoutJob::simple(
                    failure.detail.clone(),
                    detail_font,
                    TEXT_DIM,
                    body_width - 2.0 * DETAIL_PAD_X,
                );
                job.wrap.max_rows = 1;
                job.wrap.break_anywhere = true;
                f.layout_job(job)
            });
            let detail_height = detail_galley.rect.height() + 2.0 * DETAIL_PAD_Y;
            let detail_rect = Rect::from_min_size(
                Pos2::new(body_left, y),
                Vec2::new(body_width, detail_height),
            );
            ui.painter().rect_filled(detail_rect, 4.0, DETAIL_FILL);
            ui.painter().rect_stroke(
                detail_rect.shrink(0.5),
                4.0,
                Stroke::new(1.0, SECONDARY_BORDER),
                egui::StrokeKind::Inside,
            );
            ui.painter().galley(
                Pos2::new(body_left + DETAIL_PAD_X, y + DETAIL_PAD_Y),
                detail_galley,
                TEXT_DIM,
            );
            y += detail_height;
            y += ACTIONS_MARGIN_TOP;
            let button_rect = Rect::from_min_size(
                Pos2::new(body_left, y),
                Vec2::new(body_width, BUTTON_HEIGHT),
            );
            if button(
                ui,
                button_rect,
                "Réessayer",
                ButtonKind::Primary,
                "login-reessayer",
            ) {
                tracing::info!("[connexion] « Réessayer » — nouvelle tentative demandée.");
                auth_command_tx.send(AuthCommand::Retry);
            }
            y += BUTTON_HEIGHT;
        }
    }
    y += BODY_PAD_BOTTOM;

    // ── Pied : numéro de version, au style de la bannière de la fenêtre Options ─────────────
    text::paint_outlined_text(
        ui,
        Pos2::new(card.left() + FOOT_PAD_SIDE, y + (FOOT_HEIGHT - 10.0) / 2.0),
        Align2::LEFT_CENTER,
        build_info::banner_label(),
        text::title_font(&ctx, VERSION_SIZE),
        VERSION,
        text::SHADOW_BOTTOM_RIGHT,
    );
    y += FOOT_HEIGHT;

    outcome.content_height = (y - card.top()).round();
    outcome
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonKind {
    Primary,
    Secondary,
}

/// Bouton du site (`.btn-primary` / `.v-btn-secondary`) : plein cyan ou bordé, texte centré.
/// Rend `true` au clic.
fn button(ui: &mut egui::Ui, rect: Rect, label: &str, kind: ButtonKind, log_name: &str) -> bool {
    let response = ui
        .interact(rect, ui.id().with(log_name), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    // Comme les boutons du design system : l'appui fait retomber l'apparence survolée.
    let hovered = response.hovered() && !ui.input(|i| i.pointer.any_down());
    let (fill, stroke, color, font) = match kind {
        ButtonKind::Primary => (
            if hovered { ACCENT_HOVER } else { ACCENT },
            Stroke::NONE,
            ON_ACCENT,
            text::label_strong_font(ui.ctx(), BUTTON_FONT),
        ),
        ButtonKind::Secondary => (
            if hovered {
                SECONDARY_FILL_HOVER
            } else {
                Color32::TRANSPARENT
            },
            Stroke::new(
                1.0,
                if hovered {
                    SECONDARY_BORDER_HOVER
                } else {
                    SECONDARY_BORDER
                },
            ),
            TEXT,
            text::label_strong_font(ui.ctx(), BUTTON_FONT),
        ),
    };
    ui.painter().rect_filled(rect, BUTTON_RADIUS, fill);
    if stroke != Stroke::NONE {
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            BUTTON_RADIUS,
            stroke,
            egui::StrokeKind::Inside,
        );
    }
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, label, font, color);
    if response.clicked() {
        tracing::info!("[connexion] bouton « {label} » cliqué.");
        true
    } else {
        false
    }
}

/// Lien souligné centré sur `center_x` (`.v-link`) — rend sa hauteur, appelle `on_click` au clic.
fn link(
    ui: &mut egui::Ui,
    top_center: Pos2,
    label: &str,
    log_name: &str,
    on_click: impl FnOnce(),
) -> f32 {
    let font = text::label_font(ui.ctx(), LINK_SIZE);
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font, TEXT_DIM));
    let size = galley.rect.size();
    let rect = Rect::from_min_size(Pos2::new(top_center.x - size.x / 2.0, top_center.y), size);
    let response = ui
        .interact(rect, ui.id().with(log_name), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let color = if response.hovered() { TEXT } else { TEXT_DIM };
    let underline = if response.hovered() {
        TEXT
    } else {
        Color32::from_rgba_premultiplied(69, 74, 80, 128)
    };
    ui.painter().galley(rect.min, galley, color);
    let underline_y = rect.bottom() + 1.0;
    ui.painter().line_segment(
        [
            Pos2::new(rect.left(), underline_y),
            Pos2::new(rect.right(), underline_y),
        ],
        Stroke::new(1.0, underline),
    );
    if response.clicked() {
        on_click();
    }
    size.y + 2.0
}

/// Trois points cyan qui pulsent, un libellé, et à droite un compte à rebours facultatif — rend
/// la hauteur de la ligne.
fn paint_wait_row(
    ui: &mut egui::Ui,
    top_left: Pos2,
    label: &str,
    timer: Option<(&str, f32)>,
    elapsed: Duration,
    animate: bool,
) -> f32 {
    let font = text::label_font(ui.ctx(), WAIT_SIZE);
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font, TEXT_DIM));
    let row_height = galley.rect.height();
    let center_y = top_left.y + row_height / 2.0;
    let t = elapsed.as_secs_f32() / DOTS_PERIOD.as_secs_f32();
    for i in 0..3 {
        // `v-web-pulse` : 0,30 → 1 → 0,30 sur la période, chaque point décalé de 0,2 s.
        let local = if animate {
            (t - i as f32 * 0.2 / DOTS_PERIOD.as_secs_f32()).fract()
        } else {
            0.0
        };
        let local = if local < 0.0 { local + 1.0 } else { local };
        let opacity = 0.30 + 0.70 * (0.5 - 0.5 * (local * std::f32::consts::TAU).cos());
        let lift = -(0.5 - 0.5 * (local * std::f32::consts::TAU).cos());
        let x = top_left.x + WAIT_DOT / 2.0 + i as f32 * (WAIT_DOT + WAIT_DOT_GAP);
        ui.painter().circle_filled(
            Pos2::new(x, center_y + lift),
            WAIT_DOT / 2.0,
            ACCENT.gamma_multiply(opacity),
        );
    }
    let text_left = top_left.x + 3.0 * WAIT_DOT + 2.0 * WAIT_DOT_GAP + WAIT_TEXT_GAP;
    ui.painter()
        .galley(Pos2::new(text_left, top_left.y), galley, TEXT_DIM);
    if let Some((timer, right)) = timer {
        ui.painter().text(
            Pos2::new(right, center_y),
            Align2::RIGHT_CENTER,
            timer,
            FontId::monospace(DETAIL_SIZE),
            TEXT_DIM,
        );
    }
    row_height
}

/// Paragraphe à interligne fixé, replié sur `width` — rend le `y` du bas.
fn paint_paragraph(
    ui: &mut egui::Ui,
    top_left: Pos2,
    width: f32,
    text: &str,
    font: &FontId,
    color: Color32,
    line_height: f32,
) -> f32 {
    let galley = ui.fonts_mut(|f| {
        let mut job = LayoutJob::simple(text.to_owned(), font.clone(), color, width);
        job.sections[0].format.line_height = Some(line_height);
        f.layout_job(job)
    });
    let height = galley.rect.height();
    ui.painter().galley(top_left, galley, color);
    top_left.y + height
}

/// Largeur d'un texte peint par [`paint_spaced`] : la somme des glyphes plus l'interlettrage.
fn spaced_width(ui: &egui::Ui, text: &str, font: &FontId, spacing: f32) -> f32 {
    let count = text.chars().count();
    let glyphs: f32 = ui.fonts_mut(|f| text.chars().map(|c| f.glyph_width(font, c)).sum());
    glyphs + spacing * count.saturating_sub(1) as f32
}

/// Texte à interlettrage (`letter-spacing`), qu'`egui` ne connaît pas : un glyphe à la fois, ancré
/// au centre vertical de `left_center`.
fn paint_spaced(
    ui: &egui::Ui,
    left_center: Pos2,
    text: &str,
    font: &FontId,
    color: Color32,
    spacing: f32,
) {
    let mut x = left_center.x;
    for c in text.chars() {
        let width = ui.fonts_mut(|f| f.glyph_width(font, c));
        ui.painter().text(
            Pos2::new(x, left_center.y),
            Align2::LEFT_CENTER,
            c,
            font.clone(),
            color,
        );
        x += width + spacing;
    }
}

/// [`paint_spaced`], en italique simulé (voir [`paint_italic`]).
fn paint_spaced_italic(
    ui: &egui::Ui,
    left_center: Pos2,
    text: &str,
    font: &FontId,
    color: Color32,
    spacing: f32,
) {
    let mut x = left_center.x;
    for c in text.chars() {
        let (width, galley) = ui.fonts_mut(|f| {
            (
                f.glyph_width(font, c),
                f.layout_no_wrap(c.to_string(), font.clone(), color),
            )
        });
        let top_left = Pos2::new(x, left_center.y - galley.rect.height() / 2.0);
        paint_italic(ui, top_left, galley, color);
        x += width + spacing;
    }
}

/// Italique **simulé** : aucune fonte italique n'est embarquée (`assets/fonts`, Ubuntu Regular et
/// Medium seulement), et une graisse de plus coûterait un fichier pour deux mots. Le texte est
/// tessellé puis ses sommets cisaillés vers la droite, proportionnellement à leur hauteur
/// au-dessus de la ligne de base — c'est ce que fait un navigateur pour une italique synthétique.
fn paint_italic(ui: &egui::Ui, top_left: Pos2, galley: Arc<egui::Galley>, color: Color32) {
    let height = galley.rect.height();
    let shape = egui::Shape::Text(egui::epaint::TextShape::new(top_left, galley, color));
    let ctx = ui.ctx();
    let options = ctx.tessellation_options(|o| *o);
    let font_tex_size = ui.fonts(|f| f.font_image_size());
    let mut tessellator =
        egui::epaint::Tessellator::new(ctx.pixels_per_point(), options, font_tex_size, Vec::new());
    let mut mesh = egui::epaint::Mesh::default();
    tessellator.tessellate_shape(shape, &mut mesh);
    // Ligne de base ≈ 80 % de la hauteur de la galley : les jambages descendent en dessous et
    // penchent donc légèrement vers la gauche, comme dans une vraie italique.
    let baseline = top_left.y + height * 0.8;
    for vertex in &mut mesh.vertices {
        vertex.pos.x += (baseline - vertex.pos.y) * ITALIC_SHEAR;
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}

/// Anneau lumineux autour de `card` : un dégradé conique qui tourne (`conic-gradient(from angle,
/// transparent 0deg, soft 200deg, ring 300deg, transparent 360deg)`), réduit au liseré.
///
/// Un `Mesh` à couleurs par sommet : le pourtour arrondi est échantillonné finement, chaque point
/// donne quatre sommets (deux de lisière transparents pour l'anticrénelage, deux de cœur colorés
/// selon l'angle du point vu du centre). `phase` ∈ [0, 1[ est la fraction de tour déjà faite.
fn paint_ring(ui: &egui::Ui, card: Rect, phase: f32, ring: Color32, ring_soft: Color32) {
    const SAMPLES_PER_EDGE: usize = 24;
    const SAMPLES_PER_CORNER: usize = 16;
    const FEATHER: f32 = 0.75;
    // Le pseudo-élément déborde d'un pixel (`inset: -1px`) : l'anneau chevauche le liseré.
    let outer = card.expand(0.5);
    let radius = CARD_RADIUS + 0.5;
    let center = outer.center();

    // Pourtour dans le sens horaire, en partant du milieu du bord haut — avec la normale sortante
    // de chaque point.
    let mut points: Vec<(Pos2, Vec2)> = Vec::new();
    let corners = [
        (
            Pos2::new(outer.right() - radius, outer.top() + radius),
            -90.0_f32,
        ), // haut-droite
        (
            Pos2::new(outer.right() - radius, outer.bottom() - radius),
            0.0,
        ), // bas-droite
        (
            Pos2::new(outer.left() + radius, outer.bottom() - radius),
            90.0,
        ), // bas-gauche
        (
            Pos2::new(outer.left() + radius, outer.top() + radius),
            180.0,
        ), // haut-gauche
    ];
    let edges = [
        (
            Pos2::new(outer.center().x, outer.top()),
            Pos2::new(outer.right() - radius, outer.top()),
            Vec2::new(0.0, -1.0),
        ),
        (
            Pos2::new(outer.right(), outer.top() + radius),
            Pos2::new(outer.right(), outer.bottom() - radius),
            Vec2::new(1.0, 0.0),
        ),
        (
            Pos2::new(outer.right() - radius, outer.bottom()),
            Pos2::new(outer.left() + radius, outer.bottom()),
            Vec2::new(0.0, 1.0),
        ),
        (
            Pos2::new(outer.left(), outer.bottom() - radius),
            Pos2::new(outer.left(), outer.top() + radius),
            Vec2::new(-1.0, 0.0),
        ),
        (
            Pos2::new(outer.left() + radius, outer.top()),
            Pos2::new(outer.center().x, outer.top()),
            Vec2::new(0.0, -1.0),
        ),
    ];
    for (i, (from, to, normal)) in edges.iter().enumerate() {
        let n = if i == 0 || i == 4 {
            SAMPLES_PER_EDGE / 2
        } else {
            SAMPLES_PER_EDGE
        };
        for k in 0..n {
            let t = k as f32 / n as f32;
            points.push((*from + (*to - *from) * t, *normal));
        }
        if i < 4 {
            let (corner_center, start_deg) = corners[i];
            for k in 0..SAMPLES_PER_CORNER {
                let angle = (start_deg + 90.0 * k as f32 / SAMPLES_PER_CORNER as f32).to_radians();
                let normal = Vec2::new(angle.cos(), angle.sin());
                points.push((corner_center + normal * radius, normal));
            }
        }
    }

    let color_at = |p: Pos2| -> Color32 {
        // Angle horaire depuis midi, vu du centre — celui du dégradé conique CSS.
        let d = p - center;
        let angle = d.x.atan2(-d.y).rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
        let t = (angle - phase).rem_euclid(1.0) * 360.0;
        let lerp = |a: Color32, b: Color32, k: f32| {
            let k = k.clamp(0.0, 1.0);
            let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
            Color32::from_rgba_premultiplied(
                mix(a.r(), b.r()),
                mix(a.g(), b.g()),
                mix(a.b(), b.b()),
                mix(a.a(), b.a()),
            )
        };
        if t < 200.0 {
            lerp(Color32::TRANSPARENT, ring_soft, t / 200.0)
        } else if t < 300.0 {
            lerp(ring_soft, ring, (t - 200.0) / 100.0)
        } else {
            lerp(ring, Color32::TRANSPARENT, (t - 300.0) / 60.0)
        }
    };

    let mut mesh = egui::epaint::Mesh::default();
    let half = RING_WIDTH / 2.0;
    for (p, normal) in &points {
        let color = color_at(*p);
        mesh.colored_vertex(*p + *normal * (half + FEATHER), Color32::TRANSPARENT);
        mesh.colored_vertex(*p + *normal * half, color);
        mesh.colored_vertex(*p - *normal * half, color);
        mesh.colored_vertex(*p - *normal * (half + FEATHER), Color32::TRANSPARENT);
    }
    let count = points.len() as u32;
    for i in 0..count {
        let a = i * 4;
        let b = ((i + 1) % count) * 4;
        for row in 0..3 {
            mesh.add_triangle(a + row, b + row, a + row + 1);
            mesh.add_triangle(b + row, b + row + 1, a + row + 1);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}
