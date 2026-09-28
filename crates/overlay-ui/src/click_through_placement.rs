//! **Où se pose le bouton œil sur la fenêtre de jeu** (2026-09-28) — son ancrage d'origine, juste
//! après le bouton Boutique du jeu, et la position que l'utilisateur lui donne au glisser-déposer
//! (demande utilisateur : « permets à l'utilisateur de placer ce bouton où il le souhaite »).
//!
//! Le pendant de `recap_placement` pour une fenêtre sans réserve d'infobulle : la fenêtre OS est
//! exactement le bouton, le décalage persisté vise donc directement son coin haut-gauche. Les deux
//! garde-fous sont les mêmes que pour la bande Récap : le **bornage** à la zone cliente (un bouton
//! poussé hors de l'écran ne pourrait plus être rattrapé, et c'est lui qui rend l'overlay
//! interactif) et l'**aimantation** au retour près de l'ancrage d'origine.
//!
//! **Tout est en pixels PHYSIQUES**, comme `recap_placement` (voir sa doc) : seule la taille du
//! bouton naît en points logiques (`panels::click_through::SECTION_SIZE`), et l'appelant la
//! convertit, lui seul connaissant l'échelle d'affichage de sa fenêtre.
//!
//! **Le « bouton » dont on parle ici est la fenêtre AU REPOS** — le fond de section du jeu et son
//! socle. Au survol, la fenêtre s'étend pour l'infobulle ([`tip_window`]) sans que le bouton
//! bouge ; la position persistée vise toujours le coin de la section.

pub use crate::recap_placement::ClientArea;

/// **Écart entre deux sections de boutons du jeu** (demande utilisateur 2026-09-28, capture du jeu
/// avec l'overlay à l'appui : « il est trop près du bouton cadeau [...] j'aimerais qu'il ait le
/// même espace qu'entre la section Boutique et le bouton cadeau »).
///
/// Mesuré sur cette capture (écran 2560 px réduit à 2000, échelle recalée sur le bouton œil
/// lui-même, posé à x = 212) : la section Menu + Boutique finit à x ≈ 163, le cadre du cadeau
/// commence à x ≈ 171 — **8 px** de jeu entre deux sections. Ce fut 2 px jusqu'ici, l'écart entre
/// deux SOCLES d'une même colonne (`menu-button-icon-first-plan.png`, socles de 36 px tous les
/// 38 px) : c'est l'écart à l'intérieur d'une section, pas entre deux.
pub const GAME_SECTION_GAP: i32 = 8;

/// Bord droit du cadre du cadeau, dernier bouton de la rangée du jeu, en pixels physiques depuis
/// le bord gauche de la zone cliente — relevé sur la même capture (x ≈ 210, confirmé par le bloc
/// Récap, 206 px posés à x = 4, « qui finit au bord droit du bouton Boutique »).
const GIFT_RIGHT: i32 = 210;

/// Décalage du bouton tant que l'utilisateur ne l'a pas déplacé, en pixels physiques depuis le
/// coin haut-gauche de la zone cliente du jeu (`GameRect::left`, `GameRect::client_top`).
///
/// - En ordonnée, 31 : le haut du CADRE du cadeau sur la capture du 2026-09-28 (y ≈ 31 à 72) —
///   le bouton porte désormais le même cadre (`panels::click_through::SECTION_SIZE`), c'est lui
///   qui s'aligne, plus le socle seul (qui était à 32).
/// - En abscisse, 218 : le cadeau finit à x = 210, plus [`GAME_SECTION_GAP`].
pub const DEFAULT_OFFSET: (i32, i32) = (GIFT_RIGHT + GAME_SECTION_GAP, 31);

/// **Aimantation** : reposé à moins de ce rayon de son ancrage d'origine, le bouton y recolle et
/// la config oublie sa position (voir [`snap`]). La valeur de `recap_placement::SNAP_RADIUS_PX`,
/// pour la même raison : un retour « à peu près à sa place » doit rendre la place exacte, à côté
/// des boutons du jeu où trois pixels d'écart se verraient.
pub const SNAP_RADIUS_PX: i32 = crate::recap_placement::SNAP_RADIUS_PX;

/// Borne un décalage à la zone cliente du jeu : le bouton reste entièrement dans le cadre.
///
/// `size` est la taille du bouton au repos en pixels physiques. Une zone cliente plus petite que
/// lui retombe sur 0 plutôt que sur une valeur négative — le coin de saisie reste attrapable.
pub fn clamp(offset: (i32, i32), client: ClientArea, size: (i32, i32)) -> (i32, i32) {
    let max_x = (client.width - size.0).max(0);
    let max_y = (client.height - size.1).max(0);
    (offset.0.clamp(0, max_x), offset.1.clamp(0, max_y))
}

/// L'aimantation de la pose : `None` (« jamais déplacé », il suit l'ancrage) quand le bouton
/// retombe à moins de [`SNAP_RADIUS_PX`] de [`DEFAULT_OFFSET`] — voir `recap_placement::snap`
/// pour pourquoi `None` plutôt qu'une position figée.
pub fn snap(offset: (i32, i32)) -> Option<(i32, i32)> {
    let (dx, dy) = (
        (offset.0 - DEFAULT_OFFSET.0).abs(),
        (offset.1 - DEFAULT_OFFSET.1).abs(),
    );
    (dx > SNAP_RADIUS_PX || dy > SNAP_RADIUS_PX).then_some(offset)
}

/// Position de la fenêtre OS du bouton, en pixels physiques d'écran — `None` étant « jamais
/// déplacé ». Bornée au passage : c'est le seul chemin par lequel les deux hôtes la placent.
pub fn window_position(
    offset: Option<(i32, i32)>,
    client: ClientArea,
    size: (i32, i32),
) -> (i32, i32) {
    let (x, y) = clamp(offset.unwrap_or(DEFAULT_OFFSET), client, size);
    (client.left + x, client.top + y)
}

/// **La fenêtre étendue le temps du survol**, pour l'infobulle (voir `panels::click_through`) :
/// sa position et sa taille à l'écran, où le bouton se trouve dedans, et si l'infobulle s'ouvre
/// au-dessus. Tout en pixels physiques ; `base` est la position au repos ([`window_position`]),
/// `size` la taille au repos, `reserve` la place de l'infobulle (largeur totale, hauteur ajoutée).
///
/// **Le bouton ne bouge pas d'un pixel à l'écran**, et l'infobulle se lit centrée sur lui
/// (demande utilisateur 2026-09-28) :
///
/// - **En largeur**, la réserve est une bande centrée sur le bouton, **bornée à la zone cliente**
///   du jeu : contre le bord gauche, elle commence au bord et l'infobulle glisse vers la droite ;
///   contre le bord droit, l'inverse. L'infobulle, centrée sur le bouton puis retenue dans la
///   fenêtre (`design::Tooltip::slide`), ne dépasse donc jamais du jeu.
/// - **En hauteur**, dessous par défaut ; au-dessus seulement quand le bouton est posé si bas que
///   la réserve ne tient plus dessous — et qu'elle tient dessus. Un jeu trop petit pour l'un
///   comme l'autre garde le dessous.
pub fn tip_window(
    base: (i32, i32),
    size: (i32, i32),
    reserve: (i32, i32),
    client: ClientArea,
) -> TipWindow {
    let width = reserve.0.max(size.0);
    let client_right = client.left + client.width;
    let centre = base.0 + size.0 / 2;
    let band_left =
        (centre - width / 2).clamp(client.left, (client_right - width).max(client.left));
    let left = band_left.min(base.0);
    let right = (band_left + width).max(base.0 + size.0);
    let below_fits = base.1 + size.1 + reserve.1 <= client.top + client.height;
    let above_fits = base.1 - reserve.1 >= client.top;
    let above = !below_fits && above_fits;
    let top = if above { base.1 - reserve.1 } else { base.1 };
    TipWindow {
        position: (left, top),
        size: (right - left, size.1 + reserve.1),
        button_origin: (base.0 - left, base.1 - top),
        above,
    }
}

/// Voir [`tip_window`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TipWindow {
    pub position: (i32, i32),
    pub size: (i32, i32),
    /// Le coin du bouton dans la fenêtre étendue.
    pub button_origin: (i32, i32),
    pub above: bool,
}

/// Le décalage que vaut le curseur **à l'écran** pendant un glissement, pour un bouton tenu par
/// le point `grab` (pixels physiques depuis le coin de sa fenêtre, figé au début du geste).
///
/// Le curseur d'écran et non celui d'egui : ce dernier se mesure depuis le coin de la fenêtre
/// qu'on déplace, et le suivre fait vibrer la fenêtre — diagnostic complet dans
/// `recap_placement::drag_offset`.
pub fn drag_offset(
    cursor: (i32, i32),
    grab: (i32, i32),
    client: ClientArea,
    size: (i32, i32),
) -> (i32, i32) {
    clamp(
        (
            cursor.0 - grab.0 - client.left,
            cursor.1 - grab.1 - client.top,
        ),
        client,
        size,
    )
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
    const SIDE: (i32, i32) = (40, 42);

    #[test]
    fn jamais_deplace_il_suit_l_ancrage_d_origine() {
        assert_eq!(window_position(None, CLIENT, SIDE), (100 + 218, 50 + 31));
    }

    /// Au survol, la fenêtre s'étend sans jamais déplacer le bouton à l'écran.
    fn button_on_screen(tip: &TipWindow) -> (i32, i32) {
        (
            tip.position.0 + tip.button_origin.0,
            tip.position.1 + tip.button_origin.1,
        )
    }

    const RESERVE: (i32, i32) = (240, 40);

    /// Par défaut, la réserve est centrée sous le bouton.
    #[test]
    fn l_infobulle_s_ouvre_centree_dessous() {
        let base = window_position(None, CLIENT, SIDE);
        let tip = tip_window(base, SIDE, RESERVE, CLIENT);
        assert_eq!(button_on_screen(&tip), base);
        assert!(!tip.above);
        assert_eq!(tip.size, (240, 82));
        assert_eq!(tip.position.0 + tip.size.0 / 2, base.0 + SIDE.0 / 2);
        assert_eq!(tip.position.1, base.1);
    }

    /// Contre le bord gauche du jeu, la réserve commence au bord : l'infobulle glisse à droite.
    #[test]
    fn contre_le_bord_gauche_elle_glisse_a_droite() {
        let base = window_position(Some((0, 300)), CLIENT, SIDE);
        let tip = tip_window(base, SIDE, RESERVE, CLIENT);
        assert_eq!(button_on_screen(&tip), base);
        assert_eq!(tip.position.0, CLIENT.left);
        assert_eq!(tip.size.0, 240);
        assert!(!tip.above);
    }

    /// Contre le bord droit, elle finit au bord : l'infobulle glisse à gauche.
    #[test]
    fn contre_le_bord_droit_elle_glisse_a_gauche() {
        let base = window_position(Some((5000, 300)), CLIENT, SIDE);
        let tip = tip_window(base, SIDE, RESERVE, CLIENT);
        assert_eq!(button_on_screen(&tip), base);
        assert_eq!(tip.position.0 + tip.size.0, CLIENT.left + CLIENT.width);
        assert_eq!(tip.size.0, 240);
    }

    /// Tout en bas, la réserve ne tient plus dessous : elle passe au-dessus, toujours centrée.
    #[test]
    fn tout_en_bas_elle_passe_au_dessus() {
        let base = window_position(Some((900, 5000)), CLIENT, SIDE);
        let tip = tip_window(base, SIDE, RESERVE, CLIENT);
        assert!(tip.above);
        assert_eq!(button_on_screen(&tip), base);
        assert_eq!(tip.position.1, base.1 - RESERVE.1);
        assert_eq!(tip.position.0 + tip.size.0 / 2, base.0 + SIDE.0 / 2);
        // Juste assez de place dessous : elle y reste.
        let juste = (900, CLIENT.height - SIDE.1 - RESERVE.1);
        let base = window_position(Some(juste), CLIENT, SIDE);
        assert!(!tip_window(base, SIDE, RESERVE, CLIENT).above);
    }

    /// Un jeu trop bas pour l'un comme l'autre garde le dessous, le défaut.
    #[test]
    fn sans_place_nulle_part_elle_reste_dessous() {
        let tiny = ClientArea {
            height: 60,
            ..CLIENT
        };
        let base = window_position(None, tiny, SIDE);
        assert!(!tip_window(base, SIDE, RESERVE, tiny).above);
    }

    #[test]
    fn une_position_choisie_est_respectee() {
        assert_eq!(window_position(Some((900, 600)), CLIENT, SIDE), (1000, 650));
    }

    #[test]
    fn le_bouton_reste_dans_la_zone_cliente() {
        assert_eq!(clamp((-30, -5), CLIENT, SIDE), (0, 0));
        assert_eq!(clamp((5000, 5000), CLIENT, SIDE), (1880, 1038));
    }

    #[test]
    fn reposé_pres_de_l_ancrage_il_y_recolle() {
        assert_eq!(snap((DEFAULT_OFFSET.0 + 12, DEFAULT_OFFSET.1 - 12)), None);
        assert_eq!(
            snap((DEFAULT_OFFSET.0 + 13, DEFAULT_OFFSET.1)),
            Some((DEFAULT_OFFSET.0 + 13, DEFAULT_OFFSET.1))
        );
    }

    /// Le geste : le point de saisie reste sous le curseur, et deux lectures du même curseur
    /// donnent la même position — rien ne reboucle.
    #[test]
    fn le_point_de_saisie_reste_sous_le_curseur() {
        let grab = (15, 20);
        let offset = drag_offset((800, 400), grab, CLIENT, SIDE);
        assert_eq!(offset, (800 - 15 - 100, 400 - 20 - 50));
        assert_eq!(drag_offset((800, 400), grab, CLIENT, SIDE), offset);
        let (x, y) = window_position(Some(offset), CLIENT, SIDE);
        assert_eq!((x + grab.0, y + grab.1), (800, 400));
    }
}
