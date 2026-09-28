//! Snapshot testing du **bouton œil** (28 sept. 2026) — la bascule interactif / clic-traversant à
//! la souris, posée en haut à gauche de la fenêtre de jeu (`OverlayKind::ClickThrough`,
//! `panels::click_through`).
//!
//! **Ce que ces planches doivent montrer** : le glyphe dit le mode (œil ouvert en interactif, œil
//! barré en clic-traversant), et le bouton garde **sa pleine opacité dans les deux modes** — c'est
//! le seul élément encore cliquable quand le reste de l'overlay passe à
//! `render_content::CLICK_THROUGH_OPACITY`. Une régression qui l'estomperait avec les autres se
//! verrait sur la planche « œil barré ».
//!
//! La fenêtre du harnais est celle du bouton (`panels::click_through::BUTTON_SIZE`), plus les 8 px
//! de marge du harnais de chaque côté : le bouton remplit sa fenêtre OS en production.
//!
//! **Driver logiciel requis** : même prérequis que `tests/panels.rs` — `mesa-vulkan-drivers` sous
//! Linux.

use std::cell::Cell;
use std::rc::Rc;

use egui_kittest::Harness;
use overlay_engine::CatalogIndex;
use overlay_ui::panels::click_through::BUTTON_SIZE;
use overlay_ui::panels::combat::{CombatMetric, CombatSide};
use overlay_ui::panels::combat_frame::CombatFrame;
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
        .with_size(egui::Vec2::splat(BUTTON_SIZE + 2.0 * MARGE_HARNAIS))
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
                    watchlist_reset: None,
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
        })
}

/// Le centre du bouton, dans le repère du harnais.
fn centre() -> egui::Pos2 {
    let c = MARGE_HARNAIS + BUTTON_SIZE / 2.0;
    egui::pos2(c, c)
}

/// Mode interactif : l'œil ouvert, au repos.
#[test]
fn oeil_ouvert_en_mode_interactif() {
    let mut harness = harness_for(true, Rc::default());
    harness.run();
    harness.snapshot("clic_traversant_oeil_ouvert");
}

/// Mode clic-traversant : l'œil barré, **à pleine opacité** — le reste de l'overlay est estompé,
/// pas ce bouton.
#[test]
fn oeil_barre_en_clic_traversant_sans_estompe() {
    let mut harness = harness_for(false, Rc::default());
    harness.run();
    harness.snapshot("clic_traversant_oeil_barre");
}

/// Survolé : le socle s'éclaircit et le glyphe passe à l'or, comme tout bouton de premier plan.
#[test]
fn survol_du_bouton() {
    let mut harness = harness_for(true, Rc::default());
    harness.hover_at(centre());
    harness.run();
    harness.snapshot("clic_traversant_survol");
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
