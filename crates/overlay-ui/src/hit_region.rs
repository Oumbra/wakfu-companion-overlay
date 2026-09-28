//! **Où une fenêtre overlay capte les clics** (2026-09-28, demande utilisateur) — partout où elle
//! peint quelque chose, sauf ses infobulles, et nulle part ailleurs.
//!
//! Demande, presque mot pour mot : « avec l'overlay du suivi, il est impossible de cliquer en
//! dessous, là où on affiche la tooltip [...] que l'endroit où sont affichées les tooltips soit
//! cliquable, que ce ne soit pas de l'overlay ».
//!
//! ## Pourquoi il faut le calculer
//!
//! Une infobulle egui ne s'affiche qu'à l'intérieur de sa fenêtre OS : chaque panneau réserve donc
//! de la place transparente autour de son contenu (`render_content::WATCHLIST_TOOLTIP_RESERVE`,
//! `RECAP_TOOLTIP_RESERVE`, les réserves latérales du carré de contrôle…). Or une fenêtre capte
//! les clics sur **toute** sa surface, transparence comprise : `Window::set_cursor_hittest` est un
//! interrupteur par fenêtre, pas par pixel. Ces réserves étaient autant de bandes où le clic
//! n'atteignait plus le jeu.
//!
//! ## Comment
//!
//! À chaque frame, [`collect`] relève les rectangles de tout ce qui a été peint — couche par
//! couche, **sauf les couches d'infobulle** (`Order::Tooltip`) et ce qui est entièrement
//! transparent. L'hôte lit ensuite le curseur d'écran à chaque tick (20 Hz, le sondage qui existe
//! déjà pour les fenêtres de jeu) et n'active `set_cursor_hittest` que quand il survole l'un de
//! ces rectangles ([`wants_hit_test`]). Ailleurs — réserves, écarts entre les tuiles, infobulles —
//! la fenêtre laisse passer le clic au jeu.
//!
//! Deux garde-fous :
//!
//! - **Un geste commencé n'est jamais coupé** : tant qu'un bouton de la souris est enfoncé ou
//!   qu'un glisser est en cours ([`HitRegion::busy`]), la fenêtre reste cliquable même si le
//!   curseur sort de son contenu — sans quoi déplacer le Suivi par sa poignée s'arrêterait au
//!   premier pixel hors de la pastille.
//! - **egui est prévenu** du changement, puisqu'une fenêtre traversante ne reçoit plus aucun
//!   événement souris : l'hôte lui envoie le pointeur à l'activation (le survol s'affiche sans
//!   attendre un nouveau mouvement) et `PointerGone` à la désactivation (une infobulle ne reste
//!   pas ouverte sur un survol que la fenêtre ne voit plus).
//!
//! Tout est en **points logiques relatifs à la fenêtre** : c'est le repère d'egui ; l'hôte y
//! ramène le curseur d'écran, lui seul connaissant la position et l'échelle de sa fenêtre.

use egui::epaint::{ClippedShape, Shape};
use egui::{LayerId, Order, Pos2, Rect};

/// Les rectangles peints d'une frame, et l'état du geste en cours — voir la doc de module.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HitRegion {
    rects: Vec<Rect>,
    /// Un bouton de la souris enfoncé ou un glisser en cours à la fin de la frame : la fenêtre
    /// doit rester cliquable jusqu'au relâchement.
    pub busy: bool,
}

impl HitRegion {
    /// Le point (points logiques, relatif à la fenêtre) tombe-t-il sur du contenu peint ?
    pub fn contains(&self, pos: Pos2) -> bool {
        self.rects.iter().any(|rect| rect.contains(pos))
    }

    /// Les rectangles relevés, en points logiques relatifs à la fenêtre.
    pub fn rects(&self) -> &[Rect] {
        &self.rects
    }

    /// Rien de peint — une fenêtre qui n'a pas encore rendu sa première frame, typiquement.
    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }
}

/// Relève la région cliquable de la frame en cours. Seules les couches des zones egui comptent
/// (`Area`, panneaux, popups) : un peintre posé sur une couche jamais déclarée — la gerbe de
/// confettis d'un suivi abouti — reste un décor que le clic traverse. À appeler **à la fin** de la passe, une fois
/// tout peint (voir `render_content::build_ui`) : les couches sont vidées au moment où egui
/// produit sa sortie.
pub fn collect(ctx: &egui::Context) -> HitRegion {
    let mut layers: Vec<LayerId> = ctx.memory(|mem| mem.layer_ids().collect());
    // Le panneau racine peint sur la couche de fond, que l'ordre des zones ne liste pas.
    if !layers.contains(&LayerId::background()) {
        layers.push(LayerId::background());
    }
    // Lues AVANT d'entrer dans `graphics` : le contexte y est verrouillé, et le relire dedans
    // bloquerait.
    let transforms: Vec<_> = layers
        .iter()
        .map(|layer| ctx.layer_transform_to_global(*layer))
        .collect();
    let mut rects = Vec::new();
    ctx.graphics(|graphics| {
        for (layer, transform) in layers.iter().zip(transforms) {
            if matches!(layer.order, Order::Tooltip | Order::Debug) {
                continue;
            }
            let Some(list) = graphics.get(*layer) else {
                continue;
            };
            for ClippedShape { clip_rect, shape } in list.all_entries() {
                if !is_visible(shape) {
                    continue;
                }
                let mut rect = shape.visual_bounding_rect().intersect(*clip_rect);
                if let Some(transform) = transform {
                    rect = transform * rect;
                }
                if rect.is_positive() {
                    rects.push(rect);
                }
            }
        }
    });
    let busy = ctx.input(|input| input.pointer.any_down()) || ctx.dragged_id().is_some();
    HitRegion { rects, busy }
}

/// Une forme laisse-t-elle une trace à l'écran ? Seuls les rectangles et les tracés sans
/// remplissage ni contour sont écartés — egui en produit pour les cadres transparents, et ce sont
/// eux qui couvriraient la fenêtre entière. Dans le doute (maillage, texte…), la forme compte.
fn is_visible(shape: &Shape) -> bool {
    match shape {
        Shape::Noop => false,
        Shape::Vec(shapes) => shapes.iter().any(is_visible),
        Shape::Rect(rect) => {
            rect.fill.a() > 0
                || (rect.stroke.width > 0.0 && rect.stroke.color.a() > 0)
                || rect.brush.is_some()
        }
        Shape::Path(path) => {
            let stroke = match &path.stroke.color {
                egui::epaint::ColorMode::Solid(color) => color.a() > 0,
                egui::epaint::ColorMode::UV(_) => true,
            };
            path.fill.a() > 0 || (path.stroke.width > 0.0 && stroke)
        }
        _ => true,
    }
}

/// La fenêtre doit-elle capter les clics à ce tick ? `cursor` est le curseur en points logiques
/// relatifs à la fenêtre (`None` hors écran ou illisible).
///
/// Un geste en cours la garde cliquable ([`HitRegion::busy`]) ; une région encore vide — aucune
/// frame rendue — aussi, pour ne jamais rendre inerte une fenêtre qu'on n'a pas encore vue.
pub fn wants_hit_test(region: &HitRegion, cursor: Option<Pos2>) -> bool {
    region.busy || region.is_empty() || cursor.is_some_and(|pos| region.contains(pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une frame egui minimale : un carré peint au fond, une zone transparente à côté, et une
    /// infobulle ouverte en dessous du carré.
    fn frame(paint: impl FnMut(&mut egui::Ui)) -> HitRegion {
        let ctx = egui::Context::default();
        let mut region = HitRegion::default();
        let mut paint = paint;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(300.0, 200.0))),
            ..Default::default()
        };
        // Deux passes : une zone egui est invisible à sa toute première (passe de mesure).
        for _ in 0..2 {
            let mut output = ctx.run_ui(input.clone(), |ui| {
                paint(ui);
                region = collect(ui.ctx());
            });
            output.textures_delta.clear();
        }
        region
    }

    #[test]
    fn le_contenu_peint_capte_le_clic_et_le_vide_le_laisse_passer() {
        let region = frame(|ui| {
            ui.painter().rect_filled(
                Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(40.0, 40.0)),
                0.0,
                egui::Color32::RED,
            );
        });
        assert!(region.contains(egui::pos2(30.0, 30.0)));
        assert!(!region.contains(egui::pos2(150.0, 150.0)));
    }

    #[test]
    fn un_cadre_transparent_ne_couvre_pas_la_fenetre() {
        let region = frame(|ui| {
            ui.painter()
                .rect_filled(ui.max_rect(), 0.0, egui::Color32::TRANSPARENT);
            ui.painter().rect_filled(
                Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(20.0, 20.0)),
                0.0,
                egui::Color32::WHITE,
            );
        });
        assert!(!region.contains(egui::pos2(150.0, 150.0)));
        assert!(region.contains(egui::pos2(15.0, 15.0)));
    }

    #[test]
    fn une_infobulle_laisse_passer_le_clic() {
        let region = frame(|ui| {
            for (order, name, pos) in [
                (Order::Tooltip, "infobulle", egui::pos2(100.0, 100.0)),
                (Order::Foreground, "menu", egui::pos2(200.0, 10.0)),
            ] {
                egui::Area::new(egui::Id::new(name))
                    .order(order)
                    .fixed_pos(pos)
                    .show(ui.ctx(), |ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(40.0, 30.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
                    });
            }
        });
        assert!(!region.contains(egui::pos2(120.0, 110.0)), "infobulle");
        assert!(region.contains(egui::pos2(210.0, 20.0)), "menu déroulant");
    }

    #[test]
    fn un_geste_en_cours_garde_la_fenetre_cliquable() {
        let region = HitRegion {
            rects: vec![Rect::from_min_size(Pos2::ZERO, egui::vec2(10.0, 10.0))],
            busy: true,
        };
        assert!(wants_hit_test(&region, Some(egui::pos2(100.0, 100.0))));
        assert!(wants_hit_test(&region, None));
    }

    #[test]
    fn hors_du_contenu_la_fenetre_laisse_passer_le_clic() {
        let region = HitRegion {
            rects: vec![Rect::from_min_size(Pos2::ZERO, egui::vec2(10.0, 10.0))],
            busy: false,
        };
        assert!(wants_hit_test(&region, Some(egui::pos2(5.0, 5.0))));
        assert!(!wants_hit_test(&region, Some(egui::pos2(100.0, 100.0))));
        assert!(!wants_hit_test(&region, None));
    }

    #[test]
    fn une_fenetre_jamais_rendue_reste_cliquable() {
        assert!(wants_hit_test(&HitRegion::default(), None));
    }
}
