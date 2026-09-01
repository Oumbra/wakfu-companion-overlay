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

use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

/// Miroir de `Gender` (`class-icons.data.ts`) — féminin/masculin, les deux seules valeurs que le
/// jeu propose à la création de personnage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    F,
    M,
}

/// Miroir de `RosterCharacter` (`character-roster.service.ts`) — seuls les champs utiles au
/// panneau Combat sont repris (pas `id`/`label`/`gameServer` du compte, qui restent une
/// préoccupation web).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RosterCharacter {
    pub name: String,
    #[serde(rename = "className")]
    pub class_name: String,
    pub gender: Gender,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RosterAccount {
    #[serde(default)]
    characters: Vec<RosterCharacter>,
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
        for account in accounts {
            for character in account.characters {
                // Dernier écrivain gagne en cas d'homonyme entre deux comptes — comportement
                // best-effort assumé, un vrai conflit de nom entre comptes est un cas limite que
                // le web lui-même ne résout pas autrement (premier compte trouvé dans son
                // `findCharacter`, ici l'ordre d'itération du JSON n'est pas garanti identique de
                // toute façon).
                by_normalized_name.insert(normalize_wakfu_name(&character.name), character);
            }
        }
        Self { by_normalized_name }
    }

    /// Recherche insensible à la casse/accents/apostrophes — miroir de
    /// `CharacterRosterService.findCharacter`.
    pub fn find(&self, name: &str) -> Option<&RosterCharacter> {
        self.by_normalized_name.get(&normalize_wakfu_name(name))
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
