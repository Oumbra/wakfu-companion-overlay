//! **Onglet « Suivi » de la fenêtre Options** — l'écran qui compose la liste des objets et des
//! monstres suivis : ce qu'on ajoute, en quel mode, et ce qu'on retire.
//!
//! Il manquait jusqu'au 2026-09-13 : le bandeau de suivi in-game ([`panels::watchlist`]) affichait
//! depuis le 2026-09-01 une liste **éditable seulement depuis le site**, et ses deux boutons
//! « + » / « − » étaient inertes, avec une infobulle qui l'assumait (« je pense qu'on le fera plus
//! tard quand tu auras tout câblé »).
//!
//! ## Ce fichier est le portage d'une maquette validée
//!
//! La mise en page vient de `crates/overlay-testkit/examples/suivi-mockups.rs`, spécifiée avec
//! l'utilisateur en deux itérations. Les cotes, les couleurs et les règles y sont documentées une
//! par une avec leur provenance ; ce portage ne les redécide pas.
//!
//! Les cinq règles que la maquette a établies et que ce fichier tient :
//!
//! 1. **La liste n'affiche QUE l'emplacement d'objet, et QUE la cible.** Pas de nom sous la tuile
//!    (contrairement à l'onglet Alertes), et pas de valeur courante non plus : cet écran sert à
//!    *composer* une liste, pas à la lire — les compteurs vivants restent au bandeau in-game, où
//!    les lire est justement le but. La cible d'un décompte, elle, n'est pas une mesure mais un
//!    **réglage** de l'entrée, au même titre que son mode : elle reste ([`design::SlotCount::Target`]).
//! 2. **Le mode se choisit AVANT le nom.** Incrémental ou décompte, et en décompte la quantité de
//!    départ : les deux sont figés à la création côté web (« Cible fixée à la création »), les
//!    demander après coup mentirait sur ce que la tuile permet ensuite.
//! 3. **Le retrait se fait à la tuile, au survol, et sans confirmation.** La croix n'apparaît que
//!    sur la tuile survolée et vire au rouge sous le pointeur. Aucune boîte : la fenêtre est
//!    transactionnelle, « Annuler » rattrape tout et « Valider » est une seconde garde.
//! 4. **La sélection multiple est un MODE**, pas une case permanente. Sélection vide = « Supprimer
//!    tout » (aucune exclusion cochée), la règle du web — voir [`bulk_label`].
//! 5. **Rien n'est écrit avant « Valider »** : l'onglet travaille sur un brouillon que l'appelant
//!    lui prête, comme l'onglet Alertes.
//!
//! ## Le brouillon ne porte QUE des définitions
//!
//! Une entrée de suivi a deux moitiés qui ne vivent pas au même endroit : ses **définitions** (nom,
//! genre, mode, cible) se règlent ici, son **compteur** est incrémenté par le moteur au fil du log.
//! Le brouillon est pris à l'ouverture de la fenêtre ; un objet ramassé pendant qu'elle est ouverte
//! y resterait à sa valeur d'alors. C'est pourquoi la validation passe par
//! [`overlay_engine::watchlist::WatchlistState::apply_definitions`], qui **ignore le `count` du
//! brouillon** et garde celui du moteur — valider ne doit pas annuler un ramassage.

use egui::{Color32, Rect, RichText, Vec2};
use overlay_engine::{CatalogIndex, IconRef, WatchlistEntry, WatchlistKind, WatchlistMode};

use crate::design::{self, ButtonSize, ButtonVariant, DsIcon, IconContext, SlotFrame};
use crate::rarity_bridge::to_slot_rarity;
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::ui_icons::UiIcons;

// -------------------------------------------------------------------------------------------
// Jetons — repris de la maquette, où chacun porte sa provenance.
// -------------------------------------------------------------------------------------------

/// Texte courant — blanc, comme tout texte de corps du jeu.
const TEXT: Color32 = Color32::WHITE;

/// Gris des libellés secondaires — `#b8b9ba`, le gris unique du jeu (mesure convergente sur quatre
/// captures, voir `panels::options_modal::SECTION_TITLE_TEXT`).
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);

/// Fond d'une ligne de réglage — l'idiome des lignes d'aptitude du jeu
/// (`interface-personnage-aptitudes.png`, `#26282b`).
const SETTING_ROW_FILL: Color32 = Color32::from_rgb(0x26, 0x28, 0x2B);
const SETTING_ROW_RADIUS: u8 = 4;
/// Hauteur d'une ligne de formulaire — 40, comme la ligne « Fermeture automatique » des alertes.
const FORM_ROW_HEIGHT: f32 = 40.0;
const FORM_ROW_PAD_X: f32 = 12.0;
const FORM_ROW_GAP: f32 = 6.0;

const BODY_FONT_SIZE: f32 = 15.0;
/// Aération autour d'un titre de section — 18 px, la valeur arrêtée pour l'onglet Alertes.
const SECTION_GAP: f32 = 18.0;

/// Côté d'une tuile — **l'emplacement d'objet du jeu**, celui du bandeau de suivi.
const TILE: f32 = design::tokens::ITEM_SLOT_SIZE;
/// Gouttière entre deux tuiles — celle du bandeau (`panels::watchlist::TILE_GAP`).
const TILE_GAP: f32 = 12.0;

const BADGE: f32 = 14.0;
/// Retrait de la case à cocher et de la croix depuis le coin de la tuile — 5 px, et pas 4.
///
/// Le liseré d'un emplacement occupe les pixels 2 à 4 depuis le bord (voir
/// `design::item_slot::border_ring`) : à 4, la case venait s'y coller et mangeait le liseré de
/// sélection dans son coin. Le pixel de plus le laisse voir tout du long — demande explicite du
/// 2026-09-13, « décaler la checkbox d'un pixel pour laisser la visibilité sur la bordure ».
const BADGE_INSET: f32 = 5.0;

/// Voile d'une tuile SURVOLÉE, sous sa croix — il dit le survol *et* donne à la croix un fond assez
/// sombre pour rester lisible par-dessus n'importe quelle bordure de rareté.
const TILE_HOVER_SCRIM: Color32 = Color32::from_black_alpha(0x66);

/// Rouge de la croix sous le pointeur — `INFO_ALERT`, le seul rouge mesuré du jeu.
const REMOVE_HOVER: Color32 = design::tokens::INFO_ALERT;

/// Largeur d'un badge de quantité — celle de `−1000`, le plus large des libellés qu'il prend. Une
/// largeur qui suivrait le libellé ferait bouger les badges à l'appui sur `Alt`, et on relâcherait
/// la touche sur un autre badge que celui visé.
const BADGE_WIDTH: f32 = 52.0;

/// Les quantités toutes faites du web (`WatchlistTileController.targetPresets`).
const TARGET_PRESETS: [i64; 5] = [10, 50, 100, 500, 1000];

/// Domaine de la quantité de départ — `[min]="1" [max]="9999"` du web.
const TARGET_MIN: i64 = 1;
const TARGET_MAX: i64 = 9999;

/// La phrase sous le titre.
const DESC: &str = "Les objets et monstres suivis comptent automatiquement ce que vous ramassez \
                    et ce que vous vaincez. Le bandeau de suivi les affiche par-dessus le jeu.";

// -------------------------------------------------------------------------------------------
// État et contrat
// -------------------------------------------------------------------------------------------

/// Le mode du futur compteur — miroir de [`WatchlistMode`], choisi avant le nom (règle 2).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AddMode {
    #[default]
    Up,
    Down,
}

impl AddMode {
    fn to_watchlist(self) -> WatchlistMode {
        match self {
            AddMode::Up => WatchlistMode::Up,
            AddMode::Down => WatchlistMode::Down,
        }
    }
}

/// Ce que l'onglet garde d'une frame à l'autre, et qui n'appartient PAS au brouillon.
#[derive(Debug, Clone)]
pub struct SuiviTabState {
    /// Saisie du champ d'ajout.
    pub search: String,
    /// Mode du prochain ajout.
    pub mode: AddMode,
    /// Quantité de départ du prochain décompte.
    pub target: i64,
    /// Mode « sélection multiple » ouvert.
    pub select_mode: bool,
    /// Clés des tuiles cochées — voir [`entry_key`].
    pub selected: Vec<String>,
    /// La fenêtre « Objets de la recette », ouverte — `None` le reste du temps.
    pub recipe: Option<RecipeDialogState>,
}

impl Default for SuiviTabState {
    fn default() -> Self {
        Self {
            search: String::new(),
            mode: AddMode::default(),
            // 1, comme le web (`addTarget` part à 1) : c'est la plus petite cible valide.
            target: TARGET_MIN,
            select_mode: false,
            selected: Vec::new(),
            recipe: None,
        }
    }
}

/// Ce que la fenêtre de recette garde entre deux frames.
#[derive(Debug, Clone)]
pub struct RecipeDialogState {
    /// L'objet dont on décompose la recette.
    pub item_id: i64,
    pub item_name: String,
    /// Combien d'exemplaires on veut fabriquer — multiplie toutes les quantités.
    pub quantity: i64,
    /// Les ingrédients résolus, `None` tant que la résolution réseau est en vol.
    pub ingredients: Option<Vec<overlay_engine::RecipeIngredient>>,
    /// Les chemins dépliés — une ligne dépliée est remplacée par ses propres ingrédients.
    pub nested: std::collections::HashSet<String>,
}

impl RecipeDialogState {
    pub fn new(item_id: i64, item_name: String) -> Self {
        Self {
            item_id,
            item_name,
            quantity: 1,
            ingredients: None,
            nested: std::collections::HashSet::new(),
        }
    }
}

/// D'où vient la liste affichée, et si elle est modifiable.
///
/// **Deux cas seulement.** La maquette en portait un troisième, « aucun compte lié » ; il a été
/// retiré le 2026-09-13 sur décision de l'utilisateur : « l'overlay n'est utilisable qu'en mode
/// connecté ». Peindre un écran pour un état que le produit n'a pas, c'est entretenir un doute sur
/// ce qu'il est.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SuiviAvailability {
    /// La liste vient du compte et s'édite.
    #[default]
    Ready,
    /// `GET /api/v1/settings` en vol : rien à afficher tant que la réponse n'est pas là, et ce
    /// qu'on ajouterait serait écrasé par la liste qui arrive.
    Loading,
}

/// Ce que l'onglet a besoin de recevoir pour peindre de vraies données.
pub struct SuiviTabContext<'a> {
    /// **Le brouillon**, modifié en place par les gestes. Rien n'est envoyé au compte ici — seules
    /// les DÉFINITIONS comptent, voir la doc de module.
    pub entries: &'a mut Vec<WatchlistEntry>,
    /// Pour chercher objets et monstres, et pour résoudre rareté et icône de ce qui est listé.
    pub catalog: &'a CatalogIndex,
    pub remote_icons: &'a RemoteIconStore,
    pub remote_icon_textures: &'a mut RemoteIconTextures,
    /// Repli quand l'icône n'est pas encore descendue du CDN.
    pub icons: &'a UiIcons,
    pub availability: SuiviAvailability,
}

/// Ce que l'utilisateur vient de demander et que l'onglet ne sait pas faire lui-même.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum SuiviTabAction {
    #[default]
    None,
    /// Résoudre les ingrédients de cet objet — l'appelant seul a le réseau
    /// (`overlay_sync::client::fetch_item_detail`, sur un thread).
    ResolveRecipe(i64),
}

/// Identifie une entrée de façon unique, homonymes d'id différents compris — miroir de
/// `watchlistEntryKey` (dépôt web).
fn entry_key(entry: &WatchlistEntry) -> String {
    format!(
        "{}::{}",
        entry.name,
        entry
            .catalog_id
            .map(|id| id.to_string())
            .unwrap_or_default()
    )
}

/// Peint l'onglet dans le panneau de section de la fenêtre Options.
pub fn show(
    ui: &mut egui::Ui,
    panel: &design::PanelZones,
    state: &mut SuiviTabState,
    ctx: &mut SuiviTabContext<'_>,
) -> SuiviTabAction {
    let mut action = SuiviTabAction::None;
    let width = panel.inner.width();

    ui.add(design::heading("Suivi").trailing_gap(SECTION_GAP * 0.5));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP);

    add_form(ui, state, width);
    ui.add_space(SECTION_GAP * 0.75);
    if let Some(demande) = add_field(ui, state, ctx, width) {
        action = demande;
    }
    ui.add_space(SECTION_GAP);

    list_header(ui, state, ctx, width);

    if ctx.availability == SuiviAvailability::Loading {
        loading_row(ui, panel.inner);
        return action;
    }

    tile_grid(ui, panel, state, ctx);
    action
}

fn paragraph(ui: &mut egui::Ui, text: &str) {
    // **Un paragraphe, pas un bloc d'information** : `design::info_text` porte une pastille et le
    // poids d'une remarque, ce qui donnerait à cette phrase une importance qu'elle n'a pas.
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
/// La seconde ligne n'apparaît qu'en décompte : en incrémental le compteur part de zéro et monte,
/// il n'y a aucune quantité à demander, et laisser la ligne grisée occuperait la place d'un réglage
/// qui n'existe pas.
fn add_form(ui: &mut egui::Ui, state: &mut SuiviTabState, width: f32) {
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
/// fond glissant ; le jeu n'en a aucun — son vocabulaire pour un choix binaire est la case à
/// cocher, et pour un choix exclusif entre deux actions nommées, le bouton. Deux cases mutuellement
/// exclusives seraient un contresens (une case dit « oui/non », pas « l'un ou l'autre »), et une
/// liste déroulante à deux entrées cacherait la moitié du choix derrière un clic.
fn mode_line(ui: &mut egui::Ui, state: &mut SuiviTabState, row: Rect) {
    let mut cell =
        ui.new_child(egui::UiBuilder::new().max_rect(row.shrink2(Vec2::new(FORM_ROW_PAD_X, 0.0))));
    let mut choisi = None;
    let actuel = state.mode;
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
            let actif = actuel == mode;
            if ui
                .add(
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
                )
                .clicked()
            {
                choisi = Some(mode);
            }
            ui.add_space(6.0);
        }
    });
    if let Some(mode) = choisi {
        state.mode = mode;
    }
}

/// La ligne « Quantité » — les badges du web, puis le pas numérique.
///
/// **Les badges ajoutent, ils ne remplacent pas**, et `Alt` inverse le signe : c'est ce que fait
/// `setAddTargetPreset` côté web (dont la documentation dit le contraire de son code — c'est le code
/// qui fait foi, confirmé par l'utilisateur le 2026-09-13). Le web ne le dit nulle part ; ici les
/// badges le **montrent** — tant qu'`Alt` est maintenu, ils passent en or et affichent `−10 …
/// −1000`, et la mention « (Alt : retirer) » sous le libellé de la ligne le dit une fois.
fn target_line(ui: &mut egui::Ui, state: &mut SuiviTabState, row: Rect) {
    let alt = ui.input(|i| i.modifiers.alt);
    let mut cell =
        ui.new_child(egui::UiBuilder::new().max_rect(row.shrink2(Vec2::new(FORM_ROW_PAD_X, 0.0))));
    let mut delta: Option<i64> = None;
    cell.horizontal_centered(|ui| {
        // Le libellé et sa mention, empilés : la mention explique la ligne entière, elle n'est pas
        // un contrôle de plus dans la rangée.
        ui.vertical(|ui| {
            ui.label(RichText::new("Quantité").color(TEXT).size(BODY_FONT_SIZE));
            ui.label(
                RichText::new("(Alt : retirer)")
                    .color(if alt {
                        design::tokens::TEXT_GOLD
                    } else {
                        SUBDUED
                    })
                    .size(11.0),
            );
        });
        ui.add_space(12.0);
        for preset in TARGET_PRESETS {
            if ui
                .add(
                    design::button(if alt {
                        format!("−{preset}")
                    } else {
                        preset.to_string()
                    })
                    // L'or est la couleur d'ÉTAT de ce design system : les badges la prennent
                    // pendant l'appui, comme un onglet actif ou une case cochée.
                    .variant(if alt {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Secondary
                    })
                    .size(ButtonSize::Height(24.0))
                    .min_width(BADGE_WIDTH)
                    .tooltip(if alt {
                        format!("Retirer {preset} à la quantité")
                    } else {
                        format!("Ajouter {preset} à la quantité — Alt pour retirer")
                    })
                    .log_name(format!("suivi.quantite-{preset}")),
                )
                .clicked()
            {
                delta = Some(if alt { -preset } else { preset });
            }
            ui.add_space(4.0);
        }

        let pas = design::stepper(&mut state.target)
            .range(TARGET_MIN..=TARGET_MAX)
            .size(26.0)
            .field_width(54.0)
            .log_name("suivi.quantite");
        let (largeur, _) = pas.desired_size();
        let reste = ui.available_width() - largeur.unwrap_or(0.0);
        ui.add_space(reste.max(8.0));
        ui.add(pas);
    });
    if let Some(delta) = delta {
        state.target = (state.target + delta).clamp(TARGET_MIN, TARGET_MAX);
    }
}

/// Le champ d'ajout et son panneau de suggestions — **objets ET monstres**, le domaine « les deux »
/// du web.
fn add_field(
    ui: &mut egui::Ui,
    state: &mut SuiviTabState,
    ctx: &mut SuiviTabContext<'_>,
    width: f32,
) -> Option<SuiviTabAction> {
    // Champ grisé pendant une lecture en vol : ce qu'on ajouterait serait écrasé par la liste qui
    // arrive.
    let enabled = ctx.availability == SuiviAvailability::Ready;
    let min = design::tokens::AUTOCOMPLETE_MIN_QUERY_LEN;
    let (objets, monstres) = if enabled {
        (
            ctx.catalog.search_items(&state.search, min, usize::MAX),
            ctx.catalog.search_monsters(&state.search, min, usize::MAX),
        )
    } else {
        (Vec::new(), Vec::new())
    };

    // **La bande d'abord, les entrées ensuite** — l'ordre de ces deux blocs est celui des demandes
    // d'icônes au CDN, et `RemoteIconStore` sert une frame dans son ordre de demande : les icônes
    // de catégorie, qui coiffent toute la liste, doivent partir avant les images de rangées dont
    // cinq seulement sont visibles.
    let mut categories: Vec<overlay_engine::WakfuItemCategory> =
        objets.iter().map(|item| item.category).collect();
    categories.sort_by_key(|c| c.icon_number());
    categories.dedup();
    let mut filters = vec![design::AutocompleteFilter::all(
        "Tout",
        texture_id(ui, ctx, &IconRef::for_all_categories()),
    )];
    for category in categories {
        filters.push(design::AutocompleteFilter::category(
            category_key(category),
            category_label(category),
            texture_id(ui, ctx, &IconRef::for_item_category(category)),
        ));
    }
    // Le filtre « Monstres » ferme la bande, comme le web le fait en domaine « les deux ».
    if !monstres.is_empty() {
        filters.push(design::AutocompleteFilter::category(
            MONSTER_FILTER_KEY,
            "Monstres",
            texture_id(ui, ctx, &IconRef::for_monster_category()),
        ));
    }

    // Une seule liste, objets puis monstres — c'est le référentiel qui dit ce qu'est chaque entrée,
    // l'utilisateur n'a aucun sélecteur de type à actionner avant de chercher.
    let mut entries: Vec<design::AutocompleteEntry> = Vec::new();
    for item in &objets {
        let deja = ctx
            .entries
            .iter()
            .any(|e| e.catalog_id == Some(item.id) || e.name == item.name);
        let mut entry =
            design::AutocompleteEntry::new(item.name.clone(), category_key(item.category));
        if let Some((id, size)) = texture(ui, ctx, &IconRef::for_rarity(item.rarity)) {
            entry.gem = Some(id);
            entry.gem_size = size;
        }
        entry.image = texture_id(ui, ctx, &item.icon);
        entry.disabled = deja;
        if deja {
            entry.mention = Some("déjà suivi".to_string());
        } else if ctx.catalog.find_item_has_recipe(&item.name, Some(item.id)) {
            // L'action de rangée, dans le composant depuis le 2026-09-13 — elle porte sur CETTE
            // suggestion : suivre ses ingrédients plutôt que l'objet lui-même.
            entry.action = Some(DsIcon::Hammer);
            entry.action_tooltip = Some("Suivre les objets de la recette".to_string());
        }
        entries.push(entry);
    }
    for monstre in &monstres {
        let deja = ctx
            .entries
            .iter()
            .any(|e| e.catalog_id == Some(monstre.id) || e.name == monstre.name);
        // **Pas de gemme** : un monstre n'a pas de rareté.
        let mut entry = design::AutocompleteEntry::new(monstre.name.clone(), MONSTER_FILTER_KEY);
        entry.image = texture_id(ui, ctx, &monstre.icon);
        entry.disabled = deja;
        if deja {
            entry.mention = Some("déjà suivi".to_string());
        }
        entries.push(entry);
    }

    let outcome = design::autocomplete(&mut state.search)
        .placeholder("Ajouter un objet ou un monstre à suivre…")
        .width(width)
        .enabled(enabled)
        .entries(&entries)
        .filters(&filters)
        .log_name("suivi.ajout")
        .show(ui);

    if let Some(index) = outcome.selected {
        if let Some(item) = objets.get(index) {
            push_entry(
                ctx.entries,
                &item.name,
                Some(item.id),
                WatchlistKind::Item,
                state,
            );
        } else if let Some(monstre) = monstres.get(index - objets.len()) {
            push_entry(
                ctx.entries,
                &monstre.name,
                Some(monstre.id),
                WatchlistKind::Enemy,
                state,
            );
        }
    }
    if let Some(index) = outcome.action_on {
        if let Some(item) = objets.get(index) {
            state.recipe = Some(RecipeDialogState::new(item.id, item.name.clone()));
            return Some(SuiviTabAction::ResolveRecipe(item.id));
        }
    }
    None
}

/// Ajoute une entrée au brouillon, avec le mode et la cible du formulaire — et remet celui-ci à
/// zéro, comme `resetAddForm` côté web.
///
/// `count` part de ce que le mode impose, mais c'est indicatif : la validation le recalcule depuis
/// l'état vivant du moteur (voir la doc de module).
fn push_entry(
    entries: &mut Vec<WatchlistEntry>,
    name: &str,
    catalog_id: Option<i64>,
    kind: WatchlistKind,
    state: &mut SuiviTabState,
) {
    let mode = state.mode.to_watchlist();
    let countdown_target = match mode {
        WatchlistMode::Down => state.target.max(TARGET_MIN),
        WatchlistMode::Up => 0,
    };
    entries.push(WatchlistEntry {
        name: name.to_string(),
        kind,
        mode,
        count: countdown_target,
        countdown_target,
        catalog_id,
    });
    state.target = TARGET_MIN;
}

/// L'en-tête de la liste : son titre à gauche, ses commandes à droite.
fn list_header(
    ui: &mut egui::Ui,
    state: &mut SuiviTabState,
    ctx: &mut SuiviTabContext<'_>,
    width: f32,
) {
    let row = ui.allocate_space(Vec2::new(width, 34.0)).1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    let mut bascule = false;
    let mut supprimer = false;
    let total = ctx.entries.len();
    let selection = state.selected.len();
    let select_mode = state.select_mode;
    let availability = ctx.availability;
    cell.horizontal_centered(|ui| {
        ui.add(design::heading("Éléments suivis"));
        // **Rien à commander quand il n'y a rien à lister** : pendant que la liste descend, le
        // bouton disparaît au lieu de rester grisé — le geste n'a pas d'objet.
        if availability != SuiviAvailability::Ready {
            return;
        }
        let mut droite = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        let mut bouton = design::icon_button(DsIcon::Delete)
            .context(IconContext::Panel)
            .tooltip(if select_mode {
                "Quitter la sélection"
            } else {
                "Suppression multiple"
            })
            .log_name("suivi.selection");
        if select_mode {
            // Le mode ouvert se lit sur le bouton lui-même, comme un onglet actif.
            bouton = bouton.preview_state(design::IconButtonState::Hovered);
        }
        if droite.add(bouton).clicked() {
            bascule = true;
        }
        if select_mode {
            droite.add_space(8.0);
            if droite
                .add(
                    // **Rouge**, comme le « Annuler » du pied de page — décision explicite de
                    // l'utilisateur le 2026-09-13, qui prévaut sur la règle « le rouge est réservé
                    // au pied de fenêtre » que l'onglet Alertes avait posée.
                    design::button(bulk_label(selection, total))
                        .variant(ButtonVariant::Danger)
                        .size(ButtonSize::Height(28.0))
                        .min_width(150.0)
                        .tooltip(
                            "Retire les tuiles cochées du suivi — annulable tant que la fenêtre \
                             n'est pas validée",
                        )
                        .log_name("suivi.supprimer-groupe"),
                )
                .clicked()
            {
                supprimer = true;
            }
        }
    });

    if supprimer {
        // Sélection vide : on retire TOUT (aucune exclusion cochée) — voir [`bulk_label`].
        if state.selected.is_empty() {
            ctx.entries.clear();
        } else {
            let cochees: std::collections::HashSet<&String> = state.selected.iter().collect();
            ctx.entries
                .retain(|entry| !cochees.contains(&entry_key(entry)));
        }
        state.select_mode = false;
        state.selected.clear();
    } else if bascule {
        state.select_mode = !state.select_mode;
        state.selected.clear();
    }
}

/// Le libellé du bouton de suppression groupée — **la règle du web**, reprise telle quelle.
///
/// Aucune tuile cochée se lit « aucune exclusion » et non « rien à faire » : le bouton porte alors
/// « Supprimer tout » et agit sur la liste entière. Tout cocher à la main donne le même libellé, par
/// cohérence — même résultat, deux chemins pour y arriver.
fn bulk_label(selected: usize, total: usize) -> String {
    if selected == 0 || selected == total {
        "Supprimer tout".to_string()
    } else {
        format!("Supprimer ({selected})")
    }
}

/// La grille de tuiles, dans la zone défilable du panneau.
fn tile_grid(
    ui: &mut egui::Ui,
    panel: &design::PanelZones,
    state: &mut SuiviTabState,
    ctx: &mut SuiviTabContext<'_>,
) {
    // Le rendu lit le brouillon et les gestes le modifient : les collecter d'abord évite d'emprunter
    // `ctx.entries` en lecture et en écriture dans la même boucle.
    let tuiles: Vec<TileData> = ctx
        .entries
        .iter()
        .map(|entry| TileData {
            key: entry_key(entry),
            name: entry.name.clone(),
            kind: entry.kind,
            target: (entry.mode == WatchlistMode::Down).then_some(entry.countdown_target),
            icon: match entry.kind {
                WatchlistKind::Item => ctx.catalog.find_item_icon(&entry.name, entry.catalog_id),
                WatchlistKind::Enemy => {
                    ctx.catalog.find_monster_icon(&entry.name, entry.catalog_id)
                }
            },
            rarity: ctx.catalog.find_item_rarity(&entry.name, entry.catalog_id),
        })
        .collect();

    let mut retrait: Option<String> = None;
    let mut bascule: Option<String> = None;
    let select_mode = state.select_mode;
    let cochees: std::collections::HashSet<&String> = state.selected.iter().collect();

    panel.scroll_area(ui, "suivi.grille", |ui, content_width| {
        ui.spacing_mut().item_spacing = Vec2::splat(TILE_GAP);
        let per_row = (((content_width + TILE_GAP) / (TILE + TILE_GAP)).floor() as usize).max(1);
        for chunk in tuiles.chunks(per_row) {
            ui.horizontal(|ui| {
                for tuile in chunk {
                    match tracked_tile(ui, ctx, tuile, select_mode, cochees.contains(&tuile.key)) {
                        TileClick::Remove => retrait = Some(tuile.key.clone()),
                        TileClick::Toggle => bascule = Some(tuile.key.clone()),
                        TileClick::None => {}
                    }
                }
            });
        }
    });

    // **Le retrait est immédiat — sur le BROUILLON, pas sur le compte.** Rien ne part au réseau
    // avant « Valider », et « Annuler » rend la liste telle qu'elle était : le geste est déjà
    // réversible deux fois, une boîte de confirmation par-dessus n'ajoutait qu'un clic
    // (décision du 2026-09-13, appliquée du même coup à l'onglet Alertes).
    if let Some(cle) = retrait {
        ctx.entries.retain(|entry| entry_key(entry) != cle);
        state.selected.retain(|k| *k != cle);
    }
    if let Some(cle) = bascule {
        if let Some(pos) = state.selected.iter().position(|k| *k == cle) {
            state.selected.remove(pos);
        } else {
            state.selected.push(cle);
        }
    }
}

/// Ce qu'une tuile a besoin de savoir — assemblé avant la boucle, voir [`tile_grid`].
struct TileData {
    key: String,
    name: String,
    kind: WatchlistKind,
    /// La cible, pour un décompte — `None` en incrémental, et la tuile ne porte alors aucun chiffre.
    target: Option<i64>,
    icon: Option<IconRef>,
    rarity: overlay_engine::WakfuRarity,
}

enum TileClick {
    None,
    Remove,
    Toggle,
}

/// **La tuile d'un suivi — un emplacement d'objet, et rien d'autre.**
///
/// | Élément | Ce qu'il dit |
/// | --- | --- |
/// | Cadre | ce qu'est l'entrée : bordure de **rareté** pour un objet, cadre neutre pour un monstre |
/// | Bas-droit | la **cible** d'un décompte (`/50`) — et rien du tout pour un incrémental |
/// | Croix haut-droite | retrait — **seulement sur la tuile survolée**, rouge sous le pointeur |
/// | Case haut-gauche | sélection — **seulement en mode sélection**, et elle remplace la croix |
/// | Liseré or | la tuile est cochée |
///
/// Ni le nom ni la valeur courante ne sont peints — voir la règle 1 de la doc de module.
fn tracked_tile(
    ui: &mut egui::Ui,
    ctx: &mut SuiviTabContext<'_>,
    tuile: &TileData,
    select_mode: bool,
    cochee: bool,
) -> TileClick {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(TILE), egui::Sense::click());

    let frame = match tuile.kind {
        WatchlistKind::Item => SlotFrame::Rarity(to_slot_rarity(tuile.rarity)),
        // Un monstre n'a pas de rareté — cadre neutre, comme dans le bandeau in-game.
        WatchlistKind::Enemy => SlotFrame::Plain,
    };
    let icon_id = tuile
        .icon
        .as_ref()
        .and_then(|icon| texture_id(ui, ctx, icon))
        .unwrap_or_else(|| ctx.icons.unknown_entity_texture().id());

    // **`ui.put` dans un ENFANT, jamais sur le `ui` de la rangée** : `Ui::put` ouvre un scope, et un
    // scope avance le curseur du parent — la tuile suivante démarrerait au mauvais endroit. Piège
    // déjà payé sur les tuiles d'alerte, voir `panels::alerts_tab::alert_item`.
    let mut cellule = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    let mut slot = design::item_slot()
        .size(TILE)
        .frame(frame)
        .icon(icon_id)
        // **Le liseré appartient au composant**, plus à ce panneau : peint ici, il tombait au bord
        // du carré avec un coin de 4 px, soit à 2 px du liseré de rareté qu'il devait recouvrir.
        .selected(cochee)
        .log_name(tuile.name.clone());
    if let Some(target) = tuile.target {
        slot = slot.count(design::SlotCount::Target(target));
    }
    cellule.put(rect, slot);

    let ds = design::DesignSystem::get(ui.ctx());

    let mut clic = TileClick::None;
    if select_mode {
        // La case **remplace** la croix : les deux gestes s'excluent, et deux marqueurs dans deux
        // coins d'une tuile de 64 px reviendraient à demander de viser.
        let case = Rect::from_min_size(
            egui::pos2(rect.left() + BADGE_INSET, rect.top() + BADGE_INSET),
            Vec2::splat(design::tokens::CHECKBOX_SIZE),
        );
        let mut etat = cochee;
        cellule.put(
            case,
            design::checkbox(&mut etat, "").log_name(format!("suivi.cocher-{}", tuile.name)),
        );
        let zone = response
            .clone()
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        design::tooltip(&zone).text(format!("{} — cliquer pour cocher", tuile.name));
        if zone.clicked() {
            clic = TileClick::Toggle;
        }
        return clic;
    }

    // **`contains_pointer` et NON `hovered`.** La croix a sa propre zone interactive, posée
    // par-dessus la tuile : dès que le pointeur l'atteint, egui donne le survol à cette zone et
    // `response.hovered()` de la tuile retombe à faux. Avec `hovered`, la croix disparaissait donc
    // à l'instant précis où l'on visait — défaut trouvé sur la planche de survol, jamais sur une
    // capture au repos. `contains_pointer` reste vrai tant que le pointeur est dans la tuile, quel
    // que soit le widget qui a gagné le survol.
    if response.contains_pointer() {
        // Le voile dit le survol ET porte la croix — voir [`TILE_HOVER_SCRIM`].
        ui.painter()
            .rect_filled(rect, design::tokens::ITEM_SLOT_ROUNDING, TILE_HOVER_SCRIM);
        let zone_rect = Rect::from_center_size(
            egui::pos2(
                rect.right() - BADGE_INSET - BADGE / 2.0,
                rect.top() + BADGE_INSET + BADGE / 2.0,
            ),
            Vec2::splat(BADGE + 4.0),
        );
        // **Sa propre zone cliquable, avec sa propre main.** La croix n'est pas un décor peint sur
        // la tuile : elle a un geste à elle, donc un curseur à elle. Elle mange aussi le clic, pour
        // qu'un retrait n'emporte pas au passage le geste de la tuile.
        let croix = ui
            .interact(zone_rect, response.id.with("retirer"), egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        ds.paint_icon(
            ui.painter(),
            Rect::from_center_size(
                zone_rect.center(),
                design::components::icon_button::glyph_fit(
                    ds.icon_native_size(DsIcon::Close),
                    BADGE,
                ),
            ),
            DsIcon::Close,
            // Rouge sous le pointeur — depuis que le retrait ne demande plus confirmation, c'est
            // la croix qui doit dire ce qu'elle fait AVANT le clic.
            if croix.hovered() { REMOVE_HOVER } else { TEXT },
        );
        design::tooltip(&croix).text("Retirer du suivi");
        if croix.clicked() {
            return TileClick::Remove;
        }
        if croix.hovered() {
            return TileClick::None;
        }
    }

    // Le nom vit ici, et nulle part ailleurs sur la tuile.
    design::tooltip(&response).text(match tuile.target {
        Some(target) => format!("{} — décompte depuis {}", tuile.name, target),
        None => format!("{} — incrémental", tuile.name),
    });
    clic
}

/// Le rouage de chargement, centré dans les DEUX axes de la zone que la grille occuperait.
fn loading_row(ui: &mut egui::Ui, inner: Rect) {
    let reste = Rect::from_min_max(
        egui::pos2(inner.left(), ui.cursor().top()),
        inner.right_bottom(),
    );
    let mut zone = ui.new_child(egui::UiBuilder::new().max_rect(reste));
    zone.put(
        Rect::from_center_size(reste.center(), Vec2::splat(design::LoaderSize::Large.px())),
        design::loader().size(design::LoaderSize::Large),
    );
}

// -------------------------------------------------------------------------------------------
// Utilitaires
// -------------------------------------------------------------------------------------------

/// Clé du filtre « Monstres » dans la bande d'autocomplétion.
///
/// **En dehors de l'espace des catégories d'objet**, dont les clés sont des numéros d'icône à un
/// chiffre : deux filtres qui partageraient une clé se fondraient en un seul bouton, et l'un des
/// deux jeux de suggestions deviendrait inatteignable.
const MONSTER_FILTER_KEY: u16 = u16::MAX;

fn category_key(category: overlay_engine::WakfuItemCategory) -> u16 {
    category.icon_number() as u16
}

fn category_label(category: overlay_engine::WakfuItemCategory) -> &'static str {
    use overlay_engine::WakfuItemCategory as C;
    match category {
        C::Equipment => "Équipements",
        C::Resources => "Ressources",
        C::Sublimations => "Sublimations",
        C::Harvests => "Récoltes",
        C::HavenBag => "Havre-sac",
        C::Cosmetics => "Costumes",
        C::Craft => "Artisanat",
        C::Misc => "Divers",
    }
}

/// La texture d'une icône distante, si elle est déjà descendue du CDN — `None` sinon, et l'appelant
/// se peint sans elle.
fn texture_id(
    ui: &egui::Ui,
    ctx: &mut SuiviTabContext<'_>,
    icon: &IconRef,
) -> Option<egui::TextureId> {
    texture(ui, ctx, icon).map(|(id, _)| id)
}

/// La texture ET sa taille native, pour ce qui doit être peint à son rapport — la gemme de rareté.
fn texture(
    ui: &egui::Ui,
    ctx: &mut SuiviTabContext<'_>,
    icon: &IconRef,
) -> Option<(egui::TextureId, Vec2)> {
    ctx.remote_icon_textures
        .resolve(ui.ctx(), ctx.remote_icons, icon)
        .map(|handle| (handle.id(), handle.size_vec2()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_libelle_groupe_dit_tout_quand_rien_n_est_coche() {
        // Une sélection vide se lit « aucune exclusion », pas « rien à faire » — règle du web.
        assert_eq!(bulk_label(0, 12), "Supprimer tout");
        assert_eq!(bulk_label(12, 12), "Supprimer tout");
        assert_eq!(bulk_label(3, 12), "Supprimer (3)");
    }

    #[test]
    fn la_cle_distingue_deux_homonymes_d_id_different() {
        // Deux entrées de même nom et d'id différents doivent pouvoir être cochées et retirées
        // indépendamment — sinon retirer l'une emporterait l'autre.
        let a = WatchlistEntry {
            name: "Larme d'Ogrest".into(),
            kind: WatchlistKind::Item,
            mode: WatchlistMode::Up,
            count: 0,
            countdown_target: 0,
            catalog_id: Some(1),
        };
        let b = WatchlistEntry {
            catalog_id: Some(2),
            ..a.clone()
        };
        assert_ne!(entry_key(&a), entry_key(&b));
    }

    #[test]
    fn le_filtre_monstres_ne_collisionne_avec_aucune_categorie() {
        use overlay_engine::WakfuItemCategory as C;
        for categorie in [
            C::Equipment,
            C::Resources,
            C::Sublimations,
            C::Harvests,
            C::HavenBag,
            C::Cosmetics,
            C::Craft,
            C::Misc,
        ] {
            assert_ne!(category_key(categorie), MONSTER_FILTER_KEY);
        }
    }
}

/// **Ajoute au brouillon les lignes retenues par la fenêtre de recette.**
///
/// Chaque ligne devient un décompte dont la cible est la quantité calculée : c'est tout l'intérêt du
/// geste — « il me faut vingt Peaux », pas « je surveille les Peaux ». Une entrée déjà suivie n'est
/// pas dupliquée ; sa cible est **relevée** à la nouvelle valeur si celle-ci est plus haute, jamais
/// abaissée (deux crafts demandés l'un après l'autre s'additionnent dans l'intention, ils ne
/// s'annulent pas).
///
/// Le formulaire d'ajout revient à son état par défaut, comme après un ajout ordinaire.
pub fn track_recipe_lines(
    entries: &mut Vec<WatchlistEntry>,
    lignes: &[(String, i64, i64)],
    state: &mut SuiviTabState,
) {
    for (name, id, quantity) in lignes {
        let cible = (*quantity).max(TARGET_MIN);
        if let Some(existante) = entries
            .iter_mut()
            .find(|e| e.catalog_id == Some(*id) || e.name == *name)
        {
            existante.mode = WatchlistMode::Down;
            existante.countdown_target = existante.countdown_target.max(cible);
            continue;
        }
        entries.push(WatchlistEntry {
            name: name.clone(),
            kind: WatchlistKind::Item,
            mode: WatchlistMode::Down,
            count: cible,
            countdown_target: cible,
            catalog_id: Some(*id),
        });
    }
    state.recipe = None;
    state.target = TARGET_MIN;
}

#[cfg(test)]
mod recette_tests {
    use super::*;

    #[test]
    fn une_ligne_de_recette_devient_un_decompte_a_sa_quantite() {
        // « Il me faut vingt Peaux » — pas « je surveille les Peaux ».
        let mut entries = Vec::new();
        let mut state = SuiviTabState::default();
        track_recipe_lines(&mut entries, &[("Peau".into(), 30, 20)], &mut state);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].mode, WatchlistMode::Down);
        assert_eq!(entries[0].countdown_target, 20);
    }

    #[test]
    fn une_entree_deja_suivie_voit_sa_cible_relevee_jamais_abaissee() {
        let mut entries = vec![WatchlistEntry {
            name: "Peau".into(),
            kind: WatchlistKind::Item,
            mode: WatchlistMode::Down,
            count: 5,
            countdown_target: 50,
            catalog_id: Some(30),
        }];
        let mut state = SuiviTabState::default();
        // Une demande plus petite ne rabaisse pas la cible : deux crafts demandés l'un après
        // l'autre s'additionnent dans l'intention, ils ne s'annulent pas.
        track_recipe_lines(&mut entries, &[("Peau".into(), 30, 20)], &mut state);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].countdown_target, 50);
        // Une demande plus grande, si.
        track_recipe_lines(&mut entries, &[("Peau".into(), 30, 80)], &mut state);
        assert_eq!(entries[0].countdown_target, 80);
    }

    #[test]
    fn valider_la_recette_referme_sa_fenetre() {
        let mut entries = Vec::new();
        let mut state = SuiviTabState {
            recipe: Some(RecipeDialogState::new(10, "Coiffe".into())),
            ..Default::default()
        };
        track_recipe_lines(&mut entries, &[], &mut state);
        assert!(state.recipe.is_none());
    }
}
