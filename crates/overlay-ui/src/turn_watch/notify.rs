//! Notification du système — Windows, toast WinRT (`Windows.UI.Notifications`).
//!
//! Directement par le crate `windows`, déjà présent : aucune dépendance de plus, et le contrôle
//! de tout ce que le toast montre et fait.
//!
//! ## L'identité de l'émetteur
//!
//! Un toast ne s'affiche que sous un `AppUserModelID` **enregistré**. Un installeur le ferait par
//! un raccourci du menu Démarrer ; l'overlay n'en a pas (§14 point 6 du plan), et la première
//! version empruntait celui de PowerShell — toast au nom et à l'icône de « Windows PowerShell »,
//! refusé par l'utilisateur le 2026-09-14. Windows 10 1709+ offre une voie sans raccourci ni
//! droits administrateur : une clé `HKCU\Software\Classes\AppUserModelId\<AUMID>` portant
//! `DisplayName` et `IconUri`. [`register_identity`] l'écrit au démarrage (idempotent, quelques
//! octets, réversible en supprimant la clé), dépose le logo de l'overlay en PNG à côté de sa
//! config pour `IconUri`, et déclare l'AUMID au process. Le toast porte alors le nom et le logo
//! de l'overlay.
//!
//! ## Le clic
//!
//! Cliquer le toast doit **amener au premier plan la fenêtre du personnage** — c'est tout l'objet :
//! la notification dit « c'est à toi », le clic y va. Un process qui n'a pas le premier plan n'a
//! pas le droit de le donner : depuis le gestionnaire `Activated` du toast, `SetForegroundWindow`
//! était refusé et la fenêtre **clignotait** dans la barre des tâches (vu en jeu le 2026-09-14,
//! vidéo à l'appui, malgré la frappe Alt synthétique, le rattachement à la file d'entrée et
//! `SwitchToThisWindow`). La voie que Windows prévoit pour une application non empaquetée est
//! l'**activation par protocole** : le toast porte `launch="wakfu-companion:focus?hwnd=…"`, le clic
//! lance le gestionnaire de ce protocole — l'overlay lui-même, enregistré par
//! [`register_identity`] — dans un **nouveau process**, et un process que le shell vient de lancer
//! sur une action de l'utilisateur a le droit de donner le premier plan. Ce process ne fait que ça
//! ([`focus_window`]) et se termine (voir `main.rs`, tout début de `main`).
//!
//! ## Le son
//!
//! Un toast d'application non empaquetée ne peut jouer que les sons système (`ms-winsoundevent:`)
//! — jugés insipides. Le toast est donc **silencieux**, et c'est l'overlay qui joue son propre son
//! (`alert_sound::play_turn_alert`), comme pour les alertes de ramassage.

use windows::core::{HSTRING, PCWSTR};
use windows::Data::Xml::Dom::XmlDocument;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_WRITE,
    REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_MENU,
};
use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, SetForegroundWindow,
    ShowWindow, SwitchToThisWindow, SW_RESTORE,
};
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

/// Identifiant d'application de l'overlay pour le centre de notifications — le jour de
/// l'installeur, c'est lui que le raccourci du menu Démarrer portera.
pub const APP_USER_MODEL_ID: &str = "Oumbra.WakfuCompanionOverlay";
/// Ce que le centre de notifications affiche comme émetteur.
const DISPLAY_NAME: &str = "Wakfu Companion Overlay";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Enregistre l'identité de l'overlay pour les toasts — voir doc de module. `icon_png` est le
/// logo tel qu'embarqué ; il est déposé dans `dir` pour que le centre de notifications le lise.
/// Best-effort : une erreur est journalisée, et le toast tombera alors sur une icône générique.
pub fn register_identity(dir: &std::path::Path, icon_png: &[u8]) {
    let icon_path = dir.join("app-icon.png");
    if let Err(err) =
        std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&icon_path, icon_png))
    {
        tracing::warn!("[tour] icône de notification non déposée : {err}");
    }
    unsafe {
        let subkey = wide(&format!(
            "Software\\Classes\\AppUserModelId\\{APP_USER_MODEL_ID}"
        ));
        let mut key = HKEY::default();
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            None,
        );
        if status.is_err() {
            tracing::warn!("[tour] identité de notification non enregistrée : {status:?}");
            return;
        }
        for (name, value) in [
            ("DisplayName", DISPLAY_NAME.to_string()),
            ("IconUri", icon_path.display().to_string()),
        ] {
            let name_w = wide(name);
            let value_w = wide(&value);
            let bytes =
                std::slice::from_raw_parts(value_w.as_ptr() as *const u8, value_w.len() * 2);
            let status = RegSetValueExW(key, PCWSTR(name_w.as_ptr()), None, REG_SZ, Some(bytes));
            if status.is_err() {
                tracing::warn!("[tour] valeur {name} non écrite : {status:?}");
            }
        }
        let _ = RegCloseKey(key);
        register_protocol();
        if let Err(err) =
            SetCurrentProcessExplicitAppUserModelID(PCWSTR(wide(APP_USER_MODEL_ID).as_ptr()))
        {
            tracing::warn!("[tour] AppUserModelID du process non posé : {err}");
        }
    }
    tracing::info!("[tour] identité de notification : {DISPLAY_NAME} ({APP_USER_MODEL_ID})");
}

/// Schéma d'URI du protocole d'activation — `wakfu-companion:focus?hwnd=<entier>`.
pub const PROTOCOL: &str = "wakfu-companion";

/// Enregistre l'overlay comme gestionnaire du protocole [`PROTOCOL`] (HKCU, pas de droits) :
/// `HKCU\Software\Classes\wakfu-companion` avec `URL Protocol`, et `shell\open\command`
/// vers l'exécutable courant. Rejoué à chaque démarrage : si l'exécutable a bougé, la commande
/// suit.
fn register_protocol() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    unsafe {
        let set = |subkey: &str, name: Option<&str>, value: &str| {
            let subkey_w = wide(subkey);
            let mut key = HKEY::default();
            let status = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey_w.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut key,
                None,
            );
            if status.is_err() {
                tracing::warn!("[tour] protocole : clé {subkey} non créée : {status:?}");
                return;
            }
            let name_w = name.map(wide);
            let value_w = wide(value);
            let bytes =
                std::slice::from_raw_parts(value_w.as_ptr() as *const u8, value_w.len() * 2);
            let name_ptr = name_w
                .as_ref()
                .map_or(PCWSTR::null(), |n| PCWSTR(n.as_ptr()));
            let status = RegSetValueExW(key, name_ptr, None, REG_SZ, Some(bytes));
            if status.is_err() {
                tracing::warn!("[tour] protocole : valeur non écrite dans {subkey} : {status:?}");
            }
            let _ = RegCloseKey(key);
        };
        let root = format!("Software\\Classes\\{PROTOCOL}");
        set(&root, None, "URL:Wakfu Companion Overlay");
        set(&root, Some("URL Protocol"), "");
        set(
            &format!("{root}\\shell\\open\\command"),
            None,
            &format!("\"{}\" \"%1\"", exe.display()),
        );
    }
}

/// L'URI que le clic du toast lance — voir [`parse_focus_uri`] pour l'inverse.
fn focus_uri(hwnd: isize) -> String {
    format!("{PROTOCOL}:focus?hwnd={hwnd}")
}

/// La `HWND` portée par une URI de focus, si `arg` en est une — ce que `main` teste sur son
/// premier argument pour savoir s'il est lancé par un clic de toast plutôt que par l'utilisateur.
pub fn parse_focus_uri(arg: &str) -> Option<isize> {
    let rest = arg.strip_prefix(&format!("{PROTOCOL}:"))?;
    let rest = rest.strip_prefix("focus?hwnd=")?;
    rest.trim_end_matches('/').parse().ok()
}

/// Affiche un toast silencieux titre + corps dont le clic amène `hwnd` au premier plan (par
/// activation de protocole, voir doc de module). Une erreur est rendue, jamais masquée :
/// l'appelant journalise, et la surveillance de tour continue.
pub fn show(title: &str, body: &str, hwnd: isize) -> windows::core::Result<()> {
    unsafe {
        // Le thread winit a déjà son apartment COM ; WinRT s'en accommode, et un
        // `RPC_E_CHANGED_MODE` ici n'empêche pas les appels qui suivent.
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let xml = format!(
        "<toast activationType=\"protocol\" launch=\"{}\" scenario=\"reminder\">\
         <visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual>\
         <audio silent=\"true\"/></toast>",
        escape(&focus_uri(hwnd)),
        escape(title),
        escape(body)
    );
    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml))?;
    let toast = ToastNotification::CreateToastNotification(&doc)?;
    let notifier =
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_USER_MODEL_ID))?;
    notifier.Show(&toast)
}

/// Amène `hwnd` au premier plan — restaurée si minimisée. Appelée depuis le process lancé par le
/// clic du toast (activation de protocole, doc de module), qui a le droit de le faire ; les
/// leviers ci-dessous restent, pour le cas où Windows le lui disputerait quand même :
///
/// 1. une frappe **Alt** synthétique (appui puis relâchement) : le dernier process à avoir produit
///    une entrée clavier obtient le droit — c'est le contournement établi de longue date ;
/// 2. le rattachement à la file d'entrée du thread au premier plan (`AttachThreadInput`) ;
/// 3. `SwitchToThisWindow`, le mécanisme d'Alt-Tab lui-même, qui ne demande pas de permission.
pub fn focus_window(hwnd: isize) {
    let hwnd = HWND(hwnd as *mut _);
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let tap = |vk, flags| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        };
        let alt = [
            tap(VK_MENU, Default::default()),
            tap(VK_MENU, KEYEVENTF_KEYUP),
        ];
        SendInput(&alt, std::mem::size_of::<INPUT>() as i32);
        if SetForegroundWindow(hwnd).as_bool() {
            tracing::info!("[tour] fenêtre de jeu amenée au premier plan.");
            return;
        }
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, None);
        let me = GetCurrentThreadId();
        let attached =
            fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, true).as_bool();
        let _ = BringWindowToTop(hwnd);
        let ok = SetForegroundWindow(hwnd).as_bool();
        if attached {
            let _ = AttachThreadInput(me, fg_thread, false);
        }
        if ok {
            tracing::info!("[tour] fenêtre de jeu amenée au premier plan (après rattachement).");
            return;
        }
        SwitchToThisWindow(hwnd, true);
        if GetForegroundWindow().0 == hwnd.0 {
            tracing::info!("[tour] fenêtre de jeu amenée au premier plan (SwitchToThisWindow).");
        } else {
            tracing::warn!(
                "[tour] Windows a refusé de donner le premier plan à la fenêtre de jeu."
            );
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_uri_de_focus_fait_l_aller_retour() {
        let uri = focus_uri(0x1234_5678);
        assert_eq!(uri, "wakfu-companion:focus?hwnd=305419896");
        assert_eq!(parse_focus_uri(&uri), Some(0x1234_5678));
        // Le shell peut ajouter un `/` final en normalisant l'URI.
        assert_eq!(parse_focus_uri("wakfu-companion:focus?hwnd=42/"), Some(42));
        assert_eq!(parse_focus_uri("autre:focus?hwnd=42"), None);
        assert_eq!(parse_focus_uri("--log"), None);
    }
}
