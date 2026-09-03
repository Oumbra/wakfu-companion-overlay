//! Spike S3 (docs/plan-architecture.md §12, §17.2) — bibliothèque partagée par `main.rs` (overlay
//! de démonstration) et `probe.rs` (sonde d'assertions du harnais). Voir la doc de chaque module.

pub mod discovery;
pub mod topmost;
