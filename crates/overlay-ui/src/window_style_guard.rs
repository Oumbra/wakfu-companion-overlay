//! **Verrou des styles étendus des fenêtres overlay** (Windows, 2026-09-30).
//!
//! ## Le défaut corrigé
//!
//! Retour utilisateur : Alt+Échap (qui envoie la fenêtre active au fond de l'ordre Z et active la
//! suivante) tombait sur un bandeau de l'overlay au lieu de la fenêtre suivante ; le focus clavier
//! partait dans un bandeau qui n'en fait rien.
//!
//! Les bandeaux portaient pourtant `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`, posés à la main après
//! leur création. Mais **`winit` 0.30 réécrit le style étendu en entier** à chaque changement de
//! ses drapeaux (`WindowFlags::apply_diff` → `SetWindowLongW(GWL_EXSTYLE, …)`, recalculé depuis
//! son cache par `to_window_styles`) : `set_cursor_hittest` (bascule interactif / clic-traversant)
//! ou `set_visible` (réaffichage de Combat / Récap) effaçaient les deux styles et posaient
//! `WS_EX_APPWINDOW` (fenêtre sans propriétaire). Les bandeaux redevenaient des fenêtres normales,
//! activables, présentes dans Alt+Tab, et en tête de l'ordre Z puisque topmost : Alt+Échap depuis
//! le jeu les activait l'un après l'autre. Le même effacement explique le constat relevé dans
//! `sync_topmost` (« un clic en mode interactif fait de lui la fenêtre au premier plan malgré
//! `WS_EX_NOACTIVATE` »).
//!
//! ## Le mécanisme
//!
//! Plutôt que de réappliquer les styles après chaque appel `winit` (fragile : un appel oublié
//! réintroduit le défaut), la fenêtre est sous-classée et **corrige le style avant que Windows ne
//! l'applique** :
//!
//! - `WM_STYLECHANGING` (`GWL_EXSTYLE`) : la doc autorise la fenêtre à modifier
//!   `STYLESTRUCT::styleNew`. Les styles verrouillés y sont remis, `WS_EX_APPWINDOW` retiré.
//!   Le cache de `winit` reste faux, sans conséquence : chaque réécriture repasse par ce filtre.
//! - `WM_MOUSEACTIVATE` : `MA_NOACTIVATE` pour une fenêtre non activable. Couvre le cas où
//!   Windows ne consulte pas `WS_EX_NOACTIVATE` (suivi actif des fenêtres au survol, Raymond Chen,
//!   « The Old New Thing », 2024-09-19).
//!
//! Sous-classement « classique » par `GWLP_WNDPROC` + `CallWindowProcW` plutôt que
//! `SetWindowSubclass` : ce dernier vit dans `comctl32`, que l'exe (sans manifeste) chargerait en
//! version 5, et `GWLP_USERDATA` est déjà pris par `winit`. La procédure d'origine est gardée par
//! fenêtre dans une table, retirée à `WM_NCDESTROY`.

use std::sync::Mutex;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, GetWindowLongPtrW, SetWindowLongPtrW, GWLP_WNDPROC, GWL_EXSTYLE,
    MA_NOACTIVATE, STYLESTRUCT, WM_MOUSEACTIVATE, WM_NCDESTROY, WM_STYLECHANGING, WNDPROC,
    WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

/// Une fenêtre sous-classée : sa `HWND`, sa procédure d'origine, ses styles verrouillés.
/// Les `HWND` sont stockées en `isize` : `HWND` n'est ni `Send` ni `Sync`.
struct Guarded {
    hwnd: isize,
    original: isize,
    locked: u32,
}

/// Une poignée de fenêtres au plus (trois bandeaux par client, plus Options) : une `Vec` suffit.
static GUARDED: Mutex<Vec<Guarded>> = Mutex::new(Vec::new());

/// Styles d'un bandeau (Combat, Suivi, Récap, bouton œil) : jamais activable, hors Alt+Tab.
pub const OVERLAY: u32 = WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0;
/// Styles d'une fenêtre qui doit recevoir le clavier (Options, confirmation) : hors Alt+Tab
/// seulement.
pub const FOCUSABLE: u32 = WS_EX_TOOLWINDOW.0;

/// Verrouille `locked` dans le style étendu de `hwnd` et l'applique tout de suite. Sans effet si
/// la fenêtre est déjà gardée (les styles sont seulement réappliqués).
pub fn install(hwnd: HWND, locked: u32) {
    let key = hwnd.0 as isize;
    {
        let mut guarded = GUARDED.lock().unwrap_or_else(|e| e.into_inner());
        if !guarded.iter().any(|g| g.hwnd == key) {
            // SAFETY : `hwnd` est une fenêtre vivante créée par ce thread ; `guard_proc` a la
            // signature d'une `WNDPROC`.
            let original =
                unsafe { SetWindowLongPtrW(hwnd, GWLP_WNDPROC, guard_proc as *const () as isize) };
            if original == 0 {
                tracing::warn!(
                    "[style] sous-classement impossible : {:?}",
                    windows::core::Error::from_thread()
                );
                return;
            }
            guarded.push(Guarded {
                hwnd: key,
                original,
                locked,
            });
        }
    }
    reapply(hwnd);
}

/// Réécrit le style étendu courant : le filtre de `WM_STYLECHANGING` y remet les styles
/// verrouillés. Utile après `install`, et pour le raccourci Refresh.
pub fn reapply(hwnd: HWND) {
    // SAFETY : lecture/écriture du style d'une fenêtre de ce thread.
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, current);
    }
}

/// Le style étendu tel que la fenêtre le recevra : verrouillés ajoutés, `WS_EX_APPWINDOW` retiré
/// (il remettrait la fenêtre dans Alt+Tab malgré `WS_EX_TOOLWINDOW`).
fn filtered(style: u32, locked: u32) -> u32 {
    (style | locked) & !WS_EX_APPWINDOW.0
}

unsafe extern "system" fn guard_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let key = hwnd.0 as isize;
    // Copie hors du verrou : la procédure d'origine peut renvoyer un message à cette même fenêtre
    // (réentrance), qui reprendrait le verrou.
    let found = GUARDED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .find(|g| g.hwnd == key)
        .map(|g| (g.original, g.locked));
    let Some((original, locked)) = found else {
        // Inatteignable tant que la table est tenue à jour ; rien de mieux à faire que Windows.
        return unsafe {
            windows::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam)
        };
    };
    // SAFETY : `original` vient de `SetWindowLongPtrW(GWLP_WNDPROC)`, c'est une `WNDPROC` valide.
    let original: WNDPROC = unsafe { std::mem::transmute::<isize, WNDPROC>(original) };

    match msg {
        WM_STYLECHANGING if wparam.0 as i32 == GWL_EXSTYLE.0 => {
            // SAFETY : pour `WM_STYLECHANGING`, `lparam` pointe sur un `STYLESTRUCT` modifiable.
            if let Some(styles) = unsafe { (lparam.0 as *mut STYLESTRUCT).as_mut() } {
                styles.styleNew = filtered(styles.styleNew, locked);
            }
            unsafe { CallWindowProcW(original, hwnd, msg, wparam, lparam) }
        }
        WM_MOUSEACTIVATE if locked & WS_EX_NOACTIVATE.0 != 0 => LRESULT(MA_NOACTIVATE as isize),
        WM_NCDESTROY => {
            let result = unsafe { CallWindowProcW(original, hwnd, msg, wparam, lparam) };
            GUARDED
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .retain(|g| g.hwnd != key);
            result
        }
        _ => unsafe { CallWindowProcW(original, hwnd, msg, wparam, lparam) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtre_remet_les_styles_et_retire_appwindow() {
        // Le style que `winit` écrit après `set_cursor_hittest(false)` : APPWINDOW, sans les nôtres.
        let winit = WS_EX_APPWINDOW.0 | 0x0008_0000 /* LAYERED */ | 0x20 /* TRANSPARENT */;
        let out = filtered(winit, OVERLAY);
        assert_eq!(out & WS_EX_APPWINDOW.0, 0);
        assert_eq!(out & OVERLAY, OVERLAY);
        assert_eq!(out & (0x0008_0000 | 0x20), 0x0008_0000 | 0x20);
    }

    #[test]
    fn fenetre_focalisable_reste_activable() {
        let out = filtered(WS_EX_APPWINDOW.0, FOCUSABLE);
        assert_eq!(out & WS_EX_NOACTIVATE.0, 0);
        assert_ne!(out & WS_EX_TOOLWINDOW.0, 0);
    }
}
