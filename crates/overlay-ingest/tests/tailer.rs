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

/// Une ligne incomplète démesurée (fichier corrompu) n'est pas gardée en mémoire entre deux
/// `poll()` : elle est abandonnée, et les lignes suivantes restent lues normalement.
#[test]
fn une_ligne_incomplete_demesuree_est_abandonnee() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    write_new(&path, &"x".repeat(2 * 1024 * 1024));

    let mut tailer = Tailer::new(&path);
    assert!(tailer.poll().unwrap().is_empty());

    append(&path, "fin du fragment\nligne suivante\n");
    let batches = tailer.poll().unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].lines, vec!["fin du fragment", "ligne suivante"]);
}

/// Un gros rattrapage se découpe en temps linéaire : 200 000 lignes d'un coup, dont des fins de
/// ligne CRLF, toutes restituées dans l'ordre.
#[test]
fn gros_rattrapage_decoupe_en_une_passe() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    let contents: String = (0..200_000).map(|i| format!("ligne {i}\r\n")).collect();
    write_new(&path, &contents);

    let mut tailer = Tailer::new(&path);
    let lines: Vec<String> = tailer
        .poll()
        .unwrap()
        .into_iter()
        .flat_map(|batch| batch.lines)
        .collect();
    assert_eq!(lines.len(), 200_000);
    assert_eq!(lines[0], "ligne 0");
    assert_eq!(lines[199_999], "ligne 199999");
}

/// O7 : un reliquat plus gros qu'une tranche de lecture (4 Mio) est lu par tranches — aucune
/// ligne perdue ni coupée à une frontière de tranche, lots bornés à `MAX_BATCH_LINES`, et le
/// rattrapage est bien terminé à la fin du même `poll()`.
#[test]
fn rattrapage_plus_gros_qu_une_tranche_sans_ligne_coupee() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    // Longueurs de ligne variables (dont des lignes longues) : les frontières de tranche tombent
    // forcément au milieu de lignes.
    let contents: String = (0..60_000)
        .map(|i| format!("ligne {i} {}\n", "x".repeat(i % 400)))
        .collect();
    assert!(contents.len() > 9 * 1024 * 1024, "{}", contents.len());
    write_new(&path, &contents);

    let mut tailer = Tailer::new(&path);
    let batches = tailer.poll().unwrap();
    assert!(batches
        .iter()
        .all(|batch| batch.lines.len() <= MAX_BATCH_LINES));
    assert!(batches.iter().all(|batch| batch.is_initial_load));
    let lines: Vec<String> = batches.into_iter().flat_map(|batch| batch.lines).collect();
    assert_eq!(lines.len(), 60_000);
    for (i, line) in lines.iter().enumerate() {
        assert_eq!(*line, format!("ligne {i} {}", "x".repeat(i % 400)));
    }

    assert!(tailer.poll().unwrap().is_empty());
    append(&path, "direct\n");
    let batches = tailer.poll().unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].lines, vec!["direct"]);
    assert!(
        !batches[0].is_initial_load,
        "rattrapé dès le premier poll()"
    );
}
