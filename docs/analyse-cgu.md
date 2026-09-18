# Analyse de conformité aux CGU Wakfu

> Relevé du 2026-09-18, à l'état du dépôt `dev` (version 0.63.1). Lecture technique du code contre
> le texte des CGU et des règles du jeu — **pas un avis juridique**. À refaire à chaque
> fonctionnalité qui touche au client, à la fenêtre du jeu ou aux entrées clavier/souris.

## 1. Textes applicables

| Texte | URL | Rôle |
| --- | --- | --- |
| Conditions Générales d'Utilisation Ankama | https://www.wakfu.com/fr/cgu | Cadre contractuel, art. 5 (Jeux), 10 (Sanctions), 13 (Propriété intellectuelle) |
| Règles du jeu Wakfu | https://www.wakfu.com/fr/mmorpg/communaute/regles-jeu | **Prévalent** sur les CGU en cas de conflit (art. 5.3) |

Les deux pages sont derrière une redirection SSO (`account.ankama.com/sso-redirect`) : un client
HTTP sans cookies boucle en 302, il faut conserver le pot de cookies pour obtenir le contenu.

### Clauses déterminantes

- **CGU 5.2.5** — interdiction de « créer, d'utiliser ou de promouvoir un quelconque programme ou
  outil susceptible […] d'altérer l'expérience des Jeux ou de contourner les règles des Jeux, tels
  que, de manière non limitative, les bots, […] logiciels d'automatisation, logiciels de
  modification, logiciels permettant d'automatiser des actions de clic de souris (communément
  appelés « auto-clic ») ou autres logiciels non autorisés ». Ankama peut détecter « l'utilisation
  de programmes et outils non autorisés […] qui s'exécutent conjointement à un Jeu ».
- **CGU 5.2.1 / 5.2.2** — reverse engineering, décompilation, modification de tout fichier des
  Clients interdits.
- **CGU 5.2.6** — interception des protocoles, packet-sniffing, tunneling interdits.
- **CGU 5.2.7 / 5.2.8** — pas d'exploitation commerciale, pas de distribution des fichiers du
  Client.
- **CGU 5.3.2** — interdiction de « collecter des informations dans les Jeux ».
- **CGU 5.3.3** — publicité interdite ; les « sites de fans » peuvent être tolérés « à sa seule
  discrétion ».
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

À retenir : aucun programme tiers n'est autorisé par défaut, et le texte laisse à Ankama toute
discrétion. La seule position sûre est une **autorisation écrite** ; tout le reste est une
appréciation du risque.

## 2. Verdict

Le socle de l'overlay — lire `wakfu.log` et afficher par-dessus le jeu — est la partie la plus
défendable. Deux fonctionnalités ajoutées les 13 et 14 septembre 2026 déplacent nettement le projet
vers ce que les CGU nomment explicitement, et la section 10 du plan d'architecture affirme
aujourd'hui le contraire de ce que fait le code. L'exposition la plus large, mais la moins
immédiate, est la propriété intellectuelle des assets embarqués.

## 3. Points en écart

### 3.1 Frappe synthétique dans le chat du jeu — risque élevé

`crates/overlay-ui/src/chat_command.rs` et `crates/overlay-platform/src/linux/keyboard.rs`
(§9.1 sexies du plan).

- Les raccourcis globaux « Inviter » / « Suivre » (F1/F2 par défaut) tapent dans la fenêtre Wakfu
  au premier plan la séquence `Entrée`, `/i "Nom"` ou `/fol "Nom"`, `Entrée` — par `SendInput` +
  `KEYEVENTF_UNICODE` sous Windows, par l'extension XTEST sous X11.
- Le clic sur une carte d'alerte de chat tape `Entrée` puis `/w "Nom" ` (sans envoi).
- Le module Linux le documente lui-même : « XTEST injecte au niveau du SERVEUR, exactement comme un
  clavier physique : aucune application ne peut la distinguer d'une vraie frappe ».

C'est un **logiciel d'automatisation d'entrées** au sens de l'art. 5.2.5 : une touche produit une
séquence complète de frappes dans le client. Que la commande soit celle que le joueur aurait tapée
ne change pas la qualification du mécanisme — un « auto-clic » tape aussi ce que l'utilisateur
aurait cliqué. C'est le seul point jugé indéfendable en l'état.

### 3.2 Capture de la fenêtre du jeu — risque moyen

`crates/overlay-ui/src/turn_watch/` (§9.1 decies du plan), Windows seulement.

- `capture.rs` lit par `PrintWindow` (`PW_RENDERFULLCONTENT`) la bande basse de la zone client de
  chaque fenêtre Wakfu, à chaque tick, même recouverte.
- `watcher.rs` / `vision.rs` y localisent le widget « Fin du tour », en extraient l'image du nom
  du personnage, et `templates.rs` persiste ces gabarits en PNG dans le dossier de données.

Ce n'est ni de la lecture mémoire, ni de l'interception de protocole (art. 5.2.6 respecté), ni une
modification du client. Mais c'est une lecture de l'état du jeu **hors du log**, et une collecte
d'informations dans le jeu (art. 5.3.2). Le risque est celui d'un « programme s'exécutant
conjointement » lisant l'écran, comparable à un logiciel de capture ; moins grave que 3.1, mais
c'est une seconde ligne franchie par rapport à la posture d'origine.

### 3.3 Le plan contredit le code — à corriger

`docs/plan-architecture.md` §10 « Sécurité, vie privée, conformité » pose comme « contrainte de
conception non négociable » : « lecture d'un fichier texte produit par le jeu, et rien d'autre.
Jamais de lecture mémoire, jamais d'injection DLL/hook, **jamais de capture d'écran, jamais
d'automatisation d'entrées**. » Les points 3.1 et 3.2 la violent. `CLAUDE.md` demande de mettre le
plan à jour quand une décision le contredit : quelle que soit la décision prise sur 3.1 et 3.2, la
section 10 doit dire ce que le code fait réellement.

### 3.4 Assets issus du client — risque moyen, exposition large

Éléments protégés par l'art. 13.1 / 13.2, distribués dans un binaire public (GitHub Releases) :

| Élément | Emplacement | Origine |
| --- | --- | --- |
| Textures d'interface (boutons, cases, modales, ascenseurs…) | `assets/design-system/*.png` | captures du client, retouchées (`design-asset`) |
| Icônes d'interface (51) | `assets/design-system/icons/` | extraites de captures du client |
| Captures d'interfaces entières (HDV, options…) | `assets/design-system/interfaces/` | captures brutes du client (dépôt, pas le binaire) |
| Curseur animé | `assets/cursor/*.png` | « isolé pixel par pixel depuis un enregistrement d'écran » du jeu |
| Bustes et portraits de classe (36 + 36) | `crates/overlay-ui/assets/avatars/`, `class-profile/` | art officiel des classes |
| Bordures de rareté | `crates/overlay-ui/assets/items/Border-*.webp` | art du jeu |
| Icônes de sorts, d'objets, de monstres | `assets/spells.json`, `monster-spells.json`, `remote_icons.rs` | chargées depuis `vertylo.github.io/wakassets` (art Ankama rehébergé par un tiers) |
| Sons d'alerte | `crates/overlay-ui/assets/sounds/` | « fourni par l'utilisateur », provenance à documenter |

Le design system est en outre, par construction, une **œuvre dérivée** de l'interface du jeu
(`docs/design-system.md` : « que les futurs composants […] partagent le même langage visuel que le
jeu »). La tolérance des sites de fans (art. 5.3.3) est discrétionnaire et ne vaut pas licence.

### 3.5 Marque WAKFU — risque faible

Le nom du produit (`wakfu-companion-overlay`), le protocole `wakfu-companion:` enregistré pour le
toast, et l'identité du toast utilisent la marque (art. 13.3). Aucune mention de non-affiliation à
Ankama n'existe nulle part dans l'interface — ni dans l'onglet « À propos »
(`panels/a_propos_tab.rs`), ni dans le README. *Ajoutée dans l'onglet « À propos » le 2026-09-18 au
soir (voir §5, point 4) ; le README reste.*

### 3.6 Données de tiers envoyées à l'API — risque faible

`crates/overlay-engine/src/history.rs` : un échange part avec `peer_name` (le nom de l'autre
joueur), un combat avec le nom de chaque participant du groupe. Les messages de chat ne sortent pas
(seuls les filtres `chatFilters` sont synchronisés). C'est davantage une question RGPD côté
`wakfu-companion.com` (données de personnes qui n'ont rien accepté) qu'une question de CGU, mais
l'art. 5.3.2 (« collecter des informations dans les Jeux ») peut être invoqué.

### 3.7 Multicompte — remarque

L'overlay est conçu autour de plusieurs clients simultanés (une fenêtre par personnage, section
« Multicompte » des raccourcis). Il n'aide pas à contourner la limite des serveurs monocomptes,
mais le vocabulaire et les fonctionnalités visent explicitement le jeu à plusieurs comptes — à
garder en tête dans toute communication publique.

## 4. Points conformes

- **Lecture seule du log, hors du répertoire d'installation.** `overlay-ingest::discovery` ne
  cherche `wakfu.log` que sous `%APPDATA%\zaap\gamesLogs`, `~/.config/zaap`, les préfixes
  Steam/Proton et Wine. Rien n'est jamais écrit dans un dossier du jeu ; la règle « modification
  des fichiers du client » n'est pas touchée.
- **Pas de lecture mémoire, d'injection DLL, de hook clavier, de packet-sniffing** dans ce dépôt
  (grep des API `ReadProcessMemory`, `OpenProcess`, `SetWindowsHookEx`, `CreateRemoteThread`… :
  aucune occurrence). Le client est repéré par le **titre** de sa fenêtre (`"<Nom> - WAKFU"`,
  `EnumWindows` / `_NET_CLIENT_LIST`), jamais par son process.
- **Le clic-leurre du toast** (`turn_watch/notify.rs::focus_via_decoy`) envoie un clic synthétique
  sur une fenêtre de l'overlay, jamais sur le jeu ; `SetForegroundWindow` sur la fenêtre de jeu
  est un geste de fenêtrage, pas une entrée.
- **Usage non commercial** : pas de publicité, pas de paiement, `publish = false`, pas de
  dépendance à un service payant. (Le site `wakfu-companion.com` relève d'un autre dépôt.)
- **Polices sous licence** (PT Serif — OFL, Ubuntu — UFL), licences jointes dans `assets/fonts/`.
- **Mises à jour signées** (`minisign`, `overlay_sync::update`) ; le client n'est jamais altéré.
- **Pas de fouille du site** (art. 13.5) dans ce dépôt : les référentiels descendent de l'API
  `wakfu-companion`. Les skills de synchronisation (scraping de l'encyclopédie, heap dump du client
  Java) relèvent d'un autre outillage et tombent, eux, sous les art. 13.5 et 5.2.1.

## 5. Recommandations

1. **Remplacer la frappe synthétique par le presse-papiers.** Copier `/i "Nom"`, `/fol "Nom"` ou
   `/w "Nom" ` et laisser le joueur coller (`Entrée`, `Ctrl+V`, `Entrée`). Le geste reste rapide,
   aucune entrée n'est injectée dans le client, et `SendInput`/XTEST disparaissent du produit.
2. **Décider du sort de la surveillance de tour.** Soit l'assumer et la documenter au plan comme
   une lecture d'écran passive, à activer explicitement par l'utilisateur ; soit la réduire à la
   bascule du titre de fenêtre (`TickInput::current`), qui ne capture rien et couvre déjà le cas
   des héros.
3. **Mettre à jour la section 10 du plan d'architecture** pour qu'elle décrive ce que le code fait
   réellement, quelle que soit la décision.
4. **Ajouter une mention de non-affiliation** (« projet de fan, non affilié à Ankama ; WAKFU est
   une marque d'Ankama ») dans l'onglet « À propos » et le README, et documenter la provenance
   des sons. ✅ *Onglet « À propos », 2026-09-18 au soir* (`panels/a_propos_tab.rs`, constante
   `SECTIONS`) : section « Wakfu Companion » (non-affiliation, marque, propriété des éléments du
   jeu, retrait sur demande) et section « Conditions d'utilisation de Wakfu », qui dit ce que
   l'overlay fait — lecture du log, **et** les deux fonctions des §3.1 et §3.2, nommées telles
   quelles — puis rappelle l'interdiction des programmes non autorisés et laisse l'appréciation au
   joueur, avec un bouton vers `wakfu.com/fr/cgu`. Restent le README et la provenance des sons.
5. **Demander une autorisation écrite à Ankama** pour les assets graphiques et l'usage du nom, ou
   vérifier que leur politique à l'égard des contenus de fans couvre ce cas. À défaut, réduire les
   assets embarqués à ce qui ne reproduit pas l'art du jeu.
6. **Refaire ce relevé** à chaque ajout d'API système sensible (voir §6).

## 6. Recette du relevé

```bash
# API système sensibles (entrées, fenêtres, capture, process)
grep -rn -o -E "\b(SendInput|keybd_event|mouse_event|SetWindowsHookEx[AW]?|ReadProcessMemory|OpenProcess|CreateRemoteThread|PrintWindow|BitBlt|XTestFake\w+|XSendEvent|RegisterHotKey|XGrabKey)\b" crates --include=*.rs

# Accès au répertoire d'installation du jeu (doit rester vide)
grep -rn -i "program files\|steamapps/common\|Ankama Launcher\|\.jar\b" crates --include=*.rs

# Hôtes distants référencés (art Ankama rehébergé, API)
grep -rhoE "https?://[a-zA-Z0-9./_-]+" crates assets/*.json | sed -E 's#(https?://[^/]+).*#\1#' | sort | uniq -c | sort -rn
```
