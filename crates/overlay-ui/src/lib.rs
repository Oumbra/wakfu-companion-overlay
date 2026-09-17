//! Bibliothèque d'`overlay-ui` — expose la construction d'interface (`render_content::build_ui`,
//! `render_content::RenderContent`) indépendamment du fenêtrage/GPU réels, pour que
//! `overlay-testkit` (§17.1 du plan) puisse la rejouer offscreen, sans fenêtre système ni GPU
//! physique. Le binaire (`main.rs`, jamais accessible depuis l'extérieur du crate — Windows
//! uniquement) reste seul responsable du fenêtrage réel (winit/wgpu), des threads de fond et du
//! câblage complet de l'application.
//!
//! `game_window`, `alert_sound`, `frame` et `engine_thread` sont exposés ici (§17.2 du plan, Niveau
//! 2) bien qu'ils ne servent à AUCUN rendu offscreen — contrairement aux autres modules de cette
//! liste, leur raison d'être n'est pas `overlay-testkit` mais de permettre à un DEUXIÈME binaire
//! (`src/bin/wakfu-companion-overlay-x11.rs`, Linux) de réutiliser tel quel tout ce qui, dans
//! l'ancien `main.rs`, ne dépendait d'aucune API Windows : la découverte de fenêtre de jeu
//! (`game_window`, déjà câblée sur `overlay_platform::linux::x11` côté Linux), les sons d'alerte
//! (`alert_sound`, rodio pur, jamais d'appel Win32), la boucle de peinture par frame
//! (`frame::render`, générique wgpu/egui) et le thread d'ingestion
//! (`engine_thread::spawn_engine_thread`, aucun appel Windows). Seule `init_gpu` (choix du backend
//! GPU — `DX12`/`DirectComposition` côté Windows, `Vulkan`/`GL` côté Linux, voir
//! `spikes/s3-window-linux/src/main.rs`) reste dupliquée entre les deux binaires : ce n'est PAS du
//! code partageable, le choix de backend diffère fondamentalement d'un OS à l'autre (§6.2 du plan).
//!
//! `logging` (initialisation `tracing`, gestionnaire Ctrl+C) est également générique — exposé pour
//! la même raison.
//!
//! `chat_command` (2026-09-13, raccourcis multicompte — §9.1 sexies du plan) : frappe d'une
//! commande de chat du jeu (`/i`, `/fol`) dans la fenêtre au premier plan. Exposé pour la même
//! raison que `game_window`, dont il est le pendant « écriture » : les DEUX binaires s'en servent,
//! chacun avec son `imp` (`SendInput` sous Windows, XTEST via `overlay_platform::linux::keyboard`
//! sous Linux), et la sélection du personnage visé (`chat_command::partner_character`) est une
//! fonction pure testée ici plutôt que dupliquée dans chaque binaire.
//!
//! `config` (2026-09-08, modale Options — §9 du plan) : persistance du chemin de `wakfu.log` choisi
//! par l'utilisateur, même raison que le paragraphe ci-dessus (aucun appel Windows,
//! `resolve_log_path`/`save` appelés à l'identique par `main.rs` et
//! `bin/wakfu-companion-overlay-x11.rs`).
//!
//! Modules volontairement absents d'ici (restent privés à `main.rs`, spécifiques au fenêtrage
//! Win32/aux threads compte lié L4-L5, sans intérêt pour un harnais de rendu offscreen NI pour le
//! binaire Linux en mode invité, voir §17.2 « État ») : aucun pour l'instant.

pub mod alert_sound;
pub mod autostart;
pub mod avatars;
pub mod background;
pub mod build_info;
pub mod chat_command;
pub mod config;
pub mod cursor;
pub mod design;
pub mod engine_thread;
pub mod frame;
pub mod game_servers;
pub mod game_window;
pub mod logging;
pub mod panels;
pub mod portraits;
pub mod rarity_bridge;
pub mod recap_session;
pub mod remote_icons;
pub mod render_content;
pub mod shortcuts;
pub mod startup;
pub mod style;
pub mod turn_watch;
pub mod ui_icons;

/// Le mécanisme de mise à jour automatique vit dans `overlay-sync` (réseau) ; réexporté ici pour
/// que le harnais de rendu (`overlay-testkit`, qui ne dépend que de cette crate) construise les
/// états qu'il capture.
pub use overlay_sync::update;
