//! **Titre de section** du design system Wakfu — la serif grasse grise qui ouvre un groupe de
//! réglages dans un panneau. Composant **feuille** (§1 du contrat).
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! ui.add(design::heading("Fichier"));
//! ```
//!
//! ## Les deux niveaux de titre, et ce qui les distingue
//!
//! Le jeu a deux niveaux, et **c'est la couleur qui les sépare, pas le corps** : le titre de
//! fenêtre est blanc dans sa bannière turquoise ([`design::window`](super::window) le peint),
//! celui de section est gris ([`tokens::HEADING_TEXT`]) sur le fond du panneau. Même serif, même
//! corps.
//!
//! Attention en mesurant : la hauteur d'encre d'un titre **varie avec le mot** — 25 px pour
//! « Échelle de l'interface (100%) » (accent de capitale et parenthèses descendantes), 22 px pour
//! « Thème d'interface personnalisé » (jambage seul), 14 px pour « Combat » (rien du tout).
//! *C'est la ligne de base qui est stable, pas la boîte* : comparer l'encre de deux mots
//! différents pour en déduire un rapport de corps donne un résultat faux, ce qui est arrivé une
//! fois et a produit un corps de 18 là où le relevé en donne 21.
//!
//! ## L'espace réservé est celui de l'encre
//!
//! Une galley est plus haute que son encre des deux côtés : la police y réserve la place des
//! accents de capitale au-dessus et des jambages en dessous, que la plupart des titres n'utilisent
//! ni l'un ni l'autre. Réserver la galley entière ajoutait ~14 px invisibles sous le titre, sur
//! sept relevés — et obligeait à corriger à la main le rembourrage haut du panneau.
//!
//! Ce composant réserve donc [`tokens::HEADING_INK_HEIGHT`], pas la hauteur de sa galley. C'est ce
//! qui permet aux deux cotes verticales du relevé d'être utilisées telles quelles.
//!
//! ## Un seul écart sous un titre, partout
//!
//! L'espace qu'un titre laisse sous lui est [`tokens::HEADING_TO_ROW`] — 13 px, la cote du jeu —
//! et **ce n'est pas un réglage d'écran**. Cinq écrans en décidaient autrement jusqu'au
//! 2026-09-14, chacun avec sa valeur (une demi-`SECTION_GAP` ici, un littéral là) : le même titre
//! s'ouvrait sur quatre écarts selon l'onglet où on le lisait. Seul
//! [`Heading::preview_trailing_gap`] permet encore d'en changer, pour les planches de simulation
//! qui servent à mesurer une valeur candidate — jamais pour un écran.
//!
//! ## Le retrait, qui est le seul signal de niveau
//!
//! Un titre de section est **en retrait de 7 px à gauche** par rapport aux contrôles qu'il coiffe
//! ([`tokens::PANEL_PAD_TITLE_X`] contre [`tokens::PANEL_PAD_CONTROL_X`]). Dans le jeu, c'est le
//! seul signal qu'une section existe : elle n'a ni fond, ni bordure, ni filet. Le composant
//! l'applique lui-même en peignant à gauche du rectangle qu'il alloue — il n'y a rien à faire côté
//! appelant.

use egui::{Response, Ui, Widget};

use crate::design::{text, tokens};

/// Construit un titre de section.
pub fn heading(text: impl Into<String>) -> Heading {
    Heading {
        text: text.into(),
        trailing_gap: tokens::HEADING_TO_ROW,
    }
}

/// Voir [`heading`].
pub struct Heading {
    text: String,
    trailing_gap: f32,
}

impl Heading {
    /// Remplace l'écart réservé sous le titre, **réservé aux planches de simulation** — voir
    /// `overlay-testkit/examples/interligne-options.rs`, qui rend une même section pour plusieurs
    /// valeurs candidates afin de les départager au pixel.
    ///
    /// **Aucun écran de production ne l'appelle, et aucun ne doit** : l'écart sous un titre est
    /// celui du jeu ([`tokens::HEADING_TO_ROW`]), le même partout. Il l'a longtemps été en théorie
    /// seulement — cinq écrans posaient ici leur propre demi-`SECTION_GAP` ou un littéral, et le
    /// même titre s'ouvrait sur quatre écarts différents selon l'onglet. Retirés le 2026-09-14 sur
    /// demande de l'utilisateur : « les espacements doivent être génériques ». Même esprit que
    /// `Checkbox::preview_state`.
    pub fn preview_trailing_gap(mut self, gap: f32) -> Self {
        self.trailing_gap = gap;
        self
    }
}

impl Widget for Heading {
    fn ui(self, ui: &mut Ui) -> Response {
        /// Retrait du titre par rapport à l'axe des contrôles — voir la doc de module.
        const OUTDENT: f32 = tokens::PANEL_PAD_CONTROL_X - tokens::PANEL_PAD_TITLE_X;

        let font = text::title_font(ui.ctx(), tokens::HEADING_FONT_SIZE);
        let galley =
            ui.painter()
                .layout_no_wrap(self.text.clone(), font.clone(), tokens::HEADING_TEXT);
        // Le pixel supplémentaire en largeur est celui de l'ombre portée.
        let (_, rect) = ui.allocate_space(egui::vec2(
            galley.size().x + 1.0,
            tokens::HEADING_INK_HEIGHT,
        ));
        text::paint_outlined_text(
            ui,
            rect.left_top() - egui::vec2(OUTDENT, tokens::HEADING_INK_TOP),
            egui::Align2::LEFT_TOP,
            &self.text,
            font,
            tokens::HEADING_TEXT,
            text::SHADOW_BOTTOM_RIGHT,
        );
        ui.add_space(self.trailing_gap);
        ui.interact(rect, ui.id().with("ds-heading"), egui::Sense::hover())
    }
}
