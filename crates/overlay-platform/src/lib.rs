//! Primitives système spécifiques à l'OS pour `overlay-ui` : découverte de(s) fenêtre(s) de jeu par
//! titre et décision topmost/repli — voir `docs/plan-architecture.md` §17.2 (critère de sortie du
//! spike S3, point 1 : « migrer `src/discovery.rs`/`src/topmost.rs` [du spike] vers
//! `crates/overlay-platform/src/linux/x11.rs` ... migration = copie de fichier + raccordement,
//! pas une réécriture »). C'est exactement ce qui a été fait ici : `linux::x11`/`linux::topmost`
//! sont un portage direct de `spikes/s3-window-linux/src/{discovery,topmost}.rs`, validés
//! programmatiquement sous Xvfb par `spikes/s3-window-linux/harness.sh` (voir son README pour le
//! détail de cette validation, non reproduite ici).
//!
//! **Équivalent Windows volontairement absent d'ici** : `crates/overlay-ui/src/game_window.rs`
//! héberge déjà l'implémentation Windows (`EnumWindows`/`DwmGetWindowAttribute`, module privé
//! `imp` sous `#[cfg(target_os = "windows")]`) et n'a pas besoin d'être dupliquée ici — seul le
//! côté Linux (nouveau, jamais câblé avant cette session) justifiait une crate séparée
//! réutilisable indépendamment du fenêtrage.
//!
//! Module vide sous Windows (juste `mod linux {}` n'existe même pas) : `x11rb` n'est une
//! dépendance que sous `cfg(not(target_os = "windows"))` (voir `Cargo.toml`), donc `linux` lui-même
//! est gardé par le même `cfg` — jamais de dépendance X11 tirée dans le graphe Windows.

#[cfg(not(target_os = "windows"))]
pub mod linux;
