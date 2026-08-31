//! Lecture incrémentale d'un fichier suivi (« tail ») — cœur testable de l'ingestion, séparé de
//! tout déclenchement d'E/S (`notify`, minuteur de repli, etc., voir [`crate::watcher`]) pour
//! pouvoir rejouer des scénarios de rotation/troncature sans dépendre d'événements OS réels.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::rotation::FileIdentity;

/// Plafond de lignes par lot — reflète le budget du canal `crossbeam::channel<LineBatch>`
/// (docs/plan-architecture.md §3) : un fichier existant de 80 000 lignes ne doit jamais produire
/// un seul lot géant, qui gèlerait la publication du premier `UiSnapshot` intermédiaire (§5.5).
pub const MAX_BATCH_LINES: usize = 2000;

/// Un lot de lignes complètes fraîchement lues, dans l'ordre du fichier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineBatch {
    pub lines: Vec<String>,
    /// `true` tant que ce lot fait partie du rattrapage initial (relecture depuis le début lors
    /// d'une (re)connexion, ou après une rotation détectée) — voir docs/plan-architecture.md §5.3.
    /// Bascule à `false` dès qu'un `poll()` constate qu'il n'y a plus rien de neuf à lire : c'est
    /// la définition retenue ici de « avoir rattrapé le direct », l'Engine s'en sert pour ne
    /// jamais incrémenter les compteurs persistants (watchlist) pendant le rattrapage.
    pub is_initial_load: bool,
}

/// État de suivi d'un seul fichier. Ne fait aucune E/S tant que [`Tailer::poll`] n'est pas
/// appelé — c'est l'appelant (le watcher, ou un test) qui décide quand relire.
pub struct Tailer {
    path: PathBuf,
    identity: Option<FileIdentity>,
    offset: u64,
    /// Reliquat d'une ligne pas encore terminée par `\n` — jamais transmis tant qu'il n'est pas
    /// complet (docs/plan-architecture.md §5.2 : « jamais de parsing d'une demi-ligne »).
    pending: Vec<u8>,
    caught_up: bool,
}

impl Tailer {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            identity: None,
            offset: 0,
            pending: Vec::new(),
            caught_up: false,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Relit ce qu'il y a de neuf depuis le dernier appel. Renvoie une liste vide si le fichier
    /// est introuvable (**pas** une erreur : voir §5.1, l'absence n'est pas fatale) ou si rien n'a
    /// changé. Peut renvoyer plusieurs lots si plus de [`MAX_BATCH_LINES`] lignes sont arrivées
    /// d'un coup.
    pub fn poll(&mut self) -> io::Result<Vec<LineBatch>> {
        let mut file = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };

        let identity = FileIdentity::of(&file)?;
        let len = file.metadata()?.len();

        // Rotation/troncature (§5.2) : identité changée, ou taille retombée sous l'offset connu
        // (couvre le remplacement à taille croissante, où la seule taille ne suffit pas).
        let rotated = self.identity.is_some_and(|prev| prev != identity) || len < self.offset;
        if rotated {
            tracing::info!(path = %self.path.display(), "rotation/troncature détectée, relecture depuis le début");
            self.offset = 0;
            self.pending.clear();
            self.caught_up = false;
        }
        self.identity = Some(identity);

        if len == self.offset {
            // Rien de neuf : on vient de rattraper le direct (ou on l'avait déjà rattrapé).
            self.caught_up = true;
            return Ok(Vec::new());
        }

        file.seek(SeekFrom::Start(self.offset))?;
        let mut buf = Vec::with_capacity((len - self.offset) as usize);
        file.read_to_end(&mut buf)?;
        self.offset = len;
        self.pending.extend_from_slice(&buf);

        let mut lines = Vec::new();
        while let Some(pos) = self.pending.iter().position(|&b| b == b'\n') {
            let mut raw: Vec<u8> = self.pending.drain(..=pos).collect();
            raw.pop(); // '\n'
            if raw.last() == Some(&b'\r') {
                raw.pop(); // CRLF éventuel
            }
            // Décodage UTF-8 strict avec remplacement (§5.2) : une ligne corrompue ne doit jamais
            // interrompre l'ingestion des suivantes.
            lines.push(String::from_utf8_lossy(&raw).into_owned());
        }

        if lines.is_empty() {
            // Tout le contenu lu n'était qu'un reliquat de ligne incomplète : rien à publier.
            return Ok(Vec::new());
        }

        let is_initial_load = !self.caught_up;
        Ok(lines
            .chunks(MAX_BATCH_LINES)
            .map(|chunk| LineBatch {
                lines: chunk.to_vec(),
                is_initial_load,
            })
            .collect())
    }
}
