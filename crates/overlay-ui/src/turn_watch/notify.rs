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
//! la notification dit « c'est à toi », le clic y va. Deux obstacles, levés l'un après l'autre le
//! 2026-09-14, vidéo et journal à l'appui :
//!
//! 1. Depuis le gestionnaire `Activated` du toast, dans le process de l'overlay, tout est refusé
//!    et la fenêtre **clignote** dans la barre des tâches. Voie prévue par Windows pour une
//!    application non empaquetée : l'**activation par protocole** — le toast porte
//!    `launch="wakfu-companion:focus?hwnd=…"`, le clic lance le gestionnaire de ce protocole
//!    (l'overlay lui-même, enregistré par [`register_identity`]) dans un **nouveau process** qui
//!    ne fait que ça ([`focus_window`]) et se termine (voir `main.rs`, tout début de `main`).
//! 2. Ce process non plus n'a pas le droit : au clic, le premier plan est à la fenêtre du toast,
//!    une *Modern App* du shell, et la restriction de `SetForegroundWindow` est alors absolue.
//!    Ce qui la lève : **recevoir une entrée soi-même**. Une fenêtre-leurre de trois pixels sous
//!    le curseur, un clic synthétique dessus, et le process est « celui qui a reçu la dernière
//!    entrée » — le jeu ne reçoit rien, le curseur ne bouge pas ([`focus_via_decoy`]).
//! 3. Ce process est lancé **par le shell**, comme un programme qu'on double-clique : un
//!    exécutable de sous-système *console* reçoit alors de Windows une fenêtre de terminal, qui
//!    surgit par-dessus le jeu, apparaît dans la barre des tâches et lui dispute le premier plan
//!    le temps de sa course. Les deux gestionnaires possibles (ci-dessous) sont donc fenêtrés sans
//!    fenêtre — `overlay-focus` depuis le 2026-09-14, l'overlay lui-même depuis le 2026-09-17
//!    (`windows_subsystem = "windows"` en tête de `main.rs`) : rien à l'écran dans les deux cas.
//!    **Attention avant de changer le sous-système de l'un ou de l'autre** : c'est le seul
//!    garde-fou : ce gestionnaire ne doit jamais rien afficher.
//!
//! ### Quel gestionnaire est enregistré, et lequel arrive chez l'utilisateur
//!
//! [`register_protocol`] préfère `overlay-focus.exe` (minuscule, démarre plus vite) **quand il est
//! à côté de l'overlay**, et enregistre l'overlay lui-même sinon. Ce « sinon » est le cas
//! NOMINAL hors développement : la Release ne publie qu'un binaire par plateforme
//! (`.github/workflows/release.yml`, et `overlay_sync::update` en remplace exactement un), donc
//! `overlay-focus.exe` n'existe que dans un `target/` de compilation. C'est ce qui a fait durer le
//! défaut de la console : corrigé le 2026-09-14 en déportant le focus dans un binaire sans
//! console, il est resté entier chez l'utilisateur, où le gestionnaire était — et reste —
//! l'overlay. Livrer deux fichiers supposerait de les mettre à jour tous les deux ; c'est le
//! sous-système de l'overlay qui a été corrigé à la place.
//!
//! ## Le son
//!
//! Un toast d'application non empaquetée ne peut jouer que les sons système (`ms-winsoundevent:`)
//! — jugés insipides. Le toast est donc **silencieux**, et c'est l'overlay qui joue son propre son
//! (`alert_sound::play_turn_alert`), comme pour les alertes de ramassage.

use windows::core::{HSTRING, PCWSTR};
use windows::Data::Xml::Dom::XmlDocument;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, SetActiveWindow, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEINPUT,
};
use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetAncestor, GetCursorPos, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
    IsIconic, PeekMessageW, RegisterClassW, SetCursorPos, SetForegroundWindow, SetWindowPos,
    ShowWindow, TranslateMessage, WindowFromPoint, GA_ROOT, HWND_TOPMOST, MSG, PM_REMOVE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_RESTORE, WNDCLASSW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP, WS_VISIBLE,
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

/// **Retire du registre tout ce que [`register_identity`] y a écrit** — les deux arbres `HKCU`,
/// celui de l'identité de notification (`Software\\Classes\\AppUserModelId\\…`, nom d'affichage et
/// chemin de l'icône) et celui du protocole d'activation (`Software\\Classes\\wakfu-companion`, la
/// ligne de commande de l'exécutable — donc souvent le nom d'utilisateur OS dans son chemin).
///
/// Appelé par `crate::local_data` sur [`crate::local_data::Scope::Everything`] : l'effacement
/// complet promet l'état d'une installation neuve, et l'icône déposée à côté des gabarits part
/// avec le dossier de données. Best-effort comme l'enregistrement : une clé absente (identité
/// jamais posée parce que les notifications de tour n'ont jamais servi) est le résultat attendu,
/// un refus est journalisé et rien de plus.
pub fn unregister_identity() {
    unsafe {
        for subkey in [
            format!("Software\\Classes\\AppUserModelId\\{APP_USER_MODEL_ID}"),
            format!("Software\\Classes\\{PROTOCOL}"),
        ] {
            let subkey_w = wide(&subkey);
            let status = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(subkey_w.as_ptr()));
            if status.is_err() {
                // `ERROR_FILE_NOT_FOUND` inclus : rien à retirer, c'est le cas le plus courant.
                tracing::debug!("[données locales] HKCU\\{subkey} non retiré : {status:?}");
            } else {
                tracing::info!("[données locales] clé de registre retirée : HKCU\\{subkey}");
            }
        }
    }
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
    // Le binaire dédié `overlay-focus.exe` démarre plus vite : c'est lui qu'on enregistre quand il
    // est à côté de l'overlay (poste de développement) ; l'overlay lui-même sinon — le cas de
    // toute Release, qui ne publie qu'un binaire (voir la doc de module, « Quel gestionnaire est
    // enregistré »). Ni l'un ni l'autre n'ouvre de fenêtre.
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
        let focus_exe = exe.with_file_name("overlay-focus.exe");
        let handler = if focus_exe.is_file() { focus_exe } else { exe };
        let root = format!("Software\\Classes\\{PROTOCOL}");
        set(&root, None, "URL:Wakfu Companion Overlay");
        set(&root, Some("URL Protocol"), "");
        set(
            &format!("{root}\\shell\\open\\command"),
            None,
            &format!("\"{}\" \"%1\"", handler.display()),
        );
        tracing::info!(
            "[tour] protocole {PROTOCOL}: → {}",
            overlay_ingest::privacy::redact_path(&handler)
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
        "<toast activationType=\"protocol\" launch=\"{}\">\
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
/// clic du toast (activation de protocole, doc de module).
///
/// Ce process n'a pas le droit de donner le premier plan : au clic, celui-ci appartient à la
/// fenêtre du toast, une *Modern App* du shell (`ShellExperienceHost`), et la restriction de
/// `SetForegroundWindow` est alors absolue — mesuré le 2026-09-14, tout refusé pendant plus de
/// 5 s : appel direct, frappe Alt synthétique, rattachement à la file d'entrée (impossible vers
/// le shell), `SwitchToThisWindow`, minimiser-restaurer. Ce qui passe, et à coup sûr : **recevoir
/// une entrée soi-même** — voir [`focus_via_decoy`]. L'appel direct reste tenté d'abord, il
/// suffit quand le process est lancé depuis une fenêtre ordinaire (test à la main).
pub fn focus_window(hwnd: isize) {
    let hwnd = HWND(hwnd as *mut _);
    // Ce process n'a pas de journal (voir `main`) : une ligne horodatée dans `focus.log`, à côté
    // des gabarits, dit s'il a été lancé et ce que Windows a répondu.
    let trace = |msg: &str| {
        if let Some(dir) = super::templates::data_dir() {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dir.join(super::FOCUS_LOG))
            {
                let _ = writeln!(
                    f,
                    "{} hwnd={} {msg}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    hwnd.0 as isize
                );
            }
        }
    };
    unsafe {
        trace(&format!(
            "lancé — premier plan : {}",
            GetForegroundWindow().0 as isize
        ));
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        // 1. Appel direct — passe quand ce process a été lancé depuis une fenêtre ordinaire.
        if SetForegroundWindow(hwnd).as_bool() && GetForegroundWindow().0 == hwnd.0 {
            trace("direct : ok");
            return;
        }
        // 2. Le leurre — la voie qui marche depuis un toast. Le shell peut reprendre le premier
        //    plan juste après (il referme le toast et le rend à la fenêtre d'avant) : on
        //    surveille quelques centaines de millisecondes et on réapplique si besoin.
        if focus_via_decoy(hwnd, &trace) {
            trace("leurre : ok");
            for wait_ms in [300u64, 500, 800] {
                std::thread::sleep(std::time::Duration::from_millis(wait_ms));
                let fg = GetForegroundWindow();
                if fg.0 == hwnd.0 {
                    continue;
                }
                trace(&format!(
                    "premier plan repris par {} — réapplication",
                    fg.0 as isize
                ));
                if !focus_via_decoy(hwnd, &trace) {
                    trace("réapplication : refusée");
                }
            }
            trace(&format!(
                "final : premier plan = {} ({})",
                GetForegroundWindow().0 as isize,
                if GetForegroundWindow().0 == hwnd.0 {
                    "la cible"
                } else {
                    "PAS la cible"
                }
            ));
            return;
        }
        // 3. Filet : rattachement à la file d'entrée du premier plan et de la cible.
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, None);
        let target_thread = GetWindowThreadProcessId(hwnd, None);
        let me = GetCurrentThreadId();
        let att_fg =
            fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, true).as_bool();
        let att_target = target_thread != 0
            && target_thread != me
            && target_thread != fg_thread
            && AttachThreadInput(me, target_thread, true).as_bool();
        let _ = BringWindowToTop(hwnd);
        let _ = SetActiveWindow(hwnd);
        let _ = SetForegroundWindow(hwnd);
        if att_target {
            let _ = AttachThreadInput(me, target_thread, false);
        }
        if att_fg {
            let _ = AttachThreadInput(me, fg_thread, false);
        }
        if GetForegroundWindow().0 == hwnd.0 {
            trace("rattachement : ok");
        } else {
            trace("rattachement : refusé — abandon");
        }
    }
}

/// Le leurre — voir [`focus_window`].
///
/// Pas sous le curseur : au clic, il est encore sur le toast, et les notifications vivent dans une
/// bande de z-order au-dessus de tout `HWND_TOPMOST` — le clic synthétique irait au toast (vu :
/// « premier plan après clic = le toast »). Le leurre est posé **au centre de la fenêtre cible**,
/// le curseur y est déplacé le temps du clic puis remis, et `WindowFromPoint` vérifie avant de
/// cliquer que c'est bien lui qui est là : **jamais un clic dans le jeu**.
unsafe fn focus_via_decoy(target: HWND, trace: &dyn Fn(&str)) -> bool {
    unsafe extern "system" fn wndproc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        unsafe { DefWindowProcW(h, m, w, l) }
    }
    let class = wide("WakfuCompanionOverlayFocusDecoy");
    let hinst = GetModuleHandleW(PCWSTR::null()).unwrap_or_default();
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: hinst.into(),
        lpszClassName: PCWSTR(class.as_ptr()),
        ..Default::default()
    };
    // Déjà enregistrée = pas une erreur.
    let _ = RegisterClassW(&wc);
    let mut rect = RECT::default();
    if GetWindowRect(target, &mut rect).is_err() {
        trace("leurre : rectangle de la cible introuvable");
        return false;
    }
    let spot = POINT {
        x: (rect.left + rect.right) / 2,
        y: (rect.top + rect.bottom) / 2,
    };
    let Ok(decoy) = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        PCWSTR(class.as_ptr()),
        PCWSTR::null(),
        WS_POPUP | WS_VISIBLE,
        spot.x - 2,
        spot.y - 2,
        5,
        5,
        None,
        None,
        Some(hinst.into()),
        None,
    ) else {
        trace("leurre : création impossible");
        return false;
    };
    let _ = SetWindowPos(
        decoy,
        Some(HWND_TOPMOST),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
    );
    // Le temps que la fenêtre existe pour le système de hit-test — dès qu'elle y est, on y va.
    let mut msg = MSG::default();
    let mut under = HWND::default();
    for _ in 0..10 {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        under = GetAncestor(WindowFromPoint(spot), GA_ROOT);
        if under.0 == decoy.0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    if under.0 != decoy.0 {
        trace(&format!(
            "leurre : ce n'est pas lui sous le point ({}), pas de clic",
            under.0 as isize
        ));
        let _ = DestroyWindow(decoy);
        return false;
    }
    let mut saved = POINT::default();
    let _ = GetCursorPos(&mut saved);
    let _ = SetCursorPos(spot.x, spot.y);
    let click = |flags| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    let seq = [click(MOUSEEVENTF_LEFTDOWN), click(MOUSEEVENTF_LEFTUP)];
    SendInput(&seq, std::mem::size_of::<INPUT>() as i32);
    // Laisser le clic arriver dans notre file et être traité : c'est ce traitement qui fait de
    // nous le dernier process à avoir reçu une entrée. On s'arrête dès que le leurre a le premier
    // plan — quelques millisecondes en pratique, 200 au plus.
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
    while std::time::Instant::now() < deadline {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if GetForegroundWindow().0 == decoy.0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let _ = SetCursorPos(saved.x, saved.y);
    trace(&format!(
        "leurre : premier plan après clic = {} (leurre = {})",
        GetForegroundWindow().0 as isize,
        decoy.0 as isize
    ));
    let _ = SetForegroundWindow(target);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
    let mut ok = false;
    while !ok && std::time::Instant::now() < deadline {
        ok = GetForegroundWindow().0 == target.0;
        if !ok {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    let _ = DestroyWindow(decoy);
    ok
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
