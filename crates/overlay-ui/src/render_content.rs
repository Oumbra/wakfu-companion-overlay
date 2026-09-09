//! Construction de l'interface egui à partir d'un snapshot — extrait de `main.rs::render` le
//! 2026-09-03 (§17.1 du plan) pour que cette logique soit appelable sans fenêtre système ni GPU
//! physique (voir la doc de [`build_ui`]). Tout ce module est indépendant de `winit`/`wgpu`/
//! `windows`, à l'exception du type `winit::event_loop::EventLoopProxy` utilisé par
//! [`remote_icons::RemoteIconStore::spawn`] — jamais une fenêtre ni une surface réelles.

use std::sync::mpsc;

use overlay_engine::{CatalogIndex, FightSnapshot, WatchlistEntry};

use crate::panels;
use crate::panels::combat::CombatSide;
use crate::panels::combat_frame::CombatFrame;
use crate::panels::options_modal::{OptionsModalAction, OptionsModalAssets, OptionsModalState};
use crate::panels::watchlist::{WatchlistAssets, WatchlistToast};
use crate::portraits::PortraitAtlas;
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
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
/// par défaut d'egui (`icon_button::paint_tooltip_label`, fond plein, `inner_margin` 8px) contenant
/// une étiquette courte ("Alliés"/"Ennemis") sur une seule ligne tient sur ~30px de haut, plus
/// `icon_button::TOOLTIP_GAP` (5px) d'écart avec le widget — 44px laisse une marge confortable
/// au-dessus de ce total. Seul CE côté du panneau Combat gagne une marge (demande explicite :
/// « agrandis légèrement l'overlay ») : gauche/droite/bas restent collés au bord de la fenêtre de
/// jeu, décision non remise en cause ici (voir `main.rs::GAME_EDGE_MARGIN_PX`) — `main.rs`/`bin/
/// overlay-ui-x11.rs` agrandissent `WINDOW_SIZE` de ce même montant pour que le reste du panneau ne
/// soit pas compressé d'autant.
pub const COMBAT_TOP_MARGIN: f32 = 44.0;

/// Émis par le thread Engine (§3 du plan) ou le thread Auth (`spawn_auth_thread`) quand un nouvel
/// état est disponible — réveille le main thread, en `ControlFlow::Wait` le reste du temps (§6.1 :
/// pas de boucle 60 Hz forcée, l'overlay ne consomme rien tant que rien ne change). Publique : à
/// la fois `main.rs` (event loop winit) et `remote_icons::RemoteIconStore` (thread réseau partagé)
/// en ont besoin.
pub enum UserEvent {
    NewSnapshot,
    AuthStatusChanged,
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
    /// `main.rs::App::open_options_modal`/`bin/overlay-ui-x11.rs` (même méthode dupliquée).
    Options,
}

/// État de la connexion au compte (lot L4, §7.2 du plan) — publié par le thread Auth
/// (`main.rs::spawn_auth_thread`) via `Arc<ArcSwap<_>>`, lu par le main thread à chaque frame pour
/// décider d'afficher ou non l'icône de relance d'appairage (voir `build_ui`). Volontairement
/// distinct d'un simple `bool` : `Connecting` évite d'afficher l'icône pendant la toute première
/// tentative (jeton déjà stocké, ou premier appairage) — elle ne doit apparaître qu'après un échec
/// avéré.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthStatus {
    Connecting,
    /// Code d'appairage obtenu (`POST /api/v1/auth/native/pair`), en attente que l'utilisateur le
    /// saisisse sur `verification_url` — voir `overlay_sync::pair_and_wait`. Publié UNE FOIS par
    /// tentative, avant le premier sondage (`poll`), donc bien avant `Connected`/`Disconnected`.
    ///
    /// **UI de pairing (2026-09-02, lot L4)** : jusqu'ici le code n'était visible qu'en console
    /// (`tracing::info!`) — invisible pour qui joue en plein écran sans terminal à côté. `build_ui`
    /// l'affiche maintenant directement dans la fenêtre overlay (voir sa doc), le navigateur étant
    /// déjà ouvert automatiquement en best-effort (`open::that`, voir `pair_and_wait`) — ce champ
    /// couvre le cas où cette ouverture automatique échoue ou où l'onglet a été fermé par erreur.
    PairingStarted {
        pairing_code: String,
        verification_url: String,
    },
    Connected,
    /// Ni jeton valide ni appairage complété — l'icône de relance doit être visible (retour
    /// utilisateur 2026-09-01 : appairage en échec — 405 côté serveur — sans aucun moyen de
    /// retenter sans relancer tout le logiciel).
    ///
    /// `reason` porte le message d'erreur de la DERNIÈRE tentative (`attempt_connect`) — affiché
    /// en tooltip sur l'icône de relance (voir `build_ui`) : un clic qui ne se traduit par rien de
    /// visible (le serveur refuse la requête AVANT même qu'un code d'appairage existe, donc aucun
    /// navigateur ne s'ouvre) est indiscernable d'un bouton cassé sans ce message — retour
    /// utilisateur 2026-09-01 : « l'appui du bouton ne déclenche rien, pas de message d'erreur
    /// dans la console » — le message existait déjà (console), seulement invisible pour qui ne
    /// regarde pas un terminal ; il l'est maintenant aussi directement dans l'overlay.
    Disconnected {
        reason: String,
    },
}

/// Commande envoyée au thread Auth (`main.rs::spawn_auth_thread`) depuis le main thread — `Retry`
/// (clic sur l'icône de relance, `build_ui`) n'a d'effet que si PAS déjà connecté ; `Disconnect`
/// (raccourci `DISCONNECT_HOTKEY_LABEL`, `main.rs::App::disconnect_account`) n'a d'effet que si
/// déjà connecté. Les deux canaux d'origine (icône cliquée / hotkey pressé) convergent sur ce même
/// type plutôt que sur deux canaux séparés : un seul thread Auth, un seul point d'attente
/// (`command_rx.recv()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthCommand {
    Retry,
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
    pub combat_side: &'a mut CombatSide,
    pub watchlist: &'a [WatchlistEntry],
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
    pub options: Option<&'a mut OptionsModalState>,
    /// Textures du chrome de la modale Options (2026-09-09) — `Some` UNIQUEMENT pour `kind ==
    /// OverlayKind::Options`, chargées UNE FOIS par fenêtre OS (voir
    /// `main.rs`/`bin/overlay-ui-x11.rs`, `create_overlay_window`) et seulement référencées ici :
    /// contrairement à `options` (`&mut`, la frappe doit persister), ces textures ne changent
    /// jamais d'une frame à l'autre, une référence partagée suffit.
    pub options_assets: Option<&'a OptionsModalAssets>,
}

/// Ce qu'une frame de rendu a produit, au-delà de l'affichage lui-même — étend l'ancien simple
/// `bool` (`close_toast` seul) depuis le 2026-09-08 : la modale Options ajoute deux autres
/// signaux que `paint_content` doit remonter à l'appelant réel (`main.rs`/`bin/overlay-ui-x11.rs`,
/// seuls capables de créer une fenêtre OS ou de lancer un dialogue de fichier natif) SANS que
/// `paint_content`/`build_ui` eux-mêmes en dépendent — voir la doc de tête du module pour pourquoi
/// ces deux fonctions restent PURES.
#[derive(Debug, Default)]
pub struct RenderOutcome {
    /// Voir la doc historique de `build_ui` : fermeture du toast de suivi (clic sur la carte ou sa
    /// croix, `panels::watchlist::toast_card`).
    pub close_toast: bool,
    /// `true` UNIQUEMENT à la frame où le bouton "Options" du carré de contrôle
    /// (`panels::watchlist::control_button_row`) vient d'être cliqué — `kind == Watchlist`
    /// seulement, jamais émis par Combat/Options eux-mêmes.
    pub open_options: bool,
    /// Action déclenchée CETTE frame par la modale Options elle-même (`kind == Options`
    /// seulement) — voir `panels::options_modal::OptionsModalAction`.
    pub options_action: OptionsModalAction,
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
    raw_input: egui::RawInput,
    mut content: RenderContent<'_>,
) -> (egui::FullOutput, RenderOutcome) {
    let mut outcome = RenderOutcome::default();
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
                combat_side: &mut *content.combat_side,
                watchlist: content.watchlist,
                watchlist_toast: content.watchlist_toast,
                catalog: content.catalog,
                catalog_stale: content.catalog_stale,
                remote_icons: content.remote_icons,
                remote_icon_textures: &mut *content.remote_icon_textures,
                auth_status: content.auth_status,
                auth_command_tx: content.auth_command_tx,
                interactive: content.interactive,
                now: content.now,
                options: content.options.as_deref_mut(),
                options_assets: content.options_assets,
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
        combat_frame,
        icons,
        combat_side,
        watchlist,
        watchlist_toast,
        catalog,
        catalog_stale,
        remote_icons,
        remote_icon_textures,
        auth_status,
        auth_command_tx,
        interactive,
        now,
        options,
        options_assets,
    } = content;

    let mut outcome = RenderOutcome::default();
    // Marge interne nulle pour Combat sur trois côtés (refonte 2026-09-04, retour utilisateur :
    // collé au bord de la fenêtre de jeu, sans le moindre vide, pour simuler une interface qui
    // ferait partie du jeu — voir aussi `main.rs::GAME_EDGE_MARGIN_PX`, ramené à 0 pour la même
    // raison) — SEUL le haut gagne `COMBAT_TOP_MARGIN`, voir sa doc, pour que l'infobulle du switch
    // Alliés/Ennemis ait la place de s'afficher au-dessus de lui. Suivi garde sa marge d'origine
    // sur les quatre côtés : non concerné par cette demande, bande de tuiles qui a toujours besoin
    // d'un peu d'air pour ne pas coller aux boutons d'interface du jeu. Options (2026-09-08) est
    // une fenêtre dédiée qui remplit tout son espace elle-même (voir `panels::options_modal::show`,
    // bannière/corps/pied de page peints jusqu'aux bords) : aucune marge, comme Combat.
    let inner_margin = match kind {
        OverlayKind::Combat => egui::Margin {
            left: 0,
            right: 0,
            top: COMBAT_TOP_MARGIN as i8,
            bottom: 0,
        },
        OverlayKind::Watchlist => egui::Margin::same(6),
        OverlayKind::Options => egui::Margin::ZERO,
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
                                        let retry = ui.add(egui::Button::new("🔌").small());
                                        // `combat::show_tooltip_above` plutôt qu'un `on_hover_text`
                                        // brut (refonte 2026-09-06, design system tooltip) — voir
                                        // sa doc.
                                        panels::combat::show_tooltip_above(
                                            &retry,
                                            &format!(
                                                "Connecter le compte (relance l'appairage, ouvre \
                                                 le navigateur).\nDernier échec : {reason}"
                                            ),
                                        );
                                        if retry.clicked() {
                                            auth_command_tx.send(AuthCommand::Retry);
                                        }
                                    },
                                );
                            });
                            ui.add_space(4.0);
                        }
                        // UI de pairing (2026-09-02, lot L4) : le code n'était jusqu'ici visible
                        // qu'en console (`tracing::info!`) — invisible pour qui joue en plein
                        // écran sans terminal à côté. Le navigateur s'est déjà ouvert tout seul
                        // (best-effort, `open::that` dans `pair_and_wait`) ; cette carte couvre
                        // le cas où cette ouverture échoue ou où l'onglet a été fermé par erreur
                        // — code affiché en GRAND (il faut pouvoir le lire et le taper sans
                        // plisser les yeux), bouton de copie, et bouton pour rouvrir la page si
                        // besoin. Reste minimal : pas de fenêtre dédiée, une simple carte dans la
                        // zone Combat comme le reste de ces indicateurs.
                        AuthStatus::PairingStarted {
                            pairing_code,
                            verification_url,
                        } => {
                            egui::Frame::new()
                                .fill(ui.visuals().extreme_bg_color)
                                .corner_radius(4.0)
                                .inner_margin(6.0)
                                .show(ui, |ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.weak("Connexion du compte — code d'appairage");
                                        ui.add_space(2.0);
                                        ui.label(
                                            egui::RichText::new(pairing_code)
                                                .monospace()
                                                .size(20.0)
                                                .strong(),
                                        );
                                        ui.add_space(2.0);
                                        ui.horizontal(|ui| {
                                            if ui.small_button("📋 Copier").clicked() {
                                                ui.ctx().copy_text(pairing_code.clone());
                                            }
                                            if ui.small_button("🌐 Ouvrir la page").clicked() {
                                                let _ = open::that(verification_url);
                                            }
                                        });
                                    });
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
                                    panels::combat::show_tooltip_above(
                                        &warning,
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
                            icons,
                            catalog,
                            remote_icons,
                            remote_icon_textures,
                        },
                        watchlist,
                        watchlist_toast,
                        now,
                    );
                    outcome.close_toast = watchlist_outcome.close_toast;
                    outcome.open_options = watchlist_outcome.open_options;
                }
                // Modale Options (2026-09-08, §9 du plan) — voir `panels::options_modal`. `options`
                // est `Some` uniquement pour ce `kind` (voir la doc de `RenderContent::options`) ;
                // un appelant qui créerait une fenêtre `OverlayKind::Options` sans fournir cet état
                // ne verrait simplement rien peint ici plutôt que de paniquer — jamais souhaitable
                // en pratique (voir `main.rs`/`bin/overlay-ui-x11.rs`, qui le fournissent toujours
                // pour ce cas), mais plus sûr qu'un `expect` sur un chemin de rendu.
                OverlayKind::Options => {
                    if let (Some(state), Some(assets)) = (options, options_assets) {
                        outcome.options_action = panels::options_modal::show(ui, state, assets);
                    }
                }
            }
        });
    outcome
}
