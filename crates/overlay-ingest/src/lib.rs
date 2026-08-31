//! Suivi (tail) de `wakfu.log` : découverte de chemin, lecture incrémentale, détection de
//! rotation/troncature — voir docs/plan-architecture.md §5. Thread IO du diagramme d'exécution
//! §3 : ce crate ne parse rien, il découpe des lignes complètes et les publie par lots.

pub mod discovery;
pub mod rotation;
pub mod tailer;
pub mod watcher;

pub use rotation::FileIdentity;
pub use tailer::{LineBatch, Tailer, MAX_BATCH_LINES};
