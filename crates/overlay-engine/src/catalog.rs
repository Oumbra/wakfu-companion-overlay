//! Catalogue objets/monstres (lot L3, §7.4 du plan) — pour l'instant réduit au STRICT nécessaire
//! pour résoudre une icône réelle d'objet/monstre suivi dans le panneau Suivi (retour utilisateur
//! 2026-09-02 : « comme les images de ressources/monstres n'est pas présent c'est très compliqué
//! pour l'utilisateur » de distinguer les tuiles entre elles avec la même icône générique
//! partout). PAS encore le catalogue complet visé au plan (résolution de recette, familles de
//! monstres, donjons, repli hors-ligne embarqué) — uniquement de quoi transformer un nom/id
//! d'objet ou de monstre en référence d'icône (`IconRef`), miroir réduit de
//! `CatalogService.findWakfuItemEntry`/`findWakfuMonsterEntry` (`catalog.service.ts`).
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
/// `wakfu-companion`, dont l'arité DOIT rester en phase avec cette struct). Recette/catégorie ne
/// sont pas encore exploitées ici — seules l'icône (`gfxId`) et la rareté intéressent cette
/// itération — mais doivent rester déclarées pour que la désérialisation positionnelle de serde
/// consomme le tuple entier plutôt que de rejeter la ligne pour arité inattendue.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct RawItemRow(i64, String, String, String, String, i64, i64, i64, i64);

/// Miroir de `RawItemRow` pour `data["monsters"]` — `[id, fr, en, es, pt, gfxId, family(-1 si
/// null), isBoss(0|1), isArchi(0|1), isDominant(0|1)]`. Même remarque sur les champs non encore
/// lus (famille/boss/archi/dominant).
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
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

#[derive(Clone)]
struct ItemEntry {
    icon: IconRef,
    rarity: WakfuRarity,
}

struct MonsterEntry {
    icon: IconRef,
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
    monsters_by_name: HashMap<String, IconRef>,
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
        for RawItemRow(id, fr, en, es, pt, gfx_id, rarity_sort_order, ..) in raw_items {
            let entry = ItemEntry {
                icon: IconRef {
                    kind: IconKind::Item,
                    gfx_id: gfx_id.to_string(),
                },
                rarity: rarity_from_sort_order(rarity_sort_order),
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
        for RawMonsterRow(id, fr, en, es, pt, gfx_id, ..) in raw_monsters {
            let icon = IconRef {
                kind: IconKind::Monster,
                gfx_id,
            };
            for name in [&fr, &en, &es, &pt] {
                index
                    .monsters_by_name
                    .entry(normalize_wakfu_name(name))
                    .or_insert_with(|| icon.clone());
            }
            index.monsters_by_id.insert(id, MonsterEntry { icon });
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

    /// Miroir de `find_item_icon` pour un monstre.
    pub fn find_monster_icon(&self, name: &str, catalog_id: Option<i64>) -> Option<IconRef> {
        if let Some(id) = catalog_id {
            if let Some(entry) = self.monsters_by_id.get(&id) {
                return Some(entry.icon.clone());
            }
        }
        self.monsters_by_name
            .get(&normalize_wakfu_name(name))
            .cloned()
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
                // raritySortOrder=2 -> "rare" (voir rarity_from_sort_order).
                [24029, "Larme d'Ogrest", "Ogrest's Tear", "Lágrima de Ogrest", "Lágrima de Ogrest", 1234, 2, 0, 1],
            ],
            "monsters": [
                [24875, "El Pochito", "El Pochito", "El Pochito", "El Pochito", "5421", -1, 1, 0, 0],
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
