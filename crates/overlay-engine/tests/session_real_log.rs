//! Tests d'intégration sur le vrai `wakfu.log` vendu depuis `wakfu-companion` (même fichier que
//! le spike S2, `spikes/s2-engine-quickjs/tests/wakfu.log`, 10 975 lignes) — pas un fichier
//! synthétique : on veut vérifier que `Engine` produit un récap plausible sur de vraies données de
//! jeu, pas juste que le code s'exécute sans paniquer.

use std::fs;

use overlay_engine::Engine;
use overlay_ingest::{LineBatch, Tailer};

const WAKFU_LOG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/wakfu.log");

#[test]
fn ingest_vrai_wakfu_log_produit_un_recap_plausible() {
    let mut tailer = Tailer::new(WAKFU_LOG);
    let mut engine = Engine::new().expect("création de l'Engine");

    let mut total_entries = 0usize;
    loop {
        let batches = tailer.poll().expect("poll() du tailer");
        if batches.is_empty() {
            break;
        }
        for batch in &batches {
            let entries = engine.ingest_batch(batch).expect("ingestion d'un lot");
            total_entries += entries.len();
        }
    }

    assert!(
        total_entries > 1000,
        "volume d'événements attendu significatif, obtenu {total_entries}"
    );

    let snapshot = engine.snapshot();
    assert!(
        snapshot.totals.fights_won + snapshot.totals.fights_lost > 0,
        "aucun combat détecté sur un vrai log de combat"
    );
    assert!(snapshot.totals.xp_gained > 0, "aucun gain d'XP détecté");

    let fight = snapshot
        .fights
        .last()
        .expect("aucun combat courant/dernier après un vrai log de combat");
    assert!(
        !fight.fighters.is_empty(),
        "le dernier combat n'a aucun combattant enregistré"
    );
    assert!(
        fight.fighters.iter().any(|f| f.total_damage > 0),
        "aucun dégât enregistré dans le dernier combat"
    );
}

/// Garde-fou pour la subtilité documentée dans `Engine::ingest_batch` : un rattrapage qui
/// s'étale sur plusieurs `LineBatch` (à cause de `MAX_BATCH_LINES`, voir `overlay-ingest`) ne doit
/// réinitialiser le parser/l'état de session qu'une seule fois, au tout premier lot — jamais entre
/// deux lots du même rattrapage. Vérifié en comparant le récap obtenu en un seul lot contre celui
/// obtenu en deux, sur le même contenu réel.
#[test]
fn rattrapage_en_plusieurs_lots_donne_le_meme_recap_quun_lot_unique() {
    let content = fs::read_to_string(WAKFU_LOG).unwrap();
    let lines: Vec<String> = content.lines().map(str::to_string).collect();
    let split_at = lines.len() / 2;

    let mut engine_single = Engine::new().unwrap();
    let batch_single = LineBatch {
        lines: lines.clone(),
        is_initial_load: true,
    };
    engine_single.ingest_batch(&batch_single).unwrap();
    let snapshot_single = engine_single.snapshot();

    let mut engine_split = Engine::new().unwrap();
    let batch1 = LineBatch {
        lines: lines[..split_at].to_vec(),
        is_initial_load: true,
    };
    let batch2 = LineBatch {
        lines: lines[split_at..].to_vec(),
        is_initial_load: true,
    };
    engine_split.ingest_batch(&batch1).unwrap();
    engine_split.ingest_batch(&batch2).unwrap();
    let snapshot_split = engine_split.snapshot();

    assert_eq!(
        snapshot_single.totals, snapshot_split.totals,
        "un rattrapage découpé en plusieurs lots ne doit pas changer le récap final"
    );
}

/// Une ligne ajoutée après le rattrapage initial doit se comporter comme une vraie reprise en
/// direct : le parser ne doit **pas** être réinitialisé (sinon toute correlation de combat en
/// cours serait perdue à chaque nouvelle ligne).
#[test]
fn lot_en_direct_apres_rattrapage_ne_reinitialise_pas_letat() {
    let content = fs::read_to_string(WAKFU_LOG).unwrap();
    let lines: Vec<String> = content.lines().map(str::to_string).collect();

    let mut engine = Engine::new().unwrap();
    engine
        .ingest_batch(&LineBatch {
            lines,
            is_initial_load: true,
        })
        .unwrap();
    let before = engine.snapshot();

    // Lot en direct vide de sens métier (ne doit rien changer aux totaux), mais surtout ne doit
    // pas effacer le combat en cours enregistré par le rattrapage précédent.
    engine
        .ingest_batch(&LineBatch {
            lines: vec![" INFO 23:59:59,999 [x] (y:1) - ligne sans rapport".to_string()],
            is_initial_load: false,
        })
        .unwrap();
    let after = engine.snapshot();

    assert_eq!(
        before.totals, after.totals,
        "un lot en direct sans nouvel événement ne doit pas changer le récap"
    );
    assert_eq!(
        before.fights, after.fights,
        "le combat en cours ne doit pas être perdu"
    );
}

/// Régression réelle (retour utilisateur 2026-09-02, vidéo à l'appui) : `wakfu.log` peut être
/// remplacé par un fichier neuf **en cours de partie** (rotation) — `overlay_ingest::tailer` le
/// détecte et relit depuis le début avec `is_initial_load: true`, EXACTEMENT le même signal qu'un
/// tout premier lancement. Le fichier rotaté ne rejoue PAS l'historique déjà lu (vérifié en
/// conditions réelles : le combat était toujours en cours en jeu, mais le panneau Combat a perdu
/// tous ses alliés/ennemis pile au moment de la rotation). Un deuxième rattrapage (`is_initial_
/// load: true`) survenant APRÈS que l'Engine ait déjà un vécu de session ne doit donc jamais
/// effacer ce vécu, contrairement au tout premier.
#[test]
fn rotation_en_cours_de_session_ne_perd_pas_le_combat_en_cours() {
    let content = fs::read_to_string(WAKFU_LOG).unwrap();
    let lines: Vec<String> = content.lines().map(str::to_string).collect();
    let split_at = lines.len() / 2;

    let mut engine = Engine::new().unwrap();
    engine
        .ingest_batch(&LineBatch {
            lines: lines[..split_at].to_vec(),
            is_initial_load: true,
        })
        .unwrap();
    let before = engine.snapshot();
    assert!(
        !before.fights.is_empty(),
        "précondition : au moins un combat après le premier rattrapage"
    );

    // Rotation en cours de session : `is_initial_load: true` de nouveau, mais le fichier rotaté
    // (simulé ici) ne contient qu'une poignée de nouvelles lignes sans rapport — PAS un rejeu de
    // l'historique déjà lu (comportement réel observé de Wakfu).
    engine
        .ingest_batch(&LineBatch {
            lines: vec![" INFO 23:59:59,999 [x] (y:1) - ligne sans rapport".to_string()],
            is_initial_load: true,
        })
        .unwrap();
    let after = engine.snapshot();

    assert_eq!(
        before.fights, after.fights,
        "une rotation en cours de session ne doit jamais effacer un combat déjà suivi"
    );
}
