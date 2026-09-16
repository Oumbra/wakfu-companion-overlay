//! Onglet « Chat » de la fenêtre Options — **les recherches qui font sonner l'overlay**, et rien
//! d'autre : le jeu affiche déjà son chat et filtre déjà ses canaux (recadrage de l'utilisateur,
//! 2026-09-13, après une première maquette qui reportait le panneau web entier). Ce que le joueur
//! ne peut pas faire dans le jeu, c'est être **prévenu** : un mot, sur un canal ou sur tous, et
//! quand un message y correspond, un son et une carte par-dessus le jeu (`panels::watchlist::
//! toast_card`, variante `WatchlistToastReason::Chat`), dont le clic prépare une réponse en privé.
//!
//! Construit sur le modèle de `panels::alerts_tab` — même squelette (titre, phrase, formulaire,
//! grille défilable), mêmes jetons, même contrat transactionnel : l'onglet travaille sur un
//! brouillon ([`ChatDraft`]) que « Valider » commit avec tous les autres onglets (voir
//! `main.rs::commit_chat`).
//!
//! **Le son et la fermeture de la carte ne se règlent plus ici** (2026-09-15) : l'essai du son, sa
//! sourdine et la fermeture automatique sont partis dans la section « Chat » de l'onglet
//! « Paramètres », avec celles du Suivi et des Alertes — voir [`crate::panels::notifications`].
//! Cet onglet ne garde que ce qu'il liste : les recherches.
//!
//! ## Ce qui vient du compte, et ce qui reste local
//!
//! Les **recherches** sont la clé `chatFilters` du compte (`/api/v1/settings`), au format du web
//! (voir `overlay_engine::chat_alert`) : un compte utilisé depuis le site et depuis l'overlay voit la
//! même liste des deux côtés. La **durée de la carte**, elle, n'a pas d'équivalent web (le panneau
//! web n'affiche pas de carte) et le serveur n'accepte que des clés connues : elle vit dans la config
//! locale (`config::OverlayConfig`), comme l'encombrement du panneau Combat. C'est une exception
//! assumée au principe « ce qui appartient au joueur passe par le compte » — le jour où le serveur
//! porte une clé pour ce réglage, elle y migre.
//!
//! ## La maquette validée
//!
//! `crates/overlay-testkit/examples/chat-mockups.rs`, planche « recherches en tuiles » (retenue par
//! l'utilisateur contre la liste en lignes) : canal d'abord, mot ensuite, « Ajouter » enfin, ancré
//! au bord droit ; tuiles à légende ([`design::legend_tile`]) quatre par rangée, croix révélée au
//! survol comme les tuiles d'Alertes ; aucune couleur de canal (les thèmes du jeu).
//!
//! ## La suppression multiple, 2026-09-16
//!
//! Demande utilisateur : « ajouter le système de la suppression multiple, comme dans l'onglet
//! Suivi ». La mécanique est **partagée** — elle vit dans [`crate::panels::bulk_select`], avec les
//! onglets « Suivi » et « Alertes ». Ici toutes les recherches se retirent (aucune n'est
//! « par défaut », contrairement aux Alertes), et la case à cocher se pose au coin **haut-droit**
//! de la tuile, celui de la croix qu'elle remplace : le haut-gauche porte la légende.

use egui::{Color32, Rect, RichText, Vec2};
use overlay_engine::{
    ChatFilter, ChatFilterScope, DEFAULT_ALERT_DURATION_SECONDS, MAX_ALERT_DURATION_SECONDS,
    MIN_ALERT_DURATION_SECONDS,
};

use crate::design::{self, ButtonSize, ButtonVariant, DsIcon, InputSize};
use crate::panels::{bulk_select, feature_switch, notifications};

// -------------------------------------------------------------------------------------------
// Jetons — repris TELS QUELS de `panels::alerts_tab`, pour que les deux onglets se ressemblent
// au pixel (voir leur provenance là-bas).
// -------------------------------------------------------------------------------------------

const TEXT: Color32 = Color32::WHITE;
const BODY_FONT_SIZE: f32 = 15.0;
const SECTION_GAP: f32 = 18.0;
/// Gouttière entre deux tuiles — `panels::alerts_tab::TILE_GAP`, le pas de la grille d'Alertes.
const TILE_GAP: f32 = 12.0;
/// Tuiles par rangée — quatre : à la largeur utile du panneau, ce qui laisse à chaque tuile la
/// place d'un mot de quinze lettres sans ellipse (maquette du 2026-09-13).
const TILES_PER_ROW: usize = 4;
/// Côté de la croix de retrait et son retrait depuis le coin — `panels::alerts_tab::TILE_BADGE`
/// et `TILE_BADGE_INSET`.
const TILE_BADGE: f32 = 14.0;
const TILE_BADGE_INSET: f32 = 8.0;
/// Rouge de la croix sous le pointeur — `panels::alerts_tab::REMOVE_HOVER`.
const REMOVE_HOVER: Color32 = design::tokens::INFO_ALERT;

const DESC: &str = "Un son est joué et une carte s'affiche par-dessus le jeu dès qu'un message du \
                    chat contient l'un des mots ci-dessous, sur le canal choisi ou sur tous.";

/// Réglages de la carte d'alerte de chat — le pendant de `duration_seconds`/`manual_close` de
/// `AlertProfile`, propre à l'onglet Chat (décision utilisateur du 2026-09-13 : « réglage propre à
/// l'onglet Chat »). Persisté localement, voir la doc de module.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChatToastSettings {
    /// Durée d'affichage de la carte, en secondes. Ignorée quand [`Self::manual_close`] est vrai.
    pub duration_seconds: f32,
    /// La carte ne se ferme qu'à la main.
    pub manual_close: bool,
}

impl Default for ChatToastSettings {
    fn default() -> Self {
        Self {
            duration_seconds: DEFAULT_ALERT_DURATION_SECONDS,
            manual_close: false,
        }
    }
}

impl ChatToastSettings {
    /// Pose une durée, bornée aux mêmes limites que celle des alertes de ramassage.
    pub fn set_duration(&mut self, seconds: f32) {
        self.duration_seconds = if seconds.is_finite() {
            seconds.clamp(MIN_ALERT_DURATION_SECONDS, MAX_ALERT_DURATION_SECONDS)
        } else {
            DEFAULT_ALERT_DURATION_SECONDS
        };
    }
}

/// Ce qu'il faut à la section « Chat » de l'onglet « Paramètres » pour régler la fermeture de
/// cette carte — voir `panels::notifications::ToastClose` : le bornage reste ici, le peintre n'en
/// refait pas un à lui.
impl notifications::ToastClose for ChatToastSettings {
    fn manual_close(&self) -> bool {
        self.manual_close
    }
    fn set_manual_close(&mut self, manual: bool) {
        self.manual_close = manual;
    }
    fn duration_seconds(&self) -> f32 {
        self.duration_seconds
    }
    fn set_duration(&mut self, seconds: f32) {
        ChatToastSettings::set_duration(self, seconds);
    }
}

/// Le brouillon de l'onglet — ce que « Valider » commit.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChatDraft {
    pub filters: Vec<ChatFilter>,
    pub toast: ChatToastSettings,
}

impl ChatDraft {
    /// Ajoute une recherche. `false` si le mot est vide ou si la même recherche (mot ET canal)
    /// existe déjà — la règle d'`addFilter` côté web.
    pub fn add(&mut self, scope: ChatFilterScope, raw: &str) -> Result<(), AddError> {
        let filter = ChatFilter::new(scope, raw).ok_or(AddError::Empty)?;
        if self.filters.contains(&filter) {
            return Err(AddError::Duplicate);
        }
        self.filters.push(filter);
        Ok(())
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.filters.len() {
            self.filters.remove(index);
        }
    }

    /// Retire les recherches dont la clé ([`filter_key`]) est cochée. Rend combien sont parties.
    pub fn remove_keys(&mut self, keys: &[String]) -> usize {
        let cochees: std::collections::HashSet<&String> = keys.iter().collect();
        let avant = self.filters.len();
        self.filters
            .retain(|filter| !cochees.contains(&filter_key(filter)));
        avant - self.filters.len()
    }
}

/// Identifie une recherche — la clé de coche du mode sélection, jumelle de
/// `panels::suivi_tab::entry_key` et `panels::alerts_tab::entry_key`.
///
/// Le couple (canal, mot) est déjà ce qui rend une recherche unique : `ChatDraft::add` refuse le
/// doublon sur ce couple exact, et rien d'autre ne distingue deux entrées.
pub(crate) fn filter_key(filter: &ChatFilter) -> String {
    format!("{}::{}", filter.scope.label(), filter.text)
}

/// Pourquoi une recherche n'a pas été ajoutée — chaque cas a sa phrase, aucun n'est une erreur
/// du programme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddError {
    Empty,
    Duplicate,
}

impl AddError {
    pub fn message(self) -> &'static str {
        match self {
            AddError::Empty => "Saisissez un mot à rechercher.",
            AddError::Duplicate => "Cette recherche existe déjà pour ce canal.",
        }
    }
}

/// Ce qui survit d'une frame à l'autre et n'appartient pas au brouillon : les saisies en cours.
#[derive(Debug, Clone)]
pub struct ChatTabState {
    /// Le canal choisi dans le sélecteur — « Tous les canaux » à l'ouverture, comme le sélecteur
    /// « Global » du web.
    pub scope: ChatFilterScope,
    /// Le mot en cours de saisie.
    pub input: String,
    /// La durée telle que tapée — voir `panels::notifications::AutoClose::input` pour pourquoi
    /// une chaîne. **Le champ qu'elle alimente est peint dans l'onglet « Paramètres »** depuis le
    /// 2026-09-15 ; elle reste ici, avec le brouillon dont elle règle la carte.
    pub duration_input: String,
    /// La dernière raison pour laquelle « Ajouter » n'a rien ajouté, effacée à la prochaine
    /// frappe ou au prochain ajout réussi.
    pub notice: Option<AddError>,
    /// Mode « sélection multiple » ouvert — voir [`crate::panels::bulk_select`].
    pub select_mode: bool,
    /// Clés des tuiles cochées — voir [`filter_key`].
    pub selected: Vec<String>,
}

impl Default for ChatTabState {
    fn default() -> Self {
        Self {
            scope: ChatFilterScope::All,
            input: String::new(),
            duration_input: String::new(),
            notice: None,
            select_mode: false,
            selected: Vec::new(),
        }
    }
}

/// Ce que l'onglet reçoit — le brouillon et sa disponibilité.
pub struct ChatTabContext<'a> {
    pub draft: &'a mut ChatDraft,
    /// **La fonctionnalité est-elle active ?** — brouillon de la case « Activer la recherche » peinte tout
    /// en haut de l'onglet (voir `panels::feature_switch`), pas un réglage que cet onglet
    /// applique : c'est « Valider » qui l'emporte, comme le reste de la fenêtre. Décochée, tout le
    /// contenu sous la case est grisé et inerte.
    pub enabled: &'a mut bool,
    pub availability: ChatAvailability,
}

/// Le brouillon est-il là ? Mêmes trois cas que `AlertsAvailability`, pour les mêmes raisons :
/// les recherches descendent du compte.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ChatAvailability {
    #[default]
    Ready,
    Loading,
    NoAccount,
}

/// **Cet onglet ne demande plus rien à l'hôte** : « Tester le son » est parti dans l'onglet
/// « Paramètres » avec le reste des réglages de notification (2026-09-15, voir
/// `panels::notifications`), et il était la seule intention que cet écran produisait.
pub fn show(
    ui: &mut egui::Ui,
    panel: &design::PanelZones,
    state: &mut ChatTabState,
    ctx: &mut ChatTabContext<'_>,
) {
    let width = panel.inner.width();

    ui.add(design::heading("Chat"));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP);

    // Voir `panels::feature_switch` — l'interrupteur grise tout ce qui suit quand il est décoché.
    feature_switch::show(
        ui,
        ctx.enabled,
        "Activer la recherche",
        "Décoché, aucun message du chat ne fait plus sonner l'overlay ni n'affiche de carte. \
         Vos recherches sont conservées.",
        "chat.activer",
    );

    match ctx.availability {
        ChatAvailability::Loading => {
            loading_row(ui, panel.inner);
            return;
        }
        ChatAvailability::NoAccount => {
            ui.add(
                design::info_text(
                    "Aucun compte lié : les recherches sont enregistrées sur votre compte, \
                     liez-le depuis l'onglet « Paramètres ».",
                )
                .width(width)
                .log_name("chat.sans-compte"),
            );
            return;
        }
        ChatAvailability::Ready => {}
    }

    // **La ligne de création AVANT le titre de la liste** (demande du 2026-09-16, la même que
    // pour le champ d'ajout des Alertes) : on ajoute, puis on voit ce qu'on a — le titre coiffe la
    // grille qu'il nomme, pas le formulaire qui l'alimente.
    add_row(ui, state, ctx.draft, width);
    if let Some(notice) = state.notice {
        ui.add_space(6.0);
        ui.add(
            design::info_text(notice.message())
                .tone(design::InfoTone::Alert)
                .width(width)
                .log_name("chat.ajout-refuse"),
        );
    }
    ui.add_space(SECTION_GAP);

    // **Les commandes de suppression multiple sont dans cette ligne** depuis le 2026-09-16 —
    // mécanique partagée, voir `panels::bulk_select`. Toutes les recherches se retirent : le
    // nombre de tuiles EST le nombre de retirables, et une liste vide ne montre aucun bouton.
    let demande = bulk_select::show(
        ui,
        width,
        bulk_select::BulkHeader {
            title: "Recherches",
            removable: ctx.draft.filters.len(),
            bulk_tooltip:
                "Retire les recherches cochées — annulable tant que la fenêtre n'est pas \
                           validée",
            log_prefix: "chat",
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
            let retirees = ctx.draft.filters.len();
            ctx.draft.filters.clear();
            tracing::info!(retirees, "[options] recherches de chat retirées en bloc");
        }
        bulk_select::BulkRequest::Keys(cles) => {
            let retirees = ctx.draft.remove_keys(&cles);
            tracing::info!(retirees, "[options] recherches de chat retirées en bloc");
        }
    }

    if ctx.draft.filters.is_empty() {
        ui.add_space(bulk_select::HEADER_TO_PARAGRAPH);
        ui.add(
            design::info_text(
                "Aucune recherche. Choisissez un canal, saisissez un mot, puis cliquez sur Ajouter.",
            )
            .width(width)
            .log_name("chat.vide"),
        );
        return;
    }
    ui.add_space(bulk_select::HEADER_GAP);

    tile_grid(ui, panel, state, ctx.draft);
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

/// Canal, mot, « Ajouter » — **dans cet ordre** (demande explicite) : on dit d'abord OÙ chercher,
/// puis QUOI. `Entrée` dans le champ ajoute aussi.
///
/// **La ligne déborde la largeur utile de 7 px à droite** (retour du 2026-09-13) : la marge entre
/// le bouton et le bord du panneau doit être celle qui sépare le bord gauche du sélecteur
/// (`PANEL_PAD_CONTROL_X`), or la largeur utile s'arrête 26 px avant le bord — la réserve de barre
/// de défilement, que ce formulaire n'a pas à ménager. Le bouton est **ancré au bord droit** dans
/// un enfant en sens inverse : sa marge droite ne dépend alors d'aucune largeur voisine.
fn add_row(ui: &mut egui::Ui, state: &mut ChatTabState, draft: &mut ChatDraft, width: f32) {
    const SCOPE_WIDTH: f32 = 170.0;
    const ADD_WIDTH: f32 = 92.0;
    const GAP: f32 = 12.0;
    let total =
        width + design::components::scroll_area::RESERVE_X - design::tokens::PANEL_PAD_CONTROL_X;
    let row = ui
        .allocate_space(Vec2::new(total, design::tokens::SELECT_HEIGHT))
        .1;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row));
    cell.spacing_mut().item_spacing.x = 0.0;
    let mut submit = false;
    cell.horizontal_centered(|ui| {
        let mut select = design::select(&mut state.scope)
            .option(ChatFilterScope::All, ChatFilterScope::All.label())
            .width(SCOPE_WIDTH)
            .log_name("chat.canal");
        for channel in overlay_engine::CHAT_CHANNELS {
            select = select.option(
                ChatFilterScope::Channel(channel),
                overlay_engine::channel_label(channel),
            );
        }
        select.show(ui);
        ui.add_space(GAP);
        let response = ui.add(
            design::input(&mut state.input)
                .size(InputSize::Search)
                .clearable(true)
                .placeholder("Mot ou expression à rechercher…")
                .width(total - SCOPE_WIDTH - ADD_WIDTH - GAP * 2.0)
                .log_name("chat.mot"),
        );
        if response.changed() {
            state.notice = None;
        }
        // `Entrée` dans le champ : egui lui retire le focus au même moment, c'est le signal.
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            submit = true;
        }
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        if right
            .add(
                design::button("Ajouter")
                    .variant(ButtonVariant::Primary)
                    .size(ButtonSize::Compact)
                    .width(ADD_WIDTH)
                    .log_name("chat.ajouter"),
            )
            .clicked()
        {
            submit = true;
        }
    });
    if submit {
        match draft.add(state.scope, &state.input) {
            Ok(()) => {
                tracing::info!(
                    scope = state.scope.label(),
                    "[options] recherche de chat ajoutée : « {} »",
                    state.input.trim().to_lowercase()
                );
                state.input.clear();
                state.notice = None;
            }
            Err(err) => state.notice = Some(err),
        }
    }
}

/// La grille de recherches : des tuiles à légende, quatre par rangée, la croix de retrait révélée
/// au survol — l'idiome des tuiles d'Alertes (`panels::alerts_tab::alert_item`).
fn tile_grid(
    ui: &mut egui::Ui,
    panel: &design::PanelZones,
    state: &mut ChatTabState,
    draft: &mut ChatDraft,
) {
    // Collecter avant de muter : le rendu lit le brouillon, le geste le modifie.
    // La légende prend la couleur du canal (celle du client, `tokens::chat_channel_color`) ;
    // « Tous les canaux » garde le gris des légendes.
    let items: Vec<TileData> = draft
        .filters
        .iter()
        .map(|f| {
            let color = match f.scope {
                ChatFilterScope::All => None,
                ChatFilterScope::Channel(channel) => {
                    Some(design::tokens::chat_channel_color(channel))
                }
            };
            TileData {
                key: filter_key(f),
                legend: f.scope.label().to_owned(),
                legend_color: color,
                text: f.text.clone(),
            }
        })
        .collect();
    let mut removed: Option<usize> = None;
    let mut coche: Option<String> = None;
    let select_mode = state.select_mode;
    let cochees: std::collections::HashSet<&String> = state.selected.iter().collect();
    panel.scroll_area(ui, "chat.recherches", |ui, content_width| {
        ui.spacing_mut().item_spacing = Vec2::splat(TILE_GAP);
        let tile_width =
            (content_width - TILE_GAP * (TILES_PER_ROW as f32 - 1.0)) / TILES_PER_ROW as f32;
        for (row_index, chunk) in items.chunks(TILES_PER_ROW).enumerate() {
            ui.horizontal(|ui| {
                for (col, tuile) in chunk.iter().enumerate() {
                    let index = row_index * TILES_PER_ROW + col;
                    match filter_tile(
                        ui,
                        tuile,
                        tile_width,
                        select_mode,
                        cochees.contains(&tuile.key),
                    ) {
                        TileClick::Remove => removed = Some(index),
                        TileClick::Check => coche = Some(tuile.key.clone()),
                        TileClick::None => {}
                    }
                }
            });
        }
    });
    if let Some(cle) = coche {
        bulk_select::toggle(&mut state.selected, &cle);
    }
    if let Some(index) = removed {
        tracing::info!(index, "[options] recherche de chat retirée");
        // Une clé cochée qui ne désigne plus rien ferait mentir le compteur du bouton groupé.
        if let Some(filter) = draft.filters.get(index) {
            let cle = filter_key(filter);
            state.selected.retain(|k| *k != cle);
        }
        draft.remove(index);
    }
}

/// Ce qu'une tuile a besoin de savoir — assemblé avant la boucle, voir [`tile_grid`].
struct TileData {
    key: String,
    legend: String,
    legend_color: Option<Color32>,
    text: String,
}

/// Ce qu'un clic sur une tuile signifie.
enum TileClick {
    None,
    /// Retirer la recherche, à la croix du survol.
    Remove,
    /// Cocher ou décocher — le geste de la tuile **en mode sélection**, où il remplace la croix.
    Check,
}

/// Une tuile, et sa croix — ou, en mode sélection, sa case à cocher.
///
/// **La case remplace la croix** : les deux vivent au même coin, et les deux gestes s'excluent
/// (règle reprise du Suivi). La case est peinte par le composant, pas ici — voir
/// [`design::LegendTile::selection`].
fn filter_tile(
    ui: &mut egui::Ui,
    tuile: &TileData,
    width: f32,
    select_mode: bool,
    cochee: bool,
) -> TileClick {
    let TileData {
        legend,
        legend_color,
        text,
        ..
    } = tuile;
    let mut tile = design::legend_tile(legend, text)
        .width(width)
        .selection(select_mode.then_some(cochee))
        // **Le ton destructif**, comme au Suivi et aux Alertes : cocher ici ne mène qu'au bouton
        // « Supprimer », jamais à une autre action.
        .selection_tone(design::SelectionTone::Danger);
    if let Some(color) = legend_color {
        tile = tile.legend_color(*color);
    }
    let response = ui.add(tile.log_name(format!("chat.recherche.{text}")));
    if select_mode {
        // Dans le mode, le geste de la tuile est de cocher — et la croix ne se révèle plus.
        return if response.clicked() {
            TileClick::Check
        } else {
            TileClick::None
        };
    }
    // **`contains_pointer` et NON `hovered`** : la croix a sa propre zone, posée par-dessus la
    // tuile — dès que le pointeur l'atteint, egui lui donne le survol. Piège déjà payé au Suivi
    // et dans Alertes.
    if !response.contains_pointer() {
        return TileClick::None;
    }
    let frame_top = response.rect.top() + design::LegendTile::legend_overshoot(ui);
    let frame = Rect::from_min_max(
        egui::pos2(response.rect.left(), frame_top),
        response.rect.max,
    );
    let ds = design::DesignSystem::get(ui.ctx());
    let native = ds.icon_native_size(DsIcon::Close);
    let center = egui::pos2(
        frame.right() - TILE_BADGE_INSET - TILE_BADGE / 2.0,
        frame.top() + TILE_BADGE_INSET + TILE_BADGE / 2.0,
    );
    let icon_rect = Rect::from_center_size(
        center,
        design::components::icon_button::glyph_fit(native, TILE_BADGE),
    );
    let croix = ui
        .interact(
            Rect::from_center_size(center, Vec2::splat(TILE_BADGE + 4.0)),
            response.id.with("retirer"),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    ds.paint_icon(
        ui.painter(),
        icon_rect,
        DsIcon::Close,
        if croix.hovered() { REMOVE_HOVER } else { TEXT },
    );
    design::tooltip(&croix).text("Retirer");
    if croix.clicked() {
        TileClick::Remove
    } else {
        TileClick::None
    }
}

/// Le rouage de chargement, centré dans la zone que la grille occuperait — même geste que
/// `panels::alerts_tab::loading_row`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use overlay_engine::ChatChannel;

    #[test]
    fn ajouter_refuse_le_vide_et_le_doublon() {
        let mut draft = ChatDraft::default();
        assert_eq!(draft.add(ChatFilterScope::All, "  "), Err(AddError::Empty));
        assert_eq!(draft.add(ChatFilterScope::All, " Gelano "), Ok(()));
        assert_eq!(
            draft.add(ChatFilterScope::All, "gelano"),
            Err(AddError::Duplicate)
        );
        // Le même mot sur un autre canal est une autre recherche.
        assert_eq!(
            draft.add(ChatFilterScope::Channel(ChatChannel::Commerce), "gelano"),
            Ok(())
        );
        assert_eq!(draft.filters.len(), 2);
        draft.remove(0);
        assert_eq!(draft.filters.len(), 1);
        draft.remove(5);
        assert_eq!(draft.filters.len(), 1);
    }

    /// La lecture d'une durée tapée vit désormais dans `panels::notifications`, qui la teste ;
    /// ce qui reste ici est le bornage, qui appartient à ces réglages-ci.
    #[test]
    fn la_duree_est_bornee() {
        let mut toast = ChatToastSettings::default();
        toast.set_duration(0.0);
        assert_eq!(toast.duration_seconds, MIN_ALERT_DURATION_SECONDS);
        toast.set_duration(999.0);
        assert_eq!(toast.duration_seconds, MAX_ALERT_DURATION_SECONDS);
        assert_eq!(
            notifications::ToastClose::duration_seconds(&toast),
            MAX_ALERT_DURATION_SECONDS
        );
    }
}
