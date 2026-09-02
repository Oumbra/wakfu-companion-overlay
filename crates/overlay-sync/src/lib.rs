//! Auth native par appairage (lot L4, `docs/plan-architecture.md` §7.2) + récupération du roster
//! de personnages (`GET /api/v1/settings`) + synchronisation de l'historique (lot L5, §7.1/§7.3 —
//! `queue.rs`) — la brique réseau de l'overlay.

pub mod catalog_cache;
pub mod client;
pub mod icon_cache;
pub mod pairing;
pub mod queue;
pub mod reference_data_cache;
pub mod token_store;

pub use client::{
    fetch_account_id, fetch_catalog_index, fetch_catalog_version, fetch_dungeons,
    fetch_monster_families, fetch_settings, post_json, AccountSettings,
};
pub use pairing::{pair_and_wait, PairingHandle};
pub use queue::{client_key, FlushOutcome, SyncQueue};

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("erreur réseau : {0}")]
    Network(String),
    #[error("réponse HTTP {status} inattendue pour {path}")]
    Http { status: u16, path: String },
    #[error("réponse JSON invalide : {0}")]
    Json(String),
    #[error("l'appairage a expiré avant confirmation")]
    PairingExpired,
    #[error("erreur d'accès au trousseau/fichier de jeton : {0}")]
    TokenStore(String),
}
