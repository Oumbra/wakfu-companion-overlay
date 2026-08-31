# Spike S2 — Bundle headless + QuickJS sur `wakfu.log`

Voir [`docs/plan-architecture.md`](../../docs/plan-architecture.md) §2 (décision structurante n°1)
et §12 (feuille de route, S2). Question posée : **QuickJS, embarqué dans un process Rust,
peut-il exécuter le vrai code TypeScript de parsing du dépôt web, à la vitesse requise, sur un
fichier de log réel ?**

## Méthode

- Code métier **vendu tel quel** depuis `Oumbra/wakfu-companion` (commit voir
  `engine-js/VENDORED_FROM.txt`) : `log-parser.ts` (1 053 l.) et `log-entry.model.ts` (243 l.),
  **zéro ligne modifiée**. `LogParser` s'est avéré être une classe pure (aucune dépendance
  Angular, API publique `parseLine()`/`flush()`/`reset()`) — le meilleur candidat possible du
  dépôt pour ce test, et une bonne nouvelle en soi pour la stratégie du §2.
- Bundle avec `esbuild` (`--format=iife --target=es2020`), 30 Ko, aucune dépendance runtime restante.
- Chargé dans QuickJS via `rquickjs` 0.12 (backend `quickjs-ng`), appelé depuis un harnais Rust
  (`src/main.rs`) sur `tests/wakfu.log` — fichier réel de 10 975 lignes, vendu depuis les tests du
  dépôt web.
- Quatre passages : ligne par ligne, par lots de 2000 (conforme au modèle de threading réel, §3
  du plan), sans `JSON.stringify` (isole le coût de sérialisation), et un rejeu (parité d'état
  après `reset()`).

Reproduire : `cd engine-js && npm install && npm run build && cd .. && cargo run --release`.

## Résultats

Matériel de mesure : bac à sable cloud, Xeon @ 2.80 GHz, 4 vCPU — **probablement plus lent qu'un
PC de joueur réel** (fréquence par cœur nettement inférieure à un CPU gaming moderne). Les chiffres
absolus sont donc un plancher pessimiste, pas une mesure définitive.

| Passage | Volume | Résultat | Temps |
| --- | --- | --- | --- |
| Fichier réel, ligne par ligne | 10 975 lignes | 4 265 `LogEntry` | **364 ms** |
| Fichier réel ×8, ligne par ligne | 87 800 lignes | 32 802 `LogEntry` | **3,1 s** |
| Même volume, **par lots de 2000** | 87 800 lignes | 32 802 `LogEntry` (identique) | 3,2 s |
| Même volume, sans `JSON.stringify` | 87 800 lignes | 32 802 (compteur) | 3,0 s |
| Rejeu après `reset()` | 10 975 lignes | 4 265 (identique au 1ᵉʳ passage) | — |

## Constats

1. **Correction fonctionnelle : conforme.** Les premiers `LogEntry` produits (aperçu dans la
   sortie du binaire) correspondent exactement à ce qu'on attend d'une lecture de
   `tests/wakfu.log` : ancrage de date, chat par canal, butin, ouverture de session HDV. Le rejeu
   après `reset()` produit un résultat strictement identique — pas de fuite d'état entre deux
   (re)connexions dans le même contexte QuickJS, point sensible pour `isInitialLoad` (§5.3 du plan).
2. **Le découpage en lots n'apporte aucun gain (×1,0).** L'hypothèse initiale (overhead de
   marshaling FFI par appel) est **infirmée** : appeler QuickJS une fois par ligne ou une fois par
   lot de 2000 ne change rien au temps total.
3. **`JSON.stringify` ne représente que ~7 % du temps.** Le reste (~93 %) est le coût brut
   d'exécution du parsing (dispatch de ~20 regex par ligne, accumulation d'état) **interprété par
   QuickJS**, sans JIT — contrairement à V8 (Chrome/Node), QuickJS n'a pas de compilation à la
   volée. C'est la vraie explication du dépassement, pas un défaut d'intégration Rust.
4. **Débit mesuré : ~28 000 lignes/s.** La cible du plan (§5.5, 80 000 lignes en moins de 2 s, soit
   ~40 000 lignes/s) est dépassée d'environ ×1,4 sur ce matériel — pas d'un ordre de grandeur, mais
   réel.

## Ce que ça change pour l'architecture (§2 et §5.5 du plan)

**Le choix « réutiliser le TS via QuickJS » n'est pas remis en cause** — l'écart de perf est
mesuré, borné (×1,4, pas ×10), et concentré sur un seul moment (le chargement initial d'un fichier
volumineux), jamais sur le chemin chaud en jeu (une ligne de log toutes les quelques centaines de
ms en combat, très loin de la limite mesurée). Deux ajustements, déjà cohérents avec le reste du
plan mais que ce résultat rend **obligatoires plutôt qu'optionnels** :

1. **Le critère de succès du §5.5 doit être reformulé.** Le modèle de threading (§3) place déjà
   l'ingestion sur un thread séparé du rendu — l'UI ne gèle donc jamais, quel que soit le temps
   d'ingestion. Le vrai critère produit n'est pas « < 2 s » dans l'absolu, mais : *l'ingestion
   initiale d'un gros fichier ne bloque jamais le rendu, et progresse visiblement*. Passé de ~2 s à
   ~3-5 s pour un fichier réel de 80 k lignes, c'est un non-événement si une barre de progression
   s'affiche — **c'était déjà vrai côté web** (l'incident du 2026-08-30 documenté dans `CLAUDE.md`
   de `wakfu-companion` était un vrai gel, sans spinner ; ici il n'y a structurellement pas de gel).
2. **La publication de snapshots intermédiaires pendant `isInitialLoad` devient un prérequis de
   L1**, pas une amélioration ultérieure : publier un `UiSnapshot` toutes les quelques dizaines de
   lots (pas seulement à la fin) pour alimenter une barre de progression réelle.

Aucun changement sur le choix moteur lui-même. Une piste d'optimisation reste ouverte si le besoin
se confirme en usage réel (mesure sur machine de joueur, pas seulement ce bac à sable) : profiler le
dispatch des ~20 regex de `parseLine` pour resserrer l'ordre de test (les plus fréquentes en
premier) — une optimisation **dans le dépôt web**, qui bénéficierait aux deux clients, pas un
contournement côté overlay.

## Limites de cette mesure

- Un seul fichier réel (10 975 lignes) répété ×8 pour atteindre le volume cible — pas un fichier
  réel de cette taille. La distribution des types de lignes (beaucoup de chat, peu de combats
  simultanés) peut différer d'une vraie session de 80 000 lignes.
- Mesuré sur un carrier cloud, pas sur un PC de joueur — à revalider une fois L1 en place, sur
  matériel réel, idéalement le plus modeste visé par le projet.
- `StatsStoreService` (2 755 lignes, bien plus lourd et étroitement couplé à Angular/signals) n'est
  **pas** couvert par ce spike — seul `LogParser` l'est. C'est la pièce la plus simple à extraire ;
  rien ne garantit que `StatsStoreService` s'extrait aussi proprement (§2.1 du plan, lot à part
  entière côté `wakfu-companion`).
