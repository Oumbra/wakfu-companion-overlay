# Curseur Wakfu

Embarqués par `crates/overlay-ui/src/cursor.rs` (curseur affiché à la place du curseur système sur
les overlays interactifs, clignotement en mode « main » — voir §6.3 bis du plan). Le point chaud
est déduit de l'image à l'exécution : ces fichiers peuvent être remplacés par un détourage plus
précis sans modifier le code, tant que la pointe reste en haut à gauche.

Curseur de souris du jeu, isolé pixel par pixel depuis un enregistrement d'écran natif
(13/09/2026, 30 i/s), par soustraction du fond puis moyenne par phase sur 85 images où le
curseur est immobile. Aucune interpolation : ce sont les pixels affichés par le jeu.

| Fichier | Phase | Taille |
|---|---|---|
| `wakfu-cursor-idle.png` | repos (crème) | 26 × 33 px, RGBA |
| `wakfu-cursor-flash.png` | éclair (cyan) | 26 × 33 px, RGBA |
| `wakfu-cursor-move.png` | déplacement (croix fléchée, fixe) | 33 × 33 px, RGBA |

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

## Curseur de déplacement

Croix à quatre flèches affichée pendant le glissement d'une fenêtre, isolée de la même façon
(enregistrement 68 × 50 px du 13/09/2026, fond = médiane des images sans curseur, moyenne de
20 images immobiles, masque à 50 % de couverture). Curseur nu 31 × 31 px + marge de 1 px, alpha
binaire (pixels de bord mélangés retirés, contour aplati en noir), quatre jours
transparents entre les bras. Pas d'animation : couleur constante sur les
113 images où il est visible. Tons `#e2dfb8` (clair), `#a29b69` (ombre), `#090908` (contour).
Point chaud présumé au centre, pixel (16, 16) du bitmap avec marge.
