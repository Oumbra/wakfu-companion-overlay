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

/// Glyphe de **mode** incrusté dans le coin bas-gauche — le pendant du compteur, à l'autre coin.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotGlyph {
    /// Décompte — une cible : anneau et point.
    Countdown,
    /// Objectif — un drapeau : hampe et fanion.
    Goal,
}

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

    /// Glyphe de mode au coin bas-gauche — voir [`SlotGlyph`]. Sans appel, aucun glyphe.
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
        let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(self.icon_side()));

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

        if let Some(count) = self.count {
            paint_count(ui, rect, count);
        }
        if let Some(glyph) = self.glyph {
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
        response
    }
}

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

/// Carré du glyphe de mode dans un emplacement : coin bas-gauche, **posé sur la ligne de base de
/// la fraction** (même hauteur que les chiffres de `/cible`, retour du 2026-09-17), en retrait du
/// liseré.
pub fn glyph_rect(rect: egui::Rect) -> egui::Rect {
    let left = rect.left() + tokens::ITEM_SLOT_GLYPH_INSET_LEFT;
    let bottom = rect.bottom()
        - tokens::ITEM_SLOT_COUNT_INSET_BOTTOM
        - tokens::ITEM_SLOT_GLYPH_BASELINE_LIFT;
    egui::Rect::from_min_max(
        egui::pos2(left, bottom - tokens::ITEM_SLOT_GLYPH_SIZE),
        egui::pos2(left + tokens::ITEM_SLOT_GLYPH_SIZE, bottom),
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
    fn le_glyphe_se_pose_sur_la_ligne_de_base_de_la_fraction_en_retrait_du_lisere() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(64.0));
        let g = glyph_rect(rect);
        // Bas du glyphe = bas du texte de la cible (bottom - 4) remonté de la descente (2).
        assert_eq!(g.bottom(), 64.0 - 4.0 - 2.0);
        assert_eq!(g.height(), tokens::ITEM_SLOT_GLYPH_SIZE);
        // Retrait gauche = retrait droit du compteur : les deux coins se répondent, et le glyphe
        // reste hors de l'anneau du liseré (2 px de retrait + 2 px de trait).
        assert_eq!(g.left(), tokens::ITEM_SLOT_COUNT_INSET_RIGHT);
        assert!(g.left() >= tokens::ITEM_SLOT_PLAIN_STROKE * 2.0);
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
}
