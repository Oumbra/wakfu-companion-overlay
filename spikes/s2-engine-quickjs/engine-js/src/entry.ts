/**
 * Point d'entrée du bundle headless pour le spike S2 (docs/plan-architecture.md §12/§2).
 *
 * Expose l'API réelle de `LogParser` (vendée depuis `wakfu-companion`, voir
 * `VENDORED_FROM.txt`) sous une forme appelable depuis QuickJS/Rust : chaque fonction
 * prend/renvoie des chaînes JSON, pour ne dépendre d'aucun mécanisme de marshaling
 * spécifique au binding QuickJS choisi côté Rust.
 *
 * Ceci N'EST PAS le futur `src/engine/headless.ts` prévu au §2.1 du plan (qui vivra dans
 * le dépôt `wakfu-companion` et couvrira aussi `StatsStoreService`) : c'est un harnais de
 * spike, volontairement réduit à `LogParser` seul, pour répondre à UNE question — QuickJS
 * peut-il exécuter le vrai code de parsing à la vitesse requise ?
 */
import { LogParser } from './log-parser';

const parser = new LogParser();

function serialize(entry: unknown): string {
  return entry === null || entry === undefined ? '' : JSON.stringify(entry);
}

/** Analyse une ligne ; renvoie du JSON (LogEntry) ou une chaîne vide (rien à émettre). */
export function parseLine(rawLine: string): string {
  return serialize(parser.parseLine(rawLine));
}

/** À appeler en fin de fichier : vide un enregistrement multi-lignes en attente. */
export function flush(): string {
  return serialize(parser.flush());
}

/** Réinitialise l'état de parsing (reconnexion / nouveau fichier — voir `isInitialLoad`). */
export function resetParser(): void {
  parser.reset();
}

/**
 * Variante par lot, conforme au modèle de threading du plan (§3 : le thread IO envoie des
 * `LineBatch` de ≤ 2000 lignes, jamais ligne par ligne) — un seul aller-retour Rust↔QuickJS
 * pour tout le lot, au lieu d'un appel de fonction par ligne. `linesJoined` : lignes séparées
 * par `\n` (aucune ligne du format wakfu.log ne peut contenir ce caractère). Renvoie un JSON
 * array des `LogEntry` produits (le flush de fin de lot n'est PAS inclus : le vrai flush de
 * fin de fichier reste un appel explicite à `flush()`).
 */
export function parseBatch(linesJoined: string): string {
  const out: unknown[] = [];
  for (const line of linesJoined.split('\n')) {
    const entry = parser.parseLine(line);
    if (entry !== null) out.push(entry);
  }
  return JSON.stringify(out);
}

/**
 * Variante diagnostique (spike uniquement, PAS destinée à survivre au-delà) : même travail de
 * parsing que `parseBatch`, mais sans aucun `JSON.stringify` — ne renvoie qu'un compteur. Sert à
 * isoler le coût de la sérialisation JSON de celui du parsing/regex lui-même.
 */
export function parseBatchCountOnly(linesJoined: string): number {
  let count = 0;
  for (const line of linesJoined.split('\n')) {
    if (parser.parseLine(line) !== null) count++;
  }
  return count;
}

// QuickJS (rquickjs) évalue le bundle comme un script global : pas de `export` au sens
// module ES dans le contexte d'exécution. On republie donc l'API sur `globalThis` pour
// que le harnais Rust l'appelle par un nom stable, indépendamment du format de bundle
// (`esbuild --format=iife`, voir build.mjs).
declare const globalThis: { wakfuEngine?: unknown };
globalThis.wakfuEngine = { parseLine, flush, resetParser, parseBatch, parseBatchCountOnly };
