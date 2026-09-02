/**
 * Point d'entrée du bundle embarqué par `overlay-engine` (docs/plan-architecture.md §2, §12 — L2).
 *
 * Expose l'API réelle de `LogParser` (vendue depuis `wakfu-companion`, voir
 * `VENDORED_FROM.txt`, zéro ligne modifiée) sous une forme appelable depuis QuickJS/Rust :
 * chaque fonction prend/renvoie des chaînes JSON, pour ne dépendre d'aucun mécanisme de
 * marshaling spécifique au binding QuickJS choisi côté Rust (`rquickjs`).
 *
 * Ceci N'EST PAS le futur `src/engine/headless.ts` prévu au §2.1 du plan (qui vivra dans le
 * dépôt `wakfu-companion` et couvrira aussi `StatsStoreService`, décision d'extraction encore
 * ouverte — §14 point 3) : volontairement réduit à `LogParser` seul, validé par le spike S2
 * (`spikes/s2-engine-quickjs/`, ~28 000 lignes/s). Suffisant pour les panneaux de L2 qui ne
 * dépendent que d'événements bruts (dégâts, butin, kamas, XP, début/fin de combat) — pas encore
 * pour l'historique agrégé multi-session ni les compteurs de suivi persistants, qui restent du
 * ressort de `StatsStoreService`.
 */
import { LogParser } from './log-parser';

// `isKnownMonsterName` (voir sa doc dans `LogParser`) est fourni par le harnais Rust AVANT
// l'évaluation de ce bundle (`hostIsKnownMonsterName` posé sur les globals par
// `quickjs_engine.rs::LogParserEngine::new`, lu ici en fermeture — jamais rappelé directement
// depuis `LogParser`, qui reste volontairement sans dépendance) : reproduit le même
// branchement que `StatsStoreService` côté web (`isKnownMonsterName: (name) =>
// this.catalog.isKnownWakfuMonsterName(name)`), sur `overlay_engine::CatalogIndex` plutôt que
// `CatalogService` — seule façon de protéger un vrai monstre qui se révèle (mimique, brèche —
// voir CLAUDE.md du dépôt web) contre le repli "invocation sans annonce" du parser (retour
// utilisateur 2026-09-02 : une invocation de mécanisme apparaissait à tort côté ennemis).
// `?? false` si le harnais n'a pas encore posé de catalogue (tout premier appel avant tout
// chargement réseau/cache) — comportement historique inchangé, jamais un blocage.
declare const globalThis: {
  wakfuEngine?: unknown;
  hostIsKnownMonsterName?: (name: string) => boolean;
};

const parser = new LogParser({
  isKnownMonsterName: (name) => globalThis.hostIsKnownMonsterName?.(name) ?? false,
});

function serialize(entry: unknown): string {
  return entry === null || entry === undefined ? '' : JSON.stringify(entry);
}

/** Analyse une ligne ; renvoie du JSON (LogEntry) ou une chaîne vide (rien à émettre). */
export function parseLine(rawLine: string): string {
  return serialize(parser.parseLine(rawLine));
}

/**
 * Vide un enregistrement multi-lignes en attente (ex. résumé d'échange) — à appeler après avoir
 * traité un lot complet de lignes (voir `parseBatch`), pas seulement en fin de fichier : c'est
 * l'usage documenté par `LogParser.flush()` lui-même, pour ne jamais laisser un enregistrement en
 * attente indéfiniment si la ligne suivante tarde. Renvoie une chaîne vide s'il n'y avait rien en
 * attente — sans effet de bord, donc sûr à appeler systématiquement.
 */
export function flush(): string {
  return serialize(parser.flush());
}

/** Réinitialise l'état de parsing (reconnexion / rotation — voir `isInitialLoad`, §5.3). */
export function resetParser(): void {
  parser.reset();
}

/**
 * Variante par lot, conforme au modèle de threading du plan (§3 : le thread IO envoie des
 * `LineBatch` de ≤ 2000 lignes, jamais ligne par ligne) — un seul aller-retour Rust↔QuickJS pour
 * tout le lot, au lieu d'un appel de fonction par ligne. `linesJoined` : lignes séparées par `\n`
 * (aucune ligne du format wakfu.log ne peut contenir ce caractère). Renvoie un JSON array des
 * `LogEntry` produits (le flush de fin de lot n'est PAS inclus : appeler `flush()` séparément
 * après, comme documenté ci-dessus).
 */
export function parseBatch(linesJoined: string): string {
  const out: unknown[] = [];
  for (const line of linesJoined.split('\n')) {
    const entry = parser.parseLine(line);
    if (entry !== null) out.push(entry);
  }
  return JSON.stringify(out);
}

// QuickJS (rquickjs) évalue le bundle comme un script global : pas de `export` au sens module ES
// dans le contexte d'exécution. On republie donc l'API sur `globalThis` pour que le harnais Rust
// l'appelle par un nom stable, indépendamment du format de bundle (`esbuild --format=iife`, voir
// build.mjs).
globalThis.wakfuEngine = { parseLine, flush, resetParser, parseBatch };
