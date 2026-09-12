//! Snapshot testing du bloc « ligne de sorts » du panneau Combat et de sa sélection par le cadre
//! à médaillons (`overlay_ui::panels::combat_spell_block`, spec validée en artefact les 11-12
//! sept. 2026, refonte « Sélection par le cadre » décidée le 12 sept. au soir).
//!
//! **Même règle que `tests/panels.rs`** : le `SessionSnapshot` vient du rejeu du vrai
//! `crates/overlay-engine/tests/wakfu.log` à travers le vrai `Engine`, jamais d'un littéral. Le
//! premier combat du rejeu (six alliés, `Erz-Wouaf`, `Néo-Erz-Alcool`, `Erz-Poker`…) fournit tous
//! les cas : dernier lanceur allié suivi automatiquement (liseré et point sur lui), un allié à cinq
//! sorts dont un critique (une rangée), un allié à neuf sorts (deux rangées), trois alliés qui
//! n'ont rien lancé (jamais cliquables, jamais marqués), et la vue Ennemis sans bloc.
//!
//! **Icônes de sorts : fixtures versionnées, jamais le réseau** (§17.1 du plan). Les neuf PNG de
//! `fixtures/spells/` sont injectés dans `RemoteIconStore::empty()` par `preload`, adressés par le
//! `IconRef` que `SpellIndex::embedded()` résout LUI-MÊME pour ces sorts — la même correspondance
//! nom + classe → numéro d'image qu'au runtime, pas une seconde table écrite ici (même principe
//! que `examples/shared/wakassets_fixtures.rs`). Une mise à jour du référentiel qui changerait
//! l'image de l'un d'eux fait échouer `preload_spell_fixtures` explicitement, plutôt que de laisser
//! un pavé vide passer pour normal dans la capture.
//!
//! **Interactions par nœud d'accessibilité** (`get_by_role_and_label`) plutôt que par coordonnées
//! calculées à la main : les portraits cliquables du cadre sont déclarés `Button` avec le nom de
//! l'allié, les sorts `Image` avec « Sort n : nom ». Seul le switch Alliés/Ennemis est cliqué par
//! coordonnées (les mêmes que `tests/panels.rs`).
//!
//! **Driver logiciel requis** : même prérequis que `tests/panels.rs`.

use std::sync::atomic::{AtomicU32, Ordering};

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use overlay_engine::{CatalogIndex, Engine, FightSnapshot, SessionSnapshot, SpellIndex};
use overlay_ingest::Tailer;
use overlay_ui::panels::combat::CombatSide;
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content::{
    paint_content, AuthStatus, NoopAuthSink, OverlayKind, RenderContent,
};
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
        "wakfu-overlay-testkit-spells-{}-{}",
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

/// Les sorts du premier combat du rejeu dont l'icône est fournie en fixture — nom, classe du
/// lanceur, fichier. Le numéro de fichier N'EST PAS une table parallèle : il est vérifié contre ce
/// que le référentiel résout (voir la doc de module).
const SPELL_FIXTURES: &[(&str, &str, &str, &[u8])] = &[
    (
        "Hachure",
        "ouginak",
        "6262",
        include_bytes!("../fixtures/spells/6262.png"),
    ),
    (
        "Croc-en-jambe",
        "ouginak",
        "6261",
        include_bytes!("../fixtures/spells/6261.png"),
    ),
    (
        "Canine",
        "ouginak",
        "6287",
        include_bytes!("../fixtures/spells/6287.png"),
    ),
    (
        "Coup Laiteux",
        "pandawa",
        "2196",
        include_bytes!("../fixtures/spells/2196.png"),
    ),
    (
        "Ether",
        "pandawa",
        "2208",
        include_bytes!("../fixtures/spells/2208.png"),
    ),
    // Mécaniques de classe, ajoutées au référentiel le 12 sept. (auparavant en pavé « ? »).
    (
        "Proie",
        "ouginak",
        "6283",
        include_bytes!("../fixtures/spells/6283.png"),
    ),
    (
        "Karcham",
        "pandawa",
        "2195",
        include_bytes!("../fixtures/spells/2195.png"),
    ),
    (
        "Chamrak",
        "pandawa",
        "975",
        include_bytes!("../fixtures/spells/975.png"),
    ),
    (
        "Bond du félin",
        "ecaflip",
        "983",
        include_bytes!("../fixtures/spells/983.png"),
    ),
];

fn preload_spell_fixtures(store: &RemoteIconStore) {
    let spells = SpellIndex::embedded();
    for (name, class, expected_gfx_id, bytes) in SPELL_FIXTURES {
        let entry = spells
            .find(name, Some(class))
            .unwrap_or_else(|| panic!("{name} ({class}) absent du référentiel assets/spells.json"));
        let icon = entry
            .icon
            .as_ref()
            .unwrap_or_else(|| panic!("{name} ({class}) sans image dans le référentiel"));
        assert_eq!(
            icon.gfx_id, *expected_gfx_id,
            "{name} ({class}) : le référentiel pointe désormais sur une autre image, mettre à \
             jour fixtures/spells/"
        );
        assert!(
            store.preload(icon, bytes),
            "fixture {expected_gfx_id}.png illisible"
        );
    }
}

struct Textures {
    portraits: Option<PortraitAtlas>,
    combat_frame: Option<CombatFrame>,
    icons: Option<UiIcons>,
}

impl Textures {
    fn get_or_load(&mut self, ctx: &egui::Context) -> (&PortraitAtlas, &CombatFrame, &UiIcons) {
        overlay_ui::style::apply(ctx);
        (
            self.portraits
                .get_or_insert_with(|| PortraitAtlas::load(ctx)),
            self.combat_frame
                .get_or_insert_with(|| CombatFrame::load(ctx)),
            self.icons.get_or_insert_with(|| UiIcons::load(ctx)),
        )
    }
}

/// Nom du combattant pointé par `last_ally_caster` — l'allié que la sélection automatique doit
/// suivre.
fn last_ally_caster_name(fight: &FightSnapshot) -> &str {
    let idx = fight
        .last_ally_caster
        .expect("le premier combat du rejeu a au moins un sort allié");
    &fight.fighters[idx].name
}

#[test]
fn bloc_de_sorts_selection_par_le_cadre() {
    let snapshot = replay_real_log();
    let fight: FightSnapshot = snapshot
        .fights
        .first()
        .cloned()
        .expect("au moins un combat dans le rejeu");
    assert_eq!(
        fight.fighters.iter().filter(|f| f.is_ally).count(),
        6,
        "le premier combat du rejeu doit remplir les six médaillons"
    );
    // L'état RÉEL du rejeu, vérifié plutôt que supposé : c'est lui que la première capture montre.
    let auto_selected = last_ally_caster_name(&fight).to_string();
    assert_eq!(auto_selected, "Erz-Wouaf");

    let remote_icon_store = RemoteIconStore::empty();
    preload_spell_fixtures(&remote_icon_store);
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
    let now = std::time::Instant::now();

    let mut harness = Harness::new_ui(move |ui| {
        let ctx = ui.ctx().clone();
        let (portraits, combat_frame, icons) = textures.get_or_load(&ctx);
        paint_content(
            ui,
            RenderContent {
                kind: OverlayKind::Combat,
                fight: Some(&fight),
                portraits,
                combat_frame,
                icons,
                combat_side: &mut combat_side,
                watchlist: &[],
                watchlist_toast: None,
                catalog: &catalog,
                catalog_stale: false,
                remote_icons: &remote_icon_store,
                remote_icon_textures: &mut remote_icon_textures,
                auth_status: &auth_status,
                auth_command_tx: &auth_sink,
                interactive: true,
                now,
                options: None,
            },
        );
    });

    // 1. Suivi automatique : liseré ET point sur le dernier lanceur allié (Erz-Wouaf, cinq sorts
    //    dont un critique en 4e). Erz-Mage, Erz-Vegetal et Erz-Zob n'ont rien lancé : ni marque
    //    ni bouton.
    harness.run();
    harness.snapshot("combat_spell_block_suivi_auto");
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Erz-Mage")
            .is_none(),
        "un allié sans sort ne doit pas être cliquable"
    );

    // 2. Clic sur le portrait de Néo-Erz-Alcool (neuf sorts, deux rangées) : le liseré passe sur
    //    lui, le point reste sur Erz-Wouaf, le bloc montre ses neuf sorts.
    harness
        .get_by_role_and_label(Role::Button, "Néo-Erz-Alcool")
        .click();
    harness.run();
    harness.snapshot("combat_spell_block_epingle_deux_rangees");

    // 3. Clic sur Erz-Poker : un seul sort (`Bond du félin`) — le bloc reste à une rangée.
    harness
        .get_by_role_and_label(Role::Button, "Erz-Poker")
        .click();
    harness.run();
    harness.snapshot("combat_spell_block_epingle_un_sort");

    // 4. Clic sur le porteur du point (Erz-Wouaf) : retour au suivi automatique, puis survol de
    //    son critique : liseré ACCENT et infobulle « Croc-en-jambe · Critique » au-dessus (une
    //    ligne, sans nom de lanceur).
    harness
        .get_by_role_and_label(Role::Button, "Erz-Wouaf")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Image, "Sort 4 : Croc-en-jambe")
        .hover();
    harness.run();
    harness.snapshot("combat_spell_block_survol");

    // 5. Vue Ennemis : ni bloc ni marques. Le bouton « Ennemis » du switch est cliqué par
    //    coordonnées (mêmes repères que `tests/panels.rs::panneau_combat_tooltip_switch_allies_
    //    ennemis_au_dessus`, décalés de la colonne des portraits, présente ici : 8 de marge du
    //    harnais + 8 + 70 (cadre) + 6 (`COLUMN_GAP`) + 6 (`LEADER_PANEL_PADDING`) + 30 + 15 = 143 ;
    //    8 + 50 + 13 = 71).
    harness.remove_cursor();
    harness.drag_at(egui::pos2(143.0, 71.0));
    harness.drop_at(egui::pos2(143.0, 71.0));
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Image, "Sort 1 : Proie")
            .is_none(),
        "le bloc ne doit pas exister en vue Ennemis"
    );
    harness.snapshot("combat_spell_block_vue_ennemis");
}
