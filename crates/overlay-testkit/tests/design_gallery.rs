//! **Galerie du design system** — la planche de contrôle de `overlay_ui::design`, rendue offscreen
//! (§17.1 du plan) et comparée pixel à pixel à chaque exécution.
//!
//! Elle sert trois choses à la fois, et c'est voulu :
//!
//! 1. **Vérification visuelle** : chaque variante et chaque état d'un composant sur une seule
//!    image, publiable en Artifact (obligation CLAUDE.md — l'utilisateur travaille en terminal et
//!    ne voit pas une image lue en ligne).
//! 2. **Non-régression** : toucher au 9-slice, aux jetons ou à un asset fait bouger cette capture.
//! 3. **Documentation exécutable** : le code ci-dessous est aussi l'exemple d'usage de référence
//!    des composants — s'il devient laid à écrire, c'est l'API du composant qu'il faut revoir.
//!
//! **Tout composant ajouté à `overlay_ui::design::components` s'ajoute ici**, c'est la clause 7 du
//! contrat de composant (voir `design::components`). Les états sont posés par
//! `Button::preview_state` : aucun pointeur ne survole quoi que ce soit dans un rendu offscreen,
//! c'est le seul moyen de montrer repos / survolé / désactivé côte à côte.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use egui::{Color32, RichText, Vec2};
use egui_kittest::Harness;
use overlay_ui::design::{
    self, ButtonSize, ButtonState, ButtonVariant, CheckboxState, InfoTone, InputState, TabState,
};

/// Fond de la planche — `neutrals.panel_fill` (`docs/design-tokens.json`), le fond de panneau du
/// jeu : les textures de bouton sont détourées, elles doivent être jugées sur le fond sur lequel
/// elles vivront réellement, pas sur du transparent.
const PAGE_FILL: Color32 = Color32::from_rgb(0x18, 0x18, 0x20);
const HEADING: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9F);
const CAPTION: Color32 = Color32::from_rgb(0x9A, 0xA0, 0xA6);

fn heading(ui: &mut egui::Ui, text: &str, caption: &str) {
    ui.add_space(14.0);
    ui.label(RichText::new(text).color(HEADING).size(15.0).strong());
    ui.label(RichText::new(caption).color(CAPTION).size(12.0));
    ui.add_space(6.0);
}

#[test]
fn galerie_du_design_system() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(760.0, 1880.0))
        .build_ui(|ui| {
            overlay_ui::style::apply(ui.ctx());
            egui::Frame::NONE
                .fill(PAGE_FILL)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    gallery(ui);
                });
        });

    harness.run();
    harness.snapshot("design_gallery");
}

fn gallery(ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);

    heading(
        ui,
        "Variantes × états — gabarit Standard (52 px)",
        "Une intention par variante ; l'état « pressé » n'existe pas : l'appui retire l'apparence survolée.",
    );
    for (variant, label) in [
        (ButtonVariant::Primary, "Valider"),
        (ButtonVariant::Secondary, "Parcourir"),
        (ButtonVariant::Danger, "Annuler"),
    ] {
        ui.horizontal(|ui| {
            for state in [
                ButtonState::Idle,
                ButtonState::Hovered,
                ButtonState::Disabled,
            ] {
                ui.add(
                    design::button(label)
                        .variant(variant)
                        .width(200.0)
                        .enabled(state != ButtonState::Disabled)
                        .preview_state(state),
                );
            }
        });
    }

    heading(
        ui,
        "Toutes les tailles, aucun asset dédié",
        "Largeur libre : embouts figés, centre étiré. La HAUTEUR, elle, choisit la texture — 32/36/40 px prennent la capture 338×36 (embout 36 px), 48/52 px la capture 200×52 (embout 52 px).",
    );
    for (w, h) in [
        (110.0, 32.0),
        (160.0, 40.0),
        (200.0, 52.0),
        (338.0, 36.0),
        (520.0, 48.0),
    ] {
        ui.add(
            design::button("Valider")
                .variant(ButtonVariant::Primary)
                .size(ButtonSize::Height(h))
                .width(w),
        );
    }

    heading(
        ui,
        "Cas réel — pied de page de la modale Options",
        "Ce que remplacent large-button-cancel.png et large-button-validate.png : mêmes 338×36, sans asset dédié ni libellé incrusté — et des embouts de même largeur.",
    );
    ui.add(
        design::button("Annuler")
            .variant(ButtonVariant::Danger)
            .size(ButtonSize::Compact)
            .width(338.0)
            .log_name("options.annuler"),
    );
    ui.add(
        design::button("Valider")
            .variant(ButtonVariant::Primary)
            .size(ButtonSize::Compact)
            .width(338.0)
            .log_name("options.valider"),
    );

    heading(
        ui,
        "Largeur automatique et garde-fous",
        "Sans largeur imposée : libellé + marges, avec un plancher de proportion. Dernier cas : trop étroit — peint quand même, et journalisé une fois.",
    );
    ui.horizontal(|ui| {
        ui.add(
            design::button("OK")
                .variant(ButtonVariant::Secondary)
                .size(ButtonSize::Compact),
        );
        ui.add(
            design::button("Enregistrer les modifications")
                .variant(ButtonVariant::Secondary)
                .size(ButtonSize::Compact),
        );
        ui.add(
            design::button("Libellé beaucoup trop long")
                .variant(ButtonVariant::Danger)
                .size(ButtonSize::Compact)
                .width(120.0),
        );
    });
    heading(
        ui,
        "Champ de saisie — hauteur native 25 px",
        "Valeur en or, texte indicatif en kaki éteint : les deux couleurs sont relevées, pas choisies. Le survol ne change rien, faute de capture de référence.",
    );
    // Des valeurs distinctes par champ : un `&mut String` partagé ferait de la planche une seule
    // et même donnée affichée quatre fois, ce qui ne montrerait rien.
    let mut rempli = String::from("/home/joueur/.config/zaap/wakfu.log");
    let mut vide = String::new();
    let mut court = String::from("42");
    let mut desactive = String::from("valeur figée");
    ui.add(
        design::input(&mut rempli)
            .placeholder("Chemin vers wakfu.log")
            .width(420.0)
            .log_name("galerie.rempli"),
    );
    ui.add(
        design::input(&mut vide)
            .placeholder("Chemin vers wakfu.log")
            .width(420.0)
            .log_name("galerie.vide"),
    );
    ui.horizontal(|ui| {
        ui.add(
            design::input(&mut court)
                .placeholder("Nombre")
                .width(90.0)
                .log_name("galerie.court"),
        );
        ui.add(
            design::input(&mut desactive)
                .width(200.0)
                .enabled(false)
                .preview_state(InputState::Disabled)
                .log_name("galerie.desactive"),
        );
    });
    // Le couple réel de la modale Options : un champ à sa hauteur native, centré sur une ligne que
    // le bouton fixe à 36 px.
    ui.horizontal(|ui| {
        ui.add(
            design::input(&mut vide)
                .placeholder("Chemin vers wakfu.log")
                .width(300.0)
                .log_name("galerie.couple"),
        );
        ui.add(
            design::button("Parcourir")
                .variant(ButtonVariant::Secondary)
                .size(ButtonSize::Compact),
        );
    });

    heading(
        ui,
        "Texte d'information — pastille 12 px, interligne 23 px",
        "Une ligne et deux lignes, dans les deux tons. La pastille se centre sur la PREMIÈRE ligne, jamais sur le bloc ; le retour à la ligne s'aligne sur le texte, pas sous la pastille.",
    );
    // Largeur imposée à 420 px pour que le second message repasse à la ligne de façon
    // reproductible : sans elle, le bloc prendrait la largeur de la planche et le cas « deux
    // lignes » — celui que le relevé mesure — ne serait pas couvert.
    ui.add(
        design::info_text("Le thème sera appliqué au prochain démarrage.")
            .width(420.0)
            .log_name("galerie.info-1"),
    );
    ui.add(
        design::info_text(
            "Le fichier sélectionné doit s'appeler wakfu.log : c'est le seul journal que le \
             client écrive en continu.",
        )
        .width(420.0)
        .log_name("galerie.info-2"),
    );
    ui.add(
        design::info_text("Aucun fichier sélectionné.")
            .tone(InfoTone::Alert)
            .width(420.0)
            .log_name("galerie.alerte-1"),
    );
    ui.add(
        design::info_text(
            "Le fichier sélectionné doit s'appeler wakfu.log. Choisissez-en un autre, ou \
             corrigez le chemin à la main.",
        )
        .tone(InfoTone::Alert)
        .width(420.0)
        .log_name("galerie.alerte-2"),
    );

    heading(
        ui,
        "Barre d'onglets — hauteur native 44 px, quatre états",
        "Le survolé a EXACTEMENT le fond de l'actif : seul le libellé les distingue (blanc pour l'actif, doré pour les autres). Un portage qui ne jouerait que sur le fond les rendrait indiscernables.",
    );
    // Une valeur de sélection par barre : un `&mut` partagé afficherait quatre fois le même état.
    let mut onglet_demo = 1_u8;
    design::tabs(&mut onglet_demo)
        .entry(0, "Inactif")
        .entry(1, "Actif")
        .entry(2, "Survolé")
        .preview_state(TabState::Hovered)
        .entry(3, "Désactivé")
        .enabled(false)
        .log_name("galerie.onglets")
        .show(ui);

    // Le cas réel de la modale Options : trois entrées, deux encore désactivées.
    let mut onglet_options = 2_u8;
    design::tabs(&mut onglet_options)
        .entry(0, "Alertes")
        .enabled(false)
        .entry(1, "Personnages")
        .enabled(false)
        .entry(2, "Paramètres")
        .log_name("galerie.onglets-options")
        .show(ui);

    // Les six onglets du jeu, pour comparer les largeurs à la barre réelle.
    let mut onglet_jeu = 1_u8;
    design::tabs(&mut onglet_jeu)
        .entry(0, "Jeu")
        .entry(1, "Vidéo")
        .entry(2, "Interface")
        .preview_state(TabState::Hovered)
        .entry(3, "Son")
        .entry(4, "Commandes")
        .entry(5, "Chat")
        .log_name("galerie.onglets-jeu")
        .show(ui);

    heading(
        ui,
        "Case à cocher — 20 px, rayon 0, le seul élément carré",
        "Le LIBELLÉ porte l'état autant que la case : blanc décoché, doré coché. Le jeu double toujours son signal ; ne changer que la case perdrait la moitié de l'information.",
    );
    // Une valeur par case : un `&mut bool` partagé afficherait quatre fois le même état.
    let (mut coche, mut decoche) = (true, false);
    let (mut survole, mut desactive) = (false, true);
    ui.horizontal(|ui| {
        ui.add(design::checkbox(&mut coche, "Cochée").log_name("galerie.cb-on"));
        ui.add_space(24.0);
        ui.add(design::checkbox(&mut decoche, "Décochée").log_name("galerie.cb-off"));
    });
    ui.horizontal(|ui| {
        ui.add(
            design::checkbox(&mut survole, "Survolée")
                .preview_state(CheckboxState::Hovered)
                .log_name("galerie.cb-hover"),
        );
        ui.add_space(24.0);
        ui.add(
            design::checkbox(&mut desactive, "Désactivée")
                .enabled(false)
                .preview_state(CheckboxState::Disabled)
                .log_name("galerie.cb-off-disabled"),
        );
    });
    // Une ligne réelle du jeu, au libellé complet — le cas qui montre le corps de 15 px.
    let mut interactions = false;
    ui.add(
        design::checkbox(
            &mut interactions,
            "Autoriser les interactions au clic gauche",
        )
        .log_name("galerie.cb-reference"),
    );

    heading(
        ui,
        "Le cas de référence, aux cotes exactes du jeu",
        "Le message et la largeur du bloc d'information de l'onglet Interface (590 px, x 38..628) : à comparer directement avec interface-options-interface.png, y 436..477.",
    );
    ui.add(
        design::info_text(
            "Il peut être nécessaire de relancer le jeu après avoir rechargé le thème pour que \
             toutes les textures soient correctement chargées.",
        )
        .width(590.0)
        .log_name("galerie.info-reference"),
    );
}
