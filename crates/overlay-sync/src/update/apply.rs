//! Mise en place et installation : décompresser l'asset téléchargé, vérifier l'empreinte du
//! binaire obtenu, remplacer l'exe en cours d'exécution, relancer.
//!
//! **Deux temps, volontairement séparés.** [`stage`] (thread de fond) produit l'exe de la
//! nouvelle version sur le disque et le vérifie ; [`install_and_relaunch`] (thread principal,
//! juste avant `event_loop.exit()`) fait le remplacement et lance le nouveau process. Entre les
//! deux, l'hôte a pu fermer ses fenêtres et écrire son journal — et un échec de la première étape
//! ne touche jamais à l'exe en place.
//!
//! `self-replace` fait le remplacement d'un exe **en cours d'exécution** : sous Windows, l'exe
//! courant est RENOMMÉ (pas écrasé — un exe qui tourne ne peut pas l'être) puis le nouveau copié à
//! sa place, et l'ancien effacé au prochain démarrage ; sous Linux, un `rename` atomique. Aucun
//! droit particulier tant que le dossier de l'exe est inscriptible (§12 du plan : pas
//! d'installation sous `Program Files`).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use super::download::hex;
use super::UpdateError;

/// Argument passé au process relancé après une installation — l'hôte l'écrit au journal et
/// nettoie le dossier de mise à jour. Jamais un chemin de log : `parse_args` le retire avant.
pub const UPDATED_FROM_FLAG: &str = "--updated-from";

/// Décompresse `asset_gz` dans `staging_dir` sous `staged-{version}[.exe]`, vérifie que le binaire
/// obtenu a la taille et l'empreinte annoncées par le manifeste signé. Renvoie le chemin.
pub fn stage(
    asset_gz: &Path,
    version: &str,
    installed_size: u64,
    installed_sha256: &str,
    staging_dir: &Path,
) -> Result<PathBuf, UpdateError> {
    fs::create_dir_all(staging_dir)?;
    let staged = staging_dir.join(format!("staged-{version}{}", exe_suffix()));
    let compressed = fs::read(asset_gz)?;
    let mut decoder = flate2::read::GzDecoder::new(&compressed[..]);
    let mut binary = Vec::with_capacity(installed_size as usize);
    decoder
        .read_to_end(&mut binary)
        .map_err(|e| UpdateError::Io(format!("décompression : {e}")))?;
    if binary.len() as u64 != installed_size
        || !hex(&Sha256::digest(&binary)).eq_ignore_ascii_case(installed_sha256)
    {
        return Err(UpdateError::HashMismatch {
            what: format!("binaire {version} décompressé"),
        });
    }
    fs::write(&staged, &binary)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o755))?;
    }
    Ok(staged)
}

/// Remplace l'exe courant par `staged`, relance l'overlay avec `--updated-from <from_version>` et
/// renvoie — c'est à l'appelant de sortir de sa boucle d'événements juste après. L'ancien exe
/// n'est pas conservé par ce module : `self-replace` le déplace dans un fichier temporaire qu'il
/// efface lui-même ; le retour arrière passe par la Release précédente (§12 du plan).
pub fn install_and_relaunch(staged: &Path, from_version: &str) -> Result<(), UpdateError> {
    let current = std::env::current_exe()?;
    self_replace::self_replace(staged)
        .map_err(|e| UpdateError::Io(format!("remplacement de {} : {e}", current.display())))?;
    let _ = fs::remove_file(staged);
    Command::new(&current)
        .arg(UPDATED_FROM_FLAG)
        .arg(from_version)
        .spawn()
        .map_err(|e| UpdateError::Io(format!("relance de {} : {e}", current.display())))?;
    Ok(())
}

/// Le dossier de l'exe est-il inscriptible ? — sondé au démarrage pour annoncer un échec avant le
/// téléchargement plutôt qu'après (§12 du plan). `Ok(())` si oui, sinon le message à afficher.
pub fn check_writable_install_dir() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe
        .parent()
        .ok_or_else(|| "dossier de l'exe introuvable".to_string())?;
    let probe = dir.join(format!(".wakfu-overlay-probe-{}", std::process::id()));
    match fs::write(&probe, b"") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
            Ok(())
        }
        Err(err) => Err(format!(
            "le dossier {} n'est pas inscriptible ({err}) — déplacez l'overlay dans un dossier de \
             votre profil utilisateur pour qu'il puisse se mettre à jour",
            dir.display()
        )),
    }
}

/// Efface les restes d'une mise à jour terminée (assets, `.part`, exe mis en place). Appelé au
/// démarrage du process relancé ; silencieux si le dossier n'existe pas.
pub fn cleanup(updates_dir: &Path) {
    let _ = fs::remove_dir_all(updates_dir);
}

/// Sépare les arguments de la ligne de commande : le chemin de `wakfu.log` (premier argument
/// libre, s'il y en a un) et la version d'où l'on vient (`--updated-from X`). Les deux hôtes
/// passaient jusqu'ici `args().nth(1)` tel quel au moteur — un drapeau y aurait été pris pour un
/// chemin.
pub fn parse_args(args: impl IntoIterator<Item = String>) -> ParsedArgs {
    let mut parsed = ParsedArgs::default();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        if arg == UPDATED_FROM_FLAG {
            parsed.updated_from = it.next();
        } else if parsed.log_path.is_none() {
            parsed.log_path = Some(PathBuf::from(arg));
        }
    }
    parsed
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ParsedArgs {
    pub log_path: Option<PathBuf>,
    pub updated_from: Option<String>,
}

fn exe_suffix() -> &'static str {
    if cfg!(windows) {
        ".exe"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn gz(bytes: &[u8]) -> Vec<u8> {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        enc.write_all(bytes).unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn mise_en_place_verifie_taille_et_empreinte() {
        let dir = std::env::temp_dir().join(format!("overlay-update-stage-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let binary = b"#!/bin/sh\necho bonjour\n".to_vec();
        let asset = dir.join("asset.gz");
        fs::write(&asset, gz(&binary)).unwrap();
        let sha = hex(&Sha256::digest(&binary));

        let staged = stage(&asset, "0.21.0", binary.len() as u64, &sha, &dir).unwrap();
        assert_eq!(fs::read(&staged).unwrap(), binary);
        assert!(staged
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("staged-0.21.0"));

        let err = stage(&asset, "0.21.0", binary.len() as u64 + 1, &sha, &dir).unwrap_err();
        assert!(matches!(err, UpdateError::HashMismatch { .. }));
        let err = stage(&asset, "0.21.0", binary.len() as u64, "00", &dir).unwrap_err();
        assert!(matches!(err, UpdateError::HashMismatch { .. }));
        cleanup(&dir);
        assert!(!dir.exists());
    }

    #[test]
    fn arguments_chemin_et_drapeau() {
        let p = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        assert_eq!(p(&[]), ParsedArgs::default());
        assert_eq!(
            p(&["C:\\logs\\wakfu.log"]),
            ParsedArgs {
                log_path: Some(PathBuf::from("C:\\logs\\wakfu.log")),
                updated_from: None
            }
        );
        assert_eq!(
            p(&["--updated-from", "0.20.4"]),
            ParsedArgs {
                log_path: None,
                updated_from: Some("0.20.4".into())
            }
        );
        assert_eq!(
            p(&["--updated-from", "0.20.4", "/tmp/wakfu.log"]),
            ParsedArgs {
                log_path: Some(PathBuf::from("/tmp/wakfu.log")),
                updated_from: Some("0.20.4".into())
            }
        );
    }

    #[test]
    fn dossier_de_l_exe_inscriptible_ici() {
        // Le binaire de test vit dans `target/`, inscriptible : la sonde doit passer.
        assert_eq!(check_writable_install_dir(), Ok(()));
    }
}
