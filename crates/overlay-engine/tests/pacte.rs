//! Extraction de pacte (« comme le fait déjà le site ») : un ramassage qui suit la ligne
//! `Action [WALKON] performed on interactive element : <id>` n'est PAS du butin de combat, et
//! part vers le compte comme un événement d'historique `pact` — miroir de la fenêtre
//! `PACT_EXTRACTION_WINDOW_MS` de `stats-store.service.ts` (dépôt web, `feat: detecte et suit les
//! extractions de pacte`).
//!
//! Lignes calquées sur celles du vrai `wakfu.log` de `tests/wakfu.log` (mêmes classes/threads,
//! mêmes formats), horaires resserrés autour du scénario testé.

use overlay_engine::{Engine, HistoryEventKind, HistoryPayload, SyncEvent};
use overlay_ingest::LineBatch;

const CREATION: &str = " INFO 20:34:02,833 [AWT-EventQueue-0] (aXI:47) - CREATION DU COMBAT";
const MONSTRE_REJOINT: &str = " INFO 20:34:02,971 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=1552060439 Sac à patates breed : 2335 [-1706442036778922] isControlledByAI=true obstacleId : -1 join the fight at {Point3 : (0, -14, 0)}";
const ALLIE_REJOINT: &str = " INFO 20:34:03,046 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=1552060439 Oumbra breed : 4 [90000007] isControlledByAI=false obstacleId : -1 join the fight at {Point3 : (1, -16, 0)}";
const DEGATS: &str = " INFO 20:34:45,200 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Sac à patates: -10 025 PV (Feu)";
/// Le joueur marche sur le pacte : ouvre la fenêtre d'extraction.
const WALKON: &str = " INFO 20:35:10,412 [AWT-EventQueue-0] (bBq:74) - Action [WALKON] performed on interactive element : 3186212";
const EXTRAIT_1: &str = " INFO 20:35:52,903 [AWT-EventQueue-0] (aPV:174) - [Information (jeu)] Vous avez ramassé 5x Plâjeton .";
const EXTRAIT_2: &str = " INFO 20:35:53,102 [AWT-EventQueue-0] (aPV:174) - [Information (jeu)] Vous avez ramassé 1x Bottes Lantha .";
/// Même objet que `EXTRAIT_1`, ramassé une seconde fois dans la même fenêtre : une seule ligne
/// d'objet en sortie, quantités cumulées (miroir d'`addToPactBatch`).
const EXTRAIT_3: &str = " INFO 20:35:54,010 [AWT-EventQueue-0] (aPV:174) - [Information (jeu)] Vous avez ramassé 3x Plâjeton .";
/// Plus de 3 minutes après le dernier objet du lot (`PACT_EXTRACTION_WINDOW_MS`) : cette ligne
/// ferme la fenêtre, et ne rejoint donc jamais le lot qu'elle clôt.
const APRES_FENETRE: &str =
    " INFO 20:39:20,000 [AWT-EventQueue-0] (aWF:91) - [FIGHT] End fight with id 1552060439";
/// Ramassage de combat ordinaire, sans aucun WALKON avant lui.
const BUTIN_DE_COMBAT: &str = " INFO 20:34:50,903 [AWT-EventQueue-0] (aPV:174) - [Information (jeu)] Vous avez ramassé 5x Plâjeton .";

/// Voir la doc d'`Engine::with_stores` : jamais `Engine::new()` dans un test d'intégration, il
/// écrirait dans les vrais fichiers de production.
fn moteur(nom: &str) -> (Engine, std::path::PathBuf) {
    let store_dir = std::env::temp_dir().join(format!(
        "wakfu-overlay-engine-test-pacte-{nom}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&store_dir);
    let engine = Engine::with_stores(
        store_dir.join("watchlist-counts.json"),
        store_dir.join("fights"),
    )
    .expect("création de l'Engine");
    (engine, store_dir)
}

fn lot(lines: &[&str]) -> LineBatch {
    LineBatch {
        lines: lines.iter().map(|l| (*l).to_string()).collect(),
        is_initial_load: false,
    }
}

/// Butin tel qu'il partira vers le compte pour ce combat — le seul endroit où il vit (voir
/// `FightWorking::loot`, interne à `SessionState` jusqu'au `CombatEnd`).
fn butin_du_combat(evenements: &[SyncEvent]) -> Vec<(Option<&str>, i64)> {
    let combat = evenements
        .iter()
        .find(|event| event.kind == HistoryEventKind::Fight)
        .expect("le combat terminé produit un événement d'historique");
    let HistoryPayload::Fight(payload) = &combat.payload else {
        panic!("charge utile de combat attendue");
    };
    payload
        .loot
        .iter()
        .map(|item| (item.item_name.as_deref(), item.quantity))
        .collect()
}

#[test]
fn le_butin_extrait_du_pacte_ne_compte_pas_pour_le_combat_et_part_en_extraction() {
    let (mut engine, store_dir) = moteur("extraction");
    engine
        .ingest_batch(&lot(&[
            CREATION,
            MONSTRE_REJOINT,
            ALLIE_REJOINT,
            DEGATS,
            WALKON,
            EXTRAIT_1,
            EXTRAIT_2,
            EXTRAIT_3,
        ]))
        .expect("ingestion du combat et de l'extraction");

    assert!(
        engine
            .drain_sync_events()
            .iter()
            .all(|event| event.kind != HistoryEventKind::Pact),
        "la fenêtre est encore ouverte : rien n'est committé tant qu'aucune ligne ne l'a dépassée"
    );

    // Une ligne postérieure de plus de 3 minutes ferme la fenêtre et committe le lot — ici la fin
    // du combat, qui livre du même coup le butin retenu pour ce combat.
    engine
        .ingest_batch(&lot(&[APRES_FENETRE]))
        .expect("ingestion de la ligne qui ferme la fenêtre");
    let evenements = engine.drain_sync_events();
    assert!(
        butin_du_combat(&evenements).is_empty(),
        "un objet extrait d'un pacte n'est jamais du butin du combat ouvert au même moment"
    );
    let extraction = evenements
        .iter()
        .find(|event| event.kind == HistoryEventKind::Pact)
        .expect("une extraction de pacte est prête à synchroniser");

    assert_eq!(
        extraction.signature, "20:35:52,903|bottes lanthax1,plâjetonx8",
        "signature datée du PREMIER objet du lot, objets normalisés/triés, quantités cumulées \
         (miroir de pactExtractionSignature)"
    );
    assert_eq!(extraction.id(), format!("pact:{}", extraction.signature));

    let HistoryPayload::PactExtraction(payload) = &extraction.payload else {
        panic!("charge utile d'extraction de pacte attendue");
    };
    let lignes: Vec<(Option<&str>, i64)> = payload
        .items
        .iter()
        .map(|item| (item.item_name.as_deref(), item.quantity))
        .collect();
    assert_eq!(
        lignes,
        vec![(Some("Plâjeton"), 8), (Some("Bottes Lantha"), 1)],
        "ordre de ramassage conservé, doublon fusionné"
    );

    let _ = std::fs::remove_dir_all(&store_dir);
}

#[test]
fn un_ramassage_sans_walkon_reste_du_butin_de_combat() {
    let (mut engine, store_dir) = moteur("sans-walkon");
    engine
        .ingest_batch(&lot(&[
            CREATION,
            MONSTRE_REJOINT,
            ALLIE_REJOINT,
            DEGATS,
            BUTIN_DE_COMBAT,
            APRES_FENETRE,
        ]))
        .expect("ingestion du combat");

    let evenements = engine.drain_sync_events();
    assert_eq!(
        butin_du_combat(&evenements),
        vec![(Some("Plâjeton"), 5)],
        "sans WALKON, rien ne change au butin de combat"
    );
    assert!(
        evenements
            .iter()
            .all(|event| event.kind != HistoryEventKind::Pact),
        "aucune extraction de pacte n'est inventée"
    );

    let _ = std::fs::remove_dir_all(&store_dir);
}

/// La fenêtre expirée, un ramassage redevient du butin de combat ordinaire — sans quoi un seul
/// WALKON contaminerait tout le reste de la session.
#[test]
fn la_fenetre_expiree_rend_les_ramassages_suivants_au_combat() {
    let (mut engine, store_dir) = moteur("expiration");
    const TARDIF: &str = " INFO 20:39:25,300 [AWT-EventQueue-0] (aPV:174) - [Information (jeu)] Vous avez ramassé 2x Ceinture feuillue .";
    const FIN: &str =
        " INFO 20:39:30,000 [AWT-EventQueue-0] (aWF:91) - [FIGHT] End fight with id 1552060439";
    engine
        .ingest_batch(&lot(&[
            CREATION,
            MONSTRE_REJOINT,
            ALLIE_REJOINT,
            WALKON,
            EXTRAIT_1,
            TARDIF,
            FIN,
        ]))
        .expect("ingestion");

    let evenements = engine.drain_sync_events();
    assert_eq!(
        butin_du_combat(&evenements),
        vec![(Some("Ceinture feuillue"), 2)],
        "le ramassage postérieur à la fenêtre revient au combat"
    );
    let extraction = evenements
        .iter()
        .find(|event| event.kind == HistoryEventKind::Pact)
        .expect("le lot en attente est committé par la ligne qui dépasse la fenêtre");
    assert_eq!(extraction.signature, "20:35:52,903|plâjetonx5");

    let _ = std::fs::remove_dir_all(&store_dir);
}
