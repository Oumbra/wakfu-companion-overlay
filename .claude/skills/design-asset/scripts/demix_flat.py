"""Démélange direct fond → glyphe d'une icône d'un seul ton posée sur un décor uni.

Chaque pixel de la boîte du glyphe est projeté sur le segment couleur-fond → couleur-glyphe ;
sa position donne l'alpha. Les pixels d'ombre portée (plus sombres que le fond) projettent
en négatif et tombent à 0. Sortie : glyphe blanc, alpha porteur de l'antialiasing.
"""
import json
import sys

import numpy as np
from PIL import Image

src, dst = sys.argv[1], sys.argv[2]
x0, y0, x1, y1 = (int(v) for v in sys.argv[3].split(","))
alpha_floor = float(sys.argv[4]) if len(sys.argv) > 4 else 0.08

img = np.array(Image.open(src).convert("RGBA"))[..., :3].astype(np.float64)
h, w = img.shape[:2]

# Fond : médiane du décor hors de la boîte du glyphe (élargie de 1 px pour l'ombre).
mask = np.ones((h, w), bool)
mask[max(0, y0 - 1):y1 + 1, max(0, x0 - 1):x1 + 1] = False
bg = np.median(img[mask], axis=0)

crop = img[y0:y1, x0:x1]
lum = crop.mean(axis=-1)
# Glyphe : médiane des pixels francs (luminance au-dessus du 90e percentile).
fg = np.median(crop[lum >= np.percentile(lum, 90)], axis=0)

d = fg - bg
proj = ((crop - bg) * d).sum(-1) / (d @ d)
alpha = np.clip(proj, 0.0, 1.0)
resid = np.sqrt(((crop - (bg + alpha[..., None] * d)) ** 2).sum(-1))
alpha[alpha < alpha_floor] = 0.0

out = np.zeros((y1 - y0, x1 - x0, 4), np.uint8)
out[..., :3] = 255
out[..., 3] = np.round(alpha * 255).astype(np.uint8)
Image.fromarray(out).save(dst)

print(json.dumps({
    "bg": [round(v, 1) for v in bg], "fg": [round(v, 1) for v in fg],
    "size": [x1 - x0, y1 - y0],
    "opaque": int((alpha >= 0.98).sum()), "partial": int(((alpha > 0) & (alpha < 0.98)).sum()),
    "residual_max": round(float(resid[alpha > 0].max()), 1),
    "residual_mean": round(float(resid[alpha > 0].mean()), 1),
}, indent=2))
for row in out[..., 3]:
    print(" ".join(f"{v:3d}" for v in row))
