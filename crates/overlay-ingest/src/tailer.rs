//! Lecture incrémentale d'un fichier suivi (« tail ») — cœur testable de l'ingestion, séparé de
//! tout déclenchement d'E/S (`notify`, minuteur de repli, etc., voir [`crate::watcher`]) pour
//! pouvoir rejouer des scénarios de rotation/troncature sans dépendre d'événements OS réels.
//!
//! **`wakfu.log` est partagé par tous les clients Wakfu lancés** (multi-compte, constaté le
//! 2026-10-04 sur le fichier d'un utilisateur dont les compteurs de suivi ne bougeaient plus
//! pendant des combats en parallèle — docs/plan-architecture.md §5.2) : chaque client écrit à SON
//! propre pointeur, en écrasant ce qui s'y trouve ; le dernier lancé tronque le fichier et réécrit
//! depuis l'octet 0. Une lecture « depuis le dernier offset » ne voit donc que le client en tête :
//! les autres réécrivent DERRIÈRE la fin déjà lue. Le [`Tailer`] mémorise chaque ligne complète
//! déjà vue par (début, longueur, empreinte) et, à cadence bornée, relit tout le fichier pour
//! émettre chaque ligne absente de cette table à sa position. Miroir du `LogLineTracker` du dépôt
//! web (`src/app/core/utils/log-line-tracker.util.ts`) : tout changement se reporte des deux côtés.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::rotation::FileIdentity;

/// Plafond de lignes par lot — reflète le budget du canal `crossbeam::channel<LineBatch>`
/// (docs/plan-architecture.md §3) : un fichier existant de 80 000 lignes ne doit jamais produire
/// un seul lot géant, qui gèlerait la publication du premier `UiSnapshot` intermédiaire (§5.5).
pub const MAX_BATCH_LINES: usize = 2000;

/// Plafond d'une ligne gardée en mémoire — une vraie ligne de `wakfu.log` tient en quelques
/// centaines d'octets (résumé d'échange compris). Au-delà, la ligne reste suivie (empreinte
/// calculée au fil de l'eau) mais n'est jamais transmise : un fragment démesuré (fichier corrompu,
/// ou autre chose qu'un journal) ne doit ni occuper la mémoire ni atteindre le parseur.
const MAX_LINE_BYTES: usize = 1024 * 1024;
/// Taille d'une tranche lue (audit de sécurité du 2026-09-23, O7) : le fichier n'est jamais lu
/// d'un bloc, la mémoire ne porte qu'une tranche et la ligne en cours, face au budget de 300 Mo.
const READ_CHUNK_BYTES: usize = 4 * 1024 * 1024;
/// Débit de relecture complète : au plus une par seconde jusqu'à 4 Mo, puis une toutes les
/// `taille / 4 Mo` secondes. Un client en retard apparaît avec quelques secondes de délai au pire,
/// sans relire un très gros fichier à chaque événement `notify`.
const FULL_SCAN_BYTES_PER_SECOND: u64 = 4 * 1024 * 1024;
const MIN_FULL_SCAN_INTERVAL: Duration = Duration::from_secs(1);

/// Un lot de lignes complètes fraîchement lues, dans l'ordre du fichier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineBatch {
    pub lines: Vec<String>,
    /// `true` tant que ce lot fait partie du rattrapage initial (relecture depuis le début lors
    /// d'une (re)connexion, ou après une rotation détectée) — voir docs/plan-architecture.md §5.3.
    /// Bascule à `false` dès la fin du premier `poll()` qui a tout lu : c'est la définition retenue
    /// ici de « avoir rattrapé le direct », l'Engine s'en sert pour ne jamais incrémenter les
    /// compteurs persistants (watchlist) pendant le rattrapage.
    pub is_initial_load: bool,
}

/// Ligne complète déjà vue : `[start, start + len)` hors `\n` final. 16 octets par ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KnownLine {
    start: u64,
    len: u32,
    hash: u32,
}

impl KnownLine {
    fn end(&self) -> u64 {
        self.start + u64::from(self.len)
    }
}

/// FNV-1a 32 bits, calculé au fil de l'eau (une ligne peut chevaucher deux tranches).
const FNV_OFFSET: u32 = 0x811c_9dc5;
const FNV_PRIME: u32 = 0x0100_0193;

/// Découpe en lignes au fil des tranches. Une plage d'octets nuls (trou laissé par une troncature
/// qu'un autre client a prolongée en écrivant plus loin) sépare les lignes comme un `\n` : la
/// ligne commence après le dernier octet nul.
struct LineSplitter {
    start: u64,
    hash: u32,
    bytes: Vec<u8>,
    oversized: bool,
}

impl LineSplitter {
    fn new(start: u64) -> Self {
        Self {
            start,
            hash: FNV_OFFSET,
            bytes: Vec::new(),
            oversized: false,
        }
    }

    fn restart(&mut self, start: u64) {
        self.start = start;
        self.hash = FNV_OFFSET;
        self.bytes.clear();
        self.oversized = false;
    }

    /// Avance sur `chunk` (qui commence à la position absolue `base`) et appelle `on_line` pour
    /// chaque ligne non vide terminée par `\n`. Les octets d'une ligne inachevée restent en cours.
    fn feed(
        &mut self,
        base: u64,
        chunk: &[u8],
        on_line: &mut impl FnMut(KnownLine, Option<&[u8]>),
    ) {
        for (i, &byte) in chunk.iter().enumerate() {
            let pos = base + i as u64;
            match byte {
                b'\n' => {
                    let len = pos - self.start;
                    let mut content: &[u8] = &self.bytes;
                    if content.last() == Some(&b'\r') {
                        content = &content[..content.len() - 1];
                    }
                    let empty = if self.oversized {
                        false
                    } else {
                        content.is_empty()
                    };
                    if !empty {
                        let line = KnownLine {
                            start: self.start,
                            len: u32::try_from(len).unwrap_or(u32::MAX),
                            hash: self.hash,
                        };
                        on_line(line, (!self.oversized).then_some(content));
                    }
                    self.restart(pos + 1);
                }
                0 => self.restart(pos + 1),
                _ => {
                    self.hash = (self.hash ^ u32::from(byte)).wrapping_mul(FNV_PRIME);
                    if !self.oversized {
                        if self.bytes.len() < MAX_LINE_BYTES {
                            self.bytes.push(byte);
                        } else {
                            self.oversized = true;
                            self.bytes = Vec::new(); // libère la mémoire tout de suite
                        }
                    }
                }
            }
        }
    }
}

/// Début de ligne de journal (`INFO 15:34:15,233 ...`) : distingue une vraie ligne d'un fragment.
fn starts_with_log_header(line: &str) -> bool {
    let rest = line.trim_start();
    let Some(level_end) = rest.find(char::is_whitespace) else {
        return false;
    };
    if !matches!(
        &rest[..level_end],
        "INFO" | "WARN" | "ERROR" | "DEBUG" | "TRACE" | "FATAL"
    ) {
        return false;
    }
    let time = rest[level_end..].trim_start().as_bytes();
    // HH:MM:SS,mmm
    time.len() >= 12
        && time[..12].iter().enumerate().all(|(i, b)| match i {
            2 | 5 => *b == b':',
            8 => *b == b',',
            _ => b.is_ascii_digit(),
        })
}

/// État de suivi d'un seul fichier. Ne fait aucune E/S tant que [`Tailer::poll`] n'est pas
/// appelé — c'est l'appelant (le watcher, ou un test) qui décide quand relire.
pub struct Tailer {
    path: PathBuf,
    identity: Option<FileIdentity>,
    /// Lignes complètes déjà vues, triées par position (voir la doc du module).
    known: Vec<KnownLine>,
    last_len: u64,
    last_full_scan: Option<Instant>,
    caught_up: bool,
}

impl Tailer {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            identity: None,
            known: Vec::new(),
            last_len: 0,
            last_full_scan: None,
            caught_up: false,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Relit ce qu'il y a de neuf depuis le dernier appel. Renvoie une liste vide si le fichier
    /// est introuvable (**pas** une erreur : voir §5.1, l'absence n'est pas fatale) ou si rien n'a
    /// changé. Peut renvoyer plusieurs lots si plus de [`MAX_BATCH_LINES`] lignes sont arrivées
    /// d'un coup. Lit la seule fin du fichier, sauf quand une relecture complète est due (voir
    /// `FULL_SCAN_BYTES_PER_SECOND`) ou que la fin connue ne tient plus.
    pub fn poll(&mut self) -> io::Result<Vec<LineBatch>> {
        self.poll_inner(false)
    }

    /// Comme [`Tailer::poll`], en forçant la relecture complète (comparaison ligne à ligne avec
    /// ce qui a déjà été lu) — rattrape tout de suite un client multi-compte écrivant derrière la
    /// fin déjà lue. Sert aux tests ; `poll()` la déclenche seul à cadence bornée.
    pub fn rescan(&mut self) -> io::Result<Vec<LineBatch>> {
        self.poll_inner(true)
    }

    fn poll_inner(&mut self, force_full_scan: bool) -> io::Result<Vec<LineBatch>> {
        let mut file = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };

        let identity = FileIdentity::of(&file)?;
        let len = file.metadata()?.len();

        // Rotation (§5.2) : un autre fichier au même chemin — tout relire comme un rattrapage.
        // Une troncature EN PLACE n'en est pas une : c'est le lancement d'un client multi-compte,
        // qui réécrit depuis le début pendant que les autres continuent ; la relecture complète la
        // traite ligne à ligne (seules les lignes réécrites sont neuves).
        if self.identity.is_some_and(|prev| prev != identity) {
            tracing::info!(path = %crate::privacy::redact_path(&self.path), "rotation détectée, relecture depuis le début");
            self.known.clear();
            self.last_full_scan = None;
            self.caught_up = false;
        }
        self.identity = Some(identity);

        let full_scan_due = force_full_scan
            || self.last_full_scan.is_none_or(|at| {
                at.elapsed()
                    >= MIN_FULL_SCAN_INTERVAL
                        .max(Duration::from_secs(len / FULL_SCAN_BYTES_PER_SECOND))
            });
        if !full_scan_due && len == self.last_len {
            self.caught_up = true;
            return Ok(Vec::new());
        }
        self.last_len = len;

        // Capturé *avant* la lecture : c'est l'état de rattrapage au moment où ce lot a été
        // demandé qui détermine son `is_initial_load`, pas l'état une fois la lecture terminée.
        let is_initial_load = !self.caught_up;
        let mut out = BatchBuilder::new(is_initial_load);
        let tail_read = if full_scan_due {
            false
        } else {
            self.read_tail(&mut file, len, &mut out)?
        };
        if !tail_read {
            self.full_scan(&mut file, len, &mut out)?;
            self.last_full_scan = Some(Instant::now());
        }
        // Tout ce qui existait au début de ce `poll()` a été lu : on est désormais à jour. Sans ce
        // flag posé ici, une ligne ajoutée en direct juste après le rattrapage initial restait à
        // tort étiquetée rattrapage — observé en conditions réelles sur un vrai `wakfu.log`.
        self.caught_up = true;
        Ok(out.finish())
    }

    /// Position juste après la dernière ligne complète connue (son `\n`).
    fn tail_offset(&self) -> u64 {
        self.known.last().map_or(0, |line| line.end() + 1)
    }

    /// Lecture de la seule fin du fichier, depuis la dernière ligne connue. Renvoie `false` sans
    /// rien émettre si la jonction ne tient plus (fichier raccourci, `\n` final réécrit par un
    /// autre client) : une relecture complète est alors nécessaire.
    fn read_tail(&mut self, file: &mut File, len: u64, out: &mut BatchBuilder) -> io::Result<bool> {
        let tail = self.tail_offset();
        if len < tail {
            return Ok(false);
        }
        if tail > 0 {
            file.seek(SeekFrom::Start(tail - 1))?;
            let mut junction = [0u8; 1];
            file.read_exact(&mut junction)?;
            if junction[0] != b'\n' {
                return Ok(false);
            }
        } else {
            file.seek(SeekFrom::Start(0))?;
        }
        let mut splitter = LineSplitter::new(tail);
        let known = &mut self.known;
        read_chunks(file, tail, len, |base, chunk| {
            splitter.feed(base, chunk, &mut |line, content| {
                known.push(line);
                if let Some(content) = content {
                    out.push(String::from_utf8_lossy(content).into_owned());
                }
            });
        })?;
        Ok(true)
    }

    /// Relecture complète, comparée ligne à ligne à `known` (voir la doc du module). Le reste de
    /// l'ancienne ligne qu'un client en retard vient de recouvrir forme une « ligne » neuve sans
    /// en-tête juste avant une ligne inchangée : ce fragment n'est pas émis (le parseur le
    /// prendrait pour la suite multi-lignes de la ligne précédente) ; il est réévalué à la
    /// relecture suivante, le client l'ayant recouvert entre-temps.
    fn full_scan(&mut self, file: &mut File, len: u64, out: &mut BatchBuilder) -> io::Result<()> {
        let old = std::mem::take(&mut self.known);
        let old_end = old.last().map_or(0, |line| line.end() + 1);
        let mut next: Vec<KnownLine> = Vec::with_capacity(old.len() + 64);
        let mut old_index = 0;
        // Ligne neuve en attente de la suivante, pour savoir si c'est un fragment.
        let mut held: Option<(KnownLine, Option<String>)> = None;

        file.seek(SeekFrom::Start(0))?;
        let mut splitter = LineSplitter::new(0);
        read_chunks(file, 0, len, |base, chunk| {
            splitter.feed(base, chunk, &mut |line, content| {
                while old_index < old.len() && old[old_index].start < line.start {
                    old_index += 1;
                }
                let is_known = old.get(old_index) == Some(&line);
                if let Some((held_line, held_text)) = held.take() {
                    release(held_line, held_text, is_known, old_end, out);
                }
                if !is_known {
                    let text = content.map(|c| String::from_utf8_lossy(c).into_owned());
                    held = Some((line, text));
                }
                next.push(line);
            });
        })?;
        if let Some((held_line, held_text)) = held.take() {
            release(held_line, held_text, false, old_end, out);
        }
        self.known = next;
        Ok(())
    }
}

/// Émet une ligne neuve, sauf fragment (voir [`Tailer::full_scan`]).
fn release(
    line: KnownLine,
    text: Option<String>,
    followed_by_known_line: bool,
    old_end: u64,
    out: &mut BatchBuilder,
) {
    let Some(text) = text else { return };
    let is_fragment =
        followed_by_known_line && line.start < old_end && !starts_with_log_header(&text);
    if !is_fragment {
        out.push(text);
    }
}

/// Lit `[from, len)` par tranches de [`READ_CHUNK_BYTES`] (fichier déjà positionné sur `from`).
/// S'arrête sans erreur si le fichier raccourcit entre-temps.
fn read_chunks(
    file: &mut File,
    from: u64,
    len: u64,
    mut on_chunk: impl FnMut(u64, &[u8]),
) -> io::Result<()> {
    if len <= from {
        return Ok(());
    }
    let mut chunk = vec![0u8; READ_CHUNK_BYTES.min((len - from) as usize)];
    let mut offset = from;
    while offset < len {
        let want = READ_CHUNK_BYTES.min((len - offset) as usize);
        let read = file.read(&mut chunk[..want])?;
        if read == 0 {
            break;
        }
        on_chunk(offset, &chunk[..read]);
        offset += read as u64;
    }
    Ok(())
}

/// Range les lignes en lots d'au plus [`MAX_BATCH_LINES`].
struct BatchBuilder {
    batches: Vec<LineBatch>,
    current: Vec<String>,
    is_initial_load: bool,
}

impl BatchBuilder {
    fn new(is_initial_load: bool) -> Self {
        Self {
            batches: Vec::new(),
            current: Vec::new(),
            is_initial_load,
        }
    }

    fn push(&mut self, line: String) {
        self.current.push(line);
        if self.current.len() == MAX_BATCH_LINES {
            self.batches.push(LineBatch {
                lines: std::mem::take(&mut self.current),
                is_initial_load: self.is_initial_load,
            });
        }
    }

    fn finish(mut self) -> Vec<LineBatch> {
        if !self.current.is_empty() {
            self.batches.push(LineBatch {
                lines: self.current,
                is_initial_load: self.is_initial_load,
            });
        }
        self.batches
    }
}

#[cfg(test)]
mod tests {
    use super::starts_with_log_header;

    #[test]
    fn en_tete_de_journal_reconnu() {
        assert!(starts_with_log_header(
            " INFO 15:34:15,233 [AWT-EventQueue-0] (aNZ:174) - x"
        ));
        assert!(starts_with_log_header("ERROR 15:34:15,233 [main] x"));
        assert!(!starts_with_log_header("nus . "));
        assert!(!starts_with_log_header(":51:25,053 [AWT-EventQueue-0] x"));
        assert!(!starts_with_log_header(" INFO 15:34"));
    }
}
