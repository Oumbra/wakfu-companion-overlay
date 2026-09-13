//! **Quatre barres de défilement pour la bande du bandeau « Suivi »** — planche de décision,
//! rendue à l'identique de ce que peindrait la vraie fenêtre.
//!
//! Demande du 2026-09-13 : « je voudrais que le scroll ne s'agrandisse pas lorsque l'utilisateur
//! passe sa souris dessus, et qu'il soit repris pour faire un entre-deux — sans la souris dessus
//! il est trop petit, et quand on passe la souris dessus il est trop grand [...] il est à moitié
//! translucide, à moitié machin, ce n'est pas beau. Prépare-moi un artefact pour me proposer des
//! solutions graphiques qui s'intègrent dans la solution graphique du jeu. Peut-être utiliser le
//! scroll qui est déjà utilisé pour la modale dans l'onglet Raccourcis. »
//!
//! La variante A est celle qui est **implémentée** (`panels::watchlist::strip_scroll_area`) : la
//! barre du jeu, exactement celle de la fenêtre Options, couchée à l'horizontale. Les trois autres
//! existent pour qu'il y ait quelque chose à comparer avant de s'y tenir — chacune ne change qu'UNE
//! chose à la fois.
//!
//! Chaque variante est rendue deux fois, au repos et la poignée survolée : c'est entre ces deux
//! images-là que se juge le grief d'origine, une barre qui change de taille sous le pointeur.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`.
//!
//! ```text
//! cargo run -p overlay-testkit --example bandeau-scrollbar-variantes
//! ```

use egui::Color32;
use egui_kittest::Harness;
use overlay_ui::design::{self, tokens};

/// Côté d'une tuile — celui du bandeau (`panels::watchlist::TILE_SIZE`).
const TILE: f32 = tokens::ITEM_SLOT_SIZE;
/// Écart entre deux tuiles (`panels::watchlist::TILE_GAP`).
const TILE_GAP: f32 = 12.0;
/// Assez de tuiles pour que la bande déborde largement — le cas de la capture d'origine.
const TUILES: usize = 12;
/// Largeur de la planche : celle d'une fenêtre Suivi plafonnée en jeu.
const LARGEUR: f32 = 440.0;
/// Hauteur d'une planche — la bande, sa barre, et rien de plus.
const HAUTEUR: f32 = 92.0;

/// Un réglage de barre à comparer. Tout ce qui n'est pas listé ici est identique d'une variante à
/// l'autre : même bande, mêmes tuiles, même position de défilement.
struct Variante {
    /// Nom de fichier (`bandeau_scrollbar_<slug>_<etat>.png`).
    slug: &'static str,
    /// Épaisseur de la poignée.
    epaisseur: f32,
    /// Rayon des coins.
    rayon: u8,
    /// Teinte au repos.
    repos: Color32,
    /// Teinte au survol et pendant le glissé.
    survol: Color32,
    /// Gouttière peinte derrière la poignée, ou `TRANSPARENT` pour n'en peindre aucune — le jeu
    /// n'en a pas (« le fond du panneau tient lieu de gouttière »).
    rail: Color32,
}

/// Les quatre propositions, dans l'ordre de lecture de l'artefact.
const VARIANTES: [Variante; 4] = [
    // A — la barre du jeu, telle quelle. C'est ce qui est en place.
    Variante {
        slug: "a_jeu_6px",
        epaisseur: tokens::SCROLLBAR_WIDTH,
        rayon: tokens::SCROLLBAR_RADIUS,
        repos: tokens::SCROLLBAR_THUMB,
        survol: tokens::SCROLLBAR_THUMB_ACTIVE,
        rail: Color32::TRANSPARENT,
    },
    // B — la même, deux pixels plus épaisse. Plus facile à attraper sur un overlay qu'on vise vite
    // en plein jeu ; s'écarte du relevé de la fenêtre Options.
    Variante {
        slug: "b_jeu_8px",
        epaisseur: 8.0,
        rayon: 4,
        repos: tokens::SCROLLBAR_THUMB,
        survol: tokens::SCROLLBAR_THUMB_ACTIVE,
        rail: Color32::TRANSPARENT,
    },
    // C — la barre du jeu, plus une gouttière. Le bandeau n'a pas de fond de panneau derrière lui :
    // sans rail, rien ne dit jusqu'où la poignée peut aller.
    Variante {
        slug: "c_jeu_6px_rail",
        epaisseur: tokens::SCROLLBAR_WIDTH,
        rayon: tokens::SCROLLBAR_RADIUS,
        repos: tokens::SCROLLBAR_THUMB,
        survol: tokens::SCROLLBAR_THUMB_ACTIVE,
        rail: Color32::from_rgba_premultiplied(0x14, 0x15, 0x17, 0xB4),
    },
    // D — l'autre barre déjà validée dans cet overlay, celle du cadre ennemi et des barres de
    // dégâts (`panels::combat_scrollbar`) : 5 px, kaki uni, teinte identique au repos et au survol.
    Variante {
        slug: "d_combat_5px",
        epaisseur: 5.0,
        rayon: 2,
        repos: Color32::from_rgb(0x99, 0x8a, 0x6c),
        survol: Color32::from_rgb(0x99, 0x8a, 0x6c),
        rail: Color32::TRANSPARENT,
    },
];

/// Marge entre la poignée et le bas de la bande — celle du bandeau
/// (`panels::watchlist::STRIP_SCROLLBAR_OUTER_MARGIN`), pas les 14 px du relevé, qui séparent la
/// poignée du bord d'un panneau que le bandeau n'a pas.
const MARGE_EXTERIEURE: f32 = 2.0;

/// Marge qu'`egui_kittest` prend DANS la taille demandée, sur les quatre côtés — il faut la
/// déduire pour viser la poignée, peinte au bas du contenu et non au bas de l'image.
const MARGE_HARNAIS: f32 = 8.0;

/// Côté d'une case du damier de fond — large, volontairement : le damier serré de la planche de la
/// modale Options (12 px) sert à mesurer une translucidité, celui-ci à juger une barre de 6 px
/// posée dessus. Un fond trop agité la noie.
const CASE: f32 = 44.0;
/// Les deux teintes du damier, de la famille chromatique du jeu et **peu contrastées** — un décor
/// de jeu plausible, pas une mire.
const DAMIER_SOMBRE: Color32 = Color32::from_rgb(0x24, 0x2E, 0x22);
const DAMIER_CLAIR: Color32 = Color32::from_rgb(0x4A, 0x4B, 0x3A);

/// Peint le damier sur toute la surface du `Ui`, avant le moindre widget.
fn peindre_damier(ui: &egui::Ui) {
    let rect = ui.max_rect();
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, DAMIER_SOMBRE);
    let colonnes = (rect.width() / CASE).ceil() as i32;
    let lignes = (rect.height() / CASE).ceil() as i32;
    for ligne in 0..lignes {
        for colonne in 0..colonnes {
            if (ligne + colonne) % 2 == 0 {
                continue;
            }
            let case = egui::Rect::from_min_size(
                rect.min + egui::vec2(colonne as f32 * CASE, ligne as f32 * CASE),
                egui::vec2(CASE, CASE),
            )
            .intersect(rect);
            painter.rect_filled(case, 0.0, DAMIER_CLAIR);
        }
    }
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

/// Rend une variante dans un état donné et écrit son PNG.
///
/// `survole` pose le pointeur sur la poignée — le seul geste qui distingue les deux images d'une
/// même variante.
fn planche(variante: &Variante, survole: bool) {
    let epaisseur = variante.epaisseur;
    let rayon = variante.rayon;
    let repos = variante.repos;
    let survol = variante.survol;
    let rail = variante.rail;

    let mut harness = Harness::builder()
        .with_size(egui::vec2(LARGEUR, HAUTEUR))
        .build_ui(move |ui| {
            let ctx = ui.ctx().clone();
            overlay_ui::style::apply(&ctx);
            let icone = design::DesignSystem::get(&ctx)
                .icon(design::DsIcon::Kamas)
                .id();

            // **Damier de fond**, comme `panels.rs::modale_options_sur_damier_ne_panique_pas` : en
            // production la fenêtre est transparente et laisse voir le jeu. Une planche sur fond
            // noir plein jugerait mal les variantes translucides — un rail sombre y disparaît,
            // alors qu'il se voit sur un décor clair.
            peindre_damier(ui);

            // Le style de barre de CETTE variante — posé à la main plutôt que par
            // `design::scroll_area`, qui ne connaît (volontairement) que les jetons du jeu : une
            // planche de comparaison a le droit de sortir du design system, un panneau non.
            let style = ui.style_mut();
            let scroll = &mut style.spacing.scroll;
            scroll.floating = false;
            scroll.bar_width = epaisseur;
            scroll.bar_inner_margin = tokens::SCROLLBAR_CONTENT_MARGIN;
            scroll.bar_outer_margin = MARGE_EXTERIEURE;
            scroll.foreground_color = false;
            style.always_scroll_the_only_direction = true;
            style.spacing.item_spacing.x = 0.0;

            let visuals = ui.visuals_mut();
            visuals.extreme_bg_color = rail;
            let corner = egui::CornerRadius::same(rayon);
            for (widget, fill) in [
                (&mut visuals.widgets.noninteractive, repos),
                (&mut visuals.widgets.inactive, repos),
                (&mut visuals.widgets.hovered, survol),
                (&mut visuals.widgets.active, survol),
            ] {
                widget.bg_fill = fill;
                widget.corner_radius = corner;
            }

            egui::ScrollArea::horizontal()
                .id_salt("variante")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for i in 0..TUILES {
                            if i > 0 {
                                ui.add_space(TILE_GAP);
                            }
                            ui.add(
                                design::item_slot()
                                    .frame(design::SlotFrame::Rarity(design::ItemRarity::Common))
                                    .icon(icone)
                                    .size(TILE)
                                    .count(design::SlotCount::Simple(0))
                                    .log_name("planche.slot"),
                            );
                        }
                    });
                });
        });

    harness.run();
    if survole {
        // Sur la poignée : elle part du bord gauche de la bande, et la planche ne défile pas.
        // L'ordonnée est celle du bas du CONTENU (la taille demandée moins la marge que le harnais
        // prend DEDANS), moins la marge extérieure et la demi-épaisseur.
        let y = HAUTEUR - MARGE_HARNAIS - MARGE_EXTERIEURE - epaisseur / 2.0;
        harness.hover_at(egui::pos2(90.0, y));
        harness.run();
    }

    let image = harness
        .render()
        .expect("rendu offscreen — voir doc de module");
    let etat = if survole { "survol" } else { "repos" };
    let chemin = dossier().join(format!("bandeau_scrollbar_{}_{etat}.png", variante.slug));
    image.save(&chemin).expect("écriture de la planche");
    println!("planche écrite dans {}", chemin.display());
}

fn main() {
    for variante in &VARIANTES {
        planche(variante, false);
        planche(variante, true);
    }
}
