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
//! la notification dit « c'est à toi », le clic y va. `ToastNotification::Activated` livre le clic
//! au process tant qu'il tourne ; l'objet toast doit rester vivant pour recevoir l'événement, d'où
//! [`Toast`] que l'appelant conserve le temps que la notification puisse encore être cliquée.
//! Windows accorde le premier plan au process qu'un toast active, mais pas toujours (règles de
//! `SetForegroundWindow`) : [`focus_window`] tente l'appel direct, puis se rattache à la file
//! d'entrée du thread au premier plan pour réessayer.
//!
//! ## Le son
//!
//! Un toast d'application non empaquetée ne peut jouer que les sons système (`ms-winsoundevent:`)
//! — jugés insipides. Le toast est donc **silencieux**, et c'est l'overlay qui joue son propre son
//! (`alert_sound::play_turn_alert`), comme pour les alertes de ramassage.

use windows::core::{HSTRING, PCWSTR};
use windows::Data::Xml::Dom::XmlDocument;
use windows::Foundation::TypedEventHandler;
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

/// Un toast affiché, à garder vivant tant qu'il peut être cliqué (voir doc de module).
pub struct Toast {
    _inner: ToastNotification,
}

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
        if let Err(err) =
            SetCurrentProcessExplicitAppUserModelID(PCWSTR(wide(APP_USER_MODEL_ID).as_ptr()))
        {
            tracing::warn!("[tour] AppUserModelID du process non posé : {err}");
        }
    }
    tracing::info!("[tour] identité de notification : {DISPLAY_NAME} ({APP_USER_MODEL_ID})");
}

/// Affiche un toast silencieux titre + corps ; `on_click` est appelé (sur un thread du système)
/// quand l'utilisateur le clique. Une erreur est rendue, jamais masquée : l'appelant journalise,
/// et la surveillance de tour continue — une notification qui échoue ne doit pas la faire taire.
pub fn show(
    title: &str,
    body: &str,
    on_click: impl Fn() + Send + Sync + 'static,
) -> windows::core::Result<Toast> {
    unsafe {
        // Le thread winit a déjà son apartment COM ; WinRT s'en accommode, et un
        // `RPC_E_CHANGED_MODE` ici n'empêche pas les appels qui suivent.
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let xml = format!(
        "<toast activationType=\"foreground\" scenario=\"reminder\">\
         <visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual>\
         <audio silent=\"true\"/></toast>",
        escape(title),
        escape(body)
    );
    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml))?;
    let toast = ToastNotification::CreateToastNotification(&doc)?;
    toast.Activated(&TypedEventHandler::new(move |_, _| {
        on_click();
        Ok(())
    }))?;
    let notifier =
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_USER_MODEL_ID))?;
    notifier.Show(&toast)?;
    Ok(Toast { _inner: toast })
}

/// Amène `hwnd` au premier plan — restaurée si minimisée.
///
/// Un process qui n'a pas le premier plan n'a en principe pas le droit de le donner : Windows
/// refuse `SetForegroundWindow` et fait **clignoter** la fenêtre dans la barre des tâches à la
/// place (vu en jeu le 2026-09-14 au clic du toast). Trois leviers, du plus propre au plus
/// brutal, jusqu'à ce que l'un passe :
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
