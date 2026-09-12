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
    /// Gemme de rareté (`wakassets/rarities/{n}.png`) — le pictogramme qui précède le nom d'un
    /// objet dans le panneau de suggestions, miroir de `wakfuRarityIconUrl`
    /// (`wakfu-item-rarity.data.ts`).
    ///
    /// Ce n'est **pas** un asset du design system mais une image distante, du même CDN et du même
    /// genre que les icônes d'objets : elle passe donc par le même `RemoteIconStore`, sans rien
    /// d'embarqué. Le `gfx_id` porte ici le **numéro d'icône** de la rareté — voir
    /// [`rarity_icon_number`], et surtout sa mise en garde.
    Rarity,
    /// Icône de catégorie (`wakassets/itemTypes/{n}.png`) — les pictogrammes de la bande de
    /// filtres en tête de l'autocomplétion, miroir de `wakfuItemCategoryIconUrl`
    /// (`wakfu-item-category.data.ts`).
    ///
    /// Même nature que [`IconKind::Rarity`] : une image distante, pas un asset embarqué. Le nom du
    /// sous-dossier (`itemTypes`) est celui d'Ankama ; le web, lui, parle de « catégorie » partout
    /// — d'où le nom de la variante, qui suit le vocabulaire du produit plutôt que celui du CDN.
    ///
    /// Le `gfx_id` porte le numéro d'icône : celui d'une catégorie ([`WakfuItemCategory::
    /// icon_number`]), ou l'un des deux numéros à part que le filtre utilise sans qu'ils soient des
    /// catégories — voir [`IconRef::for_all_categories`] et [`IconRef::for_monster_category`].
    ItemCategory,
    /// Icône de sort (`wakassets/spells/{n}.png`) — le bloc « ligne de sorts » du panneau Combat
    /// (`overlay-ui::panels::combat_spell_block`), résolue par `crate::spells::SpellIndex` depuis
    /// le référentiel embarqué `assets/spells.json`. Même nature que les autres variantes : une
    /// image distante, téléchargée et mise en cache par le même `RemoteIconStore`. Le `gfx_id`
    /// est le numéro de fichier extrait de l'URL `picture` du référentiel.
    Spell,
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
            IconKind::Rarity => "rarities",
            IconKind::ItemCategory => "itemTypes",
            IconKind::Spell => "spells",
        };
        format!(
            "https://vertylo.github.io/wakassets/{folder}/{}.png",
            self.gfx_id
        )
    }

    /// La gemme d'une rareté — `wakassets/rarities/{n}.png`, comme `wakfuRarityIconUrl` côté web.
    ///
    /// Rien à résoudre dans le catalogue : la rareté suffit, contrairement à l'icône d'un objet
    /// qui a besoin de son `gfxId`. D'où un constructeur plutôt qu'une méthode de `CatalogIndex`.
    pub fn for_rarity(rarity: WakfuRarity) -> Self {
        Self {
            kind: IconKind::Rarity,
            gfx_id: rarity_icon_number(rarity).to_string(),
        }
    }

    /// L'icône d'une catégorie d'objet — `wakassets/itemTypes/{n}.png`, comme
    /// `wakfuItemCategoryIconUrl` côté web.
    pub fn for_item_category(category: WakfuItemCategory) -> Self {
        Self {
            kind: IconKind::ItemCategory,
            gfx_id: category.icon_number().to_string(),
        }
    }

    /// L'icône du bouton « Tout » de la bande de filtres — miroir de
    /// `WAKFU_ALL_CATEGORY_ICON_URL`.
    ///
    /// **Ce n'est pas une catégorie** : c'est la remise à zéro du filtre, d'où un constructeur à
    /// part plutôt qu'une variante de plus dans [`WakfuItemCategory`]. Son numéro est `-1`, qui
    /// n'appartient à aucun objet.
    pub fn for_all_categories() -> Self {
        Self {
            kind: IconKind::ItemCategory,
            gfx_id: "-1".to_string(),
        }
    }

    /// L'icône du filtre « Monstres » — miroir de `WAKFU_MONSTER_CATEGORY_ICON_URL`.
    ///
    /// **Pas une catégorie d'objet non plus** : le web filtre par `kind === 'enemy'`, pas par la
    /// catégorie d'un objet, et ce filtre n'existe que dans le domaine qui mélange les deux.
    pub fn for_monster_category() -> Self {
        Self {
            kind: IconKind::ItemCategory,
            gfx_id: "282".to_string(),
        }
    }
}

/// Catégorie large d'un objet — miroir de `WakfuItemCategory` (`wakfu-item-category.data.ts`).
///
/// **L'ordre de déclaration est celui de `WAKFU_ITEM_CATEGORIES`**, donc celui dans lequel le web
/// range les boutons de sa bande de filtres. Il coïncide avec `ITEM_CATEGORY_SORT_ORDER`, l'entier
/// par lequel l'index compact encode la catégorie d'un objet (`categorySortOrder`, 9ᵉ champ de
/// [`RawItemRow`]).
///
/// Aucune variante « Monstres » : un monstre n'est pas un objet — le web filtre dessus par `kind`,
/// pas par catégorie (voir [`IconRef::for_monster_category`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WakfuItemCategory {
    Equipment,
    Resources,
    Sublimations,
    Harvests,
    HavenBag,
    Cosmetics,
    Craft,
    Misc,
}

impl WakfuItemCategory {
    /// La catégorie encodée par l'index compact — miroir d'`ITEM_CATEGORY_SORT_ORDER`
    /// (`wakfu-item-category.data.ts`), qui est l'ordre de déclaration de cette énumération.
    ///
    /// Repli sur `Misc` pour tout ordre inconnu, comme le web retombe sur `"misc"` : une catégorie
    /// ajoutée côté serveur avant ce module ne doit pas faire disparaître l'objet du référentiel,
    /// juste le ranger dans « Divers ».
    ///
    /// **À ne pas confondre avec [`WakfuItemCategory::icon_number`]** : l'ordre de tri sert à
    /// l'encodage, le numéro d'icône vient de l'encyclopédie. Les deux n'ont aucun rapport — voir
    /// le test `ordre_de_tri_et_numero_dicone_sont_deux_tables_distinctes`.
    fn from_sort_order(order: i64) -> Self {
        match order {
            1 => WakfuItemCategory::Resources,
            2 => WakfuItemCategory::Sublimations,
            3 => WakfuItemCategory::Harvests,
            4 => WakfuItemCategory::HavenBag,
            5 => WakfuItemCategory::Cosmetics,
            6 => WakfuItemCategory::Craft,
            0 => WakfuItemCategory::Equipment,
            _ => WakfuItemCategory::Misc, // 7 (Misc lui-même) ET tout ordre non reconnu.
        }
    }

    /// Numéro d'icône `itemTypes` — miroir d'`ITEM_CATEGORY_ICON_NUMBER`. Ce sont les ids de
    /// l'arbre de filtre « Types » de l'encyclopédie officielle, sans rapport avec l'ordre de tri :
    /// ne pas les dériver de la position dans l'énumération.
    pub fn icon_number(self) -> i32 {
        match self {
            WakfuItemCategory::Equipment => 109,
            WakfuItemCategory::Resources => 226,
            WakfuItemCategory::Sublimations => 602,
            WakfuItemCategory::Harvests => 237,
            WakfuItemCategory::HavenBag => 295,
            WakfuItemCategory::Cosmetics => 525,
            WakfuItemCategory::Craft => 761,
            WakfuItemCategory::Misc => 385,
        }
    }
}

/// Numéro d'icône de rareté Ankama — miroir de `RARITY_ICON_NUMBER` (`wakfu-item-rarity.data.ts`),
/// c'est-à-dire la rareté numérique BRUTE d'Ankama (`definition.rarity` des gamedata).
///
/// **À ne pas confondre avec l'ordre de tri** ([`rarity_from_sort_order`]) : les deux tables
/// coïncident jusqu'à `Legendary` puis divergent — le tri classe souvenir 5, épique 6, relique 7,
/// alors que le jeu numérote relique 5, souvenir 6, épique 7. Prendre l'une pour l'autre affiche
/// la gemme d'une autre rareté pour ces trois-là, sans rien casser par ailleurs : exactement le
/// genre d'écart qu'on ne voit pas à la relecture, d'où le test `les_deux_tables_de_rarete_
/// divergent_bien`.
fn rarity_icon_number(rarity: WakfuRarity) -> u8 {
    match rarity {
        WakfuRarity::Old => 0,
        WakfuRarity::Common => 1,
        WakfuRarity::Rare => 2,
        WakfuRarity::Mythical => 3,
        WakfuRarity::Legendary => 4,
        WakfuRarity::Relic => 5,
        WakfuRarity::Memory => 6,
        WakfuRarity::Epic => 7,
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
/// `wakfu-companion`, dont l'arité DOIT rester en phase avec cette struct).
///
/// Les neuf champs sont désormais tous exploités — `categorySortOrder` était le dernier à ne
/// servir qu'à faire consommer le tuple entier par serde ; il alimente maintenant
/// [`CatalogIndex::find_item_category`].
#[derive(Debug, Clone, Deserialize)]
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
    id: i64,
    icon: IconRef,
    rarity: WakfuRarity,
    has_recipe: bool,
    category: WakfuItemCategory,
}

#[derive(Clone)]
struct MonsterEntry {
    id: i64,
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
        for RawItemRow(
            id,
            fr,
            en,
            es,
            pt,
            gfx_id,
            rarity_sort_order,
            has_recipe,
            category_sort_order,
        ) in raw_items
        {
            let entry = ItemEntry {
                id,
                icon: IconRef {
                    kind: IconKind::Item,
                    gfx_id: gfx_id.to_string(),
                },
                rarity: rarity_from_sort_order(rarity_sort_order),
                has_recipe: has_recipe != 0,
                category: WakfuItemCategory::from_sort_order(category_sort_order),
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
                id,
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

    /// Catégorie large d'un objet — miroir de `getWakfuItemCategory` (`wakfu-item-category.data.
    /// ts`), même repli sur `Misc` pour un objet non résolu (catalogue pas encore chargé, ou nom
    /// introuvable).
    ///
    /// C'est ce qui permet à la bande de filtres de l'autocomplétion de n'afficher que les
    /// catégories réellement présentes dans les résultats — sans elle, `IconKind::ItemCategory`
    /// sait construire l'URL d'une icône que rien ne sait choisir.
    pub fn find_item_category(&self, name: &str, catalog_id: Option<i64>) -> WakfuItemCategory {
        self.find_item_entry(name, catalog_id)
            .map_or(WakfuItemCategory::Misc, |entry| entry.category)
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

    /// Id Ankama d'un objet par NOM SEUL — miroir de `itemPayload`/`HistorySyncService.monsterId`
    /// (`history-sync.service.ts`) : sert à remplir `itemId`/`monsterId` (L5, §7.1 du plan) à partir
    /// d'un nom brut lu dans le log, jamais l'inverse (contrairement à `find_item_icon`, qui
    /// préfère un id déjà connu). `None` si le catalogue n'est pas encore chargé ou si l'objet n'y
    /// est pas trouvé — l'appelant retombe alors sur le nom brut (`itemName`), jamais une erreur.
    pub fn find_item_id(&self, name: &str) -> Option<i64> {
        self.items_by_name
            .get(&normalize_wakfu_name(name))
            .map(|entry| entry.id)
    }

    /// Miroir de `find_item_id` pour un monstre — voir `HistorySyncService.monsterId`.
    pub fn find_monster_id(&self, name: &str) -> Option<i64> {
        self.monsters_by_name
            .get(&normalize_wakfu_name(name))
            .map(|entry| entry.id)
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

    /// Drapeau `isArchi` BRUT — contrairement à `find_monster_classification` (qui priorise
    /// `boss > archi > dominant`, donc masque `isArchi` pour un monstre qui est AUSSI un boss),
    /// cette méthode répond directement à la question "cette entrée porte-t-elle le drapeau
    /// archimonstre ?", sans considération de priorité. Miroir de `entry?.isArchi === true`
    /// (`HistorySyncService.hasArchiEnemy`, `dungeon-run-grouping.util.ts`) : nécessaire pour le
    /// créneau "archimonstre pré-boss" du regroupement de donjon (`dungeon_run.rs`), où un combat
    /// SANS boss doit être reconnu comme contenant un archimonstre même si celui-ci était,
    /// ailleurs, aussi classé boss.
    pub fn find_monster_is_archi(&self, name: &str, catalog_id: Option<i64>) -> bool {
        self.find_monster_entry(name, catalog_id)
            .is_some_and(|entry| entry.is_archi)
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
    fn resout_lid_dun_objet_par_nom_seul() {
        let index = CatalogIndex::from_compact_json(&sample());
        assert_eq!(index.find_item_id("larme d'ogrest"), Some(24029));
        assert_eq!(index.find_item_id("Introuvable"), None);
    }

    #[test]
    fn resout_lid_dun_monstre_par_nom_seul() {
        let index = CatalogIndex::from_compact_json(&sample());
        assert_eq!(index.find_monster_id("el pochito"), Some(24875));
        assert_eq!(index.find_monster_id("Introuvable"), None);
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
    fn drapeau_archi_brut_reste_disponible_meme_derriere_un_classement_boss() {
        let index = CatalogIndex::from_compact_json(&sample());
        assert!(index.find_monster_is_archi("peu importe", Some(501)));
        assert!(!index.find_monster_is_archi("peu importe", Some(502))); // dominant, pas archi
        assert!(!index.find_monster_is_archi("peu importe", Some(24875))); // boss, pas archi
        assert!(!index.find_monster_is_archi("Introuvable", None));
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
        assert_eq!(
            IconRef::for_rarity(WakfuRarity::Mythical).image_url(),
            "https://vertylo.github.io/wakassets/rarities/3.png"
        );
        assert_eq!(
            IconRef::for_item_category(WakfuItemCategory::Resources).image_url(),
            "https://vertylo.github.io/wakassets/itemTypes/226.png"
        );
    }

    #[test]
    fn resout_la_categorie_dun_objet() {
        let index = CatalogIndex::from_compact_json(&sample());
        // categorySortOrder=1 dans le jeu d'essai.
        assert_eq!(
            index.find_item_category("Larme d'Ogrest", None),
            WakfuItemCategory::Resources
        );
        // categorySortOrder=6, et résolu par id plutôt que par nom.
        assert_eq!(
            index.find_item_category("peu importe", Some(9001)),
            WakfuItemCategory::Craft
        );
    }

    #[test]
    fn objet_non_resolu_retombe_sur_divers() {
        let index = CatalogIndex::from_compact_json(&sample());
        assert_eq!(
            index.find_item_category("Inconnu au bataillon", None),
            WakfuItemCategory::Misc
        );
        let vide = CatalogIndex::from_compact_json(&serde_json::json!({}));
        assert_eq!(
            vide.find_item_category("quoi que ce soit", Some(1)),
            WakfuItemCategory::Misc
        );
    }

    /// L'ordre de tri encode la catégorie dans l'index compact ; le numéro d'icône vient de
    /// l'encyclopédie. Les confondre rangerait chaque objet sous le mauvais pictogramme — et rien
    /// ne le signalerait, les deux étant de simples entiers.
    #[test]
    fn ordre_de_tri_et_numero_dicone_sont_deux_tables_distinctes() {
        for (ordre, categorie) in [
            (0, WakfuItemCategory::Equipment),
            (1, WakfuItemCategory::Resources),
            (2, WakfuItemCategory::Sublimations),
            (3, WakfuItemCategory::Harvests),
            (4, WakfuItemCategory::HavenBag),
            (5, WakfuItemCategory::Cosmetics),
            (6, WakfuItemCategory::Craft),
            (7, WakfuItemCategory::Misc),
        ] {
            assert_eq!(WakfuItemCategory::from_sort_order(ordre), categorie);
            assert_ne!(
                i32::try_from(ordre).unwrap(),
                categorie.icon_number(),
                "l'ordre de tri {ordre} ne doit jamais servir de numéro d'icône"
            );
        }
        // Un ordre inconnu range dans « Divers » plutôt que de faire disparaître l'objet.
        assert_eq!(
            WakfuItemCategory::from_sort_order(99),
            WakfuItemCategory::Misc
        );
    }

    /// Les huit numéros de catégorie, la table entière — même raison que pour les gemmes : ce sont
    /// des ids recopiés de l'encyclopédie, une ligne fausse affiche le mauvais pictogramme sans
    /// rien casser.
    #[test]
    fn chaque_categorie_a_son_numero_dicone() {
        for (categorie, attendu) in [
            (WakfuItemCategory::Equipment, 109),
            (WakfuItemCategory::Resources, 226),
            (WakfuItemCategory::Sublimations, 602),
            (WakfuItemCategory::Harvests, 237),
            (WakfuItemCategory::HavenBag, 295),
            (WakfuItemCategory::Cosmetics, 525),
            (WakfuItemCategory::Craft, 761),
            (WakfuItemCategory::Misc, 385),
        ] {
            assert_eq!(categorie.icon_number(), attendu, "catégorie {categorie:?}");
        }
    }

    /// « Tout » et « Monstres » vivent dans le même dossier que les catégories sans en être :
    /// leurs numéros ne doivent donc jamais entrer en collision avec l'un des huit.
    #[test]
    fn tout_et_monstres_ne_sont_pas_des_categories() {
        let hors_categorie = ["-1", "282"];
        assert_eq!(
            IconRef::for_all_categories().image_url(),
            "https://vertylo.github.io/wakassets/itemTypes/-1.png"
        );
        assert_eq!(
            IconRef::for_monster_category().image_url(),
            "https://vertylo.github.io/wakassets/itemTypes/282.png"
        );
        for categorie in [
            WakfuItemCategory::Equipment,
            WakfuItemCategory::Resources,
            WakfuItemCategory::Sublimations,
            WakfuItemCategory::Harvests,
            WakfuItemCategory::HavenBag,
            WakfuItemCategory::Cosmetics,
            WakfuItemCategory::Craft,
            WakfuItemCategory::Misc,
        ] {
            let numero = categorie.icon_number().to_string();
            assert!(
                !hors_categorie.contains(&numero.as_str()),
                "{categorie:?} réutilise un numéro réservé"
            );
        }
    }

    /// Les huit gemmes, une par rareté — la table entière plutôt qu'un échantillon : c'est une
    /// correspondance recopiée d'un autre dépôt, une seule ligne fausse suffit à afficher la
    /// mauvaise gemme.
    #[test]
    fn chaque_rarete_a_son_numero_de_gemme() {
        for (rarity, attendu) in [
            (WakfuRarity::Old, 0),
            (WakfuRarity::Common, 1),
            (WakfuRarity::Rare, 2),
            (WakfuRarity::Mythical, 3),
            (WakfuRarity::Legendary, 4),
            (WakfuRarity::Relic, 5),
            (WakfuRarity::Memory, 6),
            (WakfuRarity::Epic, 7),
        ] {
            assert_eq!(rarity_icon_number(rarity), attendu, "rareté {rarity:?}");
        }
    }

    /// **Le piège**, verrouillé : ordre de tri et numéro de gemme coïncident jusqu'à
    /// `Legendary` puis se croisent. Écrire l'un à la place de l'autre ne casse rien — ça affiche
    /// simplement la gemme d'une autre rareté pour ces trois-là.
    #[test]
    fn les_deux_tables_de_rarete_divergent_bien() {
        for (ordre, rarity) in [
            (5, WakfuRarity::Memory),
            (6, WakfuRarity::Epic),
            (7, WakfuRarity::Relic),
        ] {
            assert_eq!(rarity_from_sort_order(ordre), rarity);
            assert_ne!(
                u8::try_from(ordre).unwrap(),
                rarity_icon_number(rarity),
                "l'ordre de tri {ordre} et le numéro de gemme de {rarity:?} ne doivent PAS \
                 coïncider — si cette table change côté web, les deux doivent être remises à jour \
                 ensemble"
            );
        }
    }
}
