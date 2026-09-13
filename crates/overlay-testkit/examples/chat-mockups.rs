//! **Maquettes de l'onglet « Chat »** — version 2, après recadrage de l'utilisateur (2026-09-13).
//!
//! ## Ce que la version 1 faisait de trop, et pourquoi
//!
//! La v1 portait le panneau Chat du dépôt web tel quel : cases de canaux, fil de messages,
//! bouton « Aller en bas ». **C'était une erreur de périmètre**, relevée par l'utilisateur : le jeu
//! affiche déjà son chat, filtre déjà ses canaux — l'overlay n'a rien à en remontrer. Ce qui manque
//! au joueur, c'est une seule chose que le jeu ne fait pas : **être prévenu** quand un message
//! correspond à un critère qu'il a posé, par un son et une carte par-dessus le jeu, comme les
//! alertes de ramassage et de décompte du Suivi.
//!
//! L'onglet ne gère donc que **des recherches** : un canal (ou tous), un mot, et c'est tout. Tout ce
//! qui suit en découle.
//!
//! ## Ce que ces planches proposent
//!
//! - **Le formulaire est l'essentiel de l'onglet**, il n'est plus replié : canal d'abord, mot
//!   ensuite, « Ajouter » enfin — dans cet ordre, demandé explicitement.
//! - **Sans couleur de canal** : l'utilisateur peut avoir appliqué un thème au jeu, une couleur
//!   figée ne correspondrait plus à la sienne. Le canal est écrit, en préfixe, séparé du mot par un
//!   tiret : « Commerce — gelano ».
//! - **Deux présentations de la liste**, à comparer : en **lignes** (une recherche par ligne de
//!   réglage, croix à droite — [`chat_options_liste`]) et en **tuiles** (le mot au centre d'un cadre,
//!   le canal posé SUR la bordure haute comme une légende, croix révélée au survol comme les tuiles
//!   d'Alertes et de Suivi — [`chat_options_tuiles`]). Les tuiles montrent deux fois plus de
//!   recherches d'un coup.
//! - **La carte d'alerte** ([`chat_toast`]) reprend le gabarit exact du toast du Suivi
//!   (`panels::watchlist::toast_card` : aplat, bordure d'accent, titre en capitales, ombre) avec le
//!   message complet — sans confettis (ce n'est pas une célébration) et sans icône (il n'y a pas
//!   d'objet à montrer).
//! - **Cliquer la carte prépare une réponse en privé** : l'overlay écrit `/w "<auteur>"` dans le
//!   chat du jeu, le joueur n'a plus que son message à taper ([`chat_toast_survol`] montre
//!   l'infobulle qui l'annonce).
//! - **Le son est celui du web** (`chat-filter-*.mp3`, joué par `AlertSoundService.playChatFilter`),
//!   repris tel quel comme l'a été le son de ramassage — pas un son nouveau. « Tester le son »,
//!   comme dans Alertes, le fait entendre depuis l'onglet.
//!
//! Les composants sont **appelés**, jamais recopiés : `design::window`, `tabs`, `panel`,
//! `heading`, `select`, `input`, `button`, `icon_button`, `info_text`. Le chrome est celui de la
//! production, à sa taille de production. Les planches sont écrites dans `target/mockups/`, jamais
//! commitées.
//!
//! ## Ce que le portage demande, et qui n'existe pas encore
//!
//! 1. **Un détecteur côté moteur** — `LogEntry::Chat` est parsé mais jeté (`session.rs`) ; il faut
//!    y confronter chaque message aux recherches et émettre un événement d'alerte, que la fenêtre
//!    Options soit ouverte ou non. Même chemin que l'alerte de ramassage
//!    (`main.rs::spawn_engine_thread`).
//! 2. **Un toast de chat** — `WatchlistToast` est typé objet/monstre (icône de catalogue,
//!    confettis) ; la carte de chat en partage le gabarit mais pas le contenu. Soit une variante de
//!    `WatchlistToastReason`, soit une carte sœur dans le même bandeau.
//! 3. **Le son** — `alert_sound.rs` ne porte que le son de ramassage et le décompte ; embarquer
//!    `public/assets/sounds/chat-filter-c13da61f.mp3` du web comme les deux autres.
//! 4. **Écrire dans le chat du jeu** — le clic sur la carte doit donner le focus à la fenêtre
//!    du jeu puis y saisir `/w "<auteur>"` : injection de frappe par la plateforme
//!    (`overlay-platform`, `SendInput` sous Windows, XTest sous X11). La séquence exacte (faut-il
//!    d'abord Entrée pour ouvrir la saisie du chat ?) est à confirmer en jeu.
//! 5. **La persistance** — `chatFilters` est synchronisé au compte côté web (`/api/v1/settings`),
//!    comme `profile.soundItems` ; même contrat transactionnel que le reste de la fenêtre.
//! 6. **La règle de correspondance** — celle du web : minuscules, `contains` sur le texte OU
//!    l'auteur, un seul son par lot, jamais pendant la lecture initiale du fichier.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Color32, Rect, RichText, Vec2};
use egui_kittest::Harness;
use overlay_ui::design::{self, ButtonSize, ButtonVariant, DsIcon, IconContext, InputSize};
use overlay_ui::panels::options_modal;
use overlay_ui::ui_icons::UiIcons;

// -------------------------------------------------------------------------------------------
// Jetons — chacun dit d'où il vient. Reprises TELLES QUELLES des onglets existants.
// -------------------------------------------------------------------------------------------

/// Fond derrière la fenêtre — le même que `tests/panels.rs` et `alertes-mockups.rs`.
const BACKDROP: Color32 = Color32::from_rgb(0x0B, 0x0D, 0x10);

/// Fond sous le toast — celui que `tests/panels.rs` pose sous le bandeau Suivi.
const GAME_BACKDROP: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);

/// Texte courant — blanc, comme `panels::alerts_tab::TEXT`.
const TEXT: Color32 = Color32::WHITE;

/// Le gris unique du jeu — `panels::alerts_tab::SUBDUED`, `#b8b9ba`.
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);

/// Fond d'une ligne de réglage — `panels::alerts_tab::SETTING_ROW_FILL`.
const SETTING_ROW_FILL: Color32 = Color32::from_rgb(0x26, 0x28, 0x2B);
const SETTING_ROW_RADIUS: u8 = 4;

/// Corps de texte des onglets — `panels::alerts_tab::BODY_FONT_SIZE`.
const BODY_FONT_SIZE: f32 = 15.0;

/// Aération autour d'un titre de section — `panels::alerts_tab::SECTION_GAP`.
const SECTION_GAP: f32 = 18.0;

/// Ligne simple — `panels::alerts_tab::ROW_HEIGHT`, mesurée sur `interface-options-commandes.png`.
const ROW_HEIGHT: f32 = 39.0;

/// Taille de la fenêtre Options — celle de la production.
const WINDOW: Vec2 = Vec2::new(options_modal::WINDOW_SIZE.0, options_modal::WINDOW_SIZE.1);

/// Hauteur d'une recherche en LIGNE — celle d'une ligne de réglage d'Alertes (40) moins rien : la
/// croix y est un bouton icône de 28 px, il reste 6 px de part et d'autre.
const FILTER_ROW_HEIGHT: f32 = 40.0;
const FILTER_ROW_GAP: f32 = 4.0;

/// Gouttière entre deux tuiles — `panels::alerts_tab::TILE_GAP`, le pas de la grille d'Alertes.
const TILE_GAP: f32 = 12.0;

/// Tuiles par rangée — **quatre** : à la largeur utile du panneau (≈ 656 px), c'est ce qui laisse
/// à chaque tuile la place d'un mot de quinze lettres sans ellipse, légende comprise.
const TILES_PER_ROW: usize = 4;

/// Hauteur d'une tuile : une légende sur la bordure, un mot au centre, de l'air.
const TILE_HEIGHT: f32 = 54.0;

/// Bordure d'une tuile — **la bordure des champs de saisie du jeu** (`tokens::INPUT_BORDER`,
/// `#595140`, 2 px) : c'est la signature « cadre » du design system, et la seule bordure chaude
/// qu'il ait. Pas d'arrondi : les cadres du jeu ont les coins droits.
const TILE_BORDER: Color32 = design::tokens::INPUT_BORDER;
const TILE_BORDER_WIDTH: f32 = design::tokens::INPUT_BORDER_WIDTH;

/// Fond d'une tuile — le fond des champs de saisie (`tokens::INPUT_FILL`), pour la même raison.
const TILE_FILL: Color32 = design::tokens::INPUT_FILL;

/// Corps de la légende sur la bordure — petit, c'est une étiquette, pas le contenu.
const LEGEND_FONT_SIZE: f32 = 11.0;
/// Retrait de la légende depuis le bord gauche de la tuile, et respiration de chaque côté du
/// texte où la bordure s'interrompt.
const LEGEND_INSET: f32 = 10.0;
const LEGEND_GAP: f32 = 4.0;

/// Voile d'une tuile survolée — `panels::alerts_tab::TILE_HOVER_SCRIM`.
const TILE_HOVER_SCRIM: Color32 = Color32::from_black_alpha(0x66);
/// Côté de la croix de retrait — `panels::alerts_tab::TILE_BADGE`.
const TILE_BADGE: f32 = 14.0;
/// Retrait de la croix depuis le coin — 8 px comme `panels::alerts_tab::TILE_BADGE_INSET`, mais
/// depuis un coin droit et non depuis l'intérieur d'un contour de 6 px : la croix se pose au même
/// endroit visuel, à 2 px sous la bordure de 2.
const TILE_BADGE_INSET: f32 = 8.0;

// --- Toast : les cotes de `panels::watchlist::toast_card`, reprises telles quelles ---
const ACCENT: Color32 = design::tokens::OVERLAY_ACCENT;
const SURFACE_RAISED: Color32 = design::tokens::OVERLAY_SURFACE_RAISED;
const TEXT_BRIGHT: Color32 = design::tokens::OVERLAY_TEXT_BRIGHT;
const CARD_ROUNDING: f32 = 12.0;
const CARD_BORDER_WIDTH: f32 = 1.0;
const CARD_PAD_V: f32 = 10.0;
const CARD_PAD_LEFT: f32 = 16.0;
const CARD_PAD_RIGHT: f32 = 28.0;
const CARD_TEXT_GAP: f32 = 2.0;
const CARD_TITLE_FONT_SIZE: f32 = 11.0;
const CARD_BODY_FONT_SIZE: f32 = 14.0;
/// Largeur maximale du message dans la carte — un message de chat peut être long, la carte du
/// Suivi n'a jamais eu à retourner à la ligne. Bornée à la largeur de la couche du toast
/// (`TOAST_LAYER_WIDTH`, 320 px) élargie d'un tiers : au-delà, la carte masquerait trop du jeu.
const CARD_TEXT_MAX_WIDTH: f32 = 420.0;

// -------------------------------------------------------------------------------------------
// Canaux et recherches
// -------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Channel {
    Proximite,
    Groupe,
    Guilde,
    Recrutement,
    Commerce,
    Communaute,
}

impl Channel {
    const ALL: [Channel; 6] = [
        Channel::Proximite,
        Channel::Groupe,
        Channel::Guilde,
        Channel::Recrutement,
        Channel::Commerce,
        Channel::Communaute,
    ];

    fn label(self) -> &'static str {
        match self {
            Channel::Proximite => "Proximité",
            Channel::Groupe => "Groupe",
            Channel::Guilde => "Guilde",
            Channel::Recrutement => "Recrutement",
            Channel::Commerce => "Commerce",
            Channel::Communaute => "Communauté",
        }
    }
}

/// Cible d'une recherche : un canal, ou tous (« global » côté web).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    All,
    Channel(Channel),
}

impl Scope {
    fn label(self) -> &'static str {
        match self {
            Scope::All => "Tous les canaux",
            Scope::Channel(c) => c.label(),
        }
    }
}

struct Filter {
    scope: Scope,
    text: &'static str,
}

/// Neuf recherches — assez pour que la différence de densité entre lignes et tuiles se voie, et
/// pour qu'une rangée de tuiles soit incomplète.
const FILTERS: [Filter; 9] = [
    Filter {
        scope: Scope::Channel(Channel::Commerce),
        text: "gelano",
    },
    Filter {
        scope: Scope::All,
        text: "donjon",
    },
    Filter {
        scope: Scope::Channel(Channel::Recrutement),
        text: "pvm",
    },
    Filter {
        scope: Scope::Channel(Channel::Guilde),
        text: "wa wabbit",
    },
    Filter {
        scope: Scope::Channel(Channel::Commerce),
        text: "bois de bouleau",
    },
    Filter {
        scope: Scope::Channel(Channel::Proximite),
        text: "archi",
    },
    Filter {
        scope: Scope::Channel(Channel::Communaute),
        text: "mise à jour",
    },
    Filter {
        scope: Scope::Channel(Channel::Commerce),
        text: "pierre de kamas",
    },
    Filter {
        scope: Scope::All,
        text: "kralamoure",
    },
];

// -------------------------------------------------------------------------------------------
// Le harnais — chrome réel, onglet « Chat » actif, entre Alertes et Personnages.
// -------------------------------------------------------------------------------------------

/// Les onglets tels que ces planches les proposent — `OptionsTab` n'a pas encore de variante Chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Onglet {
    Suivi,
    Alertes,
    Chat,
    Personnages,
    Parametres,
}

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

fn options_harness(
    mut build: impl FnMut(&mut egui::Ui, &UiIcons, &design::PanelZones) + 'static,
) -> Harness<'static> {
    let mut icons: Option<UiIcons> = None;
    let mut tab = Onglet::Chat;
    Harness::builder().with_size(WINDOW).build_ui(move |ui| {
        overlay_ui::style::apply(ui.ctx());
        ui.style_mut().visuals.text_cursor.blink = false;
        let icons = icons.get_or_insert_with(|| UiIcons::load(ui.ctx()));
        egui::Frame::NONE.fill(BACKDROP).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let chrome = design::window("Options")
                .footer("Annuler", "Valider")
                .log_name("maquette")
                .show(ui);
            chrome.tabs(
                ui,
                design::tabs(&mut tab)
                    .entry(Onglet::Suivi, "Suivi")
                    .entry(Onglet::Alertes, "Alertes")
                    .entry(Onglet::Chat, "Chat")
                    .entry(Onglet::Personnages, "Personnages")
                    .enabled(false)
                    .entry(Onglet::Parametres, "Paramètres")
                    .log_name("maquette-onglets"),
            );
            design::panel().show(ui, chrome.content, |ui, panel| {
                build(ui, icons, panel);
            });
        });
    })
}

// -------------------------------------------------------------------------------------------
// L'onglet
// -------------------------------------------------------------------------------------------

/// Comment la liste des recherches est présentée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListStyle {
    /// Une recherche par ligne de réglage, croix à droite.
    Rows,
    /// Une grille de tuiles, canal en légende sur la bordure, croix au survol.
    Tiles,
}

struct ChatTab<'a> {
    filters: &'a [Filter],
    scope: &'a mut Scope,
    input: &'a mut String,
    style: ListStyle,
}

const DESC: &str = "Un son est joué et une carte s'affiche par-dessus le jeu dès qu'un message du \
                    chat contient l'un des mots ci-dessous, sur le canal choisi ou sur tous.";

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

fn chat_tab(ui: &mut egui::Ui, panel: &design::PanelZones, tab: &mut ChatTab<'_>) {
    let width = panel.inner.width();

    ui.add(design::heading("Chat").trailing_gap(SECTION_GAP * 0.5));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP);

    test_sound_row(ui, width);
    ui.add_space(SECTION_GAP);

    ui.add(design::heading("Recherches").trailing_gap(SECTION_GAP * 0.5));
    add_row(ui, tab.scope, tab.input, width);
    ui.add_space(SECTION_GAP * 0.75);

    if tab.filters.is_empty() {
        ui.add(
            design::info_text(
                "Aucune recherche. Choisissez un canal, saisissez un mot, puis cliquez sur Ajouter.",
            )
            .width(width)
            .log_name("chat.vide"),
        );
        return;
    }

    match tab.style {
        ListStyle::Rows => panel.scroll_area(ui, "chat.recherches", |ui, content_width| {
            ui.spacing_mut().item_spacing = Vec2::new(0.0, FILTER_ROW_GAP);
            for filter in tab.filters {
                filter_row(ui, filter, content_width);
            }
        }),
        ListStyle::Tiles => panel.scroll_area(ui, "chat.recherches", |ui, content_width| {
            ui.spacing_mut().item_spacing = Vec2::splat(TILE_GAP);
            // La légende déborde la tuile vers le haut de la moitié de sa hauteur : sans cette
            // marge, celle de la première rangée serait rognée par le clip de la zone.
            ui.add_space(LEGEND_FONT_SIZE * 0.5 + 2.0);
            let tile_width =
                (content_width - TILE_GAP * (TILES_PER_ROW as f32 - 1.0)) / TILES_PER_ROW as f32;
            for chunk in tab.filters.chunks(TILES_PER_ROW) {
                ui.horizontal(|ui| {
                    for filter in chunk {
                        filter_tile(ui, filter, Vec2::new(tile_width, TILE_HEIGHT));
                    }
                });
            }
        }),
    }
}

/// « Tester le son » — la même ligne que dans Alertes (`panels::alerts_tab::test_sound_row`).
fn test_sound_row(ui: &mut egui::Ui, width: f32) {
    let row = ui.allocate_space(Vec2::new(width, ROW_HEIGHT)).1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    cell.horizontal_centered(|ui| {
        ui.label(
            RichText::new("Tester le son de l'alerte")
                .color(TEXT)
                .size(BODY_FONT_SIZE),
        );
        ui.add_space(12.0);
        ui.add(
            design::icon_button(DsIcon::Volume)
                .context(IconContext::Panel)
                .tooltip("Jouer le son d'alerte")
                .log_name("chat.tester"),
        );
    });
}

/// Canal, mot, « Ajouter » — **dans cet ordre** (demande explicite) : on dit d'abord OÙ chercher,
/// puis QUOI.
///
/// **La ligne déborde la largeur utile de 7 px à droite** (retour du 2026-09-13) : la marge entre
/// le bouton et le bord du panneau doit être celle qui sépare le bord gauche du sélecteur, soit
/// `PANEL_PAD_CONTROL_X` (19) — or la largeur utile s'arrête 26 px avant le bord (la réserve de
/// barre de défilement, `components::scroll_area::RESERVE_X`). Le formulaire n'est pas dans la zone défilable,
/// il n'a pas de barre à ménager : il reprend ces 7 px. Les trois contrôles sont séparés du même
/// écart, posé explicitement — l'espacement implicite d'egui est mis à zéro, sinon il s'ajoutait
/// au nôtre et repoussait le bouton hors de la ligne (c'est ce qui le rétrécissait sur la v2).
fn add_row(ui: &mut egui::Ui, scope: &mut Scope, input: &mut String, width: f32) {
    const SCOPE_WIDTH: f32 = 170.0;
    /// Plus étroit que les 110 de la v2 : un libellé de sept lettres n'a pas besoin de plus.
    const ADD_WIDTH: f32 = 92.0;
    const GAP: f32 = 12.0;
    let total =
        width + design::components::scroll_area::RESERVE_X - design::tokens::PANEL_PAD_CONTROL_X;
    let row = ui
        .allocate_space(Vec2::new(total, design::tokens::SELECT_HEIGHT))
        .1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    cell.spacing_mut().item_spacing.x = 0.0;
    cell.horizontal_centered(|ui| {
        let mut select = design::select(scope)
            .option(Scope::All, "Tous les canaux")
            .width(SCOPE_WIDTH)
            .log_name("chat.canal");
        for channel in Channel::ALL {
            select = select.option(Scope::Channel(channel), channel.label());
        }
        select.show(ui);
        ui.add_space(GAP);
        ui.add(
            design::input(input)
                .size(InputSize::Search)
                .clearable(true)
                .placeholder("Mot ou expression à rechercher…")
                .width(total - SCOPE_WIDTH - ADD_WIDTH - GAP * 2.0)
                .log_name("chat.mot"),
        );
        // **Le bouton est ancré au bord droit, dans un enfant en sens inverse**, plutôt qu'ajouté
        // à la suite : sa marge droite doit valoir la marge gauche du sélecteur quelle que soit la
        // largeur des deux autres contrôles, et un ancrage le dit mieux qu'une soustraction. (C'est
        // aussi cette ligne qui a révélé, le 2026-09-13, que `design::input` faisait reculer le
        // curseur de la rangée — corrigé dans le composant depuis, voir `tests/input_row.rs`.)
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        right.add(
            design::button("Ajouter")
                .variant(ButtonVariant::Primary)
                .size(ButtonSize::Compact)
                .width(ADD_WIDTH)
                .log_name("chat.ajouter"),
        );
    });
}

/// Une recherche en LIGNE : « Commerce — gelano », croix à droite. Sur l'aplat de ligne de réglage
/// du jeu, comme « Fermeture automatique » dans Alertes. Aucune couleur de canal.
fn filter_row(ui: &mut egui::Ui, filter: &Filter, width: f32) {
    let row = ui.allocate_space(Vec2::new(width, FILTER_ROW_HEIGHT)).1;
    ui.painter()
        .rect_filled(row, SETTING_ROW_RADIUS, SETTING_ROW_FILL);
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row.shrink2(Vec2::new(12.0, 0.0))));
    cell.horizontal_centered(|ui| {
        ui.label(
            RichText::new(format!("{} —", filter.scope.label()))
                .color(SUBDUED)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(filter.text)
                .color(TEXT)
                .font(design::text::label_strong_font(ui.ctx(), BODY_FONT_SIZE)),
        );
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row.shrink2(Vec2::new(6.0, 0.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        right.add(
            design::icon_button(DsIcon::Close)
                .context(IconContext::Panel)
                .size(28.0)
                .tooltip("Retirer")
                .log_name(format!("chat.retirer.{}", filter.text)),
        );
    });
}

/// Une recherche en TUILE : le mot au centre d'un cadre, le canal SUR la bordure haute (la bordure
/// s'interrompt sous lui, comme la légende d'un cadre de formulaire), croix révélée au survol
/// sous un voile — l'idiome des tuiles d'Alertes (`panels::alerts_tab::alert_item`).
fn filter_tile(ui: &mut egui::Ui, filter: &Filter, size: Vec2) {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let ctx = ui.ctx().clone();
    let painter = ui.painter().clone();

    painter.rect_filled(rect, 0.0, TILE_FILL);

    // La légende, et la bordure qui s'interrompt sous elle.
    let legend = painter.layout_no_wrap(
        filter.scope.label().to_owned(),
        design::text::label_font(&ctx, LEGEND_FONT_SIZE),
        SUBDUED,
    );
    let legend_left = rect.left() + LEGEND_INSET;
    let legend_right = (legend_left + legend.size().x).min(rect.right() - LEGEND_INSET);
    let stroke = egui::Stroke::new(TILE_BORDER_WIDTH, TILE_BORDER);
    let half = TILE_BORDER_WIDTH / 2.0;
    let top = rect.top() + half;
    let bottom = rect.bottom() - half;
    let left = rect.left() + half;
    let right = rect.right() - half;
    painter.line_segment(
        [
            egui::pos2(left, top),
            egui::pos2(legend_left - LEGEND_GAP, top),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(legend_right + LEGEND_GAP, top),
            egui::pos2(right, top),
        ],
        stroke,
    );
    painter.line_segment([egui::pos2(left, top), egui::pos2(left, bottom)], stroke);
    painter.line_segment([egui::pos2(right, top), egui::pos2(right, bottom)], stroke);
    painter.line_segment(
        [egui::pos2(left, bottom), egui::pos2(right, bottom)],
        stroke,
    );
    painter.galley(
        egui::pos2(legend_left, rect.top() - legend.size().y / 2.0 + half),
        legend,
        SUBDUED,
    );

    // Le mot, centré, ellipsé s'il déborde — jamais rogné en silence.
    let mut job = LayoutJob {
        wrap: TextWrapping {
            max_width: size.x - LEGEND_INSET * 2.0,
            max_rows: 1,
            break_anywhere: true,
            overflow_character: Some('…'),
        },
        ..Default::default()
    };
    job.append(
        filter.text,
        0.0,
        TextFormat {
            font_id: design::text::label_strong_font(&ctx, BODY_FONT_SIZE),
            color: TEXT,
            ..Default::default()
        },
    );
    let word = ui.fonts_mut(|f| f.layout_job(job));
    let word_size = word.size();
    painter.galley(
        egui::pos2(
            rect.center().x - word_size.x / 2.0,
            rect.center().y - word_size.y / 2.0 + 2.0,
        ),
        word,
        TEXT,
    );

    // `contains_pointer` et non `hovered` : la croix a sa propre zone, voir `alert_item`.
    if response.contains_pointer() {
        painter.rect_filled(rect.shrink(TILE_BORDER_WIDTH), 0.0, TILE_HOVER_SCRIM);
        let ds = design::DesignSystem::get(&ctx);
        let native = ds.icon_native_size(DsIcon::Close);
        let side = native.x.max(native.y);
        let center = egui::pos2(
            rect.right() - TILE_BADGE_INSET - TILE_BADGE / 2.0,
            rect.top() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
        );
        let icon_rect = Rect::from_center_size(center, native * (TILE_BADGE / side));
        let croix = ui
            .interact(
                Rect::from_center_size(center, Vec2::splat(TILE_BADGE + 4.0)),
                response.id.with("retirer"),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        ds.paint_icon(
            &painter,
            icon_rect,
            DsIcon::Close,
            if croix.hovered() {
                design::tokens::INFO_ALERT
            } else {
                TEXT
            },
        );
        design::tooltip(&croix).text("Retirer");
    }
}

// -------------------------------------------------------------------------------------------
// La carte d'alerte, par-dessus le jeu
// -------------------------------------------------------------------------------------------

/// La carte de chat, au gabarit de `panels::watchlist::toast_card` — aplat `SURFACE_RAISED`,
/// bordure d'accent de 1 px, rayon 12, ombre portée, titre en capitales à l'accent, corps en
/// `TEXT_BRIGHT`. Ce qui change : pas d'icône, pas de confettis, et le corps peut retourner à la
/// ligne.
fn chat_toast_card(ui: &mut egui::Ui, scope: Scope, word: &str, author: &str, message: &str) {
    let ctx = ui.ctx().clone();
    let painter = ui.painter().clone();

    let title = format!("{} · {}", word.to_uppercase(), scope.label().to_uppercase());
    let title_galley = painter.layout_no_wrap(
        title,
        design::text::label_font(&ctx, CARD_TITLE_FONT_SIZE),
        ACCENT,
    );
    let mut body = LayoutJob {
        wrap: TextWrapping {
            max_width: CARD_TEXT_MAX_WIDTH,
            ..Default::default()
        },
        ..Default::default()
    };
    body.append(
        &format!("{author} : "),
        0.0,
        TextFormat {
            font_id: design::text::label_strong_font(&ctx, CARD_BODY_FONT_SIZE),
            color: TEXT_BRIGHT,
            ..Default::default()
        },
    );
    body.append(
        message,
        0.0,
        TextFormat {
            font_id: design::text::label_font(&ctx, CARD_BODY_FONT_SIZE),
            color: TEXT_BRIGHT,
            ..Default::default()
        },
    );
    let body_galley = ui.fonts_mut(|f| f.layout_job(body));

    let text_width = title_galley.size().x.max(body_galley.size().x);
    let text_height = title_galley.size().y + CARD_TEXT_GAP + body_galley.size().y;
    let card_size = Vec2::new(
        CARD_PAD_LEFT + text_width + CARD_PAD_RIGHT,
        text_height + 2.0 * CARD_PAD_V,
    );
    let card = Rect::from_center_size(ui.max_rect().center(), card_size);

    // **Cliquer la carte prépare une réponse en privé** (demande du 2026-09-13) : l'overlay écrit
    // `/w "<auteur>"` dans le chat du jeu, le joueur n'a plus que son message à taper. Zone de clic
    // posée AVANT la peinture, comme `toast_card` ; l'infobulle dit ce que le clic fera.
    let response = ui
        .interact(
            card,
            ui.id().with(("chat-alert", author)),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    design::tooltip(&response).text(format!(
        "Répondre en privé — écrit /w \"{author}\" dans le chat"
    ));

    painter.rect_filled(
        card.translate(egui::vec2(0.0, 6.0)),
        CARD_ROUNDING,
        Color32::from_rgba_unmultiplied(0, 0, 0, 90),
    );
    painter.rect_filled(card, CARD_ROUNDING, SURFACE_RAISED);
    painter.rect_stroke(
        card,
        CARD_ROUNDING,
        egui::Stroke::new(CARD_BORDER_WIDTH, ACCENT),
        egui::StrokeKind::Inside,
    );
    let text_left = card.left() + CARD_PAD_LEFT;
    let text_top = card.center().y - text_height / 2.0;
    let title_height = title_galley.size().y;
    painter.galley(egui::pos2(text_left, text_top), title_galley, ACCENT);
    painter.galley(
        egui::pos2(text_left, text_top + title_height + CARD_TEXT_GAP),
        body_galley,
        TEXT_BRIGHT,
    );
}

// -------------------------------------------------------------------------------------------
// Les planches
// -------------------------------------------------------------------------------------------

fn chat_harness(filters: &'static [Filter], style: ListStyle) -> Harness<'static> {
    let mut scope = Scope::Channel(Channel::Commerce);
    let mut input = String::new();
    options_harness(move |ui, _icons, panel| {
        chat_tab(
            ui,
            panel,
            &mut ChatTab {
                filters,
                scope: &mut scope,
                input: &mut input,
                style,
            },
        );
    })
}

/// **1 — En lignes.** Une recherche par ligne de réglage, « Canal — mot », croix à droite.
fn chat_options_liste() {
    let mut harness = chat_harness(&FILTERS, ListStyle::Rows);
    harness.run();
    write_mockup(&mut harness, "chat_options_liste");
}

/// **2 — En tuiles.** Quatre par rangée, le canal en légende sur la bordure, le mot au centre.
/// La deuxième tuile est survolée : voile et croix, comme au Suivi et dans Alertes.
fn chat_options_tuiles() {
    let mut harness = chat_harness(&FILTERS, ListStyle::Tiles);
    harness.run();
    // Le centre de la deuxième tuile : panneau à x ≈ 39 + une tuile et sa gouttière.
    harness.hover_at(egui::pos2(39.0 + 155.0 + TILE_GAP + 77.0, 442.0));
    harness.run();
    write_mockup(&mut harness, "chat_options_tuiles");
}

/// **3 — Aucune recherche.** Le formulaire seul, et le bloc d'information qui dit quoi faire.
fn chat_options_vide() {
    static NONE: [Filter; 0] = [];
    let mut harness = chat_harness(&NONE, ListStyle::Tiles);
    harness.run();
    write_mockup(&mut harness, "chat_options_vide");
}

/// **4 — La carte d'alerte**, par-dessus le jeu : « GELANO · COMMERCE », puis l'auteur et le message
/// complet.
fn chat_toast() {
    let mut harness = toast_harness();
    harness.run();
    write_mockup(&mut harness, "chat_toast");
}

fn toast_harness() -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(560.0, 160.0))
        .build_ui(|ui| {
            overlay_ui::style::apply(ui.ctx());
            egui::Frame::NONE.fill(GAME_BACKDROP).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                chat_toast_card(
                    ui,
                    Scope::Channel(Channel::Commerce),
                    "gelano",
                    "Huppermage-Bleu",
                    "vends Gelano 900k, prix ferme, mp si intéressé — je suis à Bonta près du zaap",
                );
            });
        })
}

/// **5 — La carte survolée** : la main, et l'infobulle qui annonce la réponse en privé.
fn chat_toast_survol() {
    let mut harness = toast_harness();
    harness.run();
    harness.hover_at(egui::pos2(280.0, 80.0));
    harness.run();
    write_mockup(&mut harness, "chat_toast_survol");
}

fn main() {
    chat_options_liste();
    println!("  chat_options_liste");
    chat_options_tuiles();
    println!("  chat_options_tuiles");
    chat_options_vide();
    println!("  chat_options_vide");
    chat_toast();
    println!("  chat_toast");
    chat_toast_survol();
    println!("  chat_toast_survol");
    let dir = mockup_dir();
    let ecrites = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    let affiche = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("{ecrites} planches écrites dans {}", affiche.display());
}
