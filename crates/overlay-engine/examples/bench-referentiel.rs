//! Mesure jetable du référentiel embarqué (`overlay_engine::referentiel`) : temps de chargement,
//! mémoire tenue par les index (allocateur de comptage, sans dépendance), coût de la recherche et
//! de la résolution des recettes.
//!
//! `cargo run --release -p overlay-engine --example bench-referentiel`
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Compte les octets alloués en ce moment et leur maximum atteint.
struct Comptage;

static COURANT: AtomicUsize = AtomicUsize::new(0);
static PIC: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Comptage {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let courant = COURANT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PIC.fetch_max(courant, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        COURANT.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static ALLOCATEUR: Comptage = Comptage;

fn mo(octets: usize) -> f64 {
    octets as f64 / (1024.0 * 1024.0)
}

fn meilleur<T>(tours: usize, mut f: impl FnMut() -> T) -> (Duration, T) {
    let mut best = Duration::MAX;
    let mut dernier = None;
    for _ in 0..tours {
        let t = Instant::now();
        let r = f();
        best = best.min(t.elapsed());
        dernier = Some(r);
    }
    (best, dernier.unwrap())
}

fn main() {
    let avant = COURANT.load(Ordering::Relaxed);
    PIC.store(avant, Ordering::Relaxed);
    let t0 = Instant::now();
    let referentiel = overlay_engine::referentiel::load();
    let duree = t0.elapsed();
    let tenu = COURANT.load(Ordering::Relaxed) - avant;
    let pic = PIC.load(Ordering::Relaxed) - avant;
    println!("premier chargement : {duree:?}");
    println!("mémoire tenue par les index : {:.1} Mo", mo(tenu));
    println!("pic pendant le chargement   : {:.1} Mo", mo(pic));

    let (best, _) = meilleur(10, overlay_engine::referentiel::load);
    println!("chargement (meilleur de 10) : {best:?}");

    let catalog = &referentiel.catalog;
    for (query, limit) in [
        ("bou", 40),
        ("bouftou", 40),
        ("bouftou", usize::MAX),
        ("pie", usize::MAX),
        ("épée", usize::MAX),
    ] {
        let (best, n) = meilleur(50, || catalog.search_items(query, 3, limit).len());
        println!("recherche {query:>8} limit={limit:<20} -> {n:>4} résultats, {best:?}");
    }

    // La recette la plus profonde parmi les objets qui en ont une.
    let craftables: Vec<i64> = catalog
        .search_items("", 0, usize::MAX)
        .into_iter()
        .chain(catalog.search_items("a", 1, usize::MAX))
        .filter(|item| catalog.find_item_has_recipe(&item.name, Some(item.id)))
        .map(|item| item.id)
        .collect();
    let t = Instant::now();
    let mut lignes = 0;
    for &id in &craftables {
        lignes += overlay_engine::resolve_recipe(id, catalog).len();
    }
    println!(
        "recettes : {} objets résolus ({lignes} lignes de 1er niveau) en {:?}",
        craftables.len(),
        t.elapsed()
    );
}
