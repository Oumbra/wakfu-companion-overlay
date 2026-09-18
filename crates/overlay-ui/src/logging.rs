//! Journalisation de session (§15 du plan) : un fichier par jour dans le dossier de données de
//! l'appli — `%APPDATA%\wakfu-companion-overlay\logs\` sous Windows, XDG équivalent sous Linux
//! (même racine que `catalog_cache`/`token_store`/`watchlist`, voir `directories::ProjectDirs`) —
//! miroir de la sortie console (mêmes lignes, même horodatage) : le but est qu'une exécution
//! passée puisse être relue directement dans ce fichier, sans dépendre d'un copier-coller du
//! terminal.
//!
//! - **Horodatage** : UTC ISO-8601 avec microsecondes (`YYYY-MM-DDTHH:MM:SS.ffffffZ`), le timer
//!   PAR DÉFAUT de `tracing-subscriber` (`fmt::time::SystemTime`) — UTC plutôt qu'heure locale :
//!   tri lexical fiable, aucune ambiguïté de fuseau/heure d'été, aucune dépendance
//!   supplémentaire. Largement au-delà du besoin exprimé (seconde/milliseconde) pour rejouer des
//!   séquences rapprochées entre threads (auth/catalogue/moteur, voir `main.rs`).
//! - **Niveau** : `info` par défaut sur le code de l'appli, `warn` sur les dépendances graphiques
//!   bruyantes (wgpu/naga) — réglable sans recompiler via la variable d'env `RUST_LOG`
//!   (`tracing_subscriber::EnvFilter`, même convention que `overlay-app`), et à chaud par la case
//!   « Journal détaillé » de la fenêtre Options ([`set_verbose`]).
//! - **Rien de personnel en `info`** (constat C6 de `docs/analyse-rgpd.md`, 2026-09-18) : ni nom
//!   de personnage, ni pseudonyme d'autre joueur, ni chemin portant le nom de compte du système
//!   (`overlay_ingest::privacy::redact_path`), ni code d'appairage, ni contenu d'un lot refusé
//!   (`EngineError::Deserialize` n'en rend que la forme). Tout cela existe encore, en `debug!`,
//!   derrière la case ci-dessus — décochée par défaut. Le fichier est local et n'est jamais
//!   téléversé, mais il vit 14 jours et s'envoie tel quel avec un rapport de bug : ce qu'il
//!   contient par défaut est ce qu'on accepte de voir partir avec.
//! - **Rotation** : un fichier par jour (`tracing_appender::rolling::Rotation::DAILY`), 14
//!   conservés au-delà desquels les plus anciens sont supprimés automatiquement
//!   (`max_log_files`) — pas de perte de repli si l'overlay tourne des semaines sans être
//!   redémarré, pas de croissance illimitée sur un poste laissé tel quel.
//! - **Plafond de volume** : 16 Mio par jour ([`MAX_LOG_BYTES_PER_DAY`], `CappedAppender`). La
//!   rotation bornait la DURÉE de conservation, pas le volume qu'une panne en boucle écrit.
//! - **Écriture SYNCHRONE** (pas de `tracing_appender::non_blocking`) : le volume de lignes émis
//!   ici est faible (pas un chemin chaud comme le rendu ~20 Hz), et ça garantit qu'aucune ligne
//!   n'est perdue si le process s'arrête brutalement — Ctrl+C en particulier (voir
//!   `install_ctrlc_handler`), documenté comme sortie valide dans la bannière de `main()`, termine
//!   normalement le process SANS dérouler les `Drop` (un `tracing_appender::non_blocking` perdrait
//!   son tampon non vidé dans ce cas précis).
//! - **Bornes de session** : chaque lancement ouvre (`init`) et ferme (`log_session_end`, appelé à
//!   CHAQUE point de sortie du process — fermeture de fenêtre, bouton « Fermer l'overlay », zone
//!   de notification, Ctrl+C, échec de démarrage) une ligne `=== session … ===` portant le PID, qui sert d'identifiant de session
//!   (suffisant pour distinguer deux lancements consécutifs dans le fichier d'un même jour).
//! - Le jeton de compte ne doit **jamais** apparaître dans ces logs (§10 du plan) — seuls des
//!   messages de statut (succès/échec) sont journalisés côté appairage/réglages, jamais la valeur.
//! - **Console sous Windows (2026-09-17)** : l'exe est fenêtré sans fenêtre
//!   (`windows_subsystem = "windows"`, voir `main.rs`), Windows ne lui ouvre donc plus de console.
//!   La couche console n'a un destinataire que si le process a été lancé depuis un terminal et
//!   s'est rattaché à sa console ([`attach_parent_console`], à appeler avant [`init`]) ; sinon
//!   (double-clic, démarrage automatique, activation de protocole) ses écritures partent dans le
//!   vide sans erreur — la bibliothèque standard traite un handle standard absent comme un puits —
//!   et **le fichier est la seule sortie**. C'est une raison de plus de ne jamais rien y écrire
//!   directement (`println!`) : ce qui n'est pas dans `tracing` n'est nulle part.
//! - **Paniques** ([`install_panic_hook`]) : le message de panique de la bibliothèque standard va
//!   sur `stderr`, c'est-à-dire nulle part dans le cas ci-dessus. Le hook le recopie d'abord dans
//!   le journal (message + fichier:ligne), puis laisse faire le hook par défaut — un plantage se
//!   lit donc dans le fichier, au lieu de n'y laisser qu'une session sans ligne de fin.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

const APP_NAME: &str = "wakfu-companion-overlay";

/// `wgpu_hal`/`wgpu_core`/`naga` (via le crate `log`, pontée automatiquement vers `tracing` par
/// `tracing-subscriber` — feature `tracing-log`, activée par défaut) sont bruyants en `info` :
/// silencieux ici comme ils l'étaient déjà avec l'ancien réglage `env_logger`
/// (`"warn,wgpu_hal=info"`), la logique est juste inversée (tout l'app en `info`, ces crates en
/// `warn`).
const DEFAULT_FILTER: &str = "info,wgpu_hal=warn,wgpu_core=warn,naga=warn";

/// Filtre du mode « Journal détaillé » : `debug` sur le code de l'appli, dépendances graphiques
/// toujours muselées (un `debug` global sur `wgpu`/`naga` noierait le fichier en quelques
/// secondes, et ce n'est pas ce que l'utilisateur demande en cochant la case).
const VERBOSE_FILTER: &str = "debug,wgpu_hal=warn,wgpu_core=warn,naga=warn,rquickjs=info";

/// **Plafond de taille du journal, par jour** (16 Mio) — constat C6 de `docs/analyse-rgpd.md`.
///
/// La rotation journalière et les 14 fichiers conservés bornaient la DURÉE de conservation, pas le
/// VOLUME : une panne en boucle (fichier de jeu illisible, lot rejeté à chaque lot, panique
/// répétée) écrit autant de lignes que la boucle en produit, sans limite, sur un disque qui n'est
/// pas le nôtre. Au-delà de ce plafond, l'écriture sur DISQUE s'arrête pour la journée après une
/// dernière ligne qui le dit ; la console, elle, continue de tout recevoir.
///
/// 16 Mio est très au-dessus d'une journée de fonctionnement normal (quelques centaines de Kio) et
/// laisse largement de quoi diagnostiquer une panne bavarde avant la coupure.
const MAX_LOG_BYTES_PER_DAY: u64 = 16 * 1024 * 1024;

static SESSION_ID: OnceLock<u32> = OnceLock::new();

/// Poignée de rechargement du filtre — voir [`set_verbose`]. `None` tant que [`init`] n'a pas été
/// appelée (tests hors harnais de journalisation, binaire de focus de `main`).
static FILTER_HANDLE: OnceLock<reload::Handle<EnvFilter, tracing_subscriber::Registry>> =
    OnceLock::new();

/// Le fichier écrit-il encore, ou le plafond du jour est-il atteint ? Lu par la fenêtre Options
/// pour ne pas promettre un journal qui ne s'écrit plus.
static DAILY_CAP_REACHED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `RollingFileAppender` plafonné en volume — voir [`MAX_LOG_BYTES_PER_DAY`].
///
/// Le compteur est remis à zéro au changement de jour UTC, c'est-à-dire exactement quand
/// `tracing-appender` ouvre son fichier suivant (`Rotation::DAILY`, même horloge) : le plafond
/// s'applique donc bien PAR FICHIER, sans avoir à demander à l'appender quel fichier il tient (il
/// ne l'expose pas).
struct CappedAppender<W: Write> {
    inner: W,
    /// Jours écoulés depuis l'époque Unix, en UTC — l'unité de rotation.
    day: u64,
    written: u64,
    /// La ligne de coupure n'est écrite qu'une fois par jour, sinon elle remplacerait le flot
    /// qu'elle vient d'arrêter.
    cut_announced: bool,
}

/// Jours écoulés depuis l'époque Unix (UTC). Avant l'époque (horloge système déréglée), `0` :
/// toute valeur constante convient, le compteur ne sert qu'à détecter un CHANGEMENT de jour.
fn utc_day() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() / 86_400)
        .unwrap_or(0)
}

impl<W: Write> CappedAppender<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            day: utc_day(),
            written: 0,
            cut_announced: false,
        }
    }
}

impl<W: Write> Write for CappedAppender<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let today = utc_day();
        if today != self.day {
            self.day = today;
            self.written = 0;
            self.cut_announced = false;
            DAILY_CAP_REACHED.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        if self.written >= MAX_LOG_BYTES_PER_DAY {
            if !self.cut_announced {
                self.cut_announced = true;
                DAILY_CAP_REACHED.store(true, std::sync::atomic::Ordering::Relaxed);
                let mio = MAX_LOG_BYTES_PER_DAY / (1024 * 1024);
                // Écrite directement : passer par `tracing` ici rappellerait ce `write`.
                let _ = self.inner.write_all(
                    format!(
                        "=== journal interrompu : plafond de {mio} Mio atteint pour aujourd'hui \
                         (la console continue de tout recevoir ; reprise au prochain fichier) ===\n"
                    )
                    .as_bytes(),
                );
                let _ = self.inner.flush();
            }
            // Ligne AVALÉE, pas perdue en erreur : un journal plein ne doit jamais faire échouer
            // ce qui journalise (`tracing` remonterait l'erreur à chaque appel derrière).
            return Ok(buf.len());
        }
        let written = self.inner.write(buf)?;
        self.written += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Le plafond du jour est-il atteint (plus rien n'est écrit sur disque) ? Voir
/// [`MAX_LOG_BYTES_PER_DAY`].
pub fn daily_cap_reached() -> bool {
    DAILY_CAP_REACHED.load(std::sync::atomic::Ordering::Relaxed)
}

/// **Réglage « Journal détaillé »** (fenêtre Options › À propos), appliqué À CHAUD : coché, le
/// fichier reçoit aussi les `debug!` — les noms de personnages, le chemin réel de `wakfu.log`, le
/// destinataire d'une réponse en privé, le détail d'une erreur de désérialisation. Décoché (le
/// défaut), ces lignes-là n'existent nulle part.
///
/// C'est le pendant assumé du constat C6 (`docs/analyse-rgpd.md`) : ces informations restent
/// diagnosticables, mais seulement quand l'utilisateur les a demandées, et jamais dans le journal
/// d'une session ordinaire.
///
/// Sans effet si `RUST_LOG` est posée (réglage du développeur, qui prime) ou si [`init`] n'a pas
/// tourné.
pub fn set_verbose(verbose: bool) {
    let Some(handle) = FILTER_HANDLE.get() else {
        return;
    };
    if std::env::var_os("RUST_LOG").is_some() {
        return;
    }
    let wanted = if verbose {
        VERBOSE_FILTER
    } else {
        DEFAULT_FILTER
    };
    // `reload` reconstruit le cache d'intérêt de `tracing` : les `debug!` déjà compilés
    // redeviennent actifs sans redémarrage.
    if let Err(err) = handle.reload(EnvFilter::new(wanted)) {
        tracing::warn!("réglage du niveau de journal impossible : {err}");
        return;
    }
    tracing::info!(verbose, "niveau de journal réglé");
}

/// Dossier des journaux — public pour que `crate::local_data` puisse les effacer (ils portent
/// noms de personnages, chemin du log et auteur de message, constat C6 de
/// `docs/analyse-rgpd.md`).
pub fn log_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", APP_NAME).map(|dirs| dirs.data_dir().join("logs"))
}

fn build_appender(dir: &Path) -> std::io::Result<RollingFileAppender> {
    tracing_appender::rolling::Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("overlay-ui")
        .filename_suffix("log")
        .max_log_files(14)
        .build(dir)
        .map_err(std::io::Error::other)
}

fn filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER))
}

/// Rattache le process à la console du process qui l'a lancé, s'il en a une — Windows seulement,
/// sans effet ailleurs. À appeler **avant** [`init`], en tout premier dans `main()`.
///
/// Un exécutable de sous-système `windows` ne reçoit aucune console de Windows, même lancé depuis
/// un terminal (c'est tout l'objet : rien à l'écran au double-clic ni au démarrage automatique).
/// `AttachConsole(ATTACH_PARENT_PROCESS)` récupère celle du parent quand il en a une — `cargo run`
/// sous `preview.ps1`, ou l'exe tapé dans un terminal — et renseigne les handles standard : les
/// `println!`/couches console qui suivent y écrivent, et Ctrl+C dans ce terminal atteint le process
/// (`install_ctrlc_handler`) comme avant. Sans parent doté d'une console (Explorateur, clé `Run`,
/// activation de protocole), l'appel échoue : c'est le cas nominal, rien à journaliser (le journal
/// n'est d'ailleurs pas encore initialisé), l'exe reste muet et invisible.
///
/// Même approche que les émulateurs de terminal et applications graphiques Rust distribuées en
/// `windows_subsystem = "windows"` qui veulent quand même répondre à `--help` dans un terminal.
#[cfg(windows)]
pub fn attach_parent_console() {
    use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    // SAFETY : appel FFI sans pointeur ni état partagé ; l'échec (pas de console parente) est
    // rapporté dans le `Result`, jamais par un comportement indéfini.
    let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
}

/// Sans objet hors Windows : un exécutable Linux lancé depuis un terminal en hérite déjà, et
/// lancé par le bureau il n'en a pas — dans les deux cas rien à faire.
#[cfg(not(windows))]
pub fn attach_parent_console() {}

/// Recopie toute panique dans le journal avant le traitement par défaut — voir la doc du module.
/// À appeler une fois, juste après [`init`] (un hook posé avant n'aurait pas de subscriber).
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = info
            .payload_as_str()
            .unwrap_or("(charge utile non textuelle)");
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "emplacement inconnu".to_string());
        tracing::error!(location = %location, "panique : {message}");
        default_hook(info);
    }));
}

/// Initialise la journalisation (console + fichier). DOIT être appelée une seule fois, en tout
/// premier dans `main()` — avant `resolve_path()` y compris, pour qu'un échec de démarrage (log de
/// jeu introuvable) soit lui aussi journalisé. Retourne le DOSSIER des logs pour affichage dans la
/// bannière de démarrage (le fichier exact du jour, `overlay-ui.<AAAA-MM-JJ>.log`, n'est pas
/// recalculé ici — `tracing-appender` ne l'expose pas — voir la doc du module pour le motif) ;
/// `None` si l'écriture sur disque n'a pas pu être mise en place (dossier de données non
/// résolvable, ou non accessible en écriture) — l'appli démarre quand même, console seule : la
/// persistance sur disque est un confort de diagnostic, jamais une condition de démarrage.
pub fn init() -> Option<PathBuf> {
    let pid = std::process::id();
    let _ = SESSION_ID.set(pid);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_thread_names(true)
        .with_line_number(true);

    let (file_layer, resolved_dir) = match log_dir() {
        Some(dir) => match std::fs::create_dir_all(&dir).and_then(|_| build_appender(&dir)) {
            Ok(appender) => {
                // `RollingFileAppender` implémente `io::Write` mais pas `MakeWriter` directement
                // (il n'est pas `Clone`) — le `Mutex` est le pont standard de `tracing-subscriber`
                // (impl générique `MakeWriter for Mutex<W: Write>`) pour une écriture SYNCHRONE
                // (voir doc du module : pas de `non_blocking`, pour ne rien perdre sur Ctrl+C).
                // `CappedAppender` s'intercale pour borner le VOLUME du jour (voir
                // `MAX_LOG_BYTES_PER_DAY`) — la couche console, elle, n'est pas plafonnée.
                let layer = tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_thread_names(true)
                    .with_line_number(true)
                    .with_writer(std::sync::Mutex::new(CappedAppender::new(appender)));
                (Some(layer), Some(dir))
            }
            Err(err) => {
                eprintln!(
                    "[log] écriture du journal sur disque impossible dans {} ({err}) — console seule."
                    , dir.display()
                );
                (None, None)
            }
        },
        None => {
            eprintln!("[log] dossier de données introuvable — journal console seule.");
            (None, None)
        }
    };

    // Filtre RECHARGEABLE : la case « Journal détaillé » de la fenêtre Options le remplace à
    // chaud (voir [`set_verbose`]). Le journal démarre toujours au niveau ordinaire — la config
    // n'est pas encore lue à cet instant, et c'est bien le défaut voulu : rien de personnel dans
    // le fichier tant que l'utilisateur n'a rien demandé.
    let (filter_layer, filter_handle) = reload::Layer::new(filter());
    let _ = FILTER_HANDLE.set(filter_handle);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    tracing::info!(
        session_id = pid,
        version = crate::build_info::VERSION,
        commit = crate::build_info::COMMIT,
        os = std::env::consts::OS,
        "=== session démarrée ==="
    );

    resolved_dir
}

/// À appeler à CHAQUE point de sortie du process (fermeture de fenêtre, bouton « Fermer
/// l'overlay », zone de notification, Ctrl+C, échec de démarrage) — voir les appels dans
/// `main.rs`. `reason` identifie le déclencheur dans le
/// journal, pour distinguer un arrêt normal d'un plantage silencieux (aucune ligne `terminée` avant
/// la prochaine `démarrée` = sortie anormale).
pub fn log_session_end(reason: &str) {
    tracing::info!(
        session_id = SESSION_ID.get().copied().unwrap_or(0),
        reason,
        "=== session terminée ==="
    );
}

/// Ctrl+C (`SIGINT`/`CTRL_C_EVENT`) est un moyen de sortie documenté dans la bannière de `main()`
/// au même titre que le bouton « Fermer l'overlay » — sans gestionnaire dédié, le comportement PAR DÉFAUT de
/// l'OS termine le process immédiatement, sans dérouler les `Drop` ni journaliser de fin de
/// session : la moitié des sorties de l'appli n'auraient jamais de borne de fin exploitable.
pub fn install_ctrlc_handler() {
    if let Err(err) = ctrlc::set_handler(|| {
        log_session_end("Ctrl+C");
        std::process::exit(0);
    }) {
        tracing::warn!("installation du gestionnaire Ctrl+C impossible : {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le plafond du jour coupe l'écriture sur disque après une ligne qui l'annonce, et n'échoue
    /// jamais : `tracing` remonterait l'erreur à chaque appel derrière (voir `CappedAppender`).
    #[test]
    fn le_plafond_coupe_l_ecriture_sans_echouer() {
        let mut appender = CappedAppender::new(Vec::new());
        let ligne = vec![b'x'; 1024];
        let mut ecrit = 0u64;
        while ecrit < MAX_LOG_BYTES_PER_DAY {
            appender
                .write_all(&ligne)
                .expect("écriture sous le plafond");
            ecrit += ligne.len() as u64;
        }
        let avant_coupure = appender.inner.len();

        // Au-delà : accepté (pas d'erreur), mais plus rien du contenu ne part sur disque.
        appender
            .write_all(b"une ligne de trop")
            .expect("une écriture au-dessus du plafond ne doit jamais échouer");
        appender
            .write_all(b"et une autre")
            .expect("idem pour les suivantes");

        let apres = String::from_utf8_lossy(&appender.inner[avant_coupure..]).into_owned();
        assert!(
            apres.contains("plafond de 16 Mio atteint"),
            "la coupure doit être annoncée dans le fichier : {apres:?}"
        );
        assert!(
            !apres.contains("une ligne de trop") && !apres.contains("et une autre"),
            "rien ne doit plus être écrit après la coupure : {apres:?}"
        );
        assert_eq!(
            apres.matches("plafond de").count(),
            1,
            "la ligne de coupure ne s'écrit qu'une fois"
        );
        assert!(daily_cap_reached());
    }

    /// Le changement de jour UTC — celui-là même qui fait ouvrir un nouveau fichier à
    /// `tracing-appender` — remet le compteur à zéro.
    #[test]
    fn le_changement_de_jour_rouvre_le_robinet() {
        let mut appender = CappedAppender::new(Vec::new());
        appender.written = MAX_LOG_BYTES_PER_DAY;
        appender.write_all(b"bloquee").expect("pas d'erreur");
        assert!(!String::from_utf8_lossy(&appender.inner).contains("bloquee"));

        appender.day -= 1;
        appender.write_all(b"jour suivant").expect("pas d'erreur");
        assert!(
            String::from_utf8_lossy(&appender.inner).contains("jour suivant"),
            "le nouveau fichier du jour repart d'un compteur vide"
        );
    }
}
