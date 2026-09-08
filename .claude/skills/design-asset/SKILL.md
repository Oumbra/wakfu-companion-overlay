---
name: design-asset
description: Transformer une capture d'écran brute du client Wakfu en asset de design system exploitable — détourer un composant (fond transparent, coins arrondis mesurés), retirer un libellé ou une valeur incrustés et reconstruire la texture derrière, extraire une icône et la normaliser (16/24 px), harmoniser les tailles d'un lot de variantes en 9-slice, et publier une planche de contrôle visuelle en Artifact. À utiliser dès qu'il faut préparer, corriger ou générifier un fichier de `assets/design-system/` (y compris `icons/`).
---

# Préparation des assets du design system

Les images de `assets/design-system/` sont des captures brutes du jeu : décor autour du
composant, libellé incrusté (« Annuler », « Rechercher »), tailles incohérentes d'une
variante à l'autre. Ce skill fait le travail de retouche à la place de l'utilisateur.

Outil : `python .claude/skills/design-asset/scripts/dsimg.py <commande>`
(Python 3 + Pillow + numpy, déjà installés ; ni OpenCV ni ImageMagick ne sont requis).
Chaque commande écrit un rapport JSON sur stdout — ce sont ces mesures qui alimentent
ensuite le code egui (rayon, marges, padding interne, couleurs du dégradé).

## Toujours commencer par `analyze`

```bash
python .claude/skills/design-asset/scripts/dsimg.py analyze assets/design-system/button-secondary.png
```

Rend : bbox et taille du composant, `corner_radius`, marges de la capture, couleurs de
remplissage haut/milieu/bas, présence d'un dégradé vertical, bbox du contenu incrusté et
son padding — plus `tol_sweep`, la taille détectée pour plusieurs tolérances.

**Lire `tol_sweep` avant tout autre traitement.** Si les tailles s'effondrent au-delà d'un
certain seuil (ex. `"8": [8, 10]` alors que `"4": [36, 36]`), le composant est presque de
la teinte du décor : refaire toutes les commandes avec le `--tol` de la valeur stable.

## Recettes

### Générifier un bouton (cas de référence)

Détourage + retrait du libellé + reconstruction du fond, en une commande :

```bash
python .claude/skills/design-asset/scripts/dsimg.py genericize \
  assets/design-system/button-secondary.png -o assets/design-system/button-secondary.png
```

Vérifier dans le JSON que `removed.bbox` correspond bien au libellé (et pas à la moitié du
bouton), puis publier une planche (voir plus bas). Écraser la source est volontaire :
l'asset générique remplace la capture. En cas de doute, écrire d'abord dans le scratchpad.

### Détourer seulement

```bash
dsimg.py cutout IMG -o OUT [--tol 12] [--radius 4] [--inset 0.5] [--no-crop]
```

Le masque est un **rectangle arrondi ajusté**, pas le contour brut : le résultat a des
bords géométriques nets, antialiasés, et les couleurs du contour sont décontaminées du
gris du décor (sans quoi l'asset porte un halo sur un autre fond).

**Le liseré de bordure fait partie du composant et doit être conservé.** La propagation
seule l'avale (une fois entrée dedans en un point, elle le parcourt en entier : il est
homogène) ; une passe de récupération le réintègre anneau par anneau. Le JSON le
rapporte sous `border_recovery` : `ring_ratios` donne, pour chaque anneau, la fraction de
pixels qui tranchent sur le décor — un liseré fait le tour et sort à ~1,0, une simple zone
sombre du décor s'arrête bien plus bas. Si la bordure manque encore sur le résultat,
c'est ce chiffre qu'il faut regarder ; `--tol` plus bas résout la plupart des cas. Passer
`border=False` n'est utile que si la récupération déborde sur un décor très contrasté.

### Retirer un libellé, un placeholder, une valeur

```bash
dsimg.py strip IMG -o OUT [--polarity light|dark|both|auto] [--grow 2] [--box x0,y0,x1,y1]
```

`--box` est l'échappatoire quand la détection automatique déborde ou rate : coordonnées
dans le repère de l'image **détourée**, lues dans `analyze`.

`--roi-inset` (automatique par défaut) exclut une couronne de la détection : sans elle, la
bordure sombre du composant serait prise pour du contenu incrusté et effacée. Si le JSON
rend un `removed.bbox` qui couvre tout le composant, c'est exactement ce qui s'est passé —
augmenter `--roi-inset`.

### Extraire une icône

```bash
dsimg.py icon IMG -o OUT --from-button --size 24 [--keep center] [--padding 2]
```

`--from-button` détoure d'abord le bouton porteur, puis exclut une couronne de bordure de
la détection (`--roi-inset`, automatique) ; sans `--size`, la sortie garde la taille native
du glyphe, ce qui est le meilleur choix par défaut — les glyphes du jeu n'ont pas tous la
même taille, et cet écart est signifiant. Le contour sombre du glyphe est conservé : c'est
ainsi qu'il apparaît en jeu.

Le réglage se déduit du contraste du glyphe sur son bouton, pas de ce que l'icône
représente :

| Cas | Options |
| --- | --- |
| Glyphe clair sur bouton brun | aucune |
| Bouton sombre, proche du décor | `--tol 4` |
| Glyphe sombre sur bouton doré | `--polarity dark --keep center --floor 45` |

`--k` est sans effet ici : sur le fond d'un bouton le MAD est trop faible pour peser dans
`max(k × MAD, floor)`, c'est `--floor` qui pilote. Un `glyph_bbox` qui fait la taille du
bouton signale que la bordure a été prise pour le glyphe — augmenter `--roi-inset`.

Réglages étalonnés icône par icône dans
[`references/recettes-icones.md`](references/recettes-icones.md) : c'est aussi le jeu
d'essai à rejouer après toute modification du code de détection.

### Aligner les variantes sur une taille commune

```bash
dsimg.py align assets/design-system/button-{primary,secondary,danger}.png \
  -o assets/design-system --to 200x48 --insets 8,8,8,8 --fill stretch
```

Redimensionnement **9-slice** : coins, arrondis et bordures sont recopiés tels quels, seuls
les bandeaux médians sont étirés (`stretch`), répétés (`tile`) ou répétés en miroir
(`mirror`). `--insets` doit dépasser le rayon des coins + l'épaisseur de bordure. Sans
`--to`, la cible est la plus grande taille du lot.

`dsimg.py resize` fait la même chose sur une seule image.

## Vérification visuelle — obligatoire, et en Artifact

L'utilisateur travaille en terminal : une image lue en ligne lui est invisible (CLAUDE.md).
Après tout traitement, produire la planche et **la publier en Artifact** :

```bash
dsimg.py sheet "assets/design-system/button-secondary.png:/chemin/out.png" \
  -o "$SCRATCHPAD/planche.html" --title "Bouton secondaire générique" --scale 2
```

Chaque entrée est soit un chemin, soit `avant:après`. La planche montre le résultat sur
damier, fond sombre et fond clair, avec un zoom ×6 — les trois révèlent les défauts qu'un
seul fond cache (halo de bord, alpha résiduel, texture ratée). Publier ensuite le fichier
avec l'outil Artifact (le HTML produit est déjà un fragment conforme : `<title>`, `<style>`,
contenu, images en base64, aucune ressource externe).

## Comment ça marche (pour régler quand ça résiste)

- **Décor / composant** : propagation depuis les bords de l'image, arrêtée par une rupture
  locale de couleur > `--tol`. Un fond en dégradé est donc toléré, un composant de la même
  teinte que le décor ne l'est pas → baisser `--tol`.
- **Contenu incrusté** : le fond d'un composant Wakfu est un dégradé vertical + hachures de
  faible amplitude. On estime le fond ligne par ligne (médiane robuste, recalculée en
  excluant ce qui a déjà été détecté), puis on seuille l'écart à `max(--k × MAD, --floor)`.
  Une seconde passe récupère le halo/l'ombre du texte. Trop de pixels retirés → monter
  `--k` ou `--floor` ; libellé incomplet → les baisser, ou augmenter `--grow`.
- **Reconstruction du fond** : recherche des meilleurs décalages globaux, puis vote médian
  des 5 meilleurs (parenté : *content-aware fill* par statistiques d'offsets). Sur une
  texture régulière comme les hachures diagonales, cela reconstruit le motif — là où une
  diffusion donnerait un aplat. Le niveau de chaque ligne est recalé sur le fond réel de
  la même ligne pour préserver le dégradé vertical. `--method diffusion` reste disponible
  pour un fond lisse ou une zone valide trop petite.

Détail des modules dans [`references/pipeline.md`](references/pipeline.md).

## Garde-fous

- Ne jamais écraser un asset sans avoir regardé la planche : ces captures ne sont pas
  reproductibles à l'identique.
- Les mesures du JSON (rayon, padding, couleurs) valent d'être reportées dans le code egui
  ou dans le plan : c'est la moitié de l'intérêt du skill.
- Aucune commande n'invente de pixels hors de l'image source : ce qui est reconstruit
  provient toujours d'une autre zone de la même image.
