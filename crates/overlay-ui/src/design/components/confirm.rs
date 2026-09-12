//! **Boîte de confirmation** du design system — la question fermée du jeu, centrée sur ce qu'elle
//! interrompt, sous un voile qui dit que le reste est inerte.
//!
//! ```ignore
//! use overlay_ui::design::{self, ConfirmChoice};
//!
//! match design::confirm_dialog("Retirer « Pierre ultime » de vos alertes ?")
//!     .over(fenetre)
//!     .log_name("alertes.retrait")
//!     .show(ui)
//! {
//!     ConfirmChoice::Yes => retirer(),
//!     ConfirmChoice::No => fermer_le_dialogue(),
//!     ConfirmChoice::Pending => {}
//! }
//! ```
//!
//! ## Ce n'est pas une popover ancrée au bouton
//!
//! Le dépôt web a `ConfirmDeleteService`, une popover collée au bouton qui l'a déclenchée. Le jeu,
//! lui, a sa propre boîte, et elle est dans les captures de référence :
//! `interface-confirm-box.png` (449 × 209) pose exactement la même forme de question. Boîte
//! **autonome et centrée**, fond gris clair, médaillon en crête qui déborde le corps, et deux
//! réponses dont la confirmation est **or**.
//!
//! **Le bouton destructeur du jeu est or, jamais rouge.** `docs/design-system.md` réserve nommément
//! le rouge au bouton « Annuler » pleine largeur d'un pied de fenêtre, et précise que « le bouton
//! "Annuler" d'une boîte de dialogue simple (`interface-confirm-box.png`, bouton "Non") reste
//! kaki/gris standard, pas rouge ». Un « Retirer » rouge est la convention web du *destructive
//! action*, pas celle du jeu.
//!
//! Le centrage règle du même coup une réserve d'ergonomie relevée sur une version antérieure de la
//! maquette : sa popover recouvrait le bouton « Valider » de la fenêtre, et son bouton de
//! confirmation tombait exactement là où « Valider » réapparaissait une fois la popover fermée —
//! un double-clic un peu vif validait la fenêtre.
//!
//! ## Écart au contrat, assumé : `show` plutôt qu'`impl Widget`
//!
//! Ce composant n'encadre aucun contenu — c'est une feuille (§1). Mais une `egui::Response` ne peut
//! pas dire *laquelle* des deux réponses a été cliquée, et ressortir le choix par un `&mut` en
//! paramètre est précisément la maladresse que §6 reproche ailleurs. Même raison que
//! `design::autocomplete` : la sortie est un [`ConfirmChoice`], nommé d'après ce qu'il est (§1 ter).
//!
//! ## D'où il vient
//!
//! Peint d'abord dans la maquette de la page Alertes (2026-09-11), puis dans `panels::alerts_tab`
//! au moment du portage. Remonté ici le 2026-09-12, quand la **garde de fermeture** de la fenêtre
//! Options lui a donné un second appelant : deux copies de cette géométrie auraient divergé au
//! premier ajustement, et le contrat veut qu'un panneau fasse de la mise en page, pas du dessin.

use egui::{Align2, Color32, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::design::{button, text, tokens, ButtonSize, ButtonVariant, DesignSystem, DsIcon};

/// Ce que l'utilisateur a répondu — ou qu'il n'a pas encore répondu.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmChoice {
    /// Le dialogue est ouvert et attend : l'appelant le repeint à la frame suivante.
    #[default]
    Pending,
    /// La réponse affirmative — celle du bouton **or**.
    Yes,
    /// La réponse négative, y compris par Échap.
    No,
}

/// Construit une boîte de confirmation posant `question`.
pub fn confirm_dialog(question: impl Into<String>) -> ConfirmDialog {
    ConfirmDialog {
        question: question.into(),
        yes: "Oui".to_string(),
        no: "Non".to_string(),
        over: None,
        log_name: None,
    }
}

/// Voir [`confirm_dialog`].
pub struct ConfirmDialog {
    question: String,
    yes: String,
    no: String,
    over: Option<Rect>,
    log_name: Option<String>,
}

impl ConfirmDialog {
    /// **Ce que la boîte interrompt** — le rectangle que le voile couvre, et sur lequel elle se
    /// centre.
    ///
    /// C'est la FENÊTRE entière qu'il faut donner, pas le panneau où vit l'appelant : un voile
    /// rogné laisserait bannière, onglets et pied de page à pleine luminosité, ce qui se lit comme
    /// « ils restent cliquables ». Sans cet appel, le voile couvre le `max_rect` du `Ui` courant,
    /// ce qui n'est correct que si l'appelant occupe déjà toute la fenêtre.
    pub fn over(mut self, rect: Rect) -> Self {
        self.over = Some(rect);
        self
    }

    /// Libellé de la réponse affirmative. Par défaut « Oui ».
    pub fn yes(mut self, label: impl Into<String>) -> Self {
        self.yes = label.into();
        self
    }

    /// Libellé de la réponse négative. Par défaut « Non ».
    pub fn no(mut self, label: impl Into<String>) -> Self {
        self.no = label.into();
        self
    }

    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Peint la boîte et rend la réponse de cette frame.
    pub fn show(self, ui: &mut Ui) -> ConfirmChoice {
        let parent = self.over.unwrap_or_else(|| ui.max_rect());
        let nom = self.log_name.as_deref().unwrap_or("confirm");

        // Tout est peint dans une couche AU-DESSUS : la boîte passe par-dessus ce qu'elle
        // interrompt, y compris un panneau dont le clip la rognerait. Et `Rect::EVERYTHING` en
        // clip, parce que le `Ui` d'où l'on vient peut être écrêté plus étroit que `parent`.
        let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(parent).layer_id(
            egui::LayerId::new(egui::Order::Foreground, egui::Id::new(("ds-confirm", nom))),
        ));
        ui.set_clip_rect(Rect::EVERYTHING);
        let ui = &mut ui;

        ui.painter().rect_filled(
            parent,
            0,
            Color32::from_black_alpha(tokens::CONFIRM_SCRIM_ALPHA),
        );
        // Le voile **avale** les clics qui passent à côté de la boîte : sans ça il ne serait qu'une
        // teinte, et ce qu'il couvre resterait réellement cliquable — l'inverse de ce qu'il annonce.
        ui.interact(
            parent,
            egui::Id::new(("ds-confirm-scrim", nom)),
            Sense::click(),
        );

        let rect = Rect::from_center_size(
            parent.center(),
            Vec2::new(tokens::CONFIRM_WIDTH, tokens::CONFIRM_HEIGHT),
        );
        ui.painter()
            .rect_filled(rect, tokens::CONFIRM_RADIUS, tokens::CONFIRM_FILL);
        ui.painter().rect_stroke(
            rect,
            tokens::CONFIRM_RADIUS,
            Stroke::new(tokens::CONFIRM_BORDER_WIDTH, tokens::CONFIRM_BORDER),
            StrokeKind::Inside,
        );

        // La crête : le médaillon du jeu déborde le haut du corps. Rendu par le glyphe d'aide du
        // manifeste sur un disque — la forme, pas l'ornement à volutes, qui n'est pas détouré.
        let crest = egui::pos2(rect.center().x, rect.top());
        ui.painter()
            .circle_filled(crest, tokens::CONFIRM_CREST_RADIUS, tokens::CONFIRM_CREST);
        ui.painter().circle_stroke(
            crest,
            tokens::CONFIRM_CREST_RADIUS,
            Stroke::new(tokens::CONFIRM_BORDER_WIDTH, tokens::CONFIRM_BORDER),
        );
        DesignSystem::get(ui.ctx()).paint_icon(
            ui.painter(),
            Rect::from_center_size(crest, Vec2::splat(tokens::CONFIRM_CREST_GLYPH)),
            DsIcon::Help,
            tokens::BUTTON_TEXT_ON_GOLD,
        );

        ui.painter().text(
            egui::pos2(rect.center().x, rect.top() + tokens::CONFIRM_QUESTION_TOP),
            Align2::CENTER_CENTER,
            &self.question,
            text::label_font(ui.ctx(), tokens::CONFIRM_FONT_SIZE),
            Color32::WHITE,
        );

        let mut choix = ConfirmChoice::Pending;
        let mut buttons = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(
            egui::pos2(
                rect.left() + tokens::CONFIRM_BUTTONS_INSET,
                rect.top() + tokens::CONFIRM_BUTTONS_TOP,
            ),
            Vec2::new(
                rect.width() - 2.0 * tokens::CONFIRM_BUTTONS_INSET,
                tokens::CONFIRM_BUTTON_HEIGHT,
            ),
        )));
        buttons.horizontal(|ui| {
            if ui
                .add(
                    button(&self.no)
                        .variant(ButtonVariant::Secondary)
                        .size(ButtonSize::Compact)
                        .width(tokens::CONFIRM_BUTTON_WIDTH)
                        .log_name(format!("{nom}-non")),
                )
                .clicked()
            {
                choix = ConfirmChoice::No;
            }
            ui.add_space(tokens::CONFIRM_BUTTON_GAP);
            if ui
                .add(
                    button(&self.yes)
                        .variant(ButtonVariant::Primary)
                        .size(ButtonSize::Compact)
                        .width(tokens::CONFIRM_BUTTON_WIDTH)
                        .log_name(format!("{nom}-oui")),
                )
                .clicked()
            {
                choix = ConfirmChoice::Yes;
            }
        });

        // **Échap répond « Non »**, jamais « Oui » : une touche ne confirme pas une action
        // destructrice. L'appelant, lui, doit s'abstenir de lire cette même touche tant qu'un
        // dialogue est ouvert — sinon le même appui ferme la boîte ET la fenêtre derrière.
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            choix = ConfirmChoice::No;
        }

        if choix != ConfirmChoice::Pending {
            tracing::debug!(
                component = "confirm_dialog",
                name = nom,
                reponse = ?choix,
                "confirmation tranchée"
            );
        }
        choix
    }
}
