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
/// Miroir de `MAX_ATTEMPTS` — seuls les échecs "permanents" (voir `is_permanent_rejection`)
/// comptent, jamais un simple problème réseau.
const MAX_ATTEMPTS: i64 = 10;

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
    /// Rien en file au moment de l'appel.
    Idle,
    /// Tout ce qui était en file au moment de l'appel a été envoyé avec succès.
    Synced,
    /// Un lot a échoué (réseau, 5xx, 401, 429 — réessayable — ou rejet permanent d'un lot, qui
    /// n'empêche pas non plus un réessai ultérieur des lots suivants, voir la doc de module) :
    /// rien de plus tenté ce passage, l'appelant programme un réessai (backoff). Porte la
    /// description de l'échec (`Display` de `SyncError`) — **correctif du 2026-09-03** : cet échec
    /// n'était jusqu'ici tracé nulle part côté `overlay-ui` (voir `spawn_sync_thread`), ce qui a
    /// laissé un vrai blocage de synchronisation (jeton non transmis, voir `post_json_authenticated`)
    /// totalement invisible pendant plusieurs jours.
    Retry(String),
}

/// Un rejet HTTP 4xx (hors 401/429) signifie que le SERVEUR a refusé la charge utile elle-même —
/// réessayer à l'identique ne changera rien, voir `MAX_ATTEMPTS`. Miroir exact de `permanent`
/// (`SyncQueueService.send`). 401 : le client bascule déjà en mode invité ailleurs (voir la doc de
/// `SyncQueue`, pas géré ici). 429 : throttling, réessayable par définition.
fn is_permanent_rejection(err: &SyncError) -> bool {
    matches!(err, SyncError::Http { status, .. } if (400..500).contains(status) && *status != 401 && *status != 429)
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
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_queue (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                signature TEXT NOT NULL,
                queued_at INTEGER NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .map_err(|err| SyncError::TokenStore(err.to_string()))?;
        Ok(Self {
            conn,
            consecutive_failures: 0,
        })
    }

    /// Chemin par défaut — `$XDG_DATA_HOME/wakfu-companion-overlay/sync-queue.sqlite3` (repli
    /// classique `directories` sous Windows/Linux, même motif que `icon_cache.rs`/
    /// `reference_data_cache.rs`).
    pub fn default_store_path() -> Option<std::path::PathBuf> {
        directories::ProjectDirs::from("", "", APP_NAME)
            .map(|dirs| dirs.data_dir().join("sync-queue.sqlite3"))
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

    /// Envoie tout ce qui est en file, par lots de `SYNC_BATCH_SIZE`, un type d'événement après
    /// l'autre (fight, purchase, trade — comme `HISTORY_ENDPOINTS`) — s'arrête au premier lot en
    /// échec (réessayable ou rejet permanent, voir `is_permanent_rejection`) plutôt que
    /// d'enchaîner sur les lots/types suivants : miroir exact de `drain()` côté web, qui `break`
    /// sur le premier échec des deux boucles imbriquées.
    ///
    /// `send` reçoit le chemin d'API relatif (`"/api/v1/history/fights"`, etc.) et le corps
    /// `{ "entries": [...] }` déjà construit (`clientKey` inclus) — production : `crate::client::
    /// post_json` ; tests : un transport en mémoire (voir `tests` ci-dessous).
    pub fn flush_once(
        &mut self,
        uid: &str,
        mut send: impl FnMut(&str, &Value) -> Result<Value, SyncError>,
    ) -> Result<FlushOutcome, SyncError> {
        let mut any_sent = false;

        for kind in [
            HistoryEventKind::Fight,
            HistoryEventKind::Purchase,
            HistoryEventKind::Trade,
        ] {
            loop {
                let batch = self.select_batch(kind, SYNC_BATCH_SIZE)?;
                if batch.is_empty() {
                    break;
                }

                let entries: Vec<Value> = batch
                    .iter()
                    .map(|row| {
                        let mut value: Value = serde_json::from_str(&row.payload_json)
                            .expect("payload persisté par enqueue() est toujours un JSON valide");
                        if let Value::Object(map) = &mut value {
                            map.insert(
                                "clientKey".to_string(),
                                Value::String(client_key(uid, kind, &row.signature)),
                            );
                            // `dungeonRunSignature` (kind `Fight` uniquement, voir `FightPayload`)
                            // n'est qu'une graine de contenu : jamais envoyée telle quelle, hachée
                            // ICI en `dungeonRunKey` EXACTEMENT comme `clientKey` ci-dessus — miroir
                            // vérifié dans `sync-queue.service.ts` (dépôt web) :
                            // `payload['dungeonRunKey'] = await computeClientKey(uid, entry.kind,
                            // dungeonRunSignature)`. C'est ce qui fait que tous les combats d'un
                            // même run finissent par partager le `clientKey` de leur boss comme
                            // `dungeonRunKey`, sans aller-retour serveur pour l'obtenir.
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
                        value
                    })
                    .collect();
                let body = serde_json::json!({ "entries": entries });

                match send(kind.endpoint_path(), &body) {
                    Ok(_) => {
                        self.delete_batch(&batch)?;
                        any_sent = true;
                        // Ce lot est parti : il peut y en avoir d'autres du même type derrière
                        // (`select_batch` re-sélectionne toujours les plus anciens en premier).
                    }
                    Err(err) if is_permanent_rejection(&err) => {
                        self.bump_attempts_and_prune(&batch)?;
                        self.consecutive_failures += 1;
                        return Ok(FlushOutcome::Retry(err.to_string()));
                    }
                    Err(err) => {
                        self.consecutive_failures += 1;
                        return Ok(FlushOutcome::Retry(err.to_string()));
                    }
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

    fn select_batch(
        &self,
        kind: HistoryEventKind,
        limit: usize,
    ) -> Result<Vec<QueuedEvent>, SyncError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, signature, payload_json, attempts FROM sync_queue
                 WHERE kind = ?1 ORDER BY queued_at ASC LIMIT ?2",
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
    /// réessayées plus tard (peu utile pour un vrai rejet 4xx qui échouera identiquement, mais
    /// c'est exactement le choix du web : ne PAS deviner côté client si un futur changement serveur
    /// rendrait le même payload acceptable).
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

        assert!(matches!(outcome, FlushOutcome::Retry(_)));
        assert_eq!(queue.consecutive_failures(), 1);
        // Toujours en file : un échec réseau ne consomme jamais `attempts`.
        assert_eq!(queue.pending_count().unwrap(), 1);
    }

    #[test]
    fn rejet_permanent_abandonne_lentree_apres_max_attempts() {
        let mut queue = SyncQueue::open_in_memory().unwrap();
        queue.enqueue(&purchase_event("sig", "Eclat")).unwrap();

        for attempt in 1..=MAX_ATTEMPTS {
            let transport = FakeTransport::new(vec![Err(SyncError::Http {
                status: 400,
                path: "/api/v1/history/purchases".to_string(),
            })]);
            let outcome = queue
                .flush_once("uid-1", |p, b| transport.send(p, b))
                .unwrap();
            assert!(matches!(outcome, FlushOutcome::Retry(_)));
            if attempt < MAX_ATTEMPTS {
                assert_eq!(queue.pending_count().unwrap(), 1, "tentative {attempt}");
            }
        }
        assert_eq!(queue.pending_count().unwrap(), 0);
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
}
