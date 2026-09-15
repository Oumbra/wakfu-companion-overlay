# Spike S3 — fenêtre transparente, ancrée, toujours au-dessus, clic-traversant (X11)

Voir [`docs/plan-architecture.md`](../../docs/plan-architecture.md) §6.4/§12/§17.2 (feuille de
route, S3 — reformulé en spike **« implémentation + harnais conjoint »** suite à la revue à trois
experts, §17.6). Question posée : **une fenêtre transparente, toujours au-dessus, traversable par
la souris, ancrée sur une fenêtre de jeu trouvée par titre, rendue avec wgpu+egui sous un rendu
logiciel (lavapipe), est-elle atteignable sous X11 sans piège de composition ?**

**État : validé.** Les trois prérequis de faisabilité posés par la revue à trois experts (§17.2) ont
été vérifiés dans cet ordre, chacun isolément avant d'investir dans le suivant. Ancrage dynamique,
click-through et focus-aware topmost confirmés **programmatiquement** (jamais une déduction depuis
une capture d'écran) via `harness.sh`, qui rejoue tout le scénario de bout en bout et échoue sur la
moindre régression. Réponse à la question posée : **oui, atteignable** — voir §"Validation" pour le
détail, §"Bugs réels trouvés" pour ce qui a fait échouer les premières tentatives.

## Méthode

- `winit` 0.30 (transparence + `_NET_WM_STATE_ABOVE` + `_NET_WM_WINDOW_TYPE_UTILITY` + click-through
  XShape, tout géré nativement côté X11 par winit — voir §"Prérequis n°1" pour ce qui a dû être
  vérifié dans son code source) + `wgpu` 30 (`vulkan` + `gles`, voir §"Prérequis n°1") + `egui` 0.36.
- Découverte de fenêtre de jeu par titre et logique topmost/délai de grâce : `x11rb` direct (pas
  `winit`, qui n'expose aucune de ces deux notions) — voir `src/discovery.rs`/`src/topmost.rs`,
  écrits comme un **portage direct** de `crates/overlay-ui/src/game_window.rs` et
  `App::sync_topmost` (Windows), migrable tel quel vers `overlay-platform::linux::x11` (critère de
  sortie de S3, §12 du plan).
- Sonde d'assertions (`src/probe.rs`) : interroge l'extension **Shape** (click-through),
  `_NET_CLIENT_LIST_STACKING` (ordre d'empilement) et `_NET_ACTIVE_WINDOW` (focus) — jamais une
  capture d'écran comme oracle de test, conformément à la réserve de l'expert X11 (§17.6).
- Orchestration (`harness.sh`) : Xvfb + openbox (+ picom en option) + fenêtre de jeu factice
  (`xterm` titré) + overlay réel + `probe`, guard de nettoyage (`trap cleanup EXIT INT TERM`) qui
  termine tous les process même en cas d'échec d'assertion en cours de route.

Build + test rapide (sans Xvfb, microsecondes) : `cargo test --lib` — couvre `topmost::decide`
(horloge injectable dès l'écriture, voir sa doc). Scénario complet : `./harness.sh
[--with-compositor] [--video]` (voir §"Prérequis système").

## Prérequis système

Aucun n'est optionnel pour `harness.sh` (`--with-compositor` change seulement le scénario testé) :

```
apt-get install --no-install-recommends \
  xvfb x11-utils x11-xserver-utils xdotool openbox picom xterm ffmpeg imagemagick \
  mesa-vulkan-drivers vulkan-tools \
  libxkbcommon-x11-0   # voir "Bug réel n°1" — winit en a besoin même en mode Poll, pas seulement Wayland
```

## Prérequis de faisabilité — vérifiés dans l'ordre, avant tout investissement supplémentaire

### Prérequis n°1 — rendu logiciel Vulkan sous Xvfb (goulot d'étranglement binaire)

Vérifié en DEUX temps, comme demandé par la revue à trois experts (§17.6) : d'abord isolément avec
`vulkaninfo --summary` (confirme `lvp_icd.json`/llvmpipe détecté par la couche Vulkan elle-même),
**puis** avec du vrai code `wgpu` (ce que `vulkaninfo` seul ne prouve pas) :

```
Adaptateur GPU : AdapterInfo { name: "llvmpipe (LLVM 20.1.2, 256 bits)", ..., backend: Vulkan, ... }
```

`Instance::new` avec `Backends::VULKAN | Backends::GL` (le filet `gles` documenté au §17.1 du plan)
plutôt qu'un seul backend forcé : sur cette machine, Vulkan (lavapipe) a suffi, `EGL
'eglInitialize' ... DRI2: failed to load driver` (GL) est un échec attendu et bénin — Xvfb n'a pas
de DRI2 réel, seul l'ICD Vulkan logiciel fonctionne ici.

### Prérequis n°2 — gestionnaire de fenêtres EWMH dans Xvfb

Xvfb seul ne fournit AUCUN WM (confirmé en pratique avant d'ajouter `openbox` : `_NET_CLIENT_LIST`
et `_NET_CLIENT_LIST_STACKING` n'existent tout simplement pas sur la racine, `scan()` renvoie
toujours une liste vide — comportement voulu, voir `discovery.rs`, pas un crash). `openbox` suffit
et est EWMH-conforme (`_NET_WM_STATE_ABOVE`, `_NET_CLIENT_LIST_STACKING` fonctionnent tous deux une
fois lancé). Extension **XTEST** (nécessaire à tout le pilotage `xdotool`) confirmée présente dans
Xvfb par défaut (`xdpyinfo`, 23 extensions listées, XTEST comprise) — vérifiée explicitement à la
demande de la revue plutôt que supposée.

### Prérequis n°3 — scénario sans compositeur par défaut, avec compositeur en secondaire

`harness.sh` sans argument (scénario par défaut) confirme le repli opaque **automatique et
silencieux** documenté au §6.4 du plan — jamais une fenêtre noire inexpliquée : le panneau reste
lisible, seul le fond passe d'un gris-bleu semi-transparent à un noir plein. `--with-compositor`
(picom, backend `xrender` — le seul utilisable sans GPU réel) confirme la vraie transparence ARGB32
: le fond du panneau se mélange visiblement avec la fenêtre en dessous (voir capture jointe à la
session — non commitée, voir `.gitignore`).

## Bugs réels trouvés (pas des suppositions — chacun a fait échouer une exécution avant d'être compris)

1. **`libxkbcommon-x11.so` manquant fait paniquer winit dès `EventLoop::new()`**, avant même la
   création de fenêtre — dépendance système, pas Rust (`apt-get install libxkbcommon-x11-0`).
2. **`TexturesDelta` panique à la première frame en build DEBUG** (`Dropped TexturesDelta with N
   unapplied deltas`) — `full_output.textures_delta.clear()` est OBLIGATOIRE depuis egui 0.36,
   sans quoi le `Drop` panique (`debug_assert!`, invisible en `--release`, ce qui explique que S1
   n'ait jamais rencontré ce piège — `preview.ps1` compile en release par défaut). Reproduit ici en
   conditions réelles avant d'être corrigé.
3. **`global-hotkey` sur X11 (XGrabKey) remonte DEUX événements par pression** (`HotKeyState::
   Pressed` ET `HotKeyState::Released`), contrairement à `WM_HOTKEY` sous Windows (un seul message
   par activation) — un simple `try_recv().is_ok()` ignorant l'état faisait basculer
   interactif/traversant deux fois par appui, annulant l'effet visible. Corrigé en filtrant sur
   `HotKeyState::Pressed`. **À vérifier si ce comportement se manifeste aussi côté production
   Windows** (`overlay-ui::main.rs::about_to_wait`, même motif `try_recv().is_ok()`) — pas
   reproduit ici (spike Linux uniquement), mais le même correctif y serait sans risque.
4. **`xterm -T "titre"` seul ne suffit pas** : le shell interactif lancé par défaut réécrit le titre
   à chaque prompt (séquence OSC), écrasant `-T`. Contourné en lançant `xterm -e sleep 3600`
   (commande statique qui ne touche jamais au titre) — nécessaire pour toute fenêtre de jeu factice
   stable dans `harness.sh`.
5. **`xdotool search --name` interprète le motif comme une regex** : un titre de test contenant des
   parenthèches a fait échouer la recherche silencieusement (chaîne vide, `set -e` a stoppé le
   script). Titres de fenêtres factices choisis sans métacaractère regex dans `harness.sh`.
6. **Table `[workspace]` vide manquante** dans `Cargo.toml` — Cargo (≥ 1.9x) refuse toute commande
   ici en croyant ce crate membre du workspace racine (`members = ["crates/*"]`) bien qu'il en soit
   explicitement exclu. **Même défaut constaté sur `spikes/s1-window-windows`/`s2-engine-quickjs`**
   (préexistant, pas introduit par ce spike, corrigé seulement ici — voir Cargo.toml).

## Validation

`./harness.sh` (scénario par défaut, sans compositeur) et `./harness.sh --with-compositor`
réussissent tous deux intégralement (code de sortie 0), assertions programmatiques :

- **Ancrage** : `overlay.x - jeu.x ≈ GAME_EDGE_MARGIN_PX` (12 px), recalculé et réappliqué dès que
  la fenêtre de jeu factice apparaît — `discovery::GameWindowTracker::scan()` la trouve par son
  titre `"Sagittarius Caecus - WAKFU"` et en extrait le rectangle réel (EWMH `_NET_FRAME_EXTENTS` +
  géométrie).
- **Click-through** : `shape_input_rects` passe de `1` (interactif) à `0` (traversant) à `1`
  (interactif) sur deux pressions du hotkey (`Ctrl+Alt+W`, simulées par `xdotool key` — vraies
  requêtes XTEST, pas un mock) — vérifié via l'extension Shape elle-même, jamais visuellement.
- **Focus-aware topmost** : `stacking_index` reste au sommet tant que la fenêtre de jeu (ou
  l'overlay) a le focus ; passage à une fenêtre non-jeu → pas encore démoté avant `DEMOTE_GRACE`
  (1,5 s), démotion confirmée après ; retour de focus sur la fenêtre de jeu → réaffirmation
  IMMÉDIATE (`topmost::decide`, testé aussi en microsecondes hors Xvfb, voir `src/topmost.rs`).

## Limites assumées (documentées, pas cachées)

- **Une seule fenêtre overlay**, ancrée sur la PREMIÈRE fenêtre de jeu trouvée — pas encore le jeu
  d'ensemble dynamique multi-fenêtres (créer/détruire une overlay par fenêtre de jeu, comme
  `overlay_ui::main::App::sync_windows` le fait déjà pour Windows). Le module `discovery` renvoie
  déjà `Vec<(String, GameWindowInfo)>` (TOUTES les fenêtres trouvées) : le câblage multi-fenêtres
  lui-même reste à porter, pas la découverte.
- **X11 natif uniquement, pas XWayland** — non testé ici (Xvfb est X11 pur, aucun compositeur
  Wayland réel derrière). Limite déjà actée au §17.2 du plan.
- **Panneau de diagnostic minimal** (personnage trouvé, mode, état topmost) — pas les vrais panneaux
  Combat/Suivi de `overlay-ui`.
- **`Box::leak`** (durée de vie `'static`), acceptable pour un spike à une seule fenêtre — la
  production utilise déjà `Arc<Window>` pour des fenêtres à durée de vie dynamique (voir
  `overlay_ui::OverlayWindow::window`), pattern à reprendre tel quel lors de la migration.

## Prochaines étapes (critère de sortie de S3, §12 du plan)

Avant de clore le lot S3 :

1. Migrer `src/discovery.rs`/`src/topmost.rs` vers `crates/overlay-platform/src/linux/x11.rs` (ou
   équivalent) — code déjà écrit comme un portage direct, migration = copie de fichier +
   raccordement au vrai `game_window.rs`/`main.rs` d'`overlay-ui`, pas une réécriture.
2. Étendre `discovery::scan()` en multi-fenêtres réel côté `overlay-ui` (créer/détruire une overlay
   par fenêtre de jeu trouvée, comme `App::sync_windows` le fait déjà pour Windows).
3. Extraire `harness.sh`/`probe.rs` vers des sous-commandes `xtask visual-check` (placement retenu
   pour l'outil de non-régression visuelle permanent, §17.1/§17.2 du plan) — rien ne doit rester
   enterré dans `spikes/` une fois ce lot clos.
4. Vérifier si le bug réel n°3 (`global-hotkey` double événement) affecte aussi la production
   Windows (`overlay-ui::main.rs::about_to_wait`) — filtrer sur `HotKeyState::Pressed` y serait sans
   risque même si non reproduit là-bas.
5. Ouvrir la question du scénario `override_redirect`/fenêtre non gérée par le WM pour
   `stacking_index=-1` (jamais rencontré dans ce spike, `xterm` est toujours géré par openbox) — à
   vérifier si le vrai client Wakfu s'avère un jour se comporter différemment.
