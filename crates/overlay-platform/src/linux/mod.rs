//! Implémentation Linux/X11 — voir la doc du crate. `x11` : découverte de fenêtre(s) de jeu par
//! titre (EWMH, via `x11rb`). `topmost` : décision pure « rester au-dessus / replier », horloge
//! injectable, sans aucune dépendance X11 (testée en microsecondes, sans display réel — voir ses
//! tests).

pub mod topmost;
pub mod x11;
