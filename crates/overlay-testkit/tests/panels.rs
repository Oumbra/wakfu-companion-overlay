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
//! main), panneau Suivi en mode `up` (voir `panneau_suivi_mode_up_ne_panique_pas`, entrée
//! construite à la main — aucun rejeu ne franchit ce mode). Combat ABSENT reste à ajouter (nécessite
//! un log de test dédié avec un combat encore `ongoing` à sa toute fin) — voir §17.1 du plan.
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
use overlay_ui::panels;
use overlay_ui::panels::combat::CombatSide;
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::panels::options_modal::{
    OptionsModalAction, OptionsModalAssets, OptionsModalState, OptionsTab,
};
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
        // `overlay_ui::style::apply` — MÊME style que les deux binaires (`main.rs`, `bin/
        // overlay-ui-x11.rs`, voir sa doc), sans quoi ces snapshots resteraient sur le thème PAR
        // DÉFAUT d'egui pour les tooltips (`egui_kittest::Harness::new_ui` crée son propre
        // `egui::Context`, qui ne passe jamais par le point de configuration des deux binaires) et
        // ne vaudraient plus rien pour vérifier visuellement le design system tooltip.
        overlay_ui::style::apply(ctx);
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
                options: None,
                options_assets: None,
            },
        );
    });

    harness.run();
    harness.snapshot("combat_apres_rejeu_reel");
}

/// Vérifie le correctif du bug rapporté (retour utilisateur : « les tooltips du switch
/// alliés/ennemis s'affichent en dessous au lieu d'au dessus [...] agrandis légèrement l'overlay
/// combat ») : l'infobulle du switch Alliés/Ennemis doit désormais s'afficher AU-DESSUS des deux
/// boutons — voir `render_content::COMBAT_TOP_MARGIN`, qui réserve la place nécessaire à
/// `RectAlign::TOP` (`combat::show_tooltip_above`) au-dessus du panneau Combat.
///
/// `fight: None` (aucun combat) plutôt qu'un rejeu réel : la colonne des portraits est alors VIDE
/// (voir `combat::show`), ce qui fixe la position du switch à des coordonnées connues et stables
/// (`x = COLUMN_GAP + LEADER_PANEL_PADDING = 12`, `y = COMBAT_TOP_MARGIN + LEADER_PANEL_PADDING =
/// 50`, dans le repère du contenu peint par `paint_content` — `COLUMN_GAP`/`LEADER_PANEL_PADDING`
/// sont privées à `combat.rs`, valeurs reprises ici à la main, même principe que `panneau_suivi_
/// tooltips_ajouter_supprimer_visibles_a_gauche` ci-dessous). `Harness::new_ui` ajoute un
/// `outer_margin(8.0)` autour de ce contenu (voir `egui_kittest::app_kind::AppKind::run_ui`) : les
/// coordonnées de survol ci-dessous l'incluent.
#[test]
fn panneau_combat_tooltip_switch_allies_ennemis_au_dessus() {
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
                options: None,
                options_assets: None,
            },
        );
    });

    harness.run();

    // Centre du bouton "Alliés" (moitié gauche du switch) : 8 (outer_margin) + 12 (x du switch) +
    // 15 (moitié de `SWITCH_OPTION_WIDTH`, 30) = 35 ; 8 + 50 (y du switch, `COMBAT_TOP_MARGIN` +
    // `LEADER_PANEL_PADDING`) + 13 (moitié de `SWITCH_HEIGHT`, 26) = 71.
    harness.hover_at(egui::pos2(35.0, 71.0));
    harness.run();
    harness.snapshot("combat_tooltip_allies_au_dessus");

    // Centre du bouton "Ennemis" (moitié droite, décalée d'un `SWITCH_OPTION_WIDTH` complet) :
    // 35 + 30 = 65 ; même y.
    harness.hover_at(egui::pos2(65.0, 71.0));
    harness.run();
    harness.snapshot("combat_tooltip_ennemis_au_dessus");
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
                options: None,
                options_assets: None,
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
                options: None,
                options_assets: None,
            },
        );
    });

    harness.run();
    harness.snapshot("watchlist_avec_toast_ramassage");
}

/// Couvre le mode `up` du compteur d'une tuile OBJET (`panels::watchlist::paint_count_inline`) —
/// jusqu'ici seul le mode `down` (voir le test précédent, avec sa fraction courant/cible en couleur
/// kamas) avait un snapshot ; ce mode-ci a sa PROPRE couleur (`TEXT_COLOR`) et c'est justement elle
/// qui a été corrigée par le troisième retour utilisateur du 2026-09-06 (« la couleur des chiffres
/// [...] c'est bien du blanc rgb(255,255,255) ? [...] j'ai l'impression que c'est ce qui change
/// véritablement la lecture ») : `0xe0e0e0` (mirroir du jeton web `--text-color`) remplacé par un
/// blanc pur, vérifié pixel par pixel sur la capture de référence du jeu. Sans ce test, une
/// régression sur CE mode précis serait passée inaperçue de toute la suite existante.
#[test]
fn panneau_suivi_mode_up_ne_panique_pas() {
    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let now = std::time::Instant::now();
    let entries = vec![WatchlistEntry {
        name: "Plume de Craqueleur".to_string(),
        kind: WatchlistKind::Item,
        mode: WatchlistMode::Up,
        count: 7,
        countdown_target: 0,
        catalog_id: None,
    }];

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
                watchlist: &entries,
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
                options_assets: None,
            },
        );
    });

    harness.run();
    harness.snapshot("watchlist_mode_up");
}

/// Reproduit le bug rapporté 2026-09-06 (deux captures d'écran à l'appui, boutons "+"/"−" du
/// bandeau Suivi) : l'infobulle "Ajouter"/"Supprimer" s'affichait à DROITE du bouton au lieu de
/// GAUCHE. Cause : `RectAlign::LEFT` (voir `panels::watchlist::show_tooltip_left`) ne peut tenir
/// que si la fenêtre a RÉELLEMENT de la place à gauche du bouton — or la fenêtre Suivi est
/// dimensionnée pile sur son contenu (`content_width`), et la colonne de contrôle en est le tout
/// premier élément, collée au bord gauche. `Harness::new_ui` (canevas 800×600 par défaut,
/// utilisé par les autres tests de ce fichier) aurait masqué le bug en donnant une marge gauche
/// que la fenêtre RÉELLE n'a jamais : ce test construit donc le harnais à la largeur EXACTE que
/// `main.rs` calculerait pour une seule entrée (`content_width(1)`, même marge 6px de chaque
/// côté que `render_content::paint_content`), seule façon de reproduire fidèlement la contrainte.
///
/// Position de survol dérivée de la mise en page (voir `panels::watchlist` : `CONTROL_TOOLTIP_
/// RESERVE`, `CONTROL_BUTTON_SIZE`, `CONTROL_BUTTON_GAP`, privées à ce module — donc recalculées
/// ici à la main plutôt qu'importées), plus l'`outer_margin(8.0)` fixe qu'`egui_kittest::AppKind::
/// run_ui` ajoute lui-même autour de tout harnais `build_ui`.
///
/// **Recalculée à la refonte 2026-09-06 (design system boutons icône)** : les deux boutons restent
/// EMPILÉS (voir doc de module — une passe intermédiaire les avait mis côte à côte par erreur,
/// corrigée sur retour utilisateur explicite le même jour), mais un fond translucide avec marge
/// (`CONTROL_BUTTON_GAP` de chaque côté) entoure désormais la colonne.
///
/// **Recalculée à nouveau le même jour** : `CONTROL_BUTTON_SIZE` 34→24 (retour utilisateur, « essaie
/// 24×24 pour voir le rendu »). Hauteur de colonne `CONTROL_BUTTON_GAP*3+24*2 = 60px`, toujours
/// PLUS HAUTE que la tuile d'entrée juste à côté (58px, `TILE_SIZE`) — de justesse —, donc encore la
/// référence de centrage vertical de `ui.horizontal`, sans décalage.
///
/// **Recalculée à la refonte 2026-09-08** (déplacement Détails/Options depuis Combat, voir doc de
/// module) : le carré passe de 1×2 à 2×2 :
/// ```text
/// [+] [−]
/// [Détails] [Options]
/// ```
/// "−" vient maintenant à DROITE de "+" (plus en dessous), avec une infobulle à DROITE
/// (`show_tooltip_right`, colonne droite — voir doc de module « infobulle par colonne, pas par
/// bouton ») au lieu de GAUCHE : ce test, à l'origine centré sur "+"/"−", couvre donc aussi
/// "Détails"/"Options" ci-dessous, mêmes abscisses que "+"/"−" (même colonne), une ligne plus bas.
///
/// Base commune : x0 = y0 = 8 (harnais) + 6 (marge Suivi) = 14. Bouton "+" : x = 14 + 88 (réserve) +
/// 4 (`CONTROL_BUTTON_GAP`, marge gauche du fond) + 12 (moitié de 24, centre du bouton) = 118 ;
/// y = 14 + 4 (même marge, haut du fond) + 12 = 30 — INCHANGÉ, "+" garde sa place. Bouton "−" :
/// x = 118 + 24 (`CONTROL_BUTTON_SIZE`) + 4 (`CONTROL_BUTTON_GAP`, écart entre les deux colonnes)
/// = 146 ; MÊME y (même ligne) = 30. "Détails" : MÊME x que "+" (même colonne) = 118 ;
/// y = 30 + 24 + 4 (`CONTROL_BUTTON_GAP`, écart entre les deux lignes) = 58. "Options" : MÊME x que
/// "−" (même colonne) = 146 ; MÊME y que "Détails" (même ligne) = 58.
#[test]
fn panneau_suivi_tooltips_par_colonne_gauche_ou_droite() {
    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let now = std::time::Instant::now();
    let entries = vec![WatchlistEntry {
        name: "Bottes Lantha".to_string(),
        kind: WatchlistKind::Item,
        mode: WatchlistMode::Down,
        count: 0,
        countdown_target: 1,
        catalog_id: None,
    }];

    let window_width = panels::watchlist::content_width(1) + 12.0;

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::Vec2::new(window_width, 150.0))
        .build_ui(move |ui| {
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
                    watchlist: &entries,
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
                    options_assets: None,
                },
            );
        });

    harness.run();

    harness.hover_at(egui::pos2(118.0, 30.0));
    harness.run();
    harness.snapshot("watchlist_tooltip_ajouter_a_gauche");

    harness.hover_at(egui::pos2(146.0, 30.0));
    harness.run();
    harness.snapshot("watchlist_tooltip_supprimer_a_droite");

    harness.hover_at(egui::pos2(118.0, 58.0));
    harness.run();
    harness.snapshot("watchlist_tooltip_details_a_gauche");

    harness.hover_at(egui::pos2(146.0, 58.0));
    harness.run();
    harness.snapshot("watchlist_tooltip_options_a_droite");
}

/// Retour utilisateur explicite 2026-09-08 : « je veux que tous les boutons se comportent EXACT de
/// la même façon que ajouter et supprimer [...] quand on clique, il repasse en mode normal et quand
/// on relâche, ils redeviennent en mode over ». Avant le correctif du 2026-09-08 (alors dans
/// `panels::icon_button::paint_icon_button`, depuis remplacé par `design::icon_button`), ce
/// n'était vrai que pour "+"/"−" (`Sense::hover()`) : un clic
/// maintenu sur "Détails"/"Options" (`Sense::click()`) gardait l'apparence "survolée" tout du long
/// (`response.hovered()` reste `true` pendant un clic pour un widget `Sense::click()`, contrairement
/// à `Sense::hover()` — comportement NATIF d'egui, voir la doc du correctif), sans jamais repasser
/// en apparence "repos".
///
/// Ce test presse (sans relâcher, `Harness::drag_at` — un simple appui maintenu, malgré son nom)
/// le bouton "Détails" et vérifie par une CAPTURE RÉELLE (pas juste une assertion sur `Response`,
/// qui n'aurait pas détecté le bug d'origine puisque `clicked()`/l'action elle-même fonctionnaient
/// déjà) qu'il repasse bien en apparence "repos" (fond/glyphe non éclaircis) malgré le curseur
/// dessus — puis qu'il revient en apparence "survolée" au relâchement (`Harness::drop_at`). Même
/// position que "Détails" dans le test précédent (118, 58) — voir son détail de calcul.
#[test]
fn panneau_suivi_clic_maintenu_repasse_en_mode_repos() {
    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let now = std::time::Instant::now();
    let entries = vec![WatchlistEntry {
        name: "Bottes Lantha".to_string(),
        kind: WatchlistKind::Item,
        mode: WatchlistMode::Down,
        count: 0,
        countdown_target: 1,
        catalog_id: None,
    }];

    let window_width = panels::watchlist::content_width(1) + 12.0;

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::Vec2::new(window_width, 150.0))
        .build_ui(move |ui| {
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
                    watchlist: &entries,
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
                    options_assets: None,
                },
            );
        });

    harness.run();

    let details_pos = egui::pos2(118.0, 58.0);

    // Survolé (curseur dessus, bouton relâché) : apparence "survolée" de référence.
    harness.hover_at(details_pos);
    harness.run();
    harness.snapshot("watchlist_details_survole_avant_clic");

    // Pressé (curseur dessus, bouton MAINTENU enfoncé, jamais relâché) : doit repasser en
    // apparence "repos" — c'est précisément le comportement que ce test protège.
    harness.drag_at(details_pos);
    harness.run();
    harness.snapshot("watchlist_details_repos_pendant_clic");

    // Relâché (curseur toujours dessus) : redevient "survolé".
    harness.drop_at(details_pos);
    harness.hover_at(details_pos);
    harness.run();
    harness.snapshot("watchlist_details_survole_apres_relachement");
}

/// Reproduit le test utilisateur 2026-09-06 (capture d'écran à l'appui) : deux décomptes à
/// GRANDES valeurs ("500/500" et "2000/2000") pour voir comment `paint_count_inline` les gère.
/// Sur l'ancien rendu (courant+"/"+cible sur une seule ligne), le texte débordait de la tuile
/// (58px) et chevauchait la tuile voisine — illisible. Ce test couvre le nouveau rendu, essai
/// demandé par l'utilisateur : la fraction cible ("/500", "/2000") passe SOUS le nombre courant,
/// même abscisse, police réduite (voir `panels::watchlist::TARGET_LINE_OFFSET`/`TARGET_FONT_SIZE`).
#[test]
fn panneau_suivi_decompte_grandes_valeurs_ne_deborde_pas() {
    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let now = std::time::Instant::now();
    let entries = vec![
        WatchlistEntry {
            name: "Mulette Bouffe Tout".to_string(),
            kind: WatchlistKind::Enemy,
            mode: WatchlistMode::Down,
            count: 500,
            countdown_target: 500,
            catalog_id: None,
        },
        WatchlistEntry {
            name: "Coiffe du Bouffe Tout".to_string(),
            kind: WatchlistKind::Item,
            mode: WatchlistMode::Down,
            count: 2000,
            countdown_target: 2000,
            catalog_id: None,
        },
    ];

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
                watchlist: &entries,
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
                options_assets: None,
            },
        );
    });

    harness.run();
    harness.snapshot("watchlist_decompte_grandes_valeurs");
}

/// Modale Options (2026-09-08, §9 du plan) — chrome pur (`panels::options_modal`), pas de rejeu de
/// log nécessaire (aucun de ses champs ne dépend d'un `SessionSnapshot`). Couvre les DEUX états
/// visuels : champ rempli sans erreur, ET message d'erreur affiché (guard de nom de fichier, voir
/// `overlay_ingest::discovery::validate_log_path`) — les deux chemins de `panels::options_modal::
/// show` qui peignent réellement des choses différentes.
#[test]
fn panneau_options_ne_panique_pas() {
    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let now = std::time::Instant::now();
    let mut options_state = OptionsModalState {
        path_input: "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log".to_string(),
        error: Some("Le fichier sélectionné doit s'appeler wakfu.log.".to_string()),
        tab: OptionsTab::default(),
    };
    // Chargées à part de `Textures` (variable locale dédiée plutôt qu'un champ supplémentaire sur
    // `Textures`, jamais utilisé par les autres tests) : `get_or_load`/cet emprunt doivent coexister
    // dans le même appel à `paint_content` sans se marcher dessus (deux emprunts `&mut` distincts,
    // sur deux variables distinctes).
    let mut options_assets: Option<OptionsModalAssets> = None;

    let mut harness = Harness::new_ui(move |ui| {
        let ctx = ui.ctx().clone();
        // Curseur de saisie FIGÉ (allumé, jamais clignotant) — depuis l'étape 1 du plan de
        // finalisation (`docs/plan-modale-options.md`), le champ de chemin prend le focus dès la
        // première frame, et son curseur apparaît donc sur cette capture. Le laisser clignoter
        // ferait dépendre le résultat du nombre de frames que `Harness::run()` juge nécessaires,
        // c'est-à-dire d'un détail d'implémentation du harnais. Seul le test est concerné : en
        // production, le curseur clignote normalement.
        ui.style_mut().visuals.text_cursor.blink = false;
        let (portraits, combat_frame, icons) = textures.get_or_load(&ctx);
        let assets = options_assets.get_or_insert_with(|| OptionsModalAssets::load(&ctx));
        paint_content(
            ui,
            RenderContent {
                kind: OverlayKind::Options,
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
                options: Some(&mut options_state),
                options_assets: Some(assets),
            },
        );
    });

    harness.run();
    harness.snapshot("options_modale_avec_erreur");
}

/// Clavier de la modale Options — étape 1 de `docs/plan-modale-options.md`.
///
/// **Le seul test de ce fichier qui ne produit aucune capture** : il ne vérifie pas un rendu mais
/// l'action remontée à l'hôte (`OptionsModalAction`), et le rendu ne change pas d'un pixel selon la
/// touche pressée.
///
/// Ce qu'il verrouille, et pourquoi ça vaut un test : avant l'étape 1, `Échap` tombait dans le filet
/// global des deux hôtes (`main.rs` / `bin/overlay-ui-x11.rs`, `event_loop.exit()`) et **fermait
/// l'overlay entier** au lieu d'annuler la saisie — la modale étant la seule fenêtre overlay
/// focalisable (§9.1 du plan), elle était aussi la seule à pouvoir déclencher ce filet.
///
/// Le champ de chemin a le focus dès la première frame (`design::input::request_focus`), donc ces
/// deux touches sont pressées **alors qu'un `TextEdit` est actif** : c'est exactement le cas où
/// elles pourraient être avalées par le champ. Elles ne le sont pas — `TextEdit` travaille sur une
/// copie filtrée des événements et laisse l'entrée globale intacte.
#[test]
fn modale_options_echap_annule_et_entree_valide() {
    const CHEMIN: &str = "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log";

    let mut options_state = OptionsModalState {
        path_input: CHEMIN.to_string(),
        error: None,
        tab: OptionsTab::default(),
    };
    let mut options_assets: Option<OptionsModalAssets> = None;
    // Les actions sont ACCUMULÉES, pas gardées une par une : `Harness::run()` rejoue plusieurs
    // frames jusqu'à stabilisation, et seule la PREMIÈRE voit l'événement clavier — retenir la
    // dernière valeur renvoyée ne verrait donc jamais que le `None` des frames suivantes.
    //
    // `RefCell` plutôt qu'un `&mut` capturé : la closure du harnais garde son emprunt pour toute sa
    // durée de vie, il faut pouvoir relire entre deux `run()` sans le rompre.
    let actions = std::cell::RefCell::new(Vec::<OptionsModalAction>::new());

    let mut harness = Harness::new_ui(|ui| {
        let ctx = ui.ctx().clone();
        let assets = options_assets.get_or_insert_with(|| OptionsModalAssets::load(&ctx));
        let action = panels::options_modal::show(ui, &mut options_state, assets);
        if action != OptionsModalAction::None {
            actions.borrow_mut().push(action);
        }
    });

    // Frames de repos : aucune touche, aucune action. Vérifie au passage que le focus initial pris
    // par le champ ne déclenche à lui seul rien du tout.
    harness.run();
    assert_eq!(actions.borrow_mut().drain(..).collect::<Vec<_>>(), vec![]);

    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(
        actions.borrow_mut().drain(..).collect::<Vec<_>>(),
        vec![OptionsModalAction::Validate(CHEMIN.to_string())],
        "Entrée doit valider le chemin courant, comme le bouton « Valider » du pied de page"
    );

    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(
        actions.borrow_mut().drain(..).collect::<Vec<_>>(),
        vec![OptionsModalAction::Cancel],
        "Échap doit annuler la modale, jamais fermer l'overlay"
    );
}

/// Modale Options **sur damier** — étape 9 de `docs/plan-modale-options.md`.
///
/// La capture `options_modale_avec_erreur` ne peut vérifier ni le coin arrondi de la fenêtre ni sa
/// translucidité : le harnais peint un panneau gris opaque derrière la modale, qui remplit le quart
/// de cercle et masque tout ce qui transparaît. Le seul rayon prononcé de toute l'interface (12,
/// tranché à la mesure le 2026-09-10 — voir `panels::options_modal::MODAL_RADIUS`) n'était donc
/// vérifié par aucun test.
///
/// Ce test peint un damier contrasté à la place de ce fond. Il y rend visibles, et donc
/// vérifiables :
///
/// - **les quatre coins arrondis** — le damier apparaît à pleine intensité dans chaque quart de
///   cercle : mesuré, le pixel (8, 8) porte la couleur exacte du damier, celui de (14, 14) celle de
///   la bannière ;
/// - **la translucidité du fond de modale** (`MODAL_BG`, alpha 235) : dans les marges latérales, le
///   contraste du damier retombe de 128 à **9,7**, soit les 8 % que cet alpha laisse passer.
///
/// Et il montre une chose qu'aucune mesure d'alpha isolée ne dit : **sous le panneau de contenu, il
/// ne reste rien du damier** (contraste 0,7). Le panneau est peint PAR-DESSUS le fond de modale, les
/// deux alphas se multiplient — 8 % de 10 % — et sa propre translucidité (`SECTION_BG`, alpha 230,
/// pourtant plus transparent que le fond) n'y change rien. Ce qu'on prendrait pour un panneau
/// translucide est en pratique opaque.
///
/// Le damier est peint **dans le `Ui` du harnais**, avant `paint_content` : c'est la seule façon
/// d'imiter ce qui se passe en production, où la fenêtre OS est transparente et laisse voir le jeu.
#[test]
fn modale_options_sur_damier_ne_panique_pas() {
    /// Côté d'une case. 12px : assez grand pour qu'un quart de cercle de rayon 12 en recouvre
    /// plusieurs — donc pour que la forme du coin se lise sans ambiguïté sur la capture.
    const CASE: f32 = 12.0;
    /// Deux teintes franchement contrastées : c'est ce contraste qui rend la translucidité
    /// mesurable. Prises dans la famille chromatique du jeu plutôt qu'en noir et blanc, pour que la
    /// planche reste comparable à une vraie scène.
    const SOMBRE: egui::Color32 = egui::Color32::from_rgb(0x24, 0x2E, 0x22);
    const CLAIR: egui::Color32 = egui::Color32::from_rgb(0xC2, 0xAE, 0x84);

    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let now = std::time::Instant::now();
    let mut options_state = OptionsModalState {
        path_input: "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log".to_string(),
        error: None,
        tab: OptionsTab::default(),
    };
    let mut options_assets: Option<OptionsModalAssets> = None;

    let mut harness = Harness::new_ui(move |ui| {
        let ctx = ui.ctx().clone();
        ui.style_mut().visuals.text_cursor.blink = false;
        let rect = ui.max_rect();
        let painter = ui.painter().clone();
        let mut y = rect.top();
        let mut ligne = 0;
        while y < rect.bottom() {
            let mut x = rect.left();
            let mut colonne = 0;
            while x < rect.right() {
                let case = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(CASE, CASE))
                    .intersect(rect);
                // La case du coin haut-gauche est CLAIRE : c'est elle que le quart de cercle
                // découpe, et un coin creusé dans une case sombre serait indiscernable du fond de
                // modale, qui est sombre lui aussi.
                painter.rect_filled(
                    case,
                    0,
                    if (ligne + colonne) % 2 == 0 {
                        CLAIR
                    } else {
                        SOMBRE
                    },
                );
                x += CASE;
                colonne += 1;
            }
            y += CASE;
            ligne += 1;
        }

        let (portraits, combat_frame, icons) = textures.get_or_load(&ctx);
        let assets = options_assets.get_or_insert_with(|| OptionsModalAssets::load(&ctx));
        paint_content(
            ui,
            RenderContent {
                kind: OverlayKind::Options,
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
                options: Some(&mut options_state),
                options_assets: Some(assets),
            },
        );
    });

    harness.run();
    harness.snapshot("options_modale_sur_damier");
}
