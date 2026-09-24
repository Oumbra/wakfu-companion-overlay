//! File d'envoi de l'historique — miroir Rust de `SyncQueueService` côté web (§7.3 du plan, lot
//! L5) : une table SQLite persistante (`sync_queue`), dédoublonnage naturel par `id = "{kind}:
//! {signature}"`, lots de 50, abandon d'une entrée après trop d'échecs "réessayables ne comptent
//! pas".
//!
//! **Découpage des responsabilités, volontairement différent du web sur un point** : ici,
//! `SyncQueue` ne porte AUCUN minuteur (pas de debounce, pas de backoff programmé) — seulement
//! `enqueue` (écriture idempotente) et `flush_once` (un passage d'envoi, s'arrête au premier
//! échec). Le rythme (regroupement, réessai à intervalle croissant) est piloté par l'appelant, le
//! thread Sync dédié (`overlay-ui`, voir §3/§7.3 du plan) — ce découpage rend `flush_once`
//! trivialement testable sans avoir à manipuler de vraies horloges/`sleep`, contrairement à
//! `SyncQueueService.flush`/`scheduleRetry` côté web (testés via les horloges factices de Vitest,
//! non disponibles ici).
//!
//! `flush_once` prend le transport HTTP comme paramètre (`send: impl FnMut(path, body) -> ...`)
//! plutôt que d'appeler `crate::client::post_json` en dur : permet de tester tout le protocole
//! (dédoublonnage, lots, abandon après `MAX_ATTEMPTS`, arrêt sur échec) avec un faux transport en
//! mémoire, sans jamais toucher au réseau — voir `tests` en bas de fichier.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use overlay_engine::{HistoryEventKind, HistoryPayload, SyncEvent};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::SyncError;

// Même précaution que `catalog_cache`/`icon_cache`/`reference_data_cache`/`token_store` : nom
// distinct sous `cfg(test)` pour qu'un test exécuté depuis ce crate n'écrive jamais dans le VRAI
// répertoire de données de production (`Connection::open` créerait le fichier sans broncher).
#[cfg(not(test))]
const APP_NAME: &str = "wakfu-companion-overlay";
#[cfg(test)]
const APP_NAME: &str = "wakfu-companion-overlay-test";

/// Miroir de `SYNC_BATCH_SIZE` (`sync-queue.service.ts`) — à garder ≤ `MAX_HISTORY_BATCH` côté
/// serveur (`server/history/parse.ts`).
const SYNC_BATCH_SIZE: usize = 50;
/// Miroir de `MAX_ATTEMPTS` — seuls les 4xx non classés (voir `Rejection::Counted`)
/// comptent, jamais un simple problème réseau.
const MAX_ATTEMPTS: i64 = 10;
/// Plafond des entrées retirées pour refus 400/413 **en un passage** (audit de sécurité du
/// 2026-09-23, O2). Au-delà, le passage s'arrête en réessayable : même si le contrôle
/// ([`SyncQueue::flush_once`]) se trompait, un refus généralisé du serveur ne peut jamais vider la
/// file d'un coup — au pire trois entrées par passage, backoff compris entre deux.
const MAX_SPLIT_DELETIONS_PER_PASS: usize = 3;
/// Âge au-delà duquel une entrée encore en file est abandonnée (`prune_older_than`, 2026-09-18)
/// — **sans équivalent web, et volontairement large** : la file n'accumule que ce que le serveur
/// n'a pas encore accepté, et une entrée y attend en clair, avec les pseudonymes des autres
/// combattants (`docs/analyse-rgpd.md` §3.3). Trente jours de serveur injoignable, ce n'est plus
/// une panne à rattraper : c'est un historique qu'on n'enverra jamais, et rien ne justifie de le
/// garder sur disque. En deçà, JAMAIS de purge par ancienneté — trois jours hors ligne sont trois
/// jours de combats à envoyer, pas à jeter.
pub const MAX_PENDING_AGE: std::time::Duration = std::time::Duration::from_secs(30 * 24 * 60 * 60);

/// `sha256_hex(uid|kind|signature)` — miroir exact de `computeClientKey` (`client-key.util.ts`).
/// Calculé ici (au moment de l'envoi), jamais dans `overlay_engine::history` : ce dernier ne
/// connaît pas `uid`, exactement comme `StatsStoreService` ne produit que la signature côté web
/// (le hachage vient plus tard, dans `SyncQueueService`).
pub fn client_key(uid: &str, kind: HistoryEventKind, signature: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(uid.as_bytes());
    hasher.update(b"|");
    hasher.update(kind.as_str().as_bytes());
    hasher.update(b"|");
    hasher.update(signature.as_bytes());
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Résultat d'un `flush_once` — pilote le rythme de réessai côté appelant (voir la doc de module).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlushOutcome {
    /// Rien en file au moment de l'appel (ou seulement des types suspendus, voir
    /// [`SyncQueue::blocked_kinds`]).
    Idle,
    /// Tout ce qui était en file au moment de l'appel (hors types suspendus) est parti — accepté,
    /// ou écarté définitivement par le serveur (voir [`Rejection`]).
    Synced,
    /// Un lot a échoué de façon **réessayable** (réseau, 5xx, 429, ou 4xx non classé — voir
    /// [`Rejection`]) : rien de plus tenté ce passage. `after` est le délai imposé par le serveur
    /// (`Retry-After` d'un 429, déjà borné par `client::parse_retry_after`) ; `None` : à l'appelant
    /// d'appliquer son backoff. `reason` est le `Display` de la `SyncError` — **correctif du
    /// 2026-09-03** : cet échec n'était jusqu'ici tracé nulle part côté `overlay-ui`, ce qui a
    /// laissé un vrai blocage de synchronisation (jeton non transmis, voir
    /// `post_json_authenticated`) totalement invisible pendant plusieurs jours.
    Retry {
        reason: String,
        after: Option<std::time::Duration>,
    },
    /// **401 : le jeton n'est plus accepté** (session révoquée depuis « Mon compte », expirée —
    /// plafond de 180 jours depuis l'appairage côté serveur). Rien n'est retiré de la file :
    /// l'appelant cesse d'envoyer, fait effacer le jeton et propose de se reconnecter
    /// (`overlay-ui`, `background::spawn_sync_thread`).
    Unauthorized(String),
}

/// **Classement d'un refus du serveur** (2026-09-23) — ce que `flush_once` fait du lot en échec.
/// Jusque-là tout 4xx hors 401/429 comptait une « tentative » et le lot était renvoyé à
/// l'identique toutes les 5 min, dix fois : un lot trop gros (413) ou refusé pour une seule entrée
/// invalide (400) bloquait toute la file de son type pendant près d'une heure, pour finir jeté en
/// entier — les entrées valides avec.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Rejection {
    /// 401 — voir [`FlushOutcome::Unauthorized`].
    SessionInvalid,
    /// 403 — le serveur refuse ce TYPE d'envoi, pas ce lot : quota de combats atteint
    /// (`history_quota_exceeded`, `server/history/guards.ts`), route réservée à une session de
    /// navigateur (`browser_session_required`, `functions/api/_auth.ts`), ou tout autre refus
    /// d'autorisation. Renvoyer ne changera rien avant un redémarrage ou une reconnexion : le type
    /// est **suspendu** pour la session ([`SyncQueue::blocked_kinds`]), sa file conservée, les
    /// autres types continuent.
    KindBlocked,
    /// 413 (lot trop volumineux) ou 400 (lot refusé à la validation — le serveur rejette le lot
    /// entier dès la première entrée invalide, `server/history/parse.ts::parseBatch`) : le lot est
    /// **redivisé** par moitié jusqu'à isoler l'entrée fautive, qui seule est retirée de la file —
    /// **seulement si un contrôle montre que le refus lui est propre** (audit du 2026-09-23, O2) :
    /// voir [`SyncQueue::flush_once`]. Sans ce contrôle, un 400 GÉNÉRALISÉ (serveur mal déployé,
    /// schéma changé) isolait puis supprimait chaque entrée l'une après l'autre : toute la file y
    /// passait en un seul passage.
    Split,
    /// 429 — limite de débit du compte ; `Retry-After` honoré.
    Throttled(Option<std::time::Duration>),
    /// Autre 4xx (404, 405, 409, 422…) : comportement historique, miroir de `permanent` côté web
    /// — une tentative de plus pour chaque entrée, abandon au bout de `MAX_ATTEMPTS`.
    Counted,
    /// 5xx, réseau, réponse illisible — y compris un **2xx qui n'est pas du JSON** (page HTML
    /// d'une SPA servie en « fail open » par l'hébergeur, constat S6) : réessai après backoff,
    /// aucune tentative consommée, rien retiré de la file.
    Transient,
}

fn classify(err: &SyncError) -> Rejection {
    match err {
        SyncError::Http { status: 401, .. } => Rejection::SessionInvalid,
        SyncError::Http { status: 403, .. } => Rejection::KindBlocked,
        SyncError::Http {
            status: 400 | 413, ..
        } => Rejection::Split,
        SyncError::Http {
            status: 429,
            retry_after,
            ..
        } => Rejection::Throttled(*retry_after),
        SyncError::Http { status, .. } if (400..500).contains(status) => Rejection::Counted,
        _ => Rejection::Transient,
    }
}

/// Bornes des dates d'événement acceptées par le serveur (`HISTORY_MIN_DATE_MS`/
/// `HISTORY_MAX_FUTURE_SKEW_MS`, `server/history/parse.ts`) : pas avant la sortie de Wakfu
/// (2012-01-01), pas plus d'un jour dans le futur.
const HISTORY_MIN_DATE_MS: i64 = 1_325_376_000_000;
const HISTORY_MAX_FUTURE_SKEW_MS: i64 = 24 * 60 * 60 * 1000;

/// Champ de date que le serveur borne, selon le type d'événement.
fn date_field(kind: HistoryEventKind) -> &'static str {
    match kind {
        HistoryEventKind::Fight => "startedAt",
        HistoryEventKind::Purchase | HistoryEventKind::Trade | HistoryEventKind::Pact => {
            "occurredAt"
        }
    }
}

/// La date de l'entrée est-elle dans les bornes du serveur ? Une date absente ou illisible ne
/// l'est pas. Cas réel visé : un combat restauré d'un `fight-*.json` écrit avant que
/// `started_at_ms` ne soit persisté, daté de l'époque Unix (`1970-01-01`) — le serveur refusait le
/// lot entier en 400, et le combat bloquait tous ceux de son lot.
fn date_in_server_range(kind: HistoryEventKind, payload: &Value, now_ms: i64) -> bool {
    payload
        .get(date_field(kind))
        .and_then(Value::as_str)
        .and_then(overlay_engine::log_time::parse_iso_utc_ms)
        .is_some_and(|ms| (HISTORY_MIN_DATE_MS..=now_ms + HISTORY_MAX_FUTURE_SKEW_MS).contains(&ms))
}

struct QueuedEvent {
    id: String,
    signature: String,
    payload_json: String,
    attempts: i64,
}

/// File persistante — un fichier SQLite par overlay (voir `default_store_path`), ou en mémoire
/// pour les tests (`SyncQueue::open_in_memory`).
pub struct SyncQueue {
    conn: Connection,
    /// Miroir de `consecutiveFailures` (`SyncQueueService`) — en mémoire seulement, jamais
    /// persisté : redémarrer l'overlay repart d'un backoff neuf, ce qui est correct (voir sa doc
    /// côté web, même raisonnement).
    consecutive_failures: u32,
    /// Types suspendus pour la session après un 403 (voir [`Rejection::KindBlocked`]), avec la
    /// raison. En mémoire seulement : un redémarrage ou une reconnexion ([`Self::unblock_all`])
    /// retente.
    blocked: Vec<(HistoryEventKind, String)>,
}

impl SyncQueue {
    /// Ouvre (ou crée) la file SQLite au chemin donné — crée le dossier parent si besoin.
    pub fn open(path: &Path) -> Result<Self, SyncError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| SyncError::TokenStore(err.to_string()))?;
        }
        let conn = Connection::open(path).map_err(|err| SyncError::TokenStore(err.to_string()))?;
        Self::from_connection(conn)
    }

    /// Pour les tests — aucun fichier sur disque, table éphémère.
    pub fn open_in_memory() -> Result<Self, SyncError> {
        let conn =
            Connection::open_in_memory().map_err(|err| SyncError::TokenStore(err.to_string()))?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, SyncError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sync_queue (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                signature TEXT NOT NULL,
                queued_at INTEGER NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS sync_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .map_err(|err| SyncError::TokenStore(err.to_string()))?;
        Ok(Self {
            conn,
            consecutive_failures: 0,
            blocked: Vec::new(),
        })
    }

    /// Chemin par défaut — `$XDG_DATA_HOME/wakfu-companion-overlay/sync-queue.sqlite3` (repli
    /// classique `directories` sous Windows/Linux, même motif que `icon_cache.rs`/
    /// `reference_data_cache.rs`).
    pub fn default_store_path() -> Option<std::path::PathBuf> {
        overlay_engine::app_dirs::project_dirs(APP_NAME)
            .map(|dirs| dirs.data_dir().join("sync-queue.sqlite3"))
    }

    /// **Rattache la file au compte `uid`** et rend le nombre d'entrées jetées (2026-09-23).
    ///
    /// La file ne mémorise pas de quel compte viennent ses entrées ; `clientKey` est calculé à
    /// l'envoi avec l'`uid` du compte connecté À CE MOMENT-LÀ. Depuis qu'une session expirée (401)
    /// laisse la file intacte pour la reconnexion, un AUTRE compte appairé ensuite aurait reçu
    /// l'historique du premier. La file retient donc l'empreinte (SHA-256, jamais l'identifiant en
    /// clair) du compte qu'elle sert : un compte différent la trouve vidée. Une file sans empreinte
    /// (écrite avant ce correctif) est adoptée telle quelle.
    pub fn claim_owner(&self, uid: &str) -> Result<usize, SyncError> {
        let fingerprint = hex_encode(&Sha256::digest(uid.as_bytes()));
        let current: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM sync_meta WHERE key = 'owner'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|err| SyncError::TokenStore(err.to_string()))?;
        let dropped = match current {
            Some(owner) if owner == fingerprint => return Ok(0),
            Some(_) => self.clear()?,
            None => 0,
        };
        self.conn
            .execute(
                "INSERT INTO sync_meta (key, value) VALUES ('owner', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [&fingerprint],
            )
            .map_err(|err| SyncError::TokenStore(err.to_string()))?;
        Ok(dropped)
    }

    /// Met un événement en file — **synchrone et rapide** (une écriture SQLite), comme
    /// `enqueue()` côté web est "synchrone et sans effet de bord coûteux" : appelable depuis le
    /// chemin d'ingestion (voir `Engine::drain_sync_events`), potentiellement des centaines de
    /// fois d'affilée lors d'un rattrapage.
    ///
    /// Idempotent par construction (`id = "{kind}:{signature}"`, clé primaire) : un rejeu du même
    /// événement avec un payload identique est un no-op ; avec un payload DIFFÉRENT (ex. un combat
    /// dont le détail est corrigé avant d'avoir pu partir), la version la plus récente remplace
    /// l'ancienne sans toucher `attempts` — miroir exact d'`enqueue()` côté web.
    pub fn enqueue(&self, event: &SyncEvent) -> Result<(), SyncError> {
        let id = event.id();
        let payload_json = serialize_payload(&event.payload);

        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT payload_json FROM sync_queue WHERE id = ?1",
                [&id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|err| SyncError::TokenStore(err.to_string()))?;

        match existing {
            Some(current) if current == payload_json => Ok(()),
            Some(_) => self
                .conn
                .execute(
                    "UPDATE sync_queue SET payload_json = ?1 WHERE id = ?2",
                    params![payload_json, id],
                )
                .map(|_| ())
                .map_err(|err| SyncError::TokenStore(err.to_string())),
            None => self
                .conn
                .execute(
                    "INSERT INTO sync_queue (id, kind, payload_json, signature, queued_at, attempts)
                     VALUES (?1, ?2, ?3, ?4, ?5, 0)",
                    params![id, event.kind.as_str(), payload_json, event.signature, now_ms()],
                )
                .map(|_| ())
                .map_err(|err| SyncError::TokenStore(err.to_string())),
        }
    }

    /// Vide la file et rend le nombre d'entrées jetées — à la **déconnexion du compte**
    /// (`SyncCommand::Deactivate`, 2026-09-18) : ce qui attendait n'a plus de destinataire, et
    /// le garder sur disque pour une reconnexion hypothétique conserverait sans fin les noms
    /// des tiers qu'il porte (`docs/analyse-rgpd.md` §3.3). Un compte reconnecté plus tard
    /// retrouvera de toute façon les combats encore dans `wakfu.log` au rattrapage suivant.
    pub fn clear(&self) -> Result<usize, SyncError> {
        self.conn
            .execute("DELETE FROM sync_queue", [])
            .map_err(|err| SyncError::TokenStore(err.to_string()))
    }

    /// Jette les entrées en file depuis plus de `max_age` (voir `MAX_PENDING_AGE`) et rend leur
    /// nombre. `queued_at` est la date de PREMIÈRE mise en file : un payload corrigé entre-temps
    /// (voir `enqueue`) ne rajeunit pas l'entrée.
    pub fn prune_older_than(&self, max_age: std::time::Duration) -> Result<usize, SyncError> {
        let cutoff = now_ms() - max_age.as_millis() as i64;
        self.conn
            .execute("DELETE FROM sync_queue WHERE queued_at < ?1", [cutoff])
            .map_err(|err| SyncError::TokenStore(err.to_string()))
    }

    /// Nombre d'entrées actuellement en file, tous types confondus — diagnostic/indicateur UI.
    pub fn pending_count(&self) -> Result<usize, SyncError> {
        self.conn
            .query_row("SELECT COUNT(*) FROM sync_queue", [], |row| {
                row.get::<_, i64>(0)
            })
            .map(|n| n as usize)
            .map_err(|err| SyncError::TokenStore(err.to_string()))
    }

    /// Échecs consécutifs depuis le dernier `Synced`/`Idle` — sert à l'appelant à calculer le
    /// backoff (`15s * 2^(n-1)`, plafonné à 5 min — voir §7.3 du plan), comme `consecutiveFailures`
    /// côté web.
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// Types d'événement suspendus pour la session (403, voir [`Rejection::KindBlocked`]) et la
    /// raison du refus. Leur file est conservée.
    pub fn blocked_kinds(&self) -> &[(HistoryEventKind, String)] {
        &self.blocked
    }

    /// Lève toutes les suspensions — à chaque (re)connexion d'un compte : un quota libéré depuis
    /// le site, ou un autre compte, a droit à un nouvel essai.
    pub fn unblock_all(&mut self) {
        self.blocked.clear();
    }

    /// Envoie tout ce qui est en file, par lots de `SYNC_BATCH_SIZE`, un type d'événement après
    /// l'autre (fight, purchase, trade, pact — comme `HISTORY_ENDPOINTS`).
    ///
    /// **Ce que devient un lot refusé** — voir [`Rejection`] (2026-09-23) : 401 arrête tout
    /// ([`FlushOutcome::Unauthorized`]) ; 403 suspend ce type et passe au suivant ; 400/413
    /// redivisent le lot jusqu'à isoler l'entrée fautive, retirée seule après contrôle (voir
    /// plus bas) ; 429, 5xx, réseau et réponse 2xx illisible
    /// arrêtent le passage (réessayable, miroir de `drain()` côté web qui `break` sur le premier
    /// échec) ; les autres 4xx consomment une tentative.
    ///
    /// Avant l'envoi, une entrée dont la date sort des bornes du serveur est retirée de la file
    /// avec un avertissement (voir [`date_in_server_range`]) — sans quoi le lot entier repartirait
    /// en 400. En 200, les entrées que le serveur a ignorées une à une (`rejected: [{ index,
    /// clientKey?, error }]`, champ facultatif) sont journalisées puis retirées comme les autres :
    /// les renvoyer n'y changerait rien.
    ///
    /// **Contrôle avant de retirer une entrée isolée** (audit du 2026-09-23, O2) : une entrée seule
    /// refusée en 400/413 n'est retirée que si le serveur a accepté, dans ce même passage, un autre
    /// lot du même type, ou accepte l'entrée SUIVANTE envoyée seule juste après (la sonde). Si la
    /// sonde est refusée à son tour — deux entrées isolées refusées d'affilée — le refus est
    /// **général** : passage arrêté en réessayable, rien retiré. Une entrée seule en file, sans
    /// contrôle possible, consomme une tentative (comme [`Rejection::Counted`]). Et jamais plus de
    /// [`MAX_SPLIT_DELETIONS_PER_PASS`] retraits par passage.
    ///
    /// Une réponse 2xx doit être un objet JSON : tout autre corps (page HTML servie en 200 par un
    /// hébergeur en « fail open », constat S6 — `client::parse_json_body` refuse déjà ce qui n'est
    /// pas `application/json`) est un échec réessayable, jamais un succès qui viderait le lot.
    ///
    /// `send` reçoit le chemin d'API relatif (`"/api/v1/history/fights"`, etc.) et le corps
    /// `{ "entries": [...] }` déjà construit (`clientKey` inclus) — production :
    /// `crate::client::post_json_authenticated` ; tests : un transport en mémoire (voir `tests`).
    pub fn flush_once(
        &mut self,
        uid: &str,
        mut send: impl FnMut(&str, &Value) -> Result<Value, SyncError>,
    ) -> Result<FlushOutcome, SyncError> {
        let mut any_sent = false;
        let now = now_ms();
        // Entrées retirées pour refus 400/413 dans ce passage, tous types confondus.
        let mut split_deletions = 0usize;

        // Même ordre que le web (`drain`, parcours des clés d'`HISTORY_ENDPOINTS`) : un type
        // d'événement par requête, les endpoints étant distincts.
        for kind in [
            HistoryEventKind::Fight,
            HistoryEventKind::Purchase,
            HistoryEventKind::Trade,
            HistoryEventKind::Pact,
        ] {
            if self.blocked.iter().any(|(blocked, _)| *blocked == kind) {
                continue;
            }
            // Taille du lot pour CE type : réduite de moitié à chaque 400/413 (voir
            // `Rejection::Split`), rétablie une fois l'entrée fautive écartée.
            let mut limit = SYNC_BATCH_SIZE;
            // Le serveur a-t-il accepté un lot de ce type dans ce passage ? Si oui, un refus
            // d'entrée isolée lui est propre (le serveur n'est pas en refus général).
            let mut kind_accepted = false;
            // Entrée isolée refusée, en attente de la sonde (l'entrée suivante envoyée seule).
            let mut suspect: Option<QueuedEvent> = None;
            // L'envoi précédent (de ce type, dans ce passage) était-il une entrée isolée refusée ?
            let mut previous_isolated_refused = false;
            loop {
                let rows = match &suspect {
                    // Sonde : l'entrée qui SUIT le suspect, seule.
                    Some(held) => {
                        let rows: Vec<QueuedEvent> = self
                            .select_batch(kind, 2)?
                            .into_iter()
                            .filter(|row| row.id != held.id)
                            .take(1)
                            .collect();
                        if rows.is_empty() {
                            // Plus rien pour contrôler : le suspect consomme une tentative.
                            let held = suspect.take().expect("suspect présent");
                            return self.retry_counted(kind, &held);
                        }
                        rows
                    }
                    None => self.select_batch(kind, limit)?,
                };
                if rows.is_empty() {
                    break;
                }
                let (batch, entries) = self.prepare_batch(uid, kind, rows, now)?;
                if batch.is_empty() {
                    continue; // tout le lot était hors bornes, et vient d'être retiré
                }
                let body = serde_json::json!({ "entries": entries });

                let result = send(kind.endpoint_path(), &body).and_then(|response| {
                    if response.is_object() {
                        Ok(response)
                    } else {
                        Err(SyncError::Json(
                            "réponse 2xx inattendue : un objet JSON était attendu".to_string(),
                        ))
                    }
                });
                match result {
                    Ok(response) => {
                        let rejected = log_rejected_entries(kind, &response);
                        self.delete_batch(&batch)?;
                        any_sent = true;
                        kind_accepted = true;
                        previous_isolated_refused = false;
                        // Seule trace d'un envoi RÉUSSI (2026-09-21) : jusqu'ici, seul l'échec
                        // était journalisé, et rien ne permettait de vérifier après coup qu'un
                        // achat ou un échange était bien parti — la file SQLite est vidée dès la
                        // réponse. `inserted` compte, côté serveur, les lignes insérées OU mises
                        // à jour (voir `server/history/ingest.ts`), donc pas les vrais doublons.
                        tracing::info!(
                            kind = kind.as_str(),
                            sent = batch.len(),
                            rejected,
                            inserted = response.get("inserted").and_then(serde_json::Value::as_i64),
                            "lot d'historique envoyé au compte"
                        );
                        // La sonde est passée : le refus du suspect lui était propre.
                        if let Some(held) = suspect.take() {
                            if let Some(outcome) =
                                self.drop_refused(kind, &held, &mut split_deletions)?
                            {
                                return Ok(outcome);
                            }
                            limit = SYNC_BATCH_SIZE;
                        }
                        // Ce lot est parti : il peut y en avoir d'autres du même type derrière
                        // (`select_batch` re-sélectionne toujours les plus anciens en premier).
                    }
                    Err(err) => match classify(&err) {
                        Rejection::SessionInvalid => {
                            return Ok(FlushOutcome::Unauthorized(err.to_string()));
                        }
                        Rejection::KindBlocked => {
                            tracing::error!(
                                kind = kind.as_str(),
                                pending = batch.len(),
                                %err,
                                "envoi de ce type d'historique refusé par le serveur — suspendu jusqu'à la prochaine connexion, file conservée"
                            );
                            self.blocked.push((kind, err.to_string()));
                            break;
                        }
                        Rejection::Split
                            if batch.len() == 1
                                && (suspect.is_some() || previous_isolated_refused) =>
                        {
                            // Deux entrées isolées refusées d'affilée : le serveur refuse tout,
                            // pas une entrée. Rien n'est retiré, réessai après backoff.
                            tracing::warn!(
                                kind = kind.as_str(),
                                %err,
                                "refus généralisé du serveur (deux entrées isolées refusées d'affilée) — rien retiré, réessai plus tard"
                            );
                            self.consecutive_failures += 1;
                            return Ok(FlushOutcome::Retry {
                                reason: err.to_string(),
                                after: None,
                            });
                        }
                        Rejection::Split if batch.len() > 1 => {
                            previous_isolated_refused = false;
                            limit = batch.len() / 2;
                            tracing::info!(
                                kind = kind.as_str(),
                                %err,
                                next_batch = limit,
                                "lot d'historique refusé — redivisé pour isoler l'entrée en cause"
                            );
                        }
                        Rejection::Split => {
                            let held = batch.into_iter().next().expect("lot d'une entrée");
                            previous_isolated_refused = true;
                            if kind_accepted {
                                // Contrôle déjà fait : ce passage a vu le serveur accepter ce type.
                                if let Some(outcome) =
                                    self.drop_refused(kind, &held, &mut split_deletions)?
                                {
                                    return Ok(outcome);
                                }
                                limit = SYNC_BATCH_SIZE;
                            } else {
                                tracing::info!(
                                    kind = kind.as_str(),
                                    %err,
                                    "entrée d'historique isolée refusée — sonde avec l'entrée suivante avant de la retirer"
                                );
                                suspect = Some(held);
                            }
                        }
                        Rejection::Throttled(after) => {
                            self.consecutive_failures += 1;
                            return Ok(FlushOutcome::Retry {
                                reason: err.to_string(),
                                after,
                            });
                        }
                        Rejection::Counted => {
                            self.bump_attempts_and_prune(&batch)?;
                            self.consecutive_failures += 1;
                            return Ok(FlushOutcome::Retry {
                                reason: err.to_string(),
                                after: None,
                            });
                        }
                        Rejection::Transient => {
                            self.consecutive_failures += 1;
                            return Ok(FlushOutcome::Retry {
                                reason: err.to_string(),
                                after: None,
                            });
                        }
                    },
                }
            }
        }

        self.consecutive_failures = 0;
        Ok(if any_sent {
            FlushOutcome::Synced
        } else {
            FlushOutcome::Idle
        })
    }

    /// Retire une entrée isolée dont le refus (400/413) a été contrôlé — sauf si le plafond
    /// [`MAX_SPLIT_DELETIONS_PER_PASS`] est atteint : `Some(Retry)` alors, et l'entrée reste.
    fn drop_refused(
        &mut self,
        kind: HistoryEventKind,
        held: &QueuedEvent,
        split_deletions: &mut usize,
    ) -> Result<Option<FlushOutcome>, SyncError> {
        if *split_deletions >= MAX_SPLIT_DELETIONS_PER_PASS {
            tracing::warn!(
                kind = kind.as_str(),
                max = MAX_SPLIT_DELETIONS_PER_PASS,
                "plafond de retraits par passage atteint — entrée refusée conservée, réessai plus tard"
            );
            self.consecutive_failures += 1;
            return Ok(Some(FlushOutcome::Retry {
                reason: format!(
                    "plus de {MAX_SPLIT_DELETIONS_PER_PASS} entrées refusées en un passage"
                ),
                after: None,
            }));
        }
        self.delete_batch(std::slice::from_ref(held))?;
        *split_deletions += 1;
        tracing::warn!(
            kind = kind.as_str(),
            "entrée d'historique refusée définitivement par le serveur — retirée de la file"
        );
        Ok(None)
    }

    /// Entrée isolée refusée sans contrôle possible (seule de son type en file) : une tentative de
    /// plus, abandon au bout de `MAX_ATTEMPTS` — comme un 4xx non classé.
    fn retry_counted(
        &mut self,
        kind: HistoryEventKind,
        held: &QueuedEvent,
    ) -> Result<FlushOutcome, SyncError> {
        self.bump_attempts_and_prune(std::slice::from_ref(held))?;
        self.consecutive_failures += 1;
        tracing::info!(
            kind = kind.as_str(),
            "entrée d'historique isolée refusée, sans autre entrée pour contrôler — une tentative consommée"
        );
        Ok(FlushOutcome::Retry {
            reason: "entrée refusée par le serveur (400/413), aucun contrôle possible".to_string(),
            after: None,
        })
    }

    /// Construit les entrées à envoyer (`clientKey`, `dungeonRunKey`) et **retire de la file** celles
    /// dont la date sort des bornes du serveur — rend les lignes gardées et leurs entrées, dans le
    /// même ordre.
    fn prepare_batch(
        &self,
        uid: &str,
        kind: HistoryEventKind,
        rows: Vec<QueuedEvent>,
        now_ms: i64,
    ) -> Result<(Vec<QueuedEvent>, Vec<Value>), SyncError> {
        let mut kept = Vec::with_capacity(rows.len());
        let mut entries = Vec::with_capacity(rows.len());
        let mut out_of_range = Vec::new();
        for row in rows {
            let mut value: Value = serde_json::from_str(&row.payload_json)
                .expect("payload persisté par enqueue() est toujours un JSON valide");
            if !date_in_server_range(kind, &value, now_ms) {
                // Jamais la signature au journal : elle porte les noms des combattants.
                tracing::warn!(
                    kind = kind.as_str(),
                    date = value.get(date_field(kind)).and_then(serde_json::Value::as_str).unwrap_or("absente"),
                    "entrée d'historique à la date hors bornes (avant 2012 ou dans le futur) — jamais envoyée, retirée de la file"
                );
                out_of_range.push(row);
                continue;
            }
            if let Value::Object(map) = &mut value {
                map.insert(
                    "clientKey".to_string(),
                    Value::String(client_key(uid, kind, &row.signature)),
                );
                // `dungeonRunSignature` (kind `Fight` uniquement, voir `FightPayload`) n'est
                // qu'une graine de contenu : jamais envoyée telle quelle, hachée ICI en
                // `dungeonRunKey` EXACTEMENT comme `clientKey` ci-dessus — miroir vérifié dans
                // `sync-queue.service.ts` (dépôt web) : `payload['dungeonRunKey'] = await
                // computeClientKey(uid, entry.kind, dungeonRunSignature)`. C'est ce qui fait que
                // tous les combats d'un même run finissent par partager le `clientKey` de leur boss
                // comme `dungeonRunKey`, sans aller-retour serveur pour l'obtenir.
                if kind == HistoryEventKind::Fight {
                    let run_key = match map.remove("dungeonRunSignature") {
                        Some(Value::String(signature)) => {
                            Value::String(client_key(uid, kind, &signature))
                        }
                        _ => Value::Null,
                    };
                    map.insert("dungeonRunKey".to_string(), run_key);
                }
            }
            kept.push(row);
            entries.push(value);
        }
        self.delete_batch(&out_of_range)?;
        Ok((kept, entries))
    }

    fn select_batch(
        &self,
        kind: HistoryEventKind,
        limit: usize,
    ) -> Result<Vec<QueuedEvent>, SyncError> {
        // `id` départage deux entrées mises en file à la même milliseconde : l'ordre doit être
        // stable d'un appel à l'autre pour que la redivision d'un lot (`Rejection::Split`)
        // reprenne bien la première moitié du lot précédent.
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, signature, payload_json, attempts FROM sync_queue
                 WHERE kind = ?1 ORDER BY queued_at ASC, id ASC LIMIT ?2",
            )
            .map_err(|err| SyncError::TokenStore(err.to_string()))?;
        let rows = stmt
            .query_map(params![kind.as_str(), limit as i64], |row| {
                Ok(QueuedEvent {
                    id: row.get(0)?,
                    signature: row.get(1)?,
                    payload_json: row.get(2)?,
                    attempts: row.get(3)?,
                })
            })
            .map_err(|err| SyncError::TokenStore(err.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| SyncError::TokenStore(err.to_string()))
    }

    fn delete_batch(&self, batch: &[QueuedEvent]) -> Result<(), SyncError> {
        for event in batch {
            self.conn
                .execute("DELETE FROM sync_queue WHERE id = ?1", [&event.id])
                .map_err(|err| SyncError::TokenStore(err.to_string()))?;
        }
        Ok(())
    }

    /// Miroir de la branche `permanent` de `send()` côté web : chaque entrée du lot voit son
    /// compteur de tentatives incrémenté, celles qui atteignent `MAX_ATTEMPTS` sont abandonnées
    /// (retirées de la file) — les autres restent, avec leur `payload_json` intact, prêtes à être
    /// réessayées plus tard. Ne sert plus qu'aux 4xx non classés (voir [`Rejection::Counted`]).
    fn bump_attempts_and_prune(&self, batch: &[QueuedEvent]) -> Result<(), SyncError> {
        for event in batch {
            let attempts = event.attempts + 1;
            if attempts >= MAX_ATTEMPTS {
                self.conn
                    .execute("DELETE FROM sync_queue WHERE id = ?1", [&event.id])
                    .map_err(|err| SyncError::TokenStore(err.to_string()))?;
            } else {
                self.conn
                    .execute(
                        "UPDATE sync_queue SET attempts = ?1 WHERE id = ?2",
                        params![attempts, event.id],
                    )
                    .map_err(|err| SyncError::TokenStore(err.to_string()))?;
            }
        }
        Ok(())
    }
}

/// Journalise les entrées qu'une réponse 200 déclare ignorées (`rejected: [{ index, clientKey?,
/// error }]`) et rend leur nombre — 0 sans le champ (serveur antérieur à son ajout). Elles
/// quittent la file avec le reste du lot : invalides aux yeux du serveur, les renvoyer n'y
/// changerait rien.
fn log_rejected_entries(kind: HistoryEventKind, response: &Value) -> usize {
    let Some(rejected) = response.get("rejected").and_then(Value::as_array) else {
        return 0;
    };
    for entry in rejected {
        tracing::warn!(
            kind = kind.as_str(),
            index = entry.get("index").and_then(serde_json::Value::as_i64),
            client_key = entry.get("clientKey").and_then(serde_json::Value::as_str),
            error = entry
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("?"),
            "entrée d'historique ignorée par le serveur — retirée de la file"
        );
    }
    rejected.len()
}

fn serialize_payload(payload: &HistoryPayload) -> String {
    serde_json::to_string(payload).expect("HistoryPayload est toujours sérialisable (pas de map à clé non-String, aucun NaN/Infinity)")
}

#[cfg(test)]
mod tests {
    use super::*;
    use overlay_engine::{FightPayload, PurchasePayload};
    use std::cell::RefCell;

    fn purchase_event(signature: &str, item: &str) -> SyncEvent {
        SyncEvent {
            kind: HistoryEventKind::Purchase,
            signature: signature.to_string(),
            payload: HistoryPayload::Purchase(PurchasePayload {
                item_id: None,
                item_name: Some(item.to_string()),
                quantity: 1,
                total_cost: 100,
                occurred_at: "2026-01-01T00:00:00.000Z".to_string(),
                game_server: None,
            }),
        }
    }

    fn fight_event(signature: &str, fight_id: i64) -> SyncEvent {
        SyncEvent {
            kind: HistoryEventKind::Fight,
            signature: signature.to_string(),
            payload: HistoryPayload::Fight(FightPayload {
                fight_id: Some(fight_id),
                started_at: "2026-01-01T00:00:00.000Z".to_string(),
                duration_ms: Some(1000),
                won: true,
                turns: 0,
                total_damage: 42,
                xp_gained: 0,
                kamas_gained: None,
                game_server: None,
                dungeon_id: None,
                dungeon_run_signature: None,
                challenges_passed: 0,
                challenges_failed: 0,
                participants: Vec::new(),
                loot: Vec::new(),
            }),
        }
    }

    /// Transport en mémoire : enregistre chaque appel, répond selon un script fourni par le test.
    struct FakeTransport {
        calls: RefCell<Vec<(String, Value)>>,
        responses: RefCell<Vec<Result<Value, SyncError>>>,
    }

    impl FakeTransport {
        fn new(responses: Vec<Result<Value, SyncError>>) -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                responses: RefCell::new(responses),
            }
        }

        fn send(&self, path: &str, body: &Value) -> Result<Value, SyncError> {
            self.calls
                .borrow_mut()
                .push((path.to_string(), body.clone()));
            if self.responses.borrow().is_empty() {
                return Ok(serde_json::json!({ "accepted": [], "inserted": 0 }));
            }
            self.responses.borrow_mut().remove(0)
        }
    }

    #[test]
    fn enqueue_puis_flush_envoie_avec_le_bon_client_key() {
        let queue = SyncQueue::open_in_memory().unwrap();
        queue
            .enqueue(&purchase_event("10:00:00,000|x|1|100", "Eclat"))
            .unwrap();

        let transport = FakeTransport::new(vec![Ok(serde_json::json!({}))]);
        let mut queue = queue;
        let outcome = queue
            .flush_once("uid-1", |path, body| transport.send(path, body))
            .unwrap();

        assert_eq!(outcome, FlushOutcome::Synced);
        assert_eq!(queue.pending_count().unwrap(), 0);
        let calls = transport.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "/api/v1/history/purchases");
        let sent_key = calls[0].1["entries"][0]["clientKey"].as_str().unwrap();
        assert_eq!(
            sent_key,
            client_key("uid-1", HistoryEventKind::Purchase, "10:00:00,000|x|1|100")
        );
    }

    /// `dungeonRunSignature` (kind `Fight` uniquement) est haché en `dungeonRunKey` à l'envoi,
    /// EXACTEMENT comme `clientKey` (voir la doc de `flush_once`) — jamais transmise telle quelle.
    #[test]
    fn dungeon_run_signature_est_hachee_en_dungeon_run_key_a_lenvoi() {
        let mut event = fight_event("10:00:00,000|42|won|", 42);
        if let HistoryPayload::Fight(fight) = &mut event.payload {
            fight.dungeon_id = Some(65);
            fight.dungeon_run_signature = Some("10:00:00,000|42|won|".to_string());
        }
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&event).unwrap();

        let transport = FakeTransport::new(vec![Ok(serde_json::json!({}))]);
        queue
            .flush_once("uid-1", |path, body| transport.send(path, body))
            .unwrap();

        let calls = transport.calls.borrow();
        let sent = &calls[0].1["entries"][0];
        assert!(
            sent.get("dungeonRunSignature").is_none(),
            "la graine brute ne doit jamais être transmise au serveur"
        );
        assert_eq!(
            sent["dungeonRunKey"].as_str().unwrap(),
            client_key("uid-1", HistoryEventKind::Fight, "10:00:00,000|42|won|"),
            "même fonction que clientKey, appliquée à la signature de run"
        );
    }

    /// Un combat sans rattachement de donjon connu envoie `dungeonRunKey: null` — jamais absent
    /// (miroir de `dungeonId: null`) et jamais une chaîne inventée.
    #[test]
    fn absence_de_rattachement_de_donjon_envoie_un_dungeon_run_key_nul() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue
            .enqueue(&fight_event("10:00:00,000|42|won|", 42))
            .unwrap();

        let transport = FakeTransport::new(vec![Ok(serde_json::json!({}))]);
        queue
            .flush_once("uid-1", |path, body| transport.send(path, body))
            .unwrap();

        let calls = transport.calls.borrow();
        assert!(calls[0].1["entries"][0]["dungeonRunKey"].is_null());
    }

    #[test]
    fn rejeu_dix_fois_du_meme_evenement_ne_produit_quune_seule_entree() {
        let queue = SyncQueue::open_in_memory().unwrap();
        for _ in 0..10 {
            queue
                .enqueue(&fight_event("10:00:00,000|42|won|", 42))
                .unwrap();
        }
        assert_eq!(queue.pending_count().unwrap(), 1);
    }

    #[test]
    fn rejeu_apres_envoi_reussi_ne_renvoie_rien_de_plus() {
        // Critère de sortie L5 (§12 du plan) : « rejeu 10x du même log => aucun doublon en base ».
        // Ici : un événement déjà synchronisé, remis en file 10x (comme le ferait une reconnexion
        // qui relit tout le fichier), ne repart jamais une 2e fois — la file elle-même le
        // redéduplique à l'écriture (id stable), et rien ne reste après le premier envoi réussi.
        let mut queue = SyncQueue::open_in_memory().unwrap();
        let transport = FakeTransport::new(vec![]);
        queue
            .enqueue(&fight_event("10:00:00,000|42|won|", 42))
            .unwrap();
        assert_eq!(
            queue
                .flush_once("uid-1", |p, b| transport.send(p, b))
                .unwrap(),
            FlushOutcome::Synced
        );
        assert_eq!(queue.pending_count().unwrap(), 0);

        for _ in 0..10 {
            queue
                .enqueue(&fight_event("10:00:00,000|42|won|", 42))
                .unwrap();
            // Le même événement revient : id identique, INSERT ré-crée la ligne (la file locale
            // ne sait pas qu'il a déjà été synchronisé — c'est `clientKey`/`ON CONFLICT DO
            // NOTHING` côté SERVEUR qui absorbe le doublon, voir la doc de module). On vérifie ici
            // la moitié locale de la garantie : jamais plus d'UNE ligne en file pour cet id, quel
            // que soit le nombre de rejeux.
            assert_eq!(queue.pending_count().unwrap(), 1);
        }
    }

    #[test]
    fn un_payload_different_pour_le_meme_id_remplace_lancien_sans_dupliquer() {
        let queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("sig", "Eclat")).unwrap();
        let mut corrected = purchase_event("sig", "Eclat");
        corrected.payload = HistoryPayload::Purchase(PurchasePayload {
            item_id: Some(999),
            item_name: None,
            quantity: 1,
            total_cost: 100,
            occurred_at: "2026-01-01T00:00:00.000Z".to_string(),
            game_server: None,
        });
        queue.enqueue(&corrected).unwrap();

        assert_eq!(queue.pending_count().unwrap(), 1);
        let transport = FakeTransport::new(vec![Ok(serde_json::json!({}))]);
        let mut queue = queue;
        queue
            .flush_once("uid-1", |p, b| transport.send(p, b))
            .unwrap();
        let calls = transport.calls.borrow();
        assert_eq!(calls[0].1["entries"][0]["itemId"], serde_json::json!(999));
    }

    #[test]
    fn echec_reseau_arrete_le_lot_et_programme_un_reessai() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("sig", "Eclat")).unwrap();
        let transport = FakeTransport::new(vec![Err(SyncError::Network("coupé".into()))]);

        let outcome = queue
            .flush_once("uid-1", |p, b| transport.send(p, b))
            .unwrap();

        assert!(matches!(outcome, FlushOutcome::Retry { .. }));
        assert_eq!(queue.consecutive_failures(), 1);
        // Toujours en file : un échec réseau ne consomme jamais `attempts`.
        assert_eq!(queue.pending_count().unwrap(), 1);
    }

    /// 4xx non classé (ici 404, route absente d'un déploiement en retard) : comportement
    /// historique, une tentative consommée par refus, abandon au bout de `MAX_ATTEMPTS`.
    #[test]
    fn rejet_non_classe_abandonne_lentree_apres_max_attempts() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("sig", "Eclat")).unwrap();

        for attempt in 1..=MAX_ATTEMPTS {
            let transport = FakeTransport::new(vec![Err(http_err(404, None, None))]);
            let outcome = queue
                .flush_once("uid-1", |p, b| transport.send(p, b))
                .unwrap();
            assert!(matches!(outcome, FlushOutcome::Retry { .. }));
            if attempt < MAX_ATTEMPTS {
                assert_eq!(queue.pending_count().unwrap(), 1, "tentative {attempt}");
            }
        }
        assert_eq!(queue.pending_count().unwrap(), 0);
    }

    #[test]
    fn clear_vide_la_file_et_compte_ce_quelle_jette() {
        let queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("a", "Eclat")).unwrap();
        queue.enqueue(&fight_event("b", 1)).unwrap();
        assert_eq!(queue.clear().unwrap(), 2);
        assert_eq!(queue.pending_count().unwrap(), 0);
        assert_eq!(queue.clear().unwrap(), 0, "rien de plus au second passage");
    }

    #[test]
    fn prune_older_than_ne_jette_que_les_entrees_perimees() {
        let queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("vieille", "Eclat")).unwrap();
        queue.enqueue(&purchase_event("recente", "Eclat")).unwrap();
        // Vieillit la première entrée d'un mois et un jour, directement en base : `queued_at` est
        // la seule chose que la purge regarde.
        let cutoff = now_ms() - (MAX_PENDING_AGE.as_millis() as i64) - 24 * 3600 * 1000;
        queue
            .conn
            .execute(
                "UPDATE sync_queue SET queued_at = ?1 WHERE signature = 'vieille'",
                [cutoff],
            )
            .unwrap();
        assert_eq!(queue.prune_older_than(MAX_PENDING_AGE).unwrap(), 1);
        assert_eq!(queue.pending_count().unwrap(), 1);
        let restante: String = queue
            .conn
            .query_row("SELECT signature FROM sync_queue", [], |row| row.get(0))
            .unwrap();
        assert_eq!(restante, "recente");
    }

    #[test]
    fn idle_quand_la_file_est_vide() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        let transport = FakeTransport::new(vec![]);
        let outcome = queue
            .flush_once("uid-1", |p, b| transport.send(p, b))
            .unwrap();
        assert_eq!(outcome, FlushOutcome::Idle);
    }

    #[test]
    fn les_trois_types_sont_envoyes_a_des_endpoints_distincts() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("p", "Eclat")).unwrap();
        queue.enqueue(&fight_event("f", 1)).unwrap();
        let transport =
            FakeTransport::new(vec![Ok(serde_json::json!({})), Ok(serde_json::json!({}))]);
        queue
            .flush_once("uid-1", |p, b| transport.send(p, b))
            .unwrap();
        let calls = transport.calls.borrow();
        let paths: Vec<&str> = calls.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"/api/v1/history/fights"));
        assert!(paths.contains(&"/api/v1/history/purchases"));
    }

    fn http_err(
        status: u16,
        code: Option<&str>,
        retry_after: Option<std::time::Duration>,
    ) -> SyncError {
        SyncError::Http {
            status,
            path: "/api/v1/history/x".to_string(),
            code: code.map(str::to_string),
            retry_after,
        }
    }

    fn fight_started_at(signature: &str, started_at: &str) -> SyncEvent {
        let mut event = fight_event(signature, 1);
        if let HistoryPayload::Fight(fight) = &mut event.payload {
            fight.started_at = started_at.to_string();
        }
        event
    }

    fn sent_item_names(body: &Value) -> Vec<String> {
        body["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["itemName"].as_str().unwrap().to_string())
            .collect()
    }

    /// 401 : tout s'arrête, rien n'est consommé — la file attend la reconnexion.
    #[test]
    fn un_401_arrete_tout_sans_toucher_a_la_file() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&fight_event("f", 1)).unwrap();
        queue.enqueue(&purchase_event("p", "Eclat")).unwrap();
        let transport = FakeTransport::new(vec![Err(http_err(401, None, None))]);

        let outcome = queue
            .flush_once("uid-1", |p, b| transport.send(p, b))
            .unwrap();

        assert!(matches!(outcome, FlushOutcome::Unauthorized(_)));
        assert_eq!(transport.calls.borrow().len(), 1, "aucun autre type tenté");
        assert_eq!(queue.pending_count().unwrap(), 2);
        let attempts: i64 = queue
            .conn
            .query_row("SELECT MAX(attempts) FROM sync_queue", [], |row| row.get(0))
            .unwrap();
        assert_eq!(attempts, 0);
    }

    /// 403 quota : le type est suspendu, sa file conservée ; les autres types partent. Au passage
    /// suivant, le type suspendu n'est même plus tenté — jusqu'à `unblock_all`.
    #[test]
    fn un_403_quota_suspend_le_type_sans_vider_sa_file() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&fight_event("f", 1)).unwrap();
        queue.enqueue(&purchase_event("p", "Eclat")).unwrap();
        let transport = FakeTransport::new(vec![
            Err(http_err(403, Some("history_quota_exceeded"), None)),
            Ok(serde_json::json!({ "inserted": 1 })),
        ]);

        let outcome = queue
            .flush_once("uid-1", |p, b| transport.send(p, b))
            .unwrap();

        assert_eq!(outcome, FlushOutcome::Synced);
        assert_eq!(queue.pending_count().unwrap(), 1, "le combat reste en file");
        assert_eq!(queue.blocked_kinds().len(), 1);
        assert_eq!(queue.blocked_kinds()[0].0, HistoryEventKind::Fight);
        assert!(queue.blocked_kinds()[0]
            .1
            .contains("history_quota_exceeded"));

        let second = FakeTransport::new(vec![]);
        assert_eq!(
            queue.flush_once("uid-1", |p, b| second.send(p, b)).unwrap(),
            FlushOutcome::Idle
        );
        assert!(
            second.calls.borrow().is_empty(),
            "type suspendu : aucun envoi"
        );

        queue.unblock_all();
        let third = FakeTransport::new(vec![]);
        assert_eq!(
            queue.flush_once("uid-1", |p, b| third.send(p, b)).unwrap(),
            FlushOutcome::Synced
        );
        assert_eq!(queue.pending_count().unwrap(), 0);
    }

    /// 403 `browser_session_required` : refus définitif de la requête, mais jamais une purge.
    #[test]
    fn un_403_session_navigateur_ne_purge_rien() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("p", "Eclat")).unwrap();
        let transport = FakeTransport::new(vec![Err(http_err(
            403,
            Some("browser_session_required"),
            None,
        ))]);
        queue
            .flush_once("uid-1", |p, b| transport.send(p, b))
            .unwrap();
        assert_eq!(queue.pending_count().unwrap(), 1);
        assert_eq!(queue.blocked_kinds()[0].0, HistoryEventKind::Purchase);
        assert_eq!(queue.consecutive_failures(), 0, "pas un échec réessayable");
    }

    /// 413 : le lot est redivisé par moitié jusqu'à passer, rien n'est perdu.
    #[test]
    fn un_413_redivise_le_lot_jusqu_a_ce_qu_il_passe() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        for i in 0..5 {
            queue
                .enqueue(&purchase_event(&format!("p{i}"), &format!("Objet {i}")))
                .unwrap();
        }
        let mut sizes = Vec::new();
        let mut sent = Vec::new();
        let outcome = queue
            .flush_once("uid-1", |_, body| {
                let names = sent_item_names(body);
                sizes.push(names.len());
                if names.len() > 2 {
                    return Err(http_err(413, None, None));
                }
                sent.extend(names);
                Ok(serde_json::json!({}))
            })
            .unwrap();

        assert_eq!(outcome, FlushOutcome::Synced);
        assert_eq!(queue.pending_count().unwrap(), 0);
        assert_eq!(sizes, vec![5, 2, 2, 1]);
        sent.sort();
        assert_eq!(sent.len(), 5, "chaque entrée envoyée une et une seule fois");
        sent.dedup();
        assert_eq!(sent.len(), 5);
    }

    /// 413 sur une entrée seule, sans aucune autre entrée pour contrôler que le refus lui est
    /// propre : une tentative consommée (O2) — abandon au bout de `MAX_ATTEMPTS`, jamais d'emblée.
    #[test]
    fn un_413_sur_une_entree_seule_consomme_une_tentative() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("p", "Eclat")).unwrap();
        for pass in 1..=MAX_ATTEMPTS {
            let transport = FakeTransport::new(vec![Err(http_err(413, None, None))]);
            let outcome = queue
                .flush_once("uid-1", |p, b| transport.send(p, b))
                .unwrap();
            assert!(matches!(outcome, FlushOutcome::Retry { .. }));
            let expected = if pass < MAX_ATTEMPTS { 1 } else { 0 };
            assert_eq!(queue.pending_count().unwrap(), expected, "passage {pass}");
        }
    }

    fn enqueue_purchases(queue: &SyncQueue, count: usize) {
        for i in 0..count {
            queue
                .enqueue(&purchase_event(&format!("p{i:03}"), &format!("Objet {i}")))
                .unwrap();
        }
    }

    /// O2 : un 400 GÉNÉRALISÉ (tout est refusé) ne retire RIEN — la sonde (entrée suivante,
    /// seule) est refusée elle aussi, le passage s'arrête en réessayable. Plusieurs passages
    /// d'affilée ne vident pas davantage la file.
    #[test]
    fn un_400_generalise_ne_supprime_rien() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        enqueue_purchases(&queue, 120);
        for _ in 0..5 {
            let outcome = queue
                .flush_once("uid-1", |_, _| Err(http_err(400, None, None)))
                .unwrap();
            assert!(matches!(outcome, FlushOutcome::Retry { after: None, .. }));
        }
        assert_eq!(queue.pending_count().unwrap(), 120);
        let attempts: i64 = queue
            .conn
            .query_row("SELECT MAX(attempts) FROM sync_queue", [], |row| row.get(0))
            .unwrap();
        assert_eq!(attempts, 0, "un refus général ne consomme aucune tentative");
        assert_eq!(queue.consecutive_failures(), 5);
    }

    /// O2 : même un contrôle trompeur ne retire jamais plus de `MAX_SPLIT_DELETIONS_PER_PASS`
    /// entrées en un passage (ici cinq entrées invalides, une sur deux en tête de file : chacune
    /// est bien isolée et contrôlée, mais seules les trois premières partent ce passage).
    #[test]
    fn les_retraits_sont_plafonnes_par_passage() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        enqueue_purchases(&queue, 20);
        let bad: Vec<String> = (0..10).step_by(2).map(|i| format!("Objet {i}")).collect();
        let outcome = queue
            .flush_once("uid-1", |_, body| {
                let names = sent_item_names(body);
                if names.iter().any(|name| bad.contains(name)) {
                    return Err(http_err(400, None, None));
                }
                Ok(serde_json::json!({}))
            })
            .unwrap();
        assert!(matches!(outcome, FlushOutcome::Retry { .. }));
        let removed = 20 - queue.pending_count().unwrap();
        // Les entrées valides parties ne comptent pas ; seules les invalides retirées sont
        // plafonnées. Restent les 5 - 3 invalides non retirées.
        let remaining: Vec<String> = {
            let mut stmt = queue
                .conn
                .prepare("SELECT payload_json FROM sync_queue")
                .unwrap();
            stmt.query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .map(|p| {
                    serde_json::from_str::<Value>(&p.unwrap()).unwrap()["itemName"]
                        .as_str()
                        .unwrap()
                        .to_string()
                })
                .collect()
        };
        let bad_left = remaining.iter().filter(|n| bad.contains(n)).count();
        assert_eq!(bad_left, 5 - MAX_SPLIT_DELETIONS_PER_PASS);
        assert!(removed >= MAX_SPLIT_DELETIONS_PER_PASS);
    }

    /// O2 : l'entrée invalide en TÊTE de file (aucun lot accepté avant elle) est retirée après
    /// que la sonde — l'entrée suivante, seule — a été acceptée.
    #[test]
    fn un_400_en_tete_de_file_est_retire_apres_sonde() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        enqueue_purchases(&queue, 4);
        let mut sent = Vec::new();
        let outcome = queue
            .flush_once("uid-1", |_, body| {
                let names = sent_item_names(body);
                if names.iter().any(|name| name == "Objet 0") {
                    return Err(http_err(400, None, None));
                }
                sent.extend(names);
                Ok(serde_json::json!({}))
            })
            .unwrap();
        assert_eq!(outcome, FlushOutcome::Synced);
        assert_eq!(queue.pending_count().unwrap(), 0);
        sent.sort();
        assert_eq!(sent, vec!["Objet 1", "Objet 2", "Objet 3"]);
    }

    /// O2 : deux entrées isolées refusées d'affilée dans un passage — même après qu'un lot a été
    /// accepté — font conclure à un refus général : la première est retirée, la seconde gardée.
    #[test]
    fn deux_entrees_isolees_refusees_d_affilee_arretent_le_passage() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        enqueue_purchases(&queue, 5);
        let outcome = queue
            .flush_once("uid-1", |_, body| {
                let names = sent_item_names(body);
                if names
                    .iter()
                    .any(|name| name == "Objet 3" || name == "Objet 4")
                {
                    return Err(http_err(400, None, None));
                }
                Ok(serde_json::json!({}))
            })
            .unwrap();
        assert!(matches!(outcome, FlushOutcome::Retry { .. }));
        assert_eq!(
            queue.pending_count().unwrap(),
            1,
            "« Objet 4 » reste en file"
        );
    }

    /// S6 : un 2xx dont le corps n'est pas l'objet JSON attendu (page HTML d'un hébergeur en
    /// « fail open », déjà convertie en `SyncError::Json` par le client, ou JSON d'une autre
    /// forme) n'est JAMAIS un succès qui viderait la file, ni un refus définitif.
    #[test]
    fn un_2xx_non_json_est_reessayable_sans_rien_retirer() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        enqueue_purchases(&queue, 3);
        for response in [
            Err(SyncError::Json("content-type text/html".to_string())),
            Ok(Value::String("<!doctype html>".to_string())),
            Ok(serde_json::json!([])),
        ] {
            let transport = FakeTransport::new(vec![response]);
            let outcome = queue
                .flush_once("uid-1", |p, b| transport.send(p, b))
                .unwrap();
            assert!(matches!(outcome, FlushOutcome::Retry { after: None, .. }));
            assert_eq!(transport.calls.borrow().len(), 1);
        }
        assert_eq!(queue.pending_count().unwrap(), 3);
        let attempts: i64 = queue
            .conn
            .query_row("SELECT MAX(attempts) FROM sync_queue", [], |row| row.get(0))
            .unwrap();
        assert_eq!(attempts, 0);
    }

    /// 400 : l'entrée invalide est isolée par redivision et retirée seule — les quatre autres
    /// partent, au lieu d'être jetées avec elle au bout de dix tentatives.
    #[test]
    fn un_400_isole_l_entree_invalide_sans_perdre_les_autres() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        for i in 0..5 {
            queue
                .enqueue(&purchase_event(&format!("p{i}"), &format!("Objet {i}")))
                .unwrap();
        }
        let mut sent = Vec::new();
        let outcome = queue
            .flush_once("uid-1", |_, body| {
                let names = sent_item_names(body);
                if names.iter().any(|name| name == "Objet 3") {
                    return Err(http_err(400, None, None));
                }
                sent.extend(names);
                Ok(serde_json::json!({}))
            })
            .unwrap();

        assert_eq!(outcome, FlushOutcome::Synced);
        assert_eq!(queue.pending_count().unwrap(), 0);
        sent.sort();
        assert_eq!(sent, vec!["Objet 0", "Objet 1", "Objet 2", "Objet 4"]);
    }

    /// 429 : réessayable, avec le délai imposé par le serveur ; la file est intacte.
    #[test]
    fn un_429_rend_le_delai_retry_after() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("p", "Eclat")).unwrap();
        let delay = std::time::Duration::from_secs(30);
        let transport = FakeTransport::new(vec![Err(http_err(429, None, Some(delay)))]);
        let outcome = queue
            .flush_once("uid-1", |p, b| transport.send(p, b))
            .unwrap();
        assert!(matches!(
            outcome,
            FlushOutcome::Retry { after: Some(after), .. } if after == delay
        ));
        assert_eq!(queue.pending_count().unwrap(), 1);
        assert_eq!(queue.consecutive_failures(), 1);
    }

    /// 5xx : backoff ordinaire (aucun délai imposé), aucune tentative consommée.
    #[test]
    fn un_5xx_est_reessayable_sans_consommer_de_tentative() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("p", "Eclat")).unwrap();
        let transport = FakeTransport::new(vec![Err(http_err(503, None, None))]);
        let outcome = queue
            .flush_once("uid-1", |p, b| transport.send(p, b))
            .unwrap();
        assert!(matches!(outcome, FlushOutcome::Retry { after: None, .. }));
        assert_eq!(queue.pending_count().unwrap(), 1);
        let attempts: i64 = queue
            .conn
            .query_row("SELECT attempts FROM sync_queue", [], |row| row.get(0))
            .unwrap();
        assert_eq!(attempts, 0);
    }

    /// 200 avec `rejected` : les entrées ignorées quittent la file comme les acceptées.
    #[test]
    fn les_entrees_rejetees_en_200_quittent_la_file() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("a", "Eclat")).unwrap();
        queue.enqueue(&purchase_event("b", "Plume")).unwrap();
        let transport = FakeTransport::new(vec![Ok(serde_json::json!({
            "accepted": ["x"],
            "inserted": 1,
            "rejected": [{ "index": 1, "clientKey": "y", "error": "itemName trop long" }]
        }))]);
        let outcome = queue
            .flush_once("uid-1", |p, b| transport.send(p, b))
            .unwrap();
        assert_eq!(outcome, FlushOutcome::Synced);
        assert_eq!(queue.pending_count().unwrap(), 0);
        assert_eq!(
            log_rejected_entries(HistoryEventKind::Purchase, &serde_json::json!({})),
            0,
            "compatible sans le champ"
        );
    }

    /// Un combat daté de l'époque Unix (restauré d'un vieux `fight-*.json`) ou du futur n'est
    /// jamais envoyé — retiré de la file — et n'empêche pas les autres de partir.
    #[test]
    fn les_dates_hors_bornes_ne_partent_jamais() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue
            .enqueue(&fight_started_at("epoch", "1970-01-01T00:00:00.000Z"))
            .unwrap();
        queue
            .enqueue(&fight_started_at("futur", "2999-01-01T00:00:00.000Z"))
            .unwrap();
        queue
            .enqueue(&fight_started_at("illisible", "hier"))
            .unwrap();
        queue
            .enqueue(&fight_started_at("ok", "2026-09-01T12:00:00.000Z"))
            .unwrap();
        let transport = FakeTransport::new(vec![]);
        let outcome = queue
            .flush_once("uid-1", |p, b| transport.send(p, b))
            .unwrap();

        assert_eq!(outcome, FlushOutcome::Synced);
        assert_eq!(queue.pending_count().unwrap(), 0);
        let calls = transport.calls.borrow();
        assert_eq!(calls.len(), 1);
        let entries = calls[0].1["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["startedAt"], "2026-09-01T12:00:00.000Z");
    }

    #[test]
    fn bornes_de_date_du_serveur() {
        let now = 1_790_000_000_000; // 2026-09
        let kind = HistoryEventKind::Purchase;
        let at = |date: &str| serde_json::json!({ "occurredAt": date });
        assert!(date_in_server_range(
            kind,
            &at("2012-01-01T00:00:00.000Z"),
            now
        ));
        assert!(!date_in_server_range(
            kind,
            &at("2011-12-31T23:59:59.999Z"),
            now
        ));
        let edge = overlay_engine::log_time::format_iso_utc(now + HISTORY_MAX_FUTURE_SKEW_MS);
        assert!(date_in_server_range(kind, &at(&edge), now));
        let beyond = overlay_engine::log_time::format_iso_utc(now + HISTORY_MAX_FUTURE_SKEW_MS + 1);
        assert!(!date_in_server_range(kind, &at(&beyond), now));
        assert!(!date_in_server_range(kind, &serde_json::json!({}), now));
        assert_eq!(
            HISTORY_MIN_DATE_MS,
            overlay_engine::log_time::utc_ms_from_civil(2012, 1, 1, 0)
        );
    }

    /// Les extractions de pacte partent sur `/history/pacts`, au format que lit
    /// `parsePactExtractionsBody` (`server/history/parse.ts`) : `clientKey`, `occurredAt`,
    /// `gameServer`, `items[{ itemId, itemName, quantity }]`.
    #[test]
    fn une_extraction_de_pacte_part_au_format_du_serveur() {
        use overlay_engine::{PactExtractionItemPayload, PactExtractionPayload};
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue
            .enqueue(&SyncEvent {
                kind: HistoryEventKind::Pact,
                signature: "12:00:00,000|eclatx2".to_string(),
                payload: HistoryPayload::PactExtraction(PactExtractionPayload {
                    occurred_at: "2026-09-01T12:00:00.000Z".to_string(),
                    game_server: None,
                    items: vec![PactExtractionItemPayload {
                        item_id: None,
                        item_name: Some("Eclat".to_string()),
                        quantity: 2,
                    }],
                }),
            })
            .unwrap();
        let transport = FakeTransport::new(vec![]);
        queue
            .flush_once("uid-1", |p, b| transport.send(p, b))
            .unwrap();
        let calls = transport.calls.borrow();
        assert_eq!(calls[0].0, "/api/v1/history/pacts");
        let entry = &calls[0].1["entries"][0];
        assert_eq!(
            entry["clientKey"],
            client_key("uid-1", HistoryEventKind::Pact, "12:00:00,000|eclatx2")
        );
        assert_eq!(entry["occurredAt"], "2026-09-01T12:00:00.000Z");
        assert!(entry["gameServer"].is_null());
        assert_eq!(
            entry["items"],
            serde_json::json!([{ "itemId": null, "itemName": "Eclat", "quantity": 2 }])
        );
    }

    /// La file suit son compte : le même la retrouve intacte, un autre la trouve vide.
    #[test]
    fn la_file_est_videe_quand_un_autre_compte_la_reprend() {
        let queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("a", "Eclat")).unwrap();
        assert_eq!(queue.claim_owner("uid-1").unwrap(), 0, "file adoptée");
        assert_eq!(queue.claim_owner("uid-1").unwrap(), 0, "même compte");
        assert_eq!(queue.pending_count().unwrap(), 1);
        assert_eq!(queue.claim_owner("uid-2").unwrap(), 1, "autre compte");
        assert_eq!(queue.pending_count().unwrap(), 0);
        let stored: String = queue
            .conn
            .query_row("SELECT value FROM sync_meta WHERE key = 'owner'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(!stored.contains("uid-2"), "jamais l'identifiant en clair");
    }
}
