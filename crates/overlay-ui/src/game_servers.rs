//! Serveurs de jeu — miroir de `GameServerService` (dépôt web) réduit à ce dont l'overlay a besoin :
//! proposer un serveur au compte dans l'onglet « Personnages », et afficher celui déjà déclaré.
//!
//! **Jamais une liste en dur** (règle du dépôt web, `functions/api/v1/game-servers.ts`) : le code
//! écrit dans `roster[].gameServer` doit être un `code` de la table `game_servers`, sans quoi le
//! site ne le reconnaît pas. La liste descend donc du réseau (`overlay_sync::client::
//! fetch_game_servers`), avec cache disque pour qu'un lancement hors ligne ne perde pas le nom des
//! serveurs (voir `background::spawn_game_servers_thread`).

use serde_json::Value;

/// Une entrée de `game_servers` — même forme que l'interface `GameServer` du web.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameServer {
    pub code: String,
    pub label: String,
    /// Un serveur fermé reste dans la table : il ne se PROPOSE plus, mais un compte qui le porte
    /// encore doit continuer à l'afficher sous son nom — voir [`GameServers::label`].
    pub is_active: bool,
}

/// La liste telle qu'elle est servie, dans l'ordre de la table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameServers {
    entries: Vec<GameServer>,
}

impl GameServers {
    /// Lit la réponse de `GET /api/v1/game-servers` — un tableau d'objets `{code, label,
    /// isActive}`. Une entrée sans `code` est ignorée : elle ne pourrait être ni proposée ni
    /// retrouvée. Jamais une erreur : une liste vide se peint (voir [`GameServers::is_empty`]).
    pub fn from_json(value: &Value) -> Self {
        let entries = value
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| {
                        let code = row.get("code")?.as_str()?.to_string();
                        let label = row
                            .get("label")
                            .and_then(Value::as_str)
                            .unwrap_or(&code)
                            .to_string();
                        Some(GameServer {
                            code,
                            label,
                            is_active: row.get("isActive").and_then(Value::as_bool).unwrap_or(true),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self { entries }
    }

    /// Ce que le sélecteur propose : les serveurs ouverts, **plus celui déjà déclaré par le compte**
    /// même s'il ne l'est plus. Sans cette exception, choisir un autre serveur puis revenir serait
    /// impossible, et le sélecteur mentirait sur l'état réel du compte.
    pub fn selectable(&self, current: Option<&str>) -> Vec<&GameServer> {
        self.entries
            .iter()
            .filter(|server| server.is_active || current.is_some_and(|code| code == server.code))
            .collect()
    }

    /// Le nom d'affichage d'un code — le code lui-même si la liste n'est pas (encore) descendue,
    /// jamais un trou : un compte déclaré sur « pandora » doit rester lisible hors ligne.
    pub fn label<'a>(&'a self, code: &'a str) -> &'a str {
        self.entries
            .iter()
            .find(|server| server.code == code)
            .map(|server| server.label.as_str())
            .unwrap_or(code)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn liste() -> GameServers {
        GameServers::from_json(&serde_json::json!([
            { "code": "pandora", "label": "Pandora", "isActive": true },
            { "code": "rubilax", "label": "Rubilax", "isActive": true },
            { "code": "ogrest", "label": "Ogrest", "isActive": false },
            { "sansCode": true },
        ]))
    }

    #[test]
    fn une_entree_sans_code_est_ignoree() {
        assert_eq!(liste().selectable(None).len(), 2);
    }

    #[test]
    fn un_serveur_ferme_reste_proposable_au_compte_qui_le_porte() {
        let servers = liste();
        let codes: Vec<&str> = servers
            .selectable(Some("ogrest"))
            .iter()
            .map(|s| s.code.as_str())
            .collect();
        assert_eq!(codes, vec!["pandora", "rubilax", "ogrest"]);
    }

    #[test]
    fn un_code_inconnu_s_affiche_tel_quel() {
        assert_eq!(liste().label("pandora"), "Pandora");
        // Liste pas encore descendue, ou serveur retiré de la table : le compte reste lisible.
        assert_eq!(GameServers::default().label("pandora"), "pandora");
    }
}
