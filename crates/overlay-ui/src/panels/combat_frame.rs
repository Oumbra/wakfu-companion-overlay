//! Cadre décoratif "totem" du panneau Combat, un par nombre d'alliés (`assets/templates/
//! template_1.png` à `template_6.png` à la racine du dépôt, copiés tels quels sous
//! `crates/overlay-ui/assets/templates/` — c'est cette copie locale au crate qu'`include_bytes!`
//! embarque, la source à la racine n'est qu'un lieu d'édition) : une colonne verticale de N
//! médaillons circulaires, chacun destiné à recevoir le portrait de classe d'un allié
//! (`crate::portraits::PortraitAtlas`), reliés par un petit connecteur latéral qui accroche
//! visuellement la barre de dégâts de la même ligne (voir `super::combat::damage_bar`, peinte
//! juste à droite de chaque médaillon par `show` ci-dessous).
//!
//! **Géométrie des médaillons** — refonte 2026-09-04 (nouveaux templates fournis par
//! l'utilisateur, dessinés pour que chaque rond de slot soit strictement remplaçable par le
//! portrait 48×48 déjà en place) : mesurée une fois pour toutes par analyse de pixels des 6 PNG
//! (bbox de chaque zone alpha=0 interne, non touchée par le bord du canevas — script Python, pas
//! reproduit ici, méthode identique à celle documentée dans `assets/_test/README.md`) plutôt que
//! déduite d'une formule, pour la même raison que la refonte précédente : les écarts entre
//! médaillons ne sont pas parfaitement constants d'un template à l'autre. Chaque `SlotRect` est le
//! coin haut-gauche + la taille de cette bbox, mise à l'échelle du canevas natif (68 px, commun
//! aux 6 PNG) vers `FRAME_WIDTH` — **le portrait est collé À CE COIN, à CETTE taille, jamais
//! centré sur un point avec une taille fixe supposée** (c'était le bug de l'ancienne géométrie :
//! un centre + `NATIVE_PORTRAIT_SIZE` fixe suppose un rond parfaitement régulier, faux d'un
//! médaillon à l'autre). Chaque template garde une largeur de canevas identique (68 px natifs) —
//! seule la hauteur varie avec N — donc un seul `FRAME_WIDTH` sert aux 6.
//!
//! **Ordre des slots = ordre d'affichage** (trié par dégâts décroissant, voir `panels::combat::
//! show`) : le médaillon du haut est toujours le plus gros dégât du camp affiché.
//!
//! **Alliés au-delà de 6** (aucun template ne va plus loin — cas rare, Wakfu ne compose
//! normalement pas d'équipe à 7+ joueurs) : `MAX_FRAME_SLOTS` borne ce module, l'appelant
//! (`panels::combat::show`) est responsable de continuer les lignes excédentaires dans le style
//! "plat" (portrait + barre, sans cadre) plutôt que de faire échouer l'affichage.

use overlay_engine::FighterDamage;

use crate::portraits::PortraitAtlas;
use crate::ui_icons::UiIcons;

use super::combat::damage_bar;

/// Nombre de médaillons du plus grand template disponible — voir la doc de module.
pub const MAX_FRAME_SLOTS: usize = 6;

/// Largeur de canevas commune aux 6 templates, à laquelle chaque PNG natif (68 px de large,
/// commun aux 6 — voir doc de module) est mis à l'échelle pour l'affichage. C'est aussi l'abscisse
/// à laquelle démarre la barre de dégâts de chaque ligne. Valeur inchangée depuis l'ancienne
/// géométrie (voir `assets/_test/README.md`) : un rond de slot natif de 41 px mis à l'échelle par
/// `FRAME_WIDTH / 68` donne ~47.6 px, la taille attendue pour un portrait 48×48 collé sans
/// redimensionnement perceptible.
const FRAME_WIDTH: f32 = 79.0;

/// Rectangle d'un médaillon en coordonnées locales au canevas (origine = coin haut-gauche du
/// template, avant tout décalage à l'écran) : le portrait s'y colle par son coin haut-gauche, à
/// SA taille — voir doc de module pour pourquoi ce n'est plus un centre + une taille fixe.
struct SlotRect {
    min: egui::Pos2,
    size: egui::Vec2,
}

struct TemplateInfo {
    bytes: &'static [u8],
    height: f32,
    slots: &'static [SlotRect],
}

macro_rules! template_asset {
    ($n:literal) => {
        include_bytes!(concat!("../../assets/templates/template_", $n, ".png")) as &[u8]
    };
}

const TEMPLATES: [TemplateInfo; MAX_FRAME_SLOTS] = [
    TemplateInfo {
        bytes: template_asset!(1),
        height: 140.6,
        slots: &[SlotRect {
            min: egui::pos2(6.97, 45.31),
            size: egui::vec2(47.63, 47.63),
        }],
    },
    TemplateInfo {
        bytes: template_asset!(2),
        height: 202.1,
        slots: &[
            SlotRect {
                min: egui::pos2(6.97, 46.47),
                size: egui::vec2(47.63, 47.63),
            },
            SlotRect {
                min: egui::pos2(6.97, 106.88),
                size: egui::vec2(47.63, 47.63),
            },
        ],
    },
    TemplateInfo {
        bytes: template_asset!(3),
        height: 263.7,
        slots: &[
            SlotRect {
                min: egui::pos2(6.97, 46.47),
                size: egui::vec2(47.63, 47.63),
            },
            SlotRect {
                min: egui::pos2(6.97, 105.72),
                size: egui::vec2(47.63, 47.63),
            },
            SlotRect {
                min: egui::pos2(6.97, 168.46),
                size: egui::vec2(47.63, 47.63),
            },
        ],
    },
    TemplateInfo {
        bytes: template_asset!(4),
        height: 334.6,
        slots: &[
            SlotRect {
                min: egui::pos2(9.29, 46.47),
                size: egui::vec2(47.63, 47.63),
            },
            SlotRect {
                min: egui::pos2(8.13, 105.72),
                size: egui::vec2(47.63, 48.79),
            },
            SlotRect {
                min: egui::pos2(8.13, 168.46),
                size: egui::vec2(47.63, 47.63),
            },
            SlotRect {
                min: egui::pos2(6.97, 230.03),
                size: egui::vec2(47.63, 48.79),
            },
        ],
    },
    TemplateInfo {
        bytes: template_asset!(5),
        height: 386.9,
        slots: &[
            SlotRect {
                min: egui::pos2(6.97, 46.47),
                size: egui::vec2(47.63, 47.63),
            },
            SlotRect {
                min: egui::pos2(6.97, 105.72),
                size: egui::vec2(47.63, 48.79),
            },
            SlotRect {
                min: egui::pos2(6.97, 168.46),
                size: egui::vec2(47.63, 47.63),
            },
            SlotRect {
                min: egui::pos2(6.97, 230.03),
                size: egui::vec2(47.63, 47.63),
            },
            SlotRect {
                min: egui::pos2(6.97, 291.60),
                size: egui::vec2(47.63, 47.63),
            },
        ],
    },
    TemplateInfo {
        bytes: template_asset!(6),
        height: 448.4,
        slots: &[
            SlotRect {
                min: egui::pos2(6.97, 46.47),
                size: egui::vec2(47.63, 47.63),
            },
            SlotRect {
                min: egui::pos2(6.97, 105.72),
                size: egui::vec2(47.63, 48.79),
            },
            SlotRect {
                min: egui::pos2(6.97, 168.46),
                size: egui::vec2(47.63, 47.63),
            },
            SlotRect {
                min: egui::pos2(6.97, 230.03),
                size: egui::vec2(47.63, 47.63),
            },
            SlotRect {
                min: egui::pos2(6.97, 291.60),
                size: egui::vec2(47.63, 47.63),
            },
            SlotRect {
                min: egui::pos2(6.97, 353.18),
                size: egui::vec2(47.63, 47.63),
            },
        ],
    },
];

pub struct CombatFrame {
    textures: [egui::TextureHandle; MAX_FRAME_SLOTS],
}

impl CombatFrame {
    /// Décode et charge les 6 templates embarqués — appelé une seule fois (voir
    /// `crate::portraits::PortraitAtlas::load`, même logique).
    pub fn load(ctx: &egui::Context) -> Self {
        let textures = std::array::from_fn(|i| {
            let decoded = image::load_from_memory(TEMPLATES[i].bytes)
                .expect("template de cadre combat embarqué invalide — asset corrompu au build")
                .to_rgba8();
            let (width, height) = decoded.dimensions();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [width as usize, height as usize],
                decoded.as_raw(),
            );
            ctx.load_texture(
                format!("combat-frame-{}", i + 1),
                color_image,
                egui::TextureOptions::LINEAR,
            )
        });
        Self { textures }
    }

    /// Dessine le cadre + les portraits + une barre de dégâts par ligne pour `fighters` (déjà
    /// triés par dégâts décroissant par l'appelant, longueur 1..=`MAX_FRAME_SLOTS` — panique en
    /// debug sinon, voir `debug_assert!`). `max_damage`/`total_damage` calculés par l'appelant sur
    /// TOUT le camp affiché, pas seulement `fighters` — voir `panels::combat::show` : un allié
    /// excédentaire relégué dans la liste "plate" (au-delà de `MAX_FRAME_SLOTS`) doit rester
    /// cohérent avec ceux affichés ici, d'où le partage de ces deux dénominateurs plutôt qu'un
    /// recalcul local. Voir `panels::combat::damage_bar` pour ce qu'ils pilotent chacun.
    ///
    /// Ordre de peinture (voir `assets/_test/README.md`, méthode déjà validée) : portrait D'ABORD,
    /// template ENSUITE par-dessus — l'anneau opaque du médaillon masque proprement tout
    /// débordement d'anti-aliasing du portrait collé exactement au coin haut-gauche + à la taille
    /// de son `SlotRect` (voir doc de module).
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        portraits: &PortraitAtlas,
        icons: &UiIcons,
        fighters: &[&FighterDamage],
        max_damage: i64,
        total_damage: i64,
    ) {
        debug_assert!(!fighters.is_empty() && fighters.len() <= MAX_FRAME_SLOTS);
        let n = fighters.len().clamp(1, MAX_FRAME_SLOTS);
        let template = &TEMPLATES[n - 1];
        let texture = &self.textures[n - 1];

        let row_height = BAR_ROW_HEIGHT.max(1.0);
        let full_rect = ui
            .allocate_exact_size(
                egui::vec2(ui.available_width(), template.height),
                egui::Sense::hover(),
            )
            .0;
        let frame_rect =
            egui::Rect::from_min_size(full_rect.min, egui::vec2(FRAME_WIDTH, template.height));

        for (fighter, slot) in fighters.iter().zip(template.slots) {
            let portrait_rect =
                egui::Rect::from_min_size(frame_rect.min + slot.min.to_vec2(), slot.size);
            let texture = fighter.class_name.as_deref().and_then(|class_name| {
                portraits.texture(class_name, fighter.gender, fighter.is_ko)
            });
            match texture {
                Some(texture) => {
                    egui::Image::new(texture).paint_at(ui, portrait_rect);
                }
                None => {
                    // Allié pas encore classifié (roster absent, `breed` inconnu de ce combat) :
                    // repli générique plutôt qu'un trou vide dans le médaillon — voir doc de
                    // module. Grisé par un tint approximatif si KO (voir `panels::combat::
                    // grey_tint_if_ko` : pas de version grisée précalculée pour cet asset unique,
                    // contrairement aux portraits de classe, voir `portraits.rs`).
                    let image = egui::Image::new(icons.unknown_entity_texture())
                        .tint(super::combat::grey_tint_if_ko(fighter.is_ko));
                    image.paint_at(ui, portrait_rect);
                }
            }
        }

        // Le template est peint APRÈS tous les portraits (voir doc de module) : un seul appel,
        // l'anneau de chaque médaillon masque le débordement de chacun d'eux d'un coup.
        egui::Image::new(texture).paint_at(ui, frame_rect);

        for (fighter, slot) in fighters.iter().zip(template.slots) {
            let slot_center_y = slot.min.y + slot.size.y / 2.0;
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(
                    frame_rect.max.x,
                    frame_rect.min.y + slot_center_y - row_height / 2.0,
                ),
                egui::vec2(full_rect.max.x - frame_rect.max.x, row_height),
            );
            damage_bar(
                ui,
                bar_rect,
                &fighter.name,
                fighter.total_damage,
                max_damage,
                total_damage,
            );
        }
    }
}

/// Hauteur d'une ligne de barre dans le cadre — voir `panels::combat::BAR_HEIGHT` pour la barre
/// "plate" (liste sans cadre) : légèrement plus haute ici pour rester proportionnée au diamètre
/// des médaillons (~47 px) plutôt que de reprendre la même constante que la liste plate.
const BAR_ROW_HEIGHT: f32 = 30.0;
