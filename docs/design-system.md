# Design système Wakfu — référentiel visuel

> Extrait par observation et analyse pixel de captures d'écran réelles du client Wakfu
> (voir §7 « Sources »). Objectif : que les futurs composants `overlay-ui` (boutons,
> formulaires, listes, tooltips…) partagent le même langage visuel que le jeu, pour ne
> jamais sortir l'utilisateur de son immersion.
>
> Les valeurs de couleur sont mesurées au pixel (script Python/Pillow, échantillonnage direct
> + détection des pixels saturés) — fiables à quelques valeurs près sur les zones de
> remplissage plates. Les couleurs prises sur un bord anti-crénelé (contour de texte, liseré
> de bordure) sont approximatives par nature : voir §8 pour le détail des incertitudes.

---

## 1. Langage visuel général

L'UI Wakfu n'est **pas** un design plat moderne : c'est un habillage « parchemin/étoffe
médiéval-fantastique » appliqué à une structure de panneaux fonctionnelle assez classique.
Trois traits structurants reviennent sur presque tous les éléments interactifs :

1. **Coins chanfreinés (cut corners), pas d'arrondi.** Boutons, bannières d'en-tête, icônes de
   fenêtre : les coins sont coupés en diagonale (silhouette octogonale), jamais de
   `border-radius` circulaire classique. Les listes/inputs simples, eux, restent rectangulaires
   à angle droit — le chanfrein est réservé aux éléments « boutons/bannières ».
2. **Texture tramée diagonale omniprésente.** Un motif de croisillons/losanges fins (aspect
   tissé, façon étoffe ou métal martelé) est plaqué en surimpression sur **tous** les fonds
   colorés d'éléments interactifs : boutons (or, kaki, gris), bannière d'en-tête (turquoise).
   Ce n'est pas un dégradé pur — c'est un dégradé + une texture répétitive.
3. **Dégradé vertical clair→sombre** sur les surfaces « boutons » et « bannières » (plus clair
   en haut, plus sombre en bas), donnant un effet de volume/bombé plutôt qu'un aplat.

Base générale : fond de panneau **anthracite bleuté quasi noir**, texte clair, accents **or/
kaki chauds** pour les états actifs/positifs, **turquoise** pour les bannières de fenêtre.

---

## 2. Palette

### 2.1 Neutres (fond, panneaux, structure)

| Rôle | Hex | Remarque |
| --- | --- | --- |
| Fond de panneau (body) | `#181820` – `#1D2024` | Couleur dominante mesurée sur tooltips, inputs, listes d'objets. Quasi identique partout → c'est LE fond de référence de toute l'UI. |
| Fond de panneau, variante plus claire | `#26292F` – `#2A2A2D` | Cadre extérieur des champs de recherche/inputs, légèrement plus clair que le fond intérieur. |
| Bordure/liseré sombre (boutons, checkbox) | `#181818` – `#101010` | Contour quasi noir des éléments « boutons ». |
| Texte principal clair | `#F0F0F0` – `#FFFFFF` | Titres de bannière, tooltips, texte de bouton secondaire au hover. |
| Texte atténué / placeholder | `#6B6B6B` – `#8A8A8A` (estimation, cf. §8) | « Min », « Veuillez choisir un prix » sur fond désaturé. |
| Icônes/texte désactivé | `#404040` – `#646465` | Bouton désactivé : tout le dégradé or est remplacé par une échelle de gris. |

### 2.2 Accent chaud — or / kaki (action, sélection)

Deux teintes de la même famille chaude, à ne pas confondre :

| Rôle | Hex (clair du dégradé) | Hex (sombre du dégradé) | Usage |
| --- | --- | --- | --- |
| **Or vif** (CTA primaire) | `#F4D89E` / `#EFE0B1` | `#E0C880` / `#A39463` | Bouton primaire (« Mettre en vente »), boutons icône actifs, chevron de dropdown. |
| **Kaki/bronze** (sélection, surbrillance de ligne) | `#A58E63` / `#9C8860` | `#675D46` / `#605840` | Ligne de liste active (« Build par défaut »), option survolée dans un `<select>`, onglet actif, fond de menu déroulant ouvert. |
| **Bordure chaude des champs de saisie** | `#595140` | — | Bordure fine (~1 px) systématique de TOUS les champs de saisie (recherche, nombre, texte, min/max) : c'est la signature « input » du design système. |

### 2.3 Bannière de fenêtre — turquoise

| Rôle | Hex | Remarque |
| --- | --- | --- |
| Bannière d'en-tête (dégradé) | `#0C6E80` → `#1D8B9C`/`#0887AE` | Fenêtres primaires (Personnage, Inventaire, Hôtel de vente, fenêtre de build). Texte de titre blanc, gras, centré. |
| En-tête de panneau secondaire (sans bannière) | fond neutre `#181820`, pas de dégradé coloré | Panneaux « enfant »/contextuels (ex. « Filtres avancés d'objets ») : juste un titre blanc sur fond neutre + croix de fermeture, pas de bannière turquoise. |

### 2.4 Rareté des objets (bordures d'emplacement d'inventaire)

Chaque rareté a une couleur de bordure dédiée, appliquée en dégradé bord clair (haut-gauche) →
bord sombre (bas-droit) sur le cadre carré de l'emplacement d'objet (~64×64 px). Ordre du jeu
(confirmé par le menu de filtre `select-multiple.png`) :

| Rareté | Bordure claire | Bordure sombre | Repère |
| --- | --- | --- | --- |
| Commun | `#C4C5C5` (arête haute, biseau lumineux) | `#67696B` (côtés/bas) | `common-items.png` — **confirmé** : cadre gris neutre par défaut, sans aucune teinte colorée (identique à un emplacement vide) |
| Rare | `#28C080` | `#0F6E3E` | `rare-items.png` |
| Mythique | `#C97E1E` | `#3B230D` | `mythical-items.png` |
| Légendaire | `#C4CC17` | `#333B0E` | `legendary-items.png` |
| Épique | `#E050A8` | `#33132A` | `epic-items.png` |
| Relique | `#A878E8` | `#2C1F42` | `relic-items.png` |
| Souvenir | `#2898C0` | `#0C2838` | `memory-items.png` |

> « Ancien objet » existe comme catégorie de filtre mais n'a **aucun rendu visuel propre** :
> ce n'est pas une rareté affichée sur un emplacement d'objet, donc pas de bordure à mesurer
> (confirmé — il n'y a rien à capturer, pas un oubli de capture).

---

## 3. Typographie

Deux familles de police coexistent visuellement dans l'UI. **Aucun fichier de police n'est
disponible côté client** (confirmé par l'utilisateur) — ce qui suit est une description
visuelle destinée à guider le choix d'une police de substitution, pas une identification de la
police source (voir §8) :

| Rôle | Style observé | Poids | Où |
| --- | --- | --- | --- |
| **Display / boutons / titres** | Serif à empattements marqués, caractère « médiéval-fantastique » (proche d'un slab-serif habillé, type *Cinzel*/*IM Fell*/serif de jeu) | Gras | Titres de bannière (« Personnage »), texte de bouton (« Mettre en vente », « Annuler »), en-têtes de section repliables (« Equipements »), en-têtes de bloc (« Combat », « Intelligence », « Chance »). |
| **Corps / données / UI dense** | Sans-serif neutre, très lisible en petite taille | Normal à semi-gras | Tooltips, contenu de tableau (prix HDV), texte de champ de saisie, libellés de statistiques, texte d'onglet inactif. |

Tailles approximatives (déduites de la hauteur des composants, pas d'un rendu de police brut) :
- Titre de bannière de fenêtre : ~18–20 px, gras, interlettrage légèrement augmenté.
- Texte de bouton : ~15–16 px, gras.
- En-tête de section repliable : ~14–15 px, gras.
- Corps/tooltip/tableau : ~13 px, normal.
- Libellé de champ / placeholder : ~13 px, normal, couleur atténuée.

Couleur du texte selon fond : **blanc/quasi-blanc** sur fond sombre ou bannière turquoise ;
**brun très sombre** (`#3A3523` env.) sur fond or vif (le bouton primaire n'a **pas** de texte
blanc — c'est le seul cas où le texte est foncé, car le fond est clair).

---

## 4. Espacements, dimensions, formes

Mesures directes sur les visuels (px = pixels de la capture, à l'échelle d'origine — à
ajuster au facteur d'échelle de rendu réel de l'overlay) :

| Élément | Dimension mesurée |
| --- | --- |
| Bouton texte standard (padding inclus) | ~178–209 px large × ~63–66 px haut pour un libellé court (1-2 mots) — padding horizontal généreux (~30-40 px de chaque côté du texte), padding vertical ~20 px |
| Bouton icône seul (carré) | ~50×54 px |
| Checkbox | 28×28 px (contour compris) |
| Champ de saisie standard (recherche/texte) | hauteur ~41–45 px |
| Champ de saisie compact (min/max de filtre) | hauteur ~33–36 px |
| Stepper numérique (− / champ / +) | hauteur ~41–48 px, boutons − / + carrés ~30×30 px de part et d'autre |
| Ligne d'onglet (tabs) | hauteur ~57–58 px |
| En-tête de section repliable (collapse) | hauteur ~38–45 px |
| Scrollbar (poignée) | largeur ~4 px visible dans une piste de 14 px |
| Emplacement d'objet (inventaire) | ~63–64 px carré, ~2 px d'écart entre emplacements, bordure colorée ~2 px |
| Bordure standard (inputs) | ~1 px, couleur `#595140` |
| Contour boutons | ~1–2 px, quasi noir |
| Coin chanfreiné (boutons/bannières) | coupe d'angle ~8–10 px |

**Radius** : jamais de coin arrondi classique. Les champs de saisie/listes ont des coins droits
(ou un chanfrein à peine perceptible ≤2 px) ; les boutons et bannières ont un vrai chanfrein
octogonal (~8–10 px).

---

## 5. Composants — états et détails

### 5.1 Bouton primaire (« Mettre en vente »)
- Repos : dégradé or `#E0C880`→`#3A3523`(bas, plus terne), texte brun foncé, contour quasi
  noir, texture tramée diagonale visible en filigrane.
- Hover : dégradé sensiblement **éclairci** (`#EFE0B1`→`#69624E`), texte plus contrasté.
- Disabled (« Veuillez choisir un prix ») : **désaturation complète** en niveaux de gris
  (`#282A30`→`#646465`), texte gris clair — aucune trace de la teinte or.

### 5.2 Bouton secondaire (« Annuler »)
- Repos : dégradé gris-brun sourd (`#605848`→`#E7E5E1` au centre — le texte blanc domine
  visuellement), plus terne que le primaire, même contour sombre et texture.
- Hover : identique en structure, tons légèrement plus clairs/saturés (`#84785E`→`#FDFCFC`).
- Partage la structure du primaire (mêmes dimensions, même chanfrein) — seule la teinte change :
  **c'est la même géométrie de composant, deux thèmes de couleur** (or = action positive/payante,
  gris-brun = action neutre/annulation).

### 5.3 Bouton icône (carré, ex. marteau d'enchantement)
- Repos : fond gris-brun sombre uni, icône claire centrée, pas de dégradé marqué.
- Hover : léger éclaircissement du fond.
- Disabled : fond assombri, icône estompée.
- Variante « toolbar » (barre d'actions du panneau Build, cf. `crop_toolbar.png`) : bouton
  rectangulaire à coins **légèrement arrondis** (pas chanfreinés), fond gris ardoise uni
  `#3A3D42` env., icône or si l'action est significative (roue crantée), gris neutre sinon
  (corbeille, undo) — famille visuelle distincte des boutons chanfreinés « transaction ».

### 5.4 Champs de saisie (texte / nombre / recherche)
- Fond : `#181820`–`#1C1E23`, bordure fine `#595140`, coins droits.
- Icône de recherche à gauche (loupe), croix de suppression à droite quand rempli.
- Placeholder en gris neutre atténué, valeur saisie en blanc.
- Stepper numérique : boutons `−`/`+` carrés aux extrémités, valeur centrée en **or**
  (`#F4D89E`) — seul cas où le texte d'un champ de saisie est coloré plutôt que blanc.

### 5.5 Select / dropdown
- Champ fermé : même structure qu'un input (fond sombre, bordure `#595140`), chevron `⌄` à
  droite.
- Menu ouvert : fond kaki uni `#675D46`, option survolée/sélectionnée en kaki plus clair
  `#A58E63`, texte blanc. Pas de séparateurs visibles entre options — juste le changement de
  fond au survol.
- Variante multi-sélection : checkbox devant chaque libellé, même palette kaki.

### 5.6 Checkbox
- Décoché : carré sombre quasi noir, fin liseré doré (`#595140`) autour.
- Coché : même carré, coche blanche pleine + liseré doré plus marqué/lumineux
  (`#D2AB5C`) — pas de changement de couleur de fond, juste l'ajout du symbole et un liseré
  plus affirmé.

### 5.7 Onglets (tabs)
- Onglet actif : dégradé kaki/or clair (`#F4D89E` au centre), texte blanc gras.
- Onglet inactif : fond sombre uni, texte or/ambre discret (`#C9A227` env., à affiner).
- Variante icône seule (barre d'onglets à pictogrammes) : même logique, l'onglet actif porte un
  fond kaki clair, les autres restent sombres.

### 5.8 En-tête repliable (collapse)
- Fermé : fond sombre uni, libellé en police display, chevron `⌄` à droite.
- Ouvert : même structure, chevron `⌃`, un léger liseré kaki (`#595140`) apparaît en haut du
  bloc (transition visuelle vers le contenu déplié).

### 5.9 Scrollbar
- Piste : quasi invisible, ton du fond de panneau (`#1F2126`–`#242327`).
- Poignée active (survolée/déplacée) : or/kaki clair `#C0AC83`.
- Poignée inactive (repos) : gris neutre `#5C5E61`.
- Largeur totale ~14 px, poignée elle-même fine (~4 px) centrée dans la piste.

### 5.10 Fenêtre / panneau (chrome)
- Bannière turquoise en dégradé + texture tramée, coins **supérieurs chanfreinés**, titre blanc
  centré en police display, icône de fermeture `×` en haut à droite (badge blanc arrondi), une
  icône ronde décorative (rivet/réglage) parfois présente sur le bord gauche de la bannière.
  Icône d'aide `?` parfois présente juste avant la croix de fermeture.
- Corps : fond anthracite quasi noir uni, pas de texture (la texture est réservée aux surfaces
  « actives/boutons » et à la bannière).
- Panneaux secondaires/contextuels (filtres, popovers) : pas de bannière turquoise — juste un
  bandeau de titre neutre + croix de fermeture, même fond de corps.

### 5.11 Tooltip
- Fond quasi noir semi-opaque `#181820`, coins légèrement arrondis, pas de bordure visible
  marquée, ombre portée diffuse. Texte blanc/quasi-blanc, sans-serif, corps de texte
  (pas la police display).

### 5.12 Badges de rareté / tags
- Étiquette de rareté sur une fiche objet (ex. « Légendaire ») : petit rectangle à coins
  légèrement arrondis, fond dans la teinte de la rareté (cf. §2.4) mais **désaturé/assombri**
  par rapport à la bordure d'emplacement, texte sombre ou clair selon contraste.
- Étiquette « Enchantement » : fond neutre gris foncé, texte gris clair — pas de couleur
  spécifique.

### 5.13 Indicateur d'état / pill (« Actif »)
- Petite pastille rectangulaire à coins arrondis, fond crème/or pâle (`#E6D290`), texte
  quasi noir, coche `✓` devant le libellé — pattern réutilisable pour tout indicateur
  « état actif/validé » futur dans l'overlay.

---

## 6. Recommandations d'implémentation pour `overlay-ui` (egui)

1. **Palette d'abord, texture ensuite.** La palette (couleurs plates, dégradés à 2 arrêts) est
   directement exploitable dans un `egui::Visuals`/thème custom (`theme.rs`, prévu par
   `docs/plan-architecture.md` §4 mais pas encore créé). Commencer par une version **sans**
   texture tramée : dégradés verticaux (`egui::Shape` avec un mesh 2 couleurs, ou simplement
   deux `Color32` interpolés) donnent déjà 80 % de la ressemblance.
2. **Chanfrein des coins** : egui ne l'a pas nativement (seulement `rounding` circulaire). Deux
   options : (a) dessiner les boutons/bannières en `Shape::convex_polygon` avec les coins coupés
   calculés à la main (~8-10 px), (b) accepter un `rounding` léger comme approximation
   dégradée si le chanfrein exact n'est pas prioritaire pour la v1.
3. **Texture tramée** : si la fidélité pixel-perfect est visée plus tard, découper un petit
   patch de texture depuis `docs/design-reference/button-primary.png` (zone de fond hors texte)
   et l'appliquer en `egui::TextureId` tuilée sur les formes — sinon, s'en passer pour l'instant
   (le degré d'immersion apporté par la seule palette + chanfrein est déjà élevé).
4. **Police** : pas de police source récupérable (confirmé, §8) — la piste « extraire la police
   du client Wakfu » est abandonnée. Choisir une police libre de substitution visuellement
   proche : un serif à empattements marqués type *IM Fell English*, *Cinzel* ou *Pirata One*
   pour le display (titres/boutons), une sans-serif système propre pour le corps. Les embarquer
   dans `assets/fonts/` (dossier déjà prévu dans l'arborescence cible) plutôt que d'attendre un
   futur fichier officiel.
5. **Un seul token de fond** (`#181820` / `#1D2024`) peut servir de `panel_fill` global — c'est
   la couleur la plus stable et la plus répétée dans tout le corpus analysé.
6. **Ne pas confondre** l'or vif (CTA) et le kaki de sélection : dans l'UI actuelle d'overlay-ui
   (ex. `combat.rs`), vérifier qu'aucun endroit n'utilise la même teinte pour « bouton
   principal » et « ligne sélectionnée » — le jeu les distingue toujours.

---

## 7. Sources

Captures d'écran fournies par l'utilisateur, analysées le 2026-09-05 :

- `docs/design-reference/*.png` (31 fichiers, copiés depuis `C:\Users\Oumbra\Pictures\design-system\`)
  — boutons, inputs, checkbox, select, tabs, collapse, scrollbar, emplacements d'objets par
  rareté. **Conservés dans le dépôt** pour que les futures sessions (y compris cloud, sans accès
  au dossier `Pictures` local) puissent les reconsulter. `common-items.png` a été recadré par
  l'utilisateur le 2026-09-05 (254×133, au lieu de 316×133 initialement) pour isoler la bordure
  « Commun » (cf. §2.4) — c'est cette version recadrée qui est conservée dans le dépôt.
- `C:\Users\Oumbra\Pictures\design-system-1.png`, `design-system-2.png`, `design-system-3.png`
  — captures pleine fenêtre (panneau Personnage/Build/Inventaire, Hôtel de vente + filtres,
  emplacements d'objets épiques). **Non copiées** dans le dépôt (poids : ~2 Mo chacune, valeur
  ajoutée principalement qualitative/contextuelle, déjà retranscrite ci-dessus) — rester
  disponibles en local si une nouvelle mesure est nécessaire.
- `C:\Users\Oumbra\Pictures\Capture d'écran 2026-09-04 16*.png` (3 fichiers) — exemples de
  tooltips. Non copiées, mêmes raisons.

---

## 8. Incertitudes / à vérifier

- **Pas de police dédiée disponible.** Confirmé par l'utilisateur (2026-09-05) : malgré
  l'intention initiale d'en fournir, aucun fichier de police n'existe côté client. La
  description de §3 reste donc **qualitative uniquement** (styles serif/sans-serif observés) —
  pour l'implémentation, voir §6 point 4 (police libre de substitution ou police système en
  attendant).
- **Couleurs de bordure de rareté (rare/mythique/légendaire/épique/relique/souvenir)** :
  mesurées par détection de pixels saturés sur toute l'image (bordure **et** contenu de l'icône
  peuvent tous deux contribuer) — fiables comme « famille de teinte » mais la valeur hex exacte
  du liseré seul (isolé comme cela a été fait pour Commun, cf. §2.4) mériterait une capture
  avec emplacement vide de chaque rareté si une fidélité pixel-perfect est requise plus tard.
- **Tailles de police en px** : déduites de la hauteur des composants, pas mesurées sur un
  rendu de glyphe isolé — ordres de grandeur fiables, valeurs exactes à ajuster à l'œil une
  fois un premier composant réel affiché dans l'overlay.
- **Couleur exacte du texte d'onglet inactif** : estimation visuelle (`#C9A227` env.), non
  confirmée par échantillonnage pixel dédié.
