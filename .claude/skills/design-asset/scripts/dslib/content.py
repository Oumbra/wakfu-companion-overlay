"""Détection du contenu incrusté dans un composant (label, placeholder, valeur, icône).

Principe : le fond d'un composant Wakfu est régulier — dégradé vertical + texture de
hachures de faible amplitude. Le contenu incrusté s'en écarte fortement en luminance.
On estime donc le fond ligne par ligne (médiane robuste, recalculée en excluant le
contenu déjà détecté), puis on seuille l'écart avec un seuil adaptatif (MAD).
"""
from __future__ import annotations

import numpy as np

from .core import bbox_of, connected_components, dilate, luma, rgb_of


def side_bands(roi: np.ndarray, side: int) -> np.ndarray:
    """Bandes latérales du ROI : les `side` premiers et derniers pixels de chaque ligne.

    Sur un bouton-icône, le glyphe occupe une grande part de la ligne : une médiane
    calculée sur la ligne entière est contaminée par le glyphe. Les bandes latérales,
    elles, sont presque toujours du fond."""
    h, w = roi.shape
    out = np.zeros_like(roi)
    for y in range(h):
        xs = np.nonzero(roi[y])[0]
        if len(xs) == 0:
            continue
        out[y, xs[:side]] = True
        out[y, xs[-side:]] = True
    return out


def _row_background(lum: np.ndarray, roi: np.ndarray, exclude: np.ndarray,
                    side: int = 0):
    """Médiane et MAD par ligne, calculées sur `roi & ~exclude`.

    `side` > 0 restreint l'estimation aux bandes latérales (fonds à glyphe large)."""
    h, w = lum.shape
    med = np.zeros(h)
    mad = np.zeros(h)
    use = roi & ~exclude
    if side > 0:
        use = use & side_bands(roi, side)
    for y in range(h):
        vals = lum[y][use[y]]
        if len(vals) < 4:
            vals = lum[y][roi[y]]
        if len(vals) == 0:
            med[y], mad[y] = 0.0, 1.0
            continue
        m = float(np.median(vals))
        med[y] = m
        mad[y] = max(float(np.median(np.abs(vals - m))), 1.0)
    return med, mad


def detect_content(rgba: np.ndarray, roi: np.ndarray | None = None, polarity: str = "auto",
                   k: float = 5.0, floor: float = 22.0, iterations: int = 3,
                   min_area: int = 6, grow: int = 2, box=None, side: int = 0):
    """Masque du contenu incrusté.

    polarity : "light" (texte clair), "dark" (texte sombre), "auto" (le plus contrasté).
    box      : (x0, y0, x1, y1) pour restreindre / forcer la zone à traiter.
    side     : n > 0 → fond estimé sur les bandes latérales (glyphe large / icône).
    """
    rgb = rgb_of(rgba)
    lum = luma(rgb)
    h, w = lum.shape
    if roi is None:
        roi = rgba[..., 3] > 128
    if box is not None:
        bx = np.zeros_like(roi)
        x0, y0, x1, y1 = box
        bx[y0:y1, x0:x1] = True
        roi = roi & bx

    bg_roi = roi if box is None else (rgba[..., 3] > 128)
    mask = np.zeros((h, w), bool)
    for it in range(iterations):
        med, mad = _row_background(lum, bg_roi, mask, side)
        # Le MAD n'a de sens qu'une fois le contenu exclu. Tant que rien n'est detecte,
        # il est gonfle par ce qu'on cherche : sur un libelle qui occupe la moitie de la
        # ligne il monte a ~35, `k * mad` depasse 190, et plus rien ne franchit le seuil
        # — la boucle ne demarre jamais. La premiere passe s'en tient donc au plancher.
        thr = (np.full(h, floor) if it == 0 else np.maximum(k * mad, floor))[:, None]
        d = lum - med[:, None]
        light = roi & (d > thr)
        dark = roi & (-d > thr)
        if polarity == "light":
            mask = light
        elif polarity == "dark":
            mask = dark
        elif polarity == "both":
            mask = light | dark
        else:
            mask = light if light.sum() >= dark.sum() else dark

    if min_area > 1 and mask.any():
        labels, n = connected_components(mask)
        sizes = np.bincount(labels.ravel())
        keep = np.zeros(sizes.shape, bool)
        keep[1:] = sizes[1:] >= min_area
        mask = keep[labels]

    if grow:
        # 2e passe « halo » : le texte du jeu porte un contour/une ombre plus étendus
        # que le glyphe lui-même. On les récupère avec un seuil abaissé, mais seulement
        # au voisinage immédiat du contenu déjà trouvé.
        med, mad = _row_background(lum, bg_roi, mask, side)
        near = dilate(mask, grow + 2) & roi
        soft = np.maximum(k * 0.4 * mad, floor * 0.45)[:, None]
        halo = near & (np.abs(lum - med[:, None]) > soft)
        mask = (dilate(mask, grow) | halo) & roi
    return mask


def content_report(mask: np.ndarray) -> dict:
    labels, n = connected_components(mask)
    boxes = []
    for i in range(1, n + 1):
        bb = bbox_of(labels == i)
        if bb:
            boxes.append({"box": list(bb), "area": int((labels == i).sum())})
    boxes.sort(key=lambda b: b["box"][0])
    return {"components": len(boxes), "boxes": boxes,
            "bbox": list(bbox_of(mask)) if mask.any() else None,
            "pixels": int(mask.sum())}
