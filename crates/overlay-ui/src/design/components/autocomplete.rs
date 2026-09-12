//! **Champ d'autocomplétion** du design system — un champ de saisie, une bande de filtres, et un
//! panneau de suggestions.
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! let issue = design::autocomplete(&mut state.saisie)
//!     .placeholder("Ajouter un objet à surveiller…")
//!     .width(560.0)
//!     .filters(&filtres)   // « Tout » en tête, puis les catégories PRÉSENTES
//!     .entries(&entrees)
//!     .log_name("alertes.ajout")
//!     .show(ui);
//! if let Some(index) = issue.selected {
//!     ajouter(&entrees[index]);
//! }
//! ```
//!
//! ## Une extension de `design::select`, pas un composant neuf
//!
//! Le panneau déplié reprend les jetons de **décor** de la liste de `select` : fond, bord, filet
//! de tête, surbrillance. C'est la même liste du jeu — il n'y avait pas de second relevé à faire.
//! Sa **rangée**, en revanche, est celle du web (35 px, marges de 10) : la liste du jeu n'a ni
//! gemme ni image à loger, et sa cadence de 28 px les collait — voir
//! [`tokens::AUTOCOMPLETE_ROW_HEIGHT`]. Ce que ce composant ajoute, et que `select` n'a pas : la
//! bande de filtres, une image par entrée, des entrées désactivées, et un seuil de déclenchement.
//!
//! ## Ce qui est porté du web, et assumé comme tel
//!
//! Le jeu n'a **pas** d'autocomplétion : aucune capture ne peut servir de référence. La géométrie
//! et le comportement viennent donc de `shared/wakfu-autocomplete` (dépôt `Oumbra/wakfu-companion`,
//! relevé le 2026-09-11), porté tel quel. C'est écrit dans `design/tokens.rs` à côté de chaque
//! valeur plutôt que laissé à deviner.
//!
//! Les cinq règles de comportement, qui ne se voient sur aucune capture :
//!
//! 1. **Rien avant [`tokens::AUTOCOMPLETE_MIN_QUERY_LEN`] caractères.** En dessous, le panneau ne
//!    s'ouvre pas — l'appelant n'a même pas à filtrer sa liste, le composant ne l'affiche pas.
//! 2. **Une entrée désactivée n'est pas sélectionnable** : grisée, sans surbrillance au survol,
//!    sautée par le clavier, et refusée par [`Autocomplete::show`] même si un clic l'atteignait.
//!    Trois barrières, parce qu'une seule finit toujours par être contournée.
//! 3. **Un filtre actif restreint la liste** à sa seule catégorie.
//! 4. **La bande se calcule sur la liste NON filtrée** — c'est-à-dire sur ce que l'appelant passe à
//!    [`Autocomplete::filters`], qu'il construit sans tenir compte du filtre actif. Elle doit rester
//!    entière quand le filtre ne laisse rien passer, sinon le bouton qui permettrait de le relâcher
//!    disparaîtrait avec les résultats, et l'utilisateur resterait coincé devant une liste vide.
//!    Dans ce cas le panneau affiche [`Autocomplete::empty_filter_label`] à la place des rangées.
//! 5. **Après une sélection** : le champ se vide, le panneau se ferme, l'entrée active repart à la
//!    première, et **le filtre revient à « Tout »**. Un filtre resté actif ferait disparaître des
//!    résultats d'une recherche sans rapport, sans que rien ne l'explique.
//!
//! ## Deux écarts au contrat, assumés
//!
//! **`show` plutôt que `impl Widget`** (§1). Une `egui::Response` ne peut pas dire *quelle* entrée a
//! été choisie, et la ressortir par un `&mut` en paramètre est précisément la maladresse que §6
//! reproche ailleurs. Même raison que pour les conteneurs (§1 bis), sur un composant qui n'en est
//! pas un : [`AutocompleteOutcome`] porte la `Response` **et** l'indice choisi.
//!
//! **Des textures en paramètre** (§1). La gemme de rareté et l'image d'un objet sont du **contenu**,
//! pas du décor : elles viennent du CDN `wakassets` par `RemoteIconStore`, le design system ne les
//! possède pas et ne peut pas les résoudre. Elles arrivent donc par [`AutocompleteEntry`], au même
//! titre que le libellé. Le décor du composant, lui (socle du champ, loupe), reste résolu en
//! interne par `DesignSystem::get`.
//!
//! ## Ce que le composant ne fait PAS
//!
//! Il ne cherche rien. L'appelant lui passe des entrées déjà trouvées, déjà triées, déjà marquées
//! « déjà suivi » — le composant ne connaît ni catalogue, ni `overlay_engine`, ni ce qu'est un
//! objet Wakfu. Une entrée n'est pour lui qu'un libellé, deux images facultatives, une clé de
//! catégorie et un drapeau. C'est ce qui lui permettra de servir aussi bien la page Alertes
//! (domaine « objets ») que le formulaire d'ajout au Suivi (objets **et** monstres) : le domaine
//! n'est pas un paramètre du composant, c'est simplement ce que l'appelant décide de lui donner.

use egui::emath::GuiRounding as _;
use egui::{Align2, Response, Sense, TextureId, Ui, Vec2};

use crate::design::components::icon_button::glyph_fit;
use crate::design::{text, tokens, DesignSystem, DsIcon, InputSize};

/// Une suggestion affichée par le panneau.
///
/// `gem` et `image` sont des textures **de contenu** (voir la doc de module) : leur absence n'est
/// pas une erreur, la rangée se peint sans elles.
#[derive(Clone, Debug)]
pub struct AutocompleteEntry {
    /// Ce qui s'affiche. Le composant ne connaît pas le nom « réel » de l'objet, seulement celui-ci.
    pub label: String,
    /// À quelle catégorie l'entrée appartient — comparée telle quelle à celle d'un filtre. Le
    /// composant n'interprète pas cette valeur, il l'égale.
    pub category: u16,
    /// La gemme de rareté, peinte à son rapport natif dans une boîte carrée.
    pub gem: Option<TextureId>,
    /// Taille native de la gemme — nécessaire pour la poser sans l'écraser (une gemme du jeu fait
    /// 13 × 20, pas un carré).
    pub gem_size: Vec2,
    /// L'image de l'objet, nue : dans ce panneau la rareté est portée par la gemme, un cadre de
    /// rareté ferait doublon.
    pub image: Option<TextureId>,
    /// Déjà dans la liste cible : grisée, non sélectionnable, sans surbrillance.
    pub disabled: bool,
    /// Mention alignée à droite (« déjà suivi »), affichée seulement si l'entrée est désactivée.
    pub mention: Option<String>,
}

impl AutocompleteEntry {
    /// Une entrée minimale : un libellé et sa catégorie. Le reste se pose par les champs publics.
    pub fn new(label: impl Into<String>, category: u16) -> Self {
        Self {
            label: label.into(),
            category,
            gem: None,
            gem_size: Vec2::splat(1.0),
            image: None,
            disabled: false,
            mention: None,
        }
    }
}

/// Un bouton de la bande de filtres.
#[derive(Clone, Debug)]
pub struct AutocompleteFilter {
    /// `None` = le bouton « Tout », qui relâche le filtre. **Ce n'est pas une catégorie** : c'est
    /// la remise à zéro, et il est actif tant qu'aucun filtre ne l'est.
    pub category: Option<u16>,
    /// L'icône du filtre — contenu, comme les images d'entrée (voir la doc de module).
    pub icon: Option<TextureId>,
    /// Infobulle du bouton.
    pub tooltip: String,
}

impl AutocompleteFilter {
    /// Le bouton « Tout ».
    pub fn all(tooltip: impl Into<String>, icon: Option<TextureId>) -> Self {
        Self {
            category: None,
            icon,
            tooltip: tooltip.into(),
        }
    }

    /// Un bouton de catégorie.
    pub fn category(category: u16, tooltip: impl Into<String>, icon: Option<TextureId>) -> Self {
        Self {
            category: Some(category),
            icon,
            tooltip: tooltip.into(),
        }
    }
}

/// Ce que rend [`Autocomplete::show`] — voir « deux écarts au contrat » dans la doc de module.
pub struct AutocompleteOutcome {
    /// La réponse du CHAMP, pas du panneau : c'est elle qui porte le focus et le survol du contrôle.
    pub response: Response,
    /// Indice, dans la liste passée à [`Autocomplete::entries`], de l'entrée choisie à cette frame.
    /// Jamais une entrée désactivée.
    pub selected: Option<usize>,
}

/// Construit un champ d'autocomplétion sur `query`. Point d'entrée unique — voir la doc de module.
pub fn autocomplete(query: &mut String) -> Autocomplete<'_> {
    Autocomplete::new(query)
}

pub struct Autocomplete<'a> {
    query: &'a mut String,
    entries: &'a [AutocompleteEntry],
    filters: &'a [AutocompleteFilter],
    placeholder: Option<String>,
    empty_filter_label: String,
    width: Option<f32>,
    min_query_len: usize,
    max_visible_rows: usize,
    enabled: bool,
    log_name: Option<String>,
    forced_open: Option<bool>,
    forced_active: Option<usize>,
    forced_filter: Option<Option<u16>>,
}

const AUCUNE_ENTREE: &[AutocompleteEntry] = &[];
const AUCUN_FILTRE: &[AutocompleteFilter] = &[];

impl<'a> Autocomplete<'a> {
    pub fn new(query: &'a mut String) -> Self {
        Self {
            query,
            entries: AUCUNE_ENTREE,
            filters: AUCUN_FILTRE,
            placeholder: None,
            empty_filter_label: "Aucun résultat dans cette catégorie".to_owned(),
            width: None,
            min_query_len: tokens::AUTOCOMPLETE_MIN_QUERY_LEN,
            max_visible_rows: tokens::AUTOCOMPLETE_MAX_VISIBLE_ROWS,
            enabled: true,
            log_name: None,
            forced_open: None,
            forced_active: None,
            forced_filter: None,
        }
    }

    /// Les suggestions, **déjà cherchées et déjà triées** par l'appelant — le composant ne cherche
    /// rien (voir la doc de module).
    pub fn entries(mut self, entries: &'a [AutocompleteEntry]) -> Self {
        self.entries = entries;
        self
    }

    /// La bande de filtres, dans l'ordre d'affichage, **calculée sur la liste NON filtrée**. Vide :
    /// pas de bande du tout.
    pub fn filters(mut self, filters: &'a [AutocompleteFilter]) -> Self {
        self.filters = filters;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Le message affiché quand le filtre actif ne laisse passer aucune entrée.
    pub fn empty_filter_label(mut self, label: impl Into<String>) -> Self {
        self.empty_filter_label = label.into();
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Longueur minimale de la requête. Le défaut est celui du web ; le baisser ouvre le panneau
    /// plus tôt, au prix d'une liste plus longue à parcourir.
    pub fn min_query_len(mut self, len: usize) -> Self {
        self.min_query_len = len;
        self
    }

    /// Nombre de rangées visibles avant que la liste ne défile.
    pub fn max_visible_rows(mut self, rows: usize) -> Self {
        self.max_visible_rows = rows.max(1);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Force l'ouverture du panneau — **réservé à la galerie et aux captures** : en rendu offscreen
    /// aucun champ n'a le focus, donc rien ne s'ouvrirait jamais.
    pub fn preview_open(mut self, open: bool) -> Self {
        self.forced_open = Some(open);
        self
    }

    /// Force l'entrée active — même usage que [`Autocomplete::preview_open`].
    pub fn preview_active(mut self, index: usize) -> Self {
        self.forced_active = Some(index);
        self
    }

    /// Force le filtre actif — même usage que [`Autocomplete::preview_open`].
    pub fn preview_filter(mut self, filter: Option<u16>) -> Self {
        self.forced_filter = Some(filter);
        self
    }

    /// Les indices, dans `self.entries`, que le filtre actif laisse passer.
    ///
    /// Des INDICES et non des références : c'est l'indice de la liste d'origine que
    /// [`AutocompleteOutcome::selected`] rend, pour que l'appelant retrouve son entrée sans avoir à
    /// refaire le filtrage de son côté.
    fn visible(&self, filter: Option<u16>) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| filter.is_none_or(|category| entry.category == category))
            .map(|(index, _)| index)
            .collect()
    }

    pub fn show(self, ui: &mut Ui) -> AutocompleteOutcome {
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let name = self
            .log_name
            .clone()
            .unwrap_or_else(|| "autocomplete".to_owned());

        let field = egui::widgets::Widget::ui(
            {
                // La barre de recherche du jeu, pas le champ de formulaire : 28 px, loupe en
                // miroir, croix d'effacement — voir `InputSize::Search` et `Input::clearable`.
                let mut input = crate::design::input(self.query)
                    .leading_icon(DsIcon::Search)
                    .size(InputSize::Search)
                    .clearable(true)
                    .width(width)
                    .enabled(self.enabled);
                if let Some(placeholder) = self.placeholder.clone() {
                    input = input.placeholder(placeholder);
                }
                input
            },
            ui,
        );

        // État d'INTERACTION, en mémoire egui indexée sur l'id du widget — comme `design::select`,
        // et pour la même raison : ce n'est pas de l'état applicatif (le contrat en interdit au
        // composant), c'est du même ordre que « ce widget a le focus ». L'appelant n'a rien à
        // stocker et rien à remettre à zéro.
        let active_id = field.id.with("ds-autocomplete-active");
        let filter_id = field.id.with("ds-autocomplete-filter");
        let mut active = self
            .forced_active
            .unwrap_or_else(|| ui.data(|d| d.get_temp::<usize>(active_id).unwrap_or(0)));
        let mut filter = self
            .forced_filter
            .unwrap_or_else(|| ui.data(|d| d.get_temp::<Option<u16>>(filter_id).unwrap_or(None)));

        // Une frappe remet l'entrée active à la première : sinon Entrée validerait un objet que la
        // nouvelle recherche n'affiche peut-être plus.
        if field.changed() {
            active = 0;
        }

        // **Le seuil.** Il se compte en CARACTÈRES, pas en octets : « clé » fait trois caractères et
        // quatre octets, et doit déclencher la recherche comme n'importe quel mot de trois lettres.
        let assez_long = self.query.chars().count() >= self.min_query_len;

        // **Le panneau survit à la perte de focus tant que le pointeur est dessus.**
        //
        // Un clic sur une suggestion se joue en deux frames : l'APPUI, qui retire le focus au champ
        // (le pointeur est sur le panneau, pas sur lui), et le RELÂCHEMENT, seul moment où egui
        // rend `clicked()` vrai. Une condition d'ouverture réduite à `field.has_focus()` ferme donc
        // le panneau à l'appui : la rangée n'est plus peinte à la frame suivante, son `clicked()`
        // n'arrive jamais, et **aucune suggestion n'est cliquable** — le défaut remonté le
        // 2026-09-12 (« le champ d'auto-complétion ne semblait pas fonctionner »).
        //
        // Le rectangle du panneau de la frame PRÉCÉDENTE sert de second ancrage : il est mémorisé
        // à chaque peinture et effacé dès que le panneau se ferme, pour qu'un rectangle périmé ne
        // puisse pas le rouvrir au simple passage de la souris.
        let panel_rect_id = field.id.with("ds-autocomplete-panel-rect");
        let dernier_panneau: Option<egui::Rect> = ui.data(|d| d.get_temp(panel_rect_id));
        let sur_le_panneau = match (dernier_panneau, ui.input(|i| i.pointer.interact_pos())) {
            (Some(rect), Some(pos)) => rect.contains(pos),
            _ => false,
        };
        // **Entrée retire le focus au champ AVANT que le panneau ne la lise.** Un `TextEdit` à
        // une ligne rend le focus sur sa touche de retour (`return_key`, voir egui), et il est
        // peint avant ce bloc : à la frame d'Entrée, `has_focus()` est déjà faux. Sans ce
        // rattrapage, la touche fermait le panneau sans rien choisir — le test au clavier
        // (`options_alertes_les_fleches_font_defiler_la_liste`) l'a montré le 2026-09-12 au soir,
        // aucun test n'ayant validé une suggestion autrement qu'au clic jusque-là. Le champ compte
        // donc comme focalisé pendant la frame où Entrée vient de le lui reprendre.
        let entree = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let au_clavier = field.has_focus() || (entree && field.lost_focus());
        let open = self.forced_open.unwrap_or(
            self.enabled
                && assez_long
                && !self.entries.is_empty()
                && (au_clavier || sur_le_panneau),
        );

        let mut selected = None;
        let mut panel_rect = None;
        if open {
            let mut visible = self.visible(filter);
            if active >= visible.len() {
                active = 0;
            }

            // Le clavier AVANT le panneau : les touches sont consommées pour que le champ de saisie
            // ne les reçoive pas (↑/↓ y déplaceraient le curseur, Échap y annulerait l'édition).
            //
            // **Une flèche fait aussi défiler.** Sans ça, l'entrée active sort de la fenêtre des
            // cinq rangées visibles dès la sixième, et l'utilisateur valide à l'aveugle une entrée
            // qu'il ne voit pas — défaut remonté le 2026-09-12 au soir. C'est le `scrollIntoView
            // ({ block: 'nearest' })` de `moveActive()` côté web : le panneau ne défile que du
            // strict nécessaire pour ramener la rangée en vue, jamais pour la centrer.
            let mut suivre_au_clavier = false;
            if self.forced_open.is_none() && au_clavier {
                if let Some(delta) = fleche(ui) {
                    active = suivante(&visible, self.entries, active, delta);
                    suivre_au_clavier = true;
                }
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                    match visible.get(active) {
                        Some(&index) if !self.entries[index].disabled => selected = Some(index),
                        // Rien à choisir (entrée désactivée, filtre vide) : le champ reprend le
                        // focus que le `TextEdit` vient de rendre, et le panneau reste ouvert —
                        // comme sur le web, où Entrée sur une entrée désactivée ne fait rien.
                        _ => field.request_focus(),
                    }
                }
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                    // Ferme le panneau SANS vider le champ : la saisie reste, le panneau se rouvre
                    // à la frappe suivante.
                    field.surrender_focus();
                }
            }

            let outcome = self.paint_panel(
                ui,
                &field,
                width,
                &visible,
                active,
                suivre_au_clavier,
                filter,
                &name,
            );
            panel_rect = Some(outcome.panel_rect);
            if let Some(index) = outcome.clicked_filter {
                // Recliquer le filtre actif le relâche.
                filter = if filter == index { None } else { index };
                active = 0;
                visible = self.visible(filter);
                let _ = &visible;
            }
            if let Some(index) = outcome.hovered_row {
                active = index;
            }
            if let Some(index) = outcome.clicked_row {
                selected = Some(index);
            }
        }

        if let Some(index) = selected {
            tracing::debug!(
                component = "autocomplete",
                name,
                entree = %self.entries[index].label,
                "entrée choisie"
            );
            // Les cinq effets d'une sélection — voir la doc de module. Le filtre revient à « Tout » :
            // le garder ferait disparaître des résultats d'une recherche sans rapport.
            self.query.clear();
            active = 0;
            filter = None;
            field.surrender_focus();
        }

        ui.data_mut(|d| {
            d.insert_temp(active_id, active);
            d.insert_temp(filter_id, filter);
            match panel_rect {
                Some(rect) => {
                    d.insert_temp(panel_rect_id, rect);
                }
                None => d.remove::<egui::Rect>(panel_rect_id),
            }
        });

        AutocompleteOutcome {
            response: field,
            selected,
        }
    }

    /// Peint le panneau déplié et rend ce qui y a été touché.
    #[allow(clippy::too_many_arguments)]
    fn paint_panel(
        &self,
        ui: &mut Ui,
        field: &Response,
        width: f32,
        visible: &[usize],
        active: usize,
        suivre_au_clavier: bool,
        filter: Option<u16>,
        name: &str,
    ) -> PanelOutcome {
        let pad = tokens::AUTOCOMPLETE_PANEL_PAD;
        let bar = if self.filters.is_empty() {
            0.0
        } else {
            tokens::AUTOCOMPLETE_FILTER_BAR_HEIGHT
        };
        let rows = visible.len().min(self.max_visible_rows).max(1) as f32;
        let corps = if visible.is_empty() {
            tokens::AUTOCOMPLETE_EMPTY_HEIGHT
        } else {
            rows * tokens::AUTOCOMPLETE_ROW_HEIGHT
        };
        // **Au pixel.** Le champ peut tomber sur une demi-ligne (sa hauteur et celle de ce qui le
        // précède ne sont pas toutes paires) : le panneau hériterait du demi-pixel, et tout ce qui
        // se peint dedans avec lui — mesuré sur la capture, le rail de la barre y perdait son
        // pixel de débord à chaque bout dans une rangée à moitié couverte.
        let panel_rect = egui::Rect::from_min_size(
            egui::pos2(
                field.rect.left(),
                field.rect.bottom() + tokens::AUTOCOMPLETE_PANEL_GAP,
            ),
            Vec2::new(width, 2.0 * pad + bar + corps),
        )
        .round_to_pixels(ui.pixels_per_point());

        let mut outcome = PanelOutcome {
            panel_rect,
            clicked_filter: None,
            hovered_row: None,
            clicked_row: None,
        };
        // `Area` au premier plan, exactement comme `design::select` : le panneau sort du flux, donc
        // ni le widget suivant ne le recouvre, ni son ouverture ne décale la mise en page.
        //
        // **L'identifiant dérive du CHAMP, jamais du `Ui` parent** : deux autocomplétions posées
        // dans le même parent partageraient sinon le même id d'`Area` et de rangées — egui le
        // signale en rouge par-dessus le rendu (« First use of widget ID … »), et les deux
        // panneaux se disputeraient le même état.
        egui::Area::new(field.id.with("ds-autocomplete-popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(panel_rect.min)
            .constrain(false)
            .show(ui.ctx(), |ui| {
                ui.set_min_size(panel_rect.size());
                let painter = ui.painter();
                painter.rect_filled(panel_rect, tokens::SELECT_RADIUS, tokens::SELECT_LIST_FILL);
                painter.rect_stroke(
                    panel_rect,
                    tokens::SELECT_RADIUS,
                    egui::Stroke::new(2.0, tokens::SELECT_LIST_BORDER),
                    egui::StrokeKind::Inside,
                );

                let mut y = panel_rect.top() + pad;
                if bar > 0.0 {
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(panel_rect.left() + pad, y),
                        Vec2::new(width - 2.0 * pad, bar),
                    );
                    outcome.clicked_filter =
                        self.paint_filters(ui, field.id, bar_rect, filter, name);
                    y += bar;
                }

                if visible.is_empty() {
                    ui.painter().text(
                        egui::pos2(
                            panel_rect.center().x,
                            y + tokens::AUTOCOMPLETE_EMPTY_HEIGHT / 2.0,
                        ),
                        Align2::CENTER_CENTER,
                        &self.empty_filter_label,
                        text::label_font(ui.ctx(), tokens::AUTOCOMPLETE_MENTION_FONT_SIZE),
                        tokens::TEXT_DISABLED,
                    );
                    return;
                }

                let liste = egui::Rect::from_min_size(
                    egui::pos2(panel_rect.left() + pad, y),
                    Vec2::new(width - 2.0 * pad, corps),
                );
                let mut contenu = ui.new_child(egui::UiBuilder::new().max_rect(liste));
                // **Rangées jointives** : la hauteur du panneau vaut `rangées × 35`, sans
                // interligne. L'espacement vertical hérité du thème (3 px) s'ajouterait à chaque
                // `allocate_exact_size` et pousserait la dernière rangée SOUS le fond du panneau —
                // défaut observé sur la galerie, le libellé de la quatrième rangée y était coupé
                // en deux.
                contenu.spacing_mut().item_spacing = Vec2::ZERO;
                if visible.len() > self.max_visible_rows {
                    // Au-delà de `max_visible_rows`, la liste défile au lieu de s'étirer — sinon le
                    // panneau finirait par sortir de la fenêtre du jeu.
                    //
                    // **Seulement dans ce cas** : une `ScrollArea` demande un repeint tant que son
                    // décalage s'anime, et un harnais offscreen (`Harness::run`) tourne alors
                    // jusqu'à sa limite d'étapes sans jamais converger. Une liste qui tient
                    // entièrement n'a rien à faire défiler, donc rien à animer.
                    //
                    // **La barre : celle du web.** Le style d'egui par défaut la faisait mince,
                    // puis large ET plus claire sous le pointeur. Trois retours du 2026-09-12
                    // l'ont fixée : d'abord « plus large, sans changer de couleur », puis le soir
                    // « retire l'élargissement ; pour dire qu'on peut agir dessus, la couleur des
                    // éléments survolés ; et un rail plus sombre que le fond », puis dans la nuit
                    // « la poignée de la couleur de la liste, et le rail qui la déborde d'un
                    // pixel de chaque côté, en butée aussi ». D'où : 8 px dans tous les états,
                    // poignée de la teinte de la liste au repos et des rangées survolées sous le
                    // pointeur, centrée sur un rail de 10 px et 1 px plus long à chaque bout.
                    // Posé dans le scope du panneau, comme `design::scroll_area` le fait pour les
                    // siens — sans son gabarit, qui est celui de la fenêtre Options du jeu, pas
                    // de ce panneau porté du web.
                    //
                    // **Le rail est peint ICI, pas par egui.** egui donne au rail exactement
                    // l'étendue de la poignée (même `cross`, même `scroll_bar_rect`) : il ne peut
                    // ni la déborder ni s'allonger d'un pixel en butée. Il est donc peint sous la
                    // `ScrollArea`, sur la colonne qu'elle réserve, et son propre fond est rendu
                    // transparent (opacités à zéro). Avec le peintre du PANNEAU, pas celui de la
                    // liste : celle-ci écrête à son rectangle, et le pixel qui dépasse à chaque
                    // bout y disparaissait (mesuré sur la capture : 175 rangées de rail pour 177
                    // attendues).
                    let inset = tokens::AUTOCOMPLETE_SCROLLBAR_TRACK_INSET;
                    let colonne = tokens::AUTOCOMPLETE_SCROLLBAR_WIDTH + 2.0 * inset;
                    let rail = egui::Rect::from_min_max(
                        egui::pos2(liste.right() - colonne, liste.top() - inset),
                        egui::pos2(liste.right(), liste.bottom() + inset),
                    );
                    ui.painter().rect_filled(
                        rail,
                        egui::CornerRadius::same(
                            tokens::AUTOCOMPLETE_SCROLLBAR_RADIUS + inset as u8,
                        ),
                        tokens::AUTOCOMPLETE_SCROLLBAR_TRACK,
                    );
                    let scroll = &mut contenu.style_mut().spacing.scroll;
                    scroll.floating = true;
                    scroll.floating_width = tokens::AUTOCOMPLETE_SCROLLBAR_WIDTH;
                    scroll.bar_width = tokens::AUTOCOMPLETE_SCROLLBAR_WIDTH;
                    // La barre a sa colonne — celle du rail : elle ne recouvre jamais la mention
                    // de droite. La marge extérieure la décolle du bord d'un pixel, ce qui la
                    // centre dans le rail.
                    scroll.floating_allocated_width = colonne;
                    scroll.bar_outer_margin = inset;
                    scroll.foreground_color = false;
                    scroll.handle_min_length = tokens::AUTOCOMPLETE_SCROLLBAR_MIN_HANDLE;
                    // Poignée pleine dans tous les états : c'est sa teinte qui dit le survol, pas
                    // une opacité ni une largeur. Le rail d'egui, lui, est invisible (voir plus
                    // haut).
                    scroll.dormant_background_opacity = 0.0;
                    scroll.active_background_opacity = 0.0;
                    scroll.interact_background_opacity = 0.0;
                    scroll.dormant_handle_opacity = 1.0;
                    scroll.active_handle_opacity = 1.0;
                    scroll.interact_handle_opacity = 1.0;
                    let visuals = contenu.visuals_mut();
                    let radius = egui::CornerRadius::same(tokens::AUTOCOMPLETE_SCROLLBAR_RADIUS);
                    for widget in [
                        &mut visuals.widgets.noninteractive,
                        &mut visuals.widgets.inactive,
                    ] {
                        widget.bg_fill = tokens::AUTOCOMPLETE_SCROLLBAR_THUMB;
                        widget.corner_radius = radius;
                    }
                    // `hovered` ne s'applique que le pointeur SUR la poignée (egui vérifie sa
                    // position, pas seulement le survol de la colonne) ; `active` pendant le
                    // glissement.
                    for widget in [&mut visuals.widgets.hovered, &mut visuals.widgets.active] {
                        widget.bg_fill = tokens::AUTOCOMPLETE_SCROLLBAR_THUMB_HOVERED;
                        widget.corner_radius = radius;
                    }
                    egui::ScrollArea::vertical()
                        .max_height(corps)
                        .auto_shrink([false; 2])
                        // Sans animation : une flèche ramène la rangée en vue à la frame même,
                        // comme `scrollIntoView` côté web — et un harnais offscreen n'a pas à
                        // attendre qu'un défilement converge.
                        .animated(false)
                        .show(&mut contenu, |ui| {
                            // La largeur DISPONIBLE, pas celle de la liste : la colonne de la
                            // barre en est retirée, et une rangée qui passerait dessous y
                            // perdrait sa mention.
                            let largeur = ui.available_width();
                            for (rang, &index) in visible.iter().enumerate() {
                                let row =
                                    self.paint_row(ui, index, rang, active, largeur, &mut outcome);
                                if suivre_au_clavier && rang == active {
                                    ui.scroll_to_rect(row, None);
                                }
                            }
                        });
                    // Pas de curseur particulier sur la barre : la main qui agrippe, demandée puis
                    // retirée dans la même nuit (2026-09-12), a vécu une version. C'est la teinte
                    // de la poignée sous le pointeur qui dit qu'on peut agir dessus.
                } else {
                    for (rang, &index) in visible.iter().enumerate() {
                        self.paint_row(
                            &mut contenu,
                            index,
                            rang,
                            active,
                            liste.width(),
                            &mut outcome,
                        );
                    }
                }
            });
        outcome
    }

    fn paint_filters(
        &self,
        ui: &Ui,
        base: egui::Id,
        rect: egui::Rect,
        actif: Option<u16>,
        name: &str,
    ) -> Option<Option<u16>> {
        let design = DesignSystem::get(ui.ctx());
        let mut clicked = None;
        for (i, filtre) in self.filters.iter().enumerate() {
            let cell = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left()
                        + tokens::AUTOCOMPLETE_FILTER_BAR_PAD
                        + i as f32
                            * (tokens::AUTOCOMPLETE_FILTER_BUTTON
                                + tokens::AUTOCOMPLETE_FILTER_GAP),
                    rect.center().y - tokens::AUTOCOMPLETE_FILTER_BUTTON / 2.0,
                ),
                Vec2::splat(tokens::AUTOCOMPLETE_FILTER_BUTTON),
            );
            // La main, comme sur tout ce qui se clique — `cursor: pointer` du web.
            let response = ui
                .interact(
                    cell,
                    base.with(("ds-autocomplete-filter", i)),
                    Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let est_actif = filtre.category == actif;
            if est_actif || response.hovered() {
                ui.painter().rect_filled(
                    cell,
                    tokens::AUTOCOMPLETE_FILTER_RADIUS,
                    tokens::SELECT_ROW_HIGHLIGHT,
                );
            }
            if est_actif {
                ui.painter().rect_stroke(
                    cell,
                    tokens::AUTOCOMPLETE_FILTER_RADIUS,
                    egui::Stroke::new(1.0, tokens::STEPPER_ICON_TINT),
                    egui::StrokeKind::Inside,
                );
            }
            if let Some(icon) = filtre.icon {
                let teinte = if est_actif || response.hovered() {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_white_alpha(tokens::AUTOCOMPLETE_FILTER_IDLE_ALPHA)
                };
                egui::Image::from_texture(egui::load::SizedTexture::new(
                    icon,
                    Vec2::splat(tokens::AUTOCOMPLETE_FILTER_BUTTON),
                ))
                .tint(teinte)
                .paint_at(ui, cell.shrink(tokens::AUTOCOMPLETE_FILTER_ICON_PAD));
            }
            let response = response.on_hover_text(&filtre.tooltip);
            if response.clicked() {
                tracing::debug!(
                    component = "autocomplete",
                    name,
                    filtre = ?filtre.category,
                    "filtre cliqué"
                );
                clicked = Some(filtre.category);
            }
        }
        let _ = design;
        // Le filet qui sépare la bande des suggestions — le même jeton que la liste dépliée du jeu.
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            egui::Stroke::new(1.0, tokens::SELECT_LIST_TOP_LINE),
        );
        clicked
    }

    /// Peint une rangée et rend son rectangle — l'appelant s'en sert pour la ramener en vue.
    fn paint_row(
        &self,
        ui: &mut Ui,
        index: usize,
        rang: usize,
        active: usize,
        width: f32,
        outcome: &mut PanelOutcome,
    ) -> egui::Rect {
        let entry = &self.entries[index];
        let (row, response) = ui.allocate_exact_size(
            Vec2::new(width, tokens::AUTOCOMPLETE_ROW_HEIGHT),
            if entry.disabled {
                Sense::hover()
            } else {
                Sense::click()
            },
        );
        // La main sur ce qui se choisit, la flèche sur ce qui ne se choisit pas — `cursor:
        // pointer` de `.wakfu-autocomplete-item-main`, `cursor: default` de sa variante
        // `.disabled`.
        let response = if entry.disabled {
            response
        } else {
            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        };

        // **Aucune surbrillance sur une entrée désactivée**, même survolée : une ligne grisée qui
        // s'allume promet un clic qui n'arrivera pas.
        if !entry.disabled && (rang == active || response.hovered()) {
            ui.painter()
                .rect_filled(row, 0, tokens::SELECT_ROW_HIGHLIGHT);
        }
        if response.hovered() && !entry.disabled {
            outcome.hovered_row = Some(rang);
        }
        if response.clicked() {
            outcome.clicked_row = Some(index);
        }

        // La géométrie du web, cote pour cote — voir le schéma au-dessus de
        // `AUTOCOMPLETE_ROW_HEIGHT` dans `tokens.rs` : marge 10, gemme dans sa boîte de 14, puis
        // une colonne d'image de 30 qui commence à 30 du bord, puis 10 d'écart, puis le nom.
        let marge = row.left() + tokens::AUTOCOMPLETE_ROW_PADDING_X;
        if let Some(gem) = entry.gem {
            let boite = egui::Rect::from_center_size(
                egui::pos2(marge + tokens::AUTOCOMPLETE_GEM_BOX / 2.0, row.center().y),
                Vec2::splat(tokens::AUTOCOMPLETE_GEM_BOX),
            );
            // À son rapport NATIF, comme `object-fit: contain` : une gemme du jeu fait 13 × 20 et
            // entre dans la boîte en 9 × 14. C'est à l'appelant de fournir `gem_size` — sans elle
            // la gemme est écrasée en carré, et c'est précisément ce que l'onglet Alertes montrait
            // jusqu'au 2026-09-12 au soir (« très fortement agrandies et aplaties »).
            let taille = glyph_fit(entry.gem_size, tokens::AUTOCOMPLETE_GEM_BOX);
            egui::Image::from_texture(egui::load::SizedTexture::new(gem, taille))
                .paint_at(ui, egui::Rect::from_center_size(boite.center(), taille));
        }
        let colonne = egui::Rect::from_min_size(
            egui::pos2(marge + tokens::AUTOCOMPLETE_IMAGE_COLUMN_OFFSET, row.top()),
            Vec2::new(tokens::AUTOCOMPLETE_IMAGE_COLUMN, row.height()),
        );
        if let Some(image) = entry.image {
            let taille = Vec2::splat(tokens::AUTOCOMPLETE_IMAGE_SIZE);
            egui::Image::from_texture(egui::load::SizedTexture::new(image, taille))
                .paint_at(ui, egui::Rect::from_center_size(colonne.center(), taille));
        }
        let x = colonne.right() + tokens::AUTOCOMPLETE_ROW_GAP;

        let couleur = if entry.disabled {
            tokens::TEXT_DISABLED
        } else {
            tokens::SELECT_TEXT
        };
        // Écrêté à la rangée : un nom long ne déborde pas sur le panneau voisin, où il passerait
        // pour un bug de mise en page.
        ui.painter()
            .with_clip_rect(row.intersect(ui.clip_rect()))
            .text(
                egui::pos2(x, row.center().y),
                Align2::LEFT_CENTER,
                &entry.label,
                text::label_font(ui.ctx(), tokens::AUTOCOMPLETE_ENTRY_FONT_SIZE),
                couleur,
            );

        if let (true, Some(mention)) = (entry.disabled, entry.mention.as_ref()) {
            ui.painter().text(
                egui::pos2(
                    row.right() - tokens::AUTOCOMPLETE_MENTION_MARGIN,
                    row.center().y,
                ),
                Align2::RIGHT_CENTER,
                mention,
                text::label_font(ui.ctx(), tokens::AUTOCOMPLETE_MENTION_FONT_SIZE),
                tokens::TEXT_DISABLED,
            );
        }
        row
    }
}

struct PanelOutcome {
    /// Le rectangle réellement peint — mémorisé par [`Autocomplete::show`] pour que le panneau
    /// survive à la frame d'appui d'un clic (voir la doc de la condition d'ouverture).
    panel_rect: egui::Rect,
    clicked_filter: Option<Option<u16>>,
    hovered_row: Option<usize>,
    clicked_row: Option<usize>,
}

/// La flèche pressée, consommée pour que le champ de saisie ne la reçoive pas.
fn fleche(ui: &Ui) -> Option<isize> {
    ui.input_mut(|i| {
        if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
            Some(1)
        } else if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
            Some(-1)
        } else {
            None
        }
    })
}

/// L'entrée active après un déplacement de `delta` — **en sautant les désactivées, et en bouclant**.
///
/// La boucle s'arrête au bout d'un tour complet : si TOUTES les entrées visibles sont désactivées,
/// elle rend l'indice de départ plutôt que de tourner indéfiniment. Ce cas n'est pas théorique —
/// chercher un objet déjà entièrement suivi le produit.
fn suivante(
    visible: &[usize],
    entries: &[AutocompleteEntry],
    depuis: usize,
    delta: isize,
) -> usize {
    if visible.is_empty() {
        return 0;
    }
    let len = visible.len() as isize;
    let mut index = depuis as isize;
    for _ in 0..visible.len() {
        index = (index + delta).rem_euclid(len);
        if !entries[visible[index as usize]].disabled {
            break;
        }
    }
    index as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entrees(desactivees: &[bool]) -> Vec<AutocompleteEntry> {
        desactivees
            .iter()
            .enumerate()
            .map(|(i, &off)| {
                let mut entry = AutocompleteEntry::new(format!("entrée {i}"), 0);
                entry.disabled = off;
                entry
            })
            .collect()
    }

    #[test]
    fn la_navigation_saute_les_entrees_desactivees() {
        let entries = entrees(&[false, true, false]);
        let visible = vec![0, 1, 2];
        // Depuis la première, ↓ saute la deuxième (désactivée) et atteint la troisième.
        assert_eq!(suivante(&visible, &entries, 0, 1), 2);
        // Et en remontant, symétriquement.
        assert_eq!(suivante(&visible, &entries, 2, -1), 0);
    }

    #[test]
    fn la_navigation_boucle() {
        let entries = entrees(&[false, false]);
        let visible = vec![0, 1];
        assert_eq!(suivante(&visible, &entries, 1, 1), 0, "depuis la dernière");
        assert_eq!(suivante(&visible, &entries, 0, -1), 1, "depuis la première");
    }

    /// Le cas qui ferait tourner une boucle naïve indéfiniment : chercher un objet dont TOUTES les
    /// suggestions sont déjà suivies.
    #[test]
    fn toutes_desactivees_ne_boucle_pas_indefiniment() {
        let entries = entrees(&[true, true, true]);
        let visible = vec![0, 1, 2];
        assert_eq!(suivante(&visible, &entries, 0, 1), 0);
    }

    #[test]
    fn liste_vide_reste_a_zero() {
        assert_eq!(suivante(&[], &entrees(&[]), 0, 1), 0);
    }

    #[test]
    fn le_filtre_restreint_aux_entrees_de_sa_categorie() {
        let mut query = String::new();
        let entries = vec![
            AutocompleteEntry::new("équipement", 1),
            AutocompleteEntry::new("ressource", 2),
            AutocompleteEntry::new("autre équipement", 1),
        ];
        let composant = autocomplete(&mut query).entries(&entries);
        assert_eq!(composant.visible(None), vec![0, 1, 2], "« Tout »");
        assert_eq!(composant.visible(Some(1)), vec![0, 2]);
        assert_eq!(composant.visible(Some(2)), vec![1]);
        assert!(composant.visible(Some(9)).is_empty(), "catégorie absente");
    }
}
