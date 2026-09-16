//! Roster de personnages déclarés par l'utilisateur (page profil du compte web) — miroir de
//! `character-roster.service.ts`, en **lecture** ([`RosterIndex`], source de vérité prioritaire
//! pour la classe/sexe d'un allié confirmé par nom exact, avant repli sur le `breed` du combat —
//! voir `class_breed.rs` et `session.rs`) et, depuis le 2026-09-16, en **écriture** ([`Roster`],
//! ce que l'onglet « Personnages » de la fenêtre Options édite).
//!
//! Pas d'IO ici (aucune dépendance réseau) : `from_settings_json` prend un `serde_json::Value`
//! déjà récupéré par `overlay-sync`, exactement comme le reste d'`overlay-engine` ne connaît que
//! des données déjà lues, et [`Roster::patch_value`] rend un `Value` que `overlay-sync` postera.
//!
//! ## Pourquoi deux types pour la même donnée
//!
//! [`RosterIndex`] est un index de lookup : il jette tout ce dont le moteur n'a pas besoin (`id`,
//! `label`, `isDefault`) et aplatit les comptes en tables par nom normalisé. C'était sans
//! conséquence tant que l'overlay ne faisait que lire.
//!
//! **Ça ne l'est plus dès qu'il écrit** : `PATCH /api/v1/settings` remplace la valeur ENTIÈRE
//! d'une clé (voir `functions/api/v1/settings.ts::onRequestPatch`), donc réécrire `roster` depuis
//! un index effacerait du compte l'identité même des comptes — et le site ne saurait plus lequel
//! est le principal. [`Roster`] garde donc la forme exacte du JSON, **y compris les champs que
//! l'overlay ne connaît pas** (voir `RosterAccount::extra`), sur le même principe que
//! `AccountSettings::profile_raw` pour la clé `profile`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use unicode_normalization::UnicodeNormalization;

/// Miroir de `Gender` (`class-icons.data.ts`) — féminin/masculin, les deux seules valeurs que le
/// jeu propose à la création de personnage. `Serialize` sert à la persistance disque des combats
/// en cours (voir `fight_store.rs`) — `FighterDamage::gender` doit survivre à un redémarrage — et
/// à la réécriture du roster sur le compte ([`Roster::patch_value`]).
/// `Hash` : clé de `HashMap` côté `overlay-ui` (`portraits::PortraitAtlas`, une texture par
/// couple classe/sexe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    F,
    M,
}

/// Miroir de `RosterCharacter` (`character-roster.service.ts`) — les trois champs que le site
/// déclare, ni plus ni moins : `gameServer` (voir [`RosterAccount`]) reste porté au niveau du
/// COMPTE, pas du personnage, et le niveau n'existe nulle part (`wakfu.log` ne le porte pas).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
///
/// **Sérialisable à l'identique de ce qui a été lu** : voir `extra`, et la doc de module pour ce
/// qu'un aller-retour appauvrissant coûterait.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RosterAccount {
    /// Identifiant stable du compte, créé par le site (`generateId`) ou par [`new_account_id`].
    /// **Jamais réattribué** : c'est lui que le site suit d'une écriture à l'autre.
    #[serde(default)]
    pub id: String,
    /// Le nom donné par l'utilisateur — **vide sur le compte principal**, que le site affiche
    /// « Principal » sans le laisser renommer (`profile-page.component.ts::tabLabel`).
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub characters: Vec<RosterCharacter>,
    /// Absent = non renseigné, jamais une valeur inventée (voir doc du champ côté web).
    #[serde(
        rename = "gameServer",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub game_server: Option<String>,
    /// Le compte créé automatiquement au premier lancement : toujours présent, et **jamais
    /// supprimable** — ni par le site, ni par l'overlay (voir [`Roster::remove_account`]).
    #[serde(rename = "isDefault", default, skip_serializing_if = "is_false")]
    pub is_default: bool,
    /// **Tout le reste du compte, tel qu'il est arrivé.** Rien n'en porte aujourd'hui, et c'est
    /// précisément pourquoi ce champ existe : le jour où le site ajoute une préférence par compte,
    /// une validation faite depuis l'overlay ne doit pas l'effacer du compte de l'utilisateur.
    #[serde(flatten)]
    extra: Map<String, Value>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl RosterAccount {
    /// Un compte neuf, tel que la modale « Nouveau compte » le crée.
    pub fn new(label: impl Into<String>, game_server: Option<String>) -> Self {
        Self {
            id: new_account_id(),
            label: label.into(),
            characters: Vec::new(),
            game_server,
            is_default: false,
            extra: Map::new(),
        }
    }

    /// Le personnage de ce compte portant ce nom (comparaison normalisée, comme partout ici).
    pub fn position(&self, name: &str) -> Option<usize> {
        let key = normalize_wakfu_name(name);
        self.characters
            .iter()
            .position(|c| normalize_wakfu_name(&c.name) == key)
    }
}

/// Identifiant de compte au format du site : `${base36(millis)}-${six caractères}` (voir
/// `generateId`, `character-roster.service.ts`).
///
/// La seconde moitié est aléatoire côté web (`Math.random`) ; elle est ici tirée du compteur de
/// nanosecondes et d'un compteur de processus, ce qui suffit à sa seule fonction — départager deux
/// comptes créés dans la même milliseconde. Pas de dépendance `rand` ajoutée à `overlay-engine`
/// pour six caractères dont personne ne lit la valeur.
pub fn new_account_id() -> String {
    static SUITE: AtomicU32 = AtomicU32::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let suffixe = u64::from(now.subsec_nanos())
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(u64::from(SUITE.fetch_add(1, Ordering::Relaxed)));
    format!(
        "{}-{:0>6}",
        base36(now.as_millis() as u64),
        base36(suffixe % 36u64.pow(6))
    )
}

/// Chiffres et minuscules, comme `Number.prototype.toString(36)`.
fn base36(mut value: u64) -> String {
    const ALPHABET: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while value > 0 {
        out.push(ALPHABET[(value % 36) as usize]);
        value /= 36;
    }
    out.reverse();
    String::from_utf8(out).expect("alphabet ASCII")
}

/// **Le roster tel qu'il s'édite** — la liste des comptes, dans l'ordre du site, chacun avec ses
/// personnages dans l'ordre que l'utilisateur leur a donné (le glisser-déposer de l'onglet et
/// celui de la page profil écrivent le même tableau).
///
/// C'est la valeur de la clé `roster` de `GET /api/v1/settings`, et c'est elle que
/// [`Roster::patch_value`] rend au `PATCH` — voir la doc de module pour pourquoi elle ne se
/// reconstruit pas depuis [`RosterIndex`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Roster {
    pub accounts: Vec<RosterAccount>,
}

impl Roster {
    /// Lit `data["roster"]` de la réponse `GET /api/v1/settings` en **garantissant un compte
    /// principal**, miroir exact de `CharacterRosterService.loadAccounts` : roster absent ou vide
    /// → un compte principal neuf ; aucun compte marqué `isDefault` (données écrites avant que le
    /// champ n'existe) → le premier le devient.
    ///
    /// Cette garantie est le pendant d'une règle de l'écran : il y a toujours un compte où
    /// déclarer un personnage, et jamais de « sans compte » à peindre.
    pub fn from_settings_json(data: &Value) -> Self {
        let mut accounts = accounts_from_settings_json(data);
        if accounts.is_empty() {
            accounts.push(RosterAccount {
                is_default: true,
                ..RosterAccount::new("", None)
            });
        } else if !accounts.iter().any(|a| a.is_default) {
            accounts[0].is_default = true;
        }
        Self { accounts }
    }

    /// La valeur de la clé `roster` à poster — voir `overlay_sync::client::patch_roster`, qui
    /// l'enveloppe dans son entrée `{ key, value, updatedAt }`.
    pub fn patch_value(&self) -> Value {
        serde_json::to_value(&self.accounts).unwrap_or(Value::Array(Vec::new()))
    }

    /// Le compte principal, celui que le site interdit de supprimer — le premier à défaut (voir
    /// [`Roster::from_settings_json`], qui le garantit).
    pub fn default_index(&self) -> usize {
        self.accounts.iter().position(|a| a.is_default).unwrap_or(0)
    }

    /// Retire un compte **sauf le principal**, comme `removeAccount` : la règle est celle du site,
    /// pas celle d'un bouton qu'on aurait oublié d'éteindre.
    pub fn remove_account(&mut self, index: usize) {
        if self.accounts.get(index).is_some_and(|a| !a.is_default) {
            self.accounts.remove(index);
        }
    }

    /// Déclare un personnage sur ce compte, **en écrasant un homonyme déjà déclaré** (comparaison
    /// normalisée) — miroir de `addCharacter`. Un nom vide n'est jamais déclaré.
    ///
    /// L'homonyme écrasé garde sa PLACE dans la liste : `addCharacter` côté web le retire puis
    /// réempile en fin de liste, ce qui casse un ordre choisi au glisser-déposer — le même reproche
    /// que sa propre doc fait à `renameCharacter`.
    pub fn declare(&mut self, account: usize, character: RosterCharacter) {
        if character.name.trim().is_empty() {
            return;
        }
        let Some(compte) = self.accounts.get_mut(account) else {
            return;
        };
        match compte.position(&character.name) {
            Some(place) => compte.characters[place] = character,
            None => compte.characters.push(character),
        }
    }

    /// Remplace le personnage de rang `index`, **et déduplique** si son nouveau nom est celui d'un
    /// autre personnage du même compte (même règle que [`Roster::declare`], côté modification).
    pub fn replace(&mut self, account: usize, index: usize, character: RosterCharacter) {
        if character.name.trim().is_empty() {
            return;
        }
        let Some(compte) = self.accounts.get_mut(account) else {
            return;
        };
        if index >= compte.characters.len() {
            return;
        }
        let key = normalize_wakfu_name(&character.name);
        compte.characters[index] = character;
        // Le rang édité est gardé quoi qu'il arrive ; ses homonymes ailleurs dans le compte
        // partent, sinon le même personnage y figurerait deux fois.
        let mut rang = 0usize;
        compte.characters.retain(|c| {
            let garde = rang == index || normalize_wakfu_name(&c.name) != key;
            rang += 1;
            garde
        });
    }

    /// Déplace un personnage dans la liste de son compte — miroir de `reorderCharacters`.
    pub fn reorder(&mut self, account: usize, from: usize, to: usize) {
        let Some(compte) = self.accounts.get_mut(account) else {
            return;
        };
        if from >= compte.characters.len() || to >= compte.characters.len() || from == to {
            return;
        }
        let moved = compte.characters.remove(from);
        compte.characters.insert(to, moved);
    }
}

/// L'entrée `{ key, value, updatedAt }` d'un `PATCH /api/v1/settings` pour la clé `roster` —
/// jumeau de `chat_alert::chat_filters_patch_entry` et de `profile::profile_patch_entry`.
///
/// La valeur est le tableau ENTIER des comptes ([`Roster::patch_value`]), jamais le seul compte
/// modifié : le serveur remplace la valeur de la clé, il ne fusionne pas (voir doc de module).
/// `updatedAt` arbitre le « dernier écrivain gagne », horodaté à l'instant de l'appel.
pub fn roster_patch_entry(roster: &Roster) -> Value {
    serde_json::json!({
        "key": "roster",
        "value": roster.patch_value(),
        "updatedAt": chrono::Utc::now().to_rfc3339(),
    })
}

/// Les comptes tels que le JSON les porte, **sans garantie de compte principal** — ce que
/// [`RosterIndex`] indexe (fidélité stricte à ce qui est écrit sur le compte) et ce dont
/// [`Roster::from_settings_json`] part avant d'appliquer la sienne.
fn accounts_from_settings_json(data: &Value) -> Vec<RosterAccount> {
    data.get("roster")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
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
    /// Les personnages de chaque compte, tels que déclarés — pour [`account_mates`]. Un client
    /// Wakfu joue le titulaire de sa fenêtre **et ses héros**, tous du même compte : c'est cette
    /// table qui dit, pour une fenêtre, quels noms peuvent y apparaître comme combattant actif.
    ///
    /// [`account_mates`]: RosterIndex::account_mates
    characters_by_account: Vec<Vec<String>>,
    account_by_normalized_name: HashMap<String, usize>,
}

impl RosterIndex {
    /// Construit l'index depuis `data["roster"]` de la réponse `GET /api/v1/settings` — vide (pas
    /// une erreur) si la clé est absente, ce qui est le cas normal en mode invité ou pour un
    /// compte qui n'a encore déclaré aucun personnage.
    pub fn from_settings_json(data: &serde_json::Value) -> Self {
        // **Les comptes BRUTS, sans la garantie de compte principal de `Roster`** : cet index dit
        // ce que le compte porte, il ne le complète pas — un compte principal inventé ici
        // n'apporterait aucun personnage à indexer et ferait mentir `characters_by_account`, dont
        // les rangs sont ceux du JSON.
        let accounts = accounts_from_settings_json(data);

        let mut by_normalized_name = HashMap::new();
        let mut game_server_by_normalized_name = HashMap::new();
        let mut characters_by_account = Vec::new();
        let mut account_by_normalized_name = HashMap::new();
        for (account_index, account) in accounts.into_iter().enumerate() {
            characters_by_account.push(
                account
                    .characters
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<Vec<_>>(),
            );
            for character in account.characters {
                // Dernier écrivain gagne en cas d'homonyme entre deux comptes — comportement
                // best-effort assumé, un vrai conflit de nom entre comptes est un cas limite que
                // le web lui-même ne résout pas autrement (premier compte trouvé dans son
                // `findCharacter`, ici l'ordre d'itération du JSON n'est pas garanti identique de
                // toute façon). Même choix pour le gameServer du compte : les deux tables
                // partagent le même dernier écrivain, jamais désynchronisées l'une de l'autre.
                let key = normalize_wakfu_name(&character.name);
                game_server_by_normalized_name.insert(key.clone(), account.game_server.clone());
                account_by_normalized_name.insert(key.clone(), account_index);
                by_normalized_name.insert(key, character);
            }
        }
        Self {
            by_normalized_name,
            game_server_by_normalized_name,
            characters_by_account,
            account_by_normalized_name,
        }
    }

    /// Les personnages du **même compte** que `name`, lui compris, tels que déclarés — vide si
    /// `name` n'est pas au roster. Un client Wakfu joue jusqu'à trois personnages d'un compte (le
    /// titulaire de la fenêtre et ses héros) : ce sont les noms qu'une fenêtre peut afficher comme
    /// combattant actif (§9.1 decies du plan, surveillance de tour).
    pub fn account_mates(&self, name: &str) -> Vec<String> {
        self.account_by_normalized_name
            .get(&normalize_wakfu_name(name))
            .and_then(|&i| self.characters_by_account.get(i))
            .cloned()
            .unwrap_or_default()
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

    #[test]
    fn les_personnages_d_un_meme_compte_se_retrouvent() {
        let data = serde_json::json!({ "roster": [
            { "characters": [
                { "name": "Oumbra", "className": "Sram", "gender": "m" },
                { "name": "Sagitta Lucis", "className": "Cra", "gender": "f" }
            ], "gameServer": "pandora" },
            { "characters": [
                { "name": "Pugio Letalis", "className": "Iop", "gender": "m" }
            ] }
        ]});
        let index = RosterIndex::from_settings_json(&data);
        let mut mates = index.account_mates("oumbra");
        mates.sort();
        assert_eq!(mates, vec!["Oumbra", "Sagitta Lucis"]);
        assert_eq!(index.account_mates("Pugio Letalis"), vec!["Pugio Letalis"]);
        assert!(index.account_mates("Inconnu").is_empty());
    }

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

    // ---------------------------------------------------------------------------------------
    // Le roster ÉDITABLE — voir la doc de module : ce qui se perd ici se perd sur le compte.
    // ---------------------------------------------------------------------------------------

    #[test]
    fn un_aller_retour_ne_perd_rien_de_ce_que_le_compte_portait() {
        // `theme` n'existe nulle part dans l'overlay : c'est exactement le champ qu'une écriture
        // naïve effacerait du compte de l'utilisateur.
        let data = serde_json::json!({ "roster": [{
            "id": "acc-1",
            "label": "",
            "isDefault": true,
            "gameServer": "pandora",
            "theme": "sombre",
            "characters": [{ "name": "Oumbra", "className": "sram", "gender": "m" }],
        }]});
        let roster = Roster::from_settings_json(&data);
        assert_eq!(roster.patch_value(), data["roster"]);
    }

    #[test]
    fn un_compte_principal_est_garanti() {
        // Roster absent : le compte principal est créé, comme `loadAccounts` côté web.
        let neuf = Roster::from_settings_json(&serde_json::json!({}));
        assert_eq!(neuf.accounts.len(), 1);
        assert!(neuf.accounts[0].is_default);
        assert!(!neuf.accounts[0].id.is_empty());

        // Données écrites avant que `isDefault` n'existe : le premier compte le devient.
        let ancien = Roster::from_settings_json(&serde_json::json!({ "roster": [
            { "id": "a", "label": "Mules", "characters": [] },
            { "id": "b", "label": "Métiers", "characters": [] },
        ]}));
        assert_eq!(ancien.default_index(), 0);
        assert!(!ancien.accounts[1].is_default);
    }

    #[test]
    fn l_index_de_lecture_ne_fabrique_aucun_compte() {
        // La garantie ci-dessus est celle de l'écran d'édition, pas celle du moteur : un compte
        // principal inventé ici donnerait un rang de compte qui ne correspond à rien du JSON.
        let index = RosterIndex::from_settings_json(&serde_json::json!({}));
        assert!(index.is_empty());
        assert!(index.account_mates("quiconque").is_empty());
    }

    #[test]
    fn declarer_un_homonyme_le_remplace_sans_le_deplacer() {
        let mut roster = Roster::from_settings_json(&sample_settings());
        roster.declare(
            0,
            RosterCharacter {
                // Casse et accents différents : c'est le même personnage (`normalize_wakfu_name`).
                name: "eclair-io".to_string(),
                class_name: "cra".to_string(),
                gender: Gender::F,
            },
        );
        let compte = &roster.accounts[0];
        assert_eq!(compte.characters.len(), 2, "aucun doublon ajouté");
        assert_eq!(
            compte.characters[0].name, "eclair-io",
            "même place qu'avant"
        );
        assert_eq!(compte.characters[0].class_name, "cra");
        assert_eq!(compte.characters[1].name, "Brise'Os", "voisin intouché");
    }

    #[test]
    fn un_nom_vide_ne_declare_personne() {
        let mut roster = Roster::from_settings_json(&sample_settings());
        roster.declare(
            0,
            RosterCharacter {
                name: "   ".to_string(),
                class_name: "iop".to_string(),
                gender: Gender::M,
            },
        );
        assert_eq!(roster.accounts[0].characters.len(), 2);
    }

    #[test]
    fn modifier_un_personnage_en_homonyme_d_un_autre_les_fusionne() {
        let mut roster = Roster::from_settings_json(&sample_settings());
        // Le rang 1 prend le nom du rang 0 : il ne peut pas rester deux « Éclair-Ïo » au compte.
        roster.replace(
            0,
            1,
            RosterCharacter {
                name: "Éclair-Ïo".to_string(),
                class_name: "sram".to_string(),
                gender: Gender::F,
            },
        );
        let compte = &roster.accounts[0];
        assert_eq!(compte.characters.len(), 1);
        assert_eq!(compte.characters[0].class_name, "sram", "l'édition gagne");
    }

    #[test]
    fn le_compte_principal_ne_se_supprime_pas() {
        let mut roster = Roster::from_settings_json(&sample_settings());
        let avant = roster.accounts.len();
        roster.remove_account(roster.default_index());
        assert_eq!(roster.accounts.len(), avant, "le principal reste");
        roster.remove_account(1);
        assert_eq!(roster.accounts.len(), avant - 1);
    }

    #[test]
    fn le_deplacement_conserve_la_liste() {
        let mut roster = Roster::from_settings_json(&sample_settings());
        roster.reorder(0, 1, 0);
        let noms: Vec<&str> = roster.accounts[0]
            .characters
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(noms, vec!["Brise'Os", "Éclair-Ïo"]);
        // Rangs hors liste : sans effet, jamais une panique.
        roster.reorder(0, 9, 0);
        roster.reorder(9, 0, 1);
        assert_eq!(roster.accounts[0].characters.len(), 2);
    }

    #[test]
    fn deux_comptes_crees_dans_la_meme_milliseconde_ont_des_identifiants_distincts() {
        let ids: std::collections::HashSet<String> = (0..50).map(|_| new_account_id()).collect();
        assert_eq!(ids.len(), 50);
    }
}
