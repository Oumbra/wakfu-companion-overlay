---
paths:
  - "crates/overlay-ui/src/**"
  - "crates/overlay-testkit/**"
  - "assets/**"
  - ".github/workflows/regen-captures.yml"
  - "scripts/verifier-digest-rendu.sh"
---

# captures

Portée : le gate de captures d'`overlay-testkit` (`egui_kittest`, `tests/snapshots/*.png`, §17.1 du
plan), la régénération des références, et la publication des rendus en artefact. Tout nouvel
apprentissage sur ce sujet se documente ici, jamais dans `CLAUDE.md`.

## Un changement visuel porte ses références régénérées — sinon le CI bloque

Le gate de captures compare 153 PNG sous un rendu Linux **figé**. Une référence produite ailleurs
— sous Windows, sous un autre Mesa — le fait rougir à coup sûr. Le réflexe « je régénère en local »
est donc faux ici, et il n'existe qu'**une** façon d'en produire sans Docker (poste Windows, session
cloud) :

> GitHub → Actions → **« Régénérer les captures (rendu du CI) »** → *Run workflow* sur `dev`.
> (`.github/workflows/regen-captures.yml` ; il réécrit les références, publie l'avant/diff/après en
> artefact, et pousse le commit `test:`.)

**Relire l'artefact fait partie du geste**, il n'est pas facultatif : régénérer, c'est affirmer « ce
nouveau rendu est correct », et git ne sait pas montrer cette affirmation — un PNG modifié s'affiche
`Bin 693015 -> 698936 bytes`. C'est là qu'on attrape le libellé rogné qui allait devenir la
référence pour toujours.

Ce qui arrive quand on l'oublie est documenté : six runs rouges d'affilée et 65 références périmées
du 2026-09-17 au 2026-09-18. `bash scripts/ci-local.sh` (sans option) le dit désormais avant le
push — il compte en échec les écarts trop grands pour du bruit de rastérisation —, et le hook
`pre-push` avertit quand un push touche du code de rendu sans une seule référence.

La version affichée ne doit jamais entrer dans une capture : `build_info::freeze_for_snapshots()`
la fige à `0.0.0` pour tout le harnais (détail dans `.claude/rules/versioning.md`).

## Publier la capture, pas seulement dire que le test passe

Le principe général est dans `CLAUDE.md` (« Rendus visuels — toujours en artefact ») : l'utilisateur
travaille en mode terminal, une image lue inline ne s'affiche pas chez lui. Il s'applique en
particulier aux snapshots du harnais de rendu offscreen — après tout changement visuel dans
`overlay-ui`, publier un artefact reprenant la capture obtenue (page entière et/ou détail recadré
si utile), pas uniquement une ligne de terminal disant que le test passe.

## Essai de la vraie fenêtre sur le poste : invisible, jamais la souris (2026-11-27)

Demande explicite de l'utilisateur : un essai lancé sur son poste **ne met jamais une fenêtre au
premier plan et ne se sert jamais de sa souris ni de son clavier**. Une première passe le faisait
(`SetForegroundWindow`, `SetCursorPos`/`mouse_event`, capture par `CopyFromScreen`) : à ne plus
refaire, même « juste pour vérifier ». Même principe que l'essai automatique de son projet
`velmire` (`VELMIRE_FENETRE=arriere`).

```
cargo build -p overlay-ui --bin wakfu-companion-overlay
target/debug/wakfu-companion-overlay.exe --autotest=<dossier>
```

- La fenêtre d'historique est créée **invisible** (`with_visible(false)`, `with_active(false)`) :
  ni à l'écran, ni dans la barre des tâches. Windows ne lui envoyant aucun `WM_PAINT`, l'hôte
  peint ses frames lui-même (`App::autotest_tick`).
- Le robot (`overlay_ui::autotest`) **injecte des événements egui** (pointeur, clic, texte) dans
  l'entrée de la fenêtre (`GpuState::injected_events`) : le vrai code les traite comme la souris.
- Les captures sont **relues sur le GPU** (`frame::capture_png` : la frame repeinte dans une
  texture hors écran) — jamais lues à l'écran.
- Rien d'autre ne démarre : ni verrou d'instance unique (l'overlay de l'utilisateur peut tourner à
  côté, sans boîte « déjà lancé »), ni moteur, ni compte, ni synchronisation, ni mise à jour, ni
  raccourcis globaux, ni icône de notification, ni fenêtre sur le jeu. La base d'historique est
  une **copie** faite dans le dossier de l'essai.
- Sortie : `journal.txt` et un PNG par étape `Capture` ; code de sortie 1 si une étape échoue
  (fenêtre non fermée par la croix, référentiel absent…). Durée : ~10 s en debug.
- Les coordonnées du scénario (`history_scenario`) sont relevées à 1100 × 820 points : un
  changement de mise en page de la fenêtre doit les mettre à jour.
- Les captures contiennent les données réelles de l'utilisateur : hors dépôt, publiées seulement
  en artefact privé.

**Vérification Linux** sans attendre le CI : WSL (`Ubuntu-22.04`, Rust et dépendances système
installés) — voir `.claude/rules/ci.md`.
