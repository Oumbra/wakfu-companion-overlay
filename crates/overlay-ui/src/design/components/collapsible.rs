//! **Bloc repliable** du design system Wakfu — un cadre à en-tête cliquable qui masque ou révèle
//! son contenu. Composant **conteneur** de la famille §1 bis du contrat, en *forme closure*.
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! design::collapsible("Le Village", &mut ouvert)
//!     .icon(icone_de_quete)
//!     .log_name("quete-village")
//!     .show(ui, |ui| {
//!         // n'importe quoi : du texte, un formulaire, un tableau, des cases à cocher…
//!     });
//! ```
//!
//! ## Ce qu'il fait, et c'est tout
//!
//! **Deux états.** Fermé, le bloc n'est que son en-tête. Ouvert, il montre son contenu et on peut
//! interagir avec. Il n'y a rien d'autre dans sa définition fonctionnelle : le contenu est
//! **entièrement libre** et le composant n'en sait rien — il lui garantit seulement un cadre, des
//! marges et un écrêtage.
//!
//! C'est la raison d'être de la forme closure : le contenu est fourni par l'appelant, le conteneur
//! le referme (il peint son cadre APRÈS l'avoir mesuré) et l'écrête. Un conteneur en forme « zone
//! rendue » ne saurait pas faire la première de ces deux choses.
//!
//! **Le contenu n'est pas appelé quand le bloc est fermé.** `show` rend donc un
//! `InnerResponse<Option<R>>` : `None` dit « la closure n'a pas tourné », ce qui n'est pas la même
//! chose qu'un contenu vide.
//!
//! ## Ce qui a été mesuré, et sur quoi
//!
//! `collapse-block.png` (fermé) et `collapse-block-opened.png` (ouvert) — le bloc de quête du jeu.
//! Les deux captures concordent : l'en-tête occupe la même bande dans l'une et dans l'autre, à
//! quatorze pixels du bord haut.
//!
//! | Grandeur | Valeur | Vérification |
//! | --- | --- | --- |
//! | En-tête | [`tokens::COLLAPSE_HEADER_HEIGHT`] | cadre fermé y 7..66, il ne contient que lui |
//! | Marge latérale | [`tokens::COLLAPSE_PAD_X`] | contenu à x=23, cadre à x=7 |
//! | Marge basse | [`tokens::COLLAPSE_PAD_BOTTOM`] | dernier texte y=279, cadre y=299 |
//! | Titre | [`tokens::COLLAPSE_TITLE_FONT_SIZE`] | 18 px d'encre sur « Le Village » |
//! | Chevron | 12 × 8, à [`tokens::COLLAPSE_PAD_X`] du bord | mesuré x 702..713, cadre à 728 |
//!
//! **Une première tentative a mesuré le mauvais asset** (`collapse-closed.png`, la colonne « Types »
//! de l'Hôtel de Vente). C'en est bien un, mais c'est *un cas d'usage* du repliable — une liste de
//! cases à cocher —, pas le composant. Le relevé qui en sortait décrivait son contenu, donc rien de
//! réutilisable.
//!
//! ## Ce qu'il ne fait pas
//!
//! - **Il ne rend pas le contenu défilable.** Un contenu qui peut déborder l'enveloppe dans une
//!   [`design::scroll_area`](super::scroll_area) — le repliable ne présume pas de sa taille.
//! - **Il ne porte pas son propre état.** L'ouverture est un `&mut bool` de l'appelant, comme la
//!   valeur d'une case à cocher : ce qui doit survivre entre deux frames appartient au panneau
//!   (§6 du contrat).

use egui::{InnerResponse, Rect, Ui, Vec2};

use crate::design::{assets::DsTexture, text, tokens, DesignSystem};

/// Construit un bloc repliable. `open` porte son état et est muté par le clic sur l'en-tête.
pub fn collapsible<'a>(title: impl Into<String>, open: &'a mut bool) -> Collapsible<'a> {
    Collapsible {
        title: title.into(),
        open,
        icon: None,
        log_name: None,
    }
}

/// Voir [`collapsible`].
pub struct Collapsible<'a> {
    title: String,
    open: &'a mut bool,
    icon: Option<DsTexture>,
    log_name: Option<String>,
}

impl<'a> Collapsible<'a> {
    /// Icône d'en-tête, à gauche du titre.
    ///
    /// **Seul endroit du design system où une texture est un paramètre**, et c'est assumé : cette
    /// icône appartient au contenu (une quête, un lieu, une famille d'objets), elle vient donc de la
    /// donnée. Le composant ne peut pas la résoudre depuis une intention comme il le fait pour ses
    /// propres textures.
    pub fn icon(mut self, icon: DsTexture) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Nom d'instance dans `overlay-ui.<date>.log`. Par défaut, le titre.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Peint le bloc sur toute la largeur disponible et, **s'il est ouvert**, appelle
    /// `add_contents` sur un `Ui` réduit à sa zone de contenu — marges appliquées, contenu écrêté.
    ///
    /// L'`inner` de la réponse vaut `None` quand le bloc est fermé : la closure n'a pas tourné.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<Option<R>> {
        let Collapsible {
            title,
            open,
            icon,
            log_name,
        } = self;

        let width = ui.available_width();
        let header_height = tokens::COLLAPSE_HEADER_HEIGHT;

        // Le cadre est peint APRÈS le contenu — sa hauteur n'est connue qu'une fois celui-ci mesuré.
        // `Painter::add` réserve la place dans la liste d'affichage et rend un index ; `Painter::set`
        // y écrit la forme définitive plus tard. C'est l'idiome egui pour un fond qui enveloppe, et
        // le seul moyen de ne pas peindre le cadre par-dessus son propre contenu.
        let frame_slot = ui.painter().add(egui::Shape::Noop);

        let start = ui.cursor().min;
        let header_rect = Rect::from_min_size(start, Vec2::new(width, header_height));
        let header_response = ui.allocate_rect(header_rect, egui::Sense::click());

        let (inner, frame_bottom) = if *open {
            // Le contenu commence sous l'en-tête, en retrait des marges latérales. Sa hauteur est
            // libre : c'est lui qui la décide, et le cadre s'y ajuste.
            let content_top = header_rect.bottom();
            let content_rect = Rect::from_min_max(
                egui::pos2(start.x + tokens::COLLAPSE_PAD_X, content_top),
                egui::pos2(
                    start.x + width - tokens::COLLAPSE_PAD_X,
                    ui.max_rect().bottom(),
                ),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
            // `intersect` plutôt qu'un `set` sec — même idiome que `design::panel` : remplacer le
            // clip de l'ancêtre laisserait le contenu déborder si le bloc se retrouvait un jour
            // dans un clip plus étroit.
            child.set_clip_rect(content_rect.intersect(ui.clip_rect()));
            let result = add_contents(&mut child);
            let used = child.min_rect().height();
            // La marge basse est réservée APRÈS le contenu : c'est elle qui donne au cadre sa
            // hauteur finale.
            let body = Rect::from_min_size(
                egui::pos2(start.x, content_top),
                Vec2::new(width, used + tokens::COLLAPSE_PAD_BOTTOM),
            );
            ui.allocate_rect(body, egui::Sense::hover());
            (Some(result), body.bottom())
        } else {
            (None, header_rect.bottom())
        };

        // **Le bas du cadre est celui que ce bloc a alloué, pas `ui.min_rect().bottom()`.** Ce
        // dernier est le rectangle occupé du `Ui` APPELANT : il ne coïncide avec le bas du bloc
        // que si celui-ci est le dernier élément posé, et la galerie a montré ce que coûte cette
        // supposition — le cadre courait jusque sous le bloc suivant, qui le recouvrait, et
        // *aucun des blocs sauf le dernier n'avait de bord bas*. Le liseré, les deux motifs
        // d'angle du bas et l'espacement qui sépare deux blocs disparaissaient avec lui.
        let frame_rect = Rect::from_min_max(
            start,
            egui::pos2(start.x + width, frame_bottom.max(header_rect.bottom())),
        );

        if ui.is_rect_visible(frame_rect) {
            let ds = DesignSystem::get(ui.ctx());
            ds.paint_to_slot(
                ui.painter(),
                frame_slot,
                frame_rect,
                DsTexture::CollapseFrame,
                egui::Color32::WHITE,
            );
            paint_header(ui, header_rect, &title, icon, *open);
        }

        if header_response.clicked() {
            *open = !*open;
            tracing::debug!(
                component = "collapsible",
                name = log_name.as_deref().unwrap_or(title.as_str()),
                open = *open,
                "clic"
            );
        }

        InnerResponse::new(inner, header_response)
    }
}

/// Peint le contenu de l'en-tête : icône optionnelle, titre, chevron.
fn paint_header(ui: &Ui, rect: Rect, title: &str, icon: Option<DsTexture>, open: bool) {
    let ds = DesignSystem::get(ui.ctx());
    let mut x = rect.left() + tokens::COLLAPSE_PAD_X;

    if let Some(icon) = icon {
        let side = rect.height() * tokens::COLLAPSE_ICON_RATIO;
        let native = ds.native_size(icon);
        let icon_rect = Rect::from_center_size(
            egui::pos2(x + side / 2.0, rect.center().y),
            super::icon_button::glyph_fit(native, side),
        );
        ds.paint(ui.painter(), icon_rect, icon, egui::Color32::WHITE);
        x += side + tokens::COLLAPSE_ICON_GAP;
    }

    // Titre en serif grasse, cerné d'une ombre bas-droite : le fond est connu (le cadre), un
    // contour complet empâterait le mot — même règle que le titre de fenêtre.
    text::paint_outlined_text(
        ui,
        egui::pos2(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        text::title_font(ui.ctx(), tokens::COLLAPSE_TITLE_FONT_SIZE),
        tokens::WINDOW_TITLE_TEXT,
        text::SHADOW_BOTTOM_RIGHT,
    );

    // Chevron — le glyphe du manifeste, retourné quand le bloc est ouvert. Le jeu ne change QUE
    // cela entre ses deux états : ni la graisse du titre, ni la couleur, ni le fond ne bougent.
    let native = ds.native_size(DsTexture::IconChevronDown);
    let chevron = Rect::from_center_size(
        egui::pos2(
            rect.right() - tokens::COLLAPSE_PAD_X - native.x / 2.0,
            rect.center().y,
        ),
        native,
    );
    if open {
        // Retournement vertical : le manifeste n'a qu'un chevron, et en fabriquer un second serait
        // un asset par état — ce que le contrat interdit.
        ds.paint_flipped_y(
            ui.painter(),
            chevron,
            DsTexture::IconChevronDown,
            tokens::ICON_TINT,
        );
    } else {
        ds.paint(
            ui.painter(),
            chevron,
            DsTexture::IconChevronDown,
            tokens::ICON_TINT,
        );
    }
}

/// Réponse d'un bloc repliable — voir [`Collapsible::show`].
pub type CollapsibleResponse<R> = InnerResponse<Option<R>>;

/// Hauteur qu'occupe un bloc fermé, sans le dessiner.
pub fn closed_height() -> f32 {
    tokens::COLLAPSE_HEADER_HEIGHT
}

/// Hauteur qu'occupera un bloc ouvert dont le contenu mesure `content_height`.
///
/// Fonction libre plutôt que calcul enfoui dans `show` : c'est ce qu'un panneau doit pouvoir
/// anticiper pour réserver sa place, et c'est éprouvable sans contexte egui.
pub fn open_height(content_height: f32) -> f32 {
    tokens::COLLAPSE_HEADER_HEIGHT + content_height.max(0.0) + tokens::COLLAPSE_PAD_BOTTOM
}

/// Largeur utile au contenu dans un bloc de largeur `width`.
pub fn content_width(width: f32) -> f32 {
    (width - 2.0 * tokens::COLLAPSE_PAD_X).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tolérance de comparaison — voir `window::tests`.
    const EPS: f32 = 0.01;

    /// À l'état fermé, le bloc du jeu fait exactement la hauteur de son en-tête : son cadre court de
    /// y=7 à y=66 sur `collapse-block.png`, et ne contient rien d'autre.
    #[test]
    fn un_bloc_ferme_fait_la_hauteur_de_son_entete() {
        assert!((closed_height() - 60.0).abs() < EPS);
    }

    /// Ouvert, il ajoute son contenu ET sa marge basse — celle-ci n'est pas optionnelle, c'est elle
    /// qui donne au cadre sa respiration sous le dernier élément.
    #[test]
    fn un_bloc_ouvert_ajoute_le_contenu_et_la_marge_basse() {
        assert!((open_height(100.0) - (60.0 + 100.0 + 20.0)).abs() < EPS);
        // Un contenu vide laisse quand même la marge : le cadre ne se referme pas sur son en-tête.
        assert!((open_height(0.0) - 80.0).abs() < EPS);
    }

    /// Un contenu de hauteur négative n'existe pas, mais un calcul qui en produirait une ne doit pas
    /// rendre un cadre plus court que son en-tête.
    #[test]
    fn un_contenu_de_hauteur_negative_ne_retrecit_pas_le_cadre() {
        assert!(open_height(-50.0) >= closed_height());
    }

    /// La largeur utile retire les deux marges, et ne devient jamais négative — un rectangle inversé
    /// se peindrait n'importe où.
    #[test]
    fn la_largeur_utile_retire_les_deux_marges_sans_devenir_negative() {
        assert!((content_width(722.0) - (722.0 - 32.0)).abs() < EPS);
        assert!(content_width(10.0) >= 0.0);
        assert!(content_width(0.0) >= 0.0);
    }
}
