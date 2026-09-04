//! Snapshot testing des panneaux egui d'`overlay-ui`, offscreen, sans fenêtre système ni GPU
//! physique (§17.1 du plan) — via `egui_kittest::Harness::new_ui` + `overlay_ui::render_content::
//! paint_content` (la même fonction que la production, voir sa doc).
//!
//! **Fixtures : jamais un `SessionSnapshot`/`RenderContent` construit à la main.** Le
//! `SessionSnapshot` utilisé ici est TOUJOURS dérivé du rejeu du vrai `wakfu.log` (le même fichier
//! que le harnais de parité, `crates/overlay-engine/tests/wakfu.log`, voir §2.2 du plan) à travers
//! le vrai `overlay_engine::Engine` — réserve de la revue à trois experts (§17.6) : un littéral
//! fait main continuerait de compiler avec des données obsolètes si `SessionSnapshot` change de
//! forme, alors qu'un rejeu réel casse la compilation ou change visiblement le rendu.
//!
//! **Portée actuelle, honnêtement limitée** : seuls les états qu'un rejeu simple (sans compte lié,
//! donc sans réglages de compte ni watchlist) produit réellement sont couverts ici — panneau
//! Combat sur le dernier combat encore suivi en fin de rejeu (si `Engine::snapshot().fights` en
//! contient un), panneau Suivi à vide. Combat ABSENT, toast de ramassage, entrées suivies réelles
//! restent à ajouter (nécessitent soit un log de test dédié avec un combat encore `ongoing` à sa
//! toute fin, soit un compte lié avec des réglages de watchlist) — voir §17.1 du plan.
//!
//! **Driver logiciel requis** : `egui_kittest` (feature `wgpu`) préfère un adaptateur logiciel
//! (lavapipe/llvmpipe, voir son code source) — sous Linux, paquet système `mesa-vulkan-drivers`
//! (voir `spikes/s3-window-linux/README.md`, même prérequis que le spike S3). Sans lui, ces tests
//! échouent à la création du renderer, pas à la comparaison d'image — repli documenté, pas un bug
//! de ce fichier.

use std::sync::atomic::{AtomicU32, Ordering};

use egui_kittest::Harness;
use overlay_engine::{CatalogIndex, Engine, FightSnapshot, SessionSnapshot};
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

/// Même précaution que `overlay_engine::tests::session_real_log` : `Engine::new()`/
/// `Engine::with_watchlist_store()` seuls écriraient dans le VRAI dossier de combats de
/// production si un combat reste `ongoing` en fin de test — voir la doc d'`Engine::with_stores`.
/// Chemin temporaire unique par appel.
fn test_engine() -> Engine {
    let dir = std::env::temp_dir().join(format!(
        "wakfu-overlay-testkit-{}-{}",
        std::process::id(),
        NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    Engine::with_stores(dir.join("watchlist-counts.json"), dir.join("fights"))
        .expect("création de l'Engine")
}

/// Rejoue le vrai `wakfu.log` de bout en bout et renvoie le `SessionSnapshot` final — voir la doc
/// de module pour pourquoi ce doit TOUJOURS être la seule source d'un `RenderContent` de test,
/// jamais un littéral fait main.
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

/// Charge les trois atlas de textures une seule fois par harnais — même principe que
/// `OverlayWindow` en production (chargées une fois à la création de la fenêtre, voir
/// `overlay_ui::main`), adapté au pattern `Harness::new_ui` (la fermeture est rappelée à chaque
/// frame, voir `Harness::run`).
struct Textures {
    portraits: Option<PortraitAtlas>,
    combat_frame: Option<CombatFrame>,
    icons: Option<UiIcons>,
}

impl Textures {
    fn new() -> Self {
        Self {
            portraits: None,
            combat_frame: None,
            icons: None,
        }
    }

    fn get_or_load(&mut self, ctx: &egui::Context) -> (&PortraitAtlas, &CombatFrame, &UiIcons) {
        let portraits = self
            .portraits
            .get_or_insert_with(|| PortraitAtlas::load(ctx));
        let combat_frame = self
            .combat_frame
            .get_or_insert_with(|| CombatFrame::load(ctx));
        let icons = self.icons.get_or_insert_with(|| UiIcons::load(ctx));
        (portraits, combat_frame, icons)
    }
}

#[test]
fn panneau_combat_sur_un_vrai_rejeu_ne_panique_pas() {
    let snapshot = replay_real_log();
    // Premier combat encore suivi en fin de rejeu (`None` si tous ont déjà été purgés, voir la
    // doc de module) — dans les deux cas, c'est l'état RÉEL produit par le moteur, jamais fabriqué.
    let fight: Option<FightSnapshot> = snapshot.fights.first().cloned();

    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
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
                fight: fight.as_ref(),
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
            },
        );
    });

    harness.run();
    harness.snapshot("combat_apres_rejeu_reel");
}

#[test]
fn panneau_suivi_vide_ne_panique_pas() {
    // Rejeu réel quand même (voir la doc de module) : un rejeu sans compte lié ne produit
    // aucune entrée suivie — `&[]` est donc l'état RÉEL de `watchlist` dans ce cas précis, pas un
    // raccourci qui évite le rejeu.
    let _snapshot = replay_real_log();

    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
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
                kind: OverlayKind::Watchlist,
                fight: None,
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
            },
        );
    });

    harness.run();
    harness.snapshot("watchlist_vide");
}
