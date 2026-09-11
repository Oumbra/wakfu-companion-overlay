# Contrat d'un composant du design system

Ce que tout fichier de `crates/overlay-ui/src/design/components/` doit respecter. `button.rs` en est
l'implémentation de référence : en cas de doute, l'ouvrir plutôt que d'improviser.

**Deux familles de composants**, décidé le 2026-09-10 après cinq dérogations d'affilée à la clause
« `impl egui::Widget` ». Le critère est mécanique, il ne laisse aucune place au jugement :

> **L'appelant fournit-il du contenu à encadrer ?**
> Non → **feuille** (§1) : bouton, champ, case, onglets, icône.
> Oui → **conteneur** (§1 bis) : fenêtre, panneau, repliable, tableau, zone défilable.

Tout le reste du contrat — états, géométrie, journalisation, vérification, interdits — s'applique
**à l'identique aux deux familles**. Seule la forme de l'API change.

## 1. API — composant feuille

```rust
// Construction : une fonction libre, aucun paramètre obligatoire hors le contenu.
pub fn button(text: impl Into<String>) -> Button;

// Paramètres : méthodes chaînées, une par intention.
design::button("Valider")
    .variant(ButtonVariant::Primary)   // intention, jamais une couleur ni un fichier
    .size(ButtonSize::Compact)         // gabarit nommé = mesure du jeu ; sinon Height(f32)
    .width(338.0)                      // dimension imposée, optionnelle
    .enabled(false)
    .tooltip("Choisir d'abord un fichier")
    .log_name("options.valider");
```

- **`impl egui::Widget`** : le composant s'ajoute par `ui.add(...)` et rend une `egui::Response`.
  Il ne décide **jamais** de ce qui se passe au clic — c'est l'appelant qui teste `.clicked()`.
- **Aucune texture, aucun `TextureHandle` en paramètre.** Le composant résout ses textures via
  `DesignSystem::get(ui.ctx())` à partir de sa variante.
- **Un gabarit nommé est une mesure**, pas un palier inventé : `ButtonSize::Standard` = 52px parce
  que `button-primary.png` fait 52px de haut dans le jeu. Toute autre valeur passe par la variante
  libre (`Height(f32)`), qui reste proportionnée (police et marges suivent la hauteur).
- **Une méthode d'aperçu** (`preview_state`) forçant l'état peint, documentée comme réservée à la
  galerie et aux captures : en rendu offscreen aucun pointeur ne survole quoi que ce soit.

## 1 bis. API — composant conteneur

`egui::Widget` ne peut pas décrire un conteneur. Sa signature est `fn ui(self, ui: &mut Ui) ->
Response` : elle n'a de place **ni pour le contenu** que l'appelant fournit, **ni pour ce que ce
contenu rend** — `Response` ne décrit qu'une interaction avec la souris, alors que la modale Options
rend un `OptionsModalAction`. Le ressortir par un `&mut` en paramètre serait exactement la
maladresse que §6 interdit ailleurs.

egui a rencontré la même limite et l'a tranchée pareil : **aucun de ses conteneurs**
(`ScrollArea`, `CollapsingHeader`, `Window`, `Frame`) n'implémente `Widget`, tous exposent un `show`
générique sur le retour du contenu.

Deux formes, et **la première est le défaut**.

### Forme closure

Le conteneur pose la géométrie, appelle le contenu, puis reprend la main :

```rust
pub fn panel() -> Panel;

impl Panel {
    pub fn show<R>(
        self,
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R>;
}
```

**Obligatoire** dès que le conteneur doit écrêter son contenu (§3) ou peindre quoi que ce soit
**après** lui. `design::scroll_area` en est l'exemplaire de référence — jusqu'ici documenté comme
« seul écart au contrat », il en est désormais le premier cas nominal.

### Forme « zone rendue »

Le conteneur peint tout son décor, puis rend le ou les rectangles où l'appelant écrira :

```rust
pub fn window(ui: &mut egui::Ui, …) -> Window;   // Window { inner: Rect, footer: FooterClick }
```

Admise **seulement** quand les deux conditions tiennent : le conteneur ne peint rien après le
contenu, **et** il rend plusieurs sorties indépendantes qu'un `InnerResponse<R>` n'empaquetterait
qu'artificiellement. Le décor de la modale Options est ce cas : il rend à la fois sa zone de contenu
et le clic de son pied de page (`panels::options_modal::chrome`, à remonter dans `design::window`).

Le prix est explicite : **cette forme ne garantit pas l'écrêtage**, l'appelant peint dans un
rectangle sans clip. C'est pour cela qu'elle n'est pas le défaut, et qu'un conteneur de cette forme
expose une méthode dédiée (`Window::scroll_area`, forme closure) pour les zones qui, elles, doivent
être écrêtées.

### Ce qui change pour la galerie

Un conteneur n'a pas d'« états » au sens de §2, et il ne se rend pas seul : son entrée dans
`design_gallery.rs` le montre **avec du contenu factice**, choisi pour exercer sa géométrie — au
moins un contenu qui déborde, pour que l'écrêtage se voie sur la capture.

## 1 ter. Nommer — la règle se lit dans le code existant

**Décidée le 2026-09-11**, après que deux sessions parallèles ont écrit le même composant et lui ont
donné deux noms de type différents pour la même chose : `DsRarity` et `ItemRarity`. Les deux se
défendaient en raisonnement ; un seul se défend en **cohérence**, et c'est le seul argument qui
compte, puisqu'un nom ne se lit jamais seul mais à côté de ses quarante voisins.

| Ce qu'on nomme | Forme | Exemples |
| --- | --- | --- |
| **Clé d'un registre d'assets** | `Ds…` | `DsTexture`, `DsIcon`, `DsTextureSpec`, `DsIconSpec` |
| Constructeur libre | `snake_case` du composant | `button()`, `item_slot()`, `scroll_area()` |
| Struct du composant | `PascalCase` du même mot | `Button`, `ItemSlot`, `ScrollArea` |
| Paramètre propre à UN composant | `<Composant><Rôle>` | `ButtonVariant`, `ButtonSize`, `InputSize`, `LoaderSize`, `TooltipSide` |
| Paramètre d'un **vocabulaire visuel** partagé | le nom du vocabulaire | `ItemRarity`, `SlotFrame`, `SlotCount`, `InfoTone`, `IconContext` |
| État de rendu | `<Composant>State` | `ButtonState`, `InputState`, `SelectState`, `SliderState` |
| Sortie autre qu'une `Response` | `<Composant><ce que c'est>` | `AutocompleteOutcome`, `PanelZones`, `FooterClick` |
| Jeton | `SCREAMING_SNAKE`, **un seul préfixe par composant** | `AUTOCOMPLETE_*`, `SELECT_*`, `SLIDER_*` |

**`Ds…` ne veut pas dire « appartient au design system ».** Tout ce qui vit dans `design::` y
appartient : le préfixe n'apprendrait rien. Il veut dire **« ceci nomme un fichier du manifeste »** —
une variante de `DsTexture` *est* un asset. C'est pour ça que `DsRarity` était le mauvais nom : une
rareté ne nomme pas un asset, elle en **choisit** un (`ItemRarity::border() -> DsTexture`). Sur les
cinquante types publics de `design::`, quatre seulement portent ce préfixe, et ce sont les deux
registres avec leurs specs.

**Un composant, un préfixe de jetons.** `design::item_slot` en a eu deux pendant une demi-journée —
`ITEM_SLOT_*` (7 jetons) et `SLOT_*` (10), pour une seule et même chose, parce qu'ils sont arrivés
en deux fois. Personne ne pouvait deviner de quel côté chercher `SLOT_BACKGROUND` plutôt que
`ITEM_SLOT_ROUNDING`. Les dix ont été renommés le jour même où cette règle a été écrite : une règle
qu'on énonce sans l'appliquer au cas qui l'a fait naître ne survit pas à la semaine.

**Comment trancher un nom en trois minutes** : lister les voisins avant d'inventer.

```bash
grep -rh "^pub enum \|^pub struct " crates/overlay-ui/src/design/components/*.rs | sort
grep -o "^pub const [A-Z_]*" crates/overlay-ui/src/design/tokens.rs | sed 's/pub const //;s/_.*//' | sort | uniq -c
```

Si le nom envisagé n'a aucun frère dans cette liste, ce n'est pas qu'il est original : c'est
presque toujours qu'il suit une règle que ce dépôt n'a pas.

## 2. États

Trois états, les mêmes pour tous les composants :

| État | Quand | Rendu |
| --- | --- | --- |
| `Idle` | par défaut | texture de repos |
| `Hovered` | survolé **et aucun bouton de souris enfoncé** | texture survolée |
| `Disabled` | `enabled(false)` | texture grisée + libellé `TEXT_DISABLED` + atténuation |

**Il n'y a pas d'état « pressé ».** Décision utilisateur (2026-09-09) : dans le jeu, appuyer fait
*disparaître* l'apparence survolée ; elle revient au relâchement. C'est ce que produit la condition
`response.hovered() && !ui.input(|i| i.pointer.any_down())`, déjà appliquée aux boutons icône
(`panels::icon_button::paint_icon_button`, correctif 2026-09-08). Toute autre famille de boutons
doit reprendre **exactement** cette condition, sans quoi les deux se comportent différemment sous
la même souris — le bug qu'il a fallu corriger une première fois.

Un composant désactivé garde `Sense::hover()` (son infobulle explique souvent *pourquoi* il est
grisé) mais perd `Sense::click()` : `clicked()` ne peut alors structurellement pas être vrai, plutôt
que d'être filtré après coup.

## 3. Géométrie

- **Toute taille est valide.** Les textures sont peintes par `design::nine_slice` : coins et liseré
  figés, bandes médianes étendues. Jamais `egui::Image::paint_at` (qui déforme les coins), jamais un
  asset par taille.
- **Les marges figées se dimensionnent sur le décor**, pas sur le rayon des coins : sur les
  composants Wakfu, les hachures sont des **embouts** d'extrémité (~50px sur un bouton 200×52), pas
  une texture de fond. Une marge trop courte laisse le motif dans la bande médiane, où il est étiré
  sur toute la longueur. `component.py insets` rend cette étendue (`decor_span`).
- **Mode de remplissage par axe** : `Stretch` pour un contenu continu (dégradé, bande médiane
  lisse), `Tile` pour un motif réellement périodique. Voir la doc de `design::nine_slice`.
- **Plusieurs textures pour une même variante, choisies par la hauteur.** Quand le jeu a capturé le
  même composant à deux hauteurs, son décor d'extrémité n'y a pas la même largeur (52px sur un
  bouton or de 52px, 34px sur le même bouton en 36px). Le 9-slice ne peut pas rattraper ça : il
  peint l'embout à l'échelle 1:1, fidèlement. Déclarer les deux textures et retenir celle dont la
  **hauteur native est la plus proche** de la hauteur demandée. Le défaut ne se voit pas sur un
  composant isolé — il saute aux yeux dès qu'un composant voisin porte le bon embout.
- **Un rectangle trop petit reste peint**, marges réduites proportionnellement : pas de panique, et
  l'anomalie se voit sur la capture.
- Le contenu est **écrêté au composant** (`Painter::with_clip_rect`) : un libellé trop long ne
  déborde pas sur le panneau voisin, où il passerait pour un bug de mise en page.
- **Le libellé est mis en page avec `design::text::label_font`**, jamais avec
  `FontId::proportional` : la police par défaut d'egui est une Light, bien plus maigre que celle du
  jeu ; le design system embarque sa propre graisse (`design::fonts`). Attention si vous cherchez à
  régler finement une position de texte : **egui arrondit la position d'un texte au pixel entier**
  (`Options::round_text_to_pixels`) — un décalage de 0,3px est rigoureusement sans effet, et une
  mesure qui l'ignore donne des résultats identiques pour des réglages différents. C'est ce qui a
  condamné la première tentative de graisse synthétique par halo.
- **La graisse d'un libellé n'est pas au choix de l'appelant** : elle vient de la variante
  (`ButtonVariant::label_strong`), parce que le jeu écrit ses boutons de pied de page plus gras que
  ceux posés dans un contenu, à hauteur d'encre identique. Ne pas la court-circuiter en passant un
  `FontId` à la main.
- **Un titre passe par `design::text::title_font`**, pas par `label_font` : le jeu utilise deux
  polices, une linéale pour ses libellés de bouton et une serif grasse pour ses titres. Les deux
  familles sont embarquées par `design::fonts`.
- **Un texte cerné passe par `design::text::paint_outlined_text`**, avec le jeu de décalages qui
  correspond à son fond — `OUTLINE_FULL` si le texte flotte nu par-dessus le jeu (fond arbitraire,
  il faut cerner de tous les côtés), `SHADOW_BOTTOM_RIGHT` si le fond est connu (bannière, encadré) :
  trois décalages suffisent, et un contour complet empâterait le mot.
- **Aucun effet hors de l'écran.** Un composant (et plus généralement un panneau) ne fait jamais
  lui-même ce qui déborde de la fenêtre : ouvrir une page, écrire un fichier, émettre une requête.
  Il **remonte l'intention** à son appelant (`Response`, structure de résultat) et l'appelant agit.
  Le harnais de capture clique pour de vrai : un composant qui ouvre une page l'ouvre à chaque
  `cargo test`, sur la machine de qui lance la suite. C'est arrivé — voir §17.3 bis du plan.
- **La hauteur d'un composant est celle de sa texture**, jamais déduite de sa largeur : un 9-slice
  existe pour qu'on l'étire en largeur *sans* toucher à sa hauteur. Déduire la hauteur d'un rapport
  d'aspect rétrécit le composant — et son libellé avec — dès que le panneau qui le porte est plus
  étroit que la fenêtre du jeu.

## 4. Journalisation

Le journal est `%APPDATA%\wakfu-companion-overlay\logs\overlay-ui.<date>.log` (§15 du plan) : c'est
lui qu'on relit pour déboguer un retour utilisateur, pas le terminal.

- `tracing::debug!(component = "<nom>", name, …, "clic")` **à l'action**, jamais par frame.
- `tracing::warn!` **une seule fois par instance** (mémorisé sur l'id du widget) quand la géométrie
  demandée ne peut pas honorer le contenu — libellé écrêté, rectangle plus petit que ses marges.
  Un défaut de mise en page est un événement unique, pas un flux à 60 Hz.
- `name` vient de `log_name` (défaut : le libellé). À renseigner dès que deux instances partagent un
  libellé, sinon les lignes du journal sont indiscernables.

## 5. Vérification

- Une entrée dans `crates/overlay-testkit/tests/design_gallery.rs`, **toutes variantes × tous
  états**.
- Une comparaison pixel avec la capture du jeu, à la même taille (étape 6 du SKILL).
- La capture publiée en **Artifact**.

## 6. Ce qu'un composant ne fait pas

- Aucune logique métier, aucun accès à `overlay_engine`, aucun `SessionSnapshot`.
- Aucun état applicatif : ce qui doit survivre entre deux frames appartient au panneau appelant.
- Aucune décision de mise en page (position, marges externes, ordre) : c'est le rôle de `panels/*`.
