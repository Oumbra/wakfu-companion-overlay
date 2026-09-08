"""Extraction d'une icône incrustée dans un bouton, avec alpha progressif.

L'icône est un glyphe posé sur le fond du bouton. Un simple seuillage donnerait un
contour crénelé ; on calcule donc un alpha continu à partir de l'écart au fond, puis
on « démélange » la couleur (le pixel observé est une composition glyphe/fond) pour
obtenir une icône propre sur n'importe quel support.
"""
from __future__ import annotations

import numpy as np
from PIL import Image

from .content import _row_background, detect_content, side_bands
from .core import bbox_of, connected_components, dilate, luma, rgb_of


def extract_icon(rgba: np.ndarray, roi: np.ndarray | None = None, polarity: str = "auto",
                 k: float = 5.0, floor: float = 22.0, soft: float = 0.45,
                 keep: str = "all", box=None, side: int | None = None):
    """Retourne (RGBA de l'icône non recadrée, masque dur, rapport)."""
    rgb = rgb_of(rgba)
    lum = luma(rgb)
    h, w = lum.shape
    if roi is None:
        roi = rgba[..., 3] > 128
    if side is None:
        side = max(3, int(roi.sum(axis=1).max()) // 6)
    hard = detect_content(rgba, roi=roi, polarity=polarity, k=k, floor=floor,
                          grow=0, min_area=4, box=box, side=side)
    if not hard.any():
        raise ValueError("aucun glyphe détecté")

    if keep == "center":
        labels, n = connected_components(dilate(hard, 2))
        cy, cx = h / 2, w / 2
        best, bd = None, None
        for i in range(1, n + 1):
            ys, xs = np.nonzero(labels == i)
            d = (ys.mean() - cy) ** 2 + (xs.mean() - cx) ** 2
            if bd is None or d < bd:
                bd, best = d, i
        hard = hard & (labels == best)

    med, mad = _row_background(lum, roi, dilate(hard, 3), side)
    bg_rgb = np.zeros_like(rgb, np.float64)
    keep_bg = roi & ~dilate(hard, 3) & side_bands(roi, side)
    for y in range(h):
        vals = rgb[y][keep_bg[y]]
        if len(vals) < 3:
            vals = rgb[y][roi[y] & ~dilate(hard, 3)[y]]
        bg_rgb[y] = np.median(vals, axis=0) if len(vals) else rgb[y].mean(axis=0)

    thr = np.maximum(k * mad, floor)[:, None]
    d = np.abs(lum - med[:, None])
    alpha = np.clip((d - thr * soft) / np.maximum(thr * (1.0 - soft), 1e-6), 0, 1)
    alpha *= dilate(hard, 2) & roi          # cantonne l'alpha au glyphe et à son bord

    a = alpha[..., None]
    pure = np.where(a > 0.02, (rgb - (1 - a) * bg_rgb) / np.maximum(a, 1e-6), rgb)
    out = np.zeros((h, w, 4), np.uint8)
    out[..., :3] = np.clip(np.round(pure), 0, 255).astype(np.uint8)
    out[..., 3] = np.round(alpha * 255).astype(np.uint8)
    return out, hard, {"bbox": list(bbox_of(alpha > 0.05)) if (alpha > 0.05).any() else None}


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
