//! Spike S4 — capturer une fenêtre Wakfu **sans focus, voire entièrement occultée** (Windows).
//!
//! Question posée (voir `docs/plan-architecture.md` §9.1 decies et §14 point 7) : une API de
//! capture obtient-elle un contenu VIVANT d'une fenêtre Wakfu (Java/JOGL) qui n'a pas le focus et
//! qu'une autre fenêtre recouvre ? La vidéo du 2026-09-14 a établi que le client continue de
//! rendre en arrière-plan ; reste à prouver que la capture programmatique le voit.
//!
//! Deux méthodes, comparées côte à côte sur les mêmes fenêtres au même instant :
//!
//! - `printwindow` : `PrintWindow(hwnd, hdc, PW_CLIENTONLY | PW_RENDERFULLCONTENT)` — GDI, la voie
//!   la plus simple, réputée capricieuse sur OpenGL.
//! - `wgc` : Windows Graphics Capture (`Windows.Graphics.Capture`, WinRT) — passe par la
//!   composition DWM, ne dépend pas de ce que l'application accepte de peindre dans un DC.
//!
//! Le binaire prend toutes les fenêtres `"<Nom> - WAKFU"` trouvées, les capture à intervalle fixe
//! pendant N secondes, écrit chaque capture en PNG plus un recadrage du coin bas-droit (là où vit
//! le widget « Fin du tour », dont le chrono change chaque seconde — c'est LUI qui prouve que la
//! capture est vivante), et conclut par fenêtre et par méthode : VIVANT, FIGÉ ou NOIR.
//!
//! Usage :
//!   cargo run --release -- list
//!   cargo run --release -- capture [--method both|printwindow|wgc] [--seconds 12] [--interval 1000] [--out captures]
//!
//! Protocole de test (à faire À LA MAIN pendant que le binaire tourne) : deux clients Wakfu en
//! combat, l'un recouvrant l'autre entièrement, puis le focus sur une troisième application. Le
//! rapport final dit ce que chaque méthode a vu de la fenêtre recouverte.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetAncestor, GetClientRect, GetForegroundWindow, GetWindowRect, GetWindowTextW,
    IsIconic, IsWindowVisible, WindowFromPoint, GA_ROOT,
};

mod printwindow;
mod wgc;

/// Une capture RGBA8, top-down, `width * height * 4` octets.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Frame {
    /// Part de pixels « quasi noirs » (les trois canaux < 8) — une capture qui a échoué sans le
    /// dire (PrintWindow sur une surface OpenGL) revient souvent entièrement noire.
    fn black_ratio(&self) -> f64 {
        let n = (self.width * self.height) as usize;
        if n == 0 {
            return 1.0;
        }
        let black = self
            .rgba
            .chunks_exact(4)
            .filter(|p| p[0] < 8 && p[1] < 8 && p[2] < 8)
            .count();
        black as f64 / n as f64
    }

    /// Recadrage du coin bas-droit — le widget « Fin du tour » y vit (voir plan §9.1 decies), et
    /// son chrono change chaque seconde : deux recadrages successifs identiques signent une
    /// capture figée.
    fn bottom_right(&self, w: u32, h: u32) -> Frame {
        let w = w.min(self.width);
        let h = h.min(self.height);
        let x0 = self.width - w;
        let y0 = self.height - h;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in y0..self.height {
            let start = ((y * self.width + x0) * 4) as usize;
            rgba.extend_from_slice(&self.rgba[start..start + (w * 4) as usize]);
        }
        Frame {
            width: w,
            height: h,
            rgba,
        }
    }

    /// Différence moyenne par canal entre deux images de même taille (0 = identiques).
    fn mean_diff(&self, other: &Frame) -> f64 {
        if self.width != other.width || self.height != other.height || self.rgba.is_empty() {
            return f64::NAN;
        }
        let sum: u64 = self
            .rgba
            .iter()
            .zip(&other.rgba)
            .map(|(a, b)| (*a as i32 - *b as i32).unsigned_abs() as u64)
            .sum();
        sum as f64 / self.rgba.len() as f64
    }

    fn save_png(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let file = fs::File::create(path)?;
        let mut enc = png::Encoder::new(file, self.width, self.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header()?;
        w.write_image_data(&self.rgba)?;
        Ok(())
    }
}

/// Une fenêtre de jeu trouvée par son titre `"<Nom> - WAKFU"` — même critère que
/// `overlay-ui::game_window` (§6.5 du plan : le process est un `java` générique, seul le titre
/// discrimine).
#[derive(Clone)]
pub struct GameWindow {
    pub hwnd: HWND,
    pub character: String,
    pub client: (u32, u32),
    pub visible: bool,
    pub minimized: bool,
}

/// `title_filter` : `None` = les fenêtres `"<Nom> - WAKFU"` (le cas réel) ; `Some(sub)` = toute
/// fenêtre dont le titre contient `sub` — pour valider la mécanique de capture sans le jeu.
fn list_game_windows(title_filter: Option<&str>) -> Vec<GameWindow> {
    struct Ctx<'a> {
        out: Vec<GameWindow>,
        filter: Option<&'a str>,
    }
    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let ctx = unsafe { &mut *(lparam.0 as *mut Ctx) };
        let out = &mut ctx.out;
        let mut buf = [0u16; 512];
        let len = unsafe { GetWindowTextW(hwnd, &mut buf) } as usize;
        if len == 0 {
            return true.into();
        }
        let title = String::from_utf16_lossy(&buf[..len]);
        let matched = match ctx.filter {
            None => title.strip_suffix(" - WAKFU").map(str::to_string),
            Some(sub) if title.contains(sub) => Some(title.clone()),
            Some(_) => None,
        };
        if let Some(character) = matched {
            let mut rect = RECT::default();
            let _ = unsafe { GetClientRect(hwnd, &mut rect) };
            out.push(GameWindow {
                hwnd,
                character: character.to_string(),
                client: (
                    (rect.right - rect.left).max(0) as u32,
                    (rect.bottom - rect.top).max(0) as u32,
                ),
                visible: unsafe { IsWindowVisible(hwnd) }.as_bool(),
                minimized: unsafe { IsIconic(hwnd) }.as_bool(),
            });
        }
        true.into()
    }
    let mut ctx = Ctx {
        out: Vec::new(),
        filter: title_filter,
    };
    let _ = unsafe { EnumWindows(Some(enum_cb), LPARAM(&mut ctx as *mut _ as isize)) };
    ctx.out
}

/// Part de la fenêtre recouverte par d'autres, mesurée en sondant une grille de points de son
/// rectangle avec `WindowFromPoint` : un point dont la fenêtre racine n'est pas `hwnd` est
/// recouvert (ou hors écran). 0.0 = entièrement visible, 1.0 = entièrement cachée. C'est cette
/// mesure, croisée avec le contenu capturé, qui prouve qu'une API voit une fenêtre occultée.
fn occlusion(hwnd: HWND) -> f64 {
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return 1.0;
    }
    let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
    if w <= 0 || h <= 0 {
        return 1.0;
    }
    const N: i32 = 7;
    let mut hidden = 0;
    for iy in 0..N {
        for ix in 0..N {
            // Grille strictement intérieure (jamais sur le bord, où vit le cadre).
            let p = POINT {
                x: rect.left + w * (2 * ix + 1) / (2 * N),
                y: rect.top + h * (2 * iy + 1) / (2 * N),
            };
            let under = unsafe { GetAncestor(WindowFromPoint(p), GA_ROOT) };
            if under.0 != hwnd.0 {
                hidden += 1;
            }
        }
    }
    hidden as f64 / (N * N) as f64
}

fn is_foreground(hwnd: HWND) -> bool {
    let fg = unsafe { GetForegroundWindow() };
    fg.0 == hwnd.0
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Method {
    PrintWindow,
    Wgc,
}

impl Method {
    fn label(self) -> &'static str {
        match self {
            Method::PrintWindow => "printwindow",
            Method::Wgc => "wgc",
        }
    }
}

struct Args {
    methods: Vec<Method>,
    seconds: u64,
    interval_ms: u64,
    out: PathBuf,
    title: Option<String>,
}

fn parse_args() -> (String, Args) {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().unwrap_or_else(|| "capture".to_string());
    let mut args = Args {
        methods: vec![Method::PrintWindow, Method::Wgc],
        seconds: 12,
        interval_ms: 1000,
        out: PathBuf::from("captures"),
        title: None,
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "--method" => {
                args.methods = match it.next().as_deref() {
                    Some("printwindow") => vec![Method::PrintWindow],
                    Some("wgc") => vec![Method::Wgc],
                    _ => vec![Method::PrintWindow, Method::Wgc],
                }
            }
            "--seconds" => args.seconds = it.next().and_then(|v| v.parse().ok()).unwrap_or(12),
            "--interval" => {
                args.interval_ms = it.next().and_then(|v| v.parse().ok()).unwrap_or(1000)
            }
            "--out" => args.out = PathBuf::from(it.next().unwrap_or_default()),
            "--title" => args.title = it.next(),
            other => eprintln!("argument ignoré : {other}"),
        }
    }
    (cmd, args)
}

/// Largeur/hauteur du recadrage « widget » : le coin bas-droit, assez large pour contenir le
/// widget « Fin du tour » quelle que soit l'échelle d'interface raisonnable.
const WIDGET_CROP: (u32, u32) = (360, 200);

fn main() {
    let (cmd, args) = parse_args();
    let windows = list_game_windows(args.title.as_deref());

    if windows.is_empty() {
        eprintln!("Aucune fenêtre \"<Nom> - WAKFU\" trouvée — lancer le jeu d'abord.");
        std::process::exit(2);
    }

    println!("Fenêtres Wakfu trouvées :");
    for w in &windows {
        println!(
            "  {:<24} hwnd={:?} client={}x{} visible={} minimisée={} premier plan={}",
            w.character,
            w.hwnd.0,
            w.client.0,
            w.client.1,
            w.visible,
            w.minimized,
            is_foreground(w.hwnd)
        );
    }
    if cmd == "list" {
        return;
    }

    fs::create_dir_all(&args.out).expect("création du dossier de sortie");
    println!(
        "\nCapture pendant {} s, toutes les {} ms, méthodes {:?} → {}",
        args.seconds,
        args.interval_ms,
        args.methods.iter().map(|m| m.label()).collect::<Vec<_>>(),
        args.out.display()
    );
    println!("→ Pendant ce temps : recouvre une fenêtre par l'autre, puis passe sur une autre application.\n");

    // Une session WGC par fenêtre, ouverte une fois pour toute la durée.
    let mut wgc_sessions: BTreeMap<String, wgc::Session> = BTreeMap::new();
    if args.methods.contains(&Method::Wgc) {
        wgc::init().expect("initialisation WinRT / D3D11");
        for w in &windows {
            match wgc::Session::open(w.hwnd) {
                Ok(s) => {
                    wgc_sessions.insert(w.character.clone(), s);
                }
                Err(e) => eprintln!("[wgc] {} : ouverture impossible — {e}", w.character),
            }
        }
    }

    // Journal : (méthode, personnage) → liste de (tick, foreground, frame widget, ratio noir)
    struct Sample {
        tick: u32,
        foreground: bool,
        occlusion: f64,
        minimized: bool,
        black: f64,
        wgc_frames: u32,
        widget: Frame,
    }
    let mut log: BTreeMap<(Method, String), Vec<Sample>> = BTreeMap::new();

    let start = Instant::now();
    let mut tick = 0u32;
    while start.elapsed() < Duration::from_secs(args.seconds) {
        let tick_start = Instant::now();
        for w in &windows {
            let fg = is_foreground(w.hwnd);
            let iconic = unsafe { IsIconic(w.hwnd) }.as_bool();
            let occ = occlusion(w.hwnd);
            for &m in &args.methods {
                let (frame, wgc_frames) = match m {
                    Method::PrintWindow => (printwindow::capture(w.hwnd), 0),
                    Method::Wgc => match wgc_sessions.get_mut(&w.character) {
                        Some(s) => {
                            let (f, n) = s.latest();
                            (f, n)
                        }
                        None => (None, 0),
                    },
                };
                let Some(frame) = frame else {
                    println!(
                        "t={:>2}  {:<11} {:<24} premier plan={:<5} minimisée={:<5} recouverte={:>3.0}% — aucune image",
                        tick,
                        m.label(),
                        w.character,
                        fg,
                        iconic,
                        occ * 100.0
                    );
                    continue;
                };
                let black = frame.black_ratio();
                let widget = frame.bottom_right(WIDGET_CROP.0, WIDGET_CROP.1);
                let base = format!("{}_{}_{:02}", m.label(), sanitize(&w.character), tick);
                let _ = frame.save_png(&args.out.join(format!("{base}.png")));
                let _ = widget.save_png(&args.out.join(format!("{base}_widget.png")));
                println!(
                    "t={:>2}  {:<11} {:<24} premier plan={:<5} minimisée={:<5} recouverte={:>3.0}% {}x{}  noir={:>5.1}%{}",
                    tick,
                    m.label(),
                    w.character,
                    fg,
                    iconic,
                    occ * 100.0,
                    frame.width,
                    frame.height,
                    black * 100.0,
                    if m == Method::Wgc {
                        format!("  images WGC reçues depuis le tick précédent={wgc_frames}")
                    } else {
                        String::new()
                    }
                );
                log.entry((m, w.character.clone())).or_default().push(Sample {
                    tick,
                    foreground: fg,
                    occlusion: occ,
                    minimized: iconic,
                    black,
                    wgc_frames,
                    widget,
                });
            }
        }
        tick += 1;
        let spent = tick_start.elapsed();
        if let Some(rest) = Duration::from_millis(args.interval_ms).checked_sub(spent) {
            std::thread::sleep(rest);
        }
    }

    println!("\n══════════════ RAPPORT ══════════════");
    for ((m, character), samples) in &log {
        let n = samples.len();
        let black_avg = samples.iter().map(|s| s.black).sum::<f64>() / n.max(1) as f64;
        // Vivant = le recadrage « widget » change d'un tick à l'autre, sur les ticks où la
        // fenêtre n'avait PAS le premier plan (c'est le cas qu'on teste).
        let mut diffs_bg = Vec::new();
        let mut diffs_occ = Vec::new();
        let mut diffs_all = Vec::new();
        for pair in samples.windows(2) {
            let d = pair[0].widget.mean_diff(&pair[1].widget);
            diffs_all.push(d);
            if !pair[0].foreground && !pair[1].foreground {
                diffs_bg.push(d);
            }
            let occluded = |s: &Sample| s.occlusion >= 0.9 && !s.minimized;
            if occluded(&pair[0]) && occluded(&pair[1]) {
                diffs_occ.push(d);
            }
        }
        let mean = |v: &[f64]| {
            if v.is_empty() {
                f64::NAN
            } else {
                v.iter().sum::<f64>() / v.len() as f64
            }
        };
        let changed_bg = diffs_bg.iter().filter(|d| **d > 0.5).count();
        let changed_occ = diffs_occ.iter().filter(|d| **d > 0.5).count();
        let wgc_total: u32 = samples.iter().map(|s| s.wgc_frames).sum();
        let verdict = if black_avg > 0.98 {
            "NOIR   — la méthode ne voit pas cette fenêtre"
        } else if diffs_bg.is_empty() {
            "?      — jamais observée sans le premier plan pendant la capture"
        } else if changed_bg * 2 >= diffs_bg.len() {
            "VIVANT — le widget change tick après tick, fenêtre en arrière-plan"
        } else {
            "FIGÉ   — le contenu ne bouge plus dès que la fenêtre perd le premier plan"
        };
        println!(
            "{:<11} {:<24} captures={:<3} noir moyen={:>5.1}%  Δwidget arrière-plan={:.2} (sur {} paires, {} changent)  Δwidget total={:.2}{}",
            m.label(),
            character,
            n,
            black_avg * 100.0,
            mean(&diffs_bg),
            diffs_bg.len(),
            changed_bg,
            mean(&diffs_all),
            if *m == Method::Wgc {
                format!("  images WGC={wgc_total}")
            } else {
                String::new()
            }
        );
        println!("            → {verdict}");
        let ticks_bg: Vec<u32> = samples.iter().filter(|s| !s.foreground).map(|s| s.tick).collect();
        println!("            ticks sans premier plan : {ticks_bg:?}");
        let ticks_occ: Vec<u32> = samples.iter().filter(|s| s.occlusion >= 0.9 && !s.minimized).map(|s| s.tick).collect();
        let ticks_min: Vec<u32> = samples.iter().filter(|s| s.minimized).map(|s| s.tick).collect();
        if !ticks_min.is_empty() {
            let frames_min: u32 = samples.iter().filter(|s| s.minimized).map(|s| s.wgc_frames).sum();
            println!(
                "            ticks minimisée : {ticks_min:?}{}",
                if *m == Method::Wgc { format!("  images WGC reçues pendant : {frames_min} (0 = DWM ne compose plus, image figée)") } else { String::new() }
            );
        }
        if diffs_occ.is_empty() {
            println!("            ticks recouverte ≥ 90 % : aucun — l'occultation n'a pas été testée");
        } else {
            println!(
                "            ticks recouverte ≥ 90 % : {ticks_occ:?}  Δwidget={:.2} ({} paires, {} changent) → {}",
                mean(&diffs_occ),
                diffs_occ.len(),
                changed_occ,
                if changed_occ * 2 >= diffs_occ.len() { "VIVANT sous occultation" } else { "FIGÉ sous occultation" }
            );
        }
    }
    println!("\nCaptures dans {} — ouvrir les *_widget.png pour voir le chrono.", args.out.display());
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}
