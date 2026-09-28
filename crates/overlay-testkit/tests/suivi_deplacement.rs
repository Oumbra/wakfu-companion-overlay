//! Snapshot testing de la **poignée du bandeau Suivi** (28 sept. 2026) — le déplacement libre,
//! l'orientation verticale et le replacement (`panels::watchlist::chrome_pill`,
//! `watchlist_placement`).
//!
//! **Ce que ces planches doivent montrer** : au survol seulement, une pastille à gauche du bandeau
//! couché (au-dessus du bandeau dressé) avec l'icône d'orientation, la poignée, et le replacement
//! une fois le bandeau déplacé ; dressé, les tuiles s'empilent sous le carré de contrôle et les
//! infobulles s'ouvrent vers le centre du jeu, dans la réserve que l'hôte ajoute au survol.
//!
//! La fenêtre du harnais est celle que l'hôte donnerait (`watchlist_placement::plan`, échelle 1),
//! plus les 8 px de marge du harnais de chaque côté.
//!
//! **Driver logiciel requis** : même prérequis que `tests/panels.rs` — `mesa-vulkan-drivers` sous
//! Linux.

use std::cell::RefCell;
use std::rc::Rc;

use egui_kittest::Harness;
use overlay_engine::{CatalogIndex, WatchlistEntry, WatchlistKind, WatchlistMode};
use overlay_ui::avatars::AvatarAtlas;
use overlay_ui::panels::combat::{CombatMetric, CombatSide};
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::panels::drag::PanelDrag;
use overlay_ui::panels::watchlist::WatchlistChrome;
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content::{
    paint_content, AuthStatus, NoopAuthSink, OverlayKind, RenderContent,
};
use overlay_ui::shortcuts::ShortcutBindings;
use overlay_ui::ui_icons::UiIcons;
use overlay_ui::watchlist_placement::{self, ClientArea, Side, StripState};

/// Marge que le harnais laisse autour du contenu.
const MARGE_HARNAIS: f32 = 8.0;

/// Une fenêtre de jeu 1920 × 1080 — de quoi calculer le gabarit comme l'hôte.
const CLIENT: ClientArea = ClientArea {
    left: 0,
    top: 0,
    width: 1920,
    height: 1080,
};

struct Textures {
    portraits: Option<PortraitAtlas>,
    combat_frame: Option<CombatFrame>,
    icons: Option<UiIcons>,
    avatars: Option<AvatarAtlas>,
}

impl Textures {
    fn get_or_load(
        &mut self,
        ctx: &egui::Context,
    ) -> (&PortraitAtlas, &CombatFrame, &UiIcons, &AvatarAtlas) {
        overlay_ui::style::apply(ctx);
        overlay_ui::build_info::freeze_for_snapshots();
        (
            self.portraits
                .get_or_insert_with(|| PortraitAtlas::load(ctx)),
            self.combat_frame
                .get_or_insert_with(|| CombatFrame::load(ctx)),
            self.icons.get_or_insert_with(|| UiIcons::load(ctx)),
            self.avatars.get_or_insert_with(|| AvatarAtlas::load(ctx)),
        )
    }
}

fn entrees(n: usize) -> Vec<WatchlistEntry> {
    ["Bottes Lantha", "Bois de Frêne", "Pierre de Lune"]
        .iter()
        .take(n)
        .map(|nom| WatchlistEntry {
            name: (*nom).to_string(),
            kind: WatchlistKind::Item,
            mode: WatchlistMode::Up,
            count: 0,
            countdown_target: 0,
            catalog_id: None,
        })
        .collect()
}

/// Ce que le bandeau a remonté pendant le test.
#[derive(Default)]
struct Remontees {
    gestes: Vec<PanelDrag>,
    orientations: usize,
    replacements: usize,
}

/// Le bandeau tel que l'hôte le peindrait : `vertical`, `moved`, et le survol de la fenêtre de
/// base — qui étend la fenêtre du bandeau vertical. `position` est celle que l'utilisateur lui a
/// donnée (le côté d'ouverture en dépend).
fn harnais(
    n: usize,
    vertical: bool,
    position: Option<(i32, i32)>,
    survol: bool,
    remontees: Rc<RefCell<Remontees>>,
) -> Harness<'static> {
    let entries = entrees(n);
    let plan = watchlist_placement::plan(
        StripState {
            entry_count: n,
            tracking_enabled: true,
            vertical,
            hovered: survol,
            ..Default::default()
        },
        position,
        CLIENT,
        1.0,
    );
    let chrome = WatchlistChrome {
        vertical,
        moved: position.is_some(),
        side: plan.side,
    };
    let mut textures = Textures {
        portraits: None,
        combat_frame: None,
        icons: None,
        avatars: None,
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
    let mut selection = Default::default();
    let completions = Default::default();
    Harness::builder()
        .with_size(
            egui::vec2(plan.size.0 as f32, plan.size.1 as f32)
                + egui::Vec2::splat(2.0 * MARGE_HARNAIS),
        )
        .build_ui(move |ui| {
            let ctx = ui.ctx().clone();
            let (portraits, combat_frame, icons, avatars) = textures.get_or_load(&ctx);
            let outcome = paint_content(
                ui,
                RenderContent {
                    kind: OverlayKind::Watchlist,
                    fight: None,
                    portraits,
                    combat_frame,
                    icons,
                    avatars: Some(avatars),
                    game_servers: &Default::default(),
                    combat_side: &mut combat_side,
                    combat_metric: &mut combat_metric,
                    watchlist: &entries,
                    watchlist_enabled: true,
                    spells_enabled: true,
                    combat_on_right: false,
                    watchlist_selection: &mut selection,
                    watchlist_completions: &completions,
                    watchlist_reset: None,
                    watchlist_toast: None,
                    catalog: &catalog,
                    catalog_stale: false,
                    remote_icons: &remote_icon_store,
                    remote_icon_textures: &mut remote_icon_textures,
                    auth_status: &auth_status,
                    auth_command_tx: &auth_sink,
                    interactive: true,
                    shortcuts: &shortcuts,
                    now,
                    recap: &Default::default(),
                    recap_cells: Default::default(),
                    recap_chrome: Default::default(),
                    watchlist_chrome: chrome,
                    watchlist_base: plan.base,
                    click_through_tip: Default::default(),
                    combat_chrome: Default::default(),
                    options: None,
                    veiled: false,
                    login: None,
                    card_settings: None,
                },
            );
            let mut remontees = remontees.borrow_mut();
            if outcome.watchlist_drag != PanelDrag::None {
                remontees.gestes.push(outcome.watchlist_drag);
            }
            if outcome.watchlist_toggle_orientation {
                remontees.orientations += 1;
            }
            if outcome.watchlist_restore_requested {
                remontees.replacements += 1;
            }
        })
}

fn press(harness: &mut Harness<'_>, pos: egui::Pos2, pressed: bool) {
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    });
}

fn cliquer(harness: &mut Harness<'_>, pos: egui::Pos2) {
    harness.hover_at(pos);
    harness.run();
    press(harness, pos, true);
    harness.run();
    press(harness, pos, false);
    harness.run();
}

/// Le centre du glyphe `index` de la pastille du bandeau COUCHÉ, qui en porte `glyphes` : une
/// colonne de 14 px tous les 20 px, centrée sur la rangée des tuiles (64 px).
fn glyphe_couche(index: usize, glyphes: usize) -> egui::Pos2 {
    let span = glyphes as f32 * 14.0 + (glyphes - 1) as f32 * 6.0 + 8.0;
    egui::pos2(
        MARGE_HARNAIS + 4.0 + 7.0,
        MARGE_HARNAIS + (64.0 - span) / 2.0 + 4.0 + 7.0 + 20.0 * index as f32,
    )
}

/// Un point du bandeau couché entre deux tuiles (aucune infobulle) — de quoi faire apparaître la pastille.
const SUR_LA_BANDE: egui::Pos2 = egui::pos2(180.0, 40.0);

/// Couché, jamais déplacé, survolé : la pastille porte l'orientation et la poignée.
#[test]
fn bandeau_couche_survole_montre_sa_poignee() {
    let mut harness = harnais(3, false, None, false, Rc::default());
    harness.hover_at(SUR_LA_BANDE);
    harness.run();
    harness.snapshot("suivi_poignee_couche");
}

/// Couché et déplacé : le glyphe de replacement s'ajoute en bas de la pastille, et la poignée
/// survolée dit ce qu'elle fait.
#[test]
fn bandeau_couche_deplace_offre_son_replacement() {
    let mut harness = harnais(3, false, Some((300, 400)), false, Rc::default());
    harness.hover_at(glyphe_couche(1, 3));
    harness.run();
    harness.snapshot("suivi_poignee_couche_deplace");
}

/// Sans pointeur, rien : la place de la pastille reste vide.
#[test]
fn bandeau_sans_pointeur_ne_montre_pas_sa_poignee() {
    let mut harness = harnais(3, false, Some((300, 400)), false, Rc::default());
    harness.run();
    harness.snapshot("suivi_poignee_hors_survol");
}

/// Dressé, collé au bord DROIT du jeu et survolé : la fenêtre s'étend à gauche, la pastille est
/// au-dessus du carré, les tuiles s'empilent, l'infobulle de la poignée s'ouvre vers le centre.
#[test]
fn bandeau_dresse_survole_s_ouvre_vers_le_centre() {
    let position = Some((1840, 300));
    let mut harness = harnais(3, true, position, true, Rc::default());
    let plan = watchlist_placement::plan(
        StripState {
            entry_count: 3,
            tracking_enabled: true,
            vertical: true,
            hovered: true,
            ..Default::default()
        },
        position,
        CLIENT,
        1.0,
    );
    assert_eq!(plan.side, Side::Left);
    let base = plan.base.expect("fenêtre étendue au survol").min;
    let poignee =
        egui::pos2(MARGE_HARNAIS, MARGE_HARNAIS) + base.to_vec2() + egui::vec2(31.0, 11.0);
    harness.hover_at(poignee);
    harness.run();
    harness.snapshot("suivi_dresse_survole");
}

/// Dressé et vide, au repos : le carré se dresse en colonne de quatre boutons.
#[test]
fn bandeau_dresse_vide_dresse_ses_boutons() {
    let mut harness = harnais(0, true, None, false, Rc::default());
    harness.run();
    harness.snapshot("suivi_dresse_vide");
}

/// Les trois gestes de la pastille remontent à l'hôte, qui seul pose la fenêtre et écrit la
/// config : la poignée tenue (saisie, suivi, relâchement), l'orientation, le replacement — et le
/// replacement n'existe pas tant que le bandeau n'a pas bougé.
#[test]
fn la_pastille_remonte_ses_trois_gestes() {
    // Jamais déplacé : deux glyphes, rien à replacer là où serait le troisième.
    let remontees = Rc::new(RefCell::new(Remontees::default()));
    let mut harness = harnais(3, false, None, false, Rc::clone(&remontees));
    harness.hover_at(SUR_LA_BANDE);
    harness.run();
    cliquer(&mut harness, glyphe_couche(0, 2));
    assert_eq!(remontees.borrow().orientations, 1, "l'orientation remonte");

    let poignee = glyphe_couche(1, 2);
    harness.hover_at(poignee);
    harness.run();
    press(&mut harness, poignee, true);
    harness.run();
    for pas in 1..=4 {
        harness.event(egui::Event::PointerMoved(
            poignee + egui::vec2(8.0 * pas as f32, 6.0),
        ));
        harness.run();
    }
    press(&mut harness, poignee + egui::vec2(32.0, 6.0), false);
    harness.run();
    {
        let remontees = remontees.borrow();
        assert!(
            matches!(remontees.gestes.first(), Some(PanelDrag::Started(_))),
            "le geste commence par le point de saisie : {:?}",
            remontees.gestes
        );
        assert!(remontees.gestes.contains(&PanelDrag::Moved));
        assert_eq!(remontees.gestes.last(), Some(&PanelDrag::Released));
        // Le point de saisie se compte depuis le coin du bandeau, marge du harnais déduite.
        if let Some(PanelDrag::Started(saisie)) = remontees.gestes.first() {
            let attendu = poignee - egui::vec2(MARGE_HARNAIS, MARGE_HARNAIS);
            assert!((saisie.x - attendu.x).abs() < 1.0 && (saisie.y - attendu.y).abs() < 1.0);
        }
    }
    let pas_de_troisieme = egui::pos2(glyphe_couche(1, 2).x, glyphe_couche(1, 2).y + 20.0);
    cliquer(&mut harness, pas_de_troisieme);
    assert_eq!(
        remontees.borrow().replacements,
        0,
        "jamais déplacé, rien à replacer"
    );

    // Déplacé : le troisième glyphe replace.
    let remontees = Rc::new(RefCell::new(Remontees::default()));
    let mut harness = harnais(3, false, Some((300, 400)), false, Rc::clone(&remontees));
    harness.hover_at(SUR_LA_BANDE);
    harness.run();
    cliquer(&mut harness, glyphe_couche(2, 3));
    assert_eq!(
        remontees.borrow().replacements,
        1,
        "déplacé, le replacement remonte"
    );
    assert_eq!(remontees.borrow().orientations, 0);
}
