//! Agrégation `LogEntry` → `SessionSnapshot`, en Rust, **pas** vendue depuis `StatsStoreService`.
//!
//! Volontairement minimal et documenté comme tel : `StatsStoreService` (2 755 l., couplé Angular,
//! extraction encore une décision ouverte — docs/plan-architecture.md §14 point 3) porte des
//! heuristiques bien plus fines (regroupement de runs de donjon, rapprochement kamas↔achat HDV…).
//! Ce module ne
//! couvre que ce qui est directement lisible dans le flux d'événements — suffisant pour les deux
//! premiers panneaux de L2 (dégâts du combat en cours, récap de session), pas pour l'historique
//! long terme ni la synchro serveur (§7, qui exigera la parité stricte que `StatsStoreService`
//! seul peut garantir).
//!
//! **`summonedBy` suivi depuis le 2026-09-02** (retour utilisateur, captures d'écran à l'appui :
//! une invocation alliée — mécanisme, totem — s'affichait à tort côté ennemis) : une
//! `FighterJoinedEntry` dont `summoned_by` est renseigné n'obtient JAMAIS de ligne dans
//! `snapshot.fighters` (ni alliée ni ennemie) — miroir du `return` anticipé de
//! `registerFighterJoin` (`stats-store.service.ts`) sur `summonNames`, voir `FightWorking::
//! summon_names`. Volontairement PAS un héritage complet du camp de l'invocateur (le web
//! lui-même n'affiche jamais l'invocation comme une ligne à part, avec sa propre classe/icône —
//! seul le filtrage compte ici) : ce serait aller au-delà de ce que `StatsStoreService` fait
//! réellement, contraire à la raison d'être du choix QuickJS (§2 du plan). `summoned_by`
//! lui-même reste résolu par le TS vendu (`log-parser.ts::parseFighterJoin`), protégé côté hôte
//! par `hostIsKnownMonsterName` (voir `quickjs_engine.rs`) contre un vrai monstre qui se révèle
//! (mimique, brèche) — jamais avalé à tort par le repli "invocation sans annonce" du parser.
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

use crate::catalog::CatalogIndex;
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

/// Un "siège" de la file d'initiative d'un combat — miroir de `InitiativeSeat`/`resolveNextActor`
/// (`stats-store.service.ts`) : mémorise l'ORDRE DE JEU réel (fixe pour toute la durée d'un combat
/// Wakfu, propriété du tour par tour) pour pouvoir distinguer PLUSIEURS combattants qui partagent
/// EXACTEMENT le même nom (pack du même monstre — retour utilisateur 2026-09-02 : « il n'y a que
/// quatre monstres qui sont toujours affichés, pas plus » sur des combats qui en affichent
/// visiblement bien plus). Le log ne relie JAMAIS une ligne de dégâts/soin à un `fighterId` précis
/// (seule la ligne de jointure `[_FL_]` le porte, voir `FighterJoinedEntry`) — voir
/// `FightWorking::resolve_next_actor` pour le mécanisme complet.
///
/// `fighter_index` pointe DIRECTEMENT vers la ligne du combattant concerné dans
/// `FightSnapshot::fighters` — plus simple que la clé de chaîne synthétique "Nom#i" du web, qui
/// n'existe là-bas que parce que `StatsStoreService` agrège AUSSI par sort/tour dans des `Map` à
/// clé texte (`attackerMap`, `healSourceMap`...), un besoin que ce module (de simples totaux par
/// combattant, pas de détail par sort) n'a pas.
#[derive(Debug)]
struct InitiativeSeat {
    fighter_index: usize,
    name: String,
    /// Nombre de tours consécutifs sautés (ce siège attendu n'a pas rejoué) sans avoir rejoué
    /// depuis — à 2, considéré définitivement hors rotation (probablement mort/parti).
    consecutive_skips: u8,
    /// `true` dès que ce siège est sorti de la rotation — ignoré par la recherche du prochain
    /// siège, mais jamais retiré du tableau (les index des autres sièges doivent rester stables).
    retired: bool,
}

/// État de travail d'un combat en cours de suivi — la partie mutable (`fighter_index`) reste
/// interne à `SessionState`, jamais exposée : `snapshot()` n'en extrait que le `FightSnapshot`
/// immuable.
#[derive(Debug)]
struct FightWorking {
    snapshot: FightSnapshot,
    /// TOUTES les lignes de `snapshot.fighters` partageant ce nom, dans l'ordre de jonction —
    /// plusieurs entrées possibles depuis que deux combattants homonymes ne sont plus fusionnés
    /// (voir `InitiativeSeat` et `resolve_next_actor`). Propre à CE combat (plus de notion de
    /// « combat courant » à vider/recréer, chaque `fight_id` a son propre index depuis sa
    /// création).
    fighter_index: HashMap<String, Vec<usize>>,
    /// Noms d'ennemis (normalisés en minuscules) déjà "résolus" — vaincus explicitement
    /// (`EnemyDefeated`) ou en fuite (`EnemyFled`) — voir le filet de rattrapage de `apply` pour
    /// `CombatEnd`. Miroir simplifié de `FightWorking.defeatedNames`/`fledNames`
    /// (`stats-store.service.ts`) : pas de comptage PAR INSTANCE comme `defeatedInstanceCounts`
    /// côté web — un nom marqué résolu l'est pour TOUTES ses instances d'un coup. Écart assumé,
    /// hors périmètre de ce portage (qui couvre l'attribution des dégâts/soins, pas le comptage de
    /// victoires par instance du panneau Suivi) : au pire, un deuxième combattant homonyme jamais
    /// explicitement vaincu ne sera pas crédité une deuxième fois par le filet de rattrapage.
    resolved_enemies: std::collections::HashSet<String>,
    /// File d'initiative de ce combat — voir `InitiativeSeat`/`resolve_next_actor`.
    initiative_seats: Vec<InitiativeSeat>,
    /// Index (dans `initiative_seats`) du prochain siège attendu à jouer.
    initiative_cursor: usize,
    /// Dernier acteur (nom brut) ayant lancé un sort — un même acteur qui enchaîne plusieurs sorts
    /// DANS LE MÊME TOUR ne doit pas faire avancer la file une deuxième fois (voir
    /// `register_fight_turn`).
    last_turn_actor: Option<String>,
    /// Dernier siège résolu pour un nom donné (voir `resolve_next_actor`) — c'est CE siège qui
    /// reçoit les dégâts/soins d'une ligne dont l'attaquant porte ce nom, jusqu'au prochain sort
    /// lancé par ce même nom (miroir de `lastResolvedSeatByName`).
    last_resolved_seat_by_name: HashMap<String, usize>,
    /// Noms (minuscules) identifiés comme une invocation de CE combat (`FighterJoinedEntry::
    /// summoned_by`) — miroir de `FightWorking.summonNames` (`stats-store.service.ts`) : jamais de
    /// ligne dans `snapshot.fighters` (voir `apply`, cas `FighterJoined`), jamais crédité en
    /// dégâts/soin (voir `fighter_mut`). Wakfu logue TOUJOURS une invocation avec
    /// `isControlledByAI: true`, quel que soit le camp réel de son invocateur — sans ce filtre, une
    /// invocation alliée (mécanisme, totem...) s'affiche à tort côté ennemis, voir la doc de module
    /// et le retour utilisateur 2026-09-02 (captures d'écran à l'appui : « Balise de Contact »,
    /// mécanisme allié, listé côté ennemis).
    summon_names: std::collections::HashSet<String>,
}

impl FightWorking {
    /// Nombre d'instances CONNUES (rejointes via `[_FL_]`) portant ce nom — miroir de
    /// `countNameInstances` (restreint aux ennemis/alliés de CE combat, notre `fighter_index` ne
    /// contient déjà qu'eux).
    fn count_name_instances(&self, name: &str) -> usize {
        self.fighter_index.get(name).map_or(0, Vec::len)
    }

    /// Miroir de `registerFightTurn` (`stats-store.service.ts`) — voir sa doc pour le
    /// raisonnement complet. Le comptage de tours (`turnSeatsSeen`/`fight.turnCount` côté web)
    /// n'est PAS porté : rien dans l'overlay n'affiche encore de numéro de tour, seule
    /// l'attribution des dégâts/soins par siège est utile ici.
    fn register_fight_turn(&mut self, actor: &str) {
        if self.last_turn_actor.as_deref() == Some(actor) {
            return; // même tour en cours (plusieurs sorts d'affilée par le même acteur).
        }
        let seat_fighter_index = self.resolve_next_actor(actor);
        self.last_turn_actor = Some(actor.to_string());
        self.last_resolved_seat_by_name
            .insert(actor.to_string(), seat_fighter_index);
    }

    /// Miroir de `resolveNextActor` (`stats-store.service.ts`) — voir sa doc détaillée côté web
    /// pour le raisonnement complet (limite résiduelle irréductible comprise : deux instances
    /// homonymes dont l'une termine son tour exactement quand l'autre commence, sans aucun autre
    /// acteur entre les deux, sont fusionnées sur le même siège — rien dans le log ne permet de
    /// distinguer ce cas d'un deuxième sort de la même instance dans son propre tour). Reproduit à
    /// l'identique sur le fond, adapté sur la forme : un siège pointe directement vers une ligne de
    /// `FightSnapshot::fighters` plutôt qu'une clé de chaîne synthétique (voir doc d'
    /// `InitiativeSeat`).
    fn resolve_next_actor(&mut self, name: &str) -> usize {
        let instance_count = self.count_name_instances(name).max(1);
        let seat_count = self.initiative_seats.len();
        let existing_seats_for_name = self
            .initiative_seats
            .iter()
            .filter(|seat| seat.name == name)
            .count();
        // Tant que ce nom n'a pas encore autant de sièges que d'instances connues, la recherche ne
        // boucle PAS par la fin de la file (voir doc de `resolveNextActor` côté web pour pourquoi :
        // impossible sinon de savoir si une occurrence plus loin dans la file est une instance déjà
        // vue qui rejoue ou une instance encore jamais vue).
        let allow_wrap = existing_seats_for_name >= instance_count;
        let max_offset = if allow_wrap {
            seat_count
        } else {
            seat_count.saturating_sub(self.initiative_cursor)
        };

        for offset in 0..max_offset {
            let idx = (self.initiative_cursor + offset) % seat_count;
            if self.initiative_seats[idx].retired || self.initiative_seats[idx].name != name {
                continue;
            }

            for skip_offset in 0..offset {
                let skipped_idx = (self.initiative_cursor + skip_offset) % seat_count;
                if self.initiative_seats[skipped_idx].retired {
                    continue;
                }
                self.initiative_seats[skipped_idx].consecutive_skips += 1;
                if self.initiative_seats[skipped_idx].consecutive_skips >= 2 {
                    self.initiative_seats[skipped_idx].retired = true;
                }
            }

            self.initiative_seats[idx].consecutive_skips = 0;
            self.initiative_cursor = (idx + 1) % seat_count;
            return self.initiative_seats[idx].fighter_index;
        }

        // Aucun siège actif existant pour ce nom (dans la plage de recherche autorisée) : associe
        // un nouveau siège à la prochaine instance CONNUE de ce nom pas encore assise. Repli sur la
        // DERNIÈRE instance connue si toutes le sont déjà (ne devrait pas arriver en pratique —
        // plus de sièges demandés pour ce nom que d'instances jamais rejointes, ex. une jointure
        // manquée en tout début de lecture) plutôt que de paniquer sur un index hors limites.
        let known = self.fighter_index.get(name).cloned().unwrap_or_default();
        let fighter_index = known
            .get(existing_seats_for_name)
            .or_else(|| known.last())
            .copied()
            .unwrap_or(0);

        self.initiative_seats.insert(
            self.initiative_cursor.min(self.initiative_seats.len()),
            InitiativeSeat {
                fighter_index,
                name: name.to_string(),
                consecutive_skips: 0,
                retired: false,
            },
        );
        // Pas de modulo ici (contrairement à la branche "trouvé" ci-dessus) : la file grandit
        // encore pendant la découverte initiale, `initiative_cursor` doit alors simplement suivre
        // la queue au fil des insertions successives.
        self.initiative_cursor += 1;
        fighter_index
    }
}

/// Accumulateur mutable — voir [`SessionSnapshot`] pour la vue immuable qu'il produit.
#[derive(Debug, Default)]
struct SessionState {
    totals: SessionTotals,
    fights: HashMap<i64, FightWorking>,
    recent_loot: Vec<LootItem>,
}

impl SessionState {
    /// Renvoie les noms d'ennemis crédités IMPLICITEMENT comme vaincus par le filet de rattrapage
    /// de `CombatEnd` ci-dessous (vide dans tous les autres cas) — l'appelant (`Engine::
    /// ingest_batch`) s'en sert pour créditer la watchlist comme s'il s'agissait d'autant de
    /// `LogEntry::EnemyDefeated` supplémentaires, seul endroit qui connaît `WatchlistState`.
    fn apply(&mut self, entry: &LogEntry, roster: Option<&RosterIndex>) -> Vec<String> {
        let mut implicitly_defeated_enemies = Vec::new();
        match entry {
            LogEntry::FighterJoined {
                fight_id,
                name,
                breed,
                is_controlled_by_ai,
                summoned_by,
                ..
            } => {
                self.ensure_fight(*fight_id);
                if summoned_by.is_some() {
                    // Voir `FightWorking::summon_names` : jamais de ligne pour une invocation,
                    // quel que soit `is_controlled_by_ai` (toujours `true` en pratique côté log,
                    // même pour une invocation alliée) — retour anticipé, comme
                    // `registerFighterJoin` côté web.
                    if let Some(fight) = self.fights.get_mut(fight_id) {
                        fight.summon_names.insert(name.to_lowercase());
                    }
                    return implicitly_defeated_enemies;
                }
                let is_ally = !is_controlled_by_ai;
                let (class_name, gender) = if is_ally {
                    resolve_ally_class(name, *breed, roster)
                } else {
                    (None, Gender::M) // un ennemi n'a jamais de classe (breed pas déterministe ici)
                };
                self.upsert_fighter(*fight_id, name, is_ally, class_name, gender);
            }
            // Établit QUI agit maintenant sous ce nom (voir `FightWorking::register_fight_turn`) —
            // c'est CE siège qui recevra les lignes de dégâts/soin de cet attaquant jusqu'au
            // prochain sort lancé par ce même nom. Miroir de `registerFightTurn`, cas
            // `'spell-cast'` de `StatsStoreService.apply()`.
            LogEntry::SpellCast {
                caster,
                fight_id: Some(fight_id),
                ..
            } => {
                if let Some(fight) = self.fights.get_mut(fight_id) {
                    fight.register_fight_turn(caster);
                }
            }
            LogEntry::Damage {
                fight_id: Some(fight_id),
                attacker,
                amount,
                ..
            } => {
                if let Some(fighter) = self.fighter_mut(*fight_id, attacker) {
                    fighter.total_damage += amount;
                }
            }
            LogEntry::Heal {
                fight_id: Some(fight_id),
                attacker,
                amount,
                ..
            } => {
                if let Some(fighter) = self.fighter_mut(*fight_id, attacker) {
                    fighter.total_heal += amount;
                }
            }
            LogEntry::EnemyDefeated {
                name,
                fight_id: Some(fight_id),
                ..
            } => {
                self.mark_resolved(*fight_id, name);
            }
            // Un combattant qui s'échappe n'a PAS été vaincu, même si le combat se termine par une
            // victoire (mimique qui se révèle puis fuit) — marqué "résolu" quand même pour que le
            // filet de rattrapage de `CombatEnd` ci-dessous ne le crédite pas à tort. Miroir de
            // `registerFightFlee` (`stats-store.service.ts`), qui n'appelle jamais `registerDefeat`.
            LogEntry::EnemyFled {
                name,
                fight_id: Some(fight_id),
                ..
            } => {
                self.mark_resolved(*fight_id, name);
            }
            LogEntry::CombatEnd {
                fight_id, result, ..
            } => {
                if let Some(fight) = self.fights.get_mut(fight_id) {
                    fight.snapshot.ongoing = false;
                    fight.snapshot.result = Some(*result);
                    // Filet de rattrapage — miroir de `finalizeFight` (`stats-store.service.ts`) :
                    // le dernier ennemi d'un combat (souvent le boss) meurt parfois EXACTEMENT en
                    // même temps que le combat se termine, sans jamais produire sa propre ligne
                    // "est KO !"/"est hors-combat !" (constaté en session, 2026-09-01 : boss "El
                    // Pochito" jamais crédité dans le Suivi malgré une victoire confirmée par le
                    // butin ramassé juste après — vérifié dans le vrai wakfu.log, aucune ligne de
                    // défaite ne précède le ramassage). Un combat GAGNÉ implique que tout ennemi
                    // ayant rejoint et jamais résolu (ni vaincu explicitement, ni en fuite) est
                    // mort en même temps que le combat.
                    if *result == FightResult::Won {
                        for fighter in &fight.snapshot.fighters {
                            if fighter.is_ally {
                                continue;
                            }
                            if fight.resolved_enemies.insert(fighter.name.to_lowercase()) {
                                implicitly_defeated_enemies.push(fighter.name.clone());
                            }
                        }
                    }
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
            // Hors périmètre de ce premier slice (voir le commentaire de module) : chat, armor,
            // combat-defeat-marker, combat-start (ne porte pas de fightId, voir le TS vendu),
            // challenge-result, market-occupation, log-date-anchor, trade-completed, et les
            // variantes sans fightId (kamas/loot hors combat, dégâts non résolus, spell-cast/
            // enemy-defeated/fled sans fightId résolu par le parser).
            _ => {}
        }
        implicitly_defeated_enemies
    }

    /// Marque un nom d'ennemi comme "résolu" pour ce combat (vaincu explicitement ou en fuite) —
    /// voir `resolved_enemies` et le filet de rattrapage de `CombatEnd` ci-dessus. No-op si le
    /// combat n'est pas suivi (jamais vu de `FighterJoined`, ex. combat déjà en cours à l'ouverture
    /// du fichier — même défense que `registerFightDefeat`/`registerFightFlee` côté web).
    fn mark_resolved(&mut self, fight_id: i64, name: &str) {
        if let Some(fight) = self.fights.get_mut(&fight_id) {
            fight.resolved_enemies.insert(name.to_lowercase());
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
            resolved_enemies: std::collections::HashSet::new(),
            initiative_seats: Vec::new(),
            initiative_cursor: 0,
            last_turn_actor: None,
            last_resolved_seat_by_name: HashMap::new(),
            summon_names: std::collections::HashSet::new(),
        });
    }

    /// Ajoute TOUJOURS une nouvelle ligne — plus de fusion des combattants homonymes (voir
    /// `InitiativeSeat` : la distinction entre plusieurs instances d'un même nom se fait
    /// maintenant à l'attribution des dégâts/soins, pas ici). Renvoie l'index de la ligne créée
    /// dans `snapshot.fighters`, utile au repli défensif de `fighter_mut`.
    fn upsert_fighter(
        &mut self,
        fight_id: i64,
        name: &str,
        is_ally: bool,
        class_name: Option<String>,
        gender: Gender,
    ) -> usize {
        let fight = self
            .fights
            .get_mut(&fight_id)
            .expect("ensure_fight doit toujours être appelé avant upsert_fighter");
        let idx = fight.snapshot.fighters.len();
        fight.snapshot.fighters.push(FighterDamage {
            name: name.to_string(),
            is_ally,
            total_damage: 0,
            total_heal: 0,
            class_name,
            gender,
        });
        fight
            .fighter_index
            .entry(name.to_string())
            .or_default()
            .push(idx);
        idx
    }

    /// Combattant à créditer pour une ligne de dégâts/soin dont l'attaquant est nommé `name` —
    /// résout via la file d'initiative (voir `InitiativeSeat`/`FightWorking::resolve_next_actor`)
    /// le SIÈGE précis à créditer quand `name` est porté par plusieurs combattants du même combat,
    /// en s'appuyant sur le dernier sort lancé par ce nom (`last_resolved_seat_by_name`, mis à jour
    /// par `register_fight_turn`, JAMAIS ici) — miroir de `StatsStoreService.apply()`, cas
    /// `'damage'`/`'heal'` (`lastResolvedSeatByName.get(entry.attacker) ?? entry.attacker`).
    ///
    /// Écart assumé avec le web pour le repli (aucun sort encore vu pour ce nom ce combat) : le
    /// web retombe sur une clé "nom brut" qui, pour un nom AMBIGU, ne correspond à AUCUNE ligne de
    /// `buildEntityDamageRows` — ces dégâts sont donc silencieusement perdus côté web. Ici, plus
    /// simple et strictement meilleur : repli sur la PREMIÈRE instance jointe de ce nom, jamais de
    /// dégâts perdus.
    ///
    /// Ajoute aussi défensivement le combattant lui-même s'il n'a jamais été vu rejoindre (ne
    /// devrait pas arriver — voir la doc de `FighterJoinedEntry`, émis pour chaque combattant —
    /// mais un combat en cours au moment de la connexion peut en avoir manqué le début) : sans
    /// classe, comme un allié pas encore classifié (voir doc de `FighterDamage::class_name`).
    ///
    /// `None` si `name` est une invocation connue de ce combat (`FightWorking::summon_names`) —
    /// miroir du filtre `summonNames` de `addDamage`/`ensurePresent` côté web : ses actions dont
    /// l'attaquant n'a PAS été réattribué à son invocateur par le parser (repli `resolveEffectTail`
    /// le plus profond, voir `log-parser.ts`) ne créditent personne, plutôt que de créer une ligne
    /// pour l'invocation elle-même (voir la doc de module, cas `FighterJoined`).
    fn fighter_mut(&mut self, fight_id: i64, name: &str) -> Option<&mut FighterDamage> {
        self.ensure_fight(fight_id);

        let known_idx = {
            let fight = self
                .fights
                .get(&fight_id)
                .expect("ensure_fight vient de garantir sa présence");
            if fight.summon_names.contains(&name.to_lowercase()) {
                return None;
            }
            fight
                .last_resolved_seat_by_name
                .get(name)
                .copied()
                .or_else(|| {
                    fight
                        .fighter_index
                        .get(name)
                        .and_then(|ids| ids.first().copied())
                })
        };
        let idx = match known_idx {
            Some(idx) => idx,
            None => self.upsert_fighter(fight_id, name, true, None, Gender::M),
        };

        let fight = self
            .fights
            .get_mut(&fight_id)
            .expect("ensure_fight vient de garantir sa présence");
        Some(&mut fight.snapshot.fighters[idx])
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
    /// `true` dès que `state` a été initialisé une première fois par `ingest_batch` — sert à NE
    /// PLUS jamais vider `state` lors d'un rattrapage ULTÉRIEUR (voir `ingest_batch`).
    ///
    /// Régression réelle (retour utilisateur 2026-09-02, vidéo à l'appui) : `wakfu.log` peut être
    /// remplacé par un fichier neuf **en cours de partie** (rotation à date fixe côté client
    /// Wakfu), et le fichier rotaté ne rejoue PAS l'historique déjà lu — confirmé par la vidéo,
    /// où le panneau Combat perd tous ses alliés/ennemis pile au moment où
    /// `overlay_ingest::tailer` logue « rotation/troncature détectée », alors que le combat était
    /// toujours en cours en jeu. `Tailer::poll` (voir sa doc) marque cette relecture avec
    /// `is_initial_load: true` — EXACTEMENT le même signal qu'un tout premier lancement contre un
    /// `wakfu.log` déjà volumineux — ce qui déclenchait avant ce champ le même
    /// `state = SessionState::default()` que pour un vrai premier rattrapage, effaçant à tort un
    /// combat encore actif. Ce champ distingue les deux cas : au tout premier rattrapage, `state`
    /// est de toute façon déjà vide (le reset est un no-op) ; à toute rotation SUIVANTE, `state`
    /// contient un vécu de session légitime (combats en cours, totaux) qui doit survivre — seul le
    /// PARSER (contexte transitoire QuickJS, resynchronisation avec la position de lecture) a
    /// besoin d'être réinitialisé, jamais `state`.
    state_initialized: bool,
    /// Roster déclaré par l'utilisateur (compte, lot L4) — délibérément PORTÉ PAR `Engine`, pas
    /// par `SessionState` : ce dernier est entièrement recréé à chaque nouveau rattrapage
    /// (`SessionState::default()` ci-dessous), ce qui effacerait le roster à chaque
    /// reconnexion/rotation de `wakfu.log` si on le stockait là.
    roster: Option<RosterIndex>,
    /// Suivi (watchlist, §9) — même raison qu'au-dessus : un compteur doit survivre à un
    /// rattrapage, PORTÉ PAR `Engine` et jamais recréé avec `SessionState`. Voir `watchlist.rs`
    /// pour la frontière définitions (compte)/compteurs (local à l'overlay).
    watchlist: WatchlistState,
    /// Alertes de décompte à 0 (voir `WatchlistAlert`) accumulées depuis le dernier
    /// `drain_watchlist_alerts` — motif « drain » (comme `drainSyncEvents()` esquissé au §2.1 du
    /// plan pour le futur moteur headless) plutôt que de changer la signature d'`ingest_batch` :
    /// l'hôte (thread dédié `overlay-ui`) les récupère à son rythme, sans coupler cette API au
    /// détail de la watchlist.
    pending_alerts: Vec<crate::watchlist::WatchlistAlert>,
}

impl Engine {
    pub fn new() -> Result<Self, crate::quickjs_engine::EngineError> {
        Self::with_watchlist_store(crate::watchlist::default_store_path())
    }

    /// Comme `new()`, avec un chemin de compteurs de suivi explicite — **PUBLIC MAIS RÉSERVÉ AUX
    /// TESTS D'INTÉGRATION** (`tests/*.rs`). `#[cfg(test)]` sur `watchlist::APP_NAME` ne protège
    /// que les tests INTERNES de ce crate (`src/watchlist.rs::tests`, compilés avec `cfg(test)`
    /// actif pour `overlay-engine` lui-même) : un test d'intégration est un crate SÉPARÉ qui
    /// dépend d'`overlay-engine` normalement, sans `cfg(test)` actif pour lui — `Engine::new()` y
    /// résout donc le VRAI chemin de production. Bug réel vécu en session (2026-09-01) : un test
    /// d'intégration a écrasé le fichier de compteurs réel de l'utilisateur (compteurs d'objets
    /// perdus) avant que ce constructeur n'existe — toujours passer par lui (avec un chemin de
    /// fichier temporaire) depuis `tests/`, jamais `Engine::new()`.
    pub fn with_watchlist_store(
        store_path: std::path::PathBuf,
    ) -> Result<Self, crate::quickjs_engine::EngineError> {
        Ok(Self {
            parser: crate::quickjs_engine::LogParserEngine::new()?,
            state: SessionState::default(),
            in_initial_sweep: false,
            state_initialized: false,
            roster: None,
            watchlist: WatchlistState::new(store_path),
            pending_alerts: Vec::new(),
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

    /// Relaie le catalogue (lot L3, `overlay_engine::catalog`) au parser QuickJS (voir
    /// `quickjs_engine.rs::LogParserEngine::set_catalog`) — protège `hostIsKnownMonsterName`
    /// contre un vrai monstre qui se révèle (mimique, brèche) confondu à tort avec une invocation
    /// (voir la doc de module). Appelé par l'hôte à chaque (re)chargement du catalogue, comme
    /// `set_roster`/`set_watchlist_entries` — n'affecte que les `FighterJoined` futurs, jamais un
    /// combat déjà résolu. Prend un `Arc` (voir sa doc côté `quickjs_engine.rs`), pas une valeur
    /// possédée : simple partage de référence avec l'`Arc<ArcSwap<CatalogIndex>>` déjà détenu par
    /// l'hôte, jamais de copie du catalogue entier.
    pub fn set_catalog(&mut self, catalog: std::sync::Arc<CatalogIndex>) {
        self.parser.set_catalog(catalog);
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

    /// Vide et renvoie les alertes de décompte accumulées depuis le dernier appel (voir
    /// `pending_alerts`) — à appeler par l'hôte après chaque `ingest_batch` pour déclencher
    /// toast/son (§9 du plan, « Alertes de drop »). Vide dans l'immense majorité des appels.
    pub fn drain_watchlist_alerts(&mut self) -> Vec<crate::watchlist::WatchlistAlert> {
        std::mem::take(&mut self.pending_alerts)
    }

    /// Ingère un lot déjà lu par `overlay-ingest`, met à jour l'état de session en place, et
    /// renvoie les `LogEntry` produits (utile pour un affichage brut — chat, journal — que ce
    /// premier slice de L2 n'agrège pas encore, voir le module `session`).
    pub fn ingest_batch(
        &mut self,
        batch: &overlay_ingest::LineBatch,
    ) -> Result<Vec<LogEntry>, crate::quickjs_engine::EngineError> {
        if batch.is_initial_load && !self.in_initial_sweep {
            // Nouveau rattrapage (reconnexion ou rotation, §5.3) : le PARSER repart toujours de
            // zéro (contexte transitoire QuickJS resynchronisé avec la position de lecture). La
            // watchlist n'est PAS réinitialisée ici (voir son champ ci-dessus) : ses compteurs
            // sont un suivi persistant, pas un état de combat.
            self.parser.reset()?;
            // `state`, en revanche, n'est vidé qu'au TOUT PREMIER rattrapage (voir
            // `state_initialized`) — jamais à une rotation ultérieure en cours de session, qui
            // effacerait à tort un combat encore actif (retour utilisateur 2026-09-02, vidéo à
            // l'appui : Wakfu ne rejoue pas l'historique déjà lu après rotation).
            if !self.state_initialized {
                self.state = SessionState::default();
                self.state_initialized = true;
            }
        }
        self.in_initial_sweep = batch.is_initial_load;

        let entries = self.parser.parse_lines(&batch.lines)?;
        for entry in &entries {
            let implicitly_defeated = self.state.apply(entry, self.roster.as_ref());
            // Miroir du gating `currentBatchIsInitialLoad` de `registerLoot`/`registerDefeat` côté
            // web (voir `watchlist.rs`) : le contenu déjà présent dans le fichier au premier
            // chargement ne doit pas regonfler un compteur qui persiste d'une session à l'autre.
            if !batch.is_initial_load {
                self.pending_alerts.extend(self.watchlist.apply(entry));
                // Filet de rattrapage du dernier ennemi d'un combat (voir la doc de
                // `SessionState::apply`, cas `CombatEnd`) : crédité à la watchlist comme s'il
                // s'agissait d'autant de `LogEntry::EnemyDefeated` supplémentaires — même chemin,
                // pas de logique dupliquée dans `WatchlistState`.
                for name in implicitly_defeated {
                    let alerts = self.watchlist.apply(&LogEntry::EnemyDefeated {
                        time: entry.time().to_string(),
                        name,
                        fight_id: None,
                    });
                    self.pending_alerts.extend(alerts);
                }
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

    /// Comme `fighter_joined`, mais pour une invocation (`summoned_by` renseigné) — `wakfu.log`
    /// logue TOUJOURS `is_controlled_by_ai: true` pour une invocation, même alliée, d'où le
    /// paramètre fixé en dur ici plutôt qu'exposé à l'appelant (jamais autre chose en pratique).
    fn summoned_fighter_joined(fight_id: i64, name: &str, summoned_by: &str) -> LogEntry {
        LogEntry::FighterJoined {
            time: "12:00:00,000".to_string(),
            fight_id,
            name: name.to_string(),
            breed: 0,
            fighter_id: 2,
            is_controlled_by_ai: true,
            summoned_by: Some(summoned_by.to_string()),
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

    fn spell_cast(fight_id: i64, caster: &str) -> LogEntry {
        LogEntry::SpellCast {
            time: "12:00:00,500".to_string(),
            caster: caster.to_string(),
            spell: "sort".to_string(),
            critical: false,
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

    /// Régression réelle (retour utilisateur, 2026-09-02, captures d'écran à l'appui) :
    /// « il n'y a que quatre monstres qui sont toujours affichés, pas plus » sur des combats qui
    /// en affichent visiblement bien plus — deux ennemis EXACTEMENT homonymes (pack du même
    /// monstre) se fusionnaient en une seule ligne. Séquence délibérément entrelacée pour exercer
    /// la file d'initiative (`InitiativeSeat`/`resolve_next_actor`) : un sort d'un tiers ("Oumbra")
    /// s'intercale entre les deux tours de "Mouton", condition nécessaire pour que le deuxième
    /// tour soit reconnu comme une AUTRE instance plutôt que fusionné avec le premier (limite
    /// documentée, voir la doc de `resolve_next_actor`).
    #[test]
    fn deux_ennemis_homonymes_dans_un_meme_combat_restent_deux_lignes_distinctes() {
        let mut state = SessionState::default();
        state.apply(&fighter_joined(1, "Mouton", 1, true), None); // 1ʳᵉ instance
        state.apply(&fighter_joined(1, "Mouton", 1, true), None); // 2ᵉ instance, MÊME nom
        state.apply(&fighter_joined(1, "Oumbra", 9, false), None); // allié, sert d'intercalaire

        state.apply(&spell_cast(1, "Mouton"), None); // 1er tour de Mouton -> siège #1
        state.apply(&damage(1, "Mouton", 50), None);
        state.apply(&spell_cast(1, "Oumbra"), None); // tour intercalaire (autre acteur)
        state.apply(&spell_cast(1, "Mouton"), None); // tour SUIVANT de Mouton -> siège #2
        state.apply(&damage(1, "Mouton", 30), None);

        let snapshot = state.snapshot();
        let fight = &snapshot.fights[0];
        let moutons: Vec<i64> = fight
            .fighters
            .iter()
            .filter(|f| f.name == "Mouton")
            .map(|f| f.total_damage)
            .collect();

        assert_eq!(
            fight.fighters.len(),
            3,
            "3 combattants distincts ont rejoint (2 Mouton + 1 Oumbra), aucune fusion attendue"
        );
        assert_eq!(
            moutons,
            vec![50, 30],
            "chaque tour de Mouton doit créditer une instance DIFFÉRENTE, pas toujours la même"
        );
    }

    /// Retour utilisateur 2026-09-02 (captures d'écran à l'appui) : l'invocation d'un allié
    /// (mécanisme, totem) s'affichait à tort côté ennemis — `wakfu.log` logue TOUJOURS
    /// `is_controlled_by_ai: true` pour une invocation, quel que soit le camp réel de son
    /// invocateur. `summoned_by` doit donc primer sur `is_controlled_by_ai` : aucune ligne pour
    /// l'invocation, ni alliée ni ennemie (voir la doc de module).
    #[test]
    fn une_invocation_nobtient_jamais_sa_propre_ligne() {
        let mut state = SessionState::default();
        state.apply(&fighter_joined(1, "Oumbra", 9, false), None); // allié invocateur
        state.apply(
            &summoned_fighter_joined(1, "Balise de Contact", "Oumbra"),
            None,
        );

        let fight = &state.fights[&1];
        assert_eq!(
            fight.snapshot.fighters.len(),
            1,
            "seul l'invocateur doit avoir une ligne, jamais son invocation"
        );
        assert_eq!(fight.snapshot.fighters[0].name, "Oumbra");
    }

    /// Complète le test ci-dessus : si une ligne de dégâts porte malgré tout le nom BRUT de
    /// l'invocation (cas non réattribué par le parser, voir `fighter_mut`), ces dégâts ne doivent
    /// créditer PERSONNE — ni l'invocation (pas de ligne), ni l'invocateur par erreur (miroir du
    /// filtre `summonNames` d'`addDamage`/`ensurePresent` côté web).
    #[test]
    fn les_degats_bruts_dune_invocation_ne_creditent_personne() {
        let mut state = SessionState::default();
        state.apply(&fighter_joined(1, "Oumbra", 9, false), None);
        state.apply(
            &summoned_fighter_joined(1, "Balise de Contact", "Oumbra"),
            None,
        );
        state.apply(&damage(1, "Balise de Contact", 999), None);

        let fight = &state.fights[&1];
        assert_eq!(fight.snapshot.fighters.len(), 1);
        assert_eq!(
            fight.snapshot.fighters[0].total_damage, 0,
            "les dégâts d'une invocation non réattribuée ne doivent créditer personne"
        );
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

    fn combat_end(fight_id: i64, result: FightResult) -> LogEntry {
        LogEntry::CombatEnd {
            time: "12:00:02,000".to_string(),
            fight_id,
            result,
        }
    }

    fn enemy_defeated(fight_id: i64, name: &str) -> LogEntry {
        LogEntry::EnemyDefeated {
            time: "12:00:01,500".to_string(),
            name: name.to_string(),
            fight_id: Some(fight_id),
        }
    }

    fn enemy_fled(fight_id: i64, name: &str) -> LogEntry {
        LogEntry::EnemyFled {
            time: "12:00:01,500".to_string(),
            name: name.to_string(),
            fight_id: Some(fight_id),
        }
    }

    /// Régression réelle (retour utilisateur, 2026-09-01) : un boss qui meurt exactement en même
    /// temps que la fin du combat n'a pas toujours droit à sa propre ligne "est KO !" — voir la
    /// doc de `SessionState::apply`, cas `CombatEnd`.
    #[test]
    fn filet_de_rattrapage_credite_un_ennemi_jamais_vaincu_explicitement_sur_victoire() {
        let mut state = SessionState::default();
        state.apply(&fighter_joined(1, "El Pochito", 1, true), None);
        state.apply(&fighter_joined(1, "Oumbra Canin", 15, false), None);

        let implicitly_defeated = state.apply(&combat_end(1, FightResult::Won), None);

        assert_eq!(implicitly_defeated, vec!["El Pochito".to_string()]);
    }

    #[test]
    fn filet_de_rattrapage_ne_double_compte_pas_un_ennemi_deja_vaincu_explicitement() {
        let mut state = SessionState::default();
        state.apply(&fighter_joined(1, "Bwork", 1, true), None);
        state.apply(&enemy_defeated(1, "Bwork"), None); // ligne "est KO !" explicite, avant la fin

        let implicitly_defeated = state.apply(&combat_end(1, FightResult::Won), None);

        assert!(
            implicitly_defeated.is_empty(),
            "déjà crédité explicitement, le filet ne doit rien ajouter"
        );
    }

    /// Demande explicite de l'utilisateur (cas des mimiques, voir `registerFightFlee` côté web) :
    /// une fuite n'est jamais requalifiée en victoire sous prétexte que le reste de l'équipe a
    /// gagné.
    #[test]
    fn filet_de_rattrapage_ne_credite_jamais_un_ennemi_en_fuite() {
        let mut state = SessionState::default();
        state.apply(&fighter_joined(1, "Mimique", 1, true), None);
        state.apply(&enemy_fled(1, "Mimique"), None);

        let implicitly_defeated = state.apply(&combat_end(1, FightResult::Won), None);

        assert!(implicitly_defeated.is_empty());
    }

    #[test]
    fn filet_de_rattrapage_ne_sapplique_pas_sur_une_defaite() {
        let mut state = SessionState::default();
        state.apply(&fighter_joined(1, "Bwork", 1, true), None);

        let implicitly_defeated = state.apply(&combat_end(1, FightResult::Lost), None);

        assert!(implicitly_defeated.is_empty());
    }
}
