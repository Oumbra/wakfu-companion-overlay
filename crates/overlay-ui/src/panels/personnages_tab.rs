//! **Onglet « Personnages » de la fenêtre Options** — les comptes du roster, leurs personnages, et
//! les deux modales qui les créent.
//!
//! Porté le 2026-09-16 depuis les maquettes validées ce jour-là
//! (`crates/overlay-testkit/examples/personnages-mockups.rs`, treize planches) — dernier onglet de
//! la fenêtre à recevoir son contenu.
//!
//! ## Ce que cet écran édite, et ce qu'il ne touche pas
//!
//! Il édite un **brouillon** du roster du compte ([`overlay_engine::Roster`]), comme les trois
//! autres onglets de liste : rien n'est écrit tant que « Valider » n'a pas été cliqué, et
//! « Annuler » abandonne tout (voir `panels::options_modal`). La validation réécrit la clé
//! `roster` ENTIÈRE (`overlay_sync::client::patch_roster`) — d'où la forme fidèle du brouillon,
//! `id`/`isDefault` et champs inconnus compris, expliquée dans `overlay_engine::roster`.
//!
//! ## Les règles de l'écran
//!
//! 1. **La tuile « + » ouvre la grille**, toujours première : à quarante personnages, une tuile
//!    d'ajout posée à la fin obligerait à défiler jusqu'en bas pour en créer un de plus.
//! 2. **Une seule modale pour créer et modifier** : modifier, c'est reprendre les trois mêmes
//!    champs (nom, sexe, classe) déjà remplis. Elle s'intitule donc « Personnage », sans
//!    « Nouveau » — un titre qui annoncerait une création mentirait une fois sur deux.
//! 3. **Le sexe survit d'une création à la suivante** (voir
//!    [`PersonnagesTabState::gender_default`]) : qui déclare six mules féminines ne reclique pas
//!    six fois. Il ne survit pas à la fermeture de l'overlay.
//! 4. **Les deux boutons d'une tuile sont LOIN l'un de l'autre** : « Modifier » au centre du buste
//!    (le geste courant, là où le pointeur arrive), « Supprimer » au coin (où l'on ne va que si on
//!    y va exprès). Et seul « Modifier » porte un socle rond — au centre du buste, rien d'autre ne
//!    dirait qu'on peut cliquer ; la croix, elle, garde le glyphe nu des tuiles d'Alertes et de
//!    Chat, l'idiome de l'application.
//! 5. **Le nom se complète depuis le journal** (voir [`JournalCharacter`]) sans jamais s'y
//!    contraindre : l'overlay lit `wakfu.log`, il connaît donc des noms que le site ne connaîtra
//!    jamais. Il les PROPOSE. Un nom qu'aucune suggestion ne porte sort du champ tel quel.
//!
//! ## Deux écarts à la maquette, et pourquoi
//!
//! - **Retirer un personnage ne demande pas confirmation**, contrairement à ce que la note de la
//!   maquette laissait entendre. La fenêtre est transactionnelle : « Annuler » rattrape le geste,
//!   comme aux trois autres onglets, et la suppression multiple de `panels::bulk_select` ne
//!   confirme rien non plus. Retirer un COMPTE, si : il emporte ses personnages, et la question
//!   porte leur décompte.
//! - **Un compte ne se renomme pas depuis ici** : la maquette donne à la ligne de compte deux
//!   boutons (« + » et la corbeille) et pas un troisième. La modale sait le faire — elle sert déjà
//!   aux deux côtés pour les personnages — mais rien ne l'ouvre, et inventer un bouton dans une
//!   maquette validée n'est pas au programme de ce portage.

use egui::{Color32, Pos2, Rect, RichText, Vec2};
use overlay_engine::{Gender, Roster, RosterCharacter};

use crate::avatars::{AvatarAtlas, AVATAR_SIZE};
use crate::design::{self, DsIcon, IconContext, InputSize};
use crate::game_servers::GameServers;
use crate::panels::{bulk_select, tile_reorder};
use crate::ui_icons::UiIcons;

// -------------------------------------------------------------------------------------------
// Jetons — chacun dit d'où il vient. Relevés sur les maquettes, jamais réinventés ici.
// -------------------------------------------------------------------------------------------

const TEXT: Color32 = Color32::WHITE;
/// `panels::alerts_tab::SUBDUED`.
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);
/// `panels::alerts_tab::SECTION_GAP`.
const SECTION_GAP: f32 = 18.0;
/// `panels::alerts_tab::BODY_FONT_SIZE`.
const BODY_FONT_SIZE: f32 = 15.0;
/// Voile posé sur une tuile survolée — `tokens::LEGEND_TILE_HOVER_SCRIM`.
const TILE_HOVER_SCRIM: Color32 = Color32::from_black_alpha(0x66);

/// Aplat du bandeau de nom — `panels::alerts_tab::SETTING_ROW_FILL`, **le même pour tous** : le
/// turquoise « personnage actif » d'un jet intermédiaire a été retiré le 2026-09-16.
const BAND: Color32 = Color32::from_rgb(0x26, 0x28, 0x2B);

/// Cadre d'une tuile — `design::legend_tile` (`tokens::LEGEND_TILE_FILL` / `_BORDER`).
const TILE_FILL: Color32 = Color32::from_rgb(0x0E, 0x11, 0x15);
const TILE_BORDER: Color32 = Color32::from_rgb(0x59, 0x51, 0x40);
const TILE_BORDER_WIDTH: f32 = 2.0;
/// Le même cadre, éclairci, pour la tuile survolée : le voile seul se lit sur une carte d'objet
/// claire, pas sur un buste déjà sombre.
const TILE_BORDER_HOVER: Color32 = Color32::from_rgb(0x9A, 0x8C, 0x6E);
/// **Arrondi des tuiles** — celui du champ de saisie (`tokens::INPUT_RADIUS`), demandé le
/// 2026-09-16 : « un peu plus forcé, le même que la bordure de l'input texte ».
const TILE_RADIUS: u8 = design::tokens::INPUT_RADIUS;

/// La marge autour du buste, qui vaut pour les trois bords.
const TILE_PAD: f32 = 10.0;
/// Largeur d'une tuile : **80 de buste et 10 de marge de chaque côté**.
const TILE_W: f32 = AVATAR_SIZE + 2.0 * TILE_PAD;
/// Hauteur du bandeau de nom, relevée sur les captures du panneau de héros du jeu.
const TILE_BAND: f32 = 22.0;
/// 2 de cadre + 10 + 80 de buste + 2 + 22 de bandeau + 2 de cadre.
const TILE_H: f32 = 118.0;
/// Côté du glyphe d'un badge révélé au survol.
const BADGE: f32 = 14.0;
/// Diamètre du socle rond qui porte le bouton de modification.
const BADGE_DISC: f32 = 26.0;
/// Distance de la croix nue au coin haut-droit — `alerts_tab::TILE_BADGE_INSET`, l'idiome des
/// tuiles d'Alertes et de Chat, que ce bouton-là reprend tel quel.
const CROSS_INSET: f32 = 8.0;
/// Fond du socle — assez opaque pour détacher le glyphe du buste, assez sombre pour rester du jeu.
const BADGE_DISC_FILL: Color32 = Color32::from_black_alpha(0xB4);
/// Gouttière minimale entre deux tuiles — `alerts_tab::TILE_GAP`. La grille en pose davantage
/// quand la largeur restante le permet : les tuiles ont une largeur FIXE, c'est donc l'espacement
/// qui absorbe le reste, jamais la tuile qui s'étire.
const TILE_GAP_MIN: f32 = 10.0;
/// Corps du nom dans le bandeau.
const BAND_FONT_SIZE: f32 = 12.5;

const DESC: &str = "Déclarez les personnages de vos comptes : l'overlay les reconnaît dans le \
                    journal, les distingue des ennemis et rattache vos gains au bon compte.";

/// Les 18 classes dans l'ordre du jeu, avec le nom que porte leur **infobulle**.
///
/// Les clés sont celles du site (`class-icons.data.ts`) et des assets de bustes, les libellés ceux
/// de `CLASS_NAMES.fr` — l'ordre, lui, est celui de l'écran de création de personnage du jeu, pas
/// l'ordre alphabétique des deux tables.
const CLASSES: &[(&str, &str)] = &[
    ("feca", "Féca"),
    ("osamodas", "Osamodas"),
    ("enutrof", "Enutrof"),
    ("sram", "Sram"),
    ("xelor", "Xélor"),
    ("ecaflip", "Ecaflip"),
    ("eniripsa", "Eniripsa"),
    ("iop", "Iop"),
    ("cra", "Crâ"),
    ("sadida", "Sadida"),
    ("sacrier", "Sacrieur"),
    ("pandawa", "Pandawa"),
    ("rogue", "Roublard"),
    ("zobal", "Zobal"),
    ("foggernaut", "Steamer"),
    ("eliotrope", "Éliotrope"),
    ("huppermage", "Huppermage"),
    ("ouginak", "Ouginak"),
];

/// Minuscules **sans accents** — ce sur quoi la recherche de classe compare.
///
/// Écrit à la main plutôt que tiré d'une crate de normalisation Unicode : quatre des dix-huit noms
/// portent un accent (Féca, Xélor, Crâ, Éliotrope), tous latins-1, et une dépendance de plus dans
/// `overlay-ui` pour ça ne se justifie pas. À ne pas confondre avec
/// `overlay_engine::roster::normalize_wakfu_name`, qui compare des NOMS DE PERSONNAGE (et gère les
/// apostrophes typographiques du jeu) : ici on compare une requête à des noms de classe.
fn normalise(texte: &str) -> String {
    texte
        .chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            'ñ' => 'n',
            autre => autre,
        })
        .collect()
}

/// Rang d'une classe dans [`CLASSES`] — `None` pour une classe qu'aucun asset ne porte (roster
/// écrit par une version plus récente du site).
fn class_index(class_name: &str) -> Option<usize> {
    CLASSES.iter().position(|(key, _)| *key == class_name)
}

// -------------------------------------------------------------------------------------------
// État
// -------------------------------------------------------------------------------------------

/// Le brouillon est-il là ? Mêmes cas que `ChatAvailability`, pour la même raison : le roster
/// descend du compte. Pas de variante « sans compte » — l'overlay ne s'utilise pas déconnecté
/// (décision du 2026-09-13), et la maquette a retiré cet état le 2026-09-16.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PersonnagesAvailability {
    #[default]
    Ready,
    Loading,
}

/// **Un personnage déjà vu dans `wakfu.log`**, tel que le champ de nom le propose.
///
/// Dérivé par l'hôte des combats de la session (`SessionSnapshot::fights`, combattants alliés) à
/// l'ouverture de la fenêtre : la classe y est celle que le `breed` de la ligne `[_FL_]` donne.
/// **Le sexe n'est PAS dans le log** — `session::resolve_ally_class` retombe sur `Gender::M` faute
/// de mieux — donc la suggestion pré-remplit un masculin que le switch corrige d'un clic.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalCharacter {
    pub name: String,
    pub class_name: Option<String>,
    pub gender: Gender,
}

/// **Les personnages alliés déjà vus cette session**, prêts à être proposés par le champ de nom —
/// dédupliqués par nom normalisé, triés par nom, et **privés de ceux qui n'ont aucune classe**
/// (`FighterDamage::class_name` vaut `None` pour un allié que rien n'a encore classifié : une
/// suggestion sans classe n'apporterait que son orthographe, et la modale exige une classe).
///
/// Dérivée du snapshot de session plutôt que collectée par le moteur : les combats y sont déjà
/// tous là, avec leurs combattants et la classe que leur `breed` a donnée — un index cumulatif de
/// plus côté moteur, publié à chaque frame, coûterait sans rien apporter.
pub fn journal_from_session(snapshot: &overlay_engine::SessionSnapshot) -> Vec<JournalCharacter> {
    let mut vus: Vec<JournalCharacter> = Vec::new();
    let mut connus: std::collections::HashSet<String> = std::collections::HashSet::new();
    for fight in &snapshot.fights {
        for fighter in fight.fighters.iter().filter(|f| f.is_ally) {
            let Some(class_name) = fighter.class_name.clone() else {
                continue;
            };
            if !connus.insert(overlay_engine::roster::normalize_wakfu_name(&fighter.name)) {
                continue;
            }
            vus.push(JournalCharacter {
                name: fighter.name.clone(),
                class_name: Some(class_name),
                gender: fighter.gender,
            });
        }
    }
    vus.sort_by(|a, b| a.name.cmp(&b.name));
    vus
}

/// La modale de personnage, quand elle est ouverte.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterEditor {
    /// Rang édité dans le compte affiché — `None` pour une création.
    pub index: Option<usize>,
    pub name: String,
    /// La recherche de classe, à droite du switch.
    pub search: String,
    pub gender: Gender,
    /// Rang dans [`CLASSES`].
    pub class: Option<usize>,
}

/// La modale de compte, quand elle est ouverte. Un compte du roster, c'est un libellé et un
/// serveur de jeu : le site n'en demande pas plus (`CharacterRosterService.addAccount`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccountEditor {
    pub name: String,
    pub server: Option<String>,
}

/// L'état de l'onglet — vit dans `OptionsModalState`, donc survit au changement d'onglet mais pas
/// à la fermeture de la fenêtre.
#[derive(Debug, Clone, PartialEq)]
pub struct PersonnagesTabState {
    /// Le compte affiché, rang dans `Roster::accounts`.
    pub account: usize,
    /// Mode « suppression multiple » (voir `panels::bulk_select`).
    pub select_mode: bool,
    /// Les noms cochés dans ce mode.
    pub selected: Vec<String>,
    pub editor: Option<CharacterEditor>,
    pub account_editor: Option<AccountEditor>,
    /// Le compte dont la suppression attend confirmation.
    pub pending_account_removal: Option<usize>,
    /// **Le sexe que la prochaine création proposera** — voir règle 3 de la doc de module.
    pub gender_default: Gender,
    /// Les noms vus au journal, posés par l'hôte à l'ouverture de la fenêtre.
    pub journal: Vec<JournalCharacter>,
}

impl Default for PersonnagesTabState {
    fn default() -> Self {
        Self {
            account: 0,
            select_mode: false,
            selected: Vec::new(),
            editor: None,
            account_editor: None,
            pending_account_removal: None,
            // ♂ par défaut, décision du 2026-09-16.
            gender_default: Gender::M,
            journal: Vec::new(),
        }
    }
}

impl PersonnagesTabState {
    /// Quitte les modes et les modales — appelé quand le compte affiché change : les coches d'un
    /// compte n'ont aucun sens sur un autre, et une modale ouverte y écrirait.
    fn reset_gestes(&mut self) {
        self.select_mode = false;
        self.selected.clear();
        self.editor = None;
        self.account_editor = None;
        self.pending_account_removal = None;
    }
}

/// Ce que l'onglet reçoit de la fenêtre pour peindre.
pub struct PersonnagesTabContext<'a> {
    /// Le brouillon du roster — modifié en place, renvoyé au compte par « Valider ».
    pub roster: &'a mut Roster,
    /// Les serveurs de jeu (`crate::game_servers`) — vide tant que la liste n'est pas descendue,
    /// ce qui n'empêche ni d'afficher ni de garder celui que le compte porte déjà.
    pub servers: &'a GameServers,
    pub avatars: &'a AvatarAtlas,
    /// Repli quand la classe d'un personnage n'a pas de buste (classe inconnue de cette version).
    pub icons: &'a UiIcons,
    pub availability: PersonnagesAvailability,
    /// La fenêtre entière — ce sur quoi une modale se pose.
    pub window: Rect,
}

// -------------------------------------------------------------------------------------------
// L'écran
// -------------------------------------------------------------------------------------------

pub fn show(
    ui: &mut egui::Ui,
    panel: &design::PanelZones,
    state: &mut PersonnagesTabState,
    ctx: &mut PersonnagesTabContext<'_>,
) {
    let width = panel.inner.width();

    ui.add(design::heading("Personnages"));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP);

    if ctx.availability == PersonnagesAvailability::Loading {
        loading_row(ui, panel.inner);
        return;
    }

    // Un compte qui a disparu (roster relu pendant que la fenêtre était ouverte) ramène l'écran au
    // principal plutôt que de peindre du vide.
    if state.account >= ctx.roster.accounts.len() {
        state.account = ctx.roster.default_index();
        state.reset_gestes();
    }

    account_row(ui, state, ctx, width);
    ui.add_space(SECTION_GAP * 0.75);

    // Le titre de la liste et ses commandes de suppression multiple — le même en-tête qu'au Suivi,
    // aux Alertes et au Chat, avec la règle qui va avec : aucun bouton tant qu'il n'y a rien à
    // supprimer.
    let compte_len = ctx.roster.accounts[state.account].characters.len();
    let demande = bulk_select::show(
        ui,
        width,
        bulk_select::BulkHeader {
            title: "Personnages du compte",
            removable: compte_len,
            bulk_tooltip: "Retire les personnages cochés de ce compte — annulable tant que la \
                           fenêtre n'est pas validée.",
            log_prefix: "personnages",
            enabled: true,
        },
        bulk_select::BulkSelection {
            mode: &mut state.select_mode,
            keys: &mut state.selected,
        },
    );
    match demande {
        bulk_select::BulkRequest::None => {}
        bulk_select::BulkRequest::All => {
            let retires = compte_len;
            ctx.roster.accounts[state.account].characters.clear();
            tracing::info!(retires, "[options] personnages retirés en bloc");
        }
        bulk_select::BulkRequest::Keys(cles) => {
            let avant = compte_len;
            ctx.roster.accounts[state.account]
                .characters
                .retain(|c| !cles.contains(&c.name));
            let retires = avant - ctx.roster.accounts[state.account].characters.len();
            tracing::info!(retires, "[options] personnages retirés en bloc");
        }
    }

    // **Avant la grille, jamais après** : la zone défilable prend toute la hauteur restante, et un
    // bloc ajouté à sa suite se peindrait PAR-DESSUS la tuile « + » (relevé sur maquette le
    // 2026-09-16 — ce n'était pas un artefact de planche, c'était le rendu réel).
    if ctx.roster.accounts[state.account].characters.is_empty() {
        ui.add_space(bulk_select::HEADER_TO_PARAGRAPH);
        ui.add(
            design::info_text(
                "Aucun personnage sur ce compte. Cliquez sur la tuile « + » : le nom demandé est \
                 celui qui apparaît en jeu, au caractère près — c'est ce que l'overlay cherche \
                 dans le journal.",
            )
            .width(width)
            .log_name("personnages.vide"),
        );
        ui.add_space(SECTION_GAP * 0.75);
    } else {
        ui.add_space(bulk_select::HEADER_GAP);
    }

    grille(ui, panel, state, ctx);

    // Les modales passent APRÈS la grille : elles se posent sur la fenêtre entière, dans leur
    // propre couche (voir `couche_modale`).
    if let Some(index) = state.pending_account_removal {
        confirmation_de_compte(ui, state, ctx, index);
    } else if state.account_editor.is_some() {
        modale_compte(ui, state, ctx);
    } else if state.editor.is_some() {
        modale_personnage(ui, state, ctx);
    }
}

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

/// Le rouage d'attente, centré dans ce qui reste du panneau — `chat_tab::loading_row`.
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

/// Le libellé d'un compte — miroir de `profile-page.component.ts::tabLabel` : « Principal » pour le
/// compte par défaut (que le site ne laisse pas renommer), le libellé choisi sinon, un nom
/// générique tant qu'il n'a pas été renseigné.
fn account_label(roster: &Roster, index: usize) -> String {
    let Some(compte) = roster.accounts.get(index) else {
        return String::new();
    };
    if compte.is_default {
        return "Principal".to_string();
    }
    let label = compte.label.trim();
    if label.is_empty() {
        format!("Compte {}", index + 1)
    } else {
        label.to_string()
    }
}

/// La ligne « compte + serveur ». Les deux boutons de droite portent sur le COMPTE, jamais sur un
/// personnage : « + » ouvre la modale de création de compte, la corbeille retire celui qui est
/// choisi — éteinte sur le compte principal, que le site interdit de supprimer (`isDefault`).
fn account_row(
    ui: &mut egui::Ui,
    state: &mut PersonnagesTabState,
    ctx: &mut PersonnagesTabContext<'_>,
    width: f32,
) {
    // La ligne n'est pas dans la zone défilable : elle reprend la réserve de barre, comme le
    // formulaire d'ajout de `chat_tab`.
    let total =
        width + design::components::scroll_area::RESERVE_X - design::tokens::PANEL_PAD_CONTROL_X;
    let row = ui
        .allocate_space(Vec2::new(total, design::tokens::SELECT_HEIGHT))
        .1;
    let principal = ctx.roster.accounts[state.account].is_default;
    let libelles: Vec<String> = (0..ctx.roster.accounts.len())
        .map(|i| account_label(ctx.roster, i))
        .collect();
    let serveur_courant = ctx.roster.accounts[state.account].game_server.clone();
    let mut compte_choisi = state.account;
    let mut serveur_choisi = serveur_courant.clone();

    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    cell.spacing_mut().item_spacing.x = 0.0;
    let mut ajouter = false;
    let mut retirer = false;
    cell.horizontal_centered(|ui| {
        ui.label(
            RichText::new("Compte")
                .color(SUBDUED)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        );
        ui.add_space(10.0);
        let mut select = design::select(&mut compte_choisi)
            .width(210.0)
            .log_name("personnages.compte");
        for (index, libelle) in libelles.iter().enumerate() {
            select = select.option(index, libelle.clone());
        }
        select.show(ui);
        ui.add_space(20.0);
        ui.label(
            RichText::new("Serveur")
                .color(SUBDUED)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        );
        ui.add_space(10.0);
        let mut select = design::select(&mut serveur_choisi)
            .width(170.0)
            .log_name("personnages.serveur");
        for serveur in ctx.servers.selectable(serveur_courant.as_deref()) {
            select = select.option(Some(serveur.code.clone()), serveur.label.clone());
        }
        // **« Aucun » en dernier, et jamais absent** : un compte peut légitimement n'avoir aucun
        // serveur déclaré (voir `RosterAccount::game_server`), et c'est même l'état de départ.
        select = select.option(None, "Aucun");
        select.show(ui);

        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        retirer = right
            .add(
                design::icon_button(DsIcon::Delete)
                    .context(IconContext::Panel)
                    .enabled(!principal)
                    .tooltip(if principal {
                        "Le compte principal ne se supprime pas"
                    } else {
                        "Supprimer ce compte"
                    })
                    .log_name("personnages.compte.retirer"),
            )
            .clicked();
        right.add_space(design::tokens::PANEL_PAD_CONTROL_X);
        ajouter = right
            .add(
                design::icon_button(DsIcon::Plus)
                    .context(IconContext::Panel)
                    .tooltip("Ajouter un compte")
                    .log_name("personnages.compte.ajouter"),
            )
            .clicked();
    });

    if compte_choisi != state.account {
        state.account = compte_choisi;
        state.reset_gestes();
        tracing::info!(
            compte = %account_label(ctx.roster, compte_choisi),
            "[options] compte affiché changé"
        );
        return;
    }
    if serveur_choisi != serveur_courant {
        tracing::info!(
            serveur = serveur_choisi.as_deref().unwrap_or("aucun"),
            "[options] serveur de jeu du compte changé"
        );
        ctx.roster.accounts[state.account].game_server = serveur_choisi;
    }
    if ajouter {
        state.account_editor = Some(AccountEditor::default());
    }
    if retirer && !principal {
        state.pending_account_removal = Some(state.account);
    }
}

// -------------------------------------------------------------------------------------------
// La grille
// -------------------------------------------------------------------------------------------

/// Ce qu'un geste sur une tuile demande — appliqué après la boucle, jamais pendant : la grille lit
/// la liste que ces gestes modifient.
enum TileAction {
    Edit(usize),
    Remove(usize),
    Check(String),
}

fn grille(
    ui: &mut egui::Ui,
    panel: &design::PanelZones,
    state: &mut PersonnagesTabState,
    ctx: &mut PersonnagesTabContext<'_>,
) {
    let personnages: Vec<RosterCharacter> = ctx.roster.accounts[state.account].characters.clone();
    let select_mode = state.select_mode;
    let cochees: std::collections::HashSet<&String> = state.selected.iter().collect();
    let mut action: Option<TileAction> = None;
    let mut creer = false;
    let mut deplacement: Option<(usize, usize)> = None;

    panel.scroll_area(ui, "personnages.grille", |ui, content_width| {
        // Le plus grand nombre de tuiles qui tienne AVEC la gouttière minimale entre elles — et non
        // une division qui compterait une gouttière de trop au bout de la rangée.
        let cols = (1..=8)
            .take_while(|n| *n as f32 * TILE_W + (*n as f32 - 1.0) * TILE_GAP_MIN <= content_width)
            .last()
            .unwrap_or(1);
        let gap = if cols > 1 {
            (content_width - cols as f32 * TILE_W) / (cols - 1) as f32
        } else {
            0.0
        };
        ui.spacing_mut().item_spacing = Vec2::new(gap, TILE_GAP_MIN);
        let size = Vec2::new(TILE_W, TILE_H);
        // **La tuile « + » disparaît pendant une suppression multiple** : le mode est exclusif, on
        // retire ou on ajoute, jamais les deux dans le même geste.
        let ajout = !select_mode;
        let mut rang = 0usize;
        let mut index = 0usize;
        while index < personnages.len() || (rang == 0 && ajout) {
            ui.horizontal(|ui| {
                let mut colonne = 0usize;
                if rang == 0 && ajout {
                    creer |= add_tile(ui, size);
                    colonne += 1;
                }
                while colonne < cols && index < personnages.len() {
                    let perso = &personnages[index];
                    let issue = hero_tile(
                        ui,
                        perso,
                        index,
                        size,
                        select_mode.then(|| cochees.contains(&perso.name)),
                        ctx.avatars,
                        ctx.icons,
                    );
                    if let Some(geste) = issue.action {
                        action = Some(geste);
                    }
                    if let Some(depuis) = issue.dropped {
                        deplacement = Some((depuis, index));
                    }
                    index += 1;
                    colonne += 1;
                }
            });
            rang += 1;
        }
    });

    if creer {
        state.editor = Some(CharacterEditor {
            index: None,
            name: String::new(),
            search: String::new(),
            gender: state.gender_default,
            class: None,
        });
    }
    if let Some((depuis, vers)) = deplacement {
        ctx.roster.reorder(state.account, depuis, vers);
        tracing::info!(depuis, vers, "[options] personnage déplacé");
    }
    match action {
        Some(TileAction::Edit(index)) => {
            if let Some(perso) = personnages.get(index) {
                state.editor = Some(CharacterEditor {
                    index: Some(index),
                    name: perso.name.clone(),
                    search: String::new(),
                    gender: perso.gender,
                    class: class_index(&perso.class_name),
                });
            }
        }
        Some(TileAction::Remove(index)) => {
            if let Some(perso) = personnages.get(index) {
                // Une clé cochée qui ne désigne plus rien ferait mentir le compteur du bouton
                // groupé — même précaution qu'au Chat.
                state.selected.retain(|k| k != &perso.name);
                tracing::info!(nom = %perso.name, "[options] personnage retiré du compte");
            }
            let compte = &mut ctx.roster.accounts[state.account];
            if index < compte.characters.len() {
                compte.characters.remove(index);
            }
        }
        Some(TileAction::Check(nom)) => bulk_select::toggle(&mut state.selected, &nom),
        None => {}
    }
}

/// Ce qu'une tuile rend à la grille.
#[derive(Default)]
struct TileOutcome {
    action: Option<TileAction>,
    /// Rang de la tuile lâchée sur celle-ci, à la frame du dépôt.
    dropped: Option<usize>,
}

/// **La tuile « + », toujours la première de la grille.** Elle ouvre la modale de personnage.
///
/// **Ni bandeau ni libellé** : elle n'a pas de nom à porter, et un bandeau vide en bas de cadre lui
/// donnait l'air d'une tuile de personnage à qui il manquerait quelque chose. Le « + » est donc
/// centré dans le cadre entier, sur les deux axes.
fn add_tile(ui: &mut egui::Ui, size: Vec2) -> bool {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let painter = ui.painter().clone();
    let survol = response.contains_pointer();
    let teinte = if survol {
        design::tokens::TEXT_GOLD
    } else {
        design::tokens::ICON_TINT
    };
    painter.rect_filled(rect, TILE_RADIUS, TILE_FILL);
    painter.rect_stroke(
        rect,
        TILE_RADIUS,
        egui::Stroke::new(
            TILE_BORDER_WIDTH,
            if survol {
                design::tokens::TEXT_GOLD
            } else {
                TILE_BORDER
            },
        ),
        egui::StrokeKind::Inside,
    );
    paint_glyph(ui, rect.center(), DsIcon::Plus, 30.0, teinte);
    design::tooltip(&response).text("Ajouter un personnage à ce compte");
    if response.clicked() {
        tracing::info!("[options] modale de personnage ouverte (création)");
    }
    response.clicked()
}

/// Une carte de héros : buste détouré posé sur le fond de la tuile, **bandeau de nom seul** collé en
/// bas. Pas de nom de classe : le portrait la dit déjà, et dans le jeu on la connaît.
fn hero_tile(
    ui: &mut egui::Ui,
    perso: &RosterCharacter,
    index: usize,
    size: Vec2,
    cochee: Option<bool>,
    avatars: &AvatarAtlas,
    icons: &UiIcons,
) -> TileOutcome {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let painter = ui.painter().clone();
    let mut outcome = TileOutcome::default();

    // **La carte d'abord, le geste ensuite.** `tile_reorder` peint le voile de la place d'origine
    // et le liseré de la destination PAR-DESSUS la tuile qu'on vient de peindre : l'appeler avant
    // reviendrait à recouvrir les deux du fond de la carte, et un déplacement ne se verrait pas.
    painter.rect_filled(rect, TILE_RADIUS, TILE_FILL);
    let survol_pointeur = response.contains_pointer();
    // **Un seul porteur de l'or à la fois** : la tuile cochée, ou celle qu'on survole. La
    // destination d'un déplacement, elle, reçoit son liseré de `tile_reorder` (voir plus bas).
    let bordure = match (cochee, survol_pointeur) {
        (Some(true), _) => design::tokens::ITEM_SLOT_SELECTED_BORDER,
        (_, true) => TILE_BORDER_HOVER,
        _ => TILE_BORDER,
    };
    painter.rect_stroke(
        rect,
        TILE_RADIUS,
        egui::Stroke::new(TILE_BORDER_WIDTH, bordure),
        egui::StrokeKind::Inside,
    );
    paint_avatar(ui, avatar_rect(rect), perso, false, avatars, icons);

    let band = band_rect(rect);
    painter.rect_filled(band, 0.0, BAND);
    let font = design::text::label_strong_font(ui.ctx(), BAND_FONT_SIZE);
    let dispo = band.width() - 10.0;
    // **L'ellipse déclenche l'infobulle, et elle seule** : un nom qui tient en entier n'a rien à
    // redire, et une infobulle qui répète le libellé visible est du bruit. Même règle que le
    // `tooltipOnlyIfTruncated` du site.
    let coupe = text_width(ui, &perso.name, font.clone()) > dispo;
    painted_ellipsed(ui, &painter, &perso.name, band.center(), dispo, font, TEXT);

    let geste = if cochee.is_some() {
        // Pas de déplacement en mode sélection : le geste de la tuile est alors de cocher.
        tile_reorder::Gesture::default()
    } else {
        tile_reorder::handle_with(
            ui,
            &response,
            index,
            TILE_RADIUS,
            |ui, ghost| {
                let (place, _) = ui.allocate_exact_size(ghost.size(), egui::Sense::hover());
                paint_ghost(ui, place, perso, avatars, icons);
            },
            |ui, cible| {
                ui.painter().rect_stroke(
                    cible,
                    TILE_RADIUS,
                    egui::Stroke::new(TILE_BORDER_WIDTH, design::tokens::ITEM_SLOT_SELECTED_BORDER),
                    egui::StrokeKind::Inside,
                );
            },
        )
    };
    outcome.dropped = geste.dropped;
    // **Pendant un déplacement, plus aucun badge** : la tuile sous le pointeur est une destination,
    // pas une cible de clic (même règle qu'au Suivi).
    let survol = survol_pointeur && !geste.in_flight();

    if let Some(cochee) = cochee {
        // Coin haut-droit, comme `design::legend_tile` : la case remplace les badges de survol, les
        // deux ne s'affichent jamais ensemble.
        design::paint_checkbox(
            ui,
            Rect::from_min_size(
                egui::pos2(
                    rect.right() - TILE_BORDER_WIDTH - 4.0 - design::tokens::CHECKBOX_SIZE,
                    rect.top() + TILE_BORDER_WIDTH + 4.0,
                ),
                Vec2::splat(design::tokens::CHECKBOX_SIZE),
            ),
            cochee,
            Color32::WHITE,
        );
        if response.clicked() {
            outcome.action = Some(TileAction::Check(perso.name.clone()));
        }
        return outcome;
    }

    if !survol {
        if coupe {
            design::tooltip(&response).anchor(rect).text(&perso.name);
        }
        return outcome;
    }

    // Le voile s'arrête au bord haut du bandeau : le couvrir rendrait le nom gris sur gris à
    // l'instant où le pointeur l'atteint, c'est-à-dire exactement quand on le lit.
    painter.rect_filled(
        Rect::from_min_max(rect.min, egui::pos2(rect.right(), band.top()))
            .shrink(TILE_BORDER_WIDTH),
        0.0,
        TILE_HOVER_SCRIM,
    );
    let modifier = tile_button(
        ui,
        rect,
        egui::pos2(rect.center().x, band.top() - AVATAR_SIZE / 2.0 - 4.0),
        DsIcon::Edit,
        "Modifier",
        true,
        egui::Id::new(("personnages.modifier", index)),
    );
    let retirer = tile_button(
        ui,
        rect,
        egui::pos2(
            rect.right() - CROSS_INSET - BADGE / 2.0,
            rect.top() + CROSS_INSET + BADGE / 2.0,
        ),
        DsIcon::Close,
        "Supprimer",
        false,
        egui::Id::new(("personnages.retirer", index)),
    );
    // **Le nom complet ne s'ouvre que si aucun bouton n'est visé** : deux infobulles à la fois se
    // recouvriraient.
    if coupe && !modifier.contains_pointer() && !retirer.contains_pointer() {
        design::tooltip(&response).anchor(rect).text(&perso.name);
    }
    if retirer.clicked() {
        outcome.action = Some(TileAction::Remove(index));
    } else if modifier.clicked() || response.clicked() {
        // La tuile entière ouvre la modale : viser le petit socle n'est pas une condition, c'est
        // une aide.
        outcome.action = Some(TileAction::Edit(index));
    }
    outcome
}

/// **Un bouton de tuile** : un glyphe, une zone cliquable, une infobulle — et, quand `socle` le
/// demande, le disque qui dit « ceci est un bouton ».
///
/// **Le socle n'est PAS pour les deux** (décision du 2026-09-16) : il porte le bouton de
/// modification, posé au centre du buste, où rien d'autre ne signalerait qu'on peut cliquer. La
/// croix de retrait, elle, garde le glyphe nu des tuiles d'Alertes et de Chat — c'est l'idiome de
/// l'application, et l'entourer d'un disque l'aurait mise en avant alors qu'elle est le geste qu'on
/// ne veut PAS faire par mégarde.
fn tile_button(
    ui: &mut egui::Ui,
    tuile: Rect,
    center: Pos2,
    icon: DsIcon,
    tooltip: &str,
    socle: bool,
    id: egui::Id,
) -> egui::Response {
    let disc = Rect::from_center_size(center, Vec2::splat(BADGE_DISC));
    // Main sous le pointeur, comme la croix des tuiles d'Alertes et de Chat : la tuile en dessous
    // n'en affiche pas, c'est donc ici que le geste se signale.
    let response = ui
        .interact(disc, id, egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let survol = response.contains_pointer();
    if socle {
        let painter = ui.painter();
        painter.circle_filled(center, BADGE_DISC / 2.0, BADGE_DISC_FILL);
        painter.circle_stroke(
            center,
            BADGE_DISC / 2.0,
            egui::Stroke::new(
                1.0,
                if survol {
                    design::tokens::TEXT_GOLD
                } else {
                    TILE_BORDER_HOVER
                },
            ),
        );
    }
    paint_glyph(
        ui,
        center,
        icon,
        BADGE,
        if survol {
            design::tokens::TEXT_GOLD
        } else {
            design::tokens::ICON_TINT
        },
    );
    // **Ancrée sur la tuile, pas sur le bouton** : ancrée sur lui, l'infobulle de « Modifier »
    // s'ouvrait juste au-dessus de son socle, c'est-à-dire par-dessus le bouton « Supprimer ».
    design::tooltip(&response).anchor(tuile).text(tooltip);
    response
}

fn band_rect(tile: Rect) -> Rect {
    Rect::from_min_max(
        egui::pos2(
            tile.left() + TILE_BORDER_WIDTH,
            tile.bottom() - TILE_BORDER_WIDTH - TILE_BAND,
        ),
        egui::pos2(
            tile.right() - TILE_BORDER_WIDTH,
            tile.bottom() - TILE_BORDER_WIDTH,
        ),
    )
}

fn avatar_rect(tile: Rect) -> Rect {
    Rect::from_min_size(
        egui::pos2(tile.center().x - AVATAR_SIZE / 2.0, tile.top() + TILE_PAD),
        Vec2::splat(AVATAR_SIZE),
    )
}

/// Le fantôme d'un déplacement — la même carte, repeinte dans la couche qui suit le pointeur.
fn paint_ghost(
    ui: &egui::Ui,
    rect: Rect,
    perso: &RosterCharacter,
    avatars: &AvatarAtlas,
    icons: &UiIcons,
) {
    let painter = ui.painter();
    painter.rect_filled(rect, TILE_RADIUS, TILE_FILL);
    painter.rect_stroke(
        rect,
        TILE_RADIUS,
        egui::Stroke::new(TILE_BORDER_WIDTH, TILE_BORDER),
        egui::StrokeKind::Inside,
    );
    paint_avatar(ui, avatar_rect(rect), perso, false, avatars, icons);
    let band = band_rect(rect);
    painter.rect_filled(band, 0.0, BAND);
    painted_ellipsed(
        ui,
        painter,
        &perso.name,
        band.center(),
        band.width() - 10.0,
        design::text::label_strong_font(ui.ctx(), BAND_FONT_SIZE),
        TEXT,
    );
}

/// Le buste d'un personnage, peint à sa taille NATIVE dans `rect`. Repli sur le portrait générique
/// d'`UiIcons` quand la classe est inconnue — jamais un trou.
fn paint_avatar(
    ui: &egui::Ui,
    rect: Rect,
    perso: &RosterCharacter,
    gris: bool,
    avatars: &AvatarAtlas,
    icons: &UiIcons,
) {
    paint_class_avatar(
        ui,
        rect,
        &perso.class_name,
        perso.gender,
        gris,
        avatars,
        icons,
    );
}

fn paint_class_avatar(
    ui: &egui::Ui,
    rect: Rect,
    class_name: &str,
    gender: Gender,
    gris: bool,
    avatars: &AvatarAtlas,
    icons: &UiIcons,
) {
    let texture = avatars
        .texture(class_name, gender, gris)
        .map(|handle| handle.id())
        .unwrap_or_else(|| icons.unknown_entity_texture().id());
    ui.painter().image(
        texture,
        rect,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// Un glyphe peint à l'échelle demandée, sans socle — le « + » de la tuile d'ajout.
fn paint_glyph(ui: &egui::Ui, center: Pos2, icon: DsIcon, side: f32, tint: Color32) {
    let ds = design::DesignSystem::get(ui.ctx());
    let native = ds.icon_native_size(icon);
    let fit = native.x.max(native.y);
    ds.paint_icon(
        ui.painter(),
        Rect::from_center_size(center, native * (side / fit)),
        icon,
        tint,
    );
}

/// Largeur qu'occuperait `text` sans contrainte — ce qui permet de savoir si l'ellipse a mordu.
fn text_width(ui: &egui::Ui, text: &str, font: egui::FontId) -> f32 {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::text::TextFormat {
            font_id: font,
            color: TEXT,
            ..Default::default()
        },
    );
    ui.fonts_mut(|f| f.layout_job(job)).size().x
}

/// Texte d'une ligne, centré et ellipsé s'il déborde — jamais rogné en silence.
fn painted_ellipsed(
    ui: &egui::Ui,
    painter: &egui::Painter,
    text: &str,
    center: Pos2,
    max_width: f32,
    font: egui::FontId,
    color: Color32,
) {
    let mut job = egui::text::LayoutJob {
        wrap: egui::text::TextWrapping {
            max_width,
            max_rows: 1,
            break_anywhere: true,
            overflow_character: Some('…'),
        },
        ..Default::default()
    };
    job.append(
        text,
        0.0,
        egui::text::TextFormat {
            font_id: font,
            color,
            ..Default::default()
        },
    );
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let size = galley.size();
    painter.galley(
        egui::pos2(center.x - size.x / 2.0, center.y - size.y / 2.0),
        galley,
        color,
    );
}

// -------------------------------------------------------------------------------------------
// Les modales
// -------------------------------------------------------------------------------------------

/// Ouvre une couche AU-DESSUS du panneau et y peint le voile — l'idiome de
/// `design::confirm_dialog`. Sans elle, le clip du panneau rognerait la bannière de la modale.
///
/// **`Order::Middle` et non `Foreground`**, contrairement à la boîte de confirmation : le panneau
/// de `design::autocomplete` est une `egui::Area` en `Foreground`, et deux couches de MÊME ordre se
/// départagent par leur identifiant — la modale passerait donc par-dessus le panneau de
/// suggestions, qui disparaîtrait purement et simplement. `Middle` couvre tout le contenu de la
/// fenêtre (qui vit dans la couche de base) et laisse le popup au-dessus, là où il doit être.
fn couche_modale(ui: &mut egui::Ui, window: Rect, nom: &'static str) -> egui::Ui {
    let mut couche = ui.new_child(egui::UiBuilder::new().max_rect(window).layer_id(
        egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new(("personnages.modale", nom)),
        ),
    ));
    couche.set_clip_rect(Rect::EVERYTHING);
    couche.painter().rect_filled(
        window,
        0.0,
        Color32::from_black_alpha(design::tokens::CONFIRM_SCRIM_ALPHA),
    );
    // Le voile **avale** les clics qui passent à côté de la modale : sans ça il ne serait qu'une
    // teinte, et ce qu'il couvre resterait réellement cliquable — l'inverse de ce qu'il annonce.
    couche.interact(
        window,
        egui::Id::new(("personnages.modale.voile", nom)),
        egui::Sense::click(),
    );
    couche
}

/// Taille de la modale de personnage. **640 de large et non 600** : la largeur est commandée par la
/// grille de classes (voir [`COLONNE`]), et il faut qu'elle tienne avec de la marge des deux côtés.
const PERSO_MODALE: Vec2 = Vec2::new(640.0, 620.0);
const CLASSE_COLS: usize = 6;
const CLASSE_GAP: f32 = 10.0;

/// **Largeur utile de la modale — celle de la grille de classes, et elle commande tout le reste.**
///
/// Décision du 2026-09-16, après un rendu où le champ de saisie s'arrêtait 16 points avant la
/// dernière colonne de portraits : les marges doivent être les MÊMES pour les champs, le switch et
/// les portraits. Comme une tuile de classe a une largeur fixe (le buste natif) et que leur nombre
/// est fixe, c'est la grille qui donne la mesure — et non l'inverse. Tout se pose donc dans une
/// colonne de cette largeur, **centrée** dans le panneau, quitte à ne pas coller aux marges
/// standard de section.
const COLONNE: f32 = CLASSE_COLS as f32 * AVATAR_SIZE + (CLASSE_COLS as f32 - 1.0) * CLASSE_GAP;

/// **La modale de personnage — création ET modification** (voir règle 2 de la doc de module).
fn modale_personnage(
    ui: &mut egui::Ui,
    state: &mut PersonnagesTabState,
    ctx: &mut PersonnagesTabContext<'_>,
) {
    let Some(mut editeur) = state.editor.clone() else {
        return;
    };
    let mut couche = couche_modale(ui, ctx.window, "personnage");
    let rect = Rect::from_center_size(ctx.window.center(), PERSO_MODALE);
    let mut modale = couche.new_child(egui::UiBuilder::new().max_rect(rect));
    modale.set_clip_rect(Rect::EVERYTHING);
    let chrome = design::window("Personnage")
        .footer("Annuler", "Valider")
        .close_button(true)
        .log_name("personnages.modale")
        .show(&mut modale);

    design::panel().show(&mut modale, chrome.content, |ui, _panel| {
        // La colonne centrée : tout ce qui suit s'y pose, donc tout est aligné sur la grille.
        let zone = ui.max_rect();
        let colonne = Rect::from_min_max(
            egui::pos2(zone.center().x - COLONNE / 2.0, zone.top()),
            egui::pos2(zone.center().x + COLONNE / 2.0, zone.bottom()),
        );
        let mut ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(colonne)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let ui = &mut ui;

        // 1. Le nom, en tête : c'est ce qu'on vient écrire.
        //
        // **Un `design::autocomplete` et non un champ nu** : voir règle 5 de la doc de module. Les
        // noms déjà déclarés sur CE compte sont proposés désactivés — les redéclarer ne ferait
        // qu'écraser ce qui existe, et la modification se fait au crayon de la tuile.
        let deja: std::collections::HashSet<String> = ctx.roster.accounts[state.account]
            .characters
            .iter()
            .filter(|c| Some(c.name.as_str()) != editeur_nom_initial(&editeur, state, ctx))
            .map(|c| overlay_engine::roster::normalize_wakfu_name(&c.name))
            .collect();
        let entrees: Vec<design::AutocompleteEntry> = state
            .journal
            .iter()
            .map(|vu| {
                let mut entree = design::AutocompleteEntry::new(vu.name.clone(), 0);
                entree.image = vu.class_name.as_deref().and_then(|class| {
                    ctx.avatars
                        .texture(class, vu.gender, false)
                        .map(|handle| handle.id())
                });
                if deja.contains(&overlay_engine::roster::normalize_wakfu_name(&vu.name)) {
                    entree.disabled = true;
                    entree.mention = Some("déjà déclaré".to_string());
                }
                entree
            })
            .collect();
        let issue = design::autocomplete(&mut editeur.name)
            .placeholder("Nom du personnage, exactement comme en jeu…")
            .entries(&entrees)
            .width(COLONNE)
            // **Ni loupe, ni champ vidé** (2026-09-16) : ce n'est pas une recherche, c'est le nom
            // du personnage. Il s'écrit librement — le composant ne valide rien, un nom qu'aucune
            // suggestion ne porte sort du champ tel quel — et choisir une suggestion le REMPLIT au
            // lieu de l'effacer.
            .search_icon(false)
            .fill_on_select(true)
            .log_name("personnages.modale.nom")
            .show(ui);
        // Une suggestion apporte AUSSI sa classe et son sexe : c'est tout l'intérêt de connaître le
        // journal. Le sexe qu'elle porte est un masculin par défaut (voir `JournalCharacter`), donc
        // il ne remplace jamais un choix déjà fait — la classe, elle, est sûre.
        if let Some(index) = issue.selected {
            if let Some(vu) = state.journal.get(index) {
                if let Some(rang) = vu.class_name.as_deref().and_then(class_index) {
                    editeur.class = Some(rang);
                }
            }
        }
        ui.add_space(12.0);

        // 2. Sexe à gauche, recherche de classe à droite — une seule ligne, les deux filtres de la
        //    grille qui suit.
        let row = ui
            .allocate_space(Vec2::new(COLONNE, design::tokens::SWITCH_HEIGHT))
            .1;
        let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
        cell.spacing_mut().item_spacing.x = 0.0;
        cell.horizontal_centered(|ui| {
            // **Le premier slot est le masculin, et c'est le défaut** — voir règle 3.
            ui.add(
                design::switch(&mut editeur.gender)
                    .slot(Gender::M, "Masculin")
                    .icon(DsIcon::Male)
                    .slot(Gender::F, "Féminin")
                    .icon(DsIcon::Female)
                    .log_name("personnages.modale.sexe"),
            );
            let mut droite = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(row)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            droite.add(
                design::input(&mut editeur.search)
                    .size(InputSize::Search)
                    .leading_icon(DsIcon::Search)
                    .placeholder("Rechercher une classe…")
                    .clearable(true)
                    .width(270.0)
                    .log_name("personnages.modale.recherche"),
            );
        });
        ui.add_space(16.0);

        // 3. Les classes. **Le filtre ne mord qu'à trois caractères** — le seuil de
        //    `tokens::AUTOCOMPLETE_MIN_QUERY_LEN`, déjà celui du champ ci-dessus — et compare sur
        //    une forme sans casse ni accents (voir [`normalise`]), sans quoi « cra » ne trouverait
        //    pas « Crâ » et « eli » manquerait « Éliotrope ».
        let visibles = classes_visibles(&editeur.search);
        ui.spacing_mut().item_spacing = Vec2::splat(CLASSE_GAP);
        for chunk in visibles.chunks(CLASSE_COLS) {
            ui.horizontal(|ui| {
                for (index, (class, label)) in chunk {
                    // **La tuile fait exactement la taille du buste** : un liseré fin l'entoure,
                    // quitte à rogner un pixel de l'image aux coins.
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::splat(AVATAR_SIZE), egui::Sense::click());
                    let survol = response.contains_pointer();
                    let retenue = editeur.class == Some(*index);
                    // **Tout est gris, sauf ce qu'on vise.** Le même geste que la galerie
                    // d'avatars : la couleur suit le curseur, et la classe retenue la garde.
                    paint_class_avatar(
                        ui,
                        rect,
                        class,
                        editeur.gender,
                        !(survol || retenue),
                        ctx.avatars,
                        ctx.icons,
                    );
                    ui.painter().rect_stroke(
                        rect,
                        TILE_RADIUS,
                        egui::Stroke::new(
                            if survol || retenue { 2.0 } else { 1.0 },
                            if survol || retenue {
                                design::tokens::TEXT_GOLD
                            } else {
                                TILE_BORDER
                            },
                        ),
                        egui::StrokeKind::Inside,
                    );
                    // Le nom de la classe n'est plus écrit sous le portrait : il est dans
                    // l'infobulle, au-dessus de la tuile visée.
                    design::tooltip(&response).text(*label);
                    if response.clicked() {
                        editeur.class = Some(*index);
                    }
                }
            });
        }
        if visibles.is_empty() {
            ui.add_space(20.0);
            ui.add(
                design::info_text("Aucune classe ne porte ce nom.")
                    .width(COLONNE)
                    .log_name("personnages.modale.vide"),
            );
        }
    });

    // **« Valider » exige un nom ET une classe** : un personnage sans classe n'aurait pas de buste,
    // et un sans nom ne serait jamais reconnu dans le journal. Le bouton reste cliquable et ne fait
    // simplement rien — la modale montre déjà ce qui manque (champ vide, aucune tuile dorée), et
    // griser « Valider » sans dire pourquoi est le reproche classique.
    let complet = !editeur.name.trim().is_empty() && editeur.class.is_some();
    let ferme = match chrome.footer {
        design::FooterClick::Validate if complet => {
            let (class_name, _) = CLASSES[editeur.class.expect("vérifié juste au-dessus")];
            let personnage = RosterCharacter {
                name: editeur.name.trim().to_string(),
                class_name: class_name.to_string(),
                gender: editeur.gender,
            };
            match editeur.index {
                Some(index) => {
                    tracing::info!(nom = %personnage.name, "[options] personnage modifié");
                    ctx.roster.replace(state.account, index, personnage);
                }
                None => {
                    tracing::info!(nom = %personnage.name, "[options] personnage déclaré");
                    ctx.roster.declare(state.account, personnage);
                    // Le sexe choisi devient le défaut de la création suivante — voir règle 3.
                    state.gender_default = editeur.gender;
                }
            }
            true
        }
        design::FooterClick::Validate => false,
        design::FooterClick::Cancel => true,
        design::FooterClick::None => chrome.close,
    };
    if ferme {
        state.editor = None;
    } else {
        state.editor = Some(editeur);
    }
}

/// Le nom que l'édition en cours porte AU ROSTER (pas dans le champ) — ce qu'il ne faut pas
/// compter comme un doublon de lui-même dans les suggestions.
fn editeur_nom_initial<'a>(
    editeur: &CharacterEditor,
    state: &PersonnagesTabState,
    ctx: &'a PersonnagesTabContext<'_>,
) -> Option<&'a str> {
    let index = editeur.index?;
    ctx.roster
        .accounts
        .get(state.account)?
        .characters
        .get(index)
        .map(|c| c.name.as_str())
}

/// Les classes que la recherche laisse passer, avec leur rang dans [`CLASSES`].
fn classes_visibles(recherche: &str) -> Vec<(usize, &'static (&'static str, &'static str))> {
    let requete = normalise(recherche);
    let filtre =
        (requete.chars().count() >= design::tokens::AUTOCOMPLETE_MIN_QUERY_LEN).then_some(requete);
    CLASSES
        .iter()
        .enumerate()
        .filter(|(_, (_, label))| match filtre.as_deref() {
            Some(q) => normalise(label).contains(q),
            None => true,
        })
        .collect()
}

/// Taille de la modale de compte — deux champs, rien de plus.
const COMPTE_MODALE: Vec2 = Vec2::new(460.0, 420.0);

/// **La modale de compte.** Un compte du roster, c'est un libellé et un serveur de jeu : le site
/// n'en demande pas plus (`CharacterRosterService.addAccount`).
fn modale_compte(
    ui: &mut egui::Ui,
    state: &mut PersonnagesTabState,
    ctx: &mut PersonnagesTabContext<'_>,
) {
    let Some(mut editeur) = state.account_editor.clone() else {
        return;
    };
    let mut couche = couche_modale(ui, ctx.window, "compte");
    let rect = Rect::from_center_size(ctx.window.center(), COMPTE_MODALE);
    let mut modale = couche.new_child(egui::UiBuilder::new().max_rect(rect));
    modale.set_clip_rect(Rect::EVERYTHING);
    let chrome = design::window("Nouveau compte")
        .footer("Annuler", "Valider")
        .close_button(true)
        .log_name("personnages.compte.modale")
        .show(&mut modale);

    design::panel().show(&mut modale, chrome.content, |ui, panel| {
        let width = panel.inner.width();
        ui.label(
            RichText::new("Nom du compte")
                .color(SUBDUED)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        );
        ui.add_space(6.0);
        ui.add(
            design::input(&mut editeur.name)
                .size(InputSize::Search)
                .placeholder("Mules, Métiers, Second écran…")
                .width(width)
                .log_name("personnages.compte.nom"),
        );
        ui.add_space(14.0);
        ui.label(
            RichText::new("Serveur de jeu")
                .color(SUBDUED)
                .font(design::text::label_font(ui.ctx(), BODY_FONT_SIZE)),
        );
        ui.add_space(6.0);
        let courant = editeur.server.clone();
        let mut select = design::select(&mut editeur.server)
            .width(220.0)
            .log_name("personnages.compte.serveur");
        for serveur in ctx.servers.selectable(courant.as_deref()) {
            select = select.option(Some(serveur.code.clone()), serveur.label.clone());
        }
        select = select.option(None, "Aucun");
        select.show(ui);
        ui.add_space(14.0);
        ui.add(
            design::info_text(
                "Le serveur départage deux personnages de même nom. Il se change plus tard.",
            )
            .width(width)
            .log_name("personnages.compte.aide"),
        );
    });

    // Un compte sans nom est refusé : la liste déroulante en afficherait deux « Compte 2 »
    // indiscernables. Comme pour la modale de personnage, « Valider » ne fait rien plutôt que d'être
    // grisé sans explication.
    let ferme = match chrome.footer {
        design::FooterClick::Validate if !editeur.name.trim().is_empty() => {
            let compte = overlay_engine::RosterAccount::new(
                editeur.name.trim().to_string(),
                editeur.server.clone(),
            );
            tracing::info!(nom = %compte.label, "[options] compte ajouté au roster");
            ctx.roster.accounts.push(compte);
            // Le nouveau compte devient celui qu'on regarde : c'est pour y déclarer qu'on vient de
            // le créer.
            state.account = ctx.roster.accounts.len() - 1;
            state.select_mode = false;
            state.selected.clear();
            true
        }
        design::FooterClick::Validate => false,
        design::FooterClick::Cancel => true,
        design::FooterClick::None => chrome.close,
    };
    if ferme {
        state.account_editor = None;
    } else {
        state.account_editor = Some(editeur);
    }
}

/// **La suppression d'un compte** — la question porte le nom du compte ET son décompte : un
/// « Supprimer ce compte ? » laisserait l'utilisateur ignorer ce qu'il emporte.
fn confirmation_de_compte(
    ui: &mut egui::Ui,
    state: &mut PersonnagesTabState,
    ctx: &mut PersonnagesTabContext<'_>,
    index: usize,
) {
    let Some(compte) = ctx.roster.accounts.get(index) else {
        state.pending_account_removal = None;
        return;
    };
    let nom = account_label(ctx.roster, index);
    let question = match compte.characters.len() {
        0 => format!("Supprimer le compte « {nom} » ?"),
        1 => format!("Supprimer le compte « {nom} » et son personnage ?"),
        n => format!("Supprimer le compte « {nom} » et ses {n} personnages ?"),
    };
    match design::confirm_dialog(&question)
        .over(ctx.window)
        .log_name("personnages.compte.confirmation")
        .show(ui)
    {
        design::ConfirmChoice::Yes => {
            tracing::info!(compte = %nom, "[options] compte retiré du roster");
            ctx.roster.remove_account(index);
            state.account = ctx.roster.default_index();
            state.reset_gestes();
        }
        design::ConfirmChoice::No => state.pending_account_removal = None,
        design::ConfirmChoice::Pending => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_classes_couvrent_les_bustes_disponibles() {
        // Une classe listée ici sans buste sortirait une tuile vide dans la grille de choix.
        let mut cles: Vec<&str> = CLASSES.iter().map(|(key, _)| *key).collect();
        cles.sort_unstable();
        let mut attendues = overlay_engine::class_breed::CLASS_PORTRAIT_ORDER.to_vec();
        attendues.sort_unstable();
        assert_eq!(cles, attendues);
    }

    #[test]
    fn la_recherche_de_classe_ignore_casse_et_accents() {
        // Sous le seuil : la grille reste entière.
        assert_eq!(classes_visibles("cr").len(), CLASSES.len());
        assert_eq!(classes_visibles("").len(), CLASSES.len());
        // « cra » doit trouver « Crâ », « eli » doit trouver « Éliotrope ».
        let cra = classes_visibles("CRA");
        assert_eq!(cra.len(), 1);
        assert_eq!(cra[0].1 .0, "cra");
        let eli = classes_visibles("éLI");
        assert_eq!(eli.len(), 1);
        assert_eq!(eli[0].1 .0, "eliotrope");
        // Un nom qu'aucune classe ne porte : la grille est vide, l'écran le dit.
        assert!(classes_visibles("zzz").is_empty());
    }

    #[test]
    fn le_libelle_de_compte_suit_celui_du_site() {
        let roster = Roster::from_settings_json(&serde_json::json!({ "roster": [
            { "id": "a", "label": "", "isDefault": true, "characters": [] },
            { "id": "b", "label": "  Mules  ", "characters": [] },
            { "id": "c", "label": "   ", "characters": [] },
        ]}));
        assert_eq!(account_label(&roster, 0), "Principal");
        assert_eq!(account_label(&roster, 1), "Mules");
        // Sans libellé : un nom générique, jamais une entrée vide dans la liste déroulante.
        assert_eq!(account_label(&roster, 2), "Compte 3");
    }

    #[test]
    fn une_classe_inconnue_du_registre_n_a_pas_de_rang() {
        assert_eq!(class_index("iop"), Some(7));
        assert_eq!(class_index("archimonstre"), None);
    }
}
