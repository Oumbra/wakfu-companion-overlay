//! **Pagination** du design system Wakfu — « Page 1 / 12 » et ses deux flèches, le bloc qui
//! parcourt un tableau de l'Hôtel de Vente. Composant **feuille** (§1 du contrat), à ceci près
//! qu'il rend une sortie propre plutôt qu'une `egui::Response` : deux flèches, deux intentions
//! distinctes, qu'une seule `Response` ne saurait pas dire.
//!
//! ```ignore
//! use overlay_ui::design::{self, PaginationStep};
//!
//! match design::pagination(page, total).log_name("hdv.historique").show(ui).step {
//!     Some(PaginationStep::Previous) => page -= 1,
//!     Some(PaginationStep::Next) => page += 1,
//!     None => {}
//! }
//! ```
//!
//! ## Ce n'est pas un pied de tableau — et c'est un constat, pas une préférence
//!
//! Relevé : [`hdv-table.json`](../../../../../docs/design-system/hdv-table.json), nœud `pager`. La
//! pagination est **en bas** dans Historique et Rechercher, mais **en haut à droite** dans Mes
//! offres. Elle ne peut donc pas être une zone de [`super::table`] : c'est un bloc autonome que la
//! page place où elle veut. Il alloue exactement sa largeur naturelle et laisse la mise en page à
//! son appelant, comme le veut §6 du contrat.
//!
//! ## Ce que la capture a appris que le relevé n'avait pas vu
//!
//! Le relevé décrivait « deux flèches de 7 px espacées de 39 px ». Agrandie ×5, la zone montre
//! autre chose : **deux boutons icône de 36 × 36**, socle arrondi et hachures diagonales comprises,
//! portant chacun un triangle. Ce composant ne peint donc aucun socle — il compose deux
//! [`super::icon_button`] en contexte panneau, et les deux flèches sont **le même glyphe**
//! ([`DsIcon::TriangleRight`]), l'une retournée par `mirrored`.
//!
//! | Grandeur | Valeur | Détail |
//! | --- | --- | --- |
//! | Socle | **36 × 36** | y 903..938, exactement [`tokens::ICON_BUTTON_SIZE`] |
//! | Gouttière entre flèches | **4 px** | fin du premier socle 1216, début du second 1221 |
//! | Texte → première flèche | **7 px** | fin de l'encre 1174, début du socle 1181 |
//! | Capitale du libellé | **13 px** | le « P » de « Page », y 915..927 |
//! | Doré du libellé | `#f4d89e` | la teinte des onglets inactifs, au pixel près |
//! | Triangle désactivé | `#5c5f5f` | mesuré (92, 95, 95) |
//!
//! **Le numéro courant est doré, pas blanc** — le relevé le disait blanc. Mesure : (244, 216, 158)
//! sur ses pixels pleins, la même valeur que « Page ». Seuls la barre oblique et le total sont
//! blancs. C'est ce qui donne au bloc sa lecture : *ce qui bouge est en or, ce qui borne est en
//! blanc*.
//!
//! ## Ce qui n'est pas relevable, et ce qu'on en fait
//!
//! **Les deux flèches sont grisées sur les trois captures** — le jeu n'y a aucune page à parcourir
//! (« Page 0 / 0 »). L'apparence d'une flèche *active* n'existe donc nulle part. Plutôt que de
//! l'inventer, le composant laisse [`super::icon_button`] rendre ses états habituels : socle de
//! panneau et glyphe `ICON_TINT` au repos, doré au survol. Si une capture d'un tableau peuplé
//! arrive un jour, c'est ici qu'elle se vérifiera.
//!
//! Le socle désactivé, lui, est mesurable, et il **diverge nettement** : le jeu le peint à
//! (36, 37, 41) sur un fond à (28, 30, 34) — huit niveaux au-dessus de son fond — là où le contexte
//! `Panel` d'[`super::icon_button`] pose `ButtonIconDisabled` en pleine opacité, à 65. L'asset a été
//! détouré d'un écran plus clair, et rien dans `icon_button` ne le ramène au fond sur lequel il est
//! posé. **C'est un écart d'`icon_button`, pas de la pagination** : le corriger ici, en teintant à
//! la main, créerait un second réglage du même socle. Il se corrige là-bas, et il vaut pour les
//! quatre boutons désactivés de l'overlay.

use egui::{Response, Ui, Vec2};

use crate::design::{icons::DsIcon, text, tokens};

use super::icon_button::{icon_button, IconContext};

/// Le sens d'un pas de pagination. L'appelant décide de ce qu'il en fait — le composant ne connaît
/// ni la source des données ni la façon d'en changer de page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaginationStep {
    Previous,
    Next,
}

/// Ce que rend [`Pagination::show`].
pub struct PaginationOutcome {
    /// Le pas demandé pendant cette frame, s'il y en a un.
    pub step: Option<PaginationStep>,
    /// Interaction sur le bloc entier — pour une infobulle, ou pour savoir qu'il est survolé.
    pub response: Response,
}

/// Quelles flèches sont actives, **en données**, pour une page courante et un total.
///
/// La règle tient en une ligne, mais elle est ici et testée plutôt que dissoute dans le rendu : le
/// cas dégénéré du jeu — « Page 0 / 0 », les deux flèches grisées — est celui qu'on voit sur les
/// trois captures, et c'est exactement celui qu'un `page > 0` naïf casserait.
///
/// La numérotation est celle de l'appelant, affichée telle quelle : le composant ne renumérote
/// rien (§6 du contrat).
pub fn arrows_enabled(page: usize, total: usize) -> (bool, bool) {
    (page > 1, total > 0 && page < total)
}

/// Construit un bloc de pagination. `page` et `total` sont affichés tels quels.
pub fn pagination(page: usize, total: usize) -> Pagination {
    Pagination {
        page,
        total,
        log_name: None,
        preview_hovered: None,
    }
}

/// Voir [`pagination`].
pub struct Pagination {
    page: usize,
    total: usize,
    log_name: Option<String>,
    preview_hovered: Option<PaginationStep>,
}

impl Pagination {
    /// Nom d'instance pour la journalisation (défaut : `pagination`). À renseigner dès que deux
    /// tableaux paginés coexistent.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Force le survol d'une flèche — galerie et captures uniquement : en rendu hors écran, aucun
    /// pointeur ne survole quoi que ce soit.
    pub fn preview_hovered(mut self, step: PaginationStep) -> Self {
        self.preview_hovered = Some(step);
        self
    }

    /// Hauteur du bloc — celle d'un bouton icône, le plus haut de ses trois éléments.
    pub fn height() -> f32 {
        tokens::ICON_BUTTON_SIZE
    }

    /// Largeur des deux flèches et de leur gouttière, hors libellé.
    fn arrows_width() -> f32 {
        2.0 * tokens::ICON_BUTTON_SIZE + tokens::PAGINATION_ARROW_GAP
    }

    /// Les deux segments du libellé : ce qui bouge, puis ce qui borne.
    ///
    /// Séparés parce qu'ils n'ont pas la même couleur, et écrits d'un seul tenant côté total (« /
    /// 12 » et non « / » puis « 12 ») pour que l'espace autour de la barre soit celui de la police
    /// et non une valeur choisie.
    fn segments(&self) -> (String, String) {
        (format!("Page {}", self.page), format!(" / {}", self.total))
    }

    pub fn show(self, ui: &mut Ui) -> PaginationOutcome {
        let font = text::label_font(ui.ctx(), tokens::PAGINATION_FONT_SIZE);
        let (courant, total) = self.segments();
        let galley_courant =
            ui.painter()
                .layout_no_wrap(courant.clone(), font.clone(), tokens::PAGINATION_LABEL);
        let galley_total =
            ui.painter()
                .layout_no_wrap(total.clone(), font.clone(), tokens::PAGINATION_TOTAL);

        let label_width = galley_courant.size().x + galley_total.size().x;
        let width = label_width + tokens::PAGINATION_TEXT_GAP + Self::arrows_width();
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, Self::height()), egui::Sense::hover());

        // Le libellé est centré sur l'axe des socles : le relevé ne donne pas d'autre ancrage, et
        // c'est le seul alignement qui tienne quand la hauteur du bloc suit celle des boutons.
        let baseline_y = rect.center().y;
        let mut x = rect.left();
        for (galley, couleur) in [
            (&galley_courant, tokens::PAGINATION_LABEL),
            (&galley_total, tokens::PAGINATION_TOTAL),
        ] {
            ui.painter().galley(
                egui::pos2(x, baseline_y - galley.size().y / 2.0),
                galley.clone(),
                couleur,
            );
            x += galley.size().x;
        }

        let (prev_enabled, next_enabled) = arrows_enabled(self.page, self.total);
        let name = self
            .log_name
            .clone()
            .unwrap_or_else(|| "pagination".to_owned());

        let mut step = None;
        let mut arrow_x = rect.left() + label_width + tokens::PAGINATION_TEXT_GAP;
        // Le glyphe pointe vers la GAUCHE dans le fichier (voir `DsIcon::TriangleRight`) : c'est
        // donc « suivant » qui est retourné, pas « précédent ». La première capture l'a montré en
        // rendant les deux flèches à l'envers.
        for (sens, enabled, mirrored) in [
            (PaginationStep::Previous, prev_enabled, false),
            (PaginationStep::Next, next_enabled, true),
        ] {
            let arrow_rect = egui::Rect::from_min_size(
                egui::pos2(arrow_x, rect.top()),
                Vec2::splat(tokens::ICON_BUTTON_SIZE),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(arrow_rect));
            let mut bouton = icon_button(DsIcon::TriangleRight)
                .context(IconContext::Panel)
                .mirrored(mirrored)
                .enabled(enabled)
                .log_name(format!(
                    "{name}.{}",
                    match sens {
                        PaginationStep::Previous => "precedent",
                        PaginationStep::Next => "suivant",
                    }
                ));
            if self.preview_hovered == Some(sens) && enabled {
                bouton = bouton.preview_state(super::icon_button::IconButtonState::Hovered);
            }
            if child.add(bouton).clicked() {
                tracing::debug!(
                    component = "pagination",
                    name = name.as_str(),
                    page = self.page,
                    total = self.total,
                    sens = match sens {
                        PaginationStep::Previous => "precedent",
                        PaginationStep::Next => "suivant",
                    },
                    "clic"
                );
                step = Some(sens);
            }
            arrow_x += tokens::ICON_BUTTON_SIZE + tokens::PAGINATION_ARROW_GAP;
        }

        PaginationOutcome { step, response }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_tableau_vide_grise_les_deux_fleches() {
        // Le cas des trois captures du jeu : « Page 0 / 0 ».
        assert_eq!(arrows_enabled(0, 0), (false, false));
    }

    #[test]
    fn la_premiere_page_ne_recule_pas_et_la_derniere_n_avance_pas() {
        assert_eq!(arrows_enabled(1, 3), (false, true));
        assert_eq!(arrows_enabled(2, 3), (true, true));
        assert_eq!(arrows_enabled(3, 3), (true, false));
        // Une page unique n'est ni la précédente ni la suivante de quoi que ce soit.
        assert_eq!(arrows_enabled(1, 1), (false, false));
    }

    #[test]
    fn une_page_hors_bornes_ne_propose_pas_d_aller_plus_loin() {
        // L'appelant affiche ce qu'il veut ; le composant ne renumérote pas, mais il ne propose
        // pas non plus d'avancer au-delà du total.
        assert_eq!(arrows_enabled(7, 3), (true, false));
        assert_eq!(arrows_enabled(0, 3), (false, true));
    }

    #[test]
    fn le_libelle_se_coupe_en_deux_segments_de_couleurs_differentes() {
        let (courant, total) = pagination(3, 12).segments();
        assert_eq!(courant, "Page 3");
        assert_eq!(total, " / 12");
        // L'espace autour de la barre appartient au second segment : c'est celui de la police,
        // pas une valeur choisie.
        assert!(total.starts_with(' '));
    }

    #[test]
    fn la_geometrie_vient_des_jetons() {
        assert_eq!(Pagination::height(), tokens::ICON_BUTTON_SIZE);
        assert_eq!(
            Pagination::arrows_width(),
            2.0 * tokens::ICON_BUTTON_SIZE + tokens::PAGINATION_ARROW_GAP
        );
    }
}
