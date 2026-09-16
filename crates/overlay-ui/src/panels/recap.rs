//! **Récap de session** — la bande posée en haut à gauche de la fenêtre de jeu, sous les boutons
//! d'interface du client (2026-09-16, demande utilisateur).
//!
//! Cinq chiffres, et cinq seulement : XP gagnée, kamas nets, combats gagnés − perdus, challenges
//! réussis − échoués, durée de la session. C'est exactement la « bande coup d'œil » du web
//! (`session-recap.component.html`, bloc `.recap-bandeau`), réduite à ce qui se lit d'un regard
//! par-dessus un jeu : le site, lui, déplie sous cette bande l'XP par personnage, le détail des
//! kamas, le butin et les accordéons par donjon — de la consultation APRÈS coup, pas du temps réel.
//!
//! **Une section « Recap » commande son affichage** (onglet « Paramètres » de la fenêtre Options,
//! juste après « Combat ») — voir `panels::feature_switch::FeatureToggles::recap`, active par
//! défaut comme ses voisines.
//!
//! ## La durée vient de l'overlay, jamais du fichier
//!
//! **Décision explicite de l'utilisateur (2026-09-16)** : la durée affichée ici est le temps
//! d'exécution de l'overlay — le chrono part au lancement du processus et avance tant qu'il tourne.
//! Le web fait exactement l'inverse (`StatsStoreService.accumulateSessionDuration` : somme des
//! écarts entre deux lignes horodatées de `wakfu.log`, coupée au-delà de cinq minutes de silence,
//! voir le CLAUDE.md du dépôt web), et c'est ce qu'on ne veut PAS ici : un `wakfu.log` contient
//! typiquement plusieurs sessions de jeu, et l'overlay relit tout le fichier à son démarrage — la
//! durée dérivée du fichier annoncerait donc le temps de jeu d'avant-hier à qui vient d'ouvrir
//! l'overlay.
//!
//! Conséquence assumée, à connaître avant de croire à un bug : **les quatre autres chiffres, eux,
//! couvrent tout le fichier relu** (`overlay_engine::SessionTotals`, alimenté par le rattrapage
//! initial au même titre que par les lignes lues en direct). Un overlay lancé au milieu d'une
//! partie affiche donc l'XP de toute la partie en face d'une durée qui démarre à zéro. Les aligner
//! demanderait de trancher ce qu'est « la session » côté moteur — un autre chantier, et une
//! décision qui n'a pas été prise.
//!
//! ## Ce que ce module ne fait pas
//!
//! Pas de bouton, pas d'état, aucune intention remontée : cette bande ne fait qu'afficher. Elle
//! renvoie la **largeur** qu'elle vient d'occuper, pour que l'hôte ajuste sa fenêtre OS à son
//! contenu (même mécanique que `panels::login::LoginOutcome::content_height`) — une fenêtre plus
//! large que sa bande capterait les clics sur du vide en mode interactif.

use egui::Color32;
use overlay_engine::SessionTotals;

use crate::design::{self, text, tokens, DsIcon, TooltipSide};
use crate::panels::combat::format_fr_thousands;

/// Hauteur de la bande, fond compris — la fenêtre OS est dimensionnée dessus
/// (`main.rs::window_spec`).
///
/// 32 px : la rangée du jeu la plus proche par la fonction (une bande d'information posée sur
/// l'écran, pas un contrôle) est le carré de boutons du Suivi, dont le socle fait 30 px de côté.
/// Deux de plus pour que le cerne du texte ne touche pas le bord du fond.
pub const HEIGHT: f32 = 32.0;

/// Rembourrage horizontal du fond translucide, de chaque côté de la rangée de cellules.
const PADDING_X: f32 = 10.0;

/// Côté du glyphe de chaque cellule. `tokens::SWITCH_ICON_SIZE` (16 px) — la taille à laquelle
/// cette interface peint déjà un glyphe posé à côté d'un libellé court, dans les deux switches du
/// panneau Combat.
const ICON_SIZE: f32 = tokens::SWITCH_ICON_SIZE;

/// Écart entre le glyphe d'une cellule et son chiffre.
const ICON_GAP: f32 = 6.0;

/// Demi-écart entre deux cellules : il y en a un de chaque côté du filet qui les sépare.
const CELL_GAP: f32 = 9.0;

/// Épaisseur du filet vertical entre deux cellules — `tokens::SWITCH_SEPARATOR_WIDTH` (2 px), la
/// valeur que le jeu emploie entre deux cases d'un même switch. Un filet d'un seul pixel, posé à
/// une abscisse fractionnaire, se serait étalé en deux colonnes grises par anti-aliasing (et aurait
/// rendu la capture de non-régression sensible au sous-pixel, voir `show`, qui arrondit ses
/// abscisses pour cette raison).
const SEPARATOR_WIDTH: f32 = tokens::SWITCH_SEPARATOR_WIDTH;

/// Hauteur du filet, en fraction de celle de la bande — il ne touche ni le haut ni le bas, comme
/// le séparateur d'onglets du jeu (`tokens::TAB_SEPARATOR_RAMP_*`, même intention : marquer sans
/// cloisonner).
const SEPARATOR_HEIGHT_RATIO: f32 = 0.55;

/// Couleur du filet — la bordure de switch du design system, la teinte que cette interface emploie
/// déjà pour séparer deux cases voisines d'une même rangée.
const SEPARATOR_COLOR: Color32 = tokens::SWITCH_SEPARATOR;

/// Rayon d'angle du fond translucide — celui du carré de contrôle du Suivi
/// (`panels::watchlist::PANEL_BACKDROP_ROUNDING`), la même bande posée sur le même jeu.
const BACKDROP_ROUNDING: f32 = 6.0;

/// Taille du texte des chiffres. `tokens::CHECKBOX_FONT_SIZE` (15 px) — le plus petit corps que
/// cette interface emploie pour du texte à lire, pas pour un badge.
const FONT_SIZE: f32 = tokens::CHECKBOX_FONT_SIZE;

/// Couleur d'un chiffre neutre (XP, durée) — le blanc cassé des noms de combattants du panneau
/// Combat.
const TEXT_COLOR: Color32 = Color32::from_rgb(0xE8, 0xED, 0xF2);

/// Couleur des kamas — l'or du design system, celui de la monnaie partout dans cette interface.
const KAMAS_COLOR: Color32 = tokens::TEXT_GOLD;

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

/// Une case de la bande : son glyphe, ce qu'elle affiche, et ce que son infobulle en dit.
struct Cell {
    icon: DsIcon,
    tooltip: &'static str,
    segments: Vec<Segment>,
}

/// Les cinq cases, dans l'ordre du web (`.recap-bandeau`) : XP, Kamas, Combats, Challenges, Durée.
///
/// **Aucune icône inventée** — les cinq viennent du registre `design::DsIcon`, donc du jeu :
/// `Xp` et `Kamas` sont les glyphes évidents, `MetricDamage` (la dague du switch de grandeur du
/// panneau Combat) tient pour les combats et `Trophy` pour les challenges. La durée prend
/// `Calendar`, la seule notion de temps que le registre porte — le jeu n'a pas de cadran, et en
/// dessiner un ici serait la première icône de cette interface à ne venir de nulle part.
fn cells(totals: &SessionTotals, uptime: std::time::Duration) -> Vec<Cell> {
    let net_kamas = totals.kamas_gained - totals.kamas_lost;
    vec![
        Cell {
            icon: DsIcon::Xp,
            tooltip: "Expérience gagnée depuis le début de la session",
            segments: vec![Segment {
                text: format!("+{}", format_fr_thousands(totals.xp_gained)),
                color: TEXT_COLOR,
            }],
        },
        Cell {
            icon: DsIcon::Kamas,
            // Le web ouvre ici une infobulle détaillée (combat, ventes HDV, achats, échanges) que
            // le moteur de l'overlay ne ventile pas : `SessionTotals` ne porte qu'un gagné et un
            // dépensé. L'infobulle dit donc ce qu'elle sait, sans promettre le détail du site.
            tooltip: "Kamas gagnés moins kamas dépensés depuis le début de la session",
            // **Pas de symbole « ₭ » derrière le nombre**, contrairement au web. Ubuntu, la police
            // embarquée de cette interface (`design::fonts`), ne couvre pas U+20AD : la première
            // version de cette bande affichait un « ? » à sa place, vu sur la capture du harnais.
            // Le glyphe Kamas à gauche dit déjà de quelle monnaie il s'agit — ajouter une police
            // entière pour un caractère serait payer 100 % du poids pour 0,5 % du signe.
            segments: vec![Segment {
                text: format_fr_thousands(net_kamas),
                color: KAMAS_COLOR,
            }],
        },
        Cell {
            icon: DsIcon::MetricDamage,
            tooltip: "Combats gagnés − combats perdus",
            segments: win_loss(totals.fights_won, totals.fights_lost),
        },
        Cell {
            icon: DsIcon::Trophy,
            tooltip: "Challenges réussis − challenges échoués",
            segments: win_loss(totals.challenges_passed, totals.challenges_failed),
        },
        Cell {
            icon: DsIcon::Calendar,
            tooltip: "Durée de la session — temps écoulé depuis le lancement de l'overlay",
            segments: vec![Segment {
                text: format_duration(uptime),
                color: TEXT_COLOR,
            }],
        },
    ]
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

/// `HH:MM:SS`, heures non bornées (un `99:00:00` reste lisible, un `03:00:00` qui repart à zéro
/// mentirait) — même format que le web (`SessionRecapComponent.duration`, `00:00:00` au départ).
pub fn format_duration(uptime: std::time::Duration) -> String {
    let total = uptime.as_secs();
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

/// Peint la bande et renvoie la largeur qu'elle occupe, fond compris — voir la doc de module pour
/// ce que l'hôte en fait.
///
/// Le contenu est calé en HAUT À GAUCHE de `ui` : la fenêtre OS peut être plus grande que la bande
/// (elle l'est, entre deux ajustements de largeur), le vide qui reste est transparent.
pub fn show(ui: &mut egui::Ui, totals: &SessionTotals, uptime: std::time::Duration) -> f32 {
    // **Le chrono avance tout seul.** Cette architecture ne rend une frame que lorsque quelque
    // chose change (`ControlFlow::Wait`, §6.1 du plan : l'overlay ne consomme rien au repos) — un
    // snapshot du moteur, un survol, un raccourci. La durée, elle, change sans que rien d'autre ne
    // bouge : sans ce rendez-vous, elle resterait figée sur l'écran entre deux lots de log. Une
    // seconde, c'est la granularité de ce qu'elle affiche (`HH:MM:SS`) — rien à gagner à
    // redessiner plus souvent, et l'hôte honore ce délai via `RenderOutcome`/`next_redraw_at`.
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs(1));

    let cells = cells(totals, uptime);
    let font = text::label_font(ui.ctx(), FONT_SIZE);
    let ds = design::DesignSystem::get(ui.ctx());

    // Mise en page d'abord, peinture ensuite : la largeur du fond dépend de celle des chiffres, et
    // un fond peint après les cellules passerait par-dessus elles.
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
    // telle quelle, chaque filet et chaque glyphe de la bande atterrirait à une abscisse fractionnaire,
    // donc étalé sur deux colonnes de pixels par anti-aliasing. Sur un overlay posé par-dessus un
    // jeu, un trait net vaut mieux qu'un trait juste à un demi-pixel près — et c'est aussi ce qui
    // rend la capture de non-régression reproductible d'une machine de rendu à l'autre.
    let cell_widths: Vec<f32> = galleys
        .iter()
        .map(|segments| {
            (ICON_SIZE + ICON_GAP + segments.iter().map(|g| g.size().x).sum::<f32>()).ceil()
        })
        .collect();
    let content_width: f32 = cell_widths.iter().sum::<f32>()
        + (cell_widths.len().saturating_sub(1) as f32) * (2.0 * CELL_GAP + SEPARATOR_WIDTH);
    let total_width = content_width + 2.0 * PADDING_X;

    let origin = ui.max_rect().min;
    let band = egui::Rect::from_min_size(origin, egui::vec2(total_width, HEIGHT));
    ui.painter()
        .rect_filled(band, BACKDROP_ROUNDING, tokens::OVERLAY_BACKDROP);

    let mut x = band.min.x + PADDING_X;
    for (index, (cell, segments)) in cells.iter().zip(&galleys).enumerate() {
        if index > 0 {
            x += CELL_GAP;
            let half = (band.height() * SEPARATOR_HEIGHT_RATIO / 2.0).round();
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(x.round(), (band.center().y - half).round()),
                    egui::vec2(SEPARATOR_WIDTH, half * 2.0),
                ),
                0.0,
                SEPARATOR_COLOR,
            );
            x += SEPARATOR_WIDTH + CELL_GAP;
        }
        let cell_rect = egui::Rect::from_min_size(
            egui::pos2(x, band.min.y),
            egui::vec2(cell_widths[index], HEIGHT),
        );
        paint_cell(ui, &ds, cell_rect, cell, segments);
        x += cell_widths[index];
    }

    total_width
}

/// Glyphe puis chiffres d'une cellule, centrés verticalement dans `rect`, et l'infobulle qui dit
/// de quoi il s'agit.
///
/// **Infobulle EN DESSOUS** (`TooltipSide::Below`) : cette bande est collée au bord haut de la
/// fenêtre de jeu, il n'y a rien au-dessus d'elle — la même raison qui a fait passer toutes les
/// infobulles du Suivi sous sa bande (voir `render_content::WATCHLIST_TOOLTIP_RESERVE`).
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
    // Ces trois glyphes portent leurs couleurs natives (catégorie `couleur` du registre) : les
    // teinter effacerait ce qu'elles disent. Les autres sont blancs dans le fichier et prennent la
    // teinte demandée — ici le blanc neutre du texte, pour que glyphe et chiffre aillent ensemble.
    let tint = if cell.icon.native_color() {
        Color32::WHITE
    } else {
        TEXT_COLOR
    };
    ds.paint_icon(ui.painter(), icon_rect, cell.icon, tint);

    let mut x = rect.min.x + ICON_SIZE + ICON_GAP;
    for (segment, galley) in cell.segments.iter().zip(galleys) {
        let pos = egui::pos2(x, rect.center().y - galley.size().y / 2.0);
        // Cerne COMPLET : cette bande flotte sur l'écran de jeu, dont le fond est arbitraire — voir
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
        ui.id().with(("recap-cell", cell.tooltip)),
        egui::Sense::hover(),
    );
    design::tooltip(&response)
        .side(TooltipSide::Below)
        .text(cell.tooltip);
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

    /// Cinq cases, toujours — c'est le contrat de la demande (XP, kamas, combats, challenges,
    /// durée), et l'ordre de la bande du web.
    #[test]
    fn la_bande_porte_cinq_cases_dans_l_ordre_du_web() {
        let totals = SessionTotals {
            kamas_gained: 1_500,
            kamas_lost: 500,
            xp_gained: 42_000,
            loot_count: 3,
            fights_won: 7,
            fights_lost: 2,
            challenges_passed: 4,
            challenges_failed: 1,
        };
        let cells = cells(&totals, std::time::Duration::from_secs(3600));
        let icons: Vec<DsIcon> = cells.iter().map(|c| c.icon).collect();
        assert_eq!(
            icons,
            vec![
                DsIcon::Xp,
                DsIcon::Kamas,
                DsIcon::MetricDamage,
                DsIcon::Trophy,
                DsIcon::Calendar
            ]
        );
        // Les kamas sont NETS, jamais le seul gagné (le web affiche `stats.netKamas()`).
        // Les kamas s'écrivent sans symbole de monnaie — voir `cells`.
        assert_eq!(cells[1].segments[0].text, "1 000");
        assert_eq!(cells[0].segments[0].text, "+42 000");
        assert_eq!(cells[2].segments[0].text, "7");
        assert_eq!(cells[2].segments[2].text, "2");
        assert_eq!(cells[3].segments[0].text, "4");
        assert_eq!(cells[3].segments[2].text, "1");
        assert_eq!(cells[4].segments[0].text, "01:00:00");
    }
}
