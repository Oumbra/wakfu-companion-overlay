//! Composants du design system — voir la doc de `crate::design` pour la répartition des rôles.
//!
//! **Contrat commun à tout composant de ce dossier** (le respecter est ce qui rend les composants
//! interchangeables et débogables ; le skill `.claude/skills/ui-component/` le détaille et
//! l'applique) :
//!
//! 1. **Une fonction libre de construction** (`design::button("Valider")`) et un `struct` à
//!    méthodes chaînées pour les paramètres. Aucun paramètre obligatoire hors le contenu.
//! 2. **`impl egui::Widget`** : le composant s'ajoute par `ui.add(...)` et rend une
//!    `egui::Response`. Il ne décide jamais de ce qui se passe au clic.
//! 3. **Aucune texture en paramètre.** Le composant résout ses textures via `DesignSystem::get`,
//!    à partir de sa variante — l'appelant nomme une intention (`Primary`), pas un fichier.
//! 4. **États uniformes** : repos / survolé / désactivé, et la règle d'appui commune à toute
//!    l'UI — *tant qu'un bouton de souris est enfoncé, l'apparence retombe au repos ; elle
//!    revient au survol au relâchement* (voir `ButtonState`, la même règle dans tous les
//!    composants qui réagissent au survol).
//! 5. **Toute taille est valide.** Les textures sont peintes en 9-slice
//!    (`design::nine_slice`) : jamais un asset par taille.
//! 6. **Journalisation** : un `tracing::debug!` à l'action (clic), un `tracing::warn!` une seule
//!    fois par instance quand la géométrie demandée ne peut pas honorer le contenu (libellé
//!    tronqué). Le nom d'instance (`log_name`) rend la ligne exploitable dans
//!    `overlay-ui.<date>.log`.
//! 7. **Une entrée dans la galerie** (`crates/overlay-testkit/tests/design_gallery.rs`) : toute
//!    variante et tout état visibles sur une capture unique, comparée à chaque exécution.

pub mod autocomplete;
pub mod button;
pub mod checkbox;
pub mod collapsible;
pub mod heading;
pub mod icon;
pub mod icon_button;
pub mod info_text;
pub mod input;
pub mod item_slot;
pub mod loader;
pub mod meter;
pub mod pagination;
pub mod panel;
pub mod portrait;
pub mod scroll_area;
pub mod select;
pub mod separator;
pub mod slider;
pub mod stepper;
pub mod table;
pub mod tabs;
pub mod tooltip;
pub mod window;
