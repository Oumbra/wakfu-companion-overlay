//! Frontière métier (docs/plan-architecture.md §2 et §9, lot L2) : transforme les `LineBatch`
//! d'`overlay-ingest` en `LogEntry` typés (QuickJS + `LogParser` vendu depuis `wakfu-companion`,
//! voir `engine-js/`) puis en `SessionSnapshot` agrégé (Rust, voir `session.rs`) — les deux
//! premières briques que les panneaux de l'UI (L2) afficheront.

pub mod catalog;
pub mod chat_alert;
pub mod class_breed;
pub mod dungeon;
pub mod dungeon_run;
pub mod fight_store;
pub mod history;
pub mod log_time;
pub mod model;
pub mod monster_family;
pub mod profile;
pub mod quickjs_engine;
pub mod recipe;
pub mod roster;
pub mod session;
pub mod spells;
pub mod watchlist;

pub use catalog::{
    CatalogIndex, IconKind, IconRef, ItemSuggestion, MonsterClassification, MonsterSuggestion,
    WakfuItemCategory, WakfuRarity,
};
pub use chat_alert::{
    channel_label, chat_filters_from_account_data, chat_filters_from_settings_json,
    chat_filters_patch_entry, chat_filters_to_settings_json, matching_filter, ChatAlert,
    ChatFilter, ChatFilterScope, CHAT_CHANNELS,
};
pub use dungeon::{DungeonEntry, DungeonIndex, WakfuDungeonType};
pub use history::{
    fight_signature, purchase_signature, trade_signature, FightLootPayload,
    FightParticipantPayload, FightPayload, FightSide, FightSpellPayload, HistoryEventKind,
    HistoryPayload, PurchasePayload, SyncEvent, TradeDirection, TradeItemPayload, TradePayload,
    HDV_KAMAS_SALE_ITEM,
};
pub use model::{ChatChannel, DamageElement, FightResult, LogEntry, TradeItem, TradeSide};
pub use monster_family::{MonsterFamilyEntry, MonsterFamilyIndex};
pub use profile::{
    find_enabled_sound_item, profile_patch_entry, AlertProfile, LootAlert, SoundItemEntry,
    DEFAULT_ALERT_DURATION_SECONDS, DEFAULT_SOUND_ITEM_NAMES, MAX_ALERT_DURATION_SECONDS,
    MIN_ALERT_DURATION_SECONDS,
};
pub use quickjs_engine::{EngineError, LogParserEngine};
pub use recipe::{flatten_for_tracking, resolve_recipe, RecipeIngredient};
pub use roster::{roster_patch_entry, Gender, Roster, RosterAccount, RosterCharacter, RosterIndex};
pub use session::{
    Engine, FightSnapshot, FighterDamage, LootItem, SessionSnapshot, SessionTotals,
    SpellCastRecord, MAX_LAST_TURN_CASTS,
};
pub use spells::{
    preload_embedded as preload_spell_indexes, referential_path, resolve_cast, MonsterSpellIndex,
    ResolvedSpell, SpellEntry, SpellIndex, SpellTable,
};
pub use watchlist::{
    watchlist_from_settings_json, watchlist_patch_entry, WatchlistAlert, WatchlistEntry,
    WatchlistKind, WatchlistMode,
};
