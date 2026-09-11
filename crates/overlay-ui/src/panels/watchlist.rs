//! Panneau "Suivi" (watchlist, §9 du plan) — bande horizontale de tuiles carrées, à l'image du
//! bandeau du dépôt web (`tracker-strip.component`/`.kpi`) : demande utilisateur explicite
//! 2026-09-01/02 (captures d'écran de référence à l'appui), qui remplace la première version en
//! liste verticale de lignes (retours utilisateur précédents, gardée en mémoire dans l'historique
//! Git mais plus dans ce fichier). Rendu dans sa PROPRE fenêtre overlay (`OverlayKind::Watchlist`,
//! `main.rs`), décorrélée de la fenêtre Combat — demande utilisateur explicite : les deux zones
//! doivent pouvoir, à terme, être pilotées indépendamment en visibilité.
//!
//! Toujours en LECTURE SEULE (voir `overlay_engine::watchlist` pour la frontière
//! définitions/compteurs) : les deux boutons "+"/"−" (voir `control_button`) reprennent l'intention
//! du bandeau web (ajouter un suivi / sélection multiple + suppression) mais restent INERTES ici —
//! aucun formulaire d'ajout, aucune sélection ne sont câblés côté overlay pour cette itération
//! (demande utilisateur : "je pense qu'on le fera plus tard quand tu auras tout câblé").
//! Un survol affiche un tooltip explicite plutôt que de laisser un bouton cliquable qui ne ferait
//! rien silencieusement (retour utilisateur déjà vécu sur le bouton de connexion au compte).
//!
//! Icône réelle de chaque tuile (retour utilisateur 2026-09-02 : « comme les images de
//! ressources/monstres n'est pas présent c'est très compliqué pour l'utilisateur » de distinguer
//! les tuiles entre elles) — résolue via `overlay_engine::CatalogIndex` (lot L3 réduit, voir
//! `catalog.rs`) puis téléchargée/décodée par `remote_icons::RemoteIconStore`, mise en cache par
//! fenêtre dans `remote_icons::RemoteIconTextures`. Repli sur l'icône générique (`UiIcons::
//! unknown_entity_texture`) tant que l'entrée n'est pas résolue par le catalogue OU que son icône
//! n'a pas fini de télécharger — jamais une tuile vide ou un blocage du rendu.
//!
//! **Refonte visuelle 2026-09-02** (retour utilisateur, capture d'écran du bandeau web à l'appui,
//! comparée au rendu overlay) — trois écarts corrigés pour rester RACCORD avec le web :
//! - Bordure/fond selon la RARETÉ de l'objet (`--rarity-color`, `styles.css` du dépôt web, voir
//!   `RARITY_COLORS`) plutôt qu'un simple bleu/orange objet/ennemi — miroir de `.kpi[class*=
//!   'rarity-']` (`tracker-strip.component.css`) : bordure pleine de la couleur de rareté, fond en
//!   dégradé diagonal de `mix(rarity 35%, panel-bg)` vers `panel-bg`. Un ennemi n'a pas de rareté
//!   (`.kpi.is-monster`) : bordure grise unie (`--text-muted`), fond plat, jamais de dégradé.
//! - Badge de compteur déplacé à L'EXTÉRIEUR du coin bas-droit (miroir de `.kpi-count-badge`,
//!   `bottom:-7px; right:-7px` — la tuile elle-même n'a plus AUCUN élément qui la recouvre),
//!   au lieu d'être peint PAR-DESSUS l'icône comme dans la version précédente.
//! - Couleurs du texte du badge : mode `up` en clair neutre, mode `down` avec la valeur COURANTE
//!   en couleur kamas (`--kama-color`, or) et la cible en gris (`--text-muted`) — miroir de
//!   `.kpi-count-badge.is-fraction`/`.kpi-count-badge-target`.
//!
//! **Refonte 2026-09-06** (retour utilisateur explicite, images de référence à l'appui) — cette
//! fois pour coller à l'apparence du JEU plutôt qu'à celle du dépôt web :
//! - Les deux tuiles "+"/"−" (bordure pointillée + glyphe ASCII dessinés à la main) sont
//!   remplacées par deux VRAIES icônes du jeu (`UiIcons::watchlist_add`/`watchlist_remove`, voir sa
//!   doc) empilées verticalement (+ au-dessus de −) plutôt que côte à côte — gain de place
//!   explicitement demandé (34px de large empilés contre 2×58px+écart côte à côte auparavant) ET
//!   réutilisation d'icônes que l'utilisateur reconnaît déjà dans l'interface du jeu, plutôt que
//!   des glyphes maison. Éclaircissement au survol précalculé (`ui_icons::brighten`, pas de tint
//!   dynamique, voir sa doc). Infobulle à GAUCHE désormais (`design::tooltip`, côté `Left`), pas au-dessus
//!   comme le reste du panneau — demande explicite pour CES deux boutons précisément.
//! - Les tuiles OBJET (`WatchlistKind::Item`) n'utilisent plus le dégradé diagonal dessiné à la
//!   main du point précédent : la texture `Border-<RARETÉ>.webp` correspondante (`UiIcons::
//!   item_border`, voir sa doc et `docs/design-system.md` §2.4/§7), exactement l'asset
//!   d'emplacement d'objet du jeu, sert de fond de la tuile — plus fidèle qu'une approximation de
//!   dégradé mesurée au pixel. Géométrie mesurée une fois par script Python/Pillow sur les 7
//!   fichiers (identique sur les 7) : fenêtre intérieure = pixels 52..460 d'un canevas 512×512,
//!   voir `ITEM_BORDER_INNER_MARGIN_RATIO`. Les tuiles ENNEMI (`WatchlistKind::Enemy`) gardent le
//!   style précédent (fond plat, bordure grise unie) — demande explicite : « pour les monstres, on
//!   verra ultérieurement comment on fait ».
//!
//! **Correctif same-day** (retour utilisateur, captures d'écran jeu/overlay à l'appui) : la
//! première version de ce point peignait la texture de bordure PAR-DESSUS l'icône. Or la fenêtre
//! intérieure de ces textures (52..460, ci-dessus) n'est PAS un trou transparent — un script
//! Python/Pillow sur les octets décodés le confirme : c'est un aplat semi-transparent (~70 %
//! d'opacité) teinté par la rareté (ex. `Border-LEGENDARY.webp` → `(139,149,0)`, un olive/jaune).
//! Peinte après l'icône, cette texture recouvrait donc l'icône ENTIÈRE d'un voile coloré — d'où le
//! rendu « en opacité » constaté (« j'ai l'impression que tu as mis les objets en opacité »),
//! flagrant sur les raretés à teinte franche (jaune/olive), plus discret sur une rareté déjà proche
//! en teinte de l'icône (ex. orangé). Le fond est maintenant peint EN PREMIER (comme un dégradé de
//! fond classique), l'icône ENSUITE par-dessus, opaque : elle recouvre l'essentiel de cet aplat, ne
//! laissant dépasser que l'anneau de rareté et un mince liseré — exactement le rendu du jeu. Icône
//! aussi agrandie (`ITEM_ICON_FILL_RATIO` 0.9 → 0.96, second retour du même message : « les objets
//! doivent être plus gros ») : rien n'empêche plus de s'approcher du bord de la fenêtre intérieure
//! maintenant que la bordure ne risque plus de la recouvrir.
//! - Le badge de compteur en pilule (débordant hors du coin bas-droit) est retiré : les captures de
//!   référence du jeu montrent un simple nombre en texte cerné de noir (même procédé que
//!   `design::text::paint_outlined_text`, réutilisé ici), incrusté DANS le coin bas-droit de la tuile —
//!   voir `paint_count_inline`. Plus aucun débordement hors tuile : `content_width` n'a donc plus
//!   besoin de réserver `BADGE_OVERFLOW` (retiré).
//!
//! **Refonte 2026-09-06 (design system boutons icône)** — retour utilisateur explicite (image de
//! référence à l'appui, `menu-button-icon-first-plan.png`), les boutons "+"/"−" doivent maintenant
//! utiliser EXACTEMENT le même système que les boutons Combat (lien externe, Options) : voir doc de
//! `ui_icons` et `panels::icon_button`. Deux changements :
//! - Les icônes "+"/"−" déjà intégrées à un fond (`watchlist-add.png`/`watchlist-remove.png`, retour
//!   précédent ci-dessus) sont remplacées par des glyphes SEULS à fond transparent
//!   (`UiIcons::icon_plus`/`icon_minus`), composés avec le MÊME socle `button_background`/
//!   `button_background_hover` que Combat (`control_button`, désormais un fin appel à
//!   `icon_button::paint_icon_button`) — plus une variante "survolée" approximée par éclaircissement
//!   (`brighten`, retiré), mais la même paire de teintes exactes `#c5cbcc`/`#f4d89f` partout.
//! - Empilement VERTICAL conservé (`control_button_row`) — une première passe l'avait remplacé par
//!   une disposition horizontale (mauvaise lecture de « pas en vertical mais toujours en horizontal
//!   [comme Combat] », qui décrivait Combat, pas une consigne pour le Suivi), corrigée dans l'heure
//!   suivante sur retour utilisateur explicite, capture d'écran de l'artefact à l'appui : « le
//!   bouton plus et moins qui doivent être verticaux et pas horizontaux comme les autres boutons
//!   external link et options ». Fond translucide ajouté derrière la paire (`icon_button::
//!   PANEL_BACKDROP_FILL`), avec une marge symétrique sur les quatre côtés (`CONTROL_BUTTON_GAP`,
//!   réutilisée aussi comme marge — même convention que `combat::ICON_BUTTON_GAP`), mais SANS
//!   changer l'orientation : Combat et Suivi partagent le même socle et la même teinte d'icône,
//!   PAS la même disposition. `content_width` recalcule `control_row_width` en conséquence (un seul
//!   bouton de large, pas deux). L'infobulle reste à GAUCHE (`design::tooltip`, côté `Left`) : la réserve
//!   `CONTROL_TOOLTIP_RESERVE` protège les deux boutons, empilés à la MÊME abscisse.
//!
//! **Refonte 2026-09-08 (déplacement Détails/Options depuis Combat)** — demande utilisateur
//! explicite : le panneau Combat n'est pas toujours affiché (aucun combat en cours), contrairement
//! au panneau Suivi qui reste visible en permanence ; les boutons "Détails"/"lien externe" et
//! "Options" (jusque-là dans `combat::bottom_toolbar`, retirée) rejoignent donc ce bandeau, dans le
//! carré 2×2 que devient `control_button_row` :
//! ```text
//! [+] [−]
//! [🔗] [⚙]
//! ```
//! "+" garde sa place (coin haut-gauche), "−" vient à sa DROITE (au lieu d'en dessous), "Détails"
//! (lien externe) sous le "+", "Options" (l'écrou) sous le "−" — disposition donnée explicitement
//! par l'utilisateur (Options/Détails initialement intervertis par erreur, corrigé dans l'heure sur
//! nouvelle demande explicite). Contrairement à "+"/"−" (restent INERTES, `Sense::hover()` seul),
//! "Options"/"Détails" restent CLIQUABLES (`Sense::click()`) exactement comme dans `combat::
//! bottom_toolbar` avant leur déplacement.
//!
//! **Infobulle par COLONNE, pas par bouton** (retour utilisateur explicite : « la souris doit
//! pouvoir passer d'un bouton à l'autre sans qu'il y ait un problème au niveau de la tooltip » — un
//! bouton de droite affichant son infobulle à GAUCHE la ferait apparaître PAR-DESSUS son voisin de
//! gauche, gênant le survol de ce dernier) : la colonne de GAUCHE ("+"/"Détails") affiche son
//! infobulle à GAUCHE (`design::tooltip`, côté `Left`), la colonne de DROITE ("−"/"Options") l'affiche à
//! DROITE (côté `Right`) — voir
//! `control_button`, dont le côté d'infobulle dépend maintenant de la COLONNE (paramètre
//! `TooltipSide`), pas du rôle inerte/cliquable du bouton. `CONTROL_TOOLTIP_RESERVE` (mesurée sur
//! "Supprimer (Ctrl+Shift+S)", le plus long des quatre libellés — désormais sur la colonne DROITE)
//! est réutilisée SYMÉTRIQUEMENT des deux côtés du carré (voir `content_width`) : plus un seul
//! côté à réserver, il en faut désormais un de chaque, sans quoi l'infobulle du bouton de droite
//! (« Supprimer »/« Options ») n'aurait pas la place de s'afficher entièrement à droite.
//!
//! Ce carré est maintenant peint INCONDITIONNELLEMENT (voir `show`, le garde `if !entries.
//! is_empty()` qui masquait TOUT le bandeau, boutons compris, est retiré) — sans quoi Options/
//! Détails resteraient inatteignables tant qu'aucun objet n'est suivi, ce que la fenêtre elle-même
//! ne laissait déjà plus deviner : `content_width(0)` réservait DÉJÀ la largeur du carré de
//! contrôle avant ce changement (voir sa doc), seul `show` ne peignait rien dans cet espace.
//!
//! **Règle supplémentaire** (demande utilisateur explicite) : le bouton "−" est visuellement
//! DÉSACTIVÉ (`design::icon_button`, paramètre `enabled`) tant qu'aucune entrée n'est suivie — rien
//! à supprimer dans ce cas. "+"/"Options"/"Détails" restent toujours activés (aucune des trois
//! actions ne dépend du contenu de la watchlist).
//!
//! **Migration 2026-09-10 (décision utilisateur, captures à l'appui)** : les quatre boutons du
//! carré de contrôle passent de `panels::icon_button::paint_icon_button` — qui prenait quatre
//! `egui::TextureHandle` en paramètres, à charge de l'appelant de les câbler — au composant
//! `design::icon_button`, qui résout ses textures par le manifeste. Ce panneau ne connaît plus
//! aucune texture de bouton, seulement quatre `DsTexture`. Détail des écarts de rendu et de ce qui
//! reste à la charge du panneau (le placement des infobulles) dans la doc de `control_button` ;
//! avant/après dans `overlay-testkit/tests/icon_button_migration.rs`.

use overlay_engine::{CatalogIndex, WatchlistEntry, WatchlistKind, WatchlistMode};

use crate::design::{self, text, DsIcon, IconContext};
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::ui_icons::UiIcons;

/// Durée d'affichage du toast d'alerte avant fermeture automatique (§9 du plan : « toast ≤ 5 s,
/// non bloquant ») — exportée pour que `main.rs` calcule `hide_at` avec la même valeur, sans la
/// dupliquer. Peut aussi être fermé PLUS TÔT par un clic (voir `toast_card`) : les deux cohabitent,
/// contrairement au réglage exclusif `ProfileService.alertManualClose` côté web.
pub const TOAST_DURATION: std::time::Duration = std::time::Duration::from_secs(5);

/// Distingue les deux déclencheurs de toast possibles (miroir de `LootAlertEvent.reason`,
/// `loot-alert.service.ts`) — seul le libellé affiché change (voir `toast_card`), le son a déjà
/// été choisi par l'appelant (`main.rs::spawn_engine_thread`, `alert_sound::{play_countdown_alert,
/// play_loot_alert}`) avant même la construction de ce toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchlistToastReason {
    /// Un décompte de suivi (mode `down`) vient d'atteindre 0.
    Countdown,
    /// Un objet à son activé (compte, voir `overlay_engine::profile`) vient d'être ramassé —
    /// `quantity` affichée seulement si > 1 (voir `toast_card`).
    Loot { quantity: i64 },
}

/// Un confetti du toast — mêmes bornes aléatoires que `buildConfetti()`
/// (`loot-alert.component.ts`) : position/délai/durée/rotation/couleur tirés UNE FOIS à la
/// création du toast (voir `build_confetti`, appelé par `main.rs::spawn_engine_thread`) puis
/// rejoués en boucle tant que le toast reste affiché — miroir de l'animation CSS `confetti-fall`
/// (`animation-iteration-count: infinite`), ici recalculée à chaque frame depuis
/// `WatchlistToast::created_at` puisque `egui` n'a pas de moteur d'animation CSS (voir `toast_card`).
#[derive(Debug, Clone, Copy)]
pub struct ConfettiPiece {
    /// Position horizontale, en fraction (0.0-1.0) de `TOAST_LAYER_WIDTH` — miroir de `left`
    /// (`Math.random() * 100`, en %).
    left_frac: f32,
    /// Retard avant le début de la chute, en secondes — miroir de `delay` (0.0-0.3s).
    delay: f32,
    /// Durée d'une chute complète, en secondes — miroir de `duration` (1.1-1.9s).
    duration: f32,
    /// Rotation totale atteinte en fin de chute, en radians — miroir de `rotate` (0-360deg,
    /// `--rot` en CSS).
    rotation: f32,
    color: egui::Color32,
}

/// Mêmes 8 couleurs que `CONFETTI_COLORS` (`loot-alert.component.ts`), reprises telles quelles.
const CONFETTI_COLORS: [egui::Color32; 8] = [
    egui::Color32::from_rgb(0xff, 0xb7, 0x03),
    egui::Color32::from_rgb(0xfb, 0x85, 0x00),
    egui::Color32::from_rgb(0x21, 0x9e, 0xbc),
    egui::Color32::from_rgb(0x8e, 0xca, 0xe6),
    egui::Color32::from_rgb(0xff, 0x00, 0x6e),
    egui::Color32::from_rgb(0x83, 0x38, 0xec),
    egui::Color32::from_rgb(0x3a, 0x86, 0xff),
    egui::Color32::from_rgb(0x06, 0xd6, 0xa0),
];
/// Même effectif que `CONFETTI_PIECE_COUNT` (`loot-alert.component.ts`).
const CONFETTI_PIECE_COUNT: usize = 28;

/// Générateur pseudo-aléatoire minimal (xorshift64) — pas de dépendance `rand` pour la seule
/// dispersion visuelle des confettis (aucun besoin cryptographique ni même de reproductibilité).
struct SmallRng(u64);

impl SmallRng {
    /// Graine dérivée de l'horloge système à chaque appel de `build_confetti`, pour que deux
    /// toasts consécutifs n'affichent pas exactement la même dispersion.
    fn seeded() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9_7f4a_7c15);
        Self(nanos | 1) // jamais 0 : état absorbant du xorshift
    }

    /// Suivant, dans `[0.0, 1.0)`.
    fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u32 << 24) as f32
    }
}

/// Construit un jeu de confettis aléatoire — miroir de `buildConfetti()`
/// (`loot-alert.component.ts`), appelé une fois par déclenchement de toast (voir
/// `main.rs::spawn_engine_thread`) pour que la dispersion reste stable tant que le toast est
/// affiché, plutôt que régénérée à chaque frame.
pub fn build_confetti() -> Vec<ConfettiPiece> {
    let mut rng = SmallRng::seeded();
    (0..CONFETTI_PIECE_COUNT)
        .map(|_| ConfettiPiece {
            left_frac: rng.next_f32(),
            delay: rng.next_f32() * 0.3,
            duration: 1.1 + rng.next_f32() * 0.8,
            rotation: rng.next_f32() * std::f32::consts::TAU,
            color: CONFETTI_COLORS
                [(rng.next_f32() * CONFETTI_COLORS.len() as f32) as usize % CONFETTI_COLORS.len()],
        })
        .collect()
}

/// Un décompte de suivi à 0 OU un ramassage à son activé (voir `WatchlistToastReason`) — construit
/// par `main.rs::spawn_engine_thread` à réception de l'alerte, publié via `ArcSwap` (comme
/// `watchlist`/`snapshot`) pour que le thread UI l'affiche sans coupler le thread Engine au rendu.
#[derive(Debug, Clone)]
pub struct WatchlistToast {
    pub name: String,
    pub kind: WatchlistKind,
    pub reason: WatchlistToastReason,
    /// Id catalogue de l'objet/monstre, quand connu — résolution non ambiguë de l'icône affichée
    /// (voir `toast_card`), même principe que `WatchlistEntry::catalog_id`.
    pub catalog_id: Option<i64>,
    /// Instant de création — sert de référence de temps à l'animation d'entrée et à la boucle de
    /// confettis (voir `toast_card`), indépendamment de `hide_at`.
    pub created_at: std::time::Instant,
    /// Dispersion tirée une fois à la création (voir `build_confetti`) — stable tant que le toast
    /// reste affiché.
    pub confetti: Vec<ConfettiPiece>,
    /// Instant auquel le toast doit cesser de s'afficher — comparé à l'instant courant (`now`,
    /// fourni par l'appelant, voir `show`/`is_active`) à chaque rendu plutôt que de faire expirer
    /// activement l'`ArcSwap` : cette architecture n'a pas de boucle de rendu continue (§6.1 du
    /// plan), `main.rs::render`
    /// reprogramme lui-même un redessin à cette échéance via `OverlayWindow::next_redraw_at` pour
    /// que le toast disparaisse sans qu'aucun autre événement n'ait à se produire. Peut aussi être
    /// effacé PLUS TÔT par un clic (voir `toast_card`, `main.rs::window_event`).
    pub hide_at: std::time::Instant,
}

/// Vrai tant que `toast` n'a pas atteint son expiration — source de vérité unique utilisée à la
/// fois ici (`show`) et par `main.rs` (gabarit dynamique de la fenêtre Suivi, garde d'affichage
/// quand la watchlist elle-même est vide) sur « un toast est-il actuellement affiché ? ».
///
/// `now` est fourni par l'appelant plutôt que lu ici (horloge injectable, §17.1 du plan) : lire
/// `Instant::now()` à l'intérieur de cette fonction rendrait le résultat dépendant de l'instant
/// réel d'exécution, donc non reproductible pour un futur harnais de rendu offscreen qui fige
/// `now` — un même `WatchlistToast` doit produire le même résultat quel que soit le moment où le
/// test tourne. `main.rs` calcule `now` UNE FOIS par frame (`window_event`) et le fait transiter
/// jusqu'ici comme jusqu'à `toast_card`, pour qu'un même rendu utilise une seule référence de
/// temps cohérente.
pub fn is_active(toast: Option<&WatchlistToast>, now: std::time::Instant) -> bool {
    toast.is_some_and(|t| t.hide_at > now)
}

/// 58×58, coins arrondis 10px — mêmes dimensions que `.kpi` (`tracker-strip.component.css`),
/// repliée (pas l'état `.expanded` au clic, jamais câblé côté overlay pour l'instant).
const TILE_SIZE: f32 = 58.0;
/// Un peu plus que le simple espacement visuel du web (`.kpi-strip { gap: ... }`) — écart
/// suffisant pour distinguer clairement deux tuiles adjacentes.
const TILE_GAP: f32 = 12.0;
const TILE_ROUNDING: f32 = 10.0;
/// `app-item-icon [size]="30"` dans le template web — mêmes proportions ; ne s'applique plus qu'aux
/// tuiles ENNEMI (voir `entry_tile`), les tuiles OBJET utilisant désormais `ITEM_ICON_SIZE`.
const ICON_SIZE: f32 = 30.0;

/// Fond translucide peint sous le carré de contrôle, AVANT ses boutons.
///
/// La planche de référence fournie par l'utilisateur (2026-09-06) montre un léger fond noir
/// semi-opaque derrière la bande de boutons de premier plan du jeu, visible dans le petit écart
/// entre deux boutons adjacents et sur les bords. Même teinte que `panels::combat::
/// LEADER_PANEL_FILL`, un bandeau translucide déjà validé ailleurs dans cette interface — pas une
/// nouvelle valeur d'opacité inventée pour l'occasion.
///
/// Vivait dans `panels::icon_button` jusqu'au 2026-09-10, du temps où `combat::bottom_toolbar` en
/// avait besoin elle aussi ; ce carré en est le seul utilisateur depuis que les quatre boutons y
/// ont été regroupés.
const PANEL_BACKDROP_FILL: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(10, 12, 16, 150);
/// Rayon d'angle du fond translucide — voir [`PANEL_BACKDROP_FILL`].
const PANEL_BACKDROP_ROUNDING: f32 = 6.0;

/// Taille (largeur ET hauteur) du socle des 4 boutons du carré de contrôle
/// (`DsTexture::ButtonIconFirstPlan`, voir doc de module, migration 2026-09-10) — mise à l'échelle
/// du socle NATIF (`design::tokens::ICON_BUTTON_SIZE`, 36×36), pas une taille fixe indépendante. Ramenée
/// de 34×34 (taille déjà validée par l'utilisateur avant le passage au design system commun) à
/// 24×24 — à l'essai, retour utilisateur explicite 2026-09-06 (« essaie vingt-quatre sur
/// vingt-quatre pour voir le rendu que ça fait »).
const CONTROL_BUTTON_SIZE: f32 = 24.0;
/// Écart entre deux boutons adjacents du carré de contrôle (voir doc de module, refonte
/// 2026-09-06 — 1×2 boutons empilés à l'origine, 2×2 depuis la refonte 2026-09-08), réutilisé
/// aussi comme marge du fond translucide sur les quatre côtés — volontairement plus serré que
/// `TILE_GAP` (les boutons du carré forment un seul groupe visuel, pas des entrées indépendantes).
const CONTROL_BUTTON_GAP: f32 = 4.0;
/// Espace réservé de CHAQUE CÔTÉ du carré de contrôle, pour que l'infobulle des boutons de sa
/// colonne de GAUCHE (`design::tooltip`, côté `Left`) et de sa colonne de DROITE (côté `Right`) aient
/// matériellement la place de s'afficher entièrement — retour utilisateur 2026-09-06, capture
/// d'écran à l'appui : sans la réserve GAUCHE, l'infobulle de "+" s'affichait à DROITE (chevauchant
/// la première tuile) malgré `RectAlign::LEFT` demandé, car la fenêtre Suivi est dimensionnée pile
/// sur son contenu (`content_width`) — la colonne de contrôle est le tout premier élément, collé au
/// bord gauche de la fenêtre à quelques pixels de marge près (`WATCHLIST_INNER_MARGIN`, `main.rs`) :
/// `RectAlign::find_best_align` (voir sa doc, `design::tooltip`) rejette alors
/// LEFT/LEFT_START/LEFT_END, aucun n'y tenant, et retombe sur RIGHT — PHYSIQUEMENT, une popup ne
/// peut pas se peindre en dehors de la fenêtre qui la contient (contrairement à `combat::
/// `design::tooltip`, où le problème ne concernait qu'un AXE d'alignement, ici il n'existe aucun
/// repli qui n'exige pas de place à gauche).
///
/// Valeur mesurée (pas devinée) : rendu offscreen du bouton survolé sur un canevas large (sans
/// contrainte de bord), diff pixel par pixel avec le même rendu non survolé — l'infobulle
/// "Supprimer" (le plus long des quatre libellés du carré) occupe alors ~74px de large avec ~4px
/// d'écart depuis le bord du bouton, soit ~78px de pied total ; arrondi à 88px pour absorber marge
/// d'erreur de mesure/anticrénelage.
///
/// **Refonte 2026-09-08** : réutilisée SYMÉTRIQUEMENT à DROITE du carré depuis que la colonne de
/// droite ("−"/"Options") affiche elle aussi son infobulle à droite (côté `Right`, voir
/// doc de module) — sans cette réserve miroir, le même problème de place se reproduirait côté
/// droit dès que la watchlist est vide ou n'a que peu d'entrées (`content_width` n'aurait alors
/// presque aucune largeur après le carré). "Supprimer" restant le plus long des quatre libellés et
/// se trouvant justement sur cette colonne droite, AUCUNE nouvelle mesure n'est nécessaire — même
/// valeur, réutilisée telle quelle des deux côtés.
///
/// **Contrepartie assumée** : `content_width` (donc la largeur de FENÊTRE) grandit d'autant, et la
/// fenêtre Suivi étant centrée horizontalement sur cette largeur (`main.rs::anchor_position`), la
/// bande de tuiles visible se retrouve décalée d'environ la moitié de la réserve GAUCHE (~44px) à
/// DROITE du centre réel de la fenêtre de jeu — même compromis déjà accepté pour l'élargissement
/// temporaire du toast (`TOAST_LAYER_WIDTH`, `toast_card`), ici permanent tant que la bande est
/// affichée plutôt que ponctuel.
const CONTROL_TOOLTIP_RESERVE: f32 = 88.0;

/// Marge intérieure des textures `Border-<RARETÉ>.webp` — mesurée par script Python/Pillow
/// (bbox de la fenêtre où l'icône doit se peindre, transition alpha/couleur repérée à 52px puis
/// 460px sur un canevas 512×512, IDENTIQUE sur les 7 fichiers) : voir doc de module et
/// `docs/design-system.md` §2.4/§7. `1.0 - 2.0 * ITEM_BORDER_INNER_MARGIN_RATIO` donne la fraction
/// de `TILE_SIZE` correspondant à la fenêtre intérieure (~0.797, `entry_tile`).
const ITEM_BORDER_INNER_MARGIN_RATIO: f32 = 52.0 / 512.0;
/// Fraction de la fenêtre intérieure mesurée (voir ci-dessus) effectivement occupée par l'icône
/// d'objet — pas 100% : les captures de référence (`common-items.png` et consorts, voir
/// `docs/design-system.md` §7) montrent toujours une petite marge entre l'icône et le cadre, jamais
/// un remplissage pixel-perfect du carré intérieur. Relevé de 0.9 à 0.96 (retour utilisateur : «
/// les objets doivent être plus gros ») une fois le fond de bordure repeint EN DESSOUS de l'icône
/// (voir doc de module, correctif same-day) — plus aucun risque que la bordure recouvre un débord.
const ITEM_ICON_FILL_RATIO: f32 = 0.96;
/// Taille cible de l'icône d'une tuile OBJET — dérivée des deux constantes ci-dessus plutôt qu'une
/// valeur fixe indépendante, pour rester proportionnée si `TILE_SIZE` change un jour.
const ITEM_ICON_SIZE: f32 =
    TILE_SIZE * (1.0 - 2.0 * ITEM_BORDER_INNER_MARGIN_RATIO) * ITEM_ICON_FILL_RATIO;

/// Marge entre le texte du compteur (voir `paint_count_inline`) et le bord DROIT de la tuile —
/// assez pour rester lisible par-dessus le cadre de rareté (dont le liseré occupe déjà quelques
/// pixels, voir `ITEM_BORDER_INNER_MARGIN_RATIO`) sans empiéter dessus. Distincte de
/// `COUNT_INSET_BOTTOM` depuis le retour utilisateur 2026-09-06 (« décale d'un pixel vers la
/// gauche [...] et descends-le d'un pixel vers le bas ») : les deux marges n'ont plus besoin de
/// rester égales.
const COUNT_INSET_RIGHT: f32 = 6.0;
/// Marge entre le texte du compteur et le bord BAS de la tuile — voir `COUNT_INSET_RIGHT`, 1px de
/// moins pour rapprocher le nombre du bas (même retour utilisateur).
const COUNT_INSET_BOTTOM: f32 = 4.0;

/// Taille du compteur (voir `paint_count_inline`) — retour utilisateur 2026-09-06, capture d'écran
/// du jeu à l'appui (quantités d'objets en bas-droit d'un emplacement) : à 11px le nombre était
/// « pas du tout lisible » par comparaison. Un premier essai à 15px s'est avéré « beaucoup trop
/// élevé » une fois comparé en jeu (second retour, captures d'écran des deux côte à côte) — ramené
/// à 13px, puis affiné à 14px (troisième retour, après test en conditions réelles).
const COUNT_FONT_SIZE: f32 = 14.0;

/// Essai demandé par l'utilisateur 2026-09-06 (test volontaire à grandes valeurs, captures d'écran
/// à l'appui : un décompte "500/500" ou "2000/2000" — deux fois un nombre à 3-4 chiffres plus le
/// symbole de fraction sur UNE SEULE ligne — débordait de la tuile (58px) et chevauchait la tuile
/// voisine). Plutôt que de composer courant+"/"+cible côte à côte (voir l'ancienne version de
/// `paint_count_inline`, gardée en mémoire dans l'historique Git), la fraction cible passe
/// maintenant SOUS le nombre courant : `TARGET_LINE_OFFSET` décale son ancrage vers le bas depuis
/// la MÊME position que le nombre courant, `TARGET_FONT_SIZE` réduit sa police puisqu'elle ne
/// porte plus qu'une information secondaire. Le nombre courant, lui, retrouve exactement
/// l'emplacement/la police du mode `up` (aucun décalage horizontal pour laisser place à la
/// fraction, qui n'est plus sur la même ligne) — demande explicite : « la couleur et le nombre de
/// l'élément courant [...] le même emplacement que tous les autres nombres ».
///
/// Ramené de 20 à 12px (retour utilisateur suivant, sur le premier essai) : les deux lignes
/// étaient trop écartées l'une de l'autre.
const TARGET_LINE_OFFSET: f32 = 12.0;
/// Police de la fraction cible (ex. "/500") sous le nombre courant — plus petite que
/// `COUNT_FONT_SIZE` (essai demandé : « on réduit la fonte [...] on la mettrait en onze ou en
/// dix »), gardée en monospace comme le reste du compteur.
const TARGET_FONT_SIZE: f32 = 10.0;

/// Largeur de contenu nécessaire pour afficher `entry_count` entrées + la colonne de contrôle
/// ("+"/"−" empilés), SANS la marge de fenêtre (`egui::Frame::NONE.inner_margin`, ajoutée côté
/// appelant) — utilisée par `main.rs` pour dimensionner dynamiquement la fenêtre Suivi (retour
/// utilisateur 2026-09-02 : « je ne veux pas de fond, je veux que ça reste transparent, mais [...]
/// l'overlay n'a pas plus de taille s'il n'y a pas besoin » — une fenêtre plus large que son
/// contenu reste cliquable/bloquante sur toute sa zone même là où rien n'est visible, l'utilisateur
/// ne peut alors pas deviner où s'arrête l'overlay). Seule source de vérité pour `TILE_SIZE`/
/// `TILE_GAP`/`CONTROL_BUTTON_SIZE` : `main.rs` ne les duplique pas.
///
/// Refonte 2026-09-06 : la colonne de contrôle vaut maintenant `CONTROL_BUTTON_SIZE` de large (les
/// deux boutons sont empilés, plus côte à côte) au lieu de `2 * TILE_SIZE + TILE_GAP` — et plus
/// aucune réserve `BADGE_OVERFLOW` : le compteur ne déborde plus de sa tuile (voir
/// `paint_count_inline`). `CONTROL_TOOLTIP_RESERVE` ajoutée le même jour (voir sa doc) : espace à
/// gauche de la colonne pour que l'infobulle "Ajouter"/"Supprimer" puisse réellement s'afficher à
/// gauche plutôt que de retomber à droite faute de place.
///
/// **Refonte 2026-09-06 (design system boutons icône)** : la colonne de contrôle utilise maintenant
/// `control_row_width` (un fond translucide autour des boutons empilés, voir `control_button_row`)
/// au lieu du seul `CONTROL_BUTTON_SIZE` — largeur quasi identique (le fond n'ajoute qu'une petite
/// marge de chaque côté), la disposition reste VERTICALE (retour utilisateur explicite : « plus et
/// moins doivent être verticaux, pas horizontaux comme Combat » — une première tentative les avait
/// passés en horizontal par erreur, corrigée le même jour).
///
/// **Refonte 2026-09-08** : le carré de contrôle passe de 1×2 à 2×2 boutons (voir doc de module,
/// déplacement Détails/Options depuis Combat) — `control_row_width` gagne une colonne (voir sa
/// doc), et n'est plus jamais omis même sans entrée (`entry_count == 0` continuait déjà à le
/// réserver AVANT ce changement, seul `show` ne le peignait pas — voir doc de module).
///
/// **Refonte 2026-09-08 (infobulles par colonne)** : la colonne DROITE du carré ("−"/"Options")
/// affiche désormais son infobulle à DROITE (côté `Right`, voir doc de module) — l'espace
/// après le carré (`TILE_GAP + entries_width`, jusqu'ici seulement un espacement visuel avant la
/// première tuile) doit donc lui aussi garantir au moins `CONTROL_TOOLTIP_RESERVE` de large, MÊME
/// SANS ENTRÉE, sans quoi cette infobulle n'aurait pas la place de s'afficher entièrement (exactement
/// le problème que cette réserve résolvait déjà à GAUCHE, voir sa doc). `.max(...)` plutôt qu'une
/// simple addition : quand les tuiles d'entrées fournissent déjà assez de largeur, aucun espace
/// supplémentaire n'est ajouté (le carré reste juste avant la première tuile, comme avant).
pub fn content_width(entry_count: usize) -> f32 {
    let entries_width = if entry_count == 0 {
        0.0
    } else {
        entry_count as f32 * TILE_SIZE + (entry_count as f32 - 1.0) * TILE_GAP
    };
    let right_of_control = (TILE_GAP + entries_width).max(CONTROL_TOOLTIP_RESERVE);
    CONTROL_TOOLTIP_RESERVE + control_row_width() + right_of_control
}

/// Largeur du fond translucide derrière le carré de contrôle (voir `control_button_row`) — DEUX
/// boutons de large depuis la refonte 2026-09-08 (carré 2×2 : "+"/"−" en haut, "Options"/"Détails"
/// en dessous, voir doc de module) plus une marge symétrique de `CONTROL_BUTTON_GAP` de chaque côté
/// et entre les deux colonnes — MÊME formule que `control_row_height` (le carré est, comme son nom
/// l'indique, un carré : largeur et hauteur coïncident). Fonction plutôt que constante : combine
/// deux `const f32`, une multiplication de `f32` en contexte `const` restant plus fragile à faire
/// évoluer ici qu'un simple appel.
fn control_row_width() -> f32 {
    CONTROL_BUTTON_GAP * 3.0 + CONTROL_BUTTON_SIZE * 2.0
}

/// Hauteur du même fond translucide — DEUX boutons empilés (voir `control_row_width`) plus une
/// marge symétrique en haut/bas et l'écart entre les deux au milieu.
fn control_row_height() -> f32 {
    CONTROL_BUTTON_GAP * 3.0 + CONTROL_BUTTON_SIZE * 2.0
}

// Jetons repris tels quels de `:root` (`styles.css`, thème sombre par défaut — seul thème que
// l'overlay reproduit pour l'instant, voir `panels::combat::ACCENT` et sa propre justification).
/// `--panel-bg`.
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(0x1e, 0x1e, 0x1e);
/// `--surface-well` — fond du badge de compteur.
const SURFACE_WELL: egui::Color32 = egui::Color32::from_rgb(0x18, 0x18, 0x18);
/// `--border-strong` — bordure du badge ET bordure "monstre" (`.kpi.is-monster`).
const BORDER_STRONG: egui::Color32 = egui::Color32::from_rgb(0x4d, 0x4d, 0x4d);
/// `--text-muted` — cible grisée d'un décompte, texte des tuiles "+"/"−".
const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(0x88, 0x88, 0x88);
/// Gris de la fraction cible d'un décompte (`paint_count_inline`) — plus clair que `TEXT_MUTED`
/// (retour utilisateur 2026-09-06 : « éclaircir le gris utilisé pour la partie fraction »).
/// Constante dédiée plutôt qu'un simple relèvement de `TEXT_MUTED` : ce dernier reste utilisé tel
/// quel ailleurs (tuiles "+"/"−", bordure "monstre") où aucun éclaircissement n'a été demandé.
const TARGET_TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(0xb0, 0xb0, 0xb0);
/// Compte en mode `up` (`paint_count_inline`) — mirait `--text-color` (`0xe0e0e0`, gris très clair)
/// jusqu'au retour utilisateur 2026-09-06 : « la couleur des chiffres [...] c'est bien du blanc
/// rgb(255,255,255) ? [...] j'ai l'impression que c'est ce qui change véritablement la lecture ».
/// Vérifié par échantillonnage pixel sur sa capture d'écran de référence (quantités du jeu) :
/// `(255,255,255)` exactement, pas le gris du jeton web — ce badge suit ici l'apparence du JEU
/// (voir doc de module, refonte 2026-09-06), pas le dépôt web, d'où la divergence assumée.
const TEXT_COLOR: egui::Color32 = egui::Color32::WHITE;
/// `--kama-color` — valeur COURANTE d'un décompte (`.kpi-count-badge.is-fraction`).
const KAMA_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 215, 0);

// `--accent` — bordure ET titre du toast (`loot-alert-card`/`loot-alert-title`,
// `loot-alert.component.css`), les deux réutilisent le même jeton quel que soit `reason`. Repris du
// jeton partagé : voir `tokens::OVERLAY_ACCENT`.
use crate::design::tokens::OVERLAY_ACCENT as ACCENT;
/// `--surface-raised` — fond du toast (`loot-alert-card`, dégradé à deux arrêts IDENTIQUES côté
/// web donc simple aplat ici).
const SURFACE_RAISED: egui::Color32 = egui::Color32::from_rgb(0x26, 0x26, 0x26);
/// `--text-bright` — nom de l'objet/monstre dans le toast (`loot-alert-name`).
const TEXT_BRIGHT: egui::Color32 = egui::Color32::from_rgb(0xf2, 0xf2, 0xf2);
/// `--tint-medium` — fond du bouton de fermeture au repos (`loot-alert-close`).
const TINT_MEDIUM: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 31);
/// `--tint-strong` — fond du bouton de fermeture survolé (`loot-alert-close:hover`).
const TINT_STRONG: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 46);

/// Largeur de la couche de confettis (`.confetti-layer`, `loot-alert.component.css`) — reprise
/// telle quelle du web (320px), centrée sur le même axe que la carte : `main.rs::
/// watchlist_target_width` s'en sert pour élargir la fenêtre Suivi le temps qu'un toast est
/// affiché, sans quoi les confettis les plus excentrés seraient rognés par le bord de fenêtre.
pub const TOAST_LAYER_WIDTH: f32 = 320.0;
/// Hauteur SUPPLÉMENTAIRE à réserver sous la bande de tuiles quand un toast est affiché (carte +
/// dépassement des confettis, voir `toast_card`) — ajoutée par `main.rs::watchlist_target_height`
/// à la hauteur de base, seulement tant qu'un toast est actif (voir `is_active`), pour ne pas
/// garder en permanence une zone de fenêtre cliquable/bloquante plus grande que nécessaire (même
/// principe que `watchlist_target_width` pour la largeur).
pub const TOAST_AREA_HEIGHT: f32 = 170.0;

const CARD_ROUNDING: f32 = 12.0;
const CARD_BORDER_WIDTH: f32 = 1.0;
const CARD_PAD_V: f32 = 10.0;
const CARD_PAD_LEFT: f32 = 16.0;
/// Plus large qu'à gauche : réserve la place du bouton de fermeture (coin haut-droit, voir
/// `close_rect`) — miroir de `padding: 10px 30px 10px 18px` (`loot-alert-card`).
const CARD_PAD_RIGHT: f32 = 28.0;
const CARD_ICON_GAP: f32 = 10.0;
/// Écart vertical entre le titre et le nom — miroir de l'empilement `flex-direction: column` sans
/// gap explicite de `.loot-alert-text` (léger espace naturel entre deux lignes de texte).
const CARD_TEXT_GAP: f32 = 2.0;
/// Écart entre le bas de la bande de tuiles et le haut de la carte.
const CARD_TOP_GAP: f32 = 12.0;
const CLOSE_BTN_SIZE: f32 = 18.0;
const CLOSE_BTN_ROUNDING: f32 = 4.0;
const CLOSE_BTN_MARGIN: f32 = 4.0;
/// Durée de l'animation d'entrée — miroir de `@keyframes loot-pop` (0.35s). `egui` n'exprimant pas
/// de transformation `scale`, l'entrée est ici un fondu + léger glissement vertical plutôt qu'un
/// vrai zoom (voir `toast_card`).
const POP_DURATION: f32 = 0.35;
/// Les confettis démarrent au-dessus du haut de la carte — miroir de `top: -10px` du calque de
/// confettis relatif au conteneur, dont le `margin-top: 40px` pousse la carte plus bas (ici
/// ramené à une valeur plus modeste, la fenêtre Suivi ayant nettement moins de hauteur disponible
/// qu'une page web).
const CONFETTI_TOP_OVERSHOOT: f32 = 18.0;
/// Distance de chute d'un confetti — réduite par rapport aux 220px web (`translateY(220px)`,
/// `@keyframes confetti-fall`) pour tenir dans `TOAST_AREA_HEIGHT`.
const CONFETTI_FALL_HEIGHT: f32 = 120.0;
/// Même taille que `.confetti-piece` (8x8px, `loot-alert.component.css`).
const CONFETTI_SIZE: f32 = 8.0;

/// Dépendances de rendu communes à ce panneau (icônes UI génériques, catalogue, store/cache
/// d'icônes réelles) — regroupées ici pour que `show` reste sous la limite clippy
/// `too_many_arguments` une fois `now` ajouté (horloge injectable, voir sa doc et celle de
/// `is_active`) : même motif que `RenderContent`/`AppState` (`main.rs`), pas un
/// `#[allow(clippy::too_many_arguments)]`.
pub struct WatchlistAssets<'a> {
    pub icons: &'a UiIcons,
    pub catalog: &'a CatalogIndex,
    pub remote_icons: &'a RemoteIconStore,
    pub remote_icon_textures: &'a mut RemoteIconTextures,
}

/// Renvoie `true` quand l'utilisateur vient de fermer le toast affiché (clic sur la carte ou sur
/// sa croix, voir `toast_card`) — `main.rs::window_event` est seul à détenir un accès en écriture
/// à l'`ArcSwap` du toast, donc seul à pouvoir agir sur ce signal.
///
/// `now` : voir la doc de `is_active` — même horloge injectable, propagée jusqu'à `toast_card`
/// (fondu d'entrée, chute des confettis).
///
/// **2026-09-08 (modale Options)** : `show` renvoie désormais [`WatchlistOutcome`] plutôt qu'un
/// simple `bool` — le clic sur "Options" (`control_button_row`) doit remonter jusqu'à
/// `render_content::paint_content`, qui seul peut déclencher l'ouverture d'une fenêtre OS dédiée
/// (`main.rs`/`bin/overlay-ui-x11.rs`, nouveau cas `OverlayKind::Options`) ; `close_toast` garde
/// exactement son rôle d'avant (fermeture du toast, voir plus bas).
pub fn show(
    ui: &mut egui::Ui,
    assets: WatchlistAssets<'_>,
    entries: &[WatchlistEntry],
    toast: Option<&WatchlistToast>,
    now: std::time::Instant,
) -> WatchlistOutcome {
    let WatchlistAssets {
        icons,
        catalog,
        remote_icons,
        remote_icon_textures,
    } = assets;
    // Le carré de contrôle ("+"/"−"/"Options"/"Détails", voir `control_button_row`) est peint
    // INCONDITIONNELLEMENT depuis la refonte 2026-09-08 (voir doc de module) — Options/Détails
    // doivent rester atteignables même sans aucune entrée suivie, contrairement aux tuiles
    // d'entrées elles-mêmes (`entries.iter()` ci-dessous, seule partie encore vide si `entries`
    // l'est). Un ramassage à son activé (`overlay_engine::profile`, INDÉPENDANT de la watchlist)
    // doit de toute façon pouvoir déclencher un toast même sans entrée suivie — cette garde ne
    // s'est donc jamais étendue au toast, peint plus bas hors de ce bloc.
    let mut style = (**ui.style()).clone();
    style_thin_scrollbar(&mut style);
    ui.set_style(style);

    // Renseigné par `control_button_row` dans la fermeture ci-dessous (voir la doc de `show`) —
    // `false` par défaut : aucune raison de rouvrir la modale si elle l'est déjà tant que
    // l'utilisateur n'a pas recliqué sur "Options".
    let mut open_options = false;
    let mut open_web_app = false;

    egui::ScrollArea::horizontal()
        .id_salt("watchlist-strip")
        .auto_shrink([false, true])
        // Un peu plus que la seule hauteur des tuiles (58px) : donne à la barre de défilement
        // flottante une bande dégagée sous les icônes/badges plutôt que de la faire chevaucher
        // presque entièrement — retour utilisateur 2026-09-02 : « impossible de l'agripper ».
        .min_scrolled_height(TILE_SIZE + 14.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Réserve à GAUCHE de la colonne de contrôle pour que son infobulle ait la
                // place de s'afficher à gauche (voir `CONTROL_TOOLTIP_RESERVE`) — zone
                // transparente, aucun élément peint ni interactif dedans.
                ui.add_space(CONTROL_TOOLTIP_RESERVE);

                // Carré "+"/"−"/"Options"/"Détails" (voir doc de module, refonte 2026-09-08) —
                // `ui.horizontal` centre ses enfants verticalement par défaut, ce qui aligne
                // naturellement ce carré sur le centre des tuiles d'entrée (58px) juste à côté.
                let clicks = control_button_row(ui, entries.is_empty());
                open_options = clicks.options;
                open_web_app = clicks.details;
                ui.add_space(TILE_GAP);

                for (i, entry) in entries.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(TILE_GAP);
                    }
                    entry_tile(
                        ui,
                        icons,
                        catalog,
                        remote_icons,
                        remote_icon_textures,
                        entry,
                    );
                }
            });
        });

    ui.add_space(6.0);

    let close_toast = match toast.filter(|t| t.hide_at > now) {
        Some(toast) => toast_card(
            ui,
            icons,
            catalog,
            remote_icons,
            remote_icon_textures,
            toast,
            now,
        ),
        None => false,
    };

    WatchlistOutcome {
        close_toast,
        open_options,
        open_web_app,
    }
}

/// Ce que `show` a produit CETTE frame — voir sa doc pour pourquoi un simple `bool` (juste
/// `close_toast`, avant le 2026-09-08) ne suffit plus.
#[derive(Debug, Clone, Copy, Default)]
pub struct WatchlistOutcome {
    pub close_toast: bool,
    pub open_options: bool,
    /// `true` à la frame où "Détails" vient d'être cliqué : l'appelant ouvre la web app. Le
    /// panneau ne l'ouvre PAS lui-même — voir `control_button_row`.
    pub open_web_app: bool,
}

/// Barre de défilement fine, flottante et sombre plutôt que le style natif par défaut (épais, pris
/// dans le flux, gris clair) — retour utilisateur 2026-09-02 (capture d'écran à l'appui) : « le
/// scroll passe SUR les objets », « ce n'est pas très moderne », « qu'on ait juste à scroller
/// avec la molette plutôt que de bouger le scroll manuellement ». `ScrollStyle::thin()` (preset
/// `egui`, flottant + fin au repos + s'élargit au survol) répond exactement à ça — seules les
/// COULEURS restent celles d'egui par défaut (pensées pour un thème clair) sans cet ajustement,
/// remplacées ici par les jetons déjà utilisés pour le badge de compteur, cohérents avec le reste
/// du panneau. S'applique à tout le `Style` de CE contexte egui (une fenêtre = un contexte, voir
/// `main.rs`) : sans effet sur la fenêtre Combat, qui n'a pas de zone défilante.
fn style_thin_scrollbar(style: &mut egui::Style) {
    style.spacing.scroll = egui::style::ScrollStyle::thin();
    style.visuals.widgets.noninteractive.bg_fill = SURFACE_WELL;
    style.visuals.widgets.inactive.bg_fill = BORDER_STRONG;
    style.visuals.widgets.hovered.bg_fill = TEXT_MUTED;
    style.visuals.widgets.active.bg_fill = TEXT_MUTED;
    // « La molette n'est pas prise en compte » (retour utilisateur 2026-09-02) : par défaut, egui
    // ne route la molette (verticale) vers une zone de défilement HORIZONTALE que si l'utilisateur
    // maintient Maj — `always_scroll_the_only_direction` lève cette exigence quand une seule
    // direction est activée (notre cas, `ScrollArea::horizontal()`), exactement le comportement
    // demandé (« que si on utilise la molette [...] ça applique le scroll »).
    style.always_scroll_the_only_direction = true;
    // Élimine l'espacement AUTOMATIQUE qu'`egui` insère entre deux éléments d'un même
    // `ui.horizontal` (`spacing.item_spacing`, par défaut ~8px) — sans ça, chaque `add_space`
    // explicite de `TILE_GAP` s'additionne à cet espacement caché, et `content_width` (dont
    // `main.rs` dépend pour dimensionner la fenêtre) sous-estime la largeur réellement occupée :
    // c'est la cause du décalage constaté par l'utilisateur (« il manque littéralement un
    // objet » — la dernière tuile débordait hors de la fenêtre, trop étroite de quelques dizaines
    // de pixels). `TILE_GAP` reste la SEULE source d'espacement horizontal après ce réglage.
    style.spacing.item_spacing.x = 0.0;
}

/// Multiplie le canal alpha de `color` par `factor` (`0.0..=1.0`) — sert au fondu d'entrée du
/// toast (voir `POP_DURATION`) et au fondu de sortie de chaque confetti (voir `toast_card`),
/// appliqué uniformément à tous les éléments peints (fond, bordure, texte, icône).
fn with_alpha(color: egui::Color32, factor: f32) -> egui::Color32 {
    let a = (color.a() as f32 * factor.clamp(0.0, 1.0)).round() as u8;
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}

/// Les 4 coins d'un carré de côté `2*half` centré sur `center`, tournés de `angle` radians —
/// `epaint` n'a pas de primitive « rectangle tourné », un confetti (`toast_card`) se peint donc
/// comme un polygone convexe à 4 points calculés à la main.
fn rotated_square(center: egui::Pos2, half: f32, angle: f32) -> [egui::Pos2; 4] {
    let (sin, cos) = angle.sin_cos();
    [
        egui::vec2(-half, -half),
        egui::vec2(half, -half),
        egui::vec2(half, half),
        egui::vec2(-half, half),
    ]
    .map(|c| center + egui::vec2(c.x * cos - c.y * sin, c.x * sin + c.y * cos))
}

/// Carte d'alerte de ramassage/décompte — miroir visuel de `loot-alert.component.html`/`.css` du
/// dépôt web : icône réelle, titre coloré (`ACCENT`, identique pour les deux `reason` — voir sa
/// doc), nom (+ quantité si > 1), bouton de fermeture, confettis tombants en fond. Contrairement à
/// `LootAlertComponent` (minuterie OU fermeture manuelle, réglage exclusif côté web via
/// `ProfileService.alertManualClose` — pas encore porté ici), les DEUX cohabitent déjà : minuterie
/// fixe (voir `WatchlistToast::hide_at`) ET fermeture au clic (carte entière ou croix), sans que
/// l'utilisateur ait à choisir. Renvoie `true` quand CE clic doit effacer le toast (voir la doc de
/// `show` — seul `main.rs::window_event` peut écrire dans l'`ArcSwap` correspondant).
///
/// `now` : voir la doc de `is_active` — pilote à la fois le fondu d'entrée (`pop_t`/`slide`, via
/// `elapsed`) et la boucle de confettis ci-dessous. Fourni par l'appelant plutôt que lu ici :
/// c'était le seul point de ce panneau encore couplé à l'horloge murale, ce qui aurait rendu deux
/// rendus du même `WatchlistToast` visuellement différents selon l'instant réel d'exécution —
/// incompatible avec un harnais de rendu offscreen déterministe (§17.1 du plan). `toast.confetti`
/// lui n'a jamais posé ce problème : sa dispersion est tirée une fois à la création du toast
/// (`build_confetti`, appelé par `main.rs::spawn_engine_thread`), jamais régénérée ici.
fn toast_card(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    toast: &WatchlistToast,
    now: std::time::Instant,
) -> bool {
    let elapsed = now
        .saturating_duration_since(toast.created_at)
        .as_secs_f32();

    // Entrée en fondu + léger glissement vertical — repli sur ce qu'un painter bas niveau sait
    // exprimer facilement, `egui` n'ayant pas de transformation `scale` de calque comme
    // `@keyframes loot-pop` (CSS).
    let pop_t = (elapsed / POP_DURATION).min(1.0);
    let pop_eased = 1.0 - (1.0 - pop_t) * (1.0 - pop_t); // ease-out quadratique
    let card_alpha = pop_eased;
    let slide = (1.0 - pop_eased) * 10.0;

    let title = match toast.reason {
        WatchlistToastReason::Countdown => "COMPTEUR ÉPUISÉ !",
        WatchlistToastReason::Loot { .. } => "OBJET OBTENU !",
    };
    let name_text = match toast.reason {
        WatchlistToastReason::Loot { quantity } if quantity > 1 => {
            format!("{} × {quantity}", toast.name)
        }
        _ => toast.name.clone(),
    };

    // Police du design system, et non la proportionnelle par défaut d'egui — une Ubuntu *Light*,
    // plus maigre que tout ce que le jeu écrit. Les corps, eux, ne bougent pas : ce sont des cotes
    // du portage web, pas des mesures du client.
    let title_font = text::label_font(ui.ctx(), 11.0);
    let name_font = text::label_font(ui.ctx(), 14.0);
    let painter = ui.painter();
    let title_galley = painter.layout_no_wrap(title.to_string(), title_font, ACCENT);
    let name_galley = painter.layout_no_wrap(name_text, name_font, TEXT_BRIGHT);

    let text_width = title_galley.size().x.max(name_galley.size().x);
    let text_height = title_galley.size().y + CARD_TEXT_GAP + name_galley.size().y;
    let content_height = ICON_SIZE.max(text_height);
    let card_width = CARD_PAD_LEFT + ICON_SIZE + CARD_ICON_GAP + text_width + CARD_PAD_RIGHT;
    let card_height = content_height + 2.0 * CARD_PAD_V;

    let center_x = ui.max_rect().center().x;
    let confetti_top = ui.cursor().top() + slide;
    let card_top = confetti_top + CONFETTI_TOP_OVERSHOOT + CARD_TOP_GAP;
    let card_rect = egui::Rect::from_min_size(
        egui::pos2(center_x - card_width / 2.0, card_top),
        egui::vec2(card_width, card_height),
    );

    // Zone de clic AVANT la peinture, même motif que `entry_tile` — la carte ENTIÈRE ferme le
    // toast, pas seulement sa croix (demande utilisateur explicite).
    let card_response = ui
        .interact(
            card_rect,
            ui.id().with(("loot-alert-card", toast.name.as_str())),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    // --- Confettis (peints AVANT la carte pour rester visuellement derrière elle) ---
    let layer_left = center_x - TOAST_LAYER_WIDTH / 2.0;
    for piece in &toast.confetti {
        if elapsed < piece.delay {
            continue; // pas encore démarré, miroir de `animation-delay`
        }
        let local = (elapsed - piece.delay) % piece.duration; // boucle, miroir de `infinite`
        let t = (local / piece.duration).clamp(0.0, 1.0);
        let eased = t * t; // approximation de la temporisation CSS `ease-in`
        let alpha = (1.0 - eased) * card_alpha; // rejoint le fondu d'entrée de la carte
        if alpha <= 0.01 {
            continue;
        }
        let x = layer_left + piece.left_frac * TOAST_LAYER_WIDTH;
        let y = confetti_top + eased * CONFETTI_FALL_HEIGHT;
        let points = rotated_square(
            egui::pos2(x, y),
            CONFETTI_SIZE / 2.0,
            piece.rotation * eased,
        );
        ui.painter().add(egui::Shape::convex_polygon(
            points.to_vec(),
            with_alpha(piece.color, alpha),
            egui::Stroke::NONE,
        ));
    }

    // --- Carte ---
    let painter = ui.painter();
    painter.rect_filled(
        card_rect.translate(egui::vec2(0.0, 6.0)),
        CARD_ROUNDING,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, (90.0 * card_alpha) as u8),
    ); // ombre portée approximée (`box-shadow`) en une seule passe plutôt qu'un flou multi-passes
    painter.rect_filled(
        card_rect,
        CARD_ROUNDING,
        with_alpha(SURFACE_RAISED, card_alpha),
    );
    painter.rect_stroke(
        card_rect,
        CARD_ROUNDING,
        egui::Stroke::new(CARD_BORDER_WIDTH, with_alpha(ACCENT, card_alpha)),
        egui::StrokeKind::Inside,
    );

    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(
            card_rect.left() + CARD_PAD_LEFT + ICON_SIZE / 2.0,
            card_rect.center().y,
        ),
        egui::vec2(ICON_SIZE, ICON_SIZE),
    );
    let icon_ref = match toast.kind {
        WatchlistKind::Item => catalog.find_item_icon(&toast.name, toast.catalog_id),
        WatchlistKind::Enemy => catalog.find_monster_icon(&toast.name, toast.catalog_id),
    };
    let remote_texture = icon_ref
        .as_ref()
        .and_then(|icon_ref| remote_icon_textures.resolve(ui.ctx(), remote_icons, icon_ref));
    let icon_tint = egui::Color32::from_white_alpha((255.0 * card_alpha) as u8);
    match &remote_texture {
        Some(texture) => egui::Image::new(texture)
            .tint(icon_tint)
            .paint_at(ui, icon_rect),
        None => egui::Image::new(icons.unknown_entity_texture())
            .tint(icon_tint)
            .paint_at(ui, icon_rect),
    }

    let text_left = icon_rect.right() + CARD_ICON_GAP;
    let text_top = card_rect.center().y - text_height / 2.0;
    let painter = ui.painter();
    painter.galley(
        egui::pos2(text_left, text_top),
        title_galley.clone(),
        with_alpha(ACCENT, card_alpha),
    );
    painter.galley(
        egui::pos2(text_left, text_top + title_galley.size().y + CARD_TEXT_GAP),
        name_galley.clone(),
        with_alpha(TEXT_BRIGHT, card_alpha),
    );

    // --- Bouton de fermeture (coin haut-droit, miroir de `loot-alert-close`) ---
    let close_rect = egui::Rect::from_min_size(
        egui::pos2(
            card_rect.right() - CLOSE_BTN_MARGIN - CLOSE_BTN_SIZE,
            card_rect.top() + CLOSE_BTN_MARGIN,
        ),
        egui::vec2(CLOSE_BTN_SIZE, CLOSE_BTN_SIZE),
    );
    let close_response = ui
        .interact(
            close_rect,
            ui.id().with(("loot-alert-close", toast.name.as_str())),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let (close_bg, close_glyph) = if close_response.hovered() {
        (TINT_STRONG, TEXT_BRIGHT)
    } else {
        (TINT_MEDIUM, TEXT_MUTED)
    };
    let painter = ui.painter();
    painter.rect_filled(
        close_rect,
        CLOSE_BTN_ROUNDING,
        with_alpha(close_bg, card_alpha),
    );
    painter.text(
        close_rect.center(),
        egui::Align2::CENTER_CENTER,
        "×",
        text::label_font(ui.ctx(), 12.0),
        with_alpha(close_glyph, card_alpha),
    );
    // `design::tooltip` plutôt qu'un `on_hover_text` brut (refonte 2026-09-06, design
    // system tooltip) : même fond/texte/écart que le reste de l'UI, voir sa doc — pas de raison
    // qu'un tooltip ponctuel comme celui-ci reste sur le thème par défaut d'egui.
    design::tooltip(&close_response).text("Fermer");

    card_response.clicked() || close_response.clicked()
}

// Les deux enveloppes `show_tooltip_left` / `show_tooltip_right` ont été retirées le 2026-09-11 :
// leur placement et leurs replis sont désormais `design::tooltip` et `design::TooltipSide`, qui
// portent aussi la raison de cet ordre de replis. Ce qui reste ici est ce qui appartient à CE
// panneau : le carré de contrôle est collé au bord gauche de la fenêtre Suivi (quelques pixels de
// `WATCHLIST_INNER_MARGIN`), une infobulle strictement à gauche n'y tiendrait pas — d'où la réserve
// `CONTROL_TOOLTIP_RESERVE`, symétrique des deux côtés.

/// Côté d'infobulle d'un bouton du carré de contrôle — voir `control_button` et doc de module
/// (« infobulle par COLONNE, pas par bouton »). Un type dédié plutôt que `bool`/deux fonctions
/// séparées comme avant la refonte 2026-09-08 : `control_button` peignait jusque-là deux variantes
/// quasi identiques (`control_button`/`control_button_click`, distinguées par `Sense`) qui
/// codaient chacune EN DUR un côté de tooltip — devenu faux dès que "−"/"Options" (même colonne
/// DROITE, `Sense` différent) ont eu besoin du même côté DROITE l'un que l'autre.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TooltipSide {
    Left,
    Right,
}

/// Carré 2×2 de boutons du bandeau, sur un fond translucide (voir doc de module, refonte
/// 2026-09-08) : "+"/"−" en haut (INERTES, `Sense::hover()`), "Détails"/"Options" en dessous
/// (CLIQUABLES, `Sense::click()` — déplacés depuis `combat::bottom_toolbar`), tous peints par
/// `control_button` — même fond que `bottom_toolbar` utilisait déjà ([`PANEL_BACKDROP_FILL`]),
/// marge symétrique de `CONTROL_BUTTON_GAP` sur les quatre côtés ET entre
/// les deux colonnes/lignes.
///
/// Disposition (donnée explicitement par l'utilisateur, voir doc de module) :
/// ```text
/// [+] [−]
/// [🔗] [⚙]
/// ```
/// `watchlist_empty` désactive visuellement "−" (voir `control_button`, paramètre `enabled`) — rien
/// à supprimer tant qu'aucune entrée n'est suivie ; les trois autres boutons restent toujours
/// activés. Infobulle par COLONNE (voir `TooltipSide` et doc de module) : GAUCHE pour "+"/"Détails",
/// DROITE pour "−"/"Options" — jamais l'inverse du rôle inerte/cliquable du bouton, qui ne pilotait
/// le côté qu'AVANT cette refonte.
/// Renvoie les clics de CETTE frame — voir [`ControlRowClicks`] et la doc de `show`.
fn control_button_row(ui: &mut egui::Ui, watchlist_empty: bool) -> ControlRowClicks {
    let row_rect = ui
        .allocate_exact_size(
            egui::vec2(control_row_width(), control_row_height()),
            egui::Sense::hover(),
        )
        .0;
    ui.painter()
        .rect_filled(row_rect, PANEL_BACKDROP_ROUNDING, PANEL_BACKDROP_FILL);

    let add_top_left = row_rect.min + egui::vec2(CONTROL_BUTTON_GAP, CONTROL_BUTTON_GAP);
    control_button(
        ui,
        add_top_left,
        DsIcon::Plus,
        "watchlist-add",
        "Ajouter (Ctrl+Shift+A)",
        true,
        TooltipSide::Left,
    );

    // "−" à DROITE de "+" (pas en dessous, contrairement à l'ancienne disposition 1×2 — voir doc de
    // module) : désactivé tant que `watchlist_empty`, infobulle à DROITE (colonne droite).
    let remove_top_left = add_top_left + egui::vec2(CONTROL_BUTTON_SIZE + CONTROL_BUTTON_GAP, 0.0);
    control_button(
        ui,
        remove_top_left,
        DsIcon::Minus,
        "watchlist-remove",
        "Supprimer (Ctrl+Shift+S)",
        !watchlist_empty,
        TooltipSide::Right,
    );

    // "Détails" sous "+" (colonne GAUCHE) — même icône/action (ouvrir la web app) que l'ancien
    // bouton "lien externe" de `combat::bottom_toolbar`.
    let details_top_left = add_top_left + egui::vec2(0.0, CONTROL_BUTTON_SIZE + CONTROL_BUTTON_GAP);
    let details_response = control_button(
        ui,
        details_top_left,
        DsIcon::ExternalLink,
        "watchlist-details",
        "Détails (Ctrl+Shift+D)",
        true,
        TooltipSide::Left,
    );
    // Le clic est seulement REMONTÉ, jamais exécuté ici. Ce bouton appelait
    // `open::that(base_url())` directement, et c'était un vrai défaut : un panneau qui produit un
    // effet hors de l'écran devient impossible à peindre sans le produire. Les captures de
    // non-régression cliquent réellement dessus (`panels.rs::
    // panneau_suivi_clic_maintenu_repasse_en_mode_repos`, `Harness::drop_at`), donc chaque
    // `cargo test` ouvrait le navigateur de la personne qui lançait la suite — signalé par
    // l'utilisateur le 2026-09-09, après plusieurs ouvertures dans la journée.
    //
    // Un panneau peint et rend compte ; ouvrir une page appartient à l'hôte, comme le bouton
    // "Options" juste en dessous le faisait déjà.
    let details = details_response.clicked();

    // "Options" sous "−" (colonne DROITE) — même icône/action que l'ancien bouton de `combat::
    // bottom_toolbar`.
    let options_top_left =
        remove_top_left + egui::vec2(0.0, CONTROL_BUTTON_SIZE + CONTROL_BUTTON_GAP);
    let options_response = control_button(
        ui,
        options_top_left,
        DsIcon::Option,
        "watchlist-options",
        "Options (Ctrl+Shift+O)",
        true,
        TooltipSide::Right,
    );
    ControlRowClicks {
        options: options_response.clicked(),
        details,
    }
}

/// Ce que la rangée de contrôles a produit CETTE frame. Deux booléens plutôt qu'un, depuis que
/// "Détails" remonte lui aussi son clic au lieu d'ouvrir le navigateur lui-même.
#[derive(Debug, Clone, Copy, Default)]
struct ControlRowClicks {
    /// Le bouton "Options" vient d'être cliqué.
    options: bool,
    /// Le bouton "Détails" vient d'être cliqué — l'hôte ouvre la web app.
    details: bool,
}

/// Bouton du carré de contrôle — un [`design::icon_button`] posé dans un rect que ce panneau
/// calcule, plus le placement de son infobulle.
///
/// **Migration 2026-09-10 (décision utilisateur).** Ces quatre boutons passaient par
/// `panels::icon_button::paint_icon_button`, qui prenait quatre `egui::TextureHandle` en
/// paramètres : chaque appelant devait connaître et câbler ses textures. Ils nomment maintenant une
/// intention (`DsIcon::Plus`), et le manifeste résout les fichiers. Ce qui change à l'écran,
/// captures à l'appui (`overlay-testkit/tests/icon_button_migration.rs`) :
///
/// - les glyphes sont ceux du design system, calés sur l'étalon mesuré dans le jeu
///   (`tokens::ICON_BUTTON_CONTENT`) — le « − » redevient le trait fin du jeu au lieu d'une barre
///   pleine, les trois autres grossissent d'un dixième ;
/// - l'état désactivé peint `button-icon-disabled.png`, le socle grisé du jeu, au lieu d'assombrir
///   le socle de repos faute d'asset dédié à l'époque ;
/// - les socles de repos et de survol ne bougent pas d'un pixel : les fichiers d'`assets/ui/`
///   étaient déjà, octet pour octet, ceux du design system.
///
/// **Ce que le composant ne prend pas en charge, et pourquoi ça reste ici.** Le placement de
/// l'infobulle (`side`, voir [`TooltipSide`]) est une décision de mise en page : « + » et
/// « Détails » l'ouvrent à gauche, « − » et « Options » à droite, pour qu'elle ne recouvre jamais
/// l'autre colonne. `design::icon_button` expose bien un `.tooltip()`, mais il retombe sur le
/// placement par défaut d'egui — l'utiliser casserait cette règle, que
/// `panneau_suivi_tooltips_par_colonne_gauche_ou_droite` vérifie. Le composant peint et rend une
/// `Response` ; le panneau décide où poser l'infobulle.
///
/// Le paramètre `sense` disparaît en revanche : le composant sait qu'un bouton actif se clique et
/// qu'un bouton désactivé ne réagit qu'au survol. « + » et « − » restent INERTES (voir doc de
/// module) — c'est leur appelant qui ignore leur clic, pas leur `Sense` qui l'empêche. Cela ne
/// change rien à l'apparence : depuis le correctif du 2026-09-08, la règle « un appui retire le
/// survol » est écrite `response.hovered() && !pointer.any_down()`, identique pour les deux
/// `Sense`.
///
/// Curseur "main" au survol même pour "+"/"−" malgré leur inertie (retour utilisateur explicite
/// 2026-09-06) : porté par le composant, comme le curseur par défaut d'un bouton désactivé.
/// Renvoie la `Response` : le clic (pour "Détails"/"Options") est géré par l'appelant
/// (`control_button_row`), qui seul connaît l'action associée à chaque bouton.
fn control_button(
    ui: &mut egui::Ui,
    top_left: egui::Pos2,
    glyph: design::DsIcon,
    log_name: &str,
    tooltip: &str,
    enabled: bool,
    side: TooltipSide,
) -> egui::Response {
    let rect = egui::Rect::from_min_size(top_left, egui::Vec2::splat(CONTROL_BUTTON_SIZE));
    let response = ui.put(
        rect,
        design::icon_button(glyph)
            .context(IconContext::FirstPlan)
            .size(CONTROL_BUTTON_SIZE)
            .enabled(enabled)
            .log_name(log_name),
    );
    match side {
        TooltipSide::Left => design::tooltip(&response)
            .side(design::TooltipSide::Left)
            .text(tooltip),
        TooltipSide::Right => design::tooltip(&response)
            .side(design::TooltipSide::Right)
            .text(tooltip),
    }
    response
}

/// Tuile d'une entrée suivie : icône réelle si le catalogue la résout et qu'elle a fini de
/// télécharger (voir doc de module), repli générique sinon.
///
/// Deux styles de cadre selon `entry.kind` :
/// - OBJET : la texture `Border-<RARETÉ>.webp` (`UiIcons::item_border`) sert de FOND de la tuile,
///   peinte AVANT l'icône (voir doc de module, correctif same-day) — sa fenêtre intérieure n'est
///   pas un trou transparent mais un aplat teinté par la rareté, l'icône (opaque) peinte par-dessus
///   en recouvre l'essentiel, ne laissant dépasser que l'anneau de rareté et un mince liseré.
/// - ENNEMI : inchangé (fond plat + bordure grise unie, peinte après l'icône — un simple contour ne
///   craint pas cet ordre) — pas de rareté, pas d'asset dédié pour l'instant (demande utilisateur :
///   « pour les monstres, on verra ultérieurement »).
///
/// Compteur incrusté dans le coin bas-droit (voir `paint_count_inline`), nom complet en tooltip —
/// jamais tronqué silencieusement sans recours, y compris avec une icône réelle (contrairement au
/// web, dont l'image elle-même porte souvent assez d'info visuelle).
fn entry_tile(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    entry: &WatchlistEntry,
) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(TILE_SIZE, TILE_SIZE), egui::Sense::hover());

    // Fond sombre uni dans tous les cas — pour un ENNEMI, seul fond de la tuile (voir plus bas) ;
    // pour un OBJET, simple filet visible sous les coins arrondis de la texture de bordure peinte
    // juste après (celle-ci a ses propres coins arrondis avec un alpha dégradé, voir doc de
    // module), jamais sa couleur dominante.
    ui.painter().rect_filled(rect, TILE_ROUNDING, PANEL_BG);

    // OBJET seulement : fond de bordure peint ICI, AVANT l'icône (voir doc de la fonction) —
    // l'ordre inverse (bordure après icône) est le bug corrigé le jour même : la fenêtre
    // "intérieure" de cette texture n'est pas transparente, peinte après elle voilait l'icône
    // entière d'un aplat teinté par la rareté.
    if let WatchlistKind::Item = entry.kind {
        let rarity = catalog.find_item_rarity(&entry.name, entry.catalog_id);
        egui::Image::new(icons.item_border(rarity)).paint_at(ui, rect);
    }

    let icon_ref = match entry.kind {
        WatchlistKind::Item => catalog.find_item_icon(&entry.name, entry.catalog_id),
        WatchlistKind::Enemy => catalog.find_monster_icon(&entry.name, entry.catalog_id),
    };
    let remote_texture = icon_ref
        .as_ref()
        .and_then(|icon_ref| remote_icon_textures.resolve(ui.ctx(), remote_icons, icon_ref));

    // `paint_at` peint directement DANS le rect donné (ignore fit_to_exact_size/
    // maintain_aspect_ratio, qui ne s'appliquent qu'au layout via `ui.add`) — même motif que
    // `panels::combat::draw_centered_icon`, pas la peine de les poser ici.
    let icon_size = match entry.kind {
        WatchlistKind::Item => ITEM_ICON_SIZE,
        WatchlistKind::Enemy => ICON_SIZE,
    };
    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(icon_size, icon_size));
    match &remote_texture {
        Some(texture) => egui::Image::new(texture).paint_at(ui, icon_rect),
        None => egui::Image::new(icons.unknown_entity_texture()).paint_at(ui, icon_rect),
    }

    if let WatchlistKind::Enemy = entry.kind {
        ui.painter().rect_stroke(
            rect,
            TILE_ROUNDING,
            egui::Stroke::new(2.0, BORDER_STRONG),
            egui::StrokeKind::Inside,
        );
    }

    paint_count_inline(ui, rect, entry);

    // `design::tooltip` plutôt qu'un `on_hover_text` brut — voir sa doc (refonte
    // 2026-09-06, design system tooltip).
    design::tooltip(&response).text(&entry.name);
}

/// Compteur incrusté dans le coin bas-droit de la tuile — miroir des captures de référence du jeu
/// (`docs/design-system.md` §7, ex. `rare-items.png` : un simple nombre cerné de noir, PAS de
/// pastille/pilule de fond) : remplace l'ancien badge en pilule qui débordait hors de la tuile
/// (voir doc de module, refonte 2026-09-06). Réutilise `design::text::paint_outlined_text` (même procédé
/// que le pourcentage de dégâts sur un portrait) — un contour double (1px + 2px) essayé un temps en
/// même temps que `COUNT_FONT_SIZE` 15px a été jugé « trop » une fois comparé en jeu (retour
/// utilisateur, captures d'écran des deux côte à côte) : le contour simple à 1px, déjà validé
/// ailleurs dans l'UI, reste le bon réglage.
///
/// Couleur selon le mode (inchangé depuis la refonte 2026-09-02) :
/// - `up` : le compte seul, en clair neutre (`TEXT_COLOR`).
/// - `down` : compte courant EN COULEUR KAMAS (`KAMA_COLOR`) sur cible grisée (`TEXT_MUTED`),
///   PAS de conversion en "déjà collecté" (le web n'affiche que `count`/`countdownTarget` bruts).
///
/// **Essai 2026-09-06** (voir `TARGET_LINE_OFFSET`) : le nombre courant et la fraction cible ne
/// partagent plus la même ligne — la cible (ex. "/500") se peint sous le nombre courant, même
/// abscisse `right`, réduite à `TARGET_FONT_SIZE`.
///
/// **Refonte 2026-09-06 (bis)** (retour utilisateur explicite) : c'est maintenant la FRACTION qui
/// garde l'ancrage `bottom` du mode `up` (l'emplacement standard de TOUS les suivis incrémentaux),
/// et le nombre courant qui remonte de `TARGET_LINE_OFFSET` au-dessus — inversion du point de
/// référence par rapport à l'essai précédent, sur demande explicite (« remonter le groupe jusqu'à
/// ce que la fraction soit exactement à l'emplacement des suivis incrémentaux »). Couleur de la
/// fraction éclaircie à cette occasion (`TARGET_TEXT_COLOR`, plus clair que `TEXT_MUTED`).
fn paint_count_inline(ui: &egui::Ui, tile_rect: egui::Rect, entry: &WatchlistEntry) {
    let (current_text, target_part) = match entry.mode {
        WatchlistMode::Down => (
            entry.count.to_string(),
            Some(format!("/{}", entry.countdown_target)),
        ),
        WatchlistMode::Up => (entry.count.to_string(), None),
    };
    let current_color = if target_part.is_some() {
        KAMA_COLOR
    } else {
        TEXT_COLOR
    };

    let right = tile_rect.right() - COUNT_INSET_RIGHT;
    let bottom = tile_rect.bottom() - COUNT_INSET_BOTTOM;
    // Le groupe courant+fraction remonte de `TARGET_LINE_OFFSET` par rapport à l'ancien ancrage du
    // nombre courant (retour utilisateur 2026-09-06 : remonter tout le groupe jusqu'à ce que la
    // fraction retombe exactement à l'emplacement standard des suivis incrémentaux, c'est-à-dire
    // `bottom` — l'ancrage qu'utilise déjà le nombre courant seul en mode `up`, INCHANGÉ). Ce
    // décalage ne s'applique donc QUE quand une fraction est réellement affichée (mode `down`) —
    // le mode `up` garde exactement `bottom`, sans quoi « le même emplacement que tous les autres
    // nombres » (demande d'origine, voir doc de module) ne serait plus respecté.
    let current_bottom = if target_part.is_some() {
        bottom - TARGET_LINE_OFFSET
    } else {
        bottom
    };

    crate::design::text::paint_outlined_text(
        ui,
        egui::pos2(right, current_bottom),
        egui::Align2::RIGHT_BOTTOM,
        &current_text,
        egui::FontId::monospace(COUNT_FONT_SIZE),
        current_color,
        crate::design::text::OUTLINE_FULL,
    );

    if let Some(target_text) = &target_part {
        crate::design::text::paint_outlined_text(
            ui,
            egui::pos2(right, bottom),
            egui::Align2::RIGHT_BOTTOM,
            target_text,
            egui::FontId::monospace(TARGET_FONT_SIZE),
            TARGET_TEXT_COLOR,
            crate::design::text::OUTLINE_FULL,
        );
    }
}
