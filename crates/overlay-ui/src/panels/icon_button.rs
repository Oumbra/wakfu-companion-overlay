//! Bouton "icône" générique du design system (retour utilisateur 2026-09-06, image de référence
//! `menu-button-icon-first-plan.png` à l'appui) — un socle (`UiIcons::button_background`/
//! `button_background_hover`) et une icône à fond transparent recolorée (`UiIcons::
//! external_link_icon`/`options_icon`/`icon_plus`/`icon_minus`, toutes recolorées au chargement en
//! `#c5cbcc` au repos / `#f4d89f` survolée, voir doc de `ui_icons`) centrée dessus, mis à l'échelle
//! dans le MÊME ratio. Composant PARTAGÉ entre `panels::combat` (boutons lien externe et Options,
//! `bottom_toolbar`) et `panels::watchlist` (boutons "+"/"−", `control_button_row`) — demande
//! explicite de l'utilisateur : « appliques ce système de boutons aux quatre boutons », plutôt que
//! deux implémentations parallèles qui auraient fini par diverger. Le `Sense` (clic pour Combat,
//! survol seul pour Suivi qui reste inerte, voir `panels::watchlist`) et le texte d'infobulle (et
//! son alignement, au-dessus pour Combat, à gauche pour Suivi) restent à la charge de l'appelant :
//! ce composant ne fait QUE peindre le socle + l'icône et renvoyer la `Response` d'interaction.
//!
//! Fond translucide (`PANEL_BACKDROP_FILL`/`_ROUNDING`) : la planche de référence fournie par
//! l'utilisateur montre un léger fond noir semi-opaque derrière la bande de boutons, visible dans
//! le petit écart entre deux boutons adjacents et sur les bords — reproduit ici comme un simple
//! rectangle arrondi peint par l'appelant AVANT ses boutons (voir `combat::bottom_toolbar` et
//! `watchlist::control_button_row`), même teinte que `panels::combat::LEADER_PANEL_FILL` (bandeau
//! translucide déjà validé ailleurs dans cette UI — pas de nouvelle valeur d'opacité inventée).
pub const PANEL_BACKDROP_FILL: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(10, 12, 16, 150);
pub const PANEL_BACKDROP_ROUNDING: f32 = 6.0;

/// Peint le socle (repos ou survolé selon `response.hovered()`) puis l'icône (repos ou survolée)
/// centrée dessus, mis à l'échelle de `rect` (doit être carré, comme le socle et l'icône) — voir
/// doc de module. `id_source` distingue plusieurs boutons icône dans le même conteneur egui (même
/// mécanisme que `combat::paint_side_switch`).
#[allow(clippy::too_many_arguments)]
pub fn paint_icon_button(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id_source: &str,
    sense: egui::Sense,
    background: &egui::TextureHandle,
    background_hover: &egui::TextureHandle,
    icon: &egui::TextureHandle,
    icon_hover: &egui::TextureHandle,
) -> egui::Response {
    let response = ui
        .interact(rect, ui.id().with(id_source), sense)
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let hovered = response.hovered();

    let bg_texture = if hovered { background_hover } else { background };
    egui::Image::new(bg_texture).paint_at(ui, rect);

    // Ratio commun dérivé de la largeur du socle — appliqué tel quel à l'icône, pour qu'elle reste
    // proportionnée au socle quelle que soit la taille cible du bouton (même logique que l'ancien
    // `combat::paint_icon_button`, avant son extraction ici).
    let scale = rect.width() / background.size_vec2().x;
    let icon_texture = if hovered { icon_hover } else { icon };
    let icon_rect = egui::Rect::from_center_size(rect.center(), icon_texture.size_vec2() * scale);
    egui::Image::new(icon_texture).paint_at(ui, icon_rect);

    response
}
