//! Événements d'historique synchronisés vers le compte (L5, §7.1 du plan) — miroir Rust de
//! `history-event.model.ts` côté web : mêmes formes de payload, mêmes formules de signature.
//!
//! **Statut (2026-09-02, complété depuis le dépôt `wakfu-companion` local)** : la quasi-totalité
//! des champs listés comme volontairement partiels dans une itération précédente de ce module sont
//! désormais alimentés — ventilation par sort/élément (`spells`), `xpGained` par participant,
//! résolution `monsterId`/`itemId` via le catalogue (voir `session::build_fight_sync_event`),
//! `gameServer` (déduit du dernier personnage du roster reconnu dans le log — voir
//! `session::Engine::current_game_server`, miroir de `GameServerService`), et récupération de
//! kamas HDV sans achat adjacent (`HDV_KAMAS_SALE_ITEM`, voir `session::SessionState::apply` et
//! `considerHdvKamaGain`/`resolvePendingHdvKamaGain` côté `stats-store.service.ts`).
//!
//! **Reste volontairement hors périmètre** (voir la doc de `FightPayload::dungeon_id` pour le
//! détail) : le regroupement de plusieurs combats en un seul run de donjon multi-salles
//! (`dungeon-run-grouping.util.ts`, ~240 lignes de heuristique) et la détection de brèche/brèche
//! ultime (`findDungeonForEnemies`, priorités 0 et 2) — seul le cas « ce combat contient lui-même
//! le boss d'un donjon classique » est porté. `turns` (nombre de tours) reste également à `0` :
//! rien dans `overlay-engine::session` ne compte les tours aujourd'hui (aucun panneau n'en affiche
//! le besoin, voir §9 du plan), et ce champ n'entre dans aucune signature/idempotence.
//!
//! Tous ces champs acceptent `null`/liste vide côté serveur (`server/history/parse.ts`) : un champ
//! non résolu (catalogue pas encore chargé, roster vide, personnage jamais reconnu) part donc tel
//! quel, jamais une erreur.

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

/// Nom d'objet sentinelle d'une récupération de kamas à l'Hôtel de Vente — miroir exact de
/// `HDV_KAMAS_SALE_ITEM` (`stats-store.service.ts`) : un gain de kamas hors combat (`KamaGain`
/// sans `fightId`) non expliqué par un échange tout juste conclu est enregistré comme un
/// `PurchasePayload` avec ce nom, `quantity = 0` (ni `itemId` ni vraie quantité, uniquement le
/// montant dans `totalCost`) — même endpoint que l'achat classique, seul le nom sentinelle permet
/// de distinguer les deux dans la même table côté serveur (`fights.kamasFromHdvSales` vs
/// `kamasSpentOnPurchases`). Voir `session::SessionState::apply`.
pub const HDV_KAMAS_SALE_ITEM: &str = "__hdv_kamas_sale__";

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
    /// Id Ankama du donjon dont ce combat contient LUI-MÊME le boss — résolu via
    /// `DungeonIndex::find_by_boss_monster_id` sur les ennemis de ce combat (voir
    /// `session::resolve_dungeon_assignment`). `None` hors donjon, mais aussi pour une simple
    /// SALLE d'un donjon multi-combats dont le boss n'est pas dans CE combat précis : le
    /// regroupement multi-salles (`dungeon-run-grouping.util.ts` côté web, groupDungeonRuns) n'est
    /// volontairement pas porté — heuristique de corrélation entre PLUSIEURS combats, exactement
    /// la catégorie que le §2 du plan réserve au moteur TS partagé, pas à ce module Rust. Le
    /// serveur tolère déjà ce cas (`fights.ts`, `COALESCE` sur `dungeonId`/`dungeonRunKey` :
    /// « quand le boss apparaîtra à son tour dans l'historique connu ») : une salle envoyée sans
    /// rattachement aujourd'hui n'est pas une donnée perdue, juste un rattachement différé.
    pub dungeon_id: Option<i64>,
    /// Signature de contenu du combat REPRÉSENTATIF du run (voir `fight_signature`) — jamais
    /// envoyée telle quelle : `overlay_sync::queue::flush_once` la hache en `dungeonRunKey` au
    /// moment de l'envoi via `client_key(uid, Fight, signature)`, EXACTEMENT la même fonction que
    /// pour `clientKey` (miroir de `SyncQueueService.send`, vérifié dans `sync-queue.service.ts`
    /// du dépôt web : `computeClientKey(uid, entry.kind, dungeonRunSignature)`). Dans le seul cas
    /// porté ici (ce combat contient son propre boss), cette signature est TOUJOURS identique à la
    /// signature du combat lui-même — d'où `dungeonRunKey` du run == `clientKey` du combat de boss,
    /// sans aller-retour serveur pour l'obtenir (voir `history-sync.service.ts::runSignature`).
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
