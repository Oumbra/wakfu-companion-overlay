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

#[path = "../examples/shared/wakassets_fixtures.rs"]
mod wakassets_fixtures;

use egui::{Color32, RichText, Vec2};
use egui_kittest::Harness;
use overlay_ui::design::{
    self, ButtonSize, ButtonState, ButtonVariant, CheckboxState, DsIcon, IconButtonState,
    IconContext, InfoTone, InputSize, InputState, LoaderSize, PaginationStep, SelectState,
    SliderState, TabState, TableAlign, TableBody, TableColumn,
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
        // 4160 -> 4500 le 2026-09-11 : le lot d'icônes sans appelant fait passer les rangées
        // « Bouton icône » et « Glyphes du manifeste » sur deux lignes (`horizontal_wrapped`,
        // la largeur fixe ne les contient plus sur une seule) et ajoute la section « Glyphes
        // sans socle connu » — sans cette hauteur, le bas de la galerie sortait du canevas.
        // 4500 -> 4900 le 2026-09-11 : section « Rouage de chargement » (une rangée de quatre
        // tailles, une rangée d'images figées, un cas hors intervalle).
        // 4900 -> 5570 le 2026-09-11 : section « Filet de séparation » (deux blocs repliables
        // portant le motif intitulé/filet/corps, plus deux filets nus) — 661 px de contenu de
        // plus, et les ~75 px de marge basse que le réglage précédent gardait déjà.
        // 5570 -> 5680 le 2026-09-11 : état d'erreur d'`input` (champ refusé, son message, et le
        // cas « désactivé ET en erreur » où `Disabled` l'emporte).
        // 5680 -> 5860 le 2026-09-11 : section « Curseur de réglage » (trois positions, le motif à
        // libellés, le désactivé et le cas dégénéré).
        // 5860 -> 5900 le 2026-09-11 : rangée des curseurs gradués (26, 11 et 3 crans).
        // 5900 -> 5990 le 2026-09-11 : section « Glyphe seul » (`design::icon`, quatre tailles,
        // trois glyphes non carrés, deux teintes).
        // 5990 -> 6225 le 2026-09-11 : section « Onglets à pictogramme » (une barre aux 66 px du
        // jeu, une barre étirée).
        // 6225 -> 6885 le 2026-09-11 : section « Autocomplétion » (cinq cas, dont trois dépliés
        // dont le panneau est peint hors flux et demande donc sa réserve explicite).
        // 6885 -> 7180 le 2026-09-11 : section « Emplacement d'objet » (les sept raretés, le cadre
        // simple, les deux formes de compteur).
        // 7180 -> 7400 le 2026-09-11 : section « Jauge » (cinq taux, quatre teintes, un cas
        // dégénéré).
        // 7400 -> 7530 le 2026-09-11 : section « Portrait » (deux formes, le grisé, le
        // pourcentage).
        // PLAFOND ATTEINT le 2026-09-12 : wgpu refuse une texture de plus de 8192 px de côté
        // (« Dimension Y value 9110 exceeds the limit of 8192 »). Il reste 662 px ici. Toute
        // section qui ne tient pas dedans prend sa propre planche — voir `galerie_du_tableau`.
        .with_size(Vec2::new(760.0, 7990.0))
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
    // Le motif complet d'une valeur refusée : le bord du champ et le message portent le MÊME rouge
    // — un seul signal, en deux endroits. Le champ reste éditable, c'est tout l'objet de l'état.
    {
        let mut fautif = String::from("C:/Wakfu/introuvable.log");
        ui.add(
            design::input(&mut fautif)
                .width(420.0)
                .error(true)
                .log_name("galerie.errone"),
        );
        ui.add(
            design::info_text("Fichier introuvable à ce chemin.")
                .tone(InfoTone::Alert)
                .width(420.0)
                .log_name("galerie.errone-message"),
        );
        // Désactivé ET en erreur : `Disabled` l'emporte, le bord reste gris. On ne corrige pas ce
        // qu'on ne peut pas éditer.
        let mut inerte = String::from("les deux à la fois");
        ui.add(
            design::input(&mut inerte)
                .width(420.0)
                .error(true)
                .enabled(false)
                .log_name("galerie.errone-desactive"),
        );
    }
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
        "Onglets à pictogramme — le libellé devient l'infobulle",
        "La variante icône du jeu (icon-tabs.png, 268 × 44) : 66 px par onglet, 2 de gouttière. Le pictogramme REMPLACE le libellé au rendu, mais le libellé reste — sans lui, une barre de pictogrammes n'apprend à personne ce que fait chaque onglet, et le journal n'aurait que des indices pour nommer ce qu'on a cliqué.",
    );
    {
        #[derive(Clone, Copy, PartialEq)]
        enum Vue {
            Combat,
            Suivi,
            Objets,
            Reglages,
        }
        let mut vue = Vue::Combat;
        design::tabs(&mut vue)
            .entry(Vue::Combat, "Combat")
            .icon(DsIcon::Cards)
            .entry(Vue::Suivi, "Suivi")
            .icon(DsIcon::Trophy)
            .entry(Vue::Objets, "Objets")
            .icon(DsIcon::BagIn)
            .entry(Vue::Reglages, "Réglages")
            .icon(DsIcon::Settings1)
            .enabled(false)
            .fit_content()
            .log_name("galerie.onglets-icone")
            .show(ui);
        ui.label(
            RichText::new("fit_content — les 66 px du jeu ; le dernier est désactivé")
                .color(CAPTION)
                .size(12.0),
        );
        // Étirée : la variante icône suit la même règle de largeur que la variante texte, les
        // pictogrammes restent centrés dans des onglets plus larges.
        let mut etiree = Vue::Suivi;
        design::tabs(&mut etiree)
            .entry(Vue::Combat, "Combat")
            .icon(DsIcon::Cards)
            .entry(Vue::Suivi, "Suivi")
            .icon(DsIcon::Trophy)
            .entry(Vue::Objets, "Objets")
            .icon(DsIcon::BagIn)
            .log_name("galerie.onglets-icone-etires")
            .show(ui);
        ui.label(
            RichText::new("par défaut — parts égales sur la largeur disponible")
                .color(CAPTION)
                .size(12.0),
        );
    }

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
        // `horizontal_wrapped` plutôt que `horizontal` : le lot d'icônes sans appelant
        // (2026-09-11) a fait dépasser la largeur fixe du canevas (760px) à une seule ligne.
        ui.horizontal_wrapped(|ui| {
            for (icon, name) in [
                (DsIcon::Option, "option"),
                (DsIcon::ExternalLink, "lien"),
                (DsIcon::Plus, "plus"),
                (DsIcon::Minus, "moins"),
                (DsIcon::Volume, "volume"),
                (DsIcon::VolumeMute, "volume-muet"),
                (DsIcon::Eye, "oeil"),
                (DsIcon::EyeOff, "oeil-barre"),
                (DsIcon::BagIn, "sac-entree"),
                (DsIcon::BagOut, "sac-sortie"),
                (DsIcon::Filter, "filtre"),
                (DsIcon::Lock, "cadenas"),
                (DsIcon::Order, "classement"),
                (DsIcon::Pact, "pacte"),
                (DsIcon::Save, "enregistrer"),
                (DsIcon::Sort, "tri"),
                (DsIcon::TriangleRight, "triangle"),
            ] {
                ui.add(
                    design::icon_button(icon)
                        .context(context)
                        .log_name(format!("galerie.{name}")),
                );
            }
            ui.add_space(20.0);
            ui.add(
                design::icon_button(DsIcon::Option)
                    .context(context)
                    .preview_state(IconButtonState::Hovered)
                    .log_name("galerie.icone-survol"),
            );
            ui.add(
                design::icon_button(DsIcon::Option)
                    .context(context)
                    .enabled(false)
                    .preview_state(IconButtonState::Disabled)
                    .log_name("galerie.icone-off"),
            );
        });
    }

    heading(
        ui,
        "Barre de recherche — la loupe DANS le champ, la croix à droite",
        "Le jeu pose toujours sa loupe à l'intérieur du champ, jamais sur un socle à côté — manche en bas à gauche, teinte de la loupe et non du texte indicatif (empty-input-search.png). La boîte fait 28 px, pas 25 : même encre, plus d'air. La deuxième ligne, la seule qui porte une valeur, est celle qui vérifie la gouttière ET la croix d'effacement : elle n'apparaît qu'avec une valeur (input-search.png), et le texte s'arrête avant elle.",
    );
    let mut recherche_vide = String::new();
    let mut recherche_pleine = String::from("pierre");
    ui.add(
        design::input(&mut recherche_vide)
            .leading_icon(DsIcon::Search)
            .size(InputSize::Search)
            .clearable(true)
            .placeholder("Rechercher")
            .width(420.0)
            .log_name("galerie.recherche-vide"),
    );
    ui.add(
        design::input(&mut recherche_pleine)
            .leading_icon(DsIcon::Search)
            .size(InputSize::Search)
            .clearable(true)
            .width(420.0)
            .log_name("galerie.recherche-pleine"),
    );
    ui.add(
        design::input(&mut recherche_pleine)
            .leading_icon(DsIcon::Search)
            .size(InputSize::Search)
            .clearable(true)
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
    ui.horizontal_wrapped(|ui| {
        for icon in [
            DsIcon::Option,
            DsIcon::ExternalLink,
            DsIcon::Plus,
            DsIcon::Minus,
            DsIcon::Close,
            DsIcon::Delete,
            DsIcon::Help,
            DsIcon::Undo,
            DsIcon::Volume,
            DsIcon::VolumeMute,
            DsIcon::Eye,
            DsIcon::EyeOff,
            DsIcon::BagIn,
            DsIcon::BagOut,
            DsIcon::Filter,
            DsIcon::Lock,
            DsIcon::Order,
            DsIcon::Pact,
            DsIcon::Save,
            DsIcon::Sort,
            DsIcon::TriangleRight,
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
        (DsIcon::Search, design::tokens::INPUT_PLACEHOLDER),
        (DsIcon::Tick, Color32::from_rgb(0x7A, 0xC7, 0x4F)),
        (DsIcon::ChevronDown, design::tokens::SELECT_TEXT),
        (DsIcon::Info, design::tokens::INFO_DOT),
    ] {
        let native = design::DesignSystem::get(ui.ctx()).icon_native_size(icon);
        ds.paint_icon(
            ui.painter(),
            egui::Rect::from_center_size(egui::pos2(x + native.x / 2.0, row.center().y), native),
            icon,
            tint,
        );
        x += native.x + 22.0;
    }

    heading(
        ui,
        "Glyphes sans socle connu — lot sans appelant (2026-09-11)",
        "Les treize icônes du lot du 2026-09-11 dont la mesure `--from-button` n'est pas consignée (marquées `libre` dans `design::icons`, voir la doc de chaque variante) : à défaut de certitude sur un socle porteur, elles sont peintes ici à leur taille de fichier, en teinte neutre — la même prudence que la rangée du dessus.",
    );
    ui.horizontal_wrapped(|ui| {
        for icon in [
            DsIcon::Book,
            DsIcon::Calendar,
            DsIcon::Cards,
            DsIcon::Characters,
            DsIcon::Grid,
            DsIcon::Hammer,
            DsIcon::Kamas,
            DsIcon::Pin,
            DsIcon::Repeat,
            DsIcon::Settings1,
            DsIcon::Settings2,
            DsIcon::Trophy,
            DsIcon::Xp,
        ] {
            let native = ds.icon_native_size(icon);
            let (rect, _) =
                ui.allocate_exact_size(native + Vec2::splat(14.0), egui::Sense::hover());
            ds.paint_icon(
                ui.painter(),
                egui::Rect::from_center_size(rect.center(), native),
                icon,
                design::tokens::ICON_TINT,
            );
        }
    });

    heading(
        ui,
        "Portrait — carré ou rond, grisé sur décision de l'appelant",
        "Le gabarit de combat loge ses portraits dans des médaillons ronds, la liste plate les pose carrés : c'est la forme des cadres du jeu, pas une préférence. Le pourcentage déborde du carré englobant vers l'EXTÉRIEUR — sur un rond, ce coin est hors du disque, et c'est ce qui l'empêche de recouvrir le visage.",
    );
    {
        use overlay_ui::design::PortraitShape;
        // Un glyphe du manifeste tient lieu de portrait : la galerie n'a ni atlas de classes ni
        // catalogue distant. Ce qu'on vérifie ici est la FORME, le grisé et le pourcentage.
        let faux = design::DesignSystem::get(ui.ctx())
            .icon(DsIcon::Characters)
            .id();
        ui.horizontal(|ui| {
            for (shape, dimmed, percent, nom) in [
                (PortraitShape::Square, false, None, "carré"),
                (PortraitShape::Round, false, None, "rond"),
                (PortraitShape::Round, true, None, "rond, KO"),
                (PortraitShape::Round, false, Some(42), "avec %"),
                (PortraitShape::Round, true, Some(7), "KO + %"),
                (PortraitShape::Square, false, Some(100), "carré, 100 %"),
            ] {
                ui.vertical(|ui| {
                    ui.add(
                        design::portrait(faux)
                            .shape(shape)
                            .size(48.0)
                            .dimmed(dimmed)
                            .percent(percent),
                    );
                    ui.label(RichText::new(nom).color(CAPTION).size(11.0));
                });
                ui.add_space(14.0);
            }
        });
    }

    heading(
        ui,
        "Jauge — six couches concentriques, et un arrondi conditionnel",
        "Les coins DROITS du remplissage ne s'arrondissent que s'il atteint le bout de la piste : sinon son bord tombe au milieu, et un coin arrondi y suggérerait un bord qui n'existe pas. Le curseur de fin disparaît à 100 % pour la même raison — il se confondrait avec le bord droit.",
    );
    {
        let mut valeur = 0.0_f32;
        ui.horizontal(|ui| {
            for f in [0.0_f32, 0.08, 0.5, 0.999, 1.0] {
                ui.vertical(|ui| {
                    ui.add(design::meter(f).width(130.0));
                    ui.label(
                        RichText::new(format!("{:.0} %", f * 100.0))
                            .color(CAPTION)
                            .size(11.0),
                    );
                });
                ui.add_space(8.0);
            }
            valeur += 1.0;
        });
        // Teintes : le remplissage vient de l'appelant, le reflet reste fixe — c'est ce qui se voit
        // ici, les quatre barres partageant le même liseré clair.
        ui.horizontal(|ui| {
            for (couleur, nom) in [
                (design::tokens::METER_FILL, "défaut"),
                (design::tokens::OVERLAY_ACCENT, "accent"),
                (design::tokens::INFO_ALERT, "alerte"),
                (design::tokens::TEXT_DISABLED, "grisé"),
            ] {
                ui.vertical(|ui| {
                    ui.add(design::meter(0.62).fill(couleur).width(130.0));
                    ui.label(RichText::new(nom).color(CAPTION).size(11.0));
                });
                ui.add_space(8.0);
            }
        });
        // Dégénérée : plus fine que ses deux bordures. Les rayons tombent à zéro plutôt que de
        // passer sous zéro, et la jauge reste peinte.
        ui.add(design::meter(0.5).width(130.0).height(6.0));
        ui.label(
            RichText::new("hauteur 6 — plus fine que ses deux bordures de 2")
                .color(CAPTION)
                .size(12.0),
        );
    }

    heading(
        ui,
        "Emplacement d'objet — l'ordre de peinture est le composant",
        "Les sept raretés, puis un cadre simple. La bordure de rareté se peint SOUS l'icône : sa fenêtre intérieure n'est pas un trou transparent mais un aplat teinté à ~70 %, et l'ordre inverse voile l'icône entière. Un cadre simple fait le contraire — c'est un liseré net, il passe par-dessus.",
    );
    {
        use overlay_ui::design::{ItemRarity, SlotCount, SlotFrame};
        // Une icône factice : la galerie n'a pas de catalogue distant. Le glyphe du manifeste tient
        // ce rôle — ce qu'on vérifie ici est le CADRE et son ordre, pas l'icône.
        let faux_icone = design::DesignSystem::get(ui.ctx()).icon(DsIcon::Kamas).id();
        ui.horizontal_wrapped(|ui| {
            for (rarity, nom) in [
                (ItemRarity::Common, "Common"),
                (ItemRarity::Rare, "Rare"),
                (ItemRarity::Mythical, "Mythical"),
                (ItemRarity::Legendary, "Legendary"),
                (ItemRarity::Memory, "Memory"),
                (ItemRarity::Epic, "Epic"),
                (ItemRarity::Relic, "Relic"),
            ] {
                ui.vertical(|ui| {
                    ui.add(
                        design::item_slot()
                            .frame(SlotFrame::Rarity(rarity))
                            .icon(faux_icone)
                            .log_name(format!("galerie.slot-{nom}")),
                    );
                    ui.label(RichText::new(nom).color(CAPTION).size(11.0));
                });
                ui.add_space(8.0);
            }
        });
        ui.horizontal(|ui| {
            // Cadre simple, avec et sans compteur, plus les deux formes de compteur.
            ui.add(
                design::item_slot()
                    .frame(SlotFrame::Plain)
                    .icon(faux_icone)
                    .log_name("galerie.slot-simple"),
            );
            ui.add_space(8.0);
            ui.add(
                design::item_slot()
                    .frame(SlotFrame::Rarity(ItemRarity::Legendary))
                    .icon(faux_icone)
                    .count(SlotCount::Simple(42))
                    .log_name("galerie.slot-compte"),
            );
            ui.add_space(8.0);
            ui.add(
                design::item_slot()
                    .frame(SlotFrame::Rarity(ItemRarity::Rare))
                    .icon(faux_icone)
                    .count(SlotCount::Fraction {
                        current: 137,
                        target: 500,
                    })
                    .log_name("galerie.slot-fraction"),
            );
            ui.add_space(8.0);
            // Sans icône : l'emplacement se peint quand même, le vide se voit.
            ui.add(
                design::item_slot()
                    .frame(SlotFrame::Rarity(ItemRarity::Epic))
                    .log_name("galerie.slot-vide"),
            );
            ui.add_space(8.0);
            // **Sous le minimum** : peint quand même — un rectangle trop petit doit se voir sur la
            // capture, pas paniquer — et signalé une fois au journal (clause 4 du contrat).
            ui.add(
                design::item_slot()
                    .size(6.0)
                    .frame(SlotFrame::Plain)
                    .icon(faux_icone)
                    .log_name("galerie.slot-minuscule"),
            );
        });
        ui.label(
            RichText::new(
                "cadre simple · compteur simple · fraction · sans icône · 6 px, sous le minimum",
            )
            .color(CAPTION)
            .size(12.0),
        );
    }

    heading(
        ui,
        "Glyphe seul — `design::icon`, sans socle ni clic",
        "Le composant qui manquait pour poser une icône dans une ligne : il alloue un CARRÉ et y inscrit le glyphe en gardant son rapport. Le chevron (14 × 8) le montre — il ne devient jamais carré, quelle que soit la taille demandée.",
    );
    ui.horizontal(|ui| {
        for size in [12.0_f32, 16.0, 24.0, 32.0] {
            ui.vertical(|ui| {
                ui.add(design::icon(DsIcon::Kamas).size(size));
                ui.label(
                    RichText::new(format!("{size:.0}"))
                        .color(CAPTION)
                        .size(11.0),
                );
            });
            ui.add_space(10.0);
        }
        ui.add_space(20.0);
        // Deux glyphes NON carrés à la même taille demandée : c'est le rapport préservé qui se
        // voit, pas la taille. Un `Vec2::splat` les aplatirait tous les deux à l'identique.
        for icon in [DsIcon::ChevronDown, DsIcon::Info, DsIcon::TriangleRight] {
            ui.add(design::icon(icon).size(24.0));
            ui.add_space(10.0);
        }
        ui.add_space(20.0);
        ui.add(
            design::icon(DsIcon::Lock)
                .size(24.0)
                .tint(design::tokens::TEXT_DISABLED),
        );
        ui.add(
            design::icon(DsIcon::Tick)
                .size(24.0)
                .tint(design::tokens::INFO_ALERT),
        );
    });

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

    heading(
        ui,
        "Pas numérique — socle 32 px, champ à la hauteur de ses boutons",
        "Deux boutons icône au contexte Stepper et un champ entre eux, tous de la même hauteur. Aux trois tailles : la gouttière et le glyphe suivent le socle. Le deuxième est aux cotes du jeu (socle 32, champ 106) — le jeu, lui, y met un champ de 26 px, écart assumé au profit de l'uniformité.",
    );
    for (cote, champ, valeur, borne) in [
        (26.0_f32, 38.0_f32, 1_i64, 1_i64..=99),
        (32.0, 106.0, 12, 1..=99),
        (40.0, 140.0, 7, 1..=99),
    ] {
        let mut v = valeur;
        ui.add(
            design::stepper(&mut v)
                .size(cote)
                .field_width(champ)
                .range(borne)
                .log_name("galerie.pas"),
        );
    }
    // Les deux bornes : à la borne basse le « − » est désactivé, à la haute le « + ».
    ui.horizontal(|ui| {
        let mut bas = 1_i64;
        ui.add(
            design::stepper(&mut bas)
                .field_width(70.0)
                .range(1..=99)
                .log_name("galerie.pas-borne-basse"),
        );
        ui.add_space(18.0);
        let mut haut = 99_i64;
        ui.add(
            design::stepper(&mut haut)
                .field_width(70.0)
                .range(1..=99)
                .log_name("galerie.pas-borne-haute"),
        );
        ui.add_space(18.0);
        let mut off = 42_i64;
        ui.add(
            design::stepper(&mut off)
                .field_width(70.0)
                .enabled(false)
                .log_name("galerie.pas-desactive"),
        );
    });

    heading(
        ui,
        "Chrome de fenêtre — bannière, onglets, panneau, pied de page",
        "Les trois conteneurs à l'échelle réduite : design::window pose le décor et rend ses zones, design::panel écrête son contenu, design::heading titre une section. Le dernier item de la liste est volontairement hors du panneau — l'écrêtage doit le couper net.",
    );
    {
        // Rendu à 640 × 300 plutôt qu'aux 720 × 561 de la vraie fenêtre : ce qui est vérifié ici
        // est la COMPOSITION des trois conteneurs, pas la cote de la fenêtre Options — celle-là a
        // ses propres snapshots (`options_modale_sur_damier.png`). Une fenêtre à ces dimensions
        // montre au passage que le décor tient à n'importe quelle taille, ce qu'un 9-slice promet.
        let (rect, _) = ui.allocate_exact_size(Vec2::new(640.0, 300.0), egui::Sense::hover());
        let mut tab = GalleryTab::Reglages;
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            let chrome = design::window("Fenêtre")
                .footer("Annuler", "Valider")
                .log_name("galerie.fenetre")
                .show(ui);
            chrome.tabs(
                ui,
                design::tabs(&mut tab)
                    .entry(GalleryTab::Reglages, "Réglages")
                    .entry(GalleryTab::Avance, "Avancé")
                    .entry(GalleryTab::Indisponible, "Indisponible")
                    .enabled(false)
                    .log_name("galerie.fenetre-onglets"),
            );
            design::panel().show(ui, chrome.content, |ui, _panel| {
                ui.add(design::heading("Section"));
                for n in 1..=6 {
                    ui.label(
                        egui::RichText::new(format!("Ligne de contenu n° {n}"))
                            .color(design::tokens::INFO_TEXT)
                            .size(14.0),
                    );
                    ui.add_space(6.0);
                }
            });
        });
    }

    heading(
        ui,
        "Bloc repliable — deux états, contenu libre",
        "Fermé, le bloc n'est que son en-tête (60 px). Ouvert, il encadre ce qu'on lui donne : ici du texte, mais ce pourrait être un formulaire ou un tableau. Le cadre est peint APRÈS le contenu, sa hauteur en dépend.",
    );
    {
        let mut ferme = false;
        design::collapsible("Bloc fermé", &mut ferme)
            .icon(DsIcon::Info)
            .log_name("galerie.repliable-ferme")
            .show(ui, |ui| {
                ui.label("jamais rendu tant que le bloc est fermé");
            });
        ui.add_space(10.0);

        let mut ouvert = true;
        design::collapsible("Bloc ouvert", &mut ouvert)
            .icon(DsIcon::Info)
            .log_name("galerie.repliable-ouvert")
            .show(ui, |ui| {
                for ligne in [
                    "Le contenu est entièrement libre : le composant n'en sait rien.",
                    "Il lui garantit un cadre, des marges et un écrêtage, rien de plus.",
                ] {
                    ui.label(
                        egui::RichText::new(ligne)
                            .color(design::tokens::INFO_TEXT)
                            .size(14.0),
                    );
                    ui.add_space(6.0);
                }
                ui.add(design::checkbox(
                    &mut true.clone(),
                    "y compris un autre composant",
                ));
            });
        ui.add_space(10.0);

        // Survolé — état forcé, aucun pointeur ne survole quoi que ce soit hors écran. Le jeu
        // éclaircit le CADRE ENTIER, pas seulement l'en-tête : c'est pour le montrer que ce
        // troisième bloc est ouvert plutôt que fermé.
        let mut survole = true;
        design::collapsible("Bloc survolé", &mut survole)
            .icon(DsIcon::Info)
            .preview_hovered(true)
            .log_name("galerie.repliable-survole")
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("le fond s'éclaircit de dix niveaux, contenu compris")
                        .color(design::tokens::INFO_TEXT)
                        .size(14.0),
                );
            });
        ui.add_space(10.0);

        // Sans icône, et avec un contenu qui déborde : l'écrêtage doit se voir.
        let mut sans_icone = true;
        design::collapsible("Sans icône, contenu écrêté", &mut sans_icone)
            .log_name("galerie.repliable-nu")
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Une ligne délibérément trop longue pour la largeur du bloc, afin que la coupe se voie sur la capture plutôt que d'être affirmée dans un commentaire.",
                    )
                    .color(design::tokens::INFO_TEXT)
                    .size(14.0),
                );
            });
    }

    heading(
        ui,
        "Curseur de réglage — gradué ou continu, selon ce qu'il y a à choisir",
        "Un curseur gradué annonce où la poignée peut s'immobiliser ; un curseur continu n'a rien à annoncer. La distinction est mesurée : l'échelle d'interface du jeu porte 26 graduations, le volume aucune. La rainure, elle, n'a pas de couleur — elle assombrit le fond.",
    );
    {
        let mut volume = 0.0_f32;
        // Trois positions sur la largeur du jeu (200 px) — les deux extrémités et le milieu, ce
        // que trois curseurs de 200 laissent tenir sur une ligne de 744. Aux extrémités, le disque
        // doit affleurer le bord de la rainure sans le dépasser : c'est ce que `track_travel`
        // garantit, et c'est là qu'on regarde le moins.
        ui.horizontal(|ui| {
            for fraction in [0.0, 0.5, 1.0] {
                ui.add(
                    design::slider(&mut volume)
                        .width(design::tokens::SLIDER_TRACK_WIDTH_REF)
                        .preview_fraction(fraction)
                        .log_name(format!("galerie.slider-{fraction}")),
                );
                ui.add_space(12.0);
            }
        });
        // Gradué — les 26 crans de l'échelle d'interface du jeu, puis deux comptes plus courts
        // pour que le pas se lise. La poignée ne s'immobilise que sur une graduation.
        ui.horizontal(|ui| {
            for (steps, fraction) in [(26_usize, 0.2_f32), (11, 0.5), (3, 1.0)] {
                ui.add(
                    design::slider(&mut volume)
                        .width(design::tokens::SLIDER_TRACK_WIDTH_REF)
                        .steps(steps)
                        .preview_fraction(fraction)
                        .log_name(format!("galerie.slider-{steps}-crans")),
                );
                ui.add_space(12.0);
            }
        });
        // Le motif du jeu : un libellé de chaque côté, à la gouttière relevée. Les mots
        // appartiennent à l'appelant — « Min »/« Max » pour un volume, « 50 % »/« 200 % » pour une
        // échelle d'interface.
        ui.horizontal(|ui| {
            ui.label(RichText::new("Min").color(CAPTION).size(13.0));
            ui.add_space(design::tokens::SLIDER_LABEL_GAP);
            ui.add(
                design::slider(&mut volume)
                    .width(design::tokens::SLIDER_TRACK_WIDTH_REF)
                    .preview_fraction(0.35)
                    .log_name("galerie.slider-libelles"),
            );
            ui.add_space(design::tokens::SLIDER_LABEL_GAP);
            ui.label(RichText::new("Max").color(CAPTION).size(13.0));
        });
        ui.horizontal(|ui| {
            ui.add(
                design::slider(&mut volume)
                    .width(design::tokens::SLIDER_TRACK_WIDTH_REF)
                    .preview_fraction(0.6)
                    .enabled(false)
                    .preview_state(SliderState::Disabled)
                    .log_name("galerie.slider-desactive"),
            );
            ui.add_space(12.0);
            // Dégénéré : plus étroit que la poignée. La course tombe à zéro, le disque reste posé
            // à gauche au lieu de reculer quand la valeur monte.
            ui.add(
                design::slider(&mut volume)
                    .width(10.0)
                    .preview_fraction(1.0)
                    .log_name("galerie.slider-degenere"),
            );
        });
    }

    heading(
        ui,
        "Filet de séparation — un bevel, pas un trait",
        "Deux lignes plates de 2 px, claire sur sombre. Les teintes sont des écarts autour du fond : sur une surface survolée le jeu recalcule les deux, et c'est pour ça que le filet a sa propre paire. Montré ici sur son fond d'origine — celui du bloc repliable, pour lequel il a été mesuré.",
    );
    {
        // Le motif du jeu : intitulé, filet, corps. Deux fois dans le bloc de quête relevé, à
        // l'identique — c'est ce qui a justifié d'en faire un composant plutôt qu'un `hline`.
        let sections = [
            ("Description", "Un paragraphe de corps, posé sous le filet."),
            (
                "Objectifs",
                "Le même motif, répété — c'est de là que vient le composant.",
            ),
        ];
        let mut motif = true;
        design::collapsible("Le motif du jeu", &mut motif)
            .icon(DsIcon::Info)
            .log_name("galerie.filet-motif")
            .show(ui, |ui| {
                // L'espacement par défaut de la galerie (8 px) s'ajouterait aux cotes du relevé
                // et on verrait 18 px là où le jeu en met 10. Il est neutralisé pour que cette
                // section montre les vraies cotes — c'est aussi l'exemple d'usage de référence.
                ui.spacing_mut().item_spacing.y = 0.0;
                for (titre, corps) in sections {
                    ui.label(
                        RichText::new(titre)
                            .color(Color32::from_rgb(0xBD, 0xBD, 0xBE))
                            .size(14.0)
                            .strong(),
                    );
                    ui.add_space(10.0);
                    ui.add(design::separator().log_name(format!("galerie.filet-{titre}")));
                    ui.label(
                        RichText::new(corps)
                            .color(design::tokens::INFO_TEXT)
                            .size(14.0),
                    );
                    ui.add_space(10.0);
                }
            });
        ui.add_space(10.0);

        // Le même motif sur un bloc survolé. Sans `on_hovered_surface`, le filet garderait ses
        // teintes de repos et sa ligne claire se fondrait dans le fond éclairci — un niveau
        // d'écart. Les deux blocs se comparent ligne à ligne sur la capture.
        let mut survole = true;
        design::collapsible("Le même, surface survolée", &mut survole)
            .icon(DsIcon::Info)
            .preview_hovered(true)
            .log_name("galerie.filet-survole")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(
                    RichText::new("Description")
                        .color(Color32::from_rgb(0xBD, 0xBD, 0xBE))
                        .size(14.0)
                        .strong(),
                );
                ui.add_space(10.0);
                ui.add(
                    design::separator()
                        .on_hovered_surface(true)
                        .log_name("galerie.filet-survole-1"),
                );
                ui.label(
                    RichText::new("le bevel a suivi le fond, dix niveaux plus haut")
                        .color(design::tokens::INFO_TEXT)
                        .size(14.0),
                );
                ui.add_space(10.0);
                // Ce que donnerait l'oubli du paramètre : la ligne claire du repos sur le fond
                // survolé. Elle ne se voit plus, et c'est le but de la montrer.
                ui.label(
                    RichText::new("Sans on_hovered_surface — le même filet, au repos")
                        .color(Color32::from_rgb(0xBD, 0xBD, 0xBE))
                        .size(14.0)
                        .strong(),
                );
                ui.add_space(10.0);
                ui.add(design::separator().log_name("galerie.filet-survole-oubli"));
                ui.label(
                    RichText::new("la ligne claire a disparu dans le fond")
                        .color(design::tokens::INFO_TEXT)
                        .size(14.0),
                );
                ui.add_space(10.0);
            });
        ui.add_space(10.0);

        // Largeur imposée et largeur disponible, sur le fond de page nu : le composant ne pose
        // aucune marge latérale, c'est l'appelant qui décide de son étendue.
        ui.add(
            design::separator()
                .width(200.0)
                .log_name("galerie.filet-200"),
        );
        ui.label(
            RichText::new("width(200) — largeur imposée")
                .color(CAPTION)
                .size(12.0),
        );
        ui.add(design::separator().log_name("galerie.filet-plein"));
        ui.label(
            RichText::new("par défaut — toute la largeur disponible")
                .color(CAPTION)
                .size(12.0),
        );
    }

    heading(
        ui,
        "Rouage de chargement — de 48 à 124 px, toujours carré",
        "Une planche de 16 images peinte par région, 24 i/s. Small 48 est le plancher (décision utilisateur), Native 124 la taille du jeu ; Medium et Large sont des paliers choisis. Image figée par preview_frame : hors écran, l'horloge ne tourne pas.",
    );
    ui.horizontal(|ui| {
        for (size, label) in [
            (LoaderSize::Small, "Small · 48"),
            (LoaderSize::Medium, "Medium · 72"),
            (LoaderSize::Large, "Large · 96"),
            (LoaderSize::Native, "Native · 124"),
        ] {
            ui.vertical(|ui| {
                ui.add(design::loader().size(size).preview_frame(0).log_name(label));
                ui.label(RichText::new(label).color(CAPTION).size(12.0));
            });
            ui.add_space(14.0);
        }
    });
    // La boucle, une image sur deux : un tour de dent (8 images) sur la ligne, et la tête qui
    // se balance d'une image à l'autre — c'est ce qui distingue la boucle de 16 d'une boucle de 8.
    ui.horizontal(|ui| {
        for frame in (0..16).step_by(2) {
            ui.add(
                design::loader()
                    .size(LoaderSize::Small)
                    .preview_frame(frame)
                    .log_name(format!("galerie.loader-{frame:02}")),
            );
        }
    });
    // Hors intervalle : 20 px demandés, 48 peints — et un `warn!` unique dans le journal.
    ui.add(
        design::loader()
            .size(LoaderSize::Px(20.0))
            .preview_frame(0)
            .log_name("galerie.loader-hors-intervalle"),
    );

    section_label(ui);
    section_autocomplete(ui);
}

/// Le libellé élidé — **trois largeurs pour un même texte**, parce que c'est le rapport entre les
/// deux qui décide de l'ellipse, et rien d'autre.
///
/// L'infobulle ne se voit pas ici : elle demande un curseur, et aucun ne survole quoi que ce soit
/// en rendu offscreen. C'est `tests/panels.rs` qui la vérifie, par survol simulé.
fn section_label(ui: &mut egui::Ui) {
    heading(
        ui,
        "design::label — libellé élidé",
        "Même texte, trois largeurs : l'ellipse vient du rapport entre les deux, pas du texte. L'infobulle ne se voit pas ici — elle demande un curseur, et `tests/panels.rs` la vérifie par survol simulé.",
    );
    for largeur in [220.0_f32, 130.0, 80.0] {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{largeur:.0} px"))
                    .color(CAPTION)
                    .size(11.0),
            );
            ui.add_space(8.0);
            ui.add(
                design::label("Plan \"Epée de Brâkmar\"")
                    .width(largeur)
                    .log_name(format!("galerie.label-{largeur:.0}")),
            );
        });
    }
    ui.add_space(8.0);
    heading(
        ui,
        "Les trois alignements",
        "À largeur assez large pour que le placement se voie.",
    );
    for (align, nom) in [
        (egui::Align::Min, "gauche"),
        (egui::Align::Center, "centré"),
        (egui::Align::Max, "droite"),
    ] {
        ui.horizontal(|ui| {
            ui.label(RichText::new(nom).color(CAPTION).size(11.0));
            ui.add_space(8.0);
            ui.add(
                design::label("Pierre ultime")
                    .width(200.0)
                    .align(align)
                    .log_name(format!("galerie.label-{nom}")),
            );
        });
    }
}

/// **Seconde planche** — le tableau et sa pagination, parce que la première a atteint le plafond
/// matériel.
///
/// `wgpu` refuse une texture de plus de 8192 px de côté, et `galerie_du_design_system` en occupe
/// déjà 7530. Ce n'est donc pas un choix de présentation : une section de 1580 px n'y entre plus.
/// La clause 7 du contrat de composant est respectée — toute variante et tout état du tableau sont
/// sur une capture unique, celle-ci — et les prochaines sections trop grandes suivront le même
/// chemin plutôt que de rogner sur ce qu'elles montrent.
#[test]
fn galerie_du_tableau() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(760.0, 2430.0))
        .build_ui(|ui| {
            overlay_ui::style::apply(ui.ctx());
            egui::Frame::NONE
                .fill(PAGE_FILL)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);
                    section_table(ui);
                    section_pagination(ui);
                });
        });

    harness.run();
    harness.snapshot("design_gallery_table");
}

/// **Troisième planche** — la boîte de confirmation, qui ne peut pas vivre dans les deux autres.
///
/// Son voile couvre tout ce que l'appelant lui donne : posée dans un canevas de 7530 px, elle
/// assombrirait la galerie entière et masquerait toutes les autres sections. Elle a donc sa propre
/// planche, à la taille d'une vraie fenêtre — et c'est aussi la seule façon de montrer ce que ce
/// composant a de particulier : **ce qu'il assombrit**.
///
/// Du contenu factice est peint dessous exprès (un titre, un champ, deux boutons) : sans lui, le
/// voile ne se verrait pas, et c'est lui qu'il faut juger.
#[test]
fn galerie_de_la_confirmation() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(560.0, 320.0))
        .build_ui(|ui| {
            overlay_ui::style::apply(ui.ctx());
            let fenetre = ui.max_rect();
            egui::Frame::NONE
                .fill(PAGE_FILL)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);
                    // Le contenu que la boîte interrompt — c'est lui qui rend le voile lisible.
                    ui.add(design::heading("Objets suivis"));
                    ui.add_space(6.0);
                    let mut saisie = "Pierre ultime".to_owned();
                    ui.add(
                        design::input(&mut saisie)
                            .width(400.0)
                            .log_name("galerie.confirm-fond-champ"),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.add(
                            design::button("Annuler")
                                .variant(design::ButtonVariant::Danger)
                                .size(design::ButtonSize::Compact)
                                .width(180.0)
                                .log_name("galerie.confirm-fond-annuler"),
                        );
                        ui.add_space(10.0);
                        ui.add(
                            design::button("Valider")
                                .variant(design::ButtonVariant::Primary)
                                .size(design::ButtonSize::Compact)
                                .width(180.0)
                                .log_name("galerie.confirm-fond-valider"),
                        );
                    });
                    // Et la boîte par-dessus, sur la fenêtre ENTIÈRE — marge du cadre comprise.
                    design::confirm_dialog("Retirer « Pierre ultime » de vos alertes ?")
                        .over(fenetre)
                        .log_name("galerie.confirm")
                        .show(ui);
                });
        });

    harness.run();
    harness.snapshot("design_gallery_confirm");
}

/// L'autocomplétion — le seul composant de la galerie dont le panneau **sort de son rectangle**
/// (comme `select` déplié), d'où les espaces réservés sous chaque cas.
///
/// `preview_open`/`preview_active`/`preview_filter` forcent l'état peint : hors écran, aucun champ
/// n'a le focus, donc rien ne s'ouvrirait jamais.
fn section_autocomplete(ui: &mut egui::Ui) {
    use wakassets_fixtures::{CategoryFilter, CategoryIcons, ItemIcons, RarityGems, GEM_NATIVE};

    // **Gardées en mémoire egui, pas rechargées à chaque frame** : un `TextureHandle` libère sa
    // texture quand le dernier exemplaire tombe, et un chargement local peindrait donc des cases
    // vides — le rendu a lieu après la fin de cette fonction.
    //
    // Le chargement se fait HORS du verrou de `data_mut` : `load` appelle `Context::load_texture`,
    // qui demande ce même verrou — l'imbriquer fige egui dix secondes puis fait paniquer le test.
    let gems = charge_une_fois(ui, "galerie.gemmes", RarityGems::load);
    let cats = charge_une_fois(ui, "galerie.categories", CategoryIcons::load);
    let objets = charge_une_fois(ui, "galerie.objets", ItemIcons::load);
    let largeur = 560.0;

    // Les images viennent des fixtures du harnais, qui tiennent lieu de ce que `RemoteIconStore`
    // télécharge au runtime : le composant ne les résout pas lui-même, ce sont du CONTENU.
    // `image` n'est posée que sur les deux premières rangées : au runtime l'icône arrive du CDN
    // APRÈS la suggestion, et une rangée doit rester lisible sans elle. Les deux cas sont donc
    // visibles sur la même capture.
    let entree = |label: &str, categorie: u16, rarete, deja: bool, image: Option<usize>| {
        let mut entry = design::AutocompleteEntry::new(label, categorie);
        entry.gem = Some(gems.texture_id(rarete));
        entry.gem_size = GEM_NATIVE;
        entry.image = image.map(|rang| objets.texture_id(rang));
        entry.disabled = deja;
        if deja {
            entry.mention = Some("déjà suivi".to_owned());
        }
        entry
    };
    let entrees = vec![
        entree(
            "Pierre d'aventure",
            2,
            overlay_engine::WakfuRarity::Mythical,
            true,
            Some(0),
        ),
        entree(
            "Pierre de dolomite",
            2,
            overlay_engine::WakfuRarity::Common,
            false,
            Some(1),
        ),
        entree(
            "Pierre de lune",
            1,
            overlay_engine::WakfuRarity::Rare,
            false,
            None,
        ),
        entree(
            "Pierre ponce",
            7,
            overlay_engine::WakfuRarity::Common,
            false,
            None,
        ),
    ];
    let filtre = |f: CategoryFilter, categorie: Option<u16>| match categorie {
        None => design::AutocompleteFilter::all(f.label(), Some(cats.texture_id(f))),
        Some(c) => design::AutocompleteFilter::category(c, f.label(), Some(cats.texture_id(f))),
    };
    let filtres = vec![
        filtre(CategoryFilter::All, None),
        filtre(CategoryFilter::Equipment, Some(1)),
        filtre(CategoryFilter::Resources, Some(2)),
        filtre(CategoryFilter::Craft, Some(7)),
    ];

    heading(
        ui,
        "Autocomplétion — replié, et le seuil de trois caractères",
        "Sous le seuil, le panneau ne s'ouvre pas : ce n'est pas une liste vide, c'est une liste qui ne s'affiche pas.",
    );
    let mut vide = String::new();
    design::autocomplete(&mut vide)
        .placeholder("Ajouter un objet à surveiller…")
        .width(largeur)
        .entries(&entrees)
        .filters(&filtres)
        .log_name("galerie.autocomplete-replie")
        .show(ui);
    let mut court = String::from("pi");
    design::autocomplete(&mut court)
        .width(largeur)
        .entries(&entrees)
        .filters(&filtres)
        .log_name("galerie.autocomplete-sous-seuil")
        .show(ui);

    heading(
        ui,
        "Déplié — « Tout » actif, une entrée déjà suivie",
        "La deuxième rangée porte l'entrée active (celle que le clavier désigne). La première est grisée, sans surbrillance : elle n'est pas sélectionnable. Les deux dernières n'ont pas encore leur icône — la rangée reste lisible sans elle.",
    );
    let mut saisi = String::from("pierre");
    design::autocomplete(&mut saisi)
        .width(largeur)
        .entries(&entrees)
        .filters(&filtres)
        .preview_open(true)
        .preview_active(1)
        .log_name("galerie.autocomplete-deplie")
        .show(ui);
    // Le panneau est peint dans une `Area` hors flux : sans cette réserve, la section suivante
    // passerait dessous.
    ui.add_space(4.0 + 38.0 + 4.0 * design::tokens::AUTOCOMPLETE_ROW_HEIGHT);

    heading(
        ui,
        "Au-delà de cinq rangées — la liste défile",
        "Neuf entrées, cinq visibles : la barre du web (8 px, rayon 4) sur un rail plus sombre que la liste. Elle ne s’élargit pas ; sous le pointeur, sa poignée prend la teinte des rangées survolées. Sa colonne est réservée : la mention de droite ne passe jamais dessous. Les flèches font défiler la liste pour garder l’entrée active en vue.",
    );
    let mut long = String::from("pierre");
    let beaucoup: Vec<design::AutocompleteEntry> = (1..=9)
        .map(|i| {
            let mut e = design::AutocompleteEntry::new(format!("Pierre de taille n° {i}"), 1);
            if i == 3 {
                e.disabled = true;
                e.mention = Some("déjà suivi".to_owned());
            }
            e
        })
        .collect();
    design::autocomplete(&mut long)
        .width(largeur)
        .entries(&beaucoup)
        .filters(&filtres)
        .preview_open(true)
        .log_name("galerie.autocomplete-defile")
        .show(ui);
    ui.add_space(4.0 + 38.0 + 5.0 * design::tokens::AUTOCOMPLETE_ROW_HEIGHT);

    heading(
        ui,
        "Un filtre actif — et un filtre sans résultat",
        "À gauche « Équipements » ne laisse qu'une entrée. À droite « Craft » n'en laisse aucune : la bande RESTE, sinon le bouton qui permettrait de la relâcher disparaîtrait avec les rangées.",
    );
    let mut filtre_actif = String::from("pierre");
    design::autocomplete(&mut filtre_actif)
        .width(largeur)
        .entries(&entrees)
        .filters(&filtres)
        .preview_open(true)
        .preview_filter(Some(1))
        .log_name("galerie.autocomplete-filtre")
        .show(ui);
    ui.add_space(4.0 + 38.0 + design::tokens::AUTOCOMPLETE_ROW_HEIGHT);

    let mut filtre_vide = String::from("pierre");
    let sans_resultat: Vec<design::AutocompleteEntry> = entrees
        .iter()
        .filter(|e| e.category != 9)
        .cloned()
        .collect();
    design::autocomplete(&mut filtre_vide)
        .width(largeur)
        .entries(&sans_resultat)
        .filters(&filtres)
        .preview_open(true)
        .preview_filter(Some(9))
        .log_name("galerie.autocomplete-filtre-vide")
        .show(ui);
    ui.add_space(4.0 + 38.0 + 34.0);
}

/// Le tableau — ses trois corps (peuplé, vide, en chargement), son défilement et son cas dégénéré.
///
/// **Le fond en bandes est relégué au dernier bloc**, et c'est un arbitrage : il démontre que le
/// zébrage éclaircit le décor au lieu de le remplacer (sur un fond uni, la démonstration serait
/// invisible), mais il rend les lignes illisibles — retour utilisateur du 2026-09-12, « on a
/// l'impression que c'est un damier ». Les cas d'usage se jugent donc sur le fond de la planche,
/// et la mesure garde son bloc à part, clairement annoncé comme tel.
fn section_table(ui: &mut egui::Ui) {
    let largeur = 728.0;

    let colonnes = || {
        vec![
            TableColumn::fixed("Date", 104.0),
            TableColumn::flex("Nom", 1.0),
            TableColumn::fixed("Niv.", 56.0).align(TableAlign::Center),
            TableColumn::fixed("Prix", 108.0).align(TableAlign::End),
        ]
    };

    // Quatre offres factices — dont une dont le nom déborde de sa colonne, pour que l'écrêtage se
    // voie (clause « au moins un contenu qui déborde » du contrat, §1 bis).
    let offres: [(&str, &str, i32, &str); 4] = [
        ("12/09 14:32", "Coiffe du Bouftou Royal", 50, "12 400"),
        (
            "12/09 13:58",
            "Cape de Tofu enragé aux mille et une plumes du Bouftou de Sidimote",
            35,
            "980",
        ),
        ("11/09 22:07", "Anneau de Dragodinde", 65, "145 000"),
        ("11/09 19:41", "Amulette du Chafer", 20, "3 210"),
    ];

    let cellule = |ui: &mut egui::Ui, texte: &str, couleur: Color32| {
        ui.label(RichText::new(texte).color(couleur).size(15.0));
    };

    heading(
        ui,
        "Tableau — peuplé",
        "Lignes de 60 px, en-tête à 14 px d'encre, écart de 7 px : les trois cotes du relevé HDV. Date et Nom sont à gauche, Niv. et Prix à droite — un nombre s'aligne par ses unités. La deuxième ligne porte un nom trop long : il est coupé à la colonne, pas au tableau.",
    );
    design::table()
        .columns(colonnes())
        .body(TableBody::Rows(offres.len()))
        .log_name("galerie.table-peuple")
        .show(ui, |row| {
            let (date, nom, niveau, prix) = offres[row.index()];
            row.cell(|ui| cellule(ui, date, CAPTION));
            row.cell(|ui| cellule(ui, nom, Color32::WHITE));
            row.cell(|ui| cellule(ui, &niveau.to_string(), Color32::WHITE));
            row.cell(|ui| cellule(ui, prix, HEADING));
        });

    heading(
        ui,
        "Vide — avec message, puis comme le fait le jeu",
        "Le jeu n'affiche RIEN dans un tableau à « 0 Objet » : pas de message, pas d'illustration. Le message est donc une décision de l'overlay, et il reste facultatif — sans empty_text, le corps garde sa hauteur et demeure vide (à droite du titre suivant).",
    );
    design::table()
        .columns(colonnes())
        .body(TableBody::Empty)
        .empty_text("Aucune vente sur la période")
        .log_name("galerie.table-vide")
        .show(ui, |_| {});

    design::table()
        .columns(colonnes())
        .body(TableBody::Rows(0))
        .log_name("galerie.table-vide-muet")
        .show(ui, |_| {});

    heading(
        ui,
        "En chargement — le rouage du jeu, rien d'autre",
        "Seul état des trois à ne rien inventer : c'est la planche d'animation relevée sur les écrans de chargement du client. Image figée par preview_loader_frame, sans quoi deux captures différeraient.",
    );
    design::table()
        .columns(colonnes())
        .body(TableBody::Loading)
        .preview_loader_frame(0)
        .log_name("galerie.table-chargement")
        .show(ui, |_| {});

    heading(
        ui,
        "Corps borné — douze lignes dans la place de quatre",
        "max_height met le CORPS seul dans une design::scroll_area : l'en-tête ne défile pas. La réserve permanente de 26 px de la barre est prise sur la largeur des colonnes dès que la borne existe, que la barre serve ou non — sinon les colonnes sauteraient le jour où une ligne de plus la fait apparaître.",
    );
    design::table()
        .columns(colonnes())
        .body(TableBody::Rows(12))
        .max_height(4.0 * 60.0)
        .log_name("galerie.table-defilant")
        .show(ui, |row| {
            let (date, nom, niveau, prix) = offres[row.index() % offres.len()];
            row.cell(|ui| cellule(ui, date, CAPTION));
            row.cell(|ui| cellule(ui, nom, Color32::WHITE));
            row.cell(|ui| cellule(ui, &niveau.to_string(), Color32::WHITE));
            row.cell(|ui| cellule(ui, prix, HEADING));
        });

    heading(
        ui,
        "Cas dégénéré — des colonnes fixes qui ne tiennent pas",
        "268 px de largeurs imposées dans 200 px : tout est réduit du même facteur et les élastiques tombent à zéro. Un tableau tassé se voit et se corrige ; un tableau qui déborde de son panneau passe pour un bug du panneau voisin.",
    );
    design::table()
        .columns(colonnes())
        .body(TableBody::Rows(2))
        .width(200.0)
        .log_name("galerie.table-serre")
        .show(ui, |row| {
            let (date, nom, niveau, prix) = offres[row.index()];
            row.cell(|ui| cellule(ui, date, CAPTION));
            row.cell(|ui| cellule(ui, nom, Color32::WHITE));
            row.cell(|ui| cellule(ui, &niveau.to_string(), Color32::WHITE));
            row.cell(|ui| cellule(ui, prix, HEADING));
        });

    heading(
        ui,
        "Contrôle du zébrage — trois lignes sur un fond qui change",
        "Ce bloc n'est pas un cas d'usage, c'est une MESURE : le tableau du jeu n'a pas de fond propre, son zébrage éclaircit le décor au lieu de le remplacer. Sur un fond uni la démonstration serait invisible, d'où les six bandes — qui rendent en revanche les lignes illisibles, raison pour laquelle elles ne servent qu'ici.",
    );
    let controle = design::table()
        .columns(vec![
            TableColumn::flex("Nom", 1.0),
            TableColumn::numeric("Prix", 108.0),
        ])
        .body(TableBody::Rows(3))
        .log_name("galerie.table-zebrage");
    let fond = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(largeur, controle.height()));
    for (index, gris) in [0x10, 0x1C, 0x26, 0x1A, 0x2E, 0x14].into_iter().enumerate() {
        let bande = egui::Rect::from_min_size(
            egui::pos2(fond.left() + index as f32 * fond.width() / 6.0, fond.top()),
            Vec2::new(fond.width() / 6.0, fond.height()),
        );
        ui.painter()
            .rect_filled(bande, 0, Color32::from_rgb(gris, gris, gris + 4));
    }
    controle.show(ui, |row| {
        let (_, nom, _, prix) = offres[row.index()];
        row.cell(|ui| cellule(ui, nom, Color32::WHITE));
        row.cell(|ui| cellule(ui, prix, HEADING));
    });
}

/// La pagination — les quatre positions possibles dans une suite de pages, et le cas du jeu.
///
/// Elle est ici plutôt que dans `galerie_du_design_system` pour la même raison que le tableau (le
/// plafond de 8192 px), et à côté de lui parce que c'est ensemble qu'ils se relisent — même si,
/// justement, ce n'est PAS un pied de tableau : dans « Mes offres » le jeu la pose en haut.
fn section_pagination(ui: &mut egui::Ui) {
    heading(
        ui,
        "Pagination — les quatre positions, et celle du jeu",
        "Deux boutons icône de 36 px et un même triangle, l'un retourné. Ce qui bouge est en or (« Page » et le numéro courant), ce qui borne est en blanc (la barre et le total). Le premier cas est celui des trois captures relevées : « Page 0 / 0 », les deux flèches grisées.",
    );
    for (page, total, libelle) in [
        (0usize, 0usize, "0 / 0 — le cas du jeu, rien à parcourir"),
        (1, 12, "première page — reculer est impossible"),
        (6, 12, "au milieu — les deux flèches actives"),
        (12, 12, "dernière page — avancer est impossible"),
    ] {
        ui.horizontal(|ui| {
            design::pagination(page, total)
                .log_name(format!("galerie.pagination-{page}-{total}"))
                .show(ui);
            ui.add_space(16.0);
            ui.label(RichText::new(libelle).color(CAPTION).size(12.0));
        });
    }

    heading(
        ui,
        "Survol forcé, et un total qui déborde",
        "preview_hovered peint l'état survolé : hors écran, aucun pointeur ne survole quoi que ce soit. À droite, le bloc s'élargit de lui-même — sa largeur est celle de son libellé, jamais une valeur figée.",
    );
    ui.horizontal(|ui| {
        design::pagination(6, 12)
            .preview_hovered(PaginationStep::Next)
            .log_name("galerie.pagination-survol")
            .show(ui);
        ui.add_space(24.0);
        design::pagination(137, 1482)
            .log_name("galerie.pagination-large")
            .show(ui);
    });
}

/// Charge une fois pour toutes un jeu de textures et le garde en mémoire egui.
///
/// Le chargement a lieu **hors** du verrou de `data_mut` — voir l'appelant.
fn charge_une_fois<T: Clone + Send + Sync + 'static>(
    ui: &egui::Ui,
    cle: &'static str,
    charge: impl FnOnce(&egui::Context) -> T,
) -> T {
    let id = egui::Id::new(cle);
    if let Some(valeur) = ui.data(|d| d.get_temp::<T>(id)) {
        return valeur;
    }
    let valeur = charge(ui.ctx());
    ui.data_mut(|d| d.insert_temp(id, valeur.clone()));
    valeur
}

/// Onglets de la fenêtre de démonstration ci-dessus — un type à part, parce qu'une barre d'onglets
/// est générique sur ce que l'appelant lui donne à sélectionner et qu'une galerie ne doit pas
/// emprunter celui d'un panneau réel pour l'illustrer.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GalleryTab {
    Reglages,
    Avance,
    Indisponible,
}
