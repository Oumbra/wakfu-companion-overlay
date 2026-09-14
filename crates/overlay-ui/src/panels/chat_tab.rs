//! Onglet « Chat » de la fenêtre Options — **les recherches qui font sonner l'overlay**, et rien
//! d'autre : le jeu affiche déjà son chat et filtre déjà ses canaux (recadrage de l'utilisateur,
//! 2026-09-13, après une première maquette qui reportait le panneau web entier). Ce que le joueur
//! ne peut pas faire dans le jeu, c'est être **prévenu** : un mot, sur un canal ou sur tous, et
//! quand un message y correspond, un son et une carte par-dessus le jeu (`panels::watchlist::
//! toast_card`, variante `WatchlistToastReason::Chat`), dont le clic prépare une réponse en privé.
//!
//! Construit sur le modèle de `panels::alerts_tab` — même squelette (titre, phrase, « Tester le
//! son », ligne de fermeture, formulaire, grille défilable), mêmes jetons, même contrat
//! transactionnel : l'onglet travaille sur un brouillon ([`ChatDraft`]) que « Valider » commit
//! avec tous les autres onglets (voir `main.rs::commit_chat`).
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

use egui::{Color32, Rect, RichText, Vec2};
use overlay_engine::{
    ChatFilter, ChatFilterScope, DEFAULT_ALERT_DURATION_SECONDS, MAX_ALERT_DURATION_SECONDS,
    MIN_ALERT_DURATION_SECONDS,
};

use crate::design::{self, ButtonSize, ButtonVariant, DsIcon, IconContext, InputSize};

// -------------------------------------------------------------------------------------------
// Jetons — repris TELS QUELS de `panels::alerts_tab`, pour que les deux onglets se ressemblent
// au pixel (voir leur provenance là-bas).
// -------------------------------------------------------------------------------------------

const TEXT: Color32 = Color32::WHITE;
const SUBDUED: Color32 = Color32::from_rgb(0xB8, 0xB9, 0xBA);
const SETTING_ROW_FILL: Color32 = Color32::from_rgb(0x26, 0x28, 0x2B);
const SETTING_ROW_RADIUS: u8 = 4;
const SETTING_ROW_HEIGHT: f32 = 40.0;
const ROW_HEIGHT: f32 = 39.0;
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
    /// La durée telle que tapée — voir `AlertsTabState::duration_input` pour pourquoi une chaîne.
    pub duration_input: String,
    /// La dernière raison pour laquelle « Ajouter » n'a rien ajouté, effacée à la prochaine
    /// frappe ou au prochain ajout réussi.
    pub notice: Option<AddError>,
}

impl Default for ChatTabState {
    fn default() -> Self {
        Self {
            scope: ChatFilterScope::All,
            input: String::new(),
            duration_input: String::new(),
            notice: None,
        }
    }
}

/// Ce que l'onglet reçoit — le brouillon et sa disponibilité.
pub struct ChatTabContext<'a> {
    pub draft: &'a mut ChatDraft,
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

/// Ce que l'onglet demande à l'hôte — jamais exécuté ici (§17.3 bis du plan).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ChatTabAction {
    #[default]
    None,
    /// « Tester le son » : jouer le son de recherche.
    TestSound,
}

pub fn show(
    ui: &mut egui::Ui,
    panel: &design::PanelZones,
    state: &mut ChatTabState,
    ctx: &mut ChatTabContext<'_>,
) -> ChatTabAction {
    let mut action = ChatTabAction::None;
    let width = panel.inner.width();

    ui.add(design::heading("Chat"));
    paragraph(ui, DESC);
    ui.add_space(SECTION_GAP);

    if test_sound_row(ui, width) {
        action = ChatTabAction::TestSound;
    }
    ui.add_space(SECTION_GAP);

    match ctx.availability {
        ChatAvailability::Loading => {
            loading_row(ui, panel.inner);
            return action;
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
            return action;
        }
        ChatAvailability::Ready => {}
    }

    close_settings_row(ui, state, &mut ctx.draft.toast, width);
    ui.add_space(SECTION_GAP);

    ui.add(design::heading("Recherches"));
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
    ui.add_space(SECTION_GAP * 0.75);

    if ctx.draft.filters.is_empty() {
        ui.add(
            design::info_text(
                "Aucune recherche. Choisissez un canal, saisissez un mot, puis cliquez sur Ajouter.",
            )
            .width(width)
            .log_name("chat.vide"),
        );
        return action;
    }

    tile_grid(ui, panel, ctx.draft);
    action
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

/// « Tester le son » — la même ligne que dans Alertes ; c'est le son du web
/// (`chat-filter-*.mp3`) que l'hôte joue, voir `alert_sound::play_chat_alert`.
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
                    .log_name("chat.tester"),
            )
            .clicked();
    });
    clicked
}

/// Le bloc « Fermeture automatique » — copie conforme de `panels::alerts_tab::close_settings_row`,
/// sur les réglages de la carte de chat.
fn close_settings_row(
    ui: &mut egui::Ui,
    state: &mut ChatTabState,
    toast: &mut ChatToastSettings,
    width: f32,
) {
    let row = ui.allocate_space(Vec2::new(width, SETTING_ROW_HEIGHT)).1;
    ui.painter()
        .rect_filled(row, SETTING_ROW_RADIUS, SETTING_ROW_FILL);
    let mut auto = !toast.manual_close;
    let mut cell = ui.new_child(egui::UiBuilder::new().max_rect(row.shrink2(Vec2::new(12.0, 0.0))));
    cell.horizontal_centered(|ui| {
        if ui
            .add(design::checkbox(&mut auto, "Fermeture automatique").log_name("chat.auto"))
            .clicked()
        {
            toast.manual_close = !auto;
        }
        ui.add_space(12.0);
        let response = ui.add(
            design::input(&mut state.duration_input)
                .size(InputSize::Standard)
                .width(52.0)
                .enabled(auto)
                .log_name("chat.duree"),
        );
        if response.lost_focus() {
            toast.set_duration(parse_duration(
                &state.duration_input,
                toast.duration_seconds,
            ));
            state.duration_input = format_duration(toast.duration_seconds);
        }
        ui.label(RichText::new("sec.").color(SUBDUED).size(BODY_FONT_SIZE));
    });
}

/// Lit une durée tapée — virgule décimale comprise ; une saisie illisible garde la valeur en
/// place (voir `panels::alerts_tab::parse_duration`).
pub fn parse_duration(raw: &str, actuelle: f32) -> f32 {
    raw.trim()
        .replace(',', ".")
        .parse::<f32>()
        .unwrap_or(actuelle)
}

/// Écrit une durée dans le champ — sans décimale inutile, avec la virgule française.
pub fn format_duration(seconds: f32) -> String {
    if seconds.fract().abs() < f32::EPSILON {
        format!("{}", seconds as i64)
    } else {
        format!("{seconds:.1}").replace('.', ",")
    }
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
fn tile_grid(ui: &mut egui::Ui, panel: &design::PanelZones, draft: &mut ChatDraft) {
    // Collecter avant de muter : le rendu lit le brouillon, le geste le modifie.
    // La légende prend la couleur du canal (celle du client, `tokens::chat_channel_color`) ;
    // « Tous les canaux » garde le gris des légendes.
    let items: Vec<(String, Option<Color32>, String)> = draft
        .filters
        .iter()
        .map(|f| {
            let color = match f.scope {
                ChatFilterScope::All => None,
                ChatFilterScope::Channel(channel) => {
                    Some(design::tokens::chat_channel_color(channel))
                }
            };
            (f.scope.label().to_owned(), color, f.text.clone())
        })
        .collect();
    let mut removed: Option<usize> = None;
    panel.scroll_area(ui, "chat.recherches", |ui, content_width| {
        ui.spacing_mut().item_spacing = Vec2::splat(TILE_GAP);
        let tile_width =
            (content_width - TILE_GAP * (TILES_PER_ROW as f32 - 1.0)) / TILES_PER_ROW as f32;
        for (row_index, chunk) in items.chunks(TILES_PER_ROW).enumerate() {
            ui.horizontal(|ui| {
                for (col, (legend, color, text)) in chunk.iter().enumerate() {
                    let index = row_index * TILES_PER_ROW + col;
                    if filter_tile(ui, legend, *color, text, tile_width) {
                        removed = Some(index);
                    }
                }
            });
        }
    });
    if let Some(index) = removed {
        tracing::info!(index, "[options] recherche de chat retirée");
        draft.remove(index);
    }
}

/// Une tuile, et sa croix. `true` quand la croix vient d'être cliquée.
fn filter_tile(
    ui: &mut egui::Ui,
    legend: &str,
    legend_color: Option<Color32>,
    text: &str,
    width: f32,
) -> bool {
    let mut tile = design::legend_tile(legend, text).width(width);
    if let Some(color) = legend_color {
        tile = tile.legend_color(color);
    }
    let response = ui.add(tile.log_name(format!("chat.recherche.{text}")));
    // **`contains_pointer` et NON `hovered`** : la croix a sa propre zone, posée par-dessus la
    // tuile — dès que le pointeur l'atteint, egui lui donne le survol. Piège déjà payé au Suivi
    // et dans Alertes.
    if !response.contains_pointer() {
        return false;
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
    croix.clicked()
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

    #[test]
    fn la_duree_est_bornee_et_survit_a_une_saisie_illisible() {
        let mut toast = ChatToastSettings::default();
        toast.set_duration(0.0);
        assert_eq!(toast.duration_seconds, MIN_ALERT_DURATION_SECONDS);
        toast.set_duration(999.0);
        assert_eq!(toast.duration_seconds, MAX_ALERT_DURATION_SECONDS);
        assert_eq!(parse_duration("3,5", 1.0), 3.5);
        assert_eq!(parse_duration("abc", 1.0), 1.0);
        assert_eq!(format_duration(4.0), "4");
        assert_eq!(format_duration(3.5), "3,5");
    }
}
