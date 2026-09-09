# Contrat d'un composant du design system

Ce que tout fichier de `crates/overlay-ui/src/design/components/` doit respecter. `button.rs` en est
l'implémentation de référence : en cas de doute, l'ouvrir plutôt que d'improviser.

## 1. API

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
