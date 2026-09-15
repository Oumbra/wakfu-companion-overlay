//! Câblage minimal de L1 (`overlay-ingest`) — voir docs/plan-architecture.md §4 et §12.
//!
//! Ce binaire n'est **pas** encore l'overlay final : pas de fenêtre, pas de rendu, pas de config
//! persistée, pas de hotkey (ça, c'est L2+). Son seul but est de rendre L1 observable sur un vrai
//! `wakfu.log` — découverte de chemin, rattrapage initial, suivi en direct, rotation — avant de
//! commencer l'UI par-dessus. `main.rs`/`config.rs`/`hotkey.rs` grossiront à partir d'ici au fil
//! des lots suivants, pas d'un nouveau binaire.
//!
//! Usage :
//!   cargo run -p overlay-app                  # découverte automatique du chemin (§5.1)
//!   cargo run -p overlay-app -- <chemin.log>   # chemin explicite (utile pour rejouer un fichier)

use std::env;
use std::path::PathBuf;

use overlay_ingest::{discovery, watcher};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let path = resolve_path();
    tracing::info!("=== overlay-app (L1) — suivi de {} ===", path.display());
    tracing::info!("Ctrl+C pour arrêter.");

    let rx = watcher::spawn(&path);
    let mut batch_count: u64 = 0;
    let mut total_lines: u64 = 0;

    for result in rx {
        match result {
            Ok(batch) => {
                batch_count += 1;
                total_lines += batch.lines.len() as u64;
                let tag = if batch.is_initial_load {
                    "RATTRAPAGE"
                } else {
                    "DIRECT    "
                };
                tracing::info!(
                    "[lot #{batch_count:>4}] {tag} {:>5} ligne(s)  (cumul {total_lines})",
                    batch.lines.len()
                );
                // Aperçu court : voir que ce sont de vraies lignes du log, sans noyer la console
                // sur un fichier de plusieurs milliers de lignes.
                const APERCU: usize = 3;
                for line in batch.lines.iter().take(APERCU) {
                    tracing::info!("    {line}");
                }
                if batch.lines.len() > APERCU {
                    tracing::info!("    … ({} ligne(s) de plus)", batch.lines.len() - APERCU);
                }
            }
            Err(err) => tracing::error!("[erreur de lecture] {err}"),
        }
    }
}

/// Argument en ligne de commande si présent, sinon découverte automatique (§5.1) — l'échec de
/// découverte n'est pas silencieux : on liste les chemins essayés, comme le fera plus tard le
/// sélecteur manuel côté UI.
fn resolve_path() -> PathBuf {
    if let Some(arg) = env::args().nth(1) {
        return PathBuf::from(arg);
    }
    match discovery::discover() {
        Some(path) => path,
        None => {
            tracing::error!("wakfu.log introuvable aux emplacements connus. Chemins essayés :");
            for candidate in discovery::candidate_paths() {
                tracing::error!("  - {}", candidate.display());
            }
            tracing::error!(
                "Précisez le chemin explicitement : cargo run -p overlay-app -- <chemin>"
            );
            std::process::exit(1);
        }
    }
}
