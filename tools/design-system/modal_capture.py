"""Outillage commun aux textures de la modale Options découpées des captures du jeu.

Les six captures `assets/design-system/interfaces/interface-options-*.png` montrent la
même fenêtre avec six contenus différents. C'est ce qui permet d'en retirer le contenu
sans rien inventer : ce qui change d'une capture à l'autre est du contenu, ce qui ne
change pas est le chrome.

Ce module porte les briques partagées par `build_modal_body.py` (le corps) et
`build_modal_section.py` (le panneau de contenu). Les constantes de géométrie, elles,
restent dans chaque script : elles ne se ressemblent pas.
"""
import glob
import os

import numpy as np
from PIL import Image
from scipy.ndimage import (binary_dilation, binary_erosion, gaussian_filter,
                           gaussian_filter1d, grey_opening, label, median_filter)

SRC = 'assets/design-system/interfaces'
W, H = 720, 561

# Les six captures ne sont pas cadrées au même pixel : mesuré par corrélation sur le
# chrome (les bords du panneau, invariants d'un onglet à l'autre), résidu nul après
# recalage. Sans lui, les bordures de 2 px se dédoublent dans les empilements.
SHIFTS = {'chat': (1, 0), 'son': (0, -1), 'video': (0, -1)}
# Capture de référence du §9 du design-system : c'est d'elle que vient l'alpha, la
# médiane des six déchiquetant les escaliers d'angle.
REFERENCE = 'jeu'


def tag_of(path):
    return os.path.basename(path)[len('interface-options-'):-len('.png')]


def load():
    """Charge les six captures recalées. Rend (tags, images RGB, alpha de référence)."""
    files = sorted(glob.glob(os.path.join(SRC, 'interface-options-*.png')))
    tags, imgs, alpha = [], [], None
    for f in files:
        tag = tag_of(f)
        raw = np.asarray(Image.open(f).convert('RGBA')).astype(np.float64)
        dy, dx = SHIFTS.get(tag, (0, 0))
        raw = np.roll(np.roll(raw, dy, axis=0), dx, axis=1)
        tags.append(tag)
        imgs.append(raw[:, :, :3])
        if tag == REFERENCE:
            alpha = raw[:, :, 3]
    return tags, imgs, alpha


def rect_mask(rects, shape=(H, W)):
    m = np.zeros(shape, bool)
    for x0, y0, x1, y1 in rects:
        m[y0:y1, x0:x1] = True
    return m


def diffuse(img, mask, sigma):
    """Champ lisse étendu depuis les seuls pixels connus."""
    den = gaussian_filter(mask.astype(float), sigma)
    if img.ndim == 2:
        return gaussian_filter(img * mask, sigma) / np.maximum(den, 1e-9)
    return np.dstack([gaussian_filter(img[:, :, c] * mask, sigma) / np.maximum(den, 1e-9)
                      for c in range(img.shape[2])])


# --- hachures ---------------------------------------------------------------
#
# Le décor de la fenêtre Options est un réseau de lignes fines à 45° (pente 1,000,
# vérifiée par projection). Ce n'est pas une trame de fond mais un CADRE : dense dans
# les angles et le long des bords, absent du centre. Il court par-dessus tout ce qui est
# posé dans la fenêtre — onglets, boutons, panneau de contenu — ce qui permet de le
# relever même là où un composant le recouvre.

def diag_se(sign, length):
    se = np.zeros((length, length), bool)
    for k in range(length):
        se[k, k if sign > 0 else length - 1 - k] = True
    return se


def directional(img, length=17):
    """Ne laisse passer que ce qui est aligné à ±45°.

    Une ouverture morphologique par un segment oblique efface le texte, les cadres et
    les bords de widgets, et garde les hachures.
    """
    lum = img.mean(2)
    hf = np.clip(lum - median_filter(lum, 9), -3, 3)
    o = np.maximum(grey_opening(hf, footprint=diag_se(1, length)),
                   grey_opening(hf, footprint=diag_se(-1, length)))
    blob = np.clip(grey_opening(hf, footprint=np.ones((3, 3), bool)), 0, None)
    return np.clip(o - 0.5 * blob, 0, None)


def diag_close(m, sign, gap):
    """Fermeture 1D le long des diagonales de pente `sign`, sans mélanger les
    diagonales voisines (ce que ferait une fermeture 2D)."""
    h, w = m.shape
    ys, xs = np.mgrid[0:h, 0:w]
    d = ys - sign * xs
    d -= d.min()
    grid = np.zeros((d.max() + 1, w), bool)
    grid[d.ravel(), xs.ravel()] = m.ravel()
    dil = np.zeros_like(grid)
    for k in range(-(gap // 2), gap // 2 + 1):
        dil |= np.roll(grid, k, axis=1)
    ero = np.ones_like(grid)
    for k in range(-(gap // 2), gap // 2 + 1):
        ero &= np.roll(dil, k, axis=1)
    return (grid | ero)[d.ravel(), xs.ravel()].reshape(h, w)


def hatching(imgs, zone, edges=None, votes_min=2, seuil=0.3, gap=30):
    """Tracé des hachures, relevé sur toutes les captures puis rendu à trait net.

    `zone` borne le résultat ; `edges` retire une couronne le long des contours de
    composants, dont les bords chanfreinés sont eux aussi à 45° et passeraient pour des
    hachures.
    """
    acc = np.zeros(zone.shape)
    votes = np.zeros(zone.shape, int)
    for g in imgs:
        v = directional(g)
        acc = np.maximum(acc, v)
        votes += (v > 0.25)
    m = (votes >= votes_min) & (acc > seuil) & zone
    if edges is not None:
        m &= ~edges
    m &= ~binary_dilation(binary_erosion(m, np.ones((3, 3))), np.ones((7, 7)))
    lab, _ = label(m, np.ones((3, 3)))
    sizes = np.bincount(lab.ravel())
    ids = np.nonzero(sizes >= 6)[0]
    m = np.isin(lab, ids[ids != 0])
    closed = diag_close(m, 1, gap) | diag_close(m, -1, gap)
    # la fermeture empâte les croisements : on retire à nouveau ce qui dépasse
    # l'épaisseur d'un trait
    closed &= ~binary_dilation(binary_erosion(closed, np.ones((3, 3))), np.ones((5, 5)))
    return gaussian_filter((closed & zone).astype(float), 0.55)


def measure_insets(support, share=0.90, step=5):
    """Marges à figer en 9-slice : de chaque côté, la plus petite marge qui contienne
    `share` du décor de sa moitié.

    Même intention que les marges de bouton du manifeste (`design::assets`) — ce sont les
    hachures qui dimensionnent, pas les coins.
    """
    m = support > 0.05
    out = {}
    for axis, lo_name, hi_name in ((1, 'left', 'right'), (0, 'top', 'bottom')):
        prof = m.sum(axis=1 - axis).astype(float)
        n = len(prof)
        half = n // 2
        lo_total, hi_total = prof[:half].sum(), prof[half:].sum()
        cum, cum_r = np.cumsum(prof), np.cumsum(prof[::-1])
        lo = next((i + 1 for i in range(half) if cum[i] >= lo_total * share), half)
        hi = next((i + 1 for i in range(half) if cum_r[i] >= hi_total * share), half)
        out[lo_name] = int(np.ceil(lo / step) * step)
        out[hi_name] = int(np.ceil(hi / step) * step)
    return out


def grain_field(sigma, shape, seed=5):
    """Grain synthétique : légèrement étiré verticalement, comme le grain mesuré sur les
    marges (autocorrélation verticale plus forte qu'horizontale), mais sans les longues
    traînées qu'une corrélation trop marquée produirait.

    Resynthétisé plutôt que recopié : recoller des morceaux de marge laissait un damier
    visible aux jointures.
    """
    rng = np.random.default_rng(seed)
    n = gaussian_filter1d(rng.normal(size=shape), 0.9, axis=0)
    n = n - gaussian_filter1d(n, 0.9, axis=1)
    n *= sigma / n.std()
    out = np.dstack([n, n, n])
    out += np.dstack([gaussian_filter(rng.normal(size=shape), 0.6) * sigma * 0.25
                      for _ in range(3)])
    return out


# Les hachures suivent le bleu du fond plutôt que d'être neutres.
TINT = np.array([0.88, 0.97, 1.06])


def clean_alpha(alpha):
    """Efface les pixels semi-transparents isolés des angles.

    La capture d'origine en porte quelques-uns, détachés du bord par au moins un pixel
    transparent : de l'antialiasing du client resté accroché au détourage, pas la courbe
    elle-même. Peints tels quels, ils apparaissent en points clairs dans l'arrondi.
    """
    a = alpha.copy()
    for y in range(a.shape[0]):
        row = a[y]
        opaque = np.nonzero(row >= 128)[0]
        if len(opaque) == 0:
            continue
        lo, hi = opaque[0], opaque[-1]
        left, right = row[:lo], row[hi + 1:]
        if len(left) and (left == 0).any():
            row[:np.nonzero(left == 0)[0][-1] + 1] = 0
        if len(right) and (right == 0).any():
            row[hi + 1 + np.nonzero(right == 0)[0][0]:] = 0
    return a


def save_rgba(rgb, alpha, path):
    os.makedirs(os.path.dirname(path) or '.', exist_ok=True)
    rgba = np.dstack([np.clip(rgb, 0, 255), np.clip(alpha, 0, 255)]).astype(np.uint8)
    Image.fromarray(rgba, 'RGBA').save(path, optimize=True)
    return path
