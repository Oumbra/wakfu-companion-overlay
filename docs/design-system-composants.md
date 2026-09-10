# Catalogue des composants — `overlay_ui::design`

Ce que l'overlay sait déjà composer, et avec quels paramètres. **À lire avant d'écrire le moindre
widget** : si un composant existe, on l'étend, on n'en écrit pas un second (skill
`.claude/skills/ui-component/`, étape 1).

- Le langage visuel et les mesures brutes sont dans [`design-system.md`](design-system.md) ; les
  couleurs dans [`design-tokens.json`](design-tokens.json).
- Le contrat que respecte tout composant :
  [`.claude/skills/ui-component/references/contrat-composant.md`](../.claude/skills/ui-component/references/contrat-composant.md).
- La planche de contrôle : `crates/overlay-testkit/tests/snapshots/design_gallery.png`, régénérée
  par `UPDATE_SNAPSHOTS=1 cargo test -p overlay-testkit --test design_gallery`.

---

## Bouton texte — `design::button` (2026-09-09)

`crates/overlay-ui/src/design/components/button.rs`

```rust
use overlay_ui::design::{self, ButtonSize, ButtonVariant};

if ui.add(design::button("Valider").variant(ButtonVariant::Primary)).clicked() { … }

ui.add(
    design::button("Annuler")
        .variant(ButtonVariant::Danger)
        .size(ButtonSize::Compact)
        .width(338.0)
        .log_name("options.annuler"),
);
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `variant` | `Primary` (or), `Secondary` (gris-brun), `Danger` (rouge) | `Secondary` |
| `size` | `Standard` (52px), `Compact` (36px), `Height(f32)` | `Standard` |
| `width` / `min_width` | largeur imposée / plancher | largeur = libellé + marges |
| `enabled` | `bool` | `true` |
| `tooltip` | texte d'infobulle | aucune |
| `log_name` | nom d'instance pour le journal | le libellé |
| `preview_state` | `Idle` / `Hovered` / `Disabled` — **galerie et captures uniquement** | état réel |

**Variante = intention, pas couleur.** `Danger` (rouge) est réservé au pattern « bouton pleine
largeur du pied de page d'une modale » (§5.2 du design-system) ; l'« Annuler » d'une simple boîte de
dialogue reste `Secondary`.

**États** : repos / survolé / désactivé. Pas d'état « pressé » — l'appui retire l'apparence
survolée, elle revient au relâchement (même règle que les boutons icône).

**Textures** (manifeste `design/assets.rs`, fichiers de `assets/design-system/`) :
`button-{primary,secondary,danger}.png` et leurs `-hover`, plus `button-disabled.png` **partagée par
les trois variantes** (§5.1 : l'état désactivé est une désaturation complète, une seule capture
existe).

**Une variante peut avoir plusieurs textures, choisies par la HAUTEUR.** `Primary` en a deux :
`button-primary.png` (200×52, bouton de fenêtre HDV) et `button-primary-compact.png` (338×36, pied
de page de modale, générifiée depuis `large-button-validate.png`). Ce n'est pas une redondance qu'un
étirement absorberait : l'embout décoratif mesure 52px sur la première et 34px sur la seconde.
Rendre un bouton de pied de page avec la texture de fenêtre lui donne un embout une fois et demie
trop large — invisible seul, criant à côté du bouton rouge voisin. Le composant retient la texture
dont la **hauteur native est la plus proche** de la hauteur demandée.

**Découpage 9-slice** : les hachures diagonales ne sont pas une texture de fond mais un **embout**
d'extrémité — mesuré (`component.py insets`) à 43–51px des bords sur les textures 200×52 / 169×52,
29–34px sur les 338×36, le centre étant un dégradé lisse. Marges figées : **52px** à gauche et à
droite sur la famille 52px, **36px** sur la famille 36px, **6px** en haut et en bas (arrondi 3–4px +
liseré 2px), étirement sur les deux axes. Un bouton de 500px porte donc exactement deux embouts,
comme un bouton de 200px.

**Police du libellé** : egui n'embarque qu'une Ubuntu Light et n'expose aucun réglage de graisse,
mais rien n'oblige à s'en contenter — `design::fonts` embarque `assets/fonts/Ubuntu-Medium.ttf` et
l'enregistre comme famille nommée `ds-label`, que `design::text::label_font` sert aux composants.
La proportionnelle par défaut d'egui reste celle du reste de l'interface : seuls les libellés du
design system changent.

La première version fabriquait une graisse **synthétique** (galley repeinte sur ses huit voisins
immédiats à opacité réduite). Retirée le 2026-09-09 : un halo est un contour, pas une graisse — il
épaissit le mot en dégradant son contraste, le libellé devient flou là où celui du jeu est net.
Une leçon de cet épisode survit néanmoins à la rustine, parce qu'elle se redécouvre douloureusement :
**egui arrondit la position d'un texte au pixel entier** (`Options::round_text_to_pixels`), donc
aucun effet visuel ne peut être réglé par un déplacement fractionnaire de texte.

**Graisse du libellé** : le jeu en utilise **deux**, et la variante décide laquelle.
`design::fonts` embarque Ubuntu Regular (`ds-label`) et Ubuntu Medium (`ds-label-strong`) ;
`ButtonVariant::label_strong` sert la seconde à `Primary` et `Danger`, la première à `Secondary`.

La preuve tient dans une seule capture, `interface-options-interface.png` : ses quatre boutons de
contenu et ses deux boutons de pied de page ont **exactement la même hauteur d'encre — 13px — et pas
la même graisse**. Fût moyen 1,79 à 1,92px pour le contenu contre 2,02px pour « Annuler », soit un
rapport de 0,82 en encre par colonne ; Regular/Medium rendent 0,79 dans les mêmes conditions,
Light/Medium 0,69.

Deux pièges dans cette mesure. **Seul le rapport interne à une capture veut dire quelque chose** :
comparer un fût mesuré sur fond kaki à un rendu blanc sur noir donne des chiffres qui ne se
correspondent pas, la normalisation et le seuillage ne coupant pas au même endroit. Et
**« Valider » n'est pas une troisième graisse** malgré son fût de 3,17px : c'est le seul texte
sombre sur fond clair de l'interface, et la capture y montre une ombre cuite.

Que la graisse suive la variante est une **corrélation observée**, pas une loi : `Primary`/`Danger`
sont les deux intentions du pied de page de modale, `Secondary` est le contenu. Si un `Primary`
maigre apparaît un jour dans un panneau de contenu, c'est `label_strong` qu'il faudra ouvrir à un
réglage explicite.

**Police d'un titre** : le jeu n'utilise **pas une seule police**. Ses libellés de bouton sont dans
une linéale, mais ses titres — « Options » sur la bannière de modale, « Barres de raccourcis » en
titre de section — sont dans une **serif grasse**. `design::fonts` embarque donc aussi
`assets/fonts/PTSerif-Bold.ttf` sous la famille nommée `ds-title`, servie par
`design::text::title_font`. Vingt-six serifs ont été comparées aux deux échantillons du jeu ;
PT Serif Bold est la meilleure libre (~2 % d'écart de chasse sur « Options »).

**Cerne d'un texte** : `design::text::paint_outlined_text` repeint le texte décalé avant de le
peindre plein, et prend sa liste de décalages en paramètre. Deux jeux nommés, et le choix entre eux
n'est pas esthétique — il dépend de ce qu'il y a **derrière** le texte :

| Jeu de décalages | Quand | Pourquoi |
| --- | --- | --- |
| `OUTLINE_FULL` | texte flottant nu par-dessus le jeu (Combat, Suivi) | le fond est **arbitraire** : une ombre d'un seul côté devient illisible dès que ce fond est clair de ce côté-là |
| `SHADOW_BOTTOM_RIGHT` | texte posé sur un fond **connu** (bannière, encadré de section) | trois décalages au lieu de huit ; un contour complet empâte le mot. Le jeu éclaire ses titres depuis le haut-gauche (masse sombre mesurée à +2,6px en x, +1,8px en y) |

Les décalages sont des pixels **entiers** — voir ci-dessus, egui arrondit la position d'un texte au
pixel.

**Hauteur d'un bouton** : celle de sa texture, jamais déduite de sa largeur. Un 9-slice existe pour
qu'on l'étire en largeur *sans* toucher à sa hauteur ; le jeu affiche son bouton de pied de page à
36px quelle que soit la fenêtre (son bandeau de titre fait 56px, chez lui comme chez nous). La
modale Options déduisait la sienne du rapport d'aspect de la texture et tombait à 27px — le libellé
suivant la hauteur, il y perdait 3px d'encre sur 13.

**Mesures** :

| Grandeur | Valeur | Origine |
| --- | --- | --- |
| Hauteurs natives | 52px (HDV), 36px (pied de page de modale) | taille des textures |
| Corps de police | `17/36 × hauteur` | balayage rendu dans egui et comparé au pixel aux libellés gravés du jeu ; 17px est le seul corps qui retrouve les 13px d'encre de « Annuler » ET de « Valider », à hauteur de bouton native |
| Police du libellé | Ubuntu Medium | onze candidats comparés à la référence : Medium 17 est le seul au-dessus de 0,50 de recouvrement de forme sur les DEUX libellés (0,52/0,52 contre 0,59/0,32 pour Regular). Le jeu n'est pas cohérent entre ses deux boutons, aucune police ne colle aux deux |
| Police d'un titre | PT Serif Bold | vingt-six serifs comparées aux deux titres du jeu ; meilleure correspondance libre. Corps 21 sur la bannière (choix utilisateur : le 22 colle exactement à l'encre du jeu, 21 × 82, le 21 rend 20 × 78 et est jugé mieux proportionné) et 18 en titre de section, déduit du rapport d'encre du jeu entre ses deux niveaux de titre (17px contre 21px) |
| Graisse du libellé | Regular (contenu) / Medium (pied de page) | même hauteur d'encre 13px dans les deux cas sur `interface-options-interface.png`, mais fût 1,79–1,92px contre 2,02px — rapport 0,82, quand Regular/Medium donne 0,79 et Light/Medium 0,69 |
| Marge horizontale | `0,55 × hauteur` | ordre de grandeur de §4 du design-system — **estimation** |
| Largeur minimale | `2,5 × hauteur` | garde-fou de proportion — **réglage**, pas une mesure |

**Remplace** : les assets taillés sur mesure `assets/design-system/large-button-cancel.png` /
`large-button-validate.png` — un PNG par taille **et** par libellé.

**Migration faite (2026-09-09).** Les trois boutons de `panels::options_modal` — « Annuler »,
« Valider », « Sélectionner le fichier » — sont des appels à `design::button`. Ont disparu avec eux
les quatre PNG de `crates/overlay-ui/assets/ui/options/` (copies octet pour octet d'assets déjà au
manifeste) et le module `panels::nine_slice`, seconde implémentation du 9-slice (marge unique,
étirement seul, pas de répétition). Le gain visible est sur « Sélectionner le fichier » : rendu à
700px depuis une texture de 169px, son ancien 9-slice ne figeait que 14px de chaque côté — les
croisillons tombaient dans la bande médiane et s'y étiraient sur près de 200px.

---

## `design::input` — champ de saisie

**API** : `design::input(&mut valeur)`, plus `.placeholder(...)`, `.width(...)`, `.size(...)`,
`.enabled(...)`, `.tooltip(...)`, `.log_name(...)`. Rend une `Response` : `changed()` dit à quelle
frame la valeur a bougé.

**La valeur vit chez l'appelant.** Le composant l'écrit, l'appelant la relit — c'est un champ de
formulaire, pas un état interne. **Vide = le texte indicatif s'affiche** : une `String` vide *est*
l'absence de valeur pour un champ texte, une `Option` n'ajouterait aucune information et obligerait
chaque appelant à trancher entre `None` et `Some("")`.

**Largeur par défaut : toute la place disponible** — l'inverse du bouton, qui se cale sur son
libellé. Un champ de saisie n'a pas de contenu au moment où on le place ; sa largeur ne peut venir
que de la mise en page.

**Mesures** (barre de recherche de l'onglet Commandes, x 30..500 × y 138..163, recoupée avec les
assets isolés via `dsimg.py analyze`) :

| Grandeur | Valeur | Origine |
| --- | --- | --- |
| Hauteur native | 25px | capture ET `large-input-text-width-placeholder.png` (composant 258 × 25) — deux sources indépendantes d'accord |
| Bord | 2px `#595140` | deux lignes et deux colonnes pleines sur la capture ; même couleur sur les quatre assets de champ |
| Rayon | 4 | `dsimg.py analyze`, IoU 1,0 sur deux assets — **le seul composant du jeu qui ne soit pas à 2** |
| Fond | `#0e1115` | capture |
| Retrait du texte | 6px du bord extérieur | premier glyphe de « Rechercher » à x=36 pour un champ à x=30 |
| Corps | 17px (encre 13) | même encre qu'un libellé de bouton, donc même corps |
| Valeur saisie | **`#f4d89e`, or** | pic identique sur `input-search.png`, `input-number.png` et `large-input-number.png` |
| Texte indicatif | `#83775b`, kaki éteint | pic sur « Rechercher » et « Min » |

**Une valeur saisie est or, pas blanche** — le constat le plus contre-intuitif du relevé, et il
tient sur trois assets indépendants. Le texte indicatif est peint à la main plutôt que confié à
`TextEdit::hint_text` : egui écrase la couleur d'un `hint_text` par `Visuals::weak_text_color()`.

**Un champ fait 25px là où un bouton en fait 36**, et ce n'est pas une incohérence à corriger : le
jeu compose réellement des lignes où le champ est plus bas que ce qui l'accompagne. C'est à la mise
en page de centrer le plus petit.

**Inventé, faute de capture** — signalé pour ne pas être pris plus tard pour une mesure : l'état
**survolé** ne change rien (inventer un éclaircissement serait inventer du design), et l'état
**désactivé** reprend les jetons du bouton désactivé.

**Remplace** : le champ repeint à la main dans `panels::options_modal`, dont les trois constantes
locales étaient toutes fausses (fond `#1C1E23`, bord 1px, rayon 2, valeur blanche).

---

## `design::info_text` — texte d'information (2026-09-10)

`crates/overlay-ui/src/design/components/info_text.rs`

```rust
use overlay_ui::design::{self, InfoTone};

ui.add(design::info_text("Le thème sera appliqué au prochain démarrage."));

ui.add(
    design::info_text("Le fichier sélectionné doit s'appeler wakfu.log.")
        .tone(InfoTone::Alert)
        .width(inner_rect.width())
        .log_name("options-erreur"),
);
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `tone` | `Info` (pastille dorée, texte blanc), `Alert` (tout en rouge) | `Info` |
| `width` | largeur imposée | toute la place disponible |
| `tooltip` | texte d'infobulle | aucune |
| `log_name` | nom d'instance pour le journal | les quatre premiers mots |

**Mesures** (nœud `info` de [`releve-options-interface.json`](design-system/releve-options-interface.json)
— le bloc en bas de l'onglet Interface, **le seul de toute la fenêtre Options du jeu**) :

| Grandeur | Valeur | Origine |
| --- | --- | --- |
| Pastille | 12 × 12px | boîte `[38, 438, 50, 450]` |
| Position de la pastille | centrée sur la **première ligne** | axe 444, contre 445 pour l'encre de la ligne 1 |
| Écart pastille → texte | 7px | la pastille finit à x=50, le texte commence à x=57 |
| Lignes suivantes | alignées sur le **texte** | seconde ligne à x=57 elle aussi |
| Interligne | 23px | haut d'encre à haut d'encre (y=436 puis y=459) |
| Corps | 17px (encre 13) | le même qu'un libellé de bouton |
| Graisse | regular | pas la Medium du pied de page |
| Texte | **`#ffffff`, blanc pur** | pic mesuré |
| Pastille | `#a69064` | jeton `info-dot` du relevé |

**Le texte d'information est blanc pur, pas gris**, et c'est le piège de ce composant. Note du
relevé : « l'impression de gris vient du fond et de l'absence de graisse, pas de la couleur. Un
portage qui le grise s'écarte de la référence. »

**La pastille est centrée sur la première ligne, pas sur le bloc** — sur un message de deux lignes,
un centrage sur le bloc la descendrait de 11px. Le retour à la ligne, lui, s'aligne sur le texte :
la pastille reste seule dans sa colonne.

**L'interligne est une mesure, pas la hauteur naturelle de la police.** Sans le `line_height` imposé
à 23px, egui empile les lignes à ~20px et le bloc se resserre par rapport au jeu.

**Inventé, faute de capture** : le ton **`Alert`**. Le jeu n'a aucune variante d'alerte relevée —
seule sa *composition* est une extension ; sa couleur est `accent_danger.top` (`#c9524a`), le haut
du bouton « Annuler », le seul rouge que le design system ait mesuré.

**Deux écarts au contrat de composant, assumés** :

- **Pas de trois états, pas de `preview_state`.** Ni survol, ni désactivé, ni clic : ce n'est pas un
  contrôle, et inventer un survol ici serait inventer du design. Il garde `Sense::hover()` pour
  pouvoir porter une infobulle.
- **Pas de 9-slice.** `DsTexture::IconInfo` (`icons/icon-info.png`, 27 × 28) est la première icône du
  manifeste et la seule texture à marges nulles : figer des coins sur un glyphe de 27px le
  déformerait dès qu'on le peint à 12, et il n'y a rien de périodique à répéter. Elle passe malgré
  tout par `DesignSystem::paint`, pour garder un seul chemin de peinture. Blanche dans le fichier,
  elle prend sa couleur par teinte — c'est ce qui permet au ton `Alert` d'exister sans second
  fichier.

**Remplace** : le `ui.label(RichText::new(err).color(ERROR_TEXT).size(13.0))` de
`panels::options_modal` — le dernier texte de la modale à échapper au design system.

---

## `design::tabs` — barre d'onglets (2026-09-10)

`crates/overlay-ui/src/design/components/tabs.rs`

```rust
use overlay_ui::design::{self, TabState};

design::tabs(&mut state.tab)
    .entry(OptionsTab::Alertes, "Alertes")
    .enabled(false)
    .entry(OptionsTab::Personnages, "Personnages")
    .enabled(false)
    .entry(OptionsTab::Parametres, "Paramètres")
    .log_name("options-onglets")
    .show(ui);
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `entry(valeur, libellé)` | une entrée, dans l'ordre d'affichage | — |
| `enabled` | **s'applique à la dernière entrée déclarée** | `true` |
| `preview_state` | `Idle` / `Hovered` / `Active` / `Disabled`, sur la dernière entrée — **galerie et captures uniquement** | état réel |
| `log_name` | nom d'instance pour le journal | `"tabs"` |

La valeur sélectionnée vit chez l'appelant, comme celle de `design::input` ; `Response::changed()`
dit à quelle frame elle a bougé. `show(ui)` est un alias d'`ui.add(...)`, plus lisible quand le
composant porte une liste d'entrées chaînées.

**Quatre états, pas trois** — un onglet porte en plus la notion d'être *celui qui est sélectionné* :

| État | Fond | Libellé |
| --- | --- | --- |
| Inactif | sombre `#363734` | doré `#f4d89e` |
| Survolé | **kaki, celui de l'actif** | doré `#f4d89e` |
| Actif | kaki `#625a47` | **blanc `#ffffff`** |
| Désactivé | sombre | `TEXT_DISABLED` — inventé |

**Le piège de ce composant**, énoncé tel quel par le relevé : « l'état survolé d'un onglet reprend
exactement le fond de l'état actif ; la seule différence relevée est la couleur du libellé. Un
portage qui ne distingue que par le fond rendrait les deux états indiscernables. » Deux textures
suffisent donc pour quatre états.

**Mesures** (`releve-modale-options.json`, nœuds `tabbar` et `tab-*`, recoupées au pixel sur
`interface-options-video.png` ligne y=110) :

| Grandeur | Valeur | Origine |
| --- | --- | --- |
| Hauteur | **44px**, native | `[72, 116]` dans une bande `[56, 124]` |
| Séquence entre deux onglets | bord 2px + séparateur 2px + bord 2px | identique sur la capture et sur l'asset |
| Séparateur | `#595140` | capture de la modale (x 100-101) — le relevé dit `#837d70`, l'asset `#6d6657` ; la capture de la fenêtre qu'on reproduit l'emporte |
| Corps du libellé | 17px (encre 13) | comme tous les libellés du jeu |

**Les 6px de gouttière du relevé sont ceux du remplissage**, pas de la boîte : chaque onglet porte
ses deux bords de 2px, le composant les pose donc à 2px l'un de l'autre et peint le séparateur dans
cet intervalle.

**Le rayon n'est pas sur l'onglet, il est sur la barre.** Le premier segment de l'asset a un coin
arrondi (rayon 4) ; les segments du milieu sont parfaitement droits. `tab-active.png` est découpée du
premier segment **coin gauche redressé** (reconstruit par le miroir du bord droit, plat),
`tab-inactive.png` du segment du milieu. Le rayon des deux extrémités de la barre **n'est pas
reproduit** : le bord d'un onglet (`#1c1e21`) et le fond de modale qui l'entoure (`#1c2023`) sont de
la même valeur, l'arrondi y est invisible.

**Inventé, faute de référence** :

- **La règle de largeur.** Les six onglets du jeu font 77, 83, 103, 83, 133 et 106px pour des encres
  de 26, 44, 73, 28, 100 et 35 : ni un padding constant, ni le nombre de caractères, ni une largeur
  minimale unique n'en rendent compte. Le composant retient le padding stable sur les deux libellés
  longs (`TAB_PADDING_X` = 16) et un plancher au plus petit onglet relevé (`TAB_MIN_WIDTH` = 77). Les
  libellés longs tombent à 1 et 6px de la référence, les courts ressortent plus étroits.
- **L'état désactivé.** Aucune capture d'onglet grisé ; fond inactif et libellé `TEXT_DISABLED`, par
  cohérence avec le bouton désactivé.

**Remplace** : la barre peinte à la main de `panels::options_modal` — texture `menu-tabs.png` de
44px étirée à 31, libellés en `FontId::proportional(12.0)`, trois tiers égaux, aucun clic.

---

## À faire — composants identifiés, pas encore écrits

Par ordre de fréquence d'usage constatée dans l'overlay et dans les interfaces du jeu relevées :

| Composant | Assets disponibles | Notes |
| --- | --- | --- |
| **Bouton icône** | `button-icon[-hover,-disabled].png`, `button-icon-first-plan[-hover].png`, `icons/*.png` | Existe déjà en `panels::icon_button::paint_icon_button`, mais **hors contrat** : prend quatre `TextureHandle` en paramètres. À reprendre en `design::icon_button(icon).context(FirstPlan|Panel)` — deux contextes de socle, une icône, un clic. |
| **Case à cocher** | `checkbox-{true,false}.png` | §5.6. |
| **Onglets icône** | `icon-tabs.png` | Les onglets TEXTE sont faits (`design::tabs`) ; la variante à pictogrammes reste à écrire. §5.7. |
| **Select / dropdown** | `select-simple.png`, `select-multiple.png` | §5.5. |
| **En-tête repliable** | `collapse-closed.png`, `collapse-width-5th-opened.png` | §5.8. |
| **Chrome de fenêtre** | `modal-header.png`, `decoration-{top,right,bottom}.png`, `flat-template_2-without-decorations.png` | Bannière turquoise + corps + décorations ; §5.10 et §9. |
| **Scrollbar** | `scrollbar-{active,inactive}.png` | §5.9. |
| **Tuile de portrait / d'ennemi** | `crates/overlay-ui/assets/templates/*.png`, planche d'avatars | Panneau Combat — aujourd'hui entièrement dans `panels::combat` et `panels::combat_frame`. |
| **Barre de dégâts** | aucun (dessiné à la main) | Total, noms, barres proportionnelles — aujourd'hui dans `panels::combat`. |
