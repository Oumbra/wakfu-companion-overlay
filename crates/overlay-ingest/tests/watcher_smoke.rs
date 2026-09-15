//! Vérification du watcher réel (`notify` + repli, `src/watcher.rs`) — volontairement `#[ignore]`
//! par défaut : dépend d'événements filesystem réels, potentiellement plus lent et moins
//! déterministe en CI que les tests synchrones de `tests/tailer.rs`, qui couvrent la vraie
//! logique testable. À lancer à la main après toute modification de `src/watcher.rs` :
//! `cargo test -p overlay-ingest --test watcher_smoke -- --ignored --nocapture`.

use std::fs;
use std::io::Write;
use std::time::Duration;

use overlay_ingest::watcher;

#[test]
#[ignore]
fn suivi_temps_reel_et_rotation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wakfu.log");
    fs::write(&path, b"").unwrap();

    let rx = watcher::spawn(&path);

    let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
    writeln!(f, "premiere ligne").unwrap();
    drop(f);

    let batch = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("aucun lot reçu (notify silencieux ?)")
        .expect("poll() en erreur");
    assert_eq!(batch.lines, vec!["premiere ligne"]);
    println!("OK : lot reçu via notify en direct : {:?}", batch.lines);

    // Rotation : suppression + recréation au même chemin.
    fs::remove_file(&path).unwrap();
    let mut f = fs::File::create(&path).unwrap();
    writeln!(f, "apres rotation").unwrap();
    drop(f);

    let batch = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("aucun lot reçu après rotation")
        .expect("poll() en erreur après rotation");
    assert_eq!(batch.lines, vec!["apres rotation"]);
    assert!(
        batch.is_initial_load,
        "rotation = nouveau rattrapage, voir tailer.rs"
    );
    println!("OK : rotation détectée en direct : {:?}", batch.lines);
}
