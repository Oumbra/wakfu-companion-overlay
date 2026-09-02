//! Catalogue objets/monstres (lot L3, §7.4 du plan) — miroir réduit de `CatalogService.
//! findWakfuItemEntry`/`findWakfuMonsterEntry` (`catalog.service.ts`). Né du STRICT nécessaire
//! pour résoudre une icône réelle d'objet/monstre suivi dans le panneau Suivi (retour utilisateur
//! 2026-09-02 : « comme les images de ressources/monstres n'est pas présent c'est très compliqué
//! pour l'utilisateur » de distinguer les tuiles entre elles avec la même icône générique
//! partout), étendu depuis à la résolution de la recette (`find_item_has_recipe`) et du classement
//! boss/archimonstre/dominant d'un monstre (`find_monster_classification`, voir
//! `MonsterClassification`) — la résolution des DONJONS et des FAMILLES DE MONSTRES par elles-mêmes
//! (noms localisés, pas seulement l'id de famille d'un monstre) vit dans `dungeon.rs`/
//! `monster_family.rs`, deux modules frères plutôt qu'ici (formats serveur différents — tableaux
//! d'objets complets, pas des tuples compacts, voir leur doc de tête).
//!
//! Pas d'IO ici, comme `roster.rs`/`watchlist.rs` : `CatalogIndex::from_compact_json` prend un
//! `serde_json::Value` déjà récupéré par `overlay-sync` (voir `client::fetch_catalog_index`,
//! format documenté par `server/catalog/compact-index.ts` côté `wakfu-companion` — tuples, PAS
//! d'objets à clés répétées, ~1,14 Mo bruts / ~348 Ko gzip pour ~11 700 entrées).

use std::collections::HashMap;

use serde::Deserialize;

use crate::roster::normalize_wakfu_name;

/// D'où vient l'icône — détermine le sous-dossier `wakassets` (voir `IconRef::image_url`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconKind {
    Item,
    Monster,
}

/// Référence suffisante pour construire l'URL de l'icône réelle — voir `image_url`. `gfx_id` est
/// toujours une chaîne ici même pour un objet (dont le `gfxId` catalogue est numérique) : la seule
/// utilisation qu'on en fait est une interpolation dans une URL, pas de calcul dessus.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IconRef {
    pub kind: IconKind,
    pub gfx_id: String,
}

impl IconRef {
    /// Miroir exact d'`itemImageCandidates`/`monsterImageCandidates` (`item-icon.component.ts`,
    /// `entity-icon.component.ts`) — CDN communautaire `wakassets`, seule source retenue par le
    /// web pour ce niveau de détail (pas de repli officiel Ankama, `pictureUrl` volontairement
    /// exclu de l'index compact, voir sa doc côté serveur). Un seul candidat ici (pas les deux
    /// sources `monsters`/`monsterIllustrations` du web pour un monstre) : suffisant pour la
    /// grande majorité des cas, un vrai repli en cascade viendra avec le reste du lot L3 si
    /// nécessaire une fois le premier résultat observé en usage réel.
    pub fn image_url(&self) -> String {
        let folder = match self.kind {
            IconKind::Item => "items",
            IconKind::Monster => "monsters",
        };
        format!(
            "https://vertylo.github.io/wakassets/{folder}/{}.png",
            self.gfx_id
        )
    }
}

/// Rareté d'objet — miroir de `WakfuRarity` (`wakfu-item-rarity.data.ts`). `Old` (« Ancien »,
/// objets historiques retirés du jeu) n'est en pratique jamais renvoyée au runtime côté web (les
/// objets `old` sont exclus de ce qui est exposé par le catalogue serveur) ; conservée ici
/// uniquement pour que `rarity_from_sort_order` couvre bien les 8 valeurs de `RARITY_SORT_ORDER`
/// sans repli arbitraire sur une valeur voisine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakfuRarity {
    Old,
    Common,
    Rare,
    Mythical,
    Legendary,
    Memory,
    Epic,
    Relic,
}

/// Miroir de `RARITY_SORT_ORDER` (`wakfu-item-rarity.data.ts`) — l'index compact
/// (`server/catalog/compact-index.ts`) encode la rareté par cet entier, pas par son nom. Toute
/// valeur inconnue (référentiel étendu côté serveur avant ce module) retombe sur `Common`, comme
/// `getWakfuItemRarity` (`?? 'common'`) pour un objet non résolu.
fn rarity_from_sort_order(order: i64) -> WakfuRarity {
    match order {
        0 => WakfuRarity::Old,
        2 => WakfuRarity::Rare,
        3 => WakfuRarity::Mythical,
        4 => WakfuRarity::Legendary,
        5 => WakfuRarity::Memory,
        6 => WakfuRarity::Epic,
        7 => WakfuRarity::Relic,
        _ => WakfuRarity::Common, // 1 (Common lui-même) ET tout ordre non reconnu.
    }
}

/// Un tuple positionnel de `data["items"]` — `[id, fr, en, es, pt, gfxId, raritySortOrder,
/// hasRecipe(0|1), categorySortOrder]` (voir `server/catalog/compact-index.ts` côté
/// `wakfu-companion`, dont l'arité DOIT rester en phase avec cette struct). `categorySortOrder`
/// n'est pas encore exploité ici (aucun filtre par catégorie côté overlay, contrairement à
/// l'autocomplétion web) — mais doit rester déclaré pour que la désérialisation positionnelle de
/// serde consomme le tuple entier plutôt que de rejeter la ligne pour arité inattendue.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct RawItemRow(i64, String, String, String, String, i64, i64, i64, i64);

/// Miroir de `RawItemRow` pour `data["monsters"]` — `[id, fr, en, es, pt, gfxId, family(-1 si
/// null), isBoss(0|1), isArchi(0|1), isDominant(0|1)]`.
#[derive(Debug, Clone, Deserialize)]
struct RawMonsterRow(
    i64,
    String,
    String,
    String,
    String,
    String,
    i64,
    i64,
    i64,
    i64,
);

/// Classement d'un monstre au sein de son combat — miroir (partiel) de la priorité utilisée par
/// `resolveFightTypeClassification` (`fight-image.util.ts`) : `boss > archimonstre > dominant`.
/// Le 4ᵉ palier web (« plus gros dégât ») n'a pas d'équivalent ici — c'est une donnée de combat en
/// cours, pas une propriété statique du référentiel, donc hors du périmètre de `CatalogIndex` ; un
/// futur consommateur (panneau Combat conscient du donjon, pas encore construit — voir §9 du plan,
/// PAS encore dans la liste des panneaux) l'ajoutera lui-même par-dessus ce classement de base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MonsterClassification {
    // Ordre de variantes délibéré : `Ord` dérivé croît dans le MÊME ordre de priorité que
    // `resolveFightTypeClassification` (`None` le plus faible, `Boss` le plus fort) — un
    // `.max()` entre plusieurs monstres d'un même combat donne directement la bonne réponse,
    // sans réécrire la comparaison à la main côté appelant.
    None,
    Dominant,
    Archi,
    Boss,
}

#[derive(Clone)]
struct ItemEntry {
    icon: IconRef,
    rarity: WakfuRarity,
    has_recipe: bool,
}

#[derive(Clone)]
struct MonsterEntry {
    icon: IconRef,
    family_id: Option<i64>,
    is_boss: bool,
    is_archi: bool,
    is_dominant: bool,
}

impl MonsterEntry {
    fn classification(&self) -> MonsterClassification {
        if self.is_boss {
            MonsterClassification::Boss
        } else if self.is_archi {
            MonsterClassification::Archi
        } else if self.is_dominant {
            MonsterClassification::Dominant
        } else {
            MonsterClassification::None
        }
    }
}

/// Index en RAM, construit une seule fois — O(1) par nom normalisé ET par id, jamais un balayage
/// linéaire (voir la mise en garde de §7.4 du plan : « le piège vécu côté web, ~10 s de gel, n'est
/// pas une bonne intention à éviter, c'est un test de non-régression »).
#[derive(Default)]
pub struct CatalogIndex {
    items_by_id: HashMap<i64, ItemEntry>,
    /// Nom `fr` prioritaire — un objet homonyme dans une autre langue écraserait sinon le nom
    /// français au chargement (l'index est construit dans l'ordre `items`, une seule passe) ; en
    /// pratique la distinction ne joue que si le NOM (normalisé) est identique entre deux entrées
    /// différentes, cas limite déjà accepté tel quel côté web (`findWakfuItemEntry`, « renvoie une
    /// seule entrée arbitrairement »).
    items_by_name: HashMap<String, ItemEntry>,
    monsters_by_id: HashMap<i64, MonsterEntry>,
    /// Même priorité `fr` qu'`items_by_name` ci-dessus, même raison.
    monsters_by_name: HashMap<String, MonsterEntry>,
}

impl CatalogIndex {
    /// Construit l'index depuis `{ items: [...], monsters: [...] }` — la réponse de
    /// `GET /api/v1/catalog/` telle quelle (voir `client::fetch_catalog_index`). Une ligne mal
    /// formée (tuple incomplet/type inattendu) est silencieusement ignorée plutôt que de faire
    /// échouer tout le chargement — un catalogue partiel (repli sur l'icône générique pour les
    /// entrées manquantes) reste préférable à aucun catalogue du tout.
    pub fn from_compact_json(data: &serde_json::Value) -> Self {
        let mut index = Self::default();

        let raw_items: Vec<RawItemRow> = data
            .get("items")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        for RawItemRow(id, fr, en, es, pt, gfx_id, rarity_sort_order, has_recipe, ..) in raw_items {
            let entry = ItemEntry {
                icon: IconRef {
                    kind: IconKind::Item,
                    gfx_id: gfx_id.to_string(),
                },
                rarity: rarity_from_sort_order(rarity_sort_order),
                has_recipe: has_recipe != 0,
            };
            for name in [&fr, &en, &es, &pt] {
                index
                    .items_by_name
                    .entry(normalize_wakfu_name(name))
                    .or_insert_with(|| entry.clone());
            }
            index.items_by_id.insert(id, entry);
        }

        let raw_monsters: Vec<RawMonsterRow> = data
            .get("monsters")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        for RawMonsterRow(id, fr, en, es, pt, gfx_id, family, is_boss, is_archi, is_dominant) in
            raw_monsters
        {
            let entry = MonsterEntry {
                icon: IconRef {
                    kind: IconKind::Monster,
                    gfx_id,
                },
                family_id: (family != -1).then_some(family),
                is_boss: is_boss != 0,
                is_archi: is_archi != 0,
                is_dominant: is_dominant != 0,
            };
            for name in [&fr, &en, &es, &pt] {
                index
                    .monsters_by_name
                    .entry(normalize_wakfu_name(name))
                    .or_insert_with(|| entry.clone());
            }
            index.monsters_by_id.insert(id, entry);
        }

        index
    }

    /// Résout l'icône d'un objet — `catalog_id` (capturé sans ambiguïté à l'ajout du suivi, voir
    /// `overlay_engine::WatchlistEntry::catalog_id`) prioritaire sur le nom, miroir de la même
    /// préférence côté web (`findWakfuItemEntryById` avant `findWakfuItemEntry(name)` partout où
    /// l'id est déjà connu). `None` si le catalogue n'est pas encore chargé ou si l'objet n'y est
    /// pas trouvé — jamais une erreur, l'appelant retombe sur l'icône générique dans les deux cas.
    pub fn find_item_icon(&self, name: &str, catalog_id: Option<i64>) -> Option<IconRef> {
        self.find_item_entry(name, catalog_id)
            .map(|entry| entry.icon.clone())
    }

    /// Rareté d'un objet — miroir de `getWakfuItemRarity` (`wakfu-item-rarity.data.ts`), même
    /// repli sur `Common` pour un objet non résolu (catalogue pas encore chargé, ou nom introuvable
    /// — ex. ajouté au suivi sous un nom que le référentiel ne connaît pas).
    pub fn find_item_rarity(&self, name: &str, catalog_id: Option<i64>) -> WakfuRarity {
        self.find_item_entry(name, catalog_id)
            .map_or(WakfuRarity::Common, |entry| entry.rarity)
    }

    fn find_item_entry(&self, name: &str, catalog_id: Option<i64>) -> Option<&ItemEntry> {
        if let Some(id) = catalog_id {
            if let Some(entry) = self.items_by_id.get(&id) {
                return Some(entry);
            }
        }
        self.items_by_name.get(&normalize_wakfu_name(name))
    }

    /// A-t-il une recette de craft ? Miroir du booléen `hasRecipe` de l'index compact (`server/
    /// catalog/compact-index.ts`), lui-même reflétant `WakfuItemRow.hasRecipe` — un simple drapeau,
    /// pas la recette elle-même (ingrédients/quantités), que l'index compact n'embarque volontairement
    /// pas (voir sa doc de tête côté `wakfu-companion` : alourdirait l'index pour un besoin qu'aucun
    /// panneau de l'overlay ne couvre encore, §9 du plan). `false` pour un objet non résolu, comme
    /// `find_item_rarity`.
    pub fn find_item_has_recipe(&self, name: &str, catalog_id: Option<i64>) -> bool {
        self.find_item_entry(name, catalog_id)
            .is_some_and(|entry| entry.has_recipe)
    }

    /// Miroir de `find_item_icon` pour un monstre.
    pub fn find_monster_icon(&self, name: &str, catalog_id: Option<i64>) -> Option<IconRef> {
        self.find_monster_entry(name, catalog_id)
            .map(|entry| entry.icon.clone())
    }

    /// Classement boss/archimonstre/dominant d'un monstre — voir `MonsterClassification`.
    /// `MonsterClassification::None` pour un monstre non résolu ou sans classement particulier,
    /// jamais une erreur (même repli que le reste de ce module).
    pub fn find_monster_classification(
        &self,
        name: &str,
        catalog_id: Option<i64>,
    ) -> MonsterClassification {
        self.find_monster_entry(name, catalog_id)
            .map_or(MonsterClassification::None, MonsterEntry::classification)
    }

    /// Id de famille de monstre (`monster_families.id`, voir `monster_family.rs`) — `None` si le
    /// monstre n'a pas de famille encyclopédie connue (28 sur 851 au référentiel actuel, voir doc
    /// serveur) ou n'est pas résolu.
    pub fn find_monster_family_id(&self, name: &str, catalog_id: Option<i64>) -> Option<i64> {
        self.find_monster_entry(name, catalog_id)
            .and_then(|entry| entry.family_id)
    }

    fn find_monster_entry(&self, name: &str, catalog_id: Option<i64>) -> Option<&MonsterEntry> {
        if let Some(id) = catalog_id {
            if let Some(entry) = self.monsters_by_id.get(&id) {
                return Some(entry);
            }
        }
        self.monsters_by_name.get(&normalize_wakfu_name(name))
    }

    pub fn is_empty(&self) -> bool {
        self.items_by_id.is_empty() && self.monsters_by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> serde_json::Value {
        serde_json::json!({
            "items": [
                // raritySortOrder=2 -> "rare" (voir rarity_from_sort_order). hasRecipe=0.
                [24029, "Larme d'Ogrest", "Ogrest's Tear", "Lágrima de Ogrest", "Lágrima de Ogrest", 1234, 2, 0, 1],
                // hasRecipe=1 — objet craftable, voir find_item_has_recipe.
                [9001, "Pain de Craqueleur", "Craqueleur Bread", "Pan de Craqueleur", "Pão de Craqueleur", 4321, 1, 1, 6],
            ],
            "monsters": [
                // isBoss=1, pas de famille (-1).
                [24875, "El Pochito", "El Pochito", "El Pochito", "El Pochito", "5421", -1, 1, 0, 0],
                // family=42, isArchi=1.
                [501, "Bwork Archi", "Bwork Archi", "Bwork Archi", "Bwork Archi", "9999", 42, 0, 1, 0],
                // family=42, isDominant=1 — même famille que le précédent, classement différent.
                [502, "Bwork Dominant", "Bwork Dominant", "Bwork Dominant", "Bwork Dominant", "8888", 42, 0, 0, 1],
                // aucun classement particulier ni famille.
                [503, "Bwork Lambda", "Bwork Lambda", "Bwork Lambda", "Bwork Lambda", "7777", -1, 0, 0, 0],
            ],
        })
    }

    #[test]
    fn resout_un_objet_par_id() {
        let index = CatalogIndex::from_compact_json(&sample());
        let icon = index.find_item_icon("peu importe", Some(24029)).unwrap();
        assert_eq!(icon.kind, IconKind::Item);
        assert_eq!(icon.gfx_id, "1234");
    }

    #[test]
    fn resout_un_objet_par_nom_insensible_a_la_casse_et_aux_accents() {
        let index = CatalogIndex::from_compact_json(&sample());
        let icon = index.find_item_icon("larme d'ogrest", None).unwrap();
        assert_eq!(icon.gfx_id, "1234");
    }

    #[test]
    fn resout_la_rarete_dun_objet_par_id() {
        let index = CatalogIndex::from_compact_json(&sample());
        assert_eq!(
            index.find_item_rarity("peu importe", Some(24029)),
            WakfuRarity::Rare
        );
    }

    #[test]
    fn objet_non_resolu_retombe_sur_la_rarete_commune() {
        let index = CatalogIndex::from_compact_json(&sample());
        assert_eq!(
            index.find_item_rarity("Introuvable", None),
            WakfuRarity::Common
        );
    }

    #[test]
    fn resout_un_monstre_par_id_avec_un_gfx_id_texte() {
        let index = CatalogIndex::from_compact_json(&sample());
        let icon = index.find_monster_icon("peu importe", Some(24875)).unwrap();
        assert_eq!(icon.kind, IconKind::Monster);
        assert_eq!(icon.gfx_id, "5421");
    }

    #[test]
    fn resout_le_drapeau_de_recette() {
        let index = CatalogIndex::from_compact_json(&sample());
        assert!(index.find_item_has_recipe("peu importe", Some(9001)));
        assert!(!index.find_item_has_recipe("peu importe", Some(24029)));
        assert!(!index.find_item_has_recipe("Introuvable", None));
    }

    #[test]
    fn classe_un_boss() {
        let index = CatalogIndex::from_compact_json(&sample());
        assert_eq!(
            index.find_monster_classification("peu importe", Some(24875)),
            MonsterClassification::Boss
        );
    }

    #[test]
    fn classe_un_archimonstre_et_un_dominant_distinctement() {
        let index = CatalogIndex::from_compact_json(&sample());
        assert_eq!(
            index.find_monster_classification("peu importe", Some(501)),
            MonsterClassification::Archi
        );
        assert_eq!(
            index.find_monster_classification("peu importe", Some(502)),
            MonsterClassification::Dominant
        );
    }

    #[test]
    fn monstre_sans_classement_ni_famille_retombe_sur_none() {
        let index = CatalogIndex::from_compact_json(&sample());
        assert_eq!(
            index.find_monster_classification("peu importe", Some(503)),
            MonsterClassification::None
        );
        assert_eq!(index.find_monster_family_id("peu importe", Some(503)), None);
        assert_eq!(
            index.find_monster_classification("Introuvable", None),
            MonsterClassification::None
        );
    }

    #[test]
    fn resout_lid_de_famille_dun_monstre_par_nom() {
        let index = CatalogIndex::from_compact_json(&sample());
        assert_eq!(index.find_monster_family_id("bwork archi", None), Some(42));
    }

    #[test]
    fn ordre_de_priorite_boss_gt_archi_gt_dominant_gt_none() {
        use MonsterClassification::{Archi, Boss, Dominant, None as ClNone};
        assert!(Boss > Archi);
        assert!(Archi > Dominant);
        assert!(Dominant > ClNone);
    }

    #[test]
    fn objet_inconnu_renvoie_none_sans_paniquer() {
        let index = CatalogIndex::from_compact_json(&sample());
        assert!(index.find_item_icon("Inconnu", None).is_none());
        assert!(index.find_item_icon("Inconnu", Some(999)).is_none());
    }

    #[test]
    fn catalogue_absent_ou_vide_ne_plante_pas() {
        let index = CatalogIndex::from_compact_json(&serde_json::json!({}));
        assert!(index.is_empty());
        assert!(index.find_item_icon("quoi que ce soit", None).is_none());
    }

    #[test]
    fn image_url_utilise_le_bon_sous_dossier() {
        let item = IconRef {
            kind: IconKind::Item,
            gfx_id: "1234".to_string(),
        };
        assert_eq!(
            item.image_url(),
            "https://vertylo.github.io/wakassets/items/1234.png"
        );
        let monster = IconRef {
            kind: IconKind::Monster,
            gfx_id: "5421".to_string(),
        };
        assert_eq!(
            monster.image_url(),
            "https://vertylo.github.io/wakassets/monsters/5421.png"
        );
    }
}
