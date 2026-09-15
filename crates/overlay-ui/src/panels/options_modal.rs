//! Modale "Options" — voir §9.1 du plan d'architecture. Ouverte par le bouton "Options" du carré
//! de contrôle (`panels::watchlist::control_button_row`) ou le raccourci global `Ctrl+Shift+O`
//! (voir `main.rs`/`bin/overlay-ui-x11.rs`). L'onglet "Paramètres" expose les réglages LOCAUX de
//! l'overlay : le chemin de `wakfu.log` à suivre, et la section « Combat » — l'affichage du
//! panneau de combat en dehors des combats (2026-09-13) et la notification de tour (2026-09-14).
//! Tous sont persistés par `config::OverlayConfig`, jamais sur le compte — contrairement aux
//! onglets "Suivi" et "Alertes".
//!
//! **Interrupteurs de fonctionnalité (2026-09-15, §9.1 duodecies)** : les onglets "Suivi",
//! "Alertes" et "Chat" s'ouvrent chacun sur une case « Activer … » (`panels::feature_switch`) qui
//! grise et rend inerte tout le reste de leur écran quand elle est décochée. Elles voyagent
//! ensemble dans [`OptionsModalState::features`], sont un brouillon comme le reste de la fenêtre,
//! et sont persistées en LOCAL (`config::OverlayConfig::features`) malgré leur place dans des
//! onglets qui, eux, règlent le compte : ce qu'on accepte de voir par-dessus son jeu dépend de la
//! machine, pas du joueur.
//!
//! **Refonte 2026-09-09 — chrome basé sur les VRAIES textures du jeu, plus des formes peintes à la
//! main** (voir `panels::chamfer`, toujours utilisé ailleurs pour la barre de dégâts, mais plus
//! ici) : chaque mesure ci-dessous vient d'une capture d'écran réelle de la fenêtre Options du jeu
//! (`assets/design-system/interfaces/interface-options-*.png`, six onglets, chrome identique sur
//! les six), affinée par `.claude/skills/design-asset/scripts/dsimg.py analyze` puis VALIDÉE avec
//! l'utilisateur via une simulation HTML/CSS interactive avant ce portage (méthode explicitement
//! demandée : itérer en HTML, moins coûteux qu'itérer directement en Rust/egui, PUIS porter une
//! fois la maquette acceptée).
//!
//! Différences avec la toute première version (chamfrein peint à la main) :
//! - **Coins ARRONDIS, pas chanfreinés** — vérifié au pixel sur `modal-header.png` (rayon ≈12px) :
//!   la modale entière ET son encadré interne ("section") sont arrondis, ce dernier avec un rayon
//!   PLUS PRONONCÉ (18px) que la modale (12px) — constaté directement sur la référence, pas déduit.
//! - **Bannière peinte avec la vraie texture du jeu** (`modal-header.png`) plutôt qu'un dégradé
//!   approximé à la main — son rayon de coin mesuré (2-4px) est assez petit pour tolérer un
//!   étirement uniforme sans déformation perceptible.
//! - **Menu à trois entrées** ("Alertes", "Personnages", "Paramètres") au-dessus de la section,
//!   texture `menu-tabs.png` (dérivée de `tabs-with-first-tab-active.png` du design system, miroir
//!   horizontal pour que le segment actif kaki tombe sur "Paramètres", dernière entrée) — seule
//!   "Paramètres" est câblée (contenu de cette modale), "Alertes"/"Personnages" restent des stubs
//!   visuels en attente d'un futur chantier.
//! - **Bouton "Sélectionner le fichier"** sous le champ (pas à côté).
//! - Fenêtre plus haute (`WINDOW_SIZE`, ratio aligné sur les 720:561 mesurés de la vraie fenêtre du
//!   jeu — demande explicite : « garder une cohérence par rapport au rendu [du jeu] »), section
//!   renommée "Fichier" (au lieu de "Fichier wakfu.log", redondant avec le contenu du champ), champ
//!   + bouton empilés verticalement (au lieu de côte à côte).
//!
//! **Refonte 2026-09-09 (2) — les trois boutons passent sur `design::button`.** Ils étaient peints
//! ici à la main : deux images taillées sur mesure pour le pied de page (libellé gravé dedans, un
//! fichier par libellé) et un 9-slice local pour le bouton "Sélectionner le fichier". Ce sont
//! maintenant trois appels au composant du design system, qui apporte avec lui le survol, l'état
//! pressé, le curseur, l'écrêtage du libellé et la trace de journal. Ont disparu avec eux : les
//! quatre PNG de `assets/ui/options/` (tous des copies octet pour octet d'assets déjà déclarés au
//! manifeste `design::assets`) et le module `panels::nine_slice`, doublon de `design::nine_slice`
//! sans marges par côté ni mode de remplissage.
//!
//! Gain visible : le bouton "Sélectionner le fichier" est rendu à 700px de large depuis une texture
//! de 169px. L'ancien 9-slice ne figeait que 14px de chaque côté, bien moins que les 52px de décor
//! mesurés — les croisillons tombaient dans la bande médiane et s'étiraient sur près de 200px.
//!
//! **Refonte 2026-09-10 — le CORPS passe sur une texture du jeu** (`DsTexture::ModalBody`,
//! `modal-body.png`). Il était peint d'un aplat (`MODAL_BG`, `#1C2023`) : la bannière venait du
//! jeu, le fond sous elle non, et l'écart se voyait. La texture est découpée des six captures de
//! la fenêtre Options puis débarrassée de son contenu — onglets, bouton de réinitialisation,
//! panneau de section, boutons de pied de page (`tools/design-system/build_modal_body.py`, §9 ter
//! du design-system). Elle apporte le grain du jeu et ses hachures d'angle, qui ne sont pas une
//! trame de fond mais un CADRE : denses dans les angles et le long des bords, absentes du centre —
//! le même constat que pour les boutons, à l'échelle de la fenêtre.
//!
//! Deux conséquences sur ce fichier : la couleur du fond ne s'y règle plus (seule sa translucidité
//! reste ici, `MODAL_BODY_TINT`), et les deux angles BAS sont désormais portés par l'alpha de la
//! texture, plus par un `corner_radius`.
//!
//! **Pas de validation filesystem ICI** : cette fonction ne fait que peindre et renvoyer l'INTENTION
//! de l'utilisateur (`OptionsModalAction`) — c'est l'appelant (`main.rs`/`bin/overlay-ui-x11.rs`,
//! qui seuls savent comment déclencher un dialogue de fichier natif et parler au thread Engine) qui
//! valide via `overlay_ingest::discovery::validate_log_path` et alimente [`OptionsModalState::error`]
//! en retour pour le prochain redessin.

use overlay_sync::update::{self, UpdateStatus};

use crate::design::{self, ButtonSize, ButtonVariant};
use crate::panels::alerts_tab::{self, AlertsTabContext, AlertsTabState};
use crate::panels::chat_tab::{self, ChatAvailability, ChatDraft, ChatTabState};
use crate::panels::feature_switch::FeatureToggles;
use crate::panels::notifications::{self, AlertMutes};
use crate::panels::{raccourcis_tab, recipe_dialog, suivi_tab};
use crate::shortcuts::ShortcutBindings;

/// Taille de la fenêtre OS dédiée à cette modale (voir `main.rs::create_overlay_window`, cas
/// `OverlayKind::Options`).
///
/// **760 × 810 depuis le 2026-09-12**, contre 560 × 436 auparavant : c'est ce que l'onglet
/// « Alertes » demande pour tenir cinq tuiles par rangée et trois rangées visibles (demande
/// explicite du 2026-09-11, validée sur maquette). À 560 de large, la grille n'avait la place que
/// de trois tuiles, et d'une seule rangée en hauteur.
///
/// Ce qui est abandonné au passage, et assumé : le **rapport d'aspect 720:561 de la vraie fenêtre
/// Options du jeu**, que la version précédente reproduisait sur demande de cohérence visuelle.
/// Arbitrage au profit du contenu — une fenêtre au bon ratio dont l'onglet principal ne tient pas
/// n'est pas plus fidèle, elle est juste inutilisable.
pub const WINDOW_SIZE: (f32, f32) = (760.0, 810.0);

// Rembourrage du panneau de contenu — trois axes, et trois seulement. Le relevé de section est
// catégorique : « x=29 pour les titres de section, x=36 pour tout contrôle indenté, x=62 pour le
// texte qui suit une case. Aucun autre alignement n'a été relevé dans les six onglets. » Rapportés
// au bord du panneau (x=17), cela donne les deux valeurs ci-dessous.
//
// La première version mettait 22px partout, titre compris : le titre et ses contrôles étaient donc
// sur le même axe, ce qui efface le seul signal de niveau que le jeu utilise.

// Le champ de chemin est un composant du design system (`design::input`) depuis le 2026-09-10 :
// ses couleurs, son rayon et son retrait de texte ne sont plus des constantes de ce panneau. Les
// trois qui vivaient ici étaient d'ailleurs fausses — fond #1C1E23 au lieu de #0E1115, bord d'1px
// au lieu de 2, rayon 2 au lieu de 4.

/// Gouttière entre le champ de chemin et le bouton "Sélectionner le fichier", posés sur la MÊME
/// ligne (demande utilisateur 2026-09-09 : le bouton ne doit plus prendre toute la largeur).
///
/// 10px, la valeur du relevé (`docs/design-system/releve-section-options.json`, nœud `sel-ctrl`) :
/// c'est l'écart que le jeu laisse entre un libellé et le contrôle posé à sa droite, le seul écart
/// intra-ligne qu'il ait été possible de mesurer. Elle figure bien à l'échelle d'espacement de la
/// section (6, 7, 9, **10**, 11, 17, 21, 30, 31), ce n'est pas une valeur inventée pour l'occasion.
const FIELD_TO_BROWSE_GAP: f32 = 10.0;
/// Hauteur d'une ligne de contrôle — le champ ET le bouton qui l'accompagne.
///
/// 36px, relevé sur les six onglets de la modale du jeu : **tout contrôle posé sur une ligne de
/// contenu y fait 36px de haut**, liste déroulante comme bouton (`releve-section-options.json`,
/// nœuds `sel-ctrl` 233..269 et `depl-ctrl` 314..350 ; `releve-modale-options.json`, note sur le
/// rythme vertical : « une ligne portant un contrôle de 36px passe à 39px »). C'est aussi la
/// hauteur native de la texture de bouton compact, donc la seule hauteur à laquelle un bouton ne
/// subit aucun étirement vertical.
///
/// Remplace deux valeurs qui ne s'accordaient ni entre elles ni avec le jeu : un champ à 34px et
/// un bouton à 40px, alors que le pied de page était déjà à 36.
const ROW_HEIGHT: f32 = 36.0;

/// Écart entre la ligne de contrôle et le message d'erreur qui la commente.
///
/// **9px, pas 10.** Le relevé de section ne cote pas ce cas précis (le jeu n'a qu'un seul bloc
/// d'information, en bas de l'onglet Interface, et il suit une grille de boutons, pas un champ) —
/// c'est donc la valeur de son échelle d'espacement la plus proche du cas « un commentaire collé au
/// contrôle qu'il commente » : celle qui sépare un titre du contrôle pleine largeur qu'il coiffe
/// (7 à 9). Le 10 d'avant venait du même geste qu'ailleurs dans ce fichier — une valeur choisie.
const INFO_GAP: f32 = 9.0;

/// Hauteur du champ de chemin — sa hauteur NATIVE, plus basse que la ligne qui le porte. Voir
/// `design::components::input` pour la mesure et pourquoi les deux diffèrent.
const FIELD_HEIGHT: f32 = design::InputSize::Standard.height();

/// Air entre la fin d'une section et le titre de la suivante.
///
/// **17px, relevé** (`docs/design-system/releve-section-options.json`, onglet Jeu) : la section
/// « Jouabilité » s'y arrête à y=269 et le titre « Mode de déplacement » ouvre la suivante à
/// y=286. C'est le seul signal de regroupement que le jeu emploie — ses sections n'ont ni fond, ni
/// filet, ni bordure (note du relevé) : cet écart est donc la séparation elle-même, pas une
/// décoration qu'on pourrait resserrer.
const SECTION_GAP: f32 = 17.0;

/// Onglet affiché par la modale.
///
/// Onglet affiché par la modale — **quatre entrées câblées sur cinq** depuis le 2026-09-13.
///
/// « Alertes » a reçu son contenu le 2026-09-12 (`panels::alerts_tab`), « Suivi » le lendemain
/// (`panels::suivi_tab`), « Raccourcis » le même jour (`panels::raccourcis_tab`) ; « Personnages »
/// reste affiché désactivé plutôt que masqué, décision du 2026-09-10 : un onglet qui apparaît est
/// un changement de mise en page, pas un changement d'état.
///
/// **« Raccourcis » se place AVANT « Paramètres »** (demande du 2026-09-13) : les deux écrans
/// règlent l'overlay lui-même, et celui qu'on vient rouvrir est celui des touches — le chemin de
/// `wakfu.log` se règle une fois.
///
/// **« Suivi » ouvre le menu**, avant « Alertes » : c'est l'écran qu'on vient chercher le plus
/// souvent — composer ce qu'on suit se refait à chaque session de jeu, régler ses alertes une fois
/// pour toutes. L'ouverture suit l'ordre du menu (voir [`OptionsTab::default`]).
///
/// **Deux boutons du bandeau de suivi in-game ouvrent CETTE fenêtre, chacun sur un onglet
/// différent** (`panels::watchlist::control_button_row`, refonte 2026-09-13) : « + » sur « Suivi »
/// (l'écran où il mène), « Options » sur « Paramètres » (ce bandeau porte déjà son propre accès à
/// la composition de la liste suivie, « Options » y sert donc à autre chose : le chemin de
/// `wakfu.log`). Chacun a un raccourci global qui le double (`Ctrl+Shift+A`/`Ctrl+Shift+O`) et doit
/// mener au MÊME endroit — l'un et l'autre passent l'onglet à `App::open_options_modal`
/// (`main.rs`/`bin/overlay-ui-x11.rs`) explicitement, sans jamais s'en remettre à
/// [`OptionsTab::default`] : un bouton et son raccourci qui atterriraient sur deux onglets
/// différents serait le bug exact corrigé ce jour-là (retour utilisateur explicite).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsTab {
    /// La liste des objets et monstres suivis — ce qu'on ajoute, en quel mode, ce qu'on retire.
    Suivi,
    /// Les alertes de ramassage — objets à son activé et fermeture du toast.
    Alertes,
    /// Les recherches de chat — mot et canal qui font sonner l'overlay, et la carte qui va avec
    /// (`panels::chat_tab`). Entre Alertes et Personnages : maquette validée le 2026-09-13.
    Chat,
    Personnages,
    /// Les raccourcis clavier globaux de l'overlay (`panels::raccourcis_tab`) — **placé avant
    /// « Paramètres »**, demande utilisateur explicite du 2026-09-13 : « ajouter un onglet
    /// "Raccourcis", avant paramètre ».
    Raccourcis,
    /// Le chemin de `wakfu.log`. Ce fut l'onglet d'ouverture tant qu'il était le seul câblé.
    Parametres,
}

impl Default for OptionsTab {
    /// **La fenêtre s'ouvre sur la PREMIÈRE entrée du menu**, quelle qu'elle soit — demande du
    /// 2026-09-12, nuit : « quand on ouvre les options, il faut qu'on arrive sur le premier
    /// onglet ; si plus tard il y a une autre option en premier, il faudra atterrir sur celle-là ».
    /// Le défaut suivait jusque-là « Paramètres », seul onglet câblé à l'origine ; il était devenu
    /// le dernier du menu sans que l'ouverture ne suive. Si l'ordre des entrées change dans
    /// [`show`], ce défaut change avec lui.
    fn default() -> Self {
        Self::Suivi
    }
}

/// État mutable de la modale, propriété de la fenêtre OS qui l'affiche (voir
/// `main.rs`/`bin/overlay-ui-x11.rs`, nouveau champ `OverlayWindow` réservé au cas
/// `OverlayKind::Options`) — persiste d'une frame à l'autre, contrairement à [`OptionsModalAction`]
/// qui ne vit que le temps d'un `show`.
#[derive(Debug, Default, Clone)]
pub struct OptionsModalState {
    /// Contenu ACTUEL du champ texte — initialisé au chemin actif au moment de l'ouverture de la
    /// modale (voir l'appelant), modifié librement par la frappe ou par [`OptionsModalAction::Browse`]
    /// une fois le dialogue natif résolu. Rien n'est pris en compte tant que
    /// [`OptionsModalAction::Validate`] n'a pas été renvoyé ET jugé valide par l'appelant (§5.1 du
    /// plan : « tant que l'utilisateur n'a pas validé son choix, rien n'est pris en compte »).
    pub path_input: String,
    /// Message d'erreur de la DERNIÈRE tentative de validation (`Browse` sur un fichier mal nommé,
    /// ou clic sur "Valider" avec un chemin invalide) — `None` tant qu'aucune tentative n'a encore
    /// échoué. Vidé par l'appelant dès qu'une nouvelle tentative commence.
    pub error: Option<String>,
    /// Onglet affiché.
    pub tab: OptionsTab,
    /// Le panneau Combat reste-t-il affiché en dehors des combats ? — case à cocher de l'onglet
    /// « Paramètres », initialisée par l'hôte au réglage en vigueur (`config::OverlayConfig::
    /// combat_always_visible`) et prise en compte seulement à « Valider », comme le chemin de log
    /// et les deux brouillons (§5.1 du plan).
    pub combat_always_visible: bool,
    /// Prévenir par une notification du système qu'un personnage du joueur doit jouer ? — case à
    /// cocher de la section « Combat » de l'onglet « Paramètres » (2026-09-14), même mécanique de
    /// brouillon que la case ci-dessus : initialisée par l'hôte au réglage en vigueur
    /// (`config::OverlayConfig::turn_notification`), prise en compte seulement à « Valider ».
    pub turn_notification: bool,
    /// Couper le son de la notification de tour ? — case sous la précédente, dont elle dépend
    /// (`config::OverlayConfig::turn_notification_muted`), même mécanique de brouillon.
    pub turn_notification_muted: bool,
    /// **Les trois interrupteurs de fonctionnalité** — cases « Activer le suivi » / « Activer les
    /// alertes » / « Activer la recherche », tout en haut de leur onglet respectif
    /// (`panels::feature_switch`, 2026-09-15). Même mécanique de brouillon que les cases
    /// ci-dessus : initialisés par l'hôte au réglage en vigueur (`config::OverlayConfig::
    /// features`), pris en compte seulement à « Valider ».
    ///
    /// **`Default` vaut ici « tout actif »**, et non `false` comme pour un `bool` nu : c'est
    /// [`FeatureToggles`] qui le garantit, pour que `OptionsModalState::default()` — utilisé par
    /// les tests et le harnais de rendu — n'ouvre jamais une fenêtre dont les trois onglets
    /// seraient grisés.
    pub features: FeatureToggles,
    /// **Les deux sourdines** — cases « Couper le son des notifications » des onglets « Suivi » et
    /// « Chat », sous leur ligne « Tester le son de l'alerte » (`panels::notifications`, 2026-09-15).
    /// Même mécanique de brouillon que les cases ci-dessus : initialisées par l'hôte au réglage en
    /// vigueur (`config::OverlayConfig::alert_mutes`), prises en compte seulement à « Valider ».
    ///
    /// **Distinctes de [`Self::features`]** : une sourdine ne coupe que le SON, la carte de
    /// l'alerte continue de s'afficher — c'est le demi-pas entre « tout actif » et une
    /// fonctionnalité éteinte.
    pub mutes: AlertMutes,
    /// Ce que l'onglet « Suivi » garde entre deux frames — saisie, mode, quantité, sélection
    /// multiple, fenêtre de recette ouverte. **Pas la liste** : celle-ci est le brouillon ci-dessous.
    pub suivi: suivi_tab::SuiviTabState,
    /// **Le brouillon de suivi** — une copie des entrées du compte, modifiée librement, et prise en
    /// compte seulement à « Valider ».
    ///
    /// `None` tant que les réglages ne sont pas descendus du compte, même raison que pour les
    /// alertes : l'onglet affiche alors son rouage plutôt qu'une liste provisoire que la réponse
    /// démentirait.
    pub suivi_draft: Option<Vec<overlay_engine::WatchlistEntry>>,
    /// D'où vient la liste suivie, et si elle est modifiable — posé par l'hôte à l'ouverture.
    pub suivi_availability: suivi_tab::SuiviAvailability,
    /// Ce que l'onglet « Raccourcis » garde entre deux frames — recherche, case en écoute, dernier
    /// refus. **Pas les combinaisons** : celles-ci sont le brouillon ci-dessous.
    pub raccourcis: raccourcis_tab::RaccourcisTabState,
    /// **Le brouillon de raccourcis** — une copie des combinaisons EN VIGUEUR (posée par l'hôte à
    /// l'ouverture, `main.rs::open_options_modal`), modifiée librement, et prise en compte
    /// seulement à « Valider ».
    ///
    /// Pas d'`Option` ici, contrairement aux alertes et au suivi : les raccourcis sont un réglage
    /// LOCAL (`config::OverlayConfig`), jamais descendu du compte — il n'y a donc rien à attendre,
    /// et aucun état « en chargement » à afficher.
    pub shortcuts: ShortcutBindings,
    /// Ce que l'onglet « Alertes » garde entre deux frames — saisie du champ d'ajout, durée en
    /// cours de frappe, confirmation de retrait ouverte. **Pas le profil** : celui-ci est le
    /// brouillon ci-dessous.
    pub alerts: AlertsTabState,
    /// **Le brouillon d'alertes** — une copie du profil du compte, modifiée librement, et prise en
    /// compte seulement à « Valider » (§5.1 du plan, comme le chemin de log).
    ///
    /// `None` tant que les réglages ne sont pas descendus du compte : l'onglet affiche alors son
    /// rouage plutôt qu'une liste provisoire que la réponse écraserait.
    pub alerts_draft: Option<overlay_engine::AlertProfile>,
    /// D'où vient la liste d'alertes, et si elle est modifiable — posé par l'hôte à l'ouverture de
    /// la modale, parce que lui seul sait si un compte est lié et si une requête est en vol.
    pub alerts_availability: alerts_tab::AlertsAvailability,
    pub chat: ChatTabState,
    /// Brouillon de l'onglet « Chat » — même principe que `alerts_draft` : `None` tant que le
    /// compte n'a pas répondu.
    pub chat_draft: Option<ChatDraft>,
    pub chat_availability: ChatAvailability,
    /// **L'état de référence**, figé à l'ouverture : le chemin de log et le profil d'alerte tels
    /// qu'ils étaient avant que l'utilisateur ne touche à quoi que ce soit.
    ///
    /// C'est lui qui répond à « y a-t-il des modifications en attente ? » ([`OptionsModalState::
    /// is_dirty`]), et donc lui qui décide si fermer doit demander confirmation. Sans référence, la
    /// modale ne pourrait comparer qu'à elle-même.
    pub initial: OptionsInitial,
    /// Un compte est-il connecté ? — posé par l'hôte à l'ouverture (lui seul connaît
    /// `AuthStatus`). Décide si le bouton « Se déconnecter » de l'onglet « Paramètres » est actif : le
    /// presser sans compte lié ne ferait rien de visible, mieux vaut que ça se voie avant le clic.
    pub account_connected: bool,
    /// La confirmation de déconnexion est ouverte — voir la section « Compte » de [`show`]. Un
    /// champ distinct de [`Self::pending_close`] : les deux boîtes posent des questions
    /// différentes, et une seule peut être ouverte à la fois (voir `show`).
    pub pending_disconnect: bool,
    /// Où en est la mise à jour automatique — copié par l'hôte depuis l'état publié par le
    /// thread de mise à jour AVANT chaque rendu (jamais figé à l'ouverture : une vérification
    /// lancée depuis cette fenêtre doit s'y voir aboutir). Décide de la ligne d'information et
    /// du bouton de la section « Mise à jour » (2026-09-15, `docs/plan-mise-a-jour.md` §8.2).
    pub update: UpdateStatus,
    /// Installer automatiquement les mises à jour au démarrage ? — case de la section « Mise à
    /// jour », même mécanique de brouillon que les autres cases : initialisée par l'hôte au
    /// réglage en vigueur (`config::OverlayConfig::auto_update`), prise en compte à « Valider ».
    pub auto_update: bool,
    /// La confirmation d'installation est ouverte, pour cette version — « Mettre à jour vers X »
    /// ferme l'overlay de jeu le temps de l'installation, ce qui mérite un « oui » explicite.
    /// Troisième boîte exclusive avec les deux autres (voir `show`).
    pub pending_install: Option<String>,
    /// Une confirmation d'abandon est ouverte — voir [`OptionsModalState::is_dirty`].
    ///
    /// Posée par le clic sur « Annuler », par la croix de la bannière (2026-09-13), par Échap, **ou
    /// par l'hôte** quand la croix de la fenêtre OS est actionnée : les quatre gestes ferment la
    /// même chose et méritent la même garde.
    pub pending_close: bool,
}

/// L'état de la fenêtre à son ouverture — voir [`OptionsModalState::initial`].
#[derive(Debug, Default, Clone, PartialEq)]
pub struct OptionsInitial {
    pub path: String,
    pub alerts: Option<overlay_engine::AlertProfile>,
    /// Les entrées suivies telles qu'elles étaient à l'ouverture — c'est elles que « Annuler »
    /// abandonne, et leur comparaison au brouillon qui décide si la garde de fermeture s'ouvre.
    pub suivi: Option<Vec<overlay_engine::WatchlistEntry>>,
    pub chat: Option<ChatDraft>,
    /// L'affichage permanent du panneau Combat tel qu'il était à l'ouverture — une case cochée
    /// puis décochée revient donc à « aucune modification », et la garde de fermeture ne s'ouvre
    /// pas pour rien.
    pub combat_always_visible: bool,
    /// La notification de tour telle qu'elle était à l'ouverture — même rôle que le champ
    /// ci-dessus.
    pub turn_notification: bool,
    /// Le son coupé tel qu'il était à l'ouverture — même rôle.
    pub turn_notification_muted: bool,
    /// Les trois interrupteurs tels qu'ils étaient à l'ouverture — même rôle que les champs
    /// ci-dessus : c'est leur comparaison au brouillon qui décide si fermer demande confirmation.
    pub features: FeatureToggles,
    /// Les deux sourdines telles qu'elles étaient à l'ouverture — même rôle que les champs
    /// ci-dessus.
    pub mutes: AlertMutes,
    /// Les raccourcis tels qu'ils étaient à l'ouverture — même rôle que les champs ci-dessus :
    /// c'est leur comparaison au brouillon qui décide si fermer demande confirmation.
    pub shortcuts: ShortcutBindings,
    /// La mise à jour automatique telle qu'elle était à l'ouverture — même rôle.
    pub auto_update: bool,
}

impl OptionsModalState {
    /// Y a-t-il des modifications en attente ?
    ///
    /// **Ce que la garde de fermeture protège** : un joueur qui ajoute trois objets puis ferme par
    /// réflexe perdait tout, en silence — la fenêtre est transactionnelle, rien n'est écrit avant
    /// « Valider ».
    ///
    /// Le **changement d'onglet**, lui, n'intercepte rien : le brouillon lui survit, et demander
    /// confirmation à chaque aller-retour entre deux onglets rendrait la fenêtre inutilisable.
    /// Ce que « Valider » emporte de cette fenêtre — voir [`OptionsCommit`]. Les deux gestes qui
    /// valident (le bouton du pied de page et la touche Entrée) passent par ici, pour qu'aucun des
    /// deux ne puisse oublier un réglage que l'autre emporte.
    pub fn commit(&self) -> OptionsCommit {
        OptionsCommit {
            path: self.path_input.clone(),
            combat_always_visible: self.combat_always_visible,
            turn_notification: self.turn_notification,
            turn_notification_muted: self.turn_notification_muted,
            features: self.features,
            mutes: self.mutes,
            shortcuts: self.shortcuts.clone(),
            auto_update: self.auto_update,
        }
    }

    /// Ce que « Valider » produit — action de validation, **ou** refus sur place quand deux
    /// raccourcis partagent la même combinaison.
    ///
    /// Le doublon est le seul cas que cette fenêtre peut trancher elle-même : l'OS refuserait le
    /// second enregistrement (même `HotKey::id`), et personne d'autre n'a la liste sous les yeux.
    /// Le chemin de log, lui, reste validé par l'hôte (`discovery::validate_log_path`) — un panneau
    /// ne touche pas au disque.
    ///
    /// Bascule sur l'onglet « Raccourcis » en cas de refus : le message y est, et l'utilisateur a
    /// pu cliquer « Valider » depuis n'importe quel autre écran.
    pub fn validate(&mut self) -> OptionsModalAction {
        if let Some((first, second)) = self.shortcuts.conflict() {
            self.tab = OptionsTab::Raccourcis;
            self.raccourcis.capturing = None;
            self.raccourcis.error = Some(raccourcis_tab::conflict_message(
                first,
                second,
                self.shortcuts.get(first),
            ));
            return OptionsModalAction::None;
        }
        self.raccourcis.error = None;
        OptionsModalAction::Validate(self.commit())
    }

    pub fn is_dirty(&self) -> bool {
        self.path_input.trim() != self.initial.path.trim()
            || self.combat_always_visible != self.initial.combat_always_visible
            || self.turn_notification != self.initial.turn_notification
            || self.turn_notification_muted != self.initial.turn_notification_muted
            || self.features != self.initial.features
            || self.mutes != self.initial.mutes
            || self.alerts_draft != self.initial.alerts
            || self.suivi_draft != self.initial.suivi
            || self.chat_draft != self.initial.chat
            || self.shortcuts != self.initial.shortcuts
            || self.auto_update != self.initial.auto_update
    }
}

/// Ce que l'utilisateur vient de demander CETTE frame — `None` la plupart du temps (aucun bouton
/// cliqué). Voir doc de module : ne porte aucune garantie de validité, c'est à l'appelant de
/// vérifier avant d'agir.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum OptionsModalAction {
    #[default]
    None,
    Cancel,
    /// Ouvrir l'explorateur de fichiers natif — l'appelant seul sait le faire (`rfd`, sur un thread
    /// dédié pour ne jamais geler le rendu, voir sa doc dans `main.rs`).
    Browse,
    /// Les réglages de l'onglet « Paramètres » tels qu'ils sont à l'instant du clic — voir
    /// [`OptionsCommit`].
    Validate(OptionsCommit),
    /// Jouer le son d'alerte, depuis l'onglet « Alertes » — l'appelant seul a le périphérique
    /// audio (`alert_sound::play_loot_alert`).
    TestAlertSound,
    /// « Tester le son » de l'onglet « Chat » : jouer le son de recherche
    /// (`alert_sound::play_chat_alert`).
    TestChatSound,
    /// « Tester le son » de l'onglet « Suivi » : jouer le son du décompte arrivé à 0
    /// (`alert_sound::play_countdown_alert`).
    TestCountdownSound,
    /// Déconnecter le compte, depuis la section « Compte » de l'onglet « Paramètres »
    /// (**confirmée**, voir `show`) — l'appelant seul parle au thread Auth
    /// (`main.rs::App::disconnect_account`) et sait refermer cette fenêtre derrière.
    ///
    /// **Immédiat, jamais un brouillon** : contrairement au chemin, aux alertes, au suivi et aux
    /// raccourcis, ce que cette action déclenche ne passe pas par « Valider » et ne se rattrape pas
    /// par « Annuler ».
    Disconnect,
    /// Résoudre les ingrédients de cet objet, depuis l'onglet « Suivi » — l'appelant seul a le
    /// réseau (`overlay_sync::client::fetch_item_detail`, sur un thread).
    ResolveRecipe(i64),
    /// « Recherche de mise à jour » (section « Mise à jour ») : l'hôte demande au thread de mise
    /// à jour une vérification sans installation (`background::UpdateCommand::Check`). Immédiat,
    /// comme `Disconnect` — mais sans rien à confirmer, il ne change rien à la machine.
    CheckUpdate,
    /// « Mettre à jour vers X », **confirmé** : l'hôte referme cette fenêtre et les overlays de
    /// jeu, repasse par l'écran de chargement et laisse le thread de mise à jour télécharger,
    /// mettre en place, puis installe et relance (`App::install_update_if_ready`). Immédiat et
    /// sans retour : ce que cette fenêtre avait en brouillon est abandonné, comme à la
    /// déconnexion.
    InstallUpdate,
}

/// Ce que « Valider » emporte de l'onglet « Paramètres ».
///
/// **Une struct plutôt qu'un `String` nu** depuis le 2026-09-13, où l'onglet a gagné un second
/// réglage : un tuple anonyme de plus à chaque case à cocher ajoutée ferait une action dont
/// personne ne saurait dire, au site d'appel, quel booléen est lequel.
///
/// Ne porte aucune garantie de validité (voir doc de module) : `path` est le texte BRUT du champ,
/// pas un `PathBuf` vérifié — c'est l'hôte qui tranche, via
/// `overlay_ingest::discovery::validate_log_path`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OptionsCommit {
    /// Chemin brut tel que tapé/affiché dans le champ au moment du clic.
    pub path: String,
    /// État de la case « Afficher le panneau de combat en dehors des combats ».
    pub combat_always_visible: bool,
    /// État de la case « Me prévenir quand un de mes personnages doit jouer ».
    pub turn_notification: bool,
    /// État de la case « Couper le son des notifications » — emporté tel quel même si la case
    /// au-dessus est décochée (il ne fait alors rien, et sera retrouvé si on la recoche).
    pub turn_notification_muted: bool,
    /// État des trois cases « Activer … » (`panels::feature_switch`) — ce que l'hôte persiste
    /// (`config::OverlayConfig::set_features`) et transmet au thread Engine
    /// (`engine_thread::EngineCommand::SetFeatures`).
    pub features: FeatureToggles,
    /// État des deux cases « Couper le son des notifications » (`panels::notifications`) — ce que
    /// l'hôte persiste (`config::OverlayConfig::set_alert_mutes`) et transmet au thread Engine
    /// (`engine_thread::EngineCommand::SetAlertMutes`).
    pub mutes: AlertMutes,
    /// Les raccourcis tels qu'ils sont dans le brouillon au moment du clic — déjà garantis SANS
    /// DOUBLON (la validation est refusée sur place sinon, voir `show`), mais pas garantis
    /// enregistrables : c'est l'OS qui tranche, et l'hôte qui encaisse un refus
    /// (`shortcuts::ShortcutRegistry::apply`).
    pub shortcuts: ShortcutBindings,
    /// État de la case « Installer automatiquement les mises à jour au démarrage » — ce que
    /// l'hôte persiste (`config::OverlayConfig::auto_update`) ; il ne s'applique qu'au prochain
    /// lancement.
    pub auto_update: bool,
}

/// Ce que la modale doit recevoir de l'hôte pour peindre ses onglets.
///
/// Seul l'onglet « Alertes » en a besoin — il liste de vrais objets, avec leurs icônes descendues
/// du CDN et leur rareté lue au catalogue. L'onglet « Paramètres », lui, n'a jamais eu besoin de
/// rien : c'est pourquoi `show` s'en passait jusqu'au 2026-09-12.
pub struct OptionsModalContext<'a> {
    pub catalog: &'a overlay_engine::CatalogIndex,
    pub remote_icons: &'a crate::remote_icons::RemoteIconStore,
    pub remote_icon_textures: &'a mut crate::remote_icons::RemoteIconTextures,
    /// Repli quand l'icône d'un objet n'est pas encore descendue.
    pub icons: &'a crate::ui_icons::UiIcons,
}

/// Clé mémoire « le focus initial a déjà été donné » — voir [`show`].
fn focus_given_id() -> egui::Id {
    egui::Id::new("options-modal-focus-initial")
}

/// Peint la modale dans TOUT le rectangle disponible de `ui` (fenêtre OS dédiée, voir doc de
/// module) et renvoie l'action déclenchée par cette frame, le cas échéant.
pub fn show(
    ui: &mut egui::Ui,
    state: &mut OptionsModalState,
    ctx: &mut OptionsModalContext<'_>,
) -> OptionsModalAction {
    let mut action = OptionsModalAction::None;
    let window = ui.max_rect();
    // **Y avait-il un dialogue ouvert en ARRIVANT dans cette frame ?**
    //
    // Capturé ici, avant que quoi que ce soit ne l'ouvre ou ne le ferme, parce que le filet clavier
    // du bas s'en sert — et que le lire à ce moment-là consommait le même appui DEUX fois : Échap
    // fermait la garde (« Non »), après quoi le filet voyait un état sans dialogue, relisait le
    // même Échap et rouvrait la garde. La boîte semblait ne jamais se fermer. Attrapé par
    // `options_garde_de_fermeture_au_clavier`.
    //
    // **Deux dialogues possibles, jamais en même temps** : la garde de fermeture et, depuis le
    // 2026-09-13, la confirmation de déconnexion (section « Compte » de l'onglet « Paramètres »).
    // Elles s'excluent par construction (voir leur `else if` plus bas) ; la capture ci-dessus vaut
    // pour l'une comme pour l'autre — c'est le double appui d'Échap qu'elle empêche.
    let dialogue_a_l_entree =
        state.pending_close || state.pending_disconnect || state.pending_install.is_some();

    // Première frame de CETTE modale ? Sert au focus initial du champ de chemin (voir plus bas).
    // Le drapeau vit dans la mémoire egui du contexte, qui est neuf à chaque ouverture : la modale
    // a sa propre fenêtre OS, créée à l'ouverture et détruite à la fermeture (voir
    // `main.rs::open_options_modal` / `PostRedraw::CloseOptions`). Rouvrir la modale redonne donc
    // bien le focus, refermer et rouvrir n'en garde aucune trace.
    let first_frame = !ui.data_mut(|d| {
        let seen = d.get_temp::<bool>(focus_given_id()).unwrap_or(false);
        d.insert_temp(focus_given_id(), true);
        seen
    });

    // Tout le décor de la fenêtre — `design::window` depuis le 2026-09-10 (lot 1 de
    // `docs/plan-composants-ui.md`). Il vivait ici, dans une fonction `chrome()` de ce panneau,
    // faute d'une place dans le design system pour un composant qui encadre du contenu ; le
    // contrat en a une depuis (§1 bis, composant conteneur). Le déplacement est à pixel constant,
    // ce que les deux snapshots de cette modale vérifient à chaque exécution.
    let chrome = design::window("Options")
        .footer("Annuler", "Valider")
        // La croix en haut à droite, comme sur toutes les fenêtres du jeu (demande du
        // 2026-09-13, sur captures) — et elle fait exactement ce que fait « Annuler ».
        .close_button(true)
        // Numéro de version à gauche de la bannière, à la même distance du bord que la croix à
        // droite (2026-09-14) : la fenêtre Options est la seule « racine » de l'overlay, donc le
        // seul endroit où un utilisateur peut lire quelle version tourne sans ouvrir son journal.
        .version(true)
        .log_name("options")
        .show(ui);

    // « Personnages » est le dernier onglet sans contenu porté. Il reste affiché désactivé plutôt
    // que masqué — un onglet qui apparaît est un changement de mise en page, pas un changement
    // d'état.
    chrome.tabs(
        ui,
        design::tabs(&mut state.tab)
            .entry(OptionsTab::Suivi, "Suivi")
            .entry(OptionsTab::Alertes, "Alertes")
            .entry(OptionsTab::Chat, "Chat")
            .entry(OptionsTab::Personnages, "Personnages")
            .enabled(false)
            .entry(OptionsTab::Raccourcis, "Raccourcis")
            .entry(OptionsTab::Parametres, "Paramètres")
            .log_name("options-onglets"),
    );

    match chrome.footer {
        // **« Annuler » ne ferme plus tout de suite quand il y a des modifications en attente** :
        // il ouvre la garde. C'est le même bouton, mais ce qu'il abandonne n'est plus rien. La
        // croix de la bannière est ce même bouton, ailleurs — même garde, même action.
        design::FooterClick::Cancel => {
            if state.is_dirty() {
                state.pending_close = true;
            } else {
                action = OptionsModalAction::Cancel;
            }
        }
        design::FooterClick::Validate => action = state.validate(),
        design::FooterClick::None if chrome.close => {
            if state.is_dirty() {
                state.pending_close = true;
            } else {
                action = OptionsModalAction::Cancel;
            }
        }
        design::FooterClick::None => {}
    }

    // Un seul panneau de section, deux contenus — c'est l'onglet qui décide. Le focus initial du
    // champ de chemin ne se donne qu'à la première frame de la fenêtre : ouverte sur « Alertes »
    // (le défaut d'`OptionsTab`), elle ne le donne donc à personne, et « Paramètres » se clique.
    let mut suivi_action = suivi_tab::SuiviTabAction::None;
    design::panel().show(ui, chrome.content, |ui, panel| {
        if state.tab == OptionsTab::Chat {
            // Même arbitrage que pour les alertes : tant que les recherches ne sont pas descendues
            // du compte, l'onglet affiche son rouage.
            let mut vide = ChatDraft::default();
            let availability = state.chat_availability;
            let draft = state.chat_draft.as_mut().unwrap_or(&mut vide);
            chat_tab::show(
                ui,
                panel,
                &mut state.chat,
                &mut chat_tab::ChatTabContext {
                    draft,
                    availability,
                    enabled: &mut state.features.chat,
                },
            );
            return;
        }
        if state.tab == OptionsTab::Suivi {
            // Même arbitrage que pour les alertes : tant que les entrées ne sont pas descendues du
            // compte, l'onglet affiche son rouage. Une liste vide servie en attendant se lirait
            // comme « vous ne suivez rien ».
            let mut vide = Vec::new();
            let availability = state.suivi_availability;
            let entries = state.suivi_draft.as_mut().unwrap_or(&mut vide);
            suivi_action = suivi_tab::show(
                ui,
                panel,
                &mut state.suivi,
                &mut suivi_tab::SuiviTabContext {
                    entries,
                    catalog: ctx.catalog,
                    remote_icons: ctx.remote_icons,
                    remote_icon_textures: ctx.remote_icon_textures,
                    icons: ctx.icons,
                    availability,
                    enabled: &mut state.features.suivi,
                },
            );
            return;
        }
        if state.tab == OptionsTab::Alertes {
            // Le brouillon n'existe pas tant que les réglages ne sont pas descendus du compte :
            // l'onglet le sait et affiche son rouage. Un `AlertProfile` par défaut servi en
            // attendant afficherait dix objets que la réponse pourrait démentir.
            let mut vide = overlay_engine::AlertProfile::default();
            let availability = state.alerts_availability;
            let profile = state.alerts_draft.as_mut().unwrap_or(&mut vide);
            alerts_tab::show(
                ui,
                panel,
                &mut state.alerts,
                &mut AlertsTabContext {
                    profile,
                    catalog: ctx.catalog,
                    remote_icons: ctx.remote_icons,
                    remote_icon_textures: ctx.remote_icon_textures,
                    icons: ctx.icons,
                    availability,
                    window,
                    enabled: &mut state.features.alerts,
                },
            );
            return;
        }
        if state.tab == OptionsTab::Raccourcis {
            raccourcis_tab::show(ui, panel, &mut state.raccourcis, &mut state.shortcuts);
            return;
        }
        // **Tout l'onglet défile** depuis le 2026-09-15 : il portait quatre sections, il en porte
        // sept — « Fichier », « Combat », une section de notifications par fonctionnalité
        // (`panels::notifications`), « Mise à jour » et « Compte ». La dernière tombait hors de la
        // fenêtre sans que rien ne le dise, et agrandir la fenêtre pour suivre chaque réglage
        // ajouté n'est pas une option : c'est une fenêtre posée par-dessus un jeu.
        panel.scroll_area(ui, "options-parametres", |ui, width| {
            // La largeur utile vient de la zone défilable : la réserve de barre y est déjà
            // déduite (voir `design::PanelZones::scroll_area`).
            let inner_width = width;
            ui.add(design::heading("Fichier"));

            // Le champ et le bouton partagent une ligne : le bouton prend sa largeur naturelle
            // (libellé + marges du design system, voir `Button::desired_size`) et le champ occupe tout
            // le reste. C'est le bouton qui commande, pas l'inverse — une largeur figée pour lui
            // désaccorderait le couple dès que le libellé ou la fenêtre changent.
            let browse = design::button("Parcourir")
                .variant(ButtonVariant::Secondary)
                .size(ButtonSize::Height(ROW_HEIGHT))
                .log_name("options-parcourir");
            let browse_width = browse.desired_size(ui).x;
            let row_rect = ui.allocate_space(egui::vec2(inner_width, ROW_HEIGHT)).1;
            // Plancher à zéro : si la ligne devenait plus étroite que le bouton, un rectangle de
            // largeur négative serait inversé par egui et peint n'importe où. Zéro le rend invisible,
            // ce qui se voit sur une capture — c'est la règle du contrat de composant.
            let field_width = (row_rect.width() - browse_width - FIELD_TO_BROWSE_GAP).max(0.0);
            // Le champ garde sa hauteur native (25px) et se centre sur la ligne, que le bouton fixe à
            // 36 : le jeu compose réellement des lignes où le champ est plus bas que ce qui
            // l'accompagne (voir `design::components::input`), et la règle du design system est que la
            // hauteur d'un composant est celle de sa référence — pas celle de son voisin.
            let field_rect = egui::Rect::from_center_size(
                egui::pos2(row_rect.left() + field_width / 2.0, row_rect.center().y),
                egui::vec2(field_width, FIELD_HEIGHT),
            );
            let browse_rect = egui::Rect::from_min_size(
                egui::pos2(row_rect.right() - browse_width, row_rect.top()),
                egui::vec2(browse_width, ROW_HEIGHT),
            );
            // Focus initial dans le champ à l'ouverture : la modale est la SEULE fenêtre overlay
            // focalisable (§9.1 du plan, `WS_EX_NOACTIVATE` délibérément omis pour elle), et son unique
            // réglage est ce champ — devoir cliquer dedans avant de pouvoir taper n'a aucune raison
            // d'être. Une seule frame, sinon le champ reprendrait le focus indéfiniment.
            ui.put(
                field_rect,
                design::input(&mut state.path_input)
                    .placeholder("Chemin vers wakfu.log")
                    .width(field_width)
                    // Un chemin se retape rarement à partir de l'ancien : la croix vide d'un geste.
                    .clearable(true)
                    // Le champ porte l'alerte en même temps que le message ci-dessous : celui-ci est
                    // sous le bouton « Parcourir » et hors du regard de qui vient de taper.
                    .error(state.error.is_some())
                    .request_focus(first_frame)
                    .log_name("options-chemin"),
            );

            if ui.put(browse_rect, browse).clicked() {
                action = OptionsModalAction::Browse;
            }

            // Message d'erreur — `design::info_text` au ton alerte depuis le 2026-09-10. C'était le
            // dernier texte de la modale à échapper au design system : un `ui.label` à la police
            // proportionnelle d'egui, au corps 13 arbitraire, sans pastille ni interligne relevé. Son
            // rouge (`#e06055`) n'appartenait à aucune capture ; celui du composant est le rouge mesuré
            // du bouton « Annuler ».
            if let Some(err) = &state.error {
                ui.add_space(INFO_GAP);
                ui.add(
                    design::info_text(err)
                        .tone(design::InfoTone::Alert)
                        .width(inner_width)
                        .log_name("options-erreur"),
                );
            }

            // **Section « Combat »** (2026-09-14) — tout ce que l'overlay fait autour d'un combat,
            // en un seul endroit.
            //
            // Elle s'appelait « Affichage » et ne portait qu'une case, qui parlait déjà du panneau de
            // COMBAT ; la notification de tour arrivée le même jour en aurait fait une deuxième
            // section sur le même sujet. **Fusionnées sur décision de l'utilisateur** le 2026-09-14 :
            // « déplace la case Affichage dans la section Combat ». « Affichage » disparaît donc, elle
            // n'avait rien d'autre à porter.
            //
            // Les deux réglages vivent dans la config LOCALE (`config::OverlayConfig`), pas sur le
            // compte, pour deux raisons voisines : ce qu'on accepte de voir par-dessus son jeu dépend
            // de l'écran qu'on a devant soi, et une notification du système — le seul effet de
            // l'overlay qui sorte de l'écran de jeu — dépend de la machine (démon de notifications
            // présent ou non, téléphone apparié…). Jamais du joueur.
            //
            // Les deux cases sont des brouillons comme le reste de cette fenêtre : elles basculent
            // librement, et seul « Valider » l'emporte (voir `OptionsCommit`). Leur retour
            // (`changed()`) n'est donc pas lu — il n'y a rien à déclencher à la bascule.
            //
            // Ordre : l'affichage permanent d'abord (ce qu'on voit en dehors d'un combat), la
            // notification ensuite (ce qui arrive pendant) — du plus passif au plus intrusif.
            ui.add_space(SECTION_GAP);
            ui.add(design::heading("Combat"));
            ui.add(
                design::checkbox(
                    &mut state.combat_always_visible,
                    "Afficher le panneau de combat en dehors des combats",
                )
                .tooltip(
                    "Décoché, le panneau de combat n'apparaît qu'au début d'un combat et se referme \
                     quand il est terminé.",
                )
                .log_name("options-combat-toujours-visible"),
            );
            // **L'interligne des lignes d'option** (2026-09-14) — voir `tokens::CHECKBOX_ROW_GAP` : le
            // jeu laisse 11px entre deux cases, pas zéro. Posé ici et pas dans `design::checkbox`
            // parce que le relevé le range du côté de la mise en page, et parce qu'un écart porté par
            // le composant s'ajouterait au `SECTION_GAP` qui suit la dernière ligne d'un bloc.
            ui.add_space(design::tokens::CHECKBOX_ROW_GAP);
            ui.add(
                design::checkbox(
                    &mut state.turn_notification,
                    "Me prévenir quand un de mes personnages doit jouer",
                )
                .tooltip(
                    "Une notification du système annonce le personnage dont c'est le tour, uniquement \
                     si sa fenêtre de jeu n'est pas celle que vous avez sous les yeux.",
                )
                .log_name("options-notification-de-tour"),
            );
            // **Le son, sous la notification et en retrait** (demande du 2026-09-14) : la case ne
            // vaut que si la notification est active — grisée sinon, sans changer de valeur (un son
            // coupé le reste si on désactive puis réactive la notification). Sa case commence là où
            // commence le LIBELLÉ de la case du dessus (case + écart, voir `design::checkbox`) : la
            // première version reprenait le retrait titre → contrôle (7 px), jugé trop faible au
            // rendu — « aligner la partie gauche avec le début du m d'en haut ».
            ui.add_space(design::tokens::CHECKBOX_ROW_GAP);
            ui.horizontal(|ui| {
                ui.add_space(design::tokens::CHECKBOX_SIZE + design::tokens::CHECKBOX_LABEL_GAP);
                ui.add(
                    design::checkbox(
                        &mut state.turn_notification_muted,
                        notifications::MUTE_LABEL,
                    )
                    .enabled(state.turn_notification)
                    .tooltip("La notification s'affiche sans jouer de son.")
                    .log_name("options-notification-de-tour-sans-son"),
                );
            });

            // **Les trois sections de notifications** (2026-09-15) — le Suivi, les Alertes et le
            // Chat, dans l'ordre du menu d'onglets, juste après « Combat » qui porte déjà les
            // siennes depuis le 2026-09-14.
            //
            // Demande utilisateur : « déplacer le test de son, le choix de coupure de son, et la
            // gestion du temps de fermeture de la notification de tous les onglets dans des
            // sections dédiées, après la section Combat ». Les trois onglets réglaient chacun les
            // siennes, avec des formulations qui avaient divergé ; les quatre sections d'ici
            // disent maintenant la même chose de la même façon, fonctionnalité par fonctionnalité.
            //
            // Ce que chaque section porte dépend de ce que sa fonctionnalité fait entendre et
            // voir, et rien n'a été uniformisé de force : le Suivi n'affiche pas de carte à
            // fermer (son alerte est un son et un bandeau permanent), et les Alertes n'ont pas de
            // sourdine globale — le son d'un ramassage se coupe déjà objet par objet, à la tuile
            // (`panels::alerts_tab`).
            //
            // Une fonctionnalité éteinte (`panels::feature_switch`) grise sa section entière : il
            // n'y a ni son à essayer ni carte à fermer quand rien ne se déclenche.
            ui.add_space(SECTION_GAP);
            ui.add(design::heading("Suivi"));
            if notifications::section(
                ui,
                inner_width,
                notifications::Section {
                    log_prefix: "suivi",
                    enabled: state.features.suivi,
                    muted: Some(&mut state.mutes.suivi),
                    auto_close: None,
                },
            ) {
                action = OptionsModalAction::TestCountdownSound;
            }

            ui.add_space(SECTION_GAP);
            ui.add(design::heading("Alertes"));
            // Le brouillon d'alertes descend du compte : tant qu'il n'est pas là, la ligne de
            // fermeture se peint grisée sur un profil de repli plutôt que d'apparaître en cours
            // de route (voir `notifications::AutoClose::available`). Le son, lui, s'essaie sans
            // compte — il ne dépend que du périphérique audio.
            let mut alertes_repli = overlay_engine::AlertProfile::default();
            let alertes_prêtes = state.alerts_draft.is_some();
            let profil = state.alerts_draft.as_mut().unwrap_or(&mut alertes_repli);
            if notifications::section(
                ui,
                inner_width,
                notifications::Section {
                    log_prefix: "alertes",
                    enabled: state.features.alerts,
                    muted: None,
                    auto_close: Some(notifications::AutoClose {
                        available: alertes_prêtes,
                        settings: profil,
                        input: &mut state.alerts.duration_input,
                    }),
                },
            ) {
                action = OptionsModalAction::TestAlertSound;
            }

            ui.add_space(SECTION_GAP);
            ui.add(design::heading("Chat"));
            let mut chat_repli = ChatDraft::default();
            let chat_prêt = state.chat_draft.is_some();
            let chat = state.chat_draft.as_mut().unwrap_or(&mut chat_repli);
            if notifications::section(
                ui,
                inner_width,
                notifications::Section {
                    log_prefix: "chat",
                    enabled: state.features.chat,
                    muted: Some(&mut state.mutes.chat),
                    auto_close: Some(notifications::AutoClose {
                        available: chat_prêt,
                        settings: &mut chat.toast,
                        input: &mut state.chat.duration_input,
                    }),
                },
            ) {
                action = OptionsModalAction::TestChatSound;
            }

            // **Section « Mise à jour »** (2026-09-15, `docs/plan-mise-a-jour.md` §8.2, décisions du
            // mainteneur) : la version courante n'est PAS rappelée ici, la bannière de la fenêtre la
            // porte déjà. Une ligne d'information (dernière vérification, version disponible et son
            // poids), la case d'installation automatique, et UN bouton dont le libellé suit l'état :
            // « Recherche de mise à jour » → « Recherche… » → « Mettre à jour vers X » /
            // « Réessayer ». Pas de bouton « Notes de version » pour l'instant (aucune note n'est
            // rédigée aujourd'hui). L'habillage du bouton de recherche est à revoir avec le design
            // system, plus tard.
            //
            // Comme « Se déconnecter » (section « Compte », dessous), « Mettre à jour » n'est PAS un brouillon : il ferme l'overlay
            // de jeu le temps de l'installation — d'où sa confirmation. « Recherche », lui, ne touche
            // à rien.
            ui.add_space(SECTION_GAP);
            ui.add(design::heading("Mise à jour"));
            let (info, tone) = update_info_line(&state.update, std::time::Instant::now());
            ui.add(
                design::info_text(info)
                    .tone(tone)
                    .width(inner_width)
                    .log_name("options-mise-a-jour-info"),
            );
            ui.add_space(INFO_GAP);
            ui.add(
                design::checkbox(
                    &mut state.auto_update,
                    "Installer automatiquement les mises à jour au démarrage",
                )
                .tooltip(
                    "Au lancement, une version plus récente est téléchargée et installée avant \
                     d'ouvrir l'overlay. Décochée, elle est seulement signalée ici.",
                )
                .log_name("options-mise-a-jour-auto"),
            );
            ui.add_space(design::tokens::CHECKBOX_ROW_GAP);
            let button_spec = update_button(&state.update);
            let update_button = design::button(button_spec.label)
                .variant(button_spec.variant)
                .size(ButtonSize::Height(ROW_HEIGHT))
                .enabled(button_spec.enabled)
                .tooltip(button_spec.tooltip)
                .log_name("options-mise-a-jour-bouton");
            let update_size = update_button.desired_size(ui);
            let row = ui.allocate_space(egui::vec2(inner_width, ROW_HEIGHT)).1;
            if ui
                .put(
                    egui::Rect::from_center_size(row.center(), update_size),
                    update_button,
                )
                .clicked()
            {
                match &state.update {
                    UpdateStatus::Available { version, .. } => {
                        state.pending_install = Some(version.clone());
                    }
                    _ => action = OptionsModalAction::CheckUpdate,
                }
            }

            // **Section « Compte »** (2026-09-13) — la déconnexion, qui était jusque-là un raccourci
            // global (`Ctrl+Alt+D`, voir `crate::shortcuts`). Demande utilisateur : en faire un bouton,
            // ici, avec de quoi comprendre ce qu'il fait avant de le presser.
            //
            // **Ce bouton n'est PAS un brouillon**, contrairement à tout le reste de cette fenêtre : il
            // agit tout de suite (l'hôte efface le jeton et l'overlay revient à son écran de
            // connexion), et « Annuler » ne le rattraperait pas. C'est précisément ce qui justifie la
            // confirmation qu'il ouvre — là où l'onglet « Alertes » a pu retirer la sienne, son retrait
            // d'objet étant annulable jusqu'à « Valider » (voir `alerts_tab`, règle 4).
            ui.add_space(SECTION_GAP);
            ui.add(design::heading("Compte"));
            ui.add(
                design::info_text(
                    "L'overlay ne fonctionne qu'avec un compte connecté : c'est lui qui porte la liste \
                     suivie, les alertes et l'historique synchronisé. Vous déconnecter ramène l'overlay \
                     à son écran de connexion, et il faudra réappairer l'application pour le réutiliser.",
                )
                .tone(design::InfoTone::Info)
                .width(inner_width)
                .log_name("options-compte-info"),
            );
            ui.add_space(INFO_GAP);
            // **Rouge et centré** (demande utilisateur, 2026-09-13), là où ce bouton était secondaire
            // et aligné à gauche comme les réglages au-dessus. Les deux vont ensemble : c'est la seule
            // action de cette fenêtre qui échappe à « Annuler », et noyée dans la colonne des réglages
            // elle ne se distinguait pas d'un champ de plus. La couleur avertit, la confirmation
            // rattrape le geste — l'une ne remplace pas l'autre.
            //
            // La texture `Danger` est native en **36 px**, soit exactement `ROW_HEIGHT` : ni dégradé
            // étiré ni embout à la mauvaise échelle (voir `design::ButtonVariant::textures`).
            //
            // **Largeur naturelle**, jamais figée — même règle que la ligne de recherche de l'onglet
            // « Raccourcis » : une largeur en dur écrête le libellé dès que la police ou le mot
            // changent, ce qui s'est déjà vu (« Réinitialise »).
            let disconnect = design::button("Se déconnecter")
                .variant(ButtonVariant::Danger)
                .size(ButtonSize::Height(ROW_HEIGHT))
                .enabled(state.account_connected)
                .tooltip(if state.account_connected {
                    "Effacer la session enregistrée et revenir à l'écran de connexion"
                } else {
                    "Aucun compte connecté"
                })
                .log_name("options-deconnecter");
            let disconnect_size = disconnect.desired_size(ui);
            let row = ui.allocate_space(egui::vec2(inner_width, ROW_HEIGHT)).1;
            if ui
                .put(
                    egui::Rect::from_center_size(row.center(), disconnect_size),
                    disconnect,
                )
                .clicked()
            {
                state.pending_disconnect = true;
            }
        });
    });

    match suivi_action {
        suivi_tab::SuiviTabAction::ResolveRecipe(id) => {
            action = OptionsModalAction::ResolveRecipe(id)
        }
        suivi_tab::SuiviTabAction::None => {}
    }

    // **La fenêtre de recette, peinte EN DERNIER et sur la fenêtre entière** : son voile doit
    // passer par-dessus tout ce qu'elle interrompt, pied de page compris — même règle que la garde
    // de fermeture.
    if state.suivi_draft.is_some() {
        if let Some(mut dialogue) = state.suivi.recipe.take() {
            match recipe_dialog::show(ui, window, &mut dialogue, ctx) {
                recipe_dialog::RecipeChoice::Pending => state.suivi.recipe = Some(dialogue),
                recipe_dialog::RecipeChoice::Cancel => {}
                recipe_dialog::RecipeChoice::Track(lignes) => {
                    if let Some(brouillon) = state.suivi_draft.as_mut() {
                        suivi_tab::track_recipe_lines(brouillon, &lignes, &mut state.suivi);
                    }
                }
            }
        }
    }

    // **La confirmation de déconnexion** (2026-09-13), peinte avant la garde de fermeture et, comme
    // elle, sur la fenêtre ENTIÈRE. Elle passe en premier parce qu'elle EXCLUT la seconde : le
    // `else if` ci-dessous garantit qu'une seule boîte est à l'écran, deux voiles superposés ne
    // disant plus quel pied de page est inerte.
    //
    // Pourquoi confirmer ici, alors que l'onglet « Alertes » a retiré sa confirmation de retrait :
    // celle-là portait sur un brouillon qu'« Annuler » rattrapait ; celle-ci efface la session et
    // renvoie l'overlay à son écran de connexion, sans retour possible sans réappairer.
    if let Some(version) = state.pending_install.clone() {
        let choix = design::confirm_dialog(format!(
            "Fermer l'overlay et installer la version {version} ?"
        ))
        .over(window)
        .log_name("options.mise-a-jour")
        .show(ui);
        match choix {
            design::ConfirmChoice::Yes => {
                state.pending_install = None;
                action = OptionsModalAction::InstallUpdate;
            }
            design::ConfirmChoice::No => state.pending_install = None,
            design::ConfirmChoice::Pending => {}
        }
    } else if state.pending_disconnect {
        let choix = design::confirm_dialog("Déconnecter le compte de l'overlay ?")
            .over(window)
            .log_name("options.deconnexion")
            .show(ui);
        match choix {
            design::ConfirmChoice::Yes => {
                state.pending_disconnect = false;
                action = OptionsModalAction::Disconnect;
            }
            design::ConfirmChoice::No => state.pending_disconnect = false,
            design::ConfirmChoice::Pending => {}
        }
    }
    // **La garde de fermeture**, peinte en dernier et sur la fenêtre ENTIÈRE.
    else if state.pending_close {
        let choix = design::confirm_dialog("Abandonner les modifications en cours ?")
            .over(window)
            .log_name("options.abandon")
            .show(ui);
        match choix {
            design::ConfirmChoice::Yes => {
                state.pending_close = false;
                action = OptionsModalAction::Cancel;
            }
            design::ConfirmChoice::No => state.pending_close = false,
            design::ConfirmChoice::Pending => {}
        }
    }

    // Clavier — lu APRÈS les boutons : un clic de cette frame l'emporte sur une touche de la même
    // frame (cas de figure théorique, mais l'ordre doit être décidé plutôt que subi).
    //
    // Ces deux touches sont traitées ICI, dans le panneau, et non par l'hôte, pour deux raisons.
    // La première est le contrat (§17.3 bis du plan) : un panneau ne produit aucun effet de bord,
    // il remonte une intention — `Cancel`/`Validate` sont exactement les intentions que les boutons
    // du pied de page produisent déjà. La seconde est que l'hôte, lui, ne peut PAS distinguer un
    // Échap destiné à la modale : son filet global `Échap → event_loop.exit()` fermait l'overlay
    // entier (voir `main.rs`/`bin/overlay-ui-x11.rs`, où ce filet exclut désormais cette fenêtre).
    //
    // `TextEdit` ne retire pas ces événements de l'entrée globale (il travaille sur une copie
    // filtrée, `InputState::filtered_events`) : les lire ici reste fiable même quand le champ de
    // chemin a le focus — ce qui est le cas dès l'ouverture.
    //
    // **Deux restrictions posées le 2026-09-12, avec l'onglet « Alertes »** :
    //
    // - *Entrée* ne vaut « Valider » que sur « Paramètres ». Sur « Alertes », la touche appartient
    //   au champ d'autocomplétion, où elle choisit une suggestion — et comme `TextEdit` ne retire
    //   pas l'événement de l'entrée globale (c'est ce qui rend cette lecture fiable ici), le même
    //   appui aurait à la fois ajouté un objet ET fermé la fenêtre derrière.
    // - Ni l'une ni l'autre ne passe tant qu'une **confirmation de retrait** est ouverte : c'est
    //   elle qui prend Échap (pour se fermer), et son voile dit précisément que le pied de page
    //   est inerte.
    // - Une **confirmation ouverte** (retrait d'objet ou garde de fermeture) prend Échap pour elle :
    //   la lire ici aussi fermerait la boîte ET la fenêtre derrière, du même appui.
    if matches!(action, OptionsModalAction::None) && !dialogue_a_l_entree {
        let (cancel, validate) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Enter),
            )
        });
        if cancel {
            // Échap est un « Annuler » : il passe par la même garde que le bouton.
            if state.is_dirty() {
                state.pending_close = true;
            } else {
                action = OptionsModalAction::Cancel;
            }
        } else if validate && state.tab == OptionsTab::Parametres {
            action = state.validate();
        }
    }

    action
}

/// La ligne d'information de la section « Mise à jour » et son ton — une fonction libre, pour
/// que ses formulations soient testées sans peindre.
pub fn update_info_line(
    status: &UpdateStatus,
    now: std::time::Instant,
) -> (String, design::InfoTone) {
    let since = |at: std::time::Instant| {
        let secs = now.saturating_duration_since(at).as_secs();
        if secs < 60 {
            "à l'instant".to_string()
        } else if secs < 3600 {
            format!("il y a {} min", secs / 60)
        } else {
            format!("il y a {} h", secs / 3600)
        }
    };
    match status {
        UpdateStatus::Idle => (
            "Aucune vérification depuis le lancement.".to_string(),
            design::InfoTone::Info,
        ),
        UpdateStatus::Checking => ("Recherche en cours…".to_string(), design::InfoTone::Info),
        UpdateStatus::UpToDate { checked_at } => (
            format!(
                "Dernière vérification {} · vous êtes à jour.",
                since(*checked_at)
            ),
            design::InfoTone::Info,
        ),
        UpdateStatus::Available {
            version,
            download_size,
            mandatory,
            checked_at,
            ..
        } => (
            format!(
                "Dernière vérification {} · version {version} disponible · {}{}",
                since(*checked_at),
                update::human_size(*download_size),
                if *mandatory { " · obligatoire" } else { "" }
            ),
            design::InfoTone::Info,
        ),
        UpdateStatus::Downloading {
            version,
            received,
            total,
        } => (
            format!(
                "Téléchargement de la version {version} : {} / {}",
                update::human_size(*received),
                update::human_size(*total)
            ),
            design::InfoTone::Info,
        ),
        UpdateStatus::Verifying { version } => (
            format!("Vérification de la version {version}…"),
            design::InfoTone::Info,
        ),
        UpdateStatus::ReadyToInstall { version, .. } | UpdateStatus::Installing { version } => (
            format!("Installation de la version {version}…"),
            design::InfoTone::Info,
        ),
        UpdateStatus::Unavailable { reason, checked_at } => (
            format!(
                "Dernière vérification {} · impossible ({reason}).",
                since(*checked_at)
            ),
            design::InfoTone::Alert,
        ),
        UpdateStatus::Failed {
            headline, detail, ..
        } => (
            format!("Mise à jour impossible : {headline} ({detail})"),
            design::InfoTone::Alert,
        ),
    }
}

/// Le bouton unique de la section « Mise à jour », selon l'état.
pub struct UpdateButtonSpec {
    pub label: String,
    pub variant: ButtonVariant,
    pub enabled: bool,
    pub tooltip: &'static str,
}

pub fn update_button(status: &UpdateStatus) -> UpdateButtonSpec {
    match status {
        UpdateStatus::Available { version, .. } => UpdateButtonSpec {
            label: format!("Mettre à jour vers {version}"),
            variant: ButtonVariant::Primary,
            enabled: true,
            tooltip: "Ferme l'overlay, installe la nouvelle version et le relance",
        },
        UpdateStatus::Checking => UpdateButtonSpec {
            label: "Recherche…".to_string(),
            variant: ButtonVariant::Secondary,
            enabled: false,
            tooltip: "Lecture de la dernière version publiée",
        },
        UpdateStatus::Downloading { .. }
        | UpdateStatus::Verifying { .. }
        | UpdateStatus::ReadyToInstall { .. }
        | UpdateStatus::Installing { .. } => UpdateButtonSpec {
            label: "Mise à jour en cours…".to_string(),
            variant: ButtonVariant::Secondary,
            enabled: false,
            tooltip: "L'overlay se relancera une fois la version installée",
        },
        UpdateStatus::Failed { .. } => UpdateButtonSpec {
            label: "Réessayer".to_string(),
            variant: ButtonVariant::Secondary,
            enabled: true,
            tooltip: "Rechercher à nouveau la dernière version publiée",
        },
        UpdateStatus::Idle | UpdateStatus::UpToDate { .. } | UpdateStatus::Unavailable { .. } => {
            UpdateButtonSpec {
                label: "Recherche de mise à jour".to_string(),
                variant: ButtonVariant::Secondary,
                enabled: true,
                tooltip: "Lire la dernière version publiée, sans rien installer",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L'état d'une fenêtre qu'on vient d'ouvrir : référence et brouillon accordés, donc rien en
    /// attente.
    fn fenetre_ouverte(chemin: &str, combat_always_visible: bool) -> OptionsModalState {
        OptionsModalState {
            path_input: chemin.to_string(),
            combat_always_visible,
            initial: OptionsInitial {
                path: chemin.to_string(),
                combat_always_visible,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn la_case_de_mise_a_jour_automatique_est_un_brouillon() {
        let mut state = fenetre_ouverte("/jeu/wakfu.log", false);
        state.auto_update = true;
        state.initial.auto_update = true;
        assert!(!state.is_dirty());
        state.auto_update = false;
        assert!(state.is_dirty());
        assert!(!state.commit().auto_update);
    }

    #[test]
    fn ligne_et_bouton_de_la_section_mise_a_jour_suivent_l_etat() {
        use overlay_sync::update::UpdateStatus;
        let now = std::time::Instant::now();
        let (ligne, _) = update_info_line(
            &UpdateStatus::UpToDate { checked_at: now },
            now + std::time::Duration::from_secs(185),
        );
        assert_eq!(
            ligne,
            "Dernière vérification il y a 3 min · vous êtes à jour."
        );
        assert_eq!(
            update_button(&UpdateStatus::Idle).label,
            "Recherche de mise à jour"
        );
        let disponible = UpdateStatus::Available {
            version: "0.21.0".into(),
            download_size: 3_100_000,
            mandatory: false,
            notes_url: None,
            checked_at: now,
        };
        let (ligne, tone) = update_info_line(&disponible, now);
        assert_eq!(
            ligne,
            "Dernière vérification à l'instant · version 0.21.0 disponible · 3,1 Mo"
        );
        assert_eq!(tone, design::InfoTone::Info);
        let bouton = update_button(&disponible);
        assert_eq!(bouton.label, "Mettre à jour vers 0.21.0");
        assert!(bouton.enabled);
        assert_eq!(bouton.variant, ButtonVariant::Primary);
        assert!(!update_button(&UpdateStatus::Checking).enabled);
        let (_, tone) = update_info_line(
            &UpdateStatus::Unavailable {
                reason: "hors ligne".into(),
                checked_at: now,
            },
            now,
        );
        assert_eq!(tone, design::InfoTone::Alert);
    }

    #[test]
    fn une_fenetre_intouchee_n_a_rien_en_attente() {
        assert!(!fenetre_ouverte("/jeu/wakfu.log", false).is_dirty());
        // Y compris quand le réglage est actif : la référence part de l'état EN VIGUEUR, pas du
        // défaut — sinon la garde de fermeture s'ouvrirait dès l'ouverture chez qui a coché.
        assert!(!fenetre_ouverte("/jeu/wakfu.log", true).is_dirty());
    }

    #[test]
    fn cocher_la_case_met_des_modifications_en_attente() {
        let mut state = fenetre_ouverte("/jeu/wakfu.log", false);
        state.combat_always_visible = true;
        assert!(
            state.is_dirty(),
            "fermer après avoir coché doit passer par la garde, comme pour le chemin de log"
        );
        // Recochée dans l'autre sens, la fenêtre redevient intouchée : la garde ne s'ouvre pas
        // pour un aller-retour.
        state.combat_always_visible = false;
        assert!(!state.is_dirty());
    }

    #[test]
    fn valider_emporte_le_chemin_et_la_case() {
        let mut state = fenetre_ouverte("/jeu/wakfu.log", false);
        state.combat_always_visible = true;
        state.turn_notification = true;
        assert_eq!(
            state.commit(),
            OptionsCommit {
                path: "/jeu/wakfu.log".to_string(),
                combat_always_visible: true,
                turn_notification: true,
                turn_notification_muted: false,
                features: FeatureToggles::default(),
                mutes: AlertMutes::default(),
                shortcuts: ShortcutBindings::default(),
                auto_update: false,
            }
        );
    }

    /// **Décocher une fonctionnalité est un brouillon comme le reste de la fenêtre** : la garde de
    /// fermeture s'ouvre tant que « Valider » n'a pas emporté la bascule, et l'aller-retour la
    /// referme. Sans quoi couper le Suivi par erreur, puis fermer, le couperait pour de bon.
    #[test]
    fn couper_une_fonctionnalite_met_des_modifications_en_attente() {
        let mut state = fenetre_ouverte("/jeu/wakfu.log", false);
        // Une fenêtre qui s'ouvre sur « tout actif » n'a rien en attente — c'est le défaut de
        // `FeatureToggles`, des deux côtés (brouillon et référence).
        assert!(!state.is_dirty());

        state.features.suivi = false;
        assert!(
            state.is_dirty(),
            "décocher « Activer le suivi » doit ouvrir la garde de fermeture"
        );
        assert!(!state.commit().features.suivi, "« Valider » l'emporte");

        state.features.suivi = true;
        assert!(
            !state.is_dirty(),
            "recocher ramène la fenêtre à son état d'ouverture"
        );
    }

    /// **Couper le son d'une alerte est un brouillon comme le reste de la fenêtre** : la garde de
    /// fermeture s'ouvre tant que « Valider » n'a pas emporté la bascule, et l'aller-retour la
    /// referme. La sourdine du Suivi et celle du Chat sont indépendantes — couper l'une ne doit
    /// pas faire taire l'autre.
    #[test]
    fn couper_le_son_dune_alerte_met_des_modifications_en_attente() {
        let mut state = fenetre_ouverte("/jeu/wakfu.log", false);
        assert!(!state.is_dirty());

        state.mutes.suivi = true;
        assert!(
            state.is_dirty(),
            "cocher « Couper le son des notifications » doit ouvrir la garde de fermeture"
        );
        let commit = state.commit();
        assert!(commit.mutes.suivi, "« Valider » l'emporte");
        assert!(!commit.mutes.chat, "l'autre onglet garde son son");

        state.mutes.suivi = false;
        assert!(
            !state.is_dirty(),
            "décocher ramène la fenêtre à son état d'ouverture"
        );
    }

    /// La case « Couper le son des notifications » est un brouillon comme sa voisine, et se
    /// garde même quand la notification est décochée : « Valider » l'emporte telle quelle.
    #[test]
    fn couper_le_son_est_un_brouillon_independant_de_la_notification() {
        let mut state = fenetre_ouverte("/jeu/wakfu.log", false);
        state.turn_notification_muted = true;
        assert!(state.is_dirty());
        assert!(state.commit().turn_notification_muted);
        state.turn_notification_muted = false;
        assert!(!state.is_dirty());
    }

    /// La case « Me prévenir quand un de mes personnages doit jouer » suit la même mécanique de
    /// brouillon que sa voisine : la garde de fermeture s'ouvre si on la bascule, et se referme
    /// sur un aller-retour.
    #[test]
    fn cocher_la_notification_de_tour_met_des_modifications_en_attente() {
        let mut state = fenetre_ouverte("/jeu/wakfu.log", false);
        state.turn_notification = true;
        assert!(state.is_dirty());
        state.turn_notification = false;
        assert!(!state.is_dirty());
    }

    /// Toucher un raccourci met des modifications en attente, au même titre que le chemin ou la
    /// case : fermer sans valider doit passer par la garde.
    #[test]
    fn changer_un_raccourci_met_des_modifications_en_attente() {
        let mut state = fenetre_ouverte("/jeu/wakfu.log", false);
        assert!(!state.is_dirty());
        state.shortcuts.set(
            crate::shortcuts::ShortcutAction::Quit,
            crate::shortcuts::Shortcut::parse("Ctrl+Alt+K").expect("combinaison de test valide"),
        );
        assert!(state.is_dirty());
        // Remis comme avant, la fenêtre redevient intouchée — la garde ne s'ouvre pas pour un
        // aller-retour.
        state.shortcuts = state.initial.shortcuts.clone();
        assert!(!state.is_dirty());
    }

    /// Ajouter une recherche de chat met des modifications en attente, comme un objet d'alerte.
    #[test]
    fn ajouter_une_recherche_de_chat_met_des_modifications_en_attente() {
        let mut state = fenetre_ouverte("/jeu/wakfu.log", false);
        state.chat_draft = Some(ChatDraft::default());
        state.initial.chat = Some(ChatDraft::default());
        assert!(!state.is_dirty());
        state
            .chat_draft
            .as_mut()
            .expect("brouillon posé juste avant")
            .add(overlay_engine::ChatFilterScope::All, "gelano")
            .expect("ajout d'une recherche");
        assert!(state.is_dirty());
        state.chat_draft = state.initial.chat.clone();
        assert!(!state.is_dirty());
    }

    /// Deux actions sur la même combinaison : « Valider » REFUSE sur place (l'OS rejetterait le
    /// second enregistrement), bascule sur l'onglet concerné et porte le message.
    #[test]
    fn valider_refuse_deux_raccourcis_identiques() {
        let mut state = fenetre_ouverte("/jeu/wakfu.log", false);
        state.tab = OptionsTab::Parametres;
        let toggle = crate::shortcuts::ShortcutAction::Toggle.default_shortcut();
        state
            .shortcuts
            .set(crate::shortcuts::ShortcutAction::Details, toggle);

        assert_eq!(state.validate(), OptionsModalAction::None);
        assert_eq!(state.tab, OptionsTab::Raccourcis);
        assert!(state
            .raccourcis
            .error
            .as_deref()
            .is_some_and(|message| message.contains(&toggle.label())));

        // Conflit levé : la validation repasse, et le message s'efface.
        state.shortcuts = ShortcutBindings::default();
        assert!(matches!(state.validate(), OptionsModalAction::Validate(_)));
        assert_eq!(state.raccourcis.error, None);
    }
}
