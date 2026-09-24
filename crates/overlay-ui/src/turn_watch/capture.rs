//! Capture de la **bande basse** d'une fenêtre de jeu — Windows, `PrintWindow`.
//!
//! Choix tranché par le spike S4 (`spikes/s4-capture-hors-focus/`, 2026-09-14) : `PrintWindow`
//! avec `PW_RENDERFULLCONTENT` obtient un contenu vivant d'une fenêtre Wakfu recouverte à 100 %,
//! par l'autre client comme par une application tierce — sa réputation sur OpenGL ne s'est pas
//! vérifiée sur ce client Java/JOGL. Synchrone, à la demande, aucune session à tenir : c'est ce
//! qui la fait préférer à Windows Graphics Capture, gardée en repli documenté (§9.1 decies).
//!
//! Le rendu porte sur toute la zone client (DWM ne sait pas faire moins), mais **la copie vers la
//! mémoire ne prend que les dernières lignes** (`GetDIBits`, `startScan`/`cLines`) : le widget vit
//! en bas de la fenêtre, [`BAND_HEIGHT`] lignes suffisent — 2 Mo par lecture en 2560 × 1392 au
//! lieu de 14.
//!
//! Une fenêtre **minimisée** est hors d'atteinte (S4 : `PrintWindow` renvoie faux) : `IsIconic` le
//! dit avant d'essayer, et la fonction rend `None` — jamais une image noire qui passerait pour
//! « pas de nom affiché ».

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, IsIconic};

use super::vision::Band;

const PW_CLIENTONLY: u32 = 0x1;
const PW_RENDERFULLCONTENT: u32 = 0x2;

/// Hauteur de la bande lue, en pixels : le widget fait ~140 px plus ~30 de bande de nom à
/// l'échelle 100 %, ancré en bas. 400 lignes couvrent jusqu'à une échelle d'interface de ×2,5.
pub const BAND_HEIGHT: u32 = 400;

/// La bande basse de la zone client de `hwnd`, ou `None` (minimisée, taille nulle, échec GDI).
pub fn capture_bottom_band(hwnd: HWND) -> Option<Band> {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            return None;
        }
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect).ok()?;
        let w = (rect.right - rect.left).max(0);
        let h = (rect.bottom - rect.top).max(0);
        if w == 0 || h == 0 {
            return None;
        }
        let band_h = (h as u32).min(BAND_HEIGHT) as i32;

        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp.into());

        let printed = PrintWindow(
            hwnd,
            mem,
            PRINT_WINDOW_FLAGS(PW_CLIENTONLY | PW_RENDERFULLCONTENT),
        )
        .as_bool();

        let mut band = None;
        if printed {
            // `startScan`/`cLines` se comptent depuis le BAS du bitmap source, quel que soit le
            // signe de `biHeight` — vérifié empiriquement (S4, sous-commande `dibtest`, 2026-09-14 :
            // `startScan = h - band_h` rendait le HAUT de la fenêtre). `startScan = 0` désigne donc
            // les `band_h` dernières lignes à l'écran, et `biHeight` négatif les range top-down
            // dans le tampon : la ligne 0 du tampon est la plus haute de la bande.
            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bgra = vec![0u8; (w * band_h * 4) as usize];
            let lines = GetDIBits(
                mem,
                bmp,
                0,
                band_h as u32,
                Some(bgra.as_mut_ptr() as *mut _),
                &mut info,
                DIB_RGB_COLORS,
            );
            if lines == band_h {
                // `as_chunks_mut` plutôt que `chunks_exact_mut(4)` : la taille est une constante,
                // le compilateur la connaît donc au lieu de la déduire à l'exécution (et clippy
                // le réclame). Ce lint n'était vu de personne — `clippy-linux` ne compile pas ce
                // fichier, réservé à Windows, et le job `build-windows` ne lançait que `build`.
                for px in bgra.as_chunks_mut::<4>().0 {
                    px.swap(0, 2);
                    px[3] = 255;
                }
                band = Some(Band {
                    width: w as u32,
                    height: band_h as u32,
                    rgba: bgra,
                });
            }
        }

        SelectObject(mem, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        band
    }
}
