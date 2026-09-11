//! **Filet de séparation** du design system Wakfu — les deux lignes de 2 px qui séparent
//! l'intitulé d'une section de son corps. Composant **feuille** (§1 du contrat).
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! ui.add(design::heading("Description"));
//! ui.add(design::separator());
//! ui.label("…");
//! ```
//!
//! ## Ce n'est pas un trait, c'est un bevel
//!
//! La tentation est d'écrire `painter.hline(…, Stroke::new(1.0, gris))` et de passer à autre
//! chose. Le jeu ne fait pas ça : il empile **deux lignes plates de 2 px**, une claire au-dessus
//! d'une sombre, sans dégradé entre elles. C'est une gravure — la lumière vient d'en haut, le
//! creux est en dessous — et c'est ce qui la distingue d'une bordure CSS. Rendu en un seul trait
//! gris, le filet paraît posé *sur* le fond au lieu d'y être creusé.
//!
//! ## Le bevel suit son fond, d'où [`Separator::on_hovered_surface`]
//!
//! Mesuré sur `collapse-block-opened.png` et sa variante survolée, au pixel :
//!
//! | | fond | ligne claire | ligne sombre |
//! | --- | --- | --- | --- |
//! | repos | `#28292b` | `#323335` (+10) | `#222325` (−6) |
//! | survol | `#323436` | `#3c3e3f` (+10) | `#2b2c2e` (−7) |
//!
//! Les deux lignes ne sont donc pas des couleurs absolues : ce sont des **écarts constants autour
//! de la teinte du fond**, que le jeu recalcule quand la surface s'éclaircit. Un filet resté à sa
//! teinte de repos sur un bloc survolé se voit immédiatement — la ligne claire du repos
//! (`#323335`) est à un niveau du fond survolé (`#323436`), elle y disparaît.
//!
//! Le paramètre s'appelle `on_hovered_surface` et non `hovered` parce que c'est bien ce qu'il
//! dit : **le filet ne se survole pas lui-même**. Il n'est pas cliquable, rien ne s'y passe, et
//! `response.hovered()` sur ses 4 px de haut ne renseignerait sur rien d'utile. Ce qu'il faut lui
//! dire, c'est l'état de la *surface qui le porte* — typiquement un
//! [`design::collapsible`](super::collapsible) dont l'en-tête est survolé, qui éclaircit son cadre
//! entier, contenu compris.
//!
//! Les quatre teintes sont conservées telles quelles plutôt que calculées par offset : le décalage
//! de la ligne sombre n'est pas le même dans les deux états (−6 au repos, −7 au survol) et ne suit
//! pas non plus le même écart sur les trois canaux. Deux paires mesurées disent la vérité ; une
//! formule à un paramètre l'approcherait.
//!
//! ## Ce que le composant ne fait pas : ses marges latérales
//!
//! Dans la capture, le filet court de x=16 à x=719 sur un cadre large de 732 — 16 px de marge à
//! gauche, 12 à droite. **Cette asymétrie n'est pas celle du filet, c'est celle du bloc qui le
//! porte** : le chevron de l'en-tête s'arrête exactement au même x=719, et les intitulés de
//! section commencent au même x≈17. Le filet ne fait que remplir la largeur utile qu'on lui donne.
//!
//! Le composant occupe donc toute la largeur disponible et **ne pose aucune marge latérale** — §6
//! du contrat, les marges externes appartiennent à l'appelant. Un conteneur qui veut les cotes du
//! jeu les applique lui-même, ou passe une largeur explicite à [`Separator::width`].
//!
//! ## L'écart vertical
//!
//! Relevé : 10 px entre le bas de l'encre de l'intitulé et le haut du filet, 12 px entre le bas du
//! filet et le haut de l'encre du corps. Seul le second est porté par ce composant
//! ([`tokens::SEPARATOR_GAP_BELOW`], surchargeable par [`Separator::trailing_gap`]) — même
//! répartition que [`design::heading`](super::heading), qui réserve l'écart *sous* lui et laisse
//! celui du dessus à l'élément précédent. Deux composants qui réserveraient chacun leurs deux
//! côtés doubleraient l'espace à chaque jonction.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::design::tokens;

/// Construit un filet de séparation, sur toute la largeur disponible.
pub fn separator() -> Separator {
    Separator {
        width: None,
        on_hovered_surface: false,
        trailing_gap: tokens::SEPARATOR_GAP_BELOW,
        log_name: None,
    }
}

/// Voir [`separator`].
pub struct Separator {
    width: Option<f32>,
    on_hovered_surface: bool,
    trailing_gap: f32,
    log_name: Option<String>,
}

impl Separator {
    /// Largeur imposée. Par défaut, toute la largeur disponible du `Ui`.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Déclare que la **surface portant le filet** est survolée, pour que le bevel suive son fond.
    ///
    /// Ce n'est pas le survol du filet lui-même, qui n'en a pas — voir la doc de module.
    pub fn on_hovered_surface(mut self, hovered: bool) -> Self {
        self.on_hovered_surface = hovered;
        self
    }

    /// Écart réservé sous le filet, avant la première ligne du corps. Par défaut
    /// [`tokens::SEPARATOR_GAP_BELOW`], la cote du relevé.
    pub fn trailing_gap(mut self, gap: f32) -> Self {
        self.trailing_gap = gap;
        self
    }

    /// Nom d'instance pour le journal. À renseigner dès que deux filets d'un même panneau
    /// pourraient produire la même ligne.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }
}

impl Widget for Separator {
    fn ui(self, ui: &mut Ui) -> Response {
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(width.max(0.0), tokens::SEPARATOR_HEIGHT),
            Sense::hover(),
        );

        if width <= 0.0 {
            // Rien n'est peint : sans cette ligne, un filet posé dans un `horizontal()` dont la
            // largeur est déjà consommée disparaît sans laisser de trace. Un défaut de mise en
            // page est un événement unique, pas un flux à 60 Hz.
            let warned_id = response.id.with("ds-separator-empty");
            let already = ui.ctx().data_mut(|d| {
                let seen = d.get_temp::<bool>(warned_id).unwrap_or(false);
                d.insert_temp(warned_id, true);
                seen
            });
            if !already {
                tracing::warn!(
                    component = "separator",
                    name = self.log_name.as_deref().unwrap_or("separator"),
                    largeur = width,
                    "largeur nulle ou négative, le filet n'est pas peint"
                );
            }
            ui.add_space(self.trailing_gap);
            return response;
        }

        let (light, dark) = if self.on_hovered_surface {
            (tokens::SEPARATOR_LIGHT_HOVER, tokens::SEPARATOR_DARK_HOVER)
        } else {
            (tokens::SEPARATOR_LIGHT, tokens::SEPARATOR_DARK)
        };

        let painter = ui.painter();
        let mid = rect.top() + tokens::SEPARATOR_BEVEL_HEIGHT;
        // Coins carrés : le filet du jeu est franc à ses deux extrémités, sans arrondi ni fondu —
        // les transitions mesurées sur la texture le sont sur un seul pixel.
        painter.rect_filled(
            egui::Rect::from_min_max(rect.left_top(), egui::pos2(rect.right(), mid)),
            0.0,
            light,
        );
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(rect.left(), mid), rect.right_bottom()),
            0.0,
            dark,
        );

        ui.add_space(self.trailing_gap);
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_deux_lignes_partagent_la_hauteur() {
        assert!(
            (tokens::SEPARATOR_BEVEL_HEIGHT * 2.0 - tokens::SEPARATOR_HEIGHT).abs() < f32::EPSILON,
            "le filet est exactement deux lignes de bevel empilées"
        );
    }

    #[test]
    fn le_bevel_encadre_son_fond_dans_les_deux_etats() {
        // La propriété qui fonde `on_hovered_surface` : dans chaque état, la ligne claire est
        // au-dessus du fond et la sombre en dessous. Si une mesure était recopiée de travers, le
        // filet s'inverserait ou se fondrait dans la surface.
        for (fond, clair, sombre) in [
            (
                tokens::SEPARATOR_SURFACE_REST,
                tokens::SEPARATOR_LIGHT,
                tokens::SEPARATOR_DARK,
            ),
            (
                tokens::SEPARATOR_SURFACE_HOVER,
                tokens::SEPARATOR_LIGHT_HOVER,
                tokens::SEPARATOR_DARK_HOVER,
            ),
        ] {
            assert!(clair.r() > fond.r() && clair.g() > fond.g() && clair.b() > fond.b());
            assert!(sombre.r() < fond.r() && sombre.g() < fond.g() && sombre.b() < fond.b());
        }
    }

    #[test]
    fn la_ligne_claire_du_repos_disparaitrait_sur_un_fond_survole() {
        // Le fait mesuré qui justifie d'avoir deux paires plutôt qu'une : un écart d'un seul
        // niveau, invisible à l'œil. C'est le piège que `on_hovered_surface` évite.
        let repos = tokens::SEPARATOR_LIGHT;
        let fond = tokens::SEPARATOR_SURFACE_HOVER;
        let ecart = (repos.r() as i32 - fond.r() as i32).abs()
            + (repos.g() as i32 - fond.g() as i32).abs()
            + (repos.b() as i32 - fond.b() as i32).abs();
        assert!(
            ecart <= 3,
            "la ligne claire de repos est à {ecart} niveaux cumulés du fond survolé"
        );
    }
}
