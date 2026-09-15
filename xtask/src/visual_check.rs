//! `xtask visual-check` — extraction de `spikes/s3-window-linux/{harness.sh,src/probe.rs}` vers un
//! outillage permanent (docs/plan-architecture.md §17.2, point 3 du critère de sortie de S3), une
//! fois qu'un vrai binaire existait à piloter (`crates/overlay-ui/src/bin/overlay-ui-x11.rs`,
//! §17.2 « État »). Orchestre Xvfb + un WM EWMH (openbox) + une fenêtre de jeu factice (xterm
//! titré) + le VRAI `overlay-ui-x11`, pilotage par `xdotool`, vérifications programmatiques par
//! le protocole X11 lui-même (`x11rb`, jamais une déduction depuis une capture d'écran — même
//! réserve que le spike, §17.6 du plan), artefact final pour revue humaine.
//!
//! **Portée** : ancrage, click-through (extension Shape), topmost/focus-aware avec délai de
//! grâce — les trois mêmes familles d'assertion que `harness.sh`, adaptées aux DEUX fenêtres
//! réelles (Combat + Suivi) plutôt qu'au panneau de diagnostic unique du spike. Ne remplace pas
//! `overlay_platform::linux::topmost` (7 tests unitaires, sans Xvfb) ni les tests d'intégration
//! de `Tailer` : ce harnais-ci vérifie que le VRAI binaire câble bien ces briques ensemble sur un
//! display X11 réel, pas leur logique interne.
//!
//! **Prérequis système** (mêmes que le spike, voir son README) : `Xvfb`, `openbox`, `xdotool`,
//! `xterm`, `mesa-vulkan-drivers`, `imagemagick` (`import`, capture finale) — jamais optionnels,
//! `#[cfg]`-gated ici (compile sur toute plateforme, x11rb est un client protocole pur) mais
//! n'a de sens qu'exécuté sous Linux avec un display X11 disponible.
//!
//! **Gouvernance** (§17.2 du plan) : jamais un gate CI par défaut — cette sous-commande n'est
//! PAS appelée par `.github/workflows/ci.yml`, strictement à la demande (`cargo run -p xtask --
//! visual-check`, ou un futur `workflow_dispatch` dédié).

use std::io::Write as _;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::shape::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, Window};
use x11rb::rust_connection::RustConnection;

const DISPLAY_NUM: &str = ":98";
const GAME_TITLE: &str = "Testeur - WAKFU";
/// Même délai de grâce que `overlay_platform::linux::topmost::DEMOTE_GRACE` — attendu ici pour
/// vérifier que le VRAI binaire l'applique bien via le protocole X11, pas relu depuis la
/// constante (couplage volontaire à la valeur observable, pas au nom Rust).
const DEMOTE_GRACE: Duration = Duration::from_millis(1500);

/// Termine tous les process orchestrés à la sortie de portée, même en cas de panique en cours de
/// scénario (`?`/`panic!` plus bas) — sinon un échec laisse un display Xvfb orphelin qui ferait
/// échouer la prochaine exécution sans rapport avec le bug réel (même réserve que
/// `harness.sh::cleanup`, portée ici sur le destructeur Rust plutôt qu'un `trap` bash).
struct ProcessGuard(Vec<Child>);

impl ProcessGuard {
    fn spawn(&mut self, mut command: Command) -> &mut Child {
        let child = command.spawn().unwrap_or_else(|err| {
            panic!("échec de lancement de {command:?} : {err}");
        });
        self.0.push(child);
        self.0.last_mut().expect("vient d'être poussé")
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn xdotool(args: &[&str]) -> String {
    let output = Command::new("xdotool")
        .env("DISPLAY", DISPLAY_NUM)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("échec de lancement de xdotool {args:?} : {err}"));
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn find_window_ids(name_substring: &str) -> Vec<u32> {
    xdotool(&["search", "--name", name_substring])
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect()
}

fn window_name(xid: u32) -> String {
    xdotool(&["getwindowname", &xid.to_string()])
}

/// Trouve, parmi les fenêtres dont le titre contient `safe_substring` (motif SANS caractère
/// multi-octets — voir plus bas pourquoi), celle dont le VRAI titre (relu ici en Rust, jamais
/// reproposé à `xdotool` comme pattern) contient aussi `exact_substring`.
///
/// **Contournement d'un bug réel de `xdotool search --name`** (constaté ici, pas supposé) : le
/// titre des fenêtres overlay contient un tiret cadratin `—` (U+2014, 3 octets UTF-8) —
/// `xdotool getwindowname` le restitue correctement (vérifié par `od -c`), mais dès qu'un tel
/// caractère apparaît DANS LE PATTERN passé à `search --name`, la recherche échoue
/// systématiquement (regex POSIX étendue sous-jacente de `libxdo`, insensible à la locale du
/// process appelant — testée explicitement en `C.UTF-8` ici, même échec). D'où la séparation :
/// `safe_substring` (ASCII pur, ex. `"wakfu-companion-overlay"`) sert de pattern `xdotool`,
/// `exact_substring` (peut contenir `—`) n'est comparé qu'à des chaînes déjà récupérées.
fn find_window_by_title(safe_substring: &str, exact_substring: &str) -> Option<u32> {
    find_window_ids(safe_substring)
        .into_iter()
        .find(|&xid| window_name(xid).contains(exact_substring))
}

fn window_geometry_x(xid: u32) -> i32 {
    let out = xdotool(&["getwindowgeometry", "--shell", &xid.to_string()]);
    out.lines()
        .find_map(|line| line.strip_prefix("X="))
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| panic!("géométrie introuvable pour 0x{xid:x} : {out:?}"))
}

/// Nombre de rectangles de la région d'entrée XShape pour `window` — 0 = click-through actif, ≥1
/// = interactif. Exactement le mécanisme que `Window::set_cursor_hittest` de winit pose en
/// interne (voir `spikes/s3-window-linux/README.md`).
fn shape_input_rects(conn: &RustConnection, window: Window) -> Option<usize> {
    let present = conn
        .extension_information(shape::X11_EXTENSION_NAME)
        .expect("QueryExtension SHAPE")
        .is_some();
    if !present {
        return None;
    }
    let rects = conn
        .shape_get_rectangles(window, shape::SK::INPUT)
        .expect("envoi ShapeGetRectangles")
        .reply()
        .expect("réponse ShapeGetRectangles");
    Some(rects.rectangles.len())
}

fn intern(conn: &RustConnection, name: &str) -> u32 {
    conn.intern_atom(false, name.as_bytes())
        .expect("envoi InternAtom")
        .reply()
        .expect("réponse InternAtom")
        .atom
}

/// `(index, total)` de `window` dans `_NET_CLIENT_LIST_STACKING` — `(-1, 0)` si absent (WM non
/// EWMH, ou fenêtre non gérée par le WM), jamais fatal.
fn stacking_position(conn: &RustConnection, root: Window, window: Window) -> (i64, usize) {
    let atom = intern(conn, "_NET_CLIENT_LIST_STACKING");
    let windows: Option<Vec<Window>> = conn
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, u32::MAX)
        .expect("envoi GetProperty _NET_CLIENT_LIST_STACKING")
        .reply()
        .ok()
        .and_then(|reply| {
            (reply.value_len > 0)
                .then(|| reply.value32().map(|it| it.collect()))
                .flatten()
        });
    match windows {
        Some(windows) => {
            let total = windows.len();
            let index = windows
                .iter()
                .position(|&w| w == window)
                .map(|i| i as i64)
                .unwrap_or(-1);
            (index, total)
        }
        None => (-1, 0),
    }
}

fn wait_until(mut condition: impl FnMut() -> bool, timeout: Duration, what: &str) {
    let deadline = Instant::now() + timeout;
    loop {
        if condition() {
            return;
        }
        if Instant::now() >= deadline {
            panic!("ÉCHEC : délai dépassé en attendant : {what}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

pub fn run() {
    // Build AVANT de toucher X11 — un échec de compilation doit rester une erreur cargo lisible,
    // pas noyé dans le bruit d'un Xvfb déjà lancé pour rien.
    println!("=== Build overlay-ui-x11 ===");
    let status = Command::new("cargo")
        .args([
            "build",
            "--quiet",
            "-p",
            "overlay-ui",
            "--bin",
            "overlay-ui-x11",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/..")
        .status()
        .expect("lancement de cargo build");
    assert!(status.success(), "cargo build a échoué");
    let binary = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target/debug/overlay-ui-x11")
        .canonicalize()
        .expect("le binaire vient d'être construit");

    let mut guard = ProcessGuard(Vec::new());

    println!("=== Xvfb ({DISPLAY_NUM}) ===");
    guard.spawn({
        let mut cmd = Command::new("Xvfb");
        cmd.args([
            DISPLAY_NUM,
            "-screen",
            "0",
            "1280x800x24",
            "+extension",
            "RENDER",
            "+extension",
            "XTEST",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
        cmd
    });
    std::thread::sleep(Duration::from_secs(1));

    println!("=== Gestionnaire de fenêtres EWMH (openbox) ===");
    guard.spawn({
        let mut cmd = Command::new("openbox");
        cmd.env("DISPLAY", DISPLAY_NUM)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        cmd
    });
    std::thread::sleep(Duration::from_secs(1));

    println!("=== Fenêtre de jeu factice (xterm) ===");
    guard.spawn({
        let mut cmd = Command::new("xterm");
        cmd.env("DISPLAY", DISPLAY_NUM)
            .args([
                "-T",
                GAME_TITLE,
                "-geometry",
                "80x24+150+150",
                "-e",
                "sleep",
                "3600",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        cmd
    });
    wait_until(
        || !find_window_ids(GAME_TITLE).is_empty(),
        Duration::from_secs(5),
        "fenêtre de jeu factice visible",
    );
    let game_xid = find_window_ids(GAME_TITLE)[0];
    println!("fenêtre de jeu factice : XID=0x{game_xid:x}");

    println!("=== overlay-ui-x11 (vrai binaire) ===");
    let wakfu_log = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../crates/overlay-engine/tests/wakfu.log")
        .canonicalize()
        .expect("wakfu.log de test présent");
    guard.spawn({
        let mut cmd = Command::new(&binary);
        cmd.env("DISPLAY", DISPLAY_NUM)
            .env("RUST_LOG", "warn")
            .arg(&wakfu_log)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        cmd
    });
    wait_until(
        || find_window_ids("wakfu-companion-overlay").len() >= 2,
        Duration::from_secs(10),
        "les deux fenêtres overlay (Combat + Suivi) visibles",
    );
    let overlay_xids = find_window_ids("wakfu-companion-overlay");
    assert_eq!(
        overlay_xids.len(),
        2,
        "attendu exactement 2 fenêtres overlay (Combat + Suivi), trouvé {overlay_xids:?}"
    );
    let combat_xid = find_window_by_title("wakfu-companion-overlay", "Testeur — Combat")
        .expect("fenêtre Combat introuvable par titre");
    println!("overlay Combat : XID=0x{combat_xid:x}");

    println!("=== Assertion : ancrage (bord gauche + marge) ===");
    let game_x = window_geometry_x(game_xid);
    let overlay_x = window_geometry_x(combat_xid);
    let diff = overlay_x - game_x;
    assert!(
        (8..=20).contains(&diff),
        "ancrage — overlay.x - jeu.x = {diff}, attendu ~12 (GAME_EDGE_MARGIN_PX)"
    );
    println!("OK : ancrage — overlay.x - jeu.x = {diff}");

    let (conn, screen_num) = RustConnection::connect(Some(DISPLAY_NUM))
        .expect("connexion X11 pour les sondes (probe.rs, ex-spike S3)");
    let root = conn.setup().roots[screen_num].root;

    println!("=== Assertion : click-through (extension Shape) ===");
    let before = shape_input_rects(&conn, combat_xid).expect("extension Shape présente");
    assert_eq!(before, 1, "shape_input_rects (interactif, défaut)");
    xdotool(&["key", "--clearmodifiers", "ctrl+alt+w"]);
    std::thread::sleep(Duration::from_millis(300));
    let after_1 = shape_input_rects(&conn, combat_xid).expect("extension Shape présente");
    assert_eq!(after_1, 0, "shape_input_rects (après 1 bascule)");
    xdotool(&["key", "--clearmodifiers", "ctrl+alt+w"]);
    std::thread::sleep(Duration::from_millis(300));
    let after_2 = shape_input_rects(&conn, combat_xid).expect("extension Shape présente");
    assert_eq!(after_2, 1, "shape_input_rects (après 2 bascules)");
    println!("OK : click-through — 1 -> 0 -> 1 sur deux bascules exactement (jamais de double-événement)");

    println!("=== Assertion : topmost / focus-aware ===");
    xdotool(&["windowactivate", &game_xid.to_string()]);
    std::thread::sleep(Duration::from_millis(300));
    let (index, total) = stacking_position(&conn, root, combat_xid);
    assert_eq!(
        index as usize,
        total - 1,
        "overlay Combat au sommet du z-order pendant que le jeu a le focus"
    );
    println!("OK : overlay au sommet (stacking_index={index}/{total})");

    println!("=== Fenêtre non-jeu + délai de grâce (démotion) ===");
    guard.spawn({
        let mut cmd = Command::new("xterm");
        cmd.env("DISPLAY", DISPLAY_NUM)
            .args([
                "-T",
                "Autre appli",
                "-geometry",
                "40x10+700+400",
                "-e",
                "sleep",
                "3600",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        cmd
    });
    wait_until(
        || !find_window_ids("Autre appli").is_empty(),
        Duration::from_secs(5),
        "fenêtre non-jeu visible",
    );
    let other_xid = find_window_ids("Autre appli")[0];
    xdotool(&["windowactivate", &other_xid.to_string()]);
    std::thread::sleep(Duration::from_millis(300));
    let (index, total) = stacking_position(&conn, root, combat_xid);
    assert_eq!(
        index as usize,
        total - 1,
        "pas encore démoté (avant le délai de grâce de {DEMOTE_GRACE:?})"
    );
    std::thread::sleep(DEMOTE_GRACE + Duration::from_millis(500));
    let (index_after, total_after) = stacking_position(&conn, root, combat_xid);
    assert_ne!(
        index_after as usize,
        total_after - 1,
        "toujours au sommet après le délai de grâce (démotion attendue)"
    );
    println!("OK : démoté après le délai de grâce (stacking_index={index_after}, était {index})");

    println!("=== Réaffirmation immédiate au retour de focus ===");
    xdotool(&["windowactivate", &game_xid.to_string()]);
    std::thread::sleep(Duration::from_millis(300));
    let (index, total) = stacking_position(&conn, root, combat_xid);
    assert_eq!(index as usize, total - 1, "réaffirmé immédiatement");
    println!("OK : réaffirmé immédiatement (stacking_index={index}/{total})");

    println!("=== Artefact de revue humaine ===");
    let artifact = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("visual-check.png");
    let status = Command::new("import")
        .env("DISPLAY", DISPLAY_NUM)
        .args(["-window", "root", artifact.to_str().expect("chemin UTF-8")])
        .status()
        .expect("lancement de import (imagemagick)");
    if status.success() {
        println!("image : {}", artifact.display());
    } else {
        eprintln!("capture d'écran échouée (non fatal — les assertions ont déjà toutes passé)");
    }

    std::io::stdout().flush().ok();
    println!("=== Toutes les assertions sont passées ===");
}
