"""Trace une ébauche SVG de chaque icône PNG du design system.

Entrée : assets/design-system/icons/*.png — glyphes blanc pur + alpha, 7 à 28 px de côté.
Sortie : assets/design-system/icons-svg/<même nom>.svg — un seul `<path fill="currentColor">`,
viewBox aux dimensions natives du PNG, coordonnées en pixels du PNG à deux décimales.

Ce que fait le traçage
----------------------
* Seul le canal alpha compte : les 38 icônes sont blanches (une seule couleur RGB sous alpha,
  sauf `icon-search` qui porte des nuances de gris) — la silhouette est entièrement dans l'alpha.
* L'alpha est suréchantillonné ×16 en bicubique avant seuillage, sinon potrace trace des
  marches d'escalier d'un pixel et les arrondit n'importe comment. Le lissage bicubique donne
  des angles légèrement arrondis : c'est le principal défaut des ébauches, à corriger à la main.
* Le seuil (`--thr`, 0,5 par défaut) décide où passe le contour dans les pixels
  semi-transparents. Un trait d'un pixel à 40 % d'alpha disparaît à 0,5 : `characters`, `minus`
  et `order` sont meilleurs à 0,35 (mesuré par intersection sur union contre le PNG re-rasterisé,
  session du 2026-09-11). Les glyphes dont plus de 60 % de l'encre est semi-transparente
  (`characters`, `eye`, `grid`, `info`, `search`) restent sous 0,85 quel que soit le seuil :
  ils sont à redessiner, pas à retracer.
* potracer (portage Python de potrace) trace les pixels à `True` comme du NOIR — mais son
  orientation de remplissage est inversée par rapport à ce que rend un navigateur ou resvg avec
  la règle `nonzero` : sans inverser le masque, le SVG obtenu peint le FOND et non l'encre.
  D'où le `~mask`.

Dépendances : `pip install pillow numpy potracer`.

Usage : `python tools/design-system/vectorize_icons.py [--thr 0.5] [--only icon-eye ...]`
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
# Seuils retenus par icône quand 0,5 n'est pas le meilleur (voir docstring).
THRESHOLDS = {"icon-characters": 0.35, "icon-minus": 0.35, "icon-order": 0.35}


def fmt(v: float) -> str:
    s = f"{v:.2f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def trace(png: str, thr: float) -> str:
    im = Image.open(png).convert("RGBA")
    w, h = im.size
    alpha = im.getchannel("A").resize((w * UPSCALE, h * UPSCALE), Image.BICUBIC)
    mask = np.array(alpha) >= int(thr * 255)
    bitmap = potrace.Bitmap(~mask)
    path = bitmap.trace(
        turdsize=UPSCALE * UPSCALE // 4,  # ignore les taches de moins d'un quart de pixel
        turnpolicy=potrace.POTRACE_TURNPOLICY_MINORITY,
        alphamax=1.0,
        opticurve=True,
        opttolerance=0.2,
    )
    d = []
    for curve in path:
        start = curve.start_point
        d.append(f"M{fmt(start.x / UPSCALE)} {fmt(start.y / UPSCALE)}")
        for seg in curve.segments:
            end = seg.end_point
            if seg.is_corner:
                c = seg.c
                d.append(
                    f"L{fmt(c.x / UPSCALE)} {fmt(c.y / UPSCALE)}"
                    f"L{fmt(end.x / UPSCALE)} {fmt(end.y / UPSCALE)}"
                )
            else:
                c1, c2 = seg.c1, seg.c2
                d.append(
                    f"C{fmt(c1.x / UPSCALE)} {fmt(c1.y / UPSCALE)} "
                    f"{fmt(c2.x / UPSCALE)} {fmt(c2.y / UPSCALE)} "
                    f"{fmt(end.x / UPSCALE)} {fmt(end.y / UPSCALE)}"
                )
        d.append("Z")
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}">'
        f'<path fill="currentColor" d="{"".join(d)}"/></svg>\n'
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--thr", type=float, default=None,
                        help="seuil d'alpha (0-1) imposé à toutes les icônes ; sinon 0,5 ou la valeur retenue par icône")
    parser.add_argument("--only", nargs="*", default=None, help="noms d'icônes (sans extension) à traiter")
    parser.add_argument("--out", default=DST, help="dossier de sortie")
    args = parser.parse_args()

    os.makedirs(args.out, exist_ok=True)
    for png in sorted(glob.glob(os.path.join(SRC, "*.png"))):
        name = os.path.splitext(os.path.basename(png))[0]
        if args.only and name not in args.only:
            continue
        thr = args.thr if args.thr is not None else THRESHOLDS.get(name, 0.5)
        svg = trace(png, thr)
        out = os.path.join(args.out, name + ".svg")
        with open(out, "w", encoding="utf-8") as f:
            f.write(svg)
        print(f"{name:24} seuil {thr:.2f}  {os.path.getsize(png):5} o PNG -> {len(svg.encode()):5} o SVG")


if __name__ == "__main__":
    main()
