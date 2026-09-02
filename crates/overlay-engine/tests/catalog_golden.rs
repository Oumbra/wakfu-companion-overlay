//! Golden files de non-régression du catalogue (lot L3, §7.4 du plan — voir la ligne de statut
//! « Résolution d'objet identique au web sur les golden files » dans `docs/plan-architecture.md`
//! §12). Contrairement aux tests unitaires de `catalog.rs`/`dungeon.rs`/`monster_family.rs` (fixtures
//! minces, inline, un cas à la fois), ces trois fichiers sous `tests/golden/` forment un petit
//! univers COHÉRENT et RÉUTILISÉ tel quel (mêmes ids croisés d'un fichier à l'autre — le donjon
//! Larventura référence le monstre-boss El Pochito, la famille Bworks regroupe deux monstres du
//! catalogue) : un test qui casse ici signale soit une régression de résolution, soit un changement
//! de FORME du format serveur (`compact-index.ts`/`dungeons.ts`/`monster-families.ts` côté
//! `wakfu-companion`) qu'il faut répercuter délibérément dans ce crate — jamais une simple
//! coïncidence de fixture locale.
//!
//! ⚠️ Contenu placeholder (voir `overlay_sync::catalog_cache` pour la même mise en garde côté
//! repli embarqué) : ce sandbox ne peut pas atteindre la vraie base Neon pour capturer un VRAI
//! extrait du référentiel de production — seuls id 65 (Larventura) et id 24029 (Larme d'Ogrest)
//! sont des ancrages réels documentés ailleurs dans ce dépôt (voir `docs/plan-architecture.md` et
//! le CLAUDE.md de `wakfu-companion`, carte Récap) ; le reste (gfxId, rareté, ids de monstres/
//! familles secondaires) est une fixture inventée mais interne-cohérente.

use overlay_engine::{
    CatalogIndex, DungeonIndex, IconKind, MonsterClassification, MonsterFamilyIndex, WakfuRarity,
};

fn load_golden(name: &str) -> serde_json::Value {
    let path = format!("{}/tests/golden/{name}", env!("CARGO_MANIFEST_DIR"));
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("lecture de {path} : {err}"));
    serde_json::from_str(&content).unwrap_or_else(|err| panic!("JSON invalide dans {path} : {err}"))
}

#[test]
fn resolution_du_catalogue_objets_monstres_stable() {
    let catalog = CatalogIndex::from_compact_json(&load_golden("catalog-index.json"));
    assert!(!catalog.is_empty());

    // Objet résolu par id — icône ET rareté.
    let icon = catalog
        .find_item_icon("peu importe", Some(24029))
        .expect("Larme d'Ogrest doit être résolue par id");
    assert_eq!(icon.kind, IconKind::Item);
    assert_eq!(icon.gfx_id, "1234");
    assert_eq!(
        catalog.find_item_rarity("peu importe", Some(24029)),
        WakfuRarity::Rare
    );
    assert!(!catalog.find_item_has_recipe("peu importe", Some(24029)));

    // Objet résolu par nom, insensible à la casse/accents — recette présente cette fois.
    assert!(catalog.find_item_has_recipe("pain de craqueleur", None));

    // Monstre boss résolu par id.
    assert_eq!(
        catalog.find_monster_classification("peu importe", Some(24875)),
        MonsterClassification::Boss
    );

    // Deux monstres de la même famille, classements distincts.
    assert_eq!(
        catalog.find_monster_family_id("peu importe", Some(501)),
        Some(42)
    );
    assert_eq!(
        catalog.find_monster_classification("peu importe", Some(501)),
        MonsterClassification::Archi
    );
    assert_eq!(
        catalog.find_monster_classification("peu importe", Some(502)),
        MonsterClassification::Dominant
    );
    assert_eq!(
        catalog.find_monster_classification("peu importe", Some(503)),
        MonsterClassification::None
    );
}

#[test]
fn resolution_des_donjons_stable() {
    let dungeons = DungeonIndex::from_json(&load_golden("dungeons.json"));
    assert!(!dungeons.is_empty());

    let larventura = dungeons.find_by_id(65).expect("donjon 65 doit être résolu");
    assert_eq!(larventura.fr, "Larventura");
    assert_eq!(larventura.boss_monster_ids, vec![24875]);
    assert_eq!(larventura.monster_family_ids, vec![42]);

    // Résolution inverse : le monstre-boss du catalogue mène au bon donjon.
    let by_boss = dungeons
        .find_by_boss_monster_id(24875)
        .expect("El Pochito doit résoudre Larventura");
    assert_eq!(by_boss.id, 65);

    // Donjon sans boss ni famille (type ARCADE) reste résolu par id.
    let sans_boss = dungeons.find_by_id(12).expect("donjon 12 doit être résolu");
    assert!(sans_boss.boss_monster_ids.is_empty());
}

#[test]
fn resolution_des_familles_de_monstres_stable() {
    let families = MonsterFamilyIndex::from_json(&load_golden("monster-families.json"));
    assert!(!families.is_empty());

    let bworks = families
        .find_by_id(42)
        .expect("famille 42 doit être résolue");
    assert_eq!(bworks.fr, "Bworks");
    assert_eq!(bworks.en, "Bworks");
}

/// Vérifie la cohérence CROISÉE entre les trois fichiers golden : le donjon Larventura (65)
/// référence le monstre-boss 24875 (catalogue) ET la famille 42 (référentiel familles) — un
/// consommateur futur (panneau Combat conscient du donjon, pas encore construit — voir §9 du plan)
/// pourra chaîner ces trois index sans jamais retomber sur un id orphelin. Ce test échouerait si
/// une future édition des fixtures cassait cette cohérence par inadvertance (id changé d'un côté,
/// pas de l'autre).
#[test]
fn les_trois_referentiels_golden_restent_croises_de_facon_coherente() {
    let catalog = CatalogIndex::from_compact_json(&load_golden("catalog-index.json"));
    let dungeons = DungeonIndex::from_json(&load_golden("dungeons.json"));
    let families = MonsterFamilyIndex::from_json(&load_golden("monster-families.json"));

    let larventura = dungeons.find_by_id(65).unwrap();
    for &boss_id in &larventura.boss_monster_ids {
        assert!(
            catalog
                .find_monster_icon("peu importe", Some(boss_id))
                .is_some(),
            "le boss {boss_id} de Larventura doit exister dans le catalogue monstres"
        );
    }
    for &family_id in &larventura.monster_family_ids {
        assert!(
            families.find_by_id(family_id).is_some(),
            "la famille {family_id} de Larventura doit exister dans le référentiel familles"
        );
    }
}
