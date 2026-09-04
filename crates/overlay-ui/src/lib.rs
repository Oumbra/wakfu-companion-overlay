//! Bibliothèque d'`overlay-ui` — expose la construction d'interface (`render_content::build_ui`,
//! `render_content::RenderContent`) indépendamment du fenêtrage/GPU réels, pour que
//! `overlay-testkit` (§17.1 du plan) puisse la rejouer offscreen, sans fenêtre système ni GPU
//! physique. Le binaire (`main.rs`, jamais accessible depuis l'extérieur du crate — Windows
//! uniquement) reste seul responsable du fenêtrage réel (winit/wgpu), des threads de fond et du
//! câblage complet de l'application.
//!
//! Modules volontairement absents d'ici (restent privés à `main.rs`, spécifiques au fenêtrage/aux
//! threads, sans intérêt pour un harnais de rendu offscreen) : `alert_sound`, `game_window`,
//! `logging`.

pub mod panels;
pub mod portraits;
pub mod remote_icons;
pub mod render_content;
pub mod ui_icons;
