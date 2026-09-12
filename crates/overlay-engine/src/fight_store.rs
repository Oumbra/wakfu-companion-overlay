//! Persistance disque du (des) combat(s) EN COURS — complète `Engine::state_initialized` (voir sa
//! doc, `session.rs`) : ce champ protège déjà un combat actif d'une rotation de `wakfu.log`
//! survenant PENDANT que l'overlay tourne, mais rien ne protège une rotation survenue PENDANT que
//! l'overlay est ARRÊTÉ — au redémarrage, `Engine::new()` repart d'un `SessionState` vide, et le
//! nouveau `wakfu.log` ne rejoue jamais l'historique déjà lu (même constat, voir la doc citée) :
//! sans ce module, un combat toujours en cours en jeu réapparaîtrait à zéro (dégâts perdus) le
//! temps qu'une nouvelle ligne de dégâts survienne.
//!
//! Un fichier JSON par combat encore `ongoing`, `fight-{fight_id}.json`, sous
//! `%APPDATA%/wakfu-companion-overlay/data/` (voir `default_store_dir`, même racine
//! `directories::ProjectDirs` que `watchlist::default_store_path`) : écrit après chaque lot qui
//! touche un combat encore en cours (voir `Engine::ingest_batch`), supprimé dès que le combat se
//! termine (`CombatEnd`) — il n'y a alors plus rien à restaurer pour ce `fight_id`. Au démarrage,
//! `Engine::new()`/`with_stores` recharge tout fichier restant (donc forcément un combat qui n'a
//! jamais reçu sa ligne de fin) et le réinjecte dans `SessionState` avant tout premier lot ingéré
//! (voir `SessionState::restore_fight`).
//!
//! Ce qui est persisté est volontairement réduit à `FightSnapshot` (ce que l'UI affiche : liste des
//! combattants alliés/ennemis et leurs totaux) plutôt qu'à `FightWorking` (l'état interne
//! d'attribution des dégâts par siège d'initiative, propre à un process). La file d'initiative
//! repart neuve pour les lignes futures de ce combat après restauration — écart assumé, sans
//! conséquence pratique dans l'immense majorité des cas : par construction on ne restaure jamais
//! un combat pour lequel de nouvelles lignes "a rejoint le combat" vont encore arriver (le fichier
//! rotaté ne les rejoue pas, voir plus haut), donc au pire un ennemi homonyme non encore distingué
//! reprend au premier siège disponible plutôt qu'au bon — cas déjà documenté comme limite
//! résiduelle de `FightWorking::resolve_next_actor`.

use std::path::{Path, PathBuf};

use crate::session::FightSnapshot;

/// Nom d'app DISTINCT en test — même précaution que `watchlist::APP_NAME` (2026-09-01) : un test
/// qui écrirait dans le VRAI dossier de combats de l'utilisateur pourrait y laisser des fichiers
/// parasites, voire (pire) restaurer au prochain lancement réel un combat fictif issu d'un test.
#[cfg(not(test))]
const APP_NAME: &str = "wakfu-companion-overlay";
#[cfg(test)]
const APP_NAME: &str = "wakfu-companion-overlay-test";

/// Âge maximal toléré pour un fichier de combat persisté avant d'être considéré comme abandonné
/// (crash sans jamais recevoir `CombatEnd`, ou overlay resté éteint plus d'une journée) — au-delà,
/// le fichier est supprimé sans être restauré plutôt que de réafficher indéfiniment, au prochain
/// démarrage, un combat que l'utilisateur a très probablement quitté depuis longtemps.
const MAX_FIGHT_AGE: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

/// Dossier de stockage par défaut — voir la doc de module.
pub fn default_store_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", APP_NAME)
        .map(|dirs| dirs.data_dir().join("data"))
        .unwrap_or_else(|| PathBuf::from("data"))
}

fn fight_path(dir: &Path, fight_id: i64) -> PathBuf {
    dir.join(format!("fight-{fight_id}.json"))
}

/// Persiste (ou met à jour) l'état d'un combat encore en cours — best-effort, comme
/// `watchlist::save_to` : une écriture échouée ne doit jamais interrompre l'ingestion du log,
/// seulement priver un futur redémarrage de la restauration.
pub fn save_fight(dir: &Path, fight: &FightSnapshot) {
    debug_assert!(
        fight.ongoing,
        "un combat déjà terminé doit être supprimé (delete_fight), jamais persisté"
    );
    let Ok(json) = serde_json::to_string(fight) else {
        return; // ne devrait jamais arriver (types simples, toujours sérialisables)
    };
    if let Err(err) = std::fs::create_dir_all(dir) {
        tracing::warn!(dir = %dir.display(), %err, "impossible de créer le dossier de sauvegarde des combats");
        return;
    }
    let path = fight_path(dir, fight.fight_id);
    if let Err(err) = std::fs::write(&path, json) {
        tracing::warn!(path = %path.display(), %err, "impossible d'écrire l'état du combat (non restaurable après redémarrage)");
    }
}

/// Supprime le fichier d'un combat qui vient de se terminer (`CombatEnd`) ou qui a été purgé de la
/// mémoire (`SessionState::prune_ended_fights`) — il n'y a alors plus rien à restaurer pour ce
/// `fight_id`. Silencieux si le fichier n'existe déjà plus.
pub fn delete_fight(dir: &Path, fight_id: i64) {
    let path = fight_path(dir, fight_id);
    if let Err(err) = std::fs::remove_file(&path) {
        if err.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(path = %path.display(), %err, "impossible de supprimer l'état de combat persisté");
        }
    }
}

/// Recharge tous les combats encore persistés — appelé une seule fois, à la construction de
/// `Engine` (voir `Engine::with_stores`), avant tout premier lot ingéré. Best-effort : un fichier
/// absent, corrompu, déjà terminé (ne devrait jamais arriver, voir `save_fight`) ou trop ancien
/// (voir `MAX_FIGHT_AGE`) est simplement ignoré — et nettoyé du disque au passage pour ne pas le
/// retenter à chaque démarrage — jamais une erreur bloquante.
pub fn load_ongoing_fights(dir: &Path) -> Vec<FightSnapshot> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new(); // dossier absent : premier lancement, ou rien à restaurer
    };
    let mut fights = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let is_stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > MAX_FIGHT_AGE);
        if is_stale {
            let _ = std::fs::remove_file(&path);
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        match serde_json::from_str::<FightSnapshot>(&content) {
            Ok(fight) if fight.ongoing => fights.push(fight),
            // Un fichier de combat déjà terminé n'a rien à faire là (aurait dû être supprimé à
            // `CombatEnd`, voir `save_fight`) — nettoyage défensif plutôt qu'une restauration
            // erronée d'un combat déjà résolu.
            Ok(_) => {
                let _ = std::fs::remove_file(&path);
            }
            Err(_) => {} // fichier corrompu : ignoré, jamais fatal
        }
    }
    fights
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roster::Gender;
    use crate::session::FighterDamage;
    use std::sync::atomic::{AtomicU32, Ordering};

    static NEXT_TEST_ID: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "wakfu-overlay-fight-store-test-{}-{}",
            std::process::id(),
            NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn fighter(name: &str, is_ally: bool, damage: i64) -> FighterDamage {
        FighterDamage {
            name: name.to_string(),
            is_ally,
            total_damage: damage,
            total_heal: 0,
            class_name: None,
            gender: Gender::M,
            xp_gained: 0,
            spells: std::collections::HashMap::new(),
            is_ko: false,
            last_turn_casts: Vec::new(),
            last_turn: 0,
        }
    }

    fn ongoing_fight(fight_id: i64) -> FightSnapshot {
        FightSnapshot {
            fight_id,
            ongoing: true,
            result: None,
            fighters: vec![fighter("Oumbra", true, 120), fighter("Bwork", false, 0)],
            // Valeur non nulle délibérée (pas `0`) : un round-trip save/load qui perdrait
            // silencieusement ce champ (regression du correctif du 2026-09-04, voir sa doc) doit
            // faire échouer le test ci-dessous, pas passer par coïncidence avec le repli
            // `#[serde(default)]`.
            started_at_ms: 1_757_000_000_000,
            last_ally_caster: None,
        }
    }

    #[test]
    fn save_puis_load_restitue_le_combat_a_lidentique() {
        let dir = temp_dir();
        let fight = ongoing_fight(1);
        save_fight(&dir, &fight);

        let restored = load_ongoing_fights(&dir);
        assert_eq!(restored, vec![fight]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_fight_retire_le_fichier_et_plus_rien_nest_restaure() {
        let dir = temp_dir();
        save_fight(&dir, &ongoing_fight(2));
        delete_fight(&dir, 2);

        assert!(load_ongoing_fights(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_fight_sur_un_fichier_absent_ne_panique_pas() {
        let dir = temp_dir();
        delete_fight(&dir, 999); // dossier même pas créé
    }

    #[test]
    fn dossier_absent_renvoie_une_liste_vide() {
        let dir = temp_dir(); // jamais créé
        assert!(load_ongoing_fights(&dir).is_empty());
    }

    #[test]
    fn plusieurs_combats_en_cours_sont_tous_restaures() {
        let dir = temp_dir();
        save_fight(&dir, &ongoing_fight(1));
        save_fight(&dir, &ongoing_fight(2));

        let mut restored = load_ongoing_fights(&dir);
        restored.sort_by_key(|f| f.fight_id);
        assert_eq!(restored.len(), 2);
        assert_eq!(restored[0].fight_id, 1);
        assert_eq!(restored[1].fight_id, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn un_fichier_corrompu_est_ignore_sans_faire_echouer_les_autres() {
        let dir = temp_dir();
        save_fight(&dir, &ongoing_fight(1));
        std::fs::write(dir.join("fight-2.json"), "{ceci n'est pas du json valide").unwrap();

        let restored = load_ongoing_fights(&dir);
        assert_eq!(restored, vec![ongoing_fight(1)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn un_fichier_de_combat_deja_termine_est_ignore_et_nettoye() {
        let dir = temp_dir();
        let mut ended = ongoing_fight(1);
        ended.ongoing = false;
        ended.result = Some(crate::model::FightResult::Won);
        // Écrit directement (save_fight refuse un combat terminé via debug_assert) : simule un
        // fichier resté sur disque malgré tout (bug hypothétique ailleurs).
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(fight_path(&dir, 1), serde_json::to_string(&ended).unwrap()).unwrap();

        assert!(load_ongoing_fights(&dir).is_empty());
        assert!(
            !fight_path(&dir, 1).exists(),
            "le fichier obsolète doit être nettoyé"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
