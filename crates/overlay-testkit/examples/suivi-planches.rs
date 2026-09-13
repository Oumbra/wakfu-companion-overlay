//! **Planches de l'onglet « Suivi »** — le VRAI panneau (`overlay_ui::panels::suivi_tab`), rendu
//! dans les états qu'on veut montrer, avec un vrai catalogue et de vraies icônes.
//!
//! **Ce fichier a remplacé une maquette.** `suivi-mockups.rs` a servi à spécifier l'écran avec
//! l'utilisateur en deux itérations, puis l'écran a été porté (2026-09-13) : garder une maquette à
//! côté de son implémentation, c'est entretenir deux vérités qui divergeront. Ces planches
//! appellent `options_modal::show`, donc ce qu'elles montrent **est** ce que l'overlay affiche.
//!
//! Ce qu'elles ajoutent aux captures de non-régression (`tests/panels.rs`) : un catalogue peuplé,
//! les gemmes et icônes de catégorie des fixtures, et les états qui demandent un pointeur ou une
//! touche — survol d'une croix, `Alt` maintenu, panneau de suggestions déplié, fenêtre de recette.
//! Les captures, elles, restent volontairement sur un index vide et un rendu au repos : ce sont
//! deux besoins différents.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`.
//!
//! ```text
//! cargo run -p overlay-testkit --example suivi-planches
//! ```

use egui_kittest::Harness;
use overlay_engine::{CatalogIndex, WatchlistEntry, WatchlistKind, WatchlistMode};
use overlay_ui::panels::options_modal::{
    self, OptionsModalContext, OptionsModalState, OptionsTab, WINDOW_SIZE,
};
use overlay_ui::panels::suivi_tab::{AddMode, RecipeDialogState, SuiviAvailability, SuiviTabState};
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::ui_icons::UiIcons;

/// Le catalogue des planches — assez d'objets pour que la recherche « tofu » rende un panneau
/// crédible, et deux monstres pour que la bande de filtres porte son bouton « Monstres ».
///
/// `[id, fr, en, es, pt, gfxId, rarity_sort, has_recipe, category_sort]` pour les objets,
/// `[id, fr, en, es, pt, gfxId, family, isBoss, isArchi, isDominant]` pour les monstres.
fn catalogue() -> CatalogIndex {
    CatalogIndex::from_compact_json(&serde_json::json!({
        "items": [
            [201, "Plume de Tofu", "Tofu Feather", "Pluma", "Pena", 1201, 1, 0, 1],
            [202, "Coiffe du Tofu", "Tofu Headgear", "Casco", "Elmo", 1202, 2, 1, 0],
            [203, "Cape du Tofu", "Tofu Cape", "Capa", "Capa", 1203, 2, 1, 0],
            [204, "Bec de Tofu", "Tofu Beak", "Pico", "Bico", 1204, 0, 0, 1],
            [205, "Bois de Frêne", "Ash Wood", "Fresno", "Freixo", 1205, 0, 0, 1],
            [206, "Pierre de Lune", "Moonstone", "Piedra", "Pedra", 1206, 4, 0, 1],
            [207, "Griffe de Craqueleur", "Claw", "Garra", "Garra", 1207, 5, 0, 1],
            [208, "Minerai de Fer", "Iron Ore", "Mineral", "Minerio", 1208, 0, 0, 1],
        ],
        "monsters": [
            [301, "Tofu", "Tofu", "Tofu", "Tofu", "2301", -1, 0, 0, 0],
            [302, "Tofu Royal", "Royal Tofu", "Tofu Real", "Tofu Real", "2302", -1, 1, 0, 0],
            [303, "Bouftou", "Gobball", "Jalato", "Jalato", "2303", -1, 0, 0, 0],
            [304, "Chafer Élite", "Elite Chafer", "Chafer", "Chafer", "2304", -1, 0, 1, 0],
        ],
    }))
}

/// Huit suivis, objets et monstres mêlés, les deux modes côte à côte.
fn entrees() -> Vec<WatchlistEntry> {
    fn e(
        name: &str,
        kind: WatchlistKind,
        mode: WatchlistMode,
        target: i64,
        id: i64,
    ) -> WatchlistEntry {
        WatchlistEntry {
            name: name.to_string(),
            kind,
            mode,
            count: target,
            countdown_target: target,
            catalog_id: Some(id),
        }
    }
    vec![
        e(
            "Bois de Frêne",
            WatchlistKind::Item,
            WatchlistMode::Down,
            1000,
            205,
        ),
        e(
            "Plume de Tofu",
            WatchlistKind::Item,
            WatchlistMode::Up,
            0,
            201,
        ),
        e(
            "Pierre de Lune",
            WatchlistKind::Item,
            WatchlistMode::Down,
            50,
            206,
        ),
        e("Bouftou", WatchlistKind::Enemy, WatchlistMode::Up, 0, 303),
        e(
            "Chafer Élite",
            WatchlistKind::Enemy,
            WatchlistMode::Up,
            0,
            304,
        ),
        e(
            "Griffe de Craqueleur",
            WatchlistKind::Item,
            WatchlistMode::Down,
            10,
            207,
        ),
        e(
            "Minerai de Fer",
            WatchlistKind::Item,
            WatchlistMode::Up,
            0,
            208,
        ),
        e(
            "Tofu Royal",
            WatchlistKind::Enemy,
            WatchlistMode::Down,
            3,
            302,
        ),
    ]
}

/// Ce qui distingue une planche d'une autre.
#[derive(Clone, Default)]
struct Planche {
    mode: AddMode,
    target: i64,
    select_mode: bool,
    selected: Vec<String>,
    availability: SuiviAvailability,
    /// Saisie déjà dans le champ — le panneau de suggestions s'ouvre quand elle est renseignée.
    search: String,
    /// La fenêtre de recette, ouverte — et si ses ingrédients sont déjà descendus.
    recipe: Option<bool>,
}

fn ecrire(harness: &mut Harness<'static>, nom: &str) {
    let dir = dossier();
    let image = harness
        .render()
        .expect("rendu offscreen — voir doc de module");
    image
        .save(dir.join(format!("{nom}.png")))
        .expect("écriture de la planche");
    println!("  {nom}");
}

/// Où les planches sont écrites — `target/mockups`, **jamais commité** : ce sont des images
/// jetables, destinées à un artefact, pas des captures de non-régression (celles-là vivent dans
/// `tests/snapshots/`). Le dossier est vidé une fois par exécution, pour qu'une planche renommée
/// n'y survive pas, indiscernable d'une vivante.
fn dossier() -> std::path::PathBuf {
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

/// Ouvre le harnais sur la fenêtre Options, onglet « Suivi », dans l'état demandé.
fn harnais(p: Planche) -> Harness<'static> {
    let recette = p.recipe.map(|resolus| {
        let mut dialogue = RecipeDialogState::new(202, "Coiffe du Tofu".to_string());
        dialogue.quantity = 2;
        if resolus {
            dialogue.ingredients = Some(ingredients());
            // Une ligne dépliée, pour montrer la cascade et les quantités multipliées.
            dialogue.nested.insert("2".to_string());
        }
        dialogue
    });
    let state = OptionsModalState {
        suivi: SuiviTabState {
            mode: p.mode,
            target: if p.target > 0 { p.target } else { 1 },
            select_mode: p.select_mode,
            selected: p.selected,
            search: p.search,
            recipe: recette,
        },
        suivi_draft: Some(entrees()),
        suivi_availability: p.availability,
        path_input: String::new(),
        error: None,
        tab: OptionsTab::Suivi,
        alerts: Default::default(),
        alerts_draft: None,
        alerts_availability: Default::default(),
        initial: Default::default(),
        pending_close: false,
    };
    let etat = std::rc::Rc::new(std::cell::RefCell::new(state));
    let catalog = catalogue();
    // **Les jeux de textures doivent SURVIVRE à la construction** : un `TextureHandle` libère sa
    // texture quand son dernier exemplaire tombe, et la planche sortirait avec des cases vides.
    // Chargés une fois, gardés entre les frames — le store et son cache de textures compris, qui
    // sinon repartiraient à vide à chaque frame et n'auraient jamais rien à servir.
    let mut icons: Option<UiIcons> = None;
    let mut remote: Option<(RemoteIconStore, RemoteIconTextures)> = None;
    Harness::builder()
        .with_size(egui::vec2(WINDOW_SIZE.0, WINDOW_SIZE.1))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            // **Infobulles sans délai** — même motif que le curseur qui ne clignote pas : le
            // harnais rend quelques frames, pas les 0,3 s qu'egui attend avant d'afficher une
            // infobulle. Sans ça, une planche censée en montrer une sort sans elle.
            ui.style_mut().interaction.tooltip_delay = 0.0;
            let icons = icons.get_or_insert_with(|| UiIcons::load(ui.ctx()));
            let (remote_icons, remote_icon_textures) =
                remote.get_or_insert_with(|| (store_prerempli(), RemoteIconTextures::default()));
            options_modal::show(
                ui,
                &mut etat.borrow_mut(),
                &mut OptionsModalContext {
                    catalog: &catalog,
                    remote_icons,
                    remote_icon_textures,
                    icons,
                },
            );
        })
}

/// **Un store d'icônes prérempli depuis les fixtures du harnais** — pas de réseau ici, et une
/// planche qui montre des cases vides ne juge rien.
///
/// `RemoteIconStore::preload` passe par le même décodage que le thread réseau et remplit la même
/// table : le panneau ne fait aucune différence entre une icône descendue du CDN et celle-ci. Les
/// gemmes de rareté et les icônes de catégorie sont de vraies images `wakassets`, versionnées sous
/// `crates/overlay-testkit/fixtures/` ; les images d'objets et de monstres, elles, n'en ont pas —
/// leur `gfxId` est propre au référentiel réel — et gardent le repli générique du Suivi, exactement
/// comme au premier affichage en jeu.
fn store_prerempli() -> RemoteIconStore {
    use overlay_engine::{IconRef, WakfuItemCategory, WakfuRarity};
    let store = RemoteIconStore::empty();

    const GEMMES: [(WakfuRarity, &[u8]); 8] = [
        (
            WakfuRarity::Old,
            include_bytes!("../fixtures/rarities/0.png"),
        ),
        (
            WakfuRarity::Common,
            include_bytes!("../fixtures/rarities/1.png"),
        ),
        (
            WakfuRarity::Rare,
            include_bytes!("../fixtures/rarities/2.png"),
        ),
        (
            WakfuRarity::Mythical,
            include_bytes!("../fixtures/rarities/3.png"),
        ),
        (
            WakfuRarity::Legendary,
            include_bytes!("../fixtures/rarities/4.png"),
        ),
        (
            WakfuRarity::Relic,
            include_bytes!("../fixtures/rarities/5.png"),
        ),
        (
            WakfuRarity::Memory,
            include_bytes!("../fixtures/rarities/6.png"),
        ),
        (
            WakfuRarity::Epic,
            include_bytes!("../fixtures/rarities/7.png"),
        ),
    ];
    for (rarete, octets) in GEMMES {
        store.preload(&IconRef::for_rarity(rarete), octets);
    }

    const CATEGORIES: [(WakfuItemCategory, &[u8]); 8] = [
        (
            WakfuItemCategory::Equipment,
            include_bytes!("../fixtures/item-types/109.png"),
        ),
        (
            WakfuItemCategory::Resources,
            include_bytes!("../fixtures/item-types/226.png"),
        ),
        (
            WakfuItemCategory::Sublimations,
            include_bytes!("../fixtures/item-types/237.png"),
        ),
        (
            WakfuItemCategory::Harvests,
            include_bytes!("../fixtures/item-types/295.png"),
        ),
        (
            WakfuItemCategory::HavenBag,
            include_bytes!("../fixtures/item-types/385.png"),
        ),
        (
            WakfuItemCategory::Cosmetics,
            include_bytes!("../fixtures/item-types/525.png"),
        ),
        (
            WakfuItemCategory::Craft,
            include_bytes!("../fixtures/item-types/602.png"),
        ),
        (
            WakfuItemCategory::Misc,
            include_bytes!("../fixtures/item-types/761.png"),
        ),
    ];
    for (categorie, octets) in CATEGORIES {
        store.preload(&IconRef::for_item_category(categorie), octets);
    }
    store.preload(
        &IconRef::for_all_categories(),
        include_bytes!("../fixtures/item-types/-1.png"),
    );
    store.preload(
        &IconRef::for_monster_category(),
        include_bytes!("../fixtures/item-types/282.png"),
    );
    store
}

/// Les ingrédients de « Coiffe du Tofu », tels que `resolve_recipe` les rendrait.
fn ingredients() -> Vec<overlay_engine::RecipeIngredient> {
    use overlay_engine::{RecipeIngredient, WakfuRarity};
    vec![
        RecipeIngredient {
            name: "Plume de Tofu".into(),
            id: 201,
            rarity: WakfuRarity::Common,
            quantity: 12,
            has_recipe: false,
            children: Vec::new(),
        },
        RecipeIngredient {
            name: "Bec de Tofu".into(),
            id: 204,
            rarity: WakfuRarity::Common,
            quantity: 4,
            has_recipe: false,
            children: Vec::new(),
        },
        RecipeIngredient {
            name: "Cape du Tofu".into(),
            id: 203,
            rarity: WakfuRarity::Rare,
            quantity: 2,
            has_recipe: true,
            children: vec![
                RecipeIngredient {
                    name: "Bois de Frêne".into(),
                    id: 205,
                    rarity: WakfuRarity::Common,
                    quantity: 5,
                    has_recipe: false,
                    children: Vec::new(),
                },
                RecipeIngredient {
                    name: "Minerai de Fer".into(),
                    id: 208,
                    rarity: WakfuRarity::Common,
                    quantity: 2,
                    has_recipe: false,
                    children: Vec::new(),
                },
            ],
        },
    ]
}

fn main() {
    // 1 — L'onglet au repos, mode incrémental : aucune tuile ne porte de compteur, seules celles
    // qui décomptent portent leur cible.
    let mut h = harnais(Planche::default());
    h.run();
    ecrire(&mut h, "suivi_incremental");

    // 2 — Mode décompte : la ligne « Quantité » apparaît, badges et mention « (Alt : retirer) ».
    let mut h = harnais(Planche {
        mode: AddMode::Down,
        target: 250,
        ..Default::default()
    });
    h.run();
    ecrire(&mut h, "suivi_decompte");

    // 3 — `Alt` maintenu : les badges passent en or et retirent. La touche est **réellement
    // enfoncée** dans le harnais, pas simulée par un drapeau de planche — c'est le vrai code qui
    // lit `i.modifiers.alt`.
    let mut h = harnais(Planche {
        mode: AddMode::Down,
        target: 250,
        ..Default::default()
    });
    h.run();
    // **`ModifiersChanged`, et rien d'autre.** C'est le SEUL événement dont egui tire
    // `InputState::modifiers` (`input_state/mod.rs`) — celui que `i.modifiers.alt` lit dans le vrai
    // code, et celui qu'`egui-winit` émet quand la touche est enfoncée ou relâchée. Un
    // `Event::Key { modifiers: ALT }` porte bien le modificateur, mais **seulement pour cette
    // touche-là** : il ne change pas l'état global, et cette planche sortait donc identique à la
    // précédente, badges positifs compris. Défaut de planche relevé par l'utilisateur le
    // 2026-09-13 — le code, lui, était juste.
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::ALT));
    h.run();
    ecrire(&mut h, "suivi_decompte_alt");

    // 4 — Le champ déplié : objets ET monstres, la bande porte « Monstres », et le marteau des deux
    // objets craftables attend son survol.
    let mut h = harnais(Planche {
        search: "tofu".to_string(),
        ..Default::default()
    });
    h.run();
    // Le champ doit avoir le focus pour que le panneau s'ouvre — un clic réel, comme un joueur.
    let champ = egui::pos2(300.0, 300.0);
    h.drag_at(champ);
    h.run();
    h.drop_at(champ);
    h.run();
    ecrire(&mut h, "suivi_autocompletion");

    // 5 — Le marteau survolé : il prend sa couleur pleine et montre son infobulle.
    let mut h = harnais(Planche {
        search: "tofu".to_string(),
        ..Default::default()
    });
    h.run();
    h.drag_at(champ);
    h.run();
    h.drop_at(champ);
    h.run();
    // Deuxième rangée (« Cape du Tofu »), colonne de droite : le marteau.
    //
    // `run_steps` après le survol : l'infobulle du design system apparaît en fondu, donc demande un
    // repeint tant qu'elle s'anime — `run`, qui boucle jusqu'à stabilité, la manquerait ou
    // tournerait jusqu'à sa limite de pas.
    h.hover_at(egui::pos2(686.0, 300.0 + 25.0 + 38.0 + 35.0 + 17.0));
    h.run_steps(6);
    ecrire(&mut h, "suivi_recette_survolee");

    // 6 — Le survol d'une tuile, souris SUR LA CROIX : elle vire au rouge.
    //
    // Deux survols, dans cet ordre : la croix n'existe que sur une tuile déjà survolée, donc le
    // premier la fait apparaître et le second se pose dessus. C'est exactement ce que fait un vrai
    // pointeur qui entre dans la tuile puis glisse vers son coin.
    let mut h = harnais(Planche::default());
    h.run();
    h.hover_at(egui::pos2(79.0, 394.0));
    h.run();
    h.hover_at(egui::pos2(100.0, 373.0));
    h.run();
    ecrire(&mut h, "suivi_survol_retrait");

    // 7 & 8 — La sélection multiple, partielle puis vide.
    let mut h = harnais(Planche {
        select_mode: true,
        selected: vec![
            "Plume de Tofu::201".to_string(),
            "Bouftou::303".to_string(),
            "Minerai de Fer::208".to_string(),
        ],
        ..Default::default()
    });
    h.run();
    ecrire(&mut h, "suivi_selection_partielle");

    let mut h = harnais(Planche {
        select_mode: true,
        ..Default::default()
    });
    h.run();
    ecrire(&mut h, "suivi_selection_vide");

    // 9 & 10 — La fenêtre de recette, pendant puis après la résolution réseau.
    //
    // **`run_steps` et non `run`** partout où un rouage tourne : `Harness::run` boucle jusqu'à ce
    // que l'interface cesse de demander un repeint, et un loader n'en cesse jamais — c'est
    // justement ce qu'il est. Trois pas suffisent à le poser sur une image stable.
    let mut h = harnais(Planche {
        recipe: Some(false),
        ..Default::default()
    });
    h.run_steps(3);
    ecrire(&mut h, "suivi_recette_chargement");

    let mut h = harnais(Planche {
        recipe: Some(true),
        ..Default::default()
    });
    h.run();
    ecrire(&mut h, "suivi_recette");

    // 11 — La liste en cours de lecture depuis le compte (rouage, voir plus haut).
    let mut h = harnais(Planche {
        availability: SuiviAvailability::Loading,
        ..Default::default()
    });
    h.run_steps(3);
    ecrire(&mut h, "suivi_chargement");

    let dir = dossier();
    let ecrites = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    let affiche = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("{ecrites} planches écrites dans {}", affiche.display());
}
