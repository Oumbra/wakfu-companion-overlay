//! Affiche le toast de tour tel que l'overlay l'enverra — pour voir l'émetteur, l'icône, le son
//! et le clic sans lancer un combat : `cargo run -p overlay-ui --example toast_de_tour`.
//! Le clic lance le gestionnaire du protocole `wakfu-companion:` — l'overlay, qui l'enregistre à
//! SON démarrage vers son propre exécutable : lancer l'overlay d'abord, sinon rien n'est
//! enregistré (cet exemple ne le fait pas, il pointerait sur lui-même).
fn main() {
    #[cfg(windows)]
    {
        use overlay_ui::turn_watch::notify;
        let target = overlay_ui::game_window::GameWindowTracker::new()
            .scan()
            .first()
            .map(|(_, info)| info.hwnd.0 as isize)
            .unwrap_or(0);
        match notify::show(
            "Pugio Letalis doit jouer",
            "C'est à toi — clique pour passer sur sa fenêtre",
            target,
        ) {
            Ok(()) => {
                overlay_ui::alert_sound::play_turn_alert();
                println!("toast envoyé (cible hwnd={target})");
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
            Err(e) => println!("échec : {e}"),
        }
    }
    #[cfg(not(windows))]
    println!("Windows seulement");
}
