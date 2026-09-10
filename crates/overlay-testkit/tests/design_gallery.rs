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
    self, ButtonSize, ButtonState, ButtonVariant, CheckboxState, DsTexture, IconButtonState,
    IconContext, InfoTone, InputState, SelectState, TabState,
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
        .with_size(Vec2::new(760.0, 2700.0))
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
        "Barre d'onglets — 44 px, pleine largeur à parts égales, quatre états",
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

    // Les six onglets du jeu, en `fit_content()` : c'est LA barre à comparer à
    // `interface-options-video.png`, donc la seule qui doit garder les largeurs relevées plutôt que
    // de se partager la planche à égalité. Les deux barres au-dessus montrent le défaut.
    let mut onglet_jeu = 1_u8;
    design::tabs(&mut onglet_jeu)
        .fit_content()
        .entry(0, "Jeu")
        .entry(1, "Vidéo")
        .entry(2, "Interface")
        .preview_state(TabState::Hovered)
        .entry(3, "Son")
        .entry(4, "Commandes")
        .entry(5, "Chat")
        .log_name("galerie.onglets-jeu")
        .show(ui);

    // Barre à UN SEUL onglet : le seul cas où les quatre coins sont arrondis, et le seul que le jeu
    // n'ait jamais produit. Le composant y superpose ses deux textures d'extrémité, chacune écrêtée
    // à sa moitié — ce chemin de peinture n'a pas d'autre couverture.
    let mut onglet_seul = 0_u8;
    design::tabs(&mut onglet_seul)
        .fit_content()
        .entry(0, "Seul")
        .log_name("galerie.onglet-seul")
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
        "Liste déroulante — socle 36 px, entrées 28 px",
        "Le socle fait 36 px BORD COMPRIS : les 32 px qu'on lit en mesurant le remplissage excluent les deux bords. Le chevron est peint à 14 × 8, sa taille native.",
    );
    let mut theme = 1_u8;
    let mut theme_desactive = 0_u8;
    ui.horizontal(|ui| {
        ui.add(
            design::select(&mut theme)
                .option(0, "Sombre")
                .option(1, "Thème d'interface personnalisé")
                .width(300.0)
                .log_name("galerie.select"),
        );
        ui.add_space(20.0);
        ui.add(
            design::select(&mut theme_desactive)
                .option(0, "Indisponible")
                .width(200.0)
                .enabled(false)
                .preview_state(SelectState::Disabled)
                .log_name("galerie.select-off"),
        );
    });

    // Les deux états dépliés. `preview_open` les force : en rendu offscreen, personne ne clique sur
    // le socle. Ils sont posés en DERNIER sur la planche parce que leur liste sort du flux — elle
    // recouvrirait ce qui suit, ce qui est exactement le comportement voulu en production.
    let mut filtre = 1_u8;
    let mut raretes: Vec<u8> = vec![1, 3];
    ui.add_space(10.0);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(230.0);
            ui.add(
                design::select(&mut filtre)
                    .option(0, "Tous")
                    .option(1, "À faire")
                    .option(2, "Terminées")
                    .option(3, "Masquées")
                    .width(220.0)
                    .preview_open(true)
                    .log_name("galerie.select-ouvert"),
            );
        });
        ui.add_space(30.0);
        ui.vertical(|ui| {
            ui.set_width(230.0);
            ui.add(
                design::select_multi(&mut raretes)
                    .option(0, "Commun")
                    .option(1, "Rare")
                    .option(2, "Mythique")
                    .option(3, "Légendaire")
                    .summary("Toutes")
                    .width(220.0)
                    .preview_open(true)
                    .preview_hovered(0)
                    .log_name("galerie.select-multi"),
            );
        });
    });
    // La place que les deux listes occupent hors du flux, pour que la planche ne les tronque pas.
    ui.add_space(4.0 * 28.0 + 10.0);

    heading(
        ui,
        "Barre de défilement — poignée 6 px, aucun rail",
        "Le fond du panneau tient lieu de gouttière : la barre ne peint qu'une poignée. Réserve totale à droite : 26 px (6 + 6 + 14), la même que le jeu garde même quand la barre ne sert pas.",
    );
    // Un cadre au fond du panneau, pour juger la poignée sur la surface où elle vivra vraiment.
    egui::Frame::NONE
        .fill(Color32::from_rgb(0x15, 0x18, 0x1C))
        .show(ui, |ui| {
            ui.set_width(360.0);
            ui.set_height(120.0);
            design::scroll_area("galerie.scroll").show(ui, |ui| {
                for i in 1..=12 {
                    ui.label(
                        RichText::new(format!("Ligne de contenu n° {i}"))
                            .color(Color32::WHITE)
                            .size(15.0),
                    );
                }
            });
        });

    heading(
        ui,
        "Bouton icône — socle 36 px, deux contextes",
        "L'appelant nomme une intention (FirstPlan ou Panel, IconOption ou IconPlus) : les cinq socles et les glyphes sont résolus par le manifeste, plus par lui. Les icônes sont blanches dans leurs fichiers et prennent leur couleur par teinte.",
    );
    for context in [IconContext::FirstPlan, IconContext::Panel] {
        ui.horizontal(|ui| {
            for (icon, name) in [
                (DsTexture::IconOption, "option"),
                (DsTexture::IconExternalLink, "lien"),
                (DsTexture::IconPlus, "plus"),
                (DsTexture::IconMinus, "moins"),
            ] {
                ui.add(
                    design::icon_button(icon)
                        .context(context)
                        .log_name(format!("galerie.{name}")),
                );
            }
            ui.add_space(20.0);
            ui.add(
                design::icon_button(DsTexture::IconOption)
                    .context(context)
                    .preview_state(IconButtonState::Hovered)
                    .log_name("galerie.icone-survol"),
            );
            ui.add(
                design::icon_button(DsTexture::IconOption)
                    .context(context)
                    .enabled(false)
                    .preview_state(IconButtonState::Disabled)
                    .log_name("galerie.icone-off"),
            );
        });
    }

    // Le champ à ornement — la loupe est posée par le COMPOSANT, dans son clip, et la gouttière
    // qu'elle impose vaut pour le texte indicatif comme pour la valeur saisie. C'est ce que
    // vérifie la deuxième ligne, la seule qui porte une valeur.
    let mut recherche_vide = String::new();
    let mut recherche_pleine = String::from("pierre");
    ui.add(
        design::input(&mut recherche_vide)
            .leading_icon(DsTexture::IconSearch)
            .placeholder("Rechercher")
            .width(420.0)
            .log_name("galerie.recherche-vide"),
    );
    ui.add(
        design::input(&mut recherche_pleine)
            .leading_icon(DsTexture::IconSearch)
            .width(420.0)
            .log_name("galerie.recherche-pleine"),
    );
    ui.add(
        design::input(&mut recherche_pleine)
            .leading_icon(DsTexture::IconSearch)
            .width(200.0)
            .enabled(false)
            .preview_state(InputState::Disabled)
            .log_name("galerie.recherche-off"),
    );

    heading(
        ui,
        "Glyphes du manifeste — teintés, jamais recolorés en amont",
        "Tous les glyphes déclarés, à leur taille de fichier. Ils sont blancs dans leurs octets et prennent leur couleur par teinte : c'est ce qui permet à une même icône de servir au repos, au survol et désactivée sans second fichier. Ceux qui vivent sur un socle sont normalisés à 18 px par le manifeste (ligne du dessus) ; ceux qui vivent dans un champ ou à côté d'un libellé gardent leur taille propre.",
    );
    ui.horizontal(|ui| {
        for icon in [
            DsTexture::IconOption,
            DsTexture::IconExternalLink,
            DsTexture::IconPlus,
            DsTexture::IconMinus,
            DsTexture::IconClose,
            DsTexture::IconDelete,
            DsTexture::IconHelp,
            DsTexture::IconUndo,
        ] {
            ui.add(design::icon_button(icon).context(IconContext::Panel));
        }
    });
    // Les glyphes SANS socle — leur taille est celle que leur donne le composant qui les porte,
    // d'où l'absence de `icon_content_size` (voir sa doc dans `design::assets`).
    let ds = design::DesignSystem::get(ui.ctx());
    let (row, _) = ui.allocate_exact_size(Vec2::new(400.0, 28.0), egui::Sense::hover());
    let mut x = row.left() + 8.0;
    for (icon, tint) in [
        (DsTexture::IconSearch, design::tokens::INPUT_PLACEHOLDER),
        (DsTexture::IconTick, Color32::from_rgb(0x7A, 0xC7, 0x4F)),
        (DsTexture::IconChevronDown, design::tokens::SELECT_TEXT),
        (DsTexture::IconInfo, design::tokens::INFO_DOT),
    ] {
        let native = design::DesignSystem::get(ui.ctx()).native_size(icon);
        ds.paint(
            ui.painter(),
            egui::Rect::from_center_size(egui::pos2(x + native.x / 2.0, row.center().y), native),
            icon,
            tint,
        );
        x += native.x + 22.0;
    }

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
