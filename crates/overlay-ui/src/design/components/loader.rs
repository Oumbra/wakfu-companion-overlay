//! **Rouage de chargement** du design system Wakfu — l'animation que le client affiche pendant un
//! chargement : un rouage à seize dents qui tourne, avec au centre la silhouette d'une tête de
//! personnage qui se balance.
//!
//! ```ignore
//! use overlay_ui::design::{self, LoaderSize};
//!
//! ui.add(design::loader());                                   // 124 px, taille native (1×)
//! ui.add(design::loader().size(LoaderSize::Small));           // 48 px, le plancher
//! ui.add(design::loader().size(LoaderSize::Px(80.0)).tooltip("Synchronisation…"));
//! ```
//!
//! Le composant est **toujours carré** et **toujours animé** : il n'a rien à décider, l'appelant
//! choisit seulement une taille et, s'il le veut, une infobulle. Il ne porte aucun état — repos,
//! survol et désactivé n'ont pas de sens pour un indicateur qui ne se clique pas.
//!
//! ## Origine
//!
//! Enregistrement du client Wakfu du 2026-09-11 (`Enregistrement 2026-09-11 142031.mp4`,
//! 158 × 156 px, 30 i/s), sept écrans de chargement. L'animation en a été isolée image par image :
//! fond retiré par estimation du fond sur toute la plage, blanc conservé avec un alpha 8 bits.
//! Résultat dans `assets/design-system/` : `loader-sheet.png` (la planche que peint ce composant),
//! `loader.apng` et `loader.gif` (la même boucle, pour tout ce qui n'est pas egui).
//!
//! ## Ce qui a été mesuré
//!
//! | Grandeur | Valeur | Détail |
//! | --- | --- | --- |
//! | Image | **124 × 124 px** | 118 px d'encre (rayon extérieur 58,5) + 3 px de marge transparente de chaque côté |
//! | Dents | 16 | comptées sur le profil angulaire au rayon des dents |
//! | Pas de rotation | 2,8° par image | une dent (22,5°) toutes les 8 images, un tour en 128 images (5,3 s) |
//! | Boucle | **16 images** | la tête a une période double de celle des dents : les images 1–8 et 9–16 ont les mêmes angles de rouage, seule la tête diffère |
//! | Cadence | **24 i/s** | 4 images nouvelles sur 5 enregistrées à 30 i/s |
//! | Couleur | blanc pur | le sprite n'a pas de couleur propre, seulement de l'alpha |
//!
//! ## Ce qui est choisi, pas mesuré
//!
//! Le jeu ne montre ce rouage qu'à une taille. Le plancher de 48 px est une **décision
//! utilisateur** (2026-09-11) ; les paliers intermédiaires (`Medium`, `Large`) sont **choisis** pour
//! couvrir l'intervalle en marches à peu près égales — voir [`LoaderSize`]. Au-dessus de 1× le
//! bitmap serait flouté par l'agrandissement, en dessous de 48 px les seize dents se confondent
//! (pas de dent ≈ 9 px de circonférence à 48, 6 px à 32) : une taille hors de l'intervalle est
//! ramenée à la borne, et le journal le dit une fois.

use std::time::Duration;

use egui::{pos2, Color32, Rect, Response, Sense, Ui, Vec2, Widget};

use crate::design::{assets::DsTexture, tokens, DesignSystem};

/// Taille du rouage — toujours un carré.
///
/// `Native` est la seule **mesure** (la taille de la capture). `Small` est le **plancher demandé
/// par l'utilisateur**, `Medium` et `Large` des paliers **choisis** pour couvrir l'intervalle par
/// marches d'environ 25 px. `Px` accepte n'importe quelle valeur de l'intervalle
/// `[LOADER_MIN_SIZE, LOADER_NATIVE_SIZE]` et ramène le reste à la borne.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoaderSize {
    /// 48 px — le plancher. Pour une ligne de texte, un pied de page, un bouton en attente.
    Small,
    /// 72 px — pour un bloc ou une section de panneau en cours de chargement.
    Medium,
    /// 96 px — pour un panneau entier vide en attente de données.
    Large,
    /// 124 px — la taille du jeu (1×), pour un écran de chargement.
    Native,
    /// Taille libre, ramenée dans `[48, 124]`.
    Px(f32),
}

impl LoaderSize {
    /// Côté demandé, en pixels, **avant** rappel dans l'intervalle.
    pub fn requested(self) -> f32 {
        match self {
            LoaderSize::Small => tokens::LOADER_SIZE_SMALL,
            LoaderSize::Medium => tokens::LOADER_SIZE_MEDIUM,
            LoaderSize::Large => tokens::LOADER_SIZE_LARGE,
            LoaderSize::Native => tokens::LOADER_NATIVE_SIZE,
            LoaderSize::Px(px) => px,
        }
    }

    /// Côté effectivement peint, en pixels : la valeur demandée ramenée dans l'intervalle
    /// `[LOADER_MIN_SIZE, LOADER_NATIVE_SIZE]`. Utile à un panneau qui doit réserver la place
    /// avant d'ajouter le composant.
    pub fn px(self) -> f32 {
        self.requested()
            .clamp(tokens::LOADER_MIN_SIZE, tokens::LOADER_NATIVE_SIZE)
    }
}

/// Construit un rouage de chargement à sa taille native. Point d'entrée unique — voir la doc de
/// module.
pub fn loader() -> Loader {
    Loader::new()
}

pub struct Loader {
    size: LoaderSize,
    tooltip: Option<String>,
    log_name: Option<String>,
    forced_frame: Option<usize>,
}

impl Default for Loader {
    fn default() -> Self {
        Self::new()
    }
}

impl Loader {
    pub fn new() -> Self {
        Self {
            size: LoaderSize::Native,
            tooltip: None,
            log_name: None,
            forced_frame: None,
        }
    }

    pub fn size(mut self, size: LoaderSize) -> Self {
        self.size = size;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Nom d'instance pour la journalisation (défaut : `loader`). À renseigner dès que deux
    /// rouages coexistent, sinon les lignes du journal sont indiscernables.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Fige l'animation sur une image de la boucle (`0..16`, modulo) **sans passer par l'horloge**
    /// — réservé à la galerie de contrôle et aux captures, où deux rendus doivent produire le même
    /// pixel. L'animation ne demande alors aucun rafraîchissement.
    pub fn preview_frame(mut self, frame: usize) -> Self {
        self.forced_frame = Some(frame % tokens::LOADER_FRAMES);
        self
    }

    /// Image de la boucle à l'instant `time` (secondes, horloge egui).
    fn frame_at(time: f64) -> usize {
        // `time` est positif et borné par la durée de la session : la troncation est sans risque
        // et le modulo ramène dans la boucle.
        ((time * tokens::LOADER_FPS).floor() as usize) % tokens::LOADER_FRAMES
    }

    /// Délai jusqu'au prochain changement d'image — c'est ce que le composant demande à egui plutôt
    /// qu'un rafraîchissement continu : 24 réveils par seconde, pas un par frame d'écran.
    fn until_next_frame(time: f64) -> Duration {
        let period = 1.0 / tokens::LOADER_FPS;
        let next = ((time / period).floor() + 1.0) * period;
        // Jamais zéro : un délai nul ferait tourner egui en boucle sur le même instant.
        Duration::from_secs_f64((next - time).max(period / 8.0))
    }

    /// Région de la planche qui porte l'image `frame` : une grille de `LOADER_SHEET_COLUMNS`
    /// colonnes, lue ligne par ligne.
    fn frame_uv(frame: usize) -> Rect {
        let cols = tokens::LOADER_SHEET_COLUMNS;
        let rows = tokens::LOADER_FRAMES.div_ceil(cols);
        let (col, row) = (frame % cols, frame / cols);
        let (cw, rh) = (1.0 / cols as f32, 1.0 / rows as f32);
        Rect::from_min_max(
            pos2(col as f32 * cw, row as f32 * rh),
            pos2((col + 1) as f32 * cw, (row + 1) as f32 * rh),
        )
    }
}

impl Widget for Loader {
    fn ui(self, ui: &mut Ui) -> Response {
        let requested = self.size.requested();
        let side = self.size.px();
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());

        if side != requested {
            // Un défaut de mise en page est un événement unique, pas un flux à 24 Hz.
            let warned_id = response.id.with("ds-loader-clamped");
            let already = ui.ctx().data_mut(|d| {
                let seen = d.get_temp::<bool>(warned_id).unwrap_or(false);
                d.insert_temp(warned_id, true);
                seen
            });
            if !already {
                tracing::warn!(
                    component = "loader",
                    name = self.log_name.as_deref().unwrap_or("loader"),
                    demande = requested,
                    peint = side,
                    "taille hors de l'intervalle [48, 124], ramenée à la borne"
                );
            }
        }

        let frame = match self.forced_frame {
            Some(frame) => frame,
            None => {
                let time = ui.input(|i| i.time);
                ui.ctx().request_repaint_after(Self::until_next_frame(time));
                Self::frame_at(time)
            }
        };

        if ui.is_rect_visible(rect) {
            DesignSystem::get(ui.ctx()).paint_region(
                ui.painter(),
                rect,
                DsTexture::LoaderSheet,
                Self::frame_uv(frame),
                Color32::WHITE,
            );
        }

        match self.tooltip {
            Some(tooltip) => response.on_hover_text(tooltip),
            None => response,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_boucle_avance_a_24_images_par_seconde() {
        assert_eq!(Loader::frame_at(0.0), 0);
        assert_eq!(Loader::frame_at(1.0 / 24.0), 1);
        assert_eq!(Loader::frame_at(15.0 / 24.0), 15);
        // Une boucle complète dure 16/24 s : on retombe sur la première image.
        assert_eq!(Loader::frame_at(16.0 / 24.0 + 1e-9), 0);
    }

    #[test]
    fn le_reveil_vise_le_prochain_changement_d_image() {
        let period = 1.0 / 24.0;
        let d = Loader::until_next_frame(0.0).as_secs_f64();
        assert!((d - period).abs() < 1e-9, "{d}");
        // À mi-chemin d'une image, il reste une demi-période.
        let d = Loader::until_next_frame(period * 0.5).as_secs_f64();
        assert!((d - period * 0.5).abs() < 1e-9, "{d}");
        assert!(Loader::until_next_frame(period * 0.999_999_9).as_secs_f64() > 0.0);
    }

    #[test]
    fn la_planche_se_lit_ligne_par_ligne() {
        let uv = Loader::frame_uv(0);
        assert_eq!(
            (uv.min.x, uv.min.y, uv.max.x, uv.max.y),
            (0.0, 0.0, 0.25, 0.25)
        );
        let uv = Loader::frame_uv(5);
        assert_eq!(
            (uv.min.x, uv.min.y, uv.max.x, uv.max.y),
            (0.25, 0.25, 0.5, 0.5)
        );
        let uv = Loader::frame_uv(15);
        assert_eq!(
            (uv.min.x, uv.min.y, uv.max.x, uv.max.y),
            (0.75, 0.75, 1.0, 1.0)
        );
    }

    #[test]
    fn la_taille_est_ramenee_dans_l_intervalle() {
        assert_eq!(LoaderSize::Px(10.0).px(), tokens::LOADER_MIN_SIZE);
        assert_eq!(LoaderSize::Px(400.0).px(), tokens::LOADER_NATIVE_SIZE);
        assert_eq!(LoaderSize::Px(80.0).px(), 80.0);
        assert_eq!(LoaderSize::Native.px(), 124.0);
        assert_eq!(LoaderSize::Small.px(), 48.0);
    }
}
