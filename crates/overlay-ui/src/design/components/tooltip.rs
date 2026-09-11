//! **Infobulle** du design system Wakfu — le fond, la marge et l'écart mesurés sur les captures du
//! jeu, et surtout **le placement**, qui est tout l'objet de ce composant.
//!
//! ```ignore
//! use overlay_ui::design::{self, TooltipSide};
//!
//! // Au-dessus, le défaut de cette interface.
//! design::tooltip(&response).text("Ajouter à la liste");
//!
//! // Sur le côté, quand un voisin ne doit pas être recouvert.
//! design::tooltip(&response).side(TooltipSide::Right).text("Options");
//!
//! // Contenu libre, si un texte ne suffit pas.
//! design::tooltip(&response).show(|ui| { ui.add(design::info_text("…")); });
//! ```
//!
//! ## Pourquoi un composant, alors que `on_hover_text` existe
//!
//! Parce qu'egui place ses infobulles **sous le curseur** par défaut, ce que l'utilisateur a jugé
//! désagréable dès le 2026-09-04 (« la tooltip apparaît sous le curseur, pas au-dessus du
//! portrait »). Trois enveloppes maison ont suivi — `combat::show_tooltip_above`,
//! `watchlist::show_tooltip_left` et `_right` — rigoureusement identiques à un alignement près,
//! chacune avec sa liste de replis recopiée. Ce composant les remplace toutes les trois.
//!
//! ## Les replis, qui portent trois retours utilisateur
//!
//! Un alignement qui ne tient pas **ne doit pas retomber n'importe où**. La liste est donc
//! ordonnée, et cet ordre est le résultat de bugs rapportés, pas une préférence :
//!
//! 1. **Le côté demandé**, centré.
//! 2. **Le même côté, réaligné** (`_START`, `_END`). Ajoutés avant les replis opposés le
//!    2026-09-06 : un widget proche d'un bord horizontal reste ainsi du bon côté, simplement
//!    décalé, tant qu'il reste de la place *de ce côté-là tout court*.
//! 3. **Le côté opposé**, en dernier recours seulement — et jamais le défaut d'egui
//!    (`BOTTOM_START`, « sous la souris »), qui est précisément ce qu'on fuit.
//!
//! Le cas qui a fait ajouter l'étape 2 : le bouton Détails du panneau Combat, collé au bord droit,
//! débordait en `TOP` centré et retombait *sous* le curseur ; son voisin Options, un peu plus loin
//! du bord, ne débordait pas et ne révélait donc jamais le problème.
//!
//! Et un repli ne résout pas tout : le switch Alliés/Ennemis, tout premier widget du panneau
//! Combat, n'avait **réellement** aucune place au-dessus de lui. Aucun alignement n'y pouvait rien
//! — la réponse est dans `render_content::COMBAT_TOP_MARGIN`, qui réserve cette place dans le
//! panneau. Un composant d'infobulle ne peut pas créer de l'espace qui n'existe pas.
//!
//! ## Le côté se choisit par colonne, pas par bouton
//!
//! Retour utilisateur du 2026-09-08 : « la souris doit pouvoir passer d'un bouton à l'autre sans
//! qu'il y ait un problème au niveau de la tooltip ». Dans le carré de contrôle du Suivi, un bouton
//! de droite qui afficherait son infobulle à gauche la poserait **par-dessus son voisin de
//! gauche**, gênant le survol de ce dernier. D'où [`TooltipSide::Left`] pour la colonne de gauche
//! et [`TooltipSide::Right`] pour celle de droite — le côté vient de la position dans la grille,
//! jamais du rôle du bouton.
//!
//! Ce placement demande de la place : `watchlist::CONTROL_TOOLTIP_RESERVE` en réserve autant des
//! deux côtés du carré. Le composant ne réserve rien lui-même (§6 du contrat : la mise en page
//! appartient au panneau).

use egui::{RectAlign, Response, Ui};

use crate::design::tokens;

/// Côté où l'infobulle se place par rapport au widget survolé.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipSide {
    /// Au-dessus — **le défaut de cette interface**, et non celui d'egui, qui place en dessous.
    #[default]
    Above,
    /// À gauche : pour un widget dont le voisin de droite ne doit pas être recouvert.
    Left,
    /// À droite : le symétrique.
    Right,
}

impl TooltipSide {
    /// Alignement principal, puis les cinq replis dans leur ordre — voir la doc de module.
    fn alignments(self) -> (RectAlign, [RectAlign; 5]) {
        match self {
            TooltipSide::Above => (
                RectAlign::TOP,
                [
                    RectAlign::TOP_START,
                    RectAlign::TOP_END,
                    RectAlign::BOTTOM,
                    RectAlign::BOTTOM_START,
                    RectAlign::BOTTOM_END,
                ],
            ),
            TooltipSide::Left => (
                RectAlign::LEFT,
                [
                    RectAlign::LEFT_START,
                    RectAlign::LEFT_END,
                    RectAlign::RIGHT,
                    RectAlign::RIGHT_START,
                    RectAlign::RIGHT_END,
                ],
            ),
            TooltipSide::Right => (
                RectAlign::RIGHT,
                [
                    RectAlign::RIGHT_START,
                    RectAlign::RIGHT_END,
                    RectAlign::LEFT,
                    RectAlign::LEFT_START,
                    RectAlign::LEFT_END,
                ],
            ),
        }
    }
}

/// Construit une infobulle attachée à `response`. Rien ne s'affiche tant que [`Tooltip::text`] ou
/// [`Tooltip::show`] n'a pas été appelé.
pub fn tooltip(response: &Response) -> Tooltip<'_> {
    Tooltip {
        response,
        side: TooltipSide::default(),
        gap: tokens::TOOLTIP_GAP,
    }
}

/// Voir [`tooltip`].
pub struct Tooltip<'a> {
    response: &'a Response,
    side: TooltipSide,
    gap: f32,
}

impl Tooltip<'_> {
    /// Côté de placement. Par défaut [`TooltipSide::Above`].
    pub fn side(mut self, side: TooltipSide) -> Self {
        self.side = side;
        self
    }

    /// Écart entre le widget et son infobulle. Par défaut [`tokens::TOOLTIP_GAP`], mesuré.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Affiche un texte simple — le cas de tous les appels de l'application.
    pub fn text(self, text: impl Into<String>) {
        let text = text.into();
        self.show(|ui| paint_label(ui, &text));
    }

    /// Affiche un contenu libre. Forme closure du contrat (§1 bis), à ceci près qu'elle ne prend
    /// pas le `Ui` appelant : une infobulle ne se pose pas dans le flux de mise en page, elle
    /// s'accroche à une `Response` déjà rendue.
    pub fn show<R>(self, add_contents: impl FnOnce(&mut Ui) -> R) {
        let (align, alternatives) = self.side.alignments();
        let mut tip = egui::Tooltip::for_enabled(self.response);
        tip.popup = tip
            .popup
            .align(align)
            .align_alternatives(&alternatives)
            .gap(self.gap);
        tip.show(add_contents);
    }
}

/// Peint un libellé d'infobulle : largeur maximale du thème, couleur du design system.
///
/// Le blanc vient de [`tokens::TOOLTIP_TEXT`], mesuré sur les captures — ce n'est pas le gris
/// qu'egui donne à un label non interactif.
///
/// **La police reste celle du thème**, et c'est le seul écart connu de ce composant au contrat, qui
/// veut un libellé mis en page par `design::text::label_font`. Le corriger est un **changement
/// visuel** : il déplacerait les six captures d'infobulle de la suite de rendu, que le plan demande
/// justement inchangées pour vérifier que cette migration ne change rien. Les deux ne peuvent pas
/// se faire dans le même commit — la police attend donc sa propre décision, et cette ligne existe
/// pour qu'on ne la prenne pas plus tard pour un oubli.
fn paint_label(ui: &mut Ui, label: &str) {
    ui.set_max_width(ui.spacing().tooltip_width);
    ui.label(egui::RichText::new(label).color(tokens::TOOLTIP_TEXT));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_cote_demande_passe_avant_ses_replis_et_le_cote_oppose_vient_en_dernier() {
        // L'ordre EST le composant : c'est lui qui porte les trois retours utilisateur. Une
        // permutation ferait retomber une infobulle du mauvais côté sans qu'aucun test de rendu
        // ne bronche — les snapshots ne montrent que le cas nominal, jamais les replis.
        for (side, principal, meme_cote, oppose) in [
            (
                TooltipSide::Above,
                RectAlign::TOP,
                [RectAlign::TOP_START, RectAlign::TOP_END],
                RectAlign::BOTTOM,
            ),
            (
                TooltipSide::Left,
                RectAlign::LEFT,
                [RectAlign::LEFT_START, RectAlign::LEFT_END],
                RectAlign::RIGHT,
            ),
            (
                TooltipSide::Right,
                RectAlign::RIGHT,
                [RectAlign::RIGHT_START, RectAlign::RIGHT_END],
                RectAlign::LEFT,
            ),
        ] {
            let (got, alts) = side.alignments();
            assert_eq!(got, principal, "{side:?} : alignement principal");
            assert_eq!(
                &alts[0..2],
                &meme_cote,
                "{side:?} : replis du même côté d'abord"
            );
            assert_eq!(
                alts[2], oppose,
                "{side:?} : le côté opposé en dernier recours"
            );
        }
    }

    #[test]
    fn aucun_repli_ne_pose_l_infobulle_sous_la_souris() {
        // `BOTTOM_START` est le défaut d'egui, celui que toute cette mécanique existe pour fuir.
        // Il reste admis comme dernier repli de `Above` — là, tomber dessous est le seul choix
        // qui reste — mais jamais pour un placement latéral, qui a deux côtés à sa disposition.
        for side in [TooltipSide::Left, TooltipSide::Right] {
            let (_, alts) = side.alignments();
            assert!(
                !alts.contains(&RectAlign::BOTTOM_START),
                "{side:?} ne doit jamais retomber sous la souris",
            );
        }
    }

    #[test]
    fn au_dessus_est_le_defaut() {
        assert_eq!(TooltipSide::default(), TooltipSide::Above);
    }
}
