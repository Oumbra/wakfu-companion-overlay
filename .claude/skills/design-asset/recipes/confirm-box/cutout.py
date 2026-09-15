"""Detourage de `interface-confirm-box.png`.

Le `cutout` generique du skill ajuste un **rectangle arrondi** sur la composante detectee :
il rend le corps de la boite et laisse le decor emporter la crete (medaillon, volutes,
filet dore) qui deborde au-dessus, et l'ornement de pied qui deborde en dessous. Ici le
masque est donc compose de trois morceaux mesures au pixel :

  * le corps        rectangle arrondi [13,45] -> [433,193], r = 2
  * la crete        y < 45, ornement beige/dore isole du decor par sa chrominance
  * le pied         y >= 193, meme critere

La decontamination des bords (`bleed_edges`) et l'antialiasing des coins viennent du
pipeline du skill : seul le masque est fait main.
"""
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, ".claude/skills/design-asset/scripts")
from dslib.core import connected_components  # noqa: E402
from dslib.segment import bleed_edges, rounded_rect_alpha  # noqa: E402

SRC = "assets/design-system/interfaces/interface-confirm-box.png"

# Mesures pixel (profils de lignes et de colonnes, voir le rapport de session).
PANEL = (13, 45, 433, 193)   # x0, y0, x1, y1 — bord exterieur du corps
PANEL_RADIUS = 2


def warm_mask(rgb):
    """Les ornements sont beige/dore ; tout le decor autour est gris-bleu ou vert.

    Le critere porte sur la chrominance et non sur la luminance : le rocher clair
    derriere les volutes est plus lumineux qu'elles, mais jamais plus rouge que vert.
    """
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    return (r > g + 3) & (r - b > 22)


def dilate(m, r=1):
    out = m.copy()
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            out |= np.roll(np.roll(m, dy, 0), dx, 1)
    return out


def ornament(warm, band, anchor, min_area=12):
    """Garde les composantes chaudes de `band` qui rejoignent le corps.

    Ce qui reste de chaud dans le decor, c'est le monstre derriere le medaillon : il
    s'arrete bien avant le filet, la ou tous les ornements reels le touchent.
    """
    m = warm & band
    lab, n = connected_components(m)
    keep = np.zeros_like(m)
    for i in range(1, n + 1):
        comp = lab == i
        ys = np.nonzero(comp.any(axis=1))[0]
        near = ys.max() >= anchor if band[0].any() else ys.min() <= anchor
        if near and comp.sum() >= min_area:
            keep |= comp
    # Le cerne sombre du glyphe n'est pas chaud : une dilatation le reintegre. Pas de
    # remplissage des trous — le contre-poincon du « ? » et l'oeil de la volute laissent
    # voir le decor, c'est du vide et non du composant.
    return dilate(keep, 1) & band


def main(out_path):
    rgba = np.asarray(Image.open(SRC).convert("RGBA")).astype(np.float64)
    rgb = rgba[..., :3].astype(int)
    h, w = rgb.shape[:2]

    x0, y0, x1, y1 = PANEL
    body = rounded_rect_alpha(w, h, x0, y0, x1, y1, PANEL_RADIUS)

    warm = warm_mask(rgb)
    above = np.zeros((h, w), bool)
    above[:y0, :] = True
    below = np.zeros((h, w), bool)
    below[y1:, :] = True

    crest = ornament(warm, above, anchor=y0 - 6)
    foot = ornament(warm, below, anchor=y1 + 8)
    # Le liseré noir qui coiffe le filet de pied n'est pas chaud : il est sous le corps,
    # entre lui et le filet, donc tout ce qui est encadre verticalement par du pied.
    foot |= np.cumsum(foot, axis=0).astype(bool) & np.cumsum(foot[::-1], axis=0)[::-1].astype(bool)
    foot &= below

    alpha = np.clip(body + crest.astype(np.float64) + foot.astype(np.float64), 0.0, 1.0)

    solid = alpha >= 0.999
    out = rgba.copy()
    out[..., 3] = alpha * 255.0
    out = bleed_edges(out, solid, passes=3, keep=solid)
    out[..., 3] = alpha * 255.0

    ys, xs = np.nonzero(alpha > 0.004)
    bx0, by0, bx1, by1 = xs.min(), ys.min(), xs.max() + 1, ys.max() + 1
    crop = out[by0:by1, bx0:bx1]
    Image.fromarray(np.clip(crop, 0, 255).astype(np.uint8)).save(out_path)
    print({
        "output": out_path,
        "bbox": [int(bx0), int(by0), int(bx1), int(by1)],
        "size": [int(bx1 - bx0), int(by1 - by0)],
        "panel_in_crop": [x0 - int(bx0), y0 - int(by0), x1 - int(bx0), y1 - int(by0)],
        "crest_px": int(crest.sum()),
        "foot_px": int(foot.sum()),
    })


if __name__ == "__main__":
    main(sys.argv[1])
