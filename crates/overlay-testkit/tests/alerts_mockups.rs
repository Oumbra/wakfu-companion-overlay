//! **Maquettes de la page « Alerte »** — trois portages concurrents de
//! `https://claude-dev.wakfu-companion.com/fr/profile/alerts` (dépôt web `Oumbra/wakfu-companion`,
//! `features/profile-page`, onglet `alerts`) dans le design system du jeu, rendus offscreen et
//! publiés en Artifact (obligation CLAUDE.md).
//!
//! **Ce fichier est un support de DÉCISION, pas une couverture de non-régression.** Il existe pour
//! qu'un choix de mise en page se fasse sur une image, pas sur une description — et il est destiné
//! à être **retiré** une fois la direction retenue implémentée dans `panels::`. Ses trois captures
//! restent néanmoins des snapshots comparés, pour la même raison que la galerie : une maquette qui
//! bouge en silence quand un jeton change ne vaut rien comme référence.
//!
//! ## La fonctionnalité à porter
//!
//! Sept éléments, relevés sur la capture fournie par l'utilisateur et sur
//! `profile-page.component.html` (bloc `activeTab() === 'alerts'`) :
//!
//! 1. un titre « Alerte » et un bouton d'aide `?` ;
//! 2. une phrase d'explication ;
//! 3. une ligne « Tester le son de l'alerte » et son bouton ;
//! 4. « Fermeture de l'alerte : `N` sec. » avec un choix Auto / Manuelle ;
//! 5. un sous-titre « Objets suivis » ;
//! 6. un champ d'ajout d'objet (autocomplétion sur le catalogue) ;
//! 7. la liste des objets suivis : icône, nom, bascule du son, retrait — la bordure de chaque
//!    tuile portant la RARETÉ de l'objet.
//!
//! ## Ce que les trois maquettes ne partagent pas
//!
//! | | A — Fenêtre Options | B — Liste HDV | C — Panneau in-game |
//! | --- | --- | --- | --- |
//! | Contenant | modale du jeu (`ModalBody`) | modale du jeu | aucun, posé sur le jeu |
//! | Objets suivis | grille de tuiles 58 px | lignes zébrées | grille dense 58 px |
//! | Bascule du son | badge sur la tuile | case à cocher en colonne | badge sur la tuile |
//! | Auto / Manuelle | case à cocher (idiome jeu) | case à cocher | case à cocher |
//! | Se lit à | 720 px | 720 px | ~330 px |
//!
//! ## Emprunts assumés au jeu, et emprunts impossibles
//!
//! - **La tuile d'objet à bordure de rareté existe déjà** dans l'overlay (`panels::watchlist::
//!   entry_tile`) et vient du jeu (`Border-<RARETÉ>.webp`, l'emplacement d'inventaire) : les
//!   maquettes A et C la reprennent telle quelle plutôt que d'inventer une tuile d'alerte.
//! - **Les icônes d'objets ne sont pas disponibles hors ligne** : elles viennent du CDN
//!   (`remote_icons`), inaccessible depuis le harnais. Toutes les tuiles portent donc le repli
//!   `UiIcons::unknown_entity_texture()` — celui que l'overlay affiche réellement tant qu'une icône
//!   n'a pas fini de télécharger. Ce qui est jugé ici est la tuile, pas le dessin de l'objet.
//! - **Le jeu n'a pas d'icône « son actif / son coupé »** — aucun des 34 glyphes de
//!   `assets/design-system/icons/` n'est un haut-parleur, et `interface-options-son.png` règle son
//!   volume avec des cases à cocher et des curseurs, sans jamais dessiner de haut-parleur. Les
//!   maquettes utilisent donc la CASE À COCHER du design system pour l'état sonore, et le badge de
//!   tuile est un carré coché/décoché. Un haut-parleur reste à extraire du client si l'on veut le
//!   pictogramme du web (voir le skill `design-asset`).
//! - **Le jeu n'a pas de « switch » à deux positions** comme celui du web (Auto / Manuelle) : son
//!   idiome pour un choix binaire est la case à cocher (`interface-options-son.png` : « Couper la
//!   musique », « Lecture continue »). Les trois maquettes suivent le jeu, pas le web.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use egui::{Color32, Rect, RichText, Stroke, StrokeKind, Vec2};
use egui_kittest::Harness;
use overlay_engine::WakfuRarity;
use overlay_ui::design::{self, ButtonSize, ButtonVariant, DsTexture, IconContext, InputSize};
use overlay_ui::ui_icons::UiIcons;

// -------------------------------------------------------------------------------------------
// Jetons propres aux maquettes — tout ce qui n'est PAS déjà dans `design::tokens`.
//
// Chacun dit d'où il vient : mesure sur une capture du jeu, valeur reprise d'un panneau existant
// de l'overlay, ou choix de mise en page assumé comme tel. La règle de rédaction du crate
// (« une valeur devinée qui n'annonce pas qu'elle est devinée est le pire cas ») vaut ici aussi,
// même pour un fichier de maquette.
// -------------------------------------------------------------------------------------------

/// Fond derrière la modale — le damier sombre sur lequel `tests/panels.rs` juge déjà la fenêtre
/// Options, pour qu'un bord translucide se voie.
const BACKDROP: Color32 = Color32::from_rgb(0x0B, 0x0D, 0x10);

/// Fond d'un panneau in-game (maquette C) — `PANEL_BG` de `panels::watchlist`, la valeur que les
/// panneaux Combat et Suivi utilisent déjà par-dessus le jeu.
const PANEL_BG: Color32 = Color32::from_rgba_premultiplied(0x14, 0x16, 0x1A, 0xE0);

/// Or du jeu — `TAB_LABEL_IDLE` de `design::tokens`, repris ici pour les titres de section serif.
const GOLD: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9E);

/// Texte courant d'un panneau — le clair neutre de `panels::watchlist`.
const TEXT: Color32 = Color32::from_rgb(0xE4, 0xE6, 0xE8);

/// Texte secondaire (unités, compteurs, en-têtes de colonne) — le gris du jeu, mesuré sur les
/// en-têtes « Nom / Niv. / Prix » de `interface-hdv-achat.png`.
const TEXT_MUTED: Color32 = Color32::from_rgb(0x9A, 0xA0, 0xA6);

/// Côté d'une tuile d'objet — `TILE_SIZE` de `panels::watchlist`, inchangé : c'est la même tuile.
const TILE: f32 = 58.0;

/// Gouttière entre deux tuiles — `TILE_GAP` de `panels::watchlist`.
const TILE_GAP: f32 = 6.0;

/// Fenêtre intérieure de `Border-<RARETÉ>.webp` — `ITEM_BORDER_INNER_MARGIN_RATIO` de
/// `panels::watchlist`, mesurée une fois pour toutes sur les sept fichiers (52/512).
const BORDER_INNER_RATIO: f32 = 52.0 / 512.0;

/// Part de la fenêtre intérieure occupée par l'icône — `ITEM_ICON_FILL_RATIO` de
/// `panels::watchlist`, réglé sur retour utilisateur (« les objets doivent être plus gros »).
const ICON_FILL_RATIO: f32 = 0.96;

/// Côté du badge posé dans un coin de tuile (bascule du son, retrait).
///
/// **Choix de mise en page, pas une mesure** : le jeu incruste bien des marqueurs dans les coins
/// d'un emplacement d'objet (`interface-personnage-equiement.png` : cadenas en haut-gauche, coche
/// verte en bas-gauche, pastille bleue en haut-droite), mais aucun n'est mesurable au pixel sur la
/// capture disponible. 16 px sur une tuile de 58 reprend la proportion qu'on y lit.
const TILE_BADGE: f32 = 16.0;

/// Largeur d'une cellule de grille quand la tuile porte son libellé (maquette A).
///
/// **Choix de mise en page, tranché par la mesure du pire cas** : `Plan "Epée de Brâkmar"` fait
/// 132 px au corps de 12, et aucune largeur raisonnable ne le montre entier — mais 104 px en
/// affichent assez (`Plan "Epée de B…`) pour distinguer deux plans voisins, là où les 58 px de la
/// tuile nue s'arrêtent à `Plan "Ep…`, identique pour les quatre. La grille du web tient le même
/// raisonnement avec son `minmax(140px, 1fr)`.
const TILE_LABEL_CELL: f32 = 104.0;

/// Hauteur d'une ligne de la liste (maquette B) — mesurée sur `interface-hdv-achat.png` : les
/// bandes zébrées de la liste d'offres y font 40 px (y 355..395, 395..435…).
const ROW_HEIGHT: f32 = 40.0;

/// Fond d'une ligne PAIRE de la liste (maquette B) — mesuré sur la même capture, bande claire.
const ROW_EVEN: Color32 = Color32::from_rgb(0x1E, 0x20, 0x24);

/// Fond d'une ligne IMPAIRE — bande sombre de la même capture.
const ROW_ODD: Color32 = Color32::from_rgb(0x17, 0x19, 0x1C);

/// Largeur intérieure d'une modale de 720 px, marges du décor déduites.
///
/// La texture `modal-body.png` porte un cadre de hachures dont les marges sont déclarées dans
/// `design::assets::MODAL_BODY_SLICE` (130 à gauche, 205 à droite) — mais ce sont des marges de
/// 9-slice, pas des marges de CONTENU : la fenêtre Options du jeu écrit son contenu bien plus près
/// du bord (x=30 sur `interface-options-son.png`, pour une fenêtre de 720). C'est cette mesure-là
/// qui vaut ici.
const MODAL_PAD_X: f32 = 30.0;

/// Les onze objets de la capture fournie par l'utilisateur, dans son ordre.
///
/// **Les raretés sont une reconstitution, pas une donnée** : la page web colore la bordure d'une
/// tuile par `rarityClass()`, mais la capture ne dit pas quelle rareté chaque objet porte
/// réellement dans le référentiel. Les deux teintes qu'on y distingue (orange pour les cinq
/// pierres, jaune-vert pour les plans et la combinaison) sont reportées sur les raretés du jeu qui
/// leur ressemblent le plus, et deux entrées sont poussées vers des raretés extrêmes (Relique,
/// Souvenir) pour que la planche montre l'écart entre bordures plutôt qu'un camaïeu.
const ITEMS: &[(&str, WakfuRarity, bool)] = &[
    ("Pierre d'aventure", WakfuRarity::Mythical, true),
    ("Pierre d'équilibre", WakfuRarity::Mythical, true),
    ("Pierre d'entourage", WakfuRarity::Mythical, false),
    ("Pierre de vitesse", WakfuRarity::Relic, true),
    ("Pierre ultime", WakfuRarity::Memory, true),
    ("Influence III", WakfuRarity::Legendary, true),
    ("Plan \"Epée de Bonta\"", WakfuRarity::Legendary, true),
    ("Plan \"Epée de Brâkmar\"", WakfuRarity::Legendary, false),
    ("Plan \"Epée de Sufokia\"", WakfuRarity::Legendary, true),
    ("Plan \"Epée d'Amakna\"", WakfuRarity::Legendary, true),
    ("Combinaison Lardante", WakfuRarity::Epic, true),
];

/// La phrase d'explication de la page web (clé i18n `profile.alertsDesc`).
const DESC: &str = "Objets qui déclenchent une alerte sonore et un message à l'écran lorsqu'ils \
                    sont ramassés.";

// -------------------------------------------------------------------------------------------
// Briques partagées par les trois maquettes
// -------------------------------------------------------------------------------------------

/// Titre de section en serif doré — l'idiome du jeu pour découper une fenêtre en blocs
/// (« Musique », « Sons - Ambiance » sur `interface-options-son.png` ; « Équipements », « Builds »
/// sur `interface-personnage-equiement.png`). Le jeu n'y met ni filet ni fond : le seul changement
/// de police suffit.
fn section_title(ui: &mut egui::Ui, text: &str, count: Option<usize>) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        let font = design::text::title_font(ui.ctx(), 19.0);
        ui.label(RichText::new(text).color(GOLD).font(font));
        if let Some(count) = count {
            ui.label(
                RichText::new(format!("({count})"))
                    .color(TEXT_MUTED)
                    .size(15.0),
            );
        }
    });
    ui.add_space(4.0);
}

/// Champ de recherche : le champ du design system, avec la loupe du jeu peinte À L'INTÉRIEUR,
/// collée au bord gauche — c'est ainsi que le jeu la pose (`interface-hdv-achat.png`,
/// `interface-personnage-equiement.png`), jamais sur un socle de bouton à côté.
///
/// Le décalage du texte que cette loupe impose n'est pas exprimable par l'API actuelle d'`Input`
/// (`design::components::input` n'a pas de notion d'ornement) : la maquette le simule par un
/// espace de tête dans le texte affiché. **C'est la limite la plus concrète que ces maquettes
/// révèlent** — un vrai portage demande une option `leading_icon` sur le composant.
fn search_field(ui: &mut egui::Ui, text: &mut String, placeholder: &str, width: f32) {
    // Le retrait imposé par la loupe est simulé par des espaces de tête — voir la doc de cette
    // fonction : c'est le pis-aller qui SIGNALE le manque, pas une façon de faire à reproduire.
    let response = ui.add(
        design::input(text)
            .placeholder(format!("      {placeholder}"))
            .size(InputSize::Standard)
            .width(width)
            .log_name("maquette.recherche"),
    );
    let ds = design::DesignSystem::get(ui.ctx());
    let icon = Vec2::new(14.0, 14.0);
    let center = egui::pos2(
        response.rect.left() + 10.0 + icon.x / 2.0,
        response.rect.center().y,
    );
    ds.paint(
        ui.painter(),
        Rect::from_center_size(center, icon),
        DsTexture::IconSearch,
        design::tokens::INPUT_PLACEHOLDER,
    );
}

/// Une tuile d'objet, exactement celle du panneau Suivi (`panels::watchlist::entry_tile`) :
/// fond sombre, texture de bordure de la rareté peinte AVANT l'icône (l'ordre inverse voilerait
/// l'icône d'un aplat teinté — bug corrigé le 2026-09-06, voir la doc de ce panneau), icône
/// centrée par-dessus.
///
/// Ce que la maquette ajoute, et qui n'existe pas dans le panneau Suivi : deux badges de coin,
/// la bascule du son à gauche et le retrait à droite — les deux commandes que la page web pose sur
/// sa tuile.
fn item_tile(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    name: &str,
    rarity: WakfuRarity,
    sound_on: bool,
    removable: bool,
    label_width: Option<f32>,
) -> egui::Response {
    // La CELLULE peut être plus large que la tuile : c'est ce qui donne au libellé la place que
    // 58 px ne lui laissent pas. La tuile reste à sa taille du jeu, centrée dans sa cellule.
    let cell = Vec2::new(label_width.unwrap_or(TILE), TILE);
    let (cell_rect, response) = ui.allocate_exact_size(cell, egui::Sense::hover());
    let rect = Rect::from_center_size(
        egui::pos2(cell_rect.center().x, cell_rect.top() + TILE / 2.0),
        Vec2::splat(TILE),
    );
    let painter = ui.painter();

    painter.rect_filled(rect, 2, PANEL_BG);
    egui::Image::new(icons.item_border(rarity)).paint_at(ui, rect);

    let inner = TILE * (1.0 - 2.0 * BORDER_INNER_RATIO) * ICON_FILL_RATIO;
    egui::Image::new(icons.unknown_entity_texture()).paint_at(
        ui,
        Rect::from_center_size(rect.center(), Vec2::splat(inner)),
    );

    let ds = design::DesignSystem::get(ui.ctx());
    let badge = Vec2::splat(TILE_BADGE);

    // Bascule du son — la case à cocher du design system, réduite au badge. Faute de haut-parleur
    // dans les assets du jeu (voir la doc de module), c'est le signal d'état le plus proche de son
    // vocabulaire.
    ds.paint(
        painter,
        Rect::from_min_size(rect.left_top() + Vec2::splat(2.0), badge),
        if sound_on {
            DsTexture::CheckboxChecked
        } else {
            DsTexture::CheckboxUnchecked
        },
        Color32::WHITE,
    );

    if removable {
        let corner = Rect::from_min_size(
            egui::pos2(rect.right() - TILE_BADGE - 2.0, rect.top() + 2.0),
            badge,
        );
        painter.rect_filled(corner, 2, Color32::from_black_alpha(0xB0));
        ds.paint(
            painter,
            Rect::from_center_size(corner.center(), Vec2::splat(8.0)),
            DsTexture::IconClose,
            TEXT,
        );
    }

    // Nom sous la tuile — cerné comme tout texte posé sur une image dans cette UI
    // (`design::text::paint_outlined_text`, le même procédé que le compteur du panneau Suivi).
    //
    // **Absent quand `label_width` est `None`** : le jeu n'écrit JAMAIS le nom sous un emplacement
    // d'inventaire, il le réserve à l'infobulle (`interface-personnage-equiement.png`, où 21
    // équipements tiennent sans un seul libellé). C'est ce que fait la maquette C ; la A garde les
    // libellés du web, au prix d'une cellule presque deux fois plus large.
    if let Some(width) = label_width {
        design::text::paint_outlined_text(
            ui,
            egui::pos2(cell_rect.center().x, rect.bottom() + 3.0),
            egui::Align2::CENTER_TOP,
            &elide(ui, name, width - 4.0),
            design::text::label_font(ui.ctx(), 12.0),
            TEXT,
            design::text::SHADOW_BOTTOM_RIGHT,
        );
    }

    response.on_hover_text(name)
}

/// Tronque un libellé à la largeur donnée, avec une ellipse — les noms d'objets du jeu
/// (« Plan "Epée de Brâkmar" ») dépassent largement une tuile de 58 px, et c'est déjà ce que la
/// page web fait de son côté (`text-overflow: ellipsis`).
fn elide(ui: &egui::Ui, text: &str, max_width: f32) -> String {
    let font = design::text::label_font(ui.ctx(), 12.0);
    let measure = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE)
            .rect
            .width()
    };
    if measure(text) <= max_width {
        return text.to_string();
    }
    let mut cut: String = text.to_string();
    while !cut.is_empty() && measure(&format!("{cut}…")) > max_width {
        cut.pop();
    }
    format!("{}…", cut.trim_end())
}

/// La ligne « Fermeture de l'alerte », rendue avec l'idiome du jeu : une case à cocher pour le
/// mode, un champ pour la durée.
///
/// **Écart assumé avec le web**, qui utilise un switch à deux positions « Auto | Manuelle » : le
/// jeu n'a pas ce contrôle (voir la doc de module). « Fermeture automatique » cochée = mode Auto,
/// décochée = mode Manuelle ; le champ de durée se grise avec le mode, exactement comme le web
/// désactive son champ quand `alertManualClose()` est vrai.
fn close_mode_row(ui: &mut egui::Ui, auto: &mut bool, seconds: &mut String) {
    ui.horizontal(|ui| {
        ui.add(design::checkbox(auto, "Fermeture automatique").log_name("maquette.auto"));
        ui.add_space(16.0);
        ui.add(
            design::input(seconds)
                .size(InputSize::Standard)
                .width(56.0)
                .enabled(*auto)
                .log_name("maquette.duree"),
        );
        ui.label(RichText::new("sec.").color(TEXT_MUTED).size(15.0));
    });
}

/// La ligne « Tester le son de l'alerte » — un libellé et un bouton.
///
/// Le bouton porte le glyphe `IconOption` (le rouage) faute de haut-parleur dans les assets du
/// jeu : c'est un **pis-aller signalé**, pas un choix. Voir la doc de module.
fn test_sound_row(ui: &mut egui::Ui, context: IconContext) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Tester le son de l'alerte")
                .color(TEXT)
                .size(15.0),
        );
        ui.add(
            design::icon_button(DsTexture::IconOption)
                .context(context)
                .tooltip("Tester le son de l'alerte")
                .log_name("maquette.test-son"),
        );
    });
}

/// Dispose les tuiles en grille, autant par ligne que la largeur le permet — le comportement de la
/// grille du web (`repeat(auto-fill, minmax(...))`) et de l'inventaire du jeu.
fn tile_grid(ui: &mut egui::Ui, icons: &UiIcons, width: f32, label_width: Option<f32>) {
    let cell = label_width.unwrap_or(TILE);
    let per_row = (((width + TILE_GAP) / (cell + TILE_GAP)).floor() as usize).max(1);
    // Place réservée SOUS la rangée : la hauteur du libellé quand il y en a un, la seule gouttière
    // sinon.
    let row_gap = if label_width.is_some() {
        20.0
    } else {
        TILE_GAP
    };
    for chunk in ITEMS.chunks(per_row) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = TILE_GAP;
            for (name, rarity, sound_on) in chunk {
                item_tile(ui, icons, name, *rarity, *sound_on, true, label_width);
            }
        });
        ui.add_space(row_gap);
    }
}

/// Ouvre le harnais avec le style de l'overlay déjà appliqué — les trois maquettes partagent ce
/// préambule, il n'a pas à être recopié.
///
/// **`UiIcons` est chargé UNE fois et gardé entre les frames** (`get_or_insert_with`, le même
/// motif que `tests/panels.rs`) : le recharger à chaque frame libérerait les `TextureHandle` du
/// frame précédent en fin de closure, et les tuiles se peindraient vides — constaté sur la
/// première version de ce fichier, où bordures de rareté et icônes avaient purement disparu.
fn harness(
    size: Vec2,
    mut build: impl FnMut(&mut egui::Ui, &UiIcons) + 'static,
) -> Harness<'static> {
    let mut icons: Option<UiIcons> = None;
    Harness::builder().with_size(size).build_ui(move |ui| {
        overlay_ui::style::apply(ui.ctx());
        let icons = icons.get_or_insert_with(|| UiIcons::load(ui.ctx()));
        egui::Frame::NONE.fill(BACKDROP).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            build(ui, icons);
        });
    })
}

/// Peint le corps de modale du jeu et rend l'`Ui` intérieure, marges de contenu posées.
fn modal_frame<R>(
    ui: &mut egui::Ui,
    rect: Rect,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let ds = design::DesignSystem::get(ui.ctx());
    ds.paint(ui.painter(), rect, DsTexture::ModalBody, Color32::WHITE);
    let inner = rect.shrink2(Vec2::new(MODAL_PAD_X, 22.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.set_clip_rect(inner);
        add_contents(ui)
    })
    .inner
}

// -------------------------------------------------------------------------------------------
// Maquette A — Fenêtre Options
// -------------------------------------------------------------------------------------------

/// **Le portage le plus littéral de l'idiome du jeu** : la page d'alertes devient un onglet de la
/// fenêtre Options de l'overlay, à côté de ceux qui existent déjà (`panels::options_modal`).
///
/// Anatomie reprise de `interface-options-son.png`, dans l'ordre : barre d'onglets et son bouton
/// de réinitialisation à droite, bloc d'information à pastille, sections en serif doré, pied de
/// page Annuler / Valider. Les objets suivis sont une grille de tuiles d'inventaire.
///
/// Ce que cette direction gagne : rien à inventer, tout existe déjà — l'utilisateur retrouve la
/// fenêtre qu'il connaît. Ce qu'elle coûte : une modale plein écran pour régler une alerte, alors
/// que l'overlay est fait pour être consulté en jouant.
#[test]
fn maquette_a_fenetre_options() {
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;
    let mut tab = 1_u8;

    let mut harness = harness(Vec2::new(760.0, 720.0), move |ui, icons| {
        let modal = Rect::from_min_size(
            ui.max_rect().min + Vec2::splat(20.0),
            Vec2::new(720.0, 680.0),
        );
        modal_frame(ui, modal, |ui| {
            let width = ui.available_width();

            // Barre d'onglets + réinitialisation, comme le jeu la pose : le bouton n'est PAS dans
            // la barre, il est à côté, à hauteur d'onglet.
            ui.horizontal(|ui| {
                ui.scope_builder(
                    egui::UiBuilder::new().max_rect(Rect::from_min_size(
                        ui.cursor().min,
                        Vec2::new(width - 44.0, design::tokens::TAB_HEIGHT),
                    )),
                    |ui| {
                        design::tabs(&mut tab)
                            .entry(0, "Général")
                            .entry(1, "Alertes")
                            .entry(2, "Suivi")
                            .entry(3, "Combat")
                            .log_name("maquette-a.onglets")
                            .show(ui);
                    },
                );
                ui.add(
                    design::icon_button(DsTexture::IconUndo)
                        .context(IconContext::Panel)
                        .tooltip("Rétablir les valeurs par défaut")
                        .log_name("maquette-a.reset"),
                );
            });

            ui.add_space(12.0);
            ui.add(
                design::info_text(DESC)
                    .width(width)
                    .log_name("maquette-a.desc"),
            );

            section_title(ui, "Son", None);
            test_sound_row(ui, IconContext::Panel);

            section_title(ui, "Fermeture de l'alerte", None);
            close_mode_row(ui, &mut auto, &mut seconds);

            section_title(ui, "Objets suivis", Some(ITEMS.len()));
            ui.horizontal(|ui| {
                search_field(
                    ui,
                    &mut search,
                    "Ajouter un objet à surveiller…",
                    width - 44.0,
                );
                ui.add(
                    design::icon_button(DsTexture::IconPlus)
                        .context(IconContext::Panel)
                        .tooltip("Ajouter l'objet saisi")
                        .log_name("maquette-a.ajouter"),
                );
            });
            ui.add_space(10.0);
            tile_grid(ui, icons, width, Some(TILE_LABEL_CELL));

            // Pied de page — les deux boutons de la fenêtre Options du jeu, à leur gabarit
            // compact (338 × 36), chacun sur la moitié de la largeur.
            let footer = ui.max_rect().bottom() - 36.0;
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(Rect::from_min_max(
                    egui::pos2(ui.max_rect().left(), footer),
                    ui.max_rect().max,
                )),
                |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            design::button("Annuler")
                                .variant(ButtonVariant::Danger)
                                .size(ButtonSize::Compact)
                                .width(width / 2.0 - 6.0),
                        );
                        ui.add(
                            design::button("Valider")
                                .variant(ButtonVariant::Primary)
                                .size(ButtonSize::Compact)
                                .width(width / 2.0 - 6.0),
                        );
                    });
                },
            );
        });
    });

    harness.run();
    harness.snapshot("alertes_maquette_a_fenetre_options");
}

// -------------------------------------------------------------------------------------------
// Maquette B — Liste façon Hôtel de Vente
// -------------------------------------------------------------------------------------------

/// **La direction qui assume les noms longs.** La grille de tuiles tronque tous les libellés
/// (« Plan "Epée de Brâkmar" » tient dans 58 px seulement en `Plan "Epé…` — la page web a
/// exactement ce défaut sur la capture fournie) ; l'Hôtel de Vente du jeu, lui, liste ses objets
/// en LIGNES zébrées, avec une colonne par attribut.
///
/// Anatomie reprise de `interface-hdv-achat.png` : barre de recherche et boutons d'outils, ligne
/// d'en-têtes de colonnes en gris, bandes alternées de 40 px, compteur « N objets » en tête et
/// action de vidage à droite.
///
/// Ce que cette direction gagne : le nom complet, une colonne de son lisible d'un coup d'œil, et
/// une place qui ne dépend pas du nombre d'objets. Ce qu'elle coûte : la bordure de rareté, très
/// présente en grille, se réduit ici à un liseré autour d'une icône de 28 px.
#[test]
fn maquette_b_liste_hdv() {
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;
    let mut sounds: Vec<bool> = ITEMS.iter().map(|(_, _, on)| *on).collect();

    let mut harness = harness(Vec2::new(760.0, 780.0), move |ui, icons| {
        let modal = Rect::from_min_size(
            ui.max_rect().min + Vec2::splat(20.0),
            Vec2::new(720.0, 740.0),
        );
        modal_frame(ui, modal, |ui| {
            let width = ui.available_width();

            // En-tête : titre serif, compteur, aide — l'anatomie de tête de fenêtre du jeu
            // (`interface-personnage-equiement.png` : « Équipements (21) » puis les outils).
            ui.horizontal(|ui| {
                let font = design::text::title_font(ui.ctx(), 22.0);
                ui.label(RichText::new("Alerte").color(GOLD).font(font));
                ui.label(
                    RichText::new(format!("({})", ITEMS.len()))
                        .color(TEXT_MUTED)
                        .size(16.0),
                );
                ui.add_space(width - 320.0);
                ui.add(
                    design::icon_button(DsTexture::IconHelp)
                        .context(IconContext::Panel)
                        .tooltip("À quoi sert cette page ?")
                        .log_name("maquette-b.aide"),
                );
                ui.add(
                    design::icon_button(DsTexture::IconUndo)
                        .context(IconContext::Panel)
                        .tooltip("Rétablir la liste par défaut")
                        .log_name("maquette-b.reset"),
                );
            });
            ui.add_space(6.0);
            ui.add(
                design::info_text(DESC)
                    .width(width)
                    .log_name("maquette-b.desc"),
            );

            section_title(ui, "Alerte sonore", None);
            test_sound_row(ui, IconContext::Panel);
            ui.add_space(4.0);
            close_mode_row(ui, &mut auto, &mut seconds);

            section_title(ui, "Objets suivis", Some(ITEMS.len()));
            ui.horizontal(|ui| {
                search_field(
                    ui,
                    &mut search,
                    "Ajouter un objet à surveiller…",
                    width - 200.0,
                );
                ui.add(
                    design::button("Ajouter")
                        .variant(ButtonVariant::Secondary)
                        .size(ButtonSize::Compact)
                        .width(110.0),
                );
                ui.add(
                    design::icon_button(DsTexture::IconDelete)
                        .context(IconContext::Panel)
                        .tooltip("Vider la liste")
                        .log_name("maquette-b.vider"),
                );
            });

            ui.add_space(10.0);

            // En-têtes de colonnes — gris, sans fond ni filet, comme « Nom / Niv. / Prix ».
            let col_sound = width - 150.0;
            let col_remove = width - 40.0;
            ui.horizontal(|ui| {
                let top = ui.cursor().min;
                let painter = ui.painter();
                let font = design::text::label_font(ui.ctx(), 13.0);
                painter.text(
                    egui::pos2(top.x, top.y),
                    egui::Align2::LEFT_TOP,
                    "Objet",
                    font.clone(),
                    TEXT_MUTED,
                );
                painter.text(
                    egui::pos2(ui.max_rect().left() + col_sound, top.y),
                    egui::Align2::LEFT_TOP,
                    "Son",
                    font.clone(),
                    TEXT_MUTED,
                );
                let _ = font;
                ui.allocate_space(Vec2::new(width, 18.0));
            });

            // La liste vit dans la zone de défilement du design system — onze objets ne tiennent
            // pas dans la fenêtre, et c'est le cas NORMAL d'une watchlist un peu fournie. Le
            // composant apporte la poignée de 6 px du jeu, sans rail.
            design::scroll_area("maquette-b.liste")
                .auto_shrink(false)
                .show(ui, |ui| {
                    // Zébrage : `item_spacing.y` à zéro, sans quoi une gouttière de fond de panneau
                    // s'intercale entre deux bandes et le damier se lit comme trois couleurs.
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (i, (name, rarity, _)) in ITEMS.iter().enumerate() {
                        let row = Rect::from_min_size(
                            egui::pos2(ui.max_rect().left(), ui.cursor().min.y),
                            Vec2::new(width, ROW_HEIGHT),
                        );
                        ui.painter().rect_filled(
                            row,
                            0,
                            if i % 2 == 0 { ROW_EVEN } else { ROW_ODD },
                        );

                        // Icône dans son emplacement de rareté, réduit à la hauteur de ligne.
                        let slot = Rect::from_center_size(
                            egui::pos2(row.left() + 20.0, row.center().y),
                            Vec2::splat(30.0),
                        );
                        egui::Image::new(icons.item_border(*rarity)).paint_at(ui, slot);
                        egui::Image::new(icons.unknown_entity_texture()).paint_at(
                            ui,
                            Rect::from_center_size(
                                slot.center(),
                                Vec2::splat(
                                    30.0 * (1.0 - 2.0 * BORDER_INNER_RATIO) * ICON_FILL_RATIO,
                                ),
                            ),
                        );

                        ui.painter().text(
                            egui::pos2(row.left() + 44.0, row.center().y),
                            egui::Align2::LEFT_CENTER,
                            name,
                            design::text::label_font(ui.ctx(), 15.0),
                            TEXT,
                        );

                        // `new_child` et NON `scope_builder` : ce dernier alloue dans le parent la place
                        // qu'il a utilisée, ce qui décalait chaque bande zébrée de la hauteur d'une case à
                        // cocher — les bandes faisaient alors 60 px pour un fond peint à 40, et le damier
                        // ne retombait plus jamais sur ses lignes.
                        let mut cell =
                            ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(
                                egui::pos2(row.left() + col_sound, row.center().y - 10.0),
                                Vec2::new(90.0, 20.0),
                            )));
                        cell.add(
                            design::checkbox(&mut sounds[i], "")
                                .log_name(format!("maquette-b.son-{i}")),
                        );

                        let remove = Rect::from_center_size(
                            egui::pos2(row.left() + col_remove, row.center().y),
                            Vec2::splat(20.0),
                        );
                        design::DesignSystem::get(ui.ctx()).paint(
                            ui.painter(),
                            Rect::from_center_size(remove.center(), Vec2::new(11.0, 12.0)),
                            DsTexture::IconClose,
                            TEXT_MUTED,
                        );

                        ui.allocate_space(Vec2::new(
                            width - design::components::scroll_area::RESERVE_X,
                            ROW_HEIGHT,
                        ));
                    }
                });
        });
    });

    harness.run();
    harness.snapshot("alertes_maquette_b_liste_hdv");
}

// -------------------------------------------------------------------------------------------
// Maquette C — Panneau in-game
// -------------------------------------------------------------------------------------------

/// **La direction qui refuse la modale.** L'overlay n'est pas une application à fenêtres : ses
/// deux panneaux existants (Combat, Suivi) sont des bandeaux posés par-dessus le jeu, sans cadre
/// de fenêtre, sur un fond translucide. Cette maquette porte les alertes dans ce registre-là —
/// la liste des objets suivis y devient un bandeau consultable et modifiable EN JOUANT, pas un
/// écran de configuration qu'on ouvre entre deux combats.
///
/// Anatomie reprise de `panels::watchlist` : fond `PANEL_BG`, en-tête compact d'une ligne, grille
/// dense de tuiles 58 px, barre d'outils d'icônes sur socle `FirstPlan` (le socle des boutons
/// posés par-dessus le jeu, pas celui d'un panneau de fenêtre).
///
/// Ce que cette direction gagne : cohérence avec ce que l'overlay est déjà, et une largeur qui
/// tient à côté du jeu. Ce qu'elle coûte : plus de place pour la phrase d'explication ni pour un
/// pied de page — les réglages fins (durée, mode de fermeture) doivent tenir sur une ligne, et
/// l'aide passe par une infobulle.
#[test]
fn maquette_c_panneau_ingame() {
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = harness(Vec2::new(400.0, 420.0), move |ui, icons| {
        let panel = Rect::from_min_size(
            ui.max_rect().min + Vec2::splat(16.0),
            Vec2::new(340.0, 388.0),
        );
        ui.painter().rect_filled(panel, 4, PANEL_BG);
        ui.painter().rect_stroke(
            panel,
            4,
            Stroke::new(1.0, Color32::from_rgb(0x2C, 0x2F, 0x35)),
            StrokeKind::Inside,
        );

        let inner = panel.shrink(10.0);
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
            ui.set_clip_rect(inner);
            let width = ui.available_width();

            // En-tête d'une ligne : titre, compteur, aide. Pas de barre d'onglets — ce panneau
            // n'est pas une fenêtre, il est déjà ce qu'il affiche.
            ui.horizontal(|ui| {
                let font = design::text::title_font(ui.ctx(), 18.0);
                ui.label(RichText::new("Alertes").color(GOLD).font(font));
                ui.label(
                    RichText::new(format!("({})", ITEMS.len()))
                        .color(TEXT_MUTED)
                        .size(14.0),
                );
                ui.add_space(width - 190.0);
                ui.add(
                    design::icon_button(DsTexture::IconOption)
                        .context(IconContext::FirstPlan)
                        .tooltip("Tester le son de l'alerte")
                        .log_name("maquette-c.test-son"),
                );
                ui.add(
                    design::icon_button(DsTexture::IconHelp)
                        .context(IconContext::FirstPlan)
                        .tooltip(DESC)
                        .log_name("maquette-c.aide"),
                );
            });

            ui.add_space(8.0);

            // Réglage de fermeture sur UNE ligne — la contrainte de largeur ne laisse pas la
            // place au bloc de la maquette A.
            close_mode_row(ui, &mut auto, &mut seconds);

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                search_field(ui, &mut search, "Ajouter un objet…", width - 44.0);
                ui.add(
                    design::icon_button(DsTexture::IconPlus)
                        .context(IconContext::FirstPlan)
                        .tooltip("Ajouter l'objet saisi")
                        .log_name("maquette-c.ajouter"),
                );
            });

            ui.add_space(10.0);
            tile_grid(ui, icons, width, None);
        });
    });

    harness.run();
    harness.snapshot("alertes_maquette_c_panneau_ingame");
}
