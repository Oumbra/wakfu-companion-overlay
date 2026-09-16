# Composants — réglages étalonnés

Pendant de [`recettes-icones.md`](recettes-icones.md), pour tout ce qui n'est pas un
glyphe : boutons, boutons-icônes, barres d'onglets. Sert à retrouver le réglage d'un
asset déjà traité et à vérifier qu'une modification du code ne casse aucun cas.

Les captures d'origine sont dans l'historique git — commit
`style: ajoute les captures brutes onglets, pin et tick` pour le lot des onglets.

## Réglages retenus

| Asset | Commande | Options |
| --- | --- | --- |
| `button-secondary` | `genericize` | — |
| `button-secondary-hover` | `genericize` | — |
| `button-disabled` | `genericize` | — |
| `button-icon` | `genericize` | `--floor 45` |
| `button-icon-hover` | `genericize` | `--floor 45` |
| `button-icon-disabled` | `genericize` | — |
| `icon-tabs` | `genericize` | `--tol 3 --roi-inset 1 --grow 3 --floor 14 --parts 2,2,63,42 69,2,131,42 137,2,199,42 205,2,266,42` |
| `tabs-with-first-tab-active` | `strip` | `--parts 0,0,261,44 263,0,521,44 522,0,782,44` |
| `tabs-with-first-tab-active-and-hover-2nd-tab` | `strip` | idem |
| `switch-first-slot-active` | `cutout --tol 8` puis `strip` | `--roi-inset 6 --grow 2 --parts 0,0,42,44 44,0,87,44` |
| `switch-second-slot-active` | `cutout --tol 8` puis `strip` | `--roi-inset 6 --grow 2 --parts 0,0,44,44 46,0,88,44` |
| `switch-first-slot-active-and-second-slot-hover` | `cutout --tol 8` puis `strip` | `--roi-inset 6 --grow 2 --parts 0,0,42,44 44,0,88,44` |

## Ce que ce lot a appris

**La zone source de la reconstruction doit exclure la bordure.** Sur un bouton-icône de
36 px, le glyphe occupe le centre et les seuls décalages disponibles vont chercher leurs
pixels à ±15 px — c'est-à-dire dans le liseré sombre. Le résultat portait un fantôme
noirâtre en plein milieu, et le recalage par ligne empirait le mal en calculant son niveau
de référence sur une ligne qui contient ce même liseré. La couronne exclue de la détection
(`--roi-inset`) est désormais exclue aussi de la source et de la référence.

**Un libellé qui occupe la moitié de la ligne bloque sa propre détection.** Le seuil vaut
`max(k × MAD, floor)` ; sur les lignes de texte de `button-disabled` le MAD monte à 39, le
seuil à 195, et plus rien ne le franchit. La boucle d'exclusion itérative ne peut pas
démarrer : elle attend une première détection qui n'arrive jamais. La première passe s'en
tient donc au plancher, et les suivantes reprennent le seuil adaptatif une fois le texte
exclu. Symptôme à reconnaître : `removed.pixels` ridiculement bas (543 px pour une phrase
entière) et un masque réduit aux hampes et aux points.

**Un libellé pleine largeur se répare verticalement.** Il ne laisse aucune texture propre
à sa gauche ni à sa droite : tous les décalages horizontaux recopient d'autres lettres.
`--max-dy` vaut donc par défaut la hauteur du masque, ce qui autorise le saut par-dessus
le libellé ; la pénalité sur `dy` conserve la priorité à l'horizontale quand elle marche.

**Une barre d'onglets se traite onglet par onglet.** Le fond est estimé ligne par ligne :
une ligne qui traverse l'onglet actif (beige clair) puis deux onglets inactifs (gris
sombre) n'a pas de médiane commune, et la détection ne rend plus rien d'exploitable.
Deux découpages selon la capture :

- onglets **séparés par du décor** → `--components all`, un rectangle arrondi ajusté sur
  chacun ;
- onglets **dans un cadre commun** (`tabs-with-first-tab-active` : séparateurs de 2 px,
  coins arrondis seulement à l'extérieur) → `--parts` à la main. La propagation ne les
  sépare pas, et il n'y a rien à détourer puisque la capture *est* l'asset.

**`icon-tabs` se détoure d'un bloc, pas onglet par onglet.** Sa bordure (28) est à peine
plus sombre que le décor (35) : la récupération de bordure la refuse (`ring_ratios`
[0,023]) et `--components all` rend quatre cases sans cadre. À `--tol 3` la propagation
s'arrête sur le cadre et rend la barre entière — c'est le `tol_sweep` qui le montre.
Ensuite seulement, `--parts` sépare les quatre cases pour le retrait des glyphes.

**Le contraste de la texture pilote `--floor`, y compris entre variantes.** Les
bouton-icônes brun et hover demandent `--floor 45` : à 22, le masque avale la bande
diagonale claire, plus aucun décalage ne passe le critère de couverture et la
reconstruction retombe sur la diffusion. La variante `disabled`, elle, est grise et peu
contrastée : à 45 le manche du marteau échappe au masque et laisse une traînée. Deux
variantes du même composant, deux seuils.

**Les coins arrondis sont le premier endroit où le détourage casse.** Deux causes, toutes
deux corrigées, et toutes deux invisibles ailleurs que dans un coin :

- la récupération de bordure absorbait l'anneau *entier* dès que sa continuité était
  atteinte. La dilatation par un carré ajoute, en diagonale d'un coin arrondi, des pixels
  qui sont du décor : la boîte gonflait de 2 px, le rayon ajusté tombait à 4 au lieu de 6,
  et l'alpha laissait passer un éclat de décor à la place du liseré noir. Ce sont
  désormais les pixels qui tranchent, un à un, qui entrent — la continuité ne décide plus
  que si l'anneau *est* un liseré. Sur `button-icon`, `radius_fit_iou` passe de 0,93 à
  0,97 ;
- la décontamination réécrivait le pixel opaque le plus extérieur avec la moyenne de ses
  voisins. Sur un bord droit ces voisins sont du liseré, donc rien ne bouge ; dans un coin
  la moitié du voisinage est du remplissage, et le noir s'éclaircissait.

Le symptôme à reconnaître sur la planche : dans la vignette des coins, un angle beige ou
gris là où le bord droit est noir, ou un liseré qui s'amincit en approchant de l'arc.
`icon-tabs` était le cas le plus visible — sa barre de 268 px porte un cadre épais, et le
coin en perdait la moitié.

## Le switch de genre (2026-09-16) — un biseau n'est pas un libellé

Deux captures du même sélecteur à deux cases (♂ actif, puis ♀ actif). Le détourage est sans
histoire (`radius_fit_iou` 1,0, liseré récupéré sur deux anneaux, `--tol 8`). Le piège est dans
`analyze` : la case active porte un **biseau clair de 2 px en haut et en bas**, et la détection de
contenu le rend comme deux « composants » de 44 × 8 en plus des glyphes. Ce sont des bandes de
texture, pas du contenu incrusté : `--roi-inset 6` les sort de la détection (les glyphes sont à
y 13..31, il reste de la marge), et `removed.bbox` retombe sur les seuls glyphes (18 × 18 et
14 × 20 avec `--grow 2`).

**Case par case, comme une barre d'onglets** (`--parts`), en excluant les deux colonnes du
séparateur : les deux cases n'ont pas le même fond (kaki clair / gris-brun), une passe unique
n'aurait pas de médiane commune.

**Les deux captures ne font pas la même largeur** (87 et 88) : la case inactive y mesure 41 et
42 px, l'ombre intérieure côté liseré tenant sur 1 ou 2 px selon le rendu du jeu. Égalisées à 88
par duplication d'une colonne de remplissage de la case inactive — texture uniforme, invisible —
pour que les quatre cases découpées ensuite (`switch-slot-*.png`, voir le catalogue des
composants) aient des largeurs cohérentes d'un état à l'autre.

**Troisième capture, le survol** (`switch-first-slot-active-and-second-slot-hover`, ♂ actif et
souris sur ♀) : 87 px comme la première, même recette, même égalisation à 88 (colonne 60
dupliquée). Elle n'est découpée en rien : la case survolée mesure identique à la case active
(écart moyen 1,0/255, biseaux compris, sans ombre intérieure), le composant réutilise donc les
textures actives pour l'état survolé. Le fichier reste comme référence de comparaison.
