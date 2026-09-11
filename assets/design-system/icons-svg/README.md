# Ébauches SVG des icônes — non utilisées par le manifeste

Ces 38 fichiers sont des **ébauches tracées automatiquement** depuis `../icons/*.png` par
`tools/design-system/vectorize_icons.py` (évaluation PNG vs SVG des 2026-09-11).
Aucune n'est embarquée par `crates/overlay-ui/src/design/assets.rs` : le manifeste continue
de charger les PNG.

## Ce qu'elles sont

Un groupe `fill="currentColor"` de contours empilés, viewBox aux dimensions natives du PNG :
la teinte se pose par `color`, comme la teinte multiplicative des PNG aujourd'hui.

**Multi-niveaux** (37 icônes) : l'alpha est tracé à dix seuils et les dix contours sont
empilés avec une opacité calibrée sur l'alpha moyen de chaque bande. Ça conserve le modelé
du PNG — ombres, arrondis d'antialiasing, silhouettes fantômes de `characters` — que la
première passe (silhouette à un seul seuil) bouchait entièrement. Agrandi ×4, le rendu est
à 5,7/255 d'un agrandissement Lanczos du PNG (silhouette : 19,9 ; lissage bilinéaire
d'egui : 6,6). Contreparties : 6 à 32 Ko par fichier, et un léger halo sur les bords les
plus francs à taille native (12/255 d'écart moyen).

**Silhouette** (`icon-minus` seule) : un contour au seuil 0,5. La barre de 14 × 2 s'amincit
en multi-niveaux (30/255 d'écart agrandi contre 8,4).

## Ce qu'elles ne sont pas

Un SVG fidèle à un PNG de 12 px reste, par construction, un agrandissement **doux** : il
conserve le pixel de transition, donc le flou proportionnel. Des glyphes nets à ×3 demandent
un redessin vectoriel à la main ; ces fichiers peuvent alors servir de calque de départ
(`--levels 1` donne la silhouette, plus commode à retoucher).

Régénérer : `pip install pillow numpy potracer && python tools/design-system/vectorize_icons.py`.
