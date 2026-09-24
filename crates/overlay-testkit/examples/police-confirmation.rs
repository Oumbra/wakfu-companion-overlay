//! **Choix de la police de la question d'une boîte de confirmation** — retour utilisateur du
//! 2026-09-15 : « l'écriture est un peu trop grasse, peut-être un poil petite ; il faudrait qu'elle
//! soit plus light et un peu plus grande ».
//!
//! Le composant écrivait sa question en `text::label_font` (Ubuntu **Regular**) au corps **15**, en
//! **blanc pur**. Les trois valeurs venaient du rendu à la main d'avant le détourage, aucune n'avait
//! été mesurée sur la capture du jeu.
//!
//! Or la capture se mesure, et elle donne deux points de contrôle indiscutables, parce qu'ils ne
//! dépendent d'aucune convention de conversion encre → corps :
//!
//! | Échantillon du jeu | Chasse | Hauteur d'encre |
//! | --- | --- | --- |
//! | « Êtes-vous sûr(e) de vouloir supprimer ce » | 325 px | 21 px (accent de Ê aux jambages) |
//! | « build ? » | 51 px | 13 px (hampe à ligne de base) |
//!
//! Cet outil rend les deux mêmes chaînes dans chaque police candidate, **sur le fond réel du corps
//! et dans la couleur mesurée de l'encre du jeu**, pour que l'antialiasing se compare à armes
//! égales — un texte blanc pur et un texte gris clair ne s'érodent pas au même seuil, et la
//! comparaison porterait alors sur l'exposition et non sur la lettre.
//!
//! ```bash
//! cargo run -p overlay-testkit --example police-confirmation
//! # écrit target/police-confirmation/<famille>-<corps>.png
//! ```
//!
//! Les captures partent dans `target/`, jamais au dépôt (§17.4 du plan) : c'est un outil de
//! décision, pas un rendu de référence.

use egui::{Align2, Color32, FontFamily, FontId, Vec2};
use egui_kittest::Harness;

/// Les deux lignes du jeu, telles qu'il les coupe. La seconde est la mesure décisive : ni accent ni
/// jambage, une hampe franche et une chasse courte, donc peu de bruit de seuillage.
const LIGNE_1: &str = "Êtes-vous sûr(e) de vouloir supprimer ce";
const LIGNE_2: &str = "build ?";

/// Fond du corps de la boîte, mesuré sur `interface-confirm-box-cutout.png`.
const FOND: Color32 = Color32::from_rgb(0x59, 0x5A, 0x58);

/// Couleur de l'encre du jeu, mesurée au cœur des lettres (érosion 2×2, 463 px) : **gris neutre**,
/// R 208,5 G 208,8 B 208,4. Ni blanc — le composant demandait 255 — ni jaune.
const ENCRE: Color32 = Color32::from_rgb(0xD0, 0xD1, 0xD0);

/// Familles candidates. `Proportional` est la proportionnelle par défaut d'`egui`, c'est-à-dire
/// **Ubuntu Light** : `epaint_default_fonts` n'en embarque qu'une (voir `design::fonts`). Elle est
/// donc disponible sans ajouter de fichier au dépôt — si elle est retenue, le composant devra
/// néanmoins passer par une famille nommée, pour ne pas dépendre de ce qu'`egui` choisit par défaut.
fn familles() -> Vec<(&'static str, FontFamily)> {
    vec![
        ("light", FontFamily::Proportional),
        (
            "regular",
            FontFamily::Name(overlay_ui::design::fonts::LABEL.into()),
        ),
        (
            "medium",
            FontFamily::Name(overlay_ui::design::fonts::LABEL_STRONG.into()),
        ),
    ]
}

/// Corps balayés. Le composant est à 15 ; le retour demande « un peu plus grande », et la hauteur
/// d'encre du jeu (13 px de hampe) situe la cible plus haut — d'où un balayage qui déborde des deux
/// côtés plutôt qu'une valeur devinée.
const CORPS: [f32; 7] = [14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0];

/// Taille de la planche d'un candidat — la largeur du corps de la boîte, deux lignes de haut.
const TAILLE: Vec2 = Vec2::new(420.0, 90.0);

/// Ordonnées des deux lignes, au centre de leur boîte d'encre.
const Y1: f32 = 26.0;
const Y2: f32 = 64.0;

fn sortie() -> std::path::PathBuf {
    let dir = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(target) => std::path::PathBuf::from(target),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"),
    }
    .join("police-confirmation");
    std::fs::create_dir_all(&dir).expect("création de target/police-confirmation");
    dir
}

fn rendu(famille: FontFamily, corps: f32) -> image::RgbaImage {
    let mut harness = Harness::builder().with_size(TAILLE).build_ui(move |ui| {
        overlay_ui::style::apply(ui.ctx());
        let painter = ui.painter();
        painter.rect_filled(
            egui::Rect::from_min_size(egui::Pos2::ZERO, TAILLE),
            0.0,
            FOND,
        );
        // `style::apply` n'a d'effet qu'à la passe suivante (`Context::set_fonts`) : tant que la
        // famille n'est pas liée, peindre dedans ferait paniquer `epaint`. On retombe donc sur la
        // proportionnelle par défaut pour cette seule première passe, exactement comme le fait
        // `design::text::famille` — la capture est prise après stabilisation.
        let liee = ui.ctx().fonts(|fonts| fonts.families().contains(&famille));
        let font = if liee {
            FontId::new(corps, famille.clone())
        } else {
            FontId::proportional(corps)
        };
        for (texte, y) in [(LIGNE_1, Y1), (LIGNE_2, Y2)] {
            painter.text(
                egui::pos2(TAILLE.x / 2.0, y),
                Align2::CENTER_CENTER,
                texte,
                font.clone(),
                ENCRE,
            );
        }
    });
    harness.run();
    harness.render().expect("rendu offscreen")
}

fn main() {
    let dir = sortie();
    for (nom, famille) in familles() {
        for corps in CORPS {
            let image = rendu(famille.clone(), corps);
            let chemin = dir.join(format!("{nom}-{:02}.png", corps as u32));
            image.save(&chemin).expect("écriture de la capture");
            println!("écrit {}", chemin.display());
        }
    }
}
