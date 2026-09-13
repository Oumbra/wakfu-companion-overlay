# Curseur Wakfu

Curseur de souris du jeu, isolé pixel par pixel depuis un enregistrement d'écran natif
(13/09/2026, 30 i/s), par soustraction du fond puis moyenne par phase sur 85 images où le
curseur est immobile. Aucune interpolation : ce sont les pixels affichés par le jeu.

| Fichier | Phase | Taille |
|---|---|---|
| `wakfu-cursor-idle.png` | repos (crème) | 26 × 33 px, RGBA |
| `wakfu-cursor-flash.png` | éclair (cyan) | 26 × 33 px, RGBA |

- Curseur nu : 24 × 31 px ; les bitmaps ajoutent une marge transparente de 1 px.
- Point chaud (pointe) : pixel (2, 0) du bitmap avec marge.
- Alpha binaire (masque de différence avec le fond).

## Animation

Bascule entre les deux bitmaps, sans fondu (changement entre deux images consécutives) :

| Phase | Durée |
|---|---|
| éclair (cyan) | 16 images · ≈ 533 ms |
| repos (crème) | 16 images · ≈ 533 ms |

Période ≈ 1,07 – 1,09 s (32,6 images en moyenne sur cinq cycles), rapport cyclique 50 %.

## Palette

Le dessin (face claire, bande d'ombre, ombre foncée, contour) est le même dans les deux phases ;
seuls les tons changent.

| Ton | Repos | Éclair |
|---|---|---|
| clair | `#f9f8c6` | `#dcf9f9` |
| ombre | `#afa277` | `#50c3cb` |
| ombre foncée | `#68624a` | `#287c82` |
| contour | `#090908` | inchangé |
