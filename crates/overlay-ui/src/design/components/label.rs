//! **Libellé élidé** du design system — un texte sur **une seule ligne**, coupé par une ellipse
//! quand il ne tient pas, et qui révèle alors son contenu entier en infobulle. Composant
//! **feuille** (§1 du contrat).
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! ui.add(
//!     design::label("Plan \"Epée de Brâkmar\"")
//!         .width(108.0)
//!         .align(egui::Align::Center)
//!         .log_name("alertes.nom"),
//! );
//! ```
//!
//! ## Pourquoi une ligne, et pas deux
//!
//! Un nom d'objet Wakfu dépasse souvent la largeur qu'on peut lui donner — les quatre
//! « Plan "Epée de … " » partagent leurs quatorze premiers caractères. Le réflexe est de le faire
//! passer à la ligne pour les distinguer. **C'est une fausse bonne idée dans ce produit** : la
//! tuile porte déjà l'icône de l'objet, et c'est elle qui lève l'ambiguïté d'un coup d'œil, bien
//! avant le texte. Passer le nom sur deux lignes fait grandir chaque tuile pour résoudre un
//! problème que l'utilisateur n'a pas — décision explicite du 2026-09-12, après un essai à deux
//! lignes justement revenu en arrière.
//!
//! Ce qui manque, quand le nom est coupé, ce n'est pas de la place : c'est **un moyen de lire la
//! suite**. D'où l'infobulle, et d'où ce composant plutôt qu'un `ui.label` par appelant.
//!
//! ## L'infobulle n'apparaît que si le texte est réellement coupé
//!
//! Un nom qui tient en entier n'a rien à révéler : une infobulle qui répète ce qui est déjà lisible
//! est du bruit. C'est la règle du dépôt web (`[tooltipOnlyIfTruncated]="true"`), et c'est egui qui
//! répond ici — `Galley::elided`, pas une comparaison de chaînes qu'un nom finissant déjà par
//! « … » mettrait en défaut.
//!
//! L'infobulle est celle du design system ([`design::tooltip`](super::tooltip)), pas le
//! `on_hover_text` d'egui : même socle, même police, même délai que partout ailleurs.

use egui::{Align, Color32, Response, Sense, Ui, Vec2, Widget};

use crate::design::{text, tokens, TooltipSide};

/// Construit un libellé élidé.
pub fn label(text: impl Into<String>) -> Label {
    Label {
        text: text.into(),
        width: None,
        size: tokens::LABEL_FONT_SIZE,
        color: tokens::LABEL_TEXT,
        align: Align::Center,
        tooltip_side: TooltipSide::default(),
        log_name: None,
    }
}

/// Voir [`label`].
pub struct Label {
    text: String,
    width: Option<f32>,
    size: f32,
    color: Color32,
    align: Align,
    tooltip_side: TooltipSide,
    log_name: Option<String>,
}

impl Label {
    /// Largeur imposée — le texte s'élide au-delà.
    ///
    /// Sans cet appel, le libellé prend la largeur disponible du `Ui`, ce qui n'est le bon
    /// comportement que dans une colonne. Dans une cellule de grille, la donner explicitement est
    /// le seul moyen d'obtenir une ellipse au bon endroit.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Corps de la police. Par défaut [`tokens::LABEL_FONT_SIZE`].
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Couleur du texte. Par défaut [`tokens::LABEL_TEXT`].
    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    /// Alignement dans la largeur imposée. Par défaut centré.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Côté de l'infobulle. Par défaut au-dessus.
    pub fn tooltip_side(mut self, side: TooltipSide) -> Self {
        self.tooltip_side = side;
        self
    }

    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Ce texte sera-t-il coupé à cette largeur — donc **ce libellé portera-t-il son infobulle** ?
    ///
    /// Sert à l'appelant qui pose sa PROPRE infobulle sur une zone plus large (une tuile, une
    /// ligne) et doit s'effacer là où celle du libellé s'affichera : deux infobulles sous le même
    /// curseur se peignent l'une sur l'autre. Sans cette réponse, l'appelant devrait soit dupliquer
    /// la mise en page, soit se taire partout — et c'est ce second choix qui a laissé une zone
    /// muette au milieu de chaque tuile, le temps d'une capture.
    pub fn elides(ui: &Ui, text: &str, width: f32, size: f32) -> bool {
        mise_en_page(ui, text, width, size, Color32::WHITE).elided
    }

    /// Distance entre le haut d'un libellé de ce corps et sa **ligne de base** — le bas des lettres
    /// sans jambage, là où l'œil voit finir le texte. Utile à un appelant qui règle un vide SOUS
    /// un libellé : la ligne (`height`) descend bien plus bas que l'encre, et un écart mesuré
    /// depuis son bord se lit trop grand (les tuiles d'alerte, 2026-09-12 : « à l'œil, entre le
    /// libellé et la bordure, il n'y a pas cinq pixels »).
    ///
    /// Lue sur le premier glyphe de la mise en page, dont `pos.y` est sa ligne de base dans la
    /// rangée (`epaint::text::Glyph::pos`), arrondie au pixel par egui — donc entière, comme tout
    /// ce qui se compare à une capture.
    pub fn baseline(ui: &Ui, size: f32) -> f32 {
        let galley = ui.painter().layout_no_wrap(
            "H".to_owned(),
            text::label_font(ui.ctx(), size),
            Color32::WHITE,
        );
        galley
            .rows
            .first()
            .and_then(|placed| {
                placed
                    .row
                    .glyphs
                    .first()
                    .map(|glyph| placed.pos.y + glyph.pos.y)
            })
            .unwrap_or_else(|| galley.size().y)
    }

    /// Hauteur qu'occupera un libellé de ce corps — utile à un appelant qui doit réserver sa place
    /// avant de l'ajouter.
    pub fn height(ui: &Ui, size: f32) -> f32 {
        ui.painter()
            .layout_no_wrap(
                "Ag".to_owned(),
                text::label_font(ui.ctx(), size),
                Color32::WHITE,
            )
            .size()
            .y
    }
}

/// La mise en page d'un libellé : **une seule ligne, ellipse au bout**.
///
/// `max_rows = 1` + `overflow_character` : c'est egui qui coupe et qui pose l'ellipse, au glyphe
/// près — un découpage maison au caractère mesurerait juste, mais recalculerait ce que la mise en
/// page sait déjà. Les galleys sont mémoïsés par `Fonts`, appeler ceci deux fois sur la même frame
/// ne remet donc pas en page deux fois.
fn mise_en_page(
    ui: &Ui,
    text: &str,
    width: f32,
    size: f32,
    color: Color32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(
        text.to_owned(),
        text::label_font(ui.ctx(), size),
        color,
        width,
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    ui.painter().layout_job(job)
}

impl Widget for Label {
    fn ui(self, ui: &mut Ui) -> Response {
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let galley = mise_en_page(ui, &self.text, width, self.size, self.color);
        let elided = galley.elided;

        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, galley.size().y), Sense::hover());
        if ui.is_rect_visible(rect) {
            // L'alignement se joue ici et pas dans le `LayoutJob` : `halign` y déplace l'ancre du
            // galley plutôt que son contenu, ce qui oblige l'appelant à savoir lequel des deux il
            // manipule. Un rectangle et un décalage se relisent.
            let x = match self.align {
                Align::Min => rect.left(),
                Align::Center => rect.center().x - galley.size().x / 2.0,
                Align::Max => rect.right() - galley.size().x,
            };
            ui.painter()
                .galley(egui::pos2(x, rect.top()), galley, self.color);
        }

        if elided {
            // Le texte entier, et seulement quand il est coupé — voir la doc de module.
            crate::design::tooltip(&response)
                .side(self.tooltip_side)
                .text(&self.text);
            tracing::trace!(
                component = "label",
                name = self.log_name.as_deref().unwrap_or("label"),
                largeur = width,
                "libellé élidé"
            );
        }
        response
    }
}
