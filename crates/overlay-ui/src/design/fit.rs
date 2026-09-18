//! **Une image de contenu se peint à son rapport natif** — la règle `object-fit: contain` que le
//! web applique à toutes ses images distantes (`entity-icon.component.ts`,
//! `item-icon.component.ts` : `object-fit: contain` sur une boîte carrée).
//!
//! ## Le bug qu'elle répare (2026-09-17)
//!
//! Les images du CDN `wakassets` ne sont pas toutes carrées. `monsters/` l'est, mais
//! `monsterIllustrations/` — le second dossier essayé pour un monstre, seule source de 34 des 61
//! monstres d'un fichier utilisateur (voir `IconRef::image_paths`) — sert des **bannières
//! rectangulaires**. Peintes dans le carré d'un portrait ou d'un emplacement, elles y étaient
//! **étirées** : « les images provenant de `wakassets/monsterIllustrations` sont déformées ».
//!
//! Le même défaut avait déjà été signalé sur la gemme de rareté de l'autocomplétion (13 × 20,
//! « très fortement agrandies et aplaties », 2026-09-12), réparé à l'époque dans ce seul
//! composant. La correction vit maintenant ICI, et les composants prennent une
//! [`egui::load::SizedTexture`] plutôt qu'un `TextureId` nu : une texture y arrive **toujours**
//! avec sa taille native, et plus aucun appelant ne peut oublier de la fournir.
//!
//! ## Inscrire, jamais recadrer
//!
//! `contain` et non `cover` : une bannière de boss recadrée en carré perdrait la tête du monstre,
//! justement ce qui l'identifie. Inscrite, elle reste entière — plus petite dans sa boîte, ce qui
//! est le comportement du site.

use egui::{Rect, Vec2};

/// Taille à laquelle peindre une image de `native` pixels pour l'inscrire dans `box_size`, **à son
/// rapport**.
///
/// Agrandit comme elle réduit : une icône de 32 px dans une boîte de 40 en occupe bien 40, comme
/// avant cette règle — seul le rapport est désormais préservé.
///
/// Une taille native dégénérée (zéro ou négative — une texture pas encore uploadée, un appelant
/// qui n'a rien de mieux) rend la boîte telle quelle plutôt qu'un `NaN` qui se propagerait
/// jusqu'au sommet peint.
pub fn contain(box_size: Vec2, native: Vec2) -> Vec2 {
    if native.x <= 0.0 || native.y <= 0.0 {
        return box_size;
    }
    native * (box_size.x / native.x).min(box_size.y / native.y)
}

/// Rectangle **centré dans `box_rect`** où peindre une image de `native` pixels — voir [`contain`].
///
/// C'est cette fonction que les composants appellent : `paint_at` peint sur tout le rectangle
/// qu'on lui donne, donc conserver un rapport revient toujours à lui donner un rectangle plus
/// petit, pas à régler une option de l'image.
pub fn contain_rect(box_rect: Rect, native: Vec2) -> Rect {
    Rect::from_center_size(box_rect.center(), contain(box_rect.size(), native))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_image_carree_remplit_sa_boite() {
        // Le cas de l'immense majorité des icônes (`wakassets/monsters`, `items`, `spells`) : la
        // règle ne doit RIEN y changer, sinon elle serait une régression déguisée.
        assert_eq!(
            contain(Vec2::splat(40.0), Vec2::splat(64.0)),
            Vec2::splat(40.0)
        );
    }

    #[test]
    fn une_banniere_est_limitee_par_sa_largeur() {
        // 200 × 100 dans un carré de 40 : 40 × 20, et non 40 × 40 comme avant (deux fois trop
        // haute, donc écrasée).
        assert_eq!(
            contain(Vec2::splat(40.0), Vec2::new(200.0, 100.0)),
            Vec2::new(40.0, 20.0)
        );
    }

    #[test]
    fn une_gemme_est_limitee_par_sa_hauteur() {
        // La gemme de rareté du jeu, 13 × 20 dans sa boîte de 14 — le cas déjà corrigé à la main
        // dans `autocomplete`, qui doit donner exactement le même résultat ici.
        let taille = contain(Vec2::splat(14.0), Vec2::new(13.0, 20.0));
        assert!((taille.x - 9.1).abs() < 0.01, "{taille:?}");
        assert_eq!(taille.y, 14.0);
    }

    #[test]
    fn une_taille_native_degeneree_rend_la_boite() {
        assert_eq!(contain(Vec2::splat(40.0), Vec2::ZERO), Vec2::splat(40.0));
        assert_eq!(
            contain(Vec2::splat(40.0), Vec2::new(10.0, 0.0)),
            Vec2::splat(40.0)
        );
    }

    #[test]
    fn le_rectangle_reste_centre_sur_la_boite() {
        let boite = Rect::from_min_size(egui::pos2(10.0, 10.0), Vec2::splat(40.0));
        let peint = contain_rect(boite, Vec2::new(200.0, 100.0));
        assert_eq!(peint.center(), boite.center());
        assert_eq!(peint.size(), Vec2::new(40.0, 20.0));
    }
}
