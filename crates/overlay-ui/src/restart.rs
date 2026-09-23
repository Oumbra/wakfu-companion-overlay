//! Redémarrage de l'overlay : relancer le process courant, à l'identique, juste avant de sortir de
//! la boucle d'événements.
//!
//! **Pourquoi un module et non trois lignes dans l'hôte** : il y a DEUX hôtes (`main.rs`, fenêtré
//! winit/Windows, et `bin/wakfu-companion-overlay-x11.rs`, X11), et le bouton « Redémarrer » de la
//! fenêtre Options les sert tous les deux. Une relance recopiée dans chacun aurait dérivé au
//! premier argument de ligne de commande ajouté.
//!
//! **Ce qui est relancé, et avec quoi** : l'exe courant (`std::env::current_exe`) et les arguments
//! de CE lancement, moins `--updated-from <version>` — ce drapeau dit « je viens d'une mise à
//! jour », il fait nettoyer le dossier de mise à jour et écrire une ligne de journal qui serait
//! fausse au redémarrage suivant (voir `overlay_sync::update::apply`). Le chemin de `wakfu.log`
//! passé en argument, lui, est conservé : le process neuf doit suivre le même fichier que celui
//! qu'il remplace, et non retomber sur la découverte automatique.
//!
//! **Le nouveau process est lancé AVANT que l'ancien ne sorte**, comme à l'installation d'une mise
//! à jour (`apply::install_and_relaunch`) : les deux se croisent le temps que la boucle
//! d'événements s'arrête et que les fenêtres tombent. Le seul verrou exclusif de l'overlay, celui
//! d'instance unique (`single_instance`, 2026-09-23), est attendu par le process neuf, qui porte
//! `RELAUNCH_ENV` — c'est la seule façon de relancer sans dépendre d'un script externe ou d'un
//! service.

use std::process::Command;

use overlay_sync::update::apply::UPDATED_FROM_FLAG;

/// Lance un nouveau process de l'overlay avec les arguments de celui-ci, et renvoie. **L'appelant
/// sort de sa boucle d'événements juste après** (`event_loop.exit()`), après avoir journalisé sa
/// fin de session comme pour n'importe quelle autre sortie.
///
/// Rend l'erreur telle quelle plutôt que de paniquer : un redémarrage impossible (exe déplacé
/// entre-temps, droits refusés) ne doit pas TUER l'overlay en place — l'hôte en journalise la
/// cause et reste ouvert, ce qui laisse à l'utilisateur la sortie ordinaire.
pub fn relaunch() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|err| format!("exe courant introuvable : {err}"))?;
    let args = forwarded_args(std::env::args().skip(1));
    Command::new(&exe)
        // Le process neuf attend que celui-ci relâche le verrou d'instance unique
        // (`single_instance`) au lieu de se prendre pour un doublon et de sortir.
        .env(crate::single_instance::RELAUNCH_ENV, "1")
        .args(&args)
        .spawn()
        .map_err(|err| format!("relance de {} : {err}", exe.display()))?;
    tracing::info!(
        "[redémarrage] {} relancé{}.",
        exe.display(),
        if args.is_empty() {
            String::new()
        } else {
            format!(" avec {}", args.join(" "))
        }
    );
    Ok(())
}

/// Les arguments à repasser au process neuf : ceux de ce lancement, `--updated-from <version>`
/// retiré (voir la doc de module). Une fonction libre pour que la règle soit testée sans lancer
/// quoi que ce soit.
fn forwarded_args(args: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut gardes = Vec::new();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        if arg == UPDATED_FROM_FLAG {
            // La version qui suit le drapeau part avec lui : seule, elle serait prise pour un
            // chemin de `wakfu.log` par `apply::parse_args`.
            it.next();
        } else {
            gardes.push(arg);
        }
    }
    gardes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        forwarded_args(v.iter().map(|s| (*s).to_string()))
    }

    #[test]
    fn le_chemin_de_journal_est_repasse() {
        assert_eq!(
            args(&["/tmp/wakfu.log"]),
            vec!["/tmp/wakfu.log".to_string()]
        );
    }

    #[test]
    fn le_drapeau_de_mise_a_jour_part_avec_sa_version() {
        assert_eq!(
            args(&["--updated-from", "0.22.0", "/tmp/wakfu.log"]),
            vec!["/tmp/wakfu.log".to_string()],
            "le process neuf ne vient pas d'une mise à jour : ni nettoyage, ni ligne de journal"
        );
    }

    #[test]
    fn un_drapeau_sans_version_ne_mange_pas_l_argument_suivant() {
        // `--updated-from` en fin de ligne : rien à consommer derrière, et surtout rien à
        // supprimer avant.
        assert_eq!(
            args(&["/tmp/wakfu.log", "--updated-from"]),
            vec!["/tmp/wakfu.log".to_string()]
        );
    }
}
