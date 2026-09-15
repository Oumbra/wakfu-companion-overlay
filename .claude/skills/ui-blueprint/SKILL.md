---
name: ui-blueprint
description: Relever une capture d'interface du client Wakfu et en produire une maquette cotée publiée en Artifact — blocs, tailles, gouttières, paddings, rayons, palette et échelle typographique mesurés sur l'image, puis un plan annoté qui sert d'entrée au portage egui. À utiliser quand on demande d'analyser, mesurer, « reverse-designer » ou spécifier une interface à partir d'une image (`assets/design-system/interfaces/*.png` notamment).
---

# Relevé d'interface

Regarder une capture et estimer les dimensions à l'œil produit des valeurs fausses et non
reproductibles. Ce skill mesure d'abord, interprète ensuite : la machine sort les positions,
gouttières, hauteurs d'encre et couleurs ; le modèle nomme les blocs, écrit la spec, et la
fait rendre en maquette cotée.

Outil : `python .claude/skills/ui-blueprint/scripts/uispec.py <commande>`
(Python 3 + Pillow + numpy ; réutilise `dslib` du skill `design-asset`).

## Déroulé

### 1. Regarder l'image

Ouvrir la capture avec l'outil Read. C'est le seul moyen de savoir *ce que sont* les blocs ;
les mesures ne disent que *où* ils sont.

### 2. Mesurer

```bash
U=.claude/skills/ui-blueprint/scripts/uispec.py
python $U layout   assets/design-system/interfaces/interface-options-video.png --depth 4
python $U edges    assets/design-system/interfaces/interface-options-video.png --top 24
python $U lines    assets/design-system/interfaces/interface-options-video.png --box 30,80,690,140
python $U palette  assets/design-system/interfaces/interface-options-video.png --box 30,80,690,140 --top 8
```

- **`layout`** — découpage XY récursif : un bloc par ligne, avec sa boîte, son padding réel
  et les gouttières qui l'ont produit. Sortie compacte par défaut (`--json` pour l'arbre
  complet). C'est le squelette de la mise en page.
- **`edges`** — arêtes horizontales et verticales dominantes : bords de panneaux, filets,
  séparateurs. `observed_steps` donne les écarts entre arêtes : c'est là qu'on lit le pas de
  grille réellement utilisé, plutôt que de postuler un 8 px.
- **`lines`** — bandes de texte d'une zone : hauteur d'encre, interlignes, étendue en x,
  luminance moyenne. La taille de corps vaut environ `hauteur / 0,72`.
- **`palette`** — couleurs dominantes d'une zone, avec leur part. Cadrer sur un seul élément
  à la fois : une palette calculée sur toute l'image ne dit rien d'utile.

`layout` est bruité sur les zones texturées : il propose des découpes, il ne décide pas.
Croiser avec `edges` et avec ce que montre l'image.

### 3. Écrire la spec

Un JSON décrivant l'interface : arbre de blocs cotés, jetons, observations. Format et
champs dans [`references/spec-format.md`](references/spec-format.md) ; exemple complet
dans [`references/exemple-confirm-box.json`](references/exemple-confirm-box.json).

Règles de rédaction :

- **Les boîtes sont en coordonnées absolues de la capture** — c'est ce que rendent les
  commandes de mesure, et cela évite les erreurs de conversion.
- Imbriquer les blocs selon la hiérarchie réelle : les cotes automatiques ne se calculent
  qu'entre frères. Une liste plate produit des cotes parasites entre blocs sans rapport.
- Ne consigner que ce qui a été mesuré ou vu. Une valeur inventée dans une spec de
  référence est pire que son absence — la signaler dans `notes` si elle reste incertaine.
- `notes` est l'endroit des irrégularités : deux boutons de largeurs différentes, un décor
  qui déborde du panneau, une bordure absente. Ce sont ces détails qui font échouer un
  portage, et ils ne tiennent dans aucun champ numérique.

### 4. Rendre et publier

```bash
python $U blueprint spec.json -o "$SCRATCHPAD/blueprint.html"
```

Puis publier le fichier avec l'outil Artifact — l'utilisateur travaille en terminal et ne
voit rien d'autre (CLAUDE.md). Le HTML est déjà un fragment conforme : `<title>`, `<style>`,
plan SVG coté, inventaire des blocs, jetons, observations ; aucune ressource externe.

Le plan dessine chaque bloc à l'échelle (`scale` dans la spec), avec son remplissage réel
quand `fill` est renseigné, et cote automatiquement les gouttières entre frères.

## Ce que le livrable sert à faire

La maquette n'est pas une illustration : c'est l'entrée du travail suivant. Un relevé
correct permet d'écrire le code egui avec des constantes justifiées, de repérer les valeurs
qui se répètent (donc les jetons du design system), et de comparer plus tard le rendu de
l'overlay au relevé plutôt qu'à un souvenir de la capture.

Assets isolés (boutons, champs, icônes) : c'est le skill `design-asset` qui les prépare, et
son `analyze` donne le rayon et les couleurs d'un composant seul avec plus de précision.
