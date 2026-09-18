# Analyse RGPD — reste à faire côté overlay

Reliquat de [`analyse-rgpd.md`](analyse-rgpd.md) pour ce dépôt (`wakfu-companion-overlay`), établi
le 2026-09-18 après vérification dans le code. Les six constats principaux (C1 à C6) sont clos côté
overlay ; il ne reste que des constats secondaires, aucun n'est bloquant. Les deux autres volets :
[`analyse-rgpd-site.md`](analyse-rgpd-site.md) et
[`analyse-rgpd-mainteneur.md`](analyse-rgpd-mainteneur.md).

| Prio | Tâche | Constat | État vérifié le 2026-09-18 |
| --- | --- | --- | --- |
| P2 | **Jeton de repli fichier** : créer le fichier avec le mode restrictif dès l'ouverture (`OpenOptions::mode(0o600)`), ACL utilisateur ou DPAPI sous Windows, et avertir dans l'interface (pas seulement au journal) quand le trousseau est indisponible | C7 | `token_store.rs` : aucun `0o600`, `set_permissions` ni DPAPI |
| P2 | **Démarrage automatique** : ne plus l'activer d'office au premier lancement — le demander, ou laisser décoché (loyauté, art. 5.1.a) | C12 | `autostart::enable_by_default_once` inscrit toujours par défaut |
| P2 | **Unifier les deux racines `ProjectDirs`** (`("com", "Oumbra", …)` vs `("", "", …)`) avec migration au démarrage ; l'effacement de C5 les liste toutes les deux en attendant | C13 | 1 appel sur `com/Oumbra`, 10 sur la racine vide |
| P2 | **Icônes** : les servir depuis l'API ou les embarquer, pour ne plus envoyer l'IP à `vertylo.github.io`. La mention du destinataire dans « À propos » est faite ; c'est la partie technique qui reste | C10 | `catalog.rs`, `spells.rs` chargent encore depuis `vertylo` |
| P3 | **`overlay-app`** : masquer les entrées `Chat` en console, ou sortir ce binaire du workspace livré | C14 | non traité |
| P3 | **Origine d'API surchargée** (`WAKFU_COMPANION_API_URL`) : `warn!` au démarrage et affichage de l'origine dans « À propos » quand elle diffère du défaut ; refuser `http://` pour `WAKFU_OVERLAY_UPDATE_URL` | C16 | `client.rs:37` lit la variable sans rien signaler |
| P3 | **Écran de connexion** : mettre le détail technique d'erreur (chemin d'API, message) derrière un « Détails » repliable | C17 | non traité |
| P3 | Préciser dans l'interface si la case `auto_update` coupe aussi la *vérification* GitHub (sinon le faire) | C11 | à confirmer dans `background.rs` |
| — | Optionnel, écarté le 2026-09-18 : `--purge-local-data` en ligne de commande ; à rouvrir seulement si un désinstalleur arrive | C5 | — |

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
