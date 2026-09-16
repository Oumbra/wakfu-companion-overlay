//! **Onglet « Personnages » de la fenêtre Options** — l'écran, ses gestes, et les deux modales qui
//! les portent.
//!
//! ```bash
//! cargo run -p overlay-testkit --example personnages-mockups
//! ```
//!
//! ## Où en est ce fichier
//!
//! Trois jets : la comparaison de trois directions (2026-09-14), la direction « Héros » repeinte
//! aux bustes détourés (2026-09-16 matin), et celui-ci — **les décisions du 2026-09-16 appliquées**,
//! point par point :
//!
//! | Décision | Ce que ça change ici |
//! | --- | --- |
//! | Plus de formulaire en tête | l'ajout passe par une **tuile « + » en TÊTE de grille**, toujours première : à quarante personnages, on n'a pas à défiler pour en créer un |
//! | Une seule modale | « Nouveau personnage » et « Modifier » sont **le même écran** : nom, sexe, classe, d'un coup |
//! | Sexe | `design::switch` à deux cases (♂ / ♀), **♂ par défaut**, et le choix **survit** d'une création à la suivante |
//! | Classes | grille de portraits **grisés**, colorés au survol, bordure dorée et **infobulle** au nom de classe — plus de libellé permanent |
//! | Recherche de classe | à droite du switch, seuil de **trois caractères**, insensible à la casse ET aux accents ([`normalise`]) |
//! | Titre de liste | « Personnages du compte » + les commandes de **suppression multiple** (`panels::bulk_select`), comme aux trois autres onglets |
//! | Tuile | **100 de large** (80 de buste, 10 de marge de part et d'autre), arrondi de champ de saisie, nom ellipsé + infobulle quand il déborde |
//! | Survol | **crayon** (modifier) et croix (retirer) |
//! | Bandeau turquoise | **supprimé** — tous les noms sur le même gris |
//! | « Sans compte lié » | **supprimé** — l'overlay ne s'utilise pas déconnecté |
//! | Comptes | une modale de **création de compte**, et une suppression qui annonce son décompte |
//!
//! ## Le crayon est arrivé
//!
//! Le badge de modification a porté [`DsIcon::Option`] — la roue crantée du bouton Options —
//! pendant une journée, faute de crayon au registre. L'utilisateur en a détouré un le 2026-09-16
//! (`assets/design-system/icons/icon-edit.png`) ; il est entré au registre sous
//! [`DsIcon::Edit`] et c'est lui que la tuile porte désormais.
//!
//! ## D'où vient chaque chose
//!
//! Le dépôt web (`Oumbra/wakfu-companion`, page « Profil › Personnages ») porte le système :
//! multi-comptes avec un compte principal ni renommable ni supprimable, serveur de jeu par compte,
//! sélecteur de classe, renommage, retrait avec confirmation, réordonnancement au glisser-déposer.
//! L'overlay n'en a qu'un **miroir en lecture seule** (`overlay_engine::roster::RosterIndex`).
//! Le jeu apporte la forme : grille de cartes, portrait porteur d'identité, bandeau de nom collé
//! sous le buste.
//!
//! **Le niveau n'est pas repris** : `wakfu.log` ne le porte nulle part (vérifié sur
//! `crates/overlay-engine/tests/wakfu.log` — les seuls « lvl 185 » du fichier sont des messages du
//! canal Recrutement), et `RosterCharacter` n'a pas de champ pour lui.
//!
//! ## Ce que le portage demandera, et qui n'existe pas encore
//!
//! 1. **Un roster ÉDITABLE côté moteur.** `RosterIndex::from_settings_json` jette `id`, `label` et
//!    `isDefault` ; or `PATCH /api/v1/settings` **remplace la valeur entière de la clé** —
//!    réécrire `roster` sans ces champs effacerait les comptes du site. Garder le JSON brut, comme
//!    `AccountSettings::profile_raw` le fait déjà pour `profile`.
//! 2. **`patch_roster`** dans `overlay_sync::client`, sur le modèle de `patch_chat_filters`.
//! 3. **Un chargeur d'avatars dans `overlay-ui`** — [`Avatars`] en est le brouillon, gris
//!    précalculé compris.
//! 4. **Une icône de crayon** (voir plus haut).
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use std::collections::HashMap;
use std::sync::Mutex;

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Color32, Pos2, Rect, RichText, Vec2};
use egui_kittest::Harness;
use overlay_engine::class_breed::CLASS_PORTRAIT_ORDER;
use overlay_engine::Gender;
use overlay_ui::design::{self, DsIcon, IconContext, InputSize};
use overlay_ui::panels::bulk_select::{self, BulkHeader, BulkSelection};
use overlay_ui::panels::options_modal::{self, OptionsTab};
use overlay_ui::ui_icons::UiIcons;

// -------------------------------------------------------------------------------------------
// Jetons — chacun dit d'où il vient.
// -------------------------------------------------------------------------------------------

/// Fond derrière la fenêtre — le même que `tests/panels.rs` et les autres planches.
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
/// Voile posé sur une tuile survolée — `tokens::LEGEND_TILE_HOVER_SCRIM`.
const TILE_HOVER_SCRIM: Color32 = Color32::from_black_alpha(0x66);

/// Aplat du bandeau de nom. `panels::alerts_tab::SETTING_ROW_FILL` — **le même pour tous**, sans
/// exception : le turquoise « personnage actif » du jet précédent est retiré (2026-09-16).
const BAND: Color32 = Color32::from_rgb(0x26, 0x28, 0x2B);

/// Cadre d'une tuile — `design::legend_tile` (`tokens::LEGEND_TILE_FILL` / `_BORDER`).
const TILE_FILL: Color32 = Color32::from_rgb(0x0E, 0x11, 0x15);
const TILE_BORDER: Color32 = Color32::from_rgb(0x59, 0x51, 0x40);
const TILE_BORDER_WIDTH: f32 = 2.0;
/// Le même cadre, éclairci, pour la tuile survolée : le voile seul se lit sur une carte d'objet
/// claire, pas sur un buste déjà sombre.
const TILE_BORDER_HOVER: Color32 = Color32::from_rgb(0x9A, 0x8C, 0x6E);
/// **Arrondi des tuiles** — celui du champ de saisie (`tokens::INPUT_RADIUS`, 4), demandé le
/// 2026-09-16 : « un peu plus forcé, le même que la bordure de l'input texte ».
const TILE_RADIUS: u8 = design::tokens::INPUT_RADIUS;

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

/// Les 36 bustes détourés, **en couleur et en gris**.
///
/// Le gris est précalculé au chargement (luminance Rec. 601, alpha préservé), exactement comme
/// `portraits::to_grayscale` : un `tint()` egui multiplie la couleur sans désaturer — il assombrit
/// un portrait coloré, il ne le rend jamais gris. C'est ce qui permet la grille de classes « tout
/// gris, coloré sous le curseur ».
struct Avatars {
    color: HashMap<(&'static str, Gender), egui::TextureHandle>,
    grey: HashMap<(&'static str, Gender), egui::TextureHandle>,
}

impl Avatars {
    fn load(ctx: &egui::Context) -> Self {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../overlay-ui/assets/avatars");
        let mut color = HashMap::with_capacity(CLASS_PORTRAIT_ORDER.len() * 2);
        let mut grey = HashMap::with_capacity(CLASS_PORTRAIT_ORDER.len() * 2);
        for class in CLASS_PORTRAIT_ORDER {
            for (gender, suffix) in [(Gender::F, 'f'), (Gender::M, 'm')] {
                let path = dir.join(format!("{class}-{suffix}.png"));
                let bytes = std::fs::read(&path)
                    .unwrap_or_else(|err| panic!("avatar {} illisible : {err}", path.display()));
                let decoded = image::load_from_memory(&bytes)
                    .unwrap_or_else(|err| panic!("avatar {} invalide : {err}", path.display()))
                    .to_rgba8();
                let (width, height) = decoded.dimensions();
                let size = [width as usize, height as usize];
                color.insert(
                    (class, gender),
                    ctx.load_texture(
                        format!("avatar-{class}-{suffix}"),
                        egui::ColorImage::from_rgba_unmultiplied(size, decoded.as_raw()),
                        egui::TextureOptions::LINEAR,
                    ),
                );
                let mut gris = decoded.clone();
                for pixel in gris.pixels_mut() {
                    let [r, g, b, _] = pixel.0;
                    let luma = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                    pixel.0[0] = luma;
                    pixel.0[1] = luma;
                    pixel.0[2] = luma;
                }
                grey.insert(
                    (class, gender),
                    ctx.load_texture(
                        format!("avatar-{class}-{suffix}-gris"),
                        egui::ColorImage::from_rgba_unmultiplied(size, gris.as_raw()),
                        egui::TextureOptions::LINEAR,
                    ),
                );
            }
        }
        Self { color, grey }
    }

    fn texture(&self, class: &str, gender: Gender, gris: bool) -> Option<egui::TextureId> {
        let table = if gris { &self.grey } else { &self.color };
        table
            .iter()
            .find(|((name, sex), _)| *name == class && *sex == gender)
            .map(|(_, handle)| handle.id())
    }
}

// -------------------------------------------------------------------------------------------
// Les données de démonstration — les personnages réels de l'utilisateur, relevés sur ses captures
// du panneau de héros. Les classes sont déduites de leurs noms latins.
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

/// Les 18 classes dans l'ordre du jeu, avec le nom que porte leur **infobulle**.
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

/// Minuscules **sans accents** — ce sur quoi la recherche de classe compare.
///
/// Écrit à la main plutôt que tiré d'une crate de normalisation Unicode : quatre des dix-huit noms
/// portent un accent (Féca, Xélor, Crâ, Éliotrope), tous latins-1, et une dépendance de plus dans
/// `overlay-ui` pour ça ne se justifie pas. Le portage gardera la même fonction, au même endroit
/// que le filtre.
fn normalise(texte: &str) -> String {
    texte
        .chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            'ñ' => 'n',
            autre => autre,
        })
        .collect()
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

/// Où la planche doit poser le pointeur, relevé **pendant** la frame par ce qui peint la cible.
///
/// Une infobulle ne se simule pas : `design::tooltip` ne s'ouvre que sur une `Response` réellement
/// survolée. Écrire les coordonnées à la main dans la planche, c'est les voir se décaler au premier
/// ajustement de cote — le peintre les publie donc lui-même, et la planche s'y rend.
static CIBLE: Mutex<Option<Pos2>> = Mutex::new(None);

fn viser(rect: Rect, condition: bool) {
    if condition {
        *CIBLE.lock().unwrap() = Some(rect.center());
    }
}

/// Fait tourner le harnais, pose le pointeur sur la dernière cible publiée, refait tourner.
fn survoler(harness: &mut Harness<'static>) {
    *CIBLE.lock().unwrap() = None;
    harness.run();
    let cible = *CIBLE.lock().unwrap();
    if let Some(pos) = cible {
        harness.hover_at(pos);
        // Deux tours : le premier ouvre l'infobulle, le second la peint à sa place définitive.
        harness.run();
        harness.run();
    }
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

/// Largeur qu'occuperait `text` sans contrainte — ce qui permet de savoir si l'ellipse a mordu.
fn text_width(ui: &egui::Ui, text: &str, font: egui::FontId) -> f32 {
    let mut job = LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font,
            color: TEXT,
            ..Default::default()
        },
    );
    ui.fonts_mut(|f| f.layout_job(job)).size().x
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
) {
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
}

/// Le buste d'un personnage, peint à sa taille NATIVE dans `rect`. Repli sur le portrait générique
/// d'`UiIcons` quand la classe est inconnue — jamais un trou.
fn paint_avatar(
    ui: &egui::Ui,
    rect: Rect,
    class: &str,
    gender: Gender,
    gris: bool,
    avatars: &Avatars,
    icons: &UiIcons,
) {
    let texture = avatars
        .texture(class, gender, gris)
        .unwrap_or_else(|| icons.unknown_entity_texture().id());
    ui.painter().image(
        texture,
        rect,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// Un glyphe peint à l'échelle demandée, sans socle — le « + » de la tuile d'ajout.
fn paint_glyph(ui: &egui::Ui, center: Pos2, icon: DsIcon, side: f32, tint: Color32) {
    let ds = design::DesignSystem::get(ui.ctx());
    let native = ds.icon_native_size(icon);
    let fit = native.x.max(native.y);
    ds.paint_icon(
        ui.painter(),
        Rect::from_center_size(center, native * (side / fit)),
        icon,
        tint,
    );
}

/// **Un bouton de tuile** : un glyphe, une zone cliquable, une infobulle — et, quand `socle` le
/// demande, le disque qui dit « ceci est un bouton ».
///
/// **Le socle n'est PAS pour les deux** (correction du 2026-09-16) : il porte le bouton de
/// modification, posé au centre du buste, où rien d'autre ne signalerait qu'on peut cliquer. La
/// croix de retrait, elle, garde le glyphe nu des tuiles d'Alertes et de Chat — c'est l'idiome de
/// l'application, et l'entourer d'un disque l'aurait mise en avant alors qu'elle est le geste qu'on
/// ne veut PAS faire par mégarde.
fn tile_button(
    ui: &mut egui::Ui,
    tuile: Rect,
    center: Pos2,
    icon: DsIcon,
    tooltip: &str,
    socle: bool,
    id: egui::Id,
) -> egui::Response {
    let disc = Rect::from_center_size(center, Vec2::splat(BADGE_DISC));
    let response = ui.interact(disc, id, egui::Sense::click());
    let survol = response.contains_pointer();
    if socle {
        let painter = ui.painter();
        painter.circle_filled(center, BADGE_DISC / 2.0, BADGE_DISC_FILL);
        painter.circle_stroke(
            center,
            BADGE_DISC / 2.0,
            egui::Stroke::new(
                1.0,
                if survol {
                    design::tokens::TEXT_GOLD
                } else {
                    TILE_BORDER_HOVER
                },
            ),
        );
    }
    paint_glyph(
        ui,
        center,
        icon,
        BADGE,
        if survol {
            design::tokens::TEXT_GOLD
        } else {
            design::tokens::ICON_TINT
        },
    );
    // **Ancrée sur la tuile, pas sur le bouton** : ancrée sur lui, l'infobulle de « Modifier »
    // s'ouvrait juste au-dessus de son socle, c'est-à-dire par-dessus le bouton « Supprimer ».
    design::tooltip(&response).anchor(tuile).text(tooltip);
    response
}

/// La ligne « compte + serveur ». Les deux boutons de droite portent sur le COMPTE, jamais sur un
/// personnage : « + » ouvre la modale de création de compte, la corbeille retire celui qui est
/// choisi — éteinte sur le compte principal, que le site interdit de supprimer (`isDefault`).
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

// -------------------------------------------------------------------------------------------
// La grille de personnages
// -------------------------------------------------------------------------------------------

/// Largeur d'une tuile : **80 de buste et 10 de marge de chaque côté** (décision du 2026-09-16).
const TILE_W: f32 = AVATAR_SIZE + 2.0 * TILE_PAD;
/// La marge, qui vaut aussi pour le haut — le buste est à la même distance des trois bords.
const TILE_PAD: f32 = 10.0;
/// Hauteur du bandeau de nom, relevée sur les captures du panneau de héros.
const TILE_BAND: f32 = 22.0;
/// 2 de cadre + 10 + 80 de buste + 2 + 22 de bandeau + 2 de cadre.
const TILE_H: f32 = 118.0;
/// Côté du glyphe d'un badge révélé au survol.
const BADGE: f32 = 14.0;
/// Diamètre du socle rond qui le porte — **les deux badges ont le même** (2026-09-16) : c'est ce
/// socle qui dit « ceci est un bouton », et deux formats différents diraient deux choses.
const BADGE_DISC: f32 = 26.0;
/// Distance de la croix nue au coin haut-droit — `alerts_tab::TILE_BADGE_INSET`, l'idiome des
/// tuiles d'Alertes et de Chat, que ce bouton-là reprend tel quel.
const CROSS_INSET: f32 = 8.0;
/// Fond du socle — assez opaque pour détacher le glyphe du buste, assez sombre pour rester du jeu.
const BADGE_DISC_FILL: Color32 = Color32::from_black_alpha(0xB4);

/// Ce que la planche montre — un seul état à la fois, forcé plutôt que simulé au pointeur : une
/// position de souris en dur dans une planche se décale au premier ajustement de cote.
#[derive(Clone, Copy, PartialEq)]
enum Etat {
    Repos,
    /// Tuile survolée, pointeur sur le **bandeau de nom** : voile, deux boutons, et l'infobulle du
    /// nom complet si l'ellipse a mordu.
    Survol(usize),
    /// Tuile survolée, pointeur sur le **bouton de modification** : son infobulle, et pas celle du
    /// nom — deux infobulles à la fois se recouvriraient.
    SurvolBouton(usize),
    /// Mode « suppression multiple » : cases à cocher, et ces rangs-là cochés.
    Selection(&'static [usize]),
    /// Déplacement en vol — `pris` a quitté sa place, `vise` est la destination sous le pointeur.
    Deplacement {
        pris: usize,
        vise: usize,
    },
}

impl Etat {
    /// La tuile est-elle survolée, quel que soit l'endroit ?
    fn survol(self, index: usize) -> bool {
        matches!(self, Etat::Survol(i) | Etat::SurvolBouton(i) if i == index)
    }
    /// Le pointeur est-il sur le bouton de modification de cette tuile ?
    fn sur_bouton(self, index: usize) -> bool {
        matches!(self, Etat::SurvolBouton(i) if i == index)
    }
    fn selection(self) -> Option<&'static [usize]> {
        match self {
            Etat::Selection(cochees) => Some(cochees),
            _ => None,
        }
    }
}

fn band_rect(tile: Rect) -> Rect {
    Rect::from_min_max(
        egui::pos2(
            tile.left() + TILE_BORDER_WIDTH,
            tile.bottom() - TILE_BORDER_WIDTH - TILE_BAND,
        ),
        egui::pos2(
            tile.right() - TILE_BORDER_WIDTH,
            tile.bottom() - TILE_BORDER_WIDTH,
        ),
    )
}

/// **La tuile « + », toujours la première de la grille.** À quarante personnages, une tuile d'ajout
/// posée à la fin obligerait à défiler jusqu'en bas pour en créer un de plus — décision du
/// 2026-09-16. Elle ouvre la modale de personnage.
///
/// **Ni bandeau ni libellé** (2026-09-16) : elle n'a pas de nom à porter, et un bandeau vide en
/// bas de cadre lui donnait l'air d'une tuile de personnage à qui il manquerait quelque chose. Le
/// « + » est donc centré dans le cadre entier, sur les deux axes.
fn add_tile(ui: &mut egui::Ui, size: Vec2, survolee: bool) {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let painter = ui.painter().clone();
    let survol = survolee || response.contains_pointer();
    let teinte = if survol {
        design::tokens::TEXT_GOLD
    } else {
        design::tokens::ICON_TINT
    };
    painter.rect_filled(rect, TILE_RADIUS, TILE_FILL);
    painter.rect_stroke(
        rect,
        TILE_RADIUS,
        egui::Stroke::new(
            TILE_BORDER_WIDTH,
            if survol {
                design::tokens::TEXT_GOLD
            } else {
                TILE_BORDER
            },
        ),
        egui::StrokeKind::Inside,
    );
    paint_glyph(ui, rect.center(), DsIcon::Plus, 30.0, teinte);
    design::tooltip(&response).text("Ajouter un personnage à ce compte");
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
    avatars: &Avatars,
    icons: &UiIcons,
) {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let painter = ui.painter().clone();
    let vise = matches!(etat, Etat::Deplacement { vise, .. } if vise == index);
    let pris = matches!(etat, Etat::Deplacement { pris, .. } if pris == index);
    let survol = etat.survol(index);
    let cochee = etat.selection().map(|c| c.contains(&index));

    painter.rect_filled(rect, TILE_RADIUS, TILE_FILL);
    // **Un seul porteur de l'or à la fois** : la destination d'un déplacement, ou la tuile cochée.
    let bordure = match (vise, cochee, survol) {
        (true, ..) => design::tokens::ITEM_SLOT_SELECTED_BORDER,
        (_, Some(true), _) => design::tokens::ITEM_SLOT_SELECTED_BORDER,
        (_, _, true) => TILE_BORDER_HOVER,
        _ => TILE_BORDER,
    };
    painter.rect_stroke(
        rect,
        TILE_RADIUS,
        egui::Stroke::new(TILE_BORDER_WIDTH, bordure),
        egui::StrokeKind::Inside,
    );

    paint_avatar(
        ui,
        Rect::from_min_size(
            egui::pos2(rect.center().x - AVATAR_SIZE / 2.0, rect.top() + TILE_PAD),
            Vec2::splat(AVATAR_SIZE),
        ),
        perso.class,
        perso.gender,
        false,
        avatars,
        icons,
    );

    let band = band_rect(rect);
    painter.rect_filled(band, 0.0, BAND);
    let font = design::text::label_strong_font(ui.ctx(), 12.5);
    let dispo = band.width() - 10.0;
    // **L'ellipse déclenche l'infobulle, et elle seule** : un nom qui tient en entier n'a rien à
    // redire, et une infobulle qui répète le libellé visible est du bruit. Même règle que le
    // `tooltipOnlyIfTruncated` du site.
    let coupe = text_width(ui, perso.name, font.clone()) > dispo;
    painted_ellipsed(
        ui,
        &painter,
        perso.name,
        band.center(),
        dispo,
        font,
        TEXT,
        true,
    );
    if coupe && !etat.sur_bouton(index) {
        design::tooltip(&response).anchor(rect).text(perso.name);
        viser(band, matches!(etat, Etat::Survol(i) if i == index));
    }

    if let Some(cochee) = cochee {
        // Coin haut-droit, comme `design::legend_tile` : la case remplace les badges de survol,
        // les deux ne s'affichent jamais ensemble.
        design::paint_checkbox(
            ui,
            Rect::from_min_size(
                egui::pos2(
                    rect.right() - TILE_BORDER_WIDTH - 4.0 - design::tokens::CHECKBOX_SIZE,
                    rect.top() + TILE_BORDER_WIDTH + 4.0,
                ),
                Vec2::splat(design::tokens::CHECKBOX_SIZE),
            ),
            cochee,
            Color32::WHITE,
        );
        return;
    }

    if pris {
        // La place d'origine s'efface derrière le fantôme, maintenant seul exemplaire de l'entrée —
        // `panels::tile_reorder`, à la lettre.
        painter.rect_filled(rect, TILE_RADIUS, DRAG_SOURCE_SCRIM);
    }
    if survol {
        // Le voile s'arrête au bord haut du bandeau : le couvrir rendrait le nom gris sur gris à
        // l'instant où le pointeur l'atteint, c'est-à-dire exactement quand on le lit.
        painter.rect_filled(
            Rect::from_min_max(rect.min, egui::pos2(rect.right(), band.top()))
                .shrink(TILE_BORDER_WIDTH),
            0.0,
            TILE_HOVER_SCRIM,
        );
        // **Les deux boutons sont LOIN l'un de l'autre, et c'est la règle** (2026-09-16) :
        // modifier est le geste courant, retirer est celui qui ouvre une confirmation. Côte à
        // côte, un pointeur qui glisse d'un pixel demande à supprimer. « Modifier » prend donc le
        // CENTRE — c'est là que le pointeur arrive quand on vise une tuile — et « retirer » reste
        // au coin, où l'on ne va que si on y va exprès.
        //
        let modifier = tile_button(
            ui,
            rect,
            egui::pos2(rect.center().x, band.top() - AVATAR_SIZE / 2.0 - 4.0),
            DsIcon::Edit,
            "Modifier",
            true,
            egui::Id::new(("personnages.modifier", index)),
        );
        tile_button(
            ui,
            rect,
            egui::pos2(
                rect.right() - CROSS_INSET - BADGE / 2.0,
                rect.top() + CROSS_INSET + BADGE / 2.0,
            ),
            DsIcon::Close,
            "Supprimer",
            false,
            egui::Id::new(("personnages.retirer", index)),
        );
        let _ = modifier;
        viser(
            Rect::from_center_size(
                egui::pos2(rect.center().x, band.top() - AVATAR_SIZE / 2.0 - 4.0),
                Vec2::splat(BADGE_DISC),
            ),
            etat.sur_bouton(index),
        );
    }
}

/// Gouttière minimale entre deux tuiles — `alerts_tab::TILE_GAP`. La grille en pose davantage
/// quand la largeur restante le permet : les tuiles ont une largeur FIXE (100), c'est donc
/// l'espacement qui absorbe le reste, jamais la tuile qui s'étire.
const TILE_GAP_MIN: f32 = 10.0;

/// L'écran : en-tête, ligne de compte, titre de liste avec ses commandes, grille.
#[allow(clippy::too_many_arguments)]
fn ecran(
    ui: &mut egui::Ui,
    scene: &Scene<'_>,
    compte: &mut usize,
    serveur: &mut usize,
    select_mode: &mut bool,
    cochees_keys: &mut Vec<String>,
    etat: Etat,
    liste: &[Perso],
) {
    let width = scene.panel.inner.width();
    ui.add(design::heading("Personnages"));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP);

    account_row(ui, compte, serveur, width);
    ui.add_space(SECTION_GAP * 0.75);

    // Le titre de la liste et ses commandes de suppression multiple — le même en-tête qu'au Suivi,
    // aux Alertes et au Chat, avec la règle qui va avec : aucun bouton tant qu'il n'y a rien à
    // supprimer.
    let _ = bulk_select::show(
        ui,
        width,
        BulkHeader {
            title: "Personnages du compte",
            removable: liste.len(),
            bulk_tooltip: "Retire les personnages cochés de ce compte — annulable tant que la \
                           fenêtre n'est pas validée.",
            log_prefix: "personnages",
            enabled: true,
        },
        BulkSelection {
            mode: select_mode,
            keys: cochees_keys,
        },
    );
    ui.add_space(bulk_select::HEADER_GAP);

    // **Avant la grille, jamais après** : la zone défilable prend toute la hauteur restante, et
    // un bloc ajouté à sa suite se peignait PAR-DESSUS la tuile « Nouveau » (relevé le
    // 2026-09-16). Ce n'était pas un artefact de planche, c'était le rendu réel.
    if liste.is_empty() {
        ui.add(
            design::info_text(
                "Aucun personnage sur ce compte. Cliquez sur la tuile « + » : le nom demandé est \
                 celui qui apparaît en jeu, au caractère près — c'est ce que l'overlay cherche \
                 dans le journal.",
            )
            .width(width)
            .log_name("personnages.vide"),
        );
        ui.add_space(SECTION_GAP * 0.75);
    }

    let mut fantome: Option<(Rect, usize)> = None;
    scene
        .panel
        .scroll_area(ui, "personnages.grille", |ui, content_width| {
            // Le plus grand nombre de tuiles qui tienne AVEC la gouttière minimale entre elles —
            // et non une division qui compterait une gouttière de trop au bout de la rangée.
            let cols = (1..=8)
                .take_while(|n| {
                    *n as f32 * TILE_W + (*n as f32 - 1.0) * TILE_GAP_MIN <= content_width
                })
                .last()
                .unwrap_or(1);
            let gap = if cols > 1 {
                (content_width - cols as f32 * TILE_W) / (cols - 1) as f32
            } else {
                0.0
            };
            ui.spacing_mut().item_spacing = Vec2::new(gap, TILE_GAP_MIN);
            let size = Vec2::new(TILE_W, TILE_H);
            // La tuile « + » occupe le rang 0 de la PREMIÈRE rangée ; les personnages suivent.
            // **La tuile « + » disparaît pendant une suppression multiple** : le mode est exclusif,
            // on retire ou on ajoute, jamais les deux dans le même geste.
            let ajout = etat.selection().is_none();
            let mut rang = 0usize;
            let mut index = 0usize;
            while index < liste.len() || (rang == 0 && ajout) {
                ui.horizontal(|ui| {
                    let mut colonne = 0usize;
                    if rang == 0 && ajout {
                        add_tile(ui, size, false);
                        colonne += 1;
                    }
                    while colonne < cols && index < liste.len() {
                        hero_tile(
                            ui,
                            &liste[index],
                            index,
                            size,
                            etat,
                            scene.avatars,
                            scene.icons,
                        );
                        if let Etat::Deplacement { pris, vise } = etat {
                            if vise == index {
                                fantome = Some((ui.min_rect(), pris));
                            }
                        }
                        index += 1;
                        colonne += 1;
                    }
                });
                rang += 1;
            }
        });

    // Le fantôme, peint EN DERNIER et au-dessus de tout : c'est ce qu'on tient, il ne peut pas
    // passer sous une tuile voisine.
    if let Some((rangee, pris)) = fantome {
        let cible = Rect::from_min_size(
            egui::pos2(rangee.right() - TILE_W, rangee.top()),
            Vec2::new(TILE_W, TILE_H),
        );
        let ghost = Rect::from_min_size(cible.min + Vec2::new(-16.0, -12.0), cible.size());
        let painter = ui.painter().clone();
        painter.rect_filled(ghost, TILE_RADIUS, TILE_FILL);
        painter.rect_stroke(
            ghost,
            TILE_RADIUS,
            egui::Stroke::new(TILE_BORDER_WIDTH, TILE_BORDER),
            egui::StrokeKind::Inside,
        );
        let perso = &liste[pris];
        paint_avatar(
            ui,
            Rect::from_min_size(
                egui::pos2(ghost.center().x - AVATAR_SIZE / 2.0, ghost.top() + TILE_PAD),
                Vec2::splat(AVATAR_SIZE),
            ),
            perso.class,
            perso.gender,
            false,
            scene.avatars,
            scene.icons,
        );
        let band = band_rect(ghost);
        painter.rect_filled(band, 0.0, BAND);
        painted_ellipsed(
            ui,
            &painter,
            perso.name,
            band.center(),
            band.width() - 10.0,
            design::text::label_strong_font(ui.ctx(), 12.5),
            TEXT,
            true,
        );
    }
}

// -------------------------------------------------------------------------------------------
// Les modales
// -------------------------------------------------------------------------------------------

/// Ouvre une couche AU-DESSUS du panneau et y peint le voile — l'idiome de
/// `design::confirm_dialog`. Sans elle, le clip du panneau rognerait la bannière de la modale.
///
/// **`Order::Middle` et non `Foreground`**, contrairement à la boîte de confirmation : le panneau
/// de `design::autocomplete` est une `egui::Area` en `Foreground`, et deux couches de MÊME ordre
/// se départagent par leur identifiant — la modale passait donc par-dessus le panneau de
/// suggestions, qui disparaissait purement et simplement. `Middle` couvre tout le contenu de la
/// fenêtre (qui vit dans la couche de base) et laisse le popup au-dessus, là où il doit être.
fn couche_modale(ui: &mut egui::Ui, window: Rect, nom: &str) -> egui::Ui {
    let mut couche = ui.new_child(egui::UiBuilder::new().max_rect(window).layer_id(
        egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new(("personnages.modale", nom.to_owned())),
        ),
    ));
    couche.set_clip_rect(Rect::EVERYTHING);
    couche.painter().rect_filled(
        window,
        0.0,
        Color32::from_black_alpha(design::tokens::CONFIRM_SCRIM_ALPHA),
    );
    let _ = &ui;
    couche
}

/// Taille de la modale de personnage. **640 de large et non 600** : la largeur est commandée par
/// la grille de classes (voir [`COLONNE`]), et il faut qu'elle tienne avec de la marge des deux
/// côtés.
const PERSO_MODALE: Vec2 = Vec2::new(640.0, 620.0);
const CLASSE_COLS: usize = 6;
const CLASSE_GAP: f32 = 10.0;

/// **Largeur utile de la modale — celle de la grille de classes, et elle commande tout le reste.**
///
/// Décision du 2026-09-16, après un rendu où le champ de saisie s'arrêtait 16 points avant la
/// dernière colonne de portraits : les marges doivent être les MÊMES pour les champs, le switch et
/// les portraits. Comme une tuile de classe a une largeur fixe (le buste natif) et que leur nombre
/// est fixe, c'est la grille qui donne la mesure — et non l'inverse. Tout se pose donc dans une
/// colonne de cette largeur, **centrée** dans le panneau, quitte à ne pas coller aux marges
/// standard de section.
const COLONNE: f32 = CLASSE_COLS as f32 * AVATAR_SIZE + (CLASSE_COLS as f32 - 1.0) * CLASSE_GAP;

/// Les personnages **déjà vus dans `wakfu.log`**, avec la classe et le sexe lus sur leur ligne
/// `[_FL_]` d'entrée en combat. Relevés sur le fichier de référence de l'utilisateur.
const VUS_AU_JOURNAL: &[(&str, &str, Gender)] = &[
    ("Anonyme-Zobal1", "zobal", Gender::F),
    ("Anonyme-Sadida1", "sadida", Gender::M),
    ("Anonyme-Huppermage1", "huppermage", Gender::F),
    ("Anonyme-Ecaflip1", "ecaflip", Gender::M),
    ("Anonyme-Ouginak1", "ouginak", Gender::M),
];

/// **La modale de personnage — création ET modification.** Une seule, décision du 2026-09-16 :
/// modifier, c'est reprendre les trois mêmes champs (nom, sexe, classe) déjà remplis.
///
/// Elle s'intitule **« Personnage »**, sans « Nouveau » : la même fenêtre sert aux deux, et un
/// titre qui annoncerait une création mentirait une fois sur deux.
#[allow(clippy::too_many_arguments)]
fn modale_personnage(
    ui: &mut egui::Ui,
    window: Rect,
    nom: &mut String,
    recherche: &mut String,
    genre: &mut Gender,
    choisie: Option<usize>,
    survolee: Option<usize>,
    suggestions: bool,
    avatars: &Avatars,
    icons: &UiIcons,
) {
    let mut couche = couche_modale(ui, window, "personnage");
    let rect = Rect::from_center_size(window.center(), PERSO_MODALE);
    let mut modale = couche.new_child(egui::UiBuilder::new().max_rect(rect));
    modale.set_clip_rect(Rect::EVERYTHING);
    let chrome = design::window("Personnage")
        .footer("Annuler", "Valider")
        .close_button(true)
        .log_name("personnages.modale")
        .show(&mut modale);

    design::panel().show(&mut modale, chrome.content, |ui, _panel| {
        // La colonne centrée : tout ce qui suit s'y pose, donc tout est aligné sur la grille.
        let zone = ui.max_rect();
        let colonne = Rect::from_min_max(
            egui::pos2(zone.center().x - COLONNE / 2.0, zone.top()),
            egui::pos2(zone.center().x + COLONNE / 2.0, zone.bottom()),
        );
        let mut ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(colonne)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let ui = &mut ui;

        // 1. Le nom, en tête : c'est ce qu'on vient écrire.
        //
        // **Un `design::autocomplete` et non un champ nu** (décision du 2026-09-16) : l'overlay lit
        // `wakfu.log`, il connaît donc des noms — avec leur classe et leur sexe — que le site ne
        // connaîtra jamais. Il les PROPOSE, il ne les déclare pas : un allié d'un autre joueur peut
        // figurer dans la liste, il n'y fera jamais qu'y figurer. Et la faute de frappe disparaît,
        // alors que c'est la seule erreur de cet écran qui soit invisible et casse tout.
        let entrees: Vec<design::AutocompleteEntry> = VUS_AU_JOURNAL
            .iter()
            .map(|(vu, class, gender)| {
                let mut entree = design::AutocompleteEntry::new(*vu, 0);
                entree.image = avatars.texture(class, *gender, false);
                entree
            })
            .collect();
        let mut champ = design::autocomplete(nom)
            .placeholder("Nom du personnage, exactement comme en jeu…")
            .entries(&entrees)
            .width(COLONNE)
            // **Ni loupe, ni champ vidé** (2026-09-16) : ce n'est pas une recherche, c'est le nom
            // du personnage. Il s'écrit librement — le composant ne valide rien, un nom qu'aucune
            // suggestion ne porte sort du champ tel quel — et choisir une suggestion le REMPLIT au
            // lieu de l'effacer. Les deux réglages ont été ajoutés au composant pour ce cas.
            .search_icon(false)
            .fill_on_select(true)
            .log_name("personnages.modale.nom");
        if suggestions {
            champ = champ.preview_open(true).preview_active(0);
        }
        champ.show(ui);
        ui.add_space(12.0);

        // 2. Sexe à gauche, recherche de classe à droite — une seule ligne, les deux filtres de la
        //    grille qui suit.
        let row = ui
            .allocate_space(Vec2::new(COLONNE, design::tokens::SWITCH_HEIGHT))
            .1;
        let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
        cell.spacing_mut().item_spacing.x = 0.0;
        cell.horizontal_centered(|ui| {
            // **Le premier slot est le masculin, et c'est le défaut.** Le choix vit chez l'appelant
            // et ne se réinitialise pas d'une création à la suivante : qui déclare six mules
            // féminines ne reclique pas six fois. Il ne survit PAS à la fermeture de l'overlay
            // (décision du 2026-09-16) — au démarrage suivant, le défaut revient.
            ui.add(
                design::switch(genre)
                    .slot(Gender::M, "Masculin")
                    .icon(DsIcon::Male)
                    .slot(Gender::F, "Féminin")
                    .icon(DsIcon::Female)
                    .log_name("personnages.modale.sexe"),
            );
            let mut droite = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(row)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            droite.add(
                design::input(recherche)
                    .size(InputSize::Search)
                    .leading_icon(DsIcon::Search)
                    .placeholder("Rechercher une classe…")
                    .clearable(true)
                    .width(270.0)
                    .log_name("personnages.modale.recherche"),
            );
        });
        ui.add_space(16.0);

        // 3. Les classes. **Le filtre ne mord qu'à trois caractères** — le seuil de
        //    `tokens::AUTOCOMPLETE_MIN_QUERY_LEN`, déjà celui du champ ci-dessus — et compare sur
        //    une forme sans casse ni accents (voir `normalise`), sans quoi « cra » ne trouverait
        //    pas « Crâ » et « eli » manquerait « Éliotrope ».
        let requete = normalise(recherche);
        let filtre = (requete.chars().count() >= design::tokens::AUTOCOMPLETE_MIN_QUERY_LEN)
            .then_some(requete.as_str());
        let visibles: Vec<(usize, &(&str, &str))> = CLASSES
            .iter()
            .enumerate()
            .filter(|(_, (_, label))| match filtre {
                Some(q) => normalise(label).contains(q),
                None => true,
            })
            .collect();

        ui.spacing_mut().item_spacing = Vec2::splat(CLASSE_GAP);
        for chunk in visibles.chunks(CLASSE_COLS) {
            ui.horizontal(|ui| {
                for (index, (class, label)) in chunk {
                    // **La tuile fait exactement la taille du buste** (décision du 2026-09-16) :
                    // un liseré fin l'entoure, quitte à rogner un pixel de l'image aux coins.
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::splat(AVATAR_SIZE), egui::Sense::click());
                    let survol = survolee == Some(*index) || response.contains_pointer();
                    let retenue = choisie == Some(*index);
                    // **Tout est gris, sauf ce qu'on vise.** Le même geste que la galerie
                    // d'avatars : la couleur suit le curseur, et la classe retenue la garde.
                    paint_avatar(
                        ui,
                        rect,
                        class,
                        *genre,
                        !(survol || retenue),
                        avatars,
                        icons,
                    );
                    ui.painter().rect_stroke(
                        rect,
                        TILE_RADIUS,
                        egui::Stroke::new(
                            if survol || retenue { 2.0 } else { 1.0 },
                            if survol || retenue {
                                design::tokens::TEXT_GOLD
                            } else {
                                TILE_BORDER
                            },
                        ),
                        egui::StrokeKind::Inside,
                    );
                    // Le nom de la classe n'est plus écrit sous le portrait : il est dans
                    // l'infobulle, au-dessus de la tuile visée.
                    design::tooltip(&response).text(*label);
                    viser(rect, survolee == Some(*index));
                }
            });
        }
        if visibles.is_empty() {
            ui.add_space(20.0);
            ui.add(
                design::info_text("Aucune classe ne porte ce nom.")
                    .width(COLONNE)
                    .log_name("personnages.modale.vide"),
            );
        }
    });
}

/// Taille de la modale de compte — deux champs, rien de plus.
const COMPTE_MODALE: Vec2 = Vec2::new(460.0, 420.0);

/// **La modale de compte.** Un compte du roster, c'est un libellé et un serveur de jeu : le site
/// n'en demande pas plus (`CharacterRosterService.addAccount`). Le même écran renomme un compte
/// existant, pour la même raison que la modale de personnage en tient deux.
fn modale_compte(
    ui: &mut egui::Ui,
    window: Rect,
    titre: &str,
    nom: &mut String,
    serveur: &mut usize,
) {
    let mut couche = couche_modale(ui, window, titre);
    let rect = Rect::from_center_size(window.center(), COMPTE_MODALE);
    let mut modale = couche.new_child(egui::UiBuilder::new().max_rect(rect));
    modale.set_clip_rect(Rect::EVERYTHING);
    let chrome = design::window(titre)
        .footer("Annuler", "Valider")
        .close_button(true)
        .log_name("personnages.compte.modale")
        .show(&mut modale);

    design::panel().show(&mut modale, chrome.content, |ui, panel| {
        let width = panel.inner.width();
        ui.label(
            RichText::new("Nom du compte")
                .color(SUBDUED)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        );
        ui.add_space(6.0);
        ui.add(
            design::input(nom)
                .size(InputSize::Search)
                .placeholder("Mules, Métiers, Second écran…")
                .width(width)
                .log_name("personnages.compte.nom"),
        );
        ui.add_space(14.0);
        ui.label(
            RichText::new("Serveur de jeu")
                .color(SUBDUED)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        );
        ui.add_space(6.0);
        design::select(serveur)
            .option(0usize, "Pandora")
            .option(1usize, "Rubilax")
            .option(2usize, "Aucun")
            .width(220.0)
            .log_name("personnages.compte.serveur")
            .show(ui);
        ui.add_space(14.0);
        ui.add(
            design::info_text(
                "Le serveur départage deux personnages de même nom. Il se change plus tard.",
            )
            .width(width)
            .log_name("personnages.compte.aide"),
        );
    });
}

// -------------------------------------------------------------------------------------------
// Les planches
// -------------------------------------------------------------------------------------------

/// L'écran seul, dans l'état demandé.
fn planche_ecran(fichier: &str, etat: Etat, liste: &'static [Perso]) {
    let mut compte = 0usize;
    let mut serveur = 0usize;
    let mut mode = etat.selection().is_some();
    let mut keys: Vec<String> = etat
        .selection()
        .map(|c| c.iter().map(|i| liste[*i].name.to_owned()).collect())
        .unwrap_or_default();
    let mut harness = options_harness(move |ui, scene| {
        ecran(
            ui,
            &scene,
            &mut compte,
            &mut serveur,
            &mut mode,
            &mut keys,
            etat,
            liste,
        );
    });
    survoler(&mut harness);
    write_mockup(&mut harness, fichier);
}

/// L'écran, plus la modale de personnage par-dessus.
#[allow(clippy::too_many_arguments)]
fn planche_modale(
    fichier: &str,
    nom: &'static str,
    recherche: &'static str,
    genre: Gender,
    choisie: Option<usize>,
    survolee: Option<usize>,
    suggestions: bool,
) {
    let mut compte = 0usize;
    let mut serveur = 0usize;
    let mut mode = false;
    let mut keys: Vec<String> = Vec::new();
    let mut nom = nom.to_owned();
    let mut recherche = recherche.to_owned();
    let mut genre = genre;
    let mut harness = options_harness(move |ui, scene| {
        ecran(
            ui,
            &scene,
            &mut compte,
            &mut serveur,
            &mut mode,
            &mut keys,
            Etat::Repos,
            COMPTE_PRINCIPAL,
        );
        modale_personnage(
            ui,
            scene.window,
            &mut nom,
            &mut recherche,
            &mut genre,
            choisie,
            survolee,
            suggestions,
            scene.avatars,
            scene.icons,
        );
    });
    if suggestions {
        // Le panneau de suggestions est une `egui::Area` : sa taille n'est connue qu'après une
        // première passe, et une planche rendue en une seule frame le montrerait mal placé.
        harness.run();
        harness.run();
        harness.run();
    } else {
        survoler(&mut harness);
    }
    write_mockup(&mut harness, fichier);
}

/// **La création de compte.** Rien dans les jets précédents ne montrait ce geste, alors qu'un
/// roster multi-compte commence par là.
fn planche_compte() {
    let mut compte = 0usize;
    let mut serveur = 0usize;
    let mut mode = false;
    let mut keys: Vec<String> = Vec::new();
    let mut nom = String::from("Mules");
    let mut serveur_modale = 0usize;
    let mut harness = options_harness(move |ui, scene| {
        ecran(
            ui,
            &scene,
            &mut compte,
            &mut serveur,
            &mut mode,
            &mut keys,
            Etat::Repos,
            COMPTE_PRINCIPAL,
        );
        modale_compte(
            ui,
            scene.window,
            "Nouveau compte",
            &mut nom,
            &mut serveur_modale,
        );
    });
    harness.run();
    write_mockup(&mut harness, "personnages_b10_compte");
}

/// **La suppression d'un compte** — la question porte le nom du compte ET son décompte : un
/// « Supprimer ce compte ? » laisserait l'utilisateur ignorer ce qu'il emporte.
fn planche_compte_suppression() {
    let mut compte = 1usize;
    let mut serveur = 0usize;
    let mut mode = false;
    let mut keys: Vec<String> = Vec::new();
    let mut harness = options_harness(move |ui, scene| {
        ecran(
            ui,
            &scene,
            &mut compte,
            &mut serveur,
            &mut mode,
            &mut keys,
            Etat::Repos,
            COMPTE_PRINCIPAL,
        );
        let _ = design::confirm_dialog("Supprimer le compte « Mules » et ses 6 personnages ?")
            .over(scene.window)
            .log_name("personnages.compte.confirmation")
            .show(ui);
    });
    harness.run();
    write_mockup(&mut harness, "personnages_b11_compte_suppression");
}

fn main() {
    planche_ecran("personnages_b1_ecran", Etat::Repos, COMPTE_PRINCIPAL);
    println!("  b1 — l'écran au repos");
    planche_ecran("personnages_b2_survol", Etat::Survol(9), COMPTE_PRINCIPAL);
    println!("  b2 — survol : le nom complet en infobulle");
    planche_ecran(
        "personnages_b3_boutons",
        Etat::SurvolBouton(9),
        COMPTE_PRINCIPAL,
    );
    println!("  b3 — le bouton « Modifier », au centre");
    planche_ecran(
        "personnages_b4_selection",
        Etat::Selection(&[1, 4, 7]),
        COMPTE_PRINCIPAL,
    );
    println!("  b4 — suppression multiple");
    planche_modale(
        "personnages_b5_personnage",
        "",
        "",
        Gender::M,
        None,
        None,
        false,
    );
    println!("  b5 — la modale « Personnage », vierge");
    planche_modale(
        "personnages_b6_suggestion",
        "erz",
        "",
        Gender::M,
        None,
        None,
        true,
    );
    println!("  b6 — le nom se complète depuis le journal");
    planche_modale(
        "personnages_b7_classe",
        "Telum Novum",
        "",
        Gender::F,
        None,
        Some(8),
        false,
    );
    println!("  b7 — une classe sous le curseur");
    planche_modale(
        "personnages_b8_recherche",
        "Telum Novum",
        "eli",
        Gender::F,
        None,
        Some(15),
        false,
    );
    println!("  b8 — recherche de classe");
    planche_modale(
        "personnages_b9_modification",
        "Sagitta Lucis",
        "",
        Gender::F,
        Some(8),
        None,
        false,
    );
    println!("  b9 — la même modale, pré-remplie");
    planche_compte();
    println!("  b10 — création de compte");
    planche_compte_suppression();
    println!("  b11 — suppression d'un compte");
    planche_ecran(
        "personnages_b12_deplacement",
        Etat::Deplacement { pris: 9, vise: 5 },
        COMPTE_PRINCIPAL,
    );
    println!("  b12 — déplacement en vol");
    planche_ecran("personnages_b13_vide", Etat::Repos, &[]);
    println!("  b13 — aucun personnage");
    let dir = mockup_dir();
    let ecrites = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    let affiche = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("{ecrites} planches écrites dans {}", affiche.display());
}
