//! Mesure jetable : coût de `CatalogIndex::search_items` sur le catalogue réel du poste.
use std::time::Instant;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("chemin du catalog-cache.json");
    let raw = std::fs::read_to_string(&path).expect("lecture");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("json");
    let data = value.get("index").cloned().unwrap_or(value);
    let t0 = Instant::now();
    let index = overlay_engine::CatalogIndex::from_compact_json(&data);
    println!("construction : {:?}", t0.elapsed());
    for (query, limit) in [
        ("bou", 40),
        ("bouf", 40),
        ("bouftou", 40),
        ("bouftou", usize::MAX),
        ("pie", usize::MAX),
        ("épée", usize::MAX),
    ] {
        let mut best = std::time::Duration::MAX;
        let mut n = 0;
        for _ in 0..50 {
            let t = Instant::now();
            let r = index.search_items(query, 3, limit);
            best = best.min(t.elapsed());
            n = r.len();
        }
        println!("{query:>8} limit={limit:<20} -> {n:>4} résultats, meilleur temps {best:?}");
    }
}
