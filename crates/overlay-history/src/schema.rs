//! Schéma de `history.sqlite3` et ses migrations, versionnées par `PRAGMA user_version`.
//!
//! Calqué sur les tables d'historique du site (`wakfu-companion/server/db/schema.ts`), sans
//! `user_id` (une base par poste) et avec la **signature de contenu** du moteur
//! (`overlay_engine::history`) comme clé d'unicité : c'est elle qui rend l'écriture
//! idempotente — relire dix fois le même `wakfu.log` n'écrit chaque événement qu'une fois.
//!
//! Les horodatages sont des **millisecondes UTC** (`INTEGER`), pas du texte : tri, bornes de période
//! et agrégats se font sans conversion.
//!
//! Ajouter une migration : une nouvelle entrée en fin de [`MIGRATIONS`], jamais la modification
//! d'une entrée déjà livrée (une base existante ne la rejouerait pas).

use rusqlite::Connection;

use crate::HistoryError;

/// Une migration par version : l'entrée `i` fait passer la base de la version `i` à `i + 1`.
pub(crate) const MIGRATIONS: &[&str] = &[
    // Version 1 (2026-11-27) — les quatre historiques du site.
    "
    CREATE TABLE fights (
        id INTEGER PRIMARY KEY,
        signature TEXT NOT NULL UNIQUE,
        -- `fightId` du log : diagnostic seulement, jamais unique au-delà d'une session de jeu.
        fight_log_id INTEGER,
        started_at INTEGER NOT NULL,
        duration_ms INTEGER,
        won INTEGER NOT NULL,
        turns INTEGER NOT NULL DEFAULT 0,
        total_damage INTEGER NOT NULL DEFAULT 0,
        xp_gained INTEGER NOT NULL DEFAULT 0,
        kamas_gained INTEGER,
        game_server TEXT,
        -- Rattachement de donjon : posé par le moteur ou recalculé ici (`derive`), jamais effacé.
        dungeon_id INTEGER,
        -- Clé partagée par les combats d'un même run : la signature de son combat représentatif.
        dungeon_run_key TEXT,
        challenges_passed INTEGER NOT NULL DEFAULT 0,
        challenges_failed INTEGER NOT NULL DEFAULT 0,
        -- `DUNGEON_*`, `BREACH`, `ULTIMATE_BREACH`, `DUNGEON_ROOM`, `FAMILY_<id>`, `FAMILY_NONE`,
        -- `EVENT` — voir `derive::fight_type`.
        fight_type TEXT
    );
    CREATE INDEX fights_started_at_idx ON fights (started_at, id);
    CREATE INDEX fights_dungeon_run_key_idx ON fights (dungeon_run_key);
    CREATE INDEX fights_fight_type_idx ON fights (fight_type);

    -- Une ligne par INSTANCE de combattant (trois Bouftous = trois lignes).
    CREATE TABLE fight_participants (
        fight_id INTEGER NOT NULL REFERENCES fights (id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        instance_index INTEGER NOT NULL,
        side TEXT NOT NULL CHECK (side IN ('ally', 'enemy')),
        monster_id INTEGER,
        class_name TEXT,
        damage INTEGER NOT NULL DEFAULT 0,
        defeated INTEGER NOT NULL DEFAULT 0,
        fled INTEGER NOT NULL DEFAULT 0,
        -- Ventilations par sort et par élément, en JSON comme sur le site.
        spells TEXT NOT NULL DEFAULT '[]',
        heal INTEGER NOT NULL DEFAULT 0,
        armor INTEGER NOT NULL DEFAULT 0,
        heal_spells TEXT NOT NULL DEFAULT '[]',
        armor_spells TEXT NOT NULL DEFAULT '[]',
        xp_gained INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (fight_id, name, instance_index)
    );

    CREATE TABLE fight_loot (
        fight_id INTEGER NOT NULL REFERENCES fights (id) ON DELETE CASCADE,
        line_index INTEGER NOT NULL,
        item_id INTEGER,
        item_name TEXT,
        quantity INTEGER NOT NULL,
        PRIMARY KEY (fight_id, line_index)
    );

    -- Achats à l'Hôtel de Vente ; une récupération de kamas porte le nom `__hdv_kamas_sale__`.
    CREATE TABLE purchases (
        id INTEGER PRIMARY KEY,
        signature TEXT NOT NULL UNIQUE,
        item_id INTEGER,
        item_name TEXT,
        quantity INTEGER NOT NULL,
        total_cost INTEGER NOT NULL,
        occurred_at INTEGER NOT NULL,
        game_server TEXT
    );
    CREATE INDEX purchases_occurred_at_idx ON purchases (occurred_at, id);

    CREATE TABLE trades (
        id INTEGER PRIMARY KEY,
        signature TEXT NOT NULL UNIQUE,
        peer_name TEXT NOT NULL,
        self_name TEXT NOT NULL,
        occurred_at INTEGER NOT NULL,
        kamas_acquired INTEGER NOT NULL DEFAULT 0,
        kamas_given INTEGER NOT NULL DEFAULT 0,
        game_server TEXT
    );
    CREATE INDEX trades_occurred_at_idx ON trades (occurred_at, id);

    CREATE TABLE trade_items (
        trade_id INTEGER NOT NULL REFERENCES trades (id) ON DELETE CASCADE,
        direction TEXT NOT NULL CHECK (direction IN ('acquired', 'given')),
        -- Position dans sa direction, comme `parse.ts` côté site.
        line_index INTEGER NOT NULL,
        item_id INTEGER,
        item_name TEXT,
        quantity INTEGER NOT NULL,
        PRIMARY KEY (trade_id, direction, line_index)
    );

    CREATE TABLE pact_extractions (
        id INTEGER PRIMARY KEY,
        signature TEXT NOT NULL UNIQUE,
        occurred_at INTEGER NOT NULL,
        game_server TEXT
    );
    CREATE INDEX pact_extractions_occurred_at_idx ON pact_extractions (occurred_at, id);

    CREATE TABLE pact_extraction_items (
        extraction_id INTEGER NOT NULL REFERENCES pact_extractions (id) ON DELETE CASCADE,
        line_index INTEGER NOT NULL,
        item_id INTEGER,
        item_name TEXT,
        quantity INTEGER NOT NULL,
        PRIMARY KEY (extraction_id, line_index)
    );
    ",
    // Version 2 (2026-11-27, plan §8) — import de l'historique en ligne. `client_key` est la clé
    // du site (`sha256(uid|type|signature)`, `overlay_sync::client_key`) : une ligne importée la
    // porte, une ligne écrite depuis le log la reçoit dès que le compte est connu (`account`).
    // C'est elle qui fait se reconnaître un combat importé et le même combat relu dans le log.
    // Plusieurs `NULL` restent permis par l'index unique (SQLite).
    "
    ALTER TABLE fights ADD COLUMN client_key TEXT;
    CREATE UNIQUE INDEX fights_client_key_idx ON fights (client_key);
    ALTER TABLE purchases ADD COLUMN client_key TEXT;
    CREATE UNIQUE INDEX purchases_client_key_idx ON purchases (client_key);
    ALTER TABLE trades ADD COLUMN client_key TEXT;
    CREATE UNIQUE INDEX trades_client_key_idx ON trades (client_key);
    ALTER TABLE pact_extractions ADD COLUMN client_key TEXT;
    CREATE UNIQUE INDEX pact_extractions_client_key_idx ON pact_extractions (client_key);

    -- Petites valeurs de la base : identifiant du compte, imports terminés.
    CREATE TABLE meta (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    ",
    // Version 3 (2026-11-27) — corrections manuelles d'un objet (`correct`) : une ligne corrigée
    // n'est plus réécrite par la relecture du log.
    "
    ALTER TABLE fight_loot ADD COLUMN corrected INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE purchases ADD COLUMN corrected INTEGER NOT NULL DEFAULT 0;
    ",
];

/// La version de schéma que ce binaire sait lire et écrire.
pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// Amène la base à [`SCHEMA_VERSION`], dans une transaction par migration. Une base **plus
/// récente** que ce binaire (retour à une version antérieure de l'overlay) est refusée plutôt
/// qu'écrite avec un schéma qu'il ne connaît pas.
pub(crate) fn migrate(conn: &mut Connection) -> Result<(), HistoryError> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if current > SCHEMA_VERSION {
        return Err(HistoryError::NewerSchema {
            found: current,
            supported: SCHEMA_VERSION,
        });
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", index as i64 + 1)?;
        tx.commit()?;
        tracing::info!(version = index + 1, "historique local : schéma migré");
    }
    Ok(())
}
