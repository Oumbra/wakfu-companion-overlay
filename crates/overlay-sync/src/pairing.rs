//! Appairage d'un client natif (lot L4, `docs/plan-architecture.md` §7.2) — mirroir côté overlay
//! des routes `functions/api/v1/auth/native/{pair,poll}.ts` du dépôt `wakfu-companion`.

use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::json;

use crate::client::post_json;
use crate::SyncError;

const POLL_INTERVAL: Duration = Duration::from_secs(3);
/// Pas de la sieste entre deux sondages — `should_cancel` (voir `pair_and_wait`) est consulté à
/// chaque pas, jamais seulement toutes les `POLL_INTERVAL` : un « Annuler l'appairage » cliqué dans
/// la fenêtre de connexion doit se voir en moins d'une demi-seconde, pas trois plus tard.
const CANCEL_CHECK_INTERVAL: Duration = Duration::from_millis(250);

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
    /// Instant au-delà duquel le serveur ne reconnaîtra plus ce code (`expiresInSeconds` de la
    /// réponse `pair`, déjà la borne du sondage ci-dessous) — la fenêtre de connexion en déduit
    /// son compte à rebours « expire dans mm:ss » (2026-09-14, §9.1 undecies du plan).
    pub expires_at: Instant,
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
///
/// `should_cancel` est consulté entre deux sondages (toutes les `CANCEL_CHECK_INTERVAL`) : dès
/// qu'il rend `true`, l'attente s'arrête sur [`SyncError::PairingCancelled`] — c'est le lien
/// « Annuler l'appairage » de la fenêtre de connexion (2026-09-14). Le code lui-même n'est pas
/// révoqué côté serveur (aucune route pour ça) : il expire tout seul, et le sondage qui l'attendait
/// est simplement abandonné.
pub fn pair_and_wait(
    on_started: impl FnOnce(&PairingHandle),
    should_cancel: impl Fn() -> bool,
) -> Result<String, SyncError> {
    let started = start_pairing()?;
    let deadline = Instant::now() + Duration::from_secs(started.expires_in_seconds.max(60));
    on_started(&PairingHandle {
        pairing_code: started.pairing_code.clone(),
        verification_url: started.verification_url.clone(),
        expires_at: deadline,
    });
    let _ = open::that(&started.verification_url);

    loop {
        if should_cancel() {
            return Err(SyncError::PairingCancelled);
        }
        if Instant::now() >= deadline {
            return Err(SyncError::PairingExpired);
        }
        match poll_once(&started.poll_token)? {
            PollResponse::Claimed { token } => return Ok(token),
            PollResponse::Expired => return Err(SyncError::PairingExpired),
            PollResponse::Pending => {
                // Sieste par petits pas plutôt qu'un seul `sleep(POLL_INTERVAL)` : l'annulation
                // doit être vue vite, le serveur, lui, n'a pas besoin d'être sondé plus souvent.
                let next_poll = Instant::now() + POLL_INTERVAL;
                while Instant::now() < next_poll {
                    if should_cancel() {
                        return Err(SyncError::PairingCancelled);
                    }
                    thread::sleep(CANCEL_CHECK_INTERVAL);
                }
            }
        }
    }
}
