//! **Panneau de contenu** du design system Wakfu — l'encadré sombre qui porte le contenu d'un
//! onglet à l'intérieur d'une fenêtre. Composant **conteneur** de la famille §1 bis du contrat, en
//! *forme closure* : le contenu est écrêté à l'intérieur du panneau.
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! design::panel().show(ui, chrome.content, |ui, panel| {
//!     design::heading("Fichier").show(ui);
//!     ui.add(design::input(&mut state.path));
//!     // et, si le contenu peut déborder :
//!     panel.scroll_area(ui, "options-contenu", |ui, width| { … });
//! });
//! ```
//!
//! ## Un panneau n'est pas une section
//!
//! Le relevé est catégorique (`docs/design-system/releve-section-options.json`) : dans le jeu, une
//! **section** n'a ni fond, ni bordure, ni filet — son seul signal de regroupement est
//! l'espacement, et son seul signal de niveau le retrait de 7 px de son titre par rapport à ses
//! lignes ([`tokens::PANEL_PAD_TITLE_X`] contre [`tokens::PANEL_PAD_CONTROL_X`]). Ce qui a un fond,
//! un bord et un rayon, c'est le **panneau** qui contient les sections : ce composant.
//!
//! `panels::options_modal` appelait « section » ce qui est un panneau, et lui avait donné un rayon
//! de 18 lu à l'œil — un rayon qui n'existe nulle part dans cette interface. Le relevé en donne 2.
//!
//! ## La réserve de barre de défilement, toujours posée
//!
//! Le jeu réserve 26 px à droite du contenu **même quand la barre ne sert pas** (relevé, nœud
//! `panel`). Ce panneau fait pareil : [`ScrollArea::RESERVE_X`](super::scroll_area::RESERVE_X) est
//! déduite de la largeur utile dès le premier rendu. Le jour où le contenu déborde,
//! [`PanelZones::scroll_area`] pose la barre pile dans cette réserve, sans qu'un seul pixel de
//! contenu ne bouge.

use egui::{InnerResponse, Rect, Ui};

use crate::design::{components::scroll_area, tokens};

/// Construit un panneau de contenu au style du jeu.
pub fn panel() -> Panel {
    Panel { _private: () }
}

/// Voir [`panel`].
pub struct Panel {
    /// Réservé : ce composant n'a pas encore de paramètre, et un `struct` vide construit par une
    /// fonction libre resterait constructible sans elle depuis l'extérieur du module.
    _private: (),
}

impl Panel {
    /// Peint le panneau dans `rect` et appelle `add_contents` sur un `Ui` réduit à son intérieur —
    /// rembourrages appliqués, réserve de barre de défilement déduite, contenu **écrêté**.
    ///
    /// `add_contents` reçoit aussi les [`PanelZones`] du panneau, qui portent la zone élargie où
    /// une barre de défilement a le droit de tomber.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        rect: Rect,
        add_contents: impl FnOnce(&mut Ui, &PanelZones) -> R,
    ) -> InnerResponse<R> {
        ui.painter()
            .rect_filled(rect, tokens::PANEL_RADIUS, tokens::PANEL_FILL);
        ui.painter().rect_stroke(
            rect,
            tokens::PANEL_RADIUS,
            egui::Stroke::new(tokens::PANEL_BORDER, tokens::PANEL_BORDER_COLOR),
            egui::StrokeKind::Inside,
        );

        let inner = Rect::from_min_max(
            egui::pos2(
                rect.left() + tokens::PANEL_PAD_CONTROL_X,
                rect.top() + tokens::PANEL_PAD_TOP,
            ),
            egui::pos2(
                rect.right() - scroll_area::RESERVE_X,
                rect.bottom() - tokens::PANEL_PAD_CONTROL_X,
            ),
        );
        // La zone de défilement part du même axe de contrôles, mais va jusqu'au bord du panneau :
        // sa propre réserve y remplace le retrait de droite, au lieu de s'y ajouter.
        let zones = PanelZones {
            inner,
            scroll: Rect::from_min_max(inner.min, egui::pos2(rect.right(), inner.bottom())),
        };

        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        // Clip élargi à GAUCHE du retrait des titres, et de rien d'autre : un titre de section se
        // peint en retrait de son axe de contrôles (voir `design::heading`), et un clip calé sur
        // `inner` lui mangeait sa première lettre — « Alerte » rendait « lerte », constaté sur les
        // maquettes de la page Alertes avant que ce composant n'existe.
        //
        // `intersect` et non un `set` sec : `Ui::set_clip_rect` REMPLACE le clip de l'ancêtre, et
        // le contenu déborderait si le panneau se retrouvait un jour dans un clip plus étroit.
        // Même idiome que `design::components::input`.
        const TITLE_OUTDENT: f32 = tokens::PANEL_PAD_CONTROL_X - tokens::PANEL_PAD_TITLE_X;
        child.set_clip_rect(
            Rect::from_min_max(
                egui::pos2(inner.left() - TITLE_OUTDENT, inner.top()),
                inner.max,
            )
            .intersect(ui.clip_rect()),
        );
        // **Aucun espacement implicite.** egui glisse `item_spacing.y` (3 px par défaut) entre deux
        // widgets empilés : les écarts relevés du jeu s'en trouveraient tous majorés de 3, et un
        // `add_space` y lirait autre chose que ce qu'il produit.
        child.spacing_mut().item_spacing.y = 0.0;
        let inner_result = add_contents(&mut child, &zones);

        InnerResponse::new(
            inner_result,
            ui.interact(rect, ui.id().with("ds-panel"), egui::Sense::hover()),
        )
    }
}

/// Les zones d'un panneau, passées au contenu par [`Panel::show`].
pub struct PanelZones {
    /// Rectangle intérieur — rembourrages appliqués, réserve de défilement déduite à droite. C'est
    /// le `max_rect` du `Ui` que reçoit le contenu ; il est fourni ici pour les mises en page qui
    /// ont besoin de sa largeur autrement que par `ui.available_width()`.
    pub inner: Rect,
    /// [`PanelZones::inner`] **élargi de la réserve de barre de défilement** — privé : passer par
    /// [`PanelZones::scroll_area`], qui pose la géométrie ET l'écrêtage ensemble.
    scroll: Rect,
}

impl PanelZones {
    /// Zone défilable occupant tout ce qui reste sous le curseur courant, barre comprise.
    ///
    /// La largeur utile au contenu — réserve déduite — est passée à la closure, pour que
    /// l'appelant n'ait pas à refaire cette soustraction : trois copies s'en étaient déjà glissées
    /// dans les maquettes avant que cette méthode n'existe.
    pub fn scroll_area<R>(
        &self,
        ui: &mut Ui,
        id_salt: &str,
        add_contents: impl FnOnce(&mut Ui, f32) -> R,
    ) -> R {
        let rect = Rect::from_min_max(
            egui::pos2(self.scroll.left(), ui.cursor().min.y),
            self.scroll.max,
        );
        let content_width = rect.width() - scroll_area::RESERVE_X;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        scroll_area::scroll_area(id_salt)
            .auto_shrink(false)
            .show(&mut child, |ui| add_contents(ui, content_width))
    }
}
