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
//!   dynamique, voir sa doc). Infobulle à GAUCHE désormais (`show_tooltip_left`), pas au-dessus
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
//!   `combat::paint_outlined_text`, réutilisé ici), incrusté DANS le coin bas-droit de la tuile —
//!   voir `paint_count_inline`. Plus aucun débordement hors tuile : `content_width` n'a donc plus
//!   besoin de réserver `BADGE_OVERFLOW` (retiré).

use overlay_engine::{CatalogIndex, WatchlistEntry, WatchlistKind, WatchlistMode};

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

/// Taille (largeur ET hauteur) des boutons "+"/"−" du bandeau (`UiIcons::watchlist_add`/
/// `watchlist_remove`) — taille NATIVE de l'asset (34×34), non redimensionnée : contrairement au
/// socle générique `button_background` (voir `combat::paint_icon_button`), l'utilisateur a fourni
/// ces icônes directement à la taille voulue.
const CONTROL_BUTTON_SIZE: f32 = 34.0;
/// Écart vertical entre le bouton "+" et le bouton "−", empilés (voir doc de module, refonte
/// 2026-09-06) — volontairement plus serré que `TILE_GAP` (les deux boutons forment un seul groupe
/// visuel "ajouter/supprimer", pas deux entrées indépendantes).
const CONTROL_BUTTON_GAP: f32 = 4.0;
/// Espace réservé à GAUCHE de la colonne de contrôle, pour que l'infobulle de `control_button`
/// (`show_tooltip_left`) ait matériellement la place de s'afficher à gauche — retour utilisateur
/// 2026-09-06, capture d'écran à l'appui : sans cette réserve, l'infobulle s'affichait à DROITE du
/// bouton (chevauchant la première tuile) malgré `RectAlign::LEFT` demandé, car la fenêtre Suivi
/// est dimensionnée pile sur son contenu (`content_width`) — la colonne de contrôle est le tout
/// premier élément, collé au bord gauche de la fenêtre à quelques pixels de marge près
/// (`WATCHLIST_INNER_MARGIN`, `main.rs`) : `RectAlign::find_best_align` (voir sa doc,
/// `combat::show_tooltip_above`) rejette alors LEFT/LEFT_START/LEFT_END, aucun n'y tenant, et
/// retombe sur RIGHT — PHYSIQUEMENT, une popup ne peut pas se peindre en dehors de la fenêtre qui
/// la contient (contrairement à `combat::show_tooltip_above`, où le problème ne concernait qu'un
/// AXE d'alignement, ici il n'existe aucun repli qui n'exige pas de place à gauche).
///
/// Valeur mesurée (pas devinée) : rendu offscreen du bouton survolé sur un canevas large (sans
/// contrainte de bord), diff pixel par pixel avec le même rendu non survolé — l'infobulle
/// "Supprimer" (le plus long des deux libellés) occupe alors ~74px de large avec ~4px d'écart
/// depuis le bord du bouton, soit ~78px de pied total ; arrondi à 88px pour absorber marge
/// d'erreur de mesure/anticrénelage.
///
/// **Contrepartie assumée** : `content_width` (donc la largeur de FENÊTRE) grandit d'autant, et la
/// fenêtre Suivi étant centrée horizontalement sur cette largeur (`main.rs::anchor_position`), la
/// bande de tuiles visible se retrouve décalée d'environ la moitié de cette réserve (~44px) à
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

/// Marge entre le texte du compteur (voir `paint_count_inline`) et le bord de la tuile — assez
/// pour rester lisible par-dessus le cadre de rareté (dont le liseré occupe déjà quelques pixels,
/// voir `ITEM_BORDER_INNER_MARGIN_RATIO`) sans empiéter dessus.
const COUNT_INSET: f32 = 5.0;

/// Taille du compteur (voir `paint_count_inline`) — retour utilisateur 2026-09-06, capture d'écran
/// du jeu à l'appui (quantités d'objets en bas-droit d'un emplacement) : à 11px le nombre était
/// « pas du tout lisible » par comparaison. Un premier essai à 15px s'est avéré « beaucoup trop
/// élevé » une fois comparé en jeu (second retour, captures d'écran des deux côte à côte) — ramené
/// à 13px, entre les deux.
const COUNT_FONT_SIZE: f32 = 13.0;

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
pub fn content_width(entry_count: usize) -> f32 {
    let entries_width = if entry_count == 0 {
        0.0
    } else {
        entry_count as f32 * TILE_SIZE + (entry_count as f32 - 1.0) * TILE_GAP
    };
    CONTROL_TOOLTIP_RESERVE + CONTROL_BUTTON_SIZE + TILE_GAP + entries_width
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
/// `--text-color` — texte neutre (badge en mode `up`).
const TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(0xe0, 0xe0, 0xe0);
/// `--kama-color` — valeur COURANTE d'un décompte (`.kpi-count-badge.is-fraction`).
const KAMA_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 215, 0);

/// `--accent` — bordure ET titre du toast (`loot-alert-card`/`loot-alert-title`,
/// `loot-alert.component.css`), les deux réutilisent le même jeton quel que soit `reason`.
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x00, 0xd2, 0xff);
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
pub fn show(
    ui: &mut egui::Ui,
    assets: WatchlistAssets<'_>,
    entries: &[WatchlistEntry],
    toast: Option<&WatchlistToast>,
    now: std::time::Instant,
) -> bool {
    let WatchlistAssets {
        icons,
        catalog,
        remote_icons,
        remote_icon_textures,
    } = assets;
    // Bande de tuiles absente tant que le compte ne déclare aucune entrée suivie (voir
    // `main.rs::render`, commentaire de `OverlayKind::Watchlist`) — un ramassage à son activé
    // (`overlay_engine::profile`, INDÉPENDANT de la watchlist) doit pouvoir déclencher un toast
    // même dans ce cas, d'où la garde ici plutôt qu'en amont.
    if !entries.is_empty() {
        let mut style = (**ui.style()).clone();
        style_thin_scrollbar(&mut style);
        ui.set_style(style);

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

                    // Colonne "+"/"−" empilée verticalement (voir doc de module, refonte
                    // 2026-09-06) — `ui.horizontal` centre ses enfants verticalement par défaut,
                    // ce qui aligne naturellement cette colonne (34+4+34=72px) sur le centre des
                    // tuiles d'entrée (58px) juste à côté.
                    ui.vertical(|ui| {
                        control_button(
                            ui,
                            icons.watchlist_add(),
                            icons.watchlist_add_hover(),
                            "Ajouter",
                        );
                        ui.add_space(CONTROL_BUTTON_GAP);
                        control_button(
                            ui,
                            icons.watchlist_remove(),
                            icons.watchlist_remove_hover(),
                            "Supprimer",
                        );
                    });
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
    }

    match toast.filter(|t| t.hide_at > now) {
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
    }
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

    let painter = ui.painter();
    let title_galley =
        painter.layout_no_wrap(title.to_string(), egui::FontId::proportional(11.0), ACCENT);
    let name_galley =
        painter.layout_no_wrap(name_text, egui::FontId::proportional(14.0), TEXT_BRIGHT);

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
        egui::FontId::proportional(12.0),
        with_alpha(close_glyph, card_alpha),
    );
    let close_response = close_response.on_hover_text("Fermer");

    card_response.clicked() || close_response.clicked()
}

/// Affiche `text` en infobulle à GAUCHE de `response` (`RectAlign::LEFT`) — demande utilisateur
/// explicite pour les boutons "+"/"−" du bandeau Suivi (voir doc de module, refonte 2026-09-06),
/// contrairement au reste de l'UI qui affiche ses tooltips AU-DESSUS (`combat::show_tooltip_above`,
/// voir sa doc pour le mécanisme de repli sur lequel celle-ci est calquée).
///
/// Ces deux boutons sont les tout premiers éléments du bandeau, collés au bord GAUCHE de la
/// fenêtre Suivi (seulement `WATCHLIST_INNER_MARGIN` de marge, `main.rs` — quelques pixels) : une
/// tooltip strictement à gauche ("Ajouter"/"Supprimer", bien plus large que cette marge) ne peut
/// structurellement pas y tenir. `align_alternatives` couvre ce repli exactement comme
/// `show_tooltip_above` le fait pour le bord opposé (même bug déjà rencontré et corrigé côté
/// Combat, voir sa doc) : `LEFT_START`/`LEFT_END` d'abord (repli aligné au lieu de centré, qui ne
/// déborde plus que du côté opposé au bord), `RIGHT*` en tout dernier recours plutôt que de laisser
/// egui retomber sur son défaut `BOTTOM_START` (« sous la souris », déjà jugé désagréable ailleurs).
fn show_tooltip_left(response: &egui::Response, text: &str) {
    let mut tooltip = egui::Tooltip::for_enabled(response);
    tooltip.popup = tooltip
        .popup
        .align(egui::RectAlign::LEFT)
        .align_alternatives(&[
            egui::RectAlign::LEFT_START,
            egui::RectAlign::LEFT_END,
            egui::RectAlign::RIGHT,
            egui::RectAlign::RIGHT_START,
            egui::RectAlign::RIGHT_END,
        ]);
    tooltip.show(|ui| {
        ui.set_max_width(ui.spacing().tooltip_width);
        ui.label(text);
    });
}

/// Bouton "+"/"−" du bandeau — icône du jeu à sa taille NATIVE (`CONTROL_BUTTON_SIZE`, voir sa
/// doc), fond et glyphe déjà intégrés à l'asset (contrairement à `combat::paint_icon_button`, pas
/// de socle séparé à composer). `Sense::hover()` seulement, PAS `click()` : ces deux boutons
/// restent INERTES (voir doc de module) — un survol suffit à afficher l'infobulle explicative
/// (`show_tooltip_left`), sans laisser croire qu'un clic ferait quoi que ce soit.
///
/// Curseur "main" affiché au survol malgré cette inertie (retour utilisateur explicite
/// 2026-09-06) : affordance visuelle demandée en plus de l'infobulle, en assumant que le risque
/// d'ambiguïté déjà discuté (voir doc de module) reste acceptable ici tant que le câblage réel
/// n'existe pas.
fn control_button(
    ui: &mut egui::Ui,
    icon: &egui::TextureHandle,
    icon_hover: &egui::TextureHandle,
    tooltip: &str,
) {
    let (rect, response) = ui.allocate_exact_size(
        egui::Vec2::splat(CONTROL_BUTTON_SIZE),
        egui::Sense::hover(),
    );
    // Curseur "main" malgré l'inertie du bouton (retour utilisateur explicite 2026-09-06,
    // au-dessus des réserves de `control_button` : affordance visuelle demandée même sans clic
    // câblé) — cohérent avec le réglage global `Visuals::interact_cursor` (`main.rs`) qui ne
    // couvre pas les éléments dessinés à la main comme celui-ci.
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let texture = if response.hovered() { icon_hover } else { icon };
    egui::Image::new(texture).paint_at(ui, rect);
    show_tooltip_left(&response, tooltip);
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

    response.on_hover_text(&entry.name);
}

/// Compteur incrusté dans le coin bas-droit de la tuile — miroir des captures de référence du jeu
/// (`docs/design-system.md` §7, ex. `rare-items.png` : un simple nombre cerné de noir, PAS de
/// pastille/pilule de fond) : remplace l'ancien badge en pilule qui débordait hors de la tuile
/// (voir doc de module, refonte 2026-09-06). Réutilise `combat::paint_outlined_text` (même procédé
/// que le pourcentage de dégâts sur un portrait) — un contour double (1px + 2px) essayé un temps en
/// même temps que `COUNT_FONT_SIZE` 15px a été jugé « trop » une fois comparé en jeu (retour
/// utilisateur, captures d'écran des deux côte à côte) : le contour simple à 1px, déjà validé
/// ailleurs dans l'UI, reste le bon réglage.
///
/// Couleur selon le mode (inchangé depuis la refonte 2026-09-02) :
/// - `up` : le compte seul, en clair neutre (`TEXT_COLOR`).
/// - `down` : compte courant EN COULEUR KAMAS (`KAMA_COLOR`) sur cible grisée (`TEXT_MUTED`),
///   PAS de conversion en "déjà collecté" (le web n'affiche que `count`/`countdownTarget` bruts).
fn paint_count_inline(ui: &egui::Ui, tile_rect: egui::Rect, entry: &WatchlistEntry) {
    let font = egui::FontId::monospace(COUNT_FONT_SIZE);

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

    let right = tile_rect.right() - COUNT_INSET;
    let bottom = tile_rect.bottom() - COUNT_INSET;

    // Le segment "cible" (ex. "/10") est peint EN PREMIER, ancré au coin bas-droit de la tuile ;
    // le segment "courant" est ensuite peint juste à sa GAUCHE (ancré `RIGHT_BOTTOM` sur la
    // largeur mesurée du segment cible) — évite de dupliquer la boucle de contour de
    // `paint_outlined_text` pour composer deux galleys sur une même ligne.
    let target_width = target_part
        .as_ref()
        .map(|text| {
            ui.painter()
                .layout_no_wrap(text.clone(), font.clone(), TEXT_MUTED)
                .size()
                .x
        })
        .unwrap_or(0.0);

    if let Some(target_text) = &target_part {
        super::combat::paint_outlined_text(
            ui,
            egui::pos2(right, bottom),
            egui::Align2::RIGHT_BOTTOM,
            target_text,
            font.clone(),
            TEXT_MUTED,
        );
    }
    super::combat::paint_outlined_text(
        ui,
        egui::pos2(right - target_width, bottom),
        egui::Align2::RIGHT_BOTTOM,
        &current_text,
        font,
        current_color,
    );
}
