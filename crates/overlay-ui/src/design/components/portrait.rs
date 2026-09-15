//! **Portrait de combattant** — le médaillon carré ou rond d'un allié ou d'un ennemi, son grisé
//! quand il est KO, et le pourcentage de dégâts incrusté dans son coin. Composant **feuille**
//! (§1 du contrat).
//!
//! ```ignore
//! use overlay_ui::design::{self, PortraitShape};
//!
//! let r = ui.add(
//!     design::portrait(texture_id)
//!         .shape(PortraitShape::Round)
//!         .size(48.0)
//!         .dimmed(fighter.is_ko)
//!         .percent(Some(42)),
//! );
//!
//! // Pour un appelant qui pose déjà sa géométrie — le gabarit à six emplacements :
//! design::paint_portrait(ui, rect, texture_id, PortraitShape::Round, false);
//! design::paint_portrait_percent(ui, rect, 42);
//! ```
//!
//! ## Deux formes, parce que le jeu en a deux
//!
//! Le gabarit de combat loge ses portraits dans des **médaillons ronds** ; la liste plate, celle
//! qui prend le relais au-delà de six alliés, les pose **carrés**. Ce n'est pas une préférence
//! d'affichage mais la forme des cadres du jeu, et [`PortraitShape`] la porte plutôt que de laisser
//! chaque appelant calculer son rayon — `taille / 2` écrit à deux endroits finit par diverger d'un
//! pixel.
//!
//! ## Le grisé KO est une approximation, et l'appelant décide
//!
//! [`Portrait::dimmed`] applique [`tokens::PORTRAIT_KO_TINT`], un gris moyen. Une teinte egui
//! **multiplie** : elle assombrit sans désaturer, là où un vrai niveau de gris désature. Les
//! portraits de classe ont leur version grise **précalculée** dans l'atlas et n'ont donc pas besoin
//! de cette teinte — la leur serait moins bonne. Une icône de monstre téléchargée ou le repli
//! générique, eux, n'ont pas d'équivalent gris et s'en contentent.
//!
//! Le composant ne peut pas trancher : seul l'appelant sait laquelle des trois textures il tient.
//! D'où un paramètre plutôt qu'une déduction depuis un `is_ko` que le composant ne verrait pas.
//!
//! ## Le pourcentage déborde du carré, volontairement
//!
//! Il se pose au coin bas-droit du **carré englobant**, décalé encore de
//! [`tokens::PORTRAIT_PERCENT_OFFSET`] vers l'extérieur : « comme si on traçait un carré autour du
//! rond et qu'on plaçait le pourcentage tout en bas à droite ». Sur un portrait rond, ce coin est
//! hors du disque — c'est précisément ce qu'on veut, ne jamais recouvrir le visage.
//!
//! Il prend [`tokens::OVERLAY_ACCENT`] et **non la teinte de la jauge**, après un aller-retour :
//! les deux ont été alignées un temps, puis re-séparées (« je préfère la couleur accent qu'il y
//! avait avant »). Barre et pourcentage n'ont donc pas la même couleur, et c'est voulu.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::design::{text, tokens};

/// Forme du médaillon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PortraitShape {
    /// Carré — la liste plate, au-delà de six alliés.
    #[default]
    Square,
    /// Rond — les emplacements du gabarit de combat.
    Round,
}

impl PortraitShape {
    /// Rayon d'arrondi pour un médaillon de ce côté.
    ///
    /// Fonction plutôt que constante : le rayon d'un rond **dépend de la taille**, et l'écrire à la
    /// main à chaque appel est le meilleur moyen d'obtenir deux portraits ronds qui ne le sont pas
    /// tout à fait.
    pub fn corner_radius(self, size: f32) -> u8 {
        match self {
            PortraitShape::Square => 0,
            PortraitShape::Round => (size / 2.0) as u8,
        }
    }
}

/// Construit un portrait sur une texture déjà résolue.
///
/// La texture arrive en [`egui::TextureId`] pour la même raison que dans
/// [`design::item_slot`](super::item_slot) : un portrait de combattant est du **contenu** — atlas de
/// classes, icône téléchargée ou repli — que le design system ne saurait pas nommer.
pub fn portrait(texture: egui::TextureId) -> Portrait {
    Portrait {
        texture,
        shape: PortraitShape::default(),
        size: 40.0,
        dimmed: false,
        percent: None,
    }
}

/// Voir [`portrait`].
pub struct Portrait {
    texture: egui::TextureId,
    shape: PortraitShape,
    size: f32,
    dimmed: bool,
    percent: Option<i64>,
}

impl Portrait {
    /// Forme du médaillon. Par défaut [`PortraitShape::Square`].
    pub fn shape(mut self, shape: PortraitShape) -> Self {
        self.shape = shape;
        self
    }

    /// Côté du carré englobant. Par défaut 40 px, la taille de la liste plate.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Applique le grisé KO — **seulement si la texture n'a pas de version grise précalculée**,
    /// voir la doc de module.
    pub fn dimmed(mut self, dimmed: bool) -> Self {
        self.dimmed = dimmed;
        self
    }

    /// Pourcentage incrusté au coin bas-droit. `None` n'en peint aucun.
    pub fn percent(mut self, percent: Option<i64>) -> Self {
        self.percent = percent;
        self
    }
}

impl Widget for Portrait {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(self.size), Sense::hover());
        if ui.is_rect_visible(rect) {
            paint(ui, rect, self.texture, self.shape, self.dimmed);
            if let Some(percent) = self.percent {
                paint_percent(ui, rect, percent);
            }
        }
        response
    }
}

/// Peint un portrait dans `rect`, sans rien allouer.
///
/// Séparée du `Widget` pour le gabarit de combat, qui pose ses six emplacements à des centres
/// relevés sur le template et n'a donc rien à faire de la mise en page d'egui.
pub fn paint(
    ui: &Ui,
    rect: egui::Rect,
    texture: egui::TextureId,
    shape: PortraitShape,
    dimmed: bool,
) {
    let tint = if dimmed {
        tokens::PORTRAIT_KO_TINT
    } else {
        egui::Color32::WHITE
    };
    egui::Image::new(egui::load::SizedTexture::new(texture, rect.size()))
        .corner_radius(shape.corner_radius(rect.width()))
        // `maintain_aspect_ratio(false)` : le rect donné fait foi. Les portraits de classe sont
        // carrés, mais une icône de monstre téléchargée ne l'est pas toujours — la laisser garder
        // ses proportions la ferait déborder du médaillon ou flotter dedans.
        .maintain_aspect_ratio(false)
        .tint(tint)
        .paint_at(ui, rect);
}

/// Peint le pourcentage au coin bas-droit du carré englobant `rect`.
///
/// Publique et séparée parce que le gabarit de combat la rappelle **après** avoir peint ses six
/// portraits : un texte masqué par le portrait suivant ne servirait à rien.
pub fn paint_percent(ui: &Ui, rect: egui::Rect, percent: i64) {
    text::paint_outlined_text(
        ui,
        rect.right_bottom() + tokens::PORTRAIT_PERCENT_OFFSET,
        egui::Align2::RIGHT_BOTTOM,
        &format!("{percent}%"),
        text::label_font(ui.ctx(), tokens::PORTRAIT_PERCENT_FONT_SIZE),
        tokens::OVERLAY_ACCENT,
        text::OUTLINE_FULL,
    );
}

/// Part de `damage` dans `total`, en pourcentage entier borné à 0..=100.
///
/// Fonction libre et testée : un total nul est le cas réel du tout début d'un combat, et une
/// division par zéro y produirait un `NaN` qui se propagerait jusqu'au texte peint — « NaN% » sur
/// un portrait.
pub fn percent_of(damage: i64, total: i64) -> i64 {
    if total <= 0 {
        return 0;
    }
    let ratio = (damage as f32 / total as f32).clamp(0.0, 1.0);
    (ratio as f64 * 100.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_medaillon_rond_a_le_rayon_de_son_demi_cote() {
        assert_eq!(PortraitShape::Round.corner_radius(48.0), 24);
        assert_eq!(PortraitShape::Round.corner_radius(40.0), 20);
        assert_eq!(PortraitShape::Square.corner_radius(48.0), 0);
    }

    #[test]
    fn un_total_nul_ne_produit_pas_de_nan() {
        // Le cas réel du tout début d'un combat. Sans ce garde-fou, un `NaN` remonte jusqu'au
        // texte peint et le portrait affiche « NaN% ».
        assert_eq!(percent_of(0, 0), 0);
        assert_eq!(percent_of(500, 0), 0);
        assert_eq!(percent_of(500, -1), 0);
    }

    #[test]
    fn le_pourcentage_est_borne_a_cent() {
        // Un combattant peut porter plus de dégâts que le total affiché : le camp adverse est
        // filtré, les totaux ne se recouvrent pas toujours.
        assert_eq!(percent_of(900, 500), 100);
        assert_eq!(percent_of(250, 500), 50);
        assert_eq!(percent_of(1, 3), 33);
    }
}
