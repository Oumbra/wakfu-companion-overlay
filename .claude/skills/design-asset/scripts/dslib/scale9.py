"""Redimensionnement 9-slice : change la taille d'un composant sans déformer ses
coins, sa bordure ni son arrondi. Indispensable pour aligner les variantes d'un même
composant (primary / secondary / danger, normal / hover) sur une taille commune.
"""
from __future__ import annotations

import numpy as np
from PIL import Image

from .icon import resize_rgba


def _stretch_axis(arr: np.ndarray, axis: int, target: int, mode: str) -> np.ndarray:
    cur = arr.shape[axis]
    if cur == target:
        return arr
    if mode == "stretch":
        h, w = arr.shape[:2]
        size = (target, h) if axis == 1 else (w, target)
        return resize_rgba(arr, size, Image.BILINEAR)
    idx = np.arange(target) % cur
    if mode == "mirror":
        period = max(cur * 2 - 2, 1)
        m = np.arange(target) % period
        idx = np.where(m < cur, m, period - m)
    return np.take(arr, idx, axis=axis)


def scale9(rgba: np.ndarray, width: int, height: int, insets, fill: str = "stretch") -> np.ndarray:
    """`insets` = (gauche, haut, droite, bas) en pixels, préservés tels quels."""
    l, t, r, b = insets
    h, w = rgba.shape[:2]
    if l + r >= width or t + b >= height or l + r >= w or t + b >= h:
        raise ValueError(f"marges 9-slice trop grandes pour {w}x{h} -> {width}x{height}")
    cols = [(0, l), (l, w - r), (w - r, w)]
    rows = [(0, t), (t, h - b), (h - b, h)]
    tcols = [l, width - l - r, r]
    trows = [t, height - t - b, b]
    out = np.zeros((height, width, 4), np.uint8)
    oy = 0
    for (ry0, ry1), th in zip(rows, trows):
        ox = 0
        for (rx0, rx1), tw in zip(cols, tcols):
            piece = rgba[ry0:ry1, rx0:rx1]
            if piece.size and th > 0 and tw > 0:
                piece = _stretch_axis(piece, 1, tw, fill if rx1 - rx0 != tw else "stretch")
                piece = _stretch_axis(piece, 0, th, fill if ry1 - ry0 != th else "stretch")
                out[oy:oy + th, ox:ox + tw] = piece[:th, :tw]
            ox += tw
        oy += th
    return out
