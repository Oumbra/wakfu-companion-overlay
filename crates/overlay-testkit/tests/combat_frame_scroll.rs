//! Snapshot testing du cadre ennemi à défilement (`overlay_ui::panels::combat_frame_scroll::
//! EnemyFrameScroll`) — au-delà de 6 ennemis (`combat_frame::MAX_FRAME_SLOTS`), voir sa doc de
//! module pour l'architecture (validée par plusieurs artefacts interactifs).
//!
//! **Fixtures construites à la main, contrairement à `tests/panels.rs`** : sa règle « jamais un
//! `SessionSnapshot`/`RenderContent` construit à la main » NE s'applique PAS ici — `EnemyFrameScroll
//! ::show` ne prend en entrée qu'un `&[&FighterDamage]` brut, jamais un `SessionSnapshot`/
//! `RenderContent` complet, et aucun rejeu de `wakfu.log` disponible n'aligne plus de 6 ennemis dans
//! un seul combat pour exercer ce chemin. `FighterDamage` est une structure de données simple et
//! stable (champs publics), pas un agrégat qui pourrait dériver silencieusement d'un changement de
//! forme comme le craint la doc de `tests/panels.rs` pour `SessionSnapshot`.
//!
//! **Driver logiciel requis** : même prérequis que `tests/panels.rs` (voir sa doc) —
//! `mesa-vulkan-drivers` sous Linux.

use egui_kittest::Harness;
use overlay_engine::{FighterDamage, Gender};
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::panels::combat_frame_scroll::EnemyFrameScroll;
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::ui_icons::UiIcons;

/// Ennemi minimal — jamais de classe/portrait (voir la doc de `FighterDamage::class_name` : les
/// ennemis n'en ont jamais), sexe et statuts sans effet sur ce widget laissés à leur valeur neutre.
fn enemy(name: String, total_damage: i64) -> FighterDamage {
    FighterDamage {
        name,
        is_ally: false,
        total_damage,
        total_heal: 0,
        class_name: None,
        gender: Gender::M,
        xp_gained: 0,
        spells: Default::default(),
        is_ko: false,
    }
}

/// Rejeu d'une breach à 14 monstres (au-delà des 6 emplacements du plus grand gabarit) : vérifie que
/// le cadre à défilement s'affiche sans paniquer, cadre fixe + portraits + scrollbar compris.
#[test]
fn cadre_ennemi_a_defilement_au_dela_de_six_ne_panique_pas() {
    const N: i64 = 14;
    let fighters: Vec<FighterDamage> = (1..=N)
        .map(|i| enemy(format!("Bouftou {i}"), i * 137))
        .collect();
    let refs: Vec<&FighterDamage> = fighters.iter().collect();
    let total_damage: i64 = refs.iter().map(|f| f.total_damage).sum();

    let mut portraits: Option<PortraitAtlas> = None;
    let mut combat_frame: Option<CombatFrame> = None;
    let mut icons: Option<UiIcons> = None;

    let mut harness = Harness::new_ui(move |ui| {
        let ctx = ui.ctx().clone();
        // Même style que la production (voir `tests/panels.rs::Textures::get_or_load`) — sans quoi
        // ce snapshot ne vaudrait plus rien pour vérifier visuellement le design system.
        overlay_ui::style::apply(&ctx);
        let portraits = portraits.get_or_insert_with(|| PortraitAtlas::load(&ctx));
        let combat_frame = combat_frame.get_or_insert_with(|| CombatFrame::load(&ctx));
        let icons = icons.get_or_insert_with(|| UiIcons::load(&ctx));
        EnemyFrameScroll::show(ui, combat_frame, portraits, icons, &refs, total_damage);
    });

    harness.run();
    harness.snapshot("combat_frame_scroll_14_ennemis");

    // Scrollbar cachée par défaut (voir doc de module d'`EnemyFrameScroll`) : la révéler exige de
    // survoler le cadre entier — `outer_margin(8.0)` ajouté par `Harness::new_ui` (voir
    // `tests/panels.rs` pour le même repère), cadre 70×393 : un point bien à l'intérieur suffit.
    harness.hover_at(egui::pos2(8.0 + 35.0, 8.0 + 150.0));
    harness.run();
    harness.snapshot("combat_frame_scroll_14_ennemis_survol");
}
