//! `overlay-focus` — le gestionnaire du protocole `wakfu-companion:` (Windows).
//!
//! Lancé par Windows quand l'utilisateur clique un toast de tour, avec l'URI du toast en argument
//! (`wakfu-companion:focus?hwnd=…`) ; il amène la fenêtre du personnage au premier plan et se
//! termine (voir `overlay_ui::turn_watch::notify`). L'overlay sait le faire lui-même (tout début de
//! `main.rs`), mais c'était alors un exécutable **console** : lancé par le shell, il ouvrait une
//! console noire le temps de sa course — vu par l'utilisateur le 2026-09-14, jugé inélégant à
//! raison. Celui-ci est un exécutable **fenêtré sans fenêtre** (`windows_subsystem = "windows"`) :
//! rien à l'écran, et un binaire minuscule qui démarre plus vite que l'overlay (qui, depuis le
//! 2026-09-17, est lui aussi fenêtré sans fenêtre — ce binaire reste préférable pour sa taille et
//! son temps de démarrage). `notify::register_identity` l'enregistre comme gestionnaire s'il est à
//! côté de l'overlay.
//!
//! Sous Linux ce binaire n'a pas d'objet et se termine aussitôt — il existe pour que le workspace
//! compile partout.
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    if let Some(hwnd) = std::env::args()
        .nth(1)
        .as_deref()
        .and_then(overlay_ui::turn_watch::notify::parse_focus_uri)
    {
        overlay_ui::turn_watch::notify::focus_window(hwnd);
    }
}
