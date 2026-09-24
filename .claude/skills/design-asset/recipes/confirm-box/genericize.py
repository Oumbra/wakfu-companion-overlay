"""Version generique de la boite de confirmation detouree.

Retire ce que la capture a d'incruste — la question (« Etes-vous sur(e)... ») et les deux
boutons, libelles compris — et reconstruit le fond du corps a leur place. Ce qui reste est
le **chassis** : corps, liseres, filigrane des coins, crete et pied. Le texte et les
boutons sont rendus par egui (`design::button`), jamais par la texture : un asset par
libelle est nommement interdit par le skill `ui-component`.

Reconstruction : mediane par ligne sur les pixels valides du corps. Le fond du corps est
uni horizontalement (89,90,91 a 88,90,86 du haut vers le bas), donc une ligne entiere se
retrouve exactement, la ou un vote de decalages irait chercher la texture d'une lettre
voisine. Le filigrane des coins, lui, n'est dans aucune zone effacee : il est conserve.
"""
import sys

import numpy as np
from PIL import Image

# Coordonnees dans le repere de l'image DETOUREE (origine (13, 9) dans la capture).
CROP_ORIGIN = (13, 9)
PANEL = (0, 36, 420, 184)        # corps, bord exterieur
INNER = (4, 40, 416, 178)        # zone de fond sure (hors liseres)
MESSAGE = (44, 58, 378, 102)     # boite reelle de la question (mesuree)
BUTTONS = ((40, 118, 208, 154), (216, 118, 384, 154))


def main(src, out_path):
    img = np.asarray(Image.open(src).convert("RGBA")).astype(np.float64)
    h, w = img.shape[:2]
    rgb, alpha = img[..., :3], img[..., 3]

    px0, py0, px1, py1 = PANEL
    ix0, iy0, ix1, iy1 = INNER

    hole = np.zeros((h, w), bool)
    for bx0, by0, bx1, by1 in BUTTONS:
        hole[by0:by1, bx0:bx1] = True

    # La question : lettres claires DOUBLEES d'une ombre portee sombre. Un seuil qui ne
    # garde que le clair efface les lettres et laisse leurs ombres — a contraste force, la
    # bande du message reste barbouillee. L'ecart est donc pris en valeur absolue, et la
    # boite est serree sur le texte mesure pour ne pas manger le filigrane des coins.
    mx0, my0, mx1, my1 = MESSAGE
    band = rgb[my0:my1, mx0:mx1].mean(axis=2)
    ink = np.abs(band - np.median(band))
    text = np.zeros((h, w), bool)
    text[my0:my1, mx0:mx1] = ink > 11
    for _ in range(2):
        text |= np.roll(text, 1, 0) | np.roll(text, -1, 0) | np.roll(text, 1, 1) | np.roll(text, -1, 1)
    text[:my0, :] = False
    text[my1:, :] = False
    text[:, :mx0] = False
    text[:, mx1:] = False
    hole |= text

    valid = np.zeros((h, w), bool)
    valid[iy0:iy1, ix0:ix1] = True
    valid &= ~hole

    out = rgb.copy()
    for y in range(py0, py1):
        row = valid[y]
        if row.sum() < 8:
            continue
        fill = np.median(rgb[y][row], axis=0)
        out[y][hole[y]] = fill

    res = np.concatenate([out, alpha[..., None]], axis=2)
    Image.fromarray(np.clip(res, 0, 255).astype(np.uint8)).save(out_path)
    print({"output": out_path, "removed_px": int(hole.sum()),
           "text_px": int(text.sum()), "size": [w, h]})


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
