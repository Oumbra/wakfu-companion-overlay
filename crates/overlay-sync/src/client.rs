//! Petit client HTTP bloquant (`ureq`, rustls) — voir `docs/plan-architecture.md` §7.3 pour le
//! choix définitif de rester en `ureq` plutôt que `reqwest`+`tokio`, y compris pour la file d'envoi
//! (L5, `queue.rs`) : chaque thread réseau (auth, catalogue, sync) reste un `std::thread` bloquant
//! dédié, jamais sur le chemin chaud du rendu — un seul modèle de concurrence dans tout le binaire.

use std::time::Duration;

use overlay_engine::{
    chat_filters_from_account_data, chat_filters_patch_entry, profile_patch_entry,
    roster_patch_entry, watchlist_from_settings_json, watchlist_patch_entry, AlertProfile,
    ChatFilter, Roster, RosterIndex, RosterPatch, WatchlistEntry,
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
///
/// **Schéma imposé** (audit de sécurité du 2026-09-23), comme `WAKFU_OVERLAY_UPDATE_URL` :
/// `https://`, ou `http://` vers la boucle locale (`wrangler pages dev`) seulement. Une origine
/// `http://` distante ferait voyager en clair le jeton porteur et tout l'historique. Valeur
/// refusée : ignorée (un `warn!` au premier appel), l'overlay parle à son déploiement par défaut.
pub fn base_url_override() -> Option<String> {
    let value = std::env::var("WAKFU_COMPANION_API_URL").ok()?;
    if crate::update::override_allowed(&value) {
        return Some(value);
    }
    static WARNED: std::sync::Once = std::sync::Once::new();
    WARNED.call_once(|| {
        tracing::warn!(
            "WAKFU_COMPANION_API_URL ignorée : seul https:// (ou http:// vers la boucle locale) est accepté"
        );
    });
    None
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

/// `PATCH /api/v1/settings` d'une seule entrée — le tronc commun des quatre `patch_*` ci-dessous.
/// Un lot d'une seule entrée EST l'écriture par clé (pas de route `/settings/{key}` séparée côté
/// serveur, `server/README.md` lot 6).
///
/// La réponse est un 200 même quand la clé est **refusée** (`rejected: [{ key, remoteUpdatedAt,
/// value }]`) : le compte portait une version plus récente — écrite depuis le site après notre
/// horodatage, ou, pour un correctif (`patch`, clés `profile` et `roster`), entre la lecture du
/// serveur et son écriture (compare-and-set, `functions/api/v1/settings.ts::onRequestPatch`). Le
/// refus est **journalisé, pas rejoué** : le réglage reste appliqué localement, le compte garde sa
/// version, et le prochain `GET` (prochain lancement, ou « Réessayer ») réaligne l'overlay dessus.
/// Rejouer le correctif sur la version fraîche reviendrait à faire gagner l'écriture la plus
/// ancienne — l'inverse de l'arbitrage.
fn patch_settings(token: &str, entry: Value) -> Result<Value, SyncError> {
    let key = entry
        .get("key")
        .and_then(Value::as_str)
        .unwrap_or("?")
        .to_string();
    let body = serde_json::json!({ "entries": [entry] });
    let response = patch_json_authenticated(token, "/api/v1/settings", &body)?;
    if let Some(rejected) = response.get("rejected").and_then(Value::as_array) {
        for refus in rejected {
            let remote_updated_at = refus
                .get("remoteUpdatedAt")
                .and_then(Value::as_str)
                .unwrap_or("?");
            tracing::warn!(
                key = %key,
                remote_updated_at,
                "[compte] écriture refusée — le compte porte une version plus récente, conservée"
            );
        }
    }
    Ok(response)
}

/// `PATCH /api/v1/settings` pour répliquer les compteurs de Suivi (watchlist) vers le compte —
/// voir `overlay_engine::watchlist::watchlist_patch_entry` pour le format de l'entrée envoyée, et
/// `docs/plan-architecture.md` §14 point 3 (chantier fermé le 2026-09-07, retour utilisateur : un
/// Suivi jamais visible sur le site). Valeur entière : une liste plate n'a pas de sous-clé à
/// fusionner.
pub fn patch_watchlist(token: &str, entries: &[WatchlistEntry]) -> Result<Value, SyncError> {
    patch_settings(token, watchlist_patch_entry(entries))
}

/// `PATCH /api/v1/settings` pour écrire la clé `profile` — les alertes de ramassage réglées dans
/// l'onglet « Alertes » de la fenêtre Options (2026-09-12).
///
/// **Écriture partielle depuis le 2026-09-19** (constat C9 de `docs/analyse-rgpd.md`) : `fields`
/// est le correctif produit par `AlertProfile::patch_fields` — les trois champs d'alerte, rien
/// d'autre — et le serveur le fusionne dans la valeur en compte (`server/settings/patch.ts`). Le
/// pseudo, l'avatar et le mode d'affichage des personnages, que l'overlay n'affiche nulle part,
/// ne transitent plus par lui ; jusque-là il devait renvoyer l'objet entier reçu au `GET` pour ne
/// pas les effacer.
///
/// L'horodatage est celui du poste. L'arbitrage serveur est « dernier écrivain gagne » sur cette
/// clé (`server/settings/merge.ts`) : une horloge locale en retard fait perdre l'écriture, ce qui
/// est le comportement voulu — mieux vaut refuser que régresser une modification plus récente
/// faite depuis le site (voir [`patch_settings`] pour ce que devient un refus).
pub fn patch_profile(token: &str, fields: &Value) -> Result<Value, SyncError> {
    patch_settings(token, profile_patch_entry(fields))
}

/// `PATCH /api/v1/settings` pour écrire la clé `chatFilters` — les recherches réglées dans
/// l'onglet « Chat » de la fenêtre Options (2026-09-13). Même forme que `patch_watchlist` : la clé
/// n'appartient qu'aux recherches, sa valeur est remplacée en entier, rien à préserver. Même
/// arbitrage serveur « dernier écrivain gagne » que `patch_profile`.
pub fn patch_chat_filters(token: &str, filters: &[ChatFilter]) -> Result<Value, SyncError> {
    patch_settings(token, chat_filters_patch_entry(filters))
}

/// `PATCH /api/v1/settings` pour écrire la clé `roster` — les comptes et leurs personnages tels
/// que l'onglet « Personnages » de la fenêtre Options les a édités (2026-09-16).
///
/// **Écriture partielle depuis le 2026-09-19** (constat C9) : `patch` est l'écart entre le roster
/// édité et celui que le compte avait renvoyé (`Roster::patch_against`) — les seuls comptes
/// modifiés ou créés, entiers, et les identifiants des comptes retirés ; le serveur fusionne par
/// `id`. Un correctif vide est refusé en 400 : l'appelant teste `RosterPatch::is_empty` avant.
/// Jusque-là le roster partait entier, tel que lu, pour ne rien effacer du compte — voir
/// `overlay_engine::roster`, doc de module. Même arbitrage serveur que `patch_profile`.
pub fn patch_roster(token: &str, patch: &RosterPatch) -> Result<Value, SyncError> {
    patch_settings(token, roster_patch_entry(patch))
}

/// Jeton à poser sur une route **réservée à l'overlay** — catalogue, objets, monstres, donjons,
/// familles, icônes (2026-09-20, voir la doc de tête de `session.rs`) : côté serveur, ces routes
/// ne répondent plus qu'au site (`Sec-Fetch-Site: same-origin`) et à une session valide, 403
/// sinon. `NoSession` tant que le thread Auth n'a rien publié : l'appelant garde son cache.
fn session_token() -> Result<String, SyncError> {
    crate::session::current().ok_or(SyncError::NoSession)
}

/// `GET /api/v1/items/{id}` — le détail d'un objet, dont **sa recette** (`functions/api/v1/
/// items/[id].ts` côté dépôt web).
///
/// C'est le seul endroit où les ingrédients d'une recette sont disponibles : l'index compact du
/// catalogue (`fetch_catalog_index`) n'en porte que le drapeau `hasRecipe`, volontairement — les
/// embarquer alourdirait un index que tout l'overlay charge au démarrage, pour un besoin qui ne
/// concerne qu'un dialogue.
///
/// Avec la session courante (voir [`session_token`]) : route réservée à l'overlay et au site.
pub fn fetch_item_detail(id: i64) -> Result<Value, SyncError> {
    let token = session_token()?;
    let path = format!("/api/v1/items/{id}");
    let url = format!("{}{path}", base_url());
    let response = agent()
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body(&path, response)
}

/// L'URL d'une icône `wakassets` **relayée par l'API** — `GET /api/v1/icons/{folder}/{gfxId}.png`
/// sur [`base_url`], pour un chemin `items/1234.png` (`overlay_engine::catalog::IconRef::
/// image_paths`). Depuis le 2026-09-19 (constat C10 de `docs/analyse-rgpd.md`) l'overlay ne
/// contacte plus `vertylo.github.io` : c'est le service, qui connaît déjà le compte, qui va
/// chercher l'image et la garde en cache à sa périphérie.
pub fn icon_url(path: &str) -> String {
    format!(
        "{}/api/v1/icons/{}",
        base_url().trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

/// Récupère les octets bruts d'une URL absolue (les icônes, voir [`icon_url`]), avec la session
/// courante (voir [`session_token`] — le relais d'icônes est réservé à l'overlay et au site
/// depuis le 2026-09-20). Toute réponse non 2xx est une erreur — pas de distinction faite ici
/// entre "icône inconnue" et une vraie panne réseau, l'appelant traite les deux de la même façon
/// (repli sur l'icône générique).
pub fn fetch_bytes(url: &str) -> Result<Vec<u8>, SyncError> {
    let token = session_token()?;
    let response = agent()
        .get(url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    let mut response = ensure_success(url, response)?;
    response
        .body_mut()
        .read_to_vec()
        .map_err(|err| SyncError::Network(err.to_string()))
}

fn parse_json_body(
    path: &str,
    response: ureq::http::Response<ureq::Body>,
) -> Result<Value, SyncError> {
    // Statut AVANT le corps : une réponse d'erreur n'est pas forcément du JSON (page HTML d'un
    // proxy, corps vide), et un 401 doit remonter comme `Http`, jamais comme `Json`.
    let mut response = ensure_success(path, response)?;
    // **Constat S6 (2026-09-23)** : quota de l'hébergeur épuisé, le site passe en « fail open » et
    // TOUTE URL `/api/*` répond `200 text/html` (l'`index.html` de la SPA). Un 2xx qui n'annonce
    // pas du JSON n'est donc pas une réponse de l'API : `SyncError::Json`, que la file d'envoi
    // traite comme une panne réessayable (`queue::Rejection::Transient`) — jamais comme un succès
    // qui retirerait un lot, ni comme un refus définitif.
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if !is_json_content_type(content_type) {
        return Err(SyncError::Json(format!(
            "réponse 2xx non JSON pour {path} (content-type « {} »)",
            content_type.chars().take(64).collect::<String>()
        )));
    }
    response
        .body_mut()
        .read_json()
        .map_err(|err| SyncError::Json(err.to_string()))
}

/// `application/json`, ou un type `application/*+json` (paramètres comme `; charset=utf-8`
/// ignorés, casse indifférente).
fn is_json_content_type(value: &str) -> bool {
    let essence = value
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    essence == "application/json"
        || (essence.starts_with("application/") && essence.ends_with("+json"))
}

/// Plafond d'un `Retry-After` honoré (2026-09-23) : au-delà, la file réessaie quand même au bout
/// d'une heure — un en-tête aberrant (proxy, horloge serveur décalée) ne doit pas geler l'envoi
/// pour la journée.
pub const MAX_RETRY_AFTER: Duration = Duration::from_secs(3600);

/// Taille maximale lue du corps d'une réponse d'erreur, pour y chercher son `code` : une page HTML
/// de proxy n'a pas à être chargée en entier.
const ERROR_BODY_LIMIT: u64 = 16 * 1024;

/// Rend la réponse telle quelle si son statut est 2xx, sinon `SyncError::Http` — avec le `code`
/// du corps JSON d'erreur (`{ "error": "...", "code": "history_quota_exceeded" }`, voir
/// `server/history/guards.ts` et `functions/api/_auth.ts` côté web) et l'en-tête `Retry-After`
/// (429, `server/http/api-guards.ts`). Le seul endroit qui construit une `SyncError::Http` : toutes
/// les routes classent leurs refus de la même façon.
fn ensure_success(
    path: &str,
    mut response: ureq::http::Response<ureq::Body>,
) -> Result<ureq::http::Response<ureq::Body>, SyncError> {
    let status = response.status().as_u16();
    if (200..300).contains(&status) {
        return Ok(response);
    }
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| parse_retry_after(value, now_ms()));
    let code = response
        .body_mut()
        .with_config()
        .limit(ERROR_BODY_LIMIT)
        .read_to_string()
        .ok()
        .and_then(|body| error_code(&body));
    Err(SyncError::Http {
        status,
        path: path.to_string(),
        code,
        retry_after,
    })
}

/// Le champ `code` d'un corps d'erreur JSON — `None` si le corps n'est pas du JSON ou n'en porte
/// pas (ancienne version du serveur, page d'un proxy).
fn error_code(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get("code")?
        .as_str()
        .map(str::to_string)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// **`Retry-After` interprété** (RFC 9110 §10.2.3) : un nombre de secondes, ou une date HTTP
/// (`Sun, 06 Nov 1994 08:49:37 GMT`, seule forme qu'un serveur doit produire — les formes
/// obsolètes RFC 850/asctime rendent `None`, et la file retombe sur son backoff ordinaire).
/// Borné à `[1 s, MAX_RETRY_AFTER]` : zéro ou une date passée ne doit pas faire boucler l'envoi
/// sans pause, une valeur démesurée ne doit pas le suspendre des jours.
pub fn parse_retry_after(value: &str, now_ms: i64) -> Option<Duration> {
    let value = value.trim();
    let delay = if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
        // Au-delà de u64 : forcément plus que le plafond.
        Duration::from_secs(value.parse::<u64>().unwrap_or(u64::MAX))
    } else {
        let at = parse_http_date_ms(value)?;
        Duration::from_millis(u64::try_from(at - now_ms).unwrap_or(0))
    };
    Some(delay.clamp(Duration::from_secs(1), MAX_RETRY_AFTER))
}

/// IMF-fixdate (`Sun, 06 Nov 1994 08:49:37 GMT`) → ms époque Unix.
fn parse_http_date_ms(value: &str) -> Option<i64> {
    let parts: Vec<&str> = value.split_ascii_whitespace().collect();
    let [weekday, day, month, year, time, "GMT"] = parts.as_slice() else {
        return None;
    };
    if !weekday.ends_with(',') {
        return None;
    }
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month = MONTHS.iter().position(|m| m == month)? as i64 + 1;
    let digits = |s: &str, len: usize| -> Option<i64> {
        if s.len() != len || !s.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        s.parse().ok()
    };
    let day = digits(day, 2)?;
    let year = digits(year, 4)?;
    let mut hms = time.split(':');
    let (h, m, s) = (
        digits(hms.next()?, 2)?,
        digits(hms.next()?, 2)?,
        digits(hms.next()?, 2)?,
    );
    if hms.next().is_some() || !(1..=31).contains(&day) || h > 23 || m > 59 || s > 60 {
        return None;
    }
    Some(overlay_engine::log_time::utc_ms_from_civil(
        year,
        month,
        day,
        ((h * 60 + m) * 60 + s) * 1000,
    ))
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
    let response = agent()
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    let body = parse_json_body("/api/v1/auth/me", response)?;
    body.get("user")
        .and_then(|user| user.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| SyncError::Json("champ user.id absent de /api/v1/auth/me".into()))
}

/// Ce que `POST /api/v1/auth/native/session` rend — voir [`rotate_native_session`]. Les dates sont
/// laissées telles que le serveur les écrit (ISO 8601) : elles ne servent qu'au journal, l'âge du
/// jeton se mesure à la date de sauvegarde (`token_store::token_age`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotatedSession {
    pub token: String,
    pub expires_at: String,
    /// Jusqu'à quand l'ancien jeton reste accepté — 5 min après l'appel
    /// (`NATIVE_SESSION_ROTATION_GRACE_MS`, `server/auth/pairing.ts`), le temps que le nouveau
    /// soit persisté et que les requêtes déjà parties aboutissent.
    pub previous_token_valid_until: String,
}

/// **`POST /api/v1/auth/native/session` — renouvelle le jeton** (2026-09-19, constat C5 de
/// `docs/analyse-rgpd.md` §3.5 : un jeton de 30 jours glissants, jamais renouvelé, restait le
/// même pendant toute la vie d'une session — des mois).
///
/// Le serveur émet un jeton neuf de 30 jours glissants pour le même compte et le même appareil,
/// et **remplace** l'ancienne session (`sessions.superseded_at`) : plus listée dans « Mon compte »,
/// plus jamais prolongée, acceptée encore 5 min (voir `RotatedSession::previous_token_valid_until`)
/// puis refusée. Le rythme appartient à l'overlay — le serveur ne force rien —, c'est
/// `background::attempt_connect` qui le fixe (au démarrage, jeton de plus de 7 jours).
///
/// **À l'appelant de persister le nouveau jeton AVANT de s'en servir** (`token_store::save_token`),
/// et de ne jamais traiter comme un jeton refusé un 401 survenu pendant la grâce : c'est une
/// course, pas une révocation. Une rotation depuis un jeton déjà remplacé mais encore en grâce est
/// admise côté serveur (plantage entre la réponse et l'écriture au trousseau).
pub fn rotate_native_session(token: &str) -> Result<RotatedSession, SyncError> {
    const PATH: &str = "/api/v1/auth/native/session";
    let url = format!("{}{PATH}", base_url());
    let response = agent()
        .post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send_empty()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    let body = parse_json_body(PATH, response)?;
    let champ = |nom: &str| {
        body.get(nom)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| SyncError::Json(format!("champ {nom} absent de {PATH}")))
    };
    Ok(RotatedSession {
        token: champ("token")?,
        expires_at: champ("expiresAt")?,
        previous_token_valid_until: champ("previousTokenValidUntil")?,
    })
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
    let response = agent()
        .delete(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    let body = parse_json_body(PATH, response)?;
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
    let response = agent()
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    let body = parse_json_body("/api/v1/settings", response)?;
    let data = body.get("data").cloned().unwrap_or(Value::Null);
    Ok(AccountSettings {
        roster: RosterIndex::from_settings_json(&data),
        roster_draft: Roster::from_settings_json(&data),
        watchlist: watchlist_from_settings_json(&data),
        alerts: AlertProfile::from_settings_json(&data),
        chat_filters: chat_filters_from_account_data(&data),
    })
}

/// `GET /api/v1/catalog/version` — juste assez pour détecter un changement (voir
/// `functions/api/v1/catalog/version.ts`, dépôt `wakfu-companion`) : `indexHash`, une empreinte du
/// contenu réellement servi par `fetch_catalog_index`, à comparer à celle du cache disque avant de
/// retélécharger ~350 Ko pour rien (voir §7.4 du plan).
pub fn fetch_catalog_version() -> Result<String, SyncError> {
    let token = session_token()?;
    let url = format!("{}/api/v1/catalog/version", base_url());
    let response = agent()
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
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
    let token = session_token()?;
    let url = format!("{}/api/v1/catalog/", base_url());
    let response = agent()
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body("/api/v1/catalog/", response)
}

/// `GET /api/v1/dungeons` — liste complète des donjons (voir `overlay_engine::DungeonIndex::
/// from_json`, qui en attend exactement cette forme : un tableau d'objets, pas de tuples compacts,
/// voir `functions/api/v1/dungeons.ts` côté `wakfu-companion`). Pas de endpoint `/version` séparé
/// pour ce référentiel (voir `reference_data_cache.rs`) — toujours rechargé en entier.
pub fn fetch_dungeons() -> Result<Value, SyncError> {
    let token = session_token()?;
    let url = format!("{}/api/v1/dungeons", base_url());
    let response = agent()
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body("/api/v1/dungeons", response)
}

/// `GET /api/v1/monster-families` — miroir de `fetch_dungeons` pour les familles de monstres (voir
/// `overlay_engine::MonsterFamilyIndex::from_json`).
pub fn fetch_monster_families() -> Result<Value, SyncError> {
    let token = session_token()?;
    let url = format!("{}/api/v1/monster-families", base_url());
    let response = agent()
        .get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|err| SyncError::Network(err.to_string()))?;
    parse_json_body("/api/v1/monster-families", response)
}

/// `GET /api/v1/game-servers` — les serveurs de jeu (`game_servers`, voir
/// `functions/api/v1/game-servers.ts` côté `wakfu-companion`), **jamais une liste en dur** : c'est
/// la règle du dépôt web, et le code écrit dans `roster[].gameServer` doit être un `code` de cette
/// table pour que le site le reconnaisse.
///
/// Sans authentification (contrairement au catalogue et aux donjons depuis le 2026-09-20 : cette
/// table n'est pas une donnée du jeu sous licence) — minuscule et quasi statique, mise en cache
/// disque au même titre (`reference_data_cache::ReferenceData::GameServers`).
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
    use super::{
        error_code, is_json_content_type, parse_retry_after, DEFAULT_BASE_URL, MAX_RETRY_AFTER,
    };
    use std::time::Duration;

    /// 1994-11-06T08:49:37Z — l'exemple de la RFC 9110.
    const RFC_EXAMPLE_MS: i64 = 784_111_777_000;

    #[test]
    fn retry_after_en_secondes() {
        assert_eq!(parse_retry_after("120", 0), Some(Duration::from_secs(120)));
        assert_eq!(parse_retry_after(" 7 ", 0), Some(Duration::from_secs(7)));
    }

    #[test]
    fn retry_after_borne_entre_une_seconde_et_une_heure() {
        assert_eq!(parse_retry_after("0", 0), Some(Duration::from_secs(1)));
        assert_eq!(parse_retry_after("86400", 0), Some(MAX_RETRY_AFTER));
        assert_eq!(
            parse_retry_after("99999999999999999999999", 0),
            Some(MAX_RETRY_AFTER)
        );
    }

    #[test]
    fn retry_after_en_date_http() {
        let header = "Sun, 06 Nov 1994 08:49:37 GMT";
        assert_eq!(
            parse_retry_after(header, RFC_EXAMPLE_MS - 90_000),
            Some(Duration::from_secs(90))
        );
        // Date déjà passée : réessai au plus tôt, mais jamais sans pause.
        assert_eq!(
            parse_retry_after(header, RFC_EXAMPLE_MS + 5_000),
            Some(Duration::from_secs(1))
        );
        // Lointaine : plafonnée.
        assert_eq!(parse_retry_after(header, 0), Some(MAX_RETRY_AFTER));
    }

    #[test]
    fn retry_after_illisible() {
        for value in [
            "",
            "demain",
            "-5",
            "1.5",
            "Sunday, 06-Nov-94 08:49:37 GMT",
            "Sun Nov  6 08:49:37 1994",
            "Sun, 06 Foo 1994 08:49:37 GMT",
            "Sun, 06 Nov 1994 08:49:37 UTC",
        ] {
            assert_eq!(parse_retry_after(value, 0), None, "{value}");
        }
    }

    #[test]
    fn code_d_erreur_lu_dans_le_corps_json() {
        assert_eq!(
            error_code(r#"{"error":"quota","code":"history_quota_exceeded"}"#).as_deref(),
            Some("history_quota_exceeded")
        );
        assert_eq!(error_code(r#"{"error":"non authentifié"}"#), None);
        assert_eq!(error_code("<html>502</html>"), None);
        assert_eq!(error_code(""), None);
    }

    /// S6 : seul un type JSON est une réponse de l'API — la page HTML d'une SPA servie en 200
    /// (hébergeur en « fail open ») ne l'est pas.
    #[test]
    fn seul_un_content_type_json_est_accepte() {
        for ok in [
            "application/json",
            "application/json; charset=utf-8",
            "Application/JSON",
            "application/problem+json",
        ] {
            assert!(is_json_content_type(ok), "{ok}");
        }
        for ko in [
            "",
            "text/html",
            "text/html; charset=utf-8",
            "text/plain",
            "application/jsonp",
            "text/json+html",
        ] {
            assert!(!is_json_content_type(ko), "{ko}");
        }
    }

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
