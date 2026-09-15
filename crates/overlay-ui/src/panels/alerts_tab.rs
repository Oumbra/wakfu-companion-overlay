//! **Onglet « Alertes » de la fenêtre Options** — l'écran qui règle les alertes de ramassage :
//! quels objets les déclenchent, et comment le toast se ferme.
//!
//! Il manquait jusqu'au 2026-09-12 : la mécanique existait depuis le 2026-09-02 (son + toast au
//! ramassage d'un objet à son activé, voir `overlay_engine::profile` et `panels::watchlist::
//! toast_card`) mais la liste n'était modifiable que **depuis le site**. L'onglet était affiché
//! désactivé dans le menu — « la partie alerte n'est pas accessible », retour du 2026-09-12.
//!
//! ## Ce fichier est le portage d'une maquette validée
//!
//! La mise en page vient de `crates/overlay-testkit/examples/alertes-mockups.rs`, spécifiée avec
//! l'utilisateur en trois versions (la v1 refusée à l'unanimité par une revue à trois experts).
//! Les cotes, les couleurs et les règles de comportement y sont documentées une par une avec leur
//! provenance — mesure sur une capture du jeu, reprise d'un panneau existant, ou choix assumé. Ce
//! portage ne les redécide pas : il les applique à de vraies données.
//!
//! Les quatre règles que la maquette a établies et que ce fichier tient :
//!
//! 1. **La tuile porte deux informations qui ne se gênent pas.** Le pictogramme dit l'état du
//!    SON ; l'emplacement de rareté dit ce qu'est l'OBJET. Cliquer bascule le son et ne touche que
//!    le premier.
//! 2. **Les dix objets par défaut n'ont pas de croix de retrait** (`SoundItemEntry::is_default`),
//!    parce que le web refuse structurellement de les supprimer. Leur son, lui, se coupe.
//! 3. **Le son et le toast sont deux canaux**, réglés séparément : « Tester le son » d'un côté,
//!    « Fermeture de l'alerte » de l'autre. Les empiler laissait entendre que la durée
//!    s'appliquait au son.
//! 4. **Le retrait ne demande AUCUNE confirmation** — décision du 2026-09-13, qui revient sur la
//!    boîte centrée que la maquette avait posée. La raison : cette fenêtre est déjà
//!    transactionnelle, « Annuler » rattrape tout jusqu'à la validation, et « Valider » est une
//!    seconde garde. Confirmer un geste déjà réversible deux fois, c'est une garde de trop. La
//!    croix devient **rouge au survol** ([`design::tokens::INFO_ALERT`], le seul rouge mesuré du
//!    jeu) : c'est elle qui porte désormais tout l'avertissement.
//!
//! ## Refonte du 2026-09-13 — la tuile devient un emplacement
//!
//! Demande utilisateur explicite : « afficher les objets de la même manière que les éléments
//! suivis dans l'onglet Suivi ». La carte de 118 × 93 px disparaît au profit du seul emplacement
//! de 64 px ([`TILE`]) — voir `panels::suivi_tab::tracked_tile`, dont cette tuile reprend la
//! forme et les gestes. Quatre choses tombent avec la carte, une cinquième change de rôle :
//!
//! - **la bordure d'état** (cyan quand le son est actif, gris quand il est coupé) ;
//! - **le nom sous l'emplacement**, qui se lit désormais en infobulle — et l'infobulle ne dit plus
//!   que le nom, plus ce que le clic fera ;
//! - **la croix permanente**, qui n'apparaît plus qu'au survol, avec un voile, et seulement sur un
//!   objet retirable ;
//! - **le pictogramme du son ACTIF** : le coin haut-gauche ne montre plus que le haut-parleur
//!   barré. Une tuile sans marque est une tuile qui sonnera — c'est lui qui porte seul l'état
//!   depuis que la bordure est partie.
//!
//! Rien n'est ajouté au passage : pas de chiffre, pas de cible, pas de compteur. L'emplacement du
//! Suivi sait en afficher un ([`design::SlotCount`]), une alerte n'en a aucun.
//!
//! ## Transactionnel, comme le reste de la fenêtre
//!
//! Rien n'est écrit tant que « Valider » n'a pas été cliqué (§5.1 du plan) : cet onglet travaille
//! sur un **brouillon** d'[`AlertProfile`] que l'appelant lui prête, et c'est l'appelant qui
//! l'envoie au compte. Un onglet qui appliquerait ses changements immédiatement à côté d'un onglet
//! qui commit donnerait le pire cas — un pied de page dont l'effet dépend de l'onglet affiché.

use egui::{Color32, Rect, RichText, Vec2};
use overlay_engine::{AlertProfile, CatalogIndex, IconRef, WakfuItemCategory, WakfuRarity};

use crate::design::{self, DsIcon, IconContext, InputSize, SlotFrame};
use crate::panels::feature_switch;
use crate::rarity_bridge::to_slot_rarity;
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::ui_icons::UiIcons;

// -------------------------------------------------------------------------------------------
// Jetons — repris de la maquette, où chacun porte sa provenance. Voir sa doc de module pour les
// mesures : ils ne sont pas redécidés ici.
// -------------------------------------------------------------------------------------------

/// Texte courant — blanc, comme tout texte de corps du jeu.
const TEXT: Color32 = Color32::WHITE;

/// Gris des unités et des pictogrammes d'état — `#b8b9ba`, le gris unique du jeu (mesure
/// convergente sur quatre captures, voir `panels::options_modal::SECTION_TITLE_TEXT`).
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);

/// Côté d'une tuile — **l'emplacement d'objet du jeu**, celui du Suivi et du bandeau in-game.
///
/// **Refonte du 2026-09-13** : la tuile n'est plus une carte de 118 × 93 px qui *portait* un
/// emplacement de 64, elle **est** cet emplacement. Demande utilisateur explicite — « afficher les
/// objets de la même manière que les éléments suivis dans l'onglet Suivi ». Tombent avec la carte :
/// son fond, son rayon, sa hauteur calculée sur la ligne de base du nom, sa bordure d'état
/// (cyan/gris) et le nom lui-même, qui se lit désormais en infobulle comme au Suivi.
const TILE: f32 = design::tokens::ITEM_SLOT_SIZE;
/// Gouttière entre deux tuiles — celle du Suivi (`panels::suivi_tab::TILE_GAP`), pas les 10 px de
/// l'ancienne carte.
const TILE_GAP: f32 = 12.0;
/// Côté de la croix de retrait.
const TILE_BADGE: f32 = 14.0;

/// Côté du pictogramme « son coupé » — **18 px, et non les 14 de la croix**.
///
/// Les deux glyphes ne partent pas de la même taille native : `icon-close.png` fait 13 × 14 px et
/// se rend donc à l'échelle 1:1 dans son badge, tandis que `icon-volume-mute.png` fait 26 × 26 px
/// — à 14, il était réduit de moitié, et son trait d'un pixel disparaissait dans l'interpolation.
/// Deux glyphes de même cote n'ont pas le même poids à l'écran quand l'un est réduit et l'autre
/// pas : c'est la cote qu'il fallait ajuster, pas la couleur seule. Demande du 2026-09-13 : « il
/// est vraiment petit et gris noir, c'est compliqué de le voir ».
///
/// **20 et pas 18** : une planche des cinq variantes (14 à 22 px, avec et sans épaississement) a
/// tranché à la vue. En dessous, le haut-parleur reste une tache ; au-dessus, il occupe près de la
/// moitié de la fenêtre de l'icône.
const MUTE_BADGE: f32 = 20.0;
/// Retrait des deux badges depuis leur coin — **8 px, soit 2 px À L'INTÉRIEUR du contour noir**.
///
/// Mesuré au pixel sur une tuile découpée dans `options_suivi_incremental.png` : la marge
/// transparente occupe les pixels 0-1, la bordure de rareté 2-4, le **contour noir** tombe au
/// pixel 5, et la fenêtre de l'icône commence au 6 (c'est exactement
/// [`design::tokens::ITEM_SLOT_BORDER_INNER_RATIO`], 13/128 × 64 ≈ 6,5). Les badges se posent donc
/// à 6 + 2.
///
/// **Trois passes pour arriver ici** (2026-09-13). À 5 px — le retrait du Suivi, hérité de la
/// carte — ils chevauchaient le cadre de rareté ; à 2 px ils se collaient au bord extérieur. La
/// demande était « à deux pixels des bordures noires », c'est-à-dire à l'intérieur, pas à
/// l'extérieur : « elle doit vraiment être présente à l'intérieur du bord, pour qu'il y ait un
/// tout petit espacement entre la bordure intérieure ». Le Suivi garde 5 px de son côté : il n'y
/// a que la croix, et rien n'y est peint dans le coin opposé.
const TILE_BADGE_INSET: f32 = 8.0;

/// Voile d'une tuile SURVOLÉE, sous sa croix — même teinte qu'au Suivi
/// (`panels::suivi_tab::TILE_HOVER_SCRIM`), mais **restreint à la fenêtre de l'icône** : il ne
/// couvre ni le contour noir ni la bordure de rareté (voir [`hover_scrim_rect`]).
const TILE_HOVER_SCRIM: Color32 = Color32::from_black_alpha(0x66);

/// Ombre portée du pictogramme « son coupé » — un noir **adouci**, pas le noir plein.
///
/// Un noir plein ferait au glyphe un liseré dur, visible comme un trait à part ; à cette opacité
/// l'ombre détache le blanc de l'icône qu'il recouvre sans se voir elle-même. C'est ce liseré dur
/// que la première version produisait, autour d'un glyphe gris qui plus est — « une espèce de
/// bordure noire, ça ne le rend vraiment pas lisible ».
const MUTE_SHADOW: Color32 = Color32::from_black_alpha(0xB0);

/// Rouge de la croix sous le pointeur — `INFO_ALERT`, le seul rouge mesuré du jeu.
const REMOVE_HOVER: Color32 = design::tokens::INFO_ALERT;

/// Fond d'une ligne de réglage mise en valeur — l'idiome des lignes d'aptitude du jeu
/// (`interface-personnage-aptitudes.png`, `#26282b` sur un fond de section plus sombre).
const SETTING_ROW_FILL: Color32 = Color32::from_rgb(0x26, 0x28, 0x2B);
const SETTING_ROW_RADIUS: u8 = 4;
/// Hauteur d'une ligne de réglage — les lignes d'aptitude cadencent à 32, portée à 40 : la ligne
/// porte une case à cocher de 20 px et un champ de 25, qu'un fond de 32 serrerait.
const SETTING_ROW_HEIGHT: f32 = 40.0;
/// Hauteur d'une ligne simple — mesurée sur `interface-options-commandes.png` (pas de 39 px).
const ROW_HEIGHT: f32 = 39.0;

const BODY_FONT_SIZE: f32 = 15.0;
/// Aération autour d'un titre de section — 18 px, porté de 12 après un second retour utilisateur.
const SECTION_GAP: f32 = 18.0;

/// La phrase sous le titre « Alerte » — ce que déclenche un ramassage, et **rien d'autre**.
///
/// Elle disait le son seul (`profile.alertsDesc` du dépôt web) et enchaînait sur le geste de la
/// tuile ; deux corrections du 2026-09-13 :
/// - **elle était incomplète** — un ramassage joue un son ET affiche une carte d'alerte à
///   l'écran (`panels::watchlist::toast_card`, confettis compris) ;
/// - **la phrase sur le clic a déménagé** sous « Objets suivis » ([`LIST_DESC`]), où se trouvent
///   justement les tuiles qu'elle décrit.
const DESC: &str = "Au ramassage d'un des objets ci-dessous, un son est joué et une carte \
                    d'alerte s'affiche par-dessus le jeu.";

/// La phrase sous le titre « Objets suivis » — le geste, à côté des tuiles qu'il concerne.
const LIST_DESC: &str = "Cliquez une tuile pour couper ou rétablir son alerte.";

/// Le libellé de la légende, à droite du pictogramme.
const LEGEND_LABEL: &str = "silencieux";
/// Écart entre le pictogramme de la légende et son libellé.
const LEGEND_GAP: f32 = 8.0;

// -------------------------------------------------------------------------------------------
// État et contrat
// -------------------------------------------------------------------------------------------

/// Ce que l'onglet garde d'une frame à l'autre, et qui n'appartient PAS au profil.
///
/// Le profil, lui, est le brouillon que l'appelant prête à [`show`] : il survit au changement
/// d'onglet et c'est « Valider »/« Annuler » qui en décident, pas ce type.
#[derive(Debug, Default, Clone)]
pub struct AlertsTabState {
    /// Saisie du champ d'ajout.
    pub search: String,
    /// Durée de fermeture **telle que tapée** — une chaîne, pas un nombre.
    ///
    /// **Bornée à la validation, jamais à la frappe.** Une version antérieure de la maquette la
    /// bornait à chaque frame, ce qui rendait le champ inutilisable : taper `0.75` donnait `0` →
    /// borné à `0.5` sous les doigts, puis `0.5.` → non parsable → `3.5`. Toute saisie décimale
    /// passe par un état transitoire non parsable ; l'écraser avant qu'elle soit finie interdit
    /// d'écrire la valeur voulue.
    pub duration_input: String,
}

/// Ce que l'onglet a besoin de recevoir pour peindre de vraies données.
pub struct AlertsTabContext<'a> {
    /// **Le brouillon**, modifié en place par les gestes de l'utilisateur (bascule, ajout,
    /// retrait, réglages). Rien n'est envoyé au compte ici — voir la doc de module.
    pub profile: &'a mut AlertProfile,
    /// Pour chercher un objet à ajouter, et pour résoudre rareté et icône des objets listés.
    pub catalog: &'a CatalogIndex,
    pub remote_icons: &'a RemoteIconStore,
    pub remote_icon_textures: &'a mut RemoteIconTextures,
    /// Repli quand l'icône d'un objet n'est pas encore descendue du CDN.
    pub icons: &'a UiIcons,
    /// D'où vient la liste affichée, et si elle est modifiable — voir [`AlertsAvailability`].
    pub availability: AlertsAvailability,
    /// Le rectangle de la FENÊTRE entière, pas du panneau : le voile d'une confirmation doit
    /// couvrir la bannière, les onglets et le pied de page — c'est lui qui dit qu'ils sont
    /// inertes.
    /// **La fonctionnalité est-elle active ?** — brouillon de la case « Activer les alertes » peinte tout
    /// en haut de l'onglet (voir `panels::feature_switch`), pas un réglage que cet onglet
    /// applique : c'est « Valider » qui l'emporte, comme le reste de la fenêtre. Décochée, tout le
    /// contenu sous la case est grisé et inerte.
    pub enabled: &'a mut bool,
    pub window: Rect,
}

/// Dans quel état l'écran se trouve vis-à-vis du compte.
///
/// **Trois cas, et le troisième n'était pas dans la maquette** : elle ne couvrait que le compte
/// lié. Sans compte, la liste d'alertes n'a ni source ni destination — la montrer éditable
/// laisserait croire à un réglage qui ne serait écrit nulle part, et montrer un rouage annoncerait
/// un chargement qui n'a jamais commencé.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AlertsAvailability {
    /// La liste vient du compte et s'édite.
    #[default]
    Ready,
    /// `GET /api/v1/settings` en vol : rien à afficher tant que la réponse n'est pas là, et ce
    /// qu'on ajouterait serait écrasé par la liste qui arrive.
    Loading,
    /// Aucun compte lié — l'overlay n'a rien à lire ni où écrire.
    NoAccount,
}

/// Ce que l'utilisateur vient de demander et que l'onglet ne sait pas faire lui-même.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AlertsTabAction {
    #[default]
    None,
    /// Jouer le son d'alerte — l'appelant seul a le périphérique audio
    /// (`alert_sound::play_loot_alert`).
    TestSound,
}

/// Peint l'onglet dans le panneau de section de la fenêtre Options.
pub fn show(
    ui: &mut egui::Ui,
    panel: &design::PanelZones,
    state: &mut AlertsTabState,
    ctx: &mut AlertsTabContext<'_>,
) -> AlertsTabAction {
    let mut action = AlertsTabAction::None;
    let width = panel.inner.width();

    ui.add(design::heading("Alerte"));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP);

    // Voir `panels::feature_switch` — l'interrupteur grise tout ce qui suit quand il est décoché.
    feature_switch::show(
        ui,
        ctx.enabled,
        "Activer les alertes",
        "Décoché, le ramassage d'un objet ne joue plus de son et n'affiche plus de carte \
         par-dessus le jeu. Votre liste d'objets est conservée.",
        "alertes.activer",
    );

    // **Deux canaux, deux blocs** : le SON d'abord, le TOAST ensuite.
    if test_sound_row(ui, width) {
        action = AlertsTabAction::TestSound;
    }
    ui.add_space(SECTION_GAP);
    close_settings_row(ui, state, ctx.profile, width);

    ui.add_space(SECTION_GAP);
    // **Sans compteur** : « (11) » n'apprend rien qu'un coup d'œil à la grille ne donne déjà.
    ui.add(design::heading("Objets suivis"));
    paragraph(ui, LIST_DESC);
    legend_row(ui);
    ui.add_space(SECTION_GAP);

    add_field(ui, state, ctx, width);
    ui.add_space(SECTION_GAP);

    match ctx.availability {
        AlertsAvailability::Loading => {
            loading_row(ui, panel.inner);
            return action;
        }
        AlertsAvailability::NoAccount => {
            ui.add(
                design::info_text(
                    "Aucun compte lié : les alertes se règlent sur votre compte Wakfu Companion. \
                     Connectez-vous depuis le bandeau de suivi pour les retrouver ici.",
                )
                .tone(design::InfoTone::Info)
                .width(width)
                .log_name("alertes.sans-compte"),
            );
            return action;
        }
        AlertsAvailability::Ready => {}
    }

    tile_grid(ui, panel, ctx);

    action
}

// -------------------------------------------------------------------------------------------
// Blocs
// -------------------------------------------------------------------------------------------

fn paragraph(ui: &mut egui::Ui, text: &str) {
    // **Un paragraphe, pas un bloc d'information** : `design::info_text` porte une pastille dorée
    // et le poids d'une remarque, ce qui donnait à cette phrase une importance qu'elle n'a pas.
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .color(TEXT)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        )
        .wrap_mode(egui::TextWrapMode::Wrap),
    );
}

/// **La légende du pictogramme** — « [haut-parleur barré] silencieux ».
///
/// Une tuile muette ne porte qu'un pictogramme de 14 px dans son coin ; rien ne dit ce qu'il
/// signifie, et une tuile au son actif ne porte AUCUNE marque à comparer. Demande utilisateur du
/// 2026-09-13, dans la foulée du déplacement de [`LIST_DESC`] : la légende suit la phrase qui
/// décrit le geste, juste au-dessus des tuiles.
///
/// Le pictogramme y est peint **exactement comme sur une tuile** (même taille, même blanc, même
/// cerné, voir [`paint_mute_badge`]) : une légende qui ne ressemblerait pas à ce qu'elle légende
/// ne servirait à rien.
fn legend_row(ui: &mut egui::Ui) {
    let hauteur = MUTE_BADGE.max(BODY_FONT_SIZE * 1.4);
    let (_, ligne) = ui.allocate_space(Vec2::new(ui.available_width(), hauteur));
    let ds = design::DesignSystem::get(ui.ctx());
    let glyphe = Rect::from_center_size(
        egui::pos2(ligne.left() + MUTE_BADGE / 2.0, ligne.center().y),
        design::components::icon_button::glyph_fit(
            ds.icon_native_size(DsIcon::VolumeMute),
            MUTE_BADGE,
        ),
    );
    paint_mute_badge(ui, &ds, glyphe);
    ui.painter().text(
        egui::pos2(glyphe.right() + LEGEND_GAP, ligne.center().y),
        egui::Align2::LEFT_CENTER,
        LEGEND_LABEL,
        design::text::label_font(ui.ctx(), BODY_FONT_SIZE),
        SUBDUED,
    );
}

/// La ligne « Tester le son de l'alerte » — l'alerte SONORE, et rien d'autre. Renvoie `true` au
/// clic.
fn test_sound_row(ui: &mut egui::Ui, width: f32) -> bool {
    let row = ui.allocate_space(Vec2::new(width, ROW_HEIGHT)).1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    let mut clicked = false;
    cell.horizontal_centered(|ui| {
        ui.label(
            RichText::new("Tester le son de l'alerte")
                .color(TEXT)
                .size(BODY_FONT_SIZE),
        );
        ui.add_space(12.0);
        clicked = ui
            .add(
                design::icon_button(DsIcon::Volume)
                    .context(IconContext::Panel)
                    .tooltip("Jouer le son d'alerte")
                    .log_name("alertes.tester"),
            )
            .clicked();
    });
    clicked
}

/// Le bloc « Fermeture de l'alerte » — le TOAST, pas le son.
///
/// Case à cocher plutôt que le switch « Auto | Manuelle » du web : le jeu n'a pas de switch à deux
/// positions, son idiome pour un choix binaire est la case.
fn close_settings_row(
    ui: &mut egui::Ui,
    state: &mut AlertsTabState,
    profile: &mut AlertProfile,
    width: f32,
) {
    let row = ui.allocate_space(Vec2::new(width, SETTING_ROW_HEIGHT)).1;
    ui.painter()
        .rect_filled(row, SETTING_ROW_RADIUS, SETTING_ROW_FILL);

    // La case dit « fermeture AUTOMATIQUE », le profil stocke son contraire (`manual_close`, le
    // nom du champ web). La négation vit ici, au plus près de la case, plutôt que dans le moteur
    // où elle rendrait le miroir du web illisible.
    let mut auto = !profile.manual_close;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row.shrink2(Vec2::new(12.0, 0.0))));
    cell.horizontal_centered(|ui| {
        if ui
            .add(design::checkbox(&mut auto, "Fermeture automatique").log_name("alertes.auto"))
            .clicked()
        {
            profile.manual_close = !auto;
        }
        ui.add_space(12.0);
        // **Le champ suit la case** : décochée, la fermeture est manuelle, il n'y a plus de délai
        // et la valeur n'a plus d'effet — le champ est grisé et non modifiable.
        let response = ui.add(
            design::input(&mut state.duration_input)
                .size(InputSize::Standard)
                .width(52.0)
                .enabled(auto)
                .log_name("alertes.duree"),
        );
        // La borne se pose à la PERTE DE FOCUS, pas à la frappe — voir
        // `AlertsTabState::duration_input`. C'est le moment où la saisie est finie, et le seul où
        // corriger « 0 » en « 0,5 » n'empêche pas d'écrire « 0,75 ».
        if response.lost_focus() {
            profile.set_duration(parse_duration(
                &state.duration_input,
                profile.duration_seconds,
            ));
            state.duration_input = format_duration(profile.duration_seconds);
        }
        ui.label(RichText::new("sec.").color(SUBDUED).size(BODY_FONT_SIZE));
    });
}

/// Lit une durée tapée — virgule décimale comprise.
///
/// **La virgule est le séparateur décimal d'un clavier français**, et ce champ est rempli en jeu,
/// au pavé numérique. La refuser renverrait la valeur de repli sur une saisie parfaitement
/// légitime.
///
/// Une saisie vide ou illisible garde la valeur en place plutôt que de retomber sur le défaut :
/// vider un champ par mégarde ne doit pas réécrire un réglage.
fn parse_duration(raw: &str, actuelle: f32) -> f32 {
    raw.trim()
        .replace(',', ".")
        .parse::<f32>()
        .unwrap_or(actuelle)
}

/// Écrit une durée dans le champ — sans décimale inutile (« 4 » plutôt que « 4.0 »), et avec la
/// virgule française qu'on vient d'accepter en entrée.
fn format_duration(seconds: f32) -> String {
    if (seconds.fract()).abs() < f32::EPSILON {
        format!("{}", seconds as i64)
    } else {
        format!("{seconds:.1}").replace('.', ",")
    }
}

/// Le champ d'ajout et son panneau de suggestions.
fn add_field(
    ui: &mut egui::Ui,
    state: &mut AlertsTabState,
    ctx: &mut AlertsTabContext<'_>,
    width: f32,
) {
    // Champ grisé pendant une lecture en vol : ce qu'on ajouterait serait écrasé par la liste qui
    // arrive. Un champ d'apparence active inviterait au geste que le chargement vient de retirer.
    let enabled = ctx.availability == AlertsAvailability::Ready;
    // **Toutes les correspondances, comme le web** — jusqu'au 2026-09-12 la liste était coupée à
    // quarante, et la bande de filtres, calculée sur cette liste tronquée, perdait des catégories
    // pourtant présentes : « bouftou » montrait cinq boutons ici contre huit sur le site. La
    // recherche coûte moins d'une demi-milliseconde pour cent quinze résultats sur seize mille
    // objets (mesuré sur le catalogue réel) ; c'est le panneau qui défile, pas la liste qui se
    // taille.
    let suggestions = if enabled {
        ctx.catalog.search_items(
            &state.search,
            design::tokens::AUTOCOMPLETE_MIN_QUERY_LEN,
            usize::MAX,
        )
    } else {
        Vec::new()
    };

    // **La bande d'abord, les entrées ensuite** — l'ordre de ces deux blocs est celui des demandes
    // d'icônes au CDN, et `RemoteIconStore` sert une frame dans son ordre de demande : huit
    // icônes de catégorie qui coiffent toute la liste doivent partir avant cent images de
    // rangées dont cinq seulement sont visibles. Demandées après, elles arrivaient les dernières
    // et la bande restait vide plusieurs secondes (constaté sur « tofu », 118 résultats).
    //
    // **La bande se calcule sur la liste NON filtrée** (règle 4 du composant) : elle doit rester
    // entière quand le filtre ne laisse rien passer, sinon le bouton qui permettrait de le
    // relâcher disparaîtrait avec les résultats.
    let mut categories: Vec<WakfuItemCategory> =
        suggestions.iter().map(|item| item.category).collect();
    categories.dedup_by(|a, b| a == b);
    categories.sort_by_key(|c| c.icon_number());
    categories.dedup();
    let mut filters = vec![design::AutocompleteFilter::all(
        "Toutes les catégories",
        texture_id(ui, ctx, &IconRef::for_all_categories()),
    )];
    for category in categories {
        filters.push(design::AutocompleteFilter::category(
            category_key(category),
            category_label(category),
            texture_id(ui, ctx, &IconRef::for_item_category(category)),
        ));
    }

    // Les entrées et la bande de filtres, dans le vocabulaire du composant. Les images viennent du
    // CDN par le même circuit que les tuiles du Suivi : absentes tant qu'elles descendent, la
    // rangée se peint sans elles.
    let entries: Vec<design::AutocompleteEntry> = suggestions
        .iter()
        .map(|item| {
            let deja = ctx
                .profile
                .sound_items
                .iter()
                .any(|e| e.catalog_id == Some(item.id) || e.name == item.name);
            let mut entry =
                design::AutocompleteEntry::new(item.name.clone(), category_key(item.category));
            // La gemme AVEC sa taille native : le composant la pose dans sa boîte de 14 à son
            // rapport (13 × 20 → 9 × 14, comme `object-fit: contain` sur le web). Sans elle, il
            // la prenait pour un carré et l'écrasait en 14 × 14 — « très fortement agrandies et
            // aplaties », retour du 2026-09-12 au soir.
            if let Some((id, size)) = texture(ui, ctx, &IconRef::for_rarity(item.rarity)) {
                entry.gem = Some(id);
                entry.gem_size = size;
            }
            entry.image = texture_id(ui, ctx, &item.icon);
            entry.disabled = deja;
            if deja {
                entry.mention = Some("déjà dans vos alertes".to_string());
            }
            entry
        })
        .collect();

    let outcome = design::autocomplete(&mut state.search)
        .placeholder("Ajouter un objet à surveiller…")
        .width(width)
        .enabled(enabled)
        .entries(&entries)
        .filters(&filters)
        .log_name("alertes.ajout")
        .show(ui);

    if let Some(index) = outcome.selected {
        if let Some(item) = suggestions.get(index) {
            // `catalog_id` porté dès l'ajout : c'est lui qui distingue deux homonymes de raretés
            // différentes, ici comme au Suivi.
            ctx.profile.add(&item.name, Some(item.id));
        }
    }
}

/// La grille de tuiles, dans la zone défilable du panneau.
fn tile_grid(ui: &mut egui::Ui, panel: &design::PanelZones, ctx: &mut AlertsTabContext<'_>) {
    // Le rendu lit le profil et les gestes le modifient : les collecter d'abord évite d'emprunter
    // `ctx.profile` en lecture et en écriture dans la même boucle.
    let items: Vec<TileData> = ctx
        .profile
        .sound_items
        .iter()
        .map(|entry| TileData {
            name: entry.name.clone(),
            catalog_id: entry.catalog_id,
            enabled: entry.enabled,
            is_default: entry.is_default,
            rarity: ctx.catalog.find_item_rarity(&entry.name, entry.catalog_id),
            icon: ctx.catalog.find_item_icon(&entry.name, entry.catalog_id),
        })
        .collect();

    let mut toggled: Option<(String, Option<i64>)> = None;
    let mut removal: Option<(String, Option<i64>)> = None;

    panel.scroll_area(ui, "alertes.grille", |ui, content_width| {
        ui.spacing_mut().item_spacing = Vec2::splat(TILE_GAP);
        let per_row = (((content_width + TILE_GAP) / (TILE + TILE_GAP)).floor() as usize).max(1);
        for chunk in items.chunks(per_row) {
            ui.horizontal(|ui| {
                for item in chunk {
                    match alert_item(ui, ctx, item) {
                        TileClick::Toggle => toggled = Some((item.name.clone(), item.catalog_id)),
                        TileClick::Remove => removal = Some((item.name.clone(), item.catalog_id)),
                        TileClick::None => {}
                    }
                }
            });
        }
    });

    if let Some((name, catalog_id)) = toggled {
        ctx.profile.toggle(&name, catalog_id);
    }
    // **Le retrait est immédiat — sur le BROUILLON, pas sur le compte.** Rien ne part au réseau
    // avant « Valider », et « Annuler » rend la liste telle qu'elle était : le geste est déjà
    // réversible deux fois, une boîte de confirmation par-dessus n'ajoutait qu'un clic.
    if let Some((name, catalog_id)) = removal {
        ctx.profile.remove(&name, catalog_id);
    }
}

/// Ce qu'une tuile a besoin de savoir pour se peindre — assemblé avant la boucle, voir
/// [`tile_grid`].
struct TileData {
    name: String,
    catalog_id: Option<i64>,
    enabled: bool,
    is_default: bool,
    rarity: WakfuRarity,
    icon: Option<IconRef>,
}

/// Ce qu'un clic sur une tuile signifie.
enum TileClick {
    None,
    Toggle,
    Remove,
}

/// **La tuile d'un objet en alerte — un emplacement d'objet, et rien d'autre.**
///
/// | Élément | Ce qu'il dit |
/// | --- | --- |
/// | Cadre | ce qu'est l'objet : la bordure de sa **rareté** |
/// | Coin haut-gauche | le son est **coupé** — et rien du tout quand il est actif |
/// | Coin haut-droit | retrait — **seulement sur une tuile retirable ET survolée**, rouge sous le pointeur |
/// | Voile | le survol, et un fond assez sombre pour la croix — **retirables uniquement** |
///
/// Ni nom, ni chiffre : le nom se lit en infobulle, et une alerte n'a ni compteur ni cible
/// (contrairement au Suivi, dont l'emplacement sait afficher une cible de décompte).
fn alert_item(ui: &mut egui::Ui, ctx: &mut AlertsTabContext<'_>, item: &TileData) -> TileClick {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(TILE), egui::Sense::click());

    let icon_id = item
        .icon
        .as_ref()
        .and_then(|icon| texture_id(ui, ctx, icon))
        .unwrap_or_else(|| ctx.icons.unknown_entity_texture().id());

    // **`ui.put` dans un ENFANT, jamais sur le `ui` de la rangée** : `Ui::put` ouvre un scope, et
    // un scope avance le curseur du parent — la tuile suivante démarrerait au mauvais endroit.
    let mut cellule = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    cellule.put(
        rect,
        design::item_slot()
            .size(TILE)
            .frame(SlotFrame::Rarity(to_slot_rarity(item.rarity)))
            .icon(icon_id)
            .log_name(item.name.clone()),
    );

    let ds = design::DesignSystem::get(ui.ctx());

    // **`contains_pointer` et NON `hovered`.** La croix a sa propre zone interactive, posée
    // par-dessus la tuile : dès que le pointeur l'atteint, egui donne le survol à cette zone et
    // `response.hovered()` retombe à faux — la croix disparaîtrait à l'instant précis où l'on
    // vise. Piège déjà payé au Suivi, voir `panels::suivi_tab::tracked_tile`.
    //
    // **Et seulement sur un objet retirable** : les dix objets par défaut n'ont pas de croix, donc
    // rien à révéler — « le voile ne concerne que les objets pouvant être supprimés » (retour du
    // 2026-09-13). Un voile sans croix annoncerait une action qui n'existe pas.
    let survol_retirable = !item.is_default && response.contains_pointer();
    if survol_retirable {
        ui.painter()
            .rect_filled(hover_scrim_rect(rect), 0.0, TILE_HOVER_SCRIM);
    }

    // Pictogramme du son — **peint APRÈS le voile**, sinon celui-ci l'assombrirait avec l'icône.
    // Affiché seulement quand le son est COUPÉ : une tuile sans marque est une tuile qui sonnera
    // (décision du 2026-09-13, en remplacement de la bordure d'état cyan/gris).
    if !item.enabled {
        paint_mute_badge(
            ui,
            &ds,
            badge_rect(
                ds.icon_native_size(DsIcon::VolumeMute),
                rect,
                Corner::Left,
                MUTE_BADGE,
            ),
        );
    }

    let mut clic = TileClick::None;
    if survol_retirable {
        // **Sa propre zone cliquable, avec sa propre main et sa propre infobulle.** Elle mange
        // aussi le clic, pour qu'un retrait n'emporte pas au passage la bascule du son.
        let zone_rect = Rect::from_center_size(
            badge_rect(Vec2::splat(TILE_BADGE), rect, Corner::Right, TILE_BADGE).center(),
            Vec2::splat(TILE_BADGE + 4.0),
        );
        let croix = ui
            .interact(zone_rect, response.id.with("retirer"), egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        ds.paint_icon(
            ui.painter(),
            badge_rect(
                ds.icon_native_size(DsIcon::Close),
                rect,
                Corner::Right,
                TILE_BADGE,
            ),
            DsIcon::Close,
            // Rouge sous le pointeur — depuis que le retrait ne demande plus confirmation, c'est
            // la croix qui doit dire ce qu'elle fait AVANT le clic.
            if croix.hovered() { REMOVE_HOVER } else { TEXT },
        );
        design::tooltip(&croix).text("Retirer du suivi");
        if croix.clicked() {
            return TileClick::Remove;
        }
        if croix.hovered() {
            return TileClick::None;
        }
    }

    // **Le nom, et rien que le nom** (demande du 2026-09-13). L'infobulle disait aussi ce que le
    // clic ferait (« Cliquer pour couper ») ; le pictogramme porte déjà l'état, et le nom n'est
    // plus écrit nulle part ailleurs depuis qu'il a quitté la tuile. Du même coup disparaît
    // l'exclusion des deux infobulles par position de pointeur, que le libellé peint imposait.
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    design::tooltip(&response).text(&item.name);
    if response.clicked() {
        clic = TileClick::Toggle;
    }
    clic
}

/// Le coin d'une tuile où se pose un badge.
#[derive(Clone, Copy)]
enum Corner {
    Left,
    Right,
}

/// Où peindre un badge de [`TILE_BADGE`] px dans son coin, à [`TILE_BADGE_INSET`] des deux bords.
///
/// `native` est la taille native du glyphe : `glyph_fit` l'inscrit dans le carré du badge sans le
/// déformer, exactement comme les boutons icône du design system.
fn badge_rect(native: Vec2, tile: Rect, corner: Corner, side: f32) -> Rect {
    let x = match corner {
        Corner::Left => tile.left() + TILE_BADGE_INSET + side / 2.0,
        Corner::Right => tile.right() - TILE_BADGE_INSET - side / 2.0,
    };
    Rect::from_center_size(
        egui::pos2(x, tile.top() + TILE_BADGE_INSET + side / 2.0),
        design::components::icon_button::glyph_fit(native, side),
    )
}

/// Peint le pictogramme « son coupé » — **blanc, et détaché de ce qu'il y a dessous**.
///
/// Deux couches : une ombre noire aux quatre décalages d'un pixel, puis le glyphe en [`TEXT`] —
/// **le blanc de la croix**, et non le gris [`SUBDUED`] d'avant : « il faut qu'il ait la même
/// couleur que la croix » (2026-09-13).
///
/// **Sans épaississement.** Une version intermédiaire repeignait le glyphe blanc aux mêmes quatre
/// décalages pour lui donner du corps ; la planche de variantes l'a écartée : à 20 px le trait
/// d'`icon-volume-mute.png` mesure déjà plus d'un pixel, une passe de plus bouche le creux du
/// haut-parleur et le glyphe devient une tache blanche. C'est la COTE qui le rend lisible, pas le
/// gras.
fn paint_mute_badge(ui: &egui::Ui, ds: &design::DesignSystem, rect: Rect) {
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        ds.paint_icon(
            ui.painter(),
            rect.translate(Vec2::new(dx, dy)),
            DsIcon::VolumeMute,
            MUTE_SHADOW,
        );
    }
    ds.paint_icon(ui.painter(), rect, DsIcon::VolumeMute, TEXT);
}

/// La fenêtre de l'icône — **tout ce que le voile de survol a le droit de couvrir**.
///
/// Le Suivi peint le sien sur le carré entier ; ici c'est un défaut visible, signalé sur la
/// maquette du 2026-09-13 : « le voile dépasse sur la partie inférieure et la partie de droite,
/// ce qui n'est pas possible ». Il doit s'arrêter au contour noir, à
/// [`design::tokens::ITEM_SLOT_BORDER_INNER_RATIO`] du bord — le même ratio que la fenêtre où
/// `item_slot` inscrit l'icône, donc la même limite exactement.
fn hover_scrim_rect(tile: Rect) -> Rect {
    tile.shrink(tile.width() * design::tokens::ITEM_SLOT_BORDER_INNER_RATIO)
}

/// Le rouage de chargement, centré dans les DEUX axes de la zone que la grille occuperait.
///
/// Une synchronisation en vol se signale par un loader, pas par un bloc d'information : c'est un
/// état transitoire, pas une remarque à lire.
fn loading_row(ui: &mut egui::Ui, inner: Rect) {
    let reste = Rect::from_min_max(
        egui::pos2(inner.left(), ui.cursor().top()),
        inner.right_bottom(),
    );
    let mut zone = ui.new_child(egui::UiBuilder::new().max_rect(reste));
    zone.put(
        Rect::from_center_size(reste.center(), Vec2::splat(design::LoaderSize::Large.px())),
        design::loader().size(design::LoaderSize::Large),
    );
}

// -------------------------------------------------------------------------------------------
// Utilitaires
// -------------------------------------------------------------------------------------------

/// La clé de catégorie que le composant d'autocomplétion manipule — le numéro d'icône de la
/// catégorie, c'est-à-dire ce qui distingue déjà deux filtres à l'écran.
fn category_key(category: WakfuItemCategory) -> u16 {
    category.icon_number() as u16
}

/// Le libellé d'infobulle d'un filtre de catégorie.
fn category_label(category: WakfuItemCategory) -> &'static str {
    match category {
        WakfuItemCategory::Equipment => "Équipements",
        WakfuItemCategory::Resources => "Ressources",
        WakfuItemCategory::Sublimations => "Sublimations",
        WakfuItemCategory::Harvests => "Récoltes",
        WakfuItemCategory::HavenBag => "Havre-sac",
        WakfuItemCategory::Cosmetics => "Costumes",
        WakfuItemCategory::Craft => "Artisanat",
        WakfuItemCategory::Misc => "Divers",
    }
}

/// La texture d'une icône distante, si elle est déjà descendue du CDN — `None` sinon, et
/// l'appelant se peint sans elle.
fn texture_id(
    ui: &egui::Ui,
    ctx: &mut AlertsTabContext<'_>,
    icon: &IconRef,
) -> Option<egui::TextureId> {
    texture(ui, ctx, icon).map(|(id, _)| id)
}

/// La texture ET sa taille native, pour ce qui doit être peint à son rapport — la gemme de rareté.
fn texture(
    ui: &egui::Ui,
    ctx: &mut AlertsTabContext<'_>,
    icon: &IconRef,
) -> Option<(egui::TextureId, Vec2)> {
    ctx.remote_icon_textures
        .resolve(ui.ctx(), ctx.remote_icons, icon)
        .map(|handle| (handle.id(), handle.size_vec2()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_duree_se_tape_a_la_virgule_comme_au_point() {
        // Le champ est rempli en jeu, au pavé numérique d'un clavier français : refuser la virgule
        // renverrait la valeur de repli sur une saisie parfaitement légitime.
        assert_eq!(parse_duration("1,5", 3.5), 1.5);
        assert_eq!(parse_duration("1.5", 3.5), 1.5);
        assert_eq!(parse_duration(" 2 ", 3.5), 2.0);
    }

    #[test]
    fn une_saisie_vide_garde_la_valeur_en_place() {
        // Vider un champ par mégarde ne doit pas réécrire un réglage.
        assert_eq!(parse_duration("", 2.0), 2.0);
        assert_eq!(parse_duration("abc", 2.0), 2.0);
    }

    #[test]
    fn une_duree_entiere_s_ecrit_sans_decimale() {
        assert_eq!(format_duration(4.0), "4");
        assert_eq!(format_duration(3.5), "3,5");
    }

    #[test]
    fn chaque_categorie_a_sa_propre_cle_de_filtre() {
        // Deux catégories qui partageraient une clé se fondraient en un seul bouton de filtre, et
        // l'un des deux jeux d'objets deviendrait inatteignable.
        let cles: std::collections::HashSet<u16> = [
            WakfuItemCategory::Equipment,
            WakfuItemCategory::Resources,
            WakfuItemCategory::Sublimations,
            WakfuItemCategory::Harvests,
            WakfuItemCategory::HavenBag,
            WakfuItemCategory::Cosmetics,
            WakfuItemCategory::Craft,
            WakfuItemCategory::Misc,
        ]
        .iter()
        .map(|c| category_key(*c))
        .collect();
        assert_eq!(cles.len(), 8);
    }
}
