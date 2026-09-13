//! **Maquettes de l'onglet « Chat »** — portage du panneau Chat du dépôt web
//! (`Oumbra/wakfu-companion`, `features/chat-panel`) dans la fenêtre Options de l'overlay, avec le
//! design system du jeu.
//!
//! Même démarche que `alertes-mockups.rs` : les planches sont rendues par les **vrais composants**
//! (`design::window`, `design::tabs`, `design::panel`, `design::checkbox`, `design::collapsible`,
//! `design::input`, `design::select`, `design::button`, `design::icon_button`,
//! `design::info_text`), à la taille réelle de la fenêtre Options, et écrites dans
//! `target/mockups/` — jamais commitées. Seul le contenu (messages, recherches, canaux) est une
//! fixture de ce fichier.
//!
//! ## Ce que le web fait, et ce qui en est repris
//!
//! Le panneau web (`chat-panel.component.*`) lit les messages parsés dans `wakfu.log`
//! (`[Canal] Auteur : message`), sur six canaux — Proximité, Groupe, Guilde, Recrutement, Commerce,
//! Communauté — et offre trois choses :
//!
//! 1. **Masquer des canaux** (boutons à bascule cumulables, persistés).
//! 2. **Des recherches** (mot-clé + canal ou « Global ») : un message qui correspond est **mis en
//!    évidence et déclenche un son** — il n'est jamais masqué. Persistées.
//! 3. **Le fil lui-même**, en défilement automatique tant qu'on est en bas ; dès qu'on remonte,
//!    l'auto-défilement s'arrête et un bouton « Aller en bas » apparaît.
//!
//! Les trois sont portés. Ce qui ne l'est pas : le bouton de repli du panneau (la fenêtre Options a
//! sa propre fermeture), le `?` d'aide (pas de modale d'aide dans l'overlay), le badge de non-lus
//! du rail (il n'y a pas de rail).
//!
//! ## Les décisions de design, et pourquoi
//!
//! | Web | Ici | Raison |
//! | --- | --- | --- |
//! | Boutons-jetons colorés pour les canaux | **Cases à cocher**, précédées d'une pastille de la couleur du canal | Le jeu n'a pas de jeton coloré ; son idiome pour « afficher / masquer » est la case (`interface-options-chat.png`, « Avoir accès aux canaux des langues suivantes »). La pastille sert de légende des couleurs du fil |
//! | Bandeau « Recherche » repliable, à dégradé animé | **`design::collapsible`**, replié par défaut, compte de recherches dans le titre | Même comportement (replié à l'ouverture), mais avec le bloc repliable du jeu — pas d'animation, le jeu n'en a pas |
//! | Jetons de filtre (chips) | **Lignes de réglage** (`SETTING_ROW_FILL`, comme « Fermeture automatique » dans Alertes) : pastille, canal, mot-clé, croix | Le jeu n'a pas de chip ; ses listes de réglages sont des lignes sur aplat `#26282b` |
//! | Carte à deux lignes par message, barre gauche colorée | **Une ligne par message, à la couleur du canal**, heure en gris — comme le chat du jeu lui-même | C'est le format que le joueur lit déjà dans le jeu ; une variante « cartes » est fournie pour comparer ([`chat_options_fil_cartes`]) |
//! | Surbrillance `linear-gradient` à la couleur du canal | Même dégradé, en maillage egui, plus une barre gauche de 3 px | Fidèle au web, et lisible sur une ligne unique |
//! | Bouton flottant « Aller en bas », en pilule | **`design::button`** secondaire compact, posé au bas du fil | Pas de pilule dans le jeu |
//! | Couleurs de canal du web (`--channel-*`) | **Reprises telles quelles** | Hypothèse à confirmer : le chat du jeu a ses propres couleurs par canal, non relevées à ce jour (aucune capture du chat en jeu dans `assets/design-system/interfaces/`) |
//!
//! **L'or ne marque jamais une hiérarchie, seulement un état** (leçon des maquettes Alertes) :
//! aucun titre n'est doré ici. La couleur Commerce du web (`#ffd700`) est proche de l'or d'état du
//! design system (`#f4d89e`) — c'est un point à trancher au portage (voir « Ce qu'il reste à
//! décider »).
//!
//! ## Le contrat transactionnel, et ce qu'il fait au chat
//!
//! La fenêtre Options est transactionnelle (§5.1 du plan) : rien n'est pris en compte tant que
//! « Valider » n'a pas été cliqué. **Cela vaut pour les réglages** — canaux cochés, recherches — qui
//! forment un brouillon comme `AlertProfile` dans Alertes. **Cela ne vaut pas pour le fil**, qui
//! n'est pas un réglage : il est vivant, il défile pendant que la fenêtre est ouverte, et le moteur
//! le remplit que la fenêtre soit ouverte ou non. Le son d'une recherche, lui aussi, doit jouer
//! fenêtre fermée — c'est tout l'intérêt de la fonctionnalité pour un joueur qui a le jeu sous les
//! yeux.
//!
//! ## Ce qu'il reste à décider avant portage
//!
//! 1. **Les couleurs de canal** — reprendre celles du web (ce que font ces planches), ou relever
//!    celles du chat du jeu sur une capture qui reste à fournir ?
//! 2. **Le format de ligne** — une ligne façon jeu ([`chat_options_onglet`]) ou une carte façon web
//!    ([`chat_options_fil_cartes`]) ?
//! 3. **La position de l'onglet** — troisième, entre Alertes et Personnages (ce que font ces
//!    planches), ou ailleurs ?
//! 4. **Le plafond de messages** — le web garde 2 000 messages ; le plan d'architecture prévoit un
//!    « chat borné à N messages » sans fixer N.
//!
//! ## Ce que ce portage demande, et qui n'existe pas encore
//!
//! 1. **`design::scroll_area::stick_to_bottom`** — le composant n'expose que `auto_shrink` ; le fil
//!    doit coller au bas tant qu'on n'a pas remonté. Ces planches y arrivent en simulant la molette.
//! 2. **Un `SessionSnapshot.chat`** borné — le moteur parse déjà `LogEntry::Chat` mais le jette
//!    (`session.rs`, « hors périmètre de ce premier slice »).
//! 3. **Une icône « bulle »** pour un futur onglet à pictogramme — aucune dans le registre ; ces
//!    planches restent en libellé texte comme les quatre autres onglets.
//! 4. **Le son de recherche** — `alert_sound.rs` ne porte que le son de ramassage ; le web a un
//!    troisième son (`chat-filter-*.mp3`).
//! 5. **La persistance** — `chatActiveChannels` / `chatFilters` sont synchronisés au compte côté web
//!    (`PATCH /api/v1/settings`), comme `profile.soundItems` pour Alertes.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, Rect, RichText, Vec2};
use egui_kittest::Harness;
use overlay_ui::design::{self, ButtonSize, ButtonVariant, DsIcon, IconContext, InputSize};
use overlay_ui::panels::options_modal;
use overlay_ui::ui_icons::UiIcons;

// -------------------------------------------------------------------------------------------
// Jetons — chacun dit d'où il vient. Les valeurs reprises des onglets existants le sont TELLES
// QUELLES (voir `alertes-mockups.rs` sur ce que coûte une reprise approximative).
// -------------------------------------------------------------------------------------------

/// Fond derrière la fenêtre — le même que `tests/panels.rs` et `alertes-mockups.rs`.
const BACKDROP: Color32 = Color32::from_rgb(0x0B, 0x0D, 0x10);

/// Texte courant — blanc, comme `panels::alerts_tab::TEXT`.
const TEXT: Color32 = Color32::WHITE;

/// Le gris unique du jeu — `panels::alerts_tab::SUBDUED`, `#b8b9ba`.
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);

/// Fond d'une ligne de réglage — `panels::alerts_tab::SETTING_ROW_FILL`, mesuré sur
/// `interface-personnage-aptitudes.png`.
const SETTING_ROW_FILL: Color32 = Color32::from_rgb(0x26, 0x28, 0x2B);
const SETTING_ROW_RADIUS: u8 = 4;

/// Corps de texte des onglets — `panels::alerts_tab::BODY_FONT_SIZE`.
const BODY_FONT_SIZE: f32 = 15.0;

/// Aération autour d'un titre de section — `panels::alerts_tab::SECTION_GAP`.
const SECTION_GAP: f32 = 18.0;

/// Taille de la fenêtre Options — celle de la production, sans agrandissement.
const WINDOW: Vec2 = Vec2::new(options_modal::WINDOW_SIZE.0, options_modal::WINDOW_SIZE.1);

/// Corps de texte du fil — **plus petit que le corps des réglages** : c'est un flux dense, comme le
/// chat du jeu, dont la police par défaut est nettement plus petite que celle des libellés
/// d'options (`interface-options-chat.png` propose d'ailleurs un réglage « Taille de la police »).
const MESSAGE_FONT_SIZE: f32 = 14.0;

/// Corps de l'heure, un cran sous le message : c'est une métadonnée.
const TIME_FONT_SIZE: f32 = 12.0;

/// Rembourrage vertical d'une ligne de message.
const MESSAGE_PAD_Y: f32 = 3.0;

/// Rembourrage horizontal d'une ligne — laisse la place à la barre de surbrillance.
const MESSAGE_PAD_X: f32 = 8.0;

/// Épaisseur de la barre gauche d'un message mis en évidence — celle du web (`border-left: 3px`).
const HIT_BAR_WIDTH: f32 = 3.0;

/// Opacité du départ du dégradé de surbrillance — le `35%` du `color-mix` web.
const HIT_ALPHA: u8 = 89;

/// Où le dégradé de surbrillance s'éteint — le `transparent 75%` du web.
const HIT_FADE_END: f32 = 0.75;

/// Côté de la pastille de couleur devant une case de canal, et son arrondi.
const SWATCH: f32 = 10.0;
const SWATCH_RADIUS: u8 = 2;
const SWATCH_GAP: f32 = 7.0;

/// Hauteur d'une ligne de la grille de canaux.
const CHANNEL_ROW_HEIGHT: f32 = 30.0;

/// Colonnes de la grille de canaux — six canaux en deux rangées de trois : une seule rangée ne tient
/// pas dans la largeur du panneau avec des libellés à 15 px, et six rangées gaspilleraient la
/// hauteur qui revient au fil.
const CHANNEL_COLUMNS: usize = 3;

/// Hauteur d'une ligne de recherche enregistrée.
const FILTER_ROW_HEIGHT: f32 = 36.0;

/// Fond d'une **carte** de message (variante web) — le `--chat-msg-bg` du web est `#252525`, ici
/// l'aplat de ligne de réglage du jeu, à trois niveaux près : même rôle, même valeur que le reste
/// de la fenêtre.
const CARD_FILL: Color32 = SETTING_ROW_FILL;
const CARD_RADIUS: u8 = 4;
const CARD_GAP: f32 = 6.0;

// -------------------------------------------------------------------------------------------
// Canaux — les six du web, dans l'ordre du web, aux couleurs du web (thème sombre).
// -------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Channel {
    Proximite,
    Groupe,
    Guilde,
    Recrutement,
    Commerce,
    Communaute,
}

impl Channel {
    const ALL: [Channel; 6] = [
        Channel::Proximite,
        Channel::Groupe,
        Channel::Guilde,
        Channel::Recrutement,
        Channel::Commerce,
        Channel::Communaute,
    ];

    fn label(self) -> &'static str {
        match self {
            Channel::Proximite => "Proximité",
            Channel::Groupe => "Groupe",
            Channel::Guilde => "Guilde",
            Channel::Recrutement => "Recrutement",
            Channel::Commerce => "Commerce",
            Channel::Communaute => "Communauté",
        }
    }

    /// `--channel-*` du web, thème sombre (`styles.css`). Hypothèse à confirmer contre le jeu.
    fn color(self) -> Color32 {
        match self {
            Channel::Proximite => Color32::from_rgb(0xD8, 0xD8, 0xD8),
            Channel::Groupe => Color32::from_rgb(0x00, 0xD2, 0xFF),
            Channel::Guilde => Color32::from_rgb(0xB4, 0x8C, 0xFF),
            Channel::Recrutement => Color32::from_rgb(0x2E, 0xCC, 0x71),
            Channel::Commerce => Color32::from_rgb(0xFF, 0xD7, 0x00),
            Channel::Communaute => Color32::from_rgb(0x5D, 0xAD, 0xE2),
        }
    }
}

/// Cible d'une recherche : un canal, ou tous (« Global » côté web).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterScope {
    Global,
    Channel(Channel),
}

impl FilterScope {
    fn label(self) -> &'static str {
        match self {
            FilterScope::Global => "Global",
            FilterScope::Channel(c) => c.label(),
        }
    }

    /// Une recherche « Global » n'a pas de couleur de canal : le gris du jeu, pour ne pas la
    /// confondre avec Groupe, dont la couleur web est celle de l'accent.
    fn color(self) -> Color32 {
        match self {
            FilterScope::Global => SUBDUED,
            FilterScope::Channel(c) => c.color(),
        }
    }
}

struct Filter {
    text: &'static str,
    scope: FilterScope,
}

/// Les recherches de la fixture — une par canal, une globale.
const FILTERS: [Filter; 2] = [
    Filter {
        text: "gelano",
        scope: FilterScope::Channel(Channel::Commerce),
    },
    Filter {
        text: "donjon",
        scope: FilterScope::Global,
    },
];

struct Message {
    time: &'static str,
    channel: Channel,
    author: &'static str,
    text: &'static str,
}

/// Même règle que `ChatPanelService.messageMatchesAnyFilter` du web : une recherche s'applique si
/// son canal est « Global » ou celui du message, et compare en minuscules sur le texte OU l'auteur.
fn is_hit(msg: &Message, filters: &[Filter]) -> bool {
    filters.iter().any(|f| {
        let applies = match f.scope {
            FilterScope::Global => true,
            FilterScope::Channel(c) => c == msg.channel,
        };
        applies
            && (msg.text.to_lowercase().contains(f.text)
                || msg.author.to_lowercase().contains(f.text))
    })
}

/// Trente messages — noms de personnages inventés, contenu plausible pour chaque canal, et deux
/// occurrences des recherches de la fixture (une par recherche, dans le bas du fil pour qu'elles
/// soient dans le champ quand le fil est collé au bas).
const MESSAGES: [Message; 30] = [
    Message {
        time: "21:02:11",
        channel: Channel::Proximite,
        author: "Kralamoure-Fan",
        text: "quelqu'un pour le boss de la zone ?",
    },
    Message {
        time: "21:02:34",
        channel: Channel::Guilde,
        author: "Sramounette",
        text: "gg pour le niveau 200 Yoplait !",
    },
    Message {
        time: "21:02:41",
        channel: Channel::Guilde,
        author: "Yoplait-Fraise",
        text: "merci merci, enfin",
    },
    Message {
        time: "21:03:05",
        channel: Channel::Commerce,
        author: "Marchand-Bonta",
        text: "vends 50 Bois de Bouleau 120k le lot, mp",
    },
    Message {
        time: "21:03:18",
        channel: Channel::Groupe,
        author: "Iopette",
        text: "je lance le combat, tout le monde est prêt ?",
    },
    Message {
        time: "21:03:20",
        channel: Channel::Groupe,
        author: "Xelor-du-Nord",
        text: "go",
    },
    Message {
        time: "21:03:22",
        channel: Channel::Groupe,
        author: "Osaboss",
        text: "attends je change de stuff",
    },
    Message {
        time: "21:03:50",
        channel: Channel::Recrutement,
        author: "Capitaine-Flam",
        text: "guilde Les Pieds Nickelés recrute niveau 150+, ambiance détente, discord actif",
    },
    Message {
        time: "21:04:02",
        channel: Channel::Communaute,
        author: "Enutrof-Radin",
        text: "c'est quand la prochaine mise à jour ?",
    },
    Message {
        time: "21:04:09",
        channel: Channel::Communaute,
        author: "Panda-Dodu",
        text: "la semaine prochaine normalement",
    },
    Message {
        time: "21:04:31",
        channel: Channel::Proximite,
        author: "Feca-Fana",
        text: "y a un archi juste à côté du zaap",
    },
    Message {
        time: "21:04:45",
        channel: Channel::Commerce,
        author: "Ecaflip-Chanceux",
        text: "achète Pierre de Kamas x100, 8k l'unité",
    },
    Message {
        time: "21:05:10",
        channel: Channel::Guilde,
        author: "Sramounette",
        text: "on fait le donjon du Wa Wabbit ce soir ? il me manque 3 runs",
    },
    Message {
        time: "21:05:15",
        channel: Channel::Guilde,
        author: "Cra-Cra",
        text: "moi je suis chaud",
    },
    Message {
        time: "21:05:16",
        channel: Channel::Guilde,
        author: "Eni-Douce",
        text: "partante aussi",
    },
    Message {
        time: "21:05:40",
        channel: Channel::Groupe,
        author: "Iopette",
        text: "bien joué, on enchaîne la salle suivante",
    },
    Message {
        time: "21:06:02",
        channel: Channel::Recrutement,
        author: "Zobal-Masqué",
        text: "cherche guilde PvM active, niveau 180 Zobal",
    },
    Message {
        time: "21:06:25",
        channel: Channel::Proximite,
        author: "Ouginak-Grognon",
        text: "merci pour le coup de main !",
    },
    Message {
        time: "21:06:50",
        channel: Channel::Commerce,
        author: "Marchand-Bonta",
        text: "toujours dispo les 50 Bois de Bouleau",
    },
    Message {
        time: "21:07:12",
        channel: Channel::Communaute,
        author: "Sadida-Vert",
        text: "quelqu'un connaît le drop de la Larve Dorée ?",
    },
    Message {
        time: "21:07:30",
        channel: Channel::Communaute,
        author: "Rogue-Rouge",
        text: "c'est dans la Forêt des Abrakneuses, taux faible",
    },
    Message {
        time: "21:07:58",
        channel: Channel::Groupe,
        author: "Osaboss",
        text: "j'ai plus de PA, je passe",
    },
    Message {
        time: "21:08:14",
        channel: Channel::Guilde,
        author: "Yoplait-Fraise",
        text: "il reste des places pour le donjon Blop ?",
    },
    Message {
        time: "21:08:40",
        channel: Channel::Commerce,
        author: "Huppermage-Bleu",
        text: "vends Gelano 900k, prix ferme",
    },
    Message {
        time: "21:08:55",
        channel: Channel::Proximite,
        author: "Steamer-Rouillé",
        text: "gg",
    },
    Message {
        time: "21:09:10",
        channel: Channel::Recrutement,
        author: "Capitaine-Flam",
        text: "toujours 2 places dans Les Pieds Nickelés",
    },
    Message {
        time: "21:09:33",
        channel: Channel::Groupe,
        author: "Xelor-du-Nord",
        text: "on refait un run ou on arrête là ?",
    },
    Message {
        time: "21:09:36",
        channel: Channel::Groupe,
        author: "Iopette",
        text: "un dernier et dodo",
    },
    Message {
        time: "21:10:04",
        channel: Channel::Commerce,
        author: "Ecaflip-Chanceux",
        text: "cherche Gelano pas cher, mp vos prix",
    },
    Message {
        time: "21:10:21",
        channel: Channel::Communaute,
        author: "Panda-Dodu",
        text: "bonne nuit tout le monde",
    },
];

// -------------------------------------------------------------------------------------------
// Le harnais — chrome réel, onglet « Chat » actif, entre Alertes et Personnages.
// -------------------------------------------------------------------------------------------

/// Les onglets de la fenêtre Options **tels que ces planches les proposent** : `OptionsTab` n'a
/// pas encore de variante Chat, et `design::tabs` est générique — cette énumération locale suffit
/// à rendre la barre à cinq entrées sans toucher au crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Onglet {
    Suivi,
    Alertes,
    Chat,
    Personnages,
    Parametres,
}

fn mockup_dir() -> std::path::PathBuf {
    static PURGE: std::sync::Once = std::sync::Once::new();
    let dir = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(target) => std::path::PathBuf::from(target),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"),
    }
    .join("mockups");
    PURGE.call_once(|| {
        let _ = std::fs::remove_dir_all(&dir);
    });
    std::fs::create_dir_all(&dir).expect("création de target/mockups");
    dir
}

fn write_mockup(harness: &mut Harness<'static>, name: &str) {
    let image = harness
        .render()
        .expect("rendu offscreen — voir doc de module");
    image
        .save(mockup_dir().join(format!("{name}.png")))
        .expect("écriture de la capture");
}

/// Ouvre le harnais et rend la fenêtre Options avec son chrome réel, l'onglet « Chat » actif.
///
/// Le chrome est **appelé**, jamais recopié : `design::window`, `design::tabs`, `design::panel` —
/// les mêmes composants que `panels::options_modal`.
fn options_harness(
    mut build: impl FnMut(&mut egui::Ui, &UiIcons, &design::PanelZones) + 'static,
) -> Harness<'static> {
    let mut icons: Option<UiIcons> = None;
    let mut tab = Onglet::Chat;
    Harness::builder()
        .with_size(WINDOW)
        // Le défilement simulé (voir `scroll_to_bottom`) demande quelques frames de plus que les
        // quatre par défaut.
        .with_max_steps(64)
        .build_ui(move |ui| {
            overlay_ui::style::apply(ui.ctx());
            // Aucune animation de défilement : deux rendus du même écran doivent donner le même
            // pixel.
            ui.style_mut().scroll_animation = egui::style::ScrollAnimation::none();
            ui.style_mut().visuals.text_cursor.blink = false;
            let icons = icons.get_or_insert_with(|| UiIcons::load(ui.ctx()));
            egui::Frame::NONE.fill(BACKDROP).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                let chrome = design::window("Options")
                    .footer("Annuler", "Valider")
                    .log_name("maquette")
                    .show(ui);
                chrome.tabs(
                    ui,
                    design::tabs(&mut tab)
                        .entry(Onglet::Suivi, "Suivi")
                        .entry(Onglet::Alertes, "Alertes")
                        .entry(Onglet::Chat, "Chat")
                        .entry(Onglet::Personnages, "Personnages")
                        .enabled(false)
                        .entry(Onglet::Parametres, "Paramètres")
                        .log_name("maquette-onglets"),
                );
                design::panel().show(ui, chrome.content, |ui, panel| {
                    build(ui, icons, panel);
                });
            });
        })
}

/// Colle le fil au bas, comme le ferait `stick_to_bottom` une fois porté au composant : la molette
/// est simulée au-dessus du fil, avec un débattement bien supérieur à sa hauteur.
fn scroll_to_bottom(harness: &mut Harness<'static>) {
    harness.run();
    harness.hover_at(egui::pos2(WINDOW.x * 0.5, WINDOW.y - 160.0));
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -20_000.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    // Le pointeur repart hors de la fenêtre : aucun survol ne doit rester sur la capture.
    harness.hover_at(egui::pos2(-10.0, -10.0));
    harness.run();
}

// -------------------------------------------------------------------------------------------
// L'onglet
// -------------------------------------------------------------------------------------------

/// Comment une ligne du fil est peinte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowStyle {
    /// Une ligne par message, à la couleur du canal — comme le chat du jeu.
    Game,
    /// Une carte à deux lignes, barre gauche colorée — comme le panneau web.
    Card,
}

struct ChatTab<'a> {
    /// Un booléen par canal, dans l'ordre de [`Channel::ALL`].
    visible: &'a mut [bool; 6],
    filters: &'a [Filter],
    /// Le bloc « Recherches » est-il déplié ?
    search_open: &'a mut bool,
    search_input: &'a mut String,
    search_scope: &'a mut FilterScope,
    messages: &'a [Message],
    row_style: RowStyle,
    /// Le fil n'est pas collé au bas : le bouton « Aller en bas » est visible.
    scrolled_up: bool,
}

const DESC: &str = "Les messages du chat lus dans wakfu.log. Décochez un canal pour le masquer ; \
                    une recherche met un message en évidence et joue un son, sans rien masquer.";

/// Un paragraphe, pas un bloc d'information — même règle que `panels::alerts_tab::paragraph`.
fn paragraph(ui: &mut egui::Ui, text: &str) {
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .color(TEXT)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        )
        .wrap_mode(egui::TextWrapMode::Wrap),
    );
}

fn chat_tab(ui: &mut egui::Ui, panel: &design::PanelZones, tab: &mut ChatTab<'_>) {
    let width = panel.inner.width();

    ui.add(design::heading("Chat").trailing_gap(SECTION_GAP * 0.5));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP * 0.75);

    channel_grid(ui, tab.visible, width);
    ui.add_space(SECTION_GAP * 0.75);

    search_block(ui, tab, width);
    ui.add_space(SECTION_GAP * 0.75);

    let shown: Vec<&Message> = tab
        .messages
        .iter()
        .filter(|m| {
            tab.visible[Channel::ALL
                .iter()
                .position(|c| *c == m.channel)
                .unwrap_or(0)]
        })
        .collect();
    feed_header(ui, width, shown.len(), tab.messages.len());

    if shown.is_empty() {
        ui.add_space(SECTION_GAP * 0.5);
        ui.add(
            design::info_text("Aucun message pour le moment.")
                .width(width)
                .log_name("chat.vide"),
        );
        return;
    }

    let feed_top = ui.cursor().min.y;
    panel.scroll_area(ui, "chat.fil", |ui, content_width| {
        ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);
        for msg in &shown {
            let hit = is_hit(msg, tab.filters);
            match tab.row_style {
                RowStyle::Game => message_row_game(ui, msg, hit, content_width),
                RowStyle::Card => message_card(ui, msg, hit, content_width),
            }
        }
    });

    if tab.scrolled_up {
        // Posé au bas du fil, centré — dans un enfant : `ui.put` sur le `ui` du panneau avancerait
        // son curseur, et c'est tout ce qui suit qui glisserait.
        let feed = Rect::from_min_max(
            egui::pos2(panel.inner.left(), feed_top),
            panel.inner.right_bottom(),
        );
        let button = design::button("Aller en bas")
            .variant(ButtonVariant::Secondary)
            .size(ButtonSize::Compact)
            .width(150.0)
            .tooltip("Reprendre le défilement automatique")
            .log_name("chat.aller-en-bas");
        let size = Vec2::new(150.0, ButtonSize::Compact.height());
        let rect = Rect::from_center_size(
            egui::pos2(feed.center().x, feed.bottom() - size.y * 0.5 - 8.0),
            size,
        );
        // Un voile du fond de panneau sous le bouton : sans lui, il se lit comme un mot de plus
        // dans la ligne qu'il recouvre. Le fond de fenêtre, pas un noir arbitraire.
        ui.painter().rect_filled(
            rect.expand2(Vec2::new(10.0, 6.0)),
            SETTING_ROW_RADIUS,
            Color32::from_rgba_unmultiplied(0x15, 0x18, 0x1C, 220),
        );
        let mut floating = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        floating.put(rect, button);
    }
}

/// Six cases en deux rangées de trois, chacune précédée de la pastille de couleur de son canal.
///
/// La pastille fait double emploi, et c'est voulu : elle dit quelle couleur ce canal prend dans le
/// fil (légende) et rend la rangée lisible d'un coup d'œil sans lire les libellés.
fn channel_grid(ui: &mut egui::Ui, visible: &mut [bool; 6], width: f32) {
    let column = width / CHANNEL_COLUMNS as f32;
    for (row_index, row) in Channel::ALL.chunks(CHANNEL_COLUMNS).enumerate() {
        let rect = ui.allocate_space(Vec2::new(width, CHANNEL_ROW_HEIGHT)).1;
        for (col, channel) in row.iter().enumerate() {
            let cell = Rect::from_min_size(
                egui::pos2(rect.left() + column * col as f32, rect.top()),
                Vec2::new(column, CHANNEL_ROW_HEIGHT),
            );
            let mut cell_ui = ui.new_child(egui::UiBuilder::new().max_rect(cell));
            cell_ui.horizontal_centered(|ui| {
                let (swatch, _) = ui.allocate_exact_size(Vec2::splat(SWATCH), egui::Sense::hover());
                ui.painter()
                    .rect_filled(swatch, SWATCH_RADIUS, channel.color());
                ui.add_space(SWATCH_GAP);
                let index = row_index * CHANNEL_COLUMNS + col;
                ui.add(
                    design::checkbox(&mut visible[index], channel.label())
                        .log_name(format!("chat.canal.{}", channel.label())),
                );
            });
        }
    }
}

/// Le bloc « Recherches », repliable — replié par défaut comme le bandeau « Recherche » du web.
fn search_block(ui: &mut egui::Ui, tab: &mut ChatTab<'_>, width: f32) {
    let title = if tab.filters.is_empty() {
        "Recherches".to_owned()
    } else {
        format!("Recherches ({})", tab.filters.len())
    };
    design::collapsible(title, tab.search_open)
        .icon(DsIcon::Search)
        .log_name("chat.recherches")
        .show(ui, |ui| {
            let inner_width = design::collapsible_content_width(width);
            ui.add_space(4.0);
            search_input_row(ui, tab.search_input, tab.search_scope, inner_width);
            if !tab.filters.is_empty() {
                ui.add_space(8.0);
                for filter in tab.filters {
                    filter_row(ui, filter, inner_width);
                    ui.add_space(4.0);
                }
            }
            ui.add_space(4.0);
        });
}

/// Champ de mot-clé, choix du canal, bouton « Ajouter » — les trois du web, sur une ligne.
fn search_input_row(ui: &mut egui::Ui, input: &mut String, scope: &mut FilterScope, width: f32) {
    const SCOPE_WIDTH: f32 = 150.0;
    const ADD_WIDTH: f32 = 110.0;
    const GAP: f32 = 8.0;
    let row_height = design::tokens::SELECT_HEIGHT;
    let row = ui.allocate_space(Vec2::new(width, row_height)).1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    cell.horizontal_centered(|ui| {
        ui.add(
            design::input(input)
                .leading_icon(DsIcon::Search)
                .size(InputSize::Search)
                .clearable(true)
                .placeholder("Rechercher les messages…")
                .width(width - SCOPE_WIDTH - ADD_WIDTH - GAP * 2.0)
                .log_name("chat.recherche"),
        );
        ui.add_space(GAP);
        let mut select = design::select(scope)
            .option(FilterScope::Global, "Global")
            .width(SCOPE_WIDTH)
            .log_name("chat.recherche.canal");
        for channel in Channel::ALL {
            select = select.option(FilterScope::Channel(channel), channel.label());
        }
        select.show(ui);
        ui.add_space(GAP);
        ui.add(
            design::button("Ajouter")
                .variant(ButtonVariant::Primary)
                .size(ButtonSize::Compact)
                .width(ADD_WIDTH)
                .log_name("chat.recherche.ajouter"),
        );
    });
}

/// Une recherche enregistrée : pastille, canal, mot-clé, croix de retrait — sur l'aplat de ligne
/// de réglage du jeu, comme « Fermeture automatique » dans Alertes.
fn filter_row(ui: &mut egui::Ui, filter: &Filter, width: f32) {
    let row = ui.allocate_space(Vec2::new(width, FILTER_ROW_HEIGHT)).1;
    ui.painter()
        .rect_filled(row, SETTING_ROW_RADIUS, SETTING_ROW_FILL);
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row.shrink2(Vec2::new(10.0, 0.0))));
    cell.horizontal_centered(|ui| {
        let (swatch, _) = ui.allocate_exact_size(Vec2::splat(SWATCH), egui::Sense::hover());
        ui.painter()
            .rect_filled(swatch, SWATCH_RADIUS, filter.scope.color());
        ui.add_space(SWATCH_GAP);
        ui.label(
            RichText::new(filter.scope.label())
                .color(SUBDUED)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        );
        ui.add_space(10.0);
        ui.label(
            RichText::new(filter.text)
                .color(TEXT)
                .font(design::text::label_strong_font(ui.ctx(), BODY_FONT_SIZE)),
        );
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row.shrink2(Vec2::new(4.0, 0.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        right.add(
            design::icon_button(DsIcon::Close)
                .context(IconContext::Panel)
                .size(28.0)
                .tooltip("Retirer")
                .log_name(format!("chat.recherche.retirer.{}", filter.text)),
        );
    });
}

/// Titre « Messages » à gauche, compte à droite — « 24 / 30 » dit combien le masquage de canaux
/// retire, sans autre commande : le web n'a ni « effacer » ni « pause », et ces planches non plus.
fn feed_header(ui: &mut egui::Ui, width: f32, shown: usize, total: usize) {
    let row = ui.allocate_space(Vec2::new(width, 34.0)).1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    cell.horizontal_centered(|ui| {
        ui.add(design::heading("Messages"));
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        // À zéro, rien : le bloc d'information du fil le dit déjà, un « 0 messages » en face
        // serait une redite.
        let count = match (shown, total) {
            (_, 0) => return,
            (s, t) if s == t => format!("{t} messages"),
            (s, t) => format!("{s} / {t} messages"),
        };
        right.label(
            RichText::new(count)
                .color(SUBDUED)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        );
    });
}

/// Le dégradé de surbrillance du web, en maillage : `alpha 35 %` de la couleur du canal à gauche,
/// éteint aux trois quarts de la largeur — plus la barre gauche de 3 px.
fn paint_hit(painter: &egui::Painter, rect: Rect, color: Color32) {
    let start = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), HIT_ALPHA);
    let end = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0);
    let fade_x = rect.left() + rect.width() * HIT_FADE_END;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), start);
    mesh.colored_vertex(rect.left_bottom(), start);
    mesh.colored_vertex(egui::pos2(fade_x, rect.top()), end);
    mesh.colored_vertex(egui::pos2(fade_x, rect.bottom()), end);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(mesh);
    painter.rect_filled(
        Rect::from_min_size(rect.left_top(), Vec2::new(HIT_BAR_WIDTH, rect.height())),
        0.0,
        color,
    );
}

/// Une ligne façon jeu : `21:08:40 [Commerce] Huppermage-Bleu : vends Gelano 900k` — l'heure en
/// gris, tout le reste à la couleur du canal, l'auteur en graisse.
fn message_row_game(ui: &mut egui::Ui, msg: &Message, hit: bool, width: f32) {
    let ctx = ui.ctx().clone();
    let color = msg.channel.color();
    let mut job = LayoutJob::default();
    job.wrap.max_width = width - MESSAGE_PAD_X * 2.0;
    job.append(
        &format!("{}  ", msg.time),
        0.0,
        TextFormat {
            font_id: design::text::label_font(&ctx, TIME_FONT_SIZE),
            color: SUBDUED,
            ..Default::default()
        },
    );
    job.append(
        &format!("[{}] ", msg.channel.label()),
        0.0,
        TextFormat {
            font_id: design::text::label_font(&ctx, MESSAGE_FONT_SIZE),
            color,
            ..Default::default()
        },
    );
    job.append(
        &format!("{} : ", msg.author),
        0.0,
        TextFormat {
            font_id: design::text::label_strong_font(&ctx, MESSAGE_FONT_SIZE),
            color,
            ..Default::default()
        },
    );
    job.append(
        msg.text,
        0.0,
        TextFormat {
            font_id: design::text::label_font(&ctx, MESSAGE_FONT_SIZE),
            color,
            ..Default::default()
        },
    );
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let height = galley.size().y + MESSAGE_PAD_Y * 2.0;
    let rect = ui.allocate_space(Vec2::new(width, height)).1;
    if hit {
        paint_hit(ui.painter(), rect, color);
    }
    ui.painter().galley(
        rect.left_top() + Vec2::new(MESSAGE_PAD_X, MESSAGE_PAD_Y),
        galley,
        color,
    );
}

/// Une carte façon web : heure + `[Canal]` + auteur sur la première ligne, message sur la seconde,
/// aplat sombre et barre gauche à la couleur du canal.
fn message_card(ui: &mut egui::Ui, msg: &Message, hit: bool, width: f32) {
    let ctx = ui.ctx().clone();
    let color = msg.channel.color();
    let inner = width - MESSAGE_PAD_X * 2.0 - HIT_BAR_WIDTH;

    let mut meta = LayoutJob::default();
    meta.wrap.max_width = inner;
    meta.append(
        &format!("{}  ", msg.time),
        0.0,
        TextFormat {
            font_id: design::text::label_font(&ctx, TIME_FONT_SIZE),
            color: SUBDUED,
            ..Default::default()
        },
    );
    meta.append(
        &format!("[{}]  ", msg.channel.label()),
        0.0,
        TextFormat {
            font_id: design::text::label_strong_font(&ctx, TIME_FONT_SIZE + 1.0),
            color,
            ..Default::default()
        },
    );
    meta.append(
        msg.author,
        0.0,
        TextFormat {
            font_id: design::text::label_strong_font(&ctx, TIME_FONT_SIZE + 1.0),
            color: TEXT,
            ..Default::default()
        },
    );
    let mut body = LayoutJob::default();
    body.wrap.max_width = inner;
    body.append(
        msg.text,
        0.0,
        TextFormat {
            font_id: design::text::label_font(&ctx, MESSAGE_FONT_SIZE),
            color: TEXT,
            ..Default::default()
        },
    );
    let meta = ui.fonts_mut(|f| f.layout_job(meta));
    let body = ui.fonts_mut(|f| f.layout_job(body));
    let height = meta.size().y + body.size().y + MESSAGE_PAD_Y * 2.0 + 2.0;
    let rect = ui.allocate_space(Vec2::new(width, height + CARD_GAP)).1;
    let card = Rect::from_min_size(rect.left_top(), Vec2::new(width, height));
    ui.painter().rect_filled(card, CARD_RADIUS, CARD_FILL);
    if hit {
        paint_hit(ui.painter(), card, color);
    } else {
        // Barre gauche : la couleur du canal, comme le `border-left` du web — la carte la porte
        // toujours, la surbrillance ne fait que l'accompagner d'un dégradé.
        ui.painter().rect_filled(
            Rect::from_min_size(card.left_top(), Vec2::new(HIT_BAR_WIDTH, card.height())),
            0.0,
            color,
        );
    }
    let origin = card.left_top() + Vec2::new(MESSAGE_PAD_X + HIT_BAR_WIDTH, MESSAGE_PAD_Y);
    let meta_height = meta.size().y;
    ui.painter().galley(origin, meta, TEXT);
    ui.painter()
        .galley(origin + Vec2::new(0.0, meta_height + 2.0), body, TEXT);
}

// -------------------------------------------------------------------------------------------
// Les planches
// -------------------------------------------------------------------------------------------

/// Construit un harnais avec l'état fourni. La closure de rendu doit posséder son état : d'où les
/// `move` et les `'static`.
#[allow(clippy::too_many_arguments)]
fn chat_harness(
    visible: [bool; 6],
    search_open: bool,
    row_style: RowStyle,
    messages: &'static [Message],
    filters: &'static [Filter],
    scrolled_up: bool,
) -> Harness<'static> {
    let mut visible = visible;
    let mut search_open = search_open;
    let mut search_input = String::new();
    let mut search_scope = FilterScope::Global;
    options_harness(move |ui, _icons, panel| {
        chat_tab(
            ui,
            panel,
            &mut ChatTab {
                visible: &mut visible,
                filters,
                search_open: &mut search_open,
                search_input: &mut search_input,
                search_scope: &mut search_scope,
                messages,
                row_style,
                scrolled_up,
            },
        );
    })
}

const ALL_VISIBLE: [bool; 6] = [true; 6];

/// **1 — L'onglet, au repos.** Six canaux cochés, recherches repliées (deux enregistrées), fil collé
/// au bas, façon jeu. Deux messages mis en évidence : « Gelano » sur Commerce, « donjon » partout.
fn chat_options_onglet() {
    let mut harness = chat_harness(
        ALL_VISIBLE,
        false,
        RowStyle::Game,
        &MESSAGES,
        &FILTERS,
        false,
    );
    scroll_to_bottom(&mut harness);
    write_mockup(&mut harness, "chat_options_onglet");
}

/// **2 — Les recherches dépliées.** Champ, choix du canal, « Ajouter », et les deux recherches
/// enregistrées en lignes de réglage. Le fil cède la place — c'est le prix du bloc, et la raison
/// pour laquelle il est replié par défaut.
fn chat_options_recherches_depliees() {
    let mut harness = chat_harness(
        ALL_VISIBLE,
        true,
        RowStyle::Game,
        &MESSAGES,
        &FILTERS,
        false,
    );
    scroll_to_bottom(&mut harness);
    write_mockup(&mut harness, "chat_options_recherches_depliees");
}

/// **3 — Le fil en cartes.** La même planche que la première, avec le format du web : deux lignes,
/// aplat, barre gauche colorée. À comparer à la ligne façon jeu.
fn chat_options_fil_cartes() {
    let mut harness = chat_harness(
        ALL_VISIBLE,
        false,
        RowStyle::Card,
        &MESSAGES,
        &FILTERS,
        false,
    );
    scroll_to_bottom(&mut harness);
    write_mockup(&mut harness, "chat_options_fil_cartes");
}

/// **4 — Deux canaux masqués.** Proximité et Communauté décochés : le compte passe à « n / 30 » et le
/// fil ne montre que les quatre autres.
fn chat_options_canaux_masques() {
    let visible = [false, true, true, true, true, false];
    let mut harness = chat_harness(visible, false, RowStyle::Game, &MESSAGES, &FILTERS, false);
    scroll_to_bottom(&mut harness);
    write_mockup(&mut harness, "chat_options_canaux_masques");
}

/// **5 — Aucun message.** Le fichier vient d'être ouvert, ou aucune ligne de chat n'y figure : bloc
/// d'information à la place du fil, aucune recherche enregistrée.
fn chat_options_vide() {
    static NONE: [Message; 0] = [];
    static NO_FILTERS: [Filter; 0] = [];
    let mut harness = chat_harness(
        ALL_VISIBLE,
        false,
        RowStyle::Game,
        &NONE,
        &NO_FILTERS,
        false,
    );
    harness.run();
    write_mockup(&mut harness, "chat_options_vide");
}

/// **6 — Remonté dans l'historique.** Le fil est en haut, l'auto-défilement est suspendu, le bouton
/// « Aller en bas » est posé au bas du fil.
fn chat_options_remonte() {
    let mut harness = chat_harness(
        ALL_VISIBLE,
        false,
        RowStyle::Game,
        &MESSAGES,
        &FILTERS,
        true,
    );
    harness.run();
    write_mockup(&mut harness, "chat_options_remonte");
}

fn main() {
    chat_options_onglet();
    println!("  chat_options_onglet");
    chat_options_recherches_depliees();
    println!("  chat_options_recherches_depliees");
    chat_options_fil_cartes();
    println!("  chat_options_fil_cartes");
    chat_options_canaux_masques();
    println!("  chat_options_canaux_masques");
    chat_options_vide();
    println!("  chat_options_vide");
    chat_options_remonte();
    println!("  chat_options_remonte");
    let dir = mockup_dir();
    let ecrites = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    let affiche = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("{ecrites} planches écrites dans {}", affiche.display());
}
