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
//!
//! **Et, pour une cible Windows, l'ICÔNE et le bloc de version de l'exécutable**
//! (`embed_windows_resources`) : `wakfu-companion-overlay.exe` porte le logo du projet dans
//! l'explorateur, la barre des tâches et Alt-Tab, comme il le porte déjà en zone de notification
//! (`ui_icons::app_logo_rgba`) et en icône de fenêtre. Sans ressource liée, Windows affiche l'icône
//! générique d'application, que rien dans le code ne peut corriger après coup — c'est une donnée du
//! fichier exe, pas un appel d'API.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Valeur repli, affichée telle quelle — voir la doc du module.
const UNKNOWN_COMMIT: &str = "inconnu";

/// Logo du projet, source UNIQUE de l'icône : le même fichier qu'`ui_icons::LOGO_BYTES` embarque
/// pour la zone de notification, la fenêtre de connexion et l'icône de fenêtre. L'`.ico` n'est donc
/// pas commité — il est dérivé de ce PNG à chaque build Windows (voir `write_ico`), ce qui évite
/// qu'une retouche du logo laisse l'exe avec l'ancienne image.
#[cfg(windows)]
const LOGO_PNG: &str = "assets/ui/logo-purple.png";

/// Paliers embarqués dans l'`.ico` — voir `write_ico`.
#[cfg(windows)]
const ICON_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 128];

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

    embed_windows_resources();
}

/// Icône et bloc de version de l'exécutable Windows — voir la doc du module.
///
/// `#[cfg(windows)]` désigne ici la plateforme **hôte** (un build script est compilé et exécuté
/// pour la machine qui compile), ce qui est exactement la condition des `build-dependencies` de
/// `Cargo.toml` : sans cet accord, `winresource`/`image` seraient absentes du graphe et ce fichier
/// ne compilerait pas sous Linux. La CIBLE, elle, est revérifiée ci-dessous — compiler
/// `wakfu-companion-overlay-x11` depuis Windows ne doit pas embarquer de ressource Windows.
#[cfg(not(windows))]
fn embed_windows_resources() {
    // Cross-compilation vers Windows depuis Linux : `cargo check --target x86_64-pc-windows-gnu`
    // est le seul moyen de vérifier ce binaire depuis une session cloud (plan de mise à jour,
    // phase 2), et il reste utile. Mais l'exe qui en sortirait n'aurait PAS d'icône, faute de
    // `winresource` dans le graphe — le dire plutôt que de le laisser découvrir sur le poste de
    // l'utilisateur. Un binaire livré est toujours compilé sous Windows (`release.yml`).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!(
            "cargo:warning=cible Windows depuis un hôte non-Windows : l'exe n'aura ni icône ni \
             bloc de version (voir build.rs). Un binaire livré se compile sous Windows."
        );
    }
}

#[cfg(windows)]
fn embed_windows_resources() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let logo = Path::new(LOGO_PNG);
    println!("cargo:rerun-if-changed={LOGO_PNG}");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR fourni par Cargo"));
    let icon = out_dir.join("wakfu-companion-overlay.ico");
    write_ico(logo, &icon);

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon(icon.to_str().expect("OUT_DIR représentable en UTF-8"));
    // Sans ces trois lignes, le bloc de version porterait le nom du PAQUET (`overlay-ui`) : c'est
    // ce que Windows affiche comme nom d'application dans le gestionnaire des tâches
    // (`FileDescription`) et dans les propriétés du fichier.
    resource.set("ProductName", "Wakfu Companion Overlay");
    resource.set("FileDescription", "Wakfu Companion Overlay");
    resource.set("OriginalFilename", "wakfu-companion-overlay.exe");

    // Volontairement FATAL, contrairement au hash de commit plus haut : un hash manquant dégrade un
    // diagnostic, une icône manquante se voit chez l'utilisateur et ne se rattrape pas au
    // lancement. Le compilateur de ressources (`rc.exe`) fait partie du SDK Windows, qui vient avec
    // la chaîne MSVC sans laquelle rien ne se lie de toute façon.
    resource
        .compile()
        .expect("compilation de la ressource Windows (icône, bloc de version)");
}

/// Encode `src` (PNG) en `.ico` multi-tailles vers `dest`.
///
/// Windows choisit dans le fichier la taille la plus proche de ce qu'il affiche ; ne lui donner que
/// le 128 × 128 source le laisserait réduire lui-même, avec un rendu sale aux petites tailles (16
/// et 32 px : barre des tâches, explorateur en liste). On lui sert donc chaque palier pré-réduit en
/// Lanczos. Pas de 256 : le logo source fait 128 px, l'agrandir n'ajouterait aucun détail — Windows
/// s'en charge dans les rares vues qui vont au-delà.
#[cfg(windows)]
fn write_ico(src: &Path, dest: &Path) {
    use image::codecs::ico::{IcoEncoder, IcoFrame};
    use image::{ExtendedColorType, ImageReader};

    let logo = ImageReader::open(src)
        .unwrap_or_else(|err| panic!("{} illisible : {err}", src.display()))
        .decode()
        .unwrap_or_else(|err| panic!("{} n'est pas une image valide : {err}", src.display()))
        .to_rgba8();

    let frames: Vec<IcoFrame<'_>> = ICON_SIZES
        .iter()
        .map(|&size| {
            let scaled =
                image::imageops::resize(&logo, size, size, image::imageops::FilterType::Lanczos3);
            IcoFrame::as_png(scaled.as_raw(), size, size, ExtendedColorType::Rgba8)
                .expect("taille d'icône dans 1..=256")
        })
        .collect();

    let file = std::fs::File::create(dest)
        .unwrap_or_else(|err| panic!("{} non créable : {err}", dest.display()));
    IcoEncoder::new(std::io::BufWriter::new(file))
        .encode_images(&frames)
        .expect("encodage de l'icône Windows");
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
