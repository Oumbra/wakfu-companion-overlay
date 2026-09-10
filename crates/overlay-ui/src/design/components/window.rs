//! **Chrome de fenêtre** du design system Wakfu — bannière de titre, corps, barre d'onglets et
//! pied de page. Le premier **composant conteneur** de la famille §1 bis du contrat, en *forme
//! « zone rendue »* : il peint tout son décor, puis rend les zones où l'appelant écrit.
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! let chrome = design::window("Options")
//!     .footer("Annuler", "Valider")
//!     .log_name("options")
//!     .show(ui);
//!
//! chrome.tabs(ui, design::tabs(&mut state.tab).entry(Tab::Parametres, "Paramètres"));
//! // `chrome.content` : la zone entre la barre d'onglets et le pied de page.
//! // `chrome.footer`  : ce que le pied de page vient de recevoir.
//! ```
//!
//! ## Pourquoi la forme « zone rendue » plutôt qu'une closure
//!
//! Le contrat impose la forme closure par défaut, et n'admet celle-ci qu'à deux conditions, toutes
//! deux remplies ici : le conteneur ne peint **rien après** le contenu (bannière, corps, onglets et
//! pied sont tous posés avant), et il rend **plusieurs sorties indépendantes** — la zone de contenu
//! *et* le clic de pied de page — qu'un `InnerResponse<R>` n'empaquetterait qu'artificiellement.
//!
//! Le prix, explicite : ce composant **n'écrête pas** ce que l'appelant peint dans `content`. Un
//! contenu qui doit l'être passe par [`design::panel`](super::panel), qui est en forme closure.
//!
//! ## Ce que remplace ce composant
//!
//! `panels::options_modal` peignait tout ce décor lui-même : onze rectangles calculés à la main et
//! trente constantes de mise en page pour une seule fenêtre. Le 2026-09-10, une session a extrait
//! ce décor dans une fonction `chrome()` du panneau — parce que maquetter un second onglet
//! demandait de le réutiliser, et que le recopier en aurait fait une deuxième implémentation. Elle
//! était au bon endroit dans l'intention, au mauvais dans l'arborescence : le contrat n'avait pas
//! encore de place pour un conteneur. Il en a une depuis, et c'est ce fichier.
//!
//! Le déplacement est **à pixel constant** — c'est ce que vérifient les snapshots
//! `options_modale_sur_damier.png` et `options_modale_avec_erreur.png` à chaque exécution.
//!
//! ## Mesures
//!
//! Toutes dans [`tokens`], préfixées `WINDOW_`, avec la provenance de chacune. Elles viennent du
//! relevé de la fenêtre Options du jeu (six captures au chrome identique).

use egui::{Rect, Response, Ui};

use crate::design::{
    assets::DsTexture, components::button, components::tabs::Tabs, text, tokens, DesignSystem,
};

/// Ce que le pied de page vient de recevoir.
///
/// Volontairement pauvre : le chrome ne sait pas *ce qu'il y a à valider*, seulement qu'on a
/// cliqué. C'est l'appelant qui attache la charge utile — un chemin de fichier pour l'onglet
/// Paramètres, autre chose pour un autre onglet.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FooterClick {
    #[default]
    None,
    Cancel,
    Validate,
}

/// Construit un chrome de fenêtre au style du jeu. `title` est peint dans la bannière.
pub fn window(title: impl Into<String>) -> Window {
    Window {
        title: title.into(),
        tab_bar_height: tokens::TAB_HEIGHT,
        footer: None,
        log_name: None,
    }
}

/// Voir [`window`].
pub struct Window {
    title: String,
    tab_bar_height: f32,
    footer: Option<(String, String)>,
    log_name: Option<String>,
}

impl Window {
    /// Réserve la place d'une barre d'onglets sous la bannière. Par défaut, la hauteur native de
    /// [`tokens::TAB_HEIGHT`] ; `0.0` supprime la réserve pour une fenêtre sans onglets.
    ///
    /// La barre elle-même n'est **pas** peinte ici : c'est [`WindowChrome::tabs`] qui la pose, à
    /// partir du [`Tabs`] que l'appelant construit. Un onglet est du contenu, pas du décor.
    pub fn tab_bar_height(mut self, height: f32) -> Self {
        self.tab_bar_height = height;
        self
    }

    /// Pied de page à deux boutons — annulation à gauche, validation à droite, largeur partagée.
    /// Sans cet appel, la fenêtre n'a pas de pied et son contenu descend jusqu'en bas.
    pub fn footer(mut self, cancel: impl Into<String>, validate: impl Into<String>) -> Self {
        self.footer = Some((cancel.into(), validate.into()));
        self
    }

    /// Nom d'instance dans `overlay-ui.<date>.log` — préfixe les deux boutons du pied de page.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Peint le décor dans **tout le rectangle disponible** de `ui` et rend les zones de contenu.
    pub fn show(self, ui: &mut Ui) -> WindowChrome {
        let zones = layout(ui.max_rect(), self.tab_bar_height, self.footer.is_some());
        let ds = DesignSystem::get(ui.ctx());

        // Corps — texture du jeu, ses DEUX ANGLES BAS portés par son alpha : inutile de leur
        // passer un `corner_radius`, un `Mesh` egui ne saurait de toute façon pas découper un coin.
        ds.paint(
            ui.painter(),
            zones.body,
            DsTexture::ModalBody,
            tokens::WINDOW_BODY_TINT,
        );

        // Bannière — arrondie sur les coins HAUTS uniquement pour épouser le coin de la fenêtre
        // (ses coins bas ne sont jamais visibles, masqués par le corps qui la recouvre en dessous).
        // Peinte par `egui::Image` et non par le 9-slice : un `Mesh` ne sait pas arrondir un coin.
        egui::Image::new(ds.texture(DsTexture::ModalHeader))
            .corner_radius(egui::CornerRadius {
                nw: tokens::WINDOW_RADIUS,
                ne: tokens::WINDOW_RADIUS,
                sw: 0,
                se: 0,
            })
            .paint_at(ui, zones.banner);

        // Titre — serif grasse, cerné d'une ombre portée bas-droite et non d'un contour complet :
        // le fond est ici CONNU (la bannière), on peut donc ne cerner qu'un côté, comme le jeu le
        // fait pour ses titres, éclairés depuis le haut-gauche. Un contour complet empâte le mot.
        text::paint_outlined_text(
            ui,
            zones.banner.center(),
            egui::Align2::CENTER_CENTER,
            &self.title,
            text::title_font(ui.ctx(), tokens::WINDOW_TITLE_FONT_SIZE),
            tokens::WINDOW_TITLE_TEXT,
            text::SHADOW_BOTTOM_RIGHT,
        );

        // Pied de page — deux boutons du design system, séparés par la gouttière du jeu.
        let footer = match (&self.footer, zones.footer) {
            (Some((cancel_label, validate_label)), Some((cancel_rect, validate_rect))) => {
                let prefix = self.log_name.as_deref().unwrap_or("fenetre");
                let mut click = FooterClick::None;
                if ui
                    .put(
                        cancel_rect,
                        button::button(cancel_label.clone())
                            .variant(button::ButtonVariant::Danger)
                            .size(button::ButtonSize::Height(cancel_rect.height()))
                            .width(cancel_rect.width())
                            .log_name(format!("{prefix}-annuler")),
                    )
                    .clicked()
                {
                    click = FooterClick::Cancel;
                }
                if ui
                    .put(
                        validate_rect,
                        button::button(validate_label.clone())
                            .variant(button::ButtonVariant::Primary)
                            .size(button::ButtonSize::Height(validate_rect.height()))
                            .width(validate_rect.width())
                            .log_name(format!("{prefix}-valider")),
                    )
                    .clicked()
                {
                    click = FooterClick::Validate;
                }
                click
            }
            _ => FooterClick::None,
        };

        WindowChrome {
            tab_bar: zones.tab_bar,
            content: zones.content,
            footer,
        }
    }
}

/// Ce que [`Window::show`] rend : où écrire, et ce que le pied de page a reçu.
pub struct WindowChrome {
    /// Bande réservée à la barre d'onglets, sous la bannière — hauteur nulle si la fenêtre n'en a
    /// pas. Poser la barre passe par [`WindowChrome::tabs`], qui y applique le cadrage.
    pub tab_bar: Rect,
    /// Zone entre la barre d'onglets et le pied de page : tout le contenu de la fenêtre.
    ///
    /// **Non écrêtée** — c'est le prix de la forme « zone rendue » (voir la doc de module). Un
    /// contenu qui peut déborder passe par [`design::panel`](super::panel).
    pub content: Rect,
    /// Ce que le pied de page vient de recevoir.
    pub footer: FooterClick,
}

impl WindowChrome {
    /// Pose une barre d'onglets dans [`WindowChrome::tab_bar`].
    ///
    /// La généricité sur le type d'onglet vit sur cette méthode, pas sur le conteneur : une
    /// fenêtre n'a aucune raison d'être paramétrée par le type d'onglet de son contenu — c'est
    /// aussi ce qui permet à une fenêtre sans onglets de ne pas avoir à en nommer un.
    pub fn tabs<T: PartialEq + Copy>(&self, ui: &mut Ui, tabs: Tabs<'_, T>) -> Response {
        ui.scope_builder(egui::UiBuilder::new().max_rect(self.tab_bar), |ui| {
            tabs.show(ui)
        })
        .inner
    }
}

/// Découpage du rectangle d'une fenêtre en zones.
///
/// **Fonction libre plutôt que corps de [`Window::show`]**, pour la même raison qu'
/// `icon_button::glyph_fit` : c'est le calcul du composant qui peut se tromper en silence, et il
/// s'éprouve sans contexte egui ni GPU (voir les tests en bas de ce fichier). Un décalage de
/// quelques pixels sur la gouttière du pied de page ou sur l'écart sous les onglets ne se voit sur
/// une capture qu'une fois comparée au jeu, côte à côte.
fn layout(rect: Rect, tab_bar_height: f32, has_footer: bool) -> WindowLayout {
    // La bannière porte les angles hauts ; le corps commence exactement là où elle s'arrête, les
    // deux se juxtaposent sans recouvrement.
    let banner = Rect::from_min_size(
        rect.min,
        egui::vec2(rect.width(), tokens::WINDOW_BANNER_HEIGHT),
    );
    let body = Rect::from_min_max(egui::pos2(rect.left(), banner.bottom()), rect.max);

    let content_rect = Rect::from_min_max(
        egui::pos2(
            rect.left() + tokens::WINDOW_PAD_SIDE,
            banner.bottom() + tokens::WINDOW_PAD_TOP,
        ),
        egui::pos2(
            rect.right() - tokens::WINDOW_PAD_SIDE,
            rect.bottom() - tokens::WINDOW_PAD_BOTTOM,
        ),
    );

    let tab_bar = Rect::from_min_size(
        content_rect.min,
        egui::vec2(content_rect.width(), tab_bar_height),
    );

    let (footer, content_bottom) = if has_footer {
        let height = tokens::WINDOW_FOOTER_BUTTON_HEIGHT;
        // Plancher à zéro : une fenêtre plus étroite que sa gouttière donnerait deux rectangles de
        // largeur négative, qu'egui inverserait et peindrait n'importe où. Zéro les rend
        // invisibles, ce qui se voit sur une capture — c'est la règle du contrat.
        let width = ((content_rect.width() - tokens::WINDOW_FOOTER_GUTTER) / 2.0).max(0.0);
        let top = content_rect.bottom() - height;
        (
            Some((
                Rect::from_min_size(
                    egui::pos2(content_rect.left(), top),
                    egui::vec2(width, height),
                ),
                Rect::from_min_size(
                    egui::pos2(content_rect.right() - width, top),
                    egui::vec2(width, height),
                ),
            )),
            top - tokens::WINDOW_FOOTER_GAP,
        )
    } else {
        (None, content_rect.bottom())
    };

    WindowLayout {
        banner,
        body,
        tab_bar,
        content: Rect::from_min_max(
            egui::pos2(
                content_rect.left(),
                tab_bar.bottom() + tokens::WINDOW_TAB_GAP,
            ),
            egui::pos2(content_rect.right(), content_bottom),
        ),
        footer,
    }
}

/// Ce que [`layout`] découpe — voir sa doc.
struct WindowLayout {
    banner: Rect,
    body: Rect,
    tab_bar: Rect,
    content: Rect,
    /// `(annuler, valider)`, ou `None` pour une fenêtre sans pied de page.
    footer: Option<(Rect, Rect)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tolérance de comparaison — ces cotes finissent en coordonnées de peinture flottantes, pas en
    /// pixels entiers ; un centième suffit à attraper une erreur de formule.
    const EPS: f32 = 0.01;

    /// Les cotes exactes de la fenêtre Options telle que l'overlay la crée
    /// (`panels::options_modal::WINDOW_SIZE`), à l'origine — le cas que les snapshots vérifient.
    fn fenetre_options() -> Rect {
        Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(560.0, 436.0))
    }

    #[test]
    fn la_banniere_et_le_corps_se_juxtaposent_sans_recouvrement() {
        let z = layout(fenetre_options(), tokens::TAB_HEIGHT, true);
        assert!((z.banner.height() - tokens::WINDOW_BANNER_HEIGHT).abs() < EPS);
        // Le corps commence EXACTEMENT où la bannière finit : un pixel de recouvrement ferait
        // baver la texture du corps sur l'angle arrondi de la bannière, un pixel de vide y
        // laisserait voir le jeu au travers.
        assert!((z.body.top() - z.banner.bottom()).abs() < EPS);
        assert!((z.body.bottom() - fenetre_options().bottom()).abs() < EPS);
    }

    #[test]
    fn les_deux_boutons_de_pied_partagent_la_largeur_moins_la_gouttiere() {
        let z = layout(fenetre_options(), tokens::TAB_HEIGHT, true);
        let (cancel, validate) = z.footer.expect("pied de page demandé");
        assert!(
            (cancel.width() - validate.width()).abs() < EPS,
            "largeurs inégales"
        );
        // La gouttière est ce qui les sépare, et rien d'autre.
        assert!((validate.left() - cancel.right() - tokens::WINDOW_FOOTER_GUTTER).abs() < EPS);
        // Hauteur NATIVE de la texture, jamais déduite de la largeur — la faute qui affichait des
        // boutons de 27 px là où le jeu en met 36.
        assert!((cancel.height() - tokens::WINDOW_FOOTER_BUTTON_HEIGHT).abs() < EPS);
        // Et les deux bords extérieurs tombent sur les marges de contenu de la fenêtre.
        assert!((cancel.left() - tokens::WINDOW_PAD_SIDE).abs() < EPS);
        assert!(
            (fenetre_options().right() - validate.right() - tokens::WINDOW_PAD_SIDE).abs() < EPS
        );
    }

    #[test]
    fn sans_pied_de_page_le_contenu_descend_jusqu_a_la_marge_basse() {
        let avec = layout(fenetre_options(), tokens::TAB_HEIGHT, true);
        let sans = layout(fenetre_options(), tokens::TAB_HEIGHT, false);
        assert!(sans.footer.is_none());
        assert!(
            sans.content.bottom() > avec.content.bottom(),
            "le contenu doit gagner la place du pied"
        );
        assert!(
            (fenetre_options().bottom() - sans.content.bottom() - tokens::WINDOW_PAD_BOTTOM).abs()
                < EPS
        );
    }

    #[test]
    fn une_barre_d_onglets_de_hauteur_nulle_ne_laisse_que_sa_gouttiere() {
        let sans = layout(fenetre_options(), 0.0, true);
        let avec = layout(fenetre_options(), tokens::TAB_HEIGHT, true);
        assert!((sans.tab_bar.height()).abs() < EPS);
        // Le contenu remonte exactement de la hauteur de la barre — l'écart sous elle, lui, reste.
        assert!((avec.content.top() - sans.content.top() - tokens::TAB_HEIGHT).abs() < EPS);
    }

    #[test]
    fn une_fenetre_plus_etroite_que_sa_gouttiere_ne_produit_pas_de_rectangle_inverse() {
        // 20 px de large : moins que les deux marges latérales, donc une largeur de contenu
        // négative. Rien ne doit s'inverser — un rectangle inversé se peint n'importe où.
        let z = layout(
            Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(20.0, 200.0)),
            tokens::TAB_HEIGHT,
            true,
        );
        let (cancel, validate) = z.footer.expect("pied de page demandé");
        assert!(
            cancel.width() >= 0.0,
            "largeur négative : {}",
            cancel.width()
        );
        assert!(
            validate.width() >= 0.0,
            "largeur négative : {}",
            validate.width()
        );
    }
}
