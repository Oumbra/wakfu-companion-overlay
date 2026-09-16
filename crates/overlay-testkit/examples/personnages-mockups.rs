//! **Onglet « Personnages » de la fenêtre Options — direction A, « Héros », détaillée geste par
//! geste.**
//!
//! ```bash
//! cargo run -p overlay-testkit --example personnages-mockups
//! ```
//!
//! Le premier jet (2026-09-14) proposait trois directions à comparer — A « Héros », B « Registre »,
//! C « Comptes empilés ». **A est retenue** ; B et C sont retirées de ce fichier plutôt que gardées
//! en code mort. Ce qui suit ne montre plus une direction mais **un écran et tous ses états** : ce
//! qu'on y voit au repos, et ce que chacune des sept actions y fait.
//!
//! ## Ce qui change depuis le premier jet
//!
//! 1. **Les portraits.** `crates/overlay-ui/assets/avatars/<classe>-<f|m>.png`, versés sur `dev` le
//!    2026-09-16 : 36 bustes **détourés** de 80 × 80 (fond transparent, coins arrondis), et non les
//!    médaillons carrés de 48 de `class-profile/` que `portraits::PortraitAtlas` sert au panneau
//!    Combat. Un buste détouré n'a pas de fond à lui : il se pose SUR la tuile, et c'est
//!    exactement l'anatomie des cartes du panneau de héros du jeu.
//! 2. **Plus de nom de classe sous le portrait** (demande explicite du 2026-09-16) : « on est dans
//!    le jeu, on connaît déjà les classes et leurs visuels ». Le bandeau ne porte plus que le nom
//!    du personnage — ce qui le libère et permet de le centrer.
//! 3. **Le sélecteur de classe devient une vraie modale** (`design::window` posée par-dessus la
//!    fenêtre Options voilée), et non plus un contenu qui remplace l'onglet.
//!
//! ## D'où vient chaque chose
//!
//! Le dépôt web (`Oumbra/wakfu-companion`, page « Profil › Personnages ») porte le système :
//! multi-comptes avec un compte principal ni renommable ni supprimable, serveur de jeu par compte,
//! formulaire d'ajout (portrait → sélecteur de classe, puis nom), renommage sur place, retrait avec
//! confirmation, réordonnancement au glisser-déposer, état vide. L'overlay n'en a qu'un **miroir en
//! lecture seule** (`overlay_engine::roster::RosterIndex`, alimenté par `GET /api/v1/settings`).
//!
//! Le jeu apporte la forme : grille de cartes, portrait porteur d'identité, **bandeau de nom pleine
//! largeur collé sous le portrait**, turquoise ou gris sombre. Le turquoise est ici **réaffecté au
//! personnage actif** — le dernier reconnu dans `wakfu.log`
//! (`overlay_engine::session::Engine::last_known_character`), que le site ne peut pas savoir.
//!
//! **Le niveau n'est PAS repris** : `wakfu.log` ne le porte nulle part (vérifié sur
//! `crates/overlay-engine/tests/wakfu.log` — les seuls « lvl 185 » du fichier sont des messages du
//! canal Recrutement), et `RosterCharacter` n'a pas de champ pour lui.
//!
//! ## Les planches
//!
//! | Planche | Ce qu'elle montre |
//! | --- | --- |
//! | `a1_ecran` | l'écran au repos, douze personnages |
//! | `a2_survol` | une tuile survolée : voile et croix de retrait |
//! | `a3_classe` | la modale de choix de classe, par-dessus l'onglet voilé |
//! | `a4_ajout` | classe choisie, nom saisi, « Ajouter » devenu actif |
//! | `a5_renommage` | le bandeau devenu champ de saisie |
//! | `a6_suppression` | la boîte de confirmation du retrait |
//! | `a7_deplacement` | un déplacement en vol : voile, fantôme, liseré or |
//! | `a8_vide` | compte lié, aucun personnage |
//! | `a9_sans_compte` | aucun compte lié |
//!
//! Les composants sont **appelés**, jamais recopiés : `design::window`, `tabs`, `panel`, `heading`,
//! `select`, `input`, `button`, `icon_button`, `info_text`, `confirm_dialog`, `tooltip`. Le chrome
//! est celui de la production, à sa taille de production. Les planches sont écrites dans
//! `target/mockups/`, jamais commitées.
//!
//! ## Ce que le portage demandera, et qui n'existe pas encore
//!
//! 1. **Un roster ÉDITABLE côté moteur.** `RosterIndex::from_settings_json` jette `id`, `label` et
//!    `isDefault` : il indexe pour résoudre une classe, il ne sait pas reconstruire le tableau.
//!    Or `PATCH /api/v1/settings` **remplace la valeur entière de la clé** — réécrire `roster`
//!    depuis l'overlay sans ces champs effacerait les comptes du site. Il faut garder le JSON brut,
//!    exactement comme `AccountSettings::profile_raw` le fait déjà pour `profile`.
//! 2. **`patch_roster`** dans `overlay_sync::client`, sur le modèle de `patch_chat_filters`.
//! 3. **Le personnage actif au snapshot** : `last_known_character` est privé à `Engine`.
//! 4. **Un chargeur d'avatars dans `overlay-ui`** — [`Avatars`] ci-dessous en est le brouillon,
//!    écrit ici parce qu'une maquette n'a pas à figer une API de la bibliothèque.
//! 5. **Le mode sans compte lié** : mêmes trois cas qu'`alerts_tab` (`Ready`/`Loading`/`NoAccount`).
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use std::collections::HashMap;

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Color32, Rect, RichText, Vec2};
use egui_kittest::Harness;
use overlay_engine::class_breed::CLASS_PORTRAIT_ORDER;
use overlay_engine::Gender;
use overlay_ui::design::{self, ButtonSize, ButtonVariant, DsIcon, IconContext, InputSize};
use overlay_ui::panels::options_modal::{self, OptionsTab};
use overlay_ui::ui_icons::UiIcons;

// -------------------------------------------------------------------------------------------
// Jetons — chacun dit d'où il vient. Repris TELS QUELS des onglets existants.
// -------------------------------------------------------------------------------------------

/// Fond derrière la fenêtre — le même que `tests/panels.rs`, `alertes-mockups.rs` et
/// `chat-mockups.rs`.
const BACKDROP: Color32 = Color32::from_rgb(0x0B, 0x0D, 0x10);

/// Taille de la fenêtre Options, celle de la production.
const WINDOW: Vec2 = Vec2::new(options_modal::WINDOW_SIZE.0, options_modal::WINDOW_SIZE.1);

const TEXT: Color32 = Color32::WHITE;
/// `panels::alerts_tab::SUBDUED`.
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);
/// `panels::alerts_tab::SECTION_GAP`.
const SECTION_GAP: f32 = 18.0;
/// `panels::alerts_tab::BODY_FONT_SIZE`.
const BODY_FONT_SIZE: f32 = 15.0;
/// `panels::alerts_tab::TILE_GAP`, le pas de la grille d'Alertes.
const TILE_GAP: f32 = 12.0;
/// Croix de retrait révélée au survol — `panels::alerts_tab::TILE_BADGE` / `TILE_BADGE_INSET`.
const TILE_BADGE: f32 = 14.0;
const TILE_BADGE_INSET: f32 = 8.0;
/// Voile posé sur une tuile survolée — `tokens::LEGEND_TILE_HOVER_SCRIM`.
const TILE_HOVER_SCRIM: Color32 = Color32::from_black_alpha(0x66);

/// **Le turquoise du jeu**, à sa mesure RÉELLE : `#1A6E80`, relevé au pixel sur
/// `interface-options-jeu.png` — et non le `banner_teal.gradient_end` `#1D8B9C` de
/// `design-tokens.json`, que le fichier signale lui-même comme une première mesure révisée depuis.
///
/// Ici il ne dit pas « sélectionné » mais **« actif » : le dernier personnage reconnu dans
/// `wakfu.log`**, ce que le jeu ne peut pas savoir et l'overlay si.
const ACTIVE_BAND: Color32 = Color32::from_rgb(0x1A, 0x6E, 0x80);
/// Le second état du même bandeau. `panels::alerts_tab::SETTING_ROW_FILL`, l'aplat de ligne de
/// réglage déjà posé par les deux onglets voisins — plutôt que le `#2A2E33` relevé sur les captures
/// du jeu, qui introduirait un troisième gris dans la même fenêtre pour un écart invisible.
const IDLE_BAND: Color32 = Color32::from_rgb(0x26, 0x28, 0x2B);

/// Cadre d'une tuile — ceux de `design::legend_tile` (`tokens::LEGEND_TILE_FILL` / `_BORDER`),
/// pour que la grille de Personnages et celle de Chat aient le même trait.
const TILE_FILL: Color32 = Color32::from_rgb(0x0E, 0x11, 0x15);
const TILE_BORDER: Color32 = Color32::from_rgb(0x59, 0x51, 0x40);
const TILE_BORDER_WIDTH: f32 = 2.0;
/// Le même cadre, éclairci, pour la tuile SURVOLÉE. Ajout au jeton d'Alertes : le voile seul
/// (`TILE_HOVER_SCRIM`) se lit sur une carte d'objet claire, pas sur un buste déjà sombre — la
/// tuile survolée y devenait presque indiscernable de ses voisines. Un cadre clair ≠ le cadre OR,
/// qui reste réservé à la tuile VISÉE par un déplacement.
const TILE_BORDER_HOVER: Color32 = Color32::from_rgb(0x9A, 0x8C, 0x6E);

/// Voile posé sur la place d'origine d'une tuile en vol — `panels::tile_reorder::DRAG_SOURCE_SCRIM`.
const DRAG_SOURCE_SCRIM: Color32 = Color32::from_black_alpha(0xAA);

const DESC: &str = "Déclarez les personnages de vos comptes : l'overlay les reconnaît dans le \
                    journal, les distingue des ennemis et rattache vos gains au bon compte.";

// -------------------------------------------------------------------------------------------
// Les avatars détourés — brouillon du chargeur qui ira dans `overlay-ui`
// -------------------------------------------------------------------------------------------

/// Côté natif des fichiers `assets/avatars/*.png`. **Peints à cette taille, jamais rééchantillonnés**
/// — c'est la leçon de `portraits.rs`, refondu en 36 textures indépendantes pour cette seule raison.
const AVATAR_SIZE: f32 = 80.0;

/// Les 36 bustes détourés, une texture par couple classe/sexe.
///
/// Lus **au disque** plutôt qu'`include_bytes!` : c'est un exemple, pas la bibliothèque, et une
/// macro de 18 lignes figerait ici une table que `overlay-ui` devra de toute façon écrire chez elle
/// au moment du portage. Panique si un fichier manque : c'est un asset absent du dépôt, pas un cas
/// d'exécution à tolérer.
struct Avatars {
    textures: HashMap<(&'static str, Gender), egui::TextureHandle>,
}

impl Avatars {
    fn load(ctx: &egui::Context) -> Self {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../overlay-ui/assets/avatars");
        let mut textures = HashMap::with_capacity(CLASS_PORTRAIT_ORDER.len() * 2);
        for class in CLASS_PORTRAIT_ORDER {
            for (gender, suffix) in [(Gender::F, 'f'), (Gender::M, 'm')] {
                let path = dir.join(format!("{class}-{suffix}.png"));
                let bytes = std::fs::read(&path)
                    .unwrap_or_else(|err| panic!("avatar {} illisible : {err}", path.display()));
                let decoded = image::load_from_memory(&bytes)
                    .unwrap_or_else(|err| panic!("avatar {} invalide : {err}", path.display()))
                    .to_rgba8();
                let (width, height) = decoded.dimensions();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width as usize, height as usize],
                    decoded.as_raw(),
                );
                textures.insert(
                    (class, gender),
                    ctx.load_texture(
                        format!("avatar-{class}-{suffix}"),
                        image,
                        egui::TextureOptions::LINEAR,
                    ),
                );
            }
        }
        Self { textures }
    }

    fn texture(&self, class: &str, gender: Gender) -> Option<egui::TextureId> {
        self.textures
            .iter()
            .find(|((name, sex), _)| *name == class && *sex == gender)
            .map(|(_, handle)| handle.id())
    }
}

// -------------------------------------------------------------------------------------------
// Les données de démonstration — les personnages réels de l'utilisateur, tels que ses captures du
// panneau de héros les montrent. Les classes sont déduites de leurs noms latins (« Scutum
// Tutelare » = bouclier protecteur = Féca, « Magister Thesauri » = maître du trésor = Enutrof…) :
// une fixture crédible vaut mieux que douze « Personnage 1 ».
// -------------------------------------------------------------------------------------------

struct Perso {
    name: &'static str,
    class: &'static str,
    gender: Gender,
}

const fn p(name: &'static str, class: &'static str, gender: Gender) -> Perso {
    Perso {
        name,
        class,
        gender,
    }
}

/// Le compte principal — douze personnages, de quoi remplir deux rangées et demie.
static COMPTE_PRINCIPAL: &[Perso] = &[
    p("Pugio Letalis", "sram", Gender::M),
    p("Sagitta Lucis", "cra", Gender::F),
    p("Ensis Orientalis", "iop", Gender::M),
    p("Canis Furiosus", "ouginak", Gender::M),
    p("Rota Metallica", "foggernaut", Gender::F),
    p("Imago Speculi", "zobal", Gender::F),
    p("Ignis Dolosus", "rogue", Gender::M),
    p("Monstrum Amoris", "osamodas", Gender::M),
    p("Bursa Auri", "enutrof", Gender::M),
    p("Penicillus Vitae", "eniripsa", Gender::F),
    p("Aegis Feminea", "feca", Gender::F),
    p("Arbovenenum", "sadida", Gender::M),
];

/// Le personnage **actif** : le dernier que `wakfu.log` a reconnu. Porte le bandeau turquoise.
const ACTIF: &str = "Sagitta Lucis";

/// Les 18 classes dans l'ordre du jeu, avec leur nom affiché — celui du sélecteur.
const CLASSES: &[(&str, &str)] = &[
    ("feca", "Féca"),
    ("osamodas", "Osamodas"),
    ("enutrof", "Enutrof"),
    ("sram", "Sram"),
    ("xelor", "Xélor"),
    ("ecaflip", "Ecaflip"),
    ("eniripsa", "Eniripsa"),
    ("iop", "Iop"),
    ("cra", "Crâ"),
    ("sadida", "Sadida"),
    ("sacrier", "Sacrieur"),
    ("pandawa", "Pandawa"),
    ("rogue", "Roublard"),
    ("zobal", "Zobal"),
    ("foggernaut", "Steamer"),
    ("eliotrope", "Éliotrope"),
    ("huppermage", "Huppermage"),
    ("ouginak", "Ouginak"),
];

// -------------------------------------------------------------------------------------------
// Sortie
// -------------------------------------------------------------------------------------------

fn mockup_dir() -> std::path::PathBuf {
    static PURGE: std::sync::Once = std::sync::Once::new();
    let dir = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(target) => std::path::PathBuf::from(target),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"),
    }
    .join("mockups");
    PURGE.call_once(|| {
        let _ = std::fs::remove_dir_all(&dir);
    });
    std::fs::create_dir_all(&dir).expect("création de target/mockups");
    dir
}

fn write_mockup(harness: &mut Harness<'static>, name: &str) {
    let image = harness
        .render()
        .expect("rendu offscreen — voir doc de module");
    image
        .save(mockup_dir().join(format!("{name}.png")))
        .expect("écriture de la capture");
}

// -------------------------------------------------------------------------------------------
// Le harnais — le chrome de production, onglet « Personnages » actif
// -------------------------------------------------------------------------------------------

/// Ce que l'appelant reçoit à chaque frame : de quoi peindre, et où.
struct Scene<'a> {
    icons: &'a UiIcons,
    avatars: &'a Avatars,
    panel: &'a design::PanelZones,
    /// La fenêtre entière — ce sur quoi une modale se pose.
    window: Rect,
}

fn options_harness(mut build: impl FnMut(&mut egui::Ui, Scene<'_>) + 'static) -> Harness<'static> {
    // Chargés UNE fois et gardés vivants entre les frames : un `TextureHandle` libère sa texture
    // dès que son dernier exemplaire tombe, et la planche sortirait avec des portraits vides.
    let mut icons: Option<UiIcons> = None;
    let mut avatars: Option<Avatars> = None;
    let mut tab = OptionsTab::Personnages;
    Harness::builder().with_size(WINDOW).build_ui(move |ui| {
        overlay_ui::style::apply(ui.ctx());
        // La bannière porte le numéro de version, que le hook `post-commit` incrémente à chaque
        // `feat:`/`fix:` — sans ce gel, chaque planche périmerait au commit suivant.
        overlay_ui::build_info::freeze_for_snapshots();
        ui.style_mut().visuals.text_cursor.blink = false;
        let icons = icons.get_or_insert_with(|| UiIcons::load(ui.ctx()));
        let avatars = avatars.get_or_insert_with(|| Avatars::load(ui.ctx()));
        egui::Frame::NONE.fill(BACKDROP).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let window = ui.max_rect();
            let chrome = design::window("Options")
                .footer("Annuler", "Valider")
                .close_button(true)
                .version(true)
                .log_name("maquette")
                .show(ui);
            chrome.tabs(
                ui,
                design::tabs(&mut tab)
                    .entry(OptionsTab::Suivi, "Suivi")
                    .entry(OptionsTab::Alertes, "Alertes")
                    .entry(OptionsTab::Chat, "Chat")
                    .entry(OptionsTab::Personnages, "Personnages")
                    .entry(OptionsTab::Raccourcis, "Raccourcis")
                    .entry(OptionsTab::Parametres, "Paramètres")
                    .log_name("maquette-onglets"),
            );
            design::panel().show(ui, chrome.content, |ui, panel| {
                build(
                    ui,
                    Scene {
                        icons,
                        avatars,
                        panel,
                        window,
                    },
                );
            });
        });
    })
}

// -------------------------------------------------------------------------------------------
// Briques communes
// -------------------------------------------------------------------------------------------

/// Un paragraphe, pas un bloc d'information — même règle que `panels::alerts_tab::paragraph`.
fn paragraph(ui: &mut egui::Ui, text: &str) {
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .color(TEXT)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        )
        .wrap_mode(egui::TextWrapMode::Wrap),
    );
}

/// Texte d'une ligne, ellipsé s'il déborde — jamais rogné en silence.
#[allow(clippy::too_many_arguments)]
fn painted_ellipsed(
    ui: &egui::Ui,
    painter: &egui::Painter,
    text: &str,
    pos: egui::Pos2,
    max_width: f32,
    font: egui::FontId,
    color: Color32,
    centered: bool,
) -> f32 {
    let mut job = LayoutJob {
        wrap: TextWrapping {
            max_width,
            max_rows: 1,
            break_anywhere: true,
            overflow_character: Some('…'),
        },
        ..Default::default()
    };
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font,
            color,
            ..Default::default()
        },
    );
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let size = galley.size();
    let origin = if centered {
        egui::pos2(pos.x - size.x / 2.0, pos.y - size.y / 2.0)
    } else {
        egui::pos2(pos.x, pos.y - size.y / 2.0)
    };
    painter.galley(origin, galley, color);
    size.x
}

/// Le buste détouré d'un personnage, peint à sa taille NATIVE, centré dans `rect`. Repli sur le
/// portrait générique d'`UiIcons` quand la classe est inconnue — jamais un trou.
fn paint_avatar(
    ui: &egui::Ui,
    rect: Rect,
    class: &str,
    gender: Gender,
    avatars: &Avatars,
    icons: &UiIcons,
) {
    let texture = avatars
        .texture(class, gender)
        .unwrap_or_else(|| icons.unknown_entity_texture().id());
    ui.painter().image(
        texture,
        rect,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// La croix de retrait révélée au survol d'une tuile — l'idiome des tuiles d'Alertes et de Chat.
/// `scrim` est la zone réellement voilée : elle s'arrête au bandeau de nom, qui doit rester lisible
/// à l'instant précis où le pointeur l'atteint.
fn hover_badge(ui: &mut egui::Ui, rect: Rect, scrim: Rect, log_name: String) {
    let ctx = ui.ctx().clone();
    let painter = ui.painter().clone();
    painter.rect_filled(scrim.shrink(TILE_BORDER_WIDTH), 0.0, TILE_HOVER_SCRIM);
    let ds = design::DesignSystem::get(&ctx);
    let native = ds.icon_native_size(DsIcon::Close);
    let side = native.x.max(native.y);
    let center = egui::pos2(
        rect.right() - TILE_BADGE_INSET - TILE_BADGE / 2.0,
        rect.top() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
    );
    let icon_rect = Rect::from_center_size(center, native * (TILE_BADGE / side));
    let croix = ui.interact(
        Rect::from_center_size(center, Vec2::splat(TILE_BADGE + 8.0)),
        egui::Id::new(log_name),
        egui::Sense::click(),
    );
    let tint = if croix.hovered() {
        design::tokens::INFO_ALERT
    } else {
        design::tokens::ICON_TINT
    };
    ds.paint_icon(&painter, icon_rect, DsIcon::Close, tint);
}

/// La ligne « compte + serveur », au-dessus de la grille. Les deux boutons de droite portent sur le
/// COMPTE, jamais sur un personnage : « + » en ajoute un, la corbeille retire celui qui est choisi
/// — désactivée sur le compte principal, que le web interdit de supprimer (`isDefault`).
fn account_row(ui: &mut egui::Ui, compte: &mut usize, serveur: &mut usize, width: f32) {
    // La ligne n'est pas dans la zone défilable : elle reprend la réserve de barre, comme le
    // formulaire d'ajout de `chat_tab`.
    let total =
        width + design::components::scroll_area::RESERVE_X - design::tokens::PANEL_PAD_CONTROL_X;
    let row = ui
        .allocate_space(Vec2::new(total, design::tokens::SELECT_HEIGHT))
        .1;
    let principal = *compte == 0;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    cell.spacing_mut().item_spacing.x = 0.0;
    cell.horizontal_centered(|ui| {
        ui.label(
            RichText::new("Compte")
                .color(SUBDUED)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        );
        ui.add_space(10.0);
        design::select(compte)
            .option(0usize, "Principal")
            .option(1usize, "Mules")
            .option(2usize, "Métiers")
            .width(210.0)
            .log_name("personnages.compte")
            .show(ui);
        ui.add_space(20.0);
        ui.label(
            RichText::new("Serveur")
                .color(SUBDUED)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        );
        ui.add_space(10.0);
        design::select(serveur)
            .option(0usize, "Pandora")
            .option(1usize, "Rubilax")
            .option(2usize, "Aucun")
            .width(170.0)
            .log_name("personnages.serveur")
            .show(ui);

        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        right.add(
            design::icon_button(DsIcon::Delete)
                .context(IconContext::Panel)
                .enabled(!principal)
                .tooltip(if principal {
                    "Le compte principal ne se supprime pas"
                } else {
                    "Supprimer ce compte"
                })
                .log_name("personnages.compte.retirer"),
        );
        right.add_space(design::tokens::PANEL_PAD_CONTROL_X);
        right.add(
            design::icon_button(DsIcon::Plus)
                .context(IconContext::Panel)
                .tooltip("Ajouter un compte")
                .log_name("personnages.compte.ajouter"),
        );
    });
}

/// Le formulaire d'ajout : **portrait, nom, « Ajouter » — dans cet ordre**, celui du web
/// (`character-add-form.component.html`). Le portrait est un bouton : il ouvre la modale de choix
/// de classe et porte ensuite le buste choisi.
fn add_row(
    ui: &mut egui::Ui,
    nom: &mut String,
    choisie: Option<(&str, Gender)>,
    avatars: &Avatars,
    icons: &UiIcons,
    width: f32,
) {
    // 48 et non la hauteur d'un champ : un buste de 80 réduit à 32 n'est plus qu'une tache. La
    // ligne s'aligne donc sur le portrait, pas l'inverse — les champs y restent centrés.
    const FACE: f32 = 48.0;
    const ADD_WIDTH: f32 = 100.0;
    const GAP: f32 = 12.0;
    let total =
        width + design::components::scroll_area::RESERVE_X - design::tokens::PANEL_PAD_CONTROL_X;
    let row = ui.allocate_space(Vec2::new(total, FACE)).1;
    let face = Rect::from_min_size(
        egui::pos2(row.left(), row.center().y - FACE / 2.0),
        Vec2::splat(FACE),
    );
    let response = ui.interact(
        face,
        egui::Id::new("personnages.classe"),
        egui::Sense::click(),
    );
    ui.painter()
        .rect_filled(face, design::tokens::INPUT_RADIUS, TILE_FILL);
    ui.painter().rect_stroke(
        face,
        design::tokens::INPUT_RADIUS,
        egui::Stroke::new(
            TILE_BORDER_WIDTH,
            if response.hovered() {
                design::tokens::TEXT_GOLD
            } else {
                TILE_BORDER
            },
        ),
        egui::StrokeKind::Inside,
    );
    match choisie {
        Some((class, gender)) => paint_avatar(ui, face.shrink(2.0), class, gender, avatars, icons),
        None => {
            let ds = design::DesignSystem::get(ui.ctx());
            let native = ds.icon_native_size(DsIcon::Characters);
            let side = native.x.max(native.y);
            ds.paint_icon(
                ui.painter(),
                Rect::from_center_size(face.center(), native * (18.0 / side)),
                DsIcon::Characters,
                design::tokens::INPUT_ICON,
            );
        }
    }
    design::tooltip(&response).text("Choisir la classe");

    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(
        egui::pos2(row.left() + FACE + GAP, row.top()),
        row.max,
    )));
    cell.spacing_mut().item_spacing.x = 0.0;
    cell.horizontal_centered(|ui| {
        ui.add(
            design::input(nom)
                .size(InputSize::Search)
                .placeholder("Nom du personnage…")
                .width(total - FACE - ADD_WIDTH - GAP * 2.0)
                .log_name("personnages.nom"),
        );
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(
                    egui::pos2(row.left() + FACE + GAP, row.top()),
                    row.max,
                ))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        right.add(
            design::button("Ajouter")
                .variant(ButtonVariant::Primary)
                .size(ButtonSize::Compact)
                .width(ADD_WIDTH)
                .enabled(choisie.is_some() && !nom.is_empty())
                .log_name("personnages.ajouter"),
        );
    });
}

// -------------------------------------------------------------------------------------------
// La grille de héros
// -------------------------------------------------------------------------------------------

/// Tuiles par rangée. **Cinq** : à ~675 pt de large utile, une tuile fait 125 pt pour un buste de
/// 80 — le rapport des cartes du jeu. Quatre les rendrait obèses, six couperait un nom sur deux.
const A_TILES_PER_ROW: usize = 5;
/// Hauteur d'une tuile : 2 de bordure + 4 + 80 de buste + 24 de bandeau + 2 de bordure.
const A_TILE_HEIGHT: f32 = 112.0;
/// Hauteur du bandeau de nom, relevée sur les captures du panneau de héros (~20 px), remontée à 24
/// pour que le nom respire maintenant qu'il y est seul.
const A_BAND: f32 = 24.0;

/// Ce que la planche montre — un seul état à la fois, forcé plutôt que simulé au pointeur : une
/// position de souris en dur dans une planche se décale au premier ajustement de cote.
#[derive(Clone, Copy, PartialEq)]
enum Etat {
    Repos,
    /// Tuile survolée : voile et croix de retrait.
    Survol(usize),
    /// Bandeau devenu champ de saisie.
    Renommage(usize),
    /// Déplacement en vol — `pris` a quitté sa place, `vise` est la destination sous le pointeur.
    Deplacement {
        pris: usize,
        vise: usize,
    },
}

impl Etat {
    fn survol(self, index: usize) -> bool {
        matches!(self, Etat::Survol(i) if i == index)
    }
    fn renomme(self, index: usize) -> bool {
        matches!(self, Etat::Renommage(i) if i == index)
    }
}

/// Le bandeau de nom d'une tuile — turquoise si le personnage est celui que le journal a reconnu en
/// dernier, gris sinon. **Pleine largeur intérieure, collé au bord bas**, comme les cartes du jeu.
fn band_rect(tile: Rect) -> Rect {
    Rect::from_min_max(
        egui::pos2(
            tile.left() + TILE_BORDER_WIDTH,
            tile.bottom() - TILE_BORDER_WIDTH - A_BAND,
        ),
        egui::pos2(
            tile.right() - TILE_BORDER_WIDTH,
            tile.bottom() - TILE_BORDER_WIDTH,
        ),
    )
}

/// Une carte de héros : buste détouré posé sur le fond de la tuile, **bandeau de nom seul** collé
/// en bas. Plus de nom de classe : le portrait la dit déjà, et dans le jeu on la connaît.
#[allow(clippy::too_many_arguments)]
fn hero_tile(
    ui: &mut egui::Ui,
    perso: &Perso,
    index: usize,
    size: Vec2,
    etat: Etat,
    brouillon: &mut String,
    avatars: &Avatars,
    icons: &UiIcons,
) -> Rect {
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let painter = ui.painter().clone();
    let actif = perso.name == ACTIF;
    let vise = matches!(etat, Etat::Deplacement { vise, .. } if vise == index);
    let pris = matches!(etat, Etat::Deplacement { pris, .. } if pris == index);

    painter.rect_filled(rect, 2.0, TILE_FILL);
    // **Un seul porteur du turquoise : le bandeau.** Un liseré de tuile en plus doublerait le
    // signal et entrerait en concurrence avec le liseré OR de la tuile visée pendant un
    // déplacement — deux accents pour deux sens différents sur la même arête.
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(
            TILE_BORDER_WIDTH,
            if vise {
                design::tokens::ITEM_SLOT_SELECTED_BORDER
            } else if etat.survol(index) {
                TILE_BORDER_HOVER
            } else {
                TILE_BORDER
            },
        ),
        egui::StrokeKind::Inside,
    );

    let face = Rect::from_min_size(
        egui::pos2(
            rect.center().x - AVATAR_SIZE / 2.0,
            rect.top() + TILE_BORDER_WIDTH + 4.0,
        ),
        Vec2::splat(AVATAR_SIZE),
    );
    paint_avatar(ui, face, perso.class, perso.gender, avatars, icons);

    let band = band_rect(rect);
    if etat.renomme(index) {
        // Le renommage se fait DANS la tuile, à la place du bandeau : le nom ne bouge pas d'un
        // pixel entre la lecture et la correction. Même geste que le web, sans sa popover.
        let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(band));
        cell.add(
            design::input(brouillon)
                .size(InputSize::Height(A_BAND))
                .width(band.width())
                .request_focus(true)
                .log_name(format!("personnages.renommer.{index}")),
        );
    } else {
        painter.rect_filled(band, 0.0, if actif { ACTIVE_BAND } else { IDLE_BAND });
        painted_ellipsed(
            ui,
            &painter,
            perso.name,
            band.center(),
            band.width() - 12.0,
            design::text::label_strong_font(ui.ctx(), 12.5),
            TEXT,
            true,
        );
    }

    if pris {
        // La place d'origine s'efface derrière le fantôme, maintenant seul exemplaire de l'entrée —
        // `panels::tile_reorder`, à la lettre.
        painter.rect_filled(rect, 2.0, DRAG_SOURCE_SCRIM);
    }
    if etat.survol(index) {
        // Le voile s'arrête au bord haut du bandeau : le couvrir rendrait le nom gris sur gris à
        // l'instant où le pointeur l'atteint, c'est-à-dire exactement quand on le lit.
        hover_badge(
            ui,
            rect,
            Rect::from_min_max(rect.min, egui::pos2(rect.right(), band.top())),
            format!("personnages.retirer.{}", perso.name),
        );
    }
    rect
}

/// L'écran : en-tête, ligne de compte, formulaire d'ajout, grille.
#[allow(clippy::too_many_arguments)]
fn ecran(
    ui: &mut egui::Ui,
    scene: &Scene<'_>,
    compte: &mut usize,
    serveur: &mut usize,
    nom: &mut String,
    brouillon: &mut String,
    choisie: Option<(&str, Gender)>,
    etat: Etat,
    liste: &[Perso],
) {
    let width = scene.panel.inner.width();
    ui.add(design::heading("Personnages"));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP);

    account_row(ui, compte, serveur, width);
    ui.add_space(SECTION_GAP * 0.75);
    add_row(ui, nom, choisie, scene.avatars, scene.icons, width);
    ui.add_space(SECTION_GAP * 0.75);

    let mut fantome: Option<(Rect, usize)> = None;
    scene
        .panel
        .scroll_area(ui, "personnages.grille", |ui, content_width| {
            ui.spacing_mut().item_spacing = Vec2::splat(TILE_GAP);
            let tile_width = (content_width - TILE_GAP * (A_TILES_PER_ROW as f32 - 1.0))
                / A_TILES_PER_ROW as f32;
            for (rang, chunk) in liste.chunks(A_TILES_PER_ROW).enumerate() {
                ui.horizontal(|ui| {
                    for (colonne, perso) in chunk.iter().enumerate() {
                        let index = rang * A_TILES_PER_ROW + colonne;
                        let rect = hero_tile(
                            ui,
                            perso,
                            index,
                            Vec2::new(tile_width, A_TILE_HEIGHT),
                            etat,
                            brouillon,
                            scene.avatars,
                            scene.icons,
                        );
                        if let Etat::Deplacement { pris, vise } = etat {
                            if vise == index {
                                fantome = Some((rect, pris));
                            }
                        }
                    }
                });
            }
        });

    // Le fantôme, peint EN DERNIER et au-dessus de tout : c'est ce qu'on tient, il ne peut pas
    // passer sous une tuile voisine. Posé légèrement décalé de la tuile visée, là où serait le
    // pointeur qui l'a pris par son coin haut-gauche.
    if let Some((cible, pris)) = fantome {
        let ghost = Rect::from_min_size(
            cible.min + Vec2::new(-18.0, -14.0),
            Vec2::new(cible.width(), A_TILE_HEIGHT),
        );
        let painter = ui.painter().clone();
        painter.rect_filled(ghost, 2.0, TILE_FILL.gamma_multiply(0.95));
        painter.rect_stroke(
            ghost,
            2.0,
            egui::Stroke::new(TILE_BORDER_WIDTH, TILE_BORDER),
            egui::StrokeKind::Inside,
        );
        let perso = &liste[pris];
        paint_avatar(
            ui,
            Rect::from_min_size(
                egui::pos2(
                    ghost.center().x - AVATAR_SIZE / 2.0,
                    ghost.top() + TILE_BORDER_WIDTH + 4.0,
                ),
                Vec2::splat(AVATAR_SIZE),
            ),
            perso.class,
            perso.gender,
            scene.avatars,
            scene.icons,
        );
        let band = band_rect(ghost);
        painter.rect_filled(band, 0.0, IDLE_BAND);
        painted_ellipsed(
            ui,
            &painter,
            perso.name,
            band.center(),
            band.width() - 12.0,
            design::text::label_strong_font(ui.ctx(), 12.5),
            TEXT,
            true,
        );
    }
}

// -------------------------------------------------------------------------------------------
// La modale de choix de classe
// -------------------------------------------------------------------------------------------

/// Largeur et hauteur de la modale — assez pour six colonnes de bustes NATIFS et leurs libellés,
/// sans barre de défilement : les dix-huit classes tiennent d'un coup, et c'est tout l'intérêt.
const CLASSE_MODALE: Vec2 = Vec2::new(620.0, 636.0);
/// Six colonnes × trois rangées = les dix-huit classes, dans l'ordre du jeu.
const CLASSE_COLS: usize = 6;

/// Le sélecteur de classe, **modale posée sur la fenêtre Options voilée**. Aucun composant du
/// design system ne le couvre ; il est ici assemblé de `design::window` + `design::panel`, comme le
/// ferait le panneau au moment du portage.
fn modale_classe(
    ui: &mut egui::Ui,
    window: Rect,
    genre: &mut usize,
    choisie: usize,
    survolee: Option<usize>,
    avatars: &Avatars,
    icons: &UiIcons,
) {
    // **Tout est peint dans une couche AU-DESSUS**, l'idiome de `design::confirm_dialog` : le `Ui`
    // d'où l'on vient est celui du panneau, dont le clip rognerait et le voile et la modale — la
    // bannière y passait à la trappe et la troisième rangée de classes était coupée.
    let mut couche = ui.new_child(egui::UiBuilder::new().max_rect(window).layer_id(
        egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("personnages.classe.couche"),
        ),
    ));
    couche.set_clip_rect(Rect::EVERYTHING);
    let ui = &mut couche;

    // Le voile : le reste de la fenêtre est inerte, et le dit. Même ton que `design::confirm_dialog`.
    ui.painter().rect_filled(
        window,
        0.0,
        Color32::from_black_alpha(design::tokens::CONFIRM_SCRIM_ALPHA),
    );

    let rect = Rect::from_center_size(window.center(), CLASSE_MODALE);
    let mut modale = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    modale.set_clip_rect(Rect::EVERYTHING);
    let chrome = design::window("Choisir une classe")
        .footer("Annuler", "Valider")
        .close_button(true)
        .log_name("personnages.classe.modale")
        .show(&mut modale);

    design::panel().show(&mut modale, chrome.content, |ui, panel| {
        let gender = if *genre == 0 { Gender::F } else { Gender::M };
        let row = ui
            .allocate_space(Vec2::new(
                panel.inner.width(),
                design::tokens::SELECT_HEIGHT,
            ))
            .1;
        let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
        cell.spacing_mut().item_spacing.x = 0.0;
        cell.horizontal_centered(|ui| {
            ui.label(
                RichText::new("Sexe")
                    .color(SUBDUED)
                    .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
            );
            ui.add_space(10.0);
            design::select(genre)
                .option(0usize, "Féminin")
                .option(1usize, "Masculin")
                .width(170.0)
                .log_name("personnages.classe.sexe")
                .show(ui);
        });
        ui.add_space(SECTION_GAP);

        ui.spacing_mut().item_spacing = Vec2::new(8.0, 10.0);
        let cell_width =
            (panel.inner.width() - 8.0 * (CLASSE_COLS as f32 - 1.0)) / CLASSE_COLS as f32;
        for (rang, chunk) in CLASSES.chunks(CLASSE_COLS).enumerate() {
            ui.horizontal(|ui| {
                for (colonne, (class, label)) in chunk.iter().enumerate() {
                    let index = rang * CLASSE_COLS + colonne;
                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(cell_width, AVATAR_SIZE + 20.0),
                        egui::Sense::click(),
                    );
                    let painter = ui.painter().clone();
                    let retenue = index == choisie;
                    let survol = survolee == Some(index);
                    if retenue || survol {
                        painter.rect_filled(rect, 2.0, TILE_FILL);
                        painter.rect_stroke(
                            rect,
                            2.0,
                            egui::Stroke::new(
                                TILE_BORDER_WIDTH,
                                if retenue {
                                    design::tokens::ITEM_SLOT_SELECTED_BORDER
                                } else {
                                    TILE_BORDER
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                    }
                    paint_avatar(
                        ui,
                        Rect::from_min_size(
                            egui::pos2(rect.center().x - AVATAR_SIZE / 2.0, rect.top() + 2.0),
                            Vec2::splat(AVATAR_SIZE),
                        ),
                        class,
                        gender,
                        avatars,
                        icons,
                    );
                    painted_ellipsed(
                        ui,
                        &painter,
                        label,
                        egui::pos2(rect.center().x, rect.bottom() - 9.0),
                        cell_width - 4.0,
                        design::text::label_font(ui.ctx(), 11.5),
                        if retenue {
                            design::tokens::TEXT_GOLD
                        } else {
                            SUBDUED
                        },
                        true,
                    );
                }
            });
        }
    });
}

// -------------------------------------------------------------------------------------------
// Les planches
// -------------------------------------------------------------------------------------------

/// Peint l'écran dans l'état demandé, sans rien d'autre par-dessus.
fn planche_ecran(
    nom_du_fichier: &str,
    etat: Etat,
    choisie: Option<(&'static str, Gender)>,
    saisi: &str,
) {
    let mut compte = 0usize;
    let mut serveur = 0usize;
    let mut nom = saisi.to_owned();
    let mut brouillon = String::from("Sagitta Lucis");
    let mut harness = options_harness(move |ui, scene| {
        ecran(
            ui,
            &scene,
            &mut compte,
            &mut serveur,
            &mut nom,
            &mut brouillon,
            choisie,
            etat,
            COMPTE_PRINCIPAL,
        );
    });
    harness.run();
    write_mockup(&mut harness, nom_du_fichier);
}

/// **La modale de choix de classe**, par-dessus l'écran voilé. Le Crâ est retenu (liseré et libellé
/// or), le Sram survolé.
fn planche_classe() {
    let mut compte = 0usize;
    let mut serveur = 0usize;
    let mut nom = String::new();
    let mut brouillon = String::new();
    let mut genre = 0usize;
    let mut harness = options_harness(move |ui, scene| {
        ecran(
            ui,
            &scene,
            &mut compte,
            &mut serveur,
            &mut nom,
            &mut brouillon,
            None,
            Etat::Repos,
            COMPTE_PRINCIPAL,
        );
        modale_classe(
            ui,
            scene.window,
            &mut genre,
            8,
            Some(3),
            scene.avatars,
            scene.icons,
        );
    });
    harness.run();
    write_mockup(&mut harness, "personnages_a3_classe");
}

/// **La confirmation de retrait.** Le composant du jeu, pas une popover collée au bouton : boîte
/// centrée sur la fenêtre entière, bouton affirmatif OR — voir `design::confirm_dialog`.
fn planche_suppression() {
    let mut compte = 0usize;
    let mut serveur = 0usize;
    let mut nom = String::new();
    let mut brouillon = String::new();
    let mut harness = options_harness(move |ui, scene| {
        ecran(
            ui,
            &scene,
            &mut compte,
            &mut serveur,
            &mut nom,
            &mut brouillon,
            None,
            Etat::Repos,
            COMPTE_PRINCIPAL,
        );
        let _ = design::confirm_dialog("Retirer « Ignis Dolosus » de ce compte ?")
            .over(scene.window)
            .log_name("personnages.retrait")
            .show(ui);
    });
    harness.run();
    write_mockup(&mut harness, "personnages_a6_suppression");
}

/// **Compte lié, aucun personnage.** L'état d'accueil : le formulaire, et la phrase qui dit quoi en
/// faire.
fn planche_vide() {
    let mut compte = 0usize;
    let mut serveur = 0usize;
    let mut nom = String::new();
    let mut harness = options_harness(move |ui, scene| {
        let width = scene.panel.inner.width();
        ui.add(design::heading("Personnages"));
        paragraph(ui, DESC);
        ui.add_space(SECTION_GAP);
        account_row(ui, &mut compte, &mut serveur, width);
        ui.add_space(SECTION_GAP * 0.75);
        add_row(ui, &mut nom, None, scene.avatars, scene.icons, width);
        ui.add_space(SECTION_GAP * 0.75);
        ui.add(
            design::info_text(
                "Aucun personnage sur ce compte. Choisissez une classe, saisissez le nom exact \
                 tel qu'il apparaît en jeu, puis cliquez sur Ajouter.",
            )
            .width(width)
            .log_name("personnages.vide"),
        );
    });
    harness.run();
    write_mockup(&mut harness, "personnages_a8_vide");
}

/// **Sans compte lié.** Le roster vit sur le compte Wakfu Companion : sans lui, l'overlay n'a ni
/// source ni destination. Même traitement que `panels::alerts_tab`, cas `NoAccount`.
fn planche_sans_compte() {
    let mut harness = options_harness(move |ui, scene| {
        let width = scene.panel.inner.width();
        ui.add(design::heading("Personnages"));
        paragraph(ui, DESC);
        ui.add_space(SECTION_GAP);
        ui.add(
            design::info_text(
                "Aucun compte lié : vos personnages sont enregistrés sur votre compte Wakfu \
                 Companion, et s'y retrouvent depuis le site comme depuis l'overlay. Liez un \
                 compte dans l'onglet « Paramètres » pour les déclarer ici.",
            )
            .width(width)
            .log_name("personnages.sans-compte"),
        );
    });
    harness.run();
    write_mockup(&mut harness, "personnages_a9_sans_compte");
}

fn main() {
    planche_ecran("personnages_a1_ecran", Etat::Repos, None, "");
    println!("  a1 — l'écran au repos");
    planche_ecran("personnages_a2_survol", Etat::Survol(6), None, "");
    println!("  a2 — tuile survolée");
    planche_classe();
    println!("  a3 — modale de choix de classe");
    planche_ecran(
        "personnages_a4_ajout",
        Etat::Repos,
        Some(("cra", Gender::F)),
        "Telum Novum",
    );
    println!("  a4 — classe choisie, nom saisi");
    planche_ecran("personnages_a5_renommage", Etat::Renommage(1), None, "");
    println!("  a5 — renommage sur place");
    planche_suppression();
    println!("  a6 — confirmation du retrait");
    planche_ecran(
        "personnages_a7_deplacement",
        Etat::Deplacement { pris: 10, vise: 7 },
        None,
        "",
    );
    println!("  a7 — déplacement en vol");
    planche_vide();
    println!("  a8 — aucun personnage");
    planche_sans_compte();
    println!("  a9 — aucun compte lié");
    let dir = mockup_dir();
    let ecrites = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    let affiche = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("{ecrites} planches écrites dans {}", affiche.display());
}
