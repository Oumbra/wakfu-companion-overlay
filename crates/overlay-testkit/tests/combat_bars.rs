//! Snapshot testing de la fenêtre des barres de dégâts en breach (`overlay_ui::panels::
//! combat_bars::DamageBars`, 13 sept. 2026) — au-delà de six groupes, la colonne des barres ne
//! grandit plus : conteneur invisible haut de six groupes, ascenseur à gauche (celui du cadre),
//! fondu de 8 px du côté où du contenu est masqué. Voir sa doc de module.
//!
//! **Fixture construite à la main, contrairement à `tests/panels.rs`** : même exception que
//! `tests/combat_frame_scroll.rs` — aucun rejeu de `wakfu.log` disponible n'aligne plus de six
//! ennemis à dégâts dans un seul combat. Un `FightSnapshot` (champs publics, structure de données
//! simple) suffit ici, et le panneau COMPLET est peint (`paint_content`, comme en production)
//! pour vérifier la fenêtre dans son contexte : cadre à défilement à gauche, ligne leader
//! au-dessus, bloc « ligne de sorts » au-dessous, tous deux hors défilement.
//!
//! **Driver logiciel requis** : même prérequis que `tests/panels.rs` (voir sa doc) —
//! `mesa-vulkan-drivers` sous Linux.

use egui_kittest::Harness;
use overlay_engine::{CatalogIndex, FightSnapshot, FighterDamage, Gender, SpellCastRecord};
use overlay_ui::panels::combat::{CombatMetric, CombatSide};
use overlay_ui::panels::combat_bars::scroll_offset_id;
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content::{
    paint_content, AuthStatus, NoopAuthSink, OverlayKind, RenderContent,
};
use overlay_ui::shortcuts::ShortcutBindings;
use overlay_ui::ui_icons::UiIcons;

/// Ennemi minimal — voir `tests/combat_frame_scroll.rs::enemy`.
fn enemy(name: String, total_damage: i64, casts: usize) -> FighterDamage {
    FighterDamage {
        name,
        is_ally: false,
        total_damage,
        total_heal: 0,
        total_armor: 0,
        class_name: None,
        gender: Gender::M,
        xp_gained: 0,
        spells: Default::default(),
        heal_spells: Default::default(),
        armor_spells: Default::default(),
        is_ko: false,
        last_turn_casts: (0..casts)
            .map(|i| SpellCastRecord {
                spell: format!("Sort {i}"),
                critical: i == 1,
            })
            .collect(),
        last_turn: 0,
        breed: None,
    }
}

/// Breach synthétique : `n` Bouftous aux dégâts croissants (le tri par dégâts décroissants de la
/// colonne inverse donc l'ordre des portraits — les deux listes ne défilent pas ensemble, et c'est
/// voulu). Le Bouftou 3 a lancé deux sorts et est le dernier lanceur ennemi : le bloc « ligne de
/// sorts » s'affiche sous la fenêtre, hors défilement.
fn breach(n: i64) -> FightSnapshot {
    FightSnapshot {
        fight_id: 1,
        ongoing: true,
        result: None,
        fighters: (1..=n)
            .map(|i| enemy(format!("Bouftou {i}"), i * 137, if i == 3 { 2 } else { 0 }))
            .collect(),
        started_at_ms: 0,
        last_ally_caster: None,
        last_enemy_caster: Some(2),
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

fn harness_for(fight: FightSnapshot) -> Harness<'static> {
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let mut textures = Textures {
        portraits: None,
        combat_frame: None,
        icons: None,
    };
    let mut combat_side = CombatSide::Enemies;
    let mut combat_metric = CombatMetric::default();
    let catalog = CatalogIndex::default();
    let auth_status = AuthStatus::Connected;
    let auth_sink = NoopAuthSink;
    // Raccourcis PAR DÉFAUT — voir `overlay_ui::shortcuts` : les infobulles affichent les mêmes
    // combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
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
                combat_side: &mut combat_side,
                combat_metric: &mut combat_metric,
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
                login: None,
            },
        );
    })
}

/// Six groupes exactement : la fenêtre fait la hauteur de son contenu, ni ascenseur ni fondu —
/// le rendu est celui de l'ancienne liste, au pixel.
#[test]
fn six_groupes_ni_ascenseur_ni_fondu() {
    let mut harness = harness_for(breach(6));
    harness.run();
    harness.snapshot("combat_breche_6_ennemis");
}

/// Quatorze groupes : fenêtre de six, ascenseur à gauche en face de celui du cadre, fondu en bas
/// seulement au départ, des deux côtés au milieu, en haut seulement au bout. Le bloc de sorts
/// reste sous la fenêtre dans les trois cas. Décalage posé directement dans l'état du widget
/// (voir `scroll_offset_id`), comme la molette le ferait.
#[test]
fn quatorze_groupes_defilent_dans_une_fenetre_de_six() {
    let mut harness = harness_for(breach(14));
    harness.run();
    harness.snapshot("combat_breche_14_ennemis");

    // Deux groupes et demi plus bas : le 3ᵉ groupe coupé en haut, fondu des deux côtés.
    harness
        .ctx
        .data_mut(|d| d.insert_temp(scroll_offset_id(), 90.0_f32));
    harness.run();
    harness.snapshot("combat_breche_14_ennemis_defile");

    // Tout en bas (décalage volontairement excessif, ramené à la course maximale) : fondu en haut
    // seulement, poignée en bas de la fenêtre.
    harness
        .ctx
        .data_mut(|d| d.insert_temp(scroll_offset_id(), 10_000.0_f32));
    harness.run();
    harness.snapshot("combat_breche_14_ennemis_fin");
}
