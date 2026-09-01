//! Frontière métier (docs/plan-architecture.md §2 et §9, lot L2) : transforme les `LineBatch`
//! d'`overlay-ingest` en `LogEntry` typés (QuickJS + `LogParser` vendu depuis `wakfu-companion`,
//! voir `engine-js/`) puis en `SessionSnapshot` agrégé (Rust, voir `session.rs`) — les deux
//! premières briques que les panneaux de l'UI (L2) afficheront.

pub mod catalog;
pub mod class_breed;
pub mod model;
pub mod quickjs_engine;
pub mod roster;
pub mod session;
pub mod watchlist;

pub use catalog::{CatalogIndex, IconKind, IconRef};
pub use model::{ChatChannel, DamageElement, FightResult, LogEntry, TradeItem, TradeSide};
pub use quickjs_engine::{EngineError, LogParserEngine};
pub use roster::{Gender, RosterCharacter, RosterIndex};
pub use session::{Engine, FightSnapshot, FighterDamage, LootItem, SessionSnapshot, SessionTotals};
pub use watchlist::{
    watchlist_from_settings_json, WatchlistAlert, WatchlistEntry, WatchlistKind, WatchlistMode,
};
