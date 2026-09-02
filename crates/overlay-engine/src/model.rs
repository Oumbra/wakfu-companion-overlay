//! Miroir Rust exact de `engine-js/src/log-entry.model.ts` — une seule implémentation de
//! *sémantique* (le TS vendu, exécuté par QuickJS), ce module n'en décrit que la **forme** pour
//! pouvoir la désérialiser côté Rust. Ne jamais ajouter de champ ou de variante ici qui n'existe
//! pas dans le TS source : toute divergence de forme romprait la désérialisation en silence sur
//! les champs inconnus (`serde` ignore par défaut les champs JSON non déclarés).
//!
//! Champs renommés depuis le camelCase TS (`fightId`, `isControlledByAI`, …) vers le snake_case
//! Rust via `#[serde(rename = "...")]`, un par un plutôt que `rename_all_fields` — plus verbeux,
//! mais sans dépendre d'une version précise de `serde` pour cette fonctionnalité.

use serde::{Deserialize, Serialize};

/// Éléments de dégâts/soins reconnus — miroir de `DamageElement`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum DamageElement {
    Neutre,
    Terre,
    Feu,
    Eau,
    Air,
    #[serde(rename = "Lumière")]
    Lumiere,
    Stasis,
    Inconnu,
}

/// Canaux de chat — miroir de `ChatChannelKey`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatChannel {
    Proximite,
    Groupe,
    Guilde,
    Recrutement,
    Commerce,
    Communaute,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TradeItem {
    pub name: String,
    pub quantity: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TradeSide {
    #[serde(rename = "playerName")]
    pub player_name: String,
    pub items: Vec<TradeItem>,
    pub kamas: i64,
}

/// Miroir exact de l'union `LogEntry` (`engine-js/src/log-entry.model.ts`) — voir le fichier
/// source vendu pour la documentation de chaque variante, non dupliquée ici.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LogEntry {
    Chat {
        time: String,
        channel: ChatChannel,
        #[serde(rename = "channelLabel")]
        channel_label: String,
        author: String,
        message: String,
    },
    KamaGain {
        time: String,
        amount: i64,
        #[serde(rename = "fightId")]
        fight_id: Option<i64>,
    },
    KamaLoss {
        time: String,
        amount: i64,
    },
    XpGain {
        time: String,
        character: String,
        amount: i64,
        #[serde(rename = "fightId")]
        fight_id: Option<i64>,
    },
    SpellCast {
        time: String,
        caster: String,
        spell: String,
        critical: bool,
        #[serde(rename = "fightId")]
        fight_id: Option<i64>,
    },
    Damage {
        time: String,
        target: String,
        attacker: String,
        spell: String,
        element: DamageElement,
        amount: i64,
        #[serde(rename = "fightId")]
        fight_id: Option<i64>,
    },
    Heal {
        time: String,
        target: String,
        attacker: String,
        spell: String,
        element: DamageElement,
        amount: i64,
        #[serde(rename = "fightId")]
        fight_id: Option<i64>,
    },
    Armor {
        time: String,
        target: String,
        attacker: String,
        spell: String,
        amount: i64,
        #[serde(rename = "fightId")]
        fight_id: Option<i64>,
    },
    EnemyDefeated {
        time: String,
        name: String,
        #[serde(rename = "fightId")]
        fight_id: Option<i64>,
    },
    EnemyFled {
        time: String,
        name: String,
        #[serde(rename = "fightId")]
        fight_id: Option<i64>,
    },
    CombatDefeatMarker {
        time: String,
        #[serde(rename = "fightId")]
        fight_id: Option<i64>,
    },
    CombatStart {
        time: String,
    },
    CombatEnd {
        time: String,
        #[serde(rename = "fightId")]
        fight_id: i64,
        result: FightResult,
    },
    Loot {
        time: String,
        item: String,
        quantity: i64,
        #[serde(rename = "fightId")]
        fight_id: Option<i64>,
    },
    MarketOccupation {
        time: String,
        active: bool,
    },
    ChallengeResult {
        time: String,
        name: String,
        success: bool,
        #[serde(rename = "fightId")]
        fight_id: Option<i64>,
    },
    LogDateAnchor {
        time: String,
        year: i64,
        month: i64,
        day: i64,
    },
    FighterJoined {
        time: String,
        #[serde(rename = "fightId")]
        fight_id: i64,
        name: String,
        breed: i64,
        #[serde(rename = "fighterId")]
        fighter_id: i64,
        #[serde(rename = "isControlledByAI")]
        is_controlled_by_ai: bool,
        #[serde(rename = "summonedBy")]
        summoned_by: Option<String>,
    },
    TradeCompleted {
        time: String,
        sides: [TradeSide; 2],
    },
}

// `Serialize` sert à la persistance disque du combat en cours (voir `fight_store.rs`) —
// `FightSnapshot::result` doit rester (re)sérialisable même si sa valeur reste `None` tant que le
// combat n'est pas terminé (seuls les combats `ongoing` sont persistés, voir `fight_store::save_fight`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FightResult {
    Won,
    Lost,
}

impl LogEntry {
    /// `time` (`HH:MM:SS,mmm`) est présent sur toutes les variantes — utile pour trier/afficher
    /// sans `match` exhaustif à chaque appelant.
    pub fn time(&self) -> &str {
        match self {
            LogEntry::Chat { time, .. }
            | LogEntry::KamaGain { time, .. }
            | LogEntry::KamaLoss { time, .. }
            | LogEntry::XpGain { time, .. }
            | LogEntry::SpellCast { time, .. }
            | LogEntry::Damage { time, .. }
            | LogEntry::Heal { time, .. }
            | LogEntry::Armor { time, .. }
            | LogEntry::EnemyDefeated { time, .. }
            | LogEntry::EnemyFled { time, .. }
            | LogEntry::CombatDefeatMarker { time, .. }
            | LogEntry::CombatStart { time }
            | LogEntry::CombatEnd { time, .. }
            | LogEntry::Loot { time, .. }
            | LogEntry::MarketOccupation { time, .. }
            | LogEntry::ChallengeResult { time, .. }
            | LogEntry::LogDateAnchor { time, .. }
            | LogEntry::FighterJoined { time, .. }
            | LogEntry::TradeCompleted { time, .. } => time,
        }
    }
}
