# Test d'ajustement des templates d'overlay combat

Ce dossier documente la vérification de compatibilité entre les portraits de classe
(`assets/class-profile/*.png`, 48×48 px) et les 6 templates d'équipe
(`assets/templates/template_1.png` à `template_6.png`, un par nombre d'alliés de 1 à 6).

## Constat

Les templates originaux (`assets/templates/`) ont des ronds de slot d'environ
**150-155 px de diamètre**, alors que les portraits de classe font **48×48 px**
(un cercle inscrit dans le carré, avec les 4 coins transparents — vérifié sur
plusieurs portraits, alpha=0 dans les 4 coins). Coller un portrait 48×48 sans le
redimensionner dans un tel slot laisserait un large anneau vide autour du portrait :
incompatible avec l'effet « brique de Lego » recherché.

## Correctif validé

Redimensionnement uniforme des 6 templates avec un facteur d'échelle **≈ 0.3116**
(affiné empiriquement à partir de 48/150, puis ajusté après mesure du résultat),
appliqué en conservant le ratio d'origine (chaque template garde sa largeur de
canevas de 255 px, sa hauteur variable selon le nombre de slots).

Résultat mesuré après redimensionnement : rond de slot à **47-48 px** sur les
6 templates (tolérance ±1 px due à l'anti-aliasing du PNG source, imperceptible
à l'écran). Le nombre de slots par template correspond bien à son numéro
(template_N → N ronds).

- `templates-resized/template_1.png` à `template_6.png` : templates redimensionnés
  (facteur 0.3116, ré-échantillonnage bicubique haute qualité).
- `composites/composite_template_1.png`, `_3.png`, `_6.png` : templates
  redimensionnés avec des portraits réels collés dans chaque slot (calque
  portrait dessous, template dessus), pour valider visuellement l'ajustement.
- `composites/composite_template_1_zoom.png` : zoom x6 du composite à 1 slot
  pour inspection pixel par pixel du bord de l'anneau.

## Méthode de collage utilisée pour les tests

Pour chaque slot, le portrait 48×48 est collé au coin haut-gauche de la boîte
englobante mesurée du rond (pas de redimensionnement du portrait), puis le
template est redessiné par-dessus pour que l'anneau masque proprement tout
débordement d'anti-aliasing.

## État — décision à prendre plus tard

Les templates de production (`assets/templates/`) n'ont **pas été modifiés** :
ce dossier ne sert que de preuve de validation. Le remplacement effectif des
6 templates de production par ces versions redimensionnées reste à faire quand
l'intégration dans l'overlay combat sera engagée.
