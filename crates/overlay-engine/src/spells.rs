//! Référentiel des sorts de classe — `assets/spells.json` à la racine du dépôt, embarqué dans le
//! binaire (`include_str!`, comme les polices et les assets du design system côté `overlay-ui`),
//! indexé une seule fois (`SpellIndex::embedded`) pour résoudre l'ICÔNE d'un sort lancé
//! (`session::SpellCastRecord`) dans le bloc « ligne de sorts » du panneau Combat
//! (`overlay-ui::panels::combat_spell_block`).
//!
//! **Le fichier est maintenu à la main par l'utilisateur** (18 classes plus une pseudo-classe
//! `common` de `breedId` −2 pour les sorts communs à tout le monde — Maîtrise d'Armes, Charme de
//! Masse, Os à Moelle ; une entrée par sort avec ses noms localisés et l'URL de son image sur le
//! CDN `wakassets`). Depuis la mise à jour du 12 sept. 2026 il couvre aussi les mécaniques de
//! classe (Proie, Karcham/Chamrak, Bond du félin…), fusionnées dans `spells` avec les sorts de
//! panneau. Un sort absent du référentiel n'est toujours PAS une erreur ici : `find` renvoie
//! `None` et l'UI affiche un pavé « ? » (voir `combat_spell_block`) ; une entrée sans `picture`
//! (`null`, ex. Engrènement côté Sadida) est connue mais sans icône (`SpellEntry::icon` à `None`).
//! Ce module ne doit donc jamais paniquer sur un nom inconnu ou une image absente, seulement sur
//! un fichier structurellement invalide (bug de build, comme un asset corrompu — voir
//! `SpellIndex::embedded`).
//!
//! **Clé de résolution : nom normalisé + classe du lanceur.** Le nom seul est ambigu — « Rafale »
//! et « Poursuite » existent chez deux classes chacun, avec des icônes différentes. La classe du
//! lanceur vient de `FighterDamage::class_name` (roster, sinon `breed` du combat) ; sans classe
//! connue, ou si la classe ne possède pas ce sort (roster périmé, sort commun à tout le monde —
//! pseudo-classe `common`), repli sur la PREMIÈRE entrée portant ce nom, dans l'ordre du fichier —
//! mieux qu'un « ? » pour un sort dont l'icône est de toute façon la même partout.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::catalog::{IconKind, IconRef};
use crate::class_breed::class_for_breed;
use crate::roster::normalize_wakfu_name;

/// Copie embarquée d'`assets/spells.json` — chemin relatif à CE fichier source (même convention
/// que `overlay-ui/src/design/assets.rs` pour `assets/design-system/`).
const SPELLS_JSON: &str = include_str!("../../../assets/spells.json");

/// Une entrée du référentiel, réduite à ce que l'affichage consomme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellEntry {
    /// Identifiant Ankama du sort (`id` du fichier) — informatif, non utilisé pour l'icône.
    pub id: i64,
    /// Nom français tel qu'écrit dans le fichier (c'est aussi ce que le log affiche) — le texte de
    /// l'infobulle, préféré au nom brut du log qui est identique aux accents près.
    pub name: String,
    /// Classe propriétaire, clé interne (`"ouginak"`, voir `class_breed::class_for_breed`), ou
    /// [`COMMON_CLASS`] pour un sort commun à toutes les classes — dérivée de `breedId`, JAMAIS de
    /// `breedName` (le fichier y écrit « sacrieur », « roublard », « steamer », là où le reste du
    /// dépôt dit `sacrier`/`rogue`/`foggernaut`).
    pub class_name: &'static str,
    /// Icône distante (`wakassets/spells/{n}.png`) — même circuit de téléchargement/cache que les
    /// icônes de monstres (`overlay-ui::remote_icons`), voir `IconKind::Spell`. `None` pour une
    /// entrée dont le fichier n'a pas d'image (`picture: null`) : sort connu, tuile sans icône.
    pub icon: Option<IconRef>,
}

/// `breedId` de la pseudo-classe des sorts communs à toutes les classes dans le fichier.
const COMMON_BREED_ID: i64 = -2;
/// Nom de classe interne de cette pseudo-classe — jamais une classe d'un combattant, donc jamais
/// trouvée par `by_name_and_class` : ses sorts se résolvent par le repli sur le nom seul.
pub const COMMON_CLASS: &str = "common";

#[derive(Deserialize)]
struct RawClass {
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

/// Index en lookup O(1) — voir la doc de module pour la clé de résolution.
#[derive(Debug, Default)]
pub struct SpellIndex {
    entries: Vec<SpellEntry>,
    by_name_and_class: HashMap<(String, &'static str), usize>,
    /// Première entrée portant ce nom, toutes classes confondues (repli, voir `find`).
    by_name: HashMap<String, usize>,
}

impl SpellIndex {
    /// Index du référentiel embarqué, construit au premier appel puis partagé — le fichier est
    /// petit (quelques centaines d'entrées), le coût est négligeable et payé une fois. Panique si
    /// le JSON embarqué est structurellement invalide : bug de build, pas un cas runtime (le test
    /// `le_referentiel_embarque_est_valide` le détecte avant).
    pub fn embedded() -> &'static SpellIndex {
        static INDEX: OnceLock<SpellIndex> = OnceLock::new();
        INDEX.get_or_init(|| {
            SpellIndex::from_json_str(SPELLS_JSON)
                .expect("assets/spells.json embarqué invalide — référentiel corrompu au build")
        })
    }

    /// Construit l'index depuis le contenu JSON du référentiel (format d'`assets/spells.json` :
    /// tableau de classes `{ breedName, breedId, spells: [{ id, fr, en, es, pt, picture }] }`, les
    /// champs non listés dans `RawClass`/`RawSpell` sont ignorés). Une classe dont le `breedId` ne
    /// correspond ni à une classe jouable (`class_for_breed`) ni à la pseudo-classe commune
    /// (`COMMON_BREED_ID`) est ignorée avec un avertissement plutôt que de faire échouer tout le
    /// chargement.
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        let classes: Vec<RawClass> = serde_json::from_str(json)?;
        let mut index = SpellIndex::default();
        for class in classes {
            let class_name = if class.breed_id == COMMON_BREED_ID {
                Some(COMMON_CLASS)
            } else {
                class_for_breed(class.breed_id)
            };
            let Some(class_name) = class_name else {
                tracing::warn!(
                    breed_id = class.breed_id,
                    "classe inconnue dans le référentiel de sorts, entrées ignorées"
                );
                continue;
            };
            for spell in class.spells {
                let idx = index.entries.len();
                let key = normalize_wakfu_name(&spell.fr);
                index.entries.push(SpellEntry {
                    id: spell.id,
                    name: spell.fr,
                    class_name,
                    icon: spell.picture.as_deref().map(|picture| IconRef {
                        kind: IconKind::Spell,
                        gfx_id: picture_gfx_id(picture),
                    }),
                });
                index
                    .by_name_and_class
                    .entry((key.clone(), class_name))
                    .or_insert(idx);
                index.by_name.entry(key).or_insert(idx);
            }
        }
        Ok(index)
    }

    /// Résout un sort par son nom (brut du log ou du fichier, normalisation `normalize_wakfu_name`
    /// des deux côtés) et la classe de son lanceur — voir la doc de module pour l'ordre des replis.
    /// `None` si aucune entrée ne porte ce nom, quelle que soit la classe.
    pub fn find(&self, spell_name: &str, class_name: Option<&str>) -> Option<&SpellEntry> {
        let key = normalize_wakfu_name(spell_name);
        let idx = class_name
            .and_then(|class| self.by_name_and_class.get(&(key.clone(), class)))
            .or_else(|| self.by_name.get(&key))?;
        self.entries.get(*idx)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Numéro d'image à partir de l'URL `picture` du fichier (`…/wakassets/spells/2282.png` → `2282`)
/// — c'est ce numéro, et non l'URL complète, que `IconRef` transporte : `IconRef::image_url`
/// reconstruit l'URL `wakassets` (et `overlay_sync::icon_cache` nomme son fichier de cache
/// d'après lui). Le référentiel doit donc pointer sur `wakassets/spells/` — vérifié par le test
/// `toutes_les_images_du_referentiel_sont_sur_wakassets_spells` pour qu'une mise à jour du fichier
/// vers un autre hébergement ne casse pas silencieusement l'affichage.
fn picture_gfx_id(picture: &str) -> String {
    let file = picture.rsplit('/').next().unwrap_or(picture);
    file.strip_suffix(".png").unwrap_or(file).to_string()
}

/// Préfixe d'URL attendu pour chaque `picture` du référentiel — voir `picture_gfx_id`.
pub const WAKASSETS_SPELLS_URL_PREFIX: &str = "https://vertylo.github.io/wakassets/spells/";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_referentiel_embarque_est_valide() {
        let index = SpellIndex::embedded();
        assert!(index.len() >= 400, "{} entrées seulement", index.len());
    }

    /// Les sorts communs (pseudo-classe `common`, `breedId` −2) se résolvent pour n'importe quelle
    /// classe de lanceur, par le repli sur le nom seul.
    #[test]
    fn un_sort_commun_se_resout_pour_toute_classe() {
        let index = SpellIndex::embedded();
        for class in [Some("iop"), Some("sadida"), None] {
            let entry = index.find("Charme de Masse", class).expect("sort commun");
            assert_eq!(entry.class_name, COMMON_CLASS);
            assert_eq!(entry.id, 5623);
        }
    }

    /// Les mécaniques de classe ajoutées le 12 sept. (auparavant en pavé « ? » dans le bloc).
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
            assert_eq!(entry.class_name, class);
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
    fn toutes_les_images_du_referentiel_sont_sur_wakassets_spells() {
        let classes: Vec<RawClass> = serde_json::from_str(SPELLS_JSON).unwrap();
        for class in classes {
            for spell in class.spells {
                let Some(picture) = spell.picture else {
                    continue; // entrée sans image, connue sans icône (voir la doc de module)
                };
                assert!(
                    picture.starts_with(WAKASSETS_SPELLS_URL_PREFIX) && picture.ends_with(".png"),
                    "{} ({}) : image hors de wakassets/spells — `IconRef::image_url` ne saurait \
                     pas la reconstruire : {}",
                    spell.fr,
                    spell.id,
                    picture
                );
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
        assert_eq!(entry.class_name, "ouginak");
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
            .map(|e| e.class_name)
            .collect();
        assert!(
            classes.len() >= 2,
            "le doublon a disparu du fichier : {classes:?}"
        );
        for class in &classes {
            assert_eq!(
                index.find("Rafale", Some(class)).unwrap().class_name,
                *class
            );
        }
        assert_eq!(
            index.find("Rafale", None).unwrap().class_name,
            classes[0],
            "sans classe : première entrée du fichier"
        );
    }

    #[test]
    fn une_classe_sans_ce_sort_retombe_sur_le_nom_seul() {
        let index = SpellIndex::embedded();
        // Hachure est Ouginak ; demandé pour un Iop (roster périmé), on préfère l'icône Ouginak
        // à rien du tout.
        assert_eq!(
            index.find("Hachure", Some("iop")).unwrap().class_name,
            "ouginak"
        );
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
        assert!(index.entries.iter().any(|e| e.class_name == "sacrier"));
        assert!(!index.entries.iter().any(|e| e.class_name == "sacrieur"));
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
}
