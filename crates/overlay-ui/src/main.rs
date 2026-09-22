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
//! **Fenêtre de connexion et compte obligatoire (2026-09-14, §9.1 undecies du plan)** : l'overlay
//! n'a plus de mode invité. Tant que le thread Auth n'a pas publié `AuthStatus::Connected`, la
//! seule fenêtre à l'écran est la fenêtre de connexion (`OverlayKind::Login`, `panels::login`) —
//! une fenêtre logicielle classique, centrée sur l'écran principal, présente dans la barre des
//! tâches, jamais ancrée sur le jeu. Les overlays Combat/Suivi ne naissent qu'une fois le compte
//! lié (`App::sync_windows` s'y refuse sinon) et sont TOUS fermés à la déconnexion, qui ramène
//! à cette fenêtre (`App::sync_session_windows`). Le logo du site est l'icône de fenêtre et
//! l'icône de zone de notification, dont le menu — Options / Mise à jour / Déconnecter /
//! Quitter — est le seul accès à l'overlay quand aucune fenêtre de jeu n'est ouverte
//! (`App::install_tray`). « Mise à jour » rouvre cette même fenêtre sur son écran de mise à jour,
//! seul cas où elle coexiste avec les overlays de jeu (`App::open_manual_update_window`).
//!
//! Pas de thème configurable ni de disposition repositionnable/persistée par écran, ni d'indicateur
//! d'état de synchro — décisions du mainteneur (§9 du plan, 2026-09-02 et 2026-09-18) : un overlay
//! n'est pas un site, palette fixe et ancrage automatique (`App::anchor_position`) seuls assumés ;
//! la synchro (retries + backoff, L5) finit toujours par passer, rien d'alarmant à afficher. Le récap de session reste
//! également **global** (identique sur toutes les fenêtres, pas ventilé par personnage —
//! limitation connue, voir le plan) : ce sont les deux panneaux atteignables avec `overlay-engine`
//! tel qu'il existe aujourd'hui.
//!
//! **Alertes de drop version « ramassage » (2026-09-02, §9 du plan)** : `overlay_engine::profile`
//! lit désormais `data.profile.soundItems` (`GET /api/v1/settings`) — indépendant de la watchlist,
//! n'importe quel objet ramassé avec son son activé au compte déclenche toast + son
//! (`alert_sound::play_loot_alert`), pas seulement les entrées suivies. Seul le cas
//! `reason: 'countdown'` était câblé jusqu'ici (voir `spawn_engine_thread`).

// **Exécutable fenêtré sans fenêtre** (2026-09-17, demande utilisateur : l'overlay doit tourner de
// façon invisible, jamais dans une fenêtre de terminal). Compilé en sous-système *console*
// jusqu'ici, un double-clic sur l'exe — ou son lancement automatique à l'ouverture de session
// (`autostart`) — ouvrait une console noire qui restait à l'écran tant que l'overlay tournait, et
// la fermer tuait l'overlay. Avec ce sous-système, Windows n'en crée aucune : l'overlay ne se
// manifeste que par ses fenêtres transparentes et son icône de zone de notification (Quitter y est,
// `App::install_tray`). Le journal console n'est pas perdu pour autant en développement : lancé
// depuis un terminal (`preview.ps1`), le process se rattache à la console de son parent — voir
// `logging::attach_parent_console`. Ce n'est PAS un service Windows, et ne peut pas l'être : un
// service vit en session 0, sans accès au bureau de l'utilisateur, donc sans possibilité d'afficher
// quoi que ce soit par-dessus le jeu (§11 du plan). Même modèle qu'`overlay-focus`.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use arc_swap::ArcSwap;
use egui_wgpu::wgpu;
use global_hotkey::GlobalHotKeyEvent;
use overlay_engine::{CatalogIndex, DungeonIndex, Roster, SessionSnapshot, WatchlistEntry};
use overlay_ingest::discovery;
use overlay_sync::update::{apply as update_apply, UpdateStatus};
use overlay_ui::alert_sound;
use overlay_ui::avatars::AvatarAtlas;
use overlay_ui::background::{
    spawn_auth_thread, spawn_catalog_thread, spawn_dungeon_thread, spawn_game_servers_thread,
    spawn_sync_thread, spawn_update_thread, UpdateCommand,
};
use overlay_ui::build_info;
use overlay_ui::chat_command::{self, ChatCommand};
use overlay_ui::combat_placement;
use overlay_ui::config;
use overlay_ui::engine_thread::{
    spawn_engine_thread, EngineCommand, EngineHandles, SharedAlertProfile, SharedChatFilters,
    SharedRosterDraft, WatchlistCompleted,
};
use overlay_ui::frame::{recreate_surface, render, GpuState};
use overlay_ui::game_servers::GameServers;
use overlay_ui::game_window::{self, GameRect, GameWindowTracker};
use overlay_ui::logging;
use overlay_ui::panels;
use overlay_ui::panels::alerts_tab;
use overlay_ui::panels::chat_tab;
use overlay_ui::panels::combat::{CombatMetric, CombatSide};
use overlay_ui::panels::combat_frame::CombatFrame;
use overlay_ui::panels::feature_switch::FeatureToggles;
use overlay_ui::panels::login::{self, LoginState};
use overlay_ui::panels::notifications::AlertMutes;
use overlay_ui::panels::options_modal::{self, OptionsModalAction, OptionsModalState};
use overlay_ui::panels::personnages_tab::{PersonnagesAvailability, PersonnagesTabState};
use overlay_ui::panels::suivi_tab;
use overlay_ui::panels::watchlist::WatchlistToast;
use overlay_ui::portraits::PortraitAtlas;
use overlay_ui::recap_placement;
use overlay_ui::recap_session::{self, RecapSession};
use overlay_ui::remote_icons::{RemoteIconStore, RemoteIconTextures};
use overlay_ui::render_content;
use overlay_ui::render_content::{
    AuthCommand, AuthStatus, OverlayKind, RenderContent, ResetTarget, UserEvent,
};
use overlay_ui::shortcuts::{ShortcutAction, ShortcutBindings, ShortcutRegistry};
use overlay_ui::startup::StartupProgress;
use overlay_ui::turn_watch;
use overlay_ui::ui_icons::{self, UiIcons};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
    SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Icon, Window, WindowAttributes, WindowId, WindowLevel};

#[cfg(target_os = "windows")]
use winit::platform::windows::WindowAttributesExtWindows;

// **Les combinaisons ne sont plus des constantes de ce fichier depuis le 2026-09-13** : elles sont
// personnalisables par l'utilisateur (onglet « Raccourcis » de la fenêtre Options) et vivent donc
// dans `overlay_ui::shortcuts` — `ShortcutAction` (la liste des actions et leur combinaison PAR
// DÉFAUT, inchangée par rapport aux anciennes constantes `HOTKEY_LABEL`/`DETAILS_HOTKEY_LABEL`/…,
// dont la doc a suivi là-bas), `ShortcutBindings` (les combinaisons effectives, lues de
// `config.toml`) et `ShortcutRegistry` (l'enregistrement auprès de l'OS, partagé avec
// `bin/wakfu-companion-overlay-x11.rs`). `App::hotkeys` porte le tout.

/// Voir `App::sync_topmost`.
/// Écart entre deux images pendant qu'une tuile du Suivi célèbre son aboutissement — 60 Hz.
///
/// La boucle de l'overlay est réactive et se réveille sinon toutes les 50 ms (voir
/// `about_to_wait`) : une couronne qui tourne à 20 images par seconde saccade visiblement. Ce
/// rythme n'est demandé que le temps de la célébration, jamais en continu.
const COMPLETION_FRAME: std::time::Duration = std::time::Duration::from_millis(16);

const TOPMOST_REASSERT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);
/// Délai de grâce avant repli en `HWND_NOTOPMOST` — voir `OverlayWindow::pending_demote_since` et
/// `App::sync_topmost`. Assez court pour qu'un changement de fenêtre volontaire et soutenu
/// reste respecté rapidement (ne pas recouvrir durablement une autre appli, retour utilisateur
/// 2026-09-01), assez long pour absorber un aléa de timing d'un seul tick (~50 ms) entre les deux
/// overlays d'un même personnage.
const TOPMOST_DEMOTE_GRACE: std::time::Duration = std::time::Duration::from_millis(1500);
/// Cadence de la surveillance de tour (§9.1 decies) — voir `App::sync_turn_watch`. Le chrono du
/// widget change à la seconde ; 500 ms suffisent pour voir chaque tour commencer.
const TURN_WATCH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

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
/// `watchlist_target_width`), pas une constante fixe. Plus
/// `render_content::WATCHLIST_TOOLTIP_RESERVE` (2026-09-13, voir sa doc) : la place des
/// infobulles, qui s'ouvrent toutes EN DESSOUS de la bande depuis le soir du même jour. Cette
/// réserve-là ne décale rien — elle laisse de la hauteur de fenêtre SOUS le contenu, là où la
/// version du matin la prenait AU-DESSUS et éloignait la bande du bord haut du jeu d'autant.
///
/// **Ramenée de 132 à 92 px le 2026-09-13** (retour utilisateur : « on peut réduire un peu
/// l'overlay en hauteur »). Les 132 px dataient du 2026-09-02, où ils réservaient d'un bloc
/// l'espace du toast d'alerte SOUS la bande — devenu inutile le jour même, `watchlist_target_
/// height` ajoutant `TOAST_AREA_HEIGHT` seulement tant qu'un toast est actif —, puis 16 px de plus
/// pour dégager la barre de défilement flottante des icônes. Les deux raisons ont disparu : la
/// barre du jeu (`panels::watchlist::strip_scroll_area`) prend sa place SOUS les tuiles au lieu de
/// flotter dessus. 92 px, c'est ce que la bande occupe réellement — 6 px de marge interne haute,
/// 72 px de zone défilante (58 de tuile + 14 de réserve de barre), 6 px de gouttière avant le
/// toast, 6 px de marge basse, plus 2 px d'arrondi — et rien de plus : la fenêtre reste
/// cliquable/bloquante sur toute sa surface, y compris là où elle ne peint rien.
const WATCHLIST_HEIGHT: f64 = 92.0 + render_content::WATCHLIST_TOOLTIP_RESERVE as f64;

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
fn watchlist_target_width(
    entry_count: usize,
    tracking_enabled: bool,
    toast_active: bool,
    game_width_px: i32,
) -> f64 {
    let ceiling = (game_width_px as f64 * WATCHLIST_WIDTH_FRACTION).min(WATCHLIST_MAX_CEILING);
    let tiles = panels::watchlist::content_width(entry_count, tracking_enabled) as f64;
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
fn watchlist_target_height(toast_active: bool, select_open: bool) -> f64 {
    WATCHLIST_HEIGHT
        + if toast_active {
            panels::watchlist::TOAST_AREA_HEIGHT as f64
        } else {
            0.0
        }
        // La bande du bouton de suppression groupée, le temps de la sélection multiple — la
        // fenêtre se rétracte en quittant le mode (2026-09-13).
        + if select_open {
            panels::watchlist::SELECTION_BAR_HEIGHT as f64
        } else {
            0.0
        }
}
// **La marge au bord vertical du client a rejoint `overlay_ui::combat_placement`** (2026-09-17)
// avec tout l'ancrage du panneau Combat : elle est NULLE, des deux côtés, et c'est une décision
// d'écran plus qu'un calcul. Son relevé, qu'il ne faut pas perdre :
//
// **Refonte 2026-09-04** (retour utilisateur, capture d'écran à l'appui) : la valeur initiale
// (12 px, « collé à quelques pixels près ») laissait un vide visible entre le bord de la fenêtre
// de jeu et le cadre — l'utilisateur veut que l'overlay se fonde dans le jeu (« comme si l'overlay
// faisait partie du jeu »), donc un ancrage réellement à zéro. Voir aussi
// `render_content::paint_content` : la marge interne du panneau (`Frame::inner_margin`) doit être
// nulle elle aussi, sinon un vide subsiste malgré cet ancrage.
//
// Le binaire X11, lui, était resté à 12 px — cette refonte ne l'avait pas suivi, et personne ne
// l'avait vu puisque rien ne comparait les deux hôtes. Le calcul partagé les met d'accord.
/// Marge, en pixels physiques, entre le bord HAUT de la zone cliente du jeu et l'overlay Suivi
/// (demande utilisateur explicite 2026-09-01 : « collé en haut de la fenêtre de jeu au centre »,
/// « le même espacement » que les boutons d'interface du jeu — menu/Boutique en haut-gauche, icônes
/// en haut-droite).
///
/// **Historique de mise au point (2026-09-01, deux allers-retours avec capture d'écran)** : la
/// valeur initiale (12 px, copiée de la marge latérale du Combat) était bien trop petite — l'overlay
/// apparaissait quasiment collé au très haut de la fenêtre de jeu. Diagnostic confirmé par le
/// `println!` de `create_overlay_window` (`rect.top=0 client_top=0, écart 0`) : **le client Wakfu
/// dessine lui-même sa fausse barre de titre DANS sa zone cliente** (voir `GameRect::client_top`,
/// fenêtre non décorée côté Win32) — il n'y avait donc aucune vraie barre de titre Windows à
/// exclure, `client_top` valait déjà `top`. Toute la hauteur à compenser est celle de cette fausse
/// barre de titre dessinée par le jeu, pas une histoire de coordonnées Win32 : 28 px s'est avéré
/// visuellement correct (confirmé par une deuxième capture d'écran, alignement quasi identique aux
/// boutons Menu/Boutique du jeu).
const GAME_TOP_MARGIN_PX: i32 = 28;
// L'ancrage du bloc Récap — « en haut à gauche, en dessous des boutons du jeu » (2026-09-16) —
// et le décalage que l'utilisateur lui donne à la souris depuis le 2026-09-17 vivent dans
// `overlay_ui::recap_placement` (`DEFAULT_OFFSET`, bornage et aimantation) : le binaire X11 fait
// le même calcul au pixel près, et là-bas il se teste sans serveur graphique. Les constantes
// `GAME_RECAP_TOP_MARGIN_PX`/`GAME_RECAP_EDGE_MARGIN_PX` qui vivaient ici sont devenues ce
// `DEFAULT_OFFSET`, relevé sur capture d'écran annotée — toute leur histoire y est.

// `OverlayKind`/`UserEvent`/`AuthStatus`/`AuthCommand`/`CLICK_THROUGH_OPACITY` ont migré vers
// `overlay_ui::render_content` (2026-09-03, §17.1 du plan) — voir leur doc là-bas, importés en
// tête de ce fichier. Rien ne change à leur usage ici, seul leur PROPRIÉTAIRE change : la
// construction d'UI (`render_content::build_ui`) doit pouvoir être appelée par un futur harnais
// de rendu offscreen (`overlay-testkit`) sans dépendre du binaire `wakfu-companion-overlay`, qui
// reste Windows-only (import inconditionnel de `windows::`, voir plus haut).

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
    /// Les bustes de classe de l'onglet « Personnages » — **`Some` seulement pour la fenêtre
    /// Options**, la seule qui les affiche (voir `overlay_ui::avatars`, doc de module) : 36 PNG
    /// décodés et 1,8 Mo de textures par fenêtre de jeu seraient payés pour rien.
    avatars: Option<AvatarAtlas>,
    /// Cache PAR FENÊTRE des icônes réelles d'objets/monstres déjà uploadées (voir
    /// `remote_icons::RemoteIconTextures`) — sans objet pour une fenêtre `Combat`.
    remote_icon_textures: RemoteIconTextures,
    /// Camp affiché dans la liste verticale du panneau Combat (voir `panels::combat::CombatSide`)
    /// — état PAR FENÊTRE (donc par personnage), pas global : `Allies` par défaut à chaque
    /// création de fenêtre (demande utilisateur explicite). Sans objet pour une fenêtre `Suivi`.
    combat_side: CombatSide,
    /// Grandeur mesurée par le panneau Combat — dégâts, armure donnée ou soins (voir
    /// `panels::combat::CombatMetric`). Un état par fenêtre, comme `combat_side` : deux
    /// personnages peuvent regarder deux grandeurs différentes du même combat.
    combat_metric: CombatMetric,
    /// État de la modale Options (2026-09-08, §9 du plan) — `Some` UNIQUEMENT pour `kind ==
    /// OverlayKind::Options`, voir `App::open_options_modal`. Même remarque que
    /// `bin/wakfu-companion-overlay-x11.rs` (code partagé côté `panels::options_modal`, duplication
    /// assumée côté fenêtrage OS comme le reste de ce fichier).
    options_state: Option<OptionsModalState>,
    /// État de la fenêtre de connexion (2026-09-14) — `Some` UNIQUEMENT pour `kind ==
    /// OverlayKind::Login`, voir `App::create_login_window`. Même règle qu'`options_state`.
    login_state: Option<LoginState>,
    /// Dernière hauteur demandée pour la fenêtre de connexion (voir
    /// `panels::login::LoginOutcome::content_height`) — même principe que `last_watchlist_height` :
    /// la fenêtre OS n'est retaillée que quand l'état affiché change de hauteur.
    last_login_height: Option<f32>,
    /// Fenêtre de jeu à laquelle cet overlay est ancré — **nulle pour la fenêtre de connexion**,
    /// qui n'appartient à aucun personnage (voir `sync_topmost`, qui l'ignore), **et pour la
    /// fenêtre Options ouverte alors qu'aucun client Wakfu n'est à l'écran** (2026-09-17, voir
    /// `is_detached` et `App::open_options_modal`).
    game_hwnd: HWND,
    /// Dernier rectangle connu de la fenêtre de jeu (mis à jour par `sync_windows`/`reposition`,
    /// voir `App::sync_windows`) — réutilisé par `RedrawRequested` pour le plafond de largeur
    /// dynamique du Suivi (`watchlist_target_width`) sans re-scanner les fenêtres à chaque frame.
    game_rect: GameRect,
    /// Le titulaire de la fenêtre de jeu à la création de cet overlay — sa clé, stable.
    character_name: String,
    /// Le personnage **aux commandes** de la fenêtre de jeu maintenant : son titre au dernier
    /// `scan()` (voir `sync_windows`). Le client Wakfu y met le héros dont c'est le tour — c'est
    /// ce que lit la surveillance de tour (`sync_turn_watch`, doc de `turn_watch::watcher`).
    /// Égal à `character_name` tant que la fenêtre ne joue qu'un personnage.
    active_character: String,
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
    /// Dernière hauteur demandée pour une fenêtre `Recap` — le bloc mesure ce qu'il occupe et
    /// le renvoie (`RenderOutcome::recap_height`), l'hôte y ajuste la fenêtre OS. Même rôle et
    /// même garde-fou que `last_watchlist_height` (ne pas rappeler `request_inner_size` pour
    /// rien), et même raison de fond : une fenêtre plus haute que son bloc capte les clics sur du
    /// vide. `None` pour toute autre zone.
    last_recap_height: Option<f32>,
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
    /// La fenêtre est-elle actuellement affichée à l'écran ?
    ///
    /// Toujours `true` sauf pour une fenêtre `Combat` quand l'option « Afficher le panneau de
    /// combat en dehors des combats » est décochée (le défaut) et qu'aucun combat n'est en cours —
    /// voir `App::sync_panel_visibility`. Mémorisé ici pour ne pas rappeler `Window::set_visible`
    /// à chaque tick (50 ms) alors que rien n'a changé, comme `is_topmost` pour le z-order.
    visible: bool,
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

impl OverlayWindow {
    /// Cette fenêtre n'appartient à aucune fenêtre de jeu (`game_hwnd` nul) : la fenêtre de
    /// connexion, toujours, et **la fenêtre Options ouverte sans client Wakfu à l'écran**
    /// (2026-09-17, voir `App::open_options_modal`). `sync_windows` la laisse en place — rien ne
    /// se ferme derrière elle — et `sync_topmost` ne la rétrograde jamais : sans fenêtre de jeu
    /// dont suivre le premier plan, elle reste devant jusqu'à ce que l'utilisateur la referme.
    fn is_detached(&self) -> bool {
        self.game_hwnd == HWND::default()
    }
}

/// Ce qu'il faut savoir pour poser une fenêtre `Combat`, et elle seule : de quel côté du client
/// elle se colle, à quelle hauteur l'utilisateur l'a mise, et l'échelle d'affichage de son écran.
///
/// Un type plutôt que trois paramètres de plus à `App::anchor_position`, exactement comme
/// [`RecapAnchor`] — et `CombatAnchor::default()` dit ce qu'il faut passer pour les zones qui n'en
/// lisent rien. Le calcul, lui, est dans `overlay_ui::combat_placement`, partagé avec le binaire
/// X11 ; ce type ne fait que traduire le rectangle de fenêtre de jeu de CETTE plateforme dans son
/// vocabulaire.
#[derive(Debug, Clone, Copy)]
struct CombatAnchor {
    /// Case « Afficher le panneau de combat à droite de la fenêtre de jeu »
    /// (`config::OverlayConfig::combat_on_right`) — le bord vertical, et lui seul.
    on_right: bool,
    /// Hauteur voulue par l'utilisateur (`config::OverlayConfig::combat_position_y`), `None` tant
    /// qu'il ne l'a pas déplacé : le panneau est alors centré comme il l'a toujours été.
    offset: Option<i32>,
    /// Échelle d'affichage de la fenêtre (`Window::scale_factor`) — elle ne sert qu'à convertir la
    /// réserve d'infobulle, voir `combat_placement::Panel::new`.
    scale: f64,
}

impl Default for CombatAnchor {
    /// « À gauche, jamais déplacé », et une échelle neutre : ce que passent les zones qui ne sont
    /// pas le Combat, et qui n'en lisent rien.
    fn default() -> Self {
        Self {
            on_right: false,
            offset: None,
            scale: 1.0,
        }
    }
}

impl CombatAnchor {
    fn new(on_right: bool, offset: Option<i32>, scale: f64) -> Self {
        Self {
            on_right,
            offset,
            scale,
        }
    }

    /// La fenêtre de jeu et celle du panneau, dans le vocabulaire de `combat_placement`.
    ///
    /// `top`/`height` de la fenêtre ENTIÈRE (et non `client_top` comme la Récap et le Suivi) :
    /// c'est le repère sur lequel ce panneau est centré depuis l'origine, et en changer décalerait
    /// le panneau de tout le monde sans que personne l'ait demandé.
    fn geometry(
        self,
        rect: GameRect,
        overlay_width: i32,
        overlay_height: i32,
    ) -> (combat_placement::ClientArea, combat_placement::Panel) {
        (
            combat_placement::ClientArea {
                left: rect.left,
                top: rect.top,
                width: rect.width,
                height: rect.height,
            },
            combat_placement::Panel::new(overlay_width, overlay_height, self.scale),
        )
    }
}

/// Un glissement du panneau Combat en cours (2026-09-17) — le pendant de [`RecapDragState`], en
/// une seule dimension : ce panneau ne se déplace qu'en HAUTEUR (voir `combat_placement`).
#[derive(Debug, Clone, Copy)]
struct CombatDragState {
    /// La fenêtre `Combat` saisie — en multicompte, chaque client a la sienne, et ce n'est pas
    /// parce que l'une est tenue que les autres bougent.
    window: WindowId,
    /// Ordonnée de saisie DANS la fenêtre, en pixels physiques — figée au premier appui.
    grab_y: i32,
}

/// Ce qu'il faut savoir **en plus** pour poser une fenêtre `Recap`, et elle seule : où
/// l'utilisateur a mis la bande, et l'échelle d'affichage de son écran.
///
/// Un type plutôt que deux paramètres de plus à `App::anchor_position` : les quatre autres zones
/// n'en ont que faire, et `RecapAnchor::default()` dit exactement ce qu'il faut passer pour
/// elles (« jamais déplacée, échelle sans objet »). Le calcul, lui, n'est pas ici mais dans
/// `overlay_ui::recap_placement`, partagé avec le binaire X11 — ce type ne fait que traduire le
/// rectangle de fenêtre de jeu de CETTE plateforme dans le vocabulaire de ce module.
#[derive(Debug, Clone, Copy)]
struct RecapAnchor {
    /// Décalage du bloc voulu par l'utilisateur (`config::OverlayConfig::recap_position`),
    /// `None` tant qu'il ne l'a pas déplacé.
    offset: Option<(i32, i32)>,
    /// Échelle d'affichage de la fenêtre du bloc (`Window::scale_factor`) — elle ne sert qu'à
    /// convertir la réserve d'infobulle, voir `recap_placement::Band::new`.
    scale: f64,
}

impl Default for RecapAnchor {
    /// « Jamais déplacée », et une échelle neutre : ce que passent les quatre zones qui ne sont
    /// pas la Récap, et qui n'en lisent rien.
    fn default() -> Self {
        Self {
            offset: None,
            scale: 1.0,
        }
    }
}

impl RecapAnchor {
    fn new(offset: Option<(i32, i32)>, scale: f64) -> Self {
        Self { offset, scale }
    }

    /// La fenêtre de jeu et la fenêtre du bloc, dans le vocabulaire de `recap_placement`.
    ///
    /// `client_top` et non `top` (comme pour l'ancrage du Suivi) : le client Wakfu dessine sa
    /// fausse barre de titre dans sa propre zone cliente, et c'est sous ce bord-là que tout
    /// s'aligne — d'où aussi la hauteur, comptée depuis ce bord jusqu'au bas de la fenêtre.
    fn geometry(
        self,
        rect: GameRect,
        overlay_width: i32,
        overlay_height: i32,
    ) -> (recap_placement::ClientArea, recap_placement::Band) {
        (
            recap_placement::ClientArea {
                left: rect.left,
                top: rect.client_top,
                width: rect.width,
                height: rect.top + rect.height - rect.client_top,
            },
            recap_placement::Band::new(overlay_width, overlay_height, self.scale),
        )
    }
}

/// Un glissement de la bande Récap en cours (2026-09-17) — voir `panels::recap::RecapDrag`, qui
/// dit pourquoi le geste se raconte en POSITIONS et non en écarts.
///
/// Un seul état, comme il n'y a qu'une souris : la fenêtre saisie, et l'endroit du bloc par
/// lequel on l'a attrapée. C'est ce point de saisie, invariant du début à la fin du geste, qui
/// rend le suivi exact — la position visée vaut toujours « où est le curseur à l'écran, moins
/// l'endroit par lequel on tient la bande », sans rien accumuler d'une frame à l'autre. Un
/// cumul d'écarts, lui, dériverait dès que le bornage retiendrait la bande contre un bord.
#[derive(Debug, Clone, Copy)]
struct RecapDragState {
    /// La fenêtre `Recap` saisie — une autre fenêtre de jeu peut en avoir une aussi, et ce n'est
    /// pas elle qu'on déplace.
    window: WindowId,
    /// Position de saisie DANS la fenêtre, en pixels physiques.
    grab: (i32, i32),
}

struct App {
    windows: HashMap<WindowId, OverlayWindow>,
    /// Raccourcis globaux : combinaisons EFFECTIVES (défauts ou personnalisation lue de
    /// `config.toml`), enregistrement auprès de l'OS et table `id -> action` consultée à la
    /// réception (voir `about_to_wait`). Remplace depuis le 2026-09-13 les neuf champs
    /// `*_hotkey_id` et le `GlobalHotKeyManager` qui vivaient ici — voir
    /// `overlay_ui::shortcuts::ShortcutRegistry`, partagé avec le binaire Linux.
    hotkeys: ShortcutRegistry,
    hotkey_events: &'static global_hotkey::GlobalHotKeyEventReceiver,
    interactive: bool,
    snapshot: Arc<ArcSwap<SessionSnapshot>>,
    /// Publié par le thread Engine à chaque lot ingéré (et une fois de plus dès la réception des
    /// entrées suivies par le thread Auth, voir `spawn_engine_thread`) — état COMPLET du Suivi
    /// (définitions + compteurs), déjà fusionné avec les compteurs locaux persistés (voir
    /// `overlay_engine::watchlist`). Global comme `snapshot`, pas par fenêtre : le suivi est un
    /// suivi de compte, pas d'un personnage précis.
    watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
    /// Sélection multiple du bandeau (2026-09-13) — ici et pas dans `OverlayWindow` : elle se
    /// pilote aussi au clavier (raccourci `ShortcutAction::WatchlistRemove`), et un raccourci global arrive
    /// par la boucle d'événements sans savoir quelle fenêtre existe. Il n'y a de toute façon qu'un
    /// bandeau Suivi à la fois.
    watchlist_selection: panels::watchlist::WatchlistSelection,
    /// **Les célébrations de complétion en cours** (2026-09-17) — alimentées par
    /// `completions_rx`, avancées par `tick_watchlist_completions`, lues par le rendu du bandeau.
    watchlist_completions: panels::watchlist::WatchlistCompletions,
    /// Par où les complétions arrivent du thread Engine — voir
    /// [`WatchlistCompleted`].
    completions_rx: mpsc::Receiver<WatchlistCompleted>,
    /// **L'entrée dont la réinitialisation attend confirmation** (2026-09-18) — posée à
    /// l'ouverture de `OverlayKind::ResetConfirm(ResetTarget::WatchlistCounter)`, reprise à la
    /// réponse. Ici et non dans la cible : `OverlayKind` est `Copy` (voir
    /// `ResetTarget::WatchlistCounter`). Prêtée au rendu de la confirmation, qui nomme l'objet.
    watchlist_reset_pending: Option<WatchlistEntry>,
    /// Publié par le thread Engine à chaque décompte de suivi qui vient d'atteindre 0 (voir
    /// `overlay_engine::WatchlistAlert`, §9 du plan « Alertes de drop ») — `None` initialement et
    /// après expiration (voir `WatchlistToast::hide_at`, comparé à `Instant::now()` au rendu).
    watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
    /// Le profil d'alerte du compte, tel que le dernier `GET /api/v1/settings` l'a rendu — lu à
    /// l'OUVERTURE de la fenêtre Options, pour en faire le brouillon de l'onglet « Alertes ».
    alert_profile: SharedAlertProfile,
    /// Les recherches de chat du compte, même provenance et même usage que `alert_profile` — le
    /// brouillon de l'onglet « Chat ».
    chat_filters: SharedChatFilters,
    /// Le roster du compte sous sa forme éditable — le brouillon de l'onglet « Personnages » (voir
    /// `engine_thread::SharedRosterDraft`).
    roster_draft: SharedRosterDraft,
    /// Les serveurs de jeu proposés par le sélecteur de ce même onglet — voir
    /// `background::spawn_game_servers_thread`.
    game_servers: Arc<ArcSwap<GameServers>>,
    /// Réglages de la carte d'alerte de chat EN VIGUEUR (durée, fermeture manuelle) — lus de la
    /// config locale au démarrage, réécrits à la validation de l'onglet « Chat ».
    chat_toast: chat_tab::ChatToastSettings,
    /// Réglages de la carte de **décompte arrivé à zéro** EN VIGUEUR — même provenance et même
    /// politique que `chat_toast` : lus de la config locale au démarrage, réécrits à la validation
    /// de la fenêtre Options (section « Suivi » de l'onglet « Paramètres », 2026-09-16).
    countdown_toast: suivi_tab::CountdownToastSettings,
    /// **Ce que devient un suivi complété** EN VIGUEUR (retrait, animation) — même provenance et
    /// même politique que `countdown_toast`. Lu par `about_to_wait` à chaque complétion reçue du
    /// thread Engine : c'est lui qui décide s'il y a une célébration à jouer et un retrait à
    /// envoyer (voir `panels::suivi_tab::CompletionSettings`).
    completion: suivi_tab::CompletionSettings,
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
    /// Publié par le thread Auth (voir `spawn_auth_thread`) — pilote la fenêtre de connexion et
    /// l'existence même des overlays de jeu (voir `sync_session_windows`).
    auth_status: Arc<ArcSwap<AuthStatus>>,
    /// Signale au thread Auth une commande (`AuthCommand`) : `Retry`/`CancelPairing` depuis la
    /// fenêtre de connexion (`panels::login`), `Disconnect` depuis le bouton « Déconnecter » de
    /// la fenêtre Options (voir `disconnect_account`) ou le menu de l'icône de zone de
    /// notification.
    auth_command_tx: mpsc::Sender<AuthCommand>,
    /// Avancement des chargements initiaux (catalogue, donjons, rattrapage du log — voir
    /// `overlay_ui::startup`) : tant qu'il n'est pas complet, la fenêtre de connexion montre son
    /// écran de chargement et rien d'autre n'existe (voir `sync_session_windows`).
    startup: Arc<StartupProgress>,
    /// État de la mise à jour automatique, publié par le thread de mise à jour
    /// (`background::spawn_update_thread`, 2026-09-15, `docs/plan-mise-a-jour.md` §7) — lu à
    /// chaque tick par `install_update_if_ready`, copié dans la fenêtre de connexion et la
    /// fenêtre Options avant chaque rendu.
    update_status: Arc<ArcSwap<UpdateStatus>>,
    /// Commandes au thread de mise à jour : vérification (bouton « Recherche de mise à jour »),
    /// téléchargement (« Mettre à jour vers X »), nouvelle tentative (« Réessayer »).
    update_command_tx: mpsc::Sender<UpdateCommand>,
    /// Installer automatiquement au démarrage — réglage LOCAL persisté
    /// (`config::OverlayConfig::auto_update`), même politique que `combat_always_visible` : lu
    /// au démarrage (où il décide de la première commande envoyée au thread), remplacé à la
    /// validation de la fenêtre Options, effectif au prochain lancement.
    auto_update: bool,
    /// **Journal détaillé** — réglage LOCAL persisté (`config::OverlayConfig::verbose_log`),
    /// décoché par défaut (constat C6 de `docs/analyse-rgpd.md`). Appliqué au démarrage puis à
    /// chaque validation de la fenêtre Options, à chaud (`logging::set_verbose`) : contrairement
    /// à `auto_update`, il n'attend pas le prochain lancement.
    verbose_log: bool,
    /// Voir la doc de `AppState::settings_tx` et `force_refresh`.
    settings_tx: mpsc::Sender<EngineCommand>,
    log_path: PathBuf,
    /// Le panneau Combat reste-t-il affiché en dehors des combats ? — réglage LOCAL persisté
    /// (`config::OverlayConfig::combat_always_visible`), lu au démarrage et remplacé à la
    /// validation de la fenêtre Options. `false` par défaut : voir `sync_panel_visibility`.
    combat_always_visible: bool,
    /// Le panneau Combat est-il posé à DROITE de la fenêtre de jeu ? — réglage LOCAL persisté
    /// (`config::OverlayConfig::combat_on_right`), même politique que `combat_always_visible` :
    /// lu au démarrage, remplacé à la validation de la fenêtre Options. Deux effets, tous deux
    /// immédiats : l'ancrage de la fenêtre Combat (`anchor_position`) et le miroir de son contenu
    /// (`render_content::RenderContent::combat_on_right`, voir `overlay_ui::mirror`).
    combat_on_right: bool,
    /// **À quelle hauteur le panneau Combat est posé** (2026-09-17) — réglage LOCAL persisté
    /// (`config::OverlayConfig::combat_position_y`), `None` tant que l'utilisateur ne l'a pas fait
    /// glisser. Une seule valeur pour TOUTES les fenêtres de jeu, comme `recap_position` : un seul
    /// panneau Combat par client, une seule hauteur, un seul verrou.
    combat_position_y: Option<i32>,
    /// Le panneau Combat est-il verrouillé en hauteur ? — le cadenas de sa rangée d'actions
    /// (`config::OverlayConfig::combat_locked`, déverrouillé par défaut).
    combat_locked: bool,
    /// Le glissement du panneau Combat **en cours**, s'il y en a un — voir [`CombatDragState`].
    combat_drag: Option<CombatDragState>,
    /// Prévenir par une notification du système qu'un personnage doit jouer ? — réglage LOCAL
    /// persisté (`config::OverlayConfig::turn_notification`), même politique que
    /// `combat_always_visible` : lu au démarrage, remplacé à la validation de la fenêtre Options.
    turn_notification: bool,
    /// La notification de tour sans son (`config::OverlayConfig::turn_notification_muted`) —
    /// même provenance.
    turn_notification_muted: bool,
    /// **Les trois interrupteurs de fonctionnalité** — Suivi, Alertes, Recherche de chat (cases
    /// « Activer … » de la fenêtre Options, voir `panels::feature_switch`). Réglages LOCAUX
    /// persistés (`config::OverlayConfig::features`), même politique que `combat_always_visible` :
    /// lus au démarrage, remplacés à la validation de la fenêtre Options.
    ///
    /// Deux effets ici : le bandeau de suivi n'affiche plus de tuile quand `suivi` est décoché
    /// (voir `render_window`), et le thread Engine cesse de jouer les alertes correspondantes
    /// (`EngineCommand::SetFeatures`, qui porte le détail de ce qui est coupé et de ce qui
    /// continue).
    features: FeatureToggles,
    /// **Les deux sourdines** — cases « Couper le son des notifications » des onglets « Suivi » et
    /// « Chat » (`panels::notifications`). Réglages LOCAUX persistés
    /// (`config::OverlayConfig::alert_mutes`), même politique que `features` : lus au démarrage,
    /// remplacés à la validation de la fenêtre Options.
    ///
    /// Un seul effet ici, et il est ailleurs : le thread Engine cesse de JOUER le son de l'alerte
    /// concernée (`EngineCommand::SetAlertMutes`) — sa carte, elle, continue de s'afficher.
    alert_mutes: AlertMutes,
    /// La surveillance de tour (§9.1 decies) — voir `sync_turn_watch`. Toujours construite, même
    /// option décochée : les gabarits chargés au démarrage servent dès qu'on la coche.
    turn_watcher: turn_watch::watcher::Watcher,
    /// Dernier tick de `sync_turn_watch` : la capture d'une fenêtre est bien plus chère que les
    /// sondages de 20 Hz d'`about_to_wait`, elle a sa propre cadence (`TURN_WATCH_INTERVAL`).
    turn_watch_last_tick: Option<std::time::Instant>,
    game_window: GameWindowTracker,
    /// Le scan précédent trouvait au moins une fenêtre de jeu — pour n'envoyer
    /// `EngineCommand::GameClosed` qu'au passage à « aucune », une fois par fermeture (voir
    /// `sync_windows`).
    game_was_present: bool,
    /// **La session du Récap** (2026-09-17, `overlay_ui::recap_session`) : chrono qui n'avance
    /// que fenêtre de jeu présente, compteurs de session, reprise après une pause tolérée,
    /// remise à zéro confirmée. Nourrie à chaque tick par `sync_windows` (c'est le balayage des
    /// fenêtres de jeu qui lui dit si le joueur joue), lue à chaque frame du bloc Récap
    /// (`panels::recap::RecapView`). Elle remplace `started_at`, l'instant de lancement du
    /// processus dont la durée était dérivée le premier jour.
    recap_session: RecapSession,
    /// **Le compte était-il lié au tick précédent ?** (2026-09-18, constat C5 de
    /// `docs/analyse-rgpd.md` §3.5) — le seul rôle de ce drapeau est de reconnaître la TRANSITION
    /// « connecté -> plus connecté », quelle qu'en soit l'origine : bouton de la section « Compte »,
    /// entrée de la zone de notification, ou jeton refusé par le serveur. Le thread
    /// d'authentification purge alors les données locales à tiers
    /// (`local_data::Scope::OnDisconnect`), mais le récap de session appartient à cet hôte : sans ce
    /// drapeau, l'objet en mémoire réécrirait `recap-session.json` trente secondes plus tard.
    ///
    /// Faux au démarrage : un lancement sans compte lié n'a rien à purger, et l'état de chargement
    /// (`AuthStatus::Connecting`) ne compte pour aucune des deux valeurs.
    account_was_connected: bool,
    /// **Où l'utilisateur a posé la bande Récap** (2026-09-17) — décalage du bloc depuis le coin
    /// de la zone cliente du jeu, `None` tant qu'il ne l'a pas déplacée (voir
    /// `overlay_ui::recap_placement` et `config::OverlayConfig::recap_position`).
    ///
    /// **Une seule position pour toutes les fenêtres de jeu**, et pas une par personnage
    /// (décision utilisateur du 2026-09-17) : elle est RELATIVE à la fenêtre de jeu, donc chaque
    /// client affiche sa bande au même endroit chez lui. Elle vit ici, dans `App`, pour cette
    /// raison — un champ d'`OverlayWindow` en ferait un réglage par fenêtre.
    recap_position: Option<(i32, i32)>,
    /// **La bande Récap est-elle verrouillée ?** (2026-09-17) — le cadenas de sa rangée
    /// d'actions (`panels::recap::RecapChrome::locked`), relu de la config au démarrage et
    /// réécrit à chaque bascule. Ici et non dans `OverlayWindow`, pour la même raison que
    /// `recap_position` : une seule bande pour tous les clients, un seul verrou.
    recap_locked: bool,
    /// Le glissement de la bande Récap **en cours**, s'il y en a un — voir `RecapDragState` et
    /// `panels::recap::RecapDrag`. `None` le reste du temps, c'est-à-dire presque toujours.
    recap_drag: Option<RecapDragState>,
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
    /// Résolution des ingrédients d'une recette en vol, et la fenêtre Options qui l'attend — voir
    /// `App::start_recipe_resolution`. Sondée à chaque `about_to_wait`, comme le dialogue de
    /// fichier : l'appel réseau enchaîne un aller-retour par niveau de recette et n'a aucune raison
    /// de geler le rendu.
    pending_recipe: Option<(
        WindowId,
        mpsc::Receiver<Vec<overlay_engine::RecipeIngredient>>,
    )>,
    /// Icône de zone de notification et son menu (2026-09-14) — `None` tant que `resumed` ne
    /// l'a pas posée, ou si le système l'a refusée (journalisé, jamais fatal). Voir `install_tray`.
    tray: Option<TrayMenu>,
    /// Clics sur le menu de l'icône de zone de notification — même sondage non bloquant que
    /// `hotkey_events`, à chaque `about_to_wait`.
    menu_events: &'static tray_icon::menu::MenuEventReceiver,
    /// **Écran de mise à jour ouvert à la demande** (2026-09-18) — posé par l'entrée « Mise à
    /// jour » du menu de la zone de notification (voir `open_manual_update_window`), retiré par
    /// « Fermer ». Tant qu'il vaut `true`, `sync_session_windows` garde la fenêtre de connexion
    /// ouverte **même compte lié**, en mode mise à jour (`LoginState::manual_update`) : c'est le
    /// seul endroit où cette fenêtre coexiste avec les overlays de jeu.
    manual_update: bool,
}

/// L'icône de zone de notification (`tray-icon`, même auteurs que `global-hotkey`) et les quatre
/// entrées de son menu — gardées pour reconnaître leurs clics (`MenuEvent::id`) et pour activer
/// ou griser « Options » et « Déconnecter » selon qu'un compte est lié (voir `sync_tray_menu`).
///
/// **Menu validé par l'utilisateur (2026-09-14)** : Options / Déconnecter / Quitter, rien d'autre
/// — puis **« Mise à jour » ajoutée après « Options » à sa demande (2026-09-15)** : un clic lance
/// la recherche de mise à jour (`UpdateCommand::Check`, la même que le bouton « Recherche de mise
/// à jour » de la fenêtre Options, §8 de `docs/plan-mise-a-jour.md`) et, **depuis le 2026-09-18,
/// l'affiche** — la fenêtre de connexion/démarrage s'ouvre sur son écran de mise à jour, rouage
/// et verdict compris (voir `open_manual_update_window`). Toujours active : une recherche n'a pas
/// besoin de compte, et le thread ignore de lui-même une demande pendant une opération en cours
/// ou à moins de trente secondes de la précédente. C'est le seul accès à l'overlay quand ni
/// fenêtre de jeu ni fenêtre de connexion ne sont à l'écran — et le seul moyen de quitter
/// proprement une fois connecté (les overlays ancrés sur le jeu n'ont ni croix ni barre des
/// tâches).
struct TrayMenu {
    /// Gardée en vie : l'icône disparaît de la zone de notification à la destruction.
    _icon: TrayIcon,
    options: MenuItem,
    update: MenuItem,
    disconnect: MenuItem,
    quit: MenuItem,
    /// Dernier état appliqué à « Options » et « Déconnecter » — évite un aller-retour Win32 par
    /// tick quand rien n'a changé.
    enabled_for_account: bool,
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
    /// Voir `App::combat_always_visible` — lu de la config au démarrage (`main`), jamais découvert
    /// autrement.
    combat_always_visible: bool,
    /// Voir `App::combat_on_right` — même provenance que `combat_always_visible`.
    combat_on_right: bool,
    /// Voir `App::combat_position_y` — relue de la config au démarrage.
    combat_position_y: Option<i32>,
    /// Voir `App::combat_locked` — relu de la config au démarrage.
    combat_locked: bool,
    /// Voir `App::turn_notification` — même provenance que `combat_always_visible`.
    turn_notification: bool,
    /// Voir `App::turn_notification_muted`.
    turn_notification_muted: bool,
    /// Voir `App::features` — lus de la config au démarrage (`main`).
    features: FeatureToggles,
    /// Voir `App::alert_mutes` — lues de la config au démarrage (`main`).
    alert_mutes: AlertMutes,
    /// Raccourcis EFFECTIFS au démarrage — défauts, ou personnalisation lue de `config.toml`
    /// (`config::OverlayConfig::shortcuts`). Même provenance que `combat_always_visible` : lus une
    /// fois dans `main`, jamais redécouverts.
    shortcuts: ShortcutBindings,
    snapshot: Arc<ArcSwap<SessionSnapshot>>,
    watchlist: Arc<ArcSwap<Vec<WatchlistEntry>>>,
    watchlist_toast: Arc<ArcSwap<Option<WatchlistToast>>>,
    alert_profile: SharedAlertProfile,
    chat_filters: SharedChatFilters,
    roster_draft: SharedRosterDraft,
    game_servers: Arc<ArcSwap<GameServers>>,
    /// Voir `App::chat_toast` — lu de la config au démarrage.
    chat_toast: chat_tab::ChatToastSettings,
    /// Voir `App::countdown_toast` — lu de la config au démarrage.
    countdown_toast: suivi_tab::CountdownToastSettings,
    /// Voir `App::completion` — lus de la config au démarrage.
    completion: suivi_tab::CompletionSettings,
    /// Voir `App::completions_rx` — le canal créé par `main`, avant le thread Engine.
    completions_rx: mpsc::Receiver<WatchlistCompleted>,
    /// Voir `App::recap_session` — relue du disque au démarrage, avec le réglage de la config.
    recap_session: RecapSession,
    /// Voir `App::recap_position` — relue du disque au démarrage (`config::OverlayConfig::
    /// recap_position`), et réécrite à chaque bande reposée.
    recap_position: Option<(i32, i32)>,
    /// Voir `App::recap_locked` — relu de la config au démarrage
    /// (`config::OverlayConfig::recap_locked`).
    recap_locked: bool,
    catalog: Arc<ArcSwap<CatalogIndex>>,
    catalog_stale: Arc<AtomicBool>,
    remote_icons: RemoteIconStore,
    auth_status: Arc<ArcSwap<AuthStatus>>,
    auth_command_tx: mpsc::Sender<AuthCommand>,
    /// Voir `App::startup`.
    startup: Arc<StartupProgress>,
    /// Voir `App::update_status`, `App::update_command_tx`, `App::auto_update`.
    update_status: Arc<ArcSwap<UpdateStatus>>,
    update_command_tx: mpsc::Sender<UpdateCommand>,
    auto_update: bool,
    /// Voir `App::verbose_log`.
    verbose_log: bool,
    /// Conservé (pas seulement transmis au thread Auth) pour permettre à `force_refresh` de
    /// redemander les réglages de compte à la volée — voir sa doc.
    settings_tx: mpsc::Sender<EngineCommand>,
}

impl App {
    fn new(state: AppState) -> Self {
        let AppState {
            log_path,
            combat_always_visible,
            combat_on_right,
            combat_position_y,
            combat_locked,
            turn_notification,
            turn_notification_muted,
            features,
            alert_mutes,
            shortcuts,
            snapshot,
            watchlist,
            watchlist_toast,
            alert_profile,
            chat_filters,
            roster_draft,
            game_servers,
            chat_toast,
            countdown_toast,
            completion,
            completions_rx,
            recap_session,
            recap_position,
            recap_locked,
            catalog,
            catalog_stale,
            remote_icons,
            auth_status,
            auth_command_tx,
            startup,
            update_status,
            update_command_tx,
            auto_update,
            verbose_log,
            settings_tx,
        } = state;

        // Tout ce qui touche aux combinaisons (défauts, personnalisation, enregistrement OS,
        // refus tolérés) est dans `ShortcutRegistry` — voir le commentaire en tête de fichier.
        let hotkeys = ShortcutRegistry::new(&ShortcutAction::ALL, shortcuts);

        // Identité des toasts de tour (nom + logo de l'overlay, voir `turn_watch::notify`) —
        // une clé HKCU et un PNG, idempotents, au démarrage plutôt qu'à la première notification
        // pour que le centre de notifications la connaisse avant le premier toast.
        if let Some(dir) = turn_watch::templates::data_dir() {
            turn_watch::notify::register_identity(&dir, ui_icons::app_logo_png());
        }

        Self {
            windows: HashMap::new(),
            hotkeys,
            hotkey_events: GlobalHotKeyEvent::receiver(),
            interactive: true,
            snapshot,
            watchlist,
            watchlist_selection: panels::watchlist::WatchlistSelection::default(),
            watchlist_completions: Default::default(),
            completions_rx,
            watchlist_reset_pending: None,
            watchlist_toast,
            alert_profile,
            chat_filters,
            roster_draft,
            game_servers,
            chat_toast,
            countdown_toast,
            completion,
            catalog,
            catalog_stale,
            remote_icons,
            auth_status,
            auth_command_tx,
            startup,
            update_status,
            update_command_tx,
            auto_update,
            verbose_log,
            settings_tx,
            log_path,
            combat_always_visible,
            combat_on_right,
            combat_position_y,
            combat_locked,
            combat_drag: None,
            turn_notification,
            turn_notification_muted,
            features,
            alert_mutes,
            turn_watcher: turn_watch::watcher::Watcher::new(turn_watch::templates::load_all()),
            turn_watch_last_tick: None,
            game_window: GameWindowTracker::new(),
            game_was_present: false,
            account_was_connected: false,
            recap_session,
            recap_position,
            recap_locked,
            recap_drag: None,
            banner_printed: false,
            last_foreground_heartbeat: None,
            pending_dialog: None,
            pending_recipe: None,
            tray: None,
            menu_events: MenuEvent::receiver(),
            manual_update: false,
        }
    }

    /// L'overlay est-il prêt à montrer ses panneaux de jeu ? — chargements initiaux terminés
    /// (`startup`) ET compte lié. Tant que ce n'est pas le cas, seule la fenêtre de connexion
    /// existe (voir `sync_session_windows`) ; `sync_windows` et `open_options_modal` s'y refusent.
    fn session_ready(&self) -> bool {
        self.startup.is_complete() && self.auth_status.load().is_connected()
    }

    /// Fait converger les fenêtres sur l'état du démarrage et du compte (2026-09-14, §9.1 undecies
    /// du plan) — trois situations, dans cet ordre :
    ///
    /// 1. **chargement** (un chargement initial pas fini, ou compte en cours de validation) : la
    ///    fenêtre de connexion, seule, sur son écran de chargement — rouage centré, rien d'autre.
    ///    Elle masque tout le démarrage : catalogue, référentiels, rattrapage de `wakfu.log`,
    ///    vérification du jeton et récupération des réglages du compte ;
    /// 2. **compte lié** (et tout chargé) : pas de fenêtre de connexion, `sync_windows` crée
    ///    enfin les overlays de jeu ;
    /// 3. **compte non lié** : la fenêtre de connexion, seule, sur son écran « Vous n'êtes pas
    ///    connecté » (ou appairage, ou erreur).
    ///
    /// Appelée à chaque tick d'`about_to_wait`, AVANT `sync_windows`. À la déconnexion, la fenêtre
    /// Options tombe avec les overlays — sans passer par `close_options_modal`, d'où la reprise
    /// explicite des raccourcis qu'elle avait suspendus. Ce qu'elle contenait appartient au compte
    /// qu'on vient de quitter (c'est ce que sa confirmation annonce).
    fn sync_session_windows(&mut self, event_loop: &ActiveEventLoop) {
        let auth = self.auth_status.load();
        let loading = !self.startup.is_complete() || matches!(**auth, AuthStatus::Connecting);
        let connected = !loading && auth.is_connected();
        let has_login = self.windows.values().any(|w| w.kind == OverlayKind::Login);
        if connected {
            // Seule exception à « compte lié = pas de fenêtre de connexion » : l'écran de mise à
            // jour demandé depuis le menu de la zone de notification, qui vit alors à côté des
            // overlays de jeu jusqu'à « Fermer » (voir `open_manual_update_window`).
            if self.manual_update {
                if !has_login {
                    self.create_login_window(event_loop);
                }
            } else if has_login {
                self.windows.retain(|_, w| w.kind != OverlayKind::Login);
                tracing::info!(
                    "[connexion] compte lié et chargements terminés — fenêtre de connexion fermée, overlays de jeu activés."
                );
            }
            self.account_was_connected = true;
        } else {
            // **Transition « connecté -> plus connecté »** (voir `account_was_connected`) : le récap
            // de session part avec le compte, sinon il serait réécrit juste après la purge du thread
            // d'authentification (`local_data::Scope::OnDisconnect`).
            if self.account_was_connected && !loading {
                let snapshot = self.snapshot.load();
                self.recap_session
                    .purge(&snapshot.totals, std::time::SystemTime::now());
            }
            let had_options = self
                .windows
                .values()
                .any(|w| w.kind == OverlayKind::Options);
            let before = self.windows.len();
            self.windows.retain(|_, w| w.kind == OverlayKind::Login);
            if self.windows.len() != before {
                if had_options {
                    let _ = self.hotkeys.resume();
                }
                tracing::info!(
                    "[connexion] aucun compte lié — {} fenêtre(s) de jeu fermée(s), retour à la fenêtre de connexion.",
                    before - self.windows.len()
                );
            }
            if !has_login {
                self.create_login_window(event_loop);
            }
            // Le compte quitté est acté — mais jamais pendant le chargement, où `connected`
            // est faux sans que personne ne se soit déconnecté (voir le champ).
            if !loading {
                self.account_was_connected = false;
            }
        }
        // L'écran de chargement tombe (ou revient, sur « Se connecter »), l'écran de mise à jour
        // s'ouvre ou se referme : la carte doit se redessiner tout de suite, pas au prochain
        // événement venu d'ailleurs.
        let manual_update = self.manual_update;
        for overlay in self.windows.values_mut() {
            if let Some(state) = overlay.login_state.as_mut() {
                if state.loading != loading {
                    state.loading = loading;
                    overlay.next_redraw_at = Some(std::time::Instant::now());
                    if !loading {
                        tracing::info!("[connexion] chargements terminés — écran de connexion.");
                    }
                }
                if state.manual_update != manual_update {
                    state.manual_update = manual_update;
                    overlay.next_redraw_at = Some(std::time::Instant::now());
                }
                // **La hauteur de l'écran**, que seul l'hôte connaît : le volet « À propos » ne
                // dépasse jamais 80 % de celle-ci (voir `panels::login::CARD_HEIGHT`). Relue à
                // chaque tick plutôt qu'à la création : la fenêtre peut être traînée d'un
                // moniteur à l'autre, et deux moniteurs n'ont pas la même hauteur.
                let monitor_height = overlay
                    .window
                    .current_monitor()
                    .map(|monitor| {
                        monitor.size().height as f32 / overlay.window.scale_factor() as f32
                    })
                    .unwrap_or(login::CARD_HEIGHT);
                if (state.monitor_height - monitor_height).abs() > 1.0 {
                    state.monitor_height = monitor_height;
                }
            }
        }
        self.sync_tray_menu(connected);
    }

    /// Crée la fenêtre de connexion (`OverlayKind::Login`, `panels::login`) : une **fenêtre
    /// logicielle classique**, pas un overlay — visible dans la barre des tâches et la bascule
    /// de fenêtres, focalisable, en z-order normal (jamais `HWND_TOPMOST`), centrée sur l'écran
    /// principal comme le lanceur de Discord. Sans décorations OS : la carte peint son propre
    /// bord, et se déplace par sa bannière (`LoginOutcome::drag_window`).
    ///
    /// Le logo du site (`ui_icons::app_logo_rgba`) est son icône de fenêtre ET son icône de barre
    /// des tâches (`with_taskbar_icon`, distincte sous Windows).
    ///
    /// La hauteur de départ est celle de l'écran d'accueil ; `redraw` la retaille à chaque
    /// changement d'état d'après ce que la carte a réellement occupé, en gardant son centre.
    fn create_login_window(&mut self, event_loop: &ActiveEventLoop) {
        let (rgba, width, height) = ui_icons::app_logo_rgba();
        let icon = match Icon::from_rgba(rgba, width, height) {
            Ok(icon) => Some(icon),
            Err(err) => {
                tracing::warn!("[connexion] icône de fenêtre refusée : {err}");
                None
            }
        };
        let attrs = WindowAttributes::default()
            .with_title("Wakfu Companion Overlay")
            .with_inner_size(winit::dpi::LogicalSize::new(
                login::WINDOW_WIDTH as f64,
                login::INITIAL_HEIGHT as f64,
            ))
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(WindowLevel::Normal)
            .with_resizable(false)
            .with_visible(true)
            .with_window_icon(icon.clone());
        #[cfg(target_os = "windows")]
        let attrs = attrs
            .with_taskbar_icon(icon)
            .with_no_redirection_bitmap(true);

        let window = event_loop
            .create_window(attrs)
            .expect("création de la fenêtre de connexion");
        let window = Arc::new(window);
        // Toujours interactive, jamais assujettie au mode clic-traversant des overlays.
        if let Err(err) = window.set_cursor_hittest(true) {
            tracing::warn!("set_cursor_hittest a échoué à la création : {err}");
        }

        let gpu = pollster::block_on(init_gpu(Arc::clone(&window)));
        let portraits = PortraitAtlas::load(&gpu.egui_ctx);
        let combat_frame = CombatFrame::load(&gpu.egui_ctx);
        let icons = UiIcons::load(&gpu.egui_ctx);

        Self::center_on_primary_monitor(event_loop, &window);
        window.focus_window();

        let now = std::time::Instant::now();
        let overlay = OverlayWindow {
            window,
            gpu,
            kind: OverlayKind::Login,
            avatars: None,
            portraits,
            combat_frame,
            icons,
            remote_icon_textures: RemoteIconTextures::default(),
            combat_side: CombatSide::default(),
            combat_metric: CombatMetric::default(),
            options_state: None,
            // Naît sur l'écran de chargement : `sync_session_windows` la fera basculer dès que
            // les chargements initiaux et la vérification du compte auront répondu.
            login_state: Some(LoginState::new(now)),
            last_login_height: Some(login::INITIAL_HEIGHT),
            game_hwnd: HWND::default(),
            game_rect: GameRect {
                left: 0,
                top: 0,
                width: 0,
                height: 0,
                client_top: 0,
            },
            character_name: "Connexion".to_string(),
            active_character: "Connexion".to_string(),
            last_position: None,
            last_watchlist_width: None,
            last_watchlist_height: None,
            last_recap_height: None,
            visible: true,
            is_topmost: false,
            last_topmost_reassert: None,
            pending_demote_since: None,
            next_redraw_at: Some(now),
        };
        tracing::info!("[connexion] fenêtre de connexion ouverte.");
        self.windows.insert(overlay.window.id(), overlay);
    }

    /// Centre `window` sur l'écran principal (repli : le premier écran connu) — « comme Discord
    /// au lancement ». Pas d'ancrage sur le jeu : cette fenêtre n'en dépend pas.
    fn center_on_primary_monitor(event_loop: &ActiveEventLoop, window: &Window) {
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());
        let Some(monitor) = monitor else {
            return;
        };
        let origin = monitor.position();
        let screen = monitor.size();
        let outer = window.outer_size();
        window.set_outer_position(PhysicalPosition::new(
            origin.x + (screen.width as i32 - outer.width as i32) / 2,
            origin.y + (screen.height as i32 - outer.height as i32) / 2,
        ));
    }

    /// Pose l'icône de zone de notification et son menu — une fois, au premier `resumed`
    /// (`tray-icon` exige une boucle de messages sur le thread appelant, c'est celui de winit).
    /// Jamais fatal : sans icône, l'overlay reste pilotable par ses raccourcis et sa fenêtre de
    /// connexion ; le refus est journalisé.
    fn install_tray(&mut self) {
        if self.tray.is_some() {
            return;
        }
        let menu = Menu::new();
        // « Options » et « Déconnecter » naissent grisés : rien à régler ni à quitter tant
        // qu'aucun compte n'est lié (voir `sync_tray_menu`).
        let options = MenuItem::new("Options", false, None);
        let update = MenuItem::new("Mise à jour", true, None);
        let disconnect = MenuItem::new("Déconnecter", false, None);
        let quit = MenuItem::new("Quitter", true, None);
        if let Err(err) = menu.append_items(&[
            &options,
            &update,
            &PredefinedMenuItem::separator(),
            &disconnect,
            &PredefinedMenuItem::separator(),
            &quit,
        ]) {
            tracing::warn!("[zone de notification] menu refusé : {err}");
            return;
        }
        let (rgba, width, height) = ui_icons::app_logo_rgba();
        let icon = match tray_icon::Icon::from_rgba(rgba, width, height) {
            Ok(icon) => icon,
            Err(err) => {
                tracing::warn!("[zone de notification] icône refusée : {err}");
                return;
            }
        };
        match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Wakfu Companion Overlay")
            .with_icon(icon)
            .build()
        {
            Ok(tray) => {
                tracing::info!(
                    "[zone de notification] icône posée — menu Options / Mise à jour / Déconnecter / Quitter."
                );
                self.tray = Some(TrayMenu {
                    _icon: tray,
                    options,
                    update,
                    disconnect,
                    quit,
                    enabled_for_account: false,
                });
            }
            Err(err) => tracing::warn!("[zone de notification] icône refusée : {err}"),
        }
    }

    /// Active ou grise « Options » et « Déconnecter » selon qu'un compte est lié — appelé à
    /// chaque tick par `sync_session_windows`, effectif seulement à la transition.
    fn sync_tray_menu(&mut self, connected: bool) {
        if let Some(tray) = &mut self.tray {
            if tray.enabled_for_account != connected {
                tray.options.set_enabled(connected);
                tray.disconnect.set_enabled(connected);
                tray.enabled_for_account = connected;
            }
        }
    }

    /// Entrée « Mise à jour » du menu de la zone de notification (2026-09-18, demande de
    /// l'utilisateur) : **la recherche s'affiche**, elle ne court plus en silence.
    ///
    /// La fenêtre montrée est celle de la connexion et du démarrage (`panels::login`, la même
    /// carte de 400 px), passée en mode mise à jour (`LoginState::manual_update`) : le rouage du
    /// jeu et « Recherche d'une mise à jour… », puis le verdict — « Vous êtes déjà à jour »,
    /// « Version X disponible » et son bouton, ou l'échec. Avant, seul le journal et la fenêtre
    /// Options disaient ce que ce clic avait donné.
    ///
    /// Compte lié, cette fenêtre n'existe pas : c'est `manual_update` qui la fait naître et vivre
    /// à côté des overlays de jeu (voir `sync_session_windows`), jusqu'à « Fermer ». Compte non
    /// lié, elle est déjà là et change simplement d'écran.
    ///
    /// La commande part au thread comme avant (`UpdateCommand::Check { install_if_available:
    /// false }`) : il l'ignore s'il travaille déjà, ou à moins de trente secondes de la
    /// vérification précédente — l'écran montre alors tout de suite le verdict qu'il connaît
    /// déjà, ce qui est exactement ce qu'on vient lui demander.
    fn open_manual_update_window(&mut self, event_loop: &ActiveEventLoop) {
        tracing::info!(">>> Recherche de mise à jour (zone de notification).");
        self.manual_update = true;
        let _ = self.update_command_tx.send(UpdateCommand::Check {
            install_if_available: false,
        });
        // Sans attendre le prochain tick : la fenêtre doit apparaître au clic.
        self.sync_session_windows(event_loop);
        if let Some(overlay) = self.windows.values().find(|w| w.kind == OverlayKind::Login) {
            overlay.window.focus_window();
        }
    }

    /// « Fermer » / « Plus tard » de l'écran de mise à jour manuelle : on sort du mode manuel.
    /// `sync_session_windows` referme alors la fenêtre si un compte est lié, ou la ramène à
    /// l'écran de connexion sinon.
    fn close_manual_update_window(&mut self, event_loop: &ActiveEventLoop) {
        if !self.manual_update {
            return;
        }
        tracing::info!("[mise à jour] écran de mise à jour refermé.");
        self.manual_update = false;
        self.sync_session_windows(event_loop);
    }

    /// Clics sur le menu de l'icône de zone de notification — voir `TrayMenu`. « Déconnecter »
    /// agit tout de suite, sans la confirmation de la fenêtre Options : un menu contextuel ne
    /// peut pas en ouvrir, et l'entrée est explicite.
    fn handle_tray_menu(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(event) = self.menu_events.try_recv() {
            let Some(tray) = &self.tray else {
                continue;
            };
            if event.id == *tray.options.id() {
                tracing::info!(">>> Options (zone de notification)");
                self.open_options_modal(event_loop, None, options_modal::OptionsTab::Parametres);
            } else if event.id == *tray.update.id() {
                self.open_manual_update_window(event_loop);
            } else if event.id == *tray.disconnect.id() {
                tracing::info!(">>> Déconnexion du compte demandée (zone de notification).");
                let _ = self.auth_command_tx.send(AuthCommand::Disconnect);
            } else if event.id == *tray.quit.id() {
                logging::log_session_end("Quitter (zone de notification)");
                event_loop.exit();
            }
        }
    }

    /// Scanne les fenêtres de jeu actuellement ouvertes et fait converger `self.windows` vers cet
    /// état : retire les overlays dont le client a fermé, crée un overlay pour chaque nouvelle
    /// fenêtre de jeu trouvée, repositionne les autres. Appelé au premier `resumed()` et à chaque
    /// tick d'`about_to_wait` (comme l'ancien `track_game_window` mono-fenêtre) — idempotent,
    /// rappelable sans risque.
    ///
    /// **Compte non lié ⇒ aucun overlay de jeu** (2026-09-14) : cette méthode ne fait rien tant
    /// qu'`auth_status` n'est pas `Connected` — la fenêtre de connexion est alors la seule
    /// interface (voir `sync_session_windows`), et ne participe jamais à cette convergence.
    fn sync_windows(&mut self, event_loop: &ActiveEventLoop) {
        if !self.session_ready() {
            return;
        }
        let found = self.game_window.scan();
        // Une fenêtre `Combat` créée alors qu'aucun combat n'est en cours naît MASQUÉE quand
        // l'option est décochée — voir `sync_panel_visibility`, qui la fera apparaître au premier
        // combat. Lu ici une fois pour toute la passe.
        let snapshot = self.snapshot.load();

        self.windows.retain(|_, overlay| {
            if overlay.kind == OverlayKind::Login || overlay.is_detached() {
                return true; // n'appartient à aucune fenêtre de jeu
            }
            let still_here = found.iter().any(|(_, info)| info.hwnd == overlay.game_hwnd);
            if !still_here {
                // **La modale Options suit la même règle depuis le 2026-09-12** : elle est
                // rattachée à la fenêtre de jeu depuis laquelle on l'a ouverte (voir
                // `open_options_modal`), donc elle s'en va avec elle. Un écran de réglages qui
                // survivrait au client qu'il configure n'aurait plus de raison d'être à l'écran —
                // et le brouillon qu'il porte ne serait de toute façon plus applicable.
                // Seule exception, ci-dessus : la modale ouverte SANS client à l'écran
                // (`is_detached`), qui n'a aucune fenêtre de jeu derrière laquelle disparaître.
                let quoi = if overlay.kind == OverlayKind::Options {
                    "sa fenêtre Options est fermée"
                } else {
                    "son overlay est retiré"
                };
                tracing::info!(
                    "[fenêtre de jeu] {} fermée — {quoi}.",
                    overlay.character_name
                );
            }
            still_here
        });

        for (character_name, info) in &found {
            for kind in [
                OverlayKind::Combat,
                OverlayKind::Watchlist,
                OverlayKind::Recap,
            ] {
                if let Some(existing) = self
                    .windows
                    .values_mut()
                    .find(|w| w.game_hwnd == info.hwnd && w.kind == kind)
                {
                    Self::reposition(
                        existing,
                        info.rect,
                        self.combat_on_right,
                        self.combat_position_y,
                        self.recap_position,
                    );
                    if existing.active_character != *character_name {
                        existing.active_character = character_name.clone();
                    }
                    continue;
                }
                let visible = match kind {
                    OverlayKind::Combat => panels::combat::should_show(
                        &snapshot,
                        character_name,
                        self.combat_always_visible,
                        self.features.combat,
                    ),
                    // La bande Récap n'a pas de condition de combat : seule sa case la commande
                    // (voir `sync_panel_visibility`, qui la suit ensuite à chaque tick).
                    OverlayKind::Recap => self.features.recap,
                    _ => true,
                };
                let mut overlay = Self::create_overlay_window(
                    event_loop,
                    kind,
                    info.hwnd,
                    character_name.clone(),
                    info.rect,
                    self.interactive,
                    visible,
                    self.combat_on_right,
                    self.combat_position_y,
                    self.recap_position,
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
                overlay.next_redraw_at = Some(std::time::Instant::now());
                self.windows.insert(overlay.window.id(), overlay);
            }
        }
        // **La session du Récap suit ce balayage** (2026-09-17) : au moins une fenêtre de jeu à
        // l'écran, le chrono avance ; plus aucune, il s'arrête — et c'est ici que la reprise
        // (ou la nouvelle session) se décide au retour d'une fenêtre. Voir `recap_session`.
        let game_present = !found.is_empty();
        self.recap_session
            .observe(game_present, &snapshot.totals, std::time::SystemTime::now());
        // **Le dernier client vient de se fermer** (2026-09-18) : le moteur purge du disque les
        // combats en cours abandonnés — voir `EngineCommand::GameClosed`. Au passage seulement,
        // jamais à chaque tick sans client.
        if self.game_was_present && !game_present {
            let _ = self.settings_tx.send(EngineCommand::GameClosed);
        }
        self.game_was_present = game_present;
    }

    /// Affiche ou masque chaque fenêtre `Combat` selon qu'un combat est en cours pour SON
    /// personnage — demande du 2026-09-13. La règle elle-même est
    /// `panels::combat::should_show` (voir sa doc) : cette méthode ne fait que l'appliquer aux
    /// fenêtres OS, que ce binaire seul sait manipuler.
    ///
    /// **Masquer plutôt que détruire la fenêtre** : une fenêtre OS, sa surface wgpu et ses trois
    /// atlas de textures (portraits, cadre, icônes) se recréent en dizaines de millisecondes — les
    /// refaire à chaque combat mettrait ce coût pile au moment où le joueur a besoin de voir ses
    /// dégâts. `set_visible` ne coûte rien et garde la fenêtre prête.
    ///
    /// Un tick de la surveillance de tour (§9.1 decies du plan) : pour chaque fenêtre de jeu en
    /// combat, lire le bas de la fenêtre (`turn_watch::capture`), le donner à la machine d'états
    /// (`turn_watch::watcher`), et honorer ce qu'elle rend — un gabarit à enregistrer, ou une
    /// notification à envoyer.
    ///
    /// Cadencée par `TURN_WATCH_INTERVAL`, pas par le tick de 20 Hz d'`about_to_wait` : une capture
    /// `PrintWindow` coûte quelques millisecondes par fenêtre, et un chrono qui change à la seconde
    /// n'a pas besoin de mieux que 2 Hz. Rien n'est capturé hors combat — le moteur le sait avant
    /// toute lecture d'écran.
    ///
    /// Option décochée : rien. Mais les gabarits restent chargés et le `Watcher` construit, pour
    /// qu'une case cochée en cours de session agisse au tick suivant.
    fn sync_turn_watch(&mut self) {
        if !self.turn_notification {
            return;
        }
        let now = std::time::Instant::now();
        if self
            .turn_watch_last_tick
            .is_some_and(|t| now.duration_since(t) < TURN_WATCH_INTERVAL)
        {
            return;
        }
        self.turn_watch_last_tick = Some(now);

        // Une fenêtre de jeu par client — les overlays Combat et Suivi partagent le même
        // `game_hwnd`, on ne la lit qu'une fois. `(clé, personnage aux commandes, fenêtre)`.
        let mut targets: Vec<(String, String, HWND)> = Vec::new();
        for overlay in self.windows.values() {
            if !matches!(overlay.kind, OverlayKind::Combat | OverlayKind::Watchlist) {
                continue;
            }
            if targets.iter().any(|(_, _, h)| h.0 == overlay.game_hwnd.0) {
                continue;
            }
            targets.push((
                overlay.character_name.clone(),
                overlay.active_character.clone(),
                overlay.game_hwnd,
            ));
        }
        if targets.is_empty() {
            return;
        }

        let snapshot = self.snapshot.load();
        let foreground = unsafe { GetForegroundWindow() };
        for (window, current, hwnd) in targets {
            // Le combat du personnage aux commandes — à défaut celui du titulaire (même combat en
            // pratique : les héros d'une fenêtre combattent ensemble).
            let key = overlay_engine::roster::normalize_wakfu_name(&current);
            let fight = snapshot
                .fight_for_character(&current)
                .or_else(|| snapshot.fight_for_character(&window))
                .map(|fight| {
                    let own = fight
                        .fighters
                        .iter()
                        .filter(|f| {
                            f.is_ally
                                && overlay_engine::roster::normalize_wakfu_name(&f.name) == key
                        })
                        // Le tour le plus récent — une ligne fantôme d'un ancien tour ne doit pas
                        // masquer le sort qui vient d'être lancé.
                        .max_by_key(|f| f.last_turn);
                    turn_watch::watcher::FightFacts {
                        ongoing: fight.ongoing,
                        // Un sort ou des dégâts de qui que ce soit : la phase de placement est
                        // passée.
                        engaged_by_log: fight.fighters.iter().any(|f| {
                            !f.last_turn_casts.is_empty()
                                || f.total_damage != 0
                                || f.total_heal != 0
                        }),
                        cast_len: own.map_or(0, |f| f.last_turn_casts.len()),
                        current_in_fight: own.is_some(),
                    }
                });
            // Pas de capture hors combat : le moteur le sait, inutile de lire l'écran.
            let band = fight
                .as_ref()
                .filter(|f| f.ongoing)
                .and_then(|_| turn_watch::capture::capture_bottom_band(hwnd));
            let events = self.turn_watcher.tick(turn_watch::watcher::TickInput {
                window: &window,
                current: &current,
                band: band.as_ref(),
                fight: fight.as_ref(),
                foreground: foreground.0 == hwnd.0,
                now,
            });
            for event in events {
                match event {
                    turn_watch::watcher::Event::TemplateLearned { character, glyph } => {
                        turn_watch::templates::save(&character, &glyph);
                    }
                    turn_watch::watcher::Event::Notify { character } => {
                        // Nom du personnage en `debug` (constat C6 de `docs/analyse-rgpd.md`) — la
                        // notification, elle, reste tracée en clair : c'est elle qu'on vient
                        // chercher quand le toast n'est pas apparu.
                        tracing::info!("[tour] >>> notification de tour.");
                        tracing::debug!(%character, "[tour] >>> doit jouer");
                        // Le clic ramène la fenêtre de CE personnage au premier plan — par un
                        // process frais lancé sur l'URI du toast (voir `turn_watch::notify`).
                        if let Err(err) = turn_watch::notify::show(
                            &format!("{character} doit jouer"),
                            "C'est à toi — clique pour passer sur sa fenêtre",
                            hwnd.0 as isize,
                        ) {
                            tracing::warn!("[tour] notification en échec : {err}");
                        }
                        // Le toast est silencieux (sons système seuls autorisés, jugés
                        // insipides) : le son est le nôtre — sauf coupé dans les Options.
                        if !self.turn_notification_muted {
                            alert_sound::play_turn_alert();
                        }
                    }
                }
            }
        }
    }

    /// Affiche ou masque les fenêtres dont la présence à l'écran est CONDITIONNELLE — `Combat`
    /// (selon qu'un combat est en cours pour SON personnage, demande du 2026-09-13) et `Recap`
    /// (selon sa case à cocher, 2026-09-16). `Watchlist` n'en est jamais : son bandeau porte le
    /// carré de contrôle, seul accès à la fenêtre Options depuis le jeu.
    ///
    /// **Masquer plutôt que détruire la fenêtre** : une fenêtre OS, sa surface wgpu et ses atlas
    /// de textures se recréent en dizaines de millisecondes — les refaire à chaque combat mettrait
    /// ce coût pile au moment où le joueur a besoin de voir ses dégâts. `set_visible` ne coûte
    /// rien et garde la fenêtre prête.
    ///
    /// Appelée à chaque tick d'`about_to_wait`, juste après `sync_windows` (une fenêtre tout juste
    /// créée est donc déjà au bon état) et avant `sync_topmost` (une fenêtre qui vient de
    /// réapparaître doit être promue dans la même passe) — et sans attendre ce tick à la
    /// validation de la fenêtre Options, pour que le geste et son effet soient dans la même passe.
    fn sync_panel_visibility(&mut self) {
        let snapshot = self.snapshot.load();
        let always = self.combat_always_visible;
        // Le détail des combats coupé masque toutes les fenêtres Combat au prochain tick — c'est
        // la validation de la fenêtre Options qui déclenche la passe (voir `apply_options`, qui
        // appelle cette méthode sans attendre `about_to_wait`).
        let enabled = self.features.combat;
        let recap_enabled = self.features.recap;
        for overlay in self.windows.values_mut() {
            // **Deux zones, une seule passe** (2026-09-16, arrivée de la bande Récap) : elles ont
            // la même politique — masquer plutôt que détruire, voir la doc de cette méthode — et
            // les mêmes précautions de réapparition juste en dessous. Seule la RÈGLE diffère :
            // Combat dépend du combat en cours, Récap de sa seule case à cocher.
            let wanted = match overlay.kind {
                OverlayKind::Combat => {
                    panels::combat::should_show(&snapshot, &overlay.character_name, always, enabled)
                }
                OverlayKind::Recap => recap_enabled,
                _ => continue,
            };
            if wanted == overlay.visible {
                continue;
            }
            overlay.window.set_visible(wanted);
            overlay.visible = wanted;
            if wanted {
                // Même précaution qu'à la repromotion topmost (voir `sync_topmost`, correctif
                // 2026-09-06) : `IDCompositionVisual::SetContent`/`Commit` ne sont joués qu'à la
                // création du swapchain par `wgpu-hal`, jamais rejoués ensuite — une fenêtre qui
                // réapparaît après avoir été masquée (ou qui n'a jamais été montrée depuis sa
                // création) peut donc présenter sans jamais être composée à l'écran. Seule une
                // `Surface` recréée de zéro rejoue ce chemin.
                recreate_surface(&mut overlay.gpu, &overlay.window);
                overlay.next_redraw_at = Some(std::time::Instant::now());
                // Le z-order d'une fenêtre masquée n'a pas été suivi pendant son absence :
                // réaffirmer `HWND_TOPMOST` au prochain tick plutôt qu'à la prochaine échéance
                // périodique (jusqu'à `TOPMOST_REASSERT_INTERVAL` plus tard).
                overlay.last_topmost_reassert = None;
            }
            tracing::info!(
                "[{}] {} — panneau {}",
                if overlay.kind == OverlayKind::Recap {
                    "recap"
                } else {
                    "combat"
                },
                overlay.character_name,
                if wanted { "affiché" } else { "masqué" }
            );
        }
    }

    /// Position ancrée sur la fenêtre de jeu selon la zone (voir `OverlayKind`) : Combat est collé
    /// à un bord VERTICAL, centré verticalement (comportement d'origine, S1/L2) ; Suivi est
    /// désormais collé au bord HAUT, centré horizontalement — demande utilisateur explicite
    /// 2026-09-01, à l'image du bandeau du web (`tracker-strip.component`).
    ///
    /// `combat_on_right` (2026-09-17) décide DUQUEL des deux bords verticaux il s'agit — case
    /// « Afficher le panneau de combat à droite de la fenêtre de jeu » des Options. Il ne concerne
    /// que la zone Combat ; le contenu de la fenêtre, lui, est retourné en miroir par le rendu
    /// (voir `overlay_ui::mirror`), les deux allant toujours ensemble.
    fn anchor_position(
        kind: OverlayKind,
        rect: GameRect,
        overlay_width: i32,
        overlay_height: i32,
        combat: CombatAnchor,
        recap: RecapAnchor,
    ) -> PhysicalPosition<i32> {
        match kind {
            // Combat : collé à un bord vertical (`CombatAnchor::on_right`), centré verticalement
            // tant que l'utilisateur ne l'a pas fait glisser, à sa hauteur ensuite (2026-09-17) —
            // tout le calcul, bornage compris, est dans `overlay_ui::combat_placement`, partagé
            // avec le binaire X11. La marge au bord (`GAME_EDGE_MARGIN_PX`) y est nulle des deux
            // côtés, comme ici auparavant : « comme si l'overlay faisait partie du jeu ».
            OverlayKind::Combat => {
                let (client, panel) = combat.geometry(rect, overlay_width, overlay_height);
                let (x, y) = combat_placement::window_position(
                    combat.offset,
                    combat.on_right,
                    client,
                    panel,
                );
                PhysicalPosition::new(x, y)
            }
            OverlayKind::Watchlist => PhysicalPosition::new(
                rect.left + (rect.width - overlay_width) / 2,
                rect.client_top + GAME_TOP_MARGIN_PX,
            ),
            // Récap : sous les boutons du jeu et leurs infobulles tant que l'utilisateur ne l'a
            // pas déplacée, là où il l'a posée ensuite (2026-09-17) — tout le calcul, bornage
            // compris, est dans `overlay_ui::recap_placement`, partagé avec le binaire X11. Le
            // décalage vise le BLOC ; la fenêtre, elle, commence `RECAP_TOOLTIP_RESERVE` px plus
            // haut (la marge que `paint_content` lui donne pour ses infobulles).
            OverlayKind::Recap => {
                let (client, band) = recap.geometry(rect, overlay_width, overlay_height);
                let (x, y) = recap_placement::window_position(recap.offset, client, band);
                PhysicalPosition::new(x, y)
            }
            // La confirmation de remise à zéro couvre la fenêtre de jeu ENTIÈRE, barre de titre
            // comprise : son voile part du coin de la fenêtre, pas de la zone cliente.
            OverlayKind::ResetConfirm(_) => PhysicalPosition::new(rect.left, rect.top),
            // **Rattachée à une fenêtre de jeu, la fenêtre Options EST la fenêtre de jeu**
            // (2026-09-17) : elle la couvre entière, barre de titre comprise, comme la
            // confirmation ci-dessus — son voile part du coin, et c'est le rendu qui centre la
            // modale dedans (`RenderContent::veiled`). Jusque-là elle était centrée sur les deux
            // axes à sa propre taille (2026-09-08, « au centre de l'écran de l'utilisateur au
            // niveau du jeu ») ; ce centrage vaut toujours, il a seulement changé d'étage.
            //
            // Détachée (`is_detached`), elle garde sa taille et ce bras n'est pas lu :
            // `center_on_primary_monitor` la place, et `reposition` ne la voit jamais.
            OverlayKind::Options => PhysicalPosition::new(rect.left, rect.top),
            // Jamais ancrée sur le jeu — centrée sur l'écran par `center_on_primary_monitor`,
            // et jamais repositionnée ensuite (`reposition` ne la voit pas, `sync_windows`
            // l'ignore). Ce bras n'est là que pour l'exhaustivité.
            OverlayKind::Login => PhysicalPosition::new(0, 0),
        }
    }

    // Neuf paramètres depuis que le panneau Combat se pose à droite et que la bande Récap se
    // déplace (2026-09-17) : ce sont les caractéristiques d'UNE fenêtre à créer, toutes distinctes
    // et toutes obligatoires. Un struct de paramètres ne ferait que déplacer la liste d'un cran,
    // pour trois appelants.
    #[allow(clippy::too_many_arguments)]
    fn create_overlay_window(
        event_loop: &ActiveEventLoop,
        kind: OverlayKind,
        game_hwnd: HWND,
        character_name: String,
        rect: GameRect,
        interactive: bool,
        visible: bool,
        // `combat_on_right` : voir `anchor_position` — le bord vertical où la zone Combat se colle.
        combat_on_right: bool,
        // À quelle hauteur poser le panneau Combat (`App::combat_position_y`, 2026-09-17) — sans
        // objet pour les autres zones.
        combat_offset: Option<i32>,
        // Où poser la bande Récap (`App::recap_position`) — sans objet pour les autres zones,
        // qui n'en lisent rien.
        recap_offset: Option<(i32, i32)>,
    ) -> OverlayWindow {
        let size = match kind {
            OverlayKind::Combat => WINDOW_SIZE,
            // 0 entrée à la création : rien n'est encore chargé (compte/catalogue), la fenêtre
            // démarre donc au plus étroit (juste la rangée de boutons) et s'élargit dès que
            // `watchlist` se remplit (voir le redimensionnement dans `RedrawRequested`). Suivi
            // supposé ACTIF ici (`true`) faute d'accès à `self` : c'est le défaut de la config, et
            // le premier `RedrawRequested` rétrécit la fenêtre d'une rangée de quatre boutons à
            // une rangée de deux si la case est décochée (2026-09-15) — la même frame que celle
            // qui vide déjà le bandeau de ses tuiles.
            OverlayKind::Watchlist => (
                watchlist_target_width(0, true, false, rect.width),
                watchlist_target_height(false, false),
            ),
            // Rattachée à une fenêtre de jeu : SA taille, pour que le voile la couvre en entier,
            // overlays compris (2026-09-17, voir `RenderContent::veiled`). Détachée (`game_hwnd`
            // nul, aucun client à l'écran) : celle de la modale seule, sans voile.
            OverlayKind::Options if game_hwnd != HWND::default() => {
                (rect.width as f64, rect.height as f64)
            }
            OverlayKind::Options => (
                options_modal::WINDOW_SIZE.0 as f64,
                options_modal::WINDOW_SIZE.1 as f64,
            ),
            // Récap : largeur FIXE, celle de la rangée de boutons du jeu (`panels::recap::WIDTH`) ;
            // la hauteur naît à trois lignes et suit le contenu (voir le redimensionnement dans
            // `RedrawRequested`, même mécanique que le Suivi), plus la réserve des infobulles,
            // qui s'ouvrent au-dessus (marge haute du contenu).
            OverlayKind::Recap => (
                panels::recap::WIDTH as f64,
                panels::recap::HEIGHT as f64
                    + render_content::RECAP_TOOLTIP_RESERVE as f64
                    + render_content::RECAP_ACTIONS_RESERVE as f64,
            ),
            // La taille de la fenêtre de jeu, pour que le voile la couvre en entier — overlays
            // compris, puisque cette fenêtre est créée après eux et donc au-dessus.
            OverlayKind::ResetConfirm(_) => (rect.width as f64, rect.height as f64),
            // Créée par `create_login_window`, jamais par ici — voir sa doc.
            OverlayKind::Login => (login::WINDOW_WIDTH as f64, login::INITIAL_HEIGHT as f64),
        };
        // **Une fenêtre qui couvre le jeu se mesure en pixels PHYSIQUES** : `GameRect` vient de
        // `GetWindowRect`, et la position posée plus bas (`set_outer_position`) est physique
        // aussi. Une taille logique serait multipliée par l'échelle d'affichage (125 % : un voile
        // d'un quart plus grand que le jeu, débordant en bas et à droite). Les autres zones ont
        // des tailles de MAQUETTE, en points logiques, et restent logiques.
        let covers_game = matches!(kind, OverlayKind::ResetConfirm(_))
            || (kind == OverlayKind::Options && game_hwnd != HWND::default());
        let inner_size: winit::dpi::Size = if covers_game {
            PhysicalSize::new(size.0, size.1).into()
        } else {
            winit::dpi::LogicalSize::new(size.0, size.1).into()
        };
        let title_suffix = match kind {
            OverlayKind::Combat => "Combat",
            OverlayKind::Watchlist => "Suivi",
            OverlayKind::Recap => "Recap",
            OverlayKind::ResetConfirm(_) => "Confirmation",
            OverlayKind::Options => "Options",
            OverlayKind::Login => "Connexion",
        };
        let attrs = WindowAttributes::default()
            .with_title(format!(
                "wakfu-companion-overlay — {character_name} — {title_suffix}"
            ))
            .with_inner_size(inner_size)
            .with_transparent(true)
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false)
            // Une fenêtre `Combat` peut naître MASQUÉE (aucun combat en cours, option décochée —
            // voir `sync_panel_visibility`) : demandé dès les attributs plutôt que par un
            // `set_visible(false)` juste après la création, qui la laisserait clignoter à l'écran
            // le temps d'une frame. Toujours `true` pour Suivi et Options.
            .with_visible(visible);
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
        // La confirmation de remise à zéro aussi (2026-09-17) : Échap doit pouvoir répondre
        // « Non », il lui faut le focus clavier.
        if matches!(kind, OverlayKind::Options | OverlayKind::ResetConfirm(_)) {
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
        // Payés seulement là où ils servent — voir le champ `avatars` d'`OverlayWindow`.
        let avatars = (kind == OverlayKind::Options).then(|| AvatarAtlas::load(&gpu.egui_ctx));

        let outer = window.outer_size();
        let position = Self::anchor_position(
            kind,
            rect,
            outer.width as i32,
            outer.height as i32,
            CombatAnchor::new(combat_on_right, combat_offset, window.scale_factor()),
            RecapAnchor::new(recap_offset, window.scale_factor()),
        );
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
            avatars,
            remote_icon_textures: RemoteIconTextures::default(),
            combat_side: CombatSide::default(),
            combat_metric: CombatMetric::default(),
            // Renseigné juste après par l'appelant (`open_options_modal`) pour `kind == Options`
            // — `None` ici pour Combat/Suivi, jamais consulté (voir `RenderContent::options`).
            options_state: (kind == OverlayKind::Options).then(OptionsModalState::default),
            login_state: None,
            last_login_height: None,
            game_hwnd,
            game_rect: rect,
            active_character: character_name.clone(),
            character_name,
            last_position: Some(position),
            // Déjà la largeur demandée ci-dessus (`size.0`) pour une fenêtre `Suivi` — la première
            // vérification dans `RedrawRequested` ne redemande donc rien tant que le nombre
            // d'entrées reste 0. `None` pour `Combat`, qui ne redimensionne jamais.
            last_watchlist_width: (kind == OverlayKind::Watchlist).then_some(size.0),
            last_watchlist_height: (kind == OverlayKind::Watchlist).then_some(size.1),
            // Déjà la largeur demandée ci-dessus pour une fenêtre `Recap` — même principe que le
            // Suivi juste au-dessus : la première frame ne redemande rien si elle tombe dessus.
            last_recap_height: (kind == OverlayKind::Recap).then_some(panels::recap::HEIGHT),
            visible,
            is_topmost: true, // WindowLevel::AlwaysOnTop déjà appliqué ci-dessus à la création
            last_topmost_reassert: None,
            pending_demote_since: None,
            next_redraw_at: None,
        }
    }

    /// Recolle une fenêtre overlay sur sa fenêtre de jeu selon son ancrage (voir
    /// `anchor_position`) ; n'appelle `set_outer_position` que si la position cible a changé, pour
    /// ne pas spammer le compositeur DWM 20×/s pour rien.
    fn reposition(
        overlay: &mut OverlayWindow,
        rect: GameRect,
        combat_on_right: bool,
        combat_offset: Option<i32>,
        recap_offset: Option<(i32, i32)>,
    ) {
        overlay.game_rect = rect;
        let outer = overlay.window.outer_size();
        let scale = overlay.window.scale_factor();
        let desired = Self::anchor_position(
            overlay.kind,
            rect,
            outer.width as i32,
            outer.height as i32,
            CombatAnchor::new(combat_on_right, combat_offset, scale),
            RecapAnchor::new(recap_offset, scale),
        );
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

    /// Redessine la ou les fenêtres du bandeau Suivi — appelé après une bascule venue du clavier,
    /// que rien d'autre ne signale au moteur de rendu. Même mécanisme que `toggle_interactive` :
    /// un `next_redraw_at` dans le passé immédiat, que la boucle honore au prochain tour.
    fn request_watchlist_redraw(&mut self) {
        for overlay in self.windows.values_mut() {
            if overlay.kind == OverlayKind::Watchlist {
                overlay.next_redraw_at = Some(std::time::Instant::now());
            }
        }
    }

    fn toggle_interactive(&mut self) {
        self.interactive = !self.interactive;
        for overlay in self.windows.values_mut() {
            // La fenêtre de connexion n'est pas un overlay : toujours interactive.
            if overlay.kind == OverlayKind::Login {
                continue;
            }
            if let Err(err) = overlay.window.set_cursor_hittest(self.interactive) {
                tracing::warn!("set_cursor_hittest a échoué : {err}");
            }
            overlay.next_redraw_at = Some(std::time::Instant::now());
        }
        tracing::info!(
            ">>> Bascule ({}) : mode = {}",
            self.hotkeys.bindings().label(ShortcutAction::Toggle),
            if self.interactive {
                "INTERACTIF"
            } else {
                "CLIC-TRAVERSANT"
            }
        );
    }

    /// `ShortcutAction::Refresh` : demande explicite de l'utilisateur (2026-09-02) — « il faut trouver
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
            // La fenêtre de connexion n'est ni topmost ni « outil » : rien à réaffirmer.
            if overlay.kind == OverlayKind::Login {
                overlay.next_redraw_at = Some(std::time::Instant::now());
                continue;
            }
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
            overlay.next_redraw_at = Some(std::time::Instant::now());
        }
        self.sync_topmost();
        // **(e) Le flux de log lui-même** (2026-09-17) — un panneau Combat qui répond mais n'avance
        // plus (« les boutons marchent, les dégâts ne montent plus ») ne se répare ni par un
        // redessin ni par une réaffirmation topmost : ce qu'il affiche est fidèle au dernier
        // `SessionSnapshot` publié, c'est la publication qui s'est arrêtée. Ce raccourci étant
        // voulu comme la réponse universelle à « quelque chose s'est mal affiché » (voir plus
        // haut), il relit désormais `wakfu.log` en entier et reconstruit la session — voir
        // `EngineCommand::ResyncLog`. Le chien de garde du thread Engine (`IngestWatchdog`) fait
        // de même tout seul au bout de huit secondes ; ce chemin-ci n'attend pas.
        let _ = self.settings_tx.send(EngineCommand::ResyncLog);
        let settings_tx = self.settings_tx.clone();
        // Capturé AVANT le `move` : le thread n'a pas accès à `self` (et la combinaison peut de
        // toute façon changer entre-temps, la fenêtre Options étant ouvrable pendant l'appel).
        let refresh_label = self.hotkeys.bindings().label(ShortcutAction::Refresh);
        thread::spawn(move || match overlay_sync::token_store::load_token() {
            Some(token) => match overlay_sync::client::fetch_settings(&token) {
                Ok(settings) => {
                    tracing::info!(
                        ">>> Réglages de compte redemandés ({refresh_label}) : {} entrée(s) de suivi.",
                        settings.watchlist.len()
                    );
                    let _ = settings_tx.send(EngineCommand::ApplySettings(settings));
                }
                Err(err) => {
                    tracing::warn!(
                        ">>> Échec de la nouvelle demande de réglages ({refresh_label}) : {err}"
                    );
                }
            },
            None => tracing::info!(
                ">>> Aucun jeton de compte stocké — rien à redemander ({refresh_label})."
            ),
        });
        tracing::info!(
            ">>> Rafraîchissement forcé ({})",
            self.hotkeys.bindings().label(ShortcutAction::Refresh)
        );
    }

    /// Déconnexion volontaire du compte lié (lot L4, §7.2/§14 point 3 du plan) — jusqu'à son
    /// introduction, la seule façon de révoquer une session native depuis l'overlay était d'aller
    /// effacer le jeton à la main sur disque/dans le trousseau.
    ///
    /// **Déclenchée par le bouton « Déconnecter » de l'onglet « Paramètres »** (section « Compte »,
    /// `OptionsModalAction::Disconnect`) depuis le 2026-09-13 ; c'était jusque-là un raccourci
    /// global (`Ctrl+Alt+D`), retiré à la demande de l'utilisateur — voir `overlay_ui::shortcuts`.
    /// Purement une commande envoyée au thread Auth (voir `spawn_auth_thread`)
    /// : c'est LUI qui efface le jeton (trousseau + repli fichier) et notifie le thread Engine
    /// (`EngineCommand::Disconnect`, roster et Suivi vidés) — jamais depuis ce thread (winit)
    /// directement, même raison que `force_refresh` (l'accès trousseau/fichier ne doit jamais
    /// bloquer le rendu). L'hôte voit ensuite `AuthStatus::Disconnected` et ferme tous les
    /// overlays pour ne laisser que la fenêtre de connexion (`sync_session_windows`, 2026-09-14).
    /// Sans effet si aucun compte n'est actuellement lié (voir `AuthCommand::Disconnect`, ignoré
    /// par le thread Auth hors de l'état `Connected`) ; pendant un appairage en cours, vaut
    /// annulation (voir `attempt_connect`). Pendant la validation d'un jeton (`Connecting`), le
    /// thread Auth est occupé dans `attempt_connect` ; la déconnexion redeviendra
    /// effective au prochain appui une fois cette tentative résolue.
    fn disconnect_account(&mut self) {
        let _ = self.auth_command_tx.send(AuthCommand::Disconnect);
        tracing::info!(">>> Déconnexion du compte demandée (fenêtre Options).");
    }

    /// **« Supprimer les données locales », confirmé** (2026-09-18, constat C5 de
    /// `docs/analyse-rgpd.md` §3.5) — depuis « Vos données » de l'onglet « À propos » de la
    /// fenêtre Options (`OptionsModalAction::PurgeLocalData`) ou depuis la fenêtre de connexion
    /// (`RenderOutcome::purge_local_data`, le seul chemin quand aucun compte n'est lié).
    ///
    /// Efface tout ce que l'overlay a écrit sur cette machine, puis **arrête le programme** par le
    /// même chemin que « Fermer l'overlay » : c'est la seule façon de garantir qu'aucun thread ne
    /// réécrive ce qui vient de partir (voir `local_data`, doc de module). La borne de fin de
    /// session est posée AVANT la purge — après, il n'y a plus de journal où l'écrire.
    fn purge_local_data_and_quit(&mut self, event_loop: &ActiveEventLoop) {
        tracing::warn!(
            ">>> Effacement des données locales confirmé — l'overlay efface tout ce qu'il a écrit \
             sur cette machine, puis se ferme."
        );
        logging::log_session_end("Supprimer les données locales");
        let snapshot = self.snapshot.load();
        let _ = overlay_ui::local_data::purge_everything_before_shutdown(
            &mut self.recap_session,
            &snapshot.totals,
        );
        event_loop.exit();
    }

    /// **Installe la mise à jour prête et relance** — appelé à chaque tick d'`about_to_wait`, AVANT
    /// tout le reste. Le thread de mise à jour s'arrête à `ReadyToInstall` (exe vérifié sur le
    /// disque, voir `background::spawn_update_thread`) : le remplacement de l'exe courant et la
    /// relance doivent précéder `event_loop.exit()`, que seul ce thread peut appeler. À ce moment,
    /// `StartupProgress::is_update_blocking` est levé depuis le début du téléchargement : seule la
    /// fenêtre de connexion existe (voir `sync_session_windows`), aucun overlay de jeu n'est
    /// interrompu. Un échec de remplacement (antivirus, dossier protégé) repasse en `Failed` et
    /// rend la main : l'overlay continue avec sa version, ou reste sur « Mise à jour requise » si
    /// elle était obligatoire.
    fn install_update_if_ready(&mut self, event_loop: &ActiveEventLoop) {
        let status = self.update_status.load();
        let UpdateStatus::ReadyToInstall {
            version,
            staged,
            mandatory,
        } = &**status
        else {
            return;
        };
        tracing::info!(
            "[mise à jour] installation de {} depuis {} — l'overlay va se relancer.",
            version,
            staged.display()
        );
        self.update_status.store(Arc::new(UpdateStatus::Installing {
            version: version.clone(),
        }));
        match update_apply::install_and_relaunch(staged, build_info::VERSION) {
            Ok(()) => {
                logging::log_session_end(&format!(
                    "mise à jour {} → {version}",
                    build_info::VERSION
                ));
                event_loop.exit();
            }
            Err(err) => {
                tracing::warn!("[mise à jour] installation impossible : {err}");
                self.update_status.store(Arc::new(UpdateStatus::Failed {
                    headline: "Installation impossible".to_string(),
                    detail: err.to_string(),
                    mandatory: *mandatory,
                }));
                self.startup.set_update_blocking(*mandatory);
                if !*mandatory {
                    self.startup.mark_update_resolved();
                }
                self.request_all_redraw();
            }
        }
    }

    /// « Mettre à jour vers X » confirmé dans la fenêtre Options (`OptionsModalAction::InstallUpdate`)
    /// : la fenêtre se ferme, le démarrage repasse en « mise à jour en cours » (ce qui ferme les
    /// overlays de jeu et ramène la fenêtre de connexion sur son écran de chargement au prochain
    /// tick, voir `sync_session_windows`), et le thread de mise à jour télécharge. L'installation
    /// suit dans `install_update_if_ready`.
    /// Redessine toutes les fenêtres au prochain tick — un état partagé vient de changer hors
    /// d'un événement (même effet qu'un `UserEvent`, voir `user_event`).
    fn request_all_redraw(&mut self) {
        for overlay in self.windows.values_mut() {
            overlay.next_redraw_at = Some(std::time::Instant::now());
        }
    }

    fn request_update_install(&mut self, options_window_id: WindowId) {
        tracing::info!(">>> Mise à jour demandée (fenêtre Options).");
        self.startup.set_update_blocking(true);
        let _ = self.update_command_tx.send(UpdateCommand::Download);
        self.close_options_modal(options_window_id, "Mise à jour");
    }

    /// `ShortcutAction::Details` : même action que le clic sur le bouton "Détails" (lien externe)
    /// du carré de contrôle (`panels::watchlist::control_button_row`) — voir sa doc pour
    /// `base_url()`. `open::that` est best-effort (résultat ignoré, même choix que le clic direct) :
    /// un navigateur qui ne s'ouvre pas n'est pas une raison de faire quoi que ce soit d'autre
    /// planter.
    fn open_details(&self) {
        let _ = open::that(overlay_sync::client::base_url());
        tracing::info!(
            ">>> Détails ({}) : ouverture du site.",
            self.hotkeys.bindings().label(ShortcutAction::Details)
        );
    }

    /// `ShortcutAction::InvitePartner` / `ShortcutAction::FollowPartner` : tape `/i "<nom>"` ou
    /// `/fol "<nom>"` dans le chat de la fenêtre de jeu au premier plan, en visant le personnage de
    /// l'AUTRE fenêtre — voir `chat_command`, doc de module, pour toute la mécanique et ses limites.
    ///
    /// **Un scan frais plutôt que `self.windows`** : `EnumWindows` énumère dans l'ordre de
    /// PROFONDEUR (la fenêtre la plus récemment au premier plan d'abord), ce dont
    /// `chat_command::partner_character` se sert pour désigner un partenaire stable au-delà de deux
    /// clients ; `self.windows` est une `HashMap` d'overlays, sans ordre et à deux entrées par
    /// fenêtre de jeu (Combat + Suivi). Le scan coûte un `EnumWindows`, déjà fait à chaque tick
    /// (`sync_windows`) — négligeable pour un geste manuel.
    /// Réponse en privé depuis la carte d'alerte de chat — vise la fenêtre de jeu au premier plan,
    /// ou la première trouvée : l'overlay ne prend jamais le focus (`WS_EX_NOACTIVATE`), le jeu
    /// l'a donc encore le plus souvent, et `send_whisper` le lui rend sinon.
    fn whisper_from_toast(&mut self, author: &str) {
        let foreground = unsafe { GetForegroundWindow() }.0 as usize;
        let windows: Vec<usize> = self
            .game_window
            .scan()
            .into_iter()
            .map(|(_, info)| info.hwnd.0 as usize)
            .collect();
        let target = windows
            .iter()
            .copied()
            .find(|key| *key == foreground)
            .or_else(|| windows.first().copied());
        // Le nom du destinataire est celui d'un TIERS (constat C6 de `docs/analyse-rgpd.md`) :
        // l'action se journalise, pas la personne visée — le nom part en `debug`.
        tracing::info!(">>> Répondre en privé à l'auteur de l'alerte de chat.");
        tracing::debug!(author, ">>> Répondre en privé");
        chat_command::send_whisper(author, target);
    }

    fn send_partner_command(&mut self, command: ChatCommand) {
        let label = self.hotkeys.bindings().label(match command {
            ChatCommand::Invite => ShortcutAction::InvitePartner,
            ChatCommand::Follow => ShortcutAction::FollowPartner,
        });
        // Clé numérique (`HWND` réduite à son entier) plutôt que la `HWND` elle-même : c'est ce
        // que `chat_command::partner_character` compare, et cela lui laisse la même forme des deux
        // côtés (XID `u32` sous X11) — une seule fonction pure, aucun type d'OS dans sa signature.
        let foreground = unsafe { GetForegroundWindow() }.0 as usize;
        let windows: Vec<(String, usize)> = self
            .game_window
            .scan()
            .into_iter()
            .map(|(character, info)| (character, info.hwnd.0 as usize))
            .collect();
        match chat_command::partner_character(&windows, &foreground) {
            Ok(partner) => {
                tracing::info!(">>> {} ({label}) : {partner}", command.label());
                chat_command::send(command, partner);
            }
            // Jamais une erreur remontée à l'utilisateur : il n'y a rien à réparer, seulement un
            // contexte où le geste n'a pas de sens (voir `PartnerError`).
            Err(err) => tracing::info!(">>> {} ({label}) : {}", command.label(), err.message()),
        }
    }

    /// `ShortcutAction::CombatSide` : bascule Alliés/Ennemis (`CombatSide::toggled`) de CHAQUE fenêtre Combat
    /// actuellement ouverte, pas seulement celle au premier plan — même portée globale que les
    /// autres hotkeys de cette liste. Sans effet sur les fenêtres Suivi (`combat_side` n'a de sens
    /// que pour `OverlayKind::Combat`, voir sa doc dans `OverlayWindow`).
    fn toggle_combat_side(&mut self) {
        for overlay in self.windows.values_mut() {
            if overlay.kind == OverlayKind::Combat {
                overlay.combat_side = overlay.combat_side.toggled();
                overlay.next_redraw_at = Some(std::time::Instant::now());
            }
        }
        tracing::info!(
            ">>> Bascule Alliés/Ennemis ({})",
            self.hotkeys.bindings().label(ShortcutAction::CombatSide)
        );
    }

    /// `ShortcutAction::CombatMetric` : fait tourner la grandeur mesurée (`CombatMetric::next`) de
    /// CHAQUE fenêtre Combat ouverte — même portée et même logique que `toggle_combat_side`
    /// ci-dessus, dont c'est le pendant pour le second switch du bandeau.
    fn cycle_combat_metric(&mut self) {
        for overlay in self.windows.values_mut() {
            if overlay.kind == OverlayKind::Combat {
                overlay.combat_metric = overlay.combat_metric.next();
                overlay.next_redraw_at = Some(std::time::Instant::now());
            }
        }
        tracing::info!(
            ">>> Grandeur du combat ({})",
            self.hotkeys.bindings().label(ShortcutAction::CombatMetric)
        );
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
            if overlay.kind == OverlayKind::Login || overlay.is_detached() {
                continue;
            }
            let this_relevant =
                overlay.game_hwnd == foreground || Self::hwnd_of(&overlay.window) == foreground;
            if this_relevant && !relevant_game_hwnds.contains(&overlay.game_hwnd) {
                relevant_game_hwnds.push(overlay.game_hwnd);
            }
        }

        for overlay in self.windows.values_mut() {
            // **La fenêtre de connexion n'y participe jamais** (2026-09-14) : c'est une fenêtre
            // logicielle ordinaire, en z-order normal — elle passe derrière ce que l'utilisateur
            // active, et revient par la barre des tâches, comme n'importe quelle application.
            if overlay.kind == OverlayKind::Login {
                continue;
            }
            // **La modale Options participe à ce calcul comme les autres depuis le 2026-09-12.**
            // Elle en était exclue — `HWND_TOPMOST` posé à sa création et jamais remis en
            // question — parce qu'elle naissait sans `game_hwnd` : elle restait donc au-dessus de
            // TOUT, navigateur ou autre jeu compris, y compris quand l'utilisateur avait
            // manifestement l'attention ailleurs (retour utilisateur 2026-09-12). Rattachée à une
            // vraie fenêtre de jeu, elle n'a plus besoin d'exception : elle suit le premier plan
            // de SON personnage, et un second client Wakfu ne la voit pas.
            //
            // **Sauf ouverte sans aucun client à l'écran** (`is_detached`, 2026-09-17) : il n'y a
            // alors aucun premier plan à suivre, et une fenêtre `WS_EX_TOOLWINDOW` rétrogradée
            // — absente de la barre des tâches et de l'alt-tab — serait perdue derrière la
            // première application venue, sans moyen de la retrouver ni d'en rouvrir une autre
            // (une seule modale à la fois). Elle reste donc toujours « pertinente » : réaffirmée
            // devant périodiquement comme un overlay dont le jeu a le focus, jamais démotée.
            let relevant =
                overlay.is_detached() || relevant_game_hwnds.contains(&overlay.game_hwnd);

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
                    overlay.next_redraw_at = Some(std::time::Instant::now());
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

        // **Les fenêtres qui voilent le jeu restent devant tout** (2026-09-17) : chaque
        // `SetWindowPos(HWND_TOPMOST)` ci-dessus place SA fenêtre en tête de la bande topmost —
        // donc devant le voile, si un Combat ou un Suivi de la même fenêtre de jeu vient d'être
        // réaffirmé après elle (l'ordre d'itération d'une `HashMap` n'est pas le nôtre).
        // Réaffirmées EN DERNIER, à chaque passe, tant qu'elles sont topmost : elles reprennent le
        // dessus. La fenêtre Options rattachée à un client voile depuis le même jour (voir
        // `RenderContent::veiled`) ; la confirmation de remise à zéro passe après elle, au cas où
        // les deux coexisteraient — la question doit rester lisible par-dessus le réglage.
        let mut veils: Vec<&OverlayWindow> = self
            .windows
            .values()
            .filter(|w| {
                w.is_topmost
                    && (matches!(w.kind, OverlayKind::ResetConfirm(_))
                        || (w.kind == OverlayKind::Options && !w.is_detached()))
            })
            .collect();
        veils.sort_by_key(|w| matches!(w.kind, OverlayKind::ResetConfirm(_)));
        for overlay in veils {
            unsafe {
                let _ = SetWindowPos(
                    Self::hwnd_of(&overlay.window),
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }

    /// Ouvre la modale Options (2026-09-08, §9 du plan) — bouton "Options" du carré de contrôle
    /// (`anchor_rect` = `game_rect` de la fenêtre Suivi cliquée) ou raccourci global
    /// `ShortcutAction::Options` (`anchor_rect` = celui de la première fenêtre de jeu connue, s'il
    /// y en a une). Sans effet si une modale est déjà ouverte (une seule à la fois, comme un vrai
    /// dialogue modal) — pas de file d'attente, l'utilisateur referme/valide l'existante avant d'en
    /// rouvrir une. Même méthode que `bin/wakfu-companion-overlay-x11.rs` (dupliquée, voir la doc
    /// de `lib.rs` pour pourquoi le fenêtrage OS n'est PAS partagé entre les deux binaires). Ouvre
    /// la modale Options, **rattachée à une fenêtre de jeu**.
    ///
    /// `anchor` est la fenêtre depuis laquelle elle est demandée : le bouton « Options » d'un
    /// bandeau Suivi la donne directement. Le raccourci global, lui, n'en a pas — il prend alors
    /// la fenêtre de jeu **au premier plan**, et à défaut le premier overlay connu. Sans ce
    /// rattachement la modale n'appartiendrait à personne, ce qui était précisément le défaut :
    /// voir `sync_topmost`. **Aucune fenêtre de jeu du tout** (2026-09-17) : la modale s'ouvre
    /// quand même, détachée — centrée sur l'écran principal, jamais refermée par `sync_windows`
    /// (voir `OverlayWindow::is_detached`).
    ///
    /// **Rattachée, la fenêtre OS est celle du jeu, pas celle de la modale** (2026-09-17, décision
    /// utilisateur : « un voile qui recouvre toute la fenêtre du jeu et les overlays lorsque
    /// l'utilisateur ouvre la modale d'options alors que la fenêtre de jeu est ouverte ») : elle
    /// couvre le client entier, voile tout — Combat, Suivi, Récap compris, puisqu'elle naît après
    /// eux et que `sync_topmost` la garde devant — et centre la modale dedans
    /// (`RenderContent::veiled`, `design::scrim`). Détachée, « aucun voile ne doit être
    /// appliqué » : elle garde la taille de la modale.
    ///
    /// `initial_tab` est l'onglet sur lequel la fenêtre s'ouvre — **`Paramètres` pour le bouton
    /// « Options » du bandeau de suivi** (demande utilisateur 2026-09-13 : ce bouton donne accès
    /// au chemin de `wakfu.log`, pas à la composition de la liste suivie, qui a son propre accès
    /// dans ce même bandeau), le défaut d'[`options_modal::OptionsTab`] pour le raccourci global,
    /// qui n'a pas de bandeau particulier à privilégier.
    fn open_options_modal(
        &mut self,
        event_loop: &ActiveEventLoop,
        anchor: Option<(HWND, GameRect)>,
        initial_tab: options_modal::OptionsTab,
    ) {
        if self
            .windows
            .values()
            .any(|w| w.kind == OverlayKind::Options)
        {
            return;
        }
        // Compte non lié : la fenêtre de connexion est la seule interface (2026-09-14). Le
        // raccourci global reste enregistré à ce moment-là, d'où cette garde — l'entrée de
        // l'icône de zone de notification, elle, est déjà grisée (voir `sync_tray_menu`).
        if !self.session_ready() {
            tracing::info!(
                "[options] aucun compte lié — la fenêtre Options n'est accessible qu'une fois connecté."
            );
            return;
        }
        // Sans ancre explicite (raccourci global, icône de zone de notification), la fenêtre de
        // jeu AU PREMIER PLAN est la bonne réponse : c'est celle que l'utilisateur regarde au
        // moment où il appuie. Repli sur le premier overlay connu si le premier plan n'est pas un
        // client Wakfu.
        let anchor = anchor.or_else(|| {
            let foreground = unsafe { GetForegroundWindow() };
            self.windows
                .values()
                .filter(|w| !w.is_detached())
                .find(|w| w.game_hwnd == foreground)
                .or_else(|| self.windows.values().find(|w| !w.is_detached()))
                .map(|w| (w.game_hwnd, w.game_rect))
        });
        // **Aucune fenêtre de jeu à l'écran ⇒ la modale s'ouvre quand même, seule** (2026-09-17).
        // Jusque-là elle naissait rattachée à une HWND nulle et un rectangle inventé — et
        // `sync_windows`, ne trouvant aucun client derrière cette HWND, la refermait au tick
        // suivant : « Options » depuis la zone de notification ne faisait rien tant que le jeu
        // n'était pas lancé, alors que c'est précisément là qu'on règle le chemin de `wakfu.log`
        // ou les raccourcis avant de jouer. Une modale DÉTACHÉE (`OverlayWindow::is_detached`)
        // est centrée sur l'écran principal comme la fenêtre de connexion, reste devant (voir
        // `sync_topmost`) et n'est fermée que par l'utilisateur. Elle ne se rattache pas à un
        // client lancé entre-temps : elle vit jusqu'à « Valider »/« Annuler », comme avant.
        let detached = anchor.is_none();
        let (game_hwnd, rect) = anchor.unwrap_or((
            HWND::default(),
            GameRect {
                left: 0,
                top: 0,
                width: 0,
                height: 0,
                client_top: 0,
            },
        ));
        // **Rattachée à `game_hwnd` comme n'importe quel overlay** depuis le 2026-09-12 : elle
        // suit donc le premier plan de ce personnage, et disparaît quand on passe sur un autre
        // client en multi-compte. Elle portait `HWND::default()` (nul) jusque-là, ce qui obligeait
        // `sync_windows`/`sync_topmost` à l'exclure de toute leur logique — et la laissait
        // au-dessus de TOUT, y compris d'un navigateur ou d'un autre jeu (retour utilisateur).
        //
        // Toujours interactive (`true` littéral, PAS `self.interactive`) : une modale doit capter
        // le clavier et la souris, contrairement à un overlay passif d'information.
        let mut overlay = Self::create_overlay_window(
            event_loop,
            OverlayKind::Options,
            game_hwnd,
            "Options".to_string(),
            rect,
            true,
            // Une fenêtre de réglages qu'on vient d'ouvrir est visible, toujours : seul `Combat`
            // peut naître masqué (voir `sync_panel_visibility`).
            true,
            self.combat_on_right,
            // Sans objet : cette fenêtre-ci n'est ni le panneau Combat ni la bande Récap.
            None,
            None,
        );
        if detached {
            // Pas de jeu sur lequel s'ancrer : au centre de l'écran principal, et le focus tout de
            // suite — il n'y a personne à qui le voler. `last_position` est oublié : cette fenêtre
            // n'est jamais recollée par `reposition` (voir `sync_windows`), la valeur d'ancrage
            // calculée à la création ne correspond à rien.
            Self::center_on_primary_monitor(event_loop, &overlay.window);
            overlay.last_position = None;
            tracing::info!(
                "[options] aucune fenêtre de jeu à l'écran — fenêtre Options ouverte seule, centrée sur l'écran principal."
            );
        }
        // **Le focus clavier, rattachée ou non** (2026-09-17). Il n'était demandé que pour la
        // modale détachée : ouverte depuis un bandeau, la fenêtre naissait devant mais SANS le
        // focus, qui restait au bandeau qu'on venait de cliquer (en mode interactif, ce clic fait
        // de lui la fenêtre au premier plan malgré `WS_EX_NOACTIVATE` — voir `sync_topmost`). Ses
        // deux touches ne l'atteignaient donc pas : Échap ne l'annulait pas, Entrée ne validait
        // pas, et le filet « Échap quitte l'overlay » du bandeau, lui, fermait tout (retour
        // utilisateur, voir `window_event`).
        //
        // Le prendre au jeu est ici l'effet recherché, et non un vol : cette fenêtre est la seule
        // délibérément focalisable (§9.1 du plan, il faut pouvoir taper dans le champ de chemin),
        // et depuis le voile du même jour elle couvre le client entier — on ne joue pas derrière.
        overlay.window.focus_window();
        // **Le brouillon d'alertes est une COPIE du profil du compte**, prise à l'ouverture : les
        // gestes de l'onglet la modifient librement, et seul « Valider » la renvoie (§5.1 du plan).
        // Sans compte lié, il n'y a ni liste à charger ni endroit où l'écrire — l'onglet le dit.
        let alerts_snapshot = self.alert_profile.load();
        let (alerts_draft, alerts_availability) = match alerts_snapshot.as_ref() {
            Some(profile) => (Some(profile.clone()), alerts_tab::AlertsAvailability::Ready),
            None if matches!(**self.auth_status.load(), AuthStatus::Connected) => {
                // Compte lié mais réglages pas encore revenus : c'est le seul cas où un rouage dit
                // la vérité.
                (None, alerts_tab::AlertsAvailability::Loading)
            }
            None => (None, alerts_tab::AlertsAvailability::NoAccount),
        };
        // **Le brouillon de chat**, même principe : une copie des recherches du compte, plus les
        // réglages locaux de la carte.
        let chat_snapshot = self.chat_filters.load();
        let (chat_draft, chat_availability) = match chat_snapshot.as_ref() {
            Some(filters) => (
                Some(chat_tab::ChatDraft {
                    filters: filters.clone(),
                    toast: self.chat_toast,
                }),
                chat_tab::ChatAvailability::Ready,
            ),
            None if matches!(**self.auth_status.load(), AuthStatus::Connected) => {
                (None, chat_tab::ChatAvailability::Loading)
            }
            None => (None, chat_tab::ChatAvailability::NoAccount),
        };
        // **Le brouillon de suivi**, même principe que celui des alertes : une copie des entrées
        // du compte, prise ici, modifiée librement, et renvoyée seulement à « Valider ». Les
        // compteurs qu'elle porte sont indicatifs — la validation les recalcule depuis l'état
        // vivant du moteur (voir `WatchlistState::apply_definitions`).
        let suivi_snapshot = self.watchlist.load();
        let (suivi_draft, suivi_availability) = if suivi_snapshot.is_empty()
            && !matches!(**self.auth_status.load(), AuthStatus::Connected)
        {
            // Compte pas encore lié et rien en mémoire : les réglages sont en route. L'overlay ne
            // s'adresse qu'à des utilisateurs connectés (décision du 2026-09-13), il n'y a donc pas
            // d'état « sans compte » à peindre — seulement une attente.
            (None, suivi_tab::SuiviAvailability::Loading)
        } else {
            (
                Some(suivi_snapshot.as_ref().clone()),
                suivi_tab::SuiviAvailability::Ready,
            )
        };

        // **Le compte est relu à l'OUVERTURE de la fenêtre**, pas seulement au démarrage
        // (2026-09-13). Sans ça, une liste modifiée depuis le site n'arrivait qu'au prochain
        // lancement — et le brouillon partait d'un état périmé qu'il écrasait à la validation, la
        // clé `watchlist` étant réécrite en entier. Ouvrir l'écran d'édition est le bon moment pour
        // repartir de l'état réel ; la réponse arrive par `ApplySettings` comme toute autre, et la
        // fenêtre la reprendra à sa prochaine ouverture. Sur un thread, jamais sur la boucle winit
        // (§7.3 du plan).
        let settings_tx = self.settings_tx.clone();
        thread::spawn(move || {
            let Some(token) = overlay_sync::token_store::load_token() else {
                return;
            };
            match overlay_sync::client::fetch_settings(&token) {
                Ok(settings) => {
                    tracing::info!(
                        entry_count = settings.watchlist.len(),
                        "[options] réglages relus à l'ouverture de la fenêtre"
                    );
                    let _ = settings_tx.send(EngineCommand::ApplySettings(settings));
                }
                // Best-effort : la fenêtre s'ouvre de toute façon, sur ce que l'overlay a déjà.
                Err(err) => tracing::warn!(%err, "[options] relecture des réglages impossible"),
            }
        });

        // **Le brouillon du roster**, même principe que les alertes et le chat : une copie de ce
        // que le compte porte, prise ici, modifiée librement, renvoyée à « Valider ».
        let roster_snapshot = self.roster_draft.load();
        let (personnages_draft, personnages_availability) = match roster_snapshot.as_ref() {
            Some(roster) => (Some(roster.clone()), PersonnagesAvailability::Ready),
            None => (None, PersonnagesAvailability::Loading),
        };
        // Lu une seule fois par ouverture, jamais à chaque frame : c'est un accès au registre
        // (Windows) ou au disque (Linux), et la case est un brouillon comme les autres — elle ne
        // doit surtout pas se remettre d'aplomb toute seule pendant qu'on la regarde.
        let autostart_actif = overlay_ui::autostart::is_enabled();

        overlay.options_state = Some(OptionsModalState {
            path_input: self.log_path.display().to_string(),
            error: None,
            // Voir la doc de `open_options_modal` : le bouton « Options » du bandeau de suivi
            // demande `Parametres`, le raccourci global le défaut d'`OptionsTab`.
            tab: initial_tab,
            // La case part du réglage EN VIGUEUR, pas du défaut : la fenêtre montre l'état réel,
            // et « Annuler » n'a rien à défaire tant qu'on n'y touche pas (voir `is_dirty`).
            combat_always_visible: self.combat_always_visible,
            combat_on_right: self.combat_on_right,
            turn_notification: self.turn_notification,
            turn_notification_muted: self.turn_notification_muted,
            // Idem pour les trois interrupteurs : les cases s'ouvrent sur l'état réel.
            features: self.features,
            // Idem pour les deux sourdines.
            mutes: self.alert_mutes,
            // Idem pour la fermeture de la carte de décompte (2026-09-16) — réglage local, donc
            // rien à attendre d'un compte : la ligne s'ouvre directement sur sa valeur.
            countdown_toast: self.countdown_toast,
            completion: self.completion,
            // Idem pour la reprise de la session du Récap (2026-09-17).
            recap_resume: self.recap_session.resume_settings(),
            // Même règle pour les raccourcis : le brouillon part des combinaisons ACTIVES.
            shortcuts: self.hotkeys.bindings().clone(),
            raccourcis: Default::default(),
            // Le bouton « Déconnecter » de la section « Compte » n'a de sens que sur un compte
            // lié — l'hôte est seul à connaître `AuthStatus` (voir `spawn_auth_thread`).
            account_connected: matches!(**self.auth_status.load(), AuthStatus::Connected),
            // L'avis « session conservée en clair » de la section « Compte » (constat C7).
            token_on_disk: overlay_sync::token_store::token_file_in_use(),
            pending_disconnect: false,
            // « Supprimer les données locales » : jamais en cours à l'ouverture, comme les cinq
            // autres confirmations de cette fenêtre.
            pending_purge: false,
            personnages: PersonnagesTabState {
                // Le compte affiché à l'ouverture est le principal, celui que tout roster a.
                account: personnages_draft
                    .as_ref()
                    .map(|roster| roster.default_index())
                    .unwrap_or(0),
                ..Default::default()
            },
            // La section « Mise à jour » lit l'état publié par le thread de mise à jour ; rafraîchi
            // avant chaque rendu (voir `redraw`), posé ici pour la première frame.
            update: (**self.update_status.load()).clone(),
            auto_update: self.auto_update,
            verbose_log: self.verbose_log,
            // **Le démarrage avec l'ordinateur se lit dans le SYSTÈME**, pas dans un champ de
            // l'hôte : c'est le seul réglage de cette fenêtre qu'un autre programme peut avoir
            // changé entre deux ouvertures (Gestionnaire des tâches, réglages du bureau). Voir
            // `autostart`, doc de module.
            start_with_os: autostart_actif,
            pending_install: None,
            pending_quit: false,
            pending_restart: false,
            alerts: alerts_tab::AlertsTabState {
                // Le champ de durée s'ouvre sur la valeur en place, pas vide : c'est un réglage
                // existant qu'on vient modifier.
                duration_input: alerts_draft
                    .as_ref()
                    .map(|p| format_alert_duration(p.duration_seconds))
                    .unwrap_or_default(),
                ..Default::default()
            },
            chat: chat_tab::ChatTabState {
                duration_input: format_alert_duration(self.chat_toast.duration_seconds),
                ..Default::default()
            },
            initial: options_modal::OptionsInitial {
                path: self.log_path.display().to_string(),
                alerts: alerts_draft.clone(),
                suivi: suivi_draft.clone(),
                chat: chat_draft.clone(),
                personnages: personnages_draft.clone(),
                combat_always_visible: self.combat_always_visible,
                combat_on_right: self.combat_on_right,
                turn_notification: self.turn_notification,
                turn_notification_muted: self.turn_notification_muted,
                features: self.features,
                mutes: self.alert_mutes,
                countdown_toast: self.countdown_toast,
                completion: self.completion,
                recap_resume: self.recap_session.resume_settings(),
                shortcuts: self.hotkeys.bindings().clone(),
                auto_update: self.auto_update,
                verbose_log: self.verbose_log,
                start_with_os: autostart_actif,
            },
            pending_close: false,
            alerts_draft,
            alerts_availability,
            chat_draft,
            chat_availability,
            personnages_draft,
            personnages_availability,
            suivi: suivi_tab::SuiviTabState {
                // Comme pour les Alertes et le Chat : le champ de durée s'ouvre sur la valeur en
                // place, pas vide — c'est un réglage existant qu'on vient modifier.
                duration_input: format_alert_duration(self.countdown_toast.duration_seconds),
                ..Default::default()
            },
            suivi_draft,
            suivi_availability,
        });
        overlay.next_redraw_at = Some(std::time::Instant::now());
        self.windows.insert(overlay.window.id(), overlay);
        // **Les raccourcis globaux sont rendus au système tant que cette fenêtre est ouverte** :
        // sans ça, toute combinaison déjà prise par l'overlay (à commencer par celle qui vient
        // d'ouvrir cette fenêtre) serait interceptée par l'OS et n'atteindrait jamais la case en
        // écoute de l'onglet « Raccourcis » — voir `shortcuts::ShortcutRegistry::suspend`. Rétabli
        // par `close_options_modal`, quelle que soit la façon dont la fenêtre se referme.
        self.hotkeys.suspend();
        tracing::info!("[options] modale ouverte — raccourcis globaux suspendus.");
    }

    /// **Les suivis qui viennent d'aboutir** — reçus du thread Engine, célébrés, puis retirés
    /// (2026-09-17).
    ///
    /// Appelée à chaque tick de la boucle d'événements, **pas au rendu**, et c'est tout l'intérêt :
    /// décision utilisateur, « le retrait est une conséquence du seuil, pas de l'animation ». Une
    /// fenêtre Suivi masquée, un Suivi coupé, un joueur qui regardait ailleurs : l'entrée aboutie
    /// part quand même, et part au compte.
    ///
    /// Le retrait emprunte le chemin de la suppression groupée du bandeau, sans rien y ajouter :
    /// `SetWatchlistDefinitions` avec la liste amputée → `WatchlistState::apply_definitions` →
    /// `drain_watchlist_sync` → `SyncCommand::SyncWatchlist` → `PATCH /api/v1/settings`. Le moteur
    /// garde les compteurs des entrées restantes, et un échec réseau est retenté par le thread
    /// Sync — le retrait ne se perd pas dans une coupure.
    fn tick_watchlist_completions(&mut self) {
        let now = std::time::Instant::now();
        while let Ok(completed) = self.completions_rx.try_recv() {
            tracing::info!(
                name = %completed.name,
                remove = self.completion.remove,
                animate = self.completion.animates(),
                "[suivi] entrée complétée"
            );
            self.watchlist_completions.push(
                completed.key,
                now,
                self.completion.removal_delay_seconds(),
                self.completion.remove,
            );
        }

        // **Redessiner le bandeau tant qu'une tuile célèbre.** La boucle est réactive (§6.1) :
        // sans cette demande, l'animation n'avancerait qu'au tick de 50 ms et au gré des lots du
        // moteur — soit des à-coups visibles sur une couronne qui tourne. Le rythme est rendu à la
        // bande dès la dernière image : trois secondes et demie de 60 Hz, pas une de plus.
        if self.watchlist_completions.is_animating(now) {
            let prochaine = now + COMPLETION_FRAME;
            for overlay in self.windows.values_mut() {
                if matches!(overlay.kind, OverlayKind::Watchlist) {
                    overlay.next_redraw_at = Some(match overlay.next_redraw_at {
                        Some(deja) => deja.min(prochaine),
                        None => prochaine,
                    });
                }
            }
        }

        let a_retirer = self.watchlist_completions.drain_due(now);
        if a_retirer.is_empty() {
            return;
        }
        // **La liste de référence est celle du moteur**, relue à l'instant du retrait et non à
        // celui du franchissement : entre les deux, le joueur a pu ajouter une entrée depuis la
        // fenêtre Options ou le site. Repartir d'une copie prise 3,5 s plus tôt la ferait
        // disparaître.
        let definitions: Vec<WatchlistEntry> = self
            .watchlist
            .load()
            .iter()
            .filter(|entry| {
                !a_retirer.contains(&panels::suivi_tab::key_of(&entry.name, entry.catalog_id))
            })
            .cloned()
            .collect();
        // Rien à écrire si aucune des clés ne correspond plus à une entrée vivante : elles ont pu
        // être retirées entre-temps depuis la fenêtre Options ou le site. Réécrire la clé pour
        // rien repousserait son horodatage, et le « dernier écrivain gagne » du serveur ferait
        // perdre une modification faite ailleurs (même précaution que `commit_suivi`).
        if definitions.len() == self.watchlist.load().len() {
            return;
        }
        tracing::info!(
            retirees = a_retirer.len(),
            restantes = definitions.len(),
            "[suivi] entrées complétées retirées, réplication au compte en route"
        );
        let _ = self
            .settings_tx
            .send(EngineCommand::SetWatchlistDefinitions {
                definitions,
                // Rien à oublier en plus : l'entrée complétée quitte la liste sans y revenir,
                // exactement comme un retrait depuis le bandeau. Le jour où elle est recréée,
                // c'est une entrée neuve, qui repart de la valeur de son mode.
                retirees: Vec::new(),
            });
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
                // `App::validate_and_commit_options`/`discovery::validate_log_path`, jamais sauté
                // même si l'utilisateur choisit un `.log` mal nommé dans le dialogue.
                let picked = rfd::FileDialog::new()
                    .set_title("Sélectionner le fichier wakfu.log")
                    .add_filter("wakfu.log", &["log"])
                    .pick_file();
                let _ = tx.send(picked);
            })
            .expect("échec de création du thread de dialogue de fichier");
    }

    /// Commit du brouillon de l'onglet « Chat » — calqué sur `commit_alerts` pour les recherches
    /// (clé `chatFilters` du compte, remplacée en entier) ; les réglages de la carte, locaux, sont
    /// appliqués au moteur et rendus à l'appelant pour la config (`true` = ils ont changé).
    fn commit_chat(&mut self, options_window_id: WindowId) -> bool {
        let Some(draft) = self
            .windows
            .get(&options_window_id)
            .and_then(|overlay| overlay.options_state.as_ref())
            .and_then(|state| state.chat_draft.clone())
        else {
            return false;
        };
        let toast_changed = draft.toast != self.chat_toast;
        if toast_changed {
            self.chat_toast = draft.toast;
            let _ = self
                .settings_tx
                .send(EngineCommand::SetChatToast(draft.toast));
        }
        let reference = self.chat_filters.load();
        if reference.as_ref().as_ref() == Some(&draft.filters) {
            return toast_changed;
        }
        // Appliqué localement d'abord : le prochain message doit obéir sans attendre le réseau.
        let _ = self
            .settings_tx
            .send(EngineCommand::SetChatFilters(draft.filters.clone()));
        let filters = draft.filters;
        thread::spawn(move || {
            match overlay_sync::token_store::load_token() {
            Some(token) => match overlay_sync::client::patch_chat_filters(&token, &filters) {
                Ok(_) => tracing::info!("[options] recherches de chat enregistrées sur le compte."),
                Err(err) => {
                    tracing::warn!(%err, "[options] échec de l'enregistrement des recherches de chat")
                }
            },
            None => tracing::info!(
                "[options] recherches de chat appliquées localement — aucun compte lié, rien n'est enregistré."
            ),
        }
        });
        toast_changed
    }

    /// Commit du brouillon de l'onglet « Personnages » — calqué sur [`Self::commit_chat`] : le
    /// roster est appliqué localement d'abord (le combat en cours doit reconnaître le personnage
    /// déclaré sans attendre le réseau), puis écrit sur le compte depuis un thread.
    ///
    /// **Seul l'écart part au compte** (`Roster::patch_against`, constat C9, 2026-09-19) : les
    /// comptes modifiés ou créés, et les identifiants des comptes retirés — la référence est le
    /// roster publié par le thread Engine, c'est-à-dire celui du compte ou celui de la dernière
    /// validation. Voir `overlay_engine::roster`, doc de module, et
    /// `overlay_sync::client::patch_roster`.
    fn commit_personnages(&mut self, options_window_id: WindowId) {
        let Some(roster) = self
            .windows
            .get(&options_window_id)
            .and_then(|overlay| overlay.options_state.as_ref())
            .and_then(|state| state.personnages_draft.clone())
        else {
            return;
        };
        let reference = self.roster_draft.load();
        let patch = roster.patch_against(reference.as_ref().as_ref().unwrap_or(&Roster::default()));
        if patch.is_empty() {
            return;
        }
        let _ = self.settings_tx.send(EngineCommand::SetRoster(roster));
        thread::spawn(move || match overlay_sync::token_store::load_token() {
            Some(token) => match overlay_sync::client::patch_roster(&token, &patch) {
                Ok(_) => tracing::info!("[options] roster enregistré sur le compte."),
                Err(err) => {
                    tracing::warn!(%err, "[options] échec de l'enregistrement du roster")
                }
            },
            None => tracing::info!(
                "[options] roster appliqué localement — aucun compte lié, rien n'est enregistré."
            ),
        });
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
        if connu.as_ref().as_ref() == Some(&draft) {
            return;
        }

        // Appliqué localement d'abord : la prochaine alerte doit obéir sans attendre le réseau.
        let _ = self
            .settings_tx
            .send(EngineCommand::SetAlertProfile(draft.clone()));

        // Et écrit au compte sur un thread — jamais sur la boucle winit (§7.3 du plan, même règle
        // que tous les appels réseau de ce dépôt). Les trois champs d'alerte seulement : le
        // serveur fusionne (`AlertProfile::patch_fields`, constat C9, 2026-09-19).
        let fields = draft.patch_fields();
        thread::spawn(move || match overlay_sync::token_store::load_token() {
            Some(token) => match overlay_sync::client::patch_profile(&token, &fields) {
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

    /// **« Valider » de l'onglet « Suivi »** — applique les définitions du brouillon et les réplique
    /// au compte.
    ///
    /// Deux précautions, toutes deux issues de la revue de la maquette (2026-09-13) :
    ///
    /// 1. **Seules les DÉFINITIONS partent** (nom, genre, mode, cible). Les compteurs du brouillon
    ///    sont ignorés par le moteur : il a les siens, et un objet ramassé pendant que la fenêtre
    ///    était ouverte ne doit pas être annulé par la validation (voir
    ///    `WatchlistState::apply_definitions`).
    /// 2. **La réplication passe par le chemin habituel des compteurs** — le moteur se marque
    ///    `dirty`, son prochain `drain_watchlist_sync` part vers `SyncCommand::SyncWatchlist`. Rien
    ///    de spécial à écrire ici : la liste qui monte au compte est celle du moteur, compteurs
    ///    vivants compris, jamais celle du brouillon.
    fn commit_suivi(&mut self, options_window_id: WindowId) {
        let Some(state) = self
            .windows
            .get(&options_window_id)
            .and_then(|overlay| overlay.options_state.as_ref())
        else {
            return;
        };
        let Some(draft) = state.suivi_draft.clone() else {
            return;
        };
        // Les entrées retirées pendant l'édition partent AVEC le brouillon : une entrée supprimée
        // puis recréée y figure des deux côtés, et c'est la seule chose qui la distingue d'une
        // entrée jamais touchée (voir `SuiviTabState::retirees`).
        let retirees = state.suivi.retirees.clone();
        // Rien n'a bougé : ne pas réécrire une clé pour rien, et surtout ne pas repousser son
        // horodatage — le « dernier écrivain gagne » du serveur ferait alors perdre une
        // modification faite depuis le site entre-temps. Une entrée supprimée puis recréée à
        // l'identique laisse bien la liste inchangée, mais son compteur, lui, doit repartir : elle
        // n'est pas « rien n'a bougé ».
        if state.initial.suivi.as_ref() == Some(&draft) && retirees.is_empty() {
            return;
        }
        tracing::info!(
            entry_count = draft.len(),
            removed_count = retirees.len(),
            "[options] liste de suivi validée"
        );
        let _ = self
            .settings_tx
            .send(EngineCommand::SetWatchlistDefinitions {
                definitions: draft,
                retirees,
            });
    }

    /// Résout les ingrédients d'une recette pour la fenêtre de l'onglet « Suivi » — sur un thread,
    /// jamais sur la boucle winit (§7.3 du plan).
    ///
    /// Un aller-retour par NIVEAU de recette (`GET /api/v1/items/{id}`) : c'est pourquoi la fenêtre
    /// s'ouvre avant la réponse et montre un rouage en attendant.
    fn start_recipe_resolution(&mut self, options_window_id: WindowId, item_id: i64) {
        let (tx, rx) = mpsc::channel();
        self.pending_recipe = Some((options_window_id, rx));
        let catalog = Arc::clone(&self.catalog);
        thread::Builder::new()
            .name("overlay-ui-recipe".into())
            .spawn(move || {
                let index = catalog.load();
                let mut fetch = |id: i64| overlay_sync::client::fetch_item_detail(id).ok();
                let ingredients = overlay_engine::resolve_recipe(item_id, &index, &mut fetch);
                tracing::info!(
                    item_id,
                    ingredient_count = ingredients.len(),
                    "[options] recette résolue"
                );
                let _ = tx.send(ingredients);
            })
            .ok();
    }

    /// Ouvre une confirmation du Récap (2026-09-17, `OverlayKind::ResetConfirm`) par-dessus la
    /// fenêtre de jeu d'où le glyphe a été cliqué — une seule à la fois, quelle que soit sa cible
    /// (les compteurs ou la position, voir `ResetTarget`), comme la fenêtre Options. Toujours
    /// interactive et visible : c'est une question qu'on vient de poser.
    fn open_reset_confirm(
        &mut self,
        event_loop: &ActiveEventLoop,
        game_hwnd: HWND,
        rect: GameRect,
        target: ResetTarget,
    ) {
        if self
            .windows
            .values()
            .any(|w| matches!(w.kind, OverlayKind::ResetConfirm(_)))
        {
            return;
        }
        let mut overlay = Self::create_overlay_window(
            event_loop,
            OverlayKind::ResetConfirm(target),
            game_hwnd,
            "Recap".to_string(),
            rect,
            true,
            true,
            self.combat_on_right,
            // La confirmation couvre la fenêtre de jeu entière — elle ne suit ni le panneau
            // Combat ni la bande Récap.
            None,
            None,
        );
        overlay.next_redraw_at = Some(std::time::Instant::now());
        self.windows.insert(overlay.window.id(), overlay);
        match target {
            ResetTarget::RecapSession => {
                tracing::info!("[session] confirmation de remise à zéro ouverte.")
            }
            ResetTarget::RecapPosition => {
                tracing::info!("[recap] confirmation de replacement ouverte.")
            }
            ResetTarget::CombatPosition => {
                tracing::info!("[combat] confirmation de replacement ouverte.")
            }
            ResetTarget::WatchlistCounter => {
                tracing::info!(
                    name = self
                        .watchlist_reset_pending
                        .as_ref()
                        .map(|e| e.name.as_str()),
                    "[suivi] confirmation de réinitialisation du compteur ouverte."
                )
            }
        }
    }

    /// **Le bouton de réinitialisation d'une tuile du bandeau** (2026-09-18) : retenir l'entrée,
    /// puis ouvrir la même confirmation que le Récap — voir `ResetTarget::WatchlistCounter` sur
    /// pourquoi l'entrée est retenue ici plutôt que portée par la cible.
    fn open_watchlist_reset_confirm(
        &mut self,
        event_loop: &ActiveEventLoop,
        game_hwnd: HWND,
        rect: GameRect,
        entry: WatchlistEntry,
    ) {
        self.watchlist_reset_pending = Some(entry);
        self.open_reset_confirm(event_loop, game_hwnd, rect, ResetTarget::WatchlistCounter);
    }

    /// **Le cadenas de la bande Récap** (2026-09-17) : un seul bouton, deux états, pas de
    /// confirmation — verrouiller ne perd rien et le glyphe montre aussitôt ce qu'on a obtenu.
    /// Le réglage est écrit tout de suite : il doit survivre à un arrêt brutal comme la position
    /// qu'il protège.
    fn toggle_recap_lock(&mut self) {
        self.recap_locked = !self.recap_locked;
        self.persist_config();
        tracing::info!(
            "[recap] bande {}.",
            if self.recap_locked {
                "verrouillée"
            } else {
                "déverrouillée"
            }
        );
    }

    /// **Le cadenas du panneau Combat** (2026-09-17) : même geste que celui de la bande Récap, et
    /// même politique — un seul bouton, deux états, pas de confirmation, écrit tout de suite.
    fn toggle_combat_lock(&mut self) {
        self.combat_locked = !self.combat_locked;
        self.persist_config();
        tracing::info!(
            "[combat] panneau {}.",
            if self.combat_locked {
                "verrouillé"
            } else {
                "déverrouillé"
            }
        );
    }

    /// La réponse à une confirmation du Récap : « Oui » agit sur la cible, les deux réponses
    /// ferment la fenêtre. Le bloc Récap se redessine à son prochain tick — il se redessine
    /// toutes les secondes de toute façon.
    ///
    /// **Le replacement** : `recap_position` à `None`, la config réécrite, et les fenêtres
    /// replacées — « jamais déplacée » veut dire « suit l'ancrage », voir `recap_placement::snap`.
    /// C'est le seul chemin depuis le 2026-09-18 : la fenêtre Options n'a plus de bouton
    /// « Replacer au défaut », la bande porte le sien (`panels::recap`).
    fn answer_reset_confirm(
        &mut self,
        confirm_window_id: WindowId,
        target: ResetTarget,
        confirmed: bool,
    ) {
        self.windows.remove(&confirm_window_id);
        match (target, confirmed) {
            (ResetTarget::RecapSession, true) => {
                let snapshot = self.snapshot.load();
                self.recap_session
                    .reset(&snapshot.totals, std::time::SystemTime::now());
            }
            (ResetTarget::RecapSession, false) => {
                tracing::info!("[session] remise à zéro annulée.")
            }
            (ResetTarget::RecapPosition, true) => {
                self.recap_position = None;
                self.persist_config();
                self.reposition_recap();
                tracing::info!("[recap] bande revenue à son emplacement d'origine.");
            }
            (ResetTarget::RecapPosition, false) => tracing::info!("[recap] replacement annulé."),
            (ResetTarget::CombatPosition, true) => {
                self.combat_position_y = None;
                self.persist_config();
                self.reposition_combat();
                tracing::info!("[combat] panneau revenu à sa hauteur d'origine.");
            }
            (ResetTarget::CombatPosition, false) => {
                tracing::info!("[combat] replacement annulé.")
            }
            // **Le compteur repart** : c'est le moteur qui le remet (il seul tient les compteurs
            // vivants), republie la liste et réplique au compte — même chemin qu'un ramassage.
            // L'entrée est désignée par son identité : la liste a pu bouger entre-temps.
            (ResetTarget::WatchlistCounter, true) => match self.watchlist_reset_pending.take() {
                Some(entry) => {
                    tracing::info!(name = %entry.name, "[suivi] réinitialisation du compteur confirmée.");
                    let _ = self.settings_tx.send(EngineCommand::ResetWatchlistCounter {
                        name: entry.name,
                        kind: entry.kind,
                    });
                }
                None => tracing::warn!(
                    "[suivi] réinitialisation confirmée sans entrée retenue, rien fait."
                ),
            },
            (ResetTarget::WatchlistCounter, false) => {
                self.watchlist_reset_pending = None;
                tracing::info!("[suivi] réinitialisation du compteur annulée.")
            }
        }
    }

    /// Ferme la fenêtre Options et **rend ses raccourcis globaux au système** (voir
    /// `open_options_modal` pour la suspension).
    ///
    /// Unique point de fermeture depuis le 2026-09-13 : aucun chemin (Annuler, croix, Échap,
    /// validation réussie) ne doit pouvoir laisser l'overlay sans raccourcis.
    fn close_options_modal(&mut self, options_window_id: WindowId, raison: &str) {
        self.windows.remove(&options_window_id);
        let refuses = self.hotkeys.resume();
        if !refuses.is_empty() {
            // Déjà journalisé action par action par la registry ; ce résumé dit surtout que la
            // combinaison qu'on vient de choisir n'a pas pu être prise par le système.
            tracing::warn!(
                "[options] {} raccourci(s) refusé(s) par le système — voir les lignes [raccourcis] ci-dessus.",
                refuses.len()
            );
        }
        tracing::info!("[options] fenêtre fermée ({raison}).");
    }

    /// Applique ce que « Valider » vient d'emporter de la fenêtre Options — voir
    /// [`options_modal::OptionsCommit`].
    ///
    /// **Le chemin de log commande** : tant qu'il est invalide, la fenêtre reste ouverte avec son
    /// message d'erreur et RIEN n'est pris en compte — ni les alertes, ni le suivi, ni l'affichage
    /// du panneau de combat. Un commit partiel laisserait l'utilisateur devant une fenêtre en
    /// erreur sans savoir ce qui a déjà été écrit.
    /// Recolle chaque bande Récap sur sa fenêtre de jeu — pour les fois où c'est la POSITION qui
    /// change alors que les fenêtres de jeu, elles, n'ont pas bougé (bouton « Replacer au
    /// défaut »). `sync_windows` s'en charge le reste du temps, mais au prochain tick seulement :
    /// le geste et son effet doivent être dans la même passe, comme pour les cases à cocher (voir
    /// `sync_panel_visibility`).
    fn reposition_recap(&mut self) {
        self.reposition_kind(OverlayKind::Recap);
    }

    /// Recolle chaque panneau Combat sur sa fenêtre de jeu — le pendant de
    /// [`Self::reposition_recap`] pour la hauteur du panneau (glyphe de replacement de sa rangée
    /// d'actions, bouton des Options).
    fn reposition_combat(&mut self) {
        self.reposition_kind(OverlayKind::Combat);
    }

    /// Recolle toutes les fenêtres d'une zone donnée, fenêtres de jeu inchangées — voir les deux
    /// appelants ci-dessus.
    fn reposition_kind(&mut self, kind: OverlayKind) {
        let recap_position = self.recap_position;
        let combat_on_right = self.combat_on_right;
        let combat_position_y = self.combat_position_y;
        for overlay in self.windows.values_mut() {
            if overlay.kind == kind {
                let rect = overlay.game_rect;
                Self::reposition(
                    overlay,
                    rect,
                    combat_on_right,
                    combat_position_y,
                    recap_position,
                );
            }
        }
    }

    /// **Écrit `config.toml` en entier, depuis l'état courant de l'application.**
    ///
    /// Le fichier est réécrit d'un bloc (voir `config::save`) : n'y porter que le réglage qu'on
    /// vient de changer effacerait tous les autres. Cette méthode est donc le SEUL endroit qui
    /// sache ce que la config doit contenir — jusqu'au 2026-09-17 la même construction vivait au
    /// milieu de `validate_and_commit_options`, et la bande Récap déplaçable lui ajoutait un
    /// deuxième appelant (la pose de la bande, qui n'ouvre aucune fenêtre Options). Deux copies
    /// de cette liste de champs auraient divergé au premier réglage ajouté, et le réglage oublié
    /// dans l'une serait effacé par l'autre.
    ///
    /// Best-effort, comme tout le module `config` : un échec d'écriture est journalisé là-bas,
    /// jamais fatal.
    fn persist_config(&self) {
        let mut saved = config::OverlayConfig {
            log_path: Some(self.log_path.clone()),
            combat_always_visible: self.combat_always_visible,
            combat_on_right: self.combat_on_right,
            turn_notification: self.turn_notification,
            turn_notification_muted: self.turn_notification_muted,
            auto_update: self.auto_update,
            verbose_log: self.verbose_log,
            ..Default::default()
        };
        saved.set_shortcuts(self.hotkeys.bindings());
        saved.set_chat_toast(self.chat_toast);
        saved.set_countdown_toast(self.countdown_toast);
        saved.set_completion(self.completion);
        saved.set_recap_resume(self.recap_session.resume_settings());
        saved.set_recap_position(self.recap_position);
        saved.recap_locked = self.recap_locked;
        saved.combat_position_y = self.combat_position_y;
        saved.combat_locked = self.combat_locked;
        saved.set_features(self.features);
        saved.set_alert_mutes(self.alert_mutes);
        config::save(&saved);
    }

    fn validate_and_commit_options(
        &mut self,
        options_window_id: WindowId,
        commit: options_modal::OptionsCommit,
    ) {
        let candidate = PathBuf::from(commit.path.trim());
        match discovery::validate_log_path(&candidate) {
            Ok(()) => {
                // **Le moteur n'est rechargé que si le chemin a CHANGÉ.** Jusqu'au 2026-09-12,
                // « Valider » renvoyait `ChangeLogPath` à chaque clic, chemin identique compris :
                // le thread moteur abandonnait son watcher et relisait `wakfu.log` depuis le
                // début, dans une session qui gardait son état — et chaque ligne rejouée
                // recréditait le combat en cours (retour utilisateur, vidéo à l'appui : une ligne
                // d'allié de plus, et le total qui grimpe, à chaque objet ajouté aux alertes).
                let path_changed = candidate != self.log_path;
                if path_changed {
                    tracing::info!(
                        "[options] nouveau fichier de log validé : {}",
                        candidate.display()
                    );
                    self.log_path = candidate.clone();
                    let _ = self
                        .settings_tx
                        .send(EngineCommand::ChangeLogPath(candidate.clone()));
                } else {
                    tracing::info!("[options] chemin de log inchangé, moteur non touché.");
                }
                let combat_changed = commit.combat_always_visible != self.combat_always_visible;
                if combat_changed {
                    self.combat_always_visible = commit.combat_always_visible;
                    tracing::info!(
                        "[options] panneau de combat en dehors des combats : {}",
                        if self.combat_always_visible {
                            "affiché"
                        } else {
                            "masqué"
                        }
                    );
                }
                // Le côté du panneau (2026-09-17) : rien d'autre à faire ici que de retenir la
                // valeur. Le balayage des fenêtres de jeu recolle chaque overlay à son ancrage à
                // chaque tick (`sync_windows` → `reposition`), et le rendu lit le nouveau côté à
                // la frame suivante — la fenêtre Combat traverse donc l'écran toute seule.
                let side_changed = commit.combat_on_right != self.combat_on_right;
                if side_changed {
                    self.combat_on_right = commit.combat_on_right;
                    tracing::info!(
                        "[options] panneau de combat : {}",
                        if self.combat_on_right {
                            "à droite"
                        } else {
                            "à gauche"
                        }
                    );
                }
                let turn_changed = commit.turn_notification != self.turn_notification;
                if turn_changed {
                    self.turn_notification = commit.turn_notification;
                    tracing::info!(
                        "[options] notification de tour : {}",
                        if self.turn_notification {
                            "activée"
                        } else {
                            "désactivée"
                        }
                    );
                }
                let turn_muted_changed =
                    commit.turn_notification_muted != self.turn_notification_muted;
                if turn_muted_changed {
                    self.turn_notification_muted = commit.turn_notification_muted;
                    tracing::info!(
                        "[options] son de la notification de tour : {}",
                        if self.turn_notification_muted {
                            "coupé"
                        } else {
                            "rétabli"
                        }
                    );
                }
                // **Les interrupteurs (2026-09-15)** — le thread Engine est prévenu dès qu'ils
                // bougent : c'est lui qui joue (ou ne joue plus) les alertes. Le bandeau, lui, lit
                // `self.features` directement au rendu (voir `render_window`), et les deux cases
                // de la section « Combat » ne concernent pas le moteur du tout : le détail des
                // combats passe par `sync_panel_visibility` (appelée en fin de cette méthode) et
                // le suivi des sorts par le rendu du panneau.
                let features_changed = commit.features != self.features;
                if features_changed {
                    self.features = commit.features;
                    tracing::info!(
                        suivi = self.features.suivi,
                        alertes = self.features.alerts,
                        recherche = self.features.chat,
                        combat = self.features.combat,
                        sorts = self.features.spells,
                        recap = self.features.recap,
                        recap_duree = self.features.recap_cells.duration,
                        recap_combats = self.features.recap_cells.fights,
                        recap_challenges = self.features.recap_cells.challenges,
                        "[options] fonctionnalités actives mises à jour"
                    );
                    let _ = self
                        .settings_tx
                        .send(EngineCommand::SetFeatures(self.features));
                }
                // **Les deux sourdines (2026-09-15)** — même chemin que les interrupteurs, et
                // même raison : c'est le thread Engine qui joue les sons, lui seul a besoin de
                // savoir lesquels se taisent. Rien à rafraîchir côté rendu, une sourdine ne se
                // voit pas.
                let mutes_changed = commit.mutes != self.alert_mutes;
                if mutes_changed {
                    self.alert_mutes = commit.mutes;
                    tracing::info!(
                        suivi = self.alert_mutes.suivi,
                        chat = self.alert_mutes.chat,
                        "[options] sons d'alerte coupés mis à jour"
                    );
                    let _ = self
                        .settings_tx
                        .send(EngineCommand::SetAlertMutes(self.alert_mutes));
                }
                // **La fermeture de la carte de décompte (2026-09-16)** — même chemin que les
                // sourdines ci-dessus : c'est le thread Engine qui pose `hide_at` au moment où
                // l'alerte naît, lui seul a besoin de connaître le délai.
                let countdown_toast_changed = commit.countdown_toast != self.countdown_toast;
                if countdown_toast_changed {
                    self.countdown_toast = commit.countdown_toast;
                    tracing::info!(
                        duration_seconds = self.countdown_toast.duration_seconds,
                        manual_close = self.countdown_toast.manual_close,
                        "[options] fermeture de la carte de décompte mise à jour"
                    );
                    let _ = self
                        .settings_tx
                        .send(EngineCommand::SetCountdownToast(self.countdown_toast));
                }
                // **Les deux réglages de complétion (2026-09-17)** — ils ne partent PAS au
                // thread Engine, contrairement à la fermeture ci-dessus : le moteur ne sait rien
                // de la célébration ni du retrait, c'est l'hôte qui les décide à réception de la
                // complétion (voir `about_to_wait`).
                if commit.completion != self.completion {
                    self.completion = commit.completion;
                    tracing::info!(
                        remove = self.completion.remove,
                        animate = self.completion.animates(),
                        "[options] complétion d'un suivi mise à jour"
                    );
                }
                // **La reprise de la session du Récap (2026-09-17)** — réglage local, posé sur
                // la session elle-même : il ne sert qu'au prochain retour d'une fenêtre de jeu.
                let recap_resume_changed =
                    commit.recap_resume != self.recap_session.resume_settings();
                if recap_resume_changed {
                    self.recap_session.set_resume_settings(commit.recap_resume);
                    tracing::info!(
                        enabled = commit.recap_resume.enabled,
                        minutes = commit.recap_resume.minutes,
                        "[options] reprise de la session du récap mise à jour"
                    );
                }
                // **Les raccourcis (2026-09-13)** — `apply` pendant la suspension ne touche pas
                // encore l'OS : c'est `close_options_modal`, juste après, qui enregistre
                // effectivement le nouveau jeu. Un refus de l'OS (combinaison déjà prise par une
                // autre application) n'empêche rien du reste : il est journalisé, et l'action
                // concernée reste sans raccourci pour la session.
                let shortcuts_changed = commit.shortcuts != *self.hotkeys.bindings();
                if shortcuts_changed {
                    tracing::info!("[options] raccourcis personnalisés mis à jour.");
                    self.hotkeys.apply(commit.shortcuts);
                }
                // **Le démarrage avec l'ordinateur (2026-09-16)** — le seul réglage de cette fenêtre
                // qui ne passe NI par la config NI par le thread Engine : il s'inscrit dans le système
                // (voir `autostart`, doc de module). `apply` compare à l'état réel avant d'écrire, et
                // n'échoue jamais bruyamment — un refus du système laisse simplement la case revenir
                // sur son état réel à la prochaine ouverture.
                overlay_ui::autostart::apply(commit.start_with_os);
                // **La mise à jour automatique (2026-09-15)** — persistée, effective au prochain lancement :
                // c'est là que la première commande au thread de mise à jour se décide.
                let auto_update_changed = commit.auto_update != self.auto_update;
                if auto_update_changed {
                    self.auto_update = commit.auto_update;
                    tracing::info!(
                        "[options] mise à jour automatique au démarrage : {}",
                        if self.auto_update {
                            "activée"
                        } else {
                            "désactivée"
                        }
                    );
                }
                // **Le journal détaillé (2026-09-18, constat C6)** — appliqué À CHAUD, sans
                // attendre un redémarrage : la case sert à diagnostiquer un problème en cours,
                // un réglage qui ne prendrait effet qu'au prochain lancement raterait justement
                // ce qu'on cherche à voir.
                let verbose_log_changed = commit.verbose_log != self.verbose_log;
                if verbose_log_changed {
                    self.verbose_log = commit.verbose_log;
                    logging::set_verbose(self.verbose_log);
                }
                // **La config est réécrite EN ENTIER**, et seulement si l'un des réglages a bougé :
                // le fichier est réécrit d'un bloc (voir `config::save`), n'y porter que le réglage
                // modifié effacerait les autres.
                // Les réglages de la carte de chat vivent dans la même config : commités AVANT
                // l'écriture, pour qu'elle les emporte (voir `commit_chat`).
                let chat_toast_changed = self.commit_chat(options_window_id);
                self.commit_personnages(options_window_id);
                if path_changed
                    || combat_changed
                    || side_changed
                    || turn_changed
                    || turn_muted_changed
                    || shortcuts_changed
                    || chat_toast_changed
                    || features_changed
                    || mutes_changed
                    || countdown_toast_changed
                    || recap_resume_changed
                    || auto_update_changed
                    || verbose_log_changed
                {
                    self.persist_config();
                }
                // **« Valider » commit TOUS les onglets, pas seulement celui qu'on regarde.** Le
                // pied de page est partagé : un bouton dont l'effet dépendrait de l'onglet affiché
                // serait imprévisible. Fait APRÈS la validation du chemin, et seulement si elle
                // passe — quand elle échoue, la fenêtre reste ouverte et rien n'est pris en compte,
                // alertes comprises.
                self.commit_alerts(options_window_id);
                self.commit_suivi(options_window_id);
                self.close_options_modal(options_window_id, "Valider");
                // Sans cet appel, cocher la case ne se verrait qu'au prochain tick
                // d'`about_to_wait` — 50 ms, imperceptible, mais le geste et son effet doivent
                // être dans la même passe : c'est ce qui rend la fenêtre Options vérifiable.
                self.sync_panel_visibility();
            }
            Err(err) => {
                tracing::info!("[options] chemin refusé : {}", err.message());
                if let Some(overlay) = self.windows.get_mut(&options_window_id) {
                    if let Some(state) = &mut overlay.options_state {
                        state.error = Some(err.message().to_string());
                    }
                    overlay.next_redraw_at = Some(std::time::Instant::now());
                }
            }
        }
    }
}

enum PostRedraw {
    None,
    /// Fenêtre de jeu **depuis laquelle** la modale est demandée — son `HWND` et son
    /// rectangle. La modale lui est rattachée comme n'importe quel overlay : c'est ce qui
    /// la fait suivre le premier plan de CE personnage, et disparaître quand on passe sur
    /// un autre client en multi-compte. Le troisième champ est l'onglet à ouvrir — "+"
    /// demande « Suivi », "Options" demande « Paramètres » (voir `render_content::
    /// RenderOutcome::open_watchlist`/`open_options`) : deux boutons, deux destinations,
    /// jamais le défaut implicite d'`options_modal::OptionsTab`.
    OpenOptions(HWND, GameRect, options_modal::OptionsTab),
    CloseOptions,
    /// Glyphe de remise à zéro du bloc Récap cliqué : ouvrir la confirmation par-dessus CETTE
    /// fenêtre de jeu (2026-09-17, voir `open_reset_confirm`).
    OpenResetConfirm(HWND, GameRect, ResetTarget),
    /// La confirmation a répondu — `true` pour « Oui » (voir `answer_reset_confirm`).
    AnswerResetConfirm(ResetTarget, bool),
    /// Bouton de réinitialisation d'une tuile du bandeau cliqué : retenir l'entrée et ouvrir la
    /// confirmation par-dessus CETTE fenêtre de jeu (2026-09-18, voir
    /// `open_watchlist_reset_confirm`).
    OpenWatchlistResetConfirm(HWND, GameRect, WatchlistEntry),
    /// Le cadenas de la bande Récap vient d'être cliqué (voir `toggle_recap_lock`).
    ToggleRecapLock,
    /// Le cadenas du panneau Combat vient d'être cliqué (voir `toggle_combat_lock`).
    ToggleCombatLock,
    /// Carte d'alerte de chat cliquée : préparer la réponse en privé à cet auteur (voir
    /// `whisper_from_toast`) — après le rendu, comme tout ce qui touche `self` entier.
    Whisper(String),
    BrowseOptions,
    /// Ce que « Valider » emporte de l'onglet « Paramètres » — voir
    /// `options_modal::OptionsCommit`.
    ValidateOptions(options_modal::OptionsCommit),
    /// Bouton « Déconnecter » de la section « Compte », **après confirmation** (voir
    /// `panels::options_modal`) — efface la session et referme la fenêtre.
    DisconnectAccount,
    /// Résoudre les ingrédients de cet objet pour la fenêtre de recette de l'onglet « Suivi ».
    ResolveRecipe(i64),
    /// « Recherche de mise à jour » (section « Mise à jour » de l'onglet « Paramètres ») :
    /// une vérification sans installation, demandée au thread de mise à jour.
    CheckUpdate,
    /// « Mettre à jour vers X », **après confirmation** — voir `request_update_install`.
    InstallUpdate,
    /// « Fermer l'overlay », **après confirmation** (pied de l'onglet « Paramètres »,
    /// 2026-09-16) : arrête le programme par le chemin de l'entrée « Quitter » de la zone de
    /// notification — le raccourci global « Quitter l'overlay » a été retiré le 2026-09-17.
    Quit,
    /// « Redémarrer », **après confirmation** (pied de l'onglet « Paramètres », à gauche de
    /// « Fermer l'overlay », 2026-09-17) : un process neuf est lancé (`restart::relaunch`) puis
    /// celui-ci sort, par le même chemin que [`Self::Quit`].
    Restart,
    /// « Réessayer » de l'écran « Mise à jour requise » de la fenêtre de connexion : nouvelle
    /// vérification, avec installation.
    RetryUpdate,
    /// « Mettre à jour maintenant » de l'écran de mise à jour manuelle : le téléchargement part,
    /// et son avancement s'affiche sur la même carte (voir `open_manual_update_window`).
    StartManualUpdate,
    /// « Fermer » / « Plus tard » de l'écran de mise à jour manuelle.
    CloseManualUpdate,
    /// **Ouvrir l'écran de mise à jour de la Carte** et y lancer une recherche (2026-09-22) — le
    /// lien « Mise à jour » du pied, et « Rechercher à nouveau » de cet écran même. Le même geste
    /// que l'entrée « Mise à jour » du menu de la zone de notification, dont il partage le code.
    OpenManualUpdate,
    /// **« Supprimer les données locales »**, *confirmé* — depuis « Vos données » de l'onglet
    /// « À propos » de la fenêtre Options ou depuis la fenêtre de connexion (2026-09-18, constat C5 de
    /// `docs/analyse-rgpd.md` §3.5) : tout ce que l'overlay a écrit sur cette machine est effacé,
    /// puis le programme s'arrête par le même chemin que [`Self::Quit`] — voir
    /// `purge_local_data_and_quit`.
    PurgeLocalData,
}

impl App {
    /// Rend UNE frame de la fenêtre `id` — appelé sur `WindowEvent::RedrawRequested` ET
    /// directement depuis `about_to_wait` quand un redessin est dû (`next_redraw_at`).
    ///
    /// **Pourquoi ne pas s'en remettre à `Window::request_redraw()` seul.** Mesuré le
    /// 2026-09-12 sur la modale Options (journal instrumenté, frappe pilotée par `SendInput`) :
    /// une touche arrivait bien dans `window_event`, `request_redraw()` était appelé — depuis
    /// là comme depuis `about_to_wait` — et AUCUN `RedrawRequested` ne suivait. La frame
    /// suivante ne venait que de la réaffirmation topmost périodique
    /// (`TOPMOST_REASSERT_INTERVAL`, 2 s), dont le `SetWindowPos` fait repeindre la fenêtre par
    /// Windows lui-même. Tout ce qui se tapait dans le champ d'ajout d'alerte s'appliquait
    /// donc par paquets de deux secondes (« le champ d'autocomplétion est extrêmement long »).
    /// Ces fenêtres sont des cibles DirectComposition sans surface de redirection (S1) : le
    /// `WM_PAINT` que `RedrawWindow(RDW_INTERNALPAINT)` est censé poster n'arrive pas de façon
    /// fiable. Le rendu est donc piloté par NOTRE boucle (`about_to_wait`, qui suit
    /// immédiatement chaque livraison d'événements), et `RedrawRequested` n'est plus qu'un
    /// déclencheur parmi d'autres.
    fn redraw(&mut self, event_loop: &ActiveEventLoop, id: WindowId) {
        let mut post_redraw = PostRedraw::None;
        // La bande Récap vient d'être reposée : la config est réécrite une fois le geste fini
        // (jamais pendant, voir le traitement de `RenderOutcome::recap_drag` plus bas). Un
        // drapeau local plutôt qu'une variante de `PostRedraw` : celui-là est écrasé par
        // l'action suivante quand deux se présentent, et une position perdue ne se rattrape pas
        // — il faudrait re-glisser la bande.
        let mut persist_recap_position = false;
        // Même mécanique pour la hauteur du panneau Combat (2026-09-17) : écrite une fois, au
        // relâchement.
        let mut persist_combat_position = false;
        let Some(overlay) = self.windows.get_mut(&id) else {
            return;
        };
        // Fenêtre `Combat` masquée hors combat (voir `sync_panel_visibility`) : rien à peindre,
        // et surtout rien à présenter — un `Present()` sur une surface invisible ne sert à rien.
        // C'est `sync_panel_visibility` qui replanifie un redessin en la faisant réapparaître.
        if !overlay.visible {
            return;
        }
        // Une seule lecture de l'horloge par frame (voir la doc de `RenderContent::now`)
        // — réutilisée ci-dessous pour le gabarit dynamique du Suivi ET transmise à
        // `render`/`build_ui`, plutôt que deux `Instant::now()` distincts à quelques
        // instructions d'écart qui pourraient (rarement) diverger pile à l'expiration
        // d'un toast.
        let now = std::time::Instant::now();
        let snapshot = self.snapshot.load();
        let fight = snapshot.fight_for_character(&overlay.character_name);
        let watchlist_all = self.watchlist.load();
        // **Suivi coupé : le bandeau n'affiche plus aucune tuile** — case « Activer le suivi »
        // (`panels::feature_switch`). Une liste vide plutôt qu'une fenêtre masquée, parce que le
        // bandeau porte aussi le carré de contrôle, seul accès à la fenêtre Options depuis le jeu :
        // la masquer enfermerait dehors qui vient de décocher la case. C'est exactement l'état
        // « aucune entrée suivie », déjà rendu et dimensionné comme tel (voir
        // `panels::watchlist::show` et `watchlist_target_width`) — le moteur, lui, continue de
        // compter, rien n'est perdu (voir `EngineCommand::SetFeatures`).
        let watchlist: &[WatchlistEntry] = if self.features.suivi {
            &watchlist_all
        } else {
            &[]
        };
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
                self.features.suivi,
                toast_active,
                overlay.game_rect.width,
            );
            let target_height =
                watchlist_target_height(toast_active, self.watchlist_selection.is_open());
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
                if let Some(actual) = overlay
                    .window
                    .request_inner_size(winit::dpi::LogicalSize::new(target_width, target_height))
                {
                    Self::reconfigure_surface(&mut overlay.gpu, actual);
                }
                overlay.last_watchlist_width = Some(target_width);
                overlay.last_watchlist_height = Some(target_height);
            }
        }
        let catalog = self.catalog.load();
        let game_servers = self.game_servers.load();
        let auth_status = self.auth_status.load();
        // La modale Options force sa propre interactivité (voir `App::
        // open_options_modal`) — jamais assujettie à `self.interactive` (mode
        // clic-traversant global de Combat/Suivi), sans quoi elle deviendrait elle-même
        // traversable si l'utilisateur avait basculé ce mode juste avant.
        let interactive = matches!(
            overlay.kind,
            OverlayKind::Options | OverlayKind::Login | OverlayKind::ResetConfirm(_)
        ) || self.interactive;
        let this_game_rect = overlay.game_rect;
        let this_game_hwnd = overlay.game_hwnd;
        // Voir `RenderContent::veiled` : la fenêtre Options rattachée à un client couvre sa
        // fenêtre de jeu et la voile ; détachée, elle est à la taille de la modale.
        let veiled = overlay.kind == OverlayKind::Options && !overlay.is_detached();
        // L'état de la mise à jour, copié dans la fenêtre qui l'affiche AVANT chaque rendu
        // (jamais figé à l'ouverture, voir `OptionsModalState::update`).
        let update_status = self.update_status.load();
        if let Some(state) = overlay.login_state.as_mut() {
            if state.update != **update_status {
                state.update = (**update_status).clone();
            }
        }
        if let Some(state) = overlay.options_state.as_mut() {
            state.update = (**update_status).clone();
        }
        // La vue de la session du Récap — construite ici pour chaque fenêtre, même celles qui
        // ne la peignent pas : c'est quatre entiers et une heure, moins cher qu'un branchement.
        let recap_view = panels::recap::RecapView {
            totals: self.recap_session.totals(&snapshot.totals),
            uptime: self.recap_session.uptime(),
            started_at: self.recap_session.started_at_local(),
            resumed: self.recap_session.resumed(),
        };
        // Ce que la bande Récap sait d'elle-même par l'hôte (2026-09-17) — verrou, position
        // personnalisée, et de quel côté sa rangée d'actions tient. Le côté se décide sur la
        // fenêtre de JEU (`recap_placement::actions_below`), que le panneau ne connaît pas.
        let recap_chrome = panels::recap::RecapChrome {
            locked: self.recap_locked,
            moved: self.recap_position.is_some(),
            actions_below: overlay.kind == OverlayKind::Recap && {
                let outer = overlay.window.outer_size();
                let (client, band) = RecapAnchor::new(None, overlay.window.scale_factor())
                    .geometry(overlay.game_rect, outer.width as i32, outer.height as i32);
                recap_placement::actions_below(self.recap_position, client, band)
            },
        };
        // Ce que le panneau Combat sait de lui-même par l'hôte (2026-09-17) — verrou et hauteur
        // personnalisée. Pas de côté à calculer ici : la rangée d'actions vit dans la fenêtre, et
        // c'est le miroir qui la porte du bon côté (voir `panels::combat::paint_actions_row`).
        let combat_chrome = panels::combat::CombatChrome {
            locked: self.combat_locked,
            moved: self.combat_position_y.is_some(),
        };
        let (repaint_delay, outcome) = render(
            &mut overlay.gpu,
            &overlay.window,
            event_loop,
            RenderContent {
                kind: overlay.kind,
                fight,
                portraits: &overlay.portraits,
                combat_frame: &overlay.combat_frame,
                icons: &overlay.icons,
                avatars: overlay.avatars.as_ref(),
                game_servers: &game_servers,
                combat_side: &mut overlay.combat_side,
                combat_metric: &mut overlay.combat_metric,
                watchlist,
                watchlist_enabled: self.features.suivi,
                spells_enabled: self.features.spells_visible(),
                combat_on_right: self.combat_on_right,
                combat_chrome,
                watchlist_selection: &mut self.watchlist_selection,
                watchlist_completions: &self.watchlist_completions,
                watchlist_reset: self.watchlist_reset_pending.as_ref(),
                watchlist_toast,
                catalog: &catalog,
                catalog_stale: self.catalog_stale.load(Ordering::Relaxed),
                remote_icons: &self.remote_icons,
                remote_icon_textures: &mut overlay.remote_icon_textures,
                auth_status: &auth_status,
                auth_command_tx: &self.auth_command_tx,
                interactive,
                shortcuts: self.hotkeys.bindings(),
                now,
                recap_cells: self.features.recap_cells,
                recap: &recap_view,
                options: overlay.options_state.as_mut(),
                veiled,
                recap_chrome,
                login: overlay.login_state.as_mut(),
            },
        );
        // Bloc Récap : retaillé à la hauteur qu'il vient de mesurer — même mécanique et même
        // raison que le Suivi juste au-dessus (une fenêtre plus grande que son contenu bloque les
        // clics sur du vide), à ceci près que la mesure vient du panneau lui-même plutôt que d'un
        // calcul de l'hôte : c'est la largeur des CHIFFRES qui décide si une ligne s'empile, et
        // seul le rendu la connaît. `request_inner_size` est synchrone sous Windows, d'où
        // `reconfigure_surface` ici même (voir sa doc).
        if overlay.kind == OverlayKind::Recap {
            if let Some(height) = outcome.recap_height {
                // **Jamais pendant qu'on la tient** (2026-09-17, demande utilisateur du même
                // retour d'écran) : retailler la fenêtre sous le curseur déplacerait le bloc
                // pendant le geste, et le bornage avec lui. Le contenu, lui, continue de vivre
                // — ce sont les chiffres, ils ne bougent rien. La hauteur en attente s'applique
                // à la frame suivant le relâchement, `last_recap_height` étant resté sur
                // l'ancienne valeur.
                if overlay.last_recap_height != Some(height) && self.recap_drag.is_none() {
                    let size = winit::dpi::LogicalSize::new(
                        panels::recap::WIDTH as f64,
                        height as f64
                            + render_content::RECAP_TOOLTIP_RESERVE as f64
                            + render_content::RECAP_ACTIONS_RESERVE as f64,
                    );
                    if let Some(actual) = overlay.window.request_inner_size(size) {
                        Self::reconfigure_surface(&mut overlay.gpu, actual);
                    }
                    overlay.last_recap_height = Some(height);
                    overlay.next_redraw_at = Some(std::time::Instant::now());
                }
            }
            // **La bande saisie à la souris** (2026-09-17, voir `panels::recap::RecapDrag`) :
            // le panneau remonte le geste, l'hôte pose la fenêtre — comme pour la remise à zéro,
            // le panneau n'agit jamais lui-même.
            //
            // La position visée vaut toujours « le curseur À L'ÉCRAN, moins le point du bloc par
            // lequel on le tient », le point de saisie étant figé au premier appui. Le curseur
            // d'écran vient de l'OS (`game_window::cursor_position`) et non d'egui : la position
            // qu'egui rapporte est mesurée depuis le coin de cette fenêtre-ci, donc s'en servir
            // pour la déplacer reboucle et la fait vibrer — voir `recap_placement::drag_offset`,
            // qui porte le diagnostic complet.
            let scale = overlay.window.scale_factor();
            let outer = overlay.window.outer_size();
            let (client, band) = RecapAnchor::new(None, scale).geometry(
                overlay.game_rect,
                outer.width as i32,
                outer.height as i32,
            );
            // Le curseur arrive en points logiques (repère d'egui), tout le reste est en pixels
            // d'écran — à 125 %, confondre les deux ferait partir la bande une fois et quart
            // trop loin.
            let physical = |pos: egui::Pos2| {
                (
                    (pos.x as f64 * scale).round() as i32,
                    (pos.y as f64 * scale).round() as i32,
                )
            };
            let mut place = |position: Option<(i32, i32)>| {
                let (x, y) = recap_placement::window_position(position, client, band);
                let posed = PhysicalPosition::new(x, y);
                if overlay.last_position != Some(posed) {
                    overlay.window.set_outer_position(posed);
                    overlay.last_position = Some(posed);
                }
            };
            match outcome.recap_drag {
                panels::recap::RecapDrag::None => {}
                panels::recap::RecapDrag::Started(pos) => {
                    self.recap_drag = Some(RecapDragState {
                        window: id,
                        grab: physical(pos),
                    });
                }
                // `filter` sur la fenêtre saisie : en multicompte, chaque client a sa bande, et
                // ce n'est pas parce que l'une est tenue que les autres bougent.
                panels::recap::RecapDrag::Moved => {
                    if let Some(drag) = self.recap_drag.filter(|drag| drag.window == id) {
                        // Curseur illisible : la bande reste où elle est, ce geste-ci n'a
                        // simplement pas d'effet cette frame.
                        if let Some(cursor) = game_window::cursor_position() {
                            let offset =
                                recap_placement::drag_offset(cursor, drag.grab, client, band);
                            if self.recap_position != Some(offset) {
                                self.recap_position = Some(offset);
                                place(Some(offset));
                            }
                        }
                    }
                }
                // La pose : l'aimantation d'abord (une bande revenue près de son ancrage
                // d'origine y recolle et la config oublie sa position, voir
                // `recap_placement::snap`), l'écriture ensuite — une seule, pour tout le geste.
                panels::recap::RecapDrag::Released => {
                    if self.recap_drag.is_some_and(|drag| drag.window == id) {
                        self.recap_drag = None;
                        self.recap_position = self.recap_position.and_then(recap_placement::snap);
                        place(self.recap_position);
                        persist_recap_position = true;
                        match self.recap_position {
                            Some((x, y)) => tracing::info!("[recap] bande posée en {x} / {y}."),
                            None => {
                                tracing::info!("[recap] bande revenue à son emplacement d'origine.")
                            }
                        }
                    }
                }
            }
        }
        // **Le panneau Combat saisi par sa poignée latérale** (2026-09-17) : la même mécanique que
        // la bande ci-dessus, en une seule dimension — le côté ne se déplace pas à la souris (voir
        // `combat_placement`), seule la hauteur bouge. Le curseur vient de l'OS et non d'egui,
        // pour la raison qui a fait vibrer la bande (`recap_placement::drag_offset`).
        if overlay.kind == OverlayKind::Combat {
            // Le côté est copié AVANT la fermeture qui pose la fenêtre : elle ne doit rien
            // garder de `self`, dont d'autres champs changent juste après (la hauteur, l'état du
            // glissement).
            let on_right = self.combat_on_right;
            let scale = overlay.window.scale_factor();
            let outer = overlay.window.outer_size();
            let (client, panel) = CombatAnchor::new(on_right, None, scale).geometry(
                overlay.game_rect,
                outer.width as i32,
                outer.height as i32,
            );
            let mut place = |offset: Option<i32>| {
                let (x, y) = combat_placement::window_position(offset, on_right, client, panel);
                let posed = PhysicalPosition::new(x, y);
                if overlay.last_position != Some(posed) {
                    overlay.window.set_outer_position(posed);
                    overlay.last_position = Some(posed);
                }
            };
            match outcome.combat_drag {
                panels::drag::PanelDrag::None => {}
                panels::drag::PanelDrag::Started(pos) => {
                    self.combat_drag = Some(CombatDragState {
                        window: id,
                        // Le curseur arrive en points logiques (repère d'egui), tout le reste est
                        // en pixels d'écran — à 125 %, confondre les deux ferait partir le panneau
                        // une fois et quart trop loin.
                        grab_y: (pos.y as f64 * scale).round() as i32,
                    });
                }
                panels::drag::PanelDrag::Moved => {
                    if let Some(drag) = self.combat_drag.filter(|drag| drag.window == id) {
                        // Curseur illisible : le panneau reste où il est, ce geste-ci n'a
                        // simplement pas d'effet cette frame.
                        if let Some((_, cursor_y)) = game_window::cursor_position() {
                            let offset =
                                combat_placement::drag_offset(cursor_y, drag.grab_y, client, panel);
                            if self.combat_position_y != Some(offset) {
                                self.combat_position_y = Some(offset);
                                place(Some(offset));
                            }
                        }
                    }
                }
                panels::drag::PanelDrag::Released => {
                    if self.combat_drag.is_some_and(|drag| drag.window == id) {
                        self.combat_drag = None;
                        self.combat_position_y = self
                            .combat_position_y
                            .and_then(|offset| combat_placement::snap(offset, client, panel));
                        place(self.combat_position_y);
                        persist_combat_position = true;
                        match self.combat_position_y {
                            Some(y) => tracing::info!("[combat] panneau posé à la hauteur {y}."),
                            None => {
                                tracing::info!("[combat] panneau revenu à sa hauteur d'origine.")
                            }
                        }
                    }
                }
            }
        }
        // Fenêtre de connexion : retaillée à la hauteur que la carte vient d'occuper (chaque
        // état a la sienne), en gardant son centre — `request_inner_size` est synchrone sous
        // Windows, d'où `reconfigure_surface` ici même (voir sa doc). Et déplacée à la souris
        // sur demande de sa bannière : sans décorations OS, c'est le seul moyen.
        if overlay.kind == OverlayKind::Login {
            if let Some(height) = outcome.login_height {
                if overlay.last_login_height != Some(height) {
                    let before = overlay.window.outer_size();
                    if let Some(actual) =
                        overlay
                            .window
                            .request_inner_size(winit::dpi::LogicalSize::new(
                                login::WINDOW_WIDTH as f64,
                                height as f64,
                            ))
                    {
                        Self::reconfigure_surface(&mut overlay.gpu, actual);
                    }
                    let after = overlay.window.outer_size();
                    if let Ok(position) = overlay.window.outer_position() {
                        let shift = (after.height as i32 - before.height as i32) / 2;
                        overlay.window.set_outer_position(PhysicalPosition::new(
                            position.x,
                            position.y - shift,
                        ));
                    }
                    overlay.last_login_height = Some(height);
                    overlay.next_redraw_at = Some(std::time::Instant::now());
                }
            }
            if outcome.drag_window {
                if let Err(err) = overlay.window.drag_window() {
                    tracing::warn!("[connexion] déplacement de la fenêtre refusé : {err}");
                }
            }
            if outcome.retry_update {
                post_redraw = PostRedraw::RetryUpdate;
            }
            // Écran de mise à jour manuelle (voir `open_manual_update_window`) — « Rechercher à
            // nouveau » de cet écran, et « Mise à jour » du pied de la Carte, qui l'ouvre depuis
            // n'importe quel autre écran (2026-09-22).
            if outcome.check_update {
                post_redraw = PostRedraw::OpenManualUpdate;
            }
            if outcome.install_update {
                post_redraw = PostRedraw::StartManualUpdate;
            }
            if outcome.close_update {
                post_redraw = PostRedraw::CloseManualUpdate;
            }
            if outcome.purge_local_data {
                post_redraw = PostRedraw::PurgeLocalData;
            }
        }
        // Fermeture au clic (carte ou croix, voir `panels::watchlist::toast_card`) — seul
        // point du code à détenir un accès en écriture à cet `ArcSwap` (`render` ne reçoit
        // le toast qu'en lecture, voir `RenderContent::watchlist_toast`). Le clic ayant
        // déjà fait passer `response.repaint` à `true` plus haut, le prochain redessin
        // relira `None` et n'affichera plus rien.
        if outcome.close_toast {
            self.watchlist_toast.store(Arc::new(None));
        }
        // Carte d'alerte de chat cliquée : la réponse en privé se prépare dans la fenêtre de jeu
        // — celle au premier plan si c'en est une, la première trouvée sinon (voir
        // `chat_command::send_whisper`).
        if let Some(author) = outcome.whisper_to {
            post_redraw = PostRedraw::Whisper(author);
        }
        // Suppression groupée demandée depuis le bandeau : même chemin que la validation de
        // l'onglet « Suivi » (`commit_suivi`) — seules les DÉFINITIONS partent, le moteur garde ses
        // compteurs et réplique au compte de lui-même. L'`ArcSwap` local n'est pas touché ici :
        // c'est le moteur qui republie la liste, compteurs vivants compris.
        if let Some(edition) = outcome.watchlist_edit {
            tracing::info!(
                entry_count = edition.definitions.len(),
                geste = edition.reason.label(),
                "[bandeau] définitions modifiées"
            );
            let _ = self
                .settings_tx
                .send(EngineCommand::SetWatchlistDefinitions {
                    definitions: edition.definitions,
                    // Le bandeau n'a pas de brouillon : une entrée qu'il retire disparaît de la
                    // liste dans la foulée, son compteur part avec elle (rien à oublier en plus),
                    // et il ne peut en recréer aucune.
                    retirees: Vec::new(),
                });
        }
        // Voir `render_content::RenderOutcome` (2026-09-08, §9 du plan) : bouton "+"/"Options"
        // cliqué dans le carré de contrôle de CETTE fenêtre Suivi, ou action de la modale
        // Options elle-même — jamais deux de ces trois à la fois (branches différentes du
        // `match kind` de `paint_content`).
        if outcome.open_watchlist {
            post_redraw = PostRedraw::OpenOptions(
                this_game_hwnd,
                this_game_rect,
                options_modal::OptionsTab::Suivi,
            );
        }
        if outcome.open_options {
            post_redraw = PostRedraw::OpenOptions(
                this_game_hwnd,
                this_game_rect,
                options_modal::OptionsTab::Parametres,
            );
        }
        // L'ouverture de page est faite ICI, par l'hôte, jamais par le panneau qui l'a
        // demandée : voir `RenderOutcome::open_url`. `open::that` est best-effort, comme
        // partout ailleurs — un navigateur qui ne s'ouvre pas ne fait rien planter.
        if let Some(url) = &outcome.open_url {
            let _ = open::that(url);
        }
        // Le bouton de réinitialisation d'une tuile du bandeau (2026-09-18) : même confirmation
        // que le Récap, par-dessus la fenêtre de jeu de CE bandeau, l'entrée retenue par l'hôte.
        if let Some(entry) = outcome.watchlist_reset_requested {
            post_redraw =
                PostRedraw::OpenWatchlistResetConfirm(this_game_hwnd, this_game_rect, entry);
        }
        // Le glyphe de remise à zéro du bloc Récap (2026-09-17) : la confirmation s'ouvre
        // par-dessus la fenêtre de jeu de CE bloc ; sa réponse, elle, arrive par la fenêtre de
        // confirmation elle-même, une frame plus tard.
        if outcome.recap_reset_requested {
            post_redraw = PostRedraw::OpenResetConfirm(
                this_game_hwnd,
                this_game_rect,
                ResetTarget::RecapSession,
            );
        }
        // Le glyphe de replacement de la rangée d'actions (2026-09-17) : même fenêtre, même
        // voile, autre question — « on remet le récap à son emplacement initial seulement si
        // l'utilisateur appuie sur oui ».
        if outcome.recap_restore_requested {
            post_redraw = PostRedraw::OpenResetConfirm(
                this_game_hwnd,
                this_game_rect,
                ResetTarget::RecapPosition,
            );
        }
        // Le cadenas : la bascule est immédiate et persistée, sans confirmation — rien ne se
        // perd, et le glyphe montre aussitôt l'état obtenu. Après le rendu comme tout ce qui
        // touche `self` entier (la fenêtre est empruntée jusque-là).
        if outcome.recap_toggle_lock {
            post_redraw = PostRedraw::ToggleRecapLock;
        }
        // Les deux mêmes commandes pour le panneau Combat (2026-09-17) — replacement sous
        // confirmation, cadenas immédiat.
        if outcome.combat_restore_requested {
            post_redraw = PostRedraw::OpenResetConfirm(
                this_game_hwnd,
                this_game_rect,
                ResetTarget::CombatPosition,
            );
        }
        if outcome.combat_toggle_lock {
            post_redraw = PostRedraw::ToggleCombatLock;
        }
        if let OverlayKind::ResetConfirm(target) = overlay.kind {
            match outcome.reset_choice {
                overlay_ui::design::ConfirmChoice::Pending => {}
                overlay_ui::design::ConfirmChoice::Yes => {
                    post_redraw = PostRedraw::AnswerResetConfirm(target, true)
                }
                overlay_ui::design::ConfirmChoice::No => {
                    post_redraw = PostRedraw::AnswerResetConfirm(target, false)
                }
            }
        }
        match outcome.options_action {
            OptionsModalAction::None => {}
            OptionsModalAction::Cancel => post_redraw = PostRedraw::CloseOptions,
            OptionsModalAction::Browse => post_redraw = PostRedraw::BrowseOptions,
            // Les sons d'essai se jouent par le même chemin qu'en jeu — c'est tout l'intérêt du
            // bouton : entendre ce qu'on entendra.
            OptionsModalAction::TestAlertSound => alert_sound::play_loot_alert(),
            OptionsModalAction::TestChatSound => alert_sound::play_chat_alert(),
            OptionsModalAction::TestCountdownSound => alert_sound::play_countdown_alert(),
            OptionsModalAction::TestTurnSound => alert_sound::play_turn_alert(),
            OptionsModalAction::Disconnect => post_redraw = PostRedraw::DisconnectAccount,
            OptionsModalAction::Validate(commit) => {
                post_redraw = PostRedraw::ValidateOptions(commit)
            }
            OptionsModalAction::ResolveRecipe(id) => post_redraw = PostRedraw::ResolveRecipe(id),
            OptionsModalAction::CheckUpdate => post_redraw = PostRedraw::CheckUpdate,
            // Déjà traduite en `outcome.open_url` par `render_content` (voir la variante) ;
            // ne parvient jamais ici.
            OptionsModalAction::OpenUrl(_) => {}
            OptionsModalAction::InstallUpdate => post_redraw = PostRedraw::InstallUpdate,
            OptionsModalAction::PurgeLocalData => post_redraw = PostRedraw::PurgeLocalData,
            OptionsModalAction::Quit => post_redraw = PostRedraw::Quit,
            OptionsModalAction::Restart => post_redraw = PostRedraw::Restart,
        }
        // Voir `OverlayWindow::next_redraw_at` : egui a pu demander un redessin après un
        // délai (tooltip...) que rien d'autre ne redéclenchera dans cette architecture.
        // `about_to_wait` est responsable de le consommer le moment venu.
        overlay.next_redraw_at = (repaint_delay < std::time::Duration::from_secs(3600))
            .then(|| std::time::Instant::now() + repaint_delay);

        // Après la dernière ligne qui touche `overlay` : `persist_config` a besoin de tout
        // `self`, fenêtres comprises.
        if persist_recap_position || persist_combat_position {
            self.persist_config();
        }

        match post_redraw {
            PostRedraw::None => {}
            PostRedraw::OpenOptions(hwnd, rect, tab) => {
                self.open_options_modal(event_loop, Some((hwnd, rect)), tab)
            }
            PostRedraw::CloseOptions => self.close_options_modal(id, "Annuler"),
            PostRedraw::OpenResetConfirm(hwnd, rect, target) => {
                self.open_reset_confirm(event_loop, hwnd, rect, target)
            }
            PostRedraw::AnswerResetConfirm(target, confirmed) => {
                self.answer_reset_confirm(id, target, confirmed)
            }
            PostRedraw::OpenWatchlistResetConfirm(hwnd, rect, entry) => {
                self.open_watchlist_reset_confirm(event_loop, hwnd, rect, entry)
            }
            PostRedraw::ToggleRecapLock => self.toggle_recap_lock(),
            PostRedraw::ToggleCombatLock => self.toggle_combat_lock(),
            PostRedraw::Whisper(author) => self.whisper_from_toast(&author),
            // La déconnexion referme la fenêtre : l'overlay revient à son écran de connexion, et
            // ce qu'on y réglait (liste suivie, alertes) appartient au compte qu'on vient de
            // quitter. Ce qui n'a pas été validé est donc abandonné — c'est ce que la confirmation
            // annonce avant le clic.
            PostRedraw::DisconnectAccount => {
                self.disconnect_account();
                self.close_options_modal(id, "Déconnexion");
            }
            PostRedraw::BrowseOptions => self.start_file_dialog(),
            PostRedraw::ValidateOptions(commit) => self.validate_and_commit_options(id, commit),
            PostRedraw::ResolveRecipe(item_id) => self.start_recipe_resolution(id, item_id),
            PostRedraw::CheckUpdate => {
                tracing::info!(">>> Recherche de mise à jour (fenêtre Options).");
                let _ = self.update_command_tx.send(UpdateCommand::Check {
                    install_if_available: false,
                });
            }
            PostRedraw::InstallUpdate => self.request_update_install(id),
            PostRedraw::RetryUpdate => {
                let _ = self.update_command_tx.send(UpdateCommand::Check {
                    install_if_available: true,
                });
            }
            // Même sortie que l'entrée « Quitter » de la zone de notification : la borne de fin
            // de session d'abord (§11 du plan), puis la boucle s'arrête — les fenêtres, modale
            // comprise, tombent avec elle.
            PostRedraw::Quit => {
                logging::log_session_end("Fermer l'overlay (fenêtre Options)");
                event_loop.exit();
            }
            PostRedraw::PurgeLocalData => {
                self.purge_local_data_and_quit(event_loop);
            }
            // Même sortie que « Fermer l'overlay », un process neuf en plus — lancé AVANT de
            // sortir (voir `restart::relaunch`). Une relance impossible ne ferme rien : l'overlay
            // en place reste ouvert, avec la cause au journal, plutôt que de laisser l'utilisateur
            // sans overlay du tout.
            PostRedraw::Restart => match overlay_ui::restart::relaunch() {
                Ok(()) => {
                    logging::log_session_end("Redémarrer l'overlay (fenêtre Options)");
                    event_loop.exit();
                }
                Err(err) => {
                    tracing::error!("[redémarrage] impossible de relancer l'overlay : {err}");
                }
            },
            // Même chemin que « Mettre à jour vers X » de la fenêtre Options
            // (`request_update_install`), sans fenêtre Options à refermer : le démarrage est
            // rebloqué le temps du téléchargement, ce qui referme les overlays de jeu et laisse
            // l'écran de mise à jour seul à l'écran, avec sa jauge.
            PostRedraw::StartManualUpdate => {
                tracing::info!(">>> Mise à jour demandée (écran de mise à jour).");
                self.startup.set_update_blocking(true);
                let _ = self.update_command_tx.send(UpdateCommand::Download);
            }
            PostRedraw::CloseManualUpdate => self.close_manual_update_window(event_loop),
            PostRedraw::OpenManualUpdate => self.open_manual_update_window(event_loop),
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.install_tray();
        self.sync_session_windows(event_loop);
        self.sync_windows(event_loop);
        if !self.banner_printed {
            tracing::info!("=== wakfu-companion-overlay (L2, overlay-ui) ===");
            // Chemin EXPURGÉ du nom d'utilisateur du système (constat C6 de
            // `docs/analyse-rgpd.md`, `overlay_ingest::privacy`) : la forme du chemin — Steam,
            // Wine, installation native, dossier déplacé — reste entière, c'est elle qu'on lit
            // ici ; le chemin réel part en `debug` (« Journal détaillé »).
            tracing::info!(
                "Suivi de {}",
                overlay_ingest::privacy::redact_path(&self.log_path)
            );
            tracing::debug!(path = %self.log_path.display(), "chemin de journal suivi");
            // Libellés LUS dans les raccourcis effectifs : une bannière qui annoncerait les
            // combinaisons par défaut à qui les a personnalisées serait un contresens.
            let bindings = self.hotkeys.bindings();
            tracing::info!(
                "{} pour basculer interactif / clic-traversant. \
                 {} pour forcer un rafraîchissement (overlay bloqué/mal \
                 positionné, ou Suivi resté vide). {} pour la fenêtre Options — \
                 son onglet « Raccourcis » personnalise tout ceci, sa \
                 section « Compte » déconnecte le compte lié, et son bouton \
                 « Fermer l'overlay » quitte (comme la zone de notification, \
                 ou Ctrl+C dans ce terminal). Un compte est obligatoire : la \
                 fenêtre de connexion reste seule à l'écran tant qu'aucun \
                 n'est lié.",
                bindings.label(ShortcutAction::Toggle),
                bindings.label(ShortcutAction::Refresh),
                bindings.label(ShortcutAction::Options),
            );
            self.banner_printed = true;
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            // Les deux variantes ont le même effet ici : un nouvel état est disponible (snapshot
            // de combat, ou statut de connexion au compte), toutes les fenêtres doivent redessiner
            // pour le refléter (le statut de connexion pilote la fenêtre de connexion — et, au
            // prochain tick, quelles fenêtres existent, voir `sync_session_windows`).
            UserEvent::NewSnapshot
            | UserEvent::AuthStatusChanged
            | UserEvent::StartupProgress
            | UserEvent::UpdateProgress => {
                for overlay in self.windows.values_mut() {
                    overlay.next_redraw_at = Some(std::time::Instant::now());
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
        let mut post_redraw = PostRedraw::None;

        let Some(overlay) = self.windows.get_mut(&id) else {
            return; // événement d'une fenêtre déjà retirée (client fermé entre-temps) — ignoré
        };

        let response = overlay
            .gpu
            .egui_winit
            .on_window_event(&overlay.window, &event);
        if response.repaint {
            // Pas un `request_redraw()` : le rendu est planifié pour `about_to_wait`, qui suit
            // immédiatement la livraison de cet événement et rend la frame lui-même — voir la doc
            // de `App::redraw` pour la mesure qui a imposé ce chemin.
            overlay.next_redraw_at = Some(std::time::Instant::now());
        }

        if matches!(event, WindowEvent::RedrawRequested) {
            self.redraw(event_loop, id);
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                // **La croix de la fenêtre Options ferme la MODALE, pas l'overlay.** Elle quittait
                // tout jusqu'au 2026-09-12 : cette fenêtre est la seule focalisable (§9.1 du plan),
                // donc la seule dont la croix est réellement atteignable à la souris, et elle
                // tuait la session entière. Avec des modifications en attente, elle passe par la
                // même garde que « Annuler » et Échap — les trois gestes ferment la même chose.
                if overlay.kind == OverlayKind::Options {
                    match overlay.options_state.as_mut() {
                        Some(state) if state.is_dirty() => {
                            state.pending_close = true;
                            overlay.next_redraw_at = Some(std::time::Instant::now());
                        }
                        _ => post_redraw = PostRedraw::CloseOptions,
                    }
                } else if let OverlayKind::ResetConfirm(target) = overlay.kind {
                    // Fermer la question, c'est répondre « Non ».
                    post_redraw = PostRedraw::AnswerResetConfirm(target, false);
                } else if overlay
                    .login_state
                    .as_ref()
                    .is_some_and(|state| state.manual_update)
                {
                    // **La croix de l'écran de mise à jour ferme CET écran, pas l'overlay** — même
                    // raison que la modale Options ci-dessus : cette fenêtre est focalisable, sa
                    // croix est atteignable, et l'utilisateur qui vient de demander une recherche
                    // de mise à jour ne demande pas à quitter sa session de jeu.
                    post_redraw = PostRedraw::CloseManualUpdate;
                } else {
                    logging::log_session_end("fermeture de fenêtre");
                    event_loop.exit();
                }
            }
            // **Aucun filet « Échap quitte l'overlay ».** Il y en a eu un jusqu'au 2026-09-17
            // (`Échap` → `event_loop.exit()` pour toute fenêtre autre que la modale Options, la
            // fenêtre de connexion et la confirmation de remise à zéro), posé quand les fenêtres
            // overlay étaient réputées ne jamais recevoir d'événement clavier — `WS_EX_NOACTIVATE`
            // était censé les tenir hors du premier plan.
            //
            // **Cette prémisse est fausse, et le filet a fermé l'overlay entier** (retour
            // utilisateur, 2026-09-17 : « la touche Échap en ayant la modale Options ferme
            // complètement l'overlay »). En mode interactif, un clic sur un bandeau Combat/Suivi
            // fait bel et bien de SA PROPRE `HWND` la fenêtre au premier plan malgré
            // `WS_EX_NOACTIVATE` — c'est le même constat, journaux à l'appui, qui a imposé le
            // calcul par personnage de `sync_topmost` (voir son correctif 2026-09-06/07). Le
            // geste qui déclenchait le défaut est donc parfaitement ordinaire : cliquer
            // « Options » dans le bandeau Suivi (ce clic donne le premier plan AU BANDEAU), puis
            // taper Échap pour refermer la modale — la touche partait au bandeau, pas à la
            // modale, et tuait la session.
            //
            // Le filet disparaît plutôt que de s'allonger d'une exclusion de plus : une touche
            // nue qui arrête le programme n'a pas sa place, exactement comme le raccourci global
            // « Quitter l'overlay » retiré le même jour. Les sorties propres sont le bouton
            // « Fermer l'overlay » de l'onglet « Paramètres » (confirmé), l'entrée « Quitter » de
            // la zone de notification, Alt+F4 / la croix pour les fenêtres qui en ont une
            // (`CloseRequested` ci-dessus) et Ctrl+C au terminal. Échap, lui, appartient
            // désormais aux seuls panneaux qui le lisent : il annule la modale Options, répond
            // « Non » à une confirmation, referme un sélecteur.
            WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                Self::reconfigure_surface(&mut overlay.gpu, size);
            }
            _ => {}
        }

        if let PostRedraw::CloseOptions = post_redraw {
            self.windows.remove(&id);
            tracing::info!("[options] modale fermée (Annuler).");
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Une mise à jour prête s'installe AVANT tout le reste du tick : l'exe est remplacé,
        // l'overlay relancé, et cette boucle se termine (voir `install_update_if_ready`).
        self.install_update_if_ready(event_loop);
        if event_loop.exiting() {
            return;
        }
        // **Les suivis qui viennent d'aboutir**, avant tout le reste du tick : leur retrait ne
        // dépend ni d'une fenêtre visible ni d'un rendu (voir `tick_watchlist_completions`).
        self.tick_watchlist_completions();
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
            let Some(action) = self.hotkeys.action_for(event.id) else {
                continue;
            };
            match action {
                ShortcutAction::Toggle => self.toggle_interactive(),
                ShortcutAction::Refresh => self.force_refresh(event_loop),
                ShortcutAction::Details => self.open_details(),
                ShortcutAction::Options => {
                    tracing::info!(
                        ">>> Options ({})",
                        self.hotkeys.bindings().label(ShortcutAction::Options)
                    );
                    // Même destination que le bouton "Options" qu'il double — voir la doc de
                    // `PostRedraw::OpenOptions` (2026-09-13) : un raccourci et le bouton qu'il
                    // double doivent mener au même endroit, jamais au défaut implicite
                    // d'`OptionsTab`.
                    self.open_options_modal(
                        event_loop,
                        None,
                        options_modal::OptionsTab::Parametres,
                    );
                }
                ShortcutAction::WatchlistAdd => {
                    tracing::info!(
                        ">>> Ajouter ({})",
                        self.hotkeys.bindings().label(ShortcutAction::WatchlistAdd)
                    );
                    // Même destination que le bouton "+" qu'il double — voir plus haut.
                    self.open_options_modal(event_loop, None, options_modal::OptionsTab::Suivi);
                }
                ShortcutAction::WatchlistRemove => {
                    tracing::info!(
                        ">>> Supprimer ({})",
                        self.hotkeys
                            .bindings()
                            .label(ShortcutAction::WatchlistRemove)
                    );
                    // Même geste que le bouton « − » qu'il double : il ouvre et referme le mode,
                    // il ne supprime rien. La fenêtre change de hauteur avec lui, et rien d'autre
                    // ne la redessine — sans ce réveil, le raccourci n'aurait d'effet qu'au
                    // prochain événement venu d'ailleurs.
                    self.watchlist_selection.toggle_mode();
                    self.request_watchlist_redraw();
                }
                ShortcutAction::CombatSide => self.toggle_combat_side(),
                ShortcutAction::CombatMetric => self.cycle_combat_metric(),
                ShortcutAction::InvitePartner => {
                    self.send_partner_command(ChatCommand::Invite);
                }
                ShortcutAction::FollowPartner => {
                    self.send_partner_command(ChatCommand::Follow);
                }
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
                        // Même garde que "Valider" (voir `validate_and_commit_options`) : `rfd` ne
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
                            overlay.next_redraw_at = Some(std::time::Instant::now());
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => self.pending_dialog = None,
            }
        }

        // Ingrédients d'une recette, même sondage non bloquant — voir `start_recipe_resolution`.
        // Un thread mort (`Disconnected`) laisse la fenêtre sur son rouage plutôt que d'afficher
        // une liste vide, qui se lirait comme « cette recette n'a pas d'ingrédient » ; fermer la
        // fenêtre annule de toute façon la demande.
        if let Some((window_id, rx)) = &self.pending_recipe {
            let window_id = *window_id;
            match rx.try_recv() {
                Ok(ingredients) => {
                    self.pending_recipe = None;
                    if let Some(overlay) = self.windows.get_mut(&window_id) {
                        if let Some(state) = &mut overlay.options_state {
                            if let Some(dialogue) = state.suivi.recipe.as_mut() {
                                dialogue.ingredients = Some(ingredients);
                            }
                        }
                        overlay.next_redraw_at = Some(std::time::Instant::now());
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => self.pending_recipe = None,
            }
        }

        // Menu de l'icône de zone de notification — même sondage que les hotkeys.
        self.handle_tray_menu(event_loop);

        // Fenêtre de connexion ou overlays de jeu, jamais les deux — d'après l'état du compte
        // (voir `sync_session_windows`). AVANT `sync_windows`, qui ne fait rien compte non lié.
        self.sync_session_windows(event_loop);
        // Découverte/suivi des fenêtres de jeu : même sondage périodique que le hotkey (pas d'API
        // Win32 pour être notifié d'un déplacement/redimensionnement/apparition d'une fenêtre qui
        // n'est pas la nôtre sans un hook global — un sondage à 20 Hz est largement assez réactif
        // ici et reste négligeable en coût, voir game_window.rs).
        self.sync_windows(event_loop);
        // Apparition/disparition automatique du panneau Combat — entre les deux : `sync_windows`
        // vient peut-être de créer la fenêtre, `sync_topmost` doit voir son état final.
        self.sync_panel_visibility();
        self.sync_topmost();
        // Surveillance de tour (§9.1 decies) — après `sync_windows`, qui vient de mettre à jour
        // la liste des fenêtres de jeu qu'elle lit.
        self.sync_turn_watch();

        // Honore les délais de redessin qu'egui a demandés (tooltip au survol d'un portrait,
        // typiquement) et qu'aucun `WindowEvent`/`UserEvent` ne redéclenchera de lui-même — voir
        // `OverlayWindow::next_redraw_at` et `render`. Sans ceci, la tooltip ne s'affichait qu'au
        // hasard d'un autre redessin (retour utilisateur 2026-09-01).
        let now = std::time::Instant::now();
        let mut next_wake = now + std::time::Duration::from_millis(50);
        let mut dues = Vec::new();
        for (id, overlay) in &mut self.windows {
            if let Some(due) = overlay.next_redraw_at {
                if due <= now {
                    overlay.next_redraw_at = None;
                    dues.push(*id);
                } else {
                    next_wake = next_wake.min(due);
                }
            }
        }
        // Rendu DIRECT, sans passer par `request_redraw()` — voir la doc de `App::redraw`.
        for id in dues {
            self.redraw(event_loop, id);
        }
        // Un rendu peut avoir replanifié un redessin immédiat (animation egui) : le réveil
        // suivant doit le voir sans attendre le tick de 50 ms.
        let now = std::time::Instant::now();
        for overlay in self.windows.values() {
            if let Some(due) = overlay.next_redraw_at {
                next_wake = next_wake.min(due.max(now));
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
            // Plancher `webgl2` (l'overlay ne demande rien de plus), mais avec les limites
            // de RÉSOLUTION de l'adaptateur : la swapchain de la confirmation de remise à
            // zéro couvre la fenêtre de jeu ENTIÈRE (`OverlayKind::ResetConfirm`), et
            // `downlevel_webgl2_defaults()` plafonne une texture 2D à 2048 px — un jeu en
            // 2560×1392 faisait donc paniquer `Surface::configure` au clic sur le glyphe
            // (2026-09-17). L'idiome est celui que wgpu documente sur `using_resolution`.
            required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                .using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
            ..Default::default()
        })
        .await
        .expect("création du device");

    let size = window.inner_size();
    // Même clamp défensif qu'à la reconfiguration (voir `reconfigure_surface` côté
    // Windows, `WindowEvent::Resized` côté Linux) : une erreur wgpu est FATALE par défaut,
    // une fenêtre plus grande que ce que le GPU accepte ne doit jamais tuer l'overlay.
    let max_dim = device.limits().max_texture_dimension_2d;
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
        width: size.width.clamp(1, max_dim),
        height: size.height.clamp(1, max_dim),
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

/// Ordre de priorité complet (voir `config::resolve_log_path`) : argument CLI > chemin sauvegardé
/// par une validation précédente de la modale Options (2026-09-08, §9 du plan) > découverte
/// automatique. Seule la découverte automatique peut échouer complètement (aucun argument, rien en
/// config, aucun chemin connu du système) — dans ce cas SEULEMENT, ce binaire refuse encore de
/// démarrer sans chemin explicite (l'Engine a besoin d'un chemin dès `spawn_engine_thread`, voir
/// `main`).
fn resolve_path(config: &config::OverlayConfig, cli_arg: Option<PathBuf>) -> PathBuf {
    match config::resolve_log_path(cli_arg, config) {
        Some(path) => path,
        None => {
            tracing::error!("wakfu.log introuvable aux emplacements connus. Chemins essayés :");
            for candidate in discovery::candidate_paths() {
                tracing::error!("  - {}", overlay_ingest::privacy::redact_path(&candidate));
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
    // Avant TOUTE écriture sur le disque : `local_data::has_user_data` compare la date de
    // `config.toml` à cet instant pour distinguer une installation déjà utilisée d'une première
    // ouverture, et ce fichier est écrit par le démarrage en cours.
    build_info::mark_process_start();
    // En tout premier, avant la moindre ligne de journal : sans console propre (sous-système
    // `windows`, voir l'attribut en tête de fichier), les couches console de `logging` n'ont un
    // destinataire que si le terminal qui nous a lancés nous prête le sien.
    logging::attach_parent_console();
    // Lancé par le clic d'un toast de tour (activation de protocole, voir
    // `turn_watch::notify`) : ce process n'existe que pour donner le premier plan à la fenêtre
    // du personnage, ce que l'overlay déjà en cours n'a pas le droit de faire. Rien d'autre n'est
    // initialisé — pas de journal, pas de moteur — et il se termine aussitôt.
    if let Some(hwnd) = std::env::args()
        .nth(1)
        .as_deref()
        .and_then(turn_watch::notify::parse_focus_uri)
    {
        turn_watch::notify::focus_window(hwnd);
        return;
    }
    let log_dir = logging::init();
    logging::install_ctrlc_handler();
    logging::install_panic_hook();
    if let Some(dir) = &log_dir {
        tracing::info!(
            "journal de session : {}",
            overlay_ingest::privacy::redact_path(dir)
        );
        tracing::debug!(dir = %dir.display(), "dossier du journal de session");
    }
    // À quel déploiement cette session parle — figé par le profil de compilation
    // (`overlay-sync/build.rs`) sauf surcharge `WAKFU_COMPANION_API_URL` : sans cette ligne, un
    // 401 au journal ne dit pas si le jeton a été présenté au bon serveur (cas vécu le 2026-09-17).
    tracing::info!(
        "API : {} ({})",
        overlay_sync::client::base_url(),
        if std::env::var_os("WAKFU_COMPANION_API_URL").is_some() {
            "surcharge WAKFU_COMPANION_API_URL"
        } else {
            "défaut du profil de compilation"
        }
    );

    // Référentiels de sorts (classes et monstres) construits DÈS LE DÉMARRAGE, jamais au premier
    // sort affiché en combat — décision utilisateur du 13 sept. 2026 : deux fichiers de quelques
    // centaines de Ko, très loin du budget mémoire, et aucun à-coup en jeu.
    let (class_spells, monster_spells) = overlay_engine::preload_spell_indexes();
    tracing::info!("référentiels de sorts chargés : {class_spells} sorts de classe, {monster_spells} sorts de monstres");

    // Une seule racine de dossiers depuis le 2026-09-19 (constat C13) : ce que l'ancienne
    // contenait est déplacé avant toute lecture, et elle disparaît.
    config::migrate_legacy_root();
    let saved_config = config::load();
    // **Journal détaillé**, dès que la config est lue (constat C6 de `docs/analyse-rgpd.md`) :
    // `logging::init` démarre toujours au niveau ordinaire — c'est le défaut voulu, rien de
    // personnel dans le fichier tant que rien n'a été demandé — et c'est ici, et seulement si
    // la case est cochée, que les `debug!` s'ouvrent.
    logging::set_verbose(saved_config.verbose_log);

    // `--updated-from X` (relance après une mise à jour, voir `overlay_sync::update::apply`)

    // n'est pas un chemin de log : les arguments sont triés avant de résoudre le fichier.

    let cli = update_apply::parse_args(env::args().skip(1));

    if let Some(from) = &cli.updated_from {
        tracing::info!(
            "[mise à jour] mis à jour {from} → {} — nettoyage du dossier de mise à jour.",
            build_info::VERSION
        );

        update_apply::cleanup(&overlay_sync::update::updates_dir());
    }

    let log_path = resolve_path(&saved_config, cli.log_path);
    let snapshot = Arc::new(ArcSwap::from_pointee(SessionSnapshot::default()));
    let watchlist = Arc::new(ArcSwap::from_pointee(Vec::<WatchlistEntry>::new()));
    let watchlist_toast = Arc::new(ArcSwap::from_pointee(None::<WatchlistToast>));
    let alert_profile = Arc::new(ArcSwap::from_pointee(None));
    let chat_filters: SharedChatFilters = Arc::new(ArcSwap::from_pointee(None));
    let roster_draft: SharedRosterDraft = Arc::new(ArcSwap::from_pointee(None));
    // Les serveurs de jeu ne sont pas un jalon de démarrage : voir la doc du thread.
    let game_servers: Arc<ArcSwap<GameServers>> =
        Arc::new(ArcSwap::from_pointee(GameServers::default()));
    spawn_game_servers_thread(Arc::clone(&game_servers));
    let roster: overlay_ui::engine_thread::SharedRoster = Arc::new(ArcSwap::from_pointee(None));
    let catalog = Arc::new(ArcSwap::from_pointee(CatalogIndex::default()));
    let catalog_stale = Arc::new(AtomicBool::new(false));
    let dungeons = Arc::new(ArcSwap::from_pointee(DungeonIndex::default()));
    let auth_status = Arc::new(ArcSwap::from_pointee(AuthStatus::Connecting));
    let startup = Arc::new(StartupProgress::new());
    let update_status = Arc::new(ArcSwap::from_pointee(UpdateStatus::Idle));

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
        Arc::clone(&startup),
        proxy.clone(),
    );
    spawn_dungeon_thread(Arc::clone(&dungeons), Arc::clone(&startup), proxy.clone());
    // Mise à jour automatique (2026-09-15, `docs/plan-mise-a-jour.md` §7) : la vérification part
    // tout de suite, derrière l'écran de chargement ; avec « Installer automatiquement » coché,
    // une version plus récente est installée avant d'ouvrir le moindre overlay de jeu.
    let (update_command_tx, update_command_rx) = mpsc::channel();
    spawn_update_thread(
        Arc::clone(&update_status),
        Arc::clone(&startup),
        update_command_rx,
        proxy.clone(),
    );
    let _ = update_command_tx.send(UpdateCommand::Check {
        install_if_available: saved_config.auto_update,
    });
    let remote_icons = RemoteIconStore::spawn(proxy.clone());
    // **Le canal des complétions** (2026-09-17) — voir `engine_thread::WatchlistCompleted` sur
    // pourquoi un canal et pas un `ArcSwap` : une complétion perdue est une entrée jamais retirée.
    let (completions_tx, completions_rx) = mpsc::channel();
    spawn_engine_thread(
        log_path.clone(),
        EngineHandles {
            snapshot: Arc::clone(&snapshot),
            watchlist: Arc::clone(&watchlist),
            watchlist_toast: Arc::clone(&watchlist_toast),
            completions: completions_tx,
            alert_profile: Arc::clone(&alert_profile),
            chat_filters: Arc::clone(&chat_filters),
            roster: Arc::clone(&roster),
            roster_draft: Arc::clone(&roster_draft),
            catalog: Arc::clone(&catalog),
            dungeons,
            startup: Arc::clone(&startup),
        },
        proxy,
        settings_rx,
        sync_tx,
    );
    // Les réglages de la carte de chat sont locaux : le moteur les reçoit d'ici, pas du compte.
    let _ = settings_tx.send(EngineCommand::SetChatToast(saved_config.chat_toast()));
    let _ = settings_tx.send(EngineCommand::SetCountdownToast(
        saved_config.countdown_toast(),
    ));
    // Les interrupteurs de fonctionnalité aussi — sans cet envoi, le thread Engine partirait sur
    // son défaut « tout actif » et jouerait les alertes d'une fonctionnalité coupée jusqu'à la
    // prochaine validation de la fenêtre Options.
    let _ = settings_tx.send(EngineCommand::SetFeatures(saved_config.features()));
    // Les sourdines de même : sans cet envoi, la première alerte d'une session sonnerait malgré
    // une case cochée à la session précédente.
    let _ = settings_tx.send(EngineCommand::SetAlertMutes(saved_config.alert_mutes()));

    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(AppState {
        log_path,
        combat_always_visible: saved_config.combat_always_visible,
        combat_on_right: saved_config.combat_on_right,
        combat_position_y: saved_config.combat_position_y,
        combat_locked: saved_config.combat_locked,
        turn_notification: saved_config.turn_notification,
        turn_notification_muted: saved_config.turn_notification_muted,
        features: saved_config.features(),
        alert_mutes: saved_config.alert_mutes(),
        shortcuts: saved_config.shortcuts(),
        snapshot,
        watchlist,
        watchlist_toast,
        alert_profile,
        chat_filters,
        roster_draft,
        game_servers,
        chat_toast: saved_config.chat_toast(),
        countdown_toast: saved_config.countdown_toast(),
        completion: saved_config.completion(),
        completions_rx,
        // À côté des combats en cours (`fight-*.json`) — voir la doc de module de
        // `recap_session` pour ce qui y est écrit et quand.
        recap_session: RecapSession::load(
            overlay_engine::fight_store::default_store_dir().join(recap_session::FILE_NAME),
            saved_config.recap_resume(),
            std::time::SystemTime::now(),
        ),
        recap_position: saved_config.recap_position(),
        recap_locked: saved_config.recap_locked,
        catalog,
        catalog_stale,
        remote_icons,
        auth_status,
        auth_command_tx,
        startup,
        update_status,
        update_command_tx,
        auto_update: saved_config.auto_update,
        verbose_log: saved_config.verbose_log,
        settings_tx,
    });
    event_loop.run_app(&mut app).expect("boucle d'événements");
}
