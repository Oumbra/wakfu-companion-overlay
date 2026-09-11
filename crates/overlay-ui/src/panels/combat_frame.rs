//! Cadre décoratif "totem" du panneau Combat, un par nombre d'alliés (`template_1.png` à
//! `template_6.png` sous `crates/overlay-ui/assets/templates/` — c'est cette copie locale au crate
//! qu'`include_bytes!` embarque) : une colonne verticale de N médaillons, chacun destiné à
//! recevoir le portrait de classe d'un allié (`crate::portraits::PortraitAtlas`).
//!
//! **Refonte 2026-09-07 (gabarits redessinés à la main par l'utilisateur)** : les 6 PNG ont changé
//! de dimensions (largeur commune 68→70 px, hauteurs toutes légèrement modifiées) et surtout
//! **n'ont plus de trou/anneau découpé dans le disque** — juste une plaque pleine, sans aucune
//! zone transparente au centre de chaque médaillon (vérifié pixel par pixel : aucune marque, ni
//! transparence ni couleur distincte, n'indique plus l'emplacement du portrait). Conséquences :
//! - Le masquage circulaire, auparavant fourni par l'anneau opaque du template repeint par-dessus
//!   le portrait, doit maintenant être fait **côté code** : chaque portrait est peint avec
//!   `egui::Image::corner_radius` réglé à la moitié de `NATIVE_PORTRAIT_SIZE` (un carré aux coins
//!   arrondis à 50 % devient un cercle parfait) — voir `PORTRAIT_CORNER_RADIUS`.
//! - L'ordre de peinture s'inverse : le template est désormais peint **avant** les portraits (plus
//!   de trou à masquer, c'est lui qui sert de fond), les portraits (déjà rognés en cercle) par-
//!   dessus. Infobulle + pourcentage restent peints en dernier (inchangé).
//! - Les centres de médaillon ne peuvent plus être mesurés depuis l'image (pas de trou à
//!   localiser) : ils ont été validés avec l'utilisateur via un artefact de calibration interactif
//!   (repère croix + carré 48×48, rendu avec portrait réel côté à côté pour chacun des 6 gabarits,
//!   plusieurs itérations avant validation) plutôt que mesurés par script. Axe horizontal retenu :
//!   x=29,5 pour les templates 1 à 5, x=27,5 pour le template 6 (dernier ajustement demandé
//!   explicitement par l'utilisateur sur ce gabarit précis) — pas une formule unique, une valeur
//!   par gabarit. Axe vertical : les N portraits d'un gabarit sont espacés de N+1 marges
//!   rigoureusement égales (avant le premier, entre chaque paire, après le dernier) dans la
//!   portion du gabarit qui atteint sa pleine largeur (hors chapiteaux/coins qui la rétrécissent).
//!
//! **Le portrait n'est toujours PAS mis à l'échelle** : collé à sa taille NATIVE
//! `NATIVE_PORTRAIT_SIZE` (48×48, non re-échantillonné, voir `crate::portraits`), seulement
//! rogné en cercle — inchangé par rapport à la géométrie précédente, qui collait déjà un portrait
//! 48×48 plus grand que le trou mesuré (~41 px) pour laisser l'anneau masquer le débordement ; ici
//! le rognage en cercle joue exactement ce rôle.
//! Corollaire : le canevas n'est PAS étiré à l'affichage, `FRAME_WIDTH` vaut la largeur native du
//! PNG (70 px, commune aux 6).
//!
//! **Ordre des slots** : `fighters` est dans l'ordre STABLE défini par
//! `overlay_engine::FightSnapshot::fighters` (ordre d'arrivée en combat, voir sa doc) — les
//! portraits ne changent pas de position à mesure que les dégâts évoluent, seules les barres de
//! dégâts (peintes ailleurs, voir `panels::combat::show`) restent triées et filtrées (uniquement
//! dégâts > 0). Ce module ne peint donc aucune barre : seuls le cadre, les portraits, leur
//! infobulle au survol et leur pourcentage de dégâts (coin bas-droit, voir
//! `panels::combat::paint_portrait_percent`) — le découplage complet portraits/barres est décidé
//! et assemblé par l'appelant.
//!
//! **Alliés au-delà de 6** (aucun template ne va plus loin — cas rare, Wakfu ne compose
//! normalement pas d'équipe à 7+ joueurs) : `MAX_FRAME_SLOTS` borne ce module, l'appelant
//! (`panels::combat::show`) est responsable de continuer les lignes excédentaires dans le style
//! "plat" (portrait + barre, sans cadre) plutôt que de faire échouer l'affichage.
//!
//! **Ennemis** (refonte 2026-09-07, vision ennemie) : ce cadre affiche désormais aussi les
//! ennemis jusqu'à `MAX_FRAME_SLOTS` (auparavant réservé aux alliés). Un ennemi n'a jamais de
//! `class_name` (`breed` non déterministe côté ennemi) — `show` reçoit donc en plus `catalog`/
//! `remote_icons`/`remote_icon_textures` pour résoudre son portrait RÉEL via le catalogue (voir
//! `panels::combat::resolve_fighter_texture`), repli générique tant qu'il n'a pas fini de
//! télécharger ou si le nom n'est pas reconnu — même mécanisme que la liste "plate" utilisait déjà
//! pour les ennemis avant cette refonte.

use overlay_engine::{CatalogIndex, FighterDamage};

use crate::portraits::{PortraitAtlas, NATIVE_PORTRAIT_SIZE};
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::ui_icons::UiIcons;

/// Nombre de médaillons du plus grand template disponible — voir la doc de module.
pub const MAX_FRAME_SLOTS: usize = 6;

/// Largeur de canevas commune aux 6 templates — largeur NATIVE du PNG (70 px, commune aux 6),
/// canevas non étiré à l'affichage (voir doc de module).
const FRAME_WIDTH: f32 = 70.0;

/// Rayon de rognage circulaire du portrait — moitié de `NATIVE_PORTRAIT_SIZE` (48/2), un carré
/// aux coins arrondis à ce rayon devient un cercle parfait (voir doc de module). Les gabarits
/// n'ont plus de trou/anneau pour faire ce travail à la place du code.
const PORTRAIT_CORNER_RADIUS: u8 = (NATIVE_PORTRAIT_SIZE / 2.0) as u8;

struct TemplateInfo {
    bytes: &'static [u8],
    height: f32,
    /// Centre de chaque médaillon, coordonnées locales au canevas (origine = coin haut-gauche du
    /// template, avant tout décalage à l'écran) — voir doc de module pour la méthode de
    /// validation (artefact interactif, plus une mesure de trou : il n'y en a plus).
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
        height: 130.0,
        slot_centers: &[egui::pos2(29.5, 64.5)],
    },
    TemplateInfo {
        bytes: template_asset!(2),
        height: 180.0,
        slot_centers: &[egui::pos2(29.5, 62.3), egui::pos2(29.5, 114.7)],
    },
    TemplateInfo {
        bytes: template_asset!(3),
        height: 232.0,
        slot_centers: &[
            egui::pos2(29.5, 64.2),
            egui::pos2(29.5, 116.5),
            egui::pos2(29.5, 168.8),
        ],
    },
    TemplateInfo {
        bytes: template_asset!(4),
        height: 288.0,
        slot_centers: &[
            egui::pos2(29.5, 63.4),
            egui::pos2(29.5, 115.8),
            egui::pos2(29.5, 168.2),
            egui::pos2(29.5, 220.6),
        ],
    },
    TemplateInfo {
        bytes: template_asset!(5),
        height: 339.0,
        slot_centers: &[
            egui::pos2(29.5, 63.5),
            egui::pos2(29.5, 116.0),
            egui::pos2(29.5, 168.5),
            egui::pos2(29.5, 221.0),
            egui::pos2(29.5, 273.5),
        ],
    },
    TemplateInfo {
        bytes: template_asset!(6),
        height: 393.0,
        slot_centers: &[
            egui::pos2(27.5, 66.9),
            egui::pos2(27.5, 118.7),
            egui::pos2(27.5, 170.6),
            egui::pos2(27.5, 222.4),
            egui::pos2(27.5, 274.3),
            egui::pos2(27.5, 326.1),
        ],
    },
];

/// Géométrie du plus grand gabarit (`template_6`, 6 emplacements) — voir
/// `panels::combat_frame_scroll`, qui le réutilise TEL QUEL comme fenêtre fixe pour les ennemis
/// au-delà de `MAX_FRAME_SLOTS` (aucun nouveau gabarit, aucune remesure : tout dérive des 6 centres
/// déjà validés ci-dessus).
pub(crate) struct LargestFrameGeometry {
    /// Taille de canevas du plus grand gabarit — inchangée, non étirée à l'affichage.
    pub frame_size: egui::Vec2,
    /// Abscisse commune aux 6 médaillons du plus grand gabarit.
    pub slot_x: f32,
    /// Ordonnée du 1ᵉʳ médaillon.
    pub first_center_y: f32,
    /// Pas moyen entre deux médaillons consécutifs du plus grand gabarit — écart total (dernier
    /// moins premier) divisé par le nombre d'intervalles, PAS une valeur remesurée séparément.
    pub pitch: f32,
    /// Rayon de rognage circulaire d'un portrait (moitié de `NATIVE_PORTRAIT_SIZE`).
    pub portrait_radius: f32,
}

/// Voir `LargestFrameGeometry`.
pub(crate) fn largest_frame_geometry() -> LargestFrameGeometry {
    let largest = &TEMPLATES[MAX_FRAME_SLOTS - 1];
    let first = largest.slot_centers[0];
    let last = largest.slot_centers[largest.slot_centers.len() - 1];
    let intervals = (largest.slot_centers.len() - 1) as f32;
    LargestFrameGeometry {
        frame_size: egui::vec2(FRAME_WIDTH, largest.height),
        slot_x: first.x,
        first_center_y: first.y,
        pitch: (last.y - first.y) / intervals,
        portrait_radius: PORTRAIT_CORNER_RADIUS as f32,
    }
}

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

    /// Texture du plus grand gabarit (`template_6`) — réutilisée TELLE QUELLE par
    /// `panels::combat_frame_scroll::EnemyFrameScroll` comme fenêtre fixe pour les ennemis
    /// au-delà de `MAX_FRAME_SLOTS`, sans recharger de texture séparée. Voir `LargestFrameGeometry`
    /// pour la géométrie assortie.
    pub(crate) fn largest_texture(&self) -> &egui::TextureHandle {
        &self.textures[MAX_FRAME_SLOTS - 1]
    }

    /// Dessine le cadre + les portraits pour `fighters` (longueur 1..=`MAX_FRAME_SLOTS` — panique
    /// en debug sinon, voir `debug_assert!`), dans l'ORDRE STABLE fourni par l'appelant (voir doc
    /// de module — plus trié par dégâts). `total_damage` sert uniquement à calculer le pourcentage
    /// peint sur chaque portrait (voir `panels::combat::paint_portrait_percent`) ; aucune barre
    /// n'est peinte ici (voir doc de module).
    ///
    /// Ordre de peinture (inversé depuis la refonte 2026-09-07, voir doc de module) : template
    /// D'ABORD (fond, plus de trou à masquer), portraits ENSUITE par-dessus — chacun rogné en
    /// cercle via `PORTRAIT_CORNER_RADIUS`. Infobulle + pourcentage sont peints en DERNIER,
    /// par-dessus les portraits : une zone interactive ou un texte masqués ne serviraient à rien.
    ///
    /// **Ennemis** (refonte 2026-09-07, vision ennemie) : `catalog`/`remote_icons`/
    /// `remote_icon_textures` permettent de résoudre le portrait RÉEL du monstre (voir
    /// `panels::combat::resolve_fighter_texture`) — un ennemi n'a jamais de `class_name`, sans ces
    /// paramètres tout ennemi affiché ici retomberait sur le portrait générique.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        portraits: &PortraitAtlas,
        icons: &UiIcons,
        catalog: &CatalogIndex,
        remote_icons: &RemoteIconStore,
        remote_icon_textures: &mut RemoteIconTextures,
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

        // Le template est peint D'ABORD (voir doc de fonction) : simple fond, plus de trou à
        // masquer côté image.
        egui::Image::new(texture).paint_at(ui, frame_rect);

        for (fighter, &center) in fighters.iter().zip(template.slot_centers) {
            let pos = frame_rect.min + center.to_vec2();
            let portrait_rect = egui::Rect::from_center_size(
                pos,
                egui::vec2(NATIVE_PORTRAIT_SIZE, NATIVE_PORTRAIT_SIZE),
            );
            let portrait = super::combat::resolve_fighter_texture(
                ui,
                portraits,
                catalog,
                remote_icons,
                remote_icon_textures,
                fighter,
            );
            // Le rognage en cercle et le grisé vivent dans `design::portrait`. Ce qui se décide
            // ICI est le choix de la texture et celui de teinter : un portrait de classe a sa
            // version grise **précalculée** dans l'atlas et n'a pas besoin d'une teinte, qui serait
            // moins bonne ; une icône de monstre téléchargée ou le repli générique (allié pas
            // encore classifié, ennemi que le catalogue ne résout pas) n'ont pas d'équivalent gris.
            let (texture, dimmed) = match &portrait {
                Some(super::combat::FighterPortrait::ClassPortrait(texture)) => {
                    (texture.id(), false)
                }
                Some(super::combat::FighterPortrait::RemoteMonster(texture)) => {
                    (texture.id(), fighter.is_ko)
                }
                None => (icons.unknown_entity_texture().id(), fighter.is_ko),
            };
            crate::design::paint_portrait(
                ui,
                portrait_rect,
                texture,
                crate::design::PortraitShape::Round,
                dimmed,
            );
        }

        // Infobulle (nom, demande utilisateur : les portraits ne sont plus alignés avec "leur"
        // barre depuis le découplage tri portraits/barres — sans elle, un portrait devient
        // impossible à identifier dès que sa barre n'est plus juste à côté) + pourcentage de
        // dégâts sur le portrait (bas-droite, voir `panels::combat::paint_portrait_percent`),
        // peints APRÈS les portraits — voir doc de fonction.
        for (fighter, &center) in fighters.iter().zip(template.slot_centers) {
            let pos = frame_rect.min + center.to_vec2();
            let portrait_rect = egui::Rect::from_center_size(
                pos,
                egui::vec2(NATIVE_PORTRAIT_SIZE, NATIVE_PORTRAIT_SIZE),
            );
            let id = ui.id().with(("combat-frame-slot", fighter.name.as_str()));
            let response = ui.interact(portrait_rect, id, egui::Sense::hover());
            crate::design::tooltip(&response).text(fighter.name.as_str());
            if fighter.total_damage > 0 {
                crate::design::paint_portrait_percent(
                    ui,
                    portrait_rect,
                    crate::design::portrait_percent(fighter.total_damage, total_damage),
                );
            }
        }
    }
}
