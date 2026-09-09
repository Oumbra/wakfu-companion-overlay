//! 9-slice (« scale9 ») — étire une texture asset dans un rectangle cible sans déformer ses coins
//! ni sa bordure : les quatre coins gardent leur taille NATIVE (un pixel de texture = un point
//! egui, jamais redimensionné), les quatre bandeaux s'étirent dans leur seule direction, le
//! centre s'étire dans les deux. C'est la même technique que `.claude/skills/design-asset`
//! (commandes `resize`/`align`, voir sa doc) applique déjà aux fichiers PNG eux-mêmes — utile ici
//! côté rendu pour agrandir une texture À LA VOLÉE à une taille non prévue à l'export (ex. le
//! bouton "Sélectionner le fichier" de `panels::options_modal`, bien plus large que
//! `assets/ui/options/browse-button.png` d'origine).
//!
//! Un simple redimensionnement (`egui::Image` étiré uniformément) suffit pour la plupart des
//! textures du design system Wakfu, dont le rayon de coin mesuré est petit (2-4px, voir
//! `.claude/skills/design-asset/references/recettes-composants.md`) — la déformation reste
//! imperceptible. Ce module n'est nécessaire QUE quand l'écart entre taille native et taille
//! cible est assez grand pour que la déformation d'un coin ou d'une bordure saute aux yeux.

/// Peint `texture` en 9-slice dans `rect` — voir la doc de module. `inset` (même valeur sur les
/// quatre côtés) doit dépasser le rayon de coin ET l'épaisseur de bordure NATIFS de la texture
/// (mesurés via `dsimg.py analyze`, voir chaque appelant pour la valeur retenue) : un `inset` trop
/// petit couperait la bordure au milieu de sa courbe, trop grand rétrécirait inutilement la zone
/// étirable jusqu'à l'annuler (plafonné ci-dessous à la moitié de la texture dans ce cas).
pub fn nine_slice(
    painter: &egui::Painter,
    texture: &egui::TextureHandle,
    rect: egui::Rect,
    inset: f32,
) {
    let id = texture.id();
    for (dest, uv) in nine_slice_cells(rect, texture.size_vec2(), inset) {
        painter.image(id, dest, uv, egui::Color32::WHITE);
    }
}

/// Calcule les paires (rectangle destination en points, rectangle UV 0..1) des cellules à peindre
/// — extrait de [`nine_slice`] pour rester testable sans `egui::Context` réel (aucun
/// `TextureHandle` mockable autrement, voir la doc du module `tests` plus bas). Une cellule de
/// largeur/hauteur nulle (`rect` cible plus petit que `2 × inset` sur un axe) est OMISE plutôt que
/// renvoyée avec une taille nulle — arrive en pratique si l'appelant demande une taille très
/// compacte, jamais une erreur en soi.
fn nine_slice_cells(
    rect: egui::Rect,
    tex_size: egui::Vec2,
    inset: f32,
) -> Vec<(egui::Rect, egui::Rect)> {
    let inset = inset.min(tex_size.x / 2.0).min(tex_size.y / 2.0).max(0.0);

    // Bornes UV (0..1, repère texture) et bornes destination (points egui, repère écran) sur
    // chaque axe : [bord de départ, fin du coin, début du coin opposé, bord de fin].
    let u = [0.0, inset / tex_size.x, 1.0 - inset / tex_size.x, 1.0];
    let v = [0.0, inset / tex_size.y, 1.0 - inset / tex_size.y, 1.0];
    let x = [
        rect.left(),
        rect.left() + inset,
        rect.right() - inset,
        rect.right(),
    ];
    let y = [
        rect.top(),
        rect.top() + inset,
        rect.bottom() - inset,
        rect.bottom(),
    ];

    let mut cells = Vec::with_capacity(9);
    for row in 0..3 {
        for col in 0..3 {
            let dest = egui::Rect::from_min_max(
                egui::pos2(x[col], y[row]),
                egui::pos2(x[col + 1], y[row + 1]),
            );
            if dest.width() <= 0.0 || dest.height() <= 0.0 {
                continue;
            }
            let uv = egui::Rect::from_min_max(
                egui::pos2(u[col], v[row]),
                egui::pos2(u[col + 1], v[row + 1]),
            );
            cells.push((dest, uv));
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neuf_cellules_quand_le_rect_cible_est_assez_grand() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 40.0));
        let cells = nine_slice_cells(rect, egui::vec2(40.0, 20.0), 8.0);
        assert_eq!(cells.len(), 9);
    }

    #[test]
    fn coin_haut_gauche_garde_la_taille_native_de_l_inset() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(100.0, 40.0));
        let cells = nine_slice_cells(rect, egui::vec2(40.0, 20.0), 8.0);
        let (dest, uv) = cells[0]; // rangée 0, colonne 0 = coin haut-gauche
        assert_eq!(
            dest,
            egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(8.0, 8.0))
        );
        assert_eq!(
            uv,
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(0.2, 0.4))
        );
    }

    #[test]
    fn centre_couvre_toute_la_zone_etirable_restante() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 40.0));
        let cells = nine_slice_cells(rect, egui::vec2(40.0, 20.0), 8.0);
        let center = cells[4]; // rangée 1, colonne 1
        assert_eq!(
            center.0,
            egui::Rect::from_min_max(egui::pos2(8.0, 8.0), egui::pos2(92.0, 32.0))
        );
    }

    #[test]
    fn inset_plafonne_a_la_moitie_de_la_texture_sans_paniquer() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(50.0, 20.0));
        // inset (100) largement plus grand que la texture (40x20) : ne doit ni paniquer ni
        // produire un UV hors de [0,1] (plafonné à 10.0 = 20.0/2.0, le plus petit des deux axes).
        let cells = nine_slice_cells(rect, egui::vec2(40.0, 20.0), 100.0);
        for (_, uv) in &cells {
            assert!((0.0..=1.0).contains(&uv.min.x) && (0.0..=1.0).contains(&uv.max.x));
            assert!((0.0..=1.0).contains(&uv.min.y) && (0.0..=1.0).contains(&uv.max.y));
        }
    }

    #[test]
    fn rect_cible_plus_petit_que_deux_fois_l_inset_omet_les_cellules_nulles() {
        // Sur l'axe X, 2×inset (16) dépasse la largeur cible (10) : la colonne centrale a une
        // largeur négative — omise plutôt que peinte avec une taille nulle/négative.
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 40.0));
        let cells = nine_slice_cells(rect, egui::vec2(40.0, 20.0), 8.0);
        assert!(cells.len() < 9);
    }
}
