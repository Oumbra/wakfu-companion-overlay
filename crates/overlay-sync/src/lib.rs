//! Auth native par appairage (lot L4, `docs/plan-architecture.md` §7.2) + récupération du roster
//! de personnages (`GET /api/v1/settings`) — première brique réseau de l'overlay. Volontairement
//! minimal : pas de file d'envoi, pas de synchro d'historique (L5, à venir) — juste de quoi obtenir
//! un jeton natif et lire le roster en lecture seule au démarrage.

pub mod client;
pub mod pairing;
pub mod token_store;

pub use client::{fetch_settings, AccountSettings};
pub use pairing::{pair_and_wait, PairingHandle};

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
