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
//! - **Bannière peinte avec la vraie texture du jeu** (`modal-header.png`) plutôt qu'un dégradé
//!   approximé à la main — son rayon de coin mesuré (2-4px) est assez petit pour tolérer un
//!   étirement uniforme sans déformation perceptible.
//! - **Menu à trois entrées** ("Alertes", "Personnages", "Paramètres") au-dessus de la section,
//!   texture `menu-tabs.png` (dérivée de `tabs-with-first-tab-active.png` du design system, miroir
//!   horizontal pour que le segment actif kaki tombe sur "Paramètres", dernière entrée) — seule
//!   "Paramètres" est câblée (contenu de cette modale), "Alertes"/"Personnages" restent des stubs
//!   visuels en attente d'un futur chantier.
//! - **Bouton "Sélectionner le fichier"** sous le champ (pas à côté).
//! - Fenêtre plus haute (`WINDOW_SIZE`, ratio aligné sur les 720:561 mesurés de la vraie fenêtre du
//!   jeu — demande explicite : « garder une cohérence par rapport au rendu [du jeu] »), section
//!   renommée "Fichier" (au lieu de "Fichier wakfu.log", redondant avec le contenu du champ), champ
//!   + bouton empilés verticalement (au lieu de côte à côte).
//!
//! **Refonte 2026-09-09 (2) — les trois boutons passent sur `design::button`.** Ils étaient peints
//! ici à la main : deux images taillées sur mesure pour le pied de page (libellé gravé dedans, un
//! fichier par libellé) et un 9-slice local pour le bouton "Sélectionner le fichier". Ce sont
//! maintenant trois appels au composant du design system, qui apporte avec lui le survol, l'état
//! pressé, le curseur, l'écrêtage du libellé et la trace de journal. Ont disparu avec eux : les
//! quatre PNG de `assets/ui/options/` (tous des copies octet pour octet d'assets déjà déclarés au
//! manifeste `design::assets`) et le module `panels::nine_slice`, doublon de `design::nine_slice`
//! sans marges par côté ni mode de remplissage.
//!
//! Gain visible : le bouton "Sélectionner le fichier" est rendu à 700px de large depuis une texture
//! de 169px. L'ancien 9-slice ne figeait que 14px de chaque côté, bien moins que les 52px de décor
//! mesurés — les croisillons tombaient dans la bande médiane et s'étiraient sur près de 200px.
//!
//! **Pas de validation filesystem ICI** : cette fonction ne fait que peindre et renvoyer l'INTENTION
//! de l'utilisateur (`OptionsModalAction`) — c'est l'appelant (`main.rs`/`bin/overlay-ui-x11.rs`,
//! qui seuls savent comment déclencher un dialogue de fichier natif et parler au thread Engine) qui
//! valide via `overlay_ingest::discovery::validate_log_path` et alimente [`OptionsModalState::error`]
//! en retour pour le prochain redessin.

use crate::design::{self, ButtonSize, ButtonVariant};

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
/// Gouttière entre "Annuler" et "Valider" — le jeu en laisse 15px sur une fenêtre de 720
/// (`interface-options-jeu.png` : boutons en x 18..351 et 367..700), soit ≈12px à l'échelle de
/// cette modale. La première version les collait l'un à l'autre.
const FOOTER_GUTTER: f32 = 12.0;
/// Hauteur des boutons « Annuler » / « Valider ». **La hauteur native de leur texture** (338×36),
/// et non une hauteur déduite de leur largeur.
///
/// La première version écrivait `footer_button_width * (36.0 / 338.0)`, pour « préserver les
/// proportions de l'assise ». C'était l'inverse de ce que fait le jeu, et l'inverse de ce à quoi
/// sert un 9-slice : une texture 9-slice existe précisément pour qu'on l'étire en largeur SANS
/// toucher à sa hauteur. Le jeu affiche ce bouton à 333×36 dans une fenêtre de 720 ; notre modale
/// fait 560, ce qui donnait 250×27 — et comme le corps du libellé suit la hauteur
/// (`design::tokens::BUTTON_FONT_SIZE_RATIO`), le texte rétrécissait avec la fenêtre : 10px
/// d'encre au lieu de 13.
///
/// Que ce soit bien la hauteur, et non une mise à l'échelle générale de la modale, se vérifie sur
/// la capture : le bandeau de titre mesure 56px chez le jeu comme chez nous (`BANNER_HEIGHT`),
/// parce qu'il est peint à sa taille native. Le chrome était donc déjà à 100 %, seuls les boutons
/// rétrécissaient.
const FOOTER_BUTTON_HEIGHT: f32 = 36.0;
const FIELD_HEIGHT: f32 = 34.0;
const BROWSE_BUTTON_HEIGHT: f32 = 40.0;
/// Textures embarquées du chrome de la modale — chargées UNE FOIS par fenêtre OS (voir
/// `main.rs`/`bin/overlay-ui-x11.rs`, `create_overlay_window`, même principe que
/// `panels::combat_frame::CombatFrame`/`crate::ui_icons::UiIcons`), jamais rechargées à chaque
/// frame. Fichiers sous `crates/overlay-ui/assets/ui/options/`.
///
/// **Ne contient plus aucune texture de bouton** : elles sont au manifeste du design system
/// (`design::assets`), chargées paresseusement par `DesignSystem::get`, et aucun panneau n'a plus à
/// les câbler. Ne reste ici que le chrome propre à cette modale.
pub struct OptionsModalAssets {
    /// `modal-header.png` (720×56) — fond de bannière, peint avec arrondi HAUT uniquement
    /// (`MODAL_RADIUS`) pour épouser le coin de la modale.
    banner: egui::TextureHandle,
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

    // Pied de page — deux boutons du design system, séparés par la gouttière du jeu. Largeur
    // partagée, hauteur native (voir `FOOTER_BUTTON_HEIGHT`).
    let footer_button_width = (content_rect.width() - FOOTER_GUTTER) / 2.0;
    let footer_height = FOOTER_BUTTON_HEIGHT;
    let footer_rect = egui::Rect::from_min_max(
        egui::pos2(content_rect.left(), content_rect.bottom() - footer_height),
        content_rect.max,
    );
    let cancel_rect = egui::Rect::from_min_size(
        footer_rect.min,
        egui::vec2(footer_button_width, footer_height),
    );
    let validate_rect = egui::Rect::from_min_size(
        egui::pos2(footer_rect.right() - footer_button_width, footer_rect.top()),
        egui::vec2(footer_button_width, footer_height),
    );
    if ui
        .put(
            cancel_rect,
            design::button("Annuler")
                .variant(ButtonVariant::Danger)
                .size(ButtonSize::Height(footer_height))
                .width(footer_button_width)
                .log_name("options-annuler"),
        )
        .clicked()
    {
        action = OptionsModalAction::Cancel;
    }
    if ui
        .put(
            validate_rect,
            design::button("Valider")
                .variant(ButtonVariant::Primary)
                .size(ButtonSize::Height(footer_height))
                .width(footer_button_width)
                .log_name("options-valider"),
        )
        .clicked()
    {
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
        // utilisateur explicite), pleine largeur de la section.
        if ui
            .add(
                design::button("Sélectionner le fichier")
                    .variant(ButtonVariant::Secondary)
                    .size(ButtonSize::Height(BROWSE_BUTTON_HEIGHT))
                    .width(inner_rect.width())
                    .log_name("options-parcourir"),
            )
            .clicked()
        {
            action = OptionsModalAction::Browse;
        }

        if let Some(err) = &state.error {
            ui.add_space(10.0);
            ui.label(egui::RichText::new(err).color(ERROR_TEXT).size(13.0));
        }
    });

    action
}
