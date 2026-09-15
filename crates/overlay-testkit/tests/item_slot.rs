//! **Le fond d'un emplacement ne déborde pas de son liseré.**
//!
//! Retour utilisateur du 2026-09-13, capture du bandeau de suivi à l'appui : « on a l'impression
//! qu'il y a un fond carré et dessus on a mis une bordure arrondie ». C'était exact. Le fond
//! (`tokens::ITEM_SLOT_BACKGROUND`) était peint sur le carré entier de l'emplacement avec un rayon
//! de 2 px, alors que les textures de rareté posent leur liseré **en retrait de 2 px** avec un arc
//! de 3 px : les quatre coins du fond dépassaient tout autour de la bordure arrondie, dessinant le
//! carré sombre que l'utilisateur voyait.
//!
//! Un test de pixels plutôt qu'une capture comparée : le défaut tient dans quatre pixels de coin,
//! qu'un seuil de snapshot laisserait passer sans broncher.

use egui::Color32;
use egui_kittest::Harness;
use overlay_ui::design;

/// Fond de planche, choisi parce qu'il n'apparaît nulle part dans le design system : tout pixel qui
/// le porte encore est un pixel que l'emplacement n'a pas peint.
const PAGE_FILL: Color32 = Color32::from_rgb(0xFF, 0x00, 0xFF);
const SLOT: f32 = 64.0;
const ORIGINE: egui::Pos2 = egui::pos2(16.0, 16.0);
/// Écart admis par canal sur un coin — voir l'assertion.
const TOLERANCE: i16 = 6;

#[test]
fn le_fond_d_un_emplacement_reste_dans_son_lisere() {
    for (nom, frame) in [
        ("monstre", design::SlotFrame::Plain),
        (
            "objet légendaire",
            design::SlotFrame::Rarity(design::ItemRarity::Legendary),
        ),
    ] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(96.0, 96.0))
            .build_ui(move |ui| {
                overlay_ui::style::apply(ui.ctx());
                egui::Frame::NONE.fill(PAGE_FILL).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    ui.put(
                        egui::Rect::from_min_size(ORIGINE, egui::Vec2::splat(SLOT)),
                        design::item_slot().frame(frame).size(SLOT),
                    );
                });
            });
        harness.run();
        let image = harness.render().expect("rendu offscreen");
        let ppp = harness.ctx.pixels_per_point();

        // Un pixel à l'intérieur du carré de l'emplacement, mais HORS de l'anneau où vit le
        // liseré : c'est exactement la zone que le fond débordait.
        for (dx, dy, coin) in [
            (1.0, 1.0, "haut-gauche"),
            (SLOT - 1.0, 1.0, "haut-droit"),
            (1.0, SLOT - 1.0, "bas-gauche"),
            (SLOT - 1.0, SLOT - 1.0, "bas-droit"),
        ] {
            let x = ((ORIGINE.x + dx) * ppp) as u32;
            let y = ((ORIGINE.y + dy) * ppp) as u32;
            let p = image.get_pixel(x, y);
            // Tolérance de quelques niveaux, et non l'égalité stricte : la texture de rareté pose
            // elle-même un alpha résiduel d'un ou deux niveaux dans ses coins (son propre dégradé),
            // ce qui n'est pas le défaut cherché. Le fond, lui, est OPAQUE — il sautait à
            // `#1E1E1E`, à plus de deux cents niveaux d'ici.
            let ecart = [
                (p[0] as i16 - PAGE_FILL.r() as i16).abs(),
                (p[1] as i16 - PAGE_FILL.g() as i16).abs(),
                (p[2] as i16 - PAGE_FILL.b() as i16).abs(),
            ];
            assert!(
                ecart.iter().all(|e| *e <= TOLERANCE),
                "{nom} : le coin {coin} porte {p:?} au lieu du fond de planche \
                 (écart {ecart:?}) — quelque chose déborde de l'anneau du liseré",
            );
        }
    }
}

/// Le pendant, sans quoi le test ci-dessus passerait tout aussi bien si le fond avait simplement
/// disparu : l'emplacement d'un monstre n'a que celui-là.
#[test]
fn un_emplacement_sans_rarete_garde_son_fond() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(96.0, 96.0))
        .build_ui(|ui| {
            overlay_ui::style::apply(ui.ctx());
            egui::Frame::NONE.fill(PAGE_FILL).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                ui.put(
                    egui::Rect::from_min_size(ORIGINE, egui::Vec2::splat(SLOT)),
                    design::item_slot()
                        .frame(design::SlotFrame::Plain)
                        .size(SLOT),
                );
            });
        });
    harness.run();
    let image = harness.render().expect("rendu offscreen");
    let ppp = harness.ctx.pixels_per_point();
    let centre = ORIGINE + egui::Vec2::splat(SLOT / 2.0);
    let p = image.get_pixel((centre.x * ppp) as u32, (centre.y * ppp) as u32);
    let attendu = design::tokens::ITEM_SLOT_BACKGROUND;
    assert_eq!(
        (p[0], p[1], p[2]),
        (attendu.r(), attendu.g(), attendu.b()),
        "le centre porte {p:?} : l'emplacement d'un monstre a perdu son fond",
    );
}
