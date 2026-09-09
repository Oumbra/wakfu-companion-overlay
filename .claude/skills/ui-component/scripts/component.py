#!/usr/bin/env python3
"""Outil du skill `ui-component` : prepare une texture de design system pour un
composant egui redimensionnable.

    component.py insets  IMG [--tol 24] [--safety 2]
    component.py preview IMG --insets l,t,r,b --sizes 120x32,340x32 [-o sheet.html]

`insets` mesure les marges 9-slice a figer (coins + liseré) ; `preview` rend la
texture a plusieurs tailles cibles AVANT d'ecrire la moindre ligne de Rust, pour
que le reglage se valide sur une planche et pas dans le compilateur.

Reutilise `dslib` du skill `design-asset` (meme pipeline d'image, jamais un
second portage des memes primitives).
"""
from __future__ import annotations

import argparse
import base64
import io
import json
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(
    0,
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "design-asset", "scripts"),
)
from dslib.core import bbox_of, load_rgba, save_rgba  # noqa: E402
from dslib.scale9 import scale9  # noqa: E402

ALPHA = 128


def trim(rgba: np.ndarray):
    """Rogne la marge transparente laissee par le detourage. Renvoie (image, bbox)."""
    box = bbox_of(rgba[:, :, 3] >= ALPHA)
    if box is None:
        raise SystemExit("image entierement transparente")
    x0, y0, x1, y1 = box
    return rgba[y0:y1, x0:x1], [int(x0), int(y0), int(x1), int(y1)]


def corner_radius(alpha: np.ndarray) -> int:
    """Rayon de l'arrondi : plus grand decalage du premier pixel opaque sur la
    premiere ligne / colonne, mesure aux quatre coins."""
    op = alpha >= ALPHA
    h, w = op.shape
    r = 0
    for row, col in ((0, 0), (0, w - 1), (h - 1, 0), (h - 1, w - 1)):
        line = op[row]
        idx = np.flatnonzero(line)
        if idx.size:
            r = max(r, int(idx[0]) if col == 0 else int(w - 1 - idx[-1]))
    return r


def border_width(rgba: np.ndarray) -> int:
    """Epaisseur du liseré sombre : nombre de lignes, depuis le bord haut au
    milieu du composant, dont la luminance reste sous la moitie de celle du corps."""
    h = rgba.shape[0]
    mid = rgba.shape[1] // 2
    col = rgba[:, mid, :3].astype(np.float32).mean(axis=1)
    body = float(np.median(col[h // 4 : 3 * h // 4]))
    thr = body * 0.6
    n = 0
    while n < h // 3 and col[n] < thr:
        n += 1
    return n


def stable_span(rgba: np.ndarray, tol: float) -> tuple[int, int]:
    """Colonnes reproductibles : celles qui, comparees a la colonne mediane du
    centre, restent sous `tol`. Donne la zone horizontale reellement etirable
    (le corps texture), par opposition aux extremites (coins + liseré)."""
    w = rgba.shape[1]
    core = rgba[:, w // 4 : 3 * w // 4, :].astype(np.float32)
    ref = np.median(core, axis=1)  # profil vertical de reference (H x 4)
    diff = np.abs(rgba.astype(np.float32) - ref[:, None, :]).max(axis=2)
    like = np.percentile(diff, 95, axis=0) <= tol
    c = w // 2
    left = c
    while left > 0 and like[left - 1]:
        left -= 1
    right = c
    while right < w - 1 and like[right + 1]:
        right += 1
    return left, w - 1 - right


def decor_span(rgba: np.ndarray, border: int, quiet_run: int = 10, k: float = 0.25):
    """Etendue du DECOR d'extremite, en pixels depuis chaque bord.

    Sur les composants Wakfu, les hachures diagonales ne couvrent pas le fond :
    ce sont des embouts, cantonnes aux premieres dizaines de pixels de chaque
    cote, le centre restant un degrade lisse. C'est cette etendue — et non le
    rayon des coins — qui dimensionne les marges 9-slice horizontales : une
    marge plus courte laisse un bout de motif dans la bande mediane, ou il se
    retrouve etire (ou repete) sur toute la longueur du composant.

    Mesure : ecart de chaque colonne au degrade vertical de reference (mediane
    par ligne). Le decor s'arrete la ou cet ecart retombe au niveau du bruit
    central pendant `quiet_run` colonnes d'affilee.

    ATTENTION : sans objet sur une texture qui porte encore un libelle incruste
    — le libelle, au centre, fait exploser le niveau de reference et la mesure
    rend 0. Toujours mesurer sur un asset deja generifie (skill `design-asset`).
    """
    a = rgba.astype(np.float32)
    h, w = a.shape[:2]
    ref = np.median(a[:, :, :3], axis=1, keepdims=True)
    dev = np.abs(a[:, :, :3] - ref).mean(axis=2)
    col = dev[border + 3 : h - border - 3, :].mean(axis=0)
    inner = col[border : w - border]
    if inner.size < 8:
        return 0, 0, 0.0, 0.0
    lo, hi = int(inner.size * 0.35), int(inner.size * 0.65)
    base = float(np.percentile(inner[lo:hi], 50))
    peak = float(np.percentile(inner, 97))
    threshold = base + k * (peak - base)

    def extent(seq):
        quiet = 0
        for i, value in enumerate(seq):
            if value <= threshold:
                quiet += 1
                if quiet >= quiet_run:
                    return i - quiet + 1
            else:
                quiet = 0
        return len(seq)

    return border + extent(inner), border + extent(inner[::-1]), base, peak


def cmd_insets(a):
    src = load_rgba(a.img)
    img, box = trim(src)
    h, w = img.shape[:2]
    radius = corner_radius(img[:, :, 3])
    border = border_width(img)
    geometric = max(radius, border) + a.safety
    left, right = stable_span(img, a.tol)
    d_left, d_right, base, peak = decor_span(img, max(border, 1) + 1)
    half_w, half_h = (w - 1) // 2, (h - 1) // 2

    # Horizontal : le decor commande. Vertical : la geometrie seule — le decor des
    # composants Wakfu est lateral, et moins on fige, mieux le degrade suit la hauteur.
    inset_x = min(max(geometric, d_left, d_right), half_w)
    inset_y = min(geometric, half_h)
    print(
        json.dumps(
            {
                "source": a.img,
                "source_size": [int(src.shape[1]), int(src.shape[0])],
                "trim_bbox": box,
                "trimmed_size": [int(w), int(h)],
                "corner_radius": radius,
                "border_width": border,
                "stable_span": {"left": int(left), "right": int(right)},
                "decor_span": {
                    "left": int(d_left),
                    "right": int(d_right),
                    "noise_level": round(base, 2),
                    "peak": round(peak, 2),
                },
                "geometric_minimum": int(geometric),
                "recommended_insets": {
                    "left": int(inset_x),
                    "top": int(inset_y),
                    "right": int(inset_x),
                    "bottom": int(inset_y),
                },
                "recommended_fill": {"x": "stretch", "y": "stretch"},
            },
            ensure_ascii=False,
            indent=2,
        )
    )


def _b64(arr: np.ndarray) -> str:
    buf = io.BytesIO()
    Image.fromarray(arr, "RGBA").save(buf, "PNG")
    return base64.b64encode(buf.getvalue()).decode()


def cmd_preview(a):
    src = load_rgba(a.img)
    img, _ = trim(src)
    insets = tuple(int(v) for v in a.insets.split(","))
    sizes = []
    for s in a.sizes.split(","):
        ws, hs = s.lower().split("x")
        sizes.append((int(ws), int(hs)))
    name = os.path.basename(a.img)
    rows = []
    for w, h in sizes:
        nine = scale9(img, w, h, insets, a.fill)
        naive = np.asarray(
            Image.fromarray(img, "RGBA").resize((w, h), Image.BILINEAR), dtype=np.uint8
        )
        rows.append(
            "<tr><th>{w}&times;{h}</th><td><img src='data:image/png;base64,{a}'></td>"
            "<td class='naive'><img src='data:image/png;base64,{b}'></td></tr>".format(
                w=w, h=h, a=_b64(nine), b=_b64(naive)
            )
        )
        if a.out_dir:
            save_rgba(nine, os.path.join(a.out_dir, f"{os.path.splitext(name)[0]}-{w}x{h}.png"))
    html = """<title>9-slice — {name}</title>
<style>
 body{{font:14px system-ui,sans-serif;background:#15171c;color:#e8e8ea;margin:0;padding:24px}}
 h1{{font-size:16px;margin:0 0 4px}} p{{color:#9aa0a6;margin:0 0 20px}}
 table{{border-collapse:collapse}} td,th{{padding:10px 14px;vertical-align:middle}}
 th{{color:#9aa0a6;font-weight:500;text-align:right;font-variant-numeric:tabular-nums}}
 thead th{{text-align:center;color:#f4d89f}}
 tbody tr:nth-child(odd){{background:#1c1f26}}
 td.naive{{opacity:.95;border-left:1px solid #2a2e37}}
 img{{display:block;image-rendering:pixelated}}
</style>
<h1>{name} — insets {ins}, remplissage « {fill} »</h1>
<p>Colonne de gauche : 9-slice (coins et liseré figés). Colonne de droite : simple
étirement bilinéaire, pour comparaison — c'est ce qu'on veut éviter.</p>
<table><thead><tr><th>taille</th><th>9-slice</th><th>étirement naïf</th></tr></thead>
<tbody>{rows}</tbody></table>""".format(
        name=name, ins=a.insets, fill=a.fill, rows="".join(rows)
    )
    out = a.out or "preview.html"
    with open(out, "w", encoding="utf-8") as f:
        f.write(html)
    print(json.dumps({"written": out, "sizes": [list(s) for s in sizes]}, ensure_ascii=False))


def main():
    p = argparse.ArgumentParser(description=__doc__)
    sub = p.add_subparsers(dest="cmd", required=True)

    q = sub.add_parser("insets", help="mesure les marges 9-slice a figer (JSON)")
    q.add_argument("img")
    q.add_argument("--tol", type=float, default=24.0)
    q.add_argument("--safety", type=int, default=2)
    q.set_defaults(func=cmd_insets)

    q = sub.add_parser("preview", help="planche HTML du 9-slice a plusieurs tailles")
    q.add_argument("img")
    q.add_argument("--insets", required=True, help="l,t,r,b")
    q.add_argument("--sizes", required=True, help="ex. 120x32,340x32,200x64")
    q.add_argument("--fill", default="stretch", choices=["stretch", "tile", "mirror"])
    q.add_argument("-o", "--out")
    q.add_argument("--out-dir")
    q.set_defaults(func=cmd_preview)

    a = p.parse_args()
    a.func(a)


if __name__ == "__main__":
    main()
