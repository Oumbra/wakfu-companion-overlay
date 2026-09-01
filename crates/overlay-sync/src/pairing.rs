//! Appairage d'un client natif (lot L4, `docs/plan-architecture.md` §7.2) — mirroir côté overlay
//! des routes `functions/api/v1/auth/native/{pair,poll}.ts` du dépôt `wakfu-companion`.

use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::json;

use crate::client::post_json;
use crate::SyncError;

const POLL_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Debug, Deserialize)]
struct PairResponse {
    #[serde(rename = "pairingCode")]
    pairing_code: String,
    #[serde(rename = "pollToken")]
    poll_token: String,
    #[serde(rename = "verificationUrl")]
    verification_url: String,
    #[serde(rename = "expiresInSeconds")]
    expires_in_seconds: u64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum PollResponse {
    Pending,
    Expired,
    Claimed { token: String },
}

/// Détails d'un appairage démarré, à afficher à l'utilisateur (console, ou future UI) avant de
/// sonder — voir `pair_and_wait`.
pub struct PairingHandle {
    pub pairing_code: String,
    pub verification_url: String,
}

fn start_pairing() -> Result<PairResponse, SyncError> {
    let value = post_json("/api/v1/auth/native/pair", &json!({}))?;
    serde_json::from_value(value).map_err(|err| SyncError::Json(err.to_string()))
}

fn poll_once(poll_token: &str) -> Result<PollResponse, SyncError> {
    let value = post_json(
        "/api/v1/auth/native/poll",
        &json!({ "pollToken": poll_token }),
    )?;
    serde_json::from_value(value).map_err(|err| SyncError::Json(err.to_string()))
}

/// Démarre un appairage et bloque (sur le thread appelant — TOUJOURS un thread dédié côté
/// `overlay-ui`, jamais le thread de rendu) jusqu'à confirmation, expiration, ou erreur réseau.
///
/// `on_started` reçoit le code + l'URL de vérification dès qu'ils sont connus, AVANT le premier
/// sondage — c'est le seul moyen que l'appelant a de les afficher à l'utilisateur. Le navigateur
/// par défaut est ouvert en best-effort (`open::that`, erreur ignorée : l'utilisateur peut de
/// toute façon copier l'URL affichée par `on_started`).
pub fn pair_and_wait(on_started: impl FnOnce(&PairingHandle)) -> Result<String, SyncError> {
    let started = start_pairing()?;
    on_started(&PairingHandle {
        pairing_code: started.pairing_code.clone(),
        verification_url: started.verification_url.clone(),
    });
    let _ = open::that(&started.verification_url);

    let deadline = Instant::now() + Duration::from_secs(started.expires_in_seconds.max(60));
    loop {
        if Instant::now() >= deadline {
            return Err(SyncError::PairingExpired);
        }
        match poll_once(&started.poll_token)? {
            PollResponse::Claimed { token } => return Ok(token),
            PollResponse::Expired => return Err(SyncError::PairingExpired),
            PollResponse::Pending => thread::sleep(POLL_INTERVAL),
        }
    }
}
