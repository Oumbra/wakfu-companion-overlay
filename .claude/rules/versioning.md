---
paths:
  - "Cargo.toml"
  - "Cargo.lock"
  - "crates/*/Cargo.toml"
  - ".githooks/**"
  - "scripts/bump-version.sh"
  - "scripts/install-hooks.sh"
  - "crates/overlay-ui/build.rs"
  - "crates/overlay-ui/src/build_info.rs"
---

# versioning

Portée : numéro de version du produit, hook `post-commit` qui l'incrémente d'après le type du
commit, `build_info` et son gel pour le harnais de captures. Tout nouvel apprentissage sur ce sujet
se documente ici, jamais dans `CLAUDE.md`.

## Où vit la version

La version du produit vit **uniquement** dans `[workspace.package] version` (`Cargo.toml` racine) ;
les crates y renvoient par `version.workspace = true`, et `crates/overlay-ui/build.rs` y adjoint le
hash du commit compilé. Les deux se lisent dans `overlay_ui::build_info` et s'affichent dans la
bannière de la fenêtre Options (`0.1.0`) et au journal (`0.1.0 (a1b2c3d)`).

**Ne jamais éditer ce champ, ni celui des `Cargo.lock`.** Le hook `post-commit` (`.githooks/`,
installé par `scripts/install-hooks.sh` — à relancer en début de session cloud, `core.hooksPath` est
une config locale) s'en charge, d'après le type Conventional Commits du commit :

| Type | Niveau |
| --- | --- |
| `feat!:`/`fix!:`/… ou footer `BREAKING CHANGE:` | major |
| `feat:` | minor |
| `fix:`, `style:`, `perf:`, `refactor:`, `test:` | patch |
| `docs:`, `chore:`, `ci:`, `build:`, message non conforme | aucun |

Le hook amende le commit qu'on vient de créer pour y inclure le bump : **un commit porte donc sa
propre version**. Vérifier ce qu'il ferait sans rien écrire : `bash scripts/bump-version.sh
--dry-run`. Échappatoire ponctuelle : `SKIP_VERSION_BUMP=1 git commit …`.

Deux conséquences à garder en tête :

- Un changement visuel dans `overlay-ui` ne doit **jamais** faire dépendre une capture de la version
  réelle : `build_info::freeze_for_snapshots()` (appelée par `Textures::get_or_load` dans
  `tests/panels.rs`) la fige à `0.0.0` pour tout le harnais. Sans ce gel, le gate de captures
  virerait au rouge à chaque commit.
- Le hook s'abstient pendant un `rebase`/`merge`/`cherry-pick` et sur un commit de fusion. Si un
  bump manque après une manipulation d'historique, le rattraper par un commit ordinaire plutôt que
  d'écrire le numéro à la main.

## Le préfixe de commit pilote le bump — cas vécu

La table de choix du préfixe est dans `CLAUDE.md` (« Commits »), parce que chaque commit doit la
consulter. Ce qui suit est l'exemple qui l'a motivée.

Cas vécu (2026-09-15) : l'ajout de `wakfu-overlay.pub` (clé publique de vérification des mises
à jour) a été commité en `feat:` — le hook n'était pas actif dans cette session, la version est
donc restée à 0.20.4, mais avec le hook ce commit aurait déclenché un passage à 0.21.0 sans que
rien ne change pour l'utilisateur, puisque le fichier n'est encore embarqué nulle part. C'était
un `chore:`. Le jour où le code qui *utilise* cette clé arrive, ce commit-là est le `feat:`.
