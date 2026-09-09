//! Peinture **9-slice** d'une texture du design system : rendre une image de taille native
//! `W×H` dans un rectangle de taille quelconque **sans déformer ses coins, son arrondi ni son
//! liseré**.
//!
//! C'est la primitive qui rend les composants réellement paramétrables en taille. Sans elle, la
//! seule façon d'obtenir un bouton de 340px de large à partir d'une texture de 200px est de
//! fabriquer un PNG de plus — ce qui a effectivement été fait au départ
//! (`assets/design-system/large-button-cancel.png` = `button-danger.png` à la taille du pied de
//! page de la modale Options, libellé « Annuler » compris) et ce que cette couche remplace : **une
//! texture générique par variante, toutes les tailles au rendu**.
//!
//! Découpage : la texture est coupée en 9 zones par les `Insets`. Les 4 coins sont recopiés à
//! l'identique, les 4 bords sont étendus sur un seul axe, le centre sur les deux. C'est le même
//! algorithme que `dslib.scale9` du skill `design-asset` (qui, lui, produit un PNG hors ligne) —
//! ici en `egui::Mesh`, donc sans allocation d'image ni retéléversement GPU quand la taille change.
//!
//! **Les marges ne sont pas seulement géométriques : elles délimitent le décor.** Sur les boutons
//! du jeu, les hachures diagonales ne couvrent pas le fond — ce sont des **embouts décoratifs**,
//! cantonnés aux ~50 premiers pixels de chaque extrémité (~30 pour la texture du pied de page de
//! modale, mesuré par `component.py insets`), le centre restant un dégradé lisse. Les marges
//! figées sont donc dimensionnées sur l'étendue du décor, pas sur le seul rayon des coins : c'est
//! ce qui fait qu'un bouton de 500px garde exactement deux embouts, comme un bouton de 200px, au
//! lieu d'un motif répété ou étiré sur toute sa longueur (retour utilisateur 2026-09-09).
//!
//! **Le mode de remplissage reste réglable par axe** : `Fill::Stretch` pour un contenu continu
//! (le dégradé vertical, et la bande centrale lisse des boutons), `Fill::Tile` pour un motif
//! périodique qu'il faudrait répéter à l'échelle native. Aucune texture du manifeste n'utilise
//! `Tile` depuis que le décor des boutons est reconnu comme un embout — la répétition reste
//! disponible pour une future bande décorative réellement continue.

use egui::{Color32, Mesh, Painter, Rect, Shape, TextureHandle, Vec2};

/// Marges figées d'une texture 9-slice, **en pixels de la texture source**. Deux exigences, la
/// seconde souvent bien plus contraignante que la première (toutes deux mesurées par
/// `.claude/skills/ui-component/scripts/component.py insets`) :
///
/// 1. couvrir le rayon d'arrondi + l'épaisseur du liseré — en dessous, le découpage coupe dans le
///    coin et la bande étendue recopie un morceau d'arrondi ;
/// 2. couvrir **toute l'étendue du décor** de l'extrémité (`decor_span`) — en dessous, le motif
///    déborde dans la bande médiane et se retrouve étiré ou répété sur toute la longueur.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Insets {
    pub const fn same(v: f32) -> Self {
        Self {
            left: v,
            top: v,
            right: v,
            bottom: v,
        }
    }
}

/// Comment étendre une bande médiane sur un axe — voir la doc de module pour le choix par axe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    /// Étirement continu : le bon choix pour un dégradé (l'axe vertical des boutons).
    Stretch,
    /// Répétition à l'échelle native : le bon choix pour une texture périodique (les hachures, sur
    /// l'axe horizontal). La dernière répétition est rognée, jamais compressée.
    Tile,
}

/// Description 9-slice complète d'une texture : ce qui est figé, et comment le reste s'étend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NineSlice {
    pub insets: Insets,
    pub fill_x: Fill,
    pub fill_y: Fill,
}

impl NineSlice {
    pub const fn new(insets: Insets, fill_x: Fill, fill_y: Fill) -> Self {
        Self {
            insets,
            fill_x,
            fill_y,
        }
    }
}

/// Une bande sur un axe : longueurs destination successives et intervalle de coordonnées de
/// texture (normalisé) correspondant à chacune.
struct Band {
    /// `(début, fin)` en pixels destination, relatif au bord du rectangle.
    dst: Vec<(f32, f32)>,
    /// `(u0, u1)` normalisé, un par entrée de `dst`.
    uv: Vec<(f32, f32)>,
}

/// Découpe un axe en trois zones (marge basse figée, bande médiane étendue, marge haute figée).
/// `lo`/`hi` sont les marges destination (déjà réduites si le rectangle est trop petit, voir
/// `paint`), `lo_src`/`hi_src` les marges source correspondantes.
fn split_axis(
    dst_len: f32,
    tex_len: f32,
    lo: f32,
    hi: f32,
    lo_src: f32,
    hi_src: f32,
    fill: Fill,
) -> Band {
    let mut dst = Vec::with_capacity(4);
    let mut uv = Vec::with_capacity(4);

    if lo > 0.0 {
        dst.push((0.0, lo));
        uv.push((0.0, lo_src / tex_len));
    }

    let mid_dst = (dst_len - lo - hi).max(0.0);
    let mid_src = (tex_len - lo_src - hi_src).max(1.0);
    let u_mid0 = lo_src / tex_len;
    let u_mid1 = (tex_len - hi_src) / tex_len;
    if mid_dst > 0.0 {
        match fill {
            Fill::Stretch => {
                dst.push((lo, lo + mid_dst));
                uv.push((u_mid0, u_mid1));
            }
            Fill::Tile => {
                // Répétitions entières à l'échelle native, puis un reste rogné côté texture — le
                // motif garde sa taille d'origine, ce que `Stretch` ne peut pas faire.
                let mut done = 0.0;
                while mid_dst - done > 0.5 {
                    let take = mid_src.min(mid_dst - done);
                    dst.push((lo + done, lo + done + take));
                    let ratio = take / mid_src;
                    uv.push((u_mid0, u_mid0 + (u_mid1 - u_mid0) * ratio));
                    done += take;
                }
            }
        }
    }

    if hi > 0.0 {
        dst.push((dst_len - hi, dst_len));
        uv.push(((tex_len - hi_src) / tex_len, 1.0));
    }

    Band { dst, uv }
}

/// Peint `texture` dans `rect` selon `slice`, teintée par `tint` (`Color32::WHITE` = couleurs
/// d'origine ; un blanc à alpha réduit atténue sans changer la teinte — c'est ainsi qu'est rendu
/// un état désactivé sans texture dédiée).
///
/// **Rectangle plus petit que ses propres marges** : les marges sont **rognées par l'intérieur**,
/// jamais compressées — on garde les `n` premiers pixels de la texture, on abandonne le reste de la
/// marge, et la bande médiane disparaît. Les coins, le liseré et le début du décor restent donc à
/// l'échelle 1:1 même sur un bouton plus étroit que ses deux embouts réunis ; c'est l'inverse d'un
/// redimensionnement, qui écraserait justement ce qu'on cherche à préserver. Un bouton de 8px de
/// haut n'a plus ni arrondi ni liseré complets, mais il est peint — pas de panique, pas de trou
/// noir à l'écran, et l'anomalie se voit sur la capture.
pub fn paint(
    painter: &Painter,
    rect: Rect,
    texture: &TextureHandle,
    slice: &NineSlice,
    tint: Color32,
) {
    let tex: Vec2 = texture.size_vec2();
    if tex.x < 1.0 || tex.y < 1.0 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }

    // Marges effectives, utilisées à la fois côté source et côté destination : c'est ce qui garantit
    // le 1:1. Bornées deux fois — par la texture (une marge de 6px sur une texture de 10px de haut
    // ne laisserait aucune bande médiane) puis par le rectangle cible (voir la doc).
    let effective = |lo: f32, hi: f32, tex_len: f32, dst_len: f32| -> (f32, f32) {
        let mut total = lo + hi;
        if total <= 0.0 {
            return (0.0, 0.0);
        }
        let mut k: f32 = 1.0;
        let max_src = (tex_len - 1.0).max(0.0);
        if total > max_src {
            k = k.min(max_src / total);
        }
        if total * k > dst_len {
            k = k.min(dst_len / total);
        }
        total *= k;
        debug_assert!(total <= dst_len + 0.01);
        (lo * k, hi * k)
    };
    let (l, r) = effective(slice.insets.left, slice.insets.right, tex.x, rect.width());
    let (t, b) = effective(slice.insets.top, slice.insets.bottom, tex.y, rect.height());

    let cols = split_axis(rect.width(), tex.x, l, r, l, r, slice.fill_x);
    let rows = split_axis(rect.height(), tex.y, t, b, t, b, slice.fill_y);

    let mut mesh = Mesh::with_texture(texture.id());
    for (ry, (y0, y1)) in rows.dst.iter().enumerate() {
        let (v0, v1) = rows.uv[ry];
        for (rx, (x0, x1)) in cols.dst.iter().enumerate() {
            let (u0, u1) = cols.uv[rx];
            let dst = Rect::from_min_max(
                egui::pos2(rect.min.x + x0, rect.min.y + y0),
                egui::pos2(rect.min.x + x1, rect.min.y + y1),
            );
            let uv = Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));
            mesh.add_rect_with_uv(dst, uv, tint);
        }
    }
    painter.add(Shape::mesh(mesh));
}
