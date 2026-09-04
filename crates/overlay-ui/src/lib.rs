//! Bibliothèque d'`overlay-ui` — expose la construction d'interface (`render_content::build_ui`,
//! `render_content::RenderContent`) indépendamment du fenêtrage/GPU réels, pour que
//! `overlay-testkit` (§17.1 du plan) puisse la rejouer offscreen, sans fenêtre système ni GPU
//! physique. Le binaire (`main.rs`, jamais accessible depuis l'extérieur du crate — Windows
//! uniquement) reste seul responsable du fenêtrage réel (winit/wgpu), des threads de fond et du
//! câblage complet de l'application.
//!
//! `game_window` est exposé ici (§17.2 du plan, Niveau 2) bien qu'il ne serve à AUCUN rendu
//! offscreen : contrairement aux autres modules de cette liste, sa raison d'être n'est pas
//! `overlay-testkit` mais de rendre son implémentation Linux/X11 (`overlay_platform::linux::x11`,
//! câblée depuis cette session) réellement compilable et vérifiable — `mod game_window;` privé à
//! `main.rs` (Windows-only, `windows::` importé sans `cfg`) ne l'aurait jamais exercée sur aucune
//! cible buildable. Le futur point d'entrée Linux (pas encore écrit, voir
//! `docs/plan-architecture.md` §17.2 « État ») consommera `overlay_ui::game_window` exactement
//! comme `main.rs` le fait aujourd'hui pour Windows.
//!
//! Modules volontairement absents d'ici (restent privés à `main.rs`, spécifiques au fenêtrage/aux
//! threads, sans intérêt pour un harnais de rendu offscreen) : `alert_sound`, `logging`.

pub mod game_window;
pub mod panels;
pub mod portraits;
pub mod remote_icons;
pub mod render_content;
pub mod ui_icons;
