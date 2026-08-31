//! Déclenchement d'E/S en continu : `notify` sur le répertoire parent + minuteur de repli,
//! au-dessus de [`Tailer::poll`]. Volontairement non couvert par des tests unitaires — la logique
//! qui a besoin de l'être (lecture incrémentale, rotation, troncature) est isolée dans
//! [`crate::tailer`], couverte par `tests/tailer.rs`. Ce module n'est qu'un fil qui appelle
//! `poll()` au bon moment ; sa correction dépend surtout de celle de `notify` elle-même.

use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{unbounded, Receiver, Sender};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::tailer::{LineBatch, Tailer};

/// Le client Java écrit par rafales (§5.2) : on laisse un court silence après le dernier
/// événement avant de relire, pour ne pas fragmenter une écriture en plusieurs lots inutiles.
const DEBOUNCE: Duration = Duration::from_millis(100);

/// Repli si `notify` ne remonte rien (montage réseau, certains systèmes de fichiers) — voir §5.2.
const POLL_FALLBACK_INTERVAL: Duration = Duration::from_secs(1);

/// Démarre le suivi de `path` sur un thread dédié et renvoie le canal sur lequel les lots
/// arrivent. Un premier `poll()` est effectué immédiatement (avant même le premier événement),
/// pour couvrir le cas où le fichier a déjà du contenu à la connexion.
///
/// Surveille le **répertoire parent**, pas le fichier lui-même : une rotation détruit l'inode
/// surveillé, watcher un chemin directement le laisserait orphelin (§5.2).
pub fn spawn(path: impl Into<PathBuf>) -> Receiver<std::io::Result<LineBatch>> {
    let path = path.into();
    let (out_tx, out_rx) = unbounded();

    thread::Builder::new()
        .name("overlay-ingest-watcher".into())
        .spawn(move || run(path, out_tx))
        .expect("échec de création du thread watcher");

    out_rx
}

fn run(path: PathBuf, out: Sender<std::io::Result<LineBatch>>) {
    let mut tailer = Tailer::new(&path);
    poll_and_send(&mut tailer, &out);

    let parent = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let (fs_tx, fs_rx) = std_mpsc::channel::<notify::Result<Event>>();
    // Jamais relu ensuite : sa seule raison d'être est de rester vivant (donc enregistré auprès
    // de l'OS) pour toute la durée de la boucle ; il est désenregistré au drop, en sortie de `run`.
    let _watcher: Option<RecommendedWatcher> = match RecommendedWatcher::new(
        fs_tx,
        notify::Config::default(),
    ) {
        Ok(mut w) => match w.watch(&parent, RecursiveMode::NonRecursive) {
            Ok(()) => Some(w),
            Err(err) => {
                tracing::warn!(%err, path = %parent.display(), "notify::watch a échoué, repli sur le sondage périodique seul");
                None
            }
        },
        Err(err) => {
            tracing::warn!(%err, "création du watcher notify impossible, repli sur le sondage périodique seul");
            None
        }
    };

    loop {
        match fs_rx.recv_timeout(POLL_FALLBACK_INTERVAL) {
            Ok(_event) => {
                // Débounce : on absorbe toute rafale d'événements suivante pendant DEBOUNCE avant
                // de relire, sans jamais attendre indéfiniment.
                while fs_rx.recv_timeout(DEBOUNCE).is_ok() {}
                poll_and_send(&mut tailer, &out);
            }
            Err(std_mpsc::RecvTimeoutError::Timeout) => {
                // Repli périodique (§5.2) : couvre les événements manqués par `notify`.
                poll_and_send(&mut tailer, &out);
            }
            Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                // Le watcher lui-même a été droppé (ne devrait pas arriver tant que la boucle
                // tourne) : on continue en sondage pur plutôt que de tuer le thread en silence.
                thread::sleep(POLL_FALLBACK_INTERVAL);
                poll_and_send(&mut tailer, &out);
            }
        }
    }
}

fn poll_and_send(tailer: &mut Tailer, out: &Sender<std::io::Result<LineBatch>>) {
    match tailer.poll() {
        Ok(batches) => {
            for batch in batches {
                if out.send(Ok(batch)).is_err() {
                    // Récepteur disparu : plus personne n'écoute, mais on ne panique pas pour
                    // autant — la boucle continue, `run` sera de toute façon abandonné avec le
                    // process (voir la note sur le cycle de vie plus haut).
                }
            }
        }
        Err(err) => {
            let _ = out.send(Err(err));
        }
    }
}
