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
use overlay_ui::panels::options_modal::{OptionsModalAction, OptionsModalState, OptionsTab};
use overlay_ui::panels::watchlist::{
    build_confetti, WatchlistToast, WatchlistToastReason, TOAST_DURATION,
};
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content::{
    self, paint_content, AuthStatus, NoopAuthSink, OverlayKind, RenderContent,
};
use overlay_ui::shortcuts::ShortcutBindings;
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
    // Raccourcis PAR DÉFAUT (voir `overlay_ui::shortcuts`) : les infobulles affichent donc les
    // mêmes combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
    let shortcuts = ShortcutBindings::default();
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
                // Un état neuf par frame : aucune de ces planches n'ouvre la sélection
                // multiple du bandeau (le temporaire vit jusqu'à la fin de l'instruction).
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
                options: None,
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
    // Raccourcis PAR DÉFAUT (voir `overlay_ui::shortcuts`) : les infobulles affichent donc les
    // mêmes combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
    let shortcuts = ShortcutBindings::default();
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
                // Un état neuf par frame : aucune de ces planches n'ouvre la sélection
                // multiple du bandeau (le temporaire vit jusqu'à la fin de l'instruction).
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
                options: None,
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
    // Raccourcis PAR DÉFAUT (voir `overlay_ui::shortcuts`) : les infobulles affichent donc les
    // mêmes combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
    let shortcuts = ShortcutBindings::default();
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
                // Un état neuf par frame : aucune de ces planches n'ouvre la sélection
                // multiple du bandeau (le temporaire vit jusqu'à la fin de l'instruction).
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
                options: None,
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
                hide_at: Some(created_at + TOAST_DURATION),
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
    // Raccourcis PAR DÉFAUT (voir `overlay_ui::shortcuts`) : les infobulles affichent donc les
    // mêmes combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
    let shortcuts = ShortcutBindings::default();
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
                // Un état neuf par frame : aucune de ces planches n'ouvre la sélection
                // multiple du bandeau (le temporaire vit jusqu'à la fin de l'instruction).
                watchlist_selection: &mut Default::default(),
                watchlist_toast: Some(&toast),
                catalog: &catalog,
                catalog_stale: false,
                remote_icons: &remote_icon_store,
                remote_icon_textures: &mut remote_icon_textures,
                auth_status: &auth_status,
                auth_command_tx: &auth_sink,
                interactive: true,
                shortcuts: &shortcuts,
                now,
                options: None,
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
    // Raccourcis PAR DÉFAUT (voir `overlay_ui::shortcuts`) : les infobulles affichent donc les
    // mêmes combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
    let shortcuts = ShortcutBindings::default();
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
                // Un état neuf par frame : aucune de ces planches n'ouvre la sélection
                // multiple du bandeau (le temporaire vit jusqu'à la fin de l'instruction).
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
                options: None,
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
/// **Recalculée le 2026-09-13 (infobulles par LIGNE, voir `panels::watchlist`, doc de module)** :
/// « + »/« − » ouvraient AU-DESSUS, « Détails »/« Options » EN DESSOUS — les réserves latérales ne
/// logeaient plus les libellés rallongés de leur raccourci. Puis, le même jour, retrait de la
/// réserve d'infobulle GAUCHE et de la marge interne gauche (« il faut que l'overlay démarre au
/// début du premier bouton au niveau gauche ») : les infobulles du carré ne sont plus CENTRÉES sur
/// leur bouton mais rabattues sur le bord gauche de la fenêtre, contrepartie admise dans la même
/// demande.
///
/// **Recalculée une dernière fois le soir du 2026-09-13** (retour utilisateur, capture d'un
/// bandeau vide) : les QUATRE infobulles du carré s'ouvrent en dessous, ancrées sur le carré
/// entier — « intervertir les choses [...] on gagnerait la moitié, peut-être plus, du vide » —,
/// et la marge haute du panneau tombe à ZÉRO, la barre de défilement étant passée au-dessus des
/// tuiles (`panels::watchlist::strip_scroll_area`). Tout le contenu remonte de 34 px. Celle d'une
/// TUILE, en revanche, revient à l'écart mesuré du composant (5 px sous elle) : plus rien ne se
/// peint dessous. La réserve DROITE passe de 48 à 88 px — une infobulle ancrée sur le carré
/// déborde plus loin que centrée sur un bouton.
///
/// Base commune : x0 = 8 (harnais) + 0 (marge Suivi gauche) = 8 ; y0 = 8 + 0 (marge interne
/// haute, nulle) = 8 — **ce test n'a qu'une entrée, la bande ne déborde donc pas et la barre ne
/// prend aucune place**. Bouton "+" : x = 8 + 4 (`CONTROL_BUTTON_GAP`, marge gauche du fond) + 12
/// (moitié de 24, centre du bouton) = 24 ; y = 8 + 4 (même marge, haut du fond) + 12 = 24.
/// Bouton "−" : x = 24 + 24 (`CONTROL_BUTTON_SIZE`) + 4 (`CONTROL_BUTTON_GAP`, écart entre les
/// deux colonnes) = 52 ; MÊME y (même ligne) = 24. "Détails" : MÊME x que "+" (même colonne) = 24 ;
/// y = 24 + 24 + 4 (`CONTROL_BUTTON_GAP`, écart entre les deux lignes) = 52. "Options" : MÊME x que
/// "−" (même colonne) = 52 ; MÊME y que "Détails" (même ligne) = 52. Tuile : bord gauche à
/// 8 + 60 (`control_row_width`) + 12 (`TILE_GAP`) = 80, centre à 80 + 29 = 109 ; centre
/// vertical à 37 (voir [`BANDEAU_TUILE_0`]).
#[test]
fn panneau_suivi_toutes_les_infobulles_sous_la_bande() {
    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    // Raccourcis PAR DÉFAUT (voir `overlay_ui::shortcuts`) : les infobulles affichent donc les
    // mêmes combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
    let shortcuts = ShortcutBindings::default();
    let now = std::time::Instant::now();
    let entries = vec![WatchlistEntry {
        name: "Bottes Lantha".to_string(),
        kind: WatchlistKind::Item,
        mode: WatchlistMode::Down,
        count: 0,
        countdown_target: 1,
        catalog_id: None,
    }];

    let window_width = bandeau_largeur(1);

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::Vec2::new(window_width, BANDEAU_HAUTEUR))
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
                    // Un état neuf par frame : aucune de ces planches n'ouvre la sélection
                    // multiple du bandeau (le temporaire vit jusqu'à la fin de l'instruction).
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
                    options: None,
                },
            );
        });

    harness.run();

    for (pos, nom) in [
        (BANDEAU_PLUS, "watchlist_tooltip_ajouter_dessous"),
        (BANDEAU_MOINS, "watchlist_tooltip_supprimer_dessous"),
        (BANDEAU_DETAILS, "watchlist_tooltip_details_dessous"),
        (BANDEAU_OPTIONS, "watchlist_tooltip_options_dessous"),
        (BANDEAU_TUILE_0, "watchlist_tooltip_tuile_dessous"),
    ] {
        harness.hover_at(pos);
        harness.run();
        harness.snapshot(nom);
    }
}

/// Marge fixe qu'`egui_kittest` ajoute autour de tout harnais `build_ui` — sur les QUATRE côtés,
/// et c'est ce dernier point qui compte : une fenêtre demandée à `with_size` offre 2 × 8 px de
/// MOINS que la vraie au contenu peint dedans.
const MARGE_HARNAIS: f32 = 8.0;

/// Hauteur à demander au harnais pour que le contenu dispose exactement de ce que la vraie fenêtre
/// Suivi lui donne sans toast ni sélection : `main.rs::WATCHLIST_HEIGHT` (92 px depuis le
/// resserrement du 2026-09-13) plus `render_content::WATCHLIST_TOOLTIP_RESERVE` (28 px, passés
/// au-dessous de la bande le soir du même jour), plus les deux marges du harnais.
///
/// **Ces 16 px manquaient jusqu'au 2026-09-13**, et personne ne l'avait vu : la fenêtre d'alors
/// (168 px) était si large devant son contenu que les rogner ne coupait rien. Elle ne l'est plus,
/// et la bande défilante a été la première à en pâtir — sa barre, peinte SOUS les tuiles, tombait
/// hors du cadre et n'apparaissait sur aucune capture.
const BANDEAU_HAUTEUR: f32 = 92.0 + render_content::WATCHLIST_TOOLTIP_RESERVE + 2.0 * MARGE_HARNAIS;

/// Largeur à demander au harnais, même principe que [`BANDEAU_HAUTEUR`] : ce que `main.rs::
/// watchlist_target_width` calcule (`content_width` plus la marge interne de la fenêtre Suivi —
/// 6 px à DROITE seulement depuis le 2026-09-13, voir `render_content::paint_content`), plus les
/// deux marges du harnais.
fn bandeau_largeur(entry_count: usize) -> f32 {
    panels::watchlist::content_width(entry_count) + 6.0 + 2.0 * MARGE_HARNAIS
}

/// Centres des quatre boutons du carré de contrôle — détail du calcul dans la doc de
/// [`panneau_suivi_toutes_les_infobulles_sous_la_bande`].
const BANDEAU_PLUS: egui::Pos2 = egui::pos2(25.0, 25.0);
const BANDEAU_MOINS: egui::Pos2 = egui::pos2(55.0, 25.0);
const BANDEAU_DETAILS: egui::Pos2 = egui::pos2(25.0, 55.0);
const BANDEAU_OPTIONS: egui::Pos2 = egui::pos2(55.0, 55.0);

/// Le bandeau VIDE (retour utilisateur 2026-09-13, deux captures à l'appui — voir
/// `panels::watchlist`, doc de module, « bandeau vide : rangée 1×4 ») : sans entrée suivie, les
/// quatre boutons s'alignent en UNE rangée « + », « − », « Détails », « Options », et chaque
/// infobulle s'ouvre EN DESSOUS de son bouton, centrée — plus jamais par-dessus un voisin.
///
/// Même contrainte que [`panneau_suivi_toutes_les_infobulles_sous_la_bande`] : le harnais est
/// construit à la largeur EXACTE que `main.rs` calculerait sans entrée (`content_width(0)`, plus
/// 6 px de marge à droite), et à la hauteur réelle de la fenêtre ([`BANDEAU_HAUTEUR`]) — c'est la
/// seule façon de prouver que la réserve `CONTROL_ROW_TOOLTIP_RESERVE` suffit et que la place en
/// dessous existe.
///
/// Positions : x0 = 8 (harnais) + 0 (marge Suivi gauche, nulle depuis le 2026-09-13) = 8,
/// y0 = 8 + 0 (marge interne haute, nulle elle aussi depuis le soir du même jour) = 8. Bouton
/// "+" : x = 8 + 4 (`CONTROL_BUTTON_GAP`) + 12 (moitié de 24) = 24 ; y = 8 + 4 + 12 = 24. Chaque bouton suivant est 28 px (24 + 4) plus à
/// droite : "−" 52, "Détails" 80, "Options" 108, même y. `CONTROL_ROW_TOOLTIP_RESERVE` ne joue
/// plus qu'à DROITE : c'est elle qui donne encore à la fenêtre d'un bandeau vide de quoi contenir
/// une infobulle, qui ne peut pas se peindre hors d'elle.
#[test]
fn panneau_suivi_vide_boutons_en_ligne_infobulles_dessous() {
    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let shortcuts = ShortcutBindings::default();
    let now = std::time::Instant::now();

    let window_width = bandeau_largeur(0);

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::Vec2::new(window_width, BANDEAU_HAUTEUR))
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
                    watchlist: &[],
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
                    options: None,
                },
            );
        });

    harness.run();
    harness.snapshot("watchlist_vide_rangee");

    // **Les quatre centres, recalculés** : x = 8 (marge du harnais) + 4 (`CONTROL_BUTTON_GAP`) +
    // 12 (moitié de `CONTROL_BUTTON_SIZE`) = 24 pour « + », puis un pas de 28 px ; y = 8 + 0
    // (marge interne haute, nulle) + 4 + 12 = 24.
    //
    // Ils valaient 94/122/150/178 en x et 66 en y, hérités de la réserve d'infobulle GAUCHE
    // (`CONTROL_ROW_TOOLTIP_RESERVE`, 64 px) et de la marge haute (`WATCHLIST_TOP_MARGIN`, 28 px),
    // toutes deux retirées depuis — et jamais corrigés ici. Les captures le disaient pourtant :
    // « ajouter » et « supprimer » montraient l'infobulle d'« Options », « details » et
    // « options » n'en montraient aucune, le pointeur étant tombé hors de la rangée. Un nom de
    // fichier n'est pas une assertion ; c'est la capture publiée en artefact qui l'est.
    for (x, nom) in [
        (25.0, "ajouter"),
        (55.0, "supprimer"),
        (85.0, "details"),
        (115.0, "options"),
    ] {
        harness.hover_at(egui::pos2(x, 25.0));
        harness.run();
        harness.snapshot(format!("watchlist_vide_tooltip_{nom}_dessous"));
    }
}

/// Le harnais du bandeau in-game, avec l'état que l'hôte lui prête rendu inspectable — trois tests
/// s'en servent (sélection multiple, glisser-déposer, planche du geste).
struct Bandeau {
    harness: egui_kittest::Harness<'static>,
    /// Le pendant du champ `App::watchlist_selection`.
    selection: std::rc::Rc<std::cell::RefCell<panels::watchlist::WatchlistSelection>>,
    /// Ce que le panneau a demandé d'écrire à la dernière frame — le pendant de `RenderOutcome`.
    edition: std::rc::Rc<std::cell::RefCell<Option<panels::watchlist::WatchlistEdit>>>,
    window_width: f32,
}

fn entrees_de_bandeau() -> Vec<WatchlistEntry> {
    ["Bottes Lantha", "Bois de Frêne", "Pierre de Lune"]
        .iter()
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

fn harnais_bandeau(entries: Vec<WatchlistEntry>) -> Bandeau {
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    // Raccourcis PAR DÉFAUT (voir `overlay_ui::shortcuts`) : les infobulles affichent donc les
    // mêmes combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
    let shortcuts = ShortcutBindings::default();
    let now = std::time::Instant::now();

    let selection = Rc::new(RefCell::new(
        panels::watchlist::WatchlistSelection::default(),
    ));
    let edition: Rc<RefCell<Option<panels::watchlist::WatchlistEdit>>> =
        Rc::new(RefCell::new(None));

    let window_width = bandeau_largeur(entries.len());
    let harness = egui_kittest::Harness::builder()
        .with_size(egui::Vec2::new(window_width, BANDEAU_HAUTEUR + 88.0))
        .build_ui({
            let selection = Rc::clone(&selection);
            let edition = Rc::clone(&edition);
            move |ui| {
                let ctx = ui.ctx().clone();
                let (portraits, combat_frame, icons) = textures.get_or_load(&ctx);
                let outcome = paint_content(
                    ui,
                    RenderContent {
                        kind: OverlayKind::Watchlist,
                        fight: None,
                        portraits,
                        combat_frame,
                        icons,
                        combat_side: &mut combat_side,
                        watchlist: &entries,
                        watchlist_selection: &mut selection.borrow_mut(),
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
                        options: None,
                    },
                );
                if outcome.watchlist_edit.is_some() {
                    *edition.borrow_mut() = outcome.watchlist_edit;
                }
            }
        });
    Bandeau {
        harness,
        selection,
        edition,
        window_width,
    }
}

/// **Les boutons ne défilent pas, les tuiles si** — et la barre est celle du jeu.
///
/// Retour utilisateur du 2026-09-13, capture d'un bandeau volontairement surchargé à l'appui :
/// « j'ai fait en sorte d'avoir énormément d'objets suivis pour faire afficher la scrollbar, et un
/// point important : la scrollbar ne doit pas scroller les boutons, les boutons sont fixes ; il
/// devrait y avoir un conteneur qui affiche les items slot, un peu comme le scroll pour les
/// ennemis ». Jusque-là le carré de contrôle était le PREMIER enfant de la zone défilante : les
/// quatre actions du bandeau partaient avec les tuiles.
///
/// Même retour, seconde moitié : « je voudrais que le scroll ne s'agrandisse pas lorsque
/// l'utilisateur passe sa souris dessus [...] utiliser le scroll qui est déjà utilisé pour la
/// modale dans l'onglet Raccourcis [...] gris quand l'utilisateur n'a pas sa souris dessus et doré
/// quand il passe sa souris dessus ». Les deux captures ci-dessous montrent la même barre au repos
/// et survolée : la seule différence attendue entre elles est la TEINTE de la poignée
/// (`design::tokens::SCROLLBAR_THUMB` → `SCROLLBAR_THUMB_ACTIVE`), jamais son épaisseur.
///
/// La fenêtre est volontairement PLUS ÉTROITE que son contenu — c'est le cas réel du plafond
/// `main.rs::WATCHLIST_WIDTH_FRACTION`, seul moment où la barre existe.
///
/// Position de la poignée : la zone défilante commence après le carré (x = 8 + 60 + 12 = 80) et
/// occupe toute la hauteur que la fenêtre lui laisse ; sa barre est donc collée EN BAS de cette
/// zone, pas juste sous les tuiles — y = 128 (bas du contenu : 136 de fenêtre moins la marge du
/// harnais et les 6 px de marge interne basse) moins `STRIP_SCROLLBAR_OUTER_MARGIN` (2) moins la
/// moitié des 6 px d'épaisseur, soit 115, ce que la capture confirme (poignée sur y 112..117).
/// Elle part du bord gauche de la zone tant qu'on n'a pas défilé, et s'étend ici jusqu'à x = 280
/// (12 tuiles pour 6 visibles environ).
#[test]
fn panneau_suivi_bande_defilante_boutons_fixes() {
    let entries: Vec<WatchlistEntry> = [
        "Bottes Lantha",
        "Bois de Frêne",
        "Pierre de Lune",
        "Cuir Épais",
        "Fleur de Sel",
        "Graine de Kokoko",
        "Plume de Tofu",
        "Minerai de Fer",
        "Laine de Bouftou",
        "Écaille de Crocodaille",
        "Sève de Bambou",
        "Poil de Wabbit",
    ]
    .iter()
    .map(|nom| WatchlistEntry {
        name: (*nom).to_string(),
        kind: WatchlistKind::Item,
        mode: WatchlistMode::Up,
        count: 0,
        countdown_target: 0,
        catalog_id: None,
    })
    .collect();

    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let shortcuts = ShortcutBindings::default();
    let now = std::time::Instant::now();

    // Plafonnée, comme `main.rs::watchlist_target_width` le fait à 50 % de la largeur du jeu :
    // 12 tuiles demanderaient 900 px de plus.
    const LARGEUR_PLAFONNEE: f32 = 520.0;

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::Vec2::new(LARGEUR_PLAFONNEE, BANDEAU_HAUTEUR))
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
                    options: None,
                },
            );
        });

    harness.run();
    harness.snapshot("watchlist_bande_defilante_repos");

    // Survol de la poignée : dorée, et de la MÊME épaisseur qu'au repos.
    harness.hover_at(BANDEAU_SCROLL_POIGNEE);
    harness.run();
    harness.snapshot("watchlist_bande_defilante_survol");

    // **Et la bande défile pendant que le carré reste** : la poignée tirée vers la droite emmène
    // les tuiles, les quatre boutons ne bougent pas d'un pixel. C'est LA demande, et une capture
    // au repos ne la prouve pas — seul un défilement réel le fait.
    harness.drag_at(BANDEAU_SCROLL_POIGNEE);
    harness.run();
    harness.drop_at(BANDEAU_SCROLL_POIGNEE + egui::vec2(120.0, 0.0));
    harness.run();
    harness.snapshot("watchlist_bande_defilante_defilee");
}

/// Un point sur la poignée de la bande défilante, **en tête de bande depuis le soir du
/// 2026-09-13** (voir `design::ScrollArea::bar_before`) : y = 8 (marge du harnais) + 2
/// (`STRIP_SCROLLBAR_OUTER_MARGIN`, l'air au-dessus de la barre) + 3 (moitié de
/// `SCROLLBAR_WIDTH`) = 13. Voir le calcul complet dans
/// [`panneau_suivi_bande_defilante_boutons_fixes`].
const BANDEAU_SCROLL_POIGNEE: egui::Pos2 = egui::pos2(140.0, 13.0);

/// Centres des trois tuiles du bandeau — mêmes calculs que
/// [`panneau_suivi_le_bouton_moins_ouvre_la_selection_multiple`] : première tuile centrée en
/// x = 109, pas de 70 px (`TILE_SIZE` 58 + `TILE_GAP` 12), centre vertical en y = 37. Les deux
/// coordonnées ont reculé le 2026-09-13 avec la réserve d'infobulle GAUCHE (48 px, supprimée :
/// l'overlay démarre au premier pixel des boutons) puis la marge haute (28 px de réserve
/// d'infobulle passés sous la bande, et les 6 px restants tombés avec elle). Trois entrées ne
/// débordent pas : la barre de défilement, peinte en tête de bande, ne prend ici aucune place.
const BANDEAU_TUILE_0: egui::Pos2 = egui::pos2(116.0, 40.0);
const BANDEAU_TUILE_2: egui::Pos2 = egui::pos2(268.0, 40.0);
/// Un point de prise excentré dans la première tuile — le fantôme se tient par où on l'a pris, et
/// c'est ce décalage qui laisse voir la tuile visée dessous (voir la planche de l'onglet Suivi).
const BANDEAU_TUILE_0_PRISE: egui::Pos2 = egui::pos2(99.0, 23.0);

/// **Le glisser-déposer du bandeau rend la liste réordonnée, pas une suppression.**
///
/// Le bandeau est en lecture seule sur les entrées : il ne réordonne rien lui-même, il demande
/// (`WatchlistOutcome::edit`). Ce test vérifie les deux moitiés que le module partagé ne peut pas
/// prouver seul — que le geste souris arrive bien jusqu'au panneau sur la vraie bande, et que ce
/// qui remonte est la liste entière dans le bon ordre, avec le motif qui le dit au journal.
#[test]
fn panneau_suivi_le_glisser_deposer_reordonne_la_bande() {
    let Bandeau {
        mut harness,
        selection,
        edition,
        ..
    } = harnais_bandeau(entrees_de_bandeau());
    harness.run();

    // La croix fléchée dit que la tuile se déplace, avant même qu'on l'ait prise — `overlay_ui::
    // cursor` la traduit ensuite en bitmap du jeu.
    harness.hover_at(BANDEAU_TUILE_0);
    harness.run();
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        egui::CursorIcon::Move,
        "une tuile survolée doit annoncer qu'elle se déplace"
    );

    harness.drag_at(BANDEAU_TUILE_0);
    harness.run();
    harness.hover_at(BANDEAU_TUILE_2);
    harness.run();
    harness.drop_at(BANDEAU_TUILE_2);
    harness.run();

    let edition = edition.borrow();
    let edition = edition
        .as_ref()
        .expect("aucun réordonnancement remonté : le glisser-déposer n'est pas branché");
    assert_eq!(
        edition.reason,
        panels::watchlist::WatchlistEditReason::Reorder,
        "un déplacement ne doit pas se journaliser comme une suppression"
    );
    // Retrait puis réinsertion au rang visé, comme le web : la première entrée se pose APRÈS celle
    // qu'elle visait. Les trois entrées sont toujours là — un déplacement ne retire rien.
    assert_eq!(
        edition
            .definitions
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        ["Bois de Frêne", "Pierre de Lune", "Bottes Lantha"],
    );
    assert!(
        !selection.borrow().is_open(),
        "un déplacement ne doit pas ouvrir la sélection multiple"
    );
}

/// **Le geste, en vol sur la bande in-game** — place d'origine voilée, fantôme sous le pointeur,
/// barre d'insertion or sur la tuile visée. Le pendant de `options_suivi_deplacement` pour l'autre
/// écran : les deux partagent leur mécanique (`panels::tile_reorder`), les deux planches disent
/// qu'elle se voit pareil des deux côtés.
#[test]
fn panneau_suivi_deplacement_en_vol() {
    let Bandeau { mut harness, .. } = harnais_bandeau(entrees_de_bandeau());
    harness.run();

    harness.hover_at(BANDEAU_TUILE_0_PRISE);
    harness.run();
    harness.drag_at(BANDEAU_TUILE_0_PRISE);
    harness.run();
    harness.hover_at(BANDEAU_TUILE_2);
    harness.run();
    // Une seconde frame : le fantôme et la barre suivent le pointeur de la frame précédente.
    harness.run();
    harness.snapshot("watchlist_deplacement");
}

/// **La sélection multiple du bandeau, de bout en bout** — ouvrir, cocher, supprimer.
///
/// Retour utilisateur du 2026-09-13 : « j'ai beau appuyer sur le bouton moins, le mode de
/// suppression multiple ne s'active pas ». Il avait raison, et rien ne le disait : le « − » était
/// documenté INERTE depuis le 2026-09-06, la maquette validée depuis le matin même, et aucun test
/// ne demandait ce que le bouton FAIT — les captures existantes ne regardaient que son apparence.
///
/// Ce test clique réellement, et vérifie l'état plutôt que des pixels : c'est la seule forme qui
/// aurait attrapé un bouton peint juste et branché sur rien.
///
/// Positions : voir le détail de calcul de [`panneau_suivi_toutes_les_infobulles_sous_la_bande`]
/// pour le carré de contrôle (« − » au centre en [`BANDEAU_MOINS`]). Les tuiles suivent le carré :
/// x = 8 (marge du harnais) + 60 (`control_row_width`, 2 × 24 + 3 × 4) + 12 (`TILE_GAP`) = 80
/// pour le bord gauche de la première, soit 109 pour son centre ([`BANDEAU_TUILE_0`]). Le bouton
/// de suppression groupée est 90 px sous le haut du harnais, la bande ayant remonté de 34 px avec
/// le passage des infobulles en dessous puis celui de la barre de défilement en tête (voir
/// [`BANDEAU_HAUTEUR`]).
#[test]
fn panneau_suivi_le_bouton_moins_ouvre_la_selection_multiple() {
    let Bandeau {
        mut harness,
        selection,
        edition: restantes,
        window_width,
    } = harnais_bandeau(entrees_de_bandeau());
    harness.run();
    assert!(
        !selection.borrow().is_open(),
        "la sélection ne doit pas s'ouvrir toute seule"
    );

    // 1. Le « − » ouvre le mode, et reste enfoncé tant qu'il l'est.
    clique(&mut harness, BANDEAU_MOINS);
    assert!(
        selection.borrow().is_open(),
        "le bouton « − » n'a pas ouvert la sélection multiple",
    );
    harness.snapshot("watchlist_selection_ouverte");

    // 2. Un clic sur une tuile la coche — le bouton passe de « Supprimer tout » à « Supprimer (1) ».
    clique(&mut harness, BANDEAU_TUILE_0);
    harness.snapshot("watchlist_selection_une_cochee");

    // 3. Le bouton de suppression groupée rend les entrées RESTANTES, et referme le mode.
    let bouton = egui::pos2(window_width / 2.0, 90.0);
    clique(&mut harness, bouton);
    let restantes = restantes.borrow();
    let edition = restantes
        .as_ref()
        .expect("aucune suppression remontée : le bouton de suppression groupée n'est pas branché");
    assert_eq!(
        edition.reason,
        panels::watchlist::WatchlistEditReason::BulkRemove,
        "l'écriture demandée doit se journaliser comme une suppression, pas autrement"
    );
    assert_eq!(
        edition.definitions.len(),
        2,
        "une seule tuile était cochée : il doit rester les deux autres — restantes : {:?}",
        edition
            .definitions
            .iter()
            .map(|e| &e.name)
            .collect::<Vec<_>>(),
    );
    assert!(
        !selection.borrow().is_open(),
        "la sélection doit se refermer une fois la suppression demandée",
    );
}

/// Un clic complet : survol, appui, relâchement — chacun dans sa frame, comme un vrai geste. egui
/// rattache l'appui au widget survolé, et le pointeur n'est nulle part tant qu'aucun mouvement ne
/// l'a placé (voir `options_alertes_champ_d_ajout_trouve_et_ajoute`).
fn clique(harness: &mut egui_kittest::Harness<'_>, pos: egui::Pos2) {
    harness.hover_at(pos);
    harness.run();
    harness.drag_at(pos);
    harness.run();
    harness.drop_at(pos);
    harness.run();
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
    // Raccourcis PAR DÉFAUT (voir `overlay_ui::shortcuts`) : les infobulles affichent donc les
    // mêmes combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
    let shortcuts = ShortcutBindings::default();
    let now = std::time::Instant::now();
    let entries = vec![WatchlistEntry {
        name: "Bottes Lantha".to_string(),
        kind: WatchlistKind::Item,
        mode: WatchlistMode::Down,
        count: 0,
        countdown_target: 1,
        catalog_id: None,
    }];

    let window_width = bandeau_largeur(1);

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::Vec2::new(window_width, BANDEAU_HAUTEUR))
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
                    // Un état neuf par frame : aucune de ces planches n'ouvre la sélection
                    // multiple du bandeau (le temporaire vit jusqu'à la fin de l'instruction).
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
                    options: None,
                },
            );
        });

    harness.run();

    let details_pos = BANDEAU_DETAILS;

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
    // Raccourcis PAR DÉFAUT (voir `overlay_ui::shortcuts`) : les infobulles affichent donc les
    // mêmes combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
    let shortcuts = ShortcutBindings::default();
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
                // Un état neuf par frame : aucune de ces planches n'ouvre la sélection
                // multiple du bandeau (le temporaire vit jusqu'à la fin de l'instruction).
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
                options: None,
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
    // Raccourcis PAR DÉFAUT (voir `overlay_ui::shortcuts`) : les infobulles affichent donc les
    // mêmes combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
    let shortcuts = ShortcutBindings::default();
    let now = std::time::Instant::now();
    let mut options_state = OptionsModalState {
        suivi: Default::default(),
        suivi_draft: None,
        suivi_availability: Default::default(),
        path_input: "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log".to_string(),
        error: Some("Le fichier sélectionné doit s'appeler wakfu.log.".to_string()),
        // L'onglet du chemin de log, explicitement : la fenêtre s'ouvre sur « Alertes » depuis
        // le 2026-09-12, et c'est le champ de chemin que ce test regarde.
        tab: OptionsTab::Parametres,
        ..Default::default()
    };
    // Chargées à part de `Textures` (variable locale dédiée plutôt qu'un champ supplémentaire sur
    // `Textures`, jamais utilisé par les autres tests) : `get_or_load`/cet emprunt doivent coexister
    // dans le même appel à `paint_content` sans se marcher dessus (deux emprunts `&mut` distincts,
    // sur deux variables distinctes).

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
                // Un état neuf par frame : aucune de ces planches n'ouvre la sélection
                // multiple du bandeau (le temporaire vit jusqu'à la fin de l'instruction).
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
                options: Some(&mut options_state),
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
        suivi: Default::default(),
        suivi_draft: None,
        suivi_availability: Default::default(),
        path_input: CHEMIN.to_string(),
        error: None,
        // L'onglet du chemin de log, explicitement : la fenêtre s'ouvre sur « Alertes » depuis
        // le 2026-09-12, et c'est le champ de chemin que ce test regarde.
        tab: OptionsTab::Parametres,
        // **Référence = ce qui est affiché** : cette fenêtre est intouchée, donc Échap l'annule du
        // premier coup. Une référence vide la rendrait « modifiée » dès l'ouverture, et Échap
        // ouvrirait la garde au lieu d'annuler — voir `options_garde_de_fermeture_au_clavier`.
        initial: overlay_ui::panels::options_modal::OptionsInitial {
            // L'onglet « Suivi » n'entre dans aucune de ces captures : un brouillon vide suffit à
            // satisfaire l'initialiseur sans rien changer à ce qu'elles vérifient.
            suivi: None,
            path: CHEMIN.to_string(),
            alerts: None,
            ..Default::default()
        },
        ..Default::default()
    };
    // Les actions sont ACCUMULÉES, pas gardées une par une : `Harness::run()` rejoue plusieurs
    // frames jusqu'à stabilisation, et seule la PREMIÈRE voit l'événement clavier — retenir la
    // dernière valeur renvoyée ne verrait donc jamais que le `None` des frames suivantes.
    //
    // `RefCell` plutôt qu'un `&mut` capturé : la closure du harnais garde son emprunt pour toute sa
    // durée de vie, il faut pouvoir relire entre deux `run()` sans le rompre.
    let actions = std::cell::RefCell::new(Vec::<OptionsModalAction>::new());

    let mut harness = Harness::new_ui(|ui| {
        // Même style que les binaires et que les autres harnais de ce fichier : c'est lui qui
        // installe les polices du design system. Ce test ne compare pas d'image, l'oubli n'y
        // avait donc aucun symptôme visible — il peignait la modale entière dans la
        // proportionnelle par défaut d'egui. Repéré le 2026-09-10 par l'assertion que
        // `design::text::famille` porte désormais.
        overlay_ui::style::apply(ui.ctx());
        // L'onglet « Alertes » a besoin du catalogue et des icônes distantes depuis le
        // 2026-09-12 ; ce test-ci ne quitte jamais « Paramètres », tout est donc vide.
        let icons = UiIcons::load(ui.ctx());
        let remote_icons = RemoteIconStore::empty();
        let mut remote_icon_textures = RemoteIconTextures::default();
        let catalog = CatalogIndex::default();
        let action = panels::options_modal::show(
            ui,
            &mut options_state,
            &mut panels::options_modal::OptionsModalContext {
                catalog: &catalog,
                remote_icons: &remote_icons,
                remote_icon_textures: &mut remote_icon_textures,
                icons: &icons,
            },
        );
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
        vec![OptionsModalAction::Validate(
            overlay_ui::panels::options_modal::OptionsCommit {
                path: CHEMIN.to_string(),
                // La case « Afficher le panneau de combat en dehors des combats » est décochée
                // dans cet état, et personne ne l'a touchée : « Valider » emporte le réglage tel
                // qu'il est, jamais un défaut recalculé au passage.
                combat_always_visible: false,
                // Idem pour les raccourcis : personne n'a ouvert l'onglet « Raccourcis », le
                // brouillon est celui qu'on a posé à l'ouverture (les défauts ici).
                shortcuts: ShortcutBindings::default(),
            }
        )],
        "Entrée doit valider les réglages courants, comme le bouton « Valider » du pied de page"
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
///   la bannière. Les deux coins BAS ne viennent plus d'un `corner_radius` mais de l'alpha de
///   `modal-body.png`, qui porte le même quart de cercle de rayon 12 ;
/// - **la translucidité du fond de modale** (`MODAL_BODY_TINT`, alpha 235) : dans les marges
///   latérales, le contraste du damier retombe de 128 à **9,7**, soit les 8 % que cet alpha laisse
///   passer. Cette valeur n'a pas bougé quand le fond est passé de l'aplat `MODAL_BG` à la texture
///   `DsTexture::ModalBody` (2026-09-10) : la teinte de peinture reprend exactement l'alpha que
///   portait la couleur.
///
/// Il montre aussi le **grain et les hachures d'angle** que la texture de corps apporte depuis le
/// 2026-09-10 : dans les marges, l'écart au fond nu passe de 0 à environ 6 niveaux là où un
/// croisillon court.
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
    // Raccourcis PAR DÉFAUT (voir `overlay_ui::shortcuts`) : les infobulles affichent donc les
    // mêmes combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
    let shortcuts = ShortcutBindings::default();
    let now = std::time::Instant::now();
    let mut options_state = OptionsModalState {
        suivi: Default::default(),
        suivi_draft: None,
        suivi_availability: Default::default(),
        path_input: "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log".to_string(),
        error: None,
        // L'onglet du chemin de log, explicitement : la fenêtre s'ouvre sur « Alertes » depuis
        // le 2026-09-12, et c'est le champ de chemin que ce test regarde.
        tab: OptionsTab::Parametres,
        ..Default::default()
    };

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
                // Un état neuf par frame : aucune de ces planches n'ouvre la sélection
                // multiple du bandeau (le temporaire vit jusqu'à la fin de l'instruction).
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
                options: Some(&mut options_state),
            },
        );
    });

    harness.run();
    harness.snapshot("options_modale_sur_damier");
}

/// Rend l'onglet « Alertes » de la fenêtre Options dans un état donné, et le capture.
///
/// **Le profil vient du moteur**, pas d'une liste écrite à la main : `AlertProfile::default()`
/// donne les dix `DEFAULT_SOUND_ITEM_NAMES` fusionnés, exactement ce qu'un compte neuf renvoie.
/// L'objet ajouté par le joueur est poussé par-dessus — c'est le seul qui porte une croix de
/// retrait, et les captures doivent montrer cette règle.
///
/// Les icônes d'objets restent le repli générique : elles viennent du CDN, hors de portée du
/// harnais (aucun réseau dans un test).
///
/// **Un `Harness` par test, jamais plusieurs** : `egui_kittest` refuse que deux jeux de résultats
/// de snapshot soient abandonnés séparément dans le même test (« Multiple SnapshotResults were
/// dropped without being handled »), ce qui casserait la mise à jour groupée des images.
fn capture_onglet_alertes(nom: &str, manual_close: bool) {
    use overlay_ui::panels::alerts_tab::{AlertsAvailability, AlertsTabState};

    let mut profile = overlay_engine::AlertProfile::default();
    profile.add("Combinaison Lardante", Some(4242));
    profile.manual_close = manual_close;
    // Un objet au son coupé dans la capture : c'est l'autre moitié de ce que la tuile dit.
    profile.toggle("Influence III", None);

    let mut options_state = OptionsModalState {
        suivi: Default::default(),
        suivi_draft: None,
        suivi_availability: Default::default(),
        path_input: String::new(),
        error: None,
        tab: OptionsTab::Alertes,
        alerts: AlertsTabState {
            duration_input: "3,5".to_string(),
            ..Default::default()
        },
        alerts_draft: Some(profile),
        alerts_availability: AlertsAvailability::Ready,
        initial: Default::default(),
        pending_close: false,
        ..Default::default()
    };

    // **À la taille réelle de la fenêtre** (`options_modal::WINDOW_SIZE`, 760 × 810 depuis que cet
    // onglet existe) et non aux 800 × 600 par défaut du harnais : c'est cette taille qui donne à la
    // grille ses cinq tuiles par rangée et ses trois rangées visibles.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            let catalog = CatalogIndex::default();
            panels::options_modal::show(
                ui,
                &mut options_state,
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();
    harness.snapshot(nom);
}

/// **L'onglet « Alertes »** — celui qui manquait jusqu'au 2026-09-12 : la mécanique d'alerte
/// existait, mais l'entrée de menu était désactivée et la liste ne se réglait que depuis le site.
#[test]
fn options_onglet_alertes_liste() {
    capture_onglet_alertes("options_alertes_liste", false);
}

/// Fermeture manuelle : le champ de durée se grise. La logique existait (`.enabled`), aucun rendu
/// ne la montrait.
#[test]
fn options_onglet_alertes_fermeture_manuelle() {
    capture_onglet_alertes("options_alertes_fermeture_manuelle", true);
}

/// **L'onglet « Raccourcis »** (2026-09-13) — celui qui personnalise les raccourcis clavier
/// globaux, demandé « avant paramètre ».
///
/// La capture porte les trois états que cet onglet peut montrer en même temps : une combinaison
/// PERSONNALISÉE (`Ctrl+Alt+F9` sur « Rafraîchir l'affichage », qui prouve que la liste affiche les
/// combinaisons effectives et non les défauts), une case EN ÉCOUTE (bord d'alerte et « Tapez la
/// combinaison… »), et le message d'une frappe refusée. Le reste — groupes, en-têtes de tableau,
/// champ de recherche, bouton « Réinitialiser » — vient avec.
/// **La déconnexion passe par une confirmation, et Échap y répond « Non ».**
///
/// Deux règles en une : le bouton ne déconnecte jamais du premier clic (l'action efface la session
/// et renvoie l'overlay à son écran de connexion, « Annuler » ne la rattraperait pas), et l'Échap
/// qui ferme la boîte ne doit PAS être relu par le filet clavier de la fenêtre — sinon le même
/// appui fermerait la boîte ET la fenêtre derrière, le bug déjà attrapé pour la garde de fermeture.
#[test]
fn options_deconnexion_confirmee_et_echap_repond_non() {
    const CHEMIN: &str = "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log";

    let mut options_state = OptionsModalState {
        tab: OptionsTab::Parametres,
        path_input: CHEMIN.to_string(),
        account_connected: true,
        // La confirmation est ouverte d'entrée : c'est l'état où l'appui d'Échap se joue, et le
        // clic sur le bouton qui l'ouvre est déjà couvert par le composant `design::button`.
        pending_disconnect: true,
        initial: overlay_ui::panels::options_modal::OptionsInitial {
            path: CHEMIN.to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    let actions = std::cell::RefCell::new(Vec::<OptionsModalAction>::new());
    let confirmation_ouverte = std::cell::Cell::new(false);

    let mut harness = Harness::new_ui(|ui| {
        overlay_ui::style::apply(ui.ctx());
        let icons = UiIcons::load(ui.ctx());
        let remote_icons = RemoteIconStore::empty();
        let mut remote_icon_textures = RemoteIconTextures::default();
        let catalog = CatalogIndex::default();
        let action = panels::options_modal::show(
            ui,
            &mut options_state,
            &mut panels::options_modal::OptionsModalContext {
                catalog: &catalog,
                remote_icons: &remote_icons,
                remote_icon_textures: &mut remote_icon_textures,
                icons: &icons,
            },
        );
        if action != OptionsModalAction::None {
            actions.borrow_mut().push(action);
        }
        confirmation_ouverte.set(options_state.pending_disconnect);
    });

    // Frames de repos : la boîte reste ouverte, et rien n'est déconnecté tant qu'on n'a pas répondu.
    harness.run();
    assert!(
        confirmation_ouverte.get(),
        "la confirmation s'est fermée seule"
    );
    assert_eq!(actions.borrow_mut().drain(..).collect::<Vec<_>>(), vec![]);

    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(
        !confirmation_ouverte.get(),
        "Échap doit fermer la confirmation"
    );
    assert_eq!(
        actions.borrow_mut().drain(..).collect::<Vec<_>>(),
        vec![],
        "Échap répond « Non » : ni déconnexion, ni fermeture de la fenêtre derrière"
    );
}

/// **La section « Compte » de l'onglet « Paramètres »** (2026-09-13) — la déconnexion, qui était
/// jusque-là un raccourci global (`Ctrl+Alt+D`), devenue un bouton avec ce qu'il faut pour
/// comprendre ce qu'il fait avant de le presser.
///
/// Capturée **compte connecté** (`account_connected: true`), l'état où le bouton est actif : à
/// `false` — le cas du binaire Linux, sans compte — il est grisé et son infobulle le dit.
#[test]
fn options_parametres_section_compte() {
    const CHEMIN: &str = "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log";

    let mut options_state = OptionsModalState {
        tab: OptionsTab::Parametres,
        path_input: CHEMIN.to_string(),
        account_connected: true,
        initial: overlay_ui::panels::options_modal::OptionsInitial {
            path: CHEMIN.to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            let catalog = CatalogIndex::default();
            panels::options_modal::show(
                ui,
                &mut options_state,
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();
    harness.snapshot("options_parametres_compte");
}

/// **La confirmation de déconnexion** — l'autre moitié de la section « Compte ». Le bouton est la
/// seule action de cette fenêtre qui échappe à « Annuler » (l'hôte efface le jeton dès qu'elle
/// répond « Oui ») : cette boîte est ce qui rattrape le clic par inadvertance, et son voile doit
/// couvrir la fenêtre ENTIÈRE, pied de page compris, pour que rien ne reste cliquable derrière.
///
/// La logique — Échap répond « Non » sans fermer la fenêtre derrière — est couverte par
/// `options_deconnexion_confirmee_et_echap_repond_non` ; ici, seul le rendu est figé.
#[test]
fn options_deconnexion_confirmation() {
    const CHEMIN: &str = "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log";

    let mut options_state = OptionsModalState {
        tab: OptionsTab::Parametres,
        path_input: CHEMIN.to_string(),
        account_connected: true,
        pending_disconnect: true,
        initial: overlay_ui::panels::options_modal::OptionsInitial {
            path: CHEMIN.to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            let catalog = CatalogIndex::default();
            panels::options_modal::show(
                ui,
                &mut options_state,
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();
    harness.snapshot("options_deconnexion_confirmation");
}

#[test]
fn options_onglet_raccourcis() {
    use overlay_ui::panels::raccourcis_tab::RaccourcisTabState;
    use overlay_ui::shortcuts::{Shortcut, ShortcutAction};

    let mut shortcuts = ShortcutBindings::default();
    shortcuts.set(
        ShortcutAction::Refresh,
        Shortcut::parse("Ctrl+Alt+F9").expect("combinaison de test valide"),
    );

    let mut options_state = OptionsModalState {
        tab: OptionsTab::Raccourcis,
        path_input: "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log".to_string(),
        shortcuts,
        raccourcis: RaccourcisTabState {
            capturing: Some(ShortcutAction::Quit),
            error: Some(panels::raccourcis_tab::MESSAGE_COMBINAISON_REFUSEE.to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    // À la taille réelle de la fenêtre, comme les captures des autres onglets : c'est elle qui
    // décide combien de raccourcis se lisent sans défiler.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            let catalog = CatalogIndex::default();
            panels::options_modal::show(
                ui,
                &mut options_state,
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();
    harness.snapshot("options_raccourcis");
}

/// La section « Multicompte » (2026-09-13, §9.1 sexies du plan) est la DERNIÈRE de la liste : elle
/// est hors écran sur la capture ci-dessus, qui montre le haut de l'onglet. Le champ « Rechercher »
/// la ramène — et c'est aussi la seule capture qui montre des combinaisons par défaut SANS
/// modificateur (F1/F2), l'exception que ces deux raccourcis ont introduite.
#[test]
fn options_raccourcis_section_multicompte() {
    use overlay_ui::panels::raccourcis_tab::RaccourcisTabState;

    let mut options_state = OptionsModalState {
        tab: OptionsTab::Raccourcis,
        path_input: "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log".to_string(),
        shortcuts: ShortcutBindings::default(),
        raccourcis: RaccourcisTabState {
            search: "multicompte".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            let catalog = CatalogIndex::default();
            panels::options_modal::show(
                ui,
                &mut options_state,
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();
    harness.snapshot("options_raccourcis_multicompte");
}

/// **La garde de fermeture** — Échap et « Annuler » demandent confirmation tant que des
/// modifications sont en attente.
///
/// Sans elle, un joueur qui ajoute trois objets puis ferme par réflexe perd tout, en silence : la
/// fenêtre est transactionnelle, rien n'est écrit avant « Valider ».
///
/// Ce test vérifie les deux moitiés de la règle, et la seconde compte autant que la première : une
/// fenêtre **intouchée** doit se fermer du premier coup. Une garde qui se déclencherait toujours
/// ajouterait un clic à chaque consultation.
#[test]
fn options_garde_de_fermeture() {
    use overlay_ui::panels::alerts_tab::AlertsAvailability;

    const CHEMIN: &str = "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log";

    let profil = overlay_engine::AlertProfile::default();
    let mut state = OptionsModalState {
        suivi: Default::default(),
        suivi_draft: None,
        suivi_availability: Default::default(),
        path_input: CHEMIN.to_string(),
        error: None,
        tab: OptionsTab::Alertes,
        alerts: Default::default(),
        alerts_draft: Some(profil.clone()),
        alerts_availability: AlertsAvailability::Ready,
        initial: overlay_ui::panels::options_modal::OptionsInitial {
            // L'onglet « Suivi » n'entre dans aucune de ces captures : un brouillon vide suffit à
            // satisfaire l'initialiseur sans rien changer à ce qu'elles vérifient.
            suivi: None,
            path: CHEMIN.to_string(),
            alerts: Some(profil),
            ..Default::default()
        },
        pending_close: false,
        ..Default::default()
    };

    // Rien n'a bougé : la fenêtre est propre.
    assert!(!state.is_dirty(), "fenêtre intouchée déclarée modifiée");

    // Un objet ajouté au brouillon suffit à la salir.
    state
        .alerts_draft
        .as_mut()
        .expect("brouillon")
        .add("Combinaison Lardante", Some(4242));
    assert!(state.is_dirty(), "ajout non détecté");

    // Et un chemin retouché aussi, sur l'autre onglet : la garde couvre la FENÊTRE, pas un onglet.
    let mut autre = state.clone();
    autre.alerts_draft = autre.initial.alerts.clone();
    assert!(!autre.is_dirty());
    autre.path_input = "/autre/chemin/wakfu.log".to_string();
    assert!(autre.is_dirty(), "chemin modifié non détecté");
}

/// La garde **à l'écran** : la boîte du jeu, sur un voile qui couvre la fenêtre entière.
#[test]
fn options_garde_de_fermeture_a_l_ecran() {
    use overlay_ui::panels::alerts_tab::AlertsAvailability;

    let mut profil = overlay_engine::AlertProfile::default();
    profil.add("Combinaison Lardante", Some(4242));
    let mut options_state = OptionsModalState {
        suivi: Default::default(),
        suivi_draft: None,
        suivi_availability: Default::default(),
        path_input: String::new(),
        error: None,
        tab: OptionsTab::Alertes,
        alerts: Default::default(),
        alerts_draft: Some(profil),
        alerts_availability: AlertsAvailability::Ready,
        initial: Default::default(),
        pending_close: true,
        ..Default::default()
    };

    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            let catalog = CatalogIndex::default();
            panels::options_modal::show(
                ui,
                &mut options_state,
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();
    harness.snapshot("options_garde_fermeture");
}

/// Le pendant de `modale_options_echap_annule_et_entree_valide` : **Échap sur une fenêtre modifiée
/// n'annule plus**, il ouvre la garde.
///
/// Les deux tests disent ensemble toute la règle. Celui-ci seul laisserait passer une garde qui se
/// déclenche toujours ; l'autre seul, une garde qui ne se déclenche jamais.
#[test]
fn options_garde_de_fermeture_au_clavier() {
    use overlay_ui::panels::alerts_tab::AlertsAvailability;

    const CHEMIN: &str = "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log";

    let reference = overlay_engine::AlertProfile::default();
    let mut brouillon = reference.clone();
    brouillon.add("Combinaison Lardante", Some(4242));

    let mut options_state = OptionsModalState {
        suivi: Default::default(),
        suivi_draft: None,
        suivi_availability: Default::default(),
        path_input: CHEMIN.to_string(),
        error: None,
        tab: OptionsTab::Alertes,
        alerts: Default::default(),
        alerts_draft: Some(brouillon),
        alerts_availability: AlertsAvailability::Ready,
        initial: overlay_ui::panels::options_modal::OptionsInitial {
            // L'onglet « Suivi » n'entre dans aucune de ces captures : un brouillon vide suffit à
            // satisfaire l'initialiseur sans rien changer à ce qu'elles vérifient.
            suivi: None,
            path: CHEMIN.to_string(),
            alerts: Some(reference),
            ..Default::default()
        },
        pending_close: false,
        ..Default::default()
    };
    let actions = std::cell::RefCell::new(Vec::<OptionsModalAction>::new());
    let garde_ouverte = std::cell::Cell::new(false);

    let mut harness = Harness::new_ui(|ui| {
        overlay_ui::style::apply(ui.ctx());
        let icons = UiIcons::load(ui.ctx());
        let remote_icons = RemoteIconStore::empty();
        let mut remote_icon_textures = RemoteIconTextures::default();
        let catalog = CatalogIndex::default();
        let action = panels::options_modal::show(
            ui,
            &mut options_state,
            &mut panels::options_modal::OptionsModalContext {
                catalog: &catalog,
                remote_icons: &remote_icons,
                remote_icon_textures: &mut remote_icon_textures,
                icons: &icons,
            },
        );
        if action != OptionsModalAction::None {
            actions.borrow_mut().push(action);
        }
        garde_ouverte.set(options_state.pending_close);
    });

    harness.run();
    assert!(
        !garde_ouverte.get(),
        "garde ouverte sans qu'on ait rien demandé"
    );

    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(
        actions.borrow_mut().drain(..).collect::<Vec<_>>(),
        vec![],
        "Échap ne doit RIEN fermer tant que des modifications sont en attente"
    );
    assert!(garde_ouverte.get(), "Échap aurait dû ouvrir la garde");

    // Second appui : c'est la BOÎTE qui prend Échap, et elle répond « Non » — la fenêtre reste
    // ouverte. Une touche ne confirme pas un abandon.
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(
        actions.borrow_mut().drain(..).collect::<Vec<_>>(),
        vec![],
        "Échap dans la garde ne doit pas fermer la fenêtre"
    );
    assert!(
        !garde_ouverte.get(),
        "Échap dans la garde aurait dû la refermer"
    );
}

/// **La croix de la bannière est un « Annuler » de plus** (demande du 2026-09-13, sur captures du
/// jeu) : même garde, même action.
///
/// Le centre de la croix : la fenêtre occupe (8, 8)-(752, 802) dans le harnais (8 px de marge de
/// chaque côté), son carré de 32 px est à 12 px du bord droit et centré dans la bannière de 56 —
/// soit (752 − 12 − 16, 8 + 28) = (724, 36).
///
/// Deux clics, et le second compte autant que le premier : une fenêtre **intouchée** doit se
/// fermer du premier coup, une fenêtre modifiée ne doit PAS se fermer mais ouvrir la garde. Le
/// test est écrit sur la même fenêtre pour que la position cliquée soit exactement la même — un
/// clic qui manquerait la croix passerait pour une garde qui fonctionne.
#[test]
fn options_croix_de_la_banniere_ferme_comme_annuler() {
    use overlay_ui::panels::alerts_tab::AlertsAvailability;

    const CHEMIN: &str = "/home/joueur/.config/zaap/gamesLogs/wakfu/wakfu.log";
    const CROIX: egui::Pos2 = egui::pos2(752.0 - 12.0 - 16.0, 8.0 + 28.0);

    let reference = overlay_engine::AlertProfile::default();
    let mut options_state = OptionsModalState {
        suivi: Default::default(),
        suivi_draft: None,
        suivi_availability: Default::default(),
        path_input: CHEMIN.to_string(),
        error: None,
        tab: OptionsTab::Alertes,
        alerts: Default::default(),
        alerts_draft: Some(reference.clone()),
        alerts_availability: AlertsAvailability::Ready,
        initial: overlay_ui::panels::options_modal::OptionsInitial {
            suivi: None,
            path: CHEMIN.to_string(),
            alerts: Some(reference),
            ..Default::default()
        },
        pending_close: false,
        ..Default::default()
    };
    let actions = std::cell::RefCell::new(Vec::<OptionsModalAction>::new());
    let garde_ouverte = std::cell::Cell::new(false);
    // Posé par le test entre deux clics, appliqué au brouillon depuis la closure — le harnais
    // possède l'état, on ne peut plus y toucher directement une fois construit.
    let salir = std::cell::Cell::new(false);

    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(|ui| {
            overlay_ui::style::apply(ui.ctx());
            if salir.take() {
                options_state
                    .alerts_draft
                    .as_mut()
                    .expect("brouillon")
                    .add("Combinaison Lardante", Some(4242));
            }
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            let catalog = CatalogIndex::default();
            let action = panels::options_modal::show(
                ui,
                &mut options_state,
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
            if action != OptionsModalAction::None {
                actions.borrow_mut().push(action);
            }
            garde_ouverte.set(options_state.pending_close);
        });

    harness.run();
    assert!(actions.borrow().is_empty(), "action sans clic");

    // Fenêtre intouchée : la croix ferme du premier coup, sans garde.
    harness.hover_at(CROIX);
    harness.run();
    harness.snapshot("options_croix_survolee");
    harness.drag_at(CROIX);
    harness.drop_at(CROIX);
    harness.run();
    assert_eq!(
        actions.borrow_mut().drain(..).collect::<Vec<_>>(),
        vec![OptionsModalAction::Cancel],
        "la croix d'une fenêtre intouchée doit fermer, comme « Annuler »"
    );
    assert!(
        !garde_ouverte.get(),
        "garde ouverte sur une fenêtre intouchée"
    );

    // Fenêtre modifiée : la croix n'agit pas, elle ouvre la garde.
    salir.set(true);
    harness.run();
    harness.drag_at(CROIX);
    harness.drop_at(CROIX);
    harness.run();
    assert_eq!(
        actions.borrow_mut().drain(..).collect::<Vec<_>>(),
        vec![],
        "la croix ne doit RIEN fermer tant que des modifications sont en attente"
    );
    assert!(garde_ouverte.get(), "la croix aurait dû ouvrir la garde");
}

/// **L'infobulle du nom d'objet** — celle que `design::label` pose quand le nom est coupé.
///
/// Deux captures, et la seconde compte autant que la première : un nom qui tient en entier ne doit
/// PAS révéler d'infobulle de nom (elle répéterait ce qui est déjà lisible), c'est celle de la
/// tuile qui parle alors. Une règle qui se déclencherait toujours serait aussi fausse qu'une règle
/// qui ne se déclencherait jamais.
///
/// Positions : les tuiles commencent à x = 47 et cadencent à 128 px (118 + gouttière), donc leurs
/// centres tombent à 106, 234, 362, 490, 618. La ligne du nom de la première rangée est à y ≈ 496.
fn survole_l_onglet_alertes(nom_capture: &str, x: f32, y: f32, couper: Option<&str>) {
    use overlay_ui::panels::alerts_tab::{AlertsAvailability, AlertsTabState};

    let mut profile = overlay_engine::AlertProfile::default();
    profile.add("Combinaison Lardante", Some(4242));
    if let Some(nom) = couper {
        assert!(
            profile.toggle(nom, None),
            "« {nom} » n'est pas dans le profil : sa tuile ne peut pas être coupée"
        );
    }

    let mut options_state = OptionsModalState {
        suivi: Default::default(),
        suivi_draft: None,
        suivi_availability: Default::default(),
        path_input: String::new(),
        error: None,
        tab: OptionsTab::Alertes,
        alerts: AlertsTabState {
            duration_input: "3,5".to_string(),
            ..Default::default()
        },
        alerts_draft: Some(profile),
        alerts_availability: AlertsAvailability::Ready,
        initial: Default::default(),
        pending_close: false,
        ..Default::default()
    };

    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            let catalog = CatalogIndex::default();
            panels::options_modal::show(
                ui,
                &mut options_state,
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();
    harness.hover_at(egui::pos2(x, y));
    harness.run();
    harness.snapshot(nom_capture);
}

/// **Une tuile RETIRABLE survolée montre son voile et sa croix** — et le voile s'arrête au contour
/// noir de l'emplacement, sans mordre sur la bordure de rareté (voir
/// `panels::alerts_tab::hover_scrim_rect`).
///
/// **Les ordonnées de ce fichier suivent la mise en page** : la grille est descendue de 48 px le
/// 2026-09-13, quand la phrase « Cliquez une tuile… » et la légende du pictogramme se sont posées
/// sous le titre « Objets suivis ».
///
/// Remplace, avec les deux tests suivants, les captures `..._infobulle_nom_elide` et
/// `..._infobulle_nom_entier` : le nom ne se peint plus sous la tuile depuis la refonte du
/// 2026-09-13, il n'y a donc plus d'élision à vérifier. « Combinaison Lardante » est le seul objet
/// ajouté du profil de ce test, donc le seul retirable — tuile 11, rangée 2, colonne 3.
#[test]
fn options_alertes_survol_d_une_tuile_retirable() {
    survole_l_onglet_alertes("options_alertes_survol_retirable", 231.0, 578.0, None);
}

/// **La croix sous le pointeur passe au rouge et dit ce qu'elle fait.**
///
/// Même tuile que ci-dessus, visée dans son coin haut-droit : le badge est à 8 px des deux bords,
/// son centre tombe donc à 15 px du coin.
#[test]
fn options_alertes_survol_de_la_croix() {
    survole_l_onglet_alertes("options_alertes_survol_croix", 248.0, 561.0, None);
}

/// **Un objet PAR DÉFAUT ne change pas d'aspect au survol** — ni voile, ni croix.
///
/// « Le voile ne concerne que les objets pouvant être supprimés » (retour du 2026-09-13) : les dix
/// objets par défaut n'ont pas de croix, un voile seul annoncerait une action qui n'existe pas.
/// Seule l'infobulle de nom apparaît.
#[test]
fn options_alertes_survol_d_un_objet_par_defaut() {
    survole_l_onglet_alertes("options_alertes_survol_par_defaut", 79.0, 502.0, None);
}

/// **Le champ d'ajout, exercé de bout en bout** — retour utilisateur du 2026-09-12 : « j'ai essayé
/// le champ d'auto-complétion mais celui-ci ne semblait pas fonctionner ».
///
/// Les captures précédentes vérifiaient l'AFFICHAGE de l'onglet, jamais son INTERACTION : un champ
/// peint au bon endroit peut très bien ne rien faire. Ce test clique dedans, tape trois caractères,
/// et vérifie que le panneau s'ouvre puis qu'une sélection ajoute réellement l'objet au brouillon.
///
/// Le catalogue est un **vrai `CatalogIndex`**, construit depuis le JSON compact que sert l'API —
/// pas un index vide comme dans les autres tests de ce fichier, puisque c'est précisément lui que
/// la recherche interroge.
#[test]
fn options_alertes_champ_d_ajout_trouve_et_ajoute() {
    use overlay_ui::panels::alerts_tab::{AlertsAvailability, AlertsTabState};

    // `[id, fr, en, es, pt, gfxId, rarity_sort, has_recipe, category_sort]`
    let catalog = CatalogIndex::from_compact_json(&serde_json::json!({
        "items": [
            [101, "Pierre de lune", "Moonstone", "Piedra", "Pedra", 1101, 2, 0, 1],
            [102, "Pierre de dolomite", "Dolomite", "Dolomita", "Dolomita", 1102, 1, 0, 1],
            [103, "Coiffe du Bouftou", "Gobball Headgear", "Casco", "Elmo", 1103, 3, 1, 0],
        ],
    }));
    assert_eq!(
        catalog.search_items("pierre", 3, 40).len(),
        2,
        "le catalogue de test ne répond pas — le reste du test ne prouverait rien"
    );

    let profil = overlay_engine::AlertProfile::default();
    let avant = profil.sound_items.len();
    let etat = std::rc::Rc::new(std::cell::RefCell::new(OptionsModalState {
        suivi: Default::default(),
        suivi_draft: None,
        suivi_availability: Default::default(),
        path_input: String::new(),
        error: None,
        tab: OptionsTab::Alertes,
        alerts: AlertsTabState::default(),
        alerts_draft: Some(profil),
        alerts_availability: AlertsAvailability::Ready,
        initial: Default::default(),
        pending_close: false,
        ..Default::default()
    }));

    let vu = std::rc::Rc::clone(&etat);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            panels::options_modal::show(
                ui,
                &mut vu.borrow_mut(),
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();

    // **Un clic RÉEL dans le champ**, pas un `request_focus` posé par le test : c'est le geste que
    // l'utilisateur fait, et c'est lui qui doit donner le focus. Le champ d'ajout est sous le titre
    // « Objets suivis », pleine largeur du panneau.
    let champ = egui::pos2(300.0, 441.0);
    harness.drag_at(champ);
    harness.run();
    harness.drop_at(champ);
    harness.run();

    // **Une frappe RÉELLE**, caractère par caractère, comme au clavier.
    for c in "pierre".chars() {
        harness.event(egui::Event::Text(c.to_string()));
    }
    harness.run();
    assert_eq!(
        etat.borrow().alerts.search,
        "pierre",
        "le champ n'a pas reçu la frappe — il n'avait donc pas le focus après le clic"
    );
    harness.snapshot("options_alertes_champ_deplie");

    // **L'infobulle d'un bouton de filtre, au-dessus du panneau flottant.** Un passage précédent
    // avait laissé `on_hover_text` ici, en concluant d'une planche muette que le composant du
    // design system ne savait pas sortir d'une couche déjà en avant-plan. C'était faux : les deux
    // passent par le même `egui::Tooltip::for_enabled` (voir `Response::on_hover_ui` dans egui),
    // seul l'alignement diffère — et c'est lui qui envoyait le texte hors du cadre capturé. Cette
    // capture le prouve dans les deux sens : l'infobulle sort, et elle sort AU-DESSUS.
    harness.hover_at(egui::pos2(68.0, 479.0));
    harness.run();
    harness.snapshot("options_alertes_infobulle_filtre");

    // Première suggestion : « Pierre de dolomite » (tri alphabétique sur le nom normalisé). Le
    // panneau ouvre par sa BANDE DE FILTRES : la première rangée tombe en dessous, pas
    // immédiatement sous le champ.
    let suggestion = egui::pos2(300.0, 510.0);
    // **Survol d'abord, clic ensuite** : egui rattache un appui au widget que le pointeur
    // survolait, et le pointeur n'est nulle part tant qu'aucun mouvement ne l'a placé. Sans cette
    // frame de survol, l'appui tombe sur un widget inconnu et la rangée n'est jamais cliquée.
    harness.hover_at(suggestion);
    harness.run();

    // **Appui et relâchement dans DEUX frames distinctes** — c'est le geste réel, et c'est lui qui
    // a révélé le défaut du 2026-09-12 : l'appui retire le focus au champ, et une condition
    // d'ouverture réduite à `has_focus()` fermait le panneau avant la frame du relâchement, seule
    // où egui rend `clicked()` vrai. Les garder dans la même frame masquerait la régression.
    harness.drag_at(suggestion);
    harness.run();
    harness.drop_at(suggestion);
    harness.run();
    let apres = etat
        .borrow()
        .alerts_draft
        .as_ref()
        .unwrap()
        .sound_items
        .len();
    assert_eq!(
        apres,
        avant + 1,
        "la sélection n'a rien ajouté au brouillon"
    );
    // Le champ vidé, le panneau refermé, et la tuile de plus dans la grille — les trois effets
    // visibles d'une sélection, sur une seule image.
    harness.snapshot("options_alertes_objet_ajoute");
}

/// La croix à droite du champ d'ajout vide la saisie d'un clic, et rend le focus au champ.
///
/// Le geste est le même que pour une suggestion : survol, appui, relâchement, dans trois frames.
/// L'appui retire le focus au champ (le pointeur est sur la croix, pas sur la zone d'édition) ;
/// c'est au composant de le lui rendre, sinon l'utilisateur efface pour retaper… dans le vide.
#[test]
fn options_alertes_croix_efface_la_saisie() {
    use overlay_ui::panels::alerts_tab::{AlertsAvailability, AlertsTabState};

    let catalog = CatalogIndex::from_compact_json(&serde_json::json!({
        "items": [
            [101, "Pierre de lune", "Moonstone", "Piedra", "Pedra", 1101, 2, 0, 1],
        ],
    }));
    let etat = std::rc::Rc::new(std::cell::RefCell::new(OptionsModalState {
        suivi: Default::default(),
        suivi_draft: None,
        suivi_availability: Default::default(),
        path_input: String::new(),
        error: None,
        tab: OptionsTab::Alertes,
        alerts: AlertsTabState::default(),
        alerts_draft: Some(overlay_engine::AlertProfile::default()),
        alerts_availability: AlertsAvailability::Ready,
        initial: Default::default(),
        pending_close: false,
        ..Default::default()
    }));

    let vu = std::rc::Rc::clone(&etat);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            panels::options_modal::show(
                ui,
                &mut vu.borrow_mut(),
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();

    let champ = egui::pos2(300.0, 443.0);
    harness.drag_at(champ);
    harness.run();
    harness.drop_at(champ);
    harness.run();
    for c in "pierre".chars() {
        harness.event(egui::Event::Text(c.to_string()));
    }
    harness.run();
    assert_eq!(etat.borrow().alerts.search, "pierre");

    // La croix : à 9 px du bord extérieur droit du champ, sur son axe — voir
    // `tokens::INPUT_CLEAR_INSET_RATIO`. Le champ s'arrête à x≈705 dans cette fenêtre.
    let croix = egui::pos2(691.0, 443.0);
    harness.hover_at(croix);
    harness.run();
    harness.drag_at(croix);
    harness.run();
    harness.drop_at(croix);
    harness.run();
    assert!(
        etat.borrow().alerts.search.is_empty(),
        "la croix n'a pas vidé la saisie : « {} »",
        etat.borrow().alerts.search
    );

    // Le focus est revenu au champ : une frappe repart dedans sans nouveau clic.
    harness.event(egui::Event::Text("l".to_string()));
    harness.run();
    assert_eq!(
        etat.borrow().alerts.search,
        "l",
        "le champ n'a pas repris le focus après l'effacement"
    );
}

/// ↓ six fois sur huit suggestions : l'entrée active sort de la fenêtre des cinq rangées visibles,
/// et la liste **doit défiler** pour la garder en vue. Retour du 2026-09-12 au soir : « le scroll
/// ne se synchronise pas avec les touches haut et bas — l'utilisateur ne voit pas sur quoi il va
/// appuyer sur Entrée ». La capture montre la septième rangée en surbrillance, et Entrée l'ajoute.
#[test]
fn options_alertes_les_fleches_font_defiler_la_liste() {
    use overlay_ui::panels::alerts_tab::{AlertsAvailability, AlertsTabState};

    // Huit « Pierre … », toutes retenues par « pierre » : trois de plus que les rangées visibles.
    let items: Vec<serde_json::Value> = (1..=8)
        .map(|i| {
            serde_json::json!([
                100 + i,
                format!("Pierre n° {i}"),
                format!("Stone #{i}"),
                format!("Piedra {i}"),
                format!("Pedra {i}"),
                1100 + i,
                1,
                0,
                1
            ])
        })
        .collect();
    let catalog = CatalogIndex::from_compact_json(&serde_json::json!({ "items": items }));
    assert_eq!(catalog.search_items("pierre", 3, 40).len(), 8);

    let etat = std::rc::Rc::new(std::cell::RefCell::new(OptionsModalState {
        suivi: Default::default(),
        suivi_draft: None,
        suivi_availability: Default::default(),
        path_input: String::new(),
        error: None,
        tab: OptionsTab::Alertes,
        alerts: AlertsTabState::default(),
        alerts_draft: Some(overlay_engine::AlertProfile::default()),
        alerts_availability: AlertsAvailability::Ready,
        initial: Default::default(),
        pending_close: false,
        ..Default::default()
    }));
    let vu = std::rc::Rc::clone(&etat);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            panels::options_modal::show(
                ui,
                &mut vu.borrow_mut(),
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();

    let champ = egui::pos2(300.0, 441.0);
    harness.drag_at(champ);
    harness.run();
    harness.drop_at(champ);
    harness.run();
    for c in "pierre".chars() {
        harness.event(egui::Event::Text(c.to_string()));
    }
    harness.run();
    assert_eq!(etat.borrow().alerts.search, "pierre");

    // Une flèche par frame, comme au clavier : chacune est consommée par le panneau avant que
    // le champ ne la voie.
    for _ in 0..6 {
        harness.key_press(egui::Key::ArrowDown);
        harness.run();
    }
    harness.snapshot("options_alertes_defilement_clavier");

    // Le pointeur SUR la poignée (colonne de droite du panneau, à mi-hauteur de la liste) : elle
    // prend la teinte des rangées survolées, sans s'élargir. egui ne passe en `hovered` que le
    // pointeur sur la poignée elle-même, pas seulement dans sa colonne.
    harness.hover_at(egui::pos2(700.0, 608.0));
    harness.run();
    harness.snapshot("options_alertes_poignee_survolee");

    harness.key_press(egui::Key::Enter);
    harness.run();
    let ajoute = etat
        .borrow()
        .alerts_draft
        .as_ref()
        .unwrap()
        .sound_items
        .last()
        .map(|entry| entry.name.clone());
    assert_eq!(
        ajoute.as_deref(),
        Some("Pierre n° 7"),
        "Entrée n'a pas validé l'entrée active — sept rangées plus bas que la première"
    );
}

/// Rend l'onglet « Suivi » de la fenêtre Options dans un état donné, et le capture.
///
/// **Les entrées viennent du moteur**, pas d'une liste écrite à la main : ce sont de vraies
/// [`overlay_engine::WatchlistEntry`], celles que `GET /api/v1/settings` renvoie. Les icônes, elles,
/// restent le repli générique — elles viennent du CDN, hors de portée d'un test sans réseau.
/// Le jeu d'entrées de toutes les planches et de tous les tests de l'onglet Suivi — **objets et
/// monstres mêlés, les deux modes côte à côte** : la grille doit rendre lisible d'un coup d'œil ce
/// qui porte une cible et ce qui n'en porte pas.
fn entrees_de_suivi() -> Vec<overlay_engine::WatchlistEntry> {
    use overlay_engine::{WatchlistEntry, WatchlistKind, WatchlistMode};

    fn entree(name: &str, kind: WatchlistKind, mode: WatchlistMode, target: i64) -> WatchlistEntry {
        WatchlistEntry {
            name: name.to_string(),
            kind,
            mode,
            count: target,
            countdown_target: target,
            catalog_id: None,
        }
    }

    vec![
        entree(
            "Bois de Frêne",
            WatchlistKind::Item,
            WatchlistMode::Down,
            1000,
        ),
        entree("Fleur de Kalé", WatchlistKind::Item, WatchlistMode::Up, 0),
        entree(
            "Pierre de Lune",
            WatchlistKind::Item,
            WatchlistMode::Down,
            50,
        ),
        entree("Bouftou", WatchlistKind::Enemy, WatchlistMode::Up, 0),
        entree("Larve Bleue", WatchlistKind::Enemy, WatchlistMode::Up, 0),
        entree(
            "Plume de Tofu",
            WatchlistKind::Item,
            WatchlistMode::Down,
            100,
        ),
        entree("Chafer Élite", WatchlistKind::Enemy, WatchlistMode::Up, 0),
        entree("Minerai de Fer", WatchlistKind::Item, WatchlistMode::Up, 0),
    ]
}

fn capture_onglet_suivi(
    nom: &str,
    mode: overlay_ui::panels::suivi_tab::AddMode,
    select_mode: bool,
    alt: bool,
    survol: Option<egui::Pos2>,
) {
    use overlay_ui::panels::suivi_tab::{SuiviAvailability, SuiviTabState};

    let entries = entrees_de_suivi();

    let mut options_state = OptionsModalState {
        suivi: SuiviTabState {
            mode,
            target: 250,
            select_mode,
            // Trois tuiles cochées quand la sélection est ouverte : c'est ce qui fait basculer le
            // libellé du bouton de « Supprimer tout » à « Supprimer (3) ».
            selected: if select_mode {
                entries
                    .iter()
                    .take(3)
                    .map(|e| format!("{}::", e.name))
                    .collect()
            } else {
                Vec::new()
            },
            ..Default::default()
        },
        suivi_draft: Some(entries),
        suivi_availability: SuiviAvailability::Ready,
        path_input: String::new(),
        error: None,
        tab: OptionsTab::Suivi,
        alerts: Default::default(),
        alerts_draft: None,
        alerts_availability: Default::default(),
        initial: Default::default(),
        pending_close: false,
        ..Default::default()
    };

    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            let catalog = CatalogIndex::default();
            panels::options_modal::show(
                ui,
                &mut options_state,
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();
    if alt {
        // **`ModifiersChanged`, et rien d'autre.** C'est le seul événement dont egui tire
        // `InputState::modifiers`, celui que la ligne « Quantité » lit pour inverser ses badges.
        // Un `Event::Key { modifiers: ALT }` ne porte le modificateur que pour CETTE touche : il ne
        // change pas l'état global, et une planche qui l'employait sortait identique à celle sans
        // `Alt` (relevé par l'utilisateur le 2026-09-13). Cette capture est là pour que l'écart
        // entre les deux états ne puisse plus disparaître en silence.
        harness.event(egui::Event::ModifiersChanged(egui::Modifiers::ALT));
        harness.run();
    }
    if let Some(point) = survol {
        harness.hover_at(point);
        harness.run();
    }
    harness.snapshot(nom);
}

/// **L'onglet « Suivi »** — la liste réduite à ses emplacements d'objet, et son formulaire d'ajout.
///
/// Ce que cette capture verrouille, et qui est la règle la plus facile à perdre : **les compteurs
/// ne s'affichent pas**. Seule la cible d'un décompte est peinte (`/50`), un incrémental ne porte
/// aucun chiffre — cet écran sert à composer une liste, pas à la lire.
#[test]
fn options_onglet_suivi_incremental() {
    capture_onglet_suivi(
        "options_suivi_incremental",
        overlay_ui::panels::suivi_tab::AddMode::Up,
        false,
        false,
        None,
    );
}

/// Mode décompte : la ligne « Quantité » apparaît, avec ses badges, sa mention « (Alt : retirer) »
/// et son pas numérique.
#[test]
fn options_onglet_suivi_decompte() {
    capture_onglet_suivi(
        "options_suivi_decompte",
        overlay_ui::panels::suivi_tab::AddMode::Down,
        false,
        false,
        None,
    );
}

/// Sélection multiple : cases à cocher sur toutes les tuiles, **liseré et coche rouges** sur les
/// cochées, et le bouton de suppression groupée en rouge — décision explicite de l'utilisateur
/// (2026-09-13). Le ton destructif est le même que sur le bandeau : cocher ici ne mène nulle part
/// ailleurs qu'au retrait, l'or promettait un choix qui n'existe pas.
#[test]
fn options_onglet_suivi_selection_multiple() {
    capture_onglet_suivi(
        "options_suivi_selection",
        overlay_ui::panels::suivi_tab::AddMode::Up,
        true,
        false,
        None,
    );
}

/// **`Alt` maintenu : les cinq badges de quantité retirent au lieu d'ajouter.**
///
/// Ils passent en or et affichent `−10 … −1000`, et la mention « (Alt : retirer) » s'allume avec
/// eux. C'est la seule chose de cet écran qui dépende d'un modificateur clavier, et elle était
/// invisible sur les planches jusqu'au 2026-09-13 — non parce que le code était faux, mais parce
/// que la planche n'enfonçait pas la touche par le bon événement. Cette capture rend l'écart
/// vérifiable à chaque exécution.
#[test]
fn options_onglet_suivi_decompte_alt() {
    capture_onglet_suivi(
        "options_suivi_decompte_alt",
        overlay_ui::panels::suivi_tab::AddMode::Down,
        false,
        true,
        None,
    );
}

/// **L'infobulle d'un bouton de la modale se pose AU-DESSUS de lui** — retour utilisateur du
/// 2026-09-13, capture à l'appui : celle d'« Incrémental » sortait *sous* le bouton.
///
/// La cause n'était pas dans ce panneau mais dans `design::button`, qui posait son infobulle avec
/// `Response::on_hover_text` : cet appel aligne en `RectAlign::BOTTOM_START` (voir `Popup::new`
/// dans egui) et laisse au libellé le gris de `Visuals::widgets.noninteractive`. Sept composants
/// du design system faisaient pareil ; les panneaux Combat et Suivi, eux, appelaient déjà
/// `design::tooltip` — d'où deux rendus d'infobulle dans la même application, exactement ce que
/// l'utilisateur décrivait (« le texte n'est pas en blanc et le fond est un peu trop translucide »).
///
/// Cette capture est le garde-fou de la règle : **au-dessus, et en blanc**, dans la modale comme
/// ailleurs.
#[test]
fn options_suivi_infobulle_de_bouton_au_dessus() {
    capture_onglet_suivi(
        "options_suivi_infobulle_mode",
        overlay_ui::panels::suivi_tab::AddMode::Down,
        false,
        false,
        Some(egui::pos2(252.0, 252.0)),
    );
}

/// **Un badge de quantité dit ce qu'il ajoute, pas comment faire l'inverse.**
///
/// Son infobulle portait « Ajouter 50 à la quantité — Alt pour retirer » ; la moitié après le tiret
/// est partie le 2026-09-13 sur retour utilisateur. Elle ne manque à personne : la mention
/// « (Alt : retirer) » est écrite en toutes lettres au début de la même ligne, et les cinq badges
/// basculent visiblement en « −10 … −1000 » dès qu'`Alt` est enfoncé (voir
/// [`options_onglet_suivi_decompte_alt`]).
#[test]
fn options_suivi_infobulle_de_badge_sans_mention_alt() {
    capture_onglet_suivi(
        "options_suivi_infobulle_badge",
        overlay_ui::panels::suivi_tab::AddMode::Down,
        false,
        false,
        Some(egui::pos2(240.0, 298.0)),
    );
}

/// **Le geste de réordonnancement, en vol** — la planche qui verrouille ce que l'utilisateur voit
/// pendant qu'il déplace une tuile : la place d'origine voilée, le fantôme sous le pointeur, et la
/// barre d'insertion or sur la tuile visée.
///
/// Rien de tout cela n'est fourni par la plateforme, contrairement au web où le navigateur peint
/// lui-même un fantôme natif : sans ces trois marques, le geste serait invisible. La capture est le
/// seul garde-fou possible — un test d'ordre (voir
/// [`options_suivi_le_glisser_deposer_reordonne_comme_le_web`]) prouve le résultat, pas le retour
/// visuel qui le rend praticable.
#[test]
fn options_suivi_deplacement_en_vol() {
    capture_suivi_deplacement("options_suivi_deplacement");
}

fn capture_suivi_deplacement(nom: &str) {
    let (mut harness, _etat) = harnais_suivi(overlay_ui::panels::suivi_tab::AddMode::Up);
    harness.run();

    // Première tuile prise près de son coin haut-gauche — pas en son centre : le fantôme se tient
    // par où on l'a prise, et c'est ce décalage qui laisse voir la tuile visée dessous. Une planche
    // qui saisirait au centre montrerait un fantôme parfaitement superposé à la destination, donc
    // ne montrerait justement pas ce que le geste doit rendre lisible.
    harness.drag_at(TUILE_0_PRISE);
    harness.run();
    harness.hover_at(TUILE_2);
    harness.run();
    // Une seconde frame : le fantôme et la barre suivent le pointeur de la frame précédente.
    harness.run();
    harness.snapshot(nom);
}

/// **Glisser-déposer : l'ordre obtenu est celui du web.**
///
/// `reorder` est déjà couvert unitairement (`panels::suivi_tab`) ; ce test-ci prouve l'autre
/// moitié, celle qu'une fonction pure ne peut pas montrer : que le geste de la souris, sur la vraie
/// grille et à travers la vraie modale, arrive bien jusqu'à elle avec les bons rangs.
#[test]
fn options_suivi_le_glisser_deposer_reordonne_comme_le_web() {
    let (mut harness, etat) = harnais_suivi(overlay_ui::panels::suivi_tab::AddMode::Up);
    harness.run();

    let avant: Vec<String> = etat
        .borrow()
        .suivi_draft
        .clone()
        .unwrap_or_default()
        .iter()
        .map(|e| e.name.clone())
        .collect();

    // **La croix fléchée dit que la tuile se déplace, avant même qu'on l'ait prise.** C'est le seul
    // signe de l'écran — aucune poignée, aucun libellé — et `overlay_ui::cursor` la traduit ensuite
    // en bitmap du jeu (`wakfu-cursor-move.png`).
    harness.hover_at(TUILE_0);
    harness.run();
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        egui::CursorIcon::Move,
        "une tuile survolée doit annoncer qu'elle se déplace"
    );

    harness.drag_at(TUILE_0);
    harness.run();
    harness.hover_at(TUILE_2);
    harness.run();
    // La croix tient pendant tout le geste, y compris au-dessus d'une AUTRE tuile : ce qui est sous
    // le pointeur est une destination, pas un bouton.
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        egui::CursorIcon::Move,
        "le curseur doit rester la croix pendant le déplacement"
    );
    harness.drop_at(TUILE_2);
    harness.run();

    let apres: Vec<String> = etat
        .borrow()
        .suivi_draft
        .clone()
        .unwrap_or_default()
        .iter()
        .map(|e| e.name.clone())
        .collect();

    // Retrait puis réinsertion au rang visé, comme `StatsStoreService.reorderWatchlist` : la
    // première entrée se pose APRÈS celle qu'elle visait, les deux autres remontent d'un rang.
    assert_eq!(
        apres,
        vec![
            avant[1].clone(),
            avant[2].clone(),
            avant[0].clone(),
            avant[3].clone(),
            avant[4].clone(),
            avant[5].clone(),
            avant[6].clone(),
            avant[7].clone(),
        ],
        "le glisser-déposer n'a pas réordonné le brouillon comme le web"
    );
}

/// Centres des trois premières tuiles de la grille, en mode incrémental (le formulaire n'a alors
/// qu'une ligne). Relevés sur la planche `options_suivi_incremental` : tuile de 64 px, gouttière de
/// 12, donc un pas de 76 px.
const TUILE_0: egui::Pos2 = egui::pos2(79.0, 394.0);
const TUILE_2: egui::Pos2 = egui::pos2(231.0, 394.0);
/// Un point de prise excentré dans la première tuile — voir [`capture_suivi_deplacement`].
const TUILE_0_PRISE: egui::Pos2 = egui::pos2(62.0, 377.0);

/// Le harnais de l'onglet Suivi, avec son état rendu inspectable — voir [`capture_onglet_suivi`],
/// dont c'est le même jeu d'entrées. Rendu partagé parce que deux tests ont maintenant besoin de
/// LIRE le brouillon après coup, pas seulement de le peindre.
fn harnais_suivi(
    mode: overlay_ui::panels::suivi_tab::AddMode,
) -> (
    Harness<'static>,
    std::rc::Rc<std::cell::RefCell<OptionsModalState>>,
) {
    use overlay_ui::panels::suivi_tab::{SuiviAvailability, SuiviTabState};

    let etat = std::rc::Rc::new(std::cell::RefCell::new(OptionsModalState {
        suivi: SuiviTabState {
            mode,
            target: 250,
            ..Default::default()
        },
        suivi_draft: Some(entrees_de_suivi()),
        suivi_availability: SuiviAvailability::Ready,
        path_input: String::new(),
        error: None,
        tab: OptionsTab::Suivi,
        alerts: Default::default(),
        alerts_draft: None,
        alerts_availability: Default::default(),
        initial: Default::default(),
        pending_close: false,
        ..Default::default()
    }));

    let vu = std::rc::Rc::clone(&etat);
    let harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            let catalog = CatalogIndex::default();
            panels::options_modal::show(
                ui,
                &mut vu.borrow_mut(),
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    (harness, etat)
}

/// **Une tuile suivie dit son nom, et rien d'autre.**
///
/// Elle disait « Bois de Frêne — décompte depuis 1000 ». Le mode est parti le 2026-09-13 : il se
/// lit déjà sur la tuile (la cible y est peinte, `/1000`), et l'utilisateur vient de le choisir
/// dans cette même modale. Le nom, lui, n'est écrit nulle part ailleurs sur un emplacement de
/// 64 px — c'est la seule chose que l'infobulle apporte.
#[test]
fn options_suivi_infobulle_de_tuile_sans_mode() {
    capture_onglet_suivi(
        "options_suivi_infobulle_tuile",
        overlay_ui::panels::suivi_tab::AddMode::Down,
        false,
        false,
        Some(egui::pos2(79.0, 440.0)),
    );
}

/// **Une tuile au son coupé : le haut-parleur barré dans son coin, et le nom en infobulle.**
///
/// L'infobulle disait « Cliquer pour rétablir » ; elle ne dit plus que le nom depuis le
/// 2026-09-13, comme au Suivi — le pictogramme porte déjà l'état, et le nom n'est plus écrit nulle
/// part ailleurs. « Influence III » est le sixième objet par défaut : rangée 1, colonne 6.
#[test]
fn options_alertes_infobulle_d_une_tuile_coupee() {
    survole_l_onglet_alertes(
        "options_alertes_infobulle_tuile_coupee",
        459.0,
        502.0,
        Some("Influence III"),
    );
}

/// **Le bouton de test du son, dans l'onglet Alertes** — même bascule que les boutons du Suivi, sur
/// un `design::icon_button` cette fois : son infobulle sortait sous le glyphe.
#[test]
fn options_alertes_infobulle_du_test_de_son_au_dessus() {
    survole_l_onglet_alertes("options_alertes_infobulle_test_son", 240.0, 252.0, None);
}

/// **Le champ d'ajout du Suivi, exercé de bout en bout — objets ET monstres.**
///
/// C'est la différence de fond avec celui des alertes, qui ne cherche que des objets : ici le
/// référentiel de monstres est interrogé en même temps, et une suggestion de monstre doit créer une
/// entrée de genre `Enemy`. Un test qui n'ajouterait qu'un objet ne prouverait rien de la moitié
/// nouvelle.
#[test]
fn options_suivi_champ_d_ajout_trouve_objets_et_monstres() {
    use overlay_engine::WatchlistKind;
    use overlay_ui::panels::suivi_tab::{SuiviAvailability, SuiviTabState};

    // `[id, fr, en, es, pt, gfxId, rarity_sort, has_recipe, category_sort]` pour les objets,
    // `[id, fr, en, es, pt, gfxId, family, isBoss, isArchi, isDominant]` pour les monstres.
    let catalog = CatalogIndex::from_compact_json(&serde_json::json!({
        "items": [
            [201, "Plume de Tofu", "Tofu Feather", "Pluma", "Pena", 1201, 1, 0, 1],
            [202, "Coiffe du Tofu", "Tofu Headgear", "Casco", "Elmo", 1202, 2, 1, 0],
        ],
        "monsters": [
            [301, "Tofu", "Tofu", "Tofu", "Tofu", "2301", -1, 0, 0, 0],
            [302, "Tofu Royal", "Royal Tofu", "Tofu Real", "Tofu Real", "2302", -1, 1, 0, 0],
        ],
    }));
    assert_eq!(
        catalog.search_items("tofu", 3, 40).len(),
        2,
        "le catalogue d'objets de test ne répond pas"
    );
    assert_eq!(
        catalog.search_monsters("tofu", 3, 40).len(),
        2,
        "le référentiel de monstres de test ne répond pas — c'est précisément ce que ce test vérifie"
    );

    let etat = std::rc::Rc::new(std::cell::RefCell::new(OptionsModalState {
        suivi: SuiviTabState::default(),
        suivi_draft: Some(Vec::new()),
        suivi_availability: SuiviAvailability::Ready,
        path_input: String::new(),
        error: None,
        tab: OptionsTab::Suivi,
        alerts: Default::default(),
        alerts_draft: None,
        alerts_availability: Default::default(),
        initial: Default::default(),
        pending_close: false,
        ..Default::default()
    }));

    let vu = std::rc::Rc::clone(&etat);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            panels::options_modal::show(
                ui,
                &mut vu.borrow_mut(),
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();

    // Le champ est sous le bloc de formulaire, en mode incrémental (une seule ligne).
    let champ = egui::pos2(300.0, 300.0);
    harness.drag_at(champ);
    harness.run();
    harness.drop_at(champ);
    harness.run();
    for c in "tofu".chars() {
        harness.event(egui::Event::Text(c.to_string()));
    }
    harness.run();
    assert_eq!(
        etat.borrow().suivi.search,
        "tofu",
        "le champ n'a pas reçu la frappe — il n'avait donc pas le focus après le clic"
    );

    // Les objets d'abord, les monstres ensuite : la troisième rangée est « Tofu », un MONSTRE.
    // Le panneau ouvre par sa bande de filtres, puis les rangées de 35 px.
    let rangee = egui::pos2(300.0, 300.0 + 25.0 + 38.0 + 35.0 * 2.0 + 17.0);
    harness.hover_at(rangee);
    harness.run();
    harness.drag_at(rangee);
    harness.run();
    harness.drop_at(rangee);
    harness.run();

    let ajoutees = etat.borrow().suivi_draft.clone().unwrap_or_default();
    assert_eq!(
        ajoutees.len(),
        1,
        "la sélection n'a rien ajouté au brouillon"
    );
    assert_eq!(
        ajoutees[0].kind,
        WatchlistKind::Enemy,
        "un monstre choisi doit créer une entrée de genre Enemy, pas Item"
    );
    assert_eq!(ajoutees[0].catalog_id, Some(301));
}

/// **Le mode choisi avant le nom décide de ce que l'entrée devient** — règle 2 de l'onglet.
///
/// En décompte, l'entrée créée part de la quantité du formulaire et non de zéro : c'est tout
/// l'intérêt du mode, et rien dans la grille ne le rattraperait après coup (la cible est figée à la
/// création, comme sur le web).
#[test]
fn options_suivi_le_mode_du_formulaire_decide_de_l_entree() {
    use overlay_engine::WatchlistMode;
    use overlay_ui::panels::suivi_tab::{AddMode, SuiviAvailability, SuiviTabState};

    let catalog = CatalogIndex::from_compact_json(&serde_json::json!({
        "items": [[201, "Plume de Tofu", "Tofu Feather", "Pluma", "Pena", 1201, 1, 0, 1]],
        "monsters": [],
    }));

    let etat = std::rc::Rc::new(std::cell::RefCell::new(OptionsModalState {
        suivi: SuiviTabState {
            mode: AddMode::Down,
            target: 250,
            ..Default::default()
        },
        suivi_draft: Some(Vec::new()),
        suivi_availability: SuiviAvailability::Ready,
        path_input: String::new(),
        error: None,
        tab: OptionsTab::Suivi,
        alerts: Default::default(),
        alerts_draft: None,
        alerts_availability: Default::default(),
        initial: Default::default(),
        pending_close: false,
        ..Default::default()
    }));

    let vu = std::rc::Rc::clone(&etat);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            panels::options_modal::show(
                ui,
                &mut vu.borrow_mut(),
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();

    // Le bloc porte DEUX lignes en décompte : le champ descend d'autant.
    let champ = egui::pos2(300.0, 346.0);
    harness.drag_at(champ);
    harness.run();
    harness.drop_at(champ);
    harness.run();
    for c in "tofu".chars() {
        harness.event(egui::Event::Text(c.to_string()));
    }
    harness.run();

    let rangee = egui::pos2(300.0, 346.0 + 25.0 + 38.0 + 17.0);
    harness.hover_at(rangee);
    harness.run();
    harness.drag_at(rangee);
    harness.run();
    harness.drop_at(rangee);
    harness.run();

    let ajoutees = etat.borrow().suivi_draft.clone().unwrap_or_default();
    assert_eq!(
        ajoutees.len(),
        1,
        "la sélection n'a rien ajouté au brouillon"
    );
    assert_eq!(ajoutees[0].mode, WatchlistMode::Down);
    assert_eq!(
        ajoutees[0].countdown_target, 250,
        "la cible du formulaire doit suivre l'entrée créée"
    );
    // Le formulaire revient à son défaut après un ajout, comme `resetAddForm` côté web.
    assert_eq!(etat.borrow().suivi.target, 1);
}

/// Curseur du jeu à la place du curseur système (voir `overlay_ui::cursor`) : ce que
/// `paint_content` publie dans `PlatformOutput::cursor_image`, là où les deux binaires le lisent
/// (via `egui-winit`), sans fenêtre ni GPU. Mêmes coordonnées que
/// `panneau_combat_tooltip_switch_allies_ennemis_au_dessus` (centre du bouton « Alliés », un
/// `on_hover_cursor(PointingHand)`), et un point du vide du panneau pour la flèche.
#[test]
fn le_curseur_du_jeu_remplace_le_curseur_systeme_et_clignote_sur_le_cliquable() {
    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    // Raccourcis PAR DÉFAUT (voir `overlay_ui::shortcuts`) : les infobulles affichent donc les
    // mêmes combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
    let shortcuts = ShortcutBindings::default();
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
                options: None,
            },
        );
    });
    let images = overlay_ui::cursor::images();
    let published = |harness: &Harness<'_>| harness.output().platform_output.cursor_image.clone();
    let same = |a: &Option<egui::CustomCursorImage>, b: &egui::CustomCursorImage| {
        a.as_ref()
            .is_some_and(|a| std::sync::Arc::ptr_eq(&a.rgba, &b.rgba))
    };

    // Pointeur dans le vide du panneau : flèche du jeu au repos, aucun redessin réclamé.
    harness.hover_at(egui::pos2(400.0, 400.0));
    harness.run();
    assert!(
        same(&published(&harness), &images.idle),
        "vide : attendu le bitmap de repos"
    );
    assert!(
        harness.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay
            > std::time::Duration::from_secs(60)
    );

    // Bouton « Alliés » (main) : l'éclair d'abord, et un redessin réclamé AU PLUS TARD pour la
    // prochaine bascule (l'infobulle du switch en réclame un plus tôt encore, d'où `<=` et non
    // `==`) — `now` est figé dans ce harnais, la phase ne progresse donc pas d'une frame à l'autre.
    harness.hover_at(egui::pos2(35.0, 71.0));
    harness.run();
    assert!(
        same(&published(&harness), &images.flash),
        "cliquable : attendu l'éclair"
    );
    let delay = harness.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay;
    assert!(
        delay > std::time::Duration::ZERO && delay <= overlay_ui::cursor::FLASH_DURATION,
        "délai de redessin inattendu en mode main : {delay:?}"
    );

    // Retour dans le vide : repos à nouveau.
    harness.hover_at(egui::pos2(400.0, 400.0));
    harness.run();
    assert!(same(&published(&harness), &images.idle));
}

// -------------------------------------------------------------------------------------------
// Onglet « Chat » (2026-09-14) — les recherches qui font sonner l'overlay, et la carte de chat.
// -------------------------------------------------------------------------------------------

/// Le brouillon des captures : neuf recherches — assez pour trois rangées de tuiles, une rangée
/// incomplète, et deux recherches globales.
fn recherches_de_chat() -> overlay_ui::panels::chat_tab::ChatDraft {
    use overlay_engine::{ChatChannel, ChatFilter, ChatFilterScope};
    let canal = |c: ChatChannel, mot: &str| {
        ChatFilter::new(ChatFilterScope::Channel(c), mot).expect("mot non vide")
    };
    let tous = |mot: &str| ChatFilter::new(ChatFilterScope::All, mot).expect("mot non vide");
    overlay_ui::panels::chat_tab::ChatDraft {
        filters: vec![
            canal(ChatChannel::Commerce, "gelano"),
            tous("donjon"),
            canal(ChatChannel::Recrutement, "pvm"),
            canal(ChatChannel::Guilde, "wa wabbit"),
            canal(ChatChannel::Commerce, "bois de bouleau"),
            canal(ChatChannel::Proximite, "archi"),
            canal(ChatChannel::Communaute, "mise à jour"),
            canal(ChatChannel::Groupe, "pierre de kamas"),
            tous("kralamoure"),
        ],
        toast: Default::default(),
    }
}

/// Même gabarit que `capture_onglet_alertes` : la fenêtre à sa taille réelle, l'onglet « Chat »
/// actif, un survol optionnel (la croix d'une tuile).
fn capture_onglet_chat(
    nom: &str,
    draft: Option<overlay_ui::panels::chat_tab::ChatDraft>,
    availability: overlay_ui::panels::chat_tab::ChatAvailability,
    survol: Option<egui::Pos2>,
) {
    use overlay_ui::panels::chat_tab::ChatTabState;

    let mut options_state = OptionsModalState {
        tab: OptionsTab::Chat,
        chat: ChatTabState {
            duration_input: "3,5".to_string(),
            ..Default::default()
        },
        chat_draft: draft,
        chat_availability: availability,
        ..Default::default()
    };

    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            panels::options_modal::WINDOW_SIZE.0,
            panels::options_modal::WINDOW_SIZE.1,
        ))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = UiIcons::load(ui.ctx());
            let remote_icons = RemoteIconStore::empty();
            let mut remote_icon_textures = RemoteIconTextures::default();
            let catalog = CatalogIndex::default();
            panels::options_modal::show(
                ui,
                &mut options_state,
                &mut panels::options_modal::OptionsModalContext {
                    catalog: &catalog,
                    remote_icons: &remote_icons,
                    remote_icon_textures: &mut remote_icon_textures,
                    icons: &icons,
                },
            );
        });
    harness.run();
    if let Some(pos) = survol {
        harness.hover_at(pos);
        harness.run();
    }
    harness.snapshot(nom);
}

/// **L'onglet « Chat »** : « Tester le son », fermeture automatique, formulaire canal → mot →
/// « Ajouter », neuf recherches en tuiles à légende, quatre par rangée.
#[test]
fn options_onglet_chat_recherches() {
    capture_onglet_chat(
        "options_chat_recherches",
        Some(recherches_de_chat()),
        Default::default(),
        None,
    );
}

/// Une tuile survolée : voile et croix de retrait, l'idiome des tuiles d'Alertes. La position
/// vise le centre de la deuxième tuile (panneau à x ≈ 47, tuiles de ≈ 155 px et gouttière de 12).
#[test]
fn options_onglet_chat_survol_d_une_tuile() {
    capture_onglet_chat(
        "options_chat_survol_tuile",
        Some(recherches_de_chat()),
        Default::default(),
        Some(egui::pos2(47.0 + 155.0 + 12.0 + 77.0, 470.0)),
    );
}

/// Aucune recherche : le formulaire seul, et le bloc d'information qui dit quoi faire.
#[test]
fn options_onglet_chat_vide() {
    capture_onglet_chat(
        "options_chat_vide",
        Some(Default::default()),
        Default::default(),
        None,
    );
}

/// **La carte de chat par-dessus le jeu**, produite par le VRAI moteur : une recherche posée, un
/// message de chat en direct qui lui correspond, l'alerte drainée et le toast construit comme
/// `engine_thread` le fait — jamais un `ChatAlert` fabriqué à la main.
#[test]
fn panneau_suivi_avec_carte_de_chat() {
    use overlay_engine::{ChatChannel, ChatFilter, ChatFilterScope};
    use overlay_ingest::LineBatch;

    let mut engine = test_engine();
    engine.set_chat_filters(vec![ChatFilter::new(
        ChatFilterScope::Channel(ChatChannel::Commerce),
        "gelano",
    )
    .expect("mot")]);
    engine
        .ingest_batch(&LineBatch {
            lines: vec![
                " INFO 21:08:40,000 [AWT-EventQueue-0] (aPV:174) - [Commerce] Huppermage-Bleu : \
                 vends Gelano 900k, prix ferme, mp si intéressé — je suis à Bonta près du zaap"
                    .to_string(),
            ],
            is_initial_load: false,
        })
        .expect("ingestion du message");
    let alert = engine
        .drain_chat_alerts()
        .pop()
        .expect("le message doit correspondre à la recherche");
    let created_at = std::time::Instant::now();
    let toast = WatchlistToast {
        name: alert.author.clone(),
        kind: WatchlistKind::Item,
        reason: WatchlistToastReason::Chat {
            channel: alert.channel,
            word: alert.filter.text,
            author: alert.author,
            message: alert.message,
        },
        catalog_id: None,
        created_at,
        confetti: Vec::new(),
        hide_at: Some(created_at + TOAST_DURATION),
    };

    let mut textures = Textures::new();
    let mut combat_side = CombatSide::default();
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let shortcuts = ShortcutBindings::default();
    // Une seconde après la création : le fondu d'entrée (`POP_DURATION`, 0,35 s) est fini, la
    // carte est au repos — à `created_at` pile, tout ce qui suit le fondu serait transparent.
    let now = toast.created_at + std::time::Duration::from_secs(1);

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
                watchlist_selection: &mut Default::default(),
                watchlist_toast: Some(&toast),
                catalog: &catalog,
                catalog_stale: false,
                remote_icons: &remote_icon_store,
                remote_icon_textures: &mut remote_icon_textures,
                auth_status: &auth_status,
                auth_command_tx: &auth_sink,
                interactive: true,
                shortcuts: &shortcuts,
                now,
                options: None,
            },
        );
    });
    harness.run();
    harness.snapshot("watchlist_avec_carte_de_chat");
}
