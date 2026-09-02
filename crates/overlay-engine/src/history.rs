//! Événements d'historique synchronisés vers le compte (L5, §7.1 du plan) — miroir Rust de
//! `history-event.model.ts` côté web : mêmes formes de payload, mêmes formules de signature.
//!
//! **Parité assumée volontairement partielle sur cette première itération** — voir §7 du plan
//! (statut L5) pour le détail complet des champs non encore alimentés :
//! - `FightPayload` : pas de ventilation par sort (`spells` toujours vide, `overlay-engine::
//!   session` ne suit que des totaux dégâts/soin par combattant, pas par sort/élément) ; pas
//!   d'`xpGained` PAR participant (seul le total de session est suivi) ; pas d'assignation de
//!   donjon (`dungeonId`/`dungeonRunSignature` toujours `None`).
//! - `gameServer` toujours `None` sur les trois payloads (pas de détection du serveur de jeu
//!   côté overlay pour l'instant).
//! - La "récupération de kamas HDV" (`HDV_KAMAS_SALE_ITEM`, gain de kamas hors combat corrélé à
//!   AUCUN achat/échange) n'est pas détectée : seul l'achat classique (perte de kamas suivie d'un
//!   ramassage, voir `session.rs::apply`) l'est.
//!
//! Ces champs acceptent tous `null`/liste vide côté serveur (`server/history/parse.ts`) : les
//! événements envoyés sont donc valides dès aujourd'hui, seulement moins détaillés que ce que
//! `StatsStoreService` produit — jamais rejetés pour autant.

use std::collections::HashMap;

use serde::Serialize;

/// Type d'un événement d'historique — détermine l'endpoint (`endpoint()`) ET entre dans le calcul
/// du `clientKey` (`overlay_sync::queue::client_key`, formule `sha256(uid|kind|signature)`,
/// identique au web) : la chaîne renvoyée par `as_str` DOIT rester `"fight"`/`"purchase"`/
/// `"trade"` à l'identique de `HistoryEventKind` côté TS, jamais renommée sans coordination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HistoryEventKind {
    Fight,
    Purchase,
    Trade,
}

impl HistoryEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            HistoryEventKind::Fight => "fight",
            HistoryEventKind::Purchase => "purchase",
            HistoryEventKind::Trade => "trade",
        }
    }

    /// Chemin d'API relatif — miroir de `HISTORY_ENDPOINTS` (`history-event.model.ts`).
    pub fn endpoint_path(self) -> &'static str {
        match self {
            HistoryEventKind::Fight => "/api/v1/history/fights",
            HistoryEventKind::Purchase => "/api/v1/history/purchases",
            HistoryEventKind::Trade => "/api/v1/history/trades",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "fight" => Some(HistoryEventKind::Fight),
            "purchase" => Some(HistoryEventKind::Purchase),
            "trade" => Some(HistoryEventKind::Trade),
            _ => None,
        }
    }
}

/// `value.trim().to_lowercase()` — miroir de `normalize()` (`history-event.model.ts`). La
/// casse Unicode peut différer marginalement de `String.prototype.toLowerCase()` sur des scripts
/// non latins (non pertinent ici : noms de personnages/objets Wakfu, alphabet latin étendu).
fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FightSpellPayload {
    pub spell: String,
    pub total: i64,
    pub by_element: HashMap<String, i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FightSide {
    Ally,
    Enemy,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FightParticipantPayload {
    pub side: FightSide,
    pub name: String,
    pub monster_id: Option<i64>,
    pub instance_index: i64,
    pub class_name: Option<String>,
    pub damage: i64,
    pub defeated: bool,
    pub fled: bool,
    pub spells: Vec<FightSpellPayload>,
    pub xp_gained: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FightLootPayload {
    pub item_id: Option<i64>,
    pub item_name: Option<String>,
    pub quantity: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FightPayload {
    pub fight_id: Option<i64>,
    pub started_at: String,
    pub duration_ms: Option<i64>,
    pub won: bool,
    pub turns: i64,
    pub total_damage: i64,
    pub xp_gained: i64,
    pub kamas_gained: Option<i64>,
    pub game_server: Option<String>,
    pub dungeon_id: Option<i64>,
    /// Jamais envoyé tel quel : `overlay_sync::queue` le hache en `dungeonRunKey` au moment de
    /// l'envoi, comme `SyncQueueService.send` côté web — voir la doc de `FightPayload`
    /// ci-dessus (toujours `None` cette itération, aucune assignation de donjon encore portée).
    #[serde(skip_serializing)]
    pub dungeon_run_signature: Option<String>,
    pub challenges_passed: i64,
    pub challenges_failed: i64,
    pub participants: Vec<FightParticipantPayload>,
    pub loot: Vec<FightLootPayload>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchasePayload {
    pub item_id: Option<i64>,
    pub item_name: Option<String>,
    pub quantity: i64,
    pub total_cost: i64,
    pub occurred_at: String,
    pub game_server: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeDirection {
    Acquired,
    Given,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeItemPayload {
    pub direction: TradeDirection,
    pub item_id: Option<i64>,
    pub item_name: Option<String>,
    pub quantity: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradePayload {
    pub peer_name: String,
    pub self_name: String,
    pub occurred_at: String,
    pub kamas_acquired: i64,
    pub kamas_given: i64,
    pub game_server: Option<String>,
    pub items: Vec<TradeItemPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum HistoryPayload {
    Fight(FightPayload),
    Purchase(PurchasePayload),
    Trade(TradePayload),
}

/// Un événement prêt à mettre en file — voir `overlay_sync::queue::SyncQueue::enqueue`.
/// `id()` (`"{kind}:{signature}"`) est la clé primaire de la file SQLite : dédoublonnage naturel,
/// comme `HistoryEvent.id` en IndexedDB côté web.
#[derive(Debug, Clone)]
pub struct SyncEvent {
    pub kind: HistoryEventKind,
    pub signature: String,
    pub payload: HistoryPayload,
}

impl SyncEvent {
    pub fn id(&self) -> String {
        format!("{}:{}", self.kind.as_str(), self.signature)
    }
}

/// Miroir de `fightSignature` — heure de fin, id de combat du log, résultat, et **noms** des
/// participants triés (jamais leurs dégâts, révisables après coup par une réattribution manuelle
/// côté web — non applicable à l'overlay pour l'instant, mais la formule reste identique pour ne
/// jamais diverger le jour où elle le devient).
pub fn fight_signature(
    time: &str,
    fight_id: i64,
    won: bool,
    participants: &[(String, i64)],
) -> String {
    let mut names: Vec<String> = participants
        .iter()
        .map(|(name, instance_index)| format!("{}#{instance_index}", normalize(name)))
        .collect();
    names.sort();
    let result = if won { "won" } else { "lost" };
    format!("{time}|{fight_id}|{result}|{}", names.join(","))
}

/// Miroir de `purchaseSignature`.
pub fn purchase_signature(time: &str, item: &str, quantity: i64, total_cost: i64) -> String {
    format!("{time}|{}|{quantity}|{total_cost}", normalize(item))
}

/// Miroir de `tradeSignature`.
pub fn trade_signature(
    time: &str,
    peer_name: &str,
    self_name: &str,
    kamas_acquired: i64,
    kamas_given: i64,
    items: &[(TradeDirection, String, i64)],
) -> String {
    let mut parts: Vec<String> = items
        .iter()
        .map(|(direction, name, quantity)| {
            let dir = match direction {
                TradeDirection::Acquired => "acquired",
                TradeDirection::Given => "given",
            };
            format!("{dir}:{}x{quantity}", normalize(name))
        })
        .collect();
    parts.sort();
    format!(
        "{time}|{}|{}|{kamas_acquired}|{kamas_given}|{}",
        normalize(peer_name),
        normalize(self_name),
        parts.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vecteurs de référence recalculés à la main depuis les formules TS (`fightSignature`/
    // `purchaseSignature`/`tradeSignature`, history-event.model.ts) — toute divergence future
    // d'une des deux implémentations casserait l'idempotence croisée web/overlay (§7 du plan,
    // « rejeu 10x ... y compris en alternant web et overlay »).

    #[test]
    fn fight_signature_matches_ts_formula() {
        let sig = fight_signature(
            "14:13:32,174",
            42,
            true,
            &[
                ("Bwork Mage".to_string(), 1),
                ("Chef Bandit ".to_string(), 0),
            ],
        );
        assert_eq!(sig, "14:13:32,174|42|won|bwork mage#1,chef bandit#0");
    }

    #[test]
    fn purchase_signature_matches_ts_formula() {
        let sig = purchase_signature("10:00:00,000", "  Eclat de Wakfu ", 3, 1500);
        assert_eq!(sig, "10:00:00,000|eclat de wakfu|3|1500");
    }

    #[test]
    fn trade_signature_matches_ts_formula() {
        let sig = trade_signature(
            "11:00:00,000",
            "Peer",
            "Self",
            100,
            0,
            &[
                (TradeDirection::Given, "Kwak".to_string(), 2),
                (TradeDirection::Acquired, "Bois".to_string(), 1),
            ],
        );
        assert_eq!(
            sig,
            "11:00:00,000|peer|self|100|0|acquired:boisx1,given:kwakx2"
        );
    }

    #[test]
    fn event_id_dedupes_by_kind_and_signature() {
        let event = SyncEvent {
            kind: HistoryEventKind::Purchase,
            signature: "sig".to_string(),
            payload: HistoryPayload::Purchase(PurchasePayload {
                item_id: None,
                item_name: Some("x".to_string()),
                quantity: 1,
                total_cost: 1,
                occurred_at: "2026-01-01T00:00:00.000Z".to_string(),
                game_server: None,
            }),
        };
        assert_eq!(event.id(), "purchase:sig");
    }
}
