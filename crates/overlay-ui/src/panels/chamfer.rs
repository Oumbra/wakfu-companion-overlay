//! Primitive de dessin du design system Wakfu (`docs/design-system.md` §1.1/§6.2) : rectangle à
//! coins chanfreinés (coupés en diagonale, silhouette octogonale — JAMAIS d'arrondi circulaire)
//! avec dégradé vertical clair→sombre, tel qu'utilisé sur pratiquement tous les éléments
//! « boutons/bannières » du jeu réel. Implémente l'option (a) déjà esquissée par
//! `docs/design-system.md` §6.2 : un `egui::Mesh` peint à la main, avec un sommet coloré par
//! sommet (interpolé par le rasteriseur egui) plutôt qu'une texture — suffisant pour un dégradé
//! purement VERTICAL comme celui mesuré sur toutes les références (bannière turquoise, boutons
//! or/kaki/rouge — voir §2/§9 du design-system).
//!
//! Premier composant à réutiliser ce dessin à la main : `panels::options_modal` (bannière + pied de
//! page Annuler/Valider de la modale Options). Volontairement générique (pas de dépendance à
//! `options_modal`) pour qu'un futur composant du design system (autre fenêtre modale, bouton
//! plein-largeur…) puisse le réutiliser tel quel.

/// Un coin chanfreiné par bouléen, dans l'ordre haut-gauche/haut-droit/bas-droit/bas-gauche —
/// même ordre que la convention CSS `border-radius` à 4 valeurs, pour rester lisible sans avoir à
/// se référer sans cesse à cette doc.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChamferCorners {
    pub top_left: bool,
    pub top_right: bool,
    pub bottom_right: bool,
    pub bottom_left: bool,
}

impl ChamferCorners {
    /// Seuls les deux coins du HAUT chanfreinés — bannière de fenêtre (§5.10 du design-system).
    pub const TOP: Self = Self {
        top_left: true,
        top_right: true,
        bottom_right: false,
        bottom_left: false,
    };
    /// Seul le coin bas-gauche — moitié GAUCHE d'un pied de page scindé (bouton "Annuler", voir
    /// `panels::options_modal`).
    pub const BOTTOM_LEFT: Self = Self {
        top_left: false,
        top_right: false,
        bottom_right: false,
        bottom_left: true,
    };
    /// Seul le coin bas-droit — moitié DROITE d'un pied de page scindé (bouton "Valider").
    pub const BOTTOM_RIGHT: Self = Self {
        top_left: false,
        top_right: false,
        bottom_right: true,
        bottom_left: false,
    };
}

/// Peint un rectangle à coins chanfreinés (voir doc de module) rempli d'un dégradé vertical
/// `color_top` → `color_bottom`, avec un contour optionnel de `border` (couleur quasi noire dans
/// toutes les références mesurées — voir §4/§9 du design-system, `button_border_width`/
/// `[accent_danger|banner_teal].border`).
///
/// `chamfer` est la longueur (en points egui, PAS en pixels physiques) de la coupe d'angle —
/// `docs/design-tokens.json::dimensions_px.button_chamfer_cut` donne 8-10px comme repère mesuré
/// sur les références du jeu.
pub fn chamfered_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    chamfer: f32,
    corners: ChamferCorners,
    color_top: egui::Color32,
    color_bottom: egui::Color32,
    border: Option<egui::Color32>,
) {
    // Chanfrein plafonné à la moitié du plus petit côté — un `chamfer` mal réglé (rect trop petit,
    // ex. bouton compact) ne doit jamais produire un polygone auto-intersectant.
    let chamfer = chamfer
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0)
        .max(0.0);

    let points = polygon_points(rect, chamfer, corners);

    let color_at = |y: f32| -> egui::Color32 {
        let t = if rect.height() > 0.0 {
            ((y - rect.top()) / rect.height()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        lerp_color(color_top, color_bottom, t)
    };

    let mut mesh = egui::Mesh::default();
    let center = rect.center();
    let center_idx = mesh.vertices.len() as u32;
    mesh.colored_vertex(center, color_at(center.y));
    for p in &points {
        mesh.colored_vertex(*p, color_at(p.y));
    }
    let n = points.len() as u32;
    for i in 0..n {
        let a = 1 + i;
        let b = 1 + (i + 1) % n;
        mesh.add_triangle(center_idx, a, b);
    }
    painter.add(mesh);

    if let Some(border) = border {
        painter.add(egui::Shape::closed_line(
            points,
            egui::Stroke::new(1.5, border),
        ));
    }
}

/// Sommets du polygone chanfreiné, dans le sens horaire en partant du bord gauche de la coupe
/// haut-gauche (ou du coin haut-gauche lui-même si non chanfreiné) — ordre qui donne un
/// `closed_line` propre pour le contour ET un éventail (fan) valide depuis le centre pour le
/// remplissage (`chamfered_rect`), un rectangle chanfreiné étant toujours convexe.
fn polygon_points(rect: egui::Rect, chamfer: f32, corners: ChamferCorners) -> Vec<egui::Pos2> {
    let egui::Rect { min, max } = rect;
    let mut points = Vec::with_capacity(8);

    // Haut-gauche
    if corners.top_left {
        points.push(egui::pos2(min.x, min.y + chamfer));
        points.push(egui::pos2(min.x + chamfer, min.y));
    } else {
        points.push(egui::pos2(min.x, min.y));
    }
    // Haut-droit
    if corners.top_right {
        points.push(egui::pos2(max.x - chamfer, min.y));
        points.push(egui::pos2(max.x, min.y + chamfer));
    } else {
        points.push(egui::pos2(max.x, min.y));
    }
    // Bas-droit
    if corners.bottom_right {
        points.push(egui::pos2(max.x, max.y - chamfer));
        points.push(egui::pos2(max.x - chamfer, max.y));
    } else {
        points.push(egui::pos2(max.x, max.y));
    }
    // Bas-gauche
    if corners.bottom_left {
        points.push(egui::pos2(min.x + chamfer, max.y));
        points.push(egui::pos2(min.x, max.y - chamfer));
    } else {
        points.push(egui::pos2(min.x, max.y));
    }

    points
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let lerp = |x: u8, y: u8| -> u8 { (x as f32 + (y as f32 - x as f32) * t).round() as u8 };
    egui::Color32::from_rgba_unmultiplied(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
        lerp(a.a(), b.a()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polygon_sans_chanfrein_est_le_rectangle_lui_meme() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(10.0, 20.0));
        let points = polygon_points(rect, 3.0, ChamferCorners::default());
        assert_eq!(
            points,
            vec![
                egui::pos2(0.0, 0.0),
                egui::pos2(10.0, 0.0),
                egui::pos2(10.0, 20.0),
                egui::pos2(0.0, 20.0),
            ]
        );
    }

    #[test]
    fn polygon_chanfrein_haut_ajoute_deux_sommets_par_coin_coupe() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(10.0, 20.0));
        let points = polygon_points(rect, 3.0, ChamferCorners::TOP);
        // 2 coins chanfreinés (2 sommets chacun) + 2 coins droits (1 sommet chacun) = 6.
        assert_eq!(points.len(), 6);
    }

    #[test]
    fn lerp_color_aux_bornes_renvoie_les_couleurs_d_origine() {
        let top = egui::Color32::from_rgb(0x1A, 0x6E, 0x80);
        let bottom = egui::Color32::from_rgb(0x17, 0x63, 0x72);
        assert_eq!(lerp_color(top, bottom, 0.0), top);
        assert_eq!(lerp_color(top, bottom, 1.0), bottom);
    }
}
