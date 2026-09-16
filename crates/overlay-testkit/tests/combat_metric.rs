//! Snapshot testing du switch de grandeur du panneau Combat (`overlay_ui::panels::combat::
//! CombatMetric`, 14 sept. 2026) — trois positions (Dégâts / Armure / Soins) dans le bandeau
//! leader, au-dessous du switch Alliés/Ennemis : le camp affiché et la grandeur mesurée sont deux
//! axes indépendants (demande utilisateur, l'armure donnée et les soins sont captés pour les deux
//! camps — voir `overlay_engine::session::FighterDamage`).
//!
//! **Fixture construite à la main**, même exception que `tests/combat_bars.rs` et
//! `tests/combat_frame_scroll.rs` : le `wakfu.log` vendu contient bien des soins et une ligne
//! d'armure (voir `overlay-engine/tests/session_real_log.rs`), mais pas un groupe qui répartit
//! clairement les trois grandeurs entre quatre alliés — ce que ces planches doivent justement
//! montrer d'un coup d'œil. Les valeurs sont donc choisies pour que le classement CHANGE d'une
//! grandeur à l'autre (le plus gros frappeur n'est ni le meilleur blindeur ni le meilleur
//! soigneur), seule façon de vérifier visuellement que les barres, le total et les pourcentages
//! sur les portraits suivent bien le switch.
//!
//! **Driver logiciel requis** : même prérequis que `tests/panels.rs` (voir sa doc) —
//! `mesa-vulkan-drivers` sous Linux.

use egui_kittest::Harness;
use overlay_engine::{CatalogIndex, FightSnapshot, FighterDamage, Gender};
use overlay_ui::panels::combat::{CombatMetric, CombatSide};
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content::{
    paint_content, AuthStatus, NoopAuthSink, OverlayKind, RenderContent,
};
use overlay_ui::shortcuts::ShortcutBindings;
use overlay_ui::ui_icons::UiIcons;

fn fighter(
    name: &str,
    class_name: Option<&str>,
    is_ally: bool,
    total_damage: i64,
    total_armor: i64,
    total_heal: i64,
) -> FighterDamage {
    FighterDamage {
        name: name.to_string(),
        is_ally,
        total_damage,
        total_heal,
        total_armor,
        class_name: class_name.map(str::to_string),
        gender: Gender::M,
        xp_gained: 0,
        spells: Default::default(),
        heal_spells: Default::default(),
        armor_spells: Default::default(),
        is_ko: false,
        last_turn_casts: Vec::new(),
        last_turn: 0,
        breed: None,
    }
}

/// Un groupe de quatre où chaque grandeur a son propre leader : le Iop frappe, le Féca blinde,
/// l'Eniripsa soigne, le Crâ fait un peu des trois. Deux ennemis dont un qui se blinde et se
/// soigne lui-même — c'est ce que la vue Ennemis + Armure doit montrer.
fn fight() -> FightSnapshot {
    FightSnapshot {
        fight_id: 1,
        ongoing: true,
        result: None,
        fighters: vec![
            fighter("Anonyme-Ouginak1", Some("iop"), true, 12_480, 0, 0),
            fighter("Fayto", Some("feca"), true, 3_120, 9_460, 640),
            fighter("Sagitta Lucis", Some("eniripsa"), true, 1_890, 1_200, 7_310),
            fighter("Caliburnus", Some("cra"), true, 6_740, 460, 1_050),
            fighter("Tourbillonneur", None, false, 9_310, 4_200, 2_150),
            fighter("Chafer Elite", None, false, 4_020, 0, 0),
        ],
        started_at_ms: 0,
        last_ally_caster: None,
        last_enemy_caster: None,
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

fn harness_for(side: CombatSide, metric: CombatMetric) -> Harness<'static> {
    harness_with(fight(), side, metric)
}

fn harness_with(fight: FightSnapshot, side: CombatSide, metric: CombatMetric) -> Harness<'static> {
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let mut textures = Textures {
        portraits: None,
        combat_frame: None,
        icons: None,
    };
    let mut combat_side = side;
    let mut combat_metric = metric;
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
                session_totals: &Default::default(),
                session_uptime: std::time::Duration::ZERO,
                options: None,
                login: None,
            },
        );
    })
}

/// Les trois grandeurs, côté alliés — mêmes combattants, trois classements et trois totaux. Un
/// test PAR capture, jamais une boucle : `egui_kittest` panique si plusieurs `SnapshotResults`
/// sont abandonnés sans être fusionnés (un `Harness` par capture en produit un chacun).
#[test]
fn degats_du_camp_allie() {
    let mut harness = harness_for(CombatSide::Allies, CombatMetric::Damage);
    harness.run();
    harness.snapshot("combat_metrique_degats");
}

#[test]
fn armure_du_camp_allie() {
    let mut harness = harness_for(CombatSide::Allies, CombatMetric::Armor);
    harness.run();
    harness.snapshot("combat_metrique_armure");
}

/// Total à SEPT chiffres : le Iop porté à un peu plus d'un million, comme la capture utilisateur
/// du 16 sept. 2026 (« 1 047 404 » mordait de 10 px sur la case Soins). Le total passe au corps
/// compact (16) et tient dans le bandeau, qui déborde de 6 px de chaque côté de la colonne quel
/// que soit le total — voir `combat::show_leader_row`.
#[test]
fn total_a_sept_chiffres_au_corps_compact() {
    let mut fight = fight();
    fight.fighters[0].total_damage = 1_035_654; // total du camp : 1 047 404
    let mut harness = harness_with(fight, CombatSide::Allies, CombatMetric::Damage);
    harness.run();
    harness.snapshot("combat_metrique_total_sept_chiffres");
}

#[test]
fn soins_du_camp_allie() {
    let mut harness = harness_for(CombatSide::Allies, CombatMetric::Heal);
    harness.run();
    harness.snapshot("combat_metrique_soins");
}

/// Armure côté ENNEMIS : un monstre qui se blinde a sa barre comme n'importe quel allié — c'est
/// tout l'intérêt d'avoir gardé les deux axes indépendants.
#[test]
fn armure_du_camp_ennemi() {
    let mut harness = harness_for(CombatSide::Enemies, CombatMetric::Armor);
    harness.run();
    harness.snapshot("combat_metrique_armure_ennemis");
}

/// Un camp peuplé mais sans rien à montrer pour la grandeur choisie le dit explicitement, plutôt
/// que de laisser une colonne vide sans explication (voir `combat::show`) — ici les soins côté
/// ennemis d'un combat où seul le premier ennemi en produit… donc on retire ce dernier.
#[test]
fn grandeur_sans_valeur_affiche_son_message() {
    let mut fight = fight();
    fight.fighters.retain(|f| !f.is_ally && f.total_heal == 0);
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let mut textures = Textures {
        portraits: None,
        combat_frame: None,
        icons: None,
    };
    let mut combat_side = CombatSide::Enemies;
    let mut combat_metric = CombatMetric::Heal;
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    let shortcuts = ShortcutBindings::default();
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
                // Le panneau Combat n'a ni bustes de classe ni serveurs de jeu à peindre — ils
                // n'existent que pour l'onglet « Personnages » de la fenêtre Options.
                avatars: None,
                game_servers: &Default::default(),
                combat_side: &mut combat_side,
                combat_metric: &mut combat_metric,
                watchlist: &[],
                watchlist_enabled: true,
                spells_enabled: true,
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
                session_totals: &Default::default(),
                session_uptime: std::time::Duration::ZERO,
                options: None,
                login: None,
            },
        );
    });
    harness.run();
    harness.snapshot("combat_metrique_soins_sans_valeur");
}
