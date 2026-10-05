//! **La modale d'édition d'un compteur de suivi** (2026-10-05, remontée utilisateur) — ouverte par
//! le crayon d'une tuile du bandeau in-game (`panels::watchlist::edit_button`).
//!
//! ## Pourquoi éditer à la main
//!
//! Le compteur d'un suivi n'avançait que par le log. Trois cas l'ont rendu insuffisant :
//!
//! - **un bug** a fait monter la valeur à tort, ou a manqué un ramassage — l'utilisateur doit
//!   pouvoir remettre la vraie valeur, à la baisse comme à la hausse ;
//! - **un suivi tenu avant l'overlay** : le suivi activé après coup repart de zéro, et
//!   l'utilisateur veut y reporter ce qu'il avait déjà ;
//! - **la réinitialisation**, qui avait jusque-là son propre bouton sur la tuile et sa propre
//!   confirmation : elle n'est plus qu'une valeur de plus, posée ici.
//!
//! ## Ce qu'elle montre
//!
//! Le titre « Suivi » dans la bannière ; en dessous, l'emplacement de l'élément suivi (son icône et
//! son liseré de rareté, nom et mode en infobulle) avec, à sa droite, le bouton `Undo` qui pose la
//! valeur de départ ; puis le pas numérique du design system (`design::stepper`), éditable au
//! clavier, borné à l'échelle du mode. Pied « Annuler / Valider ».
//!
//! ## Rien n'est écrit avant « Valider »
//!
//! Le pas et le bouton de réinitialisation ne modifient qu'un **brouillon** ([`CounterEditState`]),
//! tenu par l'hôte le temps que la modale est ouverte. « Annuler », la croix et Échap le jettent :
//! le compteur garde la valeur qu'il avait. « Valider » (ou Entrée) ne fait quelque chose que si le
//! brouillon diffère de la valeur lue à l'ouverture ([`CounterEditState::changed`]) — valider sans
//! rien changer vaut annuler, et un ramassage survenu pendant que la modale était ouverte n'est pas
//! écrasé.
//!
//! La réinitialisation ne demande donc plus de confirmation : elle n'écrit rien elle-même, c'est
//! « Valider » qui le fera.
//!
//! ## Où elle vit
//!
//! Dans la fenêtre voilée qui couvre le jeu, celle des confirmations
//! (`OverlayKind::ResetConfirm(ResetTarget::WatchlistCounter)`) : le voile, le centrage et le cycle
//! « ouvrir / fermer » des deux hôtes sont les mêmes, seule la boîte peinte dedans change. La
//! réponse remonte par le même `ConfirmChoice` — « Oui » pour « Valider ».

use egui::{Rect, Vec2};
use overlay_engine::{CatalogIndex, WatchlistEntry, WatchlistKind, WatchlistMode};

use crate::design::{self, ConfirmChoice, DsIcon, IconContext, SlotFrame};
use crate::rarity_bridge::to_slot_rarity;
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::ui_icons::UiIcons;

/// Taille de la modale — la bannière (56), l'emplacement, le pas et le pied, sans plus : c'est une
/// question à une valeur, pas une fenêtre de réglages.
pub const DIALOG_SIZE: Vec2 = Vec2::new(300.0, 268.0);
/// Côté de l'emplacement de l'élément suivi — celui de l'objet source de la fenêtre de recette,
/// agrandi : ici l'élément EST le sujet de la fenêtre.
const SLOT_SIZE: f32 = 56.0;
/// Côté du bouton de réinitialisation, et son écart à l'emplacement.
const UNDO_SIZE: f32 = 32.0;
const UNDO_GAP: f32 = 12.0;
/// Écart entre l'emplacement et le pas.
const STEPPER_GAP: f32 = 18.0;
/// Largeur du champ du pas — sept chiffres y tiennent, au-delà de tout compteur observé.
const FIELD_WIDTH: f32 = 120.0;
/// Plafond du mode incrémental, qui n'a pas de cible : le champ doit bien s'arrêter quelque part.
const UP_MAX: i64 = 9_999_999;

/// Le brouillon de la modale — tenu par l'hôte de l'ouverture à la réponse.
#[derive(Debug, Clone, PartialEq)]
pub struct CounterEditState {
    /// L'entrée telle qu'elle était à l'ouverture : son identité désigne le compteur au moteur, et
    /// son `count` est la valeur que « Valider » compare au brouillon.
    pub entry: WatchlistEntry,
    /// La valeur en cours d'édition.
    pub value: i64,
}

impl CounterEditState {
    /// Un brouillon qui part de la valeur affichée par la tuile.
    pub fn new(entry: WatchlistEntry) -> Self {
        let value = entry.count;
        Self { entry, value }
    }

    /// La valeur à écrire à la validation — `None` si le brouillon n'a pas bougé : « Valider »
    /// fait alors la même chose qu'« Annuler ».
    pub fn changed(&self) -> Option<i64> {
        (self.value != self.entry.count).then_some(self.value)
    }

    /// **Ce que « Valider » envoie au moteur** — `None` si le brouillon n'a pas bougé. Partagé par
    /// les deux hôtes, qui n'ont plus qu'à transmettre.
    pub fn into_command(self) -> Option<crate::engine_thread::EngineCommand> {
        let count = self.changed()?;
        Some(crate::engine_thread::EngineCommand::SetWatchlistCounter {
            name: self.entry.name,
            kind: self.entry.kind,
            count,
        })
    }
}

/// Ce dont la modale a besoin pour montrer l'élément suivi — le même jeu que la tuile du bandeau.
pub struct CounterEditDeps<'a> {
    pub icons: &'a UiIcons,
    pub catalog: &'a CatalogIndex,
    pub remote_icons: &'a RemoteIconStore,
    pub remote_icon_textures: &'a mut RemoteIconTextures,
}

/// Peint la modale centrée dans `over` sous le voile, et rend la réponse de cette frame :
/// [`ConfirmChoice::Yes`] pour « Valider » ou Entrée, [`ConfirmChoice::No`] pour « Annuler », la
/// croix ou Échap.
pub fn show(
    ui: &mut egui::Ui,
    over: Rect,
    state: &mut CounterEditState,
    deps: &mut CounterEditDeps<'_>,
) -> ConfirmChoice {
    let chrome = design::scrim(over)
        .centered(DIALOG_SIZE)
        .log_name("suivi.edition")
        .show(ui, |fenetre| {
            let chrome = design::window("Suivi")
                .tab_bar_height(0.0)
                .footer("Annuler", "Valider")
                .close_button(true)
                .log_name("suivi.edition")
                .show(fenetre);
            paint_body(fenetre, chrome.content, state, deps);
            chrome
        })
        .inner;

    let mut choix = match chrome.footer {
        design::FooterClick::Validate => ConfirmChoice::Yes,
        design::FooterClick::Cancel => ConfirmChoice::No,
        design::FooterClick::None => ConfirmChoice::Pending,
    };
    if chrome.close {
        choix = ConfirmChoice::No;
    }
    // Échap jette le brouillon, Entrée le valide. Le pas a déjà fait sienne une saisie valide à la
    // frappe (voir `design::stepper`) : la valeur lue par l'hôte est bien celle qu'on voit.
    ui.input(|i| {
        if i.key_pressed(egui::Key::Escape) {
            choix = ConfirmChoice::No;
        } else if i.key_pressed(egui::Key::Enter) {
            choix = ConfirmChoice::Yes;
        }
    });
    if choix != ConfirmChoice::Pending {
        tracing::debug!(
            name = %state.entry.name,
            avant = state.entry.count,
            apres = state.value,
            reponse = ?choix,
            "[suivi] modale d'édition tranchée"
        );
    }
    choix
}

/// L'emplacement et son bouton de réinitialisation, centrés ensemble, puis le pas centré dessous.
fn paint_body(
    ui: &mut egui::Ui,
    content: Rect,
    state: &mut CounterEditState,
    deps: &mut CounterEditDeps<'_>,
) {
    let entry = &state.entry;
    let icon_ref = match entry.kind {
        WatchlistKind::Item => deps.catalog.find_item_icon(&entry.name, entry.catalog_id),
        WatchlistKind::Enemy => deps
            .catalog
            .find_monster_icon(&entry.name, entry.catalog_id),
    };
    let icone = icon_ref
        .and_then(|icon_ref| {
            deps.remote_icon_textures
                .resolve(ui.ctx(), deps.remote_icons, &icon_ref)
                .map(|handle| egui::load::SizedTexture::from_handle(&handle))
        })
        .unwrap_or_else(|| {
            egui::load::SizedTexture::from_handle(deps.icons.unknown_entity_texture())
        });
    let frame = match entry.kind {
        WatchlistKind::Item => SlotFrame::Rarity(to_slot_rarity(
            deps.catalog.find_item_rarity(&entry.name, entry.catalog_id),
        )),
        WatchlistKind::Enemy => SlotFrame::Plain,
    };

    // **L'emplacement est centré seul**, le bouton se pose à sa droite : c'est l'élément suivi qui
    // fait l'axe de la fenêtre, comme le pas en dessous — centrer le couple les décalerait l'un de
    // l'autre.
    let slot_rect = Rect::from_center_size(
        egui::pos2(content.center().x, content.top() + SLOT_SIZE / 2.0),
        Vec2::splat(SLOT_SIZE),
    );
    let slot = ui.put(
        slot_rect,
        design::item_slot()
            .frame(frame)
            .icon(icone)
            .size(SLOT_SIZE)
            .log_name("suivi.edition.element"),
    );
    design::tooltip(&slot).text(crate::panels::watchlist::tile_tooltip(entry));

    let depart = entry.starting_count();
    let undo_rect = Rect::from_center_size(
        egui::pos2(
            slot_rect.right() + UNDO_GAP + UNDO_SIZE / 2.0,
            slot_rect.center().y,
        ),
        Vec2::splat(UNDO_SIZE),
    );
    if ui
        .put(
            undo_rect,
            design::icon_button(DsIcon::Undo)
                .context(IconContext::Panel)
                .size(UNDO_SIZE)
                .tooltip("Réinitialiser")
                .log_name("suivi.edition.reinitialiser"),
        )
        .clicked()
    {
        state.value = depart;
    }

    let max = match entry.mode {
        WatchlistMode::Up => UP_MAX,
        WatchlistMode::Down | WatchlistMode::Goal => entry.max_count().unwrap_or(UP_MAX),
    };
    let pas = design::stepper(&mut state.value)
        .range(0..=max)
        .field_width(FIELD_WIDTH)
        .log_name("suivi.edition.valeur");
    let largeur = pas.desired_size().0.unwrap_or(FIELD_WIDTH);
    let hauteur = design::tokens::STEPPER_SIZE;
    let pas_rect = Rect::from_center_size(
        egui::pos2(
            content.center().x,
            slot_rect.bottom() + STEPPER_GAP + hauteur / 2.0,
        ),
        Vec2::new(largeur, hauteur),
    );
    ui.put(pas_rect, pas);
}
