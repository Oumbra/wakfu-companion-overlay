//! **Les bords de la liste déroulante** — planche de contrôle du composant `design::select`
//! déplié, rendue par le composant lui-même.
//!
//! Demande du 2026-09-14, capture à l'appui : comparé au jeu, la liste dépliée montrait deux
//! défauts de bord.
//!
//! 1. **Les deux coins bas n'étaient pas arrondis** — la référence `select-simple-opened.png` les
//!    montre pourtant rentrants de 3 px sur les quatre dernières lignes (fond à x=7 en y=151,
//!    x=8 en 152, x=9 en 153, bord noir à x=10 en 154).
//! 2. **La mise en avant d'une entrée débordait du conteneur** — peinte après le bord et le
//!    liseré, elle les recouvrait : l'entrée du haut mangeait le liseré clair qui détache la liste
//!    du socle, celle du bas sortait par les coins.
//!
//! La planche pose les trois cas où le défaut se voyait — première entrée en avant, dernière
//! entrée en avant, choix multiple — sur le fond sombre d'un panneau, pour juger le contour.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`.
//!
//! ```text
//! cargo run -p overlay-testkit --example select-bords
//! ```

use egui::Color32;
use egui_kittest::Harness;
use overlay_ui::design;

/// Largeur des listes de la planche — celle de la capture du jeu, à 6 px près.
const LARGEUR: f32 = 214.0;
/// Gouttière entre deux colonnes.
const COL_GAP: f32 = 34.0;
/// Gris des libellés — celui des légendes de la galerie.
const CAPTION: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);
/// Fond de la planche — un gris franchement plus clair que le bord noir de la liste. Sur le noir
/// du harnais, le bord et le fond se confondaient : impossible de juger un contour qu'on ne voit
/// pas.
const FOND: Color32 = Color32::from_rgb(0x2B, 0x30, 0x36);

fn dossier() -> std::path::PathBuf {
    let dir = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(target) => std::path::PathBuf::from(target),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"),
    }
    .join("mockups");
    std::fs::create_dir_all(&dir).expect("création de target/mockups");
    dir
}

fn libelle(ui: &mut egui::Ui, texte: &str) {
    ui.label(egui::RichText::new(texte).color(CAPTION).size(13.0));
}

fn main() {
    let largeur_planche = 3.0 * (LARGEUR + COL_GAP) + 20.0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(largeur_planche, 300.0))
        .build_ui(move |ui| {
            let ctx = ui.ctx().clone();
            overlay_ui::style::apply(&ctx);
            ui.painter().rect_filled(ui.max_rect(), 0, FOND);
            ui.spacing_mut().item_spacing.y = 8.0;

            // `preview_open` force le dépli : en rendu offscreen, personne ne clique sur le socle.
            let mut premier = 0_u8;
            let mut dernier = 3_u8;
            let mut canaux: Vec<u8> = vec![0, 2];

            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(LARGEUR);
                    libelle(ui, "1re entrée en avant");
                    ui.add(
                        design::select(&mut premier)
                            .option(0, "7 j")
                            .option(1, "14 j")
                            .option(2, "28 j")
                            .width(LARGEUR)
                            .preview_open(true)
                            .log_name("planche.select-premier"),
                    );
                });
                ui.add_space(COL_GAP);
                ui.vertical(|ui| {
                    ui.set_width(LARGEUR);
                    libelle(ui, "dernière entrée en avant");
                    ui.add(
                        design::select(&mut dernier)
                            .option(0, "Proximité")
                            .option(1, "Groupe")
                            .option(2, "Guilde")
                            .option(3, "Commerce")
                            .width(LARGEUR)
                            .preview_open(true)
                            .log_name("planche.select-dernier"),
                    );
                });
                ui.add_space(COL_GAP);
                ui.vertical(|ui| {
                    ui.set_width(LARGEUR);
                    libelle(ui, "choix multiple, dernière survolée");
                    ui.add(
                        design::select_multi(&mut canaux)
                            .option(0, "Proximité")
                            .option(1, "Groupe")
                            .option(2, "Guilde")
                            .option(3, "Communauté")
                            .summary("Tous les canaux")
                            .width(LARGEUR)
                            .preview_open(true)
                            .preview_hovered(3)
                            .log_name("planche.select-multi"),
                    );
                });
            });
        });

    harness.run();
    let image = harness
        .render()
        .expect("rendu offscreen — voir doc de module");
    let nom = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "select_bords".to_owned());
    let chemin = dossier().join(format!("{nom}.png"));
    image.save(&chemin).expect("écriture de la planche");
    println!("planche écrite dans {}", chemin.display());
}
