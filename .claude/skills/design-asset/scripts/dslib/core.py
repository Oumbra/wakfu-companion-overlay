"""Primitives partagées : E/S, masques, composantes connexes, morphologie.

Dépendances volontairement limitées à Pillow + numpy (aucun OpenCV / ImageMagick :
le poste de travail n'en dispose pas et le skill doit rester exécutable tel quel).
"""
from __future__ import annotations

import numpy as np
from PIL import Image


# --------------------------------------------------------------------------- E/S

def load_rgba(path) -> np.ndarray:
    """Charge une image en tableau (h, w, 4) uint8."""
    return np.array(Image.open(path).convert("RGBA"))


def save_rgba(arr: np.ndarray, path) -> None:
    Image.fromarray(arr.astype(np.uint8), "RGBA").save(path)


def rgb_of(rgba: np.ndarray) -> np.ndarray:
    return rgba[..., :3].astype(np.int16)


def luma(rgb: np.ndarray) -> np.ndarray:
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


# ------------------------------------------------------------------- morphologie

def _shift(mask: np.ndarray, dy: int, dx: int) -> np.ndarray:
    out = np.zeros_like(mask)
    h, w = mask.shape
    ys, ye = max(0, dy), min(h, h + dy)
    xs, xe = max(0, dx), min(w, w + dx)
    out[ys:ye, xs:xe] = mask[ys - dy:ye - dy, xs - dx:xe - dx]
    return out


def dilate(mask: np.ndarray, radius: int = 1, diamond: bool = False) -> np.ndarray:
    """Dilatation par un disque (approximé) ou un losange de rayon `radius`."""
    out = mask.copy()
    for _ in range(radius):
        acc = out.copy()
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                if diamond and abs(dy) + abs(dx) > 1:
                    continue
                acc |= _shift(out, dy, dx)
        out = acc
    return out


def erode(mask: np.ndarray, radius: int = 1, diamond: bool = False) -> np.ndarray:
    return ~dilate(~mask, radius, diamond)


# --------------------------------------------------------- composantes connexes

def connected_components(mask: np.ndarray, connectivity: int = 8):
    """Étiquetage par balayage en deux passes (union-find). Retourne (labels, count)."""
    h, w = mask.shape
    labels = np.zeros((h, w), np.int32)
    parent = [0]

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    def union(a, b):
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[max(ra, rb)] = min(ra, rb)

    neigh = [(-1, 0), (0, -1)] + ([(-1, -1), (-1, 1)] if connectivity == 8 else [])
    nxt = 1
    for y in range(h):
        row = mask[y]
        for x in range(w):
            if not row[x]:
                continue
            found = []
            for dy, dx in neigh:
                ny, nx = y + dy, x + dx
                if 0 <= ny < h and 0 <= nx < w and labels[ny, nx]:
                    found.append(labels[ny, nx])
            if not found:
                parent.append(nxt)
                labels[y, x] = nxt
                nxt += 1
            else:
                m = min(found)
                labels[y, x] = m
                for f in found:
                    union(m, f)

    remap = {}
    out = np.zeros_like(labels)
    count = 0
    for y in range(h):
        for x in range(w):
            lb = labels[y, x]
            if lb:
                r = find(lb)
                if r not in remap:
                    count += 1
                    remap[r] = count
                out[y, x] = remap[r]
    return out, count


def largest_component(mask: np.ndarray, connectivity: int = 8) -> np.ndarray:
    labels, n = connected_components(mask, connectivity)
    if n == 0:
        return mask
    sizes = np.bincount(labels.ravel())
    sizes[0] = 0
    return labels == int(np.argmax(sizes))


def bbox_of(mask: np.ndarray):
    ys, xs = np.nonzero(mask)
    if len(ys) == 0:
        return None
    return int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1
