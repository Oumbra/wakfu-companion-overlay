//! **Glyphe seul** du design system — une icône posée dans le flux, sans socle ni bouton.
//! Composant **feuille** (§1 du contrat).
//!
//! ```ignore
//! use overlay_ui::design::{self, DsIcon};
//!
//! ui.add(design::icon(DsIcon::Kamas));                       // 16 px, la taille par défaut
//! ui.add(design::icon(DsIcon::Trophy).size(24.0));
//! ui.add(design::icon(DsIcon::Lock).tint(tokens::TEXT_DISABLED));
//! ```
//!
//! ## Ce qu'il fait, et ce que fait `icon_button`
//!
//! [`design::icon_button`](super::icon_button) peint un glyphe **sur un socle**, avec les états et
//! la mécanique de clic qui vont avec. Ce composant-ci ne peint que le glyphe : une icône dans une
//! ligne de texte, dans un en-tête de colonne, à côté d'un compteur. Il n'est pas cliquable —
//! l'appelant qui veut un clic prend `icon_button`, qui a le socle que ce clic mérite.
//!
//! ## Le rapport d'aspect est préservé, et ce n'est pas un détail
//!
//! Les glyphes du jeu ne sont pas carrés : le chevron fait 14 × 8, la pastille d'info 27 × 28. Ce
//! composant réutilise [`glyph_fit`](super::icon_button::glyph_fit) — le **seul** endroit du crate
//! qui calcule ce rapport — plutôt que d'inscrire le glyphe dans un `Vec2::splat`. C'est
//! exactement le défaut qu'`Input::leading_icon` portait jusqu'au 2026-09-10, invisible tant que la
//! loupe (24 × 24) était le seul glyphe passé.
//!
//! `size` donne donc le **côté du carré englobant**, pas la largeur : un chevron de 14 × 8 demandé
//! à 16 px sera peint 16 × 9.
//!
//! ## L'étalon d'encre ne s'applique pas ici
//!
//! [`DsIcon::content_size`] normalise la taille d'encre d'un glyphe détouré depuis un socle de
//! bouton, pour que deux glyphes voisins sur deux boutons paraissent de la même taille. Hors socle,
//! il n'y a pas de voisin à accorder : le glyphe occupe le carré qu'on lui donne. Un appelant qui
//! veut l'accord d'un bouton utilise `icon_button`, qui le fait.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::design::{tokens, DesignSystem, DsIcon};

/// Construit un glyphe à [`tokens::ICON_SIZE`].
pub fn icon(icon: DsIcon) -> Icon {
    Icon {
        icon,
        size: tokens::ICON_SIZE,
        tint: tokens::ICON_TINT,
    }
}

/// Voir [`icon`].
pub struct Icon {
    icon: DsIcon,
    size: f32,
    tint: egui::Color32,
}

impl Icon {
    /// Côté du carré englobant. Le glyphe y est inscrit **en gardant son rapport** — voir la doc de
    /// module.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Teinte. Les glyphes sont blancs dans leurs fichiers : la couleur se met au rendu, et c'est
    /// ce qui évite un fichier par teinte.
    pub fn tint(mut self, tint: egui::Color32) -> Self {
        self.tint = tint;
        self
    }
}

impl Widget for Icon {
    fn ui(self, ui: &mut Ui) -> Response {
        let ds = DesignSystem::get(ui.ctx());
        let native = ds.icon_native_size(self.icon);
        let drawn = super::icon_button::glyph_fit(native, self.size);
        // **Le carré est alloué, le glyphe y est centré.** Réserver la taille dessinée ferait
        // sauter la ligne d'un glyphe à l'autre — un chevron plat et une pastille haute n'auraient
        // pas la même hauteur de ligne, alors que l'appelant a demandé la même taille aux deux.
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(self.size), Sense::hover());
        if ui.is_rect_visible(rect) {
            ds.paint_icon(
                ui.painter(),
                egui::Rect::from_center_size(rect.center(), drawn),
                self.icon,
                self.tint,
            );
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::super::icon_button::glyph_fit;
    use egui::Vec2;

    #[test]
    fn un_glyphe_non_carre_garde_son_rapport() {
        // Le chevron du jeu fait 14 × 8. Demandé à 16, il doit sortir 16 × 9,14 — pas 16 × 16,
        // qui l'écraserait, ni 14 × 8, qui ignorerait la taille demandée.
        let drawn = glyph_fit(Vec2::new(14.0, 8.0), 16.0);
        assert!((drawn.x - 16.0).abs() < 1e-4, "largeur : {drawn:?}");
        assert!(
            (drawn.y - 16.0 * 8.0 / 14.0).abs() < 1e-4,
            "hauteur : {drawn:?}"
        );
        assert!(
            (drawn.x / drawn.y - 14.0 / 8.0).abs() < 1e-4,
            "le rapport d'origine doit être conservé",
        );
    }

    #[test]
    fn c_est_la_plus_grande_dimension_qui_touche_le_bord() {
        // Un glyphe plus haut que large : c'est la HAUTEUR qui vaut la taille demandée. Diviser
        // systématiquement par la largeur ferait déborder du carré alloué tout glyphe vertical.
        let drawn = glyph_fit(Vec2::new(8.0, 14.0), 16.0);
        assert!((drawn.y - 16.0).abs() < 1e-4, "hauteur : {drawn:?}");
        assert!(drawn.x <= 16.0 + 1e-4, "le glyphe doit tenir dans le carré");
    }

    #[test]
    fn un_glyphe_carre_remplit_le_carre() {
        let drawn = glyph_fit(Vec2::splat(24.0), 32.0);
        assert!((drawn.x - 32.0).abs() < 1e-4 && (drawn.y - 32.0).abs() < 1e-4);
    }
}
