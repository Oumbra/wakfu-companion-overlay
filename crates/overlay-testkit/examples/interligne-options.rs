//! **Simulation d'interligne des lignes d'option** — retour utilisateur du 2026-09-14 : « il y a un
//! gros écart entre l'espacement entre les lignes du design system du jeu et celui qui est utilisé
//! dans la modale de l'overlay ».
//!
//! Le constat est exact, et il se mesure. Le relevé
//! [`releve-section-options.json`](../../../docs/design-system/releve-section-options.json) le dit
//! depuis le premier jour — « Rythme : 31 px entre deux lignes d'option consécutives » — mais
//! l'a rangé du côté de la mise en page (« affaire de la mise en page, pas du composant », doc de
//! `design::components::checkbox`), et **aucune mise en page ne l'avait posé**. Une case fait 20 px
//! de haut, `design::panel` met `item_spacing.y` à zéro : deux cases empilées se suivaient donc à
//! 20 px, contre 31 dans le jeu. **11 px manquants par ligne**, mesurés au pixel des deux côtés :
//!
//! | Source | Haut de la 1ʳᵉ case | Haut de la 2ᵉ | Pas |
//! | --- | --- | --- | --- |
//! | `interface-options-jeu.png` (jeu) | 175 | 206 | **31 px** |
//! | `options_parametres_compte.png` (overlay, avant) | 256 | 276 | **20 px** |
//!
//! Cet outil rend la section « Combat » de l'onglet Paramètres — les trois vraies cases, dont la
//! troisième en retrait — pour une série d'interlignes candidats, chacune dans le vrai chrome
//! (`design::window` + `design::panel`) et avec les vrais composants. Il sert à juger sur pièce
//! avant d'arrêter la valeur, jamais à figer un rendu : les captures partent dans `target/`, pas au
//! dépôt (§17.4 du plan).
//!
//! ```bash
//! cargo run -p overlay-testkit --example interligne-options
//! # écrit target/interligne/interligne-XX.png, un par valeur de VARIANTES
//! ```

use egui::Vec2;
use egui_kittest::Harness;
use overlay_ui::design::{self, tokens};
use overlay_ui::panels::options_modal::OptionsTab;

/// Interlignes mis en rendu, en pixels : l'état d'avant (0), les deux valeurs proposées à vue d'œil
/// par l'utilisateur (5 et 7), la valeur **mesurée** sur le jeu (11 = pas de 31 − case de 20), et
/// une valeur au-delà pour borner le jugement (14).
const VARIANTES: [f32; 5] = [0.0, 5.0, 7.0, 11.0, 14.0];

/// Taille de rendu — la largeur réelle de la fenêtre Options
/// (`options_modal::WINDOW_SIZE`), tronquée en hauteur : seule la section qui nous occupe est
/// rendue, une fenêtre de 810 px de haut ne montrerait que du vide sous elle.
const TAILLE: Vec2 = Vec2::new(760.0, 300.0);

/// Fond posé sous la fenêtre — le gris du damier de `tests/panels.rs`, pour que les angles arrondis
/// de la fenêtre ne flottent pas sur du noir.
const BACKDROP: egui::Color32 = egui::Color32::from_rgb(0x2B, 0x2B, 0x2B);

fn sortie() -> std::path::PathBuf {
    let dir = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(target) => std::path::PathBuf::from(target),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"),
    }
    .join("interligne");
    std::fs::create_dir_all(&dir).expect("création de target/interligne");
    dir
}

/// Rend la section « Combat » avec `gap` pixels entre deux lignes d'option.
///
/// Le contenu reprend mot pour mot celui de `options_modal` (section « Combat », 2026-09-14) :
/// c'est ce qui est jugé, un texte de remplissage plus court ou plus long changerait la densité de
/// la planche et donc le jugement.
fn rendu(gap: f32) -> image::RgbaImage {
    let mut tab = OptionsTab::Parametres;
    let mut toujours_visible = false;
    let mut notification = true;
    let mut muet = false;

    let mut harness = Harness::builder().with_size(TAILLE).build_ui(move |ui| {
        overlay_ui::style::apply(ui.ctx());
        egui::Frame::NONE.fill(BACKDROP).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let chrome = design::window("Options").log_name("interligne").show(ui);
            chrome.tabs(
                ui,
                design::tabs(&mut tab)
                    .entry(OptionsTab::Parametres, "Paramètres")
                    .log_name("interligne-onglets"),
            );
            design::panel().show(ui, chrome.content, |ui, _panel| {
                ui.add(design::heading("Combat"));
                ui.add(design::checkbox(
                    &mut toujours_visible,
                    "Afficher le panneau de combat en dehors des combats",
                ));
                ui.add_space(gap);
                ui.add(design::checkbox(
                    &mut notification,
                    "Me prévenir quand un de mes personnages doit jouer",
                ));
                ui.add_space(gap);
                ui.horizontal(|ui| {
                    ui.add_space(tokens::CHECKBOX_SIZE + tokens::CHECKBOX_LABEL_GAP);
                    ui.add(design::checkbox(
                        &mut muet,
                        "Couper le son des notifications",
                    ));
                });
            });
        });
    });
    harness.run();
    harness
        .render()
        .expect("rendu offscreen — voir doc de module")
}

fn main() {
    let dir = sortie();
    for gap in VARIANTES {
        let image = rendu(gap);
        let chemin = dir.join(format!("interligne-{:02}.png", gap as u32));
        image.save(&chemin).expect("écriture de la capture");
        println!("écrit {}", chemin.display());
    }
}
