"""Bouton de fermeture d'une fenêtre — extraction du glyphe et mesure du socle.

Sources : `assets/design-system/window-close.png` et `window-close-hover.png`, deux recadrages
48 × 48 du bouton tel que le jeu le peint dans la bannière d'une fenêtre (captures utilisateur du
2026-09-13, échelle 1 — la bannière y fait 56 px, comme `modal-header.png`). Le bouton, un carré
de 32 px, y occupe (8, 8)-(40, 40) : les 8 px de bannière nue autour servent de référence de
luminosité, ce qui évite de la chercher dans une autre capture. Le bouton est posé SUR la
bannière — un carré arrondi translucide qui laisse voir les hachures, et une croix dorée. Ces deux
fichiers ne sont donc pas des textures (le 9-slice n'aurait rien à figer d'un carré dont le fond
est celui de la fenêtre), mais la référence que ce script mesure et que la galerie de contrôle
compare au rendu egui.

Ce que le script produit :

- `assets/design-system/icons/icon-close-window.png` : la croix, blanche avec alpha, démêlée du
  fond par le canal rouge — celui qui sépare le mieux l'or (`R = 244`) du sarcelle de la bannière
  (`R ≈ 16`). `dsimg.py icon` rejette la frange d'antialiasing de cette croix (alpha ≈ 0,12) et
  perd une rangée : la chaîne maison est plus fidèle sur un glyphe de 12 px.
- Sur la sortie standard, les valeurs qui vont dans `design/tokens.rs` (`WINDOW_CLOSE_*`) : le
  rapport d'assombrissement du carré dans chaque état, mesuré hors glyphe et hors liseré, celui du
  liseré au repos, et la teinte de la croix.

    python tools/design-system/build_window_close.py
"""
import os
import sys

import numpy as np
from PIL import Image

SRC = 'assets/design-system'
OUT = os.path.join(SRC, 'icons', 'icon-close-window.png')

# Le carré de 32 px dans le recadrage de 48.
BUTTON = (8, 8, 40, 40)
# Boîte du glyphe, frange d'antialiasing comprise (12 × 12, centrée dans le carré) — la boîte à
# seuil 150 fait 10 × 10, mais les pixels à R ≈ 44 qui l'entourent sont de l'encre à 12 %, pas
# du fond.
GLYPH = (18, 18, 30, 30)
# Pic de la croix, identique dans les deux états : `#F4D89F`, la teinte de survol des boutons
# icône et la teinte de repos du pas numérique.
GOLD = np.array([244, 216, 159], dtype=np.float64)


def load(name):
    return np.asarray(Image.open(os.path.join(SRC, name)).convert('RGB')).astype(np.float64)


def box_mask(shape, box, inset=0):
    m = np.zeros(shape, bool)
    x0, y0, x1, y1 = box
    m[y0 + inset : y1 - inset, x0 + inset : x1 - inset] = True
    return m


def unmix(img, background_r):
    """Alpha du glyphe par le canal rouge : `R = bg + a * (244 - bg)`."""
    x0, y0, x1, y1 = GLYPH
    r = img[y0:y1, x0:x1, 0]
    return np.clip((r - background_r) / (GOLD[0] - background_r), 0.0, 1.0)


def main():
    rest = load('window-close.png')
    hover = load('window-close-hover.png')
    shape = rest.shape[:2]

    # Trois zones : la bannière nue (le cadre de 8 px, moins 1 px contre le carré pour ne pas
    # attraper son antialiasing), l'intérieur du carré (3 px de liseré et le glyphe exclus), et
    # le liseré lui-même (la première rangée de pixels du carré, coins exclus).
    banner = ~box_mask(shape, BUTTON, inset=-1)
    inner = box_mask(shape, BUTTON, inset=3) & ~box_mask(shape, GLYPH, inset=-1)
    rim = box_mask(shape, BUTTON, inset=0) & ~box_mask(shape, BUTTON, inset=1)
    rim &= ~(box_mask(shape, BUTTON, 0) & ~box_mask(shape, (BUTTON[0] + 5, BUTTON[1], BUTTON[2] - 5, BUTTON[3]))
             & ~box_mask(shape, (BUTTON[0], BUTTON[1] + 5, BUTTON[2], BUTTON[3] - 5)))

    alphas = []
    for name, img in (('repos', rest), ('survol', hover)):
        ref = img[banner].mean(axis=0)
        fill = (img[inner].mean(axis=0) / ref).mean()
        edge = (img[rim].mean(axis=0) / ref).mean()
        print(f'{name:7s} : bannière nue RGB {ref.round(1)} ; intérieur / bannière = {fill:.3f} '
              f'(noir à alpha {1 - fill:.3f}) ; liseré / bannière = {edge:.3f} (noir à alpha {1 - edge:.3f})')
        alphas.append(unmix(img, float(np.median(img[inner][:, 0]))))

    print('écart repos/survol sur l\'alpha du glyphe :', round(float(np.abs(alphas[0] - alphas[1]).max()), 3))
    alpha = np.mean(alphas, axis=0)
    glyph = np.zeros((alpha.shape[0], alpha.shape[1], 4), np.uint8)
    glyph[..., :3] = 255
    glyph[..., 3] = np.round(alpha * 255).astype(np.uint8)
    Image.fromarray(glyph, 'RGBA').save(OUT)
    print('glyphe :', OUT, glyph.shape[1], 'x', glyph.shape[0])

    x0, y0, x1, y1 = GLYPH
    print('pic RGB du glyphe (repos) :', rest[y0:y1, x0:x1][alpha > 0.95].max(axis=0))


if __name__ == '__main__':
    sys.stdout.reconfigure(encoding='utf-8')
    main()
