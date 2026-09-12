//! Alerte sonore au ramassage — §9 du plan, « Alertes de drop », cas `reason: 'loot'` de
//! `LootAlertEvent` (`loot-alert.service.ts`), distinct de `watchlist.rs::WatchlistAlert` qui ne
//! couvre que `reason: 'countdown'`. Miroir de `ProfileService`/`StatsStoreService.registerLoot`
//! (`profile.service.ts`/`stats-store.service.ts`) : **indépendant de la watchlist** — n'importe
//! quel objet ramassé peut avoir son son activé ici, qu'il soit suivi ou non, et inversement un
//! objet suivi n'a pas forcément son son activé.
//!
//! ## L'overlay ÉCRIT ici depuis le 2026-09-12
//!
//! Jusque-là ce module était en lecture seule (« pas de UI d'édition côté overlay pour cette
//! itération, seulement sur le web ») : l'onglet « Alertes » de la fenêtre Options n'était qu'une
//! entrée de menu désactivée. Il a maintenant son contenu, donc ce module porte aussi les
//! mutations — ajout, retrait, bascule, durée du toast, fermeture manuelle — et la reconstruction
//! de l'objet à renvoyer au compte.
//!
//! **Le piège de cette écriture, et la raison d'être de [`AlertProfile::patch_value`]** : côté
//! serveur, `PATCH /api/v1/settings` remplace la valeur ENTIÈRE d'une clé (voir
//! `functions/api/v1/settings.ts::onRequestPatch`), et la clé `profile` ne contient pas que les
//! alertes — elle porte aussi le pseudo, l'avatar et le mode d'affichage des personnages, que
//! l'overlay ne connaît pas et n'affiche nulle part. Renvoyer un objet reconstruit de mémoire les
//! effacerait du compte. La réécriture repart donc TOUJOURS de l'objet brut reçu au `GET`, dont
//! elle ne remplace que les trois champs qu'elle possède.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Les dix objets à son activé d'un profil neuf — copie exacte de `DEFAULT_SOUND_ITEM_NAMES`
/// (`profile.service.ts`), ordre compris.
///
/// Ils sont **protégés du retrait** : le web ne leur donne pas de bouton de suppression, et
/// [`AlertProfile::remove`] applique la même règle. Leur son, lui, se coupe comme celui de
/// n'importe quel autre objet — c'est un état, pas une suppression.
pub const DEFAULT_SOUND_ITEM_NAMES: [&str; 10] = [
    "Pierre d'aventure",
    "Pierre d'équilibre",
    "Pierre d'entourage",
    "Pierre de vitesse",
    "Pierre ultime",
    "Influence III",
    "Plan \"Epée de Bonta\"",
    "Plan \"Epée de Brâkmar\"",
    "Plan \"Epée de Sufokia\"",
    "Plan \"Epée d'Amakna\"",
];

/// Durée d'affichage du toast d'alerte, en secondes — `DEFAULT_ALERT_DURATION_SECONDS` du web.
pub const DEFAULT_ALERT_DURATION_SECONDS: f32 = 3.5;

/// Plancher de la durée réglable — `MIN_ALERT_DURATION_SECONDS` du web.
pub const MIN_ALERT_DURATION_SECONDS: f32 = 0.5;

/// Plafond de la durée réglable.
///
/// **Il n'existe pas côté web**, qui ne borne que le bas (`Math.max`). Ajouté ici parce que le
/// toast de l'overlay est posé PAR-DESSUS LE JEU : une valeur saisie à 600 y laisserait une carte
/// au milieu de l'écran dix minutes durant, sans autre recours que de rouvrir la fenêtre Options.
/// Une page web n'a pas ce problème, elle se ferme.
pub const MAX_ALERT_DURATION_SECONDS: f32 = 30.0;

/// Un objet dont le ramassage déclenche une alerte — miroir de `SoundItemEntry`
/// (`profile.service.ts`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundItemEntry {
    pub name: String,
    pub enabled: bool,
    /// **Un des dix objets de [`DEFAULT_SOUND_ITEM_NAMES`]**, que l'utilisateur ne peut pas
    /// retirer de sa liste.
    ///
    /// Absent du JSON d'un profil ancien : `false` par défaut, comme côté web où le champ est
    /// posé par la fusion, pas par le stockage.
    #[serde(rename = "isDefault", default)]
    pub is_default: bool,
    #[serde(rename = "catalogId", default)]
    pub catalog_id: Option<i64>,
}

/// Émis par `Engine::ingest_batch` quand un objet ramassé (`LogEntry::Loot`) correspond à une
/// [`SoundItemEntry`] activée — distinct de `WatchlistAlert` (files de drain séparées sur
/// `Engine`, voir `drain_loot_alerts`/`drain_watchlist_alerts`) : les deux mécanismes sont
/// indépendants côté web (un objet peut être suivi sans avoir son son activé, et réciproquement),
/// pas un type unique artificiellement partagé.
#[derive(Debug, Clone, PartialEq)]
pub struct LootAlert {
    pub name: String,
    pub quantity: i64,
    pub catalog_id: Option<i64>,
}

/// Tout ce que la clé `profile` du compte porte d'alertes — la liste des objets **et** les deux
/// réglages du toast.
///
/// Les trois vont ensemble parce qu'ils voyagent ensemble : ils sont réglés sur le même écran et
/// écrits par le même `PATCH`. Les séparer en trois types ferait porter à l'appelant la charge de
/// les réunir au moment d'écrire, et c'est exactement là qu'on oublie un champ.
#[derive(Debug, Clone, PartialEq)]
pub struct AlertProfile {
    /// Objets à alerte, **défauts fusionnés** — voir [`AlertProfile::from_settings_json`].
    pub sound_items: Vec<SoundItemEntry>,
    /// Durée d'affichage du toast, en secondes. Ignorée quand [`Self::manual_close`] est vrai.
    pub duration_seconds: f32,
    /// Le toast ne se ferme qu'à la main.
    pub manual_close: bool,
}

impl Default for AlertProfile {
    /// Le profil d'un compte qui n'a jamais rien enregistré : les dix défauts, son activé.
    fn default() -> Self {
        Self {
            sound_items: default_sound_items(),
            duration_seconds: DEFAULT_ALERT_DURATION_SECONDS,
            manual_close: false,
        }
    }
}

/// Les dix défauts, tels qu'un profil neuf les porte.
fn default_sound_items() -> Vec<SoundItemEntry> {
    DEFAULT_SOUND_ITEM_NAMES
        .iter()
        .map(|name| SoundItemEntry {
            name: (*name).to_string(),
            enabled: true,
            is_default: true,
            catalog_id: None,
        })
        .collect()
}

/// Deux entrées désignent-elles le même objet ?
///
/// Miroir de la règle du web (`addSoundItem`/`removeSoundItem`) : **l'id du catalogue tranche
/// quand les deux en ont un** — deux objets homonymes de raretés différentes sont deux entrées
/// distinctes — et le nom normalisé sert de repli sinon, parce que les défauts sont stockés sans
/// id tant que le catalogue ne les a pas résolus.
fn meme_objet(entry: &SoundItemEntry, name: &str, catalog_id: Option<i64>) -> bool {
    match (entry.catalog_id, catalog_id) {
        (Some(a), Some(b)) => a == b,
        _ => entry.name.trim().to_lowercase() == name.trim().to_lowercase(),
    }
}

impl AlertProfile {
    /// Lit le profil d'alerte depuis l'objet `data` d'une réponse `GET /api/v1/settings`.
    ///
    /// **Les défauts sont fusionnés**, comme `mergeWithDefaultSoundItems` le fait côté web : un
    /// profil déjà enregistré garde son ordre et ses états, et tout défaut absent de sa liste est
    /// ajouté à la suite. Sans cette fusion, un objet ajouté à [`DEFAULT_SOUND_ITEM_NAMES`] ne
    /// serait jamais vu par un compte existant.
    ///
    /// Une clé absente ou mal formée donne le profil par défaut, jamais une erreur : un compte
    /// sans profil synchronisé a simplement les dix alertes d'origine, comme un profil web neuf.
    pub fn from_settings_json(data: &Value) -> Self {
        let profile = data.get("profile");
        let stored: Option<Vec<SoundItemEntry>> = profile
            .and_then(|profile| profile.get("soundItems"))
            .and_then(|value| serde_json::from_value(value.clone()).ok());

        let duration = profile
            .and_then(|profile| profile.get("alertDurationSeconds"))
            .and_then(Value::as_f64)
            .map(|seconds| seconds as f32);
        let manual = profile
            .and_then(|profile| profile.get("alertManualClose"))
            .and_then(Value::as_bool);

        // **Migration du zéro**, reprise telle quelle du web : une version antérieure stockait
        // `alertDurationSeconds: 0` pour dire « fermeture manuelle ». Le lire comme une durée
        // donnerait un toast qui disparaît à l'image suivante.
        let (duration_seconds, manual_close) = match duration {
            Some(0.0) => (DEFAULT_ALERT_DURATION_SECONDS, true),
            Some(seconds) => (clamp_duration(seconds), manual.unwrap_or(false)),
            None => (DEFAULT_ALERT_DURATION_SECONDS, manual.unwrap_or(false)),
        };

        Self {
            sound_items: merge_with_defaults(stored),
            duration_seconds,
            manual_close,
        }
    }

    /// Ajoute un objet, son activé. Renvoie `false` si la liste le contenait déjà.
    ///
    /// Un nom vide (ou blanc) est refusé — c'est ce que fait le champ d'ajout du web quand on
    /// valide à vide, et le composant d'autocomplétion de l'overlay peut renvoyer la même chose.
    pub fn add(&mut self, raw_name: &str, catalog_id: Option<i64>) -> bool {
        let name = raw_name.trim();
        if name.is_empty() {
            return false;
        }
        if self
            .sound_items
            .iter()
            .any(|entry| meme_objet(entry, name, catalog_id))
        {
            return false;
        }
        self.sound_items.push(SoundItemEntry {
            name: name.to_string(),
            enabled: true,
            is_default: false,
            catalog_id,
        });
        true
    }

    /// Retire un objet. Renvoie `false` si rien n'a été retiré — objet absent, **ou objet par
    /// défaut**, que le web refuse structurellement de supprimer.
    pub fn remove(&mut self, name: &str, catalog_id: Option<i64>) -> bool {
        let avant = self.sound_items.len();
        self.sound_items
            .retain(|entry| entry.is_default || !meme_objet(entry, name, catalog_id));
        self.sound_items.len() != avant
    }

    /// Bascule le son d'un objet. Renvoie `false` si l'objet est absent.
    ///
    /// **Vaut aussi pour les défauts** : couper le son d'un objet n'est pas le retirer de la
    /// liste, et c'est exactement la distinction que ces dix entrées matérialisent.
    pub fn toggle(&mut self, name: &str, catalog_id: Option<i64>) -> bool {
        match self
            .sound_items
            .iter_mut()
            .find(|entry| meme_objet(entry, name, catalog_id))
        {
            Some(entry) => {
                entry.enabled = !entry.enabled;
                true
            }
            None => false,
        }
    }

    /// Règle la durée du toast, bornée à [`MIN_ALERT_DURATION_SECONDS`] et
    /// [`MAX_ALERT_DURATION_SECONDS`].
    pub fn set_duration(&mut self, seconds: f32) {
        self.duration_seconds = clamp_duration(seconds);
    }

    /// L'entrée activée dont le nom correspond, le cas échéant — miroir exact de
    /// `ProfileService.findEnabledSoundItem` : nom insensible à la casse et aux espaces
    /// superflus, seule une entrée `enabled` déclenche une alerte.
    pub fn find_enabled(&self, item_name: &str) -> Option<&SoundItemEntry> {
        find_enabled_sound_item(&self.sound_items, item_name)
    }

    /// L'objet `profile` à renvoyer dans un `PATCH /api/v1/settings`, construit **sur celui reçu
    /// au `GET`**.
    ///
    /// Voir la doc de module : le serveur remplace la valeur entière de la clé, et cette clé porte
    /// aussi le pseudo, l'avatar et le mode d'affichage des personnages — que l'overlay ne
    /// connaît pas. Ils sont donc recopiés tels quels depuis `stored`, dont seuls les trois champs
    /// d'alerte sont remplacés. Un `stored` absent ou d'un autre type JSON donne un objet neuf
    /// avec les seuls champs d'alerte, ce qui est le bon comportement pour un compte qui n'avait
    /// pas encore de profil.
    pub fn patch_value(&self, stored: Option<&Value>) -> Value {
        let mut profile = match stored {
            Some(Value::Object(map)) => Value::Object(map.clone()),
            _ => Value::Object(serde_json::Map::new()),
        };
        let map = profile
            .as_object_mut()
            .expect("objet construit juste avant");
        map.insert(
            "soundItems".to_string(),
            serde_json::to_value(&self.sound_items).unwrap_or(Value::Null),
        );
        map.insert(
            "alertDurationSeconds".to_string(),
            serde_json::json!(self.duration_seconds),
        );
        map.insert(
            "alertManualClose".to_string(),
            Value::Bool(self.manual_close),
        );
        profile
    }
}

/// Construit l'entrée `PATCH /api/v1/settings` (`{ key, value, updatedAt }`) pour la clé
/// `"profile"` — symétrique de [`AlertProfile::from_settings_json`], et jumeau de
/// `watchlist::watchlist_patch_entry`.
///
/// `profile` est l'objet ENTIER produit par [`AlertProfile::patch_value`], jamais les seuls champs
/// d'alerte : le serveur remplace la valeur de la clé, il ne fusionne pas. `updatedAt` arbitre le
/// « dernier écrivain gagne », horodaté à l'instant de l'appel — miroir de
/// `RemoteUserDataRepository.sendPending()` côté web.
pub fn profile_patch_entry(profile: &Value) -> Value {
    serde_json::json!({
        "key": "profile",
        "value": profile,
        "updatedAt": chrono::Utc::now().to_rfc3339(),
    })
}

/// Borne une durée saisie, plancher et plafond compris — voir [`MAX_ALERT_DURATION_SECONDS`] pour
/// le plafond, qui n'existe pas côté web.
fn clamp_duration(seconds: f32) -> f32 {
    seconds.clamp(MIN_ALERT_DURATION_SECONDS, MAX_ALERT_DURATION_SECONDS)
}

/// Fusionne les objets par défaut avec ceux déjà enregistrés — miroir de
/// `ProfileService.mergeWithDefaultSoundItems`.
///
/// Une entrée enregistrée sous un nom de défaut **redevient un défaut** : l'ancien schéma (celui
/// que l'overlay écrivait avant ce module, et les profils web d'avant le champ) n'a pas
/// `isDefault`, et sans ce recalage les dix objets protégés arriveraient avec une croix de
/// retrait. La comparaison se fait sur le nom normalisé, comme côté web.
fn merge_with_defaults(stored: Option<Vec<SoundItemEntry>>) -> Vec<SoundItemEntry> {
    let Some(mut items) = stored else {
        return default_sound_items();
    };
    let defauts: Vec<String> = DEFAULT_SOUND_ITEM_NAMES
        .iter()
        .map(|name| name.to_lowercase())
        .collect();
    for entry in &mut items {
        entry.is_default = defauts.contains(&entry.name.trim().to_lowercase());
    }
    let presents: Vec<String> = items
        .iter()
        .map(|entry| entry.name.trim().to_lowercase())
        .collect();
    for manquant in default_sound_items() {
        if !presents.contains(&manquant.name.to_lowercase()) {
            items.push(manquant);
        }
    }
    items
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
            is_default: false,
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
    fn un_compte_sans_profil_recoit_les_dix_defauts() {
        // Et pas une liste vide : c'est ce que voit un profil web neuf, dont l'overlay est le
        // miroir. Une liste vide donnerait un écran d'alertes désert sur un compte tout neuf.
        let profil = AlertProfile::from_settings_json(&serde_json::json!({}));
        assert_eq!(profil.sound_items.len(), 10);
        assert!(profil.sound_items.iter().all(|e| e.is_default && e.enabled));
        assert_eq!(profil.duration_seconds, DEFAULT_ALERT_DURATION_SECONDS);
        assert!(!profil.manual_close);
    }

    #[test]
    fn un_profil_existant_garde_son_ordre_et_recoit_les_defauts_manquants() {
        let profil = AlertProfile::from_settings_json(&serde_json::json!({
            "profile": {
                "soundItems": [
                    { "name": "Larme d'Ogrest", "enabled": true, "catalogId": 42 },
                    { "name": "Influence III", "enabled": false },
                ],
            },
        }));
        assert_eq!(profil.sound_items[0].name, "Larme d'Ogrest");
        assert_eq!(profil.sound_items[1].name, "Influence III");
        // Les neuf défauts absents arrivent à la suite, jamais devant.
        assert_eq!(profil.sound_items.len(), 11);
        assert!(!profil.sound_items[0].is_default);
    }

    #[test]
    fn un_defaut_stocke_sans_le_champ_redevient_un_defaut() {
        // **Le cas qui donnerait une croix de retrait sur un objet protégé** : les profils écrits
        // avant `isDefault` n'ont pas le champ, et serde le lirait `false`.
        let profil = AlertProfile::from_settings_json(&serde_json::json!({
            "profile": { "soundItems": [{ "name": "PIERRE ULTIME", "enabled": true }] },
        }));
        assert!(profil.sound_items[0].is_default, "défaut non reconnu");
    }

    #[test]
    fn la_duree_zero_de_l_ancien_schema_devient_une_fermeture_manuelle() {
        let profil = AlertProfile::from_settings_json(&serde_json::json!({
            "profile": { "alertDurationSeconds": 0 },
        }));
        assert!(profil.manual_close);
        assert_eq!(profil.duration_seconds, DEFAULT_ALERT_DURATION_SECONDS);
    }

    #[test]
    fn la_duree_est_bornee_des_la_lecture() {
        let trop = AlertProfile::from_settings_json(&serde_json::json!({
            "profile": { "alertDurationSeconds": 600 },
        }));
        assert_eq!(trop.duration_seconds, MAX_ALERT_DURATION_SECONDS);
        let pas_assez = AlertProfile::from_settings_json(&serde_json::json!({
            "profile": { "alertDurationSeconds": 0.1 },
        }));
        assert_eq!(pas_assez.duration_seconds, MIN_ALERT_DURATION_SECONDS);
    }

    #[test]
    fn ajouter_deux_fois_le_meme_objet_ne_fait_rien_la_seconde() {
        let mut profil = AlertProfile::default();
        assert!(profil.add("Larme d'Ogrest", Some(7)));
        assert!(!profil.add("larme d'ogrest", Some(7)));
        assert_eq!(profil.sound_items.len(), 11);
    }

    #[test]
    fn deux_homonymes_d_id_differents_coexistent() {
        // La règle du web : l'id tranche quand les deux en ont un. Les deux « Larme d'Ogrest » du
        // jeu sont deux objets, pas un doublon.
        let mut profil = AlertProfile::default();
        assert!(profil.add("Larme d'Ogrest", Some(7)));
        assert!(profil.add("Larme d'Ogrest", Some(8)));
        assert_eq!(profil.sound_items.len(), 12);
    }

    #[test]
    fn un_objet_par_defaut_ne_se_retire_pas_mais_se_coupe() {
        let mut profil = AlertProfile::default();
        assert!(!profil.remove("Pierre ultime", None), "défaut retiré");
        assert_eq!(profil.sound_items.len(), 10);
        assert!(profil.toggle("Pierre ultime", None));
        assert!(!profil.find_enabled("Pierre ultime").is_some());
    }

    #[test]
    fn un_objet_ajoute_se_retire() {
        let mut profil = AlertProfile::default();
        profil.add("Larme d'Ogrest", Some(7));
        assert!(profil.remove("Larme d'Ogrest", Some(7)));
        assert_eq!(profil.sound_items.len(), 10);
    }

    #[test]
    fn la_reecriture_preserve_les_champs_que_l_overlay_ne_connait_pas() {
        // **Le test qui garde le pseudo et l'avatar de l'utilisateur.** Le serveur remplace la
        // valeur entière de la clé : un objet reconstruit de mémoire les effacerait du compte.
        let recu = serde_json::json!({
            "pseudo": "Oumbra",
            "avatarIndex": 12,
            "avatarSchemaVersion": 2,
            "characterViewMode": "grid",
            "soundItems": [],
            "alertDurationSeconds": 1.0,
            "alertManualClose": false,
        });
        let mut profil = AlertProfile::from_settings_json(&serde_json::json!({ "profile": recu }));
        profil.manual_close = true;
        let patch = profil.patch_value(Some(&recu));
        assert_eq!(patch["pseudo"], "Oumbra");
        assert_eq!(patch["avatarIndex"], 12);
        assert_eq!(patch["avatarSchemaVersion"], 2);
        assert_eq!(patch["characterViewMode"], "grid");
        assert_eq!(patch["alertManualClose"], true);
        assert_eq!(patch["soundItems"].as_array().map(Vec::len), Some(10));
    }

    #[test]
    fn la_reecriture_d_un_compte_sans_profil_ne_panique_pas() {
        let profil = AlertProfile::default();
        for stored in [None, Some(&Value::Null), Some(&serde_json::json!("brisé"))] {
            let patch = profil.patch_value(stored);
            assert!(patch["soundItems"].is_array());
        }
    }

    #[test]
    fn le_champ_is_default_part_bien_dans_le_json_ecrit() {
        // Le web s'en sert pour masquer le bouton de suppression : l'omettre rendrait les dix
        // objets protégés supprimables depuis le site après un enregistrement par l'overlay.
        let patch = AlertProfile::default().patch_value(None);
        assert_eq!(patch["soundItems"][0]["isDefault"], true);
        assert_eq!(patch["soundItems"][0]["enabled"], true);
    }
}
