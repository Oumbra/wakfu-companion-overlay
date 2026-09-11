"""Trace une ébauche SVG de chaque icône PNG du design system, en conservant son modelé.

Entrée : assets/design-system/icons/*.png — glyphes blanc pur + alpha, 7 à 28 px de côté.
Sortie : assets/design-system/icons-svg/<même nom>.svg — un groupe `fill="currentColor"` de
`<path>` empilés, viewBox aux dimensions natives du PNG, coordonnées en pixels du PNG.

Deux modes
----------
* **Multi-niveaux** (défaut, `--levels 10`) : l'alpha est tracé à N seuils croissants et les N
  contours sont empilés du plus large au plus étroit, chacun avec une `fill-opacity` calibrée
  pour que l'empilement reproduise l'alpha moyen de chaque bande, mesuré sur les pixels natifs.
  C'est ce qui conserve les ombres portées, les silhouettes fantômes (`characters`), les
  arrondis d'antialiasing : retour du mainteneur (2026-09-11), « c'est ce qui rend l'icône
  jolie ». Agrandi ×4, le rendu est à 5,7/255 d'un agrandissement Lanczos du PNG, là où la
  silhouette seule en est à 19,9. Contrepartie : à taille native, un glyphe à bords francs
  ressort un peu plus doux (12/255 d'écart moyen contre 7,5), et le fichier pèse 6 à 32 Ko.
* **Silhouette** (`--levels 1`) : un seul contour au seuil 0,5, opaque. Exact à taille native
  sur les bords francs, mais bouche tout le modelé : `bag-out` perd ses flèches, `xp` devient
  un pâté. Retenu pour la seule `icon-minus` (barre de 14 × 2 dont le multi-niveaux amincit
  le trait : 30/255 d'écart agrandi, contre 8,4 en silhouette).

Ce que fait le traçage
----------------------
* Seul le canal alpha compte : les 38 icônes sont blanches (une seule couleur RGB sous alpha,
  sauf `icon-search` qui porte des nuances de gris).
* L'alpha est suréchantillonné ×16 en Lanczos avant seuillage, après ajout d'une marge
  transparente d'un pixel (un contour qui touche le bord serait coupé). Lanczos et bicubique
  donnent un modelé lisse ; le bilinéaire dessine des bandes en losange, à proscrire.
* Les réglages de potrace (`alphamax`, `opttolerance`) ne changent rien de mesurable sur une
  image suréchantillonnée ×16 ; ils sont laissés à leurs valeurs usuelles. potracer (portage
  Python) plante parfois dans l'optimisation des courbes (`math domain error`) : on la
  désactive alors pour ce contour.
* potracer trace les pixels à `True` comme du NOIR, mais avec une orientation de remplissage
  inversée par rapport à ce que rendent un navigateur ou resvg : sans inverser le masque, le SVG
  peint le FOND et non l'encre. D'où le `~mask`.

Dépendances : `pip install pillow numpy potracer`.

Usage : `python tools/design-system/vectorize_icons.py [--levels 10] [--prec 1] [--only icon-eye ...]`
"""
import argparse
import glob
import os

import numpy as np
import potrace
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
SRC = os.path.join(ROOT, "assets", "design-system", "icons")
DST = os.path.join(ROOT, "assets", "design-system", "icons-svg")

UPSCALE = 16
PAD = 1
# Icônes pour lesquelles le nombre de niveaux par défaut n'est pas le bon (voir docstring).
LEVELS_OVERRIDE = {"icon-minus": 1}


def fmt(v: float, prec: int) -> str:
    s = f"{v:.{prec}f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def path_d(mask: np.ndarray, prec: int) -> str:
    bitmap = potrace.Bitmap(~mask)
    common = dict(turdsize=UPSCALE * UPSCALE // 8, turnpolicy=potrace.POTRACE_TURNPOLICY_MINORITY, alphamax=1.0)
    try:
        path = bitmap.trace(opticurve=True, opttolerance=0.5, **common)
    except ValueError:
        path = bitmap.trace(opticurve=False, **common)
    d = []
    for curve in path:
        start = curve.start_point
        d.append(f"M{fmt(start.x / UPSCALE, prec)} {fmt(start.y / UPSCALE, prec)}")
        for seg in curve.segments:
            end = seg.end_point
            if seg.is_corner:
                c = seg.c
                d.append(
                    f"L{fmt(c.x / UPSCALE, prec)} {fmt(c.y / UPSCALE, prec)}"
                    f"L{fmt(end.x / UPSCALE, prec)} {fmt(end.y / UPSCALE, prec)}"
                )
            else:
                c1, c2 = seg.c1, seg.c2
                d.append(
                    f"C{fmt(c1.x / UPSCALE, prec)} {fmt(c1.y / UPSCALE, prec)} "
                    f"{fmt(c2.x / UPSCALE, prec)} {fmt(c2.y / UPSCALE, prec)} "
                    f"{fmt(end.x / UPSCALE, prec)} {fmt(end.y / UPSCALE, prec)}"
                )
        d.append("Z")
    return "".join(d)


def trace(png: str, levels: int, prec: int) -> str:
    im = Image.open(png).convert("RGBA")
    w, h = im.size
    alpha_native = np.array(im.getchannel("A")).astype(float) / 255
    padded = Image.new("L", (w + 2 * PAD, h + 2 * PAD), 0)
    padded.paste(im.getchannel("A"), (PAD, PAD))
    alpha_up = padded.resize(((w + 2 * PAD) * UPSCALE, (h + 2 * PAD) * UPSCALE), Image.LANCZOS)
    alpha_up = np.clip(np.array(alpha_up).astype(float) / 255, 0, 1)

    if levels == 1:
        thresholds, targets = [0.5], [1.0]
    else:
        thresholds = [(i - 0.5) / levels for i in range(1, levels + 1)]
        targets = []
        for i, t in enumerate(thresholds):
            hi = thresholds[i + 1] if i + 1 < len(thresholds) else 1.01
            band = alpha_native[(alpha_native >= t) & (alpha_native < hi)]
            targets.append(float(band.mean()) if len(band) else min(1.0, (t + hi) / 2))

    paths = []
    below = 0.0  # alpha composite atteint par les contours déjà empilés
    for t, target in zip(thresholds, targets):
        mask = alpha_up >= t
        if not mask.any():
            below = target
            continue
        opacity = 1.0 if below >= 0.999 else min(1.0, max(0.0, 1 - (1 - target) / (1 - below)))
        below = target
        d = path_d(mask, prec)
        if not d:
            continue
        attr = "" if opacity >= 0.995 else f' fill-opacity="{opacity:.3f}"'
        paths.append(f'<path{attr} d="{d}"/>')

    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}">'
        f'<g fill="currentColor" transform="translate(-{PAD} -{PAD})">{"".join(paths)}</g></svg>\n'
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--levels", type=int, default=None,
                        help="nombre de seuils d'alpha empilés (défaut 10 ; 1 = silhouette seule)")
    parser.add_argument("--prec", type=int, default=1, help="décimales des coordonnées (défaut 1)")
    parser.add_argument("--only", nargs="*", default=None, help="noms d'icônes (sans extension) à traiter")
    parser.add_argument("--out", default=DST, help="dossier de sortie")
    args = parser.parse_args()

    os.makedirs(args.out, exist_ok=True)
    for png in sorted(glob.glob(os.path.join(SRC, "*.png"))):
        name = os.path.splitext(os.path.basename(png))[0]
        if args.only and name not in args.only:
            continue
        levels = args.levels if args.levels is not None else LEVELS_OVERRIDE.get(name, 10)
        svg = trace(png, levels, args.prec)
        out = os.path.join(args.out, name + ".svg")
        with open(out, "w", encoding="utf-8") as f:
            f.write(svg)
        print(f"{name:24} {levels:2} niveaux  {os.path.getsize(png):5} o PNG -> {len(svg.encode()):6} o SVG")


if __name__ == "__main__":
    main()
