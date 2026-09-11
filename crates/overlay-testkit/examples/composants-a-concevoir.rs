//! **Les composants que la page « Alerte » demande et que le design system n'a pas encore.**
//!
//! Chaque planche montre un composant dans **tous ses cas**, au même endroit, pour que la spec se
//! discute sur une image plutôt que sur une description. Ce sont des maquettes : le code ci-dessous
//! peint à la main ce que le composant devra peindre lui-même, et il est destiné à disparaître au
//! fur et à mesure que les composants existent.
//!
//! ```text
//! cargo run -p overlay-testkit --example composants-a-concevoir
//! ```
//!
//! Les plans cotés qui accompagnent ces planches sont publiés avec elles en Artifact (obligation
//! CLAUDE.md) : ce sont eux qui portent les géométries et les espacements, la planche ne montre que
//! le rendu.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use egui::{Color32, Rect, RichText, Stroke, StrokeKind, Vec2};
use egui_kittest::Harness;
use overlay_engine::WakfuRarity;
use overlay_ui::design::{self, DsTexture, IconContext, InputSize};
use overlay_ui::ui_icons::UiIcons;

// -------------------------------------------------------------------------------------------
// Jetons communs aux planches — repris de la maquette de la page Alertes, mêmes sources.
// -------------------------------------------------------------------------------------------

const PAGE_FILL: Color32 = Color32::from_rgb(0x15, 0x18, 0x1C);
const TEXT: Color32 = Color32::WHITE;
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);
const CAPTION: Color32 = Color32::from_rgb(0x8A, 0x8A, 0x8A);
const ACCENT: Color32 = Color32::from_rgb(0x00, 0xD2, 0xFF);
const MUTED_BORDER: Color32 = Color32::from_rgb(0x4D, 0x4D, 0x4D);
const SETTING_ROW_FILL: Color32 = Color32::from_rgb(0x26, 0x28, 0x2B);
const TILE_FILL: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
const CONFIRM_FILL: Color32 = Color32::from_rgb(0x58, 0x59, 0x55);
const CONFIRM_BORDER: Color32 = Color32::from_rgb(0x0E, 0x10, 0x15);

const TILE_WIDTH: f32 = 118.0;
const TILE_HEIGHT: f32 = 88.0;
const TILE_RADIUS: u8 = 4;
const TILE_BORDER_WIDTH: f32 = 2.0;
const TILE_SLOT: f32 = 44.0;
const TILE_BADGE_ROW: f32 = 18.0;
const TILE_BADGE: f32 = 14.0;
const TILE_BADGE_INSET: f32 = 5.0;
const TILE_GAP: f32 = 10.0;
const BODY_FONT_SIZE: f32 = 15.0;
const BORDER_INNER_RATIO: f32 = 52.0 / 512.0;
const ICON_FILL_RATIO: f32 = 0.96;
const SHEET_MARGIN: f32 = 18.0;
const SHEET_WIDTH: f32 = 620.0;

/// Les sept raretés du référentiel, dans l'ordre du jeu.
const RARITIES: &[(WakfuRarity, &str)] = &[
    (WakfuRarity::Common, "Commun"),
    (WakfuRarity::Rare, "Rare"),
    (WakfuRarity::Mythical, "Mythique"),
    (WakfuRarity::Legendary, "Légendaire"),
    (WakfuRarity::Relic, "Relique"),
    (WakfuRarity::Memory, "Souvenir"),
    (WakfuRarity::Epic, "Épique"),
];

// -------------------------------------------------------------------------------------------
// Harnais et briques
// -------------------------------------------------------------------------------------------

fn mockup_dir() -> std::path::PathBuf {
    static PURGE: std::sync::Once = std::sync::Once::new();
    let dir = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(target) => std::path::PathBuf::from(target),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"),
    }
    .join("composants");
    PURGE.call_once(|| {
        let _ = std::fs::remove_dir_all(&dir);
    });
    std::fs::create_dir_all(&dir).expect("création de target/composants");
    dir
}

/// Hauteur de rendu volontairement large : la planche est ensuite **recadrée** sur ce que le
/// contenu a réellement occupé (voir `write`). Une hauteur écrite à la main par planche se
/// désynchronise au premier cas ajouté — et le symptôme, une dernière bande coupée, ne se voit
/// qu'à l'œil sur l'image produite.
const SHEET_HEIGHT: f32 = 1200.0;

/// Bas du contenu de la planche en cours, relevé par `planche` à la fin du build.
type ContentBottom = std::rc::Rc<std::cell::Cell<f32>>;

fn write(harness: &mut Harness<'static>, bottom: &ContentBottom, name: &str) {
    let image = harness
        .render()
        .expect("rendu offscreen — voir doc de module");
    // `Harness::render` peint à 1 pixel par point (vérifié : une planche demandée à 620x300 rend
    // une image de 620x300) — pas de conversion de densité à faire ici.
    let height = (bottom.get() + SHEET_MARGIN)
        .ceil()
        .min(image.height() as f32) as u32;
    image::imageops::crop_imm(&image, 0, 0, image.width(), height)
        .to_image()
        .save(mockup_dir().join(format!("{name}.png")))
        .expect("écriture de la planche");
}

fn planche(
    width: f32,
    mut build: impl FnMut(&mut egui::Ui, &UiIcons) + 'static,
) -> (Harness<'static>, ContentBottom) {
    let mut icons: Option<UiIcons> = None;
    let bottom: ContentBottom = ContentBottom::default();
    let reported = bottom.clone();
    let harness = Harness::builder()
        .with_size(Vec2::new(width, SHEET_HEIGHT))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            let icons = icons.get_or_insert_with(|| UiIcons::load(ui.ctx()));
            egui::Frame::NONE
                .fill(PAGE_FILL)
                .inner_margin(SHEET_MARGIN)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    build(ui, icons);
                    reported.set(ui.min_rect().bottom());
                });
        });
    (harness, bottom)
}

/// Légende d'un cas — le nom de l'état sous l'échantillon.
fn legende(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(CAPTION).size(12.0));
}

/// Titre d'une bande de cas.
fn bande(ui: &mut egui::Ui, text: &str) {
    ui.add_space(10.0);
    ui.add(design::heading(text).trailing_gap(8.0));
}

/// L'emplacement d'objet du jeu — bordure de rareté peinte AVANT l'icône.
fn item_slot(ui: &egui::Ui, icons: &UiIcons, rect: Rect, rarity: WakfuRarity) {
    egui::Image::new(icons.item_border(rarity)).paint_at(ui, rect);
    let inner = rect.width() * (1.0 - 2.0 * BORDER_INNER_RATIO) * ICON_FILL_RATIO;
    egui::Image::new(icons.unknown_entity_texture()).paint_at(
        ui,
        Rect::from_center_size(rect.center(), Vec2::splat(inner)),
    );
}

/// La tuile d'alerte, dans l'état demandé.
fn alert_item(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    name: &str,
    rarity: WakfuRarity,
    sound_on: bool,
    removable: bool,
) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(TILE_WIDTH, TILE_HEIGHT), egui::Sense::hover());
    let state_color = if sound_on { ACCENT } else { MUTED_BORDER };
    ui.painter().rect_filled(rect, TILE_RADIUS, TILE_FILL);
    ui.painter().rect_stroke(
        rect,
        TILE_RADIUS,
        Stroke::new(TILE_BORDER_WIDTH, state_color),
        StrokeKind::Inside,
    );

    let slot = Rect::from_center_size(
        egui::pos2(
            rect.center().x,
            rect.top() + TILE_BADGE_ROW + TILE_SLOT / 2.0,
        ),
        Vec2::splat(TILE_SLOT),
    );
    item_slot(ui, icons, slot, rarity);

    ui.painter().text(
        egui::pos2(rect.center().x, slot.bottom() + 5.0),
        egui::Align2::CENTER_TOP,
        name,
        design::text::label_font(ui.ctx(), 13.0),
        TEXT,
    );

    let ds = design::DesignSystem::get(ui.ctx());
    let icon = if sound_on {
        DsTexture::IconVolume
    } else {
        DsTexture::IconVolumeMute
    };
    let native = ds.native_size(icon);
    ds.paint(
        ui.painter(),
        Rect::from_center_size(
            egui::pos2(
                rect.left() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
                rect.top() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
            ),
            design::components::icon_button::glyph_fit(native, TILE_BADGE),
        ),
        icon,
        SUBDUED,
    );

    if removable {
        let native = ds.native_size(DsTexture::IconClose);
        ds.paint(
            ui.painter(),
            Rect::from_center_size(
                egui::pos2(
                    rect.right() - TILE_BADGE_INSET - TILE_BADGE / 2.0,
                    rect.top() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
                ),
                design::components::icon_button::glyph_fit(native, TILE_BADGE - 2.0),
            ),
            DsTexture::IconClose,
            SUBDUED,
        );
    }
}

// -------------------------------------------------------------------------------------------
// 1 — `design::body` : un paragraphe de texte courant
// -------------------------------------------------------------------------------------------

/// **Le manque le plus élémentaire.** Le design system sait peindre un titre (`heading`), une
/// remarque à pastille (`info_text`) et un libellé de contrôle, mais pas une **description** — le
/// paragraphe qui suit un titre de section et explique ce que la section fait.
///
/// Détourner `info_text` pour ça donne à une description le poids d'une remarque : c'est ce que
/// faisait la page Alertes, et c'est ce que l'utilisateur a fait retirer.
fn planche_body() {
    let (mut harness, bottom) = planche(SHEET_WIDTH, |ui, _icons| {
        bande(ui, "design::body");

        legende(ui, "Une ligne");
        ui.add(
            egui::Label::new(
                RichText::new("Objets qui déclenchent une alerte sonore.")
                    .color(TEXT)
                    .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
            )
            .wrap_mode(egui::TextWrapMode::Wrap),
        );

        ui.add_space(14.0);
        legende(ui, "Plusieurs lignes, repliées sur la largeur donnée");
        ui.add(
            egui::Label::new(
                RichText::new(
                    "Objets qui déclenchent une alerte sonore et un message à l'écran lorsqu'ils \
                     sont ramassés.",
                )
                .color(TEXT)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
            )
            .wrap_mode(egui::TextWrapMode::Wrap),
        );

        ui.add_space(14.0);
        legende(ui, "Atténué — pour un texte secondaire");
        ui.add(
            egui::Label::new(
                RichText::new("Aucun objet suivi pour le moment.")
                    .color(SUBDUED)
                    .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
            )
            .wrap_mode(egui::TextWrapMode::Wrap),
        );

        ui.add_space(14.0);
        legende(ui, "À comparer : `design::info_text`, qui dit « remarque »");
        ui.add(design::info_text("Objets qui déclenchent une alerte sonore.").width(560.0));
    });
    harness.run();
    write(&mut harness, &bottom, "composant_body");
}

// -------------------------------------------------------------------------------------------
// 2 — `design::item_slot` : l'emplacement d'objet à bordure de rareté
// -------------------------------------------------------------------------------------------

/// **Déjà dupliqué deux fois** : `panels::watchlist::entry_tile` et la maquette de la page
/// Alertes. Un portage en ferait une troisième copie.
///
/// Les sept bordures sont des assets du jeu (`Border-<RARETÉ>.webp`), chargés par `UiIcons`.
/// **Elles font 512 × 512 pour des emplacements peints à 44** : les réduire à la taille utile
/// rendrait environ 7 Mio sur le budget de 300, et c'est à faire avant d'en faire un composant.
fn planche_item_slot() {
    let (mut harness, bottom) = planche(SHEET_WIDTH, |ui, icons| {
        bande(ui, "design::item_slot");

        legende(ui, "Les sept raretés, à 44 px");
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for (rarity, _) in RARITIES {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), egui::Sense::hover());
                item_slot(ui, icons, rect, *rarity);
            }
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for (_, nom) in RARITIES {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(44.0, 14.0), egui::Sense::hover());
                ui.painter().text(
                    rect.center_top(),
                    egui::Align2::CENTER_TOP,
                    nom,
                    design::text::label_font(ui.ctx(), 10.0),
                    CAPTION,
                );
            }
        });

        ui.add_space(16.0);
        legende(
            ui,
            "Toute taille — 28, 44, 58 px (la tuile du Suivi est à 58)",
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for side in [28.0_f32, 44.0, 58.0] {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), egui::Sense::hover());
                item_slot(ui, icons, rect, WakfuRarity::Legendary);
            }
        });

        ui.add_space(14.0);
        legende(
            ui,
            "L'icône est le repli générique : les vraies viennent du CDN, hors de portée du harnais.",
        );
    });
    harness.run();
    write(&mut harness, &bottom, "composant_item_slot");
}

// -------------------------------------------------------------------------------------------
// 3 — `design::alert_item` : la tuile d'un objet suivi
// -------------------------------------------------------------------------------------------

/// **Le composant central de la page.** Quatre états visibles, deux axes indépendants : le son
/// (actif / coupé) porte la couleur de bordure et le pictogramme ; le fait d'être un objet par
/// défaut retire la croix.
fn planche_alert_item() {
    let (mut harness, bottom) = planche(SHEET_WIDTH, |ui, icons| {
        bande(ui, "design::alert_item");

        legende(ui, "Son actif / son coupé — bordure d'accent ou grise");
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = TILE_GAP;
            alert_item(
                ui,
                icons,
                "Pierre d'aventure",
                WakfuRarity::Mythical,
                true,
                false,
            );
            alert_item(
                ui,
                icons,
                "Pierre d'aventure",
                WakfuRarity::Mythical,
                false,
                false,
            );
        });

        ui.add_space(16.0);
        legende(
            ui,
            "Objet ajouté par le joueur — une croix de retrait apparaît",
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = TILE_GAP;
            alert_item(ui, icons, "Combinaison La…", WakfuRarity::Epic, true, true);
            alert_item(ui, icons, "Combinaison La…", WakfuRarity::Epic, false, true);
        });

        ui.add_space(16.0);
        legende(
            ui,
            "Nom trop long — élidé, jamais tronqué au point d'être ambigu",
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = TILE_GAP;
            alert_item(
                ui,
                icons,
                "Plan \"Epée de B…",
                WakfuRarity::Legendary,
                true,
                false,
            );
            alert_item(
                ui,
                icons,
                "Influence III",
                WakfuRarity::Legendary,
                true,
                false,
            );
        });
    });
    harness.run();
    write(&mut harness, &bottom, "composant_alert_item");
}

// -------------------------------------------------------------------------------------------
// 4 — `design::setting_row` : une ligne de réglage sur aplat clair
// -------------------------------------------------------------------------------------------

/// L'idiome des lignes d'aptitude du jeu (`interface-personnage-aptitudes.png`, colonne de droite),
/// qui sert à détacher un réglage de son fond de section.
fn planche_setting_row() {
    let mut auto_on = true;
    let mut auto_off = false;
    let mut seconds_on = String::from("3.5");
    let mut seconds_off = String::from("3.5");

    let (mut harness, bottom) = planche(SHEET_WIDTH, move |ui, _icons| {
        bande(ui, "design::setting_row");
        let width = 560.0;

        legende(ui, "Contrôle actif");
        row(ui, width, |ui| {
            ui.add(design::checkbox(&mut auto_on, "Fermeture automatique"));
            ui.add_space(12.0);
            ui.add(
                design::input(&mut seconds_on)
                    .size(InputSize::Standard)
                    .width(52.0),
            );
            ui.label(RichText::new("sec.").color(SUBDUED).size(BODY_FONT_SIZE));
        });

        ui.add_space(12.0);
        legende(ui, "Contrôle conditionné — la case décochée grise le champ");
        row(ui, width, |ui| {
            ui.add(design::checkbox(&mut auto_off, "Fermeture automatique"));
            ui.add_space(12.0);
            ui.add(
                design::input(&mut seconds_off)
                    .size(InputSize::Standard)
                    .width(52.0)
                    .enabled(false),
            );
            ui.label(RichText::new("sec.").color(SUBDUED).size(BODY_FONT_SIZE));
        });

        ui.add_space(12.0);
        legende(ui, "Libellé seul, sans contrôle");
        row(ui, width, |ui| {
            ui.label(
                RichText::new("Le message se ferme au clic")
                    .color(TEXT)
                    .size(BODY_FONT_SIZE),
            );
        });
    });
    harness.run();
    write(&mut harness, &bottom, "composant_setting_row");
}

/// L'aplat d'une ligne de réglage, et son contenu centré verticalement.
fn row(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    let rect = ui.allocate_space(Vec2::new(width, 40.0)).1;
    ui.painter().rect_filled(rect, 4, SETTING_ROW_FILL);
    let mut cell =
        ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(Vec2::new(12.0, 0.0))));
    cell.horizontal_centered(add);
}

// -------------------------------------------------------------------------------------------
// 5 — `design::input_number` : un champ numérique borné
// -------------------------------------------------------------------------------------------

/// **Les deux boutons existent déjà** : le socle de pas est au manifeste
/// ([`DsTexture::ButtonStepper`], générifié depuis `button-moins.png`) et les glyphes aussi
/// ([`DsTexture::IconPlus`], [`DsTexture::IconMinus`]) — `design::icon_button` en contexte
/// `Stepper` les compose, c'est ce que peint cette planche. Ce qui manque est **le champ**
/// (`input-number.png`, 104 × 28, toujours hors manifeste) et **l'assemblage borné** des trois.
///
/// Sans lui, une durée se saisit dans un champ texte libre, et la borne doit se poser à la
/// validation — ce qui a déjà produit un bug : borner à chaque frappe rendait `0.75` et `1,5`
/// impossibles à écrire.
fn planche_input_number() {
    let mut v1 = String::from("3.5");
    let mut v2 = String::from("0.5");
    let mut v3 = String::from("3.5");

    let (mut harness, bottom) = planche(SHEET_WIDTH, move |ui, _icons| {
        bande(ui, "design::input_number");

        legende(ui, "Valeur courante — les deux boutons agissent");
        stepper_row(ui, &mut v1, true, true);

        ui.add_space(14.0);
        legende(ui, "Borne basse atteinte — « − » est désactivé");
        stepper_row(ui, &mut v2, false, true);

        ui.add_space(14.0);
        legende(ui, "Champ entier désactivé");
        stepper_row(ui, &mut v3, false, false);

        ui.add_space(14.0);
        legende(
            ui,
            "Boutons : vrais assets du jeu (socle ButtonStepper + glyphes). Champ : repli sur \
             design::input — input-number.png (104 × 28) n'est pas encore au manifeste.",
        );
    });
    harness.run();
    write(&mut harness, &bottom, "composant_input_number");
}

fn stepper_row(ui: &mut egui::Ui, value: &mut String, minus_enabled: bool, enabled: bool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.add(
            design::icon_button(DsTexture::IconMinus)
                .context(IconContext::Stepper)
                // Taille NATIVE du socle. Le défaut du bouton icône est 36 (celui des cinq socles
                // de panneau) : laissé tel quel, il agrandit le socle de pas de 32 à 36.
                .size(design::tokens::STEPPER_SIZE)
                .enabled(minus_enabled && enabled),
        );
        ui.add(
            design::input(value)
                .size(InputSize::Standard)
                .width(64.0)
                .enabled(enabled),
        );
        ui.add(
            design::icon_button(DsTexture::IconPlus)
                .context(IconContext::Stepper)
                .size(design::tokens::STEPPER_SIZE)
                .enabled(enabled),
        );
        ui.label(RichText::new("sec.").color(SUBDUED).size(BODY_FONT_SIZE));
    });
}

// -------------------------------------------------------------------------------------------
// 6 — `design::confirm_dialog` : la boîte de confirmation du jeu
// -------------------------------------------------------------------------------------------

/// Relevée sur `interface-confirm-box.png` : boîte **centrée et autonome**, fond clair `#585955`,
/// crête à médaillon, et deux réponses dont **l'acceptation est le bouton OR** — le jeu ne met
/// jamais de rouge ici, c'est une convention web.
fn planche_confirm_dialog() {
    let (mut harness, bottom) = planche(SHEET_WIDTH, |ui, _icons| {
        bande(ui, "design::confirm_dialog");
        legende(ui, "Question courte");
        confirm(ui, "Retirer cet objet de vos alertes ?");
        ui.add_space(16.0);
        legende(
            ui,
            "Question longue — le corps grandit, les boutons ne bougent pas",
        );
        confirm(
            ui,
            "Retirer « Combinaison Lardante » de vos alertes ? Cette action est définitive.",
        );
    });
    harness.run();
    write(&mut harness, &bottom, "composant_confirm_dialog");
}

/// Géométrie de la boîte — chaque cote est reprise par le plan coté publié avec la planche.
const CONFIRM_WIDTH: f32 = 420.0;
const CONFIRM_CREST_RADIUS: f32 = 18.0;
const CONFIRM_TEXT_TOP: f32 = 30.0;
const CONFIRM_TEXT_TO_BUTTONS: f32 = 12.0;
const CONFIRM_BUTTON_HEIGHT: f32 = 36.0;
const CONFIRM_BUTTON_WIDTH: f32 = 150.0;
const CONFIRM_BUTTON_GAP: f32 = 14.0;
const CONFIRM_PAD_X: f32 = 24.0;
const CONFIRM_PAD_BOTTOM: f32 = 10.0;

fn confirm(ui: &mut egui::Ui, question: &str) {
    let font = design::text::label_font(ui.ctx(), 14.0);
    // La hauteur DÉRIVE du texte replié : c'est ce que la planche annonce (« le corps grandit,
    // les boutons ne bougent pas »), et une hauteur figée l'aurait démenti au premier libellé
    // long — le cas exact qui arrive avec un nom d'objet dans la question.
    let galley = ui.painter().layout(
        question.to_owned(),
        font.clone(),
        TEXT,
        CONFIRM_WIDTH - 2.0 * CONFIRM_PAD_X,
    );
    let height = CONFIRM_TEXT_TOP
        + galley.size().y
        + CONFIRM_TEXT_TO_BUTTONS
        + CONFIRM_BUTTON_HEIGHT
        + CONFIRM_PAD_BOTTOM;

    // Le médaillon déborde du haut de la boîte : l'espace réservé le compte, sinon il repeint la
    // légende de la bande au-dessus.
    let outer = ui
        .allocate_space(Vec2::new(CONFIRM_WIDTH, height + CONFIRM_CREST_RADIUS))
        .1;
    let rect = Rect::from_min_size(
        egui::pos2(outer.left(), outer.top() + CONFIRM_CREST_RADIUS),
        Vec2::new(CONFIRM_WIDTH, height),
    );
    ui.painter().rect_filled(rect, 4, CONFIRM_FILL);
    ui.painter().rect_stroke(
        rect,
        4,
        Stroke::new(2.0, CONFIRM_BORDER),
        StrokeKind::Inside,
    );

    let crest = egui::pos2(rect.center().x, rect.top());
    ui.painter().circle_filled(
        crest,
        CONFIRM_CREST_RADIUS,
        Color32::from_rgb(0xF4, 0xD8, 0x9E),
    );
    ui.painter().circle_stroke(
        crest,
        CONFIRM_CREST_RADIUS,
        Stroke::new(2.0, CONFIRM_BORDER),
    );
    let ds = design::DesignSystem::get(ui.ctx());
    let native = ds.native_size(DsTexture::IconHelp);
    ds.paint(
        ui.painter(),
        Rect::from_center_size(
            crest,
            design::components::icon_button::glyph_fit(native, 13.0),
        ),
        DsTexture::IconHelp,
        design::tokens::BUTTON_TEXT_ON_GOLD,
    );

    ui.painter().galley(
        egui::pos2(rect.left() + CONFIRM_PAD_X, rect.top() + CONFIRM_TEXT_TOP),
        galley,
        TEXT,
    );

    let mut buttons = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(
        egui::pos2(
            rect.center().x - CONFIRM_BUTTON_WIDTH - CONFIRM_BUTTON_GAP / 2.0,
            rect.bottom() - CONFIRM_PAD_BOTTOM - CONFIRM_BUTTON_HEIGHT,
        ),
        Vec2::new(
            2.0 * CONFIRM_BUTTON_WIDTH + CONFIRM_BUTTON_GAP,
            CONFIRM_BUTTON_HEIGHT,
        ),
    )));
    buttons.horizontal(|ui| {
        ui.add(
            design::button("Non")
                .variant(design::ButtonVariant::Secondary)
                .size(design::ButtonSize::Compact)
                .width(CONFIRM_BUTTON_WIDTH),
        );
        ui.add_space(CONFIRM_BUTTON_GAP);
        ui.add(
            design::button("Oui")
                .variant(design::ButtonVariant::Primary)
                .size(design::ButtonSize::Compact)
                .width(CONFIRM_BUTTON_WIDTH),
        );
    });
}

// -------------------------------------------------------------------------------------------
// 7 — `design::autocomplete` : un champ et son panneau de suggestions
// -------------------------------------------------------------------------------------------

/// **Le vrai mécanisme d'ajout d'un objet** : une alerte a besoin d'un identifiant résolu par le
/// catalogue, pas d'un nom tapé librement.
///
/// Le vocabulaire est déjà celui de `design::select` déplié (mêmes jetons de fond, de bord, de
/// surbrillance et de cadence) : c'est une **extension de `select`** plutôt qu'un composant neuf,
/// avec une icône par entrée et une entrée désactivable.
fn planche_autocomplete() {
    let mut vide = String::new();
    let mut saisi = String::from("pierre");

    let (mut harness, bottom) = planche(SHEET_WIDTH, move |ui, icons| {
        bande(ui, "design::autocomplete");
        let width = 560.0;

        legende(ui, "Replié, vide");
        ui.add(
            design::input(&mut vide)
                .leading_icon(DsTexture::IconSearch)
                .placeholder("Ajouter un objet à surveiller…")
                .width(width),
        );

        ui.add_space(16.0);
        legende(
            ui,
            "Déplié — une entrée déjà suivie est grisée et non sélectionnable",
        );
        let field = ui.add(
            design::input(&mut saisi)
                .leading_icon(DsTexture::IconSearch)
                .width(width),
        );
        suggestions(ui, icons, field.rect);
    });
    harness.run();
    write(&mut harness, &bottom, "composant_autocomplete");
}

fn suggestions(ui: &mut egui::Ui, icons: &UiIcons, field: Rect) {
    const ENTREES: &[(&str, WakfuRarity, bool)] = &[
        ("Pierre d'aventure", WakfuRarity::Mythical, true),
        ("Pierre de dolomite", WakfuRarity::Common, false),
        ("Pierre de lune", WakfuRarity::Rare, false),
        ("Pierre ponce", WakfuRarity::Common, false),
    ];
    let row_h = design::tokens::SELECT_ROW_HEIGHT;
    let list = Rect::from_min_size(
        egui::pos2(field.left(), field.bottom() + 2.0),
        Vec2::new(field.width(), 4.0 + ENTREES.len() as f32 * row_h),
    );
    ui.allocate_space(Vec2::new(field.width(), list.height() + 4.0));

    ui.painter()
        .rect_filled(list, 2, design::tokens::SELECT_LIST_FILL);
    ui.painter().rect_stroke(
        list,
        2,
        Stroke::new(2.0, design::tokens::SELECT_LIST_BORDER),
        StrokeKind::Inside,
    );
    ui.painter().hline(
        list.x_range(),
        list.top() + 2.5,
        Stroke::new(1.0, design::tokens::SELECT_LIST_TOP_LINE),
    );

    for (i, (name, rarity, deja)) in ENTREES.iter().enumerate() {
        let row = Rect::from_min_size(
            egui::pos2(list.left() + 2.0, list.top() + 2.0 + i as f32 * row_h),
            Vec2::new(list.width() - 4.0, row_h),
        );
        if i == 1 {
            ui.painter()
                .rect_filled(row, 0, design::tokens::SELECT_ROW_HIGHLIGHT);
        }
        item_slot(
            ui,
            icons,
            Rect::from_center_size(
                egui::pos2(row.left() + 16.0, row.center().y),
                Vec2::splat(22.0),
            ),
            *rarity,
        );
        ui.painter().text(
            egui::pos2(row.left() + 34.0, row.center().y),
            egui::Align2::LEFT_CENTER,
            *name,
            design::text::label_font(ui.ctx(), BODY_FONT_SIZE),
            if *deja {
                design::tokens::TEXT_DISABLED
            } else {
                design::tokens::SELECT_TEXT
            },
        );
        if *deja {
            ui.painter().text(
                egui::pos2(row.right() - 10.0, row.center().y),
                egui::Align2::RIGHT_CENTER,
                "déjà suivi",
                design::text::label_font(ui.ctx(), 13.0),
                design::tokens::TEXT_DISABLED,
            );
        }
    }
}

fn main() {
    planche_body();
    println!("  composant_body");
    planche_item_slot();
    println!("  composant_item_slot");
    planche_alert_item();
    println!("  composant_alert_item");
    planche_setting_row();
    println!("  composant_setting_row");
    planche_input_number();
    println!("  composant_input_number");
    planche_confirm_dialog();
    println!("  composant_confirm_dialog");
    planche_autocomplete();
    println!("  composant_autocomplete");

    let dir = mockup_dir();
    let n = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    let affiche = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("{n} planches écrites dans {}", affiche.display());
}
