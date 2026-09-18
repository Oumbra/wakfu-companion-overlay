# Analyse de conformité RGPD — `wakfu-companion-overlay`

Analyse statique du code de l'overlay (version `0.63.1`, branche `dev` au 2026-09-18), menée
crate par crate : `overlay-ingest`, `overlay-engine` (dont le parseur TS vendu, `engine-js/`),
`overlay-sync`, `overlay-ui`, `overlay-platform`, `overlay-app`, plus les fixtures de test, les
spikes et la documentation. Chaque constat renvoie à un `fichier:ligne` vérifié à la main.

**Ce document n'est pas un avis juridique.** Il identifie les traitements de données à caractère
personnel que le code réalise, les confronte aux principes du RGPD (art. 5), à la licéité
(art. 6), à l'information des personnes (art. 12-14), aux droits (art. 15-17), à la protection
dès la conception (art. 25) et à la sécurité (art. 32), et propose un plan d'action. La
qualification finale (base légale retenue, contenu de la politique de confidentialité) revient
au responsable de traitement.

## 0. Résumé exécutif

L'overlay est, au sens du RGPD, un **client** du service `wakfu-companion.com` : le responsable de
traitement est l'exploitant du service (le mainteneur), et l'overlay est l'un des moyens de
collecte. Il ne manipule ni email, ni mot de passe, ni identifiant matériel, n'embarque aucune
télémétrie ni rapport de crash, et n'envoie **rien** tant qu'aucun compte n'est lié. Ces points
sont bons et sont à consigner (§5).

Six constats appellent une action, du plus urgent au moins urgent :

| # | Constat | Gravité | Réf. |
| --- | --- | --- | --- |
| C1 | Un **vrai `wakfu.log` non anonymisé** est versionné en double dans un **dépôt public** : jeton d'authentification du client de jeu, IP locale, nom de compte Windows, 6 personnages avec leurs identifiants numériques, **119 pseudonymes de tiers et l'intégralité de leurs messages de chat** — *fixture pseudonymisée le 2026-09-18 ; reste la réécriture d'historique, décision du mainteneur* | **Critique** | §3.1 |
| C2 | La doctrine « vie privée » du plan d'architecture (« jamais de capture d'écran, jamais d'automatisation d'entrées ») est **contredite par le code** : capture de la fenêtre de jeu toutes les 500 ms en combat, frappes clavier et clic souris synthétiques | Élevée | §3.2 |
| C3 | Les **pseudonymes d'autres joueurs** (coéquipiers, partenaire d'échange) sont transmis au serveur avec leurs performances, persistés en clair sur disque, et proposés à l'autocomplétion du roster — sans information ni moyen d'opposition pour ces tiers — *décision du 2026-09-18 : noms conservés (option A), autocomplétion retirée, persistance locale bornée ; reste la politique de confidentialité côté site* | Élevée | §3.3 |
| C4 | **Aucune information** sur le traitement dans l'application : pas de lien vers une politique de confidentialité, pas de mention à l'écran de connexion ni dans « À propos » — *section « Vos données » et liens ajoutés dans « À propos » le 2026-09-18 au soir ; restent l'écran de connexion et la licence* | Élevée | §3.4 |
| C5 | **Aucun effacement exerçable** : la déconnexion n'efface que le jeton ; combats, file d'envoi, gabarits d'image, journaux et configuration restent | Élevée | §3.5 |
| C6 | Le **journal applicatif** (14 jours, niveau `info`, pas de plafond) contient des noms de personnages, des auteurs de messages tiers, le code d'appairage et le nom d'utilisateur OS ; une erreur de désérialisation y recopierait un lot entier, chat compris | Moyenne | §3.6 |

Les constats secondaires (C7 à C17) sont au §3.7. Le plan d'action priorisé est au §6.

## 1. Rôles et périmètre

- **Responsable de traitement** : l'exploitant de `wakfu-companion.com` (API prod,
  `crates/overlay-sync/build.rs:24`) et de son environnement de développement
  `claude-dev.wakfu-companion.com` (`build.rs:25`, utilisé par tout binaire non-`release`,
  `build.rs:33-38`). L'overlay envoie les mêmes données aux deux selon le profil de compilation :
  l'analyse vaut pour les deux environnements.
- **Destinataires tiers** (à nommer dans l'information des personnes, art. 13.1.e) :
  - **GitHub** (`github.com/Oumbra/wakfu-companion-overlay/releases`,
    `crates/overlay-sync/src/update/mod.rs:41`) : vérification de mise à jour **à chaque
    démarrage** (`crates/overlay-ui/src/background.rs:731+`), donc IP + horodatage + User-Agent
    `ureq/3.x` par lancement. Aucun identifiant ni version envoyés (comparaison locale,
    `update/manifest.rs:104-139`).
  - **`vertylo.github.io`** (GitHub Pages d'un particulier, `crates/overlay-engine/src/catalog.rs:85,108`,
    `spells.rs:312`) : téléchargement des icônes d'objets, monstres et sorts
    (`crates/overlay-ui/src/remote_icons.rs:238-250`). Ce tiers voit l'IP de l'utilisateur et la
    liste des objets/monstres/sorts affichés.
  - **Ankama** n'est pas un destinataire : l'overlay ne contacte jamais les serveurs du jeu.
- **Personnes concernées** : l'utilisateur de l'overlay (titulaire du compte), et **des tiers** :
  les autres joueurs présents dans `wakfu.log` (coéquipiers, adversaires joueurs, partenaires
  d'échange, auteurs de messages sur les canaux publics).
- **Hors périmètre de ce document** : le serveur et le site (`Oumbra/wakfu-companion`) —
  durées de conservation côté serveur, politique de confidentialité, suppression de compte,
  page « Sessions actives ». Plusieurs recommandations ci-dessous n'ont de sens qu'avec un
  pendant côté serveur ; elles sont marquées « (serveur) ».

## 2. Cartographie des traitements (base d'un registre, art. 30)

### 2.1 Données lues dans `wakfu.log`

Un seul fichier est ouvert en lecture : `wakfu.log` (`crates/overlay-ingest/src/tailer.rs:76`).
Le découpage n'interprète rien (`LineBatch`, `tailer.rs:23-31`) ; l'extraction est faite par le
parseur TS vendu (`crates/overlay-engine/engine-js/src/log-parser.ts`).

| Donnée | Personne | Extraction | Struct Rust |
| --- | --- | --- | --- |
| Nom de personnage + identifiant numérique + classe + drapeau IA | Utilisateur **et tiers** | `FIGHTER_JOIN_RE`, `log-parser.ts:205-206` | `LogEntry::FighterJoined`, `model.rs:183-195` |
| Lanceur de sort, cible de dégâts/soins/armure | Utilisateur et tiers | `log-parser.ts:58, 207, 211` | `SpellCast`, `Damage`, `Heal`, `Armor` |
| Nom + XP gagnée | Utilisateur et tiers | `XP_RE`, `log-parser.ts:57` | `XpGain` |
| Nom du joueur en défaite | Utilisateur | `OCCUPATION_RE`, `log-parser.ts:154` | `CombatDefeatMarker` |
| **Auteur + contenu intégral d'un message de chat** (6 canaux publics) | **Tiers** | `CHAT_CONTENT_RE`, `log-parser.ts:51`, canaux `:10-32` | `LogEntry::Chat`, `model.rs:58-65` |
| **Nom des deux joueurs d'un échange**, kamas et objets | Utilisateur et **tiers** | `TRADE_DONNE_RE`, `log-parser.ts:223-225` | `TradeCompleted`/`TradeSide`, `model.rs:45-51, 196-199` |
| Kamas gagnés/perdus, butin, challenges | Utilisateur | `log-parser.ts:52-54` | `KamaGain`, `KamaLoss`, `Loot` |

Non extraits : position/carte (`FIGHTER_JOIN_RE` s'arrête avant `{Point3 …}`), messages privés
(aucun canal privé dans `resolveChatChannel`), nom de guilde en tant que tel (mais le canal
Guilde révèle l'appartenance), nom de serveur (déduit du roster du compte,
`crates/overlay-engine/src/roster.rs:56-81`).

### 2.2 Données envoyées à l'API

Tout le réseau du produit passe par `overlay-sync` (`ureq` + `rustls`, aucune désactivation TLS).
Payloads dans `crates/overlay-engine/src/history.rs`.

| Route | Contenu à caractère personnel | Réf. |
| --- | --- | --- |
| `POST /api/v1/history/fights` | `participants[].name` : **nom de chaque combattant, alliés compris** (donc d'autres joueurs), avec classe, dégâts, soins, sorts, KO, fuite ; `gameServer` | `history.rs:104-128, 138-179` ; construit `session.rs:1862-1911` |
| `POST /api/v1/history/trades` | **`peerName`** (partenaire d'échange), `selfName`, kamas, objets, `gameServer` | `history.rs:208-218` ; `session.rs:2036-2044` |
| `POST /api/v1/history/purchases` | Objet, quantité, coût, `gameServer` | `history.rs:181-190` |
| `PATCH /api/v1/settings` clé `roster` | Noms de personnages de l'utilisateur, classe, genre, `id`/`label` de compte, serveur ; **plus tout champ inconnu réémis tel quel** (`#[serde(flatten)] extra`) | `roster.rs:49-91, 271-277` |
| `PATCH /api/v1/settings` clé `profile` | **Objet `profile` entier reçu du compte, renvoyé tel quel** (pseudo, avatar) pour un simple réglage d'alerte | `crates/overlay-engine/src/profile.rs:278-316` ; `client.rs:129-141` |
| `PATCH /api/v1/settings` clé `chatFilters` | Mots-clés de recherche saisis par l'utilisateur (peuvent être un pseudo de tiers, cf. test `chat_alert.rs:302-305`) | `chat_alert.rs:74-79, 176-182` |
| `PATCH /api/v1/settings` clé `watchlist` | Compteurs par objet/monstre — non personnel | `watchlist.rs:96-106` |
| `clientKey`, `dungeonRunKey` | `sha256(uid\|kind\|signature)` : la signature contient les noms normalisés, mais la clé envoyée est un haché | `queue.rs:49-57, 255-276` |

Non envoyés (vérifié) : contenu des messages de chat, mot de passe, email, IP applicative,
identifiant machine, nom d'utilisateur OS, chemins, version d'OS, lignes brutes du log.

### 2.3 Données écrites sur disque

Deux racines `ProjectDirs` distinctes coexistent, ce qui **sépare les fichiers en deux arbres sous
Windows** (`%APPDATA%\wakfu-companion-overlay\data\` et `%APPDATA%\Oumbra\wakfu-companion-overlay\`) :
`ProjectDirs::from("", "", …)` pour le jeton, les caches, la file, les combats, les journaux ;
`ProjectDirs::from("com", "Oumbra", …)` pour `config.toml` (`config.rs:601`) et les gabarits de
tour (`turn_watch/templates.rs:19`, dont le commentaire `:17` affirme à tort « même racine que
`logs/` »). Sous Linux, les deux se confondent.

| Fichier | Contenu personnel | Chiffré | Purge | Effacé à la déconnexion |
| --- | --- | --- | --- | --- |
| `config.toml` (`config.rs:604`) | `log_path` (contient souvent le nom d'utilisateur OS) | Non | — | Non |
| Trousseau OS `wakfu-companion-overlay/native-session` (`token_store.rs:16-23`) | Jeton de session API | Oui (Credential Manager / Secret Service) | — | Oui |
| `native-session.token` (`token_store.rs:26-49`), repli si le trousseau échoue | **Jeton en clair** ; `0600` posé après écriture, Unix seulement, **aucune ACL sous Windows** | Non | — | Oui |
| `logs/overlay-ui.<date>.log` (`logging.rs:64-76`) | Voir §3.6 | Non | 14 jours, **sans plafond de taille** | Non |
| `data/fight-<id>.json` (`fight_store.rs:47-76`) | `FightSnapshot` : **noms de tous les combattants, tiers compris**, classe, genre, sorts | Non | Fin de combat, ou 24 h **au démarrage suivant seulement** | Non |
| `sync-queue.sqlite3` (`queue.rs:118-159`) | `payload_json` **en clair** : participants, `peerName`, objets, kamas | Non | Après envoi réussi ou 10 rejets ; **jamais si hors ligne ou déconnecté** (`background.rs:291-294`) | **Non** |
| `data/recap-session.json` (`recap_session.rs:84, 449-455`) | Horodatages de session, kamas/XP (habitudes de jeu) | Non | — | Non |
| `watchlist-counts.json` (`watchlist.rs:119-123`) | Compteurs par objet — non personnel | Non | Jamais | Non |
| `turn-templates/<nom>.png` + `<nom>.name` (`templates.rs:77-95`) | **Image de pixels du nom du personnage capturée à l'écran** + nom en clair | Non | Jamais (« à la main », `templates.rs:7`) | Non |
| `focus.log` (`turn_watch/notify.rs:265-278`) | Horodatages + HWND | Non | **Aucune rotation, aucun plafond** | Non |
| Caches catalogue/référentiels/icônes, `updates/` | Non personnel | Non | Jamais | Non |
| Autostart : `~/.config/autostart/*.desktop` ou `HKCU\…\Run` (`autostart.rs`) | Chemin absolu de l'exécutable | — | — | Non |

### 2.4 Autres traitements

- **Capture de la fenêtre de jeu** (Windows, option `turn_notification`, décochée par défaut) :
  `PrintWindow` sur toute la zone cliente toutes les 500 ms pendant un combat
  (`turn_watch/capture.rs:50-60`, `main.rs:155, 1484-1487`), copie mémoire des 400 dernières
  lignes, comparaison de gabarits sans OCR (`vision.rs:15-17`). Rien ne part sur le réseau ; le
  sous-rectangle du nom est écrit sur disque (ligne ci-dessus).
- **Entrées synthétiques** : `SendInput` / XTEST pour taper `/i`, `/fol`, `/w "<auteur>"` dans la
  fenêtre de jeu (`chat_command.rs`, `overlay-platform/src/linux/keyboard.rs`), déplacement de
  curseur et clic synthétique pour reprendre le premier plan (`notify.rs:435-460`). Aucune frappe
  n'est lue ni enregistrée.
- **Raccourcis globaux** via `RegisterHotKey`/`XGrabKey` (`shortcuts.rs`) : seules les
  combinaisons enregistrées remontent, pas de hook clavier bas niveau.
- **Presse-papiers** : écriture seule (« Copier le code », `login.rs:574`), jamais lu.
- **Énumération de dossiers personnels** pour trouver le log : `~/.steam/…/compatdata/*` et
  `~/.wine/drive_c/users/*` (`discovery.rs:72-100`), métadonnées seulement.
- **Journal du binaire de mise au point `overlay-app`** : recopie les 3 premières lignes brutes
  de chaque lot (`overlay-app/src/main.rs:43-55`), console seulement, non livré.

## 3. Constats détaillés

### 3.1 C1 — Fixture de test réelle, non anonymisée, dans un dépôt public (critique)

**Faits vérifiés.** `crates/overlay-engine/tests/wakfu.log` (10 975 lignes, journal du client
Wakfu du 2026-08-04) et sa copie octet pour octet `spikes/s2-engine-quickjs/tests/wakfu.log` sont
versionnés, présents sur `dev` et `main`, dans un dépôt **public depuis le 2026-09-15**
(`CLAUDE.md`). Le fichier est assumé comme réel par `tests/session_real_log.rs:1-4`. Il contient :

- ligne 1 : le nom de compte Windows de l'utilisateur (`C:\Users\<prénom>\AppData\…`) ;
- ligne 427 : `Authentication token received from dispatch server : <UUID>` — un **jeton
  d'authentification du client de jeu** (session Ankama, vraisemblablement expirée, mais un
  secret n'a pas sa place dans un dépôt) ;
- lignes 434, 469, 471 : l'**adresse IP locale** de la machine et l'hôte du serveur de chat ;
- 6 personnages de l'utilisateur avec leurs **identifiants numériques** de personnage, et des
  identifiants numériques d'autres joueurs (`[NATION] Trying to remove 90001001 …`) ;
- **119 pseudonymes distincts d'autres joueurs** sur les canaux Commerce, Communauté, Recrutement
  et Proximité, avec **le texte intégral de leurs messages** (propositions commerciales,
  questions, annonces de guilde avec mention Discord…) ;
- **43 lignes de liste d'amis** — `[Information (jeu)] <Personnage> (<compte>#<NNNN>) a rejoint
  notre monde.` / `vient de quitter notre monde.` : **12 identifiants de compte Ankama** distincts
  et les 16 personnages qui vont avec. *Manqué par la première passe de cette analyse, trouvé le
  2026-09-18 pendant la pseudonymisation.* C'est la donnée la plus identifiante du fichier : un
  `compte#NNNN` n'est pas un pseudonyme de personnage mais l'**identifiant global** de la personne
  chez Ankama, stable d'un personnage à l'autre et le même jusque sur les forums. Ces 12 personnes
  sont de surcroît, par construction, les *amis* de l'utilisateur — la liste publie donc aussi un
  lien social ;
- une ligne d'**échange** (ligne 9517, `[Trade] Ending the exchange between <nom> (id=…) and <nom>
  (id=…)`), également manquée par la première passe, avec un septième personnage de l'utilisateur
  qui n'apparaît dans aucun combat ;
- les coordonnées `Point3` de tous les combattants, kamas, XP, butin.

Des noms réels sont en outre recopiés en dur dans le code de test : pseudo et message d'un tiers
dans `tests/chat_alert_live.rs:47-48`, lignes `[_FL_]` réelles avec identifiant numérique dans
`tests/combat_orphelin.rs:16-32`, personnage tiers dans `src/session.rs:3573`, extrait réel dans
`tests/watchlist_boss_sans_ligne_ko.log`.

> **Leçon de méthode.** Les deux constats ajoutés ci-dessus ont été trouvés en cherchant les
> *identifiants numériques restants* et les *formes de ligne distinctes*, pas en relisant le
> fichier : un journal de 11 000 lignes ne se relit pas, et l'inventaire « à l'œil » d'une première
> passe rate ce qui n'apparaît que quarante fois sur onze mille. C'est la raison d'être de
> `scripts/check-fixtures.sh` — une règle par catégorie, vérifiable, plutôt qu'une lecture.

**Analyse.** Un pseudonyme de jeu associé à des propos, à un serveur et à un horodatage est une
donnée à caractère personnel (identifiant indirect, art. 4.1). Sa publication dans un dépôt
public, sans finalité de traitement légitime vis-à-vis de ces 119 personnes, contrevient à la
minimisation (art. 5.1.c), à la limitation des finalités (art. 5.1.b) et à l'intégrité et
confidentialité (art. 5.1.f). Le jeton et l'IP relèvent en plus de la sécurité (art. 32). Selon
la lecture retenue, la mise en public du 2026-09-15 peut constituer une violation de données
(art. 4.12) à documenter en interne (art. 33.5).

**Recommandations, et ce qui a été fait.**

1. ✅ **Fait le 2026-09-18** — fixture remplacée par une version **pseudonymisée de façon
   déterministe**, dans les deux copies : jeton en `00000000-0000-0000-0000-000000000000`, IP
   locale en `192.0.2.10` (plage de documentation, RFC 5737), nom de compte Windows en
   `anonymous`, modèle d'écran neutralisé, 7 personnages du compte en `Anonyme-<Classe><N>`
   (`Anonyme-Ouginak1`…) avec des identifiants numériques en `9000_000x`, 20 identifiants de
   comptes tiers en `9000_1xxx`, 119 auteurs de chat en `Anonyme-NNN` et leurs 258 messages
   remplacés par du lorem ipsum de longueur voisine, 12 identifiants de compte Ankama de la liste
   d'amis en `anonymeNN#NNNN`. Les noms de personnages y passent par la **même table** que les
   auteurs de chat : deux des seize amis parlaient aussi sur un canal public, et gardent donc un
   seul pseudonyme.

   Les identifiants numériques réels recopiés dans le code de test (`src/session.rs`,
   `tests/combat_orphelin.rs`, `tests/watchlist_boss_sans_ligne_ko.log`) sont neutralisés de la
   même façon. Le pseudonyme `Oumbra`, lui, reste : c'est le compte GitHub du mainteneur et le nom
   de personnage générique de toute la suite de tests (`ProjectDirs::from("com", "Oumbra", …)`,
   `APP_USER_MODEL_ID`, une centaine de sites) — le retirer ne cacherait rien que le dépôt ne
   publie déjà par son propre nom.

   Le contenu du chat n'est contraint par aucun test : le seul test de reconnaissance de messages
   (`chat_alert_live.rs`) construit ses lignes à la main, et les captures de l'onglet Chat sont
   peintes à partir de données synthétiques. La substitution garde en revanche le canal,
   l'horodatage, la répartition des auteurs et **les répétitions** — un même message republié cinq
   fois le reste, sinon la fenêtre de déduplication du parseur ne se comporterait plus pareil.

   La structure du fichier est intacte au bit près : 10 975 lignes, 10 998 CR et 10 975 LF, dont
   **23 retours chariot isolés au milieu de lignes**. Un éditeur qui « normalise les fins de
   ligne » les transforme en sauts de ligne et coupe autant d'enregistrements en deux — c'est
   arrivé une fois, le jour même, et c'est pour ça que la substitution se fait en mode binaire.

2. ✅ **Fait le 2026-09-18** — `.gitignore` sur `wakfu.log` hors les deux fixtures, et
   `scripts/check-fixtures.sh` : sept règles, une par catégorie trouvée (jeton, chemin
   `C:\Users\<nom>`, IP privée, auteur de chat, combattant humain, identifiant de compte Ankama,
   ligne d'échange). Branché aux deux bouts, comme le
   demande le `CLAUDE.md` : hook `pre-commit` (`.githooks/`) **et** étape du job `fmt` de
   `.github/workflows/ci.yml` **et** de `scripts/ci-local.sh`. Le hook seul ne suffirait pas —
   `core.hooksPath` ne survit pas au conteneur d'une session cloud.

3. ⏳ **Décision du mainteneur, hors session Claude** — retirer le contenu réel de l'**historique**.
   La fixture n'apparaît que dans **2 commits sur 936** (`c0168a4` et `ef8e2fb`, tous deux du
   2026-08-31), mais un commit embarque le SHA de son parent : réécrire le plus ancien change
   l'identifiant de **934 commits**. Leur contenu, lui, est conservé — messages, auteurs, dates,
   ordre, diffs. Ce qui casse réellement : les liens vers les anciens SHA, **les signatures des
   commits réécrits** (`filter-repo` ne peut pas les re-signer), le commit cible de la Release
   `v0.22.0`, et tout clone ou fork existant. Le `push --force` sur `dev` et `main` est interdit à
   une session Claude par le `CLAUDE.md`.

   Trois façons de procéder :

   - **ne rien réécrire** : le fichier réel reste téléchargeable à jamais via
     `raw/c0168a4/spikes/…/wakfu.log` — ne referme rien ;
   - **remplacer le contenu** (`filter-repo --blob-callback`, substituant l'ancien blob par la
     fixture pseudonymisée) : le fichier reste présent partout où il l'était, `git log` et
     `git blame` restent cohérents, seul le contenu sensible devient introuvable — **recommandé** ;
   - **supprimer le fichier** (`--path --invert-paths`) : même coût en SHA, mais les deux commits
     historiques perdent la fixture dont leurs tests dépendent.

   Dans tous les cas, le force-push ne suffit pas : GitHub continue de servir les anciens objets à
   qui connaît le SHA jusqu'à un ramassage que **seul le support déclenche**. Ouvrir un ticket
   GitHub Support citant les anciens SHA fait partie du geste, et vérifier d'abord *Insights →
   Forks* — un fork garde les objets.

4. ⏳ Consigner l'incident (date de mise en public, contenu, mesures) dans un registre interne.

### 3.2 C2 — Doctrine de vie privée contredite par le code (élevée)

`docs/plan-architecture.md` §10 pose une « contrainte de conception non négociable » : « lecture
d'un fichier texte produit par le jeu, et rien d'autre. Jamais de lecture mémoire, jamais
d'injection DLL/hook, **jamais de capture d'écran, jamais d'automatisation d'entrées** ». Or :

- `turn_watch/capture.rs:55-60` capture la fenêtre de jeu (`PrintWindow`, zone cliente
  complète) toutes les 500 ms en combat quand la notification de tour est active ;
- `chat_command.rs` et `overlay-platform/src/linux/keyboard.rs` synthétisent des frappes dans le
  client de jeu ; `notify.rs:435-460` déplace le curseur et clique.

Ce sont des décisions utilisateur documentées ailleurs dans le plan (§9.1 decies, §9.1 sexies,
§9.1 nonies) et elles restent locales (rien n'est transmis). Le problème est de **transparence**
(art. 5.1.a, art. 12) : toute information donnée aux utilisateurs à partir du §10 serait fausse,
et le §10 dit aussi que « le jeton ne transite jamais en clair sur disque hors trousseau », ce
que le repli fichier de `token_store.rs:32-49` contredit.

**Recommandations.** ✅ §10 réécrit le 2026-09-18 ; reste la politique de confidentialité (C4).
Réécrire le §10 pour décrire la réalité : capture de fenêtre locale sous
option, pixels non conservés hors de la bande du nom, entrées synthétiques limitées à trois
commandes de chat, repli fichier du jeton signalé. La même description doit alimenter la
politique de confidentialité (C4). Pour la capture : ne rendre que le rectangle utile quand
l'API le permet, ou documenter que le rendu complet transite en mémoire GDI sans être conservé.

### 3.3 C3 — Données de tiers transmises, persistées et proposées (élevée)

Trois chemins font sortir ou conserver le pseudonyme d'un joueur qui n'est pas l'utilisateur :

1. **Transmission au serveur** : `FightParticipantPayload.name` pour tous les combattants
   (`history.rs:108`, `session.rs:1909`) et `TradePayload.peerName` (`history.rs:211`,
   `session.rs:2037`), avec classe, dégâts, soins, sorts, kamas échangés et serveur.
2. **Persistance locale en clair** : `data/fight-<id>.json` (`fight_store.rs:65`) et
   `sync-queue.sqlite3` (`queue.rs:139`), ce dernier sans purge à la déconnexion ni par
   ancienneté.
3. **Autocomplétion du roster** : `personnages_tab.rs:202-222` (`journal_from_session`) propose
   les `is_ally` des combats, donc les coéquipiers réels ; un clic les fait monter sur le compte
   par `patch_roster`.

**Analyse.** La collecte de ces données n'est pas faite auprès de la personne (art. 14) et rien
ne l'en informe. La base légale envisageable est l'intérêt légitime (art. 6.1.f), qui suppose un
test de mise en balance documenté et une minimisation réelle (art. 5.1.c, art. 25). C'est une
parité voulue avec le site (`docs/plan-architecture.md` §7.1, « parité web stricte ») : la
décision appartient donc au **service**, pas au seul overlay, et toute correction côté client
doit être accompagnée côté serveur.

**Décision du 2026-09-18 (utilisateur) — option A : les noms restent en clair.** Trois options
ont été pesées : (A) garder les pseudonymes et documenter le traitement ; (B) remplacer les alliés
par un rôle (« Allié 2 · Iop ») ; (C) hacher `peerName` avec un sel propre au compte. B et C font
perdre l'essentiel de l'utilité de l'historique (« avec qui ai-je joué / échangé ? », et un
hachage ne se réaffiche pas) pour un gain que A obtient déjà par l'information et le droit
d'opposition. La minimisation (art. 5.1.c) n'oblige pas à supprimer une donnée utile, elle
oblige à ne pas garder ce qui ne sert pas. Ce que A exige, et qui reste **à faire côté site**
(`Oumbra/wakfu-companion`, hors périmètre de ce document) : une phrase dans la politique de
confidentialité (« l'historique conserve le pseudonyme des joueurs rencontrés en combat ou en
échange ; base : intérêt légitime ; conservation : N mois ; tout joueur peut demander le retrait
de son pseudonyme à <adresse> ») et une courte note interne de mise en balance.

Ce qui ne dépendait que de l'overlay est fait le même jour :

- ✅ **Autocomplétion retirée** (`fix:` 0.64.2) — pas restreinte : le champ de nom de la modale
  Personnage est une saisie nue (`personnages_tab.rs`, règle 5 de sa doc de module).
- ✅ **`fight-*.json` bornés à l'exécution** (`fix:` 0.64.3) — la règle des 24 h, qui ne jouait
  qu'au démarrage, est rejouée à chaque fois que le scan ne trouve plus aucune fenêtre de jeu
  (`EngineCommand::GameClosed` → `Engine::prune_stale_fights`). Le seuil est gardé plutôt que de
  tout effacer, pour le client qui plante en plein combat et s'y reconnecte après relance.
- ✅ **`sync-queue.sqlite3` ne survit plus au compte** (`fix:` 0.64.4) — la file ne contenait déjà
  que ce que le serveur n'a pas encore accepté (une entrée est effacée dès l'envoi réussi), donc
  une purge par ancienneté courte y aurait *perdu de l'historique* ; le vrai problème était
  l'enfilage sans compte, indéfini. Désormais : sans compte, les événements restent en mémoire
  (tampon borné) jusqu'à l'activation ; « Se déconnecter » vide la file ; une entrée en attente
  depuis plus de trente jours est abandonnée (`SyncQueue::MAX_PENDING_AGE`).

Recommandations initiales, conservées pour mémoire :

- (serveur + client) Décider ce que l'historique a réellement besoin de connaître des tiers. Pour
  le combat, un libellé de rôle (« Allié 2 · Iop ») suffit à l'affichage des dégâts d'équipe ; pour
  l'échange, `peerName` n'est utile qu'au titulaire et pourrait être **haché avec un sel propre au
  compte** (recherche « ai-je déjà échangé avec X » possible, lecture impossible côté serveur).
  Les personnages du **roster de l'utilisateur** restent en clair : ce sont les siens.
- Ne pas envoyer `peerName`/`participants[].name` en clair tant que la politique de
  confidentialité ne décrit pas ce traitement et sa base légale.
- Retirer les tiers de l'autocomplétion : filtrer `journal_from_session` sur les noms déjà
  présents dans le roster ou sur les fenêtres de jeu détectées (`GameWindowTracker::scan`, titre
  `"<Nom> - WAKFU"`, qui ne donne que les personnages de la machine).
- Purger `sync-queue.sqlite3` à la déconnexion (ou au moins les entrées de plus de N jours), et
  borner l'ancienneté des `fight-*.json` à l'exécution, pas seulement au démarrage.

### 3.4 C4 — Aucune information des personnes dans l'application (élevée)

Aucune occurrence de « confidentialité », « privacy », « CGU », « consentement » ou « RGPD » dans
`crates/*/src`. L'écran de connexion (`panels/login.rs`) n'affiche rien avant l'appairage ;
l'onglet « À propos » (`panels/a_propos_tab.rs`) ne contient que mise à jour, redémarrage et
fermeture, sans licence ni mention légale ; le dépôt lui-même n'a pas de fichier de licence. La
seule phrase descriptive est dans la section « Compte » des Options (`options_modal.rs:1470-1478`).

**Analyse.** Art. 12 et 13 : l'information doit être fournie au moment de la collecte, de façon
accessible. La création de compte se faisant sur le site, la politique de confidentialité doit y
exister (hors périmètre) et l'overlay doit y renvoyer et résumer ce qu'il fait de spécifique
(lecture du log, envoi de l'historique, capture de fenêtre optionnelle, journal local, tiers
GitHub et `vertylo.github.io`).

**Recommandations.** Une ligne « En vous connectant, vous acceptez la [politique de
confidentialité] » sous le bouton « Se connecter » ; une section « Données » dans « À propos »
listant ce qui est lu, envoyé, conservé et où, avec le bouton d'effacement de C5 ; un fichier de
licence à la racine du dépôt.

**Ce qui a été fait (2026-09-18, le soir).** L'onglet « À propos » ouvre désormais sur trois
sections d'information avant « Mise à jour » (`panels/a_propos_tab.rs`, constante `SECTIONS`) :
« Wakfu Companion » (non-affiliation, liens vers le site et le code source), « Conditions
d'utilisation de Wakfu » (voir `analyse-cgu.md` §5.4) et « Vos données » — ce qui est lu (le seul
`wakfu.log`, chat jamais envoyé ni enregistré), ce qui part au compte une fois appairé (combats
avec le nom des participants, achats, échanges avec le nom du partenaire, personnages, alertes,
recherches de chat), les deux destinataires tiers nommés (GitHub à chaque lancement,
`vertylo.github.io` pour les icônes — C10, C11), ce qui reste sur la machine et où (jeton au
trousseau, combats en cours, file d'envoi, journal de 14 jours, caches, gabarits de tour — C8), le
fait que la déconnexion n'efface que le jeton, et l'exercice des droits (page « Mon compte »,
`contact@wakfu-companion.com`), avec deux boutons vers la politique de confidentialité et les CGU
du service. Les textes décrivent le code tel qu'il est, capture de fenêtre et frappe synthétique
comprises (C2). **Restent** : la ligne sous « Se connecter », le fichier de licence, le bouton
d'effacement (C5) — et, côté site, une politique de confidentialité qui parle encore du seul
navigateur et ne mentionne ni l'overlay, ni GitHub, ni `vertylo.github.io` (§7).

### 3.5 C5 — Pas de droit à l'effacement exerçable (élevée)

La déconnexion (`background.rs:483-495`) efface le jeton et vide la mémoire du moteur
(`engine_thread.rs:534-557`), mais laisse sur disque : `config.toml` (dont `log_path`), les
journaux (14 jours), `data/fight-*.json`, `sync-queue.sqlite3`, `recap-session.json`,
`watchlist-counts.json`, `turn-templates/*.png|.name`, `focus.log`, les caches, l'entrée de
démarrage automatique et les clés de registre. Aucun bouton ne purge quoi que ce soit ; aucun
export n'existe ; aucune révocation du jeton côté serveur n'est appelée (le jeton reste valide
pour qui l'aurait copié ; pas d'expiration ni de rotation, `token_store.rs`, `background.rs`).

**Analyse.** Art. 17 (effacement) et art. 25 (protection par défaut). La plupart de ces fichiers
concernent l'utilisateur lui-même, mais `fight-*.json` et `sync-queue.sqlite3` contiennent des
tiers, et les gabarits sont des captures d'écran.

**Recommandations.**

- Bouton « Supprimer les données locales » (À propos ou Paramètres › Compte) qui efface les deux
  arbres de dossiers, avec confirmation ; l'exposer aussi en ligne de commande
  (`--purge-local-data`) pour la désinstallation.
- À la déconnexion : purger `sync-queue.sqlite3` (✅ 2026-09-18, voir C3), `fight-*.json`,
  `turn-templates/` et les journaux, ou proposer de le faire.
- (serveur) Route de révocation appelée à la déconnexion ; durée de vie du jeton natif.
- Unifier les deux racines `ProjectDirs` (C13) pour que l'effacement soit complet.

### 3.6 C6 — Journal applicatif trop bavard et sans plafond (moyenne)

`logs/overlay-ui.<date>.log`, niveau `info` par défaut en release (`logging.rs:60`), rotation
journalière, 14 fichiers (`logging.rs:71-73`), aucun plafond de taille, non désactivable depuis
l'interface. Y sont écrits :

- le **code d'appairage et l'URL de vérification** en clair (`background.rs:603-607`) — un
  secret à usage unique, exploitable pendant sa fenêtre de validité ;
- le chemin complet de `wakfu.log`, donc souvent le **nom d'utilisateur OS** (`main.rs:4363,
  3466-3469, 4798-4801`, `tailer.rs:104`, `watcher.rs:61`, `watchlist.rs:152`, `fight_store.rs:69`) ;
- les **noms de personnages** à chaque transition de fenêtre, de tour, de gabarit
  (`main.rs:1504, 2056-2079, 2425-2430`, `turn_watch/watcher.rs:221-296`, `templates.rs:68,
  89-93`, `session.rs:781-786`, `personnages_tab.rs:670, 1265, 1395, 1440`) ;
- l'**auteur d'un message de chat tiers**, le mot-clé et le canal (`engine_thread.rs:894-901`),
  et le destinataire d'une réponse privée (`main.rs:2211`, `chat_command.rs:129`) ;
- en cas d'échec de désérialisation d'un lot, **le JSON du lot entier**, `Chat { author,
  message }` compris (`quickjs_engine.rs:20-32`, journalisé par `engine_thread.rs:760`). La
  promesse « le chat n'est jamais stocké » n'est donc pas garantie par le code.

Non journalisés (bon) : le jeton, les corps de réponse HTTP, les lignes brutes du log (hors
`overlay-app`), le contenu des messages de chat en fonctionnement normal.

**Recommandations.** Ne plus journaliser le code d'appairage ni l'auteur de chat (le canal et le
mot suffisent au diagnostic) ; tronquer et expurger le JSON dans `EngineError::Deserialize`
(position de l'erreur + type de l'entrée, pas le contenu) ; journaliser les noms de personnages
au niveau `debug` ; remplacer le chemin complet par son suffixe ; plafonner la taille ; offrir
un réglage « journal détaillé » décoché par défaut ; effacer les journaux avec C5.

### 3.7 Constats secondaires

| # | Constat | Réf. | Recommandation |
| --- | --- | --- | --- |
| C7 | Jeton en clair sur disque en repli, `0600` posé **après** l'écriture et **Unix seulement** ; aucune ACL Windows ; repli déclenché aussi quand la relecture du trousseau diffère | `token_store.rs:32-49, 68-86` | Créer le fichier avec le mode restrictif dès l'ouverture (`OpenOptions::mode(0o600)`), poser une ACL utilisateur sous Windows ou chiffrer par DPAPI ; avertir l'utilisateur dans l'interface, pas seulement au journal |
| C8 | Gabarits de tour : PNG du nom rendu à l'écran + nom en clair, jamais purgés | `templates.rs:77-95` | Effacer avec C5 ; documenter dans « À propos » |
| C9 | `profile` renvoyé entier (pseudo, avatar) pour un réglage d'alerte ; `roster` réémet tout champ inconnu (`flatten extra`) | `profile.rs:278-316`, `roster.rs:86-90` | (serveur) `PATCH` par sous-clé ou fusion côté serveur ; à défaut, documenter que le contenu transmis n'est pas énumérable |
| C10 | Icônes chargées depuis `vertylo.github.io` : IP + centres d'intérêt vers un tiers non contractualisé | `catalog.rs:85,108`, `remote_icons.rs:238-250` | Servir les icônes depuis l'API ou les embarquer ; sinon nommer ce destinataire dans l'information |
| C11 | Vérification de mise à jour vers GitHub à chaque lancement | `background.rs:731+` | Nommer GitHub comme destinataire ; la case `auto_update` existe déjà, préciser qu'elle coupe aussi la vérification si c'est le cas |
| C12 | Démarrage automatique **activé par défaut** au premier lancement, sans le demander ; survit à la déconnexion | `config.rs:354-361`, `main.rs:4855` | Demander au premier lancement ou laisser décoché par défaut (loyauté, art. 5.1.a) |
| C13 | Deux racines `ProjectDirs` sous Windows, commentaire de `templates.rs:17` inexact | `config.rs:601`, `templates.rs:19` vs les 9 autres appels | Unifier sur une seule racine (migration au démarrage) |
| C14 | `overlay-app` journalise des lignes brutes du log (chat compris) en console | `overlay-app/src/main.rs:43-55` | Masquer les entrées `Chat` ou retirer ce binaire du workspace livré |
| C15 | Énumération de `~/.steam/…/compatdata` et `~/.wine/drive_c/users` pour trouver le log | `discovery.rs:72-100` | Proportionné (métadonnées seulement) ; le mentionner dans la description du traitement |
| C16 | `WAKFU_COMPANION_API_URL` redirige tous les payloads vers n'importe quelle origine sans avertissement ; `WAKFU_OVERLAY_UPDATE_URL` accepte `http://` | `client.rs:36-38`, `update/mod.rs:62, 264` | Journaliser en `warn` et afficher l'origine dans « À propos » quand elle diffère du défaut |
| C17 | Détail technique brut (chemin d'API, message d'erreur) affiché à l'écran de connexion | `login.rs:649-677` | Faible ; garder derrière un « Détails » repliable |

## 4. Fixtures et tests : règle à instaurer

Toute fixture dérivée d'un vrai `wakfu.log` doit passer par un script de pseudonymisation
versionné (`tools/anonymize-log.*`) avant d'entrer dans le dépôt, et les noms cités en dur dans
les tests doivent être synthétiques. Une vérification CI (`scripts/ci-local.sh` + `ci.yml`,
selon la règle « les deux se doublent ») refuse les motifs `Authentication token`, `C:\Users\`,
`L:/<ip>` et toute ligne de chat non pseudonymisée dans `crates/**/tests/**` et `spikes/**`.

## 5. Points conformes, à consigner

- Authentification par appairage d'appareil : corps de requête vide, aucun identifiant matériel,
  aucun mot de passe ni email manipulé (`pairing.rs:50-61`) ; messages d'erreur génériques sans
  énumération de compte (`background.rs:513-530`).
- Jeton jamais journalisé ni inclus dans une erreur ; corps de réponse HTTP jamais lus en
  erreur (`client.rs:207-224`) ; TLS `rustls` sans désactivation ; manifeste de mise à jour signé
  `minisign` et vérifié avant parsing ; SHA-256 des assets.
- Aucune télémétrie, aucun rapport de crash distant (`logging.rs:111-124` : journal local).
- Rien ne quitte la machine sans compte lié ; mode « invité » sans envoi (`background.rs`).
- Contenu des messages de chat ni transmis ni écrit sur disque en fonctionnement normal ; canaux
  privés non parsés ; position et carte non extraites.
- Presse-papiers jamais lu ; raccourcis globaux par `RegisterHotKey`/`XGrabKey`, pas de hook
  clavier ; aucune frappe enregistrée.
- Un seul fichier du jeu lu ; validation stricte du nom `wakfu.log` pour le chemin manuel
  (`discovery.rs:109-154`).
- Rétention des journaux bornée dans le temps (14 jours) ; combats en cours purgés à 24 h.

## 6. Plan d'action priorisé

| Priorité | Action | Constats | Qui |
| --- | --- | --- | --- |
| **P0** | ✅ Fixture pseudonymisée, tests réalignés, garde-fou `pre-commit` + CI (2026-09-18) | C1, §4 | Session |
| **P0** | ⏳ Réécriture d'historique (`filter-repo`, remplacement du blob), force-push `dev`/`main`, ticket GitHub Support | C1 | Mainteneur, hors session Claude |
| **P0** | Documenter l'incident de mise en public (2026-09-15) | C1 | Mainteneur |
| **P1** | Politique de confidentialité côté site (à étendre à l'overlay) ; lien à l'écran de connexion ; ✅ section « Vos données » dans « À propos » (2026-09-18) ; fichier de licence | C4 | Serveur + overlay |
| **P1** | ✅ §10 du plan d'architecture réécrit : lecture de la bande basse de la fenêtre de jeu sous option décochée par défaut, deux entrées synthétiques déclenchées par l'utilisateur, jeton porteur avec repli fichier signalé au journal (2026-09-18) | C2 | Overlay (`docs:`) |
| **P1** | Bouton « Supprimer les données locales » ; purge de la file, des combats, des gabarits et des journaux à la déconnexion ; révocation serveur | C5, C8 | Overlay + serveur |
| **P1** | ✅ Décision sur les noms de tiers : conservés en clair (option A, 2026-09-18) ; autocomplétion retirée, `fight-*.json` purgés à la fermeture du jeu, file de synchro vidée à la déconnexion et sans écriture hors compte (0.64.2 → 0.64.4). Reste la mention dans la politique de confidentialité et le contact d'opposition | C3 | Site (politique) |
| **P2** | Journal : retirer code d'appairage et auteur de chat, expurger `EngineError::Deserialize`, noms en `debug`, plafond de taille, réglage utilisateur | C6 | Overlay |
| **P2** | Jeton de repli : mode restrictif à la création, ACL/DPAPI Windows, avertissement dans l'interface | C7 | Overlay |
| **P2** | Unifier les racines de dossiers ; autostart non activé par défaut ; icônes servies par l'API ou embarquées ; nommer GitHub et `vertylo.github.io` comme destinataires | C10-C13 | Overlay |
| **P3** | `overlay-app` : masquer le chat ; `PATCH` par sous-clé côté serveur ; origine d'API affichée quand surchargée | C9, C14, C16 | Overlay + serveur |

## 7. Ce que ce document ne couvre pas

Le serveur et le site web (`Oumbra/wakfu-companion`) : durées de conservation de l'historique,
politique de confidentialité, suppression de compte, sessions actives et révocation, sous-traitants
(hébergeur, base Postgres/Neon, fournisseurs OAuth Discord/Google), registre des traitements
(art. 30) et, si le service est ouvert au public, mentions légales. L'overlay ne peut être conforme
seul : plusieurs recommandations ci-dessus supposent un pendant côté service.
