//! Petit client HTTP bloquant (`ureq`, rustls) — voir `docs/plan-architecture.md` §7.2 pour le
//! choix de rester en `ureq` plutôt que `reqwest`+`tokio` tant que la vraie file d'envoi
//! asynchrone (L5) n'existe pas : ce crate ne fait que quelques requêtes ponctuelles sur son
//! propre thread, jamais sur le chemin chaud du rendu.

use std::time::Duration;

use overlay_engine::RosterIndex;
use serde_json::Value;

use crate::SyncError;

const DEFAULT_BASE_URL: &str = "https://wakfu-companion.com";
const TIMEOUT: Duration = Duration::from_secs(10);

/// Origine de l'API — `WAKFU_COMPANION_API_URL` en priorité (utile contre un `wrangler pages dev`
/// local), repli sur le domaine public. Jamais codée en dur sans repli overridable.
pub fn base_url() -> String {
    std::env::var("WAKFU_COMPANION_API_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into()
}

pub(crate) fn post_json(path: &str, body: &Value) -> Result<Value, SyncError> {
    let url = format!("{}{path}", base_url());
    let response = agent()
        .post(&url)
        .send_json(body)
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body(path, response)
}

fn parse_json_body(
    path: &str,
    mut response: ureq::http::Response<ureq::Body>,
) -> Result<Value, SyncError> {
    let status = response.status().as_u16();
    let value: Value = response
        .body_mut()
        .read_json()
        .map_err(|err| SyncError::Json(err.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(SyncError::Http {
            status,
            path: path.to_string(),
        });
    }
    Ok(value)
}

/// `GET /api/v1/settings` avec `Authorization: Bearer <token>` — voir `functions/api/_auth.ts`
/// (dépôt `wakfu-companion`) pour l'acceptation du porteur en plus du cookie. Renvoie directement
/// le roster déjà indexé (`RosterIndex::from_settings_json` attend l'objet `data` de cette
/// réponse, pas la réponse entière — voir sa doc).
pub fn fetch_roster(token: &str) -> Result<RosterIndex, SyncError> {
    let url = format!("{}/api/v1/settings", base_url());
    let mut response = agent()
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    let status = response.status().as_u16();
    let body: Value = response
        .body_mut()
        .read_json()
        .map_err(|err| SyncError::Json(err.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(SyncError::Http {
            status,
            path: "/api/v1/settings".to_string(),
        });
    }
    let data = body.get("data").cloned().unwrap_or(Value::Null);
    Ok(RosterIndex::from_settings_json(&data))
}
