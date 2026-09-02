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
//! Volontairement incomplet par rapport à §9 du plan : pas encore de panneau Suivi/Alertes/État de
//! synchro (ceux-là dépendent soit de `StatsStoreService` non porté — §14 point 3 — soit de la
//! synchro serveur, L5), pas de disposition persistée par écran, pas de thème configurable. Le
//! récap de session reste également **global** (identique sur toutes les fenêtres, pas ventilé par
//! personnage — limitation connue, voir le plan) : ce sont les deux panneaux atteignables avec
//! `overlay-engine` tel qu'il existe aujourd'hui.

mod alert_sound;
mod game_window;
mod panels;
mod portraits;
mod remote_icons;
mod ui_icons;

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use arc_swap::ArcSwap;
use crossbeam_channel::RecvTimeoutError;
use egui_wgpu::wgpu;
use game_window::{GameRect, GameWindowTracker};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use overlay_engine::{CatalogIndex, Engine, FightSnapshot, SessionSnapshot, WatchlistEntry};
use overlay_ingest::discovery;
use overlay_sync::AccountSettings;
use panels::combat::CombatSide;
use panels::watchlist::WatchlistToast;
use portraits::PortraitAtlas;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use remote_icons::{RemoteIconStore, RemoteIconTextures};
use ui_icons::UiIcons;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE,
    HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
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
/// Voir `App::sync_topmost`.
const TOPMOST_REASSERT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);
// Largeur élargie 360 -> 420 (2026-09-01) pour laisser la place au portrait de classe (40px,
// voir portraits.rs) sans écraser le nom/les dégâts — réglage fin de la mise en page toujours à
// faire.
const WINDOW_SIZE: (f64, f64) = (420.0, 480.0);
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
/// le contenu demande (`content_width`, croît avec le nombre d'entrées) et le plafond
/// (`WATCHLIST_WIDTH_FRACTION` de la fenêtre de jeu, `WATCHLIST_MAX_CEILING`) — jamais l'inverse :
/// avec peu d'entrées, la fenêtre reste étroite même si le plafond est large.
fn watchlist_target_width(entry_count: usize, game_width_px: i32) -> f64 {
    let ceiling = (game_width_px as f64 * WATCHLIST_WIDTH_FRACTION).min(WATCHLIST_MAX_CEILING);
    let content = panels::watchlist::content_width(entry_count) as f64 + WATCHLIST_INNER_MARGIN;
    content.min(ceiling).max(WATCHLIST_INNER_MARGIN)
}
/// Marge, en pixels physiques, entre le bord gauche visible de la fenêtre de jeu et le bord
/// gauche de l'overlay Combat — « collé à quelques pixels près » (demande utilisateur). À ajuster
/// après avoir vu le rendu en pratique.
const GAME_EDGE_MARGIN_PX: i32 = 12;
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

/// Zone d'overlay indépendante ancrée sur une même fenêtre de jeu — demande utilisateur explicite
/// (2026-09-01) : Combat et Suivi doivent être deux fenêtres RÉELLEMENT séparées (pas seulement
/// deux panneaux dans la même fenêtre), pour permettre à terme de piloter leur visibilité
/// indépendamment (manuellement ou par un mécanisme automatique) — voir §9 du plan, qui vise à
/// terme autant de zones indépendantes que de panneaux. `OverlayWindow` reste un seul type partagé
/// (position/topmost/redraw sont identiques pour les deux) : seul `kind` distingue la taille,
/// l'ancrage (`App::anchor_position`) et le contenu rendu (`render`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayKind {
    Combat,
    Watchlist,
}

/// Émis par le thread Engine (§3 du plan) ou le thread Auth (`spawn_auth_thread`) quand un nouvel
/// état est disponible — réveille le main thread, en `ControlFlow::Wait` le reste du temps (§6.1 :
/// pas de boucle 60 Hz forcée, l'overlay ne consomme rien tant que rien ne change).
enum UserEvent {
    NewSnapshot,
    AuthStatusChanged,
}

/// État de la connexion au compte (lot L4, §7.2 du plan) — publié par le thread Auth
/// (`spawn_auth_thread`) via `Arc<ArcSwap<_>>`, lu par le main thread à chaque frame pour décider
/// d'afficher ou non l'icône de relance d'appairage (voir `render`). Volontairement distinct d'un
/// simple `bool` : `Connecting` évite d'afficher l'icône pendant la toute première tentative
/// (jeton déjà stocké, ou premier appairage) — elle ne doit apparaître qu'après un échec avéré.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AuthStatus {
    Connecting,
    Connected,
    /// Ni jeton valide ni appairage complété — l'icône de relance doit être visible (retour
    /// utilisateur 2026-09-01 : appairage en échec — 405 côté serveur — sans aucun moyen de
    /// retenter sans relancer tout le logiciel).
    ///
    /// `reason` porte le message d'erreur de la DERNIÈRE tentative (`attempt_connect`) — affiché
    /// en tooltip sur l'icône de relance (voir `render`) : un clic qui ne se traduit par rien de
    /// visible (le serveur refuse la requête AVANT même qu'un code d'appairage existe, donc aucun
    /// navigateur ne s'ouvre) est indiscernable d'un bouton cassé sans ce message — retour
    /// utilisateur 2026-09-01 : « l'appui du bouton ne déclenche rien, pas de message d'erreur
    /// dans la console » — le message existait déjà (console), seulement invisible pour qui ne
    /// regarde pas un terminal ; il l'est maintenant aussi directement dans l'overlay.
    Disconnected {
        reason: String,
    },
}

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    egui_ctx: egui::Context,
    egui_winit: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

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
    /// État `HWND_TOPMOST`/`HWND_NOTOPMOST` déjà appliqué — évite un `SetWindowPos` par tick pour
    /// rien (voir `App::sync_topmost`).
    is_topmost: bool,
    /// Dernière réaffirmation PÉRIODIQUE de `HWND_TOPMOST` (voir `App::sync_topmost` et
    /// `TOPMOST_REASSERT_INTERVAL`) — distincte d'un changement d'état détecté (`is_topmost`),
    /// qui reste réaffirmé immédiatement quel que soit ce champ.
    last_topmost_reassert: Option<std::time::Instant>,
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
    /// Un seul thread/état de téléchargement d'icônes PARTAGÉ par toutes les fenêtres (voir
    /// `remote_icons::RemoteIconStore`) — chaque fenêtre garde son propre cache de textures déjà
    /// uploadées (`OverlayWindow::remote_icon_textures`), mais ne retélécharge jamais une icône
    /// qu'une AUTRE fenêtre a déjà demandée.
    remote_icons: RemoteIconStore,
    /// Publié par le thread Auth (voir `spawn_auth_thread`) — piloté l'affichage de l'icône de
    /// relance d'appairage (`render`).
    auth_status: Arc<ArcSwap<AuthStatus>>,
    /// Signale au thread Auth qu'un appairage doit être retenté (clic sur l'icône de relance,
    /// visible uniquement quand `auth_status` vaut `Disconnected` — voir `render`).
    auth_retry_tx: mpsc::Sender<()>,
    /// Voir la doc de `AppState::settings_tx` et `force_refresh`.
    settings_tx: mpsc::Sender<AccountSettings>,
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
    remote_icons: RemoteIconStore,
    auth_status: Arc<ArcSwap<AuthStatus>>,
    auth_retry_tx: mpsc::Sender<()>,
    /// Conservé (pas seulement transmis au thread Auth) pour permettre à `force_refresh` de
    /// redemander les réglages de compte à la volée — voir sa doc.
    settings_tx: mpsc::Sender<AccountSettings>,
}

impl App {
    fn new(state: AppState) -> Self {
        let AppState {
            log_path,
            snapshot,
            watchlist,
            watchlist_toast,
            catalog,
            remote_icons,
            auth_status,
            auth_retry_tx,
            settings_tx,
        } = state;

        let hotkey_manager = GlobalHotKeyManager::new().expect("création GlobalHotKeyManager");
        let toggle_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW);
        let refresh_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyR);
        let quit_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyQ);
        hotkey_manager
            .register(toggle_hotkey)
            .expect("enregistrement du hotkey global");
        hotkey_manager
            .register(refresh_hotkey)
            .expect("enregistrement du hotkey de rafraîchissement");
        hotkey_manager
            .register(quit_hotkey)
            .expect("enregistrement du hotkey de sortie");

        Self {
            windows: HashMap::new(),
            hotkey_manager,
            hotkey_events: GlobalHotKeyEvent::receiver(),
            toggle_hotkey_id: toggle_hotkey.id(),
            refresh_hotkey_id: refresh_hotkey.id(),
            quit_hotkey_id: quit_hotkey.id(),
            interactive: true,
            snapshot,
            watchlist,
            watchlist_toast,
            catalog,
            remote_icons,
            auth_status,
            auth_retry_tx,
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
                println!(
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
                println!("[fenêtre de jeu] {character_name} trouvée — overlay {kind:?} créé.");
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
            OverlayKind::Watchlist => (watchlist_target_width(0, rect.width), WATCHLIST_HEIGHT),
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
            eprintln!("set_cursor_hittest a échoué à la création : {err}");
        }

        let gpu = pollster::block_on(init_gpu(Arc::clone(&window)));
        let portraits = PortraitAtlas::load(&gpu.egui_ctx);
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
            println!(
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
            is_topmost: true, // WindowLevel::AlwaysOnTop déjà appliqué ci-dessus à la création
            last_topmost_reassert: None,
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

    fn toggle_interactive(&mut self) {
        self.interactive = !self.interactive;
        for overlay in self.windows.values() {
            if let Err(err) = overlay.window.set_cursor_hittest(self.interactive) {
                eprintln!("set_cursor_hittest a échoué : {err}");
            }
            overlay.window.request_redraw();
        }
        println!(
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
    /// Ne redétecte PAS les fenêtres de jeu elles-mêmes (`sync_windows` le fait déjà en continu,
    /// ~20 Hz, voir `about_to_wait`) — force : (a) le prochain redessin de CHAQUE fenêtre
    /// (recalcule au passage la largeur du Suivi, voir `WindowEvent::RedrawRequested`), (b) une
    /// réaffirmation topmost IMMÉDIATE (bypass `TOPMOST_REASSERT_INTERVAL`, voir `sync_topmost`),
    /// et (c) depuis le retour utilisateur du 2026-09-02 (Suivi resté vide en tout début de
    /// session malgré plusieurs `Ctrl+Alt+R`), une NOUVELLE tentative de récupération des réglages
    /// de compte (roster + suivi) — un redessin seul ne peut rien montrer si `watchlist` (l'
    /// `ArcSwap` publié par le thread Engine, voir `spawn_engine_thread`) n'a en réalité jamais
    /// reçu les entrées suivies (jeton pas encore chargé, requête réseau pas encore aboutie au
    /// moment du tout premier lot de réglages). Non bloquant : lancé sur un thread éphémère dédié,
    /// jamais depuis ce thread (winit) ni le thread Engine.
    fn force_refresh(&mut self) {
        for overlay in self.windows.values_mut() {
            overlay.last_topmost_reassert = None;
            overlay.window.request_redraw();
        }
        self.sync_topmost();
        let settings_tx = self.settings_tx.clone();
        thread::spawn(move || match overlay_sync::token_store::load_token() {
            Some(token) => match overlay_sync::client::fetch_settings(&token) {
                Ok(settings) => {
                    println!(
                        ">>> Réglages de compte redemandés ({REFRESH_HOTKEY_LABEL}) : {} entrée(s) de suivi."
                        , settings.watchlist.len()
                    );
                    let _ = settings_tx.send(settings);
                }
                Err(err) => {
                    println!(">>> Échec de la nouvelle demande de réglages ({REFRESH_HOTKEY_LABEL}) : {err}");
                }
            },
            None => println!(
                ">>> Aucun jeton de compte stocké — rien à redemander ({REFRESH_HOTKEY_LABEL})."
            ),
        });
        println!(">>> Rafraîchissement forcé ({REFRESH_HOTKEY_LABEL})");
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
            // strictement gratuit, autant l'éviter sur des heures de jeu). La transition vers
            // NOTOPMOST, elle, reste optimisée comme avant (pas cette même urgence).
            let transitioned = overlay.is_topmost != relevant;
            let due_for_reassert = relevant
                && overlay
                    .last_topmost_reassert
                    .is_none_or(|t| now.duration_since(t) >= TOPMOST_REASSERT_INTERVAL);
            if !transitioned && !due_for_reassert {
                continue;
            }

            let insert_after = if relevant {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            };
            let hwnd = Self::hwnd_of(&overlay.window);
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    Some(insert_after),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            overlay.is_topmost = relevant;
            if relevant {
                overlay.last_topmost_reassert = Some(now);
            }
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.sync_windows(event_loop);
        if !self.banner_printed {
            println!("=== wakfu-companion-overlay (L2, overlay-ui) ===");
            println!("Suivi de {}", self.log_path.display());
            println!(
                "{HOTKEY_LABEL} pour basculer interactif / clic-traversant. \
                 {REFRESH_HOTKEY_LABEL} pour forcer un rafraîchissement (overlay bloqué/mal \
                 positionné, ou Suivi resté vide). {QUIT_HOTKEY_LABEL} ou Ctrl+C (dans ce \
                 terminal) pour quitter.\n"
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
            WindowEvent::CloseRequested => event_loop.exit(),
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
                // Clamp défensif (voir S1, README.md §"Bug de redimensionnement") : un resize
                // excessif ne doit jamais faire planter l'overlay, quelle qu'en soit la cause.
                let max_dim = overlay.gpu.device.limits().max_texture_dimension_2d;
                overlay.gpu.config.width = size.width.min(max_dim);
                overlay.gpu.config.height = size.height.min(max_dim);
                overlay
                    .gpu
                    .surface
                    .configure(&overlay.gpu.device, &overlay.gpu.config);
            }
            WindowEvent::RedrawRequested => {
                let snapshot = self.snapshot.load();
                let fight = snapshot.fight_for_character(&overlay.character_name);
                let watchlist = self.watchlist.load();
                if overlay.kind == OverlayKind::Watchlist {
                    // Largeur pilotée par le CONTENU (retour utilisateur 2026-09-02 : une fenêtre
                    // plus large que nécessaire reste cliquable/bloquante sur toute sa zone même
                    // transparente, l'utilisateur ne peut alors pas deviner où s'arrête l'overlay)
                    // — voir la doc de `watchlist_target_width`. Comparée à la dernière largeur
                    // DEMANDÉE (`last_watchlist_width`), pas à la taille réelle actuelle de la
                    // fenêtre : `request_inner_size` sur Windows n'est pas garanti instantané,
                    // comparer à la taille réelle redemanderait inutilement tant que le compositeur
                    // n'a pas fini d'appliquer la précédente demande.
                    let target = watchlist_target_width(watchlist.len(), overlay.game_rect.width);
                    if overlay.last_watchlist_width != Some(target) {
                        let _ = overlay
                            .window
                            .request_inner_size(winit::dpi::LogicalSize::new(
                                target,
                                WATCHLIST_HEIGHT,
                            ));
                        overlay.last_watchlist_width = Some(target);
                    }
                }
                let watchlist_toast_guard = self.watchlist_toast.load();
                let watchlist_toast: Option<&WatchlistToast> = (**watchlist_toast_guard).as_ref();
                let catalog = self.catalog.load();
                let auth_status = self.auth_status.load();
                let repaint_delay = render(
                    &mut overlay.gpu,
                    &overlay.window,
                    RenderContent {
                        kind: overlay.kind,
                        fight,
                        portraits: &overlay.portraits,
                        icons: &overlay.icons,
                        combat_side: &mut overlay.combat_side,
                        watchlist: &watchlist,
                        watchlist_toast,
                        catalog: &catalog,
                        remote_icons: &self.remote_icons,
                        remote_icon_textures: &mut overlay.remote_icon_textures,
                        auth_status: &auth_status,
                        auth_retry_tx: &self.auth_retry_tx,
                    },
                );
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
                self.force_refresh();
            } else if event.id == self.quit_hotkey_id {
                println!(">>> Sortie ({QUIT_HOTKEY_LABEL})");
                event_loop.exit();
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
    println!("Adaptateur GPU : {:?}", adapter.get_info());

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
    // Délai de tooltip par défaut d'egui (0,5s, `show_tooltips_only_when_still=true` : le
    // minuteur repart de zéro à chaque micro-mouvement de la souris, pas seulement au premier
    // survol) — trop long/imprévisible dans cette architecture SANS boucle de rendu continue
    // (§6.1) : chaque redessin dépend du réveil `about_to_wait`/`next_redraw_at`, qui n'apporte
    // qu'une granularité de 50ms au mieux, jamais un vrai 60Hz qui masquerait la latence. Retour
    // utilisateur 2026-09-02 : « il faut bien quasiment quatre, cinq secondes avant que la
    // tooltip s'affiche » — `show_tooltips_only_when_still` désactivé (affichée dès le survol,
    // sans exiger une souris parfaitement immobile) et le délai réduit à 150ms (perceptible comme
    // quasi immédiat, tout en évitant un flash sur un simple passage de souris).
    // `style_mut_of` (par thème, egui 0.36) plutôt que `style_mut` (retiré) — appliqué aux DEUX
    // thèmes : ce réglage ne touche qu'à l'interaction, pas aux couleurs (seul le thème sombre est
    // par ailleurs reproduit ici, voir `panels::combat::ACCENT`), autant ne pas dépendre de celui
    // qu'egui choisit par défaut.
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        egui_ctx.style_mut_of(theme, |style| {
            style.interaction.show_tooltips_only_when_still = false;
            style.interaction.tooltip_delay = 0.15;
        });
    }
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

/// Regroupe les paramètres de `render` au-delà de `gpu`/`window` — sinon `too_many_arguments`
/// (clippy), la fonction ayant crû à mesure que le panneau Combat (icônes, camp affiché) et
/// l'icône de relance d'appairage (statut de connexion, canal de retentative) s'y sont ajoutés.
struct RenderContent<'a> {
    kind: OverlayKind,
    fight: Option<&'a FightSnapshot>,
    portraits: &'a PortraitAtlas,
    icons: &'a UiIcons,
    combat_side: &'a mut CombatSide,
    watchlist: &'a [WatchlistEntry],
    watchlist_toast: Option<&'a WatchlistToast>,
    catalog: &'a CatalogIndex,
    remote_icons: &'a RemoteIconStore,
    remote_icon_textures: &'a mut RemoteIconTextures,
    auth_status: &'a AuthStatus,
    auth_retry_tx: &'a mpsc::Sender<()>,
}

/// **Refonte 2026-09-01** (retour utilisateur, capture d'écran à l'appui) : le nom du personnage,
/// l'état interactif/clic-traversant et le rappel du raccourci n'apportaient rien (l'utilisateur
/// sait déjà quel personnage est le sien et derrière quelle fenêtre de jeu il joue) — retirés, de
/// même que le titre "Dégâts du combat" (voir `panels::combat`) et toute la section "Récap de
/// session" (Kamas/XP/Combats/Butin — retour utilisateur : la garder n'a plus de sens une fois le
/// reste simplifié, sera repensée dans un autre chantier). Le fond opaque du panneau (une grande
/// plaque sombre visible même quand il n'y a presque rien à afficher, voir la capture) est
/// également retiré : `Frame::NONE`, seuls les widgets eux-mêmes restent visibles par-dessus le
/// jeu.
///
/// Renvoie le délai de redessin demandé par egui pour CETTE fenêtre (`ViewportOutput::
/// repaint_delay`, ex. le délai d'apparition d'une tooltip) — voir `OverlayWindow::next_redraw_at`
/// pour pourquoi l'appelant doit impérativement en tenir compte, cette architecture n'ayant pas de
/// boucle de rendu continue.
fn render(gpu: &mut GpuState, window: &Window, content: RenderContent<'_>) -> std::time::Duration {
    let RenderContent {
        kind,
        fight,
        portraits,
        icons,
        combat_side,
        watchlist,
        watchlist_toast,
        catalog,
        remote_icons,
        remote_icon_textures,
        auth_status,
        auth_retry_tx,
    } = content;

    let raw_input = gpu.egui_winit.take_egui_input(window);
    let mut full_output = gpu.egui_ctx.run_ui(raw_input, |ui| {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(6))
            .show(ui, |ui| match kind {
                // Zone Combat : dégâts du combat en cours + icône de connexion au compte. Cette
                // dernière reste ici (pas dans la zone Suivi) — ni l'une ni l'autre zone n'en est
                // propriétaire de façon évidente, mais Combat est la fenêtre "historique", la
                // moins perturbante à faire bouger encore une fois.
                OverlayKind::Combat => {
                    // Icône de relance d'appairage — visible UNIQUEMENT quand la connexion au
                    // compte a échoué (retour utilisateur 2026-09-01 : 405 côté serveur au premier
                    // appairage, aucun moyen de retenter sans relancer tout le logiciel), réduite
                    // au minimum et collée à droite (retour utilisateur : la barre pleine largeur
                    // précédente était trop imposante) — le libellé passe en tooltip. `reason`
                    // (message d'erreur de la dernière tentative) y est ajouté : un clic qui ne se
                    // traduit par rien de visible (le serveur refuse la requête avant même qu'un
                    // code d'appairage existe, donc aucun navigateur ne s'ouvre) est indiscernable
                    // d'un bouton cassé sans lui — retour utilisateur : « l'appui du bouton ne
                    // déclenche rien, pas de message d'erreur dans la console » (le message
                    // existait déjà, seulement en console). Un clic renvoie sur
                    // `spawn_auth_thread`, qui relance un appairage COMPLET (rouvre le navigateur
                    // avec un nouveau code, voir `overlay_sync::pair_and_wait`).
                    match auth_status {
                        AuthStatus::Disconnected { reason } => {
                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let retry = ui
                                            .add(egui::Button::new("🔌").small())
                                            .on_hover_text(format!(
                                                "Connecter le compte (relance l'appairage, ouvre \
                                                 le navigateur).\nDernier échec : {reason}"
                                            ));
                                        if retry.clicked() {
                                            let _ = auth_retry_tx.send(());
                                        }
                                    },
                                );
                            });
                            ui.add_space(4.0);
                        }
                        // Retour visuel qu'un clic a bien déclenché quelque chose — son absence
                        // donnait l'impression que le bouton ne faisait rien (retour utilisateur :
                        // « on dirait que ça ne fait rien »).
                        AuthStatus::Connecting => {
                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| ui.weak("Connexion…"),
                                );
                            });
                            ui.add_space(4.0);
                        }
                        AuthStatus::Connected => {}
                    }

                    panels::combat::show(
                        ui,
                        fight,
                        portraits,
                        icons,
                        catalog,
                        remote_icons,
                        remote_icon_textures,
                        combat_side,
                    );
                }
                // Zone Suivi — fenêtre INDÉPENDANTE de Combat (demande utilisateur explicite
                // 2026-09-01) : bande de tuiles façon `tracker-strip` du web, voir
                // `panels::watchlist`. Rien affiché tant que le compte ne déclare aucune entrée
                // (fenêtre transparente vide plutôt qu'un cadre vide disgracieux).
                OverlayKind::Watchlist => {
                    if !watchlist.is_empty() {
                        panels::watchlist::show(
                            ui,
                            icons,
                            catalog,
                            remote_icons,
                            remote_icon_textures,
                            watchlist,
                            watchlist_toast,
                        );
                    }
                }
            });
    });
    // Capturé AVANT de consommer `full_output` ci-dessous (tessellate/textures_delta le vident
    // progressivement) — voir la doc de `render` et `OverlayWindow::next_redraw_at` : c'est le
    // SEUL moyen d'honorer un délai de redessin demandé par egui (tooltip, animation...) dans
    // cette architecture sans boucle de rendu continue. Repli `Duration::MAX` ("pas de redessin
    // demandé") si jamais le viewport racine n'a pas d'entrée — ne devrait pas arriver en
    // pratique (une seule fenêtre racine par `egui::Context`, jamais de sous-viewport ici).
    let mut repaint_delay = full_output
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .map_or(std::time::Duration::MAX, |viewport| viewport.repaint_delay);
    // Le toast d'alerte disparaît de lui-même après `WatchlistToast::hide_at` (voir sa doc) — sans
    // ceci, rien ne redéclencherait de redessin à cette échéance dans cette architecture sans
    // boucle continue (§6.1), le toast resterait affiché indéfiniment jusqu'au prochain redessin
    // dû à une AUTRE cause.
    if let Some(toast) = watchlist_toast {
        let now = std::time::Instant::now();
        if toast.hide_at > now {
            repaint_delay = repaint_delay.min(toast.hide_at - now);
        }
    }

    gpu.egui_winit
        .handle_platform_output(window, full_output.platform_output);

    let paint_jobs = gpu
        .egui_ctx
        .tessellate(full_output.shapes, full_output.pixels_per_point);

    // Traité inconditionnellement, *avant* toute sortie anticipée ci-dessous : `textures_delta`
    // doit être appliqué (set) puis libéré (free), et explicitement vidé (`clear`) ensuite, quel
    // que soit le sort de cette frame — `TexturesDelta` panique (`debug_assert!`, donc invisible
    // en `--release`, ce qui l'a caché jusqu'ici) si ses collections ne sont pas vides à sa
    // destruction, et les itérer par référence ne les vide pas.
    for (id, deltas) in &full_output.textures_delta.set {
        for delta in deltas {
            gpu.egui_renderer
                .update_texture(&gpu.device, &gpu.queue, *id, delta);
        }
    }
    for id in &full_output.textures_delta.free {
        gpu.egui_renderer.free_texture(id);
    }
    full_output.textures_delta.clear();

    let output_frame = match gpu.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(frame) => frame,
        wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
        wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
            return repaint_delay;
        }
        wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
            gpu.surface.configure(&gpu.device, &gpu.config);
            return repaint_delay;
        }
        wgpu::CurrentSurfaceTexture::Validation => {
            eprintln!("get_current_texture: erreur de validation");
            return repaint_delay;
        }
    };
    let view = output_frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let screen_descriptor = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [gpu.config.width, gpu.config.height],
        pixels_per_point: full_output.pixels_per_point,
    };

    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("overlay-ui-encoder"),
        });
    gpu.egui_renderer.update_buffers(
        &gpu.device,
        &gpu.queue,
        &mut encoder,
        &paint_jobs,
        &screen_descriptor,
    );

    {
        let mut render_pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("overlay-ui-egui-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                ..Default::default()
            })
            .forget_lifetime();
        gpu.egui_renderer
            .render(&mut render_pass, &paint_jobs, &screen_descriptor);
    }

    gpu.queue.submit(Some(encoder.finish()));
    gpu.queue.present(output_frame);
    repaint_delay
}

/// Thread Engine (§3 du plan) : lit `wakfu.log` en continu, alimente `overlay-engine`, publie
/// chaque nouveau `SessionSnapshot` par `ArcSwap` et réveille le main thread. Ne rappelle jamais
/// l'UI directement — l'UI ne lit que la dernière valeur publiée (§3 : « zéro verrou sur le
/// chemin de rendu »). Un seul thread/`Engine` pour toutes les fenêtres overlay (voir doc de
/// module) : `wakfu.log` est partagé par tous les clients, `SessionSnapshot::fights` porte déjà
/// tous les combats simultanés.
///
/// Reçoit aussi (lot L4) les réglages de compte récupérés par le thread Auth
/// (`spawn_auth_thread`) via `settings_rx` — appliqués à `Engine` de façon non bloquante entre
/// deux lots, jamais en attendant activement dessus (voir la sélection `recv_timeout` ci-dessous,
/// seule façon de sonder les DEUX canaux — lignes de log et réglages de compte — sans thread de
/// sondage dédié). Publie aussi `watchlist` en plus de `snapshot` : contrairement au roster (qui
/// n'influence l'affichage qu'indirectement, via la classe résolue au prochain `FighterJoined`),
/// les entrées suivies sont DIRECTEMENT affichées (`panels::watchlist`) — sans cette publication
/// immédiate, la liste resterait vide à l'écran jusqu'au prochain lot de lignes de log.
///
/// Publie aussi `watchlist_toast` (§9 du plan, « Alertes de drop ») dès qu'`Engine::
/// drain_watchlist_alerts` renvoie quelque chose après un lot ingéré — jamais depuis la boucle
/// `settings_rx` ci-dessus, `set_watchlist_entries`/`merge_config` ne peut par construction jamais
/// déclencher d'alerte (voir `overlay_engine::watchlist::WatchlistState::merge_config`, qui
/// n'appelle jamais `increment`). Le son est joué sur son propre thread éphémère
/// (`alert_sound::play_countdown_alert`), jamais en bloquant CE thread — bloquer ici retarderait
/// l'ingestion du log pour tous les personnages.
fn spawn_engine_thread(
    log_path: PathBuf,
    snapshot: Arc<ArcSwap<SessionSnapshot>>,
    watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
    watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
    catalog: Arc<ArcSwap<CatalogIndex>>,
    proxy: EventLoopProxy<UserEvent>,
    settings_rx: mpsc::Receiver<AccountSettings>,
) {
    thread::Builder::new()
        .name("overlay-engine".into())
        .spawn(move || {
            let mut engine = match Engine::new() {
                Ok(engine) => engine,
                Err(err) => {
                    eprintln!("[erreur fatale] création de l'Engine QuickJS : {err}");
                    return;
                }
            };
            // Dernier catalogue déjà transmis à l'Engine (voir `Engine::set_catalog`) — comparé par
            // pointeur à chaque tick pour ne relayer qu'un VRAI changement (`spawn_catalog_thread`
            // republie via `ArcSwap::store`, jamais une mutation en place). Protège
            // `hostIsKnownMonsterName` (voir `quickjs_engine.rs`) contre un vrai monstre qui se
            // révèle (mimique, brèche) confondu à tort avec une invocation — retour utilisateur
            // 2026-09-02.
            let mut last_seen_catalog: Option<Arc<CatalogIndex>> = None;
            let rx = overlay_ingest::watcher::spawn(&log_path);
            loop {
                // Non bloquant : n'attend jamais activement les réglages de compte, seulement les
                // lignes de log (voir recv_timeout plus bas) — un compte jamais lié ne doit pas
                // retarder l'ingestion d'un seul milliseconde.
                while let Ok(settings) = settings_rx.try_recv() {
                    let entry_count = settings.watchlist.len();
                    tracing::info!(
                        entry_count,
                        "réglages de compte appliqués à l'Engine (lot L4)"
                    );
                    engine.set_roster(Some(settings.roster));
                    engine.set_watchlist_entries(settings.watchlist);
                    watchlist.store(Arc::new(engine.watchlist_entries().to_vec()));
                    let _ = proxy.send_event(UserEvent::NewSnapshot);
                }
                let current_catalog = catalog.load_full();
                let already_seen = last_seen_catalog
                    .as_ref()
                    .is_some_and(|seen| Arc::ptr_eq(seen, &current_catalog));
                if !already_seen {
                    engine.set_catalog(Arc::clone(&current_catalog));
                    last_seen_catalog = Some(current_catalog);
                }
                match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                    Ok(Ok(batch)) => {
                        if let Err(err) = engine.ingest_batch(&batch) {
                            tracing::warn!(%err, "échec d'ingestion d'un lot, ligne(s) ignorée(s)");
                            continue;
                        }
                        snapshot.store(Arc::new(engine.snapshot()));
                        watchlist.store(Arc::new(engine.watchlist_entries().to_vec()));
                        for alert in engine.drain_watchlist_alerts() {
                            tracing::info!(name = %alert.name, "alerte de suivi (décompte à 0)");
                            alert_sound::play_countdown_alert();
                            watchlist_toast.store(Arc::new(Some(WatchlistToast {
                                name: alert.name,
                                kind: alert.kind,
                                hide_at: std::time::Instant::now()
                                    + panels::watchlist::TOAST_DURATION,
                            })));
                        }
                        let _ = proxy.send_event(UserEvent::NewSnapshot);
                    }
                    Ok(Err(err)) => tracing::warn!(%err, "erreur de lecture de wakfu.log"),
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break, // watcher arrêté (process en fin de vie)
                }
            }
        })
        .expect("échec de création du thread Engine");
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
/// plutôt que de rester sur un `CatalogIndex::default()` vide. Pas encore de bandeau « catalogue
/// daté » visible dans l'UI pour signaler ce cas (§9 du plan ne liste pas encore un tel indicateur
/// parmi les panneaux) — seulement un `tracing::warn!`, à reprendre par un futur lot L2.
fn spawn_catalog_thread(catalog: Arc<ArcSwap<CatalogIndex>>, proxy: EventLoopProxy<UserEvent>) {
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

/// **Boucle de retentative** (2026-09-01, retour utilisateur : appairage en échec — 405 côté
/// serveur — sans aucun moyen de retenter sans relancer tout le logiciel) : une tentative échouée
/// (`attempt_connect` renvoie `Err(raison)`) publie `AuthStatus::Disconnected { reason }` (voir
/// `status`) plutôt que de laisser le thread mourir — `render` en déduit l'icône de relance (dont
/// le tooltip affiche `reason`), et son clic pousse dans `retry_rx` pour reprendre cette boucle.
/// Toujours pas d'UI de pairing complète dans la fenêtre overlay (hors périmètre de cette
/// itération, voir le panneau "État de synchro" du plan §9, toujours à construire) : le code
/// d'appairage reste affiché en console, seuls le déclenchement d'une nouvelle tentative et la
/// raison du dernier échec sont maintenant visibles depuis l'overlay.
fn spawn_auth_thread(
    settings_tx: mpsc::Sender<AccountSettings>,
    status: Arc<ArcSwap<AuthStatus>>,
    retry_rx: mpsc::Receiver<()>,
    proxy: EventLoopProxy<UserEvent>,
) {
    thread::Builder::new()
        .name("overlay-auth".into())
        .spawn(move || loop {
            status.store(Arc::new(AuthStatus::Connecting));
            let _ = proxy.send_event(UserEvent::AuthStatusChanged);

            let result = attempt_connect(&settings_tx);

            let connected = result.is_ok();
            status.store(Arc::new(match result {
                Ok(()) => AuthStatus::Connected,
                Err(reason) => AuthStatus::Disconnected { reason },
            }));
            let _ = proxy.send_event(UserEvent::AuthStatusChanged);

            if connected {
                return; // rien de plus à faire — même comportement qu'avant cette itération
            }
            // Attend un clic sur l'icône de relance (voir `render`) avant de retenter — jamais de
            // nouvelle tentative automatique en boucle, ce serait spammer le serveur/le navigateur
            // pour un utilisateur qui n'a peut-être pas l'intention de lier son compte.
            if retry_rx.recv().is_err() {
                return; // App fermée (canal fermé avec l'émetteur) — rien à retenter.
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
/// en mode invité (repli `breed`, aucun suivi affiché).
fn attempt_connect(settings_tx: &mpsc::Sender<AccountSettings>) -> Result<(), String> {
    if let Some(token) = overlay_sync::token_store::load_token() {
        match overlay_sync::client::fetch_settings(&token) {
            Ok(settings) => {
                // Nombre d'entrées loggé (pas seulement « récupéré ») — diagnostic ajouté après un
                // retour utilisateur 2026-09-02 (Suivi resté vide au tout premier lancement) : sans
                // ça, impossible de savoir si le problème vient d'une réponse déjà vide ou d'une
                // course entre son application et le premier redessin du Suivi (voir
                // `force_refresh`).
                println!(
                    "[compte] réglages récupérés depuis le jeton natif déjà connu ({} entrée(s) de suivi).",
                    settings.watchlist.len()
                );
                let _ = settings_tx.send(settings);
                return Ok(());
            }
            Err(err) => {
                println!(
                    "[compte] jeton natif invalide/expiré ({err}) — nouvel appairage nécessaire."
                );
                overlay_sync::token_store::clear_token();
            }
        }
    }

    let token = match overlay_sync::pair_and_wait(|handle| {
        println!("\n=== Connexion du compte (optionnelle) ===");
        println!(
            "Ouvre {} et entre le code : {}",
            handle.verification_url, handle.pairing_code
        );
        println!(
            "(l'overlay fonctionne aussi sans compte lié — repli sur la classe détectée automatiquement)\n"
        );
    }) {
        Ok(token) => token,
        Err(err) => {
            println!("[compte] appairage non complété ({err}) — l'overlay continue sans roster.");
            // Le message le plus utile ici précise que la requête de DÉPART (obtenir un code) a
            // échoué — donc qu'aucun navigateur n'a pu s'ouvrir (retour utilisateur 2026-09-01 :
            // « il devrait ouvrir le navigateur... rien ne se passe ») : ce n'est pas un appairage
            // abandonné/expiré après affichage d'un code, l'échec est plus en amont.
            return Err(format!("impossible de démarrer l'appairage ({err})"));
        }
    };

    if let Err(err) = overlay_sync::token_store::save_token(&token) {
        // Volontairement `eprintln!`, pas seulement `tracing::warn!` (invisible par défaut ici,
        // voir main() — aucun subscriber `tracing` installé, seulement `env_logger` pour la
        // façade `log`) : un jeton non sauvegardé fait silencieusement recommencer l'appairage à
        // chaque lancement, ça DOIT être vu.
        eprintln!(
            "[compte] échec de sauvegarde du jeton natif ({err}) — sera redemandé au prochain lancement."
        );
    }
    match overlay_sync::client::fetch_settings(&token) {
        Ok(settings) => {
            println!(
                "[compte] connecté — réglages récupérés ({} entrée(s) de suivi).",
                settings.watchlist.len()
            );
            let _ = settings_tx.send(settings);
            Ok(())
        }
        Err(err) => {
            println!("[compte] échec de récupération des réglages après appairage ({err}).");
            Err(format!("réglages injoignables après appairage ({err})"))
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
            eprintln!("wakfu.log introuvable aux emplacements connus. Chemins essayés :");
            for candidate in discovery::candidate_paths() {
                eprintln!("  - {}", candidate.display());
            }
            eprintln!("\nPrécisez le chemin explicitement : cargo run -p overlay-ui -- <chemin>");
            std::process::exit(1);
        }
    }
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,wgpu_hal=info"),
    )
    .init();

    let log_path = resolve_path();
    let snapshot = Arc::new(ArcSwap::from_pointee(SessionSnapshot::default()));
    let watchlist = Arc::new(ArcSwap::from_pointee(Vec::<WatchlistEntry>::new()));
    let watchlist_toast = Arc::new(ArcSwap::from_pointee(None::<WatchlistToast>));
    let catalog = Arc::new(ArcSwap::from_pointee(CatalogIndex::default()));
    let auth_status = Arc::new(ArcSwap::from_pointee(AuthStatus::Connecting));

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("création de l'event loop");
    let proxy = event_loop.create_proxy();
    let (settings_tx, settings_rx) = mpsc::channel();
    let (auth_retry_tx, auth_retry_rx) = mpsc::channel();
    spawn_auth_thread(
        settings_tx.clone(),
        Arc::clone(&auth_status),
        auth_retry_rx,
        proxy.clone(),
    );
    spawn_catalog_thread(Arc::clone(&catalog), proxy.clone());
    let remote_icons = RemoteIconStore::spawn(proxy.clone());
    spawn_engine_thread(
        log_path.clone(),
        Arc::clone(&snapshot),
        Arc::clone(&watchlist),
        Arc::clone(&watchlist_toast),
        Arc::clone(&catalog),
        proxy,
        settings_rx,
    );

    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(AppState {
        log_path,
        snapshot,
        watchlist,
        watchlist_toast,
        catalog,
        remote_icons,
        auth_status,
        auth_retry_tx,
        settings_tx,
    });
    event_loop.run_app(&mut app).expect("boucle d'événements");
}
