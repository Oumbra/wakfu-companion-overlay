//! Implémentation Linux/X11 — voir la doc du crate. `x11` : découverte de fenêtre(s) de jeu par
//! titre (EWMH, via `x11rb`). `topmost` : décision pure « rester au-dessus / replier », horloge
//! injectable, sans aucune dépendance X11 (testée en microsecondes, sans display réel — voir ses
//! tests). `keyboard` : frappe clavier synthétique (XTEST), pendant Linux de `SendInput` —
//! utilisée par les raccourcis multicompte (`overlay_ui::chat_command`).

pub mod keyboard;
pub mod topmost;
pub mod x11;
