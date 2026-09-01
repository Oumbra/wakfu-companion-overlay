//! Repérage de **toutes** les fenêtres du jeu Wakfu actuellement ouvertes — une par personnage
//! connecté (multi-compte, plusieurs clients simultanés sous le même compte Windows). Chaque
//! fenêtre overlay se positionne collée au bord gauche de SA fenêtre de jeu, verticalement
//! centrée, et affiche le combat de SON personnage (voir `main.rs::sync_windows` et
//! `overlay_engine::SessionSnapshot::fight_for_character`).
//!
//! Le titre de la fenêtre de jeu est `"<Nom du personnage> - WAKFU"` (ex. `"Sagittarius Caecus -
//! WAKFU"`) — le nom du personnage varie selon l'utilisateur et la session, mais le suffixe
//! `" - WAKFU"` est constant : c'est lui qu'on utilise pour identifier la fenêtre. Vérifié en
//! conditions réelles (`Get-Process | Where MainWindowTitle -ne ""`) : le client tourne sous un
//! process `java`/`javaw` (Wakfu est une application Java) — matcher sur le nom d'exécutable
//! (`wakfu.exe`) était une hypothèse fausse, corrigée ici avant d'avoir été committée. Le nom du
//! process n'est donc pas assez distinctif à lui seul (n'importe quel `java(w).exe` le porte) ;
//! seul le suffixe de titre discrimine correctement, ET c'est lui qui donne le nom du personnage
//! (2026-09-01 : confirmé en test réel multi-compte — c'est ce qui permet de rapprocher une
//! fenêtre de jeu du bon combat).
//!
//! **Multi-fenêtre** (2026-09-01, retour utilisateur) : `EnumWindows` s'arrêtait auparavant à la
//! **première** fenêtre trouvée — correct pour un seul client ouvert, mais en multi-compte
//! l'overlay se retrouvait ancré sur une fenêtre arbitraire, sans rapport avec le combat affiché
//! (dérivé du seul `wakfu.log`, partagé et entrelacé par tous les clients — voir
//! `overlay_engine::session`, doc de module). `scan()` collecte maintenant **toutes** les fenêtres
//! correspondantes à chaque appel.
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
    /// Coordonnée écran du bord HAUT de la zone CLIENTE (contenu du jeu, sous la barre de titre
    /// Windows) — distincte de `top` (bord extérieur de la fenêtre, barre de titre comprise, voir
    /// `window_rect`). Nécessaire pour ancrer l'overlay Suivi (`OverlayKind::Watchlist`,
    /// `main.rs::anchor_position`) : `top` convenait pour le centrage vertical du panneau Combat
    /// (l'écart lié à la barre de titre y est noyé dans un grand rectangle), mais un ancrage HAUT
    /// avec une petite marge fixe rendait visible cet écart (retour utilisateur 2026-09-01,
    /// capture d'écran à l'appui : l'overlay Suivi apparaissait bien plus haut que les éléments
    /// d'interface du jeu ayant la même intention d'ancrage).
    pub client_top: i32,
}

pub use imp::GameWindowTracker;

#[cfg(target_os = "windows")]
mod imp {
    use super::GameRect;
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClientRect, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, IsWindow,
        IsWindowVisible,
    };

    /// Suffixe distinctif et invariant du titre de la fenêtre du client Wakfu — voir le
    /// commentaire de module.
    const TITLE_SUFFIX: &str = " - WAKFU";

    /// Une fenêtre de jeu trouvée — `hwnd` sert de clé stable tant que la fenêtre vit (voir
    /// `main.rs::sync_windows`, qui diffuse un scan contre l'état précédent par `hwnd`).
    #[derive(Debug, Clone, Copy)]
    pub struct GameWindowInfo {
        pub hwnd: HWND,
        pub rect: GameRect,
    }

    /// Sans état : chaque `scan()` refait un `EnumWindows` complet (coût négligeable, déjà le
    /// principe accepté pour le cas mono-fenêtre — voir §6.5 du plan d'archi). Un type nommé
    /// plutôt que des fonctions libres, pour rester cohérent avec l'appel côté `main.rs`
    /// (`self.game_window.scan()`) et laisser la porte ouverte à un futur état interne (debounce,
    /// cache) sans changer l'API appelante.
    #[derive(Debug, Default)]
    pub struct GameWindowTracker;

    impl GameWindowTracker {
        pub fn new() -> Self {
            Self
        }

        /// Toutes les fenêtres de jeu actuellement visibles, avec le nom de personnage extrait du
        /// titre (suffixe retiré) et son rectangle.
        pub fn scan(&mut self) -> Vec<(String, GameWindowInfo)> {
            let mut found: Vec<(String, HWND)> = Vec::new();
            unsafe {
                let _ = EnumWindows(
                    Some(enum_proc),
                    LPARAM(std::ptr::addr_of_mut!(found) as isize),
                );
            }
            found
                .into_iter()
                .filter_map(|(character_name, hwnd)| {
                    // Un rectangle introuvable (fenêtre fermée entre l'énumération et cet appel,
                    // ou appel DWM/GetWindowRect en échec) exclut simplement cette fenêtre de ce
                    // scan — elle réapparaîtra au prochain si elle est toujours là.
                    window_rect(hwnd).map(|rect| (character_name, GameWindowInfo { hwnd, rect }))
                })
                .collect()
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
        if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
            return None;
        }

        // Bord haut RÉEL de la zone cliente (sous la barre de titre) — voir la doc de
        // `GameRect::client_top`. `GetClientRect` donne une origine (0,0), `ClientToScreen` la
        // convertit en coordonnées écran ; repli sur `rect.top` (bord extérieur) si l'un des deux
        // appels échoue plutôt que d'écarter toute la fenêtre pour ça — seul l'ancrage du panneau
        // Suivi en dépend, tout le reste continue de fonctionner avec une valeur approximative.
        let client_top = client_top_screen_y(hwnd).unwrap_or(rect.top);

        Some(GameRect {
            left: rect.left,
            top: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
            client_top,
        })
    }

    fn client_top_screen_y(hwnd: HWND) -> Option<i32> {
        let mut client_rect = RECT::default();
        unsafe { GetClientRect(hwnd, &mut client_rect) }.ok()?;
        let mut origin = POINT { x: 0, y: 0 };
        unsafe { ClientToScreen(hwnd, &mut origin) }
            .as_bool()
            .then_some(origin.y)
    }

    extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                return true.into();
            }
            let title = window_title(hwnd);
            let Some(character_name) = title.strip_suffix(TITLE_SUFFIX) else {
                return true.into(); // continue l'énumération
            };
            let out = &mut *(lparam.0 as *mut Vec<(String, HWND)>);
            out.push((character_name.to_string(), hwnd));
            true.into() // continue : on veut TOUTES les fenêtres, pas seulement la première
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
    /// (`_NET_CLIENT_LIST` + `XGetWindowProperty`/`_NET_WM_NAME` pour le titre et le nom de
    /// personnage, `_NET_FRAME_EXTENTS` pour le rectangle visible).
    #[derive(Debug, Clone, Copy)]
    pub struct GameWindowInfo {
        pub rect: GameRect,
    }

    #[derive(Debug, Default)]
    pub struct GameWindowTracker;

    impl GameWindowTracker {
        pub fn new() -> Self {
            Self
        }

        pub fn scan(&mut self) -> Vec<(String, GameWindowInfo)> {
            Vec::new()
        }
    }
}
