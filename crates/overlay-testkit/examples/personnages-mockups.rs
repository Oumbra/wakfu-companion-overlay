//! **Maquettes de l'onglet « Personnages »** de la fenêtre Options — trois directions à comparer,
//! plus les états et le sélecteur de classe qu'elles partagent.
//!
//! ```bash
//! cargo run -p overlay-testkit --example personnages-mockups
//! ```
//!
//! ## Ce qui est porté, et d'où
//!
//! Le dépôt web (`Oumbra/wakfu-companion`, page « Profil › Personnages », `profile-page.
//! component.html` et `core/services/character-roster.service.ts`) porte tout le système :
//! multi-comptes avec un compte « Principal » ni renommable ni supprimable, serveur de jeu par
//! compte, formulaire d'ajout (portrait → sélecteur de classe, nom), deux vues de liste
//! (grille de tuiles / lignes), renommage sur place, retrait avec confirmation, réordonnancement
//! par glisser-déposer, état vide illustré. L'overlay n'en avait qu'un **miroir en lecture seule**
//! (`overlay_engine::roster::RosterIndex`, alimenté par `GET /api/v1/settings`).
//!
//! ## Ce que le jeu apporte, et qui ne vient pas du web
//!
//! Le panneau de sélection de héros du client Wakfu (quatre captures fournies par l'utilisateur le
//! 2026-09-14) : grille de cartes à quatre colonnes, **portrait porteur d'identité** (aucun nom de
//! classe écrit), niveau incrusté au coin bas-droit du portrait, et surtout un **bandeau de nom
//! pleine largeur collé sous le portrait, turquoise ou gris sombre** selon l'appartenance. C'est
//! cette anatomie que la direction A reprend, et le turquoise que les trois réaffectent au
//! **personnage actif** — le dernier reconnu dans `wakfu.log`
//! (`overlay_engine::session::Engine::last_known_character`), une information que le web n'a pas.
//!
//! **Le niveau n'est PAS repris** : `wakfu.log` ne le porte nulle part (vérifié sur
//! `crates/overlay-engine/tests/wakfu.log` — les seuls « lvl 185 » du fichier sont des messages du
//! canal Recrutement), et `RosterCharacter` n'a pas de champ pour lui. L'afficher demanderait une
//! saisie à la main et une clé de plus côté compte, des deux côtés.
//!
//! ## Les trois directions
//!
//! - **A — « Héros »** ([`personnages_a_heros`]) : la grille du jeu, cinq tuiles par rangée, un
//!   compte à la fois choisi en tête. La plus proche du client, et la seule où le portrait fait
//!   plus que décorer.
//! - **B — « Registre »** ([`personnages_b_registre`]) : un `design::table`, une ligne par
//!   personnage, **tous comptes confondus**, avec recherche. La seule qui tienne les quarante
//!   personnages d'un vrai multi-compte sans naviguer.
//! - **C — « Comptes empilés »** ([`personnages_c_comptes`]) : un `design::collapsible` par
//!   compte, sa grille dedans. Tous les comptes visibles d'un coup, la structure avant le contenu.
//!
//! Les composants sont **appelés**, jamais recopiés : `design::window`, `tabs`, `panel`, `heading`,
//! `select`, `input`, `button`, `icon_button`, `table`, `collapsible`, `info_text`, `portrait`. Le
//! chrome est celui de la production, à sa taille de production. Les planches sont écrites dans
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
//! 4. **Un sélecteur de classe** (36 portraits, bascule ♂/♀) — aucun composant du design system ne
//!    le couvre ; [`personnages_selecteur_classe`] en propose le gabarit.
//! 5. **Le mode sans compte lié** : mêmes trois cas qu'`alerts_tab` (`Ready`/`Loading`/`NoAccount`).
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Color32, Rect, RichText, Vec2};
use egui_kittest::Harness;
use overlay_engine::Gender;
use overlay_ui::design::{self, ButtonSize, ButtonVariant, DsIcon, IconContext, InputSize};
use overlay_ui::panels::options_modal::{self, OptionsTab};
use overlay_ui::portraits::PortraitAtlas;
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
/// L'écart se voyait : le bandeau d'une tuile ressortait plus clair et plus saturé que la bannière
/// de la fenêtre qui le contient.
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

const DESC: &str = "Déclarez les personnages de vos comptes : l'overlay les reconnaît dans le \
                    journal, les distingue des ennemis et rattache vos gains au bon compte.";

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
    /// Nom de classe affiché — celui que le web écrit sous le portrait en vue grille.
    class_label: &'static str,
}

const fn p(
    name: &'static str,
    class: &'static str,
    gender: Gender,
    class_label: &'static str,
) -> Perso {
    Perso {
        name,
        class,
        gender,
        class_label,
    }
}

/// Le compte principal — douze personnages, de quoi remplir deux rangées et demie.
static COMPTE_PRINCIPAL: &[Perso] = &[
    p("Pugio Letalis", "sram", Gender::M, "Sram"),
    p("Sagitta Lucis", "cra", Gender::F, "Crâ"),
    p("Ensis Orientalis", "iop", Gender::M, "Iop"),
    p("Canis Furiosus", "ouginak", Gender::M, "Ouginak"),
    p("Rota Metallica", "foggernaut", Gender::F, "Steamer"),
    p("Imago Speculi", "zobal", Gender::F, "Zobal"),
    p("Ignis Dolosus", "rogue", Gender::M, "Roublard"),
    p("Monstrum Amoris", "osamodas", Gender::M, "Osamodas"),
    p("Bursa Auri", "enutrof", Gender::F, "Enutrof"),
    p("Penicillus Vitae", "eniripsa", Gender::F, "Eniripsa"),
    p("Aegis Feminea", "feca", Gender::F, "Féca"),
    p("Arbovenenum", "sadida", Gender::M, "Sadida"),
];

/// Un deuxième compte, plus court.
static COMPTE_MULE: &[Perso] = &[
    p("Magister Thesauri", "enutrof", Gender::M, "Enutrof"),
    p("Cura Pictor", "eniripsa", Gender::M, "Eniripsa"),
    p("Scutum Tutelare", "feca", Gender::M, "Féca"),
    p("Rhynchanthera", "sadida", Gender::M, "Sadida"),
    p("Cartarum Lusor", "ecaflip", Gender::M, "Ecaflip"),
    p("Fanaticus Personae", "sacrier", Gender::M, "Sacrieur"),
];

/// Un troisième, pour que la direction C ait trois blocs à empiler.
static COMPTE_CRAFT: &[Perso] = &[
    p("Contra Horologii", "xelor", Gender::F, "Xélor"),
    p("Porta Divina", "eliotrope", Gender::F, "Éliotrope"),
    p("Runae Desolationis", "huppermage", Gender::M, "Huppermage"),
    p("Bambusa Vacillans", "pandawa", Gender::M, "Pandawa"),
];

/// Le personnage reconnu en dernier dans `wakfu.log` — celui qui porte le bandeau turquoise.
const ACTIF: &str = "Sagitta Lucis";

/// Une ligne du registre (direction B) : le personnage, et le compte auquel il appartient.
struct Ligne {
    perso: &'static Perso,
    compte: &'static str,
    serveur: &'static str,
}

fn registre() -> Vec<Ligne> {
    let mut lignes = Vec::new();
    for perso in COMPTE_PRINCIPAL {
        lignes.push(Ligne {
            perso,
            compte: "Principal",
            serveur: "Pandora",
        });
    }
    for perso in COMPTE_MULE {
        lignes.push(Ligne {
            perso,
            compte: "Mules",
            serveur: "Pandora",
        });
    }
    for perso in COMPTE_CRAFT {
        lignes.push(Ligne {
            perso,
            compte: "Métiers",
            serveur: "Rubilax",
        });
    }
    lignes
}

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

fn options_harness(
    mut build: impl FnMut(&mut egui::Ui, &UiIcons, &PortraitAtlas, &design::PanelZones, Rect) + 'static,
) -> Harness<'static> {
    // Chargés UNE fois et gardés vivants entre les frames : un `TextureHandle` libère sa texture
    // dès que son dernier exemplaire tombe, et la planche sortirait avec des portraits vides.
    let mut icons: Option<UiIcons> = None;
    let mut portraits: Option<PortraitAtlas> = None;
    let mut tab = OptionsTab::Personnages;
    Harness::builder().with_size(WINDOW).build_ui(move |ui| {
        overlay_ui::style::apply(ui.ctx());
        // La bannière porte le numéro de version, que le hook `post-commit` incrémente à chaque
        // `feat:`/`fix:` — sans ce gel, chaque planche périmerait au commit suivant.
        overlay_ui::build_info::freeze_for_snapshots();
        ui.style_mut().visuals.text_cursor.blink = false;
        let icons = icons.get_or_insert_with(|| UiIcons::load(ui.ctx()));
        let portraits = portraits.get_or_insert_with(|| PortraitAtlas::load(ui.ctx()));
        egui::Frame::NONE.fill(BACKDROP).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let window = ui.max_rect();
            let chrome = design::window("Options")
                .footer("Annuler", "Valider")
                .close_button(true)
                .version(true)
                .log_name("maquette")
                .show(ui);
            // L'ordre des entrées est celui de la production (`options_modal::show`). « Personnages »
            // n'y porte plus `.enabled(false)` : c'est justement l'onglet que ces planches montrent.
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
                build(ui, icons, portraits, panel, window);
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

/// Texte d'une ligne, ellipsé s'il déborde — jamais rogné en silence. Rend la largeur réellement
/// occupée, pour que l'appelant puisse poser quelque chose à sa suite.
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

/// Le portrait de classe d'un personnage, peint dans `rect`. Repli sur le portrait générique
/// d'`UiIcons` quand la classe est inconnue de l'atlas — jamais un trou.
fn paint_face(
    ui: &egui::Ui,
    rect: Rect,
    perso: &Perso,
    portraits: &PortraitAtlas,
    icons: &UiIcons,
) {
    let texture = portraits
        .texture(perso.class, perso.gender, false)
        .map(|handle| handle.id())
        .unwrap_or_else(|| icons.unknown_entity_texture().id());
    design::paint_portrait(ui, rect, texture, design::PortraitShape::Square, false);
}

/// La croix de retrait révélée au survol d'une tuile — l'idiome des tuiles d'Alertes et de Chat.
/// `scrim` est la zone réellement voilée : elle vaut la tuile entière quand rien n'y doit rester
/// lisible, et s'arrête au bandeau de nom quand il y en a un (voir [`hero_tile`]).
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

/// La ligne « compte + serveur » qui coiffe les directions A et B. Le bouton « + » ajoute un
/// compte, ancré au bord droit comme le « Ajouter » du formulaire de Chat.
fn account_row(ui: &mut egui::Ui, compte: &mut usize, serveur: &mut usize, width: f32) {
    // La ligne n'est pas dans la zone défilable : elle reprend la réserve de barre, comme le
    // formulaire d'ajout de `chat_tab`.
    let total =
        width + design::components::scroll_area::RESERVE_X - design::tokens::PANEL_PAD_CONTROL_X;
    let row = ui
        .allocate_space(Vec2::new(total, design::tokens::SELECT_HEIGHT))
        .1;
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
                .tooltip("Supprimer ce compte")
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
/// (`character-add-form.component.html`). Le portrait est un bouton : il ouvre le sélecteur de
/// classe (voir [`personnages_selecteur_classe`]) et porte ensuite la classe choisie.
fn add_row(
    ui: &mut egui::Ui,
    nom: &mut String,
    choisie: Option<&Perso>,
    portraits: &PortraitAtlas,
    icons: &UiIcons,
    width: f32,
) {
    const FACE: f32 = 36.0;
    const ADD_WIDTH: f32 = 100.0;
    const GAP: f32 = 12.0;
    let total =
        width + design::components::scroll_area::RESERVE_X - design::tokens::PANEL_PAD_CONTROL_X;
    let row = ui
        .allocate_space(Vec2::new(total, design::tokens::SELECT_HEIGHT))
        .1;
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
        Some(perso) => paint_face(ui, face.shrink(3.0), perso, portraits, icons),
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
// A — « Héros » : la grille du jeu
// -------------------------------------------------------------------------------------------

/// Tuiles par rangée. **Cinq** : à 675 pt de large utile, une tuile fait 125 pt, soit très près des
/// 105 px des cartes du jeu — quatre les rendrait obèses, six couperait un nom sur deux.
const A_TILES_PER_ROW: usize = 5;
/// Hauteur d'une tuile : 8 de marge + 48 de portrait + 4 + 14 de classe + 6 + 22 de bandeau.
const A_TILE_HEIGHT: f32 = 102.0;
/// Côté du portrait dans la tuile — **48, la taille NATIVE** des fichiers `class-profile/*.png`.
/// `portraits.rs` a justement été refondu en 36 textures indépendantes pour éviter le filtrage
/// LINEAR sur le bord des médaillons ; les peindre à 56 rendait ce travail inutile.
const A_FACE: f32 = 48.0;
/// Hauteur du bandeau de nom, relevée sur les captures du panneau de héros (~20 px).
const A_BAND: f32 = 22.0;

/// Une carte de héros : portrait au centre, classe en gris, **bandeau de nom pleine largeur collé
/// en bas** — turquoise si le personnage est celui que le journal a reconnu en dernier.
fn hero_tile(
    ui: &mut egui::Ui,
    perso: &Perso,
    size: Vec2,
    portraits: &PortraitAtlas,
    icons: &UiIcons,
) {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter().clone();
    let actif = perso.name == ACTIF;

    painter.rect_filled(rect, 2.0, TILE_FILL);
    // **Un seul porteur du turquoise : le bandeau.** Un liseré de tuile en plus doublait le signal
    // et serait entré en concurrence avec le liseré or que `panels::tile_reorder` pose sur la tuile
    // visée pendant un déplacement — deux accents pour deux sens différents sur la même arête.
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(TILE_BORDER_WIDTH, TILE_BORDER),
        egui::StrokeKind::Inside,
    );

    let face = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 8.0 + A_FACE / 2.0),
        Vec2::splat(A_FACE),
    );
    paint_face(ui, face, perso, portraits, icons);

    painted_ellipsed(
        ui,
        &painter,
        perso.class_label,
        egui::pos2(rect.center().x, face.bottom() + 11.0),
        rect.width() - 12.0,
        design::text::label_font(ui.ctx(), 11.0),
        SUBDUED,
        true,
    );

    // Le bandeau : pleine largeur intérieure, collé au bord bas, comme sur les cartes du jeu.
    let band = Rect::from_min_max(
        egui::pos2(
            rect.left() + TILE_BORDER_WIDTH,
            rect.bottom() - TILE_BORDER_WIDTH - A_BAND,
        ),
        egui::pos2(
            rect.right() - TILE_BORDER_WIDTH,
            rect.bottom() - TILE_BORDER_WIDTH,
        ),
    );
    painter.rect_filled(band, 0.0, if actif { ACTIVE_BAND } else { IDLE_BAND });
    painted_ellipsed(
        ui,
        &painter,
        perso.name,
        egui::pos2(band.left() + 7.0, band.center().y),
        band.width() - 14.0,
        design::text::label_strong_font(ui.ctx(), 12.0),
        TEXT,
        false,
    );

    if response.contains_pointer() {
        // Le voile s'arrête au bord haut du bandeau — le couvrir rendait le nom gris sur gris à
        // l'instant où le pointeur l'atteint, c'est-à-dire exactement quand on le lit.
        hover_badge(
            ui,
            rect,
            Rect::from_min_max(rect.min, egui::pos2(rect.right(), band.top())),
            format!("personnages.retirer.{}", perso.name),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn direction_a(
    ui: &mut egui::Ui,
    panel: &design::PanelZones,
    portraits: &PortraitAtlas,
    icons: &UiIcons,
    compte: &mut usize,
    serveur: &mut usize,
    nom: &mut String,
    liste: &[Perso],
) {
    let width = panel.inner.width();
    ui.add(design::heading("Personnages"));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP);

    account_row(ui, compte, serveur, width);
    ui.add_space(SECTION_GAP * 0.75);
    add_row(ui, nom, Some(&liste[1]), portraits, icons, width);
    ui.add_space(SECTION_GAP * 0.75);

    panel.scroll_area(ui, "personnages.grille", |ui, content_width| {
        ui.spacing_mut().item_spacing = Vec2::splat(TILE_GAP);
        let tile_width =
            (content_width - TILE_GAP * (A_TILES_PER_ROW as f32 - 1.0)) / A_TILES_PER_ROW as f32;
        for chunk in liste.chunks(A_TILES_PER_ROW) {
            ui.horizontal(|ui| {
                for perso in chunk {
                    hero_tile(
                        ui,
                        perso,
                        Vec2::new(tile_width, A_TILE_HEIGHT),
                        portraits,
                        icons,
                    );
                }
            });
        }
    });
}

// -------------------------------------------------------------------------------------------
// B — « Registre » : une ligne par personnage, tous comptes confondus
// -------------------------------------------------------------------------------------------

const B_ROW_HEIGHT: f32 = 44.0;

fn direction_b(
    ui: &mut egui::Ui,
    panel: &design::PanelZones,
    portraits: &PortraitAtlas,
    icons: &UiIcons,
    recherche: &mut String,
    lignes: &[Ligne],
) {
    let width = panel.inner.width();
    ui.add(design::heading("Personnages"));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP);

    // Barre : recherche à gauche, « Ajouter un personnage » ancré à droite.
    let total =
        width + design::components::scroll_area::RESERVE_X - design::tokens::PANEL_PAD_CONTROL_X;
    let row = ui
        .allocate_space(Vec2::new(total, design::tokens::SELECT_HEIGHT))
        .1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    cell.spacing_mut().item_spacing.x = 0.0;
    cell.horizontal_centered(|ui| {
        ui.add(
            design::input(recherche)
                .size(InputSize::Search)
                .leading_icon(DsIcon::Search)
                .clearable(true)
                .placeholder("Filtrer par nom, classe ou compte…")
                .width(total - 250.0)
                .log_name("personnages.recherche"),
        );
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        right.add(
            design::button("Ajouter")
                .variant(ButtonVariant::Primary)
                .size(ButtonSize::Compact)
                .width(110.0)
                .log_name("personnages.ajouter"),
        );
        right.add_space(8.0);
        right.add(
            design::icon_button(DsIcon::Order)
                .context(IconContext::Panel)
                .tooltip("Réordonner")
                .log_name("personnages.ordre"),
        );
    });
    ui.add_space(SECTION_GAP * 0.75);

    // Hauteur calée sur un nombre ENTIER de lignes : un tableau qui s'arrête au milieu d'une
    // ligne se lit comme un défaut de mise en page, pas comme une invitation à faire défiler.
    // Mesuré sur le BAS DU PANNEAU (`panel.inner`), pas sur `ui.available_height()` : le `Ui` du
    // panneau déborde son propre écrêtage, et s'y fier laissait une dixième ligne coupée en deux
    // sous le bord — exactement le défaut que ce calage doit éviter.
    let dispo = panel.inner.bottom() - ui.cursor().top() - design::Table::header_height();
    // `max_height` borne le CORPS seul, en-tête non compris (voir `Table::show`) : y ajouter la
    // hauteur d'en-tête laissait dépasser une ligne de plus sous le bord du panneau.
    let visible = (dispo / B_ROW_HEIGHT).floor().max(1.0) * B_ROW_HEIGHT;
    design::table()
        .column(design::TableColumn::fixed("", 44.0))
        .column(design::TableColumn::flex("Personnage", 2.0))
        .column(design::TableColumn::fixed("Classe", 120.0))
        .column(design::TableColumn::flex("Compte", 1.2))
        .column(design::TableColumn::fixed("Serveur", 100.0))
        .column(design::TableColumn::fixed("", 40.0))
        .body(design::TableBody::Rows(lignes.len()))
        .row_height(B_ROW_HEIGHT)
        .width(width)
        .max_height(visible)
        .empty_text("Aucun personnage déclaré.")
        .log_name("personnages.registre")
        .show(ui, |row| {
            let Some(ligne) = lignes.get(row.index()) else {
                return;
            };
            let actif = ligne.perso.name == ACTIF;
            if actif {
                // Le turquoise, réduit à un liseré : une ligne entièrement teintée hurlerait.
                let r = row.rect();
                row.response()
                    .ctx
                    .layer_painter(row.response().layer_id)
                    .rect_filled(
                        Rect::from_min_max(r.min, egui::pos2(r.left() + 3.0, r.bottom())),
                        0.0,
                        ACTIVE_BAND,
                    );
            }
            row.cell(|ui| {
                let face = Rect::from_center_size(
                    ui.max_rect().center(),
                    Vec2::splat(overlay_ui::portraits::PORTRAIT_SIZE * 0.8),
                );
                paint_face(ui, face, ligne.perso, portraits, icons);
            });
            row.cell(|ui| {
                ui.add(
                    design::label(ligne.perso.name)
                        .width(ui.available_width())
                        .align(egui::Align::LEFT)
                        .size(BODY_FONT_SIZE)
                        .color(if actif {
                            design::tokens::TEXT_GOLD
                        } else {
                            TEXT
                        })
                        .log_name("personnages.registre.nom"),
                );
            });
            row.cell(|ui| {
                ui.add(
                    design::label(ligne.perso.class_label)
                        .width(ui.available_width())
                        .align(egui::Align::LEFT)
                        .size(13.0)
                        .color(SUBDUED)
                        .log_name("personnages.registre.classe"),
                );
            });
            row.cell(|ui| {
                ui.add(
                    design::label(ligne.compte)
                        .width(ui.available_width())
                        .align(egui::Align::LEFT)
                        .size(13.0)
                        .color(SUBDUED)
                        .log_name("personnages.registre.compte"),
                );
            });
            row.cell(|ui| {
                ui.add(
                    design::label(ligne.serveur)
                        .width(ui.available_width())
                        .align(egui::Align::LEFT)
                        .size(13.0)
                        .color(SUBDUED)
                        .log_name("personnages.registre.serveur"),
                );
            });
            row.cell(|ui| {
                ui.add(
                    design::icon_button(DsIcon::Close)
                        .context(IconContext::Panel)
                        .size(26.0)
                        .tooltip("Retirer")
                        .log_name("personnages.registre.retirer"),
                );
            });
        });
}

// -------------------------------------------------------------------------------------------
// C — « Comptes empilés » : un bloc repliable par compte
// -------------------------------------------------------------------------------------------

const C_TILES_PER_ROW: usize = 3;
/// 64 — `tokens::ITEM_SLOT_SIZE`, le pas des grilles d'Alertes et du Suivi : la grille des comptes
/// se cale sur elles au lieu d'inventer sa propre hauteur.
const C_TILE_HEIGHT: f32 = 64.0;
/// 48, taille native du portrait — voir [`A_FACE`].
const C_FACE: f32 = 48.0;

/// Une tuile compacte : portrait à gauche, nom et classe à droite. Plus dense que la carte de la
/// direction A, parce qu'elle vit DANS un bloc qui a déjà son propre en-tête.
fn compact_tile(
    ui: &mut egui::Ui,
    perso: &Perso,
    size: Vec2,
    portraits: &PortraitAtlas,
    icons: &UiIcons,
) {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter().clone();
    let actif = perso.name == ACTIF;

    painter.rect_filled(rect, 2.0, TILE_FILL);
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(
            TILE_BORDER_WIDTH,
            if actif { ACTIVE_BAND } else { TILE_BORDER },
        ),
        egui::StrokeKind::Inside,
    );
    // Un seul porteur du turquoise ici aussi : la bordure. L'arête de 3 pt qu'elle portait en plus
    // disparaissait dessous, à un pixel près.

    let face = Rect::from_center_size(
        egui::pos2(rect.left() + 10.0 + C_FACE / 2.0, rect.center().y),
        Vec2::splat(C_FACE),
    );
    paint_face(ui, face, perso, portraits, icons);

    let text_left = face.right() + 9.0;
    let text_width = rect.right() - text_left - 10.0;
    painted_ellipsed(
        ui,
        &painter,
        perso.name,
        egui::pos2(text_left, rect.center().y - 9.0),
        text_width,
        design::text::label_strong_font(ui.ctx(), 13.0),
        TEXT,
        false,
    );
    painted_ellipsed(
        ui,
        &painter,
        perso.class_label,
        egui::pos2(text_left, rect.center().y + 9.0),
        text_width,
        design::text::label_font(ui.ctx(), 11.0),
        SUBDUED,
        false,
    );

    if response.contains_pointer() {
        hover_badge(
            ui,
            rect,
            rect,
            format!("personnages.c.retirer.{}", perso.name),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn compte_bloc(
    ui: &mut egui::Ui,
    titre: &str,
    serveur: &str,
    liste: &[Perso],
    ouvert: &mut bool,
    nom: &mut String,
    // Le compte principal ne se supprime pas — voir la ligne de réglage ci-dessous.
    principal: bool,
    portraits: &PortraitAtlas,
    icons: &UiIcons,
) {
    let titre_complet = format!("{titre}  ·  {serveur}  ·  {} personnages", liste.len());
    design::collapsible(titre_complet, ouvert)
        .icon(DsIcon::Characters)
        .log_name(format!("personnages.compte.{titre}"))
        .show(ui, |ui| {
            let width = ui.available_width();
            // **La ligne de réglage du compte, DANS son bloc.** Sans elle, la direction C ne sait
            // ni renommer un compte, ni lui poser un serveur, ni le supprimer — et le bloc
            // repliable n'est plus qu'un classeur. Aplat de ligne de réglage du jeu
            // (`alerts_tab::SETTING_ROW_FILL`, rayon 4, 40 pt), comme « Fermeture automatique ».
            let ligne = ui.allocate_space(Vec2::new(width, 40.0)).1;
            ui.painter().rect_filled(ligne, 4.0, IDLE_BAND);
            let mut cell =
                ui.new_child(egui::UiBuilder::new().max_rect(ligne.shrink2(Vec2::new(12.0, 0.0))));
            cell.spacing_mut().item_spacing.x = 0.0;
            cell.horizontal_centered(|ui| {
                ui.label(
                    RichText::new("Nom du compte")
                        .color(SUBDUED)
                        .font(design::text::label_font(ui.ctx(), 13.0)),
                );
                ui.add_space(10.0);
                let mut libelle = titre.to_owned();
                ui.add(
                    design::input(&mut libelle)
                        .size(InputSize::Search)
                        .width(180.0)
                        .log_name("personnages.compte.nom"),
                );
                ui.add_space(20.0);
                ui.label(
                    RichText::new("Serveur")
                        .color(SUBDUED)
                        .font(design::text::label_font(ui.ctx(), 13.0)),
                );
                ui.add_space(10.0);
                let mut choix = 0usize;
                design::select(&mut choix)
                    .option(0usize, serveur)
                    .option(1usize, "Aucun")
                    .width(150.0)
                    .log_name("personnages.compte.serveur")
                    .show(ui);
                let mut right = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(ligne.shrink2(Vec2::new(8.0, 0.0)))
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                );
                // **Rien du tout sur le compte principal**, pas un bouton désactivé : c'est le
                // traitement que les dix objets par défaut d'Alertes ont déjà reçu.
                if !principal {
                    right.add(
                        design::icon_button(DsIcon::Delete)
                            .context(IconContext::Panel)
                            .size(28.0)
                            .tooltip("Supprimer ce compte")
                            .log_name("personnages.compte.retirer"),
                    );
                }
            });
            ui.add_space(TILE_GAP);
            add_row(ui, nom, Some(&liste[0]), portraits, icons, width - 26.0);
            ui.add_space(TILE_GAP);
            ui.spacing_mut().item_spacing = Vec2::splat(TILE_GAP);
            let tile_width =
                (width - TILE_GAP * (C_TILES_PER_ROW as f32 - 1.0)) / C_TILES_PER_ROW as f32;
            for chunk in liste.chunks(C_TILES_PER_ROW) {
                ui.horizontal(|ui| {
                    for perso in chunk {
                        compact_tile(
                            ui,
                            perso,
                            Vec2::new(tile_width, C_TILE_HEIGHT),
                            portraits,
                            icons,
                        );
                    }
                });
            }
        });
}

// -------------------------------------------------------------------------------------------
// Les planches
// -------------------------------------------------------------------------------------------

/// **A — « Héros ».** La grille du panneau de héros du jeu, portée au design system : cinq tuiles
/// par rangée, portrait au centre, bandeau de nom collé en bas. « Sagitta Lucis », le dernier
/// personnage reconnu dans le journal, porte le bandeau turquoise. La troisième tuile est survolée.
fn personnages_a_heros() {
    let mut compte = 0usize;
    let mut serveur = 0usize;
    let mut nom = String::new();
    let mut harness = options_harness(move |ui, icons, portraits, panel, _window| {
        direction_a(
            ui,
            panel,
            portraits,
            icons,
            &mut compte,
            &mut serveur,
            &mut nom,
            COMPTE_PRINCIPAL,
        );
    });
    harness.run();
    write_mockup(&mut harness, "personnages_a_heros");
}

/// La même, une tuile survolée : voile et croix de retrait.
fn personnages_a_heros_survol() {
    let mut compte = 0usize;
    let mut serveur = 0usize;
    let mut nom = String::new();
    let mut harness = options_harness(move |ui, icons, portraits, panel, _window| {
        direction_a(
            ui,
            panel,
            portraits,
            icons,
            &mut compte,
            &mut serveur,
            &mut nom,
            COMPTE_PRINCIPAL,
        );
    });
    harness.run();
    // Centre de la troisième tuile de la première rangée : bord de contenu à x = 39.
    harness.hover_at(egui::pos2(39.0 + 125.4 * 2.5 + TILE_GAP * 2.0, 400.0));
    harness.run();
    write_mockup(&mut harness, "personnages_a_heros_survol");
}

/// **B — « Registre ».** Un `design::table`, les vingt-deux personnages des trois comptes d'un
/// coup, filtrables. Le liseré turquoise et le nom en or marquent le personnage actif.
fn personnages_b_registre() {
    let lignes = registre();
    let mut recherche = String::new();
    let mut harness = options_harness(move |ui, icons, portraits, panel, _window| {
        direction_b(ui, panel, portraits, icons, &mut recherche, &lignes);
    });
    harness.run();
    write_mockup(&mut harness, "personnages_b_registre");
}

/// **C — « Comptes empilés ».** Un bloc repliable par compte, sa grille compacte dedans. Le premier
/// est ouvert, les deux autres repliés — la structure se lit avant le contenu.
fn personnages_c_comptes() {
    let mut ouverts = [true, false, false];
    let mut noms = [String::new(), String::new(), String::new()];
    let mut harness = options_harness(move |ui, icons, portraits, panel, _window| {
        ui.add(design::heading("Personnages"));
        paragraph(ui, DESC);
        ui.add_space(SECTION_GAP);

        panel.scroll_area(ui, "personnages.comptes", |ui, _content_width| {
            ui.spacing_mut().item_spacing.y = TILE_GAP;
            compte_bloc(
                ui,
                "Principal",
                "Pandora",
                COMPTE_PRINCIPAL,
                &mut ouverts[0],
                &mut noms[0],
                true,
                portraits,
                icons,
            );
            compte_bloc(
                ui,
                "Mules",
                "Pandora",
                COMPTE_MULE,
                &mut ouverts[1],
                &mut noms[1],
                false,
                portraits,
                icons,
            );
            compte_bloc(
                ui,
                "Métiers",
                "Rubilax",
                COMPTE_CRAFT,
                &mut ouverts[2],
                &mut noms[2],
                false,
                portraits,
                icons,
            );
        });
    });
    harness.run();
    write_mockup(&mut harness, "personnages_c_comptes");
}

/// **Le sélecteur de classe**, commun aux trois directions et absent du design system : 18 classes
/// × 2 sexes, une bascule ♂/♀ en tête. Porté du web (`class-picker.component.html`), rendu ici au
/// gabarit du panneau — six colonnes de portraits à 64 pt, le nom de classe dessous.
fn personnages_selecteur_classe() {
    let mut genre = 0usize;
    let mut harness = options_harness(move |ui, icons, portraits, panel, _window| {
        let width = panel.inner.width();
        ui.add(design::heading("Choisir la classe"));
        paragraph(
            ui,
            "Le portrait porte l'identité du personnage dans tout l'overlay : liste de combat, \
             cadre à six médaillons, cartes d'alerte.",
        );
        ui.add_space(SECTION_GAP);

        let row = ui
            .allocate_space(Vec2::new(width, design::tokens::SELECT_HEIGHT))
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
            design::select(&mut genre)
                .option(0usize, "Féminin")
                .option(1usize, "Masculin")
                .width(170.0)
                .log_name("personnages.sexe")
                .show(ui);
        });
        ui.add_space(SECTION_GAP);

        const COLS: usize = 6;
        const FACE: f32 = 64.0;
        let gender = if genre == 0 { Gender::F } else { Gender::M };
        let classes: &[(&str, &str)] = &[
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
        panel.scroll_area(ui, "personnages.classes", |ui, content_width| {
            ui.spacing_mut().item_spacing = Vec2::splat(TILE_GAP);
            let cell_width = (content_width - TILE_GAP * (COLS as f32 - 1.0)) / COLS as f32;
            for chunk in classes.chunks(COLS) {
                ui.horizontal(|ui| {
                    for (class, label) in chunk {
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(cell_width, FACE + 22.0),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter().clone();
                        if response.contains_pointer() {
                            painter.rect_filled(rect, 2.0, TILE_FILL);
                            painter.rect_stroke(
                                rect,
                                2.0,
                                egui::Stroke::new(TILE_BORDER_WIDTH, design::tokens::TEXT_GOLD),
                                egui::StrokeKind::Inside,
                            );
                        }
                        let face = Rect::from_center_size(
                            egui::pos2(rect.center().x, rect.top() + 4.0 + FACE / 2.0),
                            Vec2::splat(FACE),
                        );
                        let texture = portraits
                            .texture(class, gender, false)
                            .map(|h| h.id())
                            .unwrap_or_else(|| icons.unknown_entity_texture().id());
                        design::paint_portrait(
                            ui,
                            face,
                            texture,
                            design::PortraitShape::Square,
                            false,
                        );
                        painted_ellipsed(
                            ui,
                            &painter,
                            label,
                            egui::pos2(rect.center().x, face.bottom() + 11.0),
                            cell_width - 6.0,
                            design::text::label_font(ui.ctx(), 12.0),
                            SUBDUED,
                            true,
                        );
                    }
                });
            }
        });
    });
    harness.run();
    harness.hover_at(egui::pos2(39.0 + 55.0, 330.0));
    harness.run();
    write_mockup(&mut harness, "personnages_selecteur_classe");
}

/// **Sans compte lié.** Le roster vit sur le compte Wakfu Companion : sans lui, l'overlay n'a ni
/// source ni destination. Même traitement que `panels::alerts_tab`, cas `NoAccount`.
fn personnages_sans_compte() {
    let mut harness = options_harness(move |ui, _icons, _portraits, panel, _window| {
        let width = panel.inner.width();
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
    write_mockup(&mut harness, "personnages_sans_compte");
}

/// **Compte lié, aucun personnage.** L'état d'accueil : le formulaire, et la phrase qui dit quoi en
/// faire.
fn personnages_vide() {
    let mut compte = 0usize;
    let mut serveur = 0usize;
    let mut nom = String::new();
    let mut harness = options_harness(move |ui, icons, portraits, panel, _window| {
        let width = panel.inner.width();
        ui.add(design::heading("Personnages"));
        paragraph(ui, DESC);
        ui.add_space(SECTION_GAP);
        account_row(ui, &mut compte, &mut serveur, width);
        ui.add_space(SECTION_GAP * 0.75);
        add_row(ui, &mut nom, None, portraits, icons, width);
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
    write_mockup(&mut harness, "personnages_vide");
}

fn main() {
    personnages_a_heros();
    println!("  personnages_a_heros");
    personnages_a_heros_survol();
    println!("  personnages_a_heros_survol");
    personnages_b_registre();
    println!("  personnages_b_registre");
    personnages_c_comptes();
    println!("  personnages_c_comptes");
    personnages_selecteur_classe();
    println!("  personnages_selecteur_classe");
    personnages_sans_compte();
    println!("  personnages_sans_compte");
    personnages_vide();
    println!("  personnages_vide");
    let dir = mockup_dir();
    let ecrites = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    let affiche = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("{ecrites} planches écrites dans {}", affiche.display());
}
