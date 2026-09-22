//! **Fenêtre de connexion** (2026-09-14, §9.1 undecies du plan) — la première interface de
//! l'overlay, et la seule tant qu'aucun compte n'est lié.
//!
//! Elle reproduit la maquette validée par l'utilisateur (direction « Fidèle au web », révisée en
//! quatre passes) : une carte de 400 px sur fond noir translucide, le logo du site, le titre
//! « WAKFU COMPANION » dans l'accent cyan du site suivi d'« OVERLAY » en italique gris, le badge
//! « beta » en haut à droite, un séparateur gravé (ligne sombre puis claire), un corps aligné à
//! gauche, et le numéro de version en pied, en bas à droite, au style du badge « beta ». Un
//! **anneau lumineux tourne en permanence** autour de la carte — gris translucide au repos, cyan
//! pendant l'appairage, rouge en erreur : la fenêtre est toujours vivante.
//!
//! Quatre états — l'écran de chargement, puis trois calqués sur [`AuthStatus`] — **plus l'écran
//! de mise à jour** (2026-09-18, [`paint_manual_update`]), qui n'a rien à voir avec le compte :
//! l'entrée « Mise à jour » du menu de la zone de notification ouvre CETTE fenêtre
//! ([`LoginState::manual_update`]), rouage compris, pour que la recherche se voie au lieu de
//! courir en silence — et elle vit alors à côté des overlays de jeu, seule exception à « compte
//! lié = pas de fenêtre de connexion ».
//!
//! Les états du compte :
//!
//! - **chargement** ([`LoginState::loading`], et `Connecting`) : la même carte, avec le rouage du
//!   jeu (`design::loader`) centré dans le corps et rien d'autre. C'est **la toute première
//!   image de l'overlay** : elle masque tout le démarrage — catalogue, référentiels, rattrapage
//!   de `wakfu.log`, vérification du jeton stocké et récupération des réglages du compte (voir
//!   `crate::startup`) — et ne cède la place qu'à l'overlay (compte lié) ou à l'écran suivant ;
//! - **non connecté** (`Disconnected { failure: None }`) : « Vous n'êtes pas connecté » + « Se
//!   connecter », et dessous la ligne d'acceptation — « En vous connectant, vous acceptez » les
//!   conditions d'utilisation et la politique de confidentialité du service, en liens (2026-09-18,
//!   `docs/analyse-rgpd.md` §3.4 : l'information des personnes se donne avant le geste) ;
//! - **appairage** (`PairingStarted`) : le code en grand, le compte à rebours, « Copier le code »,
//!   « Rouvrir la page », et le lien « Annuler l'appairage » ;
//! - **erreur** (`Disconnected { failure: Some(_) }`) : « Connexion impossible », le titre court de
//!   l'échec, le détail technique, « Réessayer » et « Copier le détail » (pour le support) ;
//!
//! **Palette du site, pas du jeu.** C'est un choix délibéré et validé : cette fenêtre n'est pas un
//! overlay posé sur le jeu mais une fenêtre logicielle classique, la porte d'entrée du compte
//! Wakfu Companion — elle reprend donc l'en-tête du site (`app-header.component`, accent
//! `#00d2ff`, badge *beta*, logo `logo-purple.png`) et ses boutons (`.btn-primary`), pas les
//! textures 9-slice de `crate::design`. Seul le rouage de l'écran de chargement vient du jeu.
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

use overlay_sync::update::{self, UpdateStatus};

use crate::build_info;
use crate::design::{self, text};
use crate::render_content::{AuthCommand, AuthCommandSink, AuthStatus};
use crate::ui_icons::UiIcons;

/// Largeur de la carte, et donc de la fenêtre OS (maquette : 400 px).
pub const WINDOW_WIDTH: f32 = 400.0;

/// **La hauteur de la Carte, la même pour tous les écrans ordinaires** (2026-09-22, demande
/// utilisateur : « si on balaye de chargement à erreur, on fait yo-yo en termes de hauteur, et
/// c'est assez perturbant »).
///
/// Chaque écran se mesurait lui-même et l'hôte retaillait la fenêtre : passer du chargement à
/// « à jour » la faisait sauter de quatre-vingt-dix pixels, y revenir la rouvrait d'autant. Sur
/// une fenêtre sans décoration posée au milieu de l'écran, ce mouvement est le plus visible de
/// toute la Carte — et il ne veut rien dire.
///
/// La zone de corps ([`BODY_HEIGHT`]) est donc constante, et le contenu y est **centré
/// verticalement** — ce que faisait déjà le rouage de l'écran de chargement, seul de son espèce.
/// Sa valeur est celle de l'écran le plus chargé, l'appairage (titre, explication, encadré du
/// code, ligne d'attente, deux boutons et un lien), avec une marge de quelques pixels.
///
/// **Trois exceptions**, et elles sont voulues : le volet À propos, qui s'ouvre exprès jusqu'à
/// [`ABOUT_SCREEN_RATIO`] de la hauteur de l'écran ; les écrans défilants à venir, sur la même
/// mécanique ; et un contenu qui déborde malgré tout — un titre d'échec inhabituellement long —,
/// auquel cas la Carte grandit de ce qu'il manque plutôt que de rogner le texte.
pub const CARD_HEIGHT: f32 = HEAD_HEIGHT + RULE_HEIGHT + BODY_HEIGHT + FOOT_HEIGHT;
/// Hauteur de la zone de corps, marges hautes et basses comprises — voir [`CARD_HEIGHT`].
const BODY_HEIGHT: f32 = 352.0;
/// Hauteur de l'en-tête déployé : du haut de la carte au séparateur.
const HEAD_HEIGHT: f32 = HEAD_PAD_TOP
    + LOGO_SIZE
    + 2.0 * HEAD_GAP
    + BETA_HEIGHT
    + TITLE_LINE
    + HEAD_GAP
    + RULE_MARGIN_TOP;
/// Hauteur de l'en-tête replié — voir [`paint_head`].
const HEAD_HEIGHT_COMPACT: f32 = 56.0;
/// Épaisseur du séparateur gravé (une ligne sombre, une ligne claire).
const RULE_HEIGHT: f32 = 2.0;
/// Durée de l'ouverture et de la fermeture du volet À propos — en-tête et hauteur ensemble.
const ABOUT_MORPH: Duration = Duration::from_millis(220);
/// Part de la hauteur de l'écran que le volet À propos ne dépasse jamais (demande utilisateur).
const ABOUT_SCREEN_RATIO: f32 = 0.8;
/// Hauteur de la fenêtre OS à sa création — [`CARD_HEIGHT`] depuis le 2026-09-22, où tous les
/// écrans ordinaires ont adopté la même. Ce n'est plus une hauteur « initiale » distincte des
/// autres ; le nom reste parce que c'est ainsi que l'hôte nomme ce qu'il demande à winit.
pub const INITIAL_HEIGHT: f32 = CARD_HEIGHT;

// ── Palette (dépôt web : `styles.css`, `app-header.component.css`) ─────────────────────────────
/// Fond de la carte — `rgba(8,10,14,.96)`. Le web est à `.78`, et la fenêtre l'a été jusqu'au
/// 2026-09-16 : « rendre moins translucide d'au moins 40 % » (demande utilisateur) l'a portée à
/// `.90`, puis le 2026-09-22 à `.96` — sur un inventaire clair, `.90` laissait encore passer assez
/// de motif pour que le texte devienne pénible à lire à l'usage. Posée sur le bureau, et non sur
/// une page déjà sombre comme au web, la carte n'a pas le fond que le web lui suppose.
const CARD_FILL: Color32 = Color32::from_rgba_premultiplied(8, 10, 13, 245); // rgba(8,10,14,.96)
/// Bordure fixe de la carte, **pour tous les écrans sauf les échecs** : le cyan du site à .22, qui
/// n'était jusqu'au 2026-09-22 que celle de l'appairage. Le gris d'origine appartenait au liseré
/// blanc qui a disparu avec lui (voir [`RingStyle`]).
const CARD_BORDER: Color32 = Color32::from_rgba_premultiplied(0, 46, 56, 56); // cyan .22
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
/// Auréole de la pastille cyan (écran de mise à jour manuelle) — l'accent du site à .18, comme
/// [`ERROR_DOT_HALO`] l'est du rouge d'erreur.
const ACCENT_HALO: Color32 = Color32::from_rgba_premultiplied(0, 37, 45, 46);
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

// ── Onde de la pastille d'état (2026-09-22) ────────────────────────────────────────────────────
/// Période de l'onde qui part de la pastille de [`paint_status_row`]. Deux secondes : 1,5 s
/// paraissait pressé sur une fenêtre qu'on garde ouverte, 3 s ne se remarquait plus (essais
/// successifs avec l'utilisateur sur maquette).
const STATUS_PULSE_PERIOD: Duration = Duration::from_secs(2);
/// Distance que l'onde parcourt au-delà du bord de la pastille avant de s'éteindre.
const STATUS_PULSE_REACH: f32 = 9.0;
/// Opacité de l'onde au départ — elle décroît linéairement jusqu'à zéro.
const STATUS_PULSE_ALPHA: f32 = 0.45;
/// Part de la période pendant laquelle l'onde court : le reste est le silence entre deux ondes.
const STATUS_PULSE_DUTY: f32 = 0.7;

// ── Jauge de téléchargement (2026-09-22) ───────────────────────────────────────────────────────
/// Période du reflet qui balaie le remplissage de la jauge — voir [`paint_card_meter`].
const SHEEN_PERIOD: Duration = Duration::from_millis(1600);
/// Largeur du reflet, en fraction de la largeur de la jauge.
const SHEEN_WIDTH_RATIO: f32 = 0.35;
/// Piste de la jauge de la Carte — le gris d'un champ du site, pas les bordures du jeu.
const METER_TRACK: Color32 = Color32::from_rgb(0x1b, 0x20, 0x28);
/// Remplissage d'une jauge **à l'arrêt** : la couleur s'éteint en même temps que le reflet.
const METER_STALLED: Color32 = Color32::from_rgb(0x4a, 0x55, 0x60);

// ── Gabarit (maquette v4, valeurs en px logiques) ──────────────────────────────────────────────
const CARD_RADIUS: f32 = 10.0;
const HEAD_PAD_TOP: f32 = 26.0;
const HEAD_PAD_SIDE: f32 = 18.0;
const LOGO_SIZE: f32 = 56.0;
/// Côté du logo dans l'en-tête replié : la moitié, comme demandé.
const LOGO_SIZE_COMPACT: f32 = 28.0;
/// Marge latérale du logo dans l'en-tête replié.
const HEAD_PAD_COMPACT: f32 = 26.0;
/// Gouttière entre le logo et le titre, puis entre le titre et le badge, en-tête replié.
const HEAD_COMPACT_GAP: f32 = 10.0;
/// De combien le badge « beta » monte au-dessus du centre du titre, en-tête replié.
const BETA_LIFT_COMPACT: f32 = 4.0;
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
// ── Ligne d'acceptation (voir [`paint_consent_notice`]) ────────────────────────────────────────
/// Ce que l'écran « non connecté » dit avant « Se connecter ». Les deux textes y sont **nommés**,
/// pas liés : le pied les ouvre, sur tous les écrans.
const CONSENT_NOTICE: &str = "En vous connectant, vous acceptez les conditions d'utilisation et \
                              la politique de confidentialité de Wakfu Companion.";
const CONSENT_SIZE: f32 = 11.0;
const CONSENT_LINE: f32 = 16.0;
/// Plus gris que le corps : une mention, pas une phrase du discours.
const CONSENT_TEXT: Color32 = Color32::from_rgb(0x6b, 0x75, 0x80);
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
/// Hauteur du pied : [`FOOT_PAD_TOP`], deux lignes de liens de [`FOOT_LINE`], puis
/// [`FOOT_PAD_BOTTOM`] — voir [`paint_foot`]. Trente pixels jusqu'au 2026-09-22, où le pied ne
/// portait que la version.
const FOOT_HEIGHT: f32 = FOOT_PAD_TOP + 2.0 * FOOT_LINE + FOOT_PAD_BOTTOM;
const FOOT_PAD_TOP: f32 = 9.0;
const FOOT_LINE: f32 = 15.0;
const FOOT_PAD_BOTTOM: f32 = 12.0;
/// Corps des liens du pied — celui de la version, un point de moins que les liens du corps.
const FOOT_LINK_SIZE: f32 = 10.0;
/// Ce qui sépare deux liens d'une même ligne du pied.
const FOOT_SEPARATOR: &str = "·";
/// Air de chaque côté du point médian.
const FOOT_SEPARATOR_PAD: f32 = 6.0;
const FOOT_SEPARATOR_COLOR: Color32 = Color32::from_rgb(0x45, 0x4d, 0x57);
/// Lien éteint : celui de la section où l'on se trouve déjà — voir [`paint_foot`].
const LINK_DISABLED: Color32 = Color32::from_rgb(0x5a, 0x64, 0x70);
/// Opacité du soulignement, rapportée à la couleur du lien.
const UNDERLINE_ALPHA: f32 = 0.45;
/// Air entre le libellé d'un lien externe et sa flèche.
const LINK_ARROW_GAP: f32 = 3.0;
/// Côté de la flèche sortante, rapporté au corps du lien.
const LINK_ARROW_RATIO: f32 = 0.7;

// ── Volet « À propos » (2026-09-22, voir [`paint_about`]) ──────────────────────────────────────
const ABOUT_PAD_TOP: f32 = 18.0;
const ABOUT_PAD_BOTTOM: f32 = 8.0;
const ABOUT_PAD_SIDE: f32 = 24.0;
/// Marge droite du texte : elle laisse passer la barre de défilement sans la chevaucher.
const ABOUT_PAD_RIGHT: f32 = 22.0;
const ABOUT_HEADING_SIZE: f32 = 15.0;
const ABOUT_HEADING_LINE: f32 = 20.0;
const ABOUT_HEADING_GAP: f32 = 8.0;
const ABOUT_BLOCK_GAP: f32 = 8.0;
const ABOUT_SECTION_GAP: f32 = 18.0;
const ABOUT_LINKS_GAP: f32 = 6.0;
/// Bande collante du bas : l'air au-dessus du bouton, le bouton, l'air en dessous.
const BACKBAR_HEIGHT: f32 = BACKBAR_PAD_TOP + BUTTON_HEIGHT + BACKBAR_PAD_BOTTOM;
const BACKBAR_PAD_TOP: f32 = 12.0;
/// Ce qui sépare le bouton « Retour » du bas de la Carte. Il n'y en avait pas : le bouton
/// touchait la bordure, et c'était visible (2026-09-22).
const BACKBAR_PAD_BOTTOM: f32 = 14.0;
/// Barre rouge d'un bloc d'avertissement du volet, et son fond.
const ALERT_BAR_WIDTH: f32 = 2.0;
const ALERT_PAD_X: f32 = 10.0;
const ALERT_PAD_Y: f32 = 6.0;
const ALERT_FILL: Color32 = Color32::from_rgba_premultiplied(16, 6, 5, 18); // rouge .07

// ── Barre de défilement de la Carte (voir [`style_card_scrollbar`]) ────────────────────────────
const SCROLLBAR_WIDTH: f32 = 6.0;
const SCROLLBAR_MARGIN: f32 = 8.0;
const SCROLLBAR_MIN_LENGTH: f32 = 24.0;
const SCROLLBAR_HANDLE: Color32 = Color32::from_rgb(0x2a, 0x30, 0x38);
/// Identifiant de la zone défilante du volet — il porte aussi sa position de défilement.
const ABOUT_SCROLL_ID: &str = "carte-a-propos";
/// Côté du rouage de l'écran de chargement — le palier « bloc en cours de chargement » du design
/// system (`LoaderSize::Medium`, 72 px), assez grand pour être le seul sujet de la carte sans
/// l'écraser.
const LOADER_SIZE: f32 = 72.0;
const FOOT_PAD_SIDE: f32 = 12.0;
// ── Mise à jour (2026-09-15, docs/plan-mise-a-jour.md §8.1 — mécanisme seulement, la mise en
// forme de l'écran de chargement est à retravailler dans une itération dédiée) ─────────────────
/// Écart entre le bas du rouage et la ligne d'état de mise à jour.
const UPDATE_LABEL_MARGIN_TOP: f32 = 14.0;
const UPDATE_LABEL_SIZE: f32 = 12.0;
const UPDATE_LABEL_LINE: f32 = 16.0;
/// Jauge de téléchargement : sous la ligne d'état, aux deux tiers de la largeur du corps.
const UPDATE_METER_MARGIN_TOP: f32 = 8.0;
const UPDATE_METER_HEIGHT: f32 = 8.0;
const UPDATE_METER_WIDTH_RATIO: f32 = 0.66;
const UPDATE_COUNT_MARGIN_TOP: f32 = 5.0;
const UPDATE_COUNT_SIZE: f32 = 10.0;
/// Le corps du badge « beta » (`BETA_HEIGHT`) : la version est son pendant, en bas à droite.
const VERSION_SIZE: f32 = BETA_HEIGHT;
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
    /// Écran de chargement (voir la doc de module) — posé par l'hôte tant que les chargements
    /// initiaux ne sont pas terminés (`crate::startup::StartupProgress::is_complete`). `true` à la
    /// création : la fenêtre naît sur le rouage.
    pub loading: bool,
    /// Où en est la mise à jour automatique (`overlay_sync::update::UpdateStatus`) — copié par
    /// l'hôte depuis l'état publié par le thread de mise à jour à chaque passage de
    /// `sync_session_windows`. Sous le rouage, l'écran de chargement en montre l'avancement
    /// (téléchargement, vérification, installation) ; une mise à jour **obligatoire** en échec
    /// remplace le rouage par « Mise à jour requise » et son bouton « Réessayer ».
    pub update: UpdateStatus,
    /// **Recherche de mise à jour demandée à la main** (2026-09-18, entrée « Mise à jour » du menu
    /// de la zone de notification) : la carte montre l'écran de mise à jour — le rouage et
    /// « Recherche d'une mise à jour… » pendant la vérification, puis son verdict (« Vous êtes
    /// déjà à jour », « Version X disponible », ou l'échec) avec de quoi fermer. Posé et retiré
    /// par l'hôte, qui garde la fenêtre ouverte tant qu'il vaut `true` — même compte lié, où elle
    /// n'existerait pas autrement (voir `App::sync_session_windows`).
    ///
    /// Il PRIME sur l'écran de chargement (`loading`) : un téléchargement lancé depuis cet écran
    /// rebloque le démarrage, et l'utilisateur doit continuer à voir l'avancement là où il l'a
    /// demandé, pas basculer sur l'écran de démarrage. Seule une mise à jour **obligatoire** en
    /// échec passe devant (voir `paint_update_required`).
    pub manual_update: bool,
    /// **La confirmation d'effacement des données locales est-elle ouverte ?** (2026-09-18,
    /// constat C5 de `docs/analyse-rgpd.md` §3.5) — le lien « Supprimer les données locales » de
    /// l'écran « non connecté » la lève, ses deux boutons la referment.
    ///
    /// Un état de la carte plutôt qu'une boîte de dialogue : `design::confirm_dialog` est du jeu
    /// (textures 9-slice, crête dorée) et cette fenêtre est du SITE — voir « Palette du site, pas
    /// du jeu » dans la doc de module. Poser la boîte du jeu ici aurait été le seul endroit de
    /// l'overlay où les deux langages visuels se superposent.
    pub purge_confirm: bool,
    /// **Le volet « À propos » est-il ouvert ?** (2026-09-22) — le lien du pied le lève, le bouton
    /// « Retour » le baisse, et la Carte revient exactement à l'écran qu'elle montrait : rien
    /// d'autre n'est mémorisé, parce que rien d'autre n'a changé.
    ///
    /// C'est le seul état qui fasse changer la Carte de taille (voir [`CARD_HEIGHT`]) : l'en-tête
    /// se replie et la fenêtre s'ouvre jusqu'à [`ABOUT_SCREEN_RATIO`] de la hauteur de l'écran, en
    /// un seul mouvement de [`ABOUT_MORPH`].
    pub about: bool,
    /// Hauteur de l'écran sur lequel la fenêtre est posée, en points logiques — posée par l'hôte,
    /// seul à connaître le moniteur (`window.current_monitor()`). Sert au plafond du volet
    /// À propos ; une valeur nulle ou absurde le ramène à [`CARD_HEIGHT`], jamais à rien.
    pub monitor_height: f32,
    /// **L'overlay a-t-il déjà écrit sur cette machine ?** — `None` tant que la question n'a pas
    /// été posée au disque (`local_data::has_user_data`), puis la réponse, gardée pour la vie de
    /// la fenêtre. Elle décide d'offrir ou non « Supprimer les données locales » : à la première
    /// ouverture, ce lien ne propose d'effacer rien du tout, et inquiète pour rien.
    pub has_local_data: Option<bool>,
}

impl LoginState {
    pub fn new(started_at: Instant) -> Self {
        Self {
            started_at,
            animate: true,
            loading: true,
            update: UpdateStatus::Idle,
            manual_update: false,
            purge_confirm: false,
            about: false,
            monitor_height: CARD_HEIGHT,
            has_local_data: None,
        }
    }
}

/// **Les horloges de la carte**, calculées une fois par frame dans [`show`] et passées aux
/// peintres qui en ont besoin. Chacune est une fraction de sa propre période, ∈ [0, 1[.
///
/// Sous `animate == false` — le harnais de captures, dont une frame doit finir par ne plus rien
/// demander — elles ne valent pas zéro mais une pose choisie : à zéro, le reflet de la jauge est
/// hors champ et l'onde de la pastille collée à son bord, c'est-à-dire que la référence ne
/// montrerait rien de ce qu'on vient d'ajouter.
#[derive(Clone, Copy, Debug)]
struct Phases {
    ring: f32,
    pulse: f32,
    sheen: f32,
}

impl Phases {
    fn new(elapsed: Duration, animate: bool) -> Self {
        let fraction = |period: Duration| (elapsed.as_secs_f32() / period.as_secs_f32()).fract();
        if animate {
            Self {
                ring: fraction(RING_PERIOD),
                pulse: fraction(STATUS_PULSE_PERIOD),
                sheen: fraction(SHEEN_PERIOD),
            }
        } else {
            Self {
                ring: 0.0,
                pulse: 0.3,
                sheen: 0.45,
            }
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
    /// « Réessayer » de l'écran « Mise à jour requise » : l'hôte relance la vérification avec
    /// installation (`background::UpdateCommand::Check { install_if_available: true }`).
    pub retry_update: bool,
    /// « Rechercher à nouveau » de l'écran de mise à jour manuelle : une vérification SANS
    /// installation (`background::UpdateCommand::Check { install_if_available: false }`), la même
    /// que l'entrée du menu de la zone de notification qui a ouvert cet écran.
    pub check_update: bool,
    /// « Mettre à jour maintenant » de l'écran de mise à jour manuelle : l'hôte lance le
    /// téléchargement (`background::UpdateCommand::Download`) — l'avancement s'affiche sur cette
    /// même carte, puis l'overlay se relance.
    pub install_update: bool,
    /// « Fermer » / « Plus tard » de l'écran de mise à jour manuelle : l'hôte sort du mode
    /// manuel (`LoginState::manual_update`), ce qui referme la fenêtre si un compte est lié, ou
    /// la ramène à l'écran de connexion sinon.
    pub close_update: bool,
    /// **« Supprimer »** de l'écran de confirmation d'effacement (2026-09-18, constat C5) :
    /// l'hôte efface les données locales (`local_data::Scope::Everything`) puis arrête le
    /// programme — comme l'action `PurgeLocalData` de la fenêtre Options, dont c'est le pendant
    /// pour un overlay sans compte lié, où cette fenêtre est la seule interface.
    pub purge_local_data: bool,
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
    let update_required = matches!(
        state.update,
        UpdateStatus::Failed {
            mandatory: true,
            ..
        }
    );
    // **Le liseré dit ce que fait la carte** (2026-09-22) : bleu au repos, arc-en-ciel dès qu'une
    // mise à jour travaille, rouge sur un échec. Voir [`RingStyle`].
    let update_failed = matches!(
        state.update,
        UpdateStatus::Failed { .. } | UpdateStatus::Unavailable { .. }
    );
    let update_working = matches!(
        state.update,
        UpdateStatus::Downloading { .. }
            | UpdateStatus::Verifying { .. }
            | UpdateStatus::ReadyToInstall { .. }
            | UpdateStatus::Installing { .. }
    ) || (state.manual_update
        && matches!(state.update, UpdateStatus::Idle | UpdateStatus::Checking));
    let failed = update_required
        || update_failed
        || matches!(auth_status, AuthStatus::Disconnected { failure: Some(_) });
    let ring_style = if failed {
        RingStyle::Error
    } else if update_working {
        RingStyle::Rainbow
    } else {
        RingStyle::Blue
    };
    let border = if failed {
        CARD_BORDER_ERROR
    } else {
        CARD_BORDER
    };
    ui.painter().rect_filled(card, CARD_RADIUS, CARD_FILL);
    ui.painter().rect_stroke(
        card.shrink(0.5),
        CARD_RADIUS,
        Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    let phases = Phases::new(elapsed, state.animate);
    paint_ring(ui, card, phases.ring, ring_style);
    if state.animate {
        ctx.request_repaint_after(ANIMATION_FRAME);
    }

    // ── En-tête, séparateur, corps, pied ───────────────────────────────────────────────────
    // **La Carte a une taille figée** (2026-09-22) — voir [`CARD_HEIGHT`]. Le pied est ancré au
    // bas de la fenêtre, l'en-tête à son haut, et le corps occupe ce qui reste : aucun écran ne
    // décide plus de la hauteur, sauf le volet À propos, qui s'ouvre exprès.
    let about_t = about_progress(&ctx, ui.id(), state);
    let head_height = paint_head(ui, card, about_t, icons, &mut outcome);
    paint_rule(ui, card, card.top() + head_height);

    let foot_top = card.bottom() - FOOT_HEIGHT;
    let body_rect = Rect::from_min_max(
        Pos2::new(
            card.left(),
            (card.top() + head_height + RULE_HEIGHT).min(foot_top),
        ),
        Pos2::new(card.right(), foot_top),
    );

    if state.about {
        paint_about(ui, body_rect, state, phases, &mut outcome);
    } else {
        // **Contenu centré dans une zone de hauteur constante.** La hauteur du contenu n'est
        // connue qu'après l'avoir peint : elle est donc d'abord mesurée dans un `Ui` invisible
        // (`UiBuilder::invisible`, qui écarte aussi bien les formes que les clics), puis le
        // vrai passage part du `y` qui centre. Deux passes, mais les galleys sont mises en
        // cache par egui : la seconde ne remet en page aucun texte.
        let inner_top = body_rect.top() + BODY_PAD_TOP;
        let inner_bottom = body_rect.bottom() - BODY_PAD_BOTTOM;
        let measured = {
            let probe = Rect::from_min_size(
                Pos2::new(body_rect.left(), inner_top),
                Vec2::new(body_rect.width(), f32::INFINITY),
            );
            let mut height = 0.0;
            ui.scope_builder(
                egui::UiBuilder::new().invisible().max_rect(probe),
                |probe_ui| {
                    height = paint_body(
                        probe_ui,
                        card,
                        inner_top,
                        state,
                        auth_status,
                        auth_command_tx,
                        now,
                        elapsed,
                        phases,
                        &mut LoginOutcome::default(),
                    ) - inner_top;
                },
            );
            height
        };
        let top = (inner_top + ((inner_bottom - inner_top) - measured) / 2.0).max(inner_top);
        paint_body(
            ui,
            card,
            top,
            state,
            auth_status,
            auth_command_tx,
            now,
            elapsed,
            phases,
            &mut outcome,
        );
    }

    paint_foot(ui, card, foot_top, state, &mut outcome);

    outcome.content_height = card_height(state, about_t);
    outcome
}

/// Où en est l'ouverture du volet À propos : 0 en écran ordinaire, 1 volet ouvert, entre les deux
/// pendant les [`ABOUT_MORPH`] de la transition. Une seule valeur pilote l'en-tête ET la hauteur
/// de la Carte — un seul mouvement, jamais deux animations qui se croisent.
fn about_progress(ctx: &egui::Context, id: egui::Id, state: &LoginState) -> f32 {
    if !state.animate {
        return if state.about { 1.0 } else { 0.0 };
    }
    ctx.animate_bool_with_time(
        id.with("carte-a-propos"),
        state.about,
        ABOUT_MORPH.as_secs_f32(),
    )
}

/// **La hauteur que la Carte demande à son hôte, pour la frame en cours.**
///
/// Elle est **calculée**, jamais mesurée après coup, et c'est essentiel : l'hôte taille la fenêtre
/// OS d'après `LoginOutcome::content_height`, qu'il lit à la frame suivante. Une hauteur mesurée
/// sur ce qui vient d'être peint arriverait donc toujours avec une frame de retard — et la
/// dernière frame de l'animation garderait la valeur de l'avant-dernière. C'est exactement ce qui
/// coupait le pied de la Carte au retour du volet À propos (2026-09-22) : la fenêtre se refermait
/// sur cent pixels de moins, la différence entre l'en-tête déployé et replié.
fn card_height(state: &LoginState, about_t: f32) -> f32 {
    let open = (state.monitor_height * ABOUT_SCREEN_RATIO).max(CARD_HEIGHT);
    (CARD_HEIGHT + (open - CARD_HEIGHT) * about_t).round()
}

/// **L'en-tête, qui se replie** (2026-09-22, demande utilisateur) — logo, titre, badge « beta »,
/// et la poignée de déplacement de la fenêtre. Rend sa hauteur.
///
/// Deux dispositions, qui sont les bornes d'une interpolation de `t` plutôt que deux fonctions :
///
/// | | `t = 0`, déployé | `t = 1`, replié |
/// | --- | --- | --- |
/// | hauteur | [`HEAD_HEIGHT`] (156 px) | [`HEAD_HEIGHT_COMPACT`] (56 px) |
/// | logo | [`LOGO_SIZE`] (56 px), centré | la moitié (28 px), à gauche |
/// | titre | sous le logo, centré | à droite du logo, sur la même ligne |
/// | badge | au-dessus du titre, à droite | après le titre |
///
/// Replié, l'en-tête rend **cent pixels** au contenu — de quoi lire une section de plus du volet
/// À propos sans défiler. C'est sa raison d'être, et celle des écrans défilants à venir.
fn paint_head(
    ui: &mut egui::Ui,
    card: Rect,
    t: f32,
    icons: &UiIcons,
    outcome: &mut LoginOutcome,
) -> f32 {
    let ctx = ui.ctx().clone();
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    let center_x = card.center().x;

    // Le titre d'abord : sa largeur place le logo replié comme le badge, dans les deux poses.
    let title_font = text::label_strong_font(&ctx, TITLE_SIZE);
    let overlay_font = text::label_font(&ctx, TITLE_SIZE);
    let title_main = "WAKFU COMPANION ";
    let title_overlay = "OVERLAY";
    let main_width = spaced_width(ui, title_main, &title_font, TITLE_SPACING);
    let overlay_width = spaced_width(ui, title_overlay, &overlay_font, TITLE_SPACING);
    let title_width = main_width + overlay_width;

    let logo_size = lerp(LOGO_SIZE, LOGO_SIZE_COMPACT);
    let logo_center = Pos2::new(
        lerp(
            center_x,
            card.left() + HEAD_PAD_COMPACT + LOGO_SIZE_COMPACT / 2.0,
        ),
        card.top() + lerp(HEAD_PAD_TOP + LOGO_SIZE / 2.0, HEAD_HEIGHT_COMPACT / 2.0),
    );
    egui::Image::new(icons.logo())
        .fit_to_exact_size(Vec2::splat(logo_size))
        .paint_at(
            ui,
            Rect::from_center_size(logo_center, Vec2::splat(logo_size)),
        );

    let deployed_title_top = card.top() + HEAD_PAD_TOP + LOGO_SIZE + 2.0 * HEAD_GAP + BETA_HEIGHT;
    let compact_title_left = card.left() + HEAD_PAD_COMPACT + LOGO_SIZE_COMPACT + HEAD_COMPACT_GAP;
    let title_left = lerp(center_x - title_width / 2.0, compact_title_left);
    let title_center_y = lerp(
        deployed_title_top + TITLE_LINE / 2.0,
        card.top() + HEAD_HEIGHT_COMPACT / 2.0,
    );
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

    // Badge « beta » : au-dessus du titre à droite déployé, après le titre replié.
    let beta_font = text::label_font(&ctx, BETA_HEIGHT);
    let beta_galley = ui.fonts_mut(|f| f.layout_no_wrap("beta".to_owned(), beta_font, BETA));
    let beta_size = beta_galley.rect.size();
    let beta_pos = Pos2::new(
        lerp(
            title_left + title_width - beta_size.x,
            title_left + title_width + HEAD_COMPACT_GAP,
        ),
        lerp(
            card.top() + HEAD_PAD_TOP + LOGO_SIZE + HEAD_GAP + (BETA_HEIGHT - beta_size.y) / 2.0,
            title_center_y - beta_size.y / 2.0 - BETA_LIFT_COMPACT,
        ),
    );
    paint_italic(ui, beta_pos, beta_galley, BETA);

    let height = lerp(HEAD_HEIGHT, HEAD_HEIGHT_COMPACT);
    // Poignée de déplacement : tout l'en-tête, du haut de la carte au séparateur.
    let head_rect = Rect::from_min_max(
        Pos2::new(card.left(), card.top()),
        Pos2::new(card.right(), card.top() + height),
    );
    let head = ui.interact(head_rect, ui.id().with("login-head"), Sense::drag());
    if head.drag_started() {
        outcome.drag_window = true;
    }
    height
}

/// **Y a-t-il quelque chose à effacer sur cette machine ?** — la question n'est posée au disque
/// qu'une fois par fenêtre, puis retenue (voir `LoginState::has_local_data`).
fn has_local_data(state: &mut LoginState) -> bool {
    *state.has_local_data.get_or_insert_with(|| {
        let answer = crate::local_data::has_user_data(crate::build_info::process_start());
        tracing::info!(
            "[données locales] sondage pour la Carte : {}.",
            if answer {
                "des données existent"
            } else {
                "rien à effacer"
            }
        );
        answer
    })
}

/// **Le volet « À propos »** (2026-09-22, demande utilisateur) — les mêmes textes que l'onglet
/// « À propos » de la fenêtre Options, dans le langage de la Carte.
///
/// Une seule source pour les phrases : [`super::a_propos_tab::SECTIONS`], plus la mention de
/// droits d'auteur et l'avertissement d'effacement du même module. **La fenêtre Options, elle, ne
/// change pas** : elle garde ses boutons de liens, et c'est la Carte qui porte sa propre table de
/// liens par section — vide pour « Vos données », dont les deux liens vivent déjà dans le pied,
/// visible en permanence sous le volet.
///
/// La zone défilante occupe tout le corps sauf la bande du bas, qui porte « Retour » : le bouton
/// reste donc visible quel que soit le défilement. Il a la largeur des autres boutons de la Carte
/// — le corps moins ses marges latérales — et **aucun fond derrière lui** : un fond de bande
/// repeignait [`CARD_FILL`] par-dessus lui-même, ce qui se voyait comme un rectangle plus opaque
/// courant jusqu'aux bordures latérales (2026-09-22, retour utilisateur).
fn paint_about(
    ui: &mut egui::Ui,
    body_rect: Rect,
    state: &mut LoginState,
    _phases: Phases,
    outcome: &mut LoginOutcome,
) {
    let backbar_top = body_rect.bottom() - BACKBAR_HEIGHT;
    let scroll_rect = Rect::from_min_max(
        body_rect.min,
        Pos2::new(body_rect.right(), backbar_top.max(body_rect.top())),
    );
    let mut url = None;
    let mut purge = false;
    let has_data = has_local_data(state);

    let mut scroll_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(scroll_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let output = egui::ScrollArea::vertical()
        .id_salt(ABOUT_SCROLL_ID)
        .auto_shrink([false; 2])
        // La barre est peinte à la main juste après — voir `paint_card_scrollbar`.
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(&mut scroll_ui, |ui| {
            ui.add_space(ABOUT_PAD_TOP);
            paint_about_content(ui, scroll_rect.width(), has_data, &mut url, &mut purge);
            ui.add_space(ABOUT_PAD_BOTTOM);
        });
    paint_card_scrollbar(ui, scroll_rect, &output);
    if let Some(url) = url {
        outcome.open_url = Some(url);
    }
    if purge {
        tracing::info!("[données locales] effacement demandé depuis le volet À propos.");
        state.about = false;
        state.purge_confirm = true;
    }

    // La bande du bas ne porte que le bouton : aucun fond à elle. La zone défilante s'arrête à
    // `backbar_top`, donc rien ne passe dessous — un fond n'aurait fait que repeindre `CARD_FILL`
    // par-dessus lui-même, d'où une bande plus opaque que le reste, jusqu'aux bordures latérales.
    let button_rect = Rect::from_min_max(
        Pos2::new(
            body_rect.left() + BODY_PAD_SIDE,
            backbar_top + BACKBAR_PAD_TOP,
        ),
        Pos2::new(
            body_rect.right() - BODY_PAD_SIDE,
            backbar_top + BACKBAR_PAD_TOP + BUTTON_HEIGHT,
        ),
    );
    if button(
        ui,
        button_rect,
        "Retour",
        ButtonKind::Secondary,
        "carte-a-propos-retour",
    ) {
        tracing::info!("[carte] « Retour » — volet À propos refermé.");
        state.about = false;
    }
}

/// Le contenu défilant du volet — voir [`paint_about`]. Peint dans le `Ui` de la zone défilable,
/// donc avec la mise en page d'egui et non les `y` absolus du reste de la Carte : c'est le seul
/// écran dont la hauteur n'est pas connue d'avance.
fn paint_about_content(
    ui: &mut egui::Ui,
    width: f32,
    has_local_data: bool,
    url: &mut Option<String>,
    purge: &mut bool,
) {
    use super::a_propos_tab::{self, Link, Section};
    use crate::design::InfoTone;

    let inner = width - ABOUT_PAD_SIDE - ABOUT_PAD_RIGHT;
    let ctx = ui.ctx().clone();
    let heading_font = text::label_strong_font(&ctx, ABOUT_HEADING_SIZE);
    let p_font = text::label_font(&ctx, P_SIZE);

    // La table de liens de la CARTE, section par section — celle de la fenêtre Options reste la
    // sienne (voir `paint_about`). « Vos données » n'en a plus : ses deux liens sont au pied.
    let links_of = |section: &Section| -> &'static [Link] {
        if section.title == a_propos_tab::WAKFU_COMPANION_TITLE {
            &[Link::Site, Link::Source]
        } else if section.title == a_propos_tab::VOS_DONNEES_TITLE {
            // Ses deux liens — politique de confidentialité, conditions d'utilisation — sont au
            // pied, visible en permanence sous le volet : les répéter ici ne dirait rien de plus.
            &[]
        } else {
            &[Link::WakfuTerms]
        }
    };

    let mut y = ui.cursor().top();
    let left = ui.max_rect().left() + ABOUT_PAD_SIDE;
    for (index, section) in a_propos_tab::SECTIONS.iter().enumerate() {
        if index > 0 {
            y += ABOUT_SECTION_GAP;
        }
        y = paint_paragraph(
            ui,
            Pos2::new(left, y),
            inner,
            section.title,
            &heading_font,
            TEXT,
            ABOUT_HEADING_LINE,
        ) + ABOUT_HEADING_GAP;
        for block in section.blocks {
            y = paint_about_block(ui, left, inner, y, block.text, block.tone, &p_font)
                + ABOUT_BLOCK_GAP;
        }
        // La mention de droits d'auteur d'Ankama porte l'année en cours : elle est calculée, pas
        // constante (voir `a_propos_tab::copyright_notice`).
        if section.title == a_propos_tab::WAKFU_COMPANION_TITLE {
            y = paint_about_block(
                ui,
                left,
                inner,
                y,
                &a_propos_tab::copyright_notice(a_propos_tab::copyright_year()),
                InfoTone::Info,
                &p_font,
            ) + ABOUT_BLOCK_GAP;
        }
        let section_links = links_of(section);
        if !section_links.is_empty() {
            y += ABOUT_LINKS_GAP;
            let mut x = left;
            for (i, link) in section_links.iter().enumerate() {
                if i > 0 {
                    ui.painter().text(
                        Pos2::new(x + FOOT_SEPARATOR_PAD, y),
                        Align2::LEFT_TOP,
                        FOOT_SEPARATOR,
                        text::label_font(&ctx, P_SIZE),
                        FOOT_SEPARATOR_COLOR,
                    );
                    x += 2.0 * FOOT_SEPARATOR_PAD
                        + link_width(ui, FOOT_SEPARATOR, P_SIZE, LinkKind::Internal);
                }
                let link = *link;
                x += link_at(
                    ui,
                    Pos2::new(x, y),
                    link.label(),
                    P_SIZE,
                    LinkKind::External,
                    true,
                    &format!("carte-a-propos-{}", link.label()),
                    || {
                        tracing::info!("[carte] « {} » (À propos) — page demandée.", link.label());
                        *url = Some(link.url());
                    },
                );
            }
            y += P_LINE;
        }
    }

    // **Le droit à l'effacement** (RGPD art. 17), sous son avertissement — et seulement s'il y a
    // quelque chose à effacer, comme sur l'écran « non connecté ».
    if has_local_data {
        y += ABOUT_SECTION_GAP;
        y = paint_about_block(
            ui,
            left,
            inner,
            y,
            a_propos_tab::PURGE_INFO,
            InfoTone::Alert,
            &p_font,
        ) + ABOUT_LINKS_GAP;
        let mut clicked = false;
        y += link_at(
            ui,
            Pos2::new(left, y),
            "Supprimer les données locales",
            P_SIZE,
            LinkKind::Danger,
            true,
            "carte-a-propos-effacer",
            || clicked = true,
        )
        .min(0.0)
            + P_LINE;
        if clicked {
            *purge = true;
        }
    }

    // La zone défilable doit connaître la hauteur du contenu : il est peint en absolu, donc rien
    // ne l'a allouée.
    let height = (y - ui.cursor().top()).max(0.0);
    ui.allocate_space(Vec2::new(width, height));
}

/// Un bloc d'information du volet : le texte, et pour un ton `Alert` la barre rouge et le fond
/// discret qui le détachent — ce que le design system du jeu fait avec sa pastille, transposé au
/// langage du site.
fn paint_about_block(
    ui: &mut egui::Ui,
    left: f32,
    width: f32,
    y: f32,
    body: &str,
    tone: crate::design::InfoTone,
    font: &FontId,
) -> f32 {
    use crate::design::InfoTone;

    let alert = tone == InfoTone::Alert;
    let (text_left, text_width, color) = if alert {
        (
            left + ALERT_BAR_WIDTH + ALERT_PAD_X,
            width - ALERT_BAR_WIDTH - 2.0 * ALERT_PAD_X,
            TEXT,
        )
    } else {
        (left, width, TEXT_MUTED)
    };
    let bottom = paint_paragraph(
        ui,
        Pos2::new(text_left, y + if alert { ALERT_PAD_Y } else { 0.0 }),
        text_width,
        body,
        font,
        color,
        P_LINE,
    );
    if !alert {
        return bottom;
    }
    let block = Rect::from_min_max(
        Pos2::new(left, y),
        Pos2::new(left + width, bottom + ALERT_PAD_Y),
    );
    ui.painter().rect_filled(block, 4.0, ALERT_FILL);
    ui.painter().rect_filled(
        Rect::from_min_max(
            block.min,
            Pos2::new(block.left() + ALERT_BAR_WIDTH, block.bottom()),
        ),
        0.0,
        ERROR_DOT,
    );
    // Le texte est déjà peint sous le fond : on le repeint par-dessus.
    paint_paragraph(
        ui,
        Pos2::new(text_left, y + ALERT_PAD_Y),
        text_width,
        body,
        font,
        color,
        P_LINE,
    );
    block.bottom()
}

/// **La barre de défilement de la Carte** : celle du site, pas celle du jeu, et peinte ici plutôt
/// que par egui.
///
/// `design::scroll_area` porte les textures 9-slice du jeu et réserve ses 26 px en permanence ;
/// celle d'egui, elle, se style mais ne se place pas — son rail occupe la largeur qu'il réserve,
/// et le relevé demande une **poignée qui flotte** : 6 px de large, coins arrondis, à 8 px du bord
/// de la Carte, sans rail ni flèche, l'accent au survol comme au glisser, et rien du tout quand le
/// contenu tient dans la hauteur. Une vingtaine de lignes suffisent à la poser exactement là.
///
/// Le glisser écrit directement le décalage dans l'état de la zone (`ScrollArea::State::store`) :
/// c'est ce que fait egui lui-même, et la molette continue de passer par lui.
fn paint_card_scrollbar(
    ui: &mut egui::Ui,
    scroll_rect: Rect,
    output: &egui::scroll_area::ScrollAreaOutput<()>,
) {
    let viewport = output.inner_rect.height();
    let content = output.content_size.y;
    let overflow = content - viewport;
    if overflow <= 0.5 || viewport <= 0.0 {
        return; // rien ne dépasse : pas de barre, et pas de réserve non plus
    }
    let track = Rect::from_min_max(
        Pos2::new(
            scroll_rect.right() - SCROLLBAR_MARGIN - SCROLLBAR_WIDTH,
            scroll_rect.top() + SCROLLBAR_MARGIN,
        ),
        Pos2::new(
            scroll_rect.right() - SCROLLBAR_MARGIN,
            scroll_rect.bottom() - SCROLLBAR_MARGIN,
        ),
    );
    let handle_height = (track.height() * viewport / content).max(SCROLLBAR_MIN_LENGTH);
    let travel = (track.height() - handle_height).max(0.0);
    let offset = output.state.offset.y.clamp(0.0, overflow);
    let handle = Rect::from_min_size(
        Pos2::new(track.left(), track.top() + travel * offset / overflow),
        Vec2::new(SCROLLBAR_WIDTH, handle_height),
    );
    let response = ui.interact(
        handle,
        ui.id().with("carte-a-propos-poignee"),
        Sense::click_and_drag(),
    );
    let color = if response.is_pointer_button_down_on() {
        ACCENT_HOVER
    } else if response.hovered() {
        ACCENT
    } else {
        SCROLLBAR_HANDLE
    };
    ui.painter()
        .rect_filled(handle, SCROLLBAR_WIDTH / 2.0, color);
    if response.dragged() && travel > 0.0 {
        let mut state = output.state;
        state.offset.y =
            (offset + response.drag_delta().y * overflow / travel).clamp(0.0, overflow);
        state.store(ui.ctx(), egui::Id::new(ABOUT_SCROLL_ID));
        ui.ctx().request_repaint();
    }
}

/// Séparateur gravé sous l'en-tête : 1 px sombre puis 1 px clair, sur la largeur du contenu.
fn paint_rule(ui: &egui::Ui, card: Rect, y: f32) {
    let left = card.left() + HEAD_PAD_SIDE;
    let right = card.right() - HEAD_PAD_SIDE;
    ui.painter().rect_filled(
        Rect::from_min_max(Pos2::new(left, y), Pos2::new(right, y + 1.0)),
        0.0,
        RULE_DARK,
    );
    ui.painter().rect_filled(
        Rect::from_min_max(Pos2::new(left, y + 1.0), Pos2::new(right, y + RULE_HEIGHT)),
        0.0,
        RULE_LIGHT,
    );
}

/// Le corps de la Carte : l'écran que son état commande, peint à partir de `top`. Rend le `y` du
/// bas de ce qu'il a peint — c'est cette hauteur que [`show`] mesure pour centrer le contenu dans
/// la zone de corps.
#[allow(clippy::too_many_arguments)]
fn paint_body(
    ui: &mut egui::Ui,
    card: Rect,
    top: f32,
    state: &mut LoginState,
    auth_status: &AuthStatus,
    auth_command_tx: &dyn AuthCommandSink,
    now: Instant,
    elapsed: Duration,
    phases: Phases,
    outcome: &mut LoginOutcome,
) -> f32 {
    let ctx = ui.ctx().clone();
    let center_x = card.center().x;
    let mut y = top;
    let body_left = card.left() + BODY_PAD_SIDE;
    let body_width = card.width() - 2.0 * BODY_PAD_SIDE;
    let h2_font = text::label_strong_font(&ctx, H2_SIZE);
    let p_font = text::label_font(&ctx, P_SIZE);
    // Le compte en cours de vérification (`Connecting`) est un chargement comme les autres : la
    // fenêtre ne dit « non connecté » qu'une fois la réponse connue.
    let loading = state.loading || matches!(auth_status, AuthStatus::Connecting);
    // Une mise à jour OBLIGATOIRE qui a échoué prend toute la place de l'écran de chargement :
    // la version en cours ne peut pas continuer, et « Réessayer » est la seule issue (le
    // démarrage reste bloqué par `StartupProgress::set_update_blocking`, voir
    // `background::download_and_stage`).
    if let UpdateStatus::Failed {
        headline,
        detail,
        mandatory: true,
    } = &state.update
    {
        y = paint_update_required(
            ui, body_left, body_width, y, headline, detail, phases, &h2_font, &p_font, outcome,
        );
    } else if state.manual_update {
        // Recherche de mise à jour demandée depuis le menu de la zone de notification : cet
        // écran occupe toute la carte jusqu'à ce que l'utilisateur le referme.
        y = paint_manual_update(
            ui, &ctx, card, body_left, body_width, y, state, phases, &h2_font, &p_font, outcome,
        );
    } else if loading {
        // Le corps prend la place qu'il occuperait sur l'écran « non connecté » (même hauteur
        // totale, voir `INITIAL_HEIGHT`), le rouage en son centre exact.
        let body_height =
            (INITIAL_HEIGHT - (y - card.top()) - BODY_PAD_BOTTOM - FOOT_HEIGHT).max(LOADER_SIZE);
        let body_rect =
            Rect::from_min_size(Pos2::new(body_left, y), Vec2::new(body_width, body_height));
        let loader_rect = Rect::from_center_size(body_rect.center(), Vec2::splat(LOADER_SIZE));
        let mut loader = design::loader()
            .size(design::LoaderSize::Px(LOADER_SIZE))
            .log_name("login-chargement");
        if !state.animate {
            loader = loader.preview_frame(0);
        }
        ui.put(loader_rect, loader);
        // L'avancement de la mise à jour, sous le rouage — rien tant qu'il n'y a rien à dire
        // (l'écran de chargement d'origine reste identique au pixel près).
        paint_update_progress(
            ui,
            &ctx,
            body_rect,
            loader_rect,
            &state.update,
            false,
            sheen_for(&state.update, phases),
        );
        y += body_height;
    } else {
        match auth_status {
            // **L'effacement des données locales, à confirmer** (2026-09-18, constat C5 de
            // `docs/analyse-rgpd.md` §3.5) — un écran de la carte, pas une boîte du jeu : voir
            // `LoginState::purge_confirm`. Il prend la place de l'écran « non connecté » tant que
            // la question est posée, et n'existe que là : un appairage en cours ou une erreur de
            // connexion ont leur propre geste à finir d'abord.
            AuthStatus::Disconnected { failure: None }
            | AuthStatus::Connecting
            | AuthStatus::Connected
                if state.purge_confirm =>
            {
                y = paint_paragraph(
                    ui,
                    Pos2::new(body_left, y),
                    body_width,
                    "Supprimer les données locales ?",
                    &h2_font,
                    TEXT,
                    H2_LINE,
                ) + H2_GAP;
                y = paint_paragraph(
                    ui,
                    Pos2::new(body_left, y),
                    body_width,
                    "L'overlay va effacer de cet ordinateur tout ce qu'il y a écrit : réglages, \
                 combats en cours, file d'envoi, gabarits de tour, journaux, caches et session \
                 enregistrée.",
                    &p_font,
                    TEXT_MUTED,
                    P_LINE,
                ) + P_GAP;
                y = paint_paragraph(
                    ui,
                    Pos2::new(body_left, y),
                    body_width,
                    "Il se fermera ensuite. Votre compte et son historique, eux, restent sur le \
                 site.",
                    &p_font,
                    TEXT_MUTED,
                    P_LINE,
                );
                y += ACTIONS_MARGIN_TOP;
                // Deux boutons côte à côte, à la disposition de l'écran d'appairage : l'action
                // qu'on est venu faire à DROITE en primaire (cyan du site), le retour en arrière
                // à gauche en secondaire. C'est l'utilisateur qui a demandé l'effacement en
                // arrivant ici — cet écran ne surgit jamais de lui-même —, et lui donner un
                // « Supprimer » effacé pour l'en dissuader mentirait sur ce que le geste engage :
                // ce qui protège du clic par inadvertance, c'est ce double écran, pas la
                // couleur.
                let half = (body_width - ACTIONS_GAP) / 2.0;
                let cancel_rect =
                    Rect::from_min_size(Pos2::new(body_left, y), Vec2::new(half, BUTTON_HEIGHT));
                let confirm_rect = Rect::from_min_size(
                    Pos2::new(body_left + half + ACTIONS_GAP, y),
                    Vec2::new(half, BUTTON_HEIGHT),
                );
                if button(
                    ui,
                    cancel_rect,
                    "Annuler",
                    ButtonKind::Secondary,
                    "login-effacement-annuler",
                ) {
                    tracing::info!("[données locales] effacement annulé.");
                    state.purge_confirm = false;
                }
                if button(
                    ui,
                    confirm_rect,
                    "Supprimer",
                    ButtonKind::Primary,
                    "login-effacement-confirmer",
                ) {
                    tracing::info!(
                        "[données locales] effacement confirmé depuis la fenêtre de connexion."
                    );
                    state.purge_confirm = false;
                    outcome.purge_local_data = true;
                }
                y += BUTTON_HEIGHT;
            }
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
                if button(
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
                // Information des personnes (art. 12-13 du RGPD, `docs/analyse-rgpd.md` §3.4) :
                // ce que la connexion engage est dit AVANT le geste. Les deux textes sont nommés,
                // et le pied les ouvre — voir `paint_consent_notice`.
                y += LINK_MARGIN_TOP;
                y += paint_consent_notice(ui, center_x, body_width, y);
                // **Le droit à l'effacement, exerçable ici** (RGPD art. 17,
                // `docs/analyse-rgpd.md` §3.5, constat C5) : sans compte lié, cette fenêtre est la
                // SEULE interface de l'overlay — la fenêtre Options, qui porte le même bouton sous
                // « Vos données » de l'onglet « À propos », est alors inatteignable. Un lien discret et non un bouton :
                // ce n'est pas ce qu'on vient faire sur cet écran, mais il faut pouvoir le faire
                // après s'être déconnecté, c'est-à-dire exactement ici.
                //
                // **Seulement s'il y a quelque chose à effacer** (2026-09-22) : voir
                // `LoginState::has_local_data` et `local_data::has_user_data`.
                if has_local_data(state) {
                    y += LINK_MARGIN_TOP;
                    let mut confirm = false;
                    y += link(
                        ui,
                        Pos2::new(center_x, y),
                        "Supprimer les données locales",
                        LinkKind::Danger,
                        "login-effacer-donnees",
                        || {
                            tracing::info!("[données locales] effacement demandé — confirmation.");
                            confirm = true;
                        },
                    );
                    if confirm {
                        state.purge_confirm = true;
                    }
                }
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
                let code_rect = Rect::from_min_size(
                    Pos2::new(body_left, y),
                    Vec2::new(body_width, code_height),
                );
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
                    LinkKind::Internal,
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
                y = paint_status_row(
                    ui,
                    &ctx,
                    body_left,
                    y,
                    "CONNEXION IMPOSSIBLE",
                    StatusTone::ERROR,
                    Some(phases.pulse),
                );
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
                y = paint_detail(ui, body_left, body_width, y, &failure.detail);
                // Deux boutons côte à côte, comme sur la carte d'appairage : « Réessayer », et
                // « Copier le détail » pour le support (constat C17 de `docs/analyse-rgpd.md`,
                // décision du 2026-09-19 : un bouton de copie, pas de bloc repliable) — le détail
                // tronqué à l'écran part entier dans le presse-papiers, titre compris, sans
                // recopie à la main ni capture d'écran.
                y += ACTIONS_MARGIN_TOP;
                let half = (body_width - ACTIONS_GAP) / 2.0;
                let retry_rect =
                    Rect::from_min_size(Pos2::new(body_left, y), Vec2::new(half, BUTTON_HEIGHT));
                let copy_rect = Rect::from_min_size(
                    Pos2::new(body_left + half + ACTIONS_GAP, y),
                    Vec2::new(half, BUTTON_HEIGHT),
                );
                if button(
                    ui,
                    retry_rect,
                    "Réessayer",
                    ButtonKind::Primary,
                    "login-reessayer",
                ) {
                    tracing::info!("[connexion] « Réessayer » — nouvelle tentative demandée.");
                    auth_command_tx.send(AuthCommand::Retry);
                }
                if button(
                    ui,
                    copy_rect,
                    "Copier le détail",
                    ButtonKind::Secondary,
                    "login-copier-detail",
                ) {
                    tracing::info!("[connexion] détail de l'échec copié.");
                    ctx.copy_text(failure_clipboard_text(&failure.headline, &failure.detail));
                }
                y += BUTTON_HEIGHT;
            }
        }
    }

    y
}

/// Ce qui se peint sous le rouage : le libellé de l'étape, sa couleur, et — pendant un
/// téléchargement seulement — la jauge (fraction 0→1) avec son compteur « 4,2 Mo / 11,8 Mo ».
type UpdateProgressLine = (String, Color32, Option<(f32, String)>);

/// Ce que l'écran de chargement dit de la mise à jour sous son rouage — une ligne, plus une jauge
/// pendant le téléchargement. Rien à peindre (`None`) quand l'état n'a rien à y dire.
///
/// `manual` distingue les deux écrans qui s'en servent :
///
/// - **écran de chargement** (`false`) : silencieux pour `Idle`, `Checking` (le rouage tourne
///   déjà, et le démarrage ne dit pas ce qu'il cherche) et `UpToDate` (rien à annoncer) ;
/// - **écran de mise à jour manuelle** (`true`, voir [`paint_manual_update`]) : la recherche EST
///   le sujet de l'écran, elle s'annonce donc ; les verdicts (`UpToDate`, `Available`,
///   `Unavailable`, `Failed`), eux, ne passent pas par ici — ils ont leur propre composition.
fn update_progress_line(status: &UpdateStatus, manual: bool) -> Option<UpdateProgressLine> {
    let line: UpdateProgressLine = match status {
        UpdateStatus::Available {
            version,
            mandatory: false,
            ..
        } => (
            format!("Version {version} disponible — Options › Paramètres"),
            TEXT_MUTED,
            None,
        ),
        UpdateStatus::Downloading {
            version,
            received,
            total,
        } => {
            let fraction = if *total == 0 {
                0.0
            } else {
                (*received as f32 / *total as f32).clamp(0.0, 1.0)
            };
            (
                format!("Téléchargement de la version {version}"),
                TEXT,
                Some((
                    fraction,
                    format!(
                        "{} / {}",
                        update::human_size(*received),
                        update::human_size(*total)
                    ),
                )),
            )
        }
        UpdateStatus::Verifying { version } => {
            (format!("Vérification de la version {version}…"), TEXT, None)
        }
        UpdateStatus::ReadyToInstall { version, .. } | UpdateStatus::Installing { version } => {
            (format!("Installation de la version {version}…"), TEXT, None)
        }
        UpdateStatus::Unavailable { .. } => (
            "Mise à jour : vérification impossible (hors ligne ?)".to_string(),
            TEXT_DIM,
            None,
        ),
        UpdateStatus::Failed {
            headline,
            mandatory: false,
            ..
        } => (
            format!("Mise à jour impossible ({headline}) — démarrage avec la version actuelle"),
            ERROR_TEXT,
            None,
        ),
        // La recherche elle-même : annoncée sur l'écran manuel, muette au démarrage.
        UpdateStatus::Idle | UpdateStatus::Checking if manual => {
            ("Recherche d'une mise à jour…".to_string(), TEXT, None)
        }
        UpdateStatus::Idle
        | UpdateStatus::Checking
        | UpdateStatus::UpToDate { .. }
        | UpdateStatus::Available { .. }
        | UpdateStatus::Failed { .. } => return None,
    };
    Some(line)
}

/// La phase du reflet de la jauge pour cet état : **seul un téléchargement qui avance en a un**.
/// Vérification, installation et échec n'ont pas de jauge du tout ; une jauge figée en garderait
/// une, et le reflet promettrait un progrès qui n'a plus lieu.
fn sheen_for(status: &UpdateStatus, phases: Phases) -> Option<f32> {
    matches!(status, UpdateStatus::Downloading { .. }).then_some(phases.sheen)
}

/// Peint sous le rouage ce que [`update_progress_line`] rend pour cet état — rien si elle ne rend
/// rien.
///
/// `sheen` est la phase du reflet de la jauge (voir [`paint_card_meter`]), `None` quand rien
/// n'avance : le reflet dit « ça continue », et son absence dit le contraire.
fn paint_update_progress(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    body_rect: Rect,
    loader_rect: Rect,
    status: &UpdateStatus,
    manual: bool,
    sheen: Option<f32>,
) {
    let Some((label, color, meter)) = update_progress_line(status, manual) else {
        return;
    };
    let font = text::label_font(ctx, UPDATE_LABEL_SIZE);
    let mut y = loader_rect.bottom() + UPDATE_LABEL_MARGIN_TOP;
    let galley = ui.fonts_mut(|f| {
        let mut job = LayoutJob::simple(label, font.clone(), color, body_rect.width());
        job.sections[0].format.line_height = Some(UPDATE_LABEL_LINE);
        job.halign = egui::Align::Center;
        f.layout_job(job)
    });
    ui.painter()
        .galley(Pos2::new(body_rect.center().x, y), galley.clone(), color);
    y += galley.rect.height();
    if let Some((fraction, count)) = meter {
        y += UPDATE_METER_MARGIN_TOP;
        let width = (body_rect.width() * UPDATE_METER_WIDTH_RATIO).round();
        let meter_rect = Rect::from_min_size(
            Pos2::new((body_rect.center().x - width / 2.0).round(), y),
            Vec2::new(width, UPDATE_METER_HEIGHT),
        );
        paint_card_meter(ui, meter_rect, fraction, sheen);
        y += UPDATE_METER_HEIGHT + UPDATE_COUNT_MARGIN_TOP;
        let count_font = text::label_font(ctx, UPDATE_COUNT_SIZE);
        let count_galley = ui.fonts_mut(|f| f.layout_no_wrap(count, count_font, TEXT_DIM));
        ui.painter().galley(
            Pos2::new(body_rect.center().x - count_galley.rect.width() / 2.0, y),
            count_galley,
            TEXT_DIM,
        );
    }
}

/// **La jauge de la Carte** — piste arrondie, remplissage, et un reflet qui le balaie.
///
/// Pas [`design::components::meter`] : celle-là est la jauge du JEU (deux bordures concentriques,
/// curseur de fin, reflet fixe au tiers supérieur), et cette fenêtre est du site — voir « Palette
/// du site, pas du jeu » dans la doc de module. Elle l'a pourtant empruntée jusqu'au 2026-09-22,
/// faute d'équivalent.
///
/// **Le reflet** (2026-09-22, demande utilisateur) : une bande claire de [`SHEEN_WIDTH_RATIO`] de
/// la largeur traverse la jauge de gauche à droite toutes les [`SHEEN_PERIOD`], écrêtée au
/// remplissage. Elle court même quand l'octet ne bouge pas — c'est précisément ce qu'elle dit,
/// « le téléchargement continue » — et `sheen: None` l'éteint dès qu'il s'arrête, en même temps
/// que la couleur du remplissage ([`METER_STALLED`]).
fn paint_card_meter(ui: &egui::Ui, rect: Rect, fraction: f32, sheen: Option<f32>) {
    let radius = rect.height() / 2.0;
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, radius, METER_TRACK);
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= 0.0 {
        return;
    }
    let fill_rect =
        Rect::from_min_size(rect.min, Vec2::new(rect.width() * fraction, rect.height()));
    let fill = if sheen.is_some() {
        ACCENT
    } else {
        METER_STALLED
    };
    painter.rect_filled(fill_rect, radius, fill);
    let Some(phase) = sheen else {
        return;
    };
    // La bande part entièrement à gauche du remplissage et le quitte entièrement à droite : elle
    // apparaît et disparaît par les bords, jamais au milieu.
    let band = (rect.width() * SHEEN_WIDTH_RATIO).max(1.0);
    let travel = fill_rect.width() + 2.0 * band;
    let left = fill_rect.left() - band + travel * phase.rem_euclid(1.0);
    let mut mesh = egui::epaint::Mesh::default();
    let clear = Color32::from_rgba_premultiplied(0, 0, 0, 0);
    let crest = Color32::from_rgba_premultiplied(191, 191, 191, 191); // blanc .75
    for (i, (x, color)) in [
        (left, clear),
        (left + band / 2.0, crest),
        (left + band, clear),
    ]
    .into_iter()
    .enumerate()
    {
        mesh.colored_vertex(Pos2::new(x, fill_rect.top()), color);
        mesh.colored_vertex(Pos2::new(x, fill_rect.bottom()), color);
        if i > 0 {
            let a = (i as u32 - 1) * 2;
            mesh.add_triangle(a, a + 1, a + 2);
            mesh.add_triangle(a + 1, a + 2, a + 3);
        }
    }
    ui.painter()
        .with_clip_rect(fill_rect)
        .add(egui::Shape::mesh(mesh));
}

/// **L'écran de mise à jour manuelle** (2026-09-18, demande de l'utilisateur) — ce que montre la
/// carte quand la recherche a été demandée depuis l'entrée « Mise à jour » du menu de la zone de
/// notification (`LoginState::manual_update`).
///
/// C'est **la même carte que la connexion et le démarrage** : même logo, même titre, même anneau,
/// même version en pied — l'utilisateur retrouve la fenêtre qu'il connaît, avec le rouage du jeu
/// pendant que la recherche court, puis son verdict :
///
/// | État | Corps |
/// | --- | --- |
/// | `Idle`, `Checking` | rouage + « Recherche d'une mise à jour… » |
/// | `Downloading`, `Verifying`, `ReadyToInstall`, `Installing` | rouage + l'étape en cours, jauge pendant le téléchargement |
/// | `UpToDate` | « Vous êtes déjà à jour » + « Fermer » |
/// | `Available` | « Version X disponible » + « Mettre à jour maintenant » et « Plus tard » |
/// | `Unavailable` | « Vérification impossible » (hors ligne ?) + « Rechercher à nouveau » et « Fermer » |
/// | `Failed` non obligatoire | « Mise à jour impossible » + le détail technique + « Réessayer » et « Fermer » |
///
/// Une mise à jour **obligatoire** en échec n'arrive jamais ici : elle a son propre écran, qui
/// passe devant (voir [`paint_update_required`] et `show`).
///
/// Rend la position sous le bloc — la carte se mesure elle-même et l'hôte taille la fenêtre OS à
/// ce qu'elle a occupé, comme pour tous les autres états.
#[allow(clippy::too_many_arguments)]
fn paint_manual_update(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    card: Rect,
    body_left: f32,
    body_width: f32,
    mut y: f32,
    state: &LoginState,
    phases: Phases,
    h2_font: &FontId,
    p_font: &FontId,
    outcome: &mut LoginOutcome,
) -> f32 {
    match &state.update {
        // ── Une opération court : le rouage, et ce qu'il est en train de faire ─────────────────
        // Exactement la composition de l'écran de chargement (même hauteur totale, rouage au
        // centre exact du corps) : passer de la recherche au téléchargement ne fait pas sauter la
        // fenêtre.
        UpdateStatus::Idle
        | UpdateStatus::Checking
        | UpdateStatus::Downloading { .. }
        | UpdateStatus::Verifying { .. }
        | UpdateStatus::ReadyToInstall { .. }
        | UpdateStatus::Installing { .. } => {
            let body_height = (INITIAL_HEIGHT - (y - card.top()) - BODY_PAD_BOTTOM - FOOT_HEIGHT)
                .max(LOADER_SIZE);
            let body_rect =
                Rect::from_min_size(Pos2::new(body_left, y), Vec2::new(body_width, body_height));
            let loader_rect = Rect::from_center_size(body_rect.center(), Vec2::splat(LOADER_SIZE));
            let mut loader = design::loader()
                .size(design::LoaderSize::Px(LOADER_SIZE))
                .log_name("login-mise-a-jour");
            if !state.animate {
                loader = loader.preview_frame(0);
            }
            ui.put(loader_rect, loader);
            paint_update_progress(
                ui,
                ctx,
                body_rect,
                loader_rect,
                &state.update,
                true,
                sheen_for(&state.update, phases),
            );
            y + body_height
        }
        // ── « Vous êtes déjà à jour » ──────────────────────────────────────────────────────────
        UpdateStatus::UpToDate { .. } => {
            y = paint_status_row(
                ui,
                ctx,
                body_left,
                y,
                "À JOUR",
                StatusTone::ACCENT,
                Some(phases.pulse),
            );
            y = paint_paragraph(
                ui,
                Pos2::new(body_left, y),
                body_width,
                "Vous êtes déjà à jour",
                h2_font,
                TEXT,
                H2_LINE,
            ) + H2_GAP;
            y = paint_paragraph(
                ui,
                Pos2::new(body_left, y),
                body_width,
                &format!(
                    "L'overlay utilise la dernière version publiée ({}). Il n'y a rien à \
                     installer.",
                    build_info::banner_label()
                ),
                p_font,
                TEXT_MUTED,
                P_LINE,
            );
            y += ACTIONS_MARGIN_TOP;
            if button(
                ui,
                Rect::from_min_size(
                    Pos2::new(body_left, y),
                    Vec2::new(body_width, BUTTON_HEIGHT),
                ),
                "Fermer",
                ButtonKind::Secondary,
                "login-maj-fermer",
            ) {
                outcome.close_update = true;
            }
            y + BUTTON_HEIGHT
        }
        // ── Une version est disponible : à l'utilisateur de dire quand ─────────────────────────
        UpdateStatus::Available {
            version,
            download_size,
            ..
        } => {
            y = paint_status_row(
                ui,
                ctx,
                body_left,
                y,
                "MISE À JOUR DISPONIBLE",
                StatusTone::ACCENT,
                Some(phases.pulse),
            );
            y = paint_paragraph(
                ui,
                Pos2::new(body_left, y),
                body_width,
                &format!("Version {version} disponible"),
                h2_font,
                TEXT,
                H2_LINE,
            ) + H2_GAP;
            y = paint_paragraph(
                ui,
                Pos2::new(body_left, y),
                body_width,
                &format!(
                    "L'overlay va télécharger {} puis se relancer pour terminer l'installation. \
                     Vos fenêtres de jeu se fermeront le temps de la mise à jour.",
                    update::human_size(*download_size)
                ),
                p_font,
                TEXT_MUTED,
                P_LINE,
            );
            y += ACTIONS_MARGIN_TOP;
            if button(
                ui,
                Rect::from_min_size(
                    Pos2::new(body_left, y),
                    Vec2::new(body_width, BUTTON_HEIGHT),
                ),
                "Mettre à jour maintenant",
                ButtonKind::Primary,
                "login-maj-installer",
            ) {
                tracing::info!(
                    "[mise à jour] « Mettre à jour maintenant » — téléchargement demandé."
                );
                outcome.install_update = true;
            }
            y += BUTTON_HEIGHT + LINK_MARGIN_TOP;
            let link_height = link(
                ui,
                Pos2::new(card.center().x, y),
                "Plus tard",
                LinkKind::Internal,
                "login-maj-plus-tard",
                || {
                    tracing::info!("[mise à jour] « Plus tard » — écran de mise à jour refermé.");
                    outcome.close_update = true;
                },
            );
            y + link_height
        }
        // ── Vérification impossible, ou mise à jour en échec ───────────────────────────────────
        UpdateStatus::Unavailable { reason, .. } => paint_manual_update_failure(
            ui,
            ctx,
            card,
            body_left,
            body_width,
            y,
            "VÉRIFICATION IMPOSSIBLE",
            "Impossible de vérifier les mises à jour",
            "L'overlay n'a pas pu lire le manifeste de version : le serveur n'a pas répondu comme \
             attendu. Vérifiez votre connexion internet, puis réessayez.",
            reason,
            ManualUpdateRetry::Check,
            phases,
            h2_font,
            p_font,
            outcome,
        ),
        UpdateStatus::Failed {
            headline, detail, ..
        } => paint_manual_update_failure(
            ui,
            ctx,
            card,
            body_left,
            body_width,
            y,
            "MISE À JOUR IMPOSSIBLE",
            headline,
            "La mise à jour n'a pas pu s'installer : l'overlay continue avec sa version actuelle. \
             Vérifiez votre connexion internet, puis réessayez.",
            detail,
            ManualUpdateRetry::Install,
            phases,
            h2_font,
            p_font,
            outcome,
        ),
    }
}

/// Ce que « Réessayer » relance sur un échec de l'écran de mise à jour manuelle.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ManualUpdateRetry {
    /// Vérification seule — la lecture du manifeste a échoué, il n'y a rien à installer encore.
    Check,
    /// Téléchargement et installation — le verdict était connu, c'est la suite qui a échoué.
    Install,
}

/// Les deux écrans d'échec de la mise à jour manuelle (vérification impossible, mise à jour
/// impossible) : même composition que l'écran d'erreur de connexion — point rouge, titre,
/// explication, détail technique — puis « Réessayer » et le lien « Fermer ». Rend la position
/// sous le bloc.
#[allow(clippy::too_many_arguments)]
fn paint_manual_update_failure(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    card: Rect,
    body_left: f32,
    body_width: f32,
    mut y: f32,
    status_label: &str,
    headline: &str,
    explanation: &str,
    detail: &str,
    retry: ManualUpdateRetry,
    phases: Phases,
    h2_font: &FontId,
    p_font: &FontId,
    outcome: &mut LoginOutcome,
) -> f32 {
    y = paint_status_row(
        ui,
        ctx,
        body_left,
        y,
        status_label,
        StatusTone::ERROR,
        Some(phases.pulse),
    );
    y = paint_paragraph(
        ui,
        Pos2::new(body_left, y),
        body_width,
        headline,
        h2_font,
        TEXT,
        H2_LINE,
    ) + H2_GAP;
    y = paint_paragraph(
        ui,
        Pos2::new(body_left, y),
        body_width,
        explanation,
        p_font,
        TEXT_MUTED,
        P_LINE,
    );
    y += DETAIL_MARGIN_TOP;
    y = paint_detail(ui, body_left, body_width, y, detail);
    y += ACTIONS_MARGIN_TOP;
    if button(
        ui,
        Rect::from_min_size(
            Pos2::new(body_left, y),
            Vec2::new(body_width, BUTTON_HEIGHT),
        ),
        "Réessayer",
        ButtonKind::Primary,
        "login-maj-reessayer",
    ) {
        tracing::info!("[mise à jour] « Réessayer » (écran de mise à jour) — nouvelle tentative.");
        match retry {
            ManualUpdateRetry::Check => outcome.check_update = true,
            ManualUpdateRetry::Install => outcome.install_update = true,
        }
    }
    y += BUTTON_HEIGHT + LINK_MARGIN_TOP;
    let link_height = link(
        ui,
        Pos2::new(card.center().x, y),
        "Fermer",
        LinkKind::Internal,
        "login-maj-fermer-echec",
        || {
            outcome.close_update = true;
        },
    );
    y + link_height
}

/// La ligne de statut d'un écran de la carte : pastille auréolée puis libellé en capitales, comme
/// « CONNEXION IMPOSSIBLE ». Rend la position sous la ligne, marge comprise.
///
/// **L'onde** (2026-09-22, demande utilisateur) : un anneau part du bord de la pastille, s'écarte
/// de [`STATUS_PULSE_REACH`] et s'efface, toutes les [`STATUS_PULSE_PERIOD`]. `pulse` est la phase
/// dans la période, ∈ [0, 1[ ; `None` fige l'onde à son départ — c'est ce que réclame le harnais de
/// captures, dont une frame doit finir par ne plus rien demander.
fn paint_status_row(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    body_left: f32,
    y: f32,
    label: &str,
    tone: StatusTone,
    pulse: Option<f32>,
) -> f32 {
    let font = text::label_strong_font(ctx, STATUS_SIZE);
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font, tone.text));
    let center_y = y + galley.rect.height() / 2.0;
    let center = Pos2::new(body_left + STATUS_DOT / 2.0, center_y);
    // L'onde passe SOUS l'auréole et la pastille : elle en sort, elle ne les recouvre pas.
    if let Some(phase) = pulse {
        let t = phase.rem_euclid(1.0) / STATUS_PULSE_DUTY;
        if t <= 1.0 {
            // Départ vif puis ralentissement — la courbe d'une onde qui se dissipe.
            let eased = 1.0 - (1.0 - t) * (1.0 - t);
            let alpha = STATUS_PULSE_ALPHA * (1.0 - t);
            ui.painter().circle_stroke(
                center,
                STATUS_DOT / 2.0 + STATUS_PULSE_REACH * eased,
                Stroke::new(1.5, tone.dot.gamma_multiply(alpha)),
            );
        }
    }
    ui.painter()
        .circle_filled(center, STATUS_DOT / 2.0 + 3.0, tone.halo);
    ui.painter().circle_filled(
        Pos2::new(body_left + STATUS_DOT / 2.0, center_y),
        STATUS_DOT / 2.0,
        tone.dot,
    );
    let height = galley.rect.height();
    ui.painter().galley(
        Pos2::new(body_left + STATUS_DOT + STATUS_GAP, y),
        galley,
        tone.text,
    );
    y + height + STATUS_MARGIN_BOTTOM
}

/// Les trois couleurs d'une ligne de statut — texte, pastille, auréole.
#[derive(Clone, Copy)]
struct StatusTone {
    text: Color32,
    dot: Color32,
    halo: Color32,
}

impl StatusTone {
    /// Ce qui a échoué : « CONNEXION IMPOSSIBLE », « MISE À JOUR REQUISE »…
    const ERROR: Self = Self {
        text: ERROR_TEXT,
        dot: ERROR_DOT,
        halo: ERROR_DOT_HALO,
    };
    /// Ce qui va bien, ou ce qui attend une décision : « À JOUR », « MISE À JOUR DISPONIBLE ».
    const ACCENT: Self = Self {
        text: ACCENT,
        dot: ACCENT,
        halo: ACCENT_HALO,
    };
}

/// L'encadré du détail technique (police à chasse fixe, une ligne, tronquée au besoin) — le même
/// sur l'écran d'erreur de connexion, « Mise à jour requise » et les échecs de mise à jour
/// manuelle. Rend la position sous l'encadré.
/// Ce que « Copier le détail » met dans le presse-papiers : le titre court puis le détail
/// technique complet, sur deux lignes — de quoi coller tel quel dans un message au support.
pub fn failure_clipboard_text(headline: &str, detail: &str) -> String {
    format!("{headline}\n{detail}")
}

fn paint_detail(ui: &mut egui::Ui, body_left: f32, body_width: f32, y: f32, detail: &str) -> f32 {
    let font = FontId::monospace(DETAIL_SIZE);
    let galley = ui.fonts_mut(|f| {
        let mut job = LayoutJob::simple(
            detail.to_owned(),
            font,
            TEXT_DIM,
            body_width - 2.0 * DETAIL_PAD_X,
        );
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        f.layout_job(job)
    });
    let height = galley.rect.height() + 2.0 * DETAIL_PAD_Y;
    let rect = Rect::from_min_size(Pos2::new(body_left, y), Vec2::new(body_width, height));
    ui.painter().rect_filled(rect, 4.0, DETAIL_FILL);
    ui.painter().rect_stroke(
        rect.shrink(0.5),
        4.0,
        Stroke::new(1.0, SECONDARY_BORDER),
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(
        Pos2::new(body_left + DETAIL_PAD_X, y + DETAIL_PAD_Y),
        galley,
        TEXT_DIM,
    );
    y + height
}

/// L'écran « Mise à jour requise » — même composition que l'écran d'erreur de connexion (point
/// rouge, titre, explication, détail technique, bouton), pour une mise à jour obligatoire qui n'a
/// pas pu s'installer. Rend la position sous le bloc.
#[allow(clippy::too_many_arguments)]
fn paint_update_required(
    ui: &mut egui::Ui,
    body_left: f32,
    body_width: f32,
    mut y: f32,
    headline: &str,
    detail: &str,
    phases: Phases,
    h2_font: &FontId,
    p_font: &FontId,
    outcome: &mut LoginOutcome,
) -> f32 {
    let ctx = ui.ctx().clone();
    y = paint_status_row(
        ui,
        &ctx,
        body_left,
        y,
        "MISE À JOUR REQUISE",
        StatusTone::ERROR,
        Some(phases.pulse),
    );
    y = paint_paragraph(
        ui,
        Pos2::new(body_left, y),
        body_width,
        headline,
        h2_font,
        TEXT,
        H2_LINE,
    ) + H2_GAP;
    y = paint_paragraph(
        ui,
        Pos2::new(body_left, y),
        body_width,
        "Cette version de l'overlay ne peut plus être utilisée avec le serveur : une mise à jour \
         est nécessaire, et elle n'a pas pu s'installer. Vérifiez votre connexion internet, puis \
         réessayez.",
        p_font,
        TEXT_MUTED,
        P_LINE,
    );
    y += DETAIL_MARGIN_TOP;
    y = paint_detail(ui, body_left, body_width, y, detail);
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
        "login-reessayer-mise-a-jour",
    ) {
        tracing::info!("[mise à jour] « Réessayer » — nouvelle tentative demandée.");
        outcome.retry_update = true;
    }
    y + BUTTON_HEIGHT
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

/// **Les trois familles de liens de la Carte** (2026-09-22, demande utilisateur) — un lien dit où
/// il mène avant d'être lu.
///
/// | Famille | Couleur | Ce qu'un clic fait |
/// | --- | --- | --- |
/// | [`LinkKind::External`] | accent `#00d2ff`, **flèche sortante** | ouvre le navigateur |
/// | [`LinkKind::Internal`] | gris `#b4bcc5` | change d'écran sans quitter la Carte |
/// | [`LinkKind::Danger`] | rouge `#ff8a83` | efface quelque chose |
///
/// La flèche est réservée à l'externe : rien ne s'ouvre ailleurs pour les deux autres. Elle est
/// **tracée à la main** ([`paint_external_arrow`]) parce qu'aucune fonte embarquée n'a « ↗ ».
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LinkKind {
    External,
    Internal,
    Danger,
}

impl LinkKind {
    /// Couleur au repos, puis au survol.
    fn colors(self) -> (Color32, Color32) {
        match self {
            LinkKind::External => (ACCENT, Color32::from_rgb(0x7c, 0xe6, 0xff)),
            LinkKind::Internal => (TEXT_DIM, TEXT),
            LinkKind::Danger => (ERROR_TEXT, Color32::from_rgb(0xff, 0xb0, 0xab)),
        }
    }
}

/// Flèche sortante d'un lien externe : la diagonale et les deux côtés du coin, dans le carré
/// `rect`. Trois segments, parce que les fontes embarquées (`assets/fonts`, Ubuntu Regular et
/// Medium) n'ont pas le caractère « ↗ » et qu'un carré blanc vaudrait pire que rien.
fn paint_external_arrow(ui: &egui::Ui, rect: Rect, color: Color32) {
    let s = rect.width();
    let at = |x: f32, y: f32| Pos2::new(rect.left() + s * x, rect.top() + s * y);
    let stroke = Stroke::new((s * 0.14).max(1.0), color);
    let painter = ui.painter();
    painter.line_segment([at(0.15, 0.85), at(0.85, 0.15)], stroke);
    painter.line_segment([at(0.33, 0.15), at(0.85, 0.15)], stroke);
    painter.line_segment([at(0.85, 0.15), at(0.85, 0.67)], stroke);
}

/// Largeur totale qu'occupera [`link_at`] pour ce libellé, flèche comprise.
fn link_width(ui: &egui::Ui, label: &str, size: f32, kind: LinkKind) -> f32 {
    let font = text::label_font(ui.ctx(), size);
    let width = ui
        .fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font, TEXT_DIM))
        .rect
        .width();
    if kind == LinkKind::External {
        width + LINK_ARROW_GAP + size * LINK_ARROW_RATIO
    } else {
        width
    }
}

/// Lien souligné dont le coin haut-gauche est `top_left` — rend sa largeur, appelle `on_click` au
/// clic. `enabled` à `false` le peint éteint et sourd : c'est ainsi que le pied traite le lien de
/// la section où l'on se trouve déjà, pour qu'aucun lien ne ramène là où l'on est.
#[allow(clippy::too_many_arguments)]
fn link_at(
    ui: &mut egui::Ui,
    top_left: Pos2,
    label: &str,
    size: f32,
    kind: LinkKind,
    enabled: bool,
    log_name: &str,
    on_click: impl FnOnce(),
) -> f32 {
    let font = text::label_font(ui.ctx(), size);
    let (rest, hover) = kind.colors();
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font, rest));
    let text_size = galley.rect.size();
    let width = link_width(ui, label, size, kind);
    let rect = Rect::from_min_size(top_left, Vec2::new(width, text_size.y));
    if !enabled {
        ui.painter().galley(rect.min, galley, LINK_DISABLED);
        return width;
    }
    let response = ui
        .interact(rect, ui.id().with(log_name), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let color = if response.hovered() { hover } else { rest };
    ui.painter().galley(rect.min, galley, color);
    let underline_y = rect.top() + text_size.y + 1.0;
    ui.painter().line_segment(
        [
            Pos2::new(rect.left(), underline_y),
            Pos2::new(rect.left() + text_size.x, underline_y),
        ],
        Stroke::new(1.0, color.gamma_multiply(UNDERLINE_ALPHA)),
    );
    if kind == LinkKind::External {
        let side = size * LINK_ARROW_RATIO;
        paint_external_arrow(
            ui,
            Rect::from_min_size(
                Pos2::new(
                    rect.left() + text_size.x + LINK_ARROW_GAP,
                    rect.top() + (text_size.y - side) / 2.0,
                ),
                Vec2::splat(side),
            ),
            color,
        );
    }
    if response.clicked() {
        on_click();
    }
    width
}

/// Lien souligné centré sur `top_center.x` — rend sa hauteur, appelle `on_click` au clic.
fn link(
    ui: &mut egui::Ui,
    top_center: Pos2,
    label: &str,
    kind: LinkKind,
    log_name: &str,
    on_click: impl FnOnce(),
) -> f32 {
    let width = link_width(ui, label, LINK_SIZE, kind);
    // Police résolue AVANT le verrou des fontes : `label_font` le prend aussi, et l'imbriquer
    // bloque le rendu — c'est le piège de tout ce module (voir les autres appels).
    let font = text::label_font(ui.ctx(), LINK_SIZE);
    let height = ui
        .fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font, TEXT_DIM))
        .rect
        .height();
    link_at(
        ui,
        Pos2::new(top_center.x - width / 2.0, top_center.y),
        label,
        LINK_SIZE,
        kind,
        true,
        log_name,
        on_click,
    );
    height + 2.0
}

/// **Le pied de la Carte** (2026-09-22, demande utilisateur) — présent sur tous les écrans, volet
/// À propos compris.
///
/// Deux lignes, et leur ordre n'est pas indifférent : **les actions de la Carte d'abord**, les
/// sorties vers le navigateur ensuite, du plus engageant au moins engageant et du plus court au
/// plus long.
///
/// 1. « Mise à jour » · « À propos » — internes, elles changent l'écran sans quitter la fenêtre ;
/// 2. « Conditions d'utilisation » · « Politique de confidentialité » — externes, avec flèche.
///
/// **Le lien de la section où l'on se trouve est éteint** : on ne revient pas à l'écran de mise à
/// jour depuis l'écran de mise à jour. La version reste en bas à droite, comme depuis toujours.
fn paint_foot(
    ui: &mut egui::Ui,
    card: Rect,
    top: f32,
    state: &mut LoginState,
    outcome: &mut LoginOutcome,
) {
    use super::a_propos_tab::Link;

    let ctx = ui.ctx().clone();
    let center_x = card.center().x;
    let mut y = top + FOOT_PAD_TOP;

    // Ligne 1 — les deux sections de la Carte. « Mise à jour » relance la recherche : c'est ce que
    // fait déjà l'entrée du menu de la zone de notification, et l'écran qui s'ensuit est le même.
    let on_update = state.manual_update;
    let on_about = state.about;
    let mut open_about = false;
    let mut check_update = false;
    foot_row(
        ui,
        center_x,
        y,
        &[
            ("Mise à jour", LinkKind::Internal, !on_update),
            ("À propos", LinkKind::Internal, !on_about),
        ],
        |index| match index {
            0 => {
                tracing::info!("[carte] « Mise à jour » (pied) — recherche demandée.");
                check_update = true;
            }
            _ => {
                tracing::info!("[carte] « À propos » (pied) — volet ouvert.");
                open_about = true;
            }
        },
    );
    if check_update {
        // C'est l'hôte qui ouvre l'écran de mise à jour (`PostRedraw::OpenManualUpdate`) : il
        // pose `manual_update`, que cette carte se contente de relire à chaque tick. Le volet, lui,
        // se referme ici — sinon il resterait par-dessus l'écran qu'on vient de demander.
        outcome.check_update = true;
        state.about = false;
    }
    if open_about {
        state.about = true;
    }

    // Ligne 2 — les deux textes du service, sur le site. Les mêmes libellés et les mêmes URL que
    // l'onglet « À propos » de la fenêtre Options (`a_propos_tab::Link`) : ils n'existent
    // qu'une fois.
    y += FOOT_LINE;
    let mut url = None;
    foot_row(
        ui,
        center_x,
        y,
        &[
            (Link::TermsOfService.label(), LinkKind::External, true),
            (Link::PrivacyPolicy.label(), LinkKind::External, true),
        ],
        |index| {
            let link = if index == 0 {
                Link::TermsOfService
            } else {
                Link::PrivacyPolicy
            };
            tracing::info!("[carte] « {} » (pied) — page demandée.", link.label());
            url = Some(link.url());
        },
    );
    if url.is_some() {
        outcome.open_url = url;
    }

    // La version, en bas à droite, au style exact du badge « beta » (demande du 2026-09-14 :
    // « en bas à droite, en italique et de la même taille que le mot beta »).
    let version_font = text::label_font(&ctx, VERSION_SIZE);
    let version_galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(build_info::banner_label().to_owned(), version_font, VERSION)
    });
    let version_size = version_galley.rect.size();
    paint_italic(
        ui,
        Pos2::new(
            card.right() - FOOT_PAD_SIDE - version_size.x,
            card.bottom() - FOOT_PAD_BOTTOM - version_size.y,
        ),
        version_galley,
        VERSION,
    );
}

/// Une ligne de liens du pied, centrée sur `center_x` et séparée de points médians. `on_click`
/// reçoit l'indice du lien cliqué.
fn foot_row(
    ui: &mut egui::Ui,
    center_x: f32,
    top: f32,
    links: &[(&str, LinkKind, bool)],
    mut on_click: impl FnMut(usize),
) {
    let font = text::label_font(ui.ctx(), FOOT_LINK_SIZE);
    let separator_width = ui
        .fonts_mut(|f| f.layout_no_wrap(FOOT_SEPARATOR.to_owned(), font.clone(), TEXT_DIM))
        .rect
        .width();
    let widths: Vec<f32> = links
        .iter()
        .map(|(label, kind, _)| link_width(ui, label, FOOT_LINK_SIZE, *kind))
        .collect();
    let total: f32 = widths.iter().sum::<f32>()
        + (links.len().saturating_sub(1)) as f32 * (separator_width + 2.0 * FOOT_SEPARATOR_PAD);
    let mut x = center_x - total / 2.0;
    let mut clicked = None;
    for (index, ((label, kind, enabled), width)) in links.iter().zip(&widths).enumerate() {
        if index > 0 {
            x += FOOT_SEPARATOR_PAD;
            ui.painter().text(
                Pos2::new(x, top),
                Align2::LEFT_TOP,
                FOOT_SEPARATOR,
                font.clone(),
                FOOT_SEPARATOR_COLOR,
            );
            x += separator_width + FOOT_SEPARATOR_PAD;
        }
        link_at(
            ui,
            Pos2::new(x, top),
            label,
            FOOT_LINK_SIZE,
            *kind,
            *enabled,
            &format!("carte-pied-{index}-{label}"),
            || clicked = Some(index),
        );
        x += width;
    }
    if let Some(index) = clicked {
        on_click(index);
    }
}

/// **La ligne d'acceptation sous « Se connecter »** — l'information due avant le geste (RGPD
/// art. 12-13, `docs/analyse-rgpd.md` §3.4, constat C4).
///
/// Une phrase, et plus deux liens (2026-09-22, demande utilisateur). Elle en portait jusque-là,
/// juste au-dessus des deux mêmes liens que le pied venait d'acquérir : « ça ne répète pas la même
/// chose, parce que là en plus c'est relativement court, on l'a l'un au-dessus de l'autre, c'est
/// vraiment perturbant ». Les deux textes sont donc **nommés en toutes lettres**, et c'est le pied
/// qui les ouvre — quelques dizaines de pixels plus bas, sur tous les écrans.
///
/// Onze points, en italique et plus gris que le corps : l'information reste donnée avant le geste,
/// sans prendre la place d'une action — elle occupe deux lignes au lieu des cinq de l'ancienne
/// composition. Rend la hauteur occupée.
fn paint_consent_notice(ui: &mut egui::Ui, center_x: f32, width: f32, top: f32) -> f32 {
    let font = text::label_font(ui.ctx(), CONSENT_SIZE);
    let galley = ui.fonts_mut(|f| {
        let mut job = LayoutJob::simple(CONSENT_NOTICE.to_owned(), font, CONSENT_TEXT, width);
        job.sections[0].format.line_height = Some(CONSENT_LINE);
        job.halign = egui::Align::Center;
        f.layout_job(job)
    });
    let height = galley.rect.height();
    // Italique simulé : `paint_italic` cisaille les sommets proportionnellement à leur hauteur
    // au-dessus de la ligne de base, ce qui vaut ligne par ligne dans un galley qui en a deux.
    paint_italic(ui, Pos2::new(center_x, top), galley, CONSENT_TEXT);
    height
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

/// **Le ton du liseré animé** (2026-09-22, demande utilisateur) — ce que la carte dit d'elle-même
/// avant qu'on lise un mot.
///
/// Trois états, et un seul par frame :
///
/// - [`RingStyle::Blue`] : le repos. C'est l'accent du site qui tourne en permanence — le blanc
///   translucide d'origine ne voulait rien dire, et la carte est bleue partout ailleurs ;
/// - [`RingStyle::Rainbow`] : **une mise à jour travaille** — recherche, téléchargement,
///   vérification, installation. Le spectre entier réparti sur le pourtour : c'est la seule chose
///   qui bouge quand le rouage tourne dans le vide ;
/// - [`RingStyle::Error`] : ce qui a échoué, connexion comme mise à jour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RingStyle {
    Blue,
    Rainbow,
    Error,
}

impl RingStyle {
    /// Couleur d'un point du pourtour, repéré par `u` — sa position le long du bord **rapportée au
    /// périmètre**, phase déjà retranchée (voir [`paint_ring`]).
    fn color_at(self, u: f32) -> Color32 {
        let t = u.rem_euclid(1.0) * 360.0;
        match self {
            // Le spectre complet, opaque : à 1,5 px d'épaisseur, une comète teintée ne se
            // distinguerait pas d'une comète grise.
            RingStyle::Rainbow => egui::ecolor::Hsva::new(t / 360.0, 1.0, 1.0, 1.0).into(),
            // La comète du site : transparente, puis un halo, puis la tête vive, puis la coupure.
            _ => {
                let (bright, soft) = match self {
                    RingStyle::Error => (
                        Color32::from_rgb(0xff, 0x5f, 0x57),
                        Color32::from_rgba_premultiplied(41, 15, 13, 46),
                    ),
                    _ => (ACCENT, Color32::from_rgba_premultiplied(0, 32, 38, 38)),
                };
                let clear = Color32::from_rgba_premultiplied(bright.r(), bright.g(), bright.b(), 0);
                if t < 200.0 {
                    lerp_color(clear, soft, t / 200.0)
                } else if t < 300.0 {
                    lerp_color(soft, bright, (t - 200.0) / 100.0)
                } else {
                    lerp_color(bright, clear, (t - 300.0) / 60.0)
                }
            }
        }
    }
}

/// Interpolation linéaire de deux couleurs prémultipliées.
fn lerp_color(a: Color32, b: Color32, k: f32) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
    Color32::from_rgba_premultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

/// Liseré lumineux autour de `card` : un dégradé qui **parcourt le bord**, réduit à 1,5 px.
///
/// Un `Mesh` à couleurs par sommet : le pourtour arrondi est échantillonné finement, chaque point
/// donne quatre sommets (deux de lisière transparents pour l'anticrénelage, deux de cœur colorés).
/// `phase` ∈ [0, 1[ est la fraction de tour déjà faite.
///
/// **Paramétré par la longueur d'arc, pas par l'angle** (2026-09-22). Le dégradé conique du web
/// colore chaque point d'après son angle vu du centre ; transposé tel quel, il donnait une comète
/// qui **expédiait les petits côtés et rampait sur les grands**. Sur une carte haute — chargement,
/// connexion, recherche, téléchargement — le bord du bas ne couvre qu'un mince secteur angulaire :
/// l'animation y passait en une fraction de seconde, au point que l'utilisateur l'a signalée comme
/// absente, alors qu'elle se voyait bien sur les cartes courtes (« à jour », « erreur »). En
/// repérant chaque point par la distance parcourue depuis le milieu du bord haut, divisée par le
/// périmètre, la tête avance à vitesse constante sur les quatre côtés, quelle que soit la hauteur.
fn paint_ring(ui: &egui::Ui, card: Rect, phase: f32, style: RingStyle) {
    const SAMPLES_PER_EDGE: usize = 24;
    const SAMPLES_PER_CORNER: usize = 16;
    const FEATHER: f32 = 0.75;
    // Le pseudo-élément déborde d'un pixel (`inset: -1px`) : l'anneau chevauche le liseré.
    let outer = card.expand(0.5);
    let radius = CARD_RADIUS + 0.5;

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

    // Longueur d'arc cumulée en chaque point, et périmètre total — voir la doc de la fonction.
    let count = points.len();
    let mut travelled = Vec::with_capacity(count);
    let mut total = 0.0_f32;
    for i in 0..count {
        travelled.push(total);
        total += (points[(i + 1) % count].0 - points[i].0).length();
    }
    if total <= 0.0 {
        return;
    }

    let mut mesh = egui::epaint::Mesh::default();
    let half = RING_WIDTH / 2.0;
    for (i, (p, normal)) in points.iter().enumerate() {
        let color = style.color_at(travelled[i] / total - phase);
        mesh.colored_vertex(*p + *normal * (half + FEATHER), Color32::TRANSPARENT);
        mesh.colored_vertex(*p + *normal * half, color);
        mesh.colored_vertex(*p - *normal * half, color);
        mesh.colored_vertex(*p - *normal * (half + FEATHER), Color32::TRANSPARENT);
    }
    let count = count as u32;
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
