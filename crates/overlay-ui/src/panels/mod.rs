//! Panneaux de l'overlay (docs/plan-architecture.md §9) — chacun deviendra à terme une zone
//! indépendante, activable/désactivable séparément (demande utilisateur : Combat à gauche, Récap
//! en haut-gauche, Suivi en haut-centre). Cette itération extrait `combat` et `watchlist` — le
//! récap reste inline dans `main.rs::render` (voir la doc de tête de ce fichier pour le périmètre
//! exact).

pub mod alerts_tab;
pub mod bulk_select;
pub mod chamfer;
pub mod chat_tab;
pub mod combat;
pub mod combat_bars;
pub mod combat_frame;
pub mod combat_frame_scroll;
mod combat_scrollbar;
pub mod combat_spell_block;
pub mod feature_switch;
pub mod login;
pub mod notifications;
pub mod options_modal;
pub mod personnages_tab;
pub mod raccourcis_tab;
pub mod recipe_dialog;
pub mod suivi_tab;
pub mod tile_reorder;
pub mod watchlist;
