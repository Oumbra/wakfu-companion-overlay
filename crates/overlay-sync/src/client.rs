//! Petit client HTTP bloquant (`ureq`, rustls) — voir `docs/plan-architecture.md` §7.3 pour le
//! choix définitif de rester en `ureq` plutôt que `reqwest`+`tokio`, y compris pour la file d'envoi
//! (L5, `queue.rs`) : chaque thread réseau (auth, catalogue, sync) reste un `std::thread` bloquant
//! dédié, jamais sur le chemin chaud du rendu — un seul modèle de concurrence dans tout le binaire.

use std::time::Duration;

use overlay_engine::{
    chat_filters_from_account_data, chat_filters_patch_entry, profile_patch_entry,
    roster_patch_entry, watchlist_from_settings_json, watchlist_patch_entry, AlertProfile,
    ChatFilter, Roster, RosterIndex, WatchlistEntry,
};
use serde_json::Value;

use crate::SyncError;

// **Origine de l'API figée à la compilation, d'après le profil** (voir `build.rs` du crate) :
// `release` → prod (`https://wakfu-companion.com`), tout autre profil (`preview`, `debug`, tests)
// → déploiement dev (`https://claude-dev.wakfu-companion.com`). Un binaire de Release vise donc
// TOUJOURS la prod (décision du mainteneur, 2026-09-15, docs/plan-mise-a-jour.md §10 point 6), et
// un exe de preview TOUJOURS dev, quelle que soit la façon dont il est lancé.
//
// Historique : jusqu'au 2026-09-15 la constante pointait sur le domaine dev, parce que les routes
// d'appairage natif (`/api/v1/auth/native/*`) n'y étaient déployées qu'en preview ; jusqu'au
// 2026-09-17 elle valait la prod en dur, et seuls les scripts `preview.{ps1,sh}` posaient
// `WAKFU_COMPANION_API_URL` — un `target/preview/overlay-ui.exe` lancé hors script visait la prod.
// Tant que l'appairage natif n'est pas déployé en prod, un binaire `release` ne peut pas se
// connecter — c'est une condition de sortie de la première Release, pas une raison de repointer.
const DEFAULT_BASE_URL: &str = env!("WAKFU_COMPANION_API_DEFAULT_URL");
const TIMEOUT: Duration = Duration::from_secs(10);

/// Origine de l'API — `WAKFU_COMPANION_API_URL` en priorité (utile contre un `wrangler pages dev`
/// local), repli sur `DEFAULT_BASE_URL` ci-dessus (choisie par le profil de compilation). Jamais
/// codée en dur sans repli overridable. Tracée une fois au démarrage par `overlay-ui` (`main.rs`),
/// pour que le journal dise à quel déploiement une session a parlé.
pub fn base_url() -> String {
    base_url_override().unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
}

/// L'origine **surchargée** par `WAKFU_COMPANION_API_URL`, si elle l'est — `None` quand l'overlay
/// parle au déploiement de son profil de compilation. Sert à le dire à l'utilisateur (« À propos »,
/// constat C16 de `docs/analyse-rgpd.md`, 2026-09-19) : c'est là que partent ses données, il doit
/// pouvoir le voir sans ouvrir le journal.
pub fn base_url_override() -> Option<String> {
    std::env::var("WAKFU_COMPANION_API_URL").ok()
}

/// **Un seul agent pour tout le processus**, cloné à chaque appel (`ureq::Agent` est un `Arc`
/// sous le capot) — et non un agent neuf par requête, comme jusqu'au 2026-09-12.
///
/// Un agent porte le réservoir de connexions : le reconstruire à chaque appel rouvrait une
/// connexion TLS complète par icône téléchargée. Mesuré sur `wakassets` : une icône toutes les
/// ~145 ms, en série — cent résultats d'autocomplétion mettaient quinze secondes à s'illustrer.
fn agent() -> ureq::Agent {
    static AGENT: std::sync::OnceLock<ureq::Agent> = std::sync::OnceLock::new();
    AGENT
        .get_or_init(|| {
            ureq::Agent::config_builder()
                .timeout_global(Some(TIMEOUT))
                // Un statut 4xx/5xx est une RÉPONSE, pas une panne : chaque fonction ci-dessous
                // lit `response.status()` et produit `SyncError::Http { status, path }`. Avec le
                // défaut de `ureq` (`true`), `.call()` rendait `Err(StatusCode(401))`, converti en
                // `SyncError::Network("http status: 401")` — et ces contrôles de statut étaient du
                // code mort. Conséquence vécue le 2026-09-17 : un jeton refusé (401) passait pour
                // « serveur injoignable », était conservé, et « Réessayer » bouclait sur le même
                // refus au lieu de relancer l'appairage (`background::attempt_connect`) ; de même
                // `queue::is_permanent_rejection` ne voyait jamais un 4xx.
                .http_status_as_error(false)
                .build()
                .into()
        })
        .clone()
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

/// `PATCH /api/v1/settings` pour écrire la clé `profile` — les alertes de ramassage réglées dans
/// l'onglet « Alertes » de la fenêtre Options (2026-09-12).
///
/// **`profile` doit être l'objet ENTIER**, pas les seuls champs d'alerte : le serveur remplace la
/// valeur de la clé, il ne fusionne pas. C'est `AlertProfile::patch_value` qui le construit, à
/// partir de l'objet reçu au `GET` (`AccountSettings::profile_raw`) — l'appeler autrement
/// effacerait le pseudo et l'avatar du compte.
///
/// L'horodatage est celui du poste. L'arbitrage serveur est « dernier écrivain gagne » sur cette
/// valeur (`server/settings/merge.ts`) : une horloge locale en retard fait perdre l'écriture, ce
/// qui est le comportement voulu — mieux vaut refuser que régresser une modification plus récente
/// faite depuis le site.
pub fn patch_profile(token: &str, profile: &Value) -> Result<Value, SyncError> {
    let body = serde_json::json!({ "entries": [profile_patch_entry(profile)] });
    patch_json_authenticated(token, "/api/v1/settings", &body)
}

/// `PATCH /api/v1/settings` pour écrire la clé `chatFilters` — les recherches réglées dans
/// l'onglet « Chat » de la fenêtre Options (2026-09-13). Même forme que `patch_watchlist` : la clé
/// n'appartient qu'aux recherches, sa valeur est remplacée en entier, rien à préserver. Même
/// arbitrage serveur « dernier écrivain gagne » que `patch_profile`.
pub fn patch_chat_filters(token: &str, filters: &[ChatFilter]) -> Result<Value, SyncError> {
    let body = serde_json::json!({ "entries": [chat_filters_patch_entry(filters)] });
    patch_json_authenticated(token, "/api/v1/settings", &body)
}

/// `PATCH /api/v1/settings` pour écrire la clé `roster` — les comptes et leurs personnages tels
/// que l'onglet « Personnages » de la fenêtre Options les a édités (2026-09-16).
///
/// **Le roster doit être celui qui a été LU**, modifié, pas un reconstruit : la clé porte l'identité
/// des comptes (`id`, `label`, `isDefault`) et des champs que l'overlay n'affiche nulle part — voir
/// `overlay_engine::roster`, doc de module, qui explique ce qu'une réécriture appauvrissante
/// coûterait au compte. Même arbitrage serveur « dernier écrivain gagne » que `patch_profile`.
pub fn patch_roster(token: &str, roster: &Roster) -> Result<Value, SyncError> {
    let body = serde_json::json!({ "entries": [roster_patch_entry(roster)] });
    patch_json_authenticated(token, "/api/v1/settings", &body)
}

/// `GET /api/v1/items/{id}` — le détail d'un objet, dont **sa recette** (`functions/api/v1/
/// items/[id].ts` côté dépôt web).
///
/// C'est le seul endroit où les ingrédients d'une recette sont disponibles : l'index compact du
/// catalogue (`fetch_catalog_index`) n'en porte que le drapeau `hasRecipe`, volontairement — les
/// embarquer alourdirait un index que tout l'overlay charge au démarrage, pour un besoin qui ne
/// concerne qu'un dialogue.
///
/// **Sans authentification** : ce référentiel est public, comme le catalogue et les donjons.
pub fn fetch_item_detail(id: i64) -> Result<Value, SyncError> {
    let path = format!("/api/v1/items/{id}");
    let url = format!("{}{path}", base_url());
    let response = agent()
        .get(&url)
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body(&path, response)
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
    // Statut AVANT le corps : une réponse d'erreur n'est pas forcément du JSON (page HTML d'un
    // proxy, corps vide), et un 401 doit remonter comme `Http`, jamais comme `Json`.
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(SyncError::Http {
            status,
            path: path.to_string(),
        });
    }
    response
        .body_mut()
        .read_json()
        .map_err(|err| SyncError::Json(err.to_string()))
}

/// Ce qu'`overlay-engine` a besoin de connaître du compte au démarrage — un seul
/// `GET /api/v1/settings` (voir `fetch_settings`) suffit aux deux, `roster` et `watchlist` étant
/// deux clés du même objet `data` (voir `functions/api/v1/settings.ts`, dépôt `wakfu-companion`).
pub struct AccountSettings {
    pub roster: RosterIndex,
    /// **Le même roster, sous sa forme éditable** — ce que l'onglet « Personnages » prend en
    /// brouillon à l'ouverture de la fenêtre Options, et ce que `patch_roster` réécrit.
    ///
    /// Les deux viennent du même JSON et ne peuvent pas diverger ; ils ne s'en déduisent pas l'un
    /// l'autre pour autant (voir `overlay_engine::roster`, doc de module) : l'index jette
    /// l'identité des comptes, que l'écriture ne peut pas se permettre de perdre.
    pub roster_draft: Roster,
    /// Liste des entrées suivies — définitions SEULEMENT (nom/genre/mode/cible), lues en lecture
    /// seule depuis le compte comme le roster. Les compteurs (`count`) eux-mêmes restent locaux à
    /// l'overlay pour cette première version (voir `overlay_engine::watchlist`, décision
    /// utilisateur 2026-09-01, `docs/plan-architecture.md` §14 point 3) : `Engine::
    /// set_watchlist_entries` écrase le `count` de chaque entrée reçue ici par le compteur local
    /// déjà en cours, s'il existe.
    pub watchlist: Vec<WatchlistEntry>,
    /// Alertes de ramassage — objets à son activé et réglages du toast (`data.profile`, voir
    /// `overlay_engine::profile`). INDÉPENDANT de `watchlist` : un objet peut avoir son son
    /// activé sans être suivi, et réciproquement.
    pub alerts: AlertProfile,
    /// **L'objet `profile` brut, tel que le compte l'a renvoyé** — `None` si la clé est absente.
    ///
    /// Conservé pour une seule raison, et elle est décisive : `PATCH /api/v1/settings` remplace la
    /// valeur ENTIÈRE d'une clé, et `profile` porte aussi le pseudo, l'avatar et le mode
    /// d'affichage des personnages, que l'overlay n'affiche nulle part. Réécrire les alertes sans
    /// repartir de cet objet effacerait ces champs du compte — voir
    /// `AlertProfile::patch_value`, qui le prend en entrée.
    pub profile_raw: Option<Value>,
    /// Recherches de chat (`data.chatFilters`, voir `overlay_engine::chat_alert`) — la liste que
    /// l'onglet « Chat » édite et que le moteur confronte à chaque message.
    pub chat_filters: Vec<ChatFilter>,
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
    if !(200..300).contains(&status) {
        return Err(SyncError::Http {
            status,
            path: "/api/v1/auth/me".to_string(),
        });
    }
    let body: Value = response
        .body_mut()
        .read_json()
        .map_err(|err| SyncError::Json(err.to_string()))?;
    body.get("user")
        .and_then(|user| user.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| SyncError::Json("champ user.id absent de /api/v1/auth/me".into()))
}

/// **`DELETE /api/v1/auth/native/session` — efface la session côté SERVEUR** (2026-09-18, constat
/// C5 de `docs/analyse-rgpd.md` §3.5).
///
/// Effacer le jeton de la machine ne le rendait pas invalide : il restait utilisable par qui en
/// aurait pris copie avant, et n'avait ni expiration courte ni rotation. Cette route (ajoutée le
/// même jour côté `wakfu-companion`, `functions/api/v1/auth/native/session.ts`) supprime la ligne
/// de la table `sessions` — pas seulement un `revoked_at` — et purge au passage les appairages
/// natifs périmés, qui portent un jeton en clair jusqu'au premier sondage.
///
/// **À appeler AVANT `token_store::clear_token`** : c'est le jeton qui prouve au serveur de quelle
/// session il s'agit. Une fois effacé localement, plus rien ne permet de la désigner.
///
/// Rend `deleted` : `false` si la ligne n'existait plus (appel rejoué). L'appelant traite l'échec
/// comme non bloquant — un effacement local hors ligne doit aboutir de toute façon, et le serveur
/// finira par voir la session expirer.
pub fn delete_native_session(token: &str) -> Result<bool, SyncError> {
    const PATH: &str = "/api/v1/auth/native/session";
    let url = format!("{}{PATH}", base_url());
    let mut response = agent()
        .delete(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(SyncError::Http {
            status,
            path: PATH.to_string(),
        });
    }
    let body: Value = response
        .body_mut()
        .read_json()
        .map_err(|err| SyncError::Json(err.to_string()))?;
    Ok(body
        .get("deleted")
        .and_then(Value::as_bool)
        .unwrap_or(false))
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
    if !(200..300).contains(&status) {
        return Err(SyncError::Http {
            status,
            path: "/api/v1/settings".to_string(),
        });
    }
    let body: Value = response
        .body_mut()
        .read_json()
        .map_err(|err| SyncError::Json(err.to_string()))?;
    let data = body.get("data").cloned().unwrap_or(Value::Null);
    Ok(AccountSettings {
        roster: RosterIndex::from_settings_json(&data),
        roster_draft: Roster::from_settings_json(&data),
        watchlist: watchlist_from_settings_json(&data),
        alerts: AlertProfile::from_settings_json(&data),
        profile_raw: data.get("profile").cloned(),
        chat_filters: chat_filters_from_account_data(&data),
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

/// `GET /api/v1/game-servers` — les serveurs de jeu (`game_servers`, voir
/// `functions/api/v1/game-servers.ts` côté `wakfu-companion`), **jamais une liste en dur** : c'est
/// la règle du dépôt web, et le code écrit dans `roster[].gameServer` doit être un `code` de cette
/// table pour que le site le reconnaisse.
///
/// Sans authentification, comme le catalogue et les donjons — table minuscule et quasi statique,
/// mise en cache disque au même titre (`reference_data_cache::ReferenceData::GameServers`).
pub fn fetch_game_servers() -> Result<Value, SyncError> {
    let url = format!("{}/api/v1/game-servers", base_url());
    let response = agent()
        .get(&url)
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body("/api/v1/game-servers", response)
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_BASE_URL;

    /// Le défaut compilé est l'une des deux origines connues, jamais une valeur vide ni un
    /// `http://` ; en profil de développement (`cargo test` nu, comme `ci.yml`), c'est le
    /// déploiement dev. Un profil optimisé n'est pas tranché ici : `preview` hérite de `release`
    /// (donc sans `debug_assertions`) mais vise dev, seul `build.rs` connaît le nom du profil.
    #[test]
    fn default_base_url_follows_build_profile() {
        assert!(
            DEFAULT_BASE_URL == "https://claude-dev.wakfu-companion.com"
                || DEFAULT_BASE_URL == "https://wakfu-companion.com",
            "{DEFAULT_BASE_URL}"
        );
        if cfg!(debug_assertions) {
            assert_eq!(DEFAULT_BASE_URL, "https://claude-dev.wakfu-companion.com");
        }
    }
}
