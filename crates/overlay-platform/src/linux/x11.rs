//! Découverte de fenêtre(s) de jeu par titre, sous X11 — équivalent direct de
//! `crates/overlay-ui/src/game_window.rs` (Windows, module `imp` sous `#[cfg(target_os =
//! "windows")]`), voir sa doc de module pour le contexte complet (multi-compte, `" - WAKFU"` comme
//! seul discriminant fiable puisque le process est `java` générique). Mêmes types (`GameRect`,
//! `GameWindowInfo`, `scan()` renvoyant `Vec<(String, GameWindowInfo)>`) que le pendant Windows,
//! pour que `game_window.rs` puisse appeler l'un ou l'autre selon la cible sans logique
//! supplémentaire côté appelant.
//!
//! Migré depuis `spikes/s3-window-linux/src/discovery.rs` (§17.2 du plan, critère de sortie de
//! S3, point 1) — code inchangé, seul le module qui l'héberge change : validé programmatiquement
//! sous Xvfb par `spikes/s3-window-linux/harness.sh` avant cette migration (voir son README pour
//! le détail de cette validation, non reproduite ici). Câblé à `overlay_ui::game_window` depuis
//! cette session (voir sa doc) ; pas encore câblé à un vrai rendu winit X11 multi-fenêtres côté
//! production (voir `docs/plan-architecture.md` §17.2, section « État »).
//!
//! Mécanique EWMH utilisée (§6.5 du plan) :
//! - `_NET_CLIENT_LIST` sur la fenêtre racine : liste des fenêtres de haut niveau gérées par le WM
//!   — nécessite un WM EWMH (`openbox` dans ce spike/harnais, voir README.md) ; sans lui, cette
//!   propriété n'existe pas et `scan()` renvoie toujours une liste vide (pas un crash, voir
//!   `list_top_level_windows`).
//! - `_NET_WM_NAME` (UTF8_STRING) prioritaire, repli sur `WM_NAME` (STRING legacy) pour le titre —
//!   même ordre que documenté au plan.
//! - `_NET_FRAME_EXTENTS` (CARDINAL[4] = left,right,top,bottom) pour la marge de décoration ajoutée
//!   par le WM autour de la fenêtre cliente — absente tant que le WM ne l'a pas encore posée
//!   (course possible juste après le mapping), repli sur des extents nuls plutôt qu'écarter la
//!   fenêtre.
//!
//! **Différence assumée avec la prod réelle** : la fenêtre factice de ce spike/harnais (`xterm`,
//! voir README.md) porte une VRAIE décoration ajoutée par le WM (barre de titre openbox), donc
//! `client_top != top` s'y observe réellement — alors que le vrai client Wakfu (Java/JOGL) peint sa
//! propre fausse barre de titre dans sa zone cliente et n'a AUCUNE décoration côté WM (comme documenté
//! côté Windows, `GameRect::client_top`), donnant `client_top == top` en conditions réelles. L'algorithme
//! (lire `_NET_FRAME_EXTENTS`) est le même dans les deux cas ; seul le résultat numérique diffère
//! selon que la fenêtre observée est décorée ou non — ce n'est pas un bug de ce module, juste un
//! écart entre le harnais de test et le jeu réel, à noter en le lisant.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, Window};
use x11rb::rust_connection::RustConnection;

/// Suffixe distinctif et invariant du titre de la fenêtre du client Wakfu — voir le commentaire de
/// module. Exportée pour que le harnais (fenêtres `xterm` factices) l'utilise TELLE QUELLE plutôt
/// que de la recopier à la main (réserve de la revue à trois experts, §17.6 du plan : éviter toute
/// désynchronisation silencieuse entre la détection réelle et son harnais de test).
pub const TITLE_SUFFIX: &str = " - WAKFU";

/// Rectangle (coordonnées racine, pixels) de la fenêtre de jeu trouvée — mêmes champs que
/// `overlay_ui::game_window::GameRect` (voir sa doc), pour rester un portage direct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    /// Bord haut de la zone CLIENTE (sans la décoration éventuelle du WM) — voir la doc de module
    /// pour pourquoi ce champ diffère de `top` dans CE harnais (xterm décorée) alors qu'il coïncide
    /// avec `top` sur le vrai client Wakfu (non décoré).
    pub client_top: i32,
}

/// Une fenêtre de jeu trouvée — `window` (l'identifiant XID) sert de clé stable tant que la
/// fenêtre vit, comme `HWND` côté Windows.
#[derive(Debug, Clone, Copy)]
pub struct GameWindowInfo {
    pub window: Window,
    pub rect: GameRect,
}

pub struct GameWindowTracker {
    conn: RustConnection,
    root: Window,
    net_client_list: u32,
    net_wm_name: u32,
    net_frame_extents: u32,
    net_active_window: u32,
    utf8_string: u32,
    wm_name: u32,
}

impl GameWindowTracker {
    /// Ouvre sa PROPRE connexion X11 (via `$DISPLAY`) — indépendante de celle de `main.rs`
    /// (rendu winit), comme `overlay-platform::linux::x11` le fera une fois migré : la découverte
    /// de fenêtres n'a besoin d'aucun état partagé avec le rendu.
    pub fn connect() -> Result<Self, x11rb::errors::ConnectError> {
        let (conn, screen_num) = RustConnection::connect(None)?;
        let root = conn.setup().roots[screen_num].root;
        let atom = |name: &str| -> u32 {
            conn.intern_atom(false, name.as_bytes())
                .expect("envoi InternAtom")
                .reply()
                .expect("réponse InternAtom")
                .atom
        };
        Ok(Self {
            net_client_list: atom("_NET_CLIENT_LIST"),
            net_wm_name: atom("_NET_WM_NAME"),
            net_frame_extents: atom("_NET_FRAME_EXTENTS"),
            net_active_window: atom("_NET_ACTIVE_WINDOW"),
            utf8_string: atom("UTF8_STRING"),
            wm_name: AtomEnum::WM_NAME.into(),
            conn,
            root,
        })
    }

    /// `_NET_ACTIVE_WINDOW` (propriété du WM sur la racine) — équivalent EWMH de
    /// `GetForegroundWindow` (Windows), utilisé par `main.rs::poll_topmost` pour déterminer
    /// `relevant` (voir `topmost::decide`). `None` si le WM ne la maintient pas (pas de fenêtre
    /// active connue, ou WM non EWMH) — traité comme « rien de pertinent n'a le focus », jamais
    /// une erreur.
    pub fn active_window(&self) -> Option<Window> {
        let reply = self
            .conn
            .get_property(
                false,
                self.root,
                self.net_active_window,
                AtomEnum::WINDOW,
                0,
                1,
            )
            .ok()?
            .reply()
            .ok()?;
        let value = reply.value32()?.next()?;
        (value != 0).then_some(value)
    }

    /// Toutes les fenêtres de jeu actuellement visibles (titre finissant par `TITLE_SUFFIX`), avec
    /// le nom de personnage extrait (suffixe retiré) et leur rectangle — même contrat que
    /// `overlay_ui::game_window::GameWindowTracker::scan`.
    pub fn scan(&self) -> Vec<(String, GameWindowInfo)> {
        let Some(windows) = self.client_list() else {
            // `_NET_CLIENT_LIST` absente : pas de WM EWMH actif (voir doc de module) — repli sur
            // liste vide, jamais un crash. C'est exactement le cas qu'un futur
            // `overlay-platform::linux::x11` doit tolérer (WM minimaliste sans EWMH côté
            // utilisateur final).
            return Vec::new();
        };

        windows
            .into_iter()
            .filter_map(|window| {
                let title = self.window_title(window)?;
                let character_name = title.strip_suffix(TITLE_SUFFIX)?.to_string();
                let rect = self.window_rect(window)?;
                Some((character_name, GameWindowInfo { window, rect }))
            })
            .collect()
    }

    fn client_list(&self) -> Option<Vec<Window>> {
        let reply = self
            .conn
            .get_property(
                false,
                self.root,
                self.net_client_list,
                AtomEnum::WINDOW,
                0,
                u32::MAX,
            )
            .ok()?
            .reply()
            .ok()?;
        if reply.value_len == 0 {
            return None;
        }
        let windows: Vec<Window> = reply.value32()?.collect();
        Some(windows)
    }

    /// `_NET_WM_NAME` (UTF8_STRING) prioritaire, repli sur `WM_NAME` (STRING legacy) — même ordre
    /// que documenté au §6.5 du plan. `None` si aucune des deux propriété n'est lisible (fenêtre
    /// fermée entre l'énumération et cet appel, par exemple) — exclut simplement cette fenêtre de
    /// ce scan, jamais fatal.
    fn window_title(&self, window: Window) -> Option<String> {
        let read = |property: u32, r#type: u32| -> Option<String> {
            let reply = self
                .conn
                .get_property(false, window, property, r#type, 0, u32::MAX)
                .ok()?
                .reply()
                .ok()?;
            (reply.value_len > 0).then(|| String::from_utf8_lossy(&reply.value).into_owned())
        };
        read(self.net_wm_name, self.utf8_string)
            .or_else(|| read(self.wm_name, AtomEnum::STRING.into()))
    }

    fn window_rect(&self, window: Window) -> Option<GameRect> {
        let geom = self.conn.get_geometry(window).ok()?.reply().ok()?;
        let translated = self
            .conn
            .translate_coordinates(window, self.root, 0, 0)
            .ok()?
            .reply()
            .ok()?;
        let client_left = translated.dst_x as i32;
        let client_top = translated.dst_y as i32;

        let (ext_left, ext_right, ext_top, ext_bottom) =
            self.frame_extents(window).unwrap_or((0, 0, 0, 0));

        Some(GameRect {
            left: client_left - ext_left,
            top: client_top - ext_top,
            width: geom.width as i32 + ext_left + ext_right,
            height: geom.height as i32 + ext_top + ext_bottom,
            client_top,
        })
    }

    /// `(left, right, top, bottom)` — `None` si la propriété n'est pas encore posée par le WM
    /// (voir la doc de module) ; l'appelant retombe alors sur des extents nuls.
    fn frame_extents(&self, window: Window) -> Option<(i32, i32, i32, i32)> {
        let reply = self
            .conn
            .get_property(
                false,
                window,
                self.net_frame_extents,
                AtomEnum::CARDINAL,
                0,
                4,
            )
            .ok()?
            .reply()
            .ok()?;
        let mut values = reply.value32()?;
        Some((
            values.next()? as i32,
            values.next()? as i32,
            values.next()? as i32,
            values.next()? as i32,
        ))
    }
}
