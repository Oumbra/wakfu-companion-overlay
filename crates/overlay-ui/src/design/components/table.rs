//! **Tableau** du design system Wakfu — l'en-tête de colonnes gris et les lignes zébrées des
//! tableaux de l'Hôtel de Vente. Composant **conteneur** de la famille §1 bis du contrat, en
//! *forme closure* : l'appelant peint le contenu de chaque cellule, le tableau pose la géométrie.
//!
//! ```ignore
//! use overlay_ui::design::{self, TableBody, TableColumn};
//!
//! design::table()
//!     .column(TableColumn::fixed("Date", 110.0))
//!     .column(TableColumn::flex("Nom", 1.0))
//!     .column(TableColumn::fixed("Prix", 90.0).align(design::TableAlign::End))
//!     .body(TableBody::Rows(offres.len()))
//!     .max_height(12.0 * design::tokens::TABLE_ROW_HEIGHT)
//!     .log_name("hdv.historique")
//!     .show(ui, |row| {
//!         let offre = &offres[row.index()];
//!         row.cell(|ui| { ui.label(&offre.date); });
//!         row.cell(|ui| { ui.label(&offre.nom); });
//!         row.cell(|ui| { ui.label(offre.prix.to_string()); });
//!     });
//! ```
//!
//! ## Ce que le relevé donne, et ce qu'il ne donne pas
//!
//! Source : [`hdv-table.json`](../../../../../docs/design-system/hdv-table.json), relevé du
//! 2026-09-12 sur les trois tableaux de l'Hôtel de Vente.
//!
//! | Grandeur | Valeur | Détail |
//! | --- | --- | --- |
//! | Hauteur de ligne | **60 px** | invariant sur les trois captures, à trois origines différentes |
//! | Encre → première ligne | **7 px** | identique sur les trois |
//! | Encre d'en-tête | 14 px | y 179..193, linéale (vérifié ×4), `#b9babb` |
//! | Zébrage | blanc à 12/255 | un **éclaircissement**, voir plus bas |
//!
//! **Les trois tableaux relevés sont vides** (« 0 Objet »). Tout ce qui concerne une ligne remplie
//! — alignement des valeurs, typographie des cellules, icône d'objet de la colonne Nom, texte trop
//! long — est donc *hors du relevé*. Ce composant n'en invente rien : il **donne la cellule à
//! l'appelant** et ne peint aucun contenu. Seul [`tokens::TABLE_CELL_PAD_X`] est un choix, et il le
//! dit.
//!
//! ## Le zébrage éclaircit, il ne colore pas
//!
//! Le tableau du jeu **n'a pas de fond propre** : le décor se lit à travers. Le relevé le
//! démontre — le rapport entre bande claire et bande nue vaut 0,70 sur quatre colonnes, alors que
//! les valeurs absolues varient d'une colonne à l'autre. Une ligne sur deux porte donc un blanc
//! translucide ([`tokens::TABLE_ROW_STRIPE`]), jamais deux aplats opaques : sur un overlay posé
//! par-dessus un jeu en mouvement, deux aplats seraient faux à chaque frame.
//!
//! ## Les positions de colonne viennent de l'appelant, et c'est un constat du relevé
//!
//! Les six libellés ont partout la même largeur d'encre d'une capture à l'autre — même corps
//! partout — mais leurs abscisses varient avec la largeur du tableau **sans règle lisible** : entre
//! « Enchantement » et « Quantité » l'écart vaut 160 px dans deux captures sur trois, ailleurs rien
//! ne se répète. Le relevé conclut qu'il faut « soit une quatrième capture, soit accepter que le
//! composant impose sa propre répartition ». C'est ce second choix qui est fait ici, en le disant :
//! [`column_spans`] répartit en données — largeurs fixes d'abord, reste au prorata des poids
//! élastiques — et un test le verrouille.
//!
//! ## Les deux états que le jeu ne montre pas
//!
//! [`TableBody::Empty`] et [`TableBody::Loading`] sont **une décision de l'overlay**, pas un relevé :
//! les tableaux à « 0 Objet » du jeu sont simplement vides, sans message, et aucune capture ne
//! montre un tableau en cours de chargement. Les deux sont construits avec des éléments déjà
//! mesurés (le rouage de [`super::loader`], le gris de [`tokens::TEXT_DISABLED`]) plutôt qu'avec
//! des teintes inventées, et leurs deux hauteurs sont annoncées comme choisies dans les jetons.
//!
//! ## Ce que le tableau ne peint PAS
//!
//! La bande claire de 8 px sous le tableau, que le relevé attribuait à un « liseré bas ». Mesure de
//! contrôle : elle traverse **toute la largeur de la capture** (x 0..1278), bien au-delà des bornes
//! du tableau (x 24..1262). Elle appartient au décor de la fenêtre, pas au tableau — la peindre ici
//! aurait posé une barre claire sous n'importe quel tableau de l'overlay.
//!
//! La **pagination** n'en fait pas partie non plus : elle est en bas dans Historique et Rechercher,
//! mais en haut à droite dans Mes offres. C'est un composant autonome que la page place où elle
//! veut, pas un pied de tableau.

use egui::{vec2, Align, Layout, Rect, Response, Sense, Ui, UiBuilder};

use crate::design::{components::scroll_area, text, tokens};

/// Largeur d'une colonne — imposée, ou partagée au prorata de ce qui reste.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TableWidth {
    /// Largeur imposée, en pixels.
    Fixed(f32),
    /// Part du reste, au prorata de ce poids. `Flex(2.0)` prend deux fois `Flex(1.0)`.
    Flex(f32),
}

impl TableWidth {
    /// Ce que cette colonne réserve **avant** partage — 0 pour une élastique.
    fn fixed_px(self) -> f32 {
        match self {
            TableWidth::Fixed(px) => px.max(0.0),
            TableWidth::Flex(_) => 0.0,
        }
    }

    /// Poids de cette colonne dans le partage du reste — 0 pour une fixe.
    fn flex_weight(self) -> f32 {
        match self {
            TableWidth::Fixed(_) => 0.0,
            TableWidth::Flex(weight) => weight.max(0.0),
        }
    }
}

/// Alignement horizontal d'une colonne — libellé d'en-tête **et** contenu des cellules, les deux
/// ensemble : une colonne de nombres alignée à droite dont l'en-tête resterait à gauche se lirait
/// comme un défaut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableAlign {
    Start,
    Center,
    End,
}

impl TableAlign {
    fn egui_align(self) -> Align {
        match self {
            TableAlign::Start => Align::Min,
            TableAlign::Center => Align::Center,
            TableAlign::End => Align::Max,
        }
    }

    fn layout(self) -> Layout {
        Layout::left_to_right(Align::Center).with_main_align(self.egui_align())
    }
}

/// Une colonne : son libellé d'en-tête, sa largeur et son alignement.
#[derive(Clone, Debug)]
pub struct TableColumn {
    label: String,
    width: TableWidth,
    align: TableAlign,
}

impl TableColumn {
    /// Colonne de largeur imposée.
    pub fn fixed(label: impl Into<String>, px: f32) -> Self {
        Self {
            label: label.into(),
            width: TableWidth::Fixed(px),
            align: TableAlign::Start,
        }
    }

    /// Colonne élastique : elle prend sa part du reste, au prorata de `weight`.
    pub fn flex(label: impl Into<String>, weight: f32) -> Self {
        Self {
            label: label.into(),
            width: TableWidth::Flex(weight),
            align: TableAlign::Start,
        }
    }

    pub fn align(mut self, align: TableAlign) -> Self {
        self.align = align;
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Ce que le tableau a à montrer. **Trois cas mutuellement exclusifs**, donc un enum et non un
/// compte doublé d'un booléen — qui laisserait exprimer « en chargement avec douze lignes ».
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableBody {
    /// `n` lignes, peintes par la closure de [`Table::show`].
    Rows(usize),
    /// Rien à montrer. Le message est celui d'[`Table::empty_text`].
    Empty,
    /// Les données arrivent : le rouage du jeu, centré.
    Loading,
}

impl TableBody {
    /// Forme normalisée : **zéro ligne est un tableau vide**, pas un tableau de hauteur nulle.
    ///
    /// La règle est ici plutôt que dans le corps du rendu pour qu'elle soit testable, et pour
    /// qu'un appelant qui calcule sa mise en page à l'avance ([`Table::height`]) obtienne
    /// exactement la hauteur qui sera peinte.
    pub fn resolve(self) -> Self {
        match self {
            TableBody::Rows(0) => TableBody::Empty,
            other => other,
        }
    }

    /// Hauteur du corps, hors en-tête.
    fn height(self, row_height: f32) -> f32 {
        match self.resolve() {
            TableBody::Rows(rows) => rows as f32 * row_height,
            TableBody::Empty => tokens::TABLE_EMPTY_HEIGHT,
            TableBody::Loading => tokens::TABLE_LOADING_HEIGHT,
        }
    }
}

/// Répartition des largeurs de colonne, **en données** — voir la doc de module pour pourquoi c'est
/// le composant qui tranche et non le relevé.
///
/// La règle, dans l'ordre :
/// 1. si les largeurs imposées ne tiennent pas dans `total`, **tout** est réduit du même facteur
///    (les élastiques tombent alors à zéro) : un tableau tassé vaut mieux qu'un tableau qui déborde
///    de son panneau ;
/// 2. sinon chaque colonne imposée garde sa largeur, et le reste se partage au prorata des poids
///    élastiques ;
/// 3. **sans aucune colonne élastique, le reste n'est pas distribué** : il demeure à droite. Une
///    largeur imposée l'est vraiment, l'élargir en douce pour remplir la ligne rendrait le
///    paramètre menteur.
pub fn column_spans(columns: &[TableColumn], total: f32) -> Vec<f32> {
    let total = total.max(0.0);
    let fixed: f32 = columns.iter().map(|c| c.width.fixed_px()).sum();
    if fixed > total {
        let factor = if fixed > 0.0 { total / fixed } else { 0.0 };
        return columns
            .iter()
            .map(|c| c.width.fixed_px() * factor)
            .collect();
    }
    let weights: f32 = columns.iter().map(|c| c.width.flex_weight()).sum();
    let spare = total - fixed;
    columns
        .iter()
        .map(|c| match c.width {
            TableWidth::Fixed(px) => px.max(0.0),
            TableWidth::Flex(weight) if weights > 0.0 => spare * weight.max(0.0) / weights,
            TableWidth::Flex(_) => 0.0,
        })
        .collect()
}

/// Une ligne sur deux est éclaircie, **en commençant par la première** — relevé : la ligne 1 porte
/// la bande claire, la ligne 2 est nue.
pub fn is_striped(index: usize) -> bool {
    index.is_multiple_of(2)
}

/// Construit un tableau. Point d'entrée unique — voir la doc de module.
pub fn table() -> Table {
    Table::new()
}

/// Voir [`table`].
pub struct Table {
    columns: Vec<TableColumn>,
    body: TableBody,
    row_height: f32,
    width: Option<f32>,
    max_height: Option<f32>,
    empty_text: Option<String>,
    log_name: Option<String>,
    preview_loader_frame: Option<usize>,
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

impl Table {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            body: TableBody::Empty,
            row_height: tokens::TABLE_ROW_HEIGHT,
            width: None,
            max_height: None,
            empty_text: None,
            log_name: None,
            preview_loader_frame: None,
        }
    }

    pub fn column(mut self, column: TableColumn) -> Self {
        self.columns.push(column);
        self
    }

    pub fn columns(mut self, columns: impl IntoIterator<Item = TableColumn>) -> Self {
        self.columns.extend(columns);
        self
    }

    pub fn body(mut self, body: TableBody) -> Self {
        self.body = body;
        self
    }

    /// Message affiché quand le tableau n'a rien à montrer. Sans lui, le corps vide reste vide —
    /// ce que fait le jeu.
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty_text = Some(text.into());
        self
    }

    /// Hauteur d'une ligne. Par défaut [`tokens::TABLE_ROW_HEIGHT`], la cote du jeu — ne s'en
    /// écarter que pour un tableau qui n'est pas celui de l'Hôtel de Vente.
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = height;
        self
    }

    /// Largeur totale. Par défaut, celle dont dispose le `Ui`.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Borne la hauteur du **corps** : au-delà, il défile, en-tête figé.
    ///
    /// La barre prend la réserve permanente du jeu ([`scroll_area::RESERVE_X`], 26 px), déduite de
    /// la largeur utile des colonnes — en-tête compris, sans quoi les libellés se décaleraient
    /// d'une demi-colonne par rapport aux valeurs.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Nom d'instance pour la journalisation (défaut : `table`).
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Fige l'image du rouage de [`TableBody::Loading`] — réservé à la galerie et aux captures,
    /// où deux rendus doivent produire le même pixel.
    pub fn preview_loader_frame(mut self, frame: usize) -> Self {
        self.preview_loader_frame = Some(frame);
        self
    }

    /// Hauteur totale que le tableau occupera, **avant** de le rendre : en-tête, puis corps borné
    /// par `max_height` s'il y en a une. Un panneau qui doit réserver sa place s'en sert.
    pub fn height(&self) -> f32 {
        let body = self.body.height(self.row_height);
        let body = match self.max_height {
            Some(max) => body.min(max.max(0.0)),
            None => body,
        };
        Self::header_height() + body
    }

    /// Hauteur de l'en-tête : rembourrage haut, encre, puis l'écart à la première ligne.
    pub fn header_height() -> f32 {
        tokens::TABLE_HEADER_PAD_TOP + tokens::TABLE_HEADER_INK + tokens::TABLE_HEADER_GAP
    }

    /// Peint le tableau. `add_row` est appelée une fois par ligne, dans l'ordre.
    pub fn show(self, ui: &mut Ui, mut add_row: impl FnMut(&mut TableRow<'_>)) -> Response {
        let name = self.log_name.clone().unwrap_or_else(|| "table".to_owned());
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let body = self.body.resolve();
        // La réserve de barre est prise sur la largeur utile DÈS QU'une hauteur est bornée, que la
        // barre serve ou non : c'est la règle du jeu (`scroll_area::RESERVE_X`), et c'est ce qui
        // empêche les colonnes de sauter le jour où une ligne de plus fait apparaître la barre.
        let content_width = match self.max_height {
            Some(_) => (width - scroll_area::RESERVE_X).max(0.0),
            None => width,
        };
        let spans = column_spans(&self.columns, content_width);

        let top = ui.cursor().min;
        let header_rect = Rect::from_min_size(top, vec2(width, Self::header_height()));
        ui.allocate_rect(header_rect, Sense::hover());
        self.paint_header(ui, header_rect, &spans);

        let body_height = body.height(self.row_height);
        let body_height = match self.max_height {
            Some(max) => body_height.min(max.max(0.0)),
            None => body_height,
        };
        let body_rect = Rect::from_min_size(
            egui::pos2(top.x, header_rect.bottom()),
            vec2(width, body_height),
        );
        let response = ui.allocate_rect(body_rect, Sense::hover());
        // **Toute l'identité du tableau pend à cette réponse.** Une `Ui` fille créée sans sel
        // d'identité HÉRITE de l'identifiant de sa mère : deux tableaux posés dans le même panneau
        // donneraient alors les mêmes identifiants de ligne, et leurs interactions se
        // mélangeraient — egui l'écrit en rouge sur la capture (« Second use of widget ID … »).
        // L'identifiant automatique de cette réponse, lui, est unique par position dans l'arbre.
        let base = response.id;

        match body {
            // Corps défilable : la `scroll_area` du design system apporte la barre du jeu et son
            // écrêtage. L'en-tête, peint au-dessus, ne bouge pas.
            TableBody::Rows(rows) if self.max_height.is_some() => {
                let mut child = ui.new_child(UiBuilder::new().id_salt(base).max_rect(body_rect));
                scroll_area::scroll_area(base.with("ds-table-body")).show(&mut child, |ui| {
                    let full = Rect::from_min_size(
                        ui.cursor().min,
                        vec2(content_width, rows as f32 * self.row_height),
                    );
                    ui.allocate_rect(full, Sense::hover());
                    self.paint_rows(ui, full, rows, &spans, base, &name, &mut add_row);
                });
            }
            TableBody::Rows(rows) => {
                let mut child = ui.new_child(UiBuilder::new().id_salt(base).max_rect(body_rect));
                self.paint_rows(
                    &mut child,
                    body_rect,
                    rows,
                    &spans,
                    base,
                    &name,
                    &mut add_row,
                );
            }
            TableBody::Empty => self.paint_empty(ui, body_rect),
            TableBody::Loading => self.paint_loading(ui, body_rect),
        }

        response
    }

    fn paint_header(&self, ui: &mut Ui, rect: Rect, spans: &[f32]) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        let font = text::label_font(ui.ctx(), tokens::TABLE_HEADER_FONT_SIZE);
        // L'encre est ancrée sur sa ligne de base plutôt que centrée dans le rembourrage : c'est
        // ce que mesure le relevé (haut du tableau → haut de l'encre = 14 px).
        let baseline = rect.top() + tokens::TABLE_HEADER_PAD_TOP + tokens::TABLE_HEADER_INK;
        let mut x = rect.left();
        for (column, span) in self.columns.iter().zip(spans) {
            let cell = Rect::from_min_size(egui::pos2(x, rect.top()), vec2(*span, rect.height()));
            let inner = cell.shrink2(vec2(tokens::TABLE_CELL_PAD_X, 0.0));
            let (anchor_x, align) = match column.align {
                TableAlign::Start => (inner.left(), egui::Align2::LEFT_BOTTOM),
                TableAlign::Center => (inner.center().x, egui::Align2::CENTER_BOTTOM),
                TableAlign::End => (inner.right(), egui::Align2::RIGHT_BOTTOM),
            };
            let painter = ui.painter().with_clip_rect(inner.intersect(ui.clip_rect()));
            painter.text(
                egui::pos2(anchor_x, baseline),
                align,
                &column.label,
                font.clone(),
                tokens::TABLE_HEADER_TEXT,
            );
            x += span;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_rows(
        &self,
        ui: &mut Ui,
        rect: Rect,
        rows: usize,
        spans: &[f32],
        base: egui::Id,
        name: &str,
        add_row: &mut impl FnMut(&mut TableRow<'_>),
    ) {
        for index in 0..rows {
            let row_rect = Rect::from_min_size(
                egui::pos2(rect.left(), rect.top() + index as f32 * self.row_height),
                vec2(rect.width(), self.row_height),
            );
            if is_striped(index) && ui.is_rect_visible(row_rect) {
                ui.painter()
                    .rect_filled(row_rect, 0, tokens::TABLE_ROW_STRIPE);
            }
            let response =
                ui.interact(row_rect, base.with(("ds-table-row", index)), Sense::click());
            if response.clicked() {
                tracing::debug!(component = "table", name, ligne = index, "clic");
            }
            let mut row = TableRow {
                ui,
                rect: row_rect,
                spans,
                columns: &self.columns,
                next: 0,
                cursor_x: row_rect.left(),
                index,
                response,
                name,
                base,
            };
            add_row(&mut row);
        }
    }

    fn paint_empty(&self, ui: &mut Ui, rect: Rect) {
        let Some(message) = self.empty_text.as_deref() else {
            return;
        };
        if !ui.is_rect_visible(rect) {
            return;
        }
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            message,
            text::label_font(ui.ctx(), tokens::TABLE_EMPTY_FONT_SIZE),
            tokens::TABLE_EMPTY_TEXT,
        );
    }

    fn paint_loading(&self, ui: &mut Ui, rect: Rect) {
        let side = super::loader::LoaderSize::Medium.px();
        let loader_rect = Rect::from_center_size(rect.center(), vec2(side, side));
        let mut child = ui.new_child(UiBuilder::new().max_rect(loader_rect));
        let mut loader = super::loader::loader()
            .size(super::loader::LoaderSize::Medium)
            .log_name(format!(
                "{}.chargement",
                self.log_name.as_deref().unwrap_or("table")
            ));
        if let Some(frame) = self.preview_loader_frame {
            loader = loader.preview_frame(frame);
        }
        // Rien d'autre : le rouage porte à lui seul l'état. Un libellé « Chargement… » serait une
        // invention de plus, et le jeu n'en met pas sous le sien.
        child.add(loader);
    }
}

/// Une ligne en cours de peinture — l'appelant y verse ses cellules, colonne par colonne.
pub struct TableRow<'a> {
    ui: &'a mut Ui,
    rect: Rect,
    spans: &'a [f32],
    columns: &'a [TableColumn],
    next: usize,
    cursor_x: f32,
    index: usize,
    response: Response,
    name: &'a str,
    /// Racine d'identité du tableau — voir [`Table::show`].
    base: egui::Id,
}

impl TableRow<'_> {
    /// Rang de la ligne, à partir de 0 — c'est par lui que l'appelant retrouve sa donnée.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Interaction de la ligne entière. L'appelant y teste `.clicked()` : le composant, lui, ne
    /// décide jamais de ce qui se passe au clic (§2 du contrat).
    pub fn response(&self) -> &Response {
        &self.response
    }

    /// Rectangle de la ligne entière, décor compris.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Laisse la colonne suivante vide et passe à celle d'après.
    pub fn skip(&mut self) {
        let _ = self.take_cell();
    }

    /// Peint la colonne suivante. Le `Ui` reçu est **écrêté à la cellule** et déjà aligné selon la
    /// colonne : un contenu trop long est coupé net plutôt que de déborder sur la colonne voisine,
    /// où il passerait pour un bug de mise en page.
    pub fn cell<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let (column, inner, align) = self.take_cell()?;
        // Sel d'identité obligatoire, pour la même raison que dans `Table::show` : une `Ui` fille
        // sans sel hérite de l'identifiant de sa mère, et deux cellules d'affilée auraient alors
        // les mêmes identifiants de widget.
        let mut child = self.ui.new_child(
            UiBuilder::new()
                .id_salt(self.base.with(("ds-table-cell", self.index, column)))
                .max_rect(inner)
                .layout(align.layout()),
        );
        child.set_clip_rect(inner.intersect(self.ui.clip_rect()));
        Some(add_contents(&mut child))
    }

    /// Avance d'une colonne et rend son rang, son rectangle utile et son alignement. `None` — et
    /// un avertissement unique — quand l'appelant demande plus de cellules qu'il n'a déclaré de
    /// colonnes.
    fn take_cell(&mut self) -> Option<(usize, Rect, TableAlign)> {
        let Some(span) = self.spans.get(self.next).copied() else {
            let warned_id = self.base.with("ds-table-overflow");
            let already = self.ui.ctx().data_mut(|d| {
                let seen = d.get_temp::<bool>(warned_id).unwrap_or(false);
                d.insert_temp(warned_id, true);
                seen
            });
            if !already {
                tracing::warn!(
                    component = "table",
                    name = self.name,
                    colonnes = self.spans.len(),
                    demandee = self.next + 1,
                    "plus de cellules demandées que de colonnes déclarées, cellule ignorée"
                );
            }
            return None;
        };
        let align = self
            .columns
            .get(self.next)
            .map(|c| c.align)
            .unwrap_or(TableAlign::Start);
        let cell = Rect::from_min_size(
            egui::pos2(self.cursor_x, self.rect.top()),
            vec2(span, self.rect.height()),
        );
        let column = self.next;
        self.cursor_x += span;
        self.next += 1;
        Some((
            column,
            cell.shrink2(vec2(tokens::TABLE_CELL_PAD_X, 0.0)),
            align,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colonnes() -> Vec<TableColumn> {
        vec![
            TableColumn::fixed("Date", 100.0),
            TableColumn::flex("Nom", 2.0),
            TableColumn::fixed("Prix", 80.0).align(TableAlign::End),
            TableColumn::flex("Vendeur", 1.0),
        ]
    }

    #[test]
    fn les_elastiques_se_partagent_ce_que_les_fixes_laissent() {
        let spans = column_spans(&colonnes(), 500.0);
        // 500 - 100 - 80 = 320, partagés 2:1.
        assert_eq!(spans, vec![100.0, 320.0 * 2.0 / 3.0, 80.0, 320.0 / 3.0]);
        let total: f32 = spans.iter().sum();
        assert!((total - 500.0).abs() < 1e-3, "{total}");
    }

    #[test]
    fn sans_colonne_elastique_le_reste_demeure_a_droite() {
        let colonnes = vec![
            TableColumn::fixed("Date", 100.0),
            TableColumn::fixed("Prix", 80.0),
        ];
        // Une largeur imposée l'est vraiment : le tableau ne remplit pas sa ligne, plutôt que
        // d'élargir en douce des colonnes que l'appelant a fixées.
        assert_eq!(column_spans(&colonnes, 500.0), vec![100.0, 80.0]);
    }

    #[test]
    fn des_fixes_trop_larges_sont_reduites_du_meme_facteur() {
        let spans = column_spans(&colonnes(), 90.0);
        // 180 de fixe pour 90 disponibles : tout est divisé par deux, les élastiques disparaissent.
        assert_eq!(spans, vec![50.0, 0.0, 40.0, 0.0]);
        let total: f32 = spans.iter().sum();
        assert!((total - 90.0).abs() < 1e-3, "{total}");
    }

    #[test]
    fn une_largeur_nulle_ne_produit_pas_de_largeur_negative() {
        for span in column_spans(&colonnes(), 0.0) {
            assert_eq!(span, 0.0);
        }
        for span in column_spans(&colonnes(), -40.0) {
            assert_eq!(span, 0.0);
        }
    }

    #[test]
    fn le_zebrage_commence_a_la_premiere_ligne() {
        assert!(is_striped(0));
        assert!(!is_striped(1));
        assert!(is_striped(2));
    }

    #[test]
    fn zero_ligne_est_un_tableau_vide() {
        assert_eq!(TableBody::Rows(0).resolve(), TableBody::Empty);
        assert_eq!(TableBody::Rows(3).resolve(), TableBody::Rows(3));
        assert_eq!(TableBody::Loading.resolve(), TableBody::Loading);
    }

    #[test]
    fn la_hauteur_se_calcule_avant_le_rendu() {
        let entete = Table::header_height();
        assert_eq!(entete, 14.0 + 14.0 + 7.0);

        let quatre = table().body(TableBody::Rows(4));
        assert_eq!(quatre.height(), entete + 4.0 * 60.0);

        // Bornée : douze lignes dans la place de trois.
        let bornee = table().body(TableBody::Rows(12)).max_height(180.0);
        assert_eq!(bornee.height(), entete + 180.0);

        // La borne ne GONFLE pas un corps plus court qu'elle.
        let courte = table().body(TableBody::Rows(1)).max_height(600.0);
        assert_eq!(courte.height(), entete + 60.0);

        // Zéro ligne prend la hauteur du corps vide, pas zéro.
        let vide = table().body(TableBody::Rows(0));
        assert_eq!(vide.height(), entete + tokens::TABLE_EMPTY_HEIGHT);
    }
}
