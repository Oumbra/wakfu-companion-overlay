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
//! Source unique : **`docs/design-system/collapse-block.json`**, le relevé outillé du bloc de quête
//! du jeu, établi sur les captures détourées `collapse-block-closed.png` (732 × 60) et
//! `collapse-block-opened.png` (732 × 210). Les deux concordent au pixel sur l'en-tête — même bande
//! à 7 px du bord haut, seule la hauteur totale change.
//!
//! | Grandeur | Valeur | Vérification |
//! | --- | --- | --- |
//! | En-tête | [`tokens::COLLAPSE_HEADER_HEIGHT`] | cadre fermé 732 × 60, il ne contient que lui |
//! | Marge latérale | [`tokens::COLLAPSE_PAD_X`] | icône et intitulés à x=17 |
//! | Marge basse | [`tokens::COLLAPSE_PAD_BOTTOM`] | `padding` du nœud `frame` |
//! | Titre | [`tokens::COLLAPSE_TITLE_FONT_SIZE`] | 14 px d'encre, `#fefefe` |
//! | Icône | [`tokens::COLLAPSE_ICON_RATIO`] | 33 × 32 dans un en-tête de 60 |
//! | Gouttière icône–titre | [`tokens::COLLAPSE_ICON_GAP`] | icône finit à x=50, titre à x=60 |
//! | Chevron | 14 × 8, à [`tokens::COLLAPSE_CHEVRON_PAD_X`] du bord | glyphe x 705..719, cadre 732 |
//!
//! **Deux relevés ont été écartés en chemin**, et les deux pour la même raison : ils décrivaient
//! autre chose que le composant. Le premier portait sur `collapse-closed.png`, la colonne « Types »
//! de l'Hôtel de Vente — *un cas d'usage* du repliable, dont le relevé détaillait la liste de cases
//! à cocher. Le second portait sur les bonnes captures mais **non détourées** : l'ombre portée et le
//! fond de jeu y comptaient comme de l'encre, d'où un titre annoncé à 18 px au lieu de 14 et une
//! icône à 35 au lieu de 33.
//!
//! ## Les deux états du survol
//!
//! Le jeu **éclaircit le cadre entier** quand le pointeur est sur l'en-tête : `#28292b` → `#323436`,
//! dix niveaux sur chaque canal. Une teinte egui multiplie et ne peut donc que foncer — d'où une
//! seconde texture, [`DsTexture::CollapseBlockHover`], et c'est le seul asset par état de ce
//! composant. Le chevron, lui, se retourne sans second fichier.
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
        forced_hover: None,
    }
}

/// Voir [`collapsible`].
pub struct Collapsible<'a> {
    title: String,
    open: &'a mut bool,
    icon: Option<DsTexture>,
    log_name: Option<String>,
    forced_hover: Option<bool>,
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

    /// Force l'état survolé, **sans passer par l'interaction** — même rôle et mêmes réserves que
    /// [`Button::preview_state`](super::button::Button::preview_state) : aucun pointeur ne survole
    /// quoi que ce soit dans un rendu offscreen, et c'est le seul moyen de montrer les deux états
    /// côte à côte sur une planche de contrôle. En usage normal, ne pas l'appeler.
    pub fn preview_hovered(mut self, hovered: bool) -> Self {
        self.forced_hover = Some(hovered);
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
            forced_hover,
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

        // Le survol de l'EN-TÊTE éclaircit le cadre ENTIER — c'est ce que font les deux captures
        // survolées du jeu, contenu compris. Survoler le contenu, lui, ne l'éclaircit pas : rien
        // n'y est cliquable, et le faire promettrait une action qui n'existe pas.
        let hovered = forced_hover.unwrap_or_else(|| header_response.hovered());

        if ui.is_rect_visible(frame_rect) {
            let ds = DesignSystem::get(ui.ctx());
            ds.paint_to_slot(
                ui.painter(),
                frame_slot,
                frame_rect,
                if hovered {
                    DsTexture::CollapseBlockHover
                } else {
                    DsTexture::CollapseBlock
                },
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
        tokens::COLLAPSE_TITLE_TEXT,
        text::SHADOW_BOTTOM_RIGHT,
    );

    // Chevron — le glyphe du manifeste, retourné quand le bloc est ouvert. Le jeu ne change QUE
    // cela entre ses deux états : ni la graisse du titre, ni la couleur, ni le fond ne bougent.
    let native = ds.native_size(DsTexture::IconChevronDown);
    let chevron = Rect::from_center_size(
        egui::pos2(
            rect.right() - tokens::COLLAPSE_CHEVRON_PAD_X - native.x / 2.0,
            rect.center().y,
        ),
        native,
    );
    if open {
        // Retournement vertical : le manifeste n'a qu'un chevron, et en fabriquer un second serait
        // un asset par état — ce que le contrat interdit. (Le cadre survolé, lui, en est un : voir
        // `DsTexture::CollapseBlockHover`, où la teinte ne pouvait pas faire le travail.)
        ds.paint_flipped_y(
            ui.painter(),
            chevron,
            DsTexture::IconChevronDown,
            tokens::COLLAPSE_CHEVRON_TINT,
        );
    } else {
        ds.paint(
            ui.painter(),
            chevron,
            DsTexture::IconChevronDown,
            tokens::COLLAPSE_CHEVRON_TINT,
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
    /// y=7 à y=66 sur `collapse-block-closed.png`, et ne contient rien d'autre.
    #[test]
    fn un_bloc_ferme_fait_la_hauteur_de_son_entete() {
        assert!((closed_height() - 60.0).abs() < EPS);
    }

    /// Ouvert, il ajoute son contenu ET sa marge basse — celle-ci n'est pas optionnelle, c'est elle
    /// qui donne au cadre sa respiration sous le dernier élément.
    #[test]
    fn un_bloc_ouvert_ajoute_le_contenu_et_la_marge_basse() {
        assert!((open_height(100.0) - (60.0 + 100.0 + 23.0)).abs() < EPS);
        // Un contenu vide laisse quand même la marge : le cadre ne se referme pas sur son en-tête.
        assert!((open_height(0.0) - 83.0).abs() < EPS);
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
        assert!((content_width(732.0) - (732.0 - 34.0)).abs() < EPS);
        assert!(content_width(10.0) >= 0.0);
        assert!(content_width(0.0) >= 0.0);
    }
}
