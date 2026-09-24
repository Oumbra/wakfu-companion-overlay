---
paths:
  - ".github/**"
  - "scripts/ci-local.sh"
  - "rust-toolchain.toml"
  - ".githooks/pre-push"
---

# ci

Portée : workflow `ci.yml`, son double local `scripts/ci-local.sh`, la toolchain épinglée, le hook
`pre-push`, le coût et les symptômes d'un CI qui ne tourne pas. Tout nouvel apprentissage sur ce
sujet se documente ici, jamais dans `CLAUDE.md` (qui ne garde que les trois garde-fous).

## Pourquoi trois garde-fous

Le CI (`.github/workflows/ci.yml`) est resté **rouge en continu du 2026-09-01 au 2026-09-10** pour
une seule raison : un écart de `cargo fmt` dans `crates/overlay-engine/src/session.rs` que rien ne
signalait avant le push. Chaque commit suivant repartait rouge sans rapport avec son contenu, et
plus personne ne lisait le verdict. Trois garde-fous existent désormais — les utiliser :

1. **Toolchain épinglée** (`rust-toolchain.toml`). `rustup` sélectionne la bonne version tout seul.
   Ne **jamais** la contourner (`cargo +stable …`) : le verdict de `fmt`/`clippy` dépend de la
   version, et c'est précisément la dérive qui a cassé le CI. Monter la version de Rust est un
   changement **explicite**, dans son propre commit, avec le reformatage qu'il entraîne.
2. **`bash scripts/ci-local.sh`** rejoue les vérifications du CI pour la plateforme courante
   (`--lint` pour format + clippy seuls, rapide). **À lancer avant tout push touchant du code** —
   y compris en session cloud, où c'est le seul retour disponible avant plusieurs minutes de CI.
3. **Hook `pre-push`** (`bash scripts/install-hooks.sh`, à relancer **en début de chaque session
   cloud** : `core.hooksPath` est une config locale, elle ne survit pas au conteneur éphémère).

Une étape ajoutée ou retirée dans `.github/workflows/ci.yml` doit l'être **aussi** dans
`scripts/ci-local.sh`, et réciproquement : les deux se doublent volontairement, un garde-fou ne
protège que ce qu'il connaît.

Le gate de captures a sa propre règle (`.claude/rules/captures.md`) : `ci-local.sh` sans option
compte en échec les écarts trop grands pour du bruit de rastérisation, et le hook `pre-push`
avertit quand un push touche du code de rendu sans une seule référence.

## Coût du CI (dépôt public — minutes Actions gratuites, sobriété conservée)

Le dépôt est **public** (vérifié sur l'API GitHub le 2026-09-15, `"private": false`) : les jobs
sur runners standard ne consomment aucun quota. Il a été privé jusque-là, et le symptôme d'un
quota épuisé reste bon à connaître si la visibilité changeait à nouveau : **tous les jobs échouent
en quelques secondes, sans log ni runner assigné** (arrivé à partir du 2026-09-09). Devant ce
symptôme, vérifier la visibilité et la facturation du compte avant de chercher une régression.

Le workflow reste sobre par principe (durée d'attente du verdict, pas seulement coût) : cache
`cargo`/`vendor`, `concurrency` (une rafale de commits n'exécute que le dernier run), et
`paths-ignore` sur `docs/**`, `**/*.md` et `.claude/**`. Ne pas étendre `paths-ignore` à
`assets/**` : ces fichiers sont embarqués par `include_bytes!` (`design/assets.rs`,
`design/fonts.rs`, `ui_icons.rs`…) et peuvent casser la compilation comme les snapshots
d'`overlay-testkit`.
