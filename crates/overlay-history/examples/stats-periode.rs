//! Outil de vérification : agrégats d'une période de la base locale, sous une forme canonique
//! (listes triées) comparable à `GET /api/v1/history/stats` du site.
//!
//! `cargo run -p overlay-history --example stats-periode -- <début ISO> <fin ISO> [base]`
use overlay_engine::log_time::parse_iso_utc_ms;
use overlay_history::stats::{GroupTotals, ItemTotal, XpTotal};
use overlay_history::HistoryStore;
use serde_json::{json, Value};

fn items(list: &[ItemTotal]) -> Value {
    let mut rows: Vec<String> = list
        .iter()
        .map(|item| {
            format!(
                "{}|{}|{}",
                item.item_id.map(|id| id.to_string()).unwrap_or_default(),
                item.item_name.clone().unwrap_or_default(),
                item.quantity
            )
        })
        .collect();
    rows.sort();
    json!(rows)
}

fn xp(list: &[XpTotal]) -> Value {
    let mut rows: Vec<String> = list
        .iter()
        .map(|row| format!("{}|{}", row.name, row.amount))
        .collect();
    rows.sort();
    json!(rows)
}

fn group(key: String, totals: &GroupTotals) -> Value {
    json!({
        "key": key, "fights": totals.fights, "dungeonRuns": totals.dungeon_runs,
        "won": totals.won, "lost": totals.lost, "kamasGained": totals.kamas_gained,
        "xpGained": totals.xp_gained, "xpByCharacter": xp(&totals.xp_by_character),
        "loot": items(&totals.loot),
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let since = parse_iso_utc_ms(&args[1]).expect("début ISO");
    let until = parse_iso_utc_ms(&args[2]).expect("fin ISO");
    let path = args
        .get(3)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| HistoryStore::default_path().unwrap());
    let referentiel = overlay_engine::referentiel::load();
    let store = HistoryStore::open(&path).unwrap();
    let started = std::time::Instant::now();
    let stats = store
        .period_stats(since, until, &referentiel.catalog)
        .unwrap();
    eprintln!("agrégats en {:?}", started.elapsed());
    let mut dungeons: Vec<Value> = stats
        .dungeons
        .iter()
        .map(|(id, totals)| group(id.to_string(), totals))
        .collect();
    dungeons.sort_by_key(|value| value["key"].as_str().unwrap().to_string());
    let mut families: Vec<Value> = stats
        .families
        .iter()
        .map(|(id, totals)| group(id.map(|id| id.to_string()).unwrap_or_default(), totals))
        .collect();
    families.sort_by_key(|value| value["key"].as_str().unwrap().to_string());
    let out = json!({
        "combats": [stats.combats.won, stats.combats.lost, stats.combats.challenges_passed,
                    stats.combats.challenges_failed],
        "kamas": [stats.kamas.from_combat, stats.kamas.from_hdv_sales,
                  stats.kamas.spent_on_purchases, stats.kamas.trades_acquired,
                  stats.kamas.trades_given, stats.kamas.trade_count],
        "xpByCharacter": xp(&stats.xp_by_character),
        "loot": items(&stats.loot),
        "dungeons": dungeons,
        "families": families,
        "pacts": items(&stats.pacts),
    });
    println!("{}", serde_json::to_string(&out).unwrap());
}
