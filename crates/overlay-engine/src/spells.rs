//! Référentiels des sorts — `assets/spells.json` (sorts de classe) et `assets/monster-spells.json`
//! (sorts de monstres) à la racine du dépôt, embarqués dans le binaire (`include_str!`, comme les
//! polices et les assets du design system côté `overlay-ui`), indexés une seule fois chacun
//! (`SpellIndex::embedded`, `MonsterSpellIndex::embedded`) pour résoudre l'ICÔNE d'un sort lancé
//! (`session::SpellCastRecord`) dans le bloc « ligne de sorts » du panneau Combat
//! (`overlay-ui::panels::combat_spell_block`). L'UI n'interroge aucun des deux directement : elle
//! appelle [`resolve_cast`], qui choisit le référentiel d'après le camp du lanceur.
//!
//! **Les deux fichiers sont maintenus à la main par l'utilisateur.**
//!
//! - `spells.json` : 18 classes plus une pseudo-classe `common` de `breedId` −2 pour les sorts
//!   communs à tout le monde (Maîtrise d'Armes, Charme de Masse, Os à Moelle) ; une entrée par sort
//!   avec ses noms localisés et l'URL de son image sur le CDN `wakassets`. Depuis la mise à jour du
//!   12 sept. 2026 il couvre aussi les mécaniques de classe (Proie, Karcham/Chamrak, Bond du
//!   félin…), fusionnées dans `spells` avec les sorts de panneau.
//! - `monster-spells.json` (ajouté le 13 sept. 2026) : un tableau `{ breedId, spells }` par
//!   monstre, où `breedId` est **l'identifiant que la ligne de jointure du log donne à chaque
//!   monstre** (`[_FL_] … Grokoko breed : 4728 […] isControlledByAI=true` ↔ `breedId: 4728`) —
//!   c'est ce qui rend la résolution possible sans catalogue. Le même nom de sort existe chez
//!   plusieurs monstres (598 noms partagés sur 2 381 entrées au fichier du 13 sept.), et 69 d'entre
//!   eux avec des images différentes selon le monstre (« Coup d'Koko » : `spells/3.png` chez le
//!   Grokoko, `spells/4.png` chez le Kokoko) — d'où la clé par `breed`, jamais par le nom seul
//!   quand le `breed` est connu. Le champ `passive` du fichier est ignoré.
//!
//! **Les deux index sont construits au démarrage de l'overlay** ([`preload_embedded`], appelé par
//! `overlay-ui` avant la première frame), jamais à la demande : deux fichiers de quelques centaines
//! de Ko, très loin du budget mémoire (300 Mo, §11 du plan), et aucun à-coup au premier sort
//! affiché en combat. `embedded()` reste paresseux par construction (`OnceLock`) pour les tests et
//! tout appelant qui n'aurait pas fait le préchargement.
//!
//! Un sort absent du référentiel n'est toujours PAS une erreur ici : `find` renvoie `None` et l'UI
//! affiche un pavé « ? » (voir `combat_spell_block`) ; une entrée sans `picture` (`null`, ex.
//! Engrènement côté Sadida) est connue mais sans icône (`SpellEntry::icon` à `None`). Ce module ne
//! doit donc jamais paniquer sur un nom inconnu ou une image absente, seulement sur un fichier
//! structurellement invalide (bug de build, comme un asset corrompu — voir `embedded`).
//!
//! **Clé de résolution : nom normalisé + propriétaire du sort** ([`SpellTable::find`]). Le nom
//! seul est ambigu — « Rafale » et « Poursuite » existent chez deux classes chacun, avec des icônes
//! différentes, « Coup d'Koko » chez quatre monstres. Le propriétaire est la classe du lanceur
//! (`FighterDamage::class_name`, roster sinon `breed` du combat) côté allié, le `breed` brut de la
//! jointure (`FighterDamage::breed`) côté ennemi. Sans propriétaire connu (roster périmé, combat
//! restauré d'une version antérieure au champ `breed`, sort commun à toutes les classes —
//! pseudo-classe `common`), ou si le propriétaire ne possède pas ce sort, repli sur la PREMIÈRE
//! entrée portant ce nom, dans l'ordre du fichier — mieux qu'un « ? » pour un sort dont l'icône est
//! de toute façon la même partout, et une icône *d'un autre monstre homonyme* dans les 69 cas
//! ambigus, seulement quand le `breed` manque. Le repli ne traverse JAMAIS les deux fichiers : un
//! monstre ne reçoit pas l'icône d'un sort de classe qui porterait le même nom, ni l'inverse.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::catalog::{IconKind, IconRef};
use crate::class_breed::class_for_breed;
use crate::roster::normalize_wakfu_name;
use crate::session::FighterDamage;

/// Copies embarquées des référentiels — chemins relatifs à CE fichier source (même convention que
/// `overlay-ui/src/design/assets.rs` pour `assets/design-system/`).
const SPELLS_JSON: &str = include_str!("../../../assets/spells.json");
const MONSTER_SPELLS_JSON: &str = include_str!("../../../assets/monster-spells.json");

/// Une entrée d'un référentiel, réduite à ce que l'affichage consomme. `K` est le type du
/// propriétaire : la clé de classe (`&'static str`) pour `spells.json`, le `breed` du monstre
/// (`i64`) pour `monster-spells.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellEntry<K> {
    /// Identifiant Ankama du sort (`id` du fichier) — informatif, non utilisé pour l'icône.
    pub id: i64,
    /// Nom français tel qu'écrit dans le fichier (c'est aussi ce que le log affiche) — le texte de
    /// l'infobulle, préféré au nom brut du log qui est identique aux accents près.
    pub name: String,
    /// Propriétaire du sort — classe (clé interne, `"ouginak"`, voir `class_breed::class_for_
    /// breed`, ou [`COMMON_CLASS`] pour un sort commun à toutes les classes — dérivée de `breedId`,
    /// JAMAIS de `breedName` : le fichier y écrit « sacrieur », « roublard », « steamer », là où le
    /// reste du dépôt dit `sacrier`/`rogue`/`foggernaut`) ou `breed` du monstre.
    pub owner: K,
    /// Icône distante (`wakassets/spells/{n}.png`) — même circuit de téléchargement/cache que les
    /// icônes de monstres (`overlay-ui::remote_icons`), voir `IconKind::Spell`. `None` pour une
    /// entrée dont le fichier n'a pas d'image (`picture: null`) : sort connu, tuile sans icône.
    pub icon: Option<IconRef>,
}

/// `breedId` de la pseudo-classe des sorts communs à toutes les classes dans `spells.json`.
const COMMON_BREED_ID: i64 = -2;
/// Nom de classe interne de cette pseudo-classe — jamais une classe d'un combattant, donc jamais
/// trouvée par la clé nom + classe : ses sorts se résolvent par le repli sur le nom seul.
pub const COMMON_CLASS: &str = "common";

/// Une classe ou un monstre du fichier — les deux référentiels ont la même forme, `spells.json`
/// porte en plus un `breedName` ignoré (voir `SpellEntry::owner`).
#[derive(Deserialize)]
struct RawOwner {
    #[serde(rename = "breedId")]
    breed_id: i64,
    spells: Vec<RawSpell>,
}

#[derive(Deserialize)]
struct RawSpell {
    id: i64,
    fr: String,
    #[serde(default)]
    picture: Option<String>,
}

/// Index en lookup O(1) d'un référentiel — voir la doc de module pour la clé de résolution. Une
/// seule structure pour les deux fichiers ([`SpellIndex`], [`MonsterSpellIndex`]) : seuls le type
/// du propriétaire et la validation du `breedId` au chargement diffèrent.
#[derive(Debug)]
pub struct SpellTable<K> {
    entries: Vec<SpellEntry<K>>,
    by_name_and_owner: HashMap<(String, K), usize>,
    /// Première entrée portant ce nom, tous propriétaires confondus (repli, voir `find`).
    by_name: HashMap<String, usize>,
}

/// Référentiel des sorts de classe (`assets/spells.json`), propriétaire = clé de classe.
pub type SpellIndex = SpellTable<&'static str>;
/// Référentiel des sorts de monstres (`assets/monster-spells.json`), propriétaire = `breed`.
pub type MonsterSpellIndex = SpellTable<i64>;

impl<K> Default for SpellTable<K> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            by_name_and_owner: HashMap::new(),
            by_name: HashMap::new(),
        }
    }
}

impl<K: Hash + Eq + Copy> SpellTable<K> {
    /// Construit l'index depuis un fichier déjà désérialisé : `owner_of` traduit le `breedId` de
    /// chaque groupe en propriétaire, ou `None` pour l'ignorer avec un avertissement plutôt que de
    /// faire échouer tout le chargement.
    fn from_raw(owners: Vec<RawOwner>, owner_of: impl Fn(i64) -> Option<K>) -> Self {
        let mut table = SpellTable::default();
        for owner in owners {
            let Some(key) = owner_of(owner.breed_id) else {
                tracing::warn!(
                    breed_id = owner.breed_id,
                    "propriétaire inconnu dans un référentiel de sorts, entrées ignorées"
                );
                continue;
            };
            for spell in owner.spells {
                table.insert(SpellEntry {
                    id: spell.id,
                    name: spell.fr,
                    owner: key,
                    icon: spell.picture.as_deref().map(|picture| IconRef {
                        kind: IconKind::Spell,
                        gfx_id: picture_gfx_id(picture),
                    }),
                });
            }
        }
        table
    }

    fn insert(&mut self, entry: SpellEntry<K>) {
        let idx = self.entries.len();
        let key = normalize_wakfu_name(&entry.name);
        self.by_name_and_owner
            .entry((key.clone(), entry.owner))
            .or_insert(idx);
        self.by_name.entry(key).or_insert(idx);
        self.entries.push(entry);
    }

    /// Résout un sort par son nom (brut du log ou du fichier, normalisation `normalize_wakfu_name`
    /// des deux côtés) et son propriétaire — voir la doc de module pour l'ordre des replis. `None`
    /// si aucune entrée ne porte ce nom, quel que soit le propriétaire.
    pub fn find(&self, spell_name: &str, owner: Option<K>) -> Option<&SpellEntry<K>> {
        self.entries.get(self.entry_index(spell_name, owner)?)
    }

    /// Position dans `entries` de l'entrée que `find` renverrait — séparée de `find` pour
    /// `resolve_cast` : une clé de classe empruntée (`&'a str`, celle de `FighterDamage`) force
    /// par covariance la table `SpellTable<&'static str>` à se lire comme `SpellTable<&'a str>`
    /// le temps de l'appel ; un index (sans durée de vie) laisse ensuite lire l'entrée dans la
    /// table statique, et rendre un `ResolvedSpell<'static>`.
    fn entry_index(&self, spell_name: &str, owner: Option<K>) -> Option<usize> {
        let key = normalize_wakfu_name(spell_name);
        owner
            .and_then(|owner| self.by_name_and_owner.get(&(key.clone(), owner)))
            .or_else(|| self.by_name.get(&key))
            .copied()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl SpellIndex {
    /// Index du référentiel de classe embarqué, construit au premier appel puis partagé — le
    /// fichier est petit (quelques centaines d'entrées), le coût est négligeable et payé une fois.
    /// Panique si le JSON embarqué est structurellement invalide : bug de build, pas un cas
    /// runtime (le test `le_referentiel_embarque_est_valide` le détecte avant).
    pub fn embedded() -> &'static SpellIndex {
        static INDEX: OnceLock<SpellIndex> = OnceLock::new();
        INDEX.get_or_init(|| {
            SpellIndex::from_json_str(SPELLS_JSON)
                .expect("assets/spells.json embarqué invalide — référentiel corrompu au build")
        })
    }

    /// Construit l'index depuis le contenu JSON du référentiel (format d'`assets/spells.json` :
    /// tableau de classes `{ breedName, breedId, spells: [{ id, fr, en, es, pt, picture }] }`, les
    /// champs non listés dans `RawOwner`/`RawSpell` sont ignorés). Une classe dont le `breedId` ne
    /// correspond ni à une classe jouable (`class_for_breed`) ni à la pseudo-classe commune
    /// (`COMMON_BREED_ID`) est ignorée avec un avertissement.
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        let classes: Vec<RawOwner> = serde_json::from_str(json)?;
        Ok(SpellTable::from_raw(classes, |breed_id| {
            if breed_id == COMMON_BREED_ID {
                Some(COMMON_CLASS)
            } else {
                class_for_breed(breed_id)
            }
        }))
    }
}

impl MonsterSpellIndex {
    /// Index du référentiel de monstres embarqué — même construction que [`SpellIndex::embedded`]
    /// (580 Ko, ~2 400 entrées au 13 sept. 2026), déclenchée au démarrage par [`preload_embedded`].
    pub fn embedded() -> &'static MonsterSpellIndex {
        static INDEX: OnceLock<MonsterSpellIndex> = OnceLock::new();
        INDEX.get_or_init(|| {
            MonsterSpellIndex::from_json_str(MONSTER_SPELLS_JSON).expect(
                "assets/monster-spells.json embarqué invalide — référentiel corrompu au build",
            )
        })
    }

    /// Construit l'index depuis le contenu JSON du référentiel (format d'`assets/monster-spells.
    /// json` : tableau de monstres `{ breedId, spells: [{ id, fr, en, es, pt, picture, passive? }]
    /// }`). Tout `breedId` est accepté tel quel : c'est un identifiant de monstre, pas une classe.
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        let monsters: Vec<RawOwner> = serde_json::from_str(json)?;
        Ok(SpellTable::from_raw(monsters, Some))
    }
}

/// Construit les deux index embarqués maintenant plutôt qu'au premier sort affiché — à appeler au
/// démarrage de l'overlay (voir la doc de module). Idempotent, sans effet si déjà fait. Renvoie le
/// nombre d'entrées (classes, monstres) pour le journal de démarrage.
pub fn preload_embedded() -> (usize, usize) {
    (
        SpellIndex::embedded().len(),
        MonsterSpellIndex::embedded().len(),
    )
}

/// Ce que l'UI consomme pour une tuile du bloc « ligne de sorts » — voir [`resolve_cast`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedSpell<'a> {
    /// Nom du référentiel (accents du fichier), texte de l'infobulle.
    pub name: &'a str,
    /// `None` : entrée connue mais sans image dans le fichier (tuile sombre, sans « ? »).
    pub icon: Option<&'a IconRef>,
}

/// Résout un sort lancé par `fighter` dans le référentiel de son camp : `spells.json` par la
/// classe du lanceur pour un allié, `monster-spells.json` par son `breed` pour un ennemi — voir la
/// doc de module pour les replis. `None` : sort absent du référentiel de ce camp (pavé « ? »).
/// C'est le SEUL point d'entrée de l'UI : `combat_spell_block` ne sait pas quel fichier répond.
pub fn resolve_cast(fighter: &FighterDamage, spell: &str) -> Option<ResolvedSpell<'static>> {
    let (name, icon) = if fighter.is_ally {
        let index = SpellIndex::embedded();
        let idx = index.entry_index(spell, fighter.class_name.as_deref())?;
        let entry = &index.entries[idx];
        (entry.name.as_str(), entry.icon.as_ref())
    } else {
        let entry = MonsterSpellIndex::embedded().find(spell, fighter.breed)?;
        (entry.name.as_str(), entry.icon.as_ref())
    };
    Some(ResolvedSpell { name, icon })
}

/// Chemin du référentiel qui répond pour ce camp — pour le message qui dit à l'utilisateur quel
/// fichier compléter quand un sort est inconnu (`combat_spell_block::warn_unknown_spell_once`).
pub fn referential_path(is_ally: bool) -> &'static str {
    if is_ally {
        "assets/spells.json"
    } else {
        "assets/monster-spells.json"
    }
}

/// Numéro d'image à partir de l'URL `picture` du fichier (`…/wakassets/spells/2282.png` → `2282`)
/// — c'est ce numéro, et non l'URL complète, que `IconRef` transporte : `IconRef::image_url`
/// reconstruit l'URL `wakassets` (et `overlay_sync::icon_cache` nomme son fichier de cache
/// d'après lui). Les référentiels doivent donc pointer sur `wakassets/spells/` — vérifié par le
/// test `toutes_les_images_des_referentiels_sont_sur_wakassets_spells` pour qu'une mise à jour d'un
/// fichier vers un autre hébergement ne casse pas silencieusement l'affichage.
fn picture_gfx_id(picture: &str) -> String {
    let file = picture.rsplit('/').next().unwrap_or(picture);
    file.strip_suffix(".png").unwrap_or(file).to_string()
}

/// Préfixe d'URL attendu pour chaque `picture` des référentiels — voir `picture_gfx_id`.
pub const WAKASSETS_SPELLS_URL_PREFIX: &str = "https://vertylo.github.io/wakassets/spells/";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roster::Gender;

    fn fighter(
        name: &str,
        is_ally: bool,
        class_name: Option<&str>,
        breed: Option<i64>,
    ) -> FighterDamage {
        FighterDamage {
            name: name.to_string(),
            is_ally,
            total_damage: 0,
            total_heal: 0,
            class_name: class_name.map(str::to_string),
            gender: Gender::M,
            xp_gained: 0,
            spells: Default::default(),
            is_ko: false,
            last_turn_casts: Vec::new(),
            last_turn: 0,
            breed,
        }
    }

    #[test]
    fn le_referentiel_embarque_est_valide() {
        let index = SpellIndex::embedded();
        assert!(index.len() >= 400, "{} entrées seulement", index.len());
    }

    #[test]
    fn le_referentiel_monstre_embarque_est_valide() {
        let index = MonsterSpellIndex::embedded();
        assert!(index.len() >= 2000, "{} entrées seulement", index.len());
    }

    /// Les sorts communs (pseudo-classe `common`, `breedId` −2) se résolvent pour n'importe quelle
    /// classe de lanceur, par le repli sur le nom seul.
    #[test]
    fn un_sort_commun_se_resout_pour_toute_classe() {
        let index = SpellIndex::embedded();
        for class in [Some("iop"), Some("sadida"), None] {
            let entry = index.find("Charme de Masse", class).expect("sort commun");
            assert_eq!(entry.owner, COMMON_CLASS);
            assert_eq!(entry.id, 5623);
        }
    }

    /// Les mécaniques de classe ajoutées au référentiel le 12 sept. (auparavant en pavé « ? »).
    #[test]
    fn les_mecaniques_de_classe_sont_presentes() {
        let index = SpellIndex::embedded();
        for (spell, class) in [
            ("Proie", "ouginak"),
            ("Karcham", "pandawa"),
            ("Chamrak", "pandawa"),
            ("Bond du félin", "ecaflip"),
        ] {
            let entry = index
                .find(spell, Some(class))
                .unwrap_or_else(|| panic!("{spell}"));
            assert_eq!(entry.owner, class);
            assert!(entry.icon.is_some(), "{spell} sans image");
        }
    }

    /// Une entrée sans `picture` reste connue (nom, classe) mais sans icône — jamais un échec de
    /// chargement de tout le référentiel.
    #[test]
    fn une_entree_sans_image_est_connue_sans_icone() {
        let index = SpellIndex::from_json_str(
            r#"[{"breedName":"iop","breedId":8,"spells":[{"id":2,"fr":"B","picture":null},{"id":3,"fr":"C"}]}]"#,
        )
        .unwrap();
        assert_eq!(index.len(), 2);
        assert!(index.find("B", Some("iop")).unwrap().icon.is_none());
        assert!(index.find("C", Some("iop")).unwrap().icon.is_none());
    }

    #[test]
    fn toutes_les_images_des_referentiels_sont_sur_wakassets_spells() {
        for (file, json) in [
            ("spells.json", SPELLS_JSON),
            ("monster-spells.json", MONSTER_SPELLS_JSON),
        ] {
            let owners: Vec<RawOwner> = serde_json::from_str(json).unwrap();
            for owner in owners {
                for spell in owner.spells {
                    let Some(picture) = spell.picture else {
                        continue; // entrée sans image, connue sans icône (voir la doc de module)
                    };
                    assert!(
                        picture.starts_with(WAKASSETS_SPELLS_URL_PREFIX)
                            && picture.ends_with(".png"),
                        "{file} : {} ({}) : image hors de wakassets/spells — `IconRef::image_url` \
                         ne saurait pas la reconstruire : {}",
                        spell.fr,
                        spell.id,
                        picture
                    );
                }
            }
        }
    }

    #[test]
    fn resout_par_nom_normalise_et_classe() {
        let index = SpellIndex::embedded();
        let entry = index
            .find("hachure", Some("ouginak"))
            .expect("Hachure (Ouginak)");
        assert_eq!(entry.name, "Hachure");
        assert_eq!(entry.owner, "ouginak");
        assert_eq!(
            entry.icon.as_ref().unwrap().image_url(),
            "https://vertylo.github.io/wakassets/spells/6262.png"
        );
    }

    /// « Rafale » existe chez deux classes : la classe du lanceur départage, le repli sans classe
    /// prend la première du fichier plutôt que rien.
    #[test]
    fn un_nom_ambigu_est_departage_par_la_classe() {
        let index = SpellIndex::embedded();
        let classes: Vec<&str> = index
            .entries
            .iter()
            .filter(|e| normalize_wakfu_name(&e.name) == "rafale")
            .map(|e| e.owner)
            .collect();
        assert!(
            classes.len() >= 2,
            "le doublon a disparu du fichier : {classes:?}"
        );
        for class in &classes {
            assert_eq!(index.find("Rafale", Some(class)).unwrap().owner, *class);
        }
        assert_eq!(
            index.find("Rafale", None).unwrap().owner,
            classes[0],
            "sans classe : première entrée du fichier"
        );
    }

    #[test]
    fn une_classe_sans_ce_sort_retombe_sur_le_nom_seul() {
        let index = SpellIndex::embedded();
        // Hachure est Ouginak ; demandé pour un Iop (roster périmé), on préfère l'icône Ouginak
        // à rien du tout.
        assert_eq!(index.find("Hachure", Some("iop")).unwrap().owner, "ouginak");
    }

    #[test]
    fn un_sort_absent_du_referentiel_donne_none() {
        let index = SpellIndex::embedded();
        assert!(index.find("Sort qui n'existe pas", Some("iop")).is_none());
        assert!(index.find("Sort qui n'existe pas", None).is_none());
    }

    #[test]
    fn le_nom_de_classe_vient_du_breed_id_pas_du_breed_name() {
        let index = SpellIndex::embedded();
        // « sacrieur » dans le fichier, `sacrier` partout ailleurs dans le dépôt.
        assert!(index.entries.iter().any(|e| e.owner == "sacrier"));
        assert!(!index.entries.iter().any(|e| e.owner == "sacrieur"));
    }

    #[test]
    fn picture_gfx_id_extrait_le_numero() {
        assert_eq!(
            picture_gfx_id("https://vertylo.github.io/wakassets/spells/2282.png"),
            "2282"
        );
        assert_eq!(picture_gfx_id("2282.png"), "2282");
        assert_eq!(picture_gfx_id("2282"), "2282");
    }

    #[test]
    fn une_classe_inconnue_est_ignoree_sans_echec() {
        let index = SpellIndex::from_json_str(
            r#"[{"breedName":"x","breedId":99,"spells":[{"id":1,"fr":"A","picture":"a.png"}]},
                {"breedName":"iop","breedId":8,"spells":[{"id":2,"fr":"B","picture":"b.png"}]}]"#,
        )
        .unwrap();
        assert_eq!(index.len(), 1);
        assert_eq!(index.find("B", Some("iop")).unwrap().id, 2);
    }

    /// « Coup d'Koko » existe chez quatre monstres avec deux images : c'est le `breed` de la
    /// jointure (4728 = Grokoko, 4729 = Kokoko dans le log de parité) qui départage.
    #[test]
    fn coup_d_koko_est_departage_par_le_breed() {
        let index = MonsterSpellIndex::embedded();
        let grokoko = index.find("Coup d'Koko", Some(4728)).expect("Grokoko");
        let kokoko = index.find("coup d'koko", Some(4729)).expect("Kokoko");
        assert_eq!(grokoko.owner, 4728);
        assert_eq!(kokoko.owner, 4729);
        assert_eq!(grokoko.icon.as_ref().unwrap().gfx_id, "3");
        assert_eq!(kokoko.icon.as_ref().unwrap().gfx_id, "4");
    }

    /// Sans `breed` (combat restauré d'avant le champ) ou avec un `breed` qui ne possède pas ce
    /// sort : première entrée du fichier portant ce nom, jamais `None` tant que le nom existe.
    #[test]
    fn un_monstre_sans_breed_connu_retombe_sur_le_nom_seul() {
        let index = MonsterSpellIndex::embedded();
        let first = index.find("Coup d'Koko", None).expect("nom connu");
        assert_eq!(index.find("Coup d'Koko", Some(-1)).unwrap(), first);
        assert!(index
            .find("Sort de monstre inexistant", Some(4728))
            .is_none());
    }

    /// `resolve_cast` choisit le fichier d'après le camp : un allié ne trouve jamais un sort de
    /// monstre, un monstre jamais un sort de classe — même si le nom existe dans l'autre fichier.
    #[test]
    fn resolve_cast_ne_traverse_jamais_les_deux_referentiels() {
        let ally = fighter("Erz-Wouaf", true, Some("ouginak"), Some(15));
        let enemy = fighter("Grokoko", false, None, Some(4728));

        let hachure = resolve_cast(&ally, "Hachure").expect("sort de classe");
        assert_eq!(hachure.name, "Hachure");
        assert_eq!(hachure.icon.unwrap().gfx_id, "6262");
        assert!(resolve_cast(&enemy, "Hachure").is_none());

        let koko = resolve_cast(&enemy, "Coup d'Koko").expect("sort de monstre");
        assert_eq!(koko.name, "Coup d'Koko");
        assert_eq!(koko.icon.unwrap().gfx_id, "3");
        assert!(resolve_cast(&ally, "Coup d'Koko").is_none());
    }

    #[test]
    fn referential_path_nomme_le_fichier_du_camp() {
        assert_eq!(referential_path(true), "assets/spells.json");
        assert_eq!(referential_path(false), "assets/monster-spells.json");
    }
}
