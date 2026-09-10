"""Découpe et reconstruit le panneau de contenu de la modale Options de Wakfu.

Entrée : assets/design-system/interfaces/interface-options-*.png (720x561)
Sortie : assets/design-system/modal-section.png — 688x375, RGBA.

Ce que disent les captures
--------------------------
* Le panneau occupe x 16..703, y 124..498 dans la fenêtre, soit 688 x 375.
* Ce n'est PAS une surface à part : c'est le fond de la fenêtre assombri d'environ neuf
  niveaux (#1E2126 -> #16191C), que le décor de la fenêtre traverse. Les hachures s'y
  retrouvent, aux quatre angles, exactement comme sur le corps.
* Bordure de 2 px, presque noire (#15171A) — ce n'est pas un trait qu'on voit, c'est ce
  qui détache le panneau du fond de la modale.
* Angles arrondis d'un rayon d'environ 4 : l'escalier court sur quatre lignes.
* Le fond est encore plus plat que celui du corps : moins de deux niveaux d'écart d'un
  bord à l'autre.

Le contenu couvre presque tout le panneau — les marges libres se comptent en quelques
pixels — donc l'intérieur est entièrement rebâti, et non recollé depuis des morceaux de
capture. Ce qui est conservé de l'image d'origine : la bordure, les angles et leur
antialiasing.
"""
import argparse
import os
import sys

import numpy as np
from scipy.ndimage import gaussian_filter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import modal_capture as mc  # noqa: E402

# --- géométrie, mesurée au pixel sur les captures recalées -------------------
PANEL = (16, 124, 704, 499)          # bornes exclusives : 688 x 375
BORDER = 2                           # épaisseur du liseré
# fenêtres de fond nu, à l'intérieur du panneau, où aucun onglet ne pose de contenu
CLEAN = [(3, 3, 12, 371), (676, 3, 685, 371), (14, 3, 660, 7)]
SCROLLBAR = (668, 0, 688, 375)       # colonne réservée à la barre de défilement


def panel_masks(shape):
    """Bordure, intérieur et couronne de contour, dans le repère du panneau."""
    h, w = shape
    inner = np.zeros((h, w), bool)
    inner[BORDER:h - BORDER, BORDER:w - BORDER] = True
    edge = np.zeros((h, w), bool)
    edge[:BORDER + 3, :] = True
    edge[h - BORDER - 3:, :] = True
    edge[:, :BORDER + 3] = True
    edge[:, w - BORDER - 3:] = True
    return inner, edge


def observed_alpha(win, panel_rgb, modal_rgb, shape, n=10):
    """Alpha lu dans les quatre angles : le panneau y laisse voir le fond de la modale,
    et chaque pixel s'y lit comme un mélange des deux couleurs."""
    h, w = shape
    obs = np.ones((h, w))
    lo, hi = panel_rgb.mean(), modal_rgb.mean()
    for (y0, y1, x0, x1) in ((0, n, 0, n), (0, n, w - n, w),
                             (h - n, h, 0, n), (h - n, h, w - n, w)):
        z = win[y0:y1, x0:x1].mean(2)
        obs[y0:y1, x0:x1] = np.clip((hi - z) / max(hi - lo, 1e-6), 0, 1)
    return obs


def rounded_alpha(shape, radius, samples=4):
    """Masque d'un rectangle arrondi, antialiasé par suréchantillonnage.

    Le design system détoure au rectangle arrondi ajusté, jamais au contour brut : le
    résultat a des bords géométriques nets, et le même arrondi se retrouve à l'identique
    aux quatre angles.
    """
    h, w = shape
    ys = (np.arange(h * samples) + 0.5) / samples
    xs = (np.arange(w * samples) + 0.5) / samples
    yy, xx = np.meshgrid(ys, xs, indexing='ij')
    inside = np.ones((h * samples, w * samples), bool)
    for (cx, cy, sx, sy) in ((radius, radius, -1, -1), (w - radius, radius, 1, -1),
                             (radius, h - radius, -1, 1), (w - radius, h - radius, 1, 1)):
        corner = ((xx - cx) * sx > 0) & ((yy - cy) * sy > 0)
        inside &= ~(corner & (np.hypot(xx - cx, yy - cy) > radius))
    return inside.reshape(h, samples, w, samples).mean((1, 3))


def fit_radius(obs, shape, win=6):
    """Rayon déduit de l'AIRE manquante dans les angles, pas d'un ajustement pixel à
    pixel : un quart de disque de rayon r retire (1 - pi/4) * r^2 pixels au carré qui le
    contient. Mesurer une aire est robuste au bruit de fond, qu'un ajustement direct
    prendrait pour de la transparence.
    """
    h, w = shape
    corners = ((0, win, 0, win), (0, win, w - win, w),
               (h - win, h, 0, win), (h - win, h, w - win, w))
    # bruit de fond : ce que « 1 - alpha » vaut là où le panneau est plein
    floor = (1.0 - obs[h // 2 - 20:h // 2 + 20, w // 2 - 20:w // 2 + 20]).mean()
    aires = [max((1.0 - obs[y0:y1, x0:x1]).sum() - floor * win * win, 0.0)
             for (y0, y1, x0, x1) in corners]
    aire = float(np.mean(aires))
    r = np.sqrt(aire / (1.0 - np.pi / 4.0))
    return int(round(r)), aire


def decontaminate(rgb, alpha, modal_rgb, floor=0.2):
    """Retire du bord la couleur du fond qu'il a mélangée.

    Sans cette passe, l'asset porte un halo clair aux angles dès qu'on le pose sur autre
    chose que le fond de modale d'origine.
    """
    a = alpha[:, :, None]
    out = np.where(a >= floor,
                   (rgb - (1.0 - a) * modal_rgb[None, None, :]) / np.maximum(a, floor),
                   rgb)
    return np.clip(out, 0, 255)


def build(imgs, alpha_src, flat_border=False):
    x0, y0, x1, y1 = PANEL
    h, w = y1 - y0, x1 - x0
    wins = [g[y0:y1, x0:x1] for g in imgs]
    stack = np.stack(wins)
    mn = np.sort(stack, 0)[0]                 # le contenu est plus clair : le minimum l'efface
    med = np.median(stack, 0)

    inner, edge = panel_masks((h, w))
    clean = mc.rect_mask(CLEAN, (h, w)) & inner
    clean &= ~mc.rect_mask([SCROLLBAR], (h, w))

    # Couleur : mesurée sur la médiane (le minimum de six échantillons bruités
    # sous-estime le fond d'environ un niveau), et son très léger dégradé étendu par
    # diffusion depuis ces seules fenêtres.
    lf = mc.diffuse(mc.diffuse(med, clean, 9.0), clean, 200.0)

    # Hachures : relevées sur les six captures dans la zone du panneau. Elles y sont
    # rares — un millier de pixels, tous dans les angles — mais ce sont elles qui
    # raccordent le panneau au décor de la fenêtre.
    zone = np.ones((h, w), bool)
    hat = mc.hatching([g[y0:y1, x0:x1] for g in imgs], zone, edges=edge, seuil=0.25)
    amp = float(np.percentile(
        (mn.mean(2) - mc.diffuse(mn.mean(2), clean, 9.0))[clean], 90)) if clean.any() else 2.0
    amp = max(amp, 2.0)

    body = lf + mc.grain_field(1.35, (h, w), seed=11) + (hat * amp)[:, :, None] * mc.TINT
    body += (med[clean].reshape(-1, 3).mean(0) - body[clean].reshape(-1, 3).mean(0))

    # La bordure et les angles viennent de la capture : deux pixels de liseré et quatre
    # lignes d'escalier ne se resynthétisent pas mieux qu'ils ne se recopient.
    out = np.where(inner[:, :, None], body, med)
    if flat_border:
        out = body

    panel_rgb = med[clean].reshape(-1, 3).mean(0)
    modal_rgb = np.median(np.stack([g[y0 + 150:y0 + 250, x0 - 8:x0 - 3] for g in imgs]),
                          0).reshape(-1, 3).mean(0)

    obs = observed_alpha(med, panel_rgb, modal_rgb, (h, w))
    radius, err = fit_radius(obs, (h, w))
    alpha = rounded_alpha((h, w), radius)
    out = decontaminate(out, alpha, modal_rgb)
    return np.clip(out, 0, 255), alpha * 255.0, hat, panel_rgb, modal_rgb, radius, err


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('sortie', nargs='?', default='assets/design-system/modal-section.png')
    args = ap.parse_args()

    _, imgs, alpha_src = mc.load()
    rgb, alpha, hat, panel_rgb, modal_rgb, radius, err = build(imgs, alpha_src)

    print('panneau           : %d x %d' % (rgb.shape[1], rgb.shape[0]))
    print('couleur du fond   : #%02X%02X%02X' % tuple(np.round(panel_rgb).astype(int)))
    print('fond de la modale : #%02X%02X%02X (écart %.1f niveaux)'
          % (*np.round(modal_rgb).astype(int), (modal_rgb - panel_rgb).mean()))
    print('rayon des angles  : %d (aire manquante %.1f px par angle)' % (radius, err))
    print('hachures          : %d px de tracé' % (hat > 0.05).sum())
    print('marges du décor   : %s' % mc.measure_insets(hat))
    print('écrit :', mc.save_rgba(rgb, alpha, args.sortie))


if __name__ == '__main__':
    main()
