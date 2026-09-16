//! **Switch à cases** du design system Wakfu — le sélecteur exclusif du jeu (choix du genre :
//! ♂ / ♀), une case par valeur, **une seule active à la fois**. Composant **feuille** (§1 du
//! contrat).
//!
//! ```ignore
//! use overlay_ui::design::{self, DsIcon};
//!
//! design::switch(&mut personnage.genre)
//!     .slot(Genre::Masculin, "Masculin").icon(DsIcon::Male)
//!     .slot(Genre::Feminin, "Féminin").icon(DsIcon::Female)
//!     .log_name("personnages.genre")
//!     .show(ui);
//!
//! // Trois positions — le sélecteur de grandeur du panneau Combat.
//! design::switch(&mut metric)
//!     .slot(CombatMetric::Damage, "Dégâts infligés")
//!     .slot(CombatMetric::Armor, "Armure donnée")
//!     .slot(CombatMetric::Heal, "Soins prodigués")
//!     .show(ui);
//! ```
//!
//! La valeur sélectionnée vit chez l'appelant, comme pour `design::tabs` ; `Response::changed()` dit
//! à quelle frame elle a bougé. Le composant ne décide jamais de ce que la sélection déclenche.
//!
//! ## Pourquoi pas un `tabs`
//!
//! C'est la question du contrat (« le composant existe-t-il déjà ? »), et la réponse est non : ce
//! n'est pas le même élément du jeu. Une barre d'onglets a un fond sombre `#363734` et un onglet
//! actif kaki au libellé **blanc**, un séparateur en dégradé, un cadre arrondi seulement aux bouts
//! d'une barre. Le switch a un **cadre** propre (liseré `#221f24`, rayon 6 aux quatre coins), un
//! séparateur plat, et ses glyphes changent de couleur — doré sur la case active, gris sur
//! l'inactive — là où l'onglet actif blanchit le sien. Six textures dédiées, tirées de deux
//! captures (`switch-first-slot-active.png` et `switch-second-slot-active.png`), aucune partagée
//! avec les onglets.
//!
//! ## Deux cases relevées, *n* cases servies
//!
//! Le jeu n'a capturé qu'un switch à deux cases. Une case du **milieu** n'a ni coin arrondi ni
//! liseré latéral : ses deux textures (`switch-slot-active.png`, `switch-slot-inactive.png`) sont
//! les 40px de remplissage des cases d'extrémité, coin redressé — la même dérivation que
//! `tab-active.png`. C'est ce qui permet un switch à trois positions (Dégâts / Armure / Soins du
//! panneau Combat) sans rien inventer d'autre que « le milieu ressemble aux bouts ». Une case
//! inactive du milieu n'a pas d'ombre intérieure : sur les captures, l'ombre est toujours du côté du
//! liseré extérieur, et une case sans liseré n'en porte pas.
//!
//! Deux cases au moins : un switch à une case n'est pas un switch, il n'est pas peint et le dit une
//! fois au journal — comme une barre d'onglets vide.
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
//! selon l'état (x 42–44 ou 44–46). Le composant donne la même largeur à toutes les cases
//! ([`tokens::SWITCH_SLOT_WIDTH`]) et laisse le 9-slice absorber l'écart — un pixel de chaque
//! côté, invisible.
//!
//! ## Le survol — la case survolée devient une case active
//!
//! Capture du 2026-09-16 (`switch-first-slot-active-and-second-slot-hover.png`, ♂ actif, souris
//! sur ♀) : la case inactive survolée prend **tout** l'aspect de la case active — fond kaki
//! `#635a47`, biseau clair de 2px en haut et en bas, glyphe doré, plus d'ombre intérieure côté
//! liseré. Écart moyen entre la case survolée et la case active de la référence : 1,0/255 (11,5
//! contre la case inactive). Le switch montre alors deux cases actives, indiscernables : c'est le
//! jeu, on ne l'améliore pas. Aucune texture de plus, `Hovered` peint les textures actives. (La
//! première version dorait le glyphe seul sans toucher au fond — inventé faute de capture, et
//! faux.)
//!
//! ## Ce que le jeu n'a pas montré
//!
//! - **L'état désactivé.** Aucune capture. Fonds atténués comme un bouton désactivé, glyphes
//!   [`tokens::TEXT_DISABLED`] ; la case sélectionnée reste reconnaissable à son fond.
//! - **Le milieu.** Voir plus haut : dérivé des extrémités.
//! - **Une hauteur autre que 44.** Deux voies, qui ne font pas la même chose :
//!   - `Switch::scale` **réduit tout** dans le même rapport — cases, séparateur, liseré, biseaux,
//!     glyphes — comme le jeu quand on baisse l'échelle de son interface. La texture entière de
//!     chaque case est peinte en un seul quad (plus de 9-slice : à l'échelle, il n'y a rien à
//!     figer), filtrée en bilinéaire par le GPU. C'est la voie du panneau Combat (36px, décision
//!     utilisateur du 2026-09-16 : « le rendu dans le jeu est imposant », mais « vraiment scaler
//!     à double dimension pour ne pas perdre le rendu visuel »). Les dimensions sont arrondies au
//!     pixel : à 36/44, une case fait 35 × 36, le séparateur 2.
//!   - `Switch::height` **impose la hauteur seule** : les 6px hauts et bas du 9-slice sont
//!     recopiés tels quels, seul le corps s'étire. À 26px il reste 14px de dégradé au lieu de 32,
//!     plus raide mais sans déformation des coins ni du liseré — c'est le rendu qui a fait dire
//!     « on a l'impression d'avoir compressé le switch » : à réserver aux écarts de quelques
//!     pixels.
//! - **Des glyphes en couleurs.** Le jeu teinte ses glyphes (doré / gris) ; un glyphe
//!   `DsIcon::native_color` est peint tel quel sur la case active et atténué
//!   ([`tokens::ICON_NATIVE_DIM`]) ailleurs — voir la catégorie `couleur` de `design::icons`.

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

    /// Teinte d'un glyphe **en couleurs** (`DsIcon::native_color`) : blanc — c'est-à-dire tel
    /// quel — sur la case active ou survolée, atténué ailleurs. Le désactivé cumule les deux
    /// atténuations (celle-ci et [`DISABLED_TINT`] sur le fond), comme un glyphe blanc y perd à
    /// la fois sa teinte et son fond.
    fn native_glyph_color(self) -> Color32 {
        match self {
            SwitchState::Active | SwitchState::Hovered => Color32::WHITE,
            SwitchState::Idle | SwitchState::Disabled => tokens::ICON_NATIVE_DIM,
        }
    }
}

/// Où la case se trouve dans le switch — c'est ce qui décide de ses coins arrondis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Position {
    First,
    Middle,
    Last,
}

impl Position {
    fn of(index: usize, count: usize) -> Self {
        if index == 0 {
            Position::First
        } else if index + 1 == count {
            Position::Last
        } else {
            Position::Middle
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
    slots: Vec<Slot<T>>,
    /// Largeur totale imposée — sinon [`Switch::natural_width`].
    width: Option<f32>,
    /// Hauteur imposée — sinon [`tokens::SWITCH_HEIGHT`].
    height: Option<f32>,
    /// Rapport d'échelle homothétique — voir [`Switch::scale`]. 1 = la taille du jeu.
    scale: f32,
    enabled: bool,
    log_name: Option<String>,
}

impl<'a, T: PartialEq + Copy> Switch<'a, T> {
    pub fn new(selected: &'a mut T) -> Self {
        Self {
            selected,
            slots: Vec::new(),
            width: None,
            height: None,
            scale: 1.0,
            enabled: true,
            log_name: None,
        }
    }

    /// Ajoute une case à droite des précédentes. L'ordre d'appel est l'ordre d'affichage ; il en
    /// faut **deux au moins**.
    pub fn slot(mut self, value: T, label: impl Into<String>) -> Self {
        self.slots.push(Slot {
            value,
            label: label.into(),
            icon: None,
            forced_state: None,
        });
        self
    }

    /// Donne un pictogramme à **la dernière case déclarée** : il remplace son libellé au rendu, et
    /// le libellé devient son infobulle. Sans effet avant le premier `slot`.
    pub fn icon(mut self, icon: DsIcon) -> Self {
        if let Some(last) = self.slots.last_mut() {
            last.icon = Some(icon);
        }
        self
    }

    /// Largeur totale imposée. Les cases se la partagent à égalité, séparateurs déduits. Sans
    /// elle, chaque case fait [`tokens::SWITCH_SLOT_WIDTH`] — les 88px du jeu pour deux cases.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Hauteur imposée — sinon [`tokens::SWITCH_HEIGHT`], celle de la texture. Voir « une hauteur
    /// autre que 44 » dans la doc de module : pas en dessous des 12px de marges figées du 9-slice
    /// plus un glyphe.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Réduit (ou agrandit) **tout le switch dans le même rapport** — cases, séparateur, liseré,
    /// biseaux et glyphes — comme le jeu quand on change l'échelle de son interface. Chaque case
    /// est alors peinte en un seul quad filtré, sans 9-slice. Les dimensions sont arrondies au
    /// pixel (case de 43 → 35 à `36/44`). `width` et `height`, s'ils sont donnés, priment sur la
    /// taille ainsi calculée. Voir « une hauteur autre que 44 » dans la doc de module.
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Active ou désactive **le switch entier** — toutes les cases ensemble, il n'y a pas de sens
    /// à n'en désactiver qu'une.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Force l'état peint de **la dernière case déclarée**, sans passer par l'interaction —
    /// réservé à la galerie de contrôle et aux captures, où aucun pointeur ne survole quoi que ce
    /// soit. Même rôle que `Button::preview_state`.
    pub fn preview_state(mut self, state: SwitchState) -> Self {
        if let Some(last) = self.slots.last_mut() {
            last.forced_state = Some(state);
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

    /// Largeur du séparateur à l'échelle — au pixel, jamais sous 1 : une gouttière de 1,6px
    /// serait peinte floue sur deux colonnes.
    fn separator_width(&self) -> f32 {
        (tokens::SWITCH_SEPARATOR_WIDTH * self.scale)
            .round()
            .max(1.0)
    }

    /// Largeur sans contrainte : `n` cases de [`tokens::SWITCH_SLOT_WIDTH`] et `n − 1`
    /// séparateurs, à l'échelle.
    fn natural_width(&self) -> f32 {
        let n = self.slots.len() as f32;
        n * (tokens::SWITCH_SLOT_WIDTH * self.scale).round()
            + (n - 1.0).max(0.0) * self.separator_width()
    }

    /// Taille que le switch occupera, sans le dessiner.
    pub fn desired_size(&self) -> Vec2 {
        Vec2::new(
            self.width.unwrap_or_else(|| self.natural_width()),
            self.height
                .unwrap_or_else(|| (tokens::SWITCH_HEIGHT * self.scale).round()),
        )
    }
}

/// Fond d'une case selon sa position et son état. Le survolé prend le fond de l'actif — c'est ce
/// que le jeu fait (voir la doc de module, « le survol »).
fn slot_texture(position: Position, state: SwitchState, selected: bool) -> DsTexture {
    let active = match state {
        SwitchState::Active | SwitchState::Hovered => true,
        SwitchState::Idle => false,
        // Désactivé, la case sélectionnée garde son fond : c'est ce qui la laisse reconnaissable.
        SwitchState::Disabled => selected,
    };
    match (position, active) {
        (Position::First, true) => DsTexture::SwitchSlotActiveFirst,
        (Position::First, false) => DsTexture::SwitchSlotInactiveFirst,
        (Position::Middle, true) => DsTexture::SwitchSlotActive,
        (Position::Middle, false) => DsTexture::SwitchSlotInactive,
        (Position::Last, true) => DsTexture::SwitchSlotActiveLast,
        (Position::Last, false) => DsTexture::SwitchSlotInactiveLast,
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

        // Moins de deux cases n'est pas un cas de mise en page, c'est un appel oublié : rien n'est
        // peint, et on le dit — une fois, sinon la ligne reviendrait à chaque frame.
        if self.slots.len() < 2 {
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
                    cases = self.slots.len(),
                    "switch incomplet : il faut deux cases au moins"
                );
            }
            return response;
        }

        let size = self.desired_size();
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::hover());
        let design = DesignSystem::get(ui.ctx());
        // Un appui de souris retire l'apparence survolée partout dans l'overlay — même condition
        // que `design::button`, `design::icon_button` et `design::tabs`.
        let pointer_down = ui.input(|i| i.pointer.any_down());
        let count = self.slots.len();
        let separator_width = self.separator_width();
        let slot_width = (rect.width() - separator_width * (count - 1) as f32) / count as f32;
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        // Atténuation des fonds ET de la gouttière quand le switch est désactivé — la même
        // opacité partout, sinon la gouttière resterait la seule chose franche du cadre.
        let (tint, dim) = if self.enabled {
            (Color32::WHITE, 1.0)
        } else {
            (DISABLED_TINT, DISABLED_TINT.a() as f32 / 255.0)
        };

        let mut clicked: Option<(usize, T)> = None;
        for (index, slot) in self.slots.iter().enumerate() {
            let left = rect.left() + index as f32 * (slot_width + separator_width);
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
                let texture = slot_texture(Position::of(index, count), state, selected);
                if self.scale == 1.0 {
                    design.paint(ui.painter(), slot_rect, texture, tint);
                } else {
                    // À l'échelle, la texture entière en un quad : coins, liseré et biseaux se
                    // réduisent avec le reste, c'est le but (doc de module).
                    design.paint_region(
                        ui.painter(),
                        slot_rect,
                        texture,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        tint,
                    );
                }

                // Le glyphe (ou le libellé de repli) est écrêté à SA case : un pictogramme trop
                // large ne doit pas déborder sur la voisine.
                let painter = ui
                    .painter()
                    .with_clip_rect(slot_rect.intersect(ui.clip_rect()));
                if let Some(icon) = slot.icon {
                    let color = if icon.native_color() {
                        state.native_glyph_color()
                    } else {
                        state.glyph_color()
                    };
                    let drawn = glyph_size(design.icon_native_size(icon)) * self.scale;
                    // Calé sur la grille de pixels : une case de 43px met son centre à une
                    // demi-position, et un glyphe de 14px peint à x + 0,5 s'étale sur deux
                    // colonnes — flou visible à ×8 sur la comparaison au jeu.
                    let min =
                        (slot_rect.center() - drawn * 0.5).round_to_pixels(ui.pixels_per_point());
                    design.paint_icon(&painter, egui::Rect::from_min_size(min, drawn), icon, color);
                } else {
                    let color = state.glyph_color();
                    let font = text::label_font(ui.ctx(), tokens::SWITCH_FONT_SIZE);
                    let galley = painter.layout_no_wrap(slot.label.clone(), font, color);
                    let pos = Align2::CENTER_CENTER
                        .align_size_within_rect(galley.size(), slot_rect)
                        .min;
                    painter.galley(pos, galley, color);
                }

                // La gouttière qui suit — jamais après la dernière case. Aucune texture ne la
                // porte : le liseré du cadre la traverse de part en part, et le séparateur
                // n'occupe que le corps entre les deux liserés.
                if index + 1 < count {
                    let gutter = egui::Rect::from_min_size(
                        egui::pos2(slot_rect.right(), rect.top()),
                        Vec2::new(separator_width, rect.height()),
                    );
                    ui.painter()
                        .rect_filled(gutter, 0, tokens::SWITCH_BORDER.gamma_multiply(dim));
                    ui.painter().rect_filled(
                        gutter.shrink2(Vec2::new(
                            0.0,
                            (tokens::SWITCH_BORDER_Y * self.scale).round(),
                        )),
                        0,
                        tokens::SWITCH_SEPARATOR.gamma_multiply(dim),
                    );
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

        if let Some((index, value)) = clicked {
            *self.selected = value;
            response.mark_changed();
            tracing::debug!(
                component = "switch",
                name,
                case = self.slots[index].label,
                "clic"
            );
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La case sélectionnée garde son fond actif dans tous les états qui le permettent, et la
    /// case survolée prend le fond actif elle aussi — mesuré sur la capture du 2026-09-16.
    #[test]
    fn le_fond_suit_la_selection_et_le_survol() {
        assert_eq!(
            slot_texture(Position::First, SwitchState::Active, true),
            DsTexture::SwitchSlotActiveFirst
        );
        assert_eq!(
            slot_texture(Position::Last, SwitchState::Active, true),
            DsTexture::SwitchSlotActiveLast
        );
        assert_eq!(
            slot_texture(Position::First, SwitchState::Hovered, false),
            DsTexture::SwitchSlotActiveFirst
        );
        assert_eq!(
            slot_texture(Position::Middle, SwitchState::Hovered, false),
            DsTexture::SwitchSlotActive
        );
        assert_eq!(
            slot_texture(Position::Last, SwitchState::Idle, false),
            DsTexture::SwitchSlotInactiveLast
        );
    }

    /// À l'échelle 36/44, tout est arrondi au pixel : case de 35, séparateur de 2, hauteur 36 —
    /// deux cases font 72, trois 109.
    #[test]
    fn l_echelle_reduit_tout_au_pixel_pres() {
        let mut v = 0u8;
        let deux = switch(&mut v).slot(0, "a").slot(1, "b").scale(36.0 / 44.0);
        assert_eq!(deux.desired_size(), Vec2::new(72.0, 36.0));
        let mut v = 0u8;
        let trois = switch(&mut v)
            .slot(0, "a")
            .slot(1, "b")
            .slot(2, "c")
            .scale(36.0 / 44.0);
        assert_eq!(trois.desired_size(), Vec2::new(109.0, 36.0));
        // Une largeur imposée prime sur l'échelle.
        let mut v = 0u8;
        let impose = switch(&mut v)
            .slot(0, "a")
            .slot(1, "b")
            .scale(0.5)
            .width(100.0);
        assert_eq!(impose.desired_size(), Vec2::new(100.0, 22.0));
    }

    /// Une case du milieu prend les textures sans coin — les seules qui conviennent entre deux
    /// séparateurs.
    #[test]
    fn la_case_du_milieu_n_a_pas_de_coin() {
        assert_eq!(
            slot_texture(Position::Middle, SwitchState::Active, true),
            DsTexture::SwitchSlotActive
        );
        assert_eq!(
            slot_texture(Position::Middle, SwitchState::Idle, false),
            DsTexture::SwitchSlotInactive
        );
        assert_eq!(Position::of(0, 3), Position::First);
        assert_eq!(Position::of(1, 3), Position::Middle);
        assert_eq!(Position::of(2, 3), Position::Last);
        // À deux cases, il n'y a pas de milieu.
        assert_eq!(Position::of(1, 2), Position::Last);
    }

    /// Désactivé, la case sélectionnée reste reconnaissable à son fond.
    #[test]
    fn desactive_garde_le_fond_de_la_case_selectionnee() {
        assert_eq!(
            slot_texture(Position::First, SwitchState::Disabled, true),
            DsTexture::SwitchSlotActiveFirst
        );
        assert_eq!(
            slot_texture(Position::Last, SwitchState::Disabled, false),
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
            .slot(0, "A")
            .icon(DsIcon::Male)
            .slot(1, "B")
            .preview_state(SwitchState::Hovered);
        assert_eq!(switch.slots.len(), 2);
        assert_eq!(switch.slots[0].icon, Some(DsIcon::Male));
        assert_eq!(switch.slots[0].forced_state, None);
        assert_eq!(switch.slots[1].icon, None);
        assert_eq!(switch.slots[1].forced_state, Some(SwitchState::Hovered));
    }

    /// La largeur par défaut est la mesure du jeu — 88 × 44 pour deux cases — et suit le nombre
    /// de cases : 43 par case, 2 par séparateur.
    #[test]
    fn la_taille_par_defaut_est_celle_du_jeu_et_suit_le_nombre_de_cases() {
        let mut selected = 0_u8;
        let deux = Switch::new(&mut selected).slot(0, "A").slot(1, "B");
        assert_eq!(deux.desired_size(), Vec2::new(88.0, 44.0));
        let mut selected = 0_u8;
        let trois = Switch::new(&mut selected)
            .slot(0, "A")
            .slot(1, "B")
            .slot(2, "C");
        assert_eq!(trois.desired_size(), Vec2::new(133.0, 44.0));
        let mut selected = 0_u8;
        let impose = Switch::new(&mut selected)
            .slot(0, "A")
            .slot(1, "B")
            .width(200.0);
        assert_eq!(impose.desired_size(), Vec2::new(200.0, 44.0));
        let mut selected = 0_u8;
        let abaisse = Switch::new(&mut selected)
            .slot(0, "A")
            .slot(1, "B")
            .width(70.0)
            .height(26.0);
        assert_eq!(abaisse.desired_size(), Vec2::new(70.0, 26.0));
    }

    #[test]
    fn un_glyphe_en_couleurs_est_peint_tel_quel_puis_attenue() {
        assert!(DsIcon::Allies.native_color());
        assert!(!DsIcon::Male.native_color());
        assert_eq!(SwitchState::Active.native_glyph_color(), Color32::WHITE);
        assert_eq!(SwitchState::Hovered.native_glyph_color(), Color32::WHITE);
        assert_eq!(
            SwitchState::Idle.native_glyph_color(),
            tokens::ICON_NATIVE_DIM
        );
    }
}
