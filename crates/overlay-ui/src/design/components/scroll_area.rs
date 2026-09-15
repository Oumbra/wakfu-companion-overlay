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
//! ## La barre passe devant
//!
//! [`ScrollArea::bar_before`] la peint AVANT le contenu — au-dessus des tuiles pour une bande
//! horizontale. Demande utilisateur du 2026-09-13 au soir, deux allers-retours après la barre
//! elle-même : « je veux que la part de scroll soit au-dessus de la ligne de suivi, pas en
//! dessous [...] il faut laisser un ou deux pixels au-dessus de la barre de scroll seulement ».
//! La place ainsi prise en tête est la seule chose qui éloigne encore la bande du haut du jeu, et
//! le bas redevient libre pour les infobulles, qui retrouvent leur écart mesuré à la tuile.
//!
//! **`egui` ne sait pas le faire** : sa barre horizontale se cale sur le bord bas du rectangle de
//! la zone, et `ScrollArea::scroll_bar_rect` ne la déplace que le long de son propre axe (« for
//! instance if you are painting a sticky header on top of it »). Sa barre est donc masquée et
//! celle-ci peinte à la main : réserve en tête, poignée, survol, glissé — les proportions et le
//! geste repris de son code, teintes et épaisseur des jetons du jeu. Deux choses en sortent
//! gagnantes : la réserve n'existe que lorsque la barre sert (une bande qui tient entière ne
//! décale rien), et une mise en page peut s'y aligner ([`ScrollArea::space_before`]).
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
    bar_before: bool,
}

impl ScrollArea {
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: egui::Id::new(id_salt),
            auto_shrink: false,
            axis: ScrollAxis::Vertical,
            outer_margin: tokens::SCROLLBAR_OUTER_MARGIN,
            bar_before: false,
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

    /// **Place la barre AVANT le contenu** — au-dessus des tuiles pour une bande horizontale — au
    /// lieu d'après. Voir la doc de module, « la barre passe devant ».
    ///
    /// Horizontal uniquement pour l'instant : le seul appelant est le bandeau « Suivi », et une
    /// barre verticale à gauche n'a été demandée nulle part.
    pub fn bar_before(mut self, bar_before: bool) -> Self {
        self.bar_before = bar_before;
        self
    }

    /// Réserve totale prise par la barre sur l'axe qui lui fait face — [`RESERVE_X`] avec la marge
    /// extérieure du jeu, moins si [`ScrollArea::outer_margin`] l'a réduite.
    pub fn reserve(&self) -> f32 {
        tokens::SCROLLBAR_CONTENT_MARGIN + tokens::SCROLLBAR_WIDTH + self.outer_margin
    }

    /// Place que la barre prend AVANT le contenu **à cette frame** — [`ScrollArea::reserve`] quand
    /// elle est affichée, `0.0` sinon — et `0.0` tant que [`ScrollArea::bar_before`] n'est pas
    /// demandé.
    ///
    /// Sert à une mise en page qui doit s'aligner sur le CONTENU et non sur le haut de la zone :
    /// dans le bandeau « Suivi », le carré de contrôle descend de cette valeur pour rester à la
    /// hauteur des tuiles plutôt que de la barre. La réponse vient de la frame précédente (voir
    /// [`ScrollArea::show_output`]) : personne ne sait avant de l'avoir peinte si une bande
    /// déborde.
    pub fn space_before(&self, ui: &Ui) -> f32 {
        if self.bar_before && bar_shown_last_frame(ui, self.id_salt) {
            self.reserve()
        } else {
            0.0
        }
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
        if self.bar_before && self.axis == ScrollAxis::Horizontal {
            return self.show_bar_before(ui, add_contents);
        }
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

    /// La bande dont la barre est peinte AU-DESSUS du contenu (voir [`ScrollArea::bar_before`]).
    ///
    /// `egui` ne sait pas le faire : sa barre horizontale se cale toujours sur le bord BAS de la
    /// zone (`outer_rect.max`), et `ScrollArea::scroll_bar_rect` ne déplace la barre que le long de
    /// son propre axe — utile pour un en-tête collant, sans effet sur le côté. La barre d'egui est
    /// donc masquée (`ScrollBarVisibility::AlwaysHidden`) et celle-ci peinte à la main : la place
    /// réservée en tête, la poignée, son survol et son glissé.
    ///
    /// En échange, le composant gagne ce qu'egui ne donnait pas : la barre n'occupe la place que
    /// lorsqu'elle sert (une bande qui tient entière ne décale rien du tout), et sa position est
    /// au pixel du relevé plutôt qu'au bord d'un rectangle calculé.
    fn show_bar_before<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::scroll_area::ScrollAreaOutput<R> {
        let id_salt = self.id_salt;
        let auto_shrink = self.auto_shrink;
        let outer_margin = self.outer_margin;
        let reserve = self.reserve();
        // La place se réserve AVANT de peindre, la nécessité ne se sait qu'APRÈS : la réponse
        // vient donc de la frame précédente. Un changement d'état redemande une frame (voir plus
        // bas), le temps que la place suive — c'est exactement ce que fait `egui` avec son propre
        // `show_scroll_this_frame`.
        let shown_last_frame = bar_shown_last_frame(ui, id_salt);

        // Un `vertical` parce que ce composant est appelé DANS une rangée horizontale : `add_space`
        // y pousserait sur le mauvais axe.
        ui.vertical(|ui| {
            let top = ui.cursor().min.y;
            let mut style = (**ui.style()).clone();
            // La molette (verticale) ne pilote une zone horizontale qu'avec Maj enfoncé par
            // défaut — voir `show_output`, même raison.
            style.always_scroll_the_only_direction = true;
            ui.set_style(style);

            if shown_last_frame {
                ui.add_space(reserve);
            }
            let output = egui::ScrollArea::horizontal()
                .id_salt(id_salt)
                .auto_shrink([auto_shrink, true])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, add_contents);

            let shown = output.content_size.x > output.inner_rect.width() + 0.5;
            ui.data_mut(|data| data.insert_temp(bar_memo_id(id_salt), shown));
            if shown != shown_last_frame {
                // La place vient de changer : une frame de plus pour la prendre (ou la rendre).
                ui.ctx().request_repaint();
            }
            if shown && shown_last_frame {
                paint_bar_before(ui, &output, top + outer_margin);
            }
            output
        })
        .inner
    }
}

/// Clé sous laquelle [`ScrollArea::show_bar_before`] mémorise « la barre servait-elle ? », d'une
/// frame à l'autre.
fn bar_memo_id(id_salt: egui::Id) -> egui::Id {
    id_salt.with("design-scroll-bar-before")
}

fn bar_shown_last_frame(ui: &Ui, id_salt: egui::Id) -> bool {
    ui.data(|data| data.get_temp::<bool>(bar_memo_id(id_salt)))
        .unwrap_or(false)
}

/// Peint la poignée au-dessus du contenu et lui donne son geste — `bar_top` est l'ordonnée du HAUT
/// de la poignée, marge extérieure déjà retirée.
///
/// Les proportions sont celles d'`egui` : une poignée longue de la fraction visible du contenu,
/// posée sur sa course au prorata du défilement. Le geste aussi : glisser la poignée défile,
/// cliquer le rail ailleurs l'y amène centrée.
fn paint_bar_before<R>(ui: &mut Ui, output: &egui::scroll_area::ScrollAreaOutput<R>, bar_top: f32) {
    let rail = egui::Rect::from_min_max(
        egui::pos2(output.inner_rect.min.x, bar_top),
        egui::pos2(output.inner_rect.max.x, bar_top + tokens::SCROLLBAR_WIDTH),
    );
    let visible = output.inner_rect.width();
    let content = output.content_size.x;
    let max_offset = (content - visible).max(0.0);
    // Jamais plus courte que deux fois son épaisseur : en dessous, la poignée devient un point
    // qu'on ne peut plus viser.
    let handle_len = (visible * visible / content).max(2.0 * tokens::SCROLLBAR_WIDTH);
    let course = (visible - handle_len).max(0.0);

    let response = ui.interact(
        rail,
        output.id.with("design-scroll-bar-before"),
        egui::Sense::click_and_drag(),
    );

    let offset = output.state.offset.x.clamp(0.0, max_offset);
    let handle_x = |offset: f32| {
        rail.min.x
            + if max_offset > 0.0 {
                offset / max_offset * course
            } else {
                0.0
            }
    };
    let handle_rect = |offset: f32| {
        egui::Rect::from_min_size(
            egui::pos2(handle_x(offset), rail.min.y),
            egui::vec2(handle_len, tokens::SCROLLBAR_WIDTH),
        )
    };

    // **La position du POINTEUR, pas le cumul des déltas** — la logique d'`egui` reprise telle
    // quelle : au premier appui, on retient où dans la poignée on l'a prise (ou, si le clic tombe
    // à côté, de quoi l'amener centrée) ; ensuite l'offset se recalcule de la position absolue.
    // Un cumul de `drag_delta` perd tout mouvement arrivé dans la même frame que le relâchement,
    // ce qui suffit à rendre la barre inerte sous un harnais de test — constaté en capture.
    let prise_id = output.id.with("design-scroll-bar-prise");
    let mut nouveau = offset;
    if let Some(pointeur) = response.interact_pointer_pos() {
        let prise = ui
            .data(|data| data.get_temp::<f32>(prise_id))
            .unwrap_or_else(|| {
                let handle = handle_rect(offset);
                let prise = if handle.contains(pointeur) {
                    pointeur.x - handle.min.x
                } else {
                    // Clic dans le rail, hors poignée : elle vient se centrer là.
                    let centre =
                        (pointeur.x - handle_len / 2.0).clamp(rail.min.x, rail.max.x - handle_len);
                    pointeur.x - centre
                };
                ui.data_mut(|data| data.insert_temp(prise_id, prise));
                prise
            });
        if course > 0.0 {
            nouveau =
                ((pointeur.x - prise - rail.min.x) / course * max_offset).clamp(0.0, max_offset);
        }
    } else {
        ui.data_mut(|data| data.remove::<f32>(prise_id));
    }
    if (nouveau - offset).abs() > 0.01 {
        // `State::store` plutôt qu'un `ScrollArea::horizontal_scroll_offset` posé à la frame
        // suivante : le défilement appartient à la zone, pas à sa barre — et forcer l'offset à
        // chaque frame écraserait la molette.
        let mut state = output.state;
        state.offset.x = nouveau;
        state.store(ui.ctx(), output.id);
        ui.ctx().request_repaint();
    }

    let tenue = response.hovered() || response.is_pointer_button_down_on();
    ui.painter().rect_filled(
        handle_rect(nouveau),
        egui::CornerRadius::same(tokens::SCROLLBAR_RADIUS),
        if tenue {
            tokens::SCROLLBAR_THUMB_ACTIVE
        } else {
            tokens::SCROLLBAR_THUMB
        },
    );
}
