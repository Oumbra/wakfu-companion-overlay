//! Port direct de `wakfu-class-breed-ids.data.ts` (id de classe joueur émis par la ligne de
//! combat `[_FL_] ... B [id] isControlledByAI=...`) et de l'ordre des lignes de la planche de
//! portraits `class-avatars-sheet-*.png` (`class-portraits.data.ts::CLASS_PORTRAIT_ORDER`) —
//! `overlay-ui` s'en sert pour calculer le rectangle UV d'un portrait, `session.rs` pour
//! résoudre la classe d'un allié confirmé.
//!
//! ⚠️ `breed` seul n'est déterministe QUE pour un combattant déjà confirmé allié
//! (`isControlledByAI == false`) — un monstre peut porter un `breed` numériquement identique à un
//! id de classe joueur (ex. "Bouftou" = breed 1, collision avec Féca) sans rapport avec sa classe.
//! Voir `EntityClassifierService.registerFighterJoin` côté web pour la même règle. Id 17
//! volontairement absent (aucune classe jouable ne l'utilise).

/// Nom de classe (clé interne, ex. `"feca"`) pour un `breed` de combattant confirmé allié —
/// `None` si `breed` ne correspond à aucune classe jouable connue.
pub fn class_for_breed(breed: i64) -> Option<&'static str> {
    match breed {
        1 => Some("feca"),
        2 => Some("osamodas"),
        3 => Some("enutrof"),
        4 => Some("sram"),
        5 => Some("xelor"),
        6 => Some("ecaflip"),
        7 => Some("eniripsa"),
        8 => Some("iop"),
        9 => Some("cra"),
        10 => Some("sadida"),
        11 => Some("sacrier"),
        12 => Some("pandawa"),
        13 => Some("rogue"),
        14 => Some("zobal"),
        15 => Some("ouginak"),
        16 => Some("foggernaut"),
        18 => Some("eliotrope"),
        19 => Some("huppermage"),
        _ => None,
    }
}

/// Ordre RÉEL des lignes de la planche `class-avatars-sheet-*.png` — miroir exact de
/// `CLASS_PORTRAIT_ORDER` (`class-portraits.data.ts`). Ne JAMAIS réordonner sans reconstruire la
/// planche en conséquence.
pub const CLASS_PORTRAIT_ORDER: [&str; 18] = [
    "cra",
    "ecaflip",
    "sacrier",
    "zobal",
    "sram",
    "ouginak",
    "iop",
    "eniripsa",
    "feca",
    "sadida",
    "enutrof",
    "eliotrope",
    "foggernaut",
    "huppermage",
    "xelor",
    "osamodas",
    "pandawa",
    "rogue",
];

/// Ligne (0-17) d'une classe dans la planche — miroir de `getClassPortraitRow`. `None` si
/// `class_name` ne correspond à aucune classe connue (l'appelant décide du repli, contrairement au
/// web qui replie silencieusement sur la ligne 0 — ici on préfère ne pas dessiner de portrait du
/// tout plutôt qu'un portrait trompeur).
pub fn class_portrait_row(class_name: &str) -> Option<usize> {
    CLASS_PORTRAIT_ORDER.iter().position(|&c| c == class_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mappe_tous_les_breeds_1_a_19_sauf_17() {
        for breed in 1..=19 {
            if breed == 17 {
                assert_eq!(class_for_breed(breed), None, "breed 17 doit rester absent");
            } else {
                assert!(
                    class_for_breed(breed).is_some(),
                    "breed {breed} devrait mapper une classe"
                );
            }
        }
    }

    #[test]
    fn rejette_un_breed_hors_plage() {
        assert_eq!(class_for_breed(0), None);
        assert_eq!(class_for_breed(20), None);
        assert_eq!(class_for_breed(-1), None);
    }

    #[test]
    fn chaque_classe_a_une_ligne_dans_la_planche() {
        for (i, class_name) in CLASS_PORTRAIT_ORDER.iter().enumerate() {
            assert_eq!(class_portrait_row(class_name), Some(i));
        }
        assert_eq!(class_portrait_row("inconnue"), None);
    }
}
