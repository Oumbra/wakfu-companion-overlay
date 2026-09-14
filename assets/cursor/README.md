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
| `wakfu-cursor-text.png` | texte (I-beam, fixe) | 19 × 30 px, RGBA |

- Curseur nu : 24 × 31 px ; les bitmaps ajoutent une marge transparente de 1 px.
- Point chaud (pointe) : pixel (2, 2) du bitmap avec marge — premier pixel au moins à moitié opaque
  en lisant de haut en bas (`cursor::hotspot`, seuil `HOTSPOT_ALPHA_MIN`).
- Re-détourés à la main le 13/09/2026 (v2, mainteneur) : contour affiné, bords lissés (alpha sur
  plusieurs niveaux). La première découpe automatique (alpha binaire, contour de 3 px) reste dans
  l'historique git.

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

## Curseur texte

I-beam affiché au survol d'un champ de saisie, isolé de la même façon (enregistrement 68 × 48 px du
13/09/2026, fond = médiane des 42 images sans curseur, moyenne de 103 images immobiles). Curseur nu
17 × 28 px + marge de 1 px, alpha binaire, 308 pixels opaques en une seule composante, masque
symétrique gauche/droite et haut/bas.

- Contour opaque de 2 px (1 px retiré aux quatre angles), aplati en `#090908`.
- Hampe de 3 px : sombre `#414037`, crème `#efebc9`, ombre `#97957f` (colonnes 8-10 du bitmap).
- Empattements de 2 rangées (face claire `#fcf8d4` dessus, ombre `#c7c4a7` dessous), encoche
  noire au raccord avec la hampe (rangées 3 et 26).
- Le masque à 50 % ne suffit pas ici : le contour noir n'est qu'à ~20 du fond du champ (`#16171b`).
  Un pixel est contour si `max(RVB) < 14` et écart au fond > 10, corps si écart ≥ 15.
- Couleurs : luminance mesurée conservée (exacte en h264), teinte crème unique reconstruite
  (`G = 0,985 R`, `B = 0,841 R`) pour effacer le sous-échantillonnage chroma 4:2:0, luminances
  distantes de ≤ 5 fusionnées — 12 tons au total.
- Point chaud proposé au centre de la hampe crème : pixel (9, 15) du bitmap avec marge.
