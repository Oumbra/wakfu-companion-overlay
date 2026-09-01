//! Panneau "Dégâts du combat" — extrait de `main.rs::render` (L2) pour préparer la séparation en
//! zones indépendantes (§9 du plan), et enrichi ici du portrait de classe de chaque allié (roster
//! déclaré par l'utilisateur, sinon `breed` du combat — voir `overlay_engine::session` pour la
//! cascade, `crate::portraits::PortraitAtlas` pour le rendu).

use overlay_engine::{FightResult, FightSnapshot};

use crate::portraits::PortraitAtlas;

pub fn show(ui: &mut egui::Ui, fight: Option<&FightSnapshot>, portraits: &PortraitAtlas) {
    ui.strong("Dégâts du combat");
    match fight {
        None => {
            ui.weak("Aucun combat pour l'instant.");
        }
        Some(fight) => {
            let mut fighters = fight.fighters.clone();
            fighters.sort_by_key(|f| std::cmp::Reverse(f.total_damage));
            let max_damage = fighters.first().map(|f| f.total_damage).unwrap_or(0).max(1);
            for fighter in &fighters {
                ui.horizontal(|ui| {
                    let color = if fighter.is_ally {
                        egui::Color32::from_rgb(110, 200, 140)
                    } else {
                        egui::Color32::from_rgb(210, 100, 100)
                    };
                    // Portrait uniquement pour un allié dont la classe a pu être résolue (roster
                    // ou breed, voir session.rs) — les ennemis n'ont pas de classe (breed pas
                    // déterministe pour eux), et un allié pas encore classifié reste affiché sans
                    // portrait plutôt qu'avec un repli trompeur.
                    if fighter.is_ally {
                        if let Some(class_name) = &fighter.class_name {
                            if let Some(image) = portraits.image(class_name, fighter.gender) {
                                ui.add(image);
                            }
                        }
                    }
                    ui.colored_label(color, &fighter.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.monospace(fighter.total_damage.to_string());
                    });
                });
                let ratio = fighter.total_damage as f32 / max_damage as f32;
                ui.add(
                    egui::ProgressBar::new(ratio)
                        .desired_height(4.0)
                        .show_percentage()
                        .text(""),
                );
            }
            let status = match fight.result {
                None => "en cours".to_string(),
                Some(FightResult::Won) => "gagné".to_string(),
                Some(FightResult::Lost) => "perdu".to_string(),
            };
            ui.small(format!("Combat #{} — {status}", fight.fight_id));
        }
    }
}
