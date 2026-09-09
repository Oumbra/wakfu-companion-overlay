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

**Découpage 9-slice** : les hachures diagonales ne sont pas une texture de fond mais un **embout**
d'extrémité — mesuré (`component.py insets`) à 43–51px des bords sur les textures 200×52 / 169×52,
29–30px sur les 338×36, le centre étant un dégradé lisse. Marges figées : **52px** à gauche et à
droite (32px pour les textures danger), **6px** en haut et en bas (arrondi 3–4px + liseré 2px),
étirement sur les deux axes. Un bouton de 500px porte donc exactement deux embouts, comme un bouton
de 200px.

**Mesures** :

| Grandeur | Valeur | Origine |
| --- | --- | --- |
| Hauteurs natives | 52px (HDV), 36px (pied de page de modale) | taille des textures |
| Corps de police | `0,449 × hauteur` | hauteur d'encre 13px du libellé « Annuler » de `large-button-cancel.png`, ÷ 0,805 (hauteur de capitale mesurée sur le rendu egui) |
| Marge horizontale | `0,55 × hauteur` | ordre de grandeur de §4 du design-system — **estimation** |
| Largeur minimale | `2,5 × hauteur` | garde-fou de proportion — **réglage**, pas une mesure |

**Remplace** : les assets taillés sur mesure `assets/design-system/large-button-cancel.png` /
`large-button-validate.png` — un PNG par taille **et** par libellé.

**Migration non faite.** `panels::options_modal` charge encore ses propres copies
(`crates/overlay-ui/assets/ui/options/footer-cancel.png` / `footer-validate.png`, 338×36, libellé
incrusté) et s'appuie sur `panels::nine_slice`, une seconde implémentation du 9-slice arrivée le
même jour (marge unique, étirement seul, pas de répétition). Le prochain lot doit : porter les trois
boutons de la modale sur `design::button`, supprimer ces quatre assets, et retirer
`panels::nine_slice` au profit de `design::nine_slice`.

---

## À faire — composants identifiés, pas encore écrits

Par ordre de fréquence d'usage constatée dans l'overlay et dans les interfaces du jeu relevées :

| Composant | Assets disponibles | Notes |
| --- | --- | --- |
| **Bouton icône** | `button-icon[-hover,-disabled].png`, `button-icon-first-plan[-hover].png`, `icons/*.png` | Existe déjà en `panels::icon_button::paint_icon_button`, mais **hors contrat** : prend quatre `TextureHandle` en paramètres. À reprendre en `design::icon_button(icon).context(FirstPlan|Panel)` — deux contextes de socle, une icône, un clic. |
| **Champ de saisie** | `input-text-width-placeholder.png`, `input-search.png`, `input-number.png`, `empty-input-search.png` | Texte / recherche / nombre ; §5.4 du design-system. |
| **Case à cocher** | `checkbox-{true,false}.png` | §5.6. |
| **Onglets** | `tabs-with-first-tab-active[-and-hover-2nd-tab].png`, `icon-tabs.png` | Onglets texte et onglets icône ; §5.7. |
| **Select / dropdown** | `select-simple.png`, `select-multiple.png` | §5.5. |
| **En-tête repliable** | `collapse-closed.png`, `collapse-width-5th-opened.png` | §5.8. |
| **Chrome de fenêtre** | `modal-header.png`, `decoration-{top,right,bottom}.png`, `flat-template_2-without-decorations.png` | Bannière turquoise + corps + décorations ; §5.10 et §9. |
| **Scrollbar** | `scrollbar-{active,inactive}.png` | §5.9. |
| **Tuile de portrait / d'ennemi** | `crates/overlay-ui/assets/templates/*.png`, planche d'avatars | Panneau Combat — aujourd'hui entièrement dans `panels::combat` et `panels::combat_frame`. |
| **Barre de dégâts** | aucun (dessiné à la main) | Total, noms, barres proportionnelles — aujourd'hui dans `panels::combat`. |
