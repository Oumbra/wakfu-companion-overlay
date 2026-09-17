//! **Miroir vertical de l'overlay Combat** — la même interface, réfléchie autour d'un axe
//! vertical, pour la poser à DROITE de la fenêtre de jeu plutôt qu'à gauche (2026-09-17, demande
//! utilisateur : « permettre à l'utilisateur d'afficher l'overlay combat à droite plutôt qu'à
//! gauche ; l'ensemble de l'overlay doit donc être affiché en miroir vertical, hormis les
//! portraits, les images de monstre, les icônes (allié, ennemi, dégât, armure, soins) et les
//! images de sort »).
//!
//! ## Pourquoi un miroir de RENDU, et pas une mise en page paramétrée
//!
//! Le panneau Combat n'est pas une pile de widgets standard : c'est une centaine de rectangles
//! calculés à la main (`panels::combat`, `combat_frame`, `combat_bars`, `combat_spell_block`, les
//! composants de `design`), chacun posé à partir d'un `rect.min.x`, d'un `left_center()` ou d'un
//! `Align2::LEFT_*`. Faire descendre un booléen « à droite » jusque dans chacun d'eux aurait voulu
//! dire relire et retourner chacun de ces calculs — et surtout, le refaire à chaque futur
//! ajustement de ce panneau, sous peine de voir la version miroir diverger silencieusement de
//! l'autre.
//!
//! Ce module prend le problème par l'autre bout, en un seul endroit : **le panneau se peint
//! exactement comme d'habitude, et ce sont les formes produites qui sont réfléchies** avant d'être
//! tessellées (voir [`mirror_painted`]), pendant que **les événements de pointeur sont réfléchis
//! en sens inverse** avant d'entrer dans egui (voir [`mirror_input`]). egui continue donc de
//! raisonner dans le repère « à gauche » de bout en bout — mise en page, survol, clic, infobulles
//! — et seul l'affichage bascule. Un futur changement du panneau est en miroir sans rien demander
//! à personne.
//!
//! ## Ce qui n'est PAS retourné : les images
//!
//! Réfléchir un rectangle est sans conséquence ; réfléchir une image en ferait une image
//! retournée, et une icône « allié » regardant à l'envers n'est pas ce qui a été demandé. Deux
//! formes portent des textures, traitées chacune pour que le DESSIN reste à l'endroit :
//!
//! - `Shape::Rect` texturé (`RectShape::brush`) : le rectangle est réfléchi et les coordonnées de
//!   texture le suivent telles quelles — un rect réfléchi reste un rect normalisé, l'image s'y
//!   peint de gauche à droite comme avant ;
//! - `Shape::Mesh` avec une vraie texture (portraits, monstres, icônes du CDN, gabarits 9-slice) :
//!   le maillage est **translaté** jusqu'à la place que sa boîte occupe après réflexion, au lieu
//!   d'être réfléchi sommet par sommet. La géométrie interne est donc rigoureusement conservée,
//!   coordonnées de texture comprises (et, pour un 9-slice, ses bordures non étirées) : l'image
//!   change de côté sans se retourner.
//!
//! Un maillage SANS texture ([`egui::TextureId::default()`], l'atlas de police et son pixel blanc
//! — c'est le cas d'un dégradé peint au maillage) est bien réfléchi sommet par sommet : ce n'est
//! pas une image, et un dégradé qui va du clair au sombre doit changer de sens avec le reste.
//!
//! Le TEXTE, lui, n'est pas encore un maillage à ce stade (`Shape::Text` porte un galley mis en
//! page, tessellé plus tard) : seule la BOÎTE du texte est déplacée, jamais ses glyphes — « 12 345 »
//! reste « 12 345 », posé à droite au lieu de gauche.
//!
//! ## L'axe : le centre de la fenêtre
//!
//! [`axis_of`] réfléchit autour du centre de la surface de rendu
//! ([`egui::Context::viewport_rect`]).
//! Ce choix a une propriété qui compte : la réflexion laisse la fenêtre globalement inchangée,
//! donc tout ce qui tenait dedans y tient encore (une infobulle contrainte au bord droit se
//! retrouve contrainte au bord gauche), et un contenu collé au bord gauche se retrouve collé au
//! bord droit — ce que l'hôte complète en ancrant la fenêtre Combat au bord droit du client
//! (`main.rs::App::anchor_position`).
//!
//! ## Limite connue
//!
//! Une forme TOURNÉE (`RectShape::angle`, `TextShape::angle`) garde son angle : une réflexion
//! devrait l'inverser, autour d'un pivot lui-même réfléchi. Rien dans l'overlay ne tourne
//! aujourd'hui — à traiter le jour où quelque chose tournera, plutôt que d'écrire un code qu'aucun
//! rendu n'exerce.

use egui::epaint::{ClippedShape, CornerRadius, Mesh, Shape};
use egui::{Context, LayerId, Pos2, Rect, TextureId};

/// L'axe de réflexion pour ce contexte : l'abscisse du centre de la surface de rendu — voir la doc
/// de module (« L'axe : le centre de la fenêtre »).
pub fn axis_of(ctx: &Context) -> f32 {
    ctx.viewport_rect().center().x
}

/// Le même axe, mais tel que l'ENTRÉE de la frame le connaît (voir [`mirror_input`], appelée avant
/// qu'egui n'ait vu quoi que ce soit de cette frame).
///
/// La taille annoncée par l'hôte prime sur celle du contexte : à la toute première frame, et à
/// celle qui suit un redimensionnement de la fenêtre, le contexte porte encore l'ancienne — un axe
/// périmé enverrait les clics à côté pendant une frame.
pub fn axis_of_input(ctx: &Context, input: &egui::RawInput) -> f32 {
    input
        .screen_rect
        .map_or_else(|| axis_of(ctx), |rect| rect.center().x)
}

/// Réfléchit **tout ce qui a été peint jusqu'ici** dans ce contexte, autour de `axis_x`.
///
/// À appeler à la toute fin d'une frame, une fois le contenu complet peint (voir
/// `render_content::paint_content`) : les formes vivent dans [`egui::Context::graphics_mut`]
/// jusqu'à la tessellation, ce qui laisse exactement cette fenêtre pour les retoucher.
///
/// Les couches parcourues sont la couche de FOND (le `CentralPanel` du panneau) et toutes les
/// zones connues d'egui (`Memory::layer_ids` : infobulles, popups, fenêtres flottantes). Une
/// couche vide ne coûte rien ; une couche qui n'existerait dans aucune de ces deux familles
/// resterait non réfléchie — aucune n'est produite par l'overlay.
pub fn mirror_painted(ctx: &Context, axis_x: f32) {
    let mut layers: Vec<LayerId> = ctx.memory(|memory| memory.layer_ids().collect());
    // **Dédoublonné, sans quoi le miroir s'annulerait lui-même** : selon l'appelant, la couche de
    // fond est déjà dans `layer_ids` (elle y figure dès qu'egui la tient pour une zone ordinaire),
    // et la réfléchir deux fois ramène exactement le contenu à sa place de départ — un bug
    // silencieux, puisque le rendu est alors rigoureusement celui d'avant.
    if !layers.contains(&LayerId::background()) {
        layers.push(LayerId::background());
    }
    ctx.graphics_mut(|graphics| {
        for layer in layers {
            let Some(list) = graphics.get_mut(layer) else {
                continue;
            };
            // `PaintList` ne donne accès à ses formes qu'une par une, par indice — `next_idx` en
            // donne le nombre, `mutate_shape` l'accès en écriture.
            for index in 0..list.next_idx().0 {
                list.mutate_shape(egui::layers::ShapeIdx(index), |clipped| {
                    mirror_clipped_shape(clipped, axis_x);
                });
            }
        }
    });
}

/// Réfléchit les événements de pointeur de `input` autour de `axis_x`, **avant** qu'egui ne les
/// voie : le pointeur réel est à droite, egui le reçoit à gauche, là où il a mis en page le
/// panneau. Survol, clic, glisser et infobulles tombent donc juste sans qu'aucun panneau ne sache
/// qu'il est affiché en miroir.
///
/// `screen_rect` (la taille de la fenêtre annoncée à egui) n'est pas touché : l'axe étant son
/// centre, il est son propre reflet.
pub fn mirror_input(input: &mut egui::RawInput, axis_x: f32) {
    for event in &mut input.events {
        match event {
            egui::Event::PointerMoved(pos) => *pos = mirror_pos(*pos, axis_x),
            egui::Event::PointerButton { pos, .. } => *pos = mirror_pos(*pos, axis_x),
            egui::Event::Touch { pos, .. } => *pos = mirror_pos(*pos, axis_x),
            // Déplacements RELATIFS : pas d'axe, seulement un sens à inverser.
            egui::Event::MouseMoved(delta) => delta.x = -delta.x,
            egui::Event::MouseWheel { delta, .. } => delta.x = -delta.x,
            _ => {}
        }
    }
}

/// Réfléchit une abscisse autour de l'axe. Une borne infinie (un rectangle de découpe « tout
/// l'écran ») devient l'infini opposé, ce qui laisse le rectangle inchangé — le comportement voulu.
#[inline]
fn mirror_x(x: f32, axis_x: f32) -> f32 {
    2.0 * axis_x - x
}

#[inline]
fn mirror_pos(pos: Pos2, axis_x: f32) -> Pos2 {
    egui::pos2(mirror_x(pos.x, axis_x), pos.y)
}

/// Réfléchit un rectangle : les deux bords horizontaux échangent leur rôle, d'où la reconstruction
/// par `from_min_max` (sans elle, `min.x > max.x` donnerait un rectangle vide).
#[inline]
fn mirror_rect(rect: Rect, axis_x: f32) -> Rect {
    Rect::from_min_max(
        egui::pos2(mirror_x(rect.max.x, axis_x), rect.min.y),
        egui::pos2(mirror_x(rect.min.x, axis_x), rect.max.y),
    )
}

/// Échange les deux coins gauches avec les deux coins droits — sans quoi un bandeau arrondi
/// seulement d'un côté garderait son arrondi du mauvais côté après réflexion.
#[inline]
fn mirror_corner_radius(radius: CornerRadius) -> CornerRadius {
    CornerRadius {
        nw: radius.ne,
        ne: radius.nw,
        sw: radius.se,
        se: radius.sw,
    }
}

/// Réfléchit une forme et son rectangle de découpe.
fn mirror_clipped_shape(clipped: &mut ClippedShape, axis_x: f32) {
    clipped.clip_rect = mirror_rect(clipped.clip_rect, axis_x);
    mirror_shape(&mut clipped.shape, axis_x);
}

/// Réfléchit une forme en place — voir la doc de module pour le sort réservé aux images.
fn mirror_shape(shape: &mut Shape, axis_x: f32) {
    match shape {
        Shape::Noop | Shape::Callback(_) => {}
        Shape::Vec(shapes) => {
            for shape in shapes {
                mirror_shape(shape, axis_x);
            }
        }
        Shape::Circle(circle) => circle.center = mirror_pos(circle.center, axis_x),
        Shape::Ellipse(ellipse) => ellipse.center = mirror_pos(ellipse.center, axis_x),
        Shape::LineSegment { points, .. } => {
            for point in points {
                *point = mirror_pos(*point, axis_x);
            }
        }
        Shape::Path(path) => {
            for point in &mut path.points {
                *point = mirror_pos(*point, axis_x);
            }
            // Réfléchir un contour en inverse le sens de parcours : le remettre à l'endroit garde
            // au chemin l'orientation sur laquelle s'appuie le lissage des bords d'un chemin
            // fermé.
            path.points.reverse();
        }
        Shape::Rect(rect) => {
            rect.rect = mirror_rect(rect.rect, axis_x);
            rect.corner_radius = mirror_corner_radius(rect.corner_radius);
            // `brush.uv` reste tel quel : un rect réfléchi est toujours un rect normalisé, sa
            // texture s'y peint dans le même sens qu'avant (voir doc de module).
        }
        Shape::Text(text) => {
            // `pos` est le coin HAUT-GAUCHE du texte mis en page : c'est sa BOÎTE qu'on réfléchit,
            // pas ses glyphes — le bord droit du texte devient son bord gauche.
            text.pos = egui::pos2(
                mirror_x(text.pos.x + text.galley.size().x, axis_x),
                text.pos.y,
            );
        }
        Shape::Mesh(mesh) => mirror_mesh(std::sync::Arc::make_mut(mesh), axis_x),
        Shape::QuadraticBezier(bezier) => {
            for point in &mut bezier.points {
                *point = mirror_pos(*point, axis_x);
            }
        }
        Shape::CubicBezier(bezier) => {
            for point in &mut bezier.points {
                *point = mirror_pos(*point, axis_x);
            }
        }
    }
}

/// Un maillage TEXTURÉ est translaté jusqu'à la place de son reflet (l'image ne se retourne pas) ;
/// un maillage sans texture est réfléchi sommet par sommet (c'est une forme, pas une image) — voir
/// la doc de module.
fn mirror_mesh(mesh: &mut Mesh, axis_x: f32) {
    if mesh.texture_id == TextureId::default() {
        for vertex in &mut mesh.vertices {
            vertex.pos = mirror_pos(vertex.pos, axis_x);
        }
        return;
    }
    let bounds = mesh.calc_bounds();
    if !bounds.is_finite() {
        return;
    }
    // Le décalage qui emmène la boîte du maillage sur son reflet : son bord gauche doit atterrir
    // là où le reflet de son bord DROIT se trouve.
    let dx = mirror_x(bounds.max.x, axis_x) - bounds.min.x;
    for vertex in &mut mesh.vertices {
        vertex.pos.x += dx;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AXIS: f32 = 100.0;

    #[test]
    fn un_rectangle_colle_a_gauche_se_colle_a_droite() {
        let rect = Rect::from_min_max(egui::pos2(0.0, 10.0), egui::pos2(30.0, 20.0));
        let mirrored = mirror_rect(rect, AXIS);
        assert_eq!(
            mirrored,
            Rect::from_min_max(egui::pos2(170.0, 10.0), egui::pos2(200.0, 20.0))
        );
        // La hauteur et la largeur sont conservées, seule l'abscisse bascule.
        assert_eq!(mirrored.size(), rect.size());
    }

    #[test]
    fn un_rectangle_de_decoupe_infini_reste_infini() {
        assert_eq!(mirror_rect(Rect::EVERYTHING, AXIS), Rect::EVERYTHING);
    }

    #[test]
    fn deux_reflexions_reviennent_au_point_de_depart() {
        let rect = Rect::from_min_max(egui::pos2(12.0, 3.0), egui::pos2(48.0, 9.0));
        assert_eq!(mirror_rect(mirror_rect(rect, AXIS), AXIS), rect);
    }

    #[test]
    fn les_coins_arrondis_changent_de_cote() {
        let radius = CornerRadius {
            nw: 6,
            ne: 0,
            sw: 4,
            se: 1,
        };
        assert_eq!(
            mirror_corner_radius(radius),
            CornerRadius {
                nw: 0,
                ne: 6,
                sw: 1,
                se: 4,
            }
        );
    }

    /// Une image (maillage texturé) ne doit pas se retourner : sa boîte bascule, sa géométrie
    /// interne — coordonnées de texture comprises — reste rigoureusement la même.
    #[test]
    fn une_image_change_de_cote_sans_se_retourner() {
        let mut mesh = Mesh::with_texture(TextureId::Managed(7));
        mesh.add_rect_with_uv(
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(40.0, 40.0)),
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        let before = mesh.clone();
        mirror_mesh(&mut mesh, AXIS);

        assert_eq!(
            mesh.calc_bounds(),
            mirror_rect(before.calc_bounds(), AXIS),
            "la boîte de l'image doit bien avoir basculé"
        );
        for (after, before) in mesh.vertices.iter().zip(&before.vertices) {
            assert_eq!(after.uv, before.uv, "la texture ne doit pas être retournée");
        }
        // Une translation pure : l'écart entre deux sommets est inchangé, signe qu'aucune
        // réflexion n'a eu lieu à l'intérieur du maillage.
        let width_before = before.vertices[1].pos.x - before.vertices[0].pos.x;
        let width_after = mesh.vertices[1].pos.x - mesh.vertices[0].pos.x;
        assert_eq!(width_after, width_before);
    }

    /// Un maillage SANS texture est une forme (un dégradé, par exemple) : lui, se réfléchit.
    #[test]
    fn un_degrade_sans_texture_se_reflechit() {
        let mut mesh = Mesh::default();
        mesh.add_rect_with_uv(
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(40.0, 10.0)),
            Rect::from_min_max(egui::epaint::WHITE_UV, egui::epaint::WHITE_UV),
            egui::Color32::WHITE,
        );
        let before = mesh.clone();
        mirror_mesh(&mut mesh, AXIS);
        for (after, before) in mesh.vertices.iter().zip(&before.vertices) {
            assert_eq!(after.pos.x, mirror_x(before.pos.x, AXIS));
        }
    }

    /// **Bout en bout, sur un vrai `egui::Context`** : un rectangle peint contre le bord GAUCHE
    /// d'une fenêtre ressort contre son bord DROIT, et le texte qui l'accompagne suit sans que ses
    /// glyphes soient retournés (`Shape::Text` conserve son galley, seule sa boîte bascule).
    ///
    /// Sans GPU ni fenêtre : `Context::run` suffit à produire les formes, qui sont justement ce
    /// que ce module retouche.
    #[test]
    fn un_contenu_colle_a_gauche_ressort_colle_a_droite() {
        let ctx = Context::default();
        let window = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 100.0));
        let input = egui::RawInput {
            screen_rect: Some(window),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            let galley = ui.painter().layout_no_wrap(
                "1 234".to_owned(),
                egui::FontId::proportional(12.0),
                egui::Color32::WHITE,
            );
            ui.painter().rect_filled(
                Rect::from_min_size(egui::pos2(0.0, 10.0), egui::vec2(30.0, 8.0)),
                0,
                egui::Color32::RED,
            );
            ui.painter()
                .galley(egui::pos2(0.0, 40.0), galley, egui::Color32::PLACEHOLDER);
            let ctx = ui.ctx();
            mirror_painted(ctx, axis_of(ctx));
        });

        // Les formes seules nous intéressent ; le reste de la sortie (dont les textures créées
        // pour le texte) doit être abandonné explicitement, sans quoi `FullOutput` panique à sa
        // destruction — une frame produite sans peintre réel n'applique aucun delta.
        let shapes = output.shapes.clone();
        output.drop_without_applying_deltas();

        let rouge = shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                Shape::Rect(rect) if rect.fill == egui::Color32::RED => Some(rect.rect),
                _ => None,
            })
            .expect("le rectangle peint doit être dans la sortie");
        assert_eq!(
            rouge,
            Rect::from_min_size(egui::pos2(170.0, 10.0), egui::vec2(30.0, 8.0))
        );

        let texte = shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                Shape::Text(text) => Some(text),
                _ => None,
            })
            .expect("le texte peint doit être dans la sortie");
        assert_eq!(texte.pos, egui::pos2(200.0 - texte.galley.size().x, 40.0));
        assert_eq!(
            texte.galley.job.text, "1 234",
            "les glyphes ne doivent jamais être retournés"
        );
    }

    /// Le pointeur réel est à droite ; egui, qui a mis en page à gauche, doit le recevoir à gauche.
    #[test]
    fn le_pointeur_est_traduit_vers_le_repere_de_mise_en_page() {
        let mut input = egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(egui::pos2(180.0, 5.0)),
                egui::Event::MouseMoved(egui::vec2(3.0, 7.0)),
            ],
            ..Default::default()
        };
        mirror_input(&mut input, AXIS);
        assert_eq!(
            input.events[0],
            egui::Event::PointerMoved(egui::pos2(20.0, 5.0))
        );
        assert_eq!(
            input.events[1],
            egui::Event::MouseMoved(egui::vec2(-3.0, 7.0))
        );
    }
}
