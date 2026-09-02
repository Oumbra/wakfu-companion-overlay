//! Roster de personnages déclarés par l'utilisateur (page profil du compte web) — miroir de
//! lecture seule de `character-roster.service.ts` : source de vérité PRIORITAIRE pour la
//! classe/sexe d'un allié confirmé par nom exact, avant repli sur le `breed` du combat (voir
//! `class_breed.rs` et `session.rs`). L'édition du roster reste une fonctionnalité web — ce module
//! ne fait que consommer le JSON déjà renvoyé par `GET /api/v1/settings` (clé `"roster"`).
//!
//! Pas d'IO ici (aucune dépendance réseau) : `from_settings_json` prend un `serde_json::Value`
//! déjà récupéré par `overlay-sync`, exactement comme le reste d'`overlay-engine` ne connaît que
//! des données déjà lues.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

/// Miroir de `Gender` (`class-icons.data.ts`) — féminin/masculin, les deux seules valeurs que le
/// jeu propose à la création de personnage. `Serialize` sert à la persistance disque des combats
/// en cours (voir `fight_store.rs`) — `FighterDamage::gender` doit survivre à un redémarrage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    F,
    M,
}

/// Miroir de `RosterCharacter` (`character-roster.service.ts`) — seuls les champs utiles au
/// panneau Combat sont repris (pas `id`/`label` du compte, qui restent une préoccupation web).
/// `gameServer` (voir `RosterAccount`) reste, lui, porté au niveau du COMPTE, pas du personnage —
/// voir `RosterIndex::find_game_server`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RosterCharacter {
    pub name: String,
    #[serde(rename = "className")]
    pub class_name: String,
    pub gender: Gender,
}

/// Miroir de `RosterAccount` (`character-roster.service.ts`) — `game_server` (le code
/// `game_servers.code`, jamais une valeur inventée) est LE SEUL moyen de résoudre `gameServer`
/// (L5, §7.1 du plan) : le log Wakfu ne contient aucune indication de serveur (voir
/// `GameServerService`, dépôt web), il faut donc le rattacher au compte qui l'a déclaré.
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RosterAccount {
    #[serde(default)]
    characters: Vec<RosterCharacter>,
    #[serde(rename = "gameServer", default)]
    game_server: Option<String>,
}

/// Port direct de `normalizeWakfuName` (`wakfu-name.util.ts`) : minuscule, espaces superflus
/// retirés, apostrophes typographiques uniformisées, accents retirés (NFD puis filtrage des
/// marques diacritiques U+0300-U+036F) — mêmes règles, même plage Unicode.
pub fn normalize_wakfu_name(name: &str) -> String {
    let lower = name.trim().to_lowercase().replace(['’', '‘'], "'");
    lower
        .nfd()
        .filter(|c| !('\u{0300}'..='\u{036f}').contains(c))
        .collect()
}

/// Index en lookup O(1) par nom normalisé, construit une seule fois à la réception du roster.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RosterIndex {
    by_normalized_name: HashMap<String, RosterCharacter>,
    /// `gameServer` du COMPTE auquel appartient ce personnage — voir `find_game_server`. `None`
    /// (compte sans serveur déclaré) distinct d'une absence d'entrée (personnage inconnu) : les
    /// deux renvoient `None` côté `find_game_server`, mais seule cette table sait laquelle.
    game_server_by_normalized_name: HashMap<String, Option<String>>,
}

impl RosterIndex {
    /// Construit l'index depuis `data["roster"]` de la réponse `GET /api/v1/settings` — vide (pas
    /// une erreur) si la clé est absente, ce qui est le cas normal en mode invité ou pour un
    /// compte qui n'a encore déclaré aucun personnage.
    pub fn from_settings_json(data: &serde_json::Value) -> Self {
        let accounts: Vec<RosterAccount> = data
            .get("roster")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let mut by_normalized_name = HashMap::new();
        let mut game_server_by_normalized_name = HashMap::new();
        for account in accounts {
            for character in account.characters {
                // Dernier écrivain gagne en cas d'homonyme entre deux comptes — comportement
                // best-effort assumé, un vrai conflit de nom entre comptes est un cas limite que
                // le web lui-même ne résout pas autrement (premier compte trouvé dans son
                // `findCharacter`, ici l'ordre d'itération du JSON n'est pas garanti identique de
                // toute façon). Même choix pour le gameServer du compte : les deux tables
                // partagent le même dernier écrivain, jamais désynchronisées l'une de l'autre.
                let key = normalize_wakfu_name(&character.name);
                game_server_by_normalized_name.insert(key.clone(), account.game_server.clone());
                by_normalized_name.insert(key, character);
            }
        }
        Self {
            by_normalized_name,
            game_server_by_normalized_name,
        }
    }

    /// Recherche insensible à la casse/accents/apostrophes — miroir de
    /// `CharacterRosterService.findCharacter`.
    pub fn find(&self, name: &str) -> Option<&RosterCharacter> {
        self.by_normalized_name.get(&normalize_wakfu_name(name))
    }

    /// Code du serveur de jeu (`game_servers.code`) du compte auquel appartient ce personnage —
    /// miroir de `GameServerService.activeServer` restreint à la résolution par nom (voir
    /// `overlay_engine::session::Engine::current_game_server` pour la déduction "dernier
    /// personnage du roster reconnu dans le log", qui appelle cette méthode). `None` aussi bien
    /// pour un personnage inconnu que pour un compte qui n'a pas déclaré de serveur — jamais une
    /// valeur inventée (voir `RosterAccount::game_server`).
    pub fn find_game_server(&self, name: &str) -> Option<String> {
        self.game_server_by_normalized_name
            .get(&normalize_wakfu_name(name))
            .cloned()
            .flatten()
    }

    pub fn is_empty(&self) -> bool {
        self.by_normalized_name.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_settings() -> serde_json::Value {
        serde_json::json!({
            "roster": [
                {
                    "id": "acc-1",
                    "label": "Principal",
                    "isDefault": true,
                    "characters": [
                        { "name": "Éclair-Ïo", "className": "iop", "gender": "m" },
                        { "name": "Brise'Os", "className": "sram", "gender": "f" },
                    ],
                    "gameServer": "pandora",
                },
                {
                    "id": "acc-2",
                    "label": "Secondaire",
                    "characters": [
                        { "name": "SansServeur", "className": "sacrieur", "gender": "m" },
                    ],
                },
            ],
        })
    }

    #[test]
    fn trouve_un_personnage_par_nom_exact() {
        let roster = RosterIndex::from_settings_json(&sample_settings());
        let found = roster.find("Éclair-Ïo").expect("personnage attendu");
        assert_eq!(found.class_name, "iop");
        assert_eq!(found.gender, Gender::M);
    }

    #[test]
    fn tolere_casse_accents_et_apostrophe_typographique() {
        let roster = RosterIndex::from_settings_json(&sample_settings());
        assert!(roster.find("eclair-io").is_some());
        assert!(roster.find("ECLAIR-IO").is_some());
        // apostrophe typographique (’) vs droite (') — le nom stocké utilise l'apostrophe droite.
        assert!(roster.find("Brise’Os").is_some());
    }

    #[test]
    fn resout_le_serveur_du_compte_dun_personnage() {
        let roster = RosterIndex::from_settings_json(&sample_settings());
        assert_eq!(
            roster.find_game_server("Éclair-Ïo"),
            Some("pandora".to_string())
        );
        // insensible casse/accents, même règle que `find`.
        assert_eq!(
            roster.find_game_server("eclair-io"),
            Some("pandora".to_string())
        );
    }

    #[test]
    fn compte_sans_serveur_declare_renvoie_none() {
        let roster = RosterIndex::from_settings_json(&sample_settings());
        assert_eq!(roster.find_game_server("SansServeur"), None);
    }

    #[test]
    fn personnage_inconnu_renvoie_none_pour_le_serveur() {
        let roster = RosterIndex::from_settings_json(&sample_settings());
        assert_eq!(roster.find_game_server("Quidam"), None);
    }

    #[test]
    fn absent_du_roster_renvoie_none() {
        let roster = RosterIndex::from_settings_json(&sample_settings());
        assert!(roster.find("Quidam").is_none());
    }

    #[test]
    fn roster_absent_ou_vide_ne_plante_pas() {
        let empty = RosterIndex::from_settings_json(&serde_json::json!({}));
        assert!(empty.is_empty());
        assert!(empty.find("quiconque").is_none());
    }
}
