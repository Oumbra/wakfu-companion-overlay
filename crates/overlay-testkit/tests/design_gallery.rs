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
    IconContext, InfoTone, InputState, LoaderSize, SelectState, SliderState, TabState,
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
        .with_size(Vec2::new(760.0, 5900.0))
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
                (DsTexture::IconOption, "option"),
                (DsTexture::IconExternalLink, "lien"),
                (DsTexture::IconPlus, "plus"),
                (DsTexture::IconMinus, "moins"),
                (DsTexture::IconVolume, "volume"),
                (DsTexture::IconVolumeMute, "volume-muet"),
                (DsTexture::IconEye, "oeil"),
                (DsTexture::IconEyeOff, "oeil-barre"),
                (DsTexture::IconBagIn, "sac-entree"),
                (DsTexture::IconBagOut, "sac-sortie"),
                (DsTexture::IconFilter, "filtre"),
                (DsTexture::IconLock, "cadenas"),
                (DsTexture::IconOrder, "classement"),
                (DsTexture::IconPact, "pacte"),
                (DsTexture::IconSave, "enregistrer"),
                (DsTexture::IconSort, "tri"),
                (DsTexture::IconTriangleRight, "triangle"),
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

    heading(
        ui,
        "Champ à ornement — la loupe DANS le champ",
        "Le jeu pose toujours sa loupe à l'intérieur du champ, jamais sur un socle à côté. Le composant réserve lui-même la gouttière : la deuxième ligne, la seule qui porte une valeur, est celle qui le vérifie — le texte y démarre après l'icône, pas dessous.",
    );
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
    ui.horizontal_wrapped(|ui| {
        for icon in [
            DsTexture::IconOption,
            DsTexture::IconExternalLink,
            DsTexture::IconPlus,
            DsTexture::IconMinus,
            DsTexture::IconClose,
            DsTexture::IconDelete,
            DsTexture::IconHelp,
            DsTexture::IconUndo,
            DsTexture::IconVolume,
            DsTexture::IconVolumeMute,
            DsTexture::IconEye,
            DsTexture::IconEyeOff,
            DsTexture::IconBagIn,
            DsTexture::IconBagOut,
            DsTexture::IconFilter,
            DsTexture::IconLock,
            DsTexture::IconOrder,
            DsTexture::IconPact,
            DsTexture::IconSave,
            DsTexture::IconSort,
            DsTexture::IconTriangleRight,
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
        "Glyphes sans socle connu — lot sans appelant (2026-09-11)",
        "Les treize icônes du lot du 2026-09-11 dont la mesure `--from-button` n'est pas consignée (voir `DsTexture::icon_content_size`, doc de chaque variante) : à défaut de certitude sur un socle porteur, elles sont peintes ici à leur taille de fichier, en teinte neutre — la même prudence que la rangée du dessus.",
    );
    ui.horizontal_wrapped(|ui| {
        for icon in [
            DsTexture::IconBook,
            DsTexture::IconCalendar,
            DsTexture::IconCards,
            DsTexture::IconCharacters,
            DsTexture::IconGrid,
            DsTexture::IconHammer,
            DsTexture::IconKamas,
            DsTexture::IconPin,
            DsTexture::IconRepeat,
            DsTexture::IconSettings1,
            DsTexture::IconSettings2,
            DsTexture::IconTrophy,
            DsTexture::IconXp,
        ] {
            let native = ds.native_size(icon);
            let (rect, _) =
                ui.allocate_exact_size(native + Vec2::splat(14.0), egui::Sense::hover());
            ds.paint(
                ui.painter(),
                egui::Rect::from_center_size(rect.center(), native),
                icon,
                design::tokens::ICON_TINT,
            );
        }
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
            .icon(DsTexture::IconInfo)
            .log_name("galerie.repliable-ferme")
            .show(ui, |ui| {
                ui.label("jamais rendu tant que le bloc est fermé");
            });
        ui.add_space(10.0);

        let mut ouvert = true;
        design::collapsible("Bloc ouvert", &mut ouvert)
            .icon(DsTexture::IconInfo)
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
            .icon(DsTexture::IconInfo)
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
            .icon(DsTexture::IconInfo)
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
            .icon(DsTexture::IconInfo)
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
