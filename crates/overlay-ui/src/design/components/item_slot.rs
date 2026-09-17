//! **Emplacement d'objet** du design system Wakfu — le carré qui porte une icône d'objet, sa
//! bordure de rareté et son compteur. Composant **feuille** (§1 du contrat).
//!
//! ```ignore
//! use overlay_ui::design::{self, ItemRarity, SlotFrame};
//!
//! ui.add(
//!     design::item_slot()
//!         .frame(SlotFrame::Rarity(ItemRarity::Legendary))
//!         .icon(egui::load::SizedTexture::from_handle(&handle))
//!         .count(SlotCount::Fraction { current: 42, target: 500 }),
//! );
//! ```
//!
//! ## L'ordre de peinture, qui est la raison d'être de ce composant
//!
//! **La bordure se peint AVANT l'icône. Jamais après.** Ce n'est pas une préférence : la fenêtre
//! intérieure des textures `Border-*.webp` **n'est pas un trou transparent** — c'est un aplat
//! semi-transparent (~70 % d'opacité) teinté par la rareté, vérifié sur les octets décodés. Peinte
//! par-dessus, elle recouvre l'icône entière d'un voile coloré.
//!
//! Le bug a existé : « j'ai l'impression que tu as mis les objets en opacité », rapporté le jour
//! même de l'arrivée de ces textures, flagrant sur les raretés à teinte franche (le jaune-olive de
//! `Border-LEGENDARY.webp`), plus discret sur celles déjà proches de l'icône en teinte. Il est
//! rattrapable à l'œil une fois, pas à chaque relecture — d'où [`paint_order`], une fonction libre
//! que son test verrouille.
//!
//! ## Ce que l'appelant fournit, et ce qu'il ne fournit pas
//!
//! L'icône arrive en [`egui::load::SizedTexture`], **déjà résolue**. Ce n'est pas une entorse au
//! contrat (« aucune texture en paramètre ») : cette règle vise les assets du design system, que le
//! composant doit résoudre depuis une intention. Une icône d'objet est du **contenu** — elle est
//! téléchargée, mise en cache et indexée par le catalogue, tout cela hors du design system. Le
//! composant ne saurait pas la nommer.
//!
//! Elle arrive **avec sa taille native**, et pas en `TextureId` nu, parce qu'elle est peinte à son
//! rapport ([`crate::design::fit`]) : les icônes de `wakassets/items` et `wakassets/monsters` sont
//! carrées, mais un monstre servi par `wakassets/monsterIllustrations` est une **bannière
//! rectangulaire**, écrasée dans le carré de l'emplacement jusqu'au 2026-09-17 (retour
//! utilisateur : « les images provenant de `wakassets/monsterIllustrations` sont déformées »).
//!
//! La **rareté**, en revanche, est une intention : [`ItemRarity`] est un type du design system, et
//! c'est au panneau de traduire son `WakfuRarity` métier — un composant n'accède pas à
//! `overlay_engine` (§6).
//!
//! ## Une taille impossible se voit ET se dit
//!
//! Toute taille est valide — le cadre est une texture carrée mise à l'échelle, l'icône suit en
//! proportion. Mais sous [`tokens::ITEM_SLOT_MIN_SIZE`], le liseré et la marge du cadre mangent
//! tout le carré : l'emplacement est **peint quand même** (un rectangle trop petit doit se voir sur
//! la capture, pas paniquer — §3 du contrat) et signalé **une fois par instance** au journal
//! (§4). C'est `%APPDATA%\…\overlay-ui.<date>.log` qu'on relit après un retour utilisateur, pas
//! le terminal — d'où [`ItemSlot::log_name`] dès que deux emplacements voisins doivent se
//! distinguer.

use egui::{load::SizedTexture, Response, Sense, Ui, Vec2, Widget};

use crate::design::{fit, text, tokens, DesignSystem, DsTexture};

/// Rareté d'un objet, **du point de vue du design system** : elle ne sert qu'à choisir une bordure.
///
/// Un type propre plutôt que le `WakfuRarity` du moteur, que le contrat interdit à un composant de
/// connaître. La traduction appartient au panneau, qui a déjà les deux sous les yeux.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemRarity {
    Common,
    Rare,
    Mythical,
    Legendary,
    Memory,
    Epic,
    Relic,
}

impl ItemRarity {
    /// Texture de bordure correspondante.
    pub fn border(self) -> DsTexture {
        match self {
            ItemRarity::Common => DsTexture::ItemBorderCommon,
            ItemRarity::Rare => DsTexture::ItemBorderRare,
            ItemRarity::Mythical => DsTexture::ItemBorderMythical,
            ItemRarity::Legendary => DsTexture::ItemBorderLegendary,
            ItemRarity::Memory => DsTexture::ItemBorderMemory,
            ItemRarity::Epic => DsTexture::ItemBorderEpic,
            ItemRarity::Relic => DsTexture::ItemBorderRelic,
        }
    }

    /// **La couleur sur laquelle l'arc-en-ciel d'une complétion se condense** — voir
    /// [`ItemSlot::completion`].
    ///
    /// **Mesurée sur les textures elles-mêmes**, pas choisie : pour chacun des sept
    /// `Border-*.webp`, la teinte des pixels du liseré (l'anneau entre
    /// [`tokens::ITEM_SLOT_BORDER_INSET_RATIO`] et [`tokens::ITEM_SLOT_BORDER_INNER_RATIO`] du
    /// bord, bande médiane du côté gauche), moyennée sur le vingtième le plus saturé — le liseré
    /// est un dégradé, sa partie la plus vive est celle qui donne son nom à la rareté. Un
    /// échantillon pris au hasard dans l'anneau rendrait le gris de l'ombre, pas la couleur.
    ///
    /// Le composant ne peut pas relire ses textures au runtime pour retrouver ces teintes
    /// (`DesignSystem` ne rend que des `TextureId`), et un jeton par rareté serait sept jetons de
    /// plus dans `tokens.rs` pour une table que seule cette méthode lit.
    pub fn seal_color(self) -> egui::Color32 {
        match self {
            ItemRarity::Common => egui::Color32::from_rgb(0xDB, 0xDB, 0xDB),
            ItemRarity::Rare => egui::Color32::from_rgb(0x19, 0xFF, 0x95),
            ItemRarity::Mythical => egui::Color32::from_rgb(0xF1, 0x82, 0x00),
            ItemRarity::Legendary => egui::Color32::from_rgb(0xEC, 0xFD, 0x05),
            ItemRarity::Memory => egui::Color32::from_rgb(0x1F, 0xB7, 0xFF),
            ItemRarity::Epic => egui::Color32::from_rgb(0xFF, 0x6F, 0xCB),
            ItemRarity::Relic => egui::Color32::from_rgb(0xB8, 0x7C, 0xFF),
        }
    }
}

/// Cadre d'un emplacement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotFrame {
    /// Bordure de rareté du jeu — la fenêtre intérieure est teintée, d'où l'ordre de peinture.
    Rarity(ItemRarity),
    /// Trait simple, sans rareté : les tuiles d'ennemi, qui n'en ont pas.
    Plain,
}

/// Compteur incrusté dans le coin bas-droit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotCount {
    /// Un nombre seul.
    Simple(i64),
    /// Un nombre courant sur une cible — la cible garde l'ancrage bas, le courant remonte
    /// au-dessus d'elle. **Inversion demandée explicitement** le 2026-09-06 : c'est la fraction qui
    /// doit tomber à l'emplacement standard des suivis incrémentaux, pas le nombre courant.
    Fraction { current: i64, target: i64 },
    /// La cible SEULE, sans valeur courante — `/50` à l'emplacement standard.
    ///
    /// **Écrite pour l'onglet « Suivi » de la fenêtre Options** (2026-09-13), qui sert à *composer*
    /// une liste et non à la lire : y afficher les compteurs vivants « polluait en informations »
    /// (retour utilisateur). La cible, elle, n'est pas une mesure mais un **réglage** de l'entrée,
    /// au même titre que son mode — elle reste donc visible. Les compteurs, eux, gardent leur place
    /// dans le bandeau in-game, où les lire est justement le but.
    Target(i64),
}

/// Glyphe de **mode** incrusté dans le coin haut-gauche — à l'opposé du compteur, qui tient le
/// coin bas-droit.
///
/// Décompte et objectif affichent la même fraction (« 2/5 » se lit « il en reste 2 » ou « j'en ai
/// 2 ») : sans marque, deux tuiles de modes différents sont identiques au pixel près. Le glyphe
/// reprend les formes du switch d'ajout du site (`target` et `goal-flag`), que l'utilisateur a
/// vues en créant le suivi. **Décision du 2026-09-17** : glyphe seul, dans la couleur du texte
/// qu'il accompagne (l'or du nombre courant dans le bandeau, le gris de la cible dans l'onglet
/// Suivi) — ni couleur propre au mode, ni liseré, celui-ci codant déjà la rareté et la sélection.
/// L'incrémental n'en porte pas : sans cible, il n'y a rien à lever.
///
/// Peint au **vecteur** (traits et polygone cernés de noir comme les chiffres) plutôt qu'en
/// texture : à 8 px, une icône du design system serait floue, et le cerne doit être celui du
/// texte voisin.
///
/// **Ne se peint pas en mode sélection** : la case à cocher occupe le même coin, au même retrait
/// ([`tokens::ITEM_SLOT_GLYPH_INSET`]), et le recouvrirait de toute façon. Le mode reste lisible
/// dans l'infobulle et dans le sélecteur de l'onglet Suivi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotGlyph {
    /// Décompte — une cible : anneau et point.
    Countdown,
    /// Objectif — un drapeau : hampe et fanion.
    Goal,
}

/// **Où en est la célébration de complétion**, à un instant donné — voir
/// [`ItemSlot::completion`].
///
/// Une structure de valeurs plutôt qu'une suite d'instructions, et calculée par une **fonction
/// libre testée** ([`completion_phase`]), pour la même raison que [`paint_order`] : une séquence
/// écrite en dur dans la peinture ne se relit pas et ne se vérifie pas. Ici s'ajoute une raison
/// propre à l'animation — le harnais de captures fige des instants précis (`t = 0,4 s`,
/// `1,2 s`, `2,1 s`) et doit pouvoir affirmer ce que chacun contient sans peindre quoi que ce soit.
///
/// Tous les champs sont **sans unité et bornés**, sauf [`Self::angle`] (radians) : le composant les
/// traduit en pixels d'après son propre côté, de sorte qu'un emplacement de 32 px célèbre comme un
/// de 64.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompletionPhase {
    /// Échelle de l'emplacement entier — 1,0 au repos.
    pub scale: f32,
    /// Largeur de la couronne, en fraction de [`tokens::ITEM_SLOT_SIZE`]. Zéro = aucune couronne.
    pub crown: f32,
    /// Rotation de l'arc-en-ciel, en radians.
    pub angle: f32,
    /// Condensation : 0 = arc-en-ciel pur, 1 = couleur de rareté pleine.
    pub seal: f32,
    /// Intensité du halo derrière la couronne.
    pub glow: f32,
    /// Éclat au centre de l'emplacement.
    pub flash: f32,
    /// Onde circulaire qui s'écarte : 0 = au bord de l'emplacement, 1 = évanouie. `None` tant
    /// qu'aucune onde n'est partie.
    pub wave: Option<f32>,
    /// Dissolution : 0 = emplacement intact, 1 = entièrement parti.
    pub dissolve: f32,
}

impl CompletionPhase {
    /// La phase d'un emplacement qui ne célèbre pas — et celle d'une célébration terminée.
    pub const REST: Self = Self {
        scale: 1.0,
        crown: 0.0,
        angle: 0.0,
        seal: 0.0,
        glow: 0.0,
        flash: 0.0,
        wave: None,
        dissolve: 0.0,
    };

    /// Vrai quand il n'y a plus rien à peindre — l'emplacement est dissous.
    pub fn finished(self) -> bool {
        self.dissolve >= 1.0
    }
}

/// **La séquence de la célébration, en données** — `elapsed` est le temps écoulé depuis le
/// franchissement du seuil, en secondes.
///
/// Les cinq moments, et ce qui les justifie (variante « Rareté scellée », validée le 2026-09-17) :
///
/// | Jusqu'à | Ce qui se passe |
/// | --- | --- |
/// | [`tokens::ITEM_SLOT_COMPLETION_LIFT_END`] | l'emplacement se soulève — on regarde CETTE tuile |
/// | [`tokens::ITEM_SLOT_COMPLETION_SPIN_END`] | la couronne arc-en-ciel tourne, **en accélérant** |
/// | [`tokens::ITEM_SLOT_COMPLETION_SEAL_END`] | elle se condense sur la couleur de rareté, éclat |
/// | [`tokens::ITEM_SLOT_COMPLETION_DISSOLVE_END`] | l'emplacement se dissout en particules |
/// | [`tokens::ITEM_SLOT_COMPLETION_DURATION`] | plus rien — l'hôte retire l'entrée |
///
/// **L'accélération n'est pas un ornement** : une rotation à vitesse constante se lit comme un
/// chargement qui attend, une rotation qui accélère se lit comme quelque chose qui aboutit. C'est
/// elle qui fait que l'éclat arrive comme une conclusion et non comme une interruption.
pub fn completion_phase(elapsed: f32) -> CompletionPhase {
    if elapsed < 0.0 || elapsed >= tokens::ITEM_SLOT_COMPLETION_DURATION {
        // Au-delà de la durée, `dissolve` reste à 1 : `finished()` doit rester vrai pour un
        // appelant qui interrogerait la phase après coup, sans que rien ne soit peint.
        return CompletionPhase {
            dissolve: if elapsed < 0.0 { 0.0 } else { 1.0 },
            ..CompletionPhase::REST
        };
    }

    let lift = progress(elapsed, 0.0, tokens::ITEM_SLOT_COMPLETION_LIFT_END);
    let spin = progress(
        elapsed,
        tokens::ITEM_SLOT_COMPLETION_LIFT_END,
        tokens::ITEM_SLOT_COMPLETION_SPIN_END,
    );
    let seal = progress(
        elapsed,
        tokens::ITEM_SLOT_COMPLETION_SPIN_END,
        tokens::ITEM_SLOT_COMPLETION_SEAL_END,
    );
    let dissolve = progress(
        elapsed,
        tokens::ITEM_SLOT_COMPLETION_DISSOLVE_START,
        tokens::ITEM_SLOT_COMPLETION_DISSOLVE_END,
    );

    // L'éclat part avec la condensation et s'éteint en un tiers de celle-ci : il ponctue, il ne
    // dure pas. L'onde le suit sur toute la fin de la condensation et un peu au-delà.
    let flash_span = (tokens::ITEM_SLOT_COMPLETION_SEAL_END
        - tokens::ITEM_SLOT_COMPLETION_SPIN_END)
        * FLASH_SPAN_RATIO;
    // **Rien avant la fin de la rotation** : sans cette garde, `progress` rend 0 pendant toute la
    // rotation, donc `1 - 0` — un éclat à pleine puissance dès la première image, qui blanchissait
    // la tuile avant même qu'elle ait tourné (vu sur la première planche de galerie).
    let flash = if elapsed < tokens::ITEM_SLOT_COMPLETION_SPIN_END {
        0.0
    } else {
        1.0 - progress(
            elapsed,
            tokens::ITEM_SLOT_COMPLETION_SPIN_END,
            tokens::ITEM_SLOT_COMPLETION_SPIN_END + flash_span,
        )
    };
    let wave = (elapsed >= tokens::ITEM_SLOT_COMPLETION_SPIN_END).then(|| {
        progress(
            elapsed,
            tokens::ITEM_SLOT_COMPLETION_SPIN_END,
            tokens::ITEM_SLOT_COMPLETION_DISSOLVE_START,
        )
    });

    // La couronne existe dès le soulèvement (fine, presque un reflet) et disparaît avec la
    // dissolution : elle n'a pas à survivre à l'emplacement qu'elle borde.
    let epaisseur = if seal > 0.0 {
        // Elle se resserre en se scellant — le trait devient net, comme un liseré de rareté.
        lerp(
            tokens::ITEM_SLOT_COMPLETION_CROWN_MAX,
            tokens::ITEM_SLOT_COMPLETION_CROWN_MIN,
            seal,
        )
    } else {
        lerp(
            tokens::ITEM_SLOT_COMPLETION_CROWN_MIN,
            tokens::ITEM_SLOT_COMPLETION_CROWN_MAX,
            ease_out(spin),
        )
    };

    CompletionPhase {
        // Soulèvement, puis retour lent à l'échelle normale pendant la rotation ; la dissolution
        // reprend un souffle d'expansion, pour que l'emplacement parte vers l'extérieur.
        scale: 1.0
            + (tokens::ITEM_SLOT_COMPLETION_LIFT_SCALE - 1.0) * ease_out(lift)
            + DISSOLVE_EXPANSION * dissolve,
        crown: epaisseur * (1.0 - dissolve) / tokens::ITEM_SLOT_SIZE,
        // `ease_in` sur la rotation : trois tours dont le dernier vaut la moitié du temps.
        angle: ease_in(spin) * tokens::ITEM_SLOT_COMPLETION_TURNS * std::f32::consts::TAU,
        seal,
        glow: (spin.max(seal) * (1.0 - dissolve)).min(1.0),
        flash: flash.max(0.0),
        wave,
        dissolve,
    }
}

/// Part de l'intervalle `[from, to]` déjà parcourue par `value`, bornée à `[0, 1]`.
fn progress(value: f32, from: f32, to: f32) -> f32 {
    if to <= from {
        return if value >= to { 1.0 } else { 0.0 };
    }
    ((value - from) / (to - from)).clamp(0.0, 1.0)
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

fn ease_in(t: f32) -> f32 {
    t * t * t
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// Part de la condensation pendant laquelle l'éclat est visible — voir [`completion_phase`].
const FLASH_SPAN_RATIO: f32 = 0.8;

/// Ce que la dissolution ajoute à l'échelle de l'emplacement : un souffle, pas un saut.
const DISSOLVE_EXPANSION: f32 = 0.06;

/// Ordre dans lequel les couches d'un emplacement se peignent.
///
/// Fonction libre et testée pour une seule raison : **l'inverser a déjà produit un bug**, et ce bug
/// ne se voit que sur les raretés à teinte franche. Le décrire en données plutôt qu'en suite
/// d'instructions rend l'erreur visible à la relecture *et* saisissable par un test.
pub fn paint_order(frame: SlotFrame) -> &'static [SlotLayer] {
    match frame {
        // Fond, PUIS bordure, PUIS icône : la bordure est sous l'icône, pas par-dessus.
        SlotFrame::Rarity(_) => &[SlotLayer::Background, SlotLayer::Border, SlotLayer::Icon],
        // Sans rareté, le trait se pose au contraire APRÈS l'icône : c'est un liseré net, pas un
        // aplat, et il doit rester visible si l'icône déborde.
        SlotFrame::Plain => &[SlotLayer::Background, SlotLayer::Icon, SlotLayer::Border],
    }
}

/// Une couche d'un emplacement — voir [`paint_order`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotLayer {
    Background,
    Border,
    Icon,
}

/// **Ton d'une sélection** — ce que le fait d'être retenu annonce.
///
/// Demande utilisateur du 2026-09-13 : une sélection multiple ne sert pas toujours à supprimer, et
/// les deux ne doivent pas se ressembler. Rien d'autre ne change entre les deux — même anneau, même
/// case, même retrait : **seule la couleur**.
///
/// Le vocabulaire est celui des boutons (`ButtonVariant::Danger`), pas un nom inventé : dans cette
/// interface, le rouge du bouton « Annuler » est la couleur d'une action qui détruit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionTone {
    /// Retenu pour une action quelconque — l'or, la couleur d'état de l'interface.
    #[default]
    Neutral,
    /// Retenu pour être supprimé — le rouge mesuré du jeu.
    Danger,
}

impl SelectionTone {
    /// Couleur du liseré.
    pub fn border(self) -> egui::Color32 {
        match self {
            SelectionTone::Neutral => tokens::ITEM_SLOT_SELECTED_BORDER,
            SelectionTone::Danger => tokens::ITEM_SLOT_SELECTED_BORDER_DANGER,
        }
    }

    /// Teinte de la case à cocher — **appliquée seulement quand elle est cochée**.
    ///
    /// Une case décochée ne dit rien de l'action : elle annonce un geste possible, pas un objet
    /// retenu. La teindre en rouge ferait passer pour « à supprimer » ce que l'utilisateur n'a
    /// justement pas choisi, et le ferait sur les huit tuiles à la fois. Le ton porte donc sur ce
    /// qui est retenu, jamais sur ce qui ne l'est pas.
    ///
    /// Le tint d'egui **multiplie** : sur la case cochée, dont le carré intérieur est blanc, il
    /// rend exactement la couleur demandée, et assombrit le cadre doré vers la même teinte.
    pub fn checkbox_tint(self, checked: bool) -> egui::Color32 {
        match (self, checked) {
            (SelectionTone::Danger, true) => tokens::ITEM_SLOT_SELECTED_BORDER_DANGER,
            _ => egui::Color32::WHITE,
        }
    }
}

/// **L'anneau où se peint le liseré d'un emplacement** : son rectangle et son rayon de coin.
///
/// Les textures de rareté ne collent pas leur liseré au bord du carré — elles le posent à
/// [`tokens::ITEM_SLOT_BORDER_INSET_RATIO`] du bord, avec un coin de
/// [`tokens::ITEM_SLOT_BORDER_CORNER_RATIO`]. Tout trait qui veut *coïncider* avec elles doit viser
/// cet anneau-là : le cadre simple d'un monstre, et le liseré de sélection d'un panneau.
///
/// **Fonction publique parce que deux mondes s'en servent.** Le composant l'utilise pour ses
/// propres traits, mais une superposition faite par-dessus un emplacement déjà peint (une maquette,
/// une sélection posée par un panneau qui ne construit pas le slot) n'a aucun moyen de la
/// reconstituer sans recopier deux ratios — et c'est précisément cette recopie qui a produit le
/// décalage signalé le 2026-09-13.
pub fn border_ring(rect: egui::Rect) -> (egui::Rect, f32) {
    let side = rect.width().min(rect.height());
    (
        rect.shrink(side * tokens::ITEM_SLOT_BORDER_INSET_RATIO),
        side * tokens::ITEM_SLOT_BORDER_CORNER_RATIO,
    )
}

/// Construit un emplacement d'objet vide.
pub fn item_slot() -> ItemSlot {
    ItemSlot {
        frame: SlotFrame::Plain,
        icon: None,
        count: None,
        glyph: None,
        completion: None,
        size: tokens::ITEM_SLOT_SIZE,
        selection: None,
        selection_tone: SelectionTone::Neutral,
        log_name: None,
    }
}

/// Voir [`item_slot`].
pub struct ItemSlot {
    frame: SlotFrame,
    icon: Option<SizedTexture>,
    count: Option<SlotCount>,
    glyph: Option<SlotGlyph>,
    completion: Option<CompletionPhase>,
    size: f32,
    selection: Option<bool>,
    selection_tone: SelectionTone,
    log_name: Option<String>,
}

impl ItemSlot {
    /// Cadre. Par défaut [`SlotFrame::Plain`].
    pub fn frame(mut self, frame: SlotFrame) -> Self {
        self.frame = frame;
        self
    }

    /// Icône déjà résolue, **avec sa taille native** — voir la doc de module sur pourquoi ce n'est
    /// pas une texture du design system, et pourquoi la taille l'accompagne. Sans icône,
    /// l'emplacement est peint vide.
    pub fn icon(mut self, icon: SizedTexture) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Compteur incrusté. Sans appel, aucun compteur.
    pub fn count(mut self, count: SlotCount) -> Self {
        self.count = Some(count);
        self
    }

    /// **Célébration de complétion** : le temps écoulé depuis que le décompte est arrivé à 0 ou
    /// que l'objectif a atteint sa cible, en secondes. `None` — le défaut — pour un emplacement
    /// ordinaire.
    ///
    /// L'emplacement se soulève, sa bordure devient une couronne arc-en-ciel qui tourne en
    /// accélérant, la couronne se **condense sur la couleur de rareté de l'objet**
    /// ([`ItemRarity::seal_color`]), éclate, puis l'emplacement se dissout en particules. La
    /// séquence entière tient en [`tokens::ITEM_SLOT_COMPLETION_DURATION`] et se lit dans
    /// [`completion_phase`].
    ///
    /// **L'appelant passe un temps, pas une phase**, et c'est délibéré : le panneau connaît
    /// l'instant du franchissement (l'hôte le lui donne), pas le découpage de l'animation, qui est
    /// une décision du design system. C'est aussi ce qui rend la capture de référence possible —
    /// un temps figé rend toujours la même image.
    ///
    /// **La couleur du sceau vient du cadre**, jamais de l'appelant (§ « une intention, pas une
    /// couleur ») : la rareté d'un objet, ou l'or de l'interface pour un ennemi, qui n'en a pas.
    ///
    /// Ce que le composant ne fait PAS : la gerbe de confettis. Elle sort largement du carré, et
    /// un emplacement ne peint pas hors de lui-même — c'est au panneau de la poser, au-dessus de
    /// sa bande et hors de sa zone défilante (voir `panels::watchlist`).
    pub fn completion(mut self, elapsed_seconds: Option<f32>) -> Self {
        self.completion = elapsed_seconds.map(completion_phase);
        self
    }

    /// Glyphe de mode au coin haut-gauche — voir [`SlotGlyph`]. Sans appel, aucun glyphe.
    pub fn glyph(mut self, glyph: Option<SlotGlyph>) -> Self {
        self.glyph = glyph;
        self
    }

    /// Côté du carré. Par défaut [`tokens::ITEM_SLOT_SIZE`].
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// **Mode « sélection multiple »** : `None` hors du mode, `Some(cochée)` dedans.
    ///
    /// Dans le mode, l'emplacement porte une **case à cocher** à son coin haut-gauche, et un
    /// **liseré or** quand elle est cochée — posé exactement sur celui du cadre, par-dessus tout le
    /// reste. L'appel s'écrit `.selection(select_mode.then_some(cochée))`.
    ///
    /// **Les deux appartiennent au composant, pas à l'appelant.** L'onglet Suivi les peignait
    /// lui-même : le liseré à 4 px de rayon et collé au bord, donc à côté du liseré de rareté qu'il
    /// était censé recouvrir (voir [`border_ring`]), et la case en widget interactif, qui volait à
    /// la tuile le clic des 20 px qu'elle couvre. Le second panneau qui en aurait eu besoin — le
    /// bandeau in-game — aurait recopié les deux, défauts compris.
    ///
    /// La case ne prend aucun geste : **cocher est le clic de la tuile**, que l'appelant lit sur la
    /// [`egui::Response`] rendue. Elle est un signe d'état, pas un second contrôle.
    pub fn selection(mut self, selection: Option<bool>) -> Self {
        self.selection = selection;
        self
    }

    /// Ton de la sélection — voir [`SelectionTone`]. Par défaut [`SelectionTone::Neutral`], l'or.
    ///
    /// Sans effet hors du mode sélection : un emplacement qu'on ne peut pas cocher n'annonce
    /// aucune action.
    pub fn selection_tone(mut self, tone: SelectionTone) -> Self {
        self.selection_tone = tone;
        self
    }

    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Côté de l'icône, pour ce cadre et ce côté d'emplacement.
    ///
    /// Une bordure de rareté mange sa marge : l'icône s'inscrit dans la fenêtre intérieure de la
    /// texture, réduite de [`tokens::ITEM_SLOT_ICON_FILL`]. Un cadre simple n'a pas cette
    /// contrainte — son icône garde la taille que l'appelant lui donne par [`ItemSlot::size`].
    fn icon_side(&self) -> f32 {
        match self.frame {
            SlotFrame::Rarity(_) => {
                self.size
                    * (1.0 - 2.0 * tokens::ITEM_SLOT_BORDER_INNER_RATIO)
                    * tokens::ITEM_SLOT_ICON_FILL
            }
            // Un cadre simple ne contraint rien : sans cote propre, l'icône remplirait tout le
            // carré. C'est ce qu'a montré le snapshot du décompte en migrant le panneau Suivi —
            // la bordure de rareté masquait le défaut sur les objets, l'ennemi l'a révélé.
            SlotFrame::Plain => self.size * tokens::ITEM_SLOT_PLAIN_ICON_FILL,
        }
    }
}

impl Widget for ItemSlot {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(self.size), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        // Un emplacement plus petit que [`tokens::ITEM_SLOT_MIN_SIZE`] ne peut plus rien montrer :
        // le liseré et la marge du cadre mangent tout, l'icône n'a plus de place. **Peint quand
        // même** — le contrat veut qu'un rectangle trop petit se voie sur la capture plutôt que de
        // paniquer — mais dit **une fois par instance**, sur l'id de la réponse : un défaut de mise
        // en page est un événement, pas un flux à 60 Hz. C'est le journal de l'overlay qu'on relit
        // après un retour utilisateur, pas le terminal (§15 du plan).
        if self.size < tokens::ITEM_SLOT_MIN_SIZE {
            let warned_id = response.id.with("ds-item-slot-size");
            let already = ui.ctx().data_mut(|d| {
                let seen = d.get_temp::<bool>(warned_id).unwrap_or(false);
                d.insert_temp(warned_id, true);
                seen
            });
            if !already {
                tracing::warn!(
                    component = "item_slot",
                    name = self.log_name.as_deref().unwrap_or("item_slot"),
                    taille = self.size,
                    minimum = tokens::ITEM_SLOT_MIN_SIZE,
                    "emplacement trop petit pour son cadre"
                );
            }
        }

        let ds = DesignSystem::get(ui.ctx());

        // **La célébration déforme l'emplacement, elle ne le remplace pas** : tout ce qui suit
        // peint la même chose qu'un emplacement ordinaire, dans un carré agrandi et sur une couche
        // qui s'efface. Hors célébration, `CompletionPhase::REST` rend l'échelle à 1 et l'opacité
        // à 1 — le rendu est identique au pixel, ce que vérifient les captures déjà en place.
        //
        // **Un enfant, pas un `scope` du `ui` de l'appelant** : `new_child` ne touche pas au
        // curseur du parent (l'emplacement a déjà alloué sa place plus haut), là où un
        // `scope_builder` la réserverait une seconde fois — même raison qu'`input.rs`.
        let phase = self.completion.unwrap_or(CompletionPhase::REST);
        let rect = egui::Rect::from_center_size(rect.center(), rect.size() * phase.scale);
        let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        let ui = &mut ui;
        ui.multiply_opacity(1.0 - phase.dissolve);

        let icon_rect = egui::Rect::from_center_size(
            rect.center(),
            Vec2::splat(self.icon_side() * phase.scale),
        );

        for layer in paint_order(self.frame) {
            match layer {
                SlotLayer::Background => {
                    // **Dans l'anneau, pas sur le carré.** Peint sur `rect` avec le rayon de 2 px
                    // de l'emplacement, ce fond dépassait des quatre coins des textures de rareté,
                    // qui sont en retrait de 2 px et arrondies à 3 : on voyait un carré sombre
                    // derrière une bordure arrondie (retour utilisateur du 2026-09-13, capture à
                    // l'appui). Le même `border_ring` que le liseré le fait rentrer exactement
                    // dedans — et il remplit toujours l'emplacement d'un monstre, dont c'est le
                    // seul fond, jusque SOUS son trait (peint `Inside` sur ce même anneau).
                    let (anneau, rayon) = border_ring(rect);
                    ui.painter()
                        .rect_filled(anneau, rayon, tokens::ITEM_SLOT_BACKGROUND);
                }
                SlotLayer::Border => match self.frame {
                    SlotFrame::Rarity(rarity) => {
                        ds.paint(ui.painter(), rect, rarity.border(), egui::Color32::WHITE);
                    }
                    SlotFrame::Plain => {
                        // **Le même anneau que les textures de rareté**, pas le bord du carré :
                        // sans cela un monstre et un objet côte à côte ont leurs liserés décalés
                        // de 2 px, avec des coins qui ne suivent pas le même arc.
                        let (anneau, rayon) = border_ring(rect);
                        ui.painter().rect_stroke(
                            anneau,
                            rayon,
                            egui::Stroke::new(
                                tokens::ITEM_SLOT_PLAIN_STROKE,
                                tokens::ITEM_SLOT_PLAIN_BORDER,
                            ),
                            egui::StrokeKind::Inside,
                        );
                    }
                },
                SlotLayer::Icon => {
                    if let Some(icon) = self.icon {
                        // Inscrite dans la fenêtre de l'emplacement, à son rapport : une bannière
                        // de `monsterIllustrations` s'y pose entière et centrée, une icône carrée
                        // la remplit comme avant (voir `design::fit`).
                        let peint = fit::contain_rect(icon_rect, icon.size);
                        egui::Image::new(SizedTexture::new(icon.id, peint.size()))
                            .paint_at(ui, peint);
                    }
                }
            }
        }

        // **La couronne remplace le liseré, elle ne s'y ajoute pas** : même anneau
        // (`border_ring`), peint juste après le cadre pour le recouvrir, et sous le compteur —
        // le nombre qui vient d'atteindre sa cible doit rester lisible pendant qu'on le fête.
        if phase.crown > 0.0 {
            paint_crown(ui, rect, self.frame, phase);
        }

        if let Some(count) = self.count {
            paint_count(ui, rect, count);
        }
        if let (Some(glyph), None) = (self.glyph, self.selection) {
            paint_glyph(ui, rect, glyph, glyph_color(self.count));
        }

        // La sélection vient APRÈS tout le reste. Le liseré se pose sur le MÊME anneau que le
        // cadre : il remplace visuellement la bordure de l'emplacement, il ne s'ajoute pas à côté.
        if let Some(checked) = self.selection {
            if checked {
                let (anneau, rayon) = border_ring(rect);
                ui.painter().rect_stroke(
                    anneau,
                    rayon,
                    egui::Stroke::new(tokens::ITEM_SLOT_PLAIN_STROKE, self.selection_tone.border()),
                    egui::StrokeKind::Inside,
                );
            }
            crate::design::components::checkbox::paint(
                ui,
                egui::Rect::from_min_size(
                    rect.min + Vec2::splat(tokens::ITEM_SLOT_SELECTION_INSET),
                    Vec2::splat(tokens::CHECKBOX_SIZE),
                ),
                checked,
                self.selection_tone.checkbox_tint(checked),
            );
        }

        // L'éclat, l'onde et les particules viennent en dernier, et hors de l'opacité de la couche
        // (voir `paint_burst`).
        if self.completion.is_some() {
            paint_burst(ui, rect, self.frame, phase);
        }
        response
    }
}

/// Peint la couronne de complétion sur l'anneau du cadre — voir [`ItemSlot::completion`].
///
/// **`egui` n'a pas de dégradé conique** : l'arc-en-ciel est une suite de traits posés le long du
/// contour arrondi, chacun de la teinte correspondant à sa position, décalée par
/// [`CompletionPhase::angle`]. À [`tokens::ITEM_SLOT_COMPLETION_CROWN_SEGMENTS`] segments on n'en
/// distingue aucun à 64 px, et la couronne tourne parce que la teinte glisse le long du contour —
/// aucune géométrie ne bouge, ce qui la garde exactement sur le liseré qu'elle recouvre.
fn paint_crown(ui: &Ui, rect: egui::Rect, frame: SlotFrame, phase: CompletionPhase) {
    let (anneau, rayon) = border_ring(rect);
    let cote = rect.width().min(rect.height());
    let largeur = phase.crown * cote;
    let sceau = seal_color(frame);
    let painter = ui.painter();

    // Le halo d'abord, sous la couronne : le même contour, plus large et transparent. Il déborde
    // légèrement du carré — c'est voulu, une célébration qui tient strictement dans son cadre ne
    // se remarque pas.
    if phase.glow > 0.0 {
        let halo = (phase.glow * GLOW_ALPHA * 255.0) as u8;
        paint_ring_segments(painter, anneau, rayon, largeur * GLOW_WIDTH_FACTOR, |t| {
            crown_color(t, phase, sceau).gamma_multiply(halo as f32 / 255.0)
        });
    }

    paint_ring_segments(painter, anneau, rayon, largeur, |t| {
        crown_color(t, phase, sceau)
    });
}

/// La teinte de la couronne à la position `t` (fraction du contour), une fois la condensation
/// appliquée : arc-en-ciel pur au départ, couleur de rareté pleine à l'arrivée.
fn crown_color(t: f32, phase: CompletionPhase, sceau: egui::Color32) -> egui::Color32 {
    let arc = rainbow(t + phase.angle / std::f32::consts::TAU);
    mix(arc, sceau, phase.seal)
}

/// La couleur sur laquelle l'arc-en-ciel se condense — celle de la rareté, ou l'**or de
/// l'interface** pour un cadre simple.
///
/// Un ennemi n'a pas de rareté (`SlotFrame::Plain`), et il fallait bien lui donner une couleur de
/// fin : l'or est celle que cette interface emploie déjà pour dire « retenu, accompli »
/// ([`SelectionTone::Neutral`]), pas une teinte inventée pour l'occasion.
fn seal_color(frame: SlotFrame) -> egui::Color32 {
    match frame {
        SlotFrame::Rarity(rarity) => rarity.seal_color(),
        SlotFrame::Plain => tokens::ITEM_SLOT_SELECTED_BORDER,
    }
}

/// Une teinte de l'arc-en-ciel — `t` est un tour complet, et se replie hors de `[0, 1[`.
fn rainbow(t: f32) -> egui::Color32 {
    egui::ecolor::Hsva::new(
        t.rem_euclid(1.0),
        tokens::ITEM_SLOT_COMPLETION_RAINBOW_SATURATION,
        tokens::ITEM_SLOT_COMPLETION_RAINBOW_VALUE,
        1.0,
    )
    .into()
}

/// Interpolation linéaire entre deux couleurs opaques.
fn mix(from: egui::Color32, to: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let canal = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    egui::Color32::from_rgb(
        canal(from.r(), to.r()),
        canal(from.g(), to.g()),
        canal(from.b(), to.b()),
    )
}

/// Parcourt le contour arrondi de `rect` et y pose des traits colorés — voir [`paint_crown`].
///
/// `color` reçoit la position du segment sur le contour, de 0 à 1.
fn paint_ring_segments(
    painter: &egui::Painter,
    rect: egui::Rect,
    rayon: f32,
    largeur: f32,
    color: impl Fn(f32) -> egui::Color32,
) {
    let n = tokens::ITEM_SLOT_COMPLETION_CROWN_SEGMENTS;
    let mut precedent = ring_point(rect, rayon, 0.0);
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let point = ring_point(rect, rayon, t);
        painter.line_segment(
            [precedent, point],
            egui::Stroke::new(largeur, color(t - 0.5 / n as f32)),
        );
        precedent = point;
    }
}

/// Le point du contour arrondi de `rect` à la fraction `t` de son périmètre, en partant du milieu
/// du bord HAUT et en tournant dans le sens horaire.
///
/// Le départ n'est pas le coin haut-gauche, et ce n'est pas indifférent : la teinte de départ de
/// l'arc-en-ciel se pose ainsi au milieu d'un côté, là où l'œil la suit, plutôt que sur un coin où
/// la couture rouge → violet se remarquerait.
fn ring_point(rect: egui::Rect, rayon: f32, t: f32) -> egui::Pos2 {
    let rayon = rayon
        .min(rect.width() / 2.0)
        .min(rect.height() / 2.0)
        .max(0.0);
    let droit = rect.width() - 2.0 * rayon;
    let haut = rect.height() - 2.0 * rayon;
    let arc = std::f32::consts::FRAC_PI_2 * rayon;
    let perimetre = 2.0 * (droit + haut) + 4.0 * arc;
    // Départ au milieu du bord haut : un demi-côté droit déjà parcouru.
    let mut reste = (t.rem_euclid(1.0) * perimetre + droit / 2.0).rem_euclid(perimetre);

    // Demi-bord haut (droite), coin haut-droit, bord droit, coin bas-droit, bord bas, coin
    // bas-gauche, bord gauche, coin haut-gauche, demi-bord haut (gauche).
    let etapes: [(f32, u8); 8] = [
        (droit / 2.0, 0),
        (arc, 1),
        (haut, 2),
        (arc, 3),
        (droit, 4),
        (arc, 5),
        (haut, 6),
        (arc, 7),
    ];
    for (longueur, quoi) in etapes {
        if reste <= longueur || longueur <= 0.0 {
            let part = if longueur > 0.0 {
                reste / longueur
            } else {
                0.0
            };
            return match quoi {
                0 => egui::pos2(rect.center().x + droit / 2.0 * part, rect.top()),
                1 => coin(rect.right() - rayon, rect.top() + rayon, rayon, -0.25, part),
                2 => egui::pos2(rect.right(), rect.top() + rayon + haut * part),
                3 => coin(
                    rect.right() - rayon,
                    rect.bottom() - rayon,
                    rayon,
                    0.0,
                    part,
                ),
                4 => egui::pos2(rect.right() - rayon - droit * part, rect.bottom()),
                5 => coin(
                    rect.left() + rayon,
                    rect.bottom() - rayon,
                    rayon,
                    0.25,
                    part,
                ),
                6 => egui::pos2(rect.left(), rect.bottom() - rayon - haut * part),
                _ => coin(rect.left() + rayon, rect.top() + rayon, rayon, 0.5, part),
            };
        }
        reste -= longueur;
    }
    egui::pos2(rect.center().x, rect.top())
}

/// Un point sur le quart de cercle d'un coin — `depart` est en tours (0 = est, 0,25 = sud).
fn coin(cx: f32, cy: f32, rayon: f32, depart: f32, part: f32) -> egui::Pos2 {
    let angle = (depart + part * 0.25) * std::f32::consts::TAU;
    egui::pos2(cx + rayon * angle.cos(), cy + rayon * angle.sin())
}

/// Peint l'éclat, l'onde et les particules de dissolution — voir [`ItemSlot::completion`].
fn paint_burst(ui: &Ui, rect: egui::Rect, frame: SlotFrame, phase: CompletionPhase) {
    // **L'opacité de la couche est remise à plein** : ces trois couches sont ce qui reste quand
    // l'emplacement s'en va, elles ne doivent pas s'effacer avec lui.
    let mut painter = ui.painter().clone();
    painter.set_opacity(1.0);
    let painter = &painter;
    let centre = rect.center();
    let cote = rect.width().min(rect.height());
    let sceau = seal_color(frame);

    // L'éclat : un disque de la couleur de rareté, blanchi à cœur. Peint en additif serait plus
    // juste, mais `egui` ne mélange qu'en alpha — un blanc à faible opacité fait le même office
    // par-dessus une tuile sombre.
    if phase.flash > 0.0 {
        let rayon = cote * FLASH_RADIUS_RATIO;
        painter.circle_filled(
            centre,
            rayon,
            sceau.gamma_multiply(phase.flash * FLASH_OUTER_ALPHA),
        );
        painter.circle_filled(
            centre,
            rayon * FLASH_CORE_RATIO,
            egui::Color32::WHITE.gamma_multiply(phase.flash * FLASH_CORE_ALPHA),
        );
    }

    // L'onde : un anneau qui s'écarte en s'affinant et en pâlissant.
    if let Some(onde) = phase.wave.filter(|o| *o < 1.0) {
        painter.circle_stroke(
            centre,
            cote * (WAVE_START_RATIO + (WAVE_END_RATIO - WAVE_START_RATIO) * ease_out(onde)),
            egui::Stroke::new(
                lerp(WAVE_STROKE_MAX, WAVE_STROKE_MIN, onde),
                sceau.gamma_multiply((1.0 - onde) * WAVE_ALPHA),
            ),
        );
    }

    // Les particules : l'emplacement part vers le haut en s'éteignant. **Déterministes** — leur
    // dispersion vient d'un hachage de leur index, jamais d'une horloge : une capture de référence
    // doit rendre deux fois la même image (voir `build_info::freeze_for_snapshots`, même exigence).
    if phase.dissolve > 0.0 && phase.dissolve < 1.0 {
        let n = tokens::ITEM_SLOT_COMPLETION_MOTES;
        for i in 0..n {
            let (fx, fy, retard) = mote_seed(i);
            // Chaque particule part à son tour : celles du bas montent en premier.
            let avance =
                ((phase.dissolve - retard * MOTE_STAGGER) / (1.0 - MOTE_STAGGER)).clamp(0.0, 1.0);
            if avance <= 0.0 {
                continue;
            }
            let x = rect.left() + rect.width() * fx;
            let y = rect.top() + rect.height() * fy
                - cote * tokens::ITEM_SLOT_COMPLETION_MOTE_RISE * ease_out(avance);
            let teinte = if i % MOTE_SEAL_EVERY == 0 {
                sceau
            } else {
                tokens::ITEM_SLOT_COUNT_TEXT
            };
            painter.rect_filled(
                egui::Rect::from_center_size(
                    egui::pos2(x, y),
                    Vec2::splat(tokens::ITEM_SLOT_COMPLETION_MOTE_SIZE),
                ),
                0.0,
                teinte.gamma_multiply((1.0 - avance) * MOTE_ALPHA),
            );
        }
    }
}

/// Position et retard d'une particule de dissolution, tirés de son seul index.
///
/// Un hachage entier (constantes de Knuth) plutôt qu'un générateur : pas d'état à porter, pas
/// d'horloge, et deux exécutions rendent exactement la même dispersion.
fn mote_seed(i: usize) -> (f32, f32, f32) {
    let hash = |graine: u64| {
        let mut x = graine.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        x ^= x >> 29;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 32;
        (x >> 40) as f32 / (1u32 << 24) as f32
    };
    let i = i as u64;
    (hash(i * 3), hash(i * 3 + 1), hash(i * 3 + 2))
}

/// Opacité du halo derrière la couronne, et ce dont il est plus large qu'elle.
const GLOW_ALPHA: f32 = 0.30;
/// Voir [`GLOW_ALPHA`].
const GLOW_WIDTH_FACTOR: f32 = 2.6;

/// Rayon de l'éclat, en fraction du côté de l'emplacement, et opacités de son disque puis de son
/// cœur blanc.
const FLASH_RADIUS_RATIO: f32 = 0.62;
/// Voir [`FLASH_RADIUS_RATIO`].
const FLASH_OUTER_ALPHA: f32 = 0.55;
/// Voir [`FLASH_RADIUS_RATIO`].
const FLASH_CORE_RATIO: f32 = 0.45;
/// Voir [`FLASH_RADIUS_RATIO`].
const FLASH_CORE_ALPHA: f32 = 0.80;

/// Rayons de départ et d'arrivée de l'onde, en fraction du côté, ses deux épaisseurs et son
/// opacité de départ.
const WAVE_START_RATIO: f32 = 0.46;
/// Voir [`WAVE_START_RATIO`].
const WAVE_END_RATIO: f32 = 1.05;
/// Voir [`WAVE_START_RATIO`].
const WAVE_STROKE_MAX: f32 = 3.0;
/// Voir [`WAVE_START_RATIO`].
const WAVE_STROKE_MIN: f32 = 0.6;
/// Voir [`WAVE_START_RATIO`].
const WAVE_ALPHA: f32 = 0.7;

/// Part de la dissolution consacrée à l'échelonnement des particules — à 0, elles partiraient
/// toutes ensemble, et l'emplacement disparaîtrait d'un bloc.
const MOTE_STAGGER: f32 = 0.45;
/// Une particule sur combien prend la couleur du sceau plutôt que le blanc du compteur.
const MOTE_SEAL_EVERY: usize = 3;
/// Opacité de départ d'une particule.
const MOTE_ALPHA: f32 = 0.9;

/// Peint le compteur dans le coin bas-droit.
///
/// Un nombre cerné de noir, **sans pastille** : les captures du jeu n'en montrent aucune, et le
/// badge en pilule qui débordait de la tuile a été retiré pour cette raison. Le cerne est simple
/// (1 px) — un double contour essayé un temps a été jugé « trop » en conditions réelles.
fn paint_count(ui: &Ui, rect: egui::Rect, count: SlotCount) {
    let right = rect.right() - tokens::ITEM_SLOT_COUNT_INSET_RIGHT;
    let bottom = rect.bottom() - tokens::ITEM_SLOT_COUNT_INSET_BOTTOM;
    match count {
        SlotCount::Simple(value) => {
            text::paint_outlined_text(
                ui,
                egui::pos2(right, bottom),
                egui::Align2::RIGHT_BOTTOM,
                &value.to_string(),
                egui::FontId::monospace(tokens::ITEM_SLOT_COUNT_FONT_SIZE),
                tokens::ITEM_SLOT_COUNT_TEXT,
                text::OUTLINE_FULL,
            );
        }
        SlotCount::Target(target) => {
            // Même ancrage et mêmes jetons que la cible d'une fraction : les deux tombent au même
            // endroit, seule la ligne du courant disparaît.
            text::paint_outlined_text(
                ui,
                egui::pos2(right, bottom),
                egui::Align2::RIGHT_BOTTOM,
                &format!("/{target}"),
                egui::FontId::monospace(tokens::ITEM_SLOT_TARGET_FONT_SIZE),
                tokens::ITEM_SLOT_TARGET_TEXT,
                text::OUTLINE_FULL,
            );
        }
        SlotCount::Fraction { current, target } => {
            // La FRACTION garde l'ancrage bas — l'emplacement standard de tous les suivis
            // incrémentaux — et le nombre courant remonte au-dessus. Demande explicite du
            // 2026-09-06, qui inverse le point de référence d'un essai précédent.
            text::paint_outlined_text(
                ui,
                egui::pos2(right, bottom),
                egui::Align2::RIGHT_BOTTOM,
                &format!("/{target}"),
                egui::FontId::monospace(tokens::ITEM_SLOT_TARGET_FONT_SIZE),
                tokens::ITEM_SLOT_TARGET_TEXT,
                text::OUTLINE_FULL,
            );
            text::paint_outlined_text(
                ui,
                egui::pos2(right, bottom - tokens::ITEM_SLOT_TARGET_LINE_OFFSET),
                egui::Align2::RIGHT_BOTTOM,
                &current.to_string(),
                egui::FontId::monospace(tokens::ITEM_SLOT_COUNT_FONT_SIZE),
                tokens::ITEM_SLOT_COUNT_CURRENT,
                text::OUTLINE_FULL,
            );
        }
    }
}

/// Couleur du glyphe de mode : **celle du texte qu'il accompagne**, jamais une couleur à lui.
///
/// Dans le bandeau, la fraction met le nombre courant en or et c'est lui que l'œil lit : le glyphe
/// est en or. Dans l'onglet Suivi, seule la cible s'affiche, en gris : le glyphe est gris. Sans
/// compteur (cas théorique), le blanc du compteur simple.
pub fn glyph_color(count: Option<SlotCount>) -> egui::Color32 {
    match count {
        Some(SlotCount::Fraction { .. }) => tokens::ITEM_SLOT_COUNT_CURRENT,
        Some(SlotCount::Target(_)) => tokens::ITEM_SLOT_TARGET_TEXT,
        Some(SlotCount::Simple(_)) | None => tokens::ITEM_SLOT_COUNT_TEXT,
    }
}

/// Carré du glyphe de mode dans un emplacement : coin **haut-gauche**, à
/// [`tokens::ITEM_SLOT_GLYPH_INSET`] du bord sur les deux axes — le coin de la case à cocher,
/// son cerne noir posé là où elle commence, hors de l'anneau du liseré.
pub fn glyph_rect(rect: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        rect.min + Vec2::splat(tokens::ITEM_SLOT_GLYPH_INSET),
        Vec2::splat(tokens::ITEM_SLOT_GLYPH_SIZE),
    )
}

/// Peint le glyphe de mode — voir [`SlotGlyph`] et [`glyph_rect`].
///
/// Cerné de noir par le même procédé que [`text::paint_outlined_text`] : une copie noire par
/// décalage de [`text::OUTLINE_FULL`], puis la forme pleine. Le fond d'une tuile est arbitraire
/// (icône claire ou sombre), une ombre d'un seul côté ne suffirait pas.
fn paint_glyph(ui: &Ui, rect: egui::Rect, glyph: SlotGlyph, color: egui::Color32) {
    let boite = glyph_rect(rect);
    let painter = ui.painter();
    let peindre = |offset: Vec2, couleur: egui::Color32| {
        let b = boite.translate(offset);
        match glyph {
            SlotGlyph::Goal => {
                // Hampe sur toute la hauteur, fanion triangulaire accroché en haut.
                painter.line_segment(
                    [b.left_top(), b.left_bottom()],
                    egui::Stroke::new(1.0, couleur),
                );
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        egui::pos2(b.left() + 1.0, b.top()),
                        egui::pos2(b.right(), b.top() + b.height() * 0.3),
                        egui::pos2(b.left() + 1.0, b.top() + b.height() * 0.6),
                    ],
                    couleur,
                    egui::Stroke::NONE,
                ));
            }
            SlotGlyph::Countdown => {
                // Anneau et point, la cible du switch web réduite à sa plus simple forme.
                let rayon = b.width() / 2.0;
                painter.circle_stroke(b.center(), rayon - 0.5, egui::Stroke::new(1.0, couleur));
                painter.circle_filled(b.center(), rayon * 0.3, couleur);
            }
        }
    };
    for offset in text::OUTLINE_FULL {
        peindre(*offset, egui::Color32::BLACK);
    }
    peindre(Vec2::ZERO, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_glyphe_prend_la_couleur_du_texte_qu_il_accompagne() {
        assert_eq!(
            glyph_color(Some(SlotCount::Fraction {
                current: 2,
                target: 5
            })),
            tokens::ITEM_SLOT_COUNT_CURRENT
        );
        assert_eq!(
            glyph_color(Some(SlotCount::Target(5))),
            tokens::ITEM_SLOT_TARGET_TEXT
        );
        assert_eq!(glyph_color(None), tokens::ITEM_SLOT_COUNT_TEXT);
    }

    #[test]
    fn le_glyphe_tient_le_coin_haut_gauche_a_deux_pixels_au_moins_du_lisere() {
        // Retour du 2026-09-17 : « en haut à gauche, deux à trois pixels d'écart de la bordure, en
        // haut et sur le côté ». Même retrait sur les deux axes, et le liseré (pixels 2 à 4) reste
        // lisible entre le bord et le glyphe, avec deux pixels de fond avant même son cerne.
        let carre =
            egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(tokens::ITEM_SLOT_SIZE));
        let g = glyph_rect(carre);
        assert_eq!(
            g.min,
            egui::pos2(tokens::ITEM_SLOT_GLYPH_INSET, tokens::ITEM_SLOT_GLYPH_INSET)
        );
        assert_eq!(g.size(), Vec2::splat(tokens::ITEM_SLOT_GLYPH_SIZE));
        let (anneau, _) = border_ring(carre);
        let fin_du_lisere = anneau.left() - carre.left() + tokens::ITEM_SLOT_PLAIN_STROKE;
        assert!(
            g.left() - 1.0 >= fin_du_lisere + 2.0 && g.top() - 1.0 >= fin_du_lisere + 2.0,
            "le glyphe ({}) doit laisser au moins 2 px de fond après le liseré, qui finit à {fin_du_lisere}",
            g.left()
        );
        // Le même coin que la case à cocher, son cerne d'un pixel posé là où elle commence : en
        // mode sélection, elle le remplace.
        assert_eq!(
            g.min - Vec2::splat(1.0),
            carre.min + Vec2::splat(tokens::ITEM_SLOT_SELECTION_INSET)
        );
    }

    #[test]
    fn une_bordure_de_rarete_se_peint_sous_l_icone() {
        // **Le bug que ce test verrouille** : la fenêtre intérieure de `Border-*.webp` n'est pas un
        // trou transparent mais un aplat teinté à ~70 %. Peinte après l'icône, elle la recouvre
        // entièrement — « j'ai l'impression que tu as mis les objets en opacité ».
        let ordre = paint_order(SlotFrame::Rarity(ItemRarity::Legendary));
        let border = ordre.iter().position(|l| *l == SlotLayer::Border).unwrap();
        let icon = ordre.iter().position(|l| *l == SlotLayer::Icon).unwrap();
        assert!(border < icon, "la bordure de rareté passe SOUS l'icône");
    }

    #[test]
    fn un_cadre_simple_se_peint_au_contraire_par_dessus() {
        // Un liseré net, pas un aplat : il doit rester visible si l'icône déborde. L'inverse de la
        // bordure de rareté, et c'est pour porter cette différence que `paint_order` existe.
        let ordre = paint_order(SlotFrame::Plain);
        let border = ordre.iter().position(|l| *l == SlotLayer::Border).unwrap();
        let icon = ordre.iter().position(|l| *l == SlotLayer::Icon).unwrap();
        assert!(icon < border, "un cadre simple se pose par-dessus l'icône");
    }

    #[test]
    fn l_anneau_du_lisere_tombe_sur_celui_des_textures_de_rarete() {
        // **Le bug que ce test verrouille** (2026-09-13) : le cadre simple et le liseré de
        // sélection se peignaient au bord du carré, avec des rayons de coin choisis à l'œil (2 et
        // 4). Les textures de rareté, elles, posent leur liseré 2 px plus au centre — un monstre et
        // un objet côte à côte n'avaient donc pas la même bordure, et cocher une tuile traçait un
        // or décalé « au-delà de la bordure de l'item slot ».
        //
        // Les deux cotes attendues sont celles MESURÉES sur le canevas de 128 des sept fichiers :
        // l'alpha saute en x = 4, et l'arc extérieur a un rayon de 6. À l'échelle de rendu (64),
        // cela fait 2 et 3.
        let carre =
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::splat(tokens::ITEM_SLOT_SIZE));
        let (anneau, rayon) = border_ring(carre);
        assert_eq!(anneau.left() - carre.left(), 2.0, "marge du liseré à 64 px");
        assert_eq!(carre.right() - anneau.right(), 2.0, "marge symétrique");
        assert_eq!(rayon, 3.0, "rayon du coin du liseré à 64 px");
    }

    #[test]
    fn les_deux_tons_ne_different_que_par_la_couleur() {
        // La règle de la demande, mot pour mot : « exactement les mêmes choses, c'est juste la
        // couleur qui change ». Un ton qui se mettrait à décider d'une géométrie — un liseré plus
        // épais pour « insister » sur la suppression — romprait la seule chose que l'utilisateur a
        // demandé de garder identique.
        assert_ne!(
            SelectionTone::Neutral.border(),
            SelectionTone::Danger.border(),
            "les deux tons doivent se distinguer"
        );
        assert_eq!(
            SelectionTone::Danger.border(),
            tokens::INFO_ALERT,
            "le rouge est celui du bouton « Annuler », pas un rouge choisi à l'œil"
        );
    }

    #[test]
    fn seule_la_case_cochee_prend_le_ton_destructif() {
        // Une case vide annonce un geste possible, pas un objet retenu : la teindre dirait « ces
        // huit tuiles vont être supprimées » alors que rien n'a été choisi.
        assert_eq!(
            SelectionTone::Danger.checkbox_tint(false),
            egui::Color32::WHITE,
            "une case décochée reste neutre, même en mode suppression"
        );
        assert_eq!(
            SelectionTone::Danger.checkbox_tint(true),
            tokens::ITEM_SLOT_SELECTED_BORDER_DANGER,
            "une case cochée porte le ton, comme le liseré"
        );
        // Le ton neutre ne teinte jamais : le tint multiplie, et l'or de la case est déjà dans
        // l'asset.
        for coche in [false, true] {
            assert_eq!(
                SelectionTone::Neutral.checkbox_tint(coche),
                egui::Color32::WHITE
            );
        }
    }

    #[test]
    fn la_case_a_cocher_degage_le_lisere_au_lieu_de_s_y_coller() {
        // Deux retours utilisateur sur ce seul retrait : « décaler la checkbox d'un pixel », puis
        // « d'au moins 2 px ». Ce que l'un et l'autre demandent est que le liseré reste LISIBLE tout
        // du long — donc que la case commence franchement après lui, pas à son contact.
        let carre =
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::splat(tokens::ITEM_SLOT_SIZE));
        let (anneau, _) = border_ring(carre);
        let fin_du_lisere = anneau.left() - carre.left() + tokens::ITEM_SLOT_PLAIN_STROKE;
        assert!(
            tokens::ITEM_SLOT_SELECTION_INSET >= fin_du_lisere + 2.0,
            "la case ({}) doit dégager le liseré, qui finit à {fin_du_lisere}",
            tokens::ITEM_SLOT_SELECTION_INSET
        );
    }

    #[test]
    fn l_anneau_suit_le_cote_de_l_emplacement() {
        // Des **fractions**, pas des cotes : la texture est étirée sur tout le carré (`ICON_SLICE`,
        // un 9-slice dégénéré), donc son liseré suit la taille. Un anneau figé en pixels serait
        // faux partout ailleurs qu'à 64.
        for cote in [32.0_f32, 64.0, 128.0] {
            let carre = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::splat(cote));
            let (anneau, rayon) = border_ring(carre);
            assert_eq!(
                anneau.left() - carre.left(),
                cote / 32.0,
                "marge à {cote} px"
            );
            assert_eq!(rayon, cote * 6.0 / 128.0, "rayon à {cote} px");
        }
    }

    #[test]
    fn le_fond_est_toujours_la_premiere_couche() {
        for frame in [SlotFrame::Plain, SlotFrame::Rarity(ItemRarity::Common)] {
            assert_eq!(paint_order(frame)[0], SlotLayer::Background);
        }
    }

    #[test]
    fn les_sept_raretes_ont_chacune_leur_bordure() {
        // Une paire copiée-collée dans le `match` donnerait deux raretés à la même texture, sans
        // que rien ne bronche : un objet légendaire porterait le cadre d'un mythique.
        let bordures: Vec<_> = [
            ItemRarity::Common,
            ItemRarity::Rare,
            ItemRarity::Mythical,
            ItemRarity::Legendary,
            ItemRarity::Memory,
            ItemRarity::Epic,
            ItemRarity::Relic,
        ]
        .iter()
        .map(|r| r.border())
        .collect();
        let uniques: std::collections::HashSet<_> = bordures.iter().collect();
        assert_eq!(uniques.len(), 7, "deux raretés partagent une bordure");
    }

    #[test]
    fn chaque_cadre_a_sa_propre_taille_d_icone_et_toutes_deux_tiennent_dans_le_carre() {
        // Les deux tailles sont **indépendantes**, et c'est la leçon de la migration : une bordure
        // de rareté impose sa fenêtre intérieure (≈ 0,797 du côté, réduite encore de 4 %), un cadre
        // simple n'impose rien — d'où une cote propre, qui vient du template web. Aucune des deux
        // ne se déduit de l'autre ; la seule propriété commune est qu'elles tiennent dans le carré.
        let cote = tokens::ITEM_SLOT_SIZE;
        let avec = item_slot()
            .frame(SlotFrame::Rarity(ItemRarity::Epic))
            .icon_side();
        let sans = item_slot().frame(SlotFrame::Plain).icon_side();
        for (side, quoi) in [(avec, "bordure de rareté"), (sans, "cadre simple")] {
            assert!(
                side > 0.0 && side <= cote,
                "{quoi} : icône de {side} dans un carré de {cote}"
            );
        }
        assert!(
            (avec - sans).abs() > 1.0,
            "les deux cotes sont distinctes : les confondre a produit un monstre deux fois trop \
             gros dans sa tuile",
        );
    }

    #[test]
    fn une_completion_passe_par_ses_cinq_moments_dans_l_ordre() {
        // Le découpage est une décision, pas une mesure — ce test le fige pour que personne ne le
        // « simplifie » sans le voir : sans lui, réordonner deux bornes de `tokens.rs` ne casse
        // rien à la compilation et rend une animation qui éclate avant d'avoir tourné.
        let repos = completion_phase(-1.0);
        assert_eq!(repos, CompletionPhase::REST, "rien avant le franchissement");

        let leve = completion_phase(tokens::ITEM_SLOT_COMPLETION_LIFT_END * 0.5);
        assert!(leve.scale > 1.0, "l'emplacement se soulève");
        assert_eq!(leve.seal, 0.0, "il ne se scelle pas encore");
        assert!(leve.wave.is_none(), "aucune onde avant l'éclat");
        assert_eq!(
            leve.flash, 0.0,
            "et surtout AUCUN éclat : la première version en peignait un à pleine puissance dès \
             la première image, faute de garde avant la fin de la rotation",
        );

        let tourne = completion_phase(tokens::ITEM_SLOT_COMPLETION_SPIN_END * 0.9);
        assert!(
            tourne.angle > 0.0 && tourne.crown > 0.0,
            "la couronne tourne"
        );
        assert_eq!(tourne.seal, 0.0, "arc-en-ciel pur tant qu'elle tourne");
        assert_eq!(tourne.flash, 0.0, "l'éclat n'arrive qu'à la condensation");
        assert_eq!(tourne.dissolve, 0.0, "rien ne se dissout encore");

        let scelle = completion_phase(
            (tokens::ITEM_SLOT_COMPLETION_SPIN_END + tokens::ITEM_SLOT_COMPLETION_SEAL_END) / 2.0,
        );
        assert!(
            scelle.seal > 0.0 && scelle.seal < 1.0,
            "la condensation est en cours",
        );
        assert!(scelle.flash > 0.0, "l'éclat accompagne la condensation");
        assert!(scelle.wave.is_some(), "l'onde est partie avec l'éclat");

        let fond = completion_phase(
            (tokens::ITEM_SLOT_COMPLETION_DISSOLVE_START
                + tokens::ITEM_SLOT_COMPLETION_DISSOLVE_END)
                / 2.0,
        );
        assert_eq!(fond.seal, 1.0, "scellé sur la rareté avant de partir");
        assert!(fond.dissolve > 0.0 && fond.dissolve < 1.0, "il se dissout");
        assert!(!fond.finished());

        let fini = completion_phase(tokens::ITEM_SLOT_COMPLETION_DURATION);
        assert!(fini.finished(), "plus rien à peindre au terme");
        assert_eq!(fini.crown, 0.0, "et surtout plus de couronne");
    }

    #[test]
    fn la_rotation_accelere_au_lieu_d_etre_reguliere() {
        // C'est ce qui fait lire l'éclat comme une conclusion plutôt que comme une interruption
        // (voir `completion_phase`) : le dernier tiers du temps doit emporter bien plus que le
        // tiers de la rotation.
        let debut = tokens::ITEM_SLOT_COMPLETION_LIFT_END;
        let span = tokens::ITEM_SLOT_COMPLETION_SPIN_END - debut;
        let premier = completion_phase(debut + span / 3.0).angle;
        let dernier = completion_phase(tokens::ITEM_SLOT_COMPLETION_SPIN_END).angle
            - completion_phase(debut + 2.0 * span / 3.0).angle;
        assert!(
            dernier > premier * 3.0,
            "le dernier tiers ({dernier} rad) doit emporter beaucoup plus que le premier \
             ({premier} rad)",
        );
    }

    #[test]
    fn chaque_rarete_se_scelle_sur_une_couleur_qui_lui_est_propre() {
        // Les sept teintes sont MESURÉES sur les `Border-*.webp` (voir `seal_color`) : deux
        // raretés qui partageraient la leur signalerait une mesure recopiée, pas une coïncidence.
        let couleurs: Vec<_> = [
            ItemRarity::Common,
            ItemRarity::Rare,
            ItemRarity::Mythical,
            ItemRarity::Legendary,
            ItemRarity::Memory,
            ItemRarity::Epic,
            ItemRarity::Relic,
        ]
        .into_iter()
        .map(|r| r.seal_color().to_array())
        .collect();
        let uniques: std::collections::HashSet<_> = couleurs.iter().collect();
        assert_eq!(uniques.len(), 7, "deux raretés partagent leur sceau");
    }

    #[test]
    fn un_ennemi_se_scelle_sur_l_or_de_l_interface() {
        // Il n'a pas de rareté : le repli doit être une couleur DÉJÀ employée ici pour dire
        // « accompli », pas une teinte inventée pour l'occasion.
        assert_eq!(
            seal_color(SlotFrame::Plain),
            tokens::ITEM_SLOT_SELECTED_BORDER
        );
    }

    #[test]
    fn la_couronne_suit_le_contour_arrondi_sans_jamais_en_sortir() {
        // La couronne se pose sur l'anneau du cadre (`border_ring`) : un point qui en sortirait
        // peindrait par-dessus la tuile voisine.
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), Vec2::splat(64.0));
        let (anneau, rayon) = border_ring(rect);
        for i in 0..64 {
            let point = ring_point(anneau, rayon, i as f32 / 64.0);
            assert!(
                anneau.expand(0.01).contains(point),
                "point {point:?} hors de l'anneau {anneau:?}",
            );
        }
        // Et le tour est FERMÉ : la fin rejoint le départ, sinon la couture se verrait.
        let depart = ring_point(anneau, rayon, 0.0);
        let arrivee = ring_point(anneau, rayon, 1.0);
        assert!(
            depart.distance(arrivee) < 0.01,
            "le contour ne se referme pas : {depart:?} vs {arrivee:?}",
        );
    }

    #[test]
    fn les_particules_de_dissolution_sont_deterministes() {
        // Une capture de référence doit rendre deux fois la même image — d'où un hachage d'index
        // et non un générateur semé sur l'horloge (voir `mote_seed`).
        for i in 0..tokens::ITEM_SLOT_COMPLETION_MOTES {
            let (x, y, retard) = mote_seed(i);
            assert_eq!((x, y, retard), mote_seed(i), "tirage instable pour {i}");
            for (valeur, quoi) in [(x, "x"), (y, "y"), (retard, "retard")] {
                assert!(
                    (0.0..1.0).contains(&valeur),
                    "{quoi} hors bornes pour {i} : {valeur}",
                );
            }
        }
    }
}
