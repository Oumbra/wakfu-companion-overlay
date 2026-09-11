//! **Le pont entre la rareté du moteur et celle du design system** — une fonction, et c'est tout
//! l'intérêt du fichier.
//!
//! `overlay_engine::WakfuRarity` est une donnée du jeu : elle vient du catalogue, elle a un ordre
//! de tri et un numéro d'icône Ankama. [`design::ItemRarity`](crate::design::ItemRarity) est une
//! **intention de peinture** : sept bordures, rien d'autre. §6 du contrat de composant interdit au
//! design system de connaître le moteur, et c'est ce qui lui permet d'être rendu par le harnais
//! sans catalogue ni session.
//!
//! La conversion vivait dans `panels::watchlist`, qui était seul à en avoir besoin. Elle est
//! remontée ici le 2026-09-11 quand les **maquettes du testkit** sont passées au composant :
//! `panels/*` n'est pas une bibliothèque, un exemple n'a pas à traverser un panneau pour traduire
//! une rareté — et deux copies de cette table divergeraient au premier oubli du repli d'`Old`.

use overlay_engine::WakfuRarity;

use crate::design::ItemRarity;

/// La bordure à peindre pour une rareté du catalogue.
///
/// **`Old` retombe sur `Common`** : le jeu n'a pas de bordure « ancien », et cette rareté n'est de
/// toute façon jamais résolue au runtime (le catalogue serveur n'expose pas les objets `old` —
/// voir la doc de `WakfuRarity`). C'est ce que faisait déjà `UiIcons::item_border` avant que le
/// composant n'existe.
pub fn to_slot_rarity(rarity: WakfuRarity) -> ItemRarity {
    match rarity {
        WakfuRarity::Old | WakfuRarity::Common => ItemRarity::Common,
        WakfuRarity::Rare => ItemRarity::Rare,
        WakfuRarity::Mythical => ItemRarity::Mythical,
        WakfuRarity::Legendary => ItemRarity::Legendary,
        WakfuRarity::Memory => ItemRarity::Memory,
        WakfuRarity::Epic => ItemRarity::Epic,
        WakfuRarity::Relic => ItemRarity::Relic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_retombe_sur_commun() {
        assert_eq!(to_slot_rarity(WakfuRarity::Old), ItemRarity::Common);
    }

    #[test]
    fn les_sept_bordures_sont_toutes_atteignables() {
        // Sans ce test, deux raretés du moteur pourraient pointer la même bordure et une septième
        // ne jamais sortir — invisible à la relecture d'un `match` de huit lignes.
        let atteintes: Vec<ItemRarity> = [
            WakfuRarity::Common,
            WakfuRarity::Rare,
            WakfuRarity::Mythical,
            WakfuRarity::Legendary,
            WakfuRarity::Memory,
            WakfuRarity::Epic,
            WakfuRarity::Relic,
        ]
        .into_iter()
        .map(to_slot_rarity)
        .collect();
        for bordure in [
            ItemRarity::Common,
            ItemRarity::Rare,
            ItemRarity::Mythical,
            ItemRarity::Legendary,
            ItemRarity::Memory,
            ItemRarity::Epic,
            ItemRarity::Relic,
        ] {
            assert!(
                atteintes.contains(&bordure),
                "{bordure:?} n'est jamais peinte"
            );
        }
    }
}
