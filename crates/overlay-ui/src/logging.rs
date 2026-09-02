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
//!   (`tracing_subscriber::EnvFilter`, même convention que `overlay-app`).
//! - **Rotation** : un fichier par jour (`tracing_appender::rolling::Rotation::DAILY`), 14
//!   conservés au-delà desquels les plus anciens sont supprimés automatiquement
//!   (`max_log_files`) — pas de perte de repli si l'overlay tourne des semaines sans être
//!   redémarré, pas de croissance illimitée sur un poste laissé tel quel.
//! - **Écriture SYNCHRONE** (pas de `tracing_appender::non_blocking`) : le volume de lignes émis
//!   ici est faible (pas un chemin chaud comme le rendu ~20 Hz), et ça garantit qu'aucune ligne
//!   n'est perdue si le process s'arrête brutalement — Ctrl+C en particulier (voir
//!   `install_ctrlc_handler`), documenté comme sortie valide dans la bannière de `main()`, termine
//!   normalement le process SANS dérouler les `Drop` (un `tracing_appender::non_blocking` perdrait
//!   son tampon non vidé dans ce cas précis).
//! - **Bornes de session** : chaque lancement ouvre (`init`) et ferme (`log_session_end`, appelé à
//!   CHAQUE point de sortie du process — fermeture de fenêtre, hotkey Quitter, Ctrl+C, échec de
//!   démarrage) une ligne `=== session … ===` portant le PID, qui sert d'identifiant de session
//!   (suffisant pour distinguer deux lancements consécutifs dans le fichier d'un même jour).
//! - Le jeton de compte ne doit **jamais** apparaître dans ces logs (§10 du plan) — seuls des
//!   messages de statut (succès/échec) sont journalisés côté appairage/réglages, jamais la valeur.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

const APP_NAME: &str = "wakfu-companion-overlay";

/// `wgpu_hal`/`wgpu_core`/`naga` (via le crate `log`, pontée automatiquement vers `tracing` par
/// `tracing-subscriber` — feature `tracing-log`, activée par défaut) sont bruyants en `info` :
/// silencieux ici comme ils l'étaient déjà avec l'ancien réglage `env_logger`
/// (`"warn,wgpu_hal=info"`), la logique est juste inversée (tout l'app en `info`, ces crates en
/// `warn`).
const DEFAULT_FILTER: &str = "info,wgpu_hal=warn,wgpu_core=warn,naga=warn";

static SESSION_ID: OnceLock<u32> = OnceLock::new();

fn log_dir() -> Option<PathBuf> {
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
                let layer = tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_thread_names(true)
                    .with_line_number(true)
                    .with_writer(std::sync::Mutex::new(appender));
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

    tracing_subscriber::registry()
        .with(filter())
        .with(stdout_layer)
        .with(file_layer)
        .init();

    tracing::info!(
        session_id = pid,
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        "=== session démarrée ==="
    );

    resolved_dir
}

/// À appeler à CHAQUE point de sortie du process (fermeture de fenêtre, hotkey Quitter, Ctrl+C,
/// échec de démarrage) — voir les appels dans `main.rs`. `reason` identifie le déclencheur dans le
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
/// au même titre que le hotkey Quitter — sans gestionnaire dédié, le comportement PAR DÉFAUT de
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
