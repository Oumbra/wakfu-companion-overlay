//! Cadre décoratif "totem" du panneau Combat, un par nombre d'alliés (`assets/templates/
//! template_1.png` à `template_6.png`, copie de `assets/_test/templates-resized/` à la racine du
//! dépôt — voir `assets/_test/README.md` pour la méthode de validation d'origine) : une colonne
//! verticale de N médaillons circulaires, chacun destiné à recevoir le portrait de classe d'un
//! allié (`crate::portraits::PortraitAtlas`), reliés par un petit connecteur latéral qui accroche
//! visuellement la barre de dégâts de la même ligne (voir `super::combat::damage_bar`, peinte
//! juste à droite de chaque médaillon par `show` ci-dessous).
//!
//! **Géométrie des médaillons** : mesurée une fois pour toutes par analyse de pixels des 6 PNG
//! (zones alpha=0 internes, non touchées par le bord du canevas — script Python, pas reproduit
//! ici) plutôt que déduite d'une formule : les écarts verticaux entre médaillons ne sont pas
//! parfaitement constants d'un template à l'autre (arrondis de redimensionnement indépendants,
//! voir `assets/_test/README.md`), donc une formule aurait introduit une erreur de 1-2 px que la
//! mesure directe évite. Chaque template garde une largeur de canevas identique (79 px) — seule la
//! hauteur varie avec N — donc un seul `FRAME_WIDTH` sert aux 6.
//!
//! **Ordre des slots = ordre d'affichage** (trié par dégâts décroissant, voir `panels::combat::
//! show`) : le médaillon du haut est toujours le plus gros dégât du camp affiché.
//!
//! **Alliés au-delà de 6** (aucun template ne va plus loin — cas rare, Wakfu ne compose
//! normalement pas d'équipe à 7+ joueurs) : `MAX_FRAME_SLOTS` borne ce module, l'appelant
//! (`panels::combat::show`) est responsable de continuer les lignes excédentaires dans le style
//! "plat" (portrait + barre, sans cadre) plutôt que de faire échouer l'affichage.

use overlay_engine::FighterDamage;

use crate::portraits::{PortraitAtlas, NATIVE_PORTRAIT_SIZE};
use crate::ui_icons::UiIcons;

use super::combat::damage_bar;

/// Nombre de médaillons du plus grand template disponible — voir la doc de module.
pub const MAX_FRAME_SLOTS: usize = 6;

/// Largeur de canevas commune aux 6 templates (mesurée, voir doc de module) — c'est aussi
/// l'abscisse à laquelle démarre la barre de dégâts de chaque ligne.
const FRAME_WIDTH: f32 = 79.0;

struct TemplateInfo {
    bytes: &'static [u8],
    height: f32,
    /// Centre de chaque médaillon, coordonnées locales au canevas (origine = coin haut-gauche du
    /// template, avant tout décalage à l'écran) — voir doc de module pour la méthode de mesure.
    slot_centers: &'static [egui::Pos2],
}

macro_rules! template_asset {
    ($n:literal) => {
        include_bytes!(concat!("../../assets/templates/template_", $n, ".png")) as &[u8]
    };
}

const TEMPLATES: [TemplateInfo; MAX_FRAME_SLOTS] = [
    TemplateInfo {
        bytes: template_asset!(1),
        height: 141.0,
        slot_centers: &[egui::Pos2 { x: 31.0, y: 69.0 }],
    },
    TemplateInfo {
        bytes: template_asset!(2),
        height: 203.0,
        slot_centers: &[
            egui::Pos2 { x: 31.0, y: 69.0 },
            egui::Pos2 { x: 31.0, y: 131.0 },
        ],
    },
    TemplateInfo {
        bytes: template_asset!(3),
        height: 265.0,
        slot_centers: &[
            egui::Pos2 { x: 31.0, y: 69.0 },
            egui::Pos2 { x: 31.0, y: 131.0 },
            egui::Pos2 { x: 31.0, y: 192.5 },
        ],
    },
    TemplateInfo {
        bytes: template_asset!(4),
        height: 336.0,
        slot_centers: &[
            egui::Pos2 { x: 31.0, y: 70.0 },
            egui::Pos2 { x: 31.0, y: 130.5 },
            egui::Pos2 { x: 31.0, y: 192.5 },
            egui::Pos2 { x: 31.0, y: 255.0 },
        ],
    },
    TemplateInfo {
        bytes: template_asset!(5),
        height: 388.0,
        slot_centers: &[
            egui::Pos2 { x: 31.0, y: 69.0 },
            egui::Pos2 { x: 31.0, y: 131.0 },
            egui::Pos2 { x: 31.0, y: 192.0 },
            egui::Pos2 { x: 31.0, y: 254.0 },
            egui::Pos2 { x: 31.0, y: 316.0 },
        ],
    },
    TemplateInfo {
        bytes: template_asset!(6),
        height: 450.0,
        slot_centers: &[
            egui::Pos2 { x: 31.0, y: 69.0 },
            egui::Pos2 { x: 31.0, y: 131.0 },
            egui::Pos2 { x: 31.0, y: 192.0 },
            egui::Pos2 { x: 31.0, y: 254.0 },
            egui::Pos2 { x: 31.0, y: 316.0 },
            egui::Pos2 { x: 31.0, y: 378.0 },
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
    /// débordement du portrait 48×48 collé sans redimensionnement dans un trou mesuré à 45-48 px.
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

        for (fighter, &center) in fighters.iter().zip(template.slot_centers) {
            let pos = frame_rect.min + center.to_vec2();
            let portrait_rect = egui::Rect::from_center_size(
                pos,
                egui::vec2(NATIVE_PORTRAIT_SIZE, NATIVE_PORTRAIT_SIZE),
            );
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

        for (fighter, &center) in fighters.iter().zip(template.slot_centers) {
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(
                    frame_rect.max.x,
                    frame_rect.min.y + center.y - row_height / 2.0,
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
