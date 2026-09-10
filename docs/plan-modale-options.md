# Plan de finalisation — modale Options

Feuille de route pour amener la modale Options (`crates/overlay-ui/src/panels/options_modal.rs`,
§9.1 du [plan d'architecture](plan-architecture.md)) au niveau du design system du jeu.

**Ce plan est majoritairement un plan de composants.** Sept des dix étapes créent un composant
réutilisable dans `overlay_ui::design`, pas un morceau de modale. La modale est le premier client de
ces composants, elle n'en est pas la destination : chacun sera repris par les futurs onglets
(Alertes, Personnages), par le panneau Combat et par le Suivi.

## Où en est ce plan (2026-09-10)

**Les neuf étapes non optionnelles sont faites.** Seule l'étape 10 (décorations de fenêtre) reste,
et elle est explicitement « à ne faire que sur demande ».

| # | Étape | État | Ce qui l'a close |
| --- | --- | --- | --- |
| 1 | Clavier (Échap / Entrée) | ✅ | `fix: Échap annule la modale, Entrée valide` |
| 2 | `design::info_text` | ✅ | `feat: composant texte d'information` |
| 3 | `design::tabs` | ✅ | `feat: composant barre d'onglets` |
| 4 | Rythme vertical | ✅ | `fix: rythme vertical du panneau sur ses cotes` |
| 5 | `design::checkbox` | ✅ | `feat: composant case à cocher` |
| 6 | `design::select` | ✅ | `feat: composant liste déroulante` |
| 7 | `design::scroll_area` | ✅ | `feat: composant barre de défilement` |
| 8 | `design::icon_button` | ✅ | `feat: composant bouton icône` |
| 9 | Finitions de chrome | ✅ | `fix: finitions de chrome de la modale` |
| 10 | Décorations | — | optionnel, sur demande |

**Ce qui reste ouvert, et qui n'était pas du périmètre :**

- **La migration des quatre boutons icône** de `panels::combat` et `panels::watchlist` vers
  `design::icon_button`. Leur socle est déjà celui du design system — `assets/ui/button-background.png`
  est *octet pour octet* `assets/design-system/button-icon-first-plan.png` — mais leurs glyphes sont
  normalisés autrement (`ui_icons::normalize_icon_content`). Trancher demande une capture de
  référence qui n'existe pas, et changerait deux panneaux hors de cette feuille de route.
- **Le contenu de l'onglet « Paramètres »**, qui n'apparaît dans aucune ligne du tableau ci-dessous
  et c'est volontaire : il s'alimentera maintenant que les composants existent.
- **Le bouton de réinitialisation** (étape 8), qui attend qu'un réglage réinitialisable existe.
- **L'ombre portée de 2 px de la poignée de défilement** (étape 7), qu'egui n'expose pas.
- Les onglets **Alertes** et **Personnages**, présents et désactivés, prêts à être câblés.

---

## Comment lire ce document

- Les étapes s'enchaînent sans coupure, mais **chacune peut être rouverte** : un écart au design
  system peut apparaître plus tard, une capture de référence peut arriver après coup, une demande
  peut changer. Une étape close n'interdit pas d'y revenir, et y revenir n'oblige pas à rejouer les
  suivantes.
- Chaque étape se termine par : capture de non-régression régénérée, **artefact publié** (l'utilisateur
  travaille en terminal, une image inline ne s'affiche pas chez lui — voir `CLAUDE.md`), et un
  **commit isolé** sur `dev` (le code et la documentation ne se mélangent jamais dans un commit).
- Toute création de composant suit le contrat
  [`.claude/skills/ui-component/references/contrat-composant.md`](../.claude/skills/ui-component/references/contrat-composant.md)
  et s'ajoute à la planche [`design_gallery.png`](../crates/overlay-testkit/tests/snapshots/design_gallery.png)
  ainsi qu'au catalogue [`design-system-composants.md`](design-system-composants.md).
- **Un écart détecté se corrige tout de suite et se signale** — il ne va pas dans une liste
  d'attente. Cette règle prime sur l'ordre des étapes ci-dessous.
- Les estimations sont en **séances**, une séance valant l'ordre de grandeur du chantier
  `design::input` (relevé + code + galerie + doc + capture + artefact).

## Décisions déjà prises

| Question | Décision (2026-09-10) |
| --- | --- |
| Que met-on dans l'onglet « Paramètres » ? | **Rien de plus pour l'instant.** Le contenu s'alimentera au fur et à mesure que les composants existeront — ce sont eux qui débloquent les réglages, pas l'inverse. |
| Le message d'erreur rouge | **Même composant que le texte d'information, en rouge.** L'icône d'information est en cours de génération côté utilisateur ; elle sera teintée en rouge pour ce ton. Registre visé : **alerte**, pas erreur. |
| Les onglets « Alertes » / « Personnages » | **Conservés.** Vides et non cliquables pour l'instant, ils le deviendront peu après ce chantier. |
| Rayon du coin de fenêtre : 12 ou 2 ? | **12, tranché par la mesure.** Voir l'étape 9 — le relevé portait une valeur fausse, corrigée le 2026-09-10. |

---

## Acquis — ne pas refaire

| Acquis | Où |
| --- | --- |
| Chrome sur les vraies textures du jeu : bannière 56 px, fond de modale `#1C2023`, panneau `#15181C` bordé `#131518` au rayon 2 | `options_modal.rs` |
| Les trois axes de contenu : titre à +12, contrôles à +19, rembourrage haut de 13 pour poser l'encre à +19 | `options_modal.rs` |
| `design::button` — trois variantes, deux graisses de libellé, hauteur native 36, 9-slice à embouts | `design/components/button.rs` |
| `design::input` — 25 px, bord 2 px `#595140`, rayon 4, valeur en or `#f4d89e` | `design/components/input.rs` |
| Pied de page aux cotes du jeu (boutons 36 px, gouttière 12) | `options_modal.rs` |
| Règle « un panneau ne fait aucun effet de bord hors écran » — il remonte l'intention à l'hôte | §17.3 bis du plan |
| Trois relevés de référence | `docs/design-system/releve-{modale-options,section-options,options-interface}.json` |
| Capture de non-régression | `crates/overlay-testkit/tests/snapshots/options_modale_avec_erreur.png` |

---

# Étape 1 — Le clavier : Échap annule, Entrée valide

**Ce n'est pas un composant. C'est le seul défaut de comportement du lot, d'où sa place en tête.**

**Estimation : ≈ 0,3 séance (2 h).**

### État actuel

`crates/overlay-ui/src/main.rs:1354` et `crates/overlay-ui/src/bin/overlay-ui-x11.rs:710` traitent
`Échap` par `event_loop.exit()` — **l'overlay entier se ferme**. Le commentaire qui accompagne ce
filet dit qu'il ne se déclenche jamais, « ces fenêtres portent `WS_EX_NOACTIVATE`, donc ne reçoivent
jamais le focus clavier ». C'était vrai quand il a été écrit.

Ça ne l'est plus : la modale Options est **la seule fenêtre overlay focalisable**, et délibérément
(§9.1 du plan : « `WS_EX_NOACTIVATE` omis côté Windows : il faut pouvoir taper dans le champ de
chemin »). Taper Échap dans la modale tue donc l'overlay au lieu d'annuler la saisie.

### À faire

- `Échap` → `OptionsModalAction::Cancel` quand la fenêtre focalisée est la modale ; le filet global
  ne s'applique qu'aux autres fenêtres.
- `Entrée` → `OptionsModalAction::Validate`, cohérent avec le bouton primaire du pied de page.
- Focus initial dans le champ de chemin à l'ouverture de la modale — aujourd'hui il faut cliquer
  dedans avant de pouvoir taper.
- Les deux hôtes (Windows et X11) portent le même code dupliqué : les corriger **tous les deux**.

### Critère de fin

Un test du harnais qui envoie `Échap` puis `Entrée` à la modale et vérifie l'action renvoyée. La
capture ne change pas — c'est la seule étape sans rendu visuel.

---

# Étape 2 — COMPOSANT `design::info_text`

**Composant à créer.** Le premier client est le message d'erreur de la modale ; le suivant sera
l'onglet Alertes.

**Estimation : ≈ 1 séance.**

### État actuel

Le message d'erreur est un `ui.label(RichText::new(err).color(ERROR_TEXT).size(13.0))` — **le seul
texte de la modale qui échappe encore au design system** : police proportionnelle par défaut d'egui,
corps arbitraire, pas de pastille, pas d'interligne relevé.

### Cotes du design system

Source : [`releve-options-interface.json`](design-system/releve-options-interface.json), nœud `info`
(bloc d'information en bas de l'onglet Interface).

| Grandeur | Valeur | Détail |
| --- | --- | --- |
| Pastille | **12 × 12 px**, `#a69064` | boîte `[38, 438, 50, 450]` |
| Position de la pastille | centrée sur la **première ligne**, pas sur le bloc | la première ligne occupe `[57, 436, 628, 454]`, la pastille est centrée dessus |
| Écart pastille → texte | **7 px** | pastille finit à x=50, texte commence à x=57 |
| Lignes suivantes | alignées sur le **texte**, pas sur la pastille | seconde ligne à x=57 elle aussi |
| Interligne | **23 px** | y=436 puis y=459 |
| Corps | ~17 px (encre 13) | même corps qu'un libellé de bouton |
| Graisse | **regular** (`ds-label`) | pas la Medium du pied de page |
| Couleur | **blanc pur `#ffffff`** | voir l'avertissement ci-dessous |

> **Le texte d'information est blanc pur, pas gris.** Note du relevé, à ne pas contourner :
> « l'impression de gris vient du fond et de l'absence de graisse, pas de la couleur. Un portage qui
> le grise s'écarte de la référence. »

### Ce qu'il faut construire

```rust
design::info_text("Le fichier sélectionné doit s'appeler wakfu.log.")
    .tone(InfoTone::Alert)   // Info par défaut
```

- Deux tons : **`Info`** (pastille dorée `#a69064`, texte blanc) et **`Alert`** (même composition,
  teintée en rouge). Le jeu n'ayant **aucune** variante d'alerte relevée, `Alert` est une
  **extension assumée** — à documenter comme telle dans la fiche du composant, au même titre que
  les états inventés de `design::input`.
- La pastille devient l'**icône d'information** en cours de génération côté utilisateur, dès qu'elle
  est disponible dans `assets/design-system/icons/` (préparation via le skill `design-asset`). Tant
  qu'elle n'est pas là, la pastille pleine du relevé fait l'affaire — le composant ne change pas
  d'API en passant de l'une à l'autre.
- Retour à la ligne automatique sur la largeur disponible, avec l'interligne de 23 px et le retrait
  des lignes suivantes sur le texte.

### Critère de fin

Entrée dans la galerie (les deux tons, une ligne et deux lignes), fiche au catalogue,
`options_modale_avec_erreur.png` régénérée, artefact comparant le rendu au bloc d'information du
jeu.

---

# Étape 3 — COMPOSANT `design::tabs` (menu / onglets)

**Composant à créer, et le plus réutilisé du lot.** Il servira à chaque fenêtre à onglets de
l'overlay, pas seulement ici.

**Estimation : ≈ 1,5 séance.**

### État actuel

La modale peint sa barre d'onglets à la main :

- texture `menu-tabs.png` (782 × 44) **étirée à 31 px de haut** alors que sa hauteur native est 44 ;
- libellés en `egui::FontId::proportional(12.0)` — la police par défaut d'egui, pas `ds-label` ;
- répartition en trois tiers égaux, sans rapport avec les segments réels de la texture ;
- **aucun clic** : les trois entrées sont décoratives, « Paramètres » est actif en dur.

### Cotes du design system

Source : [`releve-modale-options.json`](design-system/releve-modale-options.json), nœud `tabbar` et
jetons.

| Grandeur | Valeur |
| --- | --- |
| Hauteur d'un onglet | **44 px** (`[72, 116]` dans une bande `[56, 124]`) |
| Fond inactif | `#363734` |
| Fond actif | `#625a47` |
| Séparateur | `#837d70` |
| Libellé | ~17 px, encre 13 px, centré |
| Libellé inactif **et survolé** | doré `#f4d89e` |
| Libellé actif | blanc `#ffffff` |
| Largeur | **variable, dictée par le libellé** — les six onglets du jeu font 77, 83, 103, 83, 133 et 106 px |

> **Le piège de cette barre**, énoncé tel quel dans le relevé : « l'état survolé d'un onglet reprend
> exactement le fond de l'état actif ; la seule différence relevée est la couleur du libellé (blanc
> pour l'actif, doré pour survolé et inactif). Un portage qui ne distingue que par le fond rendrait
> les deux états indiscernables. »

### Ce qu'il faut construire

```rust
let mut onglet = OptionsTab::Parametres;
design::tabs(&mut onglet)
    .entry(OptionsTab::Alertes, "Alertes").enabled(false)
    .entry(OptionsTab::Personnages, "Personnages").enabled(false)
    .entry(OptionsTab::Parametres, "Paramètres")
    .show(ui);
```

- **Trois entrées conservées.** « Alertes » et « Personnages » restent vides et **non cliquables**
  pour l'instant ; ils le deviendront peu après ce chantier, donc le composant porte dès maintenant
  un état désactivé plutôt qu'un affichage en dur.
- Quatre états par entrée : inactif, survolé, actif, désactivé.
- Largeur de chaque onglet dictée par son libellé (comme `design::button`), pas un partage en parts
  égales.
- Hauteur native 44 px — voir la leçon de `design::button` : la hauteur d'un composant est celle de
  sa texture, jamais déduite de sa largeur.
- **Relevé préalable à faire** : les segments réels de `tabs-with-first-tab-active.png` et de
  `tabs-with-first-tab-active-and-hover-2nd-tab.png` (skill `design-asset`) pour établir le
  découpage 9-slice — combien de pixels de décor à gauche, à droite, et où commence la bande
  étirable.

> ⚠️ `assets/design-system/tabs-with-first-tab-active-and-hover-2nd-tab.png` est **modifié en
> attente d'index** par une session parallèle. Ne jamais le mettre en index depuis cette
> feuille de route ; toujours `git add` des chemins explicites, jamais `-A`.

### Critère de fin

Entrée dans la galerie couvrant les quatre états, fiche au catalogue, modale portée sur le
composant, capture régénérée, artefact comparant la barre au jeu à l'échelle 3.

---

# Étape 4 — Rythme vertical du contenu

**Ce n'est pas un composant : c'est l'application d'une échelle d'espacement déjà relevée.**

**Estimation : ≈ 0,5 séance.**

### État actuel

Un `ui.add_space(10.0)` fait office de rythme entre le titre « Fichier » et sa ligne de contrôle.
Valeur choisie, pas relevée.

### Cotes du design system

Source : [`releve-section-options.json`](design-system/releve-section-options.json), note « Rythme ».

| Écart | Valeur |
| --- | --- |
| Haut d'un titre de section → haut de sa première ligne | **30 px** |
| Entre deux lignes d'option consécutives | **31 px** |
| Fin d'un bloc → titre de section suivant | **17 px** |
| Titre → contrôle **pleine largeur** qu'il coiffe | **7 à 9 px** |

Le cas de la modale est le dernier : « Fichier » coiffe une ligne pleine largeur, donc **7 à 9 px**,
pas 10 — et surtout pas 30, qui est le cas d'un titre coiffant des lignes indentées.

> Rappel du même relevé : le pas de grille **n'est pas régulier**. « Les écarts se groupent autour
> de 2, 4, 6, puis 11-12, 15-16, 19, 26, 31 et 36. Postuler un pas de 8 px décalerait tout le
> contenu. » Ne pas arrondir ces valeurs à une échelle inventée.

### À faire

Remplacer les espacements en dur par des constantes nommées portant leur source, et vérifier sur la
capture que le titre et sa ligne tombent sur le bon écart.

---

# Étape 5 — COMPOSANT `design::checkbox`

**Composant à créer.** C'est lui qui débloque le premier réglage booléen — donc le premier contenu
réel de l'onglet Paramètres.

**Estimation : ≈ 1 séance.**

### Assets disponibles

`assets/design-system/checkbox-true.png`, `checkbox-false.png` (§5.6 du design-system).

### Cotes du design system

Source : [`releve-section-options.json`](design-system/releve-section-options.json), nœuds `cb1` à
`cb3` et leurs libellés.

| Grandeur | Valeur |
| --- | --- |
| Case | **20 × 20 px**, à l'axe des contrôles (x=36 dans le jeu, +19 chez nous) |
| Rayon | **0** — la case est le seul élément carré de l'interface |
| Libellé | à **x=62**, soit 6 px après la case ; encre 10 px |
| Couleur du libellé | **blanc décoché, doré `#f4d89e` coché** |
| Rythme | 31 px entre deux lignes de case consécutives |

> La couleur du libellé porte l'état **en plus** de la case. C'est le même principe que la barre
> d'onglets : le jeu double toujours son signal visuel. Un portage qui ne change que la case perd la
> moitié de l'information.

### Critère de fin

Galerie (coché / décoché / survolé / désactivé), fiche au catalogue, artefact comparatif. La modale
n'a encore aucun réglage booléen à porter — le composant est livré prêt, son premier usage viendra
avec le contenu.

---

# Étape 6 — COMPOSANT `design::select`

**Composant à créer.** Deuxième débloqueur de contenu : tout réglage à choix multiple en dépend.

**Estimation : ≈ 1,5 séance.**

### Assets disponibles

`select-simple.png`, `select-simple-opened.png`, `select-multiple.png` (§5.5).

### Cotes du design system

Sources : [`releve-options-interface.json`](design-system/releve-options-interface.json) (nœud
`select-theme`) et [`releve-section-options.json`](design-system/releve-section-options.json).

| Grandeur | Valeur |
| --- | --- |
| Hauteur | **36 px, bord sombre compris** |
| Dégradé | `#7a6f57` → `#625946`, liseré `#a59f91`, ombre `#423b2e`, bord `#0e1015` |
| Rayon | 2 |

> **Attention au piège de la hauteur** : « le chiffre de 32 px qu'on lit en mesurant le remplissage
> est trompeur — il exclut les 2 px de bord haut et bas. »

> **Il n'existe pas de largeur de liste unique.** Le relevé mesure 210, 208, 560 et 650 px selon le
> contrôle : « la largeur est décidée contrôle par contrôle ». Le composant ne doit donc **pas**
> imposer de largeur par défaut autre que la place disponible, comme `design::input`.

### À construire

État replié, état déplié (liste flottante), sélection simple et multiple, survol d'une entrée.
L'état déplié est le seul composant du lot qui peint **hors de son rectangle alloué** — à traiter
avec une couche egui dédiée.

---

# Étape 7 — COMPOSANT `design::scrollbar`

**Composant à créer.** Il lève une déviation aujourd'hui documentée dans le code.

**Estimation : ≈ 1 séance.**

### Assets disponibles

`scrollbar-active.png`, `scrollbar-inactive.png` (§5.9).

### Cotes du design system

Source : [`releve-modale-options.json`](design-system/releve-modale-options.json), nœud
`scrollbar-thumb`.

| Grandeur | Valeur |
| --- | --- |
| Poignée | **6 px de large**, `#515356`, rayon 3 |
| Rail | **aucun** — « le fond du panneau tient lieu de gouttière » |
| Ombre | 2 px portée à droite |
| Marge poignée → bord droit du panneau | 14 px |
| Réserve à droite dans le panneau | **26 px, même quand la barre ne sert pas** |

### Ce que ça corrige

`options_modal.rs` reflète aujourd'hui l'axe des contrôles (19 px) à droite du panneau au lieu des
26 px du jeu, avec ce commentaire : « Nous n'en avons pas [de barre] : reprendre 26 px laisserait
une marge droite inexpliquée, plus large que la gauche. Déviation assumée, la seule de ce bloc. »
Une fois le composant écrit, la réserve de 26 px redevient justifiée et la déviation disparaît.

Devient **nécessaire** dès que le contenu de l'onglet dépasse la hauteur du panneau — donc
probablement pendant l'étape 6.

---

# Étape 8 — COMPOSANT `design::icon_button`

**Composant à créer — et surtout à *reprendre* : une implémentation existe déjà, hors contrat.**

**Estimation : ≈ 1,5 séance.**

### État actuel

`panels::icon_button::paint_icon_button` fonctionne mais **ne respecte pas le contrat de
composant** : il prend quatre `egui::TextureHandle` en paramètres, ce qui oblige chaque appelant à
connaître et câbler les textures — exactement le symptôme qui a motivé la création du design system.

### À construire

```rust
design::icon_button(icon)
    .context(IconContext::FirstPlan)   // ou Panel
```

Deux contextes de socle (`button-icon.png` / `button-icon-first-plan.png` et leurs `-hover`,
`button-icon-disabled.png`), une icône prise dans `assets/design-system/icons/`, un clic. Les
textures sont résolues par le manifeste `design::assets`, plus par l'appelant.

### Le cas « réinitialisation »

Le jeu emploie ce bouton deux fois dans la fenêtre Options, en **36 × 36** :

- dans la barre d'onglets, à droite — et **il n'est pas centré sur la ligne des onglets : son axe
  est 9 px plus bas** (note du relevé, à ne pas « corriger ») ;
- à l'intérieur du panneau, à droite d'un réglage isolé, pour ne réinitialiser que celui-là.

Aucun des deux n'est requis par la modale telle qu'elle est aujourd'hui : à faire quand un réglage
réinitialisable existera.

---

# Étape 9 — Finitions de chrome

**Estimation : ≈ 0,5 séance.**

### Le rayon du coin : tranché, 12

**Réglé le 2026-09-10, aucun arbitrage nécessaire.** Le relevé portait `radius.window: "2"`, le code
applique 12. La mesure départage : sur le coin haut-gauche de `interface-options-video.png` **comme**
sur `modal-header.png`, le premier pixel opaque part de x=12 sur la première ligne et atteint x=0
douze lignes plus bas — la signature d'un quart de cercle de rayon 12. Un rayon 2 ne creuserait que
deux pixels sur deux lignes.

Le 2 du relevé était l'épaisseur de la **bordure** de fenêtre (`#2a2e30`, mesurée sur un bord droit),
reportée par erreur sur le coin. `releve-modale-options.json` est corrigé, avec la note qui explique
la confusion. **Le coin de fenêtre est le seul rayon prononcé de toute l'interface ; tout le reste
est à 2.**

### Reste à faire

- **Gouttière du pied de page** : 12 px chez nous, 11 px relevés entre `btn-cancel` (finit à x=355)
  et `btn-confirm` (commence à x=366). Écart d'un pixel, à reprendre pour la forme.
- **Une capture qui montre enfin le chrome.** `options_modale_avec_erreur.png` ne peut pas servir à
  vérifier le coin ni la translucidité : le harnais `egui_kittest` peint un panneau gris opaque
  derrière la modale, qui remplit le quart de cercle. Il faut une capture sur fond damier ou sur une
  imitation de scène de jeu — sans quoi le seul rayon prononcé de l'interface n'est vérifié par
  aucun test.

---

# Étape 10 — Décorations de fenêtre (optionnel)

**Estimation : ≈ 1 séance. À ne faire que sur demande.**

`decoration-top.png`, `decoration-right.png`, `decoration-bottom.png`,
`flat-template_2-without-decorations.png` (§5.10 et §9 du design-system) : les débords ornementaux
du chrome des fenêtres du jeu. Purement décoratif, sans effet sur la lisibilité ni sur les cotes.

---

## Chronologie et dépendances

| # | Étape | Nature | Estimation | Bloque |
| --- | --- | --- | --- | --- |
| 1 | Clavier (Échap / Entrée) | correction | 0,3 | — |
| 2 | `design::info_text` | **composant** | 1 | — |
| 3 | `design::tabs` | **composant** | 1,5 | les onglets Alertes / Personnages |
| 4 | Rythme vertical | cotes | 0,5 | — |
| 5 | `design::checkbox` | **composant** | 1 | tout réglage booléen |
| 6 | `design::select` | **composant** | 1,5 | tout réglage à choix |
| 7 | `design::scrollbar` | **composant** | 1 | le contenu qui déborde |
| 8 | `design::icon_button` | **composant** | 1,5 | les réglages réinitialisables |
| 9 | Finitions de chrome | cotes | 0,5 | — |
| 10 | Décorations | décoratif | 1 | — |

**Total : ≈ 9,8 séances**, dont **7,5 en création de composants réutilisables** — soit les trois
quarts de l'effort investis hors de la modale elle-même.

Le contenu de l'onglet « Paramètres » n'apparaît dans aucune ligne de ce tableau, et c'est
volontaire : il s'alimentera au fur et à mesure que les composants existeront.
