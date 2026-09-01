//! Panneaux de l'overlay (docs/plan-architecture.md §9) — chacun deviendra à terme une zone
//! indépendante, activable/désactivable séparément (demande utilisateur : Combat à gauche, Récap
//! en haut-gauche, Suivi en haut-centre). Cette itération n'extrait que `combat` — le récap reste
//! inline dans `main.rs::render` (voir la doc de tête de ce fichier pour le périmètre exact).

pub mod combat;
