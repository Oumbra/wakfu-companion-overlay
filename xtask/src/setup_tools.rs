//! `xtask setup-tools` — installe les outils Cargo globaux dont ce dépôt a besoin (aujourd'hui :
//! RTK, voir `.claude/hooks/rtk-hook.sh`), avec une version
//! épinglée par outil — même esprit que `rust-toolchain.toml` : une montée de version est un
//! changement explicite dans son propre commit, jamais une dérive d'une machine à l'autre.
//!
//! **Point d'entrée unique**, sur un poste de dev comme dans un conteneur de session cloud — pour
//! ne jamais dupliquer la logique d'installation à deux endroits qui finiraient par diverger
//! (cette duplication a existé : un hook `SessionStart`, retiré le 2026-09-20, installait RTK par
//! le `curl | sh` officiel ; celle-ci en est le pendant « via Cargo »).
//!
//! Chaque outil est un paquet crates.io ordinaire (`cargo install --locked --version …`) : RTK y
//! est distribué sous le nom `brokk-rtk` (republication du même projet, `rtk-ai/rtk`, sous un nom
//! de crate différent — `rtk` était déjà pris sur crates.io par un paquet sans rapport), mais
//! produit bien un binaire nommé `rtk`. **Sa version crates.io ne suit pas forcément le même
//! numéro que les tags GitHub du projet** (ex. le binaire précompilé `v0.49.0` du poste du
//! mainteneur) : c'est la même famille d'outil, pas un miroir
//! version pour version. Si l'exactitude du numéro de version compte plus que la simplicité
//! d'installation via Cargo, préférer le script officiel (`rtk-ai/rtk/install.sh`).
//!
//! Un outil déjà présent dans le `PATH` (quelle que soit sa provenance : `cargo install`, binaire
//! précompilé, paquet système…) n'est jamais réinstallé — on vérifie sa présence, pas son numéro
//! de version exact, pour rester rapide sur les machines qui l'ont déjà.

use std::process::{Command, Stdio};

/// Un outil Cargo global à garantir présent, avec sa version épinglée.
struct DevTool {
    /// Nom du binaire produit, tel qu'on le cherche dans le `PATH` (ex. `rtk`).
    bin_name: &'static str,
    /// Nom du paquet crates.io à installer (peut différer du nom du binaire, voir la doc RTK).
    crate_name: &'static str,
    /// Version exacte, épinglée — voir la doc de module.
    version: &'static str,
}

const TOOLS: &[DevTool] = &[DevTool {
    bin_name: "rtk",
    crate_name: "brokk-rtk",
    version: "0.42.4",
}];

/// `true` si `bin_name` répond dans le `PATH` courant — peu importe sa provenance ou sa version
/// exacte, voir la doc de module.
fn is_installed(bin_name: &str) -> bool {
    Command::new(bin_name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

pub fn run() {
    let mut failed = Vec::new();

    for tool in TOOLS {
        if is_installed(tool.bin_name) {
            println!(
                "xtask setup-tools : {} déjà présent dans le PATH, rien à faire.",
                tool.bin_name
            );
            continue;
        }

        println!(
            "xtask setup-tools : installation de {} {} (paquet {})…",
            tool.bin_name, tool.version, tool.crate_name
        );
        let status = Command::new("cargo")
            .args([
                "install",
                "--locked",
                "--version",
                tool.version,
                tool.crate_name,
            ])
            .status();

        match status {
            Ok(status) if status.success() => {
                println!("xtask setup-tools : {} installé.", tool.bin_name);
            }
            Ok(status) => {
                eprintln!(
                    "xtask setup-tools : échec de l'installation de {} (code {}).",
                    tool.crate_name, status
                );
                failed.push(tool.crate_name);
            }
            Err(err) => {
                eprintln!(
                    "xtask setup-tools : impossible de lancer `cargo install` pour {} : {err}",
                    tool.crate_name
                );
                failed.push(tool.crate_name);
            }
        }
    }

    if !failed.is_empty() {
        eprintln!(
            "xtask setup-tools : outils non installés : {}.",
            failed.join(", ")
        );
        std::process::exit(1);
    }
}
