//! Vue **Combats** — port de `features/fight-history/` (site) : recherche sur les combattants et
//! le butin, groupes par jour repliables (avec « Tout replier »), runs de donjon réunis sous une
//! entrée repliable, et pour chaque combat son résumé (heure, illustration, résultat, tours et
//! durée) puis, déplié, son détail : alliés et ennemis en dégâts, armure ou soins, butin et XP.
//!
//! Le modèle ([`entries`], [`matches`], [`illustration`]) est séparé du rendu : c'est lui que les
//! tests comparent au site.
//!
//! **Pas encore porté** : le regroupement « Donjons & familles », la vue par tour et la ventilation
//! par sort (le JSON des sorts n'est pas relu), le tri et la recherche du butin.

use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::NaiveDate;
use egui::{Align2, Color32, Rect, Sense, Ui};
use overlay_engine::roster::normalize_wakfu_name;
use overlay_engine::{CatalogIndex, DungeonIndex, IconRef};
use overlay_history::read::{FightRow, ParticipantRow};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
}

/// État de la vue.
#[derive(Default)]
pub struct CombatsState {
    pub list: ListState<FightRow>,
    /// Combats dépliés.
    pub expanded: HashSet<i64>,
    /// Runs de donjon dépliés (clé : [`Entry::key`]).
    pub expanded_runs: HashSet<i64>,
    /// Grandeur choisie pour chaque combat déplié.
    pub metric: HashMap<i64, Metric>,
}

const SUMMARY_HEIGHT: f32 = 40.0;
const IMAGE_SIZE: f32 = 28.0;
const TOGGLE_ALL_WIDTH: f32 = 140.0;
/// Victoire / défaite (`.fight-badge.won`, `.lost`).
const WON: Color32 = items::GAIN;
const LOST: Color32 = items::LOSS;
const DETAIL_FILL: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 70);
const BAR_TRACK: Color32 = Color32::from_rgba_premultiplied(14, 14, 14, 14);
const ALLY_BAR: Color32 = Color32::from_rgb(0x3E, 0x9B, 0xD6);
const ENEMY_BAR: Color32 = Color32::from_rgb(0xC9, 0x52, 0x4A);
const PARTICIPANT_ROW: f32 = 28.0;
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
        expanded,
        expanded_runs,
        metric,
    } = state;
    let ListState {
        query: search,
        collapsed: collapsed_days,
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
    let groups = items::group_by_day(
        kept.iter().copied(),
        |entry| entry.representative().started_at,
        items::SortOrder::Newest,
    );
    let all_days: BTreeSet<NaiveDate> = groups.iter().map(|(day, _)| *day).collect();

    // Recherche, puis « Tout replier » / « Tout déplier ».
    ui.horizontal(|ui| {
        ui.add(
            design::input(search)
                .placeholder("Rechercher un ennemi, un allié, un objet...")
                .clearable(true)
                .width(zones.inner.width() - TOGGLE_ALL_WIDTH - 12.0)
                .log_name("historique.combats.recherche"),
        );
        let any_open = all_days.iter().any(|day| !collapsed_days.contains(day));
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
                collapsed_days.extend(all_days.iter().copied());
            } else {
                collapsed_days.clear();
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
        for (day, day_entries) in &groups {
            let collapsed = collapsed_days.contains(day);
            let count: usize = day_entries.iter().map(|entry| entry.fights().len()).sum();
            if items::day_header(
                ui,
                width,
                &format::relative_day(*day, ctx.today),
                DayTail::Text(&fight_count(count)),
                !collapsed,
            ) {
                ListState::<FightRow>::toggle(collapsed_days, *day);
            }
            if collapsed {
                continue;
            }
            for entry in day_entries {
                match entry {
                    Entry::Single(fight) => {
                        fight_block(ui, width, fight, false, expanded, metric, ctx, textures);
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
                                fight_block(
                                    ui, width, fight, true, expanded, metric, ctx, textures,
                                );
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

/// Un combat : son résumé, et son détail s'il est déplié.
#[allow(clippy::too_many_arguments)]
fn fight_block(
    ui: &mut Ui,
    width: f32,
    fight: &FightRow,
    in_run: bool,
    expanded: &mut HashSet<i64>,
    metrics: &mut HashMap<i64, Metric>,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let open = expanded.contains(&fight.id);
    if fight_summary(ui, width, fight, in_run, open, ctx, textures) {
        if open {
            expanded.remove(&fight.id);
        } else {
            expanded.insert(fight.id);
        }
    }
    if open {
        let metric = metrics.entry(fight.id).or_default();
        fight_detail(ui, width, fight, metric, ctx, textures);
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
    metric: &mut Metric,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let inner_width = width - 24.0;
    let frame_top = ui.cursor().top();
    let shape = ui.painter().add(egui::Shape::Noop);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.add(
            design::switch(metric)
                .slot(Metric::Damage, "Dégâts")
                .slot(Metric::Armor, "Armure")
                .slot(Metric::Heal, "Soins")
                .width(300.0)
                .log_name("historique.combats.grandeur"),
        );
    });
    ui.add_space(8.0);
    let column_gap = 16.0;
    let column_width = (inner_width - column_gap) / 2.0;
    let allies: Vec<&ParticipantRow> = fight.participants.iter().filter(|p| p.is_ally).collect();
    let enemies: Vec<&ParticipantRow> = fight.participants.iter().filter(|p| !p.is_ally).collect();
    let rows = allies.len().max(enemies.len()) as f32;
    let (columns, _) = ui.allocate_exact_size(
        egui::vec2(width, 22.0 + rows * PARTICIPANT_ROW),
        Sense::hover(),
    );
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
        participants_column(ui, column, title, side, *metric, bar, ctx, textures);
    }
    if fight.won && (fight.kamas_gained.unwrap_or(0) > 0 || !fight.loot.is_empty()) {
        ui.add_space(10.0);
        section_title(ui, width, "Butin", &loot_summary(fight));
        for line in &fight.loot {
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
        section_title(ui, width, "Expérience", "");
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

/// `12 345 ₭ · 7 objet(s)`.
fn loot_summary(fight: &FightRow) -> String {
    let mut parts = Vec::new();
    if let Some(kamas) = fight.kamas_gained.filter(|&kamas| kamas > 0) {
        parts.push(format::kamas(kamas));
    }
    if !fight.loot.is_empty() {
        parts.push(format!("{} objet(s)", fight.loot.len()));
    }
    parts.join(" · ")
}

fn section_title(ui: &mut Ui, width: f32, title: &str, tail: &str) {
    let (row, _) = ui.allocate_exact_size(egui::vec2(width, 22.0), Sense::hover());
    let painter = ui.painter_at(row);
    let title = painter.text(
        egui::pos2(row.left() + 12.0, row.center().y),
        Align2::LEFT_CENTER,
        title,
        text::label_strong_font(ui.ctx(), 13.0),
        tokens::TEXT_GOLD,
    );
    painter.text(
        egui::pos2(title.right() + 10.0, row.center().y),
        Align2::LEFT_CENTER,
        tail,
        text::label_font(ui.ctx(), 13.0),
        tokens::OVERLAY_TEXT_MUTED,
    );
}

/// Une colonne de combattants : titre, puis une ligne par combattant (icône, nom, valeur, part du
/// camp et barre proportionnelle), triés par valeur décroissante.
#[allow(clippy::too_many_arguments)]
fn participants_column(
    ui: &Ui,
    column: Rect,
    title: &str,
    side: &[&ParticipantRow],
    metric: Metric,
    bar: Color32,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
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
        // Barre en fond de ligne, proportionnelle au meilleur du camp.
        painter.rect_filled(row, 3.0, BAR_TRACK);
        let fill = Rect::from_min_size(
            row.min,
            egui::vec2(row.width() * value as f32 / best as f32, row.height()),
        );
        painter.rect_filled(fill, 3.0, bar.gamma_multiply(0.45));
        let icon_rect = Rect::from_center_size(
            egui::pos2(row.left() + 14.0, row.center().y),
            egui::vec2(20.0, 20.0),
        );
        if !participant.is_ally {
            if let Some(icon) = ctx
                .catalog
                .find_monster_icon(&participant.name, participant.monster_id)
            {
                super::paint_remote_icon(
                    ui,
                    &painter,
                    icon_rect,
                    &icon,
                    ctx.remote_icons,
                    textures,
                );
            }
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
        let name_rect = Rect::from_min_max(
            egui::pos2(icon_rect.right() + 8.0, row.top()),
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn participant(name: &str, ally: bool, damage: i64) -> ParticipantRow {
        ParticipantRow {
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
