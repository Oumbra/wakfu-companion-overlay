//! **Maquettes de l'onglet « Suivi »** — le formulaire d'ajout au Suivi et sa liste, portés du
//! dépôt web `Oumbra/wakfu-companion` (`features/tracker`, `features/tracker-strip`,
//! `core/utils/watchlist-tile-controller.ts`) dans le design system du jeu.
//!
//! Demande utilisateur du 2026-09-12, dans la foulée de l'onglet « Alertes » (`alertes-mockups.rs`,
//! porté depuis en `panels::alerts_tab`) : « ajouter un onglet Suivi dans la modale Options […]
//! un peu sur l'exemple de la section Alerte sur la partie objet suivi ». Ce fichier est donc le
//! **frère** de `alertes-mockups.rs` et lui emprunte tout ce qui est déjà tranché — chrome de
//! fenêtre, rythme vertical, champ d'autocomplétion, boîte de confirmation. Ce qu'il ajoute est
//! listé dans « Ce que ce portage demande » plus bas.
//!
//! ## Les cinq règles que ces maquettes posent
//!
//! 1. **La liste n'affiche QUE l'emplacement d'objet** — pas de nom sous la tuile, contrairement à
//!    l'onglet Alertes. Demande explicite : « le rendu sera différent, on n'affiche que les item
//!    slots ». C'est exactement ce que fait déjà le bandeau Suivi in-game (`panels::watchlist`),
//!    dont ces tuiles reprennent la géométrie ([`design::tokens::ITEM_SLOT_SIZE`], 64 px) et le
//!    compteur peint dans le coin bas-droit ([`design::SlotCount`]) — un suivi se reconnaît à son
//!    icône et à son compteur, le nom vit dans l'infobulle.
//! 2. **Le formulaire d'ajout porte le MODE avant le nom.** Incrémental ou décompte, et en
//!    décompte la quantité de départ : ces deux réglages sont **figés à la création** côté web
//!    (`WatchlistTileController.add`, et `tracker.countdownTargetLocked` : « Cible fixée à la
//!    création »). Les demander après coup serait mentir sur ce que la tuile permet.
//! 3. **Le retrait se fait à la tuile, au survol** — une croix apparaît sur la tuile survolée, et
//!    rien ne dépasse tant que la souris est ailleurs. C'est le `.kpi-card-del` du web, et ça tient
//!    dans 64 px là où un nom n'y tiendrait pas.
//! 4. **La sélection multiple est un MODE**, pas une case permanente : un bouton l'ouvre, chaque
//!    tuile gagne alors sa case à cocher **à la place** de sa croix, et un bouton de suppression
//!    groupée apparaît. Sélection vide = « Supprimer tout » (aucune exclusion cochée) — la règle du
//!    web, voir [`bulk_label`].
//! 5. **Rien n'est écrit avant « Valider »**, comme tout le reste de la fenêtre (§5.1 du plan) :
//!    ajouts, retraits et suppressions groupées vivent dans un brouillon, et « Annuler » le jette
//!    en entier. C'est ce que la demande appelle « le bouton annuler n'enregistrera absolument
//!    aucune des modifications ».
//!
//! ## Ce que ce portage demande, et qui n'existe pas encore
//!
//! Une maquette sert à chiffrer un portage. Par ordre de nécessité :
//!
//! 1. **`CatalogIndex::search_monsters`** — le champ d'ajout du Suivi cherche « objets ET
//!    monstres » (`domain="both"` côté web), or l'index ne sait chercher que les objets
//!    ([`CatalogIndex::search_items`], écrit pour les alertes). Les monstres sont déjà indexés
//!    (`monsters_by_id`/`monsters_by_name`) mais sans la liste triée par nom d'affichage que la
//!    recherche exige. **Le plus gros morceau du lot, et le seul sans lequel rien ne marche.**
//! 2. **`Stepper` au clavier (↑/↓)** — demande explicite (« flèche du haut, flèche du bas qui
//!    pourront augmenter la valeur »). [`design::stepper`] n'a aujourd'hui que ses deux boutons ;
//!    le web pilote les quatre flèches (`app-input-number`). À ajouter **au composant**, pas ici.
//! 3. **Un bouton d'action par entrée d'autocomplétion** — l'icône « recette »
//!    (`tracker.recipeTooltip`) vit **dans la rangée de suggestion**, à droite du nom
//!    (`.wakfu-autocomplete-recipe-btn`). `design::AutocompleteEntry` n'a pas de place pour ça :
//!    il faut un `action: Option<DsIcon>` et un `AutocompleteOutcome::action_on: Option<usize>`.
//! 4. **La résolution des recettes** — l'index compact ne porte que le drapeau `hasRecipe`
//!    ([`CatalogIndex::find_item_has_recipe`]), jamais les ingrédients. Le web les résout
//!    récursivement par `GET /api/v1/items/{id}` (`CatalogService.resolveRecipeIngredients`, un
//!    aller-retour par niveau) ; l'overlay a besoin du même appel dans `overlay_sync::client`, avec
//!    la même protection anti-cycle par ancêtres.
//! 5. **L'ÉCRITURE de la liste suivie** — `overlay_engine::watchlist` est aujourd'hui *lecture
//!    seule* sur les définitions : « la LISTE des entrées suivies reste éditée sur le web et lue en
//!    lecture seule ici ». Le transport existe déjà (`client::patch_watchlist`, écrit pour
//!    répliquer les compteurs) ; ce qui manque est le brouillon et la frontière brouillon → compte
//!    au moment du « Valider ».
//! 6. **Un quatrième onglet** — `OptionsTab` n'a que `Alertes`/`Personnages`/`Parametres`. Ces
//!    maquettes rendent leur propre énumération ([`MockTab`]) pour ne rien changer au code de
//!    production avant que la mise en page soit validée.
//!
//! ## Deux points relevés dans le code web, à trancher avec l'utilisateur
//!
//! - **Les badges de quantité ADDITIONNENT, ils ne remplacent pas.** La doc de
//!   `setAddTargetPreset` dit « remplace intégralement la cible », le code fait
//!   `addTarget.set(addTarget() + value * (event.altKey ? -1 : 1))` — un clic sur « 100 » ajoute
//!   cent, Alt+clic en retire cent. Les maquettes suivent le **code**, pas la doc, et l'écrivent
//!   dans l'infobulle des badges. À confirmer : c'est peut-être la doc qui dit l'intention.
//! - **Le retrait individuel est confirmé, le retrait groupé ne l'est pas** (web :
//!   `ConfirmDeleteService` d'un côté, `confirmBulkDelete` immédiat de l'autre, « le passage par ce
//!   mode dédié + une sélection explicite tient lieu de confirmation »). Ces maquettes gardent
//!   cette asymétrie — voir [`suivi_confirmation_retrait`] — mais dans un écran transactionnel où
//!   « Annuler » rattrape tout, on pourrait aussi bien n'en confirmer aucun.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.
//!
//! ```text
//! cargo run -p overlay-testkit --example suivi-mockups
//! ```

#[path = "shared/wakassets_fixtures.rs"]
mod wakassets_fixtures;

use egui::{Color32, Rect, RichText, Stroke, StrokeKind, Vec2};
use egui_kittest::Harness;
use overlay_engine::WakfuRarity;
use overlay_ui::design::{self, ButtonSize, ButtonVariant, DsIcon, IconContext, SlotFrame};
use overlay_ui::rarity_bridge::to_slot_rarity;
use overlay_ui::ui_icons::UiIcons;

use wakassets_fixtures::{CategoryFilter, CategoryIcons, RarityGems, GEM_NATIVE};

// -------------------------------------------------------------------------------------------
// Jetons
//
// Chacun dit d'où il vient : mesure sur une capture du jeu, reprise TELLE QUELLE d'un panneau
// existant, ou choix assumé. Les jetons repris de `alertes-mockups.rs` gardent leur valeur et leur
// provenance — un jeton « repris » qui change de valeur en route est le pire cas, il annonce une
// mesure là où il n'y en a plus (défaut réel de la v1 des maquettes d'alertes).
// -------------------------------------------------------------------------------------------

/// Fond derrière la fenêtre — le sombre que `tests/panels.rs` pose sous la modale Options, pour
/// qu'un bord translucide se voie.
const BACKDROP: Color32 = Color32::from_rgb(0x0B, 0x0D, 0x10);

/// Taille de la fenêtre Options dans ces maquettes — **celle des maquettes d'alertes**, à
/// l'identique : les deux onglets vivent dans la même fenêtre, une planche à une autre taille ne
/// se comparerait pas.
const WINDOW: Vec2 = Vec2::new(760.0, 810.0);

/// Texte courant — blanc, comme tout texte de corps du jeu (`panels::alerts_tab::TEXT`).
const TEXT: Color32 = Color32::WHITE;

/// Gris des libellés secondaires et des unités — `#b8b9ba`, le gris unique du jeu (mesure
/// convergente sur quatre captures, voir `panels::options_modal::SECTION_TITLE_TEXT`).
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);

/// Fond d'une ligne de réglage mise en valeur — l'idiome des lignes d'aptitude du jeu
/// (`interface-personnage-aptitudes.png`, `#26282b` sur un fond de section plus sombre). Repris de
/// `panels::alerts_tab::SETTING_ROW_FILL`.
const SETTING_ROW_FILL: Color32 = Color32::from_rgb(0x26, 0x28, 0x2B);
const SETTING_ROW_RADIUS: u8 = 4;
/// Hauteur d'une ligne de formulaire — 40, comme la ligne « Fermeture automatique » de l'onglet
/// Alertes : les lignes d'aptitude du jeu cadencent à 32, portées à 40 dès qu'un contrôle de 26 à
/// 32 px y vit.
const FORM_ROW_HEIGHT: f32 = 40.0;
/// Rembourrage latéral dans une ligne de formulaire — repris tel quel de `close_settings_row`.
const FORM_ROW_PAD_X: f32 = 12.0;
/// Écart vertical entre les deux lignes du bloc de formulaire.
const FORM_ROW_GAP: f32 = 6.0;

/// Corps du texte courant — celui des libellés de ligne du panneau (`alerts_tab::BODY_FONT_SIZE`).
const BODY_FONT_SIZE: f32 = 15.0;

/// Aération autour d'un titre de section — 18 px, la valeur arrêtée pour l'onglet Alertes après
/// deux retours utilisateur.
const SECTION_GAP: f32 = 18.0;

/// Côté d'une tuile de la liste — **l'emplacement d'objet du jeu**
/// ([`design::tokens::ITEM_SLOT_SIZE`], 64 px), c'est-à-dire exactement la tuile du bandeau Suivi
/// in-game (`panels::watchlist::TILE_SIZE`). La liste n'affiche rien d'autre, donc la tuile n'a
/// aucune raison d'être plus petite qu'en jeu.
const TILE: f32 = design::tokens::ITEM_SLOT_SIZE;

/// Gouttière entre deux tuiles — **12, celle du bandeau Suivi** (`panels::watchlist::TILE_GAP`).
/// Les 10 px de l'onglet Alertes valaient pour des tuiles qui portaient leur nom ; ici la grille
/// est faite des mêmes tuiles qu'en jeu, elle en garde le rythme.
const TILE_GAP: f32 = 12.0;

/// Côté d'un badge de coin sur une tuile (croix de retrait) — repris de
/// `panels::alerts_tab::TILE_BADGE`.
const BADGE: f32 = 14.0;
/// Retrait d'un badge depuis le coin de la tuile.
const BADGE_INSET: f32 = 4.0;

/// Voile posé sur une tuile SURVOLÉE, sous sa croix de retrait.
///
/// **Deux rôles en un seul aplat** : il dit que la tuile est survolée (le web le dit par un
/// `transform: scale`, hors de portée d'un emplacement peint en 9-slice), et il donne à la croix un
/// fond assez sombre pour rester lisible par-dessus n'importe quelle bordure de rareté. Sans lui,
/// la croix se perd sur les raretés claires (or, mémoire) — l'onglet Alertes n'avait pas ce
/// problème, sa croix tombant sur le fond plat de la tuile, jamais sur la bordure.
const TILE_HOVER_SCRIM: Color32 = Color32::from_black_alpha(0x66);

/// Liseré d'une tuile COCHÉE en sélection multiple — l'or du jeu, la couleur que ce design system
/// réserve aux **états** (case cochée, onglet inactif, valeur saisie). Une tuile cochée est un
/// état, pas une hiérarchie.
const TILE_SELECTED: Color32 = design::tokens::TEXT_GOLD;
const TILE_SELECTED_WIDTH: f32 = 2.0;

/// Les quantités toutes faites du web (`WatchlistTileController.targetPresets`) — jamais une borne,
/// seulement une aide de saisie.
const TARGET_PRESETS: [i64; 5] = [10, 50, 100, 500, 1000];

/// Domaine de la quantité de départ — `[min]="1" [max]="9999"` du web (`tracker.component.html`).
const TARGET_RANGE: std::ops::RangeInclusive<i64> = 1..=9999;

/// La phrase sous le titre — dérivée de `tracker.empty`/`tracker.searchPlaceholder` du dépôt web,
/// qui n'a pas de description de section pour le Suivi (son en-tête tient en un mot).
const DESC: &str = "Les objets et monstres suivis comptent automatiquement ce que vous ramassez \
                    et ce que vous vaincez. Le bandeau de suivi les affiche par-dessus le jeu.";

// -------------------------------------------------------------------------------------------
// Données de démonstration
// -------------------------------------------------------------------------------------------

/// Ce qui distingue une tuile d'objet d'une tuile de monstre.
///
/// **Un monstre n'a pas de rareté** : sa tuile prend le cadre neutre ([`SlotFrame::Plain`]), comme
/// dans le bandeau in-game où « les tuiles ENNEMI gardent le fond plat, bordure grise unie ».
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Item(WakfuRarity),
    Monster,
}

struct Tracked {
    name: &'static str,
    kind: Kind,
    /// Ce que le compteur affiche — en décompte, ce qu'il RESTE à trouver.
    count: i64,
    /// Renseignée pour un décompte, `None` pour un incrémental.
    target: Option<i64>,
}

impl Tracked {
    const fn up(name: &'static str, kind: Kind, count: i64) -> Self {
        Self {
            name,
            kind,
            count,
            target: None,
        }
    }
    const fn down(name: &'static str, kind: Kind, count: i64, target: i64) -> Self {
        Self {
            name,
            kind,
            count,
            target: Some(target),
        }
    }
}

/// Quatorze suivis, objets et monstres mêlés — assez pour remplir deux rangées et montrer les deux
/// modes de compteur côte à côte, ce que la grille doit rendre lisible d'un coup d'œil.
///
/// **Les raretés sont une reconstitution**, comme dans les maquettes d'alertes : deux entrées sont
/// poussées vers des raretés extrêmes pour que la planche montre l'écart entre bordures plutôt
/// qu'un camaïeu.
const TRACKED: &[Tracked] = &[
    Tracked::down("Bois de Frêne", Kind::Item(WakfuRarity::Common), 340, 1000),
    Tracked::up("Fleur de Kalé", Kind::Item(WakfuRarity::Rare), 62),
    Tracked::down("Pierre de Lune", Kind::Item(WakfuRarity::Mythical), 7, 50),
    Tracked::up("Bouftou", Kind::Monster, 128),
    Tracked::up("Larve Bleue", Kind::Monster, 41),
    Tracked::down("Plume de Tofu", Kind::Item(WakfuRarity::Common), 18, 100),
    Tracked::up("Chafer Élite", Kind::Monster, 9),
    Tracked::down(
        "Griffe de Craqueleur",
        Kind::Item(WakfuRarity::Legendary),
        2,
        10,
    ),
    Tracked::up("Pierre ultime", Kind::Item(WakfuRarity::Memory), 3),
    Tracked::down("Dragodinde Rousse", Kind::Monster, 5, 20),
    Tracked::up("Minerai de Fer", Kind::Item(WakfuRarity::Common), 806),
    Tracked::up("Écaille de Wabbit", Kind::Item(WakfuRarity::Epic), 14),
    Tracked::down("Tofu Royal", Kind::Monster, 1, 3),
    Tracked::up("Bois de Bombu", Kind::Item(WakfuRarity::Relic), 27),
];

/// Ce que le panneau de suggestions affiche pour la saisie « tofu ».
///
/// **Objets ET monstres dans la même liste**, exactement comme le champ `domain="both"` du web :
/// c'est le référentiel qui dit ce qu'est chaque entrée, l'utilisateur n'a aucun sélecteur de type
/// à actionner avant de chercher. Le dernier champ dit si l'entrée a une recette (bouton
/// d'ingrédients dans la rangée) ; une entrée déjà suivie est grisée.
const SUGGESTIONS: &[Suggestion] = &[
    Suggestion {
        name: "Plume de Tofu",
        rarity: Some(WakfuRarity::Common),
        filter: CategoryFilter::Resources,
        already: true,
        recipe: false,
    },
    Suggestion {
        name: "Tofu",
        rarity: None,
        filter: CategoryFilter::Enemy,
        already: false,
        recipe: false,
    },
    Suggestion {
        name: "Tofu Royal",
        rarity: None,
        filter: CategoryFilter::Enemy,
        already: true,
        recipe: false,
    },
    Suggestion {
        name: "Coiffe du Tofu",
        rarity: Some(WakfuRarity::Rare),
        filter: CategoryFilter::Equipment,
        already: false,
        recipe: true,
    },
    Suggestion {
        name: "Cape du Tofu",
        rarity: Some(WakfuRarity::Rare),
        filter: CategoryFilter::Equipment,
        already: false,
        recipe: true,
    },
    Suggestion {
        name: "Tofukaz",
        rarity: None,
        filter: CategoryFilter::Enemy,
        already: false,
        recipe: false,
    },
];

struct Suggestion {
    name: &'static str,
    /// `None` pour un monstre — pas de gemme de rareté dans sa rangée.
    rarity: Option<WakfuRarity>,
    filter: CategoryFilter,
    already: bool,
    recipe: bool,
}

/// Les ingrédients de « Coiffe du Tofu », tels que `resolveRecipeIngredients` les rendrait.
///
/// Le troisième a lui-même une recette : c'est ce qui justifie le bouton d'imbrication de chaque
/// ligne (`tracker.recipeNestTooltip`, « Suivre les ingrédients de cet objet plutôt que l'objet
/// lui-même »), et la maquette le montre déplié.
const INGREDIENTS: &[(&str, WakfuRarity, i64, bool)] = &[
    ("Plume de Tofu", WakfuRarity::Common, 12, false),
    ("Bec de Tofu", WakfuRarity::Common, 4, false),
    ("Cuir Bouilli", WakfuRarity::Rare, 2, true),
];

/// Les ingrédients de « Cuir Bouilli », affichés quand sa ligne est dépliée — leurs quantités sont
/// **multipliées par celle du parent**, comme le fait le web (`multiplier: ing.quantity *
/// multiplier`).
const SUB_INGREDIENTS: &[(&str, WakfuRarity, i64)] = &[
    ("Peau de Bouftou", WakfuRarity::Common, 5),
    ("Sel", WakfuRarity::Common, 2),
];

// -------------------------------------------------------------------------------------------
// État d'un rendu
// -------------------------------------------------------------------------------------------

/// Le mode du futur compteur, choisi AVANT le nom — voir la règle 2 de la doc de module.
#[derive(Clone, Copy, PartialEq)]
enum AddMode {
    Up,
    Down,
}

/// L'onglet rendu par la barre de ces maquettes.
///
/// Une énumération **locale** : `OptionsTab` n'a pas d'entrée « Suivi », et lui en ajouter une
/// avant que la mise en page soit validée changerait le code de production pour une maquette.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MockTab {
    Suivi,
    Alertes,
    Personnages,
    Parametres,
}

/// Ce qu'un rendu de l'onglet a besoin de savoir.
struct SuiviTab<'a> {
    mode: AddMode,
    /// Quantité de départ d'un décompte — n'a de sens qu'en [`AddMode::Down`].
    target: &'a mut i64,
    search: &'a mut String,
    /// Indice de la tuile survolée, le harnais offscreen n'ayant pas de souris.
    hovered: Option<usize>,
    /// Mode « sélection multiple » ouvert, et les tuiles cochées.
    select_mode: bool,
    selected: &'a [usize],
    availability: Availability,
    /// Le panneau de suggestions, déplié — `None` quand le champ est au repos.
    suggestions: Option<&'a SuggestionPreview>,
}

/// Dans quel état l'écran se trouve vis-à-vis du compte — **les trois mêmes cas que l'onglet
/// Alertes** (`panels::alerts_tab::AlertsAvailability`), et pour la même raison : sans compte lié,
/// la liste suivie n'a ni source ni destination.
#[derive(Clone, Copy, PartialEq)]
enum Availability {
    Ready,
    Loading,
    NoAccount,
}

// -------------------------------------------------------------------------------------------
// Le contenu de l'onglet
// -------------------------------------------------------------------------------------------

fn suivi_tab(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    panel: &design::PanelZones,
    state: &mut SuiviTab<'_>,
) {
    let width = panel.inner.width();

    ui.add(design::heading("Suivi").trailing_gap(SECTION_GAP * 0.5));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP);

    add_form(ui, state, width);
    ui.add_space(SECTION_GAP * 0.75);
    add_field(ui, icons, state, width);
    ui.add_space(SECTION_GAP);

    list_header(ui, state, width);

    match state.availability {
        Availability::Loading => {
            loading_row(ui, panel.inner);
            return;
        }
        Availability::NoAccount => {
            ui.add(
                design::info_text(
                    "Aucun compte lié : le suivi se règle sur votre compte Wakfu Companion. \
                     Connectez-vous depuis le bandeau de suivi pour le retrouver ici.",
                )
                .width(width)
                .log_name("suivi.sans-compte"),
            );
            return;
        }
        Availability::Ready => {}
    }

    tile_grid(ui, icons, panel, state);
}

fn paragraph(ui: &mut egui::Ui, text: &str) {
    // **Un paragraphe, pas un bloc d'information** : `design::info_text` porte une pastille et le
    // poids d'une remarque, ce qui donnerait à cette phrase une importance qu'elle n'a pas (même
    // arbitrage que `panels::alerts_tab::paragraph`).
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .color(TEXT)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        )
        .wrap_mode(egui::TextWrapMode::Wrap),
    );
}

/// **Le bloc de formulaire** : le mode, et en décompte la quantité de départ.
///
/// Les deux lignes vivent sur le même aplat que la ligne « Fermeture automatique » de l'onglet
/// Alertes — l'idiome du jeu pour « un libellé à gauche, des contrôles à droite ». La seconde
/// n'apparaît qu'en décompte : en incrémental le compteur part de zéro et monte, il n'y a aucune
/// quantité à demander, et laisser la ligne grisée occuperait la place d'un réglage qui n'existe
/// pas (c'est l'arbitrage inverse du champ « durée » des alertes, qui reste affiché parce qu'il
/// garde une valeur entre deux bascules).
fn add_form(ui: &mut egui::Ui, state: &mut SuiviTab<'_>, width: f32) {
    let rows = if state.mode == AddMode::Down { 2 } else { 1 };
    let height = FORM_ROW_HEIGHT * rows as f32 + FORM_ROW_GAP * (rows - 1) as f32;
    let block = ui.allocate_space(Vec2::new(width, height)).1;
    ui.painter()
        .rect_filled(block, SETTING_ROW_RADIUS, SETTING_ROW_FILL);

    let mode_row = Rect::from_min_size(block.min, Vec2::new(width, FORM_ROW_HEIGHT));
    mode_line(ui, state, mode_row);

    if state.mode == AddMode::Down {
        let target_row = Rect::from_min_size(
            egui::pos2(block.left(), mode_row.bottom() + FORM_ROW_GAP),
            Vec2::new(width, FORM_ROW_HEIGHT),
        );
        target_line(ui, state, target_row);
    }
}

/// La ligne « Type de compteur » — **deux boutons segmentés**, l'actif en or.
///
/// **Choix assumé, faute d'idiome relevé.** Le web emploie un interrupteur à deux positions avec
/// fond glissant (`.icon-switch`, `tracker.component.html`) ; le jeu n'en a aucun — son vocabulaire
/// pour un choix binaire est la case à cocher, et pour un choix exclusif entre deux actions
/// nommées, le bouton. Deux cases à cocher mutuellement exclusives seraient un contresens
/// (une case dit « oui/non », pas « l'un ou l'autre »), et une liste déroulante à deux entrées
/// cacherait la moitié du choix derrière un clic. Restent deux boutons, dont l'actif prend la
/// variante **primaire** — l'or que ce design system réserve aux états.
///
/// Alternative à trancher avec l'utilisateur : la même paire rendue par [`design::select`] à deux
/// options, plus compacte mais qui masque l'option non retenue.
fn mode_line(ui: &mut egui::Ui, state: &mut SuiviTab<'_>, row: Rect) {
    let mut cell =
        ui.new_child(egui::UiBuilder::new().max_rect(row.shrink2(Vec2::new(FORM_ROW_PAD_X, 0.0))));
    cell.horizontal_centered(|ui| {
        ui.label(
            RichText::new("Type de compteur")
                .color(TEXT)
                .size(BODY_FONT_SIZE),
        );
        ui.add_space(12.0);
        for (mode, label, tooltip) in [
            (
                AddMode::Up,
                "Incrémental",
                "Le compteur part de zéro et monte à chaque trouvaille.",
            ),
            (
                AddMode::Down,
                "Décompte",
                "Le compteur part de la quantité choisie et descend vers zéro.",
            ),
        ] {
            let actif = state.mode == mode;
            ui.add(
                design::button(label)
                    .variant(if actif {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Secondary
                    })
                    .size(ButtonSize::Height(28.0))
                    .min_width(104.0)
                    .tooltip(tooltip)
                    .log_name(format!("suivi.mode-{label}")),
            );
            ui.add_space(6.0);
        }
    });
}

/// La ligne « Quantité » — les badges du web, puis le pas numérique.
///
/// **Les badges ajoutent, le pas ajuste.** C'est ce que fait `setAddTargetPreset` (voir la doc de
/// module) : un clic sur « 100 » ajoute cent à la valeur en cours, Alt+clic en retire cent — d'où
/// l'infobulle qui le dit, parce que rien dans un badge « 100 » ne laisse deviner s'il pose ou
/// ajoute. Le pas, lui, est [`design::stepper`] : ses deux boutons et son champ saisissable, plus
/// les flèches ↑/↓ qui restent à lui ajouter (point 2 de la doc de module).
fn target_line(ui: &mut egui::Ui, state: &mut SuiviTab<'_>, row: Rect) {
    let mut cell =
        ui.new_child(egui::UiBuilder::new().max_rect(row.shrink2(Vec2::new(FORM_ROW_PAD_X, 0.0))));
    cell.horizontal_centered(|ui| {
        ui.label(RichText::new("Quantité").color(TEXT).size(BODY_FONT_SIZE));
        ui.add_space(12.0);
        for preset in TARGET_PRESETS {
            ui.add(
                design::button(preset.to_string())
                    .variant(ButtonVariant::Secondary)
                    .size(ButtonSize::Height(24.0))
                    .min_width(46.0)
                    .tooltip(format!(
                        "Ajouter {preset} à la quantité — Alt+clic pour en retirer autant"
                    ))
                    .log_name(format!("suivi.quantite-{preset}")),
            );
            ui.add_space(4.0);
        }
        // Le pas est poussé à DROITE de la ligne : c'est la valeur qui compte, les badges ne sont
        // qu'un raccourci vers elle. Les coller ensemble laisserait croire à un sixième badge.
        let pas = design::stepper(state.target)
            .range(TARGET_RANGE)
            .size(28.0)
            .field_width(62.0)
            .log_name("suivi.quantite");
        let (largeur, _) = pas.desired_size();
        let reste = ui.available_width() - largeur.unwrap_or(0.0);
        ui.add_space(reste.max(8.0));
        ui.add(pas);
    });
}

/// Le champ d'ajout et son panneau de suggestions — **[`design::autocomplete`]**, le composant
/// écrit pour les alertes, ici en domaine « objets ET monstres ».
///
/// Ce que la maquette fournit reste du CONTENU : les entrées, leurs gemmes, leurs images, et la
/// bande de filtres. Deux différences avec l'onglet Alertes, toutes deux dans les données :
///
/// - la bande porte **un filtre « Monstres » de plus** (`CategoryFilter::Enemy`, qui n'existe côté
///   web qu'en domaine `both`) ;
/// - une rangée de monstre **n'a pas de gemme** — un monstre n'a pas de rareté.
fn add_field(ui: &mut egui::Ui, _icons: &UiIcons, state: &mut SuiviTab<'_>, width: f32) {
    let enabled = state.availability == Availability::Ready;
    let vide: Vec<design::AutocompleteEntry> = Vec::new();
    let sans_filtre: Vec<design::AutocompleteFilter> = Vec::new();
    let (entries, filters) = match state.suggestions {
        Some(preview) => (&preview.entries, &preview.filters),
        None => (&vide, &sans_filtre),
    };

    let mut champ = design::autocomplete(state.search)
        .placeholder("Ajouter un objet ou un monstre à suivre…")
        .width(width)
        .enabled(enabled)
        .entries(entries)
        .filters(filters)
        .log_name("suivi.ajout");
    if let Some(preview) = state.suggestions {
        champ = champ.preview_open(true).preview_active(preview.active);
    }
    let outcome = champ.show(ui);

    if state.suggestions.is_some() {
        recipe_buttons_overlay(ui, outcome.response.rect);
    }
}

/// **Les boutons « recette », peints PAR-DESSUS le panneau — et c'est précisément le point 3 de la
/// doc de module.**
///
/// `design::AutocompleteEntry` n'a aucune place pour une action de rangée : la maquette calcule donc
/// la géométrie du panneau (les jetons `AUTOCOMPLETE_*` la donnent exactement) et pose l'icône
/// là où elle ira. **Ce code n'est pas portable tel quel** : le portage ajoute
/// `AutocompleteEntry::action` et `AutocompleteOutcome::action_on`, et cette fonction disparaît.
/// Elle n'existe que pour que la planche montre le geste au lieu d'en parler.
fn recipe_buttons_overlay(ui: &mut egui::Ui, field: Rect) {
    // **Au-dessus du panneau, pas dedans** : `design::autocomplete` peint le sien dans une couche
    // d'avant-plan, et une peinture faite dans le `Ui` courant passerait dessous sans rien montrer
    // (constaté sur le premier rendu de cette planche).
    let mut couche = ui.new_child(egui::UiBuilder::new().max_rect(ui.max_rect()).layer_id(
        egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("suivi-recette-overlay")),
    ));
    couche.set_clip_rect(Rect::EVERYTHING);
    let ui = &mut couche;
    let pad = design::tokens::AUTOCOMPLETE_PANEL_PAD;
    let premiere_ligne = field.bottom()
        + design::tokens::AUTOCOMPLETE_PANEL_GAP
        + pad
        + design::tokens::AUTOCOMPLETE_FILTER_BAR_HEIGHT;
    let ds = design::DesignSystem::get(ui.ctx());
    for (rang, s) in SUGGESTIONS.iter().enumerate() {
        if !s.recipe || rang >= design::tokens::AUTOCOMPLETE_MAX_VISIBLE_ROWS {
            continue;
        }
        let haut = premiere_ligne + rang as f32 * design::tokens::AUTOCOMPLETE_ROW_HEIGHT;
        let centre = egui::pos2(
            field.right() - pad - design::tokens::AUTOCOMPLETE_ROW_PADDING_X - 11.0,
            haut + design::tokens::AUTOCOMPLETE_ROW_HEIGHT / 2.0,
        );
        let boite = Rect::from_center_size(centre, Vec2::splat(22.0));
        ui.painter()
            .rect_filled(boite, 4, Color32::from_black_alpha(0x55));
        ds.paint_icon(
            ui.painter(),
            Rect::from_center_size(
                centre,
                design::components::icon_button::glyph_fit(
                    ds.icon_native_size(DsIcon::Hammer),
                    14.0,
                ),
            ),
            DsIcon::Hammer,
            design::tokens::TEXT_GOLD,
        );
    }
}

/// L'en-tête de la liste : son titre à gauche, ses commandes à droite.
///
/// **La sélection multiple est un mode, et son bouton le dit** : un bouton icône « corbeille » qui
/// reste enfoncé tant que le mode est ouvert, et à côté de lui le bouton de suppression groupée —
/// jamais l'inverse (un bouton « Supprimer » visible en permanence inviterait au geste avant toute
/// sélection). C'est la disposition du web (`.kpi-remove-wrap`), transposée aux composants du jeu.
fn list_header(ui: &mut egui::Ui, state: &SuiviTab<'_>, width: f32) {
    let row = ui.allocate_space(Vec2::new(width, 34.0)).1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    cell.horizontal_centered(|ui| {
        ui.add(design::heading("Éléments suivis"));
        let mut droite = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        // **Rien à commander quand il n'y a rien à lister** : sans compte lié, ou pendant que la
        // liste descend, le bouton de sélection multiple disparaît au lieu de rester grisé. Un
        // bouton grisé annonce un geste momentanément indisponible ; ici le geste n'a pas d'objet.
        if state.availability != Availability::Ready {
            return;
        }
        let icon_state = if state.select_mode {
            // Le mode ouvert se lit sur le bouton lui-même : il reste à l'état survolé, comme un
            // onglet actif. C'est ce que fait le web avec sa classe `.active`.
            Some(design::IconButtonState::Hovered)
        } else {
            None
        };
        let mut bouton = design::icon_button(DsIcon::Delete)
            .context(IconContext::Panel)
            .tooltip(if state.select_mode {
                "Quitter la sélection"
            } else {
                "Suppression multiple"
            })
            .log_name("suivi.selection");
        if let Some(forced) = icon_state {
            bouton = bouton.preview_state(forced);
        }
        droite.add(bouton);
        if state.select_mode {
            droite.add_space(8.0);
            droite.add(
                // **Or, jamais rouge.** Règle établie par l'onglet Alertes et tenue ici : le rouge
                // de ce design system est celui du « Annuler » pleine largeur d'un pied de fenêtre,
                // et lui seul. Une action destructive se signale par ce qu'elle dit et par la
                // confirmation qu'elle demande, pas par une couleur que le jeu emploie ailleurs.
                design::button(bulk_label(state.selected.len(), TRACKED.len()))
                    .variant(ButtonVariant::Primary)
                    .size(ButtonSize::Height(28.0))
                    .min_width(150.0)
                    .tooltip(
                        "Retire les tuiles cochées du suivi — annulable tant que la fenêtre \
                              n'est pas validée",
                    )
                    .log_name("suivi.supprimer-groupe"),
            );
        }
    });
}

/// Le libellé du bouton de suppression groupée — **la règle du web**, reprise telle quelle
/// (`WatchlistTileController.bulkDeleteLabel`).
///
/// Aucune tuile cochée se lit « aucune exclusion » et non « rien à faire » : le bouton porte alors
/// « Supprimer tout » et agit sur la liste entière. Tout cocher à la main donne le même libellé,
/// par cohérence — même résultat, deux chemins pour y arriver.
fn bulk_label(selected: usize, total: usize) -> String {
    if selected == 0 || selected == total {
        "Supprimer tout".to_string()
    } else {
        format!("Supprimer ({selected})")
    }
}

/// La grille de tuiles, dans la zone défilable du panneau.
fn tile_grid(ui: &mut egui::Ui, icons: &UiIcons, panel: &design::PanelZones, state: &SuiviTab<'_>) {
    panel.scroll_area(ui, "suivi.grille", |ui, content_width| {
        ui.spacing_mut().item_spacing = Vec2::splat(TILE_GAP);
        let per_row = (((content_width + TILE_GAP) / (TILE + TILE_GAP)).floor() as usize).max(1);
        for (rang, chunk) in TRACKED.chunks(per_row).enumerate() {
            ui.horizontal(|ui| {
                for (colonne, entry) in chunk.iter().enumerate() {
                    let index = rang * per_row + colonne;
                    tracked_tile(
                        ui,
                        icons,
                        entry,
                        TileState {
                            hovered: state.hovered == Some(index),
                            select_mode: state.select_mode,
                            selected: state.selected.contains(&index),
                        },
                    );
                }
            });
        }
    });
}

struct TileState {
    hovered: bool,
    select_mode: bool,
    selected: bool,
}

/// **La tuile d'un suivi — un emplacement d'objet, et rien d'autre.**
///
/// | Élément | Ce qu'il dit |
/// | --- | --- |
/// | Cadre | ce qu'est l'entrée : bordure de **rareté** pour un objet, cadre neutre pour un monstre |
/// | Compteur bas-droit | `128` en incrémental, `7` sur `/50` en décompte ([`design::SlotCount`]) |
/// | Croix haut-droite | retrait — **seulement sur la tuile survolée**, et jamais en mode sélection |
/// | Case haut-gauche | sélection — **seulement en mode sélection**, et elle remplace la croix |
/// | Liseré or | la tuile est cochée |
///
/// Le nom **n'est pas peint** : il vit dans l'infobulle (demande explicite, voir la règle 1 de la
/// doc de module). Une tuile de 64 px ne peut pas porter « Griffe de Craqueleur » sans l'amputer, et
/// l'icône lève l'ambiguïté bien avant le texte — c'est l'arbitrage déjà rendu pour les tuiles
/// d'alerte, poussé ici jusqu'au bout.
fn tracked_tile(ui: &mut egui::Ui, icons: &UiIcons, entry: &Tracked, tile: TileState) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(TILE), egui::Sense::click());

    let frame = match entry.kind {
        Kind::Item(rarity) => SlotFrame::Rarity(to_slot_rarity(rarity)),
        // Un monstre n'a pas de rareté — cadre neutre, comme dans le bandeau in-game.
        Kind::Monster => SlotFrame::Plain,
    };
    let count = match entry.target {
        Some(target) => design::SlotCount::Fraction {
            current: entry.count,
            target,
        },
        None => design::SlotCount::Simple(entry.count),
    };

    // `ui.put` dans un ENFANT, jamais sur le `ui` de la rangée : `Ui::put` ouvre un scope, et un
    // scope avance le curseur du parent — la tuile suivante démarrerait au mauvais endroit. Piège
    // déjà payé sur les tuiles d'alerte, voir `panels::alerts_tab::alert_item`.
    let mut cellule = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    cellule.put(
        rect,
        design::item_slot()
            .size(TILE)
            .frame(frame)
            .icon(icons.unknown_entity_texture().id())
            .count(count)
            .log_name(entry.name.to_string()),
    );

    let ds = design::DesignSystem::get(ui.ctx());

    if tile.selected {
        ui.painter().rect_stroke(
            rect,
            4,
            Stroke::new(TILE_SELECTED_WIDTH, TILE_SELECTED),
            StrokeKind::Inside,
        );
    }

    if tile.select_mode {
        // La case **remplace** la croix : les deux gestes s'excluent, et deux marqueurs dans deux
        // coins d'une tuile de 64 px reviendraient à demander de viser.
        let case = Rect::from_min_size(
            egui::pos2(rect.left() + BADGE_INSET, rect.top() + BADGE_INSET),
            Vec2::splat(design::tokens::CHECKBOX_SIZE),
        );
        let mut coche = tile.selected;
        cellule.put(
            case,
            design::checkbox(&mut coche, "").log_name(format!("suivi.cocher-{}", entry.name)),
        );
    } else if tile.hovered {
        // Le voile dit le survol ET porte la croix — voir [`TILE_HOVER_SCRIM`].
        ui.painter().rect_filled(rect, 4, TILE_HOVER_SCRIM);
        let croix = Rect::from_center_size(
            egui::pos2(
                rect.right() - BADGE_INSET - BADGE / 2.0,
                rect.top() + BADGE_INSET + BADGE / 2.0,
            ),
            design::components::icon_button::glyph_fit(ds.icon_native_size(DsIcon::Close), BADGE),
        );
        ds.paint_icon(ui.painter(), croix, DsIcon::Close, TEXT);
    }

    // Le nom vit ici, et nulle part ailleurs sur la tuile.
    design::tooltip(&response).text(match entry.target {
        Some(target) => format!(
            "{} — décompte, {} restant sur {}",
            entry.name, entry.count, target
        ),
        None => format!("{} — {} au compteur", entry.name, entry.count),
    });
}

/// Le rouage de chargement, centré dans les DEUX axes de la zone que la grille occuperait — repris
/// de `panels::alerts_tab::loading_row`.
fn loading_row(ui: &mut egui::Ui, inner: Rect) {
    let reste = Rect::from_min_max(
        egui::pos2(inner.left(), ui.cursor().top()),
        inner.right_bottom(),
    );
    let mut zone = ui.new_child(egui::UiBuilder::new().max_rect(reste));
    zone.put(
        Rect::from_center_size(reste.center(), Vec2::splat(design::LoaderSize::Large.px())),
        design::loader()
            .size(design::LoaderSize::Large)
            .preview_frame(3),
    );
}

// -------------------------------------------------------------------------------------------
// La fenêtre « Suivre les objets de la recette »
// -------------------------------------------------------------------------------------------

/// **Une fenêtre du design system posée par-dessus la fenêtre Options**, pas un composant nouveau.
///
/// `design::window` peint dans tout le `max_rect` du `Ui` qu'on lui donne : il suffit de lui en
/// donner un plus petit, dans une couche au-dessus, derrière le voile de confirmation
/// ([`design::tokens::CONFIRM_SCRIM_ALPHA`]). Le portage n'a donc **rien à écrire** pour ce
/// dialogue — c'est le seul écran du lot dans ce cas.
///
/// Le contenu reprend la modale du web (`recipe-quantity-modal.component.html`) : l'objet source en
/// tête, la quantité voulue, puis un ingrédient par ligne avec sa quantité **multipliée par cette
/// quantité**, et un bouton d'imbrication sur les lignes qui ont elles-mêmes une recette.
fn recipe_dialog(ui: &mut egui::Ui, icons: &UiIcons, window: Rect, quantity: &mut i64) {
    let mut couche = ui.new_child(egui::UiBuilder::new().max_rect(window).layer_id(
        egui::LayerId::new(egui::Order::Foreground, egui::Id::new("suivi-recette")),
    ));
    couche.set_clip_rect(Rect::EVERYTHING);
    couche.painter().rect_filled(
        window,
        0,
        Color32::from_black_alpha(design::tokens::CONFIRM_SCRIM_ALPHA),
    );

    // 500 × 580 : la fenêtre doit montrer l'objet source, la quantité, et les trois ingrédients
    // AVEC la ligne dépliée de « Cuir Bouilli » — une boîte plus courte coupait la liste, et une
    // liste coupée ne laisse pas juger de la lisibilité des quantités multipliées.
    let boite = Rect::from_center_size(window.center(), Vec2::new(500.0, 580.0));
    let mut fenetre = couche.new_child(egui::UiBuilder::new().max_rect(boite));
    let chrome = design::window("Objets de la recette")
        .footer("Annuler", "Suivre")
        .log_name("suivi.recette")
        .show(&mut fenetre);

    design::panel().show(&mut fenetre, chrome.content, |ui, panel| {
        let width = panel.inner.width();

        // L'objet source — même emplacement que dans la grille, à une taille réduite : c'est un
        // rappel de ce qu'on décompose, pas le sujet de la fenêtre.
        let ligne = ui.allocate_space(Vec2::new(width, 44.0)).1;
        let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(ligne));
        cell.horizontal_centered(|ui| {
            ui.add(
                design::item_slot()
                    .size(40.0)
                    .frame(SlotFrame::Rarity(to_slot_rarity(WakfuRarity::Rare)))
                    .icon(icons.unknown_entity_texture().id())
                    .log_name("suivi.recette.source"),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new("Coiffe du Tofu")
                    .color(design::tokens::TEXT_GOLD)
                    .size(BODY_FONT_SIZE),
            );
        });

        ui.add_space(10.0);
        let ligne = ui.allocate_space(Vec2::new(width, FORM_ROW_HEIGHT)).1;
        ui.painter()
            .rect_filled(ligne, SETTING_ROW_RADIUS, SETTING_ROW_FILL);
        let mut cell = ui.new_child(
            egui::UiBuilder::new().max_rect(ligne.shrink2(Vec2::new(FORM_ROW_PAD_X, 0.0))),
        );
        cell.horizontal_centered(|ui| {
            ui.label(RichText::new("Quantité").color(TEXT).size(BODY_FONT_SIZE));
            let pas = design::stepper(quantity)
                .range(1..=9999)
                .size(28.0)
                .field_width(62.0)
                .log_name("suivi.recette.quantite");
            let (largeur, _) = pas.desired_size();
            let reste = ui.available_width() - largeur.unwrap_or(0.0);
            ui.add_space(reste.max(8.0));
            ui.add(pas);
        });

        ui.add_space(14.0);
        ui.add(design::heading("Ingrédients").trailing_gap(8.0));

        panel.scroll_area(ui, "suivi.recette.liste", |ui, content_width| {
            for (nom, rarete, quantite, imbricable) in INGREDIENTS {
                ingredient_row(
                    ui,
                    icons,
                    content_width,
                    nom,
                    *rarete,
                    *quantite * *quantity,
                    *imbricable,
                    0.0,
                );
                if *imbricable {
                    // Ligne dépliée : les sous-ingrédients portent la quantité du parent en
                    // facteur, comme le web.
                    for (sous_nom, sous_rarete, sous_quantite) in SUB_INGREDIENTS {
                        ingredient_row(
                            ui,
                            icons,
                            content_width,
                            sous_nom,
                            *sous_rarete,
                            *sous_quantite * *quantite * *quantity,
                            false,
                            22.0,
                        );
                    }
                }
            }
        });
    });
}

/// Une ligne d'ingrédient : emplacement, nom, quantité, et le bouton qui déplie sa propre recette.
#[allow(clippy::too_many_arguments)]
fn ingredient_row(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    width: f32,
    name: &str,
    rarity: WakfuRarity,
    quantity: i64,
    nestable: bool,
    indent: f32,
) {
    let ligne = ui.allocate_space(Vec2::new(width, 38.0)).1;
    let ligne = Rect::from_min_max(egui::pos2(ligne.left() + indent, ligne.top()), ligne.max);
    ui.painter()
        .rect_filled(ligne, SETTING_ROW_RADIUS, SETTING_ROW_FILL);
    let mut cell =
        ui.new_child(egui::UiBuilder::new().max_rect(ligne.shrink2(Vec2::new(8.0, 0.0))));
    cell.horizontal_centered(|ui| {
        ui.add(
            design::item_slot()
                .size(28.0)
                .frame(SlotFrame::Rarity(to_slot_rarity(rarity)))
                .icon(icons.unknown_entity_texture().id())
                .log_name(format!("suivi.recette.{name}")),
        );
        ui.add_space(8.0);
        ui.label(RichText::new(name).color(TEXT).size(13.0));
        let mut droite = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(ligne.shrink2(Vec2::new(8.0, 0.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        if nestable {
            droite.add(
                design::icon_button(DsIcon::Hammer)
                    .context(IconContext::Panel)
                    .size(24.0)
                    .preview_state(design::IconButtonState::Hovered)
                    .tooltip("Suivre les ingrédients de cet objet plutôt que l'objet lui-même")
                    .log_name(format!("suivi.recette.imbriquer-{name}")),
            );
            droite.add_space(8.0);
        } else {
            droite.add_space(32.0);
        }
        droite.label(
            RichText::new(format!("×{quantity}"))
                .color(SUBDUED)
                .size(13.0),
        );
    });
    ui.add_space(4.0);
}

// -------------------------------------------------------------------------------------------
// Le panneau de suggestions
// -------------------------------------------------------------------------------------------

/// Ce que la maquette fournit au composant d'autocomplétion : des entrées et des filtres.
///
/// Les jeux de textures doivent **survivre à la construction** — un `TextureHandle` libère sa
/// texture quand son dernier exemplaire tombe, et la planche sortirait avec des gemmes invisibles
/// (piège déjà payé dans les maquettes d'alertes).
struct SuggestionPreview {
    entries: Vec<design::AutocompleteEntry>,
    filters: Vec<design::AutocompleteFilter>,
    active: usize,
    _gems: RarityGems,
    _cats: CategoryIcons,
}

impl SuggestionPreview {
    fn new(ctx: &egui::Context, icons: &UiIcons) -> Self {
        let gems = RarityGems::load(ctx);
        let cats = CategoryIcons::load(ctx);
        let entries = SUGGESTIONS
            .iter()
            .map(|s| {
                let mut entry = design::AutocompleteEntry::new(
                    s.name,
                    s.filter.category_key().unwrap_or_default(),
                );
                // Pas de gemme sur un monstre : la rareté est une notion d'objet.
                if let Some(rarity) = s.rarity {
                    entry.gem = Some(gems.texture_id(rarity));
                    entry.gem_size = GEM_NATIVE;
                }
                entry.image = Some(icons.unknown_entity_texture().id());
                entry.disabled = s.already;
                if s.already {
                    entry.mention = Some("déjà suivi".to_owned());
                } else if s.recipe {
                    // **Provisoire, et c'est le point 3 de la doc de module** : l'icône de recette
                    // doit être un BOUTON dans la rangée, pas une mention de texte. Faute de place
                    // pour elle dans le composant, la planche la nomme pour qu'on voie où elle va.
                    entry.mention = Some("recette".to_owned());
                }
                entry
            })
            .collect();

        // Seules les catégories PRÉSENTES dans les résultats ont un bouton, « Tout » en tête. La
        // nouveauté par rapport aux alertes est la dernière : « Monstres ».
        let mut filters = vec![design::AutocompleteFilter::all(
            CategoryFilter::All.label(),
            Some(cats.texture_id(CategoryFilter::All)),
        )];
        let mut vus: Vec<CategoryFilter> = Vec::new();
        for s in SUGGESTIONS {
            if !vus.contains(&s.filter) {
                vus.push(s.filter);
            }
        }
        for filtre in vus {
            if let Some(key) = filtre.category_key() {
                filters.push(design::AutocompleteFilter::category(
                    key,
                    filtre.label(),
                    Some(cats.texture_id(filtre)),
                ));
            }
        }

        Self {
            entries,
            filters,
            // La deuxième : la première est « déjà suivi », donc pas sélectionnable.
            active: 1,
            _gems: gems,
            _cats: cats,
        }
    }
}

// -------------------------------------------------------------------------------------------
// Harnais
// -------------------------------------------------------------------------------------------

/// Où les planches sont écrites — `target/mockups`, **jamais commité** : ce sont des images
/// jetables, destinées à un artefact, pas des captures de non-régression.
///
/// Le dossier est vidé une fois par exécution : sans ça, la capture d'un rendu renommé y survit
/// indéfiniment, indiscernable d'une capture vivante.
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

/// Ouvre le harnais et rend la fenêtre Options avec son chrome réel, l'onglet « Suivi » actif.
///
/// **Le décor n'est pas recopié** : `design::window`, `design::tabs` et `design::panel` le
/// peignent, les mêmes composants que la vraie modale.
fn options_harness(
    mut build: impl FnMut(&mut egui::Ui, &UiIcons, &design::PanelZones, Rect) + 'static,
) -> Harness<'static> {
    let mut icons: Option<UiIcons> = None;
    let mut tab = MockTab::Suivi;
    Harness::builder().with_size(WINDOW).build_ui(move |ui| {
        overlay_ui::style::apply(ui.ctx());
        let icons = icons.get_or_insert_with(|| UiIcons::load(ui.ctx()));
        egui::Frame::NONE.fill(BACKDROP).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let window = ui.max_rect();
            let chrome = design::window("Options")
                .footer("Annuler", "Valider")
                .log_name("maquette")
                .show(ui);
            chrome.tabs(
                ui,
                design::tabs(&mut tab)
                    .entry(MockTab::Suivi, "Suivi")
                    .entry(MockTab::Alertes, "Alertes")
                    .entry(MockTab::Personnages, "Personnages")
                    .enabled(false)
                    .entry(MockTab::Parametres, "Paramètres")
                    .log_name("maquette-onglets"),
            );
            design::panel().show(ui, chrome.content, |ui, panel| {
                build(ui, icons, panel, window);
            });
        });
    })
}

/// Les paramètres d'une planche — un `struct` plutôt que huit booléens positionnels.
#[derive(Clone)]
struct Planche {
    mode: AddMode,
    target: i64,
    search: String,
    hovered: Option<usize>,
    select_mode: bool,
    selected: Vec<usize>,
    availability: Availability,
    suggestions: bool,
}

impl Default for Planche {
    fn default() -> Self {
        Self {
            mode: AddMode::Up,
            target: 100,
            search: String::new(),
            hovered: None,
            select_mode: false,
            selected: Vec::new(),
            availability: Availability::Ready,
            suggestions: false,
        }
    }
}

/// Rend une planche et l'écrit.
fn planche(nom: &str, p: Planche) {
    let mut preview: Option<SuggestionPreview> = None;
    let mut harness = options_harness(move |ui, icons, panel, _window| {
        if p.suggestions && preview.is_none() {
            preview = Some(SuggestionPreview::new(ui.ctx(), icons));
        }
        let mut target = p.target;
        let mut search = p.search.clone();
        suivi_tab(
            ui,
            icons,
            panel,
            &mut SuiviTab {
                mode: p.mode,
                target: &mut target,
                search: &mut search,
                hovered: p.hovered,
                select_mode: p.select_mode,
                selected: &p.selected,
                availability: p.availability,
                suggestions: preview.as_ref(),
            },
        );
    });
    harness.run();
    write_mockup(&mut harness, nom);
    println!("  {nom}");
}

// -------------------------------------------------------------------------------------------
// Planches
// -------------------------------------------------------------------------------------------

fn main() {
    // 1 — L'onglet au repos, mode incrémental : le formulaire tient sur une ligne, la grille est
    // la vedette.
    planche("suivi_incremental", Planche::default());

    // 2 — Mode décompte : la ligne « Quantité » apparaît, avec ses badges et son pas numérique.
    planche(
        "suivi_decompte",
        Planche {
            mode: AddMode::Down,
            target: 250,
            ..Planche::default()
        },
    );

    // 3 — Le champ déplié : objets ET monstres dans la même liste, la bande de filtres porte
    // « Monstres », et les entrées déjà suivies sont grisées.
    planche(
        "suivi_autocompletion",
        Planche {
            mode: AddMode::Down,
            target: 250,
            search: "tofu".to_string(),
            suggestions: true,
            ..Planche::default()
        },
    );

    // 4 — Le survol d'une tuile : son voile et sa croix de retrait, et rien sur les autres.
    planche(
        "suivi_survol_retrait",
        Planche {
            hovered: Some(6),
            ..Planche::default()
        },
    );

    // 5 — La sélection multiple, sélection partielle : cases à cocher partout, liseré or sur les
    // cochées, bouton « Supprimer (3) ».
    planche(
        "suivi_selection_partielle",
        Planche {
            select_mode: true,
            selected: vec![1, 4, 9],
            ..Planche::default()
        },
    );

    // 6 — La sélection multiple, rien de coché : le bouton dit « Supprimer tout » — aucune
    // exclusion cochée, voir `bulk_label`.
    planche(
        "suivi_selection_vide",
        Planche {
            select_mode: true,
            ..Planche::default()
        },
    );

    // 7 — La confirmation de retrait d'une tuile.
    suivi_confirmation_retrait();

    // 8 — La fenêtre « Objets de la recette », ouverte depuis une suggestion.
    suivi_recette();

    // 9 & 10 — Les deux états où la liste n'est pas éditable.
    planche(
        "suivi_chargement",
        Planche {
            availability: Availability::Loading,
            ..Planche::default()
        },
    );
    planche(
        "suivi_sans_compte",
        Planche {
            availability: Availability::NoAccount,
            ..Planche::default()
        },
    );

    let dir = mockup_dir();
    let ecrites = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    let affiche = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("{ecrites} planches écrites dans {}", affiche.display());
}

/// **Le retrait d'une tuile passe par la boîte du jeu**, pas par une popover ancrée au bouton comme
/// le web — même arbitrage que l'onglet Alertes, et pour la même raison : le jeu n'a qu'une forme
/// de confirmation, centrée et voilée, et son bouton affirmatif est **or, jamais rouge**.
fn suivi_confirmation_retrait() {
    let mut harness = options_harness(move |ui, icons, panel, window| {
        let mut target = 100;
        let mut search = String::new();
        suivi_tab(
            ui,
            icons,
            panel,
            &mut SuiviTab {
                mode: AddMode::Up,
                target: &mut target,
                search: &mut search,
                hovered: Some(6),
                select_mode: false,
                selected: &[],
                availability: Availability::Ready,
                suggestions: None,
            },
        );
        design::confirm_dialog("Retirer « Chafer Élite » de votre suivi ?")
            .over(window)
            .log_name("suivi.retrait")
            .show(ui);
    });
    harness.run();
    write_mockup(&mut harness, "suivi_confirmation_retrait");
    println!("  suivi_confirmation_retrait");
}

/// La fenêtre des ingrédients, ouverte par le bouton « recette » d'une suggestion.
fn suivi_recette() {
    let mut harness = options_harness(move |ui, icons, panel, window| {
        let mut target = 100;
        let mut search = String::from("tofu");
        suivi_tab(
            ui,
            icons,
            panel,
            &mut SuiviTab {
                mode: AddMode::Up,
                target: &mut target,
                search: &mut search,
                hovered: None,
                select_mode: false,
                selected: &[],
                availability: Availability::Ready,
                suggestions: None,
            },
        );
        let mut quantite = 2;
        recipe_dialog(ui, icons, window, &mut quantite);
    });
    harness.run();
    write_mockup(&mut harness, "suivi_recette");
    println!("  suivi_recette");
}
