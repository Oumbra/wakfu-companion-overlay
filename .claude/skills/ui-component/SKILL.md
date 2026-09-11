---
name: ui-component
description: Construire, étendre ou corriger un composant réutilisable du design system egui de l'overlay (`crates/overlay-ui/src/design/components/`) à partir des assets de `assets/design-system/`, d'un relevé `ui-blueprint` et d'une spec fonctionnelle — API paramétrable (libellé, variante, taille, état), textures peintes en 9-slice à n'importe quelle dimension, journalisation, entrée dans la galerie de contrôle et publication de la capture en Artifact. À utiliser dès qu'on demande un bouton, un champ, un onglet, une modale, un bandeau… plutôt que de repeindre un widget à la main dans un panneau.
---

# Composants du design system

**Une interface se compose, elle ne se redessine pas.** Avant ce skill, chaque panneau peignait ses
propres boutons (sept constantes de couleur et un `chamfer` par bouton dans
`panels::options_modal`) et chaque nouvelle taille demandait **un asset de plus, libellé compris**
(`assets/design-system/large-button-cancel.png` = `button-danger.png` à la taille du pied de page
de la modale Options, avec « Annuler » incrusté). Le travail était donc refait à chaque fois, et un
défaut se corrigeait autant de fois qu'il avait été recopié.

La cible : `ui.add(design::button("Valider").variant(ButtonVariant::Primary))`. Le composant connaît
ses textures, ses états, ses proportions, et sait se rendre à n'importe quelle taille.

## Les trois entrées, et ce qu'on en tire

| Entrée | Fournie par | Ce qu'on en tire |
| --- | --- | --- |
| **Assets** `assets/design-system/*.png` | skill `design-asset` (détourage, retrait du libellé) | textures par variante et par état, marges 9-slice |
| **Relevé** de l'interface porteuse | skill `ui-blueprint` | tailles, gouttières, paddings, position dans la page |
| **Spec fonctionnelle** | l'utilisateur | paramètres, états, comportements, ce qui est cliquable |

Si l'un manque, le produire **avant** d'écrire du Rust : un composant écrit sur des valeurs devinées
est un composant qu'il faudra refaire.

## Déroulé

### 1. Le composant existe-t-il déjà ?

Lire [`docs/design-system-composants.md`](../../../docs/design-system-composants.md) — le catalogue.
Si le composant y est, la réponse à « il me faut un bouton un peu différent » est **d'ajouter un
paramètre au composant existant**, jamais d'en écrire un second. Deux composants pour un même
élément du jeu divergent en quelques semaines : c'est exactement l'état dont ce skill sort.

### 2. Préparer la texture et mesurer son découpage

Asset pas encore générique (libellé incrusté, décor autour) → **skill `design-asset`** d'abord
(`dsimg.py genericize`). Puis :

```bash
C=.claude/skills/ui-component/scripts/component.py
python $C insets assets/design-system/button-primary.png
```

Rend la taille utile (marge transparente retirée), le **rayon d'arrondi**, l'**épaisseur du
liseré**, l'**étendue du décor** (`decor_span`) et les **marges 9-slice recommandées**. Ce sont ces
valeurs qui vont dans le manifeste, pas une estimation.

**Ce sont les hachures qui dimensionnent les marges horizontales, pas les coins.** Sur les
composants Wakfu, les croisillons diagonaux ne sont pas une texture de fond : ce sont des **embouts**
cantonnés aux premières dizaines de pixels de chaque côté (mesuré : ~50px sur les boutons 200×52,
~30px sur les 338×36), le centre restant un dégradé lisse. Une marge dimensionnée sur le seul rayon
des coins laisse le motif dans la bande médiane, où il est étiré — ou répété — sur toute la longueur
du composant. Le résultat est immédiatement faux à l'œil : un bouton large couvert de croisillons au
lieu d'en porter deux, un à chaque bout (retour utilisateur 2026-09-09).

`decor_span` **n'a aucun sens sur une texture qui porte encore un libellé** : le libellé, au centre,
fait exploser le niveau de référence et la mesure rend 0. Générifier d'abord (`design-asset`).

Vérifier le réglage **avant** d'écrire du Rust, sur une planche :

```bash
python $C preview assets/design-system/button-primary.png \
  --insets 52,6,52,6 --sizes 110x32,200x52,338x36,520x48 -o "$SCRATCHPAD/9slice.html"
```

La colonne de droite montre l'étirement bilinéaire naïf, pour comparaison : c'est le défaut qu'on
cherche à ne pas avoir (coins déformés, liseré épaissi d'un côté).

**Le mode de remplissage se choisit par axe.** `Stretch` pour un contenu continu (un dégradé, ou une
bande médiane lisse), `Tile` pour un motif réellement périodique qu'il faut répéter à son échelle.
Une fois le décor entièrement contenu dans les marges figées, `Stretch` sur les deux axes suffit —
c'est le cas des boutons.

### 3. Déclarer les textures dans le manifeste

`crates/overlay-ui/src/design/assets.rs` : une variante de `DsTexture` et une ligne dans `spec()`
par fichier. Les fichiers sont référencés **directement dans `assets/design-system/`**, jamais
recopiés dans le crate (une copie se désynchronise au premier retraitement d'asset).

C'est le seul endroit du crate où un chemin d'asset est écrit. Un composant ne connaît que des
`DsTexture`.

### 4. Écrire le composant

Un fichier par composant dans `crates/overlay-ui/src/design/components/`. Le **contrat** (API,
états, journalisation, ce qui est interdit) est dans
[`references/contrat-composant.md`](references/contrat-composant.md) — le lire, il est court.

**Feuille ou conteneur ?** C'est la première chose à trancher, et le critère est mécanique :
*l'appelant fournit-il du contenu à encadrer ?* Non → **feuille**, `impl egui::Widget`, référence
`button.rs`. Oui → **conteneur**, un `show` générique sur le retour du contenu, référence
`scroll_area.rs`. §1 et §1 bis du contrat. Le reste — états, géométrie, journalisation, galerie —
est commun aux deux.

**Les noms se lisent dans le code existant, ils ne s'inventent pas** : §1 ter du contrat donne la
table (`Ds…` pour ce qui nomme un asset, `<Composant><Rôle>` pour un paramètre, un seul préfixe de
jetons par composant) et les deux `grep` qui la vérifient en trois minutes. Deux sessions ont donné
deux noms au même type le 2026-09-11 faute de cette table.

Les valeurs numériques (corps de police, marges, proportions) vont dans `design/tokens.rs`, avec
**la provenance de chaque valeur** : mesure pixel, capture de référence, ou aveu explicite qu'il
s'agit d'une estimation. Une valeur devinée qui n'annonce pas qu'elle est devinée est le pire cas.

### 5. Ajouter le composant à la galerie

`crates/overlay-testkit/tests/design_gallery.rs` — toute variante et tout état, sur une seule
capture. `Button::preview_state` (et son équivalent pour les autres composants) force l'état peint :
en rendu offscreen aucun pointeur ne survole quoi que ce soit.

```bash
UPDATE_SNAPSHOTS=1 cargo test -p overlay-testkit --test design_gallery
```

Sans `UPDATE_SNAPSHOTS=1`, le test **compare** — c'est le filet de non-régression : toucher au
9-slice, à un jeton ou à un asset fait bouger cette image.

### 6. Comparer au jeu — l'étape qui rattrape les erreurs de proportion

Le rendu ne vaut que comparé à la capture d'origine, **pixel à pixel, à la même taille**. Découper
le composant dans `design_gallery.png` et l'empiler sur l'asset du jeu correspondant, sur le même
fond, agrandi ×2.

C'est ce qui a rattrapé, sur le bouton, un libellé 45 % trop gros : la convention typographique
« corps ≈ hauteur d'encre / 0,72 » du skill `ui-blueprint` vaut pour la police du jeu, pas pour
celle d'egui (0,805 mesuré). Aucune relecture de code n'aurait signalé ça.

### 7. Publier la planche en Artifact — obligatoire

L'utilisateur travaille en terminal : une image lue en ligne lui est **invisible** (CLAUDE.md). La
capture de galerie, la comparaison au jeu et la planche 9-slice se publient en Artifact. « Le test
passe » n'est pas une vérification visuelle.

### 8. Tenir le catalogue à jour

Ajouter/mettre à jour l'entrée dans `docs/design-system-composants.md` : paramètres, variantes,
états, textures utilisées, mesures. C'est ce que lira la prochaine session avant de réécrire le
composant pour la deuxième fois.

## Ce qui est interdit

- **Un asset par taille.** S'il faut un PNG de plus pour un bouton plus large, le 9-slice est mal
  réglé ou pas utilisé.
- **Un asset par libellé.** Le texte est rendu par egui, toujours.
- **Peindre un widget du design system dans un panneau.** `panels/*` fait de la mise en page et
  porte l'état applicatif ; il ne dessine pas de dégradé ni de bordure de bouton.
- **Une texture en paramètre de composant.** L'appelant nomme une intention (`Primary`), pas un
  fichier — c'est ce qui distingue ce composant de `panels::icon_button::paint_icon_button`, qui
  prend encore quatre `TextureHandle` et reste à migrer.
- **Une constante sans provenance.** Voir étape 4.

## Migrer un panneau existant

Remplacer un widget peint à la main est un **changement visuel** : produire la capture avant/après,
la publier, et ne pas supprimer les constantes de l'ancien code tant que la comparaison n'a pas été
validée par l'utilisateur. Les assets rendus inutiles par la migration (un PNG taillé pour une
taille et un libellé) se suppriment dans le même commit, en le disant.
