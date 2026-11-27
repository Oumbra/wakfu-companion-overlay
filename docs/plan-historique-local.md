# Plan — historique local, overlay autonome, fin du site applicatif

> Décision du mainteneur du **2026-11-27**. Ce document décrit le virage « tout sur le poste » :
> l'overlay devient l'unique outil, il conserve et affiche lui-même tout l'historique de
> l'utilisateur, sans compte, sans serveur applicatif et sans navigateur. Il **remplace** les
> parties de [`plan-architecture.md`](plan-architecture.md) consacrées à la synchronisation
> serveur, à l'authentification native et au compte obligatoire (§7, §7.5, §9.1 undecies, §10,
> §14) — chacune y porte désormais un renvoi ici.

---

## 0. Pourquoi

- **Coût** : le site applicatif `wakfu-companion` repose sur Cloudflare Pages + Functions et sur une
  base Neon Postgres, payants au-delà des offres gratuites.
- **Double maintenance** : chaque évolution du parseur, du modèle d'historique ou des heuristiques
  (donjons, HDV, pactes) doit être faite deux fois, en TypeScript et en Rust, avec un risque de
  divergence permanent (voir `plan-architecture.md` §2).
- **Objectif utilisateur** : installer Wakfu Companion et s'en servir immédiatement — plus de
  connexion Discord ou Google, plus d'appairage, plus de navigateur.

## 1. Décisions acquises

| Sujet | Décision |
| --- | --- |
| Lieu des données | **Tout reste sur le poste.** Aucune donnée d'historique ne quitte la machine. |
| Stockage local | **SQLite, en clair.** Pas de chiffrement local : ce sont des données de jeu, déjà présentes en clair dans `wakfu.log`, et le chiffrement n'apporterait qu'un coût d'écriture et de lecture. |
| Chiffrement | **Uniquement** pour la future sauvegarde vers un stockage tiers (phase 2, §11) : chiffré avant l'envoi, déchiffré à la récupération. |
| Compte et connexion | **Supprimés** : appairage, jeton, file d'envoi, fenêtre de connexion, synchronisation des réglages. |
| Langue | **Français uniquement.** Le parseur ne lit que des logs en français ; le catalogue embarqué ne garde que les noms français ; aucune internationalisation n'est portée depuis le site. |
| Images du jeu | **wakassets + un petit CDN sur le dépôt GitHub de l'overlay** pour ce que wakassets n'a pas (§5). |
| Interface d'historique | **Une vraie fenêtre autonome**, distincte des fenêtres overlay, navigable comme une application. |
| Périmètre porté | Uniquement ce que l'overlay n'a pas : **historique des combats, récaps, achats, échanges, pactes**. Le suivi, les alertes sonores, les personnages, les alertes de chat existent déjà dans l'overlay et ne sont pas repris du site. |
| « À propos » | Quitte la modale d'options de l'overlay et devient une page de la fenêtre autonome. |
| Pages légales | **Restent sur le site** `wakfu-companion.com` (réécrites pour le nouveau contexte) ; l'application y renvoie par des liens. |
| Site `wakfu-companion.com` | Conservé, réduit à **une page de présentation statique en français**, sans base ni API, hormis la Function de téléchargement (§10). |
| Historique déjà en ligne | **Importé une fois** dans la base locale avant l'arrêt des serveurs (§8). |

## 2. État des lieux (mesuré le 2026-11-27)

### 2.1 Ce que l'overlay a déjà

- Le moteur produit **déjà** tous les événements d'historique : `overlay-engine/src/history.rs`
  (`FightPayload`, `PurchasePayload` — récupération de kamas HDV comprise via
  `__hdv_kamas_sale__` —, `TradePayload`, `PactExtractionPayload`), chacun avec sa signature de
  contenu, livrés par `Engine::drain_sync_events()`.
- Ils partent aujourd'hui dans une **file d'envoi** SQLite (`overlay-sync/src/queue.rs`,
  `rusqlite` bundled) puis sont **supprimés** dès que le serveur les accepte. Rien n'est relu pour
  être affiché ; le moteur ne garde que 20 combats en mémoire (`MAX_TRACKED_FIGHTS`).
- Le regroupement de runs de donjon est déjà porté (`overlay-engine/src/dungeon_run.rs`).
- Dépendances utiles déjà présentes : `rusqlite`, `serde_json`, `ureq`, `image` (png, webp),
  `flate2`, `rfd`, `directories`.
- Design system : `window`, `tabs`, `table` (calquée sur l'historique HDV du jeu), `pagination`,
  `scroll_area`, `collapsible`, `stepper`, `input`, `select`, `tooltip`, `item_slot`, `meter`… —
  `table` et `pagination` ne sont encore utilisées par aucun panneau.
- Le panneau Combat sait déjà dessiner le détail d'un combat (barres, ventilation par sort et par
  élément : `combat_bars`, `combat_spell_block`) ; le bandeau Récap sait dessiner une bande
  d'indicateurs (`panels/recap.rs`).
- **Manquent** : un stockage d'historique persistant, toute interface d'historique, une fenêtre
  normale redimensionnable (seule la fenêtre de connexion n'est pas une fenêtre overlay
  aujourd'hui), et l'indépendance vis-à-vis de la session (catalogue, icônes et référentiels
  exigent un jeton depuis le 2026-09-20).

### 2.2 Ce que le site affiche (à reproduire)

Volumes du code d'historique côté site, hors tests : ~6 500 lignes pour les six vues, ~2 650 pour
les composants partagés, ~2 900 pour les services et utilitaires utiles à l'affichage (le reste,
~1 200 lignes, ne sert qu'à l'envoi). Côté serveur, la partie lecture se résume aux cinq `GET`
d'historique et à `functions/api/v1/history/stats.ts` (548 lignes, 13 requêtes SQL) ; la partie
ingestion recalcule le rattachement aux runs de donjon (`server/history/dungeon-run.ts`, 473 l.)
et le type de combat (`fight-type.ts`, 189 l.).

Le site n'utilise **aucune bibliothèque de graphiques** (barres CSS seulement), n'a **aucun export**
d'historique et **aucune comparaison** entre périodes.

### 2.3 Volumes de données

| Donnée | Taille | Remarque |
| --- | --- | --- |
| Catalogue complet (`wakfu-companion/repository/*.json`) | 8,6 Mo bruts | 11 844 objets, 1 252 monstres, 140 donjons, 156 familles |
| Index compact actuel (4 langues) | 349 Ko gzip | Déjà embarqué en repli (480 Ko) ; français seul ≈ −60 % sur les noms |
| Images wakassets utilisées | ~161 Mo, ~15 000 PNG | `items` 82 Mo, `monsters` 43, `bossIllustrations` 14, `spells` 11, `monsterIllustrations` 9,2, petits dossiers ~2 Mo |
| Images absentes de wakassets | ~430 fichiers, ~3–5 Mo estimés | Détail §5.3 |
| `public/` du site utile à l'overlay | ~300 Ko | En-têtes du récap, brèches, `rarity-base`, `unknown-entity`, quelques pictos |
| Historique local d'un joueur intensif | quelques dizaines de Mo par an (estimation) | **À mesurer** sur un import réel (§12, phase 2) |

## 3. Périmètre fonctionnel

### 3.1 Vues à porter

**Combats** (site : `features/fight-history/`)
- Recherche insensible à la casse et aux accents sur les noms des combattants et du butin ; un run
  de donjon reste entier si l'un de ses combats correspond.
- Regroupement au choix : **par jour** (Aujourd'hui / Hier / Avant-hier / date) ou **par donjon et
  famille** (sections fixes : donjons 2/3/4 salles, 3 joueurs, boss ultime, brèche, brèche ultime,
  arcade, puis familles). Le mode « Origine » (session / compte) du site disparaît : il n'y a plus
  qu'une origine.
- Groupes repliables, état mémorisé ; bouton tout replier / tout déplier.
- Runs de donjon regroupés en une entrée repliable (salles, archimonstre facultatif, salles
  rejouées, tentatives de boss multiples), avec badge de pierre de donjon si elle a été obtenue.
- Ligne de combat : heure (date complète en infobulle), illustration (boss > archimonstre >
  dominant > plus gros dégâts, repli brèche), résultat (gagné / perdu / interrompu), tours et durée.
- Détail déplié : onglets Dégâts / Armure / Soins, total ou par tour, alliés et ennemis avec barre
  proportionnelle, KO, fuite, ventilation par sort et par élément ; butin (kamas, recherche, tri
  A→Z / quantité / rareté) ; XP par personnage. L'en-tête d'un combat déplié reste collé en haut.
- Bandeau « combat en cours » : **non porté** — c'est le rôle du panneau Combat de l'overlay.

**Récap** (site : `features/session-recap/`)
- Granularités : **session, jour, mois, année** (pas de semaine).
- Navigation de période : précédent / suivant, libellés relatifs, sélecteur calendrier (grille des
  jours, des mois, de la décennie). Bornes calculées dans le calendrier local, sans piège
  d'heure d'été.
- Trois modes de détail : **cumulé** (bande d'indicateurs : XP, kamas nets avec ventilation
  combat / ventes HDV / achats / échanges reçus / donnés, combats gagnés–perdus, défis
  réussis–échoués ; XP par personnage ; butin de la période ; pactes), **par donjon et famille**
  (accordéon par section, mini-bande d'indicateurs, XP, butin), **par type de donjon** (les 7 types
  toujours affichés, grisés à zéro, plus « Autres »).

**Achats** (site : `features/purchases/`)
- Recherche (nom d'objet, date), tri récent / ancien, groupes par jour avec total dépensé.
- Achats agrégés par objet dans la journée ; la récupération de kamas HDV est épinglée en tête du
  jour avec l'icône `itemTypes/614`.

**Échanges** (site : `features/trades/`)
- Recherche (partenaire, date, objet), tri, groupes par jour avec nombre d'échanges.
- Chaque échange : « moi ⇄ partenaire », kamas reçus / donnés, colonnes Acquis / Donnés.

**Pactes** (site : `features/pact/`)
- Recherche (date, objet), tri, groupes par jour ; chaque extraction : nombre d'objets et liste.

**À propos** : version, mise à jour, liens (site, code source, CGU d'Ankama, pages légales du site),
soit le contenu actuel de `panels/a_propos_tab.rs`.

### 3.2 Non portés

Tout le reste du site : suivi en direct, alertes, personnages, filtres de chat (déjà dans
l'overlay), profil, personnalisation du tableau de bord et des panneaux, thèmes clairs et modes
daltoniens, onboarding, export RGPD du compte, sessions actives, correction manuelle d'objets
homonymes (à reconsidérer plus tard si le besoin réapparaît), internationalisation.

## 4. Données de référence embarquées

**Fait (phase 0, 2026-11-27).**

- L'outil du mainteneur `tools/referentiel` (son propre workspace, jamais compilé par le CI ni par
  la release) génère `assets/referentiel.json` depuis `wakfu-companion/repository/*.json`, en
  reprenant les règles d'`import-catalog.ts` / `compact-index.ts` (exclusion des anciennes raretés,
  catégorie large déduite de la sous-catégorie, dédoublonnage nom + rareté + gfxId). Contenu,
  **français seul** : objets `[id, nom, gfxId, rareté, a_recette, catégorie]`, monstres
  `[id, nom, gfxId, famille, boss, archi, dominant]`, recettes `[id, [[ingrédient, quantité]…]]`,
  donjons, familles. ~1 Mo de JSON, une entrée par ligne (diffs lisibles), embarqué par
  `include_str!` (`overlay_engine::referentiel`) ; il remplace `catalog-index.json.gz`.
  Commande : `referentiel catalogue --repository ../wakfu-companion/repository --sortie
  assets/referentiel.json`, à relancer quand le dépôt web met son référentiel à jour.
- **Plus aucun appel** à `/api/v1/catalog/*`, `/dungeons`, `/monster-families`, `/items/{id}`,
  `/game-servers`, `/icons/*` : le référentiel suit les releases. `catalog_cache.rs`,
  `reference_data_cache.rs`, `session.rs`, le binaire `gen-catalog-fallback` et l'indicateur
  « catalogue daté » ont disparu ; les quatre anciens fichiers de cache sont supprimés au
  démarrage (`local_data::remove_obsolete_caches`).
- Un seul thread de fond (`background::spawn_referentiel_thread`) construit catalogue et donjons au
  lancement. Les recettes se résolvent localement (`recipe::resolve_recipe(id, &catalog)`).
- `CatalogIndex`, `DungeonIndex` et `MonsterFamilyIndex` ont perdu les colonnes en/es/pt.
- Les serveurs de jeu sont une liste embarquée (`GameServers::embedded` : Pandora, Rubilax,
  Ogrest — mêmes codes que la table `game_servers` du site, pour l'import).

## 5. Images du jeu

### 5.1 Ordre de résolution

**Fait (phase 0, 2026-11-27)**, plus simple que prévu : pas de drapeau de disponibilité dans le
référentiel.

1. Cache disque (`data/icons/…`, `icon_cache.rs`, inchangé — les icônes déjà reçues de l'API
   restent valables, ce sont les mêmes fichiers).
2. **wakassets** (`https://vertylo.github.io/wakassets/{dossier}/{gfxId}.png`), tous les chemins
   candidats de l'icône (`IconRef::image_paths`).
3. **CDN de l'overlay** (`https://oumbra.github.io/wakfu-companion-overlay/…`), mêmes chemins.

`overlay_sync::client::icon_urls` construit cette liste ; `remote_icons.rs` garde sa mécanique
(téléchargements parallèles, cache de textures par fenêtre) et s'arrête à la première réponse. Un
404 sur wakassets ne coûte qu'aux ~350 images que seul le CDN porte, une fois par poste. Les petits
dossiers (`rarities`, `itemTypes`…) passent par le même chemin ; les embarquer reste possible si un
démarrage hors ligne sans cache le demande un jour.

Conséquence pour la politique de confidentialité (§10) : l'adresse IP du joueur et la liste des
icônes qu'il affiche partent désormais vers GitHub Pages (wakassets et le CDN de l'overlay), et non
plus vers l'API du site qui les relayait (constat C10 de `docs/analyse-rgpd.md`). La page
« Confidentialité » réécrite doit le dire.

### 5.2 CDN de l'overlay (GitHub Pages)

- Le dépôt `Oumbra/wakfu-companion-overlay` est public : GitHub Pages est gratuit.
- Un dossier `cdn/` à la racine, avec la même arborescence que wakassets (`items/`, `monsters/`,
  `bossIllustrations/`, `monstersfamily/`…).
- Un workflow `pages.yml` (`actions/upload-pages-artifact` avec `path: cdn/`, puis
  `actions/deploy-pages`) ne publie **que** ce dossier, sur push vers `main` touchant `cdn/**`. Pas
  de branche `gh-pages`.
- URL : `https://oumbra.github.io/wakfu-companion-overlay/{dossier}/{gfxId}.png`.
- Limites GitHub Pages : site ≤ 1 Go, bande passante indicative 100 Go/mois — très au-delà du
  besoin, chaque image n'étant téléchargée qu'une fois par poste.
- Le déploiement suit `main`, donc les releases : catalogue embarqué et images manquantes arrivent
  ensemble.

### 5.3 Ce qui manque à wakassets

Mesuré contre le commit `9159bb1` (2026-07-02, dernière mise à jour du dépôt) :

| Type | Total | Absents | Dont disponibles sur `static.ankama.com` |
| --- | --- | --- | --- |
| Objets | 11 844 | 356 (surtout rares et mythiques) | 159 |
| Monstres | 1 252 | 60 | 5 |
| Illustrations de donjons | 140 | 13 (surtout des brèches) | — |
| Familles | 156 | 2 | — |

Ces images ne sont **affichées nulle part aujourd'hui** : le site comme l'overlay montrent une
icône générique à leur place. Ce sont les contenus les plus récents (objets d'id 32 874 à 33 723 :
décorations de Havre-Enclos et de Havre-Sac, porte-bonheurs, reliquèmes, nouveaux équipements).

**Elles existent dans le client de jeu** (vérifié le 2026-11-27 sur une installation réelle) :
`<installation Wakfu>/contents/gui_jar/gui.jar` (730 Mo) contient `icons/items/64/{gfxId}.tga`
(11 277 icônes), `icons/monsterIllustrations/`, `icons/bossIllustrations/`, `icons/spells/`,
`icons/monstersFamily/`, `icons/aptitudes/`, `icons/timePointBonus/`… — les mêmes dossiers que
wakassets, qui en est manifestement une extraction convertie en PNG.

| Type absent de wakassets | Trouvé dans `gui.jar` |
| --- | --- |
| Objets (356) | **355** |
| Illustrations de donjons (13) | les 2 donjons récents (Hominideux, Verts de vase) ; les 11 brèches utilisent déjà les images de brèche du site |
| Monstres (60) | 14 ; la plupart des autres ont un **gfxId erroné dans le catalogue** (ex. « Bilbyboy Framboise » → `10`), à corriger dans le référentiel |

**Règle (décision du mainteneur, 2026-11-27) : l'overlay ne lit, n'ouvre ni n'extrait jamais
aucun fichier du client de jeu** — seul `wakfu.log` est lu, comme aujourd'hui. Les images
manquantes sont mises à disposition par le **CDN de l'overlay** (§5.2). Leur récupération se fait
**en amont, dans le projet** (outil lancé à la main sur le poste du mainteneur, à chaque patch du
jeu), et le résultat — des PNG — est commité dans `cdn/`. Elle n'a lieu **ni pendant la
compilation** (aucun `build.rs`, aucune étape de CI), **ni pendant l'exécution** de l'overlay.

Commande (poste du mainteneur, après un patch du jeu ou une mise à jour du référentiel) :
`referentiel images --repository ../wakfu-companion/repository --wakassets ../wakassets
--gui-jar "<installation Wakfu>/contents/gui_jar/gui.jar" --sortie cdn`. L'outil réécrit `cdn/`
avec les seules images absentes de wakassets, converties en PNG (350 fichiers, 2,3 Mo au
2026-11-27), et liste celles qu'il n'a trouvées nulle part.

## 6. Stockage local

**Fait (phase 1, 2026-11-27)** : crate `overlay-history`, thread `background::spawn_history_thread`.
Écarts et précisions par rapport au plan initial ci-dessous :

- **Clé d'idempotence** : la signature de contenu du moteur, unique par table. À l'import (§8),
  la signature de chaque ligne du serveur sera recalculée depuis son contenu et vérifiée contre
  sa `clientKey` (`sha256(uid|type|signature)`, uid connu pendant l'import) : un événement déjà
  importé puis relu dans `wakfu.log` ne fera pas doublon.
- **Clé de run de donjon** : la signature du combat représentatif du run (celle que le moteur
  donne déjà), et non `migrated:<id>` comme le serveur quand aucune clé n'existe.
- **Horodatages** en millisecondes UTC (`INTEGER`), pas en texte.
- **Base ouverte le temps d'un lot**, puis refermée : un fichier SQLite ouvert ne se supprime pas
  sous Windows, et « Supprimer les données locales » doit l'emporter. L'historique ne part pas à
  la déconnexion (ce n'est pas une donnée du compte).
- **Pendant la transition**, les événements partent à la fois vers l'historique local et vers la
  file d'envoi au site (inchangée jusqu'à la phase 4).
- **Mesures** (`cargo run --release -p overlay-history --example bench-historique`, 20 000
  combats de 8 participants) : lot de 50 combats écrit en 13 ms, relecture complète sans ajout en
  1,5 s, page de 50 combats en 0,06 ms, agrégat sur la moitié de l'historique en 2 ms, **3,2 Ko par
  combat** sur disque (ventilations par sort comprises) — un joueur à 10 000 combats par an
  occupe ~32 Mo par an.
- **Rejeu** (`crates/overlay-history/tests/rejeu_log.rs`) : dix lancements sur le `wakfu.log` de
  référence, aucun doublon ; victoires, défaites, XP et kamas des combats identiques à ce que le
  moteur a émis. L'XP **hors combat** (métiers) n'est pas stockée, pas plus que sur le site.

### 6.1 Fichier et schéma

- `data/history.sqlite3`, à côté des autres fichiers de données (`app_dirs.rs`). Mode WAL.
- Schéma calqué sur les tables d'historique Neon (`wakfu-companion/server/db/schema.ts`), sans
  `user_id` : `fights`, `fight_participants`, `fight_loot`, `purchases`, `trades`, `trade_items`,
  `pact_extractions`, `pact_extraction_items`. Les ventilations par sort restent en JSON dans
  `fight_participants`, comme en base aujourd'hui.
- Idempotence : contrainte `UNIQUE (kind, signature)` et `INSERT … ON CONFLICT DO NOTHING`. La
  signature produite par le moteur suffit ; `clientKey` (qui mélangeait l'uid) disparaît.
- Index sur les horodatages (`started_at`, `occurred_at`) pour les listes et les agrégats par
  période, sur `dungeon_id` / `dungeon_run_key` / `fight_type` pour les regroupements.
- Versionnement du schéma par `PRAGMA user_version` et migrations dans le code.

### 6.2 Écriture

- Le thread moteur continue de vider `drain_sync_events()` ; un thread d'écriture remplace le
  thread de synchronisation et insère par transactions groupées. Pas de réseau, pas de backoff.
- Le recalcul fait aujourd'hui par le serveur à l'ingestion est porté : rattachement d'un combat à
  son run de donjon sur **tout** l'historique (`dungeon-run.ts`) et type de combat (`fight-type.ts`).

### 6.3 Lecture et agrégats

- Les listes se lisent page par page (curseur sur horodatage + id), jamais en entier : la mémoire
  reste indépendante de la taille de l'historique (budget de 300 Mo inchangé).
- Les agrégats du récap reprennent `stats.ts` en SQL SQLite, avec ses règles à reproduire
  exactement :
  1. exclure tout combat contenant un ennemi de la famille 161 (« Extra Incarnam ») ;
  2. un run de donjon (et ses victoires / défaites) ne compte que les combats contenant un boss ;
  3. famille représentative d'un combat hors donjon : boss > archimonstre > dominant > plus gros
     dégâts ;
  4. ventes HDV et achats séparés par la sentinelle `__hdv_kamas_sale__`.
- La granularité « session » reste calculée en mémoire par le moteur, comme aujourd'hui.

**Fait (phase 3, 2026-11-27) — `HistoryStore::period_stats`** (`overlay-history/src/stats.rs`) :
combats lus avec leurs ennemis et agrégés en Rust, le référentiel embarqué tenant lieu de la table
`monsters` du serveur ; achats, échanges, pactes en SQL. 78 ms sur tout l'historique réel
(1 112 combats). **Comparé au site** (`GET /api/v1/history/stats`, trois périodes : tout, un mois,
un jour) avec l'outil `cargo run -p overlay-history --example stats-periode -- <début> <fin>` :
combats, défis, kamas, XP par personnage, butin, pactes et regroupement par donjon identiques ;
regroupement par famille identique sauf le butin de deux familles, sur des combats où tous les
ennemis ont 0 dégât et appartiennent à deux familles — Postgres (`DISTINCT ON`) départage au
hasard, l'overlay de façon stable (premier ennemi dans l'ordre du log). Écart accepté.

## 7. Fenêtre application

- Fenêtre winit **normale** : niveau normal, icône dans la barre des tâches, redimensionnable,
  taille et position mémorisées. **Dans son propre processus** (décision du 2026-11-27, voir
  ci-dessous), pas à côté des fenêtres overlay.
- Ouverte depuis la modale d'options, le menu de la zone de notification et un raccourci. Un
  paramètre de page (`Combats`, `Récap`, `Achats`, `Échanges`, `Pactes`, `À propos`) permet à
  l'overlay de l'ouvrir directement au bon endroit.
- Navigation par onglets en haut ; pied de page avec liens vers les pages légales du site et la
  mention imposée par la licence des données Ankama : « WAKFU MMORPG : © 2012-[année en cours]
  Ankama Studio. Tous droits réservés. », année calculée.
- **Deux hôtes à câbler** : `main.rs` (Windows) et `bin/wakfu-companion-overlay-x11.rs` (Linux).
- Design system de l'overlay, complété par cinq composants (skill `ui-component`) :
  1. interrupteur segmenté à icônes (le `.icon-switch` du site) ;
  2. en-tête de groupe repliable et collant (jour, donjon, run) ;
  3. sélecteur de période (jours / mois / décennie) ;
  4. liste de butin avec recherche et tri ;
  5. ligne d'échange en deux colonnes (acquis / donnés).
  Le détail d'un combat réutilise le rendu du panneau Combat ; la bande d'indicateurs du récap
  réutilise celle du bandeau Récap.

**Fait (phase 3, en cours — 2026-11-27).**

- **Un processus à part** : `wakfu-companion-overlay --historique[=onglet]`, lancé par l'entrée
  « Historique » du menu de la zone de notification. Il s'ouvre sans le jeu et sans l'overlay, a son
  propre verrou (une seconde ouverture ramène la première au premier plan), ne démarre ni moteur,
  ni compte, ni synchronisation, ni raccourcis, ni icône de notification, et se termine avec sa
  fenêtre. Demande de l'utilisateur (« une fenêtre vraiment en dehors »), et la mesure le
  justifie : le contexte GPU d'une fenêtre (pilote DX12) coûte **~310 Mo privés à lui seul**
  (mesuré en debug par `--autotest-scenario=memoire`), et une fenêtre de l'overlay le gardait à
  moitié (~195 Mo) après sa fermeture. Un processus le rend en entier.
- **Budget mémoire** (décision du 2026-11-27) : 300 Mo pour l'overlay, **500 Mo pour le processus
  d'historique**, soit 800 Mo au pire. Mesures : fenêtre ouverte ~405 Mo privés en debug ; la liste
  des achats entière n'ajoute que 2 à 4 Mo.
- **Achats** : port de la vue du site — recherche, tri, jours repliables, icône de chaque objet
  (wakassets puis CDN) et de l'Hôtel de vente (`itemTypes/614`), liseré et dégradé de rareté,
  « Charger plus » (50 achats, jusqu'au bout du dernier jour) et menu 1 semaine / 1 mois / 1 an.
  Identique au site (empreinte du texte du DOM) sur la partie que le site affiche.
- **Essai automatique** de la vraie fenêtre, invisible et sans la souris : `--autotest=<dossier>`
  (`.claude/rules/captures.md`).
- **Échanges** et **Pactes** : ports des vues du site — recherche (personnage, objet, date), tri,
  jours repliables avec leur nombre d'entrées, cartes « moi ⇄ partenaire » avec kamas et colonnes
  Acquis / Cédés (empilées sous 480 px), extractions avec leurs objets ; « Charger plus ». Les
  briques communes aux trois listes (barre de recherche et tri, en-tête de jour, ligne d'objet avec
  icône et rareté) sont dans `panels/history/items.rs`.
- **Combats** : lecture des combats avec combattants et butin (`HistoryStore::fights`, sans le JSON
  des sorts). Recherche sur les combattants et le butin, groupes par jour, « Tout replier », runs de
  donjon réunis par leur clé de run, résumé (illustration : boss > archimonstre > dominant > plus
  gros dégâts ; Victoire / Défaite ; tours et durée) et détail déplié : alliés et ennemis en dégâts,
  armure ou soins avec barres proportionnelles, butin, XP. **Écart voulu** : le détail est propre à
  la fenêtre, pas celui du panneau Combat (taillé pour une colonne étroite posée sur le jeu).
- **Récap**, mode cumulé : jour, mois ou année dans le calendrier local, précédent / suivant, bande
  kamas nets / XP / combats / challenges (dessinée pour la fenêtre, pas celle du bandeau Récap),
  détail des kamas, XP par personnage, butin et pacte (`HistoryStore::period_stats`).
- **À propos** : les sections d'information et leurs liens, partagées avec l'onglet des Options
  (`a_propos_tab::show_sections`), sans mise à jour, redémarrage ni effacement.
- **Taille et position** retrouvées d'une ouverture à l'autre (`historique-fenetre.json`, à part de
  `config.toml` puisque deux processus tournent), ignorées si la bannière ne tombe plus sur un écran.
- Le symbole ₭ manque dans la police du design system : un montant est toujours suivi de l'icône
  Kamas (`paint_amount`, `paint_amount_left`).
- Reste :
  - Combats : regroupement « Donjons & familles », vue par tour, ventilation par sort (relire le
    JSON des sorts à la demande), portraits de classe des alliés, tri et recherche du butin ;
  - Récap : granularité « session », sélecteur calendrier, modes par donjon et famille et par type ;
  - hôte Linux : le binaire X11 n'a ni le mode `--historique` ni l'entrée de menu ;
  - rafraîchir la fenêtre quand la base change (l'overlay écrit pendant qu'elle est ouverte).

## 8. Import de l'historique en ligne

- Une version de transition propose, dans la fenêtre application, « Récupérer mon historique en
  ligne » : connexion (appairage existant) une dernière fois, puis lecture paginée de
  `GET /api/v1/history/{fights,purchases,trades,pacts}` (`limit=200`, curseur `before`), insertion
  dans la base locale avec dédoublonnage par signature, barre de progression.
- Les GET renvoient déjà `dungeonId`, `dungeonRunKey` et `fightType` : rien à recalculer pour les
  combats importés.
- Une fois l'import fait (ou refusé), le jeton est effacé.
- Le site garde ces endpoints et l'authentification **le temps d'une période d'import annoncée**,
  puis Functions et Neon sont arrêtés.
- Seuls les comptes ont des données sur le serveur : le mainteneur les préviendra par un **envoi
  d'e-mails dédié** (texte à rédiger le moment venu), la durée de la période étant fixée à ce
  moment-là.

**Fait (phase 2, 2026-11-27).**

- **Déclenchement automatique** tant que la connexion existe encore : dès que l'uid du compte est
  connu (`background::activate_sync_queue`), un thread jetable (`spawn_history_import`) importe
  tout, **une fois par compte** (`meta.import_done:<uid>`). Un échec n'est pas noté : la connexion
  suivante recommence, sans doublon. Le bouton « Récupérer mon historique en ligne » et la barre de
  progression viendront avec la fenêtre (phase 3) ; en attendant, l'avancement est journalisé
  (`[import]`). Ainsi, les comptes connectés d'ici la version de transition ont déjà tout en local.
- **Dédoublonnage par la clé du site.** Les GET ne rendent pas la signature d'origine, seulement
  `clientKey = sha256(uid|type|signature)`. Schéma v2 : colonne `client_key` (unique, nullable)
  sur les quatre tables, table `meta`. Avec l'uid connu (`set_account_uid`), chaque ligne lue dans
  le log reçoit sa clé — rattrapage des lignes antérieures compris —, donc :
  - un événement déjà lu dans le log n'est pas réimporté (son rattachement de donjon est seulement
    complété par celui du site, jamais écrasé) ;
  - une ligne importée (signature provisoire `import:<clientKey>`) que le log produit plus tard est
    **adoptée** : elle prend la vraie signature au lieu d'être doublée ;
  - une copie importée avant que l'uid soit connu est fondue dans la ligne locale à la pose de
    l'uid.
- Combats importés : dungeonId, clé de run (`migrated:<id>` comprise) et type gardés tels que le
  serveur les a calculés ; un type absent (anciennes lignes) est complété par les écritures
  suivantes (500 combats par lot au plus).
- Une page = une transaction, base ouverte le temps d'une page (`busy_timeout` de 10 s entre
  l'import et le thread Historique) ; 3 essais par page sur panne réseau, 429 ou 5xx.
- **Import réel (compte du mainteneur, prod, 2026-11-27)** : 1 562 entrées en 4,6 s (1 112 combats,
  423 achats dont 63 récupérations de kamas, 20 échanges, 7 pactes ; 10 août → 26 novembre), base
  de 3,2 Mo, aucune entrée écartée, aucun combat sans type ni combat de donjon sans run.
- **Doublons hérités du serveur** : 9 groupes de combats identiques (même début, même issue, même
  XP ; jusqu'à 6 copies) existent déjà sur le site sous des `clientKey` différentes — signatures
  d'anciennes versions du moteur. Recopiés tels quels (parité avec le site) ; un dédoublonnage par
  contenu pourra se faire dans la fenêtre (phase 3) si on le juge utile.
- **Comparaison avec le site (2026-11-27)** : les quatre GET relus dans le navigateur connecté au
  compte, puis comparés à la base locale. Totaux identiques (combats gagnés/perdus, XP, kamas,
  participants, butin, achats et récupérations de kamas, échanges, pactes et leurs objets), et
  **empreintes ligne à ligne identiques** (SHA-256 des lignes triées : clé, date, issue, XP, kamas,
  donjon, run, type, nombres de participants et de lignes de butin ; mêmes principes pour les
  trois autres tables).

## 9. Retrait du compte et de la connexion

À faire **après** la période d'import (l'import a besoin de la connexion une dernière fois).

| Zone | Contenu retiré |
| --- | --- |
| `overlay-sync` | `pairing.rs`, `token_store.rs`, `session.rs`, `queue.rs`, l'authentification de `client.rs`, `catalog_cache.rs`, `reference_data_cache.rs` ; le crate se réduit à la mise à jour signée et au téléchargement d'icônes |
| `overlay-ui` | `panels/login.rs` (connexion, appairage ; l'écran de chargement ne survit que pour le téléchargement et la vérification d'une nouvelle version — décision du 2026-11-27 —, le démarrage ouvre directement les fenêtres overlay), les threads Auth / Sync / Catalogue / Donjons de `background.rs`, les branches compte de `main.rs`, du binaire X11, d'`engine_thread.rs`, de `render_content.rs`, d'`options_modal.rs`, l'onglet « À propos » de la modale |
| `local_data.rs` | Le périmètre `OnDisconnect` disparaît ; `Everything` reste |
| Moteur | Roster, liste de suivi et profil d'alertes ne viennent plus de `GET /api/v1/settings` : ils sont déjà locaux (`config.toml`), les branches de fusion avec le compte disparaissent |
| Dépendances | `keyring` (et `secret-service` sous Linux) |
| Build | `overlay-sync/build.rs` (base d'API par profil), `WAKFU_COMPANION_API_URL` |

## 10. Site `wakfu-companion.com` (dépôt `wakfu-companion`)

- **Une page de présentation statique, en français**, qui défile : fonctionnalités de l'overlay et
  de la fenêtre historique, captures, lien de téléchargement direct en bas de page, sans connexion.
- **Pages légales** sur le site, réécrites :
  - *Mentions légales* : l'hébergement ne cite plus Neon ; il cite Cloudflare (page statique et
    téléchargement), GitHub (distribution, CDN d'images) et wakassets ; la propriété
    intellectuelle couvre aussi les images publiées sur le CDN de l'overlay.
  - *Politique de confidentialité* : aucune donnée collectée ; tout reste sur le poste (liste des
    fichiers et emplacements) ; seules connexions sortantes : vérification de mise à jour et
    téléchargement d'images, chez GitHub, qui voit l'adresse IP ; notification de tour lue en
    local ; effacement par la purge locale ou la désinstallation.
  - *Conditions d'utilisation* : retrait du compte, de l'âge minimum, de la suppression de compte,
    de l'export du profil et de l'historique conservé sur nos serveurs ; maintien de la
    description passive de l'outil, de la position vis-à-vis d'Ankama, des raccourcis optionnels
    qui tapent dans le chat, de l'absence de garantie, de la responsabilité, de l'évolution du
    service et du droit applicable.
- **Function de téléchargement conservée** (`functions/api/v1/overlay/download/[platform].ts`) :
  elle relaie le binaire gzip avec `Content-Encoding: gzip` et livre un `.exe` directement
  exécutable. C'est la seule Function restante, dans l'offre gratuite de Cloudflare.
- `contact@wakfu-companion.com` reste l'adresse de contact.
- Tout le reste disparaît : Angular, service worker, autres Functions, Neon, workflows RGPD.

## 11. Phase 2 — sauvegarde chiffrée (plus tard)

- Export de la base locale, **chiffré avant envoi** avec une clé dérivée d'une phrase secrète
  (Argon2id) ; déchiffré à la récupération.
- Destination au choix de l'utilisateur (dossier, disque, service de stockage), ou espace géré par
  Wakfu Companion qui ne stocke qu'un blob opaque, lié à un identifiant et protégé par
  authentification forte, sans jamais connaître la clé.
- Conséquence assumée : phrase secrète perdue = sauvegarde illisible.

## 12. Phases

| Phase | Contenu | Critère de sortie |
| --- | --- | --- |
| **0 — Données** ✅ | Référentiel français embarqué (`tools/referentiel`) ; dossier `cdn/` + `pages.yml` ; résolution d'icônes wakassets puis CDN | L'overlay démarre et résout noms et icônes sans jeton ni API |
| **1 — Stockage** ✅ | `history.sqlite3`, écriture depuis le moteur, recalcul runs de donjon et type de combat | Rejeu 10× du même log : aucun doublon ✅ ; totaux identiques au site sur un même log : vérifiés contre le moteur, la comparaison avec le site se fait sur un import réel (phase 2) |
| **2 — Import** ✅ | Import de l'historique en ligne, automatique à la connexion ; bouton et progression dans la fenêtre (phase 3) | Historique d'un compte réel importé ✅ (1 562 entrées, 4,6 s, 3,2 Mo), totaux et lignes identiques au site ✅ |
| **3 — Fenêtre** | Fenêtre application, 5 composants, vues Combats / Récap / Achats / Échanges / Pactes / À propos, Windows et X11 | Parité fonctionnelle avec le site sur le périmètre §3.1 ; captures de référence |
| **4 — Nettoyage** | Retrait compte et connexion (§9), du multilingue du catalogue, mise à jour de `plan-architecture.md` | Plus aucun appel à `wakfu-companion.com` hors liens ouverts dans le navigateur |
| **5 — Site** | Page statique, pages légales réécrites, arrêt des Functions (sauf téléchargement) et de Neon | Coût d'hébergement nul hors domaine |
| Plus tard | Sauvegarde chiffrée (§11) | — |

## 13. Décisions ouvertes

1. ~~Écran de chargement~~ — **tranché le 2026-11-27** : seulement pendant le téléchargement et la
   vérification d'une nouvelle version (§9).
2. ~~Durée de la période d'import~~ — **tranché le 2026-11-27** : fixée lors de l'envoi d'e-mails
   aux comptes existants (§8).
3. ~~Source des images~~ — **tranché le 2026-11-27** : wakassets + CDN de l'overlay (§5) ;
   l'overlay ne lit jamais les fichiers du client de jeu.

## 14. Risques

- **Dépendance à wakassets**, dépôt tiers non mis à jour depuis juillet 2026 : le cache local
  protège les images déjà vues ; en dernier recours, le CDN de l'overlay peut héberger davantage
  de dossiers.
- **Parité des agrégats** avec le site : les règles §6.3 sont subtiles ; les comparer sur un même
  log et un même import avant l'arrêt du site.
- **Deux hôtes** (Windows, X11) pour une fenêtre de plus : coût de câblage doublé, harnais
  `overlay-testkit` à étendre.
