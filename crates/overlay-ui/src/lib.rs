//! Bibliothèque d'`overlay-ui` — expose la construction d'interface (`render_content::build_ui`,
//! `render_content::RenderContent`) indépendamment du fenêtrage/GPU réels, pour que
//! `overlay-testkit` (§17.1 du plan) puisse la rejouer offscreen, sans fenêtre système ni GPU
//! physique. Le binaire (`main.rs`, jamais accessible depuis l'extérieur du crate — Windows
//! uniquement) reste seul responsable du fenêtrage réel (winit/wgpu), des threads de fond et du
//! câblage complet de l'application.
//!
//! `game_window`, `alert_sound`, `frame` et `engine_thread` sont exposés ici (§17.2 du plan,
//! Niveau 2) bien qu'ils ne servent à AUCUN rendu offscreen — contrairement aux autres modules de
//! cette liste, leur raison d'être n'est pas `overlay-testkit` mais de permettre à un DEUXIÈME
//! binaire (`src/bin/overlay-ui-x11.rs`, Linux) de réutiliser tel quel tout ce qui, dans l'ancien
//! `main.rs`, ne dépendait d'aucune API Windows : la découverte de fenêtre de jeu (`game_window`,
//! déjà câblée sur `overlay_platform::linux::x11` côté Linux), les sons d'alerte (`alert_sound`,
//! rodio pur, jamais d'appel Win32), la boucle de peinture par frame (`frame::render`, générique
//! wgpu/egui) et le thread d'ingestion (`engine_thread::spawn_engine_thread`, aucun appel
//! Windows). Seule `init_gpu` (choix du backend GPU — `DX12`/`DirectComposition` côté Windows,
//! `Vulkan`/`GL` côté Linux, voir `spikes/s3-window-linux/src/main.rs`) reste dupliquée entre les
//! deux binaires : ce n'est PAS du code partageable, le choix de backend diffère fondamentalement
//! d'un OS à l'autre (§6.2 du plan).
//!
//! `logging` (initialisation `tracing`, gestionnaire Ctrl+C) est également générique — exposé pour
//! la même raison.
//!
//! Modules volontairement absents d'ici (restent privés à `main.rs`, spécifiques au fenêtrage
//! Win32/aux threads compte lié L4-L5, sans intérêt pour un harnais de rendu offscreen NI pour le
//! binaire Linux en mode invité, voir §17.2 « État ») : aucun pour l'instant.

pub mod alert_sound;
pub mod engine_thread;
pub mod frame;
pub mod game_window;
pub mod logging;
pub mod panels;
pub mod portraits;
pub mod remote_icons;
pub mod render_content;
pub mod ui_icons;
