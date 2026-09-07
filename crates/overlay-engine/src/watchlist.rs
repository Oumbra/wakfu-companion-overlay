//! Suivi (watchlist) — compteurs d'objets/ennemis, §9 du plan. Miroir direct, réimplémenté en
//! Rust plutôt que vendu depuis `StatsStoreService` (`registerLoot`/`registerDefeat`/
//! `incrementWatched`, `stats-store.service.ts`) — voir `docs/plan-architecture.md` §14 point 3
//! pour la décision : la logique de comptage réelle tient en une trentaine de lignes isolées côté
//! web (simple égalité de nom, `isInitialLoad` gating), sans rien de comparable aux heuristiques
//! de corrélation kamas↔HDV qui ont motivé la décision B du §2 (réutiliser le TS plutôt que
//! réécrire) — un port direct minimise le risque de divergence pour CE périmètre précis, sans
//! attendre l'extraction complète du moteur headless (toujours nécessaire pour le récap de
//! session complet, toujours une décision ouverte pour cette partie-là).
//!
//! **Deux sources bien distinctes, décision utilisateur 2026-09-01** :
//! - La LISTE des entrées suivies (nom/genre/mode/cible) reste éditée sur le web et lue en
//!   lecture seule ici, comme le roster (`WatchlistEntry::from_settings_json`, même source
//!   `GET /api/v1/settings`, clé `"watchlist"`).
//! - Les COMPTEURS (`count`), eux, sont incrémentés et persistés **localement** par l'overlay en
//!   premier lieu (`store_path`, jamais perdu même hors ligne), PUIS répliqués vers le compte via
//!   `PATCH /api/v1/settings` — voir `drain_pending_sync`/`watchlist_patch_entry` ci-dessous et
//!   `overlay_sync::client::patch_watchlist` côté transport. **Fermé le 2026-09-07** (retour
//!   utilisateur : un Suivi jamais visible sur le site) : cette réplication manquait entièrement
//!   jusqu'ici (voir `docs/plan-architecture.md` §14 point 3) — un overlay et le web utilisés en
//!   parallèle sur le même personnage restent malgré tout `count`-divergents PENDANT la fenêtre
//!   de debounce/backoff (miroir exact de `RemoteUserDataRepository` côté web, jamais
//!   instantané non plus là-bas).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::LogEntry;

/// Miroir de `WatchlistKind` (`stats-store.service.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatchlistKind {
    Enemy,
    Item,
}

/// Miroir de `WatchlistCounterMode` — 'up' compte vers le haut depuis 0, 'down' décompte depuis
/// `countdown_target` vers 0 (borné, jamais négatif — voir `WatchlistState::increment`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatchlistMode {
    Up,
    Down,
}

/// Émis par `WatchlistState::apply` quand un décompte (mode `down`) vient d'atteindre 0 — §9 du
/// plan, « Alertes de drop » : « toast + son quand un objet suivi tombe ». Miroir minimal de
/// `LootAlertEvent` (`loot-alert.service.ts`), réduit au seul cas `reason: 'countdown'` : le cas
/// `reason: 'loot'` (son configurable par objet ramassé, indépendant de la watchlist — voir
/// `ProfileService.findEnabledSoundItem`) est couvert séparément par `overlay_engine::profile::
/// LootAlert`.
#[derive(Debug, Clone, PartialEq)]
pub struct WatchlistAlert {
    pub name: String,
    pub kind: WatchlistKind,
    /// Repris de `WatchlistEntry::catalog_id` de l'entrée qui vient de déclencher l'alerte —
    /// résolution non ambiguë de l'icône affichée par le toast (`panels::watchlist`) en cas
    /// d'homonymes, même principe que `LootAlert::catalog_id`.
    pub catalog_id: Option<i64>,
}

/// Miroir de `WatchlistEntry` (`stats-store.service.ts`). `catalog_id` est lu depuis le compte
/// mais n'intervient JAMAIS dans le comptage (le log ne référence jamais un id, seulement un nom
/// — voir la doc TS d'origine) : conservé uniquement pour un futur affichage d'icône exacte en
/// cas d'homonymes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchlistEntry {
    pub name: String,
    pub kind: WatchlistKind,
    pub mode: WatchlistMode,
    #[serde(default)]
    pub count: i64,
    #[serde(rename = "countdownTarget", default)]
    pub countdown_target: i64,
    #[serde(rename = "catalogId", default)]
    pub catalog_id: Option<i64>,
}

/// Nom d'app DISTINCT en test — même précaution que `overlay_sync::token_store` (2026-09-01) :
/// un test qui écrirait dans le VRAI fichier de compteurs de l'utilisateur écraserait un suivi en
/// cours à chaque `cargo test`.
#[cfg(not(test))]
const APP_NAME: &str = "wakfu-companion-overlay";
#[cfg(test)]
const APP_NAME: &str = "wakfu-companion-overlay-test";

/// Emplacement par défaut du fichier de compteurs locaux — `Engine` l'utilise implicitement à la
/// construction ; les tests de ce module passent un chemin de fichier temporaire explicite à
/// `load_from`/`save_to` plutôt que de dépendre de celui-ci.
pub fn default_store_path() -> PathBuf {
    directories::ProjectDirs::from("", "", APP_NAME)
        .map(|dirs| dirs.data_dir().join("watchlist-counts.json"))
        .unwrap_or_else(|| PathBuf::from("watchlist-counts.json"))
}

/// Compteur persisté pour une entrée (`name` + `kind`, voir `increment` — deux entrées peuvent
/// partager un nom si l'une est `enemy` et l'autre `item`, cas limite mais géré). Fichier séparé
/// des entrées elles-mêmes (`Vec<WatchlistEntry>` vient du compte, voir doc de module) : seul le
/// `count` doit survivre localement à un redémarrage.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedCounts {
    #[serde(default)]
    by_key: HashMap<String, i64>,
}

fn counter_key(name: &str, kind: WatchlistKind) -> String {
    format!("{:?}:{}", kind, name.to_lowercase())
}

fn load_from(path: &Path) -> PersistedCounts {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_to(path: &Path, counts: &PersistedCounts) {
    let Ok(json) = serde_json::to_string(counts) else {
        return; // ne devrait jamais arriver (types simples, toujours sérialisables)
    };
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            tracing::warn!(path = %path.display(), "impossible de créer le dossier des compteurs de suivi");
            return;
        }
    }
    if let Err(err) = std::fs::write(path, json) {
        tracing::warn!(path = %path.display(), %err, "impossible d'écrire les compteurs de suivi (perdus au redémarrage)");
    }
}

/// Accumulateur mutable des entrées suivies — voir la doc de module pour la frontière
/// définitions (compte)/compteurs (local). Vit sur `Engine`, PAS sur `SessionState` : contrairement
/// aux dégâts d'un combat, un compteur de suivi doit survivre à un rattrapage `isInitialLoad`
/// (rotation du log, reconnexion) — voir `Engine::ingest_batch`.
#[derive(Debug, Default)]
pub struct WatchlistState {
    entries: Vec<WatchlistEntry>,
    store_path: PathBuf,
    /// `true` dès qu'un `count` a changé (`apply`) ou qu'un rattrapage est nécessaire
    /// (`merge_config`, voir sa doc) depuis le dernier `drain_pending_sync` — voir cette dernière
    /// pour l'usage. Jamais réinitialisé par `persist()` (fichier LOCAL, indépendant de la
    /// réplication réseau) : seul `drain_pending_sync` l'efface, PAS un envoi réseau réussi — cette
    /// distinction ne vit pas ici (voir sa doc pour pourquoi un échec réseau n'a pas besoin de
    /// remarquer `dirty`).
    dirty: bool,
}

impl WatchlistState {
    /// Charge les compteurs persistés localement (`store_path`, best-effort — un fichier absent
    /// ou corrompu redémarre simplement à 0, jamais une erreur bloquante) sans encore aucune
    /// entrée : voir `merge_config`, appelé par l'hôte dès la première réponse
    /// `GET /api/v1/settings`.
    pub fn new(store_path: PathBuf) -> Self {
        Self {
            entries: Vec::new(),
            store_path,
            dirty: false,
        }
    }

    /// Remplace la liste des entrées suivies par celle du compte (`from_settings_json`), en
    /// conservant le `count` local déjà en cours pour toute entrée déjà connue (clé
    /// `name`+`kind`) — c'est ce qui rend le comptage local persistant d'un redémarrage à l'autre
    /// malgré un re-fetch du compte à chaque connexion. Une entrée disparue du compte (supprimée
    /// sur le web) disparaît aussi d'ici ; une entrée nouvelle démarre avec le `count`/
    /// `countdown_target` que le compte lui donne.
    ///
    /// Marque l'état `dirty` (voir `take_pending_sync`) dès qu'au moins un `count` local diffère
    /// de celui reçu ici : un compte qui répond avec une valeur en retard (compteur incrémenté
    /// hors ligne, ou dernier envoi resté en échec avant une déconnexion/fermeture) doit être
    /// rattrapé, exactement comme `RemoteUserDataRepository.pull()` pousse tout champ « jamais
    /// envoyé » ou localement plus récent côté web.
    pub fn merge_config(&mut self, incoming: Vec<WatchlistEntry>) {
        let persisted = load_from(&self.store_path);
        let mut needs_catchup = false;
        self.entries = incoming
            .into_iter()
            .map(|mut entry| {
                if let Some(&count) = persisted.by_key.get(&counter_key(&entry.name, entry.kind)) {
                    if count != entry.count {
                        needs_catchup = true;
                    }
                    entry.count = count;
                }
                entry
            })
            .collect();
        if needs_catchup {
            self.dirty = true;
        }
    }

    pub fn entries(&self) -> &[WatchlistEntry] {
        &self.entries
    }

    /// Vide et renvoie un instantané des entrées suivies à répliquer vers le compte si `dirty`
    /// (`None` dans l'immense majorité des appels) — à appeler par l'hôte (`overlay-ui`) après
    /// chaque `apply`/`merge_config`, même motif « drain » qu'`Engine::drain_sync_events`/
    /// `drain_watchlist_alerts`. Comme pour `drain_sync_events`, l'hôte relaie l'instantané tel
    /// quel au thread Sync sans jamais rappeler l'`Engine` (§3 du plan) : un échec réseau ne doit
    /// donc PAS reposer sur un nouveau drain pour être réessayé — c'est au thread Sync de garder
    /// le dernier envoi en attente et de le retenter (backoff), pas à ce module.
    pub fn drain_pending_sync(&mut self) -> Option<Vec<WatchlistEntry>> {
        if !self.dirty {
            return None;
        }
        self.dirty = false;
        Some(self.entries.clone())
    }

    /// Applique un `LogEntry` déjà déterminé comme HORS rattrapage initial par l'appelant (voir
    /// `Engine::ingest_batch`) — miroir du gating `if (this.currentBatchIsInitialLoad) return;`
    /// fait côté web dans `registerLoot`/`registerDefeat`, pas ici : `WatchlistState` n'a aucune
    /// notion de rattrapage en cours, exactement comme `roster` sur `Engine`. Renvoie les alertes
    /// (voir `WatchlistAlert`) déclenchées par CET événement, à faire remonter à l'hôte (toast +
    /// son) — vide dans l'immense majorité des cas (mode `up`, ou aucune entrée qui vient
    /// d'atteindre 0).
    pub fn apply(&mut self, entry: &LogEntry) -> Vec<WatchlistAlert> {
        let (changed, alerts) = match entry {
            LogEntry::Loot { item, quantity, .. } => self.increment(item, *quantity),
            LogEntry::EnemyDefeated { name, .. } => self.increment(name, 1),
            _ => (false, Vec::new()),
        };
        if changed {
            self.persist();
            self.dirty = true;
        }
        alerts
    }

    /// Incrémente (mode `up`) ou décompte vers 0 (mode `down`) TOUTE entrée dont le nom matche —
    /// miroir exact d'`incrementWatched` : le matching se fait sur le nom SEUL, pas sur `kind`
    /// (un objet et un ennemi de même nom incrémenteraient tous les deux, comme côté web).
    /// Renvoie `true` si au moins une entrée a été modifiée (sert à ne persister sur disque que
    /// quand c'est utile), et la liste des entrées qui viennent de FRANCHIR le seuil de 0 (compte
    /// strictement positif avant, nul après) — un compte déjà à 0 qui reste à 0 n'alerte pas une
    /// deuxième fois, miroir du `if (entry.count > 0 && next === 0)` d'`incrementWatched`.
    fn increment(&mut self, raw_name: &str, by: i64) -> (bool, Vec<WatchlistAlert>) {
        let normalized = raw_name.trim().to_lowercase();
        let mut changed = false;
        let mut alerts = Vec::new();
        for entry in &mut self.entries {
            if entry.name.to_lowercase() != normalized {
                continue;
            }
            changed = true;
            match entry.mode {
                WatchlistMode::Down => {
                    let was_positive = entry.count > 0;
                    entry.count = (entry.count - by).max(0);
                    if was_positive && entry.count == 0 {
                        alerts.push(WatchlistAlert {
                            name: entry.name.clone(),
                            kind: entry.kind,
                            catalog_id: entry.catalog_id,
                        });
                    }
                }
                WatchlistMode::Up => entry.count += by,
            }
        }
        (changed, alerts)
    }

    fn persist(&self) {
        let by_key = self
            .entries
            .iter()
            .map(|e| (counter_key(&e.name, e.kind), e.count))
            .collect();
        save_to(&self.store_path, &PersistedCounts { by_key });
    }
}

/// Construit la liste des entrées suivies depuis `data["watchlist"]` de la réponse
/// `GET /api/v1/settings` — vide (pas une erreur) si la clé est absente, comme
/// `RosterIndex::from_settings_json` (mode invité, ou compte sans watchlist déclarée).
pub fn watchlist_from_settings_json(data: &serde_json::Value) -> Vec<WatchlistEntry> {
    data.get("watchlist")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// Construit l'entrée `PATCH /api/v1/settings` (`{ key, value, updatedAt }`, voir
/// `functions/api/v1/settings.ts::onRequestPatch` côté dépôt web) pour la clé `"watchlist"` —
/// symétrique de `watchlist_from_settings_json`. `value` est la liste COMPLÈTE des entrées suivies
/// (le serveur remplace la clé entière, pas un merge, voir sa doc) ; `updatedAt` arbitre le
/// « dernier écrivain gagne » — horodaté à l'instant de l'appel, miroir de
/// `RemoteUserDataRepository.sendPending()` (`this.local.updatedAt(key) ?? new Date()`) côté web.
pub fn watchlist_patch_entry(entries: &[WatchlistEntry]) -> serde_json::Value {
    serde_json::json!({
        "key": "watchlist",
        "value": entries,
        "updatedAt": chrono::Utc::now().to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, mode: WatchlistMode, countdown_target: i64) -> WatchlistEntry {
        WatchlistEntry {
            name: name.to_string(),
            kind: WatchlistKind::Item,
            mode,
            count: if mode == WatchlistMode::Down {
                countdown_target
            } else {
                0
            },
            countdown_target,
            catalog_id: None,
        }
    }

    fn enemy(name: &str) -> WatchlistEntry {
        WatchlistEntry {
            name: name.to_string(),
            kind: WatchlistKind::Enemy,
            mode: WatchlistMode::Up,
            count: 0,
            countdown_target: 0,
            catalog_id: None,
        }
    }

    fn loot(item_name: &str, quantity: i64) -> LogEntry {
        LogEntry::Loot {
            time: "12:00:00,000".to_string(),
            item: item_name.to_string(),
            quantity,
            fight_id: None,
        }
    }

    fn defeated(name: &str) -> LogEntry {
        LogEntry::EnemyDefeated {
            time: "12:00:00,000".to_string(),
            name: name.to_string(),
            fight_id: None,
        }
    }

    /// `apply()` persiste sur `store_path` à chaque changement (voir `WatchlistState::apply`) :
    /// même pour ces tests de logique pure qui n'appellent jamais `merge_config`/`load_from`, un
    /// chemin RELATIF écrirait un fichier parasite dans le dossier du crate à chaque `cargo test`
    /// (vécu en session, 2026-09-01) — toujours passer par le dossier temp du système, jamais un
    /// chemin relatif.
    fn state_with(entries: Vec<WatchlistEntry>) -> WatchlistState {
        let path = std::env::temp_dir().join(format!(
            "wakfu-overlay-watchlist-test-pure-{}-{}.json",
            std::process::id(),
            NEXT_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let mut state = WatchlistState::new(path);
        state.entries = entries;
        state
    }

    static NEXT_TEST_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    #[test]
    fn ramassage_incremente_une_entree_item_en_mode_up() {
        let mut state = state_with(vec![item("Larve Bleue", WatchlistMode::Up, 0)]);
        state.apply(&loot("Larve Bleue", 3));
        assert_eq!(state.entries()[0].count, 3);
    }

    #[test]
    fn ennemi_vaincu_incremente_une_entree_enemy() {
        let mut state = state_with(vec![enemy("Bwork Sram")]);
        state.apply(&defeated("Bwork Sram"));
        assert_eq!(state.entries()[0].count, 1);
    }

    #[test]
    fn insensible_a_la_casse_et_aux_espaces_superflus() {
        let mut state = state_with(vec![enemy("Bwork Sram")]);
        state.apply(&defeated("  bwork sram  "));
        assert_eq!(state.entries()[0].count, 1);
    }

    #[test]
    fn mode_down_decompte_vers_zero_sans_jamais_passer_negatif() {
        let mut state = state_with(vec![item("Ortie Sauvage", WatchlistMode::Down, 2)]);
        assert_eq!(state.entries()[0].count, 2);
        state.apply(&loot("Ortie Sauvage", 1));
        assert_eq!(state.entries()[0].count, 1);
        state.apply(&loot("Ortie Sauvage", 5)); // dépasse largement la cible restante
        assert_eq!(state.entries()[0].count, 0);
    }

    #[test]
    fn mode_down_declenche_une_alerte_en_atteignant_zero_pour_la_premiere_fois() {
        let mut state = state_with(vec![item("Ortie Sauvage", WatchlistMode::Down, 2)]);
        assert!(state.apply(&loot("Ortie Sauvage", 1)).is_empty()); // 2 -> 1, pas encore 0
        let alerts = state.apply(&loot("Ortie Sauvage", 1)); // 1 -> 0
        assert_eq!(
            alerts,
            vec![WatchlistAlert {
                name: "Ortie Sauvage".to_string(),
                kind: WatchlistKind::Item,
                catalog_id: None,
            }]
        );
    }

    #[test]
    fn mode_down_ne_realerte_pas_une_fois_deja_a_zero() {
        let mut state = state_with(vec![item("Ortie Sauvage", WatchlistMode::Down, 1)]);
        assert!(!state.apply(&loot("Ortie Sauvage", 1)).is_empty()); // 1 -> 0, alerte
        assert!(state.apply(&loot("Ortie Sauvage", 1)).is_empty()); // déjà à 0, silence
    }

    #[test]
    fn mode_up_ne_declenche_jamais_dalerte() {
        let mut state = state_with(vec![item("Larve Bleue", WatchlistMode::Up, 0)]);
        assert!(state.apply(&loot("Larve Bleue", 100)).is_empty());
    }

    #[test]
    fn aucune_entree_correspondante_ne_change_rien() {
        let mut state = state_with(vec![item("Larve Bleue", WatchlistMode::Up, 0)]);
        state.apply(&loot("Autre Objet", 1));
        assert_eq!(state.entries()[0].count, 0);
    }

    #[test]
    fn entree_item_et_entree_enemy_de_meme_nom_incrementent_toutes_les_deux() {
        // Comportement hérité tel quel du web (`incrementWatched` matche par nom seul, pas par
        // kind) — cas limite documenté plutôt que "corrigé" en silence, voir doc de `increment`.
        let mut state = state_with(vec![item("Chafer", WatchlistMode::Up, 0), enemy("Chafer")]);
        state.apply(&loot("Chafer", 1));
        assert_eq!(state.entries()[0].count, 1);
        assert_eq!(state.entries()[1].count, 1);
    }

    #[test]
    fn merge_config_conserve_le_compte_local_pour_une_entree_deja_connue() {
        let dir = std::env::temp_dir().join(format!(
            "wakfu-overlay-watchlist-test-{}",
            std::process::id()
        ));
        let path = dir.join("counts.json");
        let mut state = WatchlistState::new(path.clone());

        state.merge_config(vec![item("Larve Bleue", WatchlistMode::Up, 0)]);
        state.apply(&loot("Larve Bleue", 4));
        assert_eq!(state.entries()[0].count, 4);

        // Simule un redémarrage : nouvelle instance, même fichier, re-fetch du compte avec un
        // `count` de départ à 0 (comme si le compte n'avait jamais rien compté côté web) — le
        // compte local persisté doit gagner.
        let mut restarted = WatchlistState::new(path.clone());
        restarted.merge_config(vec![item("Larve Bleue", WatchlistMode::Up, 0)]);
        assert_eq!(restarted.entries()[0].count, 4);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn merge_config_retire_une_entree_disparue_du_compte() {
        let dir = std::env::temp_dir().join(format!(
            "wakfu-overlay-watchlist-test-retrait-{}",
            std::process::id()
        ));
        let path = dir.join("counts.json");
        let mut state = WatchlistState::new(path);

        state.merge_config(vec![
            item("Larve Bleue", WatchlistMode::Up, 0),
            enemy("Bwork"),
        ]);
        state.merge_config(vec![enemy("Bwork")]); // "Larve Bleue" retirée côté compte

        assert_eq!(state.entries().len(), 1);
        assert_eq!(state.entries()[0].name, "Bwork");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn apply_marque_dirty_et_drain_pending_sync_le_vide() {
        let mut state = state_with(vec![enemy("Bwork Sram")]);
        assert_eq!(state.drain_pending_sync(), None); // rien à synchroniser au départ
        state.apply(&defeated("Bwork Sram"));
        let pending = state.drain_pending_sync();
        assert!(pending.is_some());
        assert_eq!(pending.unwrap()[0].count, 1);
        // Un deuxième drain sans nouveau changement entre-temps ne renvoie plus rien.
        assert_eq!(state.drain_pending_sync(), None);
    }

    #[test]
    fn apply_sans_entree_correspondante_ne_marque_pas_dirty() {
        let mut state = state_with(vec![item("Larve Bleue", WatchlistMode::Up, 0)]);
        state.apply(&loot("Autre Objet", 1));
        assert_eq!(state.drain_pending_sync(), None);
    }

    #[test]
    fn merge_config_marque_dirty_si_le_compte_est_en_retard_sur_le_local() {
        let dir = std::env::temp_dir().join(format!(
            "wakfu-overlay-watchlist-test-rattrapage-{}",
            std::process::id()
        ));
        let path = dir.join("counts.json");
        let mut state = WatchlistState::new(path.clone());

        state.merge_config(vec![item("Larve Bleue", WatchlistMode::Up, 0)]);
        assert_eq!(state.drain_pending_sync(), None); // premier fetch, rien à rattraper
        state.apply(&loot("Larve Bleue", 4));
        assert!(state.drain_pending_sync().is_some()); // drainé par l'appel ci-dessus

        // Redémarrage : le compte répond toujours avec un `count` à 0 (l'envoi précédent n'a
        // jamais atteint le compte, ex. overlay fermé avant la synchro) — le compte local
        // persisté (4) doit à la fois gagner ET redéclencher un rattrapage.
        let mut restarted = WatchlistState::new(path);
        restarted.merge_config(vec![item("Larve Bleue", WatchlistMode::Up, 0)]);
        let pending = restarted.drain_pending_sync();
        assert_eq!(pending.map(|p| p[0].count), Some(4));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn merge_config_ne_marque_pas_dirty_si_deja_a_jour() {
        let dir = std::env::temp_dir().join(format!(
            "wakfu-overlay-watchlist-test-deja-a-jour-{}",
            std::process::id()
        ));
        let path = dir.join("counts.json");
        let mut state = WatchlistState::new(path.clone());

        state.merge_config(vec![item("Larve Bleue", WatchlistMode::Up, 0)]);
        state.apply(&loot("Larve Bleue", 4));
        state.drain_pending_sync(); // synchro "réussie" simulée

        // Le compte répond maintenant avec le count déjà à jour (4) : rien à rattraper. (`item()`
        // n'affecte `count` qu'en mode `Down`, voir sa doc — construit donc directement ici.)
        let mut up_to_date = item("Larve Bleue", WatchlistMode::Up, 0);
        up_to_date.count = 4;
        let mut restarted = WatchlistState::new(path);
        restarted.merge_config(vec![up_to_date]);
        assert_eq!(restarted.drain_pending_sync(), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn watchlist_patch_entry_porte_la_cle_watchlist_et_les_entrees() {
        let entries = vec![item("Larve Bleue", WatchlistMode::Up, 0)];
        let patch = watchlist_patch_entry(&entries);
        assert_eq!(patch["key"], "watchlist");
        assert_eq!(patch["value"][0]["name"], "Larve Bleue");
        assert!(patch["updatedAt"].is_string());
    }

    #[test]
    fn watchlist_from_settings_json_absente_renvoie_liste_vide() {
        let entries = watchlist_from_settings_json(&serde_json::json!({}));
        assert!(entries.is_empty());
    }

    #[test]
    fn watchlist_from_settings_json_parse_les_entrees_reelles() {
        let entries = watchlist_from_settings_json(&serde_json::json!({
            "watchlist": [
                {
                    "name": "Larve Bleue",
                    "kind": "item",
                    "mode": "up",
                    "count": 7,
                    "countdownTarget": 0,
                    "catalogId": 123,
                },
            ],
        }));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Larve Bleue");
        assert_eq!(entries[0].kind, WatchlistKind::Item);
        assert_eq!(entries[0].count, 7);
        assert_eq!(entries[0].catalog_id, Some(123));
    }
}
