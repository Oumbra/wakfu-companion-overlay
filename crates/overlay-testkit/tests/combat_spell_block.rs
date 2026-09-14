//! Snapshot testing du bloc « ligne de sorts » du panneau Combat et de sa sélection par le cadre
//! à médaillons (`overlay_ui::panels::combat_spell_block`, spec validée en artefact les 11-12
//! sept. 2026, refonte « Sélection par le cadre » décidée le 12 sept. au soir, **vue Ennemis
//! ajoutée le 13 sept.** — proposition « Sorts ennemis du combat » en artefact, revue par un
//! expert).
//!
//! **Même règle que `tests/panels.rs`** : le `SessionSnapshot` vient du rejeu du vrai
//! `crates/overlay-engine/tests/wakfu.log` à travers le vrai `Engine`, jamais d'un littéral. Le
//! premier combat du rejeu (six alliés, `Anonyme-Ouginak1`, `Anonyme-Pandawa1`, `Anonyme-Ecaflip1`… contre six
//! Koko : deux `Tikoko`, deux `Grokoko`, deux `Kokoko`) fournit tous les cas : dernier lanceur
//! allié suivi automatiquement (liseré et point sur lui), un allié à cinq sorts dont un critique
//! (une rangée), un allié à neuf sorts (deux rangées), trois alliés qui n'ont rien lancé (jamais
//! cliquables, jamais marqués) ; côté ennemi, un dernier lanceur suivi automatiquement, des
//! homonymes (deux lignes, deux boutons du même nom) et des sorts résolus par le `breed` du
//! monstre (« Coup d'Koko » : `spells/4.png` chez un Tikoko ou un Kokoko, `spells/3.png` chez un
//! Grokoko).
//!
//! **Icônes de sorts : fixtures versionnées, jamais le réseau** (§17.1 du plan). Les PNG de
//! `fixtures/spells/` sont injectés dans `RemoteIconStore::empty()` par `preload`, adressés par le
//! `IconRef` que `overlay_engine::resolve_cast` résout LUI-MÊME pour ces sorts et ces lanceurs —
//! la même correspondance nom + classe (ou nom + `breed`) → numéro d'image qu'au runtime, pas une
//! seconde table écrite ici (même principe que `examples/shared/wakassets_fixtures.rs`). Une mise
//! à jour d'un référentiel qui changerait l'image de l'un d'eux fait échouer `preload_spell_
//! fixtures` explicitement, plutôt que de laisser un pavé vide passer pour normal dans la capture.
//!
//! **Interactions par nœud d'accessibilité** (`get_by_role_and_label`) plutôt que par coordonnées
//! calculées à la main : les portraits cliquables du cadre sont déclarés `Button` avec le nom du
//! combattant (deux homonymes = deux boutons du même nom, `get_all_by_role_and_label`), les sorts
//! `Image` avec « Sort n : nom ». Seul le switch Alliés/Ennemis est cliqué par coordonnées (les
//! mêmes que `tests/panels.rs`).
//!
//! **Driver logiciel requis** : même prérequis que `tests/panels.rs`.

use std::sync::atomic::{AtomicU32, Ordering};

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use overlay_engine::{resolve_cast, CatalogIndex, Engine, FightSnapshot, SessionSnapshot};
use overlay_ingest::Tailer;
use overlay_ui::panels::combat::CombatSide;
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

/// Les sorts du premier combat du rejeu dont l'icône est fournie en fixture — nom du LANCEUR
/// (tel qu'il apparaît dans `FightSnapshot::fighters`, le premier de ce nom), sort, fichier. Le
/// numéro de fichier N'EST PAS une table parallèle : il est vérifié contre ce que `resolve_cast`
/// résout pour ce lanceur (classe pour un allié, `breed` pour un ennemi — voir la doc de module).
const SPELL_FIXTURES: &[(&str, &str, &str, &[u8])] = &[
    (
        "Anonyme-Ouginak1",
        "Hachure",
        "6262",
        include_bytes!("../fixtures/spells/6262.png"),
    ),
    (
        "Anonyme-Ouginak1",
        "Croc-en-jambe",
        "6261",
        include_bytes!("../fixtures/spells/6261.png"),
    ),
    (
        "Anonyme-Ouginak1",
        "Canine",
        "6287",
        include_bytes!("../fixtures/spells/6287.png"),
    ),
    (
        "Anonyme-Pandawa1",
        "Coup Laiteux",
        "2196",
        include_bytes!("../fixtures/spells/2196.png"),
    ),
    (
        "Anonyme-Pandawa1",
        "Ether",
        "2208",
        include_bytes!("../fixtures/spells/2208.png"),
    ),
    // Mécaniques de classe, ajoutées au référentiel le 12 sept. (auparavant en pavé « ? »).
    (
        "Anonyme-Ouginak1",
        "Proie",
        "6283",
        include_bytes!("../fixtures/spells/6283.png"),
    ),
    (
        "Anonyme-Pandawa1",
        "Karcham",
        "2195",
        include_bytes!("../fixtures/spells/2195.png"),
    ),
    (
        "Anonyme-Pandawa1",
        "Chamrak",
        "975",
        include_bytes!("../fixtures/spells/975.png"),
    ),
    (
        "Anonyme-Ecaflip1",
        "Bond du félin",
        "983",
        include_bytes!("../fixtures/spells/983.png"),
    ),
    // Sorts de monstres (13 sept.) : le même nom chez deux monstres, deux images — c'est le
    // `breed` de la jointure qui départage. Tous les sorts des Koko du log n'utilisent que ces
    // deux fichiers.
    (
        "Grokoko",
        "Coup d'Koko",
        "3",
        include_bytes!("../fixtures/spells/3.png"),
    ),
    (
        "Tikoko",
        "Coup d'Koko",
        "4",
        include_bytes!("../fixtures/spells/4.png"),
    ),
    (
        "Kokoko",
        "Coup d'Koko",
        "4",
        include_bytes!("../fixtures/spells/4.png"),
    ),
];

fn preload_spell_fixtures(store: &RemoteIconStore, fight: &FightSnapshot) {
    for (caster, spell, expected_gfx_id, bytes) in SPELL_FIXTURES {
        let fighter = fight
            .fighters
            .iter()
            .find(|f| f.name == *caster)
            .unwrap_or_else(|| panic!("{caster} absent du premier combat du rejeu"));
        let resolved = resolve_cast(fighter, spell)
            .unwrap_or_else(|| panic!("{spell} ({caster}) absent du référentiel de son camp"));
        let icon = resolved
            .icon
            .unwrap_or_else(|| panic!("{spell} ({caster}) sans image dans le référentiel"));
        assert_eq!(
            icon.gfx_id, *expected_gfx_id,
            "{spell} ({caster}) : le référentiel pointe désormais sur une autre image, mettre à \
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

/// Coordonnées des deux boutons du switch Alliés/Ennemis dans le harnais — mêmes repères que
/// `tests/panels.rs::panneau_combat_tooltip_switch_allies_ennemis_au_dessus`, décalés de la
/// colonne des portraits, présente ici. Abscisse : 8 de marge du harnais, 8, 70 (cadre), 6
/// (`COLUMN_GAP`), 6 (`LEADER_PANEL_PADDING`) et 15, soit 113 pour « Alliés », 30 de plus (143)
/// pour « Ennemis ». Ordonnée : 8, 50 et 13, soit 71.
const SWITCH_ALLIES: egui::Pos2 = egui::pos2(113.0, 71.0);
const SWITCH_ENEMIES: egui::Pos2 = egui::pos2(143.0, 71.0);

fn click_at(harness: &mut Harness<'_>, pos: egui::Pos2) {
    harness.remove_cursor();
    harness.drag_at(pos);
    harness.drop_at(pos);
    harness.run();
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
    assert_eq!(
        fight.fighters.iter().filter(|f| !f.is_ally).count(),
        6,
        "et les six médaillons ennemis"
    );
    // L'état RÉEL du rejeu, vérifié plutôt que supposé : c'est lui que les captures montrent.
    let auto_selected = last_ally_caster_name(&fight).to_string();
    assert_eq!(auto_selected, "Anonyme-Ouginak1");
    let last_enemy = fight
        .last_enemy_caster
        .expect("le premier combat du rejeu a au moins un sort ennemi");
    assert_eq!(fight.fighters[last_enemy].name, "Tikoko");
    assert_eq!(fight.fighters[last_enemy].breed, Some(4730));
    // Libellé « Sort 1 : nom » du premier sort du tour d'un combattant, tel que le bloc le déclare
    // à l'accessibilité — nom du référentiel de son camp.
    let first_spell_label = |idx: usize| {
        let fighter = &fight.fighters[idx];
        let cast = &fighter.last_turn_casts[0];
        let name =
            resolve_cast(fighter, &cast.spell).map_or(cast.spell.clone(), |r| r.name.to_string());
        format!("Sort 1 : {name}")
    };
    let last_enemy_first_spell = first_spell_label(last_enemy);
    // Un autre ennemi à épingler : le premier Grokoko ayant lancé un sort (son « Coup d'Koko »
    // porte l'AUTRE image, `spells/3.png`), et le libellé de son premier sort pour le survol.
    let (pinned_enemy, pinned_name) = fight
        .fighters
        .iter()
        .enumerate()
        .find(|(_, f)| !f.is_ally && f.name == "Grokoko" && !f.last_turn_casts.is_empty())
        .map(|(i, f)| (i, f.name.clone()))
        .expect("un Grokoko a lancé un sort");
    let pinned_first_spell = first_spell_label(pinned_enemy);
    let homonyms_before_pinned = fight.fighters[..pinned_enemy]
        .iter()
        .filter(|f| f.name == pinned_name)
        .count();

    let remote_icon_store = RemoteIconStore::empty();
    preload_spell_fixtures(&remote_icon_store, &fight);
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
    // Raccourcis PAR DÉFAUT — voir `overlay_ui::shortcuts` : les infobulles affichent les mêmes
    // combinaisons qu'avant leur personnalisation, captures inchangées de ce fait.
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
                login: None,
            },
        );
    });

    // 1. Suivi automatique : liseré ET point sur le dernier lanceur allié (Anonyme-Ouginak1, cinq sorts
    //    dont un critique en 4e). Anonyme-Huppermage1, Anonyme-Sadida1 et Anonyme-Zobal1 n'ont rien lancé : ni marque
    //    ni bouton.
    harness.run();
    harness.snapshot("combat_spell_block_suivi_auto");
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Anonyme-Huppermage1")
            .is_none(),
        "un allié sans sort ne doit pas être cliquable"
    );

    // 2. Clic sur le portrait de Anonyme-Pandawa1 (neuf sorts, deux rangées) : le liseré passe sur
    //    lui, le point reste sur Anonyme-Ouginak1, le bloc montre ses neuf sorts.
    harness
        .get_by_role_and_label(Role::Button, "Anonyme-Pandawa1")
        .click();
    harness.run();
    harness.snapshot("combat_spell_block_epingle_deux_rangees");

    // 3. Clic sur Anonyme-Ecaflip1 : un seul sort (`Bond du félin`) — le bloc reste à une rangée.
    harness
        .get_by_role_and_label(Role::Button, "Anonyme-Ecaflip1")
        .click();
    harness.run();
    harness.snapshot("combat_spell_block_epingle_un_sort");

    // 4. Clic sur le porteur du point (Anonyme-Ouginak1) : retour au suivi automatique, puis survol de
    //    son critique : liseré ACCENT et infobulle « Croc-en-jambe · Critique » au-dessus (une
    //    ligne, sans nom de lanceur).
    harness
        .get_by_role_and_label(Role::Button, "Anonyme-Ouginak1")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Image, "Sort 4 : Croc-en-jambe")
        .hover();
    harness.run();
    harness.snapshot("combat_spell_block_survol");

    // 5. Vue Ennemis (13 sept.) : le bloc de CE camp, en suivi automatique sur le dernier lanceur
    //    ennemi (un Tikoko) — liseré et point sur son médaillon, ses sorts du tour en dessous du
    //    dernier groupe, image `spells/4.png` (breed 4730). Les sorts alliés ne sont plus dans
    //    l'arbre d'accessibilité : chaque vue montre le bloc de son camp.
    click_at(&mut harness, SWITCH_ENEMIES);
    assert!(
        harness
            .query_by_role_and_label(Role::Image, "Sort 1 : Proie")
            .is_none(),
        "le bloc allié ne doit pas exister en vue Ennemis"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Image, last_enemy_first_spell.as_str())
            .is_some(),
        "le bloc ennemi doit montrer les sorts du dernier lanceur ennemi"
    );
    harness.snapshot("combat_spell_block_ennemis_suivi_auto");

    // 6. Clic sur le premier Grokoko ayant lancé un sort (deux Grokoko homonymes = deux boutons
    //    du même nom, choisi par sa position dans `fighters`) : le liseré passe sur lui, le point
    //    reste sur le Tikoko, le bloc montre ses sorts — son « Coup d'Koko » porte l'AUTRE image
    //    (`spells/3.png`, breed 4728), la clé par `breed` à l'œuvre.
    harness
        .get_all_by_role_and_label(Role::Button, pinned_name.as_str())
        .nth(homonyms_before_pinned)
        .expect("le Grokoko épinglable a un bouton")
        .click();
    harness.run();
    harness.snapshot("combat_spell_block_ennemis_epingle");

    // 7. Survol de son premier sort : liseré ACCENT et infobulle au-dessus, même règle qu'en vue
    //    Alliés (nom du référentiel monstre, « · Critique » s'il y a lieu).
    harness
        .get_by_role_and_label(Role::Image, pinned_first_spell.as_str())
        .hover();
    harness.run();
    harness.snapshot("combat_spell_block_ennemis_survol");

    // 8. Retour en vue Alliés : l'état allié est retrouvé tel quel (suivi automatique sur
    //    Anonyme-Ouginak1, capture identique à l'étape 1) — l'épingle ennemie n'a touché à rien de ce
    //    côté. Puis retour en vue Ennemis : l'épingle sur le Grokoko a survécu à l'aller-retour.
    click_at(&mut harness, SWITCH_ALLIES);
    harness.snapshot("combat_spell_block_suivi_auto");
    click_at(&mut harness, SWITCH_ENEMIES);
    assert!(
        harness
            .query_by_role_and_label(Role::Image, pinned_first_spell.as_str())
            .is_some(),
        "l'épingle ennemie doit survivre à un aller-retour en vue Alliés"
    );
}
