//! **Miroir vertical de l'overlay Combat** — la même interface, basculée de l'autre côté, pour la
//! poser à DROITE de la fenêtre de jeu plutôt qu'à gauche (2026-09-17, demande utilisateur).
//!
//! ## Ce qui bascule, et ce qui ne bascule pas
//!
//! La première version de ce module réfléchissait TOUT, forme par forme. Essayée en jeu le jour
//! même, elle a été rejetée sur trois points, qui disent ensemble ce que le miroir doit être :
//!
//! - **le gabarit du cadre n'était pas retourné** (il était traité comme une image, donc
//!   simplement déplacé) : son ornement pointait du mauvais côté, « le rendu est complètement
//!   affreux » ;
//! - **l'ordre des cases des switches s'inversait** : ce doit rester Alliés puis Ennemis, Dégâts
//!   puis Armure puis Soins, quel que soit le côté ;
//! - **le sens de lecture s'inversait** : le total passait avant son switch, le chiffre de dégâts
//!   avant le nom qu'il qualifie, les sorts du dernier au premier.
//!
//! D'où la règle, qui n'est pas « tout réfléchir » mais **réfléchir la PLACE des blocs, jamais
//! leur contenu** :
//!
//! - ce qui est peint librement — le gabarit du cadre, les ascenseurs, les fondus — est
//!   **réfléchi** : c'est du décor, il doit regarder vers le jeu ;
//! - ce qui forme un bloc qu'on LIT — un bandeau et son switch, un groupe nom + dégâts + barre, la
//!   ligne de sorts, un portrait et son pourcentage — est déclaré par [`upright`] /
//!   [`upright_in`] : **sa boîte va à la place du reflet, son contenu ne bouge pas d'un pixel**.
//!   Il se lit donc toujours de gauche à droite, dans le même ordre, avec ses images à l'endroit.
//!
//! Une infobulle, un menu, tout ce qui vit dans sa propre couche egui, est un bloc de ce genre
//! sans avoir à le déclarer : sa couche entière est déplacée telle quelle.
//!
//! ## Pourquoi au rendu, et pas dans la mise en page
//!
//! Le panneau Combat est une centaine de rectangles calculés à la main (`panels::combat`,
//! `combat_frame`, `combat_bars`, `combat_spell_block`, les composants de `design`). Faire
//! descendre un booléen « à droite » jusque dans chacun d'eux aurait voulu dire retourner chacun
//! de ces calculs, puis le refaire à chaque futur ajustement, sous peine de voir la version miroir
//! diverger en silence.
//!
//! Ici, le panneau se peint comme d'habitude ; ce sont les formes produites qui sont réfléchies
//! avant d'être tessellées ([`apply`]), pendant que les événements de pointeur font le chemin
//! inverse avant d'entrer dans egui ([`mirror_input`]). egui continue de raisonner dans le repère
//! « à gauche » de bout en bout — mise en page, survol, clic, placement des infobulles — et les
//! panneaux n'ont à déclarer qu'une chose : **où sont leurs blocs**.
//!
//! ## Le pointeur fait le chemin exactement inverse — par morceaux, lui aussi
//!
//! La première version de l'entrée était une réflexion, symétrique de la sortie d'alors. Elle est
//! devenue FAUSSE le jour où les blocs ont cessé d'être réfléchis pour être déplacés, et l'essai
//! en jeu du 2026-09-17 l'a dit sans ambiguïté : « graphiquement ils sont tracés à un endroit,
//! mais le survol reste identique à leur version non mirrorée ». Sur un switch de trois cases,
//! survoler la case de GAUCHE visait celle de DROITE : pas d'infobulle là où on la montrait, et un
//! clic qui activait le voisin.
//!
//! La règle manquante tient en une phrase : **l'entrée doit être l'inverse exact de la sortie**,
//! donc elle est par morceaux comme elle. [`apply`] laisse derrière lui la carte des blocs
//! déplacés — leur boîte telle qu'elle est AFFICHÉE, et de combien elle a bougé ; [`mirror_input`]
//! la relit :
//!
//! - le point tombe dans un bloc affiché → il subit le déplacement INVERSE de ce bloc. Il vise
//!   donc l'élément qui est sous le curseur, sans retournement à l'intérieur du bloc ;
//! - il tombe ailleurs — le décor — → il est réfléchi, comme le décor l'a été.
//!
//! Corollaire à la sortie : une infobulle née d'un survol suit le déplacement du bloc survolé au
//! lieu d'aller à la place de son reflet. Sans ça, egui la placerait près du pointeur de MISE EN
//! PAGE et le miroir l'enverrait là où ce pointeur se reflète — à l'autre bout du panneau.
//!
//! Deux limites, assumées : la carte est celle de la frame PRÉCÉDENTE (c'est la seule qui existe
//! quand l'entrée est traduite — la frame en cours n'a encore rien peint), donc la première frame
//! après l'armement du miroir, comme celle qui suit un redimensionnement, retombe sur la réflexion
//! seule ; et un bloc ne connaît de lui-même que son ENCRE, si bien qu'une zone cliquable qui
//! déborderait de ce que le bloc peint retombe elle aussi sur la réflexion — d'où l'ancre
//! explicite d'[`upright_in`] dès qu'un bloc ne remplit pas son emplacement.
//!
//! ## L'axe : le centre de la fenêtre
//!
//! [`axis_of`] réfléchit autour du centre de la surface de rendu
//! ([`egui::Context::viewport_rect`]). Ce choix a une propriété qui compte : la réflexion laisse la
//! fenêtre globalement inchangée, donc tout ce qui tenait dedans y tient encore (une infobulle
//! contrainte au bord droit se retrouve contrainte au bord gauche), et un contenu collé au bord
//! gauche se retrouve collé au bord droit — ce que l'hôte complète en ancrant la fenêtre Combat au
//! bord droit du client (`main.rs::App::anchor_position`).
//!
//! ## Mode d'emploi
//!
//! `render_content::paint_content` [`arm`]e le miroir avant de peindre le panneau et l'[`apply`]e
//! après. Entre les deux, [`upright`]/[`upright_in`] ne coûtent rien tant que le miroir n'est pas
//! armé : un panneau les appelle sans se demander de quel côté il est affiché.
//!
//! ## Limite connue
//!
//! Une forme TOURNÉE (`RectShape::angle`, `TextShape::angle`) garde son angle&nbsp;: une réflexion
//! devrait l'inverser, autour d'un pivot lui-même réfléchi. Rien dans l'overlay ne tourne
//! aujourd'hui — à traiter le jour où quelque chose tournera, plutôt que d'écrire un code qu'aucun
//! rendu n'exerce.

use egui::epaint::{CornerRadius, Shape};
use egui::{Context, LayerId, Pos2, Rect};

/// **Le miroir est armé pour cette frame**, et autour de quel axe — voir [`arm`]. Vit dans
/// `Context::data` le temps d'une frame ; [`apply`] le reprend et le jette.
///
/// Séparé des blocs (ci-dessous) parce qu'il est lu à chaque [`upright`], et qu'une valeur `Copy`
/// se teste sans recopier la liste des blocs déjà déclarés.
#[derive(Clone, Copy, Default)]
struct Armed {
    axis_x: f32,
    /// Profondeur d'imbrication des blocs en cours de peinture : un bloc déclaré DANS un autre
    /// n'est pas enregistré (son parent le déplace déjà, l'enregistrer le déplacerait deux fois).
    depth: u32,
}

/// Les blocs déclarés depuis [`arm`] : pour chacun, la plage de formes qu'il a peintes dans sa
/// couche, et l'ancre qui décide de son déplacement.
#[derive(Clone, Default)]
struct Blocks(Vec<Block>);

#[derive(Clone)]
struct Block {
    layer: LayerId,
    start: usize,
    end: usize,
    /// Rectangle dont la place commande celle du bloc — `None` : la boîte de ce que le bloc a
    /// peint (voir [`upright`]).
    anchor: Option<Rect>,
}

/// **La carte que [`apply`] laisse à l'entrée de la frame suivante** : pour chaque bloc déplacé,
/// sa boîte TELLE QU'ELLE EST AFFICHÉE et le déplacement qui l'y a menée — voir la doc de module
/// (« Le pointeur fait le chemin exactement inverse »).
///
/// Ne contient que les blocs de la couche de FOND. Une infobulle, elle, n'est pas dans la carte :
/// egui ne la survole jamais (sa couche n'est pas interactive), et l'y mettre ferait viser au
/// pointeur qui la frôle un point logique arbitraire — le bloc en dessous perdrait son survol, la
/// bulle se fermerait, et elle se rouvrirait à la frame d'après.
#[derive(Clone, Default)]
struct Shifted(Vec<ShiftedBlock>);

#[derive(Clone, Copy)]
struct ShiftedBlock {
    on_screen: Rect,
    dx: f32,
}

impl Shifted {
    /// Le déplacement du bloc affiché sous ce point, s'il y en a un — **le dernier peint gagne**,
    /// c'est celui qu'on voit par-dessus les autres.
    fn under(&self, pos: Pos2) -> Option<f32> {
        self.0
            .iter()
            .rev()
            .find(|block| block.on_screen.contains(pos))
            .map(|block| block.dx)
    }
}

/// Ce qu'a subi la dernière position de pointeur traduite par [`mirror_input`] : de combien elle a
/// été déplacée, et si c'était au titre d'un bloc (plutôt que du décor réfléchi).
///
/// Lu par [`apply`] pour poser les infobulles, et par [`mirror_input`] lui-même pour les
/// déplacements RELATIFS, qui n'ont pas de position à quoi se raccrocher.
#[derive(Clone, Copy)]
struct PointerShift {
    dx: f32,
    in_block: bool,
}

/// Comment un bloc rejoint sa place à l'écran — voir [`shift_range`].
#[derive(Clone, Copy)]
enum Move {
    /// Sa boîte va à la place de son reflet : le cas de tout bloc du panneau.
    Mirrored,
    /// Déplacement imposé : celui du bloc sous le pointeur, pour l'infobulle qui en naît.
    By(f32),
}

fn state_id() -> egui::Id {
    egui::Id::new("wakfu-overlay-mirror")
}

fn shift_id() -> egui::Id {
    egui::Id::new("wakfu-overlay-mirror-shifts")
}

fn pointer_id() -> egui::Id {
    egui::Id::new("wakfu-overlay-mirror-pointer")
}

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

/// **Arme le miroir pour la frame en cours** : à partir d'ici, [`upright`] enregistre les blocs
/// qu'on lui déclare, et [`apply`] réfléchira tout ce qui a été peint entre les deux.
///
/// À appeler AVANT la première forme du panneau : ce qui est peint avant n'est pas concerné.
pub fn arm(ctx: &Context, axis_x: f32) {
    ctx.data_mut(|data| {
        data.insert_temp(state_id(), Armed { axis_x, depth: 0 });
        data.insert_temp(state_id(), Blocks::default());
    });
}

/// **Déclare un bloc qui se lit** : sa PLACE est réfléchie, son CONTENU ne l'est pas — voir la doc
/// de module.
///
/// L'ancre est la boîte de ce que le bloc a peint. Quand cette boîte ne dit pas exactement la
/// place du bloc — un contenu qui ne remplit pas son emplacement, deux appels qui doivent se
/// déplacer ensemble — passer ce rectangle explicitement avec [`upright_in`].
///
/// Sans miroir armé (le cas ordinaire, panneau à gauche), l'appel se réduit à exécuter `f` : rien
/// n'est mesuré, rien n'est enregistré.
pub fn upright<R>(ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    upright_impl(ui, None, f)
}

/// [`upright`] avec une ancre EXPLICITE : le bloc est déplacé de sorte que `anchor` aille à la
/// place de son reflet, quoi que le bloc peigne à l'intérieur.
///
/// C'est ce qu'il faut dès que deux appels doivent rester solidaires — un portrait et le
/// pourcentage peint par-dessus, peints en deux temps — ou qu'un bloc doit se caler sur son
/// emplacement plutôt que sur son encre.
pub fn upright_in<R>(ui: &mut egui::Ui, anchor: Rect, f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    upright_impl(ui, Some(anchor), f)
}

fn upright_impl<R>(
    ui: &mut egui::Ui,
    anchor: Option<Rect>,
    f: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let ctx = ui.ctx().clone();
    // `false` : miroir non armé, ou bloc imbriqué dans un autre — dans les deux cas il n'y a rien
    // à enregistrer, seulement le contenu à peindre.
    if !enter(&ctx) {
        return f(ui);
    }
    let layer = ui.layer_id();
    let start = ctx.graphics_mut(|graphics| graphics.entry(layer).next_idx().0);
    let out = f(ui);
    let end = ctx.graphics_mut(|graphics| graphics.entry(layer).next_idx().0);
    ctx.data_mut(|data| {
        leave(data);
        if end > start {
            data.get_temp_mut_or_default::<Blocks>(state_id())
                .0
                .push(Block {
                    layer,
                    start,
                    end,
                    anchor,
                });
        }
    });
    out
}

/// Ouvre un bloc : vrai seulement si le miroir est armé ET qu'aucun bloc n'est déjà ouvert. La
/// profondeur monte dans tous les cas où le miroir est armé, pour que la sortie compte juste.
fn enter(ctx: &Context) -> bool {
    ctx.data_mut(|data| {
        let Some(mut armed) = data.get_temp::<Armed>(state_id()) else {
            return false;
        };
        armed.depth += 1;
        data.insert_temp(state_id(), armed);
        armed.depth == 1
    })
}

fn leave(data: &mut egui::util::IdTypeMap) {
    if let Some(mut armed) = data.get_temp::<Armed>(state_id()) {
        armed.depth = armed.depth.saturating_sub(1);
        data.insert_temp(state_id(), armed);
    }
}

/// **Applique le miroir à tout ce qui a été peint depuis [`arm`]**, puis le désarme.
///
/// À appeler à la toute fin d'une frame, une fois le contenu complet peint (voir
/// `render_content::paint_content`) : les formes vivent dans [`egui::Context::graphics_mut`]
/// jusqu'à la tessellation, ce qui laisse exactement cette fenêtre pour les retoucher.
///
/// Les couches parcourues sont la couche de FOND (le `CentralPanel` du panneau) et toutes les
/// zones connues d'egui (`Memory::layer_ids` : infobulles, popups, fenêtres flottantes). La couche
/// de fond est traitée bloc par bloc ; **chaque autre couche est déplacée d'un seul tenant**,
/// comme un bloc — une infobulle change de côté sans que son texte ni son cadre ne bougent l'un
/// par rapport à l'autre. Quand le pointeur est posé sur un bloc, ces couches suivent le
/// déplacement de CE bloc : une infobulle naît du survol, elle doit s'ouvrir là où on survole.
///
/// Laisse derrière elle la carte des blocs déplacés, que [`mirror_input`] lira à la frame suivante
/// pour traduire le pointeur — voir la doc de module.
pub fn apply(ctx: &Context) {
    let Some((axis_x, blocks)) = ctx.data_mut(|data| {
        let armed = data.remove_temp::<Armed>(state_id())?;
        let blocks = data.remove_temp::<Blocks>(state_id()).unwrap_or_default();
        Some((armed.axis_x, blocks.0))
    }) else {
        return;
    };
    let background = LayerId::background();
    let viewport = ctx.viewport_rect();
    // Le déplacement qu'a subi le pointeur en entrant (voir `mirror_input`) : une infobulle née de
    // ce survol le suit, au lieu d'aller à la place de son reflet.
    let floating = match ctx.data(|data| data.get_temp::<PointerShift>(pointer_id())) {
        Some(PointerShift { dx, in_block: true }) => Move::By(dx),
        _ => Move::Mirrored,
    };
    let mut shifted = Vec::new();

    let mut layers: Vec<LayerId> = ctx.memory(|memory| memory.layer_ids().collect());
    // **Dédoublonné, sans quoi le miroir s'annulerait lui-même** : selon l'appelant, la couche de
    // fond est déjà dans `layer_ids`, et la traiter deux fois ramène exactement le contenu à sa
    // place de départ — un bug silencieux, puisque le rendu est alors rigoureusement celui d'avant.
    if !layers.contains(&background) {
        layers.push(background);
    }

    ctx.graphics_mut(|graphics| {
        for layer in layers {
            let Some(list) = graphics.get_mut(layer) else {
                continue;
            };
            let count = list.next_idx().0;
            if count == 0 {
                continue;
            }
            if layer != background {
                // Une zone (infobulle, popup) est un bloc à elle seule — voir la doc de fonction.
                // Bornée à la fenêtre : egui l'y avait contrainte AVANT le miroir, du côté
                // gauche ; son reflet la collerait au bord droit, coin arrondi contre l'arête.
                shift_range(list, 0, count, None, floating, axis_x, Some(viewport));
                continue;
            }
            let mut layer_blocks: Vec<&Block> =
                blocks.iter().filter(|block| block.layer == layer).collect();
            layer_blocks.sort_by_key(|block| block.start);
            // Tout ce qui n'appartient à aucun bloc est du décor : réfléchi, pas déplacé.
            let mut next = 0usize;
            for block in &layer_blocks {
                reflect_range(list, next, block.start.min(count), axis_x);
                next = block.end.min(count);
            }
            reflect_range(list, next, count, axis_x);
            for block in layer_blocks {
                // Pas de bornage ici, contrairement aux zones : les blocs d'un même panneau se
                // répondent (les barres entre elles, le bandeau au-dessus d'elles), et en
                // recaler un seul le désalignerait des autres.
                let moved = shift_range(
                    list,
                    block.start.min(count),
                    block.end.min(count),
                    block.anchor,
                    Move::Mirrored,
                    axis_x,
                    None,
                );
                // Dans l'ordre de peinture : `Shifted::under` prend le dernier, donc celui du
                // dessus.
                if let Some((anchor, dx)) = moved {
                    shifted.push(ShiftedBlock {
                        on_screen: anchor.translate(egui::vec2(dx, 0.0)),
                        dx,
                    });
                }
            }
        }
    });
    ctx.data_mut(|data| data.insert_temp(shift_id(), Shifted(shifted)));
}

/// Réfléchit chaque forme de la plage, une par une — le traitement du décor.
fn reflect_range(list: &mut egui::layers::PaintList, start: usize, end: usize, axis_x: f32) {
    for index in start..end {
        list.mutate_shape(egui::layers::ShapeIdx(index), |clipped| {
            clipped.clip_rect = mirror_rect(clipped.clip_rect, axis_x);
            reflect_shape(&mut clipped.shape, axis_x);
        });
    }
}

/// Déplace la plage d'un seul tenant, de sorte que son ancre aille à la place que dit `how` — le
/// traitement d'un bloc (voir [`upright`]). Rien n'est réfléchi à l'intérieur.
///
/// Renvoie l'ancre retenue et le déplacement appliqué, de quoi tenir la carte que lira l'entrée
/// ([`Shifted`]) ; `None` quand la plage ne peint rien de mesurable.
fn shift_range(
    list: &mut egui::layers::PaintList,
    start: usize,
    end: usize,
    anchor: Option<Rect>,
    how: Move,
    axis_x: f32,
    keep_inside: Option<Rect>,
) -> Option<(Rect, f32)> {
    let anchor = anchor.unwrap_or_else(|| {
        let mut bounds = Rect::NOTHING;
        for index in start..end {
            list.mutate_shape(egui::layers::ShapeIdx(index), |clipped| {
                // **L'ombre portée ne compte pas dans la boîte** : elle est DÉCALÉE sous ce
                // qu'elle ombre (`Shadow::offset`), donc elle déborde d'un seul côté et tirerait
                // le bloc de quelques pixels — observé sur l'infobulle d'un portrait, dont le
                // texte finissait par toucher le bord de sa bulle.
                if matches!(&clipped.shape, Shape::Rect(rect) if rect.blur_width > 0.0) {
                    return;
                }
                let shape_bounds = clipped.shape.visual_bounding_rect();
                if shape_bounds.is_finite() && shape_bounds.is_positive() {
                    bounds = bounds.union(shape_bounds);
                }
            });
        }
        bounds
    });
    if !anchor.is_finite() || !anchor.is_positive() {
        return None;
    }
    // Le bord gauche du bloc doit atterrir là où se trouve le reflet de son bord DROIT.
    let mut dx = match how {
        Move::Mirrored => mirror_x(anchor.max.x, axis_x) - anchor.min.x,
        Move::By(dx) => dx,
    };
    if let Some(bounds) = keep_inside {
        let moved = anchor.translate(egui::vec2(dx, 0.0));
        if moved.width() <= bounds.width() {
            dx += (bounds.max.x - moved.max.x).min(0.0) + (bounds.min.x - moved.min.x).max(0.0);
        }
    }
    let delta = egui::vec2(dx, 0.0);
    for index in start..end {
        list.mutate_shape(egui::layers::ShapeIdx(index), |clipped| {
            clipped.clip_rect = mirror_clip(clipped.clip_rect, anchor, delta, how, axis_x);
            clipped.shape.translate(delta);
        });
    }
    Some((anchor, dx))
}

/// Le rectangle de découpe d'une forme appartenant à un bloc déplacé — **déplacé avec elle, ou
/// réfléchi**, selon d'où il vient :
///
/// - un clip serré DANS le bloc (au sens horizontal) découpe un élément précis du bloc : la case
///   d'un switch écrête son pictogramme, par exemple. Il suit donc le bloc, sans quoi la forme
///   déplacée sort de son propre clip et **disparaît** — c'est ce qui a effacé les icônes des deux
///   switches au premier essai de cette mécanique ;
/// - un clip venu de plus haut (la fenêtre, la bande de défilement d'un cadre) est une région de
///   l'écran, pas une pièce du bloc : il est réfléchi comme le reste du décor. Un clip large et
///   centré est son propre reflet ; un clip qui épouse une colonne se retrouve sur la colonne
///   réfléchie, là où le bloc atterrit.
///
/// `how` change la seconde branche quand le bloc ne va pas à la place de son reflet — voir le
/// corps.
fn mirror_clip(clip: Rect, anchor: Rect, delta: egui::Vec2, how: Move, axis_x: f32) -> Rect {
    if clip.min.x >= anchor.min.x && clip.max.x <= anchor.max.x {
        return clip.translate(delta);
    }
    let reflected = mirror_rect(clip, axis_x);
    match how {
        Move::Mirrored => reflected,
        // Déplacement IMPOSÉ (l'infobulle qui suit le bloc survolé) : le reflet d'un clip ne tombe
        // plus là où le bloc atterrit, et la forme sortirait de son propre clip — donc il suit,
        // sauf s'il est son propre reflet (un clip d'écran : infini, ou centré sur l'axe), qui est
        // une région de l'écran et n'a aucune raison de bouger.
        Move::By(_) if reflected == clip => clip,
        Move::By(_) => clip.translate(delta),
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

/// Réfléchit une forme en place — le décor, donc y compris les images : c'est ce qui retourne le
/// gabarit du cadre vers le jeu. Ce qui ne doit pas se retourner n'est pas ici, mais dans un bloc
/// (voir [`upright`]).
fn reflect_shape(shape: &mut Shape, axis_x: f32) {
    match shape {
        Shape::Noop | Shape::Callback(_) => {}
        Shape::Vec(shapes) => {
            for shape in shapes {
                reflect_shape(shape, axis_x);
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
            // au chemin l'orientation sur laquelle s'appuie le lissage des bords d'un chemin fermé.
            path.points.reverse();
        }
        Shape::Rect(rect) => {
            // Une image portée par un rect (`RectShape::brush`) est bien RETOURNÉE ici : les
            // coordonnées de texture sont inversées avec le rectangle.
            rect.rect = mirror_rect(rect.rect, axis_x);
            rect.corner_radius = mirror_corner_radius(rect.corner_radius);
            if let Some(brush) = &mut rect.brush {
                let brush = std::sync::Arc::make_mut(brush);
                brush.uv = Rect::from_min_max(
                    egui::pos2(brush.uv.max.x, brush.uv.min.y),
                    egui::pos2(brush.uv.min.x, brush.uv.max.y),
                );
            }
        }
        Shape::Text(text) => {
            // `pos` est le coin HAUT-GAUCHE du texte mis en page : c'est sa BOÎTE qu'on réfléchit,
            // jamais ses glyphes — un texte retourné ne se lirait plus. Du texte hors bloc est
            // rare (un message d'état oublié) ; le déclarer dans un bloc reste préférable, l'ordre
            // des mots d'une même ligne étant alors le seul à ne pas basculer.
            text.pos = egui::pos2(
                mirror_x(text.pos.x + text.galley.size().x, axis_x),
                text.pos.y,
            );
        }
        Shape::Mesh(mesh) => {
            for vertex in &mut std::sync::Arc::make_mut(mesh).vertices {
                vertex.pos = mirror_pos(vertex.pos, axis_x);
            }
        }
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

/// **Traduit les événements de pointeur de `input` vers le repère de mise en page**, avant
/// qu'egui ne les voie : le pointeur réel est à droite, egui le reçoit là où il a mis en page le
/// panneau. Survol, clic, glisser et infobulles tombent donc juste sans qu'aucun panneau ne sache
/// qu'il est affiché en miroir.
///
/// **C'est l'inverse exact de [`apply`], morceau par morceau** — la doc de module dit pourquoi une
/// simple réflexion ne suffit pas (et ce qu'elle cassait) : un point posé sur un bloc affiché
/// remonte le déplacement de ce bloc, un point posé sur le décor est réfléchi. La carte des blocs
/// est celle de la frame précédente, la seule qui existe à cet instant ; sans carte — première
/// frame, panneau qui vient de passer à droite — tout est réfléchi, comme avant.
///
/// `screen_rect` (la taille de la fenêtre annoncée à egui) n'est pas touché : l'axe étant son
/// centre, il est son propre reflet.
pub fn mirror_input(ctx: &Context, input: &mut egui::RawInput, axis_x: f32) {
    let shifted = ctx
        .data(|data| data.get_temp::<Shifted>(shift_id()))
        .unwrap_or_default();
    let mut pointer = ctx.data(|data| data.get_temp::<PointerShift>(pointer_id()));
    for event in &mut input.events {
        match event {
            egui::Event::PointerMoved(pos)
            | egui::Event::PointerButton { pos, .. }
            | egui::Event::Touch { pos, .. } => {
                let (moved, in_block) = match shifted.under(*pos) {
                    Some(dx) => (egui::pos2(pos.x - dx, pos.y), true),
                    None => (mirror_pos(*pos, axis_x), false),
                };
                pointer = Some(PointerShift {
                    dx: pos.x - moved.x,
                    in_block,
                });
                *pos = moved;
            }
            // Déplacements RELATIFS : pas de position à traduire, seulement un SENS — inversé sur
            // le décor, qui est réfléchi ; conservé dans un bloc, qui est seulement déplacé. D'où
            // la position retenue ci-dessus : elle seule dit dans lequel des deux on se trouve.
            egui::Event::MouseMoved(delta) | egui::Event::MouseWheel { delta, .. }
                if !pointer.is_some_and(|pointer| pointer.in_block) =>
            {
                delta.x = -delta.x;
            }
            _ => {}
        }
    }
    // Relu par `apply` à la fin de cette même frame, pour poser les infobulles là où on survole.
    if let Some(pointer) = pointer {
        ctx.data_mut(|data| data.insert_temp(pointer_id(), pointer));
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

    /// Le clip d'un élément DANS un bloc suit le bloc ; celui qui vient de plus haut est réfléchi
    /// — voir [`mirror_clip`], et les icônes de switch que la première version faisait disparaître.
    #[test]
    fn un_clip_serre_suit_son_bloc_un_clip_herite_est_reflechi() {
        let anchor = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(40.0, 10.0));
        let delta = egui::vec2(160.0, 0.0);
        let case = Rect::from_min_max(egui::pos2(4.0, 1.0), egui::pos2(18.0, 9.0));
        assert_eq!(
            mirror_clip(case, anchor, delta, Move::Mirrored, AXIS),
            case.translate(delta),
            "la case d'un switch écrête son pictogramme : elle doit partir avec lui"
        );
        let fenetre = Rect::from_min_max(egui::pos2(-10.0, -10.0), egui::pos2(210.0, 90.0));
        assert_eq!(
            mirror_clip(fenetre, anchor, delta, Move::Mirrored, AXIS),
            mirror_rect(fenetre, AXIS),
            "un clip venu de plus haut est une région de l'écran, pas une pièce du bloc"
        );
        // Déplacement imposé (l'infobulle qui suit le bloc survolé) : un clip asymétrique suit,
        // sans quoi la forme déplacée sortirait de son clip ; un clip d'écran ne bouge pas.
        let asymetrique = Rect::from_min_max(egui::pos2(-10.0, -10.0), egui::pos2(60.0, 90.0));
        assert_eq!(
            mirror_clip(asymetrique, anchor, delta, Move::By(delta.x), AXIS),
            asymetrique.translate(delta)
        );
        assert_eq!(
            mirror_clip(fenetre, anchor, delta, Move::By(delta.x), AXIS),
            fenetre,
            "la fenêtre est son propre reflet : elle reste où elle est"
        );
    }

    /// Une frame complète sur un vrai `egui::Context`, sans GPU ni fenêtre : **le décor est
    /// retourné, le bloc déclaré change seulement de place**.
    ///
    /// C'est la règle entière du module en un test : le gabarit du cadre (ici un rectangle
    /// texturé) doit basculer, pendant que le bloc garde son ordre de lecture — deux carrés
    /// peints l'un après l'autre restent dans le même ordre, et le texte reste lisible.
    #[test]
    fn le_decor_est_retourne_le_bloc_declare_change_seulement_de_place() {
        let ctx = Context::default();
        let window = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 100.0));
        let input = egui::RawInput {
            screen_rect: Some(window),
            ..Default::default()
        };
        let decor = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(20.0, 100.0));
        let bloc = Rect::from_min_max(egui::pos2(30.0, 10.0), egui::pos2(90.0, 30.0));
        let premier = Rect::from_min_max(egui::pos2(32.0, 12.0), egui::pos2(42.0, 22.0));
        let second = Rect::from_min_max(egui::pos2(50.0, 12.0), egui::pos2(60.0, 22.0));

        let output = ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            arm(&ctx, axis_of(&ctx));
            ui.painter().rect_filled(decor, 0, egui::Color32::RED);
            upright_in(ui, bloc, |ui| {
                ui.painter().rect_filled(premier, 0, egui::Color32::GREEN);
                ui.painter().rect_filled(second, 0, egui::Color32::BLUE);
            });
            apply(&ctx);
        });
        let shapes = output.shapes.clone();
        output.drop_without_applying_deltas();

        let peint = |couleur: egui::Color32| {
            shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    Shape::Rect(rect) if rect.fill == couleur => Some(rect.rect),
                    _ => None,
                })
                .expect("forme peinte absente de la sortie")
        };

        // Décor : réfléchi, il passe de l'autre bord.
        assert_eq!(peint(egui::Color32::RED), mirror_rect(decor, AXIS));

        // Bloc : sa PLACE est le reflet de la sienne…
        let vert = peint(egui::Color32::GREEN);
        let bleu = peint(egui::Color32::BLUE);
        let place = vert.union(bleu);
        assert!(
            place.min.x > 100.0,
            "le bloc doit avoir changé de côté (trouvé {place:?})"
        );
        // … et son CONTENU n'a pas bougé : le vert reste à gauche du bleu, chacun à sa taille et
        // à son écart d'origine. Un miroir naïf les aurait échangés.
        assert!(vert.max.x <= bleu.min.x, "l'ordre de lecture doit tenir");
        assert_eq!(vert.size(), premier.size());
        assert_eq!(bleu.min.x - vert.min.x, second.min.x - premier.min.x);
    }

    /// Le pointeur réel est à droite ; egui, qui a mis en page à gauche, doit le recevoir à
    /// gauche. Sur le DÉCOR — ici sans aucune carte de blocs — c'est une réflexion, comme la
    /// sortie.
    #[test]
    fn le_pointeur_est_traduit_vers_le_repere_de_mise_en_page() {
        let ctx = Context::default();
        let mut input = egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(egui::pos2(180.0, 5.0)),
                egui::Event::MouseMoved(egui::vec2(3.0, 7.0)),
            ],
            ..Default::default()
        };
        mirror_input(&ctx, &mut input, AXIS);
        assert_eq!(
            input.events[0],
            egui::Event::PointerMoved(egui::pos2(20.0, 5.0))
        );
        assert_eq!(
            input.events[1],
            egui::Event::MouseMoved(egui::vec2(-3.0, 7.0))
        );
    }

    /// La fenêtre des tests de frame complète : 200 × 100, donc un axe à 100 (`AXIS`).
    const WINDOW: Rect = Rect {
        min: egui::pos2(0.0, 0.0),
        max: egui::pos2(200.0, 100.0),
    };

    /// Un switch de trois cases, déclaré comme un bloc — la disposition exacte qui a révélé le
    /// bug. Sa boîte va à la place de son reflet : `[10, 70]` devient `[130, 190]`, et la case
    /// « Dégâts » reste la PREMIÈRE, donc en `[130, 150]`.
    const SWITCH: Rect = Rect {
        min: egui::pos2(10.0, 10.0),
        max: egui::pos2(70.0, 30.0),
    };
    fn case(index: usize) -> Rect {
        let left = SWITCH.min.x + 20.0 * index as f32;
        Rect::from_min_max(
            egui::pos2(left, SWITCH.min.y),
            egui::pos2(left + 20.0, SWITCH.max.y),
        )
    }

    /// Peint une frame avec le switch ci-dessus (et, au besoin, une infobulle), miroir armé, et
    /// rend les formes telles qu'elles sortent du miroir.
    fn frame(ctx: &Context, tooltip: Option<Rect>) -> Vec<egui::epaint::ClippedShape> {
        let input = egui::RawInput {
            screen_rect: Some(WINDOW),
            ..Default::default()
        };
        let output = ctx.clone().run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            arm(&ctx, axis_of(&ctx));
            upright_in(ui, SWITCH, |ui| {
                for index in 0..3 {
                    ui.painter()
                        .rect_filled(case(index), 0, egui::Color32::GREEN);
                }
            });
            if let Some(tooltip) = tooltip {
                egui::Area::new(egui::Id::new("bulle"))
                    .order(egui::Order::Tooltip)
                    .fixed_pos(tooltip.min)
                    .show(&ctx, |ui| {
                        ui.painter().rect_filled(tooltip, 0, egui::Color32::YELLOW);
                    });
            }
            apply(&ctx);
        });
        let shapes = output.shapes.clone();
        output.drop_without_applying_deltas();
        shapes
    }

    /// **Le cœur du correctif** : survoler la case qu'on VOIT vise cette case-là.
    ///
    /// La case « Dégâts » est peinte en `[130, 150]` ; le pointeur posé en son milieu doit
    /// ressortir au milieu de la case « Dégâts » de la mise en page, `[10, 30]`. Une simple
    /// réflexion aurait donné 60 — la TROISIÈME case, celle que l'essai en jeu activait à la
    /// place de la première.
    #[test]
    fn le_pointeur_vise_la_case_qu_on_voit_pas_sa_symetrique() {
        let ctx = Context::default();
        frame(&ctx, None);
        let mut input = egui::RawInput {
            events: vec![egui::Event::PointerMoved(egui::pos2(140.0, 20.0))],
            ..Default::default()
        };
        mirror_input(&ctx, &mut input, AXIS);
        assert_eq!(
            input.events[0],
            egui::Event::PointerMoved(egui::pos2(20.0, 20.0)),
            "le milieu de la première case affichée doit viser la première case, pas la dernière"
        );

        // Hors du bloc, le décor : réfléchi, comme avant.
        let mut input = egui::RawInput {
            events: vec![egui::Event::PointerMoved(egui::pos2(195.0, 80.0))],
            ..Default::default()
        };
        mirror_input(&ctx, &mut input, AXIS);
        assert_eq!(
            input.events[0],
            egui::Event::PointerMoved(egui::pos2(5.0, 80.0))
        );
    }

    /// L'infobulle née d'un survol s'ouvre là où on survole : elle suit le bloc survolé, elle ne
    /// va pas à la place de son reflet — qui est à l'autre bout du panneau.
    #[test]
    fn l_infobulle_suit_le_bloc_survole() {
        let ctx = Context::default();
        // Première frame : elle laisse la carte des blocs, que la traduction du pointeur relit.
        frame(&ctx, None);
        let mut input = egui::RawInput {
            events: vec![egui::Event::PointerMoved(egui::pos2(140.0, 20.0))],
            ..Default::default()
        };
        mirror_input(&ctx, &mut input, AXIS);

        // egui pose la bulle sous la case de MISE EN PAGE ; c'est sous la case AFFICHÉE qu'elle
        // doit finir, donc déplacée comme le switch.
        let bulle = Rect::from_min_max(egui::pos2(12.0, 40.0), egui::pos2(42.0, 56.0));
        // Deux frames : egui consacre la première d'une zone nouvelle à la dimensionner.
        frame(&ctx, Some(bulle));
        let shapes = frame(&ctx, Some(bulle));
        // Reconnue à sa TAILLE, pas à sa couleur : une zone qui s'ouvre apparaît en fondu, donc
        // avec un jaune déjà atténué par l'opacité de l'animation.
        let peinte = shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                Shape::Rect(rect) if rect.rect.size() == bulle.size() => Some(rect.rect),
                _ => None,
            })
            .expect("la bulle n'a pas été peinte");
        assert_eq!(
            peinte,
            bulle.translate(egui::vec2(120.0, 0.0)),
            "la bulle doit suivre le déplacement du switch (+120), pas se réfléchir"
        );
    }
}
