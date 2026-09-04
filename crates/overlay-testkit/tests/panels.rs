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
//! **Portée actuelle, honnêtement limitée** : panneau Combat sur le dernier combat encore suivi en
//! fin de rejeu (si `Engine::snapshot().fights` en contient un), panneau Suivi à vide (rejeu sans
//! configuration de watchlist), panneau Suivi avec entrées réelles ET toast de ramassage actif
//! (voir `panneau_suivi_avec_toast_de_ramassage_ne_panique_pas` — la LISTE des entrées est une
//! configuration explicite, comme le ferait un compte lié via `Engine::set_watchlist_entries`,
//! mais le FRANCHISSEMENT à 0 qui déclenche le toast provient du vrai rejeu, jamais fabriqué à la
//! main). Combat ABSENT reste à ajouter (nécessite un log de test dédié avec un combat encore
//! `ongoing` à sa toute fin) — voir §17.1 du plan.
//!
//! **Driver logiciel requis** : `egui_kittest` (feature `wgpu`) préfère un adaptateur logiciel
//! (lavapipe/llvmpipe, voir son code source) — sous Linux, paquet système `mesa-vulkan-drivers`
//! (voir `spikes/s3-window-linux/README.md`, même prérequis que le spike S3). Sans lui, ces tests
//! échouent à la création du renderer, pas à la comparaison d'image — repli documenté, pas un bug
//! de ce fichier.

use std::sync::atomic::{AtomicU32, Ordering};

use egui_kittest::Harness;
use overlay_engine::{
    CatalogIndex, Engine, FightSnapshot, SessionSnapshot, WatchlistEntry, WatchlistKind,
    WatchlistMode,
};
use overlay_ingest::Tailer;
use overlay_ui::panels::combat::CombatSide;
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::panels::watchlist::{
    build_confetti, WatchlistToast, WatchlistToastReason, TOAST_DURATION,
};
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

#[test]
fn panneau_suivi_avec_toast_de_ramassage_ne_panique_pas() {
    // Contrairement aux deux tests précédents, la LISTE des entrées suivies est ici une
    // configuration explicite (`Engine::set_watchlist_entries`, alimentée en production par
    // `GET /api/v1/settings` — un compte lié n'est qu'un transport HTTP autour du même appel), pas
    // une donnée produite par le rejeu — voir la doc de module. Ce qui DOIT provenir du vrai
    // contenu du `wakfu.log`, c'est le franchissement à 0 qui déclenche le toast
    // (`WatchlistState::increment`), jamais fabriqué à la main.
    //
    // **Piège évité** : `Engine::ingest_batch` n'applique JAMAIS la watchlist pendant un
    // rattrapage initial (`batch.is_initial_load`, miroir du gating `currentBatchIsInitialLoad`
    // côté web — voir sa doc) : sinon lancer l'overlay sur un `wakfu.log` déjà long re-alerterait
    // sur tout ce qui a déjà été ramassé avant même son démarrage. Un scénario de toast honnête
    // doit donc reproduire les DEUX phases réelles, pas un simple rejeu en une passe :
    // 1. rattrapage complet du fichier existant tel quel, watchlist encore vide (comme au tout
    //    premier lancement, avant toute réponse `GET /api/v1/settings`) ;
    // 2. le compte "vient de répondre" : watchlist configurée maintenant, hors rattrapage ;
    // 3. le jeu continue — une ligne supplémentaire, au format réel (identique à celle trouvée
    //    ligne 537 du fichier source, « Vous avez ramassé 1x Bottes Lantha »), ajoutée à une COPIE
    //    du fichier de test après la phase de rattrapage. Seul le TIMING de cette ligne est
    //    contrôlé par le test — jamais un `WatchlistAlert`/`WatchlistToast` fabriqué à la main.
    let log_path = std::env::temp_dir().join(format!(
        "wakfu-overlay-testkit-toast-{}-{}.log",
        std::process::id(),
        NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::copy(WAKFU_LOG, &log_path).expect("copie du wakfu.log de test");

    let mut engine = test_engine();
    let mut tailer = Tailer::new(&log_path);

    // Phase 1 : rattrapage complet, avant toute configuration de watchlist.
    loop {
        let batches = tailer.poll().expect("poll() du tailer (rattrapage)");
        if batches.is_empty() {
            break;
        }
        for batch in &batches {
            engine
                .ingest_batch(batch)
                .expect("ingestion d'un lot (rattrapage)");
        }
    }

    // Phase 2 : le compte vient de répondre — cible de décompte à 1 sur un objet ramassé 15 fois
    // dans le fichier (voir ci-dessus), donc déjà purgée par le rattrapage : rien à re-déclencher
    // ici, seul un ramassage ultérieur doit compter.
    engine.set_watchlist_entries(vec![WatchlistEntry {
        name: "Bottes Lantha".to_string(),
        kind: WatchlistKind::Item,
        mode: WatchlistMode::Down,
        count: 1,
        countdown_target: 1,
        catalog_id: None,
    }]);

    // Phase 3 : le jeu continue, un nouveau ramassage réel survient après le rattrapage.
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&log_path)
            .expect("ouverture du log en écriture pour simuler la suite du jeu");
        writeln!(
            file,
            " INFO 22:03:40,000 [AWT-EventQueue-0] (aPV:174) - [Information (jeu)] Vous avez ramassé 1x Bottes Lantha ."
        )
        .expect("écriture de la ligne simulant la suite du jeu");
    }

    let mut toast: Option<WatchlistToast> = None;
    loop {
        let batches = tailer.poll().expect("poll() du tailer (suite)");
        if batches.is_empty() {
            break;
        }
        for batch in &batches {
            engine
                .ingest_batch(batch)
                .expect("ingestion d'un lot (suite)");
        }
        // Même construction que `overlay_ui::engine_thread::spawn_engine_thread` à réception
        // d'une alerte réelle — voir sa doc. Le toast le plus récent écrase le précédent, comme en
        // production (un seul emplacement affiché à la fois).
        for alert in engine.drain_watchlist_alerts() {
            let created_at = std::time::Instant::now();
            toast = Some(WatchlistToast {
                name: alert.name,
                kind: alert.kind,
                reason: WatchlistToastReason::Countdown,
                catalog_id: alert.catalog_id,
                created_at,
                confetti: build_confetti(),
                hide_at: created_at + TOAST_DURATION,
            });
        }
    }
    let _ = std::fs::remove_file(&log_path);
    let toast = toast.expect(
        "la ligne ajoutée après le rattrapage (voir la doc de ce test) doit avoir déclenché \
         l'alerte de décompte à 0",
    );
    let watchlist_entries = engine.watchlist_entries().to_vec();

    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let now = toast.created_at; // dans la fenêtre d'affichage (`TOAST_DURATION`), rendu reproductible

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
                watchlist: &watchlist_entries,
                watchlist_toast: Some(&toast),
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
    harness.snapshot("watchlist_avec_toast_ramassage");
}
