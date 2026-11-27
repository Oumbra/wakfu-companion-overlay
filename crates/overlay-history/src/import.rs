//! **Import de l'historique en ligne** (2026-11-27, `docs/plan-historique-local.md` §8) : les pages
//! de `GET /api/v1/history/{fights,purchases,trades,pacts}` du site, écrites dans la base locale.
//!
//! Le réseau n'est pas ici : [`import_all`] reçoit une fonction qui rend une page (le JSON de la
//! réponse), ce qui la rend testable sans serveur. `overlay-ui` lui passe `overlay_sync::client::
//! fetch_history_page`.
//!
//! - Chaque entrée porte sa `client_key` : une entrée déjà présente (importée, ou écrite depuis le
//!   log avec la même clé — voir `account`) n'est pas recopiée. Réimporter ne change rien.
//! - Les combats arrivent avec leur rattachement de donjon et leur type, calculés par le serveur :
//!   rien à recalculer, sauf un type absent (anciennes lignes), complété par le prochain
//!   `HistoryStore::record`.
//! - Une page = une transaction : un import interrompu garde les pages faites, et le suivant
//!   repart du début sans rien doubler.

use rusqlite::{params, OptionalExtension, Transaction};
use serde::Deserialize;
use serde_json::Value;

use crate::account::IMPORTED_SIGNATURE_PREFIX;
use crate::{HistoryError, HistoryStore};
use overlay_engine::log_time::parse_iso_utc_ms;

/// Taille de page demandée — le maximum du site (`MAX_PAGE_SIZE`).
pub const PAGE_SIZE: u32 = 200;

/// Les quatre historiques du site, dans l'ordre de l'import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    Fights,
    Purchases,
    Trades,
    Pacts,
}

impl ImportKind {
    pub const ALL: [ImportKind; 4] = [
        ImportKind::Fights,
        ImportKind::Purchases,
        ImportKind::Trades,
        ImportKind::Pacts,
    ];

    /// Chemin de la route de lecture paginée.
    pub fn path(self) -> &'static str {
        match self {
            ImportKind::Fights => "/api/v1/history/fights",
            ImportKind::Purchases => "/api/v1/history/purchases",
            ImportKind::Trades => "/api/v1/history/trades",
            ImportKind::Pacts => "/api/v1/history/pacts",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ImportKind::Fights => "combats",
            ImportKind::Purchases => "achats",
            ImportKind::Trades => "échanges",
            ImportKind::Pacts => "pactes",
        }
    }
}

/// Ce qu'une page (ou tout l'import) a fait.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PageReport {
    pub inserted: usize,
    /// Entrées déjà dans la base (importées avant, ou relues dans le log).
    pub present: usize,
    /// Entrées illisibles (date, champ obligatoire) — écartées sans bloquer la page.
    pub skipped: usize,
}

impl PageReport {
    fn add(&mut self, other: PageReport) {
        self.inserted += other.inserted;
        self.present += other.present;
        self.skipped += other.skipped;
    }
}

/// Avancement, rendu après chaque page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportProgress {
    pub kind: ImportKind,
    /// Pages de ce type déjà écrites.
    pub pages: usize,
    /// Cumul de ce type.
    pub report: PageReport,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError<E: std::error::Error + 'static> {
    #[error("lecture de l'historique en ligne : {0}")]
    Fetch(#[source] E),
    #[error(transparent)]
    History(#[from] HistoryError),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    entries: Vec<Value>,
    next_before: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FightEntry {
    client_key: String,
    started_at: String,
    duration_ms: Option<i64>,
    won: bool,
    #[serde(default)]
    turns: i64,
    #[serde(default)]
    total_damage: i64,
    #[serde(default)]
    xp_gained: i64,
    kamas_gained: Option<i64>,
    game_server: Option<String>,
    dungeon_id: Option<i64>,
    dungeon_run_key: Option<String>,
    #[serde(default)]
    challenges_passed: i64,
    #[serde(default)]
    challenges_failed: i64,
    fight_type: Option<String>,
    #[serde(default)]
    participants: Vec<ParticipantEntry>,
    #[serde(default)]
    loot: Vec<LootEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParticipantEntry {
    side: String,
    name: String,
    monster_id: Option<i64>,
    #[serde(default)]
    instance_index: i64,
    class_name: Option<String>,
    #[serde(default)]
    damage: i64,
    #[serde(default)]
    defeated: bool,
    #[serde(default)]
    fled: bool,
    #[serde(default)]
    spells: Value,
    #[serde(default)]
    heal: i64,
    #[serde(default)]
    armor: i64,
    #[serde(default)]
    heal_spells: Value,
    #[serde(default)]
    armor_spells: Value,
    #[serde(default)]
    xp_gained: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LootEntry {
    line_index: Option<i64>,
    item_id: Option<i64>,
    item_name: Option<String>,
    quantity: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PurchaseEntry {
    client_key: String,
    item_id: Option<i64>,
    item_name: Option<String>,
    quantity: i64,
    total_cost: i64,
    occurred_at: String,
    game_server: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemEntry {
    item_id: Option<i64>,
    item_name: Option<String>,
    quantity: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TradeEntry {
    client_key: String,
    peer_name: String,
    self_name: String,
    occurred_at: String,
    #[serde(default)]
    kamas_acquired: i64,
    #[serde(default)]
    kamas_given: i64,
    game_server: Option<String>,
    #[serde(default)]
    acquired: Vec<ItemEntry>,
    #[serde(default)]
    given: Vec<ItemEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PactEntry {
    client_key: String,
    occurred_at: String,
    game_server: Option<String>,
    #[serde(default)]
    items: Vec<ItemEntry>,
}

/// Importe tout l'historique en ligne, type par type, page par page jusqu'à `nextBefore: null`.
///
/// - `open` ouvre la base pour **une** page puis la rend : la base n'est pas tenue ouverte pendant
///   les requêtes réseau (même règle que le thread Historique, voir `overlay-ui`).
/// - `fetch(kind, before)` rend la réponse JSON d'une page.
/// - `on_page` est appelé après chaque page écrite.
pub fn import_all<E: std::error::Error + 'static>(
    mut open: impl FnMut() -> Result<HistoryStore, HistoryError>,
    mut fetch: impl FnMut(ImportKind, Option<&str>) -> Result<Value, E>,
    mut on_page: impl FnMut(&ImportProgress),
) -> Result<PageReport, ImportError<E>> {
    let mut total = PageReport::default();
    for kind in ImportKind::ALL {
        let mut progress = ImportProgress {
            kind,
            pages: 0,
            report: PageReport::default(),
        };
        let mut before: Option<String> = None;
        loop {
            let body = fetch(kind, before.as_deref()).map_err(ImportError::Fetch)?;
            let mut store = open()?;
            let (report, next) = store.import_page(kind, &body)?;
            drop(store);
            progress.pages += 1;
            progress.report.add(report);
            on_page(&progress);
            match next {
                // Garde-fou : un curseur qui ne bouge plus bouclerait sans fin.
                Some(next) if before.as_deref() != Some(next.as_str()) => before = Some(next),
                _ => break,
            }
        }
        total.add(progress.report);
    }
    Ok(total)
}

impl HistoryStore {
    /// Écrit une page d'un GET d'historique du site, en une transaction. Rend le curseur de la page
    /// suivante (`nextBefore`), `None` à la dernière.
    pub fn import_page(
        &mut self,
        kind: ImportKind,
        body: &Value,
    ) -> Result<(PageReport, Option<String>), HistoryError> {
        let page = Page::deserialize(body).map_err(|source| HistoryError::ImportPage {
            kind: kind.label(),
            source,
        })?;
        let tx = self.conn.transaction()?;
        let mut report = PageReport::default();
        for entry in page.entries {
            let written = match kind {
                ImportKind::Fights => parse(entry).map(|e| import_fight(&tx, e)),
                ImportKind::Purchases => parse(entry).map(|e| import_purchase(&tx, e)),
                ImportKind::Trades => parse(entry).map(|e| import_trade(&tx, e)),
                ImportKind::Pacts => parse(entry).map(|e| import_pact(&tx, e)),
            };
            match written.transpose()?.flatten() {
                Some(true) => report.inserted += 1,
                Some(false) => report.present += 1,
                None => report.skipped += 1,
            }
        }
        tx.commit()?;
        Ok((report, page.next_before))
    }
}

/// Une entrée illisible est écartée (journalisée), pas fatale à la page.
fn parse<T: for<'de> Deserialize<'de>>(entry: Value) -> Option<T> {
    match serde_json::from_value(entry) {
        Ok(parsed) => Some(parsed),
        Err(err) => {
            tracing::warn!(%err, "import de l'historique : entrée illisible écartée");
            None
        }
    }
}

fn existing(tx: &Transaction<'_>, table: &str, key: &str) -> rusqlite::Result<Option<i64>> {
    tx.query_row(
        &format!("SELECT id FROM {table} WHERE client_key = ?1"),
        [key],
        |row| row.get(0),
    )
    .optional()
}

fn json_list(value: &Value) -> String {
    if value.is_array() {
        value.to_string()
    } else {
        "[]".to_string()
    }
}

/// `Ok(Some(true))` : insérée ; `Ok(Some(false))` : déjà là ; `Ok(None)` : date illisible.
fn import_fight(tx: &Transaction<'_>, fight: FightEntry) -> rusqlite::Result<Option<bool>> {
    let Some(started_at) = parse_iso_utc_ms(&fight.started_at) else {
        return Ok(None);
    };
    if let Some(id) = existing(tx, "fights", &fight.client_key)? {
        // Déjà là : seul le rattachement de donjon peut compléter la ligne, jamais l'écraser.
        tx.execute(
            "UPDATE fights SET
                fight_type = CASE WHEN dungeon_id IS NULL AND ?2 IS NOT NULL
                                  THEN COALESCE(?4, fight_type) ELSE fight_type END,
                dungeon_id = COALESCE(dungeon_id, ?2),
                dungeon_run_key = COALESCE(dungeon_run_key, ?3)
             WHERE id = ?1",
            params![
                id,
                fight.dungeon_id,
                fight.dungeon_run_key,
                fight.fight_type
            ],
        )?;
        return Ok(Some(false));
    }
    tx.execute(
        "INSERT INTO fights (signature, client_key, started_at, duration_ms, won, turns,
            total_damage, xp_gained, kamas_gained, game_server, dungeon_id, dungeon_run_key,
            challenges_passed, challenges_failed, fight_type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            format!("{IMPORTED_SIGNATURE_PREFIX}{}", fight.client_key),
            fight.client_key,
            started_at,
            fight.duration_ms,
            fight.won,
            fight.turns,
            fight.total_damage,
            fight.xp_gained,
            fight.kamas_gained,
            fight.game_server,
            fight.dungeon_id,
            fight.dungeon_run_key,
            fight.challenges_passed,
            fight.challenges_failed,
            fight.fight_type,
        ],
    )?;
    let fight_id = tx.last_insert_rowid();
    let mut participant = tx.prepare_cached(
        "INSERT INTO fight_participants (fight_id, name, instance_index, side, monster_id,
            class_name, damage, defeated, fled, spells, heal, armor, heal_spells, armor_spells,
            xp_gained)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT (fight_id, name, instance_index) DO NOTHING",
    )?;
    for p in &fight.participants {
        if p.side != "ally" && p.side != "enemy" {
            continue;
        }
        participant.execute(params![
            fight_id,
            p.name,
            p.instance_index,
            p.side,
            p.monster_id,
            p.class_name,
            p.damage,
            p.defeated,
            p.fled,
            json_list(&p.spells),
            p.heal,
            p.armor,
            json_list(&p.heal_spells),
            json_list(&p.armor_spells),
            p.xp_gained,
        ])?;
    }
    let mut loot = tx.prepare_cached(
        "INSERT INTO fight_loot (fight_id, line_index, item_id, item_name, quantity)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (fight_id, line_index) DO NOTHING",
    )?;
    for (position, row) in fight.loot.iter().enumerate() {
        loot.execute(params![
            fight_id,
            row.line_index.unwrap_or(position as i64),
            row.item_id,
            row.item_name,
            row.quantity
        ])?;
    }
    Ok(Some(true))
}

fn import_purchase(
    tx: &Transaction<'_>,
    purchase: PurchaseEntry,
) -> rusqlite::Result<Option<bool>> {
    let Some(occurred_at) = parse_iso_utc_ms(&purchase.occurred_at) else {
        return Ok(None);
    };
    if existing(tx, "purchases", &purchase.client_key)?.is_some() {
        return Ok(Some(false));
    }
    tx.execute(
        "INSERT INTO purchases (signature, client_key, item_id, item_name, quantity, total_cost,
            occurred_at, game_server)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            format!("{IMPORTED_SIGNATURE_PREFIX}{}", purchase.client_key),
            purchase.client_key,
            purchase.item_id,
            purchase.item_name,
            purchase.quantity,
            purchase.total_cost,
            occurred_at,
            purchase.game_server,
        ],
    )?;
    Ok(Some(true))
}

fn import_trade(tx: &Transaction<'_>, trade: TradeEntry) -> rusqlite::Result<Option<bool>> {
    let Some(occurred_at) = parse_iso_utc_ms(&trade.occurred_at) else {
        return Ok(None);
    };
    if existing(tx, "trades", &trade.client_key)?.is_some() {
        return Ok(Some(false));
    }
    tx.execute(
        "INSERT INTO trades (signature, client_key, peer_name, self_name, occurred_at,
            kamas_acquired, kamas_given, game_server)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            format!("{IMPORTED_SIGNATURE_PREFIX}{}", trade.client_key),
            trade.client_key,
            trade.peer_name,
            trade.self_name,
            occurred_at,
            trade.kamas_acquired,
            trade.kamas_given,
            trade.game_server,
        ],
    )?;
    let trade_id = tx.last_insert_rowid();
    let mut item = tx.prepare_cached(
        "INSERT INTO trade_items (trade_id, direction, line_index, item_id, item_name, quantity)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for (direction, rows) in [("acquired", &trade.acquired), ("given", &trade.given)] {
        for (line_index, row) in rows.iter().enumerate() {
            item.execute(params![
                trade_id,
                direction,
                line_index as i64,
                row.item_id,
                row.item_name,
                row.quantity
            ])?;
        }
    }
    Ok(Some(true))
}

fn import_pact(tx: &Transaction<'_>, pact: PactEntry) -> rusqlite::Result<Option<bool>> {
    let Some(occurred_at) = parse_iso_utc_ms(&pact.occurred_at) else {
        return Ok(None);
    };
    if existing(tx, "pact_extractions", &pact.client_key)?.is_some() {
        return Ok(Some(false));
    }
    tx.execute(
        "INSERT INTO pact_extractions (signature, client_key, occurred_at, game_server)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            format!("{IMPORTED_SIGNATURE_PREFIX}{}", pact.client_key),
            pact.client_key,
            occurred_at,
            pact.game_server,
        ],
    )?;
    let extraction_id = tx.last_insert_rowid();
    let mut item = tx.prepare_cached(
        "INSERT INTO pact_extraction_items (extraction_id, line_index, item_id, item_name,
            quantity)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for (line_index, row) in pact.items.iter().enumerate() {
        item.execute(params![
            extraction_id,
            line_index as i64,
            row.item_id,
            row.item_name,
            row.quantity
        ])?;
    }
    Ok(Some(true))
}
