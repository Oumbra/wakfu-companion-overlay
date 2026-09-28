//! **Où se pose le bandeau Suivi sur la fenêtre de jeu** (2026-09-28) — son ancrage d'origine,
//! centré en haut du jeu, et la position que l'utilisateur lui donne par sa poignée (demande
//! utilisateur : « que l'utilisateur puisse déplacer le suivi comme il l'entend, n'importe où sur
//! l'écran, et que ce soit enregistré comme le récap »).
//!
//! Le pendant de `recap_placement` et de `click_through_placement`, avec deux différences qui
//! tiennent au bandeau lui-même :
//!
//! - **sa taille change** (une entrée de plus l'élargit, un toast l'agrandit, l'orientation la
//!   retourne) : l'ancrage d'origine se recalcule sur la taille du moment ([`default_offset`]), et
//!   le bornage aussi — un bandeau posé contre le bord droit qui gagne une tuile reste dans le jeu ;
//! - **vertical, il s'élargit au survol** ([`Expansion`]) : ses infobulles et son toast s'ouvrent
//!   sur le côté, dans une réserve que la fenêtre ne prend que le temps où la souris est sur le
//!   bandeau, pour ne pas laisser en permanence une bande transparente qui capte les clics du jeu.
//!
//! **Tout est en pixels PHYSIQUES**, comme `recap_placement` (voir sa doc) : l'appelant convertit
//! les tailles logiques du panneau, lui seul connaissant l'échelle d'affichage de sa fenêtre.
//!
//! **Le « bandeau » dont on parle ici est la fenêtre de BASE** — sans l'extension de survol. La
//! position persistée vise son coin haut-gauche, et l'extension se pose autour d'elle sans jamais
//! la déplacer ([`Expansion::window`]).

pub use crate::recap_placement::ClientArea;

/// Ordonnée du bandeau jamais déplacé, en pixels physiques sous le bord haut de la zone cliente
/// (`GameRect::client_top`) : la hauteur des boutons d'interface du jeu. Voir l'historique de
/// cette valeur dans `main.rs` (`GAME_TOP_MARGIN_PX`, deux allers-retours avec capture d'écran le
/// 2026-09-01) — elle vit ici depuis que les deux hôtes la partagent avec le déplacement.
pub const DEFAULT_TOP: i32 = 28;

/// **Aimantation** : reposé à moins de ce rayon de son ancrage d'origine, le bandeau y recolle et
/// la config oublie sa position (voir [`snap`]) — la valeur de `recap_placement::SNAP_RADIUS_PX`,
/// pour la même raison.
pub const SNAP_RADIUS_PX: i32 = crate::recap_placement::SNAP_RADIUS_PX;

/// La fenêtre de base du bandeau, en pixels physiques.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strip {
    pub width: i32,
    pub height: i32,
    /// La colonne de la poignée, à GAUCHE du bandeau horizontal (`panels::watchlist::
    /// CHROME_RESERVE`) — zéro en vertical, où la poignée est au-dessus. Elle ne compte pas
    /// dans le centrage : le bandeau jamais déplacé reste exactement où il était avant qu'elle
    /// n'existe, et la poignée vient se poser à sa gauche.
    pub chrome_left: i32,
}

/// Décalage du bandeau jamais déplacé, depuis le coin haut-gauche de la zone cliente : centré
/// horizontalement (poignée exclue, voir [`Strip::chrome_left`]), à [`DEFAULT_TOP`] du haut.
pub fn default_offset(client: ClientArea, strip: Strip) -> (i32, i32) {
    let band = strip.width - strip.chrome_left;
    ((client.width - band) / 2 - strip.chrome_left, DEFAULT_TOP)
}

/// Borne un décalage à la zone cliente du jeu : le bandeau reste entièrement dans le cadre, sa
/// poignée comprise — c'est elle qui permet de le ramener. Une zone cliente plus petite que lui
/// retombe sur 0 plutôt que sur une valeur négative.
pub fn clamp(offset: (i32, i32), client: ClientArea, strip: Strip) -> (i32, i32) {
    let max_x = (client.width - strip.width).max(0);
    let max_y = (client.height - strip.height).max(0);
    (offset.0.clamp(0, max_x), offset.1.clamp(0, max_y))
}

/// L'aimantation de la pose : `None` (« jamais déplacé », il suit l'ancrage) quand le bandeau
/// retombe à moins de [`SNAP_RADIUS_PX`] de son ancrage d'origine — calculé sur sa taille du
/// moment, puisque cet ancrage en dépend.
pub fn snap(offset: (i32, i32), client: ClientArea, strip: Strip) -> Option<(i32, i32)> {
    let origin = default_offset(client, strip);
    let (dx, dy) = ((offset.0 - origin.0).abs(), (offset.1 - origin.1).abs());
    (dx > SNAP_RADIUS_PX || dy > SNAP_RADIUS_PX).then_some(offset)
}

/// Position de la fenêtre de base, en pixels physiques d'écran — `None` étant « jamais
/// déplacé ». Bornée au passage : c'est le seul chemin par lequel les deux hôtes la placent.
pub fn window_position(offset: Option<(i32, i32)>, client: ClientArea, strip: Strip) -> (i32, i32) {
    let (x, y) = clamp(
        offset.unwrap_or_else(|| default_offset(client, strip)),
        client,
        strip,
    );
    (client.left + x, client.top + y)
}

/// Le décalage que vaut le curseur **à l'écran** pendant un glissement, pour un bandeau tenu par
/// le point `grab` (pixels physiques depuis le coin de sa fenêtre de base, figé au début du
/// geste). Le curseur d'écran et non celui d'egui — diagnostic dans `recap_placement::drag_offset`.
pub fn drag_offset(
    cursor: (i32, i32),
    grab: (i32, i32),
    client: ClientArea,
    strip: Strip,
) -> (i32, i32) {
    clamp(
        (
            cursor.0 - grab.0 - client.left,
            cursor.1 - grab.1 - client.top,
        ),
        client,
        strip,
    )
}

/// Le côté où le bandeau VERTICAL ouvre ses infobulles et son toast : **vers le centre du jeu**.
/// Collé à gauche, tout s'ouvre à droite ; collé à droite, à gauche — de l'autre côté, ils
/// sortiraient de l'écran de jeu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Side {
    Left,
    #[default]
    Right,
}

/// Voir [`Side`] : le centre du bandeau comparé à celui de la zone cliente.
pub fn side(base: (i32, i32), client: ClientArea, strip: Strip) -> Side {
    if base.0 + strip.width / 2 > client.left + client.width / 2 {
        Side::Left
    } else {
        Side::Right
    }
}

/// **L'extension de la fenêtre du bandeau vertical** : une réserve sur le côté ([`Side`]), le
/// temps du survol ou d'un toast, pour y ouvrir les infobulles et la carte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Expansion {
    pub side: Side,
    /// Largeur ajoutée sur ce côté, en pixels physiques.
    pub width: i32,
    /// Hauteur minimale de la fenêtre étendue — celle du toast quand il y en a un ; la fenêtre
    /// ne rétrécit jamais sous la hauteur de base.
    pub min_height: i32,
}

impl Expansion {
    /// Position et taille de la fenêtre étendue, en pixels physiques d'écran, à partir de la
    /// position de base (`window_position`). **La fenêtre de base ne bouge pas d'un pixel** :
    /// étendue à gauche, c'est l'origine de la fenêtre qui recule d'autant, et le contenu est
    /// décalé de la même quantité ([`Self::content_offset`]).
    pub fn window(self, base: (i32, i32), strip: Strip) -> ((i32, i32), (i32, i32)) {
        let size = (strip.width + self.width, strip.height.max(self.min_height));
        match self.side {
            Side::Right => (base, size),
            Side::Left => ((base.0 - self.width, base.1), size),
        }
    }

    /// Où commence la fenêtre de base dans la fenêtre étendue, en pixels physiques.
    pub fn content_offset(self) -> i32 {
        match self.side {
            Side::Right => 0,
            Side::Left => self.width,
        }
    }
}

/// Hauteur du bandeau HORIZONTAL sans toast ni sélection, en points — voir l'historique de cette
/// valeur dans `main.rs` (92 px de bande réellement occupée, plus la réserve des infobulles qui
/// s'ouvrent dessous). Elle vivait en double dans les deux hôtes jusqu'au 2026-09-28.
pub const HORIZONTAL_HEIGHT: f32 = 92.0 + crate::render_content::WATCHLIST_TOOLTIP_RESERVE;

/// Marge à ajouter à `panels::watchlist::content_width` pour obtenir la largeur de fenêtre du
/// bandeau horizontal — voir `main.rs`.
pub const HORIZONTAL_INNER_MARGIN: f32 = 12.0;

/// Plafond de largeur du bandeau horizontal, en fraction de la largeur du jeu et en valeur
/// absolue — au-delà, les tuiles défilent. Voir `main.rs`.
pub const WIDTH_FRACTION: f32 = 0.5;
pub const MAX_CEILING: f32 = 1000.0;

/// **Plafond de hauteur de la pile de tuiles du bandeau vertical**, en fraction de la hauteur du
/// jeu (2026-09-28) — au-delà, elle défile, comme la rangée du bandeau horizontal au-delà de la
/// moitié de la largeur. Un peu plus que la moitié : un jeu est plus large que haut, et une
/// colonne collée au bord gêne moins qu'une rangée collée en haut.
pub const HEIGHT_FRACTION: f32 = 0.6;

/// **Réserve de côté du bandeau vertical survolé**, en points (2026-09-28) : de quoi ouvrir une
/// infobulle de tuile (« nom de l'objet · Décompte ») ou du carré (« Supprimer (Ctrl+Shift+D) »,
/// 140 px) à côté de la colonne. Les noms d'objets les plus longs du catalogue tiennent sur une
/// ligne de ~250 px ; 280 laissent l'écart de l'infobulle et une garde.
pub const VERTICAL_TIP_RESERVE: f32 = 280.0;

/// Ce que le bandeau affiche cette frame, tel que l'hôte le sait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StripState {
    pub entry_count: usize,
    pub tracking_enabled: bool,
    pub toast_active: bool,
    pub select_open: bool,
    pub vertical: bool,
    /// La souris est sur le bandeau (fenêtre de base) — voir [`Expansion`]. Sans effet à
    /// l'horizontale.
    pub hovered: bool,
}

/// **Le gabarit complet de la fenêtre du bandeau**, calculé une fois pour les deux hôtes
/// (2026-09-28) : sa taille, sa position à l'écran, et ce que le rendu doit en savoir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    /// Taille de la fenêtre OS, en points (`request_inner_size`).
    pub size: (f64, f64),
    /// Position de la fenêtre OS, en pixels physiques d'écran.
    pub position: (i32, i32),
    /// La fenêtre de base, en pixels physiques — pour le geste et l'aimantation.
    pub strip: Strip,
    /// Position de la fenêtre de base à l'écran — le rectangle de survol du bandeau.
    pub base_position: (i32, i32),
    /// Côté d'ouverture du bandeau vertical.
    pub side: Side,
    /// La fenêtre de base dans la fenêtre OS, en points — `Some` seulement étendue
    /// (`render_content::RenderContent::watchlist_base`).
    pub base: Option<egui::Rect>,
}

/// Voir [`Plan`]. `offset` est la position voulue par l'utilisateur
/// (`config::OverlayConfig::watchlist_position`), `scale` l'échelle d'affichage de la fenêtre.
pub fn plan(state: StripState, offset: Option<(i32, i32)>, client: ClientArea, scale: f64) -> Plan {
    use crate::panels::watchlist;
    let physical = |points: f64| (points * scale).round() as i32;
    let base_size: (f64, f64) = if state.vertical {
        let max_tiles = (client.height as f64 / scale) as f32 * HEIGHT_FRACTION;
        let content = watchlist::vertical_content_size(
            state.entry_count,
            state.tracking_enabled,
            state.select_open,
            max_tiles,
        );
        (
            (content.x + crate::render_content::WATCHLIST_MARGIN_RIGHT) as f64,
            (content.y + crate::render_content::WATCHLIST_MARGIN_BOTTOM) as f64,
        )
    } else {
        let ceiling = (client.width as f32 * WIDTH_FRACTION).min(MAX_CEILING);
        let tiles = watchlist::content_width(state.entry_count, state.tracking_enabled);
        // La couche de confettis du toast est centrée sur le MÊME axe que la bande : sans cette
        // largeur minimale pendant qu'un toast est affiché, ses confettis les plus excentrés
        // seraient rognés par le bord de la fenêtre (voir `main.rs`).
        let toast = if state.toast_active {
            watchlist::TOAST_LAYER_WIDTH
        } else {
            0.0
        };
        let width = (tiles.max(toast) + HORIZONTAL_INNER_MARGIN)
            .min(ceiling)
            .max(HORIZONTAL_INNER_MARGIN);
        let height = HORIZONTAL_HEIGHT
            + if state.toast_active {
                watchlist::TOAST_AREA_HEIGHT
            } else {
                0.0
            }
            + if state.select_open {
                watchlist::SELECTION_BAR_HEIGHT
            } else {
                0.0
            };
        (width as f64, height as f64)
    };
    let strip = Strip {
        width: physical(base_size.0),
        height: physical(base_size.1),
        chrome_left: if state.vertical {
            0
        } else {
            physical(watchlist::CHROME_RESERVE as f64)
        },
    };
    let base_position = window_position(offset, client, strip);
    let side = side(base_position, client, strip);
    let expanded = state.vertical && (state.hovered || state.toast_active);
    if !expanded {
        return Plan {
            size: base_size,
            position: base_position,
            strip,
            base_position,
            side,
            base: None,
        };
    }
    let reserve = if state.toast_active {
        VERTICAL_TIP_RESERVE.max(watchlist::TOAST_LAYER_WIDTH)
    } else {
        VERTICAL_TIP_RESERVE
    } as f64;
    let min_height = if state.toast_active {
        watchlist::TOAST_AREA_HEIGHT as f64
    } else {
        0.0
    };
    let expansion = Expansion {
        side,
        width: physical(reserve),
        min_height: physical(min_height),
    };
    let (position, _) = expansion.window(base_position, strip);
    let left = match side {
        Side::Left => reserve as f32,
        Side::Right => 0.0,
    };
    Plan {
        size: (base_size.0 + reserve, base_size.1.max(min_height)),
        position,
        strip,
        base_position,
        side,
        base: Some(egui::Rect::from_min_size(
            egui::pos2(left, 0.0),
            egui::vec2(base_size.0 as f32, base_size.1 as f32),
        )),
    }
}

/// Le curseur d'écran est-il sur le bandeau (sa fenêtre de base) ? — la condition de l'extension
/// du bandeau vertical, décidée par l'hôte sur le curseur d'ÉCRAN et non sur celui d'egui : ce
/// dernier se mesure depuis le coin de la fenêtre, qui recule quand elle s'étend à gauche, et
/// resterait faux d'une réserve entière jusqu'au prochain mouvement de souris.
pub fn hovers(cursor: (i32, i32), plan: &Plan) -> bool {
    let (x, y) = plan.base_position;
    (x..x + plan.strip.width).contains(&cursor.0) && (y..y + plan.strip.height).contains(&cursor.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une fenêtre de jeu 1920×1080 posée en (100, 50), zone cliente dès son bord haut.
    const CLIENT: ClientArea = ClientArea {
        left: 100,
        top: 50,
        width: 1920,
        height: 1080,
    };
    /// Un bandeau horizontal de 400 px, dont 26 de poignée.
    const STRIP: Strip = Strip {
        width: 400,
        height: 120,
        chrome_left: 26,
    };

    /// Jamais déplacé, le bandeau reste où il était avant la poignée : centré sur le jeu, la
    /// poignée posée à sa gauche.
    #[test]
    fn jamais_deplace_le_bandeau_reste_centre_en_haut() {
        let (x, y) = window_position(None, CLIENT, STRIP);
        let band_left = x + STRIP.chrome_left;
        let band_width = STRIP.width - STRIP.chrome_left;
        assert_eq!(band_left - CLIENT.left, (CLIENT.width - band_width) / 2);
        assert_eq!(y, CLIENT.top + DEFAULT_TOP);
    }

    #[test]
    fn une_position_choisie_est_respectee_et_bornee() {
        assert_eq!(
            window_position(Some((10, 500)), CLIENT, STRIP),
            (CLIENT.left + 10, CLIENT.top + 500)
        );
        assert_eq!(clamp((-40, -3), CLIENT, STRIP), (0, 0));
        assert_eq!(clamp((5000, 5000), CLIENT, STRIP), (1520, 960));
    }

    /// Le bandeau qui grandit contre le bord droit reste dans le jeu : le bornage se fait sur sa
    /// taille du moment.
    #[test]
    fn un_bandeau_qui_grandit_contre_le_bord_reste_dans_le_jeu() {
        let collé = Some((1520, 300));
        let plus_large = Strip {
            width: 600,
            ..STRIP
        };
        let (x, _) = window_position(collé, CLIENT, plus_large);
        assert_eq!(x + plus_large.width, CLIENT.left + CLIENT.width);
    }

    #[test]
    fn repose_pres_de_l_ancrage_il_y_recolle() {
        let origin = default_offset(CLIENT, STRIP);
        assert_eq!(snap((origin.0 + 12, origin.1 - 12), CLIENT, STRIP), None);
        assert_eq!(
            snap((origin.0 + 13, origin.1), CLIENT, STRIP),
            Some((origin.0 + 13, origin.1))
        );
    }

    #[test]
    fn le_point_de_saisie_reste_sous_le_curseur() {
        let grab = (8, 30);
        let offset = drag_offset((700, 400), grab, CLIENT, STRIP);
        let (x, y) = window_position(Some(offset), CLIENT, STRIP);
        assert_eq!((x + grab.0, y + grab.1), (700, 400));
    }

    /// Vertical, tout s'ouvre vers le centre du jeu.
    #[test]
    fn les_infobulles_du_bandeau_vertical_s_ouvrent_vers_le_centre() {
        let colonne = Strip {
            width: 90,
            height: 400,
            chrome_left: 0,
        };
        assert_eq!(side((CLIENT.left, 300), CLIENT, colonne), Side::Right);
        assert_eq!(
            side((CLIENT.left + CLIENT.width - 90, 300), CLIENT, colonne),
            Side::Left
        );
    }

    /// Étendue à gauche, la fenêtre recule et le contenu avance d'autant : la colonne ne bouge
    /// pas d'un pixel à l'écran.
    #[test]
    fn l_extension_ne_deplace_jamais_la_colonne() {
        let colonne = Strip {
            width: 90,
            height: 400,
            chrome_left: 0,
        };
        let base = (1500, 300);
        for side in [Side::Left, Side::Right] {
            let extension = Expansion {
                side,
                width: 280,
                min_height: 500,
            };
            let ((x, y), (w, h)) = extension.window(base, colonne);
            assert_eq!((x + extension.content_offset(), y), base);
            assert_eq!((w, h), (370, 500));
        }
    }

    /// Le gabarit horizontal garde la hauteur et la largeur d'avant, plus la colonne de la
    /// poignée ; le vertical s'étend au survol sans déplacer la colonne.
    #[test]
    fn le_gabarit_du_bandeau_suit_son_orientation_et_le_survol() {
        let etat = StripState {
            entry_count: 3,
            tracking_enabled: true,
            ..Default::default()
        };
        let couche = plan(etat, None, CLIENT, 1.0);
        assert_eq!(couche.size.1, HORIZONTAL_HEIGHT as f64);
        assert!(couche.base.is_none());

        let dresse = StripState {
            vertical: true,
            ..etat
        };
        let repos = plan(dresse, Some((1700, 200)), CLIENT, 1.0);
        assert!(repos.base.is_none());
        let survol = plan(
            StripState {
                hovered: true,
                ..dresse
            },
            Some((1700, 200)),
            CLIENT,
            1.0,
        );
        assert_eq!(survol.side, Side::Left);
        assert_eq!(survol.base_position, repos.base_position);
        assert_eq!(
            survol.position.0 + VERTICAL_TIP_RESERVE as i32,
            repos.position.0
        );
        assert_eq!(survol.size.0, repos.size.0 + VERTICAL_TIP_RESERVE as f64);
        assert!(hovers(repos.base_position, &repos));
    }
}
