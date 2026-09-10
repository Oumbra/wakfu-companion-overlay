//! Modale "Options" — voir §9.1 du plan d'architecture. Ouverte par le bouton "Options" du carré
//! de contrôle (`panels::watchlist::control_button_row`) ou le raccourci global `Ctrl+Shift+O`
//! (voir `main.rs`/`bin/overlay-ui-x11.rs`). Premier (et pour l'instant seul) réglage exposé
//! (onglet "Paramètres") : le chemin de `wakfu.log` à suivre.
//!
//! **Refonte 2026-09-09 — chrome basé sur les VRAIES textures du jeu, plus des formes peintes à la
//! main** (voir `panels::chamfer`, toujours utilisé ailleurs pour la barre de dégâts, mais plus
//! ici) : chaque mesure ci-dessous vient d'une capture d'écran réelle de la fenêtre Options du jeu
//! (`assets/design-system/interfaces/interface-options-*.png`, six onglets, chrome identique sur
//! les six), affinée par `.claude/skills/design-asset/scripts/dsimg.py analyze` puis VALIDÉE avec
//! l'utilisateur via une simulation HTML/CSS interactive avant ce portage (méthode explicitement
//! demandée : itérer en HTML, moins coûteux qu'itérer directement en Rust/egui, PUIS porter une
//! fois la maquette acceptée).
//!
//! Différences avec la toute première version (chamfrein peint à la main) :
//! - **Coins ARRONDIS, pas chanfreinés** — vérifié au pixel sur `modal-header.png` (rayon ≈12px) :
//!   la modale entière ET son encadré interne ("section") sont arrondis, ce dernier avec un rayon
//!   PLUS PRONONCÉ (18px) que la modale (12px) — constaté directement sur la référence, pas déduit.
//! - **Bannière peinte avec la vraie texture du jeu** (`modal-header.png`) plutôt qu'un dégradé
//!   approximé à la main — son rayon de coin mesuré (2-4px) est assez petit pour tolérer un
//!   étirement uniforme sans déformation perceptible.
//! - **Menu à trois entrées** ("Alertes", "Personnages", "Paramètres") au-dessus de la section,
//!   texture `menu-tabs.png` (dérivée de `tabs-with-first-tab-active.png` du design system, miroir
//!   horizontal pour que le segment actif kaki tombe sur "Paramètres", dernière entrée) — seule
//!   "Paramètres" est câblée (contenu de cette modale), "Alertes"/"Personnages" restent des stubs
//!   visuels en attente d'un futur chantier.
//! - **Bouton "Sélectionner le fichier"** sous le champ (pas à côté).
//! - Fenêtre plus haute (`WINDOW_SIZE`, ratio aligné sur les 720:561 mesurés de la vraie fenêtre du
//!   jeu — demande explicite : « garder une cohérence par rapport au rendu [du jeu] »), section
//!   renommée "Fichier" (au lieu de "Fichier wakfu.log", redondant avec le contenu du champ), champ
//!   + bouton empilés verticalement (au lieu de côte à côte).
//!
//! **Refonte 2026-09-09 (2) — les trois boutons passent sur `design::button`.** Ils étaient peints
//! ici à la main : deux images taillées sur mesure pour le pied de page (libellé gravé dedans, un
//! fichier par libellé) et un 9-slice local pour le bouton "Sélectionner le fichier". Ce sont
//! maintenant trois appels au composant du design system, qui apporte avec lui le survol, l'état
//! pressé, le curseur, l'écrêtage du libellé et la trace de journal. Ont disparu avec eux : les
//! quatre PNG de `assets/ui/options/` (tous des copies octet pour octet d'assets déjà déclarés au
//! manifeste `design::assets`) et le module `panels::nine_slice`, doublon de `design::nine_slice`
//! sans marges par côté ni mode de remplissage.
//!
//! Gain visible : le bouton "Sélectionner le fichier" est rendu à 700px de large depuis une texture
//! de 169px. L'ancien 9-slice ne figeait que 14px de chaque côté, bien moins que les 52px de décor
//! mesurés — les croisillons tombaient dans la bande médiane et s'étiraient sur près de 200px.
//!
//! **Refonte 2026-09-10 — le CORPS passe sur une texture du jeu** (`DsTexture::ModalBody`,
//! `modal-body.png`). Il était peint d'un aplat (`MODAL_BG`, `#1C2023`) : la bannière venait du
//! jeu, le fond sous elle non, et l'écart se voyait. La texture est découpée des six captures de
//! la fenêtre Options puis débarrassée de son contenu — onglets, bouton de réinitialisation,
//! panneau de section, boutons de pied de page (`tools/design-system/build_modal_body.py`, §9 ter
//! du design-system). Elle apporte le grain du jeu et ses hachures d'angle, qui ne sont pas une
//! trame de fond mais un CADRE : denses dans les angles et le long des bords, absentes du centre —
//! le même constat que pour les boutons, à l'échelle de la fenêtre.
//!
//! Deux conséquences sur ce fichier : la couleur du fond ne s'y règle plus (seule sa translucidité
//! reste ici, `MODAL_BODY_TINT`), et les deux angles BAS sont désormais portés par l'alpha de la
//! texture, plus par un `corner_radius`.
//!
//! **Pas de validation filesystem ICI** : cette fonction ne fait que peindre et renvoyer l'INTENTION
//! de l'utilisateur (`OptionsModalAction`) — c'est l'appelant (`main.rs`/`bin/overlay-ui-x11.rs`,
//! qui seuls savent comment déclencher un dialogue de fichier natif et parler au thread Engine) qui
//! valide via `overlay_ingest::discovery::validate_log_path` et alimente [`OptionsModalState::error`]
//! en retour pour le prochain redessin.

use crate::design::{self, ButtonSize, ButtonVariant};

/// Taille de la fenêtre OS dédiée à cette modale (voir `main.rs::create_overlay_window`, cas
/// `OverlayKind::Options`) — largeur inchangée depuis la première version (560pt), hauteur portée
/// à 436pt pour retrouver le ratio 720:561 de la vraie fenêtre Options du jeu (560 × 561 / 720 ≈
/// 436), demande explicite de cohérence visuelle avec le rendu réel plutôt qu'une boîte compacte
/// arbitraire.
pub const WINDOW_SIZE: (f32, f32) = (560.0, 436.0);

// Rembourrage du panneau de contenu — trois axes, et trois seulement. Le relevé de section est
// catégorique : « x=29 pour les titres de section, x=36 pour tout contrôle indenté, x=62 pour le
// texte qui suit une case. Aucun autre alignement n'a été relevé dans les six onglets. » Rapportés
// au bord du panneau (x=17), cela donne les deux valeurs ci-dessous.
//
// La première version mettait 22px partout, titre compris : le titre et ses contrôles étaient donc
// sur le même axe, ce qui efface le seul signal de niveau que le jeu utilise.

// Le champ de chemin est un composant du design system (`design::input`) depuis le 2026-09-10 :
// ses couleurs, son rayon et son retrait de texte ne sont plus des constantes de ce panneau. Les
// trois qui vivaient ici étaient d'ailleurs fausses — fond #1C1E23 au lieu de #0E1115, bord d'1px
// au lieu de 2, rayon 2 au lieu de 4.

/// Gouttière entre le champ de chemin et le bouton "Sélectionner le fichier", posés sur la MÊME
/// ligne (demande utilisateur 2026-09-09 : le bouton ne doit plus prendre toute la largeur).
///
/// 10px, la valeur du relevé (`docs/design-system/releve-section-options.json`, nœud `sel-ctrl`) :
/// c'est l'écart que le jeu laisse entre un libellé et le contrôle posé à sa droite, le seul écart
/// intra-ligne qu'il ait été possible de mesurer. Elle figure bien à l'échelle d'espacement de la
/// section (6, 7, 9, **10**, 11, 17, 21, 30, 31), ce n'est pas une valeur inventée pour l'occasion.
const FIELD_TO_BROWSE_GAP: f32 = 10.0;
/// Hauteur d'une ligne de contrôle — le champ ET le bouton qui l'accompagne.
///
/// 36px, relevé sur les six onglets de la modale du jeu : **tout contrôle posé sur une ligne de
/// contenu y fait 36px de haut**, liste déroulante comme bouton (`releve-section-options.json`,
/// nœuds `sel-ctrl` 233..269 et `depl-ctrl` 314..350 ; `releve-modale-options.json`, note sur le
/// rythme vertical : « une ligne portant un contrôle de 36px passe à 39px »). C'est aussi la
/// hauteur native de la texture de bouton compact, donc la seule hauteur à laquelle un bouton ne
/// subit aucun étirement vertical.
///
/// Remplace deux valeurs qui ne s'accordaient ni entre elles ni avec le jeu : un champ à 34px et
/// un bouton à 40px, alors que le pied de page était déjà à 36.
const ROW_HEIGHT: f32 = 36.0;

/// Écart entre la ligne de contrôle et le message d'erreur qui la commente.
///
/// **9px, pas 10.** Le relevé de section ne cote pas ce cas précis (le jeu n'a qu'un seul bloc
/// d'information, en bas de l'onglet Interface, et il suit une grille de boutons, pas un champ) —
/// c'est donc la valeur de son échelle d'espacement la plus proche du cas « un commentaire collé au
/// contrôle qu'il commente » : celle qui sépare un titre du contrôle pleine largeur qu'il coiffe
/// (7 à 9). Le 10 d'avant venait du même geste qu'ailleurs dans ce fichier — une valeur choisie.
const INFO_GAP: f32 = 9.0;

/// Hauteur du champ de chemin — sa hauteur NATIVE, plus basse que la ligne qui le porte. Voir
/// `design::components::input` pour la mesure et pourquoi les deux diffèrent.
const FIELD_HEIGHT: f32 = design::InputSize::Standard.height();

/// Onglet affiché par la modale.
///
/// **Trois entrées, dont deux encore vides.** « Alertes » et « Personnages » sont conservés et
/// affichés désactivés (décision utilisateur du 2026-09-10) plutôt que masqués : ils le deviendront
/// peu après ce chantier, et un onglet qui apparaît est un changement de mise en page, pas un
/// changement d'état.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum OptionsTab {
    Alertes,
    Personnages,
    /// Le seul onglet cliquable pour l'instant, et donc le défaut.
    #[default]
    Parametres,
}

/// État mutable de la modale, propriété de la fenêtre OS qui l'affiche (voir
/// `main.rs`/`bin/overlay-ui-x11.rs`, nouveau champ `OverlayWindow` réservé au cas
/// `OverlayKind::Options`) — persiste d'une frame à l'autre, contrairement à [`OptionsModalAction`]
/// qui ne vit que le temps d'un `show`.
#[derive(Debug, Default, Clone)]
pub struct OptionsModalState {
    /// Contenu ACTUEL du champ texte — initialisé au chemin actif au moment de l'ouverture de la
    /// modale (voir l'appelant), modifié librement par la frappe ou par [`OptionsModalAction::Browse`]
    /// une fois le dialogue natif résolu. Rien n'est pris en compte tant que
    /// [`OptionsModalAction::Validate`] n'a pas été renvoyé ET jugé valide par l'appelant (§5.1 du
    /// plan : « tant que l'utilisateur n'a pas validé son choix, rien n'est pris en compte »).
    pub path_input: String,
    /// Message d'erreur de la DERNIÈRE tentative de validation (`Browse` sur un fichier mal nommé,
    /// ou clic sur "Valider" avec un chemin invalide) — `None` tant qu'aucune tentative n'a encore
    /// échoué. Vidé par l'appelant dès qu'une nouvelle tentative commence.
    pub error: Option<String>,
    /// Onglet affiché. Ne bouge pas tant que « Alertes » et « Personnages » sont désactivés — le
    /// champ existe pour que le jour où ils s'activeront ne demande qu'une ligne.
    pub tab: OptionsTab,
}

/// Ce que l'utilisateur vient de demander CETTE frame — `None` la plupart du temps (aucun bouton
/// cliqué). Voir doc de module : ne porte aucune garantie de validité, c'est à l'appelant de
/// vérifier avant d'agir.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum OptionsModalAction {
    #[default]
    None,
    Cancel,
    /// Ouvrir l'explorateur de fichiers natif — l'appelant seul sait le faire (`rfd`, sur un thread
    /// dédié pour ne jamais geler le rendu, voir sa doc dans `main.rs`).
    Browse,
    /// Chemin brut tel que tapé/affiché dans le champ au moment du clic — PAS encore un `PathBuf`
    /// validé, voir doc de module.
    Validate(String),
}

/// Clé mémoire « le focus initial a déjà été donné » — voir [`show`].
fn focus_given_id() -> egui::Id {
    egui::Id::new("options-modal-focus-initial")
}

/// Peint la modale dans TOUT le rectangle disponible de `ui` (fenêtre OS dédiée, voir doc de
/// module) et renvoie l'action déclenchée par cette frame, le cas échéant.
pub fn show(ui: &mut egui::Ui, state: &mut OptionsModalState) -> OptionsModalAction {
    let mut action = OptionsModalAction::None;

    // Première frame de CETTE modale ? Sert au focus initial du champ de chemin (voir plus bas).
    // Le drapeau vit dans la mémoire egui du contexte, qui est neuf à chaque ouverture : la modale
    // a sa propre fenêtre OS, créée à l'ouverture et détruite à la fermeture (voir
    // `main.rs::open_options_modal` / `PostRedraw::CloseOptions`). Rouvrir la modale redonne donc
    // bien le focus, refermer et rouvrir n'en garde aucune trace.
    let first_frame = !ui.data_mut(|d| {
        let seen = d.get_temp::<bool>(focus_given_id()).unwrap_or(false);
        d.insert_temp(focus_given_id(), true);
        seen
    });

    // Tout le décor de la fenêtre — `design::window` depuis le 2026-09-10 (lot 1 de
    // `docs/plan-composants-ui.md`). Il vivait ici, dans une fonction `chrome()` de ce panneau,
    // faute d'une place dans le design system pour un composant qui encadre du contenu ; le
    // contrat en a une depuis (§1 bis, composant conteneur). Le déplacement est à pixel constant,
    // ce que les deux snapshots de cette modale vérifient à chaque exécution.
    let chrome = design::window("Options")
        .footer("Annuler", "Valider")
        .log_name("options")
        .show(ui);

    // « Paramètres » est le seul onglet cliquable : les deux autres n'ont pas encore de contenu
    // porté. Ils restent affichés désactivés plutôt que masqués — un onglet qui apparaît est un
    // changement de mise en page, pas un changement d'état.
    chrome.tabs(
        ui,
        design::tabs(&mut state.tab)
            .entry(OptionsTab::Alertes, "Alertes")
            .enabled(false)
            .entry(OptionsTab::Personnages, "Personnages")
            .enabled(false)
            .entry(OptionsTab::Parametres, "Paramètres")
            .log_name("options-onglets"),
    );

    match chrome.footer {
        design::FooterClick::Cancel => action = OptionsModalAction::Cancel,
        design::FooterClick::Validate => {
            action = OptionsModalAction::Validate(state.path_input.clone())
        }
        design::FooterClick::None => {}
    }

    design::panel().show(ui, chrome.content, |ui, _panel| {
        let inner_width = ui.max_rect().width();
        ui.add(design::heading("Fichier"));

        // Le champ et le bouton partagent une ligne : le bouton prend sa largeur naturelle
        // (libellé + marges du design system, voir `Button::desired_size`) et le champ occupe tout
        // le reste. C'est le bouton qui commande, pas l'inverse — une largeur figée pour lui
        // désaccorderait le couple dès que le libellé ou la fenêtre changent.
        let browse = design::button("Parcourir")
            .variant(ButtonVariant::Secondary)
            .size(ButtonSize::Height(ROW_HEIGHT))
            .log_name("options-parcourir");
        let browse_width = browse.desired_size(ui).x;
        let row_rect = ui.allocate_space(egui::vec2(inner_width, ROW_HEIGHT)).1;
        // Plancher à zéro : si la ligne devenait plus étroite que le bouton, un rectangle de
        // largeur négative serait inversé par egui et peint n'importe où. Zéro le rend invisible,
        // ce qui se voit sur une capture — c'est la règle du contrat de composant.
        let field_width = (row_rect.width() - browse_width - FIELD_TO_BROWSE_GAP).max(0.0);
        // Le champ garde sa hauteur native (25px) et se centre sur la ligne, que le bouton fixe à
        // 36 : le jeu compose réellement des lignes où le champ est plus bas que ce qui
        // l'accompagne (voir `design::components::input`), et la règle du design system est que la
        // hauteur d'un composant est celle de sa référence — pas celle de son voisin.
        let field_rect = egui::Rect::from_center_size(
            egui::pos2(row_rect.left() + field_width / 2.0, row_rect.center().y),
            egui::vec2(field_width, FIELD_HEIGHT),
        );
        let browse_rect = egui::Rect::from_min_size(
            egui::pos2(row_rect.right() - browse_width, row_rect.top()),
            egui::vec2(browse_width, ROW_HEIGHT),
        );
        // Focus initial dans le champ à l'ouverture : la modale est la SEULE fenêtre overlay
        // focalisable (§9.1 du plan, `WS_EX_NOACTIVATE` délibérément omis pour elle), et son unique
        // réglage est ce champ — devoir cliquer dedans avant de pouvoir taper n'a aucune raison
        // d'être. Une seule frame, sinon le champ reprendrait le focus indéfiniment.
        ui.put(
            field_rect,
            design::input(&mut state.path_input)
                .placeholder("Chemin vers wakfu.log")
                .width(field_width)
                .request_focus(first_frame)
                .log_name("options-chemin"),
        );

        if ui.put(browse_rect, browse).clicked() {
            action = OptionsModalAction::Browse;
        }

        // Message d'erreur — `design::info_text` au ton alerte depuis le 2026-09-10. C'était le
        // dernier texte de la modale à échapper au design system : un `ui.label` à la police
        // proportionnelle d'egui, au corps 13 arbitraire, sans pastille ni interligne relevé. Son
        // rouge (`#e06055`) n'appartenait à aucune capture ; celui du composant est le rouge mesuré
        // du bouton « Annuler ».
        if let Some(err) = &state.error {
            ui.add_space(INFO_GAP);
            ui.add(
                design::info_text(err)
                    .tone(design::InfoTone::Alert)
                    .width(inner_width)
                    .log_name("options-erreur"),
            );
        }
    });

    // Clavier — lu APRÈS les boutons : un clic de cette frame l'emporte sur une touche de la même
    // frame (cas de figure théorique, mais l'ordre doit être décidé plutôt que subi).
    //
    // Ces deux touches sont traitées ICI, dans le panneau, et non par l'hôte, pour deux raisons.
    // La première est le contrat (§17.3 bis du plan) : un panneau ne produit aucun effet de bord,
    // il remonte une intention — `Cancel`/`Validate` sont exactement les intentions que les boutons
    // du pied de page produisent déjà. La seconde est que l'hôte, lui, ne peut PAS distinguer un
    // Échap destiné à la modale : son filet global `Échap → event_loop.exit()` fermait l'overlay
    // entier (voir `main.rs`/`bin/overlay-ui-x11.rs`, où ce filet exclut désormais cette fenêtre).
    //
    // `TextEdit` ne retire pas ces événements de l'entrée globale (il travaille sur une copie
    // filtrée, `InputState::filtered_events`) : les lire ici reste fiable même quand le champ de
    // chemin a le focus — ce qui est le cas dès l'ouverture.
    if matches!(action, OptionsModalAction::None) {
        let (cancel, validate) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Enter),
            )
        });
        if cancel {
            action = OptionsModalAction::Cancel;
        } else if validate {
            action = OptionsModalAction::Validate(state.path_input.clone());
        }
    }

    action
}
