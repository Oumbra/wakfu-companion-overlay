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
