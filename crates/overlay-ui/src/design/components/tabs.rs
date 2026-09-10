//! **Barre d'onglets** du design system Wakfu — le composant le plus réutilisé du lot : chaque
//! fenêtre à onglets de l'overlay s'appuiera dessus, pas seulement la modale Options.
//!
//! ```ignore
//! use overlay_ui::design::{self, TabState};
//!
//! design::tabs(&mut self.onglet)
//!     .entry(OptionsTab::Alertes, "Alertes")
//!     .enabled(false)
//!     .entry(OptionsTab::Personnages, "Personnages")
//!     .enabled(false)
//!     .entry(OptionsTab::Parametres, "Paramètres")
//!     .log_name("options-onglets")
//!     .show(ui);
//! ```
//!
//! La valeur sélectionnée vit chez l'appelant, comme celle de `design::input` : le composant l'écrit
//! au clic, l'appelant la relit pour décider quoi peindre en dessous. `Response::changed()` dit à
//! quelle frame elle a bougé.
//!
//! ## Le piège de cette barre
//!
//! Énoncé tel quel dans [`releve-modale-options.json`](../../../../../docs/design-system/releve-modale-options.json)
//! (nœud `tab-interface`) : **« l'état survolé d'un onglet reprend exactement le fond de l'état
//! actif ; la seule différence relevée est la couleur du libellé (blanc pour l'actif, doré pour
//! survolé et inactif). Un portage qui ne distingue que par le fond rendrait les deux états
//! indiscernables. »**
//!
//! | État | Fond | Libellé |
//! | --- | --- | --- |
//! | Inactif | sombre `#363734` | doré `#f4d89e` |
//! | Survolé | **kaki, celui de l'actif** | doré `#f4d89e` |
//! | Actif | kaki `#625a47` | **blanc `#ffffff`** |
//! | Désactivé | sombre | gris — **inventé**, voir plus bas |
//!
//! Deux textures suffisent donc pour quatre états.
//!
//! ## La structure de la barre, mesurée au pixel
//!
//! Sur `interface-options-video.png` (ligne y=110) comme sur `tabs-with-first-tab-active.png`, la
//! séquence horizontale entre deux onglets est **toujours la même** :
//!
//! ```text
//! … corps │ bord 2px │ séparateur clair 2px │ bord 2px │ corps …
//!          ↑ appartient à l'onglet de gauche      ↑ à celui de droite
//! ```
//!
//! Ce qui explique les 6 px de gouttière que le relevé mesure entre deux boîtes d'onglet (21..98
//! puis 104..187) : ses boîtes cotent le **remplissage**, hors bords. Le composant pose donc les
//! onglets à 2 px l'un de l'autre — chacun portant ses deux bords — et peint le séparateur dans
//! cette gouttière.
//!
//! ## Le séparateur : ni pleine hauteur, ni d'une seule couleur
//!
//! Profil vertical de la colonne x=261 de `tabs-with-first-tab-active.png`, la première gouttière :
//!
//! | y (sur 44) | Ce qu'on y trouve |
//! | --- | --- |
//! | 0..2 | **rien** — le bord sombre de la barre, le trait ne commence pas là |
//! | 2..13 | `#837d70`, plateau clair de 11 px |
//! | 13..32 | rampe linéaire d'un plateau à l'autre, 19 px |
//! | 32..42 | `#595140`, plateau sombre de 10 px |
//! | 42..44 | **rien** — bord sombre |
//!
//! Deux conséquences, et les deux se voient à l'œil nu à côté d'une capture du jeu :
//!
//! - le trait fait **40 px, pas 44** : il s'arrête au corps de l'onglet, entre ses deux bords
//!   sombres ([`tokens::TAB_BORDER_Y`]). Peint de bord à bord, il dépasse en haut et en bas ;
//! - il **descend du clair au sombre**. Un aplat est plat là où le jeu a du relief.
//!
//! Le dégradé règle au passage une incohérence qui traînait dans les jetons — trois couleurs
//! relevées pour ce seul trait, qui sont trois hauteurs du même dégradé. Voir
//! [`tokens::TAB_SEPARATOR_TOP`].
//!
//! ## Le libellé est cerné, et pas de noir
//!
//! Le jeu repeint chaque libellé d'onglet sur **1 px dans les huit directions**, dans une version
//! très assombrie de **sa propre couleur** — 20,5 % (voir [`tokens::TAB_LABEL_OUTLINE_FACTOR`], qui
//! porte la mesure et écarte les deux autres explications possibles). C'est ce cerne qui détache le
//! mot de son fond ; sans lui, le libellé paraît posé à plat à côté de la capture du jeu.
//!
//! Ce n'est **pas** le cerne noir de `design::text::paint_outlined_text`, celui du titre de modale et
//! des dégâts de combat : mesuré ici sur deux couleurs de libellé de la même capture, un doré et un
//! blanc, il suit la couleur du texte et non le fond. Et ce n'est pas non plus la graisse synthétique
//! retirée en 2026-09-09 : huit copies de la *même* couleur empâtent le mot, huit copies nettement
//! plus sombres le détourent.
//!
//! Les boutons, eux, n'en ont pas — vérifié sur `large-button-cancel.png` et
//! `large-button-validate.png`, où l'anneau autour du libellé ne s'écarte du fond que de 4 %.
//!
//! ## Largeur : parts égales, sur toute la largeur disponible
//!
//! **Par défaut, la barre occupe toute la largeur qu'on lui donne et ses onglets s'y partagent la
//! place à égalité**, gouttières déduites. C'est un choix de l'overlay, pas un relevé, et il
//! s'écarte du jeu en connaissance de cause.
//!
//! Le jeu dimensionne chaque onglet sur son libellé : ses six onglets font 77, 83, 103, 83, 133 et
//! 106 px pour des encres de 26, 44, 73, 28, 100 et 35 px. **La règle qui produit ces largeurs
//! n'est pas retrouvable** — ni un padding constant, ni le nombre de caractères, ni une largeur
//! minimale unique n'en rendent compte. Et sa barre ne remplit pas la fenêtre : elle s'arrête à
//! x=636 sur 705, parce que le **bouton de réinitialisation** occupe la droite.
//!
//! Appliquer une règle qu'on n'a pas mesurée à trois onglets qui n'ont pas ce bouton donnerait une
//! barre courte, calée à gauche, sous un panneau pleine largeur. Les parts égales remplissent le
//! panneau, restent stables quand un libellé change, et ne prétendent pas mesurer ce qui ne l'est
//! pas.
//!
//! [`Tabs::fit_content`] rend l'autre comportement — chaque onglet à la largeur de son libellé
//! ([`tokens::TAB_PADDING_X`], plancher [`tokens::TAB_MIN_WIDTH`]). C'est ce qu'il faut pour
//! comparer à une capture du jeu, ou pour une barre qui ne doit pas s'étirer.
//!
//! ## Le rayon n'est pas sur l'onglet, il est sur la barre
//!
//! Vérifié sur l'asset : le premier segment a ses deux coins gauches arrondis, le dernier ses deux
//! coins droits, **les segments du milieu ont des coins parfaitement droits**. L'arrondi appartient
//! donc aux deux extrémités de la barre, pas à chaque onglet.
//!
//! L'escalier d'alpha, mesuré aux quatre angles, retire `4, 2, 1` pixels sur les trois premières
//! lignes et `1, 2, 3` sur les trois dernières — le haut est creusé d'un pixel de plus que le bas,
//! sur les deux côtés. Ce n'est donc pas du bruit de détourage mais la forme du jeu, et elle est
//! conservée telle quelle.
//!
//! **L'arrondi est porté par l'alpha de la texture**, comme celui d'un bouton, ce qui coûte quatre
//! fichiers de plus ([`DsTexture::TabActiveFirst`] et ses trois voisins). Les deux autres voies n'en
//! sont pas : un `Mesh` egui ne sait pas découper un coin, et peindre un patch arrondi par-dessus
//! n'efface pas le coin carré qui est dessous — l'alpha compose, il ne soustrait pas. Deux des
//! quatre fichiers sont des **miroirs horizontaux** des deux autres, faute de capture d'onglet actif
//! en fin de barre ; le corps d'un onglet étant un dégradé vertical, le miroir ne change rien.
//!
//! Une barre à **un seul onglet** n'existe pas dans le jeu, donc aucune texture n'a ses quatre coins
//! arrondis. Le composant peint alors les deux extrémités l'une sur l'autre, chacune écrêtée à sa
//! moitié.
//!
//! ## Ce qui n'a PAS de référence, et est donc inventé
//!
//! - **Le padding de [`Tabs::fit_content`].** Faute de règle retrouvable (ci-dessus), il retient le
//!   padding **stable sur les deux libellés longs** — 15 px sur « Interface », 16 sur
//!   « Commandes », d'où [`tokens::TAB_PADDING_X`] — et un plancher au plus petit onglet relevé
//!   ([`tokens::TAB_MIN_WIDTH`]). Les libellés courts ressortent donc plus étroits que dans le jeu.
//! - **L'état désactivé.** Aucune capture d'onglet grisé. Fond inactif et libellé `TEXT_DISABLED`,
//!   par cohérence avec le bouton désactivé — à remplacer par une mesure dès qu'une capture existe.
//!   Il existe dès maintenant parce que « Alertes » et « Personnages » sont exactement ce cas :
//!   présents, vides, pas encore cliquables.

use egui::{Align2, Response, Sense, Ui, Vec2, Widget};

use crate::design::{assets::DsTexture, text, tokens, DesignSystem};

/// État peint d'un onglet. Quatre, contre trois pour les autres composants : un onglet porte en plus
/// la notion d'être **celui qui est sélectionné**, qui n'a pas d'équivalent sur un bouton.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabState {
    Idle,
    Hovered,
    Active,
    Disabled,
}

/// Où l'onglet se trouve dans la barre — c'est ce qui décide de ses coins arrondis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Position {
    First,
    Middle,
    Last,
    /// Seul onglet de la barre : il porte les deux extrémités à la fois.
    Only,
}

impl Position {
    fn of(index: usize, count: usize) -> Self {
        match (index == 0, index + 1 == count) {
            (true, true) => Position::Only,
            (true, false) => Position::First,
            (false, true) => Position::Last,
            (false, false) => Position::Middle,
        }
    }
}

impl TabState {
    /// Fonds de l'état, dans l'ordre `(première position, milieu, dernière position)`.
    ///
    /// **Le survolé reprend ceux de l'actif**, c'est le piège documenté en tête de module : deux
    /// familles de textures pour quatre états.
    fn textures(self) -> (DsTexture, DsTexture, DsTexture) {
        match self {
            TabState::Active | TabState::Hovered => (
                DsTexture::TabActiveFirst,
                DsTexture::TabActive,
                DsTexture::TabActiveLast,
            ),
            TabState::Idle | TabState::Disabled => (
                DsTexture::TabInactiveFirst,
                DsTexture::TabInactive,
                DsTexture::TabInactiveLast,
            ),
        }
    }

    /// Libellé : c'est LUI qui distingue l'actif du survolé.
    fn label_color(self) -> egui::Color32 {
        match self {
            TabState::Active => tokens::TAB_LABEL_ACTIVE,
            TabState::Idle | TabState::Hovered => tokens::TAB_LABEL_IDLE,
            TabState::Disabled => tokens::TEXT_DISABLED,
        }
    }
}

struct Entry<T> {
    value: T,
    label: String,
    enabled: bool,
    /// Force l'état peint de CETTE entrée — voir [`Tabs::preview_state`].
    forced_state: Option<TabState>,
}

/// Construit une barre d'onglets sur `selected`. Point d'entrée unique — voir la doc de module.
pub fn tabs<T: PartialEq + Copy>(selected: &mut T) -> Tabs<'_, T> {
    Tabs::new(selected)
}

pub struct Tabs<'a, T> {
    selected: &'a mut T,
    entries: Vec<Entry<T>>,
    log_name: Option<String>,
    fit_content: bool,
}

impl<'a, T: PartialEq + Copy> Tabs<'a, T> {
    pub fn new(selected: &'a mut T) -> Self {
        Self {
            selected,
            entries: Vec::new(),
            log_name: None,
            fit_content: false,
        }
    }

    /// Ajoute une entrée à droite des précédentes. L'ordre d'appel est l'ordre d'affichage.
    pub fn entry(mut self, value: T, label: impl Into<String>) -> Self {
        self.entries.push(Entry {
            value,
            label: label.into(),
            enabled: true,
            forced_state: None,
        });
        self
    }

    /// Active ou désactive **la dernière entrée déclarée**. Sans effet si aucune ne l'a encore été —
    /// c'est ce qui permet d'écrire `.entry(…, "Alertes").enabled(false)` sans imbriquer un
    /// constructeur d'entrée.
    pub fn enabled(mut self, enabled: bool) -> Self {
        if let Some(last) = self.entries.last_mut() {
            last.enabled = enabled;
        }
        self
    }

    /// Force l'état peint de **la dernière entrée déclarée**, sans passer par l'interaction —
    /// réservé à la galerie de contrôle et aux captures, où aucun pointeur ne survole quoi que ce
    /// soit. Même rôle que `Button::preview_state`.
    pub fn preview_state(mut self, state: TabState) -> Self {
        if let Some(last) = self.entries.last_mut() {
            last.forced_state = Some(state);
        }
        self
    }

    /// Dimensionne chaque onglet sur son libellé au lieu de partager la largeur à égalité.
    ///
    /// **L'inverse du défaut**, qui étire la barre sur toute la largeur disponible — voir la doc de
    /// module pour pourquoi les parts égales l'emportent dans l'overlay. À réserver aux barres qui
    /// ne doivent pas s'étirer, et aux planches de comparaison avec une capture du jeu.
    pub fn fit_content(mut self) -> Self {
        self.fit_content = true;
        self
    }

    /// Nom d'instance pour la journalisation (défaut : `"tabs"`). À renseigner dès que deux barres
    /// coexistent, sinon les lignes d'`overlay-ui.<date>.log` sont indiscernables.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Alias ergonomique d'`ui.add(...)` — la forme qu'on lit le mieux quand le composant porte une
    /// liste d'entrées chaînées.
    pub fn show(self, ui: &mut Ui) -> Response {
        ui.add(self)
    }

    /// Largeur de chaque onglet, dans l'ordre. Séparée du rendu pour que la taille totale soit
    /// connue avant d'allouer quoi que ce soit.
    fn widths(&self, ui: &mut Ui) -> Vec<f32> {
        let count = self.entries.len();
        if count == 0 {
            return Vec::new();
        }

        if self.fit_content {
            let font = text::label_font(ui.ctx(), tokens::TAB_FONT_SIZE);
            return self
                .entries
                .iter()
                .map(|entry| {
                    let ink = ui
                        .fonts_mut(|f| {
                            f.layout_no_wrap(
                                entry.label.clone(),
                                font.clone(),
                                tokens::TAB_LABEL_ACTIVE,
                            )
                        })
                        .size()
                        .x;
                    (ink + 2.0 * tokens::TAB_PADDING_X).max(tokens::TAB_MIN_WIDTH)
                })
                .collect();
        }

        // Parts égales sur la largeur disponible, gouttières déduites. Les bords sont arrondis **en
        // cumulé** plutôt que chaque largeur séparément : sinon les arrondis s'additionnent et la
        // barre finit un ou deux pixels avant — ou après — le bord du panneau. Ici la somme des
        // largeurs vaut exactement la place utile, et les onglets ne diffèrent au plus que d'un
        // pixel entre eux.
        let gutters = tokens::TAB_SEPARATOR_WIDTH * (count - 1) as f32;
        let usable = (ui.available_width() - gutters).max(0.0);
        (0..count)
            .map(|index| {
                let start = (usable * index as f32 / count as f32).round();
                let end = (usable * (index + 1) as f32 / count as f32).round();
                end - start
            })
            .collect()
    }
}

/// Peint le fond d'un onglet, coins arrondis compris.
///
/// L'arrondi est porté par **l'alpha de la texture**, comme celui d'un bouton : un `Mesh` egui ne
/// sait pas découper un coin, et peindre un patch par-dessus n'efface pas le coin carré du dessous.
/// D'où quatre textures d'extrémité en plus des deux du milieu — voir la doc de module.
fn paint_face(
    design: &DesignSystem,
    painter: &egui::Painter,
    rect: egui::Rect,
    state: TabState,
    position: Position,
) {
    let (first, middle, last) = state.textures();
    let tint = egui::Color32::WHITE;
    match position {
        Position::First => design.paint(painter, rect, first, tint),
        Position::Middle => design.paint(painter, rect, middle, tint),
        Position::Last => design.paint(painter, rect, last, tint),
        // Aucune texture n'a ses quatre coins arrondis : le jeu n'a pas de barre à un seul onglet.
        // On peint les deux extrémités l'une sur l'autre, chacune écrêtée à sa moitié — leurs corps
        // sont identiques, seuls leurs coins diffèrent.
        Position::Only => {
            let (left, right) = rect.split_left_right_at_fraction(0.5);
            for (half, texture) in [(left, first), (right, last)] {
                let clipped = painter.with_clip_rect(half.intersect(painter.clip_rect()));
                design.paint(&clipped, rect, texture, tint);
            }
        }
    }
}

/// Peint le trait entre deux onglets : un **dégradé vertical**, sur le seul corps de l'onglet.
///
/// egui ne sait pas remplir un rectangle en dégradé — `rect_filled` prend une couleur unique. Le
/// maillage est le chemin normal pour ça : quatre paires de sommets, la couleur interpolée entre
/// elles par le GPU, ce qui reste exact à n'importe quelle hauteur de barre. Les deux plateaux sont
/// des bandes à couleur constante, la rampe la bande du milieu — voir la doc de module pour le
/// profil mesuré.
fn paint_separator(painter: &egui::Painter, rect: egui::Rect) {
    let ramp_start = rect.top() + rect.height() * tokens::TAB_SEPARATOR_RAMP_START;
    let ramp_end = rect.top() + rect.height() * tokens::TAB_SEPARATOR_RAMP_END;
    let stops = [
        (rect.top(), tokens::TAB_SEPARATOR_TOP),
        (ramp_start, tokens::TAB_SEPARATOR_TOP),
        (ramp_end, tokens::TAB_SEPARATOR_BOTTOM),
        (rect.bottom(), tokens::TAB_SEPARATOR_BOTTOM),
    ];

    let mut mesh = egui::Mesh::default();
    for (y, color) in stops {
        mesh.colored_vertex(egui::pos2(rect.left(), y), color);
        mesh.colored_vertex(egui::pos2(rect.right(), y), color);
    }
    for band in 0..stops.len() as u32 - 1 {
        let top_left = band * 2;
        mesh.add_triangle(top_left, top_left + 1, top_left + 2);
        mesh.add_triangle(top_left + 1, top_left + 3, top_left + 2);
    }
    painter.add(mesh);
}

impl<T: PartialEq + Copy> Widget for Tabs<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let name = self.log_name.clone().unwrap_or_else(|| "tabs".to_owned());
        let widths = self.widths(ui);

        // Une barre sans entrée n'est pas un cas de mise en page, c'est un appel oublié : elle
        // n'occupe rien et ne peint rien, ce qui laisse un trou muet dans le panneau. On le dit —
        // une fois, sinon la ligne reviendrait à chaque frame.
        if widths.is_empty() {
            let (_, response) = ui.allocate_exact_size(Vec2::ZERO, Sense::hover());
            let warned_id = response.id.with("ds-tabs-vide");
            let already = ui.data_mut(|d| {
                let seen = d.get_temp::<bool>(warned_id).unwrap_or(false);
                d.insert_temp(warned_id, true);
                seen
            });
            if !already {
                tracing::warn!(
                    component = "tabs",
                    name,
                    "barre d'onglets sans aucune entrée"
                );
            }
            return response;
        }

        let total = widths.iter().sum::<f32>()
            + tokens::TAB_SEPARATOR_WIDTH * (widths.len().saturating_sub(1)) as f32;

        let (rect, mut response) =
            ui.allocate_exact_size(Vec2::new(total, tokens::TAB_HEIGHT), Sense::hover());
        let font = text::label_font(ui.ctx(), tokens::TAB_FONT_SIZE);
        let design = DesignSystem::get(ui.ctx());
        // Un appui de souris retire l'apparence survolée partout dans l'overlay — même condition
        // que `design::button` et `design::icon_button`, sans quoi deux familles de contrôles se
        // comporteraient différemment sous la même souris.
        let pointer_down = ui.input(|i| i.pointer.any_down());

        let mut x = rect.left();
        let mut clicked: Option<(usize, T)> = None;
        let count = widths.len();
        for (index, (entry, width)) in self.entries.iter().zip(&widths).enumerate() {
            let tab_rect = egui::Rect::from_min_size(
                egui::pos2(x, rect.top()),
                Vec2::new(*width, tokens::TAB_HEIGHT),
            );
            // Un onglet désactivé garde `Sense::hover()` (son infobulle expliquera un jour pourquoi
            // il est grisé) mais perd `Sense::click()` : `clicked()` ne peut alors structurellement
            // pas être vrai, plutôt que d'être filtré après coup.
            let sense = if entry.enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let tab_response = ui.interact(tab_rect, response.id.with(index), sense);

            let active = *self.selected == entry.value;
            let state = entry.forced_state.unwrap_or(if !entry.enabled {
                TabState::Disabled
            } else if active {
                TabState::Active
            } else if tab_response.hovered() && !pointer_down {
                TabState::Hovered
            } else {
                TabState::Idle
            });

            if ui.is_rect_visible(tab_rect) {
                paint_face(
                    &design,
                    ui.painter(),
                    tab_rect,
                    state,
                    Position::of(index, count),
                );
                // Libellé écrêté à SON onglet : un libellé trop long ne doit pas déborder sur le
                // voisin, où il passerait pour un défaut de mise en page.
                let color = state.label_color();
                let galley = ui.fonts_mut(|f| {
                    f.layout_no_wrap(
                        entry.label.clone(),
                        font.clone(),
                        // La couleur est donnée au moment de peindre, pas à la mise en page : la
                        // même galley sert au cerne ET au texte.
                        egui::Color32::PLACEHOLDER,
                    )
                });
                text::paint_outlined_galley(
                    &ui.painter()
                        .with_clip_rect(tab_rect.intersect(ui.clip_rect())),
                    Align2::CENTER_CENTER
                        .align_size_within_rect(galley.size(), tab_rect)
                        .min,
                    &galley,
                    color,
                    text::dimmed(color, tokens::TAB_LABEL_OUTLINE_FACTOR),
                    text::OUTLINE_FULL,
                );
            }

            if entry.enabled {
                let tab_response = tab_response.on_hover_cursor(egui::CursorIcon::PointingHand);
                if tab_response.clicked() && !active {
                    clicked = Some((index, entry.value));
                }
            }

            // La gouttière de 2px qui suit — jamais après le dernier onglet. Elle porte deux
            // choses : le bord de la barre, qui la traverse de part en part, et le séparateur, qui
            // n'occupe que le corps entre ces deux bords.
            if index + 1 < widths.len() {
                let gutter = egui::Rect::from_min_size(
                    egui::pos2(tab_rect.right(), rect.top()),
                    Vec2::new(tokens::TAB_SEPARATOR_WIDTH, tokens::TAB_HEIGHT),
                );
                ui.painter().rect_filled(gutter, 0, tokens::TAB_BORDER);
                paint_separator(
                    ui.painter(),
                    gutter.shrink2(Vec2::new(0.0, tokens::TAB_BORDER_Y)),
                );
            }

            x = tab_rect.right() + tokens::TAB_SEPARATOR_WIDTH;
        }

        if let Some((index, value)) = clicked {
            *self.selected = value;
            response.mark_changed();
            tracing::debug!(
                component = "tabs",
                name,
                onglet = self.entries[index].label,
                "clic"
            );
        }

        response
    }
}
