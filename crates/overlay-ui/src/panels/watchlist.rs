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

/// Durée d'affichage du toast d'alerte avant fermeture automatique (§9 du plan : « toast ≤ 5 s,
/// non bloquant ») — exportée pour que `main.rs` calcule `hide_at` avec la même valeur, sans la
/// dupliquer. Peut aussi être fermé PLUS TÔT par un clic (voir `toast_card`) : les deux cohabitent,
/// contrairement au réglage exclusif `ProfileService.alertManualClose` côté web.
pub const TOAST_DURATION: std::time::Duration = std::time::Duration::from_secs(5);

/// Distingue les deux déclencheurs de toast possibles (miroir de `LootAlertEvent.reason`,
/// `loot-alert.service.ts`) — seul le libellé affiché change (voir `toast_card`), le son a déjà
/// été choisi par l'appelant (`main.rs::spawn_engine_thread`, `alert_sound::{play_countdown_alert,
/// play_loot_alert}`) avant même la construction de ce toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchlistToastReason {
    /// Un décompte de suivi (mode `down`) vient d'atteindre 0.
    Countdown,
    /// Un objet à son activé (compte, voir `overlay_engine::profile`) vient d'être ramassé —
    /// `quantity` affichée seulement si > 1 (voir `toast_card`).
    Loot { quantity: i64 },
}

/// Un confetti du toast — mêmes bornes aléatoires que `buildConfetti()`
/// (`loot-alert.component.ts`) : position/délai/durée/rotation/couleur tirés UNE FOIS à la
/// création du toast (voir `build_confetti`, appelé par `main.rs::spawn_engine_thread`) puis
/// rejoués en boucle tant que le toast reste affiché — miroir de l'animation CSS `confetti-fall`
/// (`animation-iteration-count: infinite`), ici recalculée à chaque frame depuis
/// `WatchlistToast::created_at` puisque `egui` n'a pas de moteur d'animation CSS (voir `toast_card`).
#[derive(Debug, Clone, Copy)]
pub struct ConfettiPiece {
    /// Position horizontale, en fraction (0.0-1.0) de `TOAST_LAYER_WIDTH` — miroir de `left`
    /// (`Math.random() * 100`, en %).
    left_frac: f32,
    /// Retard avant le début de la chute, en secondes — miroir de `delay` (0.0-0.3s).
    delay: f32,
    /// Durée d'une chute complète, en secondes — miroir de `duration` (1.1-1.9s).
    duration: f32,
    /// Rotation totale atteinte en fin de chute, en radians — miroir de `rotate` (0-360deg,
    /// `--rot` en CSS).
    rotation: f32,
    color: egui::Color32,
}

/// Mêmes 8 couleurs que `CONFETTI_COLORS` (`loot-alert.component.ts`), reprises telles quelles.
const CONFETTI_COLORS: [egui::Color32; 8] = [
    egui::Color32::from_rgb(0xff, 0xb7, 0x03),
    egui::Color32::from_rgb(0xfb, 0x85, 0x00),
    egui::Color32::from_rgb(0x21, 0x9e, 0xbc),
    egui::Color32::from_rgb(0x8e, 0xca, 0xe6),
    egui::Color32::from_rgb(0xff, 0x00, 0x6e),
    egui::Color32::from_rgb(0x83, 0x38, 0xec),
    egui::Color32::from_rgb(0x3a, 0x86, 0xff),
    egui::Color32::from_rgb(0x06, 0xd6, 0xa0),
];
/// Même effectif que `CONFETTI_PIECE_COUNT` (`loot-alert.component.ts`).
const CONFETTI_PIECE_COUNT: usize = 28;

/// Générateur pseudo-aléatoire minimal (xorshift64) — pas de dépendance `rand` pour la seule
/// dispersion visuelle des confettis (aucun besoin cryptographique ni même de reproductibilité).
struct SmallRng(u64);

impl SmallRng {
    /// Graine dérivée de l'horloge système à chaque appel de `build_confetti`, pour que deux
    /// toasts consécutifs n'affichent pas exactement la même dispersion.
    fn seeded() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9_7f4a_7c15);
        Self(nanos | 1) // jamais 0 : état absorbant du xorshift
    }

    /// Suivant, dans `[0.0, 1.0)`.
    fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u32 << 24) as f32
    }
}

/// Construit un jeu de confettis aléatoire — miroir de `buildConfetti()`
/// (`loot-alert.component.ts`), appelé une fois par déclenchement de toast (voir
/// `main.rs::spawn_engine_thread`) pour que la dispersion reste stable tant que le toast est
/// affiché, plutôt que régénérée à chaque frame.
pub fn build_confetti() -> Vec<ConfettiPiece> {
    let mut rng = SmallRng::seeded();
    (0..CONFETTI_PIECE_COUNT)
        .map(|_| ConfettiPiece {
            left_frac: rng.next_f32(),
            delay: rng.next_f32() * 0.3,
            duration: 1.1 + rng.next_f32() * 0.8,
            rotation: rng.next_f32() * std::f32::consts::TAU,
            color: CONFETTI_COLORS
                [(rng.next_f32() * CONFETTI_COLORS.len() as f32) as usize % CONFETTI_COLORS.len()],
        })
        .collect()
}

/// Un décompte de suivi à 0 OU un ramassage à son activé (voir `WatchlistToastReason`) — construit
/// par `main.rs::spawn_engine_thread` à réception de l'alerte, publié via `ArcSwap` (comme
/// `watchlist`/`snapshot`) pour que le thread UI l'affiche sans coupler le thread Engine au rendu.
#[derive(Debug, Clone)]
pub struct WatchlistToast {
    pub name: String,
    pub kind: WatchlistKind,
    pub reason: WatchlistToastReason,
    /// Id catalogue de l'objet/monstre, quand connu — résolution non ambiguë de l'icône affichée
    /// (voir `toast_card`), même principe que `WatchlistEntry::catalog_id`.
    pub catalog_id: Option<i64>,
    /// Instant de création — sert de référence de temps à l'animation d'entrée et à la boucle de
    /// confettis (voir `toast_card`), indépendamment de `hide_at`.
    pub created_at: std::time::Instant,
    /// Dispersion tirée une fois à la création (voir `build_confetti`) — stable tant que le toast
    /// reste affiché.
    pub confetti: Vec<ConfettiPiece>,
    /// Instant auquel le toast doit cesser de s'afficher — comparé à `Instant::now()` à chaque
    /// rendu (voir `show`/`is_active`) plutôt que de faire expirer activement l'`ArcSwap` : cette
    /// architecture n'a pas de boucle de rendu continue (§6.1 du plan), `main.rs::render`
    /// reprogramme lui-même un redessin à cette échéance via `OverlayWindow::next_redraw_at` pour
    /// que le toast disparaisse sans qu'aucun autre événement n'ait à se produire. Peut aussi être
    /// effacé PLUS TÔT par un clic (voir `toast_card`, `main.rs::window_event`).
    pub hide_at: std::time::Instant,
}

/// Vrai tant que `toast` n'a pas atteint son expiration — source de vérité unique utilisée à la
/// fois ici (`show`) et par `main.rs` (gabarit dynamique de la fenêtre Suivi, garde d'affichage
/// quand la watchlist elle-même est vide) sur « un toast est-il actuellement affiché ? ».
pub fn is_active(toast: Option<&WatchlistToast>) -> bool {
    toast.is_some_and(|t| t.hide_at > std::time::Instant::now())
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
/// `bottom:-7px; right:-7px` (`.kpi-count-badge`, `tracker-strip.component.css`) : débordement du
/// badge de compteur HORS du coin bas-droit de sa tuile (voir `count_badge`) — repris ici comme
/// constante nommée plutôt qu'un `7.0` répété, pour que `content_width` réserve exactement la même
/// marge côté DROIT de la dernière tuile. Retour utilisateur 2026-09-02 (capture d'écran à
/// l'appui) : sans cette marge, le badge de la dernière tuile de la bande était tronqué par le bord
/// de la fenêtre (dimensionnée pile sur `content_width`, voir `main.rs::watchlist_target_width`) —
/// invisible pour toutes les tuiles précédentes, dont le badge déborde dans l'espace laissé par
/// `TILE_GAP` avant la tuile suivante.
const BADGE_OVERFLOW: f32 = 7.0;

const CONTROL_BORDER: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 90);
const CONTROL_GLYPH: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 170);

/// Largeur de contenu nécessaire pour afficher `entry_count` entrées + les deux tuiles de
/// contrôle ("+"/"−"), SANS la marge de fenêtre (`egui::Frame::NONE.inner_margin`, ajoutée côté
/// appelant) — utilisée par `main.rs` pour dimensionner dynamiquement la fenêtre Suivi (retour
/// utilisateur 2026-09-02 : « je ne veux pas de fond, je veux que ça reste transparent, mais [...]
/// l'overlay n'a pas plus de taille s'il n'y a pas besoin » — une fenêtre plus large que son
/// contenu reste cliquable/bloquante sur toute sa zone même là où rien n'est visible, l'utilisateur
/// ne peut alors pas deviner où s'arrête l'overlay). Seule source de vérité pour `TILE_SIZE`/
/// `TILE_GAP` : `main.rs` ne les duplique pas.
pub fn content_width(entry_count: usize) -> f32 {
    let tile_count = entry_count as f32 + 2.0; // + les tuiles "+"/"−", toujours présentes
    tile_count * TILE_SIZE + (tile_count - 1.0).max(0.0) * TILE_GAP + BADGE_OVERFLOW
}

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

/// `--accent` — bordure ET titre du toast (`loot-alert-card`/`loot-alert-title`,
/// `loot-alert.component.css`), les deux réutilisent le même jeton quel que soit `reason`.
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x00, 0xd2, 0xff);
/// `--surface-raised` — fond du toast (`loot-alert-card`, dégradé à deux arrêts IDENTIQUES côté
/// web donc simple aplat ici).
const SURFACE_RAISED: egui::Color32 = egui::Color32::from_rgb(0x26, 0x26, 0x26);
/// `--text-bright` — nom de l'objet/monstre dans le toast (`loot-alert-name`).
const TEXT_BRIGHT: egui::Color32 = egui::Color32::from_rgb(0xf2, 0xf2, 0xf2);
/// `--tint-medium` — fond du bouton de fermeture au repos (`loot-alert-close`).
const TINT_MEDIUM: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 31);
/// `--tint-strong` — fond du bouton de fermeture survolé (`loot-alert-close:hover`).
const TINT_STRONG: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 46);

/// Largeur de la couche de confettis (`.confetti-layer`, `loot-alert.component.css`) — reprise
/// telle quelle du web (320px), centrée sur le même axe que la carte : `main.rs::
/// watchlist_target_width` s'en sert pour élargir la fenêtre Suivi le temps qu'un toast est
/// affiché, sans quoi les confettis les plus excentrés seraient rognés par le bord de fenêtre.
pub const TOAST_LAYER_WIDTH: f32 = 320.0;
/// Hauteur SUPPLÉMENTAIRE à réserver sous la bande de tuiles quand un toast est affiché (carte +
/// dépassement des confettis, voir `toast_card`) — ajoutée par `main.rs::watchlist_target_height`
/// à la hauteur de base, seulement tant qu'un toast est actif (voir `is_active`), pour ne pas
/// garder en permanence une zone de fenêtre cliquable/bloquante plus grande que nécessaire (même
/// principe que `watchlist_target_width` pour la largeur).
pub const TOAST_AREA_HEIGHT: f32 = 170.0;

const CARD_ROUNDING: f32 = 12.0;
const CARD_BORDER_WIDTH: f32 = 1.0;
const CARD_PAD_V: f32 = 10.0;
const CARD_PAD_LEFT: f32 = 16.0;
/// Plus large qu'à gauche : réserve la place du bouton de fermeture (coin haut-droit, voir
/// `close_rect`) — miroir de `padding: 10px 30px 10px 18px` (`loot-alert-card`).
const CARD_PAD_RIGHT: f32 = 28.0;
const CARD_ICON_GAP: f32 = 10.0;
/// Écart vertical entre le titre et le nom — miroir de l'empilement `flex-direction: column` sans
/// gap explicite de `.loot-alert-text` (léger espace naturel entre deux lignes de texte).
const CARD_TEXT_GAP: f32 = 2.0;
/// Écart entre le bas de la bande de tuiles et le haut de la carte.
const CARD_TOP_GAP: f32 = 12.0;
const CLOSE_BTN_SIZE: f32 = 18.0;
const CLOSE_BTN_ROUNDING: f32 = 4.0;
const CLOSE_BTN_MARGIN: f32 = 4.0;
/// Durée de l'animation d'entrée — miroir de `@keyframes loot-pop` (0.35s). `egui` n'exprimant pas
/// de transformation `scale`, l'entrée est ici un fondu + léger glissement vertical plutôt qu'un
/// vrai zoom (voir `toast_card`).
const POP_DURATION: f32 = 0.35;
/// Les confettis démarrent au-dessus du haut de la carte — miroir de `top: -10px` du calque de
/// confettis relatif au conteneur, dont le `margin-top: 40px` pousse la carte plus bas (ici
/// ramené à une valeur plus modeste, la fenêtre Suivi ayant nettement moins de hauteur disponible
/// qu'une page web).
const CONFETTI_TOP_OVERSHOOT: f32 = 18.0;
/// Distance de chute d'un confetti — réduite par rapport aux 220px web (`translateY(220px)`,
/// `@keyframes confetti-fall`) pour tenir dans `TOAST_AREA_HEIGHT`.
const CONFETTI_FALL_HEIGHT: f32 = 120.0;
/// Même taille que `.confetti-piece` (8x8px, `loot-alert.component.css`).
const CONFETTI_SIZE: f32 = 8.0;

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

/// Renvoie `true` quand l'utilisateur vient de fermer le toast affiché (clic sur la carte ou sur
/// sa croix, voir `toast_card`) — `main.rs::window_event` est seul à détenir un accès en écriture
/// à l'`ArcSwap` du toast, donc seul à pouvoir agir sur ce signal.
pub fn show(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    entries: &[WatchlistEntry],
    toast: Option<&WatchlistToast>,
) -> bool {
    // Bande de tuiles absente tant que le compte ne déclare aucune entrée suivie (voir
    // `main.rs::render`, commentaire de `OverlayKind::Watchlist`) — un ramassage à son activé
    // (`overlay_engine::profile`, INDÉPENDANT de la watchlist) doit pouvoir déclencher un toast
    // même dans ce cas, d'où la garde ici plutôt qu'en amont.
    if !entries.is_empty() {
        let mut style = (**ui.style()).clone();
        style_thin_scrollbar(&mut style);
        ui.set_style(style);

        egui::ScrollArea::horizontal()
            .id_salt("watchlist-strip")
            .auto_shrink([false, true])
            // Un peu plus que la seule hauteur des tuiles (58px) : donne à la barre de défilement
            // flottante une bande dégagée sous les icônes/badges plutôt que de la faire chevaucher
            // presque entièrement — retour utilisateur 2026-09-02 : « impossible de l'agripper ».
            .min_scrolled_height(TILE_SIZE + 14.0)
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

                    // Le badge de compteur (`count_badge`) déborde de `BADGE_OVERFLOW` px hors du coin
                    // bas-droit de sa tuile, peint directement via `ui.painter()` — donc INVISIBLE pour
                    // le calcul d'étendue du `ScrollArea` (basé sur l'espace ALLOUÉ par `horizontal`,
                    // pas sur ce qui est peint hors allocation). Sans cet espace réservé explicitement,
                    // le badge de la toute dernière tuile reste tronqué par le clip rect du `ScrollArea`
                    // une fois défilé au maximum (cas plafonné, `WATCHLIST_MAX_CEILING`) — même quand la
                    // fenêtre elle-même est assez large (voir `content_width`, qui couvre le cas non
                    // plafonné). Retour utilisateur 2026-09-02, capture d'écran à l'appui.
                    ui.add_space(BADGE_OVERFLOW);
                });
            });

        ui.add_space(6.0);
    }

    match toast.filter(|t| t.hide_at > std::time::Instant::now()) {
        Some(toast) => toast_card(
            ui,
            icons,
            catalog,
            remote_icons,
            remote_icon_textures,
            toast,
        ),
        None => false,
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
    // « La molette n'est pas prise en compte » (retour utilisateur 2026-09-02) : par défaut, egui
    // ne route la molette (verticale) vers une zone de défilement HORIZONTALE que si l'utilisateur
    // maintient Maj — `always_scroll_the_only_direction` lève cette exigence quand une seule
    // direction est activée (notre cas, `ScrollArea::horizontal()`), exactement le comportement
    // demandé (« que si on utilise la molette [...] ça applique le scroll »).
    style.always_scroll_the_only_direction = true;
    // Élimine l'espacement AUTOMATIQUE qu'`egui` insère entre deux éléments d'un même
    // `ui.horizontal` (`spacing.item_spacing`, par défaut ~8px) — sans ça, chaque `add_space`
    // explicite de `TILE_GAP` s'additionne à cet espacement caché, et `content_width` (dont
    // `main.rs` dépend pour dimensionner la fenêtre) sous-estime la largeur réellement occupée :
    // c'est la cause du décalage constaté par l'utilisateur (« il manque littéralement un
    // objet » — la dernière tuile débordait hors de la fenêtre, trop étroite de quelques dizaines
    // de pixels). `TILE_GAP` reste la SEULE source d'espacement horizontal après ce réglage.
    style.spacing.item_spacing.x = 0.0;
}

/// Multiplie le canal alpha de `color` par `factor` (`0.0..=1.0`) — sert au fondu d'entrée du
/// toast (voir `POP_DURATION`) et au fondu de sortie de chaque confetti (voir `toast_card`),
/// appliqué uniformément à tous les éléments peints (fond, bordure, texte, icône).
fn with_alpha(color: egui::Color32, factor: f32) -> egui::Color32 {
    let a = (color.a() as f32 * factor.clamp(0.0, 1.0)).round() as u8;
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}

/// Les 4 coins d'un carré de côté `2*half` centré sur `center`, tournés de `angle` radians —
/// `epaint` n'a pas de primitive « rectangle tourné », un confetti (`toast_card`) se peint donc
/// comme un polygone convexe à 4 points calculés à la main.
fn rotated_square(center: egui::Pos2, half: f32, angle: f32) -> [egui::Pos2; 4] {
    let (sin, cos) = angle.sin_cos();
    [
        egui::vec2(-half, -half),
        egui::vec2(half, -half),
        egui::vec2(half, half),
        egui::vec2(-half, half),
    ]
    .map(|c| center + egui::vec2(c.x * cos - c.y * sin, c.x * sin + c.y * cos))
}

/// Carte d'alerte de ramassage/décompte — miroir visuel de `loot-alert.component.html`/`.css` du
/// dépôt web : icône réelle, titre coloré (`ACCENT`, identique pour les deux `reason` — voir sa
/// doc), nom (+ quantité si > 1), bouton de fermeture, confettis tombants en fond. Contrairement à
/// `LootAlertComponent` (minuterie OU fermeture manuelle, réglage exclusif côté web via
/// `ProfileService.alertManualClose` — pas encore porté ici), les DEUX cohabitent déjà : minuterie
/// fixe (voir `WatchlistToast::hide_at`) ET fermeture au clic (carte entière ou croix), sans que
/// l'utilisateur ait à choisir. Renvoie `true` quand CE clic doit effacer le toast (voir la doc de
/// `show` — seul `main.rs::window_event` peut écrire dans l'`ArcSwap` correspondant).
fn toast_card(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    toast: &WatchlistToast,
) -> bool {
    let now = std::time::Instant::now();
    let elapsed = now
        .saturating_duration_since(toast.created_at)
        .as_secs_f32();

    // Entrée en fondu + léger glissement vertical — repli sur ce qu'un painter bas niveau sait
    // exprimer facilement, `egui` n'ayant pas de transformation `scale` de calque comme
    // `@keyframes loot-pop` (CSS).
    let pop_t = (elapsed / POP_DURATION).min(1.0);
    let pop_eased = 1.0 - (1.0 - pop_t) * (1.0 - pop_t); // ease-out quadratique
    let card_alpha = pop_eased;
    let slide = (1.0 - pop_eased) * 10.0;

    let title = match toast.reason {
        WatchlistToastReason::Countdown => "COMPTEUR ÉPUISÉ !",
        WatchlistToastReason::Loot { .. } => "OBJET OBTENU !",
    };
    let name_text = match toast.reason {
        WatchlistToastReason::Loot { quantity } if quantity > 1 => {
            format!("{} × {quantity}", toast.name)
        }
        _ => toast.name.clone(),
    };

    let painter = ui.painter();
    let title_galley =
        painter.layout_no_wrap(title.to_string(), egui::FontId::proportional(11.0), ACCENT);
    let name_galley =
        painter.layout_no_wrap(name_text, egui::FontId::proportional(14.0), TEXT_BRIGHT);

    let text_width = title_galley.size().x.max(name_galley.size().x);
    let text_height = title_galley.size().y + CARD_TEXT_GAP + name_galley.size().y;
    let content_height = ICON_SIZE.max(text_height);
    let card_width = CARD_PAD_LEFT + ICON_SIZE + CARD_ICON_GAP + text_width + CARD_PAD_RIGHT;
    let card_height = content_height + 2.0 * CARD_PAD_V;

    let center_x = ui.max_rect().center().x;
    let confetti_top = ui.cursor().top() + slide;
    let card_top = confetti_top + CONFETTI_TOP_OVERSHOOT + CARD_TOP_GAP;
    let card_rect = egui::Rect::from_min_size(
        egui::pos2(center_x - card_width / 2.0, card_top),
        egui::vec2(card_width, card_height),
    );

    // Zone de clic AVANT la peinture, même motif que `entry_tile`/`control_tile` — la carte
    // ENTIÈRE ferme le toast, pas seulement sa croix (demande utilisateur explicite).
    let card_response = ui.interact(
        card_rect,
        ui.id().with(("loot-alert-card", toast.name.as_str())),
        egui::Sense::click(),
    );

    // --- Confettis (peints AVANT la carte pour rester visuellement derrière elle) ---
    let layer_left = center_x - TOAST_LAYER_WIDTH / 2.0;
    for piece in &toast.confetti {
        if elapsed < piece.delay {
            continue; // pas encore démarré, miroir de `animation-delay`
        }
        let local = (elapsed - piece.delay) % piece.duration; // boucle, miroir de `infinite`
        let t = (local / piece.duration).clamp(0.0, 1.0);
        let eased = t * t; // approximation de la temporisation CSS `ease-in`
        let alpha = (1.0 - eased) * card_alpha; // rejoint le fondu d'entrée de la carte
        if alpha <= 0.01 {
            continue;
        }
        let x = layer_left + piece.left_frac * TOAST_LAYER_WIDTH;
        let y = confetti_top + eased * CONFETTI_FALL_HEIGHT;
        let points = rotated_square(
            egui::pos2(x, y),
            CONFETTI_SIZE / 2.0,
            piece.rotation * eased,
        );
        ui.painter().add(egui::Shape::convex_polygon(
            points.to_vec(),
            with_alpha(piece.color, alpha),
            egui::Stroke::NONE,
        ));
    }

    // --- Carte ---
    let painter = ui.painter();
    painter.rect_filled(
        card_rect.translate(egui::vec2(0.0, 6.0)),
        CARD_ROUNDING,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, (90.0 * card_alpha) as u8),
    ); // ombre portée approximée (`box-shadow`) en une seule passe plutôt qu'un flou multi-passes
    painter.rect_filled(
        card_rect,
        CARD_ROUNDING,
        with_alpha(SURFACE_RAISED, card_alpha),
    );
    painter.rect_stroke(
        card_rect,
        CARD_ROUNDING,
        egui::Stroke::new(CARD_BORDER_WIDTH, with_alpha(ACCENT, card_alpha)),
        egui::StrokeKind::Inside,
    );

    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(
            card_rect.left() + CARD_PAD_LEFT + ICON_SIZE / 2.0,
            card_rect.center().y,
        ),
        egui::vec2(ICON_SIZE, ICON_SIZE),
    );
    let icon_ref = match toast.kind {
        WatchlistKind::Item => catalog.find_item_icon(&toast.name, toast.catalog_id),
        WatchlistKind::Enemy => catalog.find_monster_icon(&toast.name, toast.catalog_id),
    };
    let remote_texture = icon_ref
        .as_ref()
        .and_then(|icon_ref| remote_icon_textures.resolve(ui.ctx(), remote_icons, icon_ref));
    let icon_tint = egui::Color32::from_white_alpha((255.0 * card_alpha) as u8);
    match &remote_texture {
        Some(texture) => egui::Image::new(texture)
            .tint(icon_tint)
            .paint_at(ui, icon_rect),
        None => egui::Image::new(icons.unknown_entity_texture())
            .tint(icon_tint)
            .paint_at(ui, icon_rect),
    }

    let text_left = icon_rect.right() + CARD_ICON_GAP;
    let text_top = card_rect.center().y - text_height / 2.0;
    let painter = ui.painter();
    painter.galley(
        egui::pos2(text_left, text_top),
        title_galley.clone(),
        with_alpha(ACCENT, card_alpha),
    );
    painter.galley(
        egui::pos2(text_left, text_top + title_galley.size().y + CARD_TEXT_GAP),
        name_galley.clone(),
        with_alpha(TEXT_BRIGHT, card_alpha),
    );

    // --- Bouton de fermeture (coin haut-droit, miroir de `loot-alert-close`) ---
    let close_rect = egui::Rect::from_min_size(
        egui::pos2(
            card_rect.right() - CLOSE_BTN_MARGIN - CLOSE_BTN_SIZE,
            card_rect.top() + CLOSE_BTN_MARGIN,
        ),
        egui::vec2(CLOSE_BTN_SIZE, CLOSE_BTN_SIZE),
    );
    let close_response = ui.interact(
        close_rect,
        ui.id().with(("loot-alert-close", toast.name.as_str())),
        egui::Sense::click(),
    );
    let (close_bg, close_glyph) = if close_response.hovered() {
        (TINT_STRONG, TEXT_BRIGHT)
    } else {
        (TINT_MEDIUM, TEXT_MUTED)
    };
    let painter = ui.painter();
    painter.rect_filled(
        close_rect,
        CLOSE_BTN_ROUNDING,
        with_alpha(close_bg, card_alpha),
    );
    painter.text(
        close_rect.center(),
        egui::Align2::CENTER_CENTER,
        "×",
        egui::FontId::proportional(12.0),
        with_alpha(close_glyph, card_alpha),
    );
    let close_response = close_response.on_hover_text("Fermer");

    card_response.clicked() || close_response.clicked()
}

/// Contour d'un rectangle à coins arrondis, comme suivi par un traceur — mêmes 4 arcs que
/// `rounded_gradient_rect`, mais en simples points (pas un maillage coloré) : sert à faire longer
/// une bordure en POINTILLÉS (`egui::Shape::dashed_line`, qui ne prend qu'une polyligne) le long
/// des coins arrondis plutôt que des coins droits — sans ça, `dashed_line` trace tout droit d'un
/// coin à l'autre et déborde visiblement du remplissage arrondi en dessous (voir `control_tile`).
fn rounded_rect_outline(rect: egui::Rect, radius: f32) -> Vec<egui::Pos2> {
    const ARC_SEGMENTS: usize = 6;
    let radius = radius
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0)
        .max(0.0);
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
    let mut points = Vec::with_capacity(corners.len() * (ARC_SEGMENTS + 1) + 1);
    for (center, start_angle) in corners {
        for i in 0..=ARC_SEGMENTS {
            let angle = start_angle + quarter * (i as f32 / ARC_SEGMENTS as f32);
            points.push(center + radius * egui::vec2(angle.cos(), angle.sin()));
        }
    }
    if let Some(&first) = points.first() {
        points.push(first); // referme le contour, comme les 4 coins droits d'avant
    }
    points
}

/// Tuile "+"/"−" du bandeau web — bordure en pointillés (`egui::Shape::dashed_line`, pas de
/// primitive "rectangle en pointillés" dans `epaint`) qui longe le contour ARRONDI de la tuile
/// (voir `rounded_rect_outline`) — retour utilisateur 2026-09-02 : les deux tuiles de contrôle
/// étaient les deux SEULS éléments de la bande à afficher des coins droits, incohérent avec le
/// reste (`TILE_ROUNDING` partout ailleurs, y compris le fond plein de CETTE tuile). Signale
/// visuellement qu'il s'agit d'une action, pas d'une entrée suivie, cohérent avec la charte
/// `.kpi-add` du web.
fn control_tile(ui: &mut egui::Ui, glyph: &str, tooltip: &str) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(TILE_SIZE, TILE_SIZE), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, TILE_ROUNDING, PANEL_BG);

    painter.extend(egui::Shape::dashed_line(
        &rounded_rect_outline(rect, TILE_ROUNDING),
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
    let badge_max = tile_rect.right_bottom() + egui::vec2(BADGE_OVERFLOW, BADGE_OVERFLOW);
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
