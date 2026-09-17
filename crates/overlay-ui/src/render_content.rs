//! Construction de l'interface egui à partir d'un snapshot — extrait de `main.rs::render` le
//! 2026-09-03 (§17.1 du plan) pour que cette logique soit appelable sans fenêtre système ni GPU
//! physique (voir la doc de [`build_ui`]). Tout ce module est indépendant de `winit`/`wgpu`/
//! `windows`, à l'exception du type `winit::event_loop::EventLoopProxy` utilisé par
//! [`remote_icons::RemoteIconStore::spawn`] — jamais une fenêtre ni une surface réelles.

use std::sync::mpsc;

use overlay_engine::{CatalogIndex, FightSnapshot, WatchlistEntry};

use crate::panels;
use crate::panels::combat::{CombatMetric, CombatSide};
use crate::panels::combat_frame::CombatFrame;
use crate::panels::options_modal::{OptionsModalAction, OptionsModalState};
use crate::panels::watchlist::{WatchlistAssets, WatchlistToast};
use crate::portraits::PortraitAtlas;
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::shortcuts::ShortcutBindings;
use crate::ui_icons::UiIcons;

/// Opacité de la fenêtre entière en mode CLIC-TRAVERSANT (voir `build_ui`) — seul indicateur de
/// mode restant depuis le retrait du texte d'état le 2026-09-01 (retour utilisateur 2026-09-02 :
/// « ça peut jouer sur une opacité à trente pour cent [...] pour indiquer [...] que le clic est
/// traversant [...] et que lors de la bascule, l'opacité redevient à un »). Valeur exacte demandée
/// par l'utilisateur, pas de raisonnement supplémentaire à documenter ici.
pub const CLICK_THROUGH_OPACITY: f32 = 0.3;

/// Espace réservé au-dessus du contenu du panneau Combat, pour que l'infobulle du switch Alliés/
/// Ennemis (`panels::combat::paint_side_switch`, tout premier widget peint dans ce panneau — voir
/// `show_tooltip_above`) puisse s'afficher AU-DESSUS de lui plutôt qu'en dessous (retour
/// utilisateur : « les tooltips du switch alliés/ennemis s'affichent en dessous au lieu d'au
/// dessus »). Avant ce correctif, `inner_margin` était nul sur les quatre côtés (voir `paint_
/// content` ci-dessous) : le switch était donc collé au bord SUPÉRIEUR de la fenêtre, sans
/// RIGOUREUSEMENT aucune place pour `RectAlign::TOP`, qui retombait systématiquement sur un repli
/// `BOTTOM*` (voir la doc de `show_tooltip_above`, déjà plusieurs refontes sur ce seul repli sans
/// jamais s'attaquer à la cause : l'absence de place elle-même).
///
/// Valeur choisie par observation du rendu offscreen (`overlay-testkit`, §17.1 du plan) : le popup
/// par défaut d'egui (`design::tooltip`, fond plein, `inner_margin` 8px) contenant une étiquette
/// courte ("Alliés"/"Ennemis") sur une seule ligne tient sur ~30px de haut, plus
/// `design::tokens::TOOLTIP_GAP` (5px) d'écart avec le widget — 44px laisse une marge confortable
/// au-dessus de ce total. Seul CE côté du panneau Combat gagne une marge (demande explicite : «
/// agrandis légèrement l'overlay ») : gauche/droite/bas restent collés au bord de la fenêtre de
/// jeu, décision non remise en cause ici (voir `main.rs::GAME_EDGE_MARGIN_PX`) — `main.rs`/`bin/
/// wakfu-companion-overlay-x11.rs` agrandissent `WINDOW_SIZE` de ce même montant pour que le reste
/// du panneau ne soit pas compressé d'autant.
pub const COMBAT_TOP_MARGIN: f32 = 44.0;

/// Espace réservé **sous** la bande du panneau Suivi, pour ses infobulles — voir
/// [`COMBAT_TOP_MARGIN`] pour le principe, ici retourné.
///
/// **Ce fut une marge HAUTE** (`WATCHLIST_TOP_MARGIN`, 36 px puis 28 px) du 2026-09-13 au soir du
/// même jour : les infobulles de « + »/« − » et des tuiles s'ouvraient au-dessus, il fallait leur
/// faire de la place là-haut, et cette place décalait la bande vers le BAS d'autant. Retour
/// utilisateur, deux captures à l'appui : « que la bande de suivi ne soit pas autant décalée par
/// rapport au haut de la fenêtre du jeu, c'est très dérangeant visuellement, et encore plus
/// lorsqu'il n'y a pas du tout de suivi — il y a quatre boutons qui flottent dans le vide, c'est
/// très perturbant ».
///
/// Le remède est venu avec la demande : « intervertir les choses [...] toutes les infobulles en
/// bas, comme celle du bouton Détails ; en collant la bande plus haut et en laissant juste
/// l'espace nécessaire — on gagnerait la moitié, peut-être plus, du vide qu'il y a aujourd'hui ».
/// Les quatre boutons du carré ouvrent donc sous le carré, les tuiles sous la bande (voir
/// `panels::watchlist::paint_tile_tips`), et cette réserve passe de la marge HAUTE du contenu à la
/// hauteur de fenêtre qui reste SOUS lui : elle ne décale plus rien, la bande touche le bord haut
/// de sa fenêtre.
///
/// Valeur inchangée, la mesure ne dépend pas du côté : une infobulle d'une ligne (« Supprimer
/// (Ctrl+Shift+S) ») occupe 27 px de haut, plus `design::tokens::TOOLTIP_GAP` (5 px) d'écart, soit
/// 32 px ; la marge basse de `paint_content` (6 px) en fournit déjà une partie, 28 px complètent
/// avec 2 px de garde.
///
/// L'écart qui reste entre le haut du client et la bande n'est plus que celui de l'ANCRAGE de la
/// fenêtre (`main.rs::GAME_TOP_MARGIN_PX`, 28 px sous le bord haut du client, calé sur les boutons
/// d'interface du jeu) plus les 6 px de marge interne — 34 px au lieu de 62.
pub const WATCHLIST_TOOLTIP_RESERVE: f32 = 28.0;

/// Espace réservé **au-dessus** du bloc Récap (`OverlayKind::Recap`) pour ses cinq infobulles,
/// qui s'ouvrent au-dessus de la case survolée (`TooltipSide::Above`, voir
/// `panels::recap::paint_cell`) — le même principe que [`COMBAT_TOP_MARGIN`] : sans place là-haut,
/// `RectAlign::TOP` retombe sur un repli en dessous.
///
/// **Ce fut une réserve BASSE** (28 px, le soir du 2026-09-16, infobulles `Below`) : le bloc est
/// posé sous les boutons du jeu, une infobulle ouverte au-dessus de sa première ligne recouvre
/// donc la zone où le jeu ouvre les siennes, et le Suivi venait de faire le chemin inverse. Retour
/// utilisateur le même soir : « toutes les infobulles au-dessus des éléments, à l'image de la
/// durée » — la durée, dernière ligne, retombait déjà au-dessus faute de place en dessous, et
/// c'est ce rendu-là qui a plu. La réserve change donc de côté ; c'est une marge HAUTE du contenu
/// (voir `paint_content`), et l'hôte remonte l'ancrage de la fenêtre d'autant
/// (`main.rs::GAME_RECAP_TOP_MARGIN_PX` reste l'ordonnée du BLOC, pas de la fenêtre) pour que le
/// bloc ne bouge pas d'un pixel.
///
/// Valeur relevée sur la capture `recap_tooltip_kamas_au_dessus` (`overlay-testkit`) : une
/// infobulle d'une ligne (« Kamas gagnés ») fait 33 px de haut, plus `design::tokens::TOOLTIP_GAP`
/// (5 px) d'écart avec la case, soit 38 px au-dessus de la première ligne ; `panels::recap::
/// PADDING_Y` (6 px) en fournit déjà une partie, 32 px complètent, et 4 px de garde font 36.
///
/// Un seul jeton partagé avec [`WATCHLIST_TOOLTIP_RESERVE`] aurait été tentant ; deux constantes
/// distinctes disent que ce sont deux panneaux dont les infobulles divergent (de côté, déjà).
pub const RECAP_TOOLTIP_RESERVE: f32 = 36.0;

/// Émis par le thread Engine (§3 du plan) ou le thread Auth (`spawn_auth_thread`) quand un nouvel
/// état est disponible — réveille le main thread, en `ControlFlow::Wait` le reste du temps (§6.1 :
/// pas de boucle 60 Hz forcée, l'overlay ne consomme rien tant que rien ne change). Publique : à
/// la fois `main.rs` (event loop winit) et `remote_icons::RemoteIconStore` (thread réseau partagé)
/// en ont besoin.
pub enum UserEvent {
    NewSnapshot,
    AuthStatusChanged,
    /// Un chargement initial vient de se terminer (voir `crate::startup::StartupProgress`) — la
    /// fenêtre de connexion doit réévaluer son écran de chargement.
    StartupProgress,
    /// Le thread de mise à jour a publié un nouvel `overlay_sync::update::UpdateStatus` (verdict,
    /// bloc téléchargé, fichier prêt, échec — voir `background::spawn_update_thread`) : la
    /// fenêtre de connexion et la fenêtre Options doivent redessiner ; l'hôte, lui, regarde à
    /// chaque tick si un exe est prêt à être installé.
    UpdateProgress,
}

/// Zone d'overlay indépendante ancrée sur une même fenêtre de jeu — demande utilisateur explicite
/// (2026-09-01) : Combat et Suivi doivent être deux fenêtres RÉELLEMENT séparées (pas seulement
/// deux panneaux dans la même fenêtre), pour permettre à terme de piloter leur visibilité
/// indépendamment (manuellement ou par un mécanisme automatique) — voir §9 du plan, qui vise à
/// terme autant de zones indépendantes que de panneaux. `main.rs::OverlayWindow` reste un seul
/// type partagé (position/topmost/redraw sont identiques pour les deux) : seul `kind` distingue la
/// taille, l'ancrage (`main.rs::App::anchor_position`) et le contenu rendu (`build_ui`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    Combat,
    Watchlist,
    /// Modale "Options" (2026-09-08, §9 du plan) — voir `panels::options_modal`. Contrairement à
    /// `Combat`/`Watchlist`, jamais créée automatiquement par `sync_windows` (une par fenêtre de
    /// jeu trouvée) : cette fenêtre OS est ouverte/fermée à la demande (clic sur le bouton
    /// "Options" du carré de contrôle, ou raccourci `Ctrl+Shift+O`), voir
    /// `main.rs::App::open_options_modal`/`bin/wakfu-companion-overlay-x11.rs` (même méthode
    /// dupliquée).
    ///
    /// **Rattachée à une fenêtre de jeu, elle la couvre entière** (2026-09-17, décision
    /// utilisateur) : un voile sur le jeu et ses overlays, la modale centrée dedans à sa taille
    /// habituelle — même forme que `RecapReset`, par le même composant (`design::scrim`). Ouverte
    /// SANS client à l'écran, elle reste une fenêtre à la taille de la modale, sans voile. C'est
    /// [`RenderContent::veiled`] qui distingue les deux au rendu.
    Options,
    /// **Récap de session** (2026-09-16, demande utilisateur) — la bande XP / Kamas / Combats /
    /// Challenges / Durée posée en haut à gauche de la fenêtre de jeu, sous les boutons
    /// d'interface du client. Voir `panels::recap`.
    ///
    /// Créée par `sync_windows` comme `Combat`/`Watchlist` (une par fenêtre de jeu trouvée), et
    /// masquée — jamais détruite — quand la case « Activer le récap de session » est décochée,
    /// même mécanique que le panneau Combat (`main.rs::App::sync_panel_visibility`).
    Recap,
    /// **Confirmation de remise à zéro du Récap** (2026-09-17) — la boîte du design system
    /// (`design::confirm_dialog`) sous un voile qui couvre TOUTE la fenêtre de jeu, overlays
    /// compris, centrée sur elle. Décision utilisateur : « impose une confirmBox centrée au jeu
    /// (verticalement et horizontalement) avec un fond voilé sur toute la fenêtre du jeu et des
    /// overlays (sauf cette confirmBox) ». Une fenêtre OS de la taille de la fenêtre de jeu,
    /// ouverte par l'hôte quand le glyphe du bloc est cliqué (`RenderOutcome::
    /// recap_reset_requested`), fermée à la réponse (`RenderOutcome::recap_reset_choice`) — le
    /// modèle est `Options` : à la demande, focalisable (Échap répond « Non »), toujours
    /// interactive.
    RecapReset,
    /// Fenêtre de connexion (2026-09-14, §9.1 undecies du plan) — voir `panels::login`. **La
    /// première interface de l'overlay**, et la seule tant que `AuthStatus` n'est pas `Connected` :
    /// une fenêtre logicielle classique (barre des tâches, focus, centrée sur l'écran), jamais un
    /// overlay ancré sur le jeu. `main.rs::App::sync_session_windows` la crée dès que le compte
    /// n'est pas lié et la retire dès qu'il l'est ; aucun `Combat`/`Watchlist` n'existe pendant
    /// qu'elle est affichée. Il n'y a pas de mode invité : un compte est obligatoire.
    Login,
}

/// État de la connexion au compte (lot L4, §7.2 du plan) — publié par le thread Auth
/// (`main.rs::spawn_auth_thread`) via `Arc<ArcSwap<_>>`, lu par le main thread à chaque frame.
/// Depuis le 2026-09-14 (§9.1 undecies), il pilote **quelles fenêtres existent** : la fenêtre de
/// connexion (`panels::login`) seule tant que la valeur n'est pas `Connected`, les overlays de jeu
/// seulement une fois qu'elle l'est — voir `main.rs::App::sync_session_windows`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthStatus {
    /// Tentative en cours qui n'attend rien de l'utilisateur : validation d'un jeton déjà stocké au
    /// démarrage, ou demande d'un code d'appairage juste après « Se connecter ». La fenêtre de
    /// connexion affiche « Connexion… » à la place de son bouton.
    Connecting,
    /// Code d'appairage obtenu (`POST /api/v1/auth/native/pair`), en attente que l'utilisateur le
    /// confirme sur `verification_url` — voir `overlay_sync::pair_and_wait`. Publié UNE FOIS par
    /// tentative, avant le premier sondage (`poll`), donc bien avant `Connected`/`Disconnected`.
    ///
    /// La fenêtre de connexion l'affiche en grand avec un bouton de copie, un bouton pour rouvrir
    /// la page (le navigateur s'ouvre déjà tout seul en best-effort, ce bouton couvre l'onglet
    /// fermé par erreur) et le compte à rebours jusqu'à `expires_at`.
    PairingStarted {
        pairing_code: String,
        verification_url: String,
        expires_at: std::time::Instant,
    },
    Connected,
    /// Aucun compte lié. `failure: None` est l'état **neutre** — premier lancement, déconnexion
    /// volontaire, appairage annulé, jeton refusé par le serveur — la fenêtre de connexion
    /// propose simplement « Se connecter ». `Some` est un **échec** de la dernière tentative
    /// (serveur injoignable, appairage expiré, réglages injoignables) : même fenêtre, en rouge,
    /// avec le détail technique et « Réessayer » — un clic qui ne se traduit par rien de visible
    /// est indiscernable d'un bouton cassé sans ce message (retour utilisateur 2026-09-01).
    Disconnected {
        failure: Option<AuthFailure>,
    },
}

impl AuthStatus {
    pub fn is_connected(&self) -> bool {
        matches!(self, AuthStatus::Connected)
    }
}

/// Échec d'une tentative de connexion, tel que la fenêtre de connexion l'affiche : un titre court
/// pour l'utilisateur (« Serveur injoignable ») et le détail technique en dessous (le message
/// d'erreur brut — jamais le jeton, §10 du plan).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthFailure {
    pub headline: String,
    pub detail: String,
}

/// Commande envoyée au thread Auth (`main.rs::spawn_auth_thread`) depuis le main thread.
///
/// - `Retry` — « Se connecter » ou « Réessayer » de la fenêtre de connexion : reprend une tentative
///   complète (jeton stocké si encore valide, sinon appairage). Sans effet si déjà connecté.
/// - `CancelPairing` — « Annuler l'appairage » : abandonne l'attente de confirmation en cours
///   (voir `overlay_sync::pair_and_wait`), retour à l'état neutre. Sans effet hors appairage.
/// - `Disconnect` — bouton « Déconnecter » de la fenêtre Options ou de l'icône de zone de
///   notification : efface le jeton, l'overlay revient à sa fenêtre de connexion. Sans effet si
///   pas connecté (mais vaut annulation pendant un appairage).
///
/// Un seul thread Auth, un seul point d'attente (`command_rx.recv()`) : toutes les origines
/// convergent sur ce même type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthCommand {
    Retry,
    CancelPairing,
    Disconnect,
}

/// Puits pour les commandes émises par `build_ui` vers le thread Auth — abstraction minimale
/// introduite pour que [`RenderContent`] (et donc `build_ui`) soit constructible SANS canal `mpsc`
/// réel (§17.1 du plan, réserve de la revue à trois experts sur la constructibilité de
/// `RenderContent` pour un futur harnais de test). `mpsc::Sender<AuthCommand>` l'implémente pour
/// la production (voir plus bas) ; un harnais de test passe un puits sans effet.
pub trait AuthCommandSink {
    fn send(&self, command: AuthCommand);
}

impl AuthCommandSink for mpsc::Sender<AuthCommand> {
    fn send(&self, command: AuthCommand) {
        // Même comportement qu'avant l'introduction du trait (`let _ = auth_command_tx.send(...)`
        // dans `build_ui`) : un échec d'envoi (thread Auth arrêté) n'est jamais fatal au rendu.
        let _ = mpsc::Sender::send(self, command);
    }
}

/// Puits sans effet — pour un harnais de test qui construit `RenderContent` sans thread Auth réel
/// (§17.1 du plan) : un clic sur l'icône de relance ne fait rien d'observable, ce qui est
/// exactement le comportement voulu hors production.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopAuthSink;

impl AuthCommandSink for NoopAuthSink {
    fn send(&self, _command: AuthCommand) {}
}

/// Regroupe les paramètres de `build_ui` au-delà de `ctx`/`raw_input` — sinon `too_many_arguments`
/// (clippy), la fonction ayant crû à mesure que le panneau Combat (icônes, camp affiché) et
/// l'icône de relance d'appairage (statut de connexion, canal de retentative) s'y sont ajoutés.
pub struct RenderContent<'a> {
    pub kind: OverlayKind,
    pub fight: Option<&'a FightSnapshot>,
    pub portraits: &'a PortraitAtlas,
    pub combat_frame: &'a CombatFrame,
    pub icons: &'a UiIcons,
    /// Les bustes de classe de l'onglet « Personnages » — `Some` pour la seule fenêtre Options,
    /// qui est la seule à les charger (voir `crate::avatars`, doc de module).
    pub avatars: Option<&'a crate::avatars::AvatarAtlas>,
    /// Les serveurs de jeu (`crate::game_servers`), pour le sélecteur de compte du même onglet.
    pub game_servers: &'a crate::game_servers::GameServers,
    pub combat_side: &'a mut CombatSide,
    /// Grandeur mesurée par le panneau Combat (dégâts / armure donnée / soins) — même statut que
    /// `combat_side` : un état par fenêtre overlay, porté par l'hôte (voir `CombatMetric`).
    pub combat_metric: &'a mut CombatMetric,
    pub watchlist: &'a [WatchlistEntry],
    /// État de la case « Activer le suivi » (`panels::feature_switch`) — `false` retire les boutons
    /// « + » et « − » du bandeau (retour utilisateur 2026-09-15, voir
    /// `panels::watchlist::control_button_row`). Distinct de `watchlist.is_empty()`, que l'hôte
    /// force déjà dans ce cas : un bandeau vide Suivi ACTIF garde bien ses quatre boutons.
    pub watchlist_enabled: bool,
    /// Le suivi des sorts est-il actif ? — case « Activer le suivi des sorts » de la section
    /// « Combat » des Options (2026-09-15). `false` retire le bloc « ligne de sorts » du panneau
    /// Combat et les marques qu'il pose sur les médaillons (voir `panels::combat::show`).
    ///
    /// **Déjà combiné avec la case dont il dépend** par l'hôte
    /// (`panels::feature_switch::FeatureToggles::spells_visible`) : le panneau reçoit ce qu'il
    /// doit peindre, pas deux booléens à recroiser. Le détail des combats coupé, lui, ne se voit
    /// pas ici — la fenêtre Combat n'est alors pas montrée du tout
    /// (`panels::combat::should_show`).
    pub spells_enabled: bool,
    /// **Le panneau Combat est-il posé à droite de la fenêtre de jeu ?** — case « Afficher le
    /// panneau de combat à droite de la fenêtre de jeu » de la section « Combat » des Options
    /// (2026-09-17, `config::OverlayConfig::combat_on_right`).
    ///
    /// `true` retourne TOUT le contenu de cette fenêtre en miroir vertical (voir [`crate::mirror`],
    /// qui explique pourquoi c'est un miroir de rendu et pas une mise en page paramétrée) ; l'hôte,
    /// lui, ancre la fenêtre au bord droit du client (`main.rs::App::anchor_position`). Sans objet
    /// pour les autres zones : seule la fenêtre Combat change de côté.
    pub combat_on_right: bool,
    /// Sélection multiple du bandeau (2026-09-13) — l'état vit chez l'hôte, qui seul reçoit le
    /// raccourci global `Ctrl+Shift+S` : voir `panels::watchlist::WatchlistSelection`.
    pub watchlist_selection: &'a mut panels::watchlist::WatchlistSelection,
    pub watchlist_toast: Option<&'a WatchlistToast>,
    pub catalog: &'a CatalogIndex,
    pub catalog_stale: bool,
    pub remote_icons: &'a RemoteIconStore,
    pub remote_icon_textures: &'a mut RemoteIconTextures,
    pub auth_status: &'a AuthStatus,
    pub auth_command_tx: &'a dyn AuthCommandSink,
    /// `true` en mode INTERACTIF (clics capturés), `false` en CLIC-TRAVERSANT (voir
    /// `main.rs::App::toggle_interactive`) — pilote l'opacité de la fenêtre entière (voir
    /// `build_ui`), seul indicateur de mode conservé (demande explicite de l'utilisateur
    /// 2026-09-02, en remplacement du texte/icône d'état retiré le 2026-09-01 — voir la doc de
    /// `build_ui`).
    pub interactive: bool,
    /// Raccourcis clavier EFFECTIFS (personnalisables, voir `crate::shortcuts`) — propagés jusqu'aux
    /// infobulles des boutons du carré de contrôle (`panels::watchlist`) et du switch
    /// Alliés/Ennemis (`panels::combat`), qui affichaient auparavant des combinaisons codées en
    /// dur. Passé à CHAQUE frame et jamais mémorisé par les panneaux : une validation de la fenêtre
    /// Options change les raccourcis en cours de session, sans redémarrage.
    pub shortcuts: &'a ShortcutBindings,
    /// Instant de référence pour CETTE frame — calculé UNE FOIS par `main.rs::window_event`
    /// (`WindowEvent::RedrawRequested`) et propagé jusqu'à `panels::watchlist::is_active`/`show`/
    /// `toast_card` (horloge injectable, §17.1 du plan) plutôt que lu à nouveau à chaque étage via
    /// `Instant::now()` : un même rendu doit utiliser une seule référence de temps cohérente, et
    /// cette même valeur devient reproductible pour un harnais de rendu offscreen qui la fige.
    pub now: std::time::Instant,
    /// État de la modale Options (2026-09-08, §9 du plan) — `Some` UNIQUEMENT pour
    /// `kind == OverlayKind::Options` (voir `paint_content`) ; `None` pour Combat/Watchlist ET pour
    /// tout appelant (`overlay-testkit`) qui n'exerce pas encore ce panneau. `&mut` : la frappe
    /// dans le champ de chemin (`egui::TextEdit`) doit persister d'une frame à l'autre, voir
    /// `panels::options_modal::OptionsModalState`.
    /// Totaux de la session tels que le moteur les tient (`overlay_engine::SessionTotals`) — ce
    /// Les cases facultatives de la bande Récap — durée, combats, challenges (cases « Afficher
    /// … » de la section « Recap » des Options, 2026-09-16). Voir `panels::recap::RecapCells`.
    /// L'interrupteur de la bande lui-même ne se voit pas ici : la fenêtre Récap n'est alors pas
    /// montrée du tout (même règle que le détail des combats).
    pub recap_cells: panels::recap::RecapCells,
    /// La vue de la session (`crate::recap_session::RecapSession`, 2026-09-17) : compteurs et
    /// chrono de LA SESSION — pas ceux du fichier relu ni du processus —, heure de début et
    /// nombre de reprises. Construite par l'hôte à chaque frame, ce qui la rend figeable par un
    /// harnais de test, comme `now`.
    pub recap: &'a panels::recap::RecapView,
    pub options: Option<&'a mut OptionsModalState>,
    /// **La fenêtre Options couvre la fenêtre de jeu entière** (2026-09-17, décision utilisateur :
    /// « un voile qui recouvre toute la fenêtre du jeu et les overlays lorsque l'utilisateur ouvre
    /// la modale d'options alors que la fenêtre de jeu est ouverte ») : `paint_content` voile
    /// alors toute la fenêtre (`design::scrim`) et centre la modale dedans, à sa taille
    /// habituelle (`panels::options_modal::WINDOW_SIZE`). `false` quand elle est ouverte SANS
    /// client Wakfu à l'écran (`main.rs::OverlayWindow::is_detached`) : la fenêtre OS est alors
    /// à la taille de la modale, et il n'y a rien à voiler — « si l'utilisateur ouvre la modale
    /// d'options sans le jeu, aucun voile ne doit être appliqué ». Sans objet pour les autres
    /// zones ; la confirmation de remise à zéro (`RecapReset`) voile toujours, c'est sa raison
    /// d'être.
    pub veiled: bool,
    /// État de la fenêtre de connexion (2026-09-14) — `Some` UNIQUEMENT pour
    /// `kind == OverlayKind::Login`, même règle que `options` ; `&mut` pour la même raison
    /// (l'horloge de l'anneau animé et le survol vivent d'une frame à l'autre).
    pub login: Option<&'a mut panels::login::LoginState>,
}

/// Ce qu'une frame de rendu a produit, au-delà de l'affichage lui-même — étend l'ancien simple
/// `bool` (`close_toast` seul) depuis le 2026-09-08 : la modale Options ajoute deux autres signaux
/// que `paint_content` doit remonter à l'appelant réel
/// (`main.rs`/`bin/wakfu-companion-overlay-x11.rs`, seuls capables de créer une fenêtre OS ou de
/// lancer un dialogue de fichier natif) SANS que `paint_content`/`build_ui` eux-mêmes en dépendent
/// — voir la doc de tête du module pour pourquoi ces deux fonctions restent PURES.
#[derive(Debug, Default)]
pub struct RenderOutcome {
    /// Voir la doc historique de `build_ui` : fermeture du toast de suivi (clic sur la carte ou sa
    /// croix, `panels::watchlist::toast_card`).
    pub close_toast: bool,
    /// Auteur à qui répondre en privé — carte d'alerte de chat cliquée CETTE frame (`kind ==
    /// Watchlist`). L'hôte ferme la carte et tape `/w "<auteur>" ` dans le jeu
    /// (`chat_command::send_whisper`) ; voir `panels::watchlist::WatchlistOutcome::whisper_to`.
    pub whisper_to: Option<String>,
    /// `true` UNIQUEMENT à la frame où le bouton "+" du carré de contrôle
    /// (`panels::watchlist::control_button_row`) vient d'être cliqué — `kind == Watchlist`
    /// seulement. L'appelant ouvre la modale Options sur l'onglet « Suivi » — voir
    /// `panels::watchlist::WatchlistOutcome::open_watchlist`.
    pub open_watchlist: bool,
    /// `true` UNIQUEMENT à la frame où le bouton "Options" du carré de contrôle
    /// (`panels::watchlist::control_button_row`) vient d'être cliqué — `kind == Watchlist`
    /// seulement, jamais émis par Combat/Options eux-mêmes. L'appelant ouvre la modale Options sur
    /// l'onglet « Paramètres ».
    pub open_options: bool,
    /// Action déclenchée CETTE frame par la modale Options elle-même (`kind == Options`
    /// seulement) — voir `panels::options_modal::OptionsModalAction`.
    pub options_action: OptionsModalAction,
    /// Définitions de suivi restantes après une suppression groupée demandée CETTE frame depuis le
    /// bandeau — `None` le reste du temps. L'hôte les envoie au moteur
    /// (`EngineCommand::SetWatchlistDefinitions`), par le même chemin que la validation de l'onglet
    /// « Suivi » : voir `panels::watchlist::WatchlistOutcome::edit`.
    pub watchlist_edit: Option<panels::watchlist::WatchlistEdit>,
    /// URL que l'hôte doit ouvrir dans le navigateur, le cas échéant : clic sur "Détails" du
    /// panneau Suivi (la web app) ou sur "Ouvrir la page" de la carte d'appairage (l'URL de
    /// vérification).
    ///
    /// **Aucun panneau n'appelle `open::that` lui-même**, et ce n'est pas un détail de style : les
    /// captures de non-régression cliquent réellement sur ces boutons, donc un panneau qui ouvre
    /// une page ouvre le navigateur de la personne qui lance `cargo test`. C'est arrivé, plusieurs
    /// fois dans la même journée (signalé le 2026-09-09). Faire remonter l'intention rend les
    /// panneaux inertes par construction, sans qu'aucun test n'ait à s'en préoccuper.
    pub open_url: Option<String>,
    /// La fenêtre de connexion demande à être déplacée à la souris (appui sur sa bannière, voir
    /// `panels::login`) — sans décorations OS, c'est l'hôte qui appelle `Window::drag_window`.
    pub drag_window: bool,
    /// « Réessayer » de l'écran « Mise à jour requise » de la fenêtre de connexion — l'hôte
    /// relance la vérification avec installation (voir `panels::login::LoginOutcome`).
    pub retry_update: bool,
    /// Hauteur que le bloc Récap vient d'occuper (`kind == Recap`), pour que l'hôte ajuste sa
    /// fenêtre OS au contenu — même mécanique que [`Self::login_height`], et pour la même raison
    /// qu'elle : une fenêtre plus haute que son bloc capterait les clics sur du vide en mode
    /// interactif. La largeur, elle, est fixe (`panels::recap::WIDTH`) ; c'est la hauteur qui
    /// bouge, quand une ligne trop large s'empile. Voir `panels::recap::show`.
    pub recap_height: Option<f32>,
    /// Le glyphe de remise à zéro du bloc Récap vient d'être cliqué (`kind == Recap`) : l'hôte
    /// ouvre la fenêtre de confirmation (`OverlayKind::RecapReset`). Voir `panels::recap`.
    pub recap_reset_requested: bool,
    /// Ce que la fenêtre de confirmation vient d'obtenir (`kind == RecapReset`) : `Yes` remet la
    /// session à zéro et ferme la fenêtre, `No` la ferme seulement, `Pending` la garde.
    pub recap_reset_choice: crate::design::ConfirmChoice,
    /// Hauteur de contenu que la fenêtre de connexion vient de mesurer (`kind == Login`), pour que
    /// l'hôte ajuste la fenêtre OS à l'état affiché — voir `panels::login::show`.
    pub login_height: Option<f32>,
}

/// **Refonte 2026-09-01** (retour utilisateur, capture d'écran à l'appui) : le nom du personnage,
/// l'état interactif/clic-traversant EN TEXTE et le rappel du raccourci n'apportaient rien
/// (l'utilisateur sait déjà quel personnage est le sien et derrière quelle fenêtre de jeu il
/// joue) — retirés, de
/// même que le titre "Dégâts du combat" (voir `panels::combat`) et toute la section "Récap de
/// session" (Kamas/XP/Combats/Butin — retour utilisateur : la garder n'a plus de sens une fois le
/// reste simplifié, sera repensée dans un autre chantier). Le fond opaque du panneau (une grande
/// plaque sombre visible même quand il n'y a presque rien à afficher, voir la capture) est
/// également retiré : `Frame::NONE`, seuls les widgets eux-mêmes restent visibles par-dessus le
/// jeu.
///
/// **Indicateur de mode par opacité, 2026-09-02** (retour utilisateur, vidéo à l'appui : clics sur
/// le switch Alliés/Ennemis sans effet visible, sans moyen de savoir si l'overlay était alors en
/// CLIC-TRAVERSANT — le texte d'état ci-dessus avait justement été retiré la veille comme
/// n'apportant rien) : plutôt que de réintroduire ce texte, toute la fenêtre passe à
/// `CLICK_THROUGH_OPACITY` (30 %, proposition explicite de l'utilisateur) en CLIC-TRAVERSANT et
/// revient à pleine opacité en INTERACTIF — un simple coup d'œil suffit alors à savoir si un clic
/// va être capté, sans texte à lire. `Ui::set_opacity` appliqué en tout premier, avant tout
/// widget : les enfants créés ensuite héritent de l'opacité du `Painter` au moment de leur
/// création (voir `egui::Painter::{set,multiply}_opacity`).
///
/// **Extrait de `main.rs::render` en fonction séparée (2026-09-03, §17.1 du plan)** : cette
/// fonction est PURE — elle ne touche ni `Window`, ni `egui_winit`, ni `wgpu::Surface`, seulement
/// `egui::Context`/`egui::RawInput`/`egui::FullOutput`. `main.rs::render` reste seule responsable
/// du fenêtrage/GPU ; le crate `overlay-testkit` (même section du plan) appelle exactement cette
/// fonction sans fenêtre système ni GPU physique. Renvoie le `FullOutput` produit et si CETTE
/// frame doit effacer le toast affiché (clic sur la carte/la croix, voir
/// `panels::watchlist::toast_card`) : seul l'appelant final (`main.rs::window_event`, via
/// `main.rs::render`) détient un accès en écriture à l'`ArcSwap` correspondant.
pub fn build_ui(
    ctx: &egui::Context,
    mut raw_input: egui::RawInput,
    mut content: RenderContent<'_>,
) -> (egui::FullOutput, RenderOutcome) {
    let mut outcome = RenderOutcome::default();
    // **Panneau Combat posé à droite** (2026-09-17) : le pointeur réel est à droite, egui met en
    // page à gauche — les événements sont donc traduits AVANT d'entrer, pendant que les formes
    // peintes sont réfléchies à la sortie (voir `crate::mirror` et la fin de `paint_content`).
    // Survol, clic, glisser et placement des infobulles tombent juste sans qu'aucun panneau ne
    // sache qu'il est affiché en miroir.
    if content.kind == OverlayKind::Combat && content.combat_on_right {
        let axis_x = crate::mirror::axis_of_input(ctx, &raw_input);
        crate::mirror::mirror_input(&mut raw_input, axis_x);
    }
    // Reconstruit un `RenderContent` FRAIS à chaque appel de la fermeture plutôt que de déplacer
    // `content` (capturé par la fermeture) directement dans `paint_content` : `ctx.run_ui` exige
    // `FnMut`, et déplacer un agrégat non-`Copy` hors de l'environnement capturé d'une fermeture ne
    // peut typer qu'en `FnOnce`. La fermeture ne capture `content` que PAR RÉFÉRENCE PARTAGÉE
    // (jamais déplacé, jamais muté lui-même) : lire un champ `&mut` déjà stocké dedans pour le
    // ré-emprunter (`&mut *content.xxx`, réemprunt explicite plutôt qu'implicite pour lever toute
    // ambiguïté) mute la RÉFÉRENCE POINTÉE, pas le conteneur qui la stocke — ça reste valide
    // `FnMut` sans même avoir besoin de `mut content` en paramètre (vérifié par le compilateur).
    // C'est le seul rôle de ce petit bloc de recopie ; `paint_content` (ci-dessous) porte toute la
    // vraie logique et reste appelable directement par un futur harnais de rendu offscreen
    // (`overlay-testkit`, §17.1 du plan) sans jamais passer par `build_ui`/`ctx.run_ui`.
    //
    // `content.options` (`Option<&mut OptionsModalState>`, 2026-09-08) demande `mut content` en
    // paramètre (contrairement au reste ci-dessus) : `Option<&mut _>::as_deref_mut` — l'équivalent
    // du même ré-emprunt pour un champ optionnel — a besoin d'un accès `&mut self`.
    let full_output = ctx.run_ui(raw_input, |ui| {
        outcome = paint_content(
            ui,
            RenderContent {
                kind: content.kind,
                fight: content.fight,
                portraits: content.portraits,
                combat_frame: content.combat_frame,
                icons: content.icons,
                avatars: content.avatars,
                game_servers: content.game_servers,
                combat_side: &mut *content.combat_side,
                combat_metric: &mut *content.combat_metric,
                watchlist: content.watchlist,
                watchlist_enabled: content.watchlist_enabled,
                spells_enabled: content.spells_enabled,
                combat_on_right: content.combat_on_right,
                watchlist_selection: &mut *content.watchlist_selection,
                watchlist_toast: content.watchlist_toast,
                catalog: content.catalog,
                catalog_stale: content.catalog_stale,
                remote_icons: content.remote_icons,
                remote_icon_textures: &mut *content.remote_icon_textures,
                auth_status: content.auth_status,
                auth_command_tx: content.auth_command_tx,
                interactive: content.interactive,
                shortcuts: content.shortcuts,
                now: content.now,
                recap_cells: content.recap_cells,
                recap: content.recap,
                options: content.options.as_deref_mut(),
                veiled: content.veiled,
                login: content.login.as_deref_mut(),
            },
        );
    });
    (full_output, outcome)
}

/// Peint le contenu d'UN panneau (Combat ou Suivi) dans `ui` à partir de `content` — extrait de
/// `build_ui` (voir sa doc) pour être appelable directement par un harnais de rendu offscreen
/// (`overlay-testkit`, §17.1 du plan) SANS passer par `egui::Context::run_ui`/`egui::RawInput` :
/// un tel harnais (`egui_kittest::Harness::new_ui`) fournit déjà son propre `&mut egui::Ui`, il n'a
/// besoin que de CETTE fonction, jamais de `build_ui` en entier. `content` est pris PAR VALEUR
/// (jamais capturé par une fermeture englobante) : chaque appelant reconstruit un `RenderContent`
/// frais à chaque invocation (voir `build_ui` ci-dessus et la doc du harnais) plutôt que de tenter
/// de réutiliser un agrégat capturé, ce qui éviterait toute ambiguïté de capture de fermeture.
pub fn paint_content(ui: &mut egui::Ui, content: RenderContent<'_>) -> RenderOutcome {
    let RenderContent {
        kind,
        fight,
        portraits,
        avatars,
        game_servers,
        combat_frame,
        icons,
        combat_side,
        combat_metric,
        watchlist,
        watchlist_enabled,
        spells_enabled,
        combat_on_right,
        watchlist_selection,
        watchlist_toast,
        catalog,
        catalog_stale,
        remote_icons,
        remote_icon_textures,
        auth_status,
        auth_command_tx,
        interactive,
        shortcuts,
        now,
        recap_cells,
        recap,
        options,
        veiled,
        login,
    } = content;

    let mut outcome = RenderOutcome::default();
    // Marge interne nulle pour Combat sur trois côtés (refonte 2026-09-04, retour utilisateur :
    // collé au bord de la fenêtre de jeu, sans le moindre vide, pour simuler une interface qui
    // ferait partie du jeu — voir aussi `main.rs::GAME_EDGE_MARGIN_PX`, ramené à 0 pour la même
    // raison) — SEUL le haut gagne `COMBAT_TOP_MARGIN`, voir sa doc, pour que l'infobulle du switch
    // Alliés/Ennemis ait la place de s'afficher au-dessus de lui. Suivi : voir le bloc ci-dessous.
    // Options
    // (2026-09-08) est une fenêtre dédiée qui remplit tout son espace elle-même (voir
    // `panels::options_modal::show`, bannière/corps/pied de page peints jusqu'aux bords) : aucune
    // marge, comme Combat.
    let inner_margin = match kind {
        OverlayKind::Combat => egui::Margin {
            left: 0,
            right: 0,
            top: COMBAT_TOP_MARGIN as i8,
            bottom: 0,
        },
        // Suivi : marge GAUCHE nulle depuis le 2026-09-13 (retour utilisateur : « il faut que
        // l'overlay démarre au début du premier bouton au niveau gauche, à partir du premier pixel
        // du dessin des boutons ») — le carré de contrôle est le premier élément peint, son fond
        // translucide touche donc le bord de la fenêtre. La réserve d'infobulle de 48 px qui
        // s'ajoutait devant est tombée dans le même mouvement (voir
        // `panels::watchlist::ControlLayout::tooltip_reserve`).
        //
        // **Marge HAUTE nulle elle aussi** depuis le soir du même jour, et pour la même raison
        // poussée d'un cran : les infobulles s'ouvrent toutes en dessous (leur réserve est passée
        // sous la bande, voir [`WATCHLIST_TOOLTIP_RESERVE`]), et la barre de défilement est passée
        // AU-DESSUS des tuiles (`panels::watchlist::strip_scroll_area`) — « il faut juste mettre
        // la marge nécessaire pour afficher la scrollbar au-dessus [...] laisser un ou deux pixels
        // au-dessus de la barre de scroll seulement ». Ces deux pixels-là appartiennent à la
        // bande (`panels::watchlist::STRIP_SCROLLBAR_OUTER_MARGIN`), pas au panneau : quand la
        // bande tient entière, il n'y a pas de barre, donc rien à dégager.
        OverlayKind::Watchlist => egui::Margin {
            left: 0,
            right: 6,
            top: 0,
            bottom: 6,
        },
        OverlayKind::Options => egui::Margin::ZERO,
        // La confirmation couvre sa fenêtre entière : le voile va bord à bord.
        OverlayKind::RecapReset => egui::Margin::ZERO,
        // Récap : calé à gauche de sa fenêtre, et SEUL le haut gagne une marge, celle des
        // infobulles ([`RECAP_TOOLTIP_RESERVE`], même principe que `COMBAT_TOP_MARGIN`) — le bloc
        // peint son propre fond et se place lui-même sous cette marge (voir `panels::recap::show`).
        // Cette marge ne décale PAS le bloc dans le jeu : l'hôte ancre la fenêtre d'autant plus
        // haut (voir `main.rs::anchor_position`), le bloc reste à `GAME_RECAP_TOP_MARGIN_PX`.
        OverlayKind::Recap => egui::Margin {
            left: 0,
            right: 0,
            top: RECAP_TOOLTIP_RESERVE as i8,
            bottom: 0,
        },
        // La fenêtre de connexion peint sa carte jusqu'aux bords de sa fenêtre OS (fond
        // translucide, anneau animé sur le pourtour) — voir `panels::login`.
        OverlayKind::Login => egui::Margin::ZERO,
    };
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.inner_margin(inner_margin))
        .show(ui, |ui| {
            // Voir la doc de `build_ui` : seul indicateur de mode restant, en tout premier
            // avant le moindre widget pour que tout hérite de cette opacité.
            ui.set_opacity(if interactive {
                1.0
            } else {
                CLICK_THROUGH_OPACITY
            });
            match kind {
                // Zone Combat : dégâts du combat en cours.
                OverlayKind::Combat => {
                    // **Plus aucun indicateur de compte ici depuis le 2026-09-14** : la carte
                    // d'appairage, l'icône de relance et « Connexion… » qui vivaient dans cette
                    // zone ont été remplacés par la fenêtre de connexion (`OverlayKind::Login`,
                    // `panels::login`). Cette fenêtre-ci n'existe plus que compte lié — voir
                    // `main.rs::App::sync_session_windows`.
                    // Indicateur « catalogue daté » (§7.4/§9 du plan, lot L3) — visible UNIQUEMENT
                    // quand `catalog` provient du repli hors-ligne embarqué (ni cache disque ni
                    // réseau au démarrage, voir `spawn_catalog_thread`) : les icônes/classements
                    // affichés peuvent alors dater du dernier build de l'overlay plutôt que du vrai
                    // catalogue serveur. Avant ce lot, seul un `tracing::warn!` signalait ce cas —
                    // invisible pour qui ne regarde pas les logs en jouant.
                    if catalog_stale {
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    // `combat::show_tooltip_above` plutôt qu'un `on_hover_text`
                                    // brut (refonte 2026-09-06, design system tooltip) — voir sa
                                    // doc.
                                    let warning = ui.label("📦⚠");
                                    crate::design::tooltip(&warning).text(
                                        "Catalogue hors ligne : réseau et cache local tous deux \
                                         indisponibles au démarrage, repli sur la base embarquée \
                                         dans l'overlay (peut être datée).",
                                    );
                                },
                            );
                        });
                        ui.add_space(4.0);
                    }

                    panels::combat::show(
                        ui,
                        fight,
                        portraits,
                        combat_frame,
                        icons,
                        catalog,
                        remote_icons,
                        remote_icon_textures,
                        combat_side,
                        combat_metric,
                        shortcuts,
                        spells_enabled,
                    );
                }
                // Zone Suivi — fenêtre INDÉPENDANTE de Combat (demande utilisateur explicite
                // 2026-09-01) : bande de tuiles façon `tracker-strip` du web, voir
                // `panels::watchlist`.
                //
                // **Refonte 2026-09-08** : `panels::watchlist::show` est désormais appelée
                // INCONDITIONNELLEMENT (l'ancienne garde `!watchlist.is_empty() || is_active(...)`
                // est retirée) — le carré de contrôle "+"/"−"/"Options"/"Détails" qu'elle peint doit
                // rester atteignable même sans aucune entrée suivie, "Options"/"Détails" ayant
                // rejoint ce carré depuis `combat::bottom_toolbar` (voir doc de
                // `panels::watchlist`, refonte du même jour) : le panneau Combat n'étant pas
                // toujours affiché, ces deux boutons ont besoin d'un emplacement permanent. Les
                // tuiles d'entrées, elles, restent absentes tant que `watchlist` est vide — c'est
                // `panels::watchlist::show` elle-même qui fait cette distinction en interne.
                OverlayKind::Watchlist => {
                    let watchlist_outcome = panels::watchlist::show(
                        ui,
                        WatchlistAssets {
                            shortcuts,
                            icons,
                            catalog,
                            remote_icons,
                            remote_icon_textures,
                        },
                        watchlist,
                        watchlist_enabled,
                        watchlist_selection,
                        watchlist_toast,
                        now,
                    );
                    outcome.close_toast = watchlist_outcome.close_toast;
                    outcome.whisper_to = watchlist_outcome.whisper_to;
                    outcome.open_watchlist = watchlist_outcome.open_watchlist;
                    outcome.open_options = watchlist_outcome.open_options;
                    outcome.watchlist_edit = watchlist_outcome.edit;
                    if watchlist_outcome.open_web_app {
                        outcome.open_url = Some(overlay_sync::client::base_url().to_string());
                    }
                }
                // Modale Options (2026-09-08, §9 du plan) — voir `panels::options_modal`. `options`
                // est `Some` uniquement pour ce `kind` (voir la doc de `RenderContent::options`) ;
                // un appelant qui créerait une fenêtre `OverlayKind::Options` sans fournir cet état
                // ne verrait simplement rien peint ici plutôt que de paniquer — jamais souhaitable
                // en pratique (voir `main.rs`/`bin/wakfu-companion-overlay-x11.rs`, qui le
                // fournissent toujours pour ce cas), mais plus sûr qu'un `expect` sur un chemin de
                // rendu. Fenêtre de connexion (2026-09-14) — voir `panels::login`. `login` est
                // `Some` uniquement pour ce `kind` (même règle et même repli silencieux que
                // `options`).
                OverlayKind::Login => {
                    if let Some(state) = login {
                        let login_outcome = panels::login::show(
                            ui,
                            state,
                            icons,
                            auth_status,
                            auth_command_tx,
                            now,
                        );
                        outcome.open_url = login_outcome.open_url;
                        outcome.drag_window = login_outcome.drag_window;
                        outcome.retry_update = login_outcome.retry_update;
                        outcome.login_height = Some(login_outcome.content_height);
                    }
                }
                // Récap de session (2026-09-16) — voir `panels::recap`. Deux choses remontent :
                // la hauteur qu'il vient d'occuper, dont l'hôte se sert pour redimensionner la
                // fenêtre OS, et le clic sur son glyphe de remise à zéro (2026-09-17).
                OverlayKind::Recap => {
                    let recap_outcome = panels::recap::show(ui, recap, recap_cells);
                    outcome.recap_height = Some(recap_outcome.height);
                    outcome.recap_reset_requested = recap_outcome.reset_requested;
                }
                // La confirmation de remise à zéro (2026-09-17) — voir `OverlayKind::RecapReset`.
                // `over(max_rect)` : le voile couvre la fenêtre entière, qui est celle du jeu.
                OverlayKind::RecapReset => {
                    outcome.recap_reset_choice =
                        crate::design::confirm_dialog("Remettre le récap de session à zéro ?")
                            .over(ui.max_rect())
                            .log_name("recap.remise-a-zero")
                            .show(ui);
                }
                OverlayKind::Options => {
                    if let Some(state) = options {
                        // Les mêmes dépendances que le bandeau Suivi : depuis le 2026-09-12,
                        // l'onglet « Alertes » liste de vrais objets, avec leur rareté lue au
                        // catalogue et leur icône descendue du même CDN.
                        let mut context = panels::options_modal::OptionsModalContext {
                            catalog,
                            remote_icons,
                            remote_icon_textures,
                            icons,
                            avatars,
                            game_servers,
                        };
                        outcome.options_action = if veiled {
                            // La fenêtre est celle du jeu (voir `RenderContent::veiled`) : voile
                            // bord à bord, modale centrée dedans à sa taille de toujours.
                            // `ScrimLayer::Current` : le voile est le FOND de cette fenêtre,
                            // rien n'est peint dessous — et les sélecteurs de la modale, en
                            // `Foreground`, restent au-dessus d'elle.
                            let (w, h) = panels::options_modal::WINDOW_SIZE;
                            crate::design::scrim(ui.max_rect())
                                .layer(crate::design::ScrimLayer::Current)
                                .centered(egui::vec2(w, h))
                                .log_name("options.voile")
                                .show(ui, |ui| {
                                    panels::options_modal::show(ui, state, &mut context)
                                })
                                .inner
                        } else {
                            panels::options_modal::show(ui, state, &mut context)
                        };
                    }
                }
            }
        });
    // Curseur du jeu à la place du curseur système (voir `crate::cursor`) — APRÈS tout le contenu,
    // une fois que chaque widget survolé a dit ce qu'il voulait (`PlatformOutput::cursor_icon`).
    crate::cursor::apply(ui.ctx(), now);
    // **Panneau Combat posé à droite** (2026-09-17) : le contenu vient d'être peint comme
    // d'habitude, dans le repère « à gauche » ; il ne reste qu'à réfléchir les formes produites
    // autour du centre de la fenêtre — voir `crate::mirror`, qui porte la décision et son
    // pourquoi. Tout DERNIER geste de la frame, infobulles comprises : une forme peinte après
    // resterait à l'endroit.
    //
    // L'ENTRÉE fait le chemin inverse, en amont (`build_ui`) : le clic réel, à droite, est traduit
    // en coordonnées de mise en page avant qu'egui ne le voie. C'est ce qui permet à tous les
    // panneaux d'ignorer complètement ce réglage.
    if kind == OverlayKind::Combat && combat_on_right {
        let ctx = ui.ctx();
        crate::mirror::mirror_painted(ctx, crate::mirror::axis_of(ctx));
    }
    outcome
}
