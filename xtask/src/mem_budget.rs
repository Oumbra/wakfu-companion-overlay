//! `xtask mem-budget` — garde-fou mémoire (docs/plan-architecture.md §8). Rejoue le vrai
//! `crates/overlay-engine/tests/wakfu.log` (10 975 lignes, voir le fichier), RÉPÉTÉ jusqu'à
//! cumuler ~80 000 lignes ingérées (référence explicite du plan : `overlay_ingest::tailer::
//! MAX_BATCH_LINES`, doc de module, cite littéralement « un fichier existant de 80 000 lignes »),
//! sur un seul `Engine` qui vit tout du long — puis mesure le RSS du process courant.
//!
//! **Portée honnêtement partielle, pas le budget mémoire complet du §8** : ce scénario n'exécute
//! QUE le moteur d'ingestion (`overlay-engine`/`overlay-ingest`), jamais `overlay-ui` (fenêtrage
//! winit/wgpu, poste dominant du tableau du §8, 70–120 Mo, largement hors de notre contrôle) —
//! qui ne peut de toute façon pas tourner ici (Windows-only aujourd'hui, voir §17.2). Ce garde-fou
//! couvre donc les postes « État métier » + « Catalogue » + « file/divers » du tableau du §8
//! (~33–85 Mo estimés), PAS le total de 300 Mo. Le plafond ci-dessous (`CEILING_BYTES`, 100 Mo)
//! reflète cette portée réduite, pas la cible globale du produit — mesuré à 17,1 Mo lors de
//! l'écriture de ce fichier (marge ~6× pour absorber la variance du runner CI et la croissance
//! future, ex. chargement du catalogue). Voir aussi `docs/plan-architecture.md` §17.3 sur
//! l'absence, à ce jour, d'un scénario qui piloterait le vrai binaire `overlay-ui`.
//!
//! **Simulation d'une session longue, pas 8 sessions indépendantes** : le fichier réel est
//! RÉ-APPENDÉ (jamais retronqué) à un fichier temporaire suivi par un SEUL `Tailer` qui ne repart
//! jamais de zéro — après le tout premier rattrapage (`is_initial_load`), les lots suivants sont
//! traités comme du direct, exactement comme une vraie session qui durerait plus longtemps que ce
//! qu'un unique combat couvre — plutôt qu'un unique rattrapage géant qui ne testerait que le cas
//! `is_initial_load`.

use std::io::Write as _;

use overlay_engine::Engine;
use overlay_ingest::Tailer;

/// Voir la doc de module pour ce que ce plafond couvre (et ne couvre pas).
const CEILING_BYTES: u64 = 100 * 1024 * 1024;

/// Nombre de fois où le contenu réel du log est ré-appendé — `10_975 * 8 ≈ 87 800`, au-dessus des
/// 80 000 lignes de référence du plan (voir la doc de module) avec une marge raisonnable.
const REPEATS: usize = 8;

const WAKFU_LOG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/overlay-engine/tests/wakfu.log"
);

pub fn run() {
    let real_log = std::fs::read_to_string(WAKFU_LOG).expect("lecture du wakfu.log de test");
    let line_count = real_log.lines().count();

    let tmp_dir = std::env::temp_dir().join(format!("wakfu-overlay-xtask-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).expect("création du dossier temporaire");
    let tmp_log = tmp_dir.join("wakfu.log");
    std::fs::write(&tmp_log, "").expect("création du fichier temporaire suivi");

    let mut engine = Engine::with_stores(
        tmp_dir.join("watchlist-counts.json"),
        tmp_dir.join("fights"),
    )
    .expect("création de l'Engine");
    let mut tailer = Tailer::new(&tmp_log);
    let mut ingested_lines = 0usize;

    for repeat in 1..=REPEATS {
        {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&tmp_log)
                .expect("ouverture en écriture du fichier suivi");
            file.write_all(real_log.as_bytes())
                .expect("écriture du contenu répété");
        }
        loop {
            let batches = tailer.poll().expect("poll() du tailer");
            if batches.is_empty() {
                break;
            }
            for batch in &batches {
                ingested_lines += batch.lines.len();
                engine.ingest_batch(batch).expect("ingestion d'un lot");
            }
        }
        eprintln!(
            "[mem-budget] répétition {repeat}/{REPEATS} — {ingested_lines} lignes cumulées ({line_count} par répétition)"
        );
    }

    let _ = engine.snapshot(); // matérialise le coût réel d'un snapshot, comme le ferait overlay-ui à chaque frame.

    let rss = current_rss_bytes();
    let rss_mb = rss as f64 / (1024.0 * 1024.0);
    let ceiling_mb = CEILING_BYTES as f64 / (1024.0 * 1024.0);
    eprintln!(
        "[mem-budget] RSS après {ingested_lines} lignes ingérées : {rss_mb:.1} Mo (plafond {ceiling_mb:.1} Mo — portée partielle, voir la doc de module)"
    );

    let _ = std::fs::remove_dir_all(&tmp_dir);

    if rss > CEILING_BYTES {
        eprintln!("[mem-budget] ÉCHEC : RSS au-dessus du plafond");
        std::process::exit(1);
    }
    println!("[mem-budget] OK — {rss_mb:.1} Mo <= {ceiling_mb:.1} Mo");
}

/// RSS du process courant, en octets. `sysinfo` plutôt qu'une lecture ad hoc de `/proc/self/status`
/// (Linux uniquement) : cross-platform par construction, seul point qui doit un jour tourner sur
/// le même runner que `overlay-ui` (Windows ET Linux, voir §11 du plan).
fn current_rss_bytes() -> u64 {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

    let pid = Pid::from_u32(std::process::id());
    let mut system = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().with_memory()),
    );
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    system.process(pid).map(|p| p.memory()).unwrap_or(0)
}
