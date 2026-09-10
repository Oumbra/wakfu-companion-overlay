//! **Maquettes de la page « Alerte »** — portage de
//! `https://claude-dev.wakfu-companion.com/fr/profile/alerts` (dépôt web `Oumbra/wakfu-companion`,
//! `features/profile-page`, onglet `alerts`) dans le design system du jeu.
//!
//! **Version 2, après une revue à trois experts qui a refusé la version 1 à l'unanimité.** Les
//! trois premières maquettes proposaient un choix de contenant (fenêtre Options / liste HDV /
//! bandeau in-game) ; les trois revues ont convergé sur la même réponse, et ce n'était aucune des
//! trois : **la page se scinde par fréquence d'usage**. Ce qu'on règle une fois (le son, la durée,
//! la liste des objets) va dans l'onglet « Alertes » de la fenêtre Options — qui existe déjà,
//! `OptionsTab::Alertes`, et n'attend que son contenu. Ce qu'on consulte en jouant va dans un
//! bandeau, comme les panneaux Combat et Suivi.
//!
//! ## Ce que la version 1 a fait de faux, et qui est corrigé ici
//!
//! | Défaut | Correction |
//! | --- | --- |
//! | Titres de section **dorés** | `#b8b9ba`, le gris mesuré du jeu. **L'or ne marque jamais une hiérarchie, seulement un état** (case cochée, onglet inactif, valeur saisie) — c'était le réflexe web le plus visible de la v1 |
//! | `TEXT_MUTED` `#9AA0A6` annoncé « mesuré » | La mesure donne `#b8b9ba` : le jeu n'a pas de troisième gris, ses en-têtes de colonne ont la couleur de ses titres de section |
//! | Trois jetons citant `panels::watchlist` avec **d'autres valeurs** | `TILE_GAP` 6 → **12**, `PANEL_BG` translucide → **`#1e1e1e` opaque**, `TEXT` inventé → **blanc**. La densité de grille et le fond du bandeau — ce que la v1 donnait précisément à juger — étaient faux |
//! | `ROW_HEIGHT` 40 « mesuré sur le HDV » | Le pas réel du zébrage HDV est **60**, sur trois captures. Et le zébrage n'est de toute façon pas l'idiome retenu (voir [`ROW_HEIGHT`]) |
//! | Fenêtre **sans bannière ni panneau de section** | Le chrome est désormais celui de `panels::options_modal`, **appelé** et non recopié (voir `options_modal::chrome`) |
//! | Grille de tuiles **à libellés tronqués** | Liste en lignes. Le jeu n'écrit jamais le nom sous un emplacement d'inventaire ; et 104 px ne distinguaient pas « Plan "Epée de Bonta" » de « … Brâkmar » |
//! | Badges de coin sur **plaque noire**, case à cocher écrasée de 20 à 16 px | Le jeu pose des marqueurs **plats, sans plaque, qui débordent le liseré**, et n'y met que des ÉTATS, jamais des commandes |
//! | Croix de retrait sur les **onze** objets | Les **dix premiers sont les défauts** (`DEFAULT_SOUND_ITEM_NAMES`), que le web refuse structurellement de supprimer |
//! | Boutons « Vider la liste » / « Rétablir » **inventés** | Retirés : ils n'existent pas côté web, et détruisaient tout sans confirmation |
//! | Retrait **sans confirmation** | Maquetté — voir [`alertes_options_confirmation_retrait`] |
//! | Rouage pour « Tester le son » | Bouton **texte**. Le rouage veut déjà dire « options » dans ce design system, et le jeu utilise des boutons texte pour les actions sans glyphe évident |
//! | Snapshots **comparés** | Les captures sont écrites dans `target/`, jamais commitées — voir [`write_mockup`] |
//!
//! ## Ce que ce portage demande, et qui n'existe pas encore
//!
//! Une maquette sert à chiffrer un portage. Voici ce qu'il faudra, par ordre de nécessité :
//!
//! 1. **`Input::leading_icon(DsTexture)`** — le jeu pose systématiquement la loupe DANS le champ
//!    (`interface-hdv-achat.png` x 27..39, `interface-personnage-equiement.png` x 768..780,
//!    `interface-options-commandes.png` x 36..48). `Input` ne sait pas le faire ; ces maquettes
//!    peignent la loupe par-dessus, ce qui la laisse hors du `clip_rect` du champ et masquerait
//!    une valeur saisie. Mesuré sur `empty-input-search.png` (341 × 32, champ y 2..29) : encre
//!    13 × 13, 7 px du bord, 8 px avant le premier glyphe du texte.
//! 2. **Un composant « tuile d'objet »** — déjà dupliqué entre `panels::watchlist::entry_tile` et
//!    ce fichier ; un portage en ferait une troisième copie. À trancher avant : les bordures de
//!    rareté font 512 × 512 pour des tuiles peintes à 58, et `DesignSystem::load` téléverse tout
//!    le manifeste d'un bloc — les y verser telles quelles multiplierait par 4,5 son coût.
//! 3. **`Checkbox::size(f32)`** — pour qu'un marqueur d'état n'ait plus à écraser un asset de 20 px.
//! 4. **Une confirmation de retrait** — aucune brique de ce genre n'existe dans `overlay-ui` ; le
//!    web a `ConfirmDeleteService` (popover ancrée au bouton).
//! 5. **`SoundItemEntry::is_default`** — le miroir Rust (`overlay_engine::profile`) n'a pas ce
//!    champ, la protection des dix objets par défaut est donc aujourd'hui impossible à appliquer.
//! 6. **Un panneau de suggestions** sur le champ d'ajout — c'est le vrai mécanisme d'ajout (une
//!    alerte a besoin d'un `catalogId` résolu par le catalogue, pas d'un nom libre).
//!    `design::select` sait déjà peindre une liste par-dessus le contenu suivant.
//! 7. **Une icône « haut-parleur »** — aucun des 34 glyphes de `assets/design-system/icons/` n'en
//!    est un, et l'onglet Son du jeu règle son volume sans jamais en dessiner. À extraire du client
//!    (skill `design-asset`) si l'on veut le pictogramme du web.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use egui::{Color32, Rect, RichText, Stroke, StrokeKind, Vec2};
use egui_kittest::Harness;
use overlay_engine::WakfuRarity;
use overlay_ui::design::{self, ButtonSize, ButtonVariant, DsTexture, IconContext, InputSize};
use overlay_ui::panels::options_modal::{self, OptionsModalAssets, OptionsTab};
use overlay_ui::ui_icons::UiIcons;

// -------------------------------------------------------------------------------------------
// Jetons propres aux maquettes.
//
// Chacun dit d'où il vient : mesure sur une capture du jeu, valeur REPRISE TELLE QUELLE d'un
// panneau existant, ou choix assumé comme tel. La v1 a livré trois jetons qui annonçaient une
// reprise et donnaient une autre valeur — le pire cas, puisque rien ne les distinguait à la
// lecture d'une mesure authentique. Les valeurs ci-dessous ont été relues une à une contre leur
// source, ligne par ligne.
// -------------------------------------------------------------------------------------------

/// Fond derrière une fenêtre — le même sombre que `tests/panels.rs` pose sous la modale Options,
/// pour qu'un bord translucide se voie.
const BACKDROP: Color32 = Color32::from_rgb(0x0B, 0x0D, 0x10);

/// Fond d'un bandeau posé sur le jeu — **`PANEL_BG` de `panels::watchlist:492`, à sa valeur
/// exacte** : `#1e1e1e`, et **opaque**. La v1 annonçait cette source en peignant un translucide
/// `#14161AE0` qui n'existe nulle part.
const PANEL_BG: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);

/// Bord d'un bandeau — `BORDER_STRONG` de `panels::watchlist:496`.
const BORDER_STRONG: Color32 = Color32::from_rgb(0x4D, 0x4D, 0x4D);

/// Texte courant — **`TEXT_COLOR` de `panels::watchlist:510`, c'est-à-dire blanc**. La v1 déclarait
/// reprendre ce jeton et peignait `#E4E6E8`.
const TEXT: Color32 = Color32::WHITE;

/// Gris des titres de section, en-têtes de colonne, compteurs et unités — **`#b8b9ba`**.
///
/// C'est `panels::options_modal::SECTION_TITLE_TEXT`, et c'est une mesure convergente sur quatre
/// captures indépendantes : « Musique » `#b9baba` et « Sons - Ambiance » `#b8b9ba`
/// (`interface-options-son.png`), « Équipements » `#babbbc`
/// (`interface-personnage-equiement.png`), « Types » `#bababa` (`interface-hdv-achat.png`). Les
/// en-têtes de colonne du HDV donnent la même valeur (`#b9babb` sur « Nom », `#bababc` sur
/// « Quantité ») : **le jeu n'a pas de troisième gris atténué**, et en inventer un — ce que faisait
/// le `TEXT_MUTED` `#9AA0A6` de la v1, plus sombre de 20 % et bleuté là où le gris du jeu est
/// neutre — crée un niveau de hiérarchie qui n'existe pas.
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);

/// Côté d'une tuile d'objet — `TILE_SIZE` de `panels::watchlist:298`.
const TILE: f32 = 58.0;

/// Gouttière entre deux tuiles — **`TILE_GAP` de `panels::watchlist:301`, c'est-à-dire 12**. La v1
/// annonçait cette source en écrivant 6, ce qui **doublait la densité de la grille** : exactement
/// la variable que la maquette du bandeau donnait à juger.
const TILE_GAP: f32 = 12.0;

/// Fenêtre intérieure de `Border-<RARETÉ>.webp` — `ITEM_BORDER_INNER_MARGIN_RATIO` de
/// `panels::watchlist`, mesurée une fois pour toutes sur les sept fichiers (52/512).
const BORDER_INNER_RATIO: f32 = 52.0 / 512.0;

/// Part de la fenêtre intérieure occupée par l'icône — `ITEM_ICON_FILL_RATIO` de
/// `panels::watchlist`, réglé sur retour utilisateur (« les objets doivent être plus gros »).
const ICON_FILL_RATIO: f32 = 0.96;

/// Hauteur d'une ligne de réglage — **mesurée sur `interface-options-commandes.png`**, la liste de
/// raccourcis du jeu : ses champs tombent à y = 202, 241, 280, 319, 358, soit un pas constant de
/// 39 px.
///
/// C'est l'idiome retenu pour les objets suivis, à la place de la table zébrée du HDV que
/// proposait la v1. Le jeu réserve celle-ci (pas de 60 px, une bande sur deux voilée) aux
/// **données multi-colonnes** — Nom / Niv. / Enchantement / Quantité / Prix. Pour « un libellé à
/// gauche, un contrôle à droite », il pose une liste plate dans un panneau, sans zébrage : c'est
/// très exactement la forme de la page Alerte.
const ROW_HEIGHT: f32 = 39.0;

/// Côté de l'emplacement de rareté en tête de ligne. **Choix de mise en page, pas une mesure** :
/// la ligne fait 39, 30 la remplit sans la toucher. Le jeu n'a pas de liste d'objets à cette
/// échelle dont on puisse tirer une cote.
const ROW_SLOT: f32 = 30.0;

/// Fond d'une ligne SURVOLÉE — le seul fond peint de la liste (voir [`ROW_HEIGHT`] : pas de
/// zébrage). Repris de la surbrillance d'entrée de liste déroulante du design system,
/// `tokens::SELECT_ROW_HIGHLIGHT`, à opacité réduite : c'est le seul « fond de ligne active »
/// mesuré du jeu.
const ROW_HOVER: Color32 = Color32::from_rgba_unmultiplied_const(0xA5, 0x8E, 0x63, 0x30);

/// Vert de la coche d'état du jeu — mesuré sur la coche verte d'un équipement équipé
/// (`interface-personnage-equiement.png`, x 765..780 / y 395..405).
const TICK_GREEN: Color32 = Color32::from_rgb(0x7A, 0xC7, 0x4F);

struct Item {
    name: &'static str,
    rarity: WakfuRarity,
    sound_on: bool,
    /// Objet de la liste prédéfinie : jamais retirable — voir [`ITEMS`].
    is_default: bool,
}

impl Item {
    const fn preset(name: &'static str, rarity: WakfuRarity, sound_on: bool) -> Self {
        Self {
            name,
            rarity,
            sound_on,
            is_default: true,
        }
    }
    const fn added(name: &'static str, rarity: WakfuRarity, sound_on: bool) -> Self {
        Self {
            name,
            rarity,
            sound_on,
            is_default: false,
        }
    }
}

/// Les onze objets de la capture fournie par l'utilisateur, dans son ordre.
///
/// **Les dix premiers sont exactement `DEFAULT_SOUND_ITEM_NAMES`** (dépôt web,
/// `core/services/profile.service.ts:52-63`), donc les objets par défaut — que `removeSoundItem`
/// refuse structurellement de supprimer (`e.isDefault || …`) et pour lesquels le web n'affiche
/// aucune croix. Le onzième, « Combinaison Lardante », est l'ajout du joueur : le seul retirable.
/// La v1 mettait une croix sur les onze.
///
/// **Les raretés sont une reconstitution, pas une donnée** : la capture ne dit pas quelle rareté
/// le référentiel attribue à chaque objet. Les deux teintes qu'on y distingue sont reportées sur
/// les raretés du jeu qui leur ressemblent, et deux entrées sont poussées vers des raretés
/// extrêmes pour que la planche montre l'écart entre bordures plutôt qu'un camaïeu.
const ITEMS: &[Item] = &[
    Item::preset("Pierre d'aventure", WakfuRarity::Mythical, true),
    Item::preset("Pierre d'équilibre", WakfuRarity::Mythical, true),
    Item::preset("Pierre d'entourage", WakfuRarity::Mythical, false),
    Item::preset("Pierre de vitesse", WakfuRarity::Relic, true),
    Item::preset("Pierre ultime", WakfuRarity::Memory, true),
    Item::preset("Influence III", WakfuRarity::Legendary, true),
    Item::preset("Plan \"Epée de Bonta\"", WakfuRarity::Legendary, true),
    Item::preset("Plan \"Epée de Brâkmar\"", WakfuRarity::Legendary, false),
    Item::preset("Plan \"Epée de Sufokia\"", WakfuRarity::Legendary, true),
    Item::preset("Plan \"Epée d'Amakna\"", WakfuRarity::Legendary, true),
    Item::added("Combinaison Lardante", WakfuRarity::Epic, true),
];

/// La phrase d'explication de la page web (clé i18n `profile.alertsDesc`).
const DESC: &str = "Objets qui déclenchent une alerte sonore et un message à l'écran lorsqu'ils \
                    sont ramassés.";

/// Ce que le panneau de suggestions afficherait pour la saisie « pierre » — le troisième champ dit
/// si l'objet est DÉJÀ suivi (grisé, non sélectionnable, comme côté web).
const SUGGESTIONS: &[(&str, WakfuRarity, bool)] = &[
    ("Pierre d'aventure", WakfuRarity::Mythical, true),
    ("Pierre de dolomite", WakfuRarity::Common, false),
    ("Pierre de lune", WakfuRarity::Rare, false),
    ("Pierre ponce", WakfuRarity::Common, false),
];

// -------------------------------------------------------------------------------------------
// Briques partagées
// -------------------------------------------------------------------------------------------

/// Peint l'emplacement d'objet du jeu (bordure de rareté + icône) dans `rect`.
///
/// **La bordure se peint AVANT l'icône**, jamais après : la fenêtre intérieure de
/// `Border-<RARETÉ>.webp` n'est pas transparente mais un aplat semi-opaque teinté par la rareté —
/// peinte après, elle voile l'icône entière. C'est un bug réel corrigé le 2026-09-06 dans
/// `panels::watchlist`, et la première chose que la doc du futur composant devra porter.
///
/// L'icône est le repli générique : les vraies viennent du CDN (`remote_icons`), inaccessible
/// depuis le harnais. C'est aussi ce que l'overlay affiche tant qu'un téléchargement n'a pas
/// abouti — ce qui est jugé ici est l'emplacement, pas le dessin de l'objet.
fn item_slot(ui: &egui::Ui, icons: &UiIcons, rect: Rect, rarity: WakfuRarity) {
    egui::Image::new(icons.item_border(rarity)).paint_at(ui, rect);
    let inner = rect.width() * (1.0 - 2.0 * BORDER_INNER_RATIO) * ICON_FILL_RATIO;
    egui::Image::new(icons.unknown_entity_texture()).paint_at(
        ui,
        Rect::from_center_size(rect.center(), Vec2::splat(inner)),
    );
}

/// Champ de recherche : le champ du design system, la loupe du jeu peinte à l'intérieur.
///
/// **Pis-aller de maquette, à ne pas reproduire** — voir le point 1 de la doc de module. La loupe
/// est peinte par l'appelant, donc hors du `clip_rect` du champ, et le texte indicatif est décalé
/// par des espaces de tête : une valeur SAISIE, elle, commencerait sous la loupe. C'est
/// précisément ce que `Input::leading_icon` doit régler.
fn search_field(ui: &mut egui::Ui, text: &mut String, placeholder: &str, width: f32) {
    let response = ui.add(
        design::input(text)
            .placeholder(format!("      {placeholder}"))
            .size(InputSize::Standard)
            .width(width)
            .log_name("maquette.recherche"),
    );
    // Cotes mesurées sur `empty-input-search.png` (champ y 2..29, soit 28 px de haut), exprimées
    // en fraction de la hauteur du champ comme tous les jetons de `design::components::input` :
    // la capture est à une échelle d'interface plus grande que les 25 px de `InputSize::Standard`.
    let h = response.rect.height();
    let icon = Vec2::splat(h * 13.0 / 28.0);
    let center = egui::pos2(
        response.rect.left() + h * 7.0 / 28.0 + icon.x / 2.0,
        response.rect.center().y,
    );
    design::DesignSystem::get(ui.ctx()).paint(
        ui.painter(),
        Rect::from_center_size(center, icon),
        DsTexture::IconSearch,
        design::tokens::INPUT_PLACEHOLDER,
    );
}

/// Une ligne de la liste d'objets suivis — idiome `interface-options-commandes.png` : emplacement
/// de rareté, nom complet, contrôles alignés en colonne à droite, aucun zébrage.
///
/// Le nom n'est jamais tronqué : c'est tout l'intérêt de la ligne sur la grille, où « Plan "Epée de
/// Bonta" » et « Plan "Epée de Brâkmar" » rendaient la même chaîne.
fn item_row(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    item: &Item,
    sound_on: &mut bool,
    width: f32,
    hovered: bool,
    index: usize,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), egui::Sense::hover());
    if hovered {
        ui.painter().rect_filled(rect, 2, ROW_HOVER);
    }

    item_slot(
        ui,
        icons,
        Rect::from_center_size(
            egui::pos2(rect.left() + ROW_SLOT / 2.0, rect.center().y),
            Vec2::splat(ROW_SLOT),
        ),
        item.rarity,
    );

    ui.painter().text(
        egui::pos2(rect.left() + ROW_SLOT + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        item.name,
        design::text::label_font(ui.ctx(), 15.0),
        TEXT,
    );

    // Colonne « Son » : la case à cocher du design system, à sa taille native de 20 px.
    // Précédent direct dans le jeu — panneau « Types » de `interface-hdv-achat.png` (une case par
    // famille) et « ☐ Trier par type d'équipement » de `interface-personnage-equiement.png`.
    //
    // `new_child` et non `scope_builder` : ce dernier alloue dans le parent la place qu'il a
    // utilisée, ce qui décalerait chaque ligne de la hauteur d'une case.
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(
        egui::pos2(rect.right() - 74.0, rect.center().y - 10.0),
        Vec2::splat(20.0),
    )));
    cell.add(design::checkbox(sound_on, "").log_name(format!("maquette.son-{index}")));

    // Retrait — **absent sur les dix objets par défaut**, que le web refuse de supprimer. Bouton
    // icône du design system, jamais un glyphe peint à la main : celui de la v1 n'avait ni zone
    // cliquable, ni survol, ni journal, et déformait un glyphe 13 × 14 en carré de 8.
    if !item.is_default {
        let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(
            egui::pos2(rect.right() - 36.0, rect.center().y - 18.0),
            Vec2::splat(36.0),
        )));
        cell.add(
            design::icon_button(DsTexture::IconClose)
                .context(IconContext::Panel)
                .tooltip(format!("Retirer « {} »", item.name))
                .log_name(format!("maquette.retirer-{index}")),
        );
    }
}

/// Les deux réglages sonores, en tête du panneau — « Tester » et la fermeture du message.
///
/// Le bouton de test est un bouton **texte**, pas une icône : aucun glyphe du design system ne dit
/// « jouer un son », et `IconOption` (le rouage) veut déjà dire « options » — il est visible avec
/// ce sens dans `interface-hdv-achat.png` (x≈1196, y≈32) et `interface-personnage-equiement.png`
/// (x≈708, y≈86). Le jeu emploie des boutons texte pour ses actions sans glyphe évident
/// (`interface-options-interface.png` : « Recharger le thème », « Ouvrir le dossier »…).
fn sound_settings(ui: &mut egui::Ui, auto: &mut bool, seconds: &mut String, width: f32) {
    // **Une seule ligne pour les deux réglages**, et non deux : le panneau de section de la
    // fenêtre Options n'offre qu'environ 330 px de haut à la taille du jeu, dont chaque ligne de
    // réglage retire une ligne d'objet visible. Les deux réglages tiennent côte à côte sans se
    // serrer, le jeu compose lui-même des lignes de ce genre (`interface-options-son.png` :
    // « ☐ Couper la musique   ☐ Lecture continue »).
    let row = ui.allocate_space(Vec2::new(width, ROW_HEIGHT)).1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    cell.horizontal(|ui| {
        ui.add(
            design::button("Tester le son")
                .variant(ButtonVariant::Secondary)
                .size(ButtonSize::Compact)
                .log_name("maquette.tester"),
        );
        ui.add_space(16.0);
        // Case à cocher plutôt que le switch « Auto | Manuelle » du web : le jeu n'a pas de switch
        // à deux positions, son idiome pour un choix binaire est la case
        // (`interface-options-son.png` : « Couper la musique », « Lecture continue »).
        ui.add(design::checkbox(auto, "Fermeture automatique").log_name("maquette.auto"));
        ui.add_space(12.0);
        ui.add(
            design::input(seconds)
                .size(InputSize::Standard)
                .width(52.0)
                .enabled(*auto)
                .log_name("maquette.duree"),
        );
        ui.label(RichText::new("sec.").color(SUBDUED).size(15.0));
    });
}

/// En-tête des colonnes de la liste — même gris que les titres de section, voir [`SUBDUED`].
fn column_header(ui: &mut egui::Ui, width: f32) {
    let rect = ui.allocate_space(Vec2::new(width, 20.0)).1;
    let font = design::text::label_font(ui.ctx(), 13.0);
    ui.painter().text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        "Objet",
        font.clone(),
        SUBDUED,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 74.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "Son",
        font,
        SUBDUED,
    );
}

/// Le panneau de suggestions du champ d'ajout — voir [`alertes_options_ajout_suggestions`].
fn suggestion_list(ui: &egui::Ui, icons: &UiIcons, inner: Rect, field_bottom: f32) {
    let list = Rect::from_min_size(
        egui::pos2(inner.left(), field_bottom + 2.0),
        Vec2::new(inner.width() * 0.62, 4.0 + SUGGESTIONS.len() as f32 * 30.0),
    );
    ui.painter()
        .rect_filled(list, 2, design::tokens::SELECT_LIST_FILL);
    ui.painter().rect_stroke(
        list,
        2,
        Stroke::new(2.0, design::tokens::SELECT_LIST_BORDER),
        StrokeKind::Inside,
    );
    for (i, (name, rarity, already)) in SUGGESTIONS.iter().enumerate() {
        let row = Rect::from_min_size(
            egui::pos2(list.left() + 2.0, list.top() + 2.0 + i as f32 * 30.0),
            Vec2::new(list.width() - 4.0, 30.0),
        );
        // Une seule entrée en surbrillance : celle que le clavier désignerait.
        if i == 1 {
            ui.painter()
                .rect_filled(row, 0, design::tokens::SELECT_ROW_HIGHLIGHT);
        }
        item_slot(
            ui,
            icons,
            Rect::from_center_size(
                egui::pos2(row.left() + 17.0, row.center().y),
                Vec2::splat(24.0),
            ),
            *rarity,
        );
        ui.painter().text(
            egui::pos2(row.left() + 36.0, row.center().y),
            egui::Align2::LEFT_CENTER,
            if *already {
                format!("{name}   — déjà suivi")
            } else {
                name.to_string()
            },
            design::text::label_font(ui.ctx(), 15.0),
            if *already {
                design::tokens::TEXT_DISABLED
            } else {
                design::tokens::SELECT_TEXT
            },
        );
    }
}

/// La boîte de confirmation de retrait, ancrée sous la croix de sa ligne — voir
/// [`alertes_options_confirmation_retrait`].
fn confirm_box(ui: &mut egui::Ui, anchor: egui::Pos2, item_name: &str) {
    let size = Vec2::new(248.0, 80.0);
    let rect = Rect::from_min_size(egui::pos2(anchor.x - size.x + 18.0, anchor.y + 2.0), size);
    // **Peinte hors du clip du panneau** : une popover ancrée sur la dernière ligne visible sort
    // forcément du panneau qui la contient, et le clip lui coupait ses deux boutons. Un portage
    // passerait par `egui::Area`, qui vit au-dessus de tout le reste ; ici on relâche le clip, ce
    // qui produit la même géométrie.
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    ui.set_clip_rect(Rect::EVERYTHING);
    let ui = &mut ui;
    ui.painter()
        .rect_filled(rect, 2, design::tokens::SELECT_LIST_FILL);
    ui.painter().rect_stroke(
        rect,
        2,
        Stroke::new(2.0, design::tokens::SELECT_LIST_BORDER),
        StrokeKind::Inside,
    );
    ui.painter().text(
        egui::pos2(rect.center().x, rect.top() + 18.0),
        egui::Align2::CENTER_CENTER,
        format!("Retirer « {item_name} » ?"),
        design::text::label_font(ui.ctx(), 14.0),
        Color32::WHITE,
    );
    let mut buttons = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(
        egui::pos2(rect.left() + 12.0, rect.top() + 34.0),
        Vec2::new(rect.width() - 24.0, 36.0),
    )));
    buttons.horizontal(|ui| {
        ui.add(
            design::button("Annuler")
                .variant(ButtonVariant::Secondary)
                .size(ButtonSize::Compact)
                .width(106.0)
                .log_name("maquette.confirm-non"),
        );
        ui.add(
            design::button("Retirer")
                .variant(ButtonVariant::Danger)
                .size(ButtonSize::Compact)
                .width(106.0)
                .log_name("maquette.confirm-oui"),
        );
    });
}

/// Écrit la capture rendue dans `target/mockups/` — **jamais un snapshot comparé**.
///
/// La revue d'architecture l'a établi : l'étape `overlay-testkit` du CI porte
/// `continue-on-error: true` (§17.3 du plan, pas encore promue en gate), un écart sur ces images
/// ne peut donc rien faire échouer. La comparaison n'achetait aucune protection et coûtait 1 Mo de
/// PNG définitivement dans l'historique d'un dépôt privé, plus un `UPDATE_SNAPSHOTS=1` à rejouer à
/// chaque changement de jeton — sur des images destinées à disparaître avec ce fichier.
///
/// `target/` est déjà ignoré par git : rien n'est commité, et la capture reste à disposition pour
/// être publiée en Artifact (obligation CLAUDE.md — l'utilisateur travaille en terminal).
fn write_mockup(harness: &mut Harness<'static>, name: &str) {
    let image = harness
        .render()
        .expect("rendu offscreen — voir doc de module");
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/mockups");
    std::fs::create_dir_all(&dir).expect("création de target/mockups");
    image
        .save(dir.join(format!("{name}.png")))
        .expect("écriture de la capture");
}

/// Ouvre le harnais et rend la fenêtre Options avec son chrome réel, l'onglet « Alertes » actif.
///
/// **Le décor n'est pas recopié : c'est `options_modal::chrome` qui le peint**, la même fonction
/// que la vraie modale — extraite pour ces maquettes, à pixel constant (le snapshot
/// `options_modale_sur_damier.png` le vérifie à chaque exécution). C'était la réserve la plus
/// lourde de la revue d'architecture : une deuxième implémentation du chrome aurait divergé en
/// quelques semaines.
fn options_harness(
    size: Vec2,
    mut build: impl FnMut(&mut egui::Ui, &UiIcons, Rect) + 'static,
) -> Harness<'static> {
    let mut icons: Option<UiIcons> = None;
    let mut assets: Option<OptionsModalAssets> = None;
    let mut tab = OptionsTab::Alertes;
    Harness::builder().with_size(size).build_ui(move |ui| {
        overlay_ui::style::apply(ui.ctx());
        // Chargés UNE fois et gardés entre les frames (même motif que `tests/panels.rs`) : les
        // recharger à chaque frame libérerait les `TextureHandle` du frame précédent en fin de
        // closure, et les emplacements de rareté se peindraient vides.
        let icons = icons.get_or_insert_with(|| UiIcons::load(ui.ctx()));
        let assets = assets.get_or_insert_with(|| OptionsModalAssets::load(ui.ctx()));
        egui::Frame::NONE.fill(BACKDROP).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let chrome = options_modal::chrome(
                ui,
                assets,
                &mut tab,
                &[OptionsTab::Alertes, OptionsTab::Parametres],
            );
            let inner = chrome.inner;
            ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
                // Clip élargi à gauche : un titre de section se peint EN RETRAIT de son axe de
                // contrôles (`PANEL_PAD_CONTROL_X - PANEL_PAD_TITLE_X`, 7 px), et un clip calé sur
                // `inner` lui mangeait sa première lettre — « Alerte » rendait « lerte ».
                ui.set_clip_rect(inner.expand2(egui::vec2(8.0, 0.0)));
                ui.spacing_mut().item_spacing.y = 0.0;
                build(ui, icons, inner);
            });
        });
    })
}

/// Le contenu de l'onglet, commun aux deux tailles et aux deux états de liste.
#[allow(clippy::too_many_arguments)]
fn alerts_tab(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    inner: Rect,
    items: &[Item],
    sounds: &mut [bool],
    search: &mut String,
    auto: &mut bool,
    seconds: &mut String,
    hovered_row: Option<usize>,
) {
    let width = inner.width();

    // Pas de titre de section « Alerte » : l'onglet actif porte déjà ce nom, et chaque ligne
    // rendue au titre est une ligne d'objet perdue dans un panneau déjà court.
    ui.add(
        design::info_text(DESC)
            .width(width)
            .log_name("maquette.desc"),
    );
    ui.add_space(6.0);
    sound_settings(ui, auto, seconds, width);

    ui.add_space(6.0);
    options_modal::section_title(ui, &format!("Objets suivis ({})", items.len()));
    search_field(ui, search, "Ajouter un objet à surveiller…", width);
    ui.add_space(6.0);

    if items.is_empty() {
        // État vide — le web ne peut pas l'atteindre (dix défauts toujours réinjectés), l'overlay
        // si : `sound_items_from_settings_json` rend une liste vide dès que la clé `profile`
        // manque, et la synchronisation exige un compte connecté. Un grand blanc ne dit rien ; ce
        // bloc dit quoi faire.
        ui.add_space(10.0);
        ui.add(
            design::info_text(
                "Aucun objet suivi. Connectez-vous à votre compte pour retrouver vos alertes, ou \
                 ajoutez un objet ci-dessus.",
            )
            .width(width)
            .log_name("maquette.vide"),
        );
        return;
    }

    column_header(ui, width);
    design::scroll_area("maquette.liste")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let row_width = width - design::components::scroll_area::RESERVE_X;
            for (i, item) in items.iter().enumerate() {
                item_row(
                    ui,
                    icons,
                    item,
                    &mut sounds[i],
                    row_width,
                    hovered_row == Some(i),
                    i,
                );
            }
        });
}

// -------------------------------------------------------------------------------------------
// 1 & 2 — L'onglet « Alertes » de la fenêtre Options, à deux tailles
// -------------------------------------------------------------------------------------------

/// **À la taille ACTUELLE de la fenêtre Options de l'overlay (560 × 436).**
///
/// C'est le constat, pas la proposition : le panneau de section n'offre alors qu'environ 220 px de
/// haut, dont les réglages sonores et les deux titres consomment l'essentiel. **Deux lignes
/// d'objets restent visibles sur onze.** La liste est bien défilante — rien n'est perdu, à la
/// différence de la v1 qui écrêtait en silence — mais consulter onze objets par une fenêtre de
/// deux lignes n'est pas utilisable.
///
/// C'est l'argument, en image, de la maquette suivante.
#[test]
fn alertes_options_taille_actuelle() {
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = options_harness(Vec2::new(560.0, 436.0), move |ui, icons, inner| {
        alerts_tab(
            ui,
            icons,
            inner,
            ITEMS,
            &mut sounds,
            &mut search,
            &mut auto,
            &mut seconds,
            None,
        );
    });
    harness.run();
    write_mockup(&mut harness, "alertes_options_taille_actuelle");
}

/// **À la taille de la fenêtre Options DU JEU (720 × 561).**
///
/// Le redimensionnement que la maquette précédente rend nécessaire, et il ne demande aucune
/// invention : `WINDOW_SIZE` de l'overlay (560 × 436) a été choisi en alignant son rapport
/// d'aspect sur les 720 × 561 mesurés de la vraie fenêtre du jeu (demande utilisateur du
/// 2026-09-09 : « garder une cohérence par rapport au rendu du jeu »). Reprendre la taille native
/// garde ce rapport **exactement**, et c'est celle à laquelle toutes les cotes du design system
/// ont été relevées.
///
/// La dernière ligne est ici montrée SURVOLÉE — c'est la seule affordance qui dise que ses
/// contrôles sont des boutons, et aucune maquette de la v1 n'avait d'état de survol.
#[test]
fn alertes_options_taille_jeu() {
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = options_harness(Vec2::new(720.0, 561.0), move |ui, icons, inner| {
        alerts_tab(
            ui,
            icons,
            inner,
            ITEMS,
            &mut sounds,
            &mut search,
            &mut auto,
            &mut seconds,
            Some(10),
        );
    });
    harness.run();
    write_mockup(&mut harness, "alertes_options_taille_jeu");
}

/// **L'état vide, celui du premier lancement.**
///
/// Aucune maquette de la v1 ne le rendait, et c'est pourtant le premier écran qu'un joueur non
/// connecté verra : contrairement au web, où dix objets par défaut sont toujours réinjectés,
/// l'overlay part d'une liste vide tant que la synchronisation n'a pas eu lieu.
#[test]
fn alertes_options_liste_vide() {
    let mut sounds: Vec<bool> = Vec::new();
    let mut search = String::new();
    // 3,5 s : le défaut réel du web (`DEFAULT_ALERT_DURATION_SECONDS`). Les autres maquettes
    // montrent 4, la valeur réglée sur la capture fournie par l'utilisateur.
    let mut seconds = String::from("3.5");
    let mut auto = true;

    let mut harness = options_harness(Vec2::new(720.0, 561.0), move |ui, icons, inner| {
        alerts_tab(
            ui,
            icons,
            inner,
            &[],
            &mut sounds,
            &mut search,
            &mut auto,
            &mut seconds,
            None,
        );
    });
    harness.run();
    write_mockup(&mut harness, "alertes_options_liste_vide");
}

// -------------------------------------------------------------------------------------------
// 3 — Ajout d'un objet, et confirmation de retrait
// -------------------------------------------------------------------------------------------

/// **Le panneau de suggestions — le vrai mécanisme d'ajout**, que la v1 ne montrait pas.
///
/// Le champ n'accepte pas un nom libre : une alerte a besoin d'un `catalogId` résolu par le
/// catalogue, sans quoi elle ne se déclenchera jamais. Chaque suggestion porte son emplacement de
/// rareté, et une entrée **déjà suivie** est grisée — miroir du web, dont `addSoundItem` retourne
/// en silence sur un doublon, ce qui laisse sinon le joueur retenter cinq fois un ajout qui ne
/// fait rien.
///
/// La liste est peinte APRÈS le contenu, donc par-dessus, comme le fait `design::select` avec sa
/// liste dépliée. Un portage passerait par lui (`egui::Area`), pour qu'elle survive aussi au clip
/// du panneau.
#[test]
fn alertes_options_ajout_suggestions() {
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::from("pierre");
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = options_harness(Vec2::new(720.0, 561.0), move |ui, icons, inner| {
        let width = inner.width();

        ui.add(
            design::info_text(DESC)
                .width(width)
                .log_name("maquette.desc"),
        );
        ui.add_space(6.0);
        sound_settings(ui, &mut auto, &mut seconds, width);
        ui.add_space(6.0);
        options_modal::section_title(ui, &format!("Objets suivis ({})", ITEMS.len()));
        search_field(ui, &mut search, "Ajouter un objet à surveiller…", width);
        let field_bottom = ui.cursor().min.y;
        ui.add_space(6.0);
        column_header(ui, width);
        let row_width = width - design::components::scroll_area::RESERVE_X;
        for (i, item) in ITEMS.iter().take(4).enumerate() {
            item_row(ui, icons, item, &mut sounds[i], row_width, false, i);
        }

        suggestion_list(ui, icons, inner, field_bottom);
    });
    harness.run();
    write_mockup(&mut harness, "alertes_options_ajout_suggestions");
}

/// **La confirmation de retrait**, parité avec le `ConfirmDeleteService` du web : un clic sur la
/// croix n'efface pas, il demande.
///
/// La v1 supprimait au premier clic, et sur les onze objets — dix d'entre eux étant les défauts
/// que le web protège. Ici, seule « Combinaison Lardante » porte une croix, et elle ouvre cette
/// boîte ancrée sous elle.
///
/// **Aucune brique de ce genre n'existe dans `overlay-ui`** : elle est peinte à la main, et c'est
/// précisément ce que cette maquette sert à chiffrer.
#[test]
fn alertes_options_confirmation_retrait() {
    // Les deux premiers défauts, puis l'objet ajouté par le joueur — le seul retirable, et donc
    // le seul que la confirmation puisse viser.
    let shown: [usize; 3] = [0, 1, ITEMS.len() - 1];
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = options_harness(Vec2::new(720.0, 561.0), move |ui, icons, inner| {
        let width = inner.width();
        ui.add(
            design::info_text(DESC)
                .width(width)
                .log_name("maquette.desc"),
        );
        ui.add_space(6.0);
        sound_settings(ui, &mut auto, &mut seconds, width);
        ui.add_space(6.0);
        options_modal::section_title(ui, &format!("Objets suivis ({})", ITEMS.len()));
        search_field(ui, &mut search, "Ajouter un objet à surveiller…", width);
        ui.add_space(6.0);
        column_header(ui, width);

        let row_width = width - design::components::scroll_area::RESERVE_X;
        let mut anchor = inner.center();
        for (rank, &i) in shown.iter().enumerate() {
            item_row(
                ui,
                icons,
                &ITEMS[i],
                &mut sounds[i],
                row_width,
                rank == shown.len() - 1,
                i,
            );
            anchor = egui::pos2(inner.left() + row_width - 18.0, ui.cursor().min.y);
        }

        confirm_box(ui, anchor, ITEMS[ITEMS.len() - 1].name);
    });
    harness.run();
    write_mockup(&mut harness, "alertes_options_confirmation_retrait");
}

// -------------------------------------------------------------------------------------------
// 4 — Le bandeau de consultation, posé sur le jeu
// -------------------------------------------------------------------------------------------

/// **Le pendant in-game : on consulte, on ne configure pas.**
///
/// La v1 mettait un champ d'ajout, une durée et un mode de fermeture dans un bandeau flottant —
/// c'est-à-dire de la configuration par-dessus le jeu. On configure une fois, on consulte en
/// permanence : ce bandeau ne porte donc que les objets surveillés et leur état sonore, plus un
/// bouton rouage qui ouvre l'onglet Alertes de la fenêtre Options. Le rouage y retrouve exactement
/// le sens que le jeu lui donne (`interface-hdv-achat.png` x≈1196 y≈32).
///
/// Trois corrections par rapport à la v1, toutes issues de la revue :
/// - fond et gouttière **réels** du panneau Suivi (`#1e1e1e` opaque, 12 px) — la v1 annonçait
///   cette source avec un fond translucide et une gouttière deux fois trop serrée ;
/// - **aucune commande sur la tuile** : le jeu ne pose dans le coin d'un emplacement d'objet qu'un
///   marqueur d'ÉTAT, jamais un bouton (21 équipements de `interface-personnage-equiement.png`,
///   où le retrait passe par le glisser-déposer ou un menu). La v1 y mettait deux commandes de
///   16 px séparées de 10, dont une destructive et sans confirmation ;
/// - le marqueur est la **coche du jeu** (`IconTick`, peinte à 16 × 11, la taille mesurée de la
///   coche verte de cette même capture), **plate, sans plaque de fond, débordant le liseré** — et
///   non une case à cocher écrasée de 20 à 16 px sur un rectangle noir.
///
/// Une tuile dont le son est COUPÉ ne porte pas de marqueur : l'absence est le signal, comme
/// l'absence de coche verte sur un équipement non équipé.
#[test]
fn alertes_bandeau_ingame() {
    let mut icons_slot: Option<UiIcons> = None;

    let mut harness = Harness::builder()
        .with_size(Vec2::new(420.0, 320.0))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            let icons = icons_slot.get_or_insert_with(|| UiIcons::load(ui.ctx()));
            egui::Frame::NONE.fill(BACKDROP).show(ui, |ui| {
                ui.set_min_size(ui.available_size());

                let panel = Rect::from_min_size(
                    ui.max_rect().min + Vec2::splat(16.0),
                    Vec2::new(388.0, 288.0),
                );
                ui.painter().rect_filled(panel, 2, PANEL_BG);
                ui.painter().rect_stroke(
                    panel,
                    2,
                    Stroke::new(1.0, BORDER_STRONG),
                    StrokeKind::Inside,
                );

                let inner = panel.shrink(12.0);
                ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
                    ui.set_clip_rect(inner);
                    let width = ui.available_width();

                    // Titre de bandeau : BLANC, pas doré. Le jeu réserve le blanc pur au titre
                    // d'un panneau (« Bibliothèque de sorts », `interface-personnage-sorts.png`,
                    // mesuré `#ffffff`) et son or aux états.
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Alertes")
                                .color(Color32::WHITE)
                                .font(design::text::title_font(ui.ctx(), 18.0)),
                        );
                        ui.label(
                            RichText::new(format!("({})", ITEMS.len()))
                                .color(SUBDUED)
                                .size(14.0),
                        );
                        ui.add_space(width - 190.0);
                        ui.add(
                            design::icon_button(DsTexture::IconOption)
                                .context(IconContext::FirstPlan)
                                .tooltip("Régler les alertes")
                                .log_name("maquette.ouvrir-options"),
                        );
                    });

                    ui.add_space(10.0);

                    let per_row =
                        (((width + TILE_GAP) / (TILE + TILE_GAP)).floor() as usize).max(1);
                    for chunk in ITEMS.chunks(per_row) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = TILE_GAP;
                            for item in chunk {
                                let (rect, response) =
                                    ui.allocate_exact_size(Vec2::splat(TILE), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 2, PANEL_BG);
                                item_slot(ui, icons, rect, item.rarity);

                                if item.sound_on {
                                    // Coche plate, débordant le liseré en bas-gauche — cotes de la
                                    // coche verte du jeu (16 × 11, x 765..780 / y 395..405 sur
                                    // `interface-personnage-equiement.png`), qui déborde elle
                                    // aussi le bord de sa tuile.
                                    design::DesignSystem::get(ui.ctx()).paint(
                                        ui.painter(),
                                        Rect::from_min_size(
                                            egui::pos2(rect.left() - 2.0, rect.bottom() - 11.0),
                                            Vec2::new(16.0, 11.0),
                                        ),
                                        DsTexture::IconTick,
                                        TICK_GREEN,
                                    );
                                }

                                response.on_hover_text(item.name);
                            }
                        });
                        ui.add_space(TILE_GAP);
                    }
                });
            });
        });

    harness.run();
    write_mockup(&mut harness, "alertes_bandeau_ingame");
}
