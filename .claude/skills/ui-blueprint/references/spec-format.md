# Format de la spec d'interface

Fichier JSON consommé par `uispec.py blueprint`. Toutes les coordonnées sont en pixels,
en repère absolu de la capture d'origine.

## Racine

| Champ | Type | Rôle |
| --- | --- | --- |
| `title` | texte | Nom de la page. Court, spécifique, sans explication accolée. |
| `subtitle` | texte | Une phrase : ce qu'est l'interface et à quoi sert le relevé. |
| `source` | texte | Chemin de la capture analysée. |
| `canvas` | `[w, h]` | Dimensions de la capture. |
| `scale` | nombre | Échelle de dessin du plan (1,5 à 2 pour une petite fenêtre). |
| `nodes` | liste | Arbre des blocs (voir plus bas). |
| `tokens` | objet | Jetons relevés : `color`, `space`, `radius`, `type`. |
| `notes` | liste de textes | Observations qui ne rentrent dans aucun champ. |

## Bloc (`nodes[]`)

| Champ | Type | Rôle |
| --- | --- | --- |
| `id` | texte | Identifiant court, en kebab-case. |
| `label` | texte | Nom affiché sur le plan et dans l'inventaire. |
| `box` | `[x0, y0, x1, y1]` | Boîte absolue, bord droit/bas exclus. |
| `role` | `surface` \| `group` \| `control` \| `text` \| `icon` | Détermine le style de tracé. |
| `radius` | nombre | Rayon des coins, en px. |
| `fill` | couleur CSS | Remplissage réel relevé ; omis, le bloc est dessiné en fil de fer. |
| `padding` | `[l, t, r, b]` | Padding interne mesuré. |
| `notes` | texte | Ce que le bloc a de particulier. |
| `children` | liste | Blocs contenus. |

`role` n'est pas décoratif : `surface` trace un trait plein neutre (panneau, fond),
`group` un trait tireté accentué (regroupement logique sans fond propre), `control` un
trait plein (bouton, champ), `text` un pointillé fin (bloc de texte), `icon` un pointillé
serré accentué.

## Cotes automatiques

Le rendu cote les gouttières **entre frères** : pour chaque parent, les enfants sont triés
sur chaque axe et l'écart entre deux voisins successifs est tracé s'il est positif. D'où
l'importance de l'imbrication — deux blocs sans relation de fratrie ne sont jamais cotés
ensemble, et deux blocs mis à plat par erreur le sont à tort.

Les paddings ne sont pas tracés : ils figurent dans l'inventaire, où ils se lisent mieux
qu'empilés sur le plan.

## Jetons (`tokens`)

```json
{
  "color":  {"surface": "#585958", "primary": "#e2c77a"},
  "space":  [4, 8, 13, 17, 27, 32],
  "radius": {"panel": "6", "control": "4"},
  "type":   [{"name": "message", "size": "14", "weight": "regular",
              "color": "#d9d5cc", "usage": "corps de la question, centré"}]
}
```

`space` se remplit avec les écarts **réellement observés**, triés — pas avec une échelle
théorique. Si les valeurs relevées ne forment pas une progression régulière, c'est une
information : le client de jeu n'applique pas de grille stricte, et le porter en supposant
un pas de 8 px décalerait tout.
