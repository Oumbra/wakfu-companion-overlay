//! Sonde d'assertions du harnais (§17.2 du plan) — interroge l'état RÉEL du serveur X pour UNE
//! fenêtre donnée, jamais une déduction depuis une capture d'écran (réserve de l'expert X11,
//! §17.6) :
//! - `shape_input_rects=<n>` — nombre de rectangles de la région d'entrée XShape (0 = click-through
//!   actif, ≥1 = interactif ; c'est exactement le mécanisme que `Window::set_cursor_hittest` de
//!   winit pose en interne, voir sa doc dans le code source citée par README.md).
//! - `stacking_index=<i>`/`stacking_total=<n>` — position de la fenêtre dans
//!   `_NET_CLIENT_LIST_STACKING` (ordre d'empilement EWMH, du plus bas au plus haut) ; `-1` si la
//!   fenêtre n'y figure pas (WM non EWMH, ou fenêtre non gérée par le WM — cas d'une fenêtre
//!   `override_redirect`, à noter si rencontré).
//! - `active_window=<xid ou none>` / `is_active=<true|false>` — `_NET_ACTIVE_WINDOW` comparé à la
//!   fenêtre interrogée.
//!
//! Usage : `probe <xid hex ou décimal>` — sortie `clé=valeur`, une par ligne, pensée pour être
//! parsée par le script d'orchestration du harnais (`harness.sh`) sans dépendance JSON.

use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::shape::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, Window};
use x11rb::rust_connection::RustConnection;

fn parse_xid(arg: &str) -> Window {
    let arg = arg.trim();
    if let Some(hex) = arg.strip_prefix("0x").or_else(|| arg.strip_prefix("0X")) {
        Window::from_str_radix(hex, 16).expect("XID hexadécimal invalide")
    } else {
        arg.parse().expect("XID décimal invalide")
    }
}

fn intern(conn: &RustConnection, name: &str) -> u32 {
    conn.intern_atom(false, name.as_bytes())
        .expect("envoi InternAtom")
        .reply()
        .expect("réponse InternAtom")
        .atom
}

fn main() {
    let xid = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: probe <xid hex ou décimal>");
        std::process::exit(2);
    });
    let window = parse_xid(&xid);

    let (conn, screen_num) =
        RustConnection::connect(None).expect("connexion X11 (DISPLAY défini ?)");
    let root = conn.setup().roots[screen_num].root;

    // --- Shape (click-through) ---
    // `query_extension` d'abord : ne pas planter avec un message obscur sur un Xvfb qui n'aurait
    // pas l'extension Shape (voir README.md — vérifiée présente sur celui de ce harnais).
    let shape_present = conn
        .extension_information(shape::X11_EXTENSION_NAME)
        .expect("QueryExtension SHAPE")
        .is_some();
    if shape_present {
        let rects = conn
            .shape_get_rectangles(window, shape::SK::INPUT)
            .expect("envoi ShapeGetRectangles")
            .reply()
            .expect("réponse ShapeGetRectangles");
        println!("shape_input_rects={}", rects.rectangles.len());
    } else {
        println!("shape_input_rects=unavailable");
    }

    // --- Stacking (_NET_CLIENT_LIST_STACKING) ---
    let net_client_list_stacking = intern(&conn, "_NET_CLIENT_LIST_STACKING");
    let stacking = conn
        .get_property(
            false,
            root,
            net_client_list_stacking,
            AtomEnum::WINDOW,
            0,
            u32::MAX,
        )
        .expect("envoi GetProperty _NET_CLIENT_LIST_STACKING")
        .reply()
        .ok()
        .and_then(|reply| {
            (reply.value_len > 0)
                .then(|| reply.value32().map(|it| it.collect::<Vec<Window>>()))
                .flatten()
        });
    match stacking {
        Some(windows) => {
            let total = windows.len();
            let index = windows
                .iter()
                .position(|&w| w == window)
                .map(|i| i as i64)
                .unwrap_or(-1);
            println!("stacking_index={index}");
            println!("stacking_total={total}");
        }
        None => {
            // Pas de WM EWMH actif (voir doc de module) — attendu et déjà documenté, jamais fatal.
            println!("stacking_index=-1");
            println!("stacking_total=0");
        }
    }

    // --- Focus (_NET_ACTIVE_WINDOW) ---
    let net_active_window = intern(&conn, "_NET_ACTIVE_WINDOW");
    let active = conn
        .get_property(false, root, net_active_window, AtomEnum::WINDOW, 0, 1)
        .expect("envoi GetProperty _NET_ACTIVE_WINDOW")
        .reply()
        .ok()
        .and_then(|reply| reply.value32().and_then(|mut it| it.next()))
        .filter(|&w| w != 0);
    match active {
        Some(active) => {
            println!("active_window=0x{active:x}");
            println!("is_active={}", active == window);
        }
        None => {
            println!("active_window=none");
            println!("is_active=false");
        }
    }
}
