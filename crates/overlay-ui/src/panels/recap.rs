//! **Récap de session** — le bloc posé en haut à gauche de la fenêtre de jeu, sous les boutons
//! d'interface du client (2026-09-16, demande utilisateur).
//!
//! Cinq chiffres, et cinq seulement : XP gagnée, kamas nets, combats gagnés − perdus, challenges
//! réussis − échoués, durée de la session. C'est exactement la « bande coup d'œil » du web
//! (`session-recap.component.html`, bloc `.recap-bandeau`), réduite à ce qui se lit d'un regard
//! par-dessus un jeu : le site, lui, déplie sous cette bande l'XP par personnage, le détail des
//! kamas, le butin et les accordéons par donjon — de la consultation APRÈS coup, pas du temps réel.
//!
//! ## Mise en page : deux colonnes, trois lignes (2026-09-16, soir)
//!
//! La première version alignait les cinq cases sur une seule ligne, séparées par des filets.
//! Refaite le soir même sur maquette de l'utilisateur, puis retouchée sur sa capture annotée :
//!
//! ```text
//! KAMAS             EXPERIENCE
//! COMBATS           CHALLENGES
//!           DUREE
//! ```
//!
//! Les deux premières lignes sont coupées en **deux moitiés égales**, et chaque case est centrée
//! dans sa moitié (« comme le système flex », retour utilisateur du 2026-09-16 tard) : kamas et
//! combats dans la moitié gauche, XP et challenges dans la droite. La durée, seule sur la
//! troisième ligne, est centrée sur toute la largeur. Chaque case garde son glyphe à gauche de son
//! chiffre ; **les glyphes restent blancs**, seul le chiffre porte une couleur : or pour les
//! kamas, accent pour l'XP et la durée, vert/rouge pour les deux cases « gagné − perdu ». (La
//! version du soir teintait le glyphe comme son chiffre — retirée sur retour utilisateur : « les
//! icônes doivent être en blanc ».)
//!
//! **Largeur FIXE** ([`WIDTH`]) : le bloc fait exactement la largeur de la rangée de boutons du
//! jeu sous laquelle il est posé (Menu … Boutique), bord à bord — c'est ce que l'utilisateur a
//! tracé sur sa capture. La largeur ne suit donc plus les chiffres ; c'est la HAUTEUR qui bouge :
//! dès qu'une case déborde de sa moitié (des millions d'XP, des millions de kamas), les deux cases
//! de sa ligne s'empilent l'une au-dessus de l'autre, chacune centrée sur toute la largeur, et le
//! bloc gagne une ligne — plutôt que deux chiffres qui se chevauchent. Voir [`show`].
//!
//! **Une section « Recap » commande son affichage** (onglet « Paramètres » de la fenêtre Options,
//! en tête de l'onglet) — voir `panels::feature_switch::FeatureToggles::recap`, active par
//! défaut comme ses voisines.
//!
//! ## Trois cases qu'on peut éteindre — [`RecapCells`] (2026-09-16, tard)
//!
//! Demande utilisateur : « options pour afficher la durée de la session, pour afficher les
//! combats (victoire / défaite), pour afficher les challenges (réussi / échoué) ». Trois cases à
//! cocher sous « Activer le récap de session », en retrait comme le suivi des sorts sous le détail
//! des combats, toutes cochées par défaut. Kamas et XP ne se règlent pas : ce sont les deux chiffres
//! pour lesquels la bande existe.
//!
//! La grille se **recompose** sur ce qui reste, elle ne laisse pas de trou :
//!
//! ```text
//! KAMAS   EXPERIENCE        KAMAS   EXPERIENCE        KAMAS   EXPERIENCE
//! COMBATS CHALLENGES            CHALLENGES               DUREE
//!       DUREE                     DUREE
//! ```
//!
//! Une paire dont une case est éteinte laisse l'autre **seule sur sa ligne, centrée sur toute la
//! largeur** — comme la durée l'est déjà ; une ligne dont les deux cases sont éteintes disparaît,
//! et le bloc perd sa hauteur (une ligne au minimum, Kamas / XP ne s'éteignant pas). C'est la
//! même mécanique que l'empilement « responsive » (voir [`layout_rows`]) : une ligne porte une ou
//! deux cases, et une case seule prend toute la largeur.
//!
//! ## La durée, et les quatre autres chiffres, viennent de la SESSION
//!
//! Depuis le 2026-09-17, les cinq chiffres ont la même vie : celle de `crate::recap_session`
//! (« la session, c'est ce que l'overlay a vu du jeu » — chrono qui n'avance que fenêtre de jeu
//! présente, reprise après une pause tolérée, remise à zéro à la demande). Ce bloc n'en connaît
//! que la vue ([`RecapView`]) : les totaux de session, le chrono, l'heure de début et le nombre de
//! reprises pour l'infobulle de la durée — « Session depuis 20:12 », et « reprise 2 fois » sur une
//! seconde ligne dès qu'il y en a eu une.
//!
//! Le premier jour (2026-09-16), la durée était le temps d'exécution du processus
//! (`App::started_at`) et les quatre autres chiffres couvraient tout le `wakfu.log` relu au
//! démarrage — un écart assumé faute d'avoir tranché ce qu'est « la session ». C'est tranché.
//!
//! ## Le glyphe de remise à zéro
//!
//! Un seul élément cliquable : le glyphe `Undo` au bout de la dernière ligne (celle de la durée
//! quand elle est affichée — la seule qui a de la place, et celle que ça remet à zéro en
//! premier), blanc comme les autres, or au survol (`tokens::ICON_TINT_HOVER`), infobulle
//! « Remettre à zéro » au-dessus. Il n'est pas une sixième case : hors de [`layout_rows`], posé à
//! `WIDTH − PADDING_X − ICON_SIZE` quel que soit le nombre de lignes. Si la dernière ligne porte
//! une case qui le toucherait (la durée éteinte : c'est alors Combats / Challenges, ou Kamas / XP),
//! cette ligne se range dans la largeur qui reste à gauche du glyphe — les autres lignes ne bougent
//! pas. Un clic ne remet rien à zéro ici : il est REMONTÉ ([`RecapOutcome::reset_requested`]) et
//! l'hôte ouvre une boîte de confirmation par-dessus toute la fenêtre de jeu
//! (`OverlayKind::ResetConfirm`) — décision utilisateur du 2026-09-17 : « impose une confirmBox
//! centrée au jeu avec un fond voilé sur toute la fenêtre du jeu et des overlays ».
//!
//! ## Glisser la bande où l'on veut (2026-09-17)
//!
//! Demande utilisateur : « placer l'overlay de recap via du drag & drop », et le retrouver au
//! même endroit après un redémarrage. **Tout le fond de la bande est saisissable** — pas de
//! poignée dédiée : le bloc fait 206 px de large, calés au pixel sur la rangée de boutons du
//! jeu, et il n'y a pas de place à prendre pour un glyphe qui ne dirait rien de plus qu'un
//! curseur « main ». Le curseur passe donc à `Grab` au survol du fond et à `Grabbing` pendant le
//! glissement, et les infobulles des cases continuent de s'ouvrir par-dessus : un survol et un
//! glissement ne se disputent rien.
//!
//! **Le glyphe de remise à zéro garde la priorité** : il capte le glissement comme le clic (voir
//! [`paint_reset_button`]), de sorte qu'un appui dessus ne fasse jamais partir la bande.
//!
//! Comme la remise à zéro, ce module **ne déplace rien lui-même** : il remonte le geste
//! ([`RecapDrag`]) et c'est l'hôte qui bouge la fenêtre OS, la borne à la fenêtre de jeu, et
//! écrit la position dans la config au relâchement (`config::OverlayConfig::recap_position`).
//! Il ne connaît donc pas non plus sa propre position à l'écran — le harnais de captures, qui
//! rend le bloc à l'origine de son `Ui`, s'en trouve inchangé.
//!
//! ## Le verrou, et le retour à l'ancrage d'origine (2026-09-17, soir)
//!
//! Demande utilisateur : « deux modes qui permettent à l'utilisateur de déplacer l'overlay de
//! récap » — un cadenas fermé et un cadenas ouvert, « les deux ne peuvent pas vivre en même
//! temps », qui disent l'état de la bande et le changent d'un clic. Verrouillée, elle ne se
//! saisit plus et **le curseur redevient celui du système** au-dessus d'elle ; déverrouillée, il
//! repasse à `Grab`/`Grabbing` comme décrit plus haut. Voir [`RecapChrome`] et [`band_drag`].
//!
//! À côté du cadenas, et **seulement une fois la bande déplacée**, un glyphe `Undo` la renvoie à
//! son ancrage d'origine — après confirmation, comme la remise à zéro des compteurs et par la
//! même fenêtre (`OverlayKind::ResetConfirm`, dont la cible dit lequel des deux on remet à zéro).
//! Les deux vivent sur une pastille posée hors du fond, en haut à gauche de la bande, qui bascule
//! en bas quand il n'y a pas la place au-dessus — voir [`paint_actions_row`].
//!
//! ## Ce que ce module ne fait pas
//!
//! Pas d'état : ce bloc affiche une vue et remonte une intention. Il renvoie aussi la **hauteur**
//! qu'il vient d'occuper, pour que l'hôte ajuste sa fenêtre OS à son contenu (même mécanique que
//! `panels::login::LoginOutcome::content_height`) — une fenêtre plus haute que son bloc capterait
//! les clics sur du vide en mode interactif.

use egui::Color32;
use overlay_engine::SessionTotals;

use crate::design::{self, text, tokens, DsIcon, TooltipSide};
use crate::panels::combat::format_fr_thousands;

/// Ce que le bloc affiche — construit par l'hôte depuis `crate::recap_session::RecapSession` à
/// chaque frame (voir la doc de module).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecapView {
    /// Les compteurs de la session (pas ceux du fichier relu).
    pub totals: SessionTotals,
    /// Le chrono de la session.
    pub uptime: std::time::Duration,
    /// Heure locale `HH:MM` du début de la session — l'infobulle de la durée.
    pub started_at: String,
    /// Nombre de reprises de la session — deuxième ligne de la même infobulle, si > 0.
    pub resumed: u32,
}

/// Ce que [`show`] rend à l'hôte.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RecapOutcome {
    /// Hauteur occupée par le bloc, fond compris — la fenêtre OS s'y retaille.
    pub height: f32,
    /// Le glyphe de remise à zéro vient d'être cliqué : à l'hôte d'ouvrir la confirmation.
    pub reset_requested: bool,
    /// Le cadenas de la rangée d'actions vient d'être cliqué : à l'hôte d'inverser
    /// [`RecapChrome::locked`] et de l'écrire dans la config (voir [`paint_actions_row`]).
    pub toggle_lock: bool,
    /// Le glyphe de replacement vient d'être cliqué : à l'hôte d'ouvrir la confirmation qui, sur
    /// un « Oui », rend son ancrage d'origine à la bande.
    pub restore_requested: bool,
    /// Le geste de déplacement de la bande, s'il y en a un cette frame — voir [`RecapDrag`].
    pub drag: RecapDrag,
}

/// **Ce que l'hôte sait de la bande et que le bloc ne peut pas savoir** (2026-09-17) : verrouillée
/// ou non, déjà déplacée ou non, et de quel côté sa rangée d'actions tient.
///
/// Les trois viennent de l'extérieur pour la même raison : ce module ne garde aucun état et ne
/// connaît pas sa position à l'écran (voir la doc de module). Le verrou vit dans la config
/// (`config::OverlayConfig::recap_locked`), le déplacement aussi
/// (`config::OverlayConfig::recap_position`), et le côté se calcule sur la fenêtre de jeu
/// (`recap_placement::actions_below`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecapChrome {
    /// **Bande verrouillée** : elle ne se saisit plus, et le curseur reste celui du système
    /// au-dessus d'elle (voir [`band_drag`]). Le cadenas de la rangée affiche alors
    /// [`DsIcon::Lock`] ; déverrouillée, [`DsIcon::LockOpen`] — jamais les deux, c'est un seul
    /// bouton qui bascule.
    pub locked: bool,
    /// **La bande a une position à elle** (`config::OverlayConfig::recap_position` renseignée) :
    /// c'est la seule condition d'affichage du glyphe de replacement — remettre à son ancrage
    /// d'origine une bande qui y est déjà n'aurait rien à faire.
    pub moved: bool,
    /// **La rangée d'actions passe SOUS le fond** faute de place au-dessus dans la fenêtre de jeu
    /// (voir [`paint_actions_row`] et `recap_placement::actions_below`).
    pub actions_below: bool,
}

impl Default for RecapChrome {
    /// L'état d'une bande jamais touchée : **verrouillée**, à son ancrage d'origine, actions
    /// au-dessus. C'est aussi ce que rend le harnais de captures, qui n'a pas de fenêtre de jeu.
    fn default() -> Self {
        Self {
            locked: true,
            moved: false,
            actions_below: false,
        }
    }
}

/// Le glisser-déposer de la bande, tel que l'hôte le reçoit (2026-09-17, voir la doc de module).
///
/// **Un alias**, et non un type à elle : le panneau Combat se déplace de la même façon depuis le
/// soir du même jour, et les deux gestes sont le MÊME — voir `panels::drag`, qui porte le type, sa
/// doc et la vibration qui l'a façonné. Les hôtes continuent d'écrire `RecapDrag::Started` : c'est
/// bien de la bande qu'ils parlent à cet endroit-là.
pub type RecapDrag = crate::panels::drag::PanelDrag;

/// Côté du glyphe de remise à zéro — plus discret que les cinq glyphes de case (16 px) : c'est
/// une commande, pas une information.
const RESET_ICON_SIZE: f32 = 14.0;

/// Côté d'un glyphe de la **rangée d'actions** (cadenas, replacement) — [`RESET_ICON_SIZE`] :
/// mêmes commandes, même discrétion, et deux tailles de glyphe-commande dans un bloc de 206 px
/// n'auraient rien dit de plus.
const ACTIONS_ICON_SIZE: f32 = RESET_ICON_SIZE;

/// Rembourrage de la pastille de la rangée d'actions, autour de ses glyphes.
///
/// **Une pastille, et pas des glyphes nus** : cette rangée est le seul morceau du bloc peint HORS
/// du fond translucide, donc directement sur l'écran de jeu, dont le fond est arbitraire — un
/// cadenas blanc sur un mur de Bonta clair serait invisible. Elle reprend le fond et le rayon de
/// la bande ([`BACKDROP_ROUNDING`], `tokens::OVERLAY_BACKDROP`) : c'est visiblement la même
/// chose, posée juste à côté.
const ACTIONS_PADDING: f32 = 4.0;

/// Écart entre le cadenas et le glyphe de replacement, dans la pastille.
const ACTIONS_GAP: f32 = 6.0;

/// Écart entre la pastille d'actions et le fond de la bande — assez pour qu'on lise deux blocs,
/// assez peu pour qu'on lise qu'ils vont ensemble.
const ACTIONS_MARGIN: f32 = 2.0;

/// **Hauteur totale de la rangée d'actions**, écart au fond compris : la place que la fenêtre OS
/// doit garder au-dessus ou au-dessous du bloc pour elle (`render_content::RECAP_ACTIONS_RESERVE`,
/// et `recap_placement::Band::actions`).
pub const ACTIONS_HEIGHT: f32 = ACTIONS_ICON_SIZE + 2.0 * ACTIONS_PADDING + ACTIONS_MARGIN;

/// Hauteur d'une ligne du bloc. 22 px : le corps de 15 px des chiffres (voir [`FONT_SIZE`]) plus
/// le cerne d'un pixel de chaque côté, et assez d'interligne pour que deux lignes de glyphes de
/// 16 px ne se touchent pas.
const ROW_HEIGHT: f32 = 22.0;

/// Interligne AJOUTÉ entre deux lignes du bloc — demande utilisateur (2026-09-16, sur capture) :
/// « 5 à 10 pixels entre chacune des lignes, pour aérer ». 8 px, au milieu de la fourchette.
const ROW_GAP: f32 = 8.0;

/// Rembourrage vertical du fond translucide, au-dessus de la première ligne et sous la dernière.
const PADDING_Y: f32 = 6.0;

/// Nombre de lignes du bloc quand tout tient et que tout est affiché — deux lignes de deux cases,
/// plus la durée seule sur la troisième (voir la doc de module). Une ligne de plus par paire de
/// cases empilée, une de moins par ligne dont toutes les cases sont éteintes ([`RecapCells`]).
const MIN_ROWS: usize = 3;

/// Hauteur du bloc à `rows` lignes, fond compris. Jamais moins d'une ligne : Kamas et XP ne
/// s'éteignent pas.
pub const fn height(rows: usize) -> f32 {
    2.0 * PADDING_Y + rows as f32 * ROW_HEIGHT + rows.saturating_sub(1) as f32 * ROW_GAP
}

/// Hauteur du bloc quand tout tient sur trois lignes — la fenêtre OS naît dessus
/// (`main.rs::create_overlay_window`) et se retaille à ce que [`show`] renvoie ensuite.
pub const HEIGHT: f32 = height(MIN_ROWS);

/// **Les cases de la bande qu'on peut éteindre** — les trois cases « Afficher … » de la section
/// « Recap » de la fenêtre Options (voir la doc de module). Kamas et XP n'y sont pas : elles sont
/// toujours affichées.
///
/// Un type à part plutôt que trois `bool` dans `FeatureToggles`, pour la même raison que celui-ci
/// existe : ils voyagent ensemble (config → Options → hôte → rendu), et leur défaut est `true` —
/// un `#[derive(Default)]` les éteindrait tous.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecapCells {
    /// La ligne « Durée de session » (chrono de l'overlay).
    pub duration: bool,
    /// La case « Combats » (gagnés − perdus).
    pub fights: bool,
    /// La case « Challenges » (réussis − échoués).
    pub challenges: bool,
}

impl Default for RecapCells {
    /// **Tout est affiché** — l'état d'une installation neuve, et celui d'un `config.toml` écrit
    /// avant l'existence de ces réglages.
    fn default() -> Self {
        Self {
            duration: true,
            fights: true,
            challenges: true,
        }
    }
}

/// La ligne de la grille à laquelle une case appartient quand tout est affiché — voir
/// [`layout_rows`], qui groupe les cases par ligne avant de décider ce qui tient côte à côte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    /// Kamas et XP — toujours là.
    Money,
    /// Combats et challenges.
    Scores,
    /// La durée, seule.
    Duration,
}

/// Largeur du bloc, fond compris — **fixe**, la fenêtre OS est dimensionnée dessus.
///
/// **Relevé sur capture d'écran annotée (2026-09-16, tard)** : l'utilisateur a tracé deux traits
/// rouges, du bord gauche du bouton Menu au bord droit du bouton Boutique, et demandé que le bloc
/// fasse « toute la largeur des deux droites ». En pixels de la capture, la rangée de boutons va
/// de x = 8 (socle du bouton Menu) à x = 214 (fin de l'ombre du bouton Boutique), soit 206 px ;
/// la zone cliente commençant à x = 4, le bloc démarre 4 px après elle
/// (`main.rs::GAME_RECAP_EDGE_MARGIN_PX`) et s'arrête au même pixel que le dernier bouton.
pub const WIDTH: f32 = 206.0;

/// Rembourrage horizontal du fond translucide, de chaque côté de la grille.
const PADDING_X: f32 = 10.0;

/// Côté du glyphe de chaque cellule. `tokens::SWITCH_ICON_SIZE` (16 px) — la taille à laquelle
/// cette interface peint déjà un glyphe posé à côté d'un libellé court, dans les deux switches du
/// panneau Combat.
const ICON_SIZE: f32 = tokens::SWITCH_ICON_SIZE;

/// Écart entre le glyphe d'une cellule et son chiffre.
const ICON_GAP: f32 = 6.0;

/// Rayon d'angle du fond translucide — celui du carré de contrôle du Suivi
/// (`panels::watchlist::PANEL_BACKDROP_ROUNDING`), la même bande posée sur le même jeu.
const BACKDROP_ROUNDING: f32 = 6.0;

/// Taille du texte des chiffres. `tokens::CHECKBOX_FONT_SIZE` (15 px) — le plus petit corps que
/// cette interface emploie pour du texte à lire, pas pour un badge.
const FONT_SIZE: f32 = tokens::CHECKBOX_FONT_SIZE;

/// Blanc cassé des noms de combattants du panneau Combat — la teinte des CINQ glyphes du bloc
/// (« les icônes doivent être en blanc », 2026-09-16) ; seuls les chiffres sont en couleur.
const TEXT_COLOR: Color32 = Color32::from_rgb(0xE8, 0xED, 0xF2);

/// Couleur du chiffre d'XP et de la durée — l'accent cyan du contenu flottant
/// (`tokens::OVERLAY_ACCENT`), demandé tel quel par l'utilisateur (« de la couleur accent »).
const ACCENT_COLOR: Color32 = tokens::OVERLAY_ACCENT;

/// Couleur du chiffre de kamas — **`#FFD700` demandé explicitement par l'utilisateur
/// (2026-09-16)**, l'or franc, pas le `tokens::TEXT_GOLD` sable (`#F4D89E`) du design system. Ce
/// bloc flotte sur l'écran de jeu, où l'or du client se lit mal ; l'or saturé, lui, dit
/// « monnaie » d'un regard, comme le jaune de la pile de kamas dans l'inventaire du jeu.
const KAMAS_COLOR: Color32 = Color32::from_rgb(0xFF, 0xD7, 0x00);

/// Vert d'un compteur favorable (combats gagnés, challenges réussis) — `--btn-active-green` du
/// thème sombre du web (`styles.css`), pour que les deux applications disent la même chose de la
/// même couleur.
const GAIN_COLOR: Color32 = Color32::from_rgb(0x2E, 0xCC, 0x71);

/// Rouge d'un compteur défavorable — `--btn-active-red`, même origine que [`GAIN_COLOR`].
const LOSS_COLOR: Color32 = Color32::from_rgb(0xE7, 0x4C, 0x3C);

/// Couleur du tiret qui sépare les deux compteurs d'une cellule « gagné − perdu » — volontairement
/// effacée : c'est une ponctuation, pas une troisième valeur.
const DASH_COLOR: Color32 = tokens::TEXT_DISABLED;

/// Un morceau de chiffre et sa couleur — une cellule en porte un (XP, kamas, durée) ou trois
/// (« gagné », tiret, « perdu »).
struct Segment {
    text: String,
    color: Color32,
}

/// Une case du bloc : son glyphe (toujours peint en [`TEXT_COLOR`]), ce qu'elle affiche, et ce
/// que son infobulle en dit.
///
/// **Infobulles de deux ou trois mots, pas une phrase** (retour utilisateur 2026-09-16 tard) : la
/// fenêtre OS du bloc fait 206 px de large ([`WIDTH`]), et une infobulle d'egui ne peut pas en
/// sortir — la première version (« Kamas gagnés moins kamas dépensés depuis le début de la
/// session ») était tronquée à l'écran. Le détail (kamas NETS, durée depuis le lancement de
/// l'overlay) reste documenté dans [`cells`] et la doc de module, pas à l'écran.
struct Cell {
    icon: DsIcon,
    tooltip: String,
    segments: Vec<Segment>,
    /// Sa ligne quand tout est affiché.
    row: Row,
}

/// Les cases **affichées**, dans l'ordre de lecture de la grille (voir [`show`]) : Kamas, XP,
/// Combats, Challenges, Durée — les trois dernières seulement si `visible` les garde. C'est
/// l'ordre de la bande du web (`.recap-bandeau`) à une inversion près, demandée sur capture le
/// 2026-09-16 : les kamas AVANT l'XP.
///
/// **Aucune icône inventée** — les cinq viennent du registre `design::DsIcon`, donc du jeu :
/// `Xp` et `Kamas` sont les glyphes évidents, `Sword` tient pour les combats, `Trophy` pour les
/// challenges et `Clock` pour la durée. Les deux derniers ont été détourés le 2026-09-16 pour ce
/// bloc, qui empruntait jusque-là la dague colorée du switch de grandeur du panneau Combat et la
/// grille de calendrier.
fn cells(view: &RecapView, visible: RecapCells) -> Vec<Cell> {
    let totals = &view.totals;
    let net_kamas = totals.kamas_gained - totals.kamas_lost;
    let mut cells = vec![
        Cell {
            icon: DsIcon::Kamas,
            tooltip: "Kamas gagnés".to_string(),
            // **Pas de symbole « ₭ » derrière le nombre**, contrairement au web. Ubuntu, la police
            // embarquée de cette interface (`design::fonts`), ne couvre pas U+20AD : la première
            // version de cette bande affichait un « ? » à sa place, vu sur la capture du harnais.
            // Le glyphe Kamas à gauche dit déjà de quelle monnaie il s'agit — ajouter une police
            // entière pour un caractère serait payer 100 % du poids pour 0,5 % du signe.
            //
            // Le signe « - » d'un solde négatif vient de `format_fr_thousands` ; un solde positif
            // s'écrit sans « + », à la différence de l'XP qui ne peut que croître.
            segments: vec![Segment {
                text: format_fr_thousands(net_kamas),
                color: KAMAS_COLOR,
            }],
            row: Row::Money,
        },
        Cell {
            icon: DsIcon::Xp,
            tooltip: "XP gagnée".to_string(),
            segments: vec![Segment {
                text: format!("+{}", format_fr_thousands(totals.xp_gained)),
                color: ACCENT_COLOR,
            }],
            row: Row::Money,
        },
    ];
    if visible.fights {
        cells.push(Cell {
            icon: DsIcon::Sword,
            tooltip: "Combats".to_string(),
            segments: win_loss(totals.fights_won, totals.fights_lost),
            row: Row::Scores,
        });
    }
    if visible.challenges {
        cells.push(Cell {
            icon: DsIcon::Trophy,
            tooltip: "Challenges".to_string(),
            segments: win_loss(totals.challenges_passed, totals.challenges_failed),
            row: Row::Scores,
        });
    }
    if visible.duration {
        cells.push(Cell {
            icon: DsIcon::Clock,
            tooltip: duration_tooltip(view),
            segments: vec![Segment {
                text: format_duration(view.uptime),
                color: ACCENT_COLOR,
            }],
            row: Row::Duration,
        });
    }
    cells
}

/// Les trois morceaux d'une cellule « gagné − perdu ».
fn win_loss(won: i64, lost: i64) -> Vec<Segment> {
    vec![
        Segment {
            text: format_fr_thousands(won),
            color: GAIN_COLOR,
        },
        Segment {
            text: " - ".to_string(),
            color: DASH_COLOR,
        },
        Segment {
            text: format_fr_thousands(lost),
            color: LOSS_COLOR,
        },
    ]
}

/// L'infobulle de la durée : « Session depuis HH:MM », et « reprise N fois » sur une seconde
/// ligne dès que la session a repris au moins une fois (décision utilisateur du 2026-09-17). Deux
/// lignes tiennent au-dessus de la case : la durée est sur la dernière ligne du bloc, les deux
/// lignes du dessus (60 px) lui laissent la place — la réserve de l'hôte
/// (`render_content::RECAP_TOOLTIP_RESERVE`) ne sert qu'à la première ligne.
fn duration_tooltip(view: &RecapView) -> String {
    match view.resumed {
        0 => format!("Session depuis {}", view.started_at),
        n => format!("Session depuis {}\nreprise {n} fois", view.started_at),
    }
}

/// `HH:MM:SS`, heures non bornées (un `99:00:00` reste lisible, un `03:00:00` qui repart à zéro
/// mentirait) — même format que le web (`SessionRecapComponent.duration`, `00:00:00` au départ).
pub fn format_duration(uptime: std::time::Duration) -> String {
    let total = uptime.as_secs();
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

/// Peint le bloc et rend la hauteur qu'il occupe, fond compris, et les intentions qu'on vient d'y
/// exprimer — voir la doc de module pour ce que l'hôte en fait. La largeur, elle, est fixe :
/// [`WIDTH`]. `visible` dit quelles cases facultatives peindre (voir [`RecapCells`]), `chrome` ce
/// que l'hôte sait et que le bloc ne peut pas savoir (verrou, bande déplacée, côté de la rangée
/// d'actions — voir [`RecapChrome`]).
///
/// Le contenu est calé en HAUT À GAUCHE de `ui` : la fenêtre OS peut être plus grande que le bloc
/// (elle l'est, entre deux ajustements de hauteur), le vide qui reste est transparent.
pub fn show(
    ui: &mut egui::Ui,
    view: &RecapView,
    visible: RecapCells,
    chrome: RecapChrome,
) -> RecapOutcome {
    // **Le chrono avance tout seul.** Cette architecture ne rend une frame que lorsque quelque
    // chose change (`ControlFlow::Wait`, §6.1 du plan : l'overlay ne consomme rien au repos) — un
    // snapshot du moteur, un survol, un raccourci. La durée, elle, change sans que rien d'autre ne
    // bouge : sans ce rendez-vous, elle resterait figée sur l'écran entre deux lots de log. Une
    // seconde, c'est la granularité de ce qu'elle affiche (`HH:MM:SS`) — rien à gagner à
    // redessiner plus souvent, et l'hôte honore ce délai via `RenderOutcome`/`next_redraw_at`.
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs(1));

    let cells = cells(view, visible);
    let font = text::label_font(ui.ctx(), FONT_SIZE);
    let ds = design::DesignSystem::get(ui.ctx());

    // Mise en page d'abord, peinture ensuite : la hauteur du fond dépend de ce qui tient sur une
    // ligne, et un fond peint après les cellules passerait par-dessus elles.
    let galleys: Vec<Vec<std::sync::Arc<egui::Galley>>> = cells
        .iter()
        .map(|cell| {
            cell.segments
                .iter()
                .map(|segment| {
                    // `Color32::PLACEHOLDER` : la couleur est donnée au moment de peindre, sinon
                    // les neuf passes du cerne sortiraient toutes de la couleur figée ici — voir
                    // `design::text::paint_outlined_galley`.
                    ui.fonts_mut(|f| {
                        f.layout_no_wrap(segment.text.clone(), font.clone(), Color32::PLACEHOLDER)
                    })
                })
                .collect()
        })
        .collect();
    // **Largeurs ARRONDIES au pixel.** La largeur d'un texte est fractionnaire ; en la propageant
    // telle quelle, chaque glyphe de la colonne de droite atterrirait à une abscisse fractionnaire,
    // donc étalé sur deux colonnes de pixels par anti-aliasing. Sur un overlay posé par-dessus un
    // jeu, un trait net vaut mieux qu'un trait juste à un demi-pixel près — et c'est aussi ce qui
    // rend la capture de non-régression reproductible d'une machine de rendu à l'autre.
    let cell_widths: Vec<f32> = galleys
        .iter()
        .map(|segments| {
            (ICON_SIZE + ICON_GAP + segments.iter().map(|g| g.size().x).sum::<f32>()).ceil()
        })
        .collect();

    let content_width = WIDTH - 2.0 * PADDING_X;
    let half_width = content_width / 2.0;
    let rows_layout = layout_rows(
        &cells.iter().map(|cell| cell.row).collect::<Vec<_>>(),
        &cell_widths,
        half_width,
    );
    let rows = rows_layout.len();

    let origin = ui.max_rect().min;
    let band = egui::Rect::from_min_size(origin, egui::vec2(WIDTH, height(rows)));
    ui.painter()
        .rect_filled(band, BACKDROP_ROUNDING, tokens::OVERLAY_BACKDROP);
    // La préhension AVANT les cases et le glyphe de remise à zéro : dans une même couche, egui
    // donne le pointeur au dernier widget déclaré, donc à celui qui est peint par-dessus. Le fond
    // est le plus bas, et c'est bien ce qu'on veut — tout ce qui est posé dessus lui reprend le
    // geste (voir `paint_reset_button`).
    let drag = band_drag(ui, band, chrome.locked);

    let content_left = band.min.x + PADDING_X;
    let row_top = |row: usize| band.min.y + PADDING_Y + row as f32 * (ROW_HEIGHT + ROW_GAP);
    // Une case centrée dans une tranche de la ligne : sa moitié quand elle en partage la ligne
    // avec une autre, toute la largeur quand elle est seule (empilée, ou la durée).
    let centered = |slot_left: f32, slot_width: f32, cell: usize| {
        content_left + slot_left + ((slot_width - cell_widths[cell]) / 2.0).round()
    };
    let place_row = |cells: &[usize], width: f32, y: f32| -> Vec<(usize, f32, f32)> {
        match *cells {
            [left, right] => vec![
                (left, centered(0.0, width / 2.0, left), y),
                (right, centered(width / 2.0, width / 2.0, right), y),
            ],
            [alone] => vec![(alone, centered(0.0, width, alone), y)],
            _ => unreachable!("une ligne porte une ou deux cases"),
        }
    };
    let mut placements: Vec<(usize, f32, f32)> = rows_layout
        .iter()
        .enumerate()
        .flat_map(|(row, cells)| place_row(cells, content_width, row_top(row)))
        .collect();

    // Le glyphe de remise à zéro, au bout de la dernière ligne (voir la doc de module). Si une
    // case de cette ligne le toucherait, la ligne entière se range dans la largeur qui reste à sa
    // gauche — le cas courant (la durée seule, centrée) garde ses 30 px de marge et ne bouge pas.
    let reset_rect = egui::Rect::from_min_size(
        egui::pos2(band.max.x - PADDING_X - ICON_SIZE, row_top(rows - 1)),
        egui::vec2(ICON_SIZE, ROW_HEIGHT),
    );
    let last_row = &rows_layout[rows - 1];
    let touches_reset = placements
        .iter()
        .filter(|(cell, _, _)| last_row.contains(cell))
        .any(|(cell, x, _)| x + cell_widths[*cell] + ICON_GAP > reset_rect.min.x);
    if touches_reset {
        placements.retain(|(cell, _, _)| !last_row.contains(cell));
        placements.extend(place_row(
            last_row,
            content_width - ICON_SIZE - ICON_GAP,
            row_top(rows - 1),
        ));
    }

    for (index, x, y) in placements {
        let cell_rect =
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell_widths[index], ROW_HEIGHT));
        paint_cell(ui, &ds, cell_rect, &cells[index], &galleys[index]);
    }
    let reset_requested = paint_reset_button(ui, &ds, reset_rect);
    let actions = paint_actions_row(ui, &ds, band, chrome);

    RecapOutcome {
        height: band.height(),
        reset_requested,
        toggle_lock: actions.toggle_lock,
        restore_requested: actions.restore_requested,
        drag,
    }
}

/// La bande saisie à la souris (voir la doc de module) : `Sense::drag()` sur tout le fond, le
/// curseur qui dit que ça s'attrape, et le geste tel que l'hôte l'attend ([`RecapDrag`]) — **sauf
/// verrouillée** ([`RecapChrome::locked`]), où rien de tout cela n'existe.
///
/// `Sense::drag()` seul, sans le clic : la bande n'a rien à faire d'un clic sur son fond, et un
/// `click_and_drag` imposerait à egui un seuil de quelques pixels avant de commencer — la bande
/// resterait collée le temps de le franchir, puis sauterait.
fn band_drag(ui: &mut egui::Ui, band: egui::Rect, locked: bool) -> RecapDrag {
    // **Verrouillée, la bande n'est plus un widget du tout** (2026-09-17, demande utilisateur) :
    // pas de zone d'interaction, donc pas de curseur « main » — « la souris repasse en mode
    // normal » — et pas le moindre geste à remonter. Le rendre en ne renvoyant rien tout en
    // gardant le `interact` aurait laissé le curseur changer au survol, c'est-à-dire promis un
    // déplacement qui n'arrive pas.
    if locked {
        return RecapDrag::None;
    }
    // `interact_pointer_pos` côté `panels::drag` (et non `pointer_latest_pos`) : la position que le
    // pointeur a POUR CE widget, celle qui reste définie tant que le bouton n'est pas relâché même
    // si le curseur sort du bloc — ce qui arrive à chaque frame où la fenêtre n'a pas encore
    // rattrapé la souris. Elle n'est lue qu'au PREMIER appui, pour le point de saisie : la suite du
    // geste se règle sur le curseur d'écran, hors de ce repère (voir [`RecapDrag`]).
    let response = ui.interact(band, ui.id().with("recap-fond"), egui::Sense::drag());
    crate::panels::drag::from_response(ui, &response)
}

/// Ce que la rangée d'actions vient de récolter — deux boutons, deux intentions, remontées
/// telles quelles par [`RecapOutcome`] : ce module n'agit jamais lui-même.
#[derive(Debug, Clone, Copy, Default)]
struct ActionsOutcome {
    toggle_lock: bool,
    restore_requested: bool,
}

/// **La rangée d'actions de la bande** (2026-09-17, demande utilisateur) : le cadenas, et le
/// glyphe de replacement quand il a lieu d'être — posés HORS du fond, sur une pastille à eux,
/// alignés sur le bord gauche de la bande.
///
/// ## Un seul cadenas, deux visages
///
/// « Les deux ne peuvent pas vivre en même temps » : ce n'est pas un couple de boutons mais un
/// seul, qui porte l'état de la bande. [`DsIcon::Lock`] quand elle est verrouillée (« clique pour
/// libérer »), [`DsIcon::LockOpen`] quand elle ne l'est pas (« clique pour figer ») — les deux
/// glyphes sont faits l'un pour l'autre, même corps au pixel près, seule l'anse change (voir leur
/// doc dans `design::icons`).
///
/// ## Le glyphe de replacement n'apparaît qu'une fois la bande déplacée
///
/// [`RecapChrome::moved`], et rien d'autre : tant que la bande est à son ancrage d'origine, il n'y
/// a rien à défaire, et un bouton grisé en permanence sur un overlay de 206 px coûterait sa place
/// pour ne rien dire. La pastille se resserre sur le seul cadenas dans ce cas.
///
/// ## Au-dessus, sauf quand ça ne tient pas
///
/// La rangée se pose au-dessus de la bande — c'est là que l'utilisateur l'a demandée, « en haut à
/// gauche » — et **bascule en dessous** quand la bande est posée si haut dans la fenêtre de jeu
/// que la rangée en sortirait ([`RecapChrome::actions_below`], calculé par l'hôte dans
/// `recap_placement::actions_below`). La fenêtre OS garde la place des deux côtés
/// (`render_content::RECAP_ACTIONS_RESERVE`), de sorte que ce choix ne retaille jamais rien : une
/// taille qui dépendrait du côté, et un côté de la position, se rebouclerait — la bande a déjà
/// payé une vibration pour une boucle de ce genre (voir `recap_placement::drag_offset`).
///
/// Les infobulles s'ouvrent **du côté de la bande** (au-dessus d'une rangée basse, en dessous
/// d'une rangée haute) : de l'autre côté, elles sortiraient de la fenêtre OS, qui ne réserve que
/// la hauteur de la rangée elle-même.
fn paint_actions_row(
    ui: &mut egui::Ui,
    ds: &design::DesignSystem,
    band: egui::Rect,
    chrome: RecapChrome,
) -> ActionsOutcome {
    let glyphs = 1 + usize::from(chrome.moved);
    let pill_size = egui::vec2(
        2.0 * ACTIONS_PADDING
            + glyphs as f32 * ACTIONS_ICON_SIZE
            + (glyphs - 1) as f32 * ACTIONS_GAP,
        ACTIONS_ICON_SIZE + 2.0 * ACTIONS_PADDING,
    );
    let top = if chrome.actions_below {
        band.max.y + ACTIONS_MARGIN
    } else {
        band.min.y - ACTIONS_MARGIN - pill_size.y
    };
    let pill = egui::Rect::from_min_size(egui::pos2(band.min.x, top), pill_size);
    ui.painter()
        .rect_filled(pill, BACKDROP_ROUNDING, tokens::OVERLAY_BACKDROP);

    let side = if chrome.actions_below {
        TooltipSide::Above
    } else {
        TooltipSide::Below
    };
    let slot = |index: usize| {
        egui::Rect::from_min_size(
            egui::pos2(
                pill.min.x + ACTIONS_PADDING + index as f32 * (ACTIONS_ICON_SIZE + ACTIONS_GAP),
                pill.min.y + ACTIONS_PADDING,
            ),
            egui::Vec2::splat(ACTIONS_ICON_SIZE),
        )
    };

    let (icon, tooltip) = if chrome.locked {
        (DsIcon::Lock, "Déverrouiller la bande")
    } else {
        (DsIcon::LockOpen, "Verrouiller la bande")
    };
    let toggle_lock = paint_action(ui, ds, slot(0), "recap-verrou", icon, tooltip, side);
    // Court-circuit volontaire : bande jamais déplacée, glyphe jamais peint — la pastille s'est
    // déjà dimensionnée dessus.
    let restore_requested = chrome.moved
        && paint_action(
            ui,
            ds,
            slot(1),
            "recap-replacer",
            DsIcon::Undo,
            "Replacer la bande",
            side,
        );

    ActionsOutcome {
        toggle_lock,
        restore_requested,
    }
}

/// Un glyphe-commande de la rangée d'actions : blanc au repos, or au survol, curseur « main »,
/// infobulle du côté demandé. Rend `true` la frame où il est cliqué.
///
/// **`click_and_drag` comme le glyphe de remise à zéro** et pour la même raison (voir
/// [`paint_reset_button`]) : un bouton qui ne sentirait que le clic laisserait le glissement au
/// fond de la bande, et un appui sur le cadenas ferait partir celle-ci.
#[allow(clippy::too_many_arguments)]
fn paint_action(
    ui: &mut egui::Ui,
    ds: &design::DesignSystem,
    rect: egui::Rect,
    id: &'static str,
    icon: DsIcon,
    tooltip: &'static str,
    side: TooltipSide,
) -> bool {
    let response = ui.interact(rect, ui.id().with(id), egui::Sense::click_and_drag());
    let tint = if response.hovered() {
        tokens::ICON_TINT_HOVER
    } else {
        TEXT_COLOR
    };
    let icon_rect = egui::Rect::from_center_size(
        rect.center(),
        fit(ds.icon_native_size(icon), ACTIONS_ICON_SIZE),
    );
    ds.paint_icon(ui.painter(), icon_rect, icon, tint);
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    design::tooltip(&response).side(side).text(tooltip);
    response.clicked()
}

/// Le glyphe de remise à zéro (voir la doc de module) : `Undo`, blanc au repos, or au survol,
/// curseur « main », infobulle au-dessus. Rend `true` la frame où il est cliqué.
///
/// Peint à la main comme les cinq glyphes de case, et non par `design::icon_button` : les quatre
/// contextes de ce composant posent un socle (texture du jeu ou voile), et un socle de 32 px sur
/// une ligne de 22 serait le seul bouton « habillé » d'un bloc où tout le reste est nu.
///
/// **`click_and_drag` et non `click`** (2026-09-17) : depuis que le fond de la bande se saisit à
/// la souris, un glyphe qui ne sentirait que le clic laisserait le fond — pourtant sous lui —
/// recevoir le glissement, et la bande partirait à chaque appui sur ce bouton. En sentant le
/// glissement lui aussi, il le capte et n'en fait rien ; le clic, lui, reste un clic tant que la
/// souris ne bouge pas, et une remise à zéro amorcée puis glissée est annulée — ce qui est la
/// bonne réponse pour un geste destructeur.
fn paint_reset_button(ui: &mut egui::Ui, ds: &design::DesignSystem, rect: egui::Rect) -> bool {
    let response = ui.interact(
        rect,
        ui.id().with("recap-reset"),
        egui::Sense::click_and_drag(),
    );
    let tint = if response.hovered() {
        tokens::ICON_TINT_HOVER
    } else {
        TEXT_COLOR
    };
    let icon_rect = egui::Rect::from_center_size(
        rect.center(),
        fit(ds.icon_native_size(DsIcon::Undo), RESET_ICON_SIZE),
    );
    ds.paint_icon(ui.painter(), icon_rect, DsIcon::Undo, tint);
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    design::tooltip(&response)
        .side(TooltipSide::Above)
        .text("Remettre à zéro");
    response.clicked()
}

/// Les lignes du bloc, de haut en bas : les indices (dans [`cells`]) des cases qu'elles portent,
/// deux quand une paire tient côte à côte, une quand elle s'empile — ou quand la ligne n'a qu'une
/// case, la durée toujours, Combats ou Challenges quand l'autre est éteinte ([`RecapCells`]).
///
/// `rows[i]` est la ligne « nominale » de la case `i`, dans l'ordre de [`cells`] (les cases d'une
/// même ligne se suivent). Décide, ligne par ligne, si deux cases tiennent chacune dans sa moitié
/// (`half_width`) ou si elles s'empilent — la règle « responsive » demandée le 2026-09-16 : un
/// chiffre trop long pour sa moitié fait passer la paire l'un au-dessus de l'autre plutôt que de
/// chevaucher son voisin. Les lignes se décident indépendamment : chaque case a sa moitié à elle,
/// l'une n'empiète jamais sur l'autre. Une ligne sans case n'existe pas — le bloc se resserre.
fn layout_rows(rows: &[Row], cell_widths: &[f32], half_width: f32) -> Vec<Vec<usize>> {
    let mut layout: Vec<Vec<usize>> = Vec::new();
    let mut index = 0;
    while index < rows.len() {
        let row = rows[index];
        let end = index + rows[index..].iter().take_while(|r| **r == row).count();
        let members: Vec<usize> = (index..end).collect();
        match members[..] {
            [left, right] => {
                if cell_widths[left] > half_width || cell_widths[right] > half_width {
                    layout.push(vec![left]);
                    layout.push(vec![right]);
                } else {
                    layout.push(vec![left, right]);
                }
            }
            _ => layout.extend(members.iter().map(|&cell| vec![cell])),
        }
        index = end;
    }
    layout
}

/// Glyphe puis chiffres d'une cellule, centrés verticalement dans `rect`, et l'infobulle qui dit
/// de quoi il s'agit.
///
/// **Infobulle AU-DESSUS** (`TooltipSide::Above`, le défaut du design system) pour les cinq
/// cases. Elles se sont ouvertes EN DESSOUS pendant une journée (2026-09-16 : le bloc est posé
/// sous les boutons du jeu, une infobulle au-dessus de la première ligne recouvre donc la zone où
/// le jeu ouvre les siennes) — mais avec une fenêtre OS qui ne laissait sous le bloc que 28 px, la
/// durée, dernière ligne, retombait déjà sur le repli « au-dessus », et le retour a tranché : « je
/// voudrais toutes les infobulles au-dessus des éléments, à l'image de la durée ». La place
/// nécessaire au-dessus de la première ligne est réservée par l'hôte
/// (`render_content::RECAP_TOOLTIP_RESERVE`, marge haute du contenu) ; une infobulle d'une case
/// de la deuxième ligne recouvre la première, comme celle de la durée recouvre la deuxième — c'est
/// le comportement demandé, pas un repli.
fn paint_cell(
    ui: &mut egui::Ui,
    ds: &design::DesignSystem,
    rect: egui::Rect,
    cell: &Cell,
    galleys: &[std::sync::Arc<egui::Galley>],
) {
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.min.x + ICON_SIZE / 2.0, rect.center().y),
        fit(ds.icon_native_size(cell.icon), ICON_SIZE),
    );
    // Les cinq glyphes sont blancs dans le fichier (catégorie `libre` du registre) et le restent
    // à l'écran — seul le chiffre porte une couleur (voir la doc de module).
    ds.paint_icon(ui.painter(), icon_rect, cell.icon, TEXT_COLOR);

    let mut x = rect.min.x + ICON_SIZE + ICON_GAP;
    for (segment, galley) in cell.segments.iter().zip(galleys) {
        let pos = egui::pos2(x, rect.center().y - galley.size().y / 2.0);
        // Cerne COMPLET : ce bloc flotte sur l'écran de jeu, dont le fond est arbitraire — voir
        // `design::text::OUTLINE_FULL`.
        text::paint_outlined_galley(
            ui.painter(),
            pos,
            galley,
            segment.color,
            Color32::BLACK,
            text::OUTLINE_FULL,
        );
        x += galley.size().x;
    }

    let response = ui.interact(
        rect,
        ui.id().with(("recap-cell", cell.icon)),
        egui::Sense::hover(),
    );
    design::tooltip(&response)
        .side(TooltipSide::Above)
        .text(cell.tooltip.clone());
}

/// Rectangle de `side` px de côté au plus, au rapport natif du glyphe — le pendant local de
/// `design::components::icon_button::glyph_fit`, qui n'est pas exposé hors du design system.
fn fit(native: egui::Vec2, side: f32) -> egui::Vec2 {
    if native.x <= 0.0 || native.y <= 0.0 {
        return egui::vec2(side, side);
    }
    let scale = (side / native.x).min(side / native.y);
    native * scale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_duree_est_en_heures_minutes_secondes() {
        assert_eq!(format_duration(std::time::Duration::ZERO), "00:00:00");
        assert_eq!(
            format_duration(std::time::Duration::from_secs(59)),
            "00:00:59"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_secs(60)),
            "00:01:00"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_secs(3661)),
            "01:01:01"
        );
    }

    /// Les heures ne repartent JAMAIS à zéro : une session de plus d'un jour reste lisible telle
    /// quelle plutôt que d'annoncer une durée fausse (voir la doc de `format_duration`).
    #[test]
    fn les_heures_ne_bouclent_pas_a_vingt_quatre() {
        assert_eq!(
            format_duration(std::time::Duration::from_secs(25 * 3600)),
            "25:00:00"
        );
    }

    /// Cinq cases, toujours — c'est le contrat de la demande (kamas, XP, combats, challenges,
    /// durée), dans l'ordre où la grille les lit ligne par ligne : les kamas d'abord, puis l'XP.
    #[test]
    fn le_bloc_porte_cinq_cases_kamas_en_tete() {
        let view = RecapView {
            totals: SessionTotals {
                kamas_gained: 1_500,
                kamas_lost: 500,
                xp_gained: 42_000,
                loot_count: 3,
                fights_won: 7,
                fights_lost: 2,
                challenges_passed: 4,
                challenges_failed: 1,
            },
            uptime: std::time::Duration::from_secs(3600),
            started_at: "20:12".to_string(),
            resumed: 0,
        };
        let cells = cells(&view, RecapCells::default());
        let icons: Vec<DsIcon> = cells.iter().map(|c| c.icon).collect();
        assert_eq!(
            icons,
            vec![
                DsIcon::Kamas,
                DsIcon::Xp,
                DsIcon::Sword,
                DsIcon::Trophy,
                DsIcon::Clock
            ]
        );
        // Les kamas sont NETS, jamais le seul gagné (le web affiche `stats.netKamas()`).
        // Les kamas s'écrivent sans symbole de monnaie — voir `cells`.
        assert_eq!(cells[0].segments[0].text, "1 000");
        assert_eq!(cells[1].segments[0].text, "+42 000");
        assert_eq!(cells[2].segments[0].text, "7");
        assert_eq!(cells[2].segments[2].text, "2");
        assert_eq!(cells[3].segments[0].text, "4");
        assert_eq!(cells[3].segments[2].text, "1");
        assert_eq!(cells[4].segments[0].text, "01:00:00");
        assert_eq!(cells[4].tooltip, "Session depuis 20:12");
    }

    /// L'infobulle de la durée gagne une seconde ligne dès la première reprise — et jamais
    /// avant : « reprise 0 fois » ne dirait rien.
    #[test]
    fn l_infobulle_de_la_duree_compte_les_reprises() {
        let mut view = RecapView {
            started_at: "09:30".to_string(),
            ..Default::default()
        };
        assert_eq!(duration_tooltip(&view), "Session depuis 09:30");
        view.resumed = 1;
        assert_eq!(
            duration_tooltip(&view),
            "Session depuis 09:30\nreprise 1 fois"
        );
        view.resumed = 3;
        assert_eq!(
            duration_tooltip(&view),
            "Session depuis 09:30\nreprise 3 fois"
        );
    }

    /// Seul le chiffre est en couleur : or `#FFD700` pour les kamas (demande utilisateur), accent
    /// pour l'XP et la durée, vert/rouge pour les compteurs — et un solde négatif garde son signe.
    #[test]
    fn les_couleurs_suivent_la_maquette() {
        let view = RecapView {
            totals: SessionTotals {
                kamas_gained: 100,
                kamas_lost: 350,
                ..Default::default()
            },
            ..Default::default()
        };
        let cells = cells(&view, RecapCells::default());
        assert_eq!(
            cells[0].segments[0].color,
            Color32::from_rgb(0xFF, 0xD7, 0x00)
        );
        assert_eq!(cells[0].segments[0].text, "-250");
        assert_eq!(cells[1].segments[0].color, ACCENT_COLOR);
        assert_eq!(cells[2].segments[0].color, GAIN_COLOR);
        assert_eq!(cells[2].segments[2].color, LOSS_COLOR);
        assert_eq!(cells[4].segments[0].color, ACCENT_COLOR);
    }

    /// Les cinq lignes nominales quand tout est affiché.
    const ALL: [Row; 5] = [
        Row::Money,
        Row::Money,
        Row::Scores,
        Row::Scores,
        Row::Duration,
    ];

    /// Tout tient : deux lignes de deux cases, chacune dans sa moitié de 93 px, et la durée seule
    /// sur la troisième.
    #[test]
    fn deux_paires_qui_tiennent_restent_sur_deux_lignes() {
        let rows = layout_rows(&ALL, &[40.0, 93.0, 50.0, 45.0, 70.0], 93.0);
        assert_eq!(rows, vec![vec![0, 1], vec![2, 3], vec![4]]);
        assert_eq!(height(rows.len()), HEIGHT);
    }

    /// Une case qui déborde de sa moitié empile SA paire (la case de gauche au-dessus), l'autre
    /// paire reste côte à côte — et le bloc gagne une ligne.
    #[test]
    fn une_case_trop_large_empile_sa_paire_seule() {
        let rows = layout_rows(&ALL, &[40.0, 94.0, 50.0, 45.0, 70.0], 93.0);
        assert_eq!(rows, vec![vec![0], vec![1], vec![2, 3], vec![4]]);
        assert_eq!(height(rows.len()), height(4));
    }

    /// Les deux paires empilées : cinq lignes, une par case.
    #[test]
    fn les_deux_paires_empilees_font_cinq_lignes() {
        let rows = layout_rows(&ALL, &[120.0, 110.0, 100.0, 45.0, 70.0], 93.0);
        assert_eq!(rows, vec![vec![0], vec![1], vec![2], vec![3], vec![4]]);
        assert_eq!(height(5), 2.0 * 6.0 + 5.0 * 22.0 + 4.0 * 8.0);
    }

    /// Les trois cases facultatives s'éteignent une à une : la case restante d'une paire prend
    /// la ligne pour elle seule, une ligne vide disparaît, Kamas et XP restent.
    #[test]
    fn les_cases_eteintes_resserrent_la_grille() {
        let view = RecapView::default();
        let icons = |visible: RecapCells| {
            cells(&view, visible)
                .iter()
                .map(|c| (c.icon, c.row))
                .collect::<Vec<_>>()
        };
        // Sans combats : Challenges seule sur sa ligne.
        let sans_combats = icons(RecapCells {
            fights: false,
            ..RecapCells::default()
        });
        assert_eq!(
            sans_combats,
            vec![
                (DsIcon::Kamas, Row::Money),
                (DsIcon::Xp, Row::Money),
                (DsIcon::Trophy, Row::Scores),
                (DsIcon::Clock, Row::Duration),
            ]
        );
        let rows: Vec<Row> = sans_combats.iter().map(|(_, row)| *row).collect();
        assert_eq!(
            layout_rows(&rows, &[40.0, 50.0, 45.0, 70.0], 93.0),
            vec![vec![0, 1], vec![2], vec![3]]
        );
        // Sans durée ni combats ni challenges : la seule ligne Kamas / XP.
        let rien = icons(RecapCells {
            duration: false,
            fights: false,
            challenges: false,
        });
        assert_eq!(rien.len(), 2);
        let rows: Vec<Row> = rien.iter().map(|(_, row)| *row).collect();
        assert_eq!(layout_rows(&rows, &[40.0, 50.0], 93.0), vec![vec![0, 1]]);
        assert_eq!(height(1), 2.0 * 6.0 + 22.0);
    }

    /// Le défaut affiche tout : c'est l'état d'une config écrite avant ces réglages.
    #[test]
    fn par_defaut_tout_est_affiche() {
        assert_eq!(
            RecapCells::default(),
            RecapCells {
                duration: true,
                fights: true,
                challenges: true,
            }
        );
    }
}
