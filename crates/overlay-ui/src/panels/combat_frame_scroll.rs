//! Cadre décoratif ennemi au-delà de `combat_frame::MAX_FRAME_SLOTS` (6) — breaches et autres
//! combats à monstres nombreux (40 à 60+, aucun gabarit n'ira jamais aussi loin). Design validé
//! itérativement avec l'utilisateur via des artefacts interactifs successifs (2026-09-07) :
//!
//! - **Pas de nouveau gabarit, pas de pavage extensible** : réutilise TEL QUEL le plus grand gabarit
//!   existant (`template_6.png`, via `combat_frame::CombatFrame::largest_texture`) comme fenêtre
//!   FIXE, immobile quel que soit le nombre réel d'ennemis — jamais redessinée pendant le scroll.
//! - Seuls les portraits défilent, clippés dans la bande verticale occupée par les 6 emplacements du
//!   gabarit (`ui.shrink_clip_rect`, l'équivalent egui d'un `overflow: hidden` CSS). Le clip fait
//!   TOUT le travail de masquage — pas besoin de repeindre le cadre par-dessus les portraits qui
//!   débordent en haut/bas pendant le défilement, contrairement à une première tentative
//!   (recomposition du cadre par pièces détachées) abandonnée en cours de calibration.
//! - Géométrie ENTIÈREMENT dérivée de `combat_frame::largest_frame_geometry` (les 6 centres déjà
//!   validés du plus grand gabarit, voir sa doc) : le pas (`pitch`) entre deux portraits qui
//!   défilent est l'écart moyen entre ces 6 centres, la bande de clip va du premier centre moins le
//!   rayon au dernier centre plus le rayon — aucune nouvelle mesure.
//! - Scrollbar : PAS les assets `scrollbar-active.png`/`scrollbar-inactive.png` fournis par
//!   l'utilisateur — essayés en texture de la barre (étirée sur toute sa hauteur), rejetés (retour
//!   utilisateur explicite : trop large, cachait les portraits). Remplacés par une barre DESSINÉE,
//!   fine (`SCROLLBAR_WIDTH`), couleur unie `SCROLLBAR_COLOR` (demande explicite), bordure noire
//!   ~1px, coins légèrement arrondis, collée au bord gauche du cadre — entre les deux pointes de
//!   décoration du gabarit, qui coïncident presque exactement avec la bande de clip des portraits
//!   (mesuré par analyse pixel du PNG, voir historique de session). Hauteur DYNAMIQUE (proportion
//!   emplacements visibles/nombre total d'ennemis, comme un ascenseur classique), cachée par défaut,
//!   visible seulement au survol du cadre ENTIER (pas seulement de la bande) — glissable, cliquable
//!   pour sauter directement à une position ; la molette fonctionne sur toute la zone.
//!
//! **État de scroll** : persisté via `egui::Context::data_mut` (stockage "temp", clé fixe) plutôt
//! que remonté jusqu'à `OverlayWindow` (voir `main.rs`) au travers de `RenderContent`/`build_ui`/
//! `paint_content` et de tous les tests `overlay-testkit` — inutile pour un état purement local à ce
//! widget, exactement comme un `egui::ScrollArea` gère sa propre position de défilement sans jamais
//! la faire remonter à l'appelant. Une seule fenêtre Combat par personnage (voir `main.rs::
//! OverlayWindow`) a donc son propre décalage, puisque chaque fenêtre porte son propre
//! `egui::Context`.

use overlay_engine::FighterDamage;

use crate::portraits::{PortraitAtlas, NATIVE_PORTRAIT_SIZE};
use crate::ui_icons::UiIcons;

use super::combat_frame::{largest_frame_geometry, CombatFrame, MAX_FRAME_SLOTS};

/// Couleur unie de la scrollbar — demande explicite de l'utilisateur, PAS une teinte dérivée du
/// thème ni des PNG essayés puis rejetés (voir doc de module).
const SCROLLBAR_COLOR: egui::Color32 = egui::Color32::from_rgb(0x99, 0x8a, 0x6c);
/// Bordure noire ~1px demandée explicitement — légèrement transparente (comme la maquette validée
/// dans l'artefact) plutôt que noir plein.
const SCROLLBAR_BORDER: egui::Color32 = egui::Color32::from_black_alpha(217);
const SCROLLBAR_BORDER_WIDTH: f32 = 1.0;
/// Coins "légèrement" arrondis — demande explicite, PAS un stade/pilule complet.
const SCROLLBAR_ROUNDING: u8 = 2;
/// Largeur RÉELLE de la barre dessinée — fine, comparable aux tiges des ornements du gabarit (PAS
/// la largeur de 14px des PNG essayés puis rejetés, qui la faisait cacher les portraits).
const SCROLLBAR_WIDTH: f32 = 5.0;
/// Largeur de la zone cliquable/glissable — plus généreuse que la barre visuelle dessinée, pour
/// rester facile à attraper (même écart que dans l'artefact de calibration).
const SCROLLBAR_HIT_WIDTH: f32 = 14.0;
/// Hauteur minimale de la barre, même à très grand nombre d'ennemis — reste cliquable et visible.
const SCROLLBAR_MIN_HEIGHT: f32 = 20.0;

/// Clé de stockage du décalage de scroll courant, en nombre FRACTIONNAIRE d'emplacements (0 = le
/// premier ennemi de `fighters` est aligné sur le 1ᵉʳ emplacement du gabarit) — voir doc de module
/// pour pourquoi ce n'est pas un champ de `OverlayWindow`.
fn scroll_offset_id() -> egui::Id {
    egui::Id::new("combat-frame-scroll-enemy-offset")
}

/// Voir la doc de module.
pub struct EnemyFrameScroll;

impl EnemyFrameScroll {
    /// Dessine le cadre fixe (plus grand gabarit) + les portraits défilants pour `fighters`
    /// (longueur STRICTEMENT supérieure à `MAX_FRAME_SLOTS` — panique en debug sinon : en dessous,
    /// c'est `CombatFrame::show` qu'il faut appeler, voir `panels::combat::show`). `total_damage`
    /// sert uniquement au pourcentage peint sur chaque portrait visible, même rôle que dans
    /// `CombatFrame::show`.
    pub fn show(
        ui: &mut egui::Ui,
        frame: &CombatFrame,
        portraits: &PortraitAtlas,
        icons: &UiIcons,
        fighters: &[&FighterDamage],
        total_damage: i64,
    ) {
        debug_assert!(fighters.len() > MAX_FRAME_SLOTS);
        let n = fighters.len();
        let geometry = largest_frame_geometry();
        // Toujours >= 1 (voir le `debug_assert!` ci-dessus : `n > MAX_FRAME_SLOTS`).
        let max_offset = (n - MAX_FRAME_SLOTS) as f32;

        let frame_rect = ui
            .allocate_exact_size(geometry.frame_size, egui::Sense::hover())
            .0;

        // Cadre fixe — peint une seule fois, jamais redessiné pendant le scroll (voir doc de
        // module) : c'est le clip ci-dessous qui masque les portraits hors-champ, pas un
        // recouvrement du cadre.
        egui::Image::new(frame.largest_texture()).paint_at(ui, frame_rect);

        // Bande de clip : du 1ᵉʳ centre moins le rayon au dernier centre plus le rayon (voir doc de
        // module) — c'est elle qui fait tout le travail de masquage.
        let clip_top = frame_rect.min.y + geometry.first_center_y - geometry.portrait_radius;
        let clip_bottom = frame_rect.min.y
            + geometry.first_center_y
            + geometry.pitch * (MAX_FRAME_SLOTS - 1) as f32
            + geometry.portrait_radius;
        let clip_rect = egui::Rect::from_min_max(
            egui::pos2(frame_rect.min.x, clip_top),
            egui::pos2(frame_rect.max.x, clip_bottom),
        );

        // Survol du cadre ENTIER (pas seulement de la bande de scrollbar) : révèle la barre et
        // active la molette — voir doc de module.
        let hovering_frame = ui
            .interact(
                frame_rect,
                ui.id().with("enemy-scroll-hover"),
                egui::Sense::hover(),
            )
            .hovered();

        let scroll_id = scroll_offset_id();
        let mut offset = ui
            .ctx()
            .data_mut(|d| *d.get_temp_mut_or_insert_with(scroll_id, || 0.0_f32));
        if hovering_frame {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                // Un mouvement de molette déplace d'une fraction de portrait (pas un ennemi entier
                // d'un coup) — défilement plus doux, comme un `ScrollArea` classique.
                offset -= scroll_delta / geometry.pitch;
            }
        }
        offset = offset.clamp(0.0, max_offset);

        // Centre vertical (repère écran) de l'ennemi d'index `i`, compte tenu du décalage courant.
        let slot_center_y = |i: usize| {
            frame_rect.min.y + geometry.first_center_y + (i as f32 - offset) * geometry.pitch
        };
        let is_visible = |cy: f32| {
            cy + geometry.portrait_radius >= clip_rect.min.y
                && cy - geometry.portrait_radius <= clip_rect.max.y
        };

        // Portraits défilants, peints UNIQUEMENT dans la bande de clip (voir doc de module) —
        // `ui.scope` isole le `shrink_clip_rect` au temps de cette fermeture.
        ui.scope(|ui| {
            ui.shrink_clip_rect(clip_rect);
            for (i, fighter) in fighters.iter().enumerate() {
                let cy = slot_center_y(i);
                if !is_visible(cy) {
                    // Culling : le clip s'en chargerait de toute façon, mais évite de peindre des
                    // portraits totalement hors-champ à chaque frame.
                    continue;
                }
                let portrait_rect = egui::Rect::from_center_size(
                    egui::pos2(frame_rect.min.x + geometry.slot_x, cy),
                    egui::vec2(NATIVE_PORTRAIT_SIZE, NATIVE_PORTRAIT_SIZE),
                );
                let radius = geometry.portrait_radius as u8;
                let texture = fighter.class_name.as_deref().and_then(|class_name| {
                    portraits.texture(class_name, fighter.gender, fighter.is_ko)
                });
                match texture {
                    Some(texture) => {
                        egui::Image::new(texture)
                            .corner_radius(radius)
                            .paint_at(ui, portrait_rect);
                    }
                    None => {
                        let image = egui::Image::new(icons.unknown_entity_texture())
                            .corner_radius(radius)
                            .tint(super::combat::grey_tint_if_ko(fighter.is_ko));
                        image.paint_at(ui, portrait_rect);
                    }
                }
            }
        });

        // Infobulle + pourcentage — même logique que `CombatFrame::show`, mais uniquement pour les
        // portraits actuellement dans la bande visible (peints ci-dessus) : un portrait hors-champ
        // ne doit recevoir ni interaction ni incrustation.
        for (i, fighter) in fighters.iter().enumerate() {
            let cy = slot_center_y(i);
            if !is_visible(cy) {
                continue;
            }
            let portrait_rect = egui::Rect::from_center_size(
                egui::pos2(frame_rect.min.x + geometry.slot_x, cy),
                egui::vec2(NATIVE_PORTRAIT_SIZE, NATIVE_PORTRAIT_SIZE),
            );
            let id = ui.id().with(("enemy-scroll-slot", fighter.name.as_str()));
            let response = ui.interact(portrait_rect, id, egui::Sense::hover());
            super::combat::show_tooltip_above(&response, fighter.name.as_str());
            if fighter.total_damage > 0 {
                super::combat::paint_portrait_percent(
                    ui,
                    portrait_rect,
                    fighter.total_damage,
                    total_damage,
                );
            }
        }

        // Scrollbar : fine, collée au bord gauche, hauteur dynamique — voir doc de module.
        let band_height = clip_rect.height();
        let visible_frac = (MAX_FRAME_SLOTS as f32 / n as f32).min(1.0);
        let thumb_height = (band_height * visible_frac).max(SCROLLBAR_MIN_HEIGHT);
        let usable = (band_height - thumb_height).max(0.0);
        let thumb_top = clip_rect.min.y + usable * (offset / max_offset);
        let hit_rect = egui::Rect::from_min_size(
            egui::pos2(frame_rect.min.x, clip_rect.min.y),
            egui::vec2(SCROLLBAR_HIT_WIDTH, band_height),
        );
        let thumb_rect = egui::Rect::from_min_size(
            egui::pos2(frame_rect.min.x, thumb_top),
            egui::vec2(SCROLLBAR_WIDTH, thumb_height),
        );

        let hit_id = ui.id().with("enemy-scroll-hit");
        let hit_response = ui.interact(hit_rect, hit_id, egui::Sense::click_and_drag());
        if (hit_response.dragged() || hit_response.clicked()) && usable > 0.0 {
            if let Some(pointer) = hit_response.interact_pointer_pos() {
                let target_top = pointer.y - thumb_height / 2.0;
                offset =
                    ((target_top - clip_rect.min.y) / usable * max_offset).clamp(0.0, max_offset);
            }
        }

        // Cachée par défaut (voir doc de module) : visible seulement au survol du cadre entier, ou
        // tant que la barre elle-même est survolée/en cours de glissement (pour ne pas disparaître
        // sous le pointeur pendant un drag qui sortirait légèrement de `frame_rect`).
        let show_scrollbar = hovering_frame || hit_response.hovered() || hit_response.dragged();
        if show_scrollbar {
            ui.painter().rect(
                thumb_rect,
                SCROLLBAR_ROUNDING,
                SCROLLBAR_COLOR,
                egui::Stroke::new(SCROLLBAR_BORDER_WIDTH, SCROLLBAR_BORDER),
                egui::StrokeKind::Outside,
            );
        }

        ui.ctx().data_mut(|d| d.insert_temp(scroll_id, offset));
    }
}
