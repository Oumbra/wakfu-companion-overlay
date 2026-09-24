//! **Bouton icône** du design system Wakfu — un socle carré et un glyphe centré dessus.
//!
//! ```ignore
//! use overlay_ui::design::{self, DsTexture, IconContext};
//!
//! if ui
//!     .add(design::icon_button(DsIcon::Option).context(IconContext::FirstPlan))
//!     .clicked()
//! {
//!     // l'appelant décide de l'action — jamais le composant
//! }
//! ```
//!
//! ## Ce que ce composant reprend, et ce qu'il corrige
//!
//! Une implémentation existait déjà — `panels::icon_button::paint_icon_button` — et elle
//! fonctionne. Elle ne respecte simplement pas le contrat de composant : elle prend **quatre
//! `egui::TextureHandle` en paramètres**, ce qui oblige chaque appelant à connaître et à câbler les
//! textures. C'est exactement le symptôme qui a motivé la création du design system.
//!
//! Ici, l'appelant nomme une **intention** (`IconContext::FirstPlan`, `DsIcon::Plus`) et le
//! manifeste résout les fichiers.
//!
//! Ce qui est repris tel quel, parce que c'est mesuré ou déjà arbitré :
//!
//! - **Le socle et l'icône partagent le même facteur d'échelle**, dérivé de la largeur du socle. Une
//!   icône reste donc proportionnée à son bouton quelle que soit la taille demandée.
//! - **Un appui de souris retire l'apparence survolée**, elle revient au relâchement — la règle
//!   commune à toute l'interface (`response.hovered() && !pointer.any_down()`), corrigée une fois en
//!   2026-09-08 parce que deux familles de boutons s'en écartaient.
//! - **Les deux teintes d'icône** : `#c5cbcc` au repos, `#f4d89f` au survol, mesurées sur
//!   `menu-button-icon-first-plan.png`. Les icônes du design system étant blanc pur avec alpha, une
//!   simple teinte les reproduit — là où `ui_icons` en charge deux copies recolorées. Ce couple est
//!   celui du **premier plan**, et de lui seul (voir la section suivante).
//!
//! ## Le glyphe ne change de couleur qu'en premier plan
//!
//! Les deux teintes ci-dessus ont été mesurées sur une planche de socles **bleus** — la barre de
//! premier plan du jeu. Elles s'étaient appliquées par défaut aux quatre contextes, faute de mesure
//! ailleurs. Retour utilisateur 2026-09-14, deux captures du jeu à l'appui (le bouton « Jouer le son
//! d'alerte » au repos et survolé) : sur le socle kaki d'un panneau, le glyphe est **blanc et le
//! reste au survol** — seule la texture du socle s'éclaircit. C'est [`tokens::PANEL_ICON_TINT`],
//! rendu par `IconContext::icon_tint` comme les teintes figées du pas et de la croix de bannière ;
//! `FirstPlan` est désormais le seul contexte dont le glyphe vire à l'or sous la souris.
//!
//! ## Deux contextes de socle
//!
//! | Contexte | Socle | Glyphe (repos → survol) | Où |
//! | --- | --- | --- | --- |
//! | `FirstPlan` | `button-icon-first-plan.png` | `#c5cbcc` → `#f4d89f` | barre de premier plan, par-dessus le jeu |
//! | `Panel` | `button-icon.png` | blanc → blanc | à l'intérieur d'un panneau |
//! | `Stepper` | `button-stepper.png` | or → or | de part et d'autre d'un champ numérique |
//! | `Banner` | **aucun — un voile peint** | or → or | dans la bannière d'une fenêtre (la croix de fermeture) |
//!
//! Les deux premiers font **36 × 36**, la taille native. `button-icon-disabled.png` est partagée
//! par les deux — le jeu n'a qu'une capture de socle grisé, comme pour le bouton texte.
//!
//! ## Le contexte `Banner` n'a pas de texture, et ce n'est pas une dérogation
//!
//! Le bouton de fermeture du jeu est un carré arrondi **translucide** posé sur la bannière : ses
//! hachures se voient au travers, au repos comme au survol (`window-close.png`,
//! `window-close-hover.png`). Une texture le figerait avec un morceau de bannière dedans, faux dès
//! que le bouton bouge d'un pixel. Le voile est donc peint — un `rect_filled` noir à l'alpha mesuré
//! ([`tokens::WINDOW_CLOSE_FILL`], `_HOVER`) et un liseré d'1 px au repos — sur ce que la fenêtre a
//! déjà peint dessous. C'est le seul contexte dont le glyphe est **doré dans les deux états** : le
//! survol de ce bouton se lit sur son voile, pas sur sa croix.
//!
//! ## La taille d'encre vient du manifeste
//!
//! Les glyphes de `assets/design-system/icons/` sont détourés au pixel près : leur fichier fait
//! exactement la taille de leur encre, et celle-ci varie d'un glyphe à l'autre (13 pour le lien
//! externe, 16 pour le rouage). Peints tels quels, ils donneraient trois hauteurs d'encre
//! différentes dans une même barre.
//!
//! Le jeu, lui, les cale sur une grille commune. Mesuré le 2026-09-10 sur
//! `menu-button-icon-first-plan.png` — déjà la source des deux teintes ci-dessus, mais personne n'y
//! avait mesuré la taille d'encre : **bbox de 16 à 20 px pour un socle de 36**, médiane 18, toutes
//! centrées au pixel près sur l'axe du socle (seuil de luminance 140, résultat invariant de 120 à
//! 180). La capture est bien à l'échelle 1 : `button-icon-first-plan.png` s'y recale à 36 × 36 avec
//! un écart moyen de 2,3/255 sur les pixels de socle, contre 4,5 et plus dès 35 ou 37.
//!
//! C'est [`tokens::ICON_BUTTON_CONTENT`], appliqué par `DsTexture::icon_content_size` — donc par
//! **le manifeste, jamais par l'appelant** : la taille d'encre est une propriété de l'asset.
//! `icon_draw_size` en bas de ce fichier fait le calcul, et ses tests l'éprouvent sans GPU.
//!
//! Cette doc a longtemps dit qu'« aucune capture de référence ne permet d'arbitrer laquelle des
//! deux normalisations est la bonne », et c'est pour cette raison que les quatre boutons icône de
//! l'overlay ne passaient pas encore par ce composant. La mesure ci-dessus a tranché : le principe
//! de normalisation d'`ui_icons` était le bon, et son étalon de 18 — réservé au seul rouage après un
//! retour « encore trop petite » — valait pour les quatre.
//!
//! Le cas que le plan appelle « réinitialisation » — le bouton 36 × 36 que le jeu pose à droite de
//! la barre d'onglets et à droite d'un réglage isolé — attend qu'un réglage réinitialisable existe.
//! Note du relevé à ne pas « corriger » le jour venu : dans la barre d'onglets, **son axe est 9 px
//! plus bas que celui des onglets**.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::design::{assets::DsTexture, icons::DsIcon, tokens, DesignSystem};

/// Sur quoi le bouton est posé — c'est ce qui choisit son socle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IconContext {
    /// Par-dessus le jeu, dans une barre de premier plan.
    #[default]
    FirstPlan,
    /// À l'intérieur d'un panneau.
    Panel,
    /// Bouton de **pas numérique** — le carré sombre qui encadre un champ (`design::stepper`).
    ///
    /// Socle et grille d'encre lui sont propres : un aplat de 32 px sans liseré, contre 36 px et un
    /// liseré kaki pour les deux autres, et une encre de 12 au lieu de 18. Voir
    /// [`tokens::STEPPER_ICON_RATIO`], qui explique pourquoi la taille d'encre est ici une
    /// propriété du couple (asset, contexte) et non de l'asset seul.
    ///
    /// **Aucune texture survolée** : le jeu n'en a capturé aucune pour cette famille, et une teinte
    /// egui *multiplie* la texture — elle ne peut donc pas l'éclaircir. Le survol d'un pas ne change
    /// donc que la teinte de son **glyphe**, qui passe au doré ([`tokens::ICON_TINT_HOVER`]) comme
    /// dans les deux autres contextes. C'est un signal suffisant, et c'est le seul écart de ce
    /// contexte — il se corrigera le jour où une capture survolée existera.
    Stepper,
    /// Dans la **bannière d'une fenêtre** — le bouton de fermeture que `design::window` pose en
    /// haut à droite quand on le lui demande.
    ///
    /// Socle de 32 px sans texture (voir la doc de module) : un voile noir translucide, arrondi,
    /// dont seule l'opacité change au survol. Encre de 12 px, dorée au repos comme au survol.
    /// Toutes les cotes sont dans les jetons `WINDOW_CLOSE_*`.
    Banner,
}

/// Ce que peint un contexte sous le glyphe.
///
/// Trois contextes ont une texture du jeu ; le quatrième ([`IconContext::Banner`]) n'en a pas et
/// ne peut pas en avoir — son fond est celui de la fenêtre, et il ne fait que l'assombrir.
enum IconSocle {
    Texture(DsTexture, egui::Color32),
    /// Voile plein `fill`, puis liseré d'1 px `rim` à l'intérieur du bord — transparent quand
    /// l'état n'en a pas.
    Veil {
        fill: egui::Color32,
        rim: egui::Color32,
    },
}

impl IconContext {
    fn background(self, hovered: bool) -> IconSocle {
        let texture = |t| IconSocle::Texture(t, egui::Color32::WHITE);
        match (self, hovered) {
            (IconContext::FirstPlan, false) => texture(DsTexture::ButtonIconFirstPlan),
            (IconContext::FirstPlan, true) => texture(DsTexture::ButtonIconFirstPlanHover),
            (IconContext::Panel, false) => texture(DsTexture::ButtonIcon),
            (IconContext::Panel, true) => texture(DsTexture::ButtonIconHover),
            // Pas de texture survolée capturée : c'est la teinte qui change (voir `tint`).
            (IconContext::Stepper, _) => texture(DsTexture::ButtonStepper),
            (IconContext::Banner, false) => IconSocle::Veil {
                fill: tokens::WINDOW_CLOSE_FILL,
                rim: tokens::WINDOW_CLOSE_RIM,
            },
            // Le bord survolé ne se distingue pas de l'intérieur : pas de liseré.
            (IconContext::Banner, true) => IconSocle::Veil {
                fill: tokens::WINDOW_CLOSE_FILL_HOVER,
                rim: egui::Color32::TRANSPARENT,
            },
        }
    }

    /// Teinte du glyphe, quand le contexte en impose une — la même dans les DEUX états.
    ///
    /// `FirstPlan` est le seul contexte à ne rien imposer : son glyphe suit le couple gris clair →
    /// or ([`tokens::ICON_TINT`] / [`tokens::ICON_TINT_HOVER`]) mesuré sur les socles bleus de
    /// `menu-button-icon-first-plan.png`. Les trois autres figent leur teinte, chacun pour une
    /// raison mesurée :
    ///
    /// - `Panel` la garde **blanche** — retour utilisateur 2026-09-14, captures du jeu à l'appui :
    ///   sur le socle kaki, le survol change la texture du socle, jamais la couleur du glyphe (voir
    ///   [`tokens::PANEL_ICON_TINT`]) ;
    /// - le pas peint son glyphe **en or au repos** (mesuré sur `large-input-number.png`, voir
    ///   [`tokens::STEPPER_ICON_TINT`]), et faute de capture survolée l'or y reste aussi la teinte
    ///   de survol : son signal de survol est son curseur, pas sa couleur — écart assumé, à
    ///   corriger le jour où une capture existera ;
    /// - la croix de bannière est **dorée dans les deux états**, mesuré : son survol se lit sur son
    ///   voile.
    fn icon_tint(self) -> Option<egui::Color32> {
        match self {
            IconContext::Panel => Some(tokens::PANEL_ICON_TINT),
            IconContext::Stepper => Some(tokens::STEPPER_ICON_TINT),
            // Doré dans les deux états, mesuré : le survol se lit sur le voile, pas sur la croix.
            IconContext::Banner => Some(tokens::WINDOW_CLOSE_ICON_TINT),
            IconContext::FirstPlan => None,
        }
    }

    /// Côté natif du socle de ce contexte — la référence d'échelle du bouton ET de son glyphe.
    fn native_size(self) -> f32 {
        match self {
            IconContext::Stepper => tokens::STEPPER_SIZE,
            IconContext::Banner => tokens::WINDOW_CLOSE_SIZE,
            _ => tokens::ICON_BUTTON_SIZE,
        }
    }

    /// Plus grande dimension d'encre du glyphe, à la taille native de ce contexte.
    ///
    /// Le contexte l'emporte sur le manifeste **quand il en a une**, parce que le jeu peint le même
    /// glyphe à deux tailles d'encre selon la famille de socle qui le porte (voir
    /// [`tokens::STEPPER_ICON_RATIO`]). Les deux autres contextes retombent sur le manifeste : leur
    /// rendu est inchangé.
    fn content_size(self, icon: DsIcon) -> Option<f32> {
        match self {
            IconContext::Stepper => Some(tokens::STEPPER_SIZE * tokens::STEPPER_ICON_RATIO),
            IconContext::Banner => Some(tokens::WINDOW_CLOSE_ICON_CONTENT),
            _ => icon.content_size(),
        }
    }

    /// Socle désactivé, sa teinte, et celle de l'icône — **pas la même mécanique selon le
    /// contexte**, et c'est mesuré, pas arbitraire.
    ///
    /// Le jeu n'a capturé qu'une texture de socle grisé, `button-icon-disabled.png`, et elle
    /// appartient au contexte `Panel` : sa luminance moyenne est de 60, contre 81 pour
    /// `button-icon.png`, le socle actif du même contexte. Un bouton désactivé y est donc plus
    /// sombre que ses voisins, ce qu'on attend.
    ///
    /// Posée sur une barre de premier plan, cette même texture s'inverse : 60 contre 41 pour
    /// `button-icon-first-plan.png`. Le bouton désactivé devient le plus lumineux du carré et
    /// attire l'œil avant les boutons actifs — constaté sur la capture de migration du carré de
    /// contrôle du Suivi (2026-09-10), qui est exactement ce cas.
    ///
    /// En `FirstPlan`, on garde donc le socle de repos et on l'assombrit, comme le faisait
    /// `panels::icon_button` faute d'asset dédié. Ce n'est pas un repli : c'est la seule des deux
    /// mécaniques qui dit « désactivé » sur ce fond-là.
    fn disabled(self) -> (IconSocle, egui::Color32) {
        match self {
            IconContext::FirstPlan => (
                IconSocle::Texture(DsTexture::ButtonIconFirstPlan, tokens::DISABLED_DIM),
                tokens::ICON_TINT_DISABLED,
            ),
            IconContext::Panel => (
                IconSocle::Texture(DsTexture::ButtonIconDisabled, egui::Color32::WHITE),
                tokens::TEXT_DISABLED,
            ),
            // Même mécanique que `FirstPlan`, et pour la même raison : aucun socle grisé n'existe
            // pour cette famille, et celui du contexte `Panel` porte un liseré kaki qui jurerait.
            IconContext::Stepper => (
                IconSocle::Texture(DsTexture::ButtonStepper, tokens::DISABLED_DIM),
                tokens::ICON_TINT_DISABLED,
            ),
            // Le jeu ne désactive jamais sa croix de fermeture : aucune capture, donc le voile de
            // repos et la croix effacée — l'état existe parce que le contrat l'exige, pas parce
            // qu'un appelant le demande.
            IconContext::Banner => (
                IconSocle::Veil {
                    fill: tokens::WINDOW_CLOSE_FILL,
                    rim: tokens::WINDOW_CLOSE_RIM,
                },
                tokens::ICON_TINT_DISABLED,
            ),
        }
    }
}

/// État visuel — les trois du contrat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconButtonState {
    Idle,
    Hovered,
    Disabled,
}

/// Construit un bouton icône. `icon` est une texture du manifeste, jamais un fichier.
pub fn icon_button(icon: DsIcon) -> IconButton {
    IconButton::new(icon)
}

pub struct IconButton {
    icon: DsIcon,
    context: IconContext,
    size: f32,
    enabled: bool,
    tooltip: Option<String>,
    log_name: Option<String>,
    forced_state: Option<IconButtonState>,
    mirrored: bool,
}

impl IconButton {
    pub fn new(icon: DsIcon) -> Self {
        Self {
            icon,
            context: IconContext::default(),
            size: tokens::ICON_BUTTON_SIZE,
            enabled: true,
            tooltip: None,
            log_name: None,
            forced_state: None,
            mirrored: false,
        }
    }

    /// Retourne le glyphe horizontalement.
    ///
    /// Le jeu réutilise un même triangle pour les deux flèches de sa pagination, l'une étant le
    /// reflet de l'autre. Un second fichier serait un doublon à retraiter le jour où le glyphe
    /// change — **le miroir n'est pas un glyphe de plus**, c'est le même vu dans l'autre sens.
    /// Réservé aux glyphes symétriques verticalement.
    pub fn mirrored(mut self, mirrored: bool) -> Self {
        self.mirrored = mirrored;
        self
    }

    pub fn context(mut self, context: IconContext) -> Self {
        self.context = context;
        self
    }

    /// Côté du bouton. **36 px par défaut, la taille native des cinq socles** — toute autre valeur
    /// met le socle ET l'icône à l'échelle dans le même rapport.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Nom d'instance pour la journalisation (défaut : `"icon-button"`). À renseigner dès qu'une
    /// barre en porte plusieurs, sans quoi les lignes du journal sont indiscernables.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Force l'état peint — galerie et captures uniquement.
    pub fn preview_state(mut self, state: IconButtonState) -> Self {
        self.forced_state = Some(state);
        self
    }
}

impl Widget for IconButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::splat(self.size),
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        // Même condition que `design::button` et `panels::icon_button` : un appui de souris retire
        // l'apparence survolée partout dans l'overlay.
        let pointer_down = ui.input(|i| i.pointer.any_down());
        let state = self.forced_state.unwrap_or({
            if !self.enabled {
                IconButtonState::Disabled
            } else if response.hovered() && !pointer_down {
                IconButtonState::Hovered
            } else {
                IconButtonState::Idle
            }
        });

        if ui.is_rect_visible(rect) {
            let design = DesignSystem::get(ui.ctx());
            let (socle, icon_tint) = match state {
                IconButtonState::Idle => (
                    self.context.background(false),
                    self.context.icon_tint().unwrap_or(tokens::ICON_TINT),
                ),
                IconButtonState::Hovered => (
                    self.context.background(true),
                    self.context.icon_tint().unwrap_or(tokens::ICON_TINT_HOVER),
                ),
                IconButtonState::Disabled => self.context.disabled(),
            };
            match socle {
                IconSocle::Texture(texture, tint) => {
                    design.paint(ui.painter(), rect, texture, tint)
                }
                IconSocle::Veil { fill, rim } => {
                    // Le rayon suit l'échelle du socle, comme le glyphe : à 32 px il vaut sa
                    // mesure, ailleurs la même proportion.
                    let radius = f32::from(tokens::WINDOW_CLOSE_RADIUS) * rect.width()
                        / self.context.native_size();
                    ui.painter().rect_filled(rect, radius, fill);
                    if rim.a() > 0 {
                        ui.painter().rect_stroke(
                            rect,
                            radius,
                            egui::Stroke::new(1.0, rim),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            }

            let icon_size = icon_draw_size(
                design.icon_native_size(self.icon),
                self.context.content_size(self.icon),
                rect.width(),
                self.context.native_size(),
            );
            let icon_rect = egui::Rect::from_center_size(rect.center(), icon_size);
            design.paint_icon_flipped(ui.painter(), icon_rect, self.icon, icon_tint, self.mirrored);
        }

        if response.clicked() {
            tracing::debug!(
                component = "icon_button",
                name = self.log_name.as_deref().unwrap_or("icon-button"),
                "clic"
            );
        }

        let response = if self.enabled {
            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            response.on_hover_cursor(egui::CursorIcon::Default)
        };
        // L'infobulle passe par le composant du design system, jamais par `on_hover_text` :
        // celui-ci aligne en `RectAlign::BOTTOM_START` (sous l'élément, voir `Popup::new`) et
        // laisse au libellé le gris d'egui. Voir `components::tooltip`.
        if let Some(tooltip) = self.tooltip {
            crate::design::tooltip(&response).text(tooltip);
        }
        response
    }
}

/// Met un glyphe à l'échelle **en conservant ses proportions**, pour tenir dans un carré de côté
/// `box_side`.
///
/// C'est la règle de mise à l'échelle d'un glyphe du design system, et il ne doit y en avoir
/// qu'une : peindre dans un carré (`Vec2::splat`) déforme tout glyphe qui n'en est pas un — le
/// trait du moins (14 × 2) devient un pavé, un chevron (14 × 8) un carré. Trois occurrences de ce
/// défaut ont été relevées pendant la revue des maquettes de la page Alertes, dont une **dans le
/// design system** : `Input::leading_icon` peignait son ornement en `Vec2::splat`, sans effet
/// visible tant que la loupe (24 × 24) était le seul glyphe passé, mais faux pour tout autre.
///
/// Extraite d'[`icon_draw_size`] pour cette raison — elle en est le cœur, et le seul appelant
/// n'était plus le bouton icône.
///
/// C'est le cas **carré** de [`crate::design::fit::contain`], la règle commune à tout ce qui se
/// peint à son rapport — glyphes du design system ici, images de contenu venues du CDN là. Une
/// seule des deux calcule, l'autre l'appelle : deux copies de cette division finiraient par
/// diverger.
pub fn glyph_fit(native: Vec2, box_side: f32) -> Vec2 {
    crate::design::fit::contain(Vec2::splat(box_side), native)
}

/// Taille à laquelle peindre l'icône sur un bouton de côté `button_size`.
///
/// Le socle et l'icône partagent le **même facteur d'échelle**, dérivé de la largeur du socle : une
/// icône reste proportionnée à son bouton à n'importe quelle taille.
///
/// `content` — la taille d'encre du contexte, à défaut celle du manifeste — ramène en plus la plus
/// grande dimension du glyphe à l'étalon du jeu **avant** cette mise à l'échelle. Sans elle, une
/// icône est peinte à sa taille de fichier, qui varie d'un glyphe détouré à l'autre.
///
/// `reference` est le côté NATIF du socle de ce contexte (36 pour un bouton icône, 32 pour un pas) :
/// c'est lui qui fixe l'échelle, pas une constante globale — les deux familles de socles du jeu
/// n'ont pas la même taille native.
///
/// Fonction libre plutôt que corps de `Widget::ui` : c'est le seul calcul du composant qui peut se
/// tromper en silence, et il s'éprouve sans GPU (voir les tests en bas de ce fichier).
pub(super) fn icon_draw_size(
    native: Vec2,
    content: Option<f32>,
    button_size: f32,
    reference: f32,
) -> Vec2 {
    let scale = button_size / reference;
    match content {
        // Rapport commun aux deux axes : une icône normalisée garde ses proportions.
        Some(target) => glyph_fit(native, target) * scale,
        None => native * scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tolérance de comparaison — ces tailles finissent en coordonnées de peinture flottantes, pas
    /// en pixels entiers ; un centième suffit à attraper une erreur de formule.
    const EPS: f32 = 0.01;

    /// **Le glyphe d'un bouton de panneau est blanc, au repos comme au survol** (retour utilisateur
    /// 2026-09-14, captures du jeu à l'appui) : seul son socle change sous la souris. Le test porte
    /// sur `icon_tint`, qui est appliquée aux DEUX états par `Widget::ui` — une teinte rendue ici
    /// est donc par construction la même au repos et au survol.
    #[test]
    fn le_glyphe_d_un_bouton_de_panneau_reste_blanc() {
        assert_eq!(
            IconContext::Panel.icon_tint(),
            Some(egui::Color32::WHITE),
            "le socle kaki ne dore pas son glyphe au survol",
        );
    }

    /// Le pendant du test précédent : le premier plan est le seul contexte à laisser `Widget::ui`
    /// choisir la teinte selon l'état, donc le seul dont le glyphe vire à l'or sous la souris.
    #[test]
    fn seul_le_premier_plan_dore_son_glyphe_au_survol() {
        assert_eq!(IconContext::FirstPlan.icon_tint(), None);
        for context in [
            IconContext::Panel,
            IconContext::Stepper,
            IconContext::Banner,
        ] {
            assert!(
                context.icon_tint().is_some(),
                "{context:?} doit figer la teinte de son glyphe",
            );
        }
    }

    /// Les quatre glyphes du carré de contrôle du Suivi, à leur taille de fichier (détourés au
    /// pixel près par le skill `design-asset`, donc canevas = encre). Recopiés ici plutôt que lus
    /// par `DesignSystem` : ce test ne doit dépendre d'aucun contexte egui ni d'aucun GPU.
    const GLYPHES: [(&str, Vec2); 4] = [
        ("icon-plus", Vec2::new(14.0, 14.0)),
        ("icon-minus", Vec2::new(14.0, 2.0)),
        ("icon-external-link", Vec2::new(13.0, 13.0)),
        ("icon-option", Vec2::new(16.0, 15.0)),
    ];

    #[test]
    fn une_icone_normalisee_atteint_l_etalon_du_jeu() {
        for (nom, native) in GLYPHES {
            let peinte = icon_draw_size(
                native,
                Some(tokens::ICON_BUTTON_CONTENT),
                24.0,
                tokens::ICON_BUTTON_SIZE,
            );
            let attendu = tokens::ICON_BUTTON_CONTENT * 24.0 / tokens::ICON_BUTTON_SIZE;
            assert!(
                (peinte.x.max(peinte.y) - attendu).abs() < EPS,
                "{nom} : plus grande dimension {} au lieu de {attendu}",
                peinte.x.max(peinte.y),
            );
        }
    }

    #[test]
    fn une_icone_normalisee_garde_ses_proportions() {
        for (nom, native) in GLYPHES {
            let peinte = icon_draw_size(
                native,
                Some(tokens::ICON_BUTTON_CONTENT),
                24.0,
                tokens::ICON_BUTTON_SIZE,
            );
            assert!(
                (peinte.x / peinte.y - native.x / native.y).abs() < EPS,
                "{nom} : rapport d'aspect {} au lieu de {}",
                peinte.x / peinte.y,
                native.x / native.y,
            );
        }
    }

    /// Le « − » est le cas qui se serait cassé en silence : 2 px de haut à l'origine, il ne survit
    /// à la normalisation que si le facteur s'applique aux DEUX axes. Une normalisation qui
    /// n'agirait que sur la plus grande dimension en ferait une barre.
    #[test]
    fn le_trait_du_moins_reste_un_trait() {
        let peinte = icon_draw_size(
            Vec2::new(14.0, 2.0),
            Some(18.0),
            24.0,
            tokens::ICON_BUTTON_SIZE,
        );
        assert!((peinte.x - 12.0).abs() < EPS, "largeur {}", peinte.x);
        assert!(
            (peinte.y - 12.0 * 2.0 / 14.0).abs() < EPS,
            "hauteur {}",
            peinte.y
        );
    }

    /// Sans étalon, on retombe exactement sur la taille de fichier mise à l'échelle du socle — le
    /// comportement de toutes les textures qui ne sont pas des icônes de bouton.
    #[test]
    fn sans_etalon_l_icone_garde_sa_taille_de_fichier() {
        for (nom, native) in GLYPHES {
            let peinte = icon_draw_size(native, None, 24.0, tokens::ICON_BUTTON_SIZE);
            let attendu = native * (24.0 / tokens::ICON_BUTTON_SIZE);
            assert!(
                (peinte - attendu).length() < EPS,
                "{nom} : {peinte:?} au lieu de {attendu:?}",
            );
        }
    }

    /// À la taille native du socle, l'étalon est la taille peinte, sans conversion.
    #[test]
    fn au_socle_natif_l_etalon_est_la_taille_peinte() {
        let peinte = icon_draw_size(
            Vec2::new(13.0, 13.0),
            Some(tokens::ICON_BUTTON_CONTENT),
            tokens::ICON_BUTTON_SIZE,
            tokens::ICON_BUTTON_SIZE,
        );
        assert!((peinte.x - tokens::ICON_BUTTON_CONTENT).abs() < EPS);
        assert!((peinte.y - tokens::ICON_BUTTON_CONTENT).abs() < EPS);
    }

    /// **Les deux familles de socles du jeu n'ont pas la même grille d'encre**, et c'est mesuré :
    /// un « + » se peint à 18 sur un socle de bouton icône (36) et à 12 sur un socle de pas (32).
    /// Confondre les deux donnerait un glyphe de pas une fois et demie trop gros.
    #[test]
    fn la_grille_d_encre_du_pas_n_est_pas_celle_du_bouton_icone() {
        let plus = Vec2::new(14.0, 14.0);
        let sur_pas = icon_draw_size(
            plus,
            Some(tokens::STEPPER_SIZE * tokens::STEPPER_ICON_RATIO),
            tokens::STEPPER_SIZE,
            tokens::STEPPER_SIZE,
        );
        let sur_bouton = icon_draw_size(
            plus,
            Some(tokens::ICON_BUTTON_CONTENT),
            tokens::ICON_BUTTON_SIZE,
            tokens::ICON_BUTTON_SIZE,
        );
        assert!((sur_pas.x - 12.0).abs() < EPS, "encre de pas {}", sur_pas.x);
        assert!(
            (sur_bouton.x - tokens::ICON_BUTTON_CONTENT).abs() < EPS,
            "encre de bouton {}",
            sur_bouton.x,
        );
    }

    /// Le socle d'un pas rendu à une autre taille met son glyphe à l'échelle dans le même rapport —
    /// et c'est SA taille native qui sert de référence, pas celle du bouton icône.
    #[test]
    fn le_glyphe_d_un_pas_suit_la_taille_de_son_socle() {
        let etalon = tokens::STEPPER_SIZE * tokens::STEPPER_ICON_RATIO;
        for cote in [24.0_f32, 32.0, 48.0] {
            let peinte = icon_draw_size(
                Vec2::new(14.0, 14.0),
                Some(etalon),
                cote,
                tokens::STEPPER_SIZE,
            );
            let attendu = etalon * cote / tokens::STEPPER_SIZE;
            assert!(
                (peinte.x - attendu).abs() < EPS,
                "socle {cote} : encre {} au lieu de {attendu}",
                peinte.x,
            );
        }
    }
}
