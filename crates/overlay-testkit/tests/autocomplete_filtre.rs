//! **Le filtre de catégorie de `design::autocomplete` survit à la session** — règle 6 du composant,
//! demandée le 2026-09-15 : « enregistrer, pour la session (tant que l'overlay n'est pas fermé),
//! les filtres de catégorie sélectionnés par l'utilisateur ».
//!
//! Ce que ce fichier prouve, et qu'aucune capture ne montrerait (le filtre mémorisé ne se voit que
//! sur la recherche SUIVANTE) :
//!
//! 1. un filtre posé reste posé après une sélection, alors que le composant vidait le champ ET le
//!    filtre jusque-là ;
//! 2. il survit à un changement de l'identifiant AUTOMATIQUE du champ — c'est toute la raison
//!    d'être de la clé tirée de `log_name` : l'id d'egui dépend du rang du widget dans son parent,
//!    et un widget de plus au-dessus du champ suffisait à effacer la mémoire ;
//! 3. il se relâche de lui-même quand sa catégorie quitte la bande, le garde-fou qui remplace la
//!    remise à zéro du web — sans lui, une recherche sans rapport n'afficherait qu'un panneau vide
//!    dont le bouton de relâchement n'est même plus peint.
//!
//! Tout se joue au CLIC et à la FRAPPE, jamais en posant l'état à la main : c'est le geste de
//! l'utilisateur qui doit produire la mémorisation, et un test qui appellerait `preview_filter` ne
//! prouverait rien de ce qui est demandé ici.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use std::cell::RefCell;
use std::rc::Rc;

use egui_kittest::Harness;
use overlay_ui::design::{self, tokens, AutocompleteEntry, AutocompleteFilter};

// Les icônes de catégorie et les gemmes que le runtime télécharge du CDN `wakassets` — ici des PNG
// embarqués, comme la galerie (voir la doc de `wakassets_fixtures`). Sans elles, la bande de
// filtres se capture VIDE : ses boutons n'ont que leur image, et une planche de trois carrés
// beiges ne dirait rien de ce que ce test démontre.
#[path = "../examples/shared/wakassets_fixtures.rs"]
mod wakassets_fixtures;

use wakassets_fixtures::{CategoryFilter, CategoryIcons, ItemIcons, RarityGems, GEM_NATIVE};

/// Ce que le test pilote d'une frame à l'autre, et ce qu'il relit après coup.
struct Etat {
    saisie: String,
    /// Les suggestions de la frame, en `(libellé, catégorie)` — le test les remplace pour jouer
    /// une recherche différente, comme le ferait le catalogue. Les textures, elles, sont résolues
    /// dans la frame : elles n'existent pas avant le premier `Context`.
    suggestions: Vec<(String, u16)>,
    /// La bande, calculée par l'appelant sur la liste NON filtrée (règle 4) — les catégories
    /// PRÉSENTES dans `suggestions`, et rien d'autre.
    bande: Vec<u16>,
    /// Ce que l'utilisateur a déjà ajouté : l'appelant grise ces entrées et les mentionne, comme
    /// l'onglet Alertes le fait de ses objets déjà surveillés. Le test s'en sert autant pour
    /// rester réaliste que pour ses assertions — une entrée grisée n'est pas sélectionnable, donc
    /// chaque phase choisit forcément une entrée NEUVE.
    deja: Vec<usize>,
    /// **Un widget de plus AVANT le champ** — il décale l'identifiant automatique d'egui, et c'est
    /// exactement ce que la clé de mémorisation doit encaisser.
    widget_avant: bool,
    /// Le rectangle du champ tel qu'il vient d'être peint : toutes les positions de clic en
    /// dérivent, plutôt que d'être des constantes à réajuster au moindre changement de gabarit.
    champ: egui::Rect,
    /// La dernière entrée choisie — l'indice rendu par le composant, donc celui de `entrees`.
    choisi: Option<usize>,
}

/// Les deux catégories de la démonstration, du vocabulaire du web : la `1` est « Équipements », la
/// `2` « Ressources ». Ce sont des clés opaques pour le composant, qui ne fait que les égaler.
const EQUIPEMENTS: u16 = 1;
const RESSOURCES: u16 = 2;

fn icone(categorie: u16) -> CategoryFilter {
    match categorie {
        EQUIPEMENTS => CategoryFilter::Equipment,
        _ => CategoryFilter::Resources,
    }
}

/// Ce que l'appelant construit à chaque frame : des entrées et une bande, images comprises.
fn habille(
    suggestions: &[(String, u16)],
    bande: &[u16],
    deja: &[usize],
    cats: &CategoryIcons,
    gems: &RarityGems,
    objets: &ItemIcons,
) -> (Vec<AutocompleteEntry>, Vec<AutocompleteFilter>) {
    let entrees = suggestions
        .iter()
        .enumerate()
        .map(|(rang, (label, categorie))| {
            let mut entry = AutocompleteEntry::new(label.clone(), *categorie);
            entry.gem = Some(gems.texture_id(overlay_engine::WakfuRarity::Rare));
            entry.gem_size = GEM_NATIVE;
            entry.image = Some(objets.texture_id(rang));
            if deja.contains(&rang) {
                entry.disabled = true;
                entry.mention = Some("déjà dans vos alertes".to_owned());
            }
            entry
        })
        .collect();
    let mut filtres = vec![AutocompleteFilter::all(
        "Toutes les catégories",
        Some(cats.texture_id(CategoryFilter::All)),
    )];
    for &c in bande {
        filtres.push(AutocompleteFilter::category(
            c,
            icone(c).label(),
            Some(cats.texture_id(icone(c))),
        ));
    }
    (entrees, filtres)
}

/// Charge une fixture une seule fois et la garde en mémoire egui : un `TextureHandle` libère sa
/// texture quand le dernier exemplaire tombe, et un chargement par frame peindrait des cases vides
/// (même précaution que `tests/design_gallery.rs`).
fn charge_une_fois<T: Clone + Send + Sync + 'static>(
    ui: &egui::Ui,
    cle: &'static str,
    charge: impl FnOnce(&egui::Context) -> T,
) -> T {
    let id = egui::Id::new(cle);
    if let Some(valeur) = ui.data(|d| d.get_temp::<T>(id)) {
        return valeur;
    }
    let valeur = charge(ui.ctx());
    ui.data_mut(|d| d.insert_temp(id, valeur.clone()));
    valeur
}

/// Le centre du `i`-ième bouton de la bande, déduit des jetons — `i = 0` est « Tout ».
fn centre_filtre(champ: egui::Rect, i: usize) -> egui::Pos2 {
    egui::pos2(
        champ.left()
            + tokens::AUTOCOMPLETE_PANEL_PAD
            + tokens::AUTOCOMPLETE_FILTER_BAR_PAD
            + i as f32 * (tokens::AUTOCOMPLETE_FILTER_BUTTON + tokens::AUTOCOMPLETE_FILTER_GAP)
            + tokens::AUTOCOMPLETE_FILTER_BUTTON / 2.0,
        champ.bottom()
            + tokens::AUTOCOMPLETE_PANEL_GAP
            + tokens::AUTOCOMPLETE_PANEL_PAD
            + tokens::AUTOCOMPLETE_FILTER_BAR_HEIGHT / 2.0,
    )
}

/// Le centre de la `rang`-ième rangée VISIBLE, sur son libellé — loin de l'action de droite, qui
/// est un autre geste (`action_on`, jamais `selected`).
fn centre_rangee(champ: egui::Rect, rang: usize) -> egui::Pos2 {
    egui::pos2(
        champ.left() + 120.0,
        champ.bottom()
            + tokens::AUTOCOMPLETE_PANEL_GAP
            + tokens::AUTOCOMPLETE_PANEL_PAD
            + tokens::AUTOCOMPLETE_FILTER_BAR_HEIGHT
            + rang as f32 * tokens::AUTOCOMPLETE_ROW_HEIGHT
            + tokens::AUTOCOMPLETE_ROW_HEIGHT / 2.0,
    )
}

/// Un clic complet : survol, appui, relâchement, chacun dans sa frame — le geste réel. egui
/// rattache l'appui au widget survolé, et l'appui retire le focus au champ : garder les trois dans
/// la même frame masquerait la moitié des régressions de ce composant (voir `panels.rs`).
fn clique(harness: &mut Harness<'_>, pos: egui::Pos2) {
    harness.hover_at(pos);
    harness.run();
    harness.drag_at(pos);
    harness.run();
    harness.drop_at(pos);
    harness.run();
}

/// Clique dans le champ pour lui donner le focus, puis frappe `texte` caractère par caractère.
fn cherche(harness: &mut Harness<'_>, champ: egui::Rect, texte: &str) {
    clique(harness, champ.center());
    for c in texte.chars() {
        harness.event(egui::Event::Text(c.to_string()));
    }
    harness.run();
}

#[test]
fn le_filtre_de_categorie_reste_pose_pour_toute_la_session() {
    // Quatre suggestions, deux catégories, et la PREMIÈRE d'une autre catégorie que les trois
    // suivantes : c'est ce qui rend chaque rangée discriminante. L'indice rendu par le composant
    // dit alors à lui seul quel filtre était réellement actif, sans avoir à lire un état interne —
    // et chaque phase en consomme une, puisqu'une entrée déjà ajoutée n'est plus sélectionnable.
    let etat = Rc::new(RefCell::new(Etat {
        suggestions: vec![
            ("Bottes du Bouftou".to_owned(), EQUIPEMENTS),
            ("Bois de Bouftou".to_owned(), RESSOURCES),
            ("Blé du Bouftou".to_owned(), RESSOURCES),
            ("Bourgeon de Bouftou".to_owned(), RESSOURCES),
        ],
        bande: vec![EQUIPEMENTS, RESSOURCES],
        deja: Vec::new(),
        saisie: String::new(),
        widget_avant: false,
        champ: egui::Rect::ZERO,
        choisi: None,
    }));

    let vu = Rc::clone(&etat);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 400.0))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            let gems = charge_une_fois(ui, "test.gemmes", RarityGems::load);
            let cats = charge_une_fois(ui, "test.categories", CategoryIcons::load);
            let objets = charge_une_fois(ui, "test.objets", ItemIcons::load);
            let mut e = vu.borrow_mut();
            if e.widget_avant {
                ui.label("un widget de plus, qui décale l'id automatique du champ");
            }
            let (entrees, filtres) =
                habille(&e.suggestions, &e.bande, &e.deja, &cats, &gems, &objets);
            let issue = design::autocomplete(&mut e.saisie)
                .placeholder("Ajouter un objet à surveiller…")
                .width(400.0)
                .entries(&entrees)
                .filters(&filtres)
                .log_name("test.autocomplete-filtre")
                .show(ui);
            e.champ = issue.response.rect;
            if let Some(index) = issue.selected {
                e.choisi = Some(index);
                // Ce que ferait l'appelant réel : l'objet ajouté est désormais « déjà dans vos
                // alertes », donc grisé à la recherche suivante.
                e.deja.push(index);
            }
        });
    harness.run();

    // --- 1. Un filtre posé, une sélection : le filtre DOIT rester ---------------------------
    let champ = etat.borrow().champ;
    cherche(&mut harness, champ, "bou");
    assert_eq!(
        etat.borrow().saisie,
        "bou",
        "le champ n'a pas reçu la frappe — il n'avait donc pas le focus après le clic"
    );

    // Le troisième bouton de la bande : « Tout », « Équipements », puis « Ressources ».
    clique(&mut harness, centre_filtre(champ, 2));
    // **Ce que le clic laisse à l'écran** : le bouton « Ressources » encadré, les deux seules
    // ressources en liste — et le curseur de saisie DE RETOUR dans le champ. Ce dernier point est
    // le correctif du 2026-09-15 : jusque-là le clic défocalisait le champ, et la capture prise
    // ici ne montrait plus qu'un champ nu, panneau refermé.
    harness.snapshot("autocomplete_filtre_actif");
    // Filtre posé : la première rangée n'est plus « Bottes » (catégorie 1) mais « Bois ».
    clique(&mut harness, centre_rangee(champ, 0));
    assert_eq!(
        etat.borrow().choisi,
        Some(1),
        "le filtre de catégorie n'a pas restreint la liste — la première rangée aurait dû être « Bois »"
    );
    assert!(
        etat.borrow().saisie.is_empty(),
        "le champ doit se vider après une sélection (règle 5)"
    );

    // --- 2. La recherche suivante retrouve le filtre ----------------------------------------
    let champ = etat.borrow().champ;
    cherche(&mut harness, champ, "bou");
    // **La démonstration de la règle 6** : une recherche NEUVE, et le filtre est toujours posé —
    // « Bottes du Bouftou » reste hors liste sans que l'utilisateur ait eu à recliquer quoi que ce
    // soit, et l'objet du tour précédent porte sa mention « déjà dans vos alertes ». Avant ce
    // changement, la même capture aurait montré les quatre entrées, filtre relâché.
    harness.snapshot("autocomplete_filtre_memorise");
    // Deuxième rangée : « Blé » si le filtre tient toujours, « Bois » (déjà ajouté, donc
    // insélectionnable) s'il est retombé à « Tout ».
    clique(&mut harness, centre_rangee(champ, 1));
    assert_eq!(
        etat.borrow().choisi,
        Some(2),
        "le filtre n'a pas survécu à la sélection — la deuxième rangée était celle de la liste non filtrée"
    );

    // --- 3. …et à un changement de l'identifiant automatique du champ -----------------------
    etat.borrow_mut().widget_avant = true;
    harness.run();
    let champ = etat.borrow().champ;
    assert!(
        champ.top() > 20.0,
        "le widget ajouté devait pousser le champ vers le bas : {champ:?}"
    );
    cherche(&mut harness, champ, "bou");
    // Troisième rangée : « Bourgeon », la seule ressource encore libre. Sans le filtre, ce rang
    // porterait « Blé », déjà ajouté au tour précédent — donc rien ne serait choisi.
    clique(&mut harness, centre_rangee(champ, 2));
    assert_eq!(
        etat.borrow().choisi,
        Some(3),
        "la mémoire du filtre a suivi l'id automatique du champ au lieu de son log_name"
    );

    // --- 4. Le garde-fou : la catégorie quitte la bande, le filtre se relâche ---------------
    {
        // Une recherche sans rapport, où la catégorie 2 n'apparaît plus : la bande ne porte plus
        // son bouton, donc rien ne dirait à l'utilisateur ce qui vide son panneau.
        let mut e = etat.borrow_mut();
        e.suggestions = vec![
            ("Épée de Boufton".to_owned(), EQUIPEMENTS),
            ("Cape de Boufton".to_owned(), EQUIPEMENTS),
        ];
        e.bande = vec![EQUIPEMENTS];
        e.deja.clear();
    }
    harness.run();
    let champ = etat.borrow().champ;
    cherche(&mut harness, champ, "bouf");
    // **Le garde-fou en image** : la bande a repris sa forme de recherche neuve — « Tout » actif,
    // les deux équipements en liste. Sans lui, le panneau afficherait « Aucun résultat dans cette
    // catégorie » pour un filtre dont le bouton n'est même plus peint.
    harness.snapshot("autocomplete_filtre_relache");
    clique(&mut harness, centre_rangee(champ, 0));
    assert_eq!(
        etat.borrow().choisi,
        Some(0),
        "le filtre mémorisé aurait dû se relâcher : sa catégorie n'est plus dans la bande, le panneau était donc vide"
    );
}
