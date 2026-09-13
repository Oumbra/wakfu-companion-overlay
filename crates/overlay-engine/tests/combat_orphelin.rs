//! Régression réelle (retour utilisateur, 2026-09-13) : le client Wakfu fermé en plein combat
//! d'entraînement (Sac à patates), puis relancé — la ligne `[FIGHT] End fight` de ce combat n'a
//! jamais été écrite. Le moteur le gardait `ongoing` pour toujours, `fight_store` le persistait
//! et le restaurait à chaque redémarrage, et `fight_for_character` le préférait à tous les combats
//! suivants : panneau Combat affiché en permanence malgré « masqué hors combat », et figé sur ce
//! combat fantôme pendant que les vrais combats défilaient. Voir `SessionState::abandon_fight`.
//!
//! Lignes AUTHENTIQUES du `wakfu.log` de l'utilisateur (mêmes identifiants de combat, mêmes
//! horaires), seulement élaguées de ce qui ne compte pas ici.

use overlay_engine::{Engine, FightSnapshot};
use overlay_ingest::LineBatch;

const CREATION: &str = " INFO 22:38:02,958 [AWT-EventQueue-0] (aXI:47) - CREATION DU COMBAT";
const SAC_REJOINT_A: &str = " INFO 22:38:02,971 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=1552060439 Sac à patates breed : 2335 [-1706442036778922] isControlledByAI=true obstacleId : -1 join the fight at {Point3 : (0, -14, 0)}";
const OUMBRA_REJOINT_A: &str = " INFO 22:38:03,046 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=1552060439 Oumbra breed : 4 [11039330] isControlledByAI=false obstacleId : -1 join the fight at {Point3 : (1, -16, 0)}";
const SORT_A: &str = " INFO 22:38:44,748 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Oumbra lance le sort Fourberie (Critiques)";
const DEGATS_A: &str = " INFO 22:38:45,200 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Sac à patates: -10 025 PV (Feu)";
/// Fermeture de la fenêtre du client, combat encore en cours.
const DECONNEXION: &str = " INFO 22:40:59,637 [AWT-EventQueue-0] (aVv:664) - Sending DisconnectionMessage to Servers. Reason : {UI Closed}";
/// Bannière de démarrage du client relancé.
const DEMARRAGE: &str = " INFO 22:43:40,781 [main] (com.ankamagames.wakfu.client.WakfuClient:284) - Configuration loaded for region WESTERN (by country detection for null): config.properties";
/// Les mêmes lignes rejouées par le client RECONNECTÉ dans le combat A — horaires de la
/// reconnexion, comme dans un vrai log (le parser écarte les répétitions strictes d'une ligne).
const SAC_REVIENT_A: &str = " INFO 22:43:52,101 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=1552060439 Sac à patates breed : 2335 [-1706442036778922] isControlledByAI=true obstacleId : -1 join the fight at {Point3 : (0, -14, 0)}";
const OUMBRA_REVIENT_A: &str = " INFO 22:43:52,140 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=1552060439 Oumbra breed : 4 [11039330] isControlledByAI=false obstacleId : -1 join the fight at {Point3 : (1, -16, 0)}";
const SORT_A2: &str = " INFO 22:43:58,302 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Oumbra lance le sort Fourberie (Critiques)";
const DEGATS_A2: &str = " INFO 22:43:58,760 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Sac à patates: -10 025 PV (Feu)";
const CREATION_B: &str = " INFO 22:44:46,905 [AWT-EventQueue-0] (aXI:47) - CREATION DU COMBAT";
const SAC_REJOINT_B: &str = " INFO 22:44:46,910 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=1552060583 Sac à patates breed : 2335 [-1706442036761254] isControlledByAI=true obstacleId : -1 join the fight at {Point3 : (0, -14, 0)}";
const OUMBRA_REJOINT_B: &str = " INFO 22:44:46,924 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=1552060583 Oumbra breed : 4 [11039330] isControlledByAI=false obstacleId : -1 join the fight at {Point3 : (0, -15, 0)}";
const SORT_B: &str = " INFO 22:44:50,503 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Oumbra lance le sort Kleptosram (Critiques)";
const DEGATS_B: &str = " INFO 22:44:50,947 [AWT-EventQueue-0] (aPV:174) - [Information (combat)] Sac à patates: -10 018 PV (Eau)";
const FIN_B: &str =
    " INFO 22:45:02,030 [AWT-EventQueue-0] (aWF:91) - [FIGHT] End fight with id 1552060583";

const COMBAT_A: i64 = 1552060439;
const COMBAT_B: i64 = 1552060583;

/// Un moteur sur des chemins de stockage temporaires dédiés — JAMAIS `Engine::new()` dans un test
/// d'intégration (voir la doc d'`Engine::with_stores` : il écrirait dans les VRAIS fichiers de
/// production, et restaurerait au prochain lancement réel un combat fictif issu d'un test).
fn moteur(nom: &str) -> (Engine, std::path::PathBuf) {
    let store_dir = std::env::temp_dir().join(format!(
        "wakfu-overlay-engine-test-combat-orphelin-{nom}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&store_dir);
    let fight_store_dir = store_dir.join("fights");
    let engine = Engine::with_stores(store_dir.join("watchlist-counts.json"), fight_store_dir)
        .expect("création de l'Engine");
    (engine, store_dir)
}

fn lot(lines: &[&str], is_initial_load: bool) -> LineBatch {
    LineBatch {
        lines: lines.iter().map(|l| (*l).to_string()).collect(),
        is_initial_load,
    }
}

fn combat(engine: &Engine, fight_id: i64) -> FightSnapshot {
    engine
        .snapshot()
        .fights
        .into_iter()
        .find(|f| f.fight_id == fight_id)
        .unwrap_or_else(|| panic!("combat {fight_id} absent de l'instantané"))
}

fn fichier_persiste(store_dir: &std::path::Path, fight_id: i64) -> bool {
    store_dir
        .join("fights")
        .join(format!("fight-{fight_id}.json"))
        .exists()
}

#[test]
fn le_client_ferme_en_combat_termine_le_combat_et_son_fichier() {
    let (mut engine, store_dir) = moteur("deconnexion");
    engine
        .ingest_batch(&lot(
            &[CREATION, SAC_REJOINT_A, OUMBRA_REJOINT_A, SORT_A, DEGATS_A],
            false,
        ))
        .expect("ingestion du combat A");
    assert!(
        combat(&engine, COMBAT_A).ongoing,
        "combat A en cours avant la coupure"
    );
    assert!(
        fichier_persiste(&store_dir, COMBAT_A),
        "un combat en cours est persisté pour survivre à un redémarrage"
    );

    engine
        .ingest_batch(&lot(&[DECONNEXION], false))
        .expect("ingestion de la déconnexion");
    let a = combat(&engine, COMBAT_A);
    assert!(
        !a.ongoing,
        "le client fermé, le combat A n'est plus en cours"
    );
    assert_eq!(a.result, None, "ni gagné ni perdu : abandonné");
    assert_eq!(
        a.fighters
            .iter()
            .find(|f| f.name == "Oumbra")
            .map(|f| f.total_damage),
        Some(10_025),
        "les totaux déjà comptés restent affichables"
    );
    assert!(
        !fichier_persiste(&store_dir, COMBAT_A),
        "un combat abandonné ne doit plus être restauré au prochain démarrage"
    );
    let _ = std::fs::remove_dir_all(&store_dir);
}

#[test]
fn le_combat_fantome_restaure_au_demarrage_est_termine_par_le_rattrapage() {
    // La session incriminée, telle quelle : un premier processus persiste le combat A en cours,
    // puis l'overlay redémarre APRÈS que le client a été fermé et relancé — `fight-A.json` est
    // restauré, et le rattrapage de `wakfu.log` (lot initial) rejoue la coupure.
    let (mut premier, store_dir) = moteur("redemarrage");
    premier
        .ingest_batch(&lot(
            &[CREATION, SAC_REJOINT_A, OUMBRA_REJOINT_A, SORT_A, DEGATS_A],
            false,
        ))
        .expect("ingestion du combat A");
    assert!(fichier_persiste(&store_dir, COMBAT_A));
    drop(premier);

    let mut second = Engine::with_stores(
        store_dir.join("watchlist-counts.json"),
        store_dir.join("fights"),
    )
    .expect("création du second Engine");
    assert!(
        combat(&second, COMBAT_A).ongoing,
        "restauré tel qu'il a été persisté : encore en cours"
    );
    second
        .ingest_batch(&lot(
            &[
                CREATION,
                SAC_REJOINT_A,
                OUMBRA_REJOINT_A,
                SORT_A,
                DEGATS_A,
                DECONNEXION,
                DEMARRAGE,
            ],
            true,
        ))
        .expect("rattrapage");
    let a = combat(&second, COMBAT_A);
    assert!(
        !a.ongoing,
        "le rattrapage a vu la coupure : combat A terminé"
    );
    assert_eq!(
        a.fighters.len(),
        2,
        "les jointures rejouées n'ont pas dupliqué les combattants restaurés"
    );
    assert!(!fichier_persiste(&store_dir, COMBAT_A));
    assert!(
        second
            .snapshot()
            .fight_for_character("Oumbra")
            .is_some_and(|f| !f.ongoing),
        "plus aucun combat en cours pour le personnage : panneau masqué hors combat"
    );
    let _ = std::fs::remove_dir_all(&store_dir);
}

#[test]
fn rejoindre_un_autre_combat_abandonne_le_precedent_meme_sans_coupure() {
    // Même incident sans aucune ligne de coupure lisible (plantage sans bannière, ligne perdue
    // dans une rotation…) : le personnage qui rejoint le combat B ne peut plus être dans A.
    let (mut engine, store_dir) = moteur("autre-combat");
    engine
        .ingest_batch(&lot(
            &[CREATION, SAC_REJOINT_A, OUMBRA_REJOINT_A, SORT_A, DEGATS_A],
            false,
        ))
        .expect("ingestion du combat A");
    engine
        .ingest_batch(&lot(
            &[
                CREATION_B,
                SAC_REJOINT_B,
                OUMBRA_REJOINT_B,
                SORT_B,
                DEGATS_B,
            ],
            false,
        ))
        .expect("ingestion du combat B");

    assert!(
        !combat(&engine, COMBAT_A).ongoing,
        "A abandonné dès que B est rejoint"
    );
    assert!(combat(&engine, COMBAT_B).ongoing);
    let affiche = engine
        .snapshot()
        .fight_for_character("Oumbra")
        .expect("un combat pour Oumbra")
        .clone();
    assert_eq!(
        affiche.fight_id, COMBAT_B,
        "le panneau suit le combat réellement en cours"
    );
    assert!(!fichier_persiste(&store_dir, COMBAT_A));
    assert!(fichier_persiste(&store_dir, COMBAT_B));

    engine
        .ingest_batch(&lot(&[FIN_B], false))
        .expect("fin du combat B");
    assert!(!combat(&engine, COMBAT_B).ongoing);
    assert!(!fichier_persiste(&store_dir, COMBAT_B));
    let _ = std::fs::remove_dir_all(&store_dir);
}

#[test]
fn un_combat_suspendu_reprend_sans_doublon_si_le_personnage_y_revient() {
    // Déconnexion PENDANT un combat, reconnexion dans le même : Wakfu rejoue les lignes `[_FL_]`
    // du combat. Il doit rouvrir, totaux compris, sans que personne n'y figure deux fois.
    let (mut engine, store_dir) = moteur("reconnexion");
    engine
        .ingest_batch(&lot(
            &[CREATION, SAC_REJOINT_A, OUMBRA_REJOINT_A, SORT_A, DEGATS_A],
            false,
        ))
        .expect("ingestion du combat A");
    engine
        .ingest_batch(&lot(&[DECONNEXION], false))
        .expect("déconnexion");
    assert!(!combat(&engine, COMBAT_A).ongoing);

    engine
        .ingest_batch(&lot(
            &[SAC_REVIENT_A, OUMBRA_REVIENT_A, SORT_A2, DEGATS_A2],
            false,
        ))
        .expect("reconnexion dans le combat A");
    let a = combat(&engine, COMBAT_A);
    assert!(a.ongoing, "le combat suspendu est rouvert");
    assert_eq!(
        a.fighters.len(),
        2,
        "aucun combattant dupliqué par les jointures rejouées"
    );
    assert_eq!(
        a.fighters
            .iter()
            .find(|f| f.name == "Oumbra")
            .map(|f| f.total_damage),
        Some(20_050),
        "les dégâts d'avant et d'après la coupure s'additionnent"
    );
    assert!(
        fichier_persiste(&store_dir, COMBAT_A),
        "de nouveau en cours, de nouveau persisté"
    );
    let _ = std::fs::remove_dir_all(&store_dir);
}
