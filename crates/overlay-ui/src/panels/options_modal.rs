//! Modale "Options" — voir §9.1 du plan d'architecture. Ouverte par le bouton "Options" du carré
//! de contrôle (`panels::watchlist::control_button_row`) ou le raccourci global `Ctrl+Shift+O`
//! (voir `main.rs`/`bin/overlay-ui-x11.rs`). Premier (et pour l'instant seul) réglage exposé
//! (onglet "Paramètres") : le chemin de `wakfu.log` à suivre.
//!
//! **Refonte 2026-09-09 — chrome basé sur les VRAIES textures du jeu, plus des formes peintes à la
//! main** (voir `panels::chamfer`, toujours utilisé ailleurs pour la barre de dégâts, mais plus
//! ici) : chaque mesure ci-dessous vient d'une capture d'écran réelle de la fenêtre Options du jeu
//! (`assets/design-system/interfaces/interface-options-*.png`, six onglets, chrome identique sur
//! les six), affinée par `.claude/skills/design-asset/scripts/dsimg.py analyze` puis VALIDÉE avec
//! l'utilisateur via une simulation HTML/CSS interactive avant ce portage (méthode explicitement
//! demandée : itérer en HTML, moins coûteux qu'itérer directement en Rust/egui, PUIS porter une
//! fois la maquette acceptée).
//!
//! Différences avec la toute première version (chamfrein peint à la main) :
//! - **Coins ARRONDIS, pas chanfreinés** — vérifié au pixel sur `modal-header.png` (rayon ≈12px) :
//!   la modale entière ET son encadré interne ("section") sont arrondis, ce dernier avec un rayon
//!   PLUS PRONONCÉ (18px) que la modale (12px) — constaté directement sur la référence, pas déduit.
//! - **Bannière/pied de page peints avec les vraies textures du jeu** (`modal-header.png`,
//!   `footer-cancel.png`/`footer-validate.png`, respectivement `large-button-cancel.png`/
//!   `large-button-validate.png` du design system) plutôt que des dégradés approximés à la main —
//!   leurs rayons de coin mesurés (2-4px) sont assez petits pour tolérer un étirement uniforme sans
//!   déformation perceptible (voir `panels::nine_slice`, doc de module, même raisonnement).
//! - **Menu à trois entrées** ("Alertes", "Personnages", "Paramètres") au-dessus de la section,
//!   texture `menu-tabs.png` (dérivée de `tabs-with-first-tab-active.png` du design system, miroir
//!   horizontal pour que le segment actif kaki tombe sur "Paramètres", dernière entrée) — seule
//!   "Paramètres" est câblée (contenu de cette modale), "Alertes"/"Personnages" restent des stubs
//!   visuels en attente d'un futur chantier.
//! - **Bouton "Sélectionner le fichier" en 9-slice** (`panels::nine_slice`) sous le champ (pas à
//!   côté) : seul élément ici qui doit s'agrandir bien au-delà de la taille native de sa texture
//!   (`browse-button.png`/`hover.png`, 169×52) sans aplatir son chanfrein/sa bordure.
//! - Fenêtre plus haute (`WINDOW_SIZE`, ratio aligné sur les 720:561 mesurés de la vraie fenêtre du
//!   jeu — demande explicite : « garder une cohérence par rapport au rendu [du jeu] »), section
//!   renommée "Fichier" (au lieu de "Fichier wakfu.log", redondant avec le contenu du champ), champ
//!   + bouton empilés verticalement (au lieu de côte à côte).
//!
//! **Pas de validation filesystem ICI** : cette fonction ne fait que peindre et renvoyer l'INTENTION
//! de l'utilisateur (`OptionsModalAction`) — c'est l'appelant (`main.rs`/`bin/overlay-ui-x11.rs`,
//! qui seuls savent comment déclencher un dialogue de fichier natif et parler au thread Engine) qui
//! valide via `overlay_ingest::discovery::validate_log_path` et alimente [`OptionsModalState::error`]
//! en retour pour le prochain redessin.

use crate::panels::nine_slice;

/// Taille de la fenêtre OS dédiée à cette modale (voir `main.rs::create_overlay_window`, cas
/// `OverlayKind::Options`) — largeur inchangée depuis la première version (560pt), hauteur portée
/// à 436pt pour retrouver le ratio 720:561 de la vraie fenêtre Options du jeu (560 × 561 / 720 ≈
/// 436), demande explicite de cohérence visuelle avec le rendu réel plutôt qu'une boîte compacte
/// arbitraire.
pub const WINDOW_SIZE: (f32, f32) = (560.0, 436.0);

/// Rayon d'arrondi de la modale ENTIÈRE (bannière haute + pied de page bas) — mesuré au pixel sur
/// `modal-header.png` (transition alpha au coin haut-gauche/haut-droit).
const MODAL_RADIUS: u8 = 12;
/// Rayon d'arrondi de l'encadré interne ("section", contenant le réglage de chemin) — visiblement
/// PLUS PRONONCÉ que `MODAL_RADIUS` sur la référence réelle, constat direct plutôt que déduit d'une
/// formule.
const SECTION_RADIUS: u8 = 18;

const BANNER_HEIGHT: f32 = 56.0;
/// Marge gauche/droite du contenu (`.body` de la simulation HTML validée) — hors bannière/pied de
/// page, qui restent pleine largeur.
const BODY_PAD_SIDE: f32 = 20.0;
/// Écart bannière → menu à onglets.
const BODY_PAD_TOP: f32 = 14.0;
/// Marge résiduelle sous le pied de page (bande sombre mesurée sous les boutons Annuler/Valider
/// sur la référence réelle, ≈12px/561) — évite que ces boutons touchent directement le bord bas
/// arrondi de la modale (retour utilisateur explicite : incohérence constatée sur ce point avec le
/// design system).
const BODY_PAD_BOTTOM: f32 = 12.0;

const MENU_HEIGHT: f32 = 31.0;
/// Écart menu → section.
const MENU_GAP: f32 = 14.0;
/// Écart section → pied de page.
const FOOTER_GAP: f32 = 14.0;

const SECTION_PAD_X: f32 = 22.0;
const SECTION_PAD_Y: f32 = 20.0;

const TITLE_TEXT: egui::Color32 = egui::Color32::WHITE;

// `panel_fill`/texte (docs/design-tokens.json::neutrals) — inchangés depuis la première version.
const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(0xF0, 0xF0, 0xF0);

/// Couleur du libellé d'un onglet INACTIF ("Alertes"/"Personnages") — ambre atténué, mesuré sur la
/// référence réelle. L'onglet ACTIF ("Paramètres") reprend `TITLE_TEXT` (blanc), comme le kaki
/// plein de la référence.
const TAB_INACTIVE_TEXT: egui::Color32 = egui::Color32::from_rgb(0xC9, 0xA8, 0x60);

// Champ de saisie (§5.4 du design-system) — bordure chaude systématique de tous les inputs.
const FIELD_FILL: egui::Color32 = egui::Color32::from_rgb(0x1C, 0x1E, 0x23);
const FIELD_BORDER: egui::Color32 = egui::Color32::from_rgb(0x59, 0x51, 0x40);
const FIELD_RADIUS: u8 = 2;

const ERROR_TEXT: egui::Color32 = egui::Color32::from_rgb(0xE0, 0x60, 0x55);

/// Fond de la modale (#1C2023) — légèrement translucide (laisse deviner le jeu derrière sur les
/// bords, comme la référence réelle) : valeur donnée par l'utilisateur au colorimètre, remplace la
/// première mesure automatique (plus sombre).
const MODAL_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(0x1C, 0x20, 0x23, 235);
/// Fond de la section interne (#13161B) — plus sombre que `MODAL_BG`, légèrement translucide.
const SECTION_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(0x13, 0x16, 0x1B, 230);

/// Marge sous le champ de chemin avant le bouton "Sélectionner le fichier" (empilés verticalement
/// — demande explicite, remplace le côte-à-côte de la première version).
const FIELD_TO_BROWSE_GAP: f32 = 8.0;
const FIELD_HEIGHT: f32 = 34.0;
const BROWSE_BUTTON_HEIGHT: f32 = 40.0;
/// `inset` du 9-slice du bouton "Sélectionner le fichier" (`nine_slice::nine_slice`, voir sa doc) —
/// dépasse largement le rayon de coin mesuré de `browse-button.png` (`corner_radius≈4`, `dsimg.py
/// analyze`) et son épaisseur de bordure, sans pour autant réduire à rien la zone étirable centrale
/// (texture native 169×52).
const BROWSE_NINE_SLICE_INSET: f32 = 14.0;

/// Textures embarquées du chrome de la modale — chargées UNE FOIS par fenêtre OS (voir
/// `main.rs`/`bin/overlay-ui-x11.rs`, `create_overlay_window`, même principe que
/// `panels::combat_frame::CombatFrame`/`crate::ui_icons::UiIcons`), jamais rechargées à chaque
/// frame. Fichiers sous `crates/overlay-ui/assets/ui/options/`, copiés depuis les assets validés
/// (`assets/design-system/`, voir la doc de module pour leur provenance).
pub struct OptionsModalAssets {
    /// `modal-header.png` (720×56) — fond de bannière, peint avec arrondi HAUT uniquement
    /// (`MODAL_RADIUS`) pour épouser le coin de la modale.
    banner: egui::TextureHandle,
    /// `footer-cancel.png`/`footer-validate.png` (338×36 chacun, libellé "Annuler"/"Valider"
    /// gravé dans la texture, comme la référence réelle) — pas de 9-slice ici : rayon de coin
    /// mesuré quasi nul (`corner_radius≈2`), l'étirement uniforme reste imperceptible.
    footer_cancel: egui::TextureHandle,
    footer_validate: egui::TextureHandle,
    /// `browse-button.png`/`browse-button-hover.png` (169×52 chacun, déjà "génériques" — aucun
    /// libellé gravé, voir `.claude/skills/design-asset`) — peints en 9-slice
    /// (`BROWSE_NINE_SLICE_INSET`), le seul élément de cette modale agrandi bien au-delà de sa
    /// taille native.
    browse: egui::TextureHandle,
    browse_hover: egui::TextureHandle,
    /// `menu-tabs.png` (782×44, dérivée de `tabs-with-first-tab-active.png` du design system par
    /// miroir horizontal — voir doc de module) — trois segments accolés, celui de droite (kaki)
    /// correspond à "Paramètres" (dernière entrée du menu, onglet actif).
    menu_tabs: egui::TextureHandle,
}

impl OptionsModalAssets {
    pub fn load(ctx: &egui::Context) -> Self {
        Self {
            banner: load_embedded_texture(
                ctx,
                "options-banner",
                include_bytes!("../../assets/ui/options/modal-header.png"),
            ),
            footer_cancel: load_embedded_texture(
                ctx,
                "options-footer-cancel",
                include_bytes!("../../assets/ui/options/footer-cancel.png"),
            ),
            footer_validate: load_embedded_texture(
                ctx,
                "options-footer-validate",
                include_bytes!("../../assets/ui/options/footer-validate.png"),
            ),
            browse: load_embedded_texture(
                ctx,
                "options-browse",
                include_bytes!("../../assets/ui/options/browse-button.png"),
            ),
            browse_hover: load_embedded_texture(
                ctx,
                "options-browse-hover",
                include_bytes!("../../assets/ui/options/browse-button-hover.png"),
            ),
            menu_tabs: load_embedded_texture(
                ctx,
                "options-menu-tabs",
                include_bytes!("../../assets/ui/options/menu-tabs.png"),
            ),
        }
    }
}

/// Décode + charge un PNG embarqué en texture egui — même séquence que
/// `panels::combat_frame::CombatFrame::load`/`crate::ui_icons::UiIcons::load` (`image` crate puis
/// `egui::ColorImage::from_rgba_unmultiplied`), extraite ici pour ne pas la répéter 6 fois.
fn load_embedded_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> egui::TextureHandle {
    let decoded = image::load_from_memory(bytes)
        .expect("asset PNG embarqué invalide — corrompu au build")
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        decoded.as_raw(),
    );
    ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR)
}

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
pub fn show(
    ui: &mut egui::Ui,
    state: &mut OptionsModalState,
    assets: &OptionsModalAssets,
) -> OptionsModalAction {
    let mut action = OptionsModalAction::None;
    let rect = ui.max_rect();

    // Fond de la modale — arrondi sur les QUATRE coins (`MODAL_RADIUS`), peint AVANT tout le reste
    // (bannière/section/pied de page viennent par-dessus).
    ui.painter().rect_filled(rect, MODAL_RADIUS, MODAL_BG);

    // Bannière — vraie texture du jeu (`modal-header.png`), arrondie sur les coins HAUTS
    // uniquement pour épouser le coin de la modale (les coins bas de la texture ne sont jamais
    // visibles, masqués par le corps qui la recouvre en dessous). Étirement uniforme : le rayon de
    // coin natif de cette texture est assez petit (≈12px/720) pour rester imperceptible, voir doc
    // de module.
    let banner_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), BANNER_HEIGHT));
    egui::Image::new(&assets.banner)
        .corner_radius(egui::CornerRadius {
            nw: MODAL_RADIUS,
            ne: MODAL_RADIUS,
            sw: 0,
            se: 0,
        })
        .paint_at(ui, banner_rect);
    // Titre agrandi + contour noir (retour utilisateur : « le titre "Options" doit être plus grand
    // et avoir un contour ») — même procédé que le texte flottant du panneau Combat, voir sa doc.
    super::combat::paint_outlined_text(
        ui,
        banner_rect.center(),
        egui::Align2::CENTER_CENTER,
        "Options",
        egui::FontId::proportional(26.0),
        TITLE_TEXT,
    );

    let content_rect = egui::Rect::from_min_max(
        egui::pos2(
            rect.left() + BODY_PAD_SIDE,
            banner_rect.bottom() + BODY_PAD_TOP,
        ),
        egui::pos2(
            rect.right() - BODY_PAD_SIDE,
            rect.bottom() - BODY_PAD_BOTTOM,
        ),
    );

    // Menu à trois entrées ("Alertes", "Personnages", "Paramètres") — texture réelle du jeu (trois
    // segments accolés, séparateur inclus), voir doc de module. Seule "Paramètres" (segment de
    // droite, actif) est câblée ; les deux autres sont des stubs visuels sans interaction pour
    // l'instant.
    let menu_rect = egui::Rect::from_min_size(
        content_rect.min,
        egui::vec2(content_rect.width(), MENU_HEIGHT),
    );
    egui::Image::new(&assets.menu_tabs).paint_at(ui, menu_rect);
    let tab_width = menu_rect.width() / 3.0;
    for (i, (label, color)) in [
        ("Alertes", TAB_INACTIVE_TEXT),
        ("Personnages", TAB_INACTIVE_TEXT),
        ("Paramètres", TITLE_TEXT),
    ]
    .into_iter()
    .enumerate()
    {
        let center =
            menu_rect.min + egui::vec2(tab_width * (i as f32 + 0.5), menu_rect.height() / 2.0);
        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(12.0),
            color,
        );
    }

    // Pied de page — vraies textures du jeu (libellé déjà gravé dedans), rangée pleine largeur du
    // contenu, chacune la moitié — hauteur dérivée du ratio natif (338×36) pour ne jamais déformer
    // verticalement l'assise du bouton.
    let footer_button_width = content_rect.width() / 2.0;
    let footer_height = footer_button_width * (36.0 / 338.0);
    let footer_rect = egui::Rect::from_min_max(
        egui::pos2(content_rect.left(), content_rect.bottom() - footer_height),
        content_rect.max,
    );
    let cancel_rect = egui::Rect::from_min_size(
        footer_rect.min,
        egui::vec2(footer_button_width, footer_height),
    );
    let validate_rect = egui::Rect::from_min_size(
        footer_rect.min + egui::vec2(footer_button_width, 0.0),
        egui::vec2(footer_rect.width() - footer_button_width, footer_height),
    );
    egui::Image::new(&assets.footer_cancel).paint_at(ui, cancel_rect);
    egui::Image::new(&assets.footer_validate).paint_at(ui, validate_rect);
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

    // Section "Fichier" — encadré interne au rayon plus prononcé que la modale (`SECTION_RADIUS`),
    // entre le menu et le pied de page.
    let section_rect = egui::Rect::from_min_max(
        egui::pos2(content_rect.left(), menu_rect.bottom() + MENU_GAP),
        egui::pos2(content_rect.right(), footer_rect.top() - FOOTER_GAP),
    );
    ui.painter()
        .rect_filled(section_rect, SECTION_RADIUS, SECTION_BG);

    let inner_rect = section_rect.shrink2(egui::vec2(SECTION_PAD_X, SECTION_PAD_Y));
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
        ui.label(
            egui::RichText::new("Fichier")
                .color(TEXT_PRIMARY)
                .size(13.5)
                .strong(),
        );
        ui.add_space(10.0);

        let field_rect = ui
            .allocate_space(egui::vec2(inner_rect.width(), FIELD_HEIGHT))
            .1;
        ui.painter()
            .rect_filled(field_rect, FIELD_RADIUS, FIELD_FILL);
        ui.painter().rect_stroke(
            field_rect,
            FIELD_RADIUS,
            egui::Stroke::new(1.0, FIELD_BORDER),
            egui::StrokeKind::Inside,
        );
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(field_rect.shrink(8.0)),
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

        ui.add_space(FIELD_TO_BROWSE_GAP);

        // Bouton "Sélectionner le fichier" — SOUS le champ (empilé verticalement, retour
        // utilisateur explicite), texture réelle du jeu agrandie en 9-slice (voir doc de module).
        let browse_rect = ui
            .allocate_space(egui::vec2(inner_rect.width(), BROWSE_BUTTON_HEIGHT))
            .1;
        let browse_response = ui
            .interact(
                browse_rect,
                ui.id().with("options-browse"),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let browse_texture = if browse_response.hovered() {
            &assets.browse_hover
        } else {
            &assets.browse
        };
        nine_slice::nine_slice(
            ui.painter(),
            browse_texture,
            browse_rect,
            BROWSE_NINE_SLICE_INSET,
        );
        ui.painter().text(
            browse_rect.center(),
            egui::Align2::CENTER_CENTER,
            "Sélectionner le fichier",
            egui::FontId::proportional(13.0),
            TITLE_TEXT,
        );
        if browse_response.clicked() {
            action = OptionsModalAction::Browse;
        }

        if let Some(err) = &state.error {
            ui.add_space(10.0);
            ui.label(egui::RichText::new(err).color(ERROR_TEXT).size(13.0));
        }
    });

    action
}
