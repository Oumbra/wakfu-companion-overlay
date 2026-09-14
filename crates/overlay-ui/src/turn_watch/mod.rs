//! Surveillance de tour — « prévenir que c'est à un de mes personnages de jouer » (§9.1 decies
//! du plan d'architecture, option `config::OverlayConfig::turn_notification`).
//!
//! Trois couches, du plus pur au plus lié à l'OS :
//!
//! - [`vision`] — lire le widget « Fin du tour » dans des pixels : localiser, extraire le nom,
//!   comparer à un gabarit. Sans OS, testé sur les captures du spike S4.
//! - [`watcher`] — la machine d'états par fenêtre : apprendre le gabarit du personnage quand le
//!   log dit qu'il joue, reconnaître son tour ensuite, décider quand notifier. Sans OS non plus :
//!   la capture et la notification lui sont injectées.
//! - `capture` / `notify` (Windows seulement) — `PrintWindow` de la bande basse d'une fenêtre, et
//!   le toast système. Le binaire X11 compile sans eux : l'option y est sans effet, et le dit.

pub mod vision;
pub mod watcher;

#[cfg(windows)]
pub mod capture;
#[cfg(windows)]
pub mod notify;
