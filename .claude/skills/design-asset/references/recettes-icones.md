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
| Glyphe à ombre portée | halo doux de 2–3 px autour du glyphe | `--floor 45` |

## Réglages retenus

Tous avec `--from-button`, sortie en taille native (pas de `--size`) et teinte blanche
appliquée en fin de pipeline (défaut).

| Icône | Options supplémentaires | Glyphe |
| --- | --- | --- |
| `icon-bag-in` | — | 10 × 16 |
| `icon-bag-out` | — | 12 × 12 |
| `icon-filter` | — | 12 × 12 |
| `icon-lock` | — | 12 × 14 |
| `icon-pact` | — | 14 × 13 |
| `icon-sort` | — | 16 × 12 |
| `icon-order` | `--floor 45` | 14 × 14 |
| `icon-delete` | `--tol 4` | 12 × 14 |
| `icon-triangle-right` | `--tol 4` | 8 × 10 |
| `icon-chevron-down` | `--polarity dark --keep center --floor 45` | 14 × 8 |
| `icon-save` | `--tol 4 --polarity dark --keep center --floor 45` | 12 × 12 |
| `icon-tick` | `--polarity dark --keep center --floor 45` | 12 × 9 |
| `icon-pin` | *sans* `--from-button` | 14 × 14 |
| `icon-trophy` | *sans* `--from-button` | 22 × 22 |
| `icon-book` | *sans* `--from-button` | 22 × 18 |
| `icon-repeat` | *sans* `--from-button` | 22 × 22 |
| `icon-calendar` | *sans* `--from-button` | 22 × 22 |
| `icon-kamas` | *sans* `--from-button` | 14 × 12 |
| `icon-xp` | *sans* `--from-button` | 16 × 11 |
| `icon-info` | *sans* `--from-button` | 27 × 28 |

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
16 × 12 : ces écarts sont dans le jeu (un triangle de lecture est plus petit qu'une paire
de flèches de tri) et les normaliser tous à 24 px détruirait ce rapport tout en
rééchantillonnant une image de 16 px. Ajouter `--size 24 --padding 2` seulement si une
grille d'icônes homogène est explicitement voulue.

**Le contour sombre du glyphe est conservé pendant l'extraction**, puis absorbé par la
teinte. Il fait partie du dessin en jeu, et sa présence pendant le détourage est ce qui
donne la bonne silhouette ; une fois l'icône blanche, il devient du blanc lui aussi.

**La teinte ne se pose pas à plat.** Repeindre tous les pixels visibles en blanc détruit
le dessin : sur `icon-bag-out` la flèche interne disparaît, sur `icon-pact` les traits du
masque se referment en pâté, et sur `icon-plus` le cerne sombre — devenu blanc — épaissit
la croix. Le mode `luma` transfère la luminance dans l'alpha, ce qui préserve le modelé :
un gris à mi-chemin devient blanc à moitié transparent.

**La polarité se lit sur le bord, pas sur la moyenne.** Un premier critère fondé sur la
luminance médiane classait `icon-plus` comme un glyphe sombre, parce que son cerne couvre
plus de pixels que la croix qu'il entoure — le résultat était une croix creuse. Comparer
le cœur de la silhouette à son bord range correctement les 16 icônes.

**Toutes les captures ne sont pas des boutons.** `icon-pin` est un glyphe posé à même le
décor sombre, sans bouton porteur : `--from-button` n'a rien à détourer et fausse tout.
Le glyphe est alors cherché dans l'image entière, ce qui marche parce que le décor est
uniforme. Signe distinctif dans `analyze` : le « composant » détecté fait la taille du
glyphe (14 × 14 ici) et non celle d'un bouton, et `border` est `null`.

**Après creusage, recadrer.** Le cerne devenu transparent laisse des marges : `dsimg.py
tint` retrime par défaut (`--no-trim` pour l'éviter). C'est ce qui fait passer `icon-plus`
de 16 × 16 à 14 × 14 sans rien perdre du dessin.

**Une ombre portée n'est pas un contour.** `icon-order` (barres noires sur beige uni) porte
un halo beige sombre de 2–3 px qui n'appartient pas au dessin : il passait pour une
transition légitime et donnait un pâté brun autour des barres. Le seuil du cœur le
sépare — `--floor 45` — parce que l'ombre est une demi-teinte là où le glyphe est franc.
Symptôme à reconnaître : le glyphe détouré traîne une masse de la couleur du bouton,
alors que le contour d'un vrai cerne est fin et régulier.

## Second lot (2026-09-09) — six glyphes sans bouton porteur

`icon-trophy`, `icon-book`, `icon-repeat`, `icon-calendar`, `icon-kamas`, `icon-xp` : aucune
de ces six captures n'est un bouton-icône. Deux sont posées sur le décor turquoise d'un
panneau (trophée, livre), quatre sur un socle sombre uni qui touche les bords de la capture
(répétition, calendrier, kamas, XP). Dans les deux cas `--from-button` n'a rien à détourer :
`analyze` rend un « composant » qui **fait la taille du glyphe** (22 × 22, 14 × 12…) et non
celle d'un bouton, exactement comme `icon-pin`. Réglages par défaut, `luma-light` sur les
six — le glyphe est clair sur son fond dans tous les cas, y compris le « Xp » cyan.

**Ne pas se fier au plateau de `tol_sweep` sur un glyphe clair.** Sur `icon-book` le balayage
descend de 27 × 25 (`--tol 4`) à un plateau 10 × 16 (`--tol ≥ 12`) : le plateau n'est pas la
bonne taille ici, c'est la moitié du livre. Le glyphe fait 22 × 18, et c'est la commande
`icon` — qui cherche le glyphe, pas le composant — qui le trouve, sans `--tol` particulier.
Le plateau de `tol_sweep` ne renseigne que sur un **composant** noyé dans son décor.

**`--floor 45` testé et écarté sur le livre et le kamas.** Les deux portent une ombre douce
qui ressemble au cas `icon-order`, mais la mesure ne suit pas : le cœur opaque est identique
(98 et 64 pixels dans les deux réglages) et le plancher relevé ne fait que redistribuer
l'alpha intermédiaire (livre : 89 → 113 pixels en 20–200, kamas : 43 → 32). Sur le livre
cette « ombre » est en fait le bas de la reliure et les petits éclats du dessin — la retirer
appauvrirait le glyphe. Régler par défaut, et garder `--floor 45` pour les vraies ombres
portées sur aplat (`icon-order`).

## `icon-info` (2026-09-10) — une pastille, pas un glyphe

Le « i » d'information est un **disque doré cerclé, posé à même le décor sombre**, pas un
glyphe sur bouton porteur : `analyze` rend un « composant » de 27 × 26 au rayon 11 — le
disque lui-même — et `--from-button` n'aurait rien à détourer, comme sur `icon-pin`.
Réglages par défaut, `luma-light` en polarité automatique : le disque part opaque et le
« i » sombre est creusé sous alpha 16. C'est exactement la construction d'`icon-help`, la
même pastille cerclée, extraite d'une capture plus petite (12 × 12).

**`--size 24` testé et écarté.** La capture fait déjà 27 px : normaliser à 24 la
rééchantillonne pour rien et écrase la barre du « i », dont l'épaisseur ne fait que 4 px.
Taille native conservée, comme sur le reste du lot.
