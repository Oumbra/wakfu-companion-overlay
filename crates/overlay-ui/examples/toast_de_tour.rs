//! Affiche le toast de tour tel que l'overlay l'enverra — pour voir l'émetteur, l'icône, le son
//! et le clic sans lancer un combat : `cargo run -p overlay-ui --example toast_de_tour`.
//! Le clic ramène au premier plan la première fenêtre Wakfu trouvée, s'il y en a une.
fn main() {
    #[cfg(windows)]
    {
        use overlay_ui::turn_watch::{notify, templates};
        if let Some(dir) = templates::data_dir() {
            notify::register_identity(&dir, overlay_ui::ui_icons::app_logo_png());
        }
        let target = overlay_ui::game_window::GameWindowTracker::new()
            .scan()
            .first()
            .map(|(_, info)| info.hwnd.0 as isize);
        match notify::show(
            "Pugio Letalis doit jouer",
            "C'est à toi — clique pour passer sur sa fenêtre",
            move || match target {
                Some(hwnd) => notify::focus_window(hwnd),
                None => println!("clic reçu — aucune fenêtre Wakfu à amener au premier plan"),
            },
        ) {
            Ok(_toast) => {
                overlay_ui::alert_sound::play_turn_alert();
                println!("toast envoyé — 20 s pour le cliquer");
                std::thread::sleep(std::time::Duration::from_secs(20));
            }
            Err(e) => println!("échec : {e}"),
        }
    }
    #[cfg(not(windows))]
    println!("Windows seulement");
}
