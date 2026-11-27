//! **Corriger un objet** — le menu « Interagir » du site (`ItemPickerComponent` pour le butin d'un
//! combat, `PurchaseReassignPickerComponent` pour les achats).
//!
//! Quand le référentiel porte plusieurs objets du même nom (raretés différentes, « Larme
//! d'Ogrest »…), la résolution par nom seul ne sait pas lequel le log désignait. Une ligne de
//! butin ou d'achat concernée porte une petite clé plate ; un clic sur elle, ou un clic droit sur
//! la ligne, ouvre le menu : choisir l'objet juste parmi les homonymes, pour tout ou partie du lot
//! (butin) ou pour les achats cochés (achats). L'écriture passe par
//! `HistoryStore::reassign_fight_loot` / `reassign_purchases`, et les vues relisent la base.
//!
//! Les vues **demandent** l'ouverture ([`open`]) ; la fenêtre peint le menu par-dessus tout
//! ([`show`]), une fois par frame, et dit si la base a changé.

use std::collections::HashSet;

use egui::{Align2, Color32, Rect, Sense, Ui};
use overlay_engine::IconRef;

use super::{format, HistoryContext};
use crate::design::{self, text, tokens, DsIcon};
use crate::remote_icons::RemoteIconTextures;

/// Un achat que la correction peut viser (`PurchaseRecord` du site).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseRecord {
    pub id: i64,
    /// ms UTC.
    pub occurred_at: i64,
    pub quantity: i64,
    pub total_cost: i64,
}

/// Ce que vise la correction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Une ligne du butin d'un combat : son objet tel qu'écrit en base, et sa quantité (borne du
    /// compteur « Modifier »).
    FightLoot {
        fight_id: i64,
        item_name: Option<String>,
        quantity: i64,
    },
    /// Les achats d'une ligne de la vue Achats (cumul d'un objet sur un jour).
    Purchases(Vec<PurchaseRecord>),
}

/// Une demande d'ouverture du menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Nom affiché : c'est lui qui désigne les homonymes.
    pub name: String,
    /// L'objet retenu aujourd'hui pour la ligne — surligné, jamais cliquable.
    pub current_id: Option<i64>,
    /// Coin haut gauche voulu (le pointeur du clic droit).
    pub pos: egui::Pos2,
    pub target: Target,
}

/// Le menu ouvert, et ce que l'utilisateur y a réglé.
struct Open {
    request: Request,
    /// Quantité à corriger (butin), de 1 au lot entier.
    quantity: i64,
    /// Achats cochés (achats) — tous au départ, comme le site.
    selected: HashSet<i64>,
}

#[derive(Default)]
pub struct CorrectionState {
    open: Option<Open>,
}

fn request_id() -> egui::Id {
    egui::Id::new("historique-correction-demande")
}

fn modal_id() -> egui::Id {
    egui::Id::new("historique-correction")
}

/// Demande l'ouverture du menu — la fenêtre l'ouvre à la frame suivante.
pub fn open(ctx: &egui::Context, request: Request) {
    tracing::info!(">>> clic historique.correction {}", request.name);
    ctx.data_mut(|data| data.insert_temp(request_id(), request));
}

/// Vrai si l'objet `name` peut être corrigé (il a des homonymes).
pub fn correctable(ctx: &HistoryContext<'_>, name: &str) -> bool {
    !ctx.catalog.item_homonyms(name).is_empty()
}

/// Côté de la clé plate et de sa case (`.loot-fix-btn` : 18 px, glyphe de 13).
pub const FIX_SLOT: f32 = 20.0;
const FIX_GLYPH: f32 = 13.0;

/// La clé plate d'une ligne corrigeable, centrée sur `center`. Rend `true` au clic.
pub fn fix_button(ui: &Ui, center: egui::Pos2, id: egui::Id) -> bool {
    let rect = Rect::from_center_size(center, egui::vec2(FIX_SLOT, FIX_SLOT));
    let response = ui
        .interact(rect, id, Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Clique pour corriger");
    let tint = if response.hovered() {
        tokens::OVERLAY_ACCENT
    } else {
        tokens::OVERLAY_TEXT_MUTED
    };
    super::paint_icon(ui, ui.painter(), center, DsIcon::Wrench, FIX_GLYPH, tint);
    response.clicked()
}

/// La clé plate et le clic droit d'une ligne : rend la position où ouvrir le menu, le cas
/// échéant. `row` est la ligne entière, `fix` le centre de la clé.
pub fn row_trigger(ui: &Ui, row: Rect, fix: egui::Pos2, id: egui::Id) -> Option<egui::Pos2> {
    let response = ui.interact(row, id.with("ligne"), Sense::click());
    if fix_button(ui, fix, id.with("cle")) {
        return Some(fix + egui::vec2(-FIX_SLOT / 2.0, FIX_SLOT / 2.0));
    }
    if response.secondary_clicked() {
        return response.interact_pointer_pos();
    }
    None
}

/// Le menu, s'il est ouvert ; rend `true` si la base vient de changer.
pub fn show(
    ui: &Ui,
    state: &mut CorrectionState,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) -> bool {
    if let Some(request) = ui.ctx().data_mut(|data| {
        let request = data.get_temp::<Request>(request_id());
        data.remove::<Request>(request_id());
        request
    }) {
        let quantity = match &request.target {
            Target::FightLoot { quantity, .. } => (*quantity).max(1),
            Target::Purchases(_) => 1,
        };
        let selected = match &request.target {
            Target::Purchases(records) => records.iter().map(|record| record.id).collect(),
            Target::FightLoot { .. } => HashSet::new(),
        };
        state.open = Some(Open {
            request,
            quantity,
            selected,
        });
    }
    let Some(open) = state.open.as_mut() else {
        return false;
    };
    let candidates = ctx.catalog.item_homonyms(&open.request.name).to_vec();

    // Le menu ne déborde pas de la fenêtre (`updateDisplayPosition` du site).
    let screen = ui.ctx().content_rect();
    let size = ui
        .ctx()
        .memory(|memory| memory.area_rect(modal_id()))
        .map_or(egui::vec2(WIDTH, 200.0), |rect| rect.size());
    let pos = egui::pos2(
        open.request
            .pos
            .x
            .min(screen.right() - size.x - MARGIN)
            .max(MARGIN),
        open.request
            .pos
            .y
            .min(screen.bottom() - size.y - MARGIN)
            .max(MARGIN),
    );
    let frame = egui::Frame::new()
        .fill(FILL)
        .stroke(egui::Stroke::new(1.0, tokens::OVERLAY_ACCENT))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(10))
        .shadow(egui::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(150),
        });
    let mut chosen = None;
    let response = egui::Modal::new(modal_id())
        .area(
            egui::Modal::default_area(modal_id())
                .anchor(Align2::LEFT_TOP, pos.to_vec2() - screen.min.to_vec2()),
        )
        .backdrop_color(Color32::from_black_alpha(102))
        .frame(frame)
        .show(ui.ctx(), |ui| {
            ui.set_width(WIDTH);
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
            title(ui);
            ui.add_space(4.0);
            if candidates.len() < 2 {
                return;
            }
            match &open.request.target {
                Target::FightLoot { quantity, .. } => {
                    quantity_row(ui, &mut open.quantity, (*quantity).max(1));
                }
                Target::Purchases(records) => {
                    records_list(ui, records, &mut open.selected);
                }
            }
            let has_selection = match &open.request.target {
                Target::FightLoot { .. } => true,
                Target::Purchases(_) => !open.selected.is_empty(),
            };
            for &id in &candidates {
                if candidate(
                    ui,
                    id,
                    open.request.current_id,
                    has_selection,
                    ctx,
                    textures,
                ) {
                    chosen = Some(id);
                }
            }
        });
    let close = response.should_close() || response.backdrop_response.secondary_clicked();
    let mut changed = false;
    if let (Some(new_id), Some(store)) = (chosen, ctx.store) {
        let result = match &open.request.target {
            Target::FightLoot {
                fight_id,
                item_name,
                ..
            } => store
                .reassign_fight_loot(
                    *fight_id,
                    open.request.current_id,
                    item_name.as_deref(),
                    open.quantity,
                    new_id,
                )
                .map(|moved| moved > 0),
            Target::Purchases(records) => {
                let ids: Vec<i64> = records
                    .iter()
                    .map(|record| record.id)
                    .filter(|id| open.selected.contains(id))
                    .collect();
                store
                    .reassign_purchases(&ids, new_id)
                    .map(|changed| changed > 0)
            }
        };
        match result {
            Ok(done) => {
                tracing::info!(
                    "[historique] {} corrigé : objet {:?} -> {new_id}",
                    open.request.name,
                    open.request.current_id
                );
                changed = done;
            }
            Err(err) => tracing::warn!(%err, "historique : correction impossible"),
        }
    }
    if close || chosen.is_some() {
        state.open = None;
    }
    changed
}

/// Largeur du menu (`.item-picker` : 280 px, à l'échelle de la fenêtre).
const WIDTH: f32 = 300.0;
const MARGIN: f32 = 8.0;
/// `--surface-raised`, `--surface-active`, `--border-strong`, `--text-bright`.
const FILL: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
const ACTIVE: Color32 = Color32::from_rgb(0x33, 0x33, 0x33);
const BORDER: Color32 = Color32::from_rgb(0x4D, 0x4D, 0x4D);
const BRIGHT: Color32 = Color32::from_rgb(0xF2, 0xF2, 0xF2);
const KAMA_GOLD: Color32 = Color32::from_rgb(255, 215, 0);

/// Texte en capitales espacées.
fn spaced_caps(
    ui: &Ui,
    label: &str,
    size: f32,
    color: Color32,
    strong: bool,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &label.to_uppercase(),
        0.0,
        egui::TextFormat {
            font_id: if strong {
                text::label_strong_font(ui.ctx(), size)
            } else {
                text::label_font(ui.ctx(), size)
            },
            color,
            extra_letter_spacing: 0.5,
            ..Default::default()
        },
    );
    ui.painter().layout_job(job)
}

/// « INTERAGIR », en bleu (`.item-picker-title`).
fn title(ui: &mut Ui) {
    let galley = spaced_caps(ui, "Interagir", 12.5, tokens::OVERLAY_ACCENT, true);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(WIDTH, galley.size().y), Sense::hover());
    ui.painter()
        .galley(rect.left_top(), galley, tokens::OVERLAY_ACCENT);
}

/// « Modifier » et le compteur de quantité (`.item-picker-modify-row`).
fn quantity_row(ui: &mut Ui, quantity: &mut i64, max: i64) {
    *quantity = (*quantity).clamp(1, max);
    let stepper = design::stepper(quantity)
        .range(1..=max)
        .size(26.0)
        .field_width(44.0)
        .log_name("historique.correction.quantite");
    let stepper_width = stepper.desired_size().0.unwrap_or(110.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(WIDTH, 32.0), Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left() + 2.0, rect.center().y),
        Align2::LEFT_CENTER,
        "Modifier",
        text::label_font(ui.ctx(), 13.0),
        BRIGHT,
    );
    ui.put(
        Rect::from_min_size(
            egui::pos2(rect.right() - 2.0 - stepper_width, rect.center().y - 13.0),
            egui::vec2(stepper_width, 26.0),
        ),
        stepper,
    );
}

/// Les achats à réattribuer, cochés (`.purchase-reassign-records`).
fn records_list(ui: &mut Ui, records: &[PurchaseRecord], selected: &mut HashSet<i64>) {
    let label = spaced_caps(
        ui,
        "Lignes à réattribuer",
        11.5,
        tokens::OVERLAY_TEXT_MUTED,
        false,
    );
    let (rect, _) = ui.allocate_exact_size(egui::vec2(WIDTH, label.size().y + 2.0), Sense::hover());
    ui.painter().galley(
        rect.left_top() + egui::vec2(2.0, 2.0),
        label,
        tokens::OVERLAY_TEXT_MUTED,
    );
    egui::ScrollArea::vertical()
        .id_salt("historique-correction-achats")
        .max_height(154.0)
        .show(ui, |ui| {
            for record in records {
                let (row, response) =
                    ui.allocate_exact_size(egui::vec2(WIDTH, 26.0), Sense::click());
                let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                if response.clicked() && !selected.remove(&record.id) {
                    selected.insert(record.id);
                }
                let checked = selected.contains(&record.id);
                let painter = ui.painter_at(row);
                let border = if response.hovered() {
                    tokens::OVERLAY_ACCENT
                } else {
                    BORDER
                };
                painter.rect_filled(row, 5.0, FILL);
                painter.rect_stroke(
                    row,
                    5.0,
                    egui::Stroke::new(1.0, border),
                    egui::StrokeKind::Inside,
                );
                let boxed = Rect::from_center_size(
                    egui::pos2(row.left() + 14.0, row.center().y),
                    egui::vec2(13.0, 13.0),
                );
                if checked {
                    painter.rect_filled(boxed, 3.0, tokens::OVERLAY_ACCENT);
                    super::paint_icon(
                        ui,
                        &painter,
                        boxed.center(),
                        DsIcon::Victory,
                        9.0,
                        Color32::BLACK,
                    );
                } else {
                    painter.rect_stroke(
                        boxed,
                        3.0,
                        egui::Stroke::new(1.0, tokens::OVERLAY_TEXT_MUTED),
                        egui::StrokeKind::Inside,
                    );
                }
                let font = text::label_font(ui.ctx(), 12.5);
                let time = painter.text(
                    egui::pos2(boxed.right() + 7.0, row.center().y),
                    Align2::LEFT_CENTER,
                    format::time_of_day(record.occurred_at),
                    font.clone(),
                    tokens::OVERLAY_TEXT_MUTED,
                );
                painter.text(
                    egui::pos2(time.right() + 62.0, row.center().y),
                    Align2::RIGHT_CENTER,
                    format!("×{}", format::thousands(record.quantity)),
                    font.clone(),
                    BRIGHT,
                );
                super::paint_amount(
                    ui,
                    &painter,
                    egui::pos2(row.right() - 8.0, row.center().y),
                    &format::thousands(record.total_cost),
                    font,
                    KAMA_GOLD,
                );
            }
        });
}

/// Un objet candidat (`.item-picker-item`) : gemme de rareté, icône, nom. Rend `true` au clic.
fn candidate(
    ui: &mut Ui,
    id: i64,
    current: Option<i64>,
    has_selection: bool,
    ctx: &HistoryContext<'_>,
    textures: &mut RemoteIconTextures,
) -> bool {
    let is_current = current == Some(id);
    let enabled = !is_current && has_selection;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (row, response) = ui.allocate_exact_size(egui::vec2(WIDTH, 34.0), sense);
    let response = if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else if !is_current {
        response.on_hover_text("Sélectionnez au moins une ligne")
    } else {
        response
    };
    let name = ctx
        .catalog
        .item_name(id)
        .unwrap_or("Objet inconnu")
        .to_string();
    let rarity = ctx.catalog.find_item_rarity(&name, Some(id));
    let painter = ui.painter_at(row);
    let (fill, border) = if is_current || (enabled && response.hovered()) {
        (ACTIVE, tokens::OVERLAY_ACCENT)
    } else {
        (FILL, BORDER)
    };
    painter.rect_filled(row, 5.0, fill);
    painter.rect_stroke(
        row,
        5.0,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    let gem = Rect::from_center_size(
        egui::pos2(row.left() + 8.0 + 6.0, row.center().y),
        egui::vec2(12.0, 12.0),
    );
    super::paint_remote_icon(
        ui,
        &painter,
        gem,
        &IconRef::for_rarity(rarity),
        ctx.remote_icons,
        textures,
    );
    let icon = Rect::from_center_size(
        egui::pos2(gem.right() + 8.0 + 11.0, row.center().y),
        egui::vec2(22.0, 22.0),
    );
    if let Some(item_icon) = ctx.catalog.find_item_icon(&name, Some(id)) {
        super::paint_remote_icon(ui, &painter, icon, &item_icon, ctx.remote_icons, textures);
    }
    let name_rect = Rect::from_min_max(
        egui::pos2(icon.right() + 8.0, row.top()),
        egui::pos2(row.right() - 8.0, row.bottom()),
    );
    painter.with_clip_rect(name_rect).text(
        name_rect.left_center(),
        Align2::LEFT_CENTER,
        &name,
        text::label_font(ui.ctx(), 13.5),
        BRIGHT,
    );
    // Désactivé : tout à 55 % (`opacity: 0.55`).
    if !enabled {
        painter.rect_filled(row, 5.0, Color32::from_black_alpha(100));
    }
    response.clicked() && enabled
}
