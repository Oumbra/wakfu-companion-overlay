//! Tests d'intégration de `Tailer::poll` — rejeu, rotation, troncature (critère de sortie de
//! L1, docs/plan-architecture.md §12). Volontairement synchrones : on pilote `poll()` à la main
//! plutôt que de dépendre d'événements `notify` réels, non déterministes en CI.

use std::fs;
use std::io::Write;
use std::path::Path;

use overlay_ingest::{Tailer, MAX_BATCH_LINES};

fn write_new(path: &Path, contents: &str) {
    fs::write(path, contents.as_bytes()).unwrap();
}

fn append(path: &Path, contents: &str) {
    let mut f = fs::OpenOptions::new().append(true).open(path).unwrap();
    f.write_all(contents.as_bytes()).unwrap();
}

#[test]
fn rejeu_simple_puis_rattrapage_du_direct() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    write_new(&path, "ligne 1\nligne 2\n");

    let mut tailer = Tailer::new(&path);

    let batches = tailer.poll().unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].lines, vec!["ligne 1", "ligne 2"]);
    assert!(
        batches[0].is_initial_load,
        "premier lot = rattrapage initial"
    );

    // Rien de neuf : lot vide, pas d'erreur.
    assert!(tailer.poll().unwrap().is_empty());

    // Nouvelle ligne, après avoir rattrapé le direct : is_initial_load doit retomber à false.
    append(&path, "ligne 3\n");
    let batches = tailer.poll().unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].lines, vec!["ligne 3"]);
    assert!(
        !batches[0].is_initial_load,
        "lot en direct, plus du rattrapage"
    );
}

#[test]
fn ligne_partielle_non_transmise_avant_completion() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    write_new(&path, "ligne complete\nreliquat sans retour");

    let mut tailer = Tailer::new(&path);
    let batches = tailer.poll().unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].lines, vec!["ligne complete"]);

    append(&path, " a la ligne\n");
    let batches = tailer.poll().unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].lines, vec!["reliquat sans retour a la ligne"]);
}

#[test]
fn fin_de_ligne_crlf_geree_comme_lf() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    write_new(&path, "ligne windows\r\nligne unix\n");

    let mut tailer = Tailer::new(&path);
    let batches = tailer.poll().unwrap();
    assert_eq!(batches[0].lines, vec!["ligne windows", "ligne unix"]);
}

#[test]
fn troncature_meme_fichier_relit_depuis_zero() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    write_new(&path, "avant troncature, une longue ligne\n");

    let mut tailer = Tailer::new(&path);
    tailer.poll().unwrap();
    assert!(tailer.poll().unwrap().is_empty()); // rattrapé

    // Troncature en place (même fichier physique, contenu plus court) : simule un log qui
    // recommencerait à zéro sans que le fichier soit recréé.
    write_new(&path, "court\n");
    let batches = tailer.poll().unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].lines, vec!["court"]);
    assert!(
        batches[0].is_initial_load,
        "troncature = nouveau rattrapage"
    );
}

#[test]
fn rotation_nouveau_fichier_meme_chemin_relit_depuis_zero() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    write_new(&path, "session precedente\n");

    let mut tailer = Tailer::new(&path);
    tailer.poll().unwrap();
    assert!(tailer.poll().unwrap().is_empty());

    // Rotation réelle : suppression puis recréation (nouvelle identité), comme le ferait un outil
    // de rotation de logs — la seule taille ne suffirait pas à la détecter si le nouveau contenu
    // est plus long que l'ancien (c'est justement le cas testé ici).
    fs::remove_file(&path).unwrap();
    write_new(
        &path,
        "nouvelle session, plus longue que l'ancienne ligne precedente\n",
    );

    let batches = tailer.poll().unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(
        batches[0].lines,
        vec!["nouvelle session, plus longue que l'ancienne ligne precedente"]
    );
    assert!(batches[0].is_initial_load, "rotation = nouveau rattrapage");
}

#[test]
fn fichier_absent_ne_produit_pas_derreur() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent.log");
    let mut tailer = Tailer::new(&path);
    assert!(tailer.poll().unwrap().is_empty());
}

#[test]
fn gros_volume_decoupe_en_lots_bornes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");

    let n = MAX_BATCH_LINES * 2 + 137;
    let mut content = String::new();
    for i in 0..n {
        content.push_str(&format!("ligne {i}\n"));
    }
    write_new(&path, &content);

    let mut tailer = Tailer::new(&path);
    let batches = tailer.poll().unwrap();

    let total: usize = batches.iter().map(|b| b.lines.len()).sum();
    assert_eq!(total, n);
    assert!(batches.iter().all(|b| b.lines.len() <= MAX_BATCH_LINES));
    assert!(batches.iter().all(|b| b.is_initial_load));
    assert_eq!(batches.first().unwrap().lines.first().unwrap(), "ligne 0");
    assert_eq!(
        batches.last().unwrap().lines.last().unwrap(),
        &format!("ligne {}", n - 1)
    );
}

#[test]
fn encodage_invalide_ne_bloque_pas_lingestion() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    let mut bytes = b"ligne valide\n".to_vec();
    bytes.extend_from_slice(&[0xFF, 0xFE, b'\n']); // octets non-UTF-8 valides
    bytes.extend_from_slice(b"ligne suivante\n");
    fs::write(&path, &bytes).unwrap();

    let mut tailer = Tailer::new(&path);
    let batches = tailer.poll().unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].lines.len(), 3);
    assert_eq!(batches[0].lines[0], "ligne valide");
    assert_eq!(batches[0].lines[2], "ligne suivante");
    // La ligne corrompue est remplacée (U+FFFD), pas perdue ni fatale pour l'ingestion.
    assert!(batches[0].lines[1].contains('\u{FFFD}'));
}
