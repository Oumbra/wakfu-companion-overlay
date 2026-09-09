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

Rend la taille utile (marge transparente retirée), le **rayon d'arrondi**, l'**épaisseur du liseré**,
la zone horizontalement reproductible, et les **marges 9-slice recommandées** — `max(rayon, liseré)
+ 2`. C'est cette valeur qui va dans le manifeste, pas une estimation.

Vérifier le réglage **avant** d'écrire du Rust, sur une planche :

```bash
python $C preview assets/design-system/button-primary.png \
  --insets 6,6,6,6 --sizes 110x32,200x52,338x36,520x48 --fill tile -o "$SCRATCHPAD/9slice.html"
```

La colonne de droite montre l'étirement bilinéaire naïf, pour comparaison : c'est le défaut qu'on
cherche à ne pas avoir (coins déformés, liseré épaissi d'un côté).

**Le mode de remplissage se choisit par axe, et c'est le piège du 9-slice.** Un dégradé vertical
doit s'**étirer** avec la hauteur ; une texture périodique (les hachures diagonales des boutons)
doit se **répéter** horizontalement, sinon les croisillons deviennent des traînées dès qu'on
dépasse la largeur native. Pour les boutons : `fill_x = Tile`, `fill_y = Stretch`.

### 3. Déclarer les textures dans le manifeste

`crates/overlay-ui/src/design/assets.rs` : une variante de `DsTexture` et une ligne dans `spec()`
par fichier. Les fichiers sont référencés **directement dans `assets/design-system/`**, jamais
recopiés dans le crate (une copie se désynchronise au premier retraitement d'asset).

C'est le seul endroit du crate où un chemin d'asset est écrit. Un composant ne connaît que des
`DsTexture`.

### 4. Écrire le composant

Un fichier par composant dans `crates/overlay-ui/src/design/components/`. Le **contrat** (API,
états, journalisation, ce qui est interdit) est dans
[`references/contrat-composant.md`](references/contrat-composant.md) — le lire, il est court, et
`button.rs` en est l'implémentation de référence.

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
