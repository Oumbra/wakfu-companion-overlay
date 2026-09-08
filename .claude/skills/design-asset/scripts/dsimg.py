#!/usr/bin/env python
"""dsimg — outillage d'images du design system (detourage, degenericisation, icones,
harmonisation de tailles, planche de controle).

    python dsimg.py <commande> --help

Toutes les commandes ecrivent un rapport JSON sur la sortie standard : c'est ce
rapport (tailles, rayon, marges, zones detectees) qui alimente ensuite le code egui.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))

from dslib import sheet as sh                                       # noqa: E402
from dslib.content import content_report, detect_content            # noqa: E402
from dslib.core import erode, load_rgba, luma, rgb_of, save_rgba    # noqa: E402
from dslib.icon import extract_icon, fit_box, trim                  # noqa: E402
from dslib.inpaint import inpaint_diffusion, inpaint_offsets        # noqa: E402
from dslib.scale9 import scale9                                     # noqa: E402
from dslib.segment import component_mask, cutout, fit_rounded_rect  # noqa: E402


def _box(s):
    if not s:
        return None
    v = [int(x) for x in s.replace(" ", "").split(",")]
    if len(v) != 4:
        raise argparse.ArgumentTypeError("format attendu : x0,y0,x1,y1")
    return tuple(v)


def _wh(s):
    w, h = s.lower().split("x")
    return int(w), int(h)


def _insets(s):
    v = [int(x) for x in s.replace(" ", "").split(",")]
    if len(v) == 1:
        v = v * 4
    if len(v) != 4:
        raise argparse.ArgumentTypeError("format attendu : l,t,r,b (ou une valeur unique)")
    return tuple(v)


def _emit(data):
    print(json.dumps(data, ensure_ascii=False, indent=2))


# --------------------------------------------------------------------- analyse

def describe(rgba, tol=12):
    mask = component_mask(rgba, tol=tol)
    x0, y0, x1, y1, r, iou = fit_rounded_rect(mask)
    rgb = rgb_of(rgba)
    h, w = mask.shape
    inner = mask.copy()
    inner[:int(y0) + 4, :] = False
    inner[int(y1) - 4:, :] = False
    inner[:, :int(x0) + 4] = False
    inner[:, int(x1) - 4:] = False
    rows = []
    for y in range(int(y0), int(y1)):
        vals = rgb[y][inner[y]]
        if len(vals):
            rows.append((y, [int(v) for v in np.median(vals, axis=0)]))
    gradient = False
    if rows:
        gradient = bool(abs(float(luma(np.array([rows[0][1]]))[0])
                            - float(luma(np.array([rows[-1][1]]))[0])) > 6)
    border = None
    if x1 - x0 > 14:
        border = [int(v) for v in np.median(rgb[int(y0), int(x0) + 6:int(x1) - 6], axis=0)]
    return {
        "image_size": [w, h],
        "component": {
            "bbox": [int(x0), int(y0), int(x1), int(y1)],
            "size": [int(x1 - x0), int(y1 - y0)],
            "corner_radius": round(float(r), 2),
            "radius_fit_iou": round(iou, 4),
            "margins": {"left": int(x0), "top": int(y0),
                        "right": int(w - x1), "bottom": int(h - y1)},
        },
        "fill": {
            "top": rows[0][1] if rows else None,
            "middle": rows[len(rows) // 2][1] if rows else None,
            "bottom": rows[-1][1] if rows else None,
            "vertical_gradient": gradient,
        },
        "border": border,
    }


def tol_sweep(rgba, values=(4, 6, 8, 12, 16, 22)):
    """Taille detectee pour plusieurs tolerances : revele les images ou le composant
    a presque la teinte du decor (le detourage s'effondre alors sur une valeur haute)."""
    out = {}
    for t in values:
        try:
            mask = component_mask(rgba, tol=t)
            x0, y0, x1, y1, _r, _iou = fit_rounded_rect(mask)
            out[str(t)] = [int(x1 - x0), int(y1 - y0)]
        except Exception:
            out[str(t)] = None
    return out


def cmd_analyze(a):
    rgba = load_rgba(a.image)
    rep = describe(rgba, a.tol)
    rep["tol_sweep"] = tol_sweep(rgba)
    cut, _ = cutout(rgba, tol=a.tol)
    content = detect_content(cut, polarity=a.polarity, grow=a.grow, box=_box(a.box),
                             k=a.k, floor=a.floor)
    rep["content"] = content_report(content)
    if rep["content"]["bbox"]:
        cx0, cy0, cx1, cy1 = rep["content"]["bbox"]
        ch, cw = cut.shape[:2]
        rep["content"]["padding"] = {"left": cx0, "top": cy0,
                                     "right": cw - cx1, "bottom": ch - cy1}
    _emit({"file": str(a.image), **rep})


# ------------------------------------------------------------------- detourage

def cmd_cutout(a):
    rgba = load_rgba(a.image)
    out, meta = cutout(rgba, tol=a.tol, radius=a.radius, crop=not a.no_crop,
                       feather_inset=a.inset)
    save_rgba(out, a.output)
    _emit({"input": str(a.image), "output": str(a.output),
           "size": [int(out.shape[1]), int(out.shape[0])], **meta})


# --------------------------------------------- retrait du contenu + inpainting

def strip(rgba, polarity="auto", grow=2, box=None, method="offsets", max_dy=6,
          k_offsets=5, min_shift=3, floor=22.0, k=5.0, roi_inset=3):
    # La bordure du composant est sombre et tranche sur le remplissage : sans retrait
    # de cette couronne, elle serait prise pour du contenu incruste et effacee.
    roi = rgba[..., 3] > 128
    if roi_inset > 0:
        roi = erode(roi, roi_inset)
    mask = detect_content(rgba, roi=roi, polarity=polarity, grow=grow, box=box,
                          floor=floor, k=k)
    if not mask.any():
        return rgba.copy(), mask, []
    if method == "diffusion":
        filled, offsets = inpaint_diffusion(rgba[..., :3], mask), []
    else:
        filled, offsets = inpaint_offsets(rgba[..., :3], mask, k=k_offsets,
                                          max_dy=max_dy, min_shift=min_shift)
    out = rgba.copy()
    out[..., :3] = np.round(filled).astype(np.uint8)
    return out, mask, offsets


def cmd_strip(a):
    rgba = load_rgba(a.image)
    out, mask, offsets = strip(rgba, a.polarity, a.grow, _box(a.box), a.method,
                               a.max_dy, a.offsets, a.min_shift, a.floor, a.k,
                               a.roi_inset if a.roi_inset >= 0 else 3)
    save_rgba(out, a.output)
    _emit({"input": str(a.image), "output": str(a.output),
           "removed": content_report(mask),
           "offsets": [{"dy": int(o[0]), "dx": int(o[1]), "err": round(float(o[2]), 1),
                        "coverage": round(float(o[3]), 3)} for o in offsets]})


def cmd_genericize(a):
    rgba = load_rgba(a.image)
    cut, meta = cutout(rgba, tol=a.tol, radius=a.radius)
    inset = a.roi_inset
    if inset < 0:
        inset = int(meta.get("border_recovery", {}).get("rings_kept", 1)) + 2
    out, mask, offsets = strip(cut, a.polarity, a.grow, _box(a.box), a.method,
                               a.max_dy, a.offsets, a.min_shift, a.floor, a.k, inset)
    save_rgba(out, a.output)
    _emit({"input": str(a.image), "output": str(a.output),
           "size": [int(out.shape[1]), int(out.shape[0])], "component": meta,
           "roi_inset": inset, "removed": content_report(mask),
           "offsets": [{"dy": int(o[0]), "dx": int(o[1])} for o in offsets]})


# ----------------------------------------------------------------------- icone

def cmd_icon(a):
    rgba = load_rgba(a.image)
    meta = {}
    if a.from_button:
        rgba, meta = cutout(rgba, tol=a.tol)
    inset = a.roi_inset
    if inset < 0:
        # La bordure du bouton porteur est sombre et tranche sur son remplissage : sans
        # retrait de cette couronne, elle est prise pour le glyphe et le masque couvre
        # tout le bouton.
        inset = (int(meta.get("border_recovery", {}).get("rings_kept", 1)) + 2
                 if a.from_button else 0)
    roi = rgba[..., 3] > 128
    if inset > 0:
        roi = erode(roi, inset)
    icon, _hard, rep = extract_icon(rgba, roi=roi, polarity=a.polarity, keep=a.keep,
                                    box=_box(a.box), floor=a.floor, k=a.k)
    if a.size:
        out = fit_box(icon, a.size, a.padding)
    else:
        out, _ = trim(icon)
    save_rgba(out, a.output)
    _emit({"input": str(a.image), "output": str(a.output), "roi_inset": inset,
           "button": meta.get("size"),
           "glyph_bbox": rep["bbox"], "size": [int(out.shape[1]), int(out.shape[0])]})


# ------------------------------------------------------------- harmonisation

def cmd_resize(a):
    rgba = load_rgba(a.image)
    w, h = a.to
    out = scale9(rgba, w, h, a.insets, a.fill)
    save_rgba(out, a.output)
    _emit({"input": str(a.image), "output": str(a.output), "size": [w, h],
           "insets": list(a.insets), "fill": a.fill})


def cmd_align(a):
    imgs = {p: load_rgba(p) for p in a.images}
    if a.to:
        w, h = a.to
    else:
        w = max(v.shape[1] for v in imgs.values())
        h = max(v.shape[0] for v in imgs.values())
    outdir = Path(a.outdir)
    outdir.mkdir(parents=True, exist_ok=True)
    res = []
    for p, arr in imgs.items():
        out = scale9(arr, w, h, a.insets, a.fill)
        dest = outdir / Path(p).name
        save_rgba(out, dest)
        res.append({"input": p, "output": str(dest),
                    "from": [int(arr.shape[1]), int(arr.shape[0])], "to": [w, h]})
    _emit({"target": [w, h], "insets": list(a.insets), "fill": a.fill, "items": res})


# --------------------------------------------------------------------- planche

def cmd_sheet(a):
    items = []
    for spec in a.images:
        if "::" in spec:
            before, after = spec.split("::", 1)
            b, af = load_rgba(before), load_rgba(after)
            cells = [sh.cell("avant", b, a.scale, "dark"),
                     sh.cell("apres", af, a.scale, "checker"),
                     sh.cell("apres x%d clair" % (a.scale * 3), af, a.scale * 3, "light"),
                     sh.cell("apres x%d sombre" % (a.scale * 3), af, a.scale * 3, "dark")]
            name = "%s -> %s (%dx%d)" % (Path(before).name, Path(after).name,
                                         af.shape[1], af.shape[0])
        else:
            arr = load_rgba(spec)
            cells = [sh.cell("damier", arr, a.scale, "checker"),
                     sh.cell("fond sombre", arr, a.scale, "dark"),
                     sh.cell("fond clair", arr, a.scale, "light"),
                     sh.cell("x%d" % (a.scale * 3), arr, a.scale * 3, "checker")]
            name = "%s - %dx%d" % (Path(spec).name, arr.shape[1], arr.shape[0])
        items.append(sh.item(name, cells))
    Path(a.output).write_text(sh.page(a.title, a.subtitle, items), encoding="utf-8")
    _emit({"output": str(a.output), "items": len(items)})


# ---------------------------------------------------------------------- parser

def main(argv=None):
    ap = argparse.ArgumentParser(prog="dsimg", description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    def tol_opt(p):
        p.add_argument("--tol", type=int, default=12,
                       help="tolerance de propagation du decor (defaut 12)")

    def content_opts(p):
        p.add_argument("--polarity", choices=["auto", "light", "dark", "both"],
                       default="auto")
        p.add_argument("--grow", type=int, default=2,
                       help="dilatation du masque (halo / ombre du texte)")
        p.add_argument("--box", default=None,
                       help="zone forcee x0,y0,x1,y1 (repere de l'image detouree)")
        p.add_argument("--k", type=float, default=5.0, help="seuil adaptatif (x MAD)")
        p.add_argument("--floor", type=float, default=22.0,
                       help="seuil plancher en luminance")
        p.add_argument("--roi-inset", dest="roi_inset", type=int, default=-1,
                       help="couronne exclue de la detection, en px (protege la "
                            "bordure du composant ; -1 = automatique)")

    def inpaint_opts(p):
        p.add_argument("--method", choices=["offsets", "diffusion"], default="offsets")
        p.add_argument("--max-dy", dest="max_dy", type=int, default=6,
                       help="amplitude verticale de recherche (0 = illimitee)")
        p.add_argument("--offsets", type=int, default=5,
                       help="nombre de decalages votants")
        p.add_argument("--min-shift", dest="min_shift", type=int, default=3)

    p = sub.add_parser("analyze", help="mesures geometriques + contenu detecte (JSON)")
    p.add_argument("image")
    tol_opt(p)
    content_opts(p)
    p.set_defaults(func=cmd_analyze)

    p = sub.add_parser("cutout", help="detoure le composant, fond transparent")
    p.add_argument("image")
    p.add_argument("-o", "--output", required=True)
    tol_opt(p)
    p.add_argument("--radius", type=float, default=None, help="force le rayon des coins")
    p.add_argument("--inset", type=float, default=0.0, help="retracte le contour (px)")
    p.add_argument("--no-crop", action="store_true")
    p.set_defaults(func=cmd_cutout)

    p = sub.add_parser("strip", help="retire le contenu incruste et reconstruit le fond")
    p.add_argument("image")
    p.add_argument("-o", "--output", required=True)
    content_opts(p)
    inpaint_opts(p)
    p.set_defaults(func=cmd_strip)

    p = sub.add_parser("genericize",
                       help="detourage + retrait du contenu (pipeline bouton complet)")
    p.add_argument("image")
    p.add_argument("-o", "--output", required=True)
    tol_opt(p)
    p.add_argument("--radius", type=float, default=None)
    content_opts(p)
    inpaint_opts(p)
    p.set_defaults(func=cmd_genericize)

    p = sub.add_parser("icon", help="extrait un glyphe et le normalise a une taille")
    p.add_argument("image")
    p.add_argument("-o", "--output", required=True)
    tol_opt(p)
    p.add_argument("--size", type=int, default=None,
                   help="boite carree de sortie (16, 24...)")
    p.add_argument("--padding", type=int, default=0)
    p.add_argument("--keep", choices=["all", "center"], default="all")
    p.add_argument("--from-button", dest="from_button", action="store_true",
                   help="l'image est un bouton-icone : detoure d'abord le bouton")
    content_opts(p)
    p.set_defaults(func=cmd_icon)

    p = sub.add_parser("resize", help="redimensionne en 9-slice (coins preserves)")
    p.add_argument("image")
    p.add_argument("-o", "--output", required=True)
    p.add_argument("--to", type=_wh, required=True, metavar="LxH")
    p.add_argument("--insets", type=_insets, default=(8, 8, 8, 8))
    p.add_argument("--fill", choices=["stretch", "tile", "mirror"], default="stretch")
    p.set_defaults(func=cmd_resize)

    p = sub.add_parser("align", help="harmonise un lot sur une taille commune (9-slice)")
    p.add_argument("images", nargs="+")
    p.add_argument("-o", "--outdir", required=True)
    p.add_argument("--to", type=_wh, default=None, metavar="LxH",
                   help="defaut : la plus grande taille du lot")
    p.add_argument("--insets", type=_insets, default=(8, 8, 8, 8))
    p.add_argument("--fill", choices=["stretch", "tile", "mirror"], default="stretch")
    p.set_defaults(func=cmd_align)

    p = sub.add_parser("sheet", help="planche de controle HTML (a publier en Artifact)")
    p.add_argument("images", nargs="+",
                   help="chemin, ou 'avant::apres' (double deux-points : les chemins "
                        "Windows contiennent deja un ':')")
    p.add_argument("-o", "--output", required=True)
    p.add_argument("--title", default="Planche de controle")
    p.add_argument("--subtitle", default="")
    p.add_argument("--scale", type=int, default=2)
    p.set_defaults(func=cmd_sheet)

    a = ap.parse_args(argv)
    return a.func(a)


if __name__ == "__main__":
    main()
