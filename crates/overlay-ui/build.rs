//! Injecte le **hash du commit courant** dans le binaire, à côté du numéro de version que Cargo y
//! met déjà tout seul (`CARGO_PKG_VERSION`, alimenté par `[workspace.package] version` de la racine
//! — voir `scripts/bump-version.sh` pour son incrémentation automatique).
//!
//! Pourquoi les deux et pas la version seule : entre deux bumps, plusieurs commits partagent le
//! même numéro (un `docs:`/`chore:` n'en déclenche aucun, et un correctif poussé juste après une
//! release porte encore la version d'avant). Un rapport de bug qui ne cite que « 0.4.2 » ne désigne
//! donc pas un état du dépôt ; avec le hash, si.
//!
//! Le hash vaut [`UNKNOWN_COMMIT`] quand il n'est pas déterminable (build depuis une archive sans
//! `.git/`, `git` absent du PATH) — jamais une erreur de compilation : ne pas pouvoir dater un
//! build n'est pas une raison de refuser de le produire.
//!
//! Surcharge : `WAKFU_OVERLAY_COMMIT=<valeur>` force le hash (chaîne de release qui construit
//! depuis un export sans `.git/`, par exemple). Prioritaire sur `git`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Valeur repli, affichée telle quelle — voir la doc du module.
const UNKNOWN_COMMIT: &str = "inconnu";

fn main() {
    // Une variable d'environnement ne fait pas partie des entrées qu'un build script surveille par
    // défaut : sans cette ligne, la changer ne rejouerait pas ce script.
    println!("cargo:rerun-if-env-changed=WAKFU_OVERLAY_COMMIT");

    let commit = std::env::var("WAKFU_OVERLAY_COMMIT")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or_else(git_short_hash)
        .unwrap_or_else(|| UNKNOWN_COMMIT.to_string());

    println!("cargo:rustc-env=WAKFU_OVERLAY_COMMIT={commit}");
}

/// Hash court du `HEAD` courant, et déclaration des fichiers git à surveiller.
///
/// Sans ces `rerun-if-changed`, le hash se figerait à sa valeur du PREMIER build : Cargo ne rejoue
/// un build script que si une de ses entrées déclarées a changé, et le contenu de `.git/` ne
/// participe à aucune dépendance de compilation. On surveille donc `HEAD` (qui change à chaque
/// bascule de branche) ET le fichier de référence qu'il désigne (qui change à chaque commit sur la
/// branche courante) — le second est le cas courant, `HEAD` étant un pointeur symbolique stable
/// tant qu'on reste sur la même branche.
fn git_short_hash() -> Option<String> {
    let git_dir = capture(&["rev-parse", "--absolute-git-dir"]).map(PathBuf::from)?;

    watch(&git_dir.join("HEAD"));
    // `--symbolic-full-name HEAD` donne `refs/heads/dev` sur une branche, et échoue en HEAD
    // détaché (rebase, bisect) — auquel cas `HEAD` seul suffit, il contient alors le hash brut.
    if let Some(head_ref) = capture(&["symbolic-ref", "--quiet", "HEAD"]) {
        watch(&git_dir.join(&head_ref));
        // Référence EMPAQUETÉE : après un `git gc`/un clone frais, `refs/heads/<branche>` n'existe
        // pas en tant que fichier, sa valeur vit dans `packed-refs`. Surveiller les deux couvre les
        // deux formes sans avoir à deviner laquelle est en vigueur.
        watch(&git_dir.join("packed-refs"));
    }

    capture(&["rev-parse", "--short", "HEAD"])
}

fn watch(path: &Path) {
    if path.exists() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

/// Sortie standard d'une commande `git`, vidée de ses espaces — `None` si `git` est absent, si le
/// dépôt n'en est pas un, ou si la commande échoue.
fn capture(args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        // Cargo garantit `CARGO_MANIFEST_DIR` aux build scripts et positionne déjà leur
        // répertoire courant dessus ; l'imposer ici rend le script indépendant de cette garantie
        // (et lisible : on voit sur quel dépôt `git` est interrogé).
        .current_dir(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let value = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}
