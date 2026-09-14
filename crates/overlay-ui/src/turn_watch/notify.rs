//! Notification du système — Windows, toast WinRT (`Windows.UI.Notifications`).
//!
//! Directement par le crate `windows`, déjà présent : aucune dépendance de plus, et le contrôle de
//! l'identité de l'émetteur, qui est le point resté ouvert (§14 point 6 du plan). Un toast n'est
//! affiché que sous un `AppUserModelID` **enregistré** — un raccourci du menu Démarrer le porte —
//! et l'overlay n'a pas d'installeur. En attendant, celui de **PowerShell**, présent sur tout
//! Windows : le toast s'affiche au nom de « Windows PowerShell », ce que l'utilisateur a accepté
//! le 2026-09-14 (« on verra plus tard pour le reste »). Le jour de l'installeur, cette constante
//! devient l'identifiant de l'overlay et rien d'autre ne change.

use windows::core::HSTRING;
use windows::Data::Xml::Dom::XmlDocument;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

/// Voir doc de module — l'identité empruntée tant que l'overlay n'a pas la sienne.
const APP_USER_MODEL_ID: &str =
    r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe";

/// Affiche un toast titre + corps. Une erreur est rendue, jamais masquée : l'appelant journalise,
/// et la surveillance de tour continue — une notification qui échoue ne doit pas la faire taire.
pub fn show(title: &str, body: &str) -> windows::core::Result<()> {
    unsafe {
        // Le thread winit a déjà son apartment COM ; WinRT s'en accommode, et un
        // `RPC_E_CHANGED_MODE` ici n'empêche pas les appels qui suivent.
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let xml = format!(
        "<toast scenario=\"reminder\"><visual><binding template=\"ToastGeneric\">\
         <text>{}</text><text>{}</text></binding></visual>\
         <audio src=\"ms-winsoundevent:Notification.Default\"/></toast>",
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

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
