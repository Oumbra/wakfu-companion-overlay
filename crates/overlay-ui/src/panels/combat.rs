//! Panneau "Dégâts du combat" — extrait de `main.rs::render` (L2), enrichi du portrait de classe
//! de chaque allié (roster déclaré par l'utilisateur, sinon `breed` du combat — voir
//! `overlay_engine::session` pour la cascade, `crate::portraits::PortraitAtlas` pour le rendu).
//!
//! **Refonte 2026-09-01** (retour utilisateur en test réel, capture d'écran à l'appui) : la liste
//! n'affichait jusqu'ici que les alliés, mélangés en dur avec un nom coloré et une barre de
//! progression qui donnaient l'impression d'un bandeau sombre derrière chaque ligne. Remplacée par
//! une liste verticale de PORTRAITS (le nom n'apparaît qu'au survol — tooltip egui standard) + une
//! barre de dégâts par ligne. Switch Alliés/Ennemis au-dessus (`CombatSide`) — l'ennemi n'a pas de
//! portrait de classe résolvable (breed pas déterministe côté ennemi, voir `class_breed.rs`) donc
//! n'était jusqu'ici jamais affiché du tout ; il l'est maintenant avec un portrait générique
//! (`UiIcons::unknown_entity_image`). Ligne "Combat #N — gagné/perdu" retirée (retour utilisateur :
//! n'apporte rien).
//!
//! **Refonte 2026-09-03** (demande utilisateur, redesign complet) :
//! - Camp Alliés : fond décoratif par nombre d'alliés (`crate::panels::combat_frame::CombatFrame`,
//!   médaillons "totem" contenant les portraits de classe, voir sa doc de module) au-delà de 6
//!   alliés (aucun template ne va plus loin), les alliés excédentaires continuent dans le style
//!   "plat" ci-dessous plutôt que de faire échouer l'affichage.
//! - Camp Ennemis, et alliés excédentaires : liste "plate" (portrait + barre par ligne).
//! - Portrait grisé (`FighterDamage::is_ko`, voir `overlay_engine::session`) pour tout combattant
//!   déjà mis KO au moins une fois ce combat — vrai niveau de gris précalculé pour les portraits de
//!   classe (`PortraitAtlas`), simple tint pour les icônes de repli/distantes (`grey_tint_if_ko`,
//!   voir sa doc — pas de version grisée précalculée pour celles-ci).
//!
//! **Refonte 2026-09-04** (retour utilisateur, second redesign) — DÉCOUPLAGE portraits/barres :
//! - Les portraits (cadre ET liste "plate") restent dans l'ordre STABLE de `FightSnapshot::
//!   fighters` (voir sa doc) — ils ne sont PLUS triés par dégâts, pour ne plus changer de position
//!   d'une frame à l'autre. Chaque portrait porte son pourcentage de dégâts en incrustation
//!   (bas-droite, voir `paint_portrait_percent`) plutôt qu'une barre associée par sa position.
//! - Les barres, elles, restent triées par dégâts décroissant, mais forment une colonne
//!   INDÉPENDANTE à droite des portraits (voir `show` ci-dessous) — nom (et désormais dégâts
//!   chiffrés, voir refonte suivante) au-dessus de sa barre (`damage_bar_group`), groupe compact,
//!   sans lien de position avec un portrait précis. Seuls les combattants ayant infligé au moins 1
//!   dégât y figurent.
//! - Infobulle au survol (nom) rétablie sur TOUS les portraits, cadre inclus.
//! - Total de dégâts du camp affiché : replacé en tête de la colonne des BARRES, mis en valeur.
//!
//! **Refonte 2026-09-04 (suite)** — retour utilisateur après capture du premier rendu :
//! - Barre de progression redessinée pour coller à la maquette (capture d'écran 2026-09-02) :
//!   double bordure (gris moyen puis presque noir), piste sombre, remplissage sarcelle/turquoise
//!   avec reflet clair, et un curseur clair fin à l'extrémité du remplissage (voir `damage_bar`).
//!   Colonnes rapprochées (`COLUMN_GAP` réduit) et barre élargie (`BAR_MAX_WIDTH` augmenté) pour
//!   profiter de l'espace regagné.
//! - Régression corrigée : le chiffre de dégâts (pas seulement le pourcentage) réapparaît, sur la
//!   ligne du nom, aligné à droite (nom à gauche, dégâts à droite — voir `damage_bar_group`).
//! - Pourcentage sur le portrait : replacé au coin bas-droit du CARRÉ englobant le portrait (donc
//!   légèrement à l'EXTÉRIEUR du disque visible, sur l'anneau du cadre) plutôt qu'à l'intérieur —
//!   pour ne plus jamais recouvrir un bout du portrait (voir `paint_portrait_percent`).
//! - Tout texte flottant par-dessus le jeu (nom, dégâts, total, pourcentage) utilise désormais un
//!   VRAI contour (8 passes décalées, voir `paint_outlined_text`) plutôt qu'une simple ombre 1px
//!   décalée — même procédé que les incrustations du jeu lui-même (ex. pourcentage de vie), gardé
//!   en référence par l'utilisateur (capture d'écran à l'appui). L'ancienne ombre simple restait
//!   illisible sur certains fonds clairs du décor.
//! - Infobulles repositionnées AU-DESSUS de l'élément survolé (`RectAlign::TOP`, voir
//!   `show_tooltip_above`) — par défaut egui les place en dessous, jugé désagréable par
//!   l'utilisateur (la tooltip apparaît sous le curseur, pas au-dessus du portrait).
//!
//! **Refonte 2026-09-04 (2e retour, après capture du rendu ci-dessus)** :
//! - Barre encore trop "pilule" (arrondi = moitié de la hauteur) : la maquette n'a en réalité
//!   qu'un léger arrondi, pas une forme en stade — voir `BAR_ROUNDING`, remesuré sur une nouvelle
//!   capture de comparaison fournie par l'utilisateur. Hauteur ramenée à 16 px (mesure d'origine,
//!   la précédente l'avait agrandie à 18 sans nécessité).
//! - `show_tooltip_above` ne suffisait pas : par défaut, une tooltip qui ne "tient" pas au-dessus
//!   (pas assez de place) retombe automatiquement en dessous (`Popup::align_alternatives`) — or
//!   c'est justement ce qui arrivait ici pour les portraits proches du haut du panneau. Repli
//!   désactivé (`align_alternatives(&[])`) : au-dessus, TOUJOURS, comme demandé explicitement.
//! - Dégâts chiffrés (total, et chiffre de chaque groupe) au format français : espace insécable
//!   tous les 3 chiffres (`format_fr_thousands`) — sans lui, un grand nombre collé était illisible
//!   d'un coup d'œil (retour utilisateur : « je ne sais pas si c'est onze mille ou cent-treize
//!   mille »).
//! - Pourcentage sur le portrait poussé encore un peu plus vers l'extérieur (voir
//!   `paint_portrait_percent`) et écart nom/barre resserré encore un peu (`GROUP_NAME_BAR_GAP`).
//! - Dégradé de couleur gris/blanc → bleu (`damage_color`) selon la part de dégâts d'un
//!   combattant dans le total du camp affiché — appliqué à la fois au pourcentage sur le portrait
//!   ET au remplissage de sa barre, avec la MÊME référence (`total_damage`, plus `max_damage` :
//!   voir le point suivant) : lecture immédiate de "qui frappe fort" sans lire le chiffre.
//! - **Correctif de cohérence** (retour utilisateur : « le premier a 59 % de dégâts mais sa barre
//!   est pleine, le deuxième a 49 % mais sa barre n'est qu'à moitié — ce n'est pas logique ») : le
//!   remplissage de la barre utilisait `damage / max_damage` (comparatif entre combattants, le
//!   plus gros dégât remplit toujours sa barre à 100 %) alors que le pourcentage affiché utilisait
//!   `damage / total_damage` (part du total du camp) — deux dénominateurs DIFFÉRENTS, d'où
//!   l'incohérence visuelle. La barre utilise désormais `damage / total_damage`, exactement comme
//!   le pourcentage : les deux racontent maintenant la même histoire.
//!
//! **Refonte 2026-09-04 (3e retour, quasiment définitif)** :
//! - Bornes du dégradé `damage_color` remplacées par les couleurs de marque fournies par
//!   l'utilisateur (`#0dbebe` à 0 %, `#e402d8` à 100 %) — plus une estimation "gris → bleu".
//! - Ligne "Total" transformée en ligne "leader" (`show_leader_row`) : le libellé "Total :" est
//!   retiré, le chiffre est aligné à DROITE (même colonne que les chiffres de dégâts de chaque
//!   groupe, pour un alignement vertical cohérent) et un bouton "lien externe" apparaît à sa place
//!   à gauche (icône dessinée au trait, voir `paint_external_link_icon` — même symbole que celui
//!   déjà utilisé par le jeu, capture d'écran de référence à l'appui), qui ouvre le site
//!   (`overlay_sync::client::base_url()`, donc `claude-dev.wakfu-companion.com` en dev comme le
//!   reste de l'overlay, `wakfu-companion.com` en prod — jamais une URL codée en dur séparément).
//! - Espace entre les groupes de chiffres (`format_fr_thousands`) : l'espace INSÉCABLE (U+00A0)
//!   rendait un écart trop discret par rapport au jeu (retour utilisateur, capture de comparaison à
//!   l'appui) — remplacé par une espace ordinaire, plus large dans la police utilisée ici. Sans
//!   risque de retour à la ligne malvenu : ce texte est toujours peint directement via
//!   `Painter::text` (voir `paint_outlined_text`), jamais mis en page par un widget qui pourrait le
//!   scinder.
//! - Curseur "main" (`CursorIcon::PointingHand`) sur tout élément cliquable de ce panneau (switch
//!   Alliés/Ennemis, nouveau bouton lien externe) — rien ne l'indiquait visuellement avant (retour
//!   utilisateur explicite). Réglage global correspondant (tous les boutons standards egui,
//!   `render_content`/`watchlist` compris) posé une fois dans `main.rs`
//!   (`Visuals::interact_cursor`) ; les éléments dessinés à la main ici (`ui.interact` brut, pas un
//!   widget `Button`) ne le reçoivent pas automatiquement, d'où l'appel explicite à
//!   `.on_hover_cursor(...)` à chaque fois.
//! - Police des noms et des dégâts chiffrés agrandie (`NAME_FONT_SIZE`, 11 → 13 px) — jugée trop
//!   petite pour rester confortablement lisible. `TOTAL_FONT_SIZE` remonté en conséquence (16 → 18)
//!   pour conserver un écart de taille net avec le reste (la ligne leader doit rester la plus
//!   visible).
//!
//! **Refonte 2026-09-04 (4e retour, quasiment définitif)** :
//! - Dégradé de `damage_color` retiré : l'utilisateur préfère une seule couleur d'accent fixe
//!   (`#ff02ff`, voir `DAMAGE_ACCENT`) plutôt qu'un dégradé — appliquée telle quelle au pourcentage
//!   sur le portrait ET au remplissage de la barre (même principe de cohérence qu'avant, juste une
//!   seule couleur au lieu de deux bornes).
//! - Écarts resserrés une 3e fois (retour utilisateur : « encore plus compact ») —
//!   `GROUP_NAME_BAR_GAP` (1 px → 0) et `ROW_GAP` (6 px → 4 px).
//! - Bouton lien externe repensé avec un fond, dessiné au trait avec des couleurs mesurées sur la
//!   capture du bouton du jeu — **rejeté au retour suivant** (voir refonte ci-dessous : l'utilisateur
//!   veut la texture RÉELLE, pas un dessin, même fidèle en couleur).
//! - Palette des templates du cadre (`assets/templates/template_*.png`) recolorée une 1re fois par
//!   transformation Teinte/Saturation/Valeur — **également rejetée** (voir refonte ci-dessous :
//!   cible mal calée, résultat trop sombre et bordures/décorations reteintées alors qu'elles ne
//!   devaient pas l'être).
//!
//! **Refonte 2026-09-04 (5e retour)** :
//! - Bouton lien externe : le dessin au trait (aussi fidèle soit-il en couleur) reste « une autre
//!   icône » aux yeux de l'utilisateur — demande réitérée, sans ambiguïté cette fois : utiliser la
//!   texture RÉELLE du jeu. Extraite de la capture d'écran par différence de couleur avec son fond
//!   (composante connexe la plus grande pour ignorer le bruit de compression, script Python, pas
//!   reproduite ici) puis recadrée — PAS redessinée, PAS recolorée. Voir `crate::ui_icons::UiIcons::
//!   external_link` pour le chargement (même mécanisme que les icônes `allies`/`enemies`
//!   existantes). Le libellé "Détails" peint en clair à côté est retiré : c'était en réalité le
//!   texte de l'infobulle au survol (l'utilisateur l'avait mal compris comme un libellé permanent),
//!   corrigé en conséquence — voir `show_tooltip_above(&response, "Détails")`.
//! - Palette des templates : premier essai rejeté (« ne correspond pas du tout ») — cible recalée
//!   sur une nouvelle capture de comparaison, avec des points de mesure MULTIPLES cette fois (corps
//!   du panneau, anneaux de médaillon, décorations) plutôt qu'un seul point pris au hasard : la
//!   cible précédente (un point pris dans une zone d'ombre du panneau) donnait un résultat bien
//!   trop sombre. Nouvelle cible : valeur ×0.88 (contre ×0.52 avant — beaucoup plus proche de
//!   l'original, l'utilisateur ayant précisé que le CONTENEUR du jeu est plus clair, pas plus
//!   sombre), saturation ×1.55. Les 6 fichiers ont été restaurés depuis `git` avant cette 2e passe
//!   (la 1re tentative, rejetée, n'était pas commitée — jamais recolorer une image déjà recolorée,
//!   l'erreur s'accumulerait).
//! - Écarts resserrés une 4e fois — `ROW_GAP` (4 px → 2 px) et `TOTAL_GAP` (6 px → 4 px, cohérence
//!   avec `ROW_GAP`) : retour utilisateur répété, l'écart entre groupes restait « non négligeable ».
//!
//! **Refonte 2026-09-04 (6e retour)** :
//! - Couleur d'accent (`DAMAGE_ACCENT`) retirée : retour utilisateur explicite (« je ne suis pas
//!   convaincu, on va revenir sur la couleur accent du site, le bleu Wakfu ») — le pourcentage sur
//!   le portrait ET le remplissage de la barre utilisent maintenant `ACCENT` (même bleu que le
//!   switch Alliés/Ennemis, charte reprise du dépôt web), plus de couleur dédiée aux dégâts.
//! - Bouton "lien externe" reconstruit une 2e fois : l'utilisateur a cette fois fourni lui-même les
//!   DEUX assets nécessaires (`assets/button-background.png` : le socle générique d'un bouton icône
//!   du jeu, et `assets/external-link.png` : l'icône à fond transparent à centrer dessus) plutôt que
//!   de me laisser extraire une texture unique d'une capture d'écran (ancienne approche, 5e retour).
//!   `paint_icon_button` compose les deux à leur taille NATIVE, sans redimensionnement (demande
//!   explicite : « pose-le tel quel pour l'instant, histoire que je vois si c'est cohérent ») — et
//!   reste un vrai petit composant réutilisable (demande explicite), même si `show_leader_row` en
//!   est pour l'instant l'unique appelant.
//! - Fond opacifié derrière la ligne leader (bouton + total, voir `show_leader_row`) : rectangle
//!   arrondi semi-transparent (`LEADER_PANEL_FILL`) pour séparer visuellement cette ligne du reste
//!   de la colonne et mettre en valeur le total — demande utilisateur, captures d'écran du jeu à
//!   l'appui (bandeau translucide derrière la barre de discussion). Les captures fournies sont des
//!   rendus déjà composités (pas de canal alpha exploitable pour mesurer précisément la
//!   transparence réelle) : couleur et opacité ici sont une approximation raisonnable du même
//!   principe visuel, pas une mesure pixel par pixel — à affiner si le rendu en jeu détonne encore.
//! - Écarts resserrés une 5e fois : `ROW_GAP` (2 px → 1 px) et le rembourrage vertical sous le nom
//!   dans `damage_bar_group` (2 px → 0, la barre touche directement le bas du texte) — retour
//!   utilisateur répété, l'écart entre la barre et la ligne nom/dégâts au-dessus restait visible.
//! - Palette des templates recalculée une 3e fois, méthode changée : au lieu d'un décalage
//!   Teinte/Saturation/Valeur UNIQUE pour toute l'image (les 2 tentatives précédentes), la
//!   transformation est désormais calculée SÉPARÉMENT pour le corps du panneau (zone claire) et les
//!   décorations (zone sombre), avec un fondu entre les deux sur une bande de luminosité
//!   intermédiaire — l'utilisateur avait explicitement décrit ces deux zones comme des couleurs
//!   très différentes (« le contenu... marron marbré » vs « les décorations... des bordures noires,
//!   limite »), ce qu'un décalage global ne pouvait pas reproduire. Cibles mesurées par moyenne de
//!   pixels (script Python, pas reproduit ici) sur deux nouvelles captures retravaillées par
//!   l'utilisateur pour isoler les couleurs (corps ≈ (103, 95, 77), décorations ≈ (46, 50, 52)) ;
//!   les 6 gabarits ont d'abord été restaurés depuis `git` (la 2e tentative, rejetée, n'était pas
//!   commitée).
//!
//! **Refonte 2026-09-05 (7e retour)** :
//! - Bouton "lien externe" réduit à `ICON_BUTTON_SIZE` (24×24, contre 37×37 natif) — le socle ET
//!   l'icône sont maintenant mis à l'échelle (dans le MÊME ratio, voir `paint_icon_button`) plutôt
//!   que peints tels quels : la version "taille native" du retour précédent n'était qu'une étape
//!   pour juger des proportions, pas la taille finale voulue.
//! - État survolé du bouton (`button-background-hover.png`, fourni par l'utilisateur, socle
//!   éclairci) — absent jusqu'ici : le socle ne changeait pas visuellement au passage de la souris,
//!   seul le curseur changeait de forme.
//! - Palette des templates, 4e passe, méthode encore changée : le corps garde la même
//!   transformation Teinte/Saturation/Valeur que la 3e passe (jugée "ok" par l'utilisateur, même
//!   si "pas encore totalement satisfait" — non retouchée ici) mais les décorations reprennent
//!   désormais leurs pixels ORIGINAUX tels quels (transformation identité), plutôt qu'une 2e cible
//!   de couleur calculée : le rendu recoloré des décorations restait rejeté (« ce n'est pas ça »).
//!   Conséquence organisationnelle : `assets/templates/` (racine du dépôt, restauré par
//!   l'utilisateur lui-même via `git checkout`) sert maintenant de RÉFÉRENCE PIVOT non recolorée
//!   pour les décorations, tandis que `crates/overlay-ui/assets/templates/` (la copie embarquée,
//!   seule à être réellement affichée en jeu) est la SEULE à porter le corps recoloré — les deux
//!   copies ne sont donc plus des miroirs identiques l'une de l'autre, contrairement à la
//!   convention suivie jusqu'ici pour ce dossier.
//! - Pourcentage sur le portrait agrandi (`PERCENT_FONT_SIZE`, 10 → 12 px) — jugé difficile à lire.
//! - Fond opacifié de la ligne leader : retour utilisateur sur une asymétrie perçue (le bouton
//!   semblait plus loin du bord gauche que le dernier chiffre du bord droit) — `LEADER_PANEL_PADDING`
//!   était et reste STRICTEMENT identique des deux côtés (même valeur utilisée pour les deux, voir
//!   `show_leader_row`) ; l'écart perçu vient très probablement du glyphe du texte (l'encre visible
//!   du dernier chiffre ne remplit pas toute la largeur d'avance que lui réserve la mise en page),
//!   pas d'un vrai déséquilibre géométrique — non corrigé ici en l'absence de mesure fiable, à
//!   confirmer sur le nouveau rendu.
//!
//! **Refonte 2026-09-05 (8e retour)** :
//! - Couleur `DAMAGE_ACCENT` (`#077982`) réintroduite pour la barre de dégâts et le pourcentage sur
//!   le portrait — de nouveau DISTINCTE du bleu `ACCENT` du switch (les deux avaient été fusionnées
//!   en 6e retour après rejet du magenta du 5e retour ; nouvelle couleur cette fois, pas un retour
//!   à l'ancienne).
//! - Tooltip du switch Alliés/Ennemis qui s'affichait SUR les boutons (les rendant incliquables,
//!   retour utilisateur avec capture) — voir `show_tooltip_above` : repli `RectAlign::BOTTOM` ajouté
//!   pour le seul cas où `TOP` ne tient RÉELLEMENT aucune place (le switch, collé au bord supérieur
//!   de la fenêtre, `inner_margin` nul).
//! - Palette des templates, 5e passe, méthode encore changée (4e passe jugée insuffisante :
//!   « les changements de couleur sont toujours trop uniformes... les décorations sont les mêmes
//!   couleurs que les couleurs de fond ») : le corps garde la même transformation Teinte/Saturation/
//!   Valeur que les passes précédentes, mais l'ornement rouages/volutes du haut et du bas de chaque
//!   template n'est plus ni recoloré ni gardé identique — ses pixels sont désormais RÉCUPÉRÉS
//!   DIRECTEMENT depuis une capture d'écran en jeu fournie par l'utilisateur (véritable rendu du
//!   jeu, donc la seule source qui contient la nuance réelle des deux matières — le PNG "template"
//!   plat n'a jamais encodé cette information, une transformation de couleur ne pouvait donc
//!   structurellement pas la faire apparaître, d'où l'échec des passes précédentes), MIROIR
//!   horizontal (l'ornement est accroché au bord opposé sur la capture de référence) puis
//!   redimensionnés sur la zone ornement (mesurée, coin haut-gauche et bas-gauche de chaque
//!   template, 38×38 px) — voir le script de recolorisation (non versionné, hors dépôt). La zone
//!   ornement elle-même reste délimitée par seuil de luminance (comme les passes précédentes) pour
//!   ne pas empiéter sur le coin de l'anneau qui partage cette même case ; seules les jointures
//!   simples entre médaillons (templates à 2+ alliés) restent en transformation identité, faute de
//!   référence en jeu pour cette zone précise.
//!
//! **Refonte 2026-09-05 (9e retour)** :
//! - Couleur de la barre de dégâts VALIDÉE (`DAMAGE_ACCENT`, `#077982`, inchangée) mais le
//!   pourcentage sur le portrait REVIENT à `ACCENT` (bleu Wakfu du switch) — l'utilisateur préfère
//!   l'ancienne couleur pour le pourcentage. Barre et pourcentage n'ont donc À NOUVEAU plus la même
//!   couleur, explicitement voulu cette fois (contrairement au 6e retour où c'était un rejet du
//!   magenta, pas une préférence pour deux teintes distinctes).
//! - Reflet du tiers supérieur de la barre (`BAR_HIGHLIGHT`, `#0dbebe`) : couleur EXPLICITE fournie
//!   par l'utilisateur, remplace l'ancienne dérivation `lighten(fill_color, 0.4)` (retirée avec
//!   `lerp_color32`, plus aucun appelant) — l'utilisateur veut CE ton précis pour l'effet de volume,
//!   pas un simple éclaircissement automatique de la couleur de remplissage.
//! - Templates : toujours pas satisfaisant pour l'utilisateur (5 passes tentées). Il compte fournir
//!   les éléments détourés à la main dans un prochain retour plutôt que de laisser une nouvelle
//!   passe automatique être tentée sur les captures brutes — AUCUN changement de template ici,
//!   en attente de ces fichiers.
//!
//! **Refonte 2026-09-05 (10e retour)** : le repli du tooltip ajouté au 8e retour ne corrigeait le
//! bug (tooltip écrasée sur le switch) QUE pour "Ennemis" — "Alliés" restait cassé (retour
//! utilisateur avec capture). Cause : `RectAlign::BOTTOM`, seul repli listé, centre la tooltip sous
//! l'élément ; pour "Alliés", collé au bord GAUCHE de la fenêtre, une tooltip centrée déborde à
//! gauche (x négatif) et `find_best_align` (voir sa doc — exige un rectangle ENTIÈREMENT contenu)
//! rejette aussi ce repli, retombant sur `TOP` (le tout premier choix, donc écrasé). Deux replis
//! supplémentaires (`BOTTOM_START`, `BOTTOM_END`, bord aligné au lieu de centré) couvrent ce cas —
//! voir `show_tooltip_above`, dont la doc corrige aussi une erreur d'un retour précédent sur ce que
//! fait réellement `RectAlign::BOTTOM_START`.
//!
//! **Refonte 2026-09-05 (11e retour)** : nouveau bouton "Options" (icône `nut.png` fournie par
//! l'utilisateur, écrou/rouage — à l'origine doré, RECOLORÉ au 12e retour, voir `UiIcons::
//! options_icon`) au bas du panneau, dans une nouvelle barre d'outils (`bottom_toolbar`) — n'ouvre
//! encore aucun panneau (réservé à une future page de réglages), infobulle "Options" au survol dès
//! maintenant. L'ancien bouton lien externe (`paint_icon_button`, ouvre la web app) est déplacé de
//! la ligne leader vers cette même barre, juste AVANT (à gauche du) le nouveau bouton Options. La
//! place qu'il laisse dans la ligne leader (`show_leader_row`) est prise par le switch Alliés/
//! Ennemis (`paint_side_switch`, ex-`side_switch`) — retour utilisateur explicite : « je trouve que
//! c'est un meilleur emplacement que là où est le switch actuellement ». L'ancienne rangée pleine
//! largeur du switch, en tête de panneau, disparaît donc.
//!
//! Pour que le switch reste TOUJOURS accessible (y compris combat vide ou camp affiché sans
//! combattant — sans quoi un utilisateur basculé sur un camp vide n'aurait plus aucun moyen de
//! revenir en arrière, le switch ayant disparu avec le reste de la ligne leader), `show` est
//! restructurée : la ligne leader et la colonne de droite sont désormais TOUJOURS peintes, un
//! message d'état ("Aucun combat pour l'instant.", "Aucun allié/ennemi pour l'instant.") remplaçant
//! simplement les groupes nom+barre quand il n'y a rien à afficher, plutôt qu'un retour anticipé de
//! la fonction qui escamotait tout, switch compris.
//!
//! **Refonte 2026-09-05 (12e retour)** : l'icône Options d'origine (doré/orangé/brun aux yeux de
//! l'utilisateur) détonnait à côté de l'icône lien externe (blanche) dans la même barre d'outils —
//! retour utilisateur explicite : « j'aimerais que tu lui appliques la même couleur que l'icône de
//! external link ». Recolorée en `#fbfbfb` (couleur de remplissage mesurée sur
//! `external-link-icon.png`, hors son léger contour sombre) : seul le canal RGB des pixels non
//! totalement transparents change, l'alpha (donc la silhouette et son anticrénelage) reste
//! identique au fichier fourni par l'utilisateur.
//!
//! **Refonte 2026-09-05 (13e retour)** : `assets/templates/` à la racine du dépôt (original non
//! recoloré des templates, gardé un temps après le nettoyage des assets — voir `combat_frame`)
//! retiré à son tour — retour utilisateur explicite : « je les ai déjà autre part, donc ils ne
//! servent à rien ici ». L'utilisateur conserve cet original HORS du dépôt ; la seule copie qui y
//! reste est la version recolorée sous `crates/overlay-ui/assets/templates/`, seule affichée en jeu.

use overlay_engine::{CatalogIndex, FightSnapshot, FighterDamage};

use crate::portraits::PortraitAtlas;
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::ui_icons::UiIcons;

use super::combat_frame::{CombatFrame, MAX_FRAME_SLOTS};

/// Camp actuellement affiché dans la liste verticale de portraits, piloté par le switch
/// (`paint_side_switch`). Un état par fenêtre overlay (donc par personnage) — voir `OverlayWindow`
/// dans `main.rs`, pas un état global : rien n'empêche de vouloir regarder les ennemis d'un
/// personnage pendant que la fenêtre d'un autre reste sur ses alliés.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CombatSide {
    /// Vue par défaut (demande utilisateur explicite) : c'est aussi la seule où le portrait
    /// apporte une vraie information tant que les ennemis n'ont pas leur propre illustration
    /// (repli générique pour l'instant, voir `UiIcons`).
    #[default]
    Allies,
    Enemies,
}

const SWITCH_HEIGHT: f32 = 26.0;
const SWITCH_OPTION_WIDTH: f32 = 30.0;
const SWITCH_ICON_SIZE: f32 = 15.0;
/// Écart vertical entre deux portraits de la liste "plate", et entre deux groupes nom+barre de la
/// colonne de droite (même rythme pour les deux colonnes, demande utilisateur explicite). Resserré
/// une 4e fois (6 px → 4 px → 2 px → 1 px, retour utilisateur répété : « il y a un écart non
/// négligeable entre les groupes, il faut le réduire »).
const ROW_GAP: f32 = 1.0;
/// Écart horizontal entre la colonne des portraits (cadre ou liste plate) et celle des barres —
/// réduit (retour utilisateur 2026-09-04 : « moins écartés, un peu plus proches ») par rapport à
/// la première version de cette refonte (12 px).
const COLUMN_GAP: f32 = 6.0;

// Charte reprise telle quelle du thème sombre par défaut du dépôt web (`styles.css` `:root`, voir
// `.icon-switch`/`.icon-switch-highlight`) — pas de palette propre à l'overlay pour ce composant.
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x00, 0xd2, 0xff);
// Couleur dédiée au remplissage de la barre de dégâts UNIQUEMENT (retour utilisateur 2026-09-05,
// 8e retour : `#077982`, un sarcelle plus sombre que `ACCENT`) — DE NOUVEAU distincte de `ACCENT`
// (fusionnées en 6e retour après rejet du magenta `#ff02ff` du 5e retour). Le pourcentage sur le
// portrait, lui, est revenu à `ACCENT` au 9e retour (« je préfère la couleur accent qu'il y avait
// avant ») — barre et pourcentage n'ont donc plus la même couleur, à nouveau explicitement voulu.
const DAMAGE_ACCENT: egui::Color32 = egui::Color32::from_rgb(0x07, 0x79, 0x82);
const TINT_MEDIUM: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 31);
const TINT_STRONG: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(255, 255, 255, 46);

/// Hauteur d'une barre de dégâts — mesurée sur la maquette fournie par l'utilisateur (capture
/// d'écran 2026-09-02, ~16 px de haut). Une première itération l'avait portée à 18 px sans
/// nécessité (retour utilisateur : « elle est plus haute que celle que je t'ai fournie ») — revenu
/// à la mesure d'origine.
const BAR_HEIGHT: f32 = 16.0;
/// Arrondi des coins de la barre — PAS `hauteur / 2` (un stade/pilule complet, ce qu'une première
/// itération avait fait) : la maquette n'a qu'un arrondi léger (retour utilisateur, capture de
/// comparaison à l'appui : « le border radius est beaucoup trop rond dans ce que tu as produit »).
const BAR_ROUNDING: f32 = 4.0;
/// Largeur maximale d'une barre — agrandie par rapport à la première version de cette refonte
/// (150 px) maintenant que `COLUMN_GAP` est réduit (voir sa doc) : l'espace regagné doit profiter
/// à la barre, pas rester vide.
const BAR_MAX_WIDTH: f32 = 190.0;
/// Épaisseur de chacune des deux bordures concentriques de la barre (voir `damage_bar`) — mesurée
/// sur la maquette (~2 px sur une barre d'environ 16 px de haut).
const BAR_BORDER_WIDTH: f32 = 2.0;
/// Écart entre le nom (+ dégâts) et sa barre, DANS un groupe (voir `damage_bar_group`) —
/// volontairement plus petit que `ROW_GAP` (qui sépare deux groupes ENTRE eux) : c'est cette
/// différence de rythme qui donne à l'œil la lecture "un nom + une barre = un groupe". Resserré une
/// 3e fois (2 px → 1 px → 0, retour utilisateur répété : « encore plus compact ») — le nom/les
/// dégâts touchent maintenant directement le haut de la barre.
const GROUP_NAME_BAR_GAP: f32 = 0.0;

// Couleurs de la barre de dégâts — mesurées pixel par pixel sur la maquette fournie par
// l'utilisateur (capture d'écran 2026-09-02 : `Capture_decran_2026-09-02_122045.png`), reprise ici
// au plus près plutôt qu'approximée à l'œil (retour utilisateur 2026-09-04 : « ça dénote du jeu »,
// la première tentative n'était pas fidèle).
/// Bordure extérieure — gris moyen, PAS noir (mesuré ~(72,72,74), contrairement à l'intuition
/// visuelle de l'utilisateur qui la décrivait comme sombre : c'est la bordure INTÉRIEURE qui l'est
/// vraiment, voir `BAR_INNER_BORDER`).
const BAR_OUTER_BORDER: egui::Color32 = egui::Color32::from_rgb(72, 72, 74);
/// Bordure intérieure — beaucoup plus sombre que l'extérieure, presque noire (mesurée
/// ~(34,35,39)) : c'est elle qui donne l'effet "double bordure" décrit par l'utilisateur.
const BAR_INNER_BORDER: egui::Color32 = egui::Color32::from_rgb(34, 35, 39);
const BAR_TRACK: egui::Color32 = egui::Color32::from_rgb(22, 23, 27);
/// Curseur de fin de remplissage — petit trait clair vertical à l'extrémité du remplissage (voir
/// `damage_bar`), mesuré ~(191,191,191) sur la maquette. C'est ce trait, décrit par l'utilisateur
/// comme « une petite barre blanche pour dire c'est ici que je suis », qui manquait entièrement à
/// la première tentative de cette refonte. Couleur fixe (pas de dégradé, voir `damage_color`) :
/// c'est un simple repère de position, pas une donnée à lire.
const BAR_END_CAP: egui::Color32 = egui::Color32::from_rgb(191, 191, 191);
/// Reflet du tiers supérieur du remplissage — couleur EXPLICITE (retour utilisateur 2026-09-05,
/// 9e retour : `#0dbebe`), plus une dérivation de `DAMAGE_ACCENT` par éclaircissement (voir l'ancien
/// `lighten`, retiré) : l'utilisateur veut ce ton précis, pas "n'importe quel bleu-vert plus clair".
const BAR_HIGHLIGHT: egui::Color32 = egui::Color32::from_rgb(0x0d, 0xbe, 0xbe);

const TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(235, 240, 245);
/// Couleur du contour peint autour de tout texte flottant par-dessus le jeu (voir
/// `paint_outlined_text`) — noir plein, comme le procédé déjà utilisé par le jeu lui-même pour ses
/// propres incrustations (référence explicite de l'utilisateur, capture d'écran à l'appui).
const TEXT_OUTLINE: egui::Color32 = egui::Color32::BLACK;

const TOTAL_FONT_SIZE: f32 = 18.0;
/// Écart entre la ligne leader et le premier groupe — réduit en cohérence avec `ROW_GAP`.
const TOTAL_GAP: f32 = 3.0;
const NAME_FONT_SIZE: f32 = 13.0;
/// Taille du pourcentage sur le portrait — agrandie une 1re fois (retour utilisateur, 7e retour :
/// « ça a l'air compliqué à lire, il en manque un ou deux pixels »).
const PERCENT_FONT_SIZE: f32 = 12.0;

/// Marge intérieure du fond opacifié de la ligne leader (voir `show_leader_row`) entre son bord et
/// le bouton/le total qu'il contient — la MÊME valeur des deux côtés (le bouton à gauche a un bord
/// net, contrairement au dernier chiffre du total dont le glyphe laisse un peu de son propre
/// espacement interne avant l'encre visible : l'écart géométrique posé ici est bien symétrique,
/// même si l'œil peut lire une petite différence côté texte — retour utilisateur, 7e retour).
const LEADER_PANEL_PADDING: f32 = 6.0;
/// Arrondi du fond opacifié de la ligne leader.
const LEADER_PANEL_ROUNDING: f32 = 6.0;
/// Couleur du fond opacifié de la ligne leader — approximation d'un bandeau translucide du jeu
/// (captures d'écran de référence sans canal alpha exploitable, voir doc de module) : noir
/// bleuté, assez opaque pour détacher la ligne du reste sans devenir un pavé plein.
const LEADER_PANEL_FILL: egui::Color32 = egui::Color32::from_rgba_unmultiplied_const(10, 12, 16, 150);

/// Taille cible (largeur ET hauteur) du socle d'un bouton icône (voir `paint_icon_button`) — le
/// socle fourni par l'utilisateur est natif en 37×37, réduit ici à 24×24 (retour utilisateur, 7e
/// retour : « réduis l'icône bouton en 24×24 ») ; l'icône à fond transparent posée dessus est mise
/// à l'échelle dans le MÊME ratio (pas une taille fixe indépendante), pour rester proportionnée au
/// socle quelle que soit sa taille cible.
const ICON_BUTTON_SIZE: f32 = 24.0;
/// Écart horizontal entre les deux boutons de la barre d'outils du bas (voir `bottom_toolbar`).
const ICON_BUTTON_GAP: f32 = 6.0;

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    fight: Option<&FightSnapshot>,
    portraits: &PortraitAtlas,
    frame: &CombatFrame,
    icons: &UiIcons,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    side: &mut CombatSide,
) {
    // Ordre STABLE (pas trié par dégâts, voir doc de module et `FightSnapshot::fighters`) — c'est
    // l'ordre des PORTRAITS, cadre et liste plate confondus. Calculé ICI, avant toute mise en page
    // (plutôt que dans un `match fight` qui pourrait s'arrêter avant), pour que la ligne leader
    // ci-dessous (`show_leader_row`, qui porte désormais le switch Alliés/Ennemis — voir refonte
    // 11e retour) soit systématiquement peinte, combat vide ou camp sans combattant compris.
    let fighters: Vec<&FighterDamage> = fight
        .map(|fight| {
            fight
                .fighters
                .iter()
                .filter(|f| f.is_ally == (*side == CombatSide::Allies))
                .collect()
        })
        .unwrap_or_default();

    // Barres : liste SÉPARÉE, triée par dégâts décroissant, uniquement les combattants ayant
    // infligé au moins 1 dégât (voir doc de module). `total_damage` est calculé sur TOUS les
    // combattants affichés (y compris ceux à 0 dégât : ils comptent pour 0 dans la somme, le
    // résultat est identique, mais c'est bien le total du camp affiché qui a du sens ici) — seule
    // référence désormais pour le remplissage ET le pourcentage (voir doc de module, correctif de
    // cohérence 2026-09-04).
    let mut bars: Vec<&FighterDamage> =
        fighters.iter().copied().filter(|f| f.total_damage > 0).collect();
    bars.sort_by_key(|f| std::cmp::Reverse(f.total_damage));

    // Vrai total (0 tant qu'il n'y a pas de combat, ou que le camp affiché est vide — on l'affiche
    // tel quel, voir `show_leader_row`) — `total_damage` (avec `.max(1)`) n'existe que pour
    // sécuriser les divisions de ratio ; sans effet sur le résultat puisqu'un dégât nul donne de
    // toute façon un ratio nul.
    let total_damage_raw = fighters.iter().map(|f| f.total_damage).sum::<i64>();
    let total_damage = total_damage_raw.max(1);

    let (framed, flat_portraits): (&[&FighterDamage], &[&FighterDamage]) =
        if *side == CombatSide::Allies {
            fighters.split_at(fighters.len().min(MAX_FRAME_SLOTS))
        } else {
            (&[], &fighters)
        };

    ui.horizontal_top(|ui| {
        // Colonne de gauche : portraits (cadre pour les 6 premiers alliés dans l'ordre stable,
        // liste plate sinon) — voir doc de module.
        ui.vertical(|ui| {
            if !framed.is_empty() {
                frame.show(ui, portraits, icons, framed, total_damage);
            }
            if !flat_portraits.is_empty() {
                if !framed.is_empty() {
                    ui.add_space(ROW_GAP);
                }
                for (i, fighter) in flat_portraits.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(ROW_GAP);
                    }
                    paint_flat_portrait(
                        ui,
                        portraits,
                        icons,
                        catalog,
                        remote_icons,
                        remote_icon_textures,
                        fighter,
                        total_damage,
                    );
                }
            }
        });

        ui.add_space(COLUMN_GAP);

        // Colonne de droite : ligne leader (switch Alliés/Ennemis + total, voir `show_leader_row`)
        // — TOUJOURS peinte, y compris sans combat ou camp vide, pour que le switch reste
        // accessible (le déplacer ici, à la place de l'ancien bouton lien externe, ne doit pas le
        // rendre inatteignable dans un état particulier) — puis soit un message d'état, soit un
        // groupe nom+dégâts+barre compact par combattant ayant infligé des dégâts, trié par
        // dégâts décroissant — indépendante du rythme vertical de la colonne des portraits
        // (demande utilisateur explicite : « il ne faut pas que les groupes soient alignés au
        // portrait »).
        ui.vertical(|ui| {
            show_leader_row(ui, icons, side, total_damage_raw);
            ui.add_space(TOTAL_GAP);
            if fighters.is_empty() {
                ui.weak(match fight {
                    None => "Aucun combat pour l'instant.",
                    Some(_) => match side {
                        CombatSide::Allies => "Aucun allié pour l'instant.",
                        CombatSide::Enemies => "Aucun ennemi pour l'instant.",
                    },
                });
            } else {
                for (i, fighter) in bars.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(ROW_GAP);
                    }
                    damage_bar_group(ui, &fighter.name, fighter.total_damage, total_damage);
                }
            }
        });
    });

    ui.add_space(TOTAL_GAP);
    bottom_toolbar(ui, icons);
}

/// Ligne "leader" en tête de la colonne des barres, sur un fond opacifié (`LEADER_PANEL_FILL`, voir
/// doc de module) qui la détache du reste de la colonne — switch Alliés/Ennemis à gauche (voir
/// `paint_side_switch`, refonte 11e retour : remplace ici l'ancien bouton lien externe, déplacé en
/// bas du panneau avec le nouveau bouton Options, voir `bottom_toolbar`), total de dégâts du camp
/// affiché à droite (même alignement que les chiffres de dégâts de chaque groupe, voir
/// `damage_bar_group`), sans libellé "Total" (retiré, demande utilisateur explicite : le contexte
/// suffit déjà, la ligne est seule tout en haut de la colonne). Appelée par `show` dans TOUS les
/// cas, y compris combat vide ou camp affiché sans combattant — voir sa doc — pour que le switch
/// reste accessible en toute circonstance.
fn show_leader_row(ui: &mut egui::Ui, icons: &UiIcons, side: &mut CombatSide, total_damage: i64) {
    let total_font = egui::FontId::proportional(TOTAL_FONT_SIZE);
    let content_height = SWITCH_HEIGHT.max(total_font.size + 2.0);
    let row_height = content_height + LEADER_PANEL_PADDING * 2.0;
    let (row_rect, _) =
        ui.allocate_exact_size(egui::vec2(BAR_MAX_WIDTH, row_height), egui::Sense::hover());

    ui.painter()
        .rect_filled(row_rect, LEADER_PANEL_ROUNDING, LEADER_PANEL_FILL);

    let switch_top_left = egui::pos2(
        row_rect.min.x + LEADER_PANEL_PADDING,
        row_rect.center().y - SWITCH_HEIGHT / 2.0,
    );
    paint_side_switch(ui, switch_top_left, side, icons);

    paint_outlined_text(
        ui,
        egui::pos2(row_rect.max.x - LEADER_PANEL_PADDING, row_rect.center().y),
        egui::Align2::RIGHT_CENTER,
        &format_fr_thousands(total_damage),
        total_font,
        TEXT_COLOR,
    );
}

/// Barre d'outils en bas du panneau Combat, ajoutée à la refonte 11e retour : bouton "lien externe"
/// (ouvre la web app — déplacé ici depuis la ligne leader, où le switch Alliés/Ennemis a pris sa
/// place, voir `show_leader_row`) suivi du bouton "Options", NOUVEAU (icône `nut.png` fournie par
/// l'utilisateur, voir `UiIcons::options_icon`) — n'ouvre encore aucun panneau : réservé à une
/// future page de réglages (demande utilisateur explicite : « qui permettrait à l'utilisateur PLUS
/// TARD d'ouvrir un panneau d'options »), seule l'infobulle "Options" au survol est déjà là.
/// Toujours peinte, quel que soit l'état du combat affiché : ce ne sont pas des actions liées au
/// combat, contrairement au reste du panneau.
///
/// **Refonte 2026-09-05 (14e retour)** : marge à gauche du premier bouton (lien externe) ajoutée,
/// de la MÊME valeur que l'écart entre les deux boutons (`ICON_BUTTON_GAP`) — retour utilisateur
/// explicite (« j'aimerais que tu appliques le même nombre de pixels entre le premier icône bouton
/// et le bord »). Auparavant collé au bord gauche du panneau, ce qui causait aussi le bug de
/// tooltip corrigé au même retour (voir `show_tooltip_above`).
fn bottom_toolbar(ui: &mut egui::Ui, icons: &UiIcons) {
    let (row_rect, _) = ui.allocate_exact_size(
        egui::vec2(
            ICON_BUTTON_GAP + ICON_BUTTON_SIZE * 2.0 + ICON_BUTTON_GAP,
            ICON_BUTTON_SIZE,
        ),
        egui::Sense::hover(),
    );
    let external_link_top_left = row_rect.min + egui::vec2(ICON_BUTTON_GAP, 0.0);

    let external_link_response = paint_icon_button(
        ui,
        external_link_top_left,
        icons.button_background(),
        icons.button_background_hover(),
        icons.external_link_icon(),
        "combat-open-wakfu-companion",
        "Détails",
    );
    if external_link_response.clicked() {
        // `base_url()` — jamais une URL codée en dur ici : c'est la même origine que le reste de
        // l'overlay parle déjà (voir `overlay_sync::client`), dev ou prod selon le déploiement.
        let _ = open::that(overlay_sync::client::base_url());
    }

    let options_top_left =
        external_link_top_left + egui::vec2(ICON_BUTTON_SIZE + ICON_BUTTON_GAP, 0.0);
    let options_response = paint_icon_button(
        ui,
        options_top_left,
        icons.button_background(),
        icons.button_background_hover(),
        icons.options_icon(),
        "combat-open-options",
        "Options",
    );
    if options_response.clicked() {
        // TODO: ouvrir le panneau d'options une fois qu'il existera (voir doc de module).
    }
}

/// Bouton "icône" du jeu — un socle (`background`, ou `background_hover` quand survolé) et une
/// icône à fond transparent (`icon`) centrée dessus, tous deux mis à l'échelle de `ICON_BUTTON_SIZE`
/// dans le MÊME ratio (voir sa doc) — une première version les peignait à leur taille native sans
/// redimensionnement (demande explicite à l'époque, le temps de juger les proportions) ; une fois
/// jugées, retour utilisateur explicite : « réduis le bouton en 24×24 ». Composant volontairement
/// générique (demande utilisateur explicite : « crée une espèce de composant qui permet de créer
/// des boutons icône ») — appelé par `bottom_toolbar` pour les boutons lien externe ET Options
/// (refonte 11e retour ; `show_leader_row` en était l'unique appelant jusque-là, avant que son
/// bouton lien externe ne soit déplacé dans cette barre d'outils).
/// `id_source` distingue plusieurs boutons icône dans le même conteneur egui (voir
/// `ui.id().with(...)`, même mécanisme que `side_switch`).
fn paint_icon_button(
    ui: &mut egui::Ui,
    top_left: egui::Pos2,
    background: &egui::TextureHandle,
    background_hover: &egui::TextureHandle,
    icon: &egui::TextureHandle,
    id_source: &str,
    tooltip: &str,
) -> egui::Response {
    // Ratio commun dérivé de la largeur du socle — appliqué tel quel à l'icône, pour qu'elle reste
    // proportionnée au socle quelle que soit `ICON_BUTTON_SIZE` (voir sa doc), plutôt qu'une taille
    // d'icône fixée indépendamment.
    let scale = ICON_BUTTON_SIZE / background.size_vec2().x;
    let bg_rect = egui::Rect::from_min_size(top_left, egui::Vec2::splat(ICON_BUTTON_SIZE));
    let response = ui
        .interact(bg_rect, ui.id().with(id_source), egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    show_tooltip_above(&response, tooltip);
    let bg_texture = if response.hovered() {
        background_hover
    } else {
        background
    };
    egui::Image::new(bg_texture).paint_at(ui, bg_rect);
    let icon_rect = egui::Rect::from_center_size(bg_rect.center(), icon.size_vec2() * scale);
    egui::Image::new(icon).paint_at(ui, icon_rect);
    response
}

/// Portrait d'une ligne de la liste "plate" (ennemis, ou alliés au-delà de `MAX_FRAME_SLOTS`) —
/// portrait de classe pour un allié classifié, sinon (ennemi, ou allié pas encore classifié)
/// portrait RÉEL du monstre si le catalogue le résout par nom, repli générique tant qu'il n'a pas
/// fini de télécharger ou si le nom n'est pas reconnu. Infobulle (nom) et pourcentage de dégâts
/// (bas-droite, si non nul) systématiques — voir doc de module.
#[allow(clippy::too_many_arguments)]
fn paint_flat_portrait(
    ui: &mut egui::Ui,
    portraits: &PortraitAtlas,
    icons: &UiIcons,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    fighter: &FighterDamage,
    total_damage: i64,
) {
    let class_portrait = fighter
        .class_name
        .as_deref()
        .and_then(|class_name| portraits.image(class_name, fighter.gender, fighter.is_ko));
    let remote_monster_texture = class_portrait.is_none().then(|| {
        catalog
            .find_monster_icon(&fighter.name, None)
            .and_then(|icon_ref| remote_icon_textures.resolve(ui.ctx(), remote_icons, &icon_ref))
    });
    let response = match (class_portrait, remote_monster_texture.flatten()) {
        (Some(image), _) => ui.add(image),
        (None, Some(texture)) => ui.add(
            egui::Image::new(&texture)
                .fit_to_exact_size(egui::vec2(
                    crate::portraits::PORTRAIT_SIZE,
                    crate::portraits::PORTRAIT_SIZE,
                ))
                .maintain_aspect_ratio(false)
                // Pas de version grisée précalculée pour une icône distante (voir doc de module) :
                // simple tint, approximation acceptée.
                .tint(grey_tint_if_ko(fighter.is_ko)),
        ),
        (None, None) => ui.add(
            icons
                .unknown_entity_image()
                .tint(grey_tint_if_ko(fighter.is_ko)),
        ),
    };
    let rect = response.rect;
    show_tooltip_above(&response, fighter.name.as_str());
    if fighter.total_damage > 0 {
        paint_portrait_percent(ui, rect, fighter.total_damage, total_damage);
    }
}

/// Tint à appliquer à une icône de repli/distante pour approximer un grisé KO — voir la doc de
/// module pour pourquoi ce n'est PAS utilisé sur les portraits de classe (`PortraitAtlas` en
/// précalcule une vraie version en niveaux de gris). Un tint multiplie chaque canal de couleur par
/// le tint : un gris moyen assombrit uniformément sans désaturer la teinte d'origine — moins bon
/// qu'un vrai niveau de gris, mais suffisant pour un repli rarement affiché (allié pas encore
/// classifié avec un cadre visible, ou une icône réseau qui n'a de toute façon pas d'équivalent
/// niveaux de gris préchargé).
pub(crate) fn grey_tint_if_ko(is_ko: bool) -> egui::Color32 {
    if is_ko {
        egui::Color32::from_gray(130)
    } else {
        egui::Color32::WHITE
    }
}

/// Pourcentage de dégâts d'un combattant par rapport au total du camp affiché, incrusté au coin
/// bas-droit du carré ENGLOBANT `rect` (portrait de classe, monstre, ou repli générique — appelé
/// aussi bien par `panels::combat_frame::CombatFrame::show` que par `paint_flat_portrait`
/// ci-dessus) — demande utilisateur explicite (retour après capture d'écran) : « comme si on
/// traçait un carré autour du rond et qu'on plaçait le pourcentage tout en bas à droite », donc
/// légèrement EN DEHORS du disque visible plutôt que dessus, pour ne jamais recouvrir le portrait.
pub(crate) fn paint_portrait_percent(ui: &egui::Ui, rect: egui::Rect, damage: i64, total_damage: i64) {
    let ratio = (damage as f32 / total_damage as f32).clamp(0.0, 1.0);
    let percent = (ratio as f64 * 100.0).round() as i64;
    let text = format!("{percent}%");
    // Décalage vers l'EXTÉRIEUR du coin (pas vers l'intérieur) — demande utilisateur : « encore un
    // peu plus sur la droite [...] pour qu'il mange un peu moins sur le portrait ».
    let pos = rect.right_bottom() + egui::vec2(2.0, 1.0);
    // `ACCENT` (bleu Wakfu, même que le switch) — retour utilisateur 2026-09-05 (9e retour) :
    // revient sur `DAMAGE_ACCENT` du 8e retour (« je préfère la couleur accent qu'il y avait
    // avant »). Résultat assumé : la barre (`DAMAGE_ACCENT`) et ce pourcentage n'ont plus la même
    // couleur — explicitement voulu, pas un oubli de cohérence.
    paint_outlined_text(
        ui,
        pos,
        egui::Align2::RIGHT_BOTTOM,
        &text,
        egui::FontId::proportional(PERCENT_FONT_SIZE),
        ACCENT,
    );
}

/// Un "groupe" nom + dégâts + barre de la colonne de droite — nom à gauche et dégâts chiffrés à
/// droite sur la MÊME ligne (retour utilisateur : le chiffre de dégâts avait disparu avec le
/// passage au pourcentage seul, régression à corriger), barre juste en dessous, quasiment collée
/// (`GROUP_NAME_BAR_GAP`) pour que l'œil lise l'ensemble comme un seul bloc — et un espace plus
/// large (`ROW_GAP`, voir l'appelant) entre deux groupes DIFFÉRENTS pour que cette distinction
/// reste lisible.
fn damage_bar_group(ui: &mut egui::Ui, name: &str, damage: i64, total_damage: i64) {
    let bar_width = ui.available_width().min(BAR_MAX_WIDTH);
    let name_font = egui::FontId::proportional(NAME_FONT_SIZE);
    // Pas de rembourrage supplémentaire sous le texte (retour utilisateur, 6e retour : « l'écart
    // entre la barre et la ligne du dessus », déjà réduit une 1re fois via `GROUP_NAME_BAR_GAP` —
    // le reste venait de cette marge, retirée).
    let name_height = name_font.size;
    let (name_rect, _) =
        ui.allocate_exact_size(egui::vec2(bar_width, name_height), egui::Sense::hover());
    paint_outlined_text(
        ui,
        name_rect.left_center(),
        egui::Align2::LEFT_CENTER,
        name,
        name_font.clone(),
        TEXT_COLOR,
    );
    paint_outlined_text(
        ui,
        name_rect.right_center(),
        egui::Align2::RIGHT_CENTER,
        &format_fr_thousands(damage),
        name_font,
        TEXT_COLOR,
    );
    ui.add_space(GROUP_NAME_BAR_GAP);
    let (bar_rect, _) =
        ui.allocate_exact_size(egui::vec2(bar_width, BAR_HEIGHT), egui::Sense::hover());
    damage_bar(ui, bar_rect, damage, total_damage);
}

/// Piste + remplissage d'une barre de dégâts, peinte dans `rect` (déjà alloué par l'appelant, voir
/// `damage_bar_group`) — reproduit la maquette fournie par l'utilisateur (capture d'écran
/// 2026-09-02) au plus près, couleurs mesurées pixel par pixel (voir les constantes `BAR_*`) :
/// arrondi léger (`BAR_ROUNDING`, PAS un stade/pilule — voir sa doc), double bordure concentrique
/// (gris moyen puis presque noir), piste sombre, remplissage `DAMAGE_ACCENT` avec reflet
/// `BAR_HIGHLIGHT` sur son tiers supérieur (couleur EXPLICITE, pas dérivée de `DAMAGE_ACCENT` —
/// voir sa doc) et un curseur clair fin à l'extrémité du remplissage. Le pourcentage peint sur le
/// portrait correspondant utilise `ACCENT`, une couleur DIFFÉRENTE (retour utilisateur 2026-09-05,
/// 9e retour — voir `paint_portrait_percent`). Ni nom ni pourcentage ici : le nom et les dégâts
/// sont peints par l'appelant au-dessus de `rect`.
fn damage_bar(ui: &mut egui::Ui, rect: egui::Rect, damage: i64, total_damage: i64) {
    let painter = ui.painter().with_clip_rect(rect);
    let rounding = BAR_ROUNDING;
    painter.rect_filled(rect, rounding, BAR_OUTER_BORDER);

    let inner_rect = rect.shrink(BAR_BORDER_WIDTH);
    let inner_rounding = (rounding - BAR_BORDER_WIDTH).max(0.0);
    painter.rect_filled(inner_rect, inner_rounding, BAR_INNER_BORDER);

    let track_rect = inner_rect.shrink(BAR_BORDER_WIDTH);
    let track_rounding = (inner_rounding - BAR_BORDER_WIDTH).max(0.0);
    painter.rect_filled(track_rect, track_rounding, BAR_TRACK);

    let ratio = (damage as f32 / total_damage as f32).clamp(0.0, 1.0);
    if ratio > 0.0 {
        let full = ratio >= 0.999;
        let track_r = track_rounding as u8;
        let fill_color = DAMAGE_ACCENT;
        // Coins droits arrondis UNIQUEMENT si le remplissage atteint le bout de la piste — sinon
        // le bord droit du remplissage tombe au milieu de la piste, un coin arrondi y serait
        // visuellement faux (un arrondi qui ne correspond à aucun bord réel de la piste).
        let fill_rounding = egui::CornerRadius {
            nw: track_r,
            sw: track_r,
            ne: if full { track_r } else { 0 },
            se: if full { track_r } else { 0 },
        };
        let fill_rect = egui::Rect::from_min_size(
            track_rect.min,
            egui::vec2(track_rect.width() * ratio, track_rect.height()),
        );
        painter.rect_filled(fill_rect, fill_rounding, fill_color);

        let highlight_rect = egui::Rect::from_min_size(
            fill_rect.min,
            egui::vec2(fill_rect.width(), fill_rect.height() * 0.35),
        );
        let highlight_rounding = egui::CornerRadius {
            nw: track_r,
            ne: fill_rounding.ne,
            sw: 0,
            se: 0,
        };
        painter.rect_filled(highlight_rect, highlight_rounding, BAR_HIGHLIGHT);

        // Curseur de fin — voir doc de fonction. Masqué quand le remplissage est complet : il se
        // confondrait avec le bord droit de la piste, sans rien apporter.
        if !full {
            const CAP_WIDTH: f32 = 2.0;
            let cap_rect = egui::Rect::from_center_size(
                egui::pos2(fill_rect.max.x, track_rect.center().y),
                egui::vec2(CAP_WIDTH, track_rect.height()),
            );
            painter.rect_filled(cap_rect, 1.0, BAR_END_CAP);
        }
    }
}

/// Formate un entier selon l'usage français : espace tous les 3 chiffres depuis la droite (ex.
/// `113574` → `113 574`) — demande utilisateur explicite : un grand nombre collé était difficile à
/// lire d'un coup d'œil (« je ne sais pas si c'est onze mille ou cent-treize mille »).
///
/// Espace ORDINAIRE (pas insécable) — une première version utilisait une espace insécable
/// (U+00A0), jugée trop discrète face au formatage du jeu lui-même (retour utilisateur, capture de
/// comparaison à l'appui : l'écart entre groupes de chiffres semblait presque absent). Sans risque
/// de retour à la ligne malvenu ici : ce texte est TOUJOURS peint directement via `Painter::text`
/// (voir `paint_outlined_text`), jamais mis en page par un widget qui pourrait le scinder.
fn format_fr_thousands(n: i64) -> String {
    let sign = if n < 0 { "-" } else { "" };
    let digits = n.unsigned_abs().to_string();
    let grouped: String = digits
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).expect("chiffres ASCII"))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{sign}{grouped}")
}

#[cfg(test)]
mod tests {
    use super::format_fr_thousands;

    #[test]
    fn format_fr_thousands_insere_une_espace_tous_les_3_chiffres() {
        assert_eq!(format_fr_thousands(0), "0");
        assert_eq!(format_fr_thousands(7), "7");
        assert_eq!(format_fr_thousands(999), "999");
        assert_eq!(format_fr_thousands(1000), "1 000");
        assert_eq!(format_fr_thousands(11000), "11 000");
        assert_eq!(format_fr_thousands(113574), "113 574");
        assert_eq!(format_fr_thousands(1234567), "1 234 567");
        assert_eq!(format_fr_thousands(-42000), "-42 000");
    }
}

/// Peint `text` avec un VRAI contour (8 copies décalées d'1 px dans chaque direction, en
/// `TEXT_OUTLINE`, puis le texte plein par-dessus) — nécessaire pour tout texte qui flotte nu
/// par-dessus le jeu (nom, dégâts, pourcentage, total) : le fond y est arbitraire, une simple
/// ombre décalée d'un côté (ancienne version) reste illisible dès que ce fond est clair de ce
/// côté-là. Même procédé que les incrustations du jeu lui-même (référence utilisateur, capture
/// d'écran à l'appui : pourcentage de vie en blanc cerné de noir).
pub(crate) fn paint_outlined_text(
    ui: &egui::Ui,
    pos: egui::Pos2,
    align: egui::Align2,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
) {
    const OFFSETS: [egui::Vec2; 8] = [
        egui::vec2(-1.0, -1.0),
        egui::vec2(0.0, -1.0),
        egui::vec2(1.0, -1.0),
        egui::vec2(-1.0, 0.0),
        egui::vec2(1.0, 0.0),
        egui::vec2(-1.0, 1.0),
        egui::vec2(0.0, 1.0),
        egui::vec2(1.0, 1.0),
    ];
    let painter = ui.painter();
    for offset in OFFSETS {
        painter.text(pos + offset, align, text, font.clone(), TEXT_OUTLINE);
    }
    painter.text(pos, align, text, font, color);
}

/// Affiche `text` en infobulle AU-DESSUS de `response` (`RectAlign::TOP`) plutôt qu'en dessous —
/// c'est le comportement PAR DÉFAUT d'egui pour une tooltip (`RectAlign::BOTTOM_START`, voir
/// `egui::Tooltip`/`egui::Popup`) que l'utilisateur juge désagréable ici (« la tooltip apparaît en
/// bas de la souris ») : le standard qu'il attend est une tooltip au-dessus de l'élément survolé.
///
/// Repli 2026-09-05 (8e retour, corrigé au 10e) : `align_alternatives` n'est plus vide. Le switch
/// Alliés/Ennemis (`side_switch`) est le tout premier widget du panneau Combat, collé au bord
/// supérieur de la fenêtre (`inner_margin` nul pour ce panneau, voir `render_content::
/// paint_content`) — il n'y a donc RIGOUREUSEMENT AUCUNE place au-dessus de lui. Avec un repli vide,
/// egui ne peut pas honorer `TOP` et l'unique position calculée se retrouve contrainte au bord de
/// la fenêtre, ÉCRASÉE sur le switch lui-même (bug rapporté, capture à l'appui).
///
/// `egui::RectAlign::find_best_align` (voir sa doc) exige que le rectangle de la tooltip tienne
/// ENTIÈREMENT dans la fenêtre pour retenir un repli — sur LES DEUX AXES, pas seulement en hauteur.
/// Un seul repli `RectAlign::BOTTOM` (centré sous l'élément) suffisait pour "Ennemis" (assez de
/// marge des deux côtés) mais PAS pour "Alliés" : ce bouton est collé au bord GAUCHE de la fenêtre,
/// et centrer une tooltip plus large que lui la fait déborder à gauche (x négatif) — repli rejeté,
/// egui retombait alors sur le tout premier choix (`TOP`), d'où le bug qui ne touchait QUE ce
/// bouton (retour utilisateur avec capture, 10e retour). `BOTTOM_START`/`BOTTOM_END` couvrent ce
/// cas (bord gauche aligné au lieu de centré, la tooltip ne peut alors déborder que du CÔTÉ
/// opposé au bord de fenêtre le plus proche) — au passage, `BOTTOM_START` n'est PAS "suit le
/// curseur" comme documenté par erreur ici auparavant : c'est un alignement ancré au rectangle du
/// widget (coin bas-gauche), au même titre que `BOTTOM` ; seul l'ancien réglage PAR DÉFAUT d'egui
/// pour une tooltip combine cet alignement à un anchor "widget entier" sans jamais essayer `TOP`
/// en premier, ce qui donnait l'impression d'un simple "en dessous" désagréable pour l'utilisateur.
///
/// **Refonte 2026-09-05 (14e retour)** : bug analogue rapporté sur le bouton lien externe
/// (`bottom_toolbar`, capture à l'appui) une fois celui-ci déplacé au 11e retour dans la barre
/// d'outils du bas, collée au bord GAUCHE — exactement la même cause que ci-dessus ("Alliés"),
/// mais l'ancien repli ne la couvrait pas : TOP échoue (déborde à gauche), et TOUS les replis
/// listés (`BOTTOM*`) sont des variantes EN DESSOUS — dès que TOP échoue pour n'importe quelle
/// raison, la tooltip finit toujours en dessous, jamais au-dessus, même quand `TOP_START` (aligné
/// au bord au lieu de centré, comme `BOTTOM_START` mais AU-DESSUS) aurait parfaitement tenu. Le
/// bouton Options, juste à côté mais plus loin du bord (voir `ICON_BUTTON_GAP` dans
/// `bottom_toolbar`), ne débordait pas et gardait donc `TOP` sans jamais révéler le problème.
/// `TOP_START`/`TOP_END` ajoutés AVANT les replis `BOTTOM*` : un widget proche d'un bord horizontal
/// reste maintenant au-dessus (juste réaligné) tant qu'il reste de la place au-dessus tout court —
/// les replis `BOTTOM*` ne restent un dernier recours que s'il n'y a RÉELLEMENT aucune place
/// au-dessus, sur aucun alignement.
pub(crate) fn show_tooltip_above(response: &egui::Response, text: &str) {
    let mut tooltip = egui::Tooltip::for_enabled(response);
    tooltip.popup = tooltip
        .popup
        .align(egui::RectAlign::TOP)
        .align_alternatives(&[
            egui::RectAlign::TOP_START,
            egui::RectAlign::TOP_END,
            egui::RectAlign::BOTTOM,
            egui::RectAlign::BOTTOM_START,
            egui::RectAlign::BOTTOM_END,
        ]);
    tooltip.show(|ui| {
        ui.set_max_width(ui.spacing().tooltip_width);
        ui.label(text);
    });
}

/// Switch à deux icônes (alliés/ennemis) avec fond glissant — même mécanique que `.icon-switch` du
/// dépôt web (`styles.css`), portée en dessin egui direct (peintre + zones cliquables) puisqu'il
/// n'y a pas de CSS ici pour l'obtenir gratuitement. Peint à un `top_left` donné, SANS allocation
/// via `ui.allocate_exact_size` (même logique que `paint_icon_button`) — depuis la refonte 11e
/// retour, ce switch est logé dans la ligne leader (`show_leader_row`) à la place de l'ancien
/// bouton lien externe, retour utilisateur explicite (« meilleur emplacement que là où est le
/// switch actuellement ») ; l'ancienne rangée pleine largeur en tête de panneau (qui s'allouait
/// elle-même son espace) a disparu.
fn paint_side_switch(
    ui: &mut egui::Ui,
    top_left: egui::Pos2,
    side: &mut CombatSide,
    icons: &UiIcons,
) {
    let option_size = egui::vec2(SWITCH_OPTION_WIDTH, SWITCH_HEIGHT);
    let allies_rect = egui::Rect::from_min_size(top_left, option_size);
    let enemies_rect =
        egui::Rect::from_min_size(top_left + egui::vec2(option_size.x, 0.0), option_size);
    let rect = allies_rect.union(enemies_rect);

    let painter = ui.painter();
    painter.rect_filled(rect, 5.0, TINT_MEDIUM);
    painter.rect_stroke(
        rect,
        5.0,
        egui::Stroke::new(1.0, TINT_STRONG),
        egui::StrokeKind::Inside,
    );
    let highlight_rect = if *side == CombatSide::Allies {
        allies_rect
    } else {
        enemies_rect
    };
    painter.rect_filled(highlight_rect.shrink(1.0), 4.0, ACCENT);

    draw_centered_icon(ui, allies_rect, icons.allies());
    draw_centered_icon(ui, enemies_rect, icons.enemies());

    let allies_response = ui
        .interact(
            allies_rect,
            ui.id().with("combat-side-allies"),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    show_tooltip_above(&allies_response, "Alliés");
    let enemies_response = ui
        .interact(
            enemies_rect,
            ui.id().with("combat-side-enemies"),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    show_tooltip_above(&enemies_response, "Ennemis");
    if allies_response.clicked() {
        *side = CombatSide::Allies;
    }
    if enemies_response.clicked() {
        *side = CombatSide::Enemies;
    }
}

fn draw_centered_icon(ui: &egui::Ui, rect: egui::Rect, texture: &egui::TextureHandle) {
    let icon_rect = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(SWITCH_ICON_SIZE, SWITCH_ICON_SIZE),
    );
    egui::Image::new(texture).paint_at(ui, icon_rect);
}
