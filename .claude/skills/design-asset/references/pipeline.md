# Anatomie du pipeline `dsimg`

Référence interne : à lire quand une commande donne un résultat inattendu et qu'il faut
intervenir dans le code plutôt qu'avec une option.

## Modules

| Fichier | Rôle |
| --- | --- |
| `dslib/core.py` | E/S RGBA, luminance, morphologie (`dilate`/`erode`), composantes connexes (union-find, deux passes), bbox. |
| `dslib/segment.py` | Séparation décor / composant, ajustement du rectangle arrondi, décontamination des bords, `cutout`. |
| `dslib/content.py` | Détection du contenu incrusté (libellé, valeur, glyphe) et rapport de composantes. |
| `dslib/inpaint.py` | Reconstruction du fond : moteur `offsets` (défaut) et moteur `diffusion`. |
| `dslib/icon.py` | Extraction de glyphe avec alpha progressif + démélange, recadrage, mise à l'échelle prémultipliée. |
| `dslib/scale9.py` | Redimensionnement 9-slice (`stretch` / `tile` / `mirror`). |
| `dslib/sheet.py` | Fragment HTML de contrôle (thème clair/sombre, damier, base64). |
| `dsimg.py` | CLI : `analyze`, `cutout`, `strip`, `genericize`, `icon`, `resize`, `align`, `sheet`. |

Aucune dépendance hors Pillow + numpy (le poste n'a ni OpenCV ni ImageMagick).

## 1. Séparation décor / composant — `segment.flood_background`

Propagation 4-connexe depuis les quatre bords de l'image. Un voisin rejoint le décor si
son écart au pixel courant est ≤ `tol` sur chaque canal. Le critère est **local**, donc un
décor en dégradé passe, mais un saut franc (bordure sombre du composant) arrête.

Ensuite : on bouche les trous (aucun décor ne peut être enclavé) et on garde la plus
grande composante.

### Récupération de la bordure — `segment.recover_border`

Le critère local a un angle mort : le liseré sombre du composant est **homogène**. Si la
propagation y entre en un seul point — un coin, un endroit où le contraste avec le décor
faiblit — elle le parcourt entièrement et classe toute la bordure en décor. Le composant
ressort alors amputé de son contour, ce qui se voit immédiatement sur l'asset.

On récupère donc après coup, anneau par anneau autour du masque. Un anneau n'est réintégré
que si la fraction de ses pixels s'écartant de la couleur du décor (seuil
`max(k × MAD, floor)`, décor mesuré dans une couronne à 2–8 px) dépasse `continuity`
(0,7 par défaut). Ce critère de **continuité** est le point clé : un liseré fait le tour du
composant et sort à ~1,0, alors qu'une zone sombre du décor ne touche qu'un côté et
s'effondre sous le seuil. Sur `button-secondary.png` : `[1.0, 0.954, 0.0]` — deux anneaux
gardés, le troisième rejeté net.

Conséquence en aval : la bordure étant dans le masque, elle est aussi dans le ROI de
`detect_content`, qui la prendrait pour du contenu sombre incrusté. D'où le `roi_inset`
(anneaux gardés + 2) appliqué avant la détection de contenu.

**Limites.** Un composant dont la teinte rejoint celle du décor laisse fuir la propagation
à l'intérieur — d'où `tol_sweep` dans `analyze`. Une capture contenant deux composants
n'en garde qu'un (le plus grand) : recadrer en amont.

## 2. Géométrie — `segment.fit_rounded_rect`

Le rayon est choisi en maximisant l'IoU **restreinte aux quatre zones de coin** : sur
l'ensemble du masque, tous les rayons plausibles donnent la même IoU à 0,1 % près et le
critère ne discrimine plus. `radius_fit_iou` dans le JSON est cette IoU de coin — en
dessous de ~0,85, le composant n'est probablement pas un rectangle arrondi.

L'alpha est rendu par suréchantillonnage ×8 (`rounded_rect_alpha`), ce qui donne un bord
antialiasé propre plutôt que le bord crénelé, et déjà mélangé au décor, de la capture.

## 3. Décontamination — `segment.bleed_edges`

Les pixels de contour d'une capture contiennent déjà du gris de décor. Recomposés sur un
autre fond avec un alpha partiel, ils produisent un liseré. On étale donc les couleurs
« sûres » (intérieur érodé) vers l'extérieur sur 3 passes **avant** d'appliquer l'alpha
géométrique.

## 4. Contenu incrusté — `content.detect_content`

1. Fond estimé ligne par ligne : médiane + MAD sur le ROI, en excluant le masque de
   l'itération précédente (3 itérations).
2. Seuil : `max(k × MAD, floor)` sur l'écart de luminance. Polarité `light` / `dark` /
   `both` / `auto` (la plus fournie des deux).
3. Filtrage par aire (`min_area`), dilatation `grow`, puis passe « halo » à seuil abaissé
   au voisinage immédiat pour récupérer contour et ombre du texte.

`side` > 0 restreint l'estimation du fond aux bandes latérales du ROI : indispensable pour
un bouton-icône, où le glyphe occupe une trop grande part de chaque ligne pour que la
médiane de ligne reste sur le fond. `icon` le calcule automatiquement (≈ largeur / 6).

## 5. Reconstruction — `inpaint.inpaint_offsets`

Pour chaque décalage candidat (dy, dx) :

- erreur = SSD moyenne entre l'image et sa version décalée, mesurée sur une **bande de
  contexte** autour du trou (pixels valides uniquement) ;
- couverture = fraction des pixels du trou dont la source décalée est valide ; les
  décalages sous 55 % sont écartés ;
- pénalité douce `dy_penalty × |dy|` : à qualité égale on préfère un décalage horizontal,
  qui ne traverse pas le dégradé vertical.

Les 5 meilleurs décalages, distants d'au moins 1 px les uns des autres, votent par médiane
par canal. `row_match` recale ensuite chaque ligne remplie sur la moyenne des pixels
valides de la même ligne. Ce qui reste non couvert part en diffusion.

Sur `button-secondary.png`, les décalages retenus sont horizontaux (`dx` ≈ 38 à 50, `dy` = 0) :
c'est la période des hachures diagonales, ce qui explique que le motif se prolonge.

## 6. Icône — `icon.extract_icon`

`alpha = clip((|L − fond| − seuil × soft) / (seuil × (1 − soft)))`, cantonné au glyphe
dilaté de 2 px. La couleur est ensuite démélangée : `pure = (observé − (1 − a) × fond) / a`,
ce qui retire la teinte du bouton porteur des pixels semi-transparents. `fit_box` recadre,
met à l'échelle en alpha prémultiplié (LANCZOS) et centre dans une boîte carrée.

## 7. 9-slice — `scale9.scale9`

Découpe en 3 × 3 selon `insets = (gauche, haut, droite, bas)`. Les quatre coins sont
recopiés, les bandeaux sont étirés le long d'un seul axe. `insets` doit couvrir rayon +
bordure, sinon l'arrondi est étiré. `tile` / `mirror` préservent une texture que `stretch`
déformerait, au prix d'une couture possible.
