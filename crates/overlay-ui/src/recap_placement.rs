//! **Où se pose la bande Récap sur la fenêtre de jeu** — l'ancrage d'origine, le décalage que
//! l'utilisateur lui donne au glisser-déposer (2026-09-17), et les deux garde-fous qui
//! l'accompagnent : le bornage à la zone cliente et l'aimantation au retour.
//!
//! **Ici et non dans les deux hôtes** : `main.rs` (Win32) et `bin/wakfu-companion-overlay-x11.rs`
//! dupliquent tout leur fenêtrage OS — c'est assumé, ils ne parlent pas à la même bibliothèque —
//! mais ce calcul-là n'est que de l'arithmétique sur des entiers. Dupliqué, il aurait dérivé
//! d'un hôte à l'autre à la première correction ; ici, il se teste sans serveur graphique, ce
//! qu'aucun des deux binaires ne permet.
//!
//! **Tout est en pixels PHYSIQUES**, comme les rectangles de fenêtre dont ces valeurs sortent
//! (`GameRect`, `Window::outer_size`) et comme les positions qu'on repose ensuite
//! (`Window::set_outer_position`). Seule la réserve d'infobulle ([`Band::reserve`]) naît en
//! points logiques et doit être convertie par l'appelant, qui seul connaît l'échelle
//! d'affichage de sa fenêtre.
//!
//! ## Le décalage décrit le BLOC, pas sa fenêtre
//!
//! La fenêtre OS du bloc commence `render_content::RECAP_TOOLTIP_RESERVE` px plus haut que ce
//! qu'on voit : c'est la place où ses infobulles s'ouvrent (voir `panels::recap::paint_cell`).
//! Le décalage persisté (`config::OverlayConfig::recap_position_x`) vise donc le coin haut-gauche
//! du **fond translucide**, pas celui de la fenêtre — sans quoi le jour où cette réserve change
//! de valeur, la bande de tous ceux qui l'ont déplacée bougerait d'autant.

/// Décalage du bloc, en pixels physiques depuis le coin haut-gauche de la zone cliente du jeu,
/// tant que l'utilisateur ne l'a pas déplacé : **l'ancrage d'origine**, sous la rangée de boutons
/// du client et aligné sur elle.
///
/// **Relevé sur capture d'écran annotée (2026-09-16, tard)** — l'utilisateur a tracé où le bloc
/// doit commencer : juste sous l'infobulle « Ouvrir/Fermer le Shop », avec le même écart que
/// l'infobulle prend elle-même sous son bouton. En pixels du client, depuis le haut de la fausse
/// barre de titre : 24 px de barre de titre, 8 px de vide, les boutons jusqu'à y = 71, 2 px de
/// vide, **l'infobulle de y = 74 à 109** (36 px, une ligne de texte, centrée sur son bouton — pas
/// sur le curseur), 2 px de vide, et le bloc à y = 112. Les valeurs précédentes (70 : dérivée du
/// design system sans capture ; 120 : première capture, garde de 10 px) sont remplacées par ce
/// relevé au pixel. À corriger sur retour d'écran si une infobulle du jeu sur deux lignes passait
/// dessous.
///
/// En abscisse, 4 px et non 0 comme l'overlay Combat : la zone cliente du jeu commence 4 px avant
/// le socle du bouton Menu (le cadre du client fait 4 px, et le bouton est posé contre lui). Le
/// bloc à 0 débordait de 4 px à gauche de la colonne des boutons ; sa largeur
/// (`panels::recap::WIDTH`, 206 px) le fait finir au bord droit du bouton Boutique, comme tracé
/// par l'utilisateur.
pub const DEFAULT_OFFSET: (i32, i32) = (4, 112);

/// **Aimantation** : reposée à moins de ce rayon de son ancrage d'origine, la bande y recolle et
/// la config oublie sa position (voir [`snap`]).
///
/// 12 px : assez pour qu'un retour « à peu près à sa place » rende exactement la position
/// d'usine — un décalage de trois pixels par rapport aux boutons du jeu se verrait, et personne
/// ne sait viser au pixel avec un bloc de 206 px sous le curseur. Assez peu pour qu'une position
/// choisie juste à côté de l'ancrage (sous les boutons, décalée d'un cheveu) reste possible.
pub const SNAP_RADIUS_PX: i32 = 12;

/// La zone CLIENTE de la fenêtre de jeu, en pixels physiques d'écran — ce que les deux hôtes
/// tirent de leur `GameRect`.
///
/// `top` est `GameRect::client_top` et non `GameRect::top` : le client Wakfu dessine sa fausse
/// barre de titre dans sa propre zone cliente (voir la doc de `GameRect::client_top`), et c'est
/// sous ce bord-là que tout l'ancrage de l'overlay se mesure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientArea {
    /// Bord gauche de la fenêtre de jeu (`GameRect::left`).
    pub left: i32,
    /// Bord haut de la zone cliente (`GameRect::client_top`).
    pub top: i32,
    /// Largeur de la fenêtre de jeu (`GameRect::width`).
    pub width: i32,
    /// Hauteur disponible **sous** [`Self::top`], jusqu'au bas de la fenêtre de jeu.
    pub height: i32,
}

/// La fenêtre OS du bloc Récap, en pixels physiques — sa taille, et la part de cette taille qui
/// n'est que la réserve des infobulles, au-dessus du fond translucide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Band {
    /// Largeur de la fenêtre (`panels::recap::WIDTH` à l'échelle de l'écran) — fixe.
    pub width: i32,
    /// Hauteur de la fenêtre, réserve comprise : elle suit le contenu, une ligne empilée de plus
    /// la fait grandir (voir `panels::recap::show`).
    pub height: i32,
    /// `render_content::RECAP_TOOLTIP_RESERVE` à l'échelle de l'écran — la hauteur qui sépare le
    /// haut de la fenêtre du haut du bloc.
    pub reserve: i32,
    /// `render_content::RECAP_ACTIONS_RESERVE` à l'échelle de l'écran (2026-09-17) — la hauteur
    /// que la fenêtre garde SOUS le bloc pour sa rangée d'actions (cadenas, replacement).
    ///
    /// **Gardée des deux côtés en permanence**, qu'elle serve en haut ou en bas : la rangée tient
    /// au-dessus du bloc dans la réserve d'infobulle ([`Self::reserve`], plus haute qu'elle), et
    /// en dessous dans celle-ci. Une fenêtre qui ne réserverait que le côté utilisé changerait de
    /// taille avec le côté, le côté dépend de la position, et la position d'une fenêtre qui
    /// change de taille — c'est la boucle qui a fait vibrer la bande (voir [`drag_offset`]).
    pub actions: i32,
}

impl Band {
    /// La fenêtre du bloc telle que l'hôte la connaît : sa taille en pixels physiques
    /// (`Window::outer_size`) et l'échelle d'affichage de son écran (`Window::scale_factor`).
    ///
    /// **La réserve d'infobulle est la seule valeur LOGIQUE de tout cet ancrage** : elle vit en
    /// points dans la mise en page (`render_content::RECAP_TOOLTIP_RESERVE`, marge haute du
    /// contenu), alors que tout le reste est en pixels d'écran. Elle est convertie ici, une
    /// bonne fois, au plus près de l'échelle qui la décide — à 125 %, l'oublier décalerait la
    /// bande de 9 px vers le haut à chaque replacement.
    pub fn new(width: i32, height: i32, scale: f64) -> Self {
        Self {
            width,
            height,
            reserve: (crate::render_content::RECAP_TOOLTIP_RESERVE as f64 * scale).round() as i32,
            actions: (crate::render_content::RECAP_ACTIONS_RESERVE as f64 * scale).round() as i32,
        }
    }

    /// Hauteur du fond translucide seul — ce que l'utilisateur voit, et ce que le bornage garde
    /// dans la fenêtre de jeu.
    fn visible_height(self) -> i32 {
        (self.height - self.reserve - self.actions).max(0)
    }
}

/// Borne un décalage à la zone cliente du jeu : le **fond translucide** reste entièrement dans le
/// cadre, du bord haut au bord bas.
///
/// **Ce qui est borné, c'est ce qu'on voit** — le bloc, pas sa fenêtre. Les deux réserves de
/// celle-ci (les infobulles au-dessus, la rangée d'actions en dessous, voir [`Band`]) peuvent
/// donc sortir du cadre : une infobulle qui s'ouvre par-dessus la barre du client, ou une rangée
/// d'actions qui passe de l'autre côté du bloc ([`actions_below`]), valent mieux qu'un bloc à qui
/// l'on interdit les 36 px du haut de l'écran de jeu. Jusqu'au 2026-09-17, le bornage gardait la
/// réserve d'infobulle dans le cadre et la bande butait donc 36 px sous le bord haut, sans que
/// rien ne l'explique à l'écran.
///
/// Sans ce bornage, une bande poussée hors de l'écran n'aurait plus aucun moyen d'être rattrapée
/// à la souris — c'est le seul geste qui la déplace. Il ne sert pas qu'au moment de la pose : il
/// est rejoué à chaque placement de la fenêtre, donc une bande posée en bas d'un client en plein
/// écran est ramenée dans le cadre quand ce client repasse en fenêtré, sans que la valeur
/// persistée bouge — la retrouver telle quelle en repassant en plein écran est le comportement
/// voulu.
///
/// Une fenêtre de jeu plus petite que la bande (cas dégénéré : client réduit au minimum) donne un
/// intervalle vide ; le décalage retombe alors sur son minimum plutôt que sur une valeur
/// négative — la bande déborde en bas ou à droite, mais son coin de saisie reste attrapable.
pub fn clamp(offset: (i32, i32), client: ClientArea, band: Band) -> (i32, i32) {
    let (x, y) = offset;
    let max_x = (client.width - band.width).max(0);
    let max_y = (client.height - band.visible_height()).max(0);
    (x.clamp(0, max_x), y.clamp(0, max_y))
}

/// **De quel côté du bloc sa rangée d'actions tient-elle** (2026-09-17) : `false` au-dessus —
/// c'est là que l'utilisateur l'a demandée, « en haut à gauche » — et `true` en dessous quand la
/// bande est posée trop haut dans la fenêtre de jeu pour que la rangée y trouve sa place.
///
/// La place se compte dans la **zone cliente du jeu**, pas dans la fenêtre OS de la bande : c'est
/// ce que l'utilisateur voit, et la fenêtre, elle, garde la hauteur de la rangée des deux côtés
/// (voir [`Band::actions`]). Une bande collée en haut n'a rien au-dessus d'elle ; la rangée passe
/// alors sous le bloc, où il reste forcément de la place puisque le bornage garde le bloc entier
/// dans le cadre.
///
/// Le cas où ni l'un ni l'autre ne tient (fenêtre de jeu plus courte que le bloc, client réduit
/// au minimum) retombe au-dessus : il vaut mieux une rangée qui déborde là où la bande déborde
/// déjà qu'une règle qui s'inverse à chaque pixel de redimensionnement.
pub fn actions_below(offset: Option<(i32, i32)>, client: ClientArea, band: Band) -> bool {
    let (_, y) = clamp(offset.unwrap_or(DEFAULT_OFFSET), client, band);
    let below = client.height - y - band.visible_height();
    y < band.actions && below >= band.actions
}

/// L'aimantation de la pose : `None` quand la bande retombe à moins de [`SNAP_RADIUS_PX`] de son
/// ancrage d'origine, c'est-à-dire « pas de position personnalisée » — ce que la config écrit en
/// effaçant ses deux clés, et ce que le bouton « Replacer au défaut » fait d'un coup.
///
/// `None` plutôt qu'un `Some(DEFAULT_OFFSET)` figé : une position écrite à 4 / 112 survivrait à
/// un changement de l'ancrage d'origine (une infobulle du jeu qui grandit, un relevé refait sur
/// capture) et clouerait la bande à l'ancienne valeur. « Jamais déplacée » doit rester « suit
/// l'ancrage », quel qu'il devienne.
pub fn snap(offset: (i32, i32)) -> Option<(i32, i32)> {
    let (dx, dy) = (
        (offset.0 - DEFAULT_OFFSET.0).abs(),
        (offset.1 - DEFAULT_OFFSET.1).abs(),
    );
    (dx > SNAP_RADIUS_PX || dy > SNAP_RADIUS_PX).then_some(offset)
}

/// Position de la **fenêtre OS** du bloc, en pixels physiques d'écran, pour un décalage donné —
/// `None` étant « jamais déplacée », donc [`DEFAULT_OFFSET`].
///
/// Le décalage est borné au passage ([`clamp`]) : c'est le seul chemin par lequel les deux hôtes
/// placent cette fenêtre, aucun ne peut donc l'oublier.
pub fn window_position(offset: Option<(i32, i32)>, client: ClientArea, band: Band) -> (i32, i32) {
    let (x, y) = clamp(offset.unwrap_or(DEFAULT_OFFSET), client, band);
    (client.left + x, client.top + y - band.reserve)
}

/// Le décalage que vaut le curseur **à l'écran** pendant un glissement, pour une bande tenue par
/// le point `grab`.
///
/// # Pourquoi le curseur d'écran, et pas celui que le panneau remonte
///
/// **Retour d'écran du 2026-09-17, capture vidéo à l'appui : la bande vibrait dans tous les sens,
/// souris immobile**, au point d'être impossible à poser. Le geste était alors calculé dans le
/// repère de la FENÊTRE : `nouvelle position = position posée + (curseur local − point de
/// saisie)`, le curseur local étant celui qu'egui rapporte, mesuré depuis le coin de la bande.
///
/// Cette formule est une boucle : déplacer la fenêtre déplace son coin, donc change le curseur
/// LOCAL **sans que la souris ait bougé**, ce qui redéplace la fenêtre à la frame suivante. Elle
/// ne serait au repos que si la fenêtre se posait dans l'instant, avant l'événement souris
/// suivant — or aucun des deux systèmes ne le garantit : entre la demande de déplacement et les
/// événements qui en tiennent compte, il y a un aller-retour avec le serveur X11 ou le gestionnaire
/// de fenêtres de Windows. Le décalage déjà appliqué se réapplique donc, la bande dépasse,
/// revient, dépasse encore — les images de la vidéo la montrent sauter d'une centaine de pixels
/// d'une frame à l'autre, curseur figé, contenu inchangé.
///
/// Le curseur d'ÉCRAN, lui, ne dépend d'aucune fenêtre : la cible vaut `curseur − point de
/// saisie`, un point fixe du bloc reste sous le pointeur, et deux frames sans mouvement de souris
/// donnent deux fois la même position — donc aucun replacement, donc rien qui vibre. C'est aussi
/// pourquoi cette fonction **ne prend pas la position actuelle de la fenêtre** : elle ne peut
/// structurellement pas la reboucler.
///
/// `cursor` et `grab` sont en pixels physiques : le curseur en coordonnées d'écran
/// (`GetCursorPos` sous Windows, `QueryPointer` sous X11), le point de saisie depuis le coin
/// haut-gauche de la FENÊTRE de la bande, réserve d'infobulle comprise — figé au début du geste,
/// il ne se relit jamais.
pub fn drag_offset(
    cursor: (i32, i32),
    grab: (i32, i32),
    client: ClientArea,
    band: Band,
) -> (i32, i32) {
    offset_of((cursor.0 - grab.0, cursor.1 - grab.1), client, band)
}

/// Le décalage que vaut une position de **fenêtre OS** — l'opération inverse de
/// [`window_position`], celle qui traduit en réglage persistable la fenêtre que l'hôte vient de
/// poser sous le curseur. Borné de la même façon, pour qu'un glissement contre un bord écrive la
/// position réellement occupée et non celle qu'on visait.
pub fn offset_of(window: (i32, i32), client: ClientArea, band: Band) -> (i32, i32) {
    clamp(
        (window.0 - client.left, window.1 - client.top + band.reserve),
        client,
        band,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une fenêtre de jeu 1920×1080 dont la zone cliente commence au bord haut (le vrai client
    /// Wakfu, non décoré), et une bande à trois lignes : 206×(78 de bloc, 36 de réserve
    /// d'infobulle au-dessus, 24 de rangée d'actions en dessous).
    fn plein_ecran() -> (ClientArea, Band) {
        (
            ClientArea {
                left: 0,
                top: 0,
                width: 1920,
                height: 1080,
            },
            Band {
                width: 206,
                height: 138,
                reserve: 36,
                actions: 24,
            },
        )
    }

    /// Sans position persistée, la bande est là où elle a toujours été : sous les boutons du jeu,
    /// sa FENÊTRE remontée de la réserve d'infobulle pour que le BLOC, lui, tombe au pixel voulu.
    #[test]
    fn sans_position_la_bande_garde_son_ancrage_d_origine() {
        let (client, band) = plein_ecran();
        assert_eq!(
            window_position(None, client, band),
            (DEFAULT_OFFSET.0, DEFAULT_OFFSET.1 - band.reserve)
        );
    }

    /// L'aller-retour position de fenêtre ↔ décalage est exact : c'est lui qui relie le geste
    /// (une fenêtre posée sous le curseur) au réglage écrit sur le disque.
    #[test]
    fn la_position_de_fenetre_et_le_decalage_se_repondent() {
        let (client, band) = plein_ecran();
        let offset = (460, 234);
        let window = window_position(Some(offset), client, band);
        assert_eq!(window, (460, 234 - band.reserve));
        assert_eq!(offset_of(window, client, band), offset);
    }

    /// La zone cliente ne commence pas toujours au bord de la fenêtre (harnais X11 sur une
    /// fenêtre décorée, voir `GameRect::client_top`) : le décalage se mesure depuis le bord
    /// CLIENT, pas depuis le coin de la fenêtre.
    #[test]
    fn le_decalage_part_du_bord_client_pas_du_coin_de_la_fenetre() {
        let client = ClientArea {
            left: 100,
            top: 130,
            width: 1280,
            height: 700,
        };
        let band = Band {
            width: 206,
            height: 138,
            reserve: 36,
            actions: 24,
        };
        let window = window_position(Some((40, 200)), client, band);
        assert_eq!(window, (140, 130 + 200 - 36));
        assert_eq!(offset_of(window, client, band), (40, 200));
    }

    /// Une position hors de la fenêtre de jeu est ramenée dedans, bande entière visible : c'est
    /// ce qui empêche de pousser la bande là où plus aucune souris ne la rattraperait.
    #[test]
    fn la_bande_ne_peut_pas_sortir_de_la_fenetre_de_jeu() {
        let (client, band) = plein_ecran();
        assert_eq!(clamp((-80, -80), client, band), (0, 0));
        assert_eq!(
            clamp((5000, 5000), client, band),
            (1920 - 206, 1080 - band.visible_height())
        );
    }

    /// Le bornage suit la fenêtre de jeu : la même position, sur un client deux fois plus petit,
    /// revient dans le cadre — la valeur persistée, elle, n'est pas touchée (c'est l'appelant qui
    /// décide de l'écrire, et il ne le fait qu'au relâchement de la souris).
    #[test]
    fn un_client_plus_petit_ramene_la_bande_dans_le_cadre() {
        let (_, band) = plein_ecran();
        let petit = ClientArea {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        };
        assert_eq!(clamp((1500, 900), petit, band), (800 - 206, 600 - 78));
    }

    /// Cas dégénéré — une fenêtre de jeu plus petite que la bande : pas de position négative, pas
    /// de panique, le coin de saisie reste dans le cadre.
    #[test]
    fn une_fenetre_plus_petite_que_la_bande_ne_donne_pas_de_position_negative() {
        let minuscule = ClientArea {
            left: 0,
            top: 0,
            width: 120,
            height: 60,
        };
        let band = Band {
            width: 206,
            height: 138,
            reserve: 36,
            actions: 24,
        };
        assert_eq!(clamp((50, 50), minuscule, band), (0, 0));
    }

    /// **Le bloc peut toucher le bord haut du cadre** depuis le 2026-09-17 : c'est lui qui est
    /// borné, pas sa fenêtre — la réserve d'infobulle, elle, déborde au-dessus. Auparavant la
    /// bande butait 36 px plus bas sans que rien ne l'explique à l'écran.
    #[test]
    fn le_bloc_peut_toucher_le_bord_haut_du_cadre() {
        let (client, band) = plein_ecran();
        assert_eq!(clamp((4, 0), client, band), (4, 0));
        assert_eq!(
            window_position(Some((4, 0)), client, band),
            (4, -band.reserve)
        );
    }

    /// La rangée d'actions se pose **au-dessus** du bloc — c'est là qu'elle est demandée — tant
    /// qu'il y a sa hauteur de place entre le haut du bloc et le bord du cadre.
    #[test]
    fn la_rangee_d_actions_reste_au_dessus_quand_la_place_y_est() {
        let (client, band) = plein_ecran();
        assert!(!actions_below(None, client, band));
        assert!(!actions_below(Some((4, band.actions)), client, band));
        assert!(!actions_below(Some((900, 500)), client, band));
    }

    /// Collée en haut du cadre, la bande n'a plus rien au-dessus d'elle : la rangée passe
    /// dessous, où le bornage garantit qu'il reste de la place.
    #[test]
    fn une_bande_collee_en_haut_descend_sa_rangee() {
        let (client, band) = plein_ecran();
        assert!(actions_below(Some((4, 0)), client, band));
        assert!(actions_below(Some((4, band.actions - 1)), client, band));
    }

    /// Cas dégénéré — un cadre à peine plus haut que le bloc : ni au-dessus ni en dessous la
    /// rangée ne tient, et elle reste au-dessus plutôt que de s'inverser à chaque pixel de
    /// redimensionnement.
    #[test]
    fn sans_place_nulle_part_la_rangee_reste_au_dessus() {
        let (_, band) = plein_ecran();
        let etroit = ClientArea {
            left: 0,
            top: 0,
            width: 1920,
            height: band.visible_height() + 10,
        };
        assert!(!actions_below(Some((4, 0)), etroit, band));
    }

    /// Reposée près de son ancrage, la bande y recolle et la config oublie sa position ; posée
    /// ailleurs, elle garde exactement ce qu'on lui a donné.
    #[test]
    fn l_aimantation_rend_l_ancrage_d_origine() {
        assert_eq!(snap(DEFAULT_OFFSET), None);
        assert_eq!(
            snap((DEFAULT_OFFSET.0 + SNAP_RADIUS_PX, DEFAULT_OFFSET.1)),
            None
        );
        let juste_hors = (DEFAULT_OFFSET.0, DEFAULT_OFFSET.1 + SNAP_RADIUS_PX + 1);
        assert_eq!(snap(juste_hors), Some(juste_hors));
        assert_eq!(snap((900, 500)), Some((900, 500)));
    }

    /// **La régression du 2026-09-17** : la bande vibrait parce que le geste se calculait dans le
    /// repère de la fenêtre qu'il déplaçait. Souris IMMOBILE, le geste doit rendre exactement la
    /// même position d'une frame à l'autre — c'est ce qui fait qu'aucun replacement n'est demandé,
    /// donc que rien ne bouge.
    ///
    /// Le test ne peut pas rejouer l'ancienne boucle : [`drag_offset`] ne prend pas la position
    /// de la fenêtre, donc elle ne peut plus s'y glisser. C'est l'invariant, et il tient par la
    /// signature autant que par cette assertion.
    #[test]
    fn souris_immobile_bande_immobile() {
        let (client, band) = plein_ecran();
        let grab = (100, 50);
        let curseur = (740, 400);
        let premiere = drag_offset(curseur, grab, client, band);
        for _ in 0..10 {
            assert_eq!(drag_offset(curseur, grab, client, band), premiere);
        }
    }

    /// Le point du bloc par lequel on tient la bande reste sous le curseur : à un pixel de souris
    /// correspond un pixel de bande, aller comme retour, sans traîne ni dépassement.
    #[test]
    fn la_bande_suit_le_curseur_pixel_pour_pixel() {
        let (client, band) = plein_ecran();
        let grab = (100, 50);
        let depart = drag_offset((740, 400), grab, client, band);
        let a_droite = drag_offset((741, 400), grab, client, band);
        let en_bas = drag_offset((740, 401), grab, client, band);
        let revenu = drag_offset((740, 400), grab, client, band);
        assert_eq!(a_droite, (depart.0 + 1, depart.1));
        assert_eq!(en_bas, (depart.0, depart.1 + 1));
        assert_eq!(revenu, depart, "un aller-retour doit revenir au même pixel");
    }

    /// Le point de saisie est bien celui du geste : tenir la bande par son coin ou par son milieu
    /// ne la pose pas au même endroit pour un même curseur.
    #[test]
    fn le_point_de_saisie_decide_de_la_pose() {
        let (client, band) = plein_ecran();
        let curseur = (740, 400);
        let par_le_coin = drag_offset(curseur, (0, 0), client, band);
        let par_le_milieu = drag_offset(curseur, (103, 57), client, band);
        assert_eq!(par_le_coin, (740, 400 + band.reserve));
        assert_eq!(
            par_le_milieu,
            (par_le_coin.0 - 103, par_le_coin.1 - 57),
            "la bande se pose décalée du point par lequel on la tient"
        );
    }

    /// Le bornage vaut pendant le geste comme à la pose : la souris continue vers le bord, la
    /// bande s'arrête au cadre du jeu — et, la souris revenant, elle repart sans avoir accumulé
    /// le trajet perdu (ce que le calcul en écarts, lui, aurait cumulé).
    #[test]
    fn contre_un_bord_la_bande_s_arrete_sans_accumuler() {
        let (client, band) = plein_ecran();
        let grab = (100, 50);
        let contre_le_bord = drag_offset((5000, 400), grab, client, band);
        let plus_loin_encore = drag_offset((9000, 400), grab, client, band);
        assert_eq!(contre_le_bord.0, client.width - band.width);
        assert_eq!(plus_loin_encore, contre_le_bord);
        let revenue = drag_offset((740, 400), grab, client, band);
        assert_eq!(revenue, (640, 350 + band.reserve));
    }
}
