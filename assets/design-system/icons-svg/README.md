# Ébauches SVG des icônes — non utilisées par le manifeste

Ces 38 fichiers sont des **ébauches tracées automatiquement** depuis `../icons/*.png` par
`tools/design-system/vectorize_icons.py` (session du 2026-09-11, évaluation PNG vs SVG).
Aucune n'est embarquée par `crates/overlay-ui/src/design/assets.rs` : le manifeste continue
de charger les PNG.

Un seul `<path fill="currentColor">` par fichier, viewBox aux dimensions natives du PNG :
la teinte se pose par `color`, comme la teinte multiplicative des PNG aujourd'hui.

Fidélité au PNG d'origine (intersection sur union des alphas, re-rasterisation `resvg` à la
taille native) :

| Statut | Icônes |
| --- | --- |
| utilisables telles quelles (≥ 0,90) | les 28 autres |
| à retoucher, angles arrondis par le lissage (0,85 à 0,90) | chevron-down, option, pact, tick, volume-mute |
| à redessiner, traits fins semi-transparents perdus (< 0,85) | characters, eye, grid, info, search |

Régénérer : `pip install pillow numpy potracer && python tools/design-system/vectorize_icons.py`.
