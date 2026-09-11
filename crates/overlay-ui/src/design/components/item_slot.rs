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

/// Construit un emplacement d'objet vide.
pub fn item_slot() -> ItemSlot {
    ItemSlot {
        frame: SlotFrame::Plain,
        icon: None,
        count: None,
        size: tokens::ITEM_SLOT_SIZE,
        log_name: None,
    }
}

/// Voir [`item_slot`].
pub struct ItemSlot {
    frame: SlotFrame,
    icon: Option<egui::TextureId>,
    count: Option<SlotCount>,
    size: f32,
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
        let ds = DesignSystem::get(ui.ctx());
        let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(self.icon_side()));

        for layer in paint_order(self.frame) {
            match layer {
                SlotLayer::Background => {
                    ui.painter().rect_filled(
                        rect,
                        tokens::ITEM_SLOT_ROUNDING,
                        tokens::SLOT_BACKGROUND,
                    );
                }
                SlotLayer::Border => match self.frame {
                    SlotFrame::Rarity(rarity) => {
                        ds.paint(ui.painter(), rect, rarity.border(), egui::Color32::WHITE);
                    }
                    SlotFrame::Plain => {
                        ui.painter().rect_stroke(
                            rect,
                            tokens::ITEM_SLOT_ROUNDING,
                            egui::Stroke::new(
                                tokens::ITEM_SLOT_PLAIN_STROKE,
                                tokens::SLOT_PLAIN_BORDER,
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
        response
    }
}

/// Peint le compteur dans le coin bas-droit.
///
/// Un nombre cerné de noir, **sans pastille** : les captures du jeu n'en montrent aucune, et le
/// badge en pilule qui débordait de la tuile a été retiré pour cette raison. Le cerne est simple
/// (1 px) — un double contour essayé un temps a été jugé « trop » en conditions réelles.
fn paint_count(ui: &Ui, rect: egui::Rect, count: SlotCount) {
    let right = rect.right() - tokens::SLOT_COUNT_INSET_RIGHT;
    let bottom = rect.bottom() - tokens::SLOT_COUNT_INSET_BOTTOM;
    match count {
        SlotCount::Simple(value) => {
            text::paint_outlined_text(
                ui,
                egui::pos2(right, bottom),
                egui::Align2::RIGHT_BOTTOM,
                &value.to_string(),
                egui::FontId::monospace(tokens::SLOT_COUNT_FONT_SIZE),
                tokens::SLOT_COUNT_TEXT,
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
                egui::FontId::monospace(tokens::SLOT_TARGET_FONT_SIZE),
                tokens::SLOT_TARGET_TEXT,
                text::OUTLINE_FULL,
            );
            text::paint_outlined_text(
                ui,
                egui::pos2(right, bottom - tokens::SLOT_TARGET_LINE_OFFSET),
                egui::Align2::RIGHT_BOTTOM,
                &current.to_string(),
                egui::FontId::monospace(tokens::SLOT_COUNT_FONT_SIZE),
                tokens::SLOT_COUNT_CURRENT,
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
