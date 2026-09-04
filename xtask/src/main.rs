//! Outillage de CI — voir `docs/plan-architecture.md` §8 (budget mémoire), §11 (CI GitHub
//! Actions) et §17.2 (harnais comportemental Xvfb/X11). `[workspace]` vide dans `Cargo.toml` (voir
//! sa doc) : ce n'est PAS un membre du workspace principal, exactement comme `spikes/` — les
//! dépendances propres à l'outillage (`sysinfo`, `x11rb`) ne pèsent jamais sur la résolution de
//! dépendances de production.
//!
//! Sous-commandes : `mem-budget` (voir sa doc) et `visual-check` (voir sa doc — jamais appelée
//! par la CI par défaut, §17.2 : « jamais un gate CI par défaut » pour ce harnais précis).

mod mem_budget;
mod visual_check;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("mem-budget") => mem_budget::run(),
        Some("visual-check") => visual_check::run(),
        Some(other) => {
            eprintln!(
                "sous-commande inconnue : {other}\nusage : cargo run -p xtask -- mem-budget|visual-check"
            );
            std::process::exit(2);
        }
        None => {
            eprintln!("usage : cargo run -p xtask -- mem-budget|visual-check");
            std::process::exit(2);
        }
    }
}
