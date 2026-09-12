//! Panneaux de l'overlay (docs/plan-architecture.md §9) — chacun deviendra à terme une zone
//! indépendante, activable/désactivable séparément (demande utilisateur : Combat à gauche, Récap
//! en haut-gauche, Suivi en haut-centre). Cette itération extrait `combat` et `watchlist` — le
//! récap reste inline dans `main.rs::render` (voir la doc de tête de ce fichier pour le périmètre
//! exact).

pub mod alerts_tab;
pub mod chamfer;
pub mod combat;
pub mod combat_frame;
pub mod combat_frame_scroll;
pub mod combat_spell_block;
pub mod options_modal;
pub mod watchlist;
