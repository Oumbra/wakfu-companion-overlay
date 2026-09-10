# Plan — structure et composants d'`overlay-ui`

Feuille de route ouverte par le **diagnostic du 2026-09-10** de la couche `crates/overlay-ui/`.
Elle prend la suite de [`plan-modale-options.md`](plan-modale-options.md), clos à neuf étapes sur
dix : celui-là a produit les huit composants *feuilles* du design system ; celui-ci produit la
**couche conteneur** qui leur manquait, résorbe les doublons que le diagnostic a mis au jour, et
ouvre les composants qui portent les données de l'overlay.

Le **catalogue** [`design-system-composants.md`](design-system-composants.md) reste la source de
vérité sur *ce qui existe et avec quels paramètres*. Ce plan dit *dans quel ordre le construire et
comment savoir que c'est fini*. Les deux ne se recopient pas.

## Où en est ce plan (2026-09-10)

Les trois décisions préalables sont prises et consignées. **Le lot 0 est fait.**

| Lot | Objet | État |
| --- | --- | --- |
| 0 | Ménage — sans dépendance, sans décision | ✅ `5ccf0d2`, `2941efd`, `973ead6` |
| 1 | La couche conteneur (`window`, `panel`, `heading`, `field`) | à faire |
| 2 | Icônes et infobulle | à faire, **terrain occupé** |
| 3 | Tests de géométrie des composants livrés | à faire |
| 4 | Formulaires — vague 2 du catalogue | à faire |
| 5 | Données — vague 3 du catalogue | à faire |
| 6 | Finitions — vague 4 du catalogue | à faire |

---

## Comment lire ce document

Les règles de conduite sont celles de [`plan-modale-options.md`](plan-modale-options.md#comment-lire-ce-document)
— commit isolé, capture régénérée, artefact publié, contrat de composant respecté, écart corrigé
tout de suite plutôt que mis en liste d'attente. Elles ne sont pas recopiées ici.

Trois ajouts propres à ce plan :

- **Chaque lot annonce les fichiers qu'il touche.** Plusieurs sessions travaillent sur ce dépôt en
  même temps ; l'ordre des lots est en partie dicté par ça (voir « Coordination » ci-dessous).
- **Chaque lot a un critère de fin mesurable** — une commande à lancer, un nombre à comparer — et
  pas seulement « c'est fait ».
- Les estimations sont en **séances**, même unité que le plan précédent (une séance ≈ le chantier
  `design::input` : relevé, code, galerie, doc, capture, artefact).

## Décisions déjà prises

| # | Décision | Commit |
| --- | --- | --- |
| 1 | **Vocabulaire des panneaux flottants** : Combat et Suivi prennent les *formes* et la *typographie* du jeu, et **gardent l'accent cyan**, promu en jeton `tokens::OVERLAY_ACCENT`. Un seul jeu de composants, teinte prise à un jeton — jamais un paramètre de thème. | `022df01` |
| 2 | **Le contrat de composant a deux familles** : *feuille* (`impl egui::Widget`) et *conteneur* (`show` générique sur le retour du contenu). Critère mécanique, deux formes admises, §1 bis du contrat. | `d8fe801` |
| 3 | **`DsIcon` distinct de `DsTexture`** — nettoyage de typage, sans urgence (motif révisé, voir lot 2). | `eb2c35f`, `1c8d215` |

## Acquis — ne pas refaire

- **Huit composants feuilles livrés** : bouton, bouton icône, champ, case, liste, onglets, zone
  défilable, texte d'information. Tous dans la galerie.
- **Le 9-slice est réglé** — marges dimensionnées sur l'étendue du décor, pas sur le rayon des coins.
- **La mise à l'échelle des glyphes est réglée** — `icon_button::glyph_fit` préserve le ratio natif,
  partagée avec `Input::leading_icon` depuis `4762cee`.
- **`panels::options_modal::chrome` est déjà un conteneur** au sens du nouveau contrat (`0963ab3`) :
  il peint bannière, corps, onglets, pied et panneau, rend sa zone de contenu et le clic de son
  pied, et expose un `scroll_area(ui, id, |ui, w| …)` en forme closure. Il est dans un panneau faute
  d'endroit où le mettre. Le lot 1 le déplace ; il ne le réécrit pas.
- **Le décor de la modale vient de vraies textures du jeu** — bannière et corps, marges 9-slice
  mesurées (`2168dbc`).

## Coordination avec les sessions parallèles

Le dépôt reçoit des commits de plusieurs sessions dans la même journée. Deux collisions ont déjà
failli se produire pendant le diagnostic. Règle de conduite, avant de démarrer un lot :

1. `git fetch origin` puis `git log --oneline -15 origin/dev` — regarder si les fichiers du lot ont
   bougé dans les dernières heures.
2. Si oui, **prendre un autre lot** plutôt que de travailler à côté : les lots 0, 3 et 4 sont
   indépendants les uns des autres et peuvent se prendre dans n'importe quel ordre.
3. Ne jamais commencer un lot dont un fichier est en cours de refonte par une autre session.

| Lot | Fichiers principalement touchés |
| --- | --- |
| 0 | `design/assets.rs`, `panels/options_modal.rs`, `render_content.rs`, `.claude/settings.json` |
| 1 | `design/components/{window,panel,heading,field}.rs` *(nouveaux)*, `panels/options_modal.rs` |
| 2 | `design/{assets,icons}.rs`, `design/components/icon_button.rs`, `panels/tooltip.rs`, galerie |
| 3 | les huit fichiers de `design/components/` — **ajouts en fin de fichier uniquement** |
| 4 | `design/components/input.rs` *(extension)*, quatre fichiers nouveaux |
| 5 | `design/tokens.rs`, `panels/{watchlist,combat}.rs`, cinq fichiers nouveaux |

---

# Lot 0 — Ménage

Trois points sans dépendance et sans décision préalable. Le premier a un gain net qui dépasse
largement son coût.

## 0.1 — La bannière de modale au manifeste ✅

### État actuel

`assets/design-system/modal-header.png` et `crates/overlay-ui/assets/ui/options/modal-header.png`
sont le **même fichier, octet pour octet** (`md5 fcddb015…`). La modale charge la copie par un
`include_bytes!` local — exactement le cas résorbé le 2026-09-09 pour quatre autres PNG (§9.2 du
plan d'architecture). C'est aussi la dernière raison d'exister de tout un appareillage :

```
panels/options_modal.rs   struct OptionsModalAssets { banner }   ← une seule texture
                          fn load_embedded_texture(…)            ← 6ᵉ copie de décoder→charger
render_content.rs         RenderContent { …, options_assets }    ← 18ᵉ champ
main.rs, bin/overlay-ui-x11.rs, tests                            ← câblage de ce champ
```

### Ce qu'il faut construire

1. `DsTexture::ModalHeader` au manifeste, référencée dans `assets/design-system/` — jamais recopiée.
2. La modale prend sa texture par `DesignSystem::get(ui.ctx()).texture(DsTexture::ModalHeader)`.
   **Point d'attention** : la bannière est peinte par `egui::Image::corner_radius` (arrondi HAUT
   seulement), pas par `DesignSystem::paint` — un 9-slice ne sait pas arrondir un coin. Prendre la
   texture au manifeste, garder le chemin de peinture actuel. C'est légitime : le manifeste sert à
   *nommer et charger*, pas à imposer une façon de peindre.
3. Supprimer `OptionsModalAssets`, `load_embedded_texture`, le champ `options_assets` de
   `RenderContent` et son câblage dans les deux binaires et les tests.
4. Supprimer `crates/overlay-ui/assets/ui/options/modal-header.png`.

### Critère de fin

```bash
# plus aucune copie d'asset du design system dans le crate
md5sum assets/design-system/modal-header.png                       # unique
grep -rn "include_bytes" crates/overlay-ui/src/panels/             # aucun résultat
# RenderContent perd un champ
grep -c "pub .*:" <(sed -n '/pub struct RenderContent/,/^}/p' crates/overlay-ui/src/render_content.rs)   # 17
```
Et le snapshot `options_modale_sur_damier.png` **inchangé** — c'est ce qui prouve que le
déplacement n'a rien altéré.

**Résultat mesuré** (`5ccf0d2`) : 51 lignes ajoutées, 123 retirées ; `RenderContent` passe de 18 à
17 champs ; les 12 tests d'`overlay-testkit` passent sans qu'aucun snapshot ne bouge. La copie du
PNG et le dossier `assets/ui/options/` ont disparu.

**Estimation** : ½ séance. **Réalisé** : conforme.

## 0.2 — L'écart de barre de défilement : à confirmer, pas à corriger ✅

### État actuel — et correction du diagnostic

Le diagnostic listait « deux barres de défilement, deux apparences » comme un doublon à résorber.
**C'est inexact, et il faut le dire ici avant que quelqu'un ne le "corrige"** : la barre de
`panels::combat_frame_scroll` est le résultat d'une décision explicite de l'utilisateur, documentée
dans son propre en-tête de module —

- les assets `scrollbar-active.png` / `scrollbar-inactive.png` ont été **essayés puis rejetés**
  (« trop large, cachait les portraits ») ;
- la couleur unie `#998a6c`, la largeur de 5 px, la bordure noire et les coins légèrement arrondis
  sont des **demandes explicites** ;
- la visibilité permanente est un **revirement assumé** après test en jeu réel (2026-09-08).

Ce n'est donc pas une divergence accidentelle mais une **variante décidée**, dans un contexte que la
barre du design system ne couvre pas : une barre posée *sur* un décor de cadre, pas dans la
gouttière d'un panneau.

### Ce qu'il faut faire

Rien dans le code, tant que l'utilisateur ne le demande pas. En revanche, **inscrire la variante au
catalogue** sous `design::scroll_area` : « variante *cadre* — barre dessinée fine posée sur un
décor, distincte de la poignée de gouttière ; assets du jeu rejetés, voir
`panels::combat_frame_scroll` ». Un écart documenté n'est plus une dette ; un écart non documenté
finit toujours par être « corrigé » par quelqu'un qui n'en connaît pas l'histoire.

**Question ouverte pour le mainteneur** : la variante doit-elle un jour remonter dans
`design::scroll_area` (paramètre `ScrollBarStyle::Frame`), ou rester locale au panneau Combat ?
Tant qu'elle n'a qu'un seul utilisateur, rester locale est le bon choix.

**Estimation** : ¼ séance (documentation seule).

## 0.3 — Un hook `SessionStart` pour l'environnement cloud ✅

### État actuel

Une session cloud démarre sur un conteneur où rien n'est prêt : `bash patches/setup-vendor.sh` n'a
pas tourné (`cargo` échoue sur `vendor/wgpu-hal-30.0.1`), `libasound2-dev` n'est pas installé
(`alsa-sys` échoue), et `core.hooksPath` n'est pas positionné — c'est une configuration **locale**,
elle ne survit pas au conteneur (voir `CLAUDE.md`). Trois manipulations à refaire à chaque session,
et le `pre-push` du dépôt est inactif tant que la troisième est oubliée.

### Ce qu'il faut construire

Un hook `SessionStart` dans `.claude/settings.json` qui enchaîne les trois, idempotent et silencieux
quand tout est déjà en place. Le skill `session-start-hook` couvre exactement ce cas.

### Critère de fin

Sur une session neuve, `cargo clippy -p overlay-ui --lib` compile sans qu'aucune commande
préparatoire n'ait été tapée à la main, et `git config core.hooksPath` rend `.githooks`.

**Estimation** : ½ séance.

---

# Lot 1 — La couche conteneur

Le cœur du chantier. Objectif : **un panneau ne calcule plus de rectangles**, il décrit ce qu'il
contient. Tout ce lot relève de la seconde famille du contrat (§1 bis).

## État actuel

`panels/options_modal.rs` fait **823 lignes**. `chrome()` en a déjà extrait le décor — c'est ce qui
rend ce lot sûr : il s'agit d'un **déplacement**, pas d'une réécriture, et le snapshot le vérifie.

## 1.1 — `design::window`

Remonter `panels::options_modal::{Chrome, chrome, FooterClick}` dans
`design/components/window.rs`, en **forme « zone rendue »** — c'est le cas nominal prévu par le
contrat : le conteneur ne peint rien après le contenu, et rend deux sorties indépendantes (la zone,
le clic de pied). Sa méthode `scroll_area` reste en forme closure.

Ce qui devient paramètre au passage, parce que c'est ce qui change d'une fenêtre à l'autre : le
titre, les entrées d'onglets et leur état d'activation, les libellés du pied de page (ou son
absence), la taille de bannière.

**Point d'attention** : `chrome()` prend aujourd'hui `assets: &OptionsModalAssets`. Le lot 0.1
supprime ce paramètre — faire 0.1 **avant** 1.1, sinon le composant naîtrait avec une texture en
paramètre, ce que le contrat interdit (§1).

## 1.2 — `design::panel` et `design::heading`

- `panel` — le panneau de contenu (`#15181c`, bord 2 px `#131518`, rayon 2), forme closure.
- `heading` — titre de fenêtre et titre de section : serif, ombre bas-droite, et surtout **l'espace
  réservé est celui de l'encre, pas celui de la galley** (le piège documenté dans `options_modal`,
  qui vaut sept relevés). Feuille, pas conteneur.

Les deux existent déjà en code dans `options_modal` (`section_title`, le bloc de peinture du
panneau) : même travail de déplacement que 1.1.

## 1.3 — `design::field`

La ligne « libellé + contrôle » : gouttière, largeur du contrôle déduite de celle du reste, et la
règle du design system — **la hauteur d'un composant est celle de sa référence, pas celle de son
voisin** (le champ garde ses 25 px natifs sur une ligne que le bouton fixe à 36). Le cas réel est
écrit et validé dans `options_modal`, il s'agit de le généraliser.

## Critère de fin du lot 1

```bash
wc -l crates/overlay-ui/src/panels/options_modal.rs          # < 300
grep -c "egui::Rect::from" crates/overlay-ui/src/panels/options_modal.rs   # 0
```
Snapshots `options_modale_sur_damier.png` et `options_modale_avec_erreur.png` **inchangés**, et les
maquettes de la page Alertes rendues à l'identique. La galerie gagne trois entrées, chacune avec du
contenu débordant pour montrer l'écrêtage.

**Estimation** : 2 à 3 séances.

---

# Lot 2 — Icônes et infobulle

**Terrain occupé au 2026-09-10** — une session parallèle travaille sur `assets.rs`,
`icon_button.rs` et la galerie des glyphes. Vérifier avant de démarrer (voir « Coordination »).

## 2.1 — `DsIcon` et `design::icon`

Motif révisé : ce n'est plus un problème d'échelle (`glyph_fit` l'a réglé) mais un **nettoyage de
typage** — `TextureSpec` porte un découpage 9-slice dont aucun glyphe ne se sert, et
`icon_content_size()` énumère à la main les variantes qui sont des icônes. Le détail et les cinq
étapes d'exécution sont dans le catalogue, section vague 1. **Rien n'en dépend** : ce lot peut
attendre indéfiniment sans bloquer les autres.

Ne **pas** déclarer les 34 icônes d'un coup : le rythme actuel — une icône entre au manifeste quand
un composant en a besoin — est le bon. Un manifeste ne doit rien contenir de mort.

## 2.2 — `design::tooltip`

### État actuel

`panels/tooltip.rs` porte le fond, la marge et l'écart mesurés ; **trois enveloppes maison** portent
le placement (`combat::show_tooltip_above`, `watchlist::show_tooltip_left` et `_right`), et le
catalogue note déjà que `icon_button.tooltip()` « ne sait pas reproduire » le placement par colonne.
Le panneau Suivi réserve 88 px de chaque côté (`CONTROL_TOOLTIP_RESERVE`) pour qu'une infobulle ait
matériellement la place de s'afficher.

### Ce qu'il faut construire

Un composant avec **placement paramétrable** (`Above` / `Left` / `Right` / `Auto`) et ses replis,
absorbant les trois enveloppes. L'historique des replis `RectAlign` est déjà écrit dans `combat.rs`
— le lire avant, il porte trois retours utilisateur successifs.

### Critère de fin

`grep -rn "show_tooltip_" crates/overlay-ui/src/panels/` ne rend plus rien, et les quatre snapshots
d'infobulle du Suivi (`watchlist_tooltip_*.png`) sont **inchangés**.

**Estimation** : 1 à 1½ séance.

---

# Lot 3 — Tests de géométrie

## État actuel

Sur les huit composants livrés, **deux** portent des tests : `icon_button` (5) et `input` (1). La
galerie attrape une régression de *rendu* ; elle n'attrape pas une régression de `desired_size` sur
un libellé long, un choix de texture par hauteur, ou un plancher de largeur négative.

## Ce qu'il faut construire

Trois tests par composant, sans GPU, sur le modèle de `icon_button::tests` :

1. **La taille désirée** suit le contenu et le gabarit (au moins deux gabarits comparés).
2. **Le choix de texture ou de variante** est celui attendu (ex. `ButtonVariant::texture` retient la
   hauteur native la plus proche — un test qui aurait attrapé l'embout une fois et demie trop large).
3. **Le cas dégénéré** ne panique pas et ne produit pas de rectangle inversé (largeur imposée
   inférieure aux marges, libellé vide, rectangle plus petit que le contenu).

## Critère de fin

```bash
grep -rc "#\[test\]" crates/overlay-ui/src/design/components/*.rs   # ≥ 3 partout
cargo test -p overlay-ui --lib
```

**Estimation** : 1 séance. **À faire avant le lot 4** : c'est le filet qui rendra les extensions de
`input` sûres.

---

# Lot 4 — Formulaires (vague 2 du catalogue)

Dans l'ordre du catalogue : extensions d'`input` (`Number`, `Search`, état d'erreur), `stepper`,
`collapsible`, `field` — si le lot 1 ne l'a pas déjà produit —, `slider`, variante icône de `tabs`.

Deux rappels qui valent pour tout ce lot :

- **`input` s'étend, il ne se duplique pas.** Une variante de plus est un paramètre de plus, jamais
  un second composant (étape 1 du skill `ui-component`).
- **`slider` n'a aucun asset découpé.** Passer par le skill `design-asset` sur
  `interface-options-son.png` avant d'écrire la moindre ligne de Rust — un composant écrit sur des
  valeurs devinées est un composant à refaire.

`stepper` et `collapsible` ont leurs cotes **déjà relevées** dans
[`design-tokens.json`](design-tokens.json) (`stepper_height` 41–48, `stepper_button_square` 30,
`collapse_header_height` 38–45).

**Critère de fin** : l'onglet Alertes de la modale Options se compose sans qu'aucun panneau ne
peigne un widget à la main.

**Estimation** : 4 à 5 séances.

---

# Lot 5 — Données (vague 3 du catalogue)

Conditionné par la **décision 1**, qui est prise. Commencer par le jeton, pas par les composants.

## 5.0 — `tokens::OVERLAY_ACCENT` d'abord

Créer le jeton et y rebrancher les constantes locales de `panels::combat` et `panels::watchlist`,
avec la typographie et les formes qui migrent au langage du jeu (les trois familles sont détaillées
dans le catalogue, vague 3). Ce doit être le **premier** pas : les composants qui suivent prennent
leur teinte à ce jeton, et l'écrire après reviendrait à les reprendre tous.

## 5.1 à 5.5

`item_slot`, `badge`, `meter`, `portrait`, puis `table` + `pagination`. Détail, matière disponible
et code absorbé : catalogue, vague 3. `table` demande un relevé `ui-blueprint` préalable sur les
captures d'interfaces — il n'existe pas encore.

**Critère de fin** : `panels/watchlist.rs` et `panels/combat.rs` ne contiennent plus aucun
`Color32::from_rgb` ni `FontId::proportional`.

**Estimation** : 5 à 7 séances.

---

# Lot 6 — Finitions (vague 4 du catalogue)

`toolbar`, `segmented`, `toast`, `dialog`, `separator`. Aucun ne bloque quoi que ce soit ; chacun
retire du code d'un panneau. À prendre par opportunité, quand un besoin réel se présente — pas pour
vider une liste.

**Estimation** : 3 à 4 séances.

---

## Une amélioration à porter au catalogue, en continu

Chaque fiche de composant devrait porter **qui le consomme en production**. La fiche du bouton icône
le fait déjà (« les quatre boutons du carré de contrôle du Suivi ») ; les autres non. C'est cette
ligne qui répond, dans six mois, à « puis-je changer ce jeton sans rien casser ? ». À remplir au fil
des lots, pas dans une passe dédiée.

## Hors périmètre de ce plan

- **Le portage de nouvelles fonctionnalités** (onglets Personnages, historique HDV, échanges) : ce
  plan fournit les composants, il ne décide pas de ce qu'on en fait.
- **Les six chemins de chargement de texture.** Le lot 0.1 en supprime un ; les cinq autres
  (`ui_icons`, `portraits`, `remote_icons`, `combat_frame`, plus `DesignSystem`) tiennent à des
  familles d'assets réellement différentes — atlas de classes, icônes distantes téléchargées,
  gabarits. Les unifier demande son propre plan, et le gain n'est pas établi.
- **Les 18 champs de `RenderContent`.** Le lot 0.1 en retire un, les lots 1 et 5 en retireront
  d'autres par ricochet. Une refonte frontale du passage d'état est un autre chantier.
