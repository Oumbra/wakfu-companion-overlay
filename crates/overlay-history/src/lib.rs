//! **Historique local** (2026-11-27, `docs/plan-historique-local.md` §6) — les combats, achats et
//! récupérations de kamas à l'Hôtel de Vente, échanges et extractions de pacte du joueur, stockés
//! sur son poste dans `data/history.sqlite3`. Remplace, à terme, l'envoi vers l'API du site : rien
//! ne quitte la machine.
//!
//! - **Écriture** ([`HistoryStore::record`]) : les événements que le moteur produit déjà
//!   (`Engine::drain_sync_events`), écrits par transaction, de façon idempotente (signature de
//!   contenu unique) — relire tout `wakfu.log` à chaque lancement n'ajoute rien.
//! - **Recalculs** (`derive`) : rattachement des salles à leur run de donjon sur l'historique
//!   local, et type de combat, comme le faisait le serveur.
//! - **Lecture** : listes et agrégats du récap viendront avec la fenêtre d'historique (phase 3).
//!
//! La base n'est **pas chiffrée** (décision du 2026-11-27) : elle vit dans le dossier de données de
//! l'utilisateur, comme le reste de l'overlay ; seule une future sauvegarde vers un tiers le sera.

mod derive;
mod schema;
mod write;

use std::path::{Path, PathBuf};

use overlay_engine::{CatalogIndex, DungeonIndex, SyncEvent};
use rusqlite::Connection;

pub use schema::SCHEMA_VERSION;
pub use write::RecordReport;

/// Nom du fichier, dans le dossier de données de l'overlay.
pub const FILE_NAME: &str = "history.sqlite3";

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("base d'historique : {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("base d'historique créée par une version plus récente de l'overlay (schéma {found}, ce binaire lit jusqu'à {supported})")]
    NewerSchema { found: i64, supported: i64 },
    #[error("dossier de données introuvable")]
    NoDataDir,
}

/// Les référentiels dont les recalculs ont besoin — ceux du référentiel embarqué.
#[derive(Clone, Copy)]
pub struct Referentiels<'a> {
    pub catalog: &'a CatalogIndex,
    pub dungeons: &'a DungeonIndex,
}

/// Les volumes de la base — pour le journal et les tests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub fights: i64,
    pub fight_participants: i64,
    pub fight_loot: i64,
    pub purchases: i64,
    pub trades: i64,
    pub trade_items: i64,
    pub pact_extractions: i64,
    pub pact_extraction_items: i64,
}

/// La base d'historique ouverte. Une seule connexion, tenue par un seul thread (celui d'écriture) ;
/// la future fenêtre d'historique ouvrira sa propre connexion en lecture (mode WAL : lectures et
/// écriture ne se bloquent pas).
pub struct HistoryStore {
    conn: Connection,
}

impl HistoryStore {
    /// `<dossier de données de l'overlay>/history.sqlite3`.
    pub fn default_path() -> Result<PathBuf, HistoryError> {
        overlay_engine::app_dirs::project_dirs("wakfu-companion-overlay")
            .map(|dirs| dirs.data_dir().join(FILE_NAME))
            .ok_or(HistoryError::NoDataDir)
    }

    /// Ouvre (et crée au besoin) la base `path`, puis la met au schéma courant.
    pub fn open(path: &Path) -> Result<Self, HistoryError> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        // WAL : la fenêtre d'historique lira pendant que le moteur écrit. `NORMAL` suffit en WAL
        // (une coupure de courant peut perdre la dernière transaction, jamais corrompre la base —
        // et le prochain lancement réécrit de toute façon ce que le log contient encore).
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Self::init(conn)
    }

    /// Une base en mémoire — tests et mesures.
    pub fn open_in_memory() -> Result<Self, HistoryError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self, HistoryError> {
        conn.pragma_update(None, "foreign_keys", true)?;
        schema::migrate(&mut conn)?;
        Ok(Self { conn })
    }

    /// Écrit un lot d'événements du moteur en **une** transaction, puis recalcule rattachements de
    /// donjon et types de combat pour les combats touchés. Idempotent : réécrire le même lot ne
    /// change rien.
    pub fn record(
        &mut self,
        events: &[SyncEvent],
        referentiels: Referentiels<'_>,
    ) -> Result<RecordReport, HistoryError> {
        let mut report = RecordReport::default();
        if events.is_empty() {
            return Ok(report);
        }
        let tx = self.conn.transaction()?;
        let mut touched = write::record_events(&tx, events, &mut report)?;
        let attached = derive::recompute_dungeon_runs(
            &tx,
            &touched,
            referentiels.catalog,
            referentiels.dungeons,
        )?;
        report.dungeon_attached = attached.len();
        touched.extend(attached);
        touched.sort_unstable();
        touched.dedup();
        derive::recompute_fight_types(&tx, &touched, referentiels.catalog, referentiels.dungeons)?;
        tx.commit()?;
        Ok(report)
    }

    pub fn counts(&self) -> Result<Counts, HistoryError> {
        let count = |table: &str| -> rusqlite::Result<i64> {
            self.conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
        };
        Ok(Counts {
            fights: count("fights")?,
            fight_participants: count("fight_participants")?,
            fight_loot: count("fight_loot")?,
            purchases: count("purchases")?,
            trades: count("trades")?,
            trade_items: count("trade_items")?,
            pact_extractions: count("pact_extractions")?,
            pact_extraction_items: count("pact_extraction_items")?,
        })
    }

    /// Accès SQL brut, en lecture — tests et diagnostic, en attendant les requêtes de la fenêtre
    /// d'historique.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests;
