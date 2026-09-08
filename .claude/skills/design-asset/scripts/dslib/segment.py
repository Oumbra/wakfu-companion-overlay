"""Segmentation d'un composant d'interface dans une capture d'écran.

Hypothèse de travail (vérifiée sur `assets/design-system/*.png`) : la capture est un
recadrage grossier autour du composant, le reste étant du décor d'interface de jeu
(gris désaturé, parfois texturé). On sépare donc « ce qui touche le bord de l'image et
se propage sans rupture de couleur » (= le décor) du reste (= le composant).
"""
from __future__ import annotations

from collections import deque

import numpy as np

from .core import (bbox_of, connected_components, dilate, erode,
                   largest_component, rgb_of)


def flood_background(rgb: np.ndarray, tol: int = 12) -> np.ndarray:
    """Marque le décor : propagation 4-connexe depuis les bords, arrêtée par une
    rupture locale de couleur > `tol` (par canal). Tolère un fond en dégradé."""
    h, w, _ = rgb.shape
    img = rgb.astype(np.int16)
    bg = np.zeros((h, w), bool)
    dq: deque = deque()
    for x in range(w):
        for y in (0, h - 1):
            if not bg[y, x]:
                bg[y, x] = True
                dq.append((y, x))
    for y in range(h):
        for x in (0, w - 1):
            if not bg[y, x]:
                bg[y, x] = True
                dq.append((y, x))
    while dq:
        y, x = dq.popleft()
        c = img[y, x]
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            ny, nx = y + dy, x + dx
            if 0 <= ny < h and 0 <= nx < w and not bg[ny, nx]:
                if int(np.abs(img[ny, nx] - c).max()) <= tol:
                    bg[ny, nx] = True
                    dq.append((ny, nx))
    return bg


def recover_border(rgb: np.ndarray, comp: np.ndarray, bg: np.ndarray,
                   max_grow: int = 4, k: float = 4.0, floor: float = 8.0,
                   continuity: float = 0.7):
    """Réintègre le liseré de bordure avalé par la propagation.

    La bordure d'un composant Wakfu est un liseré sombre homogène de 1 à 2 px. Elle est
    séparée du décor par un saut franc, mais elle est *elle-même* uniforme : il suffit
    que la propagation y entre en un point (un coin, une zone où le contraste local
    faiblit) pour qu'elle la parcoure entière et la classe en décor.

    On la récupère anneau par anneau, avec un critère de **continuité** : un anneau
    n'est réintégré que si la grande majorité de ses pixels tranche sur le décor —
    c'est ce qui distingue un liseré qui fait le tour du composant d'une simple zone
    sombre du décor, qui n'en toucherait qu'un côté.

    La continuité décide si l'anneau *est* un liseré ; ce sont ensuite les pixels pris
    un à un qui entrent, et eux seuls. Absorber l'anneau entier carre les coins : la
    dilatation par un carré ajoute, en diagonale d'un coin arrondi, des pixels qui sont
    du décor. Le rectangle arrondi s'ajuste alors sur une boîte gonflée de 2 px et son
    alpha laisse passer un éclat de décor à la place du liseré noir.

    Retourne (masque élargi, diagnostic)."""
    ring = dilate(comp, 8) & bg & ~dilate(comp, 2)
    if ring.sum() < 20:
        return comp, {"applied": False, "reason": "pas assez de décor autour"}
    vals = rgb[ring].astype(np.float64)
    ref = np.median(vals, axis=0)
    mad = float(np.median(np.abs(vals - ref).max(axis=1)))
    thr = max(k * mad, floor)
    dist = np.abs(rgb.astype(np.float64) - ref).max(axis=-1)
    cur = comp.copy()
    rings = []
    for _ in range(max_grow):
        cand = dilate(cur, 1) & ~cur & bg
        n = int(cand.sum())
        if n == 0:
            break
        tranche = cand & (dist > thr)
        ratio = float(tranche.sum()) / n
        rings.append(round(ratio, 3))
        if ratio < continuity:
            break
        cur |= tranche
    return cur, {"applied": bool(cur.sum() > comp.sum()),
                 "ring_ratios": rings,
                 "rings_kept": sum(1 for r in rings if r >= continuity),
                 "background_ref": [int(round(v)) for v in ref],
                 "threshold": round(thr, 1),
                 "pixels_recovered": int(cur.sum() - comp.sum())}


def _keep_significant(fg: np.ndarray, min_ratio: float) -> np.ndarray:
    """Ne garde que les composantes dont l'aire atteint `min_ratio` × la plus grande.

    Une capture d'onglets contient plusieurs composants de taille comparable ; le décor
    y ajoute des éclats de quelques pixels. Le rapport à la plus grande composante les
    sépare sans avoir à fixer un seuil absolu qui dépendrait de l'échelle de la
    capture."""
    labels, n = connected_components(fg)
    if n <= 1:
        return fg
    sizes = np.bincount(labels.ravel())
    sizes[0] = 0
    keep = sizes >= max(sizes.max() * min_ratio, 1)
    keep[0] = False
    return keep[labels]


def component_mask(rgba: np.ndarray, tol: int = 12, keep_largest: bool = True,
                   border: bool = True, report: dict | None = None,
                   min_ratio: float = 0.1) -> np.ndarray:
    """Masque binaire du composant (décor retiré).

    `keep_largest` conserve la seule plus grande composante ; sinon toutes celles qui
    atteignent `min_ratio` de son aire — c'est le mode des captures qui montrent
    plusieurs composants côte à côte (barre d'onglets, groupe de boutons)."""
    rgb = rgb_of(rgba)
    bg = flood_background(rgb, tol)
    fg = ~bg
    # bouche les trous internes (le décor ne peut pas être « à l'intérieur »)
    fg = ~largest_component(~fg, connectivity=4) if (~fg).any() else fg
    if fg.any():
        fg = largest_component(fg) if keep_largest else _keep_significant(fg, min_ratio)
    if border and fg.any():
        fg, diag = recover_border(rgb, fg, ~fg)
        if report is not None:
            report["border_recovery"] = diag
        fg = ~largest_component(~fg, connectivity=4) if (~fg).any() else fg
        fg = largest_component(fg) if keep_largest else _keep_significant(fg, min_ratio)
    return fg


# ------------------------------------------------------- ajustement géométrique

def rounded_rect_alpha(w: int, h: int, x0: float, y0: float, x1: float, y1: float,
                       r: float, ss: int = 8) -> np.ndarray:
    """Alpha (float 0..1) d'un rectangle arrondi, antialiasé par suréchantillonnage."""
    yy, xx = np.mgrid[0:h * ss, 0:w * ss]
    px = (xx + 0.5) / ss
    py = (yy + 0.5) / ss
    cx = np.clip(px, x0 + r, x1 - r)
    cy = np.clip(py, y0 + r, y1 - r)
    inside = (px >= x0) & (px <= x1) & (py >= y0) & (py <= y1)
    inside &= np.hypot(px - cx, py - cy) <= r + 1e-9
    return inside.reshape(h, ss, w, ss).mean(axis=(1, 3))


def fit_rounded_rect(mask: np.ndarray, max_radius: int | None = None):
    """Ajuste (x0, y0, x1, y1, r) sur un masque. Le rayon est choisi sur l'erreur
    mesurée **dans les zones de coin** uniquement : ailleurs tous les rayons se valent
    et l'IoU globale ne discrimine pas."""
    bb = bbox_of(mask)
    if bb is None:
        raise ValueError("masque vide")
    x0, y0, x1, y1 = bb
    h, w = mask.shape
    rmax = max_radius if max_radius is not None else min(x1 - x0, y1 - y0) // 2
    best = (0, -1.0)
    for r in range(0, max(rmax, 0) + 1):
        a = rounded_rect_alpha(w, h, x0, y0, x1, y1, r) >= 0.5
        corner = np.zeros_like(mask)
        c = max(r, 3)
        corner[y0:y0 + c, x0:x0 + c] = True
        corner[y0:y0 + c, x1 - c:x1] = True
        corner[y1 - c:y1, x0:x0 + c] = True
        corner[y1 - c:y1, x1 - c:x1] = True
        inter = (a & mask & corner).sum()
        union = ((a | mask) & corner).sum()
        iou = inter / union if union else 0.0
        if iou > best[1]:
            best = (r, float(iou))
    return (float(x0), float(y0), float(x1), float(y1), float(best[0]), best[1])


# -------------------------------------------------------------- décontamination

def bleed_edges(rgba: np.ndarray, solid: np.ndarray, passes: int = 3,
                keep: np.ndarray | None = None) -> np.ndarray:
    """Étale les couleurs « sûres » (intérieur de `solid`) vers l'extérieur.

    Sans cela, les pixels du contour — déjà mélangés avec le gris du décor dans la
    capture — produisent un halo gris une fois l'asset posé sur un autre fond.

    `keep` protège des pixels de la réécriture tout en les laissant servir de source une
    fois atteints. C'est ce qu'il faut pour un pixel entièrement opaque du liseré : il
    est à l'intérieur de la forme, donc jamais contaminé, et le remplacer par la moyenne
    de ses voisins l'éclaircit — au milieu d'un bord c'est invisible, mais dans un coin
    arrondi la moitié du voisinage est du remplissage, et le noir y est mangé."""
    out = rgba.copy()
    known = solid.copy()
    for _ in range(passes):
        target = dilate(known, 1) & ~known
        if not target.any():
            break
        acc = np.zeros(rgba.shape[:2] + (3,), np.float64)
        cnt = np.zeros(rgba.shape[:2], np.float64)
        src = np.where(known[..., None], out[..., :3].astype(np.float64), 0.0)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                if dy == 0 and dx == 0:
                    continue
                acc += np.roll(np.roll(src, dy, 0), dx, 1)
                cnt += np.roll(np.roll(known.astype(np.float64), dy, 0), dx, 1)
        ok = target & (cnt > 0)
        vals = np.zeros_like(acc)
        np.divide(acc, np.maximum(cnt, 1)[..., None], out=vals)
        write = ok if keep is None else (ok & ~keep)
        out[..., :3] = np.where(write[..., None], np.round(vals).astype(np.uint8),
                                out[..., :3])
        known |= ok
    return out


def cutout(rgba: np.ndarray, tol: int = 12, radius: int | None = None,
           crop: bool = True, feather_inset: float = 0.0, border: bool = True,
           keep: str = "largest"):
    """Détoure le composant : alpha géométrique (rectangle arrondi) + recadrage.

    `keep="all"` traite une capture qui montre plusieurs composants côte à côte : un
    rectangle arrondi est ajusté **sur chacun**, et les alphas sont réunis. Un seul
    rectangle englobant les recouvrirait avec les gouttières de décor entre eux.

    Retourne (image RGBA détourée, métadonnées)."""
    diag: dict = {}
    mask = component_mask(rgba, tol=tol, border=border, report=diag,
                          keep_largest=(keep != "all"))
    h, w = mask.shape
    if keep == "all":
        labels, n = connected_components(mask)
        pieces = [labels == i for i in range(1, n + 1)]
        pieces.sort(key=lambda m: bbox_of(m)[0])
    else:
        pieces = [mask]

    alpha = np.zeros((h, w))
    parts = []
    for piece in pieces:
        px0, py0, px1, py1, r_fit, iou = fit_rounded_rect(piece)
        r = float(radius) if radius is not None else r_fit
        a = rounded_rect_alpha(w, h, px0 + feather_inset, py0 + feather_inset,
                               px1 - feather_inset, py1 - feather_inset,
                               max(r - feather_inset, 0))
        alpha = np.maximum(alpha, a)
        parts.append({"bbox": [int(px0), int(py0), int(px1), int(py1)],
                      "size": [int(px1 - px0), int(py1 - py0)],
                      "radius": round(float(r), 2),
                      "radius_fit_iou": round(iou, 4)})

    opaque = alpha >= 0.999
    solid = erode(opaque, 1)
    out = bleed_edges(rgba, solid, passes=3, keep=opaque)
    out[..., 3] = np.round(np.clip(alpha, 0, 1) * 255).astype(np.uint8)

    x0 = min(p["bbox"][0] for p in parts)
    y0 = min(p["bbox"][1] for p in parts)
    x1 = max(p["bbox"][2] for p in parts)
    y1 = max(p["bbox"][3] for p in parts)
    meta = {
        "bbox": [x0, y0, x1, y1],
        "size": [x1 - x0, y1 - y0],
        "radius": parts[0]["radius"],
        "radius_fit_iou": parts[0]["radius_fit_iou"],
        "margins": {"left": x0, "top": y0, "right": int(w - x1), "bottom": int(h - y1)},
        **diag,
    }
    if len(parts) > 1:
        meta["parts"] = parts
    if crop:
        out = out[y0:y1, x0:x1]
        # les parties sont rapportées dans le repère de l'image rendue
        for p in parts:
            b = p["bbox"]
            p["bbox"] = [b[0] - x0, b[1] - y0, b[2] - x0, b[3] - y0]
    return out, meta
