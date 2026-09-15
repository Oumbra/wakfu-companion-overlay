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
//! ## Trois textures, et pourquoi pas une
//!
//! Le châssis vient de la capture, détourée puis découpée (2026-09-15, skills `design-asset` et
//! `ui-blueprint`) :
//!
//! | Texture | Taille | Rendu |
//! | --- | --- | --- |
//! | [`DsTexture::ConfirmBody`] | 420 × 148 | 9-slice, marges 6 px, étiré sur les deux axes |
//! | [`DsTexture::ConfirmCrest`] | 250 × 57 | taille native, centrée en haut |
//! | [`DsTexture::ConfirmFoot`] | 98 × 9 | taille native, centré en bas |
//!
//! Un seul 9-slice ne pouvait pas les réunir : les deux ornements sont **centrés et de largeur
//! fixe** quand le corps s'étire. Des marges assez larges pour contenir la crête — 250 px de chaque
//! côté sur un corps de 420 — ne laissent aucune bande médiane, et la laisser dans la bande médiane
//! l'étire. C'est la règle des embouts de bouton (`docs/design-system-composants.md`), poussée
//! jusqu'à la texture séparée parce que l'ornement déborde ici du rectangle du composant.
//!
//! Avant cette date, tout était peint à la main : `rect_filled` gris, disque doré, glyphe
//! [`DsIcon::Help`][crate::design::DsIcon::Help]. La doc de ce module l'assumait — « la forme, pas
//! l'ornement à volutes, qui n'est pas détouré ». Il l'est.
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
//!
//! Ses appelants sont aujourd'hui **trois, et tous dans `panels::options_modal`** : l'installation
//! d'une mise à jour, la déconnexion du compte, et la garde de fermeture. L'onglet Alertes, lui, a
//! retiré la sienne — elle portait sur un brouillon qu'« Annuler » rattrapait. C'est ce qui rend ce
//! composant intéressant à corriger une fois : les trois ont suivi sans changer d'appel.

use egui::{Color32, Rect, Sense, Ui, Vec2};

use crate::design::{button, text, tokens, ButtonSize, ButtonVariant, DesignSystem, DsTexture};

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

        // **C'est l'ensemble qui se centre, pas le corps.** La crête déborde de 36 px au-dessus et
        // le filet de pied de 9 px en dessous : centrer le seul corps ferait porter tout ce
        // débordement d'un côté, et la boîte paraîtrait posée trop bas dans ce qu'elle interrompt.
        let crest_rise = tokens::CONFIRM_CREST_HEIGHT - tokens::CONFIRM_CREST_OVERLAP;
        let total = crest_rise + tokens::CONFIRM_HEIGHT + tokens::CONFIRM_FOOT_HEIGHT;
        let rect = Rect::from_min_size(
            egui::pos2(
                parent.center().x - tokens::CONFIRM_WIDTH / 2.0,
                parent.center().y - total / 2.0 + crest_rise,
            ),
            Vec2::new(tokens::CONFIRM_WIDTH, tokens::CONFIRM_HEIGHT),
        );

        let ds = DesignSystem::get(ui.ctx());
        ds.paint(ui.painter(), rect, DsTexture::ConfirmBody, Color32::WHITE);

        // Les deux ornements sont peints à leur **taille native**, centrés : sur la capture le
        // bandeau doré de la crête s'arrête à 250 px quand le corps en fait 420 (voir
        // `assets::CONFIRM_ORNAMENT_SLICE`). La crête passe APRÈS le corps — elle descend de 21 px
        // dedans, la pointe de son losange et le cerne sombre qui la souligne.
        ds.paint(
            ui.painter(),
            Rect::from_min_size(
                egui::pos2(
                    rect.center().x - tokens::CONFIRM_CREST_WIDTH / 2.0,
                    rect.top() - crest_rise,
                ),
                Vec2::new(tokens::CONFIRM_CREST_WIDTH, tokens::CONFIRM_CREST_HEIGHT),
            ),
            DsTexture::ConfirmCrest,
            Color32::WHITE,
        );
        ds.paint(
            ui.painter(),
            Rect::from_min_size(
                egui::pos2(
                    rect.center().x - tokens::CONFIRM_FOOT_WIDTH / 2.0,
                    rect.bottom(),
                ),
                Vec2::new(tokens::CONFIRM_FOOT_WIDTH, tokens::CONFIRM_FOOT_HEIGHT),
            ),
            DsTexture::ConfirmFoot,
            Color32::WHITE,
        );

        // **La question est mise en page, pas simplement posée.** `Painter::text` n'aurait qu'une
        // ligne, et au corps mesuré la plus longue des trois questions de `panels::options_modal`
        // déborderait du corps (voir `tokens::CONFIRM_TEXT_WIDTH`). Le jeu, lui, coupe : sa capture
        // montre deux lignes à 23 px de rythme, et c'est cet interligne qui est posé ici — celui
        // d'Ubuntu au corps 18 vaut 2 px de moins.
        let mut job = egui::text::LayoutJob::simple(
            self.question.clone(),
            text::label_light_font(ui.ctx(), tokens::CONFIRM_FONT_SIZE),
            tokens::CONFIRM_INK,
            tokens::CONFIRM_TEXT_WIDTH,
        );
        job.halign = egui::Align::Center;
        for section in &mut job.sections {
            section.format.line_height = Some(tokens::CONFIRM_LINE_HEIGHT);
        }
        let galley = ui.painter().layout_job(job);
        // `halign: Center` centre chaque ligne sur l'origine en x ; en y la galley se pose par son
        // haut, d'où la demi-hauteur retirée pour que son MILIEU tombe sur la cote du relevé —
        // laquelle est le centre du bloc, et vaut donc quel que soit le nombre de lignes.
        ui.painter().galley(
            egui::pos2(
                rect.center().x,
                rect.top() + tokens::CONFIRM_QUESTION_TOP - galley.size().y / 2.0,
            ),
            galley,
            tokens::CONFIRM_INK,
        );

        // **Les deux réponses sont POSÉES, pas mises en page.** Leurs positions sont des mesures du
        // jeu, et une rangée `horizontal()` ne les rendait pas : le `item_spacing` du `Ui` appelant
        // est hérité par le `Ui` enfant et s'ajoutait à la gouttière — la galerie, qui pose
        // `(10, 8)`, donnait 18 px pour 8 mesurés, et le groupe débordait de 5 px à droite du
        // centre. Un même dialogue prenait donc deux formes selon le panneau qui l'ouvrait. Deux
        // `put` sur des rectangles calculés retirent ce paramètre caché du calcul.
        let mut choix = ConfirmChoice::Pending;
        let row_top = rect.top() + tokens::CONFIRM_BUTTONS_TOP;
        let size = Vec2::new(tokens::CONFIRM_BUTTON_WIDTH, tokens::CONFIRM_BUTTON_HEIGHT);
        let no_rect = Rect::from_min_size(
            egui::pos2(rect.left() + tokens::CONFIRM_BUTTONS_INSET, row_top),
            size,
        );
        let yes_rect = Rect::from_min_size(
            egui::pos2(no_rect.right() + tokens::CONFIRM_BUTTON_GAP, row_top),
            size,
        );
        if ui
            .put(
                no_rect,
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
        if ui
            .put(
                yes_rect,
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
