# Mise à jour automatique de l'overlay — plan (2026-09-15)

Objectif fixé par le mainteneur : **plus jamais réinstaller l'overlay à la main**. L'overlay
cherche lui-même s'il existe une version plus récente — au démarrage, derrière l'écran de
chargement, et à la demande depuis la fenêtre Options — la télécharge (en différentiel quand c'est
possible), l'installe et se relance, en montrant à l'utilisateur ce qui se passe : quelles étapes
chargent, où en est le téléchargement.

Contraintes posées : Rust et l'architecture existante (threads bloquants, `ureq`, `ArcSwap`,
`UserEvent`), l'API Cloudflare Pages du dépôt web, **aucune brique nouvelle** — ou alors gratuite et
compatible avec ce qui tourne déjà. Ce document compare les solutions, en recommande une, décrit
le flux visuel et découpe le travail. Il complète le §11 de [`plan-architecture.md`](plan-architecture.md)
(« Mise à jour : vérification `GET` de la dernière Release au démarrage, téléchargement en tâche
de fond, application au prochain lancement ») et le lot **L6 — Packaging** de sa feuille de route,
seul lot jamais démarré.

---

## 0. En deux mots

**Recommandation : GitHub Releases comme dépôt de binaires, un workflow de release déclenché par
la fusion sur `main`, un module de mise à jour maison dans `overlay-sync` (`ureq` + `self-replace`
+ `minisign-verify`), un manifeste `latest.json` signé, l'installation au démarrage derrière
l'écran de chargement existant, et le différentiel (`qbsdiff`) en seconde phase, une fois sa
taille mesurée par le CI.**

Rien de tout cela n'ajoute de service : GitHub (déjà l'hébergeur du code et du CI) et Cloudflare
Pages (déjà l'API) suffisent, gratuitement, et les trois crates ajoutées sont pures Rust, sans
runtime async, sans dépendance système.

Ce qui reste au mainteneur : fusionner `dev` dans `main` quand il veut publier. Le reste est
automatique.

---

## 1. État des lieux (vérifié dans les deux dépôts, pas supposé)

| Fait | Où | Conséquence pour le plan |
| --- | --- | --- |
| **Les deux dépôts GitHub sont publics** (`"private": false` sur l'API GitHub, 2026-09-15) alors que `CLAUDE.md` dit encore « dépôt privé, minutes comptées » | `api.github.com/repos/Oumbra/*` | Les assets de Release sont téléchargeables sans jeton, et les minutes Actions sont **gratuites** sur les runners standard. À confirmer par le mainteneur (voir §10) ; le plan reste valable si le dépôt redevient privé (variante §3.2). |
| Aucune Release, aucun tag, aucun job de release ; le CI ne compile jamais le produit en `--release` | `ci.yml` (5 jobs), `git tag` vide | Tout le lot L6 est à construire. |
| Le binaire est **mono-fichier** : polices, sons, images, sorts, moteur JS et catalogue de repli sont embarqués par `include_bytes!` (~6,1 Mo de charge utile) ; aucun fichier à côté de l'exe | `design/assets.rs`, `fonts.rs`, `alert_sound.rs`, `catalog_cache.rs`… | Mettre à jour = remplacer **un** fichier. Pas besoin d'installeur pour la mise à jour elle-même. |
| Aucun `[profile.release]` déclaré : pas de `strip`, pas de LTO | `Cargo.toml` racine | Le binaire livré est plus gros qu'il ne devrait (symboles, code mort inter-crates). Correction triviale, à faire avant la première release. |
| Client HTTP : `ureq` 3 + rustls, **pas de tokio ni reqwest**, décision « définitive » (un seul modèle de concurrence : `std::thread` bloquants) | `overlay-sync/src/client.rs:1-4`, §7.3 du plan | Écarte la crate `self_update` (tire `reqwest`). Le téléchargement sera un thread bloquant de plus, comme Auth/Catalogue/Sync. |
| `client::fetch_bytes(url)` télécharge n'importe quelle URL absolue mais **tout en mémoire**, sans progression | `client.rs:159-175` | À doubler d'une variante en flux vers un fichier, avec rappel de progression. |
| `DEFAULT_BASE_URL` pointe sur **`claude-dev.wakfu-companion.com`** avec un TODO « à repointer avant toute release réelle » | `client.rs:17-29` | Dette bloquante avant la première Release. |
| Écran de chargement **déjà en place** : `StartupProgress` (3 drapeaux atomiques `catalog`/`dungeons`/`log_replayed`, garde-fou 45 s, `pending()` liste les étapes restantes), carte `panels::login` avec rouage `design::loader` 72 px et version en pied | `startup.rs`, `login.rs:329-350`, §9.1 undecies | Point d'accroche naturel : une étape « Mise à jour » de plus, et une vraie liste d'étapes visible. |
| `design::meter(ratio)` existe (jauge 0→1 du panneau Combat), `design::button`, `design::confirm_dialog`, `design::info_text`, `design::checkbox` aussi | `design/components/` | Aucun composant à créer pour la barre de progression ni pour la section Options. |
| Fenêtre Options : onglet « Paramètres » = sections Fichier / Combat / Compte, motif « heading + info_text + bouton + confirm + `OptionsModalAction` traité par l'hôte » | `options_modal.rs:653-841` | La section « Mise à jour » suit exactement ce motif. |
| Threads de fond nommés + `ArcSwap` + `EventLoopProxy<UserEvent>` + `backoff_delay` ; `UserEvent::{NewSnapshot, AuthStatusChanged, StartupProgress}` | `background.rs`, `render_content.rs:83-89` | Un `UserEvent::UpdateProgress` et un `spawn_update_thread` s'y ajoutent sans rien changer au modèle. |
| Deux hôtes dupliquent la boucle d'événements : `main.rs` (Windows) et `bin/overlay-ui-x11.rs` (Linux) | — | Tout ce qui est partageable va dans la lib (`overlay_ui::background`, `startup`, `panels`), les deux hôtes ne font que brancher. |
| Le binaire sait déjà **se relancer avec un argument spécial** et sortir aussitôt (URI de focus) ; `logging::log_session_end` est appelé à chaque sortie | `main.rs:3159-3166` | Précédent pour un argument `--updated-from <version>` et pour une sortie propre avant relance. |
| `build.rs` accepte `WAKFU_OVERLAY_COMMIT` en surcharge, prévu pour « une chaîne de release qui construit sans `.git/` » | `overlay-ui/build.rs:28-33` | Déjà prêt pour le CI de release. |
| Déjà dans l'arbre de dépendances : `sha2` 0.10 et `flate2` 1.1 (directes), `semver` 1.0 (transitive), `rustls`/`ring` (via `ureq`) | `Cargo.lock` | Intégrité SHA-256 et gzip gratuits. Absents : `minisign`/ed25519 (pourtant nommés au §10 du plan), `zstd`, `bsdiff`. |
| Côté serveur : 25 routes `functions/api/v1/**`, aucun binding Cloudflare (ni R2, ni KV, ni D1 — décision documentée), gabarit `catalog_meta` + `GET /catalog/version` + `indexHash`, Functions free : **10 ms CPU/requête**, 100 000 requêtes/jour | `wrangler.toml`, `server/README.md` | Une façade `/api/v1/overlay/release` est possible sans base ni binding (§3.2), mais pas nécessaire au départ. |
| Aucune page de téléchargement, aucun lien vers une Release sur le site | `src/` du dépôt web | À ajouter le jour de la première Release (hors périmètre de la mise à jour, mais logique). |
| Le §11 du plan prévoit un « bundle moteur mis à jour sans nouvelle version du binaire » ; ce bundle fait **31 Ko** et est embarqué | `quickjs_engine.rs:15` | Un second canal signé pour 31 Ko n'a plus de sens une fois le binaire auto-mis à jour : à retirer du plan (§10, décision 7). |

---

## 2. Objectifs et non-objectifs

**Objectifs**

1. Au démarrage, l'overlay vérifie s'il existe une version plus récente ; si oui et si l'utilisateur
   l'autorise (défaut : oui), il la télécharge, l'installe et se relance **avant** d'ouvrir les
   overlays de jeu — jamais au milieu d'une session.
2. Depuis Options › Paramètres, un bouton « Rechercher une mise à jour » qui devient « Mettre à
   jour vers X » ; l'installation ferme les overlays, repasse par la fenêtre de chargement avec une
   barre de progression, puis relance.
3. L'écran de chargement montre **quoi** charge (sorts, catalogue, donjons, rattrapage du log,
   compte, mise à jour) et, pendant un téléchargement, **combien** (Mo reçus / total).
4. Différentiel : ne télécharger que ce qui change entre la version installée et la dernière,
   quand ça vaut le coup.
5. Intégrité : rien n'est jamais installé sans vérification de signature et de hachage (§10 du
   plan : « un asset de Release non vérifié n'est jamais chargé »).
6. Publication automatique : aucune étape manuelle au-delà de la fusion sur `main`.

**Non-objectifs (v1)**

- Installeur NSIS/MSI, AppImage, raccourci du menu Démarrer, `AppUserModelID` : c'est le reste du
  lot L6, mené séparément. L'auto-update fonctionne sur un exe posé n'importe où par l'utilisateur.
- Signature Authenticode (décision ouverte §14.4 du plan) : hors périmètre. À noter : une mise à
  jour écrite par l'overlay lui-même **ne porte pas de Mark-of-the-Web**, SmartScreen ne se
  redéclenche donc pas à chaque version — seul le tout premier téléchargement par navigateur y est
  exposé.
- Canal bêta / plusieurs canaux : le manifeste le prévoit (`channel`), l'UI non.
- Retour à la version précédente depuis l'UI : le fichier précédent est conservé un cycle, la
  commande n'est pas exposée.

---

## 3. Où vivent les binaires — solutions comparées

| Critère | **A. GitHub Releases** (direct) | **B. Releases + façade API** `/api/v1/overlay/release` | C. Cloudflare R2 | D. `public/` du site Pages |
| --- | --- | --- | --- | --- |
| Brique nouvelle | aucune (le dépôt est déjà sur GitHub, le CI aussi) | aucune (une Function de plus dans le projet Pages existant) | **oui** : premier binding du projet, secrets `wrangler` en plus | aucune |
| Coût | 0 (public : bande passante et stockage des assets illimités, 2 Gio par fichier) | 0 (1 requête par lancement, cache edge 5 min ; quota 100 000/jour) | 0 jusqu'à 10 Go et 10 M lectures/mois | 0 |
| Dépôt privé possible ? | non (les assets exigent un jeton) | **oui** : la Function porte le jeton et renvoie une URL signée temporaire (GitHub répond 302 vers `objects.githubusercontent.com`) | oui | oui |
| Historique des versions | toutes, une Release par version, notes de version incluses | idem | à gérer soi-même | non : une seule version, gonfle chaque déploiement du site |
| Taille max par fichier | 2 Gio | 2 Gio | 5 Tio | **25 Mio** — un exe release peut le dépasser |
| Contrôle (version minimale, coupe-circuit, statistiques) | via le manifeste signé publié avec la Release | **oui, côté serveur**, sans republier | non | non |
| Delta | oui (assets supplémentaires) | oui | oui | non réaliste |
| Complexité | la plus faible | faible (une Function de 60 lignes) | moyenne | faible mais couple les deux dépôts |

**Recommandation : A pour démarrer, B comme extension optionnelle (phase 4).** Le client lit un
manifeste à une URL stable qui ne dépend pas de l'API GitHub (donc sans quota de 60 requêtes/heure) :

```
https://github.com/Oumbra/wakfu-companion-overlay/releases/latest/download/latest.json
https://github.com/Oumbra/wakfu-companion-overlay/releases/latest/download/latest.json.minisig
https://github.com/Oumbra/wakfu-companion-overlay/releases/download/v0.20.0/<asset>
```

Pour passer de A à B, seule l'URL du manifeste change côté client : le format du manifeste, la
signature et les assets restent identiques. C'est ce qui rend B une **extension**, pas une
réécriture — et ce qui protège si le dépôt redevient privé.

C est écarté : c'est précisément la brique supplémentaire que le mainteneur veut éviter, et elle
n'apporte rien que A n'ait déjà. D est écarté par la limite de 25 Mio et parce qu'une Release du
binaire forcerait un déploiement du site.

---

## 4. Mécanisme client — solutions comparées

| | 1. Crate `self_update` | **2. Module maison** (`ureq` + `self-replace` + `minisign-verify`) | 3. Installeur qui se met à jour (MSI/winget, AppImageUpdate) |
| --- | --- | --- | --- |
| Colle à l'architecture | **non** : tire `reqwest` (donc `tokio`), second modèle de concurrence, contraire au §7.3 | oui : un `std::thread` bloquant de plus, `ureq` partagé | partiellement (mécanisme externe, expérience utilisateur hors overlay) |
| Signature | non (hachage seulement) | oui, ed25519 (`minisign-verify`, pure Rust, minuscule) | dépend de l'outil |
| Delta | non | oui (`qbsdiff`, phase 3) | zsync (AppImage), non (MSI) |
| Progression dans notre UI | difficile | native | non |
| Windows + Linux | oui | oui (`self-replace` gère le renommage d'un exe en cours d'exécution sous Windows) | deux outils différents |
| Code à écrire | peu | ~600 lignes (téléchargement, vérification, application, thread, UI) | packaging + intégration |

**Recommandation : 2.** Les trois crates à ajouter, toutes pures Rust :

| Crate | Version (2026-09) | Rôle | Pourquoi celle-là |
| --- | --- | --- | --- |
| `self-replace` | 1.5 | remplacer l'exe en cours d'exécution (Windows : renommage de l'exe courant puis copie, nettoyage différé ; Linux : `rename` atomique) | seule façon fiable sous Windows sans processus tiers, 0 dépendance lourde |
| `minisign-verify` | 0.2 | vérifier la signature ed25519 du manifeste | format `minisign` déjà nommé au §10 du plan ; vérification seule (la clé privée ne quitte jamais le CI) |
| `semver` | 1.0 (passe en dépendance directe) | comparer `build_info::VERSION` à `manifest.version` | déjà dans l'arbre |
| `qbsdiff` | 1.4 (phase 3) | générer (CI) et appliquer (client) un patch bsdiff | pure Rust, patch appliqué en flux vers un fichier, source (l'exe courant) lue en mémoire une fois |

Ni `zstd` ni `tar` : les assets sont compressés en **gzip** via `flate2`, déjà présent. Sur un exe
Rust, gzip perd ~15 % de compression face à zstd -19 ; c'est le prix de « pas de brique
nouvelle », et le différentiel comble largement l'écart.

---

## 5. Le manifeste signé

Un seul fichier décrit la dernière version ; il est généré par le CI et signé avec la clé
`minisign` du projet (clé privée en secret GitHub Actions, clé publique embarquée dans le binaire).
Le client ne fait confiance qu'à un manifeste dont la signature est valide ; les assets sont
ensuite vérifiés par leur SHA-256 inscrit dans ce manifeste.

```json
{
  "schema": 1,
  "channel": "stable",
  "version": "0.20.0",
  "commit": "a1b2c3d",
  "publishedAt": "2026-09-20T18:04:11Z",
  "notesUrl": "https://github.com/Oumbra/wakfu-companion-overlay/releases/tag/v0.20.0",
  "minimumVersion": "0.17.0",
  "assets": {
    "windows-x86_64": {
      "name": "wakfu-companion-overlay-0.20.0-windows-x86_64.exe.gz",
      "size": 11834112,
      "sha256": "…",
      "installed": { "size": 31457280, "sha256": "…" }
    },
    "linux-x86_64": { "name": "wakfu-companion-overlay-0.20.0-linux-x86_64.gz", "…": "…" }
  },
  "deltas": {
    "windows-x86_64": [
      { "from": "0.19.0", "fromSha256": "…", "name": "…-0.19.0-to-0.20.0-windows-x86_64.patch", "size": 3211264, "sha256": "…" },
      { "from": "0.18.3", "…": "…" }
    ]
  }
}
```

- `minimumVersion` : en dessous, la mise à jour est **obligatoire** même si l'utilisateur a
  désactivé l'automatique (cas d'une rupture d'API côté serveur). Au-dessus, elle reste
  proposée.
- `installed.sha256` : hachage de l'exe décompressé — c'est lui qui est vérifié juste avant
  `self_replace`, et c'est lui qui sert de `fromSha256` aux deltas de la version suivante.
- `deltas[*].fromSha256` : le client n'applique un patch que si le hachage de **son propre exe**
  correspond (un build local ou modifié retombe sur l'asset complet).
- URL des assets : toujours reconstruite `releases/download/v{version}/{name}` — le manifeste ne
  porte pas d'URL absolue, ce qui permet de le relayer par l'API (§3, B) sans le réécrire.

---

## 6. Chaîne de publication (CI)

**Quand publier ?** Le hook `post-commit` incrémente la version à chaque commit sur `dev` : une
Release par commit n'aurait aucun sens (et coûterait un build Windows à chaque `fix:`). La bonne
granularité est celle qui existe déjà dans les règles du dépôt : **la fusion de `dev` dans `main`,
faite par le mainteneur**. La version de la Release est celle que `Cargo.toml` porte à cet
instant ; les sauts (`0.19.0` → `0.27.2`) sont normaux et sans conséquence.

`.github/workflows/release.yml`, déclenché par `push` sur `main` et `workflow_dispatch` :

1. **`version`** — lit `[workspace.package] version` ; si le tag `v{version}` existe déjà, s'arrête
   (idempotent : un push sur `main` sans bump ne republie rien).
2. **`build-windows`** / **`build-linux`** (parallèles) — `vendor-wgpu-hal`, cache, puis
   `cargo build --release -p overlay-ui --bin overlay-ui` (Windows) et `--bin overlay-ui-x11`
   (Linux), avec `WAKFU_OVERLAY_COMMIT=${{ github.sha }}`. Artefacts éphémères.
3. **`publish`** — `cargo xtask dist` (nouvelle sous-commande de l'outillage existant) : télécharge
   les assets des 3 Releases précédentes (`gh release download`, jeton du job), décompresse,
   génère les deltas (`qbsdiff`, phase 3), compresse les exes en gzip, calcule les SHA-256, écrit
   `latest.json`, le signe (`minisign` en ligne de commande depuis le secret
   `MINISIGN_SECRET_KEY` + `MINISIGN_PASSWORD`), crée le tag et la Release
   (`softprops/action-gh-release@v2`) avec le corps généré depuis les commits Conventional
   Commits depuis le tag précédent (`feat:` → « Nouveautés », `fix:` → « Corrections »).

Ce que le mainteneur fait, une fois pour toutes : générer la paire de clés (`minisign -G`),
déposer les deux secrets, coller la clé publique dans `overlay_sync::update::PUBLIC_KEY`. Ce qu'il
fait à chaque version : `git merge dev` sur `main`, `git push`.

**Coût** : dépôt public ⇒ 0. S'il redevenait privé : ~15 min Windows (facturées ×2) + ~10 min
Linux par Release, soit ~40 min de quota par publication — raisonnable à raison d'une Release par
semaine, pas par commit.

**`ci.yml` ne change pas** (et `scripts/ci-local.sh` non plus) : le workflow de release est un
fichier séparé, qui ne rejoue pas les tests — ils ont déjà tourné sur `dev` avant la fusion.

### 6.1 Clés de signature — ce que c'est, comment les générer, où les ranger

**Le principe.** `minisign` est un outil de signature de fichiers (Frank Denis, auteur de
libsodium), basé sur la courbe ed25519. Une *paire de clés*, c'est deux fichiers texte :

| Fichier | Contenu | Qui l'a | Rôle |
| --- | --- | --- | --- |
| `wakfu-overlay.key` (clé **privée**, dite *secrète*) | la clé, **chiffrée par un mot de passe** | le mainteneur seul, et le CI via un secret | sert à **signer** `latest.json` à chaque Release |
| `wakfu-overlay.pub` (clé **publique**) | 2 lignes, ~56 caractères de base64 | tout le monde : commitée dans le dépôt, embarquée dans l'exe | sert à **vérifier** qu'un manifeste a bien été signé par la clé privée |

Ce que ça garantit : un utilisateur dont l'overlay télécharge `latest.json` n'installera jamais
un binaire que le CI du dépôt n'a pas publié — même si GitHub, le DNS ou un proxy intermédiaire
lui servait un faux manifeste, la signature ne correspondrait pas et l'overlay répondrait
`Unavailable` (rien d'installé). La clé privée ne quitte jamais deux endroits : un gestionnaire
de mots de passe du mainteneur et les secrets GitHub Actions. **Jamais dans le dépôt, jamais dans
une session Claude** (une clé qui transite par une conversation est à considérer comme brûlée).

**Génération, pas à pas (sur le poste du mainteneur).** L'outil recommandé est `rsign2`, la
réimplémentation Rust de minisign, même format, installable avec le `cargo` déjà présent :

```bash
# 1. Installer l'outil (une fois)
cargo install rsign2

# 2. Générer la paire, dans un dossier HORS du dépôt (ex. ~/wakfu-overlay-keys/)
mkdir -p ~/wakfu-overlay-keys && cd ~/wakfu-overlay-keys
rsign generate -p wakfu-overlay.pub -s wakfu-overlay.key
#    → demande un mot de passe (deux fois) : c'est celui qui chiffre wakfu-overlay.key.
#      Le choisir long et le ranger dans le gestionnaire de mots de passe AVANT de continuer.

# 3. Vérifier que la paire fonctionne
echo test > test.txt
rsign sign   -s wakfu-overlay.key -x test.minisig test.txt      # demande le mot de passe
rsign verify -p wakfu-overlay.pub -x test.minisig test.txt      # doit afficher : Signature and comment signature verified
rm test.txt test.minisig

# 4. Regarder la clé publique (c'est elle qui sera commitée)
cat wakfu-overlay.pub
#    untrusted comment: minisign public key: XXXXXXXXXXXXXXXX
#    RWQ…………………………………………………………………………………………  ← 56 caractères
```

Équivalent avec le `minisign` d'origine (C) si on le préfère : `minisign -G -p wakfu-overlay.pub
-s wakfu-overlay.key`, `minisign -Sm fichier`, `minisign -Vm fichier -p wakfu-overlay.pub`.

**Où ranger quoi.**

1. **`wakfu-overlay.pub` → dans le dépôt**, à la racine, commité sur `dev` (`chore: clé publique
   de signature des Releases`). Elle est publique par nature ; la phase 2 l'embarque dans l'exe
   par `include_str!("../../../wakfu-overlay.pub")` (`overlay_sync::update::PUBLIC_KEY`), et
   n'importe qui peut vérifier un manifeste à la main avec `rsign verify`.
2. **`wakfu-overlay.key` → deux copies, pas une de plus** :
   - le **gestionnaire de mots de passe** du mainteneur (le fichier en pièce jointe + le mot de
     passe dans la même entrée) — c'est la copie de référence, la seule qui survit à un
     changement de machine ;
   - les **secrets GitHub Actions** du dépôt `Oumbra/wakfu-companion-overlay` : *Settings →
     Secrets and variables → Actions → New repository secret*, deux entrées :
     `MINISIGN_SECRET_KEY` = le contenu **complet** du fichier `wakfu-overlay.key` (les deux
     lignes, collées telles quelles), et `MINISIGN_PASSWORD` = le mot de passe.
   Puis **supprimer** `~/wakfu-overlay-keys/wakfu-overlay.key` du disque : rien ne doit rester
   en clair sur le poste. `*.key` est ignoré par git (`.gitignore`) au cas où le fichier
   traînerait dans le dépôt par erreur.
3. **Ce que fait le CI avec** (phase 1) : `cargo xtask dist` lit les deux secrets depuis
   l'environnement, déchiffre la clé **en mémoire** le temps de signer `latest.json` (crate
   `minisign`, même format), écrit `latest.json.minisig` et n'écrit jamais la clé sur le disque
   du runner. Le secret reste donc chiffré partout où il est stocké.

**Rotation** (si la clé fuit ou si le mot de passe est perdu) : générer une nouvelle paire,
publier une version de l'overlay qui embarque **les deux** clés publiques (signée par l'ancienne,
donc acceptée par les binaires en circulation), puis basculer les secrets sur la nouvelle et
retirer l'ancienne à la version suivante. Un mot de passe perdu sans copie de la clé impose la
même procédure, avec une version intermédiaire signée par… rien : dans ce cas les binaires en
circulation refuseront la mise à jour, et il faudra une réinstallation manuelle unique. D'où la
copie de référence dans le gestionnaire de mots de passe.

**Profil release à ajouter dans `Cargo.toml`** avant la première publication, dans son propre
commit `build:` :

```toml
[profile.release]
strip = true          # symboles : -30 à -50 % de taille
lto = "fat"           # code mort inter-crates
codegen-units = 1
```

**Effet de bord constaté le lendemain (2026-09-16)** : `preview.{ps1,sh}` compilait en `--release`
par défaut, et ce profil a fait passer la recompilation de `overlay-ui` après la moindre édition
de quelques secondes à 5–10 min (`codegen-units = 1` monothread, LTO fat sur toute la chaîne).
D'où le profil **`preview`** du `Cargo.toml` racine (`inherits = "release"`, `lto = false`,
`codegen-units = 16`, `incremental = true`, `strip = false`), désormais le défaut des deux
scripts de prévisualisation — `-Release`/`--release` redonne le vrai profil pour une vérification
finale. Le binaire de Release lui-même n'est pas concerné : `release.yml` garde `--release`.

---

## 7. Côté overlay — architecture

```
                 GET latest.json (+ .minisig)        GET asset (gzip ou patch)
overlay-sync ─────────────────────────────────▶ GitHub Releases ◀──────────────────
   update/                                                                  │ flux vers
   ├─ manifest.rs   parse + minisign-verify + semver                        │ <data_dir>/updates/
   ├─ check.rs      décide : à jour / disponible / obligatoire              │
   ├─ download.rs   fetch_to_file(url, path, on_progress)   ◀──────────────┘
   └─ apply.rs      sha256 → gunzip / qbsdiff → self_replace → relance

overlay-ui
   ├─ background::spawn_update_thread   thread "overlay-update", commandes Check/Download/Install
   ├─ startup::StartupProgress          + étape `update` (Checking → …), liste d'étapes typée
   ├─ panels::login                     liste des étapes + design::meter
   ├─ panels::options_modal             section « Mise à jour » (onglet Paramètres)
   └─ main.rs / bin/overlay-ui-x11.rs   UserEvent::UpdateProgress, OptionsModalAction::InstallUpdate,
                                        sortie propre + relance avec --updated-from
```

**État publié** (`Arc<ArcSwap<UpdateStatus>>`, même motif qu'`AuthStatus`) :

```rust
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate { checked_at: Instant },
    Available { version: String, download_size: u64, mandatory: bool, notes_url: String },
    Downloading { version: String, received: u64, total: u64 },
    Verifying { version: String },
    ReadyToInstall { version: String, staged: PathBuf },
    Installing { version: String },
    Unavailable { reason: String },       // hors ligne, manifeste illisible… : on continue
    Failed { headline: String, detail: String },
}
```

**Thread `overlay-update`** (`background::spawn_update_thread`), commandes reçues par canal comme
le thread Auth : `Check`, `Download`, `Install`, `Cancel`. Il ne touche jamais à l'UI ; il publie
`UpdateStatus` et réveille l'hôte par `UserEvent::UpdateProgress`. Délais : 5 s pour le manifeste
(au-delà, `Unavailable` et le démarrage continue), délai global d'`ureq` levé pour le
téléchargement du corps (borné par le débit, pas par les 10 s du client).

**Téléchargement en flux** : `client::fetch_to_file(url, dest, on_progress)` lit `Content-Length`
et écrit par blocs de 64 Kio dans `<data_dir>/updates/<version>/<asset>.part`, en appelant le
rappel à chaque bloc ; renommé sans `.part` une fois complet et haché. Reprise par en-tête `Range`
si un `.part` existe (GitHub la supporte) — bonus peu coûteux, pas obligatoire.

**Application** (`apply.rs`) :

1. vérifier le SHA-256 du fichier téléchargé contre le manifeste ;
2. produire l'exe cible dans le même dossier de staging : `gunzip` (asset complet) **ou**
   `qbsdiff::Bspatch` avec l'exe courant comme source (delta) ;
3. vérifier `installed.sha256` sur le résultat ;
4. `self_replace::self_replace(&staged)` — sous Windows, l'exe courant est renommé (pas écrasé)
   puis le nouveau copié à sa place ; conserver l'ancien un cycle sous `<data_dir>/updates/previous.exe` ;
5. `logging::log_session_end("mise à jour")`, `Command::new(current_exe()).arg("--updated-from")
   .arg(build_info::VERSION).spawn()`, puis `event_loop.exit()`.

Au relancement, `--updated-from 0.19.0` fait écrire « mis à jour 0.19.0 → 0.20.0 » au journal et
nettoie `updates/`. Un exe dans un dossier non inscriptible (test d'écriture d'un fichier sonde au
démarrage) donne `Failed { headline: "Dossier protégé", detail: "Déplacez l'overlay dans un dossier
où vous pouvez écrire…" }` plutôt qu'un échec au moment de l'installation.

**Mémoire** : pendant l'application d'un delta, l'exe courant (~30 Mo) est en mémoire, le patch
lu en flux, la sortie écrite en flux — pic < 80 Mo, pendant l'écran de chargement où le moteur
n'a pas encore de combat en cours. Sous le budget de 300 Mo.

**Config** (`config.rs`, `#[serde(default)]` comme les autres champs) : `auto_update: bool`
(défaut `true`), `last_update_check: Option<String>`.

---

### 7.1 Ce qui a été construit (2026-09-15)

- `overlay_sync::update::{manifest, download, apply}` — `Manifest::parse_verified` (signature
  AVANT le JSON, schéma 1 seulement), `Manifest::verdict` (`semver`, `minimumVersion` ⇒
  obligatoire), `download::fetch_to_file` (flux 64 Kio, `.part`, SHA-256 au fil de l'eau,
  `Content-Length` pour la progression), `apply::stage` (gunzip + taille + SHA-256 du binaire
  installé), `apply::install_and_relaunch` (`self-replace` puis `--updated-from`),
  `apply::check_writable_install_dir` (sonde avant tout téléchargement),
  `apply::parse_args` (le drapeau n'est jamais pris pour un chemin de log). Clé publique
  embarquée par `include_str!("wakfu-overlay.pub")`. Surcharge `WAKFU_OVERLAY_UPDATE_URL`
  (dossier HTTP local servant la sortie de `xtask dist`) pour rejouer tout le mécanisme sans
  publier.
- `overlay_ui::background::spawn_update_thread` — commandes `Check { install_if_available }` et
  `Download`, anti-rafale de 30 s sur les vérifications manuelles, publication `UpdateStatus`
  au plus dix fois par seconde pendant un téléchargement. L'installation est faite par l'hôte
  (`App::install_update_if_ready`, premier geste de chaque tick) : remplacement, relance, sortie.
- `StartupProgress` : quatrième étape « vérification de mise à jour » (résolue en cinq secondes
  au plus) et drapeau « mise à jour en cours » qui suspend le garde-fou de 45 s et sert aussi à
  ramener l'écran de chargement depuis la fenêtre Options.
- Pas fait, volontairement : re-vérification périodique en jeu (phase 4), entrée dans le menu
  de zone de notification, annulation d'un téléchargement en cours, `previous.exe` conservé
  (`self-replace` gère lui-même l'ancien fichier).

## 8. Flux visuel

### 8.1 Écran de chargement (carte de connexion, fenêtre logicielle 400 px)

> **Retour du mainteneur (2026-09-15)** : la maquette ci-dessous ne convient pas en l'état et fera
> l'objet d'une itération dédiée plus tard. Ce qui suit reste la description du **mécanisme**
> (étapes, jauge, cas d'échec) ; la mise en forme est à reprendre.

Aujourd'hui : logo, titre, séparateur, rouage 72 px, version. Demain, sous le rouage :

```
            ⚙  (rouage, inchangé)

   ✓  Sorts                      (référentiel embarqué, instantané)
   ✓  Catalogue                  (cache · réseau · repli)
   ⟳  Donjons
   ✓  Rattrapage de wakfu.log
   ⟳  Compte
   ✓  Mise à jour · vous êtes à jour

   ████████████████░░░░░░░░  4 / 6
```

- Une ligne par étape, dans un ordre fixe ; glyphe **✓** terminé, **rouage** en cours, **–**
  ignoré (« Mise à jour · vérification impossible (hors ligne) »). La liste est typée
  (`StartupStep` enum + état), plus un `Vec<&str>` : `pending()` devient une vue dessus.
- Sous la liste, un `design::meter` : ratio = étapes terminées / étapes totales. Il donne une
  progression **globale** même sans téléchargement.
- **Si une version est disponible et `auto_update` vaut vrai** (ou `mandatory`), la ligne
  « Mise à jour » devient « Téléchargement de la version 0.20.0 » et la jauge passe en mode
  téléchargement : ratio = reçus / total, libellé « 4,2 / 11,8 Mo ». Puis « Vérification… »,
  « Installation… », et la fenêtre se ferme : l'overlay se relance et repasse par le même écran,
  avec « ✓ Mis à jour vers 0.20.0 ».
- **Si disponible mais `auto_update` faux** : « Mise à jour · version 0.20.0 disponible (Options
  › Paramètres) », étape terminée, on continue.
- Le **garde-fou de 45 s** ne s'applique pas à un téléchargement en cours : `is_complete()`
  ignore l'étape `update` tant qu'elle est en `Downloading`/`Verifying`/`Installing` (un
  téléchargement lent ne doit pas être coupé à mi-course) ; elle a son propre plafond de
  10 min, puis `Failed` et le démarrage continue avec la version courante.
- **Échec** (hachage faux, réseau coupé, signature invalide) : ligne rouge « Mise à jour
  impossible — l'overlay démarre avec la version actuelle », journal détaillé, nouvel essai au
  prochain lancement. Jamais bloquant, sauf `mandatory` : écran « Mise à jour requise » avec
  bouton « Réessayer » (même carte, même anneau rouge que l'état *erreur* de la connexion).
- **Captures** : quatre nouvelles références dans `tests/panels.rs` (`login_chargement_etapes`,
  `login_telechargement`, `login_mise_a_jour_requise`, `login_mise_a_jour_echec`), version figée
  par `freeze_for_snapshots` — et la **version « disponible »** affichée passe par la même
  indirection (`0.0.0` figé), sinon le gate rougirait au premier bump.

### 8.2 Fenêtre Options › Paramètres, nouvelle section « Mise à jour »

Après « Compte », même rythme (`SECTION_GAP`, `heading`, `info_text`, `INFO_GAP`, ligne de
`ROW_HEIGHT`) :

```
Mise à jour
ⓘ Dernière vérification il y a 3 min · vous êtes à jour
☑ Installer automatiquement les mises à jour au démarrage
                     [ Recherche de mise à jour ]
```

La version courante n'est **pas** répétée ici : elle est déjà dans la bannière de la fenêtre
(`design::window(...).version(true)`). Pas de bouton « Notes de version » pour l'instant (aucune
note n'est rédigée aujourd'hui ; `notesUrl` reste dans le manifeste pour plus tard).

Le bouton change d'état avec `UpdateStatus` :

| `UpdateStatus` | Bouton | Ligne d'info |
| --- | --- | --- |
| `Idle` / `UpToDate` / `Unavailable` | « Recherche de mise à jour » (Secondary — habillage à revoir avec le design system, plus tard) | « vous êtes à jour » / « vérification impossible » |
| `Checking` | « Recherche… » (désactivé, rouage 16 px à gauche) | — |
| `Available` | **« Mettre à jour vers 0.20.0 »** (Primary, or) | « version 0.20.0 disponible · 11,8 Mo » (ou « 3,1 Mo en différentiel ») |
| `Downloading`… | (la modale n'est plus là : voir ci-dessous) | — |
| `Failed` | « Réessayer » | `headline` en rouge, `detail` en infobulle |

Clic sur « Mettre à jour » → `design::confirm_dialog("Fermer l'overlay et installer la version
0.20.0 ?")` → `OptionsModalAction::InstallUpdate` → l'hôte : ferme la fenêtre Options et tous
les overlays de jeu (même chemin que « Déconnecter », sans effacer le jeton), passe
`StartupProgress` en mode *mise à jour* (toutes les autres étapes déjà ✓), et `sync_session_windows`
recrée la fenêtre de connexion sur son écran de chargement, où le téléchargement s'affiche comme au
§8.1. Ensuite relance : l'utilisateur revoit l'écran de chargement, puis ses overlays.

Une vérification manuelle ne se lance pas si une est en cours (bouton désactivé), et la commande
`Check` est refusée par le thread en dessous de 30 s entre deux appels (anti-rafale).

### 8.3 Icône de zone de notification (Windows)

**Fait (2026-09-15, demande de l'utilisateur)** : entrée « Mise à jour » juste après « Options »
(menu Options / Mise à jour / Déconnecter / Quitter, `App::install_tray`). Un clic lance la
recherche (`UpdateCommand::Check { install_if_available: false }`, la même commande que le bouton
« Recherche de mise à jour » de la fenêtre Options — même anti-rafale de 30 s, même refus pendant
une opération en cours). Toujours active : une recherche ne dépend pas du compte. Le verdict se lit
dans la section « Mise à jour » de la fenêtre Options ; le menu, lui, ne change pas de libellé.

Non fait, à décider plus tard : un libellé qui suit l'état (« Mettre à jour vers X » quand une
version est disponible, même action que le bouton) — une ligne dans `sync_tray_menu`.

---

## 9. Différentiel — ce qu'il faut savoir avant de s'y engager

- Un binaire Rust recompilé change **beaucoup** même pour un petit correctif (adresses,
  réordonnancement du code) : sur la partie code, bsdiff donne typiquement 20 à 40 % de la
  taille. La partie **données embarquées** (~6 Mo de polices, images, sons, JSON) est identique
  d'une version à l'autre et pèse ~0 dans le patch. Attendu : **patch de 3 à 6 Mo contre 10 à 13
  Mo pour l'asset complet gzip** — un gain réel (×2 à ×3), pas spectaculaire.
- Il faut l'exe **exact** de la version de départ : `fromSha256` le garantit ; sinon asset complet.
- Trois deltas par plateforme (depuis N-1, N-2, N-3) suffisent : au-delà, asset complet. Le CI
  les génère en ~10 s chacun.
- `qbsdiff` applique le patch en flux ; la mémoire reste sous le budget (§7).
- **Décision recommandée** : phase 3, après avoir fait mesurer par le CI, dès la phase 1, la
  taille qu'aurait eu le delta (`xtask dist --measure-delta`, une ligne dans le journal du job).
  Si le gain mesuré est inférieur à ×2, on ne l'implémente pas côté client et on gagne 300 lignes.

Alternative examinée et écartée : `zstd --patch-from` (delta par dictionnaire zstd) — meilleur
ratio que bsdiff sur des binaires, mais exige la crate `zstd` (bibliothèque C) et une fenêtre
mémoire de la taille de l'exe source ; `qbsdiff` reste pure Rust.

---

## 10. Décisions du mainteneur (2026-09-15)

| # | Question | Décision |
| --- | --- | --- |
| 1 | Dépôt public ? | **Oui, les deux dépôts sont publics.** Solution A telle quelle ; `CLAUDE.md` et le §11 du plan d'architecture sont à corriger (phase 0). |
| 2 | Déclencheur de Release | **Push sur `main`.** |
| 3 | Installation automatique au démarrage | **Oui par défaut**, désactivable dans Options. |
| 4 | Différentiel dès le départ ou après mesure | **Après mesure** (recommandation retenue) — voir l'explication ci-dessous. |
| 5 | Emplacement d'installation pour L6 | à acter avec l'installeur (`%LOCALAPPDATA%\Programs\…` recommandé, sans UAC). |
| 6 | `DEFAULT_BASE_URL` | **Un binaire de Release vise toujours la prod, jamais `claude-dev`.** `claude-dev` n'est disponible qu'en local via le script de preview. Conséquence : la première Release attend le déploiement de l'appairage natif en prod ; d'ici là, `release.yml` peut être en place mais la Release publiée n'est pas distribuée. Le binaire compile la base URL de prod par défaut, `WAKFU_COMPANION_API_URL` reste la surcharge de dev. **Précisé le 2026-09-17** : le défaut compilé suit le **profil** (`crates/overlay-sync/build.rs`) — `release` → prod, `preview`/debug/tests → dev — après qu'un `target/preview/overlay-ui.exe` lancé hors du script de preview (donc sans la variable) a visé la prod avec un jeton dev et bouclé sur un 401. La règle « un binaire de Release vise toujours la prod » est inchangée ; c'est le binaire qui la porte, plus le script. |
| 7 | « Bundle moteur mis à jour sans nouvelle version du binaire » | **Retiré** (recommandation retenue) — voir l'explication ci-dessous. |

**Décision 4, ce que les deux options voulaient dire.** « Dès le départ » : on écrit tout de suite
les deux moitiés du différentiel — la génération des patchs par le CI (`xtask dist`) **et** leur
application côté client (`qbsdiff` dans `apply.rs`, le choix patch/complet selon `fromSha256`, le
repli, les tests) — dans le même lot que le client, pour que la toute première mise à jour reçue
par un utilisateur soit déjà un patch. « Après mesure » : en phase 1 le CI ne fait que **calculer
et journaliser** la taille qu'aurait eu le patch entre la Release précédente et la nouvelle (une
ligne dans le journal du job, rien de publié) ; on lit ce nombre sur deux ou trois Releases
réelles, et on n'écrit la partie client que si le gain est au moins ×2 par rapport à l'asset
complet gzip. La différence est le risque de travail perdu : ~300 lignes côté client, une crate,
un chemin d'erreur de plus à tester, pour un gain que je n'ai qu'estimé (×2 à ×3). Et dans les
deux cas un patch ne peut exister qu'à partir de la **deuxième** Release : « dès le départ » ne
fait donc rien gagner sur la première.

**Décision 7, de quoi il s'agissait.** Le « bundle moteur » est `engine.bundle.js` (31 Ko), le
`LogParser` TypeScript du dépôt web compilé et exécuté par QuickJS dans `overlay-engine`. Le plan
d'architecture (§11, et §2.1 point 4) prévoyait qu'une correction de parsing faite côté web soit
publiée comme un **asset séparé** de Release, signé à part, que l'overlay télécharge et charge à
chaud — pour suivre le parser du web sans recompiler l'exe. Deux choses ont changé depuis :
l'exe se met maintenant à jour tout seul (une correction de parser = un commit dans le dépôt
overlay, fusion sur `main`, Release, et l'utilisateur l'a), et le bundle est **vendu et diverge
volontairement** du dépôt web (`engine-js/VENDORED_FROM.txt`, décision du 2026-09-13) — il n'y a
plus de « version du web » à suivre. Garder ce canal aurait coûté un second fichier signé, un
second chemin téléchargement/vérification/chargement, et une matrice exe × bundle à gérer, pour
31 Ko embarqués dans un exe qui se met déjà à jour. Le §11 du plan d'architecture est à amender en
phase 0.

**Interface (même jour)** : section « Mise à jour » sans rappel de la version (déjà dans la
bannière), ligne d'info « dernière vérification il y a X · version Y disponible · Z Mo (différentiel) »,
case « Installer automatiquement… », bouton « Recherche de mise à jour » (habillage à revoir avec
le design system plus tard) qui devient « Mettre à jour vers Y » ; pas de bouton « Notes de
version ». L'écran de chargement est à retravailler dans une itération dédiée (§8.1).

---

## 11. Phasage

| Phase | Contenu | Livrable vérifiable | Dépôt |
| --- | --- | --- | --- |
| **0 — Préalables** ✅ (2026-09-15, sauf la paire de clés, à la charge du mainteneur) | `[profile.release]` ; `DEFAULT_BASE_URL` → prod (décision 6) ; paire `minisign` + secrets ; `CLAUDE.md`/§11 corrigés (décisions 1 et 7) | **Mesuré** sur `overlay-ui-x11` (Linux x86_64, session cloud) : sans profil **49,4 Mo brut / 18,0 Mo gzip**, avec `strip`+LTO+`codegen-units=1` **28,9 Mo brut / 14,0 Mo gzip** (−41 % brut, −22 % gzip ; compilation release 4 min 40 → 6 min 38). L'exe Windows sera du même ordre. | overlay |
| **1 — Publication** ✅ outillage (2026-09-15) — première Release à la prochaine fusion sur `main` | `release.yml` ; `xtask dist` (gzip, SHA-256, `latest.json`, signature + revérification par `wakfu-overlay.pub`, mesure du delta) ; première Release `v0.x` | un exe Windows et un binaire Linux téléchargeables depuis `releases/latest`, manifeste signé vérifiable avec `minisign -V` | overlay |
| **2 — Client** ✅ code (2026-09-15) — validation bout en bout en attente de la première Release | `overlay_sync::update` (`manifest`/`download`/`apply`, 13 tests dont un serveur HTTP local) ; `background::spawn_update_thread` ; `StartupProgress` (étape « vérification », drapeau « mise à jour en cours » qui suspend le garde-fou) ; écran de chargement : ligne d'état + jauge sous le rouage, écran « Mise à jour requise » ; section Options « Mise à jour » (ligne d'info, case `auto_update`, bouton unique) ; `--updated-from` ; captures (4 nouvelles, 2 régénérées) | **Fait ici** : `cargo check` des deux binaires (X11 natif, Windows via `x86_64-pc-windows-gnu`), 220 tests `overlay-ui`, 33 `overlay-sync`, gate de captures vert (67). **Reste à faire sur une vraie machine** : installer une version N-1 et vérifier qu'elle se met à jour et se relance en N — impossible sans Release publiée, et `self-replace` sous Windows n'a pas été exercé depuis ce conteneur Linux. | overlay |
| **3 — Différentiel** | `xtask dist` génère 3 deltas ; `apply.rs` applique `qbsdiff` quand `fromSha256` correspond | même test qu'en 2 avec le journal montrant « delta 3,1 Mo appliqué » ; repli asset complet vérifié sur un exe modifié | overlay |
| **4 — Optionnel** | façade `GET /api/v1/overlay/release` (cache edge, `minimumVersion` pilotable, coupe-circuit) ; bouton « Télécharger l'overlay » sur le site ; re-vérification toutes les 6 h en jeu (badge, jamais d'installation en session) | Function testée sous `wrangler pages dev` ; lien sur le site | web + overlay |
| **L6 — Installeur** (hors de ce plan) | NSIS/MSI par utilisateur, AppImage, raccourci, `AppUserModelID` | installation propre sur machine vierge | overlay |

Chaque phase tient dans une à trois sessions ; 2 est la plus lourde (UI + captures + deux hôtes).
L'ordre est contraint : 1 avant 2 (le client a besoin d'un manifeste réel à lire), 2 avant 3.

---

## 12. Risques et parades

| Risque | Parade |
| --- | --- |
| Antivirus qui bloque le remplacement d'un exe par lui-même | `self-replace` renomme plutôt que d'écrire en place (le motif de Rust `rustup`, `uv`) ; en cas de refus, `Failed` explicite avec le chemin, l'ancienne version reste intacte |
| Exe dans un dossier protégé (`Program Files`) | sonde d'écriture au démarrage → message clair ; décision 5 pour l'installeur |
| Version installée « inconnue » (build local, hachage différent) | pas de delta, asset complet ; jamais d'échec silencieux |
| Manifeste altéré / dépôt compromis | signature `minisign` obligatoire ; sans signature valide, `Unavailable`, jamais d'installation |
| Fuite de la clé privée | rotation : nouvelle clé publique embarquée dans une version signée par l'ancienne, puis bascule ; la clé ne vit qu'en secret Actions |
| Téléchargement coupé | `.part` + reprise `Range` ; sinon nouveau téléchargement au prochain lancement |
| Deux instances de l'overlay pendant une installation | verrou fichier `<data_dir>/overlay.lock` (à vérifier : n'existe pas aujourd'hui, utile indépendamment de ce plan) |
| Mise à jour cassée (l'exe N ne démarre plus) | `previous.exe` conservé un cycle ; procédure documentée « renommez previous.exe » ; option UI en phase ultérieure |
| Gate de captures rouge à chaque bump | version affichée toujours via `build_info::banner_label()` / indirection figée à `0.0.0` |
