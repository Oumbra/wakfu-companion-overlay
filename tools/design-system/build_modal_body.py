"""Découpe et reconstruit le corps de la modale Options de Wakfu.

Entrée : assets/design-system/interfaces/interface-options-*.png (720x561)
Sortie : assets/design-system/modal-body.png — 720x505 (y=56..560), RGBA.

Ce que disent les captures
--------------------------
* Le corps commence exactement à y=56, là où s'arrête `modal-header.png`.
* Fond anthracite bleuté quasi uni (#1E2126) : le dégradé d'un bord à l'autre ne dépasse
  pas ~2 niveaux.
* Hachures : réseau de lignes à 45° (pente 1,000), dense sur le pourtour (bannière,
  côtés, bas, coins) et absent au centre — c'est un CADRE décoratif, pas une trame. Elles
  courent par-dessus les onglets et les boutons, ce qui permet de les relever même là où
  un composant les recouvre.
* Le panneau de section n'est pas opaque : c'est le même fond assombri d'environ neuf
  niveaux (voir `build_modal_section.py`).

Composants retirés : ligne d'onglets, bouton « réinitialiser », panneau de section,
boutons Annuler et Valider.
"""
import argparse
import os
import sys

import numpy as np
from scipy.ndimage import gaussian_filter, median_filter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import modal_capture as mc  # noqa: E402

W, H = mc.W, mc.H
BODY_Y0 = 56          # première ligne sous la bannière
BORDER = 2            # liseré extérieur de la fenêtre

# --- géométrie du chrome, mesurée au pixel ----------------------------------
TABS = (16, 70, 639, 119)
RESET = (663, 70, 707, 119)
PANEL = (14, 122, 706, 500)
BTN_LEFT = (13, 512, 357, 553)
BTN_RIGHT = (363, 512, 707, 553)
HOLES = [TABS, RESET, PANEL, BTN_LEFT, BTN_RIGHT]

# fond de modale réellement visible (conservé tel quel en sortie)
KEEP = [(3, 57, 717, 68),
        (3, 503, 717, 510),
        (3, 556, 717, 559),
        (3, 57, 12, 559),
        (708, 57, 717, 559),
        (644, 76, 660, 113),
        (359, 516, 361, 549)]
# sous-ensemble servant à estimer le dégradé : on s'écarte de la bannière, dont le
# turquoise déborderait dans la moyenne locale
KEEP_LF = [(3, 64, 717, 68)] + KEEP[1:]


def body_mask():
    m = np.zeros((H, W), bool)
    m[BODY_Y0:H - BORDER, BORDER:W - BORDER] = True
    return m


def component_edges(pad=5):
    """Couronne le long du contour des composants : leurs bords chanfreinés sont eux
    aussi à 45° et seraient pris pour des hachures."""
    inner = np.zeros((H, W), bool)
    outer = np.zeros((H, W), bool)
    for x0, y0, x1, y1 in HOLES:
        inner[y0 + pad:y1 - pad, x0 + pad:x1 - pad] = True
        outer[max(0, y0 - pad):y1 + pad, max(0, x0 - pad):x1 + pad] = True
    return outer & ~inner


def build(src_img, hatch, amp, flat=False, grain_seed=5):
    body = body_mask()
    keep = mc.rect_mask(KEEP) & body

    if flat:
        lf = np.ones((H, W, 3)) * src_img[mc.rect_mask(KEEP_LF)].reshape(-1, 3).mean(0)
    else:
        lf_mask = mc.rect_mask(KEEP_LF) & body
        lf = mc.diffuse(mc.diffuse(src_img, lf_mask, 9.0), lf_mask, 150.0)

    synth = lf + mc.grain_field(1.45, (H, W), grain_seed) + (hatch * amp)[:, :, None] * mc.TINT
    # recale la moyenne du fond synthétisé sur celle du fond réel
    synth += (src_img[keep].reshape(-1, 3).mean(0) - synth[keep].reshape(-1, 3).mean(0))

    if flat:
        out = synth.copy()
    else:
        # fondu asymétrique : la synthèse ne mord que de 2 px sur le fond conservé, sans
        # quoi les marges (9 px de large) perdraient leurs vraies hachures
        a = np.clip(gaussian_filter((~keep & body).astype(float), 1.3) * 1.8 - 0.4, 0, 1)
        out = src_img * (1 - a[:, :, None]) + synth * a[:, :, None]
    out[~body] = src_img[~body]           # liserés extérieurs et angles : intacts
    return np.clip(out, 0, 255)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('sortie', nargs='?', default='assets/design-system/modal-body.png',
                    help="fichier de l'asset retenu (défaut : assets/design-system/modal-body.png)")
    ap.add_argument('--variantes', metavar='DOSSIER',
                    help="écrit aussi le lot d'arbitrage complet dans DOSSIER")
    args = ap.parse_args()

    tags, imgs, alpha = mc.load()
    stack = np.stack(imgs)
    # le minimum de six échantillons bruités sous-estime le fond d'environ un niveau :
    # pour les zones de fond nu, c'est la médiane des captures qui fait foi
    med = np.median(stack, 0)

    body = body_mask()
    keep = mc.rect_mask(KEEP) & body
    drawn = mc.hatching(imgs, body, edges=component_edges())

    hf = med.mean(2) - median_filter(med.mean(2), 9)
    sel = keep & (drawn > 0.05)
    amp = float(np.percentile(hf[sel], 80)) if sel.any() else 2.5
    print('amplitude de trait : %.2f niveau' % amp)
    print('marges du décor    : %s' % mc.measure_insets(drawn[BODY_Y0:]))
    c = med[mc.rect_mask(KEEP_LF)].reshape(-1, 3).mean(0)
    print('couleur de fond    : #%02X%02X%02X' % tuple(np.round(c).astype(int)))

    # Variante retenue par l'utilisateur le 2026-09-10 : hachures redessinées à trait
    # franc, interruptions comblées — le motif se lit sur le corps sans qu'il faille le
    # chercher.
    rgb = build(med, drawn, amp)[BODY_Y0:H]
    print('écrit :', mc.save_rgba(rgb, mc.clean_alpha(alpha)[BODY_Y0:H], args.sortie))

    if args.variantes:
        d = args.variantes
        relief = np.clip(np.median(np.stack([mc.directional(g) for g in imgs]), 0), 0, None)
        relief = gaussian_filter(relief * body, 0.5)
        ref = np.percentile(relief[keep & (relief > 0.2)], 75) if (keep & (relief > 0.2)).any() else 1.0
        relief = np.sqrt(np.clip(relief, 0, None) / max(ref, 1e-6)) * ref
        a = mc.clean_alpha(alpha)[BODY_Y0:H]
        for tag, im in zip(tags, imgs):
            mc.save_rgba(build(im, relief, 1.0)[BODY_Y0:H], a, os.path.join(d, 'modal-body-%s.png' % tag))
        mc.save_rgba(build(med, relief, 1.0)[BODY_Y0:H], a, os.path.join(d, 'modal-body-consolide.png'))
        mc.save_rgba(build(med, drawn, amp)[BODY_Y0:H], a, os.path.join(d, 'modal-body-trace-net.png'))
        mc.save_rgba(build(med, relief, 1.0, flat=True)[BODY_Y0:H], a, os.path.join(d, 'modal-body-plat.png'))
        mc.save_rgba(build(med, relief * 0.0, 1.0, flat=True)[BODY_Y0:H], a, os.path.join(d, 'modal-body-uni.png'))
        print("lot d'arbitrage écrit dans", d)


if __name__ == '__main__':
    main()
