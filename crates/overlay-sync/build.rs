//! Fige l'**origine de l'API par défaut** dans le binaire, d'après le profil de compilation
//! (`client::DEFAULT_BASE_URL`) :
//!
//! - profil `release` (le seul que `.github/workflows/release.yml` livre) → **prod**,
//!   `https://wakfu-companion.com` — décision du mainteneur du 2026-09-15
//!   (`docs/plan-mise-a-jour.md` §10 point 6) ;
//! - tout autre profil (`preview`, `debug`, les tests) → **déploiement dev**,
//!   `https://claude-dev.wakfu-companion.com`.
//!
//! Jusqu'au 2026-09-17, la valeur compilée était toujours la prod et seuls les scripts
//! `crates/overlay-ui/preview.{ps1,sh}` posaient `WAKFU_COMPANION_API_URL` au lancement : un
//! `target/preview/overlay-ui.exe` lancé autrement (raccourci, protocole `wakfu-companion:`) visait
//! donc la prod avec un jeton émis par dev, et la fenêtre de connexion bouclait sur un 401.
//! La règle est désormais portée par le build lui-même : *un exe de preview parle à dev, un exe de
//! release parle à prod*, quelle que soit la façon dont on le lance. `WAKFU_COMPANION_API_URL`
//! reste la surcharge à l'exécution (voir `client::base_url`), pour un `wrangler pages dev` local.
//! Depuis le 2026-09-21 le jeton est lui aussi rangé par déploiement (`token_store::slot`) : les
//! deux exes ne s'écrasent plus leur session.
//!
//! Cargo n'expose pas le nom d'un profil personnalisé aux build scripts (`PROFILE` vaut `release`
//! pour `preview`, qui en hérite) ; on le lit dans `OUT_DIR`, qui vaut toujours
//! `<target>/<profil>/build/<crate>-<hash>/out`.

use std::path::{Path, PathBuf};

const PROD_URL: &str = "https://wakfu-companion.com";
const DEV_URL: &str = "https://claude-dev.wakfu-companion.com";

fn main() {
    // Chaque profil a son propre `OUT_DIR` : le script n'a besoin de rejouer que s'il change
    // lui-même, pas à chaque fichier touché dans le crate (comportement par défaut sans cette ligne).
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("Cargo fournit OUT_DIR"));
    let profile = profile_dir_name(&out_dir);
    let url = if profile == "release" {
        PROD_URL
    } else {
        DEV_URL
    };
    println!("cargo:rustc-env=WAKFU_COMPANION_API_DEFAULT_URL={url}");
}

/// Nom du dossier de profil dans `OUT_DIR` — `release`, `preview`, `debug`… ; chaîne vide si la
/// forme attendue n'est pas reconnue (auquel cas on retombe sur dev, jamais sur la prod par
/// accident).
fn profile_dir_name(out_dir: &Path) -> String {
    out_dir
        .ancestors()
        .nth(3)
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}
