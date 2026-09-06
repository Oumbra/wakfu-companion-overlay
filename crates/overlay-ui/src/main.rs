//! `overlay-ui` (docs/plan-architecture.md §6 et §9, lot L2) — overlay natif : fenêtres
//! transparentes Windows (DirectComposition, voir spike S1) rendant deux panneaux réels alimentés
//! par `overlay-ingest` + `overlay-engine` sur un vrai `wakfu.log` : dégâts du combat en cours,
//! récap de session. Le fenêtrage/rendu reprend telle quelle l'approche validée par S1
//! (`spikes/s1-window-windows/README.md`) — mêmes bugs déjà corrigés (patch `wgpu-hal`, alpha
//! prémultiplié, clamp de redimensionnement), pas réinventée ici.
//!
//! **Multi-fenêtre** (2026-09-01, retour utilisateur en test réel multi-compte) : `wakfu.log` est
//! partagé et entrelacé par toutes les instances du client (contrairement à Dofus, un fichier par
//! instance) — un seul `Engine`/thread suffit à le suivre (voir `spawn_engine_thread`), mais il
//! faut désormais **une fenêtre overlay par fenêtre de jeu trouvée**, chacune affichant le combat
//! de SON personnage (`overlay_engine::SessionSnapshot::fight_for_character`, le nom venant du
//! titre de fenêtre `"<Personnage> - WAKFU"`). Les fenêtres sont créées/détruites dynamiquement à
//! chaque tick (`App::sync_windows`) au gré des clients qui se lancent/se ferment — voir
//! `OverlayWindow` et le commentaire sur `Arc<Window>` (remplace la fuite `'static` d'origine,
//! plus tenable dès que des fenêtres doivent pouvoir être détruites).
//!
//! Volontairement incomplet par rapport à §9 du plan : pas encore d'État de synchro (dépend de la
//! synchro serveur, L5). Pas de thème configurable ni de disposition repositionnable/persistée par
//! écran — décision du mainteneur (§9 du plan, 2026-09-02) : un overlay n'est pas un site, palette
//! fixe et ancrage automatique (`App::anchor_position`) seuls assumés. Le récap de session reste
//! également **global** (identique sur toutes les fenêtres, pas ventilé par personnage —
//! limitation connue, voir le plan) : ce sont les deux panneaux atteignables avec `overlay-engine`
//! tel qu'il existe aujourd'hui.
//!
//! **Alertes de drop version « ramassage » (2026-09-02, §9 du plan)** : `overlay_engine::profile`
//! lit désormais `data.profile.soundItems` (`GET /api/v1/settings`) — indépendant de la watchlist,
//! n'importe quel objet ramassé avec son son activé au compte déclenche toast + son
//! (`alert_sound::play_loot_alert`), pas seulement les entrées suivies. Seul le cas
//! `reason: 'countdown'` était câblé jusqu'ici (voir `spawn_engine_thread`).

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use arc_swap::ArcSwap;
use egui_wgpu::wgpu;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use overlay_engine::{CatalogIndex, DungeonIndex, SessionSnapshot, WatchlistEntry};
use overlay_ingest::discovery;
use overlay_ui::engine_thread::{spawn_engine_thread, EngineCommand, EngineHandles, SyncCommand};
use overlay_ui::frame::{render, GpuState};
use overlay_ui::game_window::{GameRect, GameWindowTracker};
use overlay_ui::logging;
use overlay_ui::panels;
use overlay_ui::panels::combat::CombatSide;
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::panels::watchlist::WatchlistToast;
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content;
use overlay_ui::render_content::{AuthCommand, AuthStatus, OverlayKind, RenderContent, UserEvent};
use overlay_ui::ui_icons::UiIcons;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE,
    HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

#[cfg(target_os = "windows")]
use winit::platform::windows::WindowAttributesExtWindows;

const HOTKEY_LABEL: &str = "Ctrl+Alt+W";
/// Ctrl+Alt+R plutôt que F5 (suggestion initiale de l'utilisateur, 2026-09-02) : F5 est un
/// raccourci GLOBAL (`GlobalHotKeyManager`, jamais limité à une fenêtre précise malgré la demande
/// « quand on est focus sur une fenêtre de jeu ») — le voler à Wakfu (raccourcis de sort/action
/// fréquents sur les touches de fonction) ou à n'importe quelle autre appli au premier plan serait
/// activement nuisible. Même préfixe que `HOTKEY_LABEL` : cohérent, déjà éprouvé sans collision
/// connue avec le jeu.
const REFRESH_HOTKEY_LABEL: &str = "Ctrl+Alt+R";
/// Raccourci global de sortie (retour utilisateur 2026-09-02) : les fenêtres overlay portent
/// `WS_EX_NOACTIVATE` (voir `apply_extended_styles`, jamais désactivé même en mode interactif —
/// nécessaire pour ne jamais voler le focus au jeu) donc ne reçoivent JAMAIS `WindowEvent::
/// KeyboardInput`, quel que soit le mode : Échap (voir `window_event`) ne peut en pratique jamais
/// se déclencher, malgré ce qu'annonçait la bannière de démarrage. L'utilisateur devait donc
/// systématiquement faire un Ctrl+C dans le terminal (` STATUS_CONTROL_C_EXIT` en sortie — normal
/// dans ce cas, pas un plantage, mais peu clair). Même mécanisme que `HOTKEY_LABEL`/
/// `REFRESH_HOTKEY_LABEL` (hotkey GLOBAL, fonctionne sans focus sur aucune fenêtre précise) pour
/// vraiment permettre ce que la bannière annonce.
const QUIT_HOTKEY_LABEL: &str = "Ctrl+Alt+Q";
/// Déconnexion volontaire du compte (lot L4, §7.2/§14 point 3 du plan) — jusqu'ici, révoquer une
/// session native depuis l'overlay exigeait d'aller effacer le jeton à la main sur disque/dans le
/// trousseau (aucun moyen depuis l'overlay lui-même). Même famille de raccourci GLOBAL que les
/// trois précédents ; ne fait rien de visible en mode invité (aucun compte lié) — voir
/// `App::disconnect_account`.
const DISCONNECT_HOTKEY_LABEL: &str = "Ctrl+Alt+D";

/// Raccourci global pour "Détails" (bouton lien externe, `panels::combat::bottom_toolbar`) — même
/// action qu'un clic (`open::that(overlay_sync::client::base_url())`, voir `App::open_details`).
/// Retour utilisateur explicite 2026-09-06 (« à l'image de ce qu'il y a dans le jeu [...] rajoute
/// les raccourcis [...] pour le détail [...] Ctrl+Shift+D ») : modificateur CTRL+SHIFT (pas
/// CTRL+ALT comme les quatre raccourcis précédents), combinaisons données par l'utilisateur
/// lui-même pour les cinq raccourcis de ce groupe — reflétées entre parenthèses dans les tooltips
/// correspondants (voir `panels::combat::bottom_toolbar`/`paint_side_switch`,
/// `panels::watchlist::control_button_row`), à l'image du jeu.
const DETAILS_HOTKEY_LABEL: &str = "Ctrl+Shift+D";
/// Raccourci global pour "Options" (`panels::combat::bottom_toolbar`) — n'ouvre encore aucun
/// panneau, comme le clic sur le bouton lui-même (voir sa doc) : réservé à une future page de
/// réglages, juste enregistré/journalisé pour l'instant (voir `about_to_wait`).
const OPTIONS_HOTKEY_LABEL: &str = "Ctrl+Shift+O";
/// Raccourci global pour "Ajouter" (`panels::watchlist::control_button_row`) — reste INERTE comme
/// le bouton lui-même (voir doc de module de `watchlist` : aucune sélection/formulaire câblés côté
/// overlay pour cette itération), juste enregistré/journalisé pour l'instant.
const WATCHLIST_ADD_HOTKEY_LABEL: &str = "Ctrl+Shift+A";
/// Raccourci global pour "Supprimer" — même remarque que `WATCHLIST_ADD_HOTKEY_LABEL`.
const WATCHLIST_REMOVE_HOTKEY_LABEL: &str = "Ctrl+Shift+S";
/// Raccourci global pour basculer Alliés/Ennemis (`panels::combat::paint_side_switch`) — EN MODE
/// TOGGLE (retour utilisateur explicite : « ça inverse la sélection [...] si actuellement c'est
/// sélectionné allié [...] ça passe en ennemi et inversement ») plutôt que deux raccourcis séparés
/// un par camp : voir `CombatSide::toggled` et `App::toggle_combat_side`, appliqué à CHAQUE fenêtre
/// Combat actuellement ouverte — même portée globale que les hotkeys existants, pas seulement celle
/// au premier plan.
const SIDE_HOTKEY_LABEL: &str = "Ctrl+Shift+E";
/// Voir `App::sync_topmost`.
const TOPMOST_REASSERT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);
/// Délai de grâce avant repli en `HWND_NOTOPMOST` — voir `OverlayWindow::pending_demote_since` et
/// `App::sync_topmost`. Assez court pour qu'un changement de fenêtre volontaire et soutenu
/// reste respecté rapidement (ne pas recouvrir durablement une autre appli, retour utilisateur
/// 2026-09-01), assez long pour absorber un aléa de timing d'un seul tick (~50 ms) entre les deux
/// overlays d'un même personnage.
const TOPMOST_DEMOTE_GRACE: std::time::Duration = std::time::Duration::from_millis(1500);
// Largeur élargie 360 -> 420 (2026-09-01) pour laisser la place au portrait de classe (40px,
// voir portraits.rs) sans écraser le nom/les dégâts — réglage fin de la mise en page toujours à
// faire. Hauteur élargie de `render_content::COMBAT_TOP_MARGIN` (2026-09-06, retour utilisateur :
// tooltips du switch Alliés/Ennemis affichées en dessous faute de place au-dessus) : voir sa doc,
// seule façon de loger cette marge sans compresser le reste du panneau.
const WINDOW_SIZE: (f64, f64) = (420.0, 480.0 + render_content::COMBAT_TOP_MARGIN as f64);
/// Hauteur de la fenêtre du panneau Suivi (bande horizontale de tuiles, voir
/// `panels::watchlist`) — la LARGEUR, elle, suit dynamiquement le CONTENU (voir
/// `watchlist_target_width`), pas une constante fixe. 84 -> 116 px (2026-09-02) pour réserver
/// l'espace du toast d'alerte SOUS la bande de tuiles (voir `panels::watchlist::show`) — sans
/// agrandir la fenêtre à la volée à l'apparition d'un toast, ce qui aurait fait bouger la bande de
/// tuiles elle-même dont l'ancrage vient d'être mis au point avec l'utilisateur. 116 -> 132 px
/// (même jour, retour utilisateur : la barre de défilement flottante était trop souvent tassée
/// contre les icônes/badges pour être agrippée) pour la marge supplémentaire réservée par
/// `panels::watchlist::show` (`ScrollArea::min_scrolled_height`).
const WATCHLIST_HEIGHT: f64 = 132.0;
/// Même marge que `egui::Frame::NONE.inner_margin(6)` posée par `render` (6px de chaque côté) —
/// à additionner à `panels::watchlist::content_width` pour obtenir la largeur de FENÊTRE
/// nécessaire, pas seulement celle du contenu peint dedans.
const WATCHLIST_INNER_MARGIN: f64 = 12.0;
/// Plafond de largeur — fraction de la largeur de la fenêtre de jeu, jamais dépassée même avec
/// beaucoup d'entrées suivies (la bande de tuiles défile horizontalement au-delà, voir
/// `panels::watchlist::show`). Bornée par prudence plutôt que par mesure précise de l'espace
/// réellement libre entre les groupes de boutons du jeu (variable selon la résolution/l'UI du
/// client) — à ajuster si ça chevauche quand même l'interface du jeu sur une configuration donnée.
const WATCHLIST_WIDTH_FRACTION: f64 = 0.5;
const WATCHLIST_MAX_CEILING: f64 = 1000.0;

/// Largeur RÉELLEMENT nécessaire à l'affichage actuel — retour utilisateur 2026-09-02 : « je ne
/// veux pas de fond, je veux que ça reste transparent, mais [...] l'overlay n'a pas plus de taille
/// s'il n'y a pas besoin » — une fenêtre plus large que son contenu reste cliquable/bloquante sur
/// toute sa zone même là où rien n'est peint (pas de test de transparence par pixel côté Win32),
/// l'utilisateur ne peut alors pas deviner où s'arrête l'overlay. Toujours le MINIMUM entre ce que
/// le contenu demande (`content_width`, croît avec le nombre d'entrées, et avec `toast_active` —
/// voir `panels::watchlist::TOAST_LAYER_WIDTH`) et le plafond (`WATCHLIST_WIDTH_FRACTION` de la
/// fenêtre de jeu, `WATCHLIST_MAX_CEILING`) — jamais l'inverse : avec peu d'entrées et sans toast,
/// la fenêtre reste étroite même si le plafond est large.
fn watchlist_target_width(entry_count: usize, toast_active: bool, game_width_px: i32) -> f64 {
    let ceiling = (game_width_px as f64 * WATCHLIST_WIDTH_FRACTION).min(WATCHLIST_MAX_CEILING);
    let tiles = panels::watchlist::content_width(entry_count) as f64;
    // La couche de confettis est centrée sur le MÊME axe que la bande de tuiles (voir
    // `panels::watchlist::toast_card`) : sans cette largeur minimale pendant qu'un toast est
    // affiché, ses confettis les plus excentrés seraient rognés par le bord de la fenêtre.
    let toast = if toast_active {
        panels::watchlist::TOAST_LAYER_WIDTH as f64
    } else {
        0.0
    };
    let content = tiles.max(toast) + WATCHLIST_INNER_MARGIN;
    content.min(ceiling).max(WATCHLIST_INNER_MARGIN)
}

/// Hauteur nécessaire à l'affichage actuel — même principe que `watchlist_target_width`
/// (dimensionnée sur le CONTENU, pas une constante toujours large) : `WATCHLIST_HEIGHT` seule
/// tant qu'aucun toast n'est affiché, plus `panels::watchlist::TOAST_AREA_HEIGHT` pendant qu'un
/// toast (carte + confettis) est actif. L'ancrage de la fenêtre Suivi (`App::anchor_position`) ne
/// dépend que de sa LARGEUR, jamais de sa hauteur — grandir vers le bas ne déplace donc jamais la
/// bande de tuiles déjà positionnée.
fn watchlist_target_height(toast_active: bool) -> f64 {
    WATCHLIST_HEIGHT
        + if toast_active {
            panels::watchlist::TOAST_AREA_HEIGHT as f64
        } else {
            0.0
        }
}
/// Marge, en pixels physiques, entre le bord gauche visible de la fenêtre de jeu et le bord
/// gauche de l'overlay Combat.
///
/// **Refonte 2026-09-04** (retour utilisateur, capture d'écran à l'appui) : la valeur initiale
/// (12 px, « collé à quelques pixels près ») laissait un vide visible entre le bord de la fenêtre
/// de jeu et le cadre — l'utilisateur veut désormais que l'overlay se fonde dans le jeu (« comme
/// si l'overlay faisait partie du jeu »), donc un ancrage réellement à zéro. Voir aussi
/// `render_content::paint_content` : la marge interne du panneau (`Frame::inner_margin`) doit être
/// nulle elle aussi, sinon un vide subsiste malgré cette valeur à 0.
const GAME_EDGE_MARGIN_PX: i32 = 0;
/// Même principe que `GAME_EDGE_MARGIN_PX`, mais pour le bord HAUT — ancrage de l'overlay Suivi
/// (demande utilisateur explicite 2026-09-01 : « collé en haut de la fenêtre de jeu au centre »,
/// « le même espacement » que les boutons d'interface du jeu — menu/Boutique en haut-gauche, icônes
/// en haut-droite).
///
/// **Historique de mise au point (2026-09-01, deux allers-retours avec capture d'écran)** : la
/// valeur initiale (12 px, copiée de `GAME_EDGE_MARGIN_PX`) était bien trop petite — l'overlay
/// apparaissait quasiment collé au très haut de la fenêtre de jeu. Diagnostic confirmé par le
/// `println!` de `create_overlay_window` (`rect.top=0 client_top=0, écart 0`) : **le client Wakfu
/// dessine lui-même sa fausse barre de titre DANS sa zone cliente** (voir `GameRect::client_top`,
/// fenêtre non décorée côté Win32) — il n'y avait donc aucune vraie barre de titre Windows à
/// exclure, `client_top` valait déjà `top`. Toute la hauteur à compenser est celle de cette fausse
/// barre de titre dessinée par le jeu, pas une histoire de coordonnées Win32 : 28 px s'est avéré
/// visuellement correct (confirmé par une deuxième capture d'écran, alignement quasi identique aux
/// boutons Menu/Boutique du jeu).
const GAME_TOP_MARGIN_PX: i32 = 28;

// `OverlayKind`/`UserEvent`/`AuthStatus`/`AuthCommand`/`CLICK_THROUGH_OPACITY` ont migré vers
// `overlay_ui::render_content` (2026-09-03, §17.1 du plan) — voir leur doc là-bas, importés en
// tête de ce fichier. Rien ne change à leur usage ici, seul leur PROPRIÉTAIRE change : la
// construction d'UI (`render_content::build_ui`) doit pouvoir être appelée par un futur harnais
// de rendu offscreen (`overlay-testkit`) sans dépendre du binaire `overlay-ui`, qui reste
// Windows-only (import inconditionnel de `windows::`, voir plus haut).

// `EngineCommand`/`SyncCommand`/`EngineHandles`/`spawn_engine_thread` ont migré vers
// `overlay_ui::engine_thread` (§17.2 du plan, Niveau 2) — voir sa doc, importés en tête de ce
// fichier. `GpuState`/`render` ont migré vers `overlay_ui::frame` (même raison : rien dans ces
// deux-là ne dépend de Windows, seul `init_gpu` ci-dessous reste spécifique).

/// Une fenêtre overlay, ancrée sur UNE fenêtre de jeu précise — un personnage, un combat. Créée et
/// détruite dynamiquement par `App::sync_windows` au gré des clients qui se lancent/se ferment.
struct OverlayWindow {
    /// `Arc`, pas `&'static` : contrairement à la version mono-fenêtre d'origine (`Box::leak`),
    /// une fenêtre doit pouvoir être réellement détruite quand son client de jeu ferme — l'`Arc`
    /// est le motif standard wgpu+winit pour des fenêtres à durée de vie dynamique
    /// (`wgpu::Instance::create_surface` accepte `Arc<Window>`, qui garde la fenêtre en vie aussi
    /// longtemps que la `Surface`, donnant un `Surface<'static>` sans fuite).
    window: Arc<Window>,
    gpu: GpuState,
    /// Zone affichée par CETTE fenêtre (voir `OverlayKind`) — deux `OverlayWindow` distinctes
    /// partagent le même `game_hwnd`/`character_name`, une par zone.
    kind: OverlayKind,
    /// Chargée par fenêtre (chacune a son propre `egui::Context`) — léger surcoût de
    /// décodage/upload par fenêtre, négligeable pour le nombre de comptes réaliste. Utilisée par
    /// les deux zones (portrait de classe en Combat, icône générique en Suivi tant qu'aucune
    /// icône d'objet/monstre n'est câblée — voir `panels::watchlist`).
    portraits: PortraitAtlas,
    /// Cadre décoratif "totem" du panneau Combat (`panels::combat_frame`) — même remarque que
    /// `portraits` (une texture par fenêtre, coût négligeable, 6 templates au lieu de 36 portraits).
    combat_frame: CombatFrame,
    /// Icônes du switch Alliés/Ennemis + portrait générique d'ennemi — même remarque que
    /// `portraits` (une texture par fenêtre, coût négligeable).
    icons: UiIcons,
    /// Cache PAR FENÊTRE des icônes réelles d'objets/monstres déjà uploadées (voir
    /// `remote_icons::RemoteIconTextures`) — sans objet pour une fenêtre `Combat`.
    remote_icon_textures: RemoteIconTextures,
    /// Camp affiché dans la liste verticale du panneau Combat (voir `panels::combat::CombatSide`)
    /// — état PAR FENÊTRE (donc par personnage), pas global : `Allies` par défaut à chaque
    /// création de fenêtre (demande utilisateur explicite). Sans objet pour une fenêtre `Suivi`.
    combat_side: CombatSide,
    game_hwnd: HWND,
    /// Dernier rectangle connu de la fenêtre de jeu (mis à jour par `sync_windows`/`reposition`,
    /// voir `App::sync_windows`) — réutilisé par `RedrawRequested` pour le plafond de largeur
    /// dynamique du Suivi (`watchlist_target_width`) sans re-scanner les fenêtres à chaque frame.
    game_rect: GameRect,
    character_name: String,
    /// Dernière position appliquée — évite de rappeler `set_outer_position` à chaque tick (50 ms)
    /// quand la fenêtre de jeu n'a pas bougé.
    last_position: Option<PhysicalPosition<i32>>,
    /// Dernière largeur demandée pour une fenêtre `Suivi` (voir `watchlist_target_width`) — évite
    /// de rappeler `request_inner_size` à chaque frame quand le nombre d'entrées n'a pas changé.
    /// Sans objet pour une fenêtre `Combat` (toujours `None`).
    last_watchlist_width: Option<f64>,
    /// Même principe que `last_watchlist_width`, pour la HAUTEUR (voir `watchlist_target_height`)
    /// — change uniquement à l'apparition/disparition d'un toast, jamais avec le nombre d'entrées.
    last_watchlist_height: Option<f64>,
    /// État `HWND_TOPMOST`/`HWND_NOTOPMOST` déjà appliqué — évite un `SetWindowPos` par tick pour
    /// rien (voir `App::sync_topmost`).
    is_topmost: bool,
    /// Dernière réaffirmation PÉRIODIQUE de `HWND_TOPMOST` (voir `App::sync_topmost` et
    /// `TOPMOST_REASSERT_INTERVAL`) — distincte d'un changement d'état détecté (`is_topmost`),
    /// qui reste réaffirmé immédiatement quel que soit ce champ.
    last_topmost_reassert: Option<std::time::Instant>,
    /// Instant depuis lequel `relevant` est retombé à `false` en continu, tant que l'overlay est
    /// encore `HWND_TOPMOST` — `None` tant qu'il est retombé à `false` pour la première fois OU
    /// que l'overlay est déjà en retrait. Voir `TOPMOST_DEMOTE_GRACE` dans `App::sync_topmost` :
    /// retour utilisateur 2026-09-02 (vidéo à l'appui), l'overlay Combat disparaissait « un coup
    /// sur deux » en changeant de fenêtre alors que le Suivi du même personnage restait visible au
    /// même instant — la démotion en `HWND_NOTOPMOST` était jusqu'ici IMMÉDIATE dès que
    /// `GetForegroundWindow()` cessait de désigner la fenêtre de jeu ne serait-ce qu'un seul tick
    /// (~50 ms), ce qui rend le résultat sensible au moindre aléa de timing entre deux fenêtres
    /// overlay qui tournent pourtant sur le MÊME code (`sync_topmost` itère les deux de façon
    /// identique, mais Windows peut livrer les changements de premier plan avec un tick d'écart
    /// entre deux `SetWindowPos` consécutifs). Un délai de grâce absorbe ces aléas sans revenir sur
    /// le principe du 2026-09-01 (ne pas recouvrir durablement une autre appli) : la réaffirmation
    /// en topmost, elle, reste immédiate (voir plus bas) — seule la démotion est temporisée.
    pending_demote_since: Option<std::time::Instant>,
    /// Prochain redessin déjà planifié par une frame précédente qui a demandé un délai (retour
    /// egui `ViewportOutput::repaint_delay` — ex. le délai d'apparition d'une tooltip au survol
    /// d'un portrait, voir `panels::combat`) — sans ce champ, ce délai n'avait AUCUN moyen d'être
    /// honoré : cette architecture n'a pas de boucle 60 Hz (§6.1 du plan), le rendu ne se
    /// redéclenche que sur un `WindowEvent` (dont `CursorMoved`, mais UNE seule fois à l'entrée du
    /// survol) ou un `UserEvent`, jamais après un délai pur — la tooltip ne s'affichait donc
    /// qu'au hasard d'un autre redessin (retour utilisateur 2026-09-01 : « c'est bizarre, ça
    /// s'est affiché, là ça s'est caché »). Consommé par `App::about_to_wait`, qui redessine et
    /// vide ce champ une fois l'échéance atteinte.
    next_redraw_at: Option<std::time::Instant>,
}

struct App {
    windows: HashMap<WindowId, OverlayWindow>,
    #[allow(dead_code)] // jamais relu : sa seule raison d'être est de rester en vie (voir S1)
    hotkey_manager: GlobalHotKeyManager,
    hotkey_events: &'static global_hotkey::GlobalHotKeyEventReceiver,
    /// `HotKey::id()` de `HOTKEY_LABEL`/`REFRESH_HOTKEY_LABEL` — un seul `GlobalHotKeyEventReceiver`
    /// partagé pour tous les raccourcis enregistrés (API de `global_hotkey`), distingué par cet id
    /// à la réception (voir `about_to_wait`).
    toggle_hotkey_id: u32,
    refresh_hotkey_id: u32,
    quit_hotkey_id: u32,
    disconnect_hotkey_id: u32,
    /// Voir `DETAILS_HOTKEY_LABEL`/`OPTIONS_HOTKEY_LABEL`/`WATCHLIST_ADD_HOTKEY_LABEL`/
    /// `WATCHLIST_REMOVE_HOTKEY_LABEL`/`SIDE_HOTKEY_LABEL` — même mécanisme d'id que les quatre
    /// raccourcis ci-dessus, groupe distinct ajouté 2026-09-06 (design system tooltip, raccourcis
    /// affichés entre parenthèses à l'image du jeu).
    details_hotkey_id: u32,
    options_hotkey_id: u32,
    watchlist_add_hotkey_id: u32,
    watchlist_remove_hotkey_id: u32,
    side_hotkey_id: u32,
    interactive: bool,
    snapshot: Arc<ArcSwap<SessionSnapshot>>,
    /// Publié par le thread Engine à chaque lot ingéré (et une fois de plus dès la réception des
    /// entrées suivies par le thread Auth, voir `spawn_engine_thread`) — état COMPLET du Suivi
    /// (définitions + compteurs), déjà fusionné avec les compteurs locaux persistés (voir
    /// `overlay_engine::watchlist`). Global comme `snapshot`, pas par fenêtre : le suivi est un
    /// suivi de compte, pas d'un personnage précis.
    watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
    /// Publié par le thread Engine à chaque décompte de suivi qui vient d'atteindre 0 (voir
    /// `overlay_engine::WatchlistAlert`, §9 du plan « Alertes de drop ») — `None` initialement et
    /// après expiration (voir `WatchlistToast::hide_at`, comparé à `Instant::now()` au rendu).
    watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
    /// Publié par le thread Catalogue (`spawn_catalog_thread`) — d'abord depuis le cache disque
    /// (rapide, hors-ligne), puis réécrasé si le réseau confirme un contenu différent (voir
    /// `overlay_sync::catalog_cache`). Vide (`CatalogIndex::default`) tant que rien n'a encore pu
    /// être chargé — les tuiles du panneau Suivi retombent alors sur l'icône générique.
    catalog: Arc<ArcSwap<CatalogIndex>>,
    /// `true` quand `catalog` provient du repli hors-ligne EMBARQUÉ (`spawn_catalog_thread`,
    /// §7.4 du plan) — ni cache disque ni réseau disponibles au démarrage. Pilote le petit
    /// indicateur « catalogue daté » de la zone Combat (voir `render`) : ce n'était encore qu'un
    /// `tracing::warn!` invisible en jeu avant ce lot (retour utilisateur : rien ne signale à
    /// l'écran que les icônes/classements affichés peuvent dater du dernier build embarqué).
    catalog_stale: Arc<AtomicBool>,
    /// Un seul thread/état de téléchargement d'icônes PARTAGÉ par toutes les fenêtres (voir
    /// `remote_icons::RemoteIconStore`) — chaque fenêtre garde son propre cache de textures déjà
    /// uploadées (`OverlayWindow::remote_icon_textures`), mais ne retélécharge jamais une icône
    /// qu'une AUTRE fenêtre a déjà demandée.
    remote_icons: RemoteIconStore,
    /// Publié par le thread Auth (voir `spawn_auth_thread`) — piloté l'affichage de l'icône de
    /// relance d'appairage (`render`).
    auth_status: Arc<ArcSwap<AuthStatus>>,
    /// Signale au thread Auth une commande (`AuthCommand`) : `Retry` sur clic sur l'icône de
    /// relance (visible uniquement quand `auth_status` vaut `Disconnected` — voir `render`),
    /// `Disconnect` sur `DISCONNECT_HOTKEY_LABEL` (voir `disconnect_account`).
    auth_command_tx: mpsc::Sender<AuthCommand>,
    /// Voir la doc de `AppState::settings_tx` et `force_refresh`.
    settings_tx: mpsc::Sender<EngineCommand>,
    log_path: PathBuf,
    game_window: GameWindowTracker,
    /// N'affiche la bannière de démarrage qu'une fois — `resumed()` peut être rappelé par winit
    /// (perte/reprise de focus applicatif), `sync_windows` doit rester idempotent mais pas cette
    /// bannière.
    banner_printed: bool,
}

/// Regroupe les paramètres de construction d'`App` au-delà de `log_path` — sinon
/// `too_many_arguments` (clippy), le nombre d'états partagés publiés par les threads de fond
/// ayant crû au fil des lots (roster/watchlist L4, toast + catalogue L2/L3). Même motif que
/// `RenderContent` pour `render()`.
struct AppState {
    log_path: PathBuf,
    snapshot: Arc<ArcSwap<SessionSnapshot>>,
    watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
    watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
    catalog: Arc<ArcSwap<CatalogIndex>>,
    catalog_stale: Arc<AtomicBool>,
    remote_icons: RemoteIconStore,
    auth_status: Arc<ArcSwap<AuthStatus>>,
    auth_command_tx: mpsc::Sender<AuthCommand>,
    /// Conservé (pas seulement transmis au thread Auth) pour permettre à `force_refresh` de
    /// redemander les réglages de compte à la volée — voir sa doc.
    settings_tx: mpsc::Sender<EngineCommand>,
}

impl App {
    fn new(state: AppState) -> Self {
        let AppState {
            log_path,
            snapshot,
            watchlist,
            watchlist_toast,
            catalog,
            catalog_stale,
            remote_icons,
            auth_status,
            auth_command_tx,
            settings_tx,
        } = state;

        let hotkey_manager = GlobalHotKeyManager::new().expect("création GlobalHotKeyManager");
        let toggle_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW);
        let refresh_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyR);
        let quit_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyQ);
        let disconnect_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyD);
        // Ctrl+Shift (pas Ctrl+Alt) — voir la doc de `DETAILS_HOTKEY_LABEL` et consorts.
        let details_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyD);
        let options_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyO);
        let watchlist_add_hotkey =
            HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyA);
        let watchlist_remove_hotkey =
            HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyS);
        let side_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyE);
        hotkey_manager
            .register(toggle_hotkey)
            .expect("enregistrement du hotkey global");
        hotkey_manager
            .register(refresh_hotkey)
            .expect("enregistrement du hotkey de rafraîchissement");
        hotkey_manager
            .register(quit_hotkey)
            .expect("enregistrement du hotkey de sortie");
        hotkey_manager
            .register(disconnect_hotkey)
            .expect("enregistrement du hotkey de déconnexion");
        hotkey_manager
            .register(details_hotkey)
            .expect("enregistrement du hotkey Détails");
        hotkey_manager
            .register(options_hotkey)
            .expect("enregistrement du hotkey Options");
        hotkey_manager
            .register(watchlist_add_hotkey)
            .expect("enregistrement du hotkey Ajouter");
        hotkey_manager
            .register(watchlist_remove_hotkey)
            .expect("enregistrement du hotkey Supprimer");
        hotkey_manager
            .register(side_hotkey)
            .expect("enregistrement du hotkey Alliés/Ennemis");

        Self {
            windows: HashMap::new(),
            hotkey_manager,
            hotkey_events: GlobalHotKeyEvent::receiver(),
            toggle_hotkey_id: toggle_hotkey.id(),
            refresh_hotkey_id: refresh_hotkey.id(),
            quit_hotkey_id: quit_hotkey.id(),
            disconnect_hotkey_id: disconnect_hotkey.id(),
            details_hotkey_id: details_hotkey.id(),
            options_hotkey_id: options_hotkey.id(),
            watchlist_add_hotkey_id: watchlist_add_hotkey.id(),
            watchlist_remove_hotkey_id: watchlist_remove_hotkey.id(),
            side_hotkey_id: side_hotkey.id(),
            interactive: true,
            snapshot,
            watchlist,
            watchlist_toast,
            catalog,
            catalog_stale,
            remote_icons,
            auth_status,
            auth_command_tx,
            settings_tx,
            log_path,
            game_window: GameWindowTracker::new(),
            banner_printed: false,
        }
    }

    /// Scanne les fenêtres de jeu actuellement ouvertes et fait converger `self.windows` vers cet
    /// état : retire les overlays dont le client a fermé, crée un overlay pour chaque nouvelle
    /// fenêtre de jeu trouvée, repositionne les autres. Appelé au premier `resumed()` et à chaque
    /// tick d'`about_to_wait` (comme l'ancien `track_game_window` mono-fenêtre) — idempotent,
    /// rappelable sans risque.
    fn sync_windows(&mut self, event_loop: &ActiveEventLoop) {
        let found = self.game_window.scan();

        self.windows.retain(|_, overlay| {
            let still_here = found.iter().any(|(_, info)| info.hwnd == overlay.game_hwnd);
            if !still_here {
                tracing::info!(
                    "[fenêtre de jeu] {} fermée — son overlay est retiré.",
                    overlay.character_name
                );
            }
            still_here
        });

        for (character_name, info) in &found {
            for kind in [OverlayKind::Combat, OverlayKind::Watchlist] {
                if let Some(existing) = self
                    .windows
                    .values_mut()
                    .find(|w| w.game_hwnd == info.hwnd && w.kind == kind)
                {
                    Self::reposition(existing, info.rect);
                    continue;
                }
                let overlay = Self::create_overlay_window(
                    event_loop,
                    kind,
                    info.hwnd,
                    character_name.clone(),
                    info.rect,
                    self.interactive,
                );
                tracing::info!(
                    "[fenêtre de jeu] {character_name} trouvée — overlay {kind:?} créé."
                );
                // Explicite plutôt que de compter sur un premier `RedrawRequested` implicite —
                // diagnostic 2026-09-02 (Suivi resté vide au tout premier lancement) : sans
                // certitude que ce premier redessin lise `watchlist`/`snapshot` APRÈS que ces
                // `ArcSwap` aient pu être remplis par un thread de fond déjà en avance sur celui-ci
                // (compte déjà lié via jeton natif, réponse quasi instantanée), autant forcer un
                // redessin explicite dès la création plutôt que de risquer un premier rendu figé
                // sur un état encore vide.
                overlay.window.request_redraw();
                self.windows.insert(overlay.window.id(), overlay);
            }
        }
    }

    /// Position ancrée sur la fenêtre de jeu selon la zone (voir `OverlayKind`) : Combat reste
    /// collé au bord gauche, centré verticalement (comportement d'origine, S1/L2) ; Suivi est
    /// désormais collé au bord HAUT, centré horizontalement — demande utilisateur explicite
    /// 2026-09-01, à l'image du bandeau du web (`tracker-strip.component`).
    fn anchor_position(
        kind: OverlayKind,
        rect: GameRect,
        overlay_width: i32,
        overlay_height: i32,
    ) -> PhysicalPosition<i32> {
        match kind {
            OverlayKind::Combat => PhysicalPosition::new(
                rect.left + GAME_EDGE_MARGIN_PX,
                rect.top + (rect.height - overlay_height) / 2,
            ),
            OverlayKind::Watchlist => PhysicalPosition::new(
                rect.left + (rect.width - overlay_width) / 2,
                rect.client_top + GAME_TOP_MARGIN_PX,
            ),
        }
    }

    fn create_overlay_window(
        event_loop: &ActiveEventLoop,
        kind: OverlayKind,
        game_hwnd: HWND,
        character_name: String,
        rect: GameRect,
        interactive: bool,
    ) -> OverlayWindow {
        let size = match kind {
            OverlayKind::Combat => WINDOW_SIZE,
            // 0 entrée à la création : rien n'est encore chargé (compte/catalogue), la fenêtre
            // démarre donc au plus étroit (juste les 2 tuiles "+"/"−") et s'élargit dès que
            // `watchlist` se remplit (voir le redimensionnement dans `RedrawRequested`).
            OverlayKind::Watchlist => (
                watchlist_target_width(0, false, rect.width),
                watchlist_target_height(false),
            ),
        };
        let title_suffix = match kind {
            OverlayKind::Combat => "Combat",
            OverlayKind::Watchlist => "Suivi",
        };
        let attrs = WindowAttributes::default()
            .with_title(format!(
                "wakfu-companion-overlay — {character_name} — {title_suffix}"
            ))
            .with_inner_size(winit::dpi::LogicalSize::new(size.0, size.1))
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false);
        #[cfg(target_os = "windows")]
        let attrs = attrs
            .with_skip_taskbar(true)
            .with_no_redirection_bitmap(true);

        let window = event_loop
            .create_window(attrs)
            .expect("création de la fenêtre overlay");
        let window = Arc::new(window);

        let hwnd = Self::hwnd_of(&window);
        Self::apply_extended_styles(hwnd);
        if let Err(err) = window.set_cursor_hittest(interactive) {
            tracing::warn!("set_cursor_hittest a échoué à la création : {err}");
        }

        let gpu = pollster::block_on(init_gpu(Arc::clone(&window)));
        let portraits = PortraitAtlas::load(&gpu.egui_ctx);
        let combat_frame = CombatFrame::load(&gpu.egui_ctx);
        let icons = UiIcons::load(&gpu.egui_ctx);

        let outer = window.outer_size();
        let position = Self::anchor_position(kind, rect, outer.width as i32, outer.height as i32);
        window.set_outer_position(position);
        if kind == OverlayKind::Watchlist {
            // Diagnostic PERMANENT (pas juste temporaire) : l'écart entre `rect.top` (bord
            // extérieur, barre de titre comprise) et `rect.client_top` (vrai bord de la zone de
            // jeu) est la source du bug d'ancrage corrigé le 2026-09-01 (voir la doc de
            // `GameRect::client_top`) — utile pour vérifier en un coup d'œil, sur une machine
            // donnée, que `client_top` a bien été résolu (pas replié sur `rect.top`, ce qui se
            // voit ici par un écart nul) avant de retoucher `GAME_TOP_MARGIN_PX` à l'aveugle.
            tracing::debug!(
                "[overlay Suivi] rect.top={} client_top={} (écart {}) -> position.y={}",
                rect.top,
                rect.client_top,
                rect.client_top - rect.top,
                position.y
            );
        }

        OverlayWindow {
            window,
            gpu,
            kind,
            portraits,
            combat_frame,
            icons,
            remote_icon_textures: RemoteIconTextures::default(),
            combat_side: CombatSide::default(),
            game_hwnd,
            game_rect: rect,
            character_name,
            last_position: Some(position),
            // Déjà la largeur demandée ci-dessus (`size.0`) pour une fenêtre `Suivi` — la première
            // vérification dans `RedrawRequested` ne redemande donc rien tant que le nombre
            // d'entrées reste 0. `None` pour `Combat`, qui ne redimensionne jamais.
            last_watchlist_width: (kind == OverlayKind::Watchlist).then_some(size.0),
            last_watchlist_height: (kind == OverlayKind::Watchlist).then_some(size.1),
            is_topmost: true, // WindowLevel::AlwaysOnTop déjà appliqué ci-dessus à la création
            last_topmost_reassert: None,
            pending_demote_since: None,
            next_redraw_at: None,
        }
    }

    /// Recolle une fenêtre overlay sur sa fenêtre de jeu selon son ancrage (voir
    /// `anchor_position`) ; n'appelle `set_outer_position` que si la position cible a changé, pour
    /// ne pas spammer le compositeur DWM 20×/s pour rien.
    fn reposition(overlay: &mut OverlayWindow, rect: GameRect) {
        overlay.game_rect = rect;
        let outer = overlay.window.outer_size();
        let desired =
            Self::anchor_position(overlay.kind, rect, outer.width as i32, outer.height as i32);
        if overlay.last_position != Some(desired) {
            overlay.window.set_outer_position(desired);
            overlay.last_position = Some(desired);
        }
    }

    /// `WS_EX_NOACTIVATE`/`WS_EX_TOOLWINDOW`, non exposés par `winit` — voir S1 (§6.2 du plan).
    fn apply_extended_styles(hwnd: HWND) {
        unsafe {
            let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let new_style = current | (WS_EX_NOACTIVATE.0 as isize) | (WS_EX_TOOLWINDOW.0 as isize);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
        }
    }

    fn hwnd_of(window: &Window) -> HWND {
        match window.window_handle().expect("handle de fenêtre").as_raw() {
            RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as *mut _),
            other => panic!("handle de fenêtre inattendu sur Windows : {other:?}"),
        }
    }

    /// Reconfigure la surface wgpu sur la taille physique donnée — factorisé pour être appelable
    /// depuis DEUX points, pas seulement `WindowEvent::Resized` : le redimensionnement du Suivi
    /// (`RedrawRequested`, voir plus bas) appelle `Window::request_inner_size`, dont la doc winit
    /// est explicite — « on platforms where the size is entirely controlled by the user [Windows
    /// en fait partie] the applied size will be returned immediately, resize event in such case
    /// may not be generated » — et c'était jusqu'ici purement ignoré (`let _ = ...`). Sur Windows,
    /// `request_inner_size` s'applique donc TOUJOURS de façon synchrone : aucun `Resized` ne suit
    /// jamais, la surface restait configurée à l'ancienne (étroite) largeur alors que la fenêtre
    /// elle-même s'élargissait bel et bien — `get_current_texture()` échouait alors sa validation
    /// (taille de surface ≠ taille de fenêtre) et `render` abandonnait le dessin sans rien afficher
    /// (voir son bras `wgpu::CurrentSurfaceTexture::Validation`). Symptôme exact du retour
    /// utilisateur 2026-09-02 : Suivi vide malgré des logs confirmant les entrées bien reçues,
    /// `Ctrl+Alt+R` semblant ne « rien faire » alors qu'il redemandait bien les données à chaque
    /// fois — la fenêtre s'élargissait réellement, mais son contenu ne pouvait plus jamais être
    /// dessiné après ce tout premier redimensionnement synchrone.
    fn reconfigure_surface(gpu: &mut GpuState, size: PhysicalSize<u32>) {
        // Clamp défensif (voir S1, README.md §"Bug de redimensionnement") : un resize excessif ne
        // doit jamais faire planter l'overlay, quelle qu'en soit la cause.
        let max_dim = gpu.device.limits().max_texture_dimension_2d;
        gpu.config.width = size.width.min(max_dim);
        gpu.config.height = size.height.min(max_dim);
        gpu.surface.configure(&gpu.device, &gpu.config);
    }

    fn toggle_interactive(&mut self) {
        self.interactive = !self.interactive;
        for overlay in self.windows.values() {
            if let Err(err) = overlay.window.set_cursor_hittest(self.interactive) {
                tracing::warn!("set_cursor_hittest a échoué : {err}");
            }
            overlay.window.request_redraw();
        }
        tracing::info!(
            ">>> Bascule ({HOTKEY_LABEL}) : mode = {}",
            if self.interactive {
                "INTERACTIF"
            } else {
                "CLIC-TRAVERSANT"
            }
        );
    }

    /// `REFRESH_HOTKEY_LABEL` : demande explicite de l'utilisateur (2026-09-02) — « il faut trouver
    /// une solution » pour un overlay bloqué (mauvaise taille, plus au premier plan, Suivi resté
    /// masqué après un lot de réglages arrivé trop tôt) sans devoir relancer tout le processus.
    /// Explicitement voulu comme un « bouton nucléaire » (retour utilisateur 2026-09-02 : « mon
    /// envie [...] ce serait que quand l'utilisateur appuie sur ce raccourci, ça rafraîchit tout et
    /// ça redessine tout ») après une nouvelle disparition de l'overlay COMBAT cette fois (pas
    /// seulement Suivi) malgré plusieurs `Ctrl+Alt+R` — plutôt que de chercher à isoler laquelle des
    /// pistes déjà connues (topmost silencieusement démoté par Windows, voir `sync_topmost` ;
    /// fenêtre non retrouvée par un `sync_windows` pas encore repassé) était en cause CETTE fois,
    /// ce hotkey doit rester la réponse universelle à « quelque chose s'est mal affiché » sans
    /// obliger l'utilisateur à deviner quoi. Force donc, dans l'ordre : (a) un `sync_windows`
    /// IMMÉDIAT (pas seulement le prochain tick d'`about_to_wait`, ~20 Hz mais quand même un délai
    /// perceptible pour un correctif demandé à la main) — recrée toute fenêtre qu'un passage de
    /// scan aurait pu manquer ; (b) une réaffirmation `HWND_TOPMOST` INCONDITIONNELLE pour chaque
    /// fenêtre encore existante, style étendu (`WS_EX_NOACTIVATE`/`TOOLWINDOW`) réappliqué au
    /// passage — contrairement à `sync_topmost` (réservé au ballet automatique focus/pas-focus),
    /// on ne laisse pas ici le filtre `relevant` (jeu au premier plan À CET INSTANT PRÉCIS) décider
    /// si l'utilisateur a le droit de récupérer SON overlay ; `sync_topmost`, appelé juste après,
    /// reprend la main dès ce même tick si le jeu n'a en réalité pas le focus (repli immédiat en
    /// NOTOPMOST) — jamais une dérogation permanente à la règle "ne pas s'afficher par-dessus une
    /// autre appli" (retour utilisateur 2026-09-01) ; (c) un redessin de CHAQUE fenêtre (recalcule
    /// au passage la largeur du Suivi, voir `WindowEvent::RedrawRequested`) ; et (d) depuis le
    /// retour utilisateur du 2026-09-02 (Suivi resté vide en tout début de session malgré plusieurs
    /// `Ctrl+Alt+R`), une NOUVELLE tentative de récupération des réglages de compte (roster +
    /// suivi) — un redessin seul ne peut rien montrer si `watchlist` (l'`ArcSwap` publié par le
    /// thread Engine, voir `spawn_engine_thread`) n'a en réalité jamais reçu les entrées suivies.
    /// Cette dernière étape reste non bloquante : lancée sur un thread éphémère dédié, jamais
    /// depuis ce thread (winit) ni le thread Engine.
    fn force_refresh(&mut self, event_loop: &ActiveEventLoop) {
        self.sync_windows(event_loop);
        for overlay in self.windows.values_mut() {
            let hwnd = Self::hwnd_of(&overlay.window);
            Self::apply_extended_styles(hwnd);
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            overlay.is_topmost = true;
            overlay.last_topmost_reassert = None;
            overlay.pending_demote_since = None;
            overlay.window.request_redraw();
        }
        self.sync_topmost();
        let settings_tx = self.settings_tx.clone();
        thread::spawn(move || match overlay_sync::token_store::load_token() {
            Some(token) => match overlay_sync::client::fetch_settings(&token) {
                Ok(settings) => {
                    tracing::info!(
                        ">>> Réglages de compte redemandés ({REFRESH_HOTKEY_LABEL}) : {} entrée(s) de suivi."
                        , settings.watchlist.len()
                    );
                    let _ = settings_tx.send(EngineCommand::ApplySettings(settings));
                }
                Err(err) => {
                    tracing::warn!(">>> Échec de la nouvelle demande de réglages ({REFRESH_HOTKEY_LABEL}) : {err}");
                }
            },
            None => tracing::info!(
                ">>> Aucun jeton de compte stocké — rien à redemander ({REFRESH_HOTKEY_LABEL})."
            ),
        });
        tracing::info!(">>> Rafraîchissement forcé ({REFRESH_HOTKEY_LABEL})");
    }

    /// `DISCONNECT_HOTKEY_LABEL` : déconnexion volontaire du compte lié (lot L4, §7.2/§14 point 3
    /// du plan) — jusqu'ici la seule façon de révoquer une session native depuis l'overlay était
    /// d'aller effacer le jeton à la main sur disque/dans le trousseau (aucun moyen depuis
    /// l'overlay lui-même). Purement une commande envoyée au thread Auth (voir `spawn_auth_thread`)
    /// : c'est LUI qui efface le jeton (trousseau + repli fichier) et notifie le thread Engine
    /// (`EngineCommand::Disconnect`) pour revenir en mode invité (repli `breed`, Suivi vidé) —
    /// jamais depuis ce thread (winit) directement, même raison que `force_refresh` (l'accès
    /// trousseau/fichier ne doit jamais bloquer le rendu). Sans effet si aucun compte n'est
    /// actuellement lié (voir `AuthCommand::Disconnect`, ignoré par le thread Auth hors de l'état
    /// `Connected`) — ni si une tentative de connexion est en cours (`Connecting`, ex. en pleine
    /// attente de confirmation d'appairage) : le thread Auth est alors occupé dans
    /// `attempt_connect`, pas encore revenu écouter les commandes ; la déconnexion redeviendra
    /// effective au prochain appui une fois cette tentative résolue.
    fn disconnect_account(&mut self) {
        let _ = self.auth_command_tx.send(AuthCommand::Disconnect);
        tracing::info!(">>> Déconnexion du compte demandée ({DISCONNECT_HOTKEY_LABEL})");
    }

    /// `DETAILS_HOTKEY_LABEL` : même action que le clic sur le bouton "lien externe"
    /// (`panels::combat::bottom_toolbar`) — voir sa doc pour `base_url()`. `open::that` est
    /// best-effort (résultat ignoré, même choix que le clic direct) : un navigateur qui ne s'ouvre
    /// pas n'est pas une raison de faire quoi que ce soit d'autre planter.
    fn open_details(&self) {
        let _ = open::that(overlay_sync::client::base_url());
        tracing::info!(">>> Détails ({DETAILS_HOTKEY_LABEL}) : ouverture du site.");
    }

    /// `SIDE_HOTKEY_LABEL` : bascule Alliés/Ennemis (`CombatSide::toggled`) de CHAQUE fenêtre Combat
    /// actuellement ouverte, pas seulement celle au premier plan — même portée globale que les
    /// autres hotkeys de cette liste. Sans effet sur les fenêtres Suivi (`combat_side` n'a de sens
    /// que pour `OverlayKind::Combat`, voir sa doc dans `OverlayWindow`).
    fn toggle_combat_side(&mut self) {
        for overlay in self.windows.values_mut() {
            if overlay.kind == OverlayKind::Combat {
                overlay.combat_side = overlay.combat_side.toggled();
                overlay.window.request_redraw();
            }
        }
        tracing::info!(">>> Bascule Alliés/Ennemis ({SIDE_HOTKEY_LABEL})");
    }

    /// Chaque overlay au-dessus SEULEMENT si SA PROPRE fenêtre de jeu (ou lui-même) a le focus ;
    /// sinon repli en z-order normal — pour ne plus recouvrir une application quelconque devenue
    /// active (retour utilisateur 2026-09-01 : "l'overlay ne doit pas s'afficher par-dessus
    /// l'explorateur de fichiers"), ET pour que passer d'une fenêtre de jeu à l'autre en
    /// multi-compte fasse remonter le BON overlay au premier plan. Un seul `GetForegroundWindow()`
    /// par tick, comparé au `HWND` de chaque fenêtre suivie — coût négligeable.
    ///
    /// **Correctif 2026-09-01** (retour utilisateur, multi-fenêtre) : la politique précédente
    /// (« TOUTE fenêtre de jeu Wakfu remet TOUS les overlays au premier plan ») avait deux défauts
    /// en pratique avec 2+ personnages simultanés : (a) passer d'une fenêtre de jeu à l'autre ne
    /// changeait RIEN au z-order relatif entre les deux overlays (les deux restaient topmost tout
    /// du long, celui déjà au-dessus le restait indéfiniment, quel que soit le personnage
    /// réellement actif) — un seul overlay restait visible, souvent le mauvais ; (b) `Ctrl+Alt+W`
    /// bascule `interactive` pour TOUTES les fenêtres à la fois (voir `toggle_interactive`), donc
    /// l'overlay resté topmost au mauvais endroit interceptait aussi les clics destinés au jeu
    /// dessous. Callback PAR overlay : celui dont la fenêtre de jeu vient de reprendre le focus est
    /// (ré)inséré en tête du groupe topmost par ce `SetWindowPos`, les autres retombent derrière.
    fn sync_topmost(&mut self) {
        let foreground = unsafe { GetForegroundWindow() };
        let now = std::time::Instant::now();

        for overlay in self.windows.values_mut() {
            let relevant =
                overlay.game_hwnd == foreground || Self::hwnd_of(&overlay.window) == foreground;

            // Retour utilisateur 2026-09-02 : « l'overlay disparaît de manière indéterminée, il
            // n'y a rien qui permet de le réafficher ». Cause trouvée : Windows peut démoter un
            // HWND_TOPMOST tout seul (alt-tab, notification système, une autre fenêtre qui
            // réclame aussi le premier plan...) SANS passer par notre `SetWindowPos` — `is_topmost`
            // continuait alors de croire l'overlay au premier plan (rien n'avait changé de NOTRE
            // point de vue) et ne le réaffirmait donc jamais, laissant l'overlay caché derrière le
            // jeu indéfiniment.
            //
            // Fix : réaffirmer HWND_TOPMOST PÉRIODIQUEMENT (`TOPMOST_REASSERT_INTERVAL`) tant que
            // `relevant` reste vrai — immédiatement sur toute transition détectée (`is_topmost` qui
            // change), sinon au plus toutes les `TOPMOST_REASSERT_INTERVAL` (throttle ajouté après
            // un premier essai « à chaque tick », 20×/s — pas de raison connue de le soupçonner
            // dans un nouveau signalement de disparition prolongée après une longue session, mais
            // par prudence : un `SetWindowPos` qui ne change réellement rien reste rare mais pas
            // strictement gratuit, autant l'éviter sur des heures de jeu).
            if relevant {
                overlay.pending_demote_since = None;
                let transitioned = !overlay.is_topmost;
                let due_for_reassert = overlay
                    .last_topmost_reassert
                    .is_none_or(|t| now.duration_since(t) >= TOPMOST_REASSERT_INTERVAL);
                if !transitioned && !due_for_reassert {
                    continue;
                }
                let hwnd = Self::hwnd_of(&overlay.window);
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        Some(HWND_TOPMOST),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
                overlay.is_topmost = true;
                overlay.last_topmost_reassert = Some(now);
                continue;
            }

            // `relevant == false` : retour utilisateur 2026-09-02 (vidéo à l'appui), l'overlay
            // Combat disparaissait « un coup sur deux » en changeant de fenêtre alors que le Suivi
            // du même personnage restait visible au même instant, bien que les deux passent par ce
            // même code — la démotion en NOTOPMOST était jusqu'ici IMMÉDIATE dès qu'un seul tick
            // (~50 ms) voyait `GetForegroundWindow()` cesser de désigner la fenêtre de jeu, ce qui
            // rend le résultat sensible au moindre aléa d'ordonnancement entre les deux fenêtres
            // overlay (Windows peut livrer le nouveau premier plan à l'un des deux `SetWindowPos`
            // un tick avant l'autre). `pending_demote_since` absorbe cet aléa : on ne démote qu'une
            // fois `relevant` resté faux pendant `TOPMOST_DEMOTE_GRACE` en continu, pas déjà
            // démoté sinon. Un retour à `relevant == true` avant l'échéance annule la démotion sans
            // jamais avoir bougé le z-order (voir la branche `if relevant` ci-dessus, qui vide
            // `pending_demote_since`).
            if !overlay.is_topmost {
                continue;
            }
            let demote_due_at = *overlay.pending_demote_since.get_or_insert(now);
            if now.duration_since(demote_due_at) < TOPMOST_DEMOTE_GRACE {
                continue;
            }
            let hwnd = Self::hwnd_of(&overlay.window);
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_NOTOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            overlay.is_topmost = false;
            overlay.pending_demote_since = None;
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.sync_windows(event_loop);
        if !self.banner_printed {
            tracing::info!("=== wakfu-companion-overlay (L2, overlay-ui) ===");
            tracing::info!("Suivi de {}", self.log_path.display());
            tracing::info!(
                "{HOTKEY_LABEL} pour basculer interactif / clic-traversant. \
                 {REFRESH_HOTKEY_LABEL} pour forcer un rafraîchissement (overlay bloqué/mal \
                 positionné, ou Suivi resté vide). {DISCONNECT_HOTKEY_LABEL} pour déconnecter le \
                 compte lié (repli mode invité). {QUIT_HOTKEY_LABEL} ou Ctrl+C (dans ce \
                 terminal) pour quitter."
            );
            self.banner_printed = true;
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            // Les deux variantes ont le même effet ici : un nouvel état est disponible (snapshot
            // de combat, ou statut de connexion au compte), toutes les fenêtres doivent redessiner
            // pour le refléter (le statut de connexion, en particulier, pilote l'icône de relance
            // d'appairage — voir `render`).
            UserEvent::NewSnapshot | UserEvent::AuthStatusChanged => {
                for overlay in self.windows.values() {
                    overlay.window.request_redraw();
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(overlay) = self.windows.get_mut(&id) else {
            return; // événement d'une fenêtre déjà retirée (client fermé entre-temps) — ignoré
        };

        let response = overlay
            .gpu
            .egui_winit
            .on_window_event(&overlay.window, &event);
        if response.repaint {
            overlay.window.request_redraw();
        }

        match event {
            WindowEvent::CloseRequested => {
                logging::log_session_end("fermeture de fenêtre");
                event_loop.exit();
            }
            // Ne se déclenche en pratique JAMAIS (voir la doc de `QUIT_HOTKEY_LABEL`) : ces
            // fenêtres portent `WS_EX_NOACTIVATE`, donc ne reçoivent jamais le focus clavier quel
            // que soit le mode — laissé en place au cas où une future fenêtre overlay redeviendrait
            // focalisable, mais `QUIT_HOTKEY_LABEL` est le SEUL moyen fiable de quitter sans passer
            // par le terminal.
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && event.physical_key == PhysicalKey::Code(KeyCode::Escape)
                {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                Self::reconfigure_surface(&mut overlay.gpu, size);
            }
            WindowEvent::RedrawRequested => {
                // Une seule lecture de l'horloge par frame (voir la doc de `RenderContent::now`)
                // — réutilisée ci-dessous pour le gabarit dynamique du Suivi ET transmise à
                // `render`/`build_ui`, plutôt que deux `Instant::now()` distincts à quelques
                // instructions d'écart qui pourraient (rarement) diverger pile à l'expiration
                // d'un toast.
                let now = std::time::Instant::now();
                let snapshot = self.snapshot.load();
                let fight = snapshot.fight_for_character(&overlay.character_name);
                let watchlist = self.watchlist.load();
                let watchlist_toast_guard = self.watchlist_toast.load();
                let watchlist_toast: Option<&WatchlistToast> = (**watchlist_toast_guard).as_ref();
                if overlay.kind == OverlayKind::Watchlist {
                    // Gabarit piloté par le CONTENU (retour utilisateur 2026-09-02 : une fenêtre
                    // plus large que nécessaire reste cliquable/bloquante sur toute sa zone même
                    // transparente, l'utilisateur ne peut alors pas deviner où s'arrête l'overlay)
                    // — voir la doc de `watchlist_target_width`/`watchlist_target_height`.
                    // Comparé à la dernière valeur DEMANDÉE (`last_watchlist_width`/`_height`), pas
                    // à la taille réelle actuelle de la fenêtre, pour ne pas rappeler
                    // `request_inner_size` en boucle tant que rien n'a changé.
                    let toast_active = panels::watchlist::is_active(watchlist_toast, now);
                    let target_width = watchlist_target_width(
                        watchlist.len(),
                        toast_active,
                        overlay.game_rect.width,
                    );
                    let target_height = watchlist_target_height(toast_active);
                    if overlay.last_watchlist_width != Some(target_width)
                        || overlay.last_watchlist_height != Some(target_height)
                    {
                        // Sur Windows, cet appel s'applique TOUJOURS de façon synchrone — `Some`
                        // est renvoyé immédiatement et AUCUN `WindowEvent::Resized` ne suit jamais
                        // (voir la doc de `reconfigure_surface`, qui corrige exactement ce cas :
                        // sans ce bras, la surface wgpu restait configurée à l'ancienne largeur
                        // pour toujours, `render` échouait alors sa validation à chaque tentative
                        // suivante et n'affichait plus jamais rien — Suivi durablement invisible
                        // dès le tout premier élargissement, quel que soit le nombre de
                        // `Ctrl+Alt+R`).
                        if let Some(actual) =
                            overlay
                                .window
                                .request_inner_size(winit::dpi::LogicalSize::new(
                                    target_width,
                                    target_height,
                                ))
                        {
                            Self::reconfigure_surface(&mut overlay.gpu, actual);
                        }
                        overlay.last_watchlist_width = Some(target_width);
                        overlay.last_watchlist_height = Some(target_height);
                    }
                }
                let catalog = self.catalog.load();
                let auth_status = self.auth_status.load();
                let (repaint_delay, close_toast) = render(
                    &mut overlay.gpu,
                    &overlay.window,
                    RenderContent {
                        kind: overlay.kind,
                        fight,
                        portraits: &overlay.portraits,
                        combat_frame: &overlay.combat_frame,
                        icons: &overlay.icons,
                        combat_side: &mut overlay.combat_side,
                        watchlist: &watchlist,
                        watchlist_toast,
                        catalog: &catalog,
                        catalog_stale: self.catalog_stale.load(Ordering::Relaxed),
                        remote_icons: &self.remote_icons,
                        remote_icon_textures: &mut overlay.remote_icon_textures,
                        auth_status: &auth_status,
                        auth_command_tx: &self.auth_command_tx,
                        interactive: self.interactive,
                        now,
                    },
                );
                // Fermeture au clic (carte ou croix, voir `panels::watchlist::toast_card`) — seul
                // point du code à détenir un accès en écriture à cet `ArcSwap` (`render` ne reçoit
                // le toast qu'en lecture, voir `RenderContent::watchlist_toast`). Le clic ayant
                // déjà fait passer `response.repaint` à `true` plus haut, le prochain redessin
                // relira `None` et n'affichera plus rien.
                if close_toast {
                    self.watchlist_toast.store(Arc::new(None));
                }
                // Voir `OverlayWindow::next_redraw_at` : egui a pu demander un redessin après un
                // délai (tooltip...) que rien d'autre ne redéclenchera dans cette architecture.
                // `about_to_wait` est responsable de le consommer le moment venu.
                overlay.next_redraw_at = (repaint_delay < std::time::Duration::from_secs(3600))
                    .then(|| std::time::Instant::now() + repaint_delay);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Hotkey global : thread OS dédié, sondé ici sans bloquer (voir S1). `while let` (pas un
        // simple `if`) : chaque appui PHYSIQUE produit deux événements (`Pressed` PUIS `Released`,
        // voir `HotKeyState`) — les deux peuvent être en file au même tick à ~20 Hz. Filtré sur
        // `Pressed` uniquement : le code précédent réagissait aux deux, togglant deux fois de
        // suite pour un seul appui (bug réel, symptôme observé 2026-09-02 : plusieurs lignes
        // ">>> Bascule" consécutives dans les logs pour un nombre d'appuis bien moindre).
        while let Ok(event) = self.hotkey_events.try_recv() {
            if event.state != global_hotkey::HotKeyState::Pressed {
                continue;
            }
            if event.id == self.toggle_hotkey_id {
                self.toggle_interactive();
            } else if event.id == self.refresh_hotkey_id {
                self.force_refresh(event_loop);
            } else if event.id == self.quit_hotkey_id {
                logging::log_session_end(QUIT_HOTKEY_LABEL);
                event_loop.exit();
            } else if event.id == self.disconnect_hotkey_id {
                self.disconnect_account();
            } else if event.id == self.details_hotkey_id {
                self.open_details();
            } else if event.id == self.options_hotkey_id {
                // Voir la doc de `OPTIONS_HOTKEY_LABEL` : aucun panneau à ouvrir pour l'instant,
                // même no-op que le clic sur le bouton (`panels::combat::bottom_toolbar`).
                tracing::debug!(
                    ">>> Options ({OPTIONS_HOTKEY_LABEL}) : aucun panneau à ouvrir pour l'instant."
                );
            } else if event.id == self.watchlist_add_hotkey_id {
                // Voir la doc de `WATCHLIST_ADD_HOTKEY_LABEL` : bouton encore inerte.
                tracing::debug!(">>> Ajouter ({WATCHLIST_ADD_HOTKEY_LABEL}) : encore inerte.");
            } else if event.id == self.watchlist_remove_hotkey_id {
                tracing::debug!(">>> Supprimer ({WATCHLIST_REMOVE_HOTKEY_LABEL}) : encore inerte.");
            } else if event.id == self.side_hotkey_id {
                self.toggle_combat_side();
            }
        }
        // Découverte/suivi des fenêtres de jeu : même sondage périodique que le hotkey (pas d'API
        // Win32 pour être notifié d'un déplacement/redimensionnement/apparition d'une fenêtre qui
        // n'est pas la nôtre sans un hook global — un sondage à 20 Hz est largement assez réactif
        // ici et reste négligeable en coût, voir game_window.rs).
        self.sync_windows(event_loop);
        self.sync_topmost();

        // Honore les délais de redessin qu'egui a demandés (tooltip au survol d'un portrait,
        // typiquement) et qu'aucun `WindowEvent`/`UserEvent` ne redéclenchera de lui-même — voir
        // `OverlayWindow::next_redraw_at` et `render`. Sans ceci, la tooltip ne s'affichait qu'au
        // hasard d'un autre redessin (retour utilisateur 2026-09-01).
        let now = std::time::Instant::now();
        let mut next_wake = now + std::time::Duration::from_millis(50);
        for overlay in self.windows.values_mut() {
            if let Some(due) = overlay.next_redraw_at {
                if due <= now {
                    overlay.next_redraw_at = None;
                    overlay.window.request_redraw();
                } else {
                    next_wake = next_wake.min(due);
                }
            }
        }

        // Réactif (§6.1) : on attend soit un événement fenêtre, soit un `UserEvent::NewSnapshot`
        // du thread Engine, jamais de boucle 60 Hz forcée. Le sondage hotkey/fenêtre de jeu
        // ci-dessus impose quand même un réveil périodique court (borne haute de `next_wake`),
        // sans quoi ni l'un ni l'autre ne seraient vus qu'au prochain événement fenêtre.
        event_loop.set_control_flow(ControlFlow::WaitUntil(next_wake));
    }
}

async fn init_gpu(window: Arc<Window>) -> GpuState {
    // Voir spikes/s1-window-windows/README.md pour le détail complet de ce qui suit — bugs
    // wgpu-hal corrigés par le patch vendored, choix d'alpha prémultiplié, etc.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        backend_options: wgpu::BackendOptions {
            dx12: wgpu::Dx12BackendOptions {
                presentation_system: wgpu::Dx12SwapchainKind::DxgiFromVisual,
                ..Default::default()
            },
            ..Default::default()
        },
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });

    // `Arc<Window>` donne un `Surface<'static>` sans fuite (la surface garde l'`Arc` en interne,
    // la fenêtre reste vivante aussi longtemps qu'elle) — voir la doc d'`OverlayWindow::window`.
    let surface = instance
        .create_surface(Arc::clone(&window))
        .expect("création de la surface (DirectComposition visual)");

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            ..Default::default()
        })
        .await
        .expect("aucun adaptateur DX12 compatible");
    tracing::info!("Adaptateur GPU : {:?}", adapter.get_info());

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("overlay-ui-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
            ..Default::default()
        })
        .await
        .expect("création du device");

    let size = window.inner_size();
    let caps = surface.get_capabilities(&adapter);
    let format = caps
        .formats
        .iter()
        .copied()
        .find(|f| !f.is_srgb())
        .unwrap_or(caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
        view_formats: vec![],
        color_space: wgpu::SurfaceColorSpace::Auto,
    };
    surface.configure(&device, &config);

    let egui_ctx = egui::Context::default();
    // Style partagé avec le binaire Linux ET le harnais de rendu offscreen (`overlay_ui::style`,
    // voir sa doc) : délai/persistance des tooltips, curseur "main" au survol, design system
    // tooltip (retour utilisateur 2026-09-06) — plutôt qu'un réglage dupliqué à chaque point de
    // création d'`egui::Context`, qui avait déjà divergé entre les deux binaires avant ce refactor.
    overlay_ui::style::apply(&egui_ctx);
    let egui_winit = egui_winit::State::new(
        egui_ctx.clone(),
        egui::ViewportId::ROOT,
        window.as_ref(),
        Some(window.scale_factor() as f32),
        None,
        None,
    );
    let egui_renderer =
        egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());

    GpuState {
        surface,
        device,
        queue,
        config,
        egui_ctx,
        egui_winit,
        egui_renderer,
    }
}

/// Thread Auth (lot L4, §7.2 du plan) : résout l'accès au compte AVANT de bloquer sur quoi que ce
/// soit d'autre — jamais le thread Engine ni le main thread. Best-effort et jamais fatal : sans
/// jeton stocké ni appairage complété, l'overlay continue simplement en mode invité (repli
/// `breed` déjà géré par `overlay-engine::session`), exactement comme le mode invité du web.
///
/// Thread Catalogue (lot L3, §7.4 du plan — réduit pour l'instant à la résolution d'icônes, voir
/// `overlay_engine::catalog`) : résout les icônes réelles d'objets/monstres affichées par le
/// panneau Suivi (retour utilisateur 2026-09-02, capture d'écran à l'appui : icône générique
/// partout, tuiles impossibles à distinguer). Offline-first (mêmes principes que
/// `CatalogService.initialize()` côté web) : le cache disque
/// (`overlay_sync::catalog_cache`) est chargé et publié IMMÉDIATEMENT s'il existe, sans attendre
/// le réseau — le rafraîchissement qui suit ne republie que si `GET /api/v1/catalog/version`
/// (`indexHash`) a changé depuis le cache, jamais pour rien. Le MÊME `Arc<ArcSwap<CatalogIndex>>`
/// est aussi relayé au thread Engine (voir `spawn_engine_thread`, `Engine::set_catalog`) : sert
/// cette fois de garde-fou `hostIsKnownMonsterName` (`quickjs_engine.rs`) contre un vrai monstre
/// qui se révèle (mimique, brèche) confondu à tort avec une invocation.
///
/// Tout premier lancement SANS cache disque ET SANS réseau : repli sur le catalogue embarqué
/// (`overlay_sync::catalog_cache::embedded_fallback`, §7.4 du plan) — l'overlay reste utilisable
/// plutôt que de rester sur un `CatalogIndex::default()` vide. `catalog_stale` (lu par `render`,
/// icône « 📦⚠ » de la zone Combat) est mis à `true` dans ce seul cas — jamais réinitialisé à
/// `false` explicitement ailleurs dans ce thread : sa valeur initiale (posée par `main`) est déjà
/// `false`, et les autres branches de cette fonction ne s'exécutent qu'une fois par lancement, donc
/// aucune ne peut suivre une mise à `true` pour la corriger a posteriori.
fn spawn_catalog_thread(
    catalog: Arc<ArcSwap<CatalogIndex>>,
    catalog_stale: Arc<AtomicBool>,
    proxy: EventLoopProxy<UserEvent>,
) {
    thread::Builder::new()
        .name("overlay-catalog".into())
        .spawn(move || {
            let mut cached_hash = None;
            if let Some((hash, index)) = overlay_sync::catalog_cache::load() {
                catalog.store(Arc::new(CatalogIndex::from_compact_json(&index)));
                let _ = proxy.send_event(UserEvent::NewSnapshot);
                cached_hash = Some(hash);
            }

            let latest_hash = match overlay_sync::client::fetch_catalog_version() {
                Ok(hash) => hash,
                Err(err) => {
                    // Repli hors-ligne EMBARQUÉ (§7.4 du plan, `catalog_cache::embedded_fallback`)
                    // — uniquement si `cached_hash` est vide : un cache disque déjà chargé
                    // ci-dessus reste toujours préférable (référentiel plus récent que le
                    // placeholder embarqué), le réseau injoignable n'y change rien.
                    if cached_hash.is_none() {
                        tracing::warn!(
                            %err,
                            "catalogue injoignable ET aucun cache local — repli sur le catalogue embarqué (daté)"
                        );
                        catalog.store(Arc::new(CatalogIndex::from_compact_json(
                            &overlay_sync::catalog_cache::embedded_fallback(),
                        )));
                        catalog_stale.store(true, Ordering::Relaxed);
                        let _ = proxy.send_event(UserEvent::NewSnapshot);
                    } else {
                        tracing::warn!(%err, "version du catalogue injoignable, repli sur le cache local");
                    }
                    return;
                }
            };
            if cached_hash.as_deref() == Some(latest_hash.as_str()) {
                tracing::info!("catalogue déjà à jour (cache local)");
                return;
            }
            match overlay_sync::client::fetch_catalog_index() {
                Ok(index) => {
                    catalog.store(Arc::new(CatalogIndex::from_compact_json(&index)));
                    let _ = proxy.send_event(UserEvent::NewSnapshot);
                    if let Err(err) = overlay_sync::catalog_cache::save(&latest_hash, &index) {
                        tracing::warn!(
                            %err,
                            "échec de mise en cache du catalogue (retéléchargé au prochain lancement)"
                        );
                    }
                }
                Err(err) => tracing::warn!(
                    %err,
                    "téléchargement du catalogue impossible, repli sur le cache local"
                ),
            }
        })
        .expect("échec de création du thread Catalogue");
}

/// Thread Donjons (L5, §7.1 du plan — assignation `dungeonId`/`dungeonRunKey`) : miroir simplifié
/// de `spawn_catalog_thread` ci-dessus — `GET /api/v1/dungeons` n'expose pas d'endpoint `/version`
/// séparé (voir `reference_data_cache.rs`), donc pas de comparaison de hash possible : le cache
/// disque est chargé immédiatement (l'Engine reste utilisable pour la synchro dès le démarrage même
/// hors ligne), puis la version réseau REMPLACE inconditionnellement l'index en mémoire dès qu'elle
/// arrive, sans jamais bloquer ce thread ni les autres. Volume négligeable (~150 lignes, voir la
/// doc de tête de `dungeon.rs`) : pas de repli embarqué comme pour le catalogue (~1,8 Mo) — un
/// référentiel de donjons manquant laisse simplement `dungeonId` à `None`, jamais un blocage.
fn spawn_dungeon_thread(dungeons: Arc<ArcSwap<DungeonIndex>>, proxy: EventLoopProxy<UserEvent>) {
    thread::Builder::new()
        .name("overlay-dungeons".into())
        .spawn(move || {
            if let Some(rows) = overlay_sync::reference_data_cache::load(
                overlay_sync::reference_data_cache::ReferenceData::Dungeons,
            ) {
                dungeons.store(Arc::new(DungeonIndex::from_json(&rows)));
                let _ = proxy.send_event(UserEvent::NewSnapshot);
            }
            match overlay_sync::client::fetch_dungeons() {
                Ok(rows) => {
                    dungeons.store(Arc::new(DungeonIndex::from_json(&rows)));
                    let _ = proxy.send_event(UserEvent::NewSnapshot);
                    if let Err(err) = overlay_sync::reference_data_cache::save(
                        overlay_sync::reference_data_cache::ReferenceData::Dungeons,
                        &rows,
                    ) {
                        tracing::warn!(
                            %err,
                            "échec de mise en cache du référentiel de donjons (retéléchargé au prochain lancement)"
                        );
                    }
                }
                Err(err) => tracing::warn!(
                    %err,
                    "téléchargement du référentiel de donjons impossible, repli sur le cache local (dungeonId non résolu si aucun cache)"
                ),
            }
        })
        .expect("échec de création du thread Donjons");
}

/// Thread Sync (lot L5, §7.3 du plan) : possède la file SQLite persistante (`overlay_sync::
/// SyncQueue`) — écriture (`enqueue`) ET envoi réseau (`flush_once`) vivent entièrement ici, jamais
/// sur le thread Engine (voir `spawn_engine_thread`, qui ne fait que relayer des `SyncEvent` déjà
/// sérialisés sans jamais attendre dessus) ni sur le main thread. Best-effort à l'ouverture de la
/// file elle-même (dossier de données/SQLite indisponible) : le thread se termine simplement,
/// l'overlay continue sans synchro plutôt que de planter — même philosophie que
/// `spawn_catalog_thread`/`spawn_auth_thread`.
///
/// **Simplification assumée par rapport à `SyncQueueService` côté web** : pas de debounce explicite
/// de 2 s avant un envoi (`FLUSH_DEBOUNCE_MS`) — chaque `SyncCommand::Enqueue` tente un
/// `flush_once` immédiatement après écriture. Les entrées ne sont de toute façon jamais perdues
/// (persistées avant tout envoi), un lot ingéré produit rarement plus d'une poignée d'événements à
/// la fois (contrairement au rattrapage initial d'un `wakfu.log` entier, qui arrive lui aussi par
/// lots ≤ 2 000 lignes, voir `overlay-ingest`), et `flush_once` traite de toute façon TOUT ce qui
/// est en file au moment de l'appel (pas seulement le dernier lot reçu) — la seule perte réelle est
/// un envoi réseau de plus qu'avec un vrai debounce, jamais un comportement incorrect.
///
/// Le délai avant le PROCHAIN passage (`wait`, argument de `recv_timeout`) encode à la fois
/// l'inactivité (aucun compte connu : 1 h, réveillé immédiatement par la prochaine commande) et le
/// backoff après un échec réessayable (`FlushOutcome::Retry`, voir `backoff_delay` — 15 s à 5 min,
/// doublé à chaque échec consécutif, miroir de `RETRY_BASE_DELAY_MS`/`RETRY_MAX_DELAY_MS` côté web).
fn spawn_sync_thread(command_rx: mpsc::Receiver<SyncCommand>) {
    thread::Builder::new()
        .name("overlay-sync".into())
        .spawn(move || {
            let mut queue = match overlay_sync::SyncQueue::default_store_path() {
                Some(path) => match overlay_sync::SyncQueue::open(&path) {
                    Ok(queue) => queue,
                    Err(err) => {
                        tracing::warn!(
                            %err,
                            "file de synchro (SQLite) indisponible — historique non envoyé au compte cette session"
                        );
                        return;
                    }
                },
                None => {
                    tracing::warn!(
                        "impossible de résoudre le dossier de données de l'overlay — file de synchro désactivée"
                    );
                    return;
                }
            };

            // `uid` (pour `client_key`) ET `token` (pour authentifier l'envoi, voir
            // `SyncCommand::Activate`) — toujours mis à jour ensemble.
            let mut account: Option<(String, String)> = None;
            // Pas de compte connu au démarrage : n'attend qu'une commande, ne sonde jamais pour
            // rien (même philosophie que `settings_rx.try_recv()` côté thread Engine).
            let mut wait = std::time::Duration::from_secs(3600);
            loop {
                match command_rx.recv_timeout(wait) {
                    Ok(SyncCommand::Activate { uid, token }) => {
                        tracing::info!("file de synchro activée (compte connecté, lot L5)");
                        account = Some((uid, token));
                    }
                    Ok(SyncCommand::Deactivate) => {
                        tracing::info!(
                            "file de synchro désactivée (mode invité) — contenu déjà en file conservé sur disque"
                        );
                        account = None;
                    }
                    Ok(SyncCommand::Enqueue(events)) => {
                        for event in &events {
                            if let Err(err) = queue.enqueue(event) {
                                tracing::warn!(
                                    %err,
                                    kind = event.kind.as_str(),
                                    "échec d'enfilage d'un événement d'historique (SQLite)"
                                );
                            }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {} // réessai programmé (backoff) — retombe sur le flush ci-dessous
                    Err(mpsc::RecvTimeoutError::Disconnected) => break, // App fermée
                }

                wait = match &account {
                    None => std::time::Duration::from_secs(3600),
                    Some((uid, token)) => {
                        match queue.flush_once(uid, |path, body| {
                            overlay_sync::post_json_authenticated(token, path, body)
                        }) {
                            Ok(overlay_sync::FlushOutcome::Idle | overlay_sync::FlushOutcome::Synced) => {
                                std::time::Duration::from_secs(3600)
                            }
                            // **Correctif du 2026-09-03** : cet échec n'était auparavant tracé
                            // nulle part — un blocage persistant (401/429/réseau/5xx) restait
                            // invisible, backoff après backoff, jusqu'à 5 min entre essais.
                            Ok(overlay_sync::FlushOutcome::Retry(reason)) => {
                                tracing::warn!(
                                    reason = %reason,
                                    consecutive_failures = queue.consecutive_failures(),
                                    "échec d'envoi de l'historique — nouvel essai après un backoff"
                                );
                                backoff_delay(queue.consecutive_failures())
                            }
                            Err(err) => {
                                tracing::warn!(%err, "erreur de file de synchro (SQLite)");
                                std::time::Duration::from_secs(60)
                            }
                        }
                    }
                };
            }
        })
        .expect("échec de création du thread Sync");
}

/// Miroir de `RETRY_BASE_DELAY_MS`/`RETRY_MAX_DELAY_MS`/le calcul de `scheduleRetry`
/// (`sync-queue.service.ts`) — 15 s doublés à chaque échec consécutif, plafonné à 5 min.
fn backoff_delay(consecutive_failures: u32) -> std::time::Duration {
    const BASE_MS: u64 = 15_000;
    const MAX_MS: u64 = 5 * 60_000;
    let exponent = consecutive_failures.saturating_sub(1).min(20); // évite un débordement de décalage
    let delay_ms = BASE_MS.saturating_mul(1u64 << exponent).min(MAX_MS);
    std::time::Duration::from_millis(delay_ms)
}

/// **Boucle de retentative** (2026-09-01, retour utilisateur : appairage en échec — 405 côté
/// serveur — sans aucun moyen de retenter sans relancer tout le logiciel) : une tentative échouée
/// (`attempt_connect` renvoie `Err(raison)`) publie `AuthStatus::Disconnected { reason }` (voir
/// `status`) plutôt que de laisser le thread mourir — `render` en déduit l'icône de relance (dont
/// le tooltip affiche `reason`), et son clic pousse `AuthCommand::Retry` dans `command_rx` pour
/// reprendre cette boucle.
///
/// **Déconnexion volontaire** (2026-09-02, §14 point 3 du plan) : contrairement à la version
/// initiale de ce thread, une connexion réussie ne fait PLUS terminer le thread (`return`) — il
/// reste vivant, à l'écoute de `command_rx`, pour pouvoir traiter un `AuthCommand::Disconnect`
/// (raccourci `DISCONNECT_HOTKEY_LABEL`, voir `App::disconnect_account`) à tout moment tant que le
/// compte reste lié. Un `Disconnect` efface le jeton (`token_store::clear_token`), notifie le
/// thread Engine (`EngineCommand::Disconnect`, voir `spawn_engine_thread`) pour qu'il revienne en
/// mode invité, republie `AuthStatus::Disconnected`, puis attend un `AuthCommand::Retry` avant de
/// relancer un appairage — **jamais automatiquement** : une déconnexion volontaire ne doit pas
/// rouvrir le navigateur toute seule. Chaque commande hors de son état pertinent (`Retry` reçu
/// alors que déjà connecté, `Disconnect` reçu alors que déjà déconnecté ou en pleine tentative) est
/// silencieusement ignorée plutôt que de perturber l'état courant.
///
/// **UI de pairing (2026-09-02)** : le code d'appairage est désormais publié via
/// `AuthStatus::PairingStarted` (voir `attempt_connect`) et affiché directement dans la fenêtre
/// overlay (voir `render`), plus seulement en console — en plus du déclenchement d'une nouvelle
/// tentative, de la déconnexion et de la raison du dernier échec, déjà pilotables depuis l'overlay.
fn spawn_auth_thread(
    settings_tx: mpsc::Sender<EngineCommand>,
    sync_tx: mpsc::Sender<SyncCommand>,
    status: Arc<ArcSwap<AuthStatus>>,
    command_rx: mpsc::Receiver<AuthCommand>,
    proxy: EventLoopProxy<UserEvent>,
) {
    thread::Builder::new()
        .name("overlay-auth".into())
        .spawn(move || loop {
            status.store(Arc::new(AuthStatus::Connecting));
            let _ = proxy.send_event(UserEvent::AuthStatusChanged);

            let result = attempt_connect(&settings_tx, &sync_tx, &status, &proxy);

            let mut connected = result.is_ok();
            status.store(Arc::new(match result {
                Ok(()) => AuthStatus::Connected,
                Err(reason) => AuthStatus::Disconnected { reason },
            }));
            let _ = proxy.send_event(UserEvent::AuthStatusChanged);

            // Attend la commande qui justifie de reprendre la boucle externe (nouvel appel à
            // `attempt_connect`) : `Retry` seulement si PAS déjà connecté, jamais de nouvelle
            // tentative automatique en boucle (ce serait spammer le serveur/le navigateur pour un
            // utilisateur qui n'a peut-être pas l'intention de lier son compte). Une fois
            // `Disconnect` traité (voir `connected = false` ci-dessous), un `Retry` suivant reprend
            // normalement la boucle externe — c'est ce qui permet de relier un compte après une
            // déconnexion volontaire sans redémarrer l'overlay.
            loop {
                match command_rx.recv() {
                    Ok(AuthCommand::Retry) if !connected => break,
                    Ok(AuthCommand::Disconnect) if connected => {
                        overlay_sync::token_store::clear_token();
                        let _ = settings_tx.send(EngineCommand::Disconnect);
                        let _ = sync_tx.send(SyncCommand::Deactivate);
                        connected = false;
                        status.store(Arc::new(AuthStatus::Disconnected {
                            reason: "déconnecté manuellement".to_string(),
                        }));
                        let _ = proxy.send_event(UserEvent::AuthStatusChanged);
                        tracing::info!(
                            "[compte] déconnecté ({DISCONNECT_HOTKEY_LABEL}) — repli mode invité."
                        );
                    }
                    Ok(_) => continue, // commande sans effet dans l'état courant — ignorée
                    Err(_) => return,  // App fermée (canal fermé avec l'émetteur) — rien à faire.
                }
            }
        })
        .expect("échec de création du thread Auth");
}

/// Une tentative complète de connexion au compte : jeton déjà stocké et encore valide, sinon
/// nouvel appairage — voir la doc de `spawn_auth_thread` pour la boucle de retentative autour de
/// cette fonction. `Ok(())` si les réglages de compte (roster + watchlist) ont bien été récupérés
/// et transmis (`settings_tx`), `Err(_)` sinon (appairage non complété, ou réglages injoignables
/// même après appairage) avec un message COURT destiné à l'utilisateur (tooltip de l'icône de
/// relance, voir `render` — pas qu'à la console) : dans tous les cas l'overlay continue, au pire
/// en mode invité (repli `breed`, aucun suivi affiché). `status`/`proxy` servent uniquement à
/// publier `AuthStatus::PairingStarted` dès que le code d'appairage est connu (voir plus bas) —
/// `spawn_auth_thread` publie lui-même `Connecting`/`Connected`/`Disconnected` autour de l'appel.
fn attempt_connect(
    settings_tx: &mpsc::Sender<EngineCommand>,
    sync_tx: &mpsc::Sender<SyncCommand>,
    status: &Arc<ArcSwap<AuthStatus>>,
    proxy: &EventLoopProxy<UserEvent>,
) -> Result<(), String> {
    if let Some(token) = overlay_sync::token_store::load_token() {
        match overlay_sync::client::fetch_settings(&token) {
            Ok(settings) => {
                // Nombre d'entrées loggé (pas seulement « récupéré ») — diagnostic ajouté après un
                // retour utilisateur 2026-09-02 (Suivi resté vide au tout premier lancement) : sans
                // ça, impossible de savoir si le problème vient d'une réponse déjà vide ou d'une
                // course entre son application et le premier redessin du Suivi (voir
                // `force_refresh`).
                tracing::info!(
                    "[compte] réglages récupérés depuis le jeton natif déjà connu ({} entrée(s) de suivi).",
                    settings.watchlist.len()
                );
                let _ = settings_tx.send(EngineCommand::ApplySettings(settings));
                activate_sync_queue(&token, sync_tx);
                return Ok(());
            }
            Err(err) => {
                tracing::warn!(
                    "[compte] jeton natif invalide/expiré ({err}) — nouvel appairage nécessaire."
                );
                overlay_sync::token_store::clear_token();
            }
        }
    }

    let token = match overlay_sync::pair_and_wait(|handle| {
        tracing::info!("=== Connexion du compte (optionnelle) ===");
        tracing::info!(
            "Ouvre {} et entre le code : {}",
            handle.verification_url,
            handle.pairing_code
        );
        tracing::info!(
            "(l'overlay fonctionne aussi sans compte lié — repli sur la classe détectée automatiquement)"
        );
        // Voir `AuthStatus::PairingStarted` : c'est ce qui rend le code visible directement dans
        // la fenêtre overlay, pas seulement dans ces logs.
        status.store(Arc::new(AuthStatus::PairingStarted {
            pairing_code: handle.pairing_code.clone(),
            verification_url: handle.verification_url.clone(),
        }));
        let _ = proxy.send_event(UserEvent::AuthStatusChanged);
    }) {
        Ok(token) => token,
        Err(err) => {
            tracing::warn!(
                "[compte] appairage non complété ({err}) — l'overlay continue sans roster."
            );
            // Le message le plus utile ici précise que la requête de DÉPART (obtenir un code) a
            // échoué — donc qu'aucun navigateur n'a pu s'ouvrir (retour utilisateur 2026-09-01 :
            // « il devrait ouvrir le navigateur... rien ne se passe ») : ce n'est pas un appairage
            // abandonné/expiré après affichage d'un code, l'échec est plus en amont.
            return Err(format!("impossible de démarrer l'appairage ({err})"));
        }
    };

    if let Err(err) = overlay_sync::token_store::save_token(&token) {
        // `warn!` : un jeton non sauvegardé fait silencieusement recommencer l'appairage à chaque
        // lancement, ça DOIT être vu — jamais le jeton lui-même dans ce message (§10 du plan).
        tracing::warn!(
            "[compte] échec de sauvegarde du jeton natif ({err}) — sera redemandé au prochain lancement."
        );
    }
    match overlay_sync::client::fetch_settings(&token) {
        Ok(settings) => {
            tracing::info!(
                "[compte] connecté — réglages récupérés ({} entrée(s) de suivi).",
                settings.watchlist.len()
            );
            let _ = settings_tx.send(EngineCommand::ApplySettings(settings));
            activate_sync_queue(&token, sync_tx);
            Ok(())
        }
        Err(err) => {
            tracing::warn!("[compte] échec de récupération des réglages après appairage ({err}).");
            Err(format!("réglages injoignables après appairage ({err})"))
        }
    }
}

/// Résout l'`uid` (`GET /api/v1/auth/me`, voir `overlay_sync::client::fetch_account_id`) et active
/// la file de synchro (L5, §7.1/§7.3 du plan) — appelé après CHAQUE connexion réussie
/// (`attempt_connect`, jeton déjà connu ou tout juste obtenu par appairage). Best-effort et jamais
/// fatal, comme le reste de cette fonction : un échec ici laisse simplement la file inactive cette
/// session (roster/watchlist restent pleinement fonctionnels, seul l'historique ne remonte pas au
/// compte) plutôt que de faire échouer toute la connexion pour un besoin annexe.
fn activate_sync_queue(token: &str, sync_tx: &mpsc::Sender<SyncCommand>) {
    match overlay_sync::client::fetch_account_id(token) {
        Ok(uid) => {
            let _ = sync_tx.send(SyncCommand::Activate {
                uid,
                token: token.to_string(),
            });
        }
        Err(err) => {
            tracing::warn!(
                %err,
                "[compte] identifiant introuvable (/api/v1/auth/me) — historique non synchronisé cette session."
            );
        }
    }
}

fn resolve_path() -> PathBuf {
    if let Some(arg) = env::args().nth(1) {
        return PathBuf::from(arg);
    }
    match discovery::discover() {
        Some(path) => path,
        None => {
            tracing::error!("wakfu.log introuvable aux emplacements connus. Chemins essayés :");
            for candidate in discovery::candidate_paths() {
                tracing::error!("  - {}", candidate.display());
            }
            tracing::error!(
                "Précisez le chemin explicitement : cargo run -p overlay-ui -- <chemin>"
            );
            logging::log_session_end("échec de démarrage (wakfu.log introuvable)");
            std::process::exit(1);
        }
    }
}

fn main() {
    let log_dir = logging::init();
    logging::install_ctrlc_handler();
    if let Some(dir) = &log_dir {
        tracing::info!("journal de session : {}", dir.display());
    }

    let log_path = resolve_path();
    let snapshot = Arc::new(ArcSwap::from_pointee(SessionSnapshot::default()));
    let watchlist = Arc::new(ArcSwap::from_pointee(Vec::<WatchlistEntry>::new()));
    let watchlist_toast = Arc::new(ArcSwap::from_pointee(None::<WatchlistToast>));
    let catalog = Arc::new(ArcSwap::from_pointee(CatalogIndex::default()));
    let catalog_stale = Arc::new(AtomicBool::new(false));
    let dungeons = Arc::new(ArcSwap::from_pointee(DungeonIndex::default()));
    let auth_status = Arc::new(ArcSwap::from_pointee(AuthStatus::Connecting));

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("création de l'event loop");
    let proxy = event_loop.create_proxy();
    let (settings_tx, settings_rx) = mpsc::channel();
    let (auth_command_tx, auth_command_rx) = mpsc::channel();
    let (sync_tx, sync_rx) = mpsc::channel();
    spawn_sync_thread(sync_rx);
    spawn_auth_thread(
        settings_tx.clone(),
        sync_tx.clone(),
        Arc::clone(&auth_status),
        auth_command_rx,
        proxy.clone(),
    );
    spawn_catalog_thread(
        Arc::clone(&catalog),
        Arc::clone(&catalog_stale),
        proxy.clone(),
    );
    spawn_dungeon_thread(Arc::clone(&dungeons), proxy.clone());
    let remote_icons = RemoteIconStore::spawn(proxy.clone());
    spawn_engine_thread(
        log_path.clone(),
        EngineHandles {
            snapshot: Arc::clone(&snapshot),
            watchlist: Arc::clone(&watchlist),
            watchlist_toast: Arc::clone(&watchlist_toast),
            catalog: Arc::clone(&catalog),
            dungeons,
        },
        proxy,
        settings_rx,
        sync_tx,
    );

    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(AppState {
        log_path,
        snapshot,
        watchlist,
        watchlist_toast,
        catalog,
        catalog_stale,
        remote_icons,
        auth_status,
        auth_command_tx,
        settings_tx,
    });
    event_loop.run_app(&mut app).expect("boucle d'événements");
}
