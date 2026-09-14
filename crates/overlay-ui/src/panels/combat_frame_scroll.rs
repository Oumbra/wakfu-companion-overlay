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
//! - Scrollbar (dessin et interaction dans `combat_scrollbar` depuis le 13 sept. 2026, partagés
//!   avec la fenêtre des barres de dégâts) : PAS les assets `scrollbar-active.png`/
//!   `scrollbar-inactive.png` fournis par l'utilisateur — essayés en texture de la barre (étirée sur toute sa hauteur), rejetés (retour
//!   utilisateur explicite : trop large, cachait les portraits). Remplacés par une barre DESSINÉE,
//!   fine (`SCROLLBAR_WIDTH`), couleur unie `SCROLLBAR_COLOR` (demande explicite), bordure noire
//!   ~1px, coins légèrement arrondis, collée au bord gauche du cadre — entre les deux pointes de
//!   décoration du gabarit, qui coïncident presque exactement avec la bande de clip des portraits
//!   (mesuré par analyse pixel du PNG, voir historique de session). Hauteur DYNAMIQUE (proportion
//!   emplacements visibles/nombre total d'ennemis, comme un ascenseur classique), glissable,
//!   cliquable pour sauter directement à une position ; la molette fonctionne sur toute la zone.
//!   **TOUJOURS visible** (revirement explicite de l'utilisateur après test en jeu réel,
//!   2026-09-08) — la toute première version la cachait sauf survol du cadre entier, jugée
//!   finalement inutile une fois testée : la barre, fine et collée au bord, ne gêne pas assez pour
//!   justifier de la cacher.
//! - Portraits : chaque ennemi n'a jamais de `class_name` (`breed` non déterministe côté ennemi) —
//!   son portrait RÉEL est résolu via le catalogue (`panels::combat::resolve_fighter_texture`,
//!   voir sa doc), repli générique tant qu'il n'a pas fini de télécharger ou si le nom n'est pas
//!   reconnu, exactement comme la liste "plate" le faisait déjà pour les ennemis avant cette
//!   refonte.
//!
//! **État de scroll** : persisté via `egui::Context::data_mut` (stockage "temp", clé fixe) plutôt
//! que remonté jusqu'à `OverlayWindow` (voir `main.rs`) au travers de `RenderContent`/`build_ui`/
//! `paint_content` et de tous les tests `overlay-testkit` — inutile pour un état purement local à ce
//! widget, exactement comme un `egui::ScrollArea` gère sa propre position de défilement sans jamais
//! la faire remonter à l'appelant. Une seule fenêtre Combat par personnage (voir `main.rs::
//! OverlayWindow`) a donc son propre décalage, puisque chaque fenêtre porte son propre
//! `egui::Context`.

use overlay_engine::{CatalogIndex, FighterDamage};

use crate::portraits::{PortraitAtlas, NATIVE_PORTRAIT_SIZE};
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::ui_icons::UiIcons;

use super::combat::CombatMetric;
use super::combat_frame::{largest_frame_geometry, CombatFrame, SelectionMarks, MAX_FRAME_SLOTS};
use super::combat_scrollbar::DrawnScrollbar;

/// Clé de stockage du décalage de scroll courant, en nombre FRACTIONNAIRE d'emplacements (0 = le
/// premier ennemi de `fighters` est aligné sur le 1ᵉʳ emplacement du gabarit) — voir doc de module
/// pour pourquoi ce n'est pas un champ de `OverlayWindow`.
fn scroll_offset_id() -> egui::Id {
    egui::Id::new("combat-frame-scroll-enemy-offset")
}

/// Identifiant egui d'un portrait du cadre à défilement — index dans `fighters`, PAS la position
/// visible (qui change au défilement et casserait le fondu des marques), et le nom : deux
/// homonymes ne partagent pas le même `Id` (voir `CombatFrame::show`). Le même identifiant sert
/// aux marques (peintes sous le clip) et à l'interaction (hors clip).
fn slot_id(ui: &egui::Ui, index: usize, fighter: &FighterDamage) -> egui::Id {
    ui.id()
        .with(("enemy-scroll-slot", index, fighter.name.as_str()))
}

/// Voir la doc de module.
pub struct EnemyFrameScroll;

impl EnemyFrameScroll {
    /// Dessine le cadre fixe (plus grand gabarit) + les portraits défilants pour `fighters`
    /// (longueur STRICTEMENT supérieure à `MAX_FRAME_SLOTS` — panique en debug sinon : en dessous,
    /// c'est `CombatFrame::show` qu'il faut appeler, voir `panels::combat::show`). `total_damage`
    /// sert uniquement au pourcentage peint sur chaque portrait visible, même rôle que dans
    /// `CombatFrame::show`.
    ///
    /// `catalog`/`remote_icons`/`remote_icon_textures` résolvent le portrait RÉEL de chaque
    /// ennemi via le catalogue (voir `panels::combat::resolve_fighter_texture`) — sans eux, tout
    /// ennemi affiché ici retomberait sur le portrait générique (jamais de `class_name` côté
    /// ennemi).
    ///
    /// **Sélection du bloc de sorts** (13 sept. 2026, vue Ennemis) : même contrat que
    /// `CombatFrame::show` — `marks` en index dans `fighters` (la tranche complète, pas les
    /// emplacements visibles), portraits des lanceurs cliquables, marques peintes dans la bande de
    /// clip juste après chaque portrait — elles défilent et se rognent avec lui, sous le
    /// pourcentage et sous la scrollbar ; renvoie l'index cliqué cette frame. Tout cela sur les
    /// portraits VISIBLES seulement, comme l'infobulle et le pourcentage : un portrait hors-champ
    /// ne reçoit ni interaction ni marque, et le cadre ne défile jamais de lui-même vers le
    /// dernier lanceur (voir la doc de module de `combat_spell_block`).
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut egui::Ui,
        frame: &CombatFrame,
        portraits: &PortraitAtlas,
        icons: &UiIcons,
        catalog: &CatalogIndex,
        remote_icons: &RemoteIconStore,
        remote_icon_textures: &mut RemoteIconTextures,
        fighters: &[&FighterDamage],
        metric: CombatMetric,
        total_damage: i64,
        marks: Option<SelectionMarks>,
    ) -> Option<usize> {
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

        // Survol du cadre ENTIER (pas seulement de la bande de scrollbar) : active la molette —
        // voir doc de module (la scrollbar, elle, est désormais TOUJOURS visible).
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
                let portrait = super::combat::resolve_fighter_texture(
                    ui,
                    portraits,
                    catalog,
                    remote_icons,
                    remote_icon_textures,
                    fighter,
                );
                match portrait {
                    Some(super::combat::FighterPortrait::ClassPortrait(texture)) => {
                        egui::Image::new(&texture)
                            .corner_radius(radius)
                            .paint_at(ui, portrait_rect);
                    }
                    Some(super::combat::FighterPortrait::RemoteMonster(texture)) => {
                        // Portrait RÉEL du monstre — pas de version grisée précalculée pour
                        // celle-ci, simple tint si KO (voir `grey_tint_if_ko`).
                        egui::Image::new(&texture)
                            .corner_radius(radius)
                            .tint(super::combat::grey_tint_if_ko(fighter.is_ko))
                            .paint_at(ui, portrait_rect);
                    }
                    None => {
                        let image = egui::Image::new(icons.unknown_entity_texture())
                            .corner_radius(radius)
                            .tint(super::combat::grey_tint_if_ko(fighter.is_ko));
                        image.paint_at(ui, portrait_rect);
                    }
                }
                // Marques du bloc de sorts ICI, dans la bande de clip et avant tout le reste
                // (retour utilisateur du 13 sept. 2026) : le liseré suit le portrait au
                // défilement et se rogne avec lui au bord de la bande, et il passe SOUS le
                // pourcentage (peint plus bas) et SOUS la scrollbar (peinte en dernier) — jamais
                // par-dessus l'un ou l'autre.
                if let Some(marks) = marks {
                    super::combat_spell_block::paint_marks(
                        ui,
                        portrait_rect,
                        slot_id(ui, i, fighter),
                        marks.ring_slot == Some(i),
                        marks.dot_slot == Some(i),
                    );
                }
            }
        });

        // Infobulle, clic + pourcentage — même logique que `CombatFrame::show`, mais uniquement
        // pour les portraits actuellement dans la bande visible (peints ci-dessus) : un portrait
        // hors-champ ne doit recevoir ni interaction ni incrustation. Les marques sont déjà
        // peintes, sous le clip (voir ci-dessus).
        let mut clicked = None;
        for (i, fighter) in fighters.iter().enumerate() {
            let cy = slot_center_y(i);
            if !is_visible(cy) {
                continue;
            }
            let portrait_rect = egui::Rect::from_center_size(
                egui::pos2(frame_rect.min.x + geometry.slot_x, cy),
                egui::vec2(NATIVE_PORTRAIT_SIZE, NATIVE_PORTRAIT_SIZE),
            );
            let id = slot_id(ui, i, fighter);
            let selectable = marks.is_some() && !fighter.last_turn_casts.is_empty();
            let sense = if selectable {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            };
            let response = ui.interact(portrait_rect, id, sense);
            if selectable {
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, fighter.name.as_str())
                });
                if response.clicked() {
                    clicked = Some(i);
                }
                response
                    .clone()
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
            }
            crate::design::tooltip(&response).text(fighter.name.as_str());
            let measured = metric.value_of(fighter);
            if measured > 0 {
                crate::design::paint_portrait_percent(
                    ui,
                    portrait_rect,
                    crate::design::portrait_percent(measured, total_damage),
                );
            }
        }

        // Scrollbar : fine, collée au bord gauche, hauteur dynamique — dessin et interaction
        // partagés avec la fenêtre des barres (`combat_scrollbar`, 13 sept. 2026), position
        // normalisée convertie depuis/vers le décalage en emplacements.
        let scrollbar = DrawnScrollbar {
            band: clip_rect,
            hit_left: frame_rect.min.x,
            thumb_left: frame_rect.min.x,
            visible_frac: MAX_FRAME_SLOTS as f32 / n as f32,
        };
        let position = scrollbar.show(ui, ui.id().with("enemy-scroll-hit"), offset / max_offset);
        offset = (position * max_offset).clamp(0.0, max_offset);

        ui.ctx().data_mut(|d| d.insert_temp(scroll_id, offset));
        clicked
    }
}
