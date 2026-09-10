"""Découpe et reconstruit le fond (corps) de la modale Options de Wakfu.

Entrée  : assets/design-system/interfaces/interface-options-*.png (720x561)
Sortie  : modal-body*.png — 720x505 (y=56..560), contenu retiré.

Ce que disent les captures
--------------------------
* Le corps commence exactement à y=56, là où s'arrête `modal-header.png`.
* Fond anthracite bleuté quasi uni (#1D2126) : le dégradé d'un bord à l'autre
  ne dépasse pas ~2 niveaux.
* Grain anisotrope : autocorrélation 0.85 à 1 px verticalement, 0.37 puis
  négative horizontalement — de fines stries verticales, pas un bruit isotrope.
* Hachures : réseau de lignes à 45° (pente 1.000), dense sur le pourtour
  (bannière, côtés, bas, coins) et absent au centre — c'est un CADRE
  décoratif. Elles courent par-dessus les onglets et les boutons, ce qui
  permet de les relever même là où un composant les recouvre.
* Le panneau de section n'est pas opaque : c'est le même fond, assombri
  d'environ 9 niveaux (#1D2126 -> #15181B).

Composants retirés : ligne d'onglets, bouton « réinitialiser », panneau de
section, boutons Annuler et Valider.
"""
import numpy as np, glob, os, sys
from PIL import Image
from scipy.ndimage import (gaussian_filter, gaussian_filter1d, median_filter,
                           grey_opening, binary_erosion, binary_dilation, label)

SRC = 'assets/design-system/interfaces'
W, H = 720, 561
BODY_Y0 = 56          # première ligne sous la bannière
BORDER = 2            # liseré extérieur de la fenêtre

# --- géométrie du chrome, mesurée au pixel ----------------------------------
TABS      = (16,  70, 639, 119)
RESET     = (663, 70, 707, 119)
PANEL     = (14, 122, 706, 500)
BTN_LEFT  = (13, 512, 357, 553)
BTN_RIGHT = (363, 512, 707, 553)
HOLES = [TABS, RESET, PANEL, BTN_LEFT, BTN_RIGHT]

# fond de modale réellement visible (conservé tel quel en sortie)
KEEP = [(3,  57, 717,  68),
        (3, 503, 717, 510),
        (3, 556, 717, 559),
        (3,  57,  12, 559),
        (708, 57, 717, 559),
        (644, 76, 660, 113),
        (359, 516, 361, 549)]
# sous-ensemble servant à estimer le dégradé : on s'écarte de la bannière,
# dont le turquoise déborderait dans la moyenne locale
KEEP_LF = [(3,  64, 717,  68)] + KEEP[1:]


def rect_mask(rects):
    m = np.zeros((H, W), bool)
    for x0, y0, x1, y1 in rects:
        m[y0:y1, x0:x1] = True
    return m


def body_mask():
    m = np.zeros((H, W), bool)
    m[BODY_Y0:H - BORDER, BORDER:W - BORDER] = True
    return m


def diffuse(img, mask, sigma):
    """Champ lisse étendu depuis les seuls pixels connus."""
    den = gaussian_filter(mask.astype(float), sigma)
    if img.ndim == 2:
        return gaussian_filter(img * mask, sigma) / np.maximum(den, 1e-9)
    return np.dstack([gaussian_filter(img[:, :, c] * mask, sigma) / np.maximum(den, 1e-9)
                      for c in range(3)])


# --- hachures ---------------------------------------------------------------

def _diag_se(sign, L):
    se = np.zeros((L, L), bool)
    for k in range(L):
        se[k, k if sign > 0 else L - 1 - k] = True
    return se


def _component_edges(pad=5):
    """Couronne le long du contour des composants : leurs bords chanfreinés
    sont eux aussi à 45° et seraient pris pour des hachures."""
    inner = np.zeros((H, W), bool)
    outer = np.zeros((H, W), bool)
    for x0, y0, x1, y1 in HOLES:
        inner[y0 + pad:y1 - pad, x0 + pad:x1 - pad] = True
        outer[max(0, y0 - pad):y1 + pad, max(0, x0 - pad):x1 + pad] = True
    return outer & ~inner


def _diag_close(m, sign, gap):
    """Fermeture 1D le long des diagonales de pente `sign`, sans mélanger les
    diagonales voisines (ce que ferait une fermeture 2D)."""
    ys, xs = np.mgrid[0:H, 0:W]
    d = ys - sign * xs
    d -= d.min()
    grid = np.zeros((d.max() + 1, W), bool)
    grid[d.ravel(), xs.ravel()] = m.ravel()
    dil = np.zeros_like(grid)
    for k in range(-(gap // 2), gap // 2 + 1):
        dil |= np.roll(grid, k, axis=1)
    ero = np.ones_like(grid)
    for k in range(-(gap // 2), gap // 2 + 1):
        ero &= np.roll(dil, k, axis=1)
    return (grid | ero)[d.ravel(), xs.ravel()].reshape(H, W)


def directional(img, L=17):
    """Ne laisse passer que ce qui est aligné à ±45° : une ouverture par un
    segment oblique efface le texte, les cadres et les bords de widgets."""
    lum = img.mean(2)
    hf = np.clip(lum - median_filter(lum, 9), -3, 3)
    o = np.maximum(grey_opening(hf, footprint=_diag_se(1, L)),
                   grey_opening(hf, footprint=_diag_se(-1, L)))
    blob = np.clip(grey_opening(hf, footprint=np.ones((3, 3), bool)), 0, None)
    return np.clip(o - 0.5 * blob, 0, None)


def hatching_relief(imgs, keep, body):
    """Hachures relevées telles quelles : médiane des captures, normalisée sur
    l'amplitude qu'elles ont sur le fond nu (un trait ressort plus fort sur le
    jaune du bouton Valider que sur l'anthracite)."""
    med = np.median(np.stack([directional(g) for g in imgs]), 0)
    med *= body
    sel = keep & (med > 0.2)
    ref = np.percentile(med[sel], 75) if sel.any() else 1.0
    med = np.sqrt(np.clip(med, 0, None) / max(ref, 1e-6)) * ref   # compresse les excès
    return gaussian_filter(med, 0.5)


def hatching_drawn(imgs, body):
    """Hachures redessinées : le tracé est isolé puis rendu à trait net."""
    acc = np.zeros((H, W))
    votes = np.zeros((H, W), int)
    for g in imgs:
        v = directional(g)
        acc = np.maximum(acc, v)
        votes += (v > 0.25)
    m = (votes >= 2) & (acc > 0.3) & body
    m &= ~_component_edges()
    m &= ~binary_dilation(binary_erosion(m, np.ones((3, 3))), np.ones((7, 7)))
    lab, _ = label(m, np.ones((3, 3)))
    sizes = np.bincount(lab.ravel())
    ids = np.nonzero(sizes >= 6)[0]
    m = np.isin(lab, ids[ids != 0])
    closed = _diag_close(m, 1, 30) | _diag_close(m, -1, 30)
    closed &= ~binary_dilation(binary_erosion(closed, np.ones((3, 3))), np.ones((5, 5)))
    return gaussian_filter((closed & body).astype(float), 0.55)


def grain_field(sigma, seed=5):
    """Grain synthétique : légèrement étiré verticalement, comme le grain
    mesuré sur les marges (autocorrélation verticale plus forte qu'horizontale),
    mais sans les longues traînées qu'une corrélation trop marquée produirait."""
    rng = np.random.default_rng(seed)
    n = gaussian_filter1d(rng.normal(size=(H, W)), 0.9, axis=0)
    n = n - gaussian_filter1d(n, 0.9, axis=1)
    n *= sigma / n.std()
    out = np.dstack([n, n, n])
    out += np.dstack([gaussian_filter(rng.normal(size=(H, W)), 0.6) * sigma * 0.25
                      for _ in range(3)])
    return out


TINT = np.array([0.88, 0.97, 1.06])       # les hachures suivent le bleu du fond


def build(src_img, hatch, amp, flat=False, grain_seed=5):
    body = body_mask()
    keep = rect_mask(KEEP) & body

    if flat:
        lf = np.ones((H, W, 3)) * src_img[rect_mask(KEEP_LF)].reshape(-1, 3).mean(0)
    else:
        lf_mask = rect_mask(KEEP_LF) & body
        lf = diffuse(diffuse(src_img, lf_mask, 9.0), lf_mask, 150.0)

    synth = lf + grain_field(1.45, grain_seed) + (hatch * amp)[:, :, None] * TINT
    # recale la moyenne du fond synthétisé sur celle du fond réel
    synth += (src_img[keep].reshape(-1, 3).mean(0) - synth[keep].reshape(-1, 3).mean(0))

    if flat:
        out = synth.copy()
    else:
        # fondu asymétrique : la synthèse ne mord que de 2 px sur le fond conservé,
        # sans quoi les marges (9 px de large) perdraient leurs vraies hachures
        a = np.clip(gaussian_filter((~keep & body).astype(float), 1.3) * 1.8 - 0.4, 0, 1)
        a = a[:, :, None]
        out = src_img * (1 - a) + synth * a
    out[~body] = src_img[~body]           # liserés extérieurs et coins : intacts
    return np.clip(out, 0, 255)


def save(arr, alpha, outdir, name):
    """Sortie RGBA : les deux angles inférieurs sont arrondis au rayon 12, comme
    les angles supérieurs de `modal-header.png` — leur transparence fait partie
    de l'asset."""
    rgba = np.dstack([arr, alpha])[BODY_Y0:H].astype(np.uint8)
    Image.fromarray(rgba, 'RGBA').save(os.path.join(outdir, name), optimize=True)
    return name


if __name__ == '__main__':
    outdir = sys.argv[1]
    os.makedirs(outdir, exist_ok=True)
    files = sorted(glob.glob(os.path.join(SRC, 'interface-options-*.png')))
    raw = [np.asarray(Image.open(f).convert('RGBA')).astype(np.float64) for f in files]
    imgs = [r[:, :, :3] for r in raw]
    alpha = np.median(np.stack([r[:, :, 3] for r in raw]), 0)
    stack = np.stack(imgs)
    mn = np.sort(stack, 0)[0]                 # min-stack : le contenu clair s'efface
    # le minimum de 6 échantillons bruités sous-estime le fond d'environ 1
    # niveau : pour les zones de fond nu, on prend la médiane des captures
    med = np.median(stack, 0)

    body = body_mask()
    keep = rect_mask(KEEP) & body
    relief = hatching_relief(imgs, keep, body)
    drawn = hatching_drawn(imgs, body)

    hf = med.mean(2) - median_filter(med.mean(2), 9)
    amp_ref = float(np.percentile(hf[keep & (relief > 0.2)], 80)) if (keep & (relief > 0.2)).any() else 2.5
    print('amplitude de trait retenue : %.2f niveau' % amp_ref)

    for f, im in zip(files, imgs):
        tag = os.path.basename(f)[len('interface-options-'):-len('.png')]
        save(build(im, relief, 1.0), alpha, outdir, 'modal-body-%s.png' % tag)
    save(build(med, relief, 1.0), alpha, outdir, 'modal-body-consolide.png')
    save(build(med, drawn, amp_ref), alpha, outdir, 'modal-body-trace-net.png')
    save(build(med, relief, 1.0, flat=True), alpha, outdir, 'modal-body-plat.png')
    save(build(med, relief * 0.0, 1.0, flat=True), alpha, outdir, 'modal-body-uni.png')

    c = med[rect_mask(KEEP_LF)].reshape(-1, 3).mean(0)
    print('couleur de fond : #%02X%02X%02X' % tuple(np.round(c).astype(int)))
