//! Rédaction des données personnelles avant journalisation — constat **C6** de
//! `docs/analyse-rgpd.md`.
//!
//! Le journal de session (`logs/overlay-ui.<date>.log`, voir `overlay_ui::logging`) est un fichier
//! LOCAL, jamais téléversé — mais il est conservé 14 jours, lisible par tout ce qui tourne sur le
//! poste, et joint tel quel à un rapport de bug. Le chemin de `wakfu.log` y apparaissait en entier :
//! sous Windows comme sous Proton/Wine, il contient presque toujours le **nom d'utilisateur du
//! système** (`C:\Users\prenom.nom\…`, `/home/prenom/…`), c'est-à-dire une donnée personnelle qui
//! n'apporte rien au diagnostic.
//!
//! [`redact_path`] retire exactement cela, et rien d'autre : la forme du chemin — Steam/Proton,
//! Wine, installation native, dossier déplacé à la main — reste entièrement lisible, c'est elle
//! qu'on lit quand un `wakfu.log` n'est pas trouvé.
//!
//! Le chemin COMPLET reste disponible pour le diagnostic au niveau `debug` (réglage « Journal
//! détaillé » de la fenêtre Options), là où l'utilisateur l'a demandé explicitement.

use std::ffi::OsStr;
use std::path::{Component, Path, MAIN_SEPARATOR};

/// Ce qui remplace un segment de chemin portant un nom d'utilisateur.
const USER_PLACEHOLDER: &str = "<utilisateur>";

/// Segments dont le SUIVANT est un nom d'utilisateur, quelle que soit la casse : `C:\Users\moi`,
/// `/home/moi`, et la réplique Wine `~/.wine/drive_c/users/moi`.
const USER_PARENTS: [&str; 2] = ["users", "home"];

/// Rend un chemin journalisable : dossier personnel réduit à `~`, tout segment portant un nom
/// d'utilisateur remplacé par `<utilisateur>`.
///
/// ```text
/// /home/prenom/.steam/steam/steamapps/compatdata/1111/pfx/drive_c/users/steamuser/AppData/Roaming/zaap/wakfu/logs/wakfu.log
/// ~/.steam/steam/steamapps/compatdata/1111/pfx/drive_c/users/<utilisateur>/AppData/Roaming/zaap/wakfu/logs/wakfu.log
/// ```
///
/// À utiliser dans tout `info!`/`warn!`/`error!` qui cite un chemin ; `debug!` et `trace!` gardent
/// le chemin réel (voir la doc du module).
pub fn redact_path(path: &Path) -> String {
    let separator = separator_of(path);
    let (prefix, rest) = match home_dir().and_then(|home| {
        path.strip_prefix(&home)
            .ok()
            .map(|rest| (String::from("~"), rest.to_path_buf()))
    }) {
        Some((prefix, rest)) => (prefix, rest),
        None => (String::new(), path.to_path_buf()),
    };

    let os_user = os_user_name();
    let mut out = prefix;
    let mut previous_was_user_parent = false;
    for component in rest.components() {
        let raw = component.as_os_str().to_string_lossy();
        let lower = raw.to_lowercase();
        let named_user = matches!(component, Component::Normal(_))
            && (previous_was_user_parent
                || os_user
                    .as_deref()
                    .is_some_and(|user| lower == user.to_lowercase()));
        // `previous_was_user_parent` ne vaut que pour le segment IMMÉDIATEMENT suivant :
        // `users/moi/Documents/users` ne doit pas masquer `Documents`.
        previous_was_user_parent =
            matches!(component, Component::Normal(_)) && USER_PARENTS.contains(&lower.as_str());

        match component {
            // Le séparateur d'une racine fait partie du segment rendu : ne pas en ajouter un
            // second derrière lui, sinon `/home` deviendrait `//home`. Il est réécrit avec le
            // séparateur d'ENTRÉE (voir [`separator_of`]) : `Component::RootDir` rend `\` sous
            // Windows, même pour un chemin écrit avec des `/`.
            Component::RootDir => out.push(separator),
            Component::Prefix(_) => out.push_str(&raw),
            _ => {
                if !out.is_empty() && !out.ends_with(separator) {
                    out.push(separator);
                }
                out.push_str(if named_user { USER_PLACEHOLDER } else { &raw });
            }
        }
    }

    if out.is_empty() {
        // Chemin vide en entrée : rendre la même chose que `Path::display`, pas une chaîne muette.
        path.to_string_lossy().into_owned()
    } else {
        out
    }
}

/// Même service pour un chemin déjà rendu en texte (message d'erreur du système, ligne de
/// configuration) — le nom d'utilisateur y est masqué de la même façon.
pub fn redact_path_str(path: &str) -> String {
    redact_path(Path::new(path))
}

/// **Le séparateur à réutiliser en sortie** : celui que porte le chemin d'entrée, et seulement à
/// défaut celui de la plateforme.
///
/// `Path::components` normalise ce qu'il rend : sous Windows, `/opt/jeux/wakfu` ressort en
/// `\opt\jeux\wakfu`. Un chemin lu dans une configuration, un message d'erreur ou une fixture
/// changeait donc de forme au passage — or c'est justement la forme du chemin qu'on lit au journal
/// (Steam/Proton, Wine, installation native) et que ce module promet de conserver. Les quatre tests
/// de ce fichier l'ont attrapé sur le runner Windows du CI (2026-09-18).
fn separator_of(path: &Path) -> char {
    path.to_string_lossy()
        .chars()
        .find(|c| *c == '/' || *c == '\\')
        .unwrap_or(MAIN_SEPARATOR)
}

/// Dossier personnel de l'utilisateur courant. `std::env::home_dir` est de nouveau recommandée
/// depuis Rust 1.87 (son ancien comportement Windows, seul motif de la dépréciation, a été
/// corrigé) — pas de dépendance supplémentaire pour ce crate d'ingestion.
fn home_dir() -> Option<std::path::PathBuf> {
    #[allow(deprecated)]
    std::env::home_dir().filter(|home| !home.as_os_str().is_empty())
}

/// Nom de compte du système, quand il est lisible — il apparaît souvent AILLEURS que sous le
/// dossier personnel (préfixe Wine, disque secondaire, dossier de jeu déplacé).
fn os_user_name() -> Option<String> {
    ["USER", "USERNAME", "LOGNAME"]
        .iter()
        .filter_map(std::env::var_os)
        .map(|value| value.to_string_lossy().into_owned())
        // Un nom d'un seul caractère masquerait des segments légitimes (`c`, `d`…) : hors sujet.
        .find(|value| value.chars().count() > 1)
}

/// Le nom de fichier seul, quand même le dossier n'a pas à figurer au journal.
pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(OsStr::to_string_lossy)
        .map(|name| name.into_owned())
        .unwrap_or_else(|| redact_path(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le nom d'utilisateur d'un préfixe Wine est masqué même quand il n'est pas celui du système
    /// (`steamuser` est le nom par défaut d'un préfixe Proton).
    #[test]
    fn prefixe_wine_le_nom_est_masque() {
        let rendu = redact_path(Path::new(
            "/tmp/pfx/drive_c/users/steamuser/AppData/Roaming/zaap/wakfu/logs/wakfu.log",
        ));
        assert_eq!(
            rendu,
            "/tmp/pfx/drive_c/users/<utilisateur>/AppData/Roaming/zaap/wakfu/logs/wakfu.log"
        );
    }

    /// Un `users` plus loin dans le chemin ne masque que son propre suivant.
    #[test]
    fn seul_le_segment_suivant_est_masque() {
        let rendu = redact_path(Path::new("/srv/users/moi/users/partage/wakfu.log"));
        assert_eq!(
            rendu,
            "/srv/users/<utilisateur>/users/<utilisateur>/wakfu.log"
        );
    }

    /// Le dossier personnel devient `~`, et ce qui suit reste lisible tel quel.
    #[test]
    fn dossier_personnel_reduit_au_tilde() {
        let home = home_dir().expect("dossier personnel résolu dans l'environnement de test");
        let rendu = redact_path(
            &home
                .join(".steam")
                .join("steam")
                .join("steamapps")
                .join("compatdata"),
        );
        // Ce chemin-ci est construit par `Path::join`, donc avec le séparateur de la PLATEFORME :
        // l'attendu l'est aussi. Les autres tests de ce module partent de chemins POSIX écrits en
        // dur, et vérifient qu'ils ressortent tels quels — voir `separator_of`.
        let s = MAIN_SEPARATOR;
        assert_eq!(
            rendu,
            format!("~{s}.steam{s}steam{s}steamapps{s}compatdata")
        );
    }

    /// Un chemin sans rien de personnel traverse la fonction sans y perdre un segment — c'est ce
    /// qui garde le diagnostic lisible.
    #[test]
    fn chemin_sans_donnee_personnelle_inchange() {
        let rendu = redact_path(Path::new("/opt/jeux/wakfu/logs/wakfu.log"));
        assert_eq!(rendu, "/opt/jeux/wakfu/logs/wakfu.log");
    }

    #[test]
    fn nom_de_fichier_seul() {
        assert_eq!(
            file_name(Path::new("/home/moi/wakfu/wakfu.log")),
            "wakfu.log"
        );
    }
}
