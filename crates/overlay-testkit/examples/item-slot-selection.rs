//! **Les deux tons de sélection d'un emplacement d'objet** — planche de contrôle du composant
//! `design::item_slot`, rendue par le composant lui-même.
//!
//! Demande du 2026-09-13 : une sélection multiple ne sert pas toujours à supprimer. Celle de
//! l'onglet Suivi et celle du bandeau, si — mais un écran qui sélectionnerait pour comparer,
//! exporter ou déplacer n'aurait aucune raison d'emprunter le rouge d'une action destructive. Le
//! composant porte donc deux tons, et **rien d'autre ne change entre eux** : même anneau, même
//! case, même retrait, seule la couleur.
//!
//! ## Ce que la planche montre, et pourquoi elle existe à côté de la galerie
//!
//! La galerie du design system (`tests/design_gallery.rs`) couvre déjà ces états, mais noyés dans
//! une page de 8000 px où l'on ne compare rien à l'œil. Ici les deux tons sont **l'un sous
//! l'autre**, aux mêmes colonnes, pour que la seule différence visible soit celle qu'on veut
//! juger. Les deux cadres sont représentés — un objet (bordure de rareté, une texture) et un
//! monstre (cadre simple, un trait peint) — parce que c'est précisément entre ces deux-là que les
//! géométries divergeaient jusqu'au 2026-09-13.
//!
//! ## La case décochée ne prend pas le ton
//!
//! Décision de cette planche, visible à la 2ᵉ colonne : une case vide annonce un **geste possible**,
//! pas un objet retenu. La teindre en rouge dirait « ces huit tuiles vont être supprimées » alors
//! que l'utilisateur n'en a choisi aucune. Le ton porte sur ce qui est retenu, jamais sur ce qui ne
//! l'est pas — d'où `SelectionTone::checkbox_tint(checked)`, qui prend l'état en paramètre.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`.
//!
//! ```text
//! cargo run -p overlay-testkit --example item-slot-selection
//! ```

use egui::{Color32, Vec2};
use egui_kittest::Harness;
use overlay_ui::design::{self, DsIcon, ItemRarity, SelectionTone, SlotCount, SlotFrame};

/// Côté d'un emplacement — celui du jeu, et celui des deux écrans concernés.
const TILE: f32 = design::tokens::ITEM_SLOT_SIZE;
/// Gouttière entre deux colonnes de la planche.
const COL_GAP: f32 = 26.0;
/// Gris des libellés de la planche — celui des légendes de la galerie.
const CAPTION: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);

/// Les cinq états d'une colonne, dans l'ordre de lecture.
///
/// `None` d'abord : c'est l'emplacement ordinaire, la référence à laquelle comparer les quatre
/// autres.
const ETATS: [(Option<bool>, &str); 3] = [
    (None, "hors sélection"),
    (Some(false), "à cocher"),
    (Some(true), "cochée"),
];

fn dossier() -> std::path::PathBuf {
    let dir = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(target) => std::path::PathBuf::from(target),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"),
    }
    .join("mockups");
    std::fs::create_dir_all(&dir).expect("création de target/mockups");
    dir
}

/// Une rangée : un ton, ses trois états, pour un cadre donné.
fn rangee(
    ui: &mut egui::Ui,
    icone: egui::load::SizedTexture,
    frame: SlotFrame,
    ton: SelectionTone,
) {
    ui.horizontal(|ui| {
        for (selection, _) in ETATS {
            let mut slot = design::item_slot()
                .frame(frame)
                .icon(icone)
                .size(TILE)
                .selection(selection)
                .selection_tone(ton)
                .log_name("planche.slot");
            if matches!(frame, SlotFrame::Rarity(_)) {
                slot = slot.count(SlotCount::Target(50));
            }
            ui.add(slot);
            ui.add_space(COL_GAP);
        }
    });
}

fn libelle(ui: &mut egui::Ui, texte: &str, taille: f32) {
    ui.label(egui::RichText::new(texte).color(CAPTION).size(taille));
}

fn main() {
    let largeur = 3.0 * (TILE + COL_GAP) + 40.0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(largeur, 392.0))
        .build_ui(move |ui| {
            let ctx = ui.ctx().clone();
            overlay_ui::style::apply(&ctx);
            // Un glyphe du manifeste en guise d'icône : la planche juge le CADRE et sa sélection,
            // pas l'icône — et le harnais n'a pas de catalogue distant.
            let icone = egui::load::SizedTexture::from_handle(
                design::DesignSystem::get(&ctx).icon(DsIcon::Kamas),
            );

            ui.spacing_mut().item_spacing.y = 8.0;

            // L'en-tête de colonnes, une seule fois : les deux tons partagent exactement la même.
            ui.horizontal(|ui| {
                for (_, nom) in ETATS {
                    ui.allocate_ui(Vec2::new(TILE, 18.0), |ui| libelle(ui, nom, 12.0));
                    ui.add_space(COL_GAP);
                }
            });

            for (ton, titre) in [
                (SelectionTone::Neutral, "Neutral — retenu"),
                (SelectionTone::Danger, "Danger — retenu pour suppression"),
            ] {
                ui.add_space(10.0);
                libelle(ui, titre, 14.0);
                rangee(ui, icone, SlotFrame::Rarity(ItemRarity::Legendary), ton);
                rangee(ui, icone, SlotFrame::Plain, ton);
            }
        });

    harness.run();
    let image = harness
        .render()
        .expect("rendu offscreen — voir doc de module");
    let chemin = dossier().join("item_slot_selection.png");
    image.save(&chemin).expect("écriture de la planche");
    println!("planche écrite dans {}", chemin.display());
}
