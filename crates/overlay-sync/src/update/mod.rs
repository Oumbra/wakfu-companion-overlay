//! **Mise à jour automatique de l'overlay** — `docs/plan-mise-a-jour.md` §7 (phase 2).
//!
//! Le CI publie, à chaque fusion sur `main`, une Release GitHub portant un manifeste `latest.json`
//! signé `minisign` et un binaire gzip par plateforme (`xtask dist`, phase 1). Ce module est
//! l'autre moitié : lire ce manifeste, décider s'il y a plus récent que [`current version`]
//! (`semver`), télécharger l'asset de la plateforme **en flux** vers le dossier de données, le
//! vérifier (SHA-256 avant ET après décompression), puis remplacer l'exe en cours d'exécution
//! (`self-replace`) et relancer.
//!
//! Ce que ce module ne fait PAS : décider QUAND (au démarrage, sur un clic — c'est le thread de
//! fond `overlay_ui::background::spawn_update_thread`) ni comment l'afficher (fenêtre de
//! connexion, fenêtre Options). Il n'a pas non plus de runtime async : `ureq` bloquant, comme tout
//! le réseau de l'overlay (§7.3 du plan d'architecture).
//!
//! **Sécurité (§10 du plan d'architecture)** : rien n'est installé sans que la signature du
//! manifeste ait été vérifiée avec [`PUBLIC_KEY`] — la clé publique commitée à la racine du
//! dépôt, embarquée ici par `include_str!`. Un manifeste sans signature valide vaut « pas de mise
//! à jour », jamais un repli sur du contenu non signé. Les assets sont ensuite vérifiés par le
//! SHA-256 que ce manifeste signé annonce.
//!
//! Sous-modules : [`manifest`] (format + signature + verdict), [`download`] (flux vers un fichier,
//! progression), [`apply`] (décompression, hachage, remplacement, relance).

pub mod apply;
pub mod download;
pub mod manifest;

use std::path::PathBuf;
use std::time::Instant;

pub use manifest::{Asset, Manifest, Verdict};

/// La clé publique de signature des Releases — `wakfu-overlay.pub` à la racine du dépôt (générée
/// le 2026-09-15, voir `docs/plan-mise-a-jour.md` §6.1). Le CI revérifie chaque manifeste avec ce
/// même fichier avant de publier : un secret qui ne lui correspondrait pas ne produit jamais de
/// Release.
pub const PUBLIC_KEY: &str = include_str!("../../../../wakfu-overlay.pub");

/// Dépôt des Releases — l'URL `releases/latest/download/…` est un simple redirect web, hors API
/// GitHub, donc sans quota (§3 du plan).
const RELEASES_BASE_URL: &str = "https://github.com/Oumbra/wakfu-companion-overlay/releases";

/// Nom du manifeste et de sa signature, tels que `xtask dist` les publie.
pub const MANIFEST_NAME: &str = "latest.json";
pub const SIGNATURE_NAME: &str = "latest.json.minisig";

/// La plateforme de CE binaire, clé de `Manifest::assets` — même vocabulaire que `xtask dist`
/// (`--asset windows-x86_64=…`).
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub const PLATFORM: &str = "windows-x86_64";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub const PLATFORM: &str = "linux-x86_64";
#[cfg(not(any(
    all(target_os = "windows", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "x86_64")
)))]
pub const PLATFORM: &str = "unsupported";

/// Surcharge de l'origine des fichiers de Release — un dossier servi en HTTP local
/// (`python -m http.server` sur la sortie de `xtask dist`) suffit à rejouer tout le mécanisme sans
/// publier quoi que ce soit. Sans elle, GitHub Releases.
const OVERRIDE_ENV: &str = "WAKFU_OVERLAY_UPDATE_URL";

/// URL du manifeste (et de sa signature : même base, `SIGNATURE_NAME`).
pub fn manifest_url(name: &str) -> String {
    match std::env::var(OVERRIDE_ENV) {
        Ok(base) => format!("{}/{name}", base.trim_end_matches('/')),
        Err(_) => format!("{RELEASES_BASE_URL}/latest/download/{name}"),
    }
}

/// URL d'un asset d'une version donnée — reconstruite depuis `version` + `name`, le manifeste ne
/// portant aucune URL absolue (§5 du plan : c'est ce qui permet de le relayer par l'API un jour
/// sans le réécrire).
pub fn asset_url(version: &str, name: &str) -> String {
    match std::env::var(OVERRIDE_ENV) {
        Ok(base) => format!("{}/{name}", base.trim_end_matches('/')),
        Err(_) => format!("{RELEASES_BASE_URL}/download/v{version}/{name}"),
    }
}

/// Dossier de travail des mises à jour : `<data_dir>/updates/` (à côté du cache catalogue et du
/// jeton). Les téléchargements partiels (`.part`), l'exe mis en place (`staged-…`) et l'ancien exe
/// gardé un cycle (`previous…`) y vivent.
pub fn updates_dir() -> PathBuf {
    #[cfg(not(test))]
    const APP_NAME: &str = "wakfu-companion-overlay";
    #[cfg(test)]
    const APP_NAME: &str = "wakfu-companion-overlay-test";
    directories::ProjectDirs::from("", "", APP_NAME)
        .map(|dirs| dirs.data_dir().join("updates"))
        .unwrap_or_else(|| PathBuf::from("updates"))
}

/// Ce que le reste de l'overlay voit du mécanisme — publié par le thread de fond via
/// `Arc<ArcSwap<UpdateStatus>>` (même motif qu'`AuthStatus`), lu par la fenêtre de connexion et la
/// fenêtre Options. Un seul état à la fois ; les transitions sont celles du §7 du plan.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum UpdateStatus {
    /// Rien demandé encore.
    #[default]
    Idle,
    /// Manifeste en cours de lecture.
    Checking,
    /// Le binaire courant est le plus récent publié.
    UpToDate { checked_at: Instant },
    /// Une version plus récente existe ; `mandatory` si la version courante est sous
    /// `minimumVersion` du manifeste.
    Available {
        version: String,
        download_size: u64,
        mandatory: bool,
        notes_url: Option<String>,
        checked_at: Instant,
    },
    /// Téléchargement de l'asset en cours — `total` connu par `Content-Length`, sinon la taille
    /// annoncée par le manifeste.
    Downloading {
        version: String,
        received: u64,
        total: u64,
    },
    /// Hachage et décompression.
    Verifying { version: String },
    /// L'exe de la nouvelle version est prêt sur le disque ; l'hôte n'a plus qu'à
    /// [`apply::install_and_relaunch`]. `mandatory` : ce que l'hôte publiera en `Failed` si le
    /// remplacement échoue.
    ReadyToInstall {
        version: String,
        staged: PathBuf,
        mandatory: bool,
    },
    /// Remplacement en cours (quelques centaines de millisecondes).
    Installing { version: String },
    /// Vérification impossible (hors ligne, manifeste illisible, signature invalide…) — pas un
    /// échec à montrer en rouge : l'overlay démarre avec sa version, on réessaiera.
    Unavailable { reason: String, checked_at: Instant },
    /// Un téléchargement ou une installation a échoué — titre court + détail technique, comme
    /// `AuthFailure`. `mandatory` : la version courante ne peut pas continuer (voir `Available`).
    Failed {
        headline: String,
        detail: String,
        mandatory: bool,
    },
}

impl UpdateStatus {
    /// Une opération est en cours (rien d'autre à lancer tant qu'elle ne s'est pas terminée).
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            Self::Checking
                | Self::Downloading { .. }
                | Self::Verifying { .. }
                | Self::Installing { .. }
        )
    }

    /// Le démarrage doit attendre : quelque chose s'installe, ou une mise à jour obligatoire n'a
    /// pas abouti.
    pub fn blocks_startup(&self) -> bool {
        matches!(
            self,
            Self::Downloading { .. }
                | Self::Verifying { .. }
                | Self::ReadyToInstall { .. }
                | Self::Installing { .. }
                | Self::Failed {
                    mandatory: true,
                    ..
                }
        )
    }

    /// La version qu'une opération concerne, si elle en a une.
    pub fn version(&self) -> Option<&str> {
        match self {
            Self::Available { version, .. }
            | Self::Downloading { version, .. }
            | Self::Verifying { version }
            | Self::ReadyToInstall { version, .. }
            | Self::Installing { version } => Some(version),
            _ => None,
        }
    }

    /// Quand la dernière vérification s'est terminée, si elle s'est terminée.
    pub fn checked_at(&self) -> Option<Instant> {
        match self {
            Self::UpToDate { checked_at }
            | Self::Available { checked_at, .. }
            | Self::Unavailable { checked_at, .. } => Some(*checked_at),
            _ => None,
        }
    }
}

/// Ce que le mécanisme peut refuser ou rater — chaque variante correspond à un message de la
/// fenêtre de connexion ou de la fenêtre Options.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("réseau : {0}")]
    Network(String),
    #[error("réponse HTTP {status} pour {url}")]
    Http { status: u16, url: String },
    #[error("manifeste illisible : {0}")]
    Manifest(String),
    #[error("signature du manifeste invalide : {0}")]
    Signature(String),
    #[error("aucun asset publié pour {0}")]
    NoAsset(&'static str),
    #[error("empreinte SHA-256 inattendue pour {what} — fichier corrompu ou altéré")]
    HashMismatch { what: String },
    #[error("fichier : {0}")]
    Io(String),
    #[error("version illisible : {0}")]
    Version(String),
}

impl From<std::io::Error> for UpdateError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

/// Titre court d'une erreur, pour un écran — le détail technique est `err.to_string()`.
pub fn headline(err: &UpdateError) -> &'static str {
    match err {
        UpdateError::Network(_) => "Serveur de mise à jour injoignable",
        UpdateError::Http { .. } => "Réponse inattendue du serveur de mise à jour",
        UpdateError::Manifest(_) => "Manifeste de mise à jour illisible",
        UpdateError::Signature(_) => "Mise à jour non signée — refusée",
        UpdateError::NoAsset(_) => "Aucune version publiée pour cette plateforme",
        UpdateError::HashMismatch { .. } => "Fichier de mise à jour corrompu",
        UpdateError::Io(_) => "Écriture impossible",
        UpdateError::Version(_) => "Numéro de version illisible",
    }
}

/// Une taille lisible : « 3,1 Mo », « 512 Ko ». Virgule décimale — l'overlay parle français.
pub fn human_size(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1} Mo", bytes as f64 / 1_000_000.0).replace('.', ",")
    } else {
        format!("{} Ko", bytes / 1_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_par_defaut_et_surchargees() {
        std::env::remove_var(OVERRIDE_ENV);
        assert_eq!(
            manifest_url(MANIFEST_NAME),
            "https://github.com/Oumbra/wakfu-companion-overlay/releases/latest/download/latest.json"
        );
        assert_eq!(
            asset_url("0.21.0", "a.gz"),
            "https://github.com/Oumbra/wakfu-companion-overlay/releases/download/v0.21.0/a.gz"
        );
        std::env::set_var(OVERRIDE_ENV, "http://127.0.0.1:8000/dist/");
        assert_eq!(
            manifest_url(MANIFEST_NAME),
            "http://127.0.0.1:8000/dist/latest.json"
        );
        assert_eq!(
            asset_url("0.21.0", "a.gz"),
            "http://127.0.0.1:8000/dist/a.gz"
        );
        std::env::remove_var(OVERRIDE_ENV);
    }

    #[test]
    fn cle_publique_embarquee_lisible() {
        let pk = minisign_verify::PublicKey::decode(PUBLIC_KEY.trim());
        assert!(pk.is_ok(), "wakfu-overlay.pub illisible : {pk:?}");
    }

    #[test]
    fn tailles_lisibles() {
        assert_eq!(human_size(3_100_000), "3,1 Mo");
        assert_eq!(human_size(512_000), "512 Ko");
    }

    #[test]
    fn etats_bloquants() {
        assert!(!UpdateStatus::Idle.blocks_startup());
        assert!(!UpdateStatus::Checking.blocks_startup());
        assert!(UpdateStatus::Downloading {
            version: "1.0.0".into(),
            received: 0,
            total: 1
        }
        .blocks_startup());
        assert!(UpdateStatus::Failed {
            headline: "x".into(),
            detail: "y".into(),
            mandatory: true
        }
        .blocks_startup());
        assert!(!UpdateStatus::Failed {
            headline: "x".into(),
            detail: "y".into(),
            mandatory: false
        }
        .blocks_startup());
    }
}
