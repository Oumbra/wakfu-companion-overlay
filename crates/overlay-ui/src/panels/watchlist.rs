//! Panneau "Suivi" (watchlist, §9 du plan) — bande horizontale de tuiles carrées, à l'image du
//! bandeau du dépôt web (`tracker-strip.component`/`.kpi`) : demande utilisateur explicite
//! 2026-09-01 (capture d'écran de référence à l'appui), qui remplace la première version en liste
//! verticale de lignes (retours utilisateur précédents, gardée en mémoire dans l'historique Git
//! mais plus dans ce fichier). Rendu dans sa PROPRE fenêtre overlay (`OverlayKind::Watchlist`,
//! `main.rs`), décorrélée de la fenêtre Combat — demande utilisateur explicite : les deux zones
//! doivent pouvoir, à terme, être pilotées indépendamment en visibilité.
//!
//! Toujours en LECTURE SEULE (voir `overlay_engine::watchlist` pour la frontière
//! définitions/compteurs) : les deux premières tuiles ("+"/"−", voir `control_tile`) reprennent la
//! forme du bandeau web (ajouter un suivi / sélection multiple + suppression) mais restent
//! INERTES ici — aucun formulaire d'ajout, aucune sélection ne sont câblés côté overlay pour cette
//! itération (demande utilisateur : "je pense qu'on le fera plus tard quand tu auras tout câblé").
//! Un survol affiche un tooltip explicite plutôt que de laisser un bouton cliquable qui ne ferait
//! rien silencieusement (retour utilisateur déjà vécu sur le bouton de connexion au compte).
//!
//! Icône réelle de chaque tuile (retour utilisateur 2026-09-02 : « comme les images de
//! ressources/monstres n'est pas présent c'est très compliqué pour l'utilisateur » de distinguer
//! les tuiles entre elles) — résolue via `overlay_engine::CatalogIndex` (lot L3 réduit, voir
//! `catalog.rs`) puis téléchargée/décodée par `remote_icons::RemoteIconStore`, mise en cache par
//! fenêtre dans `remote_icons::RemoteIconTextures`. Repli sur l'icône générique (`UiIcons::
//! unknown_entity_texture`) tant que l'entrée n'est pas résolue par le catalogue OU que son icône
//! n'a pas fini de télécharger — jamais une tuile vide ou un blocage du rendu.

use overlay_engine::{CatalogIndex, WatchlistEntry, WatchlistKind, WatchlistMode};

use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::ui_icons::UiIcons;

/// Durée d'affichage du toast d'alerte (§9 du plan : « toast ≤ 5 s, non bloquant, jamais
/// interactif ») — exportée pour que `main.rs` calcule `hide_at` avec la même valeur, sans la
/// dupliquer.
pub const TOAST_DURATION: std::time::Duration = std::time::Duration::from_secs(5);

/// Un décompte de suivi vient d'atteindre 0 (voir `overlay_engine::WatchlistAlert`) — construit
/// par `main.rs::spawn_engine_thread` à réception de l'alerte, publié via `ArcSwap` (comme
/// `watchlist`/`snapshot`) pour que le thread UI l'affiche sans coupler le thread Engine au rendu.
#[derive(Debug, Clone)]
pub struct WatchlistToast {
    pub name: String,
    pub kind: WatchlistKind,
    /// Instant auquel le toast doit cesser de s'afficher — comparé à `Instant::now()` à chaque
    /// rendu (voir `show`) plutôt que de faire expirer activement l'`ArcSwap` : cette architecture
    /// n'a pas de boucle de rendu continue (§6.1 du plan), `main.rs::render` reprogramme lui-même
    /// un redessin à cette échéance via `OverlayWindow::next_redraw_at` pour que le toast
    /// disparaisse sans qu'aucun autre événement n'ait à se produire.
    pub hide_at: std::time::Instant,
}

const TILE_SIZE: f32 = 52.0;
const TILE_GAP: f32 = 6.0;
const TILE_ROUNDING: f32 = 8.0;
const ICON_SIZE: f32 = 30.0;

const TILE_BG: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(18, 20, 28, 235);
const CONTROL_BORDER: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 90);
const CONTROL_GLYPH: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 170);

// Même bleu que le switch/la barre de dégâts du panneau Combat (`panels::combat::ACCENT`) — un
// objet suivi partage la charte ; un ennemi suivi s'en distingue par une teinte chaude, seule
// façon de les différencier sans icône dédiée par entrée (voir la doc de module).
const ITEM_MARKER: egui::Color32 = egui::Color32::from_rgb(0x00, 0xd2, 0xff);
const ENEMY_MARKER: egui::Color32 = egui::Color32::from_rgb(0xff, 0x6b, 0x5b);

const BADGE_BG: egui::Color32 = egui::Color32::from_rgb(0x0a, 0x0c, 0x12);
const BADGE_TEXT: egui::Color32 = egui::Color32::from_rgb(235, 240, 245);

/// Couleur du texte du toast (`toast_banner`) — même teinte claire que le reste de l'interface
/// sombre de l'overlay.
const NAME_COLOR: egui::Color32 = egui::Color32::from_rgb(220, 224, 230);

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    entries: &[WatchlistEntry],
    toast: Option<&WatchlistToast>,
) {
    egui::ScrollArea::horizontal()
        .id_salt("watchlist-strip")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                control_tile(ui, "+", "Ajouter un suivi (bientôt disponible)");
                ui.add_space(TILE_GAP);
                control_tile(
                    ui,
                    "−",
                    "Sélection multiple / suppression (bientôt disponible)",
                );
                ui.add_space(TILE_GAP);

                for (i, entry) in entries.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(TILE_GAP);
                    }
                    entry_tile(
                        ui,
                        icons,
                        catalog,
                        remote_icons,
                        remote_icon_textures,
                        entry,
                    );
                }
            });
        });

    // Espace TOUJOURS réservé sous la bande de tuiles (fenêtre dimensionnée en conséquence, voir
    // `main.rs::WATCHLIST_WINDOW_SIZE`) plutôt qu'agrandir la fenêtre à la volée à l'apparition
    // d'un toast : ancrage déjà mis au point avec l'utilisateur (2026-09-01, plusieurs allers-
    // retours) pour la bande de tuiles elle-même — l'y toucher à nouveau pour un toast occasionnel
    // aurait tout redécalé. Invisible quand inactif (rien n'est peint, fond transparent).
    ui.add_space(6.0);
    if let Some(toast) = toast.filter(|t| t.hide_at > std::time::Instant::now()) {
        toast_banner(ui, toast);
    }
}

/// Toast d'alerte (§9 du plan : « toast + son quand un objet suivi tombe ») — non interactif
/// (`Sense::hover()` seulement), disparaît de lui-même après `TOAST_DURATION` (voir la doc de
/// `WatchlistToast::hide_at`). Pas de confettis contrairement à `loot-alert.component.ts` : le
/// plan (§9) ne demande qu'« un toast ≤ 5 s, non bloquant, jamais interactif », une bannière de
/// texte suffit pour cette itération.
fn toast_banner(ui: &mut egui::Ui, toast: &WatchlistToast) {
    let marker_color = match toast.kind {
        WatchlistKind::Item => ITEM_MARKER,
        WatchlistKind::Enemy => ENEMY_MARKER,
    };
    ui.horizontal(|ui| {
        let (dot_rect, _resp) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
        ui.painter()
            .circle_filled(dot_rect.center(), 4.0, marker_color);
        ui.label(
            egui::RichText::new(format!("Suivi terminé : {}", toast.name))
                .color(NAME_COLOR)
                .strong(),
        );
    });
}

/// Tuile "+"/"−" du bandeau web — bordure en pointillés (`egui::Shape::dashed_line`, pas de
/// primitive "rectangle en pointillés" dans `epaint`, reconstruite à la main à partir des 4
/// coins) : signale visuellement qu'il s'agit d'une action, pas d'une entrée suivie, cohérent avec
/// la charte `.kpi-add` du web.
fn control_tile(ui: &mut egui::Ui, glyph: &str, tooltip: &str) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(TILE_SIZE, TILE_SIZE), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, TILE_ROUNDING, TILE_BG);

    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    painter.extend(egui::Shape::dashed_line(
        &corners,
        egui::Stroke::new(1.0, CONTROL_BORDER),
        4.0,
        3.0,
    ));

    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::proportional(20.0),
        CONTROL_GLYPH,
    );

    response.on_hover_text(tooltip);
}

/// Tuile d'une entrée suivie : icône réelle si le catalogue la résout et qu'elle a fini de
/// télécharger (voir doc de module), repli générique sinon — bordée de la couleur de `kind`,
/// badge de compteur ancré en bas-à-droite (même position que `.kpi-count-badge` côté web). Nom
/// complet en tooltip — jamais tronqué silencieusement sans recours, y compris avec une icône
/// réelle (contrairement au web, dont l'image elle-même porte souvent assez d'info visuelle).
fn entry_tile(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    entry: &WatchlistEntry,
) {
    let marker_color = match entry.kind {
        WatchlistKind::Item => ITEM_MARKER,
        WatchlistKind::Enemy => ENEMY_MARKER,
    };

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(TILE_SIZE, TILE_SIZE), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, TILE_ROUNDING, TILE_BG);
    painter.rect_stroke(
        rect,
        TILE_ROUNDING,
        egui::Stroke::new(1.5, marker_color),
        egui::StrokeKind::Inside,
    );

    let icon_ref = match entry.kind {
        WatchlistKind::Item => catalog.find_item_icon(&entry.name, entry.catalog_id),
        WatchlistKind::Enemy => catalog.find_monster_icon(&entry.name, entry.catalog_id),
    };
    let remote_texture = icon_ref
        .as_ref()
        .and_then(|icon_ref| remote_icon_textures.resolve(ui.ctx(), remote_icons, icon_ref));

    // `paint_at` peint directement DANS le rect donné (ignore fit_to_exact_size/
    // maintain_aspect_ratio, qui ne s'appliquent qu'au layout via `ui.add`) — même motif que
    // `panels::combat::draw_centered_icon`, pas la peine de les poser ici.
    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(ICON_SIZE, ICON_SIZE));
    match &remote_texture {
        Some(texture) => egui::Image::new(texture).paint_at(ui, icon_rect),
        None => egui::Image::new(icons.unknown_entity_texture()).paint_at(ui, icon_rect),
    }

    count_badge(ui, rect, entry);

    response.on_hover_text(&entry.name);
}

/// Badge ancré au coin bas-droit de la tuile — texte selon le mode, miroir du bandeau web (retour
/// utilisateur 2026-09-02, capture d'écran à l'appui : « contrairement au mode incrémental qui
/// n'affiche qu'un nombre qui s'incrémente, un décompte part d'un nombre et réduit, donc
/// visuellement... le nombre courant est affiché par rapport au nombre attendu ») :
/// - `up` : le compte seul (ex. `"3"`).
/// - `down` : compte courant sur cible (ex. `"5/10"`) — PAS de conversion en "déjà collecté" (le
///   web n'affiche que `count`/`countdownTarget` bruts, jamais `target - count`).
///
/// Forme en pilule (rectangle très arrondi) plutôt qu'un cercle forcé : un cercle imposerait sa
/// hauteur comme largeur minimale, ce qui déborderait ou tronquerait un texte "10/10" bien plus
/// large qu'un simple chiffre — la pilule s'adapte à la largeur du texte dans les deux cas.
fn count_badge(ui: &mut egui::Ui, tile_rect: egui::Rect, entry: &WatchlistEntry) {
    let text = match entry.mode {
        WatchlistMode::Down => format!("{}/{}", entry.count, entry.countdown_target),
        WatchlistMode::Up => entry.count.to_string(),
    };
    let font = egui::FontId::monospace(11.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.clone(), font.clone(), BADGE_TEXT);
    let size = (galley.size() + egui::vec2(10.0, 4.0)).max(egui::vec2(18.0, 16.0));
    let center = tile_rect.right_bottom() - (size / 2.0 + egui::vec2(2.0, 2.0));
    let badge_rect = egui::Rect::from_center_size(center, size);

    let painter = ui.painter();
    painter.rect_filled(badge_rect, size.y / 2.0, BADGE_BG);
    painter.rect_stroke(
        badge_rect,
        size.y / 2.0,
        egui::Stroke::new(1.0, egui::Color32::BLACK),
        egui::StrokeKind::Inside,
    );
    painter.text(center, egui::Align2::CENTER_CENTER, &text, font, BADGE_TEXT);
}
