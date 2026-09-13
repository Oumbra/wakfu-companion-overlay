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
//!    132 px de haut posé par-dessus le jeu. L'infobulle doit donc le dire : « Ajouter un suivi
//!    (ouvre les Options) », plutôt que le simple « Ajouter » d'aujourd'hui, qui laisse attendre
//!    une saisie immédiate.
//! 2. **Le « − » ouvre un MODE, il ne supprime rien.** Même geste que l'onglet Suivi : chaque tuile
//!    gagne une case à cocher, et un bouton de suppression groupée apparaît. Recliquer le « − »
//!    quitte le mode — le bouton reste donc enfoncé tant qu'il est ouvert, comme un onglet actif.
//! 3. **Le bouton de suppression est SOUS la bande, centré.** Demande explicite. Il ne peut pas
//!    vivre dans le carré de contrôle (24 px de côté, déjà plein à quatre boutons) ni à côté des
//!    tuiles (la bande défile horizontalement, il sortirait du champ). Sous la bande, il est à
//!    portée immédiate et ne pousse rien.
//! 4. **En style danger, comme sur le web et comme dans l'onglet.** Décision déjà rendue le
//!    2026-09-13 pour l'onglet Suivi, reprise telle quelle.
//!
//! ## Un constat de la maquette : la réserve d'infobulle est trop courte
//!
//! Le bandeau garde 88 px à gauche du carré de contrôle pour que les infobulles de sa colonne
//! gauche s'ouvrent de ce côté plutôt que par-dessus les tuiles (`CONTROL_TOOLTIP_RESERVE`, posée
//! le 2026-09-06 pour ce défaut précis). **Aucun des libellés n'y tient** — ni celui proposé ici,
//! ni celui d'aujourd'hui, « Ajouter (Ctrl+Shift+A) », qui se replie déjà sur la première tuile.
//! Trois issues, à trancher : raccourcir le libellé à ce que 88 px acceptent (une dizaine de
//! caractères), élargir la réserve (la fenêtre s'élargit d'autant, et la bande se décale encore du
//! centre du jeu), ou accepter le repli à droite et retirer la réserve, qui ne sert alors plus à
//! rien. Ces planches montrent le repli, pour que le choix se fasse sur pièce.
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
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`.
//!
//! ```text
//! cargo run -p overlay-testkit --example bandeau-suivi-planches
//! ```

use egui::{Color32, Rect, Vec2};
use egui_kittest::Harness;
use overlay_engine::{CatalogIndex, WatchlistEntry, WatchlistKind, WatchlistMode};
use overlay_ui::design::{self, ButtonSize, ButtonVariant, DsIcon, IconContext};
use overlay_ui::panels::combat::CombatSide;
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content::{paint_content, OverlayKind, RenderContent};
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
/// `render_content::paint_content` pose cette marge autour du contenu du bandeau.
const CONTENT_MARGIN: f32 = 6.0;
/// La marge fixe qu'`egui_kittest` ajoute autour de tout harnais `new_ui`.
const HARNESS_MARGIN: f32 = 8.0;

/// Liseré d'une tuile COCHÉE — l'or, la couleur d'état de ce design system, comme dans l'onglet.
const TILE_SELECTED: Color32 = design::tokens::TEXT_GOLD;
const TILE_SELECTED_WIDTH: f32 = 2.0;

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

/// Bord gauche de la PREMIÈRE tuile, dans le repère du harnais — **mesuré sur la planche de
/// repos**, pas calculé.
///
/// La somme des jetons (`8 + 6 + 88 + 60 + 12`) donne 174 ; la mesure en donne 172. Les deux pixels
/// d'écart viennent de ce que la `ScrollArea` et `paint_content` posent entre eux, et qu'aucun
/// jeton public ne décrit. Une maquette qui superpose des éléments sur un rendu réel a tout intérêt
/// à **relever** la position plutôt qu'à la reconstituer : l'écart ne se verrait qu'à l'œil, sur
/// des cases décalées, et se prendrait pour un choix de design.
///
/// Méthode : sur `bandeau_repos.png`, la première colonne dont la luminance saute au-dessus du fond
/// sur la ligne médiane des tuiles, et la première ligne de même sur leur colonne médiane.
const TILE_ORIGIN: egui::Pos2 = egui::pos2(172.0, 16.0);

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
    /// L'infobulle PROPOSÉE à peindre, et sur lequel des deux boutons — voir [`paint_tooltip`].
    /// Peinte par la maquette, pas rendue par le panneau : c'est justement le libellé qui change.
    tooltip: Option<(ControlButton, &'static str)>,
}

/// Lequel des deux boutons du haut porte l'infobulle proposée.
#[derive(Clone, Copy, PartialEq)]
enum ControlButton {
    Plus,
    Minus,
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
    let tooltip = p.tooltip;
    // Chargées une fois et gardées entre les frames : un `TextureHandle` libère sa texture quand
    // son dernier exemplaire tombe, et la planche sortirait avec des cases vides.
    let mut textures: Option<(
        overlay_ui::portraits::PortraitAtlas,
        overlay_ui::panels::combat_frame::CombatFrame,
        UiIcons,
    )> = None;
    let mut combat_side = CombatSide::default();
    let remote_icons = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = overlay_ui::render_content::AuthStatus::Connected;
    let now = std::time::Instant::now();

    // **À la largeur EXACTE de la vraie fenêtre** (`content_width`, plus les deux marges) : la
    // colonne de contrôle est collée au bord gauche dans la fenêtre réelle, et une planche plus
    // large donnerait au « − » une marge qu'il n'a pas.
    let largeur = overlay_ui::panels::watchlist::content_width(total) + 2.0 * CONTENT_MARGIN;
    let hauteur = 132.0 + if select_mode { BULK_ROW_HEIGHT } else { 0.0 };

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
                    watchlist: &entries,
                    watchlist_toast: None,
                    catalog: &catalog,
                    catalog_stale: false,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    auth_status: &auth_status,
                    auth_command_tx: &overlay_ui::render_content::NoopAuthSink,
                    interactive: true,
                    now,
                    options: None,
                },
            );

            if select_mode {
                superpose_selection(ui, total, &selected);
            }
            if let Some((bouton, texte)) = tooltip {
                let carre = CONTROL_BUTTON_GAP * 3.0 + CONTROL_BUTTON_SIZE * 2.0;
                let gauche = TILE_ORIGIN.x - TILE_GAP - carre + CONTROL_BUTTON_GAP;
                let haut = TILE_ORIGIN.y + (TILE_SIZE - carre) / 2.0 + CONTROL_BUTTON_GAP;
                let x = match bouton {
                    ControlButton::Plus => gauche,
                    ControlButton::Minus => gauche + CONTROL_BUTTON_SIZE + CONTROL_BUTTON_GAP,
                };
                let ancre =
                    Rect::from_min_size(egui::pos2(x, haut), Vec2::splat(CONTROL_BUTTON_SIZE));
                paint_tooltip(ui, texte, ancre);
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
    //    par-dessus l'original, à sa position exacte dans le carré de contrôle.
    //
    // Le carré de contrôle finit `TILE_GAP` avant la première tuile, et il est centré sur la même
    // rangée qu'elles : sa géométrie se déduit donc de [`TILE_ORIGIN`], sans refaire la somme des
    // marges de gauche.
    let carre = CONTROL_BUTTON_GAP * 3.0 + CONTROL_BUTTON_SIZE * 2.0;
    let carre_gauche = TILE_ORIGIN.x - TILE_GAP - carre;
    let carre_haut = tuile_haut + (TILE_SIZE - carre) / 2.0;
    let moins = Rect::from_min_size(
        egui::pos2(
            carre_gauche + CONTROL_BUTTON_GAP * 2.0 + CONTROL_BUTTON_SIZE,
            carre_haut + CONTROL_BUTTON_GAP,
        ),
        Vec2::splat(CONTROL_BUTTON_SIZE),
    );
    couche.put(
        moins,
        design::icon_button(DsIcon::Minus)
            .context(IconContext::FirstPlan)
            .size(CONTROL_BUTTON_SIZE)
            .preview_state(design::IconButtonState::Hovered)
            .log_name("bandeau.selection-active"),
    );

    // 2. Une case à cocher au coin haut-gauche de chaque tuile, et un liseré sur les cochées.
    for index in 0..total {
        let tuile = Rect::from_min_size(
            egui::pos2(tile_left(index), tuile_haut),
            Vec2::splat(TILE_SIZE),
        );
        let cochee = selected.contains(&index);
        if cochee {
            couche.painter().rect_stroke(
                tuile,
                4,
                egui::Stroke::new(TILE_SELECTED_WIDTH, TILE_SELECTED),
                egui::StrokeKind::Inside,
            );
        }
        let mut etat = cochee;
        couche.put(
            Rect::from_min_size(
                tuile.min + Vec2::splat(4.0),
                Vec2::splat(design::tokens::CHECKBOX_SIZE),
            ),
            design::checkbox(&mut etat, "").log_name(format!("bandeau.cocher-{index}")),
        );
    }

    // 3. Le bouton de suppression groupée, SOUS la bande et centré — demande explicite.
    let bouton = design::button(bulk_label(selected.len(), total))
        .variant(ButtonVariant::Danger)
        .size(ButtonSize::Height(28.0))
        .min_width(170.0)
        .log_name("bandeau.supprimer-groupe");
    let taille = bouton.desired_size(&couche);
    couche.put(
        Rect::from_center_size(
            egui::pos2(fenetre.center().x, tuile_haut + TILE_SIZE + 8.0 + 14.0),
            taille,
        ),
        bouton,
    );
}

/// Peint une infobulle du design system au-dessus d'un rectangle, avec ses jetons.
///
/// **La maquette la peint elle-même** parce que c'est le LIBELLÉ qui est en question : celui du
/// panneau dit « Ajouter » et « Supprimer », et ces deux mots sont précisément ce que la proposition
/// corrige — le « + » n'ajoute pas sur place, et le « − » ne supprime rien.
fn paint_tooltip(ui: &mut egui::Ui, texte: &str, ancre: Rect) {
    let police = egui::FontId::proportional(14.0);
    let galley =
        ui.fonts_mut(|f| f.layout_no_wrap(texte.to_string(), police, design::tokens::TOOLTIP_TEXT));
    let marge = design::tokens::TOOLTIP_MARGIN;
    let taille = galley.size()
        + Vec2::new(
            (marge.left + marge.right) as f32,
            (marge.top + marge.bottom) as f32,
        );
    // **À GAUCHE si elle tient, à droite sinon** — c'est le repli de `design::tooltip`
    // (`TooltipSide::Left` puis ses alternatives), et il se déclenche ici : la réserve gauche du
    // bandeau fait 88 px, et **aucun de ces libellés n'y tient**, pas même celui d'aujourd'hui.
    // C'est un constat de la maquette, pas un défaut de rendu — voir la doc de module.
    let fenetre = ui.max_rect();
    let a_gauche = ancre.left() - design::tokens::TOOLTIP_GAP - taille.x;
    let x = if a_gauche >= fenetre.left() {
        a_gauche
    } else {
        ancre.right() + design::tokens::TOOLTIP_GAP
    };
    let boite = Rect::from_min_size(egui::pos2(x, ancre.center().y - taille.y / 2.0), taille);
    ui.painter()
        .rect_filled(boite, 4, design::tokens::TOOLTIP_BG_FILL);
    ui.painter().galley(
        egui::pos2(
            boite.left() + marge.left as f32,
            boite.top() + marge.top as f32,
        ),
        galley,
        design::tokens::TOOLTIP_TEXT,
    );
}

fn main() {
    // 1 — Le bandeau tel qu'il est aujourd'hui, pour comparer.
    let mut h = harnais(Planche::default());
    h.run();
    ecrire(&mut h, "bandeau_repos");

    // 2 & 3 — Les deux libellés d'infobulle proposés. Ceux d'aujourd'hui — « Ajouter » et
    // « Supprimer » — promettent l'un une saisie sur place, l'autre un effacement immédiat : ni
    // l'un ni l'autre n'est ce que le bouton fera.
    let mut h = harnais(Planche {
        tooltip: Some((
            ControlButton::Plus,
            "Ajouter un suivi dans les Options (Ctrl+Shift+A)",
        )),
        ..Default::default()
    });
    h.run();
    ecrire(&mut h, "bandeau_plus_infobulle");

    let mut h = harnais(Planche {
        tooltip: Some((
            ControlButton::Minus,
            "Sélectionner pour retirer (Ctrl+Shift+S)",
        )),
        ..Default::default()
    });
    h.run();
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
        ..Default::default()
    });
    h.run();
    ecrire(&mut h, "bandeau_selection_partielle");

    let dir = dossier();
    let ecrites = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    let affiche = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("{ecrites} planches écrites dans {}", affiche.display());
}
