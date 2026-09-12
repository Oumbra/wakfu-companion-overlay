| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |# Catalogue des composants — `overlay_ui::design`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Ce que l'overlay sait déjà composer, et avec quels paramètres. **À lire avant d'écrire le moindre
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |widget** : si un composant existe, on l'étend, on n'en écrit pas un second (skill
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`.claude/skills/ui-component/`, étape 1).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- Le langage visuel et les mesures brutes sont dans [`design-system.md`](design-system.md) ; les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  couleurs dans [`design-tokens.json`](design-tokens.json).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- Le contrat que respecte tout composant :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  [`.claude/skills/ui-component/references/contrat-composant.md`](../.claude/skills/ui-component/references/contrat-composant.md).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- La planche de contrôle : `crates/overlay-testkit/tests/snapshots/design_gallery.png`, régénérée
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  par `UPDATE_SNAPSHOTS=1 cargo test -p overlay-testkit --test design_gallery`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- L'ordre de construction de ce qui manque, avec ses critères de fin :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  [`plan-composants-ui.md`](plan-composants-ui.md). Ce catalogue dit *ce qui existe*, ce plan dit
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  *dans quel ordre construire la suite*.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## Bouton texte — `design::button` (2026-09-09)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/button.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, ButtonSize, ButtonVariant};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |if ui.add(design::button("Valider").variant(ButtonVariant::Primary)).clicked() { … }
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    design::button("Annuler")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .variant(ButtonVariant::Danger)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .size(ButtonSize::Compact)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .width(338.0)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .log_name("options.annuler"),
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `variant` | `Primary` (or), `Secondary` (gris-brun), `Danger` (rouge) | `Secondary` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `size` | `Standard` (52px), `Compact` (36px), `Height(f32)` | `Standard` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `width` / `min_width` | largeur imposée / plancher | largeur = libellé + marges |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `enabled` | `bool` | `true` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tooltip` | texte d'infobulle | aucune |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `log_name` | nom d'instance pour le journal | le libellé |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `preview_state` | `Idle` / `Hovered` / `Disabled` — **galerie et captures uniquement** | état réel |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Variante = intention, pas couleur.** `Danger` (rouge) est réservé au pattern « bouton pleine
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |largeur du pied de page d'une modale » (§5.2 du design-system) ; l'« Annuler » d'une simple boîte de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |dialogue reste `Secondary`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**États** : repos / survolé / désactivé. Pas d'état « pressé » — l'appui retire l'apparence
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |survolée, elle revient au relâchement (même règle que les boutons icône).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Textures** (manifeste `design/assets.rs`, fichiers de `assets/design-system/`) :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`button-{primary,secondary,danger}.png` et leurs `-hover`, plus `button-disabled.png` **partagée par
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |les trois variantes** (§5.1 : l'état désactivé est une désaturation complète, une seule capture
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |existe).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Une variante peut avoir plusieurs textures, choisies par la HAUTEUR.** `Primary` en a deux :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`button-primary.png` (200×52, bouton de fenêtre HDV) et `button-primary-compact.png` (338×36, pied
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |de page de modale, générifiée depuis `large-button-validate.png`). Ce n'est pas une redondance qu'un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |étirement absorberait : l'embout décoratif mesure 52px sur la première et 34px sur la seconde.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Rendre un bouton de pied de page avec la texture de fenêtre lui donne un embout une fois et demie
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |trop large — invisible seul, criant à côté du bouton rouge voisin. Le composant retient la texture
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |dont la **hauteur native est la plus proche** de la hauteur demandée.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Découpage 9-slice** : les hachures diagonales ne sont pas une texture de fond mais un **embout**
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |d'extrémité — mesuré (`component.py insets`) à 43–51px des bords sur les textures 200×52 / 169×52,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |29–34px sur les 338×36, le centre étant un dégradé lisse. Marges figées : **52px** à gauche et à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |droite sur la famille 52px, **36px** sur la famille 36px, **6px** en haut et en bas (arrondi 3–4px +
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |liseré 2px), étirement sur les deux axes. Un bouton de 500px porte donc exactement deux embouts,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |comme un bouton de 200px.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Police du libellé** : egui n'embarque qu'une Ubuntu Light et n'expose aucun réglage de graisse,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |mais rien n'oblige à s'en contenter — `design::fonts` embarque `assets/fonts/Ubuntu-Medium.ttf` et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'enregistre comme famille nommée `ds-label`, que `design::text::label_font` sert aux composants.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |La proportionnelle par défaut d'egui reste celle du reste de l'interface : seuls les libellés du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design system changent.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |La première version fabriquait une graisse **synthétique** (galley repeinte sur ses huit voisins
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |immédiats à opacité réduite). Retirée le 2026-09-09 : un halo est un contour, pas une graisse — il
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |épaissit le mot en dégradant son contraste, le libellé devient flou là où celui du jeu est net.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Une leçon de cet épisode survit néanmoins à la rustine, parce qu'elle se redécouvre douloureusement :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**egui arrondit la position d'un texte au pixel entier** (`Options::round_text_to_pixels`), donc
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |aucun effet visuel ne peut être réglé par un déplacement fractionnaire de texte.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Graisse du libellé** : le jeu en utilise **deux**, et la variante décide laquelle.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`design::fonts` embarque Ubuntu Regular (`ds-label`) et Ubuntu Medium (`ds-label-strong`) ;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`ButtonVariant::label_strong` sert la seconde à `Primary` et `Danger`, la première à `Secondary`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |La preuve tient dans une seule capture, `interface-options-interface.png` : ses quatre boutons de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |contenu et ses deux boutons de pied de page ont **exactement la même hauteur d'encre — 13px — et pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la même graisse**. Fût moyen 1,79 à 1,92px pour le contenu contre 2,02px pour « Annuler », soit un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rapport de 0,82 en encre par colonne ; Regular/Medium rendent 0,79 dans les mêmes conditions,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Light/Medium 0,69.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Deux pièges dans cette mesure. **Seul le rapport interne à une capture veut dire quelque chose** :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |comparer un fût mesuré sur fond kaki à un rendu blanc sur noir donne des chiffres qui ne se
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |correspondent pas, la normalisation et le seuillage ne coupant pas au même endroit. Et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**« Valider » n'est pas une troisième graisse** malgré son fût de 3,17px : c'est le seul texte
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sombre sur fond clair de l'interface, et la capture y montre une ombre cuite.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Que la graisse suive la variante est une **corrélation observée**, pas une loi : `Primary`/`Danger`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sont les deux intentions du pied de page de modale, `Secondary` est le contenu. Si un `Primary`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |maigre apparaît un jour dans un panneau de contenu, c'est `label_strong` qu'il faudra ouvrir à un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |réglage explicite.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Police d'un titre** : le jeu n'utilise **pas une seule police**. Ses libellés de bouton sont dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |une linéale, mais ses titres — « Options » sur la bannière de modale, « Barres de raccourcis » en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |titre de section — sont dans une **serif grasse**. `design::fonts` embarque donc aussi
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`assets/fonts/PTSerif-Bold.ttf` sous la famille nommée `ds-title`, servie par
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`design::text::title_font`. Vingt-six serifs ont été comparées aux deux échantillons du jeu ;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |PT Serif Bold est la meilleure libre (~2 % d'écart de chasse sur « Options »).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Cerne d'un texte** : `design::text::paint_outlined_text` repeint le texte décalé avant de le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |peindre plein, et prend sa liste de décalages en paramètre. Deux jeux nommés, et le choix entre eux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |n'est pas esthétique — il dépend de ce qu'il y a **derrière** le texte :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Jeu de décalages | Quand | Pourquoi |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `OUTLINE_FULL` | texte flottant nu par-dessus le jeu (Combat, Suivi) | le fond est **arbitraire** : une ombre d'un seul côté devient illisible dès que ce fond est clair de ce côté-là |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `SHADOW_BOTTOM_RIGHT` | texte posé sur un fond **connu** (bannière, encadré de section) | trois décalages au lieu de huit ; un contour complet empâte le mot. Le jeu éclaire ses titres depuis le haut-gauche (masse sombre mesurée à +2,6px en x, +1,8px en y) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Les décalages sont des pixels **entiers** — voir ci-dessus, egui arrondit la position d'un texte au
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pixel.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Hauteur d'un bouton** : celle de sa texture, jamais déduite de sa largeur. Un 9-slice existe pour
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |qu'on l'étire en largeur *sans* toucher à sa hauteur ; le jeu affiche son bouton de pied de page à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |36px quelle que soit la fenêtre (son bandeau de titre fait 56px, chez lui comme chez nous). La
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |modale Options déduisait la sienne du rapport d'aspect de la texture et tombait à 27px — le libellé
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |suivant la hauteur, il y perdait 3px d'encre sur 13.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Mesures** :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur | Origine |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Hauteurs natives | 52px (HDV), 36px (pied de page de modale) | taille des textures |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Corps de police | `17/36 × hauteur` | balayage rendu dans egui et comparé au pixel aux libellés gravés du jeu ; 17px est le seul corps qui retrouve les 13px d'encre de « Annuler » ET de « Valider », à hauteur de bouton native |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Police du libellé | Ubuntu Medium | onze candidats comparés à la référence : Medium 17 est le seul au-dessus de 0,50 de recouvrement de forme sur les DEUX libellés (0,52/0,52 contre 0,59/0,32 pour Regular). Le jeu n'est pas cohérent entre ses deux boutons, aucune police ne colle aux deux |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Police d'un titre | PT Serif Bold | vingt-six serifs comparées aux deux titres du jeu ; meilleure correspondance libre. Corps 21 sur la bannière (choix utilisateur : le 22 colle exactement à l'encre du jeu, 21 × 82, le 21 rend 20 × 78 et est jugé mieux proportionné) et 18 en titre de section, déduit du rapport d'encre du jeu entre ses deux niveaux de titre (17px contre 21px) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Graisse du libellé | Regular (contenu) / Medium (pied de page) | même hauteur d'encre 13px dans les deux cas sur `interface-options-interface.png`, mais fût 1,79–1,92px contre 2,02px — rapport 0,82, quand Regular/Medium donne 0,79 et Light/Medium 0,69 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Marge horizontale | `0,55 × hauteur` | ordre de grandeur de §4 du design-system — **estimation** |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Largeur minimale | `2,5 × hauteur` | garde-fou de proportion — **réglage**, pas une mesure |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Remplace** : les assets taillés sur mesure `assets/design-system/large-button-cancel.png` /
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`large-button-validate.png` — un PNG par taille **et** par libellé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Migration faite (2026-09-09).** Les trois boutons de `panels::options_modal` — « Annuler »,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |« Valider », « Sélectionner le fichier » — sont des appels à `design::button`. Ont disparu avec eux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |les quatre PNG de `crates/overlay-ui/assets/ui/options/` (copies octet pour octet d'assets déjà au
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |manifeste) et le module `panels::nine_slice`, seconde implémentation du 9-slice (marge unique,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |étirement seul, pas de répétition). Le gain visible est sur « Sélectionner le fichier » : rendu à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |700px depuis une texture de 169px, son ancien 9-slice ne figeait que 14px de chaque côté — les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |croisillons tombaient dans la bande médiane et s'y étiraient sur près de 200px.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::input` — champ de saisie
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**API** : `design::input(&mut valeur)`, plus `.placeholder(...)`, `.width(...)`, `.size(...)`,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`.enabled(...)`, `.error(...)`, `.read_only(...)`, `.leading_icon(...)`, `.tooltip(...)`,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`.log_name(...)`. Rend une `Response` : `changed()` dit à quelle frame la valeur a bougé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**La valeur vit chez l'appelant.** Le composant l'écrit, l'appelant la relit — c'est un champ de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |formulaire, pas un état interne. **Vide = le texte indicatif s'affiche** : une `String` vide *est*
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'absence de valeur pour un champ texte, une `Option` n'ajouterait aucune information et obligerait
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |chaque appelant à trancher entre `None` et `Some("")`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Largeur par défaut : toute la place disponible** — l'inverse du bouton, qui se cale sur son
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |libellé. Un champ de saisie n'a pas de contenu au moment où on le place ; sa largeur ne peut venir
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |que de la mise en page.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Mesures** (barre de recherche de l'onglet Commandes, x 30..500 × y 138..163, recoupée avec les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |assets isolés via `dsimg.py analyze`) :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur | Origine |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Hauteur native | 25px | capture ET `large-input-text-width-placeholder.png` (composant 258 × 25) — deux sources indépendantes d'accord |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Bord | 2px `#595140` | deux lignes et deux colonnes pleines sur la capture ; même couleur sur les quatre assets de champ |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Rayon | 4 | `dsimg.py analyze`, IoU 1,0 sur deux assets — **le seul composant du jeu qui ne soit pas à 2** |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Fond | `#0e1115` | capture |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Retrait du texte | 6px du bord extérieur | premier glyphe de « Rechercher » à x=36 pour un champ à x=30 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Corps | 17px (encre 13) | même encre qu'un libellé de bouton, donc même corps |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Valeur saisie | **`#f4d89e`, or** | pic identique sur `input-search.png`, `input-number.png` et `large-input-number.png` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Texte indicatif | `#83775b`, kaki éteint | pic sur « Rechercher » et « Min » |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Une valeur saisie est or, pas blanche** — le constat le plus contre-intuitif du relevé, et il
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |tient sur trois assets indépendants. Le texte indicatif est peint à la main plutôt que confié à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`TextEdit::hint_text` : egui écrase la couleur d'un `hint_text` par `Visuals::weak_text_color()`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Un champ fait 25px là où un bouton en fait 36**, et ce n'est pas une incohérence à corriger : le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |jeu compose réellement des lignes où le champ est plus bas que ce qui l'accompagne. C'est à la mise
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |en page de centrer le plus petit.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Inventé, faute de capture** — signalé pour ne pas être pris plus tard pour une mesure : l'état
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**survolé** ne change rien (inventer un éclaircissement serait inventer du design), et l'état
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**désactivé** reprend les jetons du bouton désactivé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**L'état d'erreur** (2026-09-11) est le quatrième, et il est propre au champ : les trois états du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design system décrivent ce que l'interface *permet*, or une valeur peut être refusée alors que le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |champ reste parfaitement actif. `Error` n'est donc pas une nuance de `Disabled` mais son contraire —
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |il faut justement revenir dans le champ.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Seul le bord change** (`INPUT_BORDER_ERROR`). La valeur reste or : la teindre en rouge la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  donnerait à lire comme un message plutôt que comme une saisie, et lui ferait perdre le contraste
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  voulu sur le fond très sombre.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Le rouge est celui d'`INFO_ALERT`**, délibérément par alias et non par seconde valeur : le champ
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  et son message sont un seul signal en deux endroits, deux rouges voisins se liraient comme deux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  alertes. Le jour où le jeu fournira une capture, c'est ce jeton qui prendra la mesure et se
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  détachera de lui-même.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Le composant ne valide rien** — il ne sait pas ce qu'est un chemin correct. La validation
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  appartient à l'appelant, qui la possède déjà, et le champ la reflète.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **`Disabled` l'emporte sur `Error`** : on ne corrige pas ce qu'on ne peut pas éditer.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Consommé en production par `panels::options_modal`, dont le champ de chemin porte désormais
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'alerte en même temps que son message — celui-ci est sous le bouton « Parcourir », hors du regard
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |de qui vient de taper.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Remplace** : le champ repeint à la main dans `panels::options_modal`, dont les trois constantes
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |locales étaient toutes fausses (fond `#1C1E23`, bord 1px, rayon 2, valeur blanche).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### La barre de recherche : `InputSize::Search` et `clearable` (2026-09-12)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Retour utilisateur sur le champ d'ajout d'alerte, avec les deux assets détourés du jeu à l'appui
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(`empty-input-search.png`, `input-search.png`, 341 × 32) : « la loupe n'est pas dans le bon sens et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |n'est pas colorée comme sur la maquette », « l'input est un tout petit peu trop petit en hauteur, ou
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |alors c'est la police qui est trop grande », et une croix d'effacement manquante. Les trois ont été
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |mesurés sur ces assets, pas ajustés à l'œil :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur | Origine |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Hauteur de la boîte | **28px** (`INPUT_SEARCH_HEIGHT`) | y 2..29 inclus sur les deux assets, identiques au pixel — ce n'est PAS le champ de 25 px de l'onglet Commandes |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Encre du texte | 12px de capitale (« R » y 11..22) | la même encre qu'un champ standard (« A » de 12 px dans 25) : **la boîte est plus haute, le corps ne change pas** — 17 px pour les deux gabarits |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Loupe | manche **en bas à gauche**, teinte `#a69064` (`INPUT_ICON`) | pic dominant de la loupe, champ vide ou rempli — plus chaude et plus claire que le kaki du texte indicatif qu'elle portait ; le glyphe du manifeste a son manche à droite, il est peint **en miroir** (`paint_icon_flipped`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Croix | encre 11 × 12 (x 320..330, y 10..21), `#675d46` (`INPUT_CLEAR_ICON`), à 9px du bord droit | `input-search.png` seulement : **absente du champ vide** |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Croix survolée | `INPUT_ICON` | **inventé**, aucune capture — la teinte de la loupe, pour ne pas ajouter de couleur |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`InputSize::Search` porte la hauteur ; les trois ratios d'ornement (7/28, 13/28, 8/28), mesurés dès
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'origine sur cette capture de 28 px, y redonnent exactement 7, 13 et 8 px. `Input::clearable(true)`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pose la croix : **la place est réservée dès qu'elle est possible**, valeur ou pas, sinon le texte
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |se décalerait au premier caractère tapé ; un clic vide la valeur, marque la réponse `changed()` et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**rend le focus au champ** — l'appui sur la croix le lui avait retiré, or on efface pour retaper.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Sans effet sur un champ désactivé ou en lecture seule. Test réel du geste dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`options_alertes_croix_efface_la_saisie` (survol, appui, relâchement, puis une frappe qui doit
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |retomber dans le champ).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Consommé par `design::autocomplete` (champ d'ajout d'alerte) et par le champ de chemin de la modale
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Options, qui garde son gabarit de 25 px mais gagne la croix.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::info_text` — texte d'information (2026-09-10)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/info_text.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, InfoTone};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(design::info_text("Le thème sera appliqué au prochain démarrage."));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    design::info_text("Le fichier sélectionné doit s'appeler wakfu.log.")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .tone(InfoTone::Alert)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .width(inner_rect.width())
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .log_name("options-erreur"),
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tone` | `Info` (pastille dorée, texte blanc), `Alert` (tout en rouge) | `Info` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `width` | largeur imposée | toute la place disponible |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tooltip` | texte d'infobulle | aucune |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `log_name` | nom d'instance pour le journal | les quatre premiers mots |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Mesures** (nœud `info` de [`releve-options-interface.json`](design-system/releve-options-interface.json)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |— le bloc en bas de l'onglet Interface, **le seul de toute la fenêtre Options du jeu**) :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur | Origine |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Pastille | 12 × 12px | boîte `[38, 438, 50, 450]` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Position de la pastille | centrée sur la **boîte de police** de la première ligne | pastille `[438, 450]`, hauteur d'x de la ligne 1 `[441, 449]` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Écart pastille → texte | 7px | la pastille finit à x=50, le texte commence à x=57 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Lignes suivantes | alignées sur le **texte** | seconde ligne à x=57 elle aussi |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Interligne | 23px | haut d'encre à haut d'encre (y=436 puis y=459) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Corps | 17px (encre 13) | le même qu'un libellé de bouton |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Graisse | regular | pas la Medium du pied de page |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Texte | **`#ffffff`, blanc pur** | pic mesuré |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Pastille | `#a69064` | jeton `info-dot` du relevé |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le texte d'information est blanc pur, pas gris**, et c'est le piège de ce composant. Note du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |relevé : « l'impression de gris vient du fond et de l'absence de graisse, pas de la couleur. Un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |portage qui le grise s'écarte de la référence. »
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**La pastille est centrée sur la première ligne, pas sur le bloc** — sur un message de deux lignes,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |un centrage sur le bloc la descendrait de 11px. Le retour à la ligne, lui, s'aligne sur le texte :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la pastille reste seule dans sa colonne.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Et sur la boîte de police de cette ligne, pas sur son interligne.** Le piège se cache dans le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`line_height` imposé ci-dessous : l'interligne mesuré (23px) dépasse la hauteur naturelle de la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |police (19,5px à ce corps), et `epaint` ne répartit pas cette différence de part et d'autre du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |texte — il cale la ligne de base sur l'ascendante et laisse les ~3,5px de rabiot **sous la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |descendante**. Diviser les 23px en deux vise donc un axe qui n'est pas celui du texte : la pastille
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |descend de 2px et vient se poser sur la ligne de base. L'axe juste est
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`ui.fonts_mut(|f| f.row_height(&font)) / 2.0`, la moitié de la hauteur naturelle.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le contrôle se fait sur la **hauteur d'x** et non sur l'encre entière, qui dépend des accents et des
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |jambages présents dans la phrase : dans le jeu la pastille commence 3px au-dessus du sommet de la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |hauteur d'x (438 contre 441) et finit sur sa dernière ligne d'encre (449) ; le rendu fait 32..43
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |contre 35..43, soit la même position au pixel près.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**L'interligne est une mesure, pas la hauteur naturelle de la police.** Sans le `line_height` imposé
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |à 23px, egui empile les lignes à ~20px et le bloc se resserre par rapport au jeu.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Inventé, faute de capture** : le ton **`Alert`**. Le jeu n'a aucune variante d'alerte relevée —
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |seule sa *composition* est une extension ; sa couleur est `accent_danger.top` (`#c9524a`), le haut
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |du bouton « Annuler », le seul rouge que le design system ait mesuré.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Deux écarts au contrat de composant, assumés** :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Pas de trois états, pas de `preview_state`.** Ni survol, ni désactivé, ni clic : ce n'est pas un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  contrôle, et inventer un survol ici serait inventer du design. Il garde `Sense::hover()` pour
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  pouvoir porter une infobulle.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Pas de 9-slice.** `DsTexture::IconInfo` (`icons/icon-info.png`, 27 × 28) est la première icône du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  manifeste et la seule texture à marges nulles : figer des coins sur un glyphe de 27px le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  déformerait dès qu'on le peint à 12, et il n'y a rien de périodique à répéter. Elle passe malgré
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  tout par `DesignSystem::paint`, pour garder un seul chemin de peinture. Blanche dans le fichier,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  elle prend sa couleur par teinte — c'est ce qui permet au ton `Alert` d'exister sans second
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  fichier.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Remplace** : le `ui.label(RichText::new(err).color(ERROR_TEXT).size(13.0))` de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`panels::options_modal` — le dernier texte de la modale à échapper au design system.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::tabs` — barre d'onglets (2026-09-10)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/tabs.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, TabState};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design::tabs(&mut state.tab)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .entry(OptionsTab::Alertes, "Alertes")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .enabled(false)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .entry(OptionsTab::Personnages, "Personnages")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .enabled(false)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .entry(OptionsTab::Parametres, "Paramètres")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .log_name("options-onglets")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .show(ui);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `entry(valeur, libellé)` | une entrée, dans l'ordre d'affichage | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `enabled` | **s'applique à la dernière entrée déclarée** | `true` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `preview_state` | `Idle` / `Hovered` / `Active` / `Disabled`, sur la dernière entrée — **galerie et captures uniquement** | état réel |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `log_name` | nom d'instance pour le journal | `"tabs"` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |La valeur sélectionnée vit chez l'appelant, comme celle de `design::input` ; `Response::changed()`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |dit à quelle frame elle a bougé. `show(ui)` est un alias d'`ui.add(...)`, plus lisible quand le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |composant porte une liste d'entrées chaînées.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Quatre états, pas trois** — un onglet porte en plus la notion d'être *celui qui est sélectionné* :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || État | Fond | Libellé |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Inactif | sombre `#363734` | doré `#f4d89e` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Survolé | **kaki, celui de l'actif** | doré `#f4d89e` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Actif | kaki `#625a47` | **blanc `#ffffff`** |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Désactivé | sombre | `TEXT_DISABLED` — inventé |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le piège de ce composant**, énoncé tel quel par le relevé : « l'état survolé d'un onglet reprend
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |exactement le fond de l'état actif ; la seule différence relevée est la couleur du libellé. Un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |portage qui ne distingue que par le fond rendrait les deux états indiscernables. » Deux textures
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |suffisent donc pour quatre états.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Mesures** (`releve-modale-options.json`, nœuds `tabbar` et `tab-*`, recoupées au pixel sur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`interface-options-video.png` ligne y=110) :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur | Origine |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Hauteur | **44px**, native | `[72, 116]` dans une bande `[56, 124]` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Séquence entre deux onglets | bord 2px + séparateur 2px + bord 2px | identique sur la capture et sur l'asset |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Hauteur du séparateur | **40px, pas 44** | y 2..41 : le corps de l'onglet, entre ses deux bords sombres |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Séparateur | **dégradé vertical `#837d70` → `#595140`** | profil de la colonne x=261 de `tabs-with-first-tab-active.png` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Profil du dégradé | 11px clair, 19px de rampe, 10px sombre | idem |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Corps du libellé | 17px (encre 13) | comme tous les libellés du jeu |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le libellé est cerné sur 1px dans les huit directions, d'une version assombrie de sa propre
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |couleur** (`TAB_LABEL_OUTLINE_FACTOR` = 0,205). Deux couleurs de libellé de la même capture le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |confirment et écartent les autres lectures : doré `#f4d89e` → cerne `#312c21`, blanc `#ffffff` →
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |cerne `#353534`. Un cerne noir translucide donnerait une couleur proportionnelle au *fond*, or le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |fond de l'onglet actif est un kaki chaud et son cerne est gris neutre ; un cerne noir opaque
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |donnerait du noir. Ce n'est donc ni le cerne noir de `design::text::paint_outlined_text` (titre de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |modale, dégâts de combat), ni la graisse synthétique retirée en 2026-09-09 — huit copies de la même
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |couleur empâtent, huit copies plus sombres détourent. Les boutons du jeu n'ont pas ce cerne (anneau à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |4 % du fond sur les deux captures de pied de page).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Les trois couleurs relevées pour le séparateur sont un seul dégradé.** `#837d70` (relevé),
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`#6d6657` (asset détouré) et `#595140` (capture de la modale, ligne y=110) ont longtemps semblé se
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |contredire, et le jeton tranchait pour la troisième. Aucune n'est fausse : ce sont trois hauteurs du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |même trait. egui ne remplissant pas un rectangle en dégradé, le trait est peint en maillage — deux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |plateaux constants et une rampe interpolée par le GPU, exact à n'importe quelle hauteur de barre.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Les 6px de gouttière du relevé sont ceux du remplissage**, pas de la boîte : chaque onglet porte
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ses deux bords de 2px, le composant les pose donc à 2px l'un de l'autre et peint le séparateur dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |cet intervalle.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le rayon n'est pas sur l'onglet, il est sur la barre.** Le premier segment de l'asset a ses deux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |coins gauches arrondis, le dernier ses deux coins droits ; les segments du milieu sont parfaitement
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |droits. L'escalier d'alpha retire `4, 2, 1` pixels sur les trois premières lignes et `1, 2, 3` sur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |les trois dernières — le haut est creusé d'un pixel de plus que le bas, sur les deux côtés, de façon
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |cohérente aux quatre angles : c'est la forme du jeu, pas du bruit de détourage, et elle est conservée
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |telle quelle.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**L'arrondi est porté par l'alpha de la texture**, comme celui d'un bouton, d'où quatre fichiers
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |d'extrémité en plus des deux du milieu. Les deux autres voies n'en sont pas : un `Mesh` egui ne sait
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pas découper un coin, et peindre un patch arrondi par-dessus n'efface pas le coin carré du dessous —
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'alpha compose, il ne soustrait pas.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Fichier | Origine | Coins arrondis |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tab-active-first.png` | 1er segment de la capture, **coin gardé** (un pixel opaque isolé retiré de l'angle bas-gauche) | gauche |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tab-active-last.png` | miroir horizontal du précédent | droite |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tab-inactive-last.png` | dernier segment de la capture | droite |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tab-inactive-first.png` | miroir horizontal du précédent | gauche |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tab-active.png` / `tab-inactive.png` | 1er segment coin redressé / segment du milieu | aucun (onglets du milieu) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le jeu n'a pas de capture d'onglet actif en fin de barre, d'où deux miroirs — le corps d'un onglet
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |étant un dégradé **vertical** et ses deux bords identiques, le miroir ne change rien. Une barre à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**un seul onglet** n'existe pas dans le jeu : aucune texture n'a ses quatre coins arrondis, le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |composant y superpose ses deux extrémités, chacune écrêtée à sa moitié.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le cerne de la barre est continu, gouttières comprises** (`TAB_BORDER` = `#1a1d1f`, moyenne des 774
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pixels opaques de la ligne y=0 de l'asset). Il vient de la texture de chaque onglet sauf dans les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |gouttières de 2px, qui n'appartiennent à aucun onglet : sans peinture explicite, le fond du panneau y
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |traverse la barre de part en part, et le cerne se retrouve entaillé de deux encoches au droit de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |chaque séparateur.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Largeur : parts égales sur toute la largeur disponible, et c'est un choix, pas un relevé.** Le jeu
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |dimensionne chaque onglet sur son libellé — ses six onglets font 77, 83, 103, 83, 133 et 106px pour
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |des encres de 26, 44, 73, 28, 100 et 35 : ni un padding constant, ni le nombre de caractères, ni une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |largeur minimale unique n'en rendent compte. Sa barre ne remplit d'ailleurs pas la fenêtre (elle
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |s'arrête à x=636 sur 705) parce que le **bouton de réinitialisation** occupe la droite. Appliquer une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |règle qu'on n'a pas mesurée à trois onglets qui n'ont pas ce bouton laissait la barre à 337px sur les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |743 du panneau, calée à gauche. `fit_content()` rend l'autre comportement, pour comparer à une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |capture du jeu ou pour une barre qui ne doit pas s'étirer.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Inventé, faute de référence** :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Le padding de `fit_content()`.** Faute de règle retrouvable (ci-dessus) : le padding stable sur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  les deux libellés longs (`TAB_PADDING_X` = 16) et un plancher au plus petit onglet relevé
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  (`TAB_MIN_WIDTH` = 77). Les libellés longs tombent à 1 et 6px de la référence, les courts
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  ressortent plus étroits.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **L'état désactivé.** Aucune capture d'onglet grisé ; fond inactif et libellé `TEXT_DISABLED`, par
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  cohérence avec le bouton désactivé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Remplace** : la barre peinte à la main de `panels::options_modal` — texture `menu-tabs.png` de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |44px étirée à 31, libellés en `FontId::proportional(12.0)`, aucun clic. Elle partageait déjà la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |largeur en trois tiers égaux ; c'est le composant qui s'en était écarté, le temps de deux captures.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### La variante pictogramme (2026-09-11)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design::tabs(&mut vue)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .entry(Vue::Combat, "Combat").icon(DsIcon::Cards)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .entry(Vue::Suivi, "Suivi").icon(DsIcon::Trophy)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .show(ui);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`.icon(...)` s'applique à la **dernière entrée déclarée**, comme `.enabled(...)`. Le pictogramme
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**remplace le libellé au rendu** — et le libellé reste, ce qui n'est pas une commodité d'API :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- il devient l'**infobulle** de l'onglet (`design::tooltip`), sans quoi une barre de pictogrammes
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  n'apprend à personne ce que fait chaque onglet ;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- il reste la **ligne de journal**, sans quoi on ne saurait plus nommer ce qui a été cliqué.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |C'est aussi pourquoi cette variante attendait le lot 2 : elle a besoin de `DsIcon` pour le glyphe
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**et** de `design::tooltip` pour le mot.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Mesures** — `assets/design-system/icon-tabs.png` (268 × 44), un gabarit à quatre onglets :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Largeur d'un onglet | **66 px** (`TAB_ICON_WIDTH`) — crêtes de séparation à x=65, 133, 201, soit un pas de 68 dont 2 de gouttière |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Hauteur | 44 px, la même que la variante texte — les deux partagent leurs textures |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Encre du pictogramme | **dérivée**, `TAB_ICON_RATIO` = 18/36 → 22 px sur 44 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Un onglet à pictogramme est donc **plus étroit** qu'un onglet texte (77 px de plancher) : il n'a pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |de mot à contenir. En mode étiré (le défaut), il suit la même règle de parts égales que la variante
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |texte ; `fit_content` lui donne les 66 px du jeu.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le ratio d'encre est dérivé, pas mesuré**, et c'est dit dans le jeton : `icon-tabs.png` est un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |gabarit **vide** — le jeu n'y a laissé aucun pictogramme. La valeur reprend le rapport du bouton
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |icône (`ICON_BUTTON_CONTENT` sur `ICON_BUTTON_SIZE`), le seul rapport glyphe/socle que le design
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |system ait mesuré. À remplacer dès qu'une capture d'onglets à pictogrammes existera.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le pictogramme prend la teinte du libellé**, pas une teinte propre : il dit la même chose qu'un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |mot d'onglet, il doit changer avec l'état de la même façon — blanc quand l'onglet est actif, doré
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sinon, gris quand il est désactivé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Vérifié au rendu : les crêtes de séparation de la galerie tombent à un pas de 68 px, crête de 2 —
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**la cote du jeu au pixel**.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::checkbox` — case à cocher (2026-09-10)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/checkbox.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |if ui.add(design::checkbox(&mut state.alertes_sonores, "Alertes sonores")).changed() {
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    // la frame où la case vient d'être basculée
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |}
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `enabled` | `bool` | `true` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tooltip` | texte d'infobulle | aucune |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `log_name` | nom d'instance pour le journal | le libellé |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `preview_state` | `Idle` / `Hovered` / `Disabled` — **galerie et captures uniquement** | état réel |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le libellé porte l'état autant que la case** : blanc décoché, doré `#f4d89e` coché. C'est le même
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |principe que la barre d'onglets — le jeu ne se repose jamais sur un seul signal visuel. Un portage
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |qui ne changerait que la case perdrait la moitié de l'information.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Toute la ligne est cliquable**, case et libellé : c'est ce que fait le jeu, et viser un carré de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |20px à la souris est une punition.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Mesures** (`releve-section-options.json`, nœuds `cb1` à `cb3` et leurs libellés) :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur | Origine |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Case | 20 × 20px | `cb1` en `[36, 173, 56, 193]`, confirmé par les deux assets 20 × 20 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Rayon | **0** | **le seul élément carré de l'interface** — tout le reste est à 2, le champ à 4 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Case → libellé | 6px | la case finit à x=56, le libellé commence à x=62 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Corps du libellé | 15px (encre 10) | jeton `libellé d'option` — plus petit qu'un libellé de bouton |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Libellé décoché / coché | `#ffffff` / `#f4d89e` | relevé |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Textures** : `checkbox-true.png` et `checkbox-false.png`, 20 × 20 toutes les deux — la taille
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |relevée exactement. Leur découpage 9-slice (5px figés) n'existe que pour honorer la règle « toute
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |taille est valide » : en pratique une case est toujours peinte à sa taille native.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Inventé, faute de capture** : l'état **survolé** ne change rien (seul le curseur change), et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'état **désactivé** teinte la case et le libellé de `TEXT_DISABLED`, par cohérence avec le bouton
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |désactivé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Pas encore utilisé** : la modale Options n'a aujourd'hui aucun réglage booléen. Le composant est
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |livré prêt ; son premier usage viendra avec le contenu de l'onglet « Paramètres ».
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::select` — liste déroulante (2026-09-10)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/select.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    design::select(&mut state.theme)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .option(Theme::Sombre, "Sombre")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .option(Theme::Clair, "Clair")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .width(560.0),
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    design::select_multi(&mut state.raretes)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .option(Rarete::Commun, "Commun")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .option(Rarete::Rare, "Rare")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .summary("Toutes"),
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `option(valeur, libellé)` | une entrée, dans l'ordre d'affichage | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `width` | largeur imposée | toute la place disponible |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `placeholder` | libellé du socle si la valeur ne correspond à aucune option (choix simple) | vide |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `summary` | libellé du socle en choix multiple (« Toutes ») | « n sélectionnées » |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `enabled` | `bool` | `true` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `log_name` | nom d'instance pour le journal | `"select"` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `preview_state` / `preview_open` / `preview_hovered` | **galerie et captures uniquement** | état réel |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Un choix simple **referme** la liste au clic ; un choix multiple la garde ouverte, pour qu'on puisse
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |en cocher plusieurs. Chaque entrée du mode multiple porte la case de `design::checkbox`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**C'est le seul composant qui peint hors de son rectangle.** La liste dépliée vit dans une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`egui::Area` au premier plan : sans ça, le widget suivant la recouvrirait, et la place qu'elle occupe
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |décalerait la mise en page à chaque ouverture. L'état ouvert/fermé reste dans la mémoire d'egui —
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |c'est de l'état d'**interaction**, pas de l'applicatif que le contrat interdit, du même ordre que
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |« ce widget a le focus ». `egui::ComboBox` procède de même.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Mesures** (`select-simple.png` colonne x=60, `select-simple-opened.png` colonne x=100, recoupées
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |avec le nœud `select-theme` de `releve-options-interface.json`) :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur | Origine |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Hauteur du socle | **36px, bord compris** | bord 2 + liseré 2 + dégradé 28 + ombre 2 + bord 2 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Rayon | 2 | comme tout le reste |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Chevron | 14 × 8px, à 8px du bord droit | **la taille native d'`icons/icon-chevron-down.png`**, au pixel |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Retrait du libellé de socle | 10px | texte à x=17 pour un socle à x=7 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Hauteur d'une entrée | 28px | surbrillance en y 71..98, entrée suivante à y=99 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Retrait du texte d'une entrée | 12px | « Tous » à x=19 pour une liste à x=7 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Fond de liste | `#675d46` | uniforme, aucun dégradé |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Entrée mise en avant | `#a58e63` | seul fond de mise en avant relevé |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le piège de la hauteur**, énoncé par le relevé : « le chiffre de 32px qu'on lit en mesurant le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |remplissage est trompeur — il exclut les 2px de bord haut et bas. »
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Aucune largeur par défaut.** Le relevé mesure 210, 208, 560 et 650px selon le contrôle : « la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |largeur est décidée contrôle par contrôle ». Le composant prend donc la place disponible, comme
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`design::input`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Texture** : `select-face.png` (220 × 36), découpée de `select-simple.png` avec libellé et chevron
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |retirés — le socle étant un dégradé purement vertical, une colonne propre répétée le reconstruit
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |exactement, sans interpolation.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Inventé, faute de référence** :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **La distinction survolée / valeur courante.** Une seule capture montre une entrée sur fond clair,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  et elle est à la fois la valeur du socle et, probablement, celle que la souris survolait. Le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  composant applique le même fond aux deux.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Le gabarit du mode multiple.** `select-multiple.png` donne 26px d'entrée contre 28 pour le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  simple ; les deux captures ne sont pas à la même échelle d'interface. Les cotes du **simple** sont
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  appliquées dans les deux modes.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **L'état désactivé** : socle et libellé en `TEXT_DISABLED`, la liste ne s'ouvre pas.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::scroll_area` — zone défilable (2026-09-10)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/scroll_area.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design::scroll_area("options-contenu").show(ui, |ui| {
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    // le contenu qui peut déborder
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |});
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `id_salt` (à la construction) | distingue deux zones du même panneau ; porte la position de défilement | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `auto_shrink` | laisse la zone se rétrécir à son contenu | `false` — un panneau du jeu occupe toute sa hauteur |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Ce n'est pas un `Widget`**, et depuis le 2026-09-10 ce n'est plus un écart : c'est le premier
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**composant conteneur** du design system (§1 bis du contrat, *forme closure*). Il prend une closure
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |de contenu, il ne peut donc pas rendre une `Response` à partir de rien — `egui::ScrollArea` n'en est
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pas un non plus, pour la même raison.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Variante « cadre » — hors de ce composant, et c'est délibéré.** La barre du cadre ennemi du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |panneau Combat (`panels::combat_frame_scroll`) ne ressemble pas à celle-ci : barre dessinée de 5 px,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |couleur unie `#998a6c`, bordure noire, toujours visible. Ce n'est **pas** une divergence à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |« corriger » — chacun de ces écarts est une demande explicite de l'utilisateur, et les assets du jeu
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(`scrollbar-active.png` / `scrollbar-inactive.png`) y ont été essayés puis **rejetés** (« trop
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |large, cachait les portraits »), voir l'en-tête de module de ce fichier. Le contexte diffère du tout
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |au tout : une barre posée *sur* un décor de cadre, pas dans la gouttière d'un panneau. Elle reste
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |locale au panneau tant qu'elle n'a qu'un utilisateur ; si un second apparaît, elle remonte ici en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |paramètre plutôt qu'en second composant.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Aucun rail.** Le relevé est catégorique : « le fond du panneau tient lieu de gouttière ». C'est ce
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |qui rend ce composant particulier — `egui::ScrollArea` peint par défaut un rail derrière sa poignée,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |et `extreme_bg_color` est le seul jeton qu'elle consulte pour ce fond. Il est mis à transparent dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |un `scope`, pour ne pas fuir vers le reste du panneau.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Mesures** (`releve-modale-options.json`, nœuds `scrollbar-thumb` et `panel`, recoupés avec les deux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |assets ligne y=150) :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur | Origine |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Poignée | 6px de large | x 685..691 sur la modale, x 5..10 sur les assets |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Rayon | 3 | relevé |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Poignée au repos | `#515356` | relevé de la modale (l'asset donne `#5e5f62`, §5.9 `#5c5e61`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Poignée survolée / tirée | `#c1ad83` | `scrollbar-active.png` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Marge contenu → poignée | 6px | 679 → 685 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Marge poignée → bord | 14px | 691 → 705 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **Réserve totale** | **26px** | 679 → 705, « même quand la barre ne sert pas » |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**`design::components::scroll_area::RESERVE_X` vaut ces 26px**, et c'est la somme des trois marges.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Une mise en page peut donc réserver la place **avant** que la barre existe, sans qu'un pixel de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |contenu ne bouge le jour où elle apparaît — c'est ce que fait `panels::options_modal`, ce qui lève la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |déviation qu'il documentait (19px au lieu de 26, faute de barre à y mettre).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Non reproduit** : l'**ombre portée de 2px** à droite de la poignée. `egui::ScrollArea` peint sa
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |poignée elle-même et n'expose aucun point d'accroche pour l'ombrer ; la reproduire demanderait de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |recalculer sa position hors d'egui — un doublon fragile de son propre calcul, pour deux pixels
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sombres sur un fond déjà sombre.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::icon_button` — bouton icône (2026-09-10)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/icon_button.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, DsTexture, IconContext};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |if ui
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .add(design::icon_button(DsTexture::IconOption).context(IconContext::FirstPlan))
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .clicked()
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |{ … }
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `context` | `FirstPlan` (par-dessus le jeu), `Panel` (dans un panneau) | `FirstPlan` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `size` | côté du bouton | 36px, la taille native des cinq socles |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `enabled` | `bool` | `true` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tooltip` / `log_name` | infobulle, nom d'instance | aucune / `"icon-button"` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `preview_state` | `Idle` / `Hovered` / `Disabled` — **galerie et captures uniquement** | état réel |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Textures** : `button-icon.png`, `button-icon-first-plan.png` et leurs `-hover` (36 × 36 toutes les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |quatre), plus `button-icon-disabled.png` **partagée par les deux contextes** — le jeu n'a capturé
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |qu'un socle grisé. Les glyphes viennent de `assets/design-system/icons/` ; **le manifeste ne porte
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |que ceux qui servent** (le dossier en compte plus de trente, et `DesignSystem::load` les téléverse
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |tous dès le premier composant peint).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Teintes** : `#c5cbcc` au repos, `#f4d89f` au survol, mesurées sur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`menu-button-icon-first-plan.png`. Les icônes du design system étant blanc pur avec alpha, une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |teinte appliquée au moment de peindre suffit — pas de copie recolorée à charger.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Taille d'encre** : les glyphes sont détourés au pixel près, donc de tailles inégales d'un fichier
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |à l'autre (13, 14, 16). Le jeu les cale sur une grille commune — **18px d'encre pour un socle de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |36**, médiane des huit icônes de `menu-button-icon-first-plan.png` (plage 16–20, seuil de luminance
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |140, invariant de 120 à 180). C'est `tokens::ICON_BUTTON_CONTENT`, appliqué par
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`DsTexture::icon_content_size` : **le manifeste, pas l'appelant** — la taille d'encre est une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |propriété de l'asset. Trois tests l'ancrent : la médiane remesurée sur la capture du jeu, le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |détourage des glyphes, et le calcul de `icon_draw_size` (dont le cas du « − », qu'un mauvais facteur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |transformerait en barre).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Échelle et survol** : le socle et l'icône partagent le même facteur d'échelle (dérivé de la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |largeur du socle), et un appui de souris retire l'apparence survolée, qui revient au relâchement —
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la règle commune à toute l'interface.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**État désactivé — deux mécaniques, une par contexte.** `button-icon-disabled.png` est le socle
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |grisé du jeu, et il appartient au contexte `Panel` : luminance moyenne 60, contre 81 pour
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`button-icon.png`, le socle actif du même contexte. Posé sur une barre de premier plan (41), il
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |s'inverse — le bouton désactivé devient le plus lumineux de la barre. En `FirstPlan`, le composant
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |garde donc le socle de repos et l'assombrit (`tokens::DISABLED_DIM`, ~43 % d'opacité).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Utilisé en production** depuis le 2026-09-10 : les quatre boutons du carré de contrôle du panneau
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Suivi (« + », « − », Détails, Options — `panels::watchlist::control_button`). Ce qui reste à la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |charge du panneau : le **placement de l'infobulle** par colonne (à gauche pour « + » et Détails, à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |droite pour « − » et Options), que le `.tooltip()` du composant ne sait pas reproduire — il retombe
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sur le placement par défaut d'egui.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le cas « réinitialisation »** attend qu'un réglage réinitialisable existe. Note du relevé à ne pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |« corriger » le jour venu : dans la barre d'onglets, **l'axe du bouton est 9px plus bas que celui des
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |onglets**.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Paire volume/muet (2026-09-11)** : `DsTexture::IconVolume` (26 × 22) et `DsTexture::IconVolumeMute`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(26 × 26), détourées par le skill `design-asset` depuis deux captures du jeu sans socle porteur —
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |contrairement aux autres glyphes de la table, prélevés sur un bouton. Entrées au manifeste et à la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |galerie de contrôle avant tout appelant réel, comme `DsTexture::ModalHeader` l'avait été pour sa
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |bannière : préparées pour le futur bouton muet/actif de la fenêtre Options
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(`interface-options-son.png`), aucun réglage de son n'étant câblé dans l'overlay à ce jour
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(`alert_sound` ne fait que jouer les sons, jamais les couper).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Paire œil/œil barré (2026-09-11)** : `DsTexture::IconEye` et `DsTexture::IconEyeOff` (16 × 14
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |toutes les deux), même provenance et même statut que la paire volume/muet — détourées par le skill
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`design-asset` depuis deux crops sans socle porteur, entrées au manifeste et à la galerie avant tout
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |appelant réel. Candidat naturel pour un futur toggle de visibilité (masquer une entrée du panneau
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Suivi, ou un champ de jeton dans la fenêtre Options) ; aucun des deux n'est câblé aujourd'hui.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Lot complet du répertoire `icons/` (2026-09-11)** : delta entre `assets/design-system/icons/*.png`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(38 fichiers) et `DsTexture::ALL` demandé par l'utilisateur (« pas tous les icônes du répertoire
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |dans le manifeste ») — 22 fichiers manquaient, tous ajoutés d'un coup : `IconBagIn`, `IconBagOut`,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`IconBook`, `IconCalendar`, `IconCards`, `IconCharacters`, `IconFilter`, `IconGrid`, `IconHammer`,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`IconKamas`, `IconLock`, `IconOrder`, `IconPact`, `IconPin`, `IconRepeat`, `IconSave`,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`IconSettings1`, `IconSettings2`, `IconSort`, `IconTriangleRight`, `IconTrophy`, `IconXp`. Aucun n'a
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |d'appelant réel, comme les deux paires ci-dessus.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`icon_content_size` n'est posé QUE sur les neuf dont l'extraction `--from-button` est consignée dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`references/recettes-icones.md` du skill `design-asset` (preuve qu'ils vivaient sur un socle de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |bouton icône dans le jeu) : `IconBagIn`, `IconBagOut`, `IconFilter`, `IconLock`, `IconOrder`,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`IconPact`, `IconSave`, `IconSort`, `IconTriangleRight`. Les treize autres (`IconBook`,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`IconCalendar`, `IconCards`, `IconCharacters`, `IconGrid`, `IconHammer`, `IconKamas`, `IconPin`,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`IconRepeat`, `IconSettings1`, `IconSettings2`, `IconTrophy`, `IconXp`) n'ont pas cette preuve — soit
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |détourés sans bouton porteur (documenté), soit sans mesure consignée (provenance retrouvée dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'historique git, pas dans le jeu d'essai du skill) — et restent donc sans normalisation, par la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |même prudence que `IconSearch`/`IconTick`/`IconChevronDown` : une taille d'encre est une propriété
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |mesurée de l'asset, jamais devinée. Peints dans la galerie à leur taille de fichier plutôt que par
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`icon_button`, faute de socle à leur donner (nouvelle section « Glyphes sans socle connu »).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-testkit/tests/design_gallery.rs` : les deux rangées d'icônes sur socle sont passées
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |de `ui.horizontal` à `ui.horizontal_wrapped`, la largeur fixe de 760px ne contenant plus tous les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |glyphes sur une seule ligne — et la hauteur du canevas de test est passée de 4160 à 4500px pour la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |même raison (le bas de la galerie sortait sinon du cadre).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::window` — chrome de fenêtre (2026-09-10)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/window.rs` — **composant conteneur**, forme « zone
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rendue » (§1 bis du contrat).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |let chrome = design::window("Options")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .footer("Annuler", "Valider")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .log_name("options")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .show(ui);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |chrome.tabs(ui, design::tabs(&mut state.tab).entry(Tab::Parametres, "Paramètres"));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |match chrome.footer { design::FooterClick::Validate => …, _ => {} }
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `title` (à la construction) | peint dans la bannière, serif grasse cernée d'une ombre bas-droite | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tab_bar_height` | hauteur réservée à la barre d'onglets ; `0.0` pour une fenêtre sans onglets | `tokens::TAB_HEIGHT` (44) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `footer` | libellés des deux boutons — annulation à gauche, validation à droite | aucun pied de page |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `log_name` | préfixe des deux boutons dans le journal | `fenetre` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Rend un `WindowChrome` : `tab_bar` (la bande d'onglets), `content` (entre onglets et pied) et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`footer` (le clic reçu). **`content` n'est pas écrêtée** — c'est le prix de la forme « zone rendue »,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |et la raison pour laquelle le contenu passe normalement par `design::panel`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**La barre d'onglets n'est pas peinte par le chrome** : `WindowChrome::tabs` la pose à partir du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`Tabs` que l'appelant construit. Un onglet est du contenu, pas du décor — et c'est aussi ce qui
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |évite de rendre la fenêtre générique sur le type d'onglet de son contenu.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Utilisé en production** : la modale Options (`panels::options_modal`) et les maquettes de la page
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Alertes (`crates/overlay-testkit/examples/alertes-mockups.rs`).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::panel` — panneau de contenu (2026-09-10)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/panel.rs` — **composant conteneur**, forme closure.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design::panel().show(ui, chrome.content, |ui, panel| {
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    ui.add(design::heading("Fichier"));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    ui.add(design::input(&mut state.path));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    panel.scroll_area(ui, "options-contenu", |ui, width| { … });
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |});
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Un panneau n'est pas une section.** Le relevé est catégorique : dans le jeu, une *section* n'a ni
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |fond, ni bordure, ni filet — son seul signal de regroupement est l'espacement, et son seul signal de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |niveau le retrait de 7 px de son titre. Ce qui a un fond (`#15181c`), un bord (2 px `#131518`) et un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rayon (2), c'est le **panneau** qui contient les sections.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Ce qu'il fait pour son contenu, et qu'aucun appelant n'a donc plus à faire : les rembourrages, la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |réserve de barre de défilement à droite (26 px, **toujours posée**, comme le jeu), l'écrêtage — élargi
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |à gauche du retrait des titres, sans quoi un titre de section perd sa première lettre —, et la mise à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |zéro de l'espacement implicite d'egui.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Utilisé en production** : la modale Options et les maquettes de la page Alertes.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::heading` — titre de section (2026-09-10)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/heading.rs` — composant **feuille**.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(design::heading("Fichier"));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `text` (à la construction) | le titre | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `trailing_gap` | écart réservé sous le titre | `tokens::HEADING_TO_ROW` (7) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Serif grasse au corps du titre de fenêtre (21), **gris `#b8b9ba` et non blanc** : la hiérarchie
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |entre les deux niveaux de titre du jeu passe par la couleur, pas par le corps.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Deux pièges que le composant absorbe : il réserve la hauteur d'**encre** (16) et non celle de sa
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |galley — réserver la galley ajoutait ~14 px invisibles sous le titre, sur sept relevés — et il
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |applique lui-même le **retrait de 7 px** qui est, dans le jeu, le seul signal qu'une section existe.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Utilisé en production** : la modale Options et les maquettes de la page Alertes.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::stepper` — pas numérique (2026-09-10)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/stepper.rs` — composant **feuille**.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(design::stepper(&mut quantite).range(1..=999).log_name("hdv-quantite"));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `value` (à la construction) | `&mut i64`, muté par les deux boutons | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `range` | domaine autorisé ; la valeur y est **écrêtée à chaque frame**, y compris celle fournie | `i64::MIN..=i64::MAX` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `step` | incrément d'un clic | 1 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `size` | côté des deux boutons, **et donc hauteur du pas** — le socle est carré dans le jeu | `tokens::STEPPER_SIZE` (32) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `field_width` | largeur du champ central ; sans elle, il prend toute la place restante | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `enabled` · `log_name` | comme partout | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Il ne peint rien lui-même** : deux `design::icon_button` au contexte `Stepper` et un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`design::input` en lecture seule. Le socle vient du manifeste (`DsTexture::ButtonStepper`), les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |glyphes aussi (`IconPlus`, `IconMinus`).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Mesures, prises sur `large-input-number.png`** (192 × 34), la seule capture dont le socle
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |coïncide avec l'asset isolé `button-moins.png` :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur | Vérification |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Socle | `STEPPER_SIZE` (32) | y=1..32 ; deux boutons symétriques, x 1..32 et x 159..190 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Gouttière | `STEPPER_GUTTER_RATIO` (10/32) | 1 + 32 + 10 + 106 + 10 + 32 + 1 = 192 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Encre du glyphe | `STEPPER_ICON_RATIO` (12/32) | 12 × 12 mesurés dans un socle de 32 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Hauteur du champ | `STEPPER_FIELD_HEIGHT_RATIO` (**1,0**) | **écart assumé** — le jeu met 26 pour 32 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Teinte du glyphe | `STEPPER_ICON_TINT` (`#f4d89f`) | mesuré au pixel — **de l'or au repos** |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**La BOÎTE du champ fait la hauteur de ses boutons — son texte, non** (décision utilisateur,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |2026-09-10). Le jeu met un champ de 26 px pour un socle de 32 : un champ plus court que ses boutons
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |a été jugé peu soigné, et l'uniformité l'emporte ici sur la fidélité.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Mais étirer la boîte ne doit **pas** étirer son contenu. Le champ est donc posé avec
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`Input::box_height`, ajouté pour ce cas, et non avec `size(InputSize::Height(...))` qui met tout à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'échelle — cette première version écrivait la valeur en corps 22 au lieu de 17, soit un tiers trop
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |gros. Vérifié après correction : **12 px d'encre, exactement comme le jeu**.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**La petite capture (`input-number.png`) n'est pas une source.** Sa gouttière concorde à un pixel
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |près, mais son glyphe fait 12 px dans un socle de 24 — le même que dans un socle de 32. Les deux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |captures ne sont donc pas le même composant à deux échelles : le jeu règle son interface de 67 % à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |233 %, et elles ont été prises à deux réglages différents. Elle sert de contrôle de cohérence, rien
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |de plus.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Une erreur de mesure corrigée le jour même** : la première version divisait par la hauteur du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**fichier** (34) au lieu de celle du **socle** (32). Elle donnait une gouttière de 0,267 au lieu de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |0,3125 et des boutons deux pixels trop hauts.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Trois choses que la comparaison au jeu a corrigées** (étape 6 du skill, sans laquelle aucune ne se
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |serait vue) : le glyphe est **doré au repos**, alors que les deux autres contextes de bouton icône
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le peignent en gris et ne passent à l'or qu'au survol ; le champ **ne garde pas sa hauteur native**
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |de 25 px mais suit son pas ; et le champ est en **lecture seule, pas désactivé** — sa valeur compte,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le jeu l'écrit en or, pas en gris.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Deux écarts assumés**, faute de capture : aucun socle survolé n'existe pour cette famille (le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |survol ne se signale donc que par le curseur — une teinte egui *multiplie* la texture, elle ne peut
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pas l'éclaircir), et le champ n'est pas éditable au clavier (valider une saisie partielle est une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |spec à part entière, sans référence pour ses états d'erreur).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Pas encore utilisé en production.** Ses clients naturels : les boutons « + » / « − » du carré de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |contrôle du Suivi, et le couple quantité du formulaire de vente HDV.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::collapsible` — bloc repliable (2026-09-11)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/collapsible.rs` — **composant conteneur**, forme closure.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design::collapsible("Le Village", &mut ouvert)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .icon(icone_de_quete)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .show(ui, |ui| {
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        // n'importe quoi : du texte, un formulaire, un tableau, des cases à cocher…
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    });
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `title` (à la construction) | peint dans l'en-tête, serif grasse | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `open` (à la construction) | `&mut bool`, basculé par le clic sur l'en-tête | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `icon` | icône d'en-tête | aucune |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `log_name` | nom d'instance au journal | le titre |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `preview_hovered` | force l'état survolé — planches de contrôle seules | l'interaction |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Deux états, et c'est toute sa définition fonctionnelle.** Fermé, le bloc n'est que son en-tête.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Ouvert, il montre son contenu et on peut interagir avec. **Le contenu est entièrement libre** — le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |composant n'en sait rien, il lui garantit un cadre, des marges et un écrêtage.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`show` rend un `InnerResponse<Option<R>>` : `None` dit que **la closure n'a pas tourné** parce que
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le bloc est fermé, ce qui n'est pas la même chose qu'un contenu vide.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Mesures** — source unique : [`collapse-block.json`](design-system/collapse-block.json), le relevé
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |outillé du mainteneur sur les captures **détourées** `collapse-block-closed.png` (732 × 60) et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`collapse-block-opened.png` (732 × 210).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur | Vérification |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || En-tête | `COLLAPSE_HEADER_HEIGHT` (60) | cadre fermé 732 × 60 — il ne contient rien d'autre |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Marge latérale | `COLLAPSE_PAD_X` (17) | icône et intitulés de section à x=17 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Marge basse | `COLLAPSE_PAD_BOTTOM` (23) | `padding` du nœud `frame` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Marge du chevron | `COLLAPSE_CHEVRON_PAD_X` (13) | glyphe x 705..719, cadre large de 732 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Titre | `COLLAPSE_TITLE_FONT_SIZE` (18) | 14 px d'encre, `#fefefe`, vérifié au rendu |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Icône | `COLLAPSE_ICON_RATIO` (32/60) | 33 × 32 dans un en-tête de 60 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Gouttière icône–titre | `COLLAPSE_ICON_GAP` (10) | icône finit à x=50, titre à x=60 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Chevron | `COLLAPSE_CHEVRON_TINT` (`#a69064`) | doré olive, **pas** `ICON_TINT` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le relevé qui précédait celui-ci portait sur les **mêmes captures non détourées** : l'ombre portée et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le fond de jeu y comptaient comme de l'encre, d'où un titre annoncé à 18 px au lieu de 14, une icône
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |à 35 au lieu de 33 et des marges de 16/20 au lieu de 17/23. `releve-collapse.json` a donc été retiré
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |— deux relevés qui se contredisent valent moins qu'un seul.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le survol éclaircit le cadre entier** (`#28292b` → `#323436`, dix niveaux sur chaque canal) quand
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le pointeur est sur l'en-tête. Survoler le contenu ne l'éclaircit pas : rien n'y est cliquable.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Quatre choses à savoir avant de le modifier :**
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Le cadre est peint APRÈS le contenu**, dans un emplacement réservé au préalable
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  (`DesignSystem::paint_to_slot`) : sa hauteur dépend de ce que le contenu a pris. C'est la raison
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  d'être de la forme closure pour ce composant.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Le chevron est retourné, pas dupliqué** (`DesignSystem::paint_flipped_y`). Un second asset pour
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  l'état ouvert serait un asset par état, aussi interdit qu'un asset par taille.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Le cadre survolé, lui, EST un second asset**, et c'est la seule exception du composant : le jeu
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  *éclaircit* son fond, or une teinte egui multiplie — elle ne sait que foncer. Quand l'état n'est
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  pas atteignable par la teinte, l'asset par état est la bonne réponse.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Les marges 9-slice du cadre sont asymétriques** (`COLLAPSE_BLOCK_SLICE` : 30 à gauche, 36 en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  haut, 10 à droite, 16 en bas) parce que son décor l'est : une gravure en circuit marque les deux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  angles **gauches** et rien à droite. Un découpage symétrique à 10 px, essayé d'abord, étirait cette
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  gravure sur toute la hauteur du bloc.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **`icon` est le seul paramètre-texture du design system**, et c'est assumé : cette icône appartient
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  au contenu (une quête, un lieu), elle vient de la donnée. Le composant ne peut pas la résoudre
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  depuis une intention.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Deux écarts assumés** : le cadre du jeu est **translucide** et ne l'est pas ici — les huit
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |captures fournies sont toutes sur le même fond de jeu, il en faudrait deux sur des fonds différents
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pour en déduire l'alpha, comme `tools/design-system/build_modal_body.py` le fait pour la modale —, et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le composant **ne rend pas son contenu défilable** : un contenu qui peut déborder s'enveloppe dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |une `design::scroll_area`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Deux des huit assets fournis ne sont pas embarqués** : `collapse-block-closed-generic.png` et son
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |jumeau survolé. Le 9-slice tiré de l'état ouvert rend les deux états (36 + 16 = 52 px tiennent dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |les 60 d'un bloc fermé, et les deux génériques concordent au pixel sur leurs bandes haute et basse).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Ils restent au dépôt comme référence de contrôle.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Pas encore utilisé en production.**
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::loader` — rouage de chargement (2026-09-11)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/loader.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, LoaderSize};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(design::loader());                                   // 124 px, taille native (1×)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(design::loader().size(LoaderSize::Small));           // 48 px, le plancher
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(design::loader().size(LoaderSize::Px(80.0)).tooltip("Synchronisation…"));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `size` | `Small` (48) / `Medium` (72) / `Large` (96) / `Native` (124) / `Px(f32)` ramené dans `[48, 124]` | `Native` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tooltip` | texte d'infobulle | aucune |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `log_name` | nom d'instance pour le journal | `loader` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `preview_frame` | image de la boucle (`0..16`) — **galerie et captures uniquement** | horloge egui |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Toujours carré, toujours animé, aucun état** : un indicateur de chargement ne se clique pas et ne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |se désactive pas. `LoaderSize::px()` rend le côté effectivement peint, pour un panneau qui réserve
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la place avant d'ajouter le composant.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**L'animation ne coûte que 24 réveils par seconde** : le composant demande à egui un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rafraîchissement au prochain changement d'image (`request_repaint_after`), pas un rendu continu.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Hors écran (galerie), `preview_frame` fige l'image et aucun réveil n'est demandé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Origine** : enregistrement du client du 2026-09-11 (`Enregistrement 2026-09-11 142031.mp4`,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |158 × 156 px, 30 i/s, sept écrans de chargement), isolé image par image — fond retiré par
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |estimation du fond sur toute la plage, blanc conservé avec alpha 8 bits.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Mesures** :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur | Origine |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Image | 124 × 124 px | 118 px d'encre (rayon 58,5) + 3 px de marge transparente |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Dents | 16 | profil angulaire au rayon des dents |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Rotation | 2,8° par image, une dent toutes les 8 images | corrélation angulaire image à image |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Boucle | **16 images** | image 386 ≡ 409, pas 399 : la tête a une période double de celle des dents |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Cadence | 24 i/s | 4 images nouvelles sur 5 enregistrées |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Choisi, pas mesuré** : le plancher de 48 px est une décision utilisateur (en dessous, les seize
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |dents se confondent) ; `Medium` et `Large` découpent l'intervalle en marches d'environ 25 px. Une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |taille hors de l'intervalle est ramenée à la borne, avec un `warn!` unique par instance.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Textures** : `loader-sheet.png` (496 × 496, grille 4 × 4 lue ligne par ligne), **une planche et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |non seize textures** — peinte par `DesignSystem::paint_region`, le seul chemin du manifeste qui
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |lit une région par ses UV. Le 9-slice déclaré (`LOADER_SLICE` = `ICON_SLICE`) ne sert pas. La
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |même boucle existe en `loader.apng` (alpha 8 bits, 24 i/s) et `loader.gif` (transparence 1 bit,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |25 i/s faute de granularité GIF) pour tout ce qui n'est pas egui.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Pas de comparaison au jeu à la même taille** : le rouage n'y existe qu'à 1×, et c'est précisément
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la capture dont il est tiré.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::separator` — filet de séparation (2026-09-11)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/separator.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.label("Description");          // l'intitulé de section
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add_space(10.0);               // la cote du dessus appartient à l'appelant
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(design::separator());      // le filet, qui réserve 12 px sous lui
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.label("le corps de la section");
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `width` | largeur imposée | toute la largeur disponible |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `on_hovered_surface` | `bool` — la **surface porteuse** est survolée | `false` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `trailing_gap` | écart réservé sous le filet | `SEPARATOR_GAP_BELOW` = 12 px |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `log_name` | nom d'instance pour le journal | `"separator"` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Aucune texture : quatre lignes de pixels sur deux teintes plates, un PNG n'aurait rien à porter.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Les mesures
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Relevées au pixel sur `collapse-block-opened.png` et `collapse-block-opened-hover.png` — les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |captures **source**, pas les génériques, qui ont justement été nettoyées de leur contenu, filet
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |compris. Le filet y apparaît deux fois (y=82..85 et y=152..155), identique dans les deux occurrences
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |et dans les deux textures.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || | fond | ligne claire (2 px) | ligne sombre (2 px) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || repos | `#28292b` | `#323335` (+10) | `#222325` (−6) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || survol | `#323436` | `#3c3e3f` (+10) | `#2b2c2e` (−7) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Quatre choses à savoir
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |1. **C'est un bevel, pas un trait.** Deux lignes plates empilées, sans dégradé : les quatre lignes
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   de pixels ne portent que deux teintes, sans valeur intermédiaire. Rendu en un `hline` gris, le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   filet paraît posé *sur* le fond au lieu d'y être creusé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |2. **Le bevel suit son fond**, d'où `on_hovered_surface`. Le jeu recalcule les deux lignes quand la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   surface s'éclaircit. Sans ce paramètre, la ligne claire du repos (`#323335`) se retrouve à **un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   niveau** du fond survolé (`#323436`) et disparaît — la galerie montre les deux cas côte à côte
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   pour que l'écart se voie plutôt que d'être affirmé ici.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |3. **Le paramètre ne s'appelle pas `hovered`** parce que le filet ne se survole pas lui-même : il
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   n'est pas cliquable, et `response.hovered()` sur ses 4 px ne renseignerait sur rien. Ce qu'on lui
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   dit, c'est l'état de la surface qui le porte.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |4. **Il ne pose aucune marge latérale.** Dans la capture le filet court de x=16 à x=719 sur un cadre
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   de 732 — 16 à gauche, 12 à droite. Cette asymétrie est celle du **bloc**, pas du filet : le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   chevron de l'en-tête s'arrête au même x=719. Le composant remplit la largeur qu'on lui donne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   (§6 du contrat) ; dans un `design::collapsible` cela donne les 17 px de `COLLAPSE_PAD_X` des deux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   côtés.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Deux corrections au relevé
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`docs/design-system/collapse-block.json` est en écart avec la texture sur deux points, tranchés en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |faveur de la mesure :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- la note de `sep-1` donne la ligne claire à `#323436` — la texture donne `#323335`. `#323436` est
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  la teinte du *fond survolé*, vraisemblablement relevée à sa place.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- la palette déclare `border-bevel-dark: #28292b`, qui est exactement le fond au repos : une ligne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  sombre de cette teinte serait invisible. La texture donne `#222325`, valeur que la note de `sep-1`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  portait déjà. C'est la palette qui est fautive, pas la note.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Comparaison au jeu : les huit lignes de pixels (quatre par état) sont **identiques au rendu**, sans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |dérive de gamma.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::slider` — curseur de réglage (2026-09-11)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/slider.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |if ui.add(design::slider(&mut state.volume)).changed() {
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    audio.set_volume(state.volume);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |}
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |// Le motif du jeu : un libellé de chaque côté, à la gouttière relevée.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.horizontal(|ui| {
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    ui.label("Min");
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    ui.add_space(design::tokens::SLIDER_LABEL_GAP);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    ui.add(design::slider(&mut state.volume).width(200.0));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    ui.add_space(design::tokens::SLIDER_LABEL_GAP);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    ui.label("Max");
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |});
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `range` | `RangeInclusive<f32>` | `0.0..=1.0` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `steps` | nombre de valeurs sélectionnables — **grade le curseur** | aucun, curseur continu |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `width` | largeur imposée | toute la largeur disponible |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `enabled` | `bool` | `true` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tooltip` / `log_name` | | aucune / `"slider"` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `preview_state` | `Idle` / `Hovered` / `Disabled` — **galerie uniquement** | état réel |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `preview_fraction` | position forcée, 0..1 — **galerie uniquement** | la valeur |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Textures : `DsTexture::SliderHandle` (`slider-handle.png`, 18 × 18). La rainure, elle, est peinte.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Les mesures
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Relevées au pixel sur les deux curseurs de volume de `interfaces/interface-options-son.png`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(« Musique », rainure y 335..342 ; « Sons - Ambiance », y 430..437), qui concordent sur toutes les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |cotes.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Rainure | 8 px de haut, 2 de liseré en haut et en bas |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Liseré | assombrit le fond dans le rapport **0,735** |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Intérieur | rapport **0,853** |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Poignée | 18 × 18, cercle (`radius_fit_iou` 1,0) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Poignée — teintes | `#c6b187` clair, `#635944` sillon, `#817358` ombre bas-droite |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Largeur relevée | 200 px (x 75..274) — un ordre de grandeur, pas un gabarit |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Gouttière du libellé | 12 px à gauche, 10 à droite |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Les graduations
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Un curseur gradué annonce **où la poignée peut s'immobiliser** ; sans `steps`, il est continu et nu.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |La distinction est mesurée, pas choisie : le curseur d'échelle d'interface
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(`interfaces/interface-options-interface.png`) porte **26 graduations**, celui du volume **aucune**
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |— pas un pixel clair le long de sa rainure.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Et elles tombent bien sur les arrêts de la poignée : les trois libellés de la capture sont centrés
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |à x = 114, 199,5 et 539,5, pour des graduations à 111, 196 et 536 — la première, la sixième et la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |vingt-sixième. Le disque, lui, est exactement sur la sixième.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Largeur | 1 px (26 graduations sur 26) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Couleur | `#e2ddd7` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Débord au-delà de la rainure | 2 px, en haut comme en bas |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Morsure sur le liseré | **1 px** sur les 2 du liseré |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Segment visible | 3 px de chaque côté, 6 px de rainure nue entre les deux |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Une graduation ne traverse pas la rainure**, elle la coupe : entre les deux segments, les pixels
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |d'une colonne graduée sont identiques à ceux d'une colonne nue.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Un écart assumé avec la capture** : dans le jeu les graduations s'arrêtent à 37 px des bords de la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rainure (x 111..536 pour une rainure de 74 à 573), soit 28 px de plus que le rayon de la poignée. Le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |composant ne reproduit pas ce retrait — une seule capture d'un curseur gradué ne dit pas si ces
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |37 px sont absolus ou proportionnels, et les extrapoler ferait 74 px de retrait sur une rainure de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |120. Trancher demande une seconde capture, à une autre largeur.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Quatre choses à savoir
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |1. **La rainure est un creux, pas une barre.** Elle n'a pas de couleur propre : elle assombrit le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   fond. C'est une mesure et non un choix de rendu — sur deux fonds qui diffèrent de deux niveaux,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   le rapport au fond reste le même sur les trois canaux. Deux constantes opaques auraient
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   reproduit la capture et rien d'autre.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |2. **La portion parcourue n'est pas remplie.** Les deux curseurs du jeu sont au minimum : rien ne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   montre ce que devient la rainure derrière la poignée. Peindre un remplissage doré serait
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   inventer du design — ce que `design::input` refuse déjà pour son état survolé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |3. **La poignée est un asset alors que ses teintes sont plates**, parce que son relief est
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   *directionnel* : liseré clair en haut à gauche, mi-ton en bas à droite, sillon sombre entre les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   deux. Le peindre demanderait deux arcs partiels pour dix-huit pixels.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |4. **Les libellés d'extrémité n'appartiennent pas au composant.** « Min »/« Max » conviennent à un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   volume, pas à une échelle d'interface qui dirait « 50 % »/« 200 % ». Le jeton donne la gouttière,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   l'appelant pose les mots.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce que la comparaison au jeu a rattrapé — trois fois
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |1. **Le creux à l'envers.** La première version peignait le liseré sur toute la hauteur puis
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   l'intérieur par-dessus. Les alphas se composent : 0,733 × 0,851 = **0,624**, et l'intérieur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   sortait plus sombre que son propre liseré. Trois bandes disjointes ramènent les rapports à 0,75
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   et 0,83, contre 0,74 et 0,85 dans le jeu.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |2. **La graduation trop longue.** Elle mordait les 2 px du liseré au lieu d'un seul : 4 px de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   segment et 4 px de rainure nue, au lieu de 3 et 6. Assez pour que le repère se lise comme un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   trait presque continu.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |3. **Le demi-pixel.** Le `Ui` appelant peut allouer à un y non entier, et un creux de 8 px posé à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   y,5 s'étale sur 9 lignes — les graduations repassaient à 4 px. La rainure est désormais calée sur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   la grille (`.round()`).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Après les trois, le profil vertical d'une graduation est **identique au jeu**, ligne par ligne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(`TTT......TTT..`), et la poignée l'est au pixel. Aucune relecture de code n'aurait vu ces trois
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |défauts.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::tooltip` — infobulle (2026-09-11)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/tooltip.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, TooltipSide};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design::tooltip(&response).text("Ajouter à la liste");            // au-dessus, le défaut
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design::tooltip(&response).side(TooltipSide::Right).text("Options");
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design::tooltip(&response).show(|ui| { /* contenu libre */ });
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `side` | `Above` / `Left` / `Right` | `Above` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `gap` | écart au widget | `TOOLTIP_GAP` = 5 px |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `text` / `show` | texte simple / contenu libre | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Aucune texture : le fond, la marge et l'ombre viennent du **thème** (`style::apply`), parce que
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`window_fill`/`menu_margin`/`popup_shadow` ne servent qu'aux infobulles dans cette interface. Un seul
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |réglage couvre donc aussi les `on_hover_text` ponctuels.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce qu'il absorbe
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Les **trois enveloppes maison** — `combat::show_tooltip_above`, `watchlist::show_tooltip_left` et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`_right` — rigoureusement identiques à un alignement près, chacune avec sa liste de replis recopiée.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`panels/tooltip.rs` disparaît avec elles ; ses quatre constantes mesurées sont remontées dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`design::tokens`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Les replis, qui sont le composant
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Un alignement qui ne tient pas ne doit pas retomber n'importe où. L'ordre est le résultat de trois
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |bugs rapportés, pas une préférence :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |1. le côté demandé, centré ;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |2. **le même côté, réaligné** (`_START`, `_END`) — ajoutés avant les replis opposés le 2026-09-06 :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   un widget proche d'un bord reste du bon côté, simplement décalé ;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |3. le côté opposé, en dernier recours — et jamais `BOTTOM_START`, le défaut d'egui (« sous la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   souris »), qui est précisément ce qu'on fuit.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le cas qui a fait ajouter l'étape 2 : le bouton Détails du panneau Combat, collé au bord droit,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |débordait en `TOP` centré et retombait sous le curseur ; son voisin Options, un peu plus loin du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |bord, ne révélait jamais le problème.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Un repli ne crée pas de la place.** Le switch Alliés/Ennemis, premier widget du panneau Combat,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |n'avait réellement aucune place au-dessus de lui : la réponse est `render_content::
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |COMBAT_TOP_MARGIN`, pas un alignement.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le côté se choisit par colonne, pas par bouton** (retour du 2026-09-08) : dans le carré de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |contrôle du Suivi, un bouton de droite dont l'infobulle partirait à gauche la poserait par-dessus
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |son voisin, gênant le survol de ce dernier.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Un écart au contrat, assumé et daté
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**La police reste celle du thème**, là où le contrat veut `design::text::label_font`. La corriger est
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |un changement visuel : elle déplacerait les six captures d'infobulle de la suite de rendu, que le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |plan demande justement **inchangées** pour prouver que cette migration ne change rien. Les deux ne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |peuvent pas tenir dans le même commit — la police attend sa propre décision.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Pas d'entrée de galerie** : une infobulle a besoin d'un survol, qui n'existe pas en rendu
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |offscreen statique. Sa vérification visuelle est ailleurs et existait déjà — les six tests dédiés
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`watchlist_tooltip_*` et `combat_tooltip_*`, qui simulent le pointeur. Ils sont restés **identiques
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |au pixel** à travers la migration, ce qui est le critère de fin que le plan fixait.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::icon` et le registre `DsIcon` (2026-09-11)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/icons.rs`, `components/icon.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, DsIcon};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(design::icon(DsIcon::Kamas));                  // 16 px par défaut
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(design::icon(DsIcon::Lock).size(24.0).tint(tokens::TEXT_DISABLED));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `size` | côté du **carré englobant** | `ICON_SIZE` = 16 px |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tint` | teinte | `ICON_TINT` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Deux registres, et pourquoi
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`DsTexture` et `DsIcon` étaient une seule énumération, ce qui obligeait chacun à porter les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |propriétés de l'autre :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Un glyphe n'a pas de 9-slice.** Les 38 icônes déclaraient toutes `ICON_SLICE`, un 9-slice
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  dégénéré présent parce que le champ était obligatoire. Il ne décrivait rien et masquait ce que la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  texture *est*.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Un fond n'a pas d'étalon d'encre.** `icon_content_size` énumérait à la main, dans un `match` de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  vingt lignes, les variantes qui sont des icônes normalisées — liste tenue en parallèle de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  l'énumération, que rien ne vérifiait.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le défaut que cette confusion a produit est daté : le test `les_glyphes_d_icone_sont_detoures_au_
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pixel_pres` sélectionnait ses cibles **par leur découpage** (`insets == 0`), faute de mieux. Il
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ratait sa cible dans les deux sens — laissant passer les glyphes sans socle, et attrapant
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`loader-sheet.png`, une planche d'atlas qu'il a fallu exempter en ajoutant `DsTexture::grid`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Aujourd'hui il balaie `DsIcon::ALL`, sans filtre ni exemption.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### La table est la source unique
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Nom de cache egui, chemin du fichier et présence d'un étalon viennent d'**un seul littéral** par
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |icône, assemblés par la macro `ds_icons!`. Il n'y a plus de façon d'écrire `ds-icon-eye` en face de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`icon-eye-off.png` — c'était possible tant que les trois étaient recopiés à la main dans `spec()`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Deux catégories d'étalon, marquées dans la table :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **`socle`** (21 icônes) — détourée depuis un socle de bouton du jeu, taille d'encre connue et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  comparable, donc normalisable sur `ICON_BUTTON_CONTENT` ;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **`libre`** (17) — détourée sans bouton porteur ou sans mesure consignée. La normaliser sur un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  étalon qu'elle ne partage pas la rendrait fausse ; elle garde sa taille native.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce que `design::icon` fait, et ce que fait `icon_button`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`icon_button` peint un glyphe **sur un socle**, avec ses états et son clic. `design::icon` ne peint
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |que le glyphe : une icône dans une ligne, un en-tête de colonne, à côté d'un compteur. Il n'est pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |cliquable — qui veut un clic prend `icon_button`, qui a le socle que ce clic mérite.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le rapport d'aspect est préservé** : les glyphes du jeu ne sont pas carrés (chevron 14 × 8,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pastille d'info 27 × 28). Le composant réutilise `glyph_fit`, le seul endroit du crate qui calcule
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ce rapport, plutôt qu'un `Vec2::splat` — exactement le défaut qu'`Input::leading_icon` portait
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |jusqu'au 2026-09-10. `size` donne donc le côté du **carré englobant**, pas la largeur : un chevron
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |demandé à 16 px sera peint 16 × 9, centré dans un carré de 16.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**L'étalon ne s'applique pas hors socle** : `content_size` sert à accorder deux glyphes voisins sur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |deux boutons. Sans voisin, le glyphe occupe le carré qu'on lui donne.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Vérification
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le refactor touche 223 usages et **aucun snapshot n'a bougé**, hormis celui de la galerie où une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |section a été *ajoutée*. Les six captures d'infobulle, les panneaux, la modale : identiques au
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pixel. C'est ce qu'on attend d'un changement de typage.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Coût mémoire, mesuré avant décision : les 38 icônes décodées en RGBA pèsent **31 Ko** — sans effet
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sur le budget de 300 Mo (§8 du plan).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::autocomplete` — champ d'autocomplétion (2026-09-11)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/autocomplete.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, AutocompleteEntry, AutocompleteFilter};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |let issue = design::autocomplete(&mut state.saisie)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .placeholder("Ajouter un objet à surveiller…")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .width(560.0)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .filters(&filtres)   // « Tout » en tête, puis les catégories PRÉSENTES
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .entries(&entrees)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .log_name("alertes.ajout")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .show(ui);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |if let Some(index) = issue.selected {
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    ajouter(&entrees[index]);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |}
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `entries` | `&[AutocompleteEntry]` — libellé, catégorie, gemme, image, désactivé, mention | vide |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `filters` | `&[AutocompleteFilter]` — `all(…)` et `category(id, …)` | vide (pas de bande) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `placeholder` / `empty_filter_label` | textes | `""` / « Aucun résultat dans cette catégorie » |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `width` | largeur imposée | largeur disponible |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `min_query_len` | seuil de déclenchement | `AUTOCOMPLETE_MIN_QUERY_LEN` = 3 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `max_visible_rows` | au-delà, la liste défile | `AUTOCOMPLETE_MAX_VISIBLE_ROWS` = 5 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `enabled` | `bool` | `true` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `preview_open` / `preview_active` / `preview_filter` | aperçu de galerie | fermé / 0 / aucun |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Rend un **`AutocompleteOutcome`** : la `Response` du champ **et** `selected: Option<usize>`, l'indice
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |dans `entries` de l'entrée choisie.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Décor résolu en interne (socle et loupe d'`InputSize::Standard`) ; **panneau déplié entièrement
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |repris de `design::select`** — fond, bord, filet de tête, surbrillance, cadence de rangée de 28 px.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |C'est la même liste du jeu, il n'y avait pas de second relevé à faire. Ce que ce composant ajoute :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la bande de filtres, une image par entrée, des entrées désactivées, et un seuil de déclenchement.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Les cinq règles de comportement, portées du web
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le jeu n'a **pas** d'autocomplétion : aucune capture ne peut servir de référence, donc rien de tout
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ceci ne se vérifie à l'œil. Relevé sur `shared/wakfu-autocomplete` (`Oumbra/wakfu-companion`) le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |2026-09-11.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |1. **Rien avant trois caractères** — comptés en *caractères*, pas en octets.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |2. **Une entrée désactivée n'est pas sélectionnable** : grisée, sans surbrillance au survol, sautée
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   par le clavier, et refusée par `show` même si un clic l'atteignait. Trois barrières, parce
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   qu'une seule finit toujours par être contournée.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |3. **Un filtre actif restreint la liste** à sa seule catégorie.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |4. **La bande se calcule sur la liste NON filtrée** — l'appelant construit `filters` sans tenir
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   compte du filtre actif. Sinon le bouton qui permettrait de relâcher un filtre sans résultat
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   disparaîtrait avec les rangées, et l'utilisateur resterait coincé devant une liste vide.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |5. **Après une sélection** : le champ se vide, le panneau se ferme, l'entrée active repart à la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   première, et **le filtre revient à « Tout »**.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Clavier : `↓`/`↑` sautent les entrées désactivées et bouclent, `Entrée` valide, `Échap` ferme. Les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |touches sont consommées **avant** le champ de saisie, sinon la flèche déplacerait le curseur de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |texte. Une liste entièrement désactivée termine quand même — la recherche s'arrête après un tour
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |complet (test `toutes_desactivees_ne_boucle_pas_indefiniment`).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Deux écarts au contrat, assumés
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**`show` plutôt que `impl Widget`** (§1) : une `Response` ne peut pas dire *quelle* entrée a été
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |choisie, et la ressortir par un `&mut` en paramètre est la maladresse que §6 reproche ailleurs.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Même raison que pour les conteneurs (§1 bis), sur un composant qui n'en est pas un.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Des textures en paramètre** (§1) : la gemme de rareté et l'image d'un objet sont du **contenu**,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pas du décor — elles viennent du CDN `wakassets` par `RemoteIconStore`, le design system ne les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |possède pas. Elles arrivent donc par `AutocompleteEntry`, au même titre que le libellé. Leur absence
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |n'est pas une erreur : la colonne reste réservée, les libellés restent alignés, et la rangée se
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |peint sans elles (c'est l'état normal tant que le CDN n'a pas répondu).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce que le composant ne fait pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Il ne cherche rien. L'appelant lui passe des entrées déjà trouvées, déjà triées, déjà marquées
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |« déjà suivi ». Le domaine n'est **pas** un paramètre : la page Alertes ne lui donne que des objets,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le futur formulaire d'ajout au Suivi lui donnera objets **et** monstres — le composant ne fait pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la différence. Les objets à recette (évolution demandée pour ce même formulaire) suivront de la même
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |façon, par l'appelant.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce que la capture a rattrapé
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **La quatrième rangée sortait du panneau.** La hauteur du panneau vaut `rangées × 28`, sans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  interligne — mais `allocate_exact_size` ajoutait les 3 px d'`item_spacing` hérités du thème entre
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  chaque rangée. Neuf pixels de trop sur quatre rangées : le libellé de la dernière était coupé en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  deux par le bord. Invisible à la relecture, évident sur la planche.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **La `ScrollArea` n'est instanciée que si la liste déborde vraiment** : toujours présente, elle
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  demande un repeint tant que son décalage s'anime, et `Harness::run` tourne alors jusqu'à sa limite
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  d'étapes sans converger.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Les identifiants dérivent de la `Response` du champ**, jamais du `Ui` parent : trois instances
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  dans le même parent partageaient sinon le même id d'`Area` et de rangées — egui l'écrit en rouge
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  par-dessus le rendu.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce qu'aucune capture ne montrait — le temps (2026-09-12)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Second retour du même jour, captures web et overlay côte à côte pour « bouftou » : « extrêmement
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |long », et « pas tous les résultats ». Trois causes, aucune dans ce composant, toutes mesurées
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |plutôt que supposées :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |1. **La fenêtre ne se redessinait qu'une fois toutes les deux secondes** pendant la frappe. Journal
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   instrumenté, frappe pilotée par `SendInput` : chaque touche arrivait dans `window_event`,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   `request_redraw()` était appelé, et aucun `RedrawRequested` ne suivait — la frame suivante venait
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   de la réaffirmation topmost périodique (`SetWindowPos`, 2 s), qui fait repeindre la fenêtre par
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   Windows. Cinq retours arrière et trois lettres s'appliquaient d'un coup, 1,3 s plus tard. Le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   `WM_PAINT` que `RedrawWindow(RDW_INTERNALPAINT)` est censé poster n'arrive pas de façon fiable
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   sur ces fenêtres DirectComposition sans surface de redirection. Corrigé dans l'hôte, pas ici :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   `App::redraw` rend la frame lui-même depuis `about_to_wait`, qui suit chaque livraison
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   d'événements — une touche produit sa frame en moins de dix millisecondes (mesuré, même méthode).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   Voir §6.2 du plan.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |2. **La recherche était coupée à quarante** (`alerts_tab::MAX_SUGGESTIONS`), et la bande de filtres,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   calculée sur la liste rendue, perdait des catégories présentes : cinq boutons ici contre huit sur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   le site. La recherche coûte moins d'une demi-milliseconde pour 115 résultats sur 16 302 objets
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   (mesuré sur le catalogue réel, `overlay-engine/examples/bench-search.rs`) — la limite protégeait
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   un coût qui n'existe pas. L'appelant passe `usize::MAX` ; le panneau défile, comme le web.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |3. **Les icônes arrivaient une par une, en série, dans l'ordre d'arrivée** : un agent HTTP neuf par
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   requête (une poignée de main TLS par icône, ~145 ms chacune), un seul thread, et les icônes des
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   préfixes abandonnés (« bou », « bouf ») servies avant celles de la requête courante. Corrigé dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   `remote_icons` : un agent partagé, quatre threads, et un **tas ordonné par frame egui** — ce que
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   la dernière frame a demandé part en premier, dans son ordre de demande. Une pile LIFO simple,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   essayée d'abord, renversait aussi l'ordre à l'intérieur d'une frame et servait la centième
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   rangée avant la première. Corollaire pour l'appelant : `alerts_tab::add_field` demande les huit
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   icônes de la bande de filtres **avant** les images des rangées — demandées après, elles
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   arrivaient les dernières et la bande restait vide (constaté sur « tofu », 118 résultats).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   Mesuré après correctif : 109 icônes à froid en 2,5 s, les cinq rangées visibles illustrées en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   moins de 500 ms.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le composant, lui, n'a changé que de champ : `InputSize::Search` et `clearable(true)`, voir
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`design::input`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### La barre du panneau (2026-09-12, soir)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Troisième retour du jour : « un tout petit peu plus large, à l'image du web ; l'élargissement au
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |survol ne me paraît pas si mal, mais conserver les couleurs de base ». La barre était celle
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |d'egui par défaut — mince, invisible au repos, plus large ET plus claire sous le pointeur. Elle
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |est désormais posée dans le scope du panneau, avec ses propres jetons (le jeu n'a pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |d'autocomplétion, c'est le web qui fait référence, comme pour tout ce composant) :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Grandeur | Valeur | Origine |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Largeur au repos | 8px (`AUTOCOMPLETE_SCROLLBAR_WIDTH`) | `::-webkit-scrollbar { width: 8px }`, `styles.css` du web |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Largeur sous le pointeur | 10px | **choix** — le web ne s'élargit pas, l'utilisateur a validé l'effet. *Retiré le soir même, voir la section suivante.* |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Rayon | 4 | `border-radius: 4px` du web |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Teinte, tous états *(au repos seulement depuis la section suivante)* | `HEADING_TEXT` (`#b8b9ba`) | le gris que l'utilisateur voyait au repos (`gray(180)` d'egui), à deux valeurs près le gris unique du jeu — **pas** `SCROLLBAR_THUMB`, mesuré sur le fond noir de la fenêtre Options et invisible sur le brun de la liste (vérifié au pixel) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Poignée minimale | 24px | **choix** — à 115 résultats, les 12 px d'egui donnaient un point |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Rail | aucun | comme dans le jeu. *Ajouté le soir même, voir la section suivante.* |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |La colonne de la barre est réservée (`floating_allocated_width`) : la mention « déjà dans vos
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |alertes » ne passe jamais dessous. Galerie : « Au-delà de cinq rangées — la liste défile ».
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Même soir, hors composant : **l'emplacement d'objet des tuiles d'alerte passe de 44 à 64 px**,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la case du Suivi (`ITEM_SLOT_SIZE`), « dix pixels de plus de chaque côté » — la tuile passe de 88
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |à 110 px de haut pour le loger, le nom garde sa ligne unique (`panels::alerts_tab`).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### La rangée du web, cote pour cote (2026-09-12, nuit)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Quatrième retour du jour, et le plus dense : « les images sont beaucoup plus collées que sur le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |web, ça manque de ce côté aéré » ; « les gemmes ont été très fortement agrandies et aplaties, sur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le web elles sont en 14 × 14 » ; « une ligne fait 35 px sur le web, quelle est la dimension ici ? » ;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le rail « plus sombre que le fond », la barre « sans s'agrandir », sa poignée « de la couleur des
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |éléments survolés » ; le pointeur en main sur les rangées ; et un bug — « le scroll ne se
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |synchronise pas avec les flèches ».
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**La gemme d'abord, parce que c'était un bug et non un choix.** Le composant peint la gemme à son
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rapport natif depuis le premier jour (`glyph_fit(entry.gem_size, 14)`), et la galerie lui passait
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |bien `GEM_NATIVE` (13 × 20). Mais l'onglet Alertes, lui, ne posait que `entry.gem` et laissait
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`gem_size` à sa valeur par défaut, `Vec2::splat(1.0)` — un carré. Une gemme de 13 × 20 se
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |retrouvait donc étirée en 14 × 14 : plus large d'un pixel, écrasée de six. `alerts_tab::texture`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rend désormais la taille du `TextureHandle` avec son id, et la gemme entre dans sa boîte en 9 × 14,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |comme `object-fit: contain` côté web.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**La rangée ensuite.** Elle suivait la cadence du select du jeu (`SELECT_ROW_HEIGHT`, 28 px) avec
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |des écarts de 6 — un choix du 2026-09-11, quand le composant se présentait comme « une extension
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |de `select` ». Sauf que la liste du jeu n'a ni gemme ni image à loger. Réponse à la question posée :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la rangée faisait **28 px** contre **35** sur le web, avec un corps de nom de 15 px contre 13,1
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(0,82 rem). Un rapport unique (15 / 13,1 ≈ 1,14, soit une rangée de 40) aurait grossi la gemme à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |16 alors qu'elle est demandée en 14 : les cotes du web sont donc portées **telles quelles**, le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |corps de 15 restant celui de l'overlay. Relevé sur `wakfu-autocomplete.component.css` :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Cote | Web | Avant | Jeton |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Hauteur de rangée | `height: 35px` | 28 | `AUTOCOMPLETE_ROW_HEIGHT` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Marge gauche et droite | `padding: 0 10px` | 6 | `AUTOCOMPLETE_ROW_PADDING_X` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Boîte de la gemme | 14 × 14 à `left: 10px` | 14 (écrasée) | `AUTOCOMPLETE_GEM_BOX` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Colonne d'image | `width: 30px; margin-left: 20px` | — | `AUTOCOMPLETE_IMAGE_COLUMN`, `_OFFSET` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Image | `[size]="24"` | 22 | `AUTOCOMPLETE_IMAGE_SIZE` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Écart colonne → nom | `gap: 10px` | 6 | `AUTOCOMPLETE_ROW_GAP` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Ce qui donne, de gauche à droite : marge 10, gemme 10..24, colonne d'image 30..60 (image de 24
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |centrée), nom à 70. La bande de filtres garde sa marge de 6 (`AUTOCOMPLETE_FILTER_BAR_PAD`, le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`padding: 6px` de `.wakfu-autocomplete-categories`), qu'elle partageait jusque-là avec la rangée.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**La barre, seconde version.** Le rail existe désormais (`AUTOCOMPLETE_SCROLLBAR_TRACK`,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`#473f30`) : le web fait le sien à 68 % de la surface qui le porte (`#1a1a1a` sur `#262626`), et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |c'est ce rapport appliqué au brun de la liste. Plus d'élargissement — 8 px dans tous les états. La
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |poignée reste grise au repos (`HEADING_TEXT`) et prend `SELECT_ROW_HIGHLIGHT` sous le pointeur et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pendant le glissement (`AUTOCOMPLETE_SCROLLBAR_THUMB_HOVERED`) : egui n'applique `hovered` que le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pointeur **sur la poignée**, pas seulement dans sa colonne. Le pointeur devient une main sur les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rangées sélectionnables et sur les filtres (`cursor: pointer` du web), reste une flèche sur une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rangée désactivée.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le défilement au clavier, et ce qu'il a révélé.** Une flèche appelle maintenant
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`ui.scroll_to_rect(rangée, None)` — le `scrollIntoView({ block: 'nearest' })` de `moveActive()`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |côté web, du strict nécessaire — et la `ScrollArea` est sans animation (`animated(false)`) pour que
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la rangée soit en vue à la frame même. Le test qui le prouve
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(`options_alertes_les_fleches_font_defiler_la_liste` : huit suggestions, six ↓, capture, Entrée) a
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |trouvé un second défaut en passant : **Entrée ne choisissait rien.** Un `TextEdit` à une ligne rend
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le focus sur sa touche de retour, et il est peint avant le panneau — à la frame d'Entrée,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`has_focus()` était déjà faux, le panneau se fermait sans sélection. Aucun test n'avait validé une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |suggestion autrement qu'au clic. Le champ compte désormais comme focalisé pendant la frame où
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Entrée vient de le lui reprendre (`lost_focus() && key_pressed(Enter)`), et si rien n'est
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sélectionnable (entrée désactivée, filtre vide) il reprend le focus pour que le panneau reste.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Même nuit, hors composant : la marge basse des tuiles d'alerte passe de 5 à 18 px, **le même écart
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sous le nom qu'au-dessus de l'emplacement** (`TILE_BOTTOM_INSET = TILE_BADGE_ROW`) ; la tuile fait
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |123 px. *Annulé une heure plus tard, voir ci-dessous.*
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Deux corrections sur le retour (2026-09-12, plus tard dans la nuit)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **La poignée prend la couleur de la liste au repos** (`AUTOCOMPLETE_SCROLLBAR_THUMB =
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  SELECT_LIST_FILL`), plus le gris `HEADING_TEXT` : « c'est peut-être ça qui me perturbait ». Sur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  son rail plus sombre, elle se lit comme un morceau de liste qui glisse dans une gouttière ; sous
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  le pointeur elle garde la teinte des rangées survolées. Une inscription d'un pixel de chaque
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  côté a été évoquée puis retirée dans la même phrase (« non, je n'ai rien dit ») : la poignée
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  fait la largeur du rail.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Le curseur devient une main qui agrippe sur la barre** (`CursorIcon::Grab`, `Grabbing` pendant
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  le glissement). egui ne rend pas la réponse de sa barre : le composant connaît sa colonne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  (`inner_rect.right()` → bord de la liste) et lit l'origine de l'appui pour savoir si le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  glissement en cours y a commencé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **La marge de 18 sous le nom des tuiles était un malentendu.** La demande était l'inverse :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  ramener la marge du HAUT à celle du bas, cinq pixels, « comme ça on va gagner en hauteur ».
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  L'emplacement remonte donc à 5 px du bord, dans la rangée des badges — ils occupent les coins
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  (14 px à 5 px du bord), lui le centre (27 px de chaque côté), ils ne se touchent pas. La tuile
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  fait 97 px (`TILE_SLOT_TOP = TILE_BADGE_INSET`, `TILE_BOTTOM_INSET = 5`), contre 110 avant le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  malentendu et 123 pendant.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce que la galerie ne pouvait pas rattraper — le clic, en vrai (2026-09-12)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |La galerie force le panneau déplié (`preview_open`), donc elle ne prouve rien du **chemin réel** :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le champ prend le focus, puis une suggestion est cliquée. Ce chemin était cassé **des deux bouts**,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |et la seule chose qui l'a montré est un test qui clique et qui tape
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(`options_alertes_champ_d_ajout_trouve_et_ajoute`). Retour utilisateur : « j'ai essayé le champ
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |d'auto-complétion mais celui-ci ne semblait pas fonctionner ».
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |1. **Le panneau ne s'ouvrait jamais.** `design::input` rendait `frame_response.union(edit_response)`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   `Response::union` conserve l'id de l'opérande de **gauche**, et `has_focus()` interroge la mémoire
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   d'egui avec cet id — pas un drapeau que l'union combinerait. L'appelant recevait donc l'id du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   cadre, qui n'est pas focalisable : `has_focus()` était **toujours faux**, ici comme partout
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   ailleurs. Corrigé dans `input.rs`, dans l'autre sens.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |2. **Aucune suggestion n'était cliquable.** Un clic tient en deux frames : l'**appui**, qui retire le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   focus au champ (le pointeur est sur le panneau, pas sur lui), et le **relâchement**, seul moment
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   où egui rend `clicked()` vrai. Une condition d'ouverture réduite à `field.has_focus()` ferme donc
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   le panneau entre les deux : la rangée n'est plus peinte à la frame du relâchement, son clic
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   n'arrive jamais. Le panneau mémorise désormais son rectangle (`ds-autocomplete-panel-rect`) et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   reste ouvert tant que le pointeur est dessus — rectangle **effacé** dès la fermeture, pour qu'un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   reste périmé ne le rouvre pas au simple passage de la souris.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le test sépare volontairement appui et relâchement en deux `run()` : les garder dans la même frame
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |masque exactement ce défaut.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Les jetons
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Tous préfixés `AUTOCOMPLETE_*` dans `design/tokens.rs`, sous un en-tête qui dit explicitement qu'il
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |s'agit d'un **portage du CSS web** et non d'une mesure sur asset : bande 38, bouton de filtre 26
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(icône 20), opacité de repos 0,6 → alpha 153, boîte de gemme 14 (une gemme 13 × 20 y entre en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |9,1 × 14, jamais un `splat`), message vide 34 — et, depuis le 2026-09-12 au soir, la rangée
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |entière du web : 35 de haut, marges 10, colonne d'image 30 à 20 de la marge, image 24, écart 10
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(voir « La rangée du web, cote pour cote »).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::item_slot` — emplacement d'objet (2026-09-11)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/item_slot.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, ItemRarity, SlotCount, SlotFrame};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    design::item_slot()
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .frame(SlotFrame::Rarity(ItemRarity::Legendary))
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .icon(texture_id)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .count(SlotCount::Fraction { current: 137, target: 500 }),
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `frame` | `Rarity(ItemRarity)` / `Plain` | `Plain` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `icon` | `egui::TextureId` déjà résolu | aucune, l'emplacement est peint vide |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `count` | `Simple(i64)` / `Fraction { current, target }` | aucun |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `size` | côté du carré | `ITEM_SLOT_SIZE` = 64 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Textures : les sept `DsTexture::ItemBorder*`, entrées au manifeste avec ce composant.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### L'ordre de peinture EST le composant
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**La bordure de rareté se peint SOUS l'icône. Le cadre simple, PAR-DESSUS.** Ce n'est pas une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |préférence :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- la fenêtre intérieure des `Border-*.webp` **n'est pas un trou transparent** — c'est un aplat
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  semi-transparent (~70 %) teinté par la rareté, vérifié sur les octets décodés. Peinte après
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  l'icône, elle la recouvre entièrement : « j'ai l'impression que tu as mis les objets en opacité »,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  rapporté le jour même de leur arrivée, flagrant sur le jaune-olive du légendaire ;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- un cadre simple est au contraire un **liseré net**, qui doit rester visible si l'icône déborde.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Cet ordre est décrit en **données** (`paint_order`, une fonction libre) plutôt qu'en suite
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |d'instructions, et deux tests le verrouillent. Un bug qu'on rattrape à l'œil une fois ne se rattrape
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pas à chaque relecture.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce que l'appelant fournit, et ce qu'il ne fournit pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |L'icône arrive en `TextureId` **déjà résolu**, et ce n'est pas une entorse à « aucune texture en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |paramètre » : cette règle vise les assets du design system, que le composant doit résoudre depuis
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |une intention. Une icône d'objet est du **contenu** — téléchargée, mise en cache, indexée par le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |catalogue, tout cela hors du design system.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |La **rareté**, elle, est une intention : `ItemRarity` est un type du design system, et c'est au
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |panneau de traduire son `WakfuRarity` métier (`watchlist::to_slot_rarity`) — un composant n'accède
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pas à `overlay_engine`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Deux tailles d'icône, indépendantes
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Cadre | Icône |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `Rarity` | la fenêtre intérieure de la texture (≈ 0,797 du côté), réduite de 4 % |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `Plain` | `ITEM_SLOT_PLAIN_ICON_FILL` ≈ 0,517 — un **rapport** (30/58 mesuré sur le template web), pas une cote : l'icône suit le côté qu'on donne à l'emplacement |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Aucune ne se déduit de l'autre**, et les confondre a produit un défaut réel : la première version
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |faisait occuper tout le carré à l'icône d'un cadre simple. La bordure de rareté masquait le problème
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sur les objets — c'est le snapshot d'une tuile d'**ennemi** qui l'a révélé, avec un monstre deux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |fois trop gros.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Vérification
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Aucun snapshot n'a bougé** : la migration de `watchlist::entry_tile` est équivalente au pixel,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |compteur compris. Quatorze constantes locales ont disparu du panneau, qui ne garde que ce qui lui
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |appartient — résoudre l'icône distante, lire la rareté au catalogue, traduire vers le design system.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |~~**Les cotes restent celles du web**~~ — **passées à celles du jeu le 2026-09-12** : le carré va de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |58 à **64 px** et le rayon des coins de 10 à **2**. Ce qui a décidé la valeur haute de la fourchette
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |relevée (`item_slot_square` 63-64, une mesure pixel d'un bord adouci n'ayant pas de frontière nette)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |est une coïncidence qui n'en est pas une : le liseré des `Border-*.webp` occupe 1/30ᵉ de leur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |canevas, soit **2,1 px rendu à 64** — exactement l'`item_slot_border` relevé sur les mêmes captures.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |À 58 il en faisait 1,9.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le rayon suit `shape.corner_style_inputs_lists` du relevé (« square_or_near_square ») : dans le jeu,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |une case d'inventaire est un carré. 2 plutôt que 0 parce que le contour extérieur des textures de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rareté est lui-même arrondi (rayon ≈ 1/16ᵉ du canevas, ≈ 4 px à 64) — un fond parfaitement
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rectangulaire pointerait hors de ses coins.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**`item_slot_gap` (2 px) n'a PAS suivi**, et c'est délibéré deux fois : cet espacement décrit la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |densité d'une grille d'inventaire, pas celle d'un bandeau de suivi posé par-dessus le jeu (où le 12
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |px vient d'un réglage utilisateur) — et surtout il n'appartient pas au composant : un emplacement ne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |connaît pas son voisin, c'est l'appelant qui espace.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Onze snapshots régénérés (la galerie et les dix du panneau Suivi), dont la **largeur de la fenêtre
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |du Suivi**, qui se calcule sur la taille de tuile. `watchlist::TILE_SIZE` ne porte d'ailleurs plus sa
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |propre valeur : il valait `58.0` en dur, la même que le jeton mais écrite deux fois — le passage du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |composant à 64 aurait laissé le bandeau à 58 sans que rien ne le signale.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |~~**Un doublon daté**~~ — **résorbé le 2026-09-11** : les deux maquettes du testkit
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(`alertes-mockups`, `composants-a-concevoir`) appellent le composant, `UiIcons::item_border` a
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |perdu son dernier appelant et les sept textures ne sont plus chargées qu'une fois. Les **7,3 Mo**
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |payés deux fois (4,9 % du budget) sont rendus. La traduction `WakfuRarity → ItemRarity` a suivi le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |même chemin : elle est passée de `panels::watchlist` à `crate::rarity_bridge`, parce qu'un exemple
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |n'a pas à traverser un panneau pour convertir une rareté.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Journalisation (clause 4)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Sous `tokens::ITEM_SLOT_MIN_SIZE` (8 px = 4 × le liseré), le cadre et sa marge mangent tout le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |carré. L'emplacement est **peint quand même** — §3 veut qu'un rectangle trop petit se voie sur la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |capture plutôt que de paniquer — et un `warn!` part **une fois par instance**, mémorisé sur l'id de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la réponse comme le fait `design::slider` pour ses crans. Le seuil est **choisi, pas mesuré**, et sa
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |doc le dit : personne ne demande sciemment un emplacement de 6 px, c'est le signe d'une largeur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |calculée tombée à rien. La galerie en montre un, à droite de la rangée des cas.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::meter` — jauge (2026-09-11)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/meter.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(design::meter(0.42).width(190.0));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(design::meter(ratio).fill(couleur).width(190.0));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |// Pour un appelant qui pose déjà sa géométrie :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design::paint_meter(ui, rect, ratio, couleur);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `fill` | teinte du remplissage | `METER_FILL` = `#077982` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `width` | largeur imposée | toute la largeur disponible |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `height` | hauteur | `METER_HEIGHT` = 16 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Six couches, et le piège est géométrique
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Là où `item_slot` avait un piège d'**ordre**, la jauge en a un de **géométrie** : chaque couche se
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |déduit de la précédente par un `shrink`, et son arrondi doit décroître d'autant. Écrire les rayons à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la main donne des coins non concentriques — visible sur un arrondi de 4 px.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Couche | Rectangle | Arrondi |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || bordure extérieure | le rectangle donné | 4 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || bordure intérieure | `shrink(2)` | 2 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || piste | `shrink(2)` encore | 0 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || remplissage | fraction de la piste | conditionnel |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || reflet | tiers supérieur du remplissage | coins hauts seulement |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || curseur de fin | 2 px à l'extrémité | 1 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### L'arrondi conditionnel
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Les coins droits du remplissage ne s'arrondissent que s'il atteint le bout de la piste.** Sinon
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |son bord tombe au milieu et un coin arrondi y suggérerait un bord qui n'existe pas. C'est
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`fill_corners`, fonction libre testée : le défaut est invisible sur une jauge pleine ou vide —
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |c'est-à-dire dans les deux cas qu'on regarde en premier.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le seuil de « pleine » est **0,999 et non 1,0** : une fraction calculée en `f32` peut sortir à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |0,9999998 pour un rapport qui vaut exactement un, et la jauge du premier combattant du classement —
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le cas le plus fréquent — afficherait alors un curseur collé au bord droit et deux coins carrés.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Les teintes
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le **remplissage** vient de l'appelant : le panneau Combat fait varier la couleur de sa barre selon
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la part de dégâts. Le **reflet** est fixe (`#0dbebe`), demandé comme un ton précis plutôt que comme
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |une dérivation du remplissage — un éclaircissement automatique a existé, il ne donnait pas ce ton.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Les six couleurs ont été **mesurées pixel par pixel** sur une maquette fournie, après une première
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |tentative approximée à l'œil qui « dénotait du jeu ». Contre-intuitif et conservé tel quel : la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |bordure *extérieure* est un gris moyen, c'est l'*intérieure* qui est presque noire.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Vérification
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Aucun snapshot n'a bougé** : la migration de `combat::damage_bar` est équivalente au pixel. Sept
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |constantes disparaissent du panneau, qui ne garde que le calcul de la part de dégâts et sa teinte —
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |du métier, que le composant ne saurait pas faire.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |---
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::portrait` — portrait de combattant (2026-09-11)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/portrait.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, PortraitShape};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    design::portrait(texture_id)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .shape(PortraitShape::Round)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .size(48.0)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .dimmed(fighter.is_ko)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .percent(Some(42)),
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |// Pour le gabarit à six emplacements, qui pose déjà ses centres :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design::paint_portrait(ui, rect, texture_id, PortraitShape::Round, dimmed);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design::paint_portrait_percent(ui, rect, design::portrait_percent(dmg, total));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `shape` | `Square` / `Round` | `Square` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `size` | côté du carré englobant | 40 (la liste plate) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `dimmed` | applique le grisé KO | `false` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `percent` | `Option<i64>` incrusté au coin | aucun |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Deux formes, parce que le jeu en a deux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le gabarit de combat loge ses portraits dans des **médaillons ronds**, la liste plate — celle qui
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |prend le relais au-delà de six alliés — les pose **carrés**. `PortraitShape::corner_radius(size)`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |porte le calcul plutôt que de le laisser à chaque appelant : `taille / 2` écrit à deux endroits finit
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |par diverger d'un pixel.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Le grisé est une approximation, et c'est l'appelant qui décide
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Une teinte egui **multiplie** : elle assombrit sans désaturer, là où un vrai niveau de gris
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |désature. Les portraits de classe ont leur version grise **précalculée** dans l'atlas et n'ont donc
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pas besoin de la teinte — la leur serait moins bonne. Une icône de monstre téléchargée ou le repli
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |générique n'ont pas d'équivalent gris et s'en contentent.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le composant ne peut pas trancher : seul l'appelant sait laquelle des trois textures il tient. D'où
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`dimmed` en paramètre plutôt qu'une déduction depuis un `is_ko` que le composant ne verrait pas.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Le pourcentage déborde du carré, volontairement
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Il se pose au coin bas-droit du **carré englobant**, décalé encore de 2 px à droite et 1 en bas :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |« comme si on traçait un carré autour du rond et qu'on plaçait le pourcentage tout en bas à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |droite », puis « encore un peu plus sur la droite pour qu'il mange un peu moins sur le portrait ».
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Sur un portrait rond, ce coin est hors du disque — c'est précisément ce qu'on veut.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Il prend `OVERLAY_ACCENT` et **non la teinte de la jauge**, après un aller-retour : les deux ont été
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |alignées un temps, puis re-séparées (« je préfère la couleur accent qu'il y avait avant »).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`portrait_percent(damage, total)` est une fonction libre testée : **un total nul est le cas réel du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |tout début d'un combat**, et une division par zéro y produirait un `NaN` qui se propage jusqu'au
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |texte peint — « NaN% » sur un portrait.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Vérification
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Aucun snapshot n'a bougé** : la migration de `combat::paint_flat_portrait` et des deux boucles du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |gabarit est équivalente au pixel.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::table` — tableau (2026-09-12)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/table.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, TableAlign, TableBody, TableColumn};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |design::table()
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .column(TableColumn::fixed("Date", 104.0))
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .column(TableColumn::flex("Nom", 1.0))
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .column(TableColumn::fixed("Prix", 108.0).align(TableAlign::End))
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .body(TableBody::Rows(offres.len()))
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .max_height(12.0 * design::tokens::TABLE_ROW_HEIGHT)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .empty_text("Aucune vente sur la période")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .log_name("hdv.historique")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .show(ui, |row| {
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        let offre = &offres[row.index()];
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        row.cell(|ui| { ui.label(&offre.date); });
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        row.cell(|ui| { ui.label(&offre.nom); });
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        row.cell(|ui| { ui.label(&offre.prix); });
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        if row.response().clicked() { /* l'appelant décide */ }
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    });
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `column` / `columns` | `TableColumn::fixed(label, px)`, `::flex(label, poids)` ou `::numeric(label, px)`, `.align(…)` | aucune colonne |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `body` | `Rows(n)` / `Empty` / `Loading` | `Empty` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `empty_text` | message du corps vide | aucun — le corps reste vide, comme dans le jeu |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `row_height` | hauteur d'une ligne | 60 (la cote du jeu) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `width` | largeur totale | celle du `Ui` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `max_height` | borne du **corps** : au-delà il défile, en-tête figé | aucune |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `preview_loader_frame` | fige le rouage de `Loading` | horloge |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Conteneur de la famille §1 bis, **forme closure**. `Table::height()` rend la hauteur totale *avant*
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le rendu : c'est ce qui permet à un appelant de peindre quelque chose derrière le tableau, ou de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |réserver sa place.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Trois cotes mesurées, et tout le reste vient de l'appelant
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Relevé : [`hdv-table.json`](design-system/hdv-table.json). Hauteur de ligne **60 px**, encre
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |d'en-tête **14 px**, écart encre → première ligne **7 px** — les trois invariants sur les trois
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |captures HDV. L'écart de 7 px est **la même valeur que `HEADING_TO_ROW`**, mesurée indépendamment
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sur une autre interface, et la teinte des libellés (`#b9babb`) est celle des titres de section
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(`#b8b9ba`) à un canal près : deux jetons partagés plutôt que deux quasi-doublons qui dériveraient.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |L'en-tête est en **linéale**, pas dans la serif des titres — vérifié sur la capture agrandie ×4,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |c'est le genre de détail qu'aucune mesure numérique ne donne. Corps 17, contrôlé au rendu : 12 px
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |d'encre et 36 px de large pour « Date », contre 12 et 37 dans le jeu.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Les trois tableaux relevés sont vides** (« 0 Objet »). Rien de ce qui concerne une ligne remplie
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |n'est mesurable : alignement des valeurs, typographie des cellules, icône d'objet de la colonne Nom,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |texte trop long. Le composant n'en invente rien — il **donne la cellule à l'appelant** et ne peint
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |aucun contenu. Seul `TABLE_CELL_PAD_X` est un choix, emprunté à `SELECT_PADDING_X`, et il l'annonce.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Les nombres à droite, le texte à gauche
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`TableColumn::numeric(label, px)` pose l'alignement à droite, **en-tête compris**. C'est une règle,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pas un raccourci d'écriture&nbsp;: des nombres alignés à gauche se comparent mal, les unités ne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |tombent plus les unes sous les autres et « 980 » paraît plus long que « 145 000 ». Un texte ou une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |date se lisent depuis leur début et restent à gauche. La règle vit dans le constructeur pour
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |qu'aucun tableau de l'overlay ne l'oublie&nbsp;; `fixed` + `align` reste disponible pour l'exception.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**`Layout::with_main_align` ne suffisait pas.** Sur une `Ui` dont le rectangle est imposé, egui pose
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |quand même le premier widget au départ de son curseur&nbsp;: la colonne « alignée à droite » sortait
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |collée à gauche. Il faut changer le sens de parcours (`right_to_left` pour `End`,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`centered_and_justified` pour `Center`). Mesuré après correction&nbsp;: « 12 400 » et l'en-tête
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |« Prix » finissent tous deux à x=725. Conséquence à connaître&nbsp;: dans une cellule à droite,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |plusieurs widgets s'empilent **de droite à gauche**, le premier posé étant le plus à droite.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Le zébrage éclaircit, il ne colore pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le tableau du jeu n'a **pas de fond propre** : le décor se lit à travers. Une ligne sur deux porte
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |donc un blanc translucide, jamais deux aplats opaques — sur un overlay posé par-dessus un jeu en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |mouvement, deux aplats seraient faux à chaque frame.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |L'alpha est déduit colonne par colonne, `(claire − nue) / (1 − nue/255)`, sur quinze colonnes de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |x=60 à x=1180 : **médiane 12,3**, valeurs de 10,1 à 16,2. La dispersion est celle du décor, pas de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la mesure. Deux colonnes seules donnaient 13,5 : l'échantillon comptait.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |La galerie le démontre sur **six bandes de fond de luminances différentes** — sur un fond uni, la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |démonstration serait invisible. Ce bloc est **relégué en fin de section** depuis un retour
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |utilisateur du 2026-09-12 («&nbsp;on a l'impression que c'est un damier&nbsp;»)&nbsp;: il démontre
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |bien ce qu'il doit démontrer, mais il rend les lignes illisibles. Les cas d'usage se jugent donc sur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |le fond uni de la planche, et la mesure garde son bloc à part, annoncé comme tel.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Les positions de colonne sont imposées par le composant, et c'est un constat du relevé
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Les six libellés ont partout la même largeur d'encre d'une capture à l'autre, mais leurs abscisses
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |varient avec la largeur du tableau **sans règle lisible** : entre « Enchantement » et « Quantité »
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'écart vaut 160 px dans deux captures sur trois, ailleurs rien ne se répète. Le relevé conclut
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |qu'il faut soit une quatrième capture, soit que le composant impose sa propre répartition. C'est ce
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |second choix, et il est **en données** : `table_column_spans(colonnes, largeur)` — fixes d'abord,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |reste au prorata des poids élastiques, réduction proportionnelle si les fixes ne tiennent pas — avec
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |quatre tests qui le verrouillent. Sans aucune colonne élastique, le reste **demeure à droite** :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |une largeur imposée l'est vraiment.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Les deux états que le jeu ne montre pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`Empty` et `Loading` sont une **décision de l'overlay**, pas un relevé : les tableaux à « 0 Objet »
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |du jeu sont simplement vides, sans message, et aucune capture ne montre un tableau en chargement.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Les deux sont construits avec des éléments déjà mesurés — le rouage de `design::loader`, le gris de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`TEXT_DISABLED` — plutôt qu'avec des teintes inventées, et leurs hauteurs sont annoncées comme
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |choisies dans les jetons. `empty_text` reste facultatif : sans lui, le corps garde sa hauteur et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |demeure vide, ce que fait le jeu.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### L'identité pend à la réponse du corps
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Une `Ui` fille créée sans sel d'identité **hérite de l'identifiant de sa mère**. Deux tableaux posés
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |dans le même panneau donnaient donc les mêmes identifiants de ligne, et egui l'écrivait en rouge sur
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la capture (« Second use of widget ID … ») — c'est la capture qui l'a montré, aucune relecture ne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'aurait signalé. Tout pend désormais à l'identifiant automatique de la réponse du corps, unique par
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |position dans l'arbre : lignes, cellules, barre de défilement et avertissement de débordement.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce que le tableau ne peint PAS
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |La bande claire de 8 px sous le tableau, que le relevé attribuait à un « liseré bas ». Mesure de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |contrôle : elle traverse **toute la largeur de la capture** (x 0..1278), bien au-delà des bornes du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |tableau (x 24..1262). Elle appartient au décor de la fenêtre — le relevé a été corrigé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |La **pagination** n'en fait pas partie non plus : en bas dans Historique et Rechercher, **en haut à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |droite** dans Mes offres. C'est un composant autonome que la page place, pas un pied de tableau.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Vérification
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Snapshot **`design_gallery_table.png`** — et c'est une *seconde* planche, pas un choix de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |présentation : `wgpu` refuse une texture de plus de 8192 px de côté et `design_gallery.png` en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |occupe déjà 7530. Six cas : peuplé (avec un nom trop long, coupé à la colonne), vide avec message,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |vide muet, en chargement, corps borné défilant, et le cas dégénéré (268 px de colonnes imposées dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |200 px) — puis le bloc de contrôle du zébrage sur fond en bandes.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::pagination` — pagination (2026-09-12)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/pagination.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, PaginationStep};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |match design::pagination(page, total).log_name("hdv.historique").show(ui).step {
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    Some(PaginationStep::Previous) => page -= 1,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    Some(PaginationStep::Next) => page += 1,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    None => {}
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |}
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `pagination(page, total)` | affichés tels quels — le composant ne renumérote rien | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `log_name` | nom d'instance | `pagination` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `preview_hovered` | force le survol d'une flèche (galerie) | horloge réelle |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Composant feuille, mais il rend un `PaginationOutcome` et non une `Response`&nbsp;: deux flèches,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |deux intentions distinctes qu'une seule `Response` ne saurait pas dire. `Pagination::height()` vaut
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`ICON_BUTTON_SIZE`&nbsp;; la largeur est celle du libellé, jamais une valeur figée.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce n'est pas un pied de tableau
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |En bas dans Historique et Rechercher, **en haut à droite** dans Mes offres. C'est un constat du
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |relevé, pas une préférence&nbsp;: le bloc alloue sa largeur naturelle et laisse la mise en page à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |son appelant.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Trois choses que la capture a apprises, contre le relevé
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |1. **Ce ne sont pas des flèches nues.** Le relevé décrivait « deux flèches de 7 px espacées de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   39 px ». Agrandie ×5, la zone montre **deux boutons icône de 36 × 36** — socle arrondi, hachures
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   diagonales — séparés de 4 px. Le composant n'en peint donc aucun&nbsp;: il compose deux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   `design::icon_button` en contexte panneau.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |2. **Le numéro courant est doré, pas blanc.** Mesuré (244, 216, 158) sur ses pixels pleins, la même
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   valeur que « Page ». Seuls la barre oblique et le total sont blancs — *ce qui bouge est en or, ce
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   qui borne est en blanc*.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |3. **Le glyphe pointe vers la gauche**, malgré son nom de fichier (`icon-triangle-right`). Vérifié
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   sur son canal alpha&nbsp;: pointe en x=0, base en x=6. C'est donc « suivant » qui est retourné.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   La première capture a rendu les deux flèches à l'envers — aucune relecture ne l'aurait dit.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Un glyphe détouré d'un bouton n'est pas forcément sur la grille de 18
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`DsIcon::TriangleRight` était déclaré `socle`, donc normalisé à `ICON_BUTTON_CONTENT` (18 px
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |d'encre). Mesure directe sur la pagination du jeu&nbsp;: **8 × 10 px d'encre dans un socle de 36**,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |soit la taille native de l'asset (7 × 10) à un pixel de détourage près. La normalisation
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'agrandissait de 80 %, ce que la première capture a montré sans ambiguïté. Passé à `libre` — et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |c'est le registre qui apprend quelque chose&nbsp;: `--from-button` dit d'où vient le détourage, pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |que le glyphe soit sur la grille.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Le corps a été réglé au rendu, pas par le calcul
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |13 px de hauteur de **capitale** dans le jeu (le « P » de « Page ») — pas les 16 px d'encre totale,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |qui incluent le jambage du « g » et donneraient un corps faux d'un tiers. Le rapport d'encre habituel
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(0,805) donnait 16&nbsp;; au rendu, 16 ne produit que 11 px d'encre et **19 en produit 13**. Contrôle
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sur le segment entier «&nbsp;Page 0 / 0&nbsp;»&nbsp;: 41 px pour « Page » contre 42 dans le jeu,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |83 px pour le libellé entier contre 82.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce qui reste inconnu
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Les deux flèches sont grisées sur les trois captures** (« Page 0 / 0 » — le jeu n'a aucune page à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |parcourir). L'apparence d'une flèche *active* n'existe nulle part&nbsp;: le composant laisse
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`icon_button` rendre ses états habituels plutôt que d'inventer une teinte.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le socle **désactivé**, lui, est mesurable et diverge nettement&nbsp;: le jeu le peint à (36, 37, 41)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sur un fond à (28, 30, 34), là où le contexte `Panel` pose `ButtonIconDisabled` en pleine opacité, à
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |65. L'asset a été détouré d'un écran plus clair et rien ne le ramène au fond sur lequel il est posé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**C'est un écart d'`icon_button`**, qui vaut pour ses quatre boutons désactivés — le corriger dans la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |pagination créerait un second réglage du même socle.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Vérification
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Snapshot `design_gallery_table.png`, section basse&nbsp;: les quatre positions possibles (0/0, 1/12,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |6/12, 12/12), un survol forcé, et un total à quatre chiffres qui élargit le bloc.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`design_gallery.png` bouge aussi, du seul fait de la renormalisation de `TriangleRight`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## À faire — composants identifiés, pas encore écrits
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Inventaire refait le 2026-09-10 à partir des assets de `assets/design-system/` (55 fichiers sur 85
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ne sont pas encore au manifeste, dont 28 icônes), des captures d'interfaces du jeu
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(`assets/design-system/interfaces/`) et du code des panneaux. Classement **par vague** et non par
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |fréquence : chaque vague fournit ce dont la suivante a besoin.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Les mesures citées comme « déjà relevées » existent dans [`design-tokens.json`](design-tokens.json)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ou dans `docs/design-system/releve-*.json` — c'est du relevé fait, pas du travail à refaire.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Vague 1 — la structure
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Fait disparaître la mise en page absolue. Rien d'autre ne devrait être écrit avant : aujourd'hui,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`panels::options_modal` porte 30 constantes de mise en page et 10 `egui::Rect::from_*` calculés à la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |main, soit environ 250 lignes qui ne font que placer des rectangles.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Composant | Ce qu'il absorbe | Matière disponible |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`design::tooltip`** | `panels::tooltip` et ses trois enveloppes (`combat::show_tooltip_above`, `watchlist::show_tooltip_left`/`_right`), placement et replis compris. | `TOOLTIP_MARGIN`, `TOOLTIP_GAP`, `TOOLTIP_BG_FILL` mesurés. |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`design::icon` + `DsIcon`** — *décidé le 2026-09-10, voir ci-dessous* | Le registre des 34 glyphes, séparé des fonds 9-slice : taille d'encre au manifeste, teinte par jeton. | 28 icônes détourées et inutilisées dans `icons/` ; `tokens::ICON_TINT`/`ICON_TINT_HOVER` et `DsTexture::icon_content_size` existent. |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**`DsIcon` est un type distinct de `DsTexture`** (décision utilisateur, 2026-09-10).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Motif révisé le même jour**, après les commits `0923a45`, `a372320` et `4762cee` d'une session
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |parallèle. L'argument d'origine — « chaque glyphe a une taille d'encre propre, un fond 9-slice n'en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |a pas, et peindre `icon-minus` (14×2) dans un carré l'étirerait » — **ne tient plus** :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`icon_button::glyph_fit(native, box_side)` met désormais tout glyphe à l'échelle **en préservant son
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ratio natif**, et `Input::leading_icon` partage la même règle. Le problème d'échelle est réglé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Ce qui reste, et qui justifie encore la séparation :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **`TextureSpec` est un type à deux visages.** Il porte un `slice: NineSlice` dont aucun glyphe ne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  se sert (tous prennent `ICON_SLICE`, marges nulles — un 9-slice dégénéré), et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  `icon_content_size()` est un `match` qui énumère à la main les variantes qui se trouvent être des
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  icônes. Déclarer une icône demande donc de penser à un champ qui ne la concerne pas, et d'ajouter
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  une ligne à une liste qui n'a pas de garde-fou.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **12 icônes au manifeste sur 34 disponibles** dans `assets/design-system/icons/`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Ce qui a changé dans l'appréciation : c'est désormais un **nettoyage de typage**, pas un déblocage.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Rien n'en dépend — ni la vague 1, ni la vague 2. À faire quand le terrain est libre, et **pas en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |parallèle** d'une session qui travaille sur `assets.rs` / `icon_button.rs` / la galerie : le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |refactor touche exactement ces trois fichiers.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Note sur le rythme : ajouter une icône au manifeste **quand un composant en a besoin** — ce que fait
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la session parallèle — est sain, et vaut mieux que déclarer les 34 d'un coup. Un manifeste ne doit
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rien contenir de mort.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Exécution, le jour venu :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |1. `design/icons.rs` — `enum DsIcon` et sa table (fichier, taille d'encre, nom de texture).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |2. Retrait des variantes d'icône de `DsTexture` et de `icon_content_size` avec elles — `DsTexture`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |   ne décrit plus que des fonds 9-slice.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |3. `design::icon` — une **feuille** au sens du contrat, qui réutilise `glyph_fit` tel quel.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |4. `icon_button` et `Input::leading_icon` prennent un `DsIcon`.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |5. La galerie des glyphes (`a372320`) suit le nouveau type ; snapshots régénérés.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Coût mémoire, mesuré avant de décider : les 34 icônes décodées en RGBA pèsent **31 Ko** au total —
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |sans effet sur le budget de 300 Mo (§8 du plan).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le contrat de composant a désormais deux familles** (décision utilisateur, 2026-09-10) : une
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**feuille** implémente `egui::Widget` ; un **conteneur** — celui qui encadre du contenu fourni par
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'appelant — expose un `show` générique sur le retour de ce contenu. Toute la vague 1 relève de la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |seconde famille. Le critère de choix et les deux formes admises sont dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |[`contrat-composant.md`](../.claude/skills/ui-component/references/contrat-composant.md) §1 bis.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`panels::options_modal::chrome` (extrait le 2026-09-10) est déjà un conteneur au sens de ce
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |contrat, écrit dans un panneau faute d'endroit où le mettre : c'est lui qui remonte dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`design::window`, sans changer de forme.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Vague 2 — les formulaires
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Ce qu'il faut pour que les onglets Alertes et Personnages de la modale Options existent.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Composant | Ce qu'il absorbe | Matière disponible |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`input`** — variantes `Number`, `Search`, état d'erreur | *Extension du composant existant*, pas un second composant (skill `ui-component`, étape 1). | `input-number.png`, `input-search.png`, `empty-input-search.png`, `large-input-*.png` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`design::field`** | Rien aujourd'hui — le *libellé à gauche, contrôle à droite* du jeu (« Prix unitaire », « Quantité », « Durée de publication »). **À ne pas confondre** avec la ligne « contrôle élastique + bouton » de la modale Options, qui n'a pas de libellé et n'a qu'un seul usage. | Captures `interface-hdv-vente-form.png` ; **relevé `ui-blueprint` d'abord**, aucune cote n'existe. |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`design::slider`** | Rien aujourd'hui. | Aucun asset découpé — passer par `design-asset` d'abord ; captures dans `interface-options-son.png` et `interface-options-interface.png`. |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`tabs`** — variante icône | Onglets à pictogrammes. | `icon-tabs.png` ; dépend du registre `DsIcon` (vague 1). §5.7. |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Vague 3 — les données
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Les composants qui portent ce que l'overlay affiche réellement.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le vocabulaire visuel de ces panneaux est tranché** (décision utilisateur, 2026-09-10) : les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |panneaux Combat et Suivi **prennent les formes et la typographie du jeu, et gardent leur accent
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |cyan**. Ce n'est pas un compromis mou, c'est le seul point où l'overlay a une contrainte que le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |jeu n'a pas : il se lit **par-dessus** le jeu, sur un fond arbitraire et mouvant. Le cyan
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`#00d2ff` n'existe nulle part dans l'interface Wakfu — c'est précisément ce qui l'empêche de s'y
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |confondre. Une jauge de dégâts or posée sur un décor or se cherche.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Concrètement, trois familles à reprendre, et rien d'autre :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Ce qui migre | Aujourd'hui | Cible |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **Les polices** | la proportionnelle par défaut d'egui — une Ubuntu *Light*, plus maigre que tout ce que le jeu écrit. Neuf appels `FontId::proportional`, plus deux `FontId::monospace` sur les compteurs de tuile. | `design::text::label_font` / `title_font` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **Les formes** | rayons hérités du CSS : tuile 10 px, carte de toast 12 px, bandeau 6 px. | emplacement du jeu — carré, bordure 2 px, rayon 0–2 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **L'accent — reste** | cyan `#00d2ff` dispersé en constantes locales de `panels::combat` et `panels::watchlist`. | **un jeton nommé**, `tokens::OVERLAY_ACCENT`, documenté comme le vocabulaire du contenu flottant — plus un vestige du portage |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Ce qui est **déjà** au langage du jeu et qu'il ne faut pas toucher : les cadres de portraits
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(gabarits du jeu), les quatre boutons du carré de contrôle (`design::icon_button`, socle
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`button-icon-first-plan.png` du jeu), les infobulles, les sept bordures de rareté.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |La conséquence pour les composants ci-dessous : **un seul jeu de composants, une seule dimension de
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |galerie**. `meter`, `badge` et `item_slot` prennent leur teinte d'un jeton — `OVERLAY_ACCENT` pour
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ce qui flotte, les jetons du jeu pour ce qui vit dans une fenêtre — jamais d'un paramètre de thème.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Composant | Ce qu'il absorbe | Matière disponible |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`design::item_slot`** | `watchlist::entry_tile` — bordure de rareté, icône, compteur incrusté, et l'ordre de peinture dont l'inversion a déjà produit un bug. | 7 `Border-*.webp`, `rarity_borders` (7 raretés), `item_slot_square` 63–64 / `border` 2 **appliqués** (2026-09-12) ; `gap` 2 laissé à l'appelant. |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`design::badge`** | `watchlist::paint_count_inline`, les étiquettes de rareté, la pastille d'état. | `status_pill_active` ; `text::OUTLINE_FULL` existe. §5.12, §5.13. |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`design::meter`** | `combat::damage_bar` — 68 lignes de rectangles empilés (bord externe, bord interne, piste, remplissage, reflet, curseur de fin, arrondis conditionnels). | Six couleurs mesurées dans `combat.rs`, à promouvoir en jetons. |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`design::portrait`** | `combat::paint_flat_portrait`, `panels::combat_frame` et son gabarit à six emplacements. | `crates/overlay-ui/assets/templates/*.png`, atlas de classes, portrait de repli. |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Vague 4 — les finitions
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Ni urgent ni structurant, mais chacun retire du code d'un panneau.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Composant | Ce qu'il absorbe | Matière disponible |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`design::toolbar`** | Le carré de contrôle du Suivi : fond translucide, gouttière, groupement de boutons icône. | `watchlist::PANEL_BACKDROP_FILL`, `menu-button-icon-first-plan.png` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`design::segmented`** | Le bascule « Objets mis en vente / Offres d'achat » ; le switch Alliés/Ennemis du panneau Combat en est une variante maison. | `tabs-with-first-tab-active.png` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`design::toast`** | `watchlist::toast_card` et ses confettis — ~300 lignes, avec son générateur pseudo-aléatoire maison. | Portage du web ; aucun asset de jeu correspondant. |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || **`design::dialog`** | Rien — la boîte de confirmation qui manquera à la première action destructrice de l'overlay. | `interfaces/interface-confirm-box.png` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Écarts restants
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Trois écarts avaient été constatés le 2026-09-10. Revérifiés le 2026-09-11, il en reste **un et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |demi** :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- ~~**`modal-header.png` dupliqué octet pour octet**~~ — **résolu** par `5ccf0d2` (« refactor :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  bannière de modale au manifeste »). La copie `crates/overlay-ui/assets/ui/options/` n'existe plus,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  `options_modal` passe par `DsTexture::ModalHeader`, et le manifeste documente la résorption.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Deux barres de défilement**, mais ce n'est **pas un doublon accidentel** :
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  `design::scroll_area` (poignée 6 px, `#515356` / `#c1ad83`, aucun rail) et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  `panels::combat_frame_scroll` (barre 5 px, `#998a6c`, bordure noire alpha 217, rayon 2). La
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  seconde est une **divergence assumée** : sa finesse et sa couleur unie viennent d'un retour
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  utilisateur explicite (la texture étirée cachait les portraits, voir la doc de module). Les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  unifier demanderait donc une variante du composant, pas une suppression — et cette variante
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  attend une décision, pas un nettoyage.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Le socle désactivé d'`icon_button` est trop clair sur un fond sombre** (constaté le 2026-09-12
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  en écrivant `design::pagination`). Le jeu peint le socle d'une flèche grisée à (36, 37, 41) sur un
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  fond à (28, 30, 34) — huit niveaux au-dessus de son fond. Le contexte `Panel` pose
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  `ButtonIconDisabled` en pleine opacité, à 65 : l'asset a été détouré d'un écran plus clair, et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  rien ne le ramène au fond sur lequel il est posé. Le contexte `FirstPlan` a déjà rencontré ce
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  problème et le traite en gardant son socle de repos sous `DISABLED_DIM` ; `Panel` ne l'a pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  encore. À corriger dans `icon_button`, pour ses quatre boutons désactivés à la fois — jamais dans
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  un composant appelant, qui créerait un second réglage du même socle.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |- **Cinq chemins de chargement de texture** (et non six : `options_modal` est passé au manifeste) —
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  `DesignSystem::load`, `ui_icons`, `portraits`, `remote_icons`, `combat_frame` — dont quatre copies
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  de la même fonction décoder → `ColorImage` → `load_texture`. Le budget mémoire (§8 du plan,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |  300 Mo) n'a donc toujours aucun point de mesure unique. C'est l'écart qui reste entier.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce que ce catalogue devrait porter en plus
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Une colonne **« qui le consomme en production »** par composant. La fiche du bouton icône la porte
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |déjà (« les quatre boutons du carré de contrôle du Suivi ») ; les autres non. C'est cette colonne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |qui répond, dans six mois, à « puis-je changer ce jeton sans rien casser ? ».
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::confirm_dialog` — boîte de confirmation (2026-09-12)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/confirm.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design::{self, ConfirmChoice};
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |match design::confirm_dialog("Retirer « Pierre ultime » de vos alertes ?")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .over(fenetre)                       // la FENÊTRE entière, pas le panneau appelant
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .log_name("alertes.retrait")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    .show(ui)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |{
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    ConfirmChoice::Yes => retirer(),
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    ConfirmChoice::No => fermer(),
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    ConfirmChoice::Pending => {}
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |}
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `confirm_dialog(question)` | la question posée | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `over` | ce que le voile couvre et sur quoi la boîte se centre | `ui.max_rect()` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `yes` / `no` | libellés des deux réponses | « Oui » / « Non » |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `log_name` | nom d'instance | `confirm` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Composant feuille, mais il rend un `ConfirmChoice` et non une `Response`&nbsp;: une `Response` ne
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |saurait pas dire *laquelle* des deux réponses a été cliquée, et ressortir le choix par un `&mut` en
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |paramètre est la maladresse que §6 reproche ailleurs.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Le voile n'est pas une teinte, c'est une information
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Tant que la boîte est ouverte, ce qu'elle couvre est **inerte** — et c'est pourquoi `over` demande
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |la fenêtre entière, pied de page compris. Un voile rogné au panneau appelant laisserait bannière,
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |onglets et boutons à pleine luminosité, ce qui se lit comme « ils restent cliquables ». Le composant
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |va plus loin que l'apparence&nbsp;: il **avale** les clics qui passent à côté de la boîte, pour que
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'inertie annoncée soit réelle.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Ce n'est pas une popover ancrée au bouton
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le dépôt web a `ConfirmDeleteService`, collé au bouton déclencheur. Le jeu a sa propre boîte, et
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |elle est dans les captures de référence&nbsp;: `interface-confirm-box.png` (449 × 209) pose
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |exactement la même forme de question. Boîte autonome et centrée, fond gris **clair** (`#585955`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |mesuré), médaillon en crête débordant le corps.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Le centrage règle du même coup une réserve d'ergonomie&nbsp;: une popover recouvrait le bouton
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |« Valider » de la fenêtre, et son bouton de confirmation tombait exactement là où « Valider »
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |réapparaissait une fois la popover fermée — un double-clic un peu vif validait la fenêtre.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Le bouton destructeur du jeu est or, jamais rouge
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`docs/design-system.md` réserve nommément le rouge au bouton « Annuler » pleine largeur d'un pied
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |de fenêtre, et précise que « le bouton "Annuler" d'une boîte de dialogue simple reste kaki/gris
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |standard, pas rouge ». Un « Retirer » rouge est la convention web du *destructive action*.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Échap répond « Non »**, jamais « Oui »&nbsp;: une touche ne confirme pas une action destructrice.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |L'appelant, lui, doit s'abstenir de lire cette même touche tant qu'un dialogue est ouvert — sinon le
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |même appui ferme la boîte ET la fenêtre derrière.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Vérification
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Sa propre planche de galerie (`design_gallery_confirm`), et non une section des deux autres&nbsp;:
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |son voile couvre tout ce qu'on lui donne, donc posé dans un canevas de 7530 px il assombrirait la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |galerie entière. La planche peint exprès du contenu dessous — titre, champ, deux boutons — parce
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |que c'est **ce qu'il assombrit** qu'il faut juger.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Deux appelants, et c'est ce qui l'a fait naître
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Peint d'abord dans la maquette de la page Alertes, puis dans `panels::alerts_tab` au portage. Remonté
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |au design system le jour où la **garde de fermeture** de la fenêtre Options lui a donné un second
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |appelant. L'extraction est **à pixel constant** — aucun des trois snapshots de l'onglet Alertes n'a
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |bougé.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## `design::label` — libellé élidé (2026-09-12)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`crates/overlay-ui/src/design/components/label.rs`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |use overlay_ui::design;
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.add(
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |    design::label("Plan \"Epée de Brâkmar\"")
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .width(108.0)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .align(egui::Align::Center)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |        .log_name("alertes.nom"),
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |);
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || Paramètre | Valeurs | Défaut |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || --- | --- | --- |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `label(text)` | le texte à afficher | — |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `width` | largeur imposée — le texte s'élide au-delà | largeur disponible du `Ui` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `size` | corps | `LABEL_FONT_SIZE` = 13 |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `color` | couleur du texte | `LABEL_TEXT` = blanc |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `align` | `Min` / `Center` / `Max` dans la largeur | `Center` |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) || `tooltip_side` | côté de l'infobulle | au-dessus |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Composant feuille (`impl Widget`). Deux fonctions associées pour l'appelant&nbsp;: `Label::height`
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |(réserver la place avant d'ajouter) et `Label::elides` (voir plus bas).
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Une ligne, jamais deux — et c'est une décision produit
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Un nom d'objet Wakfu dépasse souvent la largeur disponible&nbsp;: les quatre « Plan "Epée de … " »
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |partagent leurs quatorze premiers caractères. Le réflexe est de le faire passer à la ligne pour les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |distinguer, et c'est **ce qui a été essayé puis annulé le 2026-09-12**&nbsp;: la tuile porte déjà
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |l'icône de l'objet, et c'est elle qui lève l'ambiguïté d'un coup d'œil, bien avant le texte. Faire
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |grandir chaque tuile pour distinguer deux libellés résolvait un problème que l'utilisateur n'a pas.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |> « Ton problème de nom, c'est un faux problème puisque l'utilisateur voit des images en plus des
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |> noms. Pour lui, il y a beaucoup moins d'interprétation que toi, seulement avec des noms. »
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Ce qui manque quand un nom est coupé, ce n'est pas de la place&nbsp;: c'est **un moyen de lire la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |suite**. D'où l'infobulle.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### L'infobulle n'apparaît que si le texte est réellement coupé
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Un nom qui tient en entier n'a rien à révéler — une infobulle qui répète ce qui est déjà lisible est
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |du bruit. Même règle que le dépôt web (`[tooltipOnlyIfTruncated]="true"`), et c'est **egui** qui
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |répond ici (`Galley::elided`), pas une comparaison de chaînes qu'un nom finissant déjà par « … »
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |mettrait en défaut. L'infobulle est celle du design system, pas le `on_hover_text` d'egui&nbsp;:
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |même socle, même police, même délai que partout ailleurs.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### `Label::elides`, et la zone muette qu'elle évite
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Un appelant qui pose sa **propre** infobulle sur une zone plus large — une tuile, une ligne — doit
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |s'effacer là où celle du libellé s'affichera&nbsp;: deux infobulles sous le même curseur se
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |peignent l'une sur l'autre. Mais s'effacer *partout* sur le libellé laisse une **zone muette** au
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |milieu de la tuile dès que le nom tient en entier, puisque le libellé ne dit alors rien non plus.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |C'est arrivé, le temps d'une capture.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`Label::elides(ui, text, width, size)` répond à la seule question qui tranche. Les galleys étant
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |mémoïsés par `Fonts`, l'appeler ne remet pas le texte en page une seconde fois.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |### Vérification
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Section de galerie&nbsp;: le **même texte à trois largeurs** — l'ellipse vient du rapport entre les
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |deux, pas du texte — et les trois alignements. L'infobulle, elle, ne peut pas s'y voir&nbsp;: elle
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |demande un curseur, et aucun ne survole quoi que ce soit en rendu offscreen. C'est
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`tests/panels.rs` qui la vérifie, par survol simulé, **dans ses deux cas**&nbsp;: nom coupé →
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |infobulle du nom entier&nbsp;; nom complet → infobulle de la tuile, et pas celle du nom.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |## Piège d'appelant — `Ui::put` avance le curseur du parent (2026-09-12)
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |Ce n'est pas un défaut de composant, c'est un piège d'**appelant**, et il a produit un bug visible
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |en jeu qui a traversé une maquette validée, un portage et trois relectures avant d'être vu.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**Le symptôme** : dans la grille de l'onglet « Alertes », trois tuiles affichaient « Plan "Epée
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |de » à l'identique, sans ellipse, et les noms d'objet étaient coupés net au bord de la tuile. Tout
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |désignait la mise en forme du texte — largeur de tuile trop petite, élision mal réglée. Mesure
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |faite, le nom tenait : `Pierre d'aventure` occupait 102 px pour 108 disponibles.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**La cause** était ailleurs. Chaque tuile faisait bien 118 px de large, mais les origines de deux
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |tuiles voisines n'étaient espacées que de 91 px : **elles se chevauchaient de 27 px**, et le fond
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |opaque de la suivante effaçait la fin du nom de la précédente.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |let (rect, response) = ui.allocate_exact_size(taille_de_la_tuile, Sense::click());
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |// …
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |ui.put(emplacement_centré, design::item_slot());   // ⚠ avance le curseur du parent
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`Ui::put` ouvre un scope, et un scope termine par `advance_cursor_after_rect`. Le curseur de la
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |rangée était donc ramené au bord droit de l'**emplacement** — centré dans la tuile, donc 27 px
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |avant son bord — et la tuile suivante démarrait là.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |**La règle** : dans un conteneur qui dispose des éléments (`horizontal`, `vertical`, une grille),
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |poser quoi que ce soit à une position absolue passe par un enfant, jamais par le `ui` du conteneur.
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```rust
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |let mut cellule = ui.new_child(egui::UiBuilder::new().max_rect(rect));
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |cellule.put(emplacement_centré, design::item_slot());   // le curseur de la rangée ne bouge pas
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |```
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |`Ui::new_child` n'avance rien : c'est ce qui le distingue de `scope`/`put`/`add`. Le même geste
| Teinte, tous états *(remplacée le soir même par la couleur de la liste, voir plus bas)* | `HEADING_TEXT` (`#b8b9ba`) |vaut pour tout composant posé par rectangle à l'intérieur d'une cellule déjà allouée.
