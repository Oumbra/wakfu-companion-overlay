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
//! 1. **La tuile porte deux informations qui ne se gênent pas.** La bordure et le pictogramme
//!    disent l'état du SON ; l'emplacement de rareté et le nom disent ce qu'est l'OBJET. Cliquer
//!    bascule le son et ne touche que les deux premiers.
//! 2. **Les dix objets par défaut n'ont pas de croix de retrait** (`SoundItemEntry::is_default`),
//!    parce que le web refuse structurellement de les supprimer. Leur son, lui, se coupe.
//! 3. **Le son et le toast sont deux canaux**, réglés séparément : « Tester le son » d'un côté,
//!    « Fermeture de l'alerte » de l'autre. Les empiler laissait entendre que la durée
//!    s'appliquait au son.
//! 4. **Le retrait demande confirmation**, dans la boîte centrée du jeu — pas une popover ancrée
//!    au bouton comme le web, et son bouton de confirmation est **or, jamais rouge** : le rouge
//!    est réservé au « Annuler » pleine largeur d'un pied de fenêtre.
//!
//! ## Transactionnel, comme le reste de la fenêtre
//!
//! Rien n'est écrit tant que « Valider » n'a pas été cliqué (§5.1 du plan) : cet onglet travaille
//! sur un **brouillon** d'[`AlertProfile`] que l'appelant lui prête, et c'est l'appelant qui
//! l'envoie au compte. Un onglet qui appliquerait ses changements immédiatement à côté d'un onglet
//! qui commit donnerait le pire cas — un pied de page dont l'effet dépend de l'onglet affiché.

use egui::{Color32, Rect, RichText, Stroke, StrokeKind, Vec2};
use overlay_engine::{AlertProfile, CatalogIndex, IconRef, WakfuItemCategory, WakfuRarity};

use crate::design::{self, DsIcon, IconContext, InputSize, SlotFrame};
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

/// Largeur d'une tuile — calée pour que cinq tiennent sur une rangée dans la fenêtre agrandie.
const TILE_WIDTH: f32 = 118.0;
/// Hauteur d'une tuile : rangée de badges, emplacement d'objet, nom sur **une** ligne, marges.
///
/// Portée à 104 px un moment le 2026-09-12, pour un nom sur deux lignes — **revenu en arrière le
/// jour même, sur décision de l'utilisateur** : la tuile porte déjà l'icône de l'objet, et c'est
/// elle qui lève l'ambiguïté entre deux noms proches, bien avant le texte. Faire grandir chaque
/// tuile pour distinguer « Plan "Epée de Bonta" » de « … Brâkmar » résolvait un problème que
/// l'utilisateur n'a pas. Le nom coupé se lit en infobulle — voir `design::label`.
const TILE_HEIGHT: f32 = 88.0;
/// Gouttière entre deux tuiles — « les petites tuiles doivent être séparées sur tous les bords ».
const TILE_GAP: f32 = 10.0;
const TILE_BORDER_WIDTH: f32 = 2.0;
const TILE_RADIUS: u8 = 4;
/// Fond d'une tuile — le fond de panneau du jeu, pour que la bordure d'état porte seule le signal.
const TILE_FILL: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
/// Côté de l'emplacement d'objet — l'icône y est bien plus grande que les 32 px du web : c'est ce
/// que « afficher les objets comme dans le jeu » demande.
const TILE_SLOT: f32 = 44.0;
const TILE_BADGE_ROW: f32 = 18.0;
const TILE_BADGE: f32 = 14.0;
const TILE_BADGE_INSET: f32 = 5.0;
const TILE_NAME_INSET: f32 = 5.0;

/// Couleur du nom d'objet — **l'or du jeu** ([`design::tokens::TEXT_GOLD`]), la même teinte que le
/// libellé d'une case cochée. Demande explicite du 2026-09-12.
///
/// Le blanc qu'il portait avant le mettait sur le même plan que la description de la section et le
/// libellé « Tester le son de l'alerte », qui sont du texte courant. Un nom d'objet est une
/// **donnée**, pas une phrase — c'est le même rôle que la valeur saisie dans un champ, qui est en
/// or pour cette raison (voir [`design::tokens::INPUT_TEXT`]).
const TILE_NAME_TEXT: Color32 = design::tokens::TEXT_GOLD;

/// Bordure d'une tuile dont le son est ACTIF — `--accent` du dépôt web.
const ACCENT: Color32 = Color32::from_rgb(0x00, 0xD2, 0xFF);
/// Bordure d'une tuile dont le son est COUPÉ — le gris de bord des panneaux.
const MUTED_BORDER: Color32 = Color32::from_rgb(0x4D, 0x4D, 0x4D);

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

/// La phrase sous le titre — `profile.alertsDesc` du dépôt web.
const DESC: &str =
    "Un son est joué au ramassage des objets ci-dessous. Cliquez une tuile pour couper ou \
     rétablir son alerte.";

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
    /// L'objet dont le retrait attend une confirmation — la boîte du jeu est ouverte tant que ce
    /// champ est renseigné.
    pub pending_removal: Option<PendingRemoval>,
}

/// L'objet visé par une confirmation de retrait en cours.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingRemoval {
    pub name: String,
    pub catalog_id: Option<i64>,
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

    ui.add(design::heading("Alerte").trailing_gap(SECTION_GAP * 0.5));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP);

    // **Deux canaux, deux blocs** : le SON d'abord, le TOAST ensuite.
    if test_sound_row(ui, width) {
        action = AlertsTabAction::TestSound;
    }
    ui.add_space(SECTION_GAP);
    close_settings_row(ui, state, ctx.profile, width);

    ui.add_space(SECTION_GAP);
    // **Sans compteur** : « (11) » n'apprend rien qu'un coup d'œil à la grille ne donne déjà.
    ui.add(design::heading("Objets suivis").trailing_gap(SECTION_GAP));

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

    tile_grid(ui, panel, state, ctx);

    // La confirmation est peinte EN DERNIER et sur la fenêtre entière : son voile doit passer
    // par-dessus tout ce qu'elle interrompt, pied de page compris.
    if let Some(pending) = state.pending_removal.clone() {
        let choix =
            design::confirm_dialog(format!("Retirer « {} » de vos alertes ?", pending.name))
                .over(ctx.window)
                .log_name("alertes.retrait")
                .show(ui);
        match choix {
            design::ConfirmChoice::Yes => {
                ctx.profile.remove(&pending.name, pending.catalog_id);
                state.pending_removal = None;
            }
            design::ConfirmChoice::No => state.pending_removal = None,
            design::ConfirmChoice::Pending => {}
        }
    }

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
            entry.gem = texture_id(ui, ctx, &IconRef::for_rarity(item.rarity));
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
fn tile_grid(
    ui: &mut egui::Ui,
    panel: &design::PanelZones,
    state: &mut AlertsTabState,
    ctx: &mut AlertsTabContext<'_>,
) {
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
    let mut removal: Option<PendingRemoval> = None;

    panel.scroll_area(ui, "alertes.grille", |ui, content_width| {
        ui.spacing_mut().item_spacing = Vec2::splat(TILE_GAP);
        let per_row =
            (((content_width + TILE_GAP) / (TILE_WIDTH + TILE_GAP)).floor() as usize).max(1);
        for chunk in items.chunks(per_row) {
            ui.horizontal(|ui| {
                for item in chunk {
                    match alert_item(ui, ctx, item) {
                        TileClick::Toggle => toggled = Some((item.name.clone(), item.catalog_id)),
                        TileClick::Remove => {
                            removal = Some(PendingRemoval {
                                name: item.name.clone(),
                                catalog_id: item.catalog_id,
                            })
                        }
                        TileClick::None => {}
                    }
                }
            });
        }
    });

    if let Some((name, catalog_id)) = toggled {
        ctx.profile.toggle(&name, catalog_id);
    }
    // **Le retrait passe TOUJOURS par la confirmation**, jamais directement : c'est l'action
    // destructrice de cet écran, et la fenêtre se ferme derrière « Valider ».
    if removal.is_some() {
        state.pending_removal = removal;
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

/// **La tuile d'un objet suivi.**
///
/// | Élément | Ce qu'il dit |
/// | --- | --- |
/// | Bordure 2 px arrondie | l'état du SON : [`ACCENT`] actif, [`MUTED_BORDER`] coupé |
/// | Pictogramme haut-gauche | le même état — `Volume` / `VolumeMute` |
/// | Croix haut-droite | retrait — **seulement si l'objet n'est pas un défaut** |
/// | Emplacement à bordure de rareté | ce qu'est l'objet |
/// | Nom sous l'emplacement | ce qu'est l'objet, en toutes lettres |
fn alert_item(ui: &mut egui::Ui, ctx: &mut AlertsTabContext<'_>, item: &TileData) -> TileClick {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(TILE_WIDTH, TILE_HEIGHT), egui::Sense::click());

    let state_color = if item.enabled { ACCENT } else { MUTED_BORDER };
    ui.painter().rect_filled(rect, TILE_RADIUS, TILE_FILL);
    ui.painter().rect_stroke(
        rect,
        TILE_RADIUS,
        Stroke::new(TILE_BORDER_WIDTH, state_color),
        StrokeKind::Inside,
    );

    let icon_id = item
        .icon
        .as_ref()
        .and_then(|icon| texture_id(ui, ctx, icon))
        .unwrap_or_else(|| ctx.icons.unknown_entity_texture().id());
    let slot = Rect::from_center_size(
        egui::pos2(
            rect.center().x,
            rect.top() + TILE_BADGE_ROW + TILE_SLOT / 2.0,
        ),
        Vec2::splat(TILE_SLOT),
    );
    // **`ui.put` dans un ENFANT, jamais sur le `ui` de la rangée.**
    //
    // `Ui::put` ouvre un scope, et un scope **avance le curseur du parent** jusqu'au bord de ce
    // qu'il a posé. Appelé directement sur la rangée, il ramenait donc le curseur au bord droit de
    // l'EMPLACEMENT (centré, donc 27 px avant le bord de la tuile) : la tuile suivante démarrait
    // 27 px trop tôt et son fond opaque effaçait la fin du nom de la précédente. C'est ce qui
    // faisait lire « Pierre d'avent » et trois « Plan "Epée de » identiques — le nom était bien
    // mis en page, il était recouvert.
    let mut cellule = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    cellule.put(
        slot,
        design::item_slot()
            .size(TILE_SLOT)
            .frame(SlotFrame::Rarity(to_slot_rarity(item.rarity)))
            .icon(icon_id)
            .log_name(item.name.clone()),
    );

    // **Le nom passe par `design::label`** : une ligne, ellipse au bout, et l'infobulle qui rend
    // le nom entier quand il est coupé — c'est le composant qui porte les trois, pas la tuile.
    //
    // Posé dans la CELLULE, comme l'emplacement : `ui.add` avancerait le curseur de la rangée et
    // décalerait la tuile suivante (voir le commentaire de l'emplacement ci-dessus).
    let largeur_nom = rect.width() - 2.0 * TILE_NAME_INSET;
    let hauteur_nom = design::Label::height(ui, design::tokens::LABEL_FONT_SIZE);
    let name_rect = egui::Rect::from_min_size(
        egui::pos2(rect.center().x - largeur_nom / 2.0, slot.bottom() + 5.0),
        Vec2::new(largeur_nom, hauteur_nom),
    );
    cellule.put(
        name_rect,
        design::label(&item.name)
            .width(largeur_nom)
            .color(TILE_NAME_TEXT)
            .log_name("alertes.nom"),
    );

    let ds = design::DesignSystem::get(ui.ctx());
    let glyph = if item.enabled {
        DsIcon::Volume
    } else {
        DsIcon::VolumeMute
    };
    let native = ds.icon_native_size(glyph);
    ds.paint_icon(
        ui.painter(),
        Rect::from_center_size(
            egui::pos2(
                rect.left() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
                rect.top() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
            ),
            design::components::icon_button::glyph_fit(native, TILE_BADGE),
        ),
        glyph,
        // **Pas la couleur de la bordure** : à l'identique, le pictogramme de coupure se fondait
        // dans son propre liseré gris et devenait illisible. La bordure porte seule le signal de
        // couleur.
        SUBDUED,
    );

    // Croix de retrait — absente sur un objet par défaut, que le web refuse de supprimer.
    let mut clic = TileClick::None;
    if !item.is_default {
        let croix = Rect::from_center_size(
            egui::pos2(
                rect.right() - TILE_BADGE_INSET - TILE_BADGE / 2.0,
                rect.top() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
            ),
            Vec2::splat(TILE_BADGE + 4.0),
        );
        let zone = ui.interact(croix, response.id.with("retirer"), egui::Sense::click());
        let native = ds.icon_native_size(DsIcon::Close);
        ds.paint_icon(
            ui.painter(),
            Rect::from_center_size(
                croix.center(),
                design::components::icon_button::glyph_fit(native, TILE_BADGE - 2.0),
            ),
            DsIcon::Close,
            if zone.hovered() { TEXT } else { SUBDUED },
        );
        let zone = zone.on_hover_cursor(egui::CursorIcon::PointingHand);
        zone.clone().on_hover_text("Retirer de vos alertes");
        if zone.clicked() {
            clic = TileClick::Remove;
        }
        // La croix mange le clic : sans ça, retirer basculerait aussi le son au passage.
        if zone.hovered() {
            return clic;
        }
    }

    // **Les deux infobulles s'excluent.** Celle du nom est posée par `design::label` — elle ne
    // paraît que si le nom est coupé, et elle rend le nom entier. Celle de la tuile dit ce que le
    // clic fera. Sans cette exclusion, survoler un nom coupé en déclencherait deux, l'une sur
    // l'autre : la position du curseur tranche, et le nom gagne sur sa propre zone.
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    // Seulement là où le libellé porte DÉJÀ la sienne : sur un nom qui tient en entier, il n'en
    // pose aucune, et se taire ici laisserait une zone muette au milieu de la tuile.
    let sur_un_nom_coupe =
        design::Label::elides(ui, &item.name, largeur_nom, design::tokens::LABEL_FONT_SIZE)
            && ui
                .input(|i| i.pointer.hover_pos())
                .is_some_and(|p| name_rect.contains(p));
    if !sur_un_nom_coupe {
        design::tooltip(&response).text(format!(
            "{} — {}",
            item.name,
            if item.enabled {
                "son activé, cliquer pour couper"
            } else {
                "son coupé, cliquer pour rétablir"
            }
        ));
    }
    if response.clicked() {
        clic = TileClick::Toggle;
    }
    clic
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
    ctx.remote_icon_textures
        .resolve(ui.ctx(), ctx.remote_icons, icon)
        .map(|handle| handle.id())
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
