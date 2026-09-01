# `overlay-sync` — L4 : auth native + roster (lecture)

Auth native par appairage (device pairing) + récupération en lecture seule du roster de
personnages du compte — voir [`docs/plan-architecture.md`](../../docs/plan-architecture.md) §7.2,
§12, §14 point 2. Première brique réseau de l'overlay : pas encore de file d'envoi ni de synchro
d'historique (L5, à venir).

## Contenu

| Module | Rôle |
| --- | --- |
| `pairing.rs` | `pair_and_wait(on_started)` : `POST /api/v1/auth/native/pair`, affiche le code (callback `on_started`, appelé avant tout sondage), ouvre le navigateur par défaut (`open`, best-effort), sonde `POST /api/v1/auth/native/poll` toutes les ~3 s jusqu'à confirmation/expiration. |
| `token_store.rs` | `save_token`/`load_token`/`clear_token` — trousseau OS (`keyring`) en priorité, repli fichier `0600` explicite (jamais silencieux, `tracing::warn!`) sous le dossier de données de l'app (`directories::ProjectDirs`) si aucun trousseau n'est disponible. |
| `client.rs` | `fetch_roster(token)` : `GET /api/v1/settings` avec `Authorization: Bearer <token>` → `overlay_engine::RosterIndex`. Origine configurable (`WAKFU_COMPANION_API_URL`, repli sur le domaine public). |

## Pourquoi `ureq` plutôt que `reqwest`+`tokio`

Ce crate ne fait que quelques requêtes ponctuelles (pairing, un `GET /settings`), toujours sur son
propre thread (`overlay-app`/`overlay-ui` le spawn dédié, jamais le thread Engine ni le main
thread) — jamais sur un chemin chaud. `reqwest`+`tokio` (rt `current_thread`) reste prévu pour la
vraie file d'envoi asynchrone de L5 ; l'introduire déjà ici aurait été une dépendance non justifiée
par le besoin réel de cette itération.

## Ce qui manque volontairement ici

- Pas d'UI d'appairage dans la fenêtre overlay — le code est affiché en console
  (`overlay-ui::spawn_auth_thread`). Le panneau "État de synchro" du plan (§9) le remplacera.
- Pas de révocation/déconnexion côté overlay (`token_store::clear_token` existe mais n'est appelé
  qu'en repli sur un jeton invalide, jamais sur demande utilisateur).
- Le mode invité (aucun compte lié, ou appairage jamais complété) n'est **jamais** une erreur :
  `overlay-engine::session` retombe entièrement sur `breed` sans roster, exactement comme le mode
  invité du web.

## Tests

La logique de classification classe/roster/breed est testée côté `overlay-engine` (roster pur,
sans réseau) — voir `crates/overlay-engine/README.md`. Ce crate n'a pour l'instant pas de test
réseau automatisé : la vérification réelle (pairing bout en bout contre l'API déployée) suit la
même contrainte que côté serveur (`wakfu-companion/CLAUDE.md`) — non joignable depuis ce sandbox,
à confirmer après déploiement.
