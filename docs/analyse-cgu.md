# Analyse de conformité aux CGU Wakfu

> Premier relevé le 2026-09-18 (version 0.63.1), revérifié le 2026-09-21 (version 0.75.0), puis
> **le 2026-09-25** à l'état du dépôt `dev` (version 0.82.10, après la Carte, `SessionExpired` et la
> Release du 24 ; audit des textes « À propos » contre le code). Lecture technique du code contre le texte des CGU, des règles du jeu et de
> la licence des données — **pas un avis juridique**. À refaire à chaque fonctionnalité qui touche
> au client, à la fenêtre du jeu ou aux entrées clavier/souris (recette au §6).
>
> Périmètre : les fichiers suivis par git. Ce que `.gitignore` exclut (`target/`, `vendor/`,
> journaux `wakfu.log`, clé privée de signature, artefacts de capture) n'est pas analysé.

## 1. Textes applicables

| Texte | URL | Rôle |
| --- | --- | --- |
| Conditions Générales d'Utilisation Ankama | https://www.wakfu.com/fr/cgu | Cadre contractuel, art. 5 (Jeux), 10 (Sanctions), 13 (Propriété intellectuelle) |
| Règles du jeu Wakfu | https://www.wakfu.com/fr/mmorpg/communaute/regles-jeu | **Prévalent** sur les CGU en cas de conflit (art. 5.3) |
| Licence d'utilisation des données Wakfu (v1, 2019-03-11) | [PDF sur static.ankama.com](https://static.ankama.com/comm/2019_03/2019-03-11_Licence%20d'utilisation_Donne_es%20Wakfu_v.1%20(1).pdf), annoncée par [« Des données pour tous ! »](https://www.wakfu.com/fr/mmorpg/actualites/news/980849-donnees) | **Seule autorisation écrite** qu'Ankama donne aux projets communautaires — sur ses données JSON, pas sur ses images |

Les deux pages `wakfu.com` sont derrière une redirection SSO (`account.ankama.com/sso-redirect`) :
un client HTTP sans cookies boucle en 302, il faut conserver le pot de cookies pour obtenir le
contenu (`curl -c jar -b jar -L` avec un `User-Agent` de navigateur). Le PDF de licence se
télécharge directement.

### Clauses déterminantes

- **CGU 5.1** — licence « pour votre utilisation personnelle et non-commerciale » ; « l'utilisation
  d'autres techniques de connexion que celles fournies par Ankama […] est interdite ».
- **CGU 5.2.1 / 5.2.2** — reverse engineering, décompilation, modification de tout fichier des
  Clients, œuvres dérivées des Jeux interdits.
- **CGU 5.2.4** — « Vous n'êtes pas autorisé à utiliser les Clients pour le développement de tout
  programme informatique. »
- **CGU 5.2.5** — interdiction de « créer, d'utiliser ou de promouvoir un quelconque programme ou
  outil susceptible […] d'altérer l'expérience des Jeux ou de contourner les règles des Jeux, tels
  que, de manière non limitative, les bots, […] logiciels d'automatisation, logiciels de
  modification, logiciels permettant d'automatiser des actions de clic de souris (communément
  appelés « auto-clic ») ou autres logiciels non autorisés ». Ankama peut détecter « l'utilisation
  de programmes et outils non autorisés […] qui s'exécutent conjointement à un Jeu ».
- **CGU 5.2.6** — interception des protocoles, packet-sniffing, tunneling interdits.
- **CGU 5.2.7 / 5.2.8** — pas d'exploitation commerciale, pas de distribution des fichiers du
  Client.
- **CGU 5.3.2** — interdiction de « collecter des informations dans les Jeux » et de « mettre à
  disposition des autres utilisateurs des informations personnelles […] sur un autre utilisateur ».
- **CGU 5.3.3** — publicité interdite ; les « sites de fans » peuvent être tolérés « à sa seule
  discrétion ».
- **CGU 10.2 / 10.6** — sanctions jusqu'à la suspension définitive, et « poursuites civiles et
  pénales ».
- **CGU 13.1 / 13.2** — tout élément de l'univers (objets, personnages, œuvres d'art, thèmes,
  données liées aux Jeux…) est protégé ; copie, extraction, œuvre dérivée interdites « sans
  l'accord écrit préalable d'Ankama ».
- **CGU 13.3** — les marques (dont WAKFU) ne peuvent être utilisées sans autorisation écrite.
- **CGU 13.5** — opposition à la fouille de textes et de données (TDM) sur le Site et le Launcher.
- **Règles du jeu, « Triche »** — « La création, l'utilisation ou la promotion d'un programme
  tiers ou d'un outil non autorisé par les CGU (dont les programmes communément appelés « bot » ou
  « auto-clic ») est interdite, **quel qu'en soit l'usage** ». Grille des sanctions :
  **bannissement définitif**. « La modification du client de jeu est interdite. Ceci englobe tous
  les fichiers présents dans le répertoire d'installation du jeu. »
- **Règles du jeu, « Serveurs monocomptes »** — un seul compte connecté à la fois par serveur.
- **Licence des données, art. 1** — « licence personnelle, limitée, non exclusive, non
  transférable et non cessible d'utiliser les Données pour votre usage personnel et non
  commercial, dans le cadre de votre Projet » (« un site Internet et/ou une application sur
  l'univers de notre Jeu »). « Si vous utilisez tout ou partie des Données, vous vous engagez à
  faire apparaître la mention suivante : WAKFU MMORPG : © 2012-[année en cours] Ankama Studio.
  Tous droits réservés. »
- **Licence des données, art. 2** — pas de sous-licence ; pas d'usage « en association avec […]
  des activités contraires aux Conditions Générales d'Utilisation d'Ankama (ex : sites de triche
  […]) » ; retrait « sans délai » de tout contenu sur demande d'Ankama.

À retenir : aucun programme tiers n'est autorisé par défaut, et le texte laisse à Ankama toute
discrétion. La licence des données est le seul texte où Ankama **autorise** quelque chose à un
projet communautaire — des données JSON, à usage non commercial, avec une mention obligatoire ; elle
ne dit rien des images ni des programmes qui s'exécutent avec le jeu. Tout le reste est une
appréciation du risque.

## 2. Verdict

Le socle de l'overlay — lire `wakfu.log` et afficher par-dessus le jeu — est la partie la plus
défendable. Deux fonctionnalités ajoutées les 13 et 14 septembre 2026 (frappe de commandes dans le
chat, lecture d'image de la fenêtre) relèvent de ce que les CGU nomment explicitement ; **le
2026-09-18 l'utilisateur a décidé de les conserver**, et depuis le §10 du plan d'architecture, l'onglet
« À propos » et le README les décrivent telles qu'elles sont. Le risque, lui, n'a pas changé : il
est assumé, pas levé. L'exposition la plus large, mais la moins immédiate, reste la propriété
intellectuelle des assets embarqués.

Depuis le 2026-09-18, ce qui a bougé dans le sens de la conformité : icônes servies par l'API du
service et plus par un CDN tiers (2026-09-19), §10 du plan réécrit, onglet « À propos » avec
non-affiliation et avertissement, et — ce relevé — mention de droits d'auteur exigée par la licence
des données dans l'onglet et le README, README avec section de non-affiliation, provenance des sons
documentée.

## 3. Points en écart

### 3.1 Frappe synthétique dans le chat du jeu — risque élevé, assumé

`crates/overlay-ui/src/chat_command.rs` et `crates/overlay-platform/src/linux/keyboard.rs`
(§9.1 sexies du plan). **Mécanisme inchangé au 2026-09-25 ; les raccourcis sont désactivés
par défaut depuis ce jour.**

- Les raccourcis globaux « Inviter » / « Suivre » (F1/F2 par défaut) tapent dans la fenêtre Wakfu
  au premier plan la séquence `Entrée`, `/i "Nom"` ou `/fol "Nom"`, `Entrée` — par `SendInput` +
  `KEYEVENTF_UNICODE` sous Windows, par l'extension XTEST sous X11. **Désactivés par défaut**, y
  compris pour une configuration existante (décision du mainteneur, 2026-09-25) : la case
  « Activer les raccourcis multicompte » de l'onglet « Raccourcis » les active tous deux
  (`OverlayConfig::multiaccount_shortcuts`) ; décochée, ils ne sont pas enregistrés auprès de l'OS.
  Jusqu'au 2026-09-25 on ne pouvait que les réassigner, alors que « À propos » les disait
  optionnels.
- Le clic sur une carte d'alerte de chat tape `Entrée` puis `/w "Nom" ` (sans envoi).
- Le module Linux le documente lui-même : « XTEST injecte au niveau du SERVEUR, exactement comme un
  clavier physique : aucune application ne peut la distinguer d'une vraie frappe ».

C'est un **logiciel d'automatisation d'entrées** au sens de l'art. 5.2.5 : une touche produit une
séquence complète de frappes dans le client. Que la commande soit celle que le joueur aurait tapée
ne change pas la qualification du mécanisme — un « auto-clic » tape aussi ce que l'utilisateur
aurait cliqué. Les règles du jeu sanctionnent « quel qu'en soit l'usage » par un bannissement
définitif. C'est le point le plus exposé du projet ; la position tenue (plan §10) est qu'il ne joue
pas à la place du joueur, ne procure aucun avantage et reste déclenché par lui — un argument de
proportionnalité, pas une autorisation. L'alternative sans injection (presse-papiers, §5.1) reste
la seule qui ferait disparaître le mécanisme.

### 3.2 Capture de la fenêtre du jeu — risque moyen, assumé

`crates/overlay-ui/src/turn_watch/` (§9.1 decies du plan), Windows seulement. **Inchangé au
2026-09-21**, hors option décochée par défaut (`OverlayConfig::turn_notification`).

- `capture.rs` lit par `PrintWindow` (`PW_CLIENTONLY`) la bande basse de la zone client de chaque
  fenêtre Wakfu, toutes les 500 ms en combat, même recouverte.
- `watcher.rs` / `vision.rs` y localisent le widget « Fin du tour », en extraient l'image du nom
  du personnage, et `templates.rs` persiste ces gabarits en PNG dans le dossier de données
  (effacés à la déconnexion et par « Supprimer les données locales », RGPD C8).

Ce n'est ni de la lecture mémoire, ni de l'interception de protocole (art. 5.2.6 respecté), ni une
modification du client. Mais c'est une lecture de l'état du jeu **hors du log**, et une collecte
d'informations dans le jeu (art. 5.3.2). Le risque est celui d'un « programme s'exécutant
conjointement » lisant l'écran, comparable à un logiciel de capture ; moins grave que 3.1.

### 3.3 Le plan contredit le code — ✅ clos le 2026-09-18

`docs/plan-architecture.md` §10 décrit désormais ce que le code fait réellement (lecture de
fenêtre sous option, deux cas d'entrées synthétiques, aucun ne jouant à la place du joueur) et
consigne que toute nouvelle entrée synthétique est une décision à y inscrire. La promesse
« jamais de capture d'écran, jamais d'automatisation d'entrées » a été retirée (constat C2 de
`analyse-rgpd.md`).

### 3.4 Assets issus du client — risque moyen, exposition large

Éléments protégés par l'art. 13.1 / 13.2, distribués dans un binaire public (GitHub Releases) :

| Élément | Emplacement | Origine | État au 2026-09-21 |
| --- | --- | --- | --- |
| Textures d'interface (boutons, cases, modales, ascenseurs…) | `assets/design-system/*.png` | captures du client, retouchées (`design-asset`) | inchangé |
| Icônes d'interface (51) | `assets/design-system/icons/` | extraites de captures du client | inchangé |
| Captures d'interfaces entières (HDV, options…) | `assets/design-system/interfaces/`, `docs/design-reference/` | captures brutes du client (dépôt, pas le binaire) | inchangé |
| Curseur animé | `assets/cursor/*.png` | « isolé pixel par pixel depuis un enregistrement d'écran » du jeu | inchangé |
| Bustes et portraits de classe (36 + 36) | `crates/overlay-ui/assets/avatars/`, `class-profile/` | art officiel des classes | inchangé |
| Bordures de rareté | `crates/overlay-ui/assets/items/Border-*.webp` | art du jeu | inchangé |
| Icônes de sorts, d'objets, de monstres | `remote_icons.rs`, `overlay_sync::client::icon_url` | art Ankama (`wakassets`) **relayé par l'API `wakfu-companion`** (`/api/v1/icons/…`) depuis le 2026-09-19 | le binaire ne contacte plus `vertylo.github.io`, et depuis le 2026-09-22 (`7935f48`) `assets/spells.json` et `monster-spells.json` ne portent plus que des chemins relatifs (`spells/2282.png`) |
| Données de jeu (noms, raretés, catégories d'objets, familles de monstres, sorts) | `overlay_engine::catalog` (via l'API), `assets/*.json` | gamedata Ankama (`definition.rarity`…) et encyclopédie | couvert par la **licence des données** si la mention est affichée — faite ce jour (§5.4) |
| Sons d'alerte | `crates/overlay-ui/assets/sounds/` | `loot.mp3` = `AlertSound6` de filterblade.xyz (son d'alerte de *Path of Exile*, Grinding Gear Games) ; `countdown.mp3` et `chat-filter.mp3` « fournis par l'utilisateur » ; `turn.mp3`, qui n'est plus une copie du décompte, d'origine non documentée | provenance tenue dans `assets/sounds/README.md` ; licence de `loot.mp3` **non établie** (tiers, pas Ankama), origine des trois autres à documenter |
| Polices | `assets/fonts/` | PT Serif (OFL), Ubuntu (UFL) | conforme, licences jointes |

Le design system est en outre, par construction, une **œuvre dérivée** de l'interface du jeu
(`docs/design-system.md` : « que les futurs composants […] partagent le même langage visuel que le
jeu »). La tolérance des sites de fans (art. 5.3.3) est discrétionnaire et ne vaut pas licence ; la
licence des données ne couvre pas les images. Une demande d'autorisation écrite (§5.5) reste la
seule position sûre ; en attendant, l'engagement de retrait sur demande figure dans « À propos »
et au README.

### 3.5 Marque WAKFU — risque faible, ✅ traité

Le nom du produit (`wakfu-companion-overlay`), le protocole `wakfu-companion:` enregistré pour le
toast, et l'identité du toast utilisent la marque (art. 13.3). La mention de non-affiliation
(« projet de fan […] ni édité, ni hébergé, ni approuvé par Ankama », « WAKFU est une marque
d'Ankama ») figure dans l'onglet « À propos » depuis le 2026-09-18 (`panels/a_propos_tab.rs`,
`SECTIONS`) et, depuis ce relevé, dans le README (section « Projet non officiel »). Le README
(section « Licence ») précise que la licence MIT du dépôt ne couvre pas les éléments d'Ankama ; le
fichier `LICENSE`, lui, est le texte MIT brut.

### 3.6 Données de tiers envoyées à l'API — risque faible

`crates/overlay-engine/src/history.rs` : un échange part avec `peer_name` (le nom de l'autre
joueur), un combat avec le nom de chaque participant du groupe. Les messages de chat ne sortent pas
(seuls les filtres `chatFilters` sont synchronisés). C'est davantage une question RGPD côté
`wakfu-companion.com` (données de personnes qui n'ont rien accepté — traitée dans
`analyse-rgpd.md`) qu'une question de CGU, mais l'art. 5.3.2 (« collecter des informations dans
les Jeux ») peut être invoqué. L'onglet « À propos » le dit en clair (« vos échanges (avec le nom du
partenaire) »).

### 3.7 Multicompte — remarque

L'overlay est conçu autour de plusieurs clients simultanés (une fenêtre par personnage, section
« Multicompte » des raccourcis). Il n'aide pas à contourner la limite des serveurs monocomptes,
mais le vocabulaire et les fonctionnalités visent explicitement le jeu à plusieurs comptes — à
garder en tête dans toute communication publique.

### 3.8 « Utiliser les Clients pour le développement » (art. 5.2.4) — remarque

Les captures d'interface (`assets/design-system/interfaces/`), l'enregistrement du curseur et les
fixtures de `wakfu.log` sont du client observé pendant le développement. Le texte vise d'abord la
réutilisation du Client comme composant d'un autre programme ; l'overlay n'embarque ni n'appelle
aucun fichier du Client. Signalé pour mémoire, pas comme écart.

## 4. Points conformes

- **Lecture seule du log, hors du répertoire d'installation.** `overlay-ingest::discovery` ne
  cherche `wakfu.log` que sous `%APPDATA%\zaap\gamesLogs`, `%LOCALAPPDATA%\Ankama\zaap`,
  `~/.config/zaap`, les préfixes Steam/Proton et Wine. Rien n'est jamais écrit dans un dossier du
  jeu ; la règle « modification des fichiers du client » n'est pas touchée. Aucun accès à
  `Program Files`, `steamapps/common` ni à un `.jar` du client (la seule occurrence est un
  commentaire d'`update/apply.rs` sur le dossier de l'overlay lui-même).
- **Pas de lecture mémoire, d'injection DLL, de hook clavier, de packet-sniffing** (grep §6 :
  aucune occurrence de `ReadProcessMemory`, `OpenProcess`, `SetWindowsHookEx`,
  `CreateRemoteThread`, `keybd_event`, `mouse_event`, `XSendEvent` hors commentaire). Le client
  est repéré par le **titre** de sa fenêtre (`"<Nom> - WAKFU"`, `EnumWindows` /
  `_NET_CLIENT_LIST`), jamais par son process. `RegisterHotKey` / `XGrabKey` ne servent qu'aux
  raccourcis globaux de l'overlay.
- **Aucune connexion aux serveurs d'Ankama** : hôtes contactés par le binaire = l'API
  `wakfu-companion.com` (`claude-dev.wakfu-companion.com` pour un binaire hors Release, ou
  l'origine surchargée, affichée dans « À propos ») et `github.com` avec son CDN d'assets de
  Release (mise à jour). `www.wakfu.com` n'apparaît que comme lien ouvert dans le navigateur (CGU). Ni `wakfu.com`, ni `ankama.com`, ni le CDN gamedata — l'art. 13.5 (TDM) n'est
  pas touché par ce dépôt. Les skills de synchronisation des référentiels (scraping de
  l'encyclopédie, heap dump du client Java) relèvent d'un autre outillage et tombent, eux, sous
  les art. 13.5 et 5.2.1.
- **Le clic-leurre du toast** (`turn_watch/notify.rs::focus_via_decoy`) envoie un clic synthétique
  sur une fenêtre de l'overlay, jamais sur le jeu ; `SetForegroundWindow` sur la fenêtre de jeu
  est un geste de fenêtrage, pas une entrée.
- **Usage non commercial** : pas de publicité, pas de paiement, pas de lien de don,
  `publish = false` sur toutes les crates, pas de dépendance à un service payant — conforme à
  l'art. 5.1 et à l'art. 1 de la licence des données. (Le site `wakfu-companion.com` relève d'un
  autre dépôt.)
- **Mention de droits d'auteur de la licence des données** affichée dans « À propos »
  (`a_propos_tab::copyright_notice`, année courante) et au README.
- **Polices sous licence** (PT Serif — OFL, Ubuntu — UFL), licences jointes dans `assets/fonts/`.
- **Mises à jour signées** (`minisign`, `overlay_sync::update`) ; le client n'est jamais altéré.

## 5. Recommandations

1. **Remplacer la frappe synthétique par le presse-papiers.** Copier `/i "Nom"`, `/fol "Nom"` ou
   `/w "Nom" ` et laisser le joueur coller (`Entrée`, `Ctrl+V`, `Entrée`). Le geste reste rapide,
   aucune entrée n'est injectée dans le client, et `SendInput`/XTEST disparaissent du produit.
   *Décision utilisateur du 2026-09-18 : fonction conservée. Recommandation maintenue, à
   reconsidérer si Ankama se manifeste ou si un joueur est sanctionné. Depuis le 2026-09-25, les
   raccourcis F1/F2 sont désactivés par défaut et s'activent d'une case ; la réponse en privé depuis une alerte de chat
   reste active.*
2. **Surveillance de tour** : conservée sous option décochée par défaut (décision du 2026-09-18).
   L'alternative sans capture — la bascule du titre de fenêtre (`TickInput::current`) — reste
   documentée pour le jour où l'option serait retirée.
3. ✅ **Section 10 du plan d'architecture** mise à jour le 2026-09-18.
4. ✅ **Non-affiliation, marque, mention de droits d'auteur** : onglet « À propos » (2026-09-18,
   mention de la licence ajoutée le 2026-09-21) et README (2026-09-21). Provenance des sons
   documentée (`crates/overlay-ui/assets/sounds/README.md`) ; reste au mainteneur à établir la
   licence de `loot.mp3` et l'origine de `countdown.mp3`, `chat-filter.mp3` et `turn.mp3`.
5. **Demander une autorisation écrite à Ankama** pour les assets graphiques (design system,
   curseur, portraits, bordures, icônes) et l'usage du nom — la licence des données ne les couvre
   pas. À défaut, réduire les assets embarqués à ce qui ne reproduit pas l'art du jeu, et retirer
   du dépôt les captures brutes d'interfaces (`assets/design-system/interfaces/`,
   `docs/design-reference/`), qui ne servent qu'au relevé.
6. ✅ **URL `vertylo.github.io` retirées des référentiels** (`assets/spells.json`,
   `monster-spells.json`) le 2026-09-22 (`7935f48`) : chemins relatifs seulement.
7. **Refaire ce relevé** à chaque ajout d'API système sensible (voir §6).

## 6. Recette du relevé

```bash
# API système sensibles (entrées, fenêtres, capture, process) — attendu, hors commentaires :
# SendInput dans chat_command.rs et turn_watch/notify.rs, PrintWindow dans turn_watch/, xtest dans
# overlay-platform/src/linux/keyboard.rs, RegisterHotKey/XGrabKey dans shortcuts.rs. Rien d'autre
# (les occurrences dans des commentaires — overlay-platform/linux, lib.rs, main.rs, binaire x11,
# testkit/examples — sont attendues).
rtk grep -rn -o -E "\b(SendInput|keybd_event|mouse_event|SetWindowsHookEx[AW]?|ReadProcessMemory|OpenProcess|CreateRemoteThread|PrintWindow|BitBlt|XTestFake\w+|XSendEvent|RegisterHotKey|XGrabKey|xtest)\b" crates --include=*.rs

# Accès au répertoire d'installation du jeu (doit rester vide hors commentaires)
rtk grep -rn -i -E "program files|steamapps/common|Ankama Launcher|\.jar\b" crates --include=*.rs

# Hôtes distants référencés par le code (attendu : wakfu-companion.com,
# claude-dev.wakfu-companion.com, github.com, www.wakfu.com (lien navigateur), 127.0.0.1/localhost ;
# vertylo.github.io et example.* seulement dans des commentaires ou des tests)
rtk grep -rhoE "https?://[a-zA-Z0-9./_-]+" crates --include=*.rs | sed -E 's#(https?://[^/]+).*#\1#' | sort | uniq -c | sort -rn

# Mention de droits d'auteur et non-affiliation, dans l'onglet et le README
rtk grep -rn "Ankama Studio. Tous droits réservés" crates/overlay-ui/src README.md
rtk grep -n "aucun lien avec cette société" crates/overlay-ui/src/panels/a_propos_tab.rs README.md
```
