//! Curseur texte du jeu au survol d'un `design::input` : ce qu'`overlay_ui::cursor::apply` publie
//! dans `PlatformOutput::cursor_image` quand le pointeur est sur un VRAI champ de saisie du design
//! system (qui pose `CursorIcon::Text` par `on_hover_cursor`), et non un `set_cursor_icon` forcé
//! comme dans les tests unitaires du module. Complète
//! `le_curseur_du_jeu_remplace_le_curseur_systeme_et_clignote_sur_le_cliquable` de `panels.rs`
//! (flèche et main) pour le troisième curseur fixe.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use egui_kittest::Harness;
use overlay_ui::design::{self, InputSize};

#[test]
fn le_champ_de_saisie_montre_l_i_beam_du_jeu_sans_redessin() {
    let mut texte = String::from("gelano");
    let now = std::time::Instant::now();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 80.0))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            ui.horizontal_centered(|ui| {
                ui.add(
                    design::input(&mut texte)
                        .size(InputSize::Search)
                        .width(380.0)
                        .log_name("test.champ"),
                );
            });
            overlay_ui::cursor::apply(ui.ctx(), now);
        });
    let images = overlay_ui::cursor::images();
    let same = |a: &Option<egui::CustomCursorImage>, b: &egui::CustomCursorImage| {
        a.as_ref()
            .is_some_and(|a| std::sync::Arc::ptr_eq(&a.rgba, &b.rgba))
    };

    // Dans le vide, à droite du champ : flèche du jeu au repos.
    harness.hover_at(egui::pos2(550.0, 40.0));
    harness.run();
    assert!(
        same(&harness.output().platform_output.cursor_image, &images.idle),
        "vide : attendu le bitmap de repos"
    );

    // Sur le champ : l'I-beam du jeu, fixe — aucun redessin réclamé.
    harness.hover_at(egui::pos2(100.0, 40.0));
    harness.run();
    assert!(
        same(&harness.output().platform_output.cursor_image, &images.text),
        "champ : attendu l'I-beam du jeu, publié {:?}",
        harness
            .output()
            .platform_output
            .cursor_image
            .as_ref()
            .map(|i| (i.size, i.hotspot))
    );
    let delay = harness.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay;
    assert!(
        delay > std::time::Duration::from_secs(60),
        "l'I-beam ne doit réclamer aucun redessin : {delay:?}"
    );
}
