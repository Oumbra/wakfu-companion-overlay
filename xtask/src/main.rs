//! Outillage de CI — voir `docs/plan-architecture.md` §8 (budget mémoire) et §11 (CI GitHub
//! Actions). `[workspace]` vide dans `Cargo.toml` (voir sa doc) : ce n'est PAS un membre du
//! workspace principal, exactement comme `spikes/` — les dépendances propres à l'outillage
//! (`sysinfo`) ne pèsent jamais sur la résolution de dépendances de production.
//!
//! Sous-commande unique pour l'instant : `mem-budget` (voir sa doc). D'autres pourront s'y ajouter
//! (`visual-check`, différé — voir §17.2 « État », le rendu Linux qu'il faudrait piloter n'existe
//! pas encore).

mod mem_budget;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("mem-budget") => mem_budget::run(),
        Some(other) => {
            eprintln!("sous-commande inconnue : {other}\nusage : cargo run -p xtask -- mem-budget");
            std::process::exit(2);
        }
        None => {
            eprintln!("usage : cargo run -p xtask -- mem-budget");
            std::process::exit(2);
        }
    }
}
