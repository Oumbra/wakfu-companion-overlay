//! Cadre décoratif "totem" du panneau Combat, un par nombre d'alliés (`assets/templates/
//! template_1.png` à `template_6.png` à la racine du dépôt, copiés tels quels sous
//! `crates/overlay-ui/assets/templates/` — c'est cette copie locale au crate qu'`include_bytes!`
//! embarque, la source à la racine n'est qu'un lieu d'édition) : une colonne verticale de N
//! médaillons circulaires, chacun destiné à recevoir le portrait de classe d'un allié
//! (`crate::portraits::PortraitAtlas`).
//!
//! **Géométrie des médaillons** — refonte 2026-09-04 (nouveaux templates fournis par
//! l'utilisateur) : centre de chaque médaillon mesuré par analyse de pixels des 6 PNG (bbox de
//! chaque zone alpha=0 interne, non touchée par le bord du canevas — script Python, pas reproduit
//! ici) plutôt que déduit d'une formule, pour la même raison que la refonte précédente : les
//! écarts entre médaillons ne sont pas parfaitement constants d'un template à l'autre.
//!
//! **Le portrait n'est PAS mis à l'échelle du trou mesuré** (première version de cette refonte,
//! corrigée après retour utilisateur avec capture d'écran + composite de référence "cible.png" à
//! l'appui) : il est collé à sa taille NATIVE `NATIVE_PORTRAIT_SIZE` (48×48, non re-échantillonné,
//! voir `crate::portraits`), centré sur le trou mesuré — plus GRAND que le trou (~41 px), il
//! déborde donc volontairement dans l'anneau, que le template repeint par-dessus masque
//! proprement (voir l'ordre de peinture dans `show`). C'est exactement la méthode déjà validée
//! pour l'ancienne géométrie (voir `assets/_test/README.md`), reconduite telle quelle ici — la
//! seule chose qui change d'un jeu de templates à l'autre, ce sont les centres mesurés.
//! Corollaire : le canevas n'est PAS étiré à l'affichage, `FRAME_WIDTH` vaut la largeur native du
//! PNG (68 px, commune aux 6) — un canevas étiré aurait fallu re-proportionner le portrait en
//! conséquence pour garder le même ratio taille-portrait/taille-trou que celui mesuré sur
//! `cible.png`, inutilement compliqué face à un simple 1:1.
//!
//! **Ordre des slots — refonte 2026-09-04** (retour utilisateur, redesign des barres) : ce N'EST
//! PLUS l'ordre d'affichage trié par dégâts décroissant. `fighters` est désormais dans l'ordre
//! STABLE défini par `overlay_engine::FightSnapshot::fighters` (ordre d'arrivée en combat, voir sa
//! doc) — les portraits ne doivent plus changer de position à mesure que les dégâts évoluent,
//! seules les barres de dégâts (peintes ailleurs, voir `panels::combat::show`) restent triées et
//! filtrées (uniquement dégâts > 0). Ce module ne peint donc plus aucune barre : seuls le cadre,
//! les portraits, leur infobulle au survol et leur pourcentage de dégâts (coin bas-droit, voir
//! `panels::combat::paint_portrait_percent`) — le découplage complet portraits/barres est décidé
//! et assemblé par l'appelant.
//!
//! **Alliés au-delà de 6** (aucun template ne va plus loin — cas rare, Wakfu ne compose
//! normalement pas d'équipe à 7+ joueurs) : `MAX_FRAME_SLOTS` borne ce module, l'appelant
//! (`panels::combat::show`) est responsable de continuer les lignes excédentaires dans le style
//! "plat" (portrait + barre, sans cadre) plutôt que de faire échouer l'affichage.

use overlay_engine::FighterDamage;

use crate::portraits::{PortraitAtlas, NATIVE_PORTRAIT_SIZE};
use crate::ui_icons::UiIcons;

/// Nombre de médaillons du plus grand template disponible — voir la doc de module.
pub const MAX_FRAME_SLOTS: usize = 6;

/// Largeur de canevas commune aux 6 templates — largeur NATIVE du PNG (68 px, commune aux 6),
/// canevas non étiré à l'affichage (voir doc de module).
const FRAME_WIDTH: f32 = 68.0;

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
        height: 121.0,
        slot_centers: &[egui::pos2(26.0, 59.0)],
    },
    TemplateInfo {
        bytes: template_asset!(2),
        height: 174.0,
        slot_centers: &[egui::pos2(26.0, 60.0), egui::pos2(26.0, 112.0)],
    },
    TemplateInfo {
        bytes: template_asset!(3),
        height: 227.0,
        slot_centers: &[
            egui::pos2(26.0, 60.0),
            egui::pos2(26.0, 111.0),
            egui::pos2(26.0, 165.0),
        ],
    },
    TemplateInfo {
        bytes: template_asset!(4),
        height: 288.0,
        slot_centers: &[
            egui::pos2(28.0, 60.0),
            egui::pos2(27.0, 111.5),
            egui::pos2(27.0, 165.0),
            egui::pos2(26.0, 218.5),
        ],
    },
    TemplateInfo {
        bytes: template_asset!(5),
        height: 333.0,
        slot_centers: &[
            egui::pos2(26.0, 60.0),
            egui::pos2(26.0, 111.5),
            egui::pos2(26.0, 165.0),
            egui::pos2(26.0, 218.0),
            egui::pos2(26.0, 271.0),
        ],
    },
    TemplateInfo {
        bytes: template_asset!(6),
        height: 386.0,
        slot_centers: &[
            egui::pos2(26.0, 60.0),
            egui::pos2(26.0, 111.5),
            egui::pos2(26.0, 165.0),
            egui::pos2(26.0, 218.0),
            egui::pos2(26.0, 271.0),
            egui::pos2(26.0, 324.0),
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

    /// Dessine le cadre + les portraits pour `fighters` (longueur 1..=`MAX_FRAME_SLOTS` — panique
    /// en debug sinon, voir `debug_assert!`), dans l'ORDRE STABLE fourni par l'appelant (voir doc
    /// de module — plus trié par dégâts). `total_damage` sert uniquement à calculer le pourcentage
    /// peint sur chaque portrait (voir `panels::combat::paint_portrait_percent`) ; aucune barre
    /// n'est peinte ici (voir doc de module).
    ///
    /// Ordre de peinture (voir `assets/_test/README.md`, méthode déjà validée) : portrait D'ABORD,
    /// template ENSUITE par-dessus — l'anneau opaque du médaillon masque proprement le
    /// débordement du portrait 48×48 collé sans redimensionnement dans un trou mesuré à ~41 px
    /// (voir doc de module). Infobulle + pourcentage sont peints en DERNIER, par-dessus le cadre :
    /// une zone interactive ou un texte masqués par le cadre ne serviraient à rien.
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        portraits: &PortraitAtlas,
        icons: &UiIcons,
        fighters: &[&FighterDamage],
        total_damage: i64,
    ) {
        debug_assert!(!fighters.is_empty() && fighters.len() <= MAX_FRAME_SLOTS);
        let n = fighters.len().clamp(1, MAX_FRAME_SLOTS);
        let template = &TEMPLATES[n - 1];
        let texture = &self.textures[n - 1];

        // `FRAME_WIDTH` fixe, PAS `ui.available_width()` (contrairement à l'ancienne version) :
        // ce cadre partage désormais sa ligne avec une colonne de barres indépendante peinte par
        // l'appelant (voir doc de module) — s'étirer sur toute la largeur disponible ne laisserait
        // plus de place à cette colonne.
        let frame_rect = ui
            .allocate_exact_size(
                egui::vec2(FRAME_WIDTH, template.height),
                egui::Sense::hover(),
            )
            .0;

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

        // Infobulle (nom, demande utilisateur : les portraits ne sont plus alignés avec "leur"
        // barre depuis le découplage tri portraits/barres — sans elle, un portrait devient
        // impossible à identifier dès que sa barre n'est plus juste à côté) + pourcentage de
        // dégâts sur le portrait (bas-droite, voir `panels::combat::paint_portrait_percent`),
        // peints APRÈS le cadre — voir doc de fonction.
        for (fighter, &center) in fighters.iter().zip(template.slot_centers) {
            let pos = frame_rect.min + center.to_vec2();
            let portrait_rect = egui::Rect::from_center_size(
                pos,
                egui::vec2(NATIVE_PORTRAIT_SIZE, NATIVE_PORTRAIT_SIZE),
            );
            let id = ui.id().with(("combat-frame-slot", fighter.name.as_str()));
            let response = ui.interact(portrait_rect, id, egui::Sense::hover());
            super::combat::show_tooltip_above(&response, fighter.name.as_str());
            if fighter.total_damage > 0 {
                super::combat::paint_portrait_percent(
                    ui,
                    portrait_rect,
                    fighter.total_damage,
                    total_damage,
                );
            }
        }
    }
}
