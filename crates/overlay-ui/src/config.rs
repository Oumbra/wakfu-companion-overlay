//! Persistance de la configuration utilisateur de l'overlay — §5.1 du plan d'architecture : le
//! chemin de `wakfu.log` doit être « toujours surchargeable par la config et par un sélecteur de
//! fichier dans l'UI ». Un seul fichier TOML pour tous les réglages locaux (voir
//! [`OverlayConfig`]) plutôt qu'un fichier par réglage — il en porte deux depuis le 2026-09-13 :
//! le chemin de log et l'affichage permanent du panneau Combat.
//!
//! **Ce qui vit ici, et ce qui n'y vit pas** : la config locale porte ce qui dépend de la MACHINE
//! (un chemin de fichier) ou de la fenêtre de jeu qu'on a sous les yeux (l'encombrement de
//! l'overlay à l'écran). Tout ce qui appartient au JOUEUR — liste suivie, profil d'alertes —
//! passe par le compte (`overlay_sync::client::fetch_settings`), jamais par ce fichier.
//!
//! **Priorité de résolution du chemin au démarrage** (voir `resolve_log_path`, partagé par
//! `main.rs` et `bin/wakfu-companion-overlay-x11.rs`) : argument CLI explicite > chemin sauvegardé
//! ici (choisi via la modale Options, `panels::options_modal`) > découverte automatique
//! (`overlay_ingest::discovery::discover`).
//!
//! Emplacement du fichier : `directories::ProjectDirs` (déjà une dépendance de ce crate, utilisée
//! par ailleurs pour le repli du jeton de compte natif — voir §7.2 du plan) —
//! `%APPDATA%\Oumbra\wakfu-companion-overlay\config\config.toml` sous Windows,
//! `$XDG_CONFIG_HOME/wakfu-companion-overlay/config.toml` (ou `~/.config/...`) sous Linux.
//! Chargement/sauvegarde **best-effort** : un échec (droits insuffisants, disque plein, fichier
//! corrompu) n'est jamais fatal, seulement journalisé — un utilisateur dans ce cas retombe
//! simplement sur la découverte automatique à chaque lancement, comme avant l'existence de ce
//! module.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::shortcuts::ShortcutBindings;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayConfig {
    /// Chemin explicite de `wakfu.log`, choisi par l'utilisateur via la modale Options — voir
    /// doc de module pour l'ordre de priorité au démarrage et
    /// `engine_thread::EngineCommand::ChangeLogPath` pour le rechargement à chaud (sans
    /// redémarrer l'overlay) quand ce réglage change en cours de session.
    pub log_path: Option<PathBuf>,
    /// Le panneau Combat reste-t-il affiché **en dehors des combats** ?
    ///
    /// `false` par défaut (demande du 2026-09-13) : la fenêtre Combat n'apparaît qu'au début d'un
    /// combat et se referme quand il est terminé — le reste du temps, rien ne recouvre le jeu.
    /// `true` restaure le comportement d'origine, une fenêtre affichée en permanence (avec son
    /// « Aucun combat pour l'instant. », voir `panels::combat::show`) : c'est un choix
    /// d'encombrement à l'écran, laissé à l'utilisateur.
    ///
    /// `#[serde(default)]` : un `config.toml` écrit avant ce champ reste lisible, et retombe donc
    /// sur le nouveau défaut plutôt que de faire échouer tout le chargement (voir doc de module —
    /// un parsing en échec repart de `OverlayConfig::default()`, chemin de log compris).
    #[serde(default)]
    pub combat_always_visible: bool,
    /// Le panneau Combat est-il posé **à droite** de la fenêtre de jeu plutôt qu'à gauche ?
    ///
    /// `false` par défaut : l'overlay Combat est collé au bord GAUCHE du client depuis toujours
    /// (voir `main.rs::App::anchor_position`). `true` (demande utilisateur du 2026-09-17 —
    /// « permettre à l'utilisateur d'afficher l'overlay combat à droite plutôt qu'à gauche ») le
    /// colle au bord DROIT, et **retourne toute son interface en miroir vertical** pour qu'elle
    /// s'ouvre vers l'intérieur de l'écran plutôt que de lui tourner le dos — portraits, images de
    /// monstre, icônes et images de sort exceptées, qui restent à l'endroit (voir
    /// `crate::mirror`).
    ///
    /// Même nature que [`Self::combat_always_visible`] : un choix d'encombrement à l'écran, donc
    /// une config LOCALE (l'écran qu'on a devant soi, pas le joueur) et non un réglage de compte.
    /// `#[serde(default)]` pour la même raison qu'elle — une config écrite avant ce champ reste
    /// lisible et garde le panneau à gauche.
    #[serde(default)]
    pub combat_on_right: bool,
    /// Durée d'affichage de la carte d'alerte de **chat**, en secondes (onglet « Chat », voir
    /// `panels::chat_tab::ChatToastSettings`). **Ici et non au compte**, par exception au principe
    /// de la doc de module : ce réglage n'a pas d'équivalent web, et le serveur n'accepte que des
    /// clés connues — le jour où il en porte une, il y migre. `None` = défaut.
    #[serde(default)]
    pub chat_alert_duration_seconds: Option<f32>,
    /// La carte d'alerte de chat ne se ferme qu'à la main — même exception, même raison.
    #[serde(default)]
    pub chat_alert_manual_close: bool,
    /// Durée d'affichage de la carte de **décompte arrivé à zéro** du Suivi, en secondes — ligne
    /// « Fermeture automatique des notifications de décompte » de la section « Suivi » de l'onglet
    /// « Paramètres » (2026-09-16, voir `panels::suivi_tab::CountdownToastSettings`).
    ///
    /// **Ici et non au compte**, même exception et même raison que
    /// [`Self::chat_alert_duration_seconds`] : ce réglage n'a pas d'équivalent web, et le serveur
    /// n'accepte que des clés connues. `None` = défaut (5 s).
    ///
    /// Avant cette clé, la carte du décompte empruntait la durée du **profil d'alertes de
    /// ramassage** (`AlertProfile`, descendue du compte) : régler l'une réglait l'autre.
    #[serde(default)]
    pub countdown_alert_duration_seconds: Option<f32>,
    /// La carte de décompte ne se ferme qu'à la main — même exception, même raison.
    #[serde(default)]
    pub countdown_alert_manual_close: bool,
    /// Prévenir par une **notification du système** qu'un personnage du joueur doit jouer
    /// (section « Combat » de l'onglet Paramètres, 2026-09-14).
    ///
    /// `false` par défaut : une notification système est le seul réglage de l'overlay qui déborde
    /// de l'écran de jeu (barre de notifications, téléphone apparié sous Windows…), elle ne
    /// s'active donc que si on la demande.
    ///
    /// **Locale et non au compte**, comme ses voisines : elle dépend de la machine — un multicompte
    /// sur un seul écran n'a pas les mêmes besoins que deux écrans côte à côte, et c'est la même
    /// machine qui porte ou non un démon de notifications.
    ///
    /// `#[serde(default)]` : un `config.toml` écrit avant ce champ reste lisible, voir
    /// `combat_always_visible`.
    #[serde(default)]
    pub turn_notification: bool,
    /// La notification de tour s'affiche **sans son** — case « Couper le son des notifications »,
    /// sous `turn_notification` dont elle dépend (grisée tant que celle-ci est décochée, demande
    /// du 2026-09-14). Locale pour la même raison qu'elle : c'est la machine, et l'endroit où elle
    /// est, qui décident si un son est bienvenu. `#[serde(default)]` comme ses voisines.
    #[serde(default)]
    pub turn_notification_muted: bool,
    /// La fonctionnalité **Suivi** est-elle active ? — case « Activer le suivi », tout en haut de
    /// l'onglet du même nom (2026-09-15).
    ///
    /// Décochée, l'onglet entier est grisé et inerte (voir `panels::suivi_tab::show`), le bandeau
    /// in-game n'affiche plus aucune tuile suivie et l'alerte de décompte à zéro ne se déclenche
    /// plus (voir `engine_thread::FeatureToggles`). **Le moteur, lui, continue de compter** : rien
    /// n'est remis à zéro ni effacé du compte, et réactiver la case retrouve la liste et ses
    /// compteurs exactement où ils en étaient. Désactiver une fonctionnalité met son affichage en
    /// sourdine, ça ne détruit pas de données.
    ///
    /// **Locale et non au compte**, comme ses voisines : ce qu'on accepte de voir par-dessus son
    /// jeu dépend de l'écran qu'on a devant soi — un multicompte qui n'ouvre l'overlay que pour
    /// les combats sur une machine peut vouloir tout le Suivi sur l'autre. Le serveur, de plus,
    /// n'accepte que des clés connues (même raison que `chat_alert_duration_seconds`).
    ///
    /// **`true` par défaut, y compris pour une config écrite avant ce champ** : d'où
    /// `#[serde(default = "actif")]` et non le simple `#[serde(default)]` de ses voisines, qui
    /// vaudrait `false` et couperait le Suivi de tous ceux qui l'utilisent déjà au premier
    /// lancement de cette version.
    #[serde(default = "actif")]
    pub suivi_enabled: bool,
    /// La fonctionnalité **Alertes** est-elle active ? — case « Activer la surveillance du drop ». Décochée,
    /// l'onglet est grisé et inerte, et le ramassage d'un objet à son activé ne joue plus rien et
    /// n'affiche plus de carte. Même politique que [`Self::suivi_enabled`] pour le reste (liste
    /// conservée au compte, `true` par défaut).
    #[serde(default = "actif")]
    pub alerts_enabled: bool,
    /// La fonctionnalité **Recherche de chat** est-elle active ? — case « Activer la recherche ».
    /// Décochée, l'onglet « Chat » est grisé et inerte, et aucun message trouvé ne fait plus
    /// sonner l'overlay ni n'affiche de carte. Même politique que [`Self::suivi_enabled`].
    #[serde(default = "actif")]
    pub chat_enabled: bool,
    /// Le **détail des combats** est-il actif ? — case « Activer le détail des combats », en tête
    /// de la section « Combat » de l'onglet « Paramètres » (2026-09-15).
    ///
    /// Décochée, **aucune fenêtre Combat n'est montrée**, combat en cours compris (voir
    /// `panels::combat::should_show`) : c'est l'interrupteur de la fonctionnalité entière, pas un
    /// réglage d'encombrement comme [`Self::combat_always_visible`] — que cette case commande
    /// d'ailleurs, et grise, dans la fenêtre Options.
    ///
    /// **Le moteur continue de compter** : les combats sont toujours mesurés et synchronisés vers
    /// le compte, exactement comme pour les trois interrupteurs ci-dessus. Recocher la case
    /// retrouve le panneau en l'état, sans relire le log.
    ///
    /// **Locale et non au compte**, comme ses voisines, et `#[serde(default = "actif")]` pour la
    /// même raison qu'elles : une config écrite avant ce champ garde son panneau de combat.
    #[serde(default = "actif")]
    pub combat_enabled: bool,
    /// Le **suivi des sorts** est-il actif ? — case « Activer le suivi des sorts », sous la
    /// précédente **dont elle dépend** (grisée tant que le détail des combats est décoché, sans
    /// changer de valeur : on la retrouve telle quelle en le rallumant).
    ///
    /// Décochée, le panneau Combat garde ses portraits et ses barres et perd le bloc « ligne de
    /// sorts » (`panels::combat_spell_block`) ainsi que les marques qu'il pose sur les médaillons.
    /// Même politique que [`Self::combat_enabled`] pour le reste.
    #[serde(default = "actif")]
    pub spells_enabled: bool,
    /// La bande **Récap de session** est-elle active ? — case « Activer le récap de session », en
    /// tête de la section « Recap » de l'onglet « Paramètres » (2026-09-16).
    ///
    /// Décochée, la bande XP / Kamas / Combats / Challenges / Durée posée en haut à gauche de la
    /// fenêtre de jeu n'est plus montrée (voir `panels::recap`). Même politique que ses voisines
    /// pour le reste : le moteur continue de compter, rien n'est effacé, et
    /// `#[serde(default = "actif")]` garde la bande allumée pour une config écrite avant ce champ.
    #[serde(default = "actif")]
    pub recap_enabled: bool,
    /// La bande Récap affiche-t-elle la **durée de la session** ? — case « Afficher la durée de
    /// la session », sous l'interrupteur de la bande dont elle dépend (2026-09-16, tard). Voir
    /// `panels::recap::RecapCells` ; même politique que ses voisines, `actif` par défaut.
    #[serde(default = "actif")]
    pub recap_duration_enabled: bool,
    /// La bande Récap affiche-t-elle les **combats** (gagnés − perdus) ? — case « Afficher les
    /// combats », même famille que [`Self::recap_duration_enabled`].
    #[serde(default = "actif")]
    pub recap_fights_enabled: bool,
    /// La bande Récap affiche-t-elle les **challenges** (réussis − échoués) ? — case « Afficher
    /// les challenges », même famille que [`Self::recap_duration_enabled`].
    #[serde(default = "actif")]
    pub recap_challenges_enabled: bool,
    /// **Reprendre la session du Récap après une pause** — case « Reprendre la session après une
    /// pause de moins de … min » de la section « Recap » (2026-09-17, voir
    /// `crate::recap_session::ResumeSettings`). Décochée, chaque retour dans le jeu repart de
    /// zéro. Cochée pour une config écrite avant ce champ.
    ///
    /// **Ici et non au compte**, même exception et même raison que
    /// [`Self::countdown_alert_duration_seconds`] : pas d'équivalent web.
    #[serde(default = "actif")]
    pub recap_resume_enabled: bool,
    /// La tolérance de pause, en minutes — `None` = défaut
    /// (`crate::recap_session::DEFAULT_RESUME_MINUTES`, 60). Bornée à la lecture.
    #[serde(default)]
    pub recap_resume_minutes: Option<i64>,
    /// **Où l'utilisateur a posé la bande Récap** (2026-09-17) : abscisse du BLOC en pixels
    /// physiques depuis le bord gauche de la fenêtre de jeu. `None` = jamais déplacée, donc
    /// l'ancrage d'origine sous les boutons du client (`main.rs::GAME_RECAP_EDGE_MARGIN_PX` /
    /// `GAME_RECAP_TOP_MARGIN_PX`) — ce qui n'est pas la même chose que « posée exactement au
    /// défaut » : c'est ce que l'aimantation du glisser-déposer rétablit, et le bouton
    /// « Replacer au défaut » de la section « Recap » avec elle.
    ///
    /// **Relative à la fenêtre de jeu, jamais à l'écran** : le client se déplace, change de
    /// taille, passe d'un écran à l'autre ; seul un décalage depuis son coin garde la bande là
    /// où l'utilisateur l'a accrochée. L'origine est celle de l'ancrage
    /// (`GameRect::left`/`client_top`, donc la zone cliente — le client dessine sa fausse barre
    /// de titre dedans, voir `GameRect::client_top`).
    ///
    /// **C'est le coin du BLOC, pas celui de sa fenêtre OS** : celle-ci commence
    /// `render_content::RECAP_TOOLTIP_RESERVE` px plus haut (la place de ses infobulles). Un
    /// fichier qu'on ouvre à la main dit ainsi où l'on voit la bande, pas où commence une marge
    /// invisible.
    ///
    /// **Ici et non au compte**, comme ses voisines : une position à l'écran dépend de la
    /// fenêtre de jeu qu'on a sous les yeux, pas du joueur — et le serveur n'accepte que des
    /// clés connues (même raison que `chat_alert_duration_seconds`).
    ///
    /// Deux clés PLATES plutôt qu'une table `[recap_position]`, pour la même raison que les
    /// interrupteurs de fonctionnalité : un fichier qu'on ouvre à la main se lit mieux sans
    /// table intermédiaire, et une table ne pourrait pas se glisser avant `[shortcuts]` sans
    /// déplacer les clés de racine qui la suivent.
    #[serde(default)]
    pub recap_position_x: Option<i32>,
    /// Ordonnée du bloc Récap, même origine et même politique que [`Self::recap_position_x`] —
    /// les deux vont toujours ensemble (voir [`OverlayConfig::recap_position`], qui ne rend une
    /// position que si les deux sont là).
    #[serde(default)]
    pub recap_position_y: Option<i32>,
    /// **La bande Récap est-elle verrouillée ?** (2026-09-17) — le cadenas de sa rangée
    /// d'actions (`panels::recap::RecapChrome::locked`). Verrouillée, elle ne se saisit plus à la
    /// souris et le curseur redevient celui du système au-dessus d'elle.
    ///
    /// **Le défaut est `true`, verrouillée** : tout le fond de la bande est une poignée (elle n'a
    /// pas la place d'en porter une dédiée, voir `panels::recap`), donc non verrouillée, le
    /// moindre clic dessus en mode interactif la déplace. On la déverrouille pour la ranger, on
    /// la reverrouille ensuite — c'est le geste que les deux glyphes racontent. Une config écrite
    /// avant cette clé se verrouille donc au premier lancement, y compris pour une bande déjà
    /// déplacée : sa position, elle, ne bouge pas.
    ///
    /// **Locale et non au compte**, comme la position qu'elle protège : le serveur n'accepte que
    /// des clés connues (même raison que `chat_alert_duration_seconds`).
    #[serde(default = "actif")]
    pub recap_locked: bool,
    /// L'alerte de **décompte à zéro** du Suivi est-elle muette ? — case « Couper le son des
    /// notifications », sous la ligne « Tester le son de l'alerte » de l'onglet « Suivi »
    /// (2026-09-15, voir `panels::notifications`).
    ///
    /// Cochée, le décompte arrivé à zéro affiche toujours sa carte par-dessus le jeu : c'est le
    /// SON qui se tait, pas la fonctionnalité — celle-ci a sa propre clé ([`Self::suivi_enabled`]),
    /// qui coupe les deux canaux.
    ///
    /// **Locale et non au compte**, comme ses voisines : un son bienvenu au casque ne l'est pas
    /// forcément sur la machine du salon, et le serveur n'accepte que des clés connues (même
    /// raison que `chat_alert_duration_seconds`).
    ///
    /// `#[serde(default)]` — et non `default = "actif"` comme les trois drapeaux ci-dessus : le
    /// défaut d'une sourdine est d'être LEVÉE, c'est-à-dire `false`.
    #[serde(default)]
    pub suivi_alert_muted: bool,
    /// L'alerte de **message trouvé** du Chat est-elle muette ? — même case, dans l'onglet
    /// « Chat », et même politique que [`Self::suivi_alert_muted`] : la carte reste, le son part.
    #[serde(default)]
    pub chat_alert_muted: bool,
    /// **Installer automatiquement les mises à jour au démarrage** — case de la section « Mise à
    /// jour » de l'onglet « Paramètres » (2026-09-15, `docs/plan-mise-a-jour.md` §8.2, décision 3
    /// du mainteneur : oui par défaut). Cochée, une version plus récente trouvée derrière l'écran
    /// de chargement est téléchargée, installée et l'overlay se relance avant d'ouvrir le moindre
    /// overlay de jeu ; décochée, l'écran de chargement se contente de la signaler et le bouton
    /// « Mettre à jour » de cette même section fait le reste à la demande. Une mise à jour
    /// **obligatoire** (`minimumVersion` du manifeste) s'installe dans les deux cas.
    ///
    /// **Locale et non au compte**, comme ses voisines : c'est la machine qui se met à jour, pas le
    /// compte. `#[serde(default = "actif")]` — le défaut d'une mise à jour automatique est d'être
    /// active, une config écrite avant ce champ comprise.
    #[serde(default = "actif")]
    pub auto_update: bool,
    /// **Le lancement au démarrage de l'ordinateur a-t-il reçu son défaut ?** — le seul champ de
    /// cette structure qui ne porte pas un réglage mais un JALON (2026-09-16).
    ///
    /// Le réglage lui-même vit dans le système, pas ici (voir `crate::autostart`, doc de module) ;
    /// mais « actif par défaut » (demande utilisateur du 2026-09-16) suppose de savoir si
    /// l'overlay a déjà eu l'occasion de s'inscrire une fois. Sans ce jalon, chaque lancement
    /// réinscrirait l'overlay que l'utilisateur vient de décocher — le système ne distingue pas
    /// « jamais inscrit » de « retiré exprès ». `false` = jamais fait ; `true` dès que
    /// `autostart::enable_by_default_once` est passé, et pour toujours.
    ///
    /// `#[serde(default)]` : une config écrite avant ce champ vaut « jamais fait » — c'est ce qui
    /// inscrit aussi les installations existantes, une fois, à leur premier lancement de cette
    /// version. **Les hôtes doivent le réécrire à `true`** à chaque sauvegarde (ils rebâtissent la
    /// config depuis leurs champs, `..Default::default()` le remettrait à `false`).
    #[serde(default)]
    pub autostart_initialized: bool,
    /// Table `[shortcuts]` : `clé d'action` -> `combinaison` (`toggle = "Ctrl+Shift+W"`, voir
    /// `shortcuts::ShortcutAction::key`/`shortcuts::Shortcut::label`), alimentée par l'onglet
    /// « Raccourcis » de la fenêtre Options (2026-09-13).
    ///
    /// Volontairement une map de CHAÎNES plutôt que des champs typés : une clé inconnue (config
    /// écrite par une version ultérieure de l'overlay) ou une combinaison illisible (édition à la
    /// main malheureuse) doit être ignorée sans faire échouer la lecture de TOUT le fichier — ce
    /// que des champs typés `Shortcut` interdiraient (`toml::from_str` échouerait d'un bloc, et
    /// l'utilisateur perdrait aussi son `log_path`). La conversion tolérante est faite par
    /// [`ShortcutBindings::from_config`].
    ///
    /// `#[serde(default)]` : même raison que `combat_always_visible` ci-dessus — un `config.toml`
    /// écrit avant ce champ reste lisible (table absente = tous les raccourcis par défaut).
    ///
    /// **Doit rester le DERNIER champ de cette structure** : `toml::to_string_pretty` écrit les
    /// champs dans l'ordre de déclaration, et une table TOML ne peut pas être suivie d'une clé de
    /// racine (`log_path` après `[shortcuts]` appartiendrait à la table). Couvert par
    /// `aller_retour_toml_avec_raccourcis_personnalises`.
    #[serde(default)]
    pub shortcuts: BTreeMap<String, String>,
}

/// Valeur par défaut des drapeaux de fonctionnalité — **une fonction, parce que
/// `#[serde(default)]` ne sait produire que `bool::default()`, c'est-à-dire `false`**. Voir
/// `OverlayConfig::suivi_enabled` : le défaut d'une fonctionnalité est d'être active.
fn actif() -> bool {
    true
}

impl Default for OverlayConfig {
    /// Écrit à la main, et non dérivé, pour la seule raison des drapeaux de fonctionnalité :
    /// `bool::default()` vaut `false`, alors qu'une fonctionnalité non réglée est ACTIVE. Tous les
    /// autres champs gardent le défaut que la dérivation leur donnait.
    fn default() -> Self {
        Self {
            log_path: None,
            combat_always_visible: false,
            combat_on_right: false,
            chat_alert_duration_seconds: None,
            chat_alert_manual_close: false,
            countdown_alert_duration_seconds: None,
            countdown_alert_manual_close: false,
            turn_notification: false,
            turn_notification_muted: false,
            suivi_enabled: actif(),
            alerts_enabled: actif(),
            chat_enabled: actif(),
            combat_enabled: actif(),
            spells_enabled: actif(),
            recap_enabled: actif(),
            recap_duration_enabled: actif(),
            recap_fights_enabled: actif(),
            recap_challenges_enabled: actif(),
            recap_resume_enabled: actif(),
            recap_resume_minutes: None,
            recap_position_x: None,
            recap_position_y: None,
            recap_locked: actif(),
            suivi_alert_muted: false,
            chat_alert_muted: false,
            auto_update: actif(),
            autostart_initialized: false,
            shortcuts: BTreeMap::new(),
        }
    }
}

impl OverlayConfig {
    /// Raccourcis effectifs de cette config — défauts inclus pour toute action absente/illisible,
    /// voir [`ShortcutBindings::from_config`].
    pub fn shortcuts(&self) -> ShortcutBindings {
        ShortcutBindings::from_config(&self.shortcuts)
    }

    /// Remplace la table `[shortcuts]` par l'intégralité de `bindings` — appelée à la validation de
    /// la fenêtre Options (voir `panels::options_modal`), jamais à chaque frame.
    pub fn set_shortcuts(&mut self, bindings: &ShortcutBindings) {
        self.shortcuts = bindings.to_config();
    }

    /// Réglages de la carte de chat effectifs — défaut pour une config qui ne les porte pas.
    pub fn chat_toast(&self) -> crate::panels::chat_tab::ChatToastSettings {
        let mut toast = crate::panels::chat_tab::ChatToastSettings {
            manual_close: self.chat_alert_manual_close,
            ..Default::default()
        };
        if let Some(seconds) = self.chat_alert_duration_seconds {
            toast.set_duration(seconds);
        }
        toast
    }

    /// Reporte les réglages de la carte de chat dans la config.
    pub fn set_chat_toast(&mut self, toast: crate::panels::chat_tab::ChatToastSettings) {
        self.chat_alert_duration_seconds = Some(toast.duration_seconds);
        self.chat_alert_manual_close = toast.manual_close;
    }

    /// Réglages de la carte de décompte effectifs — défaut pour une config qui ne les porte pas,
    /// exactement comme [`Self::chat_toast`].
    pub fn countdown_toast(&self) -> crate::panels::suivi_tab::CountdownToastSettings {
        let mut toast = crate::panels::suivi_tab::CountdownToastSettings {
            manual_close: self.countdown_alert_manual_close,
            ..Default::default()
        };
        if let Some(seconds) = self.countdown_alert_duration_seconds {
            toast.set_duration(seconds);
        }
        toast
    }

    /// Reporte les réglages de la carte de décompte dans la config.
    pub fn set_countdown_toast(&mut self, toast: crate::panels::suivi_tab::CountdownToastSettings) {
        self.countdown_alert_duration_seconds = Some(toast.duration_seconds);
        self.countdown_alert_manual_close = toast.manual_close;
    }

    /// Le réglage de reprise de la session du Récap effectif — défaut pour une config qui ne le
    /// porte pas, exactement comme [`Self::countdown_toast`].
    pub fn recap_resume(&self) -> crate::recap_session::ResumeSettings {
        let mut resume = crate::recap_session::ResumeSettings {
            enabled: self.recap_resume_enabled,
            ..Default::default()
        };
        if let Some(minutes) = self.recap_resume_minutes {
            resume.set_minutes(minutes);
        }
        resume
    }

    /// Reporte le réglage de reprise de la session du Récap dans la config.
    pub fn set_recap_resume(&mut self, resume: crate::recap_session::ResumeSettings) {
        self.recap_resume_enabled = resume.enabled;
        self.recap_resume_minutes = Some(resume.minutes);
    }

    /// Où l'utilisateur a posé la bande Récap, `None` tant qu'il ne l'a pas déplacée — voir
    /// [`Self::recap_position_x`].
    ///
    /// **Les deux coordonnées ou aucune** : une config à moitié écrite (édition à la main
    /// malheureuse, fichier tronqué) retombe sur l'ancrage d'origine plutôt que de coller la
    /// bande contre un bord, et surtout plutôt que de faire échouer la lecture de tout le
    /// fichier — la même tolérance que la table `[shortcuts]`.
    pub fn recap_position(&self) -> Option<(i32, i32)> {
        self.recap_position_x.zip(self.recap_position_y)
    }

    /// Reporte la position de la bande Récap dans la config — `None` efface les deux clés et
    /// rend son ancrage d'origine à la bande (bouton « Replacer au défaut », et aimantation du
    /// glisser-déposer). Appelée au relâchement du bouton de la souris, jamais à chaque frame.
    pub fn set_recap_position(&mut self, position: Option<(i32, i32)>) {
        (self.recap_position_x, self.recap_position_y) = match position {
            Some((x, y)) => (Some(x), Some(y)),
            None => (None, None),
        };
    }

    /// Les trois interrupteurs de fonctionnalité de cette config — voir
    /// [`crate::panels::feature_switch::FeatureToggles`], qui les fait voyager ensemble jusqu'au
    /// thread Engine. Les champs restent PLATS dans le TOML (`suivi_enabled = false`) : un fichier
    /// qu'on ouvre à la main se lit mieux sans table intermédiaire, et une table ne pourrait de
    /// toute façon pas se glisser avant `[shortcuts]` sans déplacer les clés de racine.
    pub fn features(&self) -> crate::panels::feature_switch::FeatureToggles {
        crate::panels::feature_switch::FeatureToggles {
            suivi: self.suivi_enabled,
            alerts: self.alerts_enabled,
            chat: self.chat_enabled,
            combat: self.combat_enabled,
            spells: self.spells_enabled,
            recap: self.recap_enabled,
            recap_cells: crate::panels::recap::RecapCells {
                duration: self.recap_duration_enabled,
                fights: self.recap_fights_enabled,
                challenges: self.recap_challenges_enabled,
            },
        }
    }

    /// Reporte les trois interrupteurs dans la config — appelée à la validation de la fenêtre
    /// Options, jamais à chaque frame.
    pub fn set_features(&mut self, features: crate::panels::feature_switch::FeatureToggles) {
        self.suivi_enabled = features.suivi;
        self.alerts_enabled = features.alerts;
        self.chat_enabled = features.chat;
        self.combat_enabled = features.combat;
        self.spells_enabled = features.spells;
        self.recap_enabled = features.recap;
        self.recap_duration_enabled = features.recap_cells.duration;
        self.recap_fights_enabled = features.recap_cells.fights;
        self.recap_challenges_enabled = features.recap_cells.challenges;
    }

    /// Les deux sourdines de cette config — voir [`crate::panels::notifications::AlertMutes`], qui les
    /// fait voyager ensemble jusqu'au thread Engine comme `features` fait pour les interrupteurs.
    /// Champs PLATS dans le TOML pour la même raison qu'eux.
    pub fn alert_mutes(&self) -> crate::panels::notifications::AlertMutes {
        crate::panels::notifications::AlertMutes {
            suivi: self.suivi_alert_muted,
            chat: self.chat_alert_muted,
        }
    }

    /// Reporte les deux sourdines dans la config — appelée à la validation de la fenêtre Options,
    /// jamais à chaque frame.
    pub fn set_alert_mutes(&mut self, mutes: crate::panels::notifications::AlertMutes) {
        self.suivi_alert_muted = mutes.suivi;
        self.chat_alert_muted = mutes.chat;
    }
}

fn project_dirs() -> Option<directories::ProjectDirs> {
    // Mêmes qualifieurs que le reste du dépôt (organisation GitHub `Oumbra`, voir
    // `overlay_sync::token_store` pour le même motif appliqué au jeton de compte natif).
    directories::ProjectDirs::from("com", "Oumbra", "wakfu-companion-overlay")
}

fn config_file() -> Option<PathBuf> {
    project_dirs().map(|dirs| dirs.config_dir().join("config.toml"))
}

/// Charge la config persistée — `OverlayConfig::default()` (donc `log_path: None`) au tout premier
/// lancement (fichier absent) ou si sa lecture/son parsing échoue : jamais fatal, voir doc de
/// module.
pub fn load() -> OverlayConfig {
    let Some(path) = config_file() else {
        tracing::warn!(
            "[config] répertoire de configuration introuvable — réglages non persistés cette session."
        );
        return OverlayConfig::default();
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return OverlayConfig::default(),
        Err(err) => {
            tracing::warn!(
                "[config] échec de lecture de {} ({err}) — réglages par défaut.",
                path.display()
            );
            return OverlayConfig::default();
        }
    };
    match toml::from_str(&raw) {
        Ok(config) => config,
        Err(err) => {
            tracing::warn!(
                "[config] échec de lecture de {} ({err}) — réglages par défaut.",
                path.display()
            );
            OverlayConfig::default()
        }
    }
}

/// Sauvegarde `config` sur disque — best-effort (voir doc de module), crée le répertoire parent
/// si besoin. Appelée à la validation de la modale Options (jamais à chaque frame, voir
/// `panels::options_modal::OptionsModalAction::Validate`) et, une seule fois par installation,
/// au démarrage pour poser le jalon [`OverlayConfig::autostart_initialized`]
/// (`crate::autostart::enable_by_default_once`).
pub fn save(config: &OverlayConfig) {
    let Some(path) = config_file() else {
        tracing::warn!(
            "[config] répertoire de configuration introuvable — réglages non sauvegardés."
        );
        return;
    };
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::warn!(
                "[config] échec de création de {} ({err}) — réglages non sauvegardés.",
                parent.display()
            );
            return;
        }
    }
    let raw = match toml::to_string_pretty(config) {
        Ok(raw) => raw,
        Err(err) => {
            tracing::warn!("[config] échec de sérialisation ({err}) — réglages non sauvegardés.");
            return;
        }
    };
    if let Err(err) = std::fs::write(&path, raw) {
        tracing::warn!(
            "[config] échec d'écriture de {} ({err}) — réglages non sauvegardés.",
            path.display()
        );
    }
}

/// Résout le chemin de `wakfu.log` à utiliser au démarrage — voir doc de module pour l'ordre de
/// priorité. Partagée par `main.rs` (Windows) et `bin/wakfu-companion-overlay-x11.rs` (Linux), qui
/// appellent chacun `env::args().nth(1)` pour l'argument CLI (rien d'OS-spécifique là-dedans, mais
/// `std::env::args` reste appelé au point d'entrée de chaque binaire plutôt qu'ici, pour ne pas
/// faire dépendre ce module de la façon dont chaque binaire construit ses arguments).
///
/// L'échec de toute résolution (aucun argument, rien en config, découverte automatique
/// infructueuse) n'est PAS traité ici — voir la doc de `overlay_ingest::discovery::discover` :
/// c'est à l'appelant de décider (message d'erreur + sortie côté CLI headless historique, futur
/// sélecteur de fichier de la modale Options côté UI).
pub fn resolve_log_path(cli_arg: Option<PathBuf>, config: &OverlayConfig) -> Option<PathBuf> {
    cli_arg
        .or_else(|| config.log_path.clone())
        .or_else(overlay_ingest::discovery::discover)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_priorise_argument_cli() {
        let config = OverlayConfig {
            log_path: Some(PathBuf::from("/config/wakfu.log")),
            ..Default::default()
        };
        let resolved = resolve_log_path(Some(PathBuf::from("/cli/wakfu.log")), &config);
        assert_eq!(resolved, Some(PathBuf::from("/cli/wakfu.log")));
    }

    #[test]
    fn resolve_retombe_sur_la_config_sans_argument_cli() {
        let config = OverlayConfig {
            log_path: Some(PathBuf::from("/config/wakfu.log")),
            ..Default::default()
        };
        let resolved = resolve_log_path(None, &config);
        assert_eq!(resolved, Some(PathBuf::from("/config/wakfu.log")));
    }

    /// Une config écrite AVANT l'existence de la table `[shortcuts]` doit rester lisible — sans
    /// `#[serde(default)]` sur ce champ, elle ferait échouer `toml::from_str` et l'utilisateur
    /// perdrait son `log_path` au premier lancement de la nouvelle version.
    /// Les réglages de la carte de chat font l'aller-retour, et une config qui ne les porte pas
    /// retombe sur le défaut sans faire échouer la lecture.
    #[test]
    fn aller_retour_des_reglages_de_carte_de_chat() {
        let mut config = OverlayConfig::default();
        assert_eq!(
            config.chat_toast(),
            crate::panels::chat_tab::ChatToastSettings::default()
        );
        config.set_chat_toast(crate::panels::chat_tab::ChatToastSettings {
            duration_seconds: 7.5,
            manual_close: true,
        });
        let raw = toml::to_string_pretty(&config).expect("sérialisation");
        let relu: OverlayConfig = toml::from_str(&raw).expect("relecture");
        assert_eq!(relu.chat_toast().duration_seconds, 7.5);
        assert!(relu.chat_toast().manual_close);
    }

    /// Le défaut d'une fonctionnalité est d'être ACTIVE — y compris pour un `config.toml` écrit
    /// avant l'existence de ces clés (`#[serde(default = "actif")]`, voir le champ). Sans cette
    /// fonction de défaut, `#[serde(default)]` les mettrait à `false` et couperait Suivi, Alertes,
    /// Recherche, le panneau de combat et le suivi des sorts chez tous ceux qui les utilisent
    /// déjà.
    #[test]
    fn fonctionnalites_actives_par_defaut() {
        let neuve = OverlayConfig::default();
        assert!(neuve.suivi_enabled);
        assert!(neuve.alerts_enabled);
        assert!(neuve.chat_enabled);
        assert!(neuve.combat_enabled);
        assert!(neuve.spells_enabled);
        assert_eq!(
            neuve.features(),
            crate::panels::feature_switch::FeatureToggles::default()
        );

        let ancienne: OverlayConfig =
            toml::from_str("log_path = \"/config/wakfu.log\"").expect("ancienne config lisible");
        assert!(ancienne.suivi_enabled);
        assert!(ancienne.alerts_enabled);
        assert!(ancienne.chat_enabled);
        assert!(ancienne.combat_enabled);
        assert!(ancienne.spells_enabled);
    }

    /// Le jalon du démarrage automatique part à « jamais fait » — config neuve comme config
    /// écrite avant ce champ — et survit à l'aller-retour une fois levé : c'est ce qui empêche un
    /// second lancement de réinscrire ce que l'utilisateur a décoché (voir `crate::autostart`).
    #[test]
    fn le_jalon_du_demarrage_automatique_part_a_faux_et_se_conserve() {
        assert!(!OverlayConfig::default().autostart_initialized);
        let ancienne: OverlayConfig =
            toml::from_str("log_path = \"/config/wakfu.log\"").expect("ancienne config lisible");
        assert!(!ancienne.autostart_initialized);

        let config = OverlayConfig {
            autostart_initialized: true,
            ..Default::default()
        };
        let raw = toml::to_string_pretty(&config).expect("sérialisation");
        let relu: OverlayConfig = toml::from_str(&raw).expect("relecture");
        assert!(relu.autostart_initialized);
    }

    /// Une fonctionnalité coupée le reste après un aller-retour sur disque : c'est tout l'intérêt
    /// de la persister.
    #[test]
    fn aller_retour_des_fonctionnalites_coupees() {
        let config = OverlayConfig {
            suivi_enabled: false,
            chat_enabled: false,
            spells_enabled: false,
            ..Default::default()
        };
        let raw = toml::to_string_pretty(&config).expect("sérialisation");
        let relu: OverlayConfig = toml::from_str(&raw).expect("relecture");
        assert!(!relu.suivi_enabled);
        assert!(relu.alerts_enabled);
        assert!(!relu.chat_enabled);
        // Le détail des combats reste actif, seul le suivi des sorts est coupé : les deux cases
        // de la section « Combat » sont bien deux clés distinctes.
        assert!(relu.combat_enabled);
        assert!(!relu.spells_enabled);
        assert!(!relu.features().spells_visible());
    }

    /// Les trois cases de la bande Récap sont trois clés distinctes, persistées et relues telles
    /// quelles ; une config d'avant leur existence les retrouve toutes cochées.
    #[test]
    fn aller_retour_des_cases_du_recap() {
        let mut config = OverlayConfig::default();
        config.set_features(crate::panels::feature_switch::FeatureToggles {
            recap_cells: crate::panels::recap::RecapCells {
                duration: false,
                fights: true,
                challenges: false,
            },
            ..Default::default()
        });
        let raw = toml::to_string_pretty(&config).expect("sérialisation");
        let relu: OverlayConfig = toml::from_str(&raw).expect("relecture");
        assert!(relu.recap_enabled);
        assert!(!relu.recap_duration_enabled);
        assert!(relu.recap_fights_enabled);
        assert!(!relu.recap_challenges_enabled);
        assert_eq!(
            relu.features().recap_cells,
            crate::panels::recap::RecapCells {
                duration: false,
                fights: true,
                challenges: false,
            }
        );
        let ancienne: OverlayConfig = toml::from_str("recap_enabled = false\n").expect("relecture");
        assert!(!ancienne.recap_enabled);
        assert_eq!(
            ancienne.features().recap_cells,
            crate::panels::recap::RecapCells::default()
        );
    }

    /// **Le suivi des sorts coupé se retrouve tel quel**, même si le détail des combats l'éteint
    /// entre-temps : c'est `FeatureToggles::spells_visible` qui combine les deux, la config
    /// garde les deux cases séparément (voir le champ `spells_enabled`).
    #[test]
    fn detail_des_combats_coupe_garde_la_case_des_sorts() {
        let mut config = OverlayConfig::default();
        config.set_features(crate::panels::feature_switch::FeatureToggles {
            combat: false,
            ..Default::default()
        });
        let raw = toml::to_string_pretty(&config).expect("sérialisation");
        let relu: OverlayConfig = toml::from_str(&raw).expect("relecture");
        assert!(!relu.combat_enabled);
        assert!(relu.spells_enabled);
        assert!(!relu.features().spells_visible());
    }

    /// **Le défaut d'une sourdine est d'être levée** — `false` des deux côtés, y compris pour un
    /// `config.toml` écrit avant ces deux clés : personne ne doit perdre le son d'une alerte au
    /// premier lancement de cette version. Coupée, elle le reste après un aller-retour sur disque.
    #[test]
    fn aller_retour_des_sourdines() {
        let neuve = OverlayConfig::default();
        assert_eq!(
            neuve.alert_mutes(),
            crate::panels::notifications::AlertMutes::default()
        );
        assert!(!neuve.suivi_alert_muted);
        assert!(!neuve.chat_alert_muted);

        let ancienne: OverlayConfig =
            toml::from_str("log_path = \"/config/wakfu.log\"").expect("ancienne config lisible");
        assert!(!ancienne.suivi_alert_muted);
        assert!(!ancienne.chat_alert_muted);

        let mut config = OverlayConfig::default();
        config.set_alert_mutes(crate::panels::notifications::AlertMutes {
            suivi: true,
            chat: false,
        });
        let raw = toml::to_string_pretty(&config).expect("sérialisation");
        let relu: OverlayConfig = toml::from_str(&raw).expect("relecture");
        assert!(relu.alert_mutes().suivi);
        assert!(!relu.alert_mutes().chat);
    }

    #[test]
    fn config_sans_table_de_raccourcis_reste_lisible() {
        let config: OverlayConfig =
            toml::from_str("log_path = \"/config/wakfu.log\"").expect("ancienne config lisible");
        assert_eq!(config.log_path, Some(PathBuf::from("/config/wakfu.log")));
        assert_eq!(config.shortcuts(), ShortcutBindings::default());
    }

    /// Aller-retour par le FORMAT réellement écrit sur disque (`to_string_pretty`, voir `save`) :
    /// les clés de racine doivent rester AVANT la table `[shortcuts]`, sinon le fichier relu
    /// rattacherait `log_path` à la table — voir la doc du champ.
    #[test]
    fn aller_retour_toml_avec_raccourcis_personnalises() {
        let mut config = OverlayConfig {
            log_path: Some(PathBuf::from("/config/wakfu.log")),
            combat_always_visible: true,
            ..Default::default()
        };
        let mut bindings = ShortcutBindings::default();
        bindings.set(
            crate::shortcuts::ShortcutAction::Options,
            crate::shortcuts::Shortcut::parse("Ctrl+Alt+K").expect("combinaison de test valide"),
        );
        config.set_shortcuts(&bindings);

        let raw = toml::to_string_pretty(&config).expect("sérialisation");
        let relu: OverlayConfig = toml::from_str(&raw).expect("relecture");
        assert_eq!(relu, config);
        assert_eq!(relu.shortcuts(), bindings);
    }

    /// La bande Récap déplacée (2026-09-17) : les deux coordonnées font l'aller-retour, elles
    /// s'écrivent AVANT `[shortcuts]` (sans quoi la table les avalerait, voir le champ
    /// `shortcuts`), et une config qui ne les porte pas — toutes celles écrites avant ce jour —
    /// se relit sans erreur, bande à son ancrage d'origine.
    #[test]
    fn aller_retour_de_la_position_du_recap() {
        let vierge = OverlayConfig::default();
        assert_eq!(vierge.recap_position(), None);

        let mut config = OverlayConfig {
            log_path: Some(PathBuf::from("/config/wakfu.log")),
            ..Default::default()
        };
        config.set_recap_position(Some((460, 234)));
        let raw = toml::to_string_pretty(&config).expect("sérialisation");
        let position = raw.find("recap_position_x").expect("clé écrite");
        let table = raw.find("[shortcuts]").expect("table écrite");
        assert!(
            position < table,
            "les clés de position doivent précéder `[shortcuts]`, sinon la table les avale :\n{raw}"
        );
        let relu: OverlayConfig = toml::from_str(&raw).expect("relecture");
        assert_eq!(relu.recap_position(), Some((460, 234)));

        let ancienne: OverlayConfig =
            toml::from_str("log_path = \"/config/wakfu.log\"").expect("ancienne config lisible");
        assert_eq!(ancienne.recap_position(), None);
    }

    /// Une seule des deux coordonnées ne fait pas une position — une bande collée contre un bord
    /// serait pire que l'ancrage d'origine, et faire échouer toute la lecture pire encore (le
    /// `log_path` partirait avec).
    #[test]
    fn une_demi_position_du_recap_ne_deplace_rien() {
        let bancale: OverlayConfig =
            toml::from_str("recap_position_x = 460").expect("config bancale lisible");
        assert_eq!(bancale.recap_position(), None);

        let mut config = OverlayConfig::default();
        config.set_recap_position(Some((460, 234)));
        config.set_recap_position(None);
        assert_eq!(config.recap_position_x, None);
        assert_eq!(config.recap_position_y, None);
    }
}
