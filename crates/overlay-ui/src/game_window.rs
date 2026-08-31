//! Repérage et suivi de la fenêtre du jeu Wakfu — l'overlay se positionne collé au bord gauche
//! de cette fenêtre, verticalement centré (demande utilisateur), et suit tout déplacement ou
//! redimensionnement du client de jeu en continu.
//!
//! Le titre de la fenêtre de jeu est `"<Nom du personnage> - WAKFU"` (ex. `"Sagittarius Caecus -
//! WAKFU"`) — le nom du personnage varie selon l'utilisateur et la session, mais le suffixe
//! `" - WAKFU"` est constant : c'est lui qu'on utilise pour identifier la fenêtre. Vérifié en
//! conditions réelles (`Get-Process | Where MainWindowTitle -ne ""`) : le client tourne sous un
//! process `java`/`javaw` (Wakfu est une application Java) — matcher sur le nom d'exécutable
//! (`wakfu.exe`) était une hypothèse fausse, corrigée ici avant d'avoir été committée. Le nom du
//! process n'est donc pas assez distinctif à lui seul (n'importe quel `java(w).exe` le porte) ;
//! seul le suffixe de titre discrimine correctement.
//!
//! Windows uniquement pour l'instant (S3 — spike X11/Linux — différé, voir
//! `docs/plan-architecture.md` §12). `imp` ci-dessous bascule vers une implémentation vide sur les
//! autres OS pour que le reste d'`overlay-ui` n'ait pas besoin de `#[cfg]` disséminés.

/// Rectangle (coordonnées bureau, pixels physiques) de la fenêtre de jeu trouvée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

pub use imp::GameWindowTracker;

#[cfg(target_os = "windows")]
mod imp {
    use super::GameRect;
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, IsWindow, IsWindowVisible,
    };

    /// Suffixe distinctif et invariant du titre de la fenêtre du client Wakfu — voir le
    /// commentaire de module.
    const TITLE_SUFFIX: &str = " - WAKFU";

    /// Cache paresseusement le HWND trouvé : pas besoin de re-scanner tout le bureau à chaque
    /// tick (50 ms, voir `main.rs::about_to_wait`) tant que la fenêtre de jeu reste vivante.
    pub struct GameWindowTracker {
        hwnd: Option<HWND>,
    }

    impl GameWindowTracker {
        pub fn new() -> Self {
            Self { hwnd: None }
        }

        pub fn rect(&mut self) -> Option<GameRect> {
            if let Some(hwnd) = self.hwnd {
                if unsafe { IsWindow(Some(hwnd)) }.as_bool() {
                    if let Some(rect) = window_rect(hwnd) {
                        return Some(rect);
                    }
                }
                // Fenêtre fermée entre deux ticks (jeu quitté) : on oubliera ce HWND et on
                // re-scannera au prochain appel.
                self.hwnd = None;
            }
            let found = find_game_window();
            self.hwnd = found;
            found.and_then(window_rect)
        }
    }

    fn window_rect(hwnd: HWND) -> Option<GameRect> {
        let mut rect = RECT::default();
        // `GetWindowRect` inclut, sur Windows 10/11, une marge de redimensionnement invisible
        // autour des fenêtres non maximisées — l'utilisateur ne la perçoit pas comme faisant
        // partie de la fenêtre. `DWMWA_EXTENDED_FRAME_BOUNDS` donne le bord réellement visible,
        // ce qui est ce qu'on veut pour « collé à la bordure gauche » ; repli sur
        // `GetWindowRect` si l'appel DWM échoue (fenêtre non composée, etc.).
        let dwm_ok = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rect as *mut _ as *mut _,
                std::mem::size_of::<RECT>() as u32,
            )
        }
        .is_ok();
        if !dwm_ok && unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
            return None;
        }
        Some(GameRect {
            left: rect.left,
            top: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        })
    }

    fn find_game_window() -> Option<HWND> {
        let mut result: Option<HWND> = None;
        unsafe {
            let _ = EnumWindows(
                Some(enum_proc),
                LPARAM(std::ptr::addr_of_mut!(result) as isize),
            );
        }
        result
    }

    extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                return true.into();
            }
            if !window_title(hwnd).ends_with(TITLE_SUFFIX) {
                return true.into();
            }
            let result = &mut *(lparam.0 as *mut Option<HWND>);
            *result = Some(hwnd);
            false.into() // trouvé : arrêter l'énumération ici
        }
    }

    /// Titre de `hwnd`, chaîne vide si indisponible (jamais fatal : une chaîne vide ne finit pas
    /// par `TITLE_SUFFIX`, donc l'énumération continue simplement).
    fn window_title(hwnd: HWND) -> String {
        unsafe {
            let len = GetWindowTextLengthW(hwnd);
            if len <= 0 {
                return String::new();
            }
            let mut buf = vec![0u16; len as usize + 1];
            let written = GetWindowTextW(hwnd, &mut buf);
            String::from_utf16_lossy(&buf[..written as usize])
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::GameRect;

    /// TODO(S3, différé — voir `docs/plan-architecture.md` §12) : équivalent X11/XWayland
    /// (`_NET_WM_PID` + `/proc/<pid>/comm`, ou `_NET_CLIENT_LIST` + `XGetWindowProperty`).
    pub struct GameWindowTracker;

    impl GameWindowTracker {
        pub fn new() -> Self {
            Self
        }

        pub fn rect(&mut self) -> Option<GameRect> {
            None
        }
    }
}
