# Analyse RGPD — reste à faire côté overlay

Reliquat de [`analyse-rgpd.md`](analyse-rgpd.md) pour ce dépôt (`wakfu-companion-overlay`), établi
le 2026-09-18 après vérification dans le code, mis à jour le 2026-09-19. Les six constats principaux
(C1 à C6) sont clos côté overlay ; des constats secondaires, il ne reste que C12. Les deux autres volets :
[`analyse-rgpd-site.md`](analyse-rgpd-site.md) et
[`analyse-rgpd-mainteneur.md`](analyse-rgpd-mainteneur.md).

| Prio | Tâche | Constat | État |
| --- | --- | --- | --- |
| P2 | **Démarrage automatique** : ne plus l'activer d'office au premier lancement — le demander, ou laisser décoché (loyauté, art. 5.1.a) | C12 | `autostart::enable_by_default_once` inscrit toujours par défaut (vérifié le 2026-09-19) |
| — | Optionnel, écarté le 2026-09-18 : `--purge-local-data` en ligne de commande ; à rouvrir seulement si un désinstalleur arrive | C5 | — |

## Fait le 2026-09-19 (décisions du mainteneur)

| Constat | Ce qui a été fait | Où |
| --- | --- | --- |
| C7 | Fichier de repli du jeton créé en `0600` (`OpenOptions::mode`, plus de fenêtre entre écriture et `set_permissions`) ; avis en rouge dans la section « Compte » de la fenêtre Options tant que le fichier existe, avec son chemin. Pas de DPAPI (choix : `%APPDATA%` est déjà réservé au compte par l'ACL du profil) | `overlay-sync/src/token_store.rs`, `panels/options_modal.rs::token_file_notice` |
| C10 | Icônes servies par l'API : `GET /api/v1/icons/{folder}/{gfxId}.png` relaie `wakassets` avec cache de périphérie (`wakfu-companion` `a51ee02`, déployé sur `claude-dev`, vérifié : 200 `image/png`, 404 amont relayé, 400 hors liste) ; l'overlay construit ses URL par `overlay_sync::client::icon_url`, `IconRef::image_paths` ne porte plus que des chemins relatifs. « À propos » et la politique du site (1.4, 2, 4, `a5204e1`) ne nomment plus que GitHub | `overlay-engine/src/catalog.rs`, `overlay-sync/src/client.rs`, `overlay-ui/src/remote_icons.rs` |
| C13 | Racine unique `("", "", "wakfu-companion-overlay")` — décision du mainteneur, « com »/« Oumbra » ne venaient de nulle part. `overlay_engine::app_dirs` est le seul endroit qui appelle `ProjectDirs::from` (un test parcourt les sources pour y veiller) ; `config::migrate_legacy_root` déplace `config.toml` et `turn-templates/` depuis l'ancienne racine au démarrage puis la supprime ; l'effacement complet la liste encore au cas où | `overlay-engine/src/app_dirs.rs`, `overlay-ui/src/config.rs`, `overlay-ui/src/local_data.rs` |
| C14 | `overlay-app` supprimé du workspace, du CI, de `ci-local.sh` et du README ; les commentaires qui le citaient renvoient à `overlay-ui` | commit `8adb698` |
| C16 | `WAKFU_OVERLAY_UPDATE_URL` : seul `https://` est accepté, ou `http://` vers `127.0.0.1`/`localhost`/`[::1]` (le rejeu local de `xtask dist` reste possible) ; valeur refusée ignorée avec un `warn!`. `WAKFU_COMPANION_API_URL` : bloc d'alerte sous « Vos données » dans « À propos », nommant l'origine réelle | `overlay-sync/src/update/mod.rs::override_allowed`, `panels/a_propos_tab.rs::api_override_notice` |
| C17 | Bouton « Copier le détail » à côté de « Réessayer » sur l'écran d'erreur de connexion (titre + détail complet dans le presse-papiers) — décision : pas de bloc repliable | `panels/login.rs::failure_clipboard_text` |
| C11 | Rien à changer : la case ne coupe que l'installation, la vérification est annoncée « à chaque lancement » dans « À propos » et la politique | — |

## Ce qui est clos côté overlay (pour mémoire)

- C1 : fixture pseudonymisée, `.gitignore`, `scripts/check-fixtures.sh` branché au `pre-commit`,
  au CI et à `ci-local.sh` — reste la réécriture d'historique (mainteneur).
- C2 : §10 du plan d'architecture réécrit.
- C3 : autocomplétion retirée, `fight-*.json` purgés à la fermeture du jeu, file de synchro vidée à
  la déconnexion et sans écriture hors compte.
- C4 : section « Vos données » dans « À propos », ligne d'acceptation sous « Se connecter »,
  licence MIT.
- C5 : purge à la déconnexion, bouton « Supprimer les données locales », appel de
  `DELETE /api/v1/auth/native/session` (dont le déploiement dépend du site).
- C6 : plus rien de personnel au niveau `info`, plafond de 16 Mio/jour, « Journal détaillé »
  décoché par défaut, journaux vidés à la déconnexion.
- C8 : `turn-templates/` effacé à la déconnexion et par le bouton.
- C7, C10, C11, C13, C14, C16, C17 : voir le tableau du 2026-09-19 ci-dessus.
