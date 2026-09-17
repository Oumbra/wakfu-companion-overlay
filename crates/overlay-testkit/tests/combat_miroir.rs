//! Snapshot testing du panneau Combat **posé à droite** (17 sept. 2026) — case « Afficher le
//! panneau de combat à droite de la fenêtre de jeu » des Options
//! (`render_content::RenderContent::combat_on_right`, `overlay_ui::mirror`).
//!
//! **Ce que ces planches doivent montrer**, et qu'aucun test unitaire ne peut dire à notre place :
//!
//! - la DISPOSITION bascule — le cadre des portraits passe à droite, la colonne des barres à
//!   gauche, les deux bandeaux et la ligne de sorts avec elle ;
//! - **le gabarit du cadre est RETOURNÉ** : c'est du décor, son ornement doit regarder vers le
//!   jeu. C'est ce qui manquait au premier essai (« le rendu est complètement affreux ») ;
//! - **rien ne s'inverse à l'intérieur d'un bloc** : les cases du switch de camp restent Alliés
//!   puis Ennemis, celles du switch de grandeur Dégâts puis Armure puis Soins, le switch reste
//!   avant le total qu'il qualifie, le nom avant son chiffre de dégâts, et les sorts du premier
//!   au dernier ;
//! - **les images restent à l'endroit** : portraits de classe, icônes de monstre, icônes des
//!   switches, images de sort ;
//! - **l'infobulle suit** : survoler un portrait ouvre sa bulle au-dessus de lui, à sa nouvelle
//!   place.
//!
//! La planche « à gauche » est capturée avec la MÊME fixture, dans le même fichier : c'est la
//! comparaison des deux qui a du sens, et une régression du panneau ordinaire se verrait ici
//! aussi.
//!
//! **Les événements ne sont PAS traduits ici**, contrairement à la production : `paint_content` ne
//! fait que peindre, et c'est `render_content::build_ui` qui réfléchit le pointeur avant de le
//! donner à egui (voir `overlay_ui::mirror::mirror_input`, et son test unitaire). Un survol
//! demandé par l'arbre d'accessibilité vise donc la position de MISE EN PAGE — exactement ce que
//! reçoit egui en production.
//!
//! **Le curseur peint par le harnais reste, lui, à la position de mise en page** (visible sur la
//! planche d'infobulle, à gauche de l'écran alors que le portrait survolé est à droite) : il ne
//! vient pas des formes que le miroir retouche mais de `PlatformOutput::cursor_image` (voir
//! `overlay_ui::cursor`), qu'`egui_kittest` dessine par-dessus l'image finale. En production, le
//! curseur est celui du système, posé à la position RÉELLE du pointeur — donc du bon côté.
//!
//! **Fixture : le vrai `wakfu.log`**, comme `tests/panels.rs` et `tests/combat_spell_block.rs`
//! (voir leur doc de module) — six alliés dans le cadre, des sorts au dernier tour, et les images
//! de sort du lanceur suivi injectées depuis `fixtures/spells/`.
//!
//! **Driver logiciel requis** : même prérequis que `tests/panels.rs` — `mesa-vulkan-drivers` sous
//! Linux.

use std::sync::atomic::{AtomicU32, Ordering};

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable as _;
use egui_kittest::Harness;
use overlay_engine::spells::resolve_cast;
use overlay_engine::{CatalogIndex, Engine, FightSnapshot, SessionSnapshot};
use overlay_ingest::Tailer;
use overlay_ui::panels::combat::{CombatMetric, CombatSide};
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

/// Voir `tests/panels.rs::test_engine` — dossier temporaire unique, jamais le vrai dossier de
/// combats de production.
fn test_engine() -> Engine {
    let dir = std::env::temp_dir().join(format!(
        "wakfu-overlay-testkit-miroir-{}-{}",
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

/// Les sorts du dernier lanceur allié du premier combat du rejeu (Anonyme-Ouginak1), dont le bloc
/// « ligne de sorts » montre les images — sous-ensemble de `tests/combat_spell_block.rs::
/// SPELL_FIXTURES`, réduit à ce que CES planches peignent. Le numéro de fichier est vérifié
/// contre ce que le référentiel résout, comme là-bas : une image qui changerait de nom
/// casserait le test au lieu de passer en pavé « ? ».
const SPELL_FIXTURES: &[(&str, &str, &[u8])] = &[
    (
        "Hachure",
        "6262",
        include_bytes!("../fixtures/spells/6262.png"),
    ),
    (
        "Croc-en-jambe",
        "6261",
        include_bytes!("../fixtures/spells/6261.png"),
    ),
    (
        "Canine",
        "6287",
        include_bytes!("../fixtures/spells/6287.png"),
    ),
    (
        "Proie",
        "6283",
        include_bytes!("../fixtures/spells/6283.png"),
    ),
];

/// Le lanceur que le bloc suit automatiquement — celui dont les images sont préchargées.
const CASTER: &str = "Anonyme-Ouginak1";

fn preload_spell_fixtures(store: &RemoteIconStore, fight: &FightSnapshot) {
    let fighter = fight
        .fighters
        .iter()
        .find(|f| f.name == CASTER)
        .expect("le lanceur suivi est dans le premier combat du rejeu");
    for (spell, expected_gfx_id, bytes) in SPELL_FIXTURES {
        let icon = resolve_cast(fighter, spell)
            .unwrap_or_else(|| panic!("{spell} absent du référentiel de son camp"))
            .icon
            .unwrap_or_else(|| panic!("{spell} sans image dans le référentiel"));
        assert_eq!(
            icon.gfx_id, *expected_gfx_id,
            "{spell} : le référentiel pointe désormais sur une autre image, mettre à jour \
             fixtures/spells/"
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

/// Le panneau Combat du premier combat du rejeu, peint à gauche ou à droite selon
/// `combat_on_right` — tout le reste est rigoureusement identique d'une planche à l'autre, c'est
/// la condition pour que leur comparaison veuille dire quelque chose.
fn harness_for(combat_on_right: bool) -> Harness<'static> {
    let fight: FightSnapshot = replay_real_log()
        .fights
        .first()
        .cloned()
        .expect("au moins un combat dans le rejeu");
    let remote_icon_store = RemoteIconStore::empty();
    preload_spell_fixtures(&remote_icon_store, &fight);
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
        paint_content(
            ui,
            RenderContent {
                kind: OverlayKind::Combat,
                fight: Some(&fight),
                portraits,
                combat_frame,
                icons,
                // Le panneau Combat n'a ni bustes de classe ni serveurs de jeu à peindre — ils
                // n'existent que pour l'onglet « Personnages » de la fenêtre Options.
                avatars: None,
                game_servers: &Default::default(),
                combat_side: &mut combat_side,
                combat_metric: &mut combat_metric,
                watchlist: &[],
                watchlist_enabled: true,
                spells_enabled: true,
                combat_on_right,
                watchlist_selection: &mut Default::default(),
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
                options: None,
                veiled: false,
                login: None,
            },
        );
    })
}

/// La planche de référence : le panneau tel qu'il est depuis toujours, collé à gauche. Un test par
/// capture, jamais une boucle — voir `tests/combat_metric.rs`, même contrainte d'`egui_kittest`.
#[test]
fn panneau_a_gauche() {
    let mut harness = harness_for(false);
    harness.run();
    harness.snapshot("combat_miroir_gauche");
}

/// La même chose, à droite : tout est réfléchi SAUF les images (portraits, monstres, icônes des
/// switches, images de sort) et les glyphes du texte.
#[test]
fn panneau_a_droite_en_miroir() {
    let mut harness = harness_for(true);
    harness.run();
    harness.snapshot("combat_miroir_droite");
}

/// L'infobulle d'un portrait suit son médaillon : ouverte au-dessus de lui, à sa nouvelle place.
///
/// Le survol vise la position de MISE EN PAGE (celle que porte l'arbre d'accessibilité), pas la
/// position à l'écran — voir la doc de module : c'est bien ce qu'egui reçoit en production, où le
/// pointeur est traduit en amont.
#[test]
fn infobulle_de_portrait_a_droite() {
    let mut harness = harness_for(true);
    harness.run();
    harness.get_by_role_and_label(Role::Button, CASTER).hover();
    harness.run();
    harness.snapshot("combat_miroir_droite_infobulle");
}
