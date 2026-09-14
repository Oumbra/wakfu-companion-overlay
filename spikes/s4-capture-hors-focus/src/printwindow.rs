//! Méthode 1 — `PrintWindow` avec `PW_RENDERFULLCONTENT` : demande à DWM de rendre le contenu de
//! la fenêtre dans un DC mémoire, y compris pour une fenêtre occultée. Réputée ne pas fonctionner
//! (image noire) sur certaines surfaces OpenGL/D3D — c'est précisément ce qu'on veut savoir pour
//! un client Java/JOGL.

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

use crate::Frame;

const PW_CLIENTONLY: u32 = 0x1;
const PW_RENDERFULLCONTENT: u32 = 0x2;

/// Vérification empirique du sens de `startScan` dans `GetDIBits` sur un DIB top-down : lit les
/// `band_h` dernières lignes en supposant `startScan = h - band_h`, et rend aussi la capture
/// complète pour comparer. Utilisé par la sous-commande `dibtest`.
pub fn capture_partial_probe(hwnd: HWND, band_h: i32) -> Option<(Frame, Frame)> {
    let full = capture(hwnd)?;
    let partial = capture_lines(hwnd, band_h)?;
    Some((full, partial))
}

fn capture_lines(hwnd: HWND, band_h: i32) -> Option<Frame> {
    unsafe {
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect).ok()?;
        let w = (rect.right - rect.left).max(0);
        let h = (rect.bottom - rect.top).max(0);
        if w == 0 || h == 0 || band_h > h {
            return None;
        }
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp.into());
        let ok = PrintWindow(hwnd, mem, PRINT_WINDOW_FLAGS(PW_CLIENTONLY | PW_RENDERFULLCONTENT)).as_bool();
        let mut frame = None;
        if ok {
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
            let lines = GetDIBits(mem, bmp, 0, band_h as u32, Some(bgra.as_mut_ptr() as *mut _), &mut info, DIB_RGB_COLORS);
            eprintln!("[dibtest] GetDIBits(startScan={}, cLines={}) -> {} lignes", 0, band_h, lines);
            if lines == band_h {
                for px in bgra.chunks_exact_mut(4) { px.swap(0, 2); px[3] = 255; }
                frame = Some(Frame { width: w as u32, height: band_h as u32, rgba: bgra });
            }
        }
        SelectObject(mem, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        frame
    }
}

pub fn capture(hwnd: HWND) -> Option<Frame> {
    unsafe {
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect).ok()?;
        let w = (rect.right - rect.left).max(0);
        let h = (rect.bottom - rect.top).max(0);
        if w == 0 || h == 0 {
            return None;
        }

        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp.into());

        let ok = PrintWindow(
            hwnd,
            mem,
            PRINT_WINDOW_FLAGS(PW_CLIENTONLY | PW_RENDERFULLCONTENT),
        )
        .as_bool();

        let mut frame = None;
        if ok {
            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bgra = vec![0u8; (w * h * 4) as usize];
            let lines = GetDIBits(
                mem,
                bmp,
                0,
                h as u32,
                Some(bgra.as_mut_ptr() as *mut _),
                &mut info,
                DIB_RGB_COLORS,
            );
            if lines == h {
                for px in bgra.chunks_exact_mut(4) {
                    px.swap(0, 2);
                    px[3] = 255;
                }
                frame = Some(Frame {
                    width: w as u32,
                    height: h as u32,
                    rgba: bgra,
                });
            }
        }

        SelectObject(mem, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        frame
    }
}
