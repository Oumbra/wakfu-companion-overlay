//! **Pas numérique** du design system Wakfu — le « − valeur + » du jeu. Composant **feuille**
//! (§1 du contrat) : il ne prend aucun contenu, il mute une valeur.
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! ui.add(design::stepper(&mut quantite).range(1..=999).log_name("hdv-quantite"));
//! ```
//!
//! ## Ce qu'il compose, et ce qu'il n'invente pas
//!
//! Trois composants existants, posés côte à côte : deux [`design::icon_button`](super::icon_button)
//! au contexte [`IconContext::Stepper`], et un [`design::input`](super::input) entre les deux. Rien
//! n'est repeint à la main, et le glyphe des boutons vient du manifeste — le socle aussi.
//!
//! **L'asset du jeu portait ses glyphes incrustés** (`button-plus.png`, `button-moins.png`, 34 × 34
//! chacun) : exactement l'« asset par libellé » que le skill `ui-component` interdit. Un seul socle
//! générique en a été tiré ([`DsTexture::ButtonStepper`], voir sa doc), et les deux glyphes sont
//! ceux, déjà au manifeste, du reste de l'interface.
//!
//! ## Mesures
//!
//! Les deux captures du jeu — `input-number.png` (104 × 28) et `large-input-number.png` (192 × 34) —
//! donnent les **mêmes rapports**, ce qui est la seule raison de les écrire en rapports plutôt qu'en
//! pixels : le pas existe à deux échelles dans le jeu, et il s'y met à l'échelle sans se déformer.
//!
//! | Grandeur | Rapport | Vérification |
//! | --- | --- | --- |
//! | Socle | carré, côté = hauteur du pas | 26 sur la petite capture, 34 sur la grande |
//! | Gouttière | [`tokens::STEPPER_GUTTER_RATIO`] | 26 + 7 + 38 + 7 + 26 = 104 ; 34 + 9 + 106 + 9 + 34 = 192 |
//! | Encre du glyphe | [`tokens::STEPPER_ICON_RATIO`] | 12 × 12 mesurés dans un socle de 32 |
//!
//! ## Ce qui n'est pas un pas
//!
//! **Le carré de contrôle du panneau Suivi**, malgré son « + » et son « − »
//! (`panels::watchlist::control_button_row`) : c'est une grille 2 × 2 d'actions indépendantes —
//! « + » ajoute un objet au suivi, « − » en retire un — sans champ au milieu, sur le socle de
//! premier plan (36 px) et non sur celui d'un pas (32 px). Il est déjà sur
//! [`design::icon_button`](super::icon_button), qui est le bon composant pour lui.
//!
//! La distinction n'est pas cosmétique : un pas **incrémente une valeur qu'il affiche**. Sans
//! valeur affichée entre les deux boutons, ce n'est pas un pas.
//!
//! ## Ce qu'il ne fait pas
//!
//! Il ne décide pas de ce qu'une valeur hors bornes doit devenir : il **écrête** au domaine donné
//! par [`Stepper::range`], et c'est tout. Un appelant qui veut refuser la saisie plutôt que
//! l'écrêter lit la valeur après coup — le composant ne produit aucun effet de bord, comme tous les
//! autres (§6 du contrat).

use std::ops::RangeInclusive;

use egui::{Response, Ui, Vec2, Widget};

use crate::design::{
    assets::DsTexture,
    components::icon_button::{icon_button, IconContext},
    components::input::{input, InputSize},
    tokens,
};

/// Construit un pas numérique sur `value`.
pub fn stepper(value: &mut i64) -> Stepper<'_> {
    Stepper {
        value,
        range: i64::MIN..=i64::MAX,
        step: 1,
        size: tokens::STEPPER_SIZE,
        field_width: None,
        enabled: true,
        log_name: None,
    }
}

/// Voir [`stepper`].
pub struct Stepper<'a> {
    value: &'a mut i64,
    range: RangeInclusive<i64>,
    step: i64,
    size: f32,
    field_width: Option<f32>,
    enabled: bool,
    log_name: Option<String>,
}

impl<'a> Stepper<'a> {
    /// Domaine autorisé. La valeur y est écrêtée à chaque frame — y compris celle que l'appelant
    /// fournit, qui peut venir d'une configuration hors bornes.
    pub fn range(mut self, range: RangeInclusive<i64>) -> Self {
        self.range = range;
        self
    }

    /// Incrément d'un clic. 1 par défaut.
    pub fn step(mut self, step: i64) -> Self {
        self.step = step;
        self
    }

    /// Côté des deux boutons, **et donc hauteur du pas entier** : le socle est carré dans le jeu.
    /// Par défaut [`tokens::STEPPER_SIZE`], la taille native de sa texture. La gouttière et l'encre
    /// des glyphes suivent dans le même rapport.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Largeur du champ central. Sans elle, le champ prend toute la place restante — ce que fait le
    /// jeu quand le pas occupe une ligne de formulaire.
    pub fn field_width(mut self, width: f32) -> Self {
        self.field_width = Some(width);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Nom d'instance dans `overlay-ui.<date>.log` — préfixe les deux boutons.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Taille que le pas occupera, sans le dessiner. `None` en largeur si le champ n'est pas
    /// dimensionné : elle dépend alors de la place disponible, que seul `ui` connaît.
    pub fn desired_size(&self) -> (Option<f32>, f32) {
        let width = self
            .field_width
            .map(|field| field + 2.0 * (self.size + gutter(self.size)));
        (width, self.size)
    }
}

/// Gouttière entre un bouton et le champ, pour un socle de côté `size` — voir la doc de module.
fn gutter(size: f32) -> f32 {
    size * tokens::STEPPER_GUTTER_RATIO
}

impl Widget for Stepper<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Stepper {
            value,
            range,
            step,
            size,
            field_width,
            enabled,
            log_name,
        } = self;

        let gutter = gutter(size);
        let prefix = log_name.as_deref().unwrap_or("pas");
        // Écrêtage d'entrée : la valeur fournie peut venir d'une configuration hors bornes, et un
        // pas qui afficherait 9999 sur un domaine 1..=99 mentirait sur ce que la validation fera.
        *value = (*value).clamp(*range.start(), *range.end());

        let field =
            field_width.unwrap_or_else(|| (ui.available_width() - 2.0 * (size + gutter)).max(0.0));
        let total = Vec2::new(field + 2.0 * (size + gutter), size);
        let (rect, response) = ui.allocate_exact_size(total, egui::Sense::hover());

        let square = Vec2::splat(size);
        let minus_rect = egui::Rect::from_min_size(rect.min, square);
        let plus_rect =
            egui::Rect::from_min_size(egui::pos2(rect.right() - size, rect.top()), square);

        // **Le champ d'un pas ne garde pas sa hauteur native**, contrairement à un champ posé sur
        // une ligne de formulaire : mesuré sur `large-input-number.png`, il fait 26 px dans un pas
        // de 34, et il suit donc son pas. C'est le jeu qui le dit — le premier jet lui avait laissé
        // ses 25 px natifs, ce qui le rendait visiblement trop court entre ses deux boutons.
        let field_height = size * tokens::STEPPER_FIELD_HEIGHT_RATIO;
        let field_rect =
            egui::Rect::from_center_size(rect.center(), Vec2::new(field, field_height));

        let can_decrease = enabled && *value > *range.start();
        let can_increase = enabled && *value < *range.end();

        if ui
            .put(
                minus_rect,
                icon_button(DsTexture::IconMinus)
                    .context(IconContext::Stepper)
                    .size(size)
                    .enabled(can_decrease)
                    .log_name(format!("{prefix}-moins")),
            )
            .clicked()
        {
            *value = (*value - step).max(*range.start());
        }

        // Le champ est en **lecture seule**, pas désactivé : sa valeur compte et le jeu l'écrit
        // dans son or habituel — c'est ce que distingue `Input::read_only`, ajouté pour ce cas.
        //
        // Pourquoi pas éditable : le jeu laisse saisir au clavier, mais cela demande de valider une
        // saisie partielle (« 1 », « 12 », « 12a ») à chaque frappe, ce qui est une spec à part
        // entière et sans capture de référence pour ses états d'erreur. Le pas est donc, pour
        // l'instant, purement incrémental — et ce fichier le dit plutôt que de le laisser deviner.
        let mut texte = value.to_string();
        ui.put(
            field_rect,
            input(&mut texte)
                .size(InputSize::Height(field_height))
                .width(field)
                .read_only(true)
                .enabled(enabled)
                .log_name(format!("{prefix}-valeur")),
        );

        if ui
            .put(
                plus_rect,
                icon_button(DsTexture::IconPlus)
                    .context(IconContext::Stepper)
                    .size(size)
                    .enabled(can_increase)
                    .log_name(format!("{prefix}-plus")),
            )
            .clicked()
        {
            *value = (*value + step).min(*range.end());
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tolérance de comparaison — voir `window::tests`.
    const EPS: f32 = 0.01;

    /// **Le rapport que les deux captures du jeu donnent**, et la seule raison d'écrire la
    /// gouttière en rapport plutôt qu'en pixels : le pas existe à deux échelles.
    #[test]
    fn la_gouttiere_reproduit_les_deux_captures_du_jeu() {
        // 26 + 7 + 38 + 7 + 26 = 104 (`input-number.png`)
        assert!(
            (gutter(26.0) - 7.0).abs() < 0.3,
            "petite capture : {}",
            gutter(26.0)
        );
        // 34 + 9 + 106 + 9 + 34 = 192 (`large-input-number.png`)
        assert!(
            (gutter(34.0) - 9.0).abs() < 0.3,
            "grande capture : {}",
            gutter(34.0)
        );
    }

    #[test]
    fn la_largeur_desiree_somme_les_deux_boutons_leurs_gouttieres_et_le_champ() {
        let mut valeur = 1;
        let (width, height) = stepper(&mut valeur)
            .size(34.0)
            .field_width(106.0)
            .desired_size();
        assert!((height - 34.0).abs() < EPS);
        let width = width.expect("champ dimensionné");
        assert!(
            (width - 192.0).abs() < 1.0,
            "{width} au lieu des 192 px de large-input-number.png",
        );
    }

    /// Sans largeur de champ, la largeur totale n'est pas connue avant le rendu — c'est ce que dit
    /// le `None`, et ce qui permet au pas d'occuper une ligne entière.
    #[test]
    fn sans_champ_dimensionne_la_largeur_n_est_pas_connue() {
        let mut valeur = 1;
        assert_eq!(stepper(&mut valeur).desired_size().0, None);
    }
}
