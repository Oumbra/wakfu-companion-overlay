//! **Emplacement d'objet** du design system Wakfu — le carré qui porte une icône d'objet, sa
//! bordure de rareté et son compteur. Composant **feuille** (§1 du contrat).
//!
//! ```ignore
//! use overlay_ui::design::{self, ItemRarity, SlotFrame};
//!
//! ui.add(
//!     design::item_slot()
//!         .frame(SlotFrame::Rarity(ItemRarity::Legendary))
//!         .icon(texture_id)
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
//! L'icône arrive en [`egui::TextureId`], **déjà résolue**. Ce n'est pas une entorse au contrat
//! (« aucune texture en paramètre ») : cette règle vise les assets du design system, que le
//! composant doit résoudre depuis une intention. Une icône d'objet est du **contenu** — elle est
//! téléchargée, mise en cache et indexée par le catalogue, tout cela hors du design system. Le
//! composant ne saurait pas la nommer.
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

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::design::{text, tokens, DesignSystem, DsTexture};

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
        size: tokens::ITEM_SLOT_SIZE,
        selection: None,
        log_name: None,
    }
}

/// Voir [`item_slot`].
pub struct ItemSlot {
    frame: SlotFrame,
    icon: Option<egui::TextureId>,
    count: Option<SlotCount>,
    size: f32,
    selection: Option<bool>,
    log_name: Option<String>,
}

impl ItemSlot {
    /// Cadre. Par défaut [`SlotFrame::Plain`].
    pub fn frame(mut self, frame: SlotFrame) -> Self {
        self.frame = frame;
        self
    }

    /// Icône déjà résolue — voir la doc de module sur pourquoi ce n'est pas une texture du design
    /// system. Sans icône, l'emplacement est peint vide.
    pub fn icon(mut self, icon: egui::TextureId) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Compteur incrusté. Sans appel, aucun compteur.
    pub fn count(mut self, count: SlotCount) -> Self {
        self.count = Some(count);
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
                    ui.painter().rect_filled(
                        rect,
                        tokens::ITEM_SLOT_ROUNDING,
                        tokens::ITEM_SLOT_BACKGROUND,
                    );
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
                        egui::Image::new(egui::load::SizedTexture::new(icon, icon_rect.size()))
                            .paint_at(ui, icon_rect);
                    }
                }
            }
        }

        if let Some(count) = self.count {
            paint_count(ui, rect, count);
        }

        // La sélection vient APRÈS tout le reste. Le liseré se pose sur le MÊME anneau que le
        // cadre : il remplace visuellement la bordure de l'emplacement, il ne s'ajoute pas à côté.
        if let Some(checked) = self.selection {
            if checked {
                let (anneau, rayon) = border_ring(rect);
                ui.painter().rect_stroke(
                    anneau,
                    rayon,
                    egui::Stroke::new(
                        tokens::ITEM_SLOT_PLAIN_STROKE,
                        tokens::ITEM_SLOT_SELECTED_BORDER,
                    ),
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

#[cfg(test)]
mod tests {
    use super::*;

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
