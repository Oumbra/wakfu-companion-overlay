# `overlay-ui` — premier overlay réel (L2)

Fenêtre transparente Windows (DirectComposition, voir spike S1) affichant deux panneaux réels,
alimentés en direct par [`overlay-ingest`](../overlay-ingest/) + [`overlay-engine`](../overlay-engine/)
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
interactif / clic-traversant (hotkey global, fonctionne sans focus). Échap ou Ctrl+C pour quitter.

## Panneaux actuellement affichés

- **Dégâts du combat** (`src/panels/combat.rs`, extrait de `main.rs` pour préparer la séparation
  en zones indépendantes visée au §9) — switch Alliés/Ennemis, liste verticale de portraits de
  classe (nom au survol), barre de dégâts par combattant. Pas de titre ni de fond opaque (retours
  utilisateur 2026-09-01) — voir la doc de tête du fichier pour le détail des refontes.
- **Suivi** (`src/panels/watchlist.rs`) — liste en LECTURE SEULE des entrées suivies déclarées sur
  le compte (nom, marqueur de couleur objet/ennemi, compteur), affichée sous le panneau Combat
  quand le compte en déclare au moins une. Aucune édition possible depuis l'overlay : la liste
  reste éditée sur le web, seuls les compteurs sont incrémentés — et persistés localement — par
  l'overlay (voir `overlay_engine::watchlist`, `docs/plan-architecture.md` §14 point 3).

Voir §9 du plan pour le contenu complet visé : **Alertes de drop** (son + toast) et **État de
synchro** ne sont pas encore câblés — le pairing reste console-only pour l'instant (voir plus bas).
**Récap de session** (kamas/XP/combats/butin) a été retiré du panneau (retour utilisateur
2026-09-01 : n'apportait plus rien une fois le reste simplifié) — sera repensé dans un autre
chantier, `overlay_engine::session::SessionTotals` existe toujours côté moteur. Pas de disposition
persistée par écran ni de thème configurable non plus à ce stade — opacité codée en dur. Toujours
une fenêtre UNIQUE par personnage (pas encore les zones indépendantes/activables séparément visées
au §9 — Combat/Récap/Suivi).

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
