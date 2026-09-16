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
- L'ordre de construction de ce qui manque, avec ses critères de fin :
  [`plan-composants-ui.md`](plan-composants-ui.md). Ce catalogue dit *ce qui existe*, ce plan dit
  *dans quel ordre construire la suite*.

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
`.enabled(...)`, `.error(...)`, `.read_only(...)`, `.leading_icon(...)`, `.tooltip(...)`,
`.log_name(...)`. Rend une `Response` : `changed()` dit à quelle frame la valeur a bougé.

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

**L'état d'erreur** (2026-09-11) est le quatrième, et il est propre au champ : les trois états du
design system décrivent ce que l'interface *permet*, or une valeur peut être refusée alors que le
champ reste parfaitement actif. `Error` n'est donc pas une nuance de `Disabled` mais son contraire —
il faut justement revenir dans le champ.

- **Seul le bord change** (`INPUT_BORDER_ERROR`). La valeur reste or : la teindre en rouge la
  donnerait à lire comme un message plutôt que comme une saisie, et lui ferait perdre le contraste
  voulu sur le fond très sombre.
- **Le rouge est celui d'`INFO_ALERT`**, délibérément par alias et non par seconde valeur : le champ
  et son message sont un seul signal en deux endroits, deux rouges voisins se liraient comme deux
  alertes. Le jour où le jeu fournira une capture, c'est ce jeton qui prendra la mesure et se
  détachera de lui-même.
- **Le composant ne valide rien** — il ne sait pas ce qu'est un chemin correct. La validation
  appartient à l'appelant, qui la possède déjà, et le champ la reflète.
- **`Disabled` l'emporte sur `Error`** : on ne corrige pas ce qu'on ne peut pas éditer.

Consommé en production par `panels::options_modal`, dont le champ de chemin porte désormais
l'alerte en même temps que son message — celui-ci est sous le bouton « Parcourir », hors du regard
de qui vient de taper.

**Remplace** : le champ repeint à la main dans `panels::options_modal`, dont les trois constantes
locales étaient toutes fausses (fond `#1C1E23`, bord 1px, rayon 2, valeur blanche).

### Correctif du 2026-09-13 : le champ ne déplace plus le curseur de son appelant

Le `TextEdit` était posé par `ui.scope_builder` sur la zone de texte — plus courte que le champ des
marges, de la croix d'effacement et de l'icône de tête. Un scope avance le curseur du `Ui` parent
jusqu'à la fin de ce rectangle, pas du champ : dans une rangée horizontale, le widget suivant se
posait **21 px trop à gauche** sur un champ `clearable` (mesuré : bouton à 367 pour un champ allant
jusqu'à 388) ; dans une pile verticale, tout ce qui suivait un champ était **4 px trop haut**. Les
onglets Suivi et Alertes de la fenêtre Options, et la galerie, portaient ce décalage sans que
personne ne l'ait vu — il a fallu un bouton posé juste après un champ (maquettes de l'onglet Chat,
`crates/overlay-testkit/examples/chat-mockups.rs`) pour qu'il devienne un chevauchement.

Le `TextEdit` vit maintenant dans un **enfant** (`ui.new_child`), qui a son propre curseur : la
place du champ, c'est `allocate_exact_size` qui l'a prise, et elle seule. Même idiome que
`panels::alerts_tab::alert_item` (« `ui.put` dans un ENFANT, jamais sur le `ui` de la rangée »).
Test de géométrie : `crates/overlay-testkit/tests/input_row.rs` (champ nu, effaçable, à loupe et
croix). Conséquence sur les captures : tout ce qui suit un champ a descendu de 4 px, et « sec. »
après le champ de durée d'Alertes a reculé de 7 px vers la droite — les 24 snapshots concernés ont
été régénérés, aucun jeton d'espacement n'a bougé.

### La barre de recherche : `InputSize::Search` et `clearable` (2026-09-12)

Retour utilisateur sur le champ d'ajout d'alerte, avec les deux assets détourés du jeu à l'appui
(`empty-input-search.png`, `input-search.png`, 341 × 32) : « la loupe n'est pas dans le bon sens et
n'est pas colorée comme sur la maquette », « l'input est un tout petit peu trop petit en hauteur, ou
alors c'est la police qui est trop grande », et une croix d'effacement manquante. Les trois ont été
mesurés sur ces assets, pas ajustés à l'œil :

| Grandeur | Valeur | Origine |
| --- | --- | --- |
| Hauteur de la boîte | **28px** (`INPUT_SEARCH_HEIGHT`) | y 2..29 inclus sur les deux assets, identiques au pixel — ce n'est PAS le champ de 25 px de l'onglet Commandes |
| Encre du texte | 12px de capitale (« R » y 11..22) | la même encre qu'un champ standard (« A » de 12 px dans 25) : **la boîte est plus haute, le corps ne change pas** — 17 px pour les deux gabarits |
| Loupe | manche **en bas à gauche**, teinte `#a69064` (`INPUT_ICON`) | pic dominant de la loupe, champ vide ou rempli — plus chaude et plus claire que le kaki du texte indicatif qu'elle portait ; le glyphe du manifeste a son manche à droite, il est peint **en miroir** (`paint_icon_flipped`) |
| Croix | encre 11 × 12 (x 320..330, y 10..21), `#675d46` (`INPUT_CLEAR_ICON`), à 9px du bord droit | `input-search.png` seulement : **absente du champ vide** |
| Croix survolée | `INPUT_ICON` | **inventé**, aucune capture — la teinte de la loupe, pour ne pas ajouter de couleur |

`InputSize::Search` porte la hauteur ; les trois ratios d'ornement (7/28, 13/28, 8/28), mesurés dès
l'origine sur cette capture de 28 px, y redonnent exactement 7, 13 et 8 px. `Input::clearable(true)`
pose la croix : **la place est réservée dès qu'elle est possible**, valeur ou pas, sinon le texte
se décalerait au premier caractère tapé ; un clic vide la valeur, marque la réponse `changed()` et
**rend le focus au champ** — l'appui sur la croix le lui avait retiré, or on efface pour retaper.
Sans effet sur un champ désactivé ou en lecture seule. Test réel du geste dans
`options_alertes_croix_efface_la_saisie` (survol, appui, relâchement, puis une frappe qui doit
retomber dans le champ).

Consommé par `design::autocomplete` (champ d'ajout d'alerte) et par le champ de chemin de la modale
Options, qui garde son gabarit de 25 px mais gagne la croix.

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
| Position de la pastille | centrée sur la **boîte de police** de la première ligne | pastille `[438, 450]`, hauteur d'x de la ligne 1 `[441, 449]` |
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

**Et sur la boîte de police de cette ligne, pas sur son interligne.** Le piège se cache dans le
`line_height` imposé ci-dessous : l'interligne mesuré (23px) dépasse la hauteur naturelle de la
police (19,5px à ce corps), et `epaint` ne répartit pas cette différence de part et d'autre du
texte — il cale la ligne de base sur l'ascendante et laisse les ~3,5px de rabiot **sous la
descendante**. Diviser les 23px en deux vise donc un axe qui n'est pas celui du texte : la pastille
descend de 2px et vient se poser sur la ligne de base. L'axe juste est
`ui.fonts_mut(|f| f.row_height(&font)) / 2.0`, la moitié de la hauteur naturelle.

Le contrôle se fait sur la **hauteur d'x** et non sur l'encre entière, qui dépend des accents et des
jambages présents dans la phrase : dans le jeu la pastille commence 3px au-dessus du sommet de la
hauteur d'x (438 contre 441) et finit sur sa dernière ligne d'encre (449) ; le rendu fait 32..43
contre 35..43, soit la même position au pixel près.

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
| Hauteur du séparateur | **40px, pas 44** | y 2..41 : le corps de l'onglet, entre ses deux bords sombres |
| Séparateur | **dégradé vertical `#837d70` → `#595140`** | profil de la colonne x=261 de `tabs-with-first-tab-active.png` |
| Profil du dégradé | 11px clair, 19px de rampe, 10px sombre | idem |
| Corps du libellé | 17px (encre 13) | comme tous les libellés du jeu |

**Le libellé est cerné sur 1px dans les huit directions, d'une version assombrie de sa propre
couleur** (`TAB_LABEL_OUTLINE_FACTOR` = 0,205). Deux couleurs de libellé de la même capture le
confirment et écartent les autres lectures : doré `#f4d89e` → cerne `#312c21`, blanc `#ffffff` →
cerne `#353534`. Un cerne noir translucide donnerait une couleur proportionnelle au *fond*, or le
fond de l'onglet actif est un kaki chaud et son cerne est gris neutre ; un cerne noir opaque
donnerait du noir. Ce n'est donc ni le cerne noir de `design::text::paint_outlined_text` (titre de
modale, dégâts de combat), ni la graisse synthétique retirée en 2026-09-09 — huit copies de la même
couleur empâtent, huit copies plus sombres détourent. Les boutons du jeu n'ont pas ce cerne (anneau à
4 % du fond sur les deux captures de pied de page).

**Les trois couleurs relevées pour le séparateur sont un seul dégradé.** `#837d70` (relevé),
`#6d6657` (asset détouré) et `#595140` (capture de la modale, ligne y=110) ont longtemps semblé se
contredire, et le jeton tranchait pour la troisième. Aucune n'est fausse : ce sont trois hauteurs du
même trait. egui ne remplissant pas un rectangle en dégradé, le trait est peint en maillage — deux
plateaux constants et une rampe interpolée par le GPU, exact à n'importe quelle hauteur de barre.

**Les 6px de gouttière du relevé sont ceux du remplissage**, pas de la boîte : chaque onglet porte
ses deux bords de 2px, le composant les pose donc à 2px l'un de l'autre et peint le séparateur dans
cet intervalle.

**Le rayon n'est pas sur l'onglet, il est sur la barre.** Le premier segment de l'asset a ses deux
coins gauches arrondis, le dernier ses deux coins droits ; les segments du milieu sont parfaitement
droits. L'escalier d'alpha retire `4, 2, 1` pixels sur les trois premières lignes et `1, 2, 3` sur
les trois dernières — le haut est creusé d'un pixel de plus que le bas, sur les deux côtés, de façon
cohérente aux quatre angles : c'est la forme du jeu, pas du bruit de détourage, et elle est conservée
telle quelle.

**L'arrondi est porté par l'alpha de la texture**, comme celui d'un bouton, d'où quatre fichiers
d'extrémité en plus des deux du milieu. Les deux autres voies n'en sont pas : un `Mesh` egui ne sait
pas découper un coin, et peindre un patch arrondi par-dessus n'efface pas le coin carré du dessous —
l'alpha compose, il ne soustrait pas.

| Fichier | Origine | Coins arrondis |
| --- | --- | --- |
| `tab-active-first.png` | 1er segment de la capture, **coin gardé** (un pixel opaque isolé retiré de l'angle bas-gauche) | gauche |
| `tab-active-last.png` | miroir horizontal du précédent | droite |
| `tab-inactive-last.png` | dernier segment de la capture | droite |
| `tab-inactive-first.png` | miroir horizontal du précédent | gauche |
| `tab-active.png` / `tab-inactive.png` | 1er segment coin redressé / segment du milieu | aucun (onglets du milieu) |

Le jeu n'a pas de capture d'onglet actif en fin de barre, d'où deux miroirs — le corps d'un onglet
étant un dégradé **vertical** et ses deux bords identiques, le miroir ne change rien. Une barre à
**un seul onglet** n'existe pas dans le jeu : aucune texture n'a ses quatre coins arrondis, le
composant y superpose ses deux extrémités, chacune écrêtée à sa moitié.

**Le cerne de la barre est continu, gouttières comprises** (`TAB_BORDER` = `#1a1d1f`, moyenne des 774
pixels opaques de la ligne y=0 de l'asset). Il vient de la texture de chaque onglet sauf dans les
gouttières de 2px, qui n'appartiennent à aucun onglet : sans peinture explicite, le fond du panneau y
traverse la barre de part en part, et le cerne se retrouve entaillé de deux encoches au droit de
chaque séparateur.

**Largeur : parts égales sur toute la largeur disponible, et c'est un choix, pas un relevé.** Le jeu
dimensionne chaque onglet sur son libellé — ses six onglets font 77, 83, 103, 83, 133 et 106px pour
des encres de 26, 44, 73, 28, 100 et 35 : ni un padding constant, ni le nombre de caractères, ni une
largeur minimale unique n'en rendent compte. Sa barre ne remplit d'ailleurs pas la fenêtre (elle
s'arrête à x=636 sur 705) parce que le **bouton de réinitialisation** occupe la droite. Appliquer une
règle qu'on n'a pas mesurée à trois onglets qui n'ont pas ce bouton laissait la barre à 337px sur les
743 du panneau, calée à gauche. `fit_content()` rend l'autre comportement, pour comparer à une
capture du jeu ou pour une barre qui ne doit pas s'étirer.

**Inventé, faute de référence** :

- **Le padding de `fit_content()`.** Faute de règle retrouvable (ci-dessus) : le padding stable sur
  les deux libellés longs (`TAB_PADDING_X` = 16) et un plancher au plus petit onglet relevé
  (`TAB_MIN_WIDTH` = 77). Les libellés longs tombent à 1 et 6px de la référence, les courts
  ressortent plus étroits.
- **L'état désactivé.** Aucune capture d'onglet grisé ; fond inactif et libellé `TEXT_DISABLED`, par
  cohérence avec le bouton désactivé.

**Remplace** : la barre peinte à la main de `panels::options_modal` — texture `menu-tabs.png` de
44px étirée à 31, libellés en `FontId::proportional(12.0)`, aucun clic. Elle partageait déjà la
largeur en trois tiers égaux ; c'est le composant qui s'en était écarté, le temps de deux captures.

### La variante pictogramme (2026-09-11)

```rust
design::tabs(&mut vue)
    .entry(Vue::Combat, "Combat").icon(DsIcon::Cards)
    .entry(Vue::Suivi, "Suivi").icon(DsIcon::Trophy)
    .show(ui);
```

`.icon(...)` s'applique à la **dernière entrée déclarée**, comme `.enabled(...)`. Le pictogramme
**remplace le libellé au rendu** — et le libellé reste, ce qui n'est pas une commodité d'API :

- il devient l'**infobulle** de l'onglet (`design::tooltip`), sans quoi une barre de pictogrammes
  n'apprend à personne ce que fait chaque onglet ;
- il reste la **ligne de journal**, sans quoi on ne saurait plus nommer ce qui a été cliqué.

C'est aussi pourquoi cette variante attendait le lot 2 : elle a besoin de `DsIcon` pour le glyphe
**et** de `design::tooltip` pour le mot.

**Mesures** — `assets/design-system/icon-tabs.png` (268 × 44), un gabarit à quatre onglets :

| Grandeur | Valeur |
| --- | --- |
| Largeur d'un onglet | **66 px** (`TAB_ICON_WIDTH`) — crêtes de séparation à x=65, 133, 201, soit un pas de 68 dont 2 de gouttière |
| Hauteur | 44 px, la même que la variante texte — les deux partagent leurs textures |
| Encre du pictogramme | **dérivée**, `TAB_ICON_RATIO` = 18/36 → 22 px sur 44 |

Un onglet à pictogramme est donc **plus étroit** qu'un onglet texte (77 px de plancher) : il n'a pas
de mot à contenir. En mode étiré (le défaut), il suit la même règle de parts égales que la variante
texte ; `fit_content` lui donne les 66 px du jeu.

**Le ratio d'encre est dérivé, pas mesuré**, et c'est dit dans le jeton : `icon-tabs.png` est un
gabarit **vide** — le jeu n'y a laissé aucun pictogramme. La valeur reprend le rapport du bouton
icône (`ICON_BUTTON_CONTENT` sur `ICON_BUTTON_SIZE`), le seul rapport glyphe/socle que le design
system ait mesuré. À remplacer dès qu'une capture d'onglets à pictogrammes existera.

**Le pictogramme prend la teinte du libellé**, pas une teinte propre : il dit la même chose qu'un
mot d'onglet, il doit changer avec l'état de la même façon — blanc quand l'onglet est actif, doré
sinon, gris quand il est désactivé.

Vérifié au rendu : les crêtes de séparation de la galerie tombent à un pas de 68 px, crête de 2 —
**la cote du jeu au pixel**.

---

## `design::checkbox` — case à cocher (2026-09-10)

`crates/overlay-ui/src/design/components/checkbox.rs`

```rust
use overlay_ui::design;

if ui.add(design::checkbox(&mut state.alertes_sonores, "Alertes sonores")).changed() {
    // la frame où la case vient d'être basculée
}
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `enabled` | `bool` | `true` |
| `tooltip` | texte d'infobulle | aucune |
| `log_name` | nom d'instance pour le journal | le libellé |
| `preview_state` | `Idle` / `Hovered` / `Disabled` — **galerie et captures uniquement** | état réel |

**Le libellé porte l'état autant que la case** : blanc décoché, doré `#f4d89e` coché. C'est le même
principe que la barre d'onglets — le jeu ne se repose jamais sur un seul signal visuel. Un portage
qui ne changerait que la case perdrait la moitié de l'information.

**Toute la ligne est cliquable**, case et libellé : c'est ce que fait le jeu, et viser un carré de
20px à la souris est une punition.

**Mesures** (`releve-section-options.json`, nœuds `cb1` à `cb3` et leurs libellés) :

| Grandeur | Valeur | Origine |
| --- | --- | --- |
| Case | 20 × 20px | `cb1` en `[36, 173, 56, 193]`, confirmé par les deux assets 20 × 20 |
| Rayon | **0** | **le seul élément carré de l'interface** — tout le reste est à 2, le champ à 4 |
| Case → libellé | 6px | la case finit à x=56, le libellé commence à x=62 |
| Corps du libellé | 15px (encre 10) | jeton `libellé d'option` — plus petit qu'un libellé de bouton |
| Libellé décoché / coché | `#ffffff` / `#f4d89e` | relevé |
| **Pas entre deux lignes** | **31px**, soit **11px de blanc** | « Rythme : 31 px entre deux lignes d'option consécutives » — vérifié au pixel (jeu : case 1 à y=175, case 2 à y=206) et sur les onze lignes de l'onglet Chat, dont les pas vont de 30 à 31 |

**L'interligne est un jeton de MISE EN PAGE** (`tokens::CHECKBOX_ROW_GAP`, 11px), pas une affaire du
composant : `design::checkbox` alloue la hauteur d'une ligne et rien de plus, et `design::panel` met
`item_spacing.y` à zéro pour que chaque écart soit posé explicitement. Un écart porté par le
composant s'ajouterait au `SECTION_GAP` qui suit la dernière ligne d'un bloc. Il se pose donc
**entre** deux lignes, jamais après la dernière.

> Personne ne l'avait posé jusqu'au 2026-09-14 : deux cases empilées se suivaient à 20px — la
> hauteur de la case, sans un pixel de blanc — contre 31 dans le jeu. Retour utilisateur, capture du
> jeu à l'appui : « l'espacement entre les lignes du design system du jeu est beaucoup plus élevé
> que celui qui est utilisé dans la modale de l'overlay ». Les candidats ont été rendus par le vrai
> moteur avant de trancher (`cargo run -p overlay-testkit --example interligne-options`).

**Textures** : `checkbox-true.png` et `checkbox-false.png`, 20 × 20 toutes les deux — la taille
relevée exactement. Leur découpage 9-slice (5px figés) n'existe que pour honorer la règle « toute
taille est valide » : en pratique une case est toujours peinte à sa taille native.

**Inventé, faute de capture** : l'état **survolé** ne change rien (seul le curseur change), et
l'état **désactivé** teinte la case et le libellé de `TEXT_DISABLED`, par cohérence avec le bouton
désactivé.

**Usages** : les quatre sections de l'onglet « Paramètres » — « Combat » (trois lignes, dont la
dernière en retrait sous celle dont elle dépend) puis « Suivi », « Alertes » et « Chat », où les
cases « Couper le son des notifications » et « Fermeture automatique des notifications » sont
seules sur leur ligne (`panels::notifications`, 2026-09-15).

---

## `design::select` — liste déroulante (2026-09-10)

`crates/overlay-ui/src/design/components/select.rs`

```rust
use overlay_ui::design;

ui.add(
    design::select(&mut state.theme)
        .option(Theme::Sombre, "Sombre")
        .option(Theme::Clair, "Clair")
        .width(560.0),
);

ui.add(
    design::select_multi(&mut state.raretes)
        .option(Rarete::Commun, "Commun")
        .option(Rarete::Rare, "Rare")
        .summary("Toutes"),
);
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `option(valeur, libellé)` | une entrée, dans l'ordre d'affichage | — |
| `width` | largeur imposée | toute la place disponible |
| `placeholder` | libellé du socle si la valeur ne correspond à aucune option (choix simple) | vide |
| `summary` | libellé du socle en choix multiple (« Toutes ») | « n sélectionnées » |
| `enabled` | `bool` | `true` |
| `log_name` | nom d'instance pour le journal | `"select"` |
| `preview_state` / `preview_open` / `preview_hovered` | **galerie et captures uniquement** | état réel |

Un choix simple **referme** la liste au clic ; un choix multiple la garde ouverte, pour qu'on puisse
en cocher plusieurs. Chaque entrée du mode multiple porte la case de `design::checkbox`.

**C'est le seul composant qui peint hors de son rectangle.** La liste dépliée vit dans une
`egui::Area` au premier plan : sans ça, le widget suivant la recouvrirait, et la place qu'elle occupe
décalerait la mise en page à chaque ouverture. L'état ouvert/fermé reste dans la mémoire d'egui —
c'est de l'état d'**interaction**, pas de l'applicatif que le contrat interdit, du même ordre que
« ce widget a le focus ». `egui::ComboBox` procède de même.

**Mesures** (`select-simple.png` colonne x=60, `select-simple-opened.png` colonne x=100, recoupées
avec le nœud `select-theme` de `releve-options-interface.json`) :

| Grandeur | Valeur | Origine |
| --- | --- | --- |
| Hauteur du socle | **36px, bord compris** | bord 2 + liseré 2 + dégradé 28 + ombre 2 + bord 2 |
| Rayon du socle, et des coins **hauts** de la liste | 2 | comme tout le reste |
| Rayon des coins **bas** de la liste | **4** | fond rentrant de 3, 2, 1, 0px sur les quatre dernières lignes (x=7 en y=151, x=8 en 152, x=9 en 153, bord noir à x=10 en 154) |
| Chevron | 14 × 8px, à 8px du bord droit | **la taille native d'`icons/icon-chevron-down.png`**, au pixel |
| Retrait du libellé de socle | 10px | texte à x=17 pour un socle à x=7 |
| Hauteur d'une entrée | 28px | surbrillance en y 71..98, entrée suivante à y=99 |
| Retrait du texte d'une entrée | 12px | « Tous » à x=19 pour une liste à x=7 |
| Fond de liste | `#675d46` | uniforme, aucun dégradé |
| Entrée mise en avant | `#a58e63` | seul fond de mise en avant relevé |

**Les bords de la liste dépliée** (corrigés le 2026-09-14, sur capture de l'utilisateur) tiennent
entièrement à l'**ordre de peinture** : fond, puis les entrées, puis le liseré clair et le bord. Dans
l'ordre inverse, la mise en avant de l'entrée du haut mangeait le liseré et celle du bas recouvrait
le bord — la liste avait l'air de déborder de son conteneur, et le liseré n'existait que dans le
code, deux pixels de bord noir se posant par-dessus son unique pixel clair. Trois conséquences à
retenir :

- le liseré et le bord **rognent d'un pixel** la première et la dernière entrée, exactement comme la
  référence (27px utiles pour un pas de 28) ;
- la mise en avant **porte les rayons du conteneur** aux extrémités, sans quoi elle ressort, carrée,
  par les coins arrondis ;
- le bas est **plus rond que le haut** (4 contre 2) : le haut est collé au socle, il n'a pas à s'en
  détacher, le bas flotte au-dessus du contenu. Le jeu ne met d'ailleurs aucun bord en haut de la
  liste — le bord bas du socle en tient lieu — et enchaîne directement sur le liseré.

**Le piège de la hauteur**, énoncé par le relevé : « le chiffre de 32px qu'on lit en mesurant le
remplissage est trompeur — il exclut les 2px de bord haut et bas. »

**Aucune largeur par défaut.** Le relevé mesure 210, 208, 560 et 650px selon le contrôle : « la
largeur est décidée contrôle par contrôle ». Le composant prend donc la place disponible, comme
`design::input`.

**Texture** : `select-face.png` (220 × 36), découpée de `select-simple.png` avec libellé et chevron
retirés — le socle étant un dégradé purement vertical, une colonne propre répétée le reconstruit
exactement, sans interpolation.

**Inventé, faute de référence** :

- **La distinction survolée / valeur courante.** Une seule capture montre une entrée sur fond clair,
  et elle est à la fois la valeur du socle et, probablement, celle que la souris survolait. Le
  composant applique le même fond aux deux.
- **Le gabarit du mode multiple.** `select-multiple.png` donne 26px d'entrée contre 28 pour le
  simple ; les deux captures ne sont pas à la même échelle d'interface. Les cotes du **simple** sont
  appliquées dans les deux modes.
- **L'état désactivé** : socle et libellé en `TEXT_DISABLED`, la liste ne s'ouvre pas.

---

## `design::scroll_area` — zone défilable (2026-09-10)

`crates/overlay-ui/src/design/components/scroll_area.rs`

```rust
use overlay_ui::design;

design::scroll_area("options-contenu").show(ui, |ui| {
    // le contenu qui peut déborder
});
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `id_salt` (à la construction) | distingue deux zones du même panneau ; porte la position de défilement | — |
| `auto_shrink` | laisse la zone se rétrécir à son contenu, **sur l'axe qui défile** | `false` — un panneau du jeu occupe toute sa hauteur |
| `axis` | `Vertical` (barre à droite) ou `Horizontal` (barre dessous) | `Vertical` |
| `outer_margin` | marge poignée → bord de la zone | 14px, la marge du relevé |

**Horizontale depuis le 2026-09-13** (`ScrollAxis::Horizontal`). La demande vient du bandeau
« Suivi », dont la bande de tuiles défile de gauche à droite : « utiliser le scroll qui est déjà
utilisé pour la modale dans l'onglet Raccourcis […] gris quand l'utilisateur n'a pas sa souris
dessus et doré quand il passe sa souris dessus, c'est mieux dans l'ADN du jeu ». Rien à remesurer :
la barre du jeu ne change pas d'aspect parce qu'elle change d'axe — mêmes jetons, même épaisseur
**constante** (c'est précisément ce qui a fait rejeter `ScrollStyle::thin()` d'egui, qui grossit au
survol), pas de rail.

Deux réglages sont venus avec, parce qu'un bandeau posé sur le jeu n'est pas un panneau :

- `outer_margin` — les 14px du relevé séparent la poignée du bord d'un **panneau**. Le bandeau n'en
  a pas : il descend à 2px, et la réserve totale sous les tuiles tombe à 14px.
- `auto_shrink` ne vaut plus que pour l'axe qui défile ; l'axe croisé se rétracte toujours en
  horizontal (une bande prend la hauteur de ses tuiles, pas celle de la fenêtre). **Le mettre à
  `true` sur une zone bornée par son parent fait disparaître la barre** : egui dimensionne alors la
  zone sur son contenu, ne voit plus de débordement, et le contenu sort du cadre sans rien pour le
  signaler.

**Ce n'est pas un `Widget`**, et depuis le 2026-09-10 ce n'est plus un écart : c'est le premier
**composant conteneur** du design system (§1 bis du contrat, *forme closure*). Il prend une closure
de contenu, il ne peut donc pas rendre une `Response` à partir de rien — `egui::ScrollArea` n'en est
pas un non plus, pour la même raison.

**Variante « cadre » — hors de ce composant, et c'est délibéré.** La barre du cadre ennemi du
panneau Combat (`panels::combat_frame_scroll`) ne ressemble pas à celle-ci : barre dessinée de 5 px,
couleur unie `#998a6c`, bordure noire, toujours visible. Ce n'est **pas** une divergence à
« corriger » — chacun de ces écarts est une demande explicite de l'utilisateur, et les assets du jeu
(`scrollbar-active.png` / `scrollbar-inactive.png`) y ont été essayés puis **rejetés** (« trop
large, cachait les portraits »), voir l'en-tête de module de ce fichier. Le contexte diffère du tout
au tout : une barre posée *sur* un décor de cadre, pas dans la gouttière d'un panneau. Elle reste
locale au panneau tant qu'elle n'a qu'un utilisateur ; si un second apparaît, elle remonte ici en
paramètre plutôt qu'en second composant.

**Aucun rail.** Le relevé est catégorique : « le fond du panneau tient lieu de gouttière ». C'est ce
qui rend ce composant particulier — `egui::ScrollArea` peint par défaut un rail derrière sa poignée,
et `extreme_bg_color` est le seul jeton qu'elle consulte pour ce fond. Il est mis à transparent dans
un `scope`, pour ne pas fuir vers le reste du panneau.

**Mesures** (`releve-modale-options.json`, nœuds `scrollbar-thumb` et `panel`, recoupés avec les deux
assets ligne y=150) :

| Grandeur | Valeur | Origine |
| --- | --- | --- |
| Poignée | 6px de large | x 685..691 sur la modale, x 5..10 sur les assets |
| Rayon | 3 | relevé |
| Poignée au repos | `#515356` | relevé de la modale (l'asset donne `#5e5f62`, §5.9 `#5c5e61`) |
| Poignée survolée / tirée | `#c1ad83` | `scrollbar-active.png` |
| Marge contenu → poignée | 6px | 679 → 685 |
| Marge poignée → bord | 14px | 691 → 705 |
| **Réserve totale** | **26px** | 679 → 705, « même quand la barre ne sert pas » |

**`design::components::scroll_area::RESERVE_X` vaut ces 26px**, et c'est la somme des trois marges.

### `bar_before` — la barre devant le contenu (2026-09-13)

Le bandeau « Suivi » la veut **au-dessus** de ses tuiles, à deux pixels du bord de la fenêtre :
c'est alors la seule chose qui l'éloigne encore du haut du jeu, et le bas reste libre pour les
infobulles, qui retrouvent leur écart mesuré à la tuile (5 px) au lieu d'être repoussées au pied de
la zone.

`egui` ne sait pas le faire — sa barre horizontale se cale sur le bord bas du rectangle de la zone,
et `ScrollArea::scroll_bar_rect` ne la déplace que le long de son propre axe, pour un en-tête
collant. Le composant masque donc la barre d'egui (`ScrollBarVisibility::AlwaysHidden`) et peint la
sienne : réserve en tête, poignée, survol, glissé. Les proportions et le geste sont repris du code
d'egui — dont le point important, **le glissé suit la position du pointeur et non un cumul de
`drag_delta`** : un cumul perd le mouvement arrivé dans la même frame que le relâchement, ce qui
suffit à rendre la barre inerte sous un harnais de test.

Deux gains en échange du code : la réserve n'existe **que lorsque la barre sert** (une bande qui
tient entière ne décale plus rien — egui, lui, réserve dès qu'il affiche), et une mise en page peut
s'aligner dessus. `ScrollArea::space_before(ui)` rend cette place ; dans le bandeau, le carré de
contrôle en descend pour rester à hauteur des tuiles plutôt que de la barre. La réponse vient de la
frame précédente : personne ne sait avant de l'avoir peinte si une bande déborde.
Une mise en page peut donc réserver la place **avant** que la barre existe, sans qu'un pixel de
contenu ne bouge le jour où elle apparaît — c'est ce que fait `panels::options_modal`, ce qui lève la
déviation qu'il documentait (19px au lieu de 26, faute de barre à y mettre).

**Non reproduit** : l'**ombre portée de 2px** à droite de la poignée. `egui::ScrollArea` peint sa
poignée elle-même et n'expose aucun point d'accroche pour l'ombrer ; la reproduire demanderait de
recalculer sa position hors d'egui — un doublon fragile de son propre calcul, pour deux pixels
sombres sur un fond déjà sombre.

---

## `design::icon_button` — bouton icône (2026-09-10)

`crates/overlay-ui/src/design/components/icon_button.rs`

```rust
use overlay_ui::design::{self, DsTexture, IconContext};

if ui
    .add(design::icon_button(DsTexture::IconOption).context(IconContext::FirstPlan))
    .clicked()
{ … }
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `context` | `FirstPlan` (par-dessus le jeu), `Panel` (dans un panneau), `Stepper` (pas numérique), `Banner` (croix de fermeture d'une fenêtre) | `FirstPlan` |
| `size` | côté du bouton | 36px, la taille native des cinq socles |
| `enabled` | `bool` | `true` |
| `tooltip` / `log_name` | infobulle, nom d'instance | aucune / `"icon-button"` |
| `preview_state` | `Idle` / `Hovered` / `Disabled` — **galerie et captures uniquement** | état réel |

**Textures** : `button-icon.png`, `button-icon-first-plan.png` et leurs `-hover` (36 × 36 toutes les
quatre), plus `button-icon-disabled.png` **partagée par les deux contextes** — le jeu n'a capturé
qu'un socle grisé. Les glyphes viennent de `assets/design-system/icons/` ; **le manifeste ne porte
que ceux qui servent** (le dossier en compte plus de trente, et `DesignSystem::load` les téléverse
tous dès le premier composant peint).

**Teintes** : `#c5cbcc` au repos, `#f4d89f` au survol, mesurées sur
`menu-button-icon-first-plan.png`. Les icônes du design system étant blanc pur avec alpha, une
teinte appliquée au moment de peindre suffit — pas de copie recolorée à charger.

**Seul `FirstPlan` dore son glyphe au survol.** Ces deux teintes ont été mesurées sur une planche de
socles *bleus* — la barre de premier plan — et s'appliquaient par défaut aux quatre contextes faute
de mesure ailleurs. Retour utilisateur 2026-09-14, deux captures du jeu à l'appui (« Jouer le son
d'alerte », au repos et survolé) : sur le socle kaki d'un panneau, le glyphe est **blanc et le reste
au survol**, seule la texture du socle s'éclaircit (`tokens::PANEL_ICON_TINT`). Les trois autres
contextes figent donc leur teinte — blanc pour `Panel`, or pour `Stepper` et `Banner` — et
`IconContext::icon_tint` en est le seul point de décision :

| Contexte | Glyphe au repos | Glyphe au survol |
| --- | --- | --- |
| `FirstPlan` | `#c5cbcc` | `#f4d89f` |
| `Panel` | blanc | blanc |
| `Stepper` | `#f4d89f` | `#f4d89f` |
| `Banner` | `#f4d89f` | `#f4d89f` |

**Taille d'encre** : les glyphes sont détourés au pixel près, donc de tailles inégales d'un fichier
à l'autre (13, 14, 16). Le jeu les cale sur une grille commune — **18px d'encre pour un socle de
36**, médiane des huit icônes de `menu-button-icon-first-plan.png` (plage 16–20, seuil de luminance
140, invariant de 120 à 180). C'est `tokens::ICON_BUTTON_CONTENT`, appliqué par
`DsTexture::icon_content_size` : **le manifeste, pas l'appelant** — la taille d'encre est une
propriété de l'asset. Trois tests l'ancrent : la médiane remesurée sur la capture du jeu, le
détourage des glyphes, et le calcul de `icon_draw_size` (dont le cas du « − », qu'un mauvais facteur
transformerait en barre).

**Échelle et survol** : le socle et l'icône partagent le même facteur d'échelle (dérivé de la
largeur du socle), et un appui de souris retire l'apparence survolée, qui revient au relâchement —
la règle commune à toute l'interface.

**État désactivé — deux mécaniques, une par contexte.** `button-icon-disabled.png` est le socle
grisé du jeu, et il appartient au contexte `Panel` : luminance moyenne 60, contre 81 pour
`button-icon.png`, le socle actif du même contexte. Posé sur une barre de premier plan (41), il
s'inverse — le bouton désactivé devient le plus lumineux de la barre. En `FirstPlan`, le composant
garde donc le socle de repos et l'assombrit (`tokens::DISABLED_DIM`, ~43 % d'opacité).

**Utilisé en production** depuis le 2026-09-10 : les quatre boutons du carré de contrôle du panneau
Suivi (« + », « − », Détails, Options — `panels::watchlist::control_button`). Ce qui reste à la
charge du panneau : le **placement de l'infobulle** par colonne (à gauche pour « + » et Détails, à
droite pour « − » et Options), que le `.tooltip()` du composant ne sait pas reproduire — il retombe
sur le placement par défaut d'egui.

**Le cas « réinitialisation »** attend qu'un réglage réinitialisable existe. Note du relevé à ne pas
« corriger » le jour venu : dans la barre d'onglets, **l'axe du bouton est 9px plus bas que celui des
onglets**.

**Paire volume/muet (2026-09-11)** : `DsTexture::IconVolume` (26 × 22) et `DsTexture::IconVolumeMute`
(26 × 26), détourées par le skill `design-asset` depuis deux captures du jeu sans socle porteur —
contrairement aux autres glyphes de la table, prélevés sur un bouton. Entrées au manifeste et à la
galerie de contrôle avant tout appelant réel, comme `DsTexture::ModalHeader` l'avait été pour sa
bannière : préparées pour le futur bouton muet/actif de la fenêtre Options
(`interface-options-son.png`), aucun réglage de son n'étant câblé dans l'overlay à ce jour
(`alert_sound` ne fait que jouer les sons, jamais les couper).

**Paire œil/œil barré (2026-09-11)** : `DsTexture::IconEye` et `DsTexture::IconEyeOff` (16 × 14
toutes les deux), même provenance et même statut que la paire volume/muet — détourées par le skill
`design-asset` depuis deux crops sans socle porteur, entrées au manifeste et à la galerie avant tout
appelant réel. Candidat naturel pour un futur toggle de visibilité (masquer une entrée du panneau
Suivi, ou un champ de jeton dans la fenêtre Options) ; aucun des deux n'est câblé aujourd'hui.

**Lot complet du répertoire `icons/` (2026-09-11)** : delta entre `assets/design-system/icons/*.png`
(38 fichiers) et `DsTexture::ALL` demandé par l'utilisateur (« pas tous les icônes du répertoire
dans le manifeste ») — 22 fichiers manquaient, tous ajoutés d'un coup : `IconBagIn`, `IconBagOut`,
`IconBook`, `IconCalendar`, `IconCards`, `IconCharacters`, `IconFilter`, `IconGrid`, `IconHammer`,
`IconKamas`, `IconLock`, `IconOrder`, `IconPact`, `IconPin`, `IconRepeat`, `IconSave`,
`IconSettings1`, `IconSettings2`, `IconSort`, `IconTriangleRight`, `IconTrophy`, `IconXp`. Aucun n'a
d'appelant réel, comme les deux paires ci-dessus.

`icon_content_size` n'est posé QUE sur les neuf dont l'extraction `--from-button` est consignée dans
`references/recettes-icones.md` du skill `design-asset` (preuve qu'ils vivaient sur un socle de
bouton icône dans le jeu) : `IconBagIn`, `IconBagOut`, `IconFilter`, `IconLock`, `IconOrder`,
`IconPact`, `IconSave`, `IconSort`, `IconTriangleRight`. Les treize autres (`IconBook`,
`IconCalendar`, `IconCards`, `IconCharacters`, `IconGrid`, `IconHammer`, `IconKamas`, `IconPin`,
`IconRepeat`, `IconSettings1`, `IconSettings2`, `IconTrophy`, `IconXp`) n'ont pas cette preuve — soit
détourés sans bouton porteur (documenté), soit sans mesure consignée (provenance retrouvée dans
l'historique git, pas dans le jeu d'essai du skill) — et restent donc sans normalisation, par la
même prudence que `IconSearch`/`IconTick`/`IconChevronDown` : une taille d'encre est une propriété
mesurée de l'asset, jamais devinée. Peints dans la galerie à leur taille de fichier plutôt que par
`icon_button`, faute de socle à leur donner (nouvelle section « Glyphes sans socle connu »).

`crates/overlay-testkit/tests/design_gallery.rs` : les deux rangées d'icônes sur socle sont passées
de `ui.horizontal` à `ui.horizontal_wrapped`, la largeur fixe de 760px ne contenant plus tous les
glyphes sur une seule ligne — et la hauteur du canevas de test est passée de 4160 à 4500px pour la
même raison (le bas de la galerie sortait sinon du cadre).

**Contexte `Banner` (2026-09-13)** — la croix de fermeture que `design::window` pose en haut à
droite de sa bannière. **Pas de texture, et ce n'est pas une dérogation** : dans le jeu, le bouton
est un carré arrondi *translucide* de 32 px posé sur la bannière, dont les hachures se voient au
travers dans les deux états (`window-close.png`, `window-close-hover.png`, deux recadrages 48 × 48
des captures utilisateur, échelle 1). Une texture le figerait avec un morceau de bannière dedans. Le
socle est donc un voile noir peint : α 0,168 au repos avec un liseré d'1 px (α 0,115 par-dessus),
α 0,366 au survol sans liseré — mesurés par `tools/design-system/build_window_close.py`, qui
extrait aussi le glyphe. Croix `DsIcon::CloseWindow` (`icon-close-window.png`, 12 × 12, plus
grasse que `DsIcon::Close` et légèrement asymétrique comme dans le jeu), **dorée `#F4D89F` dans les
deux états** : le survol se lit sur le voile, pas sur la croix. Rayon 5, encre 12 pour 32. Jetons
`WINDOW_CLOSE_*`. L'état désactivé n'existe que parce que le contrat l'exige (voile de repos, croix
effacée) — le jeu ne grise jamais sa croix.

Vérifié sur les mêmes pixels avant/après (snapshot HEAD contre snapshot régénéré) : intérieur à
82,8 % de la bannière au repos (jeu 83,2), liseré 73,3 (73,6), survol 63,5 (63,4).

---

## `design::window` — chrome de fenêtre (2026-09-10)

`crates/overlay-ui/src/design/components/window.rs` — **composant conteneur**, forme « zone
rendue » (§1 bis du contrat).

```rust
let chrome = design::window("Options")
    .footer("Annuler", "Valider")
    .log_name("options")
    .show(ui);

chrome.tabs(ui, design::tabs(&mut state.tab).entry(Tab::Parametres, "Paramètres"));
match chrome.footer { design::FooterClick::Validate => …, _ => {} }
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `title` (à la construction) | peint dans la bannière, serif grasse cernée d'une ombre bas-droite | — |
| `tab_bar_height` | hauteur réservée à la barre d'onglets ; `0.0` pour une fenêtre sans onglets | `tokens::TAB_HEIGHT` (44) |
| `footer` | libellés des deux boutons — annulation à gauche, validation à droite | aucun pied de page |
| `close_button` | croix de fermeture en haut à droite de la bannière (2026-09-13) | `false` |
| `log_name` | préfixe des deux boutons du pied et de la croix dans le journal | `fenetre` |

Rend un `WindowChrome` : `tab_bar` (la bande d'onglets), `content` (entre onglets et pied),
`footer` (le clic reçu) et `close` (la croix vient d'être cliquée). **`content` n'est pas écrêtée** —
c'est le prix de la forme « zone rendue », et la raison pour laquelle le contenu passe normalement
par `design::panel`.

**La croix de la bannière (2026-09-13)** — un `icon_button` en contexte `Banner` (voir ci-dessus),
32 px, à 12 px du bord droit et centré dans la bannière de 56 (les deux cotes du jeu se confondent :
12 + 32 + 12). Peinte APRÈS le titre pour rester lisible sur une fenêtre étroite. Le chrome ne dit
que « cliquée » : ce que fermer veut dire appartient à l'appelant, comme pour « Annuler » — dans la
modale Options, les deux gestes passent par la même garde et produisent la même action
(`options_croix_de_la_banniere_ferme_comme_annuler`). Position ancrée par
`la_croix_est_centree_dans_la_banniere_a_la_marge_du_bord_droit`.

**La barre d'onglets n'est pas peinte par le chrome** : `WindowChrome::tabs` la pose à partir du
`Tabs` que l'appelant construit. Un onglet est du contenu, pas du décor — et c'est aussi ce qui
évite de rendre la fenêtre générique sur le type d'onglet de son contenu.

**Utilisé en production** : la modale Options (`panels::options_modal`) et les maquettes de la page
Alertes (`crates/overlay-testkit/examples/alertes-mockups.rs`).

---

## `design::panel` — panneau de contenu (2026-09-10)

`crates/overlay-ui/src/design/components/panel.rs` — **composant conteneur**, forme closure.

```rust
design::panel().show(ui, chrome.content, |ui, panel| {
    ui.add(design::heading("Fichier"));
    ui.add(design::input(&mut state.path));
    panel.scroll_area(ui, "options-contenu", |ui, width| { … });
});
```

**Un panneau n'est pas une section.** Le relevé est catégorique : dans le jeu, une *section* n'a ni
fond, ni bordure, ni filet — son seul signal de regroupement est l'espacement, et son seul signal de
niveau le retrait de 7 px de son titre. Ce qui a un fond (`#15181c`), un bord (2 px `#131518`) et un
rayon (2), c'est le **panneau** qui contient les sections.

Ce qu'il fait pour son contenu, et qu'aucun appelant n'a donc plus à faire : les rembourrages, la
réserve de barre de défilement à droite (26 px, **toujours posée**, comme le jeu), l'écrêtage — élargi
à gauche du retrait des titres, sans quoi un titre de section perd sa première lettre, **et à droite
jusqu'au bord du panneau**, sans quoi la poignée de défilement est rognée —, et la mise à zéro de
l'espacement implicite d'egui.

**Correctif du 2026-09-13** : l'écrêtage était calé sur `inner`, qui exclut la réserve de barre ; comme
`PanelZones::scroll_area` intersecte son propre clip avec celui du panneau, la poignée peinte dans
cette réserve n'apparaissait **jamais**, quel que soit le contenu. Personne ne l'avait vu parce
qu'aucune grille d'Alertes ou de Suivi ne défilait encore — c'est le fil de messages des maquettes
de l'onglet Chat (`crates/overlay-testkit/examples/chat-mockups.rs`) qui l'a révélé.

**Utilisé en production** : la modale Options, les maquettes de la page Alertes et celles de l'onglet
Chat.

---

## `design::heading` — titre de section (2026-09-10)

`crates/overlay-ui/src/design/components/heading.rs` — composant **feuille**.

```rust
ui.add(design::heading("Fichier"));
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `text` (à la construction) | le titre | — |
| `preview_trailing_gap` | écart sous le titre — **planches de simulation uniquement** | `tokens::HEADING_TO_ROW` (13) |

Serif grasse au corps du titre de fenêtre (21), **gris `#b8b9ba` et non blanc** : la hiérarchie
entre les deux niveaux de titre du jeu passe par la couleur, pas par le corps.

Deux pièges que le composant absorbe : il réserve la hauteur d'**encre** (16) et non celle de sa
galley — réserver la galley ajoutait ~14 px invisibles sous le titre, sur sept relevés — et il
applique lui-même le **retrait de 7 px** qui est, dans le jeu, le seul signal qu'une section existe.

**Ce qui se relève n'est pas l'écart sous le titre, mais la cote d'ensemble** : 30 px du haut du
titre au haut de la première ligne. Vérifiée au pixel sur les sept sections des cinq onglets du jeu
où un titre est immédiatement suivi d'un contrôle (30, 30, 30, 30, 30, 28, 32 — l'écart tient à la
rondeur de la capitale initiale). L'encre réservant 16 px, il reste **13 px** sous le titre.

> `HEADING_TO_ROW` valait 7 jusqu'au 2026-09-14 : l'overlay posait 24 px là où le jeu en pose 30 —
> « le titre est très collé à la suite », retour utilisateur. Les deux candidates 13 et 14 ne se
> départageaient pas au calcul (le bord peint d'une case commence un pixel sous le rect qui la
> porte) ; rendues toutes les deux puis remesurées avec la méthode appliquée au jeu, **13 donne 30 px
> et 16 px du bas de l'encre au haut de la case, les deux cotes du jeu exactement**, quand 14 donne
> 31 et 17.

**Un seul écart sous un titre, sur tous les écrans.** Cinq d'entre eux en décidaient autrement
jusqu'au 2026-09-14 — une demi-`SECTION_GAP` dans les onglets Suivi, Alertes, Chat et Raccourcis, un
littéral de 8 dans la fenêtre de recette — et le même titre s'ouvrait sur quatre valeurs selon
l'endroit où on le lisait. Retirés sur demande de l'utilisateur : « les espacements doivent être
génériques ». `preview_trailing_gap` en change encore, mais son nom dit à quoi il sert : les
planches de simulation qui mesurent une valeur candidate, jamais un écran. Même esprit que
`Checkbox::preview_state`.

**Utilisé en production** : la modale Options (six titres sur quatre onglets) et la fenêtre de
recette.

---

## `design::stepper` — pas numérique (2026-09-10)

`crates/overlay-ui/src/design/components/stepper.rs` — composant **feuille**.

```rust
ui.add(design::stepper(&mut quantite).range(1..=999).log_name("hdv-quantite"));
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `value` (à la construction) | `&mut i64`, muté par les deux boutons | — |
| `range` | domaine autorisé ; la valeur y est **écrêtée à chaque frame**, y compris celle fournie | `i64::MIN..=i64::MAX` |
| `step` | incrément d'un clic | 1 |
| `size` | côté des deux boutons, **et donc hauteur du pas** — le socle est carré dans le jeu | `tokens::STEPPER_SIZE` (32) |
| `field_width` | largeur du champ central ; sans elle, il prend toute la place restante | — |
| `enabled` · `log_name` | comme partout | — |

**Il ne peint rien lui-même** : deux `design::icon_button` au contexte `Stepper` et un
`design::input` en lecture seule. Le socle vient du manifeste (`DsTexture::ButtonStepper`), les
glyphes aussi (`IconPlus`, `IconMinus`).

**Mesures, prises sur `large-input-number.png`** (192 × 34), la seule capture dont le socle
coïncide avec l'asset isolé `button-moins.png` :

| Grandeur | Valeur | Vérification |
| --- | --- | --- |
| Socle | `STEPPER_SIZE` (32) | y=1..32 ; deux boutons symétriques, x 1..32 et x 159..190 |
| Gouttière | `STEPPER_GUTTER_RATIO` (10/32) | 1 + 32 + 10 + 106 + 10 + 32 + 1 = 192 |
| Encre du glyphe | `STEPPER_ICON_RATIO` (12/32) | 12 × 12 mesurés dans un socle de 32 |
| Hauteur du champ | `STEPPER_FIELD_HEIGHT_RATIO` (**1,0**) | **écart assumé** — le jeu met 26 pour 32 |
| Teinte du glyphe | `STEPPER_ICON_TINT` (`#f4d89f`) | mesuré au pixel — **de l'or au repos** |

**La BOÎTE du champ fait la hauteur de ses boutons — son texte, non** (décision utilisateur,
2026-09-10). Le jeu met un champ de 26 px pour un socle de 32 : un champ plus court que ses boutons
a été jugé peu soigné, et l'uniformité l'emporte ici sur la fidélité.

Mais étirer la boîte ne doit **pas** étirer son contenu. Le champ est donc posé avec
`Input::box_height`, ajouté pour ce cas, et non avec `size(InputSize::Height(...))` qui met tout à
l'échelle — cette première version écrivait la valeur en corps 22 au lieu de 17, soit un tiers trop
gros. Vérifié après correction : **12 px d'encre, exactement comme le jeu**.

**La petite capture (`input-number.png`) n'est pas une source.** Sa gouttière concorde à un pixel
près, mais son glyphe fait 12 px dans un socle de 24 — le même que dans un socle de 32. Les deux
captures ne sont donc pas le même composant à deux échelles : le jeu règle son interface de 67 % à
233 %, et elles ont été prises à deux réglages différents. Elle sert de contrôle de cohérence, rien
de plus.

**Une erreur de mesure corrigée le jour même** : la première version divisait par la hauteur du
**fichier** (34) au lieu de celle du **socle** (32). Elle donnait une gouttière de 0,267 au lieu de
0,3125 et des boutons deux pixels trop hauts.

**Trois choses que la comparaison au jeu a corrigées** (étape 6 du skill, sans laquelle aucune ne se
serait vue) : le glyphe est **doré au repos**, alors que les deux autres contextes de bouton icône
le peignent en gris et ne passent à l'or qu'au survol ; le champ **ne garde pas sa hauteur native**
de 25 px mais suit son pas ; et le champ est en **lecture seule, pas désactivé** — sa valeur compte,
le jeu l'écrit en or, pas en gris.

**Deux écarts assumés**, faute de capture : aucun socle survolé n'existe pour cette famille (le
survol ne se signale donc que par le curseur — une teinte egui *multiplie* la texture, elle ne peut
pas l'éclaircir), et le champ n'est pas éditable au clavier (valider une saisie partielle est une
spec à part entière, sans référence pour ses états d'erreur).

**Pas encore utilisé en production.** Ses clients naturels : les boutons « + » / « − » du carré de
contrôle du Suivi, et le couple quantité du formulaire de vente HDV.

---

## `design::collapsible` — bloc repliable (2026-09-11)

`crates/overlay-ui/src/design/components/collapsible.rs` — **composant conteneur**, forme closure.

```rust
design::collapsible("Le Village", &mut ouvert)
    .icon(icone_de_quete)
    .show(ui, |ui| {
        // n'importe quoi : du texte, un formulaire, un tableau, des cases à cocher…
    });
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `title` (à la construction) | peint dans l'en-tête, serif grasse | — |
| `open` (à la construction) | `&mut bool`, basculé par le clic sur l'en-tête | — |
| `icon` | icône d'en-tête | aucune |
| `log_name` | nom d'instance au journal | le titre |
| `preview_hovered` | force l'état survolé — planches de contrôle seules | l'interaction |

**Deux états, et c'est toute sa définition fonctionnelle.** Fermé, le bloc n'est que son en-tête.
Ouvert, il montre son contenu et on peut interagir avec. **Le contenu est entièrement libre** — le
composant n'en sait rien, il lui garantit un cadre, des marges et un écrêtage.

`show` rend un `InnerResponse<Option<R>>` : `None` dit que **la closure n'a pas tourné** parce que
le bloc est fermé, ce qui n'est pas la même chose qu'un contenu vide.

**Mesures** — source unique : [`collapse-block.json`](design-system/collapse-block.json), le relevé
outillé du mainteneur sur les captures **détourées** `collapse-block-closed.png` (732 × 60) et
`collapse-block-opened.png` (732 × 210).

| Grandeur | Valeur | Vérification |
| --- | --- | --- |
| En-tête | `COLLAPSE_HEADER_HEIGHT` (60) | cadre fermé 732 × 60 — il ne contient rien d'autre |
| Marge latérale | `COLLAPSE_PAD_X` (17) | icône et intitulés de section à x=17 |
| Marge basse | `COLLAPSE_PAD_BOTTOM` (23) | `padding` du nœud `frame` |
| Marge du chevron | `COLLAPSE_CHEVRON_PAD_X` (13) | glyphe x 705..719, cadre large de 732 |
| Titre | `COLLAPSE_TITLE_FONT_SIZE` (18) | 14 px d'encre, `#fefefe`, vérifié au rendu |
| Icône | `COLLAPSE_ICON_RATIO` (32/60) | 33 × 32 dans un en-tête de 60 |
| Gouttière icône–titre | `COLLAPSE_ICON_GAP` (10) | icône finit à x=50, titre à x=60 |
| Chevron | `COLLAPSE_CHEVRON_TINT` (`#a69064`) | doré olive, **pas** `ICON_TINT` |

Le relevé qui précédait celui-ci portait sur les **mêmes captures non détourées** : l'ombre portée et
le fond de jeu y comptaient comme de l'encre, d'où un titre annoncé à 18 px au lieu de 14, une icône
à 35 au lieu de 33 et des marges de 16/20 au lieu de 17/23. `releve-collapse.json` a donc été retiré
— deux relevés qui se contredisent valent moins qu'un seul.

**Le survol éclaircit le cadre entier** (`#28292b` → `#323436`, dix niveaux sur chaque canal) quand
le pointeur est sur l'en-tête. Survoler le contenu ne l'éclaircit pas : rien n'y est cliquable.

**Quatre choses à savoir avant de le modifier :**

- **Le cadre est peint APRÈS le contenu**, dans un emplacement réservé au préalable
  (`DesignSystem::paint_to_slot`) : sa hauteur dépend de ce que le contenu a pris. C'est la raison
  d'être de la forme closure pour ce composant.
- **Le chevron est retourné, pas dupliqué** (`DesignSystem::paint_flipped_y`). Un second asset pour
  l'état ouvert serait un asset par état, aussi interdit qu'un asset par taille.
- **Le cadre survolé, lui, EST un second asset**, et c'est la seule exception du composant : le jeu
  *éclaircit* son fond, or une teinte egui multiplie — elle ne sait que foncer. Quand l'état n'est
  pas atteignable par la teinte, l'asset par état est la bonne réponse.
- **Les marges 9-slice du cadre sont asymétriques** (`COLLAPSE_BLOCK_SLICE` : 30 à gauche, 36 en
  haut, 10 à droite, 16 en bas) parce que son décor l'est : une gravure en circuit marque les deux
  angles **gauches** et rien à droite. Un découpage symétrique à 10 px, essayé d'abord, étirait cette
  gravure sur toute la hauteur du bloc.
- **`icon` est le seul paramètre-texture du design system**, et c'est assumé : cette icône appartient
  au contenu (une quête, un lieu), elle vient de la donnée. Le composant ne peut pas la résoudre
  depuis une intention.

**Deux écarts assumés** : le cadre du jeu est **translucide** et ne l'est pas ici — les huit
captures fournies sont toutes sur le même fond de jeu, il en faudrait deux sur des fonds différents
pour en déduire l'alpha, comme `tools/design-system/build_modal_body.py` le fait pour la modale —, et
le composant **ne rend pas son contenu défilable** : un contenu qui peut déborder s'enveloppe dans
une `design::scroll_area`.

**Deux des huit assets fournis ne sont pas embarqués** : `collapse-block-closed-generic.png` et son
jumeau survolé. Le 9-slice tiré de l'état ouvert rend les deux états (36 + 16 = 52 px tiennent dans
les 60 d'un bloc fermé, et les deux génériques concordent au pixel sur leurs bandes haute et basse).
Ils restent au dépôt comme référence de contrôle.

**Pas encore utilisé en production.**

---

## `design::loader` — rouage de chargement (2026-09-11)

`crates/overlay-ui/src/design/components/loader.rs`

```rust
use overlay_ui::design::{self, LoaderSize};

ui.add(design::loader());                                   // 124 px, taille native (1×)
ui.add(design::loader().size(LoaderSize::Small));           // 48 px, le plancher
ui.add(design::loader().size(LoaderSize::Px(80.0)).tooltip("Synchronisation…"));
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `size` | `Small` (48) / `Medium` (72) / `Large` (96) / `Native` (124) / `Px(f32)` ramené dans `[48, 124]` | `Native` |
| `tooltip` | texte d'infobulle | aucune |
| `log_name` | nom d'instance pour le journal | `loader` |
| `preview_frame` | image de la boucle (`0..16`) — **galerie et captures uniquement** | horloge egui |

**Toujours carré, toujours animé, aucun état** : un indicateur de chargement ne se clique pas et ne
se désactive pas. `LoaderSize::px()` rend le côté effectivement peint, pour un panneau qui réserve
la place avant d'ajouter le composant.

**L'animation ne coûte que 24 réveils par seconde** : le composant demande à egui un
rafraîchissement au prochain changement d'image (`request_repaint_after`), pas un rendu continu.
Hors écran (galerie), `preview_frame` fige l'image et aucun réveil n'est demandé.

**Origine** : enregistrement du client du 2026-09-11 (`Enregistrement 2026-09-11 142031.mp4`,
158 × 156 px, 30 i/s, sept écrans de chargement), isolé image par image — fond retiré par
estimation du fond sur toute la plage, blanc conservé avec alpha 8 bits.

**Mesures** :

| Grandeur | Valeur | Origine |
| --- | --- | --- |
| Image | 124 × 124 px | 118 px d'encre (rayon 58,5) + 3 px de marge transparente |
| Dents | 16 | profil angulaire au rayon des dents |
| Rotation | 2,8° par image, une dent toutes les 8 images | corrélation angulaire image à image |
| Boucle | **16 images** | image 386 ≡ 409, pas 399 : la tête a une période double de celle des dents |
| Cadence | 24 i/s | 4 images nouvelles sur 5 enregistrées |

**Choisi, pas mesuré** : le plancher de 48 px est une décision utilisateur (en dessous, les seize
dents se confondent) ; `Medium` et `Large` découpent l'intervalle en marches d'environ 25 px. Une
taille hors de l'intervalle est ramenée à la borne, avec un `warn!` unique par instance.

**Textures** : `loader-sheet.png` (496 × 496, grille 4 × 4 lue ligne par ligne), **une planche et
non seize textures** — peinte par `DesignSystem::paint_region`, le seul chemin du manifeste qui
lit une région par ses UV. Le 9-slice déclaré (`LOADER_SLICE` = `ICON_SLICE`) ne sert pas. La
même boucle existe en `loader.apng` (alpha 8 bits, 24 i/s) et `loader.gif` (transparence 1 bit,
25 i/s faute de granularité GIF) pour tout ce qui n'est pas egui.

**Pas de comparaison au jeu à la même taille** : le rouage n'y existe qu'à 1×, et c'est précisément
la capture dont il est tiré.

---

## `design::separator` — filet de séparation (2026-09-11)

`crates/overlay-ui/src/design/components/separator.rs`

```rust
use overlay_ui::design;

ui.label("Description");          // l'intitulé de section
ui.add_space(10.0);               // la cote du dessus appartient à l'appelant
ui.add(design::separator());      // le filet, qui réserve 12 px sous lui
ui.label("le corps de la section");
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `width` | largeur imposée | toute la largeur disponible |
| `on_hovered_surface` | `bool` — la **surface porteuse** est survolée | `false` |
| `trailing_gap` | écart réservé sous le filet | `SEPARATOR_GAP_BELOW` = 12 px |
| `log_name` | nom d'instance pour le journal | `"separator"` |

Aucune texture : quatre lignes de pixels sur deux teintes plates, un PNG n'aurait rien à porter.

### Les mesures

Relevées au pixel sur `collapse-block-opened.png` et `collapse-block-opened-hover.png` — les
captures **source**, pas les génériques, qui ont justement été nettoyées de leur contenu, filet
compris. Le filet y apparaît deux fois (y=82..85 et y=152..155), identique dans les deux occurrences
et dans les deux textures.

| | fond | ligne claire (2 px) | ligne sombre (2 px) |
| --- | --- | --- | --- |
| repos | `#28292b` | `#323335` (+10) | `#222325` (−6) |
| survol | `#323436` | `#3c3e3f` (+10) | `#2b2c2e` (−7) |

### Quatre choses à savoir

1. **C'est un bevel, pas un trait.** Deux lignes plates empilées, sans dégradé : les quatre lignes
   de pixels ne portent que deux teintes, sans valeur intermédiaire. Rendu en un `hline` gris, le
   filet paraît posé *sur* le fond au lieu d'y être creusé.

2. **Le bevel suit son fond**, d'où `on_hovered_surface`. Le jeu recalcule les deux lignes quand la
   surface s'éclaircit. Sans ce paramètre, la ligne claire du repos (`#323335`) se retrouve à **un
   niveau** du fond survolé (`#323436`) et disparaît — la galerie montre les deux cas côte à côte
   pour que l'écart se voie plutôt que d'être affirmé ici.

3. **Le paramètre ne s'appelle pas `hovered`** parce que le filet ne se survole pas lui-même : il
   n'est pas cliquable, et `response.hovered()` sur ses 4 px ne renseignerait sur rien. Ce qu'on lui
   dit, c'est l'état de la surface qui le porte.

4. **Il ne pose aucune marge latérale.** Dans la capture le filet court de x=16 à x=719 sur un cadre
   de 732 — 16 à gauche, 12 à droite. Cette asymétrie est celle du **bloc**, pas du filet : le
   chevron de l'en-tête s'arrête au même x=719. Le composant remplit la largeur qu'on lui donne
   (§6 du contrat) ; dans un `design::collapsible` cela donne les 17 px de `COLLAPSE_PAD_X` des deux
   côtés.

### Deux corrections au relevé

`docs/design-system/collapse-block.json` est en écart avec la texture sur deux points, tranchés en
faveur de la mesure :

- la note de `sep-1` donne la ligne claire à `#323436` — la texture donne `#323335`. `#323436` est
  la teinte du *fond survolé*, vraisemblablement relevée à sa place.
- la palette déclare `border-bevel-dark: #28292b`, qui est exactement le fond au repos : une ligne
  sombre de cette teinte serait invisible. La texture donne `#222325`, valeur que la note de `sep-1`
  portait déjà. C'est la palette qui est fautive, pas la note.

Comparaison au jeu : les huit lignes de pixels (quatre par état) sont **identiques au rendu**, sans
dérive de gamma.

---

## `design::slider` — curseur de réglage (2026-09-11)

`crates/overlay-ui/src/design/components/slider.rs`

```rust
use overlay_ui::design;

if ui.add(design::slider(&mut state.volume)).changed() {
    audio.set_volume(state.volume);
}

// Le motif du jeu : un libellé de chaque côté, à la gouttière relevée.
ui.horizontal(|ui| {
    ui.label("Min");
    ui.add_space(design::tokens::SLIDER_LABEL_GAP);
    ui.add(design::slider(&mut state.volume).width(200.0));
    ui.add_space(design::tokens::SLIDER_LABEL_GAP);
    ui.label("Max");
});
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `range` | `RangeInclusive<f32>` | `0.0..=1.0` |
| `steps` | nombre de valeurs sélectionnables — **grade le curseur** | aucun, curseur continu |
| `width` | largeur imposée | toute la largeur disponible |
| `enabled` | `bool` | `true` |
| `tooltip` / `log_name` | | aucune / `"slider"` |
| `preview_state` | `Idle` / `Hovered` / `Disabled` — **galerie uniquement** | état réel |
| `preview_fraction` | position forcée, 0..1 — **galerie uniquement** | la valeur |

Textures : `DsTexture::SliderHandle` (`slider-handle.png`, 18 × 18). La rainure, elle, est peinte.

### Les mesures

Relevées au pixel sur les deux curseurs de volume de `interfaces/interface-options-son.png`
(« Musique », rainure y 335..342 ; « Sons - Ambiance », y 430..437), qui concordent sur toutes les
cotes.

| Grandeur | Valeur |
| --- | --- |
| Rainure | 8 px de haut, 2 de liseré en haut et en bas |
| Liseré | assombrit le fond dans le rapport **0,735** |
| Intérieur | rapport **0,853** |
| Poignée | 18 × 18, cercle (`radius_fit_iou` 1,0) |
| Poignée — teintes | `#c6b187` clair, `#635944` sillon, `#817358` ombre bas-droite |
| Largeur relevée | 200 px (x 75..274) — un ordre de grandeur, pas un gabarit |
| Gouttière du libellé | 12 px à gauche, 10 à droite |

### Les graduations

Un curseur gradué annonce **où la poignée peut s'immobiliser** ; sans `steps`, il est continu et nu.
La distinction est mesurée, pas choisie : le curseur d'échelle d'interface
(`interfaces/interface-options-interface.png`) porte **26 graduations**, celui du volume **aucune**
— pas un pixel clair le long de sa rainure.

Et elles tombent bien sur les arrêts de la poignée : les trois libellés de la capture sont centrés
à x = 114, 199,5 et 539,5, pour des graduations à 111, 196 et 536 — la première, la sixième et la
vingt-sixième. Le disque, lui, est exactement sur la sixième.

| Grandeur | Valeur |
| --- | --- |
| Largeur | 1 px (26 graduations sur 26) |
| Couleur | `#e2ddd7` |
| Débord au-delà de la rainure | 2 px, en haut comme en bas |
| Morsure sur le liseré | **1 px** sur les 2 du liseré |
| Segment visible | 3 px de chaque côté, 6 px de rainure nue entre les deux |

**Une graduation ne traverse pas la rainure**, elle la coupe : entre les deux segments, les pixels
d'une colonne graduée sont identiques à ceux d'une colonne nue.

**Un écart assumé avec la capture** : dans le jeu les graduations s'arrêtent à 37 px des bords de la
rainure (x 111..536 pour une rainure de 74 à 573), soit 28 px de plus que le rayon de la poignée. Le
composant ne reproduit pas ce retrait — une seule capture d'un curseur gradué ne dit pas si ces
37 px sont absolus ou proportionnels, et les extrapoler ferait 74 px de retrait sur une rainure de
120. Trancher demande une seconde capture, à une autre largeur.

### Quatre choses à savoir

1. **La rainure est un creux, pas une barre.** Elle n'a pas de couleur propre : elle assombrit le
   fond. C'est une mesure et non un choix de rendu — sur deux fonds qui diffèrent de deux niveaux,
   le rapport au fond reste le même sur les trois canaux. Deux constantes opaques auraient
   reproduit la capture et rien d'autre.

2. **La portion parcourue n'est pas remplie.** Les deux curseurs du jeu sont au minimum : rien ne
   montre ce que devient la rainure derrière la poignée. Peindre un remplissage doré serait
   inventer du design — ce que `design::input` refuse déjà pour son état survolé.

3. **La poignée est un asset alors que ses teintes sont plates**, parce que son relief est
   *directionnel* : liseré clair en haut à gauche, mi-ton en bas à droite, sillon sombre entre les
   deux. Le peindre demanderait deux arcs partiels pour dix-huit pixels.

4. **Les libellés d'extrémité n'appartiennent pas au composant.** « Min »/« Max » conviennent à un
   volume, pas à une échelle d'interface qui dirait « 50 % »/« 200 % ». Le jeton donne la gouttière,
   l'appelant pose les mots.

### Ce que la comparaison au jeu a rattrapé — trois fois

1. **Le creux à l'envers.** La première version peignait le liseré sur toute la hauteur puis
   l'intérieur par-dessus. Les alphas se composent : 0,733 × 0,851 = **0,624**, et l'intérieur
   sortait plus sombre que son propre liseré. Trois bandes disjointes ramènent les rapports à 0,75
   et 0,83, contre 0,74 et 0,85 dans le jeu.

2. **La graduation trop longue.** Elle mordait les 2 px du liseré au lieu d'un seul : 4 px de
   segment et 4 px de rainure nue, au lieu de 3 et 6. Assez pour que le repère se lise comme un
   trait presque continu.

3. **Le demi-pixel.** Le `Ui` appelant peut allouer à un y non entier, et un creux de 8 px posé à
   y,5 s'étale sur 9 lignes — les graduations repassaient à 4 px. La rainure est désormais calée sur
   la grille (`.round()`).

Après les trois, le profil vertical d'une graduation est **identique au jeu**, ligne par ligne
(`TTT......TTT..`), et la poignée l'est au pixel. Aucune relecture de code n'aurait vu ces trois
défauts.

---

## `design::tooltip` — infobulle (2026-09-11)

`crates/overlay-ui/src/design/components/tooltip.rs`

```rust
use overlay_ui::design::{self, TooltipSide};

design::tooltip(&response).text("Ajouter à la liste");            // au-dessus, le défaut
design::tooltip(&response).side(TooltipSide::Right).text("Options");
design::tooltip(&response).side(TooltipSide::Below).anchor(carre).text("Ajouter"); // sous le GROUPE
design::tooltip(&response).show(|ui| { /* contenu libre */ });
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `side` | `Above` / `Left` / `Right` / `Below` | `Above` |
| `gap` | écart au widget | `TOOLTIP_GAP` = 5 px |
| `anchor` | rectangle de placement | celui du widget |
| `text` / `show` | texte simple / contenu libre | — |

Aucune texture : le fond, la marge et l'ombre viennent du **thème** (`style::apply`), parce que
`window_fill`/`menu_margin`/`popup_shadow` ne servent qu'aux infobulles dans cette interface. Un seul
réglage couvre donc aussi les `on_hover_text` ponctuels.

### Ce qu'il absorbe

Les **trois enveloppes maison** — `combat::show_tooltip_above`, `watchlist::show_tooltip_left` et
`_right` — rigoureusement identiques à un alignement près, chacune avec sa liste de replis recopiée.
`panels/tooltip.rs` disparaît avec elles ; ses quatre constantes mesurées sont remontées dans
`design::tokens`.

### Les replis, qui sont le composant

Un alignement qui ne tient pas ne doit pas retomber n'importe où. L'ordre est le résultat de trois
bugs rapportés, pas une préférence :

1. le côté demandé, centré ;
2. **le même côté, réaligné** (`_START`, `_END`) — ajoutés avant les replis opposés le 2026-09-06 :
   un widget proche d'un bord reste du bon côté, simplement décalé ;
3. le côté opposé, en dernier recours — et jamais `BOTTOM_START`, le défaut d'egui (« sous la
   souris »), qui est précisément ce qu'on fuit.

Le cas qui a fait ajouter l'étape 2 : le bouton Détails du panneau Combat, collé au bord droit,
débordait en `TOP` centré et retombait sous le curseur ; son voisin Options, un peu plus loin du
bord, ne révélait jamais le problème.

**Un repli ne crée pas de la place.** Le switch Alliés/Ennemis, premier widget du panneau Combat,
n'avait réellement aucune place au-dessus de lui : la réponse est `render_content::
COMBAT_TOP_MARGIN`, pas un alignement.

**Le côté se choisit par colonne, pas par bouton** (retour du 2026-09-08) : dans le carré de
contrôle du Suivi, un bouton de droite dont l'infobulle partirait à gauche la poserait par-dessus
son voisin, gênant le survol de ce dernier.

### `anchor` — quand le côté ne suffit plus (2026-09-13)

Le Suivi ouvre les quatre infobulles de son carré **en dessous** depuis que la place réservée
au-dessus éloignait trop la bande du haut du jeu (§9.1 octies du plan). Mais « en dessous de + »,
c'est « par-dessus Détails » : le côté ne protège plus rien dès que le groupe a deux lignes.
`anchor` accroche donc l'infobulle à un autre rectangle que celui du widget — ici le carré entier,
et elle s'ouvre sous sa dernière ligne. Le survol reste celui du widget ; seul le placement change.
Même usage pour une tuile de la bande, ancrée jusqu'au bas de la zone défilante pour ne pas masquer
la barre peinte juste dessous.

**Il ne change qu'`interact_rect`**, le rectangle sur lequel egui ancre le popup (« we use
interact_rect so we don't show the popup relative to some clipped point »). Élargir aussi `rect`
paraît symétrique et casse tout : egui garde une infobulle ouverte tant que le pointeur est dans le
`rect` de son widget — « le cas d'une grosse infobulle qui recouvre le widget » —, et n'en autorise
qu'une par couche. Avec un `rect` élargi au groupe, l'infobulle du premier bouton survolé colle, et
les trois autres n'ouvrent plus jamais la leur. Constaté en capture avant d'être compris.

### Un écart au contrat, assumé et daté

**La police reste celle du thème**, là où le contrat veut `design::text::label_font`. La corriger est
un changement visuel : elle déplacerait les six captures d'infobulle de la suite de rendu, que le
plan demande justement **inchangées** pour prouver que cette migration ne change rien. Les deux ne
peuvent pas tenir dans le même commit — la police attend sa propre décision.

**Pas d'entrée de galerie** : une infobulle a besoin d'un survol, qui n'existe pas en rendu
offscreen statique. Sa vérification visuelle est ailleurs et existait déjà — les tests dédiés
`watchlist_tooltip_*` et `combat_tooltip_*`, qui simulent le pointeur. Ils sont restés **identiques
au pixel** à travers la migration, ce qui est le critère de fin que le plan fixait.

Ils demandent en revanche d'être **lus**, pas seulement exécutés : les quatre survols du bandeau
vide visaient 70 px trop à droite depuis le retrait de la réserve gauche, et deux captures
montraient l'infobulle d'« Options » pendant que deux autres n'en montraient aucune — sans qu'un
seul test échoue. Un nom de fichier n'est pas une assertion.

---

## `design::icon` et le registre `DsIcon` (2026-09-11)

`crates/overlay-ui/src/design/icons.rs`, `components/icon.rs`

```rust
use overlay_ui::design::{self, DsIcon};

ui.add(design::icon(DsIcon::Kamas));                  // 16 px par défaut
ui.add(design::icon(DsIcon::Lock).size(24.0).tint(tokens::TEXT_DISABLED));
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `size` | côté du **carré englobant** | `ICON_SIZE` = 16 px |
| `tint` | teinte | `ICON_TINT` |

### Deux registres, et pourquoi

`DsTexture` et `DsIcon` étaient une seule énumération, ce qui obligeait chacun à porter les
propriétés de l'autre :

- **Un glyphe n'a pas de 9-slice.** Les 38 icônes déclaraient toutes `ICON_SLICE`, un 9-slice
  dégénéré présent parce que le champ était obligatoire. Il ne décrivait rien et masquait ce que la
  texture *est*.
- **Un fond n'a pas d'étalon d'encre.** `icon_content_size` énumérait à la main, dans un `match` de
  vingt lignes, les variantes qui sont des icônes normalisées — liste tenue en parallèle de
  l'énumération, que rien ne vérifiait.

Le défaut que cette confusion a produit est daté : le test `les_glyphes_d_icone_sont_detoures_au_
pixel_pres` sélectionnait ses cibles **par leur découpage** (`insets == 0`), faute de mieux. Il
ratait sa cible dans les deux sens — laissant passer les glyphes sans socle, et attrapant
`loader-sheet.png`, une planche d'atlas qu'il a fallu exempter en ajoutant `DsTexture::grid`.
Aujourd'hui il balaie `DsIcon::ALL`, sans filtre ni exemption.

### La table est la source unique

Nom de cache egui, chemin du fichier et présence d'un étalon viennent d'**un seul littéral** par
icône, assemblés par la macro `ds_icons!`. Il n'y a plus de façon d'écrire `ds-icon-eye` en face de
`icon-eye-off.png` — c'était possible tant que les trois étaient recopiés à la main dans `spec()`.

Deux catégories d'étalon, marquées dans la table :

- **`socle`** (21 icônes) — détourée depuis un socle de bouton du jeu, taille d'encre connue et
  comparable, donc normalisable sur `ICON_BUTTON_CONTENT` ;
- **`libre`** (17) — détourée sans bouton porteur ou sans mesure consignée. La normaliser sur un
  étalon qu'elle ne partage pas la rendrait fausse ; elle garde sa taille native.

### Ce que `design::icon` fait, et ce que fait `icon_button`

`icon_button` peint un glyphe **sur un socle**, avec ses états et son clic. `design::icon` ne peint
que le glyphe : une icône dans une ligne, un en-tête de colonne, à côté d'un compteur. Il n'est pas
cliquable — qui veut un clic prend `icon_button`, qui a le socle que ce clic mérite.

**Le rapport d'aspect est préservé** : les glyphes du jeu ne sont pas carrés (chevron 14 × 8,
pastille d'info 27 × 28). Le composant réutilise `glyph_fit`, le seul endroit du crate qui calcule
ce rapport, plutôt qu'un `Vec2::splat` — exactement le défaut qu'`Input::leading_icon` portait
jusqu'au 2026-09-10. `size` donne donc le côté du **carré englobant**, pas la largeur : un chevron
demandé à 16 px sera peint 16 × 9, centré dans un carré de 16.

**L'étalon ne s'applique pas hors socle** : `content_size` sert à accorder deux glyphes voisins sur
deux boutons. Sans voisin, le glyphe occupe le carré qu'on lui donne.

### Vérification

Le refactor touche 223 usages et **aucun snapshot n'a bougé**, hormis celui de la galerie où une
section a été *ajoutée*. Les six captures d'infobulle, les panneaux, la modale : identiques au
pixel. C'est ce qu'on attend d'un changement de typage.

Coût mémoire, mesuré avant décision : les 38 icônes décodées en RGBA pèsent **31 Ko** — sans effet
sur le budget de 300 Mo (§8 du plan).

## `design::autocomplete` — champ d'autocomplétion (2026-09-11)

`crates/overlay-ui/src/design/components/autocomplete.rs`

```rust
use overlay_ui::design::{self, AutocompleteEntry, AutocompleteFilter};

let issue = design::autocomplete(&mut state.saisie)
    .placeholder("Ajouter un objet à surveiller…")
    .width(560.0)
    .filters(&filtres)   // « Tout » en tête, puis les catégories PRÉSENTES
    .entries(&entrees)
    .log_name("alertes.ajout")
    .show(ui);
if let Some(index) = issue.selected {
    ajouter(&entrees[index]);
}
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `entries` | `&[AutocompleteEntry]` — libellé, catégorie, gemme, image, désactivé, mention | vide |
| `filters` | `&[AutocompleteFilter]` — `all(…)` et `category(id, …)` | vide (pas de bande) |
| `placeholder` / `empty_filter_label` | textes | `""` / « Aucun résultat dans cette catégorie » |
| `width` | largeur imposée | largeur disponible |
| `min_query_len` | seuil de déclenchement | `AUTOCOMPLETE_MIN_QUERY_LEN` = 3 |
| `max_visible_rows` | au-delà, la liste défile | `AUTOCOMPLETE_MAX_VISIBLE_ROWS` = 5 |
| `enabled` | `bool` | `true` |
| `search_icon` | la loupe en tête de champ | `true` |
| `fill_on_select` | la sélection **remplit** le champ au lieu de le vider | `false` |
| `preview_open` / `preview_active` / `preview_filter` | aperçu de galerie | fermé / 0 / aucun |

Rend un **`AutocompleteOutcome`** : la `Response` du champ **et** `selected: Option<usize>`, l'indice
dans `entries` de l'entrée choisie.

### Deux usages, deux réglages (2026-09-16)

Le composant sert deux choses qui n'ont ni le même décor ni la même fin de geste :

| | Champ d'**ajout** (Suivi, Alertes) | **Saisie assistée** (nom d'un personnage) |
| --- | --- | --- |
| Décor | loupe — c'est une recherche | `search_icon(false)` : le nom s'écrit, il ne se cherche pas |
| Après une sélection | le champ se vide, il a fini | `fill_on_select(true)` : le libellé reste dans le champ, qui garde son focus |
| Ce qui compte | `selected`, l'entrée passée à la liste | `query`, la valeur du formulaire |

**Dans les deux cas le texte libre est accepté** : le composant ne valide rien, n'efface jamais ce
qui ne correspond à aucune entrée, et `selected` vaut simplement `None`. Les suggestions sont une
aide à la saisie, pas une liste fermée — pour une liste fermée, c'est `design::select`. Deux champs
à loupe sur un même écran, dont un qui n'en est pas une, ne se distinguent plus que par leur invite
(retour utilisateur du 2026-09-16, modale « Personnage »).

Planche dédiée : `design_gallery_autocomplete_saisie.png` (`galerie_de_la_saisie_assistee`) — la
grande galerie est pleine, son contenu dépasse déjà le plafond de 8192 px de wgpu et sa section
« Autocomplétion » y est tronquée en bas.

Décor résolu en interne (socle et loupe d'`InputSize::Standard`) ; **panneau déplié entièrement
repris de `design::select`** — fond, bord, filet de tête, surbrillance, cadence de rangée de 28 px.
C'est la même liste du jeu, il n'y avait pas de second relevé à faire. Ce que ce composant ajoute :
la bande de filtres, une image par entrée, des entrées désactivées, et un seuil de déclenchement.

### Les six règles de comportement — cinq portées du web, une propre à l'overlay

Le jeu n'a **pas** d'autocomplétion : aucune capture ne peut servir de référence, donc rien de tout
ceci ne se vérifie à l'œil. Relevé sur `shared/wakfu-autocomplete` (`Oumbra/wakfu-companion`) le
2026-09-11.

1. **Rien avant trois caractères** — comptés en *caractères*, pas en octets.
2. **Une entrée désactivée n'est pas sélectionnable** : grisée, sans surbrillance au survol, sautée
   par le clavier, et refusée par `show` même si un clic l'atteignait. Trois barrières, parce
   qu'une seule finit toujours par être contournée.
3. **Un filtre actif restreint la liste** à sa seule catégorie.
4. **La bande se calcule sur la liste NON filtrée** — l'appelant construit `filters` sans tenir
   compte du filtre actif. Sinon le bouton qui permettrait de relâcher un filtre sans résultat
   disparaîtrait avec les rangées, et l'utilisateur resterait coincé devant une liste vide.
5. **Après une sélection** : le champ se vide, le panneau se ferme, et l'entrée active repart à la
   première. Le filtre, lui, **reste** — voir la règle suivante.
6. **Le filtre de catégorie vaut pour toute la session** — le seul ÉCART au web, demandé le
   2026-09-15 (« enregistrer, pour la session, les filtres de catégorie sélectionnés »). Qui filtre
   sur « Ressources » pour ajouter une alerte va en ajouter plusieurs : remettre le filtre à
   « Tout » après chaque choix lui redemandait le même clic à chaque objet. Le filtre est mémorisé
   sous une clé tirée de `log_name` — jamais sous l'id du champ, qui est un identifiant
   **automatique** d'egui : un widget de plus au-dessus du champ le décale, et la mémoire tomberait
   avec lui. La mémoire vit dans le `Context` d'egui, donc jusqu'à la fermeture de l'overlay et
   nulle part sur le disque. Ce que la remise à zéro du web évitait — un filtre oublié qui vide une
   recherche sans rapport — est écarté autrement : **un filtre dont la catégorie a quitté la bande
   se relâche de lui-même**, et la bande dit exactement quelles catégories les résultats courants
   portent (règle 4).

**Cliquer un bouton de filtre rend le focus au champ** (2026-09-15). Le bouton est dans le panneau,
pas dans le champ : l'appui le défocalisait, et le panneau ne tenait plus que par le pointeur posé
dessus — il se refermait dès que la souris s'en écartait, et la frappe suivante tombait dans le
vide. Même rattrapage que pour `Entrée` sur une entrée désactivée. Mesuré sur
`crates/overlay-testkit/tests/autocomplete_filtre.rs`, dont la capture d'après-clic ne montrait
jusque-là qu'un champ nu.

Clavier : `↓`/`↑` sautent les entrées désactivées et bouclent, `Entrée` valide, `Échap` ferme. Les
touches sont consommées **avant** le champ de saisie, sinon la flèche déplacerait le curseur de
texte. Une liste entièrement désactivée termine quand même — la recherche s'arrête après un tour
complet (test `toutes_desactivees_ne_boucle_pas_indefiniment`).

### Deux écarts au contrat, assumés

**`show` plutôt que `impl Widget`** (§1) : une `Response` ne peut pas dire *quelle* entrée a été
choisie, et la ressortir par un `&mut` en paramètre est la maladresse que §6 reproche ailleurs.
Même raison que pour les conteneurs (§1 bis), sur un composant qui n'en est pas un.

**Des textures en paramètre** (§1) : la gemme de rareté et l'image d'un objet sont du **contenu**,
pas du décor — elles viennent du CDN `wakassets` par `RemoteIconStore`, le design system ne les
possède pas. Elles arrivent donc par `AutocompleteEntry`, au même titre que le libellé. Leur absence
n'est pas une erreur : la colonne reste réservée, les libellés restent alignés, et la rangée se
peint sans elles (c'est l'état normal tant que le CDN n'a pas répondu).

### Ce que le composant ne fait pas

Il ne cherche rien. L'appelant lui passe des entrées déjà trouvées, déjà triées, déjà marquées
« déjà suivi ». Le domaine n'est **pas** un paramètre : la page Alertes ne lui donne que des objets,
le futur formulaire d'ajout au Suivi lui donnera objets **et** monstres — le composant ne fait pas
la différence. Les objets à recette (évolution demandée pour ce même formulaire) suivront de la même
façon, par l'appelant.

### Ce que la capture a rattrapé

- **La quatrième rangée sortait du panneau.** La hauteur du panneau vaut `rangées × 28`, sans
  interligne — mais `allocate_exact_size` ajoutait les 3 px d'`item_spacing` hérités du thème entre
  chaque rangée. Neuf pixels de trop sur quatre rangées : le libellé de la dernière était coupé en
  deux par le bord. Invisible à la relecture, évident sur la planche.
- **La `ScrollArea` n'est instanciée que si la liste déborde vraiment** : toujours présente, elle
  demande un repeint tant que son décalage s'anime, et `Harness::run` tourne alors jusqu'à sa limite
  d'étapes sans converger.
- **Les identifiants dérivent de la `Response` du champ**, jamais du `Ui` parent : trois instances
  dans le même parent partageaient sinon le même id d'`Area` et de rangées — egui l'écrit en rouge
  par-dessus le rendu.

### Ce qu'aucune capture ne montrait — le temps (2026-09-12)

Second retour du même jour, captures web et overlay côte à côte pour « bouftou » : « extrêmement
long », et « pas tous les résultats ». Trois causes, aucune dans ce composant, toutes mesurées
plutôt que supposées :

1. **La fenêtre ne se redessinait qu'une fois toutes les deux secondes** pendant la frappe. Journal
   instrumenté, frappe pilotée par `SendInput` : chaque touche arrivait dans `window_event`,
   `request_redraw()` était appelé, et aucun `RedrawRequested` ne suivait — la frame suivante venait
   de la réaffirmation topmost périodique (`SetWindowPos`, 2 s), qui fait repeindre la fenêtre par
   Windows. Cinq retours arrière et trois lettres s'appliquaient d'un coup, 1,3 s plus tard. Le
   `WM_PAINT` que `RedrawWindow(RDW_INTERNALPAINT)` est censé poster n'arrive pas de façon fiable
   sur ces fenêtres DirectComposition sans surface de redirection. Corrigé dans l'hôte, pas ici :
   `App::redraw` rend la frame lui-même depuis `about_to_wait`, qui suit chaque livraison
   d'événements — une touche produit sa frame en moins de dix millisecondes (mesuré, même méthode).
   Voir §6.2 du plan.
2. **La recherche était coupée à quarante** (`alerts_tab::MAX_SUGGESTIONS`), et la bande de filtres,
   calculée sur la liste rendue, perdait des catégories présentes : cinq boutons ici contre huit sur
   le site. La recherche coûte moins d'une demi-milliseconde pour 115 résultats sur 16 302 objets
   (mesuré sur le catalogue réel, `overlay-engine/examples/bench-search.rs`) — la limite protégeait
   un coût qui n'existe pas. L'appelant passe `usize::MAX` ; le panneau défile, comme le web.
3. **Les icônes arrivaient une par une, en série, dans l'ordre d'arrivée** : un agent HTTP neuf par
   requête (une poignée de main TLS par icône, ~145 ms chacune), un seul thread, et les icônes des
   préfixes abandonnés (« bou », « bouf ») servies avant celles de la requête courante. Corrigé dans
   `remote_icons` : un agent partagé, quatre threads, et un **tas ordonné par frame egui** — ce que
   la dernière frame a demandé part en premier, dans son ordre de demande. Une pile LIFO simple,
   essayée d'abord, renversait aussi l'ordre à l'intérieur d'une frame et servait la centième
   rangée avant la première. Corollaire pour l'appelant : `alerts_tab::add_field` demande les huit
   icônes de la bande de filtres **avant** les images des rangées — demandées après, elles
   arrivaient les dernières et la bande restait vide (constaté sur « tofu », 118 résultats).
   Mesuré après correctif : 109 icônes à froid en 2,5 s, les cinq rangées visibles illustrées en
   moins de 500 ms.

Le composant, lui, n'a changé que de champ : `InputSize::Search` et `clearable(true)`, voir
`design::input`.

### La barre du panneau (2026-09-12, soir)

Troisième retour du jour : « un tout petit peu plus large, à l'image du web ; l'élargissement au
survol ne me paraît pas si mal, mais conserver les couleurs de base ». La barre était celle
d'egui par défaut — mince, invisible au repos, plus large ET plus claire sous le pointeur. Elle
est désormais posée dans le scope du panneau, avec ses propres jetons (le jeu n'a pas
d'autocomplétion, c'est le web qui fait référence, comme pour tout ce composant) :

| Grandeur | Valeur | Origine |
| --- | --- | --- |
| Largeur au repos | 8px (`AUTOCOMPLETE_SCROLLBAR_WIDTH`) | `::-webkit-scrollbar { width: 8px }`, `styles.css` du web |
| Largeur sous le pointeur | 10px | **choix** — le web ne s'élargit pas, l'utilisateur a validé l'effet. *Retiré le soir même, voir la section suivante.* |
| Rayon | 4 | `border-radius: 4px` du web |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) | le gris que l'utilisateur voyait au repos (`gray(180)` d'egui), à deux valeurs près le gris unique du jeu — **pas** `SCROLLBAR_THUMB`, mesuré sur le fond noir de la fenêtre Options et invisible sur le brun de la liste (vérifié au pixel) |
| Poignée minimale | 24px | **choix** — à 115 résultats, les 12 px d'egui donnaient un point |
| Rail | aucun | comme dans le jeu. *Ajouté le soir même, voir la section suivante.* |

La colonne de la barre est réservée (`floating_allocated_width`) : la mention « déjà dans vos
alertes » ne passe jamais dessous. Galerie : « Au-delà de cinq rangées — la liste défile ».

Même soir, hors composant : **l'emplacement d'objet des tuiles d'alerte passe de 44 à 64 px**,
la case du Suivi (`ITEM_SLOT_SIZE`), « dix pixels de plus de chaque côté » — la tuile passe de 88
à 110 px de haut pour le loger, le nom garde sa ligne unique (`panels::alerts_tab`).

### La rangée du web, cote pour cote (2026-09-12, nuit)

Quatrième retour du jour, et le plus dense : « les images sont beaucoup plus collées que sur le
web, ça manque de ce côté aéré » ; « les gemmes ont été très fortement agrandies et aplaties, sur
le web elles sont en 14 × 14 » ; « une ligne fait 35 px sur le web, quelle est la dimension ici ? » ;
le rail « plus sombre que le fond », la barre « sans s'agrandir », sa poignée « de la couleur des
éléments survolés » ; le pointeur en main sur les rangées ; et un bug — « le scroll ne se
synchronise pas avec les flèches ».

**La gemme d'abord, parce que c'était un bug et non un choix.** Le composant peint la gemme à son
rapport natif depuis le premier jour (`glyph_fit(entry.gem_size, 14)`), et la galerie lui passait
bien `GEM_NATIVE` (13 × 20). Mais l'onglet Alertes, lui, ne posait que `entry.gem` et laissait
`gem_size` à sa valeur par défaut, `Vec2::splat(1.0)` — un carré. Une gemme de 13 × 20 se
retrouvait donc étirée en 14 × 14 : plus large d'un pixel, écrasée de six. `alerts_tab::texture`
rend désormais la taille du `TextureHandle` avec son id, et la gemme entre dans sa boîte en 9 × 14,
comme `object-fit: contain` côté web.

**La rangée ensuite.** Elle suivait la cadence du select du jeu (`SELECT_ROW_HEIGHT`, 28 px) avec
des écarts de 6 — un choix du 2026-09-11, quand le composant se présentait comme « une extension
de `select` ». Sauf que la liste du jeu n'a ni gemme ni image à loger. Réponse à la question posée :
la rangée faisait **28 px** contre **35** sur le web, avec un corps de nom de 15 px contre 13,1
(0,82 rem). Un rapport unique (15 / 13,1 ≈ 1,14, soit une rangée de 40) aurait grossi la gemme à
16 alors qu'elle est demandée en 14 : les cotes du web sont donc portées **telles quelles**, le
corps de 15 restant celui de l'overlay. Relevé sur `wakfu-autocomplete.component.css` :

| Cote | Web | Avant | Jeton |
| --- | --- | --- | --- |
| Hauteur de rangée | `height: 35px` | 28 | `AUTOCOMPLETE_ROW_HEIGHT` |
| Marge gauche et droite | `padding: 0 10px` | 6 | `AUTOCOMPLETE_ROW_PADDING_X` |
| Boîte de la gemme | 14 × 14 à `left: 10px` | 14 (écrasée) | `AUTOCOMPLETE_GEM_BOX` |
| Colonne d'image | `width: 30px; margin-left: 20px` | — | `AUTOCOMPLETE_IMAGE_COLUMN`, `_OFFSET` |
| Image | `[size]="24"` | 22 | `AUTOCOMPLETE_IMAGE_SIZE` |
| Écart colonne → nom | `gap: 10px` | 6 | `AUTOCOMPLETE_ROW_GAP` |

Ce qui donne, de gauche à droite : marge 10, gemme 10..24, colonne d'image 30..60 (image de 24
centrée), nom à 70. La bande de filtres garde sa marge de 6 (`AUTOCOMPLETE_FILTER_BAR_PAD`, le
`padding: 6px` de `.wakfu-autocomplete-categories`), qu'elle partageait jusque-là avec la rangée.

**La barre, seconde version.** Le rail existe désormais (`AUTOCOMPLETE_SCROLLBAR_TRACK`,
`#473f30`) : le web fait le sien à 68 % de la surface qui le porte (`#1a1a1a` sur `#262626`), et
c'est ce rapport appliqué au brun de la liste. Plus d'élargissement — 8 px dans tous les états. La
poignée reste grise au repos (`HEADING_TEXT`) et prend `SELECT_ROW_HIGHLIGHT` sous le pointeur et
pendant le glissement (`AUTOCOMPLETE_SCROLLBAR_THUMB_HOVERED`) : egui n'applique `hovered` que le
pointeur **sur la poignée**, pas seulement dans sa colonne. Le pointeur devient une main sur les
rangées sélectionnables et sur les filtres (`cursor: pointer` du web), reste une flèche sur une
rangée désactivée.

**Le défilement au clavier, et ce qu'il a révélé.** Une flèche appelle maintenant
`ui.scroll_to_rect(rangée, None)` — le `scrollIntoView({ block: 'nearest' })` de `moveActive()`
côté web, du strict nécessaire — et la `ScrollArea` est sans animation (`animated(false)`) pour que
la rangée soit en vue à la frame même. Le test qui le prouve
(`options_alertes_les_fleches_font_defiler_la_liste` : huit suggestions, six ↓, capture, Entrée) a
trouvé un second défaut en passant : **Entrée ne choisissait rien.** Un `TextEdit` à une ligne rend
le focus sur sa touche de retour, et il est peint avant le panneau — à la frame d'Entrée,
`has_focus()` était déjà faux, le panneau se fermait sans sélection. Aucun test n'avait validé une
suggestion autrement qu'au clic. Le champ compte désormais comme focalisé pendant la frame où
Entrée vient de le lui reprendre (`lost_focus() && key_pressed(Enter)`), et si rien n'est
sélectionnable (entrée désactivée, filtre vide) il reprend le focus pour que le panneau reste.

Même nuit, hors composant : la marge basse des tuiles d'alerte passe de 5 à 18 px, **le même écart
sous le nom qu'au-dessus de l'emplacement** (`TILE_BOTTOM_INSET = TILE_BADGE_ROW`) ; la tuile fait
123 px. *Annulé une heure plus tard, voir ci-dessous.*

### Deux corrections sur le retour (2026-09-12, plus tard dans la nuit)

- **La poignée prend la couleur de la liste au repos** (`AUTOCOMPLETE_SCROLLBAR_THUMB =
  SELECT_LIST_FILL`), plus le gris `HEADING_TEXT` : « c'est peut-être ça qui me perturbait ». Sur
  son rail plus sombre, elle se lit comme un morceau de liste qui glisse dans une gouttière ; sous
  le pointeur elle garde la teinte des rangées survolées. Une inscription d'un pixel de chaque
  côté a été évoquée puis retirée dans la même phrase (« non, je n'ai rien dit ») : la poignée
  fait la largeur du rail.
- **Le curseur devient une main qui agrippe sur la barre** (`CursorIcon::Grab`, `Grabbing` pendant
  le glissement). egui ne rend pas la réponse de sa barre : le composant connaît sa colonne
  (`inner_rect.right()` → bord de la liste) et lit l'origine de l'appui pour savoir si le
  glissement en cours y a commencé. *Retirée une heure plus tard, voir ci-dessous.*
- **La marge de 18 sous le nom des tuiles était un malentendu.** La demande était l'inverse :
  ramener la marge du HAUT à celle du bas, cinq pixels, « comme ça on va gagner en hauteur ».
  L'emplacement remonte donc à 5 px du bord, dans la rangée des badges — ils occupent les coins
  (14 px à 5 px du bord), lui le centre (27 px de chaque côté), ils ne se touchent pas. La tuile
  fait 97 px (`TILE_SLOT_TOP = TILE_BADGE_INSET`, `TILE_BOTTOM_INSET = 5`), contre 110 avant le
  malentendu et 123 pendant. *Le bas est remesuré depuis la ligne de base une heure plus tard, voir
  ci-dessous.*

### Le rail autour de la poignée, et la tuile au ras du nom (2026-09-12, fin de nuit)

Retour sur l'overlay réel, après les deux corrections ci-dessus. Le composant « est nickel », à
une chose près, et la tuile n'était toujours pas d'équerre à l'œil.

- **Plus de main qui agrippe.** Demandée puis retirée dans la même nuit : la teinte de la poignée
  sous le pointeur suffit à dire qu'on peut agir dessus. Le code qui lisait la colonne et l'origine
  de l'appui est parti avec.
- **Le rail déborde la poignée d'un pixel de chaque côté, en butée aussi.** Une poignée de la
  couleur de la liste posée sur un rail exactement de sa largeur ne se distinguait plus de la
  liste quand elle touchait un bout. Le rail fait donc 10 px de large et 1 px de plus à chaque
  extrémité (`AUTOCOMPLETE_SCROLLBAR_TRACK_INSET = 1`), rayon 5 pour rester concentrique à la
  poignée de rayon 4. **egui ne sait pas faire ce rail** : il donne au sien exactement l'étendue de
  la poignée (même `cross`, même `scroll_bar_rect`). Le composant le peint lui-même sous la
  `ScrollArea`, sur la colonne qu'elle réserve (`floating_allocated_width = 10`), et rend le rail
  d'egui transparent (opacités de fond à zéro) ; `bar_outer_margin = 1` décolle la poignée du bord
  et la centre.
- **Le bas de la tuile se règle sur la ligne de base du nom.** À 97 px, la capture de l'utilisateur
  montrait 5 rangées vides entre la bordure et le cadre de l'emplacement, mais **9** entre les
  lettres du nom et la bordure basse : la ligne de texte (18 px) descend 6 px sous ses lettres, et
  les 5 px de marge étaient comptés depuis son bord. « Peu importe le nombre de pixels, autant
  d'écart en haut qu'en bas. » `design::Label::baseline` rend désormais la distance du haut d'un
  libellé à sa ligne de base (lue sur le premier glyphe, `Glyph::pos.y`, entière), et la hauteur de
  tuile devient `tile_height(ui)` = 5 + 64 + 5 + ligne de base + 5 + bordure 2, soit **93 px**.
  Le nom, lui, ne bouge pas : c'est la tuile qui remonte de 4 px sur lui.

Hors composant, la même nuit : la fenêtre Options s'ouvre sur son **premier onglet**, « Alertes »
(`OptionsTab::default()` = première entrée du menu, quelle qu'elle soit), et « Valider » ne
renvoie plus un chemin de log inchangé au moteur — ce renvoi relisait `wakfu.log` depuis le début
dans une session qui gardait son état, et dupliquait le combat en cours à chaque validation (voir
`Engine::forget_session`).

### Ce que la galerie ne pouvait pas rattraper — le clic, en vrai (2026-09-12)

La galerie force le panneau déplié (`preview_open`), donc elle ne prouve rien du **chemin réel** :
le champ prend le focus, puis une suggestion est cliquée. Ce chemin était cassé **des deux bouts**,
et la seule chose qui l'a montré est un test qui clique et qui tape
(`options_alertes_champ_d_ajout_trouve_et_ajoute`). Retour utilisateur : « j'ai essayé le champ
d'auto-complétion mais celui-ci ne semblait pas fonctionner ».

1. **Le panneau ne s'ouvrait jamais.** `design::input` rendait `frame_response.union(edit_response)`.
   `Response::union` conserve l'id de l'opérande de **gauche**, et `has_focus()` interroge la mémoire
   d'egui avec cet id — pas un drapeau que l'union combinerait. L'appelant recevait donc l'id du
   cadre, qui n'est pas focalisable : `has_focus()` était **toujours faux**, ici comme partout
   ailleurs. Corrigé dans `input.rs`, dans l'autre sens.
2. **Aucune suggestion n'était cliquable.** Un clic tient en deux frames : l'**appui**, qui retire le
   focus au champ (le pointeur est sur le panneau, pas sur lui), et le **relâchement**, seul moment
   où egui rend `clicked()` vrai. Une condition d'ouverture réduite à `field.has_focus()` ferme donc
   le panneau entre les deux : la rangée n'est plus peinte à la frame du relâchement, son clic
   n'arrive jamais. Le panneau mémorise désormais son rectangle (`ds-autocomplete-panel-rect`) et
   reste ouvert tant que le pointeur est dessus — rectangle **effacé** dès la fermeture, pour qu'un
   reste périmé ne le rouvre pas au simple passage de la souris.

Le test sépare volontairement appui et relâchement en deux `run()` : les garder dans la même frame
masque exactement ce défaut.

### Les jetons

Tous préfixés `AUTOCOMPLETE_*` dans `design/tokens.rs`, sous un en-tête qui dit explicitement qu'il
s'agit d'un **portage du CSS web** et non d'une mesure sur asset : bande 38, bouton de filtre 26
(icône 20), opacité de repos 0,6 → alpha 153, boîte de gemme 14 (une gemme 13 × 20 y entre en
9,1 × 14, jamais un `splat`), message vide 34 — et, depuis le 2026-09-12 au soir, la rangée
entière du web : 35 de haut, marges 10, colonne d'image 30 à 20 de la marge, image 24, écart 10
(voir « La rangée du web, cote pour cote »).

---

## `design::item_slot` — emplacement d'objet (2026-09-11)

`crates/overlay-ui/src/design/components/item_slot.rs`

```rust
use overlay_ui::design::{self, ItemRarity, SlotCount, SlotFrame};

ui.add(
    design::item_slot()
        .frame(SlotFrame::Rarity(ItemRarity::Legendary))
        .icon(texture_id)
        .count(SlotCount::Fraction { current: 137, target: 500 }),
);
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `frame` | `Rarity(ItemRarity)` / `Plain` | `Plain` |
| `icon` | `egui::TextureId` déjà résolu | aucune, l'emplacement est peint vide |
| `count` | `Simple(i64)` / `Fraction { current, target }` | aucun |
| `size` | côté du carré | `ITEM_SLOT_SIZE` = 64 |

Textures : les sept `DsTexture::ItemBorder*`, entrées au manifeste avec ce composant.

### L'ordre de peinture EST le composant

**La bordure de rareté se peint SOUS l'icône. Le cadre simple, PAR-DESSUS.** Ce n'est pas une
préférence :

- la fenêtre intérieure des `Border-*.webp` **n'est pas un trou transparent** — c'est un aplat
  semi-transparent (~70 %) teinté par la rareté, vérifié sur les octets décodés. Peinte après
  l'icône, elle la recouvre entièrement : « j'ai l'impression que tu as mis les objets en opacité »,
  rapporté le jour même de leur arrivée, flagrant sur le jaune-olive du légendaire ;
- un cadre simple est au contraire un **liseré net**, qui doit rester visible si l'icône déborde.

Cet ordre est décrit en **données** (`paint_order`, une fonction libre) plutôt qu'en suite
d'instructions, et deux tests le verrouillent. Un bug qu'on rattrape à l'œil une fois ne se rattrape
pas à chaque relecture.

### Ce que l'appelant fournit, et ce qu'il ne fournit pas

L'icône arrive en `TextureId` **déjà résolu**, et ce n'est pas une entorse à « aucune texture en
paramètre » : cette règle vise les assets du design system, que le composant doit résoudre depuis
une intention. Une icône d'objet est du **contenu** — téléchargée, mise en cache, indexée par le
catalogue, tout cela hors du design system.

La **rareté**, elle, est une intention : `ItemRarity` est un type du design system, et c'est au
panneau de traduire son `WakfuRarity` métier (`watchlist::to_slot_rarity`) — un composant n'accède
pas à `overlay_engine`.

### Deux tailles d'icône, indépendantes

| Cadre | Icône |
| --- | --- |
| `Rarity` | la fenêtre intérieure de la texture (≈ 0,797 du côté), réduite de 4 % |
| `Plain` | `ITEM_SLOT_PLAIN_ICON_FILL` ≈ 0,517 — un **rapport** (30/58 mesuré sur le template web), pas une cote : l'icône suit le côté qu'on donne à l'emplacement |

**Aucune ne se déduit de l'autre**, et les confondre a produit un défaut réel : la première version
faisait occuper tout le carré à l'icône d'un cadre simple. La bordure de rareté masquait le problème
sur les objets — c'est le snapshot d'une tuile d'**ennemi** qui l'a révélé, avec un monstre deux
fois trop gros.

### Vérification

**Aucun snapshot n'a bougé** : la migration de `watchlist::entry_tile` est équivalente au pixel,
compteur compris. Quatorze constantes locales ont disparu du panneau, qui ne garde que ce qui lui
appartient — résoudre l'icône distante, lire la rareté au catalogue, traduire vers le design system.

~~**Les cotes restent celles du web**~~ — **passées à celles du jeu le 2026-09-12** : le carré va de
58 à **64 px** et le rayon des coins de 10 à **2**. Ce qui a décidé la valeur haute de la fourchette
relevée (`item_slot_square` 63-64, une mesure pixel d'un bord adouci n'ayant pas de frontière nette)
est une coïncidence qui n'en est pas une : le liseré des `Border-*.webp` occupe 1/30ᵉ de leur
canevas, soit **2,1 px rendu à 64** — exactement l'`item_slot_border` relevé sur les mêmes captures.
À 58 il en faisait 1,9.

Le rayon suit `shape.corner_style_inputs_lists` du relevé (« square_or_near_square ») : dans le jeu,
une case d'inventaire est un carré. 2 plutôt que 0 parce que le contour extérieur des textures de
rareté est lui-même arrondi (rayon ≈ 1/16ᵉ du canevas, ≈ 4 px à 64) — un fond parfaitement
rectangulaire pointerait hors de ses coins.

**`item_slot_gap` (2 px) n'a PAS suivi**, et c'est délibéré deux fois : cet espacement décrit la
densité d'une grille d'inventaire, pas celle d'un bandeau de suivi posé par-dessus le jeu (où le 12
px vient d'un réglage utilisateur) — et surtout il n'appartient pas au composant : un emplacement ne
connaît pas son voisin, c'est l'appelant qui espace.

Onze snapshots régénérés (la galerie et les dix du panneau Suivi), dont la **largeur de la fenêtre
du Suivi**, qui se calcule sur la taille de tuile. `watchlist::TILE_SIZE` ne porte d'ailleurs plus sa
propre valeur : il valait `58.0` en dur, la même que le jeton mais écrite deux fois — le passage du
composant à 64 aurait laissé le bandeau à 58 sans que rien ne le signale.

~~**Un doublon daté**~~ — **résorbé le 2026-09-11** : les deux maquettes du testkit
(`alertes-mockups`, `composants-a-concevoir`) appellent le composant, `UiIcons::item_border` a
perdu son dernier appelant et les sept textures ne sont plus chargées qu'une fois. Les **7,3 Mo**
payés deux fois (4,9 % du budget) sont rendus. La traduction `WakfuRarity → ItemRarity` a suivi le
même chemin : elle est passée de `panels::watchlist` à `crate::rarity_bridge`, parce qu'un exemple
n'a pas à traverser un panneau pour convertir une rareté.

### Journalisation (clause 4)

Sous `tokens::ITEM_SLOT_MIN_SIZE` (8 px = 4 × le liseré), le cadre et sa marge mangent tout le
carré. L'emplacement est **peint quand même** — §3 veut qu'un rectangle trop petit se voie sur la
capture plutôt que de paniquer — et un `warn!` part **une fois par instance**, mémorisé sur l'id de
la réponse comme le fait `design::slider` pour ses crans. Le seuil est **choisi, pas mesuré**, et sa
doc le dit : personne ne demande sciemment un emplacement de 6 px, c'est le signe d'une largeur
calculée tombée à rien. La galerie en montre un, à droite de la rangée des cas.

---

## `design::meter` — jauge (2026-09-11)

`crates/overlay-ui/src/design/components/meter.rs`

```rust
ui.add(design::meter(0.42).width(190.0));
ui.add(design::meter(ratio).fill(couleur).width(190.0));

// Pour un appelant qui pose déjà sa géométrie :
design::paint_meter(ui, rect, ratio, couleur);
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `fill` | teinte du remplissage | `METER_FILL` = `#077982` |
| `width` | largeur imposée | toute la largeur disponible |
| `height` | hauteur | `METER_HEIGHT` = 16 |

### Six couches, et le piège est géométrique

Là où `item_slot` avait un piège d'**ordre**, la jauge en a un de **géométrie** : chaque couche se
déduit de la précédente par un `shrink`, et son arrondi doit décroître d'autant. Écrire les rayons à
la main donne des coins non concentriques — visible sur un arrondi de 4 px.

| Couche | Rectangle | Arrondi |
| --- | --- | --- |
| bordure extérieure | le rectangle donné | 4 |
| bordure intérieure | `shrink(2)` | 2 |
| piste | `shrink(2)` encore | 0 |
| remplissage | fraction de la piste | conditionnel |
| reflet | tiers supérieur du remplissage | coins hauts seulement |
| curseur de fin | 2 px à l'extrémité | 1 |

### L'arrondi conditionnel

**Les coins droits du remplissage ne s'arrondissent que s'il atteint le bout de la piste.** Sinon
son bord tombe au milieu et un coin arrondi y suggérerait un bord qui n'existe pas. C'est
`fill_corners`, fonction libre testée : le défaut est invisible sur une jauge pleine ou vide —
c'est-à-dire dans les deux cas qu'on regarde en premier.

Le seuil de « pleine » est **0,999 et non 1,0** : une fraction calculée en `f32` peut sortir à
0,9999998 pour un rapport qui vaut exactement un, et la jauge du premier combattant du classement —
le cas le plus fréquent — afficherait alors un curseur collé au bord droit et deux coins carrés.

### Les teintes

Le **remplissage** vient de l'appelant : le panneau Combat fait varier la couleur de sa barre selon
la part de dégâts. Le **reflet** est fixe (`#0dbebe`), demandé comme un ton précis plutôt que comme
une dérivation du remplissage — un éclaircissement automatique a existé, il ne donnait pas ce ton.

Les six couleurs ont été **mesurées pixel par pixel** sur une maquette fournie, après une première
tentative approximée à l'œil qui « dénotait du jeu ». Contre-intuitif et conservé tel quel : la
bordure *extérieure* est un gris moyen, c'est l'*intérieure* qui est presque noire.

### Vérification

**Aucun snapshot n'a bougé** : la migration de `combat::damage_bar` est équivalente au pixel. Sept
constantes disparaissent du panneau, qui ne garde que le calcul de la part de dégâts et sa teinte —
du métier, que le composant ne saurait pas faire.

---

## `design::portrait` — portrait de combattant (2026-09-11)

`crates/overlay-ui/src/design/components/portrait.rs`

```rust
use overlay_ui::design::{self, PortraitShape};

ui.add(
    design::portrait(texture_id)
        .shape(PortraitShape::Round)
        .size(48.0)
        .dimmed(fighter.is_ko)
        .percent(Some(42)),
);

// Pour le gabarit à six emplacements, qui pose déjà ses centres :
design::paint_portrait(ui, rect, texture_id, PortraitShape::Round, dimmed);
design::paint_portrait_percent(ui, rect, design::portrait_percent(dmg, total));
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `shape` | `Square` / `Round` | `Square` |
| `size` | côté du carré englobant | 40 (la liste plate) |
| `dimmed` | applique le grisé KO | `false` |
| `percent` | `Option<i64>` incrusté au coin | aucun |

### Deux formes, parce que le jeu en a deux

Le gabarit de combat loge ses portraits dans des **médaillons ronds**, la liste plate — celle qui
prend le relais au-delà de six alliés — les pose **carrés**. `PortraitShape::corner_radius(size)`
porte le calcul plutôt que de le laisser à chaque appelant : `taille / 2` écrit à deux endroits finit
par diverger d'un pixel.

### Le grisé est une approximation, et c'est l'appelant qui décide

Une teinte egui **multiplie** : elle assombrit sans désaturer, là où un vrai niveau de gris
désature. Les portraits de classe ont leur version grise **précalculée** dans l'atlas et n'ont donc
pas besoin de la teinte — la leur serait moins bonne. Une icône de monstre téléchargée ou le repli
générique n'ont pas d'équivalent gris et s'en contentent.

Le composant ne peut pas trancher : seul l'appelant sait laquelle des trois textures il tient. D'où
`dimmed` en paramètre plutôt qu'une déduction depuis un `is_ko` que le composant ne verrait pas.

### Le pourcentage déborde du carré, volontairement

Il se pose au coin bas-droit du **carré englobant**, décalé encore de 2 px à droite et 1 en bas :
« comme si on traçait un carré autour du rond et qu'on plaçait le pourcentage tout en bas à
droite », puis « encore un peu plus sur la droite pour qu'il mange un peu moins sur le portrait ».
Sur un portrait rond, ce coin est hors du disque — c'est précisément ce qu'on veut.

Il prend `OVERLAY_ACCENT` et **non la teinte de la jauge**, après un aller-retour : les deux ont été
alignées un temps, puis re-séparées (« je préfère la couleur accent qu'il y avait avant »).

`portrait_percent(damage, total)` est une fonction libre testée : **un total nul est le cas réel du
tout début d'un combat**, et une division par zéro y produirait un `NaN` qui se propage jusqu'au
texte peint — « NaN% » sur un portrait.

### Vérification

**Aucun snapshot n'a bougé** : la migration de `combat::paint_flat_portrait` et des deux boucles du
gabarit est équivalente au pixel.

## `design::table` — tableau (2026-09-12)

`crates/overlay-ui/src/design/components/table.rs`

```rust
use overlay_ui::design::{self, TableAlign, TableBody, TableColumn};

design::table()
    .column(TableColumn::fixed("Date", 104.0))
    .column(TableColumn::flex("Nom", 1.0))
    .column(TableColumn::fixed("Prix", 108.0).align(TableAlign::End))
    .body(TableBody::Rows(offres.len()))
    .max_height(12.0 * design::tokens::TABLE_ROW_HEIGHT)
    .empty_text("Aucune vente sur la période")
    .log_name("hdv.historique")
    .show(ui, |row| {
        let offre = &offres[row.index()];
        row.cell(|ui| { ui.label(&offre.date); });
        row.cell(|ui| { ui.label(&offre.nom); });
        row.cell(|ui| { ui.label(&offre.prix); });
        if row.response().clicked() { /* l'appelant décide */ }
    });
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `column` / `columns` | `TableColumn::fixed(label, px)`, `::flex(label, poids)` ou `::numeric(label, px)`, `.align(…)` | aucune colonne |
| `body` | `Rows(n)` / `Empty` / `Loading` | `Empty` |
| `empty_text` | message du corps vide | aucun — le corps reste vide, comme dans le jeu |
| `row_height` | hauteur d'une ligne | 60 (la cote du jeu) |
| `width` | largeur totale | celle du `Ui` |
| `max_height` | borne du **corps** : au-delà il défile, en-tête figé | aucune |
| `preview_loader_frame` | fige le rouage de `Loading` | horloge |

Conteneur de la famille §1 bis, **forme closure**. `Table::height()` rend la hauteur totale *avant*
le rendu : c'est ce qui permet à un appelant de peindre quelque chose derrière le tableau, ou de
réserver sa place.

### Trois cotes mesurées, et tout le reste vient de l'appelant

Relevé : [`hdv-table.json`](design-system/hdv-table.json). Hauteur de ligne **60 px**, encre
d'en-tête **14 px**, écart encre → première ligne **7 px** — les trois invariants sur les trois
captures HDV. L'écart de 7 px **portait le même jeton que `HEADING_TO_ROW`**, les deux mesures — sur
deux interfaces différentes — étant tombées sur la même valeur. L'alias est retiré depuis le
2026-09-14 : celle de la fenêtre Options s'est révélée fausse (13 px, voir `design::heading`) et
aurait emporté l'en-tête de tableau avec elle, alors que ses trois captures disent toujours 7. Deux
mesures qui coïncident ne sont pas une mesure commune. La teinte des libellés (`#b9babb`), elle,
reste celle des titres de section (`#b8b9ba`) à un canal près : un vrai jeton partagé.

L'en-tête est en **linéale**, pas dans la serif des titres — vérifié sur la capture agrandie ×4,
c'est le genre de détail qu'aucune mesure numérique ne donne. Corps 17, contrôlé au rendu : 12 px
d'encre et 36 px de large pour « Date », contre 12 et 37 dans le jeu.

**Les trois tableaux relevés sont vides** (« 0 Objet »). Rien de ce qui concerne une ligne remplie
n'est mesurable : alignement des valeurs, typographie des cellules, icône d'objet de la colonne Nom,
texte trop long. Le composant n'en invente rien — il **donne la cellule à l'appelant** et ne peint
aucun contenu. Seul `TABLE_CELL_PAD_X` est un choix, emprunté à `SELECT_PADDING_X`, et il l'annonce.

### Les nombres à droite, le texte à gauche

`TableColumn::numeric(label, px)` pose l'alignement à droite, **en-tête compris**. C'est une règle,
pas un raccourci d'écriture&nbsp;: des nombres alignés à gauche se comparent mal, les unités ne
tombent plus les unes sous les autres et « 980 » paraît plus long que « 145 000 ». Un texte ou une
date se lisent depuis leur début et restent à gauche. La règle vit dans le constructeur pour
qu'aucun tableau de l'overlay ne l'oublie&nbsp;; `fixed` + `align` reste disponible pour l'exception.

**`Layout::with_main_align` ne suffisait pas.** Sur une `Ui` dont le rectangle est imposé, egui pose
quand même le premier widget au départ de son curseur&nbsp;: la colonne « alignée à droite » sortait
collée à gauche. Il faut changer le sens de parcours (`right_to_left` pour `End`,
`centered_and_justified` pour `Center`). Mesuré après correction&nbsp;: « 12 400 » et l'en-tête
« Prix » finissent tous deux à x=725. Conséquence à connaître&nbsp;: dans une cellule à droite,
plusieurs widgets s'empilent **de droite à gauche**, le premier posé étant le plus à droite.

### Le zébrage éclaircit, il ne colore pas

Le tableau du jeu n'a **pas de fond propre** : le décor se lit à travers. Une ligne sur deux porte
donc un blanc translucide, jamais deux aplats opaques — sur un overlay posé par-dessus un jeu en
mouvement, deux aplats seraient faux à chaque frame.

L'alpha est déduit colonne par colonne, `(claire − nue) / (1 − nue/255)`, sur quinze colonnes de
x=60 à x=1180 : **médiane 12,3**, valeurs de 10,1 à 16,2. La dispersion est celle du décor, pas de
la mesure. Deux colonnes seules donnaient 13,5 : l'échantillon comptait.

La galerie le démontre sur **six bandes de fond de luminances différentes** — sur un fond uni, la
démonstration serait invisible. Ce bloc est **relégué en fin de section** depuis un retour
utilisateur du 2026-09-12 («&nbsp;on a l'impression que c'est un damier&nbsp;»)&nbsp;: il démontre
bien ce qu'il doit démontrer, mais il rend les lignes illisibles. Les cas d'usage se jugent donc sur
le fond uni de la planche, et la mesure garde son bloc à part, annoncé comme tel.

### Les positions de colonne sont imposées par le composant, et c'est un constat du relevé

Les six libellés ont partout la même largeur d'encre d'une capture à l'autre, mais leurs abscisses
varient avec la largeur du tableau **sans règle lisible** : entre « Enchantement » et « Quantité »
l'écart vaut 160 px dans deux captures sur trois, ailleurs rien ne se répète. Le relevé conclut
qu'il faut soit une quatrième capture, soit que le composant impose sa propre répartition. C'est ce
second choix, et il est **en données** : `table_column_spans(colonnes, largeur)` — fixes d'abord,
reste au prorata des poids élastiques, réduction proportionnelle si les fixes ne tiennent pas — avec
quatre tests qui le verrouillent. Sans aucune colonne élastique, le reste **demeure à droite** :
une largeur imposée l'est vraiment.

### Les deux états que le jeu ne montre pas

`Empty` et `Loading` sont une **décision de l'overlay**, pas un relevé : les tableaux à « 0 Objet »
du jeu sont simplement vides, sans message, et aucune capture ne montre un tableau en chargement.
Les deux sont construits avec des éléments déjà mesurés — le rouage de `design::loader`, le gris de
`TEXT_DISABLED` — plutôt qu'avec des teintes inventées, et leurs hauteurs sont annoncées comme
choisies dans les jetons. `empty_text` reste facultatif : sans lui, le corps garde sa hauteur et
demeure vide, ce que fait le jeu.

### L'identité pend à la réponse du corps

Une `Ui` fille créée sans sel d'identité **hérite de l'identifiant de sa mère**. Deux tableaux posés
dans le même panneau donnaient donc les mêmes identifiants de ligne, et egui l'écrivait en rouge sur
la capture (« Second use of widget ID … ») — c'est la capture qui l'a montré, aucune relecture ne
l'aurait signalé. Tout pend désormais à l'identifiant automatique de la réponse du corps, unique par
position dans l'arbre : lignes, cellules, barre de défilement et avertissement de débordement.

### Ce que le tableau ne peint PAS

La bande claire de 8 px sous le tableau, que le relevé attribuait à un « liseré bas ». Mesure de
contrôle : elle traverse **toute la largeur de la capture** (x 0..1278), bien au-delà des bornes du
tableau (x 24..1262). Elle appartient au décor de la fenêtre — le relevé a été corrigé.

La **pagination** n'en fait pas partie non plus : en bas dans Historique et Rechercher, **en haut à
droite** dans Mes offres. C'est un composant autonome que la page place, pas un pied de tableau.

### Vérification

Snapshot **`design_gallery_table.png`** — et c'est une *seconde* planche, pas un choix de
présentation : `wgpu` refuse une texture de plus de 8192 px de côté et `design_gallery.png` en
occupe déjà 7530. Six cas : peuplé (avec un nom trop long, coupé à la colonne), vide avec message,
vide muet, en chargement, corps borné défilant, et le cas dégénéré (268 px de colonnes imposées dans
200 px) — puis le bloc de contrôle du zébrage sur fond en bandes.

## `design::pagination` — pagination (2026-09-12)

`crates/overlay-ui/src/design/components/pagination.rs`

```rust
use overlay_ui::design::{self, PaginationStep};

match design::pagination(page, total).log_name("hdv.historique").show(ui).step {
    Some(PaginationStep::Previous) => page -= 1,
    Some(PaginationStep::Next) => page += 1,
    None => {}
}
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `pagination(page, total)` | affichés tels quels — le composant ne renumérote rien | — |
| `log_name` | nom d'instance | `pagination` |
| `preview_hovered` | force le survol d'une flèche (galerie) | horloge réelle |

Composant feuille, mais il rend un `PaginationOutcome` et non une `Response`&nbsp;: deux flèches,
deux intentions distinctes qu'une seule `Response` ne saurait pas dire. `Pagination::height()` vaut
`ICON_BUTTON_SIZE`&nbsp;; la largeur est celle du libellé, jamais une valeur figée.

### Ce n'est pas un pied de tableau

En bas dans Historique et Rechercher, **en haut à droite** dans Mes offres. C'est un constat du
relevé, pas une préférence&nbsp;: le bloc alloue sa largeur naturelle et laisse la mise en page à
son appelant.

### Trois choses que la capture a apprises, contre le relevé

1. **Ce ne sont pas des flèches nues.** Le relevé décrivait « deux flèches de 7 px espacées de
   39 px ». Agrandie ×5, la zone montre **deux boutons icône de 36 × 36** — socle arrondi, hachures
   diagonales — séparés de 4 px. Le composant n'en peint donc aucun&nbsp;: il compose deux
   `design::icon_button` en contexte panneau.
2. **Le numéro courant est doré, pas blanc.** Mesuré (244, 216, 158) sur ses pixels pleins, la même
   valeur que « Page ». Seuls la barre oblique et le total sont blancs — *ce qui bouge est en or, ce
   qui borne est en blanc*.
3. **Le glyphe pointe vers la gauche**, malgré son nom de fichier (`icon-triangle-right`). Vérifié
   sur son canal alpha&nbsp;: pointe en x=0, base en x=6. C'est donc « suivant » qui est retourné.
   La première capture a rendu les deux flèches à l'envers — aucune relecture ne l'aurait dit.

### Un glyphe détouré d'un bouton n'est pas forcément sur la grille de 18

`DsIcon::TriangleRight` était déclaré `socle`, donc normalisé à `ICON_BUTTON_CONTENT` (18 px
d'encre). Mesure directe sur la pagination du jeu&nbsp;: **8 × 10 px d'encre dans un socle de 36**,
soit la taille native de l'asset (7 × 10) à un pixel de détourage près. La normalisation
l'agrandissait de 80 %, ce que la première capture a montré sans ambiguïté. Passé à `libre` — et
c'est le registre qui apprend quelque chose&nbsp;: `--from-button` dit d'où vient le détourage, pas
que le glyphe soit sur la grille.

### Le corps a été réglé au rendu, pas par le calcul

13 px de hauteur de **capitale** dans le jeu (le « P » de « Page ») — pas les 16 px d'encre totale,
qui incluent le jambage du « g » et donneraient un corps faux d'un tiers. Le rapport d'encre habituel
(0,805) donnait 16&nbsp;; au rendu, 16 ne produit que 11 px d'encre et **19 en produit 13**. Contrôle
sur le segment entier «&nbsp;Page 0 / 0&nbsp;»&nbsp;: 41 px pour « Page » contre 42 dans le jeu,
83 px pour le libellé entier contre 82.

### Ce qui reste inconnu

**Les deux flèches sont grisées sur les trois captures** (« Page 0 / 0 » — le jeu n'a aucune page à
parcourir). L'apparence d'une flèche *active* n'existe nulle part&nbsp;: le composant laisse
`icon_button` rendre ses états habituels plutôt que d'inventer une teinte.

Le socle **désactivé**, lui, est mesurable et diverge nettement&nbsp;: le jeu le peint à (36, 37, 41)
sur un fond à (28, 30, 34), là où le contexte `Panel` pose `ButtonIconDisabled` en pleine opacité, à
65. L'asset a été détouré d'un écran plus clair et rien ne le ramène au fond sur lequel il est posé.
**C'est un écart d'`icon_button`**, qui vaut pour ses quatre boutons désactivés — le corriger dans la
pagination créerait un second réglage du même socle.

### Vérification

Snapshot `design_gallery_table.png`, section basse&nbsp;: les quatre positions possibles (0/0, 1/12,
6/12, 12/12), un survol forcé, et un total à quatre chiffres qui élargit le bloc.
`design_gallery.png` bouge aussi, du seul fait de la renormalisation de `TriangleRight`.

## À faire — composants identifiés, pas encore écrits

Inventaire refait le 2026-09-10 à partir des assets de `assets/design-system/` (55 fichiers sur 85
ne sont pas encore au manifeste, dont 28 icônes), des captures d'interfaces du jeu
(`assets/design-system/interfaces/`) et du code des panneaux. Classement **par vague** et non par
fréquence : chaque vague fournit ce dont la suivante a besoin.

Les mesures citées comme « déjà relevées » existent dans [`design-tokens.json`](design-tokens.json)
ou dans `docs/design-system/releve-*.json` — c'est du relevé fait, pas du travail à refaire.

### Vague 1 — la structure

Fait disparaître la mise en page absolue. Rien d'autre ne devrait être écrit avant : aujourd'hui,
`panels::options_modal` porte 30 constantes de mise en page et 10 `egui::Rect::from_*` calculés à la
main, soit environ 250 lignes qui ne font que placer des rectangles.

| Composant | Ce qu'il absorbe | Matière disponible |
| --- | --- | --- |
| **`design::tooltip`** | `panels::tooltip` et ses trois enveloppes (`combat::show_tooltip_above`, `watchlist::show_tooltip_left`/`_right`), placement et replis compris. | `TOOLTIP_MARGIN`, `TOOLTIP_GAP`, `TOOLTIP_BG_FILL` mesurés. |
| **`design::icon` + `DsIcon`** — *décidé le 2026-09-10, voir ci-dessous* | Le registre des 34 glyphes, séparé des fonds 9-slice : taille d'encre au manifeste, teinte par jeton. | 28 icônes détourées et inutilisées dans `icons/` ; `tokens::ICON_TINT`/`ICON_TINT_HOVER` et `DsTexture::icon_content_size` existent. |

**`DsIcon` est un type distinct de `DsTexture`** (décision utilisateur, 2026-09-10).

**Motif révisé le même jour**, après les commits `0923a45`, `a372320` et `4762cee` d'une session
parallèle. L'argument d'origine — « chaque glyphe a une taille d'encre propre, un fond 9-slice n'en
a pas, et peindre `icon-minus` (14×2) dans un carré l'étirerait » — **ne tient plus** :
`icon_button::glyph_fit(native, box_side)` met désormais tout glyphe à l'échelle **en préservant son
ratio natif**, et `Input::leading_icon` partage la même règle. Le problème d'échelle est réglé.

Ce qui reste, et qui justifie encore la séparation :

- **`TextureSpec` est un type à deux visages.** Il porte un `slice: NineSlice` dont aucun glyphe ne
  se sert (tous prennent `ICON_SLICE`, marges nulles — un 9-slice dégénéré), et
  `icon_content_size()` est un `match` qui énumère à la main les variantes qui se trouvent être des
  icônes. Déclarer une icône demande donc de penser à un champ qui ne la concerne pas, et d'ajouter
  une ligne à une liste qui n'a pas de garde-fou.
- **12 icônes au manifeste sur 34 disponibles** dans `assets/design-system/icons/`.

Ce qui a changé dans l'appréciation : c'est désormais un **nettoyage de typage**, pas un déblocage.
Rien n'en dépend — ni la vague 1, ni la vague 2. À faire quand le terrain est libre, et **pas en
parallèle** d'une session qui travaille sur `assets.rs` / `icon_button.rs` / la galerie : le
refactor touche exactement ces trois fichiers.

Note sur le rythme : ajouter une icône au manifeste **quand un composant en a besoin** — ce que fait
la session parallèle — est sain, et vaut mieux que déclarer les 34 d'un coup. Un manifeste ne doit
rien contenir de mort.

Exécution, le jour venu :

1. `design/icons.rs` — `enum DsIcon` et sa table (fichier, taille d'encre, nom de texture).
2. Retrait des variantes d'icône de `DsTexture` et de `icon_content_size` avec elles — `DsTexture`
   ne décrit plus que des fonds 9-slice.
3. `design::icon` — une **feuille** au sens du contrat, qui réutilise `glyph_fit` tel quel.
4. `icon_button` et `Input::leading_icon` prennent un `DsIcon`.
5. La galerie des glyphes (`a372320`) suit le nouveau type ; snapshots régénérés.

Coût mémoire, mesuré avant de décider : les 34 icônes décodées en RGBA pèsent **31 Ko** au total —
sans effet sur le budget de 300 Mo (§8 du plan).

**Le contrat de composant a désormais deux familles** (décision utilisateur, 2026-09-10) : une
**feuille** implémente `egui::Widget` ; un **conteneur** — celui qui encadre du contenu fourni par
l'appelant — expose un `show` générique sur le retour de ce contenu. Toute la vague 1 relève de la
seconde famille. Le critère de choix et les deux formes admises sont dans
[`contrat-composant.md`](../.claude/skills/ui-component/references/contrat-composant.md) §1 bis.

`panels::options_modal::chrome` (extrait le 2026-09-10) est déjà un conteneur au sens de ce
contrat, écrit dans un panneau faute d'endroit où le mettre : c'est lui qui remonte dans
`design::window`, sans changer de forme.

### Vague 2 — les formulaires

Ce qu'il faut pour que les onglets Alertes et Personnages de la modale Options existent.

| Composant | Ce qu'il absorbe | Matière disponible |
| --- | --- | --- |
| **`input`** — variantes `Number`, `Search`, état d'erreur | *Extension du composant existant*, pas un second composant (skill `ui-component`, étape 1). | `input-number.png`, `input-search.png`, `empty-input-search.png`, `large-input-*.png` |
| **`design::field`** | Rien aujourd'hui — le *libellé à gauche, contrôle à droite* du jeu (« Prix unitaire », « Quantité », « Durée de publication »). **À ne pas confondre** avec la ligne « contrôle élastique + bouton » de la modale Options, qui n'a pas de libellé et n'a qu'un seul usage. | Captures `interface-hdv-vente-form.png` ; **relevé `ui-blueprint` d'abord**, aucune cote n'existe. |
| **`design::slider`** | Rien aujourd'hui. | Aucun asset découpé — passer par `design-asset` d'abord ; captures dans `interface-options-son.png` et `interface-options-interface.png`. |
| **`tabs`** — variante icône | Onglets à pictogrammes. | `icon-tabs.png` ; dépend du registre `DsIcon` (vague 1). §5.7. |

### Vague 3 — les données

Les composants qui portent ce que l'overlay affiche réellement.

**Le vocabulaire visuel de ces panneaux est tranché** (décision utilisateur, 2026-09-10) : les
panneaux Combat et Suivi **prennent les formes et la typographie du jeu, et gardent leur accent
cyan**. Ce n'est pas un compromis mou, c'est le seul point où l'overlay a une contrainte que le
jeu n'a pas : il se lit **par-dessus** le jeu, sur un fond arbitraire et mouvant. Le cyan
`#00d2ff` n'existe nulle part dans l'interface Wakfu — c'est précisément ce qui l'empêche de s'y
confondre. Une jauge de dégâts or posée sur un décor or se cherche.

Concrètement, trois familles à reprendre, et rien d'autre :

| Ce qui migre | Aujourd'hui | Cible |
| --- | --- | --- |
| **Les polices** | la proportionnelle par défaut d'egui — une Ubuntu *Light*, plus maigre que tout ce que le jeu écrit. Neuf appels `FontId::proportional`, plus deux `FontId::monospace` sur les compteurs de tuile. | `design::text::label_font` / `title_font` |
| **Les formes** | rayons hérités du CSS : tuile 10 px, carte de toast 12 px, bandeau 6 px. | emplacement du jeu — carré, bordure 2 px, rayon 0–2 |
| **L'accent — reste** | cyan `#00d2ff` dispersé en constantes locales de `panels::combat` et `panels::watchlist`. | **un jeton nommé**, `tokens::OVERLAY_ACCENT`, documenté comme le vocabulaire du contenu flottant — plus un vestige du portage |

Ce qui est **déjà** au langage du jeu et qu'il ne faut pas toucher : les cadres de portraits
(gabarits du jeu), les quatre boutons du carré de contrôle (`design::icon_button`, socle
`button-icon-first-plan.png` du jeu), les infobulles, les sept bordures de rareté.

La conséquence pour les composants ci-dessous : **un seul jeu de composants, une seule dimension de
galerie**. `meter`, `badge` et `item_slot` prennent leur teinte d'un jeton — `OVERLAY_ACCENT` pour
ce qui flotte, les jetons du jeu pour ce qui vit dans une fenêtre — jamais d'un paramètre de thème.

| Composant | Ce qu'il absorbe | Matière disponible |
| --- | --- | --- |
| **`design::item_slot`** | `watchlist::entry_tile` — bordure de rareté, icône, compteur incrusté, et l'ordre de peinture dont l'inversion a déjà produit un bug. | 7 `Border-*.webp`, `rarity_borders` (7 raretés), `item_slot_square` 63–64 / `border` 2 **appliqués** (2026-09-12) ; `gap` 2 laissé à l'appelant. |
| **`design::badge`** | `watchlist::paint_count_inline`, les étiquettes de rareté, la pastille d'état. | `status_pill_active` ; `text::OUTLINE_FULL` existe. §5.12, §5.13. |
| **`design::meter`** | `combat::damage_bar` — 68 lignes de rectangles empilés (bord externe, bord interne, piste, remplissage, reflet, curseur de fin, arrondis conditionnels). | Six couleurs mesurées dans `combat.rs`, à promouvoir en jetons. |
| **`design::portrait`** | `combat::paint_flat_portrait`, `panels::combat_frame` et son gabarit à six emplacements. | `crates/overlay-ui/assets/templates/*.png`, atlas de classes, portrait de repli. |

### Vague 4 — les finitions

Ni urgent ni structurant, mais chacun retire du code d'un panneau.

| Composant | Ce qu'il absorbe | Matière disponible |
| --- | --- | --- |
| **`design::toolbar`** | Le carré de contrôle du Suivi : fond translucide, gouttière, groupement de boutons icône. | `watchlist::PANEL_BACKDROP_FILL`, `menu-button-icon-first-plan.png` |
| **`design::segmented`** | Le bascule « Objets mis en vente / Offres d'achat » ; le switch Alliés/Ennemis du panneau Combat en est une variante maison. | `tabs-with-first-tab-active.png` |
| **`design::toast`** | `watchlist::toast_card` et ses confettis — ~300 lignes, avec son générateur pseudo-aléatoire maison. | Portage du web ; aucun asset de jeu correspondant. |
| ~~**`design::dialog`**~~ | **Résolu** — `design::confirm_dialog` existe depuis le 2026-09-12 et peint ses textures détourées depuis le 2026-09-15. | `confirm-box-{body,crest,foot}.png` |

### Écarts restants

Trois écarts avaient été constatés le 2026-09-10. Revérifiés le 2026-09-11, il en reste **un et
demi** :

- ~~**`modal-header.png` dupliqué octet pour octet**~~ — **résolu** par `5ccf0d2` (« refactor :
  bannière de modale au manifeste »). La copie `crates/overlay-ui/assets/ui/options/` n'existe plus,
  `options_modal` passe par `DsTexture::ModalHeader`, et le manifeste documente la résorption.
- **Deux barres de défilement**, mais ce n'est **pas un doublon accidentel** :
  `design::scroll_area` (poignée 6 px, `#515356` / `#c1ad83`, aucun rail) et
  `panels::combat_frame_scroll` (barre 5 px, `#998a6c`, bordure noire alpha 217, rayon 2). La
  seconde est une **divergence assumée** : sa finesse et sa couleur unie viennent d'un retour
  utilisateur explicite (la texture étirée cachait les portraits, voir la doc de module). Les
  unifier demanderait donc une variante du composant, pas une suppression — et cette variante
  attend une décision, pas un nettoyage.
- **Le socle désactivé d'`icon_button` est trop clair sur un fond sombre** (constaté le 2026-09-12
  en écrivant `design::pagination`). Le jeu peint le socle d'une flèche grisée à (36, 37, 41) sur un
  fond à (28, 30, 34) — huit niveaux au-dessus de son fond. Le contexte `Panel` pose
  `ButtonIconDisabled` en pleine opacité, à 65 : l'asset a été détouré d'un écran plus clair, et
  rien ne le ramène au fond sur lequel il est posé. Le contexte `FirstPlan` a déjà rencontré ce
  problème et le traite en gardant son socle de repos sous `DISABLED_DIM` ; `Panel` ne l'a pas
  encore. À corriger dans `icon_button`, pour ses quatre boutons désactivés à la fois — jamais dans
  un composant appelant, qui créerait un second réglage du même socle.
- **Cinq chemins de chargement de texture** (et non six : `options_modal` est passé au manifeste) —
  `DesignSystem::load`, `ui_icons`, `portraits`, `remote_icons`, `combat_frame` — dont quatre copies
  de la même fonction décoder → `ColorImage` → `load_texture`. Le budget mémoire (§8 du plan,
  300 Mo) n'a donc toujours aucun point de mesure unique. C'est l'écart qui reste entier.

### Ce que ce catalogue devrait porter en plus

Une colonne **« qui le consomme en production »** par composant. La fiche du bouton icône la porte
déjà (« les quatre boutons du carré de contrôle du Suivi ») ; les autres non. C'est cette colonne
qui répond, dans six mois, à « puis-je changer ce jeton sans rien casser ? ».

## `design::confirm_dialog` — boîte de confirmation (2026-09-12)

`crates/overlay-ui/src/design/components/confirm.rs`

```rust
use overlay_ui::design::{self, ConfirmChoice};

match design::confirm_dialog("Retirer « Pierre ultime » de vos alertes ?")
    .over(fenetre)                       // la FENÊTRE entière, pas le panneau appelant
    .log_name("alertes.retrait")
    .show(ui)
{
    ConfirmChoice::Yes => retirer(),
    ConfirmChoice::No => fermer(),
    ConfirmChoice::Pending => {}
}
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `confirm_dialog(question)` | la question posée | — |
| `over` | ce que le voile couvre et sur quoi la boîte se centre | `ui.max_rect()` |
| `yes` / `no` | libellés des deux réponses | « Oui » / « Non » |
| `log_name` | nom d'instance | `confirm` |

Composant feuille, mais il rend un `ConfirmChoice` et non une `Response`&nbsp;: une `Response` ne
saurait pas dire *laquelle* des deux réponses a été cliquée, et ressortir le choix par un `&mut` en
paramètre est la maladresse que §6 reproche ailleurs.

### Le voile n'est pas une teinte, c'est une information

Tant que la boîte est ouverte, ce qu'elle couvre est **inerte** — et c'est pourquoi `over` demande
la fenêtre entière, pied de page compris. Un voile rogné au panneau appelant laisserait bannière,
onglets et boutons à pleine luminosité, ce qui se lit comme « ils restent cliquables ». Le composant
va plus loin que l'apparence&nbsp;: il **avale** les clics qui passent à côté de la boîte, pour que
l'inertie annoncée soit réelle.

### Ce n'est pas une popover ancrée au bouton

Le dépôt web a `ConfirmDeleteService`, collé au bouton déclencheur. Le jeu a sa propre boîte, et
elle est dans les captures de référence&nbsp;: `interface-confirm-box.png` (449 × 209) pose
exactement la même forme de question. Boîte autonome et centrée, fond gris **clair** (`#585955`
mesuré), médaillon en crête débordant le corps.

### Trois textures, et pourquoi pas une (2026-09-15)

Le châssis n'est plus peint à la main. Il vient de la capture, détourée (`design-asset`) puis
relevée au pixel (`ui-blueprint`, spec dans
[`design-reference/confirm-box.spec.json`](design-reference/confirm-box.spec.json))&nbsp;:

| Texture | Taille | Rendu |
| --- | --- | --- |
| `ConfirmBody` | 420 × 148 | 9-slice, marges **6 px**, `Stretch` sur les deux axes |
| `ConfirmCrest` | 250 × 57 | **taille native**, centrée en haut, peinte après le corps |
| `ConfirmFoot` | 98 × 9 | **taille native**, centré sous le corps |

**Un seul 9-slice ne pouvait pas les réunir.** Les deux ornements sont centrés et de largeur fixe
quand le corps s'étire&nbsp;: sur la capture, le bandeau doré de la crête s'arrête à 250 px alors que
le corps en fait 420, et au-delà le corps n'a qu'un biseau clair comme sur ses trois autres côtés.
Des marges assez larges pour contenir la crête — 250 px de chaque côté — ne laissent aucune bande
médiane&nbsp;; la laisser dans la bande médiane l'étire. C'est la règle des embouts de bouton (§ du
bouton texte), poussée jusqu'à la texture séparée parce que l'ornement **déborde** ici du rectangle
du composant. Le figer plutôt que l'étirer est une décision utilisateur du 2026-09-15&nbsp;: une
seule capture ne dit pas si le jeu l'allongerait sur une boîte plus large.

**Six pixels de marge, pas cinquante.** Contrairement aux boutons, ce corps n'a pas d'embout
décoratif&nbsp;: le filigrane en arcs de ses coins est mesuré à un écart de 2 niveaux au fond contre
1 au centre, pour un bruit de 8 — sous le seuil du visible, et `component.py insets` rend un
`decor_span` de 2 px. Les marges ne couvrent donc que le rayon (2 px) et le biseau (2 px), plus deux
de sécurité.

**La crête descend 21 px dans le corps** (pointe basse du losange et son cerne sombre). Le corps a
donc été reconstruit sous elle&nbsp;: sans ce nettoyage il porterait une empreinte dorée en pleine
bande supérieure, étirée avec lui.

**C'est l'ensemble qui se centre, pas le corps** — la crête déborde de 36 px au-dessus et le filet
de 9 px en dessous&nbsp;; centrer le seul corps ferait porter tout ce débordement d'un côté.

### Ce que le relevé a corrigé

| Valeur | Avant (estimée) | Après (mesurée) |
| --- | --- | --- |
| Hauteur du corps | 120 | **148** |
| Largeur d'un bouton | 150 | **168** |
| Gouttière entre boutons | 14 | **8** |
| Marge latérale de la rangée | 26 | **38** (centrage strict) |
| Rangée de boutons, depuis le haut | 68 | **82** |
| Centre du bloc de la question | 46 | **42** |

Le corps de police (15 px) et la largeur (420 px) étaient déjà justes. Deux relevés corrigent au
passage l'exemple de référence du skill `ui-blueprint` : les boutons y étaient donnés à 166 et
159 px (mesure prise sur le remplissage, liseré exclu — ils font 168 tous les deux), et le corps
noté « sans bordure visible » alors qu'il porte un bandeau de 4 px, mais **en haut seulement**.

Le centrage règle du même coup une réserve d'ergonomie&nbsp;: une popover recouvrait le bouton
« Valider » de la fenêtre, et son bouton de confirmation tombait exactement là où « Valider »
réapparaissait une fois la popover fermée — un double-clic un peu vif validait la fenêtre.

### Le bouton destructeur du jeu est or, jamais rouge

`docs/design-system.md` réserve nommément le rouge au bouton « Annuler » pleine largeur d'un pied
de fenêtre, et précise que « le bouton "Annuler" d'une boîte de dialogue simple reste kaki/gris
standard, pas rouge ». Un « Retirer » rouge est la convention web du *destructive action*.

**Échap répond « Non »**, jamais « Oui »&nbsp;: une touche ne confirme pas une action destructrice.
L'appelant, lui, doit s'abstenir de lire cette même touche tant qu'un dialogue est ouvert — sinon le
même appui ferme la boîte ET la fenêtre derrière.

### Vérification

Sa propre planche de galerie (`design_gallery_confirm`), et non une section des deux autres&nbsp;:
son voile couvre tout ce qu'on lui donne, donc posé dans un canevas de 7530 px il assombrirait la
galerie entière. La planche peint exprès du contenu dessous — titre, champ, deux boutons — parce
que c'est **ce qu'il assombrit** qu'il faut juger.

### Deux appelants, et c'est ce qui l'a fait naître

Peint d'abord dans la maquette de la page Alertes, puis dans `panels::alerts_tab` au portage. Remonté
au design system le jour où la **garde de fermeture** de la fenêtre Options lui a donné un second
appelant. L'extraction est **à pixel constant** — aucun des trois snapshots de l'onglet Alertes n'a
bougé.

Ils sont **trois** aujourd'hui, et tous dans `panels::options_modal` : installation d'une mise à
jour, déconnexion du compte, garde de fermeture. L'onglet Alertes a retiré la sienne (elle portait
sur un brouillon qu'« Annuler » rattrapait). Le passage aux textures du 2026-09-15 n'a demandé
**aucune modification chez eux** — c'est ce que ce composant achète.

### La question, relevée sur la lettre (2026-09-15)

Retour utilisateur&nbsp;: «&nbsp;l'écriture est un peu trop grasse, peut-être un poil petite&nbsp;;
il faudrait qu'elle soit plus light et un peu plus grande&nbsp;». Les trois réglages en cause
(Ubuntu **Regular**, corps **15**, **blanc pur**) venaient du rendu à la main d'avant le détourage.

La méthode, transposable&nbsp;: plutôt que de convertir une hauteur d'encre en corps — une
convention qui dépend de la police et qui avait déjà produit un libellé 45&nbsp;% trop gros — on
rejoue **les chaînes du jeu** dans chaque candidat, sur le même fond et dans la même couleur d'encre,
puis on mesure des deux côtés avec le même seuillage
(`cargo run -p overlay-testkit --example police-confirmation`).

| | chasse l1 | chasse l2 | fût | encre/col |
| --- | --- | --- | --- | --- |
| **jeu** | **325 px** | **51 px** | **2,72 px** | **5,24** |
| Regular 15 (avant) | 270 | 43 | 2,76 | 5,20 |
| Medium 17 | 314 | 50 | 3,93 | 6,69 |
| Regular 18 | 324 | 51 | 2,95 | 5,54 |
| **Light 18 (retenu)** | **317** | **50** | **2,57** | **5,00** |

Le corps 15 rendait la question **17&nbsp;% trop courte** — le retour était exact et mesurable.

**Le piège du classement&nbsp;: Medium 17 arrive premier en chasse et hauteur** (3,7&nbsp;% d'écart
contre 4,2 pour Light 18) tout en étant **44&nbsp;% trop gras**. Une police grasse compense sa
graisse par une chasse plus courte, et un critère qui ne regarde que l'encombrement la couronne. La
graisse se juge sur le **fût**, jamais sur la boîte. D'où une troisième graisse au design system,
`fonts::LABEL_LIGHT` — le texte courant du jeu est plus léger que ses libellés de bouton.

**L'encre est grise, pas blanche.** Mesurée au cœur des lettres&nbsp;: `#d0d1d0`. Le composant
demandait 255, ce qui rendait la question à 220 au cœur et 213 sur ses franges, contre 208 et 197
dans le jeu.

**Il n'y avait pas de jaune**, contrairement à l'impression rapportée — et c'est la mesure qui le
dit, pas un avis&nbsp;: cœur, franges d'antialiasing, halo et fond sont neutres à ±0,5 d'écart
rouge&nbsp;− bleu **des deux côtés**. Le seul élément chaud de la boîte est la crête dorée posée
juste au-dessus de la question (+8,2), conforme au jeu. Ce qui était réel, c'est l'excès de clarté
ci-dessus, qui fait accrocher l'œil sous l'or du médaillon.

**Le passage au corps 18 a rendu le retour à la ligne obligatoire**&nbsp;: «&nbsp;Fermer l'overlay et
installer la version X&nbsp;?&nbsp;» demande environ 356 px pour 344 utiles. La question est donc
mise en page (`LayoutJob`) et non plus posée d'un bloc, à l'interligne du jeu — **23 px**, mesuré de
centre à centre, soit 2 de plus que l'interligne naturel d'Ubuntu au corps 18. Le point de relevé
(`CONFIRM_QUESTION_TOP`) étant le **centre** du bloc, il vaut quel que soit le nombre de lignes.

### Un paramètre caché, retiré (2026-09-15)

Les deux réponses étaient posées par un `ui.horizontal()` dans un `Ui` enfant. Or `item_spacing` est
**hérité**, et il s'ajoutait à `CONFIRM_BUTTON_GAP` : la galerie, qui pose `(10, 8)`, rendait une
gouttière de 18 px pour 8 mesurés et décalait le groupe de 5 px à droite du centre. Un même dialogue
prenait donc deux formes selon le panneau qui l'ouvrait. Les deux boutons sont désormais posés par
`ui.put` sur des rectangles calculés : leurs positions sont des mesures, pas le résultat d'un layout
dont l'appelant tient une variable.


## `design::label` — libellé élidé (2026-09-12)

`crates/overlay-ui/src/design/components/label.rs`

```rust
use overlay_ui::design;

ui.add(
    design::label("Plan \"Epée de Brâkmar\"")
        .width(108.0)
        .align(egui::Align::Center)
        .log_name("alertes.nom"),
);
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `label(text)` | le texte à afficher | — |
| `width` | largeur imposée — le texte s'élide au-delà | largeur disponible du `Ui` |
| `size` | corps | `LABEL_FONT_SIZE` = 13 |
| `color` | couleur du texte | `LABEL_TEXT` = blanc |
| `align` | `Min` / `Center` / `Max` dans la largeur | `Center` |
| `tooltip_side` | côté de l'infobulle | au-dessus |

Composant feuille (`impl Widget`). Deux fonctions associées pour l'appelant&nbsp;: `Label::height`
(réserver la place avant d'ajouter) et `Label::elides` (voir plus bas).

### Une ligne, jamais deux — et c'est une décision produit

Un nom d'objet Wakfu dépasse souvent la largeur disponible&nbsp;: les quatre « Plan "Epée de … " »
partagent leurs quatorze premiers caractères. Le réflexe est de le faire passer à la ligne pour les
distinguer, et c'est **ce qui a été essayé puis annulé le 2026-09-12**&nbsp;: la tuile porte déjà
l'icône de l'objet, et c'est elle qui lève l'ambiguïté d'un coup d'œil, bien avant le texte. Faire
grandir chaque tuile pour distinguer deux libellés résolvait un problème que l'utilisateur n'a pas.

> « Ton problème de nom, c'est un faux problème puisque l'utilisateur voit des images en plus des
> noms. Pour lui, il y a beaucoup moins d'interprétation que toi, seulement avec des noms. »

Ce qui manque quand un nom est coupé, ce n'est pas de la place&nbsp;: c'est **un moyen de lire la
suite**. D'où l'infobulle.

### L'infobulle n'apparaît que si le texte est réellement coupé

Un nom qui tient en entier n'a rien à révéler — une infobulle qui répète ce qui est déjà lisible est
du bruit. Même règle que le dépôt web (`[tooltipOnlyIfTruncated]="true"`), et c'est **egui** qui
répond ici (`Galley::elided`), pas une comparaison de chaînes qu'un nom finissant déjà par « … »
mettrait en défaut. L'infobulle est celle du design system, pas le `on_hover_text` d'egui&nbsp;:
même socle, même police, même délai que partout ailleurs.

### `Label::elides`, et la zone muette qu'elle évite

Un appelant qui pose sa **propre** infobulle sur une zone plus large — une tuile, une ligne — doit
s'effacer là où celle du libellé s'affichera&nbsp;: deux infobulles sous le même curseur se
peignent l'une sur l'autre. Mais s'effacer *partout* sur le libellé laisse une **zone muette** au
milieu de la tuile dès que le nom tient en entier, puisque le libellé ne dit alors rien non plus.
C'est arrivé, le temps d'une capture.

`Label::elides(ui, text, width, size)` répond à la seule question qui tranche. Les galleys étant
mémoïsés par `Fonts`, l'appeler ne remet pas le texte en page une seconde fois.

### Vérification

Section de galerie&nbsp;: le **même texte à trois largeurs** — l'ellipse vient du rapport entre les
deux, pas du texte — et les trois alignements. L'infobulle, elle, ne peut pas s'y voir&nbsp;: elle
demande un curseur, et aucun ne survole quoi que ce soit en rendu offscreen. C'est
`tests/panels.rs` qui la vérifie, par survol simulé, **dans ses deux cas**&nbsp;: nom coupé →
infobulle du nom entier&nbsp;; nom complet → infobulle de la tuile, et pas celle du nom.

## Piège d'appelant — `Ui::put` avance le curseur du parent (2026-09-12)

Ce n'est pas un défaut de composant, c'est un piège d'**appelant**, et il a produit un bug visible
en jeu qui a traversé une maquette validée, un portage et trois relectures avant d'être vu.

**Le symptôme** : dans la grille de l'onglet « Alertes », trois tuiles affichaient « Plan "Epée
de » à l'identique, sans ellipse, et les noms d'objet étaient coupés net au bord de la tuile. Tout
désignait la mise en forme du texte — largeur de tuile trop petite, élision mal réglée. Mesure
faite, le nom tenait : `Pierre d'aventure` occupait 102 px pour 108 disponibles.

**La cause** était ailleurs. Chaque tuile faisait bien 118 px de large, mais les origines de deux
tuiles voisines n'étaient espacées que de 91 px : **elles se chevauchaient de 27 px**, et le fond
opaque de la suivante effaçait la fin du nom de la précédente.

```rust
let (rect, response) = ui.allocate_exact_size(taille_de_la_tuile, Sense::click());
// …
ui.put(emplacement_centré, design::item_slot());   // ⚠ avance le curseur du parent
```

`Ui::put` ouvre un scope, et un scope termine par `advance_cursor_after_rect`. Le curseur de la
rangée était donc ramené au bord droit de l'**emplacement** — centré dans la tuile, donc 27 px
avant son bord — et la tuile suivante démarrait là.

**La règle** : dans un conteneur qui dispose des éléments (`horizontal`, `vertical`, une grille),
poser quoi que ce soit à une position absolue passe par un enfant, jamais par le `ui` du conteneur.

```rust
let mut cellule = ui.new_child(egui::UiBuilder::new().max_rect(rect));
cellule.put(emplacement_centré, design::item_slot());   // le curseur de la rangée ne bouge pas
```

`Ui::new_child` n'avance rien : c'est ce qui le distingue de `scope`/`put`/`add`. Le même geste
vaut pour tout composant posé par rectangle à l'intérieur d'une cellule déjà allouée.

## `design::legend_tile` — tuile à légende (2026-09-13)

`crates/overlay-ui/src/design/components/legend_tile.rs`

```rust
use overlay_ui::design;

let response = ui.add(
    design::legend_tile("Commerce", "gelano")
        .width(155.0)
        .log_name("chat.recherche"),
);
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `legend_tile(légende, contenu)` | la légende sur la bordure, le texte au centre | — |
| `width` | largeur imposée | largeur disponible du `Ui` |
| `height` | hauteur du **cadre** (la légende s'y ajoute au-dessus) | `LEGEND_TILE_HEIGHT` = 54 |
| `enabled` | `false` = grisée, sans clic | `true` |
| `legend_color` | couleur de la légende — celle du canal de chat (`tokens::chat_channel_color`) | `LEGEND_TILE_LEGEND_TEXT` (gris des titres) |
| `preview_state` | `Idle` / `Hovered` / `Disabled` — galerie et captures seulement | état réel |

Composant feuille (`impl Widget`, `Sense::click()`). Trois fonctions associées :
`LegendTile::legend_overshoot` (de combien la légende déborde au-dessus du cadre),
`LegendTile::allocated_height` (la hauteur totale allouée, pour un appelant qui réserve) et
`LegendTile::paint_frame` (le cadre seul — fond, bordure interrompue, légende décrite par un
`FrameLegend` : texte, couleur, corps, côté `LegendSide::Left`/`Right`, et un fond optionnel derrière
la légende pour un cadre posé par-dessus le jeu, où sa moitié haute déborde sans panneau sombre
sous elle ; une teinte appliquée à chaque couleur — pour un appelant qui compose son propre contenu
dedans : la carte d'alerte de chat du Suivi, `panels::watchlist::chat_toast_card`, légende à droite
en 16 px avec fond).

**Infobulle** : aucune à poser. La tuile en montre une **seulement quand son contenu est élidé**,
avec le mot entier et rien d'autre — jamais pour répéter ce qui se lit déjà, ni la légende sous
les yeux (règle utilisateur du 2026-09-14).

### À quoi elle sert

À poser côte à côte des entrées qui ont **deux informations de poids inégal** : la légende dit la
catégorie, le contenu dit la chose. Née pour les recherches de l'onglet Chat (« Commerce » /
« gelano »), retenue par l'utilisateur le 2026-09-13 contre une liste en lignes : deux fois plus
d'entrées visibles d'un coup, et la catégorie lisible **sans couleur** — l'utilisateur peut avoir
appliqué un thème au jeu, une couleur figée ne correspondrait plus à la sienne.

### Ce que le jeu fournit, et ce qui est emprunté

Le cadre est **celui des champs de saisie** : bordure `#595140` de 2 px, fond `#0e1115`, coins
droits — la signature « cadre » du design system, mesurée (`INPUT_BORDER`, `INPUT_FILL`). Le jeu
n'a pas de cadre à légende : la légende posée sur la bordure, qui s'interrompt sous elle, est un
idiome de formulaire emprunté, et les cotes qui lui sont propres (hauteur 54, légende en 11 px à
10 px du bord, respiration de 4 px de chaque côté) sont des **choix de maquette validés**, pas des
mesures — les jetons `LEGEND_TILE_*` le disent un par un.

### Trois choses à savoir

- **La tuile réserve elle-même la place de sa légende** au-dessus du cadre : un appelant qui les
  empile n'a pas à savoir de combien elle déborde. La première rangée d'une grille n'est jamais
  rognée par le clip de la zone défilable — c'est le défaut qu'avait la maquette.
- **Le contenu est élidé sur une ligne**, jamais rogné en silence, avec une trace `warn!` une
  seule fois par instance ; une légende trop longue s'arrête avant le bord droit.
- **La croix de retrait reste au panneau**, comme pour `design::item_slot` dans Alertes : une
  `Response` ne porte qu'un clic, et la croix est un second clic sur une zone à part. Survolée, la
  tuile se voile (`LEGEND_TILE_HOVER_SCRIM`, le noir à 40 % des tuiles d'Alertes et du Suivi) et
  c'est sur ce voile que le panneau pose sa croix.

### Vérification

Planche dédiée `design_gallery_legend_tile.png` (la galerie principale est au plafond des
8192 px) : les trois états côte à côte, puis quatre tuiles à la largeur d'une colonne de l'onglet
Chat, dont un contenu élidé et une légende tronquée.

### Couleurs des canaux (2026-09-14)

Une légende qui nomme un canal de chat prend **la couleur que le client donne à ce canal** dans sa
légende des canaux (`/l - Proximité` en blanc, `/m - Commerce` en orange, `/r - Recrutement` en
magenta…) : jetons `CHAT_CHANNEL_*` et `tokens::chat_channel_color(ChatChannel)`, relevés à l'œil
sur une capture fournie par l'utilisateur puis corrigés par lui pour cinq canaux (Commerce reste
le relevé). La carte d'alerte du Suivi (`panels::watchlist::chat_toast_card`) reprend le cadre via
`paint_frame`, canal en haut à droite, plus grand, sur fond. « Tous les canaux » n'est pas un canal : la légende garde son gris.

---

## `design::switch` — switch à cases (2026-09-16)

`crates/overlay-ui/src/design/components/switch.rs`

```rust
use overlay_ui::design::{self, DsIcon};

design::switch(&mut personnage.genre)
    .slot(Genre::Masculin, "Masculin").icon(DsIcon::Male)
    .slot(Genre::Feminin, "Féminin").icon(DsIcon::Female)
    .log_name("personnages.genre")
    .show(ui);

// Trois positions, une seule active — le sélecteur de grandeur du panneau Combat, tel qu'il
// est appelé : glyphes en couleurs, 26 px de haut, cases de 35 px.
design::switch(&mut metric)
    .slot(CombatMetric::Damage, "Dégâts infligés (F3)").icon(DsIcon::MetricDamage)
    .slot(CombatMetric::Armor, "Armure donnée (F3)").icon(DsIcon::MetricArmor)
    .slot(CombatMetric::Heal, "Soins prodigués (F3)").icon(DsIcon::MetricHeal)
    .width(109.0)
    .height(26.0)
    .log_name("combat.grandeur")
    .show(ui);
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `slot(valeur, libellé)` | une case, dans l'ordre d'affichage — **deux au moins** | — |
| `variant` | `Frame` (le cadre du jeu) ou `FirstPlan` (le socle de bouton icône de premier plan) | `Frame` |
| `icon` | pictogramme de **la dernière case déclarée** ; remplace le libellé au rendu, qui devient l'infobulle | libellé peint |
| `width` | largeur totale, partagée à égalité entre les cases | largeur native de la variante × n + 2 × (n − 1) — 88 en `Frame`, 74 en `FirstPlan`, pour deux |
| `height` | hauteur imposée ; le 9-slice n'étire que le corps des cases (marges figées 6 + 6) | `SWITCH_HEIGHT` = 44 en `Frame`, `SWITCH_FIRST_PLAN_SIZE` = 36 en `FirstPlan` |
| `scale` | **réduction homothétique** de tout le switch — cases, séparateur, liseré, biseaux, glyphes — chaque case en un quad filtré, sans 9-slice ; dimensions arrondies au pixel | `1.0` |
| `enabled` | le switch **entier** | `true` |
| `preview_state` | `Idle` / `Hovered` / `Active` / `Disabled`, sur la dernière case — **galerie et captures uniquement** | état réel |
| `log_name` | nom d'instance pour le journal | `"switch"` |

La valeur sélectionnée vit chez l'appelant, comme celle de `design::tabs` ; `Response::changed()`
dit à quelle frame elle a bougé. Un switch de moins de deux cases n'est pas peint, et le dit une
fois au journal (`warn!`, « switch incomplet »).

**Ce n'est pas un `tabs`**, et la question a été posée avant d'écrire le fichier : le sélecteur de
genre du jeu est un autre élément — cadre propre (liseré `#221f24`, rayon 6 aux quatre coins),
séparateur plat, et des glyphes qui changent de couleur (doré sur la case active, gris sur
l'inactive) là où l'onglet actif **blanchit** son libellé sur le même kaki. Aucune texture n'est
partagée avec les onglets.

**Deux cases relevées, *n* cases servies.** Le jeu n'a capturé qu'un switch à deux cases ; la
demande d'un switch à trois positions (Dégâts / Armure / Soins du panneau Combat, aujourd'hui
peint à la main dans `panels::combat::paint_metric_switch`) est venue le jour même. Une case du
**milieu** n'a ni coin arrondi ni liseré latéral : ses deux textures sont les 40 px de
remplissage des cases d'extrémité, coin redressé — la dérivation de `tab-active.png`. Une case
inactive du milieu n'a pas d'ombre intérieure : sur les captures, l'ombre est toujours du côté du
liseré extérieur, une case sans liseré n'en porte pas.

**Mesures** (relevé `ui-blueprint` du 2026-09-16 sur l'asset générique 88 × 44, artefact
« Switch à deux cases ») :

| Grandeur | Valeur | Jeton |
| --- | --- | --- |
| Cadre | 88 × 44, liseré 2 px `#221f24`, rayon 6 | `SWITCH_SLOT_WIDTH` = 43, `SWITCH_HEIGHT`, `SWITCH_BORDER` |
| Case active | 40 × 40, kaki `#635a47`, biseau clair 2 px en haut et en bas | texture |
| Case inactive | **42** × 40, gris-brun `#514b44`, ombre intérieure 2 px côté liseré | texture |
| Séparateur | 2 px, aplat `#312d2d`, entre les deux liserés (y 2..42) | `SWITCH_SEPARATOR_WIDTH`, `SWITCH_SEPARATOR`, `SWITCH_BORDER_Y` |
| Glyphe actif / inactif | `#f4d89f` / `#a9a5a2` | `SWITCH_ICON_ACTIVE`, `SWITCH_ICON_INACTIVE` |
| Glyphes | ♂ 14 × 14 et ♀ 10 × 16, **à leur taille native**, centrés | `SWITCH_ICON_SIZE` = 16 |
| Glyphe **en couleurs** inactif | ×150/255 sur les trois canaux (estimation, voir plus bas) | `ICON_NATIVE_DIM` |

**La case inactive est 2 px plus large que l'active**, et le séparateur se déplace donc de 2 px
selon l'état — c'est ce que disent les deux captures. Le composant donne 43 px à chaque case et
laisse le 9-slice absorber l'écart d'un pixel de chaque côté.

**Un glyphe qui tient dans le carré de 16 reste à sa taille native.** Un étalon commun (comme
`ICON_BUTTON_CONTENT`) grossirait le ♂ de 14 à 16, deux pixels de plus que dans le jeu. Seul un
pictogramme plus grand — un `DsIcon::Cards` de 22 px emprunté à une autre famille — est ramené
dans le carré, rapport conservé. Le glyphe est calé sur la grille de pixels : une case de 43 px
met son centre à une demi-position, et un glyphe peint à x + 0,5 s'étale sur deux colonnes.

**Textures** — six, tirées des deux captures du jeu, parce que l'arrondi du cadre est porté par
l'alpha des cases d'extrémité (même raison que pour les onglets) :

| Fichier | Origine | Taille | Découpage |
| --- | --- | --- | --- |
| `switch-slot-active-first.png` | `switch-first-slot-active.png` x 0..42 | 42 × 44 | `SWITCH_SLICE_FIRST` |
| `switch-slot-inactive-last.png` | idem, x 44..88 | 44 × 44 | `SWITCH_SLICE_LAST` |
| `switch-slot-inactive-first.png` | `switch-second-slot-active.png` x 0..44 | 44 × 44 | `SWITCH_SLICE_FIRST` |
| `switch-slot-active-last.png` | idem, x 46..88 | 42 × 44 | `SWITCH_SLICE_LAST` |
| `switch-slot-active.png` | `switch-slot-active-first.png` x 2..42, coin redressé | 40 × 44 | `SWITCH_SLICE` |
| `switch-slot-inactive.png` | `switch-slot-inactive-last.png` x 0..40, coin redressé | 40 × 44 | `SWITCH_SLICE` |

Aucun miroir : le jeu a capturé les deux états. Marges : 8 px côté arrondi (liseré 2 + escalier
d'alpha 5 + 1), 4 côté séparateur, 6 en haut et en bas (liseré 2 + biseau 2 + 2) ; `decor_span`
au niveau du bruit, `Stretch` sur les deux axes. Les deux barres génériques 88 × 44 restent dans
`assets/design-system/` comme référence de comparaison.

**Le survol — mesuré** (capture `switch-first-slot-active-and-second-slot-hover.png` du
2026-09-16, ♂ actif et souris sur ♀, générifiée à 88 × 44 comme les deux autres) : la case
inactive survolée prend **tout** l'aspect de la case active — fond `#635a47`, biseau clair de 2 px
en haut et en bas, glyphe doré, plus d'ombre intérieure côté liseré. Écart moyen entre la case
survolée et la case active de la référence : 1,0/255 (11,5 contre la case inactive). `Hovered`
peint donc les textures actives, aucune texture de plus. La première version du composant dorait
le glyphe seul sans toucher au fond — inventé faute de capture, et faux.

**Inventé, faute de référence** — à remplacer par une mesure dès qu'une capture existera :

- **L'état désactivé.** Fonds atténués comme un bouton désactivé (alpha 190/255), glyphes
  `TEXT_DISABLED`, gouttières atténuées de même ; la case sélectionnée garde son fond actif pour
  rester reconnaissable.
- **Le repli sans pictogramme.** Le libellé est peint dans la case au corps des libellés du jeu
  (`SWITCH_FONT_SIZE` = `TAB_FONT_SIZE`, 17 px) : le jeu n'a pas de switch texte dans les
  interfaces relevées.
- **Les cases du milieu.** Dérivées des bouts, voir plus haut.
- **Une autre taille que 88 × 44.** Deux voies. `scale` réduit **tout** dans le même rapport,
  comme le jeu quand on baisse l'échelle de son interface : à 36/44 (le panneau Combat,
  `SWITCH_SCALE`), une case fait 35 × 36, le séparateur 2, les glyphes 82 % de leur taille ;
  la texture entière de chaque case est peinte en un quad filtré par le GPU (écart moyen
  1,5/255 avec une réduction Lanczos hors ligne, mesuré sur la capture du panneau). `height`
  n'impose que la hauteur : le 9-slice garde ses 6 px hauts et bas et n'étire que le corps —
  à 26 px il reste 14 px de dégradé au lieu de 32, et c'est ce rendu qui a fait dire « on a
  l'impression d'avoir compressé le switch » (2026-09-16) ; à réserver aux écarts de quelques
  pixels. Le panneau Combat a essayé les trois le même jour — 26 par `height`, 44 natif (« le
  rendu dans le jeu est relativement imposant »), puis 36 par `scale` (« vraiment scaler à
  double dimension pour ne pas perdre le rendu visuel »).
- **Des glyphes en couleurs.** Un `DsIcon` de catégorie `couleur` (`native_color()`) est peint
  tel quel sur la case active ou survolée, et multiplié par `ICON_NATIVE_DIM` (150/255) ailleurs
  — le rapport de luminance gris/or du jeu, arrondi vers le bas pour se lire sur un glyphe déjà
  sombre. Les cinq glyphes du panneau Combat sont dans ce cas, parce que leur couleur porte le
  sens (vert = alliés, orange = ennemis, deux cœurs) : passés au blanc par transfert de
  luminance, les silhouettes ne se distinguaient plus que par les bras et les cœurs devenaient
  des taches (planche d'essai du 2026-09-16).

### `SwitchVariant::FirstPlan` — le socle de bouton icône du jeu (2026-09-16, plus tard le même jour)

Demande utilisateur, une fois la variante `Frame` en place sur les deux switches du panneau
Combat : **« permettre au composant switch la texture `button-icon-first-plan[-hover].png` »**, puis
l'appliquer aux switches Alliés/Ennemis et Dégâts/Armure/Soins.

Un paramètre du composant existant, **pas un second composant** : c'est la réponse que le contrat
impose à « il me faut ce contrôle, mais dans une autre matière ». Et ce n'est pas un habillage de
plus par goût — ces deux switches sont les seuls de l'overlay à flotter **par-dessus le jeu**, à
côté des boutons icône du carré de contrôle du Suivi, et non dans une fenêtre du design system. Le
cadre kaki du sélecteur de genre y est un meuble d'interface posé sur la scène ; le socle de
premier plan est la matière que le jeu emploie à cet endroit.

| | `Frame` | `FirstPlan` |
| --- | --- | --- |
| Fond d'une case | six textures de cadre, selon sa position | **un socle carré**, le même pour toutes |
| Gouttière | liseré `SWITCH_BORDER` + séparateur `SWITCH_SEPARATOR` peints dedans | **chevauchement de 2 px**, plus un trait de 1 px `SWITCH_FIRST_PLAN_SEAM` |
| Case native | 43 × 44 | **36 × 36** (`SWITCH_FIRST_PLAN_SIZE` = `ICON_BUTTON_SIZE`) |
| Case choisie | fond kaki | **liseré `#126068` de 2 px + halo d'1 px**, sur la bordure |
| Survol | fond actif, glyphe doré | fond actif **et glyphe allumé** |
| Glyphe actif / survolé / repos | doré / doré / gris chaud | blanc / **blanc** / gris froid `ICON_TINT` |

**Aucune texture nouvelle** : `button-icon-first-plan.png` et son `-hover` sont au manifeste depuis
`design::icon_button` (36 × 36, `ICON_BUTTON_SLICE`, marges figées de 6).

#### La règle d'étanchéité — exigence utilisateur, vérifiée

**Rien de ce lot ne doit toucher la variante `Frame`.** Énoncé tel quel par l'utilisateur
(2026-09-16) : « le switch de sexe dans la modale de création de personnage ne doit pas du tout
changer ». Le chevauchement, le trait de jointure, le liseré, son halo et le survol qui allume le
glyphe appartiennent **strictement** à `FirstPlan`, et chaque différence passe par un `match` sur
la variante — jamais par une valeur partagée qu'on « ajusterait ».

Trois garde-fous, et ils sont de nature différente :

1. **Deux tests unitaires** (`la_variante_cadre_ignore_tout_ce_qui_appartient_au_premier_plan`,
   `la_teinte_du_glyphe_du_cadre_reste_celle_mesuree_sur_le_jeu`) : la gouttière du cadre reste
   positive quand celle du premier plan devient négative, les tailles natives diffèrent, et les
   teintes de glyphe du cadre restent celles mesurées sur les captures du jeu, état par état.
2. **Les captures** : sur les 108 du harnais, **19 ont bougé, toutes `combat_*` ou la planche du
   switch**. `options_personnages_modale.png`, celle qui porte le sélecteur ♂/♀, n'a pas bougé
   d'un pixel — c'est la preuve visuelle que la variante `Frame` est intacte.
3. **La taille du glyphe est volontairement commune** (plafond `SWITCH_ICON_SIZE`, 16) : c'est ce
   qu'elle a toujours été pour `Frame`. `FirstPlan` avait d'abord suivi la grille du bouton icône
   (18 sur 36), qui grossissait de 68 % les cinq glyphes du panneau Combat — ils sont de catégorie
   `couleur` et n'ont donc pas d'étalon d'encre. Le retour au plafond commun ne change rien à
   `Frame`.

#### Le liseré de la case choisie, et son halo

Quatre allers-retours sur rendu (artefact « Réglages du switch premier plan ») ont fixé, dans cet
ordre : la position, la couleur, l'épaisseur, l'opacité.

| Réglage | Valeur | D'où elle vient |
| --- | --- | --- |
| Couleur | `#126068` | **une couleur de la texture** — la teinte la plus vive de `button-icon-first-plan-hover.png` |
| Épaisseur | 2 px | celle de la bordure de la texture (colonnes x 0–1 à `#141519`, corps en 2) |
| Arrondi | 2 | celui de ses coins (alpha 24/255 en (0,0), 32 et 42 sur ses voisins, 236 en (0,2)) |
| Opacité | 85 % | choisie sur rendu comparatif 100 / 85 / 70 % |
| Halo | 1 px, 35 % | même couleur, juste à l'intérieur ; au-delà de 50 % il se lit comme un liseré de 3 px |
| Position | retrait nul | **sur** la bordure, pas dedans |
| Ordre | après toutes les cases | pour recouvrir les bordures des voisines qui la chevauchent |

**Pourquoi une couleur de la texture, et pas un accent.** L'or du design system a été essayé et
rejeté (« pas bon pour la couleur de la texture »), un gris aussi. Le socle sélectionné est un
**dégradé** — `#428087` en haut, `#18383e` en bas : une teinte prise dans sa moitié basse
(`#164047`, essayée) se détache en bas de la case et se dissout dans le haut. `#126068` tient sur
toute la hauteur.

**Pourquoi il est devenu nécessaire.** Le survol allume désormais le glyphe en même temps que le
socle (retour utilisateur : n'éclaircir que le fond laissait une icône éteinte dessus, « un effet
de décalage perturbant »). Conséquence directe : une case survolée et la case choisie sont
devenues identiques. Le liseré est ce qui les sépare.

#### Ce que le panneau Combat y gagne

Plus d'échelle à régler : `SWITCH_SCALE` (36/44, la réduction homothétique du cadre) a disparu — la
variante est native à 36. Les switches passent de 72 et 109 px à **74 et 112** (chevauchement
compris), la hauteur ne bouge pas d'un pixel, et les deux bandeaux comme `BARS_COLUMN_TOP_OFFSET`
en dérivent sans être touchés.

### Vérification

Planche dédiée `design_gallery_switch.png` (la galerie principale est au plafond des 8192 px) :
les deux états du jeu à 88 px, le survol (comparé à sa capture, ci-dessus) et le désactivé, un switch de 160 px à pictogrammes de
22 px, le repli à libellés, et deux switches à trois cases (libellés à 260 px ; pictogrammes à la
largeur native de 133 px, milieu actif, dernière survolée), et les deux switches du panneau Combat
à l'échelle 36/44 (glyphes en couleurs, dernière case survolée, puis désactivé), suivis du même
switch de camp à 72 × 36 par `height` pour comparer les deux voies. Deux rangées de plus pour
`SwitchVariant::FirstPlan` : les deux switches du panneau Combat tels qu'ils sont appelés (74 et
112 px, une case survolée, un switch désactivé), et les **quatre états côte à côte** sur un switch
portant un glyphe monochrome et un glyphe en couleurs — la seule disposition qui permette de juger
le couple actif / survolé, puisqu'ils partagent leur socle.

Et pour la variante, une preuve en creux : `options_personnages_modale.png` (le sélecteur ♂/♀)
**ne doit jamais bouger** quand on touche au premier plan. Elle n'a pas bougé au lot du
2026-09-16. Comparaison au jeu à la même
taille : écart moyen 4,4 et 3,4/255 sur les deux états — le cerne sombre des glyphes du jeu,
absorbé par la teinte comme sur toutes les icônes, fait l'essentiel des pixels en écart. Boîtes
d'encre : ♂ au pixel près, ♀ décalé de 1 px (le jeu le pose lui-même 1 px à droite du centre de
sa case de 42).

### Appelants

- **`panels::combat`** (2026-09-16) — les deux switches du panneau, camp Alliés/Ennemis
  (`show_side_row`) et grandeur Dégâts/Armure/Soins (`show_leader_row`), à la place de
  `paint_side_switch`/`paint_metric_switch` (piste `TINT_MEDIUM`, option `ACCENT`, peintes à la
  main). Choix faits sur rendu du harnais, avant / après en artefact, quatre variantes présentées :
  bandeaux opacifiés conservés — celui du camp calé à gauche sur le cadre et débordant vers la
  gouttière (retour utilisateur : garder les marges latérales du design), icônes en couleurs.
  Taille arrêtée en trois temps le même jour : 26 px par `height` (« compressé »), 44 natif
  (« imposant »), puis **`scale(36/44)`** — cases de 35 × 36, switches de 72 et 109 px, bandeaux
  de 48 px, bandeau de camp à 84 px, bandeau leader à 202 px (débord permanent de 6 px de chaque
  côté de la colonne, pour loger un total à sept chiffres au corps 16 à côté du switch de
  grandeur — voir `combat::show_leader_row`).
  Les cinq glyphes sont entrés au registre `DsIcon` (`Allies`, `Enemies`, `MetricDamage`,
  `MetricArmor`, `MetricHeal`, catégorie `couleur`), et les fichiers d'`assets/ui/` qu'`UiIcons`
  chargeait ont été supprimés.
  **Puis, le même jour** : `variant(SwitchVariant::FirstPlan)` sur les deux (demande utilisateur,
  voir la section de la variante ci-dessus). `SWITCH_SCALE` disparaît — la variante est native à
  36 px —, les switches passent à 74 et 112 px, tout le reste de la mise en page est dérivé et ne
  bouge pas. Quatre allers-retours sur rendu ont ensuite réglé le détail : taille du glyphe ramenée
  au plafond commun (elle avait grossi de 68 %), socles collés, survol qui allume le glyphe, et le
  liseré `#126068` de la case choisie avec son halo.
