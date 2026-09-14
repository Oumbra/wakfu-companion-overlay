//! Affiche le toast de tour tel que l'overlay l'enverra — pour voir l'émetteur, le son et le
//! rendu sans lancer un combat : `cargo run -p overlay-ui --example toast_de_tour`.
fn main() {
    #[cfg(windows)]
    match overlay_ui::turn_watch::notify::show("Pugio Letalis doit jouer", "C'est à toi — Wakfu")
    {
        Ok(()) => println!("toast envoyé"),
        Err(e) => println!("échec : {e}"),
    }
    #[cfg(not(windows))]
    println!("Windows seulement");
}
