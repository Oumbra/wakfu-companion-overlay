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
//! ## Le rayon n'est pas sur l'onglet, il est sur la barre
//!
//! Vérifié sur l'asset : le premier segment a un coin arrondi (rayon 4, escalier d'alpha sur quatre
//! colonnes), **les segments du milieu ont des coins parfaitement droits**. L'arrondi appartient donc
//! aux deux extrémités de la barre, pas à chaque onglet.
//!
//! **Ce rayon n'est pas reproduit** — écart assumé, quatre pixels sur deux coins. Le bord d'un
//! onglet est quasi noir (`#1c1e21`) et le fond de la modale qui l'entoure l'est tout autant
//! (`#1c2023`) : l'arrondi y est invisible à l'échelle 1:1. Le reproduire demanderait deux textures
//! de plus par état (extrémité gauche, extrémité droite) pour un pixel que personne ne voit.
//!
//! ## Ce qui n'a PAS de référence, et est donc inventé
//!
//! - **La règle de largeur du jeu n'est pas retrouvable.** Ses six onglets font 77, 83, 103, 83, 133
//!   et 106 px pour des encres de 26, 44, 73, 28, 100 et 35 px : ni un padding constant, ni le
//!   nombre de caractères, ni une largeur minimale unique n'en rendent compte. Le composant retient
//!   le padding **stable sur les deux libellés longs** — 15 px sur « Interface », 16 sur
//!   « Commandes », d'où [`tokens::TAB_PADDING_X`] — et un plancher au plus petit onglet relevé
//!   ([`tokens::TAB_MIN_WIDTH`]). Les libellés courts ressortent donc plus étroits que dans le jeu ;
//!   c'est le prix d'une règle qu'on n'a pas.
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

impl TabState {
    /// Fond : **le survolé reprend celui de l'actif**, c'est le piège documenté en tête de module.
    fn texture(self) -> DsTexture {
        match self {
            TabState::Active | TabState::Hovered => DsTexture::TabActive,
            TabState::Idle | TabState::Disabled => DsTexture::TabInactive,
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
}

impl<'a, T: PartialEq + Copy> Tabs<'a, T> {
    pub fn new(selected: &'a mut T) -> Self {
        Self {
            selected,
            entries: Vec::new(),
            log_name: None,
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
        let font = text::label_font(ui.ctx(), tokens::TAB_FONT_SIZE);
        self.entries
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
            .collect()
    }
}

impl<T: PartialEq + Copy> Widget for Tabs<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let widths = self.widths(ui);
        let total = widths.iter().sum::<f32>()
            + tokens::TAB_SEPARATOR_WIDTH * (widths.len().saturating_sub(1)) as f32;

        let (rect, mut response) =
            ui.allocate_exact_size(Vec2::new(total, tokens::TAB_HEIGHT), Sense::hover());
        let font = text::label_font(ui.ctx(), tokens::TAB_FONT_SIZE);
        let design = DesignSystem::get(ui.ctx());
        // Un appui de souris retire l'apparence survolée partout dans l'overlay — même condition
        // que `design::button` et `panels::icon_button`, sans quoi deux familles de contrôles se
        // comporteraient différemment sous la même souris.
        let pointer_down = ui.input(|i| i.pointer.any_down());
        let name = self.log_name.clone().unwrap_or_else(|| "tabs".to_owned());

        let mut x = rect.left();
        let mut clicked: Option<(usize, T)> = None;
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
                design.paint(
                    ui.painter(),
                    tab_rect,
                    state.texture(),
                    egui::Color32::WHITE,
                );
                // Libellé écrêté à SON onglet : un libellé trop long ne doit pas déborder sur le
                // voisin, où il passerait pour un défaut de mise en page.
                ui.painter()
                    .with_clip_rect(tab_rect.intersect(ui.clip_rect()))
                    .text(
                        tab_rect.center(),
                        Align2::CENTER_CENTER,
                        &entry.label,
                        font.clone(),
                        state.label_color(),
                    );
            }

            if entry.enabled {
                let tab_response = tab_response.on_hover_cursor(egui::CursorIcon::PointingHand);
                if tab_response.clicked() && !active {
                    clicked = Some((index, entry.value));
                }
            }

            // Séparateur clair, dans la gouttière de 2px qui suit — jamais après le dernier onglet.
            if index + 1 < widths.len() {
                let separator = egui::Rect::from_min_size(
                    egui::pos2(tab_rect.right(), rect.top()),
                    Vec2::new(tokens::TAB_SEPARATOR_WIDTH, tokens::TAB_HEIGHT),
                );
                ui.painter()
                    .rect_filled(separator, 0, tokens::TAB_SEPARATOR);
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
