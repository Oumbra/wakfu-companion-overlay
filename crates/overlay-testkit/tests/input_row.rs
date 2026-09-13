//! Géométrie d'un `design::input` dans une rangée horizontale : le widget qui le SUIT doit se poser
//! après le champ entier, pas après sa zone de texte.
//!
//! Défaut constaté le 2026-09-13 en maquettant l'onglet Chat (`examples/chat-mockups.rs`) : le
//! composant posait son `TextEdit` par `ui.scope_builder` sur la zone de texte — plus courte que le
//! champ des marges, de la place de la croix d'effacement et de l'icône de tête — et un scope
//! avance le curseur du parent jusqu'à la fin de CE rectangle. Le bouton suivant chevauchait le
//! champ de 17 px. Test de géométrie pur, sans capture : c'est un rectangle qui était faux, pas un
//! pixel.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use egui_kittest::Harness;
use overlay_ui::design::{self, ButtonSize, DsIcon, InputSize};

/// Rend une rangée « champ puis bouton » et renvoie `(rect du champ, rect du bouton)`.
fn rangee(clearable: bool, leading_icon: bool) -> (egui::Rect, egui::Rect) {
    let rects = std::rc::Rc::new(std::cell::Cell::new((
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
    )));
    let sortie = rects.clone();
    let mut texte = String::from("gelano");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 80.0))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.horizontal_centered(|ui| {
                let mut champ = design::input(&mut texte)
                    .size(InputSize::Search)
                    .clearable(clearable)
                    .width(380.0)
                    .log_name("test.champ");
                if leading_icon {
                    champ = champ.leading_icon(DsIcon::Search);
                }
                let champ = ui.add(champ);
                let bouton = ui.add(
                    design::button("Ajouter")
                        .size(ButtonSize::Compact)
                        .log_name("test.bouton"),
                );
                sortie.set((champ.rect, bouton.rect));
            });
        });
    harness.run();
    rects.get()
}

fn verifie(clearable: bool, leading_icon: bool) {
    let (champ, bouton) = rangee(clearable, leading_icon);
    assert!(
        bouton.left() >= champ.right(),
        "clearable={clearable} leading_icon={leading_icon} : le bouton commence à {} alors que le \
         champ va jusqu'à {} — il le chevauche de {} px",
        bouton.left(),
        champ.right(),
        champ.right() - bouton.left()
    );
}

#[test]
fn le_widget_suivant_se_pose_apres_le_champ_nu() {
    verifie(false, false);
}

#[test]
fn le_widget_suivant_se_pose_apres_le_champ_effacable() {
    verifie(true, false);
}

#[test]
fn le_widget_suivant_se_pose_apres_le_champ_a_loupe_et_croix() {
    verifie(true, true);
}
