# `overlay-ui` — premier overlay réel (L2)

Fenêtres transparentes Windows (DirectComposition, voir spike S1) affichant deux zones réelles,
alimentées en direct par [`overlay-ingest`](../overlay-ingest/) + [`overlay-engine`](../overlay-engine/)
sur un vrai `wakfu.log` — voir [`docs/plan-architecture.md`](../../docs/plan-architecture.md) §6,
§9, §12.

## Prévisualisation à tout moment

```
cd crates\overlay-ui
.\preview.ps1                          # découverte automatique du wakfu.log
.\preview.ps1 -LogPath <chemin>        # rejouer un fichier précis
.\preview.ps1 -Debug                   # build debug, plus rapide à itérer
```

Prépare `vendor/wgpu-hal-30.0.1` (patch DirectComposition, à la racine du dépôt — voir
`patches/setup-vendor.sh`) s'il est absent, puis `cargo run -p overlay-ui`. `Ctrl+Alt+W` bascule
interactif / clic-traversant (hotkey global, fonctionne sans focus). `Ctrl+Alt+R` force un
rafraîchissement de tous les overlays (redessin + réaffirmation topmost immédiate — voir
`App::force_refresh`, `main.rs`) : demande utilisateur explicite 2026-09-02 pour récupérer un
overlay bloqué (mauvaise taille, plus au premier plan) sans relancer tout le processus. Échap ou
Ctrl+C pour quitter.

## Deux fenêtres overlay indépendantes par personnage

Demande utilisateur explicite (2026-09-01) : Combat et Suivi ne sont plus deux panneaux dans une
même fenêtre, mais deux **fenêtres overlay réellement séparées** (`OverlayKind`, `main.rs`), toutes
deux ancrées sur la même fenêtre de jeu mais positionnées et redessinées indépendamment — pour
permettre à terme de piloter leur visibilité séparément (manuellement ou par un mécanisme
automatique, voir §9 du plan). Elles partagent le même type `OverlayWindow` (position/topmost/
redraw identiques pour les deux) : seul `kind` distingue la taille, l'ancrage et le contenu rendu.

- **Combat** (`src/panels/combat.rs`) — collée au bord GAUCHE de la fenêtre de jeu, centrée
  verticalement (comportement d'origine). Switch Alliés/Ennemis, liste verticale de portraits (de
  classe pour un allié classifié, **icône réelle du monstre** pour un ennemi résolu par le
  catalogue — 2026-09-02, repli générique sinon, voir plus bas), barre de dégâts par combattant,
  icône de connexion au compte. Pas de titre ni de fond opaque (retours utilisateur 2026-09-01) —
  voir la doc de tête du fichier pour le détail des refontes.
- **Suivi** (`src/panels/watchlist.rs`) — collée au bord HAUT de la fenêtre de jeu, centrée
  horizontalement, **largeur pilotée par le CONTENU** (`watchlist_target_width`, `main.rs` —
  recalculée à chaque redessin, jamais plus large que nécessaire pour les entrées actuellement
  affichées, plafonnée par une fraction de la largeur de la fenêtre de jeu réelle). Retour
  utilisateur 2026-09-02, deux allers-retours : 440px fixes étaient bien trop étroits ; une largeur
  fixe plus généreuse (fraction de la fenêtre de jeu) restait un problème dans l'autre sens — une
  fenêtre transparente plus large que son contenu réel reste cliquable/bloquante sur toute sa zone
  (pas de test de transparence par pixel côté Win32), l'utilisateur ne pouvait alors pas deviner où
  s'arrêtait l'overlay. Bande de tuiles carrées à l'image du bandeau du dépôt web
  (`tracker-strip.component`/`.kpi`, **rendu refondu 2026-09-02 pour rester raccord avec le web** :
  bordure/fond en dégradé selon la RARETÉ de l'objet — `overlay_engine::WakfuRarity`, couleurs de
  `styles.css` — bordure grise unie pour un ennemi ; badge de compteur ancré HORS du coin bas-droit
  de la tuile, plus par-dessus l'icône ; valeur courante d'un décompte en couleur kamas, cible en
  gris) : tuiles "+"/"−" reprenant la forme du web mais INERTES pour l'instant (aucun formulaire
  d'ajout ni sélection multiple câblés côté overlay), puis une tuile par entrée suivie — icône
  réelle résolue par le catalogue, repli sur l'icône générique tant qu'elle n'est pas
  résolue/téléchargée ; nom complet en tooltip, badge `"3"` en mode incrémental / `"5/10"`
  compte/cible en mode décompte. Barre de défilement horizontale fine et flottante
  (`ScrollStyle::thin()`, retour utilisateur : le style natif large et gris « passe sur les
  objets », « pas très moderne ») plutôt que la barre native épaisse. Toujours en LECTURE SEULE
  côté définitions (la liste elle-même reste éditée sur le web) ; les compteurs, eux, sont
  incrémentés — et persistés localement — par l'overlay (voir `overlay_engine::watchlist`,
  `docs/plan-architecture.md` §14 point 3). N'apparaît pas tant que le compte ne déclare aucune
  entrée. Le badge déborde de `BADGE_OVERFLOW` (7px, miroir CSS `bottom:-7px; right:-7px`) hors du
  coin bas-droit de sa tuile — réservé à la fois dans `content_width` (largeur de FENÊTRE) et par un
  `ui.add_space` de fin dans le `ScrollArea` lui-même (étendue de DÉFILEMENT, pour le cas plafonné
  par `WATCHLIST_MAX_CEILING`) : sans les deux, le badge de la toute dernière tuile de la bande
  restait tronqué (retour utilisateur 2026-09-02, capture d'écran à l'appui).

**Icônes réelles d'objets/monstres** (`remote_icons.rs`, lot L3 réduit — retour utilisateur
2026-09-02 : icône générique partout, tuiles impossibles à distinguer) : `spawn_catalog_thread`
charge le catalogue (offline-first — cache disque publié immédiatement, rafraîchi en arrière-plan
si `indexHash` a changé, voir `overlay-sync/README.md`) ; `RemoteIconStore`, un thread PARTAGÉ par
toutes les fenêtres, télécharge (repli disque, CDN communautaire `wakassets`) et décode chaque
icône une seule fois même avec plusieurs comptes ; `RemoteIconTextures`, PAR fenêtre, uploade les
octets décodés en texture `egui` (une `TextureHandle` n'est valide que pour SON `egui::Context`) —
utilisé aussi bien par le panneau Suivi que par le panneau Combat (icône de monstre par nom, aucun
id capturé au moment d'un combat contrairement à une entrée de suivi).

**Alertes de drop** (son + toast quand un décompte de suivi atteint 0, §9 du plan) : dès qu'
`overlay_engine::Engine::drain_watchlist_alerts` signale un décompte tombé à 0,
`spawn_engine_thread` joue le son d'alerte (`alert_sound.rs`, `rodio`, sur un thread éphémère
dédié — jamais bloquant pour l'ingestion du log) et publie un toast (`panels::watchlist::
WatchlistToast`) affiché sous la bande de tuiles pendant `TOAST_DURATION` (5 s), non interactif.
Réduit par rapport au web (`loot-alert.component.ts` : confettis, deux sons distincts selon la
raison, durée configurable) — seul le cas `reason: 'countdown'` est câblé, le cas `reason: 'loot'`
(son configurable par objet ramassé) dépend des réglages `profile` du compte, pas encore lus par
l'overlay.

Voir §9 du plan pour le contenu complet visé : **État de synchro** n'est pas encore câblé — le
pairing reste console-only pour l'instant (voir plus bas). **Récap de session**
(kamas/XP/combats/butin) a été retiré (retour utilisateur 2026-09-01 : n'apportait plus rien une
fois le reste simplifié) — sera repensé dans un autre chantier, `overlay_engine::session::
SessionTotals` existe toujours côté moteur. Pas de disposition persistée par écran ni de thème
configurable non plus à ce stade — opacité codée en dur.

## Compte lié (roster + suivi) — lot L4

Au démarrage, un thread `overlay-auth` dédié (`spawn_auth_thread`, `main.rs`) tente de récupérer
les réglages du compte web (`overlay_sync::client::fetch_settings`, un seul `GET /api/v1/settings`
— roster de personnages ET liste des entrées suivies, deux clés du même objet `data`), **jamais
bloquant** pour le reste de l'overlay :

1. Jeton natif déjà stocké (trousseau OS, repli fichier — voir `overlay-sync::token_store`) →
   `GET /api/v1/settings` directement.
2. Sinon (ou jeton devenu invalide) → appairage par code (`overlay_sync::pair_and_wait`) : le code
   et l'URL de confirmation (`<domaine>/pair?code=...`) sont affichés en **console**, le navigateur
   par défaut est ouvert en best-effort. Sans confirmation (ou sans réseau), l'overlay continue
   simplement sans roster ni suivi — repli entier sur `breed` (voir `overlay_engine::class_breed`),
   comme le mode invité du web.

Les réglages récupérés sont poussés à l'`Engine` (thread dédié) via un canal, appliqués de façon
non bloquante entre deux lots de lignes — jamais en attendant dessus. Domaine de l'API configurable
via `WAKFU_COMPANION_API_URL` (utile contre un `wrangler pages dev` local du dépôt
`wakfu-companion`) ; repli par défaut sur `claude-dev.wakfu-companion.com`, **pas** la prod — voir
`overlay-sync/README.md`.

## Ancrage sur la fenêtre de jeu

L'overlay se cale automatiquement au bord gauche de la fenêtre du client Wakfu, verticalement
centré dessus (`GAME_EDGE_MARGIN_PX` dans `main.rs`, actuellement 12 px), et suit tout
déplacement/redimensionnement en continu — voir `src/game_window.rs` et §6.5 du plan pour le
détail (identification par suffixe de titre `" - WAKFU"`, le client étant Java donc son nom de
process n'est pas discriminant). Windows uniquement pour l'instant ; sur les autres OS
`GameWindowTracker::rect()` renvoie toujours `None` (l'overlay reste où winit le place par défaut),
en attendant l'équivalent X11 (S3, différé).

## Validation faite sur ce dépôt

Lancé contre le vrai `wakfu.log` de cette machine, capture d'écran à l'appui : le panneau affiche
de vrais combattants et de vrais nombres (dégâts, XP, kamas) — y compris des valeurs à neuf
chiffres (XP à ce niveau de personnage dans Wakfu, confirmé dans les lignes brutes du log avant de
conclure trop vite à un bug d'agrégation). Basculement clic-traversant confirmé fonctionnel
(`Ctrl+Alt+W`, logué dans les deux sens). Réutilise tel quel le fenêtrage validé par S1
(transparence, always-on-top, `WS_EX_NOACTIVATE`, clamp de redimensionnement) — pas retesté en
détail ici, voir `spikes/s1-window-windows/README.md` pour cette partie.

Ancrage sur la fenêtre de jeu validé contre le vrai client Wakfu (capture d'écran, overlay collé
au bord gauche et centré verticalement) puis en déplaçant la fenêtre de jeu par `SetWindowPos`
pendant qu'`overlay-ui` tournait : la position recalculée était correcte (`jeu.left + 12` en x,
centrage vertical en y) et appliquée en moins d'un tick (50 ms). Au passage, un bug latent trouvé
et corrigé — `render()` retournait tôt (frame `Occluded`/`Outdated`/`Timeout`) sans avoir traité
`textures_delta`, provoquant un panic d'epaint invisible en `--release` (`debug_assert!`) mais réel
en `cargo run` debug ; les deltas sont maintenant traités et vidés (`clear()`) avant toute sortie
anticipée.
