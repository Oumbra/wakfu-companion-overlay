//! Vue **Combats** — port de `features/fight-history/` (site) : recherche sur les combattants et
//! le butin, groupes par jour repliables (avec « Tout replier »), runs de donjon réunis sous une
//! entrée repliable, et pour chaque combat son résumé (heure, illustration, résultat, tours et
//! durée) puis, déplié, son détail : alliés et ennemis en dégâts, armure ou soins, butin et XP.
//!
//! Le modèle ([`entries`], [`matches`], [`illustration`]) est séparé du rendu : c'est lui que les
//! tests comparent au site.
//!
//! Un clic sur un combattant déplie ses sorts (nom, part par élément, total), lus dans la base à
//! la demande ([`HistoryStore::participant_spells`]) puis gardés jusqu'au prochain changement de la
//! base.
//!
//! Le butin d'un combat se filtre et se trie (nom, quantité, rareté) ; la recherche et le tri sont
//! partagés par tous les combats dépliés, comme sur le site.
//!
//! **Écarts** : pas de vue par tour (la base locale ne garde pas les dégâts tour par tour) ; un
//! reclic sur le tri actif n'en inverse pas le sens (`design::switch` ne signale pas ce clic).

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use egui::{Align2, Color32, Rect, Sense, Ui};
use overlay_engine::roster::normalize_wakfu_name;
use overlay_engine::{CatalogIndex, DungeonIndex, IconRef, WakfuDungeonType, WakfuRarity};
use overlay_history::read::{FightRow, ItemLine, ParticipantRow, SpellColumn, SpellLine};
use overlay_history::HistoryStore;

use super::items::{self, CompactRow, DayTail, ListState};
use super::{format, HistoryContext};
use crate::design::{self, text, tokens};
use crate::remote_icons::RemoteIconTextures;

/// Une ligne de la liste : un combat seul, ou les combats d'un même run de donjon (du plus récent
/// au plus ancien, le combat de boss le plus récent en tête).
#[derive(Debug)]
pub enum Entry<'a> {
    Single(&'a FightRow),
    Run {
        dungeon_id: Option<i64>,
        fights: Vec<&'a FightRow>,
    },
}

impl<'a> Entry<'a> {
    /// Le combat qui représente l'entrée (son heure, son jour).
    pub fn representative(&self) -> &'a FightRow {
        match self {
            Entry::Single(fight) => fight,
            Entry::Run { fights, .. } => fights[0],
        }
    }

    pub fn fights(&self) -> &[&'a FightRow] {
        match self {
            Entry::Single(fight) => std::slice::from_ref(fight),
            Entry::Run { fights, .. } => fights,
        }
    }

    /// Clé stable de l'entrée (repli d'un run).
    pub fn key(&self) -> i64 {
        self.representative().id
    }
}

/// Les entrées de la liste : les combats qui partagent une clé de run (`dungeon_run_key`, posée
/// à l'écriture comme le serveur le faisait) sont réunis, à la place du plus récent d'entre eux.
pub fn entries<'a>(fights: &[&'a FightRow]) -> Vec<Entry<'a>> {
    let mut by_key: HashMap<&str, Vec<&FightRow>> = HashMap::new();
    for fight in fights {
        if let Some(key) = fight.dungeon_run_key.as_deref() {
            by_key.entry(key).or_default().push(fight);
        }
    }
    let mut placed: HashSet<&str> = HashSet::new();
    let mut entries = Vec::new();
    for fight in fights {
        match fight.dungeon_run_key.as_deref() {
            // Un combat d'un run déjà placé (à la place du plus récent de ses combats).
            Some(key) if placed.contains(key) => {}
            Some(key) if by_key.get(key).is_some_and(|run| run.len() > 1) => {
                if placed.insert(key) {
                    let run = by_key.remove(key).unwrap_or_default();
                    let dungeon_id = run.iter().find_map(|fight| fight.dungeon_id);
                    entries.push(Entry::Run {
                        dungeon_id,
                        fights: run,
                    });
                }
            }
            _ => entries.push(Entry::Single(fight)),
        }
    }
    entries
}

/// Un combat retenu par la recherche (`fightSearchHaystack` du site) : un combattant, allié ou
/// ennemi, ou un objet du butin. `query` est déjà normalisée.
pub fn matches(fight: &FightRow, query: &str, catalog: &CatalogIndex) -> bool {
    query.is_empty()
        || fight
            .participants
            .iter()
            .any(|p| normalize_wakfu_name(&p.name).contains(query))
        || fight
            .loot
            .iter()
            .any(|line| normalize_wakfu_name(&items::item_name(line, catalog)).contains(query))
}

/// L'illustration d'un combat (`resolveFightImageInfo`) : le boss d'un donjon, sinon un
/// archimonstre, sinon l'ennemi le plus nombreux, sinon celui qui a fait le plus de dégâts.
pub fn illustration<'a>(
    fight: &'a FightRow,
    catalog: &CatalogIndex,
    dungeons: &DungeonIndex,
) -> Option<&'a ParticipantRow> {
    let enemies: Vec<&ParticipantRow> = fight.participants.iter().filter(|p| !p.is_ally).collect();
    if let Some(boss) = enemies.iter().find(|p| {
        p.monster_id
            .is_some_and(|id| dungeons.find_by_boss_monster_id(id).is_some())
    }) {
        return Some(boss);
    }
    if let Some(archi) = enemies
        .iter()
        .find(|p| catalog.find_monster_is_archi(&p.name, p.monster_id))
    {
        return Some(archi);
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for enemy in &enemies {
        *counts.entry(enemy.name.as_str()).or_default() += 1;
    }
    let most = counts.values().copied().max().unwrap_or(0);
    if most > 1 && counts.values().filter(|&&count| count == most).count() == 1 {
        return enemies
            .iter()
            .find(|p| counts[p.name.as_str()] == most)
            .copied();
    }
    enemies.into_iter().max_by_key(|p| p.damage)
}

/// Le regroupement de la liste.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GroupMode {
    /// Par jour (Aujourd'hui, Hier, date).
    #[default]
    Day,
    /// Par donjon, puis par famille de monstres (« Donjons & familles »).
    Type,
}

/// Un groupe repliable de la liste.
#[derive(Debug)]
pub struct FightGroup<'e, 'a> {
    /// Clé stable (état replié), propre au regroupement.
    pub key: String,
    pub label: String,
    /// « 3 combats », « 2 donjons ».
    pub count: String,
    /// Titre de section à peindre AVANT ce groupe (premier groupe d'une section).
    pub section: Option<String>,
    pub entries: Vec<&'e Entry<'a>>,
}

fn fights_in(entries: &[&Entry]) -> usize {
    entries.iter().map(|entry| entry.fights().len()).sum()
}

/// Groupes par jour local du combat représentatif.
pub fn day_groups<'e, 'a>(entries: &[&'e Entry<'a>], today: NaiveDate) -> Vec<FightGroup<'e, 'a>> {
    items::group_by_day(
        entries.iter().copied(),
        |entry| entry.representative().started_at,
        items::SortOrder::Newest,
    )
    .into_iter()
    .map(|(day, entries)| FightGroup {
        key: format!("jour:{day}"),
        label: format::relative_day(day, today),
        count: fight_count(fights_in(&entries)),
        section: None,
        entries,
    })
    .collect()
}

/// Ordre des sections de donjon (`DUNGEON_TYPE_ORDER`) et leurs titres.
pub(super) const DUNGEON_SECTIONS: [(WakfuDungeonType, &str); 8] = [
    (WakfuDungeonType::TwoRooms, "Donjon 2 salles"),
    (WakfuDungeonType::ThreeRooms, "Donjon 3 salles"),
    (WakfuDungeonType::FourRooms, "Donjon 4 salles"),
    (WakfuDungeonType::ThreePlayers, "Donjon 3 joueurs"),
    (WakfuDungeonType::UltimateBoss, "Boss ultime"),
    (WakfuDungeonType::Breach, "Brèche"),
    (WakfuDungeonType::UltimateBreach, "Brèche ultime"),
    (WakfuDungeonType::Arcade, "Donjon arcade"),
];

/// Groupes « Donjons & familles » (`buildTypeGroups` du site) : un groupe par donjon, rangé sous
/// la section de son type, dans l'ordre fixe du site ; puis, sous « Familles », un groupe par
/// famille de monstres de l'illustration du combat. Dans une section, les groupes gardent l'ordre
/// chronologique de leur première entrée.
///
/// Un groupe de famille porte le nom de la famille ; sans lui (référentiel pas encore chargé,
/// monstre sans famille), le repli du site : le monstre représentatif le plus fréquent du groupe.
pub fn type_groups<'e, 'a>(
    entries: &[&'e Entry<'a>],
    catalog: &CatalogIndex,
    dungeons: &DungeonIndex,
) -> Vec<FightGroup<'e, 'a>> {
    struct Bucket<'e, 'a> {
        key: String,
        dungeon: Option<(WakfuDungeonType, String)>,
        family: Option<i64>,
        names: Vec<(String, usize)>,
        entries: Vec<&'e Entry<'a>>,
    }
    let mut buckets: Vec<Bucket> = Vec::new();
    for entry in entries {
        let dungeon_id = match entry {
            Entry::Run { dungeon_id, .. } => *dungeon_id,
            Entry::Single(fight) => fight.dungeon_id,
        };
        let dungeon = dungeon_id.and_then(|id| dungeons.find_by_id(id));
        let face = illustration(entry.representative(), catalog, dungeons);
        let family = match (dungeon, face) {
            (None, Some(face)) => catalog.find_monster_family_id(&face.name, face.monster_id),
            _ => None,
        };
        let key = match (dungeon, face, family) {
            (Some(dungeon), _, _) => format!("donjon:{}", dungeon.id),
            (None, _, Some(family)) => format!("famille:{family}"),
            (None, Some(face), None) => format!("monstre:{}", face.name),
            (None, None, _) => "autre".to_string(),
        };
        let position = match buckets.iter().position(|bucket| bucket.key == key) {
            Some(position) => position,
            None => {
                buckets.push(Bucket {
                    key,
                    dungeon: dungeon.map(|d| (d.dungeon_type, d.fr.clone())),
                    family,
                    names: Vec::new(),
                    entries: Vec::new(),
                });
                buckets.len() - 1
            }
        };
        let bucket = &mut buckets[position];
        bucket.entries.push(entry);
        if let Some(face) = face {
            match bucket.names.iter_mut().find(|(name, _)| *name == face.name) {
                Some((_, count)) => *count += 1,
                None => bucket.names.push((face.name.clone(), 1)),
            }
        }
    }
    let mut groups = Vec::new();
    for (dungeon_type, title) in DUNGEON_SECTIONS {
        let mut first = true;
        for bucket in buckets
            .iter()
            .filter(|bucket| bucket.dungeon.as_ref().map(|(t, _)| *t) == Some(dungeon_type))
        {
            let count = bucket.entries.len();
            groups.push(FightGroup {
                key: format!("type:{}", bucket.key),
                label: bucket
                    .dungeon
                    .as_ref()
                    .map_or_else(String::new, |(_, name)| name.clone()),
                count: if count == 1 {
                    "1 donjon".to_string()
                } else {
                    format!("{count} donjons")
                },
                section: first.then(|| title.to_string()),
                entries: bucket.entries.clone(),
            });
            first = false;
        }
    }
    let mut first = true;
    for bucket in buckets.iter().filter(|bucket| bucket.dungeon.is_none()) {
        // Le nom de la famille, sinon le premier nom le plus fréquent (ordre d'apparition à
        // égalité).
        let most_frequent = || {
            bucket
                .names
                .iter()
                .fold(None::<&(String, usize)>, |best, candidate| match best {
                    Some(best) if best.1 >= candidate.1 => Some(best),
                    _ => Some(candidate),
                })
                .map_or_else(|| "Autres combats".to_string(), |(name, _)| name.clone())
        };
        let label = bucket
            .family
            .and_then(super::family_name)
            .map_or_else(most_frequent, str::to_string);
        groups.push(FightGroup {
            key: format!("type:{}", bucket.key),
            label,
            count: fight_count(fights_in(&bucket.entries)),
            section: first.then(|| "Familles".to_string()),
            entries: bucket.entries.clone(),
        });
        first = false;
    }
    groups
}

/// `3 min 12sec.` — `formatDuration` du site.
pub fn duration(ms: i64) -> String {
    let seconds = (ms.max(0) + 500) / 1000;
    format!("{} min {}sec.", seconds / 60, seconds % 60)
}

/// `1 combat`, `3 combats`.
pub fn fight_count(count: usize) -> String {
    if count == 1 {
        "1 combat".to_string()
    } else {
        format!("{count} combats")
    }
}

/// La grandeur mesurée dans le détail d'un combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Metric {
    #[default]
    Damage,
    Armor,
    Heal,
}

impl Metric {
    fn of(self, participant: &ParticipantRow) -> i64 {
        match self {
            Metric::Damage => participant.damage,
            Metric::Armor => participant.armor,
            Metric::Heal => participant.heal,
        }
    }

    fn column(self) -> SpellColumn {
        match self {
            Metric::Damage => SpellColumn::Damage,
            Metric::Armor => SpellColumn::Armor,
            Metric::Heal => SpellColumn::Heal,
        }
    }
}

/// Un combattant d'un combat : `(id du combat, nom, rang de l'instance)`.
pub type ParticipantKey = (i64, String, i64);

fn participant_key(fight_id: i64, participant: &ParticipantRow) -> ParticipantKey {
    (
        fight_id,
        participant.name.clone(),
        participant.instance_index,
    )
}

/// Ce qui est déplié dans les combats, et les sorts déjà lus.
#[derive(Default)]
pub struct DetailState {
    /// Combats dépliés.
    pub expanded: HashSet<i64>,
    /// Grandeur choisie pour chaque combat déplié.
    pub metric: HashMap<i64, Metric>,
    /// Combattants dépliés (leurs sorts affichés).
    pub open_participants: HashSet<ParticipantKey>,
    /// Sorts lus, par combattant et grandeur — vidé quand la base change.
    pub spells: HashMap<(ParticipantKey, Metric), Vec<SpellLine>>,
    /// Recherche dans le butin, partagée par tous les combats dépliés.
    pub loot_query: String,
    /// Tri du butin, partagé de même.
    pub loot_sort: LootSort,
    /// Sens inversé (reclic sur le tri actif, `nextLootSortState` du site).
    pub loot_reverse: bool,
}

/// Le tri du butin d'un combat (`LootSort` du site).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LootSort {
    /// Ordre alphabétique — le défaut du site.
    #[default]
    Name,
    /// Plus grosse quantité d'abord.
    Quantity,
    /// Plus rare d'abord, puis par nom.
    Rarity,
}

/// Rang de tri d'une rareté (`RARITY_SORT_ORDER` du site) : ancienne 0 … relique 7.
fn rarity_rank(rarity: WakfuRarity) -> u8 {
    match rarity {
        WakfuRarity::Old => 0,
        WakfuRarity::Common => 1,
        WakfuRarity::Rare => 2,
        WakfuRarity::Mythical => 3,
        WakfuRarity::Legendary => 4,
        WakfuRarity::Memory => 5,
        WakfuRarity::Epic => 6,
        WakfuRarity::Relic => 7,
    }
}

/// Le butin filtré par `query` (déjà normalisée) et trié (`sortLootRows` du site).
pub fn sorted_loot<'a>(
    loot: &'a [ItemLine],
    query: &str,
    sort: LootSort,
    catalog: &CatalogIndex,
) -> Vec<&'a ItemLine> {
    let mut lines: Vec<(String, &ItemLine)> = loot
        .iter()
        .map(|line| (items::item_name(line, catalog), line))
        .filter(|(name, _)| query.is_empty() || normalize_wakfu_name(name).contains(query))
        .collect();
    let key = |name: &str| normalize_wakfu_name(name);
    match sort {
        LootSort::Name => lines.sort_by_cached_key(|(name, _)| key(name)),
        LootSort::Quantity => lines.sort_by_key(|(_, line)| std::cmp::Reverse(line.quantity)),
        LootSort::Rarity => lines.sort_by_cached_key(|(name, line)| {
            (
                std::cmp::Reverse(rarity_rank(catalog.find_item_rarity(name, line.item_id))),
                key(name),
            )
        }),
    }
    lines.into_iter().map(|(_, line)| line).collect()
}

/// État de la vue.
#[derive(Default)]
pub struct CombatsState {
    pub list: ListState<FightRow>,
    /// Regroupement choisi.
    pub mode: GroupMode,
    /// Groupes repliés (clé : [`FightGroup::key`]).
    pub collapsed_groups: HashSet<String>,
    /// Combats et combattants dépliés.
    pub detail: DetailState,
    /// Runs de donjon dépliés (clé : [`Entry::key`]).
    pub expanded_runs: HashSet<i64>,
}

const SUMMARY_HEIGHT: f32 = 40.0;
const IMAGE_SIZE: f32 = 28.0;
const TOGGLE_ALL_WIDTH: f32 = 140.0;
const MODE_SWITCH_WIDTH: f32 = 300.0;
/// Victoire / défaite (`.fight-badge.won`, `.lost`).
const WON: Color32 = items::GAIN;
const LOST: Color32 = items::LOSS;
const DETAIL_FILL: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 70);
const BAR_TRACK: Color32 = Color32::from_rgba_premultiplied(14, 14, 14, 14);
const ALLY_BAR: Color32 = Color32::from_rgb(0x3E, 0x9B, 0xD6);
const ENEMY_BAR: Color32 = Color32::from_rgb(0xC9, 0x52, 0x4A);
const PARTICIPANT_ROW: f32 = 28.0;
/// Une ligne de sort sous un combattant déplié.
const SPELL_ROW: f32 = 22.0;
/// Côté de l'icône d'un sort.
const SPELL_ICON: f32 = 18.0;
/// Couleurs des éléments (`--dmg-*`, thème sombre du site).
fn element_color(element: &str) -> Color32 {
    match element {
        "Feu" => Color32::from_rgb(0xE9, 0x78, 0x2D),
        "Air" => Color32::from_rgb(0xD6, 0x9D, 0xFC),
        "Terre" => Color32::from_rgb(0x1B, 0x80, 0x45),
        "Eau" => Color32::from_rgb(0x00, 0xE1, 0xFF),
        "Lumière" => Color32::from_rgb(0xFF, 0xD7, 0x00),
        "Stasis" => Color32::from_rgb(0xCF, 0x9F, 0xFF),
        "Neutre" => Color32::from_rgb(0xCF, 0xCF, 0xCF),
        _ => Color32::from_rgb(0x99, 0x99, 0x99),
    }
}
const LOOT_ROW: CompactRow = CompactRow {
    height: 26.0,
    pad_x: 6.0,
    gap: 6.0,
    font_size: 13.0,
};

pub fn show(
    ui: &mut Ui,
    zones: &design::PanelZones,
    state: &mut CombatsState,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    state.list.load(ctx.store, HistoryStore::fights, "combats");
    let CombatsState {
        list,
        mode,
        collapsed_groups,
        detail,
        expanded_runs,
    } = state;
    let ListState {
        query: search,
        rows,
        pager,
        ..
    } = list;
    let rows = rows.as_deref().unwrap_or_default();
    let times: Vec<i64> = rows.iter().map(|row| row.started_at).collect();
    let shown = pager.visible(&times, false);
    let query = normalize_wakfu_name(search.trim());
    let visible: Vec<&FightRow> = rows[..shown].iter().collect();
    let all_entries = entries(&visible);
    let kept: Vec<&Entry> = all_entries
        .iter()
        .filter(|entry| {
            entry
                .fights()
                .iter()
                .any(|fight| matches(fight, &query, ctx.catalog))
        })
        .collect();
    let groups = match mode {
        GroupMode::Day => day_groups(&kept, ctx.today),
        GroupMode::Type => type_groups(&kept, ctx.catalog, ctx.dungeons),
    };

    // Recherche, regroupement, puis « Tout replier » / « Tout déplier ».
    ui.horizontal(|ui| {
        ui.add(
            design::input(search)
                .placeholder("Rechercher un ennemi, un allié, un objet...")
                .clearable(true)
                .width(zones.inner.width() - MODE_SWITCH_WIDTH - TOGGLE_ALL_WIDTH - 24.0)
                .log_name("historique.combats.recherche"),
        );
        ui.add(
            design::switch(mode)
                .slot(GroupMode::Day, "Jour")
                .slot(GroupMode::Type, "Donjons & familles")
                .width(MODE_SWITCH_WIDTH)
                .log_name("historique.combats.regroupement"),
        );
        let any_open = groups
            .iter()
            .any(|group| !collapsed_groups.contains(&group.key));
        if !groups.is_empty()
            && ui
                .add(
                    design::button(if any_open {
                        "Tout replier"
                    } else {
                        "Tout déplier"
                    })
                    .size(design::ButtonSize::Compact)
                    .width(TOGGLE_ALL_WIDTH)
                    .log_name("historique.combats.tout"),
                )
                .clicked()
        {
            if any_open {
                collapsed_groups.extend(groups.iter().map(|group| group.key.clone()));
            } else {
                for group in &groups {
                    collapsed_groups.remove(&group.key);
                }
            }
        }
    });
    ui.add_space(8.0);

    zones.scroll_area(ui, "historique-combats", |ui, width| {
        if rows.is_empty() {
            ui.add(
                design::label("Aucun combat détecté pour le moment.").color(tokens::TEXT_DISABLED),
            );
        } else if groups.is_empty() {
            ui.add(
                design::label("Aucun combat ne correspond à cette recherche.")
                    .color(tokens::TEXT_DISABLED),
            );
        }
        for group in &groups {
            if let Some(section) = &group.section {
                section_header(ui, width, section);
            }
            let collapsed = collapsed_groups.contains(&group.key);
            if items::day_header(
                ui,
                width,
                &group.label,
                DayTail::Text(&group.count),
                !collapsed,
            ) && !collapsed_groups.remove(&group.key)
            {
                collapsed_groups.insert(group.key.clone());
            }
            if collapsed {
                continue;
            }
            for entry in &group.entries {
                match entry {
                    Entry::Single(fight) => {
                        fight_block(ui, width, fight, false, detail, ctx, textures);
                    }
                    Entry::Run { dungeon_id, fights } => {
                        let key = entry.key();
                        let open = expanded_runs.contains(&key);
                        if run_summary(ui, width, *dungeon_id, fights, open, ctx, textures) {
                            if open {
                                expanded_runs.remove(&key);
                            } else {
                                expanded_runs.insert(key);
                            }
                        }
                        if open {
                            for fight in fights {
                                fight_block(ui, width, fight, true, detail, ctx, textures);
                            }
                        }
                    }
                }
            }
            ui.add_space(6.0);
        }
        pager.footer(ui, width, &times, shown, "historique.combats");
    });
}

/// Titre d'une section du regroupement « Donjons & familles » (non cliquable).
fn section_header(ui: &mut Ui, width: f32, title: &str) {
    ui.add_space(6.0);
    let (row, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), Sense::hover());
    ui.painter_at(row).text(
        egui::pos2(row.left() + 4.0, row.center().y),
        Align2::LEFT_CENTER,
        title.to_uppercase(),
        text::label_strong_font(ui.ctx(), 12.0),
        tokens::OVERLAY_TEXT_MUTED,
    );
}

/// Un combat : son résumé, et son détail s'il est déplié.
#[allow(clippy::too_many_arguments)]
fn fight_block(
    ui: &mut Ui,
    width: f32,
    fight: &FightRow,
    in_run: bool,
    detail: &mut DetailState,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let open = detail.expanded.contains(&fight.id);
    if fight_summary(ui, width, fight, in_run, open, ctx, textures) {
        if open {
            detail.expanded.remove(&fight.id);
        } else {
            detail.expanded.insert(fight.id);
        }
    }
    if open {
        fight_detail(ui, width, fight, detail, ctx, textures);
    }
}

/// Le cadre commun d'une ligne de résumé : fond au survol, horloge, illustration. Rend le rectangle,
/// le bord droit de l'illustration et le clic.
#[allow(clippy::too_many_arguments)]
fn summary_frame(
    ui: &mut Ui,
    width: f32,
    indent: f32,
    started_at: i64,
    icon: Option<IconRef>,
    id: egui::Id,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) -> (Rect, f32, egui::Response) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, SUMMARY_HEIGHT), Sense::click());
    let painter = ui.painter_at(rect);
    if response.hovered() {
        painter.rect_filled(
            rect,
            4.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, 10),
        );
    }
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 0, 0, 70)),
    );
    let left = rect.left() + 12.0 + indent;
    items::entry_clock(ui, egui::pos2(left + 8.0, rect.center().y), started_at, id);
    let image = Rect::from_center_size(
        egui::pos2(left + 26.0 + IMAGE_SIZE / 2.0, rect.center().y),
        egui::vec2(IMAGE_SIZE, IMAGE_SIZE),
    );
    if let Some(icon) = &icon {
        super::paint_remote_icon(ui, &painter, image, icon, ctx.remote_icons, textures);
    }
    (rect, image.right(), response)
}

fn paint_caret(ui: &Ui, painter: &egui::Painter, rect: Rect, open: bool) {
    super::paint_caret(
        ui,
        painter,
        egui::pos2(rect.right() - 16.0, rect.center().y),
        open,
        tokens::TEXT_GOLD,
    );
}

/// Résumé d'un combat : heure, illustration, Victoire / Défaite, `N tours (durée)`. Rend `true`
/// au clic.
fn fight_summary(
    ui: &mut Ui,
    width: f32,
    fight: &FightRow,
    in_run: bool,
    open: bool,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) -> bool {
    let icon = illustration(fight, ctx.catalog, ctx.dungeons)
        .and_then(|p| ctx.catalog.find_monster_icon(&p.name, p.monster_id));
    let (rect, image_right, response) = summary_frame(
        ui,
        width,
        if in_run { 24.0 } else { 0.0 },
        fight.started_at,
        icon,
        ui.id().with(("combat-horloge", fight.id)),
        ctx,
        textures,
    );
    let painter = ui.painter_at(rect);
    let (label, color) = if fight.won {
        ("Victoire", WON)
    } else {
        ("Défaite", LOST)
    };
    let font = text::label_strong_font(ui.ctx(), 12.0);
    let galley = painter.layout_no_wrap(label.to_string(), font, color);
    let badge = Rect::from_min_size(
        egui::pos2(image_right + 12.0, rect.center().y - 11.0),
        egui::vec2(galley.size().x + 16.0, 22.0),
    );
    painter.rect_filled(badge, 11.0, color.gamma_multiply(0.18));
    painter.rect_stroke(
        badge,
        11.0,
        egui::Stroke::new(1.0, color.gamma_multiply(0.7)),
        egui::StrokeKind::Inside,
    );
    painter.galley(badge.center() - galley.size() / 2.0, galley, color);
    painter.text(
        egui::pos2(badge.right() + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        format!(
            "{} tours ({})",
            fight.turns,
            duration(fight.duration_ms.unwrap_or(0))
        ),
        text::label_font(ui.ctx(), items::FONT_SIZE),
        tokens::OVERLAY_TEXT_MUTED,
    );
    paint_caret(ui, &painter, rect, open);
    response.clicked()
}

/// Résumé d'un run de donjon : heure, illustration du boss, nom du donjon, `N combats - durée`.
fn run_summary(
    ui: &mut Ui,
    width: f32,
    dungeon_id: Option<i64>,
    fights: &[&FightRow],
    open: bool,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) -> bool {
    let representative = fights[0];
    let boss = fights
        .iter()
        .find(|fight| fight.dungeon_id.is_some())
        .copied()
        .unwrap_or(representative);
    let icon = illustration(boss, ctx.catalog, ctx.dungeons)
        .and_then(|p| ctx.catalog.find_monster_icon(&p.name, p.monster_id));
    let (rect, image_right, response) = summary_frame(
        ui,
        width,
        0.0,
        representative.started_at,
        icon,
        ui.id().with(("run-horloge", representative.id)),
        ctx,
        textures,
    );
    let painter = ui.painter_at(rect);
    let name = dungeon_id
        .and_then(|id| ctx.dungeons.find_by_id(id))
        .map_or("Donjon", |dungeon| dungeon.fr.as_str());
    let title = painter.text(
        egui::pos2(image_right + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        name,
        text::label_strong_font(ui.ctx(), items::FONT_SIZE),
        tokens::TEXT_GOLD,
    );
    let total: i64 = fights.iter().filter_map(|fight| fight.duration_ms).sum();
    painter.text(
        egui::pos2(title.right() + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        format!("{} - {}", fight_count(fights.len()), duration(total)),
        text::label_font(ui.ctx(), items::FONT_SIZE),
        tokens::OVERLAY_TEXT_MUTED,
    );
    paint_caret(ui, &painter, rect, open);
    response.clicked()
}

/// Le détail d'un combat déplié.
fn fight_detail(
    ui: &mut Ui,
    width: f32,
    fight: &FightRow,
    detail: &mut DetailState,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let inner_width = width - 24.0;
    let metric = detail.metric.entry(fight.id).or_default();
    let frame_top = ui.cursor().top();
    let shape = ui.painter().add(egui::Shape::Noop);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        // Le switch de grandeur du panneau Combat de l'overlay : mêmes pictogrammes, même socle
        // de premier plan, mêmes infobulles (`CombatMetric::tooltip`).
        ui.add(
            design::switch(metric)
                .slot(Metric::Damage, "Dégâts infligés")
                .icon(design::DsIcon::MetricDamage)
                .slot(Metric::Armor, "Armure donnée")
                .icon(design::DsIcon::MetricArmor)
                .slot(Metric::Heal, "Soins prodigués")
                .icon(design::DsIcon::MetricHeal)
                .variant(design::SwitchVariant::FirstPlan)
                .log_name("historique.combats.grandeur"),
        );
    });
    ui.add_space(8.0);
    let column_gap = 16.0;
    let column_width = (inner_width - column_gap) / 2.0;
    let metric = *metric;
    let allies: Vec<&ParticipantRow> = fight.participants.iter().filter(|p| p.is_ally).collect();
    let enemies: Vec<&ParticipantRow> = fight.participants.iter().filter(|p| !p.is_ally).collect();
    // Les sorts des combattants dépliés, lus une fois par grandeur.
    for participant in &fight.participants {
        let key = participant_key(fight.id, participant);
        if detail.open_participants.contains(&key) {
            detail.spells.entry((key, metric)).or_insert_with(|| {
                ctx.store
                    .map(|store| {
                        store
                            .participant_spells(
                                fight.id,
                                &participant.name,
                                participant.instance_index,
                                metric.column(),
                            )
                            .unwrap_or_else(|err| {
                                tracing::warn!(%err, "historique : sorts illisibles");
                                Vec::new()
                            })
                    })
                    .unwrap_or_default()
            });
        }
    }
    let spell_rows = |participant: &ParticipantRow| -> usize {
        let key = (participant_key(fight.id, participant), metric);
        if detail.open_participants.contains(&key.0) {
            detail.spells.get(&key).map_or(0, Vec::len).max(1)
        } else {
            0
        }
    };
    let column_height = |side: &[&ParticipantRow]| -> f32 {
        side.iter()
            .map(|p| PARTICIPANT_ROW + spell_rows(p) as f32 * SPELL_ROW)
            .sum::<f32>()
    };
    let (columns, _) = ui.allocate_exact_size(
        egui::vec2(
            width,
            22.0 + column_height(&allies).max(column_height(&enemies)),
        ),
        Sense::hover(),
    );
    let mut toggled = None;
    for (index, (title, side, bar)) in [
        ("Alliés", &allies, ALLY_BAR),
        ("Ennemis", &enemies, ENEMY_BAR),
    ]
    .into_iter()
    .enumerate()
    {
        let column = Rect::from_min_size(
            egui::pos2(
                columns.left() + 12.0 + index as f32 * (column_width + column_gap),
                columns.top(),
            ),
            egui::vec2(column_width, columns.height()),
        );
        let clicked = participants_column(
            ui,
            column,
            Column {
                title,
                side,
                metric,
                bar,
                fight_id: fight.id,
                detail,
            },
            ctx,
            textures,
        );
        toggled = toggled.or(clicked);
    }
    if let Some(key) = toggled {
        if !detail.open_participants.remove(&key) {
            detail.open_participants.insert(key);
        }
    }
    if fight.won && (fight.kamas_gained.unwrap_or(0) > 0 || !fight.loot.is_empty()) {
        ui.add_space(10.0);
        let kamas = fight.kamas_gained.filter(|&kamas| kamas > 0);
        let count = if fight.loot.is_empty() {
            String::new()
        } else {
            format!("{} objet(s)", fight.loot.len())
        };
        section_title(ui, width, "Butin", kamas, &count);
        if fight.loot.len() > 1 {
            loot_controls(ui, inner_width, detail, ctx, textures);
            ui.add_space(6.0);
        }
        let query = normalize_wakfu_name(detail.loot_query.trim());
        let mut lines = sorted_loot(&fight.loot, &query, detail.loot_sort, ctx.catalog);
        if detail.loot_reverse {
            lines.reverse();
        }
        if lines.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(18.0);
                ui.add(
                    design::label("Aucun objet ne correspond à cette recherche.")
                        .color(tokens::TEXT_DISABLED),
                );
            });
        }
        for line in lines {
            let (row, _) =
                ui.allocate_exact_size(egui::vec2(width, LOOT_ROW.height + 2.0), Sense::hover());
            let row = Rect::from_min_size(
                row.min + egui::vec2(12.0, 1.0),
                egui::vec2(inner_width, LOOT_ROW.height),
            );
            items::paint_compact_item(ui, ui.painter(), row, line, LOOT_ROW, ctx, textures);
        }
    }
    let xp: Vec<&ParticipantRow> = allies.iter().copied().filter(|p| p.xp_gained > 0).collect();
    if !xp.is_empty() {
        ui.add_space(10.0);
        section_title(ui, width, "Expérience", None, "");
        for participant in xp {
            let (row, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), Sense::hover());
            let painter = ui.painter_at(row);
            painter.text(
                egui::pos2(row.left() + 18.0, row.center().y),
                Align2::LEFT_CENTER,
                &participant.name,
                text::label_font(ui.ctx(), 13.0),
                Color32::WHITE,
            );
            painter.text(
                egui::pos2(row.right() - 18.0, row.center().y),
                Align2::RIGHT_CENTER,
                format!("+{}", format::thousands(participant.xp_gained)),
                text::label_strong_font(ui.ctx(), 13.0),
                WON,
            );
        }
    }
    ui.add_space(10.0);
    let frame = Rect::from_min_max(
        egui::pos2(ui.max_rect().left(), frame_top),
        egui::pos2(ui.max_rect().left() + width, ui.cursor().top()),
    );
    ui.painter()
        .set(shape, egui::Shape::rect_filled(frame, 0.0, DETAIL_FILL));
}

/// Recherche et tri du butin (`loot-controls-row` du site) : le champ prend toute la largeur
/// laissée par le switch `Az` / `Nb` / gemme de rareté ; un reclic sur le tri actif inverse son
/// sens, que l'infobulle de la case dit par ▼ / ▲.
fn loot_controls(
    ui: &mut Ui,
    inner_width: f32,
    detail: &mut DetailState,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let gem = textures
        .resolve(
            ui.ctx(),
            ctx.remote_icons,
            &IconRef::for_rarity(WakfuRarity::Old),
        )
        .map(|texture| texture.id());
    let tooltip = |mode: LootSort, label: &str| {
        if detail.loot_sort == mode {
            format!("{label} {}", if detail.loot_reverse { "▲" } else { "▼" })
        } else {
            label.to_string()
        }
    };
    let (name_tip, quantity_tip, rarity_tip) = (
        tooltip(LootSort::Name, "Nom"),
        tooltip(LootSort::Quantity, "Quantité"),
        tooltip(LootSort::Rarity, "Rareté"),
    );
    let before = detail.loot_sort;
    let switch = design::switch(&mut detail.loot_sort)
        .slot(LootSort::Name, name_tip)
        .glyph("Az")
        .slot(LootSort::Quantity, quantity_tip)
        .glyph("Nb")
        .slot(LootSort::Rarity, rarity_tip)
        .image(gem)
        .reclickable(true)
        .log_name("historique.combats.butin.tri");
    let switch_width = switch.desired_size().x;
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.add(
            design::input(&mut detail.loot_query)
                .placeholder("Rechercher un objet...")
                .clearable(true)
                .width(inner_width - switch_width - 8.0)
                .log_name("historique.combats.butin.recherche"),
        );
        changed = ui.add(switch).changed();
    });
    if changed {
        detail.loot_reverse = detail.loot_sort == before && !detail.loot_reverse;
    }
}

/// Titre de section : le titre, puis les kamas (icône comprise) et `tail`.
fn section_title(ui: &mut Ui, width: f32, title: &str, kamas: Option<i64>, tail: &str) {
    let (row, _) = ui.allocate_exact_size(egui::vec2(width, 22.0), Sense::hover());
    let painter = ui.painter_at(row);
    let title = painter.text(
        egui::pos2(row.left() + 12.0, row.center().y),
        Align2::LEFT_CENTER,
        title,
        text::label_strong_font(ui.ctx(), 13.0),
        tokens::TEXT_GOLD,
    );
    let mut x = title.right() + 10.0;
    if let Some(kamas) = kamas {
        x = super::paint_amount_left(
            ui,
            &painter,
            egui::pos2(x, row.center().y),
            &format::thousands(kamas),
            text::label_strong_font(ui.ctx(), 13.0),
            tokens::TEXT_GOLD,
        ) + 10.0;
    }
    painter.text(
        egui::pos2(x, row.center().y),
        Align2::LEFT_CENTER,
        tail,
        text::label_font(ui.ctx(), 13.0),
        tokens::OVERLAY_TEXT_MUTED,
    );
}

/// Une colonne de combattants : titre, puis une ligne par combattant (icône, nom, valeur, part du
/// camp et barre proportionnelle), triés par valeur décroissante.
#[allow(clippy::too_many_arguments)]
/// Ce qu'une colonne Alliés / Ennemis peint.
struct Column<'a> {
    title: &'a str,
    side: &'a [&'a ParticipantRow],
    metric: Metric,
    bar: Color32,
    fight_id: i64,
    detail: &'a DetailState,
}

/// Une colonne de combattants, chacun dépliable sur ses sorts. Rend le combattant cliqué.
fn participants_column(
    ui: &Ui,
    column: Rect,
    Column {
        title,
        side,
        metric,
        bar,
        fight_id,
        detail,
    }: Column<'_>,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) -> Option<ParticipantKey> {
    let mut clicked = None;
    let painter = ui.painter_at(column.expand(2.0));
    painter.text(
        column.left_top() + egui::vec2(0.0, 9.0),
        Align2::LEFT_CENTER,
        title,
        text::label_strong_font(ui.ctx(), 12.0),
        tokens::OVERLAY_TEXT_MUTED,
    );
    let mut rows: Vec<&ParticipantRow> = side.to_vec();
    rows.sort_by_key(|p| std::cmp::Reverse(metric.of(p)));
    let total: i64 = rows.iter().map(|p| metric.of(p)).sum();
    let best = rows.first().map_or(0, |p| metric.of(p)).max(1);
    let mut top = column.top() + 22.0;
    for participant in rows {
        let row = Rect::from_min_size(
            egui::pos2(column.left(), top),
            egui::vec2(column.width(), PARTICIPANT_ROW - 4.0),
        );
        let value = metric.of(participant);
        let key = participant_key(fight_id, participant);
        let open = detail.open_participants.contains(&key);
        let response = ui
            .interact(
                row,
                ui.id().with(("historique.combattant", &key)),
                Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() {
            tracing::info!(
                ">>> clic historique.combats.combattant {} ({})",
                participant.name,
                if open { "replié" } else { "déplié" }
            );
            clicked = Some(key.clone());
        }
        // Barre en fond de ligne, proportionnelle au meilleur du camp.
        painter.rect_filled(row, 3.0, BAR_TRACK);
        if response.hovered() {
            painter.rect_filled(row, 3.0, Color32::from_white_alpha(10));
        }
        let fill = Rect::from_min_size(
            row.min,
            egui::vec2(row.width() * value as f32 / best as f32, row.height()),
        );
        painter.rect_filled(fill, 3.0, bar.gamma_multiply(0.45));
        let icon_rect = Rect::from_center_size(
            egui::pos2(row.left() + 14.0, row.center().y),
            egui::vec2(20.0, 20.0),
        );
        // Un allié : le portrait de sa classe (grisé s'il est KO) ; un ennemi, ou un allié sans
        // classe connue : l'icône du monstre.
        let portrait = participant
            .class_name
            .as_deref()
            .zip(ctx.portraits)
            .and_then(|(class, atlas)| {
                atlas.texture(class, overlay_engine::Gender::M, participant.defeated)
            });
        if let Some(texture) = portrait {
            painter.image(
                texture.id(),
                icon_rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else if let Some(icon) = ctx
            .catalog
            .find_monster_icon(&participant.name, participant.monster_id)
        {
            super::paint_remote_icon(ui, &painter, icon_rect, &icon, ctx.remote_icons, textures);
        }
        let share = if total > 0 {
            format!("{:.0} %", value as f64 * 100.0 / total as f64)
        } else {
            "0 %".to_string()
        };
        let share_rect = painter.text(
            egui::pos2(row.right() - 6.0, row.center().y),
            Align2::RIGHT_CENTER,
            share,
            text::label_font(ui.ctx(), 12.0),
            tokens::OVERLAY_TEXT_MUTED,
        );
        let value_rect = painter.text(
            egui::pos2(share_rect.left() - 10.0, row.center().y),
            Align2::RIGHT_CENTER,
            format::thousands(value),
            text::label_strong_font(ui.ctx(), 13.0),
            Color32::WHITE,
        );
        super::paint_caret(
            ui,
            &painter,
            egui::pos2(icon_rect.right() + 12.0, row.center().y),
            open,
            tokens::OVERLAY_TEXT_MUTED,
        );
        let name_rect = Rect::from_min_max(
            egui::pos2(icon_rect.right() + 22.0, row.top()),
            egui::pos2(value_rect.left() - 8.0, row.bottom()),
        );
        let name = if participant.defeated {
            format!("{} (KO)", participant.name)
        } else if participant.fled {
            format!("{} (fuite)", participant.name)
        } else {
            participant.name.clone()
        };
        painter.with_clip_rect(name_rect).text(
            name_rect.left_center(),
            Align2::LEFT_CENTER,
            name,
            text::label_font(ui.ctx(), 13.0),
            if participant.defeated {
                tokens::OVERLAY_TEXT_MUTED
            } else {
                Color32::WHITE
            },
        );
        top += PARTICIPANT_ROW;
        if open {
            let spells = detail
                .spells
                .get(&(key, metric))
                .map_or(&[][..], Vec::as_slice);
            top = paint_spells(
                ui,
                &painter,
                column,
                top,
                participant,
                spells,
                ctx,
                textures,
            );
        }
    }
    clicked
}

/// Les sorts d'un combattant déplié, sous sa ligne : nom, part par élément (en couleur), total.
/// Rend le haut de la ligne suivante.
#[allow(clippy::too_many_arguments)]
fn paint_spells(
    ui: &Ui,
    painter: &egui::Painter,
    column: Rect,
    mut top: f32,
    caster: &ParticipantRow,
    spells: &[SpellLine],
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) -> f32 {
    let left = column.left() + 34.0;
    let right = column.right() - 6.0;
    if spells.is_empty() {
        painter.text(
            egui::pos2(left, top + SPELL_ROW / 2.0 - 2.0),
            Align2::LEFT_CENTER,
            "Aucun sort enregistré.",
            text::label_font(ui.ctx(), 12.0),
            tokens::TEXT_DISABLED,
        );
        return top + SPELL_ROW;
    }
    for spell in spells {
        let center_y = top + SPELL_ROW / 2.0 - 2.0;
        let total_rect = painter.text(
            egui::pos2(right, center_y),
            Align2::RIGHT_CENTER,
            format::thousands(spell.total),
            text::label_strong_font(ui.ctx(), 12.0),
            Color32::WHITE,
        );
        let mut x = total_rect.left() - 10.0;
        let mut elements = spell.by_element.clone();
        elements.sort_by_key(|(_, value)| std::cmp::Reverse(*value));
        // Un seul élément égal au total n'apprend rien : sa couleur suffit, sur le nom du sort.
        let single = elements.len() == 1;
        if !single {
            for (element, value) in elements.iter().rev() {
                let rect = painter.text(
                    egui::pos2(x, center_y),
                    Align2::RIGHT_CENTER,
                    format::thousands(*value),
                    text::label_font(ui.ctx(), 11.0),
                    element_color(element),
                );
                x = rect.left() - 8.0;
            }
        }
        // L'icône du sort, résolue comme sur le panneau Combat : référentiel des classes pour un
        // allié (par sa classe), des monstres pour un ennemi (par son id).
        let resolved = overlay_engine::resolve_spell(
            caster.is_ally,
            caster.class_name.as_deref(),
            caster.monster_id,
            &spell.spell,
        );
        let icon_rect = Rect::from_center_size(
            egui::pos2(left + SPELL_ICON / 2.0, center_y),
            egui::vec2(SPELL_ICON, SPELL_ICON),
        );
        painter.rect_filled(
            icon_rect,
            3.0,
            Color32::from_rgba_premultiplied(0, 0, 0, 90),
        );
        if let Some(icon) = resolved.and_then(|spell| spell.icon) {
            super::paint_remote_icon(ui, painter, icon_rect, icon, ctx.remote_icons, textures);
        }
        let name_left = icon_rect.right() + 6.0;
        let name_rect = Rect::from_min_max(
            egui::pos2(name_left, top),
            egui::pos2(x, top + SPELL_ROW - 2.0),
        );
        let tint = elements
            .first()
            .filter(|_| single)
            .map_or(tokens::OVERLAY_TEXT_MUTED, |(element, _)| {
                element_color(element)
            });
        painter.with_clip_rect(name_rect).text(
            egui::pos2(name_left, center_y),
            Align2::LEFT_CENTER,
            resolved.map_or(spell.spell.as_str(), |spell| spell.name),
            text::label_font(ui.ctx(), 12.0),
            tint,
        );
        top += SPELL_ROW;
    }
    top
}

#[cfg(test)]
mod tests {
    use super::*;

    fn participant(name: &str, ally: bool, damage: i64) -> ParticipantRow {
        ParticipantRow {
            instance_index: 0,
            name: name.to_string(),
            is_ally: ally,
            monster_id: None,
            class_name: None,
            damage,
            heal: 0,
            armor: 0,
            defeated: false,
            fled: false,
            xp_gained: 0,
        }
    }

    fn fight(id: i64, at: i64, run: Option<&str>, enemies: &[(&str, i64)]) -> FightRow {
        let mut participants = vec![participant("Moi", true, 100)];
        participants.extend(
            enemies
                .iter()
                .map(|(name, damage)| participant(name, false, *damage)),
        );
        FightRow {
            id,
            started_at: at,
            duration_ms: Some(65_400),
            won: true,
            turns: 3,
            xp_gained: 0,
            kamas_gained: None,
            dungeon_id: run.map(|_| 7),
            dungeon_run_key: run.map(str::to_string),
            fight_type: None,
            participants,
            loot: Vec::new(),
        }
    }

    #[test]
    fn les_combats_d_un_run_sont_reunis() {
        let fights = [
            fight(3, 3_000, Some("run"), &[("Boss", 1)]),
            fight(2, 2_000, None, &[("Bouftou", 1)]),
            fight(1, 1_000, Some("run"), &[("Salle", 1)]),
        ];
        let refs: Vec<&FightRow> = fights.iter().collect();
        let entries = entries(&refs);
        assert_eq!(entries.len(), 2);
        match &entries[0] {
            Entry::Run { fights, dungeon_id } => {
                assert_eq!(fights.iter().map(|f| f.id).collect::<Vec<_>>(), vec![3, 1]);
                assert_eq!(*dungeon_id, Some(7));
            }
            other => panic!("run attendu, trouvé {other:?}"),
        }
        assert!(matches!(entries[1], Entry::Single(f) if f.id == 2));
        // Une clé de run seule ne fait pas de run.
        let lone = [fight(4, 4_000, Some("seul"), &[])];
        let refs: Vec<&FightRow> = lone.iter().collect();
        assert!(matches!(super::entries(&refs)[0], Entry::Single(_)));
    }

    #[test]
    fn le_butin_se_filtre_et_se_trie() {
        let catalog = CatalogIndex::default();
        let line = |name: &str, quantity| ItemLine {
            item_id: None,
            item_name: Some(name.to_string()),
            quantity,
        };
        let loot = [line("Plume", 2), line("Étoffe", 9), line("Bave", 5)];
        let names = |lines: Vec<&ItemLine>| -> Vec<String> {
            lines
                .iter()
                .map(|l| l.item_name.clone().unwrap_or_default())
                .collect()
        };
        assert_eq!(
            names(sorted_loot(&loot, "", LootSort::Quantity, &catalog)),
            vec!["Étoffe", "Bave", "Plume"]
        );
        assert_eq!(
            names(sorted_loot(&loot, "", LootSort::Name, &catalog)),
            vec!["Bave", "Étoffe", "Plume"],
            "accents ignorés"
        );
        assert_eq!(
            names(sorted_loot(
                &loot,
                &normalize_wakfu_name("étof"),
                LootSort::Name,
                &catalog
            )),
            vec!["Étoffe"]
        );
    }

    #[test]
    fn hors_donjon_les_combats_vont_sous_familles() {
        let catalog = CatalogIndex::default();
        let dungeons = DungeonIndex::default();
        let fights = [
            fight(3, 3_000, None, &[("Bouftou", 5), ("Bouftou", 5)]),
            fight(2, 2_000, None, &[("Tofu", 5)]),
            fight(1, 1_000, None, &[("Bouftou", 5), ("Bouftou", 9)]),
        ];
        let refs: Vec<&FightRow> = fights.iter().collect();
        let all = entries(&refs);
        let kept: Vec<&Entry> = all.iter().collect();
        let groups = type_groups(&kept, &catalog, &dungeons);
        let labels: Vec<&str> = groups.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, vec!["Bouftou", "Tofu"]);
        assert_eq!(groups[0].section.as_deref(), Some("Familles"));
        assert_eq!(groups[1].section, None);
        assert_eq!(groups[0].count, "2 combats");
    }

    #[test]
    fn illustration_et_recherche() {
        let catalog = CatalogIndex::default();
        let dungeons = DungeonIndex::default();
        let crowd = fight(1, 0, None, &[("Bouftou", 5), ("Bouftou", 5), ("Chef", 50)]);
        assert_eq!(
            illustration(&crowd, &catalog, &dungeons).map(|p| p.name.as_str()),
            Some("Bouftou"),
            "le plus nombreux"
        );
        let duel = fight(2, 0, None, &[("Tofu", 5), ("Chef", 50)]);
        assert_eq!(
            illustration(&duel, &catalog, &dungeons).map(|p| p.name.as_str()),
            Some("Chef"),
            "sinon le plus de dégâts"
        );
        assert!(matches(&crowd, &normalize_wakfu_name("bouf"), &catalog));
        assert!(matches(&crowd, "moi", &catalog));
        assert!(!matches(&crowd, "tofu", &catalog));
        assert_eq!(duration(65_400), "1 min 5sec.");
        assert_eq!(fight_count(1), "1 combat");
    }
}
