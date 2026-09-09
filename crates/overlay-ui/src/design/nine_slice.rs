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
//! **Le mode de remplissage est par axe, et ce n'est pas un détail.** Les textures de bouton du jeu
//! portent deux motifs de natures différentes : un **dégradé vertical** (clair en haut, sombre en
//! bas) qui doit s'étirer avec la hauteur (`Fill::Stretch`), et des **hachures diagonales**
//! horizontalement périodiques qui doivent garder leur échelle quand le bouton s'allonge
//! (`Fill::Tile`). Un étirement horizontal transforme les croisillons en longues traînées —
//! visible dès 200 → 340px, vérifié sur planche (skill `ui-component`, commande `preview`) avant
//! d'écrire ce module.

use egui::{Color32, Mesh, Painter, Rect, Shape, TextureHandle, Vec2};

/// Marges figées d'une texture 9-slice, **en pixels de la texture source**. Doivent couvrir au
/// moins le rayon d'arrondi + l'épaisseur du liseré (mesurés par
/// `.claude/skills/ui-component/scripts/component.py insets`) : en dessous, le découpage coupe
/// dans le coin et la bande étendue recopie un morceau d'arrondi.
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
/// **Rectangle plus petit que ses propres marges** : les marges sont réduites proportionnellement
/// plutôt que de laisser le découpage produire des zones de longueur négative. Un bouton de 8px de
/// haut n'a plus ni arrondi ni liseré corrects, mais il est peint — pas de panique, pas de trou
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

    // Marges source : bornées par la texture elle-même (une inset de 6px sur une texture de 10px
    // de haut ne laisserait aucune bande médiane).
    let cap_src = |lo: f32, hi: f32, len: f32| -> (f32, f32) {
        let total = lo + hi;
        let max = (len - 1.0).max(0.0);
        if total > max && total > 0.0 {
            let k = max / total;
            (lo * k, hi * k)
        } else {
            (lo, hi)
        }
    };
    let (l_src, r_src) = cap_src(slice.insets.left, slice.insets.right, tex.x);
    let (t_src, b_src) = cap_src(slice.insets.top, slice.insets.bottom, tex.y);

    // Marges destination : identiques aux marges source (les coins sont peints à l'échelle 1:1),
    // sauf si le rectangle est trop petit — voir la doc.
    let cap_dst = |lo: f32, hi: f32, len: f32| -> (f32, f32) {
        let total = lo + hi;
        if total > len && total > 0.0 {
            let k = len / total;
            (lo * k, hi * k)
        } else {
            (lo, hi)
        }
    };
    let (l_dst, r_dst) = cap_dst(l_src, r_src, rect.width());
    let (t_dst, b_dst) = cap_dst(t_src, b_src, rect.height());

    let cols = split_axis(
        rect.width(),
        tex.x,
        l_dst,
        r_dst,
        l_src,
        r_src,
        slice.fill_x,
    );
    let rows = split_axis(
        rect.height(),
        tex.y,
        t_dst,
        b_dst,
        t_src,
        b_src,
        slice.fill_y,
    );

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
