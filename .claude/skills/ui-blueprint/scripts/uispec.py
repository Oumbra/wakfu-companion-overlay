#!/usr/bin/env python
"""uispec — retro-ingenierie d'une capture d'interface vers une specification mesuree.

    python uispec.py <commande> --help

`layout`, `lines`, `palette` et `edges` mesurent ; `blueprint` dessine. Le partage des
roles est volontaire : la machine mesure ce qui est mesurable (positions, gouttieres,
hauteurs de ligne, couleurs), le modele nomme les blocs et redige la spec.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np

_DSLIB = Path(__file__).resolve().parents[2] / "design-asset" / "scripts"
if _DSLIB.exists():
    sys.path.insert(0, str(_DSLIB))

from dslib.core import load_rgba, luma, rgb_of  # noqa: E402


def _emit(d):
    print(json.dumps(d, ensure_ascii=False, indent=2))


def _box(s):
    if not s:
        return None
    v = [int(x) for x in s.replace(" ", "").split(",")]
    if len(v) != 4:
        raise argparse.ArgumentTypeError("format attendu : x0,y0,x1,y1")
    return tuple(v)


# --------------------------------------------------------------------- energie

def energy(rgb: np.ndarray) -> np.ndarray:
    """Carte d'activite : norme du gradient. Le fond uni d'un panneau est a ~0,
    le texte, les bordures et les icones ressortent."""
    lum = luma(rgb)
    gy = np.zeros_like(lum)
    gx = np.zeros_like(lum)
    gy[1:, :] = np.abs(lum[1:, :] - lum[:-1, :])
    gx[:, 1:] = np.abs(lum[:, 1:] - lum[:, :-1])
    return gy + gx


def _valleys(profile: np.ndarray, threshold: float, min_len: int):
    """Intervalles ou le profil reste sous le seuil (gouttieres)."""
    out = []
    start = None
    for i, v in enumerate(profile):
        if v <= threshold:
            if start is None:
                start = i
        else:
            if start is not None and i - start >= min_len:
                out.append((start, i))
            start = None
    if start is not None and len(profile) - start >= min_len:
        out.append((start, len(profile)))
    return out


# ------------------------------------------------------- decoupage XY recursif

def xy_cut(e: np.ndarray, box, depth: int, min_gap: int, quantile: float,
           min_size: int, axis_first: str = "y"):
    """Decoupage XY recursif (analyse de mise en page) : on coupe la region dans les
    bandes de faible activite, alternativement en lignes et en colonnes."""
    x0, y0, x1, y1 = box
    node = {"box": [int(x0), int(y0), int(x1), int(y1)],
            "size": [int(x1 - x0), int(y1 - y0)], "children": []}
    if depth <= 0 or (x1 - x0) < min_size or (y1 - y0) < min_size:
        return node
    sub = e[y0:y1, x0:x1]
    if sub.size == 0:
        return node

    order = ("y", "x") if axis_first == "y" else ("x", "y")
    for axis in order:
        prof = sub.mean(axis=1 if axis == "y" else 0)
        if prof.max() <= 0:
            continue
        thr = max(float(np.quantile(prof, quantile)), prof.max() * 0.06)
        gaps = _valleys(prof, thr, min_gap)
        # gouttieres internes seulement : celles qui touchent un bord ne coupent rien
        inner = [g for g in gaps if g[0] > 0 and g[1] < len(prof)]
        if not inner:
            continue
        cuts = []
        prev = 0
        for a, b in inner:
            cuts.append((prev, a))
            prev = b
        cuts.append((prev, len(prof)))
        cuts = [c for c in cuts if c[1] - c[0] >= 2]
        if len(cuts) < 2:
            continue
        for a, b in cuts:
            child_box = ((x0, y0 + a, x1, y0 + b) if axis == "y"
                         else (x0 + a, y0, x0 + b, y1))
            child = xy_cut(e, child_box, depth - 1, min_gap, quantile, min_size,
                           "x" if axis == "y" else "y")
            child["axis"] = axis
            node["children"].append(child)
        node["split"] = axis
        node["gaps"] = [{"from": int(a + (y0 if axis == "y" else x0)),
                         "to": int(b + (y0 if axis == "y" else x0)),
                         "size": int(b - a)} for a, b in inner]
        return node
    return node


def tighten(e: np.ndarray, box, threshold_ratio: float = 0.08):
    """Retracte une boite sur son contenu reel (retire les marges vides)."""
    x0, y0, x1, y1 = box
    sub = e[y0:y1, x0:x1]
    if sub.size == 0:
        return box
    thr = max(sub.max() * threshold_ratio, 1.0)
    rows = np.nonzero(sub.mean(axis=1) > thr)[0]
    cols = np.nonzero(sub.mean(axis=0) > thr)[0]
    if len(rows) == 0 or len(cols) == 0:
        return box
    return (x0 + int(cols[0]), y0 + int(rows[0]),
            x0 + int(cols[-1]) + 1, y0 + int(rows[-1]) + 1)


def cmd_layout(a):
    rgba = load_rgba(a.image)
    rgb = rgb_of(rgba)
    e = energy(rgb)
    h, w = e.shape
    box = _box(a.box) or (0, 0, w, h)
    tree = xy_cut(e, box, a.depth, a.min_gap, a.quantile, a.min_size)

    def annotate(node):
        node["content_box"] = list(tighten(e, node["box"]))
        cb, bb = node["content_box"], node["box"]
        node["padding"] = {"left": cb[0] - bb[0], "top": cb[1] - bb[1],
                           "right": bb[2] - cb[2], "bottom": bb[3] - cb[3]}
        for c in node["children"]:
            annotate(c)
    annotate(tree)
    if a.json:
        _emit({"file": str(a.image), "image_size": [w, h], "tree": tree})
        return

    # Rendu compact par defaut : un arbre JSON complet coute beaucoup de contexte pour
    # une information qui tient en une ligne par bloc.
    lines = ["%s  %dx%d" % (a.image, w, h)]

    def walk(node, depth):
        bb, pd = node["box"], node["padding"]
        axis = node.get("axis", "")
        gap = ""
        if node.get("gaps"):
            gap = "  gaps=" + ",".join(str(g["size"]) for g in node["gaps"])
        lines.append("%s%s %d,%d %dx%d  pad %d,%d,%d,%d%s" % (
            "  " * depth, axis or "-", bb[0], bb[1], bb[2] - bb[0], bb[3] - bb[1],
            pd["left"], pd["top"], pd["right"], pd["bottom"], gap))
        for c in node["children"]:
            walk(c, depth + 1)
    walk(tree, 0)
    print("\n".join(lines))


# ------------------------------------------------------------ lignes de texte

def cmd_lines(a):
    rgba = load_rgba(a.image)
    rgb = rgb_of(rgba)
    e = energy(rgb)
    h, w = e.shape
    x0, y0, x1, y1 = _box(a.box) or (0, 0, w, h)
    sub = e[y0:y1, x0:x1]
    prof = sub.mean(axis=1)
    thr = max(float(np.quantile(prof, 0.35)), prof.max() * 0.10)
    bands, start = [], None
    for i, v in enumerate(prof):
        if v > thr and start is None:
            start = i
        elif v <= thr and start is not None:
            if i - start >= a.min_height:
                bands.append((start, i))
            start = None
    if start is not None:
        bands.append((start, len(prof)))

    lum = luma(rgb)
    out = []
    for s, t in bands:
        band = slice(y0 + s, y0 + t)
        cols = np.nonzero(e[band, x0:x1].mean(axis=0) > thr * 0.6)[0]
        item = {"y": [int(y0 + s), int(y0 + t)], "height": int(t - s),
                "x": [int(x0 + cols[0]), int(x0 + cols[-1]) + 1] if len(cols) else None}
        strong = e[band, x0:x1] > thr
        if strong.any():
            item["ink_luma"] = round(float(lum[band, x0:x1][strong].mean()), 1)
        out.append(item)
    gaps = [int(out[i + 1]["y"][0] - out[i]["y"][1]) for i in range(len(out) - 1)]
    _emit({"file": str(a.image), "box": [x0, y0, x1, y1],
           "lines": out, "gaps": gaps,
           "note": "height = hauteur d'encre de la ligne ; la taille de police vaut "
                   "environ height / 0.72 pour une ligne avec majuscules et jambages"})


# ---------------------------------------------------------------------- palette

def cmd_palette(a):
    rgba = load_rgba(a.image)
    rgb = rgb_of(rgba)
    h, w = rgb.shape[:2]
    x0, y0, x1, y1 = _box(a.box) or (0, 0, w, h)
    flat = rgb[y0:y1, x0:x1].reshape(-1, 3)
    q = (flat // a.bucket) * a.bucket
    colors, counts = np.unique(q, axis=0, return_counts=True)
    order = np.argsort(-counts)[:a.top]
    total = counts.sum()
    res = []
    for i in order:
        c = colors[i]
        exact = flat[np.all(q == c, axis=1)].mean(axis=0)
        r, g, b = (int(round(v)) for v in exact)
        res.append({"hex": "#%02x%02x%02x" % (r, g, b), "rgb": [r, g, b],
                    "share": round(float(counts[i] / total), 4)})
    _emit({"file": str(a.image), "box": [x0, y0, x1, y1], "colors": res})


# ------------------------------------------------------------------ arretes

def cmd_edges(a):
    rgba = load_rgba(a.image)
    e = energy(rgb_of(rgba))
    h, w = e.shape
    x0, y0, x1, y1 = _box(a.box) or (0, 0, w, h)
    sub = e[y0:y1, x0:x1]
    rows = sub.mean(axis=1)
    cols = sub.mean(axis=0)

    def peaks(p, offset):
        if p.max() <= 0:
            return []
        thr = float(np.quantile(p, a.quantile))
        out = []
        for i in range(1, len(p) - 1):
            if p[i] > thr and p[i] >= p[i - 1] and p[i] >= p[i + 1]:
                out.append({"pos": int(i + offset), "strength": round(float(p[i]), 2)})
        out.sort(key=lambda d: -d["strength"])
        return out[:a.top]

    hs = sorted(peaks(rows, y0), key=lambda d: d["pos"])
    vs = sorted(peaks(cols, x0), key=lambda d: d["pos"])
    steps = sorted({b["pos"] - a_["pos"] for a_, b in zip(hs, hs[1:])}
                   | {b["pos"] - a_["pos"] for a_, b in zip(vs, vs[1:])})
    _emit({"file": str(a.image), "horizontal": hs, "vertical": vs,
           "observed_steps": [int(s) for s in steps if s > 1]})


# --------------------------------------------------------------------- rendu

def cmd_blueprint(a):
    spec = json.loads(Path(a.spec).read_text(encoding="utf-8"))
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from blueprint import render  # noqa: E402
    Path(a.output).write_text(render(spec), encoding="utf-8")
    _emit({"output": str(a.output), "nodes": len(spec.get("nodes", []))})


def main(argv=None):
    ap = argparse.ArgumentParser(prog="uispec", description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("layout", help="arbre de blocs par decoupage XY (gouttieres, paddings)")
    p.add_argument("image")
    p.add_argument("--box", default=None)
    p.add_argument("--depth", type=int, default=4)
    p.add_argument("--min-gap", dest="min_gap", type=int, default=6,
                   help="largeur minimale d'une gouttiere pour couper")
    p.add_argument("--quantile", type=float, default=0.25)
    p.add_argument("--min-size", dest="min_size", type=int, default=24)
    p.add_argument("--json", action="store_true", help="arbre complet en JSON")
    p.set_defaults(func=cmd_layout)

    p = sub.add_parser("lines", help="lignes de texte : hauteur d'encre, interlignes")
    p.add_argument("image")
    p.add_argument("--box", default=None)
    p.add_argument("--min-height", dest="min_height", type=int, default=4)
    p.set_defaults(func=cmd_lines)

    p = sub.add_parser("palette", help="couleurs dominantes d'une zone")
    p.add_argument("image")
    p.add_argument("--box", default=None)
    p.add_argument("--top", type=int, default=10)
    p.add_argument("--bucket", type=int, default=16)
    p.set_defaults(func=cmd_palette)

    p = sub.add_parser("edges", help="arretes horizontales / verticales dominantes")
    p.add_argument("image")
    p.add_argument("--box", default=None)
    p.add_argument("--top", type=int, default=20)
    p.add_argument("--quantile", type=float, default=0.9)
    p.set_defaults(func=cmd_edges)

    p = sub.add_parser("blueprint", help="rend la spec JSON en page de maquette cotee")
    p.add_argument("spec")
    p.add_argument("-o", "--output", required=True)
    p.set_defaults(func=cmd_blueprint)

    a = ap.parse_args(argv)
    return a.func(a)


if __name__ == "__main__":
    main()
