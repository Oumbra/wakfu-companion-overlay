# `overlay-app` — câblage minimal de L1

Binaire minimal, **pas encore l'overlay final** : il branche [`overlay-ingest`](../overlay-ingest/)
(découverte de chemin + suivi temps réel) sur une sortie console, pour observer L1 tourner sur un
vrai `wakfu.log` avant de commencer l'UI (L2). Pas de fenêtre, pas de rendu, pas de config
persistée, pas de hotkey — voir docs/plan-architecture.md §4 : ce binaire est le point de départ de
`overlay-app`, il grossira au fil des lots suivants plutôt que d'être remplacé.

## Usage

```
cargo run -p overlay-app                    # découverte automatique du chemin (§5.1)
cargo run -p overlay-app -- <chemin.log>    # chemin explicite (pratique pour rejouer un fichier)
```

Chaque lot lu est affiché avec son étiquette (`RATTRAPAGE` pendant le rattrapage initial ou après
une rotation, `DIRECT` une fois le direct rattrapé — voir `overlay_ingest::Tailer`) et un aperçu des
premières lignes. `Ctrl+C` pour arrêter.

## Validation faite sur ce dépôt

Lancé contre le vrai `wakfu.log` de cette machine
(`%APPDATA%\zaap\gamesLogs\wakfu\logs\wakfu.log`, 2 751 lignes) : rattrapage initial en 2 lots
(2000 + 751, plafond `MAX_BATCH_LINES`), contenu réel affiché (lignes de combat, y compris une
trace d'exception Java multi-lignes — à garder en tête pour le futur parser L2, qui devra la
traiter comme plusieurs lignes de log indépendantes, pas comme un seul événement).

Cette exécution a d'ailleurs révélé un vrai bug dans `overlay-ingest` : une ligne ajoutée en direct
juste après le rattrapage initial restait étiquetée `RATTRAPAGE` (le drapeau `caught_up` du
`Tailer` ne passait à `true` qu'après un *second* appel `poll()` ne trouvant rien de neuf, jamais
garanti avant l'arrivée de la ligne suivante). Corrigé dans `overlay-ingest` — voir son
historique — et reconfirmé par une démo manuelle (ajout de ligne + rotation par renommage, comme le
fait réellement le client Wakfu) sur une copie jetable du log, jamais sur le fichier réel du jeu.
