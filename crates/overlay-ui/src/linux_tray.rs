//! Icône de zone de notification sous Linux/KDE (StatusNotifierItem sur DBus, §17.2 du plan) —
//! équivalent Linux de `main.rs::install_tray`/`sync_tray_menu`/`handle_tray_menu` (Windows,
//! `tray-icon`). Même menu (Options / Mise à jour / Déconnecter / Quitter), même politique
//! (jamais fatal si la pose échoue), mais via `ksni` : implémentation pure Rust du protocole SNI,
//! sans GTK/libappindicator — voir la doc de module de `bin/wakfu-companion-overlay-x11.rs`, qui
//! expliquait pourquoi `tray-icon` avait été écarté côté Linux. KDE Plasma (et tout hôte SNI,
//! Discord en étant un exemple courant) affiche cette icône nativement.
//!
//! Feature `blocking` de `ksni` : `LinuxTray::spawn` fait son installation DBus de façon
//! synchrone puis rend la main, le service tournant sur son propre thread — pas de runtime
//! tokio/async-io à intégrer dans ce binaire, qui n'en a par ailleurs aucun besoin.

use std::sync::mpsc;

use ksni::blocking::TrayMethods;
use ksni::menu::{MenuItem, StandardItem};

use crate::ui_icons;

/// Les quatre actions du menu — sondées côté binaire X11 par `try_recv`, exactement comme
/// `hotkey_events` (voir `about_to_wait`).
pub enum TrayEvent {
    Options,
    ManualUpdate,
    Disconnect,
    Quit,
}

pub struct LinuxTray {
    event_tx: mpsc::Sender<TrayEvent>,
    /// Options/Déconnecter ne sont utiles qu'une fois un compte lié — même politique que
    /// `main.rs::TrayMenu::enabled_for_account`.
    enabled_for_account: bool,
}

impl ksni::Tray for LinuxTray {
    fn id(&self) -> String {
        "wakfu-companion-overlay".into()
    }

    fn title(&self) -> String {
        "Wakfu Companion Overlay".into()
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::ApplicationStatus
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let (mut data, width, height) = ui_icons::app_logo_rgba();
        // RGBA -> ARGB réseau, format attendu par `ksni::Icon` (voir sa doc).
        for pixel in data.as_chunks_mut::<4>().0 {
            pixel.rotate_right(1);
        }
        vec![ksni::Icon {
            width: width as i32,
            height: height as i32,
            data,
        }]
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let options_tx = self.event_tx.clone();
        let update_tx = self.event_tx.clone();
        let disconnect_tx = self.event_tx.clone();
        let quit_tx = self.event_tx.clone();
        vec![
            StandardItem {
                label: "Options".into(),
                enabled: self.enabled_for_account,
                activate: Box::new(move |_| {
                    let _ = options_tx.send(TrayEvent::Options);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Mise à jour".into(),
                activate: Box::new(move |_| {
                    let _ = update_tx.send(TrayEvent::ManualUpdate);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Déconnecter".into(),
                enabled: self.enabled_for_account,
                activate: Box::new(move |_| {
                    let _ = disconnect_tx.send(TrayEvent::Disconnect);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quitter".into(),
                activate: Box::new(move |_| {
                    let _ = quit_tx.send(TrayEvent::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Pose l'icône — jamais fatal (même politique que `main.rs::install_tray`) : sans bus de session
/// DBus ou sans hôte SNI (CI, Game Mode/Gamescope), un simple avertissement et l'overlay continue
/// sans tray.
pub fn install_tray() -> Option<(ksni::blocking::Handle<LinuxTray>, mpsc::Receiver<TrayEvent>)> {
    let (event_tx, event_rx) = mpsc::channel();
    let tray = LinuxTray {
        event_tx,
        enabled_for_account: false,
    };
    match tray.spawn() {
        Ok(handle) => {
            tracing::info!(
                "[zone de notification] icône posée — menu Options / Mise à jour / Déconnecter / Quitter."
            );
            Some((handle, event_rx))
        }
        Err(err) => {
            tracing::warn!("[zone de notification] icône refusée : {err}");
            None
        }
    }
}

/// Grise/dégrise Options et Déconnecter selon l'état du compte — même politique que
/// `main.rs::sync_tray_menu`. L'appelant ne doit invoquer cette fonction QUE sur un changement
/// (voir `App::sync_session_windows`) : contrairement à `tray-icon` (appel Win32 local),
/// `Handle::update` traverse un canal vers le thread DBus du service.
pub fn sync_tray_menu(handle: &ksni::blocking::Handle<LinuxTray>, connected: bool) {
    handle.update(|tray| tray.enabled_for_account = connected);
}
