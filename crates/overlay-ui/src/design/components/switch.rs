//! **Switch à cases** du design system Wakfu — le sélecteur exclusif du jeu (choix du genre :
//! ♂ / ♀), une case par valeur, **une seule active à la fois**. Composant **feuille** (§1 du
//! contrat).
//!
//! ```ignore
//! use overlay_ui::design::{self, DsIcon};
//!
//! design::switch(&mut personnage.genre)
//!     .slot(Genre::Masculin, "Masculin").icon(DsIcon::Male)
//!     .slot(Genre::Feminin, "Féminin").icon(DsIcon::Female)
//!     .log_name("personnages.genre")
//!     .show(ui);
//!
//! // Trois positions — le sélecteur de grandeur du panneau Combat.
//! design::switch(&mut metric)
//!     .slot(CombatMetric::Damage, "Dégâts infligés")
//!     .slot(CombatMetric::Armor, "Armure donnée")
//!     .slot(CombatMetric::Heal, "Soins prodigués")
//!     .show(ui);
//! ```
//!
//! La valeur sélectionnée vit chez l'appelant, comme pour `design::tabs` ; `Response::changed()` dit
//! à quelle frame elle a bougé. Le composant ne décide jamais de ce que la sélection déclenche.
//!
//! ## Pourquoi pas un `tabs`
//!
//! C'est la question du contrat (« le composant existe-t-il déjà ? »), et la réponse est non : ce
//! n'est pas le même élément du jeu. Une barre d'onglets a un fond sombre `#363734` et un onglet
//! actif kaki au libellé **blanc**, un séparateur en dégradé, un cadre arrondi seulement aux bouts
//! d'une barre. Le switch a un **cadre** propre (liseré `#221f24`, rayon 6 aux quatre coins), un
//! séparateur plat, et ses glyphes changent de couleur — doré sur la case active, gris sur
//! l'inactive — là où l'onglet actif blanchit le sien. Six textures dédiées, tirées de deux
//! captures (`switch-first-slot-active.png` et `switch-second-slot-active.png`), aucune partagée
//! avec les onglets.
//!
//! ## Deux cases relevées, *n* cases servies
//!
//! Le jeu n'a capturé qu'un switch à deux cases. Une case du **milieu** n'a ni coin arrondi ni
//! liseré latéral : ses deux textures (`switch-slot-active.png`, `switch-slot-inactive.png`) sont
//! les 40px de remplissage des cases d'extrémité, coin redressé — la même dérivation que
//! `tab-active.png`. C'est ce qui permet un switch à trois positions (Dégâts / Armure / Soins du
//! panneau Combat) sans rien inventer d'autre que « le milieu ressemble aux bouts ». Une case
//! inactive du milieu n'a pas d'ombre intérieure : sur les captures, l'ombre est toujours du côté du
//! liseré extérieur, et une case sans liseré n'en porte pas.
//!
//! Deux cases au moins : un switch à une case n'est pas un switch, il n'est pas peint et le dit une
//! fois au journal — comme une barre d'onglets vide.
//!
//! ## Ce que le glyphe porte, et ce que le libellé porte
//!
//! Le pictogramme est le cas nominal — c'est le seul que le jeu montre. Le libellé reste
//! obligatoire pour les mêmes raisons que sur un onglet à pictogramme : il est l'**infobulle** de
//! la case et sa **ligne de journal**. Sans pictogramme, il est peint dans la case, au corps des
//! libellés du jeu ([`tokens::SWITCH_FONT_SIZE`]) — un repli **non vérifié contre une capture**,
//! le jeu n'ayant pas de switch texte dans les interfaces relevées.
//!
//! ## Les mesures
//!
//! Relevé `ui-blueprint` du 2026-09-16 sur l'asset générique 88 × 44 :
//!
//! | Grandeur | Valeur |
//! | --- | --- |
//! | Cadre | 88 × 44, liseré 2px `#221f24`, rayon 6 (porté par l'alpha des textures d'extrémité) |
//! | Case active | 40 × 40, kaki `#635a47`, biseau clair 2px en haut et en bas |
//! | Case inactive | 42 × 40, gris-brun `#514b44`, ombre intérieure 2px côté liseré |
//! | Séparateur | 2px, `#312d2d`, entre les deux liserés (y 2..42) |
//! | Glyphes | ♂ 14 × 14 et ♀ 10 × 16, à leur taille native, centrés dans leur case |
//! | Glyphe actif / inactif | `#f4d89f` / `#a9a5a2` |
//!
//! **La case inactive est 2px plus large que l'active**, et le séparateur se déplace donc de 2px
//! selon l'état (x 42–44 ou 44–46). Le composant donne la même largeur à toutes les cases
//! (43 dans la capture, [`tokens::SWITCH_SLOT_WIDTH`] ; 36 à l'écran, voir plus bas) et laisse
//! le 9-slice absorber l'écart — un pixel de chaque côté, invisible.
//!
//! ## Le survol — la case survolée devient une case active
//!
//! Capture du 2026-09-16 (`switch-first-slot-active-and-second-slot-hover.png`, ♂ actif, souris
//! sur ♀) : la case inactive survolée prend **tout** l'aspect de la case active — fond kaki
//! `#635a47`, biseau clair de 2px en haut et en bas, glyphe doré, plus d'ombre intérieure côté
//! liseré. Écart moyen entre la case survolée et la case active de la référence : 1,0/255 (11,5
//! contre la case inactive). Le switch montre alors deux cases actives, indiscernables : c'est le
//! jeu, on ne l'améliore pas. Aucune texture de plus, `Hovered` peint les textures actives. (La
//! première version dorait le glyphe seul sans toucher au fond — inventé faute de capture, et
//! faux.)
//!
//! ## Ce que le jeu n'a pas montré
//!
//! - **L'état désactivé.** Aucune capture. Fonds atténués comme un bouton désactivé, glyphes
//!   [`tokens::TEXT_DISABLED`] ; la case sélectionnée reste reconnaissable à son fond.
//! - **Le milieu.** Voir plus haut : dérivé des extrémités.
//! - **Une taille autre que 88 × 44.** Le composant ne se peint jamais aux dimensions de sa
//!   capture : voir « 36 × 36 dans les deux variantes » ci-dessous. `Switch::scale` multiplie
//!   ce rapport de réduction (cases, séparateur, liseré, biseaux et glyphes ensemble, comme le
//!   jeu quand on change l'échelle de son interface) ; `Switch::width` et `Switch::height`
//!   imposent une dimension, le 9-slice à l'échelle étirant le corps des cases sur ce qui reste.
//! - **Des glyphes en couleurs.** Le jeu teinte ses glyphes (doré / gris) ; un glyphe
//!   `DsIcon::native_color` est peint tel quel sur la case active et atténué
//!   ([`tokens::ICON_NATIVE_DIM`]) ailleurs — voir la catégorie `couleur` de `design::icons`.
//!
//! ## 36 × 36 dans les deux variantes (2026-09-16)
//!
//! **Décision utilisateur** : « passer les boutons du composant switch en 36 par 36 de manière
//! générique ». Une case est un carré de [`tokens::SWITCH_SLOT_SIZE`] (36, le côté d'un bouton
//! icône), quelle que soit sa matière — le composant le fait pour tous ses appelants, aucun n'a
//! d'échelle à régler. Un switch à deux cases fait 74 px en `Frame` (gouttière de 2), 70 en
//! `FirstPlan` (chevauchement de 2) ; 112 et 104 à trois cases.
//!
//! La variante premier plan y était déjà (son socle EST un bouton icône de 36). La variante cadre,
//! elle, se peignait aux 43 × 44 de sa capture — « le rendu dans le jeu est relativement
//! imposant » —, et les deux contrôles ne tombaient pas sur la même grille. Elle y est ramenée par
//! un **9-slice à l'échelle** (`DesignSystem::paint_scaled`, rapport `36 / 44` =
//! [`tokens::SWITCH_SLOT_SIZE`] / [`tokens::SWITCH_HEIGHT`]) : les marges figées de la texture —
//! coins, liseré, biseaux — sont peintes à 82 % de leur taille, comme le jeu les réduit quand on
//! baisse l'échelle de son interface, et le corps s'étire sur ce qui reste. Séparateur et liseré
//! de gouttière suivent le même rapport (2 px restent 2 une fois arrondis au pixel). **Les
//! glyphes, eux, ne bougent pas** : plafond commun de 16 aux deux variantes ([`glyph_size`]), le
//! ♂ reste à 14 et le ♀ à 16 — c'est la taille de glyphe commune que le lot premier plan a
//! arrêtée (voir « la règle d'étanchéité »), et un sélecteur de genre aux glyphes plus petits que
//! ceux du panneau Combat, sur la même grille de 36, se serait vu.
//!
//! Les deux autres voies ont été essayées avant, sur le panneau Combat, et écartées : le 9-slice
//! ordinaire (`Switch::height` seul) garde ses 6 px hauts et bas à 1:1 et ne comprime que le
//! dégradé — « on a l'impression d'avoir compressé le switch » —, et la texture entière en un
//! quad réduit tout mais ne sait plus s'étirer en largeur sans déformer ses coins. Le 9-slice à
//! l'échelle fait les deux : à la largeur native, il vaut le quad ; étiré, il vaut le 9-slice.
//!
//! La texture du jeu reste à 88 × 44 dans `assets/design-system/`, et la galerie en montre une
//! rangée à cette taille (`scale(44 / 36)`, largeur 88 imposée) pour la comparaison à la capture.
//!
//! ## Deux matières pour un même contrôle — [`SwitchVariant`]
//!
//! Le cadre décrit ci-dessus est la variante [`SwitchVariant::Frame`], celle du sélecteur de genre
//! du jeu, et elle reste le défaut. [`SwitchVariant::FirstPlan`] peint les mêmes cases sur le
//! **socle de bouton icône de premier plan** (`button-icon-first-plan.png` et son `-hover`, les
//! textures d'`IconContext::FirstPlan`) — demande utilisateur du 2026-09-16 pour les deux switches
//! du panneau Combat.
//!
//! Ce n'est pas un habillage de plus sur un coup de tête : ces deux switches sont les seuls de
//! l'overlay à flotter **par-dessus le jeu**, à côté des boutons icône du carré de contrôle du
//! Suivi, et non dans une fenêtre du design system. Le cadre kaki du sélecteur de genre y est un
//! meuble d'interface posé sur la scène ; le socle de premier plan est précisément la matière que
//! le jeu emploie là.
//!
//! Ce que la variante change, et rien d'autre :
//!
//! | | `Frame` | `FirstPlan` |
//! | --- | --- | --- |
//! | Fond d'une case | six textures de cadre, selon sa position | **un socle carré**, le même pour toutes |
//! | Gouttière | liseré + séparateur peints dedans | **rien** — chaque socle porte ses quatre coins |
//! | Case native | 36 × 36 ([`tokens::SWITCH_SLOT_SIZE`]), cadre du jeu à l'échelle 36/44 | **36 × 36** ([`tokens::SWITCH_FIRST_PLAN_SIZE`]), socle tel quel |
//! | Glyphe | sa taille native sous 16px, à l'échelle 36/44 | la grille du bouton icône (18 sur 36) |
//! | Glyphe actif / inactif | doré / gris chaud | **blanc** / gris froid du premier plan |
//!
//! ## La règle d'étanchéité — ce lot ne concerne QUE `FirstPlan`
//!
//! **Exigence utilisateur explicite (2026-09-16), et elle prime sur toute élégance de code** : le
//! chevauchement des cases, le trait de jointure, le liseré de la case choisie, son halo, et le
//! survol qui allume le glyphe avec le socle appartiennent **strictement** à
//! [`SwitchVariant::FirstPlan`]. Le switch standard — celui du sélecteur de genre de la fenêtre
//! Personnages — doit rendre **exactement** ce qu'il rendait avant ce lot, au pixel près.
//!
//! **Une exception, venue APRÈS ce lot et décidée pour elle-même** : la case de 36 (« passer les
//! boutons du composant switch en 36 par 36 de manière générique », voir plus haut). Elle change
//! la grille du composant entier, cadre compris — ce n'est pas le premier plan qui déteint, et
//! les captures de la fenêtre Personnages ont bougé pour cette seule raison. Tout le reste de la
//! liste ci-dessous reste étanche.
//!
//! Concrètement, tout ce qui diffère passe par un `match self` sur la variante, jamais par une
//! valeur partagée qu'on « ajusterait » :
//!
//! | Ce qui change | Où l'aiguillage se fait |
//! | --- | --- |
//! | gouttière positive / chevauchement | [`Switch::separator_width`] |
//! | liseré du cadre et séparateur peints dans la gouttière | `Widget::ui`, sous `== Frame` |
//! | trait de jointure | `Widget::ui`, sous `== FirstPlan` |
//! | liseré de la case choisie et son halo | `Widget::ui`, après la boucle, sous `== FirstPlan` |
//! | teinte du glyphe par état | [`SwitchVariant::glyph_color`] |
//! | fond d'une case | [`SwitchVariant::slot_texture`] |
//! | taille native d'une case | [`SwitchVariant::native_slot_size`] |
//!
//! **La taille du glyphe, elle, est commune aux deux** ([`glyph_size`], plafond
//! [`tokens::SWITCH_ICON_SIZE`]) — et c'est justement ce qu'elle était avant ce lot : `FirstPlan`
//! a d'abord suivi la grille du bouton icône (18 sur 36), qui grossissait de 68 % les cinq glyphes
//! du panneau Combat, faute d'étalon d'encre sur les glyphes en couleurs. Le retour de peindre
//! `Frame` comme avant est donc nul.
//!
//! Deux tests verrouillent cette frontière en bas de fichier, et les captures de la fenêtre
//! Personnages en sont la preuve visuelle : elles n'ont pas bougé d'un pixel.
//!
//! **Le piège de cette variante** : le jeu n'a qu'une texture survolée, et c'est elle qui sert de
//! case active — comme dans `Frame`, comme dans `design::tabs`. L'actif et le survolé partagent
//! donc leur socle, et **seule la teinte du glyphe les distingue** : blanc sur la case choisie
//! (c'est-à-dire, sur un glyphe en couleurs, sa couleur vraie), or au survol, gris au repos. Un
//! portage qui ne jouerait que sur le socle rendrait les deux indiscernables dès que la souris
//! passe sur une case non choisie.
//!

use egui::emath::GuiRounding as _;
use egui::{Align2, Color32, Response, Sense, Ui, Vec2, Widget};

use crate::design::{assets::DsTexture, text, tokens, DesignSystem, DsIcon};

/// État visuel d'une case. Quatre et non trois : une case porte en plus la notion d'être **celle
/// qui est sélectionnée** — la même extension que `TabState`.
///
/// Pas d'état « pressé » (décision utilisateur 2026-09-09, commune à tous les composants) :
/// l'appui retire l'apparence survolée, elle revient au relâchement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchState {
    Idle,
    Hovered,
    Active,
    Disabled,
}

impl SwitchState {
    /// Teinte du glyphe — ou du libellé de repli.
    fn glyph_color(self) -> Color32 {
        match self {
            SwitchState::Active | SwitchState::Hovered => tokens::SWITCH_ICON_ACTIVE,
            SwitchState::Idle => tokens::SWITCH_ICON_INACTIVE,
            SwitchState::Disabled => tokens::TEXT_DISABLED,
        }
    }

    /// Teinte d'un glyphe **en couleurs** (`DsIcon::native_color`) : blanc — c'est-à-dire tel
    /// quel — sur la case active ou survolée, atténué ailleurs. Le désactivé cumule les deux
    /// atténuations (celle-ci et [`DISABLED_TINT`] sur le fond), comme un glyphe blanc y perd à
    /// la fois sa teinte et son fond.
    fn native_glyph_color(self) -> Color32 {
        match self {
            SwitchState::Active | SwitchState::Hovered => Color32::WHITE,
            SwitchState::Idle | SwitchState::Disabled => tokens::ICON_NATIVE_DIM,
        }
    }
}

/// La matière sur laquelle les cases sont peintes — voir « deux matières pour un même contrôle »
/// dans la doc de module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SwitchVariant {
    /// Le cadre du sélecteur de genre du jeu : six textures, un liseré, un séparateur.
    #[default]
    Frame,
    /// Le socle de bouton icône de premier plan (`button-icon-first-plan.png` / `-hover`), un par
    /// case — la matière de ce que le jeu pose PAR-DESSUS la scène.
    FirstPlan,
}

impl SwitchVariant {
    /// Côté natif d'une case — **36 dans les deux variantes** (voir « 36 × 36 dans les deux
    /// variantes » dans la doc de module).
    fn native_slot_size(self) -> Vec2 {
        match self {
            SwitchVariant::Frame => Vec2::splat(tokens::SWITCH_SLOT_SIZE),
            SwitchVariant::FirstPlan => Vec2::splat(tokens::SWITCH_FIRST_PLAN_SIZE),
        }
    }

    /// Rapport entre la case servie et la texture qui la peint : `36 / 44` pour le cadre du jeu,
    /// capturé à 44 de haut ; 1 pour le socle de premier plan, qui fait déjà 36. C'est l'échelle
    /// des marges du 9-slice, du séparateur, du liseré et des glyphes — tout ce qui doit se
    /// réduire avec la case pour qu'elle reste le switch du jeu en plus petit.
    fn texture_scale(self) -> f32 {
        match self {
            SwitchVariant::Frame => tokens::SWITCH_SLOT_SIZE / tokens::SWITCH_HEIGHT,
            SwitchVariant::FirstPlan => 1.0,
        }
    }

    /// Fond d'une case. Le survolé prend le fond de l'actif dans les DEUX variantes — c'est ce que
    /// le jeu fait, et c'est pourquoi la teinte du glyphe porte seule la distinction.
    fn slot_texture(self, position: Position, state: SwitchState, selected: bool) -> DsTexture {
        let active = match state {
            SwitchState::Active | SwitchState::Hovered => true,
            SwitchState::Idle => false,
            // Désactivé, la case sélectionnée garde son fond : c'est ce qui la laisse
            // reconnaissable.
            SwitchState::Disabled => selected,
        };
        match self {
            // Un socle de premier plan porte ses quatre coins : sa position dans la rangée ne
            // change rien, contrairement aux cases d'un cadre continu.
            SwitchVariant::FirstPlan if active => DsTexture::ButtonIconFirstPlanHover,
            SwitchVariant::FirstPlan => DsTexture::ButtonIconFirstPlan,
            SwitchVariant::Frame => match (position, active) {
                (Position::First, true) => DsTexture::SwitchSlotActiveFirst,
                (Position::First, false) => DsTexture::SwitchSlotInactiveFirst,
                (Position::Middle, true) => DsTexture::SwitchSlotActive,
                (Position::Middle, false) => DsTexture::SwitchSlotInactive,
                (Position::Last, true) => DsTexture::SwitchSlotActiveLast,
                (Position::Last, false) => DsTexture::SwitchSlotInactiveLast,
            },
        }
    }

    /// Teinte du glyphe (ou du libellé de repli) pour cet état.
    ///
    /// `Frame` rend les teintes mesurées sur le sélecteur du jeu. `FirstPlan` reprend le couple du
    /// contexte premier plan ([`tokens::ICON_TINT`] → [`tokens::ICON_TINT_HOVER`], mesuré sur
    /// `menu-button-icon-first-plan.png`) et **blanchit la case choisie** : sans cela elle serait
    /// indiscernable de la case survolée, qui partage son socle.
    fn glyph_color(self, state: SwitchState, native_color: bool) -> Color32 {
        match (self, native_color) {
            (SwitchVariant::Frame, false) => state.glyph_color(),
            (SwitchVariant::Frame, true) => state.native_glyph_color(),
            // Un glyphe EN COULEURS ne se teinte pas, il s'allume : une teinte egui multiplie, le
            // blanc est donc sa couleur vraie et l'or le dorerait. **Le survol l'allume comme il
            // allume le socle** (retour utilisateur 2026-09-16) : n'éclaircir que le fond laissait
            // une icône éteinte sur un socle allumé, « un effet de décalage perturbant ». Ce que la
            // case CHOISIE a de plus est ailleurs — son liseré doré (voir `Switch::selected_rim`).
            (SwitchVariant::FirstPlan, true) => match state {
                SwitchState::Active | SwitchState::Hovered => Color32::WHITE,
                SwitchState::Idle | SwitchState::Disabled => tokens::ICON_NATIVE_DIM,
            },
            (SwitchVariant::FirstPlan, false) => match state {
                // Même raison : le survol amène le glyphe à sa teinte pleine, en même temps que le
                // socle. Le liseré doré, lui, dit laquelle est choisie.
                SwitchState::Active | SwitchState::Hovered => tokens::SWITCH_FIRST_PLAN_ICON_ACTIVE,
                SwitchState::Idle => tokens::ICON_TINT,
                SwitchState::Disabled => tokens::ICON_TINT_DISABLED,
            },
        }
    }

    /// Taille de peinture d'un glyphe dans une case de côté `side`.
    ///
    /// `Frame` peint le glyphe à sa taille de fichier tant qu'il tient dans le carré de 16 du jeu
    /// (voir [`glyph_size`]). `FirstPlan` suit la grille de son socle — l'étalon d'encre du
    /// manifeste ramené à 18 sur 36, la même règle que `design::icon_button`, sans quoi deux
    /// contrôles peints sur le MÊME socle porteraient des glyphes de tailles différentes.
    fn glyph_size(self, design: &DesignSystem, icon: DsIcon) -> Vec2 {
        // Le même plafond dans les deux variantes ([`tokens::SWITCH_ICON_SIZE`], 16) : un glyphe
        // qui y tient garde sa taille de fichier, les autres y sont ramenés en gardant leur
        // rapport. La grille du bouton icône (18 sur 36) grossissait de 68 % les cinq glyphes du
        // panneau Combat, qui n'ont pas d'étalon d'encre — retour utilisateur du 2026-09-16.
        glyph_size(design.icon_native_size(icon))
    }
}

/// Où la case se trouve dans le switch — c'est ce qui décide de ses coins arrondis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Position {
    First,
    Middle,
    Last,
}

impl Position {
    fn of(index: usize, count: usize) -> Self {
        if index == 0 {
            Position::First
        } else if index + 1 == count {
            Position::Last
        } else {
            Position::Middle
        }
    }
}

/// Atténuation des fonds d'un switch désactivé — la même que celle d'un bouton désactivé
/// (`button::DISABLED_TINT`) : un multiplicateur d'alpha, pour que l'état se lise aussi sur une
/// capture statique.
const DISABLED_TINT: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 190);

struct Slot<T> {
    value: T,
    label: String,
    /// Pictogramme qui **remplace** le libellé au rendu — voir [`Switch::icon`].
    icon: Option<DsIcon>,
    /// Force l'état peint de CETTE case — voir [`Switch::preview_state`].
    forced_state: Option<SwitchState>,
}

/// Construit un switch sur `selected`. Point d'entrée unique — voir la doc de module.
pub fn switch<T: PartialEq + Copy>(selected: &mut T) -> Switch<'_, T> {
    Switch::new(selected)
}

pub struct Switch<'a, T> {
    selected: &'a mut T,
    slots: Vec<Slot<T>>,
    /// La matière des cases — voir [`SwitchVariant`].
    variant: SwitchVariant,
    /// Largeur totale imposée — sinon [`Switch::natural_width`].
    width: Option<f32>,
    /// Hauteur imposée — sinon [`tokens::SWITCH_HEIGHT`].
    height: Option<f32>,
    /// Rapport d'échelle homothétique — voir [`Switch::scale`]. 1 = la taille du jeu.
    scale: f32,
    enabled: bool,
    log_name: Option<String>,
}

impl<'a, T: PartialEq + Copy> Switch<'a, T> {
    pub fn new(selected: &'a mut T) -> Self {
        Self {
            selected,
            slots: Vec::new(),
            variant: SwitchVariant::default(),
            width: None,
            height: None,
            scale: 1.0,
            enabled: true,
            log_name: None,
        }
    }

    /// Ajoute une case à droite des précédentes. L'ordre d'appel est l'ordre d'affichage ; il en
    /// faut **deux au moins**.
    pub fn slot(mut self, value: T, label: impl Into<String>) -> Self {
        self.slots.push(Slot {
            value,
            label: label.into(),
            icon: None,
            forced_state: None,
        });
        self
    }

    /// Donne un pictogramme à **la dernière case déclarée** : il remplace son libellé au rendu, et
    /// le libellé devient son infobulle. Sans effet avant le premier `slot`.
    pub fn icon(mut self, icon: DsIcon) -> Self {
        if let Some(last) = self.slots.last_mut() {
            last.icon = Some(icon);
        }
        self
    }

    /// Matière des cases — cadre du jeu par défaut, socle de premier plan pour un switch qui
    /// flotte PAR-DESSUS la scène (les deux du panneau Combat). Voir [`SwitchVariant`].
    pub fn variant(mut self, variant: SwitchVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Largeur totale imposée. Les cases se la partagent à égalité, séparateurs déduits. Sans
    /// elle, chaque case fait [`tokens::SWITCH_SLOT_SIZE`] de large — 74px pour deux cases, dans
    /// les deux variantes. Le 9-slice à l'échelle étire le corps des cases, coins et liseré
    /// gardés.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Hauteur imposée — sinon [`tokens::SWITCH_SLOT_SIZE`]. Le 9-slice à l'échelle étire le
    /// corps des cases ; pas en dessous des marges réduites du 9-slice (10px) plus un glyphe.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Réduit (ou agrandit) **tout le switch dans le même rapport** — cases, séparateur, liseré,
    /// biseaux et glyphes — comme le jeu quand on change l'échelle de son interface. 1 = la case
    /// de 36 ; le rapport se multiplie à celui de la texture (voir « 36 × 36 dans les deux
    /// variantes » dans la doc de module : `44 / 36` rend au cadre les dimensions de sa capture).
    /// Les dimensions sont arrondies au pixel. `width` et `height`, s'ils sont donnés, priment
    /// sur la taille ainsi calculée.
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Active ou désactive **le switch entier** — toutes les cases ensemble, il n'y a pas de sens
    /// à n'en désactiver qu'une.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Force l'état peint de **la dernière case déclarée**, sans passer par l'interaction —
    /// réservé à la galerie de contrôle et aux captures, où aucun pointeur ne survole quoi que ce
    /// soit. Même rôle que `Button::preview_state`.
    pub fn preview_state(mut self, state: SwitchState) -> Self {
        if let Some(last) = self.slots.last_mut() {
            last.forced_state = Some(state);
        }
        self
    }

    /// Nom d'instance pour la journalisation (défaut : `"switch"`). À renseigner dès que deux
    /// switches coexistent, sinon leurs lignes de journal sont indiscernables.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Alias d'`ui.add(self)`, plus lisible en bout de chaîne.
    pub fn show(self, ui: &mut Ui) -> Response {
        ui.add(self)
    }

    /// L'échelle à laquelle la texture d'une case est peinte : celle de l'appelant
    /// ([`Switch::scale`]) fois celle de la variante ([`SwitchVariant::texture_scale`]). C'est
    /// le rapport des marges du 9-slice, du séparateur, du liseré et des glyphes — pas celui de
    /// la case, qui vaut `36 × scale` dans les deux variantes.
    fn effective_scale(&self) -> f32 {
        self.scale * self.variant.texture_scale()
    }

    /// Largeur du séparateur à l'échelle — au pixel, jamais sous 1 : une gouttière de 1,6px
    /// serait peinte floue sur deux colonnes.
    fn separator_width(&self) -> f32 {
        match self.variant {
            // Un chevauchement est une gouttière négative : toute la géométrie (largeur totale,
            // largeur de case, position de chaque case) en découle sans autre changement.
            SwitchVariant::FirstPlan => -tokens::SWITCH_FIRST_PLAN_OVERLAP,
            // Le séparateur du jeu, réduit avec le cadre (36/44) puis à l'échelle de l'appelant.
            SwitchVariant::Frame => (tokens::SWITCH_SEPARATOR_WIDTH * self.effective_scale())
                .round()
                .max(1.0),
        }
    }

    /// Largeur sans contrainte : `n` cases au côté natif (36, à l'échelle de l'appelant) et
    /// `n − 1` séparateurs.
    fn natural_width(&self) -> f32 {
        let n = self.slots.len() as f32;
        n * (self.variant.native_slot_size().x * self.scale).round()
            + (n - 1.0).max(0.0) * self.separator_width()
    }

    /// Taille que le switch occupera, sans le dessiner.
    pub fn desired_size(&self) -> Vec2 {
        Vec2::new(
            self.width.unwrap_or_else(|| self.natural_width()),
            self.height
                .unwrap_or_else(|| (self.variant.native_slot_size().y * self.scale).round()),
        )
    }
}

/// Taille de peinture d'un glyphe : **native s'il tient dans le carré**, réduite sinon.
///
/// C'est la nuance de [`tokens::SWITCH_ICON_SIZE`] : le jeu peint ses deux glyphes à leur taille
/// de fichier (14 et 16 de haut), un étalon commun grossirait le ♂. Seul un glyphe plus grand que
/// le carré — un pictogramme de 22px emprunté à une autre famille — est ramené dedans, en gardant
/// son rapport (`glyph_fit`).
fn glyph_size(native: Vec2) -> Vec2 {
    if native.x.max(native.y) <= tokens::SWITCH_ICON_SIZE {
        native
    } else {
        super::icon_button::glyph_fit(native, tokens::SWITCH_ICON_SIZE)
    }
}

impl<T: PartialEq + Copy> Widget for Switch<'_, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let name = self.log_name.clone().unwrap_or_else(|| "switch".to_owned());

        // Moins de deux cases n'est pas un cas de mise en page, c'est un appel oublié : rien n'est
        // peint, et on le dit — une fois, sinon la ligne reviendrait à chaque frame.
        if self.slots.len() < 2 {
            let (_, response) = ui.allocate_exact_size(Vec2::ZERO, Sense::hover());
            let warned_id = response.id.with("ds-switch-incomplet");
            let already = ui.data_mut(|d| {
                let seen = d.get_temp::<bool>(warned_id).unwrap_or(false);
                d.insert_temp(warned_id, true);
                seen
            });
            if !already {
                tracing::warn!(
                    component = "switch",
                    name,
                    cases = self.slots.len(),
                    "switch incomplet : il faut deux cases au moins"
                );
            }
            return response;
        }

        let size = self.desired_size();
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::hover());
        let design = DesignSystem::get(ui.ctx());
        // Un appui de souris retire l'apparence survolée partout dans l'overlay — même condition
        // que `design::button`, `design::icon_button` et `design::tabs`.
        let pointer_down = ui.input(|i| i.pointer.any_down());
        let count = self.slots.len();
        let separator_width = self.separator_width();
        let effective_scale = self.effective_scale();
        let slot_width = (rect.width() - separator_width * (count - 1) as f32) / count as f32;
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        // Atténuation des fonds ET de la gouttière quand le switch est désactivé — la même
        // opacité partout, sinon la gouttière resterait la seule chose franche du cadre.
        let (tint, dim) = if self.enabled {
            (Color32::WHITE, 1.0)
        } else {
            (DISABLED_TINT, DISABLED_TINT.a() as f32 / 255.0)
        };

        let mut clicked: Option<(usize, T)> = None;
        // Le liseré de la case choisie est peint APRÈS la boucle : en `FirstPlan` les socles se
        // chevauchent, celui de droite recouvrirait sinon le liseré de son voisin de gauche.
        let mut selected_rect: Option<egui::Rect> = None;
        for (index, slot) in self.slots.iter().enumerate() {
            let left = rect.left() + index as f32 * (slot_width + separator_width);
            let slot_rect = egui::Rect::from_min_size(
                egui::pos2(left, rect.top()),
                Vec2::new(slot_width, rect.height()),
            );
            let slot_response = ui.interact(slot_rect, response.id.with(index), sense);

            let selected = *self.selected == slot.value;
            let state = slot.forced_state.unwrap_or(if !self.enabled {
                SwitchState::Disabled
            } else if selected {
                SwitchState::Active
            } else if slot_response.hovered() && !pointer_down {
                SwitchState::Hovered
            } else {
                SwitchState::Idle
            });

            if ui.is_rect_visible(slot_rect) {
                let texture =
                    self.variant
                        .slot_texture(Position::of(index, count), state, selected);
                // 9-slice à l'échelle : coins, liseré et biseaux réduits avec le reste — c'est ce
                // qui ramène le cadre du jeu de 44 à 36 sans le comprimer (doc de module) —, et
                // le corps étiré sur ce qui reste, ce qui laisse `width` et `height` libres. À
                // l'échelle 1 (le socle de premier plan), c'est le 9-slice ordinaire.
                design.paint_scaled(ui.painter(), slot_rect, texture, tint, effective_scale);

                // Le glyphe (ou le libellé de repli) est écrêté à SA case : un pictogramme trop
                // large ne doit pas déborder sur la voisine.
                let painter = ui
                    .painter()
                    .with_clip_rect(slot_rect.intersect(ui.clip_rect()));
                if let Some(icon) = slot.icon {
                    let color = self.variant.glyph_color(state, icon.native_color());
                    // `self.scale` et non `effective_scale` : le glyphe ne suit pas la réduction
                    // du cadre à 36 (plafond commun aux deux variantes, doc de module).
                    let drawn = self.variant.glyph_size(&design, icon) * self.scale;
                    // Calé sur la grille de pixels : une case de 43px met son centre à une
                    // demi-position, et un glyphe de 14px peint à x + 0,5 s'étale sur deux
                    // colonnes — flou visible à ×8 sur la comparaison au jeu.
                    let min =
                        (slot_rect.center() - drawn * 0.5).round_to_pixels(ui.pixels_per_point());
                    design.paint_icon(&painter, egui::Rect::from_min_size(min, drawn), icon, color);
                } else {
                    let color = self.variant.glyph_color(state, false);
                    let font = text::label_font(ui.ctx(), tokens::SWITCH_FONT_SIZE);
                    let galley = painter.layout_no_wrap(slot.label.clone(), font, color);
                    let pos = Align2::CENTER_CENTER
                        .align_size_within_rect(galley.size(), slot_rect)
                        .min;
                    painter.galley(pos, galley, color);
                }

                // La gouttière qui suit — jamais après la dernière case, et **rien du tout en
                // `FirstPlan`** : là, chaque socle porte ses quatre coins et son liseré, il n'y a
                // pas de cadre continu à prolonger entre deux cases. En `Frame`, aucune texture ne
                // la porte : le liseré du cadre la traverse de part en part, et le séparateur
                // n'occupe que le corps entre les deux liserés.
                // EXPLORATION — trait de 1 px sur la jointure de deux socles en `FirstPlan` :
                // les deux liserés noirs superposés y sont invisibles sur un fond sombre, un gris
                // les remplace par une séparation qui se voit sans peser.
                if index + 1 < count && self.variant == SwitchVariant::FirstPlan {
                    let x = slot_rect.right() - tokens::SWITCH_FIRST_PLAN_OVERLAP / 2.0;
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(x, rect.top() + tokens::SWITCH_BORDER_Y),
                            Vec2::new(1.0, rect.height() - tokens::SWITCH_BORDER_Y * 2.0),
                        ),
                        0,
                        tokens::SWITCH_FIRST_PLAN_SEAM,
                    );
                }
                if index + 1 < count && self.variant == SwitchVariant::Frame {
                    let gutter = egui::Rect::from_min_size(
                        egui::pos2(slot_rect.right(), rect.top()),
                        Vec2::new(separator_width, rect.height()),
                    );
                    ui.painter()
                        .rect_filled(gutter, 0, tokens::SWITCH_BORDER.gamma_multiply(dim));
                    ui.painter().rect_filled(
                        gutter.shrink2(Vec2::new(
                            0.0,
                            (tokens::SWITCH_BORDER_Y * effective_scale).round(),
                        )),
                        0,
                        tokens::SWITCH_SEPARATOR.gamma_multiply(dim),
                    );
                }
            }

            if state == SwitchState::Active {
                selected_rect = Some(slot_rect);
            }

            // L'infobulle d'une case à pictogramme porte son libellé — posée hors du test de
            // visibilité, une infobulle s'affiche sur interaction, pas sur peinture.
            if slot.icon.is_some() {
                crate::design::tooltip(&slot_response).text(slot.label.clone());
            }

            if self.enabled {
                let slot_response = slot_response.on_hover_cursor(egui::CursorIcon::PointingHand);
                if slot_response.clicked() && !selected {
                    clicked = Some((index, slot.value));
                }
            }
        }

        // Liseré doré de la case choisie (`FirstPlan` seulement) — demande utilisateur du
        // 2026-09-16, et il devient nécessaire : depuis que le survol allume l'icône ET le socle,
        // une case survolée et la case choisie ne se distinguaient plus l'une de l'autre.
        if let (SwitchVariant::FirstPlan, Some(slot_rect)) = (self.variant, selected_rect) {
            let base = tokens::SWITCH_FIRST_PLAN_RIM;
            let teinte = |alpha: u8| {
                let c = Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha);
                if self.enabled {
                    c
                } else {
                    c.gamma_multiply(dim)
                }
            };
            // `Inside` sur le rectangle EXACT du socle : le trait remplace la bordure de la
            // texture au lieu de s'y ajouter, à la même épaisseur et au même arrondi.
            ui.painter().rect_stroke(
                slot_rect,
                tokens::SWITCH_FIRST_PLAN_RIM_ROUNDING,
                egui::Stroke::new(
                    tokens::SWITCH_FIRST_PLAN_RIM_WIDTH,
                    teinte(tokens::SWITCH_FIRST_PLAN_RIM_ALPHA),
                ),
                egui::StrokeKind::Inside,
            );
            // Halo d'un pixel vers l'intérieur, même couleur, plus transparent : il adoucit la
            // marche entre le liseré et le corps du socle, sans épaissir le liseré.
            ui.painter().rect_stroke(
                slot_rect.shrink(tokens::SWITCH_FIRST_PLAN_RIM_WIDTH),
                tokens::SWITCH_FIRST_PLAN_HALO_ROUNDING,
                egui::Stroke::new(
                    tokens::SWITCH_FIRST_PLAN_HALO_WIDTH,
                    teinte(tokens::SWITCH_FIRST_PLAN_HALO_ALPHA),
                ),
                egui::StrokeKind::Inside,
            );
        }

        if let Some((index, value)) = clicked {
            *self.selected = value;
            response.mark_changed();
            tracing::debug!(
                component = "switch",
                name,
                case = self.slots[index].label,
                "clic"
            );
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **La frontière entre les deux variantes** — voir « la règle d'étanchéité » en tête de
    /// module. Ce test existe pour qu'un futur réglage du switch de premier plan ne puisse pas
    /// atteindre le sélecteur de genre sans le faire virer au rouge.
    #[test]
    fn la_variante_cadre_ignore_tout_ce_qui_appartient_au_premier_plan() {
        let mut choix = 0_u8;
        let cadre = Switch::new(&mut choix)
            .slot(0_u8, "Masculin")
            .slot(1, "Féminin");
        // La gouttière du cadre reste POSITIVE : les cases ne se chevauchent pas, et le liseré du
        // jeu se peint dedans. Un chevauchement est une gouttière négative — voir
        // `Switch::separator_width`.
        assert!(
            cadre.separator_width() > 0.0,
            "le cadre garde sa gouttière, il ne chevauche rien : {}",
            cadre.separator_width()
        );

        let mut choix = 0_u8;
        let premier_plan = Switch::new(&mut choix)
            .slot(0_u8, "Alliés")
            .slot(1, "Ennemis")
            .variant(SwitchVariant::FirstPlan);
        assert!(
            premier_plan.separator_width() < 0.0,
            "le premier plan chevauche : {}",
            premier_plan.separator_width()
        );

        // La taille native d'une case, elle, est commune depuis la demande « 36 par 36 de manière
        // générique » (2026-09-16, après ce lot) : ce n'est pas le premier plan qui déteint sur
        // le cadre, c'est le composant entier qui change de grille — voir
        // `les_deux_variantes_servent_la_meme_case_de_36`. Ce qui reste propre au cadre est SA
        // texture, peinte à l'échelle de sa capture.
        assert_eq!(
            SwitchVariant::Frame.native_slot_size(),
            Vec2::splat(tokens::SWITCH_SLOT_SIZE)
        );
        assert_eq!(
            SwitchVariant::FirstPlan.native_slot_size(),
            Vec2::splat(tokens::SWITCH_FIRST_PLAN_SIZE)
        );
        assert!(SwitchVariant::Frame.texture_scale() < 1.0);
        assert_eq!(SwitchVariant::FirstPlan.texture_scale(), 1.0);
    }

    /// Le corollaire du précédent, sur la seule chose que le lot « premier plan » a changé dans le
    /// vocabulaire des couleurs : **le survol allume le glyphe**. Le cadre, lui, garde les teintes
    /// mesurées sur les captures du jeu, état par état.
    #[test]
    fn la_teinte_du_glyphe_du_cadre_reste_celle_mesuree_sur_le_jeu() {
        for state in [
            SwitchState::Idle,
            SwitchState::Hovered,
            SwitchState::Active,
            SwitchState::Disabled,
        ] {
            assert_eq!(
                SwitchVariant::Frame.glyph_color(state, false),
                state.glyph_color(),
                "glyphe monochrome, état {state:?}"
            );
            assert_eq!(
                SwitchVariant::Frame.glyph_color(state, true),
                state.native_glyph_color(),
                "glyphe en couleurs, état {state:?}"
            );
        }
        // Le premier plan, lui, S'ÉCARTE de ces teintes — sinon la variante n'aurait pas de raison
        // d'être et ce test ne verrouillerait rien.
        assert_ne!(
            SwitchVariant::FirstPlan.glyph_color(SwitchState::Idle, false),
            SwitchState::Idle.glyph_color()
        );
    }

    /// La case sélectionnée garde son fond actif dans tous les états qui le permettent, et la
    /// case survolée prend le fond actif elle aussi — mesuré sur la capture du 2026-09-16.
    #[test]
    fn le_fond_suit_la_selection_et_le_survol() {
        assert_eq!(
            SwitchVariant::Frame.slot_texture(Position::First, SwitchState::Active, true),
            DsTexture::SwitchSlotActiveFirst
        );
        assert_eq!(
            SwitchVariant::Frame.slot_texture(Position::Last, SwitchState::Active, true),
            DsTexture::SwitchSlotActiveLast
        );
        assert_eq!(
            SwitchVariant::Frame.slot_texture(Position::First, SwitchState::Hovered, false),
            DsTexture::SwitchSlotActiveFirst
        );
        assert_eq!(
            SwitchVariant::Frame.slot_texture(Position::Middle, SwitchState::Hovered, false),
            DsTexture::SwitchSlotActive
        );
        assert_eq!(
            SwitchVariant::Frame.slot_texture(Position::Last, SwitchState::Idle, false),
            DsTexture::SwitchSlotInactiveLast
        );
    }

    /// L'échelle multiplie le côté de 36 et tout est arrondi au pixel : à `44 / 36`, le cadre
    /// retrouve les 44 de sa capture (deux cases : 90, avec `width(88)` les 88 du jeu) ; à 0,5,
    /// une case fait 18 et le séparateur 1 — jamais 0.
    #[test]
    fn l_echelle_reduit_tout_au_pixel_pres() {
        let mut v = 0u8;
        let capture = switch(&mut v).slot(0, "a").slot(1, "b").scale(44.0 / 36.0);
        assert_eq!(capture.desired_size(), Vec2::new(90.0, 44.0));
        assert!((capture.effective_scale() - 1.0).abs() < 1e-5);
        let mut v = 0u8;
        let jeu = switch(&mut v)
            .slot(0, "a")
            .slot(1, "b")
            .scale(44.0 / 36.0)
            .width(88.0);
        assert_eq!(jeu.desired_size(), Vec2::new(88.0, 44.0));
        let mut v = 0u8;
        let moitie = switch(&mut v).slot(0, "a").slot(1, "b").scale(0.5);
        assert_eq!(moitie.desired_size(), Vec2::new(37.0, 18.0));
        assert_eq!(moitie.separator_width(), 1.0);
        // Une largeur imposée prime sur l'échelle.
        let mut v = 0u8;
        let impose = switch(&mut v)
            .slot(0, "a")
            .slot(1, "b")
            .scale(0.5)
            .width(100.0);
        assert_eq!(impose.desired_size(), Vec2::new(100.0, 18.0));
    }

    /// Le cadre du jeu est peint à `36 / 44` de sa capture, le socle de premier plan tel quel :
    /// les deux variantes servent la même case de 36, et c'est la texture qui s'adapte.
    #[test]
    fn les_deux_variantes_servent_la_meme_case_de_36() {
        assert_eq!(SwitchVariant::Frame.native_slot_size(), Vec2::splat(36.0));
        assert_eq!(
            SwitchVariant::FirstPlan.native_slot_size(),
            Vec2::splat(36.0)
        );
        assert!((SwitchVariant::Frame.texture_scale() - 36.0 / 44.0).abs() < 1e-5);
        assert_eq!(SwitchVariant::FirstPlan.texture_scale(), 1.0);
        let mut v = 0u8;
        let cadre = switch(&mut v).slot(0, "a").slot(1, "b");
        let mut v = 0u8;
        let premier_plan = switch(&mut v)
            .slot(0, "a")
            .slot(1, "b")
            .variant(SwitchVariant::FirstPlan);
        // Même hauteur ; la largeur ne diffère que par la gouttière — 2 px de séparateur pour
        // le cadre, 2 px de chevauchement pour le premier plan (74 contre 70).
        assert_eq!(cadre.desired_size(), Vec2::new(74.0, 36.0));
        assert_eq!(premier_plan.desired_size(), Vec2::new(70.0, 36.0));
        // Le séparateur du cadre, 2px dans la capture, reste à 2 une fois réduit (1,64 arrondi).
        assert_eq!(cadre.separator_width(), 2.0);
    }

    /// Une case du milieu prend les textures sans coin — les seules qui conviennent entre deux
    /// séparateurs.
    #[test]
    fn la_case_du_milieu_n_a_pas_de_coin() {
        assert_eq!(
            SwitchVariant::Frame.slot_texture(Position::Middle, SwitchState::Active, true),
            DsTexture::SwitchSlotActive
        );
        assert_eq!(
            SwitchVariant::Frame.slot_texture(Position::Middle, SwitchState::Idle, false),
            DsTexture::SwitchSlotInactive
        );
        assert_eq!(Position::of(0, 3), Position::First);
        assert_eq!(Position::of(1, 3), Position::Middle);
        assert_eq!(Position::of(2, 3), Position::Last);
        // À deux cases, il n'y a pas de milieu.
        assert_eq!(Position::of(1, 2), Position::Last);
    }

    /// Désactivé, la case sélectionnée reste reconnaissable à son fond.
    #[test]
    fn desactive_garde_le_fond_de_la_case_selectionnee() {
        assert_eq!(
            SwitchVariant::Frame.slot_texture(Position::First, SwitchState::Disabled, true),
            DsTexture::SwitchSlotActiveFirst
        );
        assert_eq!(
            SwitchVariant::Frame.slot_texture(Position::Last, SwitchState::Disabled, false),
            DsTexture::SwitchSlotInactiveLast
        );
    }

    /// Les deux glyphes du jeu sont peints à leur taille native : ♂ 14 × 14 ne grossit pas à 16.
    #[test]
    fn un_glyphe_qui_tient_dans_le_carre_reste_natif() {
        assert_eq!(glyph_size(Vec2::new(14.0, 14.0)), Vec2::new(14.0, 14.0));
        assert_eq!(glyph_size(Vec2::new(10.0, 16.0)), Vec2::new(10.0, 16.0));
    }

    /// Un pictogramme plus grand que le carré y est ramené, rapport conservé.
    #[test]
    fn un_glyphe_trop_grand_est_reduit_au_carre() {
        let drawn = glyph_size(Vec2::new(22.0, 22.0));
        assert!((drawn.x - 16.0).abs() < 1e-4 && (drawn.y - 16.0).abs() < 1e-4);
        let drawn = glyph_size(Vec2::new(22.0, 18.0));
        assert!((drawn.x - 16.0).abs() < 1e-4);
        assert!((drawn.y - 16.0 * 18.0 / 22.0).abs() < 1e-4);
    }

    /// `icon` et `preview_state` visent la dernière case déclarée, et sont sans effet avant.
    #[test]
    fn icon_et_preview_state_visent_la_derniere_case() {
        let mut selected = 0_u8;
        let switch = Switch::new(&mut selected)
            .icon(DsIcon::Male)
            .slot(0, "A")
            .icon(DsIcon::Male)
            .slot(1, "B")
            .preview_state(SwitchState::Hovered);
        assert_eq!(switch.slots.len(), 2);
        assert_eq!(switch.slots[0].icon, Some(DsIcon::Male));
        assert_eq!(switch.slots[0].forced_state, None);
        assert_eq!(switch.slots[1].icon, None);
        assert_eq!(switch.slots[1].forced_state, Some(SwitchState::Hovered));
    }

    /// La taille par défaut est la case de 36 — 74 × 36 pour deux cases — et suit le nombre de
    /// cases : 36 par case, 2 par séparateur.
    #[test]
    fn la_taille_par_defaut_est_la_case_de_36_et_suit_le_nombre_de_cases() {
        let mut selected = 0_u8;
        let deux = Switch::new(&mut selected).slot(0, "A").slot(1, "B");
        assert_eq!(deux.desired_size(), Vec2::new(74.0, 36.0));
        let mut selected = 0_u8;
        let trois = Switch::new(&mut selected)
            .slot(0, "A")
            .slot(1, "B")
            .slot(2, "C");
        assert_eq!(trois.desired_size(), Vec2::new(112.0, 36.0));
        let mut selected = 0_u8;
        let impose = Switch::new(&mut selected)
            .slot(0, "A")
            .slot(1, "B")
            .width(200.0);
        assert_eq!(impose.desired_size(), Vec2::new(200.0, 36.0));
        let mut selected = 0_u8;
        let abaisse = Switch::new(&mut selected)
            .slot(0, "A")
            .slot(1, "B")
            .width(70.0)
            .height(26.0);
        assert_eq!(abaisse.desired_size(), Vec2::new(70.0, 26.0));
    }

    #[test]
    fn un_glyphe_en_couleurs_est_peint_tel_quel_puis_attenue() {
        assert!(DsIcon::Allies.native_color());
        assert!(!DsIcon::Male.native_color());
        assert_eq!(SwitchState::Active.native_glyph_color(), Color32::WHITE);
        assert_eq!(SwitchState::Hovered.native_glyph_color(), Color32::WHITE);
        assert_eq!(
            SwitchState::Idle.native_glyph_color(),
            tokens::ICON_NATIVE_DIM
        );
    }
}
