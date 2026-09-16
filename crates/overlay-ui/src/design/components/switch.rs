//! **Switch à deux cases** du design system Wakfu — le sélecteur exclusif du jeu (choix du genre :
//! ♂ / ♀), une case pour chaque valeur, une seule active à la fois. Composant **feuille** (§1 du
//! contrat).
//!
//! ```ignore
//! use overlay_ui::design::{self, DsIcon};
//!
//! design::switch(&mut personnage.genre)
//!     .first(Genre::Masculin, "Masculin").icon(DsIcon::Male)
//!     .second(Genre::Feminin, "Féminin").icon(DsIcon::Female)
//!     .log_name("personnages.genre")
//!     .show(ui);
//! ```
//!
//! La valeur sélectionnée vit chez l'appelant, comme pour `design::tabs` ; `Response::changed()` dit
//! à quelle frame elle a bougé. Le composant ne décide jamais de ce que la sélection déclenche.
//!
//! ## Pourquoi pas un `tabs` à deux entrées
//!
//! C'est la question du contrat (« le composant existe-t-il déjà ? »), et la réponse est non : ce
//! n'est pas le même élément du jeu. Une barre d'onglets a un fond sombre `#363734` et un onglet
//! actif kaki au libellé **blanc**, un séparateur en dégradé, un cadre arrondi seulement aux bouts
//! d'une barre qui peut compter six onglets. Le switch a un **cadre** propre (liseré `#221f24`,
//! rayon 6 aux quatre coins), un séparateur plat, exactement deux cases, et ses glyphes changent de
//! couleur — doré sur la case active, gris sur l'inactive — là où l'onglet actif blanchit le sien.
//! Quatre textures dédiées, tirées de deux captures (`switch-first-slot-active.png` et
//! `switch-second-slot-active.png`), et aucune partagée avec les onglets.
//!
//! ## Deux positions, pas une liste
//!
//! `first` et `second`, et pas un `entry` répétable : un switch a deux positions, ni plus ni moins,
//! et l'API le dit plutôt que de le vérifier après coup. Un switch auquel il manque une case n'est
//! pas peint — c'est un appel oublié, signalé une fois au journal comme une barre d'onglets vide.
//!
//! ## Ce que le glyphe porte, et ce que le libellé porte
//!
//! Le pictogramme est le cas nominal — c'est le seul que le jeu montre. Le libellé reste
//! obligatoire pour les mêmes raisons que sur un onglet à pictogramme : il est l'**infobulle** de
//! la case et sa **ligne de journal**. Sans pictogramme, il est peint dans la case, au corps des
//! libellés du jeu ([`tokens::SWITCH_FONT_SIZE`]) — un repli **non vérifié contre une capture**,
//! le jeu n'ayant pas de switch texte dans les interfaces relevées.
//!
//! ## Les mesures
//!
//! Relevé `ui-blueprint` du 2026-09-16 sur l'asset générique 88 × 44 :
//!
//! | Grandeur | Valeur |
//! | --- | --- |
//! | Cadre | 88 × 44, liseré 2px `#221f24`, rayon 6 (porté par l'alpha des textures d'extrémité) |
//! | Case active | 40 × 40, kaki `#635a47`, biseau clair 2px en haut et en bas |
//! | Case inactive | 42 × 40, gris-brun `#514b44`, ombre intérieure 2px côté liseré |
//! | Séparateur | 2px, `#312d2d`, entre les deux liserés (y 2..42) |
//! | Glyphes | ♂ 14 × 14 et ♀ 10 × 16, à leur taille native, centrés dans leur case |
//! | Glyphe actif / inactif | `#f4d89f` / `#a9a5a2` |
//!
//! **La case inactive est 2px plus large que l'active**, et le séparateur se déplace donc de 2px
//! selon l'état (x 42–44 ou 44–46). Le composant donne la même largeur aux deux cases et laisse le
//! 9-slice absorber l'écart — un pixel de chaque côté, invisible.
//!
//! ## Ce que le jeu n'a pas montré
//!
//! - **Le survol.** Aucune capture. La case inactive survolée prend la teinte de glyphe de la case
//!   active (doré), sans changer de fond — le signal le plus discret qui reste lisible, et celui
//!   du bouton icône de premier plan (gris → or). À remplacer par une mesure.
//! - **L'état désactivé.** Aucune capture. Fonds atténués comme un bouton désactivé, glyphes
//!   [`tokens::TEXT_DISABLED`] ; la case sélectionnée reste reconnaissable à son fond.

use egui::emath::GuiRounding as _;
use egui::{Align2, Color32, Response, Sense, Ui, Vec2, Widget};

use crate::design::{assets::DsTexture, text, tokens, DesignSystem, DsIcon};

/// État visuel d'une case. Quatre et non trois : une case porte en plus la notion d'être **celle
/// qui est sélectionnée** — la même extension que `TabState`.
///
/// Pas d'état « pressé » (décision utilisateur 2026-09-09, commune à tous les composants) :
/// l'appui retire l'apparence survolée, elle revient au relâchement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchState {
    Idle,
    Hovered,
    Active,
    Disabled,
}

impl SwitchState {
    /// Teinte du glyphe — ou du libellé de repli.
    fn glyph_color(self) -> Color32 {
        match self {
            SwitchState::Active | SwitchState::Hovered => tokens::SWITCH_ICON_ACTIVE,
            SwitchState::Idle => tokens::SWITCH_ICON_INACTIVE,
            SwitchState::Disabled => tokens::TEXT_DISABLED,
        }
    }
}

/// Atténuation des fonds d'un switch désactivé — la même que celle d'un bouton désactivé
/// (`button::DISABLED_TINT`) : un multiplicateur d'alpha, pour que l'état se lise aussi sur une
/// capture statique.
const DISABLED_TINT: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 190);

struct Slot<T> {
    value: T,
    label: String,
    /// Pictogramme qui **remplace** le libellé au rendu — voir [`Switch::icon`].
    icon: Option<DsIcon>,
    /// Force l'état peint de CETTE case — voir [`Switch::preview_state`].
    forced_state: Option<SwitchState>,
}

/// Construit un switch sur `selected`. Point d'entrée unique — voir la doc de module.
pub fn switch<T: PartialEq + Copy>(selected: &mut T) -> Switch<'_, T> {
    Switch::new(selected)
}

pub struct Switch<'a, T> {
    selected: &'a mut T,
    slots: [Option<Slot<T>>; 2],
    /// Index de la dernière case déclarée — cible de `icon` et `preview_state`.
    last: Option<usize>,
    width: f32,
    enabled: bool,
    log_name: Option<String>,
}

impl<'a, T: PartialEq + Copy> Switch<'a, T> {
    pub fn new(selected: &'a mut T) -> Self {
        Self {
            selected,
            slots: [None, None],
            last: None,
            width: tokens::SWITCH_WIDTH,
            enabled: true,
            log_name: None,
        }
    }

    fn slot(mut self, index: usize, value: T, label: impl Into<String>) -> Self {
        self.slots[index] = Some(Slot {
            value,
            label: label.into(),
            icon: None,
            forced_state: None,
        });
        self.last = Some(index);
        self
    }

    /// La case de gauche.
    pub fn first(self, value: T, label: impl Into<String>) -> Self {
        self.slot(0, value, label)
    }

    /// La case de droite.
    pub fn second(self, value: T, label: impl Into<String>) -> Self {
        self.slot(1, value, label)
    }

    /// Donne un pictogramme à **la dernière case déclarée** : il remplace son libellé au rendu, et
    /// le libellé devient son infobulle. Sans effet avant `first`/`second`.
    pub fn icon(mut self, icon: DsIcon) -> Self {
        if let Some(slot) = self.last.and_then(|i| self.slots[i].as_mut()) {
            slot.icon = Some(icon);
        }
        self
    }

    /// Largeur totale imposée (défaut : [`tokens::SWITCH_WIDTH`], les 88px du jeu). Les deux
    /// cases se la partagent à égalité, séparateur déduit. La hauteur, elle, est toujours celle
    /// de la texture.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Active ou désactive **le switch entier** — les deux cases ensemble, il n'y a pas de sens à
    /// n'en désactiver qu'une.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Force l'état peint de **la dernière case déclarée**, sans passer par l'interaction —
    /// réservé à la galerie de contrôle et aux captures, où aucun pointeur ne survole quoi que ce
    /// soit. Même rôle que `Button::preview_state`.
    pub fn preview_state(mut self, state: SwitchState) -> Self {
        if let Some(slot) = self.last.and_then(|i| self.slots[i].as_mut()) {
            slot.forced_state = Some(state);
        }
        self
    }

    /// Nom d'instance pour la journalisation (défaut : `"switch"`). À renseigner dès que deux
    /// switches coexistent, sinon leurs lignes de journal sont indiscernables.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Alias d'`ui.add(self)`, plus lisible en bout de chaîne.
    pub fn show(self, ui: &mut Ui) -> Response {
        ui.add(self)
    }

    /// Taille que le switch occupera, sans le dessiner.
    pub fn desired_size(&self) -> Vec2 {
        Vec2::new(self.width, tokens::SWITCH_HEIGHT)
    }
}

/// Fond d'une case selon sa position et son état. Le survolé garde le fond de l'inactif : seul
/// le glyphe change (voir la doc de module, « ce que le jeu n'a pas montré »).
fn slot_texture(index: usize, state: SwitchState, selected: bool) -> DsTexture {
    let active = match state {
        SwitchState::Active => true,
        SwitchState::Idle | SwitchState::Hovered => false,
        // Désactivé, la case sélectionnée garde son fond : c'est ce qui la laisse reconnaissable.
        SwitchState::Disabled => selected,
    };
    match (index, active) {
        (0, true) => DsTexture::SwitchSlotActiveFirst,
        (0, false) => DsTexture::SwitchSlotInactiveFirst,
        (_, true) => DsTexture::SwitchSlotActiveLast,
        (_, false) => DsTexture::SwitchSlotInactiveLast,
    }
}

/// Taille de peinture d'un glyphe : **native s'il tient dans le carré**, réduite sinon.
///
/// C'est la nuance de [`tokens::SWITCH_ICON_SIZE`] : le jeu peint ses deux glyphes à leur taille
/// de fichier (14 et 16 de haut), un étalon commun grossirait le ♂. Seul un glyphe plus grand que
/// le carré — un pictogramme de 22px emprunté à une autre famille — est ramené dedans, en gardant
/// son rapport (`glyph_fit`).
fn glyph_size(native: Vec2) -> Vec2 {
    if native.x.max(native.y) <= tokens::SWITCH_ICON_SIZE {
        native
    } else {
        super::icon_button::glyph_fit(native, tokens::SWITCH_ICON_SIZE)
    }
}

impl<T: PartialEq + Copy> Widget for Switch<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let name = self.log_name.clone().unwrap_or_else(|| "switch".to_owned());

        // Une case manquante n'est pas un cas de mise en page, c'est un appel oublié : rien n'est
        // peint, et on le dit — une fois, sinon la ligne reviendrait à chaque frame.
        let [Some(first), Some(second)] = &self.slots else {
            let (_, response) = ui.allocate_exact_size(Vec2::ZERO, Sense::hover());
            let warned_id = response.id.with("ds-switch-incomplet");
            let already = ui.data_mut(|d| {
                let seen = d.get_temp::<bool>(warned_id).unwrap_or(false);
                d.insert_temp(warned_id, true);
                seen
            });
            if !already {
                tracing::warn!(
                    component = "switch",
                    name,
                    "switch incomplet : il faut `first` ET `second`"
                );
            }
            return response;
        };

        let size = self.desired_size();
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::hover());
        let design = DesignSystem::get(ui.ctx());
        // Un appui de souris retire l'apparence survolée partout dans l'overlay — même condition
        // que `design::button`, `design::icon_button` et `design::tabs`.
        let pointer_down = ui.input(|i| i.pointer.any_down());
        let slot_width = (rect.width() - tokens::SWITCH_SEPARATOR_WIDTH) / 2.0;
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };

        let mut clicked: Option<(usize, T)> = None;
        for (index, slot) in [first, second].into_iter().enumerate() {
            let left = rect.left() + index as f32 * (slot_width + tokens::SWITCH_SEPARATOR_WIDTH);
            let slot_rect = egui::Rect::from_min_size(
                egui::pos2(left, rect.top()),
                Vec2::new(slot_width, rect.height()),
            );
            let slot_response = ui.interact(slot_rect, response.id.with(index), sense);

            let selected = *self.selected == slot.value;
            let state = slot.forced_state.unwrap_or(if !self.enabled {
                SwitchState::Disabled
            } else if selected {
                SwitchState::Active
            } else if slot_response.hovered() && !pointer_down {
                SwitchState::Hovered
            } else {
                SwitchState::Idle
            });

            if ui.is_rect_visible(slot_rect) {
                let tint = if state == SwitchState::Disabled {
                    DISABLED_TINT
                } else {
                    Color32::WHITE
                };
                design.paint(
                    ui.painter(),
                    slot_rect,
                    slot_texture(index, state, selected),
                    tint,
                );

                // Le glyphe (ou le libellé de repli) est écrêté à SA case : un pictogramme trop
                // large ne doit pas déborder sur la voisine.
                let painter = ui
                    .painter()
                    .with_clip_rect(slot_rect.intersect(ui.clip_rect()));
                let color = state.glyph_color();
                if let Some(icon) = slot.icon {
                    let drawn = glyph_size(design.icon_native_size(icon));
                    // Calé sur la grille de pixels : une case de 43px met son centre à une
                    // demi-position, et un glyphe de 14px peint à x + 0,5 s'étale sur deux
                    // colonnes — flou visible à ×8 sur la comparaison au jeu.
                    let min =
                        (slot_rect.center() - drawn * 0.5).round_to_pixels(ui.pixels_per_point());
                    design.paint_icon(&painter, egui::Rect::from_min_size(min, drawn), icon, color);
                } else {
                    let font = text::label_font(ui.ctx(), tokens::SWITCH_FONT_SIZE);
                    let galley = painter.layout_no_wrap(slot.label.clone(), font, color);
                    let pos = Align2::CENTER_CENTER
                        .align_size_within_rect(galley.size(), slot_rect)
                        .min;
                    painter.galley(pos, galley, color);
                }
            }

            // L'infobulle d'une case à pictogramme porte son libellé — posée hors du test de
            // visibilité, une infobulle s'affiche sur interaction, pas sur peinture.
            if slot.icon.is_some() {
                crate::design::tooltip(&slot_response).text(slot.label.clone());
            }

            if self.enabled {
                let slot_response = slot_response.on_hover_cursor(egui::CursorIcon::PointingHand);
                if slot_response.clicked() && !selected {
                    clicked = Some((index, slot.value));
                }
            }
        }

        // La gouttière entre les deux cases, qu'aucune texture ne porte : le liseré du cadre la
        // traverse de part en part, et le séparateur n'occupe que le corps entre les deux liserés.
        if ui.is_rect_visible(rect) {
            let gutter = egui::Rect::from_min_size(
                egui::pos2(rect.left() + slot_width, rect.top()),
                Vec2::new(tokens::SWITCH_SEPARATOR_WIDTH, rect.height()),
            );
            // Atténuée comme les fonds quand le switch est désactivé — la même opacité que
            // `DISABLED_TINT`, sinon la gouttière resterait la seule chose franche du cadre.
            let dim = if self.enabled {
                1.0
            } else {
                DISABLED_TINT.a() as f32 / 255.0
            };
            ui.painter()
                .rect_filled(gutter, 0, tokens::SWITCH_BORDER.gamma_multiply(dim));
            ui.painter().rect_filled(
                gutter.shrink2(Vec2::new(0.0, tokens::SWITCH_BORDER_Y)),
                0,
                tokens::SWITCH_SEPARATOR.gamma_multiply(dim),
            );
        }

        if let Some((index, value)) = clicked {
            *self.selected = value;
            response.mark_changed();
            let label = self.slots[index]
                .as_ref()
                .map(|s| s.label.as_str())
                .unwrap_or_default();
            tracing::debug!(component = "switch", name, case = label, "clic");
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La case sélectionnée garde son fond actif dans tous les états qui le permettent, et le
    /// survol ne change PAS le fond : le jeu n'a pas montré de survol, le glyphe seul le signale.
    #[test]
    fn le_fond_suit_la_selection_pas_le_survol() {
        assert_eq!(
            slot_texture(0, SwitchState::Active, true),
            DsTexture::SwitchSlotActiveFirst
        );
        assert_eq!(
            slot_texture(1, SwitchState::Active, true),
            DsTexture::SwitchSlotActiveLast
        );
        assert_eq!(
            slot_texture(0, SwitchState::Hovered, false),
            DsTexture::SwitchSlotInactiveFirst
        );
        assert_eq!(
            slot_texture(1, SwitchState::Idle, false),
            DsTexture::SwitchSlotInactiveLast
        );
    }

    /// Désactivé, la case sélectionnée reste reconnaissable à son fond.
    #[test]
    fn desactive_garde_le_fond_de_la_case_selectionnee() {
        assert_eq!(
            slot_texture(0, SwitchState::Disabled, true),
            DsTexture::SwitchSlotActiveFirst
        );
        assert_eq!(
            slot_texture(1, SwitchState::Disabled, false),
            DsTexture::SwitchSlotInactiveLast
        );
    }

    /// Les deux glyphes du jeu sont peints à leur taille native : ♂ 14 × 14 ne grossit pas à 16.
    #[test]
    fn un_glyphe_qui_tient_dans_le_carre_reste_natif() {
        assert_eq!(glyph_size(Vec2::new(14.0, 14.0)), Vec2::new(14.0, 14.0));
        assert_eq!(glyph_size(Vec2::new(10.0, 16.0)), Vec2::new(10.0, 16.0));
    }

    /// Un pictogramme plus grand que le carré y est ramené, rapport conservé.
    #[test]
    fn un_glyphe_trop_grand_est_reduit_au_carre() {
        let drawn = glyph_size(Vec2::new(22.0, 22.0));
        assert!((drawn.x - 16.0).abs() < 1e-4 && (drawn.y - 16.0).abs() < 1e-4);
        let drawn = glyph_size(Vec2::new(22.0, 18.0));
        assert!((drawn.x - 16.0).abs() < 1e-4);
        assert!((drawn.y - 16.0 * 18.0 / 22.0).abs() < 1e-4);
    }

    /// `icon` et `preview_state` visent la dernière case déclarée, et sont sans effet avant.
    #[test]
    fn icon_et_preview_state_visent_la_derniere_case() {
        let mut selected = 0_u8;
        let switch = Switch::new(&mut selected)
            .icon(DsIcon::Male)
            .first(0, "A")
            .icon(DsIcon::Male)
            .second(1, "B")
            .preview_state(SwitchState::Hovered);
        let [Some(a), Some(b)] = &switch.slots else {
            panic!("deux cases déclarées");
        };
        assert_eq!(a.icon, Some(DsIcon::Male));
        assert_eq!(a.forced_state, None);
        assert_eq!(b.icon, None);
        assert_eq!(b.forced_state, Some(SwitchState::Hovered));
    }

    /// Les gabarits sont les mesures du jeu : 88 × 44, deux cases de 43 autour d'un séparateur
    /// de 2.
    #[test]
    fn la_taille_par_defaut_est_celle_du_jeu() {
        let mut selected = 0_u8;
        let switch = Switch::new(&mut selected);
        assert_eq!(switch.desired_size(), Vec2::new(88.0, 44.0));
        assert_eq!(
            (tokens::SWITCH_WIDTH - tokens::SWITCH_SEPARATOR_WIDTH) / 2.0,
            43.0
        );
    }
}
