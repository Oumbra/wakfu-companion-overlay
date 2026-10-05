//! Snapshot testing du **bouton œil** (28 sept. 2026) — la bascule interactif / clic-traversant à
//! la souris, posée en haut à gauche de la fenêtre de jeu (`OverlayKind::ClickThrough`,
//! `panels::click_through`).
//!
//! **Ce que ces planches doivent montrer** : le glyphe dit ce que le clic FERA (œil barré en
//! interactif, « Masquer l'overlay » ; œil ouvert en clic-traversant, « Afficher l'overlay » —
//! inversés le 2026-09-28 à la demande de l'utilisateur), le bouton porte le fond de section du
//! jeu (`DsTexture::ButtonIconFirstPlanSection`), et il garde **sa pleine opacité dans les deux
//! modes** — c'est le seul élément encore cliquable quand le reste de l'overlay passe à
//! `render_content::CLICK_THROUGH_OPACITY`. Une régression qui l'estomperait avec les autres se
//! verrait sur la planche « œil ouvert ».
//!
//! La fenêtre du harnais est celle du bouton (`panels::click_through::SECTION_SIZE`), plus les
//! 8 px de marge du harnais de chaque côté : le bouton remplit sa fenêtre OS en production. Les
//! planches d'infobulle prennent la fenêtre étendue que l'hôte donne au survol, calculée par la
//! même fonction que lui (`click_through_placement::tip_window`) pour un bouton posé au centre,
//! contre chaque bord et dans un coin : l'infobulle y est centrée sur le bouton, dessous sauf
//! tout en bas, et glissée pour rester dans le jeu.
//!
//! **Driver logiciel requis** : même prérequis que `tests/panels.rs` — `mesa-vulkan-drivers` sous
//! Linux.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use egui_kittest::Harness;
use overlay_engine::CatalogIndex;
use overlay_ui::click_through_placement::{self, ClientArea};
use overlay_ui::panels::click_through::{ClickThroughTip, SECTION_SIZE, TIP_RESERVE};
use overlay_ui::panels::combat::{CombatMetric, CombatSide};
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::panels::drag::PanelDrag;
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content::{
    paint_content, AuthStatus, NoopAuthSink, OverlayKind, RenderContent,
};
use overlay_ui::shortcuts::ShortcutBindings;
use overlay_ui::ui_icons::UiIcons;

/// Marge que le harnais laisse autour du contenu.
const MARGE_HARNAIS: f32 = 8.0;

struct Textures {
    portraits: Option<PortraitAtlas>,
    combat_frame: Option<CombatFrame>,
    icons: Option<UiIcons>,
}

impl Textures {
    fn get_or_load(&mut self, ctx: &egui::Context) -> (&PortraitAtlas, &CombatFrame, &UiIcons) {
        overlay_ui::style::apply(ctx);
        overlay_ui::build_info::freeze_for_snapshots();
        (
            self.portraits
                .get_or_insert_with(|| PortraitAtlas::load(ctx)),
            self.combat_frame
                .get_or_insert_with(|| CombatFrame::load(ctx)),
            self.icons.get_or_insert_with(|| UiIcons::load(ctx)),
        )
    }
}

/// Le bouton dans sa fenêtre, le mode global valant `interactive` ; `clics` compte les frames où
/// il a remonté `RenderOutcome::toggle_interactive`.
fn harness_for(interactive: bool, clics: Rc<Cell<u32>>) -> Harness<'static> {
    harness_with_gestes(interactive, clics, Rc::default())
}

/// Une fenêtre de jeu 1920 × 1080 à l'origine de l'écran, échelle 1 : de quoi placer le bouton
/// contre chacun de ses bords.
const JEU: ClientArea = ClientArea {
    left: 0,
    top: 0,
    width: 1920,
    height: 1080,
};

/// La fenêtre étendue pour l'infobulle, **telle que l'hôte la calcule** au survol d'un bouton posé
/// à `offset` dans le jeu (`None` : son ancrage d'origine, sous les boutons du jeu) — la même
/// fonction, `click_through_placement::tip_window`. Renvoie aussi le point à survoler : le centre
/// du socle, là où il est peint dans cette fenêtre.
fn harness_infobulle(
    interactive: bool,
    offset: Option<(i32, i32)>,
) -> (Harness<'static>, egui::Pos2) {
    let size = (SECTION_SIZE.x as i32, SECTION_SIZE.y as i32);
    let reserve = (TIP_RESERVE.x as i32, TIP_RESERVE.y as i32);
    let base = click_through_placement::window_position(offset, JEU, size);
    let tip = click_through_placement::tip_window(base, size, reserve, JEU);
    let origin = egui::vec2(tip.button_origin.0 as f32, tip.button_origin.1 as f32);
    let harness = harness_complet(
        interactive,
        Rc::default(),
        Rc::default(),
        egui::vec2(tip.size.0 as f32, tip.size.1 as f32),
        ClickThroughTip {
            button_origin: origin,
            above: tip.above,
        },
    );
    (harness, centre() + origin)
}

/// Idem, en relevant aussi chaque geste de déplacement remonté (hors `PanelDrag::None`).
fn harness_with_gestes(
    interactive: bool,
    clics: Rc<Cell<u32>>,
    gestes: Rc<RefCell<Vec<PanelDrag>>>,
) -> Harness<'static> {
    harness_complet(
        interactive,
        clics,
        gestes,
        SECTION_SIZE,
        ClickThroughTip::default(),
    )
}

/// Le harnais, pour une fenêtre de `fenetre` points — celle du bouton au repos, ou la fenêtre
/// étendue au survol.
fn harness_complet(
    interactive: bool,
    clics: Rc<Cell<u32>>,
    gestes: Rc<RefCell<Vec<PanelDrag>>>,
    fenetre: egui::Vec2,
    tip: ClickThroughTip,
) -> Harness<'static> {
    let mut textures = Textures {
        portraits: None,
        combat_frame: None,
        icons: None,
    };
    let mut combat_side = CombatSide::default();
    let mut combat_metric = CombatMetric::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let shortcuts = ShortcutBindings::default();
    let now = std::time::Instant::now();
    Harness::builder()
        .with_size(fenetre + egui::Vec2::splat(2.0 * MARGE_HARNAIS))
        .build_ui(move |ui| {
            let ctx = ui.ctx().clone();
            let (portraits, combat_frame, icons) = textures.get_or_load(&ctx);
            let outcome = paint_content(
                ui,
                RenderContent {
                    kind: OverlayKind::ClickThrough,
                    fight: None,
                    portraits,
                    combat_frame,
                    icons,
                    avatars: None,
                    game_servers: &Default::default(),
                    combat_side: &mut combat_side,
                    combat_metric: &mut combat_metric,
                    watchlist: &[],
                    watchlist_enabled: true,
                    spells_enabled: true,
                    combat_on_right: false,
                    watchlist_selection: &mut Default::default(),
                    watchlist_completions: &Default::default(),
                    watchlist_counter_edit: None,
                    watchlist_toast: None,
                    catalog: &catalog,
                    catalog_stale: false,
                    remote_icons: &remote_icon_store,
                    remote_icon_textures: &mut remote_icon_textures,
                    auth_status: &auth_status,
                    auth_command_tx: &auth_sink,
                    interactive,
                    shortcuts: &shortcuts,
                    now,
                    recap: &Default::default(),
                    recap_cells: Default::default(),
                    recap_chrome: Default::default(),
                    watchlist_chrome: Default::default(),
                    watchlist_base: None,
                    click_through_tip: tip,
                    combat_chrome: Default::default(),
                    options: None,
                    veiled: false,
                    login: None,
                    card_settings: None,
                },
            );
            if outcome.toggle_interactive {
                clics.set(clics.get() + 1);
            }
            if outcome.click_through_drag != PanelDrag::None {
                gestes.borrow_mut().push(outcome.click_through_drag);
            }
        })
}

/// Le centre du socle, dans le repère du harnais.
fn centre() -> egui::Pos2 {
    egui::pos2(MARGE_HARNAIS, MARGE_HARNAIS) + egui::Vec2::splat(2.0 + 18.0)
}

/// Mode interactif : l'œil **barré** au repos — le clic masquera l'overlay.
#[test]
fn oeil_barre_en_mode_interactif() {
    let mut harness = harness_for(true, Rc::default());
    harness.run();
    harness.snapshot("clic_traversant_oeil_barre");
}

/// Mode clic-traversant : l'œil **ouvert**, **à pleine opacité** — le reste de l'overlay est
/// estompé, pas ce bouton, et le clic le fera réapparaître.
#[test]
fn oeil_ouvert_en_clic_traversant_sans_estompe() {
    let mut harness = harness_for(false, Rc::default());
    harness.run();
    harness.snapshot("clic_traversant_oeil_ouvert");
}

/// Survolé en mode interactif, à son ancrage d'origine : « Masquer l'overlay » et le raccourci,
/// **centrée sous le bouton**, à l'écart du design system sous son cadre (2026-09-28).
#[test]
fn infobulle_masquer_l_overlay() {
    let (mut harness, survol) = harness_infobulle(true, None);
    harness.hover_at(survol);
    harness.run();
    harness.snapshot("clic_traversant_infobulle_masquer");
}

/// Survolé en clic-traversant : « Afficher l'overlay » et le raccourci.
#[test]
fn infobulle_afficher_l_overlay() {
    let (mut harness, survol) = harness_infobulle(false, None);
    harness.hover_at(survol);
    harness.run();
    harness.snapshot("clic_traversant_infobulle_afficher");
}

/// Posé contre le bord gauche du jeu : l'infobulle reste dessous, glissée vers la droite juste
/// assez pour ne pas sortir du jeu.
#[test]
fn infobulle_contre_le_bord_gauche() {
    let (mut harness, survol) = harness_infobulle(true, Some((0, 300)));
    harness.hover_at(survol);
    harness.run();
    harness.snapshot("clic_traversant_infobulle_bord_gauche");
}

/// Contre le bord droit : glissée vers la gauche.
#[test]
fn infobulle_contre_le_bord_droit() {
    let (mut harness, survol) = harness_infobulle(true, Some((5000, 300)));
    harness.hover_at(survol);
    harness.run();
    harness.snapshot("clic_traversant_infobulle_bord_droit");
}

/// Tout en bas du jeu, sans la place dessous : au-dessus, centrée.
#[test]
fn infobulle_tout_en_bas() {
    let (mut harness, survol) = harness_infobulle(true, Some((900, 5000)));
    harness.hover_at(survol);
    harness.run();
    harness.snapshot("clic_traversant_infobulle_en_bas");
}

/// Dans un coin, en bas à droite : au-dessus, et glissée vers la gauche.
#[test]
fn infobulle_dans_le_coin_bas_droit() {
    let (mut harness, survol) = harness_infobulle(true, Some((5000, 5000)));
    harness.hover_at(survol);
    harness.run();
    harness.snapshot("clic_traversant_infobulle_coin_bas_droit");
}

/// Un clic remonte UNE intention de bascule — c'est l'hôte qui bascule, jamais le panneau.
#[test]
fn un_clic_remonte_une_bascule() {
    let clics = Rc::new(Cell::new(0));
    let mut harness = harness_for(false, Rc::clone(&clics));
    harness.run();
    assert_eq!(clics.get(), 0, "aucune bascule sans clic");
    harness.hover_at(centre());
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: centre(),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        });
    }
    harness.run();
    assert_eq!(clics.get(), 1, "un clic, une bascule");
}

/// Appuyer puis bouger au-delà du seuil de glissement **déplace** le bouton sans basculer le mode
/// (2026-09-28, « permets à l'utilisateur de placer ce bouton où il le souhaite ») : le panneau
/// remonte le point de saisie, le suivi, puis le relâchement — l'hôte pose la fenêtre.
#[test]
fn glisser_deplace_sans_basculer() {
    let clics = Rc::new(Cell::new(0));
    let gestes = Rc::new(RefCell::new(Vec::new()));
    let mut harness = harness_with_gestes(true, Rc::clone(&clics), Rc::clone(&gestes));
    harness.run();
    harness.hover_at(centre());
    let bouton = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    harness.event(bouton(centre(), true));
    harness.run();
    for pas in 1..=4 {
        harness.event(egui::Event::PointerMoved(
            centre() + egui::vec2(6.0 * pas as f32, 0.0),
        ));
        harness.run();
    }
    harness.event(bouton(centre() + egui::vec2(24.0, 0.0), false));
    harness.run();

    assert_eq!(clics.get(), 0, "un glissement ne bascule pas le mode");
    let gestes = gestes.borrow();
    assert!(
        matches!(gestes.first(), Some(PanelDrag::Started(_))),
        "le geste commence par le point de saisie : {gestes:?}"
    );
    assert!(
        gestes.contains(&PanelDrag::Moved),
        "le suivi remonte : {gestes:?}"
    );
    assert_eq!(gestes.last(), Some(&PanelDrag::Released), "{gestes:?}");
}
