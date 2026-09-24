//! **Le carré de contrôle du bandeau « Suivi » face à la hauteur d'un emplacement d'objet** —
//! planche de décision, à l'échelle réelle.
//!
//! Demande du 2026-09-13, dernier ajustement du bandeau : « le carré de boutons est mis au même
//! niveau en haut qu'un item slot, sauf qu'un item slot est plus haut que le carré. J'aimerais
//! donc soit qu'il soit aligné au milieu de manière verticale, soit qu'on augmente la taille des
//! quatre boutons pour que le carré, avec le fond de section en plus, fasse la hauteur d'un item
//! slot. Je te laisse me faire les deux propositions. »
//!
//! Les cotes, qui rendent l'écart mesurable plutôt que ressenti :
//!
//! | Grandeur | Valeur |
//! | --- | --- |
//! | Emplacement d'objet (`tokens::ITEM_SLOT_SIZE`) | **64 px** |
//! | Carré de contrôle (`4 × 3` de marge + `24 × 2` de bouton) | **60 px** |
//! | Écart | **4 px**, tous en bas |
//!
//! Trois planches, une par ligne de l'artefact — la première est l'état d'avant, les deux autres
//! les propositions. **La troisième a été retenue** (`panels::watchlist::CONTROL_BUTTON_SIZE`
//! vaut 26 depuis) ; les deux premières restent ici pour que la décision garde ses termes :
//!
//! 1. `carre_aligne_haut` : les deux hauts sur la même ligne, le carré 4 px trop court.
//! 2. `carre_centre` : le carré descend de 2 px, son centre sur celui des tuiles. Aucune cote de
//!    composant ne bouge — les 24 px des boutons sont un réglage utilisateur du 2026-09-06
//!    (« essaie vingt-quatre sur vingt-quatre pour voir le rendu que ça fait »).
//! 3. `carre_agrandi` : les boutons passent à 26 px, le carré devient un **64 × 64 exact**, de la
//!    taille d'une case d'inventaire du jeu. La grille est alors parfaite, au prix de la cote de
//!    bouton arrêtée à l'essai le 2026-09-06 — que la demande remettait elle-même en jeu.
//!    **C'est l'état d'aujourd'hui** : le centrage de la planche 2 est resté en place et ne
//!    déplace plus rien, l'écart qu'il comblait étant devenu nul.
//!
//! Le trait doré de chaque planche est le centre vertical des tuiles — c'est LA ligne dont parle
//! la demande.
//!
//! Les boutons et les tuiles sont les vrais composants (`design::icon_button`,
//! `design::item_slot`) ; seul l'assemblage est recopié de `panels::watchlist::control_button_row`,
//! dont les jetons sont privés (voir `bandeau-suivi-planches.rs`, même parti pris).
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`.
//!
//! ```text
//! cargo run -p overlay-testkit --example bandeau-carre-hauteur
//! ```

use egui_kittest::Harness;
use overlay_ui::design::{self, tokens};

/// Côté d'un emplacement d'objet — la hauteur à laquelle le carré doit se mesurer.
const TILE: f32 = tokens::ITEM_SLOT_SIZE;
/// Écart entre deux tuiles (`panels::watchlist::TILE_GAP`).
const TILE_GAP: f32 = 12.0;
/// Marge du fond translucide, et écart entre deux boutons
/// (`panels::watchlist::CONTROL_BUTTON_GAP`).
const BUTTON_GAP: f32 = 4.0;
/// Rayon du fond translucide (`panels::watchlist::PANEL_BACKDROP_ROUNDING`).
const BACKDROP_ROUNDING: f32 = 6.0;
/// Tuiles peintes à droite du carré — deux suffisent à lire l'alignement.
const TUILES: usize = 2;
/// Marge de la planche autour du contenu.
const MARGE: f32 = 10.0;

/// Une proposition : la taille de bouton, et si le carré se centre sur les tuiles.
struct Variante {
    slug: &'static str,
    /// Côté d'un bouton du carré (`panels::watchlist::CONTROL_BUTTON_SIZE`, 24 px jusqu'au
    /// 2026-09-13, 26 depuis).
    bouton: f32,
    /// Centrer le carré sur la hauteur d'une tuile, au lieu d'aligner les hauts.
    centre: bool,
}

const VARIANTES: [Variante; 3] = [
    Variante {
        slug: "carre_aligne_haut",
        bouton: 24.0,
        centre: false,
    },
    Variante {
        slug: "carre_centre",
        bouton: 24.0,
        centre: true,
    },
    Variante {
        slug: "carre_agrandi",
        bouton: 26.0,
        centre: false,
    },
];

/// Côté du carré (fond compris) pour une taille de bouton — deux lignes, deux colonnes, et la
/// marge du fond des deux côtés. MÊME formule que `panels::watchlist::control_row_width`.
fn carre(bouton: f32) -> f32 {
    BUTTON_GAP * 3.0 + bouton * 2.0
}

fn dossier() -> std::path::PathBuf {
    let dir = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(target) => std::path::PathBuf::from(target),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"),
    }
    .join("mockups");
    std::fs::create_dir_all(&dir).expect("création de target/mockups");
    dir
}

fn planche(variante: &Variante) {
    let bouton = variante.bouton;
    let centre = variante.centre;
    let cote = carre(bouton);
    let largeur = MARGE * 2.0 + cote + TILE_GAP + TUILES as f32 * (TILE + TILE_GAP) - TILE_GAP;
    let hauteur = MARGE * 2.0 + TILE;

    let mut harness = Harness::builder()
        .with_size(egui::vec2(largeur, hauteur))
        .build_ui(move |ui| {
            let ctx = ui.ctx().clone();
            overlay_ui::style::apply(&ctx);
            let icone = egui::load::SizedTexture::from_handle(
                design::DesignSystem::get(&ctx).icon(design::DsIcon::Kamas),
            );

            let origine = ui.max_rect().min;
            // Le haut des TUILES : c'est la ligne de référence de toute la planche.
            let haut_tuiles = origine.y;

            // Fond du carré, puis ses quatre boutons.
            let haut_carre = if centre {
                haut_tuiles + (TILE - cote) / 2.0
            } else {
                haut_tuiles
            };
            let fond = egui::Rect::from_min_size(
                egui::pos2(origine.x, haut_carre),
                egui::Vec2::splat(cote),
            );
            ui.painter()
                .rect_filled(fond, BACKDROP_ROUNDING, tokens::OVERLAY_BACKDROP);
            let pas = bouton + BUTTON_GAP;
            for (index, glyphe) in [
                design::DsIcon::Plus,
                design::DsIcon::Minus,
                design::DsIcon::ExternalLink,
                design::DsIcon::Option,
            ]
            .into_iter()
            .enumerate()
            {
                let colonne = (index % 2) as f32;
                let ligne = (index / 2) as f32;
                let rect = egui::Rect::from_min_size(
                    fond.min + egui::vec2(BUTTON_GAP + colonne * pas, BUTTON_GAP + ligne * pas),
                    egui::Vec2::splat(bouton),
                );
                ui.put(
                    rect,
                    design::icon_button(glyphe)
                        .context(design::IconContext::FirstPlan)
                        .size(bouton)
                        .log_name("planche.carre"),
                );
            }

            // Les tuiles, à droite, alignées sur `haut_tuiles`.
            for index in 0..TUILES {
                let rect = egui::Rect::from_min_size(
                    egui::pos2(
                        origine.x + cote + TILE_GAP + index as f32 * (TILE + TILE_GAP),
                        haut_tuiles,
                    ),
                    egui::Vec2::splat(TILE),
                );
                ui.put(
                    rect,
                    design::item_slot()
                        .frame(design::SlotFrame::Rarity(design::ItemRarity::Common))
                        .icon(icone)
                        .size(TILE)
                        .count(design::SlotCount::Simple(0))
                        .log_name("planche.slot"),
                );
            }

            // **La ligne dont parle la demande** — le centre vertical des tuiles, tracé par-dessus
            // tout le reste pour qu'on voie du premier coup d'œil qui est dessus et qui ne l'est
            // pas.
            let y = haut_tuiles + TILE / 2.0;
            ui.painter().hline(
                ui.max_rect().x_range(),
                y,
                egui::Stroke::new(1.0, tokens::SCROLLBAR_THUMB_ACTIVE),
            );
        });

    harness.run();
    let image = harness
        .render()
        .expect("rendu offscreen — voir doc de module");
    let chemin = dossier().join(format!("bandeau_{}.png", variante.slug));
    image.save(&chemin).expect("écriture de la planche");
    println!("planche écrite dans {}", chemin.display());
}

fn main() {
    for variante in &VARIANTES {
        planche(variante);
    }
}
