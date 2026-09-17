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
//!   VRAI contour (8 passes décalées, voir `design::text::paint_outlined_text`) plutôt qu'une simple ombre 1px
//!   décalée — même procédé que les incrustations du jeu lui-même (ex. pourcentage de vie), gardé
//!   en référence par l'utilisateur (capture d'écran à l'appui). L'ancienne ombre simple restait
//!   illisible sur certains fonds clairs du décor.
//! - Infobulles repositionnées AU-DESSUS de l'élément survolé (`RectAlign::TOP`, voir
//!   `design::tooltip`, côté `Above`) — par défaut egui les place en dessous, jugé désagréable par
//!   l'utilisateur (la tooltip apparaît sous le curseur, pas au-dessus du portrait).
//!
//! **Refonte 2026-09-04 (2e retour, après capture du rendu ci-dessus)** :
//! - Barre encore trop "pilule" (arrondi = moitié de la hauteur) : la maquette n'a en réalité
//!   qu'un léger arrondi, pas une forme en stade — voir `BAR_ROUNDING`, remesuré sur une nouvelle
//!   capture de comparaison fournie par l'utilisateur. Hauteur ramenée à 16 px (mesure d'origine,
//!   la précédente l'avait agrandie à 18 sans nécessité).
//! - le seul `RectAlign::TOP` ne suffisait pas : par défaut, une tooltip qui ne "tient" pas au-dessus
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
//!   `Painter::text` (voir `design::text::paint_outlined_text`), jamais mis en page par un widget qui pourrait le
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
//!   corrigé en conséquence — voir `design::tooltip` et son placement `Above`.
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
//!   retour utilisateur avec capture) — voir `design::tooltip` : repli `RectAlign::BOTTOM` ajouté
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
//! voir `design::tooltip`, dont la doc corrige aussi une erreur d'un retour précédent sur ce que
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
//!
//! **Refonte 2026-09-06 (design system boutons icône)** : `paint_icon_button` (socle + icône
//! centrée, jusque-là privé à ce module) est EXTRAIT vers `panels::icon_button` — composant partagé
//! avec `panels::watchlist` (boutons "+"/"−"), voir sa doc et celle de `ui_icons` : retour
//! utilisateur explicite (image de référence à l'appui, `menu-button-icon-first-plan.png`), les
//! quatre boutons icône de l'overlay doivent utiliser EXACTEMENT le même socle et la même teinte
//! d'icône. `bottom_toolbar` garde une fine enveloppe locale (`paint_toolbar_button`, tooltip
//! au-dessus + `Sense::click()`) autour du composant partagé. Fond translucide ajouté derrière la
//! barre (`icon_button::PANEL_BACKDROP_FILL`) avec une marge symétrique sur les QUATRE côtés
//! (`ICON_BUTTON_GAP`, réutilisée aussi pour le haut/bas — auparavant seuls la gauche et l'écart
//! entre boutons en avaient une, voir 14e retour) : reproduit le petit fond noir semi-opaque visible
//! entre les boutons de la planche de référence.
//!
//! **Refonte 2026-09-06 (marge du panneau, 15e retour)** : les refontes précédentes du 8e au 10e
//! retour n'avaient corrigé que les REPLIS du placement (aujourd'hui dans `design::tooltip`) (quel
//! `RectAlign::BOTTOM*` choisir une fois `TOP` rejeté) sans jamais s'attaquer à la cause — le
//! switch Alliés/Ennemis reste collé au bord SUPÉRIEUR du panneau (`inner_margin` nul), donc `TOP`
//! échoue TOUJOURS pour lui, quel que soit le repli disponible : les deux infobulles s'affichaient
//! en dessous plutôt qu'au- dessus (nouveau retour utilisateur explicite : « les tooltips du switch
//! alliés/ennemis s'affichent en dessous au lieu d'au dessus [...] agrandis légèrement l'overlay
//! combat »). Voir `render_content::COMBAT_TOP_MARGIN` : le panneau gagne une marge haute (44px)
//! suffisante pour que `TOP` tienne enfin — la fenêtre Combat
//! (`main.rs`/`bin/wakfu-companion-overlay-x11.rs::WINDOW_SIZE`) est agrandie d'autant pour ne rien
//! compresser d'autre.
//!
//! **Refonte 2026-09-07 (vision ennemie, scroll infini)** — demande utilisateur explicite (les
//! breaches peuvent aligner 40 à 60+ monstres, jamais affichables en liste plate lisible) :
//! - Camp Ennemis, jusqu'à `MAX_FRAME_SLOTS` : rejoint le même chemin que les alliés
//!   (`CombatFrame::show`, gabarit exact) — jusqu'ici réservé aux alliés, les ennemis restaient
//!   TOUJOURS en liste plate quel que soit leur nombre.
//! - Camp Ennemis, au-delà de `MAX_FRAME_SLOTS` : `panels::combat_frame_scroll::EnemyFrameScroll`
//!   (voir sa doc de module pour l'architecture complète, validée par plusieurs artefacts
//!   interactifs) — plus de liste plate dans ce cas.
//!
//! **Refonte 2026-09-08 (retour utilisateur après test en jeu réel)** — deux régressions
//! introduites par la refonte ci-dessus, corrigées :
//! - Les ennemis routés vers `CombatFrame::show`/`EnemyFrameScroll::show` s'affichaient tous avec
//!   le portrait générique (`UiIcons::unknown_entity_*`) : ces deux fonctions ne résolvaient que
//!   `PortraitAtlas` (portraits de CLASSE, alliés uniquement — un ennemi n'a jamais de
//!   `class_name`), contrairement à la liste "plate" qui retombait déjà sur le portrait RÉEL du
//!   monstre via le catalogue. `resolve_fighter_texture` (voir sa doc) factorise cette résolution
//!   catalogue/icône distante — désormais partagée par les trois chemins d'affichage d'un
//!   portrait (liste plate, cadre exact, cadre à défilement).
//! - Scrollbar d'`EnemyFrameScroll` : la condition « visible seulement au survol du cadre entier »
//!   (voir doc de module de `combat_frame_scroll`) est retirée — demande explicite de
//!   l'utilisateur après test en jeu, la barre fine collée au bord ne gênait pas assez pour
//!   justifier de la cacher. Toujours visible désormais.
//!
//! **Refonte 2026-09-08 (déplacement Détails/Options vers le Suivi)** : `bottom_toolbar` (boutons
//! "lien externe"/"Options" en bas de CE panneau, voir l'historique ci-dessus) est retirée —
//! demande utilisateur explicite : le panneau Combat n'est pas toujours affiché (aucun combat en
//! cours), contrairement au panneau Suivi qui reste visible en permanence ; ces deux boutons
//! rejoignent donc `panels::watchlist::control_button_row`, dans le même carré 2×2 que "+"/"−",
//! plutôt que de rester à un emplacement appelé à disparaître. Les raccourcis clavier globaux
//! (`DETAILS_HOTKEY_LABEL`/`OPTIONS_HOTKEY_LABEL`, `main.rs`) et les deux actions elles-mêmes
//! (`open::that(overlay_sync::client::base_url())` / TODO options) sont inchangés, seul leur point
//! d'entrée visuel bouge.
//!
//! **Refonte 2026-09-13 (sorts ennemis)** : le bloc « ligne de sorts » (`combat_spell_block`,
//! jusqu'ici vue Alliés seulement) s'affiche aussi en vue Ennemis, avec les mêmes règles — la
//! sélection est calculée pour le camp affiché, les marques et les clics passent par le cadre
//! exact (`CombatFrame::show`) ou le cadre à défilement (`EnemyFrameScroll::show`) selon le
//! nombre d'ennemis. Voir la doc de module de `combat_spell_block` et §9.1 bis du plan.
//!
//! **Refonte 2026-09-14 (armure donnée et soins)** — demande utilisateur : le panneau ne montrait
//! que les dégâts, alors que le moteur capte désormais aussi l'armure DONNÉE et les soins, pour
//! les deux camps (voir `overlay_engine::session::FighterDamage::total_armor`/`total_heal`, portés
//! par le parser vendu du dépôt web). Un switch à trois positions (`CombatMetric`,
//! `paint_metric_switch`) s'ajoute dans le bandeau leader, SOUS le switch Alliés/Ennemis : les
//! deux axes sont indépendants (l'armure d'un boss se regarde comme celle d'un allié). Tout ce que
//! le panneau chiffre suit ce switch — total de la ligne leader, tri et remplissage des barres,
//! pourcentage incrusté sur chaque portrait — parce qu'un seul point de lecture décide désormais
//! de la valeur d'un combattant (`CombatMetric::value_of`), au lieu d'un `total_damage` lu en dur
//! en quatre endroits. Un camp peuplé qui n'a rien produit de la grandeur choisie le dit
//! (« Aucune armure donnée pour l'instant. ») plutôt que de laisser une colonne muette.
//! Icônes DU JEU (`assets/ui/metric-*.png`, voir `UiIcons`) plutôt que des libellés : ce sont
//! celles du sélecteur équivalent du dépôt web, embarquées plutôt que chargées depuis le CDN — un
//! contrôle permanent ne dépend pas du réseau. Le nom de la grandeur reste à l'infobulle.
//!
//! Le bloc « ligne de sorts » (`combat_spell_block`), lui, ne dépend PAS de ce switch : il montre
//! les sorts du dernier tour dans l'ordre où ils ont été lancés, quelle que soit la grandeur
//! regardée (décision utilisateur explicite, 14 sept. 2026).
//!
//! **Refonte 2026-09-15 (échange des deux switches)** — demande utilisateur : le switch Alliés/
//! Ennemis quitte le bandeau leader (colonne des barres) pour coiffer la colonne des PORTRAITS,
//! dans un bandeau à lui (`show_side_row`) au même vocabulaire visuel — fond `LEADER_PANEL_FILL`,
//! arrondi `LEADER_PANEL_ROUNDING`, marge intérieure `LEADER_PANEL_PADDING`, air `SIDE_ROW_GAP`
//! en dessous — calé sur `combat_frame::FRAME_WIDTH`, le switch centré dedans. Le switch de
//! grandeur prend sa place dans le bandeau leader, à gauche du total : le contrôle vit désormais
//! au-dessus de ce qu'il commande (le camp au-dessus des portraits, la grandeur à côté du chiffre
//! qu'elle qualifie), et le bandeau leader retrouve sa hauteur d'avant l'arrivée de la grandeur
//! (une seule rangée, plus deux). Le switch de camp reste peint en toutes circonstances — combat
//! absent, camp vide, ennemis nombreux en cadre défilant — et reste le tout premier widget du
//! panneau, ce dont dépend la marge d'infobulle de `render_content`.
//!
//! **Les barres restent filtrées à `value_of(f) > 0` pour les trois grandeurs** — un combattant
//! qui n'a rien produit de la grandeur affichée garde son portrait mais pas de barre. Le site
//! (`Oumbra/wakfu-companion`) fait l'inverse et liste tout le roster, zéros compris : cet écart
//! est voulu (décision utilisateur explicite, 15 sept. 2026, voir CLAUDE.md « Ce que l'overlay
//! affiche diffère du site ») — temps réel ici, bilan de fin de combat là-bas. Ne pas aligner les
//! deux.
//!
//! **Deux interrupteurs le commandent (2026-09-15)**, section « Combat » de l'onglet
//! « Paramètres » (`panels::feature_switch::FeatureToggles`) :
//!
//! - « Activer le détail des combats » coupe le panneau ENTIER — c'est [`should_show`] qui le dit,
//!   et les deux hôtes qui masquent la fenêtre OS
//!   (`main.rs`/`bin/wakfu-companion-overlay-x11.rs`). Ce module
//!   ne peint donc jamais avec cette case décochée.
//! - « Activer le suivi des sorts » coupe la seule ligne de sorts (paramètre `spells_enabled` de
//!   [`show`], déjà combiné avec la case ci-dessus par l'hôte) : le bloc `combat_spell_block`,
//!   mais aussi les marques qu'il pose sur les médaillons et l'épinglage au clic — sans bloc à
//!   lire, un liseré ne désignerait plus rien. Portraits, barres et switches ne bougent pas.
//!
//! **Refonte 2026-09-16 (switches au design system)** — les deux switches, peints à la main
//! jusque-là (piste `TINT_MEDIUM` bordée de `TINT_STRONG`, option active `ACCENT`, 30 × 26 par
//! option), sont désormais `design::switch` : le sélecteur exclusif du jeu, ses six textures, ses
//! cases de fond kaki biseauté (active) ou gris-brun (inactive). Décisions prises sur rendu du
//! harnais (avant / après en artefact, quatre variantes) :
//!
//! - **À l'échelle 36/44** (`SWITCH_SCALE`, via `Switch::scale`) : le switch du jeu réduit
//!   homothétiquement — cases de 35 × 36, séparateur de 2, liseré, biseaux et glyphes réduits
//!   dans le même rapport. Deux cases = 72px, trois = 109px. Trois tailles ont été rendues le
//!   même jour avant celle-ci : 26px de haut par étirement du 9-slice (« on a l'impression
//!   d'avoir compressé le switch »), le standard de 44 (« le rendu dans le jeu est relativement
//!   imposant »), puis cette réduction — « vraiment scaler à double dimension pour ne pas perdre
//!   le rendu visuel ». Les deux bandeaux font 48px, la colonne des barres descend d'autant
//!   (`BARS_COLUMN_TOP_OFFSET`).
//! - **Bandeaux opacifiés conservés** (`LEADER_PANEL_FILL`), avec leurs marges de 6px. Le
//!   bandeau de camp, qui faisait la largeur du cadre, s'élargit à 84px pour loger le switch de
//!   72 avec ses marges : **calé à gauche sur le cadre**, il déborde de 14px vers la gouttière
//!   des colonnes (retour utilisateur : les marges latérales du bandeau doivent rester celles du
//!   design, pas se résorber sur la gauche) — sans rencontrer le bandeau leader, qui commence
//!   plus bas (`BARS_COLUMN_TOP_OFFSET`).
//! - **Variante premier plan** (demande utilisateur du même jour, après les décisions ci-dessus) :
//!   `SwitchVariant::FirstPlan` remplace le cadre kaki du sélecteur de genre par le socle de
//!   bouton icône de premier plan du jeu (`button-icon-first-plan.png` et son `-hover`), celui des
//!   boutons du carré de contrôle du Suivi. Ces deux switches sont les seuls contrôles de
//!   l'overlay à flotter PAR-DESSUS la scène : ils portent désormais la matière que le jeu emploie
//!   à cet endroit. `SWITCH_SCALE` disparaît avec ce changement — la variante est native à 36px, il
//!   n'y a plus rien à réduire. Les switches passent de 72 et 109px à **74 et 112** (une gouttière
//!   de 2px subsiste entre deux socles, qui portent chacun leurs quatre coins), la hauteur ne bouge
//!   pas, et les deux bandeaux comme `BARS_COLUMN_TOP_OFFSET` en dérivent sans être touchés. La
//!   case choisie ne se lit plus à un fond kaki mais au socle éclairci, à son icône en couleur
//!   pleine et à son liseré (voir `design::SwitchVariant`). Quatre allers-retours sur rendu ont
//!   réglé le détail le même jour : glyphe ramené au plafond commun (il avait grossi de 68 %,
//!   faute d'étalon d'encre sur les glyphes en couleurs), socles collés par un chevauchement de
//!   2 px, survol qui allume le glyphe EN MÊME TEMPS que le socle (sans quoi l'icône restait
//!   éteinte sur un fond allumé), et liseré `#126068` de la case choisie — devenu nécessaire,
//!   justement, parce que le survol allume tout.
//! - **Le total du bandeau leader** ne disposait que de 69px (190 − 12 de marges − 109 de
//!   switch) : assez pour six chiffres au corps de 18, pas pour sept (« 1 047 404 » = 79px,
//!   qui mordait de 10px sur la case Soins — capture utilisateur du même jour). Décision sur
//!   rendu (huit variantes en artefact) : le bandeau **déborde de 6px de chaque côté**
//!   (`LEADER_PANEL_OVERHANG`, en permanence — pas de saut de largeur en cours de combat) et le
//!   total passe au **corps 16 au-delà de six chiffres** (`TOTAL_FONT_SIZE_COMPACT`) : 71px
//!   dans 81, 10px d'air. Réduire seul aurait demandé 15px de corps (3px d'air, le total ne se
//!   détache plus des lignes), élargir seul 8px de chaque côté au moins, au-delà de la
//!   gouttière de 6 — le bandeau montait sur l'ornement du cadre des portraits.
//! - **Icônes en couleurs** : les cinq glyphes (`DsIcon::Allies`/`Enemies`/`Metric*`, catégorie
//!   `couleur`) restent ceux du jeu et du site, peints tels quels sur la case active et
//!   atténués ailleurs. Passés au monochrome du design system, alliés et ennemis ne se
//!   distinguaient plus que par la position des bras, et les deux cœurs devenaient des taches.
//!
//! Les infobulles (« Alliés (F2) », « Dégâts infligés (F3) »…) sont les libellés des cases, portés
//! par le composant ; `UiIcons` ne porte plus ces cinq textures.

use overlay_engine::{CatalogIndex, FightSnapshot, FighterDamage, SessionSnapshot};

use crate::design::{self, text};
use crate::portraits::PortraitAtlas;
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::shortcuts::{ShortcutAction, ShortcutBindings};
use crate::ui_icons::UiIcons;

use super::combat_bars::DamageBars;
use super::combat_frame::{CombatFrame, SelectionMarks, FRAME_WIDTH, MAX_FRAME_SLOTS};
use super::combat_frame_scroll::EnemyFrameScroll;
use super::combat_spell_block;

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

impl CombatSide {
    /// Inverse le camp affiché — utilisé par le raccourci global `ShortcutAction::CombatSide`
    /// (`main.rs::App::toggle_combat_side`) en plus du clic direct sur le switch de camp : retour
    /// utilisateur explicite (« en mode toggle, c'est-à-dire que quand on appuie, ça inverse la
    /// sélection »), un seul raccourci pour les deux camps plutôt qu'un par camp.
    pub fn toggled(self) -> Self {
        match self {
            Self::Allies => Self::Enemies,
            Self::Enemies => Self::Allies,
        }
    }
}

/// Grandeur mesurée par les barres, les pourcentages sur les portraits et le total de la ligne
/// leader — pilotée par `paint_metric_switch`, un état par fenêtre overlay comme [`CombatSide`]
/// (voir `OverlayWindow` dans `main.rs`).
///
/// Le camp affiché et la grandeur mesurée sont deux axes INDÉPENDANTS : « armure donnée par les
/// ennemis » est une question aussi légitime que « dégâts infligés par les alliés » (demande
/// utilisateur explicite, 2026-09-14 — l'armure et les soins sont captés pour les deux camps, voir
/// `overlay_engine::session::FighterDamage`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CombatMetric {
    /// Dégâts infligés — la vue historique du panneau, et le défaut.
    #[default]
    Damage,
    /// Armure DONNÉE (jamais reçue, jamais perdue — voir `FighterDamage::total_armor`).
    Armor,
    /// Soins produits.
    Heal,
}

impl CombatMetric {
    /// Dans l'ordre du switch — dégâts d'abord (le défaut), puis les deux grandeurs de soutien.
    pub const ALL: [Self; 3] = [Self::Damage, Self::Armor, Self::Heal];

    /// Icône DU JEU de cette grandeur — les mêmes que le sélecteur `app-entity-stat-tabs` du dépôt
    /// web, au registre du design system (catégorie `couleur`, voir `design::icons`).
    pub fn icon(self) -> design::DsIcon {
        match self {
            Self::Damage => design::DsIcon::MetricDamage,
            Self::Armor => design::DsIcon::MetricArmor,
            Self::Heal => design::DsIcon::MetricHeal,
        }
    }

    /// Infobulle du switch — dit ce que la grandeur compte VRAIMENT, là où l'icône seule reste
    /// ambiguë (l'armure est celle qu'on donne, pas celle qu'on encaisse).
    pub fn tooltip(self) -> &'static str {
        match self {
            Self::Damage => "Dégâts infligés",
            Self::Armor => "Armure donnée",
            Self::Heal => "Soins prodigués",
        }
    }

    /// Message affiché à la place des barres quand personne, dans le camp affiché, n'a encore
    /// produit quoi que ce soit de cette grandeur.
    pub fn empty_message(self) -> &'static str {
        match self {
            Self::Damage => "Aucun dégât pour l'instant.",
            Self::Armor => "Aucune armure donnée pour l'instant.",
            Self::Heal => "Aucun soin pour l'instant.",
        }
    }

    /// Valeur de CE combattant pour cette grandeur — le seul endroit qui décide quel champ de
    /// `FighterDamage` alimente l'affichage ; tout le panneau (barres, total, pourcentage sur le
    /// portrait, cadres) passe par ici plutôt que de lire `total_damage` en dur.
    pub fn value_of(self, fighter: &FighterDamage) -> i64 {
        match self {
            Self::Damage => fighter.total_damage,
            Self::Armor => fighter.total_armor,
            Self::Heal => fighter.total_heal,
        }
    }

    /// Grandeur suivante, en boucle — pendant de [`CombatSide::toggled`], appelée par le raccourci
    /// global `ShortcutAction::CombatMetric` (`Ctrl+Shift+V` par défaut, `main.rs::App::
    /// cycle_combat_metric`) en plus du clic direct sur le switch.
    pub fn next(self) -> Self {
        match self {
            Self::Damage => Self::Armor,
            Self::Armor => Self::Heal,
            Self::Heal => Self::Damage,
        }
    }
}

/// Hauteur des deux switches — **36px**, et elle n'est plus réglée ici : c'est le côté natif du
/// socle de premier plan (`design::tokens::SWITCH_FIRST_PLAN_SIZE`), donc ce qu'annonce
/// `Switch::desired_size` sans échelle ni hauteur imposée.
///
/// L'échelle 36/44 qui précédait (`SWITCH_SCALE`, réduction homothétique du cadre du jeu de 44 à
/// 36) a disparu avec elle : la variante premier plan n'a rien à réduire, elle est déjà à sa
/// taille. La valeur, elle, ne change pas d'un pixel — les deux bandeaux et
/// `BARS_COLUMN_TOP_OFFSET` en dérivent et ne bougent donc pas non plus.
const SWITCH_HEIGHT: f32 = design::tokens::SWITCH_FIRST_PLAN_SIZE;
/// Écart vertical entre deux portraits de la liste "plate", et entre deux groupes nom+barre de la
/// colonne de droite (même rythme pour les deux colonnes, demande utilisateur explicite). Resserré
/// une 4e fois (6 px → 4 px → 2 px → 1 px, retour utilisateur répété : « il y a un écart non
/// négligeable entre les groupes, il faut le réduire »).
pub(super) const ROW_GAP: f32 = 1.0;
/// Écart horizontal entre la colonne des portraits (cadre ou liste plate) et celle des barres —
/// réduit (retour utilisateur 2026-09-04 : « moins écartés, un peu plus proches ») par rapport à
/// la première version de cette refonte (12 px).
const COLUMN_GAP: f32 = 6.0;

// Le vocabulaire du contenu flottant vient des jetons partagés, jamais de constantes locales : ce
// panneau et le panneau Suivi en portaient plusieurs en double, sans lien déclaré entre elles.
// Voir la section `OVERLAY_*` de `design::tokens`, qui porte chaque décision et sa raison.
//
// `DAMAGE_ACCENT` a disparu le 2026-09-12 : c'était un doublon pur de `tokens::METER_FILL`, dont
// la doc porte déjà l'historique des neuf retours utilisateur qui ont séparé, fusionné puis
// re-séparé cette teinte de l'accent. `ACCENT`, `TINT_MEDIUM` et `TINT_STRONG` ne servaient plus
// ici qu'aux deux switches peints à la main, partis dans `design::switch` le 2026-09-16.
use crate::design::tokens::METER_FILL as DAMAGE_ACCENT;

/// Hauteur d'une barre de dégâts — mesurée sur la maquette fournie par l'utilisateur (capture
/// d'écran 2026-09-02, ~16 px de haut). Une première itération l'avait portée à 18 px sans
/// nécessité (retour utilisateur : « elle est plus haute que celle que je t'ai fournie ») — revenu
/// à la mesure d'origine.
pub(super) const BAR_HEIGHT: f32 = 16.0;
/// Largeur maximale d'une barre — agrandie par rapport à la première version de cette refonte
/// (150 px) maintenant que `COLUMN_GAP` est réduit (voir sa doc) : l'espace regagné doit profiter
/// à la barre, pas rester vide.
pub(super) const BAR_MAX_WIDTH: f32 = 190.0;
/// Écart entre le nom (+ dégâts) et sa barre, DANS un groupe (voir `damage_bar_group`) —
/// volontairement plus petit que `ROW_GAP` (qui sépare deux groupes ENTRE eux) : c'est cette
/// différence de rythme qui donne à l'œil la lecture "un nom + une barre = un groupe". Resserré une
/// 3e fois (2 px → 1 px → 0, retour utilisateur répété : « encore plus compact ») — le nom/les
/// dégâts touchent maintenant directement le haut de la barre.
pub(super) const GROUP_NAME_BAR_GAP: f32 = 0.0;

// Couleurs de la barre de dégâts — mesurées pixel par pixel sur la maquette fournie par
// l'utilisateur (capture d'écran 2026-09-02 : `Capture_decran_2026-09-02_122045.png`), reprise ici
// au plus près plutôt qu'approximée à l'œil (retour utilisateur 2026-09-04 : « ça dénote du jeu »,
// la première tentative n'était pas fidèle).

use crate::design::tokens::OVERLAY_TEXT as TEXT_COLOR;

const TOTAL_FONT_SIZE: f32 = 18.0;
/// Corps du total au-delà de `TOTAL_FULL_SIZE_MAX_DIGITS` chiffres. Mesuré par egui sur
/// « 1 047 404 » (16 sept. 2026) : 79px au corps de 18, 71 à 16, 66 à 15, 62 à 14 — dans les
/// 81px laissés au total (190 + 12 de débord − 12 de marges − 109 de switch), 16 laisse 10px
/// d'air ; 15 aurait tenu sans débord du bandeau mais avec 3px seulement, et à 14 le total ne se
/// détache plus des chiffres des lignes (13px).
const TOTAL_FONT_SIZE_COMPACT: f32 = 16.0;
/// Nombre de chiffres jusqu'auquel le total garde `TOTAL_FONT_SIZE` : six chiffres (« 999 999 »,
/// ≈ 9,9px par chiffre et 4,7 par espace au corps de 18, soit 64px) tiennent dans la place
/// disponible ; sept ne tiennent qu'au corps compact.
const TOTAL_FULL_SIZE_MAX_DIGITS: usize = 6;
/// Air VISIBLE entre la ligne leader et le premier groupe — 10 px depuis le 12 sept. 2026 (retour
/// utilisateur : le même écart que `combat_spell_block::BLOCK_GAP` entre le dernier groupe et le
/// bloc de sorts, « pour l'homogénéité entre les blocs »). `show` en retranche l'`item_spacing`
/// vertical d'egui, glissé après la ligne leader, pour que ce soit bien l'écart à l'écran.
pub(super) const TOTAL_GAP: f32 = 10.0;
pub(super) const NAME_FONT_SIZE: f32 = 13.0;

/// Marge intérieure du fond opacifié de la ligne leader (voir `show_leader_row`) entre son bord et
/// le bouton/le total qu'il contient — la MÊME valeur des deux côtés (le bouton à gauche a un bord
/// net, contrairement au dernier chiffre du total dont le glyphe laisse un peu de son propre
/// espacement interne avant l'encre visible : l'écart géométrique posé ici est bien symétrique,
/// même si l'œil peut lire une petite différence côté texte — retour utilisateur, 7e retour).
const LEADER_PANEL_PADDING: f32 = 6.0;
/// Débord du bandeau leader de chaque côté de la colonne des barres — **permanent**, pas
/// seulement au million (décision utilisateur du 16 sept. 2026 : le bandeau ne change pas de
/// largeur en cours de combat, seul le corps du total bascule). Vaut la gouttière des colonnes :
/// à gauche le bandeau la remplit exactement sans monter sur l'ornement du cadre des portraits
/// (dès 7px il le touchait, vu sur rendu), à droite il déborde d'autant pour rester centré sur
/// la colonne. Le switch de grandeur, calé à `LEADER_PANEL_PADDING` du bord du bandeau, tombe
/// donc au ras de la colonne. Peint hors allocation, comme le débord du bandeau de camp.
const LEADER_PANEL_OVERHANG: f32 = COLUMN_GAP;
/// Air sous le bandeau du switch Alliés/Ennemis, avant le cadre des portraits. Resserré à 5 px
/// (demande utilisateur, 15 sept. 2026 : « rapproche-le du template, au moins 5 pixels ») plutôt
/// que de reprendre `TOTAL_GAP` : ce bandeau n'introduit pas une liste comme le bandeau leader, il
/// coiffe un objet unique — le cadre — dont il doit se lire comme la coiffe, pas comme un voisin.
const SIDE_ROW_GAP: f32 = 5.0;

/// Décalage vertical de la colonne des barres, qui démarre donc plus bas que celle des portraits.
/// Vaut la hauteur du bandeau du switch de camp (`SWITCH_HEIGHT` + ses deux marges intérieures)
/// plus l'air qui le sépare du cadre (`SIDE_ROW_GAP`) : le haut du bandeau leader tombe donc
/// exactement sur la PREMIÈRE LIGNE DE PIXELS du gabarit de cadre, la décoration de son sommet —
/// règle d'alignement demandée explicitement (15 sept. 2026), tracer une droite depuis ce premier
/// pixel doit rencontrer le sommet du bloc de droite. Les deux bandeaux ne s'alignent donc pas
/// entre eux : le switch de camp appartient visuellement aux portraits qu'il commande, la ligne du
/// total appartient aux barres qu'elle chapeaute.
const BARS_COLUMN_TOP_OFFSET: f32 = SWITCH_HEIGHT + LEADER_PANEL_PADDING * 2.0 + SIDE_ROW_GAP;
/// Arrondi du fond opacifié de la ligne leader.
pub(super) const LEADER_PANEL_ROUNDING: f32 = 6.0;
/// Couleur du fond opacifié de la ligne leader — voir [`tokens::OVERLAY_BACKDROP`], qui la partage
/// avec le bloc de sorts et avec le carré de contrôle du Suivi.
pub(super) use crate::design::tokens::OVERLAY_BACKDROP as LEADER_PANEL_FILL;

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
    // Grandeur mesurée (dégâts / armure donnée / soins), voir `CombatMetric` — indépendante du
    // camp affiché, pilotée par le switch de `show_leader_row`.
    metric: &mut CombatMetric,
    // Voir `panels::watchlist::WatchlistAssets::shortcuts` : même raison, ici pour l'infobulle du
    // switch Alliés/Ennemis.
    shortcuts: &ShortcutBindings,
    // Le suivi des sorts est-il actif ? — case « Activer le suivi des sorts » de la section
    // « Combat » des Options (2026-09-15, `feature_switch::FeatureToggles::spells_visible`, qui
    // combine déjà cette case avec celle dont elle dépend). `false` retire le bloc « ligne de
    // sorts » ET les deux marques qu'il pose sur les médaillons : sans bloc à lire, un liseré et
    // un point sur un portrait ne désigneraient plus rien.
    spells_enabled: bool,
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
    // infligé au moins 1 dégât (voir doc de module) — et de même pour l'armure donnée et les
    // soins. Filtre VOULU, à ne pas retirer pour « s'aligner » sur le site, qui garde lui ses
    // lignes à zéro : voir la doc de module et CLAUDE.md. `total_damage` est calculé sur TOUS les
    // combattants affichés (y compris ceux à 0 dégât : ils comptent pour 0 dans la somme, le
    // résultat est identique, mais c'est bien le total du camp affiché qui a du sens ici) — seule
    // référence désormais pour le remplissage ET le pourcentage (voir doc de module, correctif de
    // cohérence 2026-09-04).
    let measured = *metric;
    let mut bars: Vec<&FighterDamage> = fighters
        .iter()
        .copied()
        .filter(|f| measured.value_of(f) > 0)
        .collect();
    bars.sort_by_key(|f| std::cmp::Reverse(measured.value_of(f)));

    // Vrai total (0 tant qu'il n'y a pas de combat, ou que le camp affiché est vide — on l'affiche
    // tel quel, voir `show_leader_row`) — `total_damage` (avec `.max(1)`) n'existe que pour
    // sécuriser les divisions de ratio ; sans effet sur le résultat puisqu'un dégât nul donne de
    // toute façon un ratio nul.
    let total_damage_raw = fighters.iter().map(|f| measured.value_of(f)).sum::<i64>();
    let total_damage = total_damage_raw.max(1);

    // `framed` : gabarit exact (`CombatFrame::show`, 1 à `MAX_FRAME_SLOTS` combattants). `enemy_
    // scroll` : ennemis au-delà de `MAX_FRAME_SLOTS` (voir `combat_frame_scroll`, refonte
    // 2026-09-07) — le plus grand gabarit réutilisé comme fenêtre fixe, portraits défilants dedans,
    // JAMAIS de liste plate dans ce cas (contrairement à avant cette refonte). `flat_portraits` ne
    // reste donc utile qu'aux alliés au-delà de `MAX_FRAME_SLOTS` (cas rare, voir doc de
    // `combat_frame`).
    let (framed, flat_portraits, enemy_scroll): (
        &[&FighterDamage],
        &[&FighterDamage],
        &[&FighterDamage],
    ) = match *side {
        CombatSide::Allies => {
            let (framed, flat) = fighters.split_at(fighters.len().min(MAX_FRAME_SLOTS));
            (framed, flat, &[])
        }
        CombatSide::Enemies if fighters.len() <= MAX_FRAME_SLOTS => (fighters.as_slice(), &[], &[]),
        CombatSide::Enemies => (&[], &[], fighters.as_slice()),
    };

    // Sélection du bloc « ligne de sorts » (voir `combat_spell_block`) pour le camp affiché —
    // `None` tant qu'aucun combattant de ce camp n'a lancé de sort. Recalculée après un clic sur
    // un portrait du cadre (colonne de gauche) pour que le bloc (colonne de droite) suive dans la
    // même frame. Les marques s'expriment en positions dans la tranche du cadre qui les peint :
    // `framed` (gabarit exact) ou `enemy_scroll` (ennemis nombreux) — jamais les deux.
    let is_ally = *side == CombatSide::Allies;
    // **Le combat vu par le bloc de sorts** — le même, sauf quand le suivi des sorts est coupé :
    // `None` retire d'un coup la sélection, les deux marques sur les médaillons, l'épinglage au
    // clic et le bloc lui-même. Tout ce qui sert la ligne de sorts passe par cette variable, et
    // rien d'autre : les portraits, leurs infobulles et les barres continuent de lire `fight`.
    let spell_fight = fight.filter(|_| spells_enabled);
    let mut selection =
        spell_fight.and_then(|fight| combat_spell_block::selection(ui.ctx(), fight, is_ally));
    let slot_of = |list: &[&FighterDamage], sel: usize| {
        list.iter().position(|f| {
            Some(sel) == spell_fight.and_then(|fight| combat_spell_block::fighter_index(fight, f))
        })
    };
    let marks_in = |list: &[&FighterDamage], sel: Option<combat_spell_block::SpellSelection>| {
        sel.map(|sel| SelectionMarks {
            ring_slot: slot_of(list, sel.selected),
            dot_slot: slot_of(list, sel.last_caster),
        })
    };

    ui.horizontal_top(|ui| {
        // Colonne de gauche : portraits — cadre exact (alliés ou ennemis jusqu'à `MAX_FRAME_SLOTS`),
        // cadre à défilement (ennemis au-delà), ou liste plate (alliés excédentaires) — voir doc de
        // module.
        ui.vertical(|ui| {
            // Switch Alliés/Ennemis : en tête de CETTE colonne depuis le 2026-09-15 (voir doc de
            // module) — peint inconditionnellement, avant tout test sur le contenu du cadre, pour
            // qu'il reste atteignable sans combat comme dans un camp vide (c'était déjà la raison
            // qui le gardait dans le bandeau leader, elle ne change pas de colonne avec lui).
            show_side_row(ui, side, shortcuts);
            ui.add_space(SIDE_ROW_GAP - ui.spacing().item_spacing.y);
            if !framed.is_empty() {
                let marks = marks_in(framed, selection);
                let clicked = frame.show(
                    ui,
                    portraits,
                    icons,
                    catalog,
                    remote_icons,
                    remote_icon_textures,
                    framed,
                    measured,
                    total_damage,
                    marks,
                );
                if let (Some(slot), Some(fight)) = (clicked, spell_fight) {
                    if let Some(idx) = combat_spell_block::fighter_index(fight, framed[slot]) {
                        combat_spell_block::on_portrait_clicked(ui.ctx(), fight, idx);
                        selection = combat_spell_block::selection(ui.ctx(), fight, is_ally);
                    }
                }
            }
            if !enemy_scroll.is_empty() {
                let marks = marks_in(enemy_scroll, selection);
                let clicked = EnemyFrameScroll::show(
                    ui,
                    frame,
                    portraits,
                    icons,
                    catalog,
                    remote_icons,
                    remote_icon_textures,
                    enemy_scroll,
                    measured,
                    total_damage,
                    marks,
                );
                if let (Some(slot), Some(fight)) = (clicked, spell_fight) {
                    if let Some(idx) = combat_spell_block::fighter_index(fight, enemy_scroll[slot])
                    {
                        combat_spell_block::on_portrait_clicked(ui.ctx(), fight, idx);
                        selection = combat_spell_block::selection(ui.ctx(), fight, is_ally);
                    }
                }
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
                        measured,
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
            // Sans retrancher `item_spacing.y`, contrairement aux autres `add_space` de ce
            // fichier : cet espace-ci OUVRE la colonne (rien avant lui dans le `vertical`), egui
            // n'y glisse donc aucun espacement propre — le retrancher coûtait 3 px et laissait les
            // barres 3 px trop haut (mesuré sur les captures avant/après).
            ui.add_space(BARS_COLUMN_TOP_OFFSET);
            show_leader_row(ui, metric, shortcuts, total_damage_raw);
            ui.add_space(TOTAL_GAP - ui.spacing().item_spacing.y);
            if fighters.is_empty() || bars.is_empty() {
                // Camp vide (ou pas de combat) d'abord : dire « aucun soin » alors qu'il n'y a
                // personne à soigner serait une fausse piste. Un camp peuplé mais sans rien à
                // montrer pour la grandeur choisie, lui, le dit explicitement — sans ce message,
                // basculer sur Armure dans un combat sans blindeur laissait la colonne vide sans
                // qu'on sache si c'était zéro ou un bug.
                ui.weak(match (fight, fighters.is_empty()) {
                    (None, _) => "Aucun combat pour l'instant.",
                    (Some(_), true) => match side {
                        CombatSide::Allies => "Aucun allié pour l'instant.",
                        CombatSide::Enemies => "Aucun ennemi pour l'instant.",
                    },
                    (Some(_), false) => measured.empty_message(),
                });
            } else {
                // Fenêtre bornée à six groupes, défilante au-delà (`combat_bars`, 13 sept.
                // 2026) — même rythme vertical que l'ancienne liste en dessous de six.
                DamageBars::show(ui, &bars, measured, total_damage);
            }
            // Bloc « ligne de sorts » (voir `combat_spell_block`) : après le dernier groupe, pour
            // le camp affiché, dès qu'un de ses combattants a lancé un sort (avant, rien — pas
            // même l'espace). `BLOCK_GAP` est l'air VISIBLE voulu : egui glisse déjà
            // `item_spacing.y` après le dernier widget, retranché ici pour ne pas le compter
            // deux fois.
            if let (Some(fight), Some(sel)) = (spell_fight, selection) {
                ui.add_space(combat_spell_block::BLOCK_GAP - ui.spacing().item_spacing.y);
                combat_spell_block::show(ui, fight, sel, remote_icons, remote_icon_textures);
            }
        });
    });
}

/// Bandeau du switch Alliés/Ennemis, en tête de la colonne des PORTRAITS (échange de place avec le
/// switch de grandeur, 2026-09-15 — voir doc de module) : même fond opacifié, même arrondi et même
/// marge intérieure que le bandeau leader de l'autre colonne (`LEADER_PANEL_FILL`/
/// `LEADER_PANEL_ROUNDING`/`LEADER_PANEL_PADDING`) — le camp affiché se choisit donc au-dessus de
/// ce qu'il commande, les portraits.
///
/// **Calé à gauche sur le cadre, plus large que lui** depuis le passage à `design::switch`
/// (2026-09-16) : le switch fait 72px, ses marges de 6px portent le bandeau à 84px, et c'est
/// vers la gouttière des colonnes qu'il déborde — le bord gauche du panneau reste celui du
/// cadre. La colonne, elle, n'alloue que `FRAME_WIDTH` : le débord est peint hors allocation,
/// sans décaler la colonne des barres, dont le bandeau commence plus bas
/// (`BARS_COLUMN_TOP_OFFSET`).
///
/// Appelée par `show` dans TOUS les cas, y compris sans combat ou camp vide (donc cadre non peint)
/// : c'est la même exigence qu'avant le déplacement, le switch ne doit jamais devenir inatteignable.
/// Reste aussi le TOUT PREMIER widget peint du panneau, ce dont dépend la marge supérieure réservée
/// à son infobulle (voir `render_content::COMBAT_TOOLTIP_HEADROOM`).
fn show_side_row(ui: &mut egui::Ui, side: &mut CombatSide, shortcuts: &ShortcutBindings) {
    let row_height = SWITCH_HEIGHT + LEADER_PANEL_PADDING * 2.0;
    let (row_rect, _) =
        ui.allocate_exact_size(egui::vec2(FRAME_WIDTH, row_height), egui::Sense::hover());
    // La combinaison RÉELLE, personnalisable comme toutes les autres (voir `ShortcutBindings`),
    // dans l'infobulle de chaque case — le libellé de la case EST son infobulle.
    let hotkey = shortcuts.label(ShortcutAction::CombatSide);
    let switch = design::switch(side)
        .slot(CombatSide::Allies, format!("Alliés ({hotkey})"))
        .icon(design::DsIcon::Allies)
        .slot(CombatSide::Enemies, format!("Ennemis ({hotkey})"))
        .icon(design::DsIcon::Enemies)
        .variant(design::SwitchVariant::FirstPlan)
        .log_name("combat.camp");
    let switch_size = switch.desired_size();
    let backdrop = egui::Rect::from_min_size(
        row_rect.min,
        egui::vec2(switch_size.x + LEADER_PANEL_PADDING * 2.0, row_height),
    );
    ui.painter()
        .rect_filled(backdrop, LEADER_PANEL_ROUNDING, LEADER_PANEL_FILL);

    let switch_rect = egui::Rect::from_min_size(
        backdrop.min + egui::vec2(LEADER_PANEL_PADDING, LEADER_PANEL_PADDING),
        switch_size,
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(switch_rect));
    switch.show(&mut child);
}

/// Ligne "leader" en tête de la colonne des barres, sur un fond opacifié (`LEADER_PANEL_FILL`, voir
/// doc de module) qui la détache du reste de la colonne — switch de grandeur à gauche (voir
/// `paint_metric_switch`) depuis l'échange de place du 2026-09-15, total du camp affiché pour cette
/// grandeur à droite (même alignement que les chiffres de chaque groupe, voir `damage_bar_group`),
/// sans libellé "Total" (retiré, demande utilisateur explicite : le contexte suffit déjà, la ligne
/// est seule tout en haut de la colonne).
///
/// Une SEULE rangée depuis cet échange : le switch de grandeur y remplace le switch Alliés/Ennemis
/// (parti coiffer les portraits, voir `show_side_row`) au lieu de s'ajouter sous lui, ce qui rend au
/// bandeau la hauteur qu'il avait avant l'arrivée de la grandeur. Il reste peint dans TOUS les cas,
/// y compris combat vide ou camp sans combattant — la grandeur se choisit alors aussi.
///
/// Le bandeau déborde de `LEADER_PANEL_OVERHANG` de chaque côté de la colonne, et le total passe
/// au corps `TOTAL_FONT_SIZE_COMPACT` au-delà de `TOTAL_FULL_SIZE_MAX_DIGITS` chiffres — les deux
/// ensemble font tenir un total à sept chiffres à côté du switch de grandeur (voir doc de module,
/// refonte 2026-09-16). La hauteur du bandeau ne bouge pas : c'est le switch qui la fixe.
fn show_leader_row(
    ui: &mut egui::Ui,
    metric: &mut CombatMetric,
    shortcuts: &ShortcutBindings,
    total_damage: i64,
) {
    let total_text = format_fr_thousands(total_damage);
    let digits = total_text.chars().filter(char::is_ascii_digit).count();
    let total_font_size = if digits > TOTAL_FULL_SIZE_MAX_DIGITS {
        TOTAL_FONT_SIZE_COMPACT
    } else {
        TOTAL_FONT_SIZE
    };
    let total_font = text::label_font(ui.ctx(), total_font_size);
    let inner_height = SWITCH_HEIGHT.max(total_font.size + 2.0);
    let row_height = inner_height + LEADER_PANEL_PADDING * 2.0;
    let (row_rect, _) =
        ui.allocate_exact_size(egui::vec2(BAR_MAX_WIDTH, row_height), egui::Sense::hover());
    let row_rect = row_rect.expand2(egui::vec2(LEADER_PANEL_OVERHANG, 0.0));

    ui.painter()
        .rect_filled(row_rect, LEADER_PANEL_ROUNDING, LEADER_PANEL_FILL);

    // Grandeur à gauche, total à droite, sur la même ligne : la grandeur décide de ce que raconte
    // TOUT le reste du panneau (ce total, les barres, les pourcentages sur les portraits) — la
    // poser juste à côté du chiffre qu'elle qualifie se lit d'un seul coup d'œil.
    let center_y = row_rect.min.y + LEADER_PANEL_PADDING + inner_height / 2.0;
    // Même convention d'infobulle que le switch de camp : le nom de la grandeur (ce qu'elle
    // compte VRAIMENT, voir `CombatMetric::tooltip`) et sa combinaison.
    let hotkey = shortcuts.label(ShortcutAction::CombatMetric);
    let mut switch = design::switch(metric);
    for option in CombatMetric::ALL {
        switch = switch
            .slot(option, format!("{} ({hotkey})", option.tooltip()))
            .icon(option.icon());
    }
    let switch = switch
        .variant(design::SwitchVariant::FirstPlan)
        .log_name("combat.grandeur");
    let switch_size = switch.desired_size();
    let switch_rect = egui::Rect::from_min_size(
        egui::pos2(
            row_rect.min.x + LEADER_PANEL_PADDING,
            center_y - switch_size.y / 2.0,
        ),
        switch_size,
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(switch_rect));
    switch.show(&mut child);

    text::paint_outlined_text(
        ui,
        egui::pos2(row_rect.max.x - LEADER_PANEL_PADDING, center_y),
        egui::Align2::RIGHT_CENTER,
        &total_text,
        total_font,
        TEXT_COLOR,
        text::OUTLINE_FULL,
    );
}

/// Texture résolue pour un combattant — voir `resolve_fighter_texture`. Distingue les deux
/// origines possibles car elles n'appellent PAS le même traitement KO chez l'appelant : un
/// portrait de classe est DÉJÀ grisé (précalculé, voir `PortraitAtlas`), une icône de monstre
/// distante ne l'est jamais (pas de version grisée précalculée pour elle, voir
/// `grey_tint_if_ko`) et doit être teintée par l'appelant si `fighter.is_ko`.
pub(crate) enum FighterPortrait {
    ClassPortrait(egui::TextureHandle),
    RemoteMonster(egui::TextureHandle),
}

/// Résout la texture à afficher pour `fighter` : portrait de classe si `class_name` est connu
/// (allié classifié), sinon icône RÉELLE du monstre si le catalogue la résout par nom (voir
/// `CatalogIndex::find_monster_icon` — télécharge en arrière-plan au besoin,
/// `RemoteIconTextures::resolve` renvoie `None` tant que ce n'est pas prêt), sinon `None` (repli
/// générique laissé à l'appelant, voir `UiIcons::unknown_entity_*`).
///
/// **Factorisée** entre `paint_flat_portrait` (liste plate) et les cadres à médaillons
/// (`combat_frame::CombatFrame::show`, `combat_frame_scroll::EnemyFrameScroll::show`) — ces
/// derniers n'en avaient PAS besoin avant la refonte 2026-09-07 (vision ennemie, scroll infini,
/// voir doc de module) : seuls des alliés (toujours classifiés ou jamais) y passaient jusque-là.
/// Une fois les ennemis routés vers ces mêmes cadres, ils s'y affichaient tous avec le portrait
/// générique — un ennemi n'a jamais de `class_name` (`breed` non déterministe côté ennemi, voir
/// `overlay_engine::class_breed`) — d'où cette extraction, plutôt que dupliquer la résolution
/// catalogue/icône distante dans les deux modules de cadre.
pub(crate) fn resolve_fighter_texture(
    ui: &egui::Ui,
    portraits: &PortraitAtlas,
    catalog: &CatalogIndex,
    remote_icons: &RemoteIconStore,
    remote_icon_textures: &mut RemoteIconTextures,
    fighter: &FighterDamage,
) -> Option<FighterPortrait> {
    if let Some(texture) = fighter
        .class_name
        .as_deref()
        .and_then(|class_name| portraits.texture(class_name, fighter.gender, fighter.is_ko))
    {
        return Some(FighterPortrait::ClassPortrait(texture.clone()));
    }
    let icon_ref = catalog.find_monster_icon(&fighter.name, None)?;
    let texture = remote_icon_textures.resolve(ui.ctx(), remote_icons, &icon_ref)?;
    Some(FighterPortrait::RemoteMonster(texture))
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
    metric: CombatMetric,
    total_damage: i64,
) {
    let portrait = resolve_fighter_texture(
        ui,
        portraits,
        catalog,
        remote_icons,
        remote_icon_textures,
        fighter,
    );
    // La forme, le grisé et le pourcentage vivent dans `design::portrait` depuis le 2026-09-11.
    // Ce qui reste ici est le choix de la TEXTURE et celui de teinter ou non : seul ce panneau sait
    // qu'un portrait de classe a sa version grise précalculée dans l'atlas, et qu'une icône
    // distante ou le repli n'en ont pas.
    let (texture, dimmed) = match &portrait {
        Some(FighterPortrait::ClassPortrait(texture)) => (texture.id(), false),
        Some(FighterPortrait::RemoteMonster(texture)) => (texture.id(), fighter.is_ko),
        None => (icons.unknown_entity_texture().id(), fighter.is_ko),
    };
    let response = ui.add(
        design::portrait(texture)
            .size(crate::portraits::PORTRAIT_SIZE)
            .dimmed(dimmed)
            .percent({
                let value = metric.value_of(fighter);
                (value > 0).then(|| design::portrait_percent(value, total_damage))
            }),
    );
    design::tooltip(&response).text(fighter.name.as_str());
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

/// Hauteur d'un groupe nom + dégâts + barre tel que `paint_damage_bar_group` le peint : ligne de
/// texte (`NAME_FONT_SIZE`), `item_spacing` d'egui (héritage de l'ancienne liste, où le nom et la
/// barre étaient deux allocations successives), `GROUP_NAME_BAR_GAP`, barre (`BAR_HEIGHT`).
pub(super) fn damage_group_height(ui: &egui::Ui) -> f32 {
    NAME_FONT_SIZE + ui.spacing().item_spacing.y + GROUP_NAME_BAR_GAP + BAR_HEIGHT
}

/// Un groupe « nom + dégâts + barre », peint dans `rect` (largeur = celle des barres, hauteur =
/// `damage_group_height`) : nom à gauche et dégâts à droite sur la ligne du haut (texte contouré,
/// voir `design::text`), barre en dessous (`damage_bar`). Aucune allocation : c'est
/// `combat_bars::DamageBars` qui alloue la fenêtre et place chaque groupe à sa position défilée.
/// `opacity` donne l'opacité de chaque élément (ligne de texte, puis barre) d'après son rectangle
/// — le fondu aux bords de la fenêtre (voir `combat_bars`) ; renvoie 1 hors fondu.
///
/// Pas de rembourrage supplémentaire sous le texte (retour utilisateur, 6e retour : « l'écart
/// entre la barre et la ligne du dessus », déjà réduit une 1re fois via `GROUP_NAME_BAR_GAP` —
/// le reste venait de cette marge, retirée).
pub(super) fn paint_damage_bar_group(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    name: &str,
    damage: i64,
    total_damage: i64,
    opacity: &dyn Fn(egui::Rect) -> f32,
) {
    let name_font = text::label_font(ui.ctx(), NAME_FONT_SIZE);
    let name_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), name_font.size));
    // `new_child` et non `ui.scope` pour porter l'opacité : un scope réallouerait dans `ui` — voir
    // `combat_bars::DamageBars::show`.
    {
        let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(name_rect));
        ui.multiply_opacity(opacity(name_rect));
        let ui = &ui;
        text::paint_outlined_text(
            ui,
            name_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            name,
            name_font.clone(),
            TEXT_COLOR,
            text::OUTLINE_FULL,
        );
        text::paint_outlined_text(
            ui,
            name_rect.right_center(),
            egui::Align2::RIGHT_CENTER,
            &format_fr_thousands(damage),
            name_font,
            TEXT_COLOR,
            text::OUTLINE_FULL,
        );
    }
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x, rect.max.y - BAR_HEIGHT),
        egui::vec2(rect.width(), BAR_HEIGHT),
    );
    let mut bar_ui = ui.new_child(egui::UiBuilder::new().max_rect(bar_rect));
    bar_ui.multiply_opacity(opacity(bar_rect));
    damage_bar(&mut bar_ui, bar_rect, damage, total_damage);
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
    // Les six couches — deux bordures concentriques, la piste, le remplissage, son reflet et le
    // curseur de fin — vivent dans `design::meter` depuis le 2026-09-11, avec leurs arrondis
    // conditionnels et les tests qui les verrouillent. Ce qui reste ici est le CALCUL de la part de
    // dégâts et sa couleur, qui sont du métier : la teinte varie selon la part, et le composant ne
    // sait pas ce qu'est un combattant.
    let ratio = if total_damage > 0 {
        (damage as f32 / total_damage as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    design::paint_meter(ui, rect, ratio, DAMAGE_ACCENT);
}

/// Formate un entier selon l'usage français : espace tous les 3 chiffres depuis la droite (ex.
/// `113574` → `113 574`) — demande utilisateur explicite : un grand nombre collé était difficile à
/// lire d'un coup d'œil (« je ne sais pas si c'est onze mille ou cent-treize mille »).
///
/// Espace ORDINAIRE (pas insécable) — une première version utilisait une espace insécable
/// (U+00A0), jugée trop discrète face au formatage du jeu lui-même (retour utilisateur, capture de
/// comparaison à l'appui : l'écart entre groupes de chiffres semblait presque absent). Sans risque
/// de retour à la ligne malvenu ici : ce texte est TOUJOURS peint directement via `Painter::text`
/// (voir `design::text::paint_outlined_text`), jamais mis en page par un widget qui pourrait
/// le scinder.
pub(crate) fn format_fr_thousands(n: i64) -> String {
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

// La doc de `show_tooltip_above` tenait ici : quatre retours utilisateur sur le placement d'une
// infobulle et l'ordre de ses replis. Elle est partie avec la fonction le 2026-09-11, dans
// `design::tooltip`, qui la porte intégralement — y compris le cas du switch de camp, seul widget
// de l'interface à n'avoir RÉELLEMENT aucune place au-dessus de lui, et dont la réponse n'est pas
// un repli mais `render_content::COMBAT_TOP_MARGIN`. Les deux switches eux-mêmes
// (`paint_side_switch`, `paint_metric_switch`, ex-`.icon-switch` du dépôt web porté en dessin
// egui direct) sont partis le 2026-09-16 dans `design::switch` — voir doc de module.

/// Le panneau Combat doit-il être affiché pour ce personnage ? — demande du 2026-09-13.
///
/// **Le défaut est l'apparition automatique** (`always_visible == false`, case décochée dans la
/// fenêtre Options) : le panneau n'est à l'écran que pendant un combat. Il apparaît quand le
/// combat commence et se referme quand il est terminé, plutôt que de recouvrir le jeu en
/// permanence avec son « Aucun combat pour l'instant. ».
///
/// **`ongoing`, et rien d'autre.** `SessionSnapshot::fight_for_character` rend aussi le DERNIER
/// combat terminé de ce personnage quand il n'y en a plus en cours (c'est ce qui laisse son récap
/// affiché après la victoire) : se contenter de `is_some()` laisserait donc le panneau ouvert
/// jusqu'au combat suivant, exactement ce que ce réglage doit éviter.
///
/// **`enabled == false` masque en toute circonstance** — case « Activer le détail des combats »
/// décochée (2026-09-15, `panels::feature_switch::FeatureToggles::combat`) : la fonctionnalité
/// entière est coupée, et une case d'encombrement (`always_visible`) ne peut pas rallumer un
/// panneau que son interrupteur éteint. C'est la raison de l'ordre des deux tests ci-dessous.
///
/// Vit ici, et pas dans les deux hôtes qui l'appliquent
/// (`main.rs`/`bin/wakfu-companion-overlay-x11.rs`, où le fenêtrage OS est délibérément dupliqué —
/// voir la doc de `lib.rs`) : c'est une règle du panneau Combat, la même sous Windows et sous X11,
/// et elle se teste sans fenêtre.
pub fn should_show(
    snapshot: &SessionSnapshot,
    character_name: &str,
    always_visible: bool,
    enabled: bool,
) -> bool {
    enabled
        && (always_visible
            || snapshot
                .fight_for_character(character_name)
                .is_some_and(|fight| fight.ongoing))
}

#[cfg(test)]
mod tests {
    use super::{format_fr_thousands, should_show};
    use overlay_engine::{FightResult, FightSnapshot, FighterDamage, Gender, SessionSnapshot};

    /// Un combat où `nom` est allié, en cours ou terminé.
    fn combat_de(nom: &str, ongoing: bool) -> FightSnapshot {
        FightSnapshot {
            fight_id: 1,
            ongoing,
            result: (!ongoing).then_some(FightResult::Won),
            fighters: vec![FighterDamage {
                name: nom.to_string(),
                is_ally: true,
                total_damage: 0,
                total_heal: 0,
                total_armor: 0,
                class_name: None,
                gender: Gender::M,
                xp_gained: 0,
                spells: Default::default(),
                heal_spells: Default::default(),
                armor_spells: Default::default(),
                is_ko: false,
                last_turn_casts: Vec::new(),
                last_turn: 0,
                breed: None,
            }],
            started_at_ms: 0,
            last_ally_caster: None,
            last_enemy_caster: None,
        }
    }

    fn session(fights: Vec<FightSnapshot>) -> SessionSnapshot {
        SessionSnapshot {
            fights,
            ..Default::default()
        }
    }

    #[test]
    fn masque_hors_combat_quand_l_option_est_decochee() {
        assert!(!should_show(&session(Vec::new()), "Oumbra", false, true));
    }

    #[test]
    fn affiche_pendant_un_combat_en_cours() {
        let snapshot = session(vec![combat_de("Oumbra", true)]);
        assert!(should_show(&snapshot, "Oumbra", false, true));
    }

    #[test]
    fn masque_des_que_le_combat_est_termine() {
        // Le combat reste dans le snapshot une fois fini (c'est lui que `fight_for_character` rend
        // alors) : c'est exactement le cas que `is_some()` aurait raté.
        let snapshot = session(vec![combat_de("Oumbra", false)]);
        assert!(!should_show(&snapshot, "Oumbra", false, true));
    }

    #[test]
    fn ignore_le_combat_d_un_autre_personnage() {
        // Multi-compte : chaque fenêtre suit SON personnage, jamais le combat du voisin.
        let snapshot = session(vec![combat_de("Oumbra", true)]);
        assert!(!should_show(&snapshot, "Kaelis", false, true));
    }

    #[test]
    fn l_option_cochee_affiche_en_toute_circonstance() {
        assert!(should_show(&session(Vec::new()), "Oumbra", true, true));
        let termine = session(vec![combat_de("Oumbra", false)]);
        assert!(should_show(&termine, "Oumbra", true, true));
    }

    /// « Activer le détail des combats » décoché : rien ne s'affiche, pas même pendant un combat
    /// en cours — et l'affichage permanent, qui coche pourtant « en toute circonstance » ci-dessus,
    /// ne le rattrape pas. C'est l'interrupteur de la fonctionnalité, pas un réglage
    /// d'encombrement.
    #[test]
    fn le_detail_des_combats_coupe_masque_meme_en_combat() {
        let en_cours = session(vec![combat_de("Oumbra", true)]);
        assert!(!should_show(&en_cours, "Oumbra", false, false));
        assert!(!should_show(&en_cours, "Oumbra", true, false));
        assert!(!should_show(&session(Vec::new()), "Oumbra", true, false));
    }

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
