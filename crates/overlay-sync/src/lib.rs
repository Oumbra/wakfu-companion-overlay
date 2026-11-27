//! Auth native par appairage (lot L4, `docs/plan-architecture.md` §7.2) + récupération du roster
//! de personnages (`GET /api/v1/settings`) + synchronisation de l'historique (lot L5, §7.1/§7.3 —
//! `queue.rs`) + téléchargement des icônes (`client::icon_urls`, `icon_cache.rs`) + mise à jour
//! signée (`update`) — la brique réseau de l'overlay. Le catalogue, les recettes, les donjons et
//! les serveurs de jeu sont embarqués depuis le 2026-11-27 (`overlay_engine::referentiel`).

pub mod client;
pub mod icon_cache;
pub mod pairing;
pub mod queue;
pub mod token_store;
pub mod update;

pub use client::{
    fetch_account_id, fetch_history_page, fetch_settings, patch_chat_filters,
    patch_json_authenticated, patch_roster, patch_watchlist, post_json, post_json_authenticated,
    AccountSettings,
};
pub use pairing::{pair_and_wait, PairingHandle};
pub use queue::{client_key, FlushOutcome, SyncQueue, MAX_PENDING_AGE};

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("erreur réseau : {0}")]
    Network(String),
    /// Réponse non 2xx. `code` est le champ `code` du corps JSON d'erreur quand le serveur en pose
    /// un (`history_quota_exceeded`, `browser_session_required`, `rate_limited`…) ; `retry_after`
    /// l'en-tête `Retry-After` interprété (secondes ou date HTTP, plafonné — voir
    /// `client::parse_retry_after`). Les deux pilotent la file d'envoi (`queue::flush_once`).
    #[error("réponse HTTP {status} inattendue pour {path}{}", code.as_deref().map(|c| format!(" ({c})")).unwrap_or_default())]
    Http {
        status: u16,
        path: String,
        code: Option<String>,
        retry_after: Option<std::time::Duration>,
    },
    #[error("réponse JSON invalide : {0}")]
    Json(String),
    #[error("l'appairage a expiré avant confirmation")]
    PairingExpired,
    /// Attente abandonnée à la demande de l'utilisateur (voir `pair_and_wait`) — pas un échec à
    /// afficher en rouge : la fenêtre de connexion revient simplement à son état « non connecté ».
    #[error("appairage annulé")]
    PairingCancelled,
    #[error("erreur d'accès au trousseau/fichier de jeton : {0}")]
    TokenStore(String),
}
