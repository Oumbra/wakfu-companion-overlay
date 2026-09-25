//! Raccourcis clavier globaux de l'overlay, **personnalisables** (onglet "Raccourcis" de la modale
//! Options, `panels::options_modal` — calqué sur l'onglet "Commandes" du jeu réel,
//! `assets/design-system/interfaces/interface-options-commandes.png`).
//!
//! **La déconnexion du compte n'en fait plus partie depuis le 2026-09-13** (demande utilisateur, le
//! jour même de la création de cet onglet) : elle est devenue un BOUTON de l'onglet « Paramètres »
//! (section « Compte », voir `panels::options_modal`). Une action qui renvoie l'overlay à son écran
//! de connexion n'a pas sa place derrière une combinaison globale qu'on peut frapper par
//! inadvertance, d'autant qu'elle ne se rejoue qu'en refaisant tout un appairage.
//!
//! **La fermeture de l'overlay non plus, depuis le 2026-09-17** (demande utilisateur), pour la
//! même raison : une combinaison globale qui tue le programme d'un seul geste, sans confirmation,
//! est un piège en plein combat. Elle est devenue le bouton « Fermer l'overlay » en pied de
//! l'onglet « Paramètres » (confirmé, 2026-09-16), doublé par l'entrée « Quitter » de la zone de
//! notification — et Ctrl+C dans le terminal reste.
//!
//! Jusqu'ici chaque combinaison était une constante en dur dans
//! `main.rs`/`bin/wakfu-companion-overlay-x11.rs` (`HOTKEY_LABEL`, `DETAILS_HOTKEY_LABEL`…),
//! doublée d'un libellé recopié à la main dans les tooltips des boutons
//! (`panels::watchlist::control_button_row`, `panels::combat::paint_side_switch`). Ce module
//! devient la **source unique** : la liste des actions ([`ShortcutAction`]), leur combinaison par
//! défaut, la combinaison effective après personnalisation ([`ShortcutBindings`]) et le libellé
//! affiché partout (`Ctrl+Shift+W`).
//!
//! **Portée réelle des raccourcis** : `global_hotkey` (XGrabKey sous X11, `RegisterHotKey` sous
//! Windows) — ils fonctionnent sans focus sur une fenêtre overlay (celles-ci portent
//! `WS_EX_NOACTIVATE` et ne reçoivent qu'exceptionnellement un événement clavier : un clic en mode
//! interactif peut malgré tout leur donner le premier plan, ce qui a coûté le filet « Échap » de
//! `main.rs::window_event`, retiré le 2026-09-17), donc **volés au jeu et à toute autre
//! application** : d'où le
//! garde-fou [`Shortcut::is_valid`] — une lettre nue ne peut pas être bindée, elle serait prise à
//! Wakfu lui-même dès la première ligne de chat écrite.
//!
//! **Exception : les touches de fonction nues** (F1-F12), autorisées depuis le 2026-09-13 avec les
//! raccourcis multicompte ([`ShortcutAction::InvitePartner`]/[`ShortcutAction::FollowPartner`], F1
//! et F2 par défaut — combinaisons demandées telles quelles par l'utilisateur). Elles ne
//! s'écrivent pas, donc ne gênent aucune saisie ; et « voler la touche au jeu » est ICI l'effet
//! recherché — c'est l'overlay, pas Wakfu, qui doit réagir à F1. Le revers reste entier et vaut
//! pour toute personnalisation de ce genre : la touche est confisquée à TOUTES les applications
//! tant que l'overlay tourne (F1 n'ouvre plus l'aide du navigateur). Une action multicompte ne
//! fait rien quand le premier plan n'est pas le jeu (`chat_command::PartnerError::NoGameFocused`),
//! mais la frappe est perdue pour l'application qui avait le focus.
//!
//! **Les raccourcis multicompte se désactivent d'un bloc depuis le 2026-09-25** (demande
//! utilisateur : « permets de les désactiver dans une section multi-compte avec une seule option
//! pour activer/désactiver les deux raccourcis »). Ils tapent une commande dans le chat du jeu à la
//! place du joueur (`chat_command`) — ce que les CGU d'Ankama nomment, voir `docs/analyse-cgu.md`
//! §3.1 — et l'onglet « À propos » les présentait comme optionnels alors qu'on ne pouvait que les
//! réassigner. Désactivés ([`ShortcutBindings::multiaccount_enabled`]), ils ne sont plus
//! enregistrés auprès de l'OS : F1 et F2 reviennent au jeu et aux autres applications. **Actifs
//! par défaut**, comme avant, pour ne rien retirer à ceux qui s'en servent.
//!
//! **Toutes les actions ne sont pas câblées sur les deux OS** : le binaire Linux
//! (`bin/wakfu-companion-overlay-x11.rs`) n'enregistre que les actions de
//! [`ShortcutAction::LINUX_SUPPORTED`] (voir sa doc de module : pas de thread Auth/Catalogue ni de
//! compte lié). Les autres restent personnalisables et persistées, simplement inertes là-bas.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::GlobalHotKeyManager;

/// Une action de l'overlay déclenchable au clavier. L'ordre de [`ShortcutAction::ALL`] est celui
/// d'affichage dans l'onglet "Raccourcis" (regroupé par [`ShortcutAction::section`], sections dans
/// l'ordre d'apparition) et sert d'index dans [`ShortcutBindings`] — ne pas réordonner sans
/// relire ce module.
///
/// **La doc de chaque variante conserve l'historique** des anciennes constantes de `main.rs`
/// (`HOTKEY_LABEL`, `REFRESH_HOTKEY_LABEL`… supprimées en même temps que ce module est né) : le
/// POURQUOI de chaque combinaison par défaut vaut toujours, même maintenant que l'utilisateur peut
/// en changer — c'est ce qui explique pourquoi ces défauts-là, et ce qu'une personnalisation
/// malheureuse risque de casser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShortcutAction {
    /// Bascule interactif / clic-traversant de toutes les fenêtres overlay
    /// (`main.rs::App::toggle_interactive`) — mécanisme PRINCIPAL du click-through (§6.3 du plan),
    /// historiquement le tout premier raccourci de l'overlay.
    Toggle,
    /// Rafraîchissement forcé : redessin + réaffirmation topmost immédiate + nouvelle demande des
    /// réglages de compte (`main.rs::App::force_refresh`) — demande explicite de l'utilisateur
    /// (2026-09-02) pour récupérer un overlay bloqué (mauvaise taille, plus au premier plan, Suivi
    /// resté vide) sans relancer le processus.
    ///
    /// **Ctrl+Shift+R plutôt que F5** (suggestion initiale de l'utilisateur, 2026-09-02) : ces
    /// raccourcis sont GLOBAUX, jamais limités à la fenêtre de jeu malgré la demande « quand on est
    /// focus sur une fenêtre de jeu » — voler F5 à Wakfu (sorts/actions fréquents sur les touches de
    /// fonction) ou à l'application au premier plan serait activement nuisible. Vaut toujours comme
    /// mise en garde pour une personnalisation : une touche de fonction nue est un mauvais choix
    /// ici, même si le champ l'accepte avec un modificateur.
    Refresh,
    // **Il y avait ici `Quit`** (fermeture de l'overlay, `Ctrl+Shift+Q`, `event_loop.exit()`),
    // retiré le 2026-09-17 à la demande de l'utilisateur — voir la doc de module. Il avait été
    // créé en raccourci GLOBAL plutôt qu'en Échap (retour utilisateur 2026-09-02) parce que les
    // fenêtres overlay portent `WS_EX_NOACTIVATE` (voir `main.rs::apply_extended_styles`, jamais
    // désactivé même en mode interactif — nécessaire pour ne jamais voler le focus au jeu) donc ne
    // reçoivent JAMAIS `WindowEvent::KeyboardInput` : Échap ne se déclenchait jamais malgré ce
    // qu'annonçait la bannière, et l'utilisateur en était réduit à un Ctrl+C dans le terminal
    // (`STATUS_CONTROL_C_EXIT` en sortie — normal, mais peu clair). Ce besoin-là est couvert
    // depuis par le bouton « Fermer l'overlay » de l'onglet « Paramètres » et par la zone de
    // notification. Sa clé de config (`quit`) est ignorée à la relecture, comme `disconnect`.
    /// Ouverture de la modale Options (`panels::options_modal`) — le seul raccourci qui reste
    /// atteignable quoi qu'il arrive aux autres : c'est par lui qu'on répare une personnalisation
    /// ratée.
    Options,
    /// Ouverture du site (bouton "Détails" du carré de contrôle,
    /// `open::that(overlay_sync::client::base_url())`).
    ///
    /// Fait partie du groupe de cinq raccourcis demandés explicitement par l'utilisateur le
    /// 2026-09-06 (« à l'image de ce qu'il y a dans le jeu [...] rajoute les raccourcis [...] pour
    /// le détail [...] Ctrl+Shift+D ») — combinaisons données par l'utilisateur lui-même, et
    /// reflétées entre parenthèses dans les infobulles correspondantes
    /// (`panels::watchlist::control_button_row`, `panels::combat::paint_side_switch`), à l'image du
    /// jeu. C'est cette demande-là qui rend la personnalisation utile : ces libellés d'infobulle
    /// suivent désormais la combinaison RÉELLE.
    ///
    /// **`Ctrl+Shift+S` depuis le 2026-09-18** (demande utilisateur), en même temps que le libellé
    /// est passé de « Ouvrir les détails (site) » à « Ouvrir le site » : l'action ne s'appelle plus
    /// « détails », sa lettre non plus — **S comme site**. Le `D` libéré revient à
    /// [`Self::WatchlistRemove`], qui l'a échangé contre son `S` (voir sa doc) ; les deux bougent
    /// donc ensemble, un `Ctrl+Shift+S` qui ouvrirait le site ET retirerait du suivi serait un
    /// conflit refusé par [`ShortcutBindings::conflict`].
    Details,
    /// Bouton "Ajouter" du carré de contrôle — reste INERTE comme le bouton lui-même (voir la doc
    /// de module de `panels::watchlist` : aucune sélection/formulaire câblés côté overlay pour
    /// cette itération), juste enregistré/journalisé.
    WatchlistAdd,
    /// Bouton "Supprimer" du carré de contrôle — même remarque que [`Self::WatchlistAdd`].
    ///
    /// **`Ctrl+Shift+D` depuis le 2026-09-18** (demande utilisateur) : échange de lettre avec
    /// [`Self::Details`], parti sur le `S` de « site » — **D comme delete**, la lettre du retrait
    /// dans à peu près tous les logiciels.
    WatchlistRemove,
    /// Bascule Alliés/Ennemis du panneau Combat, EN MODE TOGGLE (retour utilisateur explicite :
    /// « ça inverse la sélection [...] si actuellement c'est sélectionné allié [...] ça passe en
    /// ennemi et inversement ») plutôt que deux raccourcis séparés un par camp — voir
    /// `panels::combat::CombatSide::toggled` et `main.rs::App::toggle_combat_side`, appliqué à
    /// CHAQUE fenêtre Combat ouverte, pas seulement celle au premier plan.
    ///
    /// **`Ctrl+Shift+T` depuis le 2026-09-18** (demande utilisateur), après un `Ctrl+Shift+E` né
    /// du groupe du 2026-09-06 : **T comme toggle**, ce que fait exactement l'action — le `E`
    /// d'« ennemis » ne disait que la moitié d'une bascule qui va aussi dans l'autre sens.
    CombatSide,
    /// Fait tourner la grandeur mesurée par le panneau Combat — Dégâts → Armure → Soins → Dégâts
    /// (demande utilisateur du 2026-09-14, `Ctrl+Shift+V`). Même mode TOGGLE que [`Self::CombatSide`]
    /// juste au-dessus, et pour la même raison : une seule touche pour tout le cycle plutôt qu'une
    /// par grandeur, appliquée à CHAQUE fenêtre Combat ouverte. Voir
    /// `panels::combat::CombatMetric::next` et `main.rs::App::cycle_combat_metric`.
    CombatMetric,
    /// Invite en groupe le personnage de l'AUTRE fenêtre de jeu — tape `/i "<nom>"` dans le chat de
    /// la fenêtre au premier plan (`chat_command`, voir sa doc de module pour toute la mécanique).
    ///
    /// **F1 nue par défaut**, à la demande explicite de l'utilisateur (2026-09-13) et par exception
    /// à la règle du modificateur (voir doc de module) : le geste doit être aussi immédiat que les
    /// raccourcis du jeu qu'il imite, et c'est en jouant — donc les deux mains sur le clavier, en
    /// plein combat ou en pleine course — qu'on invite son second compte. Un `Ctrl+Shift+…` à trois
    /// doigts raterait entièrement l'intention.
    InvitePartner,
    /// Fait suivre le personnage de l'AUTRE fenêtre de jeu — `/fol "<nom>"`, **F2 nue** par défaut.
    /// Même demande, même justification et même mécanique que [`Self::InvitePartner`].
    FollowPartner,
    // **Il y avait ici `Disconnect`** (déconnexion du compte, `Ctrl+Alt+D` — le seul raccourci
    // resté en Ctrl+Alt quand les autres sont passés en Ctrl+Shift le 2026-09-06). Retiré le
    // 2026-09-13 à la demande de l'utilisateur : c'est désormais un bouton de l'onglet
    // « Paramètres », voir la doc de module. Sa clé de config (`disconnect`) est simplement ignorée
    // à la relecture d'un `config.toml` plus ancien (`ShortcutBindings::from_config`).
}

impl ShortcutAction {
    /// Toutes les actions, dans l'ordre d'affichage — voir doc du type.
    pub const ALL: [Self; 10] = [
        Self::Toggle,
        Self::Refresh,
        Self::Options,
        Self::Details,
        Self::WatchlistAdd,
        Self::WatchlistRemove,
        Self::CombatSide,
        Self::CombatMetric,
        Self::InvitePartner,
        Self::FollowPartner,
    ];

    /// Actions réellement enregistrées par le binaire Linux (`bin/wakfu-companion-overlay-x11.rs`)
    /// — voir doc de module. Les autres restent personnalisables et persistées là-bas (un même
    /// `config.toml` peut servir aux deux OS), simplement sans effet : ce binaire n'a ni thread
    /// Auth/Catalogue à rafraîchir, ni compte à déconnecter, ni fenêtre Combat à basculer.
    ///
    /// **À tenir à jour avec ce que ce binaire câble réellement** : une action ajoutée là-bas sans
    /// être ajoutée ici ne serait jamais enregistrée auprès de l'OS.
    pub const LINUX_SUPPORTED: [Self; 5] = [
        Self::Toggle,
        Self::Options,
        Self::WatchlistRemove,
        // Câblés là-bas comme sous Windows : `chat_command` a une implémentation X11 complète
        // (XTEST, `overlay_platform::linux::keyboard`) et le binaire Linux connaît déjà la fenêtre
        // active (`_NET_ACTIVE_WINDOW`) — rien de ce qui manque à ce binaire (compte lié, thread
        // Catalogue) n'entre en jeu ici.
        Self::InvitePartner,
        Self::FollowPartner,
    ];

    /// Clé stable utilisée dans `config.toml` (table `[shortcuts]`) — **jamais** la position dans
    /// [`Self::ALL`] ni le libellé affiché : ces deux-là peuvent bouger sans invalider la config
    /// déjà écrite sur le disque des utilisateurs.
    pub fn key(self) -> &'static str {
        match self {
            Self::Toggle => "toggle",
            Self::Refresh => "refresh",
            Self::Options => "options",
            Self::Details => "details",
            Self::WatchlistAdd => "watchlist_add",
            Self::WatchlistRemove => "watchlist_remove",
            Self::CombatSide => "combat_side",
            Self::CombatMetric => "combat_metric",
            Self::InvitePartner => "invite_partner",
            Self::FollowPartner => "follow_partner",
        }
    }

    /// Libellé affiché à gauche de la ligne dans l'onglet "Raccourcis" (colonne "Barre 1 - Bouton
    /// N" de la référence `interface-options-commandes.png`).
    pub fn label(self) -> &'static str {
        match self {
            Self::Toggle => "Interactif / clic-traversant",
            Self::Refresh => "Rafraîchir l'affichage",
            Self::Options => "Ouvrir les options",
            Self::Details => "Ouvrir le site",
            Self::WatchlistAdd => "Ajouter au suivi",
            Self::WatchlistRemove => "Retirer du suivi",
            Self::CombatSide => "Alterner Alliés / Ennemis",
            Self::CombatMetric => "Dégâts / Armure / Soins",
            Self::InvitePartner => "Inviter l'autre personnage",
            Self::FollowPartner => "Suivre l'autre personnage",
        }
    }

    /// En-tête de groupe sous lequel la ligne est rangée (équivalent de "Barres de raccourcis" sur
    /// la référence) — les actions d'une même section sont contiguës dans [`Self::ALL`].
    pub fn section(self) -> &'static str {
        match self {
            Self::Toggle | Self::Refresh | Self::Options => "Overlay",
            Self::Details | Self::WatchlistAdd | Self::WatchlistRemove => "Suivi",
            Self::CombatSide | Self::CombatMetric => "Combat",
            Self::InvitePartner | Self::FollowPartner => "Multicompte",
        }
    }

    /// Combinaison d'origine, celle qui était codée en dur avant ce module (voir la doc des
    /// anciennes constantes de `main.rs` pour le POURQUOI de chacune — notamment `Ctrl+Shift`
    /// plutôt que les touches de fonction, volées au jeu sinon).
    pub fn default_shortcut(self) -> Shortcut {
        let ctrl_shift = Modifiers::CONTROL | Modifiers::SHIFT;
        match self {
            Self::Toggle => Shortcut::new(ctrl_shift, Code::KeyW),
            Self::Refresh => Shortcut::new(ctrl_shift, Code::KeyR),
            Self::Options => Shortcut::new(ctrl_shift, Code::KeyO),
            Self::Details => Shortcut::new(ctrl_shift, Code::KeyS),
            Self::WatchlistAdd => Shortcut::new(ctrl_shift, Code::KeyA),
            Self::WatchlistRemove => Shortcut::new(ctrl_shift, Code::KeyD),
            Self::CombatSide => Shortcut::new(ctrl_shift, Code::KeyT),
            Self::CombatMetric => Shortcut::new(ctrl_shift, Code::KeyV),
            // Nues, par exception assumée — voir la doc de ces deux variantes.
            Self::InvitePartner => Shortcut::new(Modifiers::empty(), Code::F1),
            Self::FollowPartner => Shortcut::new(Modifiers::empty(), Code::F2),
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.key() == key)
    }

    /// Une action de la section « Multicompte » — celles que coupe
    /// [`ShortcutBindings::multiaccount_enabled`].
    pub fn is_multiaccount(self) -> bool {
        matches!(self, Self::InvitePartner | Self::FollowPartner)
    }
}

/// Clés de config des raccourcis RETIRÉS — déconnexion (`disconnect`, 2026-09-13) et fermeture de
/// l'overlay (`quit`, 2026-09-17) — voir [`ShortcutAction`] et [`ShortcutBindings::from_config`].
const RETIRED_KEYS: [&str; 2] = ["disconnect", "quit"];

/// Une combinaison `modificateurs + touche`. Enveloppe volontaire de `global_hotkey::HotKey` (dont
/// elle sait produire l'équivalent, [`Shortcut::to_hotkey`]) : ce dernier n'a **ni libellé lisible**
/// (`HotKey::to_string` donne `shift+control+KeyW`) **ni parsing tolérant** utilisable pour un
/// fichier de configuration écrit à la main, et ne sait rien de la saisie egui.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub mods: Modifiers,
    pub code: Code,
}

impl Shortcut {
    pub fn new(mods: Modifiers, code: Code) -> Self {
        Self { mods, code }
    }

    /// Au moins un modificateur — **ou** une touche de fonction, seule famille de touches nues
    /// autorisée (voir doc de module : elles ne s'écrivent pas, et les raccourcis multicompte les
    /// demandent nues). Toute autre touche nue volerait le caractère à Wakfu lui-même dès la
    /// première ligne de chat. Les combinaisons produites par [`Self::from_egui`] comme par
    /// [`Self::parse`] passent forcément par ici avant d'être retenues.
    pub fn is_valid(&self) -> bool {
        self.mods
            .intersects(Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT | Modifiers::SUPER)
            || is_function_key(self.code)
    }

    /// Équivalent `global_hotkey` à enregistrer auprès du `GlobalHotKeyManager`.
    pub fn to_hotkey(self) -> HotKey {
        HotKey::new(Some(self.mods), self.code)
    }

    /// Écriture canonique, affichée dans l'UI comme dans `config.toml` : `Ctrl+Shift+W`. Ordre des
    /// modificateurs figé (Ctrl, Alt, Shift, Super) pour que deux raccourcis identiques aient
    /// toujours la même chaîne (comparaison de conflits, diff de config lisible).
    pub fn label(&self) -> String {
        let mut out = String::new();
        if self.mods.contains(Modifiers::CONTROL) {
            out.push_str("Ctrl+");
        }
        if self.mods.contains(Modifiers::ALT) {
            out.push_str("Alt+");
        }
        if self.mods.contains(Modifiers::SHIFT) {
            out.push_str("Shift+");
        }
        if self.mods.contains(Modifiers::SUPER) {
            out.push_str("Super+");
        }
        out.push_str(key_label(self.code));
        out
    }

    /// Lecture d'une chaîne `Ctrl+Shift+W` (celle de `config.toml`, potentiellement éditée à la
    /// main) — tolérante sur la casse, les espaces et les alias usuels (`Control`, `Maj`, `Option`,
    /// `Win`). `None` si un jeton est inconnu, si aucune touche principale n'est donnée ou si la
    /// combinaison ne passe pas [`Self::is_valid`] : l'appelant retombe alors sur le défaut plutôt
    /// que de laisser une action sans raccourci (voir [`ShortcutBindings::from_config`]).
    pub fn parse(raw: &str) -> Option<Self> {
        let mut mods = Modifiers::empty();
        let mut code = None;
        for token in raw.split('+') {
            let token = token.trim();
            if token.is_empty() {
                return None;
            }
            // `to_uppercase` (Unicode) et non `to_ascii_uppercase` : "Entrée" doit donner
            // "ENTRÉE" pour que `parse_key` la reconnaisse — voir le test `aller_retour_toutes_les_touches`.
            match token.to_uppercase().as_str() {
                "CTRL" | "CONTROL" => mods |= Modifiers::CONTROL,
                "ALT" | "OPTION" => mods |= Modifiers::ALT,
                "SHIFT" | "MAJ" => mods |= Modifiers::SHIFT,
                "SUPER" | "WIN" | "META" | "CMD" => mods |= Modifiers::SUPER,
                _ => {
                    if code.is_some() {
                        // Deux touches principales : "Ctrl+A+B" n'a pas de sens.
                        return None;
                    }
                    code = Some(parse_key(token)?);
                }
            }
        }
        let shortcut = Self::new(mods, code?);
        shortcut.is_valid().then_some(shortcut)
    }

    /// Traduit une frappe capturée par egui (onglet "Raccourcis" en cours d'écoute, voir
    /// `panels::options_modal`). `None` si la touche n'a pas d'équivalent `global_hotkey`
    /// (`egui::Key` couvre des touches que `RegisterHotKey`/XGrabKey ne savent pas viser) ou si
    /// aucun modificateur n'est enfoncé — l'UI affiche alors son message d'aide sans rien changer.
    pub fn from_egui(key: egui::Key, modifiers: egui::Modifiers) -> Option<Self> {
        let mut mods = Modifiers::empty();
        // `modifiers.command` est un alias de `ctrl` hors macOS (hors périmètre du projet) : lu
        // quand même pour ne pas dépendre de la façon dont egui remplit l'un ou l'autre.
        if modifiers.ctrl || modifiers.command {
            mods |= Modifiers::CONTROL;
        }
        if modifiers.alt {
            mods |= Modifiers::ALT;
        }
        if modifiers.shift {
            mods |= Modifiers::SHIFT;
        }
        let shortcut = Self::new(mods, code_from_egui(key)?);
        shortcut.is_valid().then_some(shortcut)
    }
}

impl fmt::Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

/// Combinaison effective de CHAQUE action (jamais d'action sans raccourci : une entrée absente ou
/// illisible dans `config.toml` retombe sur [`ShortcutAction::default_shortcut`]). Indexé par
/// `action as usize`, d'où l'ordre figé de [`ShortcutAction::ALL`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutBindings {
    slots: [Shortcut; ShortcutAction::ALL.len()],
    /// Les raccourcis multicompte sont-ils actifs ? — case « Activer les raccourcis multicompte »
    /// de l'onglet « Raccourcis » (voir doc de module). Faux, les actions
    /// [`ShortcutAction::is_multiaccount`] gardent leur combinaison mais ne sont plus
    /// enregistrées, et n'entrent plus dans la détection de doublons.
    ///
    /// Persisté hors de la table `[shortcuts]`, qui ne porte que des combinaisons
    /// (`config::OverlayConfig::multiaccount_shortcuts`).
    multiaccount: bool,
}

impl Default for ShortcutBindings {
    fn default() -> Self {
        let mut slots = [ShortcutAction::Toggle.default_shortcut(); ShortcutAction::ALL.len()];
        for action in ShortcutAction::ALL {
            slots[action as usize] = action.default_shortcut();
        }
        Self {
            slots,
            multiaccount: true,
        }
    }
}

impl ShortcutBindings {
    pub fn get(&self, action: ShortcutAction) -> Shortcut {
        self.slots[action as usize]
    }

    pub fn set(&mut self, action: ShortcutAction, shortcut: Shortcut) {
        self.slots[action as usize] = shortcut;
    }

    /// Voir le champ `multiaccount`.
    pub fn multiaccount_enabled(&self) -> bool {
        self.multiaccount
    }

    /// Accès mutable au drapeau multicompte — pour la case de l'onglet « Raccourcis », qui
    /// travaille sur le brouillon comme les combinaisons.
    pub fn multiaccount_enabled_mut(&mut self) -> &mut bool {
        &mut self.multiaccount
    }

    pub fn set_multiaccount_enabled(&mut self, enabled: bool) {
        self.multiaccount = enabled;
    }

    /// L'action est-elle à enregistrer auprès de l'OS ? Faux pour une action multicompte quand
    /// celles-ci sont désactivées.
    pub fn is_active(&self, action: ShortcutAction) -> bool {
        self.multiaccount || !action.is_multiaccount()
    }

    /// Libellé prêt à afficher (`Ctrl+Shift+W`) — utilisé par les tooltips des boutons du carré de
    /// contrôle et du switch Alliés/Ennemis, qui affichent ainsi la combinaison RÉELLE de
    /// l'utilisateur et non plus une chaîne recopiée à la main.
    pub fn label(&self, action: ShortcutAction) -> String {
        self.get(action).label()
    }

    /// Reconstruit les combinaisons depuis la table `[shortcuts]` de `config.toml` — clés inconnues
    /// ignorées (config d'une version ultérieure, ou faute de frappe), valeurs illisibles
    /// remplacées par le défaut de l'action concernée. Best-effort comme tout `config` : jamais
    /// d'échec dur, seulement une trace.
    pub fn from_config(raw: &BTreeMap<String, String>) -> Self {
        let mut bindings = Self::default();
        for (key, value) in raw {
            let Some(action) = ShortcutAction::from_key(key) else {
                if RETIRED_KEYS.contains(&key.as_str()) {
                    // Clé d'un raccourci RETIRÉ (voir `ShortcutAction`), pas une faute de frappe :
                    // tout `config.toml` écrit avant le 2026-09-17 en porte au moins une.
                    // Silencieuse, donc, là où une clé vraiment inconnue mérite un avertissement.
                    continue;
                }
                tracing::warn!(
                    "[raccourcis] action inconnue « {key} » dans la configuration — ignorée."
                );
                continue;
            };
            match Shortcut::parse(value) {
                Some(shortcut) => bindings.set(action, shortcut),
                None => tracing::warn!(
                    "[raccourcis] combinaison illisible « {value} » pour « {key} » — raccourci par défaut conservé."
                ),
            }
        }
        bindings
    }

    /// Table `[shortcuts]` à écrire dans `config.toml` — TOUTES les actions, y compris celles
    /// laissées au défaut : un fichier qui liste l'intégralité des raccourcis se relit et s'édite
    /// à la main bien plus facilement qu'un fichier qui n'en montre que les personnalisés.
    pub fn to_config(&self) -> BTreeMap<String, String> {
        ShortcutAction::ALL
            .into_iter()
            .map(|action| (action.key().to_string(), self.get(action).label()))
            .collect()
    }

    /// Première paire d'actions partageant la même combinaison, s'il y en a une — un doublon fait
    /// échouer l'enregistrement de la deuxième auprès de l'OS (même `HotKey::id`), l'UI le refuse
    /// donc AVANT de valider (voir `panels::options_modal`).
    ///
    /// Une action désactivée (multicompte coupé) n'entre pas en compte : elle n'est pas
    /// enregistrée, donc ne prend sa touche à personne — et réactiver la case refait passer ce
    /// contrôle avant « Valider ».
    pub fn conflict(&self) -> Option<(ShortcutAction, ShortcutAction)> {
        for (index, first) in ShortcutAction::ALL.into_iter().enumerate() {
            for second in ShortcutAction::ALL.into_iter().skip(index + 1) {
                if self.is_active(first)
                    && self.is_active(second)
                    && self.get(first) == self.get(second)
                {
                    return Some((first, second));
                }
            }
        }
        None
    }
}

/// Enregistrement auprès de l'OS des raccourcis d'un binaire : possède le `GlobalHotKeyManager`
/// (qui doit rester en vie tant qu'on veut recevoir des événements — voir le spike S1), la table
/// `id de HotKey -> action` consultée à la réception, et les combinaisons actuellement actives.
///
/// Existe pour que `main.rs` (Windows, toutes les actions) et `bin/wakfu-companion-overlay-x11.rs`
/// (Linux, les trois de [`ShortcutAction::LINUX_SUPPORTED`]) partagent la MÊME logique
/// d'(dés)enregistrement — devenue non triviale avec la personnalisation : il faut désenregistrer
/// l'ancien jeu avant d'enregistrer le nouveau ([`Self::apply`]), et tout désenregistrer tant que
/// la modale Options est ouverte ([`Self::suspend`]) sans quoi l'OS avalerait la frappe que
/// l'utilisateur essaie justement d'assigner.
///
/// **Aucun `expect` sur l'enregistrement** (contrairement au code d'origine, qui paniquait) : une
/// combinaison peut être refusée par l'OS parce qu'une AUTRE application l'a déjà prise — cas
/// désormais atteignable en tapant simplement une combinaison malheureuse dans la modale, sans
/// aucune raison de faire tomber tout l'overlay. L'action concernée reste sans raccourci pour la
/// session (et le reste de l'overlay fonctionne), l'échec est journalisé et remonté par
/// [`Self::apply`].
pub struct ShortcutRegistry {
    manager: GlobalHotKeyManager,
    /// Actions que CE binaire sait réellement traiter — voir [`ShortcutAction::LINUX_SUPPORTED`].
    supported: Vec<ShortcutAction>,
    bindings: ShortcutBindings,
    /// Ce qui est EFFECTIVEMENT enregistré auprès de l'OS en ce moment (vide après
    /// [`Self::suspend`]) — à désenregistrer avant tout nouvel enregistrement.
    active: Vec<HotKey>,
    ids: HashMap<u32, ShortcutAction>,
    suspended: bool,
}

impl ShortcutRegistry {
    /// Crée le gestionnaire OS et enregistre `bindings` pour les actions `supported`.
    pub fn new(supported: &[ShortcutAction], bindings: ShortcutBindings) -> Self {
        let mut registry = Self {
            manager: GlobalHotKeyManager::new().expect("création GlobalHotKeyManager"),
            supported: supported.to_vec(),
            bindings,
            active: Vec::new(),
            ids: HashMap::new(),
            suspended: false,
        };
        registry.register_active();
        registry
    }

    /// Combinaisons actuellement retenues — à passer au rendu (infobulles, état initial de la
    /// modale Options). Inchangées par [`Self::suspend`] : une suspension ne concerne que l'OS.
    pub fn bindings(&self) -> &ShortcutBindings {
        &self.bindings
    }

    /// Action correspondant à un `GlobalHotKeyEvent::id` reçu, s'il vient bien de l'un de nos
    /// raccourcis.
    pub fn action_for(&self, id: u32) -> Option<ShortcutAction> {
        self.ids.get(&id).copied()
    }

    /// Remplace le jeu de raccourcis (validation de la modale Options) — renvoie les actions dont
    /// l'enregistrement a été refusé par l'OS, pour que l'appelant le signale à sa façon. Rien
    /// n'est enregistré tant que la suspension est active : le nouveau jeu prendra effet au
    /// [`Self::resume`] qui suit (c'est exactement l'ordre que suit la fermeture de la modale).
    pub fn apply(&mut self, bindings: ShortcutBindings) -> Vec<ShortcutAction> {
        self.unregister_active();
        self.bindings = bindings;
        if self.suspended {
            return Vec::new();
        }
        self.register_active()
    }

    /// Rend TOUTES les combinaisons au système — appelé à l'ouverture de la modale Options : sans
    /// ça, `Ctrl+Shift+O` (ou toute autre combinaison déjà prise par l'overlay) serait interceptée
    /// par l'OS et n'arriverait jamais jusqu'à la case en écoute.
    pub fn suspend(&mut self) {
        if self.suspended {
            return;
        }
        self.unregister_active();
        self.suspended = true;
    }

    /// Ré-enregistre les combinaisons courantes (fermeture de la modale) — renvoie comme
    /// [`Self::apply`] les actions refusées par l'OS.
    pub fn resume(&mut self) -> Vec<ShortcutAction> {
        if !self.suspended {
            return Vec::new();
        }
        self.suspended = false;
        self.register_active()
    }

    fn register_active(&mut self) -> Vec<ShortcutAction> {
        let mut rejected = Vec::new();
        for action in self.supported.clone() {
            if !self.bindings.is_active(action) {
                // Raccourcis multicompte désactivés : la touche reste au jeu et aux autres
                // applications (voir doc de module).
                continue;
            }
            let hotkey = self.bindings.get(action).to_hotkey();
            match self.manager.register(hotkey) {
                Ok(()) => {
                    self.active.push(hotkey);
                    self.ids.insert(hotkey.id(), action);
                }
                Err(err) => {
                    tracing::warn!(
                        "[raccourcis] « {} » ({}) refusé par le système ({err}) — action sans raccourci cette session.",
                        action.label(),
                        self.bindings.label(action)
                    );
                    rejected.push(action);
                }
            }
        }
        rejected
    }

    fn unregister_active(&mut self) {
        for hotkey in self.active.drain(..) {
            if let Err(err) = self.manager.unregister(hotkey) {
                tracing::warn!("[raccourcis] échec du désenregistrement de {hotkey} ({err}).");
            }
        }
        self.ids.clear();
    }
}

/// Une touche de fonction (F1-F12) — la seule famille acceptée SANS modificateur
/// ([`Shortcut::is_valid`]), voir la doc de module pour pourquoi elle fait exception.
fn is_function_key(code: Code) -> bool {
    matches!(
        code,
        Code::F1
            | Code::F2
            | Code::F3
            | Code::F4
            | Code::F5
            | Code::F6
            | Code::F7
            | Code::F8
            | Code::F9
            | Code::F10
            | Code::F11
            | Code::F12
    )
}

/// Nom affiché d'une touche principale — symétrique de [`parse_key`] : tout ce que cette fonction
/// produit doit être relisible par elle (propriété vérifiée par le test `aller_retour_toutes_les_touches`).
fn key_label(code: Code) -> &'static str {
    match code {
        Code::KeyA => "A",
        Code::KeyB => "B",
        Code::KeyC => "C",
        Code::KeyD => "D",
        Code::KeyE => "E",
        Code::KeyF => "F",
        Code::KeyG => "G",
        Code::KeyH => "H",
        Code::KeyI => "I",
        Code::KeyJ => "J",
        Code::KeyK => "K",
        Code::KeyL => "L",
        Code::KeyM => "M",
        Code::KeyN => "N",
        Code::KeyO => "O",
        Code::KeyP => "P",
        Code::KeyQ => "Q",
        Code::KeyR => "R",
        Code::KeyS => "S",
        Code::KeyT => "T",
        Code::KeyU => "U",
        Code::KeyV => "V",
        Code::KeyW => "W",
        Code::KeyX => "X",
        Code::KeyY => "Y",
        Code::KeyZ => "Z",
        Code::Digit0 => "0",
        Code::Digit1 => "1",
        Code::Digit2 => "2",
        Code::Digit3 => "3",
        Code::Digit4 => "4",
        Code::Digit5 => "5",
        Code::Digit6 => "6",
        Code::Digit7 => "7",
        Code::Digit8 => "8",
        Code::Digit9 => "9",
        Code::F1 => "F1",
        Code::F2 => "F2",
        Code::F3 => "F3",
        Code::F4 => "F4",
        Code::F5 => "F5",
        Code::F6 => "F6",
        Code::F7 => "F7",
        Code::F8 => "F8",
        Code::F9 => "F9",
        Code::F10 => "F10",
        Code::F11 => "F11",
        Code::F12 => "F12",
        Code::ArrowUp => "Haut",
        Code::ArrowDown => "Bas",
        Code::ArrowLeft => "Gauche",
        Code::ArrowRight => "Droite",
        Code::Space => "Espace",
        Code::Enter => "Entrée",
        Code::Tab => "Tab",
        Code::Backspace => "Retour",
        Code::Delete => "Suppr",
        Code::Insert => "Inser",
        Code::Home => "Origine",
        Code::End => "Fin",
        Code::PageUp => "PagePrec",
        Code::PageDown => "PageSuiv",
        // Toute autre touche est refusée à la saisie (voir `code_from_egui`) et au parsing : ce
        // bras n'est atteignable qu'en construisant un `Shortcut` à la main dans du code futur.
        _ => "?",
    }
}

/// Lecture d'un nom de touche principale — accepte le libellé français produit par [`key_label`]
/// ainsi que le nom anglais de `global_hotkey` (`KeyW`, `Space`…), pour qu'un `config.toml` écrit
/// depuis la documentation de la dépendance reste lisible.
fn parse_key(token: &str) -> Option<Code> {
    let upper = token.to_uppercase();
    let normalized = upper.strip_prefix("KEY").unwrap_or(&upper);
    let code = match normalized {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "0" | "DIGIT0" => Code::Digit0,
        "1" | "DIGIT1" => Code::Digit1,
        "2" | "DIGIT2" => Code::Digit2,
        "3" | "DIGIT3" => Code::Digit3,
        "4" | "DIGIT4" => Code::Digit4,
        "5" | "DIGIT5" => Code::Digit5,
        "6" | "DIGIT6" => Code::Digit6,
        "7" | "DIGIT7" => Code::Digit7,
        "8" | "DIGIT8" => Code::Digit8,
        "9" | "DIGIT9" => Code::Digit9,
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        "HAUT" | "ARROWUP" | "UP" => Code::ArrowUp,
        "BAS" | "ARROWDOWN" | "DOWN" => Code::ArrowDown,
        "GAUCHE" | "ARROWLEFT" | "LEFT" => Code::ArrowLeft,
        "DROITE" | "ARROWRIGHT" | "RIGHT" => Code::ArrowRight,
        "ESPACE" | "SPACE" => Code::Space,
        "ENTRÉE" | "ENTREE" | "ENTER" => Code::Enter,
        "TAB" => Code::Tab,
        "RETOUR" | "BACKSPACE" => Code::Backspace,
        "SUPPR" | "DELETE" => Code::Delete,
        "INSER" | "INSERT" => Code::Insert,
        "ORIGINE" | "HOME" => Code::Home,
        "FIN" | "END" => Code::End,
        "PAGEPREC" | "PAGEUP" => Code::PageUp,
        "PAGESUIV" | "PAGEDOWN" => Code::PageDown,
        _ => return None,
    };
    Some(code)
}

/// `egui::Key` → `global_hotkey::Code`, limité aux touches que [`key_label`] sait nommer : ce que
/// l'utilisateur ne peut pas relire dans la liste, il ne doit pas pouvoir le binder.
fn code_from_egui(key: egui::Key) -> Option<Code> {
    use egui::Key as K;
    let code = match key {
        K::A => Code::KeyA,
        K::B => Code::KeyB,
        K::C => Code::KeyC,
        K::D => Code::KeyD,
        K::E => Code::KeyE,
        K::F => Code::KeyF,
        K::G => Code::KeyG,
        K::H => Code::KeyH,
        K::I => Code::KeyI,
        K::J => Code::KeyJ,
        K::K => Code::KeyK,
        K::L => Code::KeyL,
        K::M => Code::KeyM,
        K::N => Code::KeyN,
        K::O => Code::KeyO,
        K::P => Code::KeyP,
        K::Q => Code::KeyQ,
        K::R => Code::KeyR,
        K::S => Code::KeyS,
        K::T => Code::KeyT,
        K::U => Code::KeyU,
        K::V => Code::KeyV,
        K::W => Code::KeyW,
        K::X => Code::KeyX,
        K::Y => Code::KeyY,
        K::Z => Code::KeyZ,
        K::Num0 => Code::Digit0,
        K::Num1 => Code::Digit1,
        K::Num2 => Code::Digit2,
        K::Num3 => Code::Digit3,
        K::Num4 => Code::Digit4,
        K::Num5 => Code::Digit5,
        K::Num6 => Code::Digit6,
        K::Num7 => Code::Digit7,
        K::Num8 => Code::Digit8,
        K::Num9 => Code::Digit9,
        K::F1 => Code::F1,
        K::F2 => Code::F2,
        K::F3 => Code::F3,
        K::F4 => Code::F4,
        K::F5 => Code::F5,
        K::F6 => Code::F6,
        K::F7 => Code::F7,
        K::F8 => Code::F8,
        K::F9 => Code::F9,
        K::F10 => Code::F10,
        K::F11 => Code::F11,
        K::F12 => Code::F12,
        K::ArrowUp => Code::ArrowUp,
        K::ArrowDown => Code::ArrowDown,
        K::ArrowLeft => Code::ArrowLeft,
        K::ArrowRight => Code::ArrowRight,
        K::Space => Code::Space,
        K::Enter => Code::Enter,
        K::Tab => Code::Tab,
        K::Backspace => Code::Backspace,
        K::Delete => Code::Delete,
        K::Insert => Code::Insert,
        K::Home => Code::Home,
        K::End => Code::End,
        K::PageUp => Code::PageUp,
        K::PageDown => Code::PageDown,
        _ => return None,
    };
    Some(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les combinaisons par défaut restent celles qui étaient en dur avant ce module (voir la doc
    /// des anciennes constantes de `main.rs`) : une mise à jour de l'overlay ne doit pas changer
    /// les raccourcis sous les doigts d'un utilisateur qui n'a rien personnalisé. Les trois
    /// exceptions sont des demandes explicites de l'utilisateur, datées dans la doc des variantes
    /// concernées — c'est ce test qui les rend visibles.
    #[test]
    fn defauts_identiques_aux_anciennes_constantes() {
        let bindings = ShortcutBindings::default();
        assert_eq!(bindings.label(ShortcutAction::Toggle), "Ctrl+Shift+W");
        assert_eq!(bindings.label(ShortcutAction::Refresh), "Ctrl+Shift+R");
        assert_eq!(bindings.label(ShortcutAction::Options), "Ctrl+Shift+O");
        // Échangés l'un avec l'autre le 2026-09-18 (S comme « site », D comme « delete »).
        assert_eq!(bindings.label(ShortcutAction::Details), "Ctrl+Shift+S");
        assert_eq!(bindings.label(ShortcutAction::WatchlistAdd), "Ctrl+Shift+A");
        assert_eq!(
            bindings.label(ShortcutAction::WatchlistRemove),
            "Ctrl+Shift+D"
        );
        // Ctrl+Shift+E jusqu'au 2026-09-18 — T comme « toggle ».
        assert_eq!(bindings.label(ShortcutAction::CombatSide), "Ctrl+Shift+T");
        // Né le 2026-09-14 avec le switch de grandeur, combinaison choisie par l'utilisateur.
        assert_eq!(bindings.label(ShortcutAction::CombatMetric), "Ctrl+Shift+V");
        // Nées nues (2026-09-13), à la demande de l'utilisateur — voir la doc de ces variantes.
        assert_eq!(bindings.label(ShortcutAction::InvitePartner), "F1");
        assert_eq!(bindings.label(ShortcutAction::FollowPartner), "F2");
    }

    /// Désactivés, les raccourcis multicompte gardent leur combinaison mais ne comptent plus :
    /// ni actifs, ni en conflit avec une autre action qui prendrait leur touche.
    #[test]
    fn multicompte_desactive_sort_des_actions_actives_et_des_conflits() {
        let mut bindings = ShortcutBindings::default();
        assert!(bindings.multiaccount_enabled());
        assert!(bindings.is_active(ShortcutAction::InvitePartner));
        bindings.set_multiaccount_enabled(false);
        assert!(!bindings.is_active(ShortcutAction::InvitePartner));
        assert!(!bindings.is_active(ShortcutAction::FollowPartner));
        assert!(bindings.is_active(ShortcutAction::Toggle));
        assert_eq!(bindings.label(ShortcutAction::InvitePartner), "F1");
        bindings.set(
            ShortcutAction::Options,
            Shortcut::parse("F1").expect("touche de fonction nue valide"),
        );
        assert_eq!(bindings.conflict(), None);
        bindings.set_multiaccount_enabled(true);
        assert_eq!(
            bindings.conflict(),
            Some((ShortcutAction::Options, ShortcutAction::InvitePartner))
        );
    }

    /// Aucun conflit dans les défauts — sans quoi l'overlay démarrerait avec un raccourci non
    /// enregistrable.
    #[test]
    fn defauts_sans_conflit() {
        assert_eq!(ShortcutBindings::default().conflict(), None);
    }

    #[test]
    fn conflit_detecte_sur_doublon() {
        let mut bindings = ShortcutBindings::default();
        bindings.set(
            ShortcutAction::Details,
            ShortcutAction::Toggle.default_shortcut(),
        );
        assert_eq!(
            bindings.conflict(),
            Some((ShortcutAction::Toggle, ShortcutAction::Details))
        );
    }

    #[test]
    fn parse_tolere_casse_espaces_et_alias() {
        let attendu = Shortcut::new(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyW);
        assert_eq!(Shortcut::parse("Ctrl+Shift+W"), Some(attendu));
        assert_eq!(Shortcut::parse("  control + maj + keyw "), Some(attendu));
        // Ordre des modificateurs indifférent à la lecture, canonique à l'écriture.
        assert_eq!(
            Shortcut::parse("Shift+Ctrl+W")
                .map(|s| s.label())
                .as_deref(),
            Some("Ctrl+Shift+W")
        );
    }

    /// L'exception des touches de fonction (voir doc de module) vaut des deux côtés : la relecture
    /// d'un `config.toml` qui porte `invite_partner = "F1"` doit rendre exactement F1, pas le
    /// repli silencieux sur un défaut.
    #[test]
    fn parse_accepte_une_touche_de_fonction_nue() {
        assert_eq!(
            Shortcut::parse("F1"),
            Some(Shortcut::new(Modifiers::empty(), Code::F1))
        );
        assert_eq!(
            Shortcut::parse("F12").map(|s| s.label()).as_deref(),
            Some("F12")
        );
    }

    #[test]
    fn parse_refuse_les_combinaisons_inutilisables() {
        // Sans modificateur : volerait la touche au jeu (voir doc de module). L'exception ne vaut
        // que pour les touches de fonction, jamais pour une lettre ou un chiffre.
        assert_eq!(Shortcut::parse("W"), None);
        assert_eq!(Shortcut::parse("5"), None);
        // Touche principale absente, inconnue, ou en double.
        assert_eq!(Shortcut::parse("Ctrl+Shift"), None);
        assert_eq!(Shortcut::parse("Ctrl+Impr"), None);
        assert_eq!(Shortcut::parse("Ctrl+A+B"), None);
        assert_eq!(Shortcut::parse("Ctrl++W"), None);
    }

    /// Propriété attendue de la paire [`key_label`]/[`parse_key`] : tout libellé produit se relit.
    #[test]
    fn aller_retour_toutes_les_touches() {
        for code in [
            Code::KeyA,
            Code::KeyZ,
            Code::Digit0,
            Code::Digit9,
            Code::F1,
            Code::F12,
            Code::ArrowUp,
            Code::ArrowDown,
            Code::ArrowLeft,
            Code::ArrowRight,
            Code::Space,
            Code::Enter,
            Code::Tab,
            Code::Backspace,
            Code::Delete,
            Code::Insert,
            Code::Home,
            Code::End,
            Code::PageUp,
            Code::PageDown,
        ] {
            let shortcut = Shortcut::new(Modifiers::CONTROL, code);
            assert_eq!(
                Shortcut::parse(&shortcut.label()),
                Some(shortcut),
                "aller-retour cassé pour {code:?}"
            );
        }
    }

    /// La config n'écrase que ce qu'elle décrit correctement — le reste garde le défaut, jamais
    /// d'action sans raccourci (voir `from_config`).
    #[test]
    fn from_config_ignore_les_entrees_invalides() {
        let raw = BTreeMap::from([
            ("toggle".to_string(), "Ctrl+Alt+T".to_string()),
            ("options".to_string(), "pas un raccourci".to_string()),
            ("inconnue".to_string(), "Ctrl+Alt+Z".to_string()),
        ]);
        let bindings = ShortcutBindings::from_config(&raw);
        assert_eq!(bindings.label(ShortcutAction::Toggle), "Ctrl+Alt+T");
        assert_eq!(bindings.label(ShortcutAction::Options), "Ctrl+Shift+O");
    }

    /// Une config écrite AVANT le retrait des raccourcis de déconnexion et de fermeture reste
    /// lisible : leurs clés `disconnect` et `quit` sont ignorées, et tout le reste du fichier
    /// s'applique normalement.
    #[test]
    fn from_config_ignore_les_raccourcis_retires() {
        let raw = BTreeMap::from([
            ("disconnect".to_string(), "Ctrl+Alt+D".to_string()),
            ("quit".to_string(), "Ctrl+Shift+Q".to_string()),
            ("options".to_string(), "Ctrl+Alt+O".to_string()),
        ]);
        let bindings = ShortcutBindings::from_config(&raw);
        assert_eq!(bindings.label(ShortcutAction::Options), "Ctrl+Alt+O");
        for key in RETIRED_KEYS {
            assert!(
                !bindings.to_config().contains_key(key),
                "clé retirée « {key} » réécrite"
            );
        }
    }

    #[test]
    fn aller_retour_config_complet() {
        let mut bindings = ShortcutBindings::default();
        bindings.set(
            ShortcutAction::Options,
            Shortcut::new(Modifiers::CONTROL | Modifiers::ALT, Code::F9),
        );
        let raw = bindings.to_config();
        assert_eq!(raw.len(), ShortcutAction::ALL.len());
        assert_eq!(ShortcutBindings::from_config(&raw), bindings);
    }

    #[test]
    fn saisie_egui_exige_un_modificateur() {
        assert_eq!(
            Shortcut::from_egui(egui::Key::W, egui::Modifiers::NONE),
            None
        );
        // …sauf sur une touche de fonction : l'utilisateur doit pouvoir RÉASSIGNER un raccourci
        // multicompte à une autre touche nue depuis l'onglet, pas seulement hériter de F1/F2.
        assert_eq!(
            Shortcut::from_egui(egui::Key::F3, egui::Modifiers::NONE)
                .map(|s| s.label())
                .as_deref(),
            Some("F3")
        );
        assert_eq!(
            Shortcut::from_egui(egui::Key::W, egui::Modifiers::CTRL)
                .map(|s| s.label())
                .as_deref(),
            Some("Ctrl+W")
        );
        // Touche sans équivalent `global_hotkey` : refusée plutôt que bindée sur autre chose.
        assert_eq!(
            Shortcut::from_egui(egui::Key::Escape, egui::Modifiers::CTRL),
            None
        );
    }
}
