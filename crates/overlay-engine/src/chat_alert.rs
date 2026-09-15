//! Recherches de chat et alertes qu'elles déclenchent — miroir de la partie « filtres » de
//! `ChatPanelService` (`chat-panel.service.ts`) côté web, et rien d'autre : le jeu affiche déjà
//! son chat, l'overlay n'en montre pas une ligne. Ce qu'il apporte, c'est **d'être prévenu** :
//! quand un message correspond à une recherche (un mot, sur un canal ou sur tous), le moteur émet
//! une [`ChatAlert`] que l'hôte transforme en son et en carte par-dessus le jeu — même chemin que
//! `LootAlert` (`profile.rs`), file séparée (`Engine::drain_chat_alerts`).
//!
//! ## La règle de correspondance, celle du web
//!
//! `messageMatchesAnyFilter` : une recherche s'applique si son canal est « global » ou celui du
//! message ; elle correspond si le **texte OU l'auteur**, en minuscules, contient le mot (déjà
//! rangé trimé et en minuscules). Pas d'expression régulière, pas de mot entier, pas de
//! normalisation d'accents — reproduire davantage serait inventer.
//!
//! ## Le format stocké, celui du web
//!
//! La clé `chatFilters` de `/api/v1/settings` porte `[{ "text": "gelano", "channel": "commerce" }]`,
//! `channel` valant `"global"` ou la clé d'un canal (`proximite`, `groupe`, `guilde`,
//! `recrutement`, `commerce`, `communaute`). Un ancien élément peut être une simple chaîne
//! (`"gelano"`), relue comme une recherche globale — la même migration douce que le web.
//! L'overlay lit ce format et l'écrit à l'identique : un compte utilisé depuis le site et depuis
//! l'overlay voit la même liste des deux côtés.

use serde::{Deserialize, Serialize};

use crate::model::ChatChannel;

/// Les six canaux, dans l'ordre du web (`CHAT_CHANNELS`, `log-parser.ts`).
pub const CHAT_CHANNELS: [ChatChannel; 6] = [
    ChatChannel::Proximite,
    ChatChannel::Groupe,
    ChatChannel::Guilde,
    ChatChannel::Recrutement,
    ChatChannel::Commerce,
    ChatChannel::Communaute,
];

/// Libellé affichable d'un canal — celui que le parser web pose dans `channelLabel`.
///
/// Ici et non sur `ChatChannel` : `model.rs` est un miroir strict du TS, il ne porte que ce que
/// le bundle sérialise.
pub fn channel_label(channel: ChatChannel) -> &'static str {
    match channel {
        ChatChannel::Proximite => "Proximité",
        ChatChannel::Groupe => "Groupe",
        ChatChannel::Guilde => "Guilde",
        ChatChannel::Recrutement => "Recrutement",
        ChatChannel::Commerce => "Commerce",
        ChatChannel::Communaute => "Communauté",
    }
}

/// Sur quels messages une recherche s'applique — miroir de `ChatFilterChannel`
/// (`ChatChannelKey | 'global'`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatFilterScope {
    /// Tous les canaux — `'global'` côté web.
    All,
    Channel(ChatChannel),
}

impl ChatFilterScope {
    /// Libellé affichable : le nom du canal, ou « Tous les canaux ».
    pub fn label(self) -> &'static str {
        match self {
            ChatFilterScope::All => "Tous les canaux",
            ChatFilterScope::Channel(channel) => channel_label(channel),
        }
    }
}

/// Une recherche : un mot, une portée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatFilter {
    /// **Trimé et en minuscules**, comme `addFilter` le range côté web — la comparaison n'a
    /// plus qu'à abaisser le message.
    pub text: String,
    pub scope: ChatFilterScope,
}

impl ChatFilter {
    /// Range un mot saisi. `None` si, une fois trimé, il ne reste rien : le web refuse aussi une
    /// recherche vide.
    pub fn new(scope: ChatFilterScope, raw: &str) -> Option<Self> {
        let text = raw.trim().to_lowercase();
        if text.is_empty() {
            return None;
        }
        Some(Self { text, scope })
    }
}

/// Une recherche telle que le web la stocke — forme de transport, jamais manipulée ailleurs.
#[derive(Debug, Serialize, Deserialize)]
struct StoredFilter {
    text: String,
    channel: String,
}

/// Ancien ou nouveau format d'un élément de la liste stockée.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StoredItem {
    Legacy(String),
    Current(StoredFilter),
}

const GLOBAL_KEY: &str = "global";

fn scope_from_key(key: &str) -> Option<ChatFilterScope> {
    if key == GLOBAL_KEY {
        return Some(ChatFilterScope::All);
    }
    // `ChatChannel` désérialise depuis sa clé minuscule (`#[serde(rename_all = "lowercase")]`) :
    // la même table que le bundle, sans la recopier.
    serde_json::from_value::<ChatChannel>(serde_json::Value::String(key.to_owned()))
        .ok()
        .map(ChatFilterScope::Channel)
}

fn scope_key(scope: ChatFilterScope) -> String {
    match scope {
        ChatFilterScope::All => GLOBAL_KEY.to_owned(),
        ChatFilterScope::Channel(channel) => serde_json::to_value(channel)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| GLOBAL_KEY.to_owned()),
    }
}

/// Lit la valeur de la clé `chatFilters` du compte. Une valeur absente, nulle ou d'une autre
/// forme donne une liste vide ; un élément illisible (canal inconnu, mot vide) est ignoré, jamais
/// fatal — comme la migration douce du web, qui relit ce qu'elle peut.
pub fn chat_filters_from_settings_json(value: &serde_json::Value) -> Vec<ChatFilter> {
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(
            |item| match serde_json::from_value::<StoredItem>(item.clone()).ok()? {
                StoredItem::Legacy(text) => ChatFilter::new(ChatFilterScope::All, &text),
                StoredItem::Current(stored) => {
                    ChatFilter::new(scope_from_key(&stored.channel)?, &stored.text)
                }
            },
        )
        .collect()
}

/// La valeur à écrire sous la clé `chatFilters` — le format courant du web, jamais l'ancien.
pub fn chat_filters_to_settings_json(filters: &[ChatFilter]) -> serde_json::Value {
    serde_json::to_value(
        filters
            .iter()
            .map(|f| StoredFilter {
                text: f.text.clone(),
                channel: scope_key(f.scope),
            })
            .collect::<Vec<_>>(),
    )
    .unwrap_or(serde_json::Value::Array(Vec::new()))
}

/// Lit la clé `chatFilters` de l'objet `data` de `GET /api/v1/settings` — même contrat que
/// `watchlist_from_settings_json` : l'objet `data`, pas la réponse entière.
pub fn chat_filters_from_account_data(data: &serde_json::Value) -> Vec<ChatFilter> {
    data.get("chatFilters")
        .map(chat_filters_from_settings_json)
        .unwrap_or_default()
}

/// L'entrée `{ key, value, updatedAt }` d'un `PATCH /api/v1/settings` pour la clé `chatFilters` —
/// jumeau de `watchlist_patch_entry`. La clé n'appartient qu'aux recherches : la valeur est
/// remplacée en entier, sans rien à préserver.
pub fn chat_filters_patch_entry(filters: &[ChatFilter]) -> serde_json::Value {
    serde_json::json!({
        "key": "chatFilters",
        "value": chat_filters_to_settings_json(filters),
        "updatedAt": chrono::Utc::now().to_rfc3339(),
    })
}

/// Deux recherches sont-elles la même ? Même mot ET même portée — `addFilter` refuse ce doublon.
pub fn same_filter(a: &ChatFilter, b: &ChatFilter) -> bool {
    a == b
}

/// Émis par `Engine::ingest_batch` quand un message de chat correspond à une recherche — voir la
/// doc de module. Porte tout ce que la carte affiche et ce que le clic utilise (l'auteur).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatAlert {
    pub channel: ChatChannel,
    pub author: String,
    pub message: String,
    /// La recherche qui a correspondu — la première de la liste, si plusieurs.
    pub filter: ChatFilter,
}

/// La première recherche qui correspond à ce message, s'il y en a une — la règle du web, voir
/// la doc de module.
pub fn matching_filter<'a>(
    filters: &'a [ChatFilter],
    channel: ChatChannel,
    author: &str,
    message: &str,
) -> Option<&'a ChatFilter> {
    let message = message.to_lowercase();
    let author = author.to_lowercase();
    filters.iter().find(|filter| {
        let applies = match filter.scope {
            ChatFilterScope::All => true,
            ChatFilterScope::Channel(scope) => scope == channel,
        };
        applies && (message.contains(&filter.text) || author.contains(&filter.text))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn commerce(text: &str) -> ChatFilter {
        ChatFilter::new(ChatFilterScope::Channel(ChatChannel::Commerce), text).unwrap()
    }

    #[test]
    fn le_format_du_web_se_lit_et_se_reecrit_a_l_identique() {
        let value = json!([
            { "text": "gelano", "channel": "commerce" },
            { "text": "donjon", "channel": "global" },
            "ancien"
        ]);
        let filters = chat_filters_from_settings_json(&value);
        assert_eq!(
            filters,
            vec![
                commerce("gelano"),
                ChatFilter::new(ChatFilterScope::All, "donjon").unwrap(),
                ChatFilter::new(ChatFilterScope::All, "ancien").unwrap(),
            ]
        );
        // L'ancien élément ressort au format courant, comme le web le réécrirait.
        assert_eq!(
            chat_filters_to_settings_json(&filters),
            json!([
                { "text": "gelano", "channel": "commerce" },
                { "text": "donjon", "channel": "global" },
                { "text": "ancien", "channel": "global" }
            ])
        );
    }

    #[test]
    fn l_entree_de_patch_porte_la_cle_et_le_format_du_web() {
        let entry = chat_filters_patch_entry(&[commerce("gelano")]);
        assert_eq!(entry["key"], "chatFilters");
        assert_eq!(
            entry["value"],
            json!([{ "text": "gelano", "channel": "commerce" }])
        );
        assert!(entry["updatedAt"].as_str().is_some_and(|s| s.contains('T')));
        let data = json!({ "chatFilters": [{ "text": "x", "channel": "guilde" }] });
        assert_eq!(chat_filters_from_account_data(&data).len(), 1);
        assert!(chat_filters_from_account_data(&json!({})).is_empty());
    }

    #[test]
    fn un_element_illisible_est_ignore_sans_faire_tomber_les_autres() {
        let value = json!([
            { "text": "ok", "channel": "guilde" },
            { "text": "   ", "channel": "guilde" },
            { "text": "x", "channel": "inconnu" },
            42
        ]);
        let filters = chat_filters_from_settings_json(&value);
        assert_eq!(filters.len(), 1);
        assert_eq!(
            filters[0].scope,
            ChatFilterScope::Channel(ChatChannel::Guilde)
        );
        assert!(chat_filters_from_settings_json(&serde_json::Value::Null).is_empty());
    }

    #[test]
    fn le_mot_est_range_trime_et_en_minuscules() {
        let filter = ChatFilter::new(ChatFilterScope::All, "  Gelano ").unwrap();
        assert_eq!(filter.text, "gelano");
        assert!(ChatFilter::new(ChatFilterScope::All, "   ").is_none());
    }

    #[test]
    fn la_correspondance_suit_la_regle_du_web() {
        let filters = vec![
            commerce("gelano"),
            ChatFilter::new(ChatFilterScope::All, "donjon").unwrap(),
        ];
        // Texte, insensible à la casse, sur le bon canal.
        assert!(matching_filter(&filters, ChatChannel::Commerce, "Bob", "vends GELANO").is_some());
        // Mauvais canal pour une recherche par canal.
        assert!(matching_filter(&filters, ChatChannel::Guilde, "Bob", "vends gelano").is_none());
        // Une recherche globale s'applique partout.
        assert!(
            matching_filter(&filters, ChatChannel::Guilde, "Bob", "on fait le Donjon ?").is_some()
        );
        // L'auteur compte aussi.
        let filters = vec![ChatFilter::new(ChatFilterScope::All, "kralamoure").unwrap()];
        assert!(
            matching_filter(&filters, ChatChannel::Proximite, "Kralamoure-Fan", "coucou").is_some()
        );
        // Pas de mot entier : « pvm » trouve « pvm-actif ».
        let filters = vec![ChatFilter::new(ChatFilterScope::All, "pvm").unwrap()];
        assert!(
            matching_filter(&filters, ChatChannel::Recrutement, "X", "guilde pvm-active").is_some()
        );
    }

    #[test]
    fn la_premiere_recherche_qui_correspond_est_renvoyee() {
        let filters = vec![
            ChatFilter::new(ChatFilterScope::All, "vends").unwrap(),
            commerce("gelano"),
        ];
        let found =
            matching_filter(&filters, ChatChannel::Commerce, "Bob", "vends gelano").unwrap();
        assert_eq!(found.text, "vends");
    }
}
