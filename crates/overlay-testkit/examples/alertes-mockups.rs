//! **Maquettes de la page « Alerte »** — portage de
//! `https://claude-dev.wakfu-companion.com/fr/profile/alerts` (dépôt web `Oumbra/wakfu-companion`,
//! `features/profile-page`, onglet `alerts`) dans le design system du jeu.
//!
//! **Version 2, après une revue à trois experts qui a refusé la version 1 à l'unanimité.** Les
//! trois premières maquettes proposaient un choix de contenant (fenêtre Options / liste HDV /
//! bandeau in-game) ; les trois revues ont convergé sur la même réponse, et ce n'était aucune des
//! trois : les réglages vont dans l'onglet « Alertes » de la fenêtre Options — qui existe déjà,
//! `OptionsTab::Alertes`, et n'attend que son contenu.
//!
//! Un bandeau in-game, proposé un temps pour couper un son sans ouvrir la fenêtre, a été **retiré
//! sur décision de l'utilisateur** (2026-09-11) : la page d'alertes est un écran de réglages, elle
//! n'a pas de pendant posé par-dessus le jeu.
//!
//! ## Ce que la version 1 a fait de faux, et qui est corrigé ici
//!
//! | Défaut | Correction |
//! | --- | --- |
//! | Titres de section **dorés** | `#b8b9ba`, le gris mesuré du jeu. **L'or ne marque jamais une hiérarchie, seulement un état** (case cochée, onglet inactif, valeur saisie) — c'était le réflexe web le plus visible de la v1 |
//! | `TEXT_MUTED` `#9AA0A6` annoncé « mesuré » | La mesure donne `#b8b9ba` : le jeu n'a pas de troisième gris, ses en-têtes de colonne ont la couleur de ses titres de section |
//! | Trois jetons citant `panels::watchlist` avec **d'autres valeurs** | `TILE_GAP` 6 → **12**, `PANEL_BG` translucide → **`#1e1e1e` opaque**, `TEXT` inventé → **blanc**. La densité de grille et le fond du bandeau — ce que la v1 donnait précisément à juger — étaient faux |
//! | `ROW_HEIGHT` 40 « mesuré sur le HDV » | Le pas réel du zébrage HDV est **60**, sur trois captures. Et le zébrage n'est de toute façon pas l'idiome retenu (voir [`ROW_HEIGHT`]) |
//! | Fenêtre **sans bannière ni panneau de section** | Le chrome est celui du design system, **appelé** et non recopié (`design::window` + `design::panel`) |
//! | Grille de tuiles **à libellés tronqués** | Liste en lignes. Le jeu n'écrit jamais le nom sous un emplacement d'inventaire ; et 104 px ne distinguaient pas « Plan "Epée de Bonta" » de « … Brâkmar » |
//! | Badges de coin sur **plaque noire**, case à cocher écrasée de 20 à 16 px | Le jeu pose des marqueurs **plats, sans plaque, qui débordent le liseré**, et n'y met que des ÉTATS, jamais des commandes |
//! | Croix de retrait sur les **onze** objets | Les **dix premiers sont les défauts** (`DEFAULT_SOUND_ITEM_NAMES`), que le web refuse structurellement de supprimer |
//! | Boutons « Vider la liste » / « Rétablir » **inventés** | Retirés : ils n'existent pas côté web, et détruisaient tout sans confirmation |
//! | Retrait **sans confirmation** | Maquetté — voir [`alertes_options_confirmation_retrait`] |
//! | Rouage pour « Tester le son » | Bouton **texte**. Le rouage veut déjà dire « options » dans ce design system, et le jeu utilise des boutons texte pour les actions sans glyphe évident |
//! | Snapshots **comparés** | Les captures sont écrites dans `target/`, jamais commitées — voir [`write_mockup`] |
//!
//! ## Ce que ce portage demande, et qui n'existe pas encore
//!
//! Une maquette sert à chiffrer un portage. Voici ce qu'il faudra, par ordre de nécessité :
//!
//! 1. **`Input::leading_icon(DsTexture)`** — le jeu pose systématiquement la loupe DANS le champ
//!    (`interface-hdv-achat.png` x 27..39, `interface-personnage-equiement.png` x 768..780,
//!    `interface-options-commandes.png` x 36..48). `Input` ne sait pas le faire ; ces maquettes
//!    peignent la loupe par-dessus, ce qui la laisse hors du `clip_rect` du champ et masquerait
//!    une valeur saisie. Mesuré sur `empty-input-search.png` (341 × 32, champ y 2..29) : encre
//!    13 × 13, 7 px du bord, 8 px avant le premier glyphe du texte.
//! 2. **Un composant « tuile d'objet »** — déjà dupliqué entre `panels::watchlist::entry_tile` et
//!    ce fichier ; un portage en ferait une troisième copie. À trancher avant : les bordures de
//!    rareté font 512 × 512 pour des tuiles peintes à 58, et `DesignSystem::load` téléverse tout
//!    le manifeste d'un bloc — les y verser telles quelles multiplierait par 4,5 son coût.
//! 3. **`Checkbox::size(f32)`** — pour qu'un marqueur d'état n'ait plus à écraser un asset de 20 px.
//! 4. **Une confirmation de retrait** — aucune brique de ce genre n'existe dans `overlay-ui` ; le
//!    web a `ConfirmDeleteService` (popover ancrée au bouton).
//! 5. **`SoundItemEntry::is_default`** — le miroir Rust (`overlay_engine::profile`) n'a pas ce
//!    champ, la protection des dix objets par défaut est donc aujourd'hui impossible à appliquer.
//! 6. **Un panneau de suggestions** sur le champ d'ajout — c'est le vrai mécanisme d'ajout (une
//!    alerte a besoin d'un `catalogId` résolu par le catalogue, pas d'un nom libre).
//!    `design::select` sait déjà peindre une liste par-dessus le contenu suivant.
//! 7. **Une icône « haut-parleur »** — aucun des 34 glyphes de `assets/design-system/icons/` n'en
//!    est un, et l'onglet Son du jeu règle son volume sans jamais en dessiner. À extraire du client
//!    (skill `design-asset`) si l'on veut le pictogramme du web.
//!
//! ## Le contrat transactionnel — ce que le portage doit écrire
//!
//! La page vit dans la fenêtre Options, **transactionnelle par contrat** (§5.1 du plan : « rien
//! n'est pris en compte tant que Valider n'a pas été renvoyé ET jugé valide »), et son pied de
//! page est partagé par tous les onglets. Un onglet qui appliquerait ses changements
//! immédiatement à côté d'un onglet qui commit produirait le pire cas : un pied de page dont
//! l'effet dépend de l'onglet où l'on se trouve. Le transactionnel n'est donc pas un choix libre,
//! c'est une conséquence — mais il a un prix, et ces quatre points sont ce prix. Aucun n'est
//! maquettable : ce sont des comportements, pas des écrans.
//!
//! 1. **Une garde de fermeture.** Échap, clic hors fenêtre et croix de la fenêtre OS doivent
//!    demander confirmation tant que des modifications sont en attente — sans elle, un joueur qui
//!    ajoute trois objets puis ferme par réflexe perd tout, en silence. Le **changement d'onglet**,
//!    lui, n'intercepte rien : le brouillon lui survit.
//! 2. **« Annuler » restaure le brouillon ENTIER, tous onglets confondus** — `soundItems`,
//!    `alertDurationSeconds`, `alertManualClose` *et* le chemin de `wakfu.log`. Un « Annuler » qui
//!    n'annulerait qu'une partie de ce que l'utilisateur a touché serait imprévisible.
//! 3. **Pendant qu'un dialogue de confirmation est ouvert, le pied de page est inerte.** C'est ce
//!    que dit le voile du jeu, et c'est pourquoi il couvre la fenêtre entière (voir [`confirm_box`]).
//! 4. **« Valider » réécrit TOUTE la clé `profile.soundItems`** via un `PATCH /api/v1/settings`,
//!    en *dernier écrivain gagne*. Une fenêtre laissée ouverte pendant qu'on modifie la liste
//!    depuis la page web écrasera la modification web au moment du commit. C'est le prix assumé du
//!    transactionnel : l'application immédiate ne supprimerait pas ce risque, elle réduirait la
//!    fenêtre de conflit de quelques minutes à quelques secondes.
//!
//! **Driver logiciel requis** — même prérequis que `tests/panels.rs`, voir sa doc de module.

#[path = "shared/wakassets_fixtures.rs"]
mod wakassets_fixtures;

use egui::{Color32, Rect, RichText, Stroke, StrokeKind, Vec2};
use egui_kittest::Harness;
use overlay_engine::WakfuRarity;
use overlay_ui::design::{self, ButtonSize, ButtonVariant, DsIcon, IconContext, InputSize};
use overlay_ui::panels::options_modal::OptionsTab;
use overlay_ui::ui_icons::UiIcons;

use wakassets_fixtures::{CategoryFilter, CategoryIcons, RarityGems, GEM_BOX};

// -------------------------------------------------------------------------------------------
// Jetons propres aux maquettes.
//
// Chacun dit d'où il vient : mesure sur une capture du jeu, valeur REPRISE TELLE QUELLE d'un
// panneau existant, ou choix assumé comme tel. La v1 a livré trois jetons qui annonçaient une
// reprise et donnaient une autre valeur — le pire cas, puisque rien ne les distinguait à la
// lecture d'une mesure authentique. Les valeurs ci-dessous ont été relues une à une contre leur
// source, ligne par ligne.
// -------------------------------------------------------------------------------------------

/// Fond derrière une fenêtre — le même sombre que `tests/panels.rs` pose sous la modale Options,
/// pour qu'un bord translucide se voie.
const BACKDROP: Color32 = Color32::from_rgb(0x0B, 0x0D, 0x10);

/// Texte courant — **`TEXT_COLOR` de `panels::watchlist:510`, c'est-à-dire blanc**. La v1 déclarait
/// reprendre ce jeton et peignait `#E4E6E8`.
const TEXT: Color32 = Color32::WHITE;

/// Gris des titres de section, en-têtes de colonne, compteurs et unités — **`#b8b9ba`**.
///
/// C'est `panels::options_modal::SECTION_TITLE_TEXT`, et c'est une mesure convergente sur quatre
/// captures indépendantes : « Musique » `#b9baba` et « Sons - Ambiance » `#b8b9ba`
/// (`interface-options-son.png`), « Équipements » `#babbbc`
/// (`interface-personnage-equiement.png`), « Types » `#bababa` (`interface-hdv-achat.png`). Les
/// en-têtes de colonne du HDV donnent la même valeur (`#b9babb` sur « Nom », `#bababc` sur
/// « Quantité ») : **le jeu n'a pas de troisième gris atténué**, et en inventer un — ce que faisait
/// le `TEXT_MUTED` `#9AA0A6` de la v1, plus sombre de 20 % et bleuté là où le gris du jeu est
/// neutre — crée un niveau de hiérarchie qui n'existe pas.
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);

/// Fenêtre intérieure de `Border-<RARETÉ>.webp` — `ITEM_BORDER_INNER_MARGIN_RATIO` de
/// `panels::watchlist`, mesurée une fois pour toutes sur les sept fichiers (52/512).
const BORDER_INNER_RATIO: f32 = 52.0 / 512.0;

/// Part de la fenêtre intérieure occupée par l'icône — `ITEM_ICON_FILL_RATIO` de
/// `panels::watchlist`, réglé sur retour utilisateur (« les objets doivent être plus gros »).
const ICON_FILL_RATIO: f32 = 0.96;

/// **Taille de la fenêtre Options telle que cette page la demande.**
///
/// `panels::options_modal::WINDOW_SIZE` vaut aujourd'hui 560 × 436, un rapport d'aspect aligné sur
/// les 720 × 561 de la vraie fenêtre du jeu (demande utilisateur du 2026-09-09). Cette page n'y
/// tient pas : à 560 de large, la grille n'a la place que de trois tuiles, et à 561 de haut, d'une
/// seule rangée.
///
/// Les deux dimensions sont donc portées à ce qu'il faut pour **cinq tuiles par rangée et trois
/// rangées visibles**, phrase d'explication et aération comprises — demande explicite du 2026-09-11, qui autorise l'agrandissement en largeur
/// comme en hauteur. Le rapport d'aspect du jeu n'est plus conservé : c'est un arbitrage assumé au
/// profit du contenu, à confirmer avant portage.
const WINDOW: Vec2 = Vec2::new(760.0, 810.0);

/// Largeur d'une tuile d'objet suivi — calée pour que **cinq tiennent sur une rangée** (demande
/// explicite) dans la fenêtre de [`WINDOW`]. **Choix de mise en page** : elle doit tenir le nom
/// d'un objet sans le réduire à trois lettres — « Plan "Epée de Bonta" » et « … Brâkmar » ne doivent
/// pas rendre la même chaîne, défaut mesuré sur une version précédente. La grille du dépôt web
/// tient le même raisonnement avec son `minmax(140px, 1fr)`.
const TILE_WIDTH: f32 = 118.0;

/// Hauteur d'une tuile : rangée de badges, emplacement d'objet, nom, marges.
const TILE_HEIGHT: f32 = 88.0;

/// Gouttière entre deux tuiles — **demande explicite** : « les petites tuiles doivent être
/// séparées sur tous les bords pour éviter que ce soit tout collé ».
const TILE_GAP: f32 = 10.0;

/// Épaisseur de la bordure d'état d'une tuile — 2 px, demandé explicitement.
const TILE_BORDER_WIDTH: f32 = 2.0;

/// Rayon de la bordure d'état — arrondi, demandé explicitement ; la valeur est celle du champ de
/// saisie (`tokens::INPUT_RADIUS`), le seul arrondi prononcé du design system.
const TILE_RADIUS: u8 = 4;

/// Fond d'une tuile — le fond de panneau du jeu, pour que la bordure d'état porte seule le signal.
const TILE_FILL: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);

/// Côté de l'emplacement d'objet dans une tuile — l'icône y est nettement plus grande que les
/// 32 px du web : c'est ce que « afficher les objets comme dans le jeu » demande.
const TILE_SLOT: f32 = 44.0;

/// Hauteur réservée à la rangée de badges, au-dessus de l'emplacement.
const TILE_BADGE_ROW: f32 = 18.0;

/// Côté d'un badge de coin (icône de son, croix de retrait).
const TILE_BADGE: f32 = 14.0;

/// Retrait d'un badge depuis le coin de la tuile.
const TILE_BADGE_INSET: f32 = 5.0;
/// Marge du nom de chaque côté de la tuile — ce qui reste est la largeur utile avant ellipse.
const TILE_NAME_INSET: f32 = 5.0;

/// Bleu d'accent — `ACCENT` de `panels::watchlist:516`, lui-même repris de `--accent` du dépôt web
/// (`#00d2ff`). C'est la bordure d'une tuile dont le son est ACTIF.
const ACCENT: Color32 = Color32::from_rgb(0x00, 0xD2, 0xFF);

/// Bordure d'une tuile dont le son est COUPÉ — le gris de bord des panneaux
/// (`panels::watchlist:496`). L'état ne change que la bordure et l'icône, jamais l'objet.
const MUTED_BORDER: Color32 = Color32::from_rgb(0x4D, 0x4D, 0x4D);

/// Fond d'une **ligne de réglage mise en valeur** — mesuré sur la colonne de droite de
/// `interface-personnage-aptitudes.png` : chaque ligne d'aptitude (« % Points de Vie »,
/// « Barrière », « Maîtrise Élémentaire »…) est posée sur un aplat `#26282b` à bords arrondis, sur
/// un fond de section à `#171a19` — une quinzaine de niveaux d'écart, assez pour détacher la ligne
/// sans la faire ressortir.
///
/// C'est ce que le dépôt web obtient avec le fond de son bloc « Fermeture de l'alerte », et le jeu
/// a bien un idiome pour ça : demande explicite de l'utilisateur, référence à l'appui.
const SETTING_ROW_FILL: Color32 = Color32::from_rgb(0x26, 0x28, 0x2B);

/// Rayon d'une ligne de réglage — mesuré sur les mêmes lignes (transition d'alpha sur 2 px aux
/// quatre coins).
const SETTING_ROW_RADIUS: u8 = 4;

/// Hauteur d'une ligne de réglage mise en valeur — les lignes d'aptitude du jeu cadencent à 32 px
/// (fronts mesurés à y = 216, 248, 280, 312, 344, 376). Portée ici à 40 : la ligne porte une case
/// à cocher de 20 px et un champ de 25, qu'un fond de 32 serrerait contre ses bords.
const SETTING_ROW_HEIGHT: f32 = 40.0;

/// Corps du texte courant — celui des libellés de ligne du panneau, pour qu'une description et un
/// libellé se lisent sur le même plan.
const BODY_FONT_SIZE: f32 = 15.0;

/// Aération autour d'un titre de section — **demande explicite de l'utilisateur**, et elle est
/// juste : les interfaces d'options du jeu laissent respirer leurs blocs, là où les maquettes
/// précédentes collaient le contenu à son titre.
///
/// **Porté de 12 à 18 après un second retour** (« je ne vois pas de changement ») : à 12, le
/// calcul était pourtant exact — 12 px au-dessus du rectangle réservé par `design::heading`, 12 en
/// dessous. Mais ce rectangle vaut `HEADING_INK_HEIGHT`, la boîte d'encre d'un mot SANS jambage,
/// et « Objets suivis » en a un : le « j » descend 3 px plus bas, si bien que l'œil voit 12 px
/// au-dessus du titre et 9 en dessous. Un écart plus large absorbe cette asymétrie et donne l'air
/// demandé ; corriger le jambage lui-même demanderait de mesurer l'encre réelle titre par titre,
/// ce qui ferait dépendre la mise en page du libellé.
const SECTION_GAP: f32 = 18.0;

/// Hauteur d'une ligne de réglage — **mesurée sur `interface-options-commandes.png`**, la liste de
/// raccourcis du jeu : ses champs tombent à y = 202, 241, 280, 319, 358, soit un pas constant de
/// 39 px.
///
/// C'est l'idiome retenu pour les objets suivis, à la place de la table zébrée du HDV que
/// proposait la v1. Le jeu réserve celle-ci (pas de 60 px, une bande sur deux voilée) aux
/// **données multi-colonnes** — Nom / Niv. / Enchantement / Quantité / Prix. Pour « un libellé à
/// gauche, un contrôle à droite », il pose une liste plate dans un panneau, sans zébrage : c'est
/// très exactement la forme de la page Alerte.
const ROW_HEIGHT: f32 = 39.0;

/// Fond du corps d'une boîte de confirmation — **`#585955`**, mesuré sur
/// `interface-confirm-box.png` (histogramme de x 40..410 / y 60..110 : `#585955` dominant, puis
/// `#595a56` et `#5a5b5c`, tous à un niveau les uns des autres). Un gris **clair**, à l'opposé du
/// kaki `SELECT_LIST_FILL` que la v2 lui donnait.
const CONFIRM_FILL: Color32 = Color32::from_rgb(0x58, 0x59, 0x55);

/// Bord d'une boîte de confirmation — le noir de bord commun au jeu, `tokens::SELECT_LIST_BORDER`.
const CONFIRM_BORDER: Color32 = Color32::from_rgb(0x0E, 0x10, 0x15);

/// Texte d'une boîte de confirmation — blanc, comme tout texte de corps du jeu.
const CONFIRM_TEXT: Color32 = Color32::WHITE;

/// Médaillon « ? » en crête d'une boîte de confirmation — l'or du jeu
/// (`tokens::TAB_LABEL_IDLE`), comme sur `interface-confirm-box.png` où le disque est doré et son
/// point d'interrogation sombre.
const CONFIRM_CREST: Color32 = Color32::from_rgb(0xF4, 0xD8, 0x9E);

/// Largeur du corps d'une boîte de confirmation — 420 px, mesurés sur
/// `interface-confirm-box.png` (bords à x=13 et x=433).
const CONFIRM_WIDTH: f32 = 420.0;

/// Hauteur du corps — ~144 px sur la même capture (y ≈ 47..191). Arrondi à 120 ici, la question
/// tenant sur une ligne au lieu de deux : **choix de mise en page, pas une mesure**.
const CONFIRM_HEIGHT: f32 = 120.0;

struct Item {
    name: &'static str,
    rarity: WakfuRarity,
    sound_on: bool,
    /// Objet de la liste prédéfinie : jamais retirable — voir [`ITEMS`].
    is_default: bool,
}

impl Item {
    const fn preset(name: &'static str, rarity: WakfuRarity, sound_on: bool) -> Self {
        Self {
            name,
            rarity,
            sound_on,
            is_default: true,
        }
    }
    const fn added(name: &'static str, rarity: WakfuRarity, sound_on: bool) -> Self {
        Self {
            name,
            rarity,
            sound_on,
            is_default: false,
        }
    }
}

/// Les onze objets de la capture fournie par l'utilisateur, dans son ordre.
///
/// **Les dix premiers sont exactement `DEFAULT_SOUND_ITEM_NAMES`** (dépôt web,
/// `core/services/profile.service.ts:52-63`), donc les objets par défaut — que `removeSoundItem`
/// refuse structurellement de supprimer (`e.isDefault || …`) et pour lesquels le web n'affiche
/// aucune croix. Le onzième, « Combinaison Lardante », est l'ajout du joueur : le seul retirable.
/// La v1 mettait une croix sur les onze.
///
/// **Les raretés sont une reconstitution, pas une donnée** : la capture ne dit pas quelle rareté
/// le référentiel attribue à chaque objet. Les deux teintes qu'on y distingue sont reportées sur
/// les raretés du jeu qui leur ressemblent, et deux entrées sont poussées vers des raretés
/// extrêmes pour que la planche montre l'écart entre bordures plutôt qu'un camaïeu.
const ITEMS: &[Item] = &[
    Item::preset("Pierre d'aventure", WakfuRarity::Mythical, true),
    Item::preset("Pierre d'équilibre", WakfuRarity::Mythical, true),
    Item::preset("Pierre d'entourage", WakfuRarity::Mythical, false),
    Item::preset("Pierre de vitesse", WakfuRarity::Relic, true),
    Item::preset("Pierre ultime", WakfuRarity::Memory, true),
    Item::preset("Influence III", WakfuRarity::Legendary, true),
    Item::preset("Plan \"Epée de Bonta\"", WakfuRarity::Legendary, true),
    Item::preset("Plan \"Epée de Brâkmar\"", WakfuRarity::Legendary, false),
    Item::preset("Plan \"Epée de Sufokia\"", WakfuRarity::Legendary, true),
    Item::preset("Plan \"Epée d'Amakna\"", WakfuRarity::Legendary, true),
    Item::added("Combinaison Lardante", WakfuRarity::Epic, true),
];

/// La phrase d'explication de la page — clé i18n `profile.alertsDesc` du dépôt web.
const DESC: &str = "Objets qui déclenchent une alerte sonore et un message à l'écran lorsqu'ils \
                    sont ramassés.";

/// Ce que le panneau de suggestions afficherait pour la saisie « pierre » — le troisième champ dit
/// sa catégorie (elle décide des filtres affichés, voir `category_bar`) et le dernier s'il est
/// DÉJÀ suivi (grisé, non sélectionnable, comme côté web).
/// Marge gauche d'une rangée de suggestion.
const ROW_PAD_X: f32 = 6.0;
/// Hauteur de la bande de filtres — bouton 26 + 2 × 6 de marge (`padding: 6px` côté web).
const CATEGORY_BAR_HEIGHT: f32 = 38.0;
/// Côté d'un bouton de catégorie (`.wakfu-autocomplete-category-btn`, 26 × 26).
const CATEGORY_BUTTON: f32 = 26.0;
/// Marge intérieure du bouton : l'icône occupe 20 des 26 (`padding: 3px`).
const CATEGORY_ICON_PAD: f32 = 3.0;
/// Écart entre deux boutons de catégorie.
const CATEGORY_GAP: f32 = 4.0;
/// Marge gauche de la bande.
const CATEGORY_PAD: f32 = 6.0;
/// Rayon d'angle d'un bouton de catégorie.
const CATEGORY_RADIUS: u8 = 4;
/// Écart entre la gemme, l'image et le nom d'une rangée.
const ROW_GAP: f32 = 6.0;
/// Côté de l'image d'objet d'une rangée — tient dans les 28 px de `SELECT_ROW_HEIGHT`.
const ROW_IMAGE: f32 = 22.0;

const SUGGESTIONS: &[(&str, WakfuRarity, CategoryFilter, bool)] = &[
    (
        "Pierre d'aventure",
        WakfuRarity::Mythical,
        CategoryFilter::Resources,
        true,
    ),
    (
        "Pierre de dolomite",
        WakfuRarity::Common,
        CategoryFilter::Resources,
        false,
    ),
    (
        "Pierre de lune",
        WakfuRarity::Rare,
        CategoryFilter::Equipment,
        false,
    ),
    (
        "Pierre ponce",
        WakfuRarity::Common,
        CategoryFilter::Craft,
        false,
    ),
];

// -------------------------------------------------------------------------------------------
// Briques partagées
// -------------------------------------------------------------------------------------------

/// Peint l'emplacement d'objet du jeu dans `rect` — **bordure de rareté optionnelle**.
///
/// `Some(rareté)` : la tuile d'alerte et celle du Suivi, où le cadre coloré EST le porteur de la
/// rareté. `None` : le panneau de suggestions, où la rareté est déjà dite par la gemme qui précède
/// l'image — la bordure ferait doublon (demande explicite de l'utilisateur, 2026-09-11 : « tu ne
/// dois garder que la gemme et l'image »). C'est aussi ce que fait le web, dont `app-item-icon`
/// est une image nue.
///
/// **La bordure se peint AVANT l'icône**, jamais après : la fenêtre intérieure de
/// `Border-<RARETÉ>.webp` n'est pas transparente mais un aplat semi-opaque teinté par la rareté —
/// peinte après, elle voile l'icône entière. C'est un bug réel corrigé le 2026-09-06 dans
/// `panels::watchlist`, et la première chose que la doc du futur composant devra porter.
///
/// L'icône est le repli générique : les vraies viennent du CDN (`remote_icons`), inaccessible
/// depuis le harnais. C'est aussi ce que l'overlay affiche tant qu'un téléchargement n'a pas
/// abouti — ce qui est jugé ici est l'emplacement, pas le dessin de l'objet.
fn item_slot(ui: &egui::Ui, icons: &UiIcons, rect: Rect, rarity: Option<WakfuRarity>) {
    if let Some(rarity) = rarity {
        egui::Image::new(icons.item_border(rarity)).paint_at(ui, rect);
    }
    let inner = rect.width() * (1.0 - 2.0 * BORDER_INNER_RATIO) * ICON_FILL_RATIO;
    egui::Image::new(icons.unknown_entity_texture()).paint_at(
        ui,
        Rect::from_center_size(rect.center(), Vec2::splat(inner)),
    );
}

/// Champ de recherche — le champ du design system, avec l'ornement du jeu.
///
/// **`Input::leading_icon` existe désormais** (implémenté avec ces maquettes, jetons mesurés sur
/// `empty-input-search.png`) : la loupe est peinte par le composant, dans son clip, et la
/// gouttière qu'elle impose vaut pour le texte indicatif comme pour la valeur saisie. La v2
/// simulait ce retrait par six espaces de tête, ce qui laissait la valeur saisie démarrer sous la
/// loupe — visible sur sa capture des suggestions, où la loupe couvrait le « p » de « pierre ».
fn search_field(
    ui: &mut egui::Ui,
    text: &mut String,
    placeholder: &str,
    width: f32,
    enabled: bool,
) {
    ui.add(
        design::input(text)
            .leading_icon(DsIcon::Search)
            .placeholder(placeholder)
            .size(InputSize::Standard)
            .width(width)
            .enabled(enabled)
            .log_name("maquette.recherche"),
    );
}

/// **La tuile d'un objet suivi — le composant « item alerte ».**
///
/// Spécifié par l'utilisateur (2026-09-11), après refus de la version en lignes : l'objet doit se
/// voir *comme dans le jeu*, pas se lire dans un tableau. La tuile porte donc, de haut en bas :
///
/// | Élément | Ce qu'il dit |
/// | --- | --- |
/// | Bordure du bloc, 2 px, arrondie | **l'état du SON** : [`ACCENT`] s'il est actif, [`MUTED_BORDER`] s'il est coupé |
/// | Icône de son, coin haut-gauche | le même état, en pictogramme — `IconVolume` / `IconVolumeMute` |
/// | Croix, coin haut-droit | retrait — **seulement si l'objet n'est pas un défaut** |
/// | Emplacement à bordure de rareté | ce qu'est l'objet — `Border-<RARETÉ>.webp`, l'asset du jeu |
/// | Nom en clair, sous l'emplacement | ce qu'est l'objet, en toutes lettres |
///
/// **Cliquer la tuile bascule le son, et ne touche que la bordure et l'icône** — jamais
/// l'emplacement de rareté ni le nom, qui disent ce qu'est l'objet et non ce qu'on en fait. C'est
/// ce qui permet de lire les deux informations d'un coup d'œil sans qu'elles se gênent.
fn alert_item(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    item: &Item,
    sound_on: bool,
    removable: bool,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(TILE_WIDTH, TILE_HEIGHT), egui::Sense::click());

    let state_color = if sound_on { ACCENT } else { MUTED_BORDER };
    ui.painter().rect_filled(rect, TILE_RADIUS, TILE_FILL);
    ui.painter().rect_stroke(
        rect,
        TILE_RADIUS,
        Stroke::new(TILE_BORDER_WIDTH, state_color),
        StrokeKind::Inside,
    );

    // L'emplacement d'objet du jeu, sous la rangée de badges.
    let slot = Rect::from_center_size(
        egui::pos2(
            rect.center().x,
            rect.top() + TILE_BADGE_ROW + TILE_SLOT / 2.0,
        ),
        Vec2::splat(TILE_SLOT),
    );
    item_slot(ui, icons, slot, Some(item.rarity));

    // Le nom, élidé à la largeur de la tuile. `name_rect` est la zone de survol de l'infobulle
    // du nom : le libellé seul, pas la tuile entière (demande explicite de l'utilisateur).
    let (shown, elided) = elide(ui, item.name, rect.width() - 2.0 * TILE_NAME_INSET);
    let name_rect = ui.painter().text(
        egui::pos2(rect.center().x, slot.bottom() + 5.0),
        egui::Align2::CENTER_TOP,
        shown,
        design::text::label_font(ui.ctx(), 13.0),
        TEXT,
    );

    // Icône de son — les **vraies** icônes du manifeste (`IconVolume`, `IconVolumeMute`), entrées
    // au design system depuis. Les versions précédentes n'en avaient aucune : l'une peignait un
    // rouage (qui veut dire « options »), l'autre une coche verte (qui veut dire « équipé »).
    let ds = design::DesignSystem::get(ui.ctx());
    let icon = if sound_on {
        DsIcon::Volume
    } else {
        DsIcon::VolumeMute
    };
    let native = ds.icon_native_size(icon);
    ds.paint_icon(
        ui.painter(),
        Rect::from_center_size(
            egui::pos2(
                rect.left() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
                rect.top() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
            ),
            design::components::icon_button::glyph_fit(native, TILE_BADGE),
        ),
        icon,
        // **Pas la couleur de la bordure** : demande explicite. À l'identique, l'icône de coupure
        // se fondait dans son propre liseré gris et devenait illisible. Elle prend donc la teinte
        // de la croix de retrait — la bordure porte seule le signal de couleur, l'icône dit
        // seulement lequel des deux pictogrammes s'applique.
        SUBDUED,
    );

    // Croix de retrait — **absente sur un objet par défaut**, que le dépôt web refuse
    // structurellement de supprimer (`removeSoundItem` : `e.isDefault || …`). Absente aussi du
    // bandeau in-game, où le retrait n'a pas sa place : c'est l'action destructrice, elle vit dans
    // la fenêtre Options avec sa confirmation.
    if removable && !item.is_default {
        let native = ds.icon_native_size(DsIcon::Close);
        ds.paint_icon(
            ui.painter(),
            Rect::from_center_size(
                egui::pos2(
                    rect.right() - TILE_BADGE_INSET - TILE_BADGE / 2.0,
                    rect.top() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
                ),
                design::components::icon_button::glyph_fit(native, TILE_BADGE - 2.0),
            ),
            DsIcon::Close,
            SUBDUED,
        );
    }

    // **Infobulle du nom : seulement si le nom est coupé, et seulement sur le libellé.** Un nom
    // qui tient en entier n'a rien à révéler — une infobulle qui répète ce qui est déjà lisible
    // est du bruit. Même règle que le web (`[tooltipOnlyIfTruncated]="true"` sur le nom, voir
    // `wakfu-autocomplete.component.html`).
    //
    // Les deux infobulles s'excluent : sans ça, survoler le nom en déclencherait deux, l'une
    // par-dessus l'autre. Celle du nom gagne sur sa propre zone, celle de la tuile couvre le
    // reste.
    let sur_le_nom = elided && {
        let zone = ui.interact(name_rect, response.id.with("nom"), egui::Sense::hover());
        zone.clone().on_hover_text(item.name);
        zone.hovered()
    };

    // Curseur main : la tuile entière est cliquable, et rien d'autre ne le dit — demande
    // explicite de l'utilisateur.
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if sur_le_nom {
        return response;
    }
    response.on_hover_text(format!(
        "{} — {}",
        item.name,
        if sound_on {
            "son activé, cliquer pour couper"
        } else {
            "son coupé, cliquer pour rétablir"
        }
    ))
}

/// Tronque un nom à la largeur d'une tuile, avec une ellipse.
///
/// Rend **aussi** le fait d'avoir coupé : c'est cette information qui décide de l'infobulle du
/// nom (voir `alert_item`). La déduire après coup en comparant les deux chaînes marcherait, mais
/// obligerait chaque appelant à y penser — et un nom qui finit déjà par « … » la mettrait en
/// défaut.
fn elide(ui: &egui::Ui, text: &str, max_width: f32) -> (String, bool) {
    let font = design::text::label_font(ui.ctx(), 13.0);
    let measure = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE)
            .rect
            .width()
    };
    if measure(text) <= max_width {
        return (text.to_string(), false);
    }
    let mut cut = text.to_string();
    while !cut.is_empty() && measure(&format!("{cut}…")) > max_width {
        cut.pop();
    }
    (format!("{}…", cut.trim_end()), true)
}

/// Un paragraphe de texte courant — **la brique qui manque au design system**.
///
/// Même police et même corps que les libellés de ligne (« Tester le son de l'alerte »), replié sur
/// la largeur donnée. C'est ce que `design::text` devra faire ; en attendant, la maquette le pose
/// à la main plutôt que de détourner `design::info_text`, dont la pastille et le ton disent
/// « remarque » là où il ne s'agit que d'une description.
///
/// **`design::text` coexistera avec le module `design::text` déjà là** (`text::label_font`) :
/// Rust range les modules et les fonctions dans deux espaces de noms distincts, donc
/// `design::text("…")` et `design::text::label_font(…)` se résolvent tous les deux. Légal, mais à
/// savoir avant d'écrire le composant.
fn text_paragraph(ui: &mut egui::Ui, text: &str, width: f32) {
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .color(TEXT)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        )
        .wrap_mode(egui::TextWrapMode::Wrap),
    );
    let _ = width;
}

/// La ligne « Tester le son de l'alerte » — **l'alerte SONORE, et rien d'autre**.
///
/// Elle était jusqu'ici sur la même ligne que la fermeture du toast, sous un titre « Alerte
/// sonore » qui coiffait les deux. C'était un contresens, relevé par l'utilisateur : *« la
/// fermeture de l'alerte, c'est le toast, ce n'est pas lié à l'alerte sonore en elle-même »*. Le
/// son et le message à l'écran sont deux canaux de la même alerte, réglés séparément — les empiler
/// sur une ligne laissait entendre que la durée s'appliquait au son.
fn test_sound_row(ui: &mut egui::Ui, width: f32) {
    let row = ui.allocate_space(Vec2::new(width, ROW_HEIGHT)).1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    cell.horizontal_centered(|ui| {
        ui.label(
            RichText::new("Tester le son de l'alerte")
                .color(TEXT)
                .size(15.0),
        );
        ui.add_space(12.0);
        ui.add(
            design::icon_button(DsIcon::Volume)
                .context(IconContext::Panel)
                .tooltip("Jouer le son d'alerte")
                .log_name("maquette.tester"),
        );
    });
}

/// Le bloc « Fermeture de l'alerte » — **le TOAST, pas le son**.
///
/// Posé sur un aplat clair à bords arrondis ([`SETTING_ROW_FILL`]) qui le détache du fond de
/// section. C'est l'idiome des lignes d'aptitude du jeu (`interface-personnage-aptitudes.png`,
/// colonne de droite : chaque ligne « % Points de Vie », « Barrière », « Maîtrise Élémentaire » a
/// son propre aplat arrondi), et c'est ce que le dépôt web obtient de son côté avec le fond de son
/// bloc de fermeture. Demande explicite de l'utilisateur, référence à l'appui — un réglage qui
/// porte sur un autre canal que la ligne au-dessus mérite d'être séparé visuellement, même si
/// aucune autre page de l'overlay ne le fait encore.
///
/// Case à cocher plutôt que le switch « Auto | Manuelle » du web : le jeu n'a pas de switch à deux
/// positions, son idiome pour un choix binaire est la case (`interface-options-son.png` :
/// « Couper la musique », « Lecture continue »).
fn close_settings_row(ui: &mut egui::Ui, auto: &mut bool, seconds: &mut String, width: f32) {
    let row = ui.allocate_space(Vec2::new(width, SETTING_ROW_HEIGHT)).1;
    ui.painter()
        .rect_filled(row, SETTING_ROW_RADIUS, SETTING_ROW_FILL);

    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row.shrink2(Vec2::new(12.0, 0.0))));
    cell.horizontal_centered(|ui| {
        ui.add(design::checkbox(auto, "Fermeture automatique").log_name("maquette.auto"));
        ui.add_space(12.0);
        // **Le champ suit la case** : décochée, la fermeture est manuelle, donc il n'y a plus de
        // délai avant fermeture et la valeur n'a plus d'effet — le champ est grisé et non
        // modifiable. La logique était déjà là (`.enabled`), mais aucun rendu ne la montrait.
        //
        // `design::input` est du texte libre — il n'existe pas encore de champ numérique borné
        // dans le design system, alors que le jeu en a un (`input-number.png`, `button-plus.png`,
        // `button-moins.png`). En attendant, la borne se pose **à la validation**, voir
        // `clamp_duration`.
        ui.add(
            design::input(seconds)
                .size(InputSize::Standard)
                .width(52.0)
                .enabled(*auto)
                .log_name("maquette.duree"),
        );
        ui.label(RichText::new("sec.").color(SUBDUED).size(15.0));
    });
}

/// Borne la durée saisie — `MIN_ALERT_DURATION_SECONDS` du web (0,5 s), défaut 3,5 s.
///
/// **À appeler à la VALIDATION, jamais à la frappe.** La v3 l'appliquait à chaque frame, ce qui
/// rendait le champ inutilisable : taper `0.75` donnait `"0"` → borné à `"0.5"` sous les doigts,
/// puis `"0.5."` → non parsable → `"3.5"` ; et `1,5` en disposition française tombait sur le même
/// écueil. Toute saisie décimale passe par un état transitoire non parsable, et l'écraser avant
/// qu'elle soit finie interdit d'écrire la valeur voulue.
///
/// Le web ne fait pas ça : il borne la valeur **stockée** (`setAlertDuration` → `Math.max`) et
/// laisse le champ tranquille pendant la frappe. Côté overlay, l'appel appartient au moment où
/// l'appelant lit la valeur — perte de focus, ou clic sur « Valider ».
///
/// Le besoin, lui, reste entier : « 0 » ou « abc » validé rendrait le message d'alerte invisible
/// sans le moindre signal, d'autant plus que la fenêtre est transactionnelle et se ferme derrière
/// le clic.
#[allow(dead_code)]
fn clamp_duration(raw: &str) -> f32 {
    raw.replace(',', ".").parse::<f32>().unwrap_or(3.5).max(0.5)
}

/// La gemme de rareté d'une rangée de suggestion — **la vraie image du jeu**.
///
/// Elle vient des fixtures du harnais (voir `shared/rarity_gems.rs`), qui tiennent lieu de ce que
/// `RemoteIconStore` télécharge au runtime : le fichier est choisi par
/// `IconRef::for_rarity(rarité).gfx_id`, donc par la même correspondance que celle qui construira
/// l'URL en vrai.
fn rarity_gem(ui: &egui::Ui, gems: &RarityGems, rect: Rect, rarity: WakfuRarity) {
    gems.paint(ui, rect, rarity);
}

/// La bande de filtres par catégorie, en tête du panneau — **relevée sur le web**
/// (`wakfu-autocomplete.component.ts`/`.css`).
///
/// Trois règles qui ne se devinent pas sur une capture :
///
/// 1. **Seules les catégories PRÉSENTES dans les résultats ont un bouton** (`filterButtons`) —
///    afficher un filtre qui viderait la liste n'aurait aucun sens ; la bande disparaît
///    entièrement s'il n'y en a aucune.
/// 2. **« Tout » n'est pas une catégorie** : c'est la remise à zéro, toujours en tête, active tant
///    qu'aucun filtre ne l'est.
/// 3. **Pas de filtre « Monstres »** : il n'existe qu'en domaine `both`, et cette page est en
///    domaine `item` (`profile-page.component.html`).
///
/// Un clic sur le filtre déjà actif le relâche (`toggleCategoryFilter`).
fn category_bar(
    ui: &egui::Ui,
    icons: &CategoryIcons,
    rect: Rect,
    filtres: &[CategoryFilter],
    actif: CategoryFilter,
) {
    for (i, filtre) in filtres.iter().enumerate() {
        let cell = Rect::from_min_size(
            egui::pos2(
                rect.left() + CATEGORY_PAD + i as f32 * (CATEGORY_BUTTON + CATEGORY_GAP),
                rect.center().y - CATEGORY_BUTTON / 2.0,
            ),
            Vec2::splat(CATEGORY_BUTTON),
        );
        let est_actif = *filtre == actif;
        if est_actif {
            ui.painter()
                .rect_filled(cell, CATEGORY_RADIUS, design::tokens::SELECT_ROW_HIGHLIGHT);
            ui.painter().rect_stroke(
                cell,
                CATEGORY_RADIUS,
                Stroke::new(1.0, design::tokens::STEPPER_ICON_TINT),
                StrokeKind::Inside,
            );
        }
        icons.paint(ui, cell.shrink(CATEGORY_ICON_PAD), *filtre, est_actif);
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, design::tokens::SELECT_LIST_TOP_LINE),
    );
}

/// Le panneau de suggestions du champ d'ajout — voir [`alertes_options_ajout_suggestions`].
///
/// **À la largeur exacte de son champ**, comme toute liste dépliée du jeu : `select-simple.png`
/// (220 × 36) et `select-simple-opened.png` se superposent au pixel, ce qui est précisément la
/// raison d'être de `SELECT_LIST_TOP_LINE` — un liseré qui n'aurait aucun sens si la liste était
/// plus étroite que son socle. La v2 la posait à 62 % de la largeur du champ, une convention web,
/// et les cases « Son » des lignes du dessous réapparaissaient à sa droite : on lisait des
/// suggestions dotées d'une bascule sonore.
///
/// Cadence des entrées : `tokens::SELECT_ROW_HEIGHT` (28), la mesure du select simple qui fait foi
/// — la v2 écrivait 30, qui n'est aucune des deux mesures du jeu.
fn suggestion_list(
    ui: &egui::Ui,
    icons: &UiIcons,
    gems: &RarityGems,
    cats: &CategoryIcons,
    inner: Rect,
    field_bottom: f32,
) {
    let row_h = design::tokens::SELECT_ROW_HEIGHT;
    let list = Rect::from_min_size(
        egui::pos2(inner.left(), field_bottom + 2.0),
        Vec2::new(
            inner.width(),
            4.0 + CATEGORY_BAR_HEIGHT + SUGGESTIONS.len() as f32 * row_h,
        ),
    );
    ui.painter()
        .rect_filled(list, 2, design::tokens::SELECT_LIST_FILL);
    ui.painter().rect_stroke(
        list,
        2,
        Stroke::new(2.0, design::tokens::SELECT_LIST_BORDER),
        StrokeKind::Inside,
    );
    // Liseré clair d'un pixel en tête de liste — ce qui la détache de son champ. Peint **après**
    // le bord, jamais avant : le `rect_stroke` de 2 px en `StrokeKind::Inside` le recouvrait
    // intégralement, et la v3 le déclarait appliqué alors qu'aucun pixel n'en restait. Le jeu
    // l'empile dans cet ordre — `select-simple-opened.png`, colonne x=100 : bord `#101215` en
    // y 41-42, liseré `#7e7562` en y=43, fond dès y=44.
    ui.painter().hline(
        list.x_range(),
        list.top() + 2.5,
        Stroke::new(1.0, design::tokens::SELECT_LIST_TOP_LINE),
    );
    // **La bande de filtres par catégorie**, en tête du panneau — elle ne porte QUE les catégories
    // présentes dans les résultats, « Tout » en tête (voir `category_bar`).
    let mut filtres = vec![CategoryFilter::All];
    filtres.extend(CategoryFilter::ITEM_CATEGORIES.iter().copied().filter(|c| {
        SUGGESTIONS
            .iter()
            .any(|(_, _, categorie, _)| categorie == c)
    }));
    category_bar(
        ui,
        cats,
        Rect::from_min_size(
            egui::pos2(list.left() + 2.0, list.top() + 2.0),
            Vec2::new(list.width() - 4.0, CATEGORY_BAR_HEIGHT),
        ),
        &filtres,
        CategoryFilter::All,
    );

    for (i, (name, rarity, _categorie, already)) in SUGGESTIONS.iter().enumerate() {
        let row = Rect::from_min_size(
            egui::pos2(
                list.left() + 2.0,
                list.top() + 2.0 + CATEGORY_BAR_HEIGHT + i as f32 * row_h,
            ),
            Vec2::new(list.width() - 4.0, row_h),
        );
        // Une seule entrée en surbrillance : celle que le clavier désignerait — et **jamais une
        // entrée déjà suivie**, qui n'est pas sélectionnable. Une ligne grisée qui s'allume quand
        // même promet un clic qui n'arrivera pas.
        if i == 1 && !*already {
            ui.painter()
                .rect_filled(row, 0, design::tokens::SELECT_ROW_HIGHLIGHT);
        }
        // **Gemme de rareté, puis image NUE** — l'ordre du web (`wakfu-autocomplete`). Pas de
        // bordure de rareté ici : la gemme la porte déjà, le cadre coloré ferait doublon.
        rarity_gem(
            ui,
            gems,
            Rect::from_center_size(
                egui::pos2(row.left() + ROW_PAD_X + GEM_BOX / 2.0, row.center().y),
                Vec2::splat(GEM_BOX),
            ),
            *rarity,
        );
        let image_x = row.left() + ROW_PAD_X + GEM_BOX + ROW_GAP;
        item_slot(
            ui,
            icons,
            Rect::from_center_size(
                egui::pos2(image_x + ROW_IMAGE / 2.0, row.center().y),
                Vec2::splat(ROW_IMAGE),
            ),
            None,
        );
        ui.painter().text(
            egui::pos2(image_x + ROW_IMAGE + ROW_GAP, row.center().y),
            egui::Align2::LEFT_CENTER,
            *name,
            design::text::label_font(ui.ctx(), 15.0),
            if *already {
                design::tokens::TEXT_DISABLED
            } else {
                design::tokens::SELECT_TEXT
            },
        );
        // « déjà suivi » aligné à DROITE, en second run de texte — la v2 le concaténait au nom
        // dans une seule chaîne séparée par des espaces, la même famille de rustine que les six
        // espaces du champ de recherche.
        if *already {
            ui.painter().text(
                egui::pos2(row.right() - 10.0, row.center().y),
                egui::Align2::RIGHT_CENTER,
                "déjà suivi",
                design::text::label_font(ui.ctx(), 13.0),
                design::tokens::TEXT_DISABLED,
            );
        }
    }
}

/// La boîte de confirmation du jeu — voir [`alertes_options_confirmation_retrait`].
///
/// **Ce n'est pas une popover ancrée au bouton, contrairement au web** (`ConfirmDeleteService`) et
/// contrairement à la v2 : le jeu a sa propre boîte de confirmation, et elle est dans les captures
/// de référence du dépôt — `interface-confirm-box.png` (449 × 209), qui pose exactement la même
/// question (« Êtes-vous sûr(e) de vouloir supprimer ce build ? »). Mesures relevées dessus :
///
/// | | Mesure |
/// | --- | --- |
/// | Forme | boîte **autonome, centrée** sur la fenêtre parente |
/// | Fond du corps | **`#585955`**, un gris CLAIR — pas le kaki d'une liste déroulante |
/// | Corps | x 13..433 pour une capture de 449, y ≈ 47..191 |
/// | Crête | ornement supérieur débordant du corps (médaillon « ? » et volutes) |
/// | Réponses | **« Non » secondaire, « Oui » PRIMAIRE (or)** |
///
/// **Le bouton destructeur du jeu est or, jamais rouge** : `docs/design-system.md` réserve
/// nommément le rouge au bouton « Annuler » pleine largeur d'un pied de fenêtre, et précise que
/// « le bouton "Annuler" d'une boîte de dialogue simple (`interface-confirm-box.png`, bouton
/// "Non") reste kaki/gris standard, pas rouge ». Un bouton rouge « Retirer » — ce que peignait la
/// v2 — est la convention web du *destructive action*, pas celle du jeu.
///
/// Le centrage règle du même coup la réserve d'ergonomie sur la v2 : sa popover recouvrait le
/// bouton « Valider » de la fenêtre, et son bouton « Retirer » tombait exactement là où « Valider »
/// réapparaissait une fois la popover fermée — un double-clic un peu vif validait la fenêtre.
///
/// La crête est peinte à partir de `decoration-top.png` **non détourée du fond de la capture** :
/// elle n'est pas encore au manifeste. C'est le dernier asset que ce portage demande au skill
/// `design-asset`, avec le haut-parleur.
fn confirm_box(ui: &mut egui::Ui, parent: Rect, item_name: &str) {
    // Voile sombre sur **toute** la fenêtre, pied de page compris — une boîte modale du jeu
    // assombrit ce qu'elle interrompt, et ce voile porte une information : tant que le dialogue
    // est ouvert, « Annuler » et « Valider » sont inertes. La v3 le peignait dans le scope du
    // panneau de section, dont le clip le rognait : bannière, onglets et pied restaient à pleine
    // luminosité, ce qui laissait croire qu'on pouvait encore les cliquer.
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(parent));
    ui.set_clip_rect(Rect::EVERYTHING);
    let ui = &mut ui;
    ui.painter()
        .rect_filled(parent, 0, Color32::from_black_alpha(0x88));

    let size = Vec2::new(CONFIRM_WIDTH, CONFIRM_HEIGHT);
    let rect = Rect::from_center_size(parent.center(), size);
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    ui.set_clip_rect(Rect::EVERYTHING);
    let ui = &mut ui;

    ui.painter().rect_filled(rect, 4, CONFIRM_FILL);
    ui.painter().rect_stroke(
        rect,
        4,
        Stroke::new(2.0, CONFIRM_BORDER),
        StrokeKind::Inside,
    );

    // Crête : le médaillon « ? » du jeu déborde le haut du corps. Rendu ici par le glyphe d'aide
    // du manifeste sur un disque, faute d'ornement détouré — la forme, pas le décor.
    let crest = egui::pos2(rect.center().x, rect.top());
    ui.painter().circle_filled(crest, 20.0, CONFIRM_CREST);
    ui.painter()
        .circle_stroke(crest, 20.0, Stroke::new(2.0, CONFIRM_BORDER));
    design::DesignSystem::get(ui.ctx()).paint_icon(
        ui.painter(),
        Rect::from_center_size(crest, Vec2::new(14.0, 14.0)),
        DsIcon::Help,
        design::tokens::BUTTON_TEXT_ON_GOLD,
    );

    ui.painter().text(
        egui::pos2(rect.center().x, rect.top() + 46.0),
        egui::Align2::CENTER_CENTER,
        format!("Retirer « {item_name} » de vos alertes ?"),
        design::text::label_font(ui.ctx(), 15.0),
        CONFIRM_TEXT,
    );

    let mut buttons = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(
        egui::pos2(rect.left() + 26.0, rect.top() + 68.0),
        Vec2::new(rect.width() - 52.0, 36.0),
    )));
    buttons.horizontal(|ui| {
        ui.add(
            design::button("Non")
                .variant(ButtonVariant::Secondary)
                .size(ButtonSize::Compact)
                .width(150.0)
                .log_name("maquette.confirm-non"),
        );
        ui.add_space(14.0);
        ui.add(
            design::button("Oui")
                .variant(ButtonVariant::Primary)
                .size(ButtonSize::Compact)
                .width(150.0)
                .log_name("maquette.confirm-oui"),
        );
    });
}

/// Écrit la capture rendue dans `target/mockups/` — **jamais un snapshot comparé**.
///
/// La revue d'architecture l'a établi : l'étape `overlay-testkit` du CI porte
/// `continue-on-error: true` (§17.3 du plan, pas encore promue en gate), un écart sur ces images
/// ne peut donc rien faire échouer. La comparaison n'achetait aucune protection et coûtait 1 Mo de
/// PNG définitivement dans l'historique d'un dépôt privé, plus un `UPDATE_SNAPSHOTS=1` à rejouer à
/// chaque changement de jeton — sur des images destinées à disparaître avec ce fichier.
///
/// **Le dossier est vidé une fois par exécution** (`Once`) : sans ça une capture d'un rendu
/// renommé y survit indéfiniment, indiscernable d'une capture vivante — et comme CLAUDE.md oblige
/// à publier ces images en Artifact, publier le dossier reviendrait à livrer une image périmée
/// sans le savoir. C'est arrivé entre la v2 et la v3.
///
/// `CARGO_TARGET_DIR` est respecté : le déduire de `CARGO_MANIFEST_DIR` supposerait la disposition
/// par défaut du workspace.
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

/// Ouvre le harnais et rend la fenêtre Options avec son chrome réel, l'onglet « Alertes » actif.
///
/// **Le décor n'est pas recopié : ce sont `design::window` et `design::panel` qui le peignent**,
/// les mêmes composants que la vraie modale. C'était la réserve la plus lourde de la revue
/// d'architecture : une deuxième implémentation du chrome aurait divergé en quelques semaines.
/// Extrait d'abord dans `options_modal::chrome` le 2026-09-10, puis remonté au design system le
/// jour même quand le contrat a gagné sa famille « conteneur ».
fn options_harness(
    size: Vec2,
    mut build: impl FnMut(&mut egui::Ui, &UiIcons, &design::PanelZones, Rect) + 'static,
) -> Harness<'static> {
    let mut icons: Option<UiIcons> = None;
    let mut tab = OptionsTab::Alertes;
    Harness::builder().with_size(size).build_ui(move |ui| {
        overlay_ui::style::apply(ui.ctx());
        // Chargés UNE fois et gardés entre les frames (même motif que `tests/panels.rs`) : les
        // recharger à chaque frame libérerait les `TextureHandle` du frame précédent en fin de
        // closure, et les emplacements de rareté se peindraient vides.
        let icons = icons.get_or_insert_with(|| UiIcons::load(ui.ctx()));
        egui::Frame::NONE.fill(BACKDROP).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let window = ui.max_rect();
            let chrome = design::window("Options")
                .footer("Annuler", "Valider")
                .log_name("maquette")
                .show(ui);
            chrome.tabs(
                ui,
                design::tabs(&mut tab)
                    .entry(OptionsTab::Alertes, "Alertes")
                    .entry(OptionsTab::Personnages, "Personnages")
                    .enabled(false)
                    .entry(OptionsTab::Parametres, "Paramètres")
                    .log_name("maquette-onglets"),
            );
            design::panel().show(ui, chrome.content, |ui, panel| {
                build(ui, icons, panel, window);
            });
        });
    })
}

/// Les onze objets dans l'ordre de la capture d'origine.
const ORDER_NATUREL: [usize; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

/// Ce qu'un rendu de l'onglet a besoin de savoir — un `struct` plutôt que neuf paramètres
/// positionnels, dont l'`#[allow(clippy::too_many_arguments)]` de la v2 était le symptôme.
struct AlertsTab<'a> {
    /// Indices dans [`ITEMS`], dans l'ordre d'affichage — vide pour l'état sans objet.
    order: &'a [usize],
    sounds: &'a mut [bool],
    search: &'a mut String,
    auto: &'a mut bool,
    seconds: &'a mut String,
    empty_state: EmptyState,
    /// Message d'échec du dernier enregistrement, le cas échéant.
    ///
    /// **Au-dessus de la liste, jamais dessous** : un message qu'il faut faire défiler pour voir
    /// n'est pas un message d'échec. La v3 le peignait après `alerts_tab`, dont la liste vit dans
    /// un enfant posé sur un rectangle absolu — qui n'avance pas le curseur du parent : le texte
    /// retombait sur la première ligne et les deux devenaient illisibles.
    error: Option<&'a str>,
}

/// Pourquoi la liste serait vide — **et pourquoi il ne reste qu'un seul cas**.
///
/// Deux des trois situations que rendait la version précédente sont **impossibles ici**, l'un
/// comme l'autre relevés par l'utilisateur (2026-09-11) :
///
/// - *aucun compte connecté* — l'overlay ne s'adresse qu'à des utilisateurs connectés ; il n'y a
///   pas d'état « déconnecté » à peindre ;
/// - *compte connecté, liste réellement vide* — une liste d'objets par défaut est toujours
///   réinjectée et le joueur ne peut pas la retirer, donc la grille n'est jamais vide.
///
/// Reste la synchronisation en vol, qui n'est **pas un message d'information** mais un loader —
/// voir [`loading_placeholder`].
#[derive(Clone, Copy, PartialEq)]
enum EmptyState {
    /// La liste a des objets — le cas courant.
    NotEmpty,
    /// `GET /api/v1/settings` en vol : rien à afficher tant que la réponse n'est pas là.
    Loading,
}

/// Le rouage de chargement, centré dans la zone que la grille occuperait.
///
/// Une synchronisation en vol se signale par un loader, pas par un bloc d'information : c'est un
/// état transitoire, pas une remarque à lire (requalification de l'utilisateur, 2026-09-11). Le
/// tour précédent en réservait seulement la place, `design::loader` n'existant pas encore ; il est
/// arrivé sur `dev` depuis, et la maquette l'utilise.
///
/// `preview_frame` fige l'image de la boucle : sans elle, deux rendus du même écran ne donneraient
/// pas le même pixel, et une planche qui bouge d'un rendu à l'autre ne se compare plus.
///
/// **Centré dans les DEUX axes de la zone restée vide** (demande de l'utilisateur, 2026-09-11) :
/// cette zone va du curseur — juste sous le champ d'ajout — jusqu'au bas du panneau, c'est-à-dire
/// exactement la place qu'occuperait la grille. Posé juste sous le champ comme avant, le rouage
/// laissait tout le bas du panneau désert et se lisait comme un élément de plus dans le flux, pas
/// comme l'état d'une zone entière.
fn loading_row(ui: &mut egui::Ui, inner: Rect) {
    let side = design::LoaderSize::Medium.px();
    let zone = Rect::from_min_max(egui::pos2(inner.left(), ui.cursor().min.y), inner.max);
    let mut cell = ui.new_child(
        egui::UiBuilder::new().max_rect(Rect::from_center_size(zone.center(), Vec2::splat(side))),
    );
    cell.add(
        design::loader()
            .size(design::LoaderSize::Medium)
            .preview_frame(3)
            .log_name("maquette.chargement"),
    );
}

/// Le contenu de l'onglet, commun à tous les rendus.
fn alerts_tab(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    panel: &design::PanelZones,
    state: &mut AlertsTab<'_>,
) -> f32 {
    let inner = panel.inner;
    let width = inner.width();

    // Le bouton d'aide remplace le bloc d'explication permanent de la v2 — deux gains d'un même
    // geste. Il rend deux lignes d'objets à la liste (le panneau de section est court), et il
    // rétablit l'affordance du web (`?` → `help.profileAlerts`), **le seul endroit qui explique
    // pourquoi dix objets n'ont pas de croix**. Sans elle, un joueur lit dix lignes sans croix et
    // une avec, et ça se lit comme un bug.
    // Pas de bouton d'aide ici : le `?` du dépôt web ouvre une modale d'explication, un idiome
    // de page web. Le jeu, lui, n'en pose jamais à côté d'un titre de section — celui de
    // `interface-personnage-equiement.png` est en tête de FENÊTRE, ce qui est un autre objet.
    // Retiré sur demande explicite (2026-09-11).
    ui.add(design::heading("Alerte").trailing_gap(SECTION_GAP * 0.5));
    // La phrase d'explication de la page (`profile.alertsDesc` côté web) — remise sous le titre à
    // la demande de l'utilisateur, après que le retrait du `?` qui la portait en infobulle l'ait
    // fait disparaître.
    //
    // **Un paragraphe, pas un bloc d'information** : `design::info_text` porte une pastille dorée
    // et le poids d'une remarque, ce qui donnait à cette phrase une importance qu'elle n'a pas.
    // C'est une description de section, au même corps et à la même couleur que « Tester le son de
    // l'alerte ».
    text_paragraph(ui, DESC, width);
    ui.add_space(SECTION_GAP);

    // **Deux canaux, deux blocs** : le SON d'abord, le TOAST ensuite — voir `test_sound_row` et
    // `close_settings_row`.
    test_sound_row(ui, width);
    ui.add_space(SECTION_GAP);
    close_settings_row(ui, state.auto, state.seconds, width);

    // **Un seul écart, partout** : `SECTION_GAP` sépare chaque bloc de son voisin, et un titre a
    // exactement le même espace au-dessus qu'en dessous (`trailing_gap`). Les versions
    // précédentes mélangeaient `SECTION_GAP`, sa moitié et des littéraux, ce qui donnait un titre
    // collé à son champ et un autre décollé — et l'écart ne s'ouvrait que lorsqu'un message
    // d'erreur s'intercalait, donc au hasard de l'état.
    ui.add_space(SECTION_GAP);
    // **Sans compteur** : demande explicite. « (11) » n'apprend rien qu'un coup d'œil à la grille
    // ne donne déjà.
    ui.add(design::heading("Objets suivis").trailing_gap(SECTION_GAP));

    // Champ grisé pendant une lecture en vol : ce qu'on ajouterait serait écrasé par la liste
    // qui arrive. Un champ d'apparence active inviterait au geste que le chargement vient
    // précisément de retirer.
    let can_add = state.empty_state != EmptyState::Loading;
    search_field(
        ui,
        state.search,
        "Ajouter un objet à surveiller…",
        width,
        can_add,
    );
    // Bas du champ : l'ancre du panneau de suggestions, rendue à l'appelant.
    let field_bottom = ui.cursor().min.y;
    ui.add_space(SECTION_GAP);

    if let Some(message) = state.error {
        ui.add(
            design::info_text(message)
                .tone(design::InfoTone::Alert)
                .width(width)
                .log_name("maquette.echec"),
        );
        ui.add_space(SECTION_GAP);
    }

    if state.empty_state == EmptyState::Loading {
        loading_row(ui, inner);
        return field_bottom;
    }

    // La grille de tuiles, dans la zone défilable du panneau (géométrie ET clip posés ensemble,
    // largeur utile déduite de la réserve de barre) — autant de tuiles par rangée que la
    // largeur le permet, et une barre dès que la liste déborde.
    panel.scroll_area(ui, "maquette.grille", |ui, content_width| {
        ui.spacing_mut().item_spacing = Vec2::splat(TILE_GAP);
        let per_row =
            (((content_width + TILE_GAP) / (TILE_WIDTH + TILE_GAP)).floor() as usize).max(1);
        for chunk in state.order.chunks(per_row) {
            ui.horizontal(|ui| {
                for &i in chunk {
                    if alert_item(ui, icons, &ITEMS[i], state.sounds[i], true).clicked() {
                        // Le clic bascule l'état du son — et rien d'autre : ni l'emplacement de
                        // rareté, ni le nom ne bougent.
                        state.sounds[i] = !state.sounds[i];
                    }
                }
            });
        }
    });
    field_bottom
}

// -------------------------------------------------------------------------------------------
// 1 & 2 — L'onglet « Alertes » de la fenêtre Options, à deux tailles
// -------------------------------------------------------------------------------------------

/// **L'onglet « Alertes », à la taille que cette page demande** — voir [`WINDOW`].
///
/// Cinq tuiles par rangée, trois rangées visibles : les onze objets tiennent sans défiler, et la
/// barre apparaît dès qu'il y en a davantage. L'objet ajouté par le joueur est remonté en
/// troisième position pour que sa croix de retrait — la seule de la grille — soit dans le champ.
fn alertes_options_onglet() {
    // L'objet ajouté par le joueur remonte en troisième position : c'est le seul qui porte une
    // croix, il doit être dans le champ pour que la capture montre la règle.
    let order: [usize; 11] = [0, 1, 10, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = options_harness(WINDOW, move |ui, icons, panel, _window| {
        alerts_tab(
            ui,
            icons,
            panel,
            &mut AlertsTab {
                order: &order,
                sounds: &mut sounds,
                search: &mut search,
                auto: &mut auto,
                seconds: &mut seconds,
                empty_state: EmptyState::NotEmpty,
                error: None,
            },
        );
    });
    harness.run();
    write_mockup(&mut harness, "alertes_options_onglet");
}

/// **La fermeture manuelle** — le cas où la case est décochée.
///
/// Le champ de durée y est grisé et non modifiable, et c'est la logique fonctionnelle qui
/// l'impose : si la fermeture n'est pas automatique, elle est manuelle, donc il n'y a plus de
/// délai avant fermeture et la valeur n'a plus d'effet. Un champ resté actif inviterait à régler
/// quelque chose qui ne sert à rien.
#[allow(clippy::too_many_lines)]
fn alertes_options_fermeture_manuelle() {
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = false;

    let mut harness = options_harness(WINDOW, move |ui, icons, panel, _window| {
        alerts_tab(
            ui,
            icons,
            panel,
            &mut AlertsTab {
                order: &ORDER_NATUREL,
                sounds: &mut sounds,
                search: &mut search,
                auto: &mut auto,
                seconds: &mut seconds,
                empty_state: EmptyState::NotEmpty,
                error: None,
            },
        );
    });
    harness.run();
    write_mockup(&mut harness, "alertes_options_fermeture_manuelle");
}

/// **La synchronisation en vol** — le seul état où la grille n'a rien à montrer.
///
/// Les deux autres que rendait la version précédente sont **impossibles**, l'un comme l'autre
/// écartés par l'utilisateur (2026-09-11) : l'overlay ne s'adresse qu'à des utilisateurs
/// connectés, donc pas d'état « déconnecté » ; et une liste d'objets par défaut que le joueur ne
/// peut pas retirer est toujours réinjectée, donc jamais de grille réellement vide.
///
/// Reste `GET /api/v1/settings` en vol. Le champ d'ajout est grisé le temps de la lecture — ce
/// qu'on y ajouterait serait écrasé par la liste qui arrive.
fn alertes_options_chargement() {
    let mut sounds: Vec<bool> = Vec::new();
    let mut search = String::new();
    // 3,5 s : le défaut réel du web (`DEFAULT_ALERT_DURATION_SECONDS`). Les autres maquettes
    // montrent 4, la valeur réglée sur la capture fournie par l'utilisateur.
    let mut seconds = String::from("3.5");
    let mut auto = true;

    let mut harness = options_harness(WINDOW, move |ui, icons, panel, _window| {
        alerts_tab(
            ui,
            icons,
            panel,
            &mut AlertsTab {
                order: &[],
                sounds: &mut sounds,
                search: &mut search,
                auto: &mut auto,
                seconds: &mut seconds,
                empty_state: EmptyState::Loading,
                error: None,
            },
        );
    });
    harness.run();
    write_mockup(&mut harness, "alertes_options_chargement");
}

/// **L'échec de l'enregistrement — le corollaire obligatoire du modèle transactionnel.**
///
/// La fenêtre Options ne prend rien en compte tant que « Valider » n'a pas été cliqué ET jugé
/// valide par l'appelant (§5.1 du plan). Dans ce modèle, un échec de commit qui ne se voit pas est
/// le pire cas : le joueur pose un geste explicite d'engagement, la fenêtre se ferme derrière, et
/// il découvrira la perte à la session suivante sans pouvoir la relier à quoi que ce soit.
///
/// La brique existe déjà et n'a rien à inventer : `OptionsModalState::error` alimente
/// `design::info_text` au ton `Alert`, exactement comme pour un chemin de `wakfu.log` invalide.
fn alertes_options_echec_enregistrement() {
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = options_harness(WINDOW, move |ui, icons, panel, _window| {
        alerts_tab(
            ui,
            icons,
            panel,
            &mut AlertsTab {
                order: &ORDER_NATUREL,
                sounds: &mut sounds,
                search: &mut search,
                auto: &mut auto,
                seconds: &mut seconds,
                empty_state: EmptyState::NotEmpty,
                error: Some(
                    "Vos alertes n'ont pas pu être enregistrées sur le compte. Réessayez, ou \
                         vérifiez votre connexion.",
                ),
            },
        );
    });
    harness.run();
    write_mockup(&mut harness, "alertes_options_echec_enregistrement");
}

// -------------------------------------------------------------------------------------------
// 3 — Ajout d'un objet, et confirmation de retrait
// -------------------------------------------------------------------------------------------

/// **Le panneau de suggestions — le vrai mécanisme d'ajout**, que la v1 ne montrait pas.
///
/// Le champ n'accepte pas un nom libre : une alerte a besoin d'un `catalogId` résolu par le
/// catalogue, sans quoi elle ne se déclenchera jamais. Chaque suggestion porte son emplacement de
/// rareté, et une entrée **déjà suivie** est grisée — miroir du web, dont `addSoundItem` retourne
/// en silence sur un doublon, ce qui laisse sinon le joueur retenter cinq fois un ajout qui ne
/// fait rien.
///
/// La liste est peinte APRÈS le contenu, donc par-dessus, comme le fait `design::select` avec sa
/// liste dépliée. Un portage passerait par lui (`egui::Area`), pour qu'elle survive aussi au clip
/// du panneau.
fn alertes_options_ajout_suggestions() {
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::from("pierre");
    let mut seconds = String::from("4");
    let mut auto = true;
    let mut gems: Option<RarityGems> = None;
    let mut cats: Option<CategoryIcons> = None;

    let mut harness = options_harness(WINDOW, move |ui, icons, panel, _w| {
        let gems = gems.get_or_insert_with(|| RarityGems::load(ui.ctx()));
        let cats = cats.get_or_insert_with(|| CategoryIcons::load(ui.ctx()));
        let inner = panel.inner;
        let field_bottom = alerts_tab(
            ui,
            icons,
            panel,
            &mut AlertsTab {
                order: &ORDER_NATUREL,
                sounds: &mut sounds,
                search: &mut search,
                auto: &mut auto,
                seconds: &mut seconds,
                empty_state: EmptyState::NotEmpty,
                error: None,
            },
        );
        suggestion_list(&*ui, icons, gems, cats, inner, field_bottom);
    });
    harness.run();
    write_mockup(&mut harness, "alertes_options_ajout_suggestions");
}

/// **La confirmation de retrait**, parité avec le `ConfirmDeleteService` du web : un clic sur la
/// croix n'efface pas, il demande.
///
/// La v1 supprimait au premier clic, et sur les onze objets — dix d'entre eux étant les défauts
/// que le web protège. Ici, seule « Combinaison Lardante » porte une croix, et elle ouvre cette
/// boîte ancrée sous elle.
///
/// **Aucune brique de ce genre n'existe dans `overlay-ui`** : elle est peinte à la main, et c'est
/// précisément ce que cette maquette sert à chiffrer.
fn alertes_options_confirmation_retrait() {
    // Les deux premiers défauts, puis l'objet ajouté par le joueur — le seul retirable, et donc
    // le seul que la confirmation puisse viser.
    const SHOWN: [usize; 3] = [0, 1, 10];
    let mut sounds: Vec<bool> = ITEMS.iter().map(|i| i.sound_on).collect();
    let mut search = String::new();
    let mut seconds = String::from("4");
    let mut auto = true;

    let mut harness = options_harness(WINDOW, move |ui, icons, panel, window| {
        alerts_tab(
            ui,
            icons,
            panel,
            &mut AlertsTab {
                order: &SHOWN,
                sounds: &mut sounds,
                search: &mut search,
                auto: &mut auto,
                seconds: &mut seconds,
                empty_state: EmptyState::NotEmpty,
                error: None,
            },
        );
        confirm_box(ui, window, ITEMS[10].name);
    });
    harness.run();
    write_mockup(&mut harness, "alertes_options_confirmation_retrait");
}

/// Génère les 7 planches dans `target/mockups/` et les liste.
///
/// **Un exemple et non des `#[test]`** — réserve de la revue d'architecture, et elle est juste :
/// ces rendus n'ont aucune assertion, ils ne peuvent que échouer (y compris pour une raison
/// d'environnement pure : `harness.render()` panique sans pilote Vulkan logiciel), et ils
/// ajoutaient ~10 s à chaque `cargo test -p overlay-testkit`, donc à chaque run du job
/// `test-linux` d'un dépôt privé dont les minutes sont comptées. Un artefact jetable ne doit ni
/// rougir la suite, ni coûter des minutes à chaque commit.
///
/// ```text
/// cargo run -p overlay-testkit --example alertes-mockups
/// ```
fn main() {
    alertes_options_onglet();
    println!("  alertes_options_onglet");
    alertes_options_fermeture_manuelle();
    println!("  alertes_options_fermeture_manuelle");
    alertes_options_chargement();
    println!("  alertes_options_chargement");
    alertes_options_echec_enregistrement();
    println!("  alertes_options_echec_enregistrement");
    alertes_options_ajout_suggestions();
    println!("  alertes_options_ajout_suggestions");
    alertes_options_confirmation_retrait();
    println!("  alertes_options_confirmation_retrait");
    // Le compte des FICHIERS : c'est le seul index du dossier qu'on publiera en Artifact.
    let dir = mockup_dir();
    let ecrites = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    let affiche = dir.canonicalize().unwrap_or_else(|_| dir.clone());
    println!("{ecrites} planches écrites dans {}", affiche.display());
}
