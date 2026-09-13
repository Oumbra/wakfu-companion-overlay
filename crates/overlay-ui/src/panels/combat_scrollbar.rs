//! Ascenseur DESSINÉ des fenêtres à défilement du panneau Combat — partagé depuis le 13 sept. 2026
//! entre le cadre des portraits ennemis (`combat_frame_scroll`, où il est né le 2026-09-07) et la
//! fenêtre des barres de dégâts (`combat_bars`). Un seul dessin, un seul comportement : les deux
//! ascenseurs se font face de part et d'autre de l'air qui sépare les deux colonnes, et doivent
//! rester identiques au pixel.
//!
//! Ce n'est PAS la barre de défilement du jeu (`design::scroll_area`, 6 px, grise puis dorée au
//! survol, à droite du contenu) : celle-ci est une décision propre au panneau Combat, validée par
//! artefacts interactifs (2026-09-07) — fine, couleur unie demandée explicitement, liseré noir
//! peint À L'EXTÉRIEUR du remplissage, coins légèrement arrondis, hauteur proportionnelle à la part
//! visible du contenu avec un minimum, zone de saisie plus large que le dessin, TOUJOURS visible
//! (revirement explicite de l'utilisateur après test en jeu, 2026-09-08).
//!
//! Le composant ne connaît ni combattant ni pixel de contenu : il reçoit la bande verticale que
//! parcourt la poignée, la part visible du contenu et une position normalisée (0 = tout en haut,
//! 1 = tout en bas), peint la poignée et renvoie la position après un éventuel clic ou glissé.
//! L'appelant convertit vers son propre décalage (emplacements fractionnaires pour le cadre,
//! pixels pour les barres) et le persiste lui-même.

/// Couleur unie de la poignée — demande explicite de l'utilisateur, PAS une teinte dérivée du
/// thème ni des PNG `scrollbar-*.png` essayés puis rejetés (voir doc de `combat_frame_scroll`).
const SCROLLBAR_COLOR: egui::Color32 = egui::Color32::from_rgb(0x99, 0x8a, 0x6c);
/// Liseré noir ~1px demandé explicitement — légèrement transparent (comme la maquette validée
/// dans l'artefact) plutôt que noir plein.
const SCROLLBAR_BORDER: egui::Color32 = egui::Color32::from_black_alpha(217);
const SCROLLBAR_BORDER_WIDTH: f32 = 1.0;
/// Coins "légèrement" arrondis — demande explicite, PAS un stade/pilule complet.
const SCROLLBAR_ROUNDING: u8 = 2;
/// Largeur RÉELLE de la poignée dessinée, liseré non compris — fine, comparable aux tiges des
/// ornements du gabarit (PAS la largeur de 14px des PNG essayés puis rejetés, qui la faisait
/// cacher les portraits).
pub(super) const SCROLLBAR_WIDTH: f32 = 5.0;
/// Largeur de la zone cliquable/glissable — plus généreuse que la poignée dessinée, pour rester
/// facile à attraper (même écart que dans l'artefact de calibration).
pub(super) const SCROLLBAR_HIT_WIDTH: f32 = 14.0;
/// Hauteur minimale de la poignée, même à très grand nombre d'éléments — reste cliquable et
/// visible.
const SCROLLBAR_MIN_HEIGHT: f32 = 20.0;

/// Géométrie d'un ascenseur — voir doc de module.
#[derive(Debug, Clone, Copy)]
pub(super) struct DrawnScrollbar {
    /// Bande verticale que parcourt la poignée : celle du contenu qui défile (clip des portraits
    /// pour le cadre, fenêtre des groupes pour les barres). Sa hauteur est la course totale.
    pub band: egui::Rect,
    /// Bord gauche de la zone de saisie (`SCROLLBAR_HIT_WIDTH` de large, toute la hauteur de la
    /// bande).
    pub hit_left: f32,
    /// Bord gauche de la poignée dessinée (`SCROLLBAR_WIDTH` de large ; le liseré déborde d'un
    /// pixel de chaque côté).
    pub thumb_left: f32,
    /// Part du contenu actuellement visible dans la bande, dans `0..=1` — fixe la hauteur de la
    /// poignée (proportionnelle, `SCROLLBAR_MIN_HEIGHT` au minimum).
    pub visible_frac: f32,
}

impl DrawnScrollbar {
    /// Traite clic et glissé dans la zone de saisie puis peint la poignée pour `position`
    /// (`0..=1`). Renvoie la position APRÈS interaction — égale à `position` si rien n'a bougé.
    /// La poignée est peinte à l'ancienne position : la nouvelle ne s'applique qu'à la frame
    /// suivante, une fois persistée par l'appelant (même latence d'une frame que la molette).
    pub fn show(&self, ui: &mut egui::Ui, id: egui::Id, position: f32) -> f32 {
        let band_height = self.band.height();
        let thumb_height = (band_height * self.visible_frac.clamp(0.0, 1.0))
            .max(SCROLLBAR_MIN_HEIGHT)
            .min(band_height);
        let usable = (band_height - thumb_height).max(0.0);
        let thumb_top = self.band.min.y + usable * position.clamp(0.0, 1.0);
        let hit_rect = egui::Rect::from_min_size(
            egui::pos2(self.hit_left, self.band.min.y),
            egui::vec2(SCROLLBAR_HIT_WIDTH, band_height),
        );
        let thumb_rect = egui::Rect::from_min_size(
            egui::pos2(self.thumb_left, thumb_top),
            egui::vec2(SCROLLBAR_WIDTH, thumb_height),
        );

        let mut new_position = position;
        let hit_response = ui.interact(hit_rect, id, egui::Sense::click_and_drag());
        if (hit_response.dragged() || hit_response.clicked()) && usable > 0.0 {
            if let Some(pointer) = hit_response.interact_pointer_pos() {
                // Le centre de la poignée suit le pointeur : un clic dans la piste saute
                // directement à la position visée, un glissé la suit.
                let target_top = pointer.y - thumb_height / 2.0;
                new_position = ((target_top - self.band.min.y) / usable).clamp(0.0, 1.0);
            }
        }

        // Toujours visible — voir doc de module.
        ui.painter().rect(
            thumb_rect,
            SCROLLBAR_ROUNDING,
            SCROLLBAR_COLOR,
            egui::Stroke::new(SCROLLBAR_BORDER_WIDTH, SCROLLBAR_BORDER),
            egui::StrokeKind::Outside,
        );
        new_position
    }
}
