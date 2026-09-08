"""Extraction d'une icône incrustée dans un bouton, avec alpha progressif.

Le glyphe est posé sur le fond du bouton : les pixels de son contour sont des mélanges
des deux. Un seuillage sur le seul écart de luminance au fond ne sait pas les distinguer
de la texture du bouton (hachures, grain), et laisse un nuage de pixels du bouton autour
du glyphe — visible dès qu'on pose l'icône sur un fond sombre.

On procède donc par **démélange à deux couleurs**. Pour chaque pixel de la zone de
transition, on connaît la couleur du fond `F` (reconstruite sous le glyphe) et celle du
glyphe `G` (étalée depuis le glyphe franc). Un vrai pixel de contour se trouve sur le
segment F→G : sa proportion donne l'alpha, et son écart à ce segment — le résidu — reste
faible. Un pixel de texture du bouton, lui, sort du segment : résidu élevé, donc rejeté.

Ce modèle suppose deux couleurs seulement, or les glyphes du jeu en ont trois : un cerne
sombre borde le glyphe clair. On fait donc entrer ce cerne dans le glyphe *avant* le
démélange (recherche dans les deux polarités, restreinte au voisinage connexe du cœur),
et il ne reste en transition qu'un liseré d'antialiasing d'un pixel.
"""
from __future__ import annotations

import numpy as np
from PIL import Image

from .content import _row_background, detect_content, side_bands
from .core import bbox_of, connected_components, dilate, erode, luma, rgb_of
from .inpaint import inpaint_diffusion


def _spread(rgb: np.ndarray, source: np.ndarray, target: np.ndarray,
            passes: int = 4) -> np.ndarray:
    """Étale les couleurs de `source` sur `target`, de proche en proche."""
    out = rgb.astype(np.float64).copy()
    known = source.copy()
    for _ in range(passes):
        nxt = dilate(known, 1) & ~known & (target | source)
        if not nxt.any():
            break
        acc = np.zeros_like(out)
        cnt = np.zeros(rgb.shape[:2])
        src = np.where(known[..., None], out, 0.0)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                if dy == 0 and dx == 0:
                    continue
                acc += np.roll(np.roll(src, dy, 0), dx, 1)
                cnt += np.roll(np.roll(known.astype(float), dy, 0), dx, 1)
        ok = nxt & (cnt > 0)
        vals = acc / np.maximum(cnt, 1)[..., None]
        out = np.where(ok[..., None], vals, out)
        known = known | ok
    return out


def _keep_attached(mask: np.ndarray, anchor: np.ndarray) -> np.ndarray:
    """Ne garde que les composantes connexes qui touchent `anchor`."""
    if not mask.any():
        return mask
    labels, n = connected_components(mask)
    keep = np.zeros(n + 1, bool)
    for lb in np.unique(labels[anchor & mask]):
        if lb:
            keep[lb] = True
    return keep[labels]


def extract_icon(rgba: np.ndarray, roi: np.ndarray | None = None, polarity: str = "auto",
                 k: float = 5.0, floor: float = 22.0, soft: float = 0.45,
                 keep: str = "all", box=None, side: int | None = None,
                 grow: int = 2, residual: float = 0.32, alpha_floor: float = 0.12,
                 rim_gain: float = 2.4):
    """Retourne (RGBA de l'icône non recadrée, masque dur du glyphe, rapport)."""
    rgb = rgb_of(rgba).astype(np.float64)
    lum = luma(rgb)
    h, w = lum.shape
    if roi is None:
        roi = rgba[..., 3] > 128
    if side is None:
        side = max(3, int(roi.sum(axis=1).max()) // 6)

    # 1. Cœur du glyphe : ce qui tranche franchement sur le fond, dans la polarité voulue.
    core = detect_content(rgba, roi=roi, polarity=polarity, k=k, floor=floor,
                          grow=0, min_area=4, box=box, side=side)
    if not core.any():
        raise ValueError("aucun glyphe détecté")

    if keep == "center":
        labels, n = connected_components(dilate(core, 2))
        cy, cx = h / 2, w / 2
        best, bd = None, None
        for i in range(1, n + 1):
            ys, xs = np.nonzero(labels == i)
            d = (ys.mean() - cy) ** 2 + (xs.mean() - cx) ** 2
            if bd is None or d < bd:
                bd, best = d, i
        core = core & (labels == best)

    # 2. Cerne : les glyphes du jeu sont cernés d'un liseré de la teinte opposée (noir
    #    autour d'un glyphe blanc). Ce liseré fait partie du dessin, mais il échappe à la
    #    polarité du cœur. On le récupère en cherchant les deux polarités, restreint au
    #    voisinage du cœur et à ce qui lui est connexe — sinon les hachures du bouton,
    #    tout aussi contrastées, entreraient avec.
    #    Le seuil du cerne est plus exigeant que celui du cœur (`rim_gain`) : le liseré
    #    est franc, alors que l'ombre portée d'un glyphe et les hachures du bouton sont
    #    des demi-teintes. C'est ce qui les sépare.
    both = detect_content(rgba, roi=roi, polarity="both", k=k * rim_gain,
                          floor=floor * rim_gain, grow=0, min_area=3, box=box, side=side)
    hard = core | _keep_attached(both & dilate(core, grow), core)

    # 3. Fond et couleur de glyphe, par pixel.
    band = dilate(hard, 1) & roi & ~hard
    hole = dilate(hard, 2) & roi
    if hole.sum() < roi.sum() * 0.9:
        bg_field = inpaint_diffusion(rgb.astype(np.uint8), hole, iterations=150)
    else:
        med, _m = _row_background(lum, roi, dilate(hard, 3), side)
        keep_bg = roi & ~dilate(hard, 3) & side_bands(roi, side)
        bg_field = np.zeros_like(rgb)
        for y in range(h):
            vals = rgb[y][keep_bg[y]]
            if len(vals) < 3:
                vals = rgb[y][roi[y] & ~dilate(hard, 3)[y]]
            bg_field[y] = np.median(vals, axis=0) if len(vals) else med[y]
    fg_field = _spread(rgb, hard, band, passes=2)

    # 4. Démélange à deux couleurs sur la bande de transition : un pixel de contour se
    #    trouve sur le segment fond → glyphe, sa position donne l'alpha. Un pixel de
    #    texture du bouton en sort — son résidu le trahit, et il est rejeté. C'est ce
    #    test qui supprime le nuage de pixels beiges autour du glyphe.
    d = fg_field - bg_field
    dn2 = (d ** 2).sum(axis=-1)
    proj = ((rgb - bg_field) * d).sum(axis=-1) / np.maximum(dn2, 1e-6)
    a = np.clip(proj, 0.0, 1.0)
    resid = np.sqrt(((rgb - (bg_field + a[..., None] * d)) ** 2).sum(axis=-1))
    tol = np.maximum(residual * np.sqrt(dn2), 8.0)

    alpha = np.zeros((h, w))
    alpha[hard] = 1.0
    ok = band & (dn2 > 100) & (resid <= tol)
    alpha[ok] = a[ok]
    alpha[alpha < alpha_floor] = 0.0
    alpha = np.where(_keep_attached(alpha > 0, hard), alpha, 0.0)

    # 5. Couleur : observée dans le glyphe, couleur de glyphe étalée dans la bande — le
    #    démélange y ramène, sans amplifier le bruit d'un pixel très transparent.
    pure = np.where(hard[..., None], rgb, fg_field)
    out = np.zeros((h, w, 4), np.uint8)
    out[..., :3] = np.clip(np.round(pure), 0, 255).astype(np.uint8)
    out[..., 3] = np.round(np.clip(alpha, 0, 1) * 255).astype(np.uint8)
    vis = alpha > 0.02
    return out, hard, {
        "bbox": list(bbox_of(vis)) if vis.any() else None,
        "core_pixels": int(core.sum()),
        "rim_pixels": int(hard.sum() - core.sum()),
        "band_kept": int((band & (alpha > 0)).sum()),
        "band_rejected": int((band & (alpha <= 0)).sum()),
    }


def trim(rgba: np.ndarray, threshold: int = 8):
    m = rgba[..., 3] > threshold
    bb = bbox_of(m)
    if bb is None:
        return rgba, None
    x0, y0, x1, y1 = bb
    return rgba[y0:y1, x0:x1], bb


def square_pad(rgba: np.ndarray) -> np.ndarray:
    h, w = rgba.shape[:2]
    s = max(h, w)
    out = np.zeros((s, s, 4), np.uint8)
    oy, ox = (s - h) // 2, (s - w) // 2
    out[oy:oy + h, ox:ox + w] = rgba
    return out


def resize_rgba(rgba: np.ndarray, size, resample=Image.LANCZOS) -> np.ndarray:
    """Redimensionne en alpha prémultiplié : évite les franges sombres sur le contour."""
    arr = rgba.astype(np.float64)
    a = arr[..., 3:4] / 255.0
    pre = np.concatenate([arr[..., :3] * a, arr[..., 3:4]], axis=-1)
    im = Image.fromarray(np.clip(np.round(pre), 0, 255).astype(np.uint8), "RGBA")
    im = im.resize(size, resample)
    out = np.array(im).astype(np.float64)
    a2 = out[..., 3:4] / 255.0
    rgb = np.where(a2 > 1e-3, out[..., :3] / np.maximum(a2, 1e-6), 0)
    res = np.zeros(out.shape, np.uint8)
    res[..., :3] = np.clip(np.round(rgb), 0, 255)
    res[..., 3] = np.clip(np.round(out[..., 3]), 0, 255)
    return res


def fit_box(rgba: np.ndarray, size: int, padding: int = 0) -> np.ndarray:
    """Recadre sur le contenu, met à l'échelle dans une boîte carrée `size`, centre."""
    trimmed, _ = trim(rgba)
    inner = max(size - 2 * padding, 1)
    h, w = trimmed.shape[:2]
    scale = min(inner / w, inner / h)
    nw, nh = max(1, int(round(w * scale))), max(1, int(round(h * scale)))
    small = resize_rgba(trimmed, (nw, nh))
    out = np.zeros((size, size, 4), np.uint8)
    oy, ox = (size - nh) // 2, (size - nw) // 2
    out[oy:oy + nh, ox:ox + nw] = small
    return out
