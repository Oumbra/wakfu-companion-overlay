//! **Le cadre d'un bloc repliable s'arrête à son bloc** — vérification par les pixels, parce
//! qu'aucun test unitaire ne pouvait l'attraper.
//!
//! `design::collapsible` déduisait le bas de son cadre de `ui.min_rect().bottom()`, le rectangle
//! occupé du `Ui` *appelant*. Ce n'est le bas du bloc que si celui-ci est le dernier élément posé —
//! et la galerie, qui fait `ui.set_min_size(ui.available_size())`, le forçait à la hauteur entière
//! de la planche. Tous les cadres couraient donc jusqu'au bas de la page, chacun recouvert par le
//! bloc suivant : aucun n'avait de bord bas, ni liseré, ni motifs d'angle, et deux blocs voisins se
//! touchaient au lieu d'être séparés par leur gouttière.
//!
//! Ce que la galerie ne pouvait pas dire seule : son instantané montre *une* apparence, pas
//! *laquelle est juste*. Ici l'assertion est explicite — entre deux blocs, on doit voir le fond de
//! la page.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`.

use std::cell::Cell;
use std::rc::Rc;

use egui::{Color32, Vec2};
use egui_kittest::Harness;
use overlay_ui::design;

/// Fond de la page de test, choisi très loin de toute couleur du cadre (dont le fond tourne autour
/// de `#1f2227`) : l'assertion doit distinguer « fond de page » de « intérieur de cadre » sans
/// dépendre d'une tolérance fine.
const PAGE_FILL: Color32 = Color32::from_rgb(0xFF, 0x00, 0xFF);

/// Largeur et hauteur de la planche. La hauteur est très supérieure à ce que les deux blocs
/// occupent : c'est elle que le bug étalait sur toute la page.
const SIZE: Vec2 = Vec2::new(400.0, 400.0);

/// Gouttière posée entre les deux blocs par le test.
const GAP: f32 = 10.0;

#[test]
fn le_cadre_s_arrete_au_bloc_et_laisse_voir_la_gouttiere() {
    // Les hauts des deux blocs, relevés pendant le rendu : le `Ui` racine du harnais porte une
    // marge, et une assertion qui supposerait l'origine à zéro tomberait à l'intérieur du premier
    // bloc — elle l'a fait, et accusait le composant.
    let hauts = Rc::new((Cell::new(f32::NAN), Cell::new(f32::NAN)));

    let mut harness = Harness::builder().with_size(SIZE).build_ui({
        let hauts = hauts.clone();
        move |ui| {
            overlay_ui::style::apply(ui.ctx());
            egui::Frame::NONE.fill(PAGE_FILL).show(ui, |ui| {
                // Reproduit le piège de la galerie : le `Ui` occupe toute la planche avant même que
                // le premier bloc ne soit posé.
                ui.set_min_size(ui.available_size());
                ui.spacing_mut().item_spacing.y = 0.0;

                let mut premier = false;
                let r = design::collapsible("Premier", &mut premier).show(ui, |ui| {
                    ui.label("jamais rendu");
                });
                hauts.0.set(r.response.rect.top());

                ui.add_space(GAP);

                let mut second = false;
                let r = design::collapsible("Second", &mut second).show(ui, |ui| {
                    ui.label("jamais rendu");
                });
                hauts.1.set(r.response.rect.top());
            });
        }
    });
    harness.run();
    let image = harness.render().expect("rendu offscreen");
    // Sans l'image, un test pixel qui échoue laisse aveugle — c'est arrivé en écrivant celui-ci.
    if let Ok(dir) = std::env::var("COLLAPSIBLE_DUMP") {
        image.save(format!("{dir}/collapsible.png")).expect("dump");
    }

    let header = design::collapsible_closed_height();
    let (premier, second) = (hauts.0.get(), hauts.1.get());
    // La gouttière demandée doit se retrouver telle quelle entre les deux blocs : c'est elle que le
    // débordement mangeait.
    assert!(
        (second - (premier + header + GAP)).abs() < 0.01,
        "le second bloc est à {second} au lieu de {}",
        premier + header + GAP,
    );

    let ppp = harness.ctx.pixels_per_point();
    let x = (SIZE.x * ppp / 2.0) as u32;
    for (y, ou) in [
        (premier + header + GAP / 2.0, "entre les deux blocs"),
        (second + header + GAP, "sous le second bloc"),
    ] {
        let y = (y * ppp) as u32;
        let p = image.get_pixel(x, y);
        assert_eq!(
            (p[0], p[1], p[2]),
            (PAGE_FILL.r(), PAGE_FILL.g(), PAGE_FILL.b()),
            "à y={y} ({ou}) on lit {p:?} au lieu du fond de page : un cadre déborde de son bloc",
        );
    }
}
