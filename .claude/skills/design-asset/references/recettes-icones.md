# Icônes — réglages étalonnés

Jeu d'essai constitué le 2026-09-09 sur les 11 boutons-icônes de
`assets/design-system/icons/`. Il sert à deux choses : retrouver le réglage d'une icône
déjà traitée, et vérifier après modification du code qu'aucun cas ne régresse.

Les captures d'origine (bouton entier) sont dans l'historique git, commit
`chore: renomme les captures d'icônes` — c'est là qu'il faut les reprendre pour rejouer
une extraction.

## Les trois familles

Le réglage se déduit du **contraste du glyphe par rapport à son bouton**, pas de ce que
l'icône représente.

| Famille | Aspect | Options |
| --- | --- | --- |
| Glyphe clair sur bouton brun | blanc cerné de sombre sur hachures brunes | aucune (défauts) |
| Bouton sombre, proche du décor | bouton presque noir, glyphe beige | `--tol 4` |
| Glyphe sombre sur bouton doré | noir sur doré clair texturé | `--polarity dark --keep center --floor 45` |

## Réglages retenus

Tous avec `--from-button`, sortie en taille native (pas de `--size`).

| Icône | Options supplémentaires | Glyphe |
| --- | --- | --- |
| `icon-bag-in` | — | 14 × 20 |
| `icon-bag-out` | — | 16 × 16 |
| `icon-filter` | — | 16 × 16 |
| `icon-lock` | — | 16 × 18 |
| `icon-order` | — | 16 × 16 |
| `icon-pact` | — | 17 × 17 |
| `icon-sort` | — | 19 × 16 |
| `icon-delete` | `--tol 4` | 12 × 14 |
| `icon-triangle-right` | `--tol 4` | 8 × 10 |
| `icon-chevron-down` | `--polarity dark --keep center --floor 45` | 16 × 9 |
| `icon-save` | `--tol 4 --polarity dark --keep center --floor 45` | 14 × 16 |

## Ce que ce jeu d'essai a appris

**`--k` ne sert à rien sur une icône.** Le seuil vaut `max(k × MAD, floor)` et, sur le fond
d'un bouton, le MAD est si faible que `floor` l'emporte toujours : passer `--k` de 5 à 9 ne
change pas un pixel. C'est `--floor` qu'il faut monter (45 sur les boutons dorés, dont les
hachures sont bien plus contrastées que sur les bruns).

**Le `roi_inset` n'est pas optionnel.** Sans retrait de la couronne de bordure, le glyphe
détecté couvrait tout le bouton (`glyph_bbox` = `[0, 0, 36, 36]`) : la bordure sombre du
bouton, plus étendue que le glyphe, faisait basculer `--polarity auto` sur `dark` et
emportait la texture avec elle. Le symptôme est reconnaissable — une bbox de glyphe qui
fait la taille du bouton.

**Les tailles natives sont conservées volontairement.** Les glyphes vont de 8 × 10 à
19 × 16 : ces écarts sont dans le jeu (un triangle de lecture est plus petit qu'une paire
de flèches de tri) et les normaliser tous à 24 px détruirait ce rapport tout en
rééchantillonnant une image de 16 px. Ajouter `--size 24 --padding 2` seulement si une
grille d'icônes homogène est explicitement voulue.

**Le contour sombre du glyphe est conservé.** Il fait partie du dessin en jeu ; sur fond
clair il se voit nettement, ce qui est fidèle et non un défaut de détourage.
