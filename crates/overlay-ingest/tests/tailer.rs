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
fn troncature_en_place_emet_les_lignes_reecrites_en_direct() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    write_new(&path, "avant troncature, une longue ligne\n");

    let mut tailer = Tailer::new(&path);
    tailer.poll().unwrap();
    assert!(tailer.poll().unwrap().is_empty()); // rattrapé

    // Troncature en place (même fichier physique, contenu plus court) : c'est ce que fait un
    // client Wakfu lancé alors qu'un autre tourne déjà (§5.2) — pas une rotation. Les lignes
    // réécrites sont du direct, jamais un nouveau rattrapage.
    write_new(&path, "court\n");
    let batches = tailer.poll().unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].lines, vec!["court"]);
    assert!(!batches[0].is_initial_load);
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

    // Nouvelle identité : rattrapage (`is_initial_load`). Si l'OS a réutilisé l'inode (§5.2), la
    // relecture complète voit quand même chaque ligne comme neuve, en direct — les deux issues
    // livrent la nouvelle ligne une fois et une seule.
    let batches = tailer.poll().unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(
        batches[0].lines,
        vec!["nouvelle session, plus longue que l'ancienne ligne precedente"]
    );
    assert!(tailer.poll().unwrap().is_empty());
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

/// Une ligne démesurée (fichier corrompu) n'est jamais gardée en mémoire ni transmise, même une
/// fois terminée : seules les lignes suivantes sont lues.
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
    assert_eq!(batches[0].lines, vec!["ligne suivante"]);
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

/// `wakfu.log` partagé par plusieurs clients (§5.2) : chacun écrit à SA position en écrasant ce qui
/// s'y trouve, sans jamais changer l'identité du fichier.
fn write_at(path: &Path, offset: u64, contents: &str) {
    use std::io::{Seek, SeekFrom};
    let mut f = fs::OpenOptions::new().write(true).open(path).unwrap();
    f.seek(SeekFrom::Start(offset)).unwrap();
    f.write_all(contents.as_bytes()).unwrap();
}

fn log_line(time: &str, text: &str) -> String {
    format!(" INFO {time} [AWT-EventQueue-0] (aNZ:174) - [Information (jeu)] {text}\r\n")
}

#[test]
fn client_en_retard_rattrape_par_la_relecture_complete() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    let head: String = (0..6)
        .map(|i| log_line(&format!("10:00:0{i},000"), &format!("tête {i}")))
        .collect();
    write_new(&path, &head);

    let mut tailer = Tailer::new(&path);
    assert_eq!(tailer.poll().unwrap()[0].lines.len(), 6);

    // Client lancé en dernier : réécrit depuis 0, derrière la fin déjà lue ; le client en tête
    // continue au bout. Une lecture par offset ne verrait que la 2ᵉ ligne.
    let corne = log_line("10:00:06,000", "Vous avez ramassé 1x Corne .");
    write_at(&path, 0, &corne);
    append(&path, &log_line("10:00:06,500", "tête 6"));
    let lines: Vec<String> = tailer
        .rescan()
        .unwrap()
        .into_iter()
        .flat_map(|b| b.lines)
        .collect();
    // Ni les lignes déjà lues, ni le reste de l'ancienne ligne écrasée (fragment sans en-tête).
    assert_eq!(
        lines,
        vec![
            corne.trim_end().to_string(),
            log_line("10:00:06,500", "tête 6").trim_end().to_string(),
        ]
    );

    // Le client en retard continue : le fragment qu'il recouvre n'est jamais émis.
    let karne = log_line("10:00:07,000", "Vous avez ramassé 1x Karne .");
    write_at(&path, corne.len() as u64, &karne);
    let batches = tailer.rescan().unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].lines, vec![karne.trim_end().to_string()]);
    assert!(!batches[0].is_initial_load);
    assert!(tailer.rescan().unwrap().is_empty());
}

#[test]
fn trou_d_octets_nuls_apres_troncature_separe_les_lignes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    let before = log_line("10:00:00,000", "avant") + &log_line("10:00:01,000", "avant 2");
    write_new(&path, &before);
    let mut tailer = Tailer::new(&path);
    tailer.poll().unwrap();

    // Nouveau client lancé : fichier tronqué ; le client déjà lancé écrit toujours à sa
    // position (trou d'octets nuls devant), le nouveau depuis 0.
    write_new(&path, "");
    let continued = log_line("10:00:02,000", "client 1 continue");
    write_at(&path, before.len() as u64, &continued);
    let started = log_line("10:00:02,100", "démarrage client 0");
    write_at(&path, 0, &started);

    let lines: Vec<String> = tailer
        .rescan()
        .unwrap()
        .into_iter()
        .flat_map(|b| b.lines)
        .collect();
    assert_eq!(
        lines,
        vec![
            started.trim_end().to_string(),
            continued.trim_end().to_string()
        ]
    );
}

#[test]
fn lecture_de_fin_refuse_une_jonction_reecrite() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    let first = log_line("10:00:00,000", "a");
    write_new(&path, &first);
    let mut tailer = Tailer::new(&path);
    tailer.poll().unwrap();

    // Un autre client écrase le `\n` final connu puis va plus loin : `poll()` (lecture de fin,
    // pas de relecture due) doit basculer sur la relecture complète plutôt que d'émettre un
    // fragment.
    let overwrite = log_line(
        "10:00:01,000",
        "écrasée par un client en retard plus longue",
    );
    write_at(&path, 0, &overwrite);
    let lines: Vec<String> = tailer
        .poll()
        .unwrap()
        .into_iter()
        .flat_map(|b| b.lines)
        .collect();
    assert_eq!(lines, vec![overwrite.trim_end().to_string()]);
}
