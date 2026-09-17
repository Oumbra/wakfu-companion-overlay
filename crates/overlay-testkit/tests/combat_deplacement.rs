//! Snapshot testing et comportement du **panneau Combat déplaçable en hauteur** (17 sept. 2026,
//! demande utilisateur) — cadenas, glyphe de replacement, poignée latérale : voir
//! `panels::combat::CombatChrome` et `overlay_ui::combat_placement`.
//!
//! **Ce que le panneau fait, et ce qu'il ne fait pas** : il ne déplace PAS sa fenêtre. Il remonte
//! le geste (`panels::drag::PanelDrag`) et deux intentions (bascule du verrou, demande de
//! replacement), à charge pour l'hôte de poser la fenêtre, de borner, d'aimanter et de persister —
//! exactement le partage de la bande Récap (voir `tests/panels.rs`, ses tests de la bande).
//!
//! **Ce que ces planches doivent montrer**, et qu'aucun test unitaire ne peut dire à notre place :
//!
//! - la pastille d'actions est au coin **BAS** du bord extérieur du panneau (demande utilisateur,
//!   « place les deux icônes en bas plutôt qu'en haut ») : elle ne décale rien et ne retaille pas
//!   la fenêtre — le contenu du panneau ne descend pas jusque-là ;
//! - **un seul cadenas, deux visages** — fermé verrouillé, ouvert sinon — et le glyphe de
//!   replacement seulement quand le panneau a été déplacé ;
//! - la **lisière de préhension ne s'encre qu'au survol**, par-dessus le panneau (et non sous le
//!   cadre des portraits, où elle serait invisible là où la main va) ;
//! - **posé à droite, tout cela passe à droite** : la pastille va à la place de son reflet, mais
//!   ses deux glyphes restent à l'endroit — un `Undo` réfléchi dirait « rétablir ».
//!
//! **Fixture : le vrai `wakfu.log`**, comme `tests/combat_miroir.rs`, sans les images de sort (le
//! suivi des sorts est coupé sur ces planches : il n'apprendrait rien de plus sur une poignée).
//!
//! **Driver logiciel requis** : même prérequis que `tests/panels.rs` — `mesa-vulkan-drivers` sous
//! Linux.

use std::sync::atomic::{AtomicU32, Ordering};

use egui_kittest::Harness;
use overlay_engine::{CatalogIndex, Engine, FightSnapshot, SessionSnapshot};
use overlay_ingest::Tailer;
use overlay_ui::panels::combat::{CombatChrome, CombatMetric, CombatSide};
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::panels::drag::PanelDrag;
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

/// Voir `tests/panels.rs::test_engine` — dossier temporaire unique, jamais le vrai dossier de
/// combats de production.
fn test_engine() -> Engine {
    let dir = std::env::temp_dir().join(format!(
        "wakfu-overlay-testkit-combat-deplacement-{}-{}",
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

/// Ce que le panneau a remonté à l'hôte, cumulé sur toutes les frames rendues — le harnais tient
/// les compteurs, comme l'hôte tiendrait son état.
#[derive(Default)]
struct Remontees {
    gestes: Vec<PanelDrag>,
    bascules: usize,
    replacements: usize,
}

/// Le panneau Combat du premier combat du rejeu, avec le `chrome` que l'hôte passerait et le côté
/// demandé. Le `chrome` est une cellule : un test le change entre deux passes, comme l'hôte le
/// ferait après un clic sur le cadenas.
fn harness_for(
    chrome: std::rc::Rc<std::cell::Cell<CombatChrome>>,
    on_right: bool,
    remontees: std::rc::Rc<std::cell::RefCell<Remontees>>,
) -> Harness<'static> {
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
    let mut combat_metric = CombatMetric::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let shortcuts = ShortcutBindings::default();
    let now = std::time::Instant::now();
    Harness::new_ui(move |ui| {
        let ctx = ui.ctx().clone();
        let (portraits, combat_frame, icons) = textures.get_or_load(&ctx);
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
                // Le suivi des sorts est coupé sur ces planches : le bloc « ligne de sorts »
                // n'apprendrait rien de plus sur une poignée, et il faudrait lui précharger ses
                // images (voir `tests/combat_spell_block.rs`).
                spells_enabled: false,
                combat_on_right: on_right,
                watchlist_selection: &mut Default::default(),
                watchlist_completions: &Default::default(),
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
                combat_chrome: chrome.get(),
                options: None,
                veiled: false,
                login: None,
            },
        );
        let mut remontees = remontees.borrow_mut();
        if outcome.combat_drag != PanelDrag::None {
            remontees.gestes.push(outcome.combat_drag);
        }
        if outcome.combat_toggle_lock {
            remontees.bascules += 1;
        }
        if outcome.combat_restore_requested {
            remontees.replacements += 1;
        }
    })
}

/// Un harnais sans rien à écouter — pour les planches, qui ne vérifient que le rendu.
fn planche(chrome: CombatChrome, on_right: bool) -> Harness<'static> {
    harness_for(
        std::rc::Rc::new(std::cell::Cell::new(chrome)),
        on_right,
        std::rc::Rc::new(std::cell::RefCell::new(Remontees::default())),
    )
}

fn press(harness: &mut Harness<'_>, pos: egui::Pos2, pressed: bool) {
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    });
}

/// Le cadenas, premier emplacement de la pastille d'actions — coin BAS extérieur du panneau
/// (2026-09-17, demande utilisateur). Le harnais laisse 8 px de marge tout autour d'un rendu de
/// 800 × 600, la pastille (22 px de haut) est donc collée sous y = 592 : voir
/// `panels::combat::paint_actions_row`.
fn cadenas() -> egui::Pos2 {
    egui::pos2(19.0, 581.0)
}

/// Le glyphe de replacement, deuxième emplacement de la même pastille.
fn replacer() -> egui::Pos2 {
    egui::pos2(39.0, 581.0)
}

/// Un point de la lisière de préhension — sur le bord extérieur, à mi-hauteur du panneau.
fn poignee() -> egui::Pos2 {
    egui::pos2(11.0, 300.0)
}

/// **Le panneau déverrouillé et déjà déplacé** : cadenas OUVERT et, à côté de lui, le glyphe de
/// replacement que seul un déplacement fait apparaître.
#[test]
fn panneau_deverrouille_et_deplace_montre_ses_deux_glyphes() {
    let mut harness = planche(
        CombatChrome {
            locked: false,
            moved: true,
        },
        false,
    );
    harness.run();
    harness.snapshot("combat_actions_deverrouille_deplace");
}

/// **Verrouillé et jamais déplacé** : un seul glyphe, le cadenas FERMÉ, et la pastille se resserre
/// dessus — « les deux ne peuvent pas vivre en même temps », et il n'y a rien à replacer.
#[test]
fn panneau_verrouille_ne_montre_que_son_cadenas() {
    let mut harness = planche(
        CombatChrome {
            locked: true,
            moved: false,
        },
        false,
    );
    harness.run();
    harness.snapshot("combat_actions_verrouille");
}

/// **La lisière de préhension, survolée** : elle s'encre par-dessus le panneau, avec ses trois
/// traits au milieu — c'est le seul retour visuel du geste avant que la fenêtre ne bouge.
#[test]
fn poignee_laterale_survolee_montre_sa_lisiere() {
    let mut harness = planche(
        CombatChrome {
            locked: false,
            moved: false,
        },
        false,
    );
    harness.run();
    harness.hover_at(poignee());
    harness.run();
    harness.snapshot("combat_poignee_survolee");
}

/// **Posé à droite** : la pastille passe à la place de son reflet, contre le bord droit, et ses
/// deux glyphes restent à l'endroit.
#[test]
fn panneau_a_droite_garde_ses_actions_au_bord_exterieur() {
    let mut harness = planche(
        CombatChrome {
            locked: false,
            moved: true,
        },
        true,
    );
    harness.run();
    harness.snapshot("combat_actions_a_droite");
}

/// **Ce que l'hôte attend du panneau**, et qu'aucune capture ne montrerait :
///
/// 1. verrouillé, un appui-glissé sur la lisière ne remonte RIEN — et rien ne s'y encre non plus ;
/// 2. déverrouillé, le même geste remonte `Started` (avec le point de saisie), `Moved`, puis
///    `Released` : c'est cette suite-là dont l'hôte déduit la hauteur, jamais un écart parcouru
///    (voir `panels::drag::PanelDrag`) ;
/// 3. le cadenas remonte sa bascule à l'hôte, qui seul tient le réglage et l'écrit ;
/// 4. le glyphe de replacement n'existe QUE si le panneau a été déplacé — au même pixel, sans
///    déplacement, il n'y a rien à cliquer.
#[test]
fn le_panneau_remonte_ses_gestes_et_jamais_verrouille() {
    let chrome = std::rc::Rc::new(std::cell::Cell::new(CombatChrome {
        locked: true,
        moved: false,
    }));
    let remontees = std::rc::Rc::new(std::cell::RefCell::new(Remontees::default()));
    let mut harness = harness_for(
        std::rc::Rc::clone(&chrome),
        false,
        std::rc::Rc::clone(&remontees),
    );

    // (1) Verrouillé : pas de geste, quoi qu'on fasse sur la lisière.
    harness.run();
    press(&mut harness, poignee(), true);
    harness.run();
    harness.event(egui::Event::PointerMoved(egui::pos2(11.0, 360.0)));
    harness.run();
    press(&mut harness, egui::pos2(11.0, 360.0), false);
    harness.run();
    assert!(
        remontees.borrow().gestes.is_empty(),
        "verrouillé, le panneau ne doit remonter aucun geste : {:?}",
        remontees.borrow().gestes
    );

    // (2) Déverrouillé : le geste complet, point de saisie compris.
    chrome.set(CombatChrome {
        locked: false,
        moved: false,
    });
    harness.run();
    press(&mut harness, poignee(), true);
    harness.run();
    harness.event(egui::Event::PointerMoved(egui::pos2(11.0, 360.0)));
    harness.run();
    press(&mut harness, egui::pos2(11.0, 360.0), false);
    harness.run();
    let gestes = remontees.borrow().gestes.clone();
    assert!(
        matches!(gestes.first(), Some(PanelDrag::Started(pos)) if *pos == poignee()),
        "le premier geste doit porter le point de saisie : {gestes:?}"
    );
    assert!(
        gestes.contains(&PanelDrag::Moved),
        "le glissement doit se poursuivre : {gestes:?}"
    );
    assert_eq!(
        gestes.last(),
        Some(&PanelDrag::Released),
        "le relâchement termine le geste, et c'est lui qui fait écrire l'hôte : {gestes:?}"
    );

    // (3) Le cadenas remonte sa bascule.
    press(&mut harness, cadenas(), true);
    harness.run();
    press(&mut harness, cadenas(), false);
    harness.run();
    assert_eq!(
        remontees.borrow().bascules,
        1,
        "le cadenas doit remonter sa bascule"
    );

    // (4) Le deuxième emplacement de la pastille n'existe pas tant que le panneau n'a pas bougé…
    press(&mut harness, replacer(), true);
    harness.run();
    press(&mut harness, replacer(), false);
    harness.run();
    assert_eq!(
        remontees.borrow().replacements,
        0,
        "sans déplacement, il n'y a pas de glyphe de replacement à cliquer"
    );

    // … et il apparaît dès que l'hôte dit que le panneau a une hauteur à lui.
    chrome.set(CombatChrome {
        locked: false,
        moved: true,
    });
    harness.run();
    press(&mut harness, replacer(), true);
    harness.run();
    press(&mut harness, replacer(), false);
    harness.run();
    assert_eq!(
        remontees.borrow().replacements,
        1,
        "déplacé, le glyphe de replacement doit remonter sa demande"
    );
}

/// **Le cadenas et le glyphe de replacement captent le glissement** (`Sense::click_and_drag`) :
/// sans cela, un appui sur eux laisserait la lisière — pourtant en dessous d'eux dans la pile —
/// recevoir le geste, et le panneau partirait à chaque clic sur son propre cadenas.
///
/// Le clic, lui, reste un clic tant que la souris ne bouge pas : c'est ce que le test précédent
/// vérifie.
#[test]
fn les_glyphes_d_actions_ne_font_pas_partir_le_panneau() {
    let chrome = std::rc::Rc::new(std::cell::Cell::new(CombatChrome {
        locked: false,
        moved: true,
    }));
    let remontees = std::rc::Rc::new(std::cell::RefCell::new(Remontees::default()));
    let mut harness = harness_for(
        std::rc::Rc::clone(&chrome),
        false,
        std::rc::Rc::clone(&remontees),
    );
    harness.run();
    press(&mut harness, cadenas(), true);
    harness.run();
    harness.event(egui::Event::PointerMoved(egui::pos2(19.0, 400.0)));
    harness.run();
    press(&mut harness, egui::pos2(19.0, 400.0), false);
    harness.run();
    assert!(
        remontees.borrow().gestes.is_empty(),
        "un glissement parti du cadenas ne doit pas déplacer le panneau : {:?}",
        remontees.borrow().gestes
    );
}
