//! **Zone défilable** du design system Wakfu — la barre de défilement du jeu, appliquée à une
//! `egui::ScrollArea`.
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! design::scroll_area("options-contenu").show(ui, |ui| {
//!     // le contenu qui peut déborder
//! });
//!
//! // … ou couchée, pour une bande qui défile de gauche à droite :
//! design::scroll_area("bandeau")
//!     .axis(design::ScrollAxis::Horizontal)
//!     .show(ui, |ui| { /* … */ });
//! ```
//!
//! ## Ce n'est pas un `Widget`, et c'est normal
//!
//! Seul écart au contrat de composant : il prend une **closure de contenu**, il ne peut donc pas
//! implémenter `egui::Widget`, qui rend une `Response` à partir de rien. `egui::ScrollArea` n'est pas
//! un `Widget` non plus, pour la même raison. Tout le reste du contrat s'applique : aucune texture
//! en paramètre, des jetons mesurés, une entrée dans la galerie.
//!
//! ## Ce qui a été mesuré
//!
//! Source : [`releve-modale-options.json`](../../../../../docs/design-system/releve-modale-options.json),
//! nœuds `scrollbar-thumb` et `panel`, recoupés avec `scrollbar-active.png` /
//! `scrollbar-inactive.png` (ligne y=150).
//!
//! | Grandeur | Valeur | Détail |
//! | --- | --- | --- |
//! | Poignée | **6 px de large** | x 685..691 sur la modale, x 5..10 sur les deux assets |
//! | Rayon | 3 | relevé |
//! | Rail | **aucun** | « le fond du panneau tient lieu de gouttière » |
//! | Poignée au repos | `#515356` | relevé de la modale |
//! | Poignée survolée / tirée | `#c1ad83` | `scrollbar-active.png` |
//! | Marge poignée → bord du panneau | 14 px | 691 → 705 |
//! | Marge contenu → poignée | 6 px | 679 → 685 |
//! | **Réserve totale à droite** | **26 px** | 679 → 705, « même quand la barre ne sert pas » |
//!
//! Les trois marges tombent juste : 6 + 6 + 14 = [`RESERVE_X`]. C'est ce qui permet à une mise en
//! page de réserver la place **avant** que la barre existe, sans que le contenu bouge le jour où
//! elle apparaît — voir `panels::options_modal`, qui réserve exactement cette largeur.
//!
//! ## La même barre, couchée
//!
//! Le relevé est celui d'une barre VERTICALE, la seule que la modale Options montre. Le bandeau
//! « Suivi » (`panels::watchlist`) défile, lui, à l'horizontale — et l'utilisateur a tranché le
//! 2026-09-13, capture à l'appui : c'est CETTE barre qu'il veut là aussi, « le scroll qui est
//! déjà utilisé pour la modale dans l'onglet Raccourcis », « gris quand l'utilisateur n'a pas sa
//! souris dessus et doré quand il passe sa souris dessus, c'est mieux dans l'ADN du jeu ».
//! [`ScrollAxis::Horizontal`] la couche donc sous le contenu : mêmes jetons, même épaisseur
//! constante, pas de rail — aucune mesure nouvelle, la barre du jeu ne change pas d'aspect parce
//! qu'elle change d'axe.
//!
//! Seule la marge extérieure peut avoir à bouger ([`ScrollArea::outer_margin`]) : les 14 px du
//! relevé séparent la poignée du bord d'un PANNEAU, et un bandeau posé sur le jeu n'en a pas.
//!
//! ## Ce qui n'est PAS reproduit
//!
//! L'**ombre portée de 2 px à droite de la poignée**, relevée dans le jeu. `egui::ScrollArea` peint
//! sa poignée elle-même et n'expose aucun point d'accroche pour l'ombrer ; la reproduire
//! demanderait de recalculer la position de la poignée hors d'egui, à partir de l'offset et de la
//! taille du contenu — un doublon fragile de son propre calcul, pour deux pixels sombres sur un fond
//! déjà sombre. Écart assumé, et le seul de ce composant.

use egui::Ui;

use crate::design::tokens;

/// Largeur totale que la barre réserve à droite du contenu — **la réserve du jeu, permanente**.
///
/// Le relevé est explicite : le panneau garde 26 px à droite « même quand la barre ne sert pas ».
/// Une mise en page qui n'a pas encore de contenu débordant réserve donc déjà cette largeur, et le
/// jour où elle installe une [`scroll_area`], la barre tombe pile dedans sans rien décaler.
pub const RESERVE_X: f32 =
    tokens::SCROLLBAR_CONTENT_MARGIN + tokens::SCROLLBAR_WIDTH + tokens::SCROLLBAR_OUTER_MARGIN;

/// Axe de défilement — voir la doc de module (« la même barre, couchée »).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAxis {
    /// Barre à DROITE du contenu, contenu qui défile de haut en bas. Le cas du jeu, et le défaut.
    Vertical,
    /// Barre SOUS le contenu, contenu qui défile de gauche à droite.
    Horizontal,
}

/// Construit une zone défilable au style du jeu. `id_salt` distingue deux zones du même panneau —
/// c'est lui qui porte la position de défilement d'une frame à l'autre.
pub fn scroll_area(id_salt: impl std::hash::Hash + std::fmt::Debug) -> ScrollArea {
    ScrollArea::new(id_salt)
}

pub struct ScrollArea {
    id_salt: egui::Id,
    auto_shrink: bool,
    axis: ScrollAxis,
    outer_margin: f32,
}

impl ScrollArea {
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: egui::Id::new(id_salt),
            auto_shrink: false,
            axis: ScrollAxis::Vertical,
            outer_margin: tokens::SCROLLBAR_OUTER_MARGIN,
        }
    }

    /// Laisse la zone se rétrécir à la taille de son contenu **sur l'axe qui défile**. Faux par
    /// défaut : un panneau du jeu occupe toute sa hauteur, quel que soit ce qu'il contient.
    ///
    /// L'axe CROISÉ, lui, se rétracte toujours en horizontal (une bande posée dans une rangée
    /// prend la hauteur de ses tuiles, pas celle de la fenêtre) et jamais en vertical — c'est ce
    /// que faisaient déjà les deux appelants avant que l'axe soit réglable.
    ///
    /// **Ne pas le mettre à `true` sur une zone bornée par son parent** : egui dimensionne alors
    /// la zone sur son CONTENU, ne voit plus de débordement et ne peint plus de barre du tout — le
    /// contenu sort simplement du cadre, écrêté par la fenêtre. C'est exactement ce qu'a donné la
    /// première version de la bande du bandeau « Suivi ».
    pub fn auto_shrink(mut self, auto_shrink: bool) -> Self {
        self.auto_shrink = auto_shrink;
        self
    }

    /// Couche la barre sous le contenu (voir [`ScrollAxis`] et la doc de module).
    pub fn axis(mut self, axis: ScrollAxis) -> Self {
        self.axis = axis;
        self
    }

    /// Remplace la marge entre la poignée et le bord de la zone
    /// ([`tokens::SCROLLBAR_OUTER_MARGIN`], 14 px). À ne toucher que là où il n'y a PAS de bord de
    /// panneau à respecter — voir la doc de module, cas du bandeau « Suivi ».
    pub fn outer_margin(mut self, outer_margin: f32) -> Self {
        self.outer_margin = outer_margin;
        self
    }

    /// Réserve totale prise par la barre sur l'axe qui lui fait face — [`RESERVE_X`] avec la marge
    /// extérieure du jeu, moins si [`ScrollArea::outer_margin`] l'a réduite.
    pub fn reserve(&self) -> f32 {
        tokens::SCROLLBAR_CONTENT_MARGIN + tokens::SCROLLBAR_WIDTH + self.outer_margin
    }

    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        self.show_output(ui, add_contents).inner
    }

    /// Comme [`ScrollArea::show`], mais rend la sortie complète d'`egui` — `inner_rect` (la partie
    /// RÉELLEMENT visible du contenu) est ce dont une mise en page a besoin pour se caler sur ce
    /// qu'on voit plutôt que sur ce qui est peint (voir `panels::watchlist`, dont le bouton de
    /// suppression groupée se centre sur les tuiles visibles).
    pub fn show_output<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::scroll_area::ScrollAreaOutput<R> {
        let axis = self.axis;
        let outer_margin = self.outer_margin;
        let id_salt = self.id_salt;
        let auto_shrink = self.auto_shrink;
        // Le style est posé dans un scope : il ne fuit pas vers le reste du panneau. C'est le seul
        // chemin par lequel les jetons du jeu atteignent la barre — `egui::ScrollArea` ne prend
        // aucune couleur en paramètre, elle lit `Style` au moment de peindre.
        ui.scope(|ui| {
            let style = ui.style_mut();
            let scroll = &mut style.spacing.scroll;
            scroll.floating = false;
            scroll.bar_width = tokens::SCROLLBAR_WIDTH;
            scroll.bar_inner_margin = tokens::SCROLLBAR_CONTENT_MARGIN;
            scroll.bar_outer_margin = outer_margin;
            scroll.foreground_color = false;
            // La molette (verticale) ne pilote une zone HORIZONTALE qu'avec Maj enfoncé par
            // défaut ; quand une seule direction défile, c'est une exigence sans objet — et le
            // contraire de ce qui est demandé d'un bandeau qu'on fait défiler à la molette.
            if axis == ScrollAxis::Horizontal {
                style.always_scroll_the_only_direction = true;
            }

            // **Pas de rail.** Le relevé est catégorique : « le fond du panneau tient lieu de
            // gouttière ». `extreme_bg_color` est ce qu'egui peint derrière la poignée ; le rendre
            // transparent est le seul moyen de ne rien peindre du tout. Le scope garantit que ça ne
            // touche pas le fond des champs de saisie, qui lisent le même jeton — les nôtres
            // peignent de toute façon leur propre fond (`design::input`).
            let visuals = ui.visuals_mut();
            visuals.extreme_bg_color = egui::Color32::TRANSPARENT;
            let radius = egui::CornerRadius::same(tokens::SCROLLBAR_RADIUS);
            for (widget, fill) in [
                (&mut visuals.widgets.noninteractive, tokens::SCROLLBAR_THUMB),
                (&mut visuals.widgets.inactive, tokens::SCROLLBAR_THUMB),
                (&mut visuals.widgets.hovered, tokens::SCROLLBAR_THUMB_ACTIVE),
                (&mut visuals.widgets.active, tokens::SCROLLBAR_THUMB_ACTIVE),
            ] {
                widget.bg_fill = fill;
                widget.corner_radius = radius;
            }

            let area = match axis {
                ScrollAxis::Vertical => egui::ScrollArea::vertical(),
                ScrollAxis::Horizontal => egui::ScrollArea::horizontal(),
            };
            let auto_shrink = match axis {
                ScrollAxis::Vertical => [auto_shrink, auto_shrink],
                ScrollAxis::Horizontal => [auto_shrink, true],
            };
            area.id_salt(id_salt)
                .auto_shrink(auto_shrink)
                .show(ui, add_contents)
        })
        .inner
    }
}
