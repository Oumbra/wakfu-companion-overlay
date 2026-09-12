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
use overlay_ui::alert_sound;
use overlay_ui::config;
use overlay_ui::engine_thread::{
    spawn_engine_thread, EngineCommand, EngineHandles, SharedAlertProfile, SyncCommand,
};
use overlay_ui::frame::{recreate_surface, render, GpuState};
use overlay_ui::game_window::{GameRect, GameWindowTracker};
use overlay_ui::logging;
use overlay_ui::panels;
use overlay_ui::panels::alerts_tab;
use overlay_ui::panels::combat::CombatSide;
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::panels::options_modal::{self, OptionsModalAction, OptionsModalState};
use overlay_ui::panels::watchlist::WatchlistToast;
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content;
use overlay_ui::render_content::{AuthCommand, AuthStatus, OverlayKind, RenderContent, UserEvent};
use overlay_ui::ui_icons::UiIcons;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
    SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

#[cfg(target_os = "windows")]
use winit::platform::windows::WindowAttributesExtWindows;

const HOTKEY_LABEL: &str = "Ctrl+Shift+W";
/// Ctrl+Shift+R plutôt que F5 (suggestion initiale de l'utilisateur, 2026-09-02) : F5 est un
/// raccourci GLOBAL (`GlobalHotKeyManager`, jamais limité à une fenêtre précise malgré la demande
/// « quand on est focus sur une fenêtre de jeu ») — le voler à Wakfu (raccourcis de sort/action
/// fréquents sur les touches de fonction) ou à n'importe quelle autre appli au premier plan serait
/// activement nuisible. Même préfixe que `HOTKEY_LABEL` : cohérent, déjà éprouvé sans collision
/// connue avec le jeu. CTRL+ALT+R -> CTRL+SHIFT+R (retour utilisateur 2026-09-06) : harmonisé avec
/// `DETAILS_HOTKEY_LABEL` et consorts — seul `DISCONNECT_HOTKEY_LABEL` reste sur l'ancien préfixe.
const REFRESH_HOTKEY_LABEL: &str = "Ctrl+Shift+R";
/// Raccourci global de sortie (retour utilisateur 2026-09-02) : les fenêtres overlay portent
/// `WS_EX_NOACTIVATE` (voir `apply_extended_styles`, jamais désactivé même en mode interactif —
/// nécessaire pour ne jamais voler le focus au jeu) donc ne reçoivent JAMAIS `WindowEvent::
/// KeyboardInput`, quel que soit le mode : Échap (voir `window_event`) ne peut en pratique jamais
/// se déclencher, malgré ce qu'annonçait la bannière de démarrage. L'utilisateur devait donc
/// systématiquement faire un Ctrl+C dans le terminal (` STATUS_CONTROL_C_EXIT` en sortie — normal
/// dans ce cas, pas un plantage, mais peu clair). Même mécanisme que `HOTKEY_LABEL`/
/// `REFRESH_HOTKEY_LABEL` (hotkey GLOBAL, fonctionne sans focus sur aucune fenêtre précise) pour
/// vraiment permettre ce que la bannière annonce. CTRL+ALT+Q -> CTRL+SHIFT+Q (retour utilisateur
/// 2026-09-06), même changement que `HOTKEY_LABEL`/`REFRESH_HOTKEY_LABEL`.
const QUIT_HOTKEY_LABEL: &str = "Ctrl+Shift+Q";
/// Déconnexion volontaire du compte (lot L4, §7.2/§14 point 3 du plan) — jusqu'ici, révoquer une
/// session native depuis l'overlay exigeait d'aller effacer le jeton à la main sur disque/dans le
/// trousseau (aucun moyen depuis l'overlay lui-même). Même famille de raccourci GLOBAL que les
/// trois précédents ; ne fait rien de visible en mode invité (aucun compte lié) — voir
/// `App::disconnect_account`. Resté sur CTRL+ALT lors du passage de `HOTKEY_LABEL`/
/// `REFRESH_HOTKEY_LABEL`/`QUIT_HOTKEY_LABEL` à CTRL+SHIFT (2026-09-06, pas demandé par
/// l'utilisateur pour celui-ci) — seul raccourci encore sur l'ancien préfixe.
const DISCONNECT_HOTKEY_LABEL: &str = "Ctrl+Alt+D";

/// Raccourci global pour "Détails" (bouton lien externe, désormais dans le carré de contrôle de
/// `panels::watchlist::control_button_row` — voir sa doc, refonte 2026-09-08 : déplacé depuis
/// `panels::combat::bottom_toolbar`, retirée) — même action qu'un clic
/// (`open::that(overlay_sync::client::base_url())`, voir `App::open_details`). Retour utilisateur
/// explicite 2026-09-06 (« à l'image de ce qu'il y a dans le jeu [...] rajoute les raccourcis [...]
/// pour le détail [...] Ctrl+Shift+D ») : modificateur CTRL+SHIFT, combinaisons données par
/// l'utilisateur lui-même pour les cinq raccourcis de ce groupe — reflétées entre parenthèses dans
/// les tooltips correspondants (voir `panels::combat::paint_side_switch`,
/// `panels::watchlist::control_button_row`), à l'image du jeu. `HOTKEY_LABEL`/
/// `REFRESH_HOTKEY_LABEL`/`QUIT_HOTKEY_LABEL`, initialement en CTRL+ALT, ont rejoint ce même
/// préfixe CTRL+SHIFT le même jour (voir leur doc) ; seul `DISCONNECT_HOTKEY_LABEL` reste en
/// CTRL+ALT.
const DETAILS_HOTKEY_LABEL: &str = "Ctrl+Shift+D";
/// Raccourci global pour "Options" (`panels::watchlist::control_button_row`, voir sa doc — déplacé
/// depuis `panels::combat::bottom_toolbar`, refonte 2026-09-08) — n'ouvre encore aucun panneau,
/// comme le clic sur le bouton lui-même (voir sa doc) : réservé à une future page de réglages,
/// juste enregistré/journalisé pour l'instant (voir `about_to_wait`).
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
/// Voir `App::last_foreground_heartbeat`.
const FOREGROUND_HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3);
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
    /// État de la modale Options (2026-09-08, §9 du plan) — `Some` UNIQUEMENT pour `kind ==
    /// OverlayKind::Options`, voir `App::open_options_modal`. Même remarque que
    /// `bin/overlay-ui-x11.rs` (code partagé côté `panels::options_modal`, duplication assumée
    /// côté fenêtrage OS comme le reste de ce fichier).
    options_state: Option<OptionsModalState>,
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
    /// Le profil d'alerte du compte, tel que le dernier `GET /api/v1/settings` l'a rendu — lu à
    /// l'OUVERTURE de la fenêtre Options, pour en faire le brouillon de l'onglet « Alertes ».
    alert_profile: SharedAlertProfile,
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
    /// Diagnostic 2026-09-06 (retour utilisateur, `session_id=9956`) : le journal montrait une
    /// démotion `HWND_NOTOPMOST` jamais suivie d'aucune repromotion pendant plus de 3 minutes,
    /// jusqu'à la fin de la session — impossible de savoir, seulement à partir des transitions
    /// déjà journalisées (`sync_topmost`), si `GetForegroundWindow()` désignait réellement autre
    /// chose que le jeu pendant tout ce temps (l'utilisateur ayant réellement l'attention ailleurs)
    /// ou si la détection elle-même restait bloquée sur une valeur obsolète. Sert à borner un
    /// battement de coeur périodique (voir `sync_topmost`) qui journalise le titre de la fenêtre
    /// actuellement au premier plan — mais SEULEMENT tant qu'au moins un overlay reste
    /// `HWND_NOTOPMOST` (voir son appel), pour ne pas spammer le journal en usage normal.
    last_foreground_heartbeat: Option<std::time::Instant>,
    /// Dialogue de fichier natif (`rfd`) en cours, le cas échéant — voir `App::start_file_dialog`
    /// (2026-09-08, §9 du plan). Un seul à la fois (une seule modale Options peut être ouverte),
    /// sondé sans bloquer à chaque `about_to_wait`.
    pending_dialog: Option<mpsc::Receiver<Option<PathBuf>>>,
}

/// Écrit une durée d'alerte dans le champ de l'onglet « Alertes » — sans décimale inutile, et à la
/// virgule française que le champ accepte en entrée.
fn format_alert_duration(seconds: f32) -> String {
    if seconds.fract().abs() < f32::EPSILON {
        format!("{}", seconds as i64)
    } else {
        format!("{seconds:.1}").replace('.', ",")
    }
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
    alert_profile: SharedAlertProfile,
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
            alert_profile,
            catalog,
            catalog_stale,
            remote_icons,
            auth_status,
            auth_command_tx,
            settings_tx,
        } = state;

        let hotkey_manager = GlobalHotKeyManager::new().expect("création GlobalHotKeyManager");
        let toggle_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyW);
        let refresh_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyR);
        let quit_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyQ);
        let disconnect_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyD);
        // Ctrl+Shift comme les trois raccourcis ci-dessus — voir la doc de `DETAILS_HOTKEY_LABEL`
        // et consorts ; seul `disconnect_hotkey` reste en Ctrl+Alt.
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
            alert_profile,
            catalog,
            catalog_stale,
            remote_icons,
            auth_status,
            auth_command_tx,
            settings_tx,
            log_path,
            game_window: GameWindowTracker::new(),
            banner_printed: false,
            last_foreground_heartbeat: None,
            pending_dialog: None,
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
            // La modale Options (2026-09-08) n'est PAS rattachée à une fenêtre de jeu précise
            // (voir la doc de `OverlayWindow::options_state`) — jamais retirée par ce scan, sa
            // durée de vie est pilotée exclusivement par l'utilisateur (Annuler/Valider), voir
            // `window_event`.
            if overlay.kind == OverlayKind::Options {
                return true;
            }
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
            // Centrée sur les DEUX axes (2026-09-08, §9 du plan) — « au centre de l'écran de
            // l'utilisateur au niveau du jeu », contrairement à Combat/Suivi qui restent ancrés
            // sur un bord.
            OverlayKind::Options => PhysicalPosition::new(
                rect.left + (rect.width - overlay_width) / 2,
                rect.top + (rect.height - overlay_height) / 2,
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
            OverlayKind::Options => (
                options_modal::WINDOW_SIZE.0 as f64,
                options_modal::WINDOW_SIZE.1 as f64,
            ),
        };
        let title_suffix = match kind {
            OverlayKind::Combat => "Combat",
            OverlayKind::Watchlist => "Suivi",
            OverlayKind::Options => "Options",
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
        // `WS_EX_NOACTIVATE` (voir sa doc) empêcherait la modale Options de recevoir le focus
        // clavier — inacceptable pour éditer son champ de chemin (2026-09-08, §9 du plan) : SEULE
        // cette fenêtre garde `WS_EX_TOOLWINDOW` (hors barre des tâches/alt-tab, comme
        // `with_skip_taskbar` ci-dessus) sans `WS_EX_NOACTIVATE`, contrairement à Combat/Suivi qui
        // ne doivent JAMAIS voler le focus au jeu.
        if kind == OverlayKind::Options {
            Self::apply_extended_styles_focusable(hwnd);
        } else {
            Self::apply_extended_styles(hwnd);
        }
        // La modale Options force sa propre interactivité (voir `App::open_options_modal`) — voir
        // aussi le paramètre `interactive` passé explicitement `true` par cet appelant pour ce cas.
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
            // Renseigné juste après par l'appelant (`open_options_modal`) pour `kind == Options`
            // — `None` ici pour Combat/Suivi, jamais consulté (voir `RenderContent::options`).
            options_state: (kind == OverlayKind::Options).then(OptionsModalState::default),
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

    /// Variante SANS `WS_EX_NOACTIVATE` — voir l'appelant (`create_overlay_window`, cas
    /// `OverlayKind::Options`, 2026-09-08) : cette fenêtre doit pouvoir recevoir le focus clavier
    /// pour éditer son champ de chemin, contrairement à Combat/Suivi. `WS_EX_TOOLWINDOW` seul
    /// suffit à la garder hors barre des tâches/alt-tab.
    fn apply_extended_styles_focusable(hwnd: HWND) {
        unsafe {
            let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let new_style = current | (WS_EX_TOOLWINDOW.0 as isize);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
        }
    }

    fn hwnd_of(window: &Window) -> HWND {
        match window.window_handle().expect("handle de fenêtre").as_raw() {
            RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as *mut _),
            other => panic!("handle de fenêtre inattendu sur Windows : {other:?}"),
        }
    }

    /// Titre d'un HWND quelconque (pas nécessairement une fenêtre de jeu) — même appels que
    /// `game_window::imp::window_title`, dupliqué ici plutôt que rendu `pub` là-bas : ce module-ci
    /// n'a besoin que d'un diagnostic ponctuel (voir `last_foreground_heartbeat`), pas d'exposer
    /// une API de fenêtrage générique depuis `game_window`. `"<sans titre ou HWND nul>"` si
    /// indisponible — jamais fatal, juste une ligne de journal moins précise.
    fn window_title(hwnd: HWND) -> String {
        if hwnd.0.is_null() {
            return "<aucune fenêtre au premier plan>".to_string();
        }
        unsafe {
            let len = GetWindowTextLengthW(hwnd);
            if len <= 0 {
                return "<sans titre>".to_string();
            }
            let mut buf = vec![0u16; len as usize + 1];
            let written = GetWindowTextW(hwnd, &mut buf);
            String::from_utf16_lossy(&buf[..written as usize])
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

    /// `DETAILS_HOTKEY_LABEL` : même action que le clic sur le bouton "Détails" (lien externe)
    /// du carré de contrôle (`panels::watchlist::control_button_row`) — voir sa doc pour
    /// `base_url()`. `open::that` est best-effort (résultat ignoré, même choix que le clic direct) :
    /// un navigateur qui ne s'ouvre pas n'est pas une raison de faire quoi que ce soit d'autre
    /// planter.
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

        // Diagnostic 2026-09-06 (retour utilisateur, `session_id=9956` : une démotion jamais
        // suivie d'aucune repromotion pendant plus de 3 minutes, sans une seule ligne de journal
        // entre les deux pour dire pourquoi) — voir la doc d'`App::last_foreground_heartbeat`.
        // Se déclenche UNIQUEMENT tant qu'au moins un overlay reste `HWND_NOTOPMOST` : invisible
        // en usage normal (aucun surcoût, aucun spam), mais donne enfin une trace exploitable
        // pendant un épisode bloqué — au prochain test, on saura si `GetForegroundWindow()`
        // désignait réellement autre chose que le jeu tout du long (l'utilisateur avait
        // effectivement l'attention ailleurs) ou une valeur qui ne correspond à rien de connu
        // (détection en défaut).
        if self.windows.values().any(|o| !o.is_topmost) {
            let due = self
                .last_foreground_heartbeat
                .is_none_or(|t| now.duration_since(t) >= FOREGROUND_HEARTBEAT_INTERVAL);
            if due {
                self.last_foreground_heartbeat = Some(now);
                tracing::info!(
                    "[topmost] sondage (overlay(s) toujours rétrogradé(s)) : premier plan actuel = « {} »",
                    Self::window_title(foreground)
                );
            }
        }

        // Correctif 2026-09-06/07 (retour utilisateur, `session_id=880` : Combat et Suivi
        // basculent topmost/rétrogradé six fois en moins d'une minute, PAS TOUJOURS ENSEMBLE —
        // et symptôme répété sur plusieurs sessions : « il faut interagir sur un overlay pour
        // que l'autre revienne »). Cause : malgré `WS_EX_NOACTIVATE` (censé empêcher qu'une
        // fenêtre overlay devienne jamais la fenêtre au premier plan, voir sa doc), un clic
        // dessus en mode INTERACTIF peut brièvement faire de SA PROPRE `HWND` la valeur renvoyée
        // par `GetForegroundWindow()` — observé dans plusieurs journaux (`sondage : premier plan
        // actuel = « wakfu-companion-overlay — Oumbra — Combat »`, par exemple). L'ancien calcul
        // ne comparait `foreground` qu'au `game_hwnd` OU À SA PROPRE HWND — cliquer sur Combat
        // rendait donc *Combat* relevant (son propre hwnd correspond), mais pas *Suivi* du MÊME
        // personnage (hwnd différent, ni le jeu) : Suivi se retrouvait démoté par la fenêtre
        // sœur qu'on venait pourtant d'utiliser, sans aucune raison de l'utilisateur de penser
        // que les deux étaient liées. Précalculé PAR PERSONNAGE (`game_hwnd`) avant la boucle :
        // le focus sur N'IMPORTE LEQUEL des overlays d'un personnage (ou le jeu lui-même) rend
        // TOUS les overlays de CE personnage relevant — jamais ceux d'un AUTRE personnage en
        // multi-compte, qui gardent leur propre calcul indépendant.
        let mut relevant_game_hwnds: Vec<HWND> = Vec::new();
        for overlay in self.windows.values() {
            let this_relevant =
                overlay.game_hwnd == foreground || Self::hwnd_of(&overlay.window) == foreground;
            if this_relevant && !relevant_game_hwnds.contains(&overlay.game_hwnd) {
                relevant_game_hwnds.push(overlay.game_hwnd);
            }
        }

        for overlay in self.windows.values_mut() {
            // La modale Options (2026-09-08) reste `HWND_TOPMOST` tout du long, posé une seule
            // fois à sa création (voir `create_overlay_window`) — jamais concernée par le suivi de
            // focus PAR PERSONNAGE ci-dessus (voir la doc de `OverlayWindow::options_state`), sans
            // quoi elle serait démotée après le délai de grâce faute de `game_hwnd` correspondant
            // à une vraie fenêtre de jeu.
            if overlay.kind == OverlayKind::Options {
                continue;
            }
            let relevant = relevant_game_hwnds.contains(&overlay.game_hwnd);

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
                // Journalisé UNIQUEMENT sur une vraie transition (pas la réaffirmation
                // périodique, qui tournerait sinon toutes les `TOPMOST_REASSERT_INTERVAL` pour
                // rien) — diagnostic 2026-09-06 (retour utilisateur, instabilité perçue entre les
                // deux overlays) : permet de voir dans le journal, sans vidéo à décortiquer, si
                // les DEUX fenêtres d'un même personnage (Combat + Suivi) sont promues au même
                // tick ou avec un décalage — voir aussi `GpuState::occluded_since` (`frame.rs`)
                // pour corréler avec une éventuelle occlusion juste après ce changement de style.
                if transitioned {
                    // Position/taille réelles ajoutées au diagnostic (2026-09-06, retour
                    // utilisateur : Combat systématiquement plus touché que Suivi malgré des
                    // transitions identiques) — écarte ou confirme une position aberrante
                    // (partiellement hors écran, taille nulle) comme cause d'une fenêtre
                    // "topmost" d'après nous mais invisible à l'écran, sans avoir à deviner depuis
                    // une capture d'écran à chaque fois.
                    let pos = overlay
                        .window
                        .outer_position()
                        .map(|p| format!("{},{}", p.x, p.y))
                        .unwrap_or_else(|_| "?".to_string());
                    let size = overlay.window.outer_size();
                    tracing::info!(
                        "[topmost] {} ({:?}) -> HWND_TOPMOST @ ({pos}) {}x{}",
                        overlay.character_name,
                        overlay.kind,
                        size.width,
                        size.height
                    );
                    // Correctif 2026-09-06 (retour utilisateur, plusieurs sessions : Combat
                    // "topmost" d'après ce journal, position/taille saines, mais réellement
                    // invisible à l'écran pendant plusieurs secondes, jusqu'à un clic dessus) —
                    // hypothèse `GpuState::occluded_since` (occlusion DXGI) désormais ÉCARTÉE PAR
                    // LES FAITS : aucune session incriminée n'a jamais produit la moindre ligne
                    // `[occlusion]`, alors que `get_current_texture()` continuait donc de réussir
                    // (`Success`/`Suboptimal`) sans que rien ne s'affiche réellement.
                    //
                    // Cause probable, trouvée dans le vendor `wgpu-hal` lui-même
                    // (`vendor/wgpu-hal-30.0.1/src/dx12/mod.rs`, chemin `configure_surface` /
                    // `SurfaceTarget::VisualFromWndHandle`) : `IDCompositionVisual::SetContent` +
                    // `IDCompositionDevice::Commit()` ne sont appelés QU'UNE SEULE FOIS, à la
                    // création du swapchain — jamais rejoués ensuite. Si ce tout premier `Commit`
                    // intervient avant que DWM n'ait fini d'intégrer la fenêtre dans son arbre de
                    // composition (fenêtre tout juste créée, ou restée `HWND_NOTOPMOST` un long
                    // moment), chaque `Present()` suivant peut continuer de réussir côté DXGI sans
                    // jamais être réellement composé à l'écran — jusqu'à ce qu'un événement
                    // quelconque pousse DWM à réévaluer (d'où « ça s'affiche dès que j'interagis
                    // avec »). Un `request_redraw()` seul (Present() sur le MÊME swapchain déjà
                    // potentiellement mal accroché) ne suffit PAS : un `surface.configure()`
                    // répété ne le corrige pas non plus (chemin `ResizeBuffers` de wgpu-hal, qui
                    // saute `SetContent`/`Commit` une fois la surface déjà configurée une
                    // première fois — voir la doc de `frame::recreate_surface`). Seule une
                    // `Surface` RECRÉÉE de zéro rejoue ce chemin.
                    recreate_surface(&mut overlay.gpu, &overlay.window);
                    overlay.window.request_redraw();
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
            tracing::info!(
                "[topmost] {} ({:?}) -> HWND_NOTOPMOST (après {:?} sans pertinence)",
                overlay.character_name,
                overlay.kind,
                now.duration_since(demote_due_at)
            );
            overlay.is_topmost = false;
            overlay.pending_demote_since = None;
        }
    }

    /// Ouvre la modale Options (2026-09-08, §9 du plan) — bouton "Options" du carré de contrôle
    /// (`anchor_rect` = `game_rect` de la fenêtre Suivi cliquée) ou raccourci global
    /// `OPTIONS_HOTKEY_LABEL` (`anchor_rect` = celui de la première fenêtre de jeu connue, s'il y
    /// en a une). Sans effet si une modale est déjà ouverte (une seule à la fois, comme un vrai
    /// dialogue modal) — pas de file d'attente, l'utilisateur referme/valide l'existante avant
    /// d'en rouvrir une. Même méthode que `bin/overlay-ui-x11.rs` (dupliquée, voir la doc de
    /// `lib.rs` pour pourquoi le fenêtrage OS n'est PAS partagé entre les deux binaires).
    fn open_options_modal(&mut self, event_loop: &ActiveEventLoop, anchor_rect: Option<GameRect>) {
        if self
            .windows
            .values()
            .any(|w| w.kind == OverlayKind::Options)
        {
            return;
        }
        let rect = anchor_rect
            .or_else(|| self.windows.values().next().map(|w| w.game_rect))
            .unwrap_or(GameRect {
                left: 0,
                top: 0,
                width: 1280,
                height: 720,
                client_top: 0,
            });
        // `game_hwnd: HWND::default()` (nul) — voir la doc de `OverlayWindow::options_state` pour
        // pourquoi `sync_windows`/`sync_topmost` excluent explicitement `OverlayKind::Options` de
        // toute logique basée sur ce champ. Toujours interactive (`true` littéral, PAS
        // `self.interactive`) : une modale qui doit capter le clavier/la souris pour éditer le
        // chemin, pas un overlay passif d'information comme Combat/Suivi.
        let mut overlay = Self::create_overlay_window(
            event_loop,
            OverlayKind::Options,
            HWND::default(),
            "Options".to_string(),
            rect,
            true,
        );
        // **Le brouillon d'alertes est une COPIE du profil du compte**, prise à l'ouverture : les
        // gestes de l'onglet la modifient librement, et seul « Valider » la renvoie (§5.1 du plan).
        // Sans compte lié, il n'y a ni liste à charger ni endroit où l'écrire — l'onglet le dit.
        let alerts_snapshot = self.alert_profile.load();
        let (alerts_draft, alerts_availability) = match alerts_snapshot.as_ref() {
            Some((profile, _)) => (Some(profile.clone()), alerts_tab::AlertsAvailability::Ready),
            None if matches!(**self.auth_status.load(), AuthStatus::Connected) => {
                // Compte lié mais réglages pas encore revenus : c'est le seul cas où un rouage dit
                // la vérité.
                (None, alerts_tab::AlertsAvailability::Loading)
            }
            None => (None, alerts_tab::AlertsAvailability::NoAccount),
        };
        overlay.options_state = Some(OptionsModalState {
            path_input: self.log_path.display().to_string(),
            error: None,
            // Toujours « Paramètres » à l'ouverture : c'est le défaut d'`OptionsTab`, et le
            // réglage qu'on vient chercher en premier.
            tab: Default::default(),
            alerts: alerts_tab::AlertsTabState {
                // Le champ de durée s'ouvre sur la valeur en place, pas vide : c'est un réglage
                // existant qu'on vient modifier.
                duration_input: alerts_draft
                    .as_ref()
                    .map(|p| format_alert_duration(p.duration_seconds))
                    .unwrap_or_default(),
                ..Default::default()
            },
            alerts_draft,
            alerts_availability,
        });
        overlay.window.request_redraw();
        self.windows.insert(overlay.window.id(), overlay);
        tracing::info!("[options] modale ouverte.");
    }

    /// Lance l'explorateur de fichiers natif (`rfd`) sur un thread dédié — bloquant côté OS, ne
    /// doit JAMAIS geler la boucle winit (même raison que tous les threads réseau de ce dépôt,
    /// voir §7.3 du plan pour la justification appliquée à `overlay-sync`). Le résultat (chemin
    /// choisi, ou `None` si l'utilisateur a annulé le dialogue) revient par `self.pending_dialog`,
    /// sondé à chaque `about_to_wait`.
    fn start_file_dialog(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.pending_dialog = Some(rx);
        thread::Builder::new()
            .name("overlay-ui-file-dialog".into())
            .spawn(move || {
                // Filtre par EXTENSION uniquement (`rfd` ne sait pas filtrer par nom de fichier
                // exact) — le garde-fou du NOM exact (`wakfu.log`) est appliqué après coup par
                // `App::validate_and_apply_log_path`/`discovery::validate_log_path`, jamais sauté
                // même si l'utilisateur choisit un `.log` mal nommé dans le dialogue.
                let picked = rfd::FileDialog::new()
                    .set_title("Sélectionner le fichier wakfu.log")
                    .add_filter("wakfu.log", &["log"])
                    .pick_file();
                let _ = tx.send(picked);
            })
            .expect("échec de création du thread de dialogue de fichier");
    }

    /// Valide `raw` (contenu du champ texte au moment du clic sur "Valider", ou chemin choisi par
    /// le dialogue natif) via `discovery::validate_log_path` — voir §5.1 du plan : « il ne peut
    /// sélectionner qu'un fichier wakfu.log [...] des guards pour éviter de sélectionner n'importe
    /// quoi ». Sur succès : persiste (`config::save`), recharge l'Engine À CHAUD
    /// (`EngineCommand::ChangeLogPath`, jamais de redémarrage du binaire) et ferme la modale —
    /// « lorsqu'il valide [...] c'est ce nouveau fichier qui est lu de manière continue ». Sur
    /// échec : la modale RESTE ouverte, le message d'erreur est écrit dans son état pour le
    /// prochain redessin (voir `OptionsModalState::error`) — rien n'est pris en compte tant que la
    /// validation n'a pas réussi.
    /// Applique le brouillon d'alertes de la fenêtre Options : tout de suite à l'`Engine`, et au
    /// compte sur un thread.
    ///
    /// **Ne fait rien si rien n'a changé** — rouvrir la fenêtre et cliquer « Valider » pour régler
    /// le seul chemin de log ne doit pas écrire la clé `profile` du compte, dont l'arbitrage est
    /// « dernier écrivain gagne » : une écriture inutile écraserait une modification faite depuis
    /// le site entre-temps.
    fn commit_alerts(&mut self, options_window_id: WindowId) {
        let Some(draft) = self
            .windows
            .get(&options_window_id)
            .and_then(|overlay| overlay.options_state.as_ref())
            .and_then(|state| state.alerts_draft.clone())
        else {
            return;
        };
        let connu = self.alert_profile.load();
        let (reference, raw) = match connu.as_ref() {
            Some((profile, raw)) => (Some(profile.clone()), raw.clone()),
            None => (None, None),
        };
        if reference.as_ref() == Some(&draft) {
            return;
        }

        // Appliqué localement d'abord : la prochaine alerte doit obéir sans attendre le réseau.
        let _ = self
            .settings_tx
            .send(EngineCommand::SetAlertProfile(draft.clone()));

        // Et écrit au compte sur un thread — jamais sur la boucle winit (§7.3 du plan, même règle
        // que tous les appels réseau de ce dépôt).
        let profile_value = draft.patch_value(raw.as_ref());
        thread::spawn(move || match overlay_sync::token_store::load_token() {
            Some(token) => match overlay_sync::client::patch_profile(&token, &profile_value) {
                Ok(_) => tracing::info!("[options] alertes enregistrées sur le compte."),
                // Best-effort, comme la réplication des compteurs de Suivi : le réglage est déjà
                // actif localement, et la prochaine validation réessaiera. Un échec réseau ne doit
                // pas empêcher de fermer la fenêtre.
                Err(err) => tracing::warn!(%err, "[options] échec de l'enregistrement des alertes"),
            },
            None => tracing::info!(
                "[options] alertes appliquées localement — aucun compte lié, rien n'est enregistré."
            ),
        });
    }

    fn validate_and_apply_log_path(&mut self, options_window_id: WindowId, raw: String) {
        let candidate = PathBuf::from(raw.trim());
        match discovery::validate_log_path(&candidate) {
            Ok(()) => {
                tracing::info!(
                    "[options] nouveau fichier de log validé : {}",
                    candidate.display()
                );
                self.log_path = candidate.clone();
                config::save(&config::OverlayConfig {
                    log_path: Some(candidate.clone()),
                });
                let _ = self
                    .settings_tx
                    .send(EngineCommand::ChangeLogPath(candidate));
                // **« Valider » commit TOUS les onglets, pas seulement celui qu'on regarde.** Le
                // pied de page est partagé : un bouton dont l'effet dépendrait de l'onglet affiché
                // serait imprévisible. Fait APRÈS la validation du chemin, et seulement si elle
                // passe — quand elle échoue, la fenêtre reste ouverte et rien n'est pris en compte,
                // alertes comprises.
                self.commit_alerts(options_window_id);
                self.windows.remove(&options_window_id);
            }
            Err(err) => {
                tracing::info!("[options] chemin refusé : {}", err.message());
                if let Some(overlay) = self.windows.get_mut(&options_window_id) {
                    if let Some(state) = &mut overlay.options_state {
                        state.error = Some(err.message().to_string());
                    }
                    overlay.window.request_redraw();
                }
            }
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
        // Renseigné éventuellement PAR l'emprunt de `overlay` ci-dessous (voir la fin de cette
        // fonction) — regroupé plutôt que de multiples `bool`/`Option` séparés, pour un seul
        // `match` final au lieu de plusieurs `if` empilés. Ne fait AUCUN appel `&mut self` avant
        // la fin de cette fonction : `overlay` (obtenu juste après) emprunte `self.windows` pour
        // toute la durée de son dernier usage (NLL), un appel `&mut self` plus tôt romprait la
        // compilation.
        enum PostRedraw {
            None,
            OpenOptions(GameRect),
            CloseOptions,
            BrowseOptions,
            ValidateOptions(String),
        }
        let mut post_redraw = PostRedraw::None;

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
            // Filet « Échap quitte l'overlay », **sauf pour la modale Options**.
            //
            // Il ne se déclenche en pratique jamais pour les autres fenêtres (voir la doc de
            // `QUIT_HOTKEY_LABEL`) : elles portent `WS_EX_NOACTIVATE` et ne reçoivent donc jamais le
            // focus clavier, quel que soit le mode. Laissé en place au cas où l'une d'elles
            // redeviendrait focalisable, mais `QUIT_HOTKEY_LABEL` reste le SEUL moyen fiable de
            // quitter sans passer par le terminal.
            //
            // La modale Options, elle, EST focalisable et délibérément (§9.1 du plan :
            // `WS_EX_NOACTIVATE` omis pour elle, il faut pouvoir taper dans le champ de chemin).
            // Sans cette exclusion, taper Échap dedans tuait l'overlay entier au lieu d'annuler la
            // saisie. Ses deux touches (`Échap` annule, `Entrée` valide) sont traitées par le
            // panneau lui-même, qui les remonte en `OptionsModalAction` — voir
            // `panels::options_modal::show`.
            WindowEvent::KeyboardInput { event, .. } => {
                if overlay.kind != OverlayKind::Options
                    && event.state == ElementState::Pressed
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
                // La modale Options force sa propre interactivité (voir `App::
                // open_options_modal`) — jamais assujettie à `self.interactive` (mode
                // clic-traversant global de Combat/Suivi), sans quoi elle deviendrait elle-même
                // traversable si l'utilisateur avait basculé ce mode juste avant.
                let interactive = overlay.kind == OverlayKind::Options || self.interactive;
                let this_game_rect = overlay.game_rect;
                let (repaint_delay, outcome) = render(
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
                        interactive,
                        now,
                        options: overlay.options_state.as_mut(),
                    },
                );
                // Fermeture au clic (carte ou croix, voir `panels::watchlist::toast_card`) — seul
                // point du code à détenir un accès en écriture à cet `ArcSwap` (`render` ne reçoit
                // le toast qu'en lecture, voir `RenderContent::watchlist_toast`). Le clic ayant
                // déjà fait passer `response.repaint` à `true` plus haut, le prochain redessin
                // relira `None` et n'affichera plus rien.
                if outcome.close_toast {
                    self.watchlist_toast.store(Arc::new(None));
                }
                // Voir `render_content::RenderOutcome` (2026-09-08, §9 du plan) : bouton "Options"
                // cliqué dans le carré de contrôle de CETTE fenêtre Suivi, ou action de la modale
                // Options elle-même — jamais les deux à la fois (branches différentes du `match
                // kind` de `paint_content`).
                if outcome.open_options {
                    post_redraw = PostRedraw::OpenOptions(this_game_rect);
                }
                // L'ouverture de page est faite ICI, par l'hôte, jamais par le panneau qui l'a
                // demandée : voir `RenderOutcome::open_url`. `open::that` est best-effort, comme
                // partout ailleurs — un navigateur qui ne s'ouvre pas ne fait rien planter.
                if let Some(url) = &outcome.open_url {
                    let _ = open::that(url);
                }
                match outcome.options_action {
                    OptionsModalAction::None => {}
                    OptionsModalAction::Cancel => post_redraw = PostRedraw::CloseOptions,
                    OptionsModalAction::Browse => post_redraw = PostRedraw::BrowseOptions,
                    OptionsModalAction::TestAlertSound => alert_sound::play_loot_alert(),
                    OptionsModalAction::Validate(raw) => {
                        post_redraw = PostRedraw::ValidateOptions(raw)
                    }
                }
                // Voir `OverlayWindow::next_redraw_at` : egui a pu demander un redessin après un
                // délai (tooltip...) que rien d'autre ne redéclenchera dans cette architecture.
                // `about_to_wait` est responsable de le consommer le moment venu.
                overlay.next_redraw_at = (repaint_delay < std::time::Duration::from_secs(3600))
                    .then(|| std::time::Instant::now() + repaint_delay);
            }
            _ => {}
        }

        match post_redraw {
            PostRedraw::None => {}
            PostRedraw::OpenOptions(rect) => self.open_options_modal(event_loop, Some(rect)),
            PostRedraw::CloseOptions => {
                self.windows.remove(&id);
                tracing::info!("[options] modale fermée (Annuler).");
            }
            PostRedraw::BrowseOptions => self.start_file_dialog(),
            PostRedraw::ValidateOptions(raw) => self.validate_and_apply_log_path(id, raw),
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
                tracing::info!(">>> Options ({OPTIONS_HOTKEY_LABEL})");
                self.open_options_modal(event_loop, None);
            } else if event.id == self.watchlist_add_hotkey_id {
                // Voir la doc de `WATCHLIST_ADD_HOTKEY_LABEL` : bouton encore inerte.
                tracing::debug!(">>> Ajouter ({WATCHLIST_ADD_HOTKEY_LABEL}) : encore inerte.");
            } else if event.id == self.watchlist_remove_hotkey_id {
                tracing::debug!(">>> Supprimer ({WATCHLIST_REMOVE_HOTKEY_LABEL}) : encore inerte.");
            } else if event.id == self.side_hotkey_id {
                self.toggle_combat_side();
            }
        }
        // Résultat du dialogue de fichier natif (`App::start_file_dialog`), le cas échéant — sondé
        // sans bloquer, comme les hotkeys ci-dessus. `try_recv` retourne `Empty` tant que
        // l'utilisateur n'a pas fini d'interagir avec le dialogue OS (peut prendre plusieurs
        // secondes) : `self.pending_dialog` n'est vidé QUE sur une réponse effective (`Ok`) ou un
        // thread mort (`Disconnected`, dialogue en échec) — jamais sur `Empty`, qui doit re-sonder
        // au prochain tour.
        if let Some(rx) = &self.pending_dialog {
            match rx.try_recv() {
                Ok(picked) => {
                    self.pending_dialog = None;
                    if let Some(path) = picked {
                        // Même garde que "Valider" (voir `validate_and_apply_log_path`) : `rfd` ne
                        // filtre QUE par extension, un `.log` mal nommé doit être refusé exactement
                        // pareil qu'une saisie manuelle invalide, jamais silencieusement accepté
                        // parce qu'il vient du dialogue natif — le champ affiche quand même le
                        // chemin choisi (l'utilisateur voit ce qu'il a sélectionné) accompagné du
                        // message d'erreur, plutôt que de l'ignorer en silence.
                        let validation = discovery::validate_log_path(&path);
                        if let Some(overlay) = self
                            .windows
                            .values_mut()
                            .find(|w| w.kind == OverlayKind::Options)
                        {
                            if let Some(state) = &mut overlay.options_state {
                                state.path_input = path.display().to_string();
                                state.error = validation.err().map(|e| e.message().to_string());
                            }
                            overlay.window.request_redraw();
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => self.pending_dialog = None,
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
        instance,
        surface,
        device,
        queue,
        config,
        egui_ctx,
        egui_winit,
        egui_renderer,
        occluded_since: None,
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
///
/// **Compteurs de Suivi (`SyncCommand::SyncWatchlist`, §14 point 3 du plan, chantier fermé le
/// 2026-09-07)** — même thread, mécanisme VOLONTAIREMENT séparé de la file `SyncQueue` ci-dessus :
/// pas de file SQLite ni de `client_key` idempotent (la valeur ENTIÈRE remplace la clé côté
/// serveur, voir `functions/api/v1/settings.ts::onRequestPatch`, « dernier écrivain gagne »), donc
/// rien à dédupliquer — seul le DERNIER instantané reçu compte, porté en mémoire
/// (`pending_watchlist`) plutôt qu'en base. Débounce de `WATCHLIST_DEBOUNCE` avant le premier
/// essai (miroir de `WRITE_DEBOUNCE_MS` côté web, `RemoteUserDataRepository`) : un compteur qui
/// s'incrémente à chaque kill d'un combat ne doit pas déclencher une requête par kill. Un nouvel
/// instantané reçu PENDANT l'attente (débounce ou backoff) remplace le précédent et relance un
/// débounce complet — rien n'est perdu (`WatchlistState` garde de toute façon le fichier local
/// comme vérité, voir sa doc), seul le nombre de requêtes est réduit.
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
            // Dernier instantané de Suivi reçu et pas encore répliqué avec succès (voir la doc de
            // `spawn_sync_thread` ci-dessus) — `None` tant qu'aucun `SyncCommand::SyncWatchlist`
            // n'est arrivé, ou après un envoi réussi.
            let mut pending_watchlist: Option<Vec<WatchlistEntry>> = None;
            // Instant à partir duquel retenter l'envoi watchlist (fin du débounce, ou du backoff
            // après un échec) — distinct du calcul `wait` de l'historique ci-dessous : les deux
            // mécanismes n'ont ni la même cause de délai ni le même état.
            let mut watchlist_ready_at: Option<std::time::Instant> = None;
            let mut watchlist_consecutive_failures: u32 = 0;
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
                    Ok(SyncCommand::SyncWatchlist(entries)) => {
                        pending_watchlist = Some(entries);
                        watchlist_consecutive_failures = 0;
                        watchlist_ready_at = Some(std::time::Instant::now() + WATCHLIST_DEBOUNCE);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {} // réessai programmé (backoff) — retombe sur le flush ci-dessous
                    Err(mpsc::RecvTimeoutError::Disconnected) => break, // App fermée
                }

                let history_wait = match &account {
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

                let watchlist_wait = flush_watchlist_once(
                    &account,
                    &mut pending_watchlist,
                    &mut watchlist_ready_at,
                    &mut watchlist_consecutive_failures,
                );

                wait = history_wait.min(watchlist_wait);
            }
        })
        .expect("échec de création du thread Sync");
}

/// Débounce avant le premier essai d'un envoi watchlist — miroir de `WRITE_DEBOUNCE_MS`
/// (`remote-user-data.repository.ts`) côté web.
const WATCHLIST_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(1_500);

/// Tente, si le moment est venu, de répliquer le dernier instantané de Suivi reçu — voir la doc de
/// `spawn_sync_thread` pour le mécanisme complet (débounce + backoff, sans file persistante).
/// Renvoie le délai avant le PROCHAIN passage utile pour CE mécanisme (à combiner par l'appelant
/// avec celui de l'historique, voir `wait`) : 1 h tant que rien n'est en attente ou qu'aucun
/// compte n'est connu (réveillé immédiatement par la prochaine commande), le temps restant avant
/// `ready_at` pendant un débounce/backoff en cours, ou le nouveau backoff après un échec.
fn flush_watchlist_once(
    account: &Option<(String, String)>,
    pending: &mut Option<Vec<WatchlistEntry>>,
    ready_at: &mut Option<std::time::Instant>,
    consecutive_failures: &mut u32,
) -> std::time::Duration {
    let Some(entries) = pending.as_ref() else {
        return std::time::Duration::from_secs(3600);
    };
    let Some((_, token)) = account else {
        return std::time::Duration::from_secs(3600); // pas de compte : rien à tenter avant `Activate`
    };
    let now = std::time::Instant::now();
    if let Some(at) = ready_at {
        if now < *at {
            return *at - now;
        }
    }

    match overlay_sync::patch_watchlist(token, entries) {
        Ok(_) => {
            *pending = None;
            *ready_at = None;
            *consecutive_failures = 0;
            std::time::Duration::from_secs(3600)
        }
        Err(err) => {
            *consecutive_failures += 1;
            tracing::warn!(
                %err,
                consecutive_failures = *consecutive_failures,
                "échec de synchro des compteurs de Suivi — nouvel essai après un backoff"
            );
            let delay = backoff_delay(*consecutive_failures);
            *ready_at = Some(now + delay);
            delay
        }
    }
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

/// Ordre de priorité complet (voir `config::resolve_log_path`) : argument CLI > chemin sauvegardé
/// par une validation précédente de la modale Options (2026-09-08, §9 du plan) > découverte
/// automatique. Seule la découverte automatique peut échouer complètement (aucun argument, rien en
/// config, aucun chemin connu du système) — dans ce cas SEULEMENT, ce binaire refuse encore de
/// démarrer sans chemin explicite (l'Engine a besoin d'un chemin dès `spawn_engine_thread`, voir
/// `main`).
fn resolve_path(config: &config::OverlayConfig) -> PathBuf {
    let cli_arg = env::args().nth(1).map(PathBuf::from);
    match config::resolve_log_path(cli_arg, config) {
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

    let saved_config = config::load();
    let log_path = resolve_path(&saved_config);
    let snapshot = Arc::new(ArcSwap::from_pointee(SessionSnapshot::default()));
    let watchlist = Arc::new(ArcSwap::from_pointee(Vec::<WatchlistEntry>::new()));
    let watchlist_toast = Arc::new(ArcSwap::from_pointee(None::<WatchlistToast>));
    let alert_profile = Arc::new(ArcSwap::from_pointee(None));
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
            alert_profile: Arc::clone(&alert_profile),
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
        alert_profile,
        catalog,
        catalog_stale,
        remote_icons,
        auth_status,
        auth_command_tx,
        settings_tx,
    });
    event_loop.run_app(&mut app).expect("boucle d'événements");
}
