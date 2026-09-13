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
use overlay_engine::{CatalogIndex, FighterDamage, Gender, SpellCastRecord};
use overlay_ui::panels::combat_frame::{CombatFrame, SelectionMarks};
use overlay_ui::panels::combat_frame_scroll::EnemyFrameScroll;
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::ui_icons::UiIcons;

/// Ennemi minimal — jamais de classe/portrait (voir la doc de `FighterDamage::class_name` : les
/// ennemis n'en ont jamais), sexe et statuts sans effet sur ce widget laissés à leur valeur neutre.
/// `casts` sorts du dernier tour : un ennemi qui en a lancé au moins un est cliquable et peut
/// porter une marque du bloc de sorts (voir `combat_spell_block`).
fn enemy(name: String, total_damage: i64, casts: usize) -> FighterDamage {
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
        last_turn_casts: (0..casts)
            .map(|i| SpellCastRecord {
                spell: format!("Sort {i}"),
                critical: false,
            })
            .collect(),
        last_turn: 0,
        breed: None,
    }
}

/// Clé de l'état de défilement d'`EnemyFrameScroll` — la même chaîne que `scroll_offset_id`
/// (privée) : le test règle le décalage directement, comme la molette le ferait, plutôt que de
/// synthétiser des événements de défilement.
fn scroll_offset_id() -> egui::Id {
    egui::Id::new("combat-frame-scroll-enemy-offset")
}

/// Rejeu d'une breach à 14 monstres (au-delà des 6 emplacements du plus grand gabarit) : vérifie que
/// le cadre à défilement s'affiche sans paniquer, cadre fixe + portraits + scrollbar compris.
#[test]
fn cadre_ennemi_a_defilement_au_dela_de_six_ne_panique_pas() {
    const N: i64 = 14;
    // Les Bouftou 1 et 3 ont lancé des sorts : 3 porte le liseré (sorts affichés), 1 le point
    // (dernier lanceur) — voir `marks` plus bas.
    let fighters: Vec<FighterDamage> = (1..=N)
        .map(|i| {
            enemy(
                format!("Bouftou {i}"),
                i * 137,
                if i == 1 || i == 3 { 2 } else { 0 },
            )
        })
        .collect();
    let refs: Vec<&FighterDamage> = fighters.iter().collect();
    let total_damage: i64 = refs.iter().map(|f| f.total_damage).sum();

    let mut portraits: Option<PortraitAtlas> = None;
    let mut combat_frame: Option<CombatFrame> = None;
    let mut icons: Option<UiIcons> = None;
    // Catalogue vide et store d'icônes sans thread réseau (voir `tests/panels.rs` pour le même
    // pattern) : ces 14 ennemis synthétiques ne sont résolus par aucun catalogue, le repli
    // générique (`icons.unknown_entity_texture()`) est donc le rendu attendu ici — ce test vérifie
    // la géométrie du défilement, pas la résolution d'icône de monstre (couverte ailleurs).
    let remote_icon_store = RemoteIconStore::empty();
    let mut remote_icon_textures = RemoteIconTextures::default();
    let catalog = CatalogIndex::default();

    let mut harness = Harness::new_ui(move |ui| {
        let ctx = ui.ctx().clone();
        // Même style que la production (voir `tests/panels.rs::Textures::get_or_load`) — sans quoi
        // ce snapshot ne vaudrait plus rien pour vérifier visuellement le design system.
        overlay_ui::style::apply(&ctx);
        let portraits = portraits.get_or_insert_with(|| PortraitAtlas::load(&ctx));
        let combat_frame = combat_frame.get_or_insert_with(|| CombatFrame::load(&ctx));
        let icons = icons.get_or_insert_with(|| UiIcons::load(&ctx));
        EnemyFrameScroll::show(
            ui,
            combat_frame,
            portraits,
            icons,
            &catalog,
            &remote_icon_store,
            &mut remote_icon_textures,
            &refs,
            total_damage,
            Some(SelectionMarks {
                ring_slot: Some(2),
                dot_slot: Some(0),
            }),
        );
    });

    // 1. Sans défilement : liseré sur le 3ᵉ portrait, point sur le 1ᵉʳ — tous deux SOUS le
    //    pourcentage et sous la scrollbar (retour utilisateur du 13 sept. 2026).
    harness.run();
    harness.snapshot("combat_frame_scroll_14_ennemis");

    // Scrollbar TOUJOURS visible (voir doc de module d'`EnemyFrameScroll` — revirement 2026-09-08,
    // plus de condition de survol) : ce second snapshot ne sert donc plus qu'à vérifier la tooltip
    // au survol d'un portrait précis. `outer_margin(8.0)` ajouté par `Harness::new_ui` (voir
    // `tests/panels.rs` pour le même repère), cadre 70×393 : un point bien à l'intérieur suffit.
    harness.hover_at(egui::pos2(8.0 + 35.0, 8.0 + 150.0));
    harness.run();
    harness.snapshot("combat_frame_scroll_14_ennemis_survol");

    // 3. Défilement de deux emplacements et demi : le portrait 3 (liseré) n'est plus qu'à moitié
    //    dans la bande visible — le liseré a suivi et se rogne avec lui au bord haut, le point du
    //    portrait 1 est sorti avec lui. Décalage posé directement dans l'état du widget (voir
    //    `scroll_offset_id`).
    harness.remove_cursor();
    harness
        .ctx
        .data_mut(|d| d.insert_temp(scroll_offset_id(), 2.5_f32));
    harness.run();
    harness.snapshot("combat_frame_scroll_14_ennemis_defile");
}
