//! Spike S2 (docs/plan-architecture.md §12) — question posée :
//!
//!   « QuickJS, embarqué dans un process Rust, peut-il exécuter le VRAI code TypeScript de
//!   parsing du dépôt web (`LogParser`, vendé sans modification de logique), à la vitesse
//!   requise, sur un fichier de log réel ? »
//!
//! Ce binaire n'est PAS l'ébauche du futur `overlay-engine` : c'est un harnais de mesure,
//! jetable, qui répond à cette seule question. Voir README.md de ce dossier pour le résultat.

use std::time::Instant;

use rquickjs::{Context, Function, Runtime};

const ENGINE_BUNDLE: &str = include_str!("../engine-js/dist/engine.bundle.js");
const SAMPLE_LOG: &str = include_str!("../tests/wakfu.log");

/// Cible du plan (§5.5) : 80 000 lignes ingérées en < 2 s. `tests/wakfu.log` (réel, vendé
/// depuis `wakfu-companion`) fait 10 975 lignes — on extrapole linéairement pour comparer à
/// la même cible, et on duplique aussi le fichier pour une mesure directe à volume réaliste.
const TARGET_LINES: usize = 80_000;
const TARGET_MS: u128 = 2_000;

fn run_once(js: &Context, lines: &[&str]) -> (usize, usize) {
    let mut entries = 0usize;
    let mut parse_errors = 0usize;

    js.with(|ctx| {
        let engine = ctx
            .globals()
            .get::<_, rquickjs::Object>("wakfuEngine")
            .expect("wakfuEngine absent du bundle — build.mjs a-t-il tourné ?");
        let reset: Function = engine.get("resetParser").unwrap();
        reset.call::<(), ()>(()).unwrap();

        let parse_line: Function = engine.get("parseLine").unwrap();
        for line in lines {
            match parse_line.call::<_, String>((*line,)) {
                Ok(json) if !json.is_empty() => entries += 1,
                Ok(_) => {}
                Err(err) => {
                    parse_errors += 1;
                    if parse_errors <= 3 {
                        eprintln!("  erreur de parsing sur une ligne : {err}");
                    }
                }
            }
        }

        let flush: Function = engine.get("flush").unwrap();
        if let Ok(json) = flush.call::<(), String>(()) {
            if !json.is_empty() {
                entries += 1;
            }
        }
    });

    (entries, parse_errors)
}

/// Conforme au modèle de threading réel (§3 du plan) : lots de ≤ `BATCH_SIZE` lignes, un seul
/// appel QuickJS par lot au lieu d'un appel par ligne.
const BATCH_SIZE: usize = 2000;

fn run_batched(js: &Context, lines: &[&str]) -> (usize, usize) {
    let mut entries = 0usize;
    let mut errors = 0usize;

    js.with(|ctx| {
        let engine = ctx
            .globals()
            .get::<_, rquickjs::Object>("wakfuEngine")
            .unwrap();
        let reset: Function = engine.get("resetParser").unwrap();
        reset.call::<(), ()>(()).unwrap();
        let parse_batch: Function = engine.get("parseBatch").unwrap();
        let flush: Function = engine.get("flush").unwrap();

        for chunk in lines.chunks(BATCH_SIZE) {
            let joined = chunk.join("\n");
            match parse_batch.call::<_, String>((joined,)) {
                Ok(json) => {
                    // On ne désérialise pas le détail ici (pas le sujet du spike) : compter les
                    // objets JSON de premier niveau suffit à vérifier qu'on obtient le même
                    // volume qu'en mode ligne-par-ligne.
                    let arr: Vec<serde_json::Value> =
                        serde_json::from_str(&json).unwrap_or_default();
                    entries += arr.len();
                }
                Err(err) => {
                    errors += 1;
                    eprintln!("  erreur sur un lot : {err}");
                }
            }
        }

        if let Ok(json) = flush.call::<(), String>(()) {
            if !json.is_empty() {
                entries += 1;
            }
        }
    });

    (entries, errors)
}

/// Diagnostic (spike uniquement) : même parsing, mais sans `JSON.stringify` — isole le coût de
/// la sérialisation JSON de celui du parsing/regex.
fn run_batched_count_only(js: &Context, lines: &[&str]) -> i64 {
    let mut total = 0i64;
    js.with(|ctx| {
        let engine = ctx
            .globals()
            .get::<_, rquickjs::Object>("wakfuEngine")
            .unwrap();
        let reset: Function = engine.get("resetParser").unwrap();
        reset.call::<(), ()>(()).unwrap();
        let f: Function = engine.get("parseBatchCountOnly").unwrap();
        for chunk in lines.chunks(BATCH_SIZE) {
            let joined = chunk.join("\n");
            let n: i64 = f.call((joined,)).unwrap();
            total += n;
        }
    });
    total
}

fn sample_entries(js: &Context, lines: &[&str], n: usize) -> Vec<String> {
    let mut out = Vec::new();
    js.with(|ctx| {
        let engine = ctx
            .globals()
            .get::<_, rquickjs::Object>("wakfuEngine")
            .unwrap();
        let reset: Function = engine.get("resetParser").unwrap();
        reset.call::<(), ()>(()).unwrap();
        let parse_line: Function = engine.get("parseLine").unwrap();
        for line in lines {
            if out.len() >= n {
                break;
            }
            if let Ok(json) = parse_line.call::<_, String>((*line,)) {
                if !json.is_empty() {
                    out.push(json);
                }
            }
        }
    });
    out
}

fn main() {
    let rt = Runtime::new().expect("création du runtime QuickJS");
    let ctx = Context::full(&rt).expect("création du contexte QuickJS");

    ctx.with(|ctx| {
        ctx.eval::<(), _>(ENGINE_BUNDLE)
            .expect("échec de chargement du bundle engine.bundle.js dans QuickJS");
    });

    let lines: Vec<&str> = SAMPLE_LOG.lines().collect();
    println!("=== Spike S2 — QuickJS embarqué + LogParser vendu (wakfu-companion) ===\n");
    println!("Fichier réel : tests/wakfu.log — {} lignes", lines.len());

    // --- 1. Aperçu qualitatif : les 8 premiers événements reconnus, pour vérifier qu'on
    //        n'obtient pas juste "ça tourne" mais bien les BONNES structures. ---
    println!("\n--- Aperçu des premiers LogEntry produits (JSON, tel qu'émis par le vrai parseur) ---");
    for json in sample_entries(&ctx, &lines, 8) {
        println!("  {json}");
    }

    // --- 2. Mesure sur le fichier réel tel quel. ---
    let start = Instant::now();
    let (entries, errors) = run_once(&ctx, &lines);
    let elapsed = start.elapsed();
    println!(
        "\n--- Passage 1 (fichier réel, {} lignes) ---\n  {} LogEntry produits, {} erreurs, {:?}",
        lines.len(),
        entries,
        errors,
        elapsed
    );

    // --- 3. Mesure à l'échelle de la cible du plan (80k lignes) : on répète le fichier réel
    //        pour atteindre un volume comparable, sans prétendre que c'est un fichier réel de
    //        cette taille (voir README.md, limites de la mesure). ---
    let repeats = TARGET_LINES.div_ceil(lines.len().max(1));
    let scaled: Vec<&str> = lines.iter().cycle().take(repeats * lines.len()).copied().collect();
    let start = Instant::now();
    let (entries_scaled, errors_scaled) = run_once(&ctx, &scaled);
    let elapsed_scaled = start.elapsed();
    println!(
        "\n--- Passage 2 (fichier réel x{}, {} lignes ≈ cible {} du plan) ---\n  {} LogEntry produits, {} erreurs, {:?}",
        repeats,
        scaled.len(),
        TARGET_LINES,
        entries_scaled,
        errors_scaled,
        elapsed_scaled
    );

    let verdict = if elapsed_scaled.as_millis() <= TARGET_MS {
        "OK — sous la cible de 2 s du plan (§5.5)"
    } else {
        "DÉPASSÉ — au-delà de la cible de 2 s du plan (§5.5)"
    };
    println!("\n=== Verdict (appel par ligne) : {verdict} ===");

    // --- 3bis. Même mesure, mais avec le découpage par lots réellement prévu par
    //           l'architecture (§3 : LineBatch ≤ 2000 lignes) — un appel QuickJS par lot,
    //           pas par ligne. ---
    let start = Instant::now();
    let (entries_batched, errors_batched) = run_batched(&ctx, &scaled);
    let elapsed_batched = start.elapsed();
    println!(
        "\n--- Passage 3 (mêmes {} lignes, PAR LOTS de {}, conforme §3 du plan) ---\n  {} LogEntry produits, {} erreurs, {:?}",
        scaled.len(),
        BATCH_SIZE,
        entries_batched,
        errors_batched,
        elapsed_batched
    );
    let verdict_batched = if elapsed_batched.as_millis() <= TARGET_MS {
        "OK — sous la cible de 2 s du plan (§5.5)"
    } else {
        "DÉPASSÉ — au-delà de la cible de 2 s du plan (§5.5)"
    };
    println!("=== Verdict (par lots) : {verdict_batched} ===");
    println!(
        "Accélération lots vs ligne-par-ligne : x{:.1}",
        elapsed_scaled.as_secs_f64() / elapsed_batched.as_secs_f64().max(0.000_001)
    );
    assert_eq!(
        entries_batched, entries_scaled,
        "le mode par lots doit produire EXACTEMENT le même nombre d'entrées que le mode ligne par ligne"
    );

    // --- 3ter. Isoler le coût de JSON.stringify vs celui du parsing/regex lui-même. ---
    let start = Instant::now();
    let count_only = run_batched_count_only(&ctx, &scaled);
    let elapsed_count_only = start.elapsed();
    println!(
        "\n--- Passage 4 (mêmes lignes, SANS JSON.stringify, juste un compteur) ---\n  {} LogEntry comptés, {:?}",
        count_only, elapsed_count_only
    );
    println!(
        "Part de JSON.stringify dans le temps total : {:.0} %",
        (1.0 - elapsed_count_only.as_secs_f64() / elapsed_batched.as_secs_f64()) * 100.0
    );

    // --- 4. Rejeu (parité) : ré-ingérer le même fichier depuis un état frais doit produire
    //        exactement le même nombre d'entrées — sinon l'état interne du parseur (fightStates,
    //        recentSignatures...) fuiterait d'un contexte QuickJS à l'autre. ---
    let (entries_replay, errors_replay) = run_once(&ctx, &lines);
    let identical = entries_replay == entries && errors_replay == errors;
    println!(
        "\n--- Rejeu (même contexte QuickJS, reset() explicite) : {} LogEntry, {} erreurs — {} ---",
        entries_replay,
        errors_replay,
        if identical { "identique au passage 1, OK" } else { "DIVERGENT — bug à investiguer" }
    );
}
