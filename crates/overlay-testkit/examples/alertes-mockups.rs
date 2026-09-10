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

/// Fond du corps d'une boîte de confirmation — **`#585955`**, mesuré sur
/// `interface-confirm-box.png` (histogramme de x 40..410 / y 60..110 : `#585955` dominant, puis
/// `#595a56` et `#5a5b5c`, tous à un niveau les uns des autres). Un gris **clair**, à l'opposé du
/// kaki `SELECT_LIST_FILL` que la v2 lui donnait.
const CONFIRM_FILL: Color32 = Color32::from_rgb(0x58, 0x59, 0x55);

/// Bord d'une boîte de confirmation — le noir de bord commun au jeu, `tokens::SELECT_LIST_BORDER`.
const CONFIRM_BORDER: Color32 = Color32::from_rgb(0x0E, 0x10, 0x15);

/// Texte d'une boîte de confirmation — blanc, comme tout texte de corps du jeu.
const CONFIRM_TEXT: Color32 = Color32::WHITE;

/// Médaillon « ? » en crête d'une boîte de confirmation — l'or du jeu
/// (`tokens::TAB_LABEL_IDLE`), comme sur `interface-confirm-box.png` où le disque est doré et son
/// point d'interrogation sombre.
const CONFIRM_CREST: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9E);

/// Largeur du corps d'une boîte de confirmation — 420 px, mesurés sur
/// `interface-confirm-box.png` (bords à x=13 et x=433).
const CONFIRM_WIDTH: f32 = 420.0;

/// Hauteur du corps — ~144 px sur la même capture (y ≈ 47..191). Arrondi à 120 ici, la question
/// tenant sur une ligne au lieu de deux : **choix de mise en page, pas une mesure**.
const CONFIRM_HEIGHT: f32 = 120.0;

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

/// Champ de recherche — le champ du design system, avec l'ornement du jeu.
///
/// **`Input::leading_icon` existe désormais** (implémenté avec ces maquettes, jetons mesurés sur
/// `empty-input-search.png`) : la loupe est peinte par le composant, dans son clip, et la
/// gouttière qu'elle impose vaut pour le texte indicatif comme pour la valeur saisie. La v2
/// simulait ce retrait par six espaces de tête, ce qui laissait la valeur saisie démarrer sous la
/// loupe — visible sur sa capture des suggestions, où la loupe couvrait le « p » de « pierre ».
fn search_field(ui: &mut egui::Ui, text: &mut String, placeholder: &str, width: f32) {
    ui.add(
        design::input(text)
            .leading_icon(DsTexture::IconSearch)
            .placeholder(placeholder)
            .size(InputSize::Standard)
            .width(width)
            .log_name("maquette.recherche"),
    );
}

/// Une ligne de la liste d'objets suivis — idiome `interface-options-commandes.png` : emplacement
/// de rareté, nom complet, contrôles alignés en colonne à droite, aucun zébrage.
///
/// Le nom n'est jamais tronqué : c'est tout l'intérêt de la ligne sur la grille, où « Plan "Epée de
/// Bonta" » et « Plan "Epée de Brâkmar" » rendaient la même chaîne.
#[allow(clippy::too_many_arguments)]
fn item_row(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    item: &Item,
    sound_on: &mut bool,
    width: f32,
    hovered: bool,
    index: usize,
    // `removable` : le retrait n'est offert que là où il est protégé par une confirmation, c'est-
    // à-dire la fenêtre Options. Le bandeau in-game ne porte que la bascule du son.
    removable: bool,
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
    if removable && !item.is_default {
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
        // `design::input` est du texte libre — il n'existe pas de champ numérique borné dans le
        // design system, alors que le jeu en a un (`input-number.png`, `button-plus.png`,
        // `button-moins.png`, tous présents dans les assets et aucun au manifeste). En attendant,
        // **la borne se pose au parse** : miroir de `setAlertDuration` côté web
        // (`max(0.5, valeur)`), sans quoi « 0 » ou « abc » validé rendrait le message d'alerte
        // invisible sans le moindre signal — d'autant plus grave que la fenêtre est
        // transactionnelle et se ferme derrière le clic.
        ui.add(
            design::input(seconds)
                .size(InputSize::Standard)
                .width(52.0)
                .enabled(*auto)
                .log_name("maquette.duree"),
        );
        *seconds = clamp_duration(seconds);
        ui.label(RichText::new("sec.").color(SUBDUED).size(15.0));
    });
}

/// Borne la durée saisie — `MIN_ALERT_DURATION_SECONDS` du web (0,5 s). Une saisie non numérique
/// retombe sur le défaut du web (3,5 s) plutôt que d'être effacée sous les doigts.
fn clamp_duration(raw: &str) -> String {
    match raw.replace(',', ".").parse::<f32>() {
        Ok(v) if v >= 0.5 => raw.to_string(),
        Ok(_) => "0.5".to_string(),
        Err(_) if raw.is_empty() => raw.to_string(),
        Err(_) => "3.5".to_string(),
    }
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
///
/// **À la largeur exacte de son champ**, comme toute liste dépliée du jeu : `select-simple.png`
/// (220 × 36) et `select-simple-opened.png` se superposent au pixel, ce qui est précisément la
/// raison d'être de `SELECT_LIST_TOP_LINE` — un liseré qui n'aurait aucun sens si la liste était
/// plus étroite que son socle. La v2 la posait à 62 % de la largeur du champ, une convention web,
/// et les cases « Son » des lignes du dessous réapparaissaient à sa droite : on lisait des
/// suggestions dotées d'une bascule sonore.
///
/// Cadence des entrées : `tokens::SELECT_ROW_HEIGHT` (28), la mesure du select simple qui fait foi
/// — la v2 écrivait 30, qui n'est aucune des deux mesures du jeu.
fn suggestion_list(ui: &egui::Ui, icons: &UiIcons, inner: Rect, field_bottom: f32) {
    let row_h = design::tokens::SELECT_ROW_HEIGHT;
    let list = Rect::from_min_size(
        egui::pos2(inner.left(), field_bottom + 2.0),
        Vec2::new(inner.width(), 4.0 + SUGGESTIONS.len() as f32 * row_h),
    );
    ui.painter()
        .rect_filled(list, 2, design::tokens::SELECT_LIST_FILL);
    // Liseré clair d'un pixel en tête de liste — ce qui la détache de son champ. Jeton mesuré que
    // la v2 déclarait sans l'appliquer.
    ui.painter().hline(
        list.x_range(),
        list.top() + 1.0,
        Stroke::new(1.0, design::tokens::SELECT_LIST_TOP_LINE),
    );
    ui.painter().rect_stroke(
        list,
        2,
        Stroke::new(2.0, design::tokens::SELECT_LIST_BORDER),
        StrokeKind::Inside,
    );
    for (i, (name, rarity, already)) in SUGGESTIONS.iter().enumerate() {
        let row = Rect::from_min_size(
            egui::pos2(list.left() + 2.0, list.top() + 2.0 + i as f32 * row_h),
            Vec2::new(list.width() - 4.0, row_h),
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
                egui::pos2(row.left() + 16.0, row.center().y),
                Vec2::splat(22.0),
            ),
            *rarity,
        );
        ui.painter().text(
            egui::pos2(row.left() + 34.0, row.center().y),
            egui::Align2::LEFT_CENTER,
            *name,
            design::text::label_font(ui.ctx(), 15.0),
            if *already {
                design::tokens::TEXT_DISABLED
            } else {
                design::tokens::SELECT_TEXT
            },
        );
        // « déjà suivi » aligné à DROITE, en second run de texte — la v2 le concaténait au nom
        // dans une seule chaîne séparée par des espaces, la même famille de rustine que les six
        // espaces du champ de recherche.
        if *already {
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

/// La boîte de confirmation du jeu — voir [`alertes_options_confirmation_retrait`].
///
/// **Ce n'est pas une popover ancrée au bouton, contrairement au web** (`ConfirmDeleteService`) et
/// contrairement à la v2 : le jeu a sa propre boîte de confirmation, et elle est dans les captures
/// de référence du dépôt — `interface-confirm-box.png` (449 × 209), qui pose exactement la même
/// question (« Êtes-vous sûr(e) de vouloir supprimer ce build ? »). Mesures relevées dessus :
///
/// | | Mesure |
/// | --- | --- |
/// | Forme | boîte **autonome, centrée** sur la fenêtre parente |
/// | Fond du corps | **`#585955`**, un gris CLAIR — pas le kaki d'une liste déroulante |
/// | Corps | x 13..433 pour une capture de 449, y ≈ 47..191 |
/// | Crête | ornement supérieur débordant du corps (médaillon « ? » et volutes) |
/// | Réponses | **« Non » secondaire, « Oui » PRIMAIRE (or)** |
///
/// **Le bouton destructeur du jeu est or, jamais rouge** : `docs/design-system.md` réserve
/// nommément le rouge au bouton « Annuler » pleine largeur d'un pied de fenêtre, et précise que
/// « le bouton "Annuler" d'une boîte de dialogue simple (`interface-confirm-box.png`, bouton
/// "Non") reste kaki/gris standard, pas rouge ». Un bouton rouge « Retirer » — ce que peignait la
/// v2 — est la convention web du *destructive action*, pas celle du jeu.
///
/// Le centrage règle du même coup la réserve d'ergonomie sur la v2 : sa popover recouvrait le
/// bouton « Valider » de la fenêtre, et son bouton « Retirer » tombait exactement là où « Valider »
/// réapparaissait une fois la popover fermée — un double-clic un peu vif validait la fenêtre.
///
/// La crête est peinte à partir de `decoration-top.png` **non détourée du fond de la capture** :
/// elle n'est pas encore au manifeste. C'est le dernier asset que ce portage demande au skill
/// `design-asset`, avec le haut-parleur.
fn confirm_box(ui: &mut egui::Ui, parent: Rect, item_name: &str) {
    // Voile sombre sur la fenêtre — une boîte modale du jeu assombrit ce qu'elle interrompt.
    ui.painter()
        .rect_filled(parent, 0, Color32::from_black_alpha(0x88));

    let size = Vec2::new(CONFIRM_WIDTH, CONFIRM_HEIGHT);
    let rect = Rect::from_center_size(parent.center(), size);
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    ui.set_clip_rect(Rect::EVERYTHING);
    let ui = &mut ui;

    ui.painter().rect_filled(rect, 4, CONFIRM_FILL);
    ui.painter().rect_stroke(
        rect,
        4,
        Stroke::new(2.0, CONFIRM_BORDER),
        StrokeKind::Inside,
    );

    // Crête : le médaillon « ? » du jeu déborde le haut du corps. Rendu ici par le glyphe d'aide
    // du manifeste sur un disque, faute d'ornement détouré — la forme, pas le décor.
    let crest = egui::pos2(rect.center().x, rect.top());
    ui.painter().circle_filled(crest, 20.0, CONFIRM_CREST);
    ui.painter()
        .circle_stroke(crest, 20.0, Stroke::new(2.0, CONFIRM_BORDER));
    design::DesignSystem::get(ui.ctx()).paint(
        ui.painter(),
        Rect::from_center_size(crest, Vec2::new(14.0, 14.0)),
        DsTexture::IconHelp,
        design::tokens::BUTTON_TEXT_ON_GOLD,
    );

    ui.painter().text(
        egui::pos2(rect.center().x, rect.top() + 46.0),
        egui::Align2::CENTER_CENTER,
        format!("Retirer « {item_name} » de vos alertes ?"),
        design::text::label_font(ui.ctx(), 15.0),
        CONFIRM_TEXT,
    );

    let mut buttons = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(
        egui::pos2(rect.left() + 26.0, rect.top() + 68.0),
        Vec2::new(rect.width() - 52.0, 36.0),
    )));
    buttons.horizontal(|ui| {
        ui.add(
            design::button("Non")
                .variant(ButtonVariant::Secondary)
                .size(ButtonSize::Compact)
                .width(150.0)
                .log_name("maquette.confirm-non"),
        );
        ui.add_space(14.0);
        ui.add(
            design::button("Oui")
                .variant(ButtonVariant::Primary)
                .size(ButtonSize::Compact)
                .width(150.0)
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
/// **Le dossier est vidé une fois par exécution** (`Once`) : sans ça une capture d'un rendu
/// renommé y survit indéfiniment, indiscernable d'une capture vivante — et comme CLAUDE.md oblige
/// à publier ces images en Artifact, publier le dossier reviendrait à livrer une image périmée
/// sans le savoir. C'est arrivé entre la v2 et la v3.
///
/// `CARGO_TARGET_DIR` est respecté : le déduire de `CARGO_MANIFEST_DIR` supposerait la disposition
/// par défaut du workspace.
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

/// Ouvre le harnais et rend la fenêtre Options avec son chrome réel, l'onglet « Alertes » actif.
///
/// **Le décor n'est pas recopié : c'est `options_modal::chrome` qui le peint**, la même fonction
/// que la vraie modale — extraite pour ces maquettes, à pixel constant (le snapshot
/// `options_modale_sur_damier.png` le vérifie à chaque exécution). C'était la réserve la plus
/// lourde de la revue d'architecture : une deuxième implémentation du chrome aurait divergé en
/// quelques semaines.
fn options_harness(
    size: Vec2,
    mut build: impl FnMut(&mut egui::Ui, &UiIcons, Rect, Rect, Rect) + 'static,
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
            let window = ui.max_rect();
            let inner = chrome.inner;
            let scroll = chrome.scroll;
            ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
                // Clip élargi à gauche : un titre de section se peint EN RETRAIT de son axe de
                // contrôles (`PANEL_PAD_CONTROL_X - PANEL_PAD_TITLE_X`, 7 px), et un clip calé sur
                // `inner` lui mangeait sa première lettre — « Alerte » rendait « lerte ».
                ui.set_clip_rect(inner.expand2(egui::vec2(8.0, 0.0)));
                ui.spacing_mut().item_spacing.y = 0.0;
                build(ui, icons, inner, window, scroll);
            });
        });
    })
}

/// Les onze objets dans l'ordre de la capture d'origine.
const ORDER_NATUREL: [usize; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

/// Ce qu'un rendu de l'onglet a besoin de savoir — un `struct` plutôt que neuf paramètres
/// positionnels, dont l'`#[allow(clippy::too_many_arguments)]` de la v2 était le symptôme.
struct AlertsTab<'a> {
    /// Indices dans [`ITEMS`], dans l'ordre d'affichage — vide pour l'état sans objet.
    order: &'a [usize],
    sounds: &'a mut [bool],
    search: &'a mut String,
    auto: &'a mut bool,
    seconds: &'a mut String,
    /// Rang (et non index d'objet) de la ligne survolée.
    hovered_rank: Option<usize>,
    empty_state: EmptyState,
}

/// Pourquoi la liste est vide — **trois situations distinctes, trois messages**.
///
/// La v2 n'en rendait qu'un (« Connectez-vous à votre compte… ») pour les trois, ce qui le rendait
/// **faux dans deux cas sur trois** : dire « connectez-vous » à quelqu'un qui est déjà connecté et
/// attend sa synchronisation ne se contente pas de ne rien apprendre, ça l'envoie chercher un
/// problème de connexion qui n'existe pas.
#[derive(Clone, Copy, PartialEq)]
enum EmptyState {
    /// La liste a des objets — aucun message.
    NotEmpty,
    /// `GET /api/v1/settings` en vol.
    Loading,
    /// Aucun jeton : l'overlay ne peut ni lire ni écrire la liste du compte.
    SignedOut,
    /// Compte connecté, liste réellement vide.
    NoItems,
}

impl EmptyState {
    fn message(self) -> Option<&'static str> {
        match self {
            EmptyState::NotEmpty => None,
            EmptyState::Loading => Some("Récupération de vos alertes…"),
            // Pas de « ajoutez un objet ci-dessus » ici : côté overlay la liste est lue depuis le
            // compte et l'écriture passe par un `PATCH /api/v1/settings` qui **exige un jeton**.
            // Proposer une action qui n'a nulle part où aller est pire que de n'en proposer
            // aucune.
            EmptyState::SignedOut => {
                Some("Connectez-vous à votre compte pour retrouver vos alertes.")
            }
            EmptyState::NoItems => {
                Some("Aucun objet suivi. Ajoutez-en un dans le champ ci-dessus.")
            }
        }
    }
}

/// Le contenu de l'onglet, commun à tous les rendus.
fn alerts_tab(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    inner: Rect,
    scroll: Rect,
    state: &mut AlertsTab<'_>,
) -> f32 {
    let width = inner.width();

    // Le bouton d'aide remplace le bloc d'explication permanent de la v2 — deux gains d'un même
    // geste. Il rend deux lignes d'objets à la liste (le panneau de section est court), et il
    // rétablit l'affordance du web (`?` → `help.profileAlerts`), **le seul endroit qui explique
    // pourquoi dix objets n'ont pas de croix**. Sans elle, un joueur lit dix lignes sans croix et
    // une avec, et ça se lit comme un bug.
    ui.horizontal(|ui| {
        options_modal::section_title(ui, "Alerte sonore");
        ui.add_space(6.0);
        ui.add(
            design::icon_button(DsTexture::IconHelp)
                .context(IconContext::Panel)
                .size(24.0)
                .tooltip(DESC)
                .log_name("maquette.aide"),
        );
    });
    sound_settings(ui, state.auto, state.seconds, width);

    ui.add_space(6.0);
    options_modal::section_title(ui, &format!("Objets suivis ({})", state.order.len()));
    search_field(ui, state.search, "Ajouter un objet à surveiller…", width);
    // Bas du champ : l'ancre du panneau de suggestions, rendue à l'appelant.
    let field_bottom = ui.cursor().min.y;
    ui.add_space(6.0);

    if let Some(message) = state.empty_state.message() {
        ui.add_space(10.0);
        ui.add(
            design::info_text(message)
                .width(width)
                .log_name("maquette.etat-vide"),
        );
        return field_bottom;
    }

    column_header(ui, width);
    // La liste occupe `Chrome::scroll` — `inner` élargi jusqu'au bord du panneau — pour que la
    // poignée tombe à 14 px de ce bord, comme dans le jeu, au lieu de cumuler deux retraits.
    let list_rect = Rect::from_min_max(egui::pos2(scroll.left(), ui.cursor().min.y), scroll.max);
    let mut list_ui = ui.new_child(egui::UiBuilder::new().max_rect(list_rect));
    let ui = &mut list_ui;
    design::scroll_area("maquette.liste")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let row_width = scroll.width() - design::components::scroll_area::RESERVE_X;
            for (rank, &i) in state.order.iter().enumerate() {
                item_row(
                    ui,
                    icons,
                    &ITEMS[i],
                    &mut state.sounds[i],
                    row_width,
                    state.hovered_rank == Some(rank),
                    i,
                    true,
                );
            }
        });
    field_bottom
}

// -------------------------------------------------------------------------------------------
// 1 & 2 — L'onglet « Alertes » de la fenêtre Options, à deux tailles
// -------------------------------------------------------------------------------------------

/// **À la taille ACTUELLE de la fenêtre Options de l'overlay (560 × 436).**
///
/// C'est le constat, pas la proposition : le panneau de section n'offre alors qu'environ 220 px de
/// haut, dont les réglages sonores et les deux titres consomment l'essentiel. **Une seule ligne
/// d'objet reste visible sur onze, et encore, coupée** — la v2 annonçait deux lignes là où sa
/// propre capture n'en montrait qu'une, exactement le défaut qu'elle reprochait à la v1. La liste
/// est bien défilante, rien n'est perdu ; mais consulter onze objets par une fenêtre d'une ligne
/// n'est pas utilisable.
///
/// C'est l'argument, en image, de la maquette suivante.
fn alertes_options_taille_actuelle() {
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = options_harness(
        Vec2::new(560.0, 436.0),
        move |ui, icons, inner, _window, scroll| {
            alerts_tab(
                ui,
                icons,
                inner,
                scroll,
                &mut AlertsTab {
                    order: &ORDER_NATUREL,
                    sounds: &mut sounds,
                    search: &mut search,
                    auto: &mut auto,
                    seconds: &mut seconds,
                    hovered_rank: None,
                    empty_state: EmptyState::NotEmpty,
                },
            );
        },
    );
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
/// **La deuxième ligne est montrée SURVOLÉE, et l'objet retirable figure parmi les lignes
/// visibles** — la v2 posait le survol sur la onzième ligne et la croix sur le onzième objet,
/// tous deux hors du viewport de défilement, qui n'en montre que quatre : la capture censée
/// démontrer le survol, la croix et la protection des défauts n'en montrait aucun. La liste est
/// donc réordonnée ici pour que les trois soient visibles à la fois, comme le fait déjà la
/// maquette de confirmation.
fn alertes_options_taille_jeu() {
    // L'objet ajouté par le joueur remonte en troisième position : c'est le seul qui porte une
    // croix, il doit être dans le champ pour que la capture montre la règle.
    let order: [usize; 11] = [0, 1, 10, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = options_harness(
        Vec2::new(720.0, 561.0),
        move |ui, icons, inner, _window, scroll| {
            alerts_tab(
                ui,
                icons,
                inner,
                scroll,
                &mut AlertsTab {
                    order: &order,
                    sounds: &mut sounds,
                    search: &mut search,
                    auto: &mut auto,
                    seconds: &mut seconds,
                    hovered_rank: Some(1),
                    empty_state: EmptyState::NotEmpty,
                },
            );
        },
    );
    harness.run();
    write_mockup(&mut harness, "alertes_options_taille_jeu");
}

/// **Les trois états sans objet** — sur une seule planche, parce que c'est leur différence qui
/// compte.
///
/// Aucune maquette de la v1 ne les rendait ; la v2 n'en rendait qu'un, dont le message était faux
/// dans les deux autres cas. Trois situations mènent à une liste vide côté overlay, et le web n'en
/// connaît aucune (il réinjecte toujours dix objets par défaut) :
///
/// 1. **la synchronisation est en cours** — `GET /api/v1/settings` en vol ;
/// 2. **aucun compte connecté** — `fetch_settings` exige un jeton, la liste reste vide ;
/// 3. **compte connecté, liste réellement vide** — la clé `profile` n'existe pas encore.
///
/// Le troisième est le seul où proposer un ajout a un sens : l'écriture passe par un
/// `PATCH /api/v1/settings` qui exige lui aussi un jeton.
fn alertes_options_etats_vides() {
    for (state, nom) in [
        (EmptyState::Loading, "chargement"),
        (EmptyState::SignedOut, "deconnecte"),
        (EmptyState::NoItems, "aucun-objet"),
    ] {
        let mut sounds: Vec<bool> = Vec::new();
        let mut search = String::new();
        // 3,5 s : le défaut réel du web (`DEFAULT_ALERT_DURATION_SECONDS`). Les autres maquettes
        // montrent 4, la valeur réglée sur la capture fournie par l'utilisateur.
        let mut seconds = String::from("3.5");
        let mut auto = true;

        let mut harness = options_harness(
            Vec2::new(720.0, 561.0),
            move |ui, icons, inner, _window, scroll| {
                alerts_tab(
                    ui,
                    icons,
                    inner,
                    scroll,
                    &mut AlertsTab {
                        order: &[],
                        sounds: &mut sounds,
                        search: &mut search,
                        auto: &mut auto,
                        seconds: &mut seconds,
                        hovered_rank: None,
                        empty_state: state,
                    },
                );
            },
        );
        harness.run();
        write_mockup(&mut harness, &format!("alertes_options_vide_{nom}"));
    }
}

/// **L'échec de l'enregistrement — le corollaire obligatoire du modèle transactionnel.**
///
/// La fenêtre Options ne prend rien en compte tant que « Valider » n'a pas été cliqué ET jugé
/// valide par l'appelant (§5.1 du plan). Dans ce modèle, un échec de commit qui ne se voit pas est
/// le pire cas : le joueur pose un geste explicite d'engagement, la fenêtre se ferme derrière, et
/// il découvrira la perte à la session suivante sans pouvoir la relier à quoi que ce soit.
///
/// La brique existe déjà et n'a rien à inventer : `OptionsModalState::error` alimente
/// `design::info_text` au ton `Alert`, exactement comme pour un chemin de `wakfu.log` invalide.
fn alertes_options_echec_enregistrement() {
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = options_harness(
        Vec2::new(720.0, 561.0),
        move |ui, icons, inner, _window, scroll| {
            alerts_tab(
                ui,
                icons,
                inner,
                scroll,
                &mut AlertsTab {
                    order: &ORDER_NATUREL[..3],
                    sounds: &mut sounds,
                    search: &mut search,
                    auto: &mut auto,
                    seconds: &mut seconds,
                    hovered_rank: None,
                    empty_state: EmptyState::NotEmpty,
                },
            );
            ui.add_space(8.0);
            ui.add(
            design::info_text(
                "Vos alertes n'ont pas pu être enregistrées sur le compte. Réessayez, ou vérifiez \
                 votre connexion.",
            )
            .tone(design::InfoTone::Alert)
            .width(inner.width())
            .log_name("maquette.echec"),
        );
        },
    );
    harness.run();
    write_mockup(&mut harness, "alertes_options_echec_enregistrement");
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
fn alertes_options_ajout_suggestions() {
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::from("pierre");
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = options_harness(
        Vec2::new(720.0, 561.0),
        move |ui, icons, inner, _w, scroll| {
            let field_bottom = alerts_tab(
                ui,
                icons,
                inner,
                scroll,
                &mut AlertsTab {
                    order: &ORDER_NATUREL,
                    sounds: &mut sounds,
                    search: &mut search,
                    auto: &mut auto,
                    seconds: &mut seconds,
                    hovered_rank: None,
                    empty_state: EmptyState::NotEmpty,
                },
            );
            suggestion_list(&*ui, icons, inner, field_bottom);
        },
    );
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
fn alertes_options_confirmation_retrait() {
    // Les deux premiers défauts, puis l'objet ajouté par le joueur — le seul retirable, et donc
    // le seul que la confirmation puisse viser.
    const SHOWN: [usize; 3] = [0, 1, 10];
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = options_harness(
        Vec2::new(720.0, 561.0),
        move |ui, icons, inner, window, scroll| {
            alerts_tab(
                ui,
                icons,
                inner,
                scroll,
                &mut AlertsTab {
                    order: &SHOWN,
                    sounds: &mut sounds,
                    search: &mut search,
                    auto: &mut auto,
                    seconds: &mut seconds,
                    hovered_rank: Some(2),
                    empty_state: EmptyState::NotEmpty,
                },
            );
            confirm_box(ui, window, ITEMS[10].name);
        },
    );
    harness.run();
    write_mockup(&mut harness, "alertes_options_confirmation_retrait");
}

// -------------------------------------------------------------------------------------------
// 4 — Le bandeau in-game
// -------------------------------------------------------------------------------------------

/// **Le pendant in-game — et il porte la seule action qu'on fait en jouant : couper un son.**
///
/// La v2 l'avait vidé de toute commande, ce qui lui a valu un refus argumenté : réduit à la
/// consultation, il devenait une fenêtre overlay de plus (donc un `OverlayKind`, sur un budget de
/// 300 Mo) affichant une liste **qui ne change jamais** — là où les deux bandeaux existants
/// montrent des compteurs qui bougent (Suivi) ou des dégâts en direct (Combat). Les alertes sont
/// un mécanisme *push* : le toast et le son font le travail, la liste n'a rien à annoncer.
///
/// Ce qui le justifie est donc l'action, pas l'affichage : « cet objet tombe toutes les trente
/// secondes, coupe-le » est un geste qu'on fait **pendant** qu'on joue, et aller ouvrir la fenêtre
/// Options pour ça casse la partie. Un clic, réversible, non destructif.
///
/// **En lignes et non en tuiles**, pour la même raison que l'onglet Options : quatre plans d'épée
/// partagent rareté et icône, une grille sans libellé ne dit pas sur quoi on clique. C'est aussi
/// ce qui fait disparaître le marqueur en coche verte de la v2 — dont le vert déclaré (`#7ac74f`)
/// ne se trouvait pas dans la zone que son jeton citait (mesure : `#509f35`), et dont la boîte
/// de 16 × 11 déformait un glyphe de 12 × 9. Une case à cocher de 20 px dit « son actif » sans
/// ambiguïté, là où une coche verte veut d'abord dire « équipé » dans le vocabulaire du jeu.
///
/// Le retrait, lui, n'est PAS ici : c'est l'action destructrice, elle reste dans la fenêtre
/// Options avec sa confirmation. Le rouage y mène.
fn alertes_bandeau_ingame() {
    let mut icons_slot: Option<UiIcons> = None;
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();

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
                    ui.spacing_mut().item_spacing.y = 0.0;
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
                        // Le rouage à son sens du jeu : ouvrir les réglages — ici l'onglet
                        // « Alertes » de la fenêtre Options.
                        ui.add(
                            design::icon_button(DsTexture::IconOption)
                                .context(IconContext::FirstPlan)
                                .tooltip("Régler les alertes")
                                .log_name("maquette.ouvrir-options"),
                        );
                    });

                    ui.add_space(8.0);
                    design::scroll_area("maquette.bandeau")
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            let row_width = width - design::components::scroll_area::RESERVE_X;
                            for (i, item) in ITEMS.iter().enumerate() {
                                // `removable: false` partout : le retrait vit dans la fenêtre
                                // Options, avec sa confirmation.
                                item_row(
                                    ui,
                                    icons,
                                    item,
                                    &mut sounds[i],
                                    row_width,
                                    i == 1,
                                    i,
                                    false,
                                );
                            }
                        });
                });
            });
        });

    harness.run();
    write_mockup(&mut harness, "alertes_bandeau_ingame");
}

/// Génère les 7 planches dans `target/mockups/` et les liste.
///
/// **Un exemple et non des `#[test]`** — réserve de la revue d'architecture, et elle est juste :
/// ces rendus n'ont aucune assertion, ils ne peuvent que échouer (y compris pour une raison
/// d'environnement pure : `harness.render()` panique sans pilote Vulkan logiciel), et ils
/// ajoutaient ~10 s à chaque `cargo test -p overlay-testkit`, donc à chaque run du job
/// `test-linux` d'un dépôt privé dont les minutes sont comptées. Un artefact jetable ne doit ni
/// rougir la suite, ni coûter des minutes à chaque commit.
///
/// ```text
/// cargo run -p overlay-testkit --example alertes-mockups
/// ```
fn main() {
    alertes_options_taille_actuelle();
    println!("  alertes_options_taille_actuelle");
    alertes_options_taille_jeu();
    println!("  alertes_options_taille_jeu");
    alertes_options_etats_vides();
    println!("  alertes_options_etats_vides");
    alertes_options_echec_enregistrement();
    println!("  alertes_options_echec_enregistrement");
    alertes_options_ajout_suggestions();
    println!("  alertes_options_ajout_suggestions");
    alertes_options_confirmation_retrait();
    println!("  alertes_options_confirmation_retrait");
    alertes_bandeau_ingame();
    println!("  alertes_bandeau_ingame");
    println!("7 planches écrites dans {}", mockup_dir().display());
}
