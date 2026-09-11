//! Cache disque des icônes réelles d'objets/monstres/raretés/catégories (`wakassets`, voir
//! `overlay_engine::IconRef::image_url`) — évite de retélécharger la même image à chaque
//! lancement. Un fichier PNG par icône plutôt qu'une base : le contenu ne change jamais pour un
//! `gfx_id` donné (image statique d'un CDN tiers), donc pas de notion d'expiration/`ETag` à gérer
//! ici (contrairement à `catalog_cache.rs`, dont le contenu évolue avec le référentiel du jeu).

use std::path::PathBuf;

use overlay_engine::IconKind;

#[cfg(not(test))]
const APP_NAME: &str = "wakfu-companion-overlay";
#[cfg(test)]
const APP_NAME: &str = "wakfu-companion-overlay-test";

fn cache_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", APP_NAME).map(|dirs| dirs.data_dir().join("icons"))
}

fn file_path(kind: IconKind, gfx_id: &str) -> Option<PathBuf> {
    let folder = match kind {
        IconKind::Item => "items",
        IconKind::Monster => "monsters",
        // Huit fichiers en tout, et jamais renouvelés : les gemmes se mettent en cache comme le
        // reste plutôt que d'être retéléchargées à chaque lancement. Même nom de dossier que le
        // sous-dossier du CDN (`IconRef::image_url`), pour que le cache se lise comme l'URL.
        IconKind::Rarity => "rarities",
        // Même dossier que le CDN. Attention au garde-fou juste en dessous : le bouton « Tout »
        // porte le numéro `-1`, qui ne contient aucun séparateur de chemin — il passe, et doit
        // continuer à passer si ce filtre est un jour resserré.
        IconKind::ItemCategory => "itemTypes",
    };
    // `gfx_id` vient du catalogue serveur, jamais construit à partir d'une entrée non fiable —
    // mais un id qui contiendrait par accident un séparateur de chemin ne doit quand même jamais
    // écrire hors du dossier de cache prévu (défense en profondeur, coût nul ici).
    if gfx_id.contains(['/', '\\', '.']) {
        return None;
    }
    cache_dir().map(|dir| dir.join(folder).join(format!("{gfx_id}.png")))
}

/// Octets déjà en cache, `None` si jamais téléchargée (ou id suspect, voir `file_path`) — jamais
/// une erreur, l'appelant retélécharge simplement dans ce cas.
pub fn load(kind: IconKind, gfx_id: &str) -> Option<Vec<u8>> {
    std::fs::read(file_path(kind, gfx_id)?).ok()
}

/// Écrit en cache — best-effort silencieux : une icône non mise en cache est juste
/// retéléchargée au prochain lancement, jamais une raison de faire échouer l'affichage.
pub fn save(kind: IconKind, gfx_id: &str, bytes: &[u8]) {
    let Some(path) = file_path(kind, gfx_id) else {
        return;
    };
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let _ = std::fs::write(path, bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sauvegarde_puis_lecture_coherentes() {
        let handle = std::thread::Builder::new()
            .name("icon-cache-test".into())
            .spawn(|| {
                save(IconKind::Item, "test-1234", b"contenu-png-factice");
                assert_eq!(
                    load(IconKind::Item, "test-1234"),
                    Some(b"contenu-png-factice".to_vec())
                );
                if let Some(path) = file_path(IconKind::Item, "test-1234") {
                    let _ = std::fs::remove_file(path);
                }
            })
            .unwrap();
        handle.join().unwrap();
    }

    /// Le bouton « Tout » de la bande de filtres porte le numéro `-1` : un identifiant
    /// parfaitement légitime, que le garde-fou anti-traversée ne doit pas confondre avec un
    /// chemin suspect. C'est le seul gfx_id négatif du référentiel.
    #[test]
    fn le_numero_moins_un_reste_un_identifiant_valide() {
        let chemin = file_path(IconKind::ItemCategory, "-1").expect("« -1 » est un id valide");
        assert!(chemin.ends_with("itemTypes/-1.png"));
    }

    #[test]
    fn identifiant_suspect_ne_sort_jamais_du_dossier_de_cache() {
        assert!(file_path(IconKind::Item, "../../evil").is_none());
        assert!(load(IconKind::Item, "../../evil").is_none());
    }

    #[test]
    fn icone_jamais_telechargee_renvoie_none() {
        assert!(load(IconKind::Monster, "jamais-vu").is_none());
    }
}
