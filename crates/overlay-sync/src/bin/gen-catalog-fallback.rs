//! Régénère `assets/catalog/catalog-index.json.gz` (repli hors-ligne embarqué, voir la doc de tête
//! de `catalog_cache.rs`) depuis un VRAI déploiement — `cargo run -p overlay-sync --bin
//! gen-catalog-fallback` (option `WAKFU_COMPANION_API_URL` comme le reste du crate, voir
//! `client::base_url`, pour cibler dev/prod/un `wrangler pages dev` local).
//!
//! **Session requise** (2026-09-20, `session.rs`) : `GET /api/v1/catalog/` n'est servi qu'au site
//! et à une session de l'overlay. Ce binaire réutilise le jeton natif que l'overlay de ce poste a
//! enregistré à son appairage (`token_store::load_token`, trousseau du système ou fichier de
//! repli) — le même que l'overlay pose sur cette route. Il faut donc un overlay appairé contre le
//! déploiement visé : un jeton n'est valable que pour le déploiement qui l'a émis.
//!
//! À lancer avant toute release réelle : le fichier actuellement commité est un PLACEHOLDER réduit
//! (2 entrées), construit dans un sandbox de dev sans accès réseau à Neon/`*.pages.dev` — voir la
//! doc de tête de `catalog_cache.rs` pour le détail de cette limite. Ce binaire, lui, N'A PAS cette
//! limite : lancé depuis un poste avec un vrai accès réseau à l'API, il embarque le catalogue
//! COMPLET réel (~11 700 objets / ~850 monstres, ~350 Ko gzip mesurés côté serveur).

use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Jeton de l'overlay appairé sur ce poste (voir la doc de tête) : sans lui, inutile d'appeler
    // l'API, elle répondra 403.
    match overlay_sync::token_store::load_token() {
        Some(token) => overlay_sync::session::publish(&token),
        None => {
            eprintln!(
                "aucun jeton natif enregistré sur ce poste : appairer l'overlay contre {} d'abord (le catalogue n'est servi qu'à une session valide)",
                overlay_sync::client::base_url()
            );
            std::process::exit(2);
        }
    }
    let index = overlay_sync::fetch_catalog_index().unwrap_or_else(|err| {
        eprintln!(
            "échec de récupération du catalogue depuis {} : {err}",
            overlay_sync::client::base_url()
        );
        std::process::exit(1);
    });

    let json = serde_json::to_vec(&index).expect("le catalogue récupéré doit rester sérialisable");

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder
        .write_all(&json)
        .expect("compression gzip en mémoire, ne devrait jamais échouer");
    let gz = encoder.finish().expect("finalisation gzip");

    // `env!("CARGO_MANIFEST_DIR")` = `crates/overlay-sync` — le même chemin que celui lu par
    // `include_bytes!` dans `catalog_cache.rs` (relatif à CE fichier source), donc toujours
    // cohérent quel que soit le répertoire courant depuis lequel `cargo run` est lancé.
    let dest: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "assets",
        "catalog",
        "catalog-index.json.gz",
    ]
    .iter()
    .collect();
    std::fs::create_dir_all(dest.parent().expect("dest a toujours un parent")).unwrap();
    std::fs::write(&dest, &gz).unwrap_or_else(|err| {
        eprintln!("échec d'écriture de {} : {err}", dest.display());
        std::process::exit(1);
    });

    println!(
        "OK : {} régénéré ({} octets bruts / {} octets gzip) — à commiter.",
        dest.display(),
        json.len(),
        gz.len()
    );
}
