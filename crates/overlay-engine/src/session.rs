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
//!
//! **Combats multiples simultanés** (2026-09-01, retour utilisateur multi-compte en test réel) :
//! `wakfu.log` est **partagé et entrelacé** par toutes les instances du client lancées sous le même
//! compte Windows (contrairement à Dofus, qui écrit un fichier par instance — vérifié sur le
//! disque) ; deux personnages en combat simultané produisent donc deux `fight_id` dont les lignes
//! s'entremêlent dans le même flux. `SessionState` suit chaque combat indépendamment
//! (`fights: HashMap<i64, FightWorking>`), et `overlay-ui` retrouve celui d'un personnage donné via
//! `SessionSnapshot::fight_for_character` (une fenêtre overlay par fenêtre de jeu, chacune montrant
//! le combat de SON personnage).

use std::collections::HashMap;

use crate::class_breed::class_for_breed;
use crate::model::{FightResult, LogEntry};
use crate::roster::{normalize_wakfu_name, Gender, RosterIndex};
use crate::watchlist::{WatchlistEntry, WatchlistState};

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

/// Combats gardés en mémoire au maximum (voir doc de module, multi-compte) — purge du plus ancien
/// combat **terminé** (jamais un combat `ongoing`) quand dépassé. Borne la mémoire sur une longue
/// session sans jamais perdre un combat actif, même avec plusieurs personnages en parallèle.
const MAX_TRACKED_FIGHTS: usize = 20;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionSnapshot {
    pub totals: SessionTotals,
    /// Tous les combats suivis cette session (en cours ou terminés, voir `MAX_TRACKED_FIGHTS`),
    /// triés par `fight_id` croissant — proxy simple d'ordre chronologique. En multi-compte,
    /// plusieurs entrées peuvent être `ongoing` simultanément (un `fight_id` par personnage en
    /// combat) : ne JAMAIS supposer qu'un seul combat est actif à la fois, voir
    /// `fight_for_character`.
    pub fights: Vec<FightSnapshot>,
    /// Plus récent en dernier.
    pub recent_loot: Vec<LootItem>,
}

impl SessionSnapshot {
    /// Combat auquel participe ce personnage, en tant qu'ALLIÉ — `overlay-ui` s'en sert pour
    /// choisir quoi afficher dans la fenêtre overlay ancrée sur SA fenêtre de jeu (le nom de
    /// personnage vient du titre de fenêtre, `"<Personnage> - WAKFU"`). Normalise via
    /// `normalize_wakfu_name` (accents/casse/apostrophes), même règle que le roster. Préfère un
    /// combat `ongoing` à un combat déjà terminé si jamais plusieurs correspondaient (ne devrait
    /// pas arriver en pratique — un personnage n'est que dans un seul combat à la fois) ; `None` si
    /// ce personnage n'a encore rejoint aucun combat cette session.
    pub fn fight_for_character(&self, character_name: &str) -> Option<&FightSnapshot> {
        let key = normalize_wakfu_name(character_name);
        let matches = || {
            self.fights.iter().filter(|fight| {
                fight
                    .fighters
                    .iter()
                    .any(|f| f.is_ally && normalize_wakfu_name(&f.name) == key)
            })
        };
        matches()
            .find(|fight| fight.ongoing)
            .or_else(|| matches().next_back())
    }
}

/// État de travail d'un combat en cours de suivi — la partie mutable (`fighter_index`) reste
/// interne à `SessionState`, jamais exposée : `snapshot()` n'en extrait que le `FightSnapshot`
/// immuable.
#[derive(Debug)]
struct FightWorking {
    snapshot: FightSnapshot,
    /// Index par nom dans `snapshot.fighters` — propre à CE combat (plus de notion de « combat
    /// courant » à vider/recréer, chaque `fight_id` a son propre index depuis sa création).
    fighter_index: HashMap<String, usize>,
}

/// Accumulateur mutable — voir [`SessionSnapshot`] pour la vue immuable qu'il produit.
#[derive(Debug, Default)]
struct SessionState {
    totals: SessionTotals,
    fights: HashMap<i64, FightWorking>,
    recent_loot: Vec<LootItem>,
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
                self.upsert_fighter(*fight_id, name, is_ally, class_name, gender);
            }
            LogEntry::Damage {
                fight_id: Some(fight_id),
                attacker,
                amount,
                ..
            } => {
                self.fighter_mut(*fight_id, attacker).total_damage += amount;
            }
            LogEntry::Heal {
                fight_id: Some(fight_id),
                attacker,
                amount,
                ..
            } => {
                self.fighter_mut(*fight_id, attacker).total_heal += amount;
            }
            LogEntry::CombatEnd {
                fight_id, result, ..
            } => {
                if let Some(fight) = self.fights.get_mut(fight_id) {
                    fight.snapshot.ongoing = false;
                    fight.snapshot.result = Some(*result);
                }
                match result {
                    FightResult::Won => self.totals.fights_won += 1,
                    FightResult::Lost => self.totals.fights_lost += 1,
                }
                self.prune_ended_fights();
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

    fn ensure_fight(&mut self, fight_id: i64) {
        self.fights.entry(fight_id).or_insert_with(|| FightWorking {
            snapshot: FightSnapshot {
                fight_id,
                ongoing: true,
                result: None,
                fighters: Vec::new(),
            },
            fighter_index: HashMap::new(),
        });
    }

    fn upsert_fighter(
        &mut self,
        fight_id: i64,
        name: &str,
        is_ally: bool,
        class_name: Option<String>,
        gender: Gender,
    ) {
        let Some(fight) = self.fights.get_mut(&fight_id) else {
            return;
        };
        if fight.fighter_index.contains_key(name) {
            return; // déjà rejoint (multi-compte : un même nom ne rejoint qu'une fois un combat donné)
        }
        fight
            .fighter_index
            .insert(name.to_string(), fight.snapshot.fighters.len());
        fight.snapshot.fighters.push(FighterDamage {
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
    /// combat en cours au moment de la connexion peut en avoir manqué le début), et le combat
    /// lui-même s'il n'a jamais été vu non plus (même raison). Pas de `breed` disponible ici :
    /// aucune classe, comme un allié pas encore classifié (voir doc de
    /// `FighterDamage::class_name`).
    fn fighter_mut(&mut self, fight_id: i64, name: &str) -> &mut FighterDamage {
        self.ensure_fight(fight_id);
        self.upsert_fighter(fight_id, name, true, None, Gender::M); // no-op si déjà présent
        let fight = self
            .fights
            .get_mut(&fight_id)
            .expect("ensure_fight vient de garantir sa présence");
        let idx = fight.fighter_index[name];
        &mut fight.snapshot.fighters[idx]
    }

    /// Purge le(s) plus ancien(s) combat(s) **terminé(s)** (jamais `ongoing`) au-delà de
    /// `MAX_TRACKED_FIGHTS` — voir la constante. `fight_id` croissant comme proxy d'ancienneté.
    fn prune_ended_fights(&mut self) {
        while self.fights.len() > MAX_TRACKED_FIGHTS {
            let oldest_ended = self
                .fights
                .iter()
                .filter(|(_, f)| !f.snapshot.ongoing)
                .map(|(id, _)| *id)
                .min();
            match oldest_ended {
                Some(id) => {
                    self.fights.remove(&id);
                }
                None => break, // plus rien à purger sans toucher un combat en cours
            }
        }
    }

    fn snapshot(&self) -> SessionSnapshot {
        let mut fights: Vec<FightSnapshot> =
            self.fights.values().map(|f| f.snapshot.clone()).collect();
        fights.sort_by_key(|f| f.fight_id);
        SessionSnapshot {
            totals: self.totals,
            fights,
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
    roster: Option<RosterIndex>,
    /// Suivi (watchlist, §9) — même raison qu'au-dessus : un compteur doit survivre à un
    /// rattrapage, PORTÉ PAR `Engine` et jamais recréé avec `SessionState`. Voir `watchlist.rs`
    /// pour la frontière définitions (compte)/compteurs (local à l'overlay).
    watchlist: WatchlistState,
}

impl Engine {
    pub fn new() -> Result<Self, crate::quickjs_engine::EngineError> {
        Ok(Self {
            parser: crate::quickjs_engine::LogParserEngine::new()?,
            state: SessionState::default(),
            in_initial_sweep: false,
            roster: None,
            watchlist: WatchlistState::new(crate::watchlist::default_store_path()),
        })
    }

    /// Remplace le roster utilisé pour classer les alliés (voir `resolve_ally_class`) — appelé
    /// par l'hôte (`overlay-ui`) une fois l'auth/le fetch `GET /api/v1/settings` résolus, et à
    /// chaque nouveau fetch (roster modifié sur le compte). `None` = pas de roster connu (mode
    /// invité, ou pas encore récupéré) : la classification retombe entièrement sur `breed`. Ne
    /// touche PAS aux combats déjà en cours — un allié déjà rejoint garde la classe qu'il avait à
    /// sa jonction, comme le web (le roster n'est consulté qu'à `FighterJoined`).
    pub fn set_roster(&mut self, roster: Option<RosterIndex>) {
        self.roster = roster;
    }

    /// Remplace la liste des entrées suivies par celle renvoyée par le compte (voir
    /// `watchlist_from_settings_json`), en conservant les compteurs locaux déjà en cours pour
    /// toute entrée déjà connue (voir `WatchlistState::merge_config`). Appelé par l'hôte au même
    /// moment que `set_roster` — même source `GET /api/v1/settings`.
    pub fn set_watchlist_entries(&mut self, entries: Vec<WatchlistEntry>) {
        self.watchlist.merge_config(entries);
    }

    pub fn watchlist_entries(&self) -> &[WatchlistEntry] {
        self.watchlist.entries()
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
            // état de session — c'est la sémantique `resetSessionState()` décrite au plan. La
            // watchlist n'est PAS réinitialisée ici (voir son champ ci-dessus) : ses compteurs
            // sont un suivi persistant, pas un état de combat.
            self.parser.reset()?;
            self.state = SessionState::default();
        }
        self.in_initial_sweep = batch.is_initial_load;

        let entries = self.parser.parse_lines(&batch.lines)?;
        for entry in &entries {
            self.state.apply(entry, self.roster.as_ref());
            // Miroir du gating `currentBatchIsInitialLoad` de `registerLoot`/`registerDefeat` côté
            // web (voir `watchlist.rs`) : le contenu déjà présent dans le fichier au premier
            // chargement ne doit pas regonfler un compteur qui persiste d'une session à l'autre.
            if !batch.is_initial_load {
                self.watchlist.apply(entry);
            }
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

    fn fighter_joined(
        fight_id: i64,
        name: &str,
        breed: i64,
        is_controlled_by_ai: bool,
    ) -> LogEntry {
        LogEntry::FighterJoined {
            time: "12:00:00,000".to_string(),
            fight_id,
            name: name.to_string(),
            breed,
            fighter_id: 1,
            is_controlled_by_ai,
            summoned_by: None,
        }
    }

    fn damage(fight_id: i64, attacker: &str, amount: i64) -> LogEntry {
        LogEntry::Damage {
            time: "12:00:01,000".to_string(),
            target: "cible".to_string(),
            attacker: attacker.to_string(),
            spell: "sort".to_string(),
            element: crate::model::DamageElement::Neutre,
            amount,
            fight_id: Some(fight_id),
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
        state.apply(&fighter_joined(1, "Oumbra", 1, false), Some(&roster));

        let fighter = &state.fights[&1].snapshot.fighters[0];
        assert_eq!(fighter.class_name.as_deref(), Some("cra"));
        assert_eq!(fighter.gender, Gender::F);
    }

    #[test]
    fn repli_sur_breed_si_absent_du_roster() {
        let mut state = SessionState::default();
        let roster = roster_with("QuelquUnDAutre", "sram", Gender::F);
        state.apply(&fighter_joined(1, "Oumbra", 8, false), Some(&roster)); // breed 8 = iop

        let fighter = &state.fights[&1].snapshot.fighters[0];
        assert_eq!(fighter.class_name.as_deref(), Some("iop"));
        assert_eq!(fighter.gender, Gender::M); // repli par défaut, pas d'info de sexe via breed
    }

    #[test]
    fn aucune_classe_sans_roster_ni_breed_connu() {
        let mut state = SessionState::default();
        state.apply(&fighter_joined(1, "Oumbra", 0, false), None); // breed 0 : inconnu

        let fighter = &state.fights[&1].snapshot.fighters[0];
        assert_eq!(fighter.class_name, None);
    }

    #[test]
    fn un_ennemi_na_jamais_de_classe_meme_avec_un_breed_valide() {
        let mut state = SessionState::default();
        let roster = roster_with("Monstre", "iop", Gender::M); // ne doit jamais s'appliquer
        state.apply(&fighter_joined(1, "Monstre", 1, true), Some(&roster)); // breed 1 = feca, is_controlled_by_ai=true

        let fighter = &state.fights[&1].snapshot.fighters[0];
        assert!(!fighter.is_ally);
        assert_eq!(fighter.class_name, None);
    }

    /// Cas multi-compte réel (2026-09-01) : `wakfu.log` entrelace les lignes de deux personnages
    /// en combat simultané — deux `fight_id` distincts ne doivent JAMAIS s'écraser l'un l'autre.
    #[test]
    fn deux_combats_simultanes_entrelaces_restent_independants() {
        let mut state = SessionState::default();
        // Entrelacement délibéré : A rejoint, B rejoint, dégâts A, dégâts B, dégâts A...
        state.apply(&fighter_joined(1, "PersoA", 9, false), None); // fight 1, cra
        state.apply(&fighter_joined(2, "PersoB", 4, false), None); // fight 2, sram
        state.apply(&damage(1, "PersoA", 100), None);
        state.apply(&damage(2, "PersoB", 50), None);
        state.apply(&damage(1, "PersoA", 25), None);

        let snapshot = state.snapshot();
        assert_eq!(snapshot.fights.len(), 2);
        let fight1 = snapshot.fights.iter().find(|f| f.fight_id == 1).unwrap();
        let fight2 = snapshot.fights.iter().find(|f| f.fight_id == 2).unwrap();
        assert_eq!(fight1.fighters[0].total_damage, 125);
        assert_eq!(fight2.fighters[0].total_damage, 50);
    }

    #[test]
    fn fight_for_character_retrouve_le_bon_combat_par_personnage() {
        let mut state = SessionState::default();
        state.apply(&fighter_joined(1, "Zoroark Shiny", 9, false), None);
        state.apply(&fighter_joined(2, "Canis Furiosus", 4, false), None);

        let snapshot = state.snapshot();
        assert_eq!(
            snapshot
                .fight_for_character("Zoroark Shiny")
                .map(|f| f.fight_id),
            Some(1)
        );
        assert_eq!(
            snapshot
                .fight_for_character("Canis Furiosus")
                .map(|f| f.fight_id),
            Some(2)
        );
        // Insensible casse/accents, comme le roster.
        assert_eq!(
            snapshot
                .fight_for_character("zoroark shiny")
                .map(|f| f.fight_id),
            Some(1)
        );
    }

    #[test]
    fn fight_for_character_absent_renvoie_none() {
        let mut state = SessionState::default();
        state.apply(&fighter_joined(1, "Zoroark Shiny", 9, false), None);

        let snapshot = state.snapshot();
        assert_eq!(snapshot.fight_for_character("Personne Ici"), None);
    }

    #[test]
    fn purge_ne_touche_jamais_un_combat_en_cours() {
        let mut state = SessionState::default();
        // MAX_TRACKED_FIGHTS + quelques combats, tous terminés sauf le tout premier (`ongoing`).
        state.apply(&fighter_joined(0, "EnCours", 9, false), None);
        for fight_id in 1..=(MAX_TRACKED_FIGHTS as i64 + 5) {
            state.apply(&fighter_joined(fight_id, "Autre", 9, false), None);
            state.apply(
                &LogEntry::CombatEnd {
                    time: "12:00:02,000".to_string(),
                    fight_id,
                    result: FightResult::Won,
                },
                None,
            );
        }

        assert!(state.fights.len() <= MAX_TRACKED_FIGHTS);
        assert!(
            state.fights.contains_key(&0),
            "le combat encore en cours ne doit jamais être purgé"
        );
        assert!(state.fights[&0].snapshot.ongoing);
    }
}
