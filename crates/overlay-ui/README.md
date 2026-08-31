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

- **Dégâts du combat** — combattants du combat en cours (ou du dernier terminé), triés par dégâts
  décroissants, colorés allié (vert) / ennemi (rouge).
- **Récap de session** — kamas nets, XP gagnée, combats gagnés/perdus, butin ramassé.

Voir §9 du plan pour le contenu complet visé : **Suivi (watchlist)**, **Alertes de drop** et
**État de synchro** ne sont pas encore câblés — les deux premiers dépendent de fonctionnalités
qu'`overlay-engine` ne couvre pas encore (persistance, sons), le troisième de la synchro serveur
(L4/L5). Pas de disposition persistée par écran ni de thème configurable non plus à ce stade —
opacité codée en dur.

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
