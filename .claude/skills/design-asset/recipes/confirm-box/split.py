"""Decoupe le chassis de la boite de confirmation en trois textures.

Un seul 9-slice ne peut pas rendre ce chassis : ses deux ornements — la crete en haut, le
filet en pied — sont **centres** et de largeur fixe, quand le corps, lui, s'etire. Figer des
marges horizontales assez larges pour contenir la crete (250 px de chaque cote sur un corps
de 420) ne laisse aucune bande mediane ; les laisser dans la bande mediane les etire. Les
deux sont faux. On separe donc :

  * corps   420 x 148  9-slice, marges 6 px (rayon 2 + biseau 2 + securite)
  * crete   250 x 53   posee centree en haut, jamais etiree
  * pied     98 x 9    posee centree en bas, jamais etiree

La crete descend 17 px DANS le corps : la pointe basse du losange s'arrete a y = 53 quand le
corps commence a y = 36. Le corps est donc reconstruit sous elle, par mediane de ligne sur
ses pixels hors bande — sans quoi il porterait une empreinte doree en plein milieu de sa
bande superieure, etiree avec elle.
"""
import sys

import numpy as np
from PIL import Image

SRC = "assets/design-system/confirm-box.png"

BODY = (0, 36, 420, 184)      # corps, bord exterieur
CREST = (85, 0, 335, 57)      # crete complete : bandeau + volutes + losange, cerne sombre de sa pointe compris
FOOT = (161, 184, 259, 193)   # filet de pied


def main(out_dir):
    img = np.asarray(Image.open(SRC).convert("RGBA")).astype(np.float64)
    h, w = img.shape[:2]

    cx0, cy0, cx1, cy1 = CREST
    crest = img[cy0:cy1, cx0:cx1].copy()
    Image.fromarray(np.clip(crest, 0, 255).astype(np.uint8)).save(f"{out_dir}/crest.png")

    fx0, fy0, fx1, fy1 = FOOT
    foot = img[fy0:fy1, fx0:fx1].copy()
    Image.fromarray(np.clip(foot, 0, 255).astype(np.uint8)).save(f"{out_dir}/foot.png")

    # Corps : l'empreinte de la crete est effacee et le fond repris ligne par ligne.
    bx0, by0, bx1, by1 = BODY
    body = img[by0:by1, bx0:bx1].copy()
    bh, bw = body.shape[:2]
    hole = np.zeros((bh, bw), bool)
    hole[: cy1 - by0, cx0 - bx0:cx1 - bx0] = True

    valid = np.zeros((bh, bw), bool)
    valid[:, 2:bw - 2] = True
    valid &= ~hole
    for y in range(bh):
        if valid[y].sum() < 8:
            continue
        body[y][hole[y], :3] = np.median(body[y][valid[y], :3], axis=0)
    # L'alpha du corps est conserve tel quel : ses quatre coins arrondis (rayon 2) doivent
    # rester transparents, et la zone reconstruite etait deja opaque.
    Image.fromarray(np.clip(body, 0, 255).astype(np.uint8)).save(f"{out_dir}/body.png")

    print({"body": [bw, bh], "crest": [cx1 - cx0, cy1 - cy0], "foot": [fx1 - fx0, fy1 - fy0],
           "crest_overlap_into_body": cy1 - by0})


if __name__ == "__main__":
    main(sys.argv[1])
