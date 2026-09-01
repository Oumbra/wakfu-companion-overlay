//! Panneau "Suivi" (watchlist, §9 du plan) — bande horizontale de tuiles carrées, à l'image du
//! bandeau du dépôt web (`tracker-strip.component`/`.kpi`) : demande utilisateur explicite
//! 2026-09-01/02 (captures d'écran de référence à l'appui), qui remplace la première version en
//! liste verticale de lignes (retours utilisateur précédents, gardée en mémoire dans l'historique
//! Git mais plus dans ce fichier). Rendu dans sa PROPRE fenêtre overlay (`OverlayKind::Watchlist`,
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
//!
//! **Refonte visuelle 2026-09-02** (retour utilisateur, capture d'écran du bandeau web à l'appui,
//! comparée au rendu overlay) — trois écarts corrigés pour rester RACCORD avec le web :
//! - Bordure/fond selon la RARETÉ de l'objet (`--rarity-color`, `styles.css` du dépôt web, voir
//!   `RARITY_COLORS`) plutôt qu'un simple bleu/orange objet/ennemi — miroir de `.kpi[class*=
//!   'rarity-']` (`tracker-strip.component.css`) : bordure pleine de la couleur de rareté, fond en
//!   dégradé diagonal de `mix(rarity 35%, panel-bg)` vers `panel-bg`. Un ennemi n'a pas de rareté
//!   (`.kpi.is-monster`) : bordure grise unie (`--text-muted`), fond plat, jamais de dégradé.
//! - Badge de compteur déplacé à L'EXTÉRIEUR du coin bas-droit (miroir de `.kpi-count-badge`,
//!   `bottom:-7px; right:-7px` — la tuile elle-même n'a plus AUCUN élément qui la recouvre),
//!   au lieu d'être peint PAR-DESSUS l'icône comme dans la version précédente.
//! - Couleurs du texte du badge : mode `up` en clair neutre, mode `down` avec la valeur COURANTE
//!   en couleur kamas (`--kama-color`, or) et la cible en gris (`--text-muted`) — miroir de
//!   `.kpi-count-badge.is-fraction`/`.kpi-count-badge-target`.

use overlay_engine::{CatalogIndex, WakfuRarity, WatchlistEntry, WatchlistKind, WatchlistMode};

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

/// 58×58, coins arrondis 10px — mêmes dimensions que `.kpi` (`tracker-strip.component.css`),
/// repliée (pas l'état `.expanded` au clic, jamais câblé côté overlay pour l'instant).
const TILE_SIZE: f32 = 58.0;
/// Un peu plus que le simple espacement visuel du web (`.kpi-strip { gap: ... }`) : le badge de
/// compteur déborde maintenant du coin bas-droit de la tuile (voir `count_badge`, miroir de
/// `overflow: visible` côté web) — assez de marge pour qu'un badge large ("999/999") ne chevauche
/// pas l'icône de la tuile suivante.
const TILE_GAP: f32 = 12.0;
const TILE_ROUNDING: f32 = 10.0;
/// `app-item-icon [size]="30"` dans le template web — mêmes proportions.
const ICON_SIZE: f32 = 30.0;

const CONTROL_BORDER: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 90);
const CONTROL_GLYPH: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 170);

// Jetons repris tels quels de `:root` (`styles.css`, thème sombre par défaut — seul thème que
// l'overlay reproduit pour l'instant, voir `panels::combat::ACCENT` et sa propre justification).
/// `--panel-bg`.
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(0x1e, 0x1e, 0x1e);
/// `--surface-well` — fond du badge de compteur.
const SURFACE_WELL: egui::Color32 = egui::Color32::from_rgb(0x18, 0x18, 0x18);
/// `--border-strong` — bordure du badge ET bordure "monstre" (`.kpi.is-monster`).
const BORDER_STRONG: egui::Color32 = egui::Color32::from_rgb(0x4d, 0x4d, 0x4d);
/// `--text-muted` — cible grisée d'un décompte, texte des tuiles "+"/"−".
const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(0x88, 0x88, 0x88);
/// `--text-color` — texte neutre (badge en mode `up`).
const TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(0xe0, 0xe0, 0xe0);
/// `--kama-color` — valeur COURANTE d'un décompte (`.kpi-count-badge.is-fraction`).
const KAMA_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 215, 0);

/// Couleur du texte du toast — même teinte claire que le reste de l'interface sombre de l'overlay.
const NAME_COLOR: egui::Color32 = TEXT_COLOR;

/// Miroir des 7 `.rarity-xxx { --rarity-color: ... }` de `styles.css` (thème sombre) —
/// `WakfuRarity::Old` n'est jamais réellement résolue au runtime (voir sa doc), sa couleur n'a
/// donc aucune conséquence visible ; conservée à `TEXT_MUTED` par cohérence plutôt qu'une valeur
/// arbitraire.
fn rarity_color(rarity: WakfuRarity) -> egui::Color32 {
    match rarity {
        WakfuRarity::Old => TEXT_MUTED,
        WakfuRarity::Common => egui::Color32::from_rgb(0xc8, 0xc8, 0xc8),
        WakfuRarity::Rare => egui::Color32::from_rgb(0x1d, 0xd1, 0x5f),
        WakfuRarity::Mythical => egui::Color32::from_rgb(0xd9, 0x7a, 0x00),
        WakfuRarity::Legendary => egui::Color32::from_rgb(0xc7, 0xd4, 0x00),
        WakfuRarity::Memory => egui::Color32::from_rgb(0x1f, 0x97, 0xe0),
        WakfuRarity::Epic => egui::Color32::from_rgb(0xd8, 0x4f, 0xa0),
        WakfuRarity::Relic => egui::Color32::from_rgb(0x94, 0x50, 0xd9),
    }
}

/// Miroir de `color-mix(in srgb, a t%, b)` — mélange canal par canal dans l'espace sRGB (mêmes
/// composantes que `Color32`, pas de conversion vers un espace linéaire : `color-mix(in srgb, ...)`
/// est explicitement demandé côté CSS, pas `in oklab`/`in lab`).
fn mix(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 * t + y as f32 * (1.0 - t)).round() as u8;
    egui::Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

pub fn show(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    entries: &[WatchlistEntry],
    toast: Option<&WatchlistToast>,
) {
    let mut style = (**ui.style()).clone();
    style_thin_scrollbar(&mut style);
    ui.set_style(style);

    egui::ScrollArea::horizontal()
        .id_salt("watchlist-strip")
        .auto_shrink([false, true])
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

/// Barre de défilement fine, flottante et sombre plutôt que le style natif par défaut (épais, pris
/// dans le flux, gris clair) — retour utilisateur 2026-09-02 (capture d'écran à l'appui) : « le
/// scroll passe SUR les objets », « ce n'est pas très moderne », « qu'on ait juste à scroller
/// avec la molette plutôt que de bouger le scroll manuellement ». `ScrollStyle::thin()` (preset
/// `egui`, flottant + fin au repos + s'élargit au survol) répond exactement à ça — seules les
/// COULEURS restent celles d'egui par défaut (pensées pour un thème clair) sans cet ajustement,
/// remplacées ici par les jetons déjà utilisés pour le badge de compteur, cohérents avec le reste
/// du panneau. S'applique à tout le `Style` de CE contexte egui (une fenêtre = un contexte, voir
/// `main.rs`) : sans effet sur la fenêtre Combat, qui n'a pas de zone défilante.
fn style_thin_scrollbar(style: &mut egui::Style) {
    style.spacing.scroll = egui::style::ScrollStyle::thin();
    style.visuals.widgets.noninteractive.bg_fill = SURFACE_WELL;
    style.visuals.widgets.inactive.bg_fill = BORDER_STRONG;
    style.visuals.widgets.hovered.bg_fill = TEXT_MUTED;
    style.visuals.widgets.active.bg_fill = TEXT_MUTED;
}

/// Toast d'alerte (§9 du plan : « toast + son quand un objet suivi tombe ») — non interactif
/// (`Sense::hover()` seulement), disparaît de lui-même après `TOAST_DURATION` (voir la doc de
/// `WatchlistToast::hide_at`). Pas de confettis contrairement à `loot-alert.component.ts` : le
/// plan (§9) ne demande qu'« un toast ≤ 5 s, non bloquant, jamais interactif », une bannière de
/// texte suffit pour cette itération.
fn toast_banner(ui: &mut egui::Ui, toast: &WatchlistToast) {
    let marker_color = match toast.kind {
        WatchlistKind::Item => rarity_color(WakfuRarity::Common),
        WatchlistKind::Enemy => TEXT_MUTED,
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
    painter.rect_filled(rect, TILE_ROUNDING, PANEL_BG);

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
/// télécharger (voir doc de module), repli générique sinon — bordure/fond selon la rareté (objet)
/// ou gris uni (ennemi, voir `tile_style`), badge de compteur ancré HORS du coin bas-droit (voir
/// `count_badge`). Nom complet en tooltip — jamais tronqué silencieusement sans recours, y compris
/// avec une icône réelle (contrairement au web, dont l'image elle-même porte souvent assez
/// d'info visuelle).
fn entry_tile(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    entry: &WatchlistEntry,
) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(TILE_SIZE, TILE_SIZE), egui::Sense::hover());

    let (border_color, gradient_from) = match entry.kind {
        WatchlistKind::Item => {
            let rarity = catalog.find_item_rarity(&entry.name, entry.catalog_id);
            let color = rarity_color(rarity);
            (color, Some(mix(color, PANEL_BG, 0.35)))
        }
        WatchlistKind::Enemy => (BORDER_STRONG, None),
    };

    let painter = ui.painter();
    match gradient_from {
        Some(from) => rounded_gradient_rect(painter, rect, TILE_ROUNDING, from, PANEL_BG),
        None => {
            painter.rect_filled(rect, TILE_ROUNDING, PANEL_BG);
        }
    }
    painter.rect_stroke(
        rect,
        TILE_ROUNDING,
        egui::Stroke::new(2.0, border_color),
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

/// Peint un rectangle à coins arrondis rempli d'un dégradé diagonal simple (haut-gauche vers
/// bas-droite) — miroir de `linear-gradient(to bottom right, from, to 70%)`
/// (`tracker-strip.component.css`, `.kpi[class*='rarity-']`). Pas de primitive `epaint` pour un
/// dégradé (`RectShape::fill` est un `Color32` unique) : maillé à la main avec les mêmes coins
/// arrondis qu'un `Painter::rect_filled` (4 arcs de quelques segments, largement suffisant à
/// l'échelle d'une tuile de 58px) — chaque sommet reçoit la couleur interpolée selon sa position
/// sur l'axe diagonal du rectangle (approximation propre d'un dégradé CSS, pas un rendu pixel
/// identique, invisible à cette taille).
fn rounded_gradient_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    radius: f32,
    from: egui::Color32,
    to: egui::Color32,
) {
    const ARC_SEGMENTS: usize = 6;
    let radius = radius
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0)
        .max(0.0);

    // Centre de chaque coin + plage d'angle (repère écran, y vers le bas) — même ordre que le
    // contour d'un rectangle arrondi standard, sens horaire depuis le haut-droit.
    let quarter = std::f32::consts::FRAC_PI_2;
    let corners = [
        (
            egui::pos2(rect.right() - radius, rect.top() + radius),
            -quarter,
        ), // haut-droit
        (
            egui::pos2(rect.right() - radius, rect.bottom() - radius),
            0.0,
        ), // bas-droit
        (
            egui::pos2(rect.left() + radius, rect.bottom() - radius),
            quarter,
        ), // bas-gauche
        (
            egui::pos2(rect.left() + radius, rect.top() + radius),
            2.0 * quarter,
        ), // haut-gauche
    ];

    let diagonal = (rect.width() * rect.width() + rect.height() * rect.height()).sqrt();
    let color_at = |p: egui::Pos2| -> egui::Color32 {
        let along_diagonal = (p - rect.left_top()).dot(rect.right_bottom() - rect.left_top());
        let t = (along_diagonal / (diagonal * diagonal)).clamp(0.0, 1.0);
        mix(to, from, t)
    };

    let mut mesh = egui::Mesh::default();
    let center_index = mesh.vertices.len() as u32;
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.center(),
        uv: egui::epaint::WHITE_UV,
        color: color_at(rect.center()),
    });

    let mut outline_indices = Vec::with_capacity(corners.len() * (ARC_SEGMENTS + 1));
    for (center, start_angle) in corners {
        for i in 0..=ARC_SEGMENTS {
            let angle = start_angle + quarter * (i as f32 / ARC_SEGMENTS as f32);
            let pos = center + radius * egui::vec2(angle.cos(), angle.sin());
            outline_indices.push(mesh.vertices.len() as u32);
            mesh.vertices.push(egui::epaint::Vertex {
                pos,
                uv: egui::epaint::WHITE_UV,
                color: color_at(pos),
            });
        }
    }

    for window in outline_indices.windows(2) {
        mesh.indices
            .extend_from_slice(&[center_index, window[0], window[1]]);
    }
    if let (Some(&first), Some(&last)) = (outline_indices.first(), outline_indices.last()) {
        mesh.indices.extend_from_slice(&[center_index, last, first]);
    }

    painter.add(egui::Shape::mesh(mesh));
}

/// Badge ancré HORS du coin bas-droit de la tuile (miroir de `.kpi-count-badge`, `bottom:-7px;
/// right:-7px` — la tuile elle-même reste entièrement dégagée, contrairement à la version
/// précédente qui peignait le badge PAR-DESSUS l'icône). Texte selon le mode (retour utilisateur
/// 2026-09-02, capture d'écran à l'appui) :
/// - `up` : le compte seul, en clair neutre (`TEXT_COLOR`).
/// - `down` : compte courant EN COULEUR KAMAS (`KAMA_COLOR`) sur cible grisée (`TEXT_MUTED`),
///   PAS de conversion en "déjà collecté" (le web n'affiche que `count`/`countdownTarget` bruts).
///
/// Forme en pilule (rectangle très arrondi) plutôt qu'un cercle forcé : un cercle imposerait sa
/// hauteur comme largeur minimale, ce qui déborderait ou tronquerait un texte "10/10" bien plus
/// large qu'un simple chiffre — la pilule s'adapte à la largeur du texte dans les deux cas.
fn count_badge(ui: &mut egui::Ui, tile_rect: egui::Rect, entry: &WatchlistEntry) {
    let font = egui::FontId::monospace(11.0);
    let painter = ui.painter();

    let (current_text, target_part) = match entry.mode {
        WatchlistMode::Down => (
            entry.count.to_string(),
            Some(format!("/{}", entry.countdown_target)),
        ),
        WatchlistMode::Up => (entry.count.to_string(), None),
    };
    let current_color = if target_part.is_some() {
        KAMA_COLOR
    } else {
        TEXT_COLOR
    };

    let current_galley = painter.layout_no_wrap(current_text, font.clone(), current_color);
    let target_galley =
        target_part.map(|text| painter.layout_no_wrap(text, font.clone(), TEXT_MUTED));

    let text_width = current_galley.size().x + target_galley.as_ref().map_or(0.0, |g| g.size().x);
    let text_height = current_galley
        .size()
        .y
        .max(target_galley.as_ref().map_or(0.0, |g| g.size().y));
    let size =
        (egui::vec2(text_width, text_height) + egui::vec2(12.0, 4.0)).max(egui::vec2(18.0, 16.0));

    // `bottom:-7px; right:-7px` (CSS) : le coin bas-droit du BADGE se place 7px au-delà du coin
    // bas-droit de la TUILE, pas de son centre — miroir direct plutôt qu'un simple recentrage sur
    // le coin.
    let badge_max = tile_rect.right_bottom() + egui::vec2(7.0, 7.0);
    let badge_rect = egui::Rect::from_min_size(badge_max - size, size);

    painter.rect_filled(badge_rect, badge_rect.height() / 2.0, SURFACE_WELL);
    painter.rect_stroke(
        badge_rect,
        badge_rect.height() / 2.0,
        egui::Stroke::new(1.0, BORDER_STRONG),
        egui::StrokeKind::Inside,
    );

    let text_start = badge_rect.center() - egui::vec2(text_width / 2.0, 0.0);
    painter.galley(
        egui::pos2(
            text_start.x,
            badge_rect.center().y - current_galley.size().y / 2.0,
        ),
        current_galley.clone(),
        current_color,
    );
    if let Some(target_galley) = target_galley {
        painter.galley(
            egui::pos2(
                text_start.x + current_galley.size().x,
                badge_rect.center().y - target_galley.size().y / 2.0,
            ),
            target_galley,
            TEXT_MUTED,
        );
    }
}
