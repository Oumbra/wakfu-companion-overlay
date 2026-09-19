//! Parité du parseur vendu (`engine-js/`) avec celui du site : ce fichier couvre les trois
//! signaux portés le 2026-09-19 depuis `wakfu-companion` — cycle de vie du client
//! (`client-lifecycle`), perte d'objet (`item-loss`, signature du démantèlement) et fermeture de
//! session marchand/HDV par « On annule » —, plus l'oubli du combat fantôme par le parser
//! (`LogParser.closeFight`) quand l'hôte suspend un combat sur une coupure du client.
//!
//! Lignes calquées sur celles du vrai `wakfu.log` de `tests/wakfu.log`.

use overlay_engine::{ClientLifecycleEvent, Engine, HistoryEventKind, HistoryPayload, LogEntry};
use overlay_ingest::LineBatch;

const CREATION: &str = " INFO 20:34:02,833 [AWT-EventQueue-0] (aXI:47) - CREATION DU COMBAT";
const MONSTRE_REJOINT: &str = " INFO 20:34:02,971 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=1552060439 Sac à patates breed : 2335 [-1706442036778922] isControlledByAI=true obstacleId : -1 join the fight at {Point3 : (0, -14, 0)}";
const ALLIE_REJOINT: &str = " INFO 20:34:03,046 [AWT-EventQueue-0] (faw:1405) - [_FL_] fightId=1552060439 Oumbra breed : 4 [90000007] isControlledByAI=false obstacleId : -1 join the fight at {Point3 : (1, -16, 0)}";
const ARRET_CLIENT: &str = " INFO 20:34:20,100 [AWT-EventQueue-0] (cFw:27) - Stopping cFC...";
const DEMARRAGE_CLIENT: &str = " INFO 20:30:03,892 [AWT-EventQueue-0] (cFw:27) - Starting cFC...";
const OCCUPATION_MARKET_DEBUT: &str = " INFO 20:33:04,638 [AWT-EventQueue-0] (bmI:41) - Lancement de l'occupation MARKET sur la board [bDk id=31546]{Point3 : (-12, 27, -57)}";
/// Variante de fermeture ajoutée côté web le 2026-09-15 (interruption côté serveur) — celle que
/// le parseur vendu ne reconnaissait pas.
const OCCUPATION_MARKET_ANNULEE: &str = " INFO 20:33:44,200 [AWT-EventQueue-0] (bmI:41) - On annule l'occupation MARKET sur la board [bDk id=31546] (fromServer=true, sendMessage=false)";
const OCCUPATION_MARKET_ARRETEE: &str = " INFO 20:33:50,000 [AWT-EventQueue-0] (bmI:41) - On arrête l'occupation MARKET sur la board [bDk id=31546]";
/// Seconde ouverture — horodatage distinct du premier, sans quoi le parser y verrait la
/// répétition d'une ligne déjà vue (dédoublonnage multi-compte, `DEDUPE_WINDOW_MS`).
const OCCUPATION_MARKET_DEBUT_2: &str = " INFO 20:33:47,000 [AWT-EventQueue-0] (bmI:41) - Lancement de l'occupation MARKET sur la board [bDk id=31546]{Point3 : (-12, 27, -57)}";
/// Cycle de démantèlement : la perte d'objet, puis le ramassage des ressources qu'elle produit.
const PERTE_OBJET: &str = " INFO 20:34:40,000 [AWT-EventQueue-0] (aPV:174) - [Information (jeu)] Vous avez perdu 1x Bottes Lantha .";
const RESSOURCES_RECUPEREES: &str = " INFO 20:34:40,400 [AWT-EventQueue-0] (aPV:174) - [Information (jeu)] Vous avez ramassé 5x Plâjeton .";
/// Même ramassage, mais bien après la perte d'objet (au-delà de PURCHASE_WINDOW_MS) : sans
/// rapport avec elle, donc du butin de combat ordinaire.
const RESSOURCES_TARDIVES: &str = " INFO 20:34:45,000 [AWT-EventQueue-0] (aPV:174) - [Information (jeu)] Vous avez ramassé 5x Plâjeton .";
const FIN_COMBAT: &str =
    " INFO 20:34:50,000 [AWT-EventQueue-0] (aWF:91) - [FIGHT] End fight with id 1552060439";
/// Butin hors de tout combat connu du parser, juste après une coupure : c'est lui qui atterrissait
/// dans le combat fantôme tant que le parser ne l'oubliait pas.
const BUTIN_APRES_COUPURE: &str = " INFO 20:34:30,000 [AWT-EventQueue-0] (aPV:174) - [Information (jeu)] Vous avez ramassé 7x Eclat de Wakfu .";

fn moteur(nom: &str) -> (Engine, std::path::PathBuf) {
    let store_dir = std::env::temp_dir().join(format!(
        "wakfu-overlay-engine-test-parseur-{nom}-{}",
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

fn butin_du_combat(evenements: &[overlay_engine::SyncEvent]) -> Vec<(Option<&str>, i64)> {
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
fn les_lignes_de_cycle_de_vie_du_client_sont_reconnues() {
    let (mut engine, store_dir) = moteur("cycle-de-vie");
    let entrees = engine
        .ingest_batch(&lot(&[DEMARRAGE_CLIENT, ARRET_CLIENT]))
        .expect("ingestion");

    let evenements: Vec<ClientLifecycleEvent> = entrees
        .iter()
        .filter_map(|entry| match entry {
            LogEntry::ClientLifecycle { event, .. } => Some(*event),
            _ => None,
        })
        .collect();
    assert_eq!(
        evenements,
        vec![
            ClientLifecycleEvent::Startup,
            ClientLifecycleEvent::Shutdown
        ],
        "« Starting cFC... » et « Stopping cFC... » produisent chacun leur événement"
    );

    let _ = std::fs::remove_dir_all(&store_dir);
}

/// Les deux formes de fermeture sont terminales — « On annule » (interruption côté serveur) n'était
/// pas reconnue avant ce portage, et laissait la session marchand ouverte pour toujours.
#[test]
fn les_deux_fermetures_de_session_marchand_sont_reconnues() {
    let (mut engine, store_dir) = moteur("marchand");
    let entrees = engine
        .ingest_batch(&lot(&[
            OCCUPATION_MARKET_DEBUT,
            OCCUPATION_MARKET_ANNULEE,
            OCCUPATION_MARKET_DEBUT_2,
            OCCUPATION_MARKET_ARRETEE,
        ]))
        .expect("ingestion");

    let etats: Vec<bool> = entrees
        .iter()
        .filter_map(|entry| match entry {
            LogEntry::MarketOccupation { active, .. } => Some(*active),
            _ => None,
        })
        .collect();
    assert_eq!(etats, vec![true, false, true, false]);

    let _ = std::fs::remove_dir_all(&store_dir);
}

#[test]
fn le_butin_d_un_demantelement_ne_compte_pas_pour_le_combat() {
    let (mut engine, store_dir) = moteur("demantelement");
    engine
        .ingest_batch(&lot(&[
            CREATION,
            MONSTRE_REJOINT,
            ALLIE_REJOINT,
            PERTE_OBJET,
            RESSOURCES_RECUPEREES,
            FIN_COMBAT,
        ]))
        .expect("ingestion");

    let evenements = engine.drain_sync_events();
    assert!(
        butin_du_combat(&evenements).is_empty(),
        "les ressources d'un objet démantelé ne sont pas du butin de combat"
    );

    let _ = std::fs::remove_dir_all(&store_dir);
}

#[test]
fn un_ramassage_hors_fenetre_de_demantelement_reste_du_butin() {
    let (mut engine, store_dir) = moteur("demantelement-tardif");
    engine
        .ingest_batch(&lot(&[
            CREATION,
            MONSTRE_REJOINT,
            ALLIE_REJOINT,
            PERTE_OBJET,
            RESSOURCES_TARDIVES,
            FIN_COMBAT,
        ]))
        .expect("ingestion");

    let evenements = engine.drain_sync_events();
    assert_eq!(
        butin_du_combat(&evenements),
        vec![(Some("Plâjeton"), 5)],
        "au-delà de la fenêtre, la perte d'objet n'explique plus le ramassage"
    );

    let _ = std::fs::remove_dir_all(&store_dir);
}

/// Le parser doit oublier un combat que l'hôte suspend sur une coupure du client : sinon il
/// continue de le désigner comme seul combat actif, et lui rattache le butin ramassé ensuite.
#[test]
fn le_parser_oublie_le_combat_suspendu_par_une_coupure() {
    let (mut engine, store_dir) = moteur("combat-fantome");
    const DECONNEXION: &str = " INFO 20:34:25,000 [AWT-EventQueue-0] (aVv:664) - Sending DisconnectionMessage to Servers. Reason : {UI Closed}";
    let entrees = engine
        .ingest_batch(&lot(&[
            CREATION,
            MONSTRE_REJOINT,
            ALLIE_REJOINT,
            DECONNEXION,
            BUTIN_APRES_COUPURE,
        ]))
        .expect("ingestion");

    let fight_ids: Vec<Option<i64>> = entrees
        .iter()
        .filter_map(|entry| match entry {
            LogEntry::Loot { fight_id, .. } => Some(*fight_id),
            _ => None,
        })
        .collect();
    assert_eq!(
        fight_ids,
        vec![None],
        "le combat coupé n'est plus le combat actif du parser : ce butin n'appartient à personne"
    );

    let _ = std::fs::remove_dir_all(&store_dir);
}
