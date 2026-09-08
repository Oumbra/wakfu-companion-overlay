"""Reconstruction du fond sous un contenu retiré (label, icône, valeur).

Deux moteurs, aucun modèle génératif requis :

* `offsets` — recherche des meilleurs décalages globaux (parenté : *content-aware
  fill* par statistiques d'offsets). Le fond des composants Wakfu est une texture
  régulière (hachures diagonales) : copier depuis un décalage qui aligne le motif
  reconstruit les diagonales, ce qu'une diffusion écraserait en aplat.
* `diffusion` — moyenne itérative depuis le bord du trou. Pour les fonds lisses ou
  quand la zone valide est trop petite pour trouver un décalage.
"""
from __future__ import annotations

import warnings

import numpy as np

from .core import dilate


def _shift_valid(arr: np.ndarray, dy: int, dx: int, fill=0.0):
    """Décale sans rebouclage ; retourne (décalé, masque des positions définies)."""
    h, w = arr.shape[:2]
    out = np.full_like(arr, fill)
    ok = np.zeros((h, w), bool)
    ys, ye = max(0, -dy), min(h, h - dy)
    xs, xe = max(0, -dx), min(w, w - dx)
    if ys < ye and xs < xe:
        out[ys:ye, xs:xe] = arr[ys + dy:ye + dy, xs + dx:xe + dx]
        ok[ys:ye, xs:xe] = True
    return out, ok


def inpaint_diffusion(rgb: np.ndarray, hole: np.ndarray, iterations: int = 400,
                      valid: np.ndarray | None = None) -> np.ndarray:
    out = rgb.astype(np.float64).copy()
    known = ~hole if valid is None else (valid & ~hole)
    cur = out.copy()
    cur[hole] = 0.0
    kn = known.astype(np.float64)
    for _ in range(iterations):
        acc = np.zeros_like(cur)
        cnt = np.zeros(rgb.shape[:2])
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            s, ok = _shift_valid(cur, dy, dx)
            k, _ = _shift_valid(kn, dy, dx)
            acc += s * ok[..., None]
            cnt += k * ok
        upd = np.divide(acc, np.maximum(cnt, 1e-9)[..., None])
        cur = np.where(hole[..., None] & (cnt > 0)[..., None], upd, cur)
        kn = np.where(hole, np.minimum(kn + (cnt > 0), 1.0), kn)
    return np.clip(cur, 0, 255)


def find_offsets(rgb: np.ndarray, hole: np.ndarray, k: int = 5, context: int = 6,
                 max_dx: int = 0, max_dy: int = 0, min_shift: int = 3,
                 dy_penalty: float = 6.0, valid: np.ndarray | None = None):
    """Retourne les `k` meilleurs décalages (dy, dx, erreur, couverture).

    `valid` restreint la **zone source** : tout ce qui n'y est pas ne peut ni servir de
    référence de comparaison ni être recopié. C'est indispensable sur un composant
    détouré — sans cela le liseré de bordure, sombre et à quelques pixels du contenu à
    effacer, est un décalage source parfaitement légitime et se retrouve peint au
    milieu du bouton."""
    h, w = hole.shape
    max_dx = max_dx or w
    max_dy = max_dy or h
    img = rgb.astype(np.float64)
    valid = (~hole) if valid is None else (valid & ~hole)
    band = dilate(hole, context) & valid
    n_hole = int(hole.sum())
    results = []
    for dy in range(-max_dy, max_dy + 1):
        for dx in range(-max_dx, max_dx + 1):
            if abs(dy) < min_shift and abs(dx) < min_shift:
                continue
            shifted, inb = _shift_valid(img, dy, dx)
            svalid, _ = _shift_valid(valid.astype(np.float64), dy, dx)
            usable = band & inb & (svalid > 0.5)
            n = int(usable.sum())
            if n < max(24, 0.15 * band.sum()):
                continue
            cov = float((hole & inb & (svalid > 0.5)).sum()) / max(n_hole, 1)
            if cov < 0.55:
                continue
            diff = (img - shifted)[usable]
            err = float((diff ** 2).sum(axis=-1).mean())
            results.append((err + dy_penalty * abs(dy), dy, dx, err, cov))
    results.sort(key=lambda t: t[0])
    picked = []
    for _score, dy, dx, err, cov in results:
        # évite k décalages quasi identiques (mêmes pixels sources)
        if any(abs(dy - p[0]) <= 1 and abs(dx - p[1]) <= 1 for p in picked):
            continue
        picked.append((dy, dx, err, cov))
        if len(picked) >= k:
            break
    return picked


def inpaint_offsets(rgb: np.ndarray, hole: np.ndarray, k: int = 5, context: int = 6,
                    max_dx: int = 0, max_dy: int = 0, min_shift: int = 3,
                    dy_penalty: float = 6.0, row_match: bool = True,
                    valid: np.ndarray | None = None):
    """Remplit `hole` par vote médian sur les `k` meilleurs décalages."""
    valid = (~hole) if valid is None else (valid & ~hole)
    offsets = find_offsets(rgb, hole, k, context, max_dx, max_dy, min_shift, dy_penalty,
                           valid)
    out = rgb.astype(np.float64).copy()
    if not offsets:
        return inpaint_diffusion(rgb, hole, valid=valid), []
    img = rgb.astype(np.float64)
    stack, weights = [], []
    for dy, dx, _err, _cov in offsets:
        shifted, inb = _shift_valid(img, dy, dx, np.nan)
        svalid, _ = _shift_valid(valid.astype(np.float64), dy, dx)
        usable = inb & (svalid > 0.5)
        s = shifted.copy()
        s[~usable] = np.nan
        stack.append(s)
        weights.append(usable)
    arr = np.stack(stack)                       # (k, h, w, 3)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", RuntimeWarning)
        med = np.nanmedian(arr, axis=0)
    filled = hole & np.isfinite(med).all(axis=-1)
    out[filled] = med[filled]

    if row_match:
        # recale le niveau de chaque ligne remplie sur le fond réel de la même ligne :
        # préserve le dégradé vertical du composant, que le décalage peut décaler.
        for y in range(rgb.shape[0]):
            fr = filled[y]
            if not fr.any():
                continue
            ref = valid[y]
            if ref.sum() < 4:
                continue
            delta = img[y][ref].mean(axis=0) - out[y][fr].mean(axis=0)
            out[y][fr] += delta

    rest = hole & ~filled
    if rest.any():
        tmp = out.copy()
        out = inpaint_diffusion(np.clip(tmp, 0, 255).astype(np.uint8), rest,
                                valid=valid | (hole & ~rest))
    return np.clip(out, 0, 255), offsets
