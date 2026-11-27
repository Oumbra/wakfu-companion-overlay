//! Vue **Récap** — port de `features/session-recap/` (site), mode « cumulé » : une période (jour,
//! mois ou année, bornée dans le calendrier local), parcourue par précédent / suivant, et ses
//! agrégats (`HistoryStore::period_stats`) — bande d'indicateurs (XP, kamas nets avec leur détail
//! en infobulle, combats gagnés – perdus, challenges réussis – échoués), XP par personnage, butin
//! et pactes.
//!
//! Trois affichages du détail (`detailMode` du site) : **Cumulé** (les sections ci-dessus),
//! **Donjon & Famille** (une ligne par donjon, rangée sous son type, puis une par famille de
//! monstres hors donjon) et **Type** (les types de donjon fusionnés, puis « Autres »). Chaque ligne
//! se déplie sur ses totaux, ses donjons (mode Type), l'XP par personnage et son butin.
//!
//! **Écarts** : pas de granularité « session » (le récap de la session en cours vit sur le jeu,
//! dans l'overlay), pas d'illustration dédiée aux brèches. **Ajout** : chaque section (expérience,
//! butin, pacte, donjons) se replie — la fenêtre n'a pas la hauteur bornée du site, une longue
//! liste d'expérience y repoussait tout le reste.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use chrono::{Datelike, Local, NaiveDate, TimeZone};
use egui::{Align2, Color32, Rect, Sense, Ui};
use overlay_engine::{DungeonIndex, IconRef, WakfuDungeonType};
use overlay_history::read::ItemLine;
use overlay_history::stats::{GroupTotals, ItemTotal, PeriodStats, XpTotal};

use super::combats::{loot_controls, LootFilter};
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

/// Le premier jour où l'historique existe (`HISTORY_TRACKING_START_MS` du site, 1ᵉʳ août 2026) :
/// la navigation ne remonte pas plus loin (`minOffsetForGranularity`).
pub fn tracking_start() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 8, 1).unwrap_or_default()
}

/// L'affichage du détail (`DetailMode` du site).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailMode {
    #[default]
    Cumulative,
    ByGroup,
    ByType,
}

/// La nature d'une ligne — elle choisit la vignette de repli (`#groupItemTpl` du site).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    Dungeon,
    Family,
    /// Un type de donjon fusionné (mode Type).
    Type,
    /// « Autres » : toutes les familles fusionnées (mode Type).
    Other,
}

/// Un donjon derrière une ligne Type (`DungeonTile` du site).
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    pub label: String,
    pub count: i64,
    pub picture: Option<IconRef>,
}

/// Une ligne dépliable des modes Donjon & Famille et Type (`RecapGroupRow` du site).
#[derive(Debug, Clone, PartialEq)]
pub struct RecapGroup<'a> {
    pub key: String,
    pub label: String,
    pub kind: GroupKind,
    /// Mode Type : le type de la ligne (vignette des brèches).
    pub dungeon_type: Option<WakfuDungeonType>,
    /// Titre de section à peindre AVANT cette ligne (mode Donjon & Famille).
    pub section: Option<&'static str>,
    /// Illustration du donjon ou vignette de la famille (`pictureUrl`).
    pub picture: Option<IconRef>,
    /// Ligne Type adossée à une pierre de donjon (`stoneItemId`) : sa vignette de repli, et la
    /// première ligne de son butin.
    pub stone: Option<i64>,
    pub totals: Cow<'a, GroupTotals>,
    /// Type de donjon sans aucun combat sur la période : ligne grisée, non dépliable.
    pub disabled: bool,
    /// Mode Type : les donjons fusionnés, du plus fréquent au moins fréquent.
    pub tiles: Vec<Tile>,
}

/// `rowCount` du site : les donjons terminés, sinon les combats.
pub fn row_count(totals: &GroupTotals) -> i64 {
    if totals.dungeon_runs > 0 {
        totals.dungeon_runs
    } else {
        totals.fights
    }
}

/// La pierre de donjon d'un type (`STONE_ITEM_ID_BY_TYPE` du site) — aucune pour les brèches et
/// l'arcade.
pub fn stone_for_type(dungeon_type: WakfuDungeonType) -> Option<i64> {
    match dungeon_type {
        WakfuDungeonType::TwoRooms => Some(29849),
        WakfuDungeonType::ThreeRooms => Some(29848),
        WakfuDungeonType::FourRooms => Some(29847),
        WakfuDungeonType::ThreePlayers => Some(29850),
        WakfuDungeonType::UltimateBoss => Some(29851),
        _ => None,
    }
}

fn dungeon_label(dungeons: &DungeonIndex, id: i64) -> String {
    dungeons
        .find_by_id(id)
        .map_or_else(|| "Donjon inconnu".to_string(), |d| d.fr.clone())
}

fn dungeon_picture(dungeons: &DungeonIndex, id: i64) -> Option<IconRef> {
    dungeons.find_by_id(id).and_then(|d| d.picture.clone())
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
                    kind: GroupKind::Dungeon,
                    dungeon_type: None,
                    section: None,
                    picture: dungeon_picture(dungeons, *id),
                    stone: None,
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
                    kind: GroupKind::Family,
                    dungeon_type: None,
                    section: None,
                    picture: family.and_then(super::family_picture),
                    stone: None,
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
        let mut tiles: Vec<Tile> = bucket
            .iter()
            .map(|(id, totals)| Tile {
                label: dungeon_label(dungeons, *id),
                count: row_count(totals),
                picture: dungeon_picture(dungeons, *id),
            })
            .collect();
        tiles.sort_by_key(|tile| std::cmp::Reverse(tile.count));
        rows.push(RecapGroup {
            key: format!("type:{title}"),
            label: title.to_string(),
            kind: GroupKind::Type,
            dungeon_type: Some(dungeon_type),
            section: None,
            picture: None,
            stone: stone_for_type(dungeon_type),
            disabled: bucket.is_empty(),
            totals: Cow::Owned(merge_totals(bucket.iter().map(|(_, totals)| *totals))),
            tiles,
        });
    }
    if !stats.families.is_empty() {
        rows.push(RecapGroup {
            key: "type:autres".to_string(),
            label: "Autres".to_string(),
            kind: GroupKind::Other,
            dungeon_type: None,
            section: None,
            picture: None,
            stone: None,
            disabled: false,
            totals: Cow::Owned(merge_totals(stats.families.iter().map(|(_, t)| t))),
            tiles: Vec::new(),
        });
    }
    rows
}

/// Le butin d'une liste, filtré et trié ; une pierre de donjon (`stone`) reste épinglée en tête,
/// même absente (quantité 0) et même hors recherche (`sortedRowLoot` du site).
pub fn shown_loot(
    list: &[ItemTotal],
    stone: Option<i64>,
    filter: &LootFilter,
    catalog: &overlay_engine::CatalogIndex,
) -> Vec<ItemLine> {
    let lines: Vec<ItemLine> = list
        .iter()
        .map(|item| ItemLine {
            item_id: item.item_id,
            item_name: item.item_name.clone(),
            quantity: item.quantity,
        })
        .collect();
    let Some(stone) = stone else {
        return filter.apply(&lines, catalog).into_iter().cloned().collect();
    };
    let pinned = lines
        .iter()
        .find(|line| line.item_id == Some(stone))
        .cloned()
        .unwrap_or(ItemLine {
            item_id: Some(stone),
            item_name: None,
            quantity: 0,
        });
    let rest: Vec<ItemLine> = lines
        .into_iter()
        .filter(|line| line.item_id != Some(stone))
        .collect();
    std::iter::once(pinned)
        .chain(filter.apply(&rest, catalog).into_iter().cloned())
        .collect()
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
    /// Recherche et tri du butin, partagés par toutes les listes de butin de la vue.
    pub loot: LootFilter,
    /// Recherche et tri des objets de pacte (`pactSearch` / `pactSort` du site).
    pub pacts: LootFilter,
    /// Les agrégats de la période affichée, et ce qui les a produits.
    cache: Option<(Granularity, NaiveDate, bool, PeriodStats)>,
    /// La classe de chaque personnage (portraits des lignes d'expérience).
    classes: Option<HashMap<String, String>>,
    /// Le calendrier ouvert, sur cette page ; `None` : fermé.
    picker: Option<NaiveDate>,
    /// Sections repliées (`portée/section` : `cumul/xp`, `<ligne>/butin`…).
    collapsed: HashSet<String>,
}

impl RecapState {
    /// La base a changé : les agrégats seront recalculés.
    pub fn invalidate(&mut self) {
        self.cache = None;
        self.classes = None;
    }
}

const NAV_WIDTH: f32 = super::CONTROL_HEIGHT;
/// La période entre précédent et suivant : place fixe, le libellé le plus long (« septembre 2026 »)
/// compris — le sélecteur ne s'étire pas sur la ligne.
const PERIOD_LABEL_WIDTH: f32 = 170.0;
/// Ligne d'en-tête d'un groupe (`.dungeon-row` : vignette de 24 px), dans sa boîte.
const GROUP_ROW: f32 = 26.0;
/// La boîte d'un groupe (`.group-item` du site) : fond, bordure, marges intérieures, et l'écart
/// entre deux boîtes (`.group-list`, `gap: 6px`).
const GROUP_FILL: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
const GROUP_BORDER: Color32 = Color32::from_rgb(0x33, 0x33, 0x33);
const GROUP_PAD_X: i8 = 12;
const GROUP_PAD_Y: i8 = 8;
const GROUP_GAP: f32 = 6.0;
/// Écart entre l'en-tête d'une section dépliée et son contenu.
const SECTION_GAP: f32 = 6.0;
const BAND_HEIGHT: f32 = 64.0;
/// Côté des médaillons d'en-tête de section.
const SECTION_ICON: f32 = 20.0;
const ITEM_ROW: CompactRow = CompactRow {
    height: 26.0,
    pad_x: 6.0,
    gap: 6.0,
    font_size: 13.0,
};
/// Une tuile de donjon (`.dungeon-tile` du site) : l'illustration (`.dungeon-tile-thumb`, 62 px de
/// haut, image à 88 % posée en bas) et 3 px de marge haute et basse ; écart de 10 px, et 26 px de
/// recouvrement quand elles se chevauchent. La largeur est celle que le navigateur déduit de
/// l'illustration (391 × 194) mise à la hauteur de la vignette : 62 × 0,88 × 391 / 194 ≈ 110.
const TILE_WIDTH: f32 = 112.0;
const TILE_THUMB: f32 = 62.0;
const TILE_HEIGHT: f32 = TILE_THUMB + 6.0;
const TILE_GAP: f32 = 10.0;
const TILE_STACK_STEP: f32 = TILE_WIDTH - 26.0;
const TILE_FILL: Color32 = Color32::from_rgb(0x33, 0x33, 0x33);
/// Pastille du compte (`.dungeon-tile-count`) : fond, et l'or des kamas (`--kama-color`).
const TILE_BADGE_FILL: Color32 = Color32::from_rgb(0x1B, 0x1B, 0x1B);
const KAMA_GOLD: Color32 = Color32::from_rgb(255, 215, 0);
/// Texte courant du site (`--text-color`).
const SITE_TEXT: Color32 = Color32::from_rgb(0xE0, 0xE0, 0xE0);
/// Au-delà, les tuiles se chevauchent (`DUNGEON_TILE_STACK_THRESHOLD` du site).
const TILE_STACK_THRESHOLD: usize = 5;

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
    let before = (state.granularity, start, state.mode);

    // Le menu Jour / Mois / Année surplombe tout le reste : c'est lui qui décide de ce que montre
    // la suite (retour utilisateur du 2026-11-27). Un menu, donc des onglets, comme la barre du
    // haut de la fenêtre.
    let previous = state.granularity;
    design::tabs(&mut state.granularity)
        .entry(Granularity::Day, "Jour")
        .entry(Granularity::Month, "Mois")
        .entry(Granularity::Year, "Année")
        .log_name("historique.recap.granularite")
        .show(ui);
    if state.granularity != previous {
        // Une nouvelle granularité repart de la période en cours (`setGranularity`).
        start = period_start(state.granularity, ctx.today);
        state.picker = None;
    }
    ui.add_space(8.0);

    // À gauche, précédent / période / suivant puis le calendrier (`recap-period-nav-date`) ; à
    // droite, l'affichage du détail en switch (`recap-group-switch`).
    let floor = period_start(state.granularity, tracking_start());
    let mut calendar_rect = None;
    ui.horizontal(|ui| {
        let previous_enabled = start > floor;
        if ui
            .add(
                design::icon_button(DsIcon::TriangleRight)
                    .size(NAV_WIDTH)
                    .enabled(previous_enabled)
                    .tooltip("Période précédente")
                    .log_name("historique.recap.precedente"),
            )
            .clicked()
            && previous_enabled
        {
            start = shift(state.granularity, start, -1);
        }
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(PERIOD_LABEL_WIDTH, NAV_WIDTH), Sense::hover());
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            label(state.granularity, start, ctx.today),
            text::label_strong_font(ui.ctx(), items::FONT_SIZE + 1.0),
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
        let calendar = ui.add(
            design::icon_button(DsIcon::CalendarLine)
                .size(NAV_WIDTH)
                .glyph_side(super::SWITCH_ICON)
                .tooltip("Choisir une date")
                .log_name("historique.recap.calendrier"),
        );
        if calendar.clicked() {
            state.picker = match state.picker {
                Some(_) => None,
                None => Some(start),
            };
        }
        calendar_rect = Some(calendar.rect);
        let mode_switch = super::switch(&mut state.mode)
            .slot(DetailMode::Cumulative, "Cumulé")
            .icon(DsIcon::Lines)
            .slot(DetailMode::ByGroup, "Donjon & Famille")
            .icon(DsIcon::Dungeon)
            .slot(DetailMode::ByType, "Type")
            .icon(DsIcon::CrossedSwords)
            .log_name("historique.recap.detail");
        let mode_width = mode_switch.desired_size().x;
        ui.add_space((ui.available_width() - mode_width).max(0.0));
        ui.add(mode_switch);
    });
    // Le calendrier, ouvert sous son bouton.
    if let (Some(page), Some(anchor)) = (state.picker.as_mut(), calendar_rect) {
        let mut close = false;
        let picked = super::calendar::show(
            ui,
            anchor,
            state.granularity,
            page,
            &super::calendar::Bounds {
                selected: start,
                floor,
                current,
            },
            &mut close,
        );
        if let Some(picked) = picked {
            tracing::info!(">>> clic historique.recap.calendrier {picked}");
            start = picked;
        }
        // Le clic qui vient d'ouvrir le calendrier tombe « à côté » : il ne le referme pas.
        let opening = ui.input(|i| i.pointer.any_click())
            && calendar_rect.is_some_and(|rect| {
                ui.input(|i| i.pointer.interact_pos())
                    .is_some_and(|pos| rect.contains(pos))
            });
        if close && !opening {
            state.picker = None;
        }
    }
    ui.add_space(8.0);
    // Changer de granularité, de période ou d'affichage referme l'accordéon : les lignes d'ici
    // n'ont pas de raison d'exister là-bas (`setGranularity`, `onOffsetChange`, `setDetailMode`).
    if before != (state.granularity, start, state.mode) {
        state.expanded.clear();
    }
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
    if state.classes.is_none() {
        state.classes = Some(
            ctx.store
                .and_then(|store| store.ally_classes().ok())
                .unwrap_or_default(),
        );
    }
    let RecapState {
        mode,
        expanded,
        loot,
        pacts,
        cache,
        classes,
        collapsed,
        ..
    } = state;
    let Some((_, _, _, stats)) = cache else {
        return;
    };
    let no_classes = HashMap::new();
    let classes = classes.as_ref().unwrap_or(&no_classes);
    let mode = *mode;

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
                group_block(
                    ui, width, group, expanded, collapsed, loot, classes, ctx, textures,
                );
            }
            return;
        }
        // Le bandeau porte déjà Kamas et Combats : l'expérience, le butin, puis le pacte.
        xp_section(
            ui,
            width,
            &stats.xp_by_character,
            classes,
            ctx,
            Fold::new(collapsed, "cumul/xp"),
        );
        ui.add_space(12.0);
        loot_block(
            ui,
            width,
            LootBlock {
                icon: DsIcon::HeaderLoot,
                title: "Butin",
                list: &stats.loot,
                stone: None,
                log_name: "historique.recap.butin",
            },
            Fold::new(collapsed, "cumul/butin"),
            loot,
            ctx,
            textures,
        );
        // Une section Pacte vide n'a rien à montrer (demande de l'utilisateur sur le site).
        if !stats.pacts.is_empty() {
            ui.add_space(12.0);
            loot_block(
                ui,
                width,
                LootBlock {
                    icon: DsIcon::Pact,
                    title: "Pacte",
                    list: &stats.pacts,
                    stone: None,
                    log_name: "historique.recap.pacte",
                },
                Fold::new(collapsed, "cumul/pacte"),
                pacts,
                ctx,
                textures,
            );
        }
    });
}

/// Ce qu'une case de la bande montre après son libellé.
enum Value {
    /// Des morceaux de texte en couleur (`1 066`, ` – `, `46`).
    Parts(Vec<(String, Color32)>),
    /// Un montant en kamas, suivi du glyphe Kamas.
    Kamas(String, Color32),
}

/// Une case de la bande : médaillon, libellé, puis la valeur ; éventuellement une infobulle en
/// tableau (`tooltipRows` du site).
struct Cell<'a> {
    icon: DsIcon,
    /// Teinte du pictogramme (blanc pour un médaillon en couleurs).
    tint: Color32,
    label: &'a str,
    value: Value,
    tooltip: Vec<(&'static str, String, Color32)>,
}

fn band(ui: &mut Ui, width: f32, stats: &PeriodStats) {
    band_cells(ui, width, &period_cells(stats));
}

/// Une ligne signée de l'infobulle Kamas.
fn signed(amount: i64, sign: i64) -> (String, Color32) {
    match (amount, sign) {
        (0, _) => ("0".to_string(), tokens::OVERLAY_TEXT_MUTED),
        (amount, 1) => (format!("+{}", format::thousands(amount)), items::GAIN),
        (amount, _) => (format!("-{}", format::thousands(amount)), items::LOSS),
    }
}

/// La bande de la période (`recap-bandeau` du site) : Expérience, Kamas (détail en infobulle),
/// Combats, Challenges.
fn period_cells(stats: &PeriodStats) -> [Cell<'static>; 4] {
    let c = &stats.combats;
    let k = &stats.kamas;
    let xp: i64 = stats.xp_by_character.iter().map(|xp| xp.amount).sum();
    let tooltip = [
        ("Combats", k.from_combat, 1),
        ("Hôtel de vente", k.from_hdv_sales, 1),
        ("Achats", k.spent_on_purchases, -1),
        ("Échanges reçus", k.trades_acquired, 1),
        ("Échanges donnés", k.trades_given, -1),
    ]
    .map(|(label, amount, sign)| {
        let (text, color) = signed(amount, sign);
        (label, text, color)
    })
    .to_vec();
    [
        Cell {
            icon: DsIcon::HeaderXp,
            tint: Color32::WHITE,
            label: "Expérience",
            value: Value::Parts(vec![(
                format!("+{}", format::thousands(xp)),
                tokens::OVERLAY_ACCENT,
            )]),
            tooltip: Vec::new(),
        },
        Cell {
            icon: DsIcon::HeaderKamas,
            tint: Color32::WHITE,
            label: "Kamas",
            value: Value::Kamas(format::thousands(net_kamas(stats)), tokens::TEXT_GOLD),
            tooltip,
        },
        Cell {
            icon: DsIcon::HeaderCombat,
            tint: Color32::WHITE,
            label: "Combats",
            value: Value::Parts(vec![
                (format::thousands(c.won), items::GAIN),
                (" – ".to_string(), tokens::OVERLAY_TEXT_MUTED),
                (format::thousands(c.lost), items::LOSS),
            ]),
            tooltip: Vec::new(),
        },
        Cell {
            icon: DsIcon::HeaderChallenges,
            tint: Color32::WHITE,
            label: "Challenges",
            value: Value::Parts(vec![
                (format::thousands(c.challenges_passed), items::GAIN),
                (" – ".to_string(), tokens::OVERLAY_TEXT_MUTED),
                (format::thousands(c.challenges_failed), items::LOSS),
            ]),
            tooltip: Vec::new(),
        },
    ]
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
        // En-tête : pictogramme puis libellé, centrés ensemble.
        let head_font = text::label_strong_font(ui.ctx(), 11.0);
        let head = painter.layout_no_wrap(
            cell.label.to_uppercase(),
            head_font,
            tokens::OVERLAY_TEXT_MUTED,
        );
        let icon_side = 16.0;
        let head_width = icon_side + 6.0 + head.size().x;
        let head_left = area.center().x - head_width / 2.0;
        let head_y = area.top() + 18.0;
        super::paint_icon(
            ui,
            &painter,
            egui::pos2(head_left + icon_side / 2.0, head_y),
            cell.icon,
            icon_side,
            cell.tint,
        );
        painter.galley(
            egui::pos2(head_left + icon_side + 6.0, head_y - head.size().y / 2.0),
            head,
            tokens::OVERLAY_TEXT_MUTED,
        );
        // Valeur.
        let font = text::label_strong_font(ui.ctx(), 17.0);
        let y = area.top() + 42.0;
        match &cell.value {
            Value::Parts(parts) => {
                let galleys: Vec<_> = parts
                    .iter()
                    .map(|(value, color)| {
                        painter.layout_no_wrap(value.clone(), font.clone(), *color)
                    })
                    .collect();
                let total: f32 = galleys.iter().map(|g| g.size().x).sum();
                let mut x = area.center().x - total / 2.0;
                for galley in galleys {
                    let size = galley.size();
                    painter.galley(egui::pos2(x, y - size.y / 2.0), galley, Color32::WHITE);
                    x += size.x;
                }
            }
            Value::Kamas(amount, color) => {
                let galley = painter.layout_no_wrap(amount.clone(), font.clone(), *color);
                let total = galley.size().x + 4.0 + super::KAMAS_ICON_SIZE;
                let left = area.center().x - total / 2.0;
                super::paint_amount_left(ui, &painter, egui::pos2(left, y), amount, font, *color);
            }
        }
        if !cell.tooltip.is_empty() {
            let rows = &cell.tooltip;
            ui.interact(area, ui.id().with(("bande", index)), Sense::hover())
                .on_hover_ui(|ui| {
                    egui::Grid::new("recap-kamas")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            for (label, value, color) in rows {
                                ui.label(*label);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.colored_label(*color, value);
                                    },
                                );
                                ui.end_row();
                            }
                        });
                });
        }
    }
}

/// Une section repliable : l'ensemble des sections repliées de la vue, et la clé de celle-ci
/// (`portée/section`).
struct Fold<'a> {
    collapsed: &'a mut HashSet<String>,
    key: String,
}

impl<'a> Fold<'a> {
    fn new(collapsed: &'a mut HashSet<String>, key: impl Into<String>) -> Self {
        Self {
            collapsed,
            key: key.into(),
        }
    }
}

/// Un en-tête de section (`recap-section-header` du site) : médaillon, titre, puis `tail`, et le
/// chevron au bout — toute la ligne replie ou déplie la section. Rend `true` si elle est dépliée.
#[allow(clippy::too_many_arguments)]
fn section_header(
    ui: &mut Ui,
    width: f32,
    icon: DsIcon,
    tint: Color32,
    title: &str,
    tail: &str,
    fold: Fold<'_>,
) -> bool {
    let (row, response) = ui.allocate_exact_size(egui::vec2(width, 28.0), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.clicked() {
        let open = fold.collapsed.contains(&fold.key);
        tracing::info!(
            ">>> clic historique.recap.section {} ({})",
            fold.key,
            if open { "dépliée" } else { "repliée" }
        );
        if !fold.collapsed.remove(&fold.key) {
            fold.collapsed.insert(fold.key.clone());
        }
    }
    let open = !fold.collapsed.contains(&fold.key);
    let painter = ui.painter_at(row);
    if response.hovered() {
        painter.rect_filled(row, 4.0, Color32::from_white_alpha(8));
    }
    super::paint_icon(
        ui,
        &painter,
        egui::pos2(row.left() + 4.0 + SECTION_ICON / 2.0, row.center().y),
        icon,
        SECTION_ICON,
        tint,
    );
    let title = painter.text(
        egui::pos2(row.left() + 10.0 + SECTION_ICON, row.center().y),
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
    super::paint_caret(
        ui,
        &painter,
        egui::pos2(row.right() - 14.0, row.center().y),
        open,
        tokens::TEXT_GOLD,
    );
    if open {
        ui.add_space(SECTION_GAP);
    }
    open
}

/// Un message grisé sous un en-tête de section.
fn empty_line(ui: &mut Ui, width: f32, message: &str) {
    let (row, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), Sense::hover());
    ui.painter_at(row).text(
        egui::pos2(row.left() + 10.0 + SECTION_ICON, row.center().y),
        Align2::LEFT_CENTER,
        message,
        text::label_font(ui.ctx(), 13.0),
        tokens::TEXT_DISABLED,
    );
}

/// L'expérience par personnage (`xp-list` du site) : portrait de la classe, nom, gain, et une
/// barre proportionnelle au meilleur gain.
fn xp_section(
    ui: &mut Ui,
    width: f32,
    xp: &[XpTotal],
    classes: &HashMap<String, String>,
    ctx: &HistoryContext<'_>,
    fold: Fold<'_>,
) {
    // Un personnage présent sans XP gagnée n'apprend rien.
    let rows: Vec<&XpTotal> = xp.iter().filter(|xp| xp.amount > 0).collect();
    let count = if rows.is_empty() {
        String::new()
    } else {
        format!("{} personnage(s)", rows.len())
    };
    if !section_header(
        ui,
        width,
        DsIcon::HeaderXp,
        Color32::WHITE,
        "Expérience",
        &count,
        fold,
    ) {
        return;
    }
    if rows.is_empty() {
        empty_line(ui, width, "Aucune XP enregistrée pour le moment.");
        return;
    }
    let best = rows.iter().map(|xp| xp.amount).max().unwrap_or(1).max(1);
    for entry in rows {
        let (row, _) = ui.allocate_exact_size(egui::vec2(width, 32.0), Sense::hover());
        let painter = ui.painter_at(row);
        let line_y = row.top() + 12.0;
        let icon = Rect::from_center_size(
            egui::pos2(row.left() + 4.0 + SECTION_ICON / 2.0, line_y),
            egui::vec2(20.0, 20.0),
        );
        if let Some(texture) = classes
            .get(&entry.name)
            .zip(ctx.portraits)
            .and_then(|(class, atlas)| atlas.texture(class, overlay_engine::Gender::M, false))
        {
            painter.image(
                texture.id(),
                icon,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        let left = row.left() + 10.0 + SECTION_ICON;
        painter.text(
            egui::pos2(left, line_y),
            Align2::LEFT_CENTER,
            &entry.name,
            text::label_font(ui.ctx(), 13.0),
            Color32::WHITE,
        );
        painter.text(
            egui::pos2(row.right() - 8.0, line_y),
            Align2::RIGHT_CENTER,
            format!("+{}", format::thousands(entry.amount)),
            text::label_strong_font(ui.ctx(), 13.0),
            tokens::OVERLAY_ACCENT,
        );
        let track = Rect::from_min_max(
            egui::pos2(left, row.top() + 24.0),
            egui::pos2(row.right() - 8.0, row.top() + 27.0),
        );
        painter.rect_filled(track, 2.0, Color32::from_white_alpha(14));
        let fill = Rect::from_min_size(
            track.min,
            egui::vec2(
                track.width() * entry.amount as f32 / best as f32,
                track.height(),
            ),
        );
        painter.rect_filled(fill, 2.0, tokens::OVERLAY_ACCENT.gamma_multiply(0.8));
    }
}

/// Ce qu'un bloc de butin affiche.
struct LootBlock<'a> {
    icon: DsIcon,
    title: &'a str,
    list: &'a [ItemTotal],
    stone: Option<i64>,
    log_name: &'a str,
}

/// Un bloc de butin (`loot-section` du site) : en-tête et nombre d'objets, recherche et tri dès
/// deux objets, puis la liste — « Aucun butin ramassé. » s'il n'y a rien.
#[allow(clippy::too_many_arguments)]
fn loot_block(
    ui: &mut Ui,
    width: f32,
    block: LootBlock<'_>,
    fold: Fold<'_>,
    filter: &mut LootFilter,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let count = if block.list.is_empty() {
        String::new()
    } else {
        format!("{} objet(s)", format::thousands(block.list.len() as i64))
    };
    if !section_header(
        ui,
        width,
        block.icon,
        Color32::WHITE,
        block.title,
        &count,
        fold,
    ) {
        return;
    }
    if block.list.len() > 1 {
        loot_controls(ui, 0.0, width, filter, block.log_name, ctx, textures);
        ui.add_space(6.0);
    }
    if block.list.is_empty() && block.stone.is_none() {
        empty_line(ui, width, "Aucun butin ramassé.");
        return;
    }
    let lines = shown_loot(block.list, block.stone, filter, ctx.catalog);
    let pinned = usize::from(block.stone.is_some());
    if lines.len() == pinned && !block.list.is_empty() {
        empty_line(ui, width, "Aucun objet ne correspond à cette recherche.");
    }
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

/// La vignette d'une ligne (`dungeon-thumb` du site) : l'illustration, sinon la pierre du type,
/// sinon un pictogramme selon la nature de la ligne.
fn paint_thumb(
    ui: &Ui,
    painter: &egui::Painter,
    rect: Rect,
    group: &RecapGroup<'_>,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let tint = if group.disabled {
        tokens::TEXT_DISABLED
    } else {
        tokens::OVERLAY_TEXT_MUTED
    };
    let fixed = match (group.kind, group.dungeon_type) {
        (GroupKind::Other, _) => Some(DsIcon::UnknownEntity),
        (GroupKind::Type, Some(WakfuDungeonType::Breach)) => Some(DsIcon::Breach),
        (GroupKind::Type, Some(WakfuDungeonType::UltimateBreach)) => Some(DsIcon::UltimateBreach),
        _ => None,
    };
    if let Some(icon) = fixed {
        let native = design::DesignSystem::get(ui.ctx()).icon_native_size(icon);
        // Jamais agrandie (`max-height: 24px` du site) : l'entité inconnue fait 15 px.
        let drawn = if native.x <= rect.width() && native.y <= rect.height() {
            native
        } else {
            crate::design::fit::contain(rect.size(), native)
        };
        let tint = if group.disabled {
            Color32::from_white_alpha(115)
        } else {
            Color32::WHITE
        };
        design::DesignSystem::get(ui.ctx()).paint_icon(
            painter,
            Rect::from_center_size(rect.center(), drawn),
            icon,
            tint,
        );
    } else if let Some(picture) = &group.picture {
        super::paint_remote_icon(ui, painter, rect, picture, ctx.remote_icons, textures);
    } else if let Some(stone) = group
        .stone
        .and_then(|id| ctx.catalog.find_item_icon("", Some(id)))
    {
        super::paint_remote_icon(ui, painter, rect, &stone, ctx.remote_icons, textures);
    } else {
        let icon = match group.kind {
            GroupKind::Other => DsIcon::Help,
            GroupKind::Family => DsIcon::Paw,
            GroupKind::Dungeon | GroupKind::Type => DsIcon::Dungeon,
        };
        super::paint_icon(ui, painter, rect.center(), icon, 20.0, tint);
    }
}

/// Une ligne Donjon / Famille / Type, dans sa boîte (`.group-item` du site) : chevron, vignette,
/// libellé, compte ; dépliée, sa bande (Expérience, Kamas, Victoire, Défaite), ses donjons (mode
/// Type), l'expérience et le butin — dans la même boîte.
#[allow(clippy::too_many_arguments)]
fn group_block(
    ui: &mut Ui,
    width: f32,
    group: &RecapGroup<'_>,
    expanded: &mut HashSet<String>,
    collapsed: &mut HashSet<String>,
    loot: &mut LootFilter,
    classes: &HashMap<String, String>,
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
    let inner = width - 2.0 * (f32::from(GROUP_PAD_X) + 1.0);
    egui::Frame::new()
        .fill(GROUP_FILL)
        .stroke(egui::Stroke::new(1.0, GROUP_BORDER))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(GROUP_PAD_X, GROUP_PAD_Y))
        .show(ui, |ui| {
            ui.set_width(inner);
            group_content(
                ui, inner, group, expanded, collapsed, loot, classes, ctx, textures,
            );
        });
    ui.add_space(GROUP_GAP);
}

/// Le contenu de la boîte d'un groupe — voir [`group_block`].
#[allow(clippy::too_many_arguments)]
fn group_content(
    ui: &mut Ui,
    width: f32,
    group: &RecapGroup<'_>,
    expanded: &mut HashSet<String>,
    collapsed: &mut HashSet<String>,
    loot: &mut LootFilter,
    classes: &HashMap<String, String>,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
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
    let painter = ui.painter_at(row.expand(2.0));
    // Ligne grisée : tout à 45 % (`.dungeon-row.disabled`).
    let dim = |color: Color32| {
        if group.disabled {
            color.gamma_multiply(0.45)
        } else {
            color
        }
    };
    let caret_color = if open {
        tokens::OVERLAY_ACCENT
    } else {
        tokens::OVERLAY_TEXT_MUTED
    };
    super::paint_caret(
        ui,
        &painter,
        egui::pos2(row.left() + 5.0, row.center().y),
        open,
        dim(caret_color),
    );
    // Vignette de 24 px de haut (`.dungeon-thumb`), 9 px après le chevron (`gap: 9px`).
    let thumb = Rect::from_min_size(
        egui::pos2(row.left() + 19.0, row.center().y - 13.0),
        egui::vec2(34.0, 26.0),
    );
    paint_thumb(ui, &painter, thumb, group, ctx, textures);
    let count = painter.text(
        egui::pos2(row.right(), row.center().y),
        Align2::RIGHT_CENTER,
        format!("×{}", format::thousands(row_count(&group.totals))),
        text::label_font(ui.ctx(), 13.0),
        dim(tokens::OVERLAY_TEXT_MUTED),
    );
    let name_rect = Rect::from_min_max(
        egui::pos2(thumb.right() + 9.0, row.top()),
        egui::pos2(count.left() - 9.0, row.bottom()),
    );
    painter.with_clip_rect(name_rect).text(
        name_rect.left_center(),
        Align2::LEFT_CENTER,
        &group.label,
        text::label_strong_font(ui.ctx(), 14.0),
        dim(SITE_TEXT),
    );
    if !open {
        return;
    }
    let totals = &group.totals;
    ui.add_space(8.0);
    band_cells(
        ui,
        width,
        &[
            Cell {
                icon: DsIcon::HeaderXp,
                tint: Color32::WHITE,
                label: "Expérience",
                value: Value::Parts(vec![(
                    format!("+{}", format::thousands(totals.xp_gained)),
                    tokens::OVERLAY_ACCENT,
                )]),
                tooltip: Vec::new(),
            },
            Cell {
                icon: DsIcon::HeaderKamas,
                tint: Color32::WHITE,
                label: "Kamas",
                value: Value::Kamas(
                    format!("+{}", format::thousands(totals.kamas_gained)),
                    tokens::TEXT_GOLD,
                ),
                tooltip: Vec::new(),
            },
            Cell {
                icon: DsIcon::Victory,
                tint: items::GAIN,
                label: "Victoire",
                value: Value::Parts(vec![(format::thousands(totals.won), items::GAIN)]),
                tooltip: Vec::new(),
            },
            Cell {
                icon: DsIcon::Defeat,
                tint: items::LOSS,
                label: "Défaite",
                value: Value::Parts(vec![(format::thousands(totals.lost), items::LOSS)]),
                tooltip: Vec::new(),
            },
        ],
    );
    if !group.tiles.is_empty() {
        ui.add_space(8.0);
        if section_header(
            ui,
            width,
            DsIcon::Dungeon,
            tokens::OVERLAY_TEXT_MUTED,
            "Donjons",
            "",
            Fold::new(collapsed, format!("{}/donjons", group.key)),
        ) {
            tiles(ui, width, &group.tiles, ctx, textures);
        }
    }
    ui.add_space(8.0);
    xp_section(
        ui,
        width,
        &totals.xp_by_character,
        classes,
        ctx,
        Fold::new(collapsed, format!("{}/xp", group.key)),
    );
    ui.add_space(8.0);
    loot_block(
        ui,
        width,
        LootBlock {
            icon: DsIcon::HeaderLoot,
            title: "Butin",
            list: &totals.loot,
            stone: group.stone,
            log_name: "historique.recap.groupe.butin",
        },
        Fold::new(collapsed, format!("{}/butin", group.key)),
        loot,
        ctx,
        textures,
    );
}

/// Les donjons d'une ligne Type (`dungeon-tile-stack` du site) : une tuile par donjon, son
/// illustration posée en bas et son compte en pastille dorée ; au-delà de cinq, elles se
/// chevauchent et celle qu'on survole remonte, grossit et passe devant. Le nom du donjon est
/// l'infobulle de sa tuile.
fn tiles(
    ui: &mut Ui,
    width: f32,
    list: &[Tile],
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) {
    let stacked = list.len() > TILE_STACK_THRESHOLD;
    let step = if stacked {
        TILE_STACK_STEP
    } else {
        TILE_WIDTH + TILE_GAP
    };
    // La pastille déborde de 6 px à droite et 4 px en haut : on lui garde la place.
    let usable = width - 8.0;
    let per_row = if stacked {
        usize::MAX
    } else {
        (((usable + TILE_GAP) / step).floor() as usize).max(1)
    };
    let rows = if stacked {
        1
    } else {
        list.len().div_ceil(per_row)
    };
    let top_pad = if stacked { 8.0 } else { 6.0 };
    let (area, _) = ui.allocate_exact_size(
        egui::vec2(
            width,
            top_pad + rows as f32 * TILE_HEIGHT + (rows - 1) as f32 * TILE_GAP + 4.0,
        ),
        Sense::hover(),
    );
    let rect_of = |index: usize| {
        let (row, column) = if stacked {
            (0, index)
        } else {
            (index / per_row, index % per_row)
        };
        Rect::from_min_size(
            egui::pos2(
                area.left() + column as f32 * step,
                area.top() + top_pad + row as f32 * (TILE_HEIGHT + TILE_GAP),
            ),
            egui::vec2(TILE_WIDTH, TILE_HEIGHT),
        )
    };
    // La tuile survolée : en pile, la première dont le rectangle contient le pointeur (le site
    // empile la première au-dessus : `z-index` décroissant).
    let pointer = ui.ctx().pointer_hover_pos();
    let hovered =
        (0..list.len()).find(|&index| pointer.is_some_and(|p| rect_of(index).contains(p)));
    let painter = ui.painter_at(area.expand(10.0));
    // Ordre de peinture : de la dernière à la première (la première au-dessus), la survolée en
    // dernier.
    let order = (0..list.len())
        .rev()
        .filter(|&index| Some(index) != hovered)
        .chain(hovered);
    for index in order {
        let tile = &list[index];
        let mut rect = rect_of(index);
        if Some(index) == hovered && stacked {
            rect = Rect::from_center_size(rect.center() - egui::vec2(0.0, 4.0), rect.size() * 1.12);
        }
        let scale = rect.height() / TILE_HEIGHT;
        painter.rect_filled(rect, 7.0, TILE_FILL);
        painter.rect_stroke(
            rect,
            7.0,
            egui::Stroke::new(1.0, Color32::from_white_alpha(31)),
            egui::StrokeKind::Inside,
        );
        // L'illustration : 88 % de la vignette, posée en bas (`align-items: flex-end`).
        let thumb = Rect::from_min_size(
            rect.min + egui::vec2(0.0, 3.0 * scale),
            egui::vec2(rect.width(), TILE_THUMB * scale),
        );
        let image = Rect::from_min_max(
            egui::pos2(
                thumb.center().x - thumb.width() * 0.44,
                thumb.bottom() - thumb.height() * 0.88,
            ),
            egui::pos2(thumb.center().x + thumb.width() * 0.44, thumb.bottom()),
        );
        match &tile.picture {
            Some(picture) => super::paint_remote_icon_bottom(
                ui,
                &painter,
                image,
                picture,
                ctx.remote_icons,
                textures,
            ),
            None => super::paint_icon(
                ui,
                &painter,
                image.center(),
                DsIcon::Dungeon,
                22.0,
                tokens::OVERLAY_TEXT_MUTED,
            ),
        }
        // Pastille du compte, en haut à droite et hors du cadre (`top: -4px; right: -6px`).
        let font = text::label_strong_font(ui.ctx(), 11.0);
        let galley = painter.layout_no_wrap(
            format!("×{}", format::thousands(tile.count)),
            font,
            KAMA_GOLD,
        );
        let size = galley.size() + egui::vec2(10.0, 2.0);
        let badge = Rect::from_min_size(
            egui::pos2(rect.right() + 6.0 - size.x, rect.top() - 4.0),
            size,
        );
        painter.rect_filled(badge, 8.0, TILE_BADGE_FILL);
        painter.rect_stroke(
            badge,
            8.0,
            egui::Stroke::new(1.0, Color32::from_white_alpha(46)),
            egui::StrokeKind::Inside,
        );
        painter.galley(badge.min + egui::vec2(5.0, 1.0), galley, KAMA_GOLD);
    }
    if let Some(index) = hovered {
        ui.interact(
            rect_of(index),
            ui.id().with(("tuile", index)),
            Sense::hover(),
        )
        .on_hover_text(&list[index].label);
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
