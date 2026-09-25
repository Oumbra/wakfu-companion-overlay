# Analyse RGPD — reste à faire côté overlay

Reliquat de l'analyse de conformité RGPD menée sur ce dépôt (`wakfu-companion-overlay`), établi
le 2026-09-18 après vérification dans le code, mis à jour le 2026-09-19 et revérifié le 2026-09-21
(`f443e2a`, commits du 19 au 21 : pactes, `overlay_sync::session`, jeton par déploiement).
**Plus rien à faire côté overlay** : les six constats principaux (C1 à C6) et tous les constats
secondaires, C18 et C19 compris, sont clos. L'analyse complète et la fiche des gestes du mainteneur
(réécriture d'historique, ticket GitHub Support, notes internes) ont été retirées du dépôt le
2026-09-21, tous leurs points étant clos ; le volet du site vit dans
[`docs/analyse-rgpd.md`](https://github.com/Oumbra/wakfu-companion/blob/claude/dev/docs/analyse-rgpd.md)
du dépôt `wakfu-companion` (section 8 pour le reliquat issu de l'overlay).

| Prio | Tâche | Constat | État |
| --- | --- | --- | --- |
| — | Optionnel, écarté le 2026-09-18 : `--purge-local-data` en ligne de commande ; à rouvrir seulement si un désinstalleur arrive | C5 | — |

## Fait le 2026-09-25 (audit des textes contre le code)

Relecture de chaque phrase de « À propos », des écrans de connexion et de déconnexion, du README
et des analyses face au code (`45e0828`, version 0.82.10). Constats numérotés C20 à C25.

| Constat | Ce qui a été fait | Où |
| --- | --- | --- |
| C20 | Titre de la fenêtre au premier plan (n'importe quelle application) écrit toutes les 3 s au niveau `info` tant qu'un overlay est rétrogradé ; nom du partenaire d'Inviter/Suivre en `info`/`warn`. Passés en `debug` (« Journal détaillé »), dont l'infobulle le dit | `main.rs::sync_topmost`, `main.rs`/`bin/…-x11.rs` (raccourcis multicompte), `chat_command.rs::send`, `panels/options_modal.rs` |
| C21 | « Avant l'appairage, rien ne part » était faux : `GET /api/v1/game-servers` part sans authentification à chaque lancement, et l'historique relu dans `wakfu.log` est envoyé au moment de l'appairage. La liste de ce qui part omettait le Suivi, les ventes récupérées à l'HDV et le détail des combats (soins, armure, sorts, butin, XP, kamas, serveur). Texte réécrit, test étendu | `panels/a_propos_tab.rs::SECTIONS`, README « Vos données » |
| C22 | La déconnexion vide la file d'envoi (`SyncCommand::Deactivate`) : l'historique pas encore envoyé est perdu, et aucun texte ne le disait. Constante partagée `DISCONNECT_INFO`, affichée dans la section « Compte » (Options et Carte) et dans la confirmation de la Carte | `a_propos_tab::DISCONNECT_INFO`, `options_modal.rs`, `login.rs` |
| C23 | « Journal de 14 jours » : ce sont 14 fichiers quotidiens (« vos 14 derniers jours d'utilisation »). Le plafond de 16 Mio repartait de zéro à chaque lancement : il part désormais de la taille du fichier du jour | `a_propos_tab.rs`, `logging.rs::todays_log_size` |
| C24 | « Supprimer les données locales » : la liste omettait l'inscription au démarrage et les clés de registre ; le dossier `%APPDATA%\wakfu-companion-overlay` restait vide derrière la purge. Surtout, l'ancienne racine (C13) était visée par `ProjectDirs::project_path()`, un fragment RELATIF : ni la migration ni la purge ne la trouvaient. Chemins absolus (`app_dirs::own_root`), dossier parent purgé | `overlay-engine/src/app_dirs.rs`, `config.rs::{legacy_root, own_root, migrate_legacy_root}`, `local_data.rs` |
| C25 | « Deux fonctions optionnelles » pour trois fonctions, dont F1/F2 non désactivables. Case « Activer les raccourcis multicompte » (onglet « Raccourcis », actifs par défaut) ; « Trois fonctions » dans « À propos » et le README | `shortcuts.rs`, `config.rs::multiaccount_shortcuts`, `panels/raccourcis_tab.rs` |

**Reste à confirmer par le mainteneur** : C5 (rotation du jeton) et C9 (`PATCH` partiel) dépendaient
du déploiement du site (`claude/dev` → `main` de `wakfu-companion`), et la Release v0.82.10 est
sortie le 2026-09-24. Le dépôt du site n'était pas accessible depuis la session du 25 : vérifier que
le site est déployé, puis retirer les mentions « dépend du déploiement » ci-dessous et faire pointer
le lien de l'en-tête vers `main`.

## Fait le 2026-09-21 (revérification)

| Constat | Ce qui a été fait | Où |
| --- | --- | --- |
| C18 | « vos extractions de pacte » ajouté à la liste de ce qui part au compte dans « Vos données » (flux `POST /api/v1/history/pacts` du 2026-09-19, que la liste — lue comme exhaustive — ne citait pas) ; un test vérifie que les quatre types de `HistoryEventKind` y sont nommés. Même ajout dans la politique §1.4 du site (`wakfu-companion`, quatre langues) | `panels/a_propos_tab.rs::SECTIONS` |
| C19 | `token_store::clear_all_tokens` : l'effacement complet vise tous les emplacements de jeton (prod, dev, exe courant, et tout `<slot>.token`/`<slot>.issued-at` présent dans le dossier de données), plus seulement celui de l'exe qui l'appelle — une entrée de trousseau posée par un autre déploiement survivait à « Supprimer les données locales » depuis `ec43ca6` | `overlay-sync/src/token_store.rs`, `overlay-ui/src/local_data.rs::purge` |
| — | Analyse complète réalignée (route `pacts` au §2.2, emplacements par déploiement au §2.3, jeton en mémoire (`overlay_sync::session`) au §3.5), puis retirée du dépôt le 2026-09-21 | `efcb787:docs/analyse-rgpd.md` |

## Fait le 2026-09-19 (décisions du mainteneur)

| Constat | Ce qui a été fait | Où |
| --- | --- | --- |
| C7 | Fichier de repli du jeton créé en `0600` (`OpenOptions::mode`, plus de fenêtre entre écriture et `set_permissions`) ; avis en rouge dans la section « Compte » de la fenêtre Options tant que le fichier existe, avec son chemin. Pas de DPAPI (choix : `%APPDATA%` est déjà réservé au compte par l'ACL du profil) | `overlay-sync/src/token_store.rs`, `panels/options_modal.rs::token_file_notice` |
| C10 | Icônes servies par l'API : `GET /api/v1/icons/{folder}/{gfxId}.png` relaie `wakassets` avec cache de périphérie (`wakfu-companion` `a51ee02`, déployé sur `claude-dev`, vérifié : 200 `image/png`, 404 amont relayé, 400 hors liste) ; l'overlay construit ses URL par `overlay_sync::client::icon_url`, `IconRef::image_paths` ne porte plus que des chemins relatifs. « À propos » et la politique du site (1.4, 2, 4, `a5204e1`) ne nomment plus que GitHub | `overlay-engine/src/catalog.rs`, `overlay-sync/src/client.rs`, `overlay-ui/src/remote_icons.rs` |
| C13 | Racine unique `("", "", "wakfu-companion-overlay")` — décision du mainteneur, « com »/« Oumbra » ne venaient de nulle part. `overlay_engine::app_dirs` est le seul endroit qui appelle `ProjectDirs::from` (un test parcourt les sources pour y veiller) ; `config::migrate_legacy_root` déplace `config.toml` et `turn-templates/` depuis l'ancienne racine au démarrage puis la supprime ; l'effacement complet la liste encore au cas où | `overlay-engine/src/app_dirs.rs`, `overlay-ui/src/config.rs`, `overlay-ui/src/local_data.rs` |
| C14 | `overlay-app` supprimé du workspace, du CI, de `ci-local.sh` et du README ; les commentaires qui le citaient renvoient à `overlay-ui` | commit `2103fa8` |
| C16 | `WAKFU_OVERLAY_UPDATE_URL` : seul `https://` est accepté, ou `http://` vers `127.0.0.1`/`localhost`/`[::1]` (le rejeu local de `xtask dist` reste possible) ; valeur refusée ignorée avec un `warn!`. `WAKFU_COMPANION_API_URL` : bloc d'alerte sous « Vos données » dans « À propos », nommant l'origine réelle | `overlay-sync/src/update/mod.rs::override_allowed`, `panels/a_propos_tab.rs::api_override_notice` |
| C17 | Bouton « Copier le détail » à côté de « Réessayer » sur l'écran d'erreur de connexion (titre + détail complet dans le presse-papiers) — décision : pas de bloc repliable | `panels/login.rs::failure_clipboard_text` |
| C11 | Rien à changer : la case ne coupe que l'installation, la vérification est annoncée « à chaque lancement » dans « À propos » et la politique | — |
| C5 | **Rotation du jeton natif appelée** : au démarrage, jeton accepté et file d'envoi pas encore activée, `POST /api/v1/auth/native/session` s'il a plus de 7 jours ou si sa date d'émission est inconnue ; nouveau jeton écrit au trousseau avant d'être utilisé, échec = ancien jeton conservé, jamais une déconnexion. Date d'émission dans `native-session.issued-at` (posée par `save_token`, effacée par `clear_token`). **Dépend du déploiement du site** : tant que `claude/dev` n'est pas fusionné, la prod répond 404/405 et l'overlay garde son jeton | `overlay-ui/src/background.rs::rotate_token_if_due`, `overlay-sync/src/client.rs::rotate_native_session`, `token_store.rs::token_age` |
| C9 | **Écriture partielle** de `PATCH /api/v1/settings` : `profile` ne part plus qu'avec ses trois champs d'alerte (`patch`, fusion côté serveur), `roster` avec les seuls comptes modifiés ou créés et les `removedIds` (écart avec le roster connu, `Roster::patch_against`) ; `profile_raw` et `RosterAccount::extra` retirés, le pseudo et l'avatar ne transitent plus par l'overlay ; un refus (`rejected`) est journalisé, la version du compte l'emporte. **Dépend du déploiement du site** : un serveur sans `patch` répond 400 « valeur manquante » — ne pas livrer en Release avant la fusion `claude/dev` → `main` | `overlay-engine/src/profile.rs::patch_fields`, `roster.rs::patch_against`, `overlay-sync/src/client.rs::patch_settings` |
| C12 | Démarrage automatique **décoché par défaut** : `autostart::enable_by_default_once` et le jalon `config.toml::autostart_initialized` retirés, seule la case de la fenêtre Options inscrit l'overlay. Une inscription posée d'office par les versions du 16 au 19 reste en place (la retirer d'office serait le même geste à l'envers), la case la montre et la retire | `overlay-ui/src/autostart.rs`, `overlay-ui/src/config.rs`, §9.1 novodecies du plan |

## Ce qui est clos côté overlay (pour mémoire)

- C1 : fixture pseudonymisée, `.gitignore`, `scripts/check-fixtures.sh` branché au `pre-commit`,
  au CI et à `ci-local.sh` ; historique réécrit le 2026-09-19 — reste le ticket GitHub Support
  (mainteneur).
- C2 : §10 du plan d'architecture réécrit.
- C3 : autocomplétion retirée, `fight-*.json` purgés à la fermeture du jeu, file de synchro vidée à
  la déconnexion et sans écriture hors compte.
- C4 : section « Vos données » dans « À propos », ligne d'acceptation sous « Se connecter »,
  licence MIT.
- C5 : purge à la déconnexion, bouton « Supprimer les données locales », appel de
  `DELETE /api/v1/auth/native/session` et rotation du jeton au démarrage passé 7 jours (dont le
  déploiement dépend du site).
- C6 : plus rien de personnel au niveau `info`, plafond de 16 Mio/jour, « Journal détaillé »
  décoché par défaut, journaux vidés à la déconnexion.
- C8 : `turn-templates/` effacé à la déconnexion et par le bouton.
- C7, C9, C10, C11, C12, C13, C14, C16, C17 : voir le tableau du 2026-09-19 ci-dessus.
