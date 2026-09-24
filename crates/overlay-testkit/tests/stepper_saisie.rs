//! **La saisie au clavier dans `design::stepper`** — demandée le 2026-09-16 (« permettre à
//! l'utilisateur de saisir les nombres manuellement dans l'input de décompte »), là où le champ du
//! pas était en lecture seule depuis sa création.
//!
//! Ce que ce fichier prouve, et que les tests unitaires du composant ne peuvent pas montrer : les
//! règles de `apply_draft`/`commit_draft` sont bien celles que produit un VRAI geste — un clic dans
//! le champ, des touches, un clic ailleurs — avec le brouillon rangé dans la mémoire d'egui entre
//! deux frames. Tout se joue au clic et à la frappe, jamais en posant un état à la main.
//!
//! Le domaine est `1..=500` et non celui du panneau Suivi (`1..=9999`) : il fait tenir dans trois
//! chiffres les trois cas qui comptent — au-dessus du maximum, sous le minimum, et le champ vidé.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use std::cell::RefCell;
use std::rc::Rc;

use egui_kittest::Harness;
use overlay_ui::design;

const MIN: i64 = 1;
const MAX: i64 = 500;

/// Ce que le test pilote et relit : la valeur du pas, et le rectangle qu'il vient d'occuper (les
/// positions de clic en dérivent, plutôt que d'être des constantes à réajuster).
struct Etat {
    valeur: i64,
    pas: egui::Rect,
}

/// Un clic complet : survol, appui, relâchement, chacun dans sa frame — le geste réel (même
/// raison que dans `autocomplete_filtre.rs`).
fn clique(harness: &mut Harness<'_>, pos: egui::Pos2) {
    harness.hover_at(pos);
    harness.run();
    harness.drag_at(pos);
    harness.run();
    harness.drop_at(pos);
    harness.run();
}

/// Frappe `texte` caractère par caractère, après avoir sélectionné tout le contenu du champ :
/// sans ce `Ctrl+A`, la frappe s'insérerait là où le clic a posé le curseur et le test dépendrait
/// de la largeur d'un glyphe.
fn saisit(harness: &mut Harness<'_>, texte: &str) {
    harness.event(egui::Event::Key {
        key: egui::Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    });
    for c in texte.chars() {
        harness.event(egui::Event::Text(c.to_string()));
    }
    harness.run();
}

/// Vide le champ — `Ctrl+A` puis `Retour arrière`.
fn efface(harness: &mut Harness<'_>) {
    harness.event(egui::Event::Key {
        key: egui::Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    });
    harness.event(egui::Event::Key {
        key: egui::Key::Backspace,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
}

#[test]
fn le_champ_du_pas_accepte_la_saisie_et_ecrete_comme_la_doc_le_dit() {
    let etat = Rc::new(RefCell::new(Etat {
        valeur: MIN,
        pas: egui::Rect::ZERO,
    }));

    let vu = Rc::clone(&etat);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 120.0))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            let mut e = vu.borrow_mut();
            let response = ui.add(
                design::stepper(&mut e.valeur)
                    .range(MIN..=MAX)
                    .size(26.0)
                    .field_width(54.0)
                    .log_name("test.pas"),
            );
            e.pas = response.rect;
        });
    harness.run();
    let pas = etat.borrow().pas;
    let champ = pas.center();
    // Loin du pas, dans le vide : de quoi rendre le focus sans toucher à un bouton.
    let ailleurs = egui::pos2(pas.right() + 80.0, pas.center().y);

    // 1. Un entier du domaine : la valeur suit la frappe, sans attendre la validation.
    clique(&mut harness, champ);
    saisit(&mut harness, "42");
    assert_eq!(etat.borrow().valeur, 42, "la saisie doit piloter la valeur");

    // 2. Au-dessus du maximum : écrêté sous les doigts.
    saisit(&mut harness, "999");
    assert_eq!(etat.borrow().valeur, MAX, "au-dessus du maximum : écrêté");

    // 3. Ce qui ne compose pas un entier n'entre pas — la valeur ne bouge pas d'un pouce.
    saisit(&mut harness, "abc");
    assert_eq!(
        etat.borrow().valeur,
        MAX,
        "des lettres ne doivent rien changer"
    );

    // 4. Champ vidé : la valeur RESTE, c'est tout l'intérêt du brouillon — puis elle se retrouve
    //    intacte en sortant du champ.
    efface(&mut harness);
    assert_eq!(
        etat.borrow().valeur,
        MAX,
        "un champ vidé ne doit pas faire tomber la valeur"
    );
    clique(&mut harness, ailleurs);
    assert_eq!(
        etat.borrow().valeur,
        MAX,
        "en sortant, un champ vidé retrouve sa valeur"
    );

    // 5. Sous le minimum : la frappe est acceptée telle quelle, l'écrêtage attend la sortie du
    //    champ (sans quoi on ne pourrait jamais retaper sur un domaine qui commence à 1).
    clique(&mut harness, champ);
    saisit(&mut harness, "0");
    assert_eq!(
        etat.borrow().valeur,
        MAX,
        "sous le minimum, la valeur ne bouge pas tant qu'on tape"
    );
    clique(&mut harness, ailleurs);
    assert_eq!(etat.borrow().valeur, MIN, "en sortant : écrêté au minimum");
}

#[test]
fn un_bouton_l_emporte_sur_la_saisie_en_cours() {
    // Le piège que le brouillon pose s'il survit au clic : on tape « 200 », on clique « + », et la
    // frappe abandonnée revient écraser le 201 à la frame suivante.
    let etat = Rc::new(RefCell::new(Etat {
        valeur: MIN,
        pas: egui::Rect::ZERO,
    }));

    let vu = Rc::clone(&etat);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 120.0))
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            let mut e = vu.borrow_mut();
            let response = ui.add(
                design::stepper(&mut e.valeur)
                    .range(MIN..=MAX)
                    .size(26.0)
                    .field_width(54.0)
                    .log_name("test.pas-boutons"),
            );
            e.pas = response.rect;
        });
    harness.run();
    let pas = etat.borrow().pas;

    clique(&mut harness, pas.center());
    saisit(&mut harness, "200");
    assert_eq!(etat.borrow().valeur, 200);

    // Le bouton « + » est le socle carré de droite.
    let plus = egui::pos2(pas.right() - 13.0, pas.center().y);
    clique(&mut harness, plus);
    assert_eq!(etat.borrow().valeur, 201, "le clic sur « + » incrémente");
    // Deux frames de plus : c'est là que le brouillon oublié se serait rappelé au bon souvenir de
    // la valeur.
    harness.run();
    harness.run();
    assert_eq!(
        etat.borrow().valeur,
        201,
        "la frappe abandonnée ne doit pas revenir"
    );
}
