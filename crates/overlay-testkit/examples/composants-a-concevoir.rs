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

#[path = "shared/wakassets_fixtures.rs"]
mod wakassets_fixtures;

use egui::{Color32, Rect, RichText, Stroke, StrokeKind, Vec2};
use egui_kittest::Harness;
use overlay_engine::WakfuRarity;
use overlay_ui::design::{self, DsIcon, IconContext, InputSize, SlotFrame};
use overlay_ui::rarity_bridge::to_slot_rarity;
use overlay_ui::ui_icons::UiIcons;

use wakassets_fixtures::{CategoryFilter, CategoryIcons, RarityGems, GEM_BOX};

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
/// Hauteur d'une ligne de réglage.
const SETTING_ROW_HEIGHT: f32 = 40.0;
/// Rayon d'angle de l'aplat.
const SETTING_ROW_RADIUS: u8 = 4;
/// Marge intérieure gauche et droite.
const SETTING_ROW_PAD_X: f32 = 12.0;
/// Écart entre deux lignes consécutives.
const SETTING_ROW_GAP: f32 = 12.0;
/// Écart entre deux contrôles d'une même ligne.
const SETTING_ROW_CONTROL_GAP: f32 = 12.0;
const TILE_FILL: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
/// Fond d'une infobulle egui, figurée sur la planche (voir `tooltip_apercu`).
const TOOLTIP_FILL: Color32 = Color32::from_rgb(0x10, 0x12, 0x16);
/// Écart entre le bas de la tuile et l'infobulle du nom.
const TOOLTIP_OFFSET: f32 = 6.0;
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
// Le rayon de l'aplat et son fond sombre sont dans `design::tokens` depuis que le composant
// existe (`ITEM_SLOT_RADIUS_RATIO`, `ITEM_SLOT_BACKDROP`) — plus de copie ici.
const ICON_FILL_RATIO: f32 = 0.96;
const SHEET_MARGIN: f32 = 18.0;
const SHEET_WIDTH: f32 = 620.0;
/// Hauteur d'une rangée de suggestion — `SELECT_ROW_HEIGHT`, la cadence du select simple du jeu.
/// Une image nue de 22 px y tient sans forcer ; c'est la bordure de rareté, retirée depuis, qui
/// avait fait passer cette valeur à 32.
const SUGGESTION_ROW_HEIGHT: f32 = design::tokens::SELECT_ROW_HEIGHT;
/// Marge gauche d'une rangée, et de la bande de catégories.
const SUGGESTION_PAD_X: f32 = 6.0;
/// Écart entre la gemme, l'image et le nom.
const SUGGESTION_GAP: f32 = 6.0;
/// Côté de l'image d'objet d'une rangée.
const SUGGESTION_SLOT: f32 = 22.0;
/// Hauteur de la bande de filtres — bouton 26 + 2 × 6 de marge, comme
/// `.wakfu-autocomplete-categories` (padding 6) côté web.
const CATEGORY_BAR_HEIGHT: f32 = 38.0;
/// Côté d'un bouton de catégorie — `.wakfu-autocomplete-category-btn`, 26 × 26.
const CATEGORY_BUTTON: f32 = 26.0;
/// Marge intérieure du bouton : l'icône occupe 20 des 26 (`padding: 3px` côté web).
const CATEGORY_ICON_PAD: f32 = 3.0;
/// Écart entre deux boutons.
const CATEGORY_GAP: f32 = 4.0;
/// Marge gauche de la bande.
const CATEGORY_PAD: f32 = 6.0;
/// Rayon d'angle d'un bouton.
const CATEGORY_RADIUS: u8 = 4;
/// Hauteur du message « Aucun résultat dans cette catégorie », sous la bande de filtres.
const VIDE_HEIGHT: f32 = 34.0;

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

/// L'emplacement d'objet du jeu — **`design::item_slot` depuis le 2026-09-11**.
///
/// Cette fonction ne peint plus rien : elle place le composant dans un rectangle absolu
/// (`Ui::put`, les planches posent leur géométrie elles-mêmes) et traduit la rareté du moteur.
/// L'aplat, la bordure, l'ordre de peinture et les ratios sont partis dans le composant avec
/// leurs jetons — c'était tout l'objet de cette planche.
///
/// `rarity: None` n'est pas une variante du composant : sans bordure, ce n'est plus un
/// emplacement, c'est une image. Le cas est peint ici tel quel, deux lignes, pour que la planche
/// montre encore ce que le relevé demandait — c'est d'ailleurs ce que fait `design::autocomplete`
/// pour ses rangées de suggestion.
///
/// Le harnais n'atteint pas le CDN : toutes les planches montrent donc le repli générique. C'est
/// aussi ce que l'overlay affiche tant qu'un téléchargement n'a pas abouti.
fn item_slot(ui: &mut egui::Ui, icons: &UiIcons, rect: Rect, rarity: Option<WakfuRarity>) {
    match rarity {
        Some(rarity) => {
            ui.put(
                rect,
                design::item_slot()
                    .size(rect.width())
                    .frame(SlotFrame::Rarity(to_slot_rarity(rarity)))
                    .icon(icons.unknown_entity_texture().id()),
            );
        }
        None => {
            let taille = rect.width() * (1.0 - 2.0 * BORDER_INNER_RATIO) * ICON_FILL_RATIO;
            egui::Image::new(icons.unknown_entity_texture()).paint_at(
                ui,
                Rect::from_center_size(rect.center(), Vec2::splat(taille)),
            );
        }
    }
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
    item_slot(ui, icons, slot, Some(rarity));

    ui.painter().text(
        egui::pos2(rect.center().x, slot.bottom() + 5.0),
        egui::Align2::CENTER_TOP,
        name,
        design::text::label_font(ui.ctx(), 13.0),
        TEXT,
    );

    let ds = design::DesignSystem::get(ui.ctx());
    let icon = if sound_on {
        DsIcon::Volume
    } else {
        DsIcon::VolumeMute
    };
    let native = ds.icon_native_size(icon);
    ds.paint_icon(
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
        let native = ds.icon_native_size(DsIcon::Close);
        ds.paint_icon(
            ui.painter(),
            Rect::from_center_size(
                egui::pos2(
                    rect.right() - TILE_BADGE_INSET - TILE_BADGE / 2.0,
                    rect.top() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
                ),
                design::components::icon_button::glyph_fit(native, TILE_BADGE - 2.0),
            ),
            DsIcon::Close,
            SUBDUED,
        );
    }
}

// -------------------------------------------------------------------------------------------
// 1 — `design::text` : un paragraphe de texte courant
// -------------------------------------------------------------------------------------------

/// **Le manque le plus élémentaire.** Le design system sait peindre un titre (`heading`), une
/// remarque à pastille (`info_text`) et un libellé de contrôle, mais pas une **description** — le
/// paragraphe qui suit un titre de section et explique ce que la section fait.
///
/// Détourner `info_text` pour ça donne à une description le poids d'une remarque : c'est ce que
/// faisait la page Alertes, et c'est ce que l'utilisateur a fait retirer.
///
/// **Nommé `design::text` sur demande de l'utilisateur** (2026-09-11) : le nom dit ce que le
/// composant rend. Il coexistera avec le module `design::text` déjà là (`text::label_font`) —
/// Rust range les modules et les fonctions dans deux espaces de noms distincts, donc
/// `design::text("…")` et `design::text::label_font(…)` se résolvent tous les deux.
fn planche_text() {
    let (mut harness, bottom) = planche(SHEET_WIDTH, |ui, _icons| {
        bande(ui, "design::text");

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
    write(&mut harness, &bottom, "composant_text");
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
                item_slot(ui, icons, rect, Some(*rarity));
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
                item_slot(ui, icons, rect, Some(WakfuRarity::Legendary));
            }
        });

        ui.add_space(16.0);
        legende(
            ui,
            "Sans bordure — le cas du panneau de suggestions, où la gemme porte la rareté",
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for side in [22.0_f32, 44.0] {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), egui::Sense::hover());
                item_slot(ui, icons, rect, None);
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

        ui.add_space(16.0);
        legende(
            ui,
            "Nom élidé + survol du LIBELLÉ — le nom entier, et seulement dans ce cas",
        );
        let centre_tuile = ui.cursor().min.x + TILE_WIDTH / 2.0;
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
        });
        tooltip_apercu(ui, centre_tuile, "Plan \"Epée de Boisaille\"");
    });
    harness.run();
    write(&mut harness, &bottom, "composant_alert_item");
}

/// L'infobulle du nom, **figurée** : un rendu hors écran n'a pas de souris, donc pas de survol.
///
/// Ce qu'elle montre est la règle demandée par l'utilisateur (2026-09-11) : le nom entier
/// s'affiche au survol du **libellé** — pas de la tuile — et **uniquement si le nom est élidé**.
/// Un nom qui tient en entier n'a rien à révéler. Même règle que le web, qui la pose avec
/// `[tooltipOnlyIfTruncated]="true"` sur le nom (`wakfu-autocomplete.component.html`).
fn tooltip_apercu(ui: &mut egui::Ui, centre_x: f32, texte: &str) {
    let font = design::text::label_font(ui.ctx(), 13.0);
    let galley = ui.painter().layout_no_wrap(texte.to_owned(), font, TEXT);
    let taille = galley.size() + Vec2::new(16.0, 10.0);
    // Espace ALLOUÉ, pas seulement peint : sans ça la planche se recadre au-dessus de l'infobulle
    // (`write` mesure `ui.min_rect()`), et le cas disparaît de l'image.
    let (bande, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), taille.y + TOOLTIP_OFFSET),
        egui::Sense::hover(),
    );
    let boite = Rect::from_min_size(
        egui::pos2(centre_x - taille.x / 2.0, bande.top() + TOOLTIP_OFFSET),
        taille,
    );
    ui.painter().rect_filled(boite, 3, TOOLTIP_FILL);
    ui.painter()
        .rect_stroke(boite, 3, Stroke::new(1.0, MUTED_BORDER), StrokeKind::Inside);
    ui.painter()
        .galley(boite.min + Vec2::new(8.0, 5.0), galley, TEXT);
}

// -------------------------------------------------------------------------------------------
// 4 — `design::setting_row` : l'aplat clair d'une ligne, et RIEN d'autre
// -------------------------------------------------------------------------------------------

/// L'idiome des lignes d'aptitude du jeu (`interface-personnage-aptitudes.png`, colonne de
/// droite) : un aplat plus clair que la section, à coins arrondis, qui détache une ligne de son
/// fond.
///
/// **Générique, sur retour de l'utilisateur (2026-09-11).** La version précédente peignait l'aplat
/// ET son contenu — case à cocher, libellé, champ, unité — c'est-à-dire un composant qui connaît
/// un réglage particulier. Ce n'en est pas un : c'est une SURFACE. Elle ne sait rien de ce qu'on
/// pose dedans, et le réglage de fermeture des alertes est devenu un composant à part
/// ([`planche_alert_management`]) qui l'utilise.
fn planche_setting_row() {
    let (mut harness, bottom) = planche(SHEET_WIDTH, move |ui, _icons| {
        bande(ui, "design::setting_row");
        let width = 560.0;

        legende(ui, "Un libellé");
        row(ui, width, |ui| {
            ui.label(
                RichText::new("Le message se ferme au clic")
                    .color(TEXT)
                    .size(BODY_FONT_SIZE),
            );
        });

        ui.add_space(SETTING_ROW_GAP);
        legende(ui, "Un libellé et une valeur — poussée à droite");
        row(ui, width, |ui| {
            ui.label(
                RichText::new("Objets suivis")
                    .color(TEXT)
                    .size(BODY_FONT_SIZE),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("11").color(SUBDUED).size(BODY_FONT_SIZE));
            });
        });

        ui.add_space(SETTING_ROW_GAP);
        legende(ui, "Un contrôle — la surface ne sait pas lequel");
        row(ui, width, |ui| {
            ui.add(design::button("Réinitialiser les alertes").size(design::ButtonSize::Compact));
        });

        ui.add_space(SETTING_ROW_GAP);
        legende(ui, "Deux lignes qui se suivent");
        row(ui, width, |ui| {
            ui.label(RichText::new("Première").color(TEXT).size(BODY_FONT_SIZE));
        });
        ui.add_space(SETTING_ROW_GAP);
        row(ui, width, |ui| {
            ui.label(RichText::new("Seconde").color(TEXT).size(BODY_FONT_SIZE));
        });
    });
    harness.run();
    write(&mut harness, &bottom, "composant_setting_row");
}

/// L'aplat d'une ligne de réglage, et son contenu centré verticalement.
fn row(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    let rect = ui.allocate_space(Vec2::new(width, SETTING_ROW_HEIGHT)).1;
    ui.painter()
        .rect_filled(rect, SETTING_ROW_RADIUS, SETTING_ROW_FILL);
    let mut cell = ui.new_child(
        egui::UiBuilder::new().max_rect(rect.shrink2(Vec2::new(SETTING_ROW_PAD_X, 0.0))),
    );
    cell.horizontal_centered(add);
}

// -------------------------------------------------------------------------------------------
// 5 — `design::alert_management` : le réglage de fermeture, posé sur une `setting_row`
// -------------------------------------------------------------------------------------------

/// **Le composant qui utilise la surface**, et le seul des huit qui connaisse le domaine des
/// alertes : une case à cocher, son libellé, un champ de durée et son unité.
///
/// Né de la scission demandée par l'utilisateur (2026-09-11) : `setting_row` peignait l'aplat ET
/// ce contenu-là, ce qui en faisait un composant qui ne servait qu'une fois. Séparés, la surface
/// se réutilise partout et ce réglage-ci reste une brique à part entière.
///
/// La règle qui le porte : **décocher retire le délai**, donc le champ n'a plus rien à régler —
/// il est grisé ET inerte, jamais seulement grisé.
fn planche_alert_management() {
    let mut auto_on = true;
    let mut auto_off = false;
    let mut seconds_on = String::from("3.5");
    let mut seconds_off = String::from("3.5");

    let (mut harness, bottom) = planche(SHEET_WIDTH, move |ui, _icons| {
        bande(ui, "design::alert_management");
        let width = 560.0;

        legende(ui, "Fermeture automatique — le délai se règle");
        row(ui, width, |ui| {
            alert_management(ui, &mut auto_on, &mut seconds_on);
        });

        ui.add_space(SETTING_ROW_GAP);
        legende(ui, "Fermeture au clic — le délai n'a plus de sens");
        row(ui, width, |ui| {
            alert_management(ui, &mut auto_off, &mut seconds_off);
        });
    });
    harness.run();
    write(&mut harness, &bottom, "composant_alert_management");
}

/// Le contenu du réglage — posé dans une `row`, dont il ignore tout sauf qu'elle lui donne une
/// bande à sa hauteur.
fn alert_management(ui: &mut egui::Ui, auto: &mut bool, seconds: &mut String) {
    ui.add(design::checkbox(auto, "Fermeture automatique"));
    ui.add_space(SETTING_ROW_CONTROL_GAP);
    ui.add(
        design::input(seconds)
            .size(InputSize::Standard)
            .width(52.0)
            .enabled(*auto),
    );
    ui.label(RichText::new("sec.").color(SUBDUED).size(BODY_FONT_SIZE));
}

// -------------------------------------------------------------------------------------------
// 6 — `design::input_number` : un champ numérique borné
// -------------------------------------------------------------------------------------------

/// **Les deux boutons existent déjà** : le socle de pas est au manifeste
/// ([`DsTexture::ButtonStepper`], générifié depuis `button-moins.png`) et les glyphes aussi
/// ([`DsIcon::Plus`], [`DsIcon::Minus`]) — `design::icon_button` en contexte
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
            design::icon_button(DsIcon::Minus)
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
            design::icon_button(DsIcon::Plus)
                .context(IconContext::Stepper)
                .size(design::tokens::STEPPER_SIZE)
                .enabled(enabled),
        );
        ui.label(RichText::new("sec.").color(SUBDUED).size(BODY_FONT_SIZE));
    });
}

// -------------------------------------------------------------------------------------------
// 7 — `design::confirm_dialog` : la boîte de confirmation du jeu
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
    let native = ds.icon_native_size(DsIcon::Help);
    ds.paint_icon(
        ui.painter(),
        Rect::from_center_size(
            crest,
            design::components::icon_button::glyph_fit(native, 13.0),
        ),
        DsIcon::Help,
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
// 8 — `design::autocomplete` : un champ et son panneau de suggestions
// -------------------------------------------------------------------------------------------

/// **Le vrai mécanisme d'ajout d'un objet** : une alerte a besoin d'un identifiant résolu par le
/// catalogue, pas d'un nom tapé librement.
///
/// Le vocabulaire visuel est celui de `design::select` déplié (mêmes jetons de fond, de bord, de
/// surbrillance et de cadence) : c'est une **extension de `select`** plutôt qu'un composant neuf,
/// avec une bande de filtres, une icône par entrée et une entrée désactivable.
///
/// # Les règles de comportement, relevées sur `shared/wakfu-autocomplete`
///
/// Elles ne se voient sur aucune capture, et ce sont elles qui feront le composant :
///
/// 1. **Rien avant trois caractères** (`MIN_QUERY_LENGTH = 3`, `wakfu-search.service.ts`), comptés
///    sur la requête NORMALISÉE — pas sur la frappe brute. En dessous, la recherche rend une liste
///    vide et le panneau ne s'ouvre pas du tout.
/// 2. **Une entrée déjà suivie n'est pas sélectionnable** — grisée, sans surbrillance au survol.
///    L'égalité se fait **par identifiant** quand il est connu, par nom seulement à défaut : deux
///    objets homonymes de raretés différentes ne se désactivent pas l'un l'autre.
/// 3. **Un filtre actif RESTREINT la liste** à sa seule catégorie (`results` = `rawResults` filtré).
///    Sélectionner « Équipements » ne laisse que des équipements, et rien d'autre.
/// 4. **La bande de filtres se calcule sur la liste NON filtrée.** C'est la subtilité du composant :
///    elle reste entière même quand le filtre actif ne laisse rien passer — sinon le bouton qui
///    permettrait de le relâcher disparaîtrait avec les résultats, et l'utilisateur serait coincé
///    devant une liste vide. Dans ce cas le panneau affiche « Aucun résultat dans cette catégorie »
///    à la place des rangées, la bande toujours en place.
/// 5. **Le domaine est un paramètre du composant, pas une propriété de la page** — `item`,
///    `enemy` ou `both`. Il décide de ce que la recherche interroge ET de la présence du filtre
///    « Monstres ». La page Alertes est en `item` ; le formulaire d'ajout au Suivi voudra `both`,
///    monstres compris. Le composant doit donc porter ce drapeau dès sa première version, sous
///    peine d'être à réécrire pour son deuxième appelant.
///
/// # Le clavier
///
/// Quatre touches, et une subtilité par touche :
///
/// - **↓ / ↑** parcourent la liste AFFICHÉE (donc filtrée), **en sautant les entrées
///   désactivées** et **en bouclant** (modulo) : depuis la dernière, ↓ revient à la première. La
///   rangée atteinte est ramenée dans la zone visible (`scrollIntoView`, `block: 'nearest'`), ce
///   qui compte dès que la liste défile — au-delà de cinq rangées côté web. Liste vide : la touche
///   ne fait rien.
/// - **Entrée** valide l'entrée active, **et seulement si elle n'est pas désactivée** — la
///   navigation les saute déjà, c'est une seconde barrière.
/// - **Échap** ferme le panneau **sans vider le champ** : la saisie reste, le panneau se rouvre à
///   la frappe suivante.
/// - **Frappe** : la requête change, le panneau s'ouvre, et **l'entrée active repart à la
///   première** — sinon la touche Entrée validerait un objet que la nouvelle recherche n'affiche
///   peut-être plus. Changer de filtre la réinitialise aussi, pour la même raison.
///
/// # Ce qui suit une sélection
///
/// `select` fait cinq choses, dont deux qui ne se devinent pas :
///
/// 1. refuse une entrée désactivée (troisième barrière, après le survol inerte et le clavier) ;
/// 2. émet l'objet choisi vers l'appelant ;
/// 3. **vide le champ** — prêt pour un nouvel ajout, sans avoir à effacer la saisie précédente ;
/// 4. ferme le panneau et remet l'entrée active à la première ;
/// 5. **remet le filtre par catégorie à « Tout »** : la recherche suivante repart sans filtre.
///    C'est le détail le plus facile à oublier — le garder actif ferait disparaître des résultats
///    d'une recherche sans rapport, sans que rien n'explique pourquoi.
///
/// # Hors périmètre, et pourquoi
///
/// Le web pose un **bouton « suivre les objets de la recette »** au bout des rangées dont l'objet
/// est craftable (`openRecipe`, indépendant de la sélection : il n'ajoute jamais l'objet lui-même).
/// La page Alertes le désactive explicitement (`[showRecipeButton]="false"`) — suivre une recette
/// n'a de sens que pour le SUIVI, pas pour une alerte sonore. Le composant devra donc l'accueillir
/// un jour, mais **la spécification en est reportée au formulaire d'ajout au Suivi** (décision de
/// l'utilisateur, 2026-09-11) : le concevoir maintenant, sans son cas d'usage sous les yeux,
/// reviendrait à deviner.
fn planche_autocomplete() {
    let mut vide = String::new();
    let mut saisi = String::from("pierre");

    let mut gems: Option<RarityGems> = None;
    let mut cats: Option<CategoryIcons> = None;
    let (mut harness, bottom) = planche(SHEET_WIDTH, move |ui, icons| {
        let gems = gems.get_or_insert_with(|| RarityGems::load(ui.ctx()));
        let cats = cats.get_or_insert_with(|| CategoryIcons::load(ui.ctx()));
        bande(ui, "design::autocomplete");
        let width = 560.0;

        legende(ui, "Replié, vide");
        ui.add(
            design::input(&mut vide)
                .leading_icon(DsIcon::Search)
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
                .leading_icon(DsIcon::Search)
                .width(width),
        );
        suggestions(ui, icons, gems, cats, field.rect);

        ui.add_space(16.0);
        legende(
            ui,
            "Filtre sans résultat — la bande RESTE, sinon on ne pourrait plus la relâcher",
        );
        panneau_vide(ui, cats, width);

        ui.add_space(16.0);
        legende(ui, "Un filtre actif — le recliquer le relâche");
        bande_filtres(
            ui,
            cats,
            width,
            &[
                CategoryFilter::All,
                CategoryFilter::Equipment,
                CategoryFilter::Resources,
                CategoryFilter::Craft,
            ],
            CategoryFilter::Resources,
        );

        ui.add_space(16.0);
        legende(
            ui,
            "Les dix filtres possibles — seuls ceux présents dans les résultats sont affichés",
        );
        let tous: Vec<CategoryFilter> = std::iter::once(CategoryFilter::All)
            .chain(CategoryFilter::ITEM_CATEGORIES)
            .chain(std::iter::once(CategoryFilter::Enemy))
            .collect();
        bande_filtres(ui, cats, width, &tous, CategoryFilter::All);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = CATEGORY_GAP;
            ui.add_space(CATEGORY_PAD);
            for filtre in &tous {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(CATEGORY_BUTTON, 14.0), egui::Sense::hover());
                ui.painter().text(
                    rect.center_top(),
                    egui::Align2::CENTER_TOP,
                    // Élidé : « Sublimations » fait trois fois la largeur d'un bouton. Par
                    // CARACTÈRES, jamais par octets : `label()[..4]` couperait « Récoltes » au
                    // milieu du « é » et paniquerait.
                    filtre.label().chars().take(4).collect::<String>(),
                    design::text::label_font(ui.ctx(), 9.0),
                    CAPTION,
                );
            }
        });
    });
    harness.run();
    write(&mut harness, &bottom, "composant_autocomplete");
}

/// Le panneau de suggestions — **relevé sur le composant web** (`shared/wakfu-autocomplete`,
/// captures fournies par l'utilisateur le 2026-09-11).
///
/// Une rangée porte trois choses, dans cet ordre : la **gemme de rareté**, l'**image de l'objet**,
/// son **nom**. Une entrée déjà suivie est grisée, **non cliquable, et ne réagit pas au survol** —
/// les trois ensemble : une ligne grisée qui s'allume quand même au passage de la souris promet un
/// clic qui n'arrivera pas.
fn suggestions(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    gems: &RarityGems,
    cats: &CategoryIcons,
    field: Rect,
) {
    // (nom, rareté, catégorie, déjà suivi, survolée)
    const ENTREES: &[(&str, WakfuRarity, CategoryFilter, bool, bool)] = &[
        (
            "Pierre d'aventure",
            WakfuRarity::Mythical,
            CategoryFilter::Resources,
            true,
            true,
        ),
        (
            "Pierre de dolomite",
            WakfuRarity::Common,
            CategoryFilter::Resources,
            false,
            true,
        ),
        (
            "Pierre de lune",
            WakfuRarity::Rare,
            CategoryFilter::Equipment,
            false,
            false,
        ),
        (
            "Pierre ponce",
            WakfuRarity::Common,
            CategoryFilter::Craft,
            false,
            false,
        ),
    ];
    let row_h = SUGGESTION_ROW_HEIGHT;
    let list = Rect::from_min_size(
        egui::pos2(field.left(), field.bottom() + 2.0),
        Vec2::new(
            field.width(),
            4.0 + CATEGORY_BAR_HEIGHT + ENTREES.len() as f32 * row_h,
        ),
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

    // **Uniquement les catégories présentes dans les résultats**, « Tout » en tête — la règle du
    // web, pas une bande figée. Ici : Équipements, Ressources et Craft, dans l'ordre des
    // catégories, jamais dans celui d'apparition des résultats.
    let mut filtres = vec![CategoryFilter::All];
    filtres.extend(
        CategoryFilter::ITEM_CATEGORIES
            .iter()
            .copied()
            .filter(|c| ENTREES.iter().any(|(_, _, categorie, _, _)| categorie == c)),
    );
    category_bar(
        ui,
        cats,
        Rect::from_min_size(
            egui::pos2(list.left() + 2.0, list.top() + 2.0),
            Vec2::new(list.width() - 4.0, CATEGORY_BAR_HEIGHT),
        ),
        &filtres,
        CategoryFilter::All,
    );

    for (i, (name, rarity, _categorie, deja, survolee)) in ENTREES.iter().enumerate() {
        let row = Rect::from_min_size(
            egui::pos2(
                list.left() + 2.0,
                list.top() + 2.0 + CATEGORY_BAR_HEIGHT + i as f32 * row_h,
            ),
            Vec2::new(list.width() - 4.0, row_h),
        );
        // **Pas de surbrillance sur une entrée déjà suivie**, même survolée : c'est le point du
        // cas, et c'est exactement ce que la rangée 1 montre à côté de la rangée 2.
        if *survolee && !*deja {
            ui.painter()
                .rect_filled(row, 0, design::tokens::SELECT_ROW_HIGHLIGHT);
        }

        rarity_gem(
            ui,
            gems,
            Rect::from_center_size(
                egui::pos2(
                    row.left() + SUGGESTION_PAD_X + GEM_BOX / 2.0,
                    row.center().y,
                ),
                Vec2::splat(GEM_BOX),
            ),
            *rarity,
        );
        // **Image NUE** : la gemme porte déjà la rareté, un cadre coloré ferait doublon —
        // exactement ce que fait le web (`app-item-icon`, sans bordure).
        let slot_x = row.left() + SUGGESTION_PAD_X + GEM_BOX + SUGGESTION_GAP;
        item_slot(
            ui,
            icons,
            Rect::from_center_size(
                egui::pos2(slot_x + SUGGESTION_SLOT / 2.0, row.center().y),
                Vec2::splat(SUGGESTION_SLOT),
            ),
            None,
        );
        ui.painter().text(
            egui::pos2(slot_x + SUGGESTION_SLOT + SUGGESTION_GAP, row.center().y),
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

/// La gemme de rareté d'une rangée — **la vraie image du jeu**.
///
/// Elle vient des fixtures du harnais (voir `shared/rarity_gems.rs`), qui tiennent lieu de ce que
/// `RemoteIconStore` télécharge au runtime : le fichier est choisi par
/// `IconRef::for_rarity(rarité).gfx_id`, donc par la même correspondance que celle qui construira
/// l'URL en vrai. `IconKind::Rarity` est arrivé côté `overlay-engine` depuis que cette planche
/// n'en réservait que la place.
fn rarity_gem(ui: &egui::Ui, gems: &RarityGems, rect: Rect, rarity: WakfuRarity) {
    gems.paint(ui, rect, rarity);
}

/// Une bande de filtres posée seule sur la planche, hors panneau — pour montrer ses états.
fn bande_filtres(
    ui: &mut egui::Ui,
    cats: &CategoryIcons,
    width: f32,
    filtres: &[CategoryFilter],
    actif: CategoryFilter,
) {
    let rect = ui.allocate_space(Vec2::new(width, CATEGORY_BAR_HEIGHT)).1;
    ui.painter()
        .rect_filled(rect, 2, design::tokens::SELECT_LIST_FILL);
    category_bar(&*ui, cats, rect, filtres, actif);
}

/// Le panneau quand le filtre actif ne laisse passer aucun résultat.
///
/// **Le cas qui justifie que la bande se calcule sur la liste non filtrée** : si elle se
/// calculait sur la liste affichée, elle disparaîtrait ici avec les rangées — et le bouton qui
/// permettrait de relâcher le filtre partirait avec, laissant l'utilisateur devant un panneau vide
/// sans issue. Le web le dit dans la doc de `rawResults` ; c'est la règle la moins visible du
/// composant, et celle qu'on casse en la réimplémentant de mémoire.
fn panneau_vide(ui: &mut egui::Ui, cats: &CategoryIcons, width: f32) {
    let hauteur = CATEGORY_BAR_HEIGHT + VIDE_HEIGHT;
    let rect = ui.allocate_space(Vec2::new(width, hauteur)).1;
    ui.painter()
        .rect_filled(rect, 2, design::tokens::SELECT_LIST_FILL);
    ui.painter().rect_stroke(
        rect,
        2,
        Stroke::new(2.0, design::tokens::SELECT_LIST_BORDER),
        StrokeKind::Inside,
    );
    category_bar(
        &*ui,
        cats,
        Rect::from_min_size(
            rect.min + Vec2::splat(2.0),
            Vec2::new(rect.width() - 4.0, CATEGORY_BAR_HEIGHT),
        ),
        &[
            CategoryFilter::All,
            CategoryFilter::Equipment,
            CategoryFilter::Resources,
            CategoryFilter::Craft,
        ],
        CategoryFilter::Equipment,
    );
    ui.painter().text(
        egui::pos2(
            rect.center().x,
            rect.top() + CATEGORY_BAR_HEIGHT + VIDE_HEIGHT / 2.0,
        ),
        egui::Align2::CENTER_CENTER,
        // `wakfuAutocomplete.noResultInCategory` — le libellé exact du web.
        "Aucun résultat dans cette catégorie",
        design::text::label_font(ui.ctx(), 13.0),
        design::tokens::TEXT_DISABLED,
    );
}

/// La bande de filtres par catégorie, en tête du panneau — **relevée sur le web**
/// (`wakfu-autocomplete.component.ts`/`.css`).
///
/// Trois règles qui ne se devinent pas en regardant une capture :
///
/// 1. **Seules les catégories PRÉSENTES dans les résultats ont un bouton** (`filterButtons`) —
///    afficher un filtre qui viderait la liste n'aurait aucun sens. La bande disparaît entièrement
///    s'il n'y en a aucune.
/// 2. **« Tout » n'est pas une catégorie** : c'est le bouton de remise à zéro, toujours en tête, et
///    il est actif tant qu'aucun filtre ne l'est.
/// 3. **Pas de filtre « Monstres » ici** : il n'existe qu'en domaine `both`, et la page Alertes est
///    en domaine `item` (`profile-page.component.html`). Une première version de cette planche en
///    dessinait dix, monstres compris — faux sur ce point comme sur le premier.
///
/// Un clic sur le filtre déjà actif le relâche (retour à « Tout ») : `toggleCategoryFilter`.
///
/// **La bande se calcule sur la liste NON filtrée** (`rawResults`), jamais sur la liste affichée :
/// elle doit rester entière quand le filtre actif ne laisse rien passer, sinon le bouton qui
/// permettrait de le relâcher disparaîtrait avec les résultats.
fn category_bar(
    ui: &egui::Ui,
    icons: &CategoryIcons,
    rect: Rect,
    filtres: &[CategoryFilter],
    actif: CategoryFilter,
) {
    for (i, filtre) in filtres.iter().enumerate() {
        let cell = Rect::from_min_size(
            egui::pos2(
                rect.left() + CATEGORY_PAD + i as f32 * (CATEGORY_BUTTON + CATEGORY_GAP),
                rect.center().y - CATEGORY_BUTTON / 2.0,
            ),
            Vec2::splat(CATEGORY_BUTTON),
        );
        let est_actif = *filtre == actif;
        if est_actif {
            // Aplat + cadre, les deux jetons de la famille `select` — pas l'accent cyan de la
            // tuile d'alerte, qui jurerait sur un panneau doré.
            ui.painter()
                .rect_filled(cell, CATEGORY_RADIUS, design::tokens::SELECT_ROW_HIGHLIGHT);
            ui.painter().rect_stroke(
                cell,
                CATEGORY_RADIUS,
                Stroke::new(1.0, design::tokens::STEPPER_ICON_TINT),
                StrokeKind::Inside,
            );
        }
        icons.paint(ui, cell.shrink(CATEGORY_ICON_PAD), *filtre, est_actif);
    }
    // Le filet qui sépare la bande des suggestions — `border-bottom` côté web, et le jeton de
    // liseré que la liste dépliée du jeu utilise déjà.
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, design::tokens::SELECT_LIST_TOP_LINE),
    );
}

fn main() {
    planche_text();
    println!("  composant_text");
    planche_item_slot();
    println!("  composant_item_slot");
    planche_alert_item();
    println!("  composant_alert_item");
    planche_setting_row();
    println!("  composant_setting_row");
    planche_alert_management();
    println!("  composant_alert_management");
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
