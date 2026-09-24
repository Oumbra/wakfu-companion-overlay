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
    let mut started: PairResponse =
        serde_json::from_value(value).map_err(|err| SyncError::Json(err.to_string()))?;
    if !is_valid_pairing_code(&started.pairing_code) {
        return Err(SyncError::Json(format!(
            "code d'appairage inattendu : {:?}",
            started.pairing_code
        )));
    }
    started.verification_url = trusted_verification_url(
        &crate::client::base_url(),
        &started.verification_url,
        &started.pairing_code,
    );
    Ok(started)
}

/// Code court affiché à l'utilisateur : quelques caractères alphanumériques ASCII, rien d'autre.
fn is_valid_pairing_code(code: &str) -> bool {
    (4..=16).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// URL que l'overlay acceptera d'ouvrir dans le navigateur (`open::that`) : celle du serveur
/// seulement si elle est sur l'origine de l'API, sinon reconstruite à partir du code.
///
/// Sous Windows, `open::that` passe par `ShellExecuteW`, qui ouvre ou EXÉCUTE n'importe quelle
/// cible (`\\hôte\partage\x.exe`, `file:///…`). Faire confiance à `verificationUrl` aurait
/// transformé une API compromise (ou une réponse interceptée) en exécution de code chez chaque
/// client en cours d'appairage — exactement ce que la signature des mises à jour cherche à
/// empêcher par ailleurs (audit de sécurité du 2026-09-23).
fn trusted_verification_url(base_url: &str, served: &str, pairing_code: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let prefix = format!("{base}/");
    let safe_chars = served
        .bytes()
        .all(|b| b.is_ascii_graphic() && !matches!(b, b'\\' | b'"' | b'<' | b'>' | b'`'));
    if served.starts_with(&prefix) && safe_chars {
        return served.to_string();
    }
    tracing::warn!(
        served,
        "URL de vérification hors de l'origine de l'API : reconstruite"
    );
    format!("{base}/pair?code={pairing_code}")
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

#[cfg(test)]
mod url_tests {
    use super::*;

    const BASE: &str = "https://wakfu-companion.com";

    #[test]
    fn garde_l_url_du_serveur_sur_la_meme_origine() {
        assert_eq!(
            trusted_verification_url(
                BASE,
                "https://wakfu-companion.com/pair?code=ABCD2345",
                "ABCD2345"
            ),
            "https://wakfu-companion.com/pair?code=ABCD2345"
        );
    }

    #[test]
    fn reconstruit_toute_url_hors_origine_ou_suspecte() {
        for served in [
            "\\\\attaquant\\partage\\x.exe",
            "file:///C:/Windows/System32/calc.exe",
            "https://wakfu-companion.com.evil.example/pair",
            "https://evil.example/?https://wakfu-companion.com/",
            "https://wakfu-companion.com/pair?code=A B",
        ] {
            assert_eq!(
                trusted_verification_url(BASE, served, "ABCD2345"),
                "https://wakfu-companion.com/pair?code=ABCD2345",
                "{served}"
            );
        }
    }

    #[test]
    fn refuse_un_code_d_appairage_exotique() {
        assert!(is_valid_pairing_code("ABCD2345"));
        assert!(!is_valid_pairing_code("AB"));
        assert!(!is_valid_pairing_code("ABCD&calc"));
    }
}
