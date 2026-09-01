//! Panneau "Suivi" (watchlist, §9 du plan) — liste compacte en LECTURE SEULE des entrées suivies
//! (nom/genre/mode/compteur), affichée sous le panneau Combat quand le compte en déclare au moins
//! une (voir l'appelant, `main.rs::render`). Aucune édition possible depuis l'overlay pour cette
//! première version : la liste elle-même reste éditée sur le web (voir `overlay_engine::watchlist`
//! pour la frontière définitions/compteurs), seuls les compteurs — incrémentés localement à chaque
//! ramassage/ennemi vaincu — sont propres à l'overlay.
//!
//! Pas de portrait par entrée (contrairement au panneau Combat, `portraits::PortraitAtlas`) : rien
//! de comparable n'existe pour un objet/monstre quelconque du jeu — un simple repère de couleur
//! (`marker_color`) distingue objet suivi et ennemi suivi ; le nom reste donc affiché en clair,
//! jamais caché derrière un survol comme au panneau Combat (ici le nom EST l'information, il n'y a
//! rien d'autre à montrer à sa place).

use overlay_engine::{WatchlistEntry, WatchlistKind, WatchlistMode};

const ROW_GAP: f32 = 4.0;
const MARKER_SIZE: f32 = 8.0;
const NAME_COLOR: egui::Color32 = egui::Color32::from_rgb(220, 224, 230);

// Même bleu que le switch/la barre de dégâts du panneau Combat (`panels::combat::ACCENT`) — un
// objet suivi partage la charte ; un ennemi suivi s'en distingue par une teinte chaude, seule
// façon de les différencier au premier coup d'œil sans icône dédiée par entrée.
const ITEM_MARKER: egui::Color32 = egui::Color32::from_rgb(0x00, 0xd2, 0xff);
const ENEMY_MARKER: egui::Color32 = egui::Color32::from_rgb(0xff, 0x6b, 0x5b);

/// Même famille visuelle que `panels::combat::damage_bar` (fond opaque très sombre, remplissage
/// bleu, chiffre peint dessus) — réutilisée ici pour le mode `down` uniquement : c'est le seul cas
/// où un dénominateur naturel existe (`countdown_target`), le mode `up` n'a aucun maximum connu à
/// représenter en proportion.
const BAR_HEIGHT: f32 = 14.0;
const BAR_BG: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(8, 10, 16, 235);
const BAR_FILL: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(0x00, 0xd2, 0xff, 110);
const BAR_TEXT: egui::Color32 = egui::Color32::from_rgb(235, 240, 245);
const BAR_TEXT_SHADOW: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(0, 0, 0, 200);

pub fn show(ui: &mut egui::Ui, entries: &[WatchlistEntry]) {
    ui.weak("Suivi");
    ui.add_space(4.0);
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            ui.add_space(ROW_GAP);
        }
        row(ui, entry);
    }
}

fn row(ui: &mut egui::Ui, entry: &WatchlistEntry) {
    let (marker_color, kind_label) = match entry.kind {
        WatchlistKind::Item => (ITEM_MARKER, "Objet suivi"),
        WatchlistKind::Enemy => (ENEMY_MARKER, "Ennemi suivi"),
    };

    ui.horizontal(|ui| {
        let (marker_rect, _resp) =
            ui.allocate_exact_size(egui::vec2(MARKER_SIZE, MARKER_SIZE), egui::Sense::hover());
        ui.painter()
            .circle_filled(marker_rect.center(), MARKER_SIZE / 2.0, marker_color);

        ui.label(egui::RichText::new(&entry.name).color(NAME_COLOR));

        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| match entry.mode {
                WatchlistMode::Up => {
                    ui.label(
                        egui::RichText::new(entry.count.to_string())
                            .strong()
                            .color(marker_color),
                    );
                }
                WatchlistMode::Down => {
                    let width = ui.available_width().clamp(60.0, 140.0);
                    countdown_bar(ui, width, entry.count, entry.countdown_target);
                }
            },
        );
    })
    .response
    .on_hover_text(kind_label);
}

/// Barre de progression du décompte (mode `down`) : remplissage proportionnel à ce qui a déjà été
/// collecté (`target - count`, PAS `count` lui-même — `count` décompte vers 0, voir
/// `overlay_engine::watchlist::WatchlistMode::Down`), chiffres `count/target` peints dessus.
fn countdown_bar(ui: &mut egui::Ui, width: f32, count: i64, target: i64) {
    let width = width.max(1.0);
    let (rect, _response) =
        ui.allocate_exact_size(egui::vec2(width, BAR_HEIGHT), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, BAR_BG);

    if target > 0 {
        let collected = (target - count).max(0);
        let ratio = (collected as f32 / target as f32).clamp(0.0, 1.0);
        if ratio > 0.0 {
            let fill_rect = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(rect.width() * ratio, rect.height()),
            );
            painter.rect_filled(fill_rect, 4.0, BAR_FILL);
        }
    }

    let text = format!("{count}/{target}");
    let font = egui::FontId::monospace(11.0);
    let text_pos = rect.right_center() - egui::vec2(6.0, 0.0);
    painter.text(
        text_pos + egui::vec2(1.0, 1.0),
        egui::Align2::RIGHT_CENTER,
        &text,
        font.clone(),
        BAR_TEXT_SHADOW,
    );
    painter.text(text_pos, egui::Align2::RIGHT_CENTER, &text, font, BAR_TEXT);
}
