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

/// Rayon d'arrondi de la modale ENTIÈRE (bannière haute + pied de page bas) — mesuré au pixel sur
/// `modal-header.png` (transition alpha au coin haut-gauche/haut-droit).
const MODAL_RADIUS: u8 = 12;
/// Rayon d'arrondi du panneau de contenu.
///
/// **2, pas 18.** La première version lisait « un encadré interne au rayon plus prononcé que la
/// modale » sur la référence, à l'œil. Le relevé
/// (`docs/design-system/releve-modale-options.json`, nœud `panel`) mesure 2 — le même rayon que la
/// fenêtre et que tous les contrôles. Il n'existe aucun rayon prononcé dans cette interface.
const SECTION_RADIUS: u8 = 2;

/// Épaisseur du bord du panneau de contenu. Le jeu en a un, la première version n'en peignait
/// aucun.
const SECTION_BORDER: f32 = 2.0;

const BANNER_HEIGHT: f32 = 56.0;
/// Marge gauche/droite du contenu (`.body` de la simulation HTML validée) — hors bannière/pied de
/// page, qui restent pleine largeur.
const BODY_PAD_SIDE: f32 = 20.0;
/// Écart bannière → menu à onglets.
const BODY_PAD_TOP: f32 = 14.0;
/// Marge résiduelle sous le pied de page (bande sombre mesurée sous les boutons Annuler/Valider
/// sur la référence réelle, ≈12px/561) — évite que ces boutons touchent directement le bord bas
/// arrondi de la modale (retour utilisateur explicite : incohérence constatée sur ce point avec le
/// design system).
const BODY_PAD_BOTTOM: f32 = 12.0;

/// Hauteur de la barre d'onglets — **la hauteur native de sa texture**, reprise du composant.
///
/// 44, pas 31. La première version étirait la texture de 44px à 31, ce qui aplatissait son décor et
/// son libellé avec : la même erreur que les boutons du pied de page, dont la hauteur était déduite
/// du rapport d'aspect (voir `FOOTER_BUTTON_HEIGHT`). La section de contenu perd donc 13px, et c'est
/// le bon sens de la correction — le jeu donne bien 44px à ses onglets.
const MENU_HEIGHT: f32 = design::tokens::TAB_HEIGHT;
/// Écart menu → section.
const MENU_GAP: f32 = 14.0;
/// Écart section → pied de page.
const FOOTER_GAP: f32 = 14.0;

// Rembourrage du panneau de contenu — trois axes, et trois seulement. Le relevé de section est
// catégorique : « x=29 pour les titres de section, x=36 pour tout contrôle indenté, x=62 pour le
// texte qui suit une case. Aucun autre alignement n'a été relevé dans les six onglets. » Rapportés
// au bord du panneau (x=17), cela donne les deux valeurs ci-dessous.
//
// La première version mettait 22px partout, titre compris : le titre et ses contrôles étaient donc
// sur le même axe, ce qui efface le seul signal de niveau que le jeu utilise.
/// Axe des TITRES de section, depuis le bord du panneau.
const PANEL_PAD_TITLE_X: f32 = 12.0;
/// Axe des CONTRÔLES, depuis le bord du panneau. L'écart avec l'axe des titres — 7px — est le
/// retrait qui marque le niveau (relevé de section : « le seul signal de niveau est le retrait de
/// 7px du titre par rapport à ses lignes »).
const PANEL_PAD_CONTROL_X: f32 = 19.0;
/// Rembourrage haut — **19, la valeur du relevé telle quelle** (`releve-modale-options.json`, nœud
/// `panel-content` : « Premier titre à y = 143, soit 19px sous le bord du panneau »).
///
/// Elle a longtemps valu 13, corrigée à la main des 6px que la police réserve au-dessus de l'encre
/// pour les accents de capitale : le relevé cote une encre, et ce qui était positionné était une
/// galley. Depuis l'étape 4 du plan de finalisation, c'est **l'encre du titre qui est réservée**
/// (voir `SECTION_TITLE_INK_HEIGHT`/`SECTION_TITLE_INK_TOP`), et cette cote redevient donc celle du
/// relevé — comme les cotes horizontales, qui n'ont jamais eu besoin de correction.
const PANEL_PAD_TOP: f32 = 19.0;

const TITLE_TEXT: egui::Color32 = egui::Color32::WHITE;

/// Corps du titre de bannière, en serif grasse (`design::text::title_font`).
///
/// **Choix utilisateur du 2026-09-09**, arbitré sur planche de comparaison. La mesure disait 22 :
/// c'est le corps qui reproduit **exactement** l'encre du jeu (21 × 82 px, contre 21 × 82 mesurés
/// sur `interface-options-commandes.png`) ; à 21 on rend 20 × 78, soit un pixel de moins en
/// hauteur. L'utilisateur préfère cette proportion sur la bannière — ce n'est donc pas un écart de
/// mesure à rattraper, ne pas « corriger » à 22.
const TITLE_FONT_SIZE: f32 = 21.0;

/// Corps du titre de section — **le même que le titre de fenêtre**, même serif.
///
/// La première valeur (18) venait d'un rapport calculé de travers : 17 px d'encre mesurés sur
/// « Barres de raccourcis », qui n'a aucun jambage, comparés à 21 px sur « Options », qui en a un.
/// Cap contre cap + jambage — ce ne sont pas les mêmes grandeurs, et le rapport ne voulait rien
/// dire.
///
/// Les relevés tranchent : `releve-modale-options.json` cote le titre de section à ~21 comme le
/// titre de fenêtre, et `releve-options-interface.json` mesure 25 px d'encre sur « Échelle de
/// l'interface (100%) » (accent de capitale + parenthèses descendantes) contre 22 px sur « Thème
/// d'interface personnalisé » (jambage seul) — deux mots du même corps, dont l'encre varie avec ce
/// qu'ils contiennent. Le relevé de section le dit d'ailleurs en toutes lettres : « c'est la ligne
/// de base qui est stable, pas la boîte ».
const SECTION_TITLE_FONT_SIZE: f32 = TITLE_FONT_SIZE;

/// Couleur d'un titre de section — un gris franc, **pas le blanc du titre de fenêtre**. Valeur des
/// deux relevés (`#b8b9ba`), confirmée au pic de luminance sur l'onglet Interface (`#bababb`). La
/// hiérarchie entre les deux niveaux de titre passe par la couleur, pas par le corps.
const SECTION_TITLE_TEXT: egui::Color32 = egui::Color32::from_rgb(0xB8, 0xB9, 0xBA);

// Le champ de chemin est un composant du design system (`design::input`) depuis le 2026-09-10 :
// ses couleurs, son rayon et son retrait de texte ne sont plus des constantes de ce panneau. Les
// trois qui vivaient ici étaient d'ailleurs fausses — fond #1C1E23 au lieu de #0E1115, bord d'1px
// au lieu de 2, rayon 2 au lieu de 4.

/// Translucidité du corps de la modale — **235/255, la valeur qu'avait l'aplat `MODAL_BG` qui
/// peignait ce fond jusqu'au 2026-09-10.**
///
/// La couleur, elle, ne se règle plus ici : le corps est peint depuis `DsTexture::ModalBody`, une
/// texture découpée des captures du jeu (§9 ter du design-system). Un blanc à alpha réduit atténue
/// sans changer la teinte — c'est le mécanisme de teinte documenté par `design::nine_slice::paint`,
/// déjà celui de l'état désactivé des boutons. La fenêtre laisse donc toujours deviner le jeu
/// derrière elle, comme la référence réelle.
const MODAL_BODY_TINT: egui::Color32 = egui::Color32::from_rgba_premultiplied(235, 235, 235, 235);
/// Fond du panneau de contenu (#15181C) — la valeur du relevé (nœud `panel`), neuf niveaux plus
/// sombre que le fond de la modale, ce que le découpage du corps confirme : le panneau n'est pas
/// une surface à part mais le même fond assombri, que la texture de fond traverse (§9 ter du
/// design-system). Il reste peint tant qu'il n'a pas sa propre texture.
///
/// **Ce que nous peignons ici est le PANNEAU DE CONTENU du jeu, pas une « section ».** La
/// distinction n'est pas cosmétique : le relevé de section est formel, « une section n'a ni fond,
/// ni bordure, ni filet de séparation — le seul signal de regroupement est l'espacement ». Ce qui
/// a un fond, un bord et un rayon, c'est le panneau qui contient les sections. Nommer les choses de
/// travers avait produit un encadré arrondi à 18 qui n'existe nulle part dans le jeu.
///
/// L'alpha 230 est une **déviation assumée** : la modale entière est légèrement translucide
/// (`MODAL_BODY_TINT`) et le panneau suit, alors que le jeu est opaque — il n'a pas de jeu
/// derrière lui.
const SECTION_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(0x15, 0x18, 0x1C, 230);

/// Bord du panneau de contenu — presque noir, à peine plus sombre que son fond (relevé : `#131518`
/// contre `#15181c`). Ce n'est pas un trait qu'on voit, c'est ce qui détache le panneau du fond de
/// la modale.
const SECTION_BORDER_COLOR: egui::Color32 = egui::Color32::from_rgb(0x13, 0x15, 0x18);

/// Gouttière entre le champ de chemin et le bouton "Sélectionner le fichier", posés sur la MÊME
/// ligne (demande utilisateur 2026-09-09 : le bouton ne doit plus prendre toute la largeur).
///
/// 10px, la valeur du relevé (`docs/design-system/releve-section-options.json`, nœud `sel-ctrl`) :
/// c'est l'écart que le jeu laisse entre un libellé et le contrôle posé à sa droite, le seul écart
/// intra-ligne qu'il ait été possible de mesurer. Elle figure bien à l'échelle d'espacement de la
/// section (6, 7, 9, **10**, 11, 17, 21, 30, 31), ce n'est pas une valeur inventée pour l'occasion.
const FIELD_TO_BROWSE_GAP: f32 = 10.0;
/// Gouttière entre "Annuler" et "Valider" — **11px, la valeur du relevé telle quelle**
/// (`releve-modale-options.json` : `btn-cancel` finit à x=355, `btn-confirm` commence à x=366).
///
/// Elle valait 12 : une mise à l'échelle des 15px lus à l'œil sur `interface-options-jeu.png`
/// (boutons en x 18..351 et 367..700) rapportés à la largeur de cette modale. Le relevé a mesuré
/// depuis, et il n'y a plus de raison de garder un chiffre dérivé — pas plus qu'il n'y en avait de
/// mettre le chrome à l'échelle, puisqu'il est peint à sa taille native (voir `BANNER_HEIGHT` et
/// `FOOTER_BUTTON_HEIGHT`, deux fois la même leçon).
const FOOTER_GUTTER: f32 = 11.0;
/// Hauteur des boutons « Annuler » / « Valider ». **La hauteur native de leur texture** (338×36),
/// et non une hauteur déduite de leur largeur.
///
/// La première version écrivait `footer_button_width * (36.0 / 338.0)`, pour « préserver les
/// proportions de l'assise ». C'était l'inverse de ce que fait le jeu, et l'inverse de ce à quoi
/// sert un 9-slice : une texture 9-slice existe précisément pour qu'on l'étire en largeur SANS
/// toucher à sa hauteur. Le jeu affiche ce bouton à 333×36 dans une fenêtre de 720 ; notre modale
/// fait 560, ce qui donnait 250×27 — et comme le corps du libellé suit la hauteur
/// (`design::tokens::BUTTON_FONT_SIZE_RATIO`), le texte rétrécissait avec la fenêtre : 10px
/// d'encre au lieu de 13.
///
/// Que ce soit bien la hauteur, et non une mise à l'échelle générale de la modale, se vérifie sur
/// la capture : le bandeau de titre mesure 56px chez le jeu comme chez nous (`BANNER_HEIGHT`),
/// parce qu'il est peint à sa taille native. Le chrome était donc déjà à 100 %, seuls les boutons
/// rétrécissaient.
const FOOTER_BUTTON_HEIGHT: f32 = 36.0;
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

/// Écart entre le titre de section et la ligne de contrôle **pleine largeur** qu'il coiffe.
///
/// 8, la valeur médiane des « 7 à 9 px » relevés (`releve-section-options.json`, note « Rythme »).
/// Le 10 d'avant était une valeur choisie, pas mesurée.
///
/// **Ne pas confondre avec les 30 px du même relevé**, qui séparent un titre de sa première ligne
/// quand celle-ci est INDENTÉE (une case à cocher, un libellé suivi d'un contrôle). Le cas de cette
/// modale est l'autre : « Fichier » coiffe une ligne pleine largeur, le rythme y est quatre fois
/// plus serré. Reprendre 30 décollerait le champ de son titre.
///
/// Rappel du même relevé, à ne pas contourner : le pas de grille **n'est pas régulier**. « Les
/// écarts se groupent autour de 2, 4, 6, puis 11-12, 15-16, 19, 26, 31 et 36. Postuler un pas de
/// 8 px décalerait tout le contenu. » Que cette constante vaille 8 est une coïncidence de son
/// intervalle, pas l'application d'une échelle.
const TITLE_TO_FULL_WIDTH_ROW: f32 = 7.0;

/// Hauteur d'ENCRE du titre de section, pour la serif du design system au corps
/// [`SECTION_TITLE_FONT_SIZE`] — mesurée sur la capture de non-régression (« Fichier », y 155..170).
///
/// Sert à réserver au titre la place de son encre et non celle de sa galley, plus haute des deux
/// côtés — voir le commentaire à l'endroit de l'allocation.
///
/// **Liée à la police ET au corps.** Si l'un des deux change, la re-mesurer sur la capture plutôt
/// que la reporter telle quelle. C'est aussi vrai que le mot compte : « Fichier » n'a ni accent de
/// capitale ni jambage ; un titre qui en porterait aurait une encre plus haute, et devrait alors
/// être mesuré à part.
const SECTION_TITLE_INK_HEIGHT: f32 = 16.0;

/// Écart entre le haut de la galley du titre et le haut de son encre — **la même mesure que celle
/// qui donnait autrefois `PANEL_PAD_TOP` à 13 au lieu de 19** : la place que la police réserve aux
/// accents de capitale au-dessus des minuscules.
const SECTION_TITLE_INK_TOP: f32 = 6.0;

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
/// Textures embarquées du chrome de la modale — chargées UNE FOIS par fenêtre OS (voir
/// `main.rs`/`bin/overlay-ui-x11.rs`, `create_overlay_window`, même principe que
/// `panels::combat_frame::CombatFrame`/`crate::ui_icons::UiIcons`), jamais rechargées à chaque
/// frame. Fichiers sous `crates/overlay-ui/assets/ui/options/`.
///
/// **Ne contient plus aucune texture de bouton** : elles sont au manifeste du design system
/// (`design::assets`), chargées paresseusement par `DesignSystem::get`, et aucun panneau n'a plus à
/// les câbler. Ne reste ici que le chrome propre à cette modale.
pub struct OptionsModalAssets {
    /// `modal-header.png` (720×56) — fond de bannière, peint avec arrondi HAUT uniquement
    /// (`MODAL_RADIUS`) pour épouser le coin de la modale.
    banner: egui::TextureHandle,
}

impl OptionsModalAssets {
    pub fn load(ctx: &egui::Context) -> Self {
        Self {
            banner: load_embedded_texture(
                ctx,
                "options-banner",
                include_bytes!("../../assets/ui/options/modal-header.png"),
            ),
        }
    }
}

/// Décode + charge un PNG embarqué en texture egui — même séquence que
/// `panels::combat_frame::CombatFrame::load`/`crate::ui_icons::UiIcons::load` (`image` crate puis
/// `egui::ColorImage::from_rgba_unmultiplied`), extraite ici pour ne pas la répéter 6 fois.
fn load_embedded_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> egui::TextureHandle {
    let decoded = image::load_from_memory(bytes)
        .expect("asset PNG embarqué invalide — corrompu au build")
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        decoded.as_raw(),
    );
    ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR)
}

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
pub fn show(
    ui: &mut egui::Ui,
    state: &mut OptionsModalState,
    assets: &OptionsModalAssets,
) -> OptionsModalAction {
    let mut action = OptionsModalAction::None;
    let rect = ui.max_rect();

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

    // Corps de la modale — vraie texture du jeu (`modal-body.png`) depuis le 2026-09-10, à la
    // place de l'aplat qui le peignait jusque-là. Elle porte le grain et les hachures d'angle
    // relevés sur les six captures de la fenêtre Options, et ses DEUX ANGLES BAS sont arrondis
    // dans son alpha — inutile de leur passer un `corner_radius`, un `Mesh` egui ne saurait de
    // toute façon pas découper un coin.
    //
    // Peint sous la bannière et non sur tout le rectangle : la texture commence exactement là où
    // `modal-header.png` s'arrête (y=56 sur la capture d'origine), les deux se juxtaposent sans
    // recouvrement. Les angles HAUTS restent donc portés par la bannière seule.
    let body_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.top() + BANNER_HEIGHT),
        rect.max,
    );
    design::DesignSystem::get(ui.ctx()).paint(
        ui.painter(),
        body_rect,
        design::DsTexture::ModalBody,
        MODAL_BODY_TINT,
    );

    // Bannière — vraie texture du jeu (`modal-header.png`), arrondie sur les coins HAUTS
    // uniquement pour épouser le coin de la modale (les coins bas de la texture ne sont jamais
    // visibles, masqués par le corps qui la recouvre en dessous). Étirement uniforme : le rayon de
    // coin natif de cette texture est assez petit (≈12px/720) pour rester imperceptible, voir doc
    // de module.
    let banner_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), BANNER_HEIGHT));
    egui::Image::new(&assets.banner)
        .corner_radius(egui::CornerRadius {
            nw: MODAL_RADIUS,
            ne: MODAL_RADIUS,
            sw: 0,
            se: 0,
        })
        .paint_at(ui, banner_rect);
    // Titre en serif grasse, cerné d'une ombre portée bas-droite (`SHADOW_BOTTOM_RIGHT`) et non
    // d'un contour complet : le fond est ici CONNU (la bannière), on peut donc se permettre de ne
    // cerner qu'un côté — c'est ce que fait le jeu, dont le titre est éclairé depuis le haut-gauche.
    // Un contour complet (l'état précédent) empâtait le mot. Voir `design::text` et `design::fonts`.
    design::text::paint_outlined_text(
        ui,
        banner_rect.center(),
        egui::Align2::CENTER_CENTER,
        "Options",
        design::text::title_font(ui.ctx(), TITLE_FONT_SIZE),
        TITLE_TEXT,
        design::text::SHADOW_BOTTOM_RIGHT,
    );

    let content_rect = egui::Rect::from_min_max(
        egui::pos2(
            rect.left() + BODY_PAD_SIDE,
            banner_rect.bottom() + BODY_PAD_TOP,
        ),
        egui::pos2(
            rect.right() - BODY_PAD_SIDE,
            rect.bottom() - BODY_PAD_BOTTOM,
        ),
    );

    // Barre d'onglets — `design::tabs` depuis le 2026-09-10. Elle était peinte ici à la main : une
    // texture unique de trois segments (`menu-tabs.png`) ÉTIRÉE de sa hauteur native de 44px à 31,
    // des libellés en `FontId::proportional(12.0)` (la police par défaut d'egui, pas celle du design
    // system), une répartition en trois tiers égaux sans rapport avec les segments réels, et aucun
    // clic — « Paramètres » était actif en dur.
    let menu_rect = egui::Rect::from_min_size(
        content_rect.min,
        egui::vec2(content_rect.width(), MENU_HEIGHT),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(menu_rect), |ui| {
        design::tabs(&mut state.tab)
            .entry(OptionsTab::Alertes, "Alertes")
            .enabled(false)
            .entry(OptionsTab::Personnages, "Personnages")
            .enabled(false)
            .entry(OptionsTab::Parametres, "Paramètres")
            .log_name("options-onglets")
            .show(ui);
    });

    // Pied de page — deux boutons du design system, séparés par la gouttière du jeu. Largeur
    // partagée, hauteur native (voir `FOOTER_BUTTON_HEIGHT`).
    let footer_button_width = (content_rect.width() - FOOTER_GUTTER) / 2.0;
    let footer_height = FOOTER_BUTTON_HEIGHT;
    let footer_rect = egui::Rect::from_min_max(
        egui::pos2(content_rect.left(), content_rect.bottom() - footer_height),
        content_rect.max,
    );
    let cancel_rect = egui::Rect::from_min_size(
        footer_rect.min,
        egui::vec2(footer_button_width, footer_height),
    );
    let validate_rect = egui::Rect::from_min_size(
        egui::pos2(footer_rect.right() - footer_button_width, footer_rect.top()),
        egui::vec2(footer_button_width, footer_height),
    );
    if ui
        .put(
            cancel_rect,
            design::button("Annuler")
                .variant(ButtonVariant::Danger)
                .size(ButtonSize::Height(footer_height))
                .width(footer_button_width)
                .log_name("options-annuler"),
        )
        .clicked()
    {
        action = OptionsModalAction::Cancel;
    }
    if ui
        .put(
            validate_rect,
            design::button("Valider")
                .variant(ButtonVariant::Primary)
                .size(ButtonSize::Height(footer_height))
                .width(footer_button_width)
                .log_name("options-valider"),
        )
        .clicked()
    {
        action = OptionsModalAction::Validate(state.path_input.clone());
    }

    // Section "Fichier" — encadré interne au rayon plus prononcé que la modale (`SECTION_RADIUS`),
    // entre le menu et le pied de page.
    let section_rect = egui::Rect::from_min_max(
        egui::pos2(content_rect.left(), menu_rect.bottom() + MENU_GAP),
        egui::pos2(content_rect.right(), footer_rect.top() - FOOTER_GAP),
    );
    ui.painter()
        .rect_filled(section_rect, SECTION_RADIUS, SECTION_BG);
    ui.painter().rect_stroke(
        section_rect,
        SECTION_RADIUS,
        egui::Stroke::new(SECTION_BORDER, SECTION_BORDER_COLOR),
        egui::StrokeKind::Inside,
    );

    // À droite, le jeu réserve 26px pour sa barre de défilement (relevé, nœud `panel`) **même quand
    // elle ne sert pas**. Cette réserve était autrefois une déviation assumée — nous reflétions
    // l'axe des contrôles (19px), faute de barre à y mettre : « reprendre 26px laisserait une marge
    // droite inexpliquée, plus large que la gauche ».
    //
    // Elle ne l'est plus depuis que `design::scroll_area` existe (2026-09-10). Ses trois marges —
    // 6 entre le contenu et la poignée, 6 pour la poignée, 14 jusqu'au bord — font exactement
    // `RESERVE_X`. Le jour où le contenu de l'onglet débordera, il suffira de l'envelopper dans une
    // `design::scroll_area` : la barre tombera pile dans cette réserve, sans qu'un seul pixel de
    // contenu ne bouge. C'est tout l'intérêt de la réserver dès maintenant.
    let inner_rect = egui::Rect::from_min_max(
        egui::pos2(
            section_rect.left() + PANEL_PAD_CONTROL_X,
            section_rect.top() + PANEL_PAD_TOP,
        ),
        egui::pos2(
            section_rect.right() - design::components::scroll_area::RESERVE_X,
            section_rect.bottom() - PANEL_PAD_CONTROL_X,
        ),
    );
    /// Retrait du titre par rapport à ses contrôles — voir `PANEL_PAD_CONTROL_X`.
    const TITLE_OUTDENT: f32 = PANEL_PAD_CONTROL_X - PANEL_PAD_TITLE_X;
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
        // **Aucun espacement implicite dans ce panneau.** egui glisse `item_spacing.y` (3px par
        // défaut) entre deux widgets empilés : les écarts relevés du jeu s'en trouvaient tous
        // majorés de 3, et un `add_space` y lisait autre chose que ce qu'il produisait. Tout écart
        // vertical est désormais une constante nommée de ce fichier, et rien d'autre.
        ui.spacing_mut().item_spacing.y = 0.0;

        // Même traitement que le titre de bannière (serif grasse + ombre bas-droite), au corps
        // dicté par le rapport d'encre du jeu entre ses deux niveaux de titre. Peint à la main
        // plutôt que via `ui.label` : un libellé egui ne sait pas se cerner, et le procédé doit
        // rester le même que celui de la bannière. L'espace réservé inclut le pixel d'ombre.
        let section_font = design::text::title_font(ui.ctx(), SECTION_TITLE_FONT_SIZE);
        let section_galley = ui.painter().layout_no_wrap(
            "Fichier".to_owned(),
            section_font.clone(),
            SECTION_TITLE_TEXT,
        );
        // **L'espace réservé est celui de l'ENCRE, pas celui de la galley** (2026-09-10, étape 4 du
        // plan de finalisation). C'est ce qui permet aux deux cotes verticales du relevé
        // (`PANEL_PAD_TOP` au-dessus, `TITLE_TO_FULL_WIDTH_ROW` en dessous) d'être ses valeurs
        // telles quelles, au lieu de valeurs corrigées à la main d'une marge de police.
        //
        // Une galley est plus haute que son encre des deux côtés : la police y réserve la place des
        // accents de capitale au-dessus et des jambages en dessous, que « Fichier » n'utilise ni
        // l'un ni l'autre. Réserver la galley entière ajoutait donc ~14px invisibles entre le titre
        // et sa ligne, sur 7 relevés.
        let section_title_rect = ui
            .allocate_space(egui::vec2(
                section_galley.size().x + 1.0,
                SECTION_TITLE_INK_HEIGHT,
            ))
            .1;
        design::text::paint_outlined_text(
            ui,
            section_title_rect.left_top() - egui::vec2(TITLE_OUTDENT, SECTION_TITLE_INK_TOP),
            egui::Align2::LEFT_TOP,
            "Fichier",
            section_font,
            SECTION_TITLE_TEXT,
            design::text::SHADOW_BOTTOM_RIGHT,
        );
        ui.add_space(TITLE_TO_FULL_WIDTH_ROW);

        // Le champ et le bouton partagent une ligne : le bouton prend sa largeur naturelle
        // (libellé + marges du design system, voir `Button::desired_size`) et le champ occupe tout
        // le reste. C'est le bouton qui commande, pas l'inverse — une largeur figée pour lui
        // désaccorderait le couple dès que le libellé ou la fenêtre changent.
        let browse = design::button("Parcourir")
            .variant(ButtonVariant::Secondary)
            .size(ButtonSize::Height(ROW_HEIGHT))
            .log_name("options-parcourir");
        let browse_width = browse.desired_size(ui).x;
        let row_rect = ui
            .allocate_space(egui::vec2(inner_rect.width(), ROW_HEIGHT))
            .1;
        // Plancher à zéro : si la section devenait plus étroite que le bouton, un rectangle de
        // largeur négative serait inversé par egui et peint n'importe où. Zéro le rend invisible,
        // ce qui se voit sur une capture — c'est la règle du contrat de composant.
        let field_width = (row_rect.width() - browse_width - FIELD_TO_BROWSE_GAP).max(0.0);
        // Le champ garde sa hauteur native (25px) et se centre sur la ligne, que le bouton fixe à
        // 36 : le jeu compose réellement des lignes où le champ est plus bas que ce qui l'accompagne
        // (voir `design::components::input`), et la règle du design system est que la hauteur d'un
        // composant est celle de sa référence — pas celle de son voisin.
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
                    .width(inner_rect.width())
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
