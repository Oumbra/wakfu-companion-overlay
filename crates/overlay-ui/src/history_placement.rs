//! **Taille et position de la fenêtre d'historique**, retrouvées d'une ouverture à l'autre.
//!
//! Un petit fichier à part, `historique-fenetre.json`, à côté de `config.toml` — et non une clé de
//! la configuration : la fenêtre vit dans son propre processus (`--historique`), qui peut tourner
//! en même temps que l'overlay ; deux processus qui réécrivent le même `config.toml` finiraient
//! par perdre un réglage de l'autre. L'effacement des données locales emporte tout le dossier de
//! configuration, ce fichier compris.
//!
//! Coordonnées **physiques** (pixels de l'écran), celles que winit rend et accepte. Une position
//! qui ne tombe plus sur aucun écran (écran débranché, résolution changée) est ignorée : la
//! fenêtre s'ouvre alors centrée, comme la première fois.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Un écran, en coordonnées physiques : position et taille.
#[derive(Debug, Clone, Copy)]
pub struct Screen {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Part minimale de la bannière qui doit rester sur un écran pour qu'on puisse encore attraper la
/// fenêtre.
const GRIP: i32 = 80;

impl Placement {
    /// Vrai si le haut de la fenêtre (sa bannière, la seule poignée de déplacement) tombe sur l'un
    /// des écrans, sur au moins [`GRIP`] pixels.
    pub fn reachable(&self, screens: &[Screen]) -> bool {
        screens.iter().any(|screen| {
            let left = self.x.max(screen.x);
            let right = (self.x + self.width as i32).min(screen.x + screen.width as i32);
            let top_inside = self.y >= screen.y && self.y < screen.y + screen.height as i32 - GRIP;
            right - left >= GRIP && top_inside
        })
    }
}

fn path() -> Option<PathBuf> {
    crate::config::project_dirs().map(|dirs| dirs.config_dir().join("historique-fenetre.json"))
}

/// La dernière taille et position enregistrées, s'il y en a une lisible.
pub fn load() -> Option<Placement> {
    let text = std::fs::read_to_string(path()?).ok()?;
    serde_json::from_str(&text)
        .map_err(|err| tracing::warn!(%err, "historique : placement illisible, ignoré"))
        .ok()
}

/// Enregistre la taille et la position — best-effort : un échec est journalisé, rien de plus.
pub fn save(placement: &Placement) {
    let Some(path) = path() else {
        return;
    };
    let result = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| {
            let text = serde_json::to_string(placement).map_err(std::io::Error::other)?;
            std::fs::write(&path, text)
        });
    if let Err(err) = result {
        tracing::warn!(%err, "historique : placement non enregistré");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_fenetre_hors_ecran_est_ignoree() {
        let screens = [Screen {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];
        let at = |x, y| Placement {
            x,
            y,
            width: 1100,
            height: 820,
        };
        assert!(at(400, 100).reachable(&screens));
        assert!(
            at(1800, 100).reachable(&screens),
            "bannière en partie visible"
        );
        assert!(!at(1900, 100).reachable(&screens), "plus assez de prise");
        assert!(
            !at(400, -40).reachable(&screens),
            "bannière au-dessus de l'écran"
        );
        assert!(!at(400, 1050).reachable(&screens));
        assert!(!at(-3000, 100).reachable(&screens), "écran débranché");
    }
}
