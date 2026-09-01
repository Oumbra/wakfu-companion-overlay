//! Agrégation `LogEntry` → `SessionSnapshot`, en Rust, **pas** vendue depuis `StatsStoreService`.
//!
//! Volontairement minimal et documenté comme tel : `StatsStoreService` (2 755 l., couplé Angular,
//! extraction encore une décision ouverte — docs/plan-architecture.md §14 point 3) porte des
//! heuristiques bien plus fines (classification allié/ennemi par héritage d'invocation via
//! `summonedBy`, regroupement de runs de donjon, rapprochement kamas↔achat HDV…). Ce module ne
//! couvre que ce qui est directement lisible dans le flux d'événements — suffisant pour les deux
//! premiers panneaux de L2 (dégâts du combat en cours, récap de session), pas pour l'historique
//! long terme ni la synchro serveur (§7, qui exigera la parité stricte que `StatsStoreService`
//! seul peut garantir).
//!
//! **Simplification assumée** : la classification allié/ennemi utilise directement
//! `FighterJoinedEntry::is_controlled_by_ai`, sans suivre `summonedBy` — une invocation alliée
//! (contrôlée par l'IA, `summonedBy = Some(joueur)`) sera donc affichée comme ennemie ici. Le vrai
//! comportement (héritage du camp de l'invocateur) est documenté dans le TS vendu
//! (`FighterJoinedEntry`) mais pas reproduit : ce serait dupliquer une heuristique de
//! `StatsStoreService`, contraire à la raison d'être du choix QuickJS (§2 du plan).

use std::collections::HashMap;

use crate::class_breed::class_for_breed;
use crate::model::{FightResult, LogEntry};
use crate::roster::{Gender, RosterIndex};

#[derive(Debug, Clone, PartialEq)]
pub struct FighterDamage {
    pub name: String,
    pub is_ally: bool,
    pub total_damage: i64,
    pub total_heal: i64,
    /// Classe de l'allié — roster déclaré par l'utilisateur prioritaire, sinon `breed` du combat
    /// (voir `resolve_ally_class`), sinon `None` (allié pas encore classifié, ou ennemi — les
    /// ennemis n'ont jamais de classe). `None` signifie « pas de portrait à afficher », jamais un
    /// repli inventé.
    pub class_name: Option<String>,
    /// Sexe de l'icône — connu seulement via le roster ; repli `M` sinon (même défaut que le web,
    /// `EntityClassifierService.getGender`, la détection par sort ne donne aucune info de sexe).
    pub gender: Gender,
}

/// Cascade de classe/sexe d'un allié CONFIRMÉ (`is_controlled_by_ai == false`) — miroir de
/// `EntityClassifierService.getDetectedClass`/`getGender` restreint à ce qu'`overlay-engine` sait
/// dériver seul (roster déclaré, sinon `breed` déterministe de ce combat) : roster prioritaire,
/// sinon `breed`, sinon aucune classe. Ne jamais appeler pour un ennemi — `breed` n'y est pas
/// déterministe (voir `class_breed.rs`).
fn resolve_ally_class(
    name: &str,
    breed: i64,
    roster: Option<&RosterIndex>,
) -> (Option<String>, Gender) {
    if let Some(character) = roster.and_then(|r| r.find(name)) {
        return (Some(character.class_name.clone()), character.gender);
    }
    (class_for_breed(breed).map(str::to_string), Gender::M)
}

#[derive(Debug, Clone, PartialEq)]
pub struct FightSnapshot {
    pub fight_id: i64,
    pub ongoing: bool,
    /// `None` tant que le combat est en cours.
    pub result: Option<FightResult>,
    /// Dans l'ordre d'arrivée des `FighterJoinedEntry` — stable pour l'affichage, pas trié par
    /// dégâts (l'UI trie elle-même si besoin, voir §9 du plan).
    pub fighters: Vec<FighterDamage>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionTotals {
    pub kamas_gained: i64,
    pub kamas_lost: i64,
    pub xp_gained: i64,
    pub loot_count: i64,
    pub fights_won: i64,
    pub fights_lost: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LootItem {
    pub time: String,
    pub item: String,
    pub quantity: i64,
}

/// Nombre d'objets ramassés gardés en mémoire pour l'affichage (§9 : « Alertes de drop » — ici
/// juste un historique court, pas encore les alertes elles-mêmes).
const RECENT_LOOT_CAPACITY: usize = 20;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionSnapshot {
    pub totals: SessionTotals,
    /// Combat en cours, ou le dernier terminé (reste affiché jusqu'au suivant) — `None` avant le
    /// premier combat de la session.
    pub current_fight: Option<FightSnapshot>,
    /// Plus récent en dernier.
    pub recent_loot: Vec<LootItem>,
}

/// Accumulateur mutable — voir [`SessionSnapshot`] pour la vue immuable qu'il produit.
#[derive(Debug, Default)]
struct SessionState {
    totals: SessionTotals,
    current_fight: Option<FightSnapshot>,
    recent_loot: Vec<LootItem>,
    /// Index par nom dans `current_fight.fighters`, reconstruit à chaque changement de combat —
    /// évite un parcours linéaire à chaque `DamageEntry`/`HealEntry`.
    fighter_index: HashMap<String, usize>,
}

impl SessionState {
    fn apply(&mut self, entry: &LogEntry, roster: Option<&RosterIndex>) {
        match entry {
            LogEntry::FighterJoined {
                fight_id,
                name,
                breed,
                is_controlled_by_ai,
                ..
            } => {
                self.ensure_fight(*fight_id);
                let is_ally = !is_controlled_by_ai;
                let (class_name, gender) = if is_ally {
                    resolve_ally_class(name, *breed, roster)
                } else {
                    (None, Gender::M) // un ennemi n'a jamais de classe (breed pas déterministe ici)
                };
                self.upsert_fighter(name, is_ally, class_name, gender);
            }
            LogEntry::Damage {
                fight_id: Some(fight_id),
                attacker,
                amount,
                ..
            } => {
                if self.current_fight_id() == Some(*fight_id) {
                    self.fighter_mut(attacker).total_damage += amount;
                }
            }
            LogEntry::Heal {
                fight_id: Some(fight_id),
                attacker,
                amount,
                ..
            } => {
                if self.current_fight_id() == Some(*fight_id) {
                    self.fighter_mut(attacker).total_heal += amount;
                }
            }
            LogEntry::CombatEnd {
                fight_id, result, ..
            } => {
                if self.current_fight_id() == Some(*fight_id) {
                    if let Some(fight) = &mut self.current_fight {
                        fight.ongoing = false;
                        fight.result = Some(*result);
                    }
                    match result {
                        FightResult::Won => self.totals.fights_won += 1,
                        FightResult::Lost => self.totals.fights_lost += 1,
                    }
                }
            }
            LogEntry::KamaGain { amount, .. } => self.totals.kamas_gained += amount,
            LogEntry::KamaLoss { amount, .. } => self.totals.kamas_lost += amount,
            LogEntry::XpGain { amount, .. } => self.totals.xp_gained += amount,
            LogEntry::Loot {
                item,
                quantity,
                time,
                ..
            } => {
                self.totals.loot_count += quantity;
                self.recent_loot.push(LootItem {
                    time: time.clone(),
                    item: item.clone(),
                    quantity: *quantity,
                });
                if self.recent_loot.len() > RECENT_LOOT_CAPACITY {
                    self.recent_loot.remove(0);
                }
            }
            // Hors périmètre de ce premier slice (voir le commentaire de module) : chat,
            // spell-cast, armor, enemy-defeated/fled, combat-defeat-marker, combat-start (ne
            // porte pas de fightId, voir le TS vendu), challenge-result, market-occupation,
            // log-date-anchor, trade-completed, et les variantes sans fightId (kamas/loot hors
            // combat, dégâts non résolus).
            _ => {}
        }
    }

    fn current_fight_id(&self) -> Option<i64> {
        self.current_fight.as_ref().map(|f| f.fight_id)
    }

    fn ensure_fight(&mut self, fight_id: i64) {
        if self.current_fight_id() != Some(fight_id) {
            self.current_fight = Some(FightSnapshot {
                fight_id,
                ongoing: true,
                result: None,
                fighters: Vec::new(),
            });
            self.fighter_index.clear();
        }
    }

    fn upsert_fighter(
        &mut self,
        name: &str,
        is_ally: bool,
        class_name: Option<String>,
        gender: Gender,
    ) {
        let Some(fight) = &mut self.current_fight else {
            return;
        };
        if self.fighter_index.contains_key(name) {
            return; // déjà rejoint (multi-compte : un même nom ne rejoint qu'une fois un combat donné)
        }
        self.fighter_index
            .insert(name.to_string(), fight.fighters.len());
        fight.fighters.push(FighterDamage {
            name: name.to_string(),
            is_ally,
            total_damage: 0,
            total_heal: 0,
            class_name,
            gender,
        });
    }

    /// Ajoute défensivement le combattant s'il n'a jamais été vu via `FighterJoined` (ne devrait
    /// pas arriver — voir la doc de `FighterJoinedEntry`, émis pour chaque combattant — mais un
    /// combat en cours au moment de la connexion peut en avoir manqué le début). Pas de `breed`
    /// disponible ici : aucune classe, comme un allié pas encore classifié (voir doc de
    /// `FighterDamage::class_name`).
    fn fighter_mut(&mut self, name: &str) -> &mut FighterDamage {
        if !self.fighter_index.contains_key(name) {
            self.upsert_fighter(name, true, None, Gender::M);
        }
        let idx = self.fighter_index[name];
        &mut self
            .current_fight
            .as_mut()
            .expect("fighter_index non vide implique current_fight présent")
            .fighters[idx]
    }

    fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            totals: self.totals,
            current_fight: self.current_fight.clone(),
            recent_loot: self.recent_loot.clone(),
        }
    }
}

/// Frontière métier complète (§2 du plan) : `LineBatch` → `LogEntry` (QuickJS) → `SessionSnapshot`
/// (agrégation Rust). Un thread dédié le possède (§3) ; jamais partagé entre threads directement,
/// voir `overlay-app` pour le câblage réel (`ArcSwap<SessionSnapshot>` publié vers l'UI).
pub struct Engine {
    parser: crate::quickjs_engine::LogParserEngine,
    state: SessionState,
    /// `true` tant que le dernier lot ingéré faisait partie d'un rattrapage (voir
    /// `LineBatch::is_initial_load`) — sert à ne réinitialiser le parser et l'état de session
    /// qu'au tout **premier** lot d'un nouveau rattrapage, jamais entre deux lots d'un même
    /// rattrapage qui continuent la même séquence logique (un combat, ou même une seule ligne
    /// multi-lignes, peut s'étaler sur plusieurs `LineBatch` à cause de `MAX_BATCH_LINES`).
    in_initial_sweep: bool,
    /// Roster déclaré par l'utilisateur (compte, lot L4) — délibérément PORTÉ PAR `Engine`, pas
    /// par `SessionState` : ce dernier est entièrement recréé à chaque nouveau rattrapage
    /// (`SessionState::default()` ci-dessous), ce qui effacerait le roster à chaque
    /// reconnexion/rotation de `wakfu.log` si on le stockait là.
    roster: Option<crate::roster::RosterIndex>,
}

impl Engine {
    pub fn new() -> Result<Self, crate::quickjs_engine::EngineError> {
        Ok(Self {
            parser: crate::quickjs_engine::LogParserEngine::new()?,
            state: SessionState::default(),
            in_initial_sweep: false,
            roster: None,
        })
    }

    /// Remplace le roster utilisé pour classer les alliés (voir `resolve_ally_class`) — appelé
    /// par l'hôte (`overlay-ui`) une fois l'auth/le fetch `GET /api/v1/settings` résolus, et à
    /// chaque nouveau fetch (roster modifié sur le compte). `None` = pas de roster connu (mode
    /// invité, ou pas encore récupéré) : la classification retombe entièrement sur `breed`. Ne
    /// touche PAS aux combats déjà en cours — un allié déjà rejoint garde la classe qu'il avait à
    /// sa jonction, comme le web (le roster n'est consulté qu'à `FighterJoined`).
    pub fn set_roster(&mut self, roster: Option<crate::roster::RosterIndex>) {
        self.roster = roster;
    }

    /// Ingère un lot déjà lu par `overlay-ingest`, met à jour l'état de session en place, et
    /// renvoie les `LogEntry` produits (utile pour un affichage brut — chat, journal — que ce
    /// premier slice de L2 n'agrège pas encore, voir le module `session`).
    pub fn ingest_batch(
        &mut self,
        batch: &overlay_ingest::LineBatch,
    ) -> Result<Vec<LogEntry>, crate::quickjs_engine::EngineError> {
        if batch.is_initial_load && !self.in_initial_sweep {
            // Nouveau rattrapage (reconnexion ou rotation, §5.3) : on repart de zéro, parser ET
            // état de session — c'est la sémantique `resetSessionState()` décrite au plan.
            self.parser.reset()?;
            self.state = SessionState::default();
        }
        self.in_initial_sweep = batch.is_initial_load;

        let entries = self.parser.parse_lines(&batch.lines)?;
        for entry in &entries {
            self.state.apply(entry, self.roster.as_ref());
        }
        Ok(entries)
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        self.state.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fighter_joined(name: &str, breed: i64, is_controlled_by_ai: bool) -> LogEntry {
        LogEntry::FighterJoined {
            time: "12:00:00,000".to_string(),
            fight_id: 1,
            name: name.to_string(),
            breed,
            fighter_id: 1,
            is_controlled_by_ai,
            summoned_by: None,
        }
    }

    fn roster_with(name: &str, class_name: &str, gender: Gender) -> RosterIndex {
        RosterIndex::from_settings_json(&serde_json::json!({
            "roster": [{
                "characters": [{ "name": name, "className": class_name, "gender": if gender == Gender::F { "f" } else { "m" } }],
            }],
        }))
    }

    #[test]
    fn roster_prioritaire_sur_breed_pour_un_allie_confirme() {
        let mut state = SessionState::default();
        let roster = roster_with("Oumbra", "cra", Gender::F);
        // breed=1 (feca) volontairement en désaccord avec le roster (iop) : le roster doit gagner.
        state.apply(&fighter_joined("Oumbra", 1, false), Some(&roster));

        let fighter = &state.current_fight.unwrap().fighters[0];
        assert_eq!(fighter.class_name.as_deref(), Some("cra"));
        assert_eq!(fighter.gender, Gender::F);
    }

    #[test]
    fn repli_sur_breed_si_absent_du_roster() {
        let mut state = SessionState::default();
        let roster = roster_with("QuelquUnDAutre", "sram", Gender::F);
        state.apply(&fighter_joined("Oumbra", 8, false), Some(&roster)); // breed 8 = iop

        let fighter = &state.current_fight.unwrap().fighters[0];
        assert_eq!(fighter.class_name.as_deref(), Some("iop"));
        assert_eq!(fighter.gender, Gender::M); // repli par défaut, pas d'info de sexe via breed
    }

    #[test]
    fn aucune_classe_sans_roster_ni_breed_connu() {
        let mut state = SessionState::default();
        state.apply(&fighter_joined("Oumbra", 0, false), None); // breed 0 : inconnu

        let fighter = &state.current_fight.unwrap().fighters[0];
        assert_eq!(fighter.class_name, None);
    }

    #[test]
    fn un_ennemi_na_jamais_de_classe_meme_avec_un_breed_valide() {
        let mut state = SessionState::default();
        let roster = roster_with("Monstre", "iop", Gender::M); // ne doit jamais s'appliquer
        state.apply(&fighter_joined("Monstre", 1, true), Some(&roster)); // breed 1 = feca, is_controlled_by_ai=true

        let fighter = &state.current_fight.unwrap().fighters[0];
        assert!(!fighter.is_ally);
        assert_eq!(fighter.class_name, None);
    }
}
