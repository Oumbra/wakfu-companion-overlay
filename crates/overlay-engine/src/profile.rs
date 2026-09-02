//! Alerte sonore au ramassage — §9 du plan, « Alertes de drop », cas `reason: 'loot'` de
//! `LootAlertEvent` (`loot-alert.service.ts`), jusqu'ici documenté comme hors périmètre (voir
//! `watchlist.rs::WatchlistAlert`, qui ne couvre que `reason: 'countdown'`). Miroir de
//! `ProfileService`/`StatsStoreService.registerLoot` (`profile.service.ts`/`stats-store.service.ts`) :
//! **indépendant de la watchlist** — n'importe quel objet ramassé peut avoir son son activé ici,
//! qu'il soit suivi ou non, et inversement un objet suivi n'a pas forcément son son activé.
//!
//! Liste ENTIÈREMENT lue depuis le compte (`GET /api/v1/settings`, clé `"profile"` →
//! `soundItems`) et jamais modifiée par l'overlay — contrairement aux compteurs de `watchlist`,
//! il n'existe même pas de compteur local ici : pas de UI d'édition côté overlay pour cette
//! itération, seulement sur le web (page profil).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Miroir réduit de `SoundItemEntry` (`profile.service.ts`) — `isDefault` ne sert qu'à l'UI
/// d'édition web (bouton de suppression absent pour les objets par défaut), sans objet ici.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundItemEntry {
    pub name: String,
    pub enabled: bool,
    #[serde(rename = "catalogId", default)]
    pub catalog_id: Option<i64>,
}

/// Émis par `Engine::ingest_batch` quand un objet ramassé (`LogEntry::Loot`) correspond à une
/// `SoundItemEntry` activée — distinct de `WatchlistAlert` (files de drain séparées sur `Engine`,
/// voir `drain_loot_alerts`/`drain_watchlist_alerts`) : les deux mécanismes sont indépendants côté
/// web (un objet peut être suivi sans avoir son son activé, et réciproquement), pas un type unique
/// artificiellement partagé.
#[derive(Debug, Clone, PartialEq)]
pub struct LootAlert {
    pub name: String,
    pub quantity: i64,
    pub catalog_id: Option<i64>,
}

/// Construit la liste des objets à son activé depuis `data["profile"]["soundItems"]` de la
/// réponse `GET /api/v1/settings` (miroir de `StoredProfile.soundItems`, `profile.service.ts`) —
/// vide (pas une erreur) si la clé `"profile"` ou `"soundItems"` est absente/mal formée, comme
/// `watchlist_from_settings_json` : un compte sans profil synchronisé n'a simplement aucune
/// alerte de ramassage, jamais un blocage.
pub fn sound_items_from_settings_json(data: &Value) -> Vec<SoundItemEntry> {
    data.get("profile")
        .and_then(|profile| profile.get("soundItems"))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

/// Miroir exact de `ProfileService.findEnabledSoundItem` : nom insensible à la casse et aux
/// espaces superflus, seule une entrée `enabled` déclenche une alerte.
pub fn find_enabled_sound_item<'a>(
    items: &'a [SoundItemEntry],
    item_name: &str,
) -> Option<&'a SoundItemEntry> {
    let normalized = item_name.trim().to_lowercase();
    items
        .iter()
        .find(|entry| entry.enabled && entry.name.trim().to_lowercase() == normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, enabled: bool) -> SoundItemEntry {
        SoundItemEntry {
            name: name.to_string(),
            enabled,
            catalog_id: None,
        }
    }

    #[test]
    fn trouve_une_entree_activee_insensible_a_la_casse_et_aux_espaces() {
        let items = vec![entry("Pierre d'aventure", true)];
        assert_eq!(
            find_enabled_sound_item(&items, "  PIERRE D'AVENTURE  "),
            Some(&items[0])
        );
    }

    #[test]
    fn une_entree_desactivee_ne_declenche_jamais_rien() {
        let items = vec![entry("Pierre d'aventure", false)];
        assert_eq!(find_enabled_sound_item(&items, "Pierre d'aventure"), None);
    }

    #[test]
    fn aucune_correspondance_renvoie_none_sans_paniquer() {
        let items = vec![entry("Pierre d'aventure", true)];
        assert_eq!(find_enabled_sound_item(&items, "Larve Bleue"), None);
    }

    #[test]
    fn liste_vide_ou_absente_ne_plante_pas() {
        assert!(sound_items_from_settings_json(&serde_json::json!({})).is_empty());
        assert!(sound_items_from_settings_json(&serde_json::json!({ "profile": {} })).is_empty());
    }

    #[test]
    fn parse_les_entrees_reelles_du_profil() {
        let items = sound_items_from_settings_json(&serde_json::json!({
            "profile": {
                "soundItems": [
                    { "name": "Pierre d'aventure", "enabled": true, "isDefault": true, "catalogId": 123 },
                    { "name": "Influence III", "enabled": false, "isDefault": true, "catalogId": null },
                ],
            },
        }));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "Pierre d'aventure");
        assert_eq!(items[0].catalog_id, Some(123));
        assert!(!items[1].enabled);
    }
}
