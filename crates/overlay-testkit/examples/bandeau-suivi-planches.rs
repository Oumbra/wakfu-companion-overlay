//! **Maquette des deux commandes du bandeau de suivi** — le « + » qui ouvre la fenêtre Options sur
//! l'onglet « Suivi », et le « − » qui ouvre une sélection multiple dans le bandeau lui-même.
//!
//! Demande du 2026-09-13, dans la foulée du portage de l'onglet Suivi. Les deux boutons existent
//! depuis le 2026-09-06 et sont **inertes** depuis : leur infobulle promet une action qui n'arrive
//! pas (`panels::watchlist`, doc de module — « aucun formulaire d'ajout, aucune sélection ne sont
//! câblés côté overlay pour cette itération »). Maintenant que l'écran d'édition existe, le « + »
//! a une destination, et le « − » un sens.
//!
//! ## Ce que cette maquette peint, et ce qu'elle emprunte
//!
//! Le bandeau lui-même est le **vrai panneau** (`panels::watchlist::show`) : les tuiles, le carré
//! de contrôle, les compteurs, tout vient du code. Ce que la maquette superpose est **ce qui
//! n'existe pas encore** — les cases à cocher des tuiles, le liseré des tuiles cochées, l'état
//! enfoncé du « − », et le bouton de suppression groupée sous la bande. Les positions sont
//! recalculées à partir des jetons du panneau, recopiés ici avec leur provenance : ils sont privés
//! à `panels::watchlist`, et une maquette n'est pas une raison de les rendre publics.
//!
//! ## Les quatre décisions à valider
//!
//! 1. **Le « + » n'ajoute pas sur place : il ouvre la fenêtre Options sur l'onglet « Suivi ».**
//!    C'est déjà ce que fait le bouton « Options » du même carré, à un onglet près — et il n'y a
//!    aucune place pour un champ d'autocomplétion et son panneau de suggestions dans un bandeau de
//!    132 px de haut posé par-dessus le jeu. **Les quatre libellés d'infobulle ne bougent pas** :
//!    une première version de cette maquette les rallongeait pour annoncer la destination
//!    (« Ajouter un suivi dans les Options »), l'utilisateur les a explicitement gardés tels quels
//!    le 2026-09-13. Les planches 2 et 3 les montrent donc rendus par le panneau lui-même, survol
//!    simulé — aucun texte n'est peint par cette maquette. Elles montrent au passage, sans rien en
//!    demander, que « Ajouter (Ctrl+Shift+A) » ne tient pas dans les 88 px de
//!    `CONTROL_TOOLTIP_RESERVE` et se replie à droite dès que la fenêtre est assez large pour l'y
//!    accueillir — c'est-à-dire dès qu'il y a plus de deux suivis. Comportement d'aujourd'hui,
//!    antérieur à cette maquette et hors de son périmètre.
//! 2. **Le « − » ouvre un MODE, il ne supprime rien.** Même geste que l'onglet Suivi : chaque tuile
//!    gagne une case à cocher, et un bouton de suppression groupée apparaît. Recliquer le « − »
//!    quitte le mode — le bouton reste donc enfoncé tant qu'il est ouvert, comme un onglet actif.
//! 3. **Le bouton de suppression est SOUS la bande, centré sur les TUILES.** Demande explicite, et
//!    « centré » veut bien dire centré sur la rangée de suivis, pas sur la fenêtre : celle-ci porte
//!    aussi le carré de contrôle et ses deux réserves d'infobulle, qui la déséquilibrent vers la
//!    gauche — un bouton centré sur elle tombait visiblement à côté de la rangée qu'il commande.
//!    Le bouton ne peut pas vivre dans le carré de contrôle (24 px de côté, déjà plein à quatre
//!    boutons) ni à côté des tuiles (la bande défile horizontalement, il sortirait du champ). Sous
//!    la bande, il est à portée immédiate et ne pousse rien.
//! 4. **En style danger, comme sur le web et comme dans l'onglet.** Décision déjà rendue le
//!    2026-09-13 pour l'onglet Suivi, reprise telle quelle.
//!
//! ## ⚠ Portée depuis — cette maquette est un document d'archive
//!
//! Les quatre décisions ci-dessous ont été validées, puis portées dans le panneau lui-même le
//! 2026-09-13 (`panels::watchlist::WatchlistSelection`). Ce que ce fichier superpose à la main —
//! cases à cocher, liseré, « − » enfoncé, bouton de suppression — **existe maintenant pour de
//! bon** : la référence est le panneau, et les captures qui font foi sont celles du test
//! `panneau_suivi_le_bouton_moins_ouvre_la_selection_multiple` (`tests/panels.rs`), qui clique
//! réellement au lieu de dessiner par-dessus. Ce fichier n'est gardé que pour l'historique de la
//! décision ; il peint toujours sur un bandeau dont la sélection est fermée, donc son rendu reste
//! juste, simplement redondant.
//!
//! ## Ce que ce portage demandera
//!
//! - **La fenêtre Suivi grandit quand le mode est ouvert.** Sa hauteur est une constante
//!   (`main.rs::WATCHLIST_HEIGHT`, 132 px) à laquelle l'hôte ajoute déjà la bande de toast quand il
//!   y en a un (`watchlist_target_height`). Le bouton demande le même traitement : une hauteur de
//!   plus quand la sélection est ouverte, et la fenêtre qui se rétracte en la quittant.
//! - **`WatchlistOutcome` gagne deux intentions** : ouvrir les Options *sur un onglet donné*, et
//!   retirer un lot d'entrées. Le panneau ne fait toujours aucun effet de bord (§17.3 bis du plan).
//! - **`OptionsModalState` doit pouvoir s'ouvrir sur un onglet choisi.** Il s'ouvre aujourd'hui sur
//!   le défaut d'`OptionsTab`, qui est justement « Suivi » — donc rien à écrire tant que c'est le
//!   cas, mais l'hôte doit le poser explicitement plutôt que d'en dépendre.
//! - **Le retrait passe par le même chemin que la validation de l'onglet** : les définitions
//!   restantes au moteur, qui garde ses compteurs et réplique au compte.
//! - **Le centre du bouton se prend sur la zone des tuiles, pas sur la fenêtre.** Ici les huit
//!   suivis tiennent d'un coup et cette zone est la rangée entière ; dès qu'elle défile, c'est la
//!   fenêtre visible de la `ScrollArea` qu'il faut centrer — son `Rect` est connu du panneau, qui
//!   n'a donc rien à mesurer.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`.
//!
//! ```text
//! cargo run -p overlay-testkit --example bandeau-suivi-planches
//! ```

use egui::{Rect, Vec2};
use egui_kittest::Harness;
use overlay_engine::{CatalogIndex, WatchlistEntry, WatchlistKind, WatchlistMode};
use overlay_ui::design::{self, ButtonSize, ButtonVariant, DsIcon, IconContext};
use overlay_ui::panels::combat::{CombatMetric, CombatSide};
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content::{paint_content, OverlayKind, RenderContent};
use overlay_ui::shortcuts::ShortcutBindings;
use overlay_ui::ui_icons::UiIcons;

// -------------------------------------------------------------------------------------------
// Jetons — recopiés de `panels::watchlist`, où ils sont privés. Chacun garde son nom d'origine
// pour que la correspondance soit immédiate à la relecture.
// -------------------------------------------------------------------------------------------

/// `panels::watchlist::TILE_SIZE` — l'emplacement d'objet du jeu.
const TILE_SIZE: f32 = design::tokens::ITEM_SLOT_SIZE;
/// `panels::watchlist::TILE_GAP`.
const TILE_GAP: f32 = 12.0;
/// `panels::watchlist::CONTROL_BUTTON_SIZE`.
const CONTROL_BUTTON_SIZE: f32 = 24.0;
/// `panels::watchlist::CONTROL_BUTTON_GAP`.
const CONTROL_BUTTON_GAP: f32 = 4.0;
/// **Ton de la sélection du bandeau** — elle ne sert qu'à retirer, donc le rouge.
///
/// `SelectionTone` est né de cette maquette (2026-09-13) : l'onglet Suivi et ce bandeau
/// sélectionnent tous deux pour supprimer, mais le composant doit savoir faire les deux — une
/// sélection qui servirait à autre chose garde l'or.
const TON: design::SelectionTone = design::SelectionTone::Danger;

/// `panels::watchlist::CONTROL_TOOLTIP_RESERVE` — la place gardée AUTOUR du carré de contrôle pour
/// que les infobulles s'y ouvrent centrées (88 px jusqu'au 2026-09-13, quand elles s'ouvraient sur
/// le côté ; 48 px ensuite). **Nulle à GAUCHE depuis le même jour** : l'overlay démarre au premier
/// pixel des boutons, et les infobulles s'y rabattent alignées plutôt que centrées (demande
/// utilisateur explicite).
const CONTROL_TOOLTIP_RESERVE: f32 = 0.0;
/// `render_content::paint_content` pose cette marge autour du contenu du bandeau — **sauf à
/// GAUCHE**, nulle depuis le 2026-09-13 (voir [`CONTROL_TOOLTIP_RESERVE`]).
const CONTENT_MARGIN_LEFT: f32 = 0.0;
/// La même, sur les trois autres côtés.
const CONTENT_MARGIN: f32 = 6.0;
/// **Nulle en HAUT** depuis le soir du 2026-09-13, comme à gauche : les infobulles s'ouvrent
/// toutes en dessous (leur réserve est passée sous la bande,
/// `render_content::WATCHLIST_TOOLTIP_RESERVE`) et la barre de défilement est passée au-dessus des
/// tuiles, où elle ne prend de place que lorsqu'elle sert. Les planches de ce fichier ne débordent
/// jamais : elles n'ont donc pas de barre, et rien au-dessus du carré de contrôle.
const CONTENT_TOP_MARGIN: f32 = 0.0;
/// La marge fixe qu'`egui_kittest` ajoute autour de tout harnais `new_ui`.
const HARNESS_MARGIN: f32 = 8.0;

/// Coin haut-gauche du bouton « − », dans le repère du harnais.
///
/// **Calculé, et vérifiable** : c'est exactement le repère que documente
/// `panels.rs::panneau_suivi_toutes_les_infobulles_sous_la_bande`, qui survole ces quatre boutons
/// à des coordonnées écrites à la main et tient depuis. Le contenu du bandeau commence à
/// `HARNESS_MARGIN + CONTENT_MARGIN` sur les deux axes ; le carré de contrôle vient après la
/// réserve d'infobulle gauche, et ses boutons après sa marge intérieure.
///
/// **Une première version déduisait ce point de [`TILE_ORIGIN`]** — « le carré finit `TILE_GAP`
/// avant la première tuile, et il est centré sur leur rangée » — et la seconde moitié de cette
/// phrase est fausse : le carré est aligné en HAUT du contenu, pas centré sur les tuiles. Le « − »
/// repeint sortait 2 px trop à gauche et 4 px trop bas, ce qui se voyait comme un bouton qui saute
/// en entrant dans le mode sélection — signalé par l'utilisateur sur la planche.
const MINUS_TOP_LEFT: egui::Pos2 = egui::pos2(
    HARNESS_MARGIN
        + CONTENT_MARGIN_LEFT
        + CONTROL_TOOLTIP_RESERVE
        + CONTROL_BUTTON_GAP
        + CONTROL_BUTTON_SIZE
        + CONTROL_BUTTON_GAP,
    HARNESS_MARGIN + CONTENT_TOP_MARGIN + CONTROL_BUTTON_GAP,
);

/// Coin haut-gauche du bouton « + » — voir [`MINUS_TOP_LEFT`], dont il ne diffère qu'en abscisse.
const PLUS_TOP_LEFT: egui::Pos2 = egui::pos2(
    HARNESS_MARGIN + CONTENT_MARGIN_LEFT + CONTROL_TOOLTIP_RESERVE + CONTROL_BUTTON_GAP,
    MINUS_TOP_LEFT.y,
);

/// Hauteur que la sélection ajoute sous la bande : l'écart, le bouton, l'écart.
///
/// **C'est elle qui agrandit la fenêtre** — voir la doc de module. 28 px de bouton, comme celui de
/// l'onglet Suivi, plus 8 px de part et d'autre.
const BULK_ROW_HEIGHT: f32 = 8.0 + 28.0 + 8.0;

/// Huit suivis, objets et monstres mêlés — de quoi remplir la bande sans la faire défiler.
fn entrees() -> Vec<WatchlistEntry> {
    fn e(
        name: &str,
        kind: WatchlistKind,
        mode: WatchlistMode,
        count: i64,
        target: i64,
    ) -> WatchlistEntry {
        WatchlistEntry {
            name: name.to_string(),
            kind,
            mode,
            count,
            countdown_target: target,
            catalog_id: None,
        }
    }
    vec![
        e(
            "Bois de Frêne",
            WatchlistKind::Item,
            WatchlistMode::Down,
            340,
            1000,
        ),
        e(
            "Fleur de Kalé",
            WatchlistKind::Item,
            WatchlistMode::Up,
            62,
            0,
        ),
        e(
            "Pierre de Lune",
            WatchlistKind::Item,
            WatchlistMode::Down,
            7,
            50,
        ),
        e("Bouftou", WatchlistKind::Enemy, WatchlistMode::Up, 128, 0),
        e(
            "Larve Bleue",
            WatchlistKind::Enemy,
            WatchlistMode::Up,
            41,
            0,
        ),
        e(
            "Plume de Tofu",
            WatchlistKind::Item,
            WatchlistMode::Down,
            18,
            100,
        ),
        e(
            "Chafer Élite",
            WatchlistKind::Enemy,
            WatchlistMode::Up,
            9,
            0,
        ),
        e(
            "Minerai de Fer",
            WatchlistKind::Item,
            WatchlistMode::Up,
            806,
            0,
        ),
    ]
}

/// Le libellé du bouton groupé — **la règle de l'onglet Suivi**, reprise telle quelle : une
/// sélection vide se lit « aucune exclusion », donc « Supprimer tout ».
fn bulk_label(selected: usize, total: usize) -> String {
    if selected == 0 || selected == total {
        "Supprimer tout".to_string()
    } else {
        format!("Supprimer ({selected})")
    }
}

/// Coin haut-gauche de la PREMIÈRE tuile, dans le repère du harnais — **mesuré sur la planche de
/// repos**, pas calculé.
///
/// Méthode : sur `bandeau_repos.png`, les deux liserés d'une même tuile encadrent la ligne médiane
/// en x = 172..173 et x = 230..231, soit 60 px d'un liseré à l'autre pour un carré de 64 — donc un
/// bord à 170, le liseré étant posé 2 px à l'intérieur (`design::item_slot::border_ring`). Même
/// lecture en ordonnée : liseré à y = 16, bord à 14.
///
/// **La première version relevait 172 et 16, c'est-à-dire le LISERÉ pris pour le bord** — d'où des
/// cases à cocher et un liseré de sélection décalés de 2 px sur les deux axes. Une mesure sur un
/// rendu ne dit que ce qu'on lui demande : « la première colonne claire » n'est pas « le bord de la
/// tuile » dès lors que le cadre du jeu flotte à l'intérieur de son carré.
///
/// La somme des jetons (`8 + 6 + 48 + 60 + 12`) donnerait 134 : les 4 px d'écart viennent de ce que
/// la `ScrollArea` et `paint_content` posent entre eux, et qu'aucun jeton public ne décrit. C'est
/// bien la mesure qui fait foi ici — mais pour les TUILES seulement. Le carré de contrôle, lui, se
/// calcule depuis le bord du contenu : voir [`MINUS_TOP_LEFT`], et le bug qu'a coûté la déduction
/// inverse.
const TILE_ORIGIN: egui::Pos2 = egui::pos2(130.0, 8.0 + CONTENT_TOP_MARGIN);

/// L'abscisse du bord gauche de la `n`-ième tuile — voir [`TILE_ORIGIN`].
fn tile_left(index: usize) -> f32 {
    TILE_ORIGIN.x + index as f32 * (TILE_SIZE + TILE_GAP)
}

/// Ce qui distingue une planche d'une autre.
#[derive(Clone, Default)]
struct Planche {
    /// Le mode « sélection multiple » est ouvert.
    select_mode: bool,
    /// Indices des tuiles cochées.
    selected: Vec<usize>,
}

/// Centre d'un bouton du haut, pour y poser le pointeur — voir [`survole`].
#[derive(Clone, Copy, PartialEq)]
enum ControlButton {
    Plus,
    Minus,
}

impl ControlButton {
    fn centre(self) -> egui::Pos2 {
        let coin = match self {
            ControlButton::Plus => PLUS_TOP_LEFT,
            ControlButton::Minus => MINUS_TOP_LEFT,
        };
        coin + Vec2::splat(CONTROL_BUTTON_SIZE / 2.0)
    }
}

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

fn ecrire(harness: &mut Harness<'static>, nom: &str) {
    let image = harness
        .render()
        .expect("rendu offscreen — voir doc de module");
    image
        .save(dossier().join(format!("{nom}.png")))
        .expect("écriture de la planche");
    println!("  {nom}");
}

/// Rend le bandeau réel, puis superpose ce que la maquette propose.
fn harnais(p: Planche) -> Harness<'static> {
    let entries = entrees();
    let total = entries.len();
    let select_mode = p.select_mode;
    let selected = p.selected.clone();
    // Chargées une fois et gardées entre les frames : un `TextureHandle` libère sa texture quand
    // son dernier exemplaire tombe, et la planche sortirait avec des cases vides.
    let mut textures: Option<(
        overlay_ui::portraits::PortraitAtlas,
        overlay_ui::panels::combat_frame::CombatFrame,
        UiIcons,
    )> = None;
    let mut combat_side = CombatSide::default();
    let mut combat_metric = CombatMetric::default();
    let remote_icons = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = overlay_ui::render_content::AuthStatus::Connected;
    let now = std::time::Instant::now();

    // **À la largeur EXACTE de la vraie fenêtre** (`content_width`, plus les deux marges) : la
    // colonne de contrôle est collée au bord gauche dans la fenêtre réelle, et une planche plus
    // large donnerait au « − » une marge qu'il n'a pas.
    let largeur = overlay_ui::panels::watchlist::content_width(total, true)
        + CONTENT_MARGIN_LEFT
        + CONTENT_MARGIN;
    let hauteur = 92.0
        + overlay_ui::render_content::WATCHLIST_TOOLTIP_RESERVE
        + if select_mode { BULK_ROW_HEIGHT } else { 0.0 };

    Harness::builder()
        .with_size(egui::vec2(largeur + 2.0 * HARNESS_MARGIN, hauteur))
        .build_ui(move |ui| {
            let ctx = ui.ctx().clone();
            // Le MÊME style que les deux binaires — sans lui, les infobulles retomberaient sur le
            // thème par défaut d'egui.
            overlay_ui::style::apply(&ctx);
            let (portraits, combat_frame, icons) = textures.get_or_insert_with(|| {
                (
                    overlay_ui::portraits::PortraitAtlas::load(&ctx),
                    overlay_ui::panels::combat_frame::CombatFrame::load(&ctx),
                    UiIcons::load(&ctx),
                )
            });
            paint_content(
                ui,
                RenderContent {
                    kind: OverlayKind::Watchlist,
                    fight: None,
                    portraits,
                    combat_frame,
                    icons,
                    combat_side: &mut combat_side,
                    combat_metric: &mut combat_metric,
                    watchlist: &entries,
                    watchlist_enabled: true,
                    // Un état neuf par frame : aucune de ces planches n'ouvre la sélection
                    // multiple du bandeau (le temporaire vit jusqu'à la fin de l'instruction).
                    watchlist_selection: &mut Default::default(),
                    watchlist_toast: None,
                    catalog: &catalog,
                    catalog_stale: false,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    auth_status: &auth_status,
                    auth_command_tx: &overlay_ui::render_content::NoopAuthSink,
                    interactive: true,
                    // Raccourcis PAR DÉFAUT — voir `overlay_ui::shortcuts` : ces planches
                    // montrent les infobulles telles qu'elles sont sans personnalisation.
                    shortcuts: &ShortcutBindings::default(),
                    now,
                    options: None,
                    login: None,
                },
            );

            if select_mode {
                superpose_selection(ui, total, &selected);
            }
        })
}

/// **Tout ce qui n'existe pas encore**, peint par-dessus le bandeau réel.
///
/// Dans une couche d'avant-plan : le bandeau vit dans une zone défilante, et une peinture faite
/// dans le `Ui` courant passerait sous ses tuiles.
fn superpose_selection(ui: &mut egui::Ui, total: usize, selected: &[usize]) {
    let fenetre = ui.max_rect();
    let mut couche = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(fenetre)
            .id_salt("bandeau-selection")
            .layer_id(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("bandeau-selection"),
            )),
    );
    couche.set_clip_rect(Rect::EVERYTHING);

    let tuile_haut = TILE_ORIGIN.y;

    // 1. Le « − » reste ENFONCÉ tant que le mode est ouvert — comme un onglet actif. Repeint
    //    par-dessus l'original, **à sa position exacte** : au pixel près, sans quoi il a l'air de
    //    sauter au moment où le mode s'ouvre (voir [`MINUS_TOP_LEFT`]).
    couche.put(
        Rect::from_min_size(MINUS_TOP_LEFT, Vec2::splat(CONTROL_BUTTON_SIZE)),
        design::icon_button(DsIcon::Minus)
            .context(IconContext::FirstPlan)
            .size(CONTROL_BUTTON_SIZE)
            .preview_state(design::IconButtonState::Hovered)
            .log_name("bandeau.selection-active"),
    );

    // 2. La sélection de chaque tuile — **peinte par le composant**, pas reproduite ici.
    //
    // `item_slot` porte le mode sélection depuis le 2026-09-13 : la case à cocher au coin, le liseré
    // or sur l'anneau exact du cadre, et rien à recalculer. Une maquette qui superpose sur un rendu
    // déjà peint ne peut pas réutiliser le slot du panneau — elle en repeint un par-dessus, à la
    // même place, avec les mêmes textures. Ce que le portage fera, lui, c'est passer `.selection()`
    // au slot que le panneau construit déjà.
    for index in 0..total {
        let tuile = Rect::from_min_size(
            egui::pos2(tile_left(index), tuile_haut),
            Vec2::splat(TILE_SIZE),
        );
        let cochee = selected.contains(&index);
        if cochee {
            let (anneau, rayon) = design::item_slot_border_ring(tuile);
            couche.painter().rect_stroke(
                anneau,
                rayon,
                egui::Stroke::new(design::tokens::ITEM_SLOT_PLAIN_STROKE, TON.border()),
                egui::StrokeKind::Inside,
            );
        }
        design::paint_checkbox(
            &couche,
            Rect::from_min_size(
                tuile.min + Vec2::splat(design::tokens::ITEM_SLOT_SELECTION_INSET),
                Vec2::splat(design::tokens::CHECKBOX_SIZE),
            ),
            cochee,
            TON.checkbox_tint(cochee),
        );
    }

    // 3. Le bouton de suppression groupée, SOUS la bande et centré **sur la rangée de tuiles**.
    //
    // Pas sur la fenêtre : celle-ci porte en plus le carré de contrôle et ses deux réserves
    // d'infobulle, tout à gauche, si bien que son centre tombe nettement à gauche de celui des
    // suivis. Un bouton centré sur elle se lisait comme mal posé — retour utilisateur sur la
    // première planche.
    let centre_tuiles = (tile_left(0) + tile_left(total.saturating_sub(1)) + TILE_SIZE) / 2.0;
    let bouton = design::button(bulk_label(selected.len(), total))
        .variant(ButtonVariant::Danger)
        .size(ButtonSize::Height(28.0))
        .min_width(170.0)
        .log_name("bandeau.supprimer-groupe");
    let taille = bouton.desired_size(&couche);
    couche.put(
        Rect::from_center_size(
            egui::pos2(centre_tuiles, tuile_haut + TILE_SIZE + 8.0 + 14.0),
            taille,
        ),
        bouton,
    );
}

/// Pose le pointeur au centre d'un bouton de contrôle et laisse le panneau ouvrir SON infobulle.
///
/// Même geste que `panels.rs::panneau_suivi_toutes_les_infobulles_sous_la_bande`, qui couvre déjà
/// ces quatre libellés en non-régression : la maquette ne les réécrit pas, elle les montre en place
/// sous la disposition proposée.
fn survole(harness: &mut Harness<'static>, bouton: ControlButton) {
    harness.run();
    harness.hover_at(bouton.centre());
    harness.run();
}

fn main() {
    // 1 — Le bandeau tel qu'il est aujourd'hui, pour comparer.
    let mut h = harnais(Planche::default());
    h.run();
    ecrire(&mut h, "bandeau_repos");

    // 2 & 3 — Les deux infobulles, **inchangées et rendues par le panneau**. Une première version
    // les peignait à la main pour proposer des libellés plus explicites ; l'utilisateur les garde
    // telles quelles, et une maquette n'a alors plus rien à dire par-dessus le code.
    let mut h = harnais(Planche::default());
    survole(&mut h, ControlButton::Plus);
    ecrire(&mut h, "bandeau_plus_infobulle");

    let mut h = harnais(Planche::default());
    survole(&mut h, ControlButton::Minus);
    ecrire(&mut h, "bandeau_moins_infobulle");

    // 3 — Sélection ouverte, rien de coché : le bouton dit « Supprimer tout ».
    let mut h = harnais(Planche {
        select_mode: true,
        ..Default::default()
    });
    h.run();
    ecrire(&mut h, "bandeau_selection_vide");

    // 4 — Trois tuiles cochées : liseré or, et le bouton compte.
    let mut h = harnais(Planche {
        select_mode: true,
        selected: vec![1, 3, 6],
    });
    h.run();
    ecrire(&mut h, "bandeau_selection_partielle");

    let dir = dossier();
    let ecrites = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    let affiche = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("{ecrites} planches écrites dans {}", affiche.display());
}
