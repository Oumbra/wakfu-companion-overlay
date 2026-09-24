//! **Où se pose le panneau Combat sur la fenêtre de jeu** — le bord vertical auquel il se colle
//! (case « à droite » des Options), la HAUTEUR que l'utilisateur lui donne au glisser-déposer
//! (2026-09-17, demande utilisateur), et les deux garde-fous qui l'accompagnent : le bornage à la
//! fenêtre de jeu et l'aimantation au centrage d'origine.
//!
//! **Ici et non dans les deux hôtes**, pour la même raison que `crate::recap_placement` (lire sa
//! doc de module) : `main.rs` (Win32) et `bin/wakfu-companion-overlay-x11.rs` dupliquent tout leur
//! fenêtrage OS, mais ce calcul-là n'est que de l'arithmétique sur des entiers — dupliqué, il
//! aurait dérivé d'un hôte à l'autre à la première correction, et il ne se teste nulle part
//! ailleurs sans serveur graphique.
//!
//! **Tout est en pixels PHYSIQUES**, comme les rectangles de fenêtre dont ces valeurs sortent
//! (`GameRect`, `Window::outer_size`) et comme les positions qu'on repose ensuite
//! (`Window::set_outer_position`). Seule la réserve d'infobulle ([`Panel::reserve`]) naît en points
//! logiques et est convertie par [`Panel::new`], au plus près de l'échelle qui la décide.
//!
//! ## Une seule valeur persistée : la hauteur
//!
//! « Il est bloqué sur le côté, il peut simplement le déplacer de haut en bas » (demande
//! utilisateur) : l'abscisse n'est jamais un réglage — elle vaut le bord gauche ou le bord droit
//! du client, selon la case des Options — et **la hauteur ne dépend pas du côté**. Passer de
//! gauche à droite garde donc la hauteur choisie, sans rien recalculer :
//! `config::OverlayConfig::combat_position_y` est une clé unique, lue par les deux côtés.
//!
//! ## Le décalage décrit le CONTENU, pas sa fenêtre
//!
//! La fenêtre OS du panneau commence `render_content::COMBAT_TOP_MARGIN` px plus haut que son
//! premier widget : c'est la place où l'infobulle du switch Alliés/Ennemis s'ouvre (voir sa doc).
//! Le décalage persisté vise donc le haut du CONTENU, pas celui de la fenêtre — sans quoi le jour
//! où cette réserve change de valeur, le panneau de tous ceux qui l'ont déplacé glisserait
//! d'autant.
//!
//! ## Ce qui est borné, en revanche, c'est la FENÊTRE entière
//!
//! À l'inverse du bloc Récap, dont la réserve d'infobulle peut déborder du cadre : le panneau
//! Combat peint sa rangée d'actions (cadenas, replacement — `panels::combat::paint_actions_row`)
//! DANS cette réserve, en haut de sa fenêtre. Une fenêtre qui sortirait par le haut emporterait
//! le cadenas avec elle, et le seul geste qui ramène le panneau serait alors perdu. Le prix est
//! que le contenu ne peut pas monter plus haut que `reserve` sous le bord du cadre ; c'est
//! exactement la marge que le panneau centré prend déjà aujourd'hui.

/// **Aimantation** : reposé à moins de ce rayon de son centrage d'origine, le panneau y recolle et
/// la config oublie sa hauteur (voir [`snap`]).
///
/// 12 px, comme `recap_placement::SNAP_RADIUS_PX` et pour la même raison : personne ne sait
/// remettre au pixel un panneau de 500 px de haut sous le curseur, et « à peu près au centre »
/// doit rendre exactement le centre — sans interdire une hauteur choisie juste à côté.
pub const SNAP_RADIUS_PX: i32 = 12;

/// La fenêtre de jeu, en pixels physiques d'écran — ce que les deux hôtes tirent de leur
/// `GameRect`.
///
/// **`top`/`height` sont ceux de la fenêtre ENTIÈRE** (`GameRect::top`, `GameRect::height`), pas
/// de la zone cliente comme `recap_placement::ClientArea` : le panneau Combat est centré sur toute
/// la hauteur du client depuis l'origine (S1/L2), fausse barre de titre comprise, et déplacer ce
/// repère décalerait le panneau de tout le monde sans que personne l'ait demandé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientArea {
    /// Bord gauche de la fenêtre de jeu (`GameRect::left`).
    pub left: i32,
    /// Bord haut de la fenêtre de jeu (`GameRect::top`).
    pub top: i32,
    /// Largeur de la fenêtre de jeu (`GameRect::width`).
    pub width: i32,
    /// Hauteur de la fenêtre de jeu (`GameRect::height`).
    pub height: i32,
}

/// La fenêtre OS du panneau Combat, en pixels physiques — sa taille, et la part de cette taille
/// qui n'est que la réserve d'infobulle, au-dessus du contenu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panel {
    /// Largeur de la fenêtre (`main.rs::WINDOW_SIZE`, à l'échelle de l'écran) — fixe.
    pub width: i32,
    /// Hauteur de la fenêtre, réserve comprise — fixe elle aussi : ce panneau ne se retaille pas
    /// au contenu (contrairement au Récap et au Suivi).
    pub height: i32,
    /// `render_content::COMBAT_TOP_MARGIN` à l'échelle de l'écran — la hauteur qui sépare le haut
    /// de la fenêtre du premier widget du panneau, et où vit sa rangée d'actions.
    pub reserve: i32,
}

impl Panel {
    /// La fenêtre du panneau telle que l'hôte la connaît : sa taille en pixels physiques
    /// (`Window::outer_size`) et l'échelle d'affichage de son écran (`Window::scale_factor`).
    ///
    /// **La réserve est la seule valeur LOGIQUE de tout cet ancrage** : elle vit en points dans la
    /// mise en page (`render_content::COMBAT_TOP_MARGIN`, marge haute du contenu) alors que tout
    /// le reste est en pixels d'écran. À 125 %, l'oublier décalerait le panneau de 11 px vers le
    /// haut à chaque replacement.
    pub fn new(width: i32, height: i32, scale: f64) -> Self {
        Self {
            width,
            height,
            reserve: (crate::render_content::COMBAT_TOP_MARGIN as f64 * scale).round() as i32,
        }
    }
}

/// **Le centrage d'origine** : l'ordonnée du contenu, en pixels depuis le bord haut de la fenêtre
/// de jeu, tant que l'utilisateur n'a pas déplacé le panneau — la fenêtre centrée verticalement
/// sur le client, comportement d'origine de l'overlay Combat (S1/L2).
///
/// Une fonction et non une constante, contrairement à `recap_placement::DEFAULT_OFFSET` : ce
/// défaut-ci dépend de la fenêtre de jeu, donc il change quand le client est redimensionné. C'est
/// aussi pourquoi « jamais déplacé » se persiste en `None` et non en valeur figée (voir [`snap`]).
pub fn default_offset(client: ClientArea, panel: Panel) -> i32 {
    panel.reserve + (client.height - panel.height) / 2
}

/// Borne une hauteur à la fenêtre de jeu : la **fenêtre entière** du panneau reste dans le cadre,
/// du bord haut au bord bas (voir la doc de module, « Ce qui est borné »).
///
/// Il ne sert pas qu'au moment de la pose : il est rejoué à chaque placement de la fenêtre, donc
/// un panneau posé en bas d'un client en plein écran est ramené dans le cadre quand ce client
/// repasse en fenêtré, sans que la valeur persistée bouge — la retrouver telle quelle en
/// repassant en plein écran est le comportement voulu.
///
/// Une fenêtre de jeu plus courte que le panneau (cas dégénéré : client réduit au minimum) donne
/// un intervalle vide ; la hauteur retombe alors sur son minimum plutôt que sur une valeur
/// négative — le panneau déborde en bas, mais sa rangée d'actions reste à l'écran.
pub fn clamp(offset: i32, client: ClientArea, panel: Panel) -> i32 {
    let course = (client.height - panel.height).max(0);
    offset.clamp(panel.reserve, panel.reserve + course)
}

/// L'aimantation de la pose : `None` quand le panneau retombe à moins de [`SNAP_RADIUS_PX`] de son
/// centrage d'origine, c'est-à-dire « pas de hauteur personnalisée » — ce que la config écrit en
/// effaçant sa clé, et ce que le glyphe de replacement fait d'un coup.
///
/// `None` plutôt qu'un `Some(centre)` figé : une hauteur écrite en dur survivrait à un
/// redimensionnement de la fenêtre de jeu et clouerait le panneau à un centre qui n'en est plus
/// un. « Jamais déplacé » doit rester « suit le centre », quel qu'il devienne.
pub fn snap(offset: i32, client: ClientArea, panel: Panel) -> Option<i32> {
    ((offset - default_offset(client, panel)).abs() > SNAP_RADIUS_PX).then_some(offset)
}

/// Position de la **fenêtre OS** du panneau, en pixels physiques d'écran — `None` étant « jamais
/// déplacé », donc [`default_offset`].
///
/// `on_right` est la case « Afficher le panneau de combat à droite de la fenêtre de jeu »
/// (`config::OverlayConfig::combat_on_right`) : elle décide du bord vertical, et de lui seul. La
/// marge à ce bord est nulle des deux côtés (voir `main.rs::GAME_EDGE_MARGIN_PX`, « comme si
/// l'overlay faisait partie du jeu »), la symétrie est donc exacte.
///
/// La hauteur est bornée au passage ([`clamp`]) : c'est le seul chemin par lequel les deux hôtes
/// placent cette fenêtre, aucun ne peut donc l'oublier.
pub fn window_position(
    offset: Option<i32>,
    on_right: bool,
    client: ClientArea,
    panel: Panel,
) -> (i32, i32) {
    let x = if on_right {
        client.left + client.width - panel.width
    } else {
        client.left
    };
    let y = clamp(
        offset.unwrap_or_else(|| default_offset(client, panel)),
        client,
        panel,
    );
    (x, client.top + y - panel.reserve)
}

/// La hauteur que vaut une position de **fenêtre OS** — l'opération inverse de
/// [`window_position`], celle qui traduit en réglage persistable la fenêtre que l'hôte vient de
/// poser sous le curseur. Bornée de la même façon, pour qu'un glissement contre un bord écrive la
/// hauteur réellement occupée et non celle qu'on visait.
pub fn offset_of(window_y: i32, client: ClientArea, panel: Panel) -> i32 {
    clamp(window_y - client.top + panel.reserve, client, panel)
}

/// La hauteur que vaut le curseur **à l'écran** pendant un glissement, pour un panneau tenu par le
/// point `grab_y`.
///
/// # Pourquoi le curseur d'écran, et pas celui que le panneau remonte
///
/// Exactement la raison qui a fait vibrer la bande Récap le 2026-09-17, diagnostic complet dans
/// `recap_placement::drag_offset` : un geste calculé dans le repère de la FENÊTRE qu'il déplace est
/// une boucle — déplacer la fenêtre déplace son coin, donc change le curseur LOCAL sans que la
/// souris ait bougé, ce qui la redéplace à la frame suivante. Le curseur d'ÉCRAN, lui, ne dépend
/// d'aucune fenêtre : deux frames sans mouvement de souris donnent deux fois la même position,
/// donc aucun replacement, donc rien qui bouge. C'est aussi pourquoi cette fonction **ne prend pas
/// la position actuelle de la fenêtre** : elle ne peut structurellement pas la reboucler.
///
/// `cursor_y` et `grab_y` sont en pixels physiques : le curseur en coordonnées d'écran
/// (`GetCursorPos` sous Windows, `QueryPointer` sous X11), le point de saisie depuis le haut de la
/// FENÊTRE du panneau, réserve comprise — figé au début du geste, il ne se relit jamais.
pub fn drag_offset(cursor_y: i32, grab_y: i32, client: ClientArea, panel: Panel) -> i32 {
    offset_of(cursor_y - grab_y, client, panel)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une fenêtre de jeu 1920×1080 non décorée et la fenêtre du panneau Combat à l'échelle 1 :
    /// 420 × 524, dont 44 px de réserve d'infobulle en haut (`render_content::COMBAT_TOP_MARGIN`).
    fn plein_ecran() -> (ClientArea, Panel) {
        (
            ClientArea {
                left: 0,
                top: 0,
                width: 1920,
                height: 1080,
            },
            Panel {
                width: 420,
                height: 524,
                reserve: 44,
            },
        )
    }

    /// Sans hauteur persistée, le panneau est là où il a toujours été : sa FENÊTRE centrée
    /// verticalement sur la fenêtre de jeu.
    #[test]
    fn sans_hauteur_le_panneau_garde_son_centrage() {
        let (client, panel) = plein_ecran();
        let (x, y) = window_position(None, false, client, panel);
        assert_eq!((x, y), (0, (1080 - 524) / 2));
    }

    /// Le côté ne change que l'abscisse — **la hauteur est la même des deux côtés**, c'est la
    /// demande même de l'utilisateur (« même s'il change de côté, la hauteur est conservée »).
    #[test]
    fn le_cote_ne_change_que_l_abscisse() {
        let (client, panel) = plein_ecran();
        let gauche = window_position(Some(300), false, client, panel);
        let droite = window_position(Some(300), true, client, panel);
        assert_eq!(gauche.1, droite.1);
        assert_eq!(gauche.0, 0);
        assert_eq!(droite.0, 1920 - 420);
    }

    /// L'aller-retour position de fenêtre ↔ hauteur est exact : c'est lui qui relie le geste (une
    /// fenêtre posée sous le curseur) au réglage écrit sur le disque.
    #[test]
    fn la_position_de_fenetre_et_la_hauteur_se_repondent() {
        let (client, panel) = plein_ecran();
        let offset = 300;
        let (_, window_y) = window_position(Some(offset), false, client, panel);
        assert_eq!(window_y, 300 - panel.reserve);
        assert_eq!(offset_of(window_y, client, panel), offset);
    }

    /// La fenêtre de jeu ne commence pas toujours en haut à gauche de l'écran : la hauteur se
    /// mesure depuis le bord de la fenêtre de JEU, jamais depuis celui de l'écran.
    #[test]
    fn la_hauteur_part_du_bord_de_la_fenetre_de_jeu() {
        let client = ClientArea {
            left: 100,
            top: 130,
            width: 1280,
            height: 720,
        };
        let panel = Panel {
            width: 420,
            height: 524,
            reserve: 44,
        };
        let (x, y) = window_position(Some(200), false, client, panel);
        assert_eq!((x, y), (100, 130 + 200 - 44));
        assert_eq!(offset_of(y, client, panel), 200);
    }

    /// **La fenêtre entière reste dans le cadre**, rangée d'actions comprise : c'est ce qui
    /// empêche de pousser le cadenas hors de l'écran, seul geste qui ramène le panneau ensuite.
    #[test]
    fn la_fenetre_ne_peut_pas_sortir_de_la_fenetre_de_jeu() {
        let (client, panel) = plein_ecran();
        assert_eq!(clamp(-500, client, panel), panel.reserve);
        assert_eq!(
            clamp(5000, client, panel),
            panel.reserve + 1080 - 524,
            "borné en bas par la hauteur du client moins celle de la fenêtre"
        );
        let (_, haut) = window_position(Some(-500), false, client, panel);
        assert_eq!(haut, 0, "la fenêtre touche le bord haut, sans le dépasser");
    }

    /// Le bornage suit la fenêtre de jeu : la même hauteur, sur un client deux fois plus petit,
    /// revient dans le cadre — la valeur persistée, elle, n'est pas touchée (c'est l'appelant qui
    /// décide de l'écrire, et il ne le fait qu'au relâchement de la souris).
    #[test]
    fn un_client_plus_petit_ramene_le_panneau_dans_le_cadre() {
        let (_, panel) = plein_ecran();
        let petit = ClientArea {
            left: 0,
            top: 0,
            width: 1024,
            height: 600,
        };
        assert_eq!(clamp(500, petit, panel), panel.reserve + 600 - 524);
    }

    /// Cas dégénéré — une fenêtre de jeu plus courte que le panneau : pas de hauteur négative, pas
    /// de panique, la rangée d'actions reste attrapable en haut.
    #[test]
    fn une_fenetre_plus_courte_que_le_panneau_ne_donne_pas_de_hauteur_negative() {
        let (_, panel) = plein_ecran();
        let minuscule = ClientArea {
            left: 0,
            top: 0,
            width: 800,
            height: 400,
        };
        assert_eq!(clamp(200, minuscule, panel), panel.reserve);
        let (_, y) = window_position(Some(200), false, minuscule, panel);
        assert_eq!(y, 0);
    }

    /// Reposé près de son centre, le panneau y recolle et la config oublie sa hauteur ; posé
    /// ailleurs, il garde exactement ce qu'on lui a donné.
    #[test]
    fn l_aimantation_rend_le_centrage_d_origine() {
        let (client, panel) = plein_ecran();
        let centre = default_offset(client, panel);
        assert_eq!(snap(centre, client, panel), None);
        assert_eq!(snap(centre + SNAP_RADIUS_PX, client, panel), None);
        let juste_hors = centre + SNAP_RADIUS_PX + 1;
        assert_eq!(snap(juste_hors, client, panel), Some(juste_hors));
        assert_eq!(snap(panel.reserve, client, panel), Some(panel.reserve));
    }

    /// **L'invariant de la vibration** : souris IMMOBILE, le geste doit rendre exactement la même
    /// hauteur d'une frame à l'autre — c'est ce qui fait qu'aucun replacement n'est demandé, donc
    /// que rien ne bouge (voir [`drag_offset`] et `recap_placement::drag_offset`).
    #[test]
    fn souris_immobile_panneau_immobile() {
        let (client, panel) = plein_ecran();
        let premiere = drag_offset(600, 120, client, panel);
        for _ in 0..10 {
            assert_eq!(drag_offset(600, 120, client, panel), premiere);
        }
    }

    /// Le point du panneau par lequel on le tient reste sous le curseur : à un pixel de souris
    /// correspond un pixel de panneau, aller comme retour, sans traîne ni dépassement.
    #[test]
    fn le_panneau_suit_le_curseur_pixel_pour_pixel() {
        let (client, panel) = plein_ecran();
        let depart = drag_offset(600, 120, client, panel);
        assert_eq!(drag_offset(601, 120, client, panel), depart + 1);
        assert_eq!(drag_offset(599, 120, client, panel), depart - 1);
        assert_eq!(
            drag_offset(600, 120, client, panel),
            depart,
            "un aller-retour doit revenir au même pixel"
        );
    }

    /// Le point de saisie est bien celui du geste : tenir le panneau par le haut ou par le milieu
    /// ne le pose pas au même endroit pour un même curseur.
    ///
    /// Curseur à 400 et non à 600 : tenu par son bord HAUT, un curseur à 600 pousserait la fenêtre
    /// au-delà du bord bas du cadre et le bornage l'y retiendrait — ce que le test voisin
    /// (`contre_un_bord_le_panneau_s_arrete_sans_accumuler`) vérifie déjà, et qui masquerait ici
    /// l'écart que l'on veut voir entre les deux prises.
    #[test]
    fn le_point_de_saisie_decide_de_la_pose() {
        let (client, panel) = plein_ecran();
        let par_le_haut = drag_offset(400, 0, client, panel);
        let par_le_milieu = drag_offset(400, 262, client, panel);
        assert_eq!(par_le_haut, 400 + panel.reserve);
        assert_eq!(par_le_milieu, par_le_haut - 262);
    }

    /// Le bornage vaut pendant le geste comme à la pose : la souris continue vers le bord, le
    /// panneau s'arrête au cadre du jeu — et, la souris revenant, il repart sans avoir accumulé le
    /// trajet perdu (ce que le calcul en écarts, lui, aurait cumulé).
    #[test]
    fn contre_un_bord_le_panneau_s_arrete_sans_accumuler() {
        let (client, panel) = plein_ecran();
        let contre_le_bord = drag_offset(5000, 120, client, panel);
        let plus_loin_encore = drag_offset(9000, 120, client, panel);
        assert_eq!(contre_le_bord, panel.reserve + 1080 - 524);
        assert_eq!(plus_loin_encore, contre_le_bord);
        assert_eq!(
            drag_offset(600, 120, client, panel),
            600 + panel.reserve - 120
        );
    }
}
