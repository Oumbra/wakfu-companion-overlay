//! Snapshot testing et comportement du **panneau Combat réductible** (2026-09-28, retour d'un
//! joueur : « l'overlay combat est trop large ») — voir `panels::combat::CombatChrome::collapsed`
//! et la doc de module de `panels::combat`.
//!
//! **Ce que ces planches doivent montrer** :
//!
//! - hors survol, **aucune flèche** : le panneau déplié est exactement celui d'avant ;
//! - au survol, déplié : la flèche blanche sur la ligne du fermoir, à l'EXTÉRIEUR du détail, qui
//!   pointe vers le bord de l'écran (ranger) ;
//! - réduit : le switch de camp, le total de la grandeur (icône + chiffre détourés, sans fond)
//!   entre le switch et le cadre, puis le cadre — sans bandeau leader, barres ni sorts ;
//! - réduit et survolé : la flèche contre le fermoir, pointée vers le centre du jeu (ouvrir) ;
//! - posé à droite : tout cela en miroir, la flèche retournée avec le décor, le total à l'endroit.
//!
//! **Fixture : le vrai `wakfu.log`**, comme `tests/combat_deplacement.rs`.
//!
//! **Driver logiciel requis** : même prérequis que `tests/panels.rs` — `mesa-vulkan-drivers` sous
//! Linux.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use egui_kittest::Harness;
use overlay_engine::{CatalogIndex, Engine, FightSnapshot, SessionSnapshot};
use overlay_ingest::Tailer;
use overlay_ui::panels::combat::{CombatChrome, CombatMetric, CombatSide};
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content::{
    paint_content, AuthStatus, NoopAuthSink, OverlayKind, RenderContent,
};
use overlay_ui::shortcuts::ShortcutBindings;
use overlay_ui::ui_icons::UiIcons;

const WAKFU_LOG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../overlay-engine/tests/wakfu.log"
);

static NEXT_TEST_ID: AtomicU32 = AtomicU32::new(0);

/// Voir `tests/panels.rs::test_engine`.
fn test_engine() -> Engine {
    let dir = std::env::temp_dir().join(format!(
        "wakfu-overlay-testkit-combat-repli-{}-{}",
        std::process::id(),
        NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    Engine::with_stores(dir.join("watchlist-counts.json"), dir.join("fights"))
        .expect("création de l'Engine")
}

/// Voir `tests/panels.rs::replay_real_log`.
fn replay_real_log() -> SessionSnapshot {
    let mut tailer = Tailer::new(WAKFU_LOG);
    let mut engine = test_engine();
    loop {
        let batches = tailer.poll().expect("poll() du tailer");
        if batches.is_empty() {
            break;
        }
        for batch in &batches {
            engine.ingest_batch(batch).expect("ingestion d'un lot");
        }
    }
    engine.snapshot()
}

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

/// Ce que l'hôte tiendrait : l'état réduit, la grandeur affichée, et le nombre de clics sur la
/// flèche remontés par le panneau.
struct Hote {
    chrome: Cell<CombatChrome>,
    metric: Cell<CombatMetric>,
    bascules: Cell<usize>,
}

fn hote(collapsed: bool) -> Rc<Hote> {
    Rc::new(Hote {
        chrome: Cell::new(CombatChrome {
            locked: false,
            moved: false,
            collapsed,
        }),
        metric: Cell::new(CombatMetric::default()),
        bascules: Cell::new(0),
    })
}

/// Le panneau Combat du premier combat du rejeu, dans les 800 × 600 par défaut.
fn harness_for(hote: Rc<Hote>, on_right: bool) -> Harness<'static> {
    let fight: FightSnapshot = replay_real_log()
        .fights
        .first()
        .cloned()
        .expect("au moins un combat dans le rejeu");
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let mut textures = Textures {
        portraits: None,
        combat_frame: None,
        icons: None,
    };
    let mut combat_side = CombatSide::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let shortcuts = ShortcutBindings::default();
    let now = std::time::Instant::now();
    Harness::new_ui(move |ui: &mut egui::Ui| {
        let ctx = ui.ctx().clone();
        let (portraits, combat_frame, icons) = textures.get_or_load(&ctx);
        let mut combat_metric = hote.metric.get();
        let outcome = paint_content(
            ui,
            RenderContent {
                kind: OverlayKind::Combat,
                fight: Some(&fight),
                portraits,
                combat_frame,
                icons,
                avatars: None,
                game_servers: &Default::default(),
                combat_side: &mut combat_side,
                combat_metric: &mut combat_metric,
                watchlist: &[],
                watchlist_enabled: true,
                spells_enabled: false,
                combat_on_right: on_right,
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
                interactive: true,
                shortcuts: &shortcuts,
                now,
                recap: &Default::default(),
                recap_cells: Default::default(),
                recap_chrome: Default::default(),
                watchlist_chrome: Default::default(),
                watchlist_base: None,
                click_through_tip: Default::default(),
                combat_chrome: hote.chrome.get(),
                options: None,
                veiled: false,
                login: None,
                card_settings: None,
            },
        );
        hote.metric.set(combat_metric);
        if outcome.combat_toggle_collapsed {
            hote.bascules.set(hote.bascules.get() + 1);
        }
    })
}

fn clic(harness: &mut Harness<'_>, pos: egui::Pos2) {
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        });
        harness.run();
    }
}

/// Voir `tests/combat_deplacement.rs::survol_neutre` : dans le panneau, sur rien.
fn survol_neutre() -> egui::Pos2 {
    egui::pos2(400.0, 580.0)
}

/// **Où tombent la flèche et le total dans ces planches.** Le contenu commence à (8, 52) : 8 px
/// de marge du harnais, plus `COMBAT_TOP_MARGIN` (44) en haut. Le cadre des portraits (gabarit à
/// six) commence 53 px plus bas, soit à y = 105, et sa dernière encre est à y = 497 : le fermoir
/// est au milieu, à y = 301. Réduit, le total (24 px + 5 px d'air) décale le cadre de 29 px.
///
/// La flèche : déplié, après le détail — 8 + 70 (cadre) + 6 (gouttière) + 190 (barres) + 6
/// (débord du bandeau) + 4 (air) = 284, centrée 10 px plus loin ; réduit, contre le fermoir —
/// 8 + 70 + 4 = 82, centrée à 92.
const FLECHE_DEPLIE: egui::Pos2 = egui::pos2(294.0, 301.0);
const FLECHE_REDUIT: egui::Pos2 = egui::pos2(92.0, 330.0);
/// Le total réduit : sur le bandeau du switch de camp (84 px depuis x = 8), entre y = 105 et 129.
const TOTAL_REDUIT: egui::Pos2 = egui::pos2(50.0, 117.0);

/// **Déplié et survolé** : la flèche apparaît à droite du détail, sur la ligne du fermoir,
/// pointée vers le bord gauche de l'écran.
#[test]
fn deplie_survole_montre_la_fleche_apres_le_detail() {
    let mut harness = harness_for(hote(false), false);
    harness.run();
    harness.hover_at(survol_neutre());
    harness.run();
    harness.snapshot("combat_repli_deplie_survol");
}

/// **Réduit, hors survol** : switch, total détouré sans fond, cadre — et rien d'autre.
#[test]
fn reduit_ne_garde_que_le_switch_le_total_et_le_cadre() {
    let mut harness = harness_for(hote(true), false);
    harness.run();
    harness.snapshot("combat_repli_reduit");
}

/// **Réduit et survolé** : la flèche contre le fermoir, pointée vers le centre du jeu.
#[test]
fn reduit_survole_montre_la_fleche_contre_le_fermoir() {
    let mut harness = harness_for(hote(true), false);
    harness.run();
    harness.hover_at(survol_neutre());
    harness.run();
    harness.snapshot("combat_repli_reduit_survol");
}

/// **Posé à droite, réduit et survolé** : tout passe en miroir, la flèche retournée avec le
/// décor (elle pointe toujours vers le jeu), le total reste à l'endroit.
#[test]
fn reduit_a_droite_passe_en_miroir() {
    let mut harness = harness_for(hote(true), true);
    harness.run();
    harness.hover_at(survol_neutre());
    harness.run();
    harness.snapshot("combat_repli_reduit_droite_survol");
}

/// **Ce que l'hôte attend du panneau** : la flèche remonte sa bascule — déplié comme réduit, à sa
/// place respective — et, réduit, un clic sur le total passe à la grandeur suivante.
#[test]
fn la_fleche_remonte_la_bascule_et_le_total_change_de_grandeur() {
    let deplie = hote(false);
    let mut harness = harness_for(Rc::clone(&deplie), false);
    harness.run();
    harness.hover_at(FLECHE_DEPLIE);
    harness.run();
    clic(&mut harness, FLECHE_DEPLIE);
    assert_eq!(
        deplie.bascules.get(),
        1,
        "déplié, la flèche doit être après le détail, sur la ligne du fermoir"
    );

    let reduit = hote(true);
    let mut harness = harness_for(Rc::clone(&reduit), false);
    harness.run();
    harness.hover_at(FLECHE_REDUIT);
    harness.run();
    clic(&mut harness, FLECHE_REDUIT);
    assert_eq!(
        reduit.bascules.get(),
        1,
        "réduit, la flèche doit être contre le fermoir, cadre descendu"
    );

    assert_eq!(reduit.metric.get(), CombatMetric::Damage);
    harness.hover_at(TOTAL_REDUIT);
    harness.run();
    clic(&mut harness, TOTAL_REDUIT);
    assert_eq!(
        reduit.metric.get(),
        CombatMetric::Armor,
        "réduit, un clic sur le total doit passer à la grandeur suivante"
    );
}
