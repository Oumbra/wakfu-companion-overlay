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
//! **Tout est en pixels PHYSIQUES**, comme `recap_placement` (voir sa doc) : seul le côté du
//! bouton naît en points logiques (`panels::click_through::BUTTON_SIZE`), et l'appelant le
//! convertit, lui seul connaissant l'échelle d'affichage de sa fenêtre.

pub use crate::recap_placement::ClientArea;

/// **Écart entre deux boutons de premier plan du jeu** (demande utilisateur 2026-09-28 : « le
/// même espacement entre ce nouveau bouton et ceux du jeu »).
///
/// Mesuré sur `assets/design-system/menu-button-icon-first-plan.png`, capture à l'échelle 1 de la
/// colonne de boutons de premier plan du jeu : le socle `button-icon-first-plan.png` (36 px) s'y
/// recale tous les 38 px, huit fois de suite (y = 5, 43, 81 … 271), soit 2 px de vide entre deux
/// socles. La rangée Menu … Boutique n'a pas de capture à elle ; elle est de la même famille.
pub const GAME_BUTTON_GAP: i32 = 2;

/// Décalage du bouton tant que l'utilisateur ne l'a pas déplacé, en pixels physiques depuis le
/// coin haut-gauche de la zone cliente du jeu (`GameRect::left`, `GameRect::client_top`).
///
/// - En ordonnée, 32 : le haut des boutons du jeu (24 px de fausse barre de titre, 8 px de vide,
///   même relevé que `recap_placement::DEFAULT_OFFSET`).
/// - En abscisse, 212 : le bouton Boutique finit à x = 210 (le bloc Récap, 206 px posés à x = 4,
///   « finit au bord droit du bouton Boutique »), plus [`GAME_BUTTON_GAP`].
pub const DEFAULT_OFFSET: (i32, i32) = (210 + GAME_BUTTON_GAP, 32);

/// **Aimantation** : reposé à moins de ce rayon de son ancrage d'origine, le bouton y recolle et
/// la config oublie sa position (voir [`snap`]). La valeur de `recap_placement::SNAP_RADIUS_PX`,
/// pour la même raison : un retour « à peu près à sa place » doit rendre la place exacte, à côté
/// des boutons du jeu où trois pixels d'écart se verraient.
pub const SNAP_RADIUS_PX: i32 = crate::recap_placement::SNAP_RADIUS_PX;

/// Borne un décalage à la zone cliente du jeu : le bouton reste entièrement dans le cadre.
///
/// `side` est le côté du bouton en pixels physiques. Une zone cliente plus petite que le bouton
/// retombe sur 0 plutôt que sur une valeur négative — le coin de saisie reste attrapable.
pub fn clamp(offset: (i32, i32), client: ClientArea, side: i32) -> (i32, i32) {
    let max_x = (client.width - side).max(0);
    let max_y = (client.height - side).max(0);
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
pub fn window_position(offset: Option<(i32, i32)>, client: ClientArea, side: i32) -> (i32, i32) {
    let (x, y) = clamp(offset.unwrap_or(DEFAULT_OFFSET), client, side);
    (client.left + x, client.top + y)
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
    side: i32,
) -> (i32, i32) {
    clamp(
        (
            cursor.0 - grab.0 - client.left,
            cursor.1 - grab.1 - client.top,
        ),
        client,
        side,
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
    const SIDE: i32 = 40;

    #[test]
    fn jamais_deplace_il_suit_l_ancrage_d_origine() {
        assert_eq!(window_position(None, CLIENT, SIDE), (100 + 212, 50 + 32));
    }

    #[test]
    fn une_position_choisie_est_respectee() {
        assert_eq!(window_position(Some((900, 600)), CLIENT, SIDE), (1000, 650));
    }

    #[test]
    fn le_bouton_reste_dans_la_zone_cliente() {
        assert_eq!(clamp((-30, -5), CLIENT, SIDE), (0, 0));
        assert_eq!(clamp((5000, 5000), CLIENT, SIDE), (1880, 1040));
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
