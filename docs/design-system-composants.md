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

**Textures** : `checkbox-true.png` et `checkbox-false.png`, 20 × 20 toutes les deux — la taille
relevée exactement. Leur découpage 9-slice (5px figés) n'existe que pour honorer la règle « toute
taille est valide » : en pratique une case est toujours peinte à sa taille native.

**Inventé, faute de capture** : l'état **survolé** ne change rien (seul le curseur change), et
l'état **désactivé** teinte la case et le libellé de `TEXT_DISABLED`, par cohérence avec le bouton
désactivé.

**Pas encore utilisé** : la modale Options n'a aujourd'hui aucun réglage booléen. Le composant est
livré prêt ; son premier usage viendra avec le contenu de l'onglet « Paramètres ».

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
| Rayon | 2 | comme tout le reste |
| Chevron | 14 × 8px, à 8px du bord droit | **la taille native d'`icons/icon-chevron-down.png`**, au pixel |
| Retrait du libellé de socle | 10px | texte à x=17 pour un socle à x=7 |
| Hauteur d'une entrée | 28px | surbrillance en y 71..98, entrée suivante à y=99 |
| Retrait du texte d'une entrée | 12px | « Tous » à x=19 pour une liste à x=7 |
| Fond de liste | `#675d46` | uniforme, aucun dégradé |
| Entrée mise en avant | `#a58e63` | seul fond de mise en avant relevé |

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
| `auto_shrink` | laisse la zone se rétrécir à son contenu | `false` — un panneau du jeu occupe toute sa hauteur |

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
| `context` | `FirstPlan` (par-dessus le jeu), `Panel` (dans un panneau) | `FirstPlan` |
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
| `log_name` | préfixe des deux boutons dans le journal | `fenetre` |

Rend un `WindowChrome` : `tab_bar` (la bande d'onglets), `content` (entre onglets et pied) et
`footer` (le clic reçu). **`content` n'est pas écrêtée** — c'est le prix de la forme « zone rendue »,
et la raison pour laquelle le contenu passe normalement par `design::panel`.

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
à gauche du retrait des titres, sans quoi un titre de section perd sa première lettre —, et la mise à
zéro de l'espacement implicite d'egui.

**Utilisé en production** : la modale Options et les maquettes de la page Alertes.

---

## `design::heading` — titre de section (2026-09-10)

`crates/overlay-ui/src/design/components/heading.rs` — composant **feuille**.

```rust
ui.add(design::heading("Fichier"));
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `text` (à la construction) | le titre | — |
| `trailing_gap` | écart réservé sous le titre | `tokens::HEADING_TO_ROW` (7) |

Serif grasse au corps du titre de fenêtre (21), **gris `#b8b9ba` et non blanc** : la hiérarchie
entre les deux niveaux de titre du jeu passe par la couleur, pas par le corps.

Deux pièges que le composant absorbe : il réserve la hauteur d'**encre** (16) et non celle de sa
galley — réserver la galley ajoutait ~14 px invisibles sous le titre, sur sept relevés — et il
applique lui-même le **retrait de 7 px** qui est, dans le jeu, le seul signal qu'une section existe.

**Utilisé en production** : la modale Options et les maquettes de la page Alertes.

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
design::tooltip(&response).show(|ui| { /* contenu libre */ });
```

| Paramètre | Valeurs | Défaut |
| --- | --- | --- |
| `side` | `Above` / `Left` / `Right` | `Above` |
| `gap` | écart au widget | `TOOLTIP_GAP` = 5 px |
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

### Un écart au contrat, assumé et daté

**La police reste celle du thème**, là où le contrat veut `design::text::label_font`. La corriger est
un changement visuel : elle déplacerait les six captures d'infobulle de la suite de rendu, que le
plan demande justement **inchangées** pour prouver que cette migration ne change rien. Les deux ne
peuvent pas tenir dans le même commit — la police attend sa propre décision.

**Pas d'entrée de galerie** : une infobulle a besoin d'un survol, qui n'existe pas en rendu
offscreen statique. Sa vérification visuelle est ailleurs et existait déjà — les six tests dédiés
`watchlist_tooltip_*` et `combat_tooltip_*`, qui simulent le pointeur. Ils sont restés **identiques
au pixel** à travers la migration, ce qui est le critère de fin que le plan fixait.

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
| **`design::item_slot`** | `watchlist::entry_tile` — bordure de rareté, icône, compteur incrusté, et l'ordre de peinture dont l'inversion a déjà produit un bug. | 7 `Border-*.webp`, `rarity_borders` (7 raretés), `item_slot_square` 63–64 / `gap` 2 / `border` 2 relevés. |
| **`design::badge`** | `watchlist::paint_count_inline`, les étiquettes de rareté, la pastille d'état. | `status_pill_active` ; `text::OUTLINE_FULL` existe. §5.12, §5.13. |
| **`design::meter`** | `combat::damage_bar` — 68 lignes de rectangles empilés (bord externe, bord interne, piste, remplissage, reflet, curseur de fin, arrondis conditionnels). | Six couleurs mesurées dans `combat.rs`, à promouvoir en jetons. |
| **`design::portrait`** | `combat::paint_flat_portrait`, `panels::combat_frame` et son gabarit à six emplacements. | `crates/overlay-ui/assets/templates/*.png`, atlas de classes, portrait de repli. |
| **`design::table` + `design::pagination`** | Rien aujourd'hui — mais l'historique HDV, les ventes et les échanges (§9 du plan) sont exactement cela : colonnes triables, lignes alternées, état vide, « Page 0 / 0 » et ses deux flèches. | Quatre captures complètes dans `interfaces/` ; passer par `ui-blueprint` d'abord. |

### Vague 4 — les finitions

Ni urgent ni structurant, mais chacun retire du code d'un panneau.

| Composant | Ce qu'il absorbe | Matière disponible |
| --- | --- | --- |
| **`design::toolbar`** | Le carré de contrôle du Suivi : fond translucide, gouttière, groupement de boutons icône. | `watchlist::PANEL_BACKDROP_FILL`, `menu-button-icon-first-plan.png` |
| **`design::segmented`** | Le bascule « Objets mis en vente / Offres d'achat » ; le switch Alliés/Ennemis du panneau Combat en est une variante maison. | `tabs-with-first-tab-active.png` |
| **`design::toast`** | `watchlist::toast_card` et ses confettis — ~300 lignes, avec son générateur pseudo-aléatoire maison. | Portage du web ; aucun asset de jeu correspondant. |
| **`design::dialog`** | Rien — la boîte de confirmation qui manquera à la première action destructrice de l'overlay. | `interfaces/interface-confirm-box.png` |

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
- **Cinq chemins de chargement de texture** (et non six : `options_modal` est passé au manifeste) —
  `DesignSystem::load`, `ui_icons`, `portraits`, `remote_icons`, `combat_frame` — dont quatre copies
  de la même fonction décoder → `ColorImage` → `load_texture`. Le budget mémoire (§8 du plan,
  300 Mo) n'a donc toujours aucun point de mesure unique. C'est l'écart qui reste entier.

### Ce que ce catalogue devrait porter en plus

Une colonne **« qui le consomme en production »** par composant. La fiche du bouton icône la porte
déjà (« les quatre boutons du carré de contrôle du Suivi ») ; les autres non. C'est cette colonne
qui répond, dans six mois, à « puis-je changer ce jeton sans rien casser ? ».
