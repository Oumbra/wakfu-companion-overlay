//! Agrégation `LogEntry` → `SessionSnapshot`, en Rust, **pas** vendue depuis `StatsStoreService`.
//!
//! Volontairement minimal pour ce qu'affiche L2 (dégâts du combat en cours, récap de session) —
//! pas d'extraction de `StatsStoreService` (2 755 l., couplé Angular, décision encore ouverte,
//! voir docs/plan-architecture.md §14 point 3) pour ce périmètre. La synchro serveur (§7, L5),
//! elle, EXIGE une parité plus stricte sur certains points précis : le regroupement de combats de
//! donjon multi-salles (`resolve_dungeon_assignment`, voir `dungeon_run.rs`) et la corrélation
//! kamas↔achat HDV (`consider_hdv_kama_gain`) sont désormais portés ici, vérifiés directement
//! contre le dépôt web (`../wakfu-companion`, disponible en local) plutôt que redevinés.
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
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::catalog::CatalogIndex;
use crate::class_breed::class_for_breed;
use crate::dungeon::{DungeonEntry, DungeonIndex};
use crate::dungeon_run::{
    enemy_composition_key, find_dungeon_for_enemies, group_dungeon_runs, DungeonHistoryEntry,
    GroupableFight,
};
use crate::history::{
    fight_signature, purchase_signature, trade_signature, FightLootPayload,
    FightParticipantPayload, FightPayload, FightSide, FightSpellPayload, HistoryEventKind,
    HistoryPayload, PurchasePayload, SyncEvent, TradeDirection, TradeItemPayload, TradePayload,
    HDV_KAMAS_SALE_ITEM,
};
use crate::log_time::{format_iso_utc, time_of_day_ms, LogDateTracker};
use crate::model::{DamageElement, FightResult, LogEntry};
use crate::roster::{normalize_wakfu_name, Gender, RosterIndex};
use crate::watchlist::{WatchlistEntry, WatchlistState};

/// Fenêtre de corrélation perte de kamas → ramassage suivant pour reconnaître un achat marchand/
/// HDV — miroir exact de `PURCHASE_WINDOW_MS` (`stats-store.service.ts`). Réutilisée aussi pour
/// corréler un gain de kamas hors combat à un échange tout juste conclu (voir
/// `SessionState::consider_hdv_kama_gain`/`resolve_pending_hdv_kama_gain`) — même fenêtre côté web
/// (`considerHdvKamaGain`).
const PURCHASE_WINDOW_MS: i64 = 2_000;

/// Résout `itemId`/`itemName`, mutuellement exclusifs — miroir exact d'`HistorySyncService.
/// itemPayload` (`history-sync.service.ts`) : un id catalogue connu remplace TOUJOURS le nom brut
/// (jamais les deux à la fois), et un objet non résolu (catalogue pas encore chargé, ou nom que le
/// référentiel ne connaît pas) retombe sur le nom brut seul.
fn item_payload(catalog: Option<&CatalogIndex>, name: &str) -> (Option<i64>, Option<String>) {
    match catalog.and_then(|c| c.find_item_id(name)) {
        Some(id) => (Some(id), None),
        None => (None, Some(name.to_string())),
    }
}

/// Étiquette d'élément telle qu'attendue dans `FightSpellPayload::by_element` — miroir des clés de
/// `DAMAGE_ELEMENTS` (`log-parser.ts`) : mêmes chaînes françaises que celles déjà affichées dans le
/// log lui-même, `element` n'étant reconstruit ici que pour ce besoin d'agrégation (aucun autre
/// consommateur overlay n'a besoin de sérialiser `DamageElement`, voir sa doc dans `model.rs`).
fn damage_element_label(element: DamageElement) -> &'static str {
    match element {
        DamageElement::Neutre => "Neutre",
        DamageElement::Terre => "Terre",
        DamageElement::Feu => "Feu",
        DamageElement::Eau => "Eau",
        DamageElement::Air => "Air",
        DamageElement::Lumiere => "Lumière",
        DamageElement::Stasis => "Stasis",
        DamageElement::Inconnu => "Inconnu",
    }
}

/// Données de référence nécessaires à la construction des événements d'historique (L5, §7.1 du
/// plan) et à la classification d'un allié (§2 du plan) — regroupées pour éviter une signature à
/// rallonge sur `SessionState::apply`/`build_fight_sync_event`. Toutes optionnelles : un champ
/// `None` signifie simplement « pas encore chargé/résolu », jamais une erreur (voir chaque champ
/// consommé pour son repli précis).
#[derive(Clone, Copy, Default)]
struct ApplyContext<'a> {
    roster: Option<&'a RosterIndex>,
    catalog: Option<&'a CatalogIndex>,
    dungeons: Option<&'a DungeonIndex>,
    /// Code du serveur de jeu déduit du dernier personnage du roster reconnu dans le log — voir
    /// `Engine::current_game_server`. Calculé une fois par ligne par l'appelant (`Engine::
    /// ingest_batch`), jamais recalculé ici : `SessionState` ne connaît ni le roster complet ni
    /// l'historique des personnages déjà croisés (portés par `Engine`, voir sa doc).
    game_server: Option<&'a str>,
}

/// `Serialize`/`Deserialize` servent à la persistance disque du combat en cours (voir
/// `fight_store.rs`, §9 du plan) — restauration après un redémarrage de l'overlay survenu pendant
/// une rotation de `wakfu.log`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// XP gagnée par CE combattant sur ce combat (0 pour tout ennemi, un monstre n'en gagne
    /// jamais) — miroir de `registerFightXp`/`FightRecord.xp` (`stats-store.service.ts`) : accumulé
    /// depuis `LogEntry::XpGain::character`, jamais recalculé à l'envoi (voir
    /// `build_fight_sync_event`). `#[serde(default)]` : un fichier `fight-*.json` persisté par une
    /// version de l'overlay antérieure à ce champ (voir `fight_store.rs`) reste chargeable, à `0`.
    #[serde(default)]
    pub xp_gained: i64,
    /// Dégâts infligés PAR ce combattant, ventilés par nom de sort puis par élément — miroir de
    /// `FightSpellPayload`/`SpellBreakdownRow` (`stats-store.service.ts`). Alimenté UNIQUEMENT
    /// depuis `LogEntry::Damage` (pas les soins, voir `build_fight_sync_event` pour la doc de ce
    /// choix) — `#[serde(default)]` même raison que `xp_gained` ci-dessus.
    #[serde(default)]
    pub spells: HashMap<String, HashMap<String, i64>>,
    /// Ce combattant a-t-il été mis KO au moins une fois DANS ce combat — alimenté par
    /// `LogEntry::EnemyDefeated` (voir `SessionState::apply`, cas `EnemyDefeated`), qui malgré son
    /// nom couvre aussi bien "X est KO !" (réservé aux alliés, `KO_RE`) que "X est hors-combat !"
    /// (diffusé à tout combattant, `HORS_COMBAT_RE`) — voir `log-parser.ts`. Ne redevient JAMAIS
    /// `false` une fois posé (pas de suivi de "ressuscité en plein combat" ici) : c'est un simple
    /// indicateur d'affichage (portrait grisé, `overlay-ui::panels::combat`), pas une donnée de
    /// synchro — `build_fight_sync_event` continue de dériver `defeated`/`fled` indépendamment à
    /// partir de `resolved_enemies`/`fled_names` pour le payload serveur, ce champ-ci n'y participe
    /// pas. `#[serde(default)]` même raison que `xp_gained` ci-dessus (champ ajouté après coup).
    #[serde(default)]
    pub is_ko: bool,
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

/// `Serialize`/`Deserialize` : voir `FighterDamage` — c'est la forme exacte persistée par
/// `fight_store::save_fight`/`load_ongoing_fights` (un fichier par combat encore `ongoing`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Noms (minuscules) d'ennemis résolus par une FUITE (`EnemyFled`) — sous-ensemble de
    /// `resolved_enemies` (voir `mark_resolved`) : sert uniquement à distinguer `defeated`/`fled`
    /// au moment de construire `FightParticipantPayload` (L5, §7.1), `resolved_enemies` seul ne le
    /// permettant pas (les deux causes de résolution y sont fusionnées, voir sa doc). Perdu si
    /// l'overlay redémarre en cours de combat (`restore_fight` ne le reconstruit pas, comme le
    /// reste de l'état d'attribution) — un ennemi déjà fui avant un redémarrage serait alors
    /// compté `defeated` plutôt que `fled` s'il se trouve aussi implicitement résolu par le filet
    /// de rattrapage : écart mineur assumé, cohérent avec les autres pertes déjà documentées de
    /// `restore_fight`.
    fled_names: std::collections::HashSet<String>,
    /// Instant (ms, époque Unix) de la toute première jonction de ce combat — voir
    /// `LogDateTracker::full_timestamp_ms`. Base de `FightPayload::started_at`/`duration_ms`.
    /// `0` (jamais un vrai combat, epoch 1970) uniquement pour un combat restauré depuis un fichier
    /// `fight-*.json` écrit par une version de l'overlay antérieure à ce champ — voir
    /// `restore_fight`, cas limite transitoire sans conséquence passé la toute première mise à
    /// jour.
    started_at_ms: i64,
    /// Butin ramassé PENDANT ce combat (nom, quantité) — jamais un objet déjà classé "achat" (voir
    /// le filet purchase de `SessionState::apply`) : distinct de `SessionState::recent_loot`
    /// (fenêtre glissante d'affichage, tous combats confondus) et non affiché par L2, seulement
    /// utile à `FightPayload::loot` (L5).
    loot: Vec<(String, i64)>,
    /// Kamas gagnés PENDANT ce combat — voir `FightPayload::kamas_gained`. Accumulation directe
    /// sur le `fight_id` porté par `LogEntry::KamaGain`, plus simple que le mécanisme "en attente
    /// jusqu'au prochain combat-end" de `pendingFightKamas` côté web (utile là-bas seulement parce
    /// que son fightId n'est jamais fiable à cet endroit précis du pipeline TS) : ici
    /// `LogEntry::KamaGain::fight_id` vient déjà résolu par le parser vendu, l'attribution directe
    /// suffit.
    kamas_gained: i64,
    /// XP gagnée PENDANT ce combat, TOUS participants confondus — voir `FightPayload::xp_gained`
    /// (total, distinct de la ventilation par participant portée par `FighterDamage::xp_gained`).
    /// Miroir de `record.xp.reduce((sum, row) => sum + row.amount, 0)` (`HistorySyncService.
    /// enqueueFight`), où `record.xp` n'est alimenté QUE par `registerFightXp` — donc seulement les
    /// `LogEntry::XpGain` dont le `character` a réellement rejoint CE combat (voir
    /// `SessionState::apply`, cas `XpGain`) ; jamais un nom inconnu de ce combat, même si son
    /// `fight_id` s'y résout (fightId mal résolu, combat concurrent).
    xp_gained_total: i64,
    /// Heure brute (`HH:MM:SS,mmm`) de la ligne `CombatEnd` de CE combat, et son équivalent en
    /// horodatage complet (ms) — capturés une seule fois à sa fin, `None` tant qu'il est `ongoing`.
    /// Nécessaires pour recalculer la signature/le payload d'un combat déjà terminé s'il devient
    /// SIBLING d'un run de donjon découvert par un combat plus récent (voir `dungeon_run.rs`,
    /// `SessionState::resolve_dungeon_assignment`) : sans ces deux valeurs, impossible de rebâtir
    /// fidèlement son `SyncEvent` (même `started_at`/`duration_ms`/signature qu'à l'origine) une
    /// seconde fois, longtemps après que la ligne `CombatEnd` d'origine a été traitée.
    ended_at_time: Option<String>,
    ended_at_ms: Option<i64>,
    challenges_passed: i64,
    challenges_failed: i64,
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
    /// Reconstruction de date calendaire réelle depuis `LogEntry::LogDateAnchor` — voir
    /// `log_time.rs`. Un seul tracker pour tout `wakfu.log` (flux chronologique unique, voir doc
    /// de module) : une rotation en cours de session pose simplement un nouvel ancrage par-dessus,
    /// jamais réinitialisé explicitement (pas plus que `totals`/`recent_loot` ne le sont à une
    /// rotation, voir `Engine::state_initialized`).
    date_tracker: LogDateTracker,
    /// Perte de kamas en attente d'un ramassage corrélé (fenêtre `PURCHASE_WINDOW_MS`) — miroir de
    /// `pendingPurchase` (`stats-store.service.ts`) : `(montant, heure du jour en ms)`. Volontairement
    /// PAS scopé à un combat : un achat marchand/HDV n'est jamais un événement de combat, même si un
    /// combat est actif en parallèle sur un autre personnage (multi-compte).
    pending_purchase: Option<(i64, i64)>,
    /// Gain de kamas hors combat en attente de confirmation — miroir de `pendingHdvKamaGain`
    /// (`stats-store.service.ts`) : `(montant, heure brute du log, heure du jour en ms)`. Committé
    /// comme récupération de kamas HDV (voir `HDV_KAMAS_SALE_ITEM`) dès que la ligne SUIVANTE
    /// n'est pas l'échange qui l'expliquerait (voir `resolve_pending_hdv_kama_gain`) — la ligne
    /// "Vous avez gagné" d'un échange peut précéder OU suivre de très peu son propre
    /// `TradeCompleted` selon les fichiers observés, d'où cette confirmation à un pas plutôt qu'un
    /// simple regard en arrière (voir `consider_hdv_kama_gain`, qui consulte
    /// `last_trade_completed_at_ms` pour le cas où le trade précède).
    pending_hdv_kama_gain: Option<(i64, String, i64)>,
    /// Horodatage (ms du jour) du dernier `TradeCompleted` traité — miroir de
    /// `lastTradeCompletedAtMs` : permet à `consider_hdv_kama_gain` de reconnaître un gain de kamas
    /// qui vient d'être expliqué par un échange tout juste conclu (cas où le `TradeCompleted`
    /// PRÉCÈDE la ligne de gain).
    last_trade_completed_at_ms: Option<i64>,
}

impl SessionState {
    /// Renvoie les noms d'ennemis crédités IMPLICITEMENT comme vaincus par le filet de rattrapage
    /// de `CombatEnd` ci-dessous (vide dans tous les autres cas) — l'appelant (`Engine::
    /// ingest_batch`) s'en sert pour créditer la watchlist comme s'il s'agissait d'autant de
    /// `LogEntry::EnemyDefeated` supplémentaires, seul endroit qui connaît `WatchlistState`.
    ///
    /// `sync_events` (L5, §7.1) reçoit tout événement d'historique prêt à synchroniser détecté sur
    /// CETTE ligne : un fight au `CombatEnd`, un purchase à un `Loot` corrélé à une perte de kamas
    /// récente, un trade à un `TradeCompleted`. Motif "out-param" plutôt qu'un second `Vec` en
    /// retour : `apply` a déjà un retour dédié (`implicitly_defeated_enemies`), et l'appelant a de
    /// toute façon besoin d'accumuler ces événements sur PLUSIEURS appels (tout un lot) avant de
    /// les drainer — voir `Engine::pending_sync_events`/`drain_sync_events`.
    fn apply(
        &mut self,
        entry: &LogEntry,
        ctx: ApplyContext<'_>,
        sync_events: &mut Vec<SyncEvent>,
    ) -> Vec<String> {
        let mut implicitly_defeated_enemies = Vec::new();

        // Détection d'achat marchand/HDV (miroir du préambule d'`apply` côté web,
        // `stats-store.service.ts`) : une perte de kamas immédiatement suivie (fenêtre
        // `PURCHASE_WINDOW_MS`) d'un ramassage en fait un achat, jamais du butin de combat — voir
        // la doc de `FightWorking::loot`. `pending_purchase` est remis à `None` après TOUTE ligne
        // qui n'est pas elle-même une nouvelle perte de kamas (même règle que le web : un achat
        // groupé où plusieurs objets suivent une seule perte n'associe l'achat qu'au tout premier
        // ramassage, les suivants restent traités comme avant — écart de fidélité assumé, voir la
        // doc de module de `history.rs`).
        let mut purchase_loot = false;
        if let LogEntry::Loot {
            item,
            quantity,
            time,
            fight_id,
        } = entry
        {
            if let Some((amount, pending_time_ms)) = self.pending_purchase {
                if let Some(time_ms) = time_of_day_ms(time) {
                    if time_ms - pending_time_ms <= PURCHASE_WINDOW_MS {
                        purchase_loot = true;
                        sync_events.push(build_purchase_sync_event(
                            item,
                            *quantity,
                            amount,
                            time,
                            self.date_tracker.full_timestamp_ms(time),
                            ctx,
                        ));
                    }
                }
            }
            if !purchase_loot {
                if let Some(fight_id) = fight_id {
                    if let Some(fight) = self.fights.get_mut(fight_id) {
                        fight.loot.push((item.clone(), *quantity));
                    }
                }
            }
        }
        if !matches!(entry, LogEntry::KamaLoss { .. }) {
            self.pending_purchase = None;
        }

        // Un gain de kamas hors combat en attente de confirmation (voir `pending_hdv_kama_gain`)
        // est committé comme récupération de kamas HDV dès que CETTE ligne n'est pas l'échange qui
        // l'expliquerait — avant de traiter la ligne courante elle-même. Miroir exact de
        // `resolvePendingHdvKamaGain`, appelé au même endroit côté web (après le préambule achat,
        // avant le `switch` principal).
        if let Some((amount, pending_time, pending_time_ms)) = self.pending_hdv_kama_gain.take() {
            let explained_by_this_trade = matches!(entry, LogEntry::TradeCompleted { time, .. }
                if time_of_day_ms(time).is_some_and(|ms| (ms - pending_time_ms).abs() <= PURCHASE_WINDOW_MS));
            if !explained_by_this_trade {
                let occurred_at_ms = self.date_tracker.full_timestamp_ms(&pending_time);
                sync_events.push(build_purchase_sync_event(
                    HDV_KAMAS_SALE_ITEM,
                    0,
                    amount,
                    &pending_time,
                    occurred_at_ms,
                    ctx,
                ));
            }
        }

        match entry {
            LogEntry::LogDateAnchor {
                year, month, day, ..
            } => {
                self.date_tracker.set_anchor(*year, *month, *day);
            }
            LogEntry::FighterJoined {
                fight_id,
                name,
                breed,
                is_controlled_by_ai,
                summoned_by,
                ..
            } => {
                let started_at_ms = self.date_tracker.full_timestamp_ms(entry.time());
                self.ensure_fight(*fight_id, started_at_ms);
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
                    resolve_ally_class(name, *breed, ctx.roster)
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
                spell,
                element,
                ..
            } => {
                if let Some(fighter) = self.fighter_mut(*fight_id, attacker) {
                    fighter.total_damage += amount;
                    // Ventilation par sort/élément (L5, §7.1) — miroir de `SpellBreakdownRow`, mais
                    // UNIQUEMENT pour les dégâts (voir la doc de `FighterDamage::spells` et de
                    // `build_fight_sync_event` pour le raisonnement de ce choix, les soins n'y
                    // entrent volontairement pas).
                    *fighter
                        .spells
                        .entry(spell.clone())
                        .or_default()
                        .entry(damage_element_label(*element).to_string())
                        .or_insert(0) += amount;
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
                // `EnemyDefeated` couvre aussi bien un ennemi qu'un allié (voir la doc de
                // `FighterDamage::is_ko`) : poser le drapeau d'affichage sur CE combattant précis,
                // indépendamment de `resolved_enemies` (partagé avec `EnemyFled`, où ce drapeau ne
                // doit PAS être posé — voir plus bas).
                self.mark_ko(*fight_id, name);
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
                if let Some(fight) = self.fights.get_mut(fight_id) {
                    fight.fled_names.insert(name.to_lowercase());
                }
            }
            LogEntry::CombatEnd {
                fight_id, result, ..
            } => {
                let end_ms = self.date_tracker.full_timestamp_ms(entry.time());
                let won = *result == FightResult::Won;
                if let Some(fight) = self.fights.get_mut(fight_id) {
                    fight.snapshot.ongoing = false;
                    fight.snapshot.result = Some(*result);
                    // Voir `FightWorking::ended_at_time`/`ended_at_ms` : nécessaires pour rebâtir
                    // ce combat plus tard s'il devient sibling d'un run découvert par un combat
                    // plus récent (voir `resolve_dungeon_assignment`).
                    fight.ended_at_time = Some(entry.time().to_string());
                    fight.ended_at_ms = Some(end_ms);
                    // Filet de rattrapage — miroir de `finalizeFight` (`stats-store.service.ts`) :
                    // le dernier ennemi d'un combat (souvent le boss) meurt parfois EXACTEMENT en
                    // même temps que le combat se termine, sans jamais produire sa propre ligne
                    // "est KO !"/"est hors-combat !" (constaté en session, 2026-09-01 : boss "El
                    // Pochito" jamais crédité dans le Suivi malgré une victoire confirmée par le
                    // butin ramassé juste après — vérifié dans le vrai wakfu.log, aucune ligne de
                    // défaite ne précède le ramassage). Un combat GAGNÉ implique que tout ennemi
                    // ayant rejoint et jamais résolu (ni vaincu explicitement, ni en fuite) est
                    // mort en même temps que le combat.
                    if won {
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
                // L5, §7.1 : un combat terminé est toujours prêt à synchroniser, qu'il soit gagné
                // ou perdu — construit AVANT `prune_ended_fights()` ci-dessous, qui retire ce
                // combat de `self.fights` dès que la limite `MAX_TRACKED_FIGHTS` l'exige.
                if self.fights.contains_key(fight_id) {
                    // Voir `resolve_dungeon_assignment` : ce combat rejoint-il un run multi-salles
                    // (donjon `TWO_ROOMS`/`THREE_ROOMS`/`FOUR_ROOMS`) déjà entamé ? `fight_id` est
                    // TOUJOURS le représentant d'un run trouvé ici (c'est nécessairement le combat
                    // le plus récent de la session — voir la doc de tête de `dungeon_run.rs`).
                    let (dungeon, siblings) = self.resolve_dungeon_assignment(*fight_id, ctx);
                    let dungeon_id = dungeon.as_ref().map(|(id, _)| *id);

                    // Siblings du run (salles/tentatives antérieures déjà envoyées SANS
                    // rattachement) : renvoyés à leur tour avec le MÊME dungeonId et la MÊME
                    // graine de run (la signature de CE combat, calculée une seule fois) — miroir
                    // de `HistorySyncService.recordFight`, boucle sur `assignment.siblings`.
                    if let (Some(dungeon_id), false) = (dungeon_id, siblings.is_empty()) {
                        let run_signature = self
                            .fights
                            .get(fight_id)
                            .map(|fight| fight_own_signature(fight, entry.time(), won));
                        for sibling_id in &siblings {
                            let Some(sibling) = self.fights.get(sibling_id) else {
                                continue;
                            };
                            let (Some(sib_time), Some(sib_end_ms)) =
                                (sibling.ended_at_time.clone(), sibling.ended_at_ms)
                            else {
                                continue; // ne devrait pas arriver : un sibling est par définition déjà terminé
                            };
                            let sib_won = matches!(sibling.snapshot.result, Some(FightResult::Won));
                            sync_events.push(build_fight_sync_event(
                                sibling,
                                &sib_time,
                                sib_won,
                                sib_end_ms,
                                ctx,
                                Some((dungeon_id, run_signature.clone())),
                            ));
                        }
                    }

                    if let Some(fight) = self.fights.get(fight_id) {
                        sync_events.push(build_fight_sync_event(
                            fight,
                            entry.time(),
                            won,
                            end_ms,
                            ctx,
                            dungeon,
                        ));
                    }
                }
                self.prune_ended_fights();
            }
            LogEntry::KamaGain {
                amount,
                fight_id,
                time,
            } => {
                self.totals.kamas_gained += amount;
                match fight_id {
                    Some(fight_id) => {
                        if let Some(fight) = self.fights.get_mut(fight_id) {
                            fight.kamas_gained += amount;
                        }
                    }
                    // Gain hors combat : candidat à une récupération de kamas HDV, sauf s'il vient
                    // d'être expliqué par un échange tout juste conclu — voir
                    // `consider_hdv_kama_gain` et la doc de `pending_hdv_kama_gain`.
                    None => self.consider_hdv_kama_gain(*amount, time),
                }
            }
            LogEntry::KamaLoss { amount, time } => {
                self.totals.kamas_lost += amount;
                self.pending_purchase = time_of_day_ms(time).map(|ms| (*amount, ms));
            }
            LogEntry::XpGain {
                character,
                amount,
                fight_id,
                ..
            } => {
                self.totals.xp_gained += amount;
                if let Some(fight_id) = fight_id {
                    if let Some(fight) = self.fights.get_mut(fight_id) {
                        // Miroir exact de `registerFightXp` : ni la ventilation par participant NI
                        // le total du combat (`FightPayload::xp_gained` = `record.xp.reduce(sum)`,
                        // qui ne lit QUE les entrées poussées par `registerFightXp`) ne créditent un
                        // nom qui n'a jamais rejoint CE combat précis (`isRosterMember`, ici "membre
                        // de CE combat", pas du roster de compte) — un fightId mal résolu (repli sur
                        // le dernier combat courant côté web) ne doit jamais gonfler le total d'un
                        // AUTRE combat. `fighter_index` ne pointe que vers des combattants ayant
                        // réellement rejoint ce combat, et seul le PREMIER (`ids.first()`) d'un nom
                        // partagé est crédité (plusieurs instances homonymes ne multiplient jamais
                        // l'XP par leur nombre).
                        if let Some(&idx) = fight
                            .fighter_index
                            .get(character)
                            .and_then(|ids| ids.first())
                        {
                            fight.xp_gained_total += amount;
                            fight.snapshot.fighters[idx].xp_gained += amount;
                        }
                    }
                }
            }
            LogEntry::ChallengeResult {
                success,
                fight_id: Some(fight_id),
                ..
            } => {
                if let Some(fight) = self.fights.get_mut(fight_id) {
                    if *success {
                        fight.challenges_passed += 1;
                    } else {
                        fight.challenges_failed += 1;
                    }
                }
            }
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
            LogEntry::TradeCompleted { time, sides } => {
                // Voir `pending_hdv_kama_gain` : un gain de kamas hors combat qui vient tout juste
                // d'être expliqué par CET échange est déjà traité ci-dessus (résolution en tête de
                // fonction) — cet horodatage sert au cas symétrique (le gain survient APRÈS
                // l'échange qui l'explique), consulté par `consider_hdv_kama_gain`.
                self.last_trade_completed_at_ms = time_of_day_ms(time);
                let occurred_ms = self.date_tracker.full_timestamp_ms(time);
                if let Some(event) = build_trade_sync_event(time, sides, occurred_ms, ctx) {
                    sync_events.push(event);
                }
            }
            // Hors périmètre de ce premier slice (voir le commentaire de module) : chat, armor,
            // combat-defeat-marker, combat-start (ne porte pas de fightId, voir le TS vendu),
            // market-occupation, et les variantes sans fightId (kamas/loot hors combat, dégâts non
            // résolus, spell-cast/enemy-defeated/fled/challenge-result sans fightId résolu par le
            // parser).
            _ => {}
        }
        implicitly_defeated_enemies
    }

    /// Met en attente un gain de kamas hors combat comme candidat à une récupération de kamas HDV
    /// (voir `HDV_KAMAS_SALE_ITEM`) — miroir exact de `considerHdvKamaGain` (`stats-store.service.
    /// ts`). Annulé si un `TradeCompleted` vient d'être traité dans la fenêtre `PURCHASE_WINDOW_MS`
    /// (cas où l'échange PRÉCÈDE la ligne de gain — le cas symétrique, où l'échange SUIT, est
    /// géré par `apply`, résolution en tête de fonction).
    fn consider_hdv_kama_gain(&mut self, amount: i64, time: &str) {
        let Some(time_ms) = time_of_day_ms(time) else {
            return;
        };
        if let Some(last_trade_ms) = self.last_trade_completed_at_ms {
            if (time_ms - last_trade_ms).abs() <= PURCHASE_WINDOW_MS {
                return; // déjà expliqué par l'échange qui vient d'être traité
            }
        }
        self.pending_hdv_kama_gain = Some((amount, time.to_string(), time_ms));
    }

    /// Committe SANS CONDITION le gain en attente (voir `pending_hdv_kama_gain`) — à appeler par
    /// `Engine::ingest_batch` en fin de lot, pour ne jamais le perdre si la prochaine ligne tarde
    /// à arriver (voire une reconnexion qui viderait silencieusement l'état). Miroir exact de
    /// `flushPendingHdvKamaGain`, appelé au même moment côté web (fin d'`ingest()`).
    fn flush_pending_hdv_kama_gain(
        &mut self,
        ctx: ApplyContext<'_>,
        sync_events: &mut Vec<SyncEvent>,
    ) {
        let Some((amount, time, _)) = self.pending_hdv_kama_gain.take() else {
            return;
        };
        let occurred_at_ms = self.date_tracker.full_timestamp_ms(&time);
        sync_events.push(build_purchase_sync_event(
            HDV_KAMAS_SALE_ITEM,
            0,
            amount,
            &time,
            occurred_at_ms,
            ctx,
        ));
    }

    /// Résout l'assignation de donjon d'un combat qui vient de se terminer — miroir de
    /// `HistorySyncService.resolveDungeonAssignment` (voir la doc de tête de `dungeon_run.rs`) :
    /// 1. Regroupe TOUS les combats connus de la session (`self.fights`, non `ongoing`, plus
    ///    récent en premier — voir `group_dungeon_runs`) et cherche si `fight_id` appartient à un
    ///    run multi-salles tout juste complété.
    /// 2. Si oui : renvoie `(Some((dungeonId, None)), siblings)` — `None` en 2ᵉ position signale à
    ///    `build_fight_sync_event` d'utiliser la signature de CE combat comme graine du run (il en
    ///    est le représentant, voir la doc de tête du module) ; `siblings` liste les AUTRES combats
    ///    du run (salles/tentatives déjà envoyées sans rattachement), à renvoyer à leur tour.
    /// 3. Sinon : repli sur la résolution PROPRE à ce seul combat (`find_dungeon_for_enemies`,
    ///    couvre aussi les donjons à un seul combat — brèche, arcade... — qui ne passent jamais
    ///    par le regroupement multi-salles) — `siblings` toujours vide dans ce cas.
    ///
    /// `None` sans même essayer si le catalogue ou le référentiel de donjons ne sont pas encore
    /// chargés (`ctx.catalog`/`ctx.dungeons`) — jamais une erreur, voir la doc de `ApplyContext`.
    fn resolve_dungeon_assignment(
        &self,
        fight_id: i64,
        ctx: ApplyContext<'_>,
    ) -> (Option<(i64, Option<String>)>, Vec<i64>) {
        let (Some(catalog), Some(dungeons)) = (ctx.catalog, ctx.dungeons) else {
            return (None, Vec::new());
        };

        let mut records: Vec<GroupableFight> = self
            .fights
            .values()
            .filter(|f| !f.snapshot.ongoing)
            .map(|f| GroupableFight {
                id: f.snapshot.fight_id,
                won: matches!(f.snapshot.result, Some(FightResult::Won)),
            })
            .collect();
        // Plus récent en premier — même convention que `HistoryArchiveService.mergedFights`
        // côté web, requise par `group_dungeon_runs` (voir sa doc).
        records.sort_by_key(|r| std::cmp::Reverse(r.id));

        let find_dungeon = |id: i64| -> Option<&DungeonEntry> {
            let fight = self.fights.get(&id)?;
            find_dungeon_for_enemies(catalog, dungeons, &fight_enemy_names(fight))
        };
        let has_archi_enemy = |id: i64| -> bool {
            self.fights.get(&id).is_some_and(|fight| {
                fight_enemy_names(fight)
                    .iter()
                    .any(|name| catalog.find_monster_is_archi(name, None))
            })
        };
        let room_composition_key = |id: i64| -> String {
            self.fights
                .get(&id)
                .map(|fight| enemy_composition_key(&fight_enemy_names(fight)))
                .unwrap_or_default()
        };

        let entries = group_dungeon_runs(
            &records,
            find_dungeon,
            has_archi_enemy,
            room_composition_key,
        );

        let run = entries.iter().find_map(|found| match found {
            DungeonHistoryEntry::DungeonRun { dungeon, fight_ids }
                if fight_ids.contains(&fight_id) =>
            {
                Some((*dungeon, fight_ids.clone()))
            }
            _ => None,
        });

        match run {
            Some((dungeon, fight_ids)) => {
                let siblings = fight_ids.into_iter().filter(|id| *id != fight_id).collect();
                (Some((dungeon.id, None)), siblings)
            }
            None => {
                let own = self
                    .fights
                    .get(&fight_id)
                    .and_then(|fight| {
                        find_dungeon_for_enemies(catalog, dungeons, &fight_enemy_names(fight))
                    })
                    .map(|dungeon| (dungeon.id, None));
                (own, Vec::new())
            }
        }
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

    /// Pose `FighterDamage::is_ko = true` sur CHAQUE ligne de `fight.snapshot.fighters` dont le nom
    /// correspond (comparaison insensible à la casse, même règle que `resolved_enemies`/
    /// `fled_names`) — toutes les instances d'un nom ambigu (homonymes, limite déjà assumée
    /// ailleurs dans ce module, voir `EntityClassifierService` côté web) plutôt qu'une seule au
    /// hasard. No-op si le combat n'est pas suivi ou si `name` ne correspond à aucun combattant
    /// déjà rejoint (ex. `FighterJoined` manqué, combat déjà en cours à l'ouverture du fichier) —
    /// même défense que `mark_resolved`.
    fn mark_ko(&mut self, fight_id: i64, name: &str) {
        let Some(fight) = self.fights.get_mut(&fight_id) else {
            return;
        };
        let lower = name.to_lowercase();
        for fighter in &mut fight.snapshot.fighters {
            if fighter.name.to_lowercase() == lower {
                fighter.is_ko = true;
            }
        }
    }

    fn ensure_fight(&mut self, fight_id: i64, started_at_ms: i64) {
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
            fled_names: std::collections::HashSet::new(),
            started_at_ms,
            loot: Vec::new(),
            kamas_gained: 0,
            xp_gained_total: 0,
            ended_at_time: None,
            ended_at_ms: None,
            challenges_passed: 0,
            challenges_failed: 0,
        });
    }

    /// Réinjecte un combat encore en cours retrouvé sur disque (voir `fight_store`, appelé une
    /// seule fois par `Engine::with_stores` avant tout premier lot ingéré) — reconstruit
    /// uniquement l'index nom→lignes (`fighter_index`) nécessaire pour que les dégâts/soins À
    /// VENIR continuent de créditer les combattants déjà affichés plutôt que d'en recréer des
    /// doublons ; le reste de l'état d'attribution (file d'initiative, sièges) repart neuf — voir
    /// la doc de module de `fight_store` pour le raisonnement complet de cet écart assumé.
    fn restore_fight(&mut self, fight: FightSnapshot) {
        let fight_id = fight.fight_id;
        let mut fighter_index: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, fighter) in fight.fighters.iter().enumerate() {
            fighter_index
                .entry(fighter.name.clone())
                .or_default()
                .push(idx);
        }
        self.fights.insert(
            fight_id,
            FightWorking {
                snapshot: fight,
                fighter_index,
                resolved_enemies: std::collections::HashSet::new(),
                initiative_seats: Vec::new(),
                initiative_cursor: 0,
                last_turn_actor: None,
                last_resolved_seat_by_name: HashMap::new(),
                summon_names: std::collections::HashSet::new(),
                fled_names: std::collections::HashSet::new(),
                // Pas persisté par `fight_store` (voir `FightSnapshot`) : `0` (epoch 1970) pour un
                // combat restauré, comme documenté sur `FightWorking::started_at_ms` — repli
                // sans conséquence pratique (combat déjà entamé avant le redémarrage de l'overlay,
                // cas rare).
                started_at_ms: 0,
                loot: Vec::new(),
                kamas_gained: 0,
                xp_gained_total: 0,
                // Un combat restauré est TOUJOURS `ongoing` (voir `fight_store::save_fight`, qui ne
                // persiste jamais un combat déjà terminé) : jamais de vraie fin à restaurer ici.
                ended_at_time: None,
                ended_at_ms: None,
                challenges_passed: 0,
                challenges_failed: 0,
            },
        );
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
            xp_gained: 0,
            spells: HashMap::new(),
            is_ko: false,
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
        // `0` (epoch 1970) plutôt qu'un horodatage réel : ce chemin est un pur filet défensif (un
        // combat déjà en cours au moment de la connexion, jamais vu `FighterJoined`), voir la doc
        // ci-dessus — `FightWorking::started_at_ms` documente ce repli, sans conséquence pratique.
        self.ensure_fight(fight_id, 0);

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

/// Construit l'événement d'historique d'un achat (marchand/HDV classique, ou récupération de
/// kamas HDV via `HDV_KAMAS_SALE_ITEM`) — factorisé entre les deux appelants (`SessionState::
/// apply`, préambule achat ; `flush_pending_hdv_kama_gain`) : même construction de payload, seule
/// la provenance de `item`/`quantity`/`total_cost` diffère. Miroir de `HistorySyncService.
/// recordPurchase`.
fn build_purchase_sync_event(
    item: &str,
    quantity: i64,
    total_cost: i64,
    time: &str,
    occurred_at_ms: i64,
    ctx: ApplyContext<'_>,
) -> SyncEvent {
    let signature = purchase_signature(time, item, quantity, total_cost);
    let (item_id, item_name) = item_payload(ctx.catalog, item);
    SyncEvent {
        kind: HistoryEventKind::Purchase,
        signature,
        payload: HistoryPayload::Purchase(PurchasePayload {
            item_id,
            item_name,
            quantity,
            total_cost,
            occurred_at: format_iso_utc(occurred_at_ms),
            game_server: ctx.game_server.map(str::to_string),
        }),
    }
}

/// Noms des ennemis d'un combat, dans l'ordre de jonction — entrée commune de
/// `dungeon_run::find_dungeon_for_enemies`/`enemy_composition_key`/`CatalogIndex::
/// find_monster_is_archi` partout où ce module en a besoin (résolution de donjon, regroupement de
/// runs). Facturé une seule fois plutôt que dupliqué à chaque site d'appel.
fn fight_enemy_names(fight: &FightWorking) -> Vec<String> {
    fight
        .snapshot
        .fighters
        .iter()
        .filter(|f| !f.is_ally)
        .map(|f| f.name.clone())
        .collect()
}

/// Paires `(nom, instanceIndex)` dans l'ordre de jonction — même calcul que celui fait pour
/// `FightParticipantPayload` dans `build_fight_sync_event` (`instance_seen`), mais isolé ici pour
/// pouvoir obtenir la signature d'un combat SANS construire tout son payload — nécessaire pour
/// calculer la signature du combat REPRÉSENTATIF d'un run avant de rebâtir ses éventuels siblings
/// (voir `SessionState::resolve_dungeon_assignment`).
fn fight_signature_participants(fight: &FightWorking) -> Vec<(String, i64)> {
    let mut instance_seen: HashMap<String, i64> = HashMap::new();
    fight
        .snapshot
        .fighters
        .iter()
        .map(|fighter| {
            let counter = instance_seen.entry(fighter.name.clone()).or_insert(0);
            let idx = *counter;
            *counter += 1;
            (fighter.name.clone(), idx)
        })
        .collect()
}

/// Signature de contenu d'UN combat, indépendamment de son assignation de donjon — miroir de
/// `HistorySyncService.runSignature`. Recalculable pour n'importe quel combat encore présent dans
/// `SessionState::fights`, à condition de connaître son heure de fin et son résultat (voir
/// `FightWorking::ended_at_time`) — c'est ce qui permet de retrouver, longtemps après coup, LA MÊME
/// chaîne que celle produite à l'origine par `build_fight_sync_event` pour ce combat.
fn fight_own_signature(fight: &FightWorking, time: &str, won: bool) -> String {
    let participants = fight_signature_participants(fight);
    fight_signature(time, fight.snapshot.fight_id, won, &participants)
}

/// Construit l'événement d'historique d'un combat terminé — appelé pour le combat qui vient de se
/// terminer (`CombatEnd`, voir `SessionState::apply`) ET pour chaque SIBLING d'un run de donjon
/// tout juste découvert (voir `resolve_dungeon_assignment`) : `time`/`won`/`end_ms` sont alors ceux
/// D'ORIGINE de ce sibling (voir `FightWorking::ended_at_time`/`ended_at_ms`), jamais ceux du
/// combat qui a déclenché le nouveau regroupement.
///
/// `dungeon` est résolu par l'APPELANT, jamais recalculé ici (voir `resolve_dungeon_assignment`) :
/// `None` hors donjon ; `Some((id, None))` pour un combat qui contient LUI-MÊME son donjon (sa
/// propre signature, calculée plus bas, sert alors de graine de run) ; `Some((id, Some(sig)))`
/// pour un combat appartenant à un run dont un AUTRE combat (le boss, plus récent) est le
/// représentant — `sig` est alors LA signature de ce représentant, partagée par tout le run.
///
/// `instanceIndex` recalculé ici par ordre de jonction (le Nᵉ combattant partageant ce nom obtient
/// l'index N-1) : suffisant pour distinguer des homonymes dans la signature/le payload, sans
/// dépendre de `FightWorking::fighter_index` (déjà utilisé pour un besoin différent, l'attribution
/// des dégâts par siège d'initiative — voir sa doc). `turns` reste à `0`, volontairement (voir la
/// doc de module de `history.rs`) : rien ici ne compte les tours, aucun consommateur overlay n'en
/// a besoin.
fn build_fight_sync_event(
    fight: &FightWorking,
    time: &str,
    won: bool,
    end_ms: i64,
    ctx: ApplyContext<'_>,
    dungeon: Option<(i64, Option<String>)>,
) -> SyncEvent {
    let mut instance_seen: HashMap<String, i64> = HashMap::new();
    let mut sig_participants: Vec<(String, i64)> =
        Vec::with_capacity(fight.snapshot.fighters.len());
    let participants: Vec<FightParticipantPayload> = fight
        .snapshot
        .fighters
        .iter()
        .map(|fighter| {
            let counter = instance_seen.entry(fighter.name.clone()).or_insert(0);
            let instance_index = *counter;
            *counter += 1;
            sig_participants.push((fighter.name.clone(), instance_index));

            let (defeated, fled) = if fighter.is_ally {
                (false, false)
            } else {
                let lower = fighter.name.to_lowercase();
                let fled = fight.fled_names.contains(&lower);
                let defeated = !fled && fight.resolved_enemies.contains(&lower);
                (defeated, fled)
            };
            // Un allié n'a jamais d'id monstre (aucun monstre ne porte un nom de personnage) —
            // miroir de `HistorySyncService.monsterId`, appelé côté web UNIQUEMENT pour `side ===
            // 'enemy'`.
            let monster_id = if fighter.is_ally {
                None
            } else {
                ctx.catalog.and_then(|c| c.find_monster_id(&fighter.name))
            };
            // Ventilation par sort/élément — voir `FighterDamage::spells`. Triée par nom de sort
            // pour un résultat déterministe (HashMap n'a pas d'ordre stable), utile aux tests et
            // sans aucune importance pour le serveur (simple liste, jamais indexée par position).
            let mut spells: Vec<FightSpellPayload> = fighter
                .spells
                .iter()
                .map(|(spell, by_element)| FightSpellPayload {
                    spell: spell.clone(),
                    total: by_element.values().sum(),
                    by_element: by_element.clone(),
                })
                .collect();
            spells.sort_by(|a, b| a.spell.cmp(&b.spell));
            FightParticipantPayload {
                side: if fighter.is_ally {
                    FightSide::Ally
                } else {
                    FightSide::Enemy
                },
                name: fighter.name.clone(),
                monster_id,
                instance_index,
                class_name: fighter.class_name.clone(),
                damage: fighter.total_damage,
                defeated,
                fled,
                spells,
                xp_gained: fighter.xp_gained,
            }
        })
        .collect();

    // Dégâts de l'équipe du joueur : la somme des lignes classées alliées, jamais les deux camps
    // ensemble (mélangerait dégâts subis et infligés) — miroir exact de `HistorySyncService.
    // recordFight` (`totalDamage`) côté web.
    let total_damage: i64 = participants
        .iter()
        .filter(|p| p.side == FightSide::Ally)
        .map(|p| p.damage)
        .sum();
    let loot = fight
        .loot
        .iter()
        .map(|(name, quantity)| {
            let (item_id, item_name) = item_payload(ctx.catalog, name);
            FightLootPayload {
                item_id,
                item_name,
                quantity: *quantity,
            }
        })
        .collect();
    let signature = fight_signature(time, fight.snapshot.fight_id, won, &sig_participants);
    let duration_ms = (end_ms - fight.started_at_ms).max(0);

    // `dungeon_run_signature` : quand aucune graine n'est forcée par l'appelant (`Some((id,
    // None))` — ce combat est son PROPRE représentant, voir la doc ci-dessus), sa graine est
    // directement SA PROPRE signature, jamais recalculée séparément.
    let (dungeon_id, dungeon_run_signature) = match dungeon {
        Some((id, Some(run_signature))) => (Some(id), Some(run_signature)),
        Some((id, None)) => (Some(id), Some(signature.clone())),
        None => (None, None),
    };

    SyncEvent {
        kind: HistoryEventKind::Fight,
        signature,
        payload: HistoryPayload::Fight(FightPayload {
            fight_id: Some(fight.snapshot.fight_id),
            started_at: format_iso_utc(fight.started_at_ms),
            duration_ms: Some(duration_ms),
            won,
            turns: 0,
            total_damage,
            xp_gained: fight.xp_gained_total,
            kamas_gained: Some(fight.kamas_gained),
            game_server: ctx.game_server.map(str::to_string),
            dungeon_id,
            dungeon_run_signature,
            challenges_passed: fight.challenges_passed,
            challenges_failed: fight.challenges_failed,
            participants,
            loot,
        }),
    }
}

/// Construit l'événement d'historique d'un échange — miroir de `registerTrade`
/// (`stats-store.service.ts`) : le personnage "en face" (`peerName`) est celui des deux côtés qui
/// n'appartient PAS au roster déclaré (`self_name`) ; `None` si les deux côtés appartiennent au
/// roster (échange entre deux comptes du même joueur, pas un vrai échange avec un tiers) ; repli
/// sur le premier côté comme "soi" si NI L'UN NI L'AUTRE n'est reconnu (roster pas encore chargé,
/// ou personnage pas déclaré) — même choix stable que le web plutôt que de ne rien enregistrer.
fn build_trade_sync_event(
    time: &str,
    sides: &[crate::model::TradeSide; 2],
    occurred_ms: i64,
    ctx: ApplyContext<'_>,
) -> Option<SyncEvent> {
    let [a, b] = sides;
    let a_is_self = ctx.roster.is_some_and(|r| r.find(&a.player_name).is_some());
    let b_is_self = ctx.roster.is_some_and(|r| r.find(&b.player_name).is_some());
    if a_is_self && b_is_self {
        return None;
    }
    let (self_side, other_side) = if b_is_self { (b, a) } else { (a, b) };

    let mut items = Vec::with_capacity(self_side.items.len() + other_side.items.len());
    let mut sig_items = Vec::with_capacity(items.capacity());
    for item in &other_side.items {
        let (item_id, item_name) = item_payload(ctx.catalog, &item.name);
        items.push(TradeItemPayload {
            direction: TradeDirection::Acquired,
            item_id,
            item_name,
            quantity: item.quantity,
        });
        sig_items.push((TradeDirection::Acquired, item.name.clone(), item.quantity));
    }
    for item in &self_side.items {
        let (item_id, item_name) = item_payload(ctx.catalog, &item.name);
        items.push(TradeItemPayload {
            direction: TradeDirection::Given,
            item_id,
            item_name,
            quantity: item.quantity,
        });
        sig_items.push((TradeDirection::Given, item.name.clone(), item.quantity));
    }

    let signature = trade_signature(
        time,
        &other_side.player_name,
        &self_side.player_name,
        other_side.kamas,
        self_side.kamas,
        &sig_items,
    );

    Some(SyncEvent {
        kind: HistoryEventKind::Trade,
        signature,
        payload: HistoryPayload::Trade(TradePayload {
            peer_name: other_side.player_name.clone(),
            self_name: self_side.player_name.clone(),
            occurred_at: format_iso_utc(occurred_ms),
            kamas_acquired: other_side.kamas,
            kamas_given: self_side.kamas,
            game_server: ctx.game_server.map(str::to_string),
            items,
        }),
    })
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
    /// Catalogue (lot L3) — PORTÉ PAR `Engine`, même raison que `roster` ci-dessus. Copie
    /// indépendante de celle relayée à `self.parser` (`quickjs_engine::LogParserEngine::catalog`,
    /// interne, utilisée uniquement pour `hostIsKnownMonsterName`) : celle-ci sert à résoudre
    /// `monsterId`/`itemId`/`dungeonId` (L5, §7.1) au moment de construire un `SyncEvent`, un
    /// besoin que `SessionState` (recréé à chaque rattrapage) ne peut pas porter lui-même.
    catalog: Option<std::sync::Arc<CatalogIndex>>,
    /// Référentiel des donjons — voir `set_dungeons`. `None` tant que rien ne l'a chargé : c'est
    /// le cas de PRODUCTION aujourd'hui (voir la doc de `set_dungeons`), pas une erreur — laisse
    /// simplement `dungeon_id`/`dungeon_run_signature` à `None` sur tout `FightPayload` construit
    /// (voir `build_fight_sync_event`, qui tolère déjà ce champ absent).
    dungeons: Option<std::sync::Arc<DungeonIndex>>,
    /// Dernier personnage du roster déclaré reconnu dans le log (allié confirmé à un
    /// `FighterJoined`, ou côté "soi" d'un `TradeCompleted`) — miroir de `GameServerService.
    /// lastKnownCharacter` : PORTÉ PAR `Engine`, jamais recréé avec `SessionState` (une rotation
    /// mid-session ne doit pas faire disparaître le serveur déjà déduit avant elle). Voir
    /// `current_game_server`.
    last_known_character: Option<String>,
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
    /// Objets à son activé au ramassage (compte, §9 du plan « Alertes de drop » — cas
    /// `reason: 'loot'`, voir `profile.rs`) — même raison qu'au-dessus (`roster`/`watchlist`) :
    /// PORTÉ PAR `Engine`, jamais recréé avec `SessionState`. Indépendant de `watchlist` : ce
    /// n'est PAS la liste suivie, un objet peut avoir son son activé sans être suivi et
    /// réciproquement.
    sound_items: Vec<crate::profile::SoundItemEntry>,
    /// Alertes de ramassage (voir `LootAlert`) accumulées depuis le dernier `drain_loot_alerts` —
    /// même motif « drain » que `pending_alerts`, file SÉPARÉE : les deux mécanismes sont
    /// indépendants côté web (`LootAlertService` reçoit les deux, mais depuis deux déclencheurs
    /// distincts, voir `profile.rs`).
    pending_loot_alerts: Vec<crate::profile::LootAlert>,
    /// Événements d'historique (combat/achat/échange) prêts à synchroniser, accumulés depuis le
    /// dernier `drain_sync_events` (L5, §7.1) — même motif « drain » que `pending_alerts`/
    /// `pending_loot_alerts`, file SÉPARÉE : l'hôte (`overlay-ui`) les relaie tels quels au thread
    /// Sync (§3 du plan, `overlay_sync::queue`), qui ne connaît que des payloads déjà sérialisés
    /// et ne rappelle jamais l'Engine. Contrairement aux deux files ci-dessus, alimentée aussi
    /// pendant un rattrapage (`is_initial_load`) : un combat/achat/échange déjà présent dans le
    /// fichier doit être renvoyé à chaque reconnexion (idempotence côté serveur via `clientKey`,
    /// voir la doc de `history.rs`), exactement comme `HistorySyncService` le fait côté web après
    /// `resetSessionState()`.
    pending_sync_events: Vec<SyncEvent>,
    /// Dossier de persistance des combats encore en cours (§9 du plan, voir `fight_store.rs`) —
    /// un fichier `fight-{fight_id}.json` par combat `ongoing`, mis à jour à chaque lot qui le
    /// touche (voir `ingest_batch`) et supprimé à sa fin (`CombatEnd`). Contrairement à `roster`/
    /// `watchlist`, ne porte aucun état en mémoire : simple chemin utilisé en lecture/écriture.
    fight_store_dir: PathBuf,
}

impl Engine {
    pub fn new() -> Result<Self, crate::quickjs_engine::EngineError> {
        Self::with_stores(
            crate::watchlist::default_store_path(),
            crate::fight_store::default_store_dir(),
        )
    }

    /// Comme `new()`, avec un chemin de compteurs de suivi explicite (dossier de combats en cours
    /// laissé au défaut de production, voir `with_stores` pour un contrôle complet) — **PUBLIC MAIS
    /// RÉSERVÉ AUX TESTS D'INTÉGRATION** (`tests/*.rs`). `#[cfg(test)]` sur `watchlist::APP_NAME`
    /// ne protège que les tests INTERNES de ce crate (`src/watchlist.rs::tests`, compilés avec
    /// `cfg(test)` actif pour `overlay-engine` lui-même) : un test d'intégration est un crate
    /// SÉPARÉ qui dépend d'`overlay-engine` normalement, sans `cfg(test)` actif pour lui —
    /// `Engine::new()` y résout donc le VRAI chemin de production. Bug réel vécu en session
    /// (2026-09-01) : un test d'intégration a écrasé le fichier de compteurs réel de l'utilisateur
    /// (compteurs d'objets perdus) avant que ce constructeur n'existe — toujours passer par lui
    /// (avec un chemin de fichier temporaire) depuis `tests/`, jamais `Engine::new()`.
    pub fn with_watchlist_store(
        store_path: std::path::PathBuf,
    ) -> Result<Self, crate::quickjs_engine::EngineError> {
        Self::with_stores(store_path, crate::fight_store::default_store_dir())
    }

    /// Comme `new()`, avec un chemin de compteurs de suivi ET un dossier de combats en cours
    /// explicites — à utiliser depuis `tests/*.rs` dès qu'un test laisse un combat `ongoing` à la
    /// fin (voir la doc de `with_watchlist_store` : un test d'intégration qui appellerait `new()`
    /// ou `with_watchlist_store()` seul écrirait tout de même un `fight-{fight_id}.json` dans le
    /// VRAI dossier de combats de production).
    ///
    /// Restaure ici, avant tout premier lot ingéré, les combats encore persistés depuis un
    /// précédent process (voir `fight_store::load_ongoing_fights` et `SessionState::restore_fight`)
    /// — s'il y en a au moins un, `state_initialized` démarre à `true` : le tout premier lot ingéré
    /// (forcément marqué `is_initial_load`, que ce soit un vrai premier rattrapage ou la reprise
    /// après une rotation survenue overlay éteint) ne doit PAS vider `state`, exactement comme une
    /// rotation survenue en cours de process (voir la doc de `state_initialized`) — sinon la
    /// restauration serait immédiatement écrasée par le tout premier `ingest_batch`.
    pub fn with_stores(
        store_path: std::path::PathBuf,
        fight_store_dir: std::path::PathBuf,
    ) -> Result<Self, crate::quickjs_engine::EngineError> {
        let mut state = SessionState::default();
        let restored_fights = crate::fight_store::load_ongoing_fights(&fight_store_dir);
        let has_restored_fights = !restored_fights.is_empty();
        for fight in restored_fights {
            state.restore_fight(fight);
        }
        Ok(Self {
            parser: crate::quickjs_engine::LogParserEngine::new()?,
            state,
            in_initial_sweep: false,
            state_initialized: has_restored_fights,
            roster: None,
            catalog: None,
            dungeons: None,
            last_known_character: None,
            watchlist: WatchlistState::new(store_path),
            pending_alerts: Vec::new(),
            sound_items: Vec::new(),
            pending_loot_alerts: Vec::new(),
            pending_sync_events: Vec::new(),
            fight_store_dir,
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
        // Copie propre à `Engine`, distincte de celle relayée au parser juste après — voir la doc
        // du champ `catalog` ci-dessus (L5, résolution `monsterId`/`itemId`/`dungeonId`).
        self.catalog = Some(std::sync::Arc::clone(&catalog));
        self.parser.set_catalog(catalog);
    }

    /// Remplace le référentiel des donjons consulté pour l'assignation `dungeonId` (L5, §7.1 —
    /// voir `build_fight_sync_event`). **Non appelé en production aujourd'hui** : aucun code de
    /// `overlay-ui` ne charge encore `GET /api/v1/dungeons` (contrairement au catalogue) — brancher
    /// ce chargement est un petit lot dédié, volontairement laissé de côté ici pour ne toucher
    /// aucun fichier `overlay-ui` dans ce lot-ci (décision du mainteneur, 2026-09-02). Tant que
    /// cette méthode n'est pas appelée, `dungeon_id`/`dungeon_run_signature` restent `None` sur
    /// tout `FightPayload` construit — pas une régression, un statut assumé, comme `DungeonIndex`
    /// lui-même l'était déjà pour L3 (voir §12 du plan, statut L3 : « aucun panneau n'en a besoin
    /// aujourd'hui »).
    pub fn set_dungeons(&mut self, dungeons: std::sync::Arc<DungeonIndex>) {
        self.dungeons = Some(dungeons);
    }

    /// Code du serveur de jeu déduit du dernier personnage du roster reconnu dans le log — miroir
    /// de `GameServerService.activeServer` (voir `RosterIndex::find_game_server`). `None` tant
    /// qu'aucun personnage du roster n'a encore été reconnu (ou que son compte n'a pas de serveur
    /// déclaré) — jamais une valeur inventée, exactement comme côté web.
    fn current_game_server(&self) -> Option<String> {
        let roster = self.roster.as_ref()?;
        let character = self.last_known_character.as_deref()?;
        roster.find_game_server(character)
    }

    /// Met à jour `last_known_character` (voir `current_game_server`) si CETTE ligne notifie un
    /// personnage du roster déclaré — miroir de `GameServerService.noticeCharacter`, appelé côté
    /// web depuis `registerFighterJoin` (alliés confirmés uniquement, `!isControlledByAI`) et
    /// `registerTrade` (le côté "soi" de l'échange). Sans effet si aucun roster n'est connu, ou si
    /// aucun nom de cette ligne ne correspond à un personnage déclaré.
    fn notice_character(&mut self, entry: &LogEntry) {
        let Some(roster) = self.roster.as_ref() else {
            return;
        };
        match entry {
            LogEntry::FighterJoined {
                name,
                is_controlled_by_ai: false,
                ..
            } if roster.find(name).is_some() => {
                self.last_known_character = Some(name.clone());
            }
            LogEntry::TradeCompleted { sides, .. } => {
                for side in sides {
                    if roster.find(&side.player_name).is_some() {
                        self.last_known_character = Some(side.player_name.clone());
                    }
                }
            }
            _ => {}
        }
    }

    /// Remplace la liste des entrées suivies par celle renvoyée par le compte (voir
    /// `watchlist_from_settings_json`), en conservant les compteurs locaux déjà en cours pour
    /// toute entrée déjà connue (voir `WatchlistState::merge_config`). Appelé par l'hôte au même
    /// moment que `set_roster` — même source `GET /api/v1/settings`.
    pub fn set_watchlist_entries(&mut self, entries: Vec<WatchlistEntry>) {
        self.watchlist.merge_config(entries);
    }

    /// Remplace la liste des objets à son activé au ramassage par celle renvoyée par le compte
    /// (voir `profile::sound_items_from_settings_json`) — appelé par l'hôte au même moment que
    /// `set_roster`/`set_watchlist_entries` (même source `GET /api/v1/settings`). Contrairement à
    /// `set_watchlist_entries`, rien à conserver d'une précédente valeur : cette liste ne porte
    /// aucun état local (pas de compteur), un simple remplacement suffit.
    pub fn set_sound_items(&mut self, items: Vec<crate::profile::SoundItemEntry>) {
        self.sound_items = items;
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

    /// Vide et renvoie les alertes de ramassage (son activé) accumulées depuis le dernier appel
    /// (voir `pending_loot_alerts`) — même usage que `drain_watchlist_alerts`, file séparée (voir
    /// la doc de `pending_loot_alerts`). Vide dans l'immense majorité des appels.
    pub fn drain_loot_alerts(&mut self) -> Vec<crate::profile::LootAlert> {
        std::mem::take(&mut self.pending_loot_alerts)
    }

    /// Vide et renvoie les événements d'historique accumulés depuis le dernier appel (voir
    /// `pending_sync_events`) — à appeler par l'hôte après chaque `ingest_batch` pour les relayer
    /// au thread Sync (§7.1 du plan, L5). Un lot volumineux peut en produire beaucoup d'un coup
    /// (rattrapage initial d'un `wakfu.log` déjà rempli d'historique).
    pub fn drain_sync_events(&mut self) -> Vec<SyncEvent> {
        std::mem::take(&mut self.pending_sync_events)
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
        // Combats touchés par CE lot (voir `entry_fight_id`) — persistés/supprimés sur disque une
        // fois le lot entier appliqué (§9 du plan, `fight_store.rs`), pas ligne par ligne : un
        // combat encaisse potentiellement des dizaines de lignes par lot, une écriture disque par
        // ligne serait un gaspillage sans rien apporter (seul l'état final du lot compte pour la
        // restauration après redémarrage).
        let mut touched_fight_ids = std::collections::HashSet::new();
        for entry in &entries {
            if let Some(fight_id) = entry_fight_id(entry) {
                touched_fight_ids.insert(fight_id);
            }
            self.notice_character(entry);
            let game_server = self.current_game_server();
            // Construit `ctx` en accédant directement aux champs de `self` (pas via une méthode
            // `&self`) : le vérificateur d'emprunts ne sait raisonner sur des champs disjoints
            // (`roster`/`catalog`/`dungeons`, immuables) qu'à ce niveau — masqués derrière un
            // appel de méthode, ils entreraient en conflit avec l'emprunt mutable de `self.state`
            // juste en dessous.
            let ctx = ApplyContext {
                roster: self.roster.as_ref(),
                catalog: self.catalog.as_deref(),
                dungeons: self.dungeons.as_deref(),
                game_server: game_server.as_deref(),
            };
            let mut new_sync_events = Vec::new();
            let implicitly_defeated = self.state.apply(entry, ctx, &mut new_sync_events);
            self.pending_sync_events.extend(new_sync_events);
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
                // Miroir de `registerLoot` (`stats-store.service.ts`), même gating
                // `currentBatchIsInitialLoad` que ci-dessus — indépendant de la watchlist (voir la
                // doc de `profile.rs`) : déclenché pour TOUT ramassage dont le nom a son activé au
                // compte, suivi ou non.
                if let LogEntry::Loot { item, quantity, .. } = entry {
                    if let Some(sound_entry) =
                        crate::profile::find_enabled_sound_item(&self.sound_items, item)
                    {
                        self.pending_loot_alerts.push(crate::profile::LootAlert {
                            name: item.clone(),
                            quantity: *quantity,
                            catalog_id: sound_entry.catalog_id,
                        });
                    }
                }
            }
        }
        // Un gain de kamas hors combat encore en attente de confirmation en fin de LOT n'a plus de
        // ligne suivante à attendre dans l'immédiat (la prochaine pourrait tarder, voire ne jamais
        // arriver avant une reconnexion) — committé maintenant plutôt que risqué de le perdre.
        // Miroir de `flushPendingHdvKamaGain()`, appelé au même moment côté web (fin d'`ingest()`).
        {
            let game_server = self.current_game_server();
            let ctx = ApplyContext {
                roster: self.roster.as_ref(),
                catalog: self.catalog.as_deref(),
                dungeons: self.dungeons.as_deref(),
                game_server: game_server.as_deref(),
            };
            let mut flush_events = Vec::new();
            self.state
                .flush_pending_hdv_kama_gain(ctx, &mut flush_events);
            self.pending_sync_events.extend(flush_events);
        }
        // Voir la doc de `touched_fight_ids` ci-dessus : un combat absent de `self.state.fights`
        // ici a forcément déjà été purgé (`SessionState::prune_ended_fights`, sur `CombatEnd`) —
        // son fichier n'a alors plus rien à faire sur disque non plus.
        for fight_id in touched_fight_ids {
            match self.state.fights.get(&fight_id) {
                Some(fight) if fight.snapshot.ongoing => {
                    crate::fight_store::save_fight(&self.fight_store_dir, &fight.snapshot);
                }
                _ => crate::fight_store::delete_fight(&self.fight_store_dir, fight_id),
            }
        }
        Ok(entries)
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        self.state.snapshot()
    }
}

/// `fight_id` porté par un `LogEntry`, quand la variante en a un — miroir du besoin de
/// `Engine::ingest_batch` pour savoir quels combats persister/supprimer sur disque après un lot
/// (voir `fight_store.rs`). Volontairement séparé de `LogEntry::time()` (déjà exhaustif pour un
/// besoin différent) : toutes les variantes n'ont pas de `fight_id`, `None` ici couvre à la fois
/// « pas de notion de combat » (chat, échange...) et « combat non résolu par le parser ».
fn entry_fight_id(entry: &LogEntry) -> Option<i64> {
    match entry {
        LogEntry::FighterJoined { fight_id, .. } | LogEntry::CombatEnd { fight_id, .. } => {
            Some(*fight_id)
        }
        LogEntry::KamaGain { fight_id, .. }
        | LogEntry::XpGain { fight_id, .. }
        | LogEntry::SpellCast { fight_id, .. }
        | LogEntry::Damage { fight_id, .. }
        | LogEntry::Heal { fight_id, .. }
        | LogEntry::Armor { fight_id, .. }
        | LogEntry::EnemyDefeated { fight_id, .. }
        | LogEntry::EnemyFled { fight_id, .. }
        | LogEntry::CombatDefeatMarker { fight_id, .. }
        | LogEntry::Loot { fight_id, .. }
        | LogEntry::ChallengeResult { fight_id, .. } => *fight_id,
        LogEntry::Chat { .. }
        | LogEntry::KamaLoss { .. }
        | LogEntry::CombatStart { .. }
        | LogEntry::MarketOccupation { .. }
        | LogEntry::LogDateAnchor { .. }
        | LogEntry::TradeCompleted { .. } => None,
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
        state.apply(
            &fighter_joined(1, "Oumbra", 1, false),
            ApplyContext {
                roster: Some(&roster),
                ..Default::default()
            },
            &mut Vec::new(),
        );

        let fighter = &state.fights[&1].snapshot.fighters[0];
        assert_eq!(fighter.class_name.as_deref(), Some("cra"));
        assert_eq!(fighter.gender, Gender::F);
    }

    #[test]
    fn repli_sur_breed_si_absent_du_roster() {
        let mut state = SessionState::default();
        let roster = roster_with("QuelquUnDAutre", "sram", Gender::F);
        state.apply(
            &fighter_joined(1, "Oumbra", 8, false),
            ApplyContext {
                roster: Some(&roster),
                ..Default::default()
            },
            &mut Vec::new(),
        ); // breed 8 = iop

        let fighter = &state.fights[&1].snapshot.fighters[0];
        assert_eq!(fighter.class_name.as_deref(), Some("iop"));
        assert_eq!(fighter.gender, Gender::M); // repli par défaut, pas d'info de sexe via breed
    }

    #[test]
    fn aucune_classe_sans_roster_ni_breed_connu() {
        let mut state = SessionState::default();
        state.apply(
            &fighter_joined(1, "Oumbra", 0, false),
            ApplyContext::default(),
            &mut Vec::new(),
        ); // breed 0 : inconnu

        let fighter = &state.fights[&1].snapshot.fighters[0];
        assert_eq!(fighter.class_name, None);
    }

    #[test]
    fn un_ennemi_na_jamais_de_classe_meme_avec_un_breed_valide() {
        let mut state = SessionState::default();
        let roster = roster_with("Monstre", "iop", Gender::M); // ne doit jamais s'appliquer
        state.apply(
            &fighter_joined(1, "Monstre", 1, true),
            ApplyContext {
                roster: Some(&roster),
                ..Default::default()
            },
            &mut Vec::new(),
        ); // breed 1 = feca, is_controlled_by_ai=true

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
        state.apply(
            &fighter_joined(1, "PersoA", 9, false),
            ApplyContext::default(),
            &mut Vec::new(),
        ); // fight 1, cra
        state.apply(
            &fighter_joined(2, "PersoB", 4, false),
            ApplyContext::default(),
            &mut Vec::new(),
        ); // fight 2, sram
        state.apply(
            &damage(1, "PersoA", 100),
            ApplyContext::default(),
            &mut Vec::new(),
        );
        state.apply(
            &damage(2, "PersoB", 50),
            ApplyContext::default(),
            &mut Vec::new(),
        );
        state.apply(
            &damage(1, "PersoA", 25),
            ApplyContext::default(),
            &mut Vec::new(),
        );

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
        state.apply(
            &fighter_joined(1, "Mouton", 1, true),
            ApplyContext::default(),
            &mut Vec::new(),
        ); // 1ʳᵉ instance
        state.apply(
            &fighter_joined(1, "Mouton", 1, true),
            ApplyContext::default(),
            &mut Vec::new(),
        ); // 2ᵉ instance, MÊME nom
        state.apply(
            &fighter_joined(1, "Oumbra", 9, false),
            ApplyContext::default(),
            &mut Vec::new(),
        ); // allié, sert d'intercalaire

        state.apply(
            &spell_cast(1, "Mouton"),
            ApplyContext::default(),
            &mut Vec::new(),
        ); // 1er tour de Mouton -> siège #1
        state.apply(
            &damage(1, "Mouton", 50),
            ApplyContext::default(),
            &mut Vec::new(),
        );
        state.apply(
            &spell_cast(1, "Oumbra"),
            ApplyContext::default(),
            &mut Vec::new(),
        ); // tour intercalaire (autre acteur)
        state.apply(
            &spell_cast(1, "Mouton"),
            ApplyContext::default(),
            &mut Vec::new(),
        ); // tour SUIVANT de Mouton -> siège #2
        state.apply(
            &damage(1, "Mouton", 30),
            ApplyContext::default(),
            &mut Vec::new(),
        );

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
        state.apply(
            &fighter_joined(1, "Oumbra", 9, false),
            ApplyContext::default(),
            &mut Vec::new(),
        ); // allié invocateur
        state.apply(
            &summoned_fighter_joined(1, "Balise de Contact", "Oumbra"),
            ApplyContext::default(),
            &mut Vec::new(),
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
        state.apply(
            &fighter_joined(1, "Oumbra", 9, false),
            ApplyContext::default(),
            &mut Vec::new(),
        );
        state.apply(
            &summoned_fighter_joined(1, "Balise de Contact", "Oumbra"),
            ApplyContext::default(),
            &mut Vec::new(),
        );
        state.apply(
            &damage(1, "Balise de Contact", 999),
            ApplyContext::default(),
            &mut Vec::new(),
        );

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
        state.apply(
            &fighter_joined(1, "Zoroark Shiny", 9, false),
            ApplyContext::default(),
            &mut Vec::new(),
        );
        state.apply(
            &fighter_joined(2, "Canis Furiosus", 4, false),
            ApplyContext::default(),
            &mut Vec::new(),
        );

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
        state.apply(
            &fighter_joined(1, "Zoroark Shiny", 9, false),
            ApplyContext::default(),
            &mut Vec::new(),
        );

        let snapshot = state.snapshot();
        assert_eq!(snapshot.fight_for_character("Personne Ici"), None);
    }

    #[test]
    fn purge_ne_touche_jamais_un_combat_en_cours() {
        let mut state = SessionState::default();
        // MAX_TRACKED_FIGHTS + quelques combats, tous terminés sauf le tout premier (`ongoing`).
        state.apply(
            &fighter_joined(0, "EnCours", 9, false),
            ApplyContext::default(),
            &mut Vec::new(),
        );
        for fight_id in 1..=(MAX_TRACKED_FIGHTS as i64 + 5) {
            state.apply(
                &fighter_joined(fight_id, "Autre", 9, false),
                ApplyContext::default(),
                &mut Vec::new(),
            );
            state.apply(
                &LogEntry::CombatEnd {
                    time: "12:00:02,000".to_string(),
                    fight_id,
                    result: FightResult::Won,
                },
                ApplyContext::default(),
                &mut Vec::new(),
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
        state.apply(
            &fighter_joined(1, "El Pochito", 1, true),
            ApplyContext::default(),
            &mut Vec::new(),
        );
        state.apply(
            &fighter_joined(1, "Oumbra Canin", 15, false),
            ApplyContext::default(),
            &mut Vec::new(),
        );

        let implicitly_defeated = state.apply(
            &combat_end(1, FightResult::Won),
            ApplyContext::default(),
            &mut Vec::new(),
        );

        assert_eq!(implicitly_defeated, vec!["El Pochito".to_string()]);
    }

    #[test]
    fn filet_de_rattrapage_ne_double_compte_pas_un_ennemi_deja_vaincu_explicitement() {
        let mut state = SessionState::default();
        state.apply(
            &fighter_joined(1, "Bwork", 1, true),
            ApplyContext::default(),
            &mut Vec::new(),
        );
        state.apply(
            &enemy_defeated(1, "Bwork"),
            ApplyContext::default(),
            &mut Vec::new(),
        ); // ligne "est KO !" explicite, avant la fin

        let implicitly_defeated = state.apply(
            &combat_end(1, FightResult::Won),
            ApplyContext::default(),
            &mut Vec::new(),
        );

        assert!(
            implicitly_defeated.is_empty(),
            "déjà crédité explicitement, le filet ne doit rien ajouter"
        );
    }

    /// `EnemyDefeated` couvre aussi bien "X est KO !" (allié) que "X est hors-combat !" (n'importe
    /// qui) — voir `FighterDamage::is_ko`. Régression visée : ne pas confondre avec
    /// `resolved_enemies`, qui n'a de sens que pour les ennemis (voir `build_fight_sync_event`).
    #[test]
    fn enemy_defeated_marque_is_ko_meme_pour_un_allie() {
        let mut state = SessionState::default();
        state.apply(
            &fighter_joined(1, "Oumbra", 8, false), // allié confirmé (isControlledByAI=false)
            ApplyContext::default(),
            &mut Vec::new(),
        );
        state.apply(
            &fighter_joined(1, "Bwork", 1, true),
            ApplyContext::default(),
            &mut Vec::new(),
        );

        state.apply(
            &enemy_defeated(1, "Oumbra"), // "Oumbra est KO !"
            ApplyContext::default(),
            &mut Vec::new(),
        );

        let fighters = &state.fights[&1].snapshot.fighters;
        assert!(fighters.iter().find(|f| f.name == "Oumbra").unwrap().is_ko);
        assert!(!fighters.iter().find(|f| f.name == "Bwork").unwrap().is_ko);
    }

    /// Une fuite n'est PAS un KO (`is_ko` doit rester `false`) — contrairement à `resolved_enemies`
    /// (partagé entre les deux), volontairement posé par `EnemyFled` pour empêcher le filet de
    /// rattrapage de `CombatEnd` de créditer à tort un fuyard comme vaincu (voir ce test juste en
    /// dessous, `filet_de_rattrapage_ne_credite_jamais_un_ennemi_en_fuite`), mais sans effet sur
    /// l'affichage.
    #[test]
    fn enemy_fled_ne_marque_jamais_is_ko() {
        let mut state = SessionState::default();
        state.apply(
            &fighter_joined(1, "Mimique", 1, true),
            ApplyContext::default(),
            &mut Vec::new(),
        );
        state.apply(
            &enemy_fled(1, "Mimique"),
            ApplyContext::default(),
            &mut Vec::new(),
        );

        assert!(!state.fights[&1].snapshot.fighters[0].is_ko);
    }

    /// Demande explicite de l'utilisateur (cas des mimiques, voir `registerFightFlee` côté web) :
    /// une fuite n'est jamais requalifiée en victoire sous prétexte que le reste de l'équipe a
    /// gagné.
    #[test]
    fn filet_de_rattrapage_ne_credite_jamais_un_ennemi_en_fuite() {
        let mut state = SessionState::default();
        state.apply(
            &fighter_joined(1, "Mimique", 1, true),
            ApplyContext::default(),
            &mut Vec::new(),
        );
        state.apply(
            &enemy_fled(1, "Mimique"),
            ApplyContext::default(),
            &mut Vec::new(),
        );

        let implicitly_defeated = state.apply(
            &combat_end(1, FightResult::Won),
            ApplyContext::default(),
            &mut Vec::new(),
        );

        assert!(implicitly_defeated.is_empty());
    }

    #[test]
    fn filet_de_rattrapage_ne_sapplique_pas_sur_une_defaite() {
        let mut state = SessionState::default();
        state.apply(
            &fighter_joined(1, "Bwork", 1, true),
            ApplyContext::default(),
            &mut Vec::new(),
        );

        let implicitly_defeated = state.apply(
            &combat_end(1, FightResult::Lost),
            ApplyContext::default(),
            &mut Vec::new(),
        );

        assert!(implicitly_defeated.is_empty());
    }

    // --- L5, §7.1 : événements d'historique complétés (2026-09-02) ------------------------------

    fn xp_gain(fight_id: i64, character: &str, amount: i64) -> LogEntry {
        LogEntry::XpGain {
            time: "12:00:03,000".to_string(),
            character: character.to_string(),
            amount,
            fight_id: Some(fight_id),
        }
    }

    fn damage_with_spell(
        fight_id: i64,
        attacker: &str,
        spell: &str,
        element: DamageElement,
        amount: i64,
    ) -> LogEntry {
        LogEntry::Damage {
            time: "12:00:01,000".to_string(),
            target: "cible".to_string(),
            attacker: attacker.to_string(),
            spell: spell.to_string(),
            element,
            amount,
            fight_id: Some(fight_id),
        }
    }

    /// Extrait le seul `FightPayload` d'une liste de `SyncEvent` — panique si absent/ambigu,
    /// pratique pour les assertions de ces tests (un seul combat terminé par scénario).
    fn only_fight_payload(events: &[SyncEvent]) -> &FightPayload {
        let fights: Vec<&FightPayload> = events
            .iter()
            .filter_map(|e| match &e.payload {
                HistoryPayload::Fight(f) => Some(f),
                _ => None,
            })
            .collect();
        assert_eq!(fights.len(), 1, "un seul FightPayload attendu");
        fights[0]
    }

    fn only_purchase_payload(events: &[SyncEvent]) -> &PurchasePayload {
        let purchases: Vec<&PurchasePayload> = events
            .iter()
            .filter_map(|e| match &e.payload {
                HistoryPayload::Purchase(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(purchases.len(), 1, "un seul PurchasePayload attendu");
        purchases[0]
    }

    #[test]
    fn xp_gagnee_est_ventilee_par_participant_et_totalisee_pour_le_combat() {
        let mut state = SessionState::default();
        let mut events = Vec::new();
        state.apply(
            &fighter_joined(1, "Oumbra", 9, false),
            ApplyContext::default(),
            &mut events,
        );
        state.apply(
            &xp_gain(1, "Oumbra", 1000),
            ApplyContext::default(),
            &mut events,
        );
        state.apply(
            &xp_gain(1, "Oumbra", 500),
            ApplyContext::default(),
            &mut events,
        );
        // Un nom non résolu dans ce combat (ex. XP d'un combat concurrent mal routée) ne doit
        // créditer personne — miroir d'`isRosterMember` (fight roster, pas le compte).
        state.apply(
            &xp_gain(1, "Quidam", 999),
            ApplyContext::default(),
            &mut events,
        );
        state.apply(
            &combat_end(1, FightResult::Won),
            ApplyContext::default(),
            &mut events,
        );

        let fight = only_fight_payload(&events);
        assert_eq!(
            fight.xp_gained, 1500,
            "total du combat, XP non routée exclue"
        );
        assert_eq!(fight.participants[0].xp_gained, 1500);
    }

    #[test]
    fn degats_ventiles_par_sort_et_element_uniquement_pour_les_degats() {
        let mut state = SessionState::default();
        let mut events = Vec::new();
        state.apply(
            &fighter_joined(1, "Oumbra", 9, false),
            ApplyContext::default(),
            &mut events,
        );
        state.apply(
            &damage_with_spell(1, "Oumbra", "Frappe", DamageElement::Feu, 100),
            ApplyContext::default(),
            &mut events,
        );
        state.apply(
            &damage_with_spell(1, "Oumbra", "Frappe", DamageElement::Feu, 50),
            ApplyContext::default(),
            &mut events,
        );
        state.apply(
            &damage_with_spell(1, "Oumbra", "Frappe", DamageElement::Air, 30),
            ApplyContext::default(),
            &mut events,
        );
        state.apply(
            &combat_end(1, FightResult::Won),
            ApplyContext::default(),
            &mut events,
        );

        let fight = only_fight_payload(&events);
        let spells = &fight.participants[0].spells;
        assert_eq!(spells.len(), 1, "un seul sort utilisé");
        assert_eq!(spells[0].spell, "Frappe");
        assert_eq!(spells[0].total, 180);
        assert_eq!(spells[0].by_element.get("Feu"), Some(&150));
        assert_eq!(spells[0].by_element.get("Air"), Some(&30));
    }

    fn sample_catalog() -> CatalogIndex {
        CatalogIndex::from_compact_json(&serde_json::json!({
            "items": [[100, "Larme d'Ogrest", "Ogrest's Tear", "x", "x", 1, 1, 0, 1]],
            "monsters": [
                [200, "El Pochito", "El Pochito", "x", "x", "1", -1, 1, 0, 0],
                // Pas boss (isBoss=0) — sert de "salle" pour les tests de regroupement de donjon.
                [201, "Salle Larventura", "Salle Larventura", "x", "x", "2", -1, 0, 0, 0],
            ],
        }))
    }

    #[test]
    fn monster_id_et_item_id_resolus_via_le_catalogue() {
        let catalog = sample_catalog();
        let ctx = ApplyContext {
            catalog: Some(&catalog),
            ..Default::default()
        };
        let mut state = SessionState::default();
        let mut events = Vec::new();
        state.apply(&fighter_joined(1, "El Pochito", 1, true), ctx, &mut events);
        state.apply(
            &LogEntry::Loot {
                time: "12:00:01,500".to_string(),
                item: "Larme d'Ogrest".to_string(),
                quantity: 1,
                fight_id: Some(1),
            },
            ctx,
            &mut events,
        );
        state.apply(&combat_end(1, FightResult::Won), ctx, &mut events);

        let fight = only_fight_payload(&events);
        assert_eq!(fight.participants[0].monster_id, Some(200));
        assert_eq!(fight.loot[0].item_id, Some(100));
        assert_eq!(
            fight.loot[0].item_name, None,
            "mutuellement exclusif avec itemId"
        );
    }

    #[test]
    fn game_server_deduit_du_dernier_personnage_du_roster_reconnu() {
        // `Engine::notice_character`/`current_game_server` ne sont testés qu'indirectement ici
        // (via `ApplyContext::game_server` déjà résolu) : `SessionState::apply` lui-même ne fait
        // que STAMPER la valeur reçue sur les payloads construits, jamais sa propre déduction.
        let ctx = ApplyContext {
            game_server: Some("pandora"),
            ..Default::default()
        };
        let mut state = SessionState::default();
        let mut events = Vec::new();
        state.apply(&fighter_joined(1, "Oumbra", 9, false), ctx, &mut events);
        state.apply(&combat_end(1, FightResult::Won), ctx, &mut events);

        assert_eq!(
            only_fight_payload(&events).game_server.as_deref(),
            Some("pandora")
        );
    }

    #[test]
    fn kamas_hdv_sans_achat_adjacent_est_detecte_en_fin_de_lot() {
        let mut state = SessionState::default();
        let mut events = Vec::new();
        state.apply(
            &LogEntry::KamaGain {
                time: "12:00:00,000".to_string(),
                amount: 5000,
                fight_id: None,
            },
            ApplyContext::default(),
            &mut events,
        );
        assert!(events.is_empty(), "en attente jusqu'à la fin du lot");
        state.flush_pending_hdv_kama_gain(ApplyContext::default(), &mut events);

        let purchase = only_purchase_payload(&events);
        assert_eq!(purchase.item_name.as_deref(), Some(HDV_KAMAS_SALE_ITEM));
        assert_eq!(purchase.item_id, None);
        assert_eq!(purchase.quantity, 0);
        assert_eq!(purchase.total_cost, 5000);
    }

    #[test]
    fn kamas_hdv_annule_si_explique_par_un_echange_tout_juste_conclu() {
        let mut state = SessionState::default();
        let mut events = Vec::new();
        state.apply(
            &LogEntry::KamaGain {
                time: "12:00:00,000".to_string(),
                amount: 5000,
                fight_id: None,
            },
            ApplyContext::default(),
            &mut events,
        );
        state.apply(
            &LogEntry::TradeCompleted {
                time: "12:00:00,500".to_string(), // 500 ms plus tard, dans PURCHASE_WINDOW_MS
                sides: [
                    crate::model::TradeSide {
                        player_name: "Oumbra".to_string(),
                        items: Vec::new(),
                        kamas: 5000,
                    },
                    crate::model::TradeSide {
                        player_name: "Peer".to_string(),
                        items: Vec::new(),
                        kamas: 0,
                    },
                ],
            },
            ApplyContext::default(),
            &mut events,
        );
        state.flush_pending_hdv_kama_gain(ApplyContext::default(), &mut events);

        assert!(
            events.iter().all(|e| !matches!(&e.payload, HistoryPayload::Purchase(p) if p.item_name.as_deref() == Some(HDV_KAMAS_SALE_ITEM))),
            "expliqué par l'échange, ne doit jamais devenir une vente HDV"
        );
    }

    #[test]
    fn dungeon_id_resolu_quand_ce_combat_contient_son_propre_boss() {
        let catalog = sample_catalog(); // "El Pochito", id 200, isBoss=1
        let dungeons = DungeonIndex::from_json(&serde_json::json!([
            { "id": 65, "fr": "Larventura", "en": "x", "es": "x", "pt": "x",
              "bossMonsterId": [200], "monsterFamilyId": [], "type": "TWO_ROOMS" },
        ]));
        let ctx = ApplyContext {
            catalog: Some(&catalog),
            dungeons: Some(&dungeons),
            ..Default::default()
        };
        let mut state = SessionState::default();
        let mut events = Vec::new();
        state.apply(&fighter_joined(1, "El Pochito", 1, true), ctx, &mut events);
        state.apply(&combat_end(1, FightResult::Won), ctx, &mut events);

        let fight = only_fight_payload(&events);
        assert_eq!(fight.dungeon_id, Some(65));
        // Cas "propre représentant" (§7.1 de `history.rs`) : la graine du run == la signature du
        // combat lui-même.
        assert_eq!(
            fight.dungeon_run_signature,
            Some(fight_signature(
                "12:00:02,000",
                1,
                true,
                &[("El Pochito".to_string(), 0)]
            ))
        );
    }

    #[test]
    fn dungeon_id_reste_none_hors_donjon_connu() {
        let catalog = sample_catalog();
        let ctx = ApplyContext {
            catalog: Some(&catalog),
            dungeons: Some(&DungeonIndex::from_json(&serde_json::json!([]))),
            ..Default::default()
        };
        let mut state = SessionState::default();
        let mut events = Vec::new();
        state.apply(&fighter_joined(1, "El Pochito", 1, true), ctx, &mut events);
        state.apply(&combat_end(1, FightResult::Won), ctx, &mut events);

        let fight = only_fight_payload(&events);
        assert_eq!(fight.dungeon_id, None);
        assert_eq!(fight.dungeon_run_signature, None);
    }

    /// Test d'intégration bout en bout du regroupement multi-salles (L5, §7.1) : une salle gagnée
    /// AVANT que son boss n'ait jamais été rencontré part d'abord sans rattachement (le boss n'est
    /// pas encore dans l'historique connu, voir `resolve_dungeon_assignment`) ; dès que le boss se
    /// termine à son tour, la salle DÉJÀ ENVOYÉE est rebâtie et renvoyée avec le MÊME `dungeonId`
    /// et la MÊME graine de run que le boss (miroir de `HistorySyncService.recordFight`, boucle
    /// `assignment.siblings`).
    #[test]
    fn run_de_donjon_multi_salles_renvoie_les_siblings_avec_le_meme_dungeon_run_key() {
        let catalog = sample_catalog();
        let dungeons = DungeonIndex::from_json(&serde_json::json!([
            { "id": 65, "fr": "Larventura", "en": "x", "es": "x", "pt": "x",
              "bossMonsterId": [200], "monsterFamilyId": [], "type": "TWO_ROOMS",
              "hasPreBossArchi": false },
        ]));
        let ctx = ApplyContext {
            catalog: Some(&catalog),
            dungeons: Some(&dungeons),
            ..Default::default()
        };
        let mut state = SessionState::default();
        let mut events = Vec::new();

        // Salle (fight 1), gagnée : aucun boss dedans -> pas de rattachement pour l'instant.
        state.apply(
            &fighter_joined(1, "Salle Larventura", 1, true),
            ctx,
            &mut events,
        );
        state.apply(&combat_end(1, FightResult::Won), ctx, &mut events);
        assert_eq!(events.len(), 1);
        assert_eq!(only_fight_payload(&events).dungeon_id, None);
        events.clear();

        // Boss (fight 2), gagné à son tour : le run se complète — la salle 1 doit être RENVOYÉE.
        state.apply(&fighter_joined(2, "El Pochito", 1, true), ctx, &mut events);
        state.apply(&combat_end(2, FightResult::Won), ctx, &mut events);

        assert_eq!(
            events.len(),
            2,
            "le boss ET la salle (sibling) doivent repartir"
        );
        let fights: std::collections::HashMap<i64, &FightPayload> = events
            .iter()
            .filter_map(|e| match &e.payload {
                HistoryPayload::Fight(f) => Some((f.fight_id.unwrap(), f)),
                _ => None,
            })
            .collect();
        let boss = fights[&2];
        let salle = fights[&1];
        assert_eq!(boss.dungeon_id, Some(65));
        assert_eq!(salle.dungeon_id, Some(65));
        assert!(boss.dungeon_run_signature.is_some());
        assert_eq!(
            boss.dungeon_run_signature, salle.dungeon_run_signature,
            "même graine de run pour tout le run"
        );
        // La graine du run est la signature PROPRE du combat de boss (représentant) — miroir de
        // « dungeonRunKey du run == clientKey du boss lui-même ».
        assert_eq!(
            boss.dungeon_run_signature,
            Some(fight_signature(
                "12:00:02,000",
                2,
                true,
                &[("El Pochito".to_string(), 0)]
            ))
        );
        // La salle garde SA PROPRE signature d'identité (celle qui la dédoublonne), distincte de
        // la graine de run partagée.
        assert_ne!(salle.dungeon_run_signature, None);
        assert_eq!(
            fights[&1].dungeon_id, fights[&2].dungeon_id,
            "les deux combats du run partagent le même donjon"
        );
    }

    /// Complète le test précédent : une victoire plus ANCIENNE contre le même boss n'est jamais
    /// fusionnée dans le run le plus récent (miroir de `groupDungeonRuns`, étape 1) — deux clears
    /// distincts de Larventura restent deux runs séparés, chacun avec sa propre graine.
    #[test]
    fn deux_clears_successifs_du_meme_donjon_restent_deux_runs_distincts() {
        let catalog = sample_catalog();
        let dungeons = DungeonIndex::from_json(&serde_json::json!([
            { "id": 65, "fr": "Larventura", "en": "x", "es": "x", "pt": "x",
              "bossMonsterId": [200], "monsterFamilyId": [], "type": "TWO_ROOMS",
              "hasPreBossArchi": false },
        ]));
        let ctx = ApplyContext {
            catalog: Some(&catalog),
            dungeons: Some(&dungeons),
            ..Default::default()
        };
        let mut state = SessionState::default();
        let mut events = Vec::new();

        // Premier clear complet : salle (1) puis boss (2).
        state.apply(
            &fighter_joined(1, "Salle Larventura", 1, true),
            ctx,
            &mut events,
        );
        state.apply(&combat_end(1, FightResult::Won), ctx, &mut events);
        state.apply(&fighter_joined(2, "El Pochito", 1, true), ctx, &mut events);
        state.apply(&combat_end(2, FightResult::Won), ctx, &mut events);
        events.clear();

        // Second clear complet, indépendant : salle (3) puis boss (4). La salle 3 part d'abord
        // sans rattachement (boss 4 pas encore connu, même comportement que le premier test) —
        // on vide `events` juste après pour n'observer QUE ce que déclenche la fin du boss.
        state.apply(
            &fighter_joined(3, "Salle Larventura", 1, true),
            ctx,
            &mut events,
        );
        state.apply(&combat_end(3, FightResult::Won), ctx, &mut events);
        events.clear();
        state.apply(&fighter_joined(4, "El Pochito", 1, true), ctx, &mut events);
        state.apply(&combat_end(4, FightResult::Won), ctx, &mut events);

        assert_eq!(
            events.len(),
            2,
            "le boss ET son sibling (3) repartent, jamais 1/2"
        );
        let fights: std::collections::HashMap<i64, &FightPayload> = events
            .iter()
            .filter_map(|e| match &e.payload {
                HistoryPayload::Fight(f) => Some((f.fight_id.unwrap(), f)),
                _ => None,
            })
            .collect();
        assert!(fights.contains_key(&3));
        assert!(fights.contains_key(&4));
        assert_eq!(
            fights[&3].dungeon_run_signature,
            fights[&4].dungeon_run_signature
        );
    }
}
