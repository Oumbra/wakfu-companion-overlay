//! Vue **Récap** — port de `features/session-recap/` (site), mode « cumulé » : une période (jour,
//! mois ou année, bornée dans le calendrier local), parcourue par précédent / suivant, et ses
//! agrégats (`HistoryStore::period_stats`) — bande d'indicateurs (kamas nets, XP, combats gagnés –
//! perdus, challenges réussis – échoués), détail des kamas, XP par personnage, butin et pactes.
//!
//! Trois affichages du détail (`detailMode` du site) : **Cumulé** (les sections ci-dessus),
//! **Donjon & Famille** (une ligne par donjon, rangée sous son type, puis une par famille de
//! monstres hors donjon) et **Type** (les types de donjon fusionnés, puis « Autres »). Chaque ligne
//! se déplie sur ses totaux, ses donjons (mode Type), l'XP par personnage et son butin.
//!
//! **Écarts** : pas de granularité « session » (le récap de la session en cours vit sur le jeu,
//! dans l'overlay), pas de sélecteur calendrier, pas de pierre de donjon épinglée en tête du butin
//! d'un type.

use std::borrow::Cow;
use std::collections::HashSet;

use chrono::{Datelike, Local, NaiveDate, TimeZone};
use egui::{Align2, Color32, Rect, Sense, Ui};
use overlay_engine::{DungeonIndex, WakfuDungeonType};
use overlay_history::read::ItemLine;
use overlay_history::stats::{GroupTotals, ItemTotal, PeriodStats, XpTotal};

use super::items::{self, CompactRow};
use super::{format, HistoryContext};
use crate::design::{self, text, tokens, DsIcon};
use crate::remote_icons::RemoteIconTextures;

/// La granularité de la période.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Granularity {
    #[default]
    Day,
    Month,
    Year,
}

const MONTHS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

/// Le premier jour de la période qui contient `day`.
pub fn period_start(granularity: Granularity, day: NaiveDate) -> NaiveDate {
    match granularity {
        Granularity::Day => day,
        Granularity::Month => day.with_day(1).unwrap_or(day),
        Granularity::Year => NaiveDate::from_ymd_opt(day.year(), 1, 1).unwrap_or(day),
    }
}

/// Le premier jour de la période suivante (`step` = 1) ou précédente (`step` = -1).
pub fn shift(granularity: Granularity, start: NaiveDate, step: i32) -> NaiveDate {
    match granularity {
        Granularity::Day => start + chrono::Duration::days(step as i64),
        Granularity::Month => {
            let months = start.year() * 12 + start.month0() as i32 + step;
            NaiveDate::from_ymd_opt(months.div_euclid(12), months.rem_euclid(12) as u32 + 1, 1)
                .unwrap_or(start)
        }
        Granularity::Year => NaiveDate::from_ymd_opt(start.year() + step, 1, 1).unwrap_or(start),
    }
}

/// Minuit local d'un jour, en ms UTC — sans piège d'heure d'été (le premier instant valide).
fn local_midnight_ms(day: NaiveDate) -> i64 {
    let midnight = day.and_hms_opt(0, 0, 0).unwrap_or_default();
    Local
        .from_local_datetime(&midnight)
        .earliest()
        .map_or(0, |moment| moment.timestamp_millis())
}

/// `[début, fin[` de la période, en ms UTC.
pub fn bounds(granularity: Granularity, start: NaiveDate) -> (i64, i64) {
    (
        local_midnight_ms(start),
        local_midnight_ms(shift(granularity, start, 1)),
    )
}

/// Le libellé de la période : « Aujourd'hui », « novembre 2026 », « 2026 ».
pub fn label(granularity: Granularity, start: NaiveDate, today: NaiveDate) -> String {
    match granularity {
        Granularity::Day => format::relative_day(start, today),
        Granularity::Month => format!("{} {}", MONTHS[start.month0() as usize], start.year()),
        Granularity::Year => start.year().to_string(),
    }
}

/// Kamas nets de la période : gains (combats, ventes, échanges reçus) moins dépenses (achats,
/// échanges donnés).
pub fn net_kamas(stats: &PeriodStats) -> i64 {
    let k = &stats.kamas;
    k.from_combat + k.from_hdv_sales + k.trades_acquired - k.spent_on_purchases - k.trades_given
}

/// L'affichage du détail (`DetailMode` du site).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailMode {
    #[default]
    Cumulative,
    ByGroup,
    ByType,
}

/// Une ligne dépliable des modes Donjon & Famille et Type (`RecapGroupRow` du site).
#[derive(Debug, Clone, PartialEq)]
pub struct RecapGroup<'a> {
    pub key: String,
    pub label: String,
    /// Titre de section à peindre AVANT cette ligne (mode Donjon & Famille).
    pub section: Option<&'static str>,
    /// L'id du donjon (son illustration : le boss).
    pub dungeon_id: Option<i64>,
    pub totals: Cow<'a, GroupTotals>,
    /// Type de donjon sans aucun combat sur la période : ligne grisée, non dépliable.
    pub disabled: bool,
    /// Mode Type : les donjons fusionnés et leur compte, du plus fréquent au moins fréquent.
    pub tiles: Vec<(String, i64)>,
}

/// `rowCount` du site : les donjons terminés, sinon les combats.
pub fn row_count(totals: &GroupTotals) -> i64 {
    if totals.dungeon_runs > 0 {
        totals.dungeon_runs
    } else {
        totals.fights
    }
}

fn dungeon_label(dungeons: &DungeonIndex, id: i64) -> String {
    dungeons
        .find_by_id(id)
        .map_or_else(|| "Donjon inconnu".to_string(), |d| d.fr.clone())
}

fn family_label(family: Option<i64>) -> String {
    family
        .and_then(super::family_name)
        .map_or_else(|| "Sans famille".to_string(), str::to_string)
}

/// Mode Donjon & Famille (`groupSections`) : lignes triées par compte décroissant, rangées sous le
/// type de leur donjon dans l'ordre fixe du site ; un donjon de type inconnu et les familles vont
/// sous « Familles », en dernier.
pub fn group_rows<'a>(stats: &'a PeriodStats, dungeons: &DungeonIndex) -> Vec<RecapGroup<'a>> {
    let mut rows: Vec<(Option<WakfuDungeonType>, RecapGroup)> = stats
        .dungeons
        .iter()
        .map(|(id, totals)| {
            (
                dungeons.find_by_id(*id).map(|d| d.dungeon_type),
                RecapGroup {
                    key: format!("donjon:{id}"),
                    label: dungeon_label(dungeons, *id),
                    section: None,
                    dungeon_id: Some(*id),
                    totals: Cow::Borrowed(totals),
                    disabled: false,
                    tiles: Vec::new(),
                },
            )
        })
        .chain(stats.families.iter().map(|(family, totals)| {
            (
                None,
                RecapGroup {
                    key: format!(
                        "famille:{}",
                        family.map_or("aucune".to_string(), |f| f.to_string())
                    ),
                    label: family_label(*family),
                    section: None,
                    dungeon_id: None,
                    totals: Cow::Borrowed(totals),
                    disabled: false,
                    tiles: Vec::new(),
                },
            )
        }))
        .collect();
    // Tri stable : à égalité, l'ordre des agrégats (combats décroissants).
    rows.sort_by_key(|(_, row)| std::cmp::Reverse(row_count(&row.totals)));
    let mut out = Vec::new();
    for (dungeon_type, title) in super::combats::DUNGEON_SECTIONS {
        let mut first = true;
        for (_, row) in rows.iter().filter(|(t, _)| *t == Some(dungeon_type)) {
            out.push(RecapGroup {
                section: first.then_some(title),
                ..row.clone()
            });
            first = false;
        }
    }
    let mut first = true;
    for (_, row) in rows.iter().filter(|(t, _)| t.is_none()) {
        out.push(RecapGroup {
            section: first.then_some("Familles"),
            ..row.clone()
        });
        first = false;
    }
    out
}

/// Fusionne des totaux (`mergeGroupTotals` du site) : sommes, XP par personnage et butin
/// additionnés (par id d'objet, sinon par nom).
pub fn merge_totals<'t>(rows: impl IntoIterator<Item = &'t GroupTotals>) -> GroupTotals {
    let mut merged = GroupTotals::default();
    let mut xp: Vec<XpTotal> = Vec::new();
    let mut loot: Vec<ItemTotal> = Vec::new();
    for row in rows {
        merged.fights += row.fights;
        merged.dungeon_runs += row.dungeon_runs;
        merged.won += row.won;
        merged.lost += row.lost;
        merged.kamas_gained += row.kamas_gained;
        merged.xp_gained += row.xp_gained;
        for entry in &row.xp_by_character {
            match xp.iter_mut().find(|x| x.name == entry.name) {
                Some(found) => found.amount += entry.amount,
                None => xp.push(entry.clone()),
            }
        }
        for item in &row.loot {
            let same = |other: &ItemTotal| match (item.item_id, other.item_id) {
                (Some(a), Some(b)) => a == b,
                (None, None) => item.item_name == other.item_name,
                _ => false,
            };
            match loot.iter_mut().find(|other| same(other)) {
                Some(found) => found.quantity += item.quantity,
                None => loot.push(item.clone()),
            }
        }
    }
    xp.sort_by_key(|x| std::cmp::Reverse(x.amount));
    merged.xp_by_character = xp;
    merged.loot = loot;
    merged
}

/// Mode Type (`typeRows`) : les sept types de donjon dans l'ordre du site, même vides (grisés) ;
/// l'arcade seulement si la période en compte ; puis « Autres », toutes les familles fusionnées,
/// s'il y en a. Un donjon de type inconnu est ignoré, comme sur le site.
pub fn type_rows<'a>(stats: &'a PeriodStats, dungeons: &DungeonIndex) -> Vec<RecapGroup<'a>> {
    let known: Vec<(WakfuDungeonType, i64, &GroupTotals)> = stats
        .dungeons
        .iter()
        .filter_map(|(id, totals)| {
            dungeons
                .find_by_id(*id)
                .map(|entry| (entry.dungeon_type, *id, totals))
        })
        .collect();
    let mut rows = Vec::new();
    for (dungeon_type, title) in super::combats::DUNGEON_SECTIONS {
        let bucket: Vec<(i64, &GroupTotals)> = known
            .iter()
            .filter(|(t, _, _)| *t == dungeon_type)
            .map(|(_, id, totals)| (*id, *totals))
            .collect();
        if dungeon_type == WakfuDungeonType::Arcade && bucket.is_empty() {
            continue;
        }
        let mut tiles: Vec<(String, i64)> = bucket
            .iter()
            .map(|(id, totals)| (dungeon_label(dungeons, *id), row_count(totals)))
            .collect();
        tiles.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        rows.push(RecapGroup {
            key: format!("type:{title}"),
            label: title.to_string(),
            section: None,
            dungeon_id: None,
            disabled: bucket.is_empty(),
            totals: Cow::Owned(merge_totals(bucket.iter().map(|(_, totals)| *totals))),
            tiles,
        });
    }
    if !stats.families.is_empty() {
        rows.push(RecapGroup {
            key: "type:autres".to_string(),
            label: "Autres".to_string(),
            section: None,
            dungeon_id: None,
            disabled: false,
            totals: Cow::Owned(merge_totals(stats.families.iter().map(|(_, t)| t))),
            tiles: Vec::new(),
        });
    }
    rows
}

/// État de la vue.
#[derive(Default)]
pub struct RecapState {
    pub granularity: Granularity,
    /// Affichage du détail.
    pub mode: DetailMode,
    /// Lignes dépliées (clé : [`RecapGroup::key`]).
    pub expanded: HashSet<String>,
    /// Premier jour de la période affichée ; `None` : celle d'aujourd'hui.
    pub start: Option<NaiveDate>,
    /// Les agrégats de la période affichée, et ce qui les a produits.
    cache: Option<(Granularity, NaiveDate, bool, PeriodStats)>,
}

impl RecapState {
    /// La base a changé : les agrégats seront recalculés.
    pub fn invalidate(&mut self) {
        self.cache = None;
    }
}

const NAV_WIDTH: f32 = 36.0;
const GRANULARITY_WIDTH: f32 = 270.0;
const MODE_WIDTH: f32 = 390.0;
const GROUP_ROW: f32 = 36.0;
const BAND_HEIGHT: f32 = 64.0;
const ITEM_ROW: CompactRow = CompactRow {
    height: 26.0,
    pad_x: 6.0,
    gap: 6.0,
    font_size: 13.0,
};

pub fn show(
    ui: &mut Ui,
    zones: &design::PanelZones,
    state: &mut RecapState,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let current = period_start(state.granularity, ctx.today);
    let mut start = state
        .start
        .map_or(current, |start| period_start(state.granularity, start));

    // Granularité, puis précédent / libellé / suivant.
    ui.horizontal(|ui| {
        let before = state.granularity;
        ui.add(
            design::switch(&mut state.granularity)
                .slot(Granularity::Day, "Jour")
                .slot(Granularity::Month, "Mois")
                .slot(Granularity::Year, "Année")
                .width(GRANULARITY_WIDTH)
                .log_name("historique.recap.granularite"),
        );
        if state.granularity != before {
            start = period_start(state.granularity, start);
        }
        ui.add_space(16.0);
        if ui
            .add(
                design::icon_button(DsIcon::TriangleRight)
                    .size(NAV_WIDTH)
                    .tooltip("Période précédente")
                    .log_name("historique.recap.precedente"),
            )
            .clicked()
        {
            start = shift(state.granularity, start, -1);
        }
        let label_width =
            zones.inner.width() - GRANULARITY_WIDTH - MODE_WIDTH - 2.0 * NAV_WIDTH - 84.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(label_width, NAV_WIDTH), Sense::hover());
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            label(state.granularity, start, ctx.today),
            text::label_strong_font(ui.ctx(), items::FONT_SIZE + 2.0),
            tokens::TEXT_GOLD,
        );
        // Pas de période future.
        let next_enabled = start < current;
        if ui
            .add(
                design::icon_button(DsIcon::TriangleRight)
                    .mirrored(true)
                    .size(NAV_WIDTH)
                    .enabled(next_enabled)
                    .tooltip("Période suivante")
                    .log_name("historique.recap.suivante"),
            )
            .clicked()
            && next_enabled
        {
            start = shift(state.granularity, start, 1);
        }
        ui.add_space(16.0);
        ui.add(
            design::switch(&mut state.mode)
                .slot(DetailMode::Cumulative, "Cumulé")
                .slot(DetailMode::ByGroup, "Donjon & Famille")
                .slot(DetailMode::ByType, "Type")
                .width(MODE_WIDTH)
                .log_name("historique.recap.detail"),
        );
    });
    ui.add_space(8.0);
    state.start = (start != current).then_some(start);

    // Agrégats : recalculés quand la période change, ou quand le référentiel arrive (il range les
    // combats par famille et écarte ceux d'Incarnam).
    let catalog_ready = !ctx.catalog.is_empty();
    let stale = state.cache.as_ref().is_none_or(|(g, s, ready, _)| {
        (*g, *s, *ready) != (state.granularity, start, catalog_ready)
    });
    if stale {
        let (since, until) = bounds(state.granularity, start);
        let stats = match ctx
            .store
            .map(|store| store.period_stats(since, until, ctx.catalog))
        {
            Some(Ok(stats)) => stats,
            Some(Err(err)) => {
                tracing::warn!(%err, "historique : récap illisible");
                PeriodStats::default()
            }
            None => PeriodStats::default(),
        };
        state.cache = Some((state.granularity, start, catalog_ready, stats));
    }
    let Some((_, _, _, stats)) = &state.cache else {
        return;
    };
    let (mode, expanded) = (state.mode, &mut state.expanded);

    zones.scroll_area(ui, "historique-recap", |ui, width| {
        band(ui, width, stats);
        ui.add_space(12.0);
        if mode != DetailMode::Cumulative {
            let groups = if mode == DetailMode::ByGroup {
                group_rows(stats, ctx.dungeons)
            } else {
                type_rows(stats, ctx.dungeons)
            };
            if groups.is_empty() {
                ui.add(
                    design::label("Aucun combat sur cette période.").color(tokens::TEXT_DISABLED),
                );
            }
            for group in &groups {
                group_block(ui, width, group, expanded, ctx, textures);
            }
            return;
        }
        kamas_detail(ui, width, stats);
        ui.add_space(12.0);
        section_title(ui, width, "Expérience", "");
        if stats.xp_by_character.iter().all(|xp| xp.amount <= 0) {
            ui.add(
                design::label("Aucune XP enregistrée pour le moment.").color(tokens::TEXT_DISABLED),
            );
        }
        // Un personnage présent sans XP gagnée n'apprend rien.
        for xp in stats.xp_by_character.iter().filter(|xp| xp.amount > 0) {
            let (row, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), Sense::hover());
            let painter = ui.painter_at(row);
            painter.text(
                egui::pos2(row.left() + 12.0, row.center().y),
                Align2::LEFT_CENTER,
                &xp.name,
                text::label_font(ui.ctx(), 13.0),
                Color32::WHITE,
            );
            painter.text(
                egui::pos2(row.right() - 12.0, row.center().y),
                Align2::RIGHT_CENTER,
                format!("+{}", format::thousands(xp.amount)),
                text::label_strong_font(ui.ctx(), 13.0),
                tokens::OVERLAY_ACCENT,
            );
        }
        for (title, list) in [("Butin", &stats.loot), ("Pacte", &stats.pacts)] {
            if list.is_empty() {
                continue;
            }
            ui.add_space(12.0);
            section_title(ui, width, title, &format!("{} objet(s)", list.len()));
            item_list(ui, width, list, ctx, textures);
        }
    });
}

/// Une case de la bande : glyphe blanc, libellé, puis la valeur en couleur.
struct Cell<'a> {
    icon: DsIcon,
    label: &'a str,
    parts: Vec<(String, Color32)>,
}

fn band(ui: &mut Ui, width: f32, stats: &PeriodStats) {
    band_cells(ui, width, &period_cells(stats));
}

fn period_cells(stats: &PeriodStats) -> [Cell<'static>; 4] {
    let c = &stats.combats;
    let net = net_kamas(stats);
    let xp: i64 = stats.xp_by_character.iter().map(|xp| xp.amount).sum();
    let cells = [
        Cell {
            icon: DsIcon::Kamas,
            label: "Kamas",
            parts: vec![(
                format::thousands(net),
                if net < 0 {
                    items::LOSS
                } else {
                    tokens::TEXT_GOLD
                },
            )],
        },
        Cell {
            icon: DsIcon::Xp,
            label: "Expérience",
            parts: vec![(format::thousands(xp), tokens::OVERLAY_ACCENT)],
        },
        Cell {
            icon: DsIcon::Sword,
            label: "Combats",
            parts: vec![
                (format::thousands(c.won), items::GAIN),
                (" – ".to_string(), tokens::OVERLAY_TEXT_MUTED),
                (format::thousands(c.lost), items::LOSS),
            ],
        },
        Cell {
            icon: DsIcon::Trophy,
            label: "Challenges",
            parts: vec![
                (format::thousands(c.challenges_passed), items::GAIN),
                (" – ".to_string(), tokens::OVERLAY_TEXT_MUTED),
                (format::thousands(c.challenges_failed), items::LOSS),
            ],
        },
    ];
    cells
}

/// La bande d'indicateurs : une case par entrée, séparées d'un filet.
fn band_cells(ui: &mut Ui, width: f32, cells: &[Cell<'_>]) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, BAND_HEIGHT), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, Color32::from_rgba_premultiplied(0, 0, 0, 80));
    let cell_width = width / cells.len() as f32;
    for (index, cell) in cells.iter().enumerate() {
        let area = Rect::from_min_size(
            egui::pos2(rect.left() + index as f32 * cell_width, rect.top()),
            egui::vec2(cell_width, rect.height()),
        );
        if index > 0 {
            painter.vline(
                area.left(),
                area.y_range().shrink(12.0),
                egui::Stroke::new(1.0, Color32::from_white_alpha(20)),
            );
        }
        painter.text(
            egui::pos2(area.center().x, area.top() + 18.0),
            Align2::CENTER_CENTER,
            cell.label.to_uppercase(),
            text::label_strong_font(ui.ctx(), 11.0),
            tokens::OVERLAY_TEXT_MUTED,
        );
        let font = text::label_strong_font(ui.ctx(), 17.0);
        let galleys: Vec<_> = cell
            .parts
            .iter()
            .map(|(value, color)| painter.layout_no_wrap(value.clone(), font.clone(), *color))
            .collect();
        let value_width: f32 = galleys.iter().map(|g| g.size().x).sum();
        let icon_side = 16.0;
        let total = icon_side + 8.0 + value_width;
        let y = area.top() + 42.0;
        let mut x = area.center().x - total / 2.0;
        super::paint_icon(
            ui,
            &painter,
            egui::pos2(x + icon_side / 2.0, y),
            cell.icon,
            icon_side,
            Color32::WHITE,
        );
        x += icon_side + 8.0;
        for galley in galleys {
            let size = galley.size();
            painter.galley(egui::pos2(x, y - size.y / 2.0), galley, Color32::WHITE);
            x += size.x;
        }
    }
}

/// Le détail des kamas : une ligne par source, signée.
fn kamas_detail(ui: &mut Ui, width: f32, stats: &PeriodStats) {
    let k = &stats.kamas;
    section_title(ui, width, "Kamas", "");
    for (label, amount, sign) in [
        ("Combats", k.from_combat, 1),
        ("Hôtel de vente", k.from_hdv_sales, 1),
        ("Achats", k.spent_on_purchases, -1),
        ("Échanges reçus", k.trades_acquired, 1),
        ("Échanges donnés", k.trades_given, -1),
    ] {
        let (row, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), Sense::hover());
        let painter = ui.painter_at(row);
        painter.text(
            egui::pos2(row.left() + 12.0, row.center().y),
            Align2::LEFT_CENTER,
            label,
            text::label_font(ui.ctx(), 13.0),
            Color32::WHITE,
        );
        let (text, color) = match (amount, sign) {
            (0, _) => ("0".to_string(), tokens::OVERLAY_TEXT_MUTED),
            (amount, 1) => (format!("+{}", format::thousands(amount)), items::GAIN),
            (amount, _) => (format!("-{}", format::thousands(amount)), items::LOSS),
        };
        super::paint_amount(
            ui,
            &painter,
            egui::pos2(row.right() - 12.0, row.center().y),
            &text,
            text::label_strong_font(ui.ctx(), 13.0),
            color,
        );
    }
}

fn section_title(ui: &mut Ui, width: f32, title: &str, tail: &str) {
    let (row, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), Sense::hover());
    let painter = ui.painter_at(row);
    let title = painter.text(
        egui::pos2(row.left() + 4.0, row.center().y),
        Align2::LEFT_CENTER,
        title,
        text::label_strong_font(ui.ctx(), 14.0),
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

/// Une liste d'objets cumulés, par quantité décroissante.
fn item_list(
    ui: &mut Ui,
    width: f32,
    list: &[ItemTotal],
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let mut lines: Vec<ItemLine> = list
        .iter()
        .map(|item| ItemLine {
            item_id: item.item_id,
            item_name: item.item_name.clone(),
            quantity: item.quantity,
        })
        .collect();
    lines.sort_by_key(|line| std::cmp::Reverse(line.quantity));
    for line in &lines {
        let (row, _) =
            ui.allocate_exact_size(egui::vec2(width, ITEM_ROW.height + 2.0), Sense::hover());
        if !ui.is_rect_visible(row) {
            continue;
        }
        let row = Rect::from_min_size(
            row.min + egui::vec2(0.0, 1.0),
            egui::vec2(width, ITEM_ROW.height),
        );
        items::paint_compact_item(ui, ui.painter(), row, line, ITEM_ROW, ctx, textures);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn totals(fights: i64, runs: i64, item: Option<i64>, quantity: i64) -> GroupTotals {
        GroupTotals {
            fights,
            dungeon_runs: runs,
            won: fights,
            xp_gained: 10 * fights,
            xp_by_character: vec![XpTotal {
                name: "Iop".to_string(),
                amount: 10 * fights,
            }],
            loot: item
                .map(|id| ItemTotal {
                    item_id: Some(id),
                    item_name: None,
                    quantity,
                })
                .into_iter()
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn la_fusion_additionne_xp_et_butin() {
        let a = totals(3, 1, Some(7), 2);
        let b = totals(5, 2, Some(7), 4);
        let merged = merge_totals([&a, &b]);
        assert_eq!((merged.fights, merged.dungeon_runs), (8, 3));
        assert_eq!(merged.xp_by_character[0].amount, 80);
        assert_eq!(merged.loot.len(), 1);
        assert_eq!(merged.loot[0].quantity, 6);
        assert_eq!(row_count(&merged), 3, "les donjons terminés priment");
        assert_eq!(row_count(&totals(4, 0, None, 0)), 4);
    }

    #[test]
    fn sans_donjon_connu_tout_va_sous_familles_et_autres() {
        let dungeons = DungeonIndex::default();
        let stats = PeriodStats {
            dungeons: vec![(99, totals(2, 1, None, 0))],
            families: vec![
                (Some(1), totals(6, 0, None, 0)),
                (None, totals(1, 0, None, 0)),
            ],
            ..Default::default()
        };
        let groups = group_rows(&stats, &dungeons);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].section, Some("Familles"));
        assert_eq!(
            row_count(&groups[0].totals),
            6,
            "tri par compte décroissant"
        );
        let types = type_rows(&stats, &dungeons);
        // Sept types grisés (l'arcade vide est omise), puis « Autres ».
        assert_eq!(types.len(), 8);
        assert!(types[..7].iter().all(|t| t.disabled));
        assert_eq!(types[7].label, "Autres");
        assert_eq!(types[7].totals.fights, 7);
    }

    #[test]
    fn periodes_du_calendrier_local() {
        let today = day(2026, 11, 27);
        assert_eq!(period_start(Granularity::Month, today), day(2026, 11, 1));
        assert_eq!(period_start(Granularity::Year, today), day(2026, 1, 1));
        assert_eq!(
            shift(Granularity::Month, day(2026, 1, 1), -1),
            day(2025, 12, 1)
        );
        assert_eq!(
            shift(Granularity::Month, day(2026, 12, 1), 1),
            day(2027, 1, 1)
        );
        assert_eq!(
            shift(Granularity::Day, day(2026, 3, 1), -1),
            day(2026, 2, 28)
        );
        assert_eq!(label(Granularity::Day, today, today), "Aujourd'hui");
        assert_eq!(
            label(Granularity::Month, day(2026, 8, 1), today),
            "août 2026"
        );
        assert_eq!(label(Granularity::Year, day(2025, 1, 1), today), "2025");
        let (since, until) = bounds(Granularity::Day, today);
        // 23, 24 ou 25 heures selon l'heure d'été.
        assert!((23 * 3_600_000..=25 * 3_600_000).contains(&(until - since)));
    }
}

/// Une ligne Donjon / Famille / Type : caret, illustration, libellé, compte ; dépliée, ses totaux,
/// ses donjons (mode Type), l'XP par personnage et le butin.
fn group_block(
    ui: &mut Ui,
    width: f32,
    group: &RecapGroup<'_>,
    expanded: &mut HashSet<String>,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    if let Some(section) = group.section {
        ui.add_space(6.0);
        let (row, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), Sense::hover());
        ui.painter_at(row).text(
            egui::pos2(row.left() + 4.0, row.center().y),
            Align2::LEFT_CENTER,
            section.to_uppercase(),
            text::label_strong_font(ui.ctx(), 12.0),
            tokens::OVERLAY_TEXT_MUTED,
        );
    }
    let open = !group.disabled && expanded.contains(&group.key);
    let sense = if group.disabled {
        Sense::hover()
    } else {
        Sense::click()
    };
    let (row, response) = ui.allocate_exact_size(egui::vec2(width, GROUP_ROW), sense);
    let response = if group.disabled {
        response
    } else {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    };
    if response.clicked() {
        tracing::info!(">>> clic historique.recap.groupe {}", group.label);
        if !expanded.remove(&group.key) {
            expanded.insert(group.key.clone());
        }
    }
    let painter = ui.painter_at(row);
    let fill = if response.hovered() && !group.disabled {
        Color32::from_rgba_premultiplied(30, 30, 30, 120)
    } else {
        Color32::from_rgba_premultiplied(0, 0, 0, 70)
    };
    painter.rect_filled(row.shrink2(egui::vec2(0.0, 1.0)), 4.0, fill);
    let tint = if group.disabled {
        tokens::TEXT_DISABLED
    } else {
        tokens::TEXT_GOLD
    };
    super::paint_caret(
        ui,
        &painter,
        egui::pos2(row.left() + 14.0, row.center().y),
        open,
        tint,
    );
    let icon_rect = Rect::from_center_size(
        egui::pos2(row.left() + 40.0, row.center().y),
        egui::vec2(24.0, 24.0),
    );
    let boss = group
        .dungeon_id
        .and_then(|id| ctx.dungeons.find_by_id(id))
        .and_then(|dungeon| dungeon.boss_monster_ids.first().copied())
        .and_then(|boss| ctx.catalog.find_monster_icon("", Some(boss)));
    if let Some(icon) = boss {
        super::paint_remote_icon(ui, &painter, icon_rect, &icon, ctx.remote_icons, textures);
    }
    let count = painter.text(
        egui::pos2(row.right() - 14.0, row.center().y),
        Align2::RIGHT_CENTER,
        format!("×{}", format::thousands(row_count(&group.totals))),
        text::label_strong_font(ui.ctx(), 13.0),
        if group.disabled {
            tokens::TEXT_DISABLED
        } else {
            tokens::OVERLAY_ACCENT
        },
    );
    let name_rect = Rect::from_min_max(
        egui::pos2(icon_rect.right() + 10.0, row.top()),
        egui::pos2(count.left() - 10.0, row.bottom()),
    );
    painter.with_clip_rect(name_rect).text(
        name_rect.left_center(),
        Align2::LEFT_CENTER,
        &group.label,
        text::label_strong_font(ui.ctx(), 14.0),
        if group.disabled {
            tokens::TEXT_DISABLED
        } else {
            Color32::WHITE
        },
    );
    if !open {
        return;
    }
    let totals = &group.totals;
    ui.add_space(6.0);
    band_cells(
        ui,
        width,
        &[
            Cell {
                icon: DsIcon::Xp,
                label: "Expérience",
                parts: vec![(
                    format!("+{}", format::thousands(totals.xp_gained)),
                    tokens::OVERLAY_ACCENT,
                )],
            },
            Cell {
                icon: DsIcon::Kamas,
                label: "Kamas",
                parts: vec![(
                    format!("+{}", format::thousands(totals.kamas_gained)),
                    tokens::TEXT_GOLD,
                )],
            },
            Cell {
                icon: DsIcon::Trophy,
                label: "Victoires",
                parts: vec![(format::thousands(totals.won), items::GAIN)],
            },
            Cell {
                icon: DsIcon::Sword,
                label: "Défaites",
                parts: vec![(format::thousands(totals.lost), items::LOSS)],
            },
        ],
    );
    if !group.tiles.is_empty() {
        ui.add_space(8.0);
        section_title(ui, width, "Donjons", "");
        for (name, count) in &group.tiles {
            let (row, _) = ui.allocate_exact_size(egui::vec2(width, 22.0), Sense::hover());
            let painter = ui.painter_at(row);
            painter.text(
                egui::pos2(row.left() + 12.0, row.center().y),
                Align2::LEFT_CENTER,
                name,
                text::label_font(ui.ctx(), 13.0),
                Color32::WHITE,
            );
            painter.text(
                egui::pos2(row.right() - 12.0, row.center().y),
                Align2::RIGHT_CENTER,
                format!("×{}", format::thousands(*count)),
                text::label_strong_font(ui.ctx(), 13.0),
                tokens::OVERLAY_ACCENT,
            );
        }
    }
    let xp: Vec<&XpTotal> = totals
        .xp_by_character
        .iter()
        .filter(|xp| xp.amount > 0)
        .collect();
    if !xp.is_empty() {
        ui.add_space(8.0);
        section_title(ui, width, "Expérience", "");
        for entry in xp {
            let (row, _) = ui.allocate_exact_size(egui::vec2(width, 22.0), Sense::hover());
            let painter = ui.painter_at(row);
            painter.text(
                egui::pos2(row.left() + 12.0, row.center().y),
                Align2::LEFT_CENTER,
                &entry.name,
                text::label_font(ui.ctx(), 13.0),
                Color32::WHITE,
            );
            painter.text(
                egui::pos2(row.right() - 12.0, row.center().y),
                Align2::RIGHT_CENTER,
                format!("+{}", format::thousands(entry.amount)),
                text::label_strong_font(ui.ctx(), 13.0),
                tokens::OVERLAY_ACCENT,
            );
        }
    }
    if !totals.loot.is_empty() {
        ui.add_space(8.0);
        section_title(
            ui,
            width,
            "Butin",
            &format!("{} objet(s)", totals.loot.len()),
        );
        item_list(ui, width, &totals.loot, ctx, textures);
    }
    ui.add_space(10.0);
}
