//! Identité du build : numéro de version SemVer et hash du commit dont il est issu.
//!
//! Les deux sont figés **à la compilation**, sans aucun fichier généré à committer (contrairement
//! au dépôt web, qui écrit un `build-info.data.ts` avant chaque build) : Cargo expose déjà la
//! version du manifeste, et `build.rs` y ajoute le hash. Rien à régénérer, rien à oublier.
//!
//! Ce que chacun raconte :
//! - [`VERSION`] vient de `[workspace.package] version` (racine du dépôt), incrémenté
//!   automatiquement par le hook `post-commit` selon le type Conventional Commits du commit —
//!   voir `scripts/bump-version.sh`. C'est le numéro **public**, celui qu'on cite dans une release.
//! - [`COMMIT`] désigne l'état EXACT du dépôt. Plusieurs commits partagent le même
//!   [`VERSION`] (un `docs:` n'en déclenche aucun), donc lui seul permet de reproduire un build.
//!
//! Où ils apparaissent : [`banner_label`] dans la bannière des fenêtres du design system
//! (`design::window`), et [`full_label`] dans la ligne « session démarrée » du journal
//! (`logging::init`) — le rapport de bug d'un utilisateur porte ainsi les deux, qu'il envoie une
//! capture d'écran ou son fichier de log.

use std::sync::OnceLock;

/// Version SemVer du produit, commune à toutes les crates du workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Hash court du commit compilé, ou `"inconnu"` — voir `build.rs`.
pub const COMMIT: &str = env!("WAKFU_OVERLAY_COMMIT");

/// Ce que peint la bannière des fenêtres : `0.4.2`.
///
/// Volontairement SANS le hash : la bannière est un bandeau étroit partagé avec le titre centré et
/// la croix de fermeture, et le numéro public suffit à un utilisateur qui veut savoir s'il est à
/// jour. Le hash reste à un geste de là, dans le journal ([`FULL_LABEL`]).
///
/// Sans préfixe `v` non plus (choix de l'utilisateur, 2026-09-14) : à cette place, rien d'autre
/// qu'un numéro de version ne peut s'afficher — la lettre n'apprend rien et allonge le seul élément
/// que la bannière pousse vers son titre.
pub const BANNER_LABEL: &str = env!("CARGO_PKG_VERSION");

/// Identité complète, pour le journal : `0.4.2 (a1b2c3d)`.
pub const FULL_LABEL: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("WAKFU_OVERLAY_COMMIT"),
    ")"
);

/// Surcharge posée par les tests de capture — voir [`freeze_for_snapshots`].
static FROZEN: OnceLock<&'static str> = OnceLock::new();

/// Libellé à peindre dans une bannière de fenêtre.
///
/// Passe par une fonction plutôt que d'utiliser [`BANNER_LABEL`] directement pour laisser les
/// tests de capture le figer ([`freeze_for_snapshots`]).
pub fn banner_label() -> &'static str {
    FROZEN.get().copied().unwrap_or(BANNER_LABEL)
}

/// **Réservé aux tests de capture** (`overlay-testkit`) : fige le libellé de bannière sur une
/// valeur constante.
///
/// Sans cela, le gate de captures du CI (§17.1 du plan) virerait au rouge **à chaque commit** qui
/// bump la version : toutes les images montrant une fenêtre du design system porteraient un numéro
/// différent de leur référence, pour un changement visuel que personne n'a demandé. Ce n'est pas un
/// risque théorique — le bump est automatique et la modale Options compte à elle seule une
/// trentaine de captures.
///
/// Idempotente : le premier appel gagne, les suivants sont ignorés (les tests d'un même binaire
/// partagent le process et s'exécutent en parallèle — tous doivent poser la MÊME valeur, d'où une
/// fonction sans paramètre plutôt qu'un setter libre).
pub fn freeze_for_snapshots() {
    let _ = FROZEN.set(FROZEN_LABEL);
}

/// Le libellé figé des captures. Même gabarit qu'un vrai numéro (trois composantes d'un chiffre),
/// pour que la référence rende compte de la place réellement occupée en production.
pub const FROZEN_LABEL: &str = "0.0.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_libelle_de_banniere_est_la_version_du_manifeste() {
        assert_eq!(BANNER_LABEL, VERSION);
        // Sans surcharge posée, c'est bien la vraie version qui est peinte.
        assert_eq!(banner_label(), BANNER_LABEL);
    }

    #[test]
    fn le_libelle_complet_porte_version_et_commit() {
        assert_eq!(FULL_LABEL, format!("{VERSION} ({COMMIT})"));
        assert!(!COMMIT.is_empty());
    }
}
