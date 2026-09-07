//! Petit client HTTP bloquant (`ureq`, rustls) — voir `docs/plan-architecture.md` §7.3 pour le
//! choix définitif de rester en `ureq` plutôt que `reqwest`+`tokio`, y compris pour la file d'envoi
//! (L5, `queue.rs`) : chaque thread réseau (auth, catalogue, sync) reste un `std::thread` bloquant
//! dédié, jamais sur le chemin chaud du rendu — un seul modèle de concurrence dans tout le binaire.

use std::time::Duration;

use overlay_engine::{
    sound_items_from_settings_json, watchlist_from_settings_json, watchlist_patch_entry,
    RosterIndex, SoundItemEntry, WatchlistEntry,
};
use serde_json::Value;

use crate::SyncError;

// ⚠️ Pointe vers le déploiement DEV (`claude-dev.wakfu-companion.com`), PAS la prod
// (`wakfu-companion.com`) — retour utilisateur 2026-09-01 : les routes d'appairage natif
// (`/api/v1/auth/native/*`) ne sont pas encore déployées en prod à ce stade (confirmé : la prod
// renvoie 405 sur `POST /api/v1/auth/native/pair`, le domaine dev répond 200). À repointer vers la
// prod dès que l'appairage natif y est déployé — ne pas oublier avant toute release réelle.
const DEFAULT_BASE_URL: &str = "https://claude-dev.wakfu-companion.com";
const TIMEOUT: Duration = Duration::from_secs(10);

/// Origine de l'API — `WAKFU_COMPANION_API_URL` en priorité (utile contre un `wrangler pages dev`
/// local), repli sur `DEFAULT_BASE_URL` ci-dessus. Jamais codée en dur sans repli overridable.
pub fn base_url() -> String {
    std::env::var("WAKFU_COMPANION_API_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into()
}

/// POST anonyme — utilisé par `pairing.rs` pour les deux seules routes appelées AVANT qu'un jeton
/// n'existe (`/api/v1/auth/native/{pair,poll}`). **Ne jamais l'utiliser pour l'historique** (voir
/// [`post_json_authenticated`]) : `SyncQueue::flush_once` en a besoin d'une variante qui envoie le
/// jeton — le bug corrigé le 2026-09-03 était exactement cette confusion.
pub fn post_json(path: &str, body: &Value) -> Result<Value, SyncError> {
    let url = format!("{}{path}", base_url());
    let response = agent()
        .post(&url)
        .send_json(body)
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body(path, response)
}

/// Variante authentifiée de [`post_json`] — ajoute `Authorization: Bearer <token>`, requis par les
/// routes mutatives d'historique (`/api/v1/history/{fights,purchases,trades}`, voir
/// `functions/api/_auth.ts::authenticate` côté dépôt web, qui accepte le porteur au même titre que
/// le cookie de session, mais SANS repli anonyme).
///
/// **Correctif du 2026-09-03** (retour utilisateur : une récupération de kamas HDV jamais visible
/// sur le site) : `SyncQueue::flush_once` appelait jusqu'ici `post_json` — sans jeton — pour ces
/// trois routes, exactement comme `fetch_account_id`/`fetch_settings` le font correctement pour les
/// leurs. Résultat vérifié en conditions réelles : chaque envoi d'historique échouait en `401 non
/// authentifié`, silencieusement (401 est un rejet réessayable, jamais un rejet permanent — voir
/// `is_permanent_rejection` — donc jamais tracé), depuis l'introduction de la file d'envoi (lot L5,
/// 2026-09-02). Aucun événement d'historique n'a donc jamais pu atteindre le compte avant ce jour.
pub fn post_json_authenticated(token: &str, path: &str, body: &Value) -> Result<Value, SyncError> {
    let url = format!("{}{path}", base_url());
    let response = agent()
        .post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send_json(body)
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body(path, response)
}

/// Variante `PATCH` authentifiée de [`post_json_authenticated`] — utilisée pour
/// `PATCH /api/v1/settings` (écriture PAR CLÉ, « dernier écrivain gagne », voir
/// `functions/api/v1/settings.ts::onRequestPatch` côté dépôt web), PAS pour l'historique (routes
/// `POST /api/v1/history/*`, qui restent sur [`post_json_authenticated`]).
pub fn patch_json_authenticated(token: &str, path: &str, body: &Value) -> Result<Value, SyncError> {
    let url = format!("{}{path}", base_url());
    let response = agent()
        .patch(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send_json(body)
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body(path, response)
}

/// `PATCH /api/v1/settings` pour répliquer les compteurs de Suivi (watchlist) vers le compte —
/// voir `overlay_engine::watchlist::watchlist_patch_entry` pour le format de l'entrée envoyée, et
/// `docs/plan-architecture.md` §14 point 3 (chantier fermé le 2026-09-07, retour utilisateur : un
/// Suivi jamais visible sur le site). Un lot d'une seule entrée EST l'écriture par clé (pas de
/// route `/settings/{key}` séparée côté serveur).
pub fn patch_watchlist(token: &str, entries: &[WatchlistEntry]) -> Result<Value, SyncError> {
    let body = serde_json::json!({ "entries": [watchlist_patch_entry(entries)] });
    patch_json_authenticated(token, "/api/v1/settings", &body)
}

/// Récupère les octets bruts d'une URL absolue quelconque — PAS `base_url()` (utilisé tel quel
/// pour un CDN externe, ex. les icônes `wakassets`, voir `overlay_engine::catalog::IconRef::
/// image_url`), pas d'en-tête d'authentification (jamais nécessaire hors du propre domaine de
/// l'API). Toute réponse non 2xx est une erreur — pas de distinction faite ici entre "objet
/// inconnu de ce CDN" et une vraie panne réseau, l'appelant traite les deux de la même façon
/// (repli sur l'icône générique).
pub fn fetch_bytes(url: &str) -> Result<Vec<u8>, SyncError> {
    let mut response = agent()
        .get(url)
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(SyncError::Http {
            status,
            path: url.to_string(),
        });
    }
    response
        .body_mut()
        .read_to_vec()
        .map_err(|err| SyncError::Network(err.to_string()))
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

/// Ce qu'`overlay-engine` a besoin de connaître du compte au démarrage — un seul
/// `GET /api/v1/settings` (voir `fetch_settings`) suffit aux deux, `roster` et `watchlist` étant
/// deux clés du même objet `data` (voir `functions/api/v1/settings.ts`, dépôt `wakfu-companion`).
pub struct AccountSettings {
    pub roster: RosterIndex,
    /// Liste des entrées suivies — définitions SEULEMENT (nom/genre/mode/cible), lues en lecture
    /// seule depuis le compte comme le roster. Les compteurs (`count`) eux-mêmes restent locaux à
    /// l'overlay pour cette première version (voir `overlay_engine::watchlist`, décision
    /// utilisateur 2026-09-01, `docs/plan-architecture.md` §14 point 3) : `Engine::
    /// set_watchlist_entries` écrase le `count` de chaque entrée reçue ici par le compteur local
    /// déjà en cours, s'il existe.
    pub watchlist: Vec<WatchlistEntry>,
    /// Objets à son activé au ramassage (`data.profile.soundItems`, voir
    /// `overlay_engine::profile`) — INDÉPENDANT de `watchlist` : un objet peut avoir son son
    /// activé sans être suivi, et réciproquement. Lu en lecture seule comme le reste de cette
    /// structure, jamais réécrit par l'overlay.
    pub sound_items: Vec<SoundItemEntry>,
}

/// `GET /api/v1/auth/me` avec `Authorization: Bearer <token>` — seule source de l'`uid` requis par
/// `overlay_sync::queue::client_key` (L5, §7.1 du plan) : `AuthService.uid` côté web vient du
/// cookie de session, indisponible ici (voir §7.2, appairage natif) ; le serveur expose la même
/// information (`user.id`) via cet endpoint, qui accepte déjà `Authorization: Bearer` comme
/// `/settings` (même middleware `_auth.ts`). Résolu une fois par connexion réussie
/// (`attempt_connect`), jamais recalculé par événement.
pub fn fetch_account_id(token: &str) -> Result<String, SyncError> {
    let url = format!("{}/api/v1/auth/me", base_url());
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
            path: "/api/v1/auth/me".to_string(),
        });
    }
    body.get("user")
        .and_then(|user| user.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| SyncError::Json("champ user.id absent de /api/v1/auth/me".into()))
}

/// `GET /api/v1/settings` avec `Authorization: Bearer <token>` — voir `functions/api/_auth.ts`
/// (dépôt `wakfu-companion`) pour l'acceptation du porteur en plus du cookie. `RosterIndex::
/// from_settings_json`/`watchlist_from_settings_json` attendent l'objet `data` de cette réponse,
/// pas la réponse entière — voir leur doc respective.
pub fn fetch_settings(token: &str) -> Result<AccountSettings, SyncError> {
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
    Ok(AccountSettings {
        roster: RosterIndex::from_settings_json(&data),
        watchlist: watchlist_from_settings_json(&data),
        sound_items: sound_items_from_settings_json(&data),
    })
}

/// `GET /api/v1/catalog/version` — juste assez pour détecter un changement (voir
/// `functions/api/v1/catalog/version.ts`, dépôt `wakfu-companion`) : `indexHash`, une empreinte du
/// contenu réellement servi par `fetch_catalog_index`, à comparer à celle du cache disque avant de
/// retélécharger ~350 Ko pour rien (voir §7.4 du plan).
pub fn fetch_catalog_version() -> Result<String, SyncError> {
    let url = format!("{}/api/v1/catalog/version", base_url());
    let response = agent()
        .get(&url)
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    let body = parse_json_body("/api/v1/catalog/version", response)?;
    body.get("indexHash")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| SyncError::Json("champ indexHash absent de /api/v1/catalog/version".into()))
}

/// `GET /api/v1/catalog/` — index compact objets+monstres tel quel (voir
/// `overlay_engine::CatalogIndex::from_compact_json`, qui en attend exactement cette forme :
/// `{ items: [...], monsters: [...] }`, tuples positionnels — voir `server/catalog/
/// compact-index.ts` côté `wakfu-companion` pour le format exact). ~1,14 Mo bruts / ~348 Ko gzip
/// mesurés côté serveur — jamais appelé sans avoir d'abord comparé `fetch_catalog_version` au
/// cache disque (voir `catalog_cache.rs`).
pub fn fetch_catalog_index() -> Result<Value, SyncError> {
    let url = format!("{}/api/v1/catalog/", base_url());
    let response = agent()
        .get(&url)
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body("/api/v1/catalog/", response)
}

/// `GET /api/v1/dungeons` — liste complète des donjons (voir `overlay_engine::DungeonIndex::
/// from_json`, qui en attend exactement cette forme : un tableau d'objets, pas de tuples compacts,
/// voir `functions/api/v1/dungeons.ts` côté `wakfu-companion`). Pas de endpoint `/version` séparé
/// pour ce référentiel (voir `reference_data_cache.rs`) — toujours rechargé en entier.
pub fn fetch_dungeons() -> Result<Value, SyncError> {
    let url = format!("{}/api/v1/dungeons", base_url());
    let response = agent()
        .get(&url)
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body("/api/v1/dungeons", response)
}

/// `GET /api/v1/monster-families` — miroir de `fetch_dungeons` pour les familles de monstres (voir
/// `overlay_engine::MonsterFamilyIndex::from_json`).
pub fn fetch_monster_families() -> Result<Value, SyncError> {
    let url = format!("{}/api/v1/monster-families", base_url());
    let response = agent()
        .get(&url)
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body("/api/v1/monster-families", response)
}
