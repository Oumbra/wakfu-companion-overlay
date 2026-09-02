//! Frontière métier (docs/plan-architecture.md §2 et §9, lot L2) : transforme les `LineBatch`
//! d'`overlay-ingest` en `LogEntry` typés (QuickJS + `LogParser` vendu depuis `wakfu-companion`,
//! voir `engine-js/`) puis en `SessionSnapshot` agrégé (Rust, voir `session.rs`) — les deux
//! premières briques que les panneaux de l'UI (L2) afficheront.

pub mod catalog;
pub mod class_breed;
pub mod dungeon;
pub mod fight_store;
pub mod history;
pub mod log_time;
pub mod model;
pub mod monster_family;
pub mod profile;
pub mod quickjs_engine;
pub mod roster;
pub mod session;
pub mod watchlist;

pub use catalog::{CatalogIndex, IconKind, IconRef, MonsterClassification, WakfuRarity};
pub use dungeon::{DungeonEntry, DungeonIndex};
pub use history::{
    fight_signature, purchase_signature, trade_signature, FightLootPayload,
    FightParticipantPayload, FightPayload, FightSide, FightSpellPayload, HistoryEventKind,
    HistoryPayload, PurchasePayload, SyncEvent, TradeDirection, TradeItemPayload, TradePayload,
    HDV_KAMAS_SALE_ITEM,
};
pub use model::{ChatChannel, DamageElement, FightResult, LogEntry, TradeItem, TradeSide};
pub use monster_family::{MonsterFamilyEntry, MonsterFamilyIndex};
pub use profile::{sound_items_from_settings_json, LootAlert, SoundItemEntry};
pub use quickjs_engine::{EngineError, LogParserEngine};
pub use roster::{Gender, RosterCharacter, RosterIndex};
pub use session::{Engine, FightSnapshot, FighterDamage, LootItem, SessionSnapshot, SessionTotals};
pub use watchlist::{
    watchlist_from_settings_json, WatchlistAlert, WatchlistEntry, WatchlistKind, WatchlistMode,
};
