---
paths:
  - "crates/overlay-ui/src/panels/combat*.rs"
  - "crates/overlay-testkit/tests/combat*.rs"
---

# combat-panel

Portée : le panneau Combat de l'overlay (`panels/combat*.rs`) et ses décisions d'affichage, là où
elles diffèrent volontairement du site `Oumbra/wakfu-companion`. Tout nouvel apprentissage sur ce
sujet se documente ici, jamais dans `CLAUDE.md`.

## Ce que l'overlay affiche diffère du site — écart voulu, jamais à « corriger »

Le panneau Combat ne donne une barre chiffrée (colonne de droite) qu'aux combattants ayant produit
au moins 1 point de la grandeur affichée — dégâts, armure donnée ou soins (filtre
`measured.value_of(f) > 0` dans `panels/combat.rs`, voir `CombatMetric::value_of`). Le site
(`Oumbra/wakfu-companion`) fait l'inverse : une ligne par combattant du roster, à zéro comprise,
pour les trois grandeurs.

**Décision explicite de l'utilisateur (2026-09-15) : les deux règles restent telles quelles.**
L'overlay montre le combat EN TEMPS RÉEL, où un combattant à zéro n'apprend rien (on le voit à
l'écran) et n'a pas à consommer une ligne dans un espace compté ; le site est le bilan consulté
APRÈS coup, où « ce combattant n'a rien soigné » est en soi une information. Ne pas aligner l'un
sur l'autre en croyant réparer une incohérence — le filtre équivalent côté web a justement été
RETIRÉ le même jour (`history-archive.service.ts::toFightRecord` : un combat rechargé depuis
l'archive du compte perdait ses lignes à zéro, alors que sa copie de session les gardait, et le
roster changeait donc d'un onglet Dégâts/Armure/Soin à l'autre).

Les portraits (colonne de gauche) ne sont de toute façon jamais filtrés : un combattant à zéro
reste visible dans le cadre, seule sa barre disparaît.
