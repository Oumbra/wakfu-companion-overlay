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
//! **Windows** (`imp` sous `#[cfg(target_os = "windows")]`) : `EnumWindows`/`DwmGetWindowAttribute`,
//! production depuis L2. **Linux/X11** (`imp` sous `#[cfg(not(target_os = "windows"))]`, câblé
//! depuis cette session, §17.2 du plan) : délègue à `overlay_platform::linux::x11` (migré du spike
//! S3, voir sa doc) — connexion X11 établie PARESSEUSEMENT au premier `scan()`, jamais dans `new()`
//! (voir la doc de son `imp`). Les deux `imp` exposent la même forme (`GameWindowTracker::new()`,
//! `scan() -> Vec<(String, GameWindowInfo)>`, `GameRect` commun ci-dessous) pour que le reste
//! d'`overlay-ui` n'ait besoin d'aucun `#[cfg]` disséminé.
//!
//! **Ce que ce câblage ne couvre PAS encore** : aucun point d'entrée Linux réel n'existe (le seul
//! binaire, `main.rs`, importe `windows::` sans `cfg` et ne compile donc que sous Windows) — ce
//! module est verifié par `cargo check -p overlay-ui --lib` (compile nativement sous Linux) mais
//! `scan()` n'est encore appelé par aucun code de production sur cette plateforme. Voir
//! `docs/plan-architecture.md` §17.2 « État » pour le reste du critère de sortie de S3 (fenêtre
//! winit réelle, ancrage/topmost/click-through câblés, multi-fenêtres).

/// Rectangle (coordonnées bureau, pixels physiques) de la fenêtre de jeu trouvée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    /// Coordonnée écran du bord HAUT de la zone CLIENTE Win32 — distincte de `top` en théorie
    /// (bord extérieur de la fenêtre, barre de titre Windows comprise, voir `window_rect`), utile
    /// pour ancrer l'overlay Suivi (`OverlayKind::Watchlist`, `main.rs::anchor_position`) sans que
    /// l'écart s'y noie comme au centrage vertical du panneau Combat.
    ///
    /// **En pratique sur le client Wakfu, `client_top == top` (écart nul, confirmé par le
    /// diagnostic `println!` de `create_overlay_window`, 2026-09-01)** : le client Wakfu (Java/AWT)
    /// dessine lui-même sa fausse barre de titre (bouton menu, "Sagittarius Caecus - WAKFU",
    /// réduire/agrandir/fermer) DANS sa zone cliente, en fenêtre non décorée côté Win32 — il n'y a
    /// donc aucune vraie barre de titre Windows à exclure, la zone cliente Win32 commence bien à
    /// `top`. Le calcul `GetClientRect`/`ClientToScreen` reste correct et vaut la peine d'être
    /// gardé (couvre le cas où un jeu utiliserait un JOUR une vraie fenêtre décorée), mais l'écart
    /// visible constaté par l'utilisateur ne venait PAS de là : c'était bien `GAME_TOP_MARGIN_PX`
    /// lui-même qui était trop petit pour dégager la fausse barre de titre dessinée par le jeu (~28
    /// px, voir sa doc).
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
    use overlay_platform::linux::x11::GameWindowTracker as X11Tracker;

    /// `window` (XID X11) sert de clé stable tant que la fenêtre vit — même rôle que `hwnd` côté
    /// Windows (voir `main.rs::sync_windows`, qui diffusera un jour un scan Linux contre l'état
    /// précédent par cette clé, comme il le fait déjà par `hwnd`).
    #[derive(Debug, Clone, Copy)]
    pub struct GameWindowInfo {
        pub window: u32,
        pub rect: GameRect,
    }

    /// Connexion X11 établie PARESSEUSEMENT au premier `scan()`, jamais dans `new()` : un
    /// `overlay-ui` construit dans un contexte sans `$DISPLAY` (session headless, futur test) ne
    /// doit pas paniquer avant même d'avoir tenté d'afficher quoi que ce soit — c'est `new()` qui
    /// est appelé à la construction de l'`App`, bien avant que quiconque ne se soucie de savoir si
    /// une fenêtre de jeu existe. Échec de connexion persistant (`connect_failed`, jamais retenté
    /// une fois établi) : `scan()` renvoie alors une liste vide, jamais un crash — même politique
    /// que `overlay_platform::linux::x11::GameWindowTracker::scan` face à un WM non-EWMH (voir sa
    /// doc).
    #[derive(Default)]
    pub struct GameWindowTracker {
        tracker: Option<X11Tracker>,
        connect_failed: bool,
    }

    impl std::fmt::Debug for GameWindowTracker {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("GameWindowTracker").finish_non_exhaustive()
        }
    }

    impl GameWindowTracker {
        pub fn new() -> Self {
            Self::default()
        }

        /// Toutes les fenêtres de jeu actuellement visibles — voir la doc de module pour le
        /// contrat partagé avec l'`imp` Windows. `Vec::new()` tant qu'aucune connexion X11 n'a pu
        /// être établie (voir la doc de `GameWindowTracker`), jamais un crash.
        pub fn scan(&mut self) -> Vec<(String, GameWindowInfo)> {
            if self.tracker.is_none() && !self.connect_failed {
                match X11Tracker::connect() {
                    Ok(tracker) => self.tracker = Some(tracker),
                    Err(_) => self.connect_failed = true,
                }
            }
            let Some(tracker) = &self.tracker else {
                return Vec::new();
            };
            tracker
                .scan()
                .into_iter()
                .map(|(character_name, info)| {
                    let rect = GameRect {
                        left: info.rect.left,
                        top: info.rect.top,
                        width: info.rect.width,
                        height: info.rect.height,
                        client_top: info.rect.client_top,
                    };
                    (
                        character_name,
                        GameWindowInfo {
                            window: info.window,
                            rect,
                        },
                    )
                })
                .collect()
        }
    }
}
