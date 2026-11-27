//! Le compte du site auquel l'historique local se raccroche, le temps de l'import (plan §8).
//!
//! Le site identifiait chaque événement par `client_key = sha256(uid|type|signature)` : la même
//! signature de contenu que la base locale, salée par l'identifiant du compte. Connaître l'`uid`
//! suffit donc à reconnaître, dans les deux sens, un événement déjà présent :
//!
//! - une ligne **écrite depuis le log** reçoit sa `client_key` (à l'écriture si le compte est déjà
//!   connu, sinon par [`set_account_uid`]) : l'import la trouve et n'ajoute pas de doublon ;
//! - une ligne **importée** (signature provisoire `import:<client_key>`) que le log produit plus
//!   tard est **adoptée** par l'écriture locale : elle prend sa vraie signature au lieu d'être
//!   recopiée.

use rusqlite::{params, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};

/// Préfixe de la signature provisoire d'une ligne importée (la signature d'origine ne quitte pas le
/// serveur : seul son hachage y est stocké).
pub const IMPORTED_SIGNATURE_PREFIX: &str = "import:";

const ACCOUNT_UID: &str = "account_uid";

/// Les tables qui portent une `client_key`, avec le type d'événement qui sale leur hachage —
/// `HistoryEventKind::as_str` côté moteur.
pub(crate) const KEYED_TABLES: [(&str, &str); 4] = [
    ("fights", "fight"),
    ("purchases", "purchase"),
    ("trades", "trade"),
    ("pact_extractions", "pact"),
];

/// Même calcul que `overlay_sync::client_key` et que le site (`HistoryEventService`) — vérifié
/// par un test d'égalité.
pub fn client_key(uid: &str, kind: &str, signature: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(uid.as_bytes());
    hasher.update(b"|");
    hasher.update(kind.as_bytes());
    hasher.update(b"|");
    hasher.update(signature.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn meta_get(tx: &rusqlite::Connection, key: &str) -> rusqlite::Result<Option<String>> {
    tx.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .optional()
}

pub(crate) fn meta_set(tx: &rusqlite::Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub(crate) fn account_uid(tx: &rusqlite::Connection) -> rusqlite::Result<Option<String>> {
    meta_get(tx, ACCOUNT_UID)
}

/// Ce que [`set_account_uid`] a fait — pour le journal et les tests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AccountReport {
    /// Lignes locales qui ont reçu leur `client_key`.
    pub keyed: usize,
    /// Lignes importées fondues dans la ligne locale équivalente (écrite avant que le compte soit
    /// connu, puis importée).
    pub merged: usize,
}

/// Enregistre le compte et pose la `client_key` des lignes locales qui n'en ont pas — toutes, si le
/// compte a changé. Une ligne importée qui se révèle être une ligne locale y est fondue : la ligne
/// locale reste (elle porte la vraie signature), complétée du rattachement de donjon de l'import.
pub(crate) fn set_account_uid(tx: &Transaction<'_>, uid: &str) -> rusqlite::Result<AccountReport> {
    let previous = account_uid(tx)?;
    let same_account = previous.as_deref() == Some(uid);
    let mut report = AccountReport::default();
    for (table, kind) in KEYED_TABLES {
        let filter = if same_account {
            "client_key IS NULL AND "
        } else {
            ""
        };
        let rows: Vec<(i64, String)> = tx
            .prepare(&format!(
                "SELECT id, signature FROM {table}
                 WHERE {filter}signature NOT LIKE '{IMPORTED_SIGNATURE_PREFIX}%'"
            ))?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        for (id, signature) in rows {
            let key = client_key(uid, kind, &signature);
            let twin: Option<(i64, String)> = tx
                .query_row(
                    &format!(
                        "SELECT id, signature FROM {table} WHERE client_key = ?1 AND id <> ?2"
                    ),
                    params![key, id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            if let Some((twin_id, twin_signature)) = twin {
                if !twin_signature.starts_with(IMPORTED_SIGNATURE_PREFIX) {
                    // Deux lignes locales de même clé : impossible (signatures uniques), sauf
                    // collision de hachage. On laisse la ligne sans clé.
                    continue;
                }
                if table == "fights" {
                    adopt_dungeon(tx, id, twin_id)?;
                }
                tx.execute(&format!("DELETE FROM {table} WHERE id = ?1"), [twin_id])?;
                report.merged += 1;
            }
            tx.execute(
                &format!("UPDATE {table} SET client_key = ?2 WHERE id = ?1"),
                params![id, key],
            )?;
            report.keyed += 1;
        }
    }
    meta_set(tx, ACCOUNT_UID, uid)?;
    Ok(report)
}

/// Complète le rattachement de donjon du combat `local` par celui du combat `imported`, sans jamais
/// écraser une valeur déjà présente — même règle `COALESCE` que l'écriture.
pub(crate) fn adopt_dungeon(
    tx: &Transaction<'_>,
    local: i64,
    imported: i64,
) -> rusqlite::Result<()> {
    tx.execute(
        "UPDATE fights SET
            fight_type = CASE WHEN fights.dungeon_id IS NULL AND src.dungeon_id IS NOT NULL
                              THEN src.fight_type ELSE fights.fight_type END,
            dungeon_id = COALESCE(fights.dungeon_id, src.dungeon_id),
            dungeon_run_key = COALESCE(fights.dungeon_run_key, src.dungeon_run_key)
         FROM (SELECT dungeon_id, dungeon_run_key, fight_type FROM fights WHERE id = ?2) AS src
         WHERE fights.id = ?1",
        params![local, imported],
    )?;
    Ok(())
}

/// À l'écriture d'un événement local : si une ligne importée porte sa `client_key`, elle prend la
/// vraie signature (et sera mise à jour comme une ligne locale). Rend la clé à poser sur une
/// insertion, ou `None` si elle est déjà prise par une autre ligne (ne devrait pas arriver).
pub(crate) fn adopt_imported(
    tx: &Transaction<'_>,
    table: &str,
    key: &str,
    signature: &str,
) -> rusqlite::Result<Option<String>> {
    tx.execute(
        &format!(
            "UPDATE OR IGNORE {table} SET signature = ?1
             WHERE client_key = ?2 AND signature LIKE '{IMPORTED_SIGNATURE_PREFIX}%'"
        ),
        params![signature, key],
    )?;
    let holder: Option<String> = tx
        .query_row(
            &format!("SELECT signature FROM {table} WHERE client_key = ?1"),
            [key],
            |row| row.get(0),
        )
        .optional()?;
    Ok(match holder {
        Some(holder) if holder != signature => {
            tracing::warn!(
                table,
                "historique local : clé de compte déjà prise, ligne gardée sans clé"
            );
            None
        }
        _ => Some(key.to_string()),
    })
}
