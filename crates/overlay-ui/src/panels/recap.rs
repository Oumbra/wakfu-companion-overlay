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
//! Deux colonnes calées à gauche (kamas au-dessus des combats, XP au-dessus des challenges), et la
//! durée seule sur une troisième ligne, centrée sur la largeur du bloc. Chaque case garde son
//! glyphe à gauche de son chiffre ; **les glyphes restent blancs**, seul le chiffre porte une
//! couleur : or pour les kamas, accent pour l'XP et la durée, vert/rouge pour les deux cases
//! « gagné − perdu ». (La version du soir teintait le glyphe comme son chiffre — retirée sur
//! retour utilisateur : « les icônes doivent être en blanc ».)
//!
//! **Largeur FIXE** ([`WIDTH`]) : le bloc fait exactement la largeur de la rangée de boutons du
//! jeu sous laquelle il est posé (Menu … Boutique), bord à bord — c'est ce que l'utilisateur a
//! tracé sur sa capture. La largeur ne suit donc plus les chiffres ; c'est la HAUTEUR qui bouge :
//! quand une ligne de deux cases ne tient plus dans cette largeur (des milliards d'XP, des
//! millions de kamas), ses deux cases s'empilent l'une au-dessus de l'autre, calées à gauche, et
//! le bloc gagne une ligne — plutôt que deux chiffres qui se chevauchent. Voir [`show`].
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
//! Pas de bouton, pas d'état, aucune intention remontée : ce bloc ne fait qu'afficher. Il
//! renvoie la **largeur** qu'il vient d'occuper, pour que l'hôte ajuste sa fenêtre OS à son
//! contenu (même mécanique que `panels::login::LoginOutcome::content_height`) — une fenêtre plus
//! large que son bloc capterait les clics sur du vide en mode interactif.

use egui::Color32;
use overlay_engine::SessionTotals;

use crate::design::{self, text, tokens, DsIcon, TooltipSide};
use crate::panels::combat::format_fr_thousands;

/// Hauteur d'une ligne du bloc. 22 px : le corps de 15 px des chiffres (voir [`FONT_SIZE`]) plus
/// le cerne d'un pixel de chaque côté, et assez d'interligne pour que deux lignes de glyphes de
/// 16 px ne se touchent pas.
const ROW_HEIGHT: f32 = 22.0;

/// Interligne AJOUTÉ entre deux lignes du bloc — demande utilisateur (2026-09-16, sur capture) :
/// « 5 à 10 pixels entre chacune des lignes, pour aérer ». 8 px, au milieu de la fourchette.
const ROW_GAP: f32 = 8.0;

/// Rembourrage vertical du fond translucide, au-dessus de la première ligne et sous la dernière.
const PADDING_Y: f32 = 6.0;

/// Nombre de lignes du bloc quand tout tient — deux lignes de deux cases, plus la durée seule sur
/// la troisième (voir la doc de module). Une ligne de plus par paire de cases empilée.
const MIN_ROWS: usize = 3;

/// Hauteur du bloc à `rows` lignes, fond compris.
pub const fn height(rows: usize) -> f32 {
    2.0 * PADDING_Y + rows as f32 * ROW_HEIGHT + (rows - 1) as f32 * ROW_GAP
}

/// Hauteur du bloc quand tout tient sur trois lignes — la fenêtre OS naît dessus
/// (`main.rs::create_overlay_window`) et se retaille à ce que [`show`] renvoie ensuite.
pub const HEIGHT: f32 = height(MIN_ROWS);

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

/// Gouttière entre les deux colonnes de la grille. Deux fois l'écart glyphe/chiffre — assez pour
/// que le dernier chiffre de la colonne de gauche ne se lise pas comme le début de la case de
/// droite, sans étirer le bloc.
const COLUMN_GAP: f32 = 18.0;

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
struct Cell {
    icon: DsIcon,
    tooltip: &'static str,
    segments: Vec<Segment>,
}

/// Les cinq cases, dans l'ordre de lecture de la grille (voir [`show`]) : Kamas, XP, Combats,
/// Challenges, Durée. C'est l'ordre de la bande du web (`.recap-bandeau`) à une inversion près,
/// demandée sur capture le 2026-09-16 : les kamas AVANT l'XP.
///
/// **Aucune icône inventée** — les cinq viennent du registre `design::DsIcon`, donc du jeu :
/// `Xp` et `Kamas` sont les glyphes évidents, `Sword` tient pour les combats, `Trophy` pour les
/// challenges et `Clock` pour la durée. Les deux derniers ont été détourés le 2026-09-16 pour ce
/// bloc, qui empruntait jusque-là la dague colorée du switch de grandeur du panneau Combat et la
/// grille de calendrier.
fn cells(totals: &SessionTotals, uptime: std::time::Duration) -> Vec<Cell> {
    let net_kamas = totals.kamas_gained - totals.kamas_lost;
    vec![
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
            //
            // Le signe « - » d'un solde négatif vient de `format_fr_thousands` ; un solde positif
            // s'écrit sans « + », à la différence de l'XP qui ne peut que croître.
            segments: vec![Segment {
                text: format_fr_thousands(net_kamas),
                color: KAMAS_COLOR,
            }],
        },
        Cell {
            icon: DsIcon::Xp,
            tooltip: "Expérience gagnée depuis le début de la session",
            segments: vec![Segment {
                text: format!("+{}", format_fr_thousands(totals.xp_gained)),
                color: ACCENT_COLOR,
            }],
        },
        Cell {
            icon: DsIcon::Sword,
            tooltip: "Combats gagnés − combats perdus",
            segments: win_loss(totals.fights_won, totals.fights_lost),
        },
        Cell {
            icon: DsIcon::Trophy,
            tooltip: "Challenges réussis − challenges échoués",
            segments: win_loss(totals.challenges_passed, totals.challenges_failed),
        },
        Cell {
            icon: DsIcon::Clock,
            tooltip: "Durée de la session — temps écoulé depuis le lancement de l'overlay",
            segments: vec![Segment {
                text: format_duration(uptime),
                color: ACCENT_COLOR,
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

/// Peint le bloc et renvoie la hauteur qu'il occupe, fond compris — voir la doc de module pour
/// ce que l'hôte en fait. La largeur, elle, est fixe : [`WIDTH`].
///
/// Le contenu est calé en HAUT À GAUCHE de `ui` : la fenêtre OS peut être plus grande que le bloc
/// (elle l'est, entre deux ajustements de hauteur), le vide qui reste est transparent.
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
    let layout = layout_rows(&cell_widths, content_width);
    let rows = layout.rows.len() + 1;

    let origin = ui.max_rect().min;
    let band = egui::Rect::from_min_size(origin, egui::vec2(WIDTH, height(rows)));
    ui.painter()
        .rect_filled(band, BACKDROP_ROUNDING, tokens::OVERLAY_BACKDROP);

    let content_left = band.min.x + PADDING_X;
    let right_column = content_left + layout.left_width + COLUMN_GAP;
    let row_top = |row: usize| band.min.y + PADDING_Y + row as f32 * (ROW_HEIGHT + ROW_GAP);
    let mut placements: Vec<(usize, f32, f32)> = layout
        .rows
        .iter()
        .enumerate()
        .flat_map(|(row, cells)| {
            cells.iter().enumerate().map(move |(column, &cell)| {
                let x = if column == 0 {
                    content_left
                } else {
                    right_column
                };
                (cell, x, row_top(row))
            })
        })
        .collect();
    // La durée, seule, centrée sur le bloc.
    placements.push((
        4,
        content_left + ((content_width - cell_widths[4]) / 2.0).round(),
        row_top(rows - 1),
    ));
    for (index, x, y) in placements {
        let cell_rect =
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(cell_widths[index], ROW_HEIGHT));
        paint_cell(ui, &ds, cell_rect, &cells[index], &galleys[index]);
    }

    band.height()
}

/// La grille des quatre premières cases, une fois décidé ce qui tient sur une ligne.
struct RowLayout {
    /// Les lignes, de haut en bas : les indices (dans [`cells`]) des cases qu'elles portent, une
    /// ou deux — la durée n'y figure pas, elle a toujours sa ligne à elle.
    rows: Vec<Vec<usize>>,
    /// Largeur de la colonne de gauche : la plus large des cases de gauche des lignes à DEUX
    /// cases, pour que la colonne de droite s'aligne d'une ligne à l'autre.
    left_width: f32,
}

/// Décide, paire par paire (Kamas/XP, Combats/Challenges), si les deux cases tiennent côte à côte
/// dans `content_width` ou si elles s'empilent — la règle « responsive » demandée le 2026-09-16 :
/// des chiffres trop longs (milliards d'XP, millions de kamas) passent l'un au-dessus de l'autre
/// plutôt que de se chevaucher.
///
/// La colonne de droite est calée après la case de gauche la plus large des lignes qui restent à
/// deux cases ; empiler une ligne peut donc rétrécir cette colonne et rendre de la place à une
/// autre. D'où une ligne empilée à la fois — la plus large en propre d'abord, celle qui
/// déborderait de toute façon — et la colonne recalculée entre deux, jusqu'à ce que tout tienne.
fn layout_rows(cell_widths: &[f32], content_width: f32) -> RowLayout {
    const PAIRS: [(usize, usize); 2] = [(0, 1), (2, 3)];
    let mut stacked = [false; PAIRS.len()];
    let left_width = loop {
        let left_width = PAIRS
            .iter()
            .zip(&stacked)
            .filter(|(_, stacked)| !**stacked)
            .map(|((left, _), _)| cell_widths[*left])
            .fold(0.0_f32, f32::max);
        let widest_overflowing = PAIRS
            .iter()
            .enumerate()
            .filter(|(i, (_, right))| {
                !stacked[*i] && left_width + COLUMN_GAP + cell_widths[*right] > content_width
            })
            .max_by(|(_, a), (_, b)| {
                let own = |(left, right): &(usize, usize)| cell_widths[*left] + cell_widths[*right];
                own(a).total_cmp(&own(b))
            })
            .map(|(i, _)| i);
        match widest_overflowing {
            Some(i) => stacked[i] = true,
            None => break left_width,
        }
    };
    let rows = PAIRS
        .iter()
        .zip(&stacked)
        .flat_map(|((left, right), stacked)| {
            if *stacked {
                vec![vec![*left], vec![*right]]
            } else {
                vec![vec![*left, *right]]
            }
        })
        .collect();
    RowLayout { rows, left_width }
}

/// Glyphe puis chiffres d'une cellule, centrés verticalement dans `rect`, et l'infobulle qui dit
/// de quoi il s'agit.
///
/// **Infobulle EN DESSOUS** (`TooltipSide::Below`) : ce bloc est posé sous les boutons du jeu,
/// près du bord haut de la fenêtre de jeu, et une infobulle ouverte au-dessus viendrait couvrir
/// ces boutons — la même raison qui a fait passer toutes les infobulles du Suivi sous sa bande
/// (voir `render_content::WATCHLIST_TOOLTIP_RESERVE`).
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

    /// Cinq cases, toujours — c'est le contrat de la demande (kamas, XP, combats, challenges,
    /// durée), dans l'ordre où la grille les lit ligne par ligne : les kamas d'abord, puis l'XP.
    #[test]
    fn le_bloc_porte_cinq_cases_kamas_en_tete() {
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
    }

    /// Seul le chiffre est en couleur : or `#FFD700` pour les kamas (demande utilisateur), accent
    /// pour l'XP et la durée, vert/rouge pour les compteurs — et un solde négatif garde son signe.
    #[test]
    fn les_couleurs_suivent_la_maquette() {
        let totals = SessionTotals {
            kamas_gained: 100,
            kamas_lost: 350,
            ..Default::default()
        };
        let cells = cells(&totals, std::time::Duration::ZERO);
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

    /// Tout tient : deux lignes de deux cases, la colonne de droite calée après la case de gauche
    /// la plus large.
    #[test]
    fn deux_paires_qui_tiennent_restent_sur_deux_lignes() {
        let layout = layout_rows(&[40.0, 60.0, 50.0, 45.0, 70.0], 186.0);
        assert_eq!(layout.rows, vec![vec![0, 1], vec![2, 3]]);
        assert_eq!(layout.left_width, 50.0);
    }

    /// Une paire trop large pour la ligne s'empile (la case de gauche au-dessus), l'autre reste
    /// côte à côte — et la colonne de droite ne compte plus que les lignes à deux cases.
    #[test]
    fn une_paire_trop_large_s_empile_seule() {
        let layout = layout_rows(&[120.0, 110.0, 50.0, 45.0, 70.0], 186.0);
        assert_eq!(layout.rows, vec![vec![0], vec![1], vec![2, 3]]);
        assert_eq!(layout.left_width, 50.0);
        assert_eq!(height(layout.rows.len() + 1), height(4));
    }

    /// Empiler une paire peut rendre de la place à l'autre : ici la seconde ne déborde qu'à cause
    /// de la large case de gauche de la première, et retrouve sa ligne une fois celle-ci empilée.
    #[test]
    fn empiler_une_paire_rend_de_la_place_a_l_autre() {
        // 150 + 18 + 60 = 228 > 186 : la première s'empile. Avant, la colonne de droite était à
        // 150 + 18 et 168 + 45 = 213 > 186 aurait aussi empilé la seconde ; après, 50 + 18 + 45.
        let layout = layout_rows(&[150.0, 60.0, 50.0, 45.0, 70.0], 186.0);
        assert_eq!(layout.rows, vec![vec![0], vec![1], vec![2, 3]]);
        assert_eq!(layout.left_width, 50.0);
    }

    /// Les deux paires empilées : cinq lignes, une par case.
    #[test]
    fn les_deux_paires_empilees_font_cinq_lignes() {
        let layout = layout_rows(&[120.0, 110.0, 100.0, 100.0, 70.0], 186.0);
        assert_eq!(layout.rows, vec![vec![0], vec![1], vec![2], vec![3]]);
        assert_eq!(layout.left_width, 0.0);
        assert_eq!(height(5), 2.0 * 6.0 + 5.0 * 22.0 + 4.0 * 8.0);
    }
}
