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
//! retour n'avaient corrigé que les REPLIS du placement (aujourd'hui dans `design::tooltip`) (quel `RectAlign::BOTTOM*`
//! choisir une fois `TOP` rejeté) sans jamais s'attaquer à la cause — le switch Alliés/Ennemis reste
//! collé au bord SUPÉRIEUR du panneau (`inner_margin` nul), donc `TOP` échoue TOUJOURS pour lui,
//! quel que soit le repli disponible : les deux infobulles s'affichaient en dessous plutôt qu'au-
//! dessus (nouveau retour utilisateur explicite : « les tooltips du switch alliés/ennemis
//! s'affichent en dessous au lieu d'au dessus [...] agrandis légèrement l'overlay combat »). Voir
//! `render_content::COMBAT_TOP_MARGIN` : le panneau gagne une marge haute (44px) suffisante pour que
//! `TOP` tienne enfin — la fenêtre Combat (`main.rs`/`bin/overlay-ui-x11.rs::WINDOW_SIZE`) est
//! agrandie d'autant pour ne rien compresser d'autre.
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

use overlay_engine::{CatalogIndex, FightSnapshot, FighterDamage};

use crate::design::{self, text};
use crate::portraits::PortraitAtlas;
use crate::remote_icons::{RemoteIconStore, RemoteIconTextures};
use crate::ui_icons::UiIcons;

use super::combat_frame::{CombatFrame, MAX_FRAME_SLOTS};
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
    /// Inverse le camp affiché — utilisé par le raccourci global `Ctrl+Shift+E`
    /// (`main.rs::App::toggle_combat_side`) en plus du clic direct sur `paint_side_switch` : retour
    /// utilisateur explicite (« en mode toggle, c'est-à-dire que quand on appuie, ça inverse la
    /// sélection »), un seul raccourci pour les deux camps plutôt qu'un par camp.
    pub fn toggled(self) -> Self {
        match self {
            Self::Allies => Self::Enemies,
            Self::Enemies => Self::Allies,
        }
    }
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

// L'accent du contenu flottant, repris du jeton partagé plutôt que recopié : ce panneau et le
// panneau Suivi portaient la même valeur en deux constantes locales sans lien déclaré entre elles.
// Voir `tokens::OVERLAY_ACCENT`, qui porte la décision et sa raison.
use crate::design::tokens::OVERLAY_ACCENT as ACCENT;
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
/// Largeur maximale d'une barre — agrandie par rapport à la première version de cette refonte
/// (150 px) maintenant que `COLUMN_GAP` est réduit (voir sa doc) : l'espace regagné doit profiter
/// à la barre, pas rester vide.
pub(super) const BAR_MAX_WIDTH: f32 = 190.0;
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

const TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(235, 240, 245);

const TOTAL_FONT_SIZE: f32 = 18.0;
/// Écart entre la ligne leader et le premier groupe — réduit en cohérence avec `ROW_GAP`.
const TOTAL_GAP: f32 = 3.0;
const NAME_FONT_SIZE: f32 = 13.0;

/// Marge intérieure du fond opacifié de la ligne leader (voir `show_leader_row`) entre son bord et
/// le bouton/le total qu'il contient — la MÊME valeur des deux côtés (le bouton à gauche a un bord
/// net, contrairement au dernier chiffre du total dont le glyphe laisse un peu de son propre
/// espacement interne avant l'encre visible : l'écart géométrique posé ici est bien symétrique,
/// même si l'œil peut lire une petite différence côté texte — retour utilisateur, 7e retour).
const LEADER_PANEL_PADDING: f32 = 6.0;
/// Arrondi du fond opacifié de la ligne leader.
pub(super) const LEADER_PANEL_ROUNDING: f32 = 6.0;
/// Couleur du fond opacifié de la ligne leader — approximation d'un bandeau translucide du jeu
/// (captures d'écran de référence sans canal alpha exploitable, voir doc de module) : noir
/// bleuté, assez opaque pour détacher la ligne du reste sans devenir un pavé plein.
pub(super) const LEADER_PANEL_FILL: egui::Color32 =
    egui::Color32::from_rgba_unmultiplied_const(10, 12, 16, 150);

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
    let mut bars: Vec<&FighterDamage> = fighters
        .iter()
        .copied()
        .filter(|f| f.total_damage > 0)
        .collect();
    bars.sort_by_key(|f| std::cmp::Reverse(f.total_damage));

    // Vrai total (0 tant qu'il n'y a pas de combat, ou que le camp affiché est vide — on l'affiche
    // tel quel, voir `show_leader_row`) — `total_damage` (avec `.max(1)`) n'existe que pour
    // sécuriser les divisions de ratio ; sans effet sur le résultat puisqu'un dégât nul donne de
    // toute façon un ratio nul.
    let total_damage_raw = fighters.iter().map(|f| f.total_damage).sum::<i64>();
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

    ui.horizontal_top(|ui| {
        // Colonne de gauche : portraits — cadre exact (alliés ou ennemis jusqu'à `MAX_FRAME_SLOTS`),
        // cadre à défilement (ennemis au-delà), ou liste plate (alliés excédentaires) — voir doc de
        // module.
        ui.vertical(|ui| {
            if !framed.is_empty() {
                frame.show(
                    ui,
                    portraits,
                    icons,
                    catalog,
                    remote_icons,
                    remote_icon_textures,
                    framed,
                    total_damage,
                );
            }
            if !enemy_scroll.is_empty() {
                EnemyFrameScroll::show(
                    ui,
                    frame,
                    portraits,
                    icons,
                    catalog,
                    remote_icons,
                    remote_icon_textures,
                    enemy_scroll,
                    total_damage,
                );
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
            // Bloc « ligne de sorts » (voir `combat_spell_block`) : après le dernier groupe, dès
            // qu'un allié du combat a lancé un sort (avant, rien — pas même l'espace) — quel que
            // soit le camp affiché au-dessus, il porte sur les ALLIÉS du combat (décision
            // artefact : visible aussi sur la vue Ennemis). `BLOCK_GAP` est l'air VISIBLE voulu :
            // egui glisse déjà `item_spacing.y` après le dernier widget, retranché ici pour ne pas
            // le compter deux fois.
            if let Some(fight) = fight.filter(|f| combat_spell_block::has_casting_ally(f)) {
                ui.add_space(combat_spell_block::BLOCK_GAP - ui.spacing().item_spacing.y);
                combat_spell_block::show(
                    ui,
                    fight,
                    portraits,
                    icons,
                    overlay_engine::SpellIndex::embedded(),
                    remote_icons,
                    remote_icon_textures,
                );
            }
        });
    });
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
    let total_font = text::label_font(ui.ctx(), TOTAL_FONT_SIZE);
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

    text::paint_outlined_text(
        ui,
        egui::pos2(row_rect.max.x - LEADER_PANEL_PADDING, row_rect.center().y),
        egui::Align2::RIGHT_CENTER,
        &format_fr_thousands(total_damage),
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
            .percent(
                (fighter.total_damage > 0)
                    .then(|| design::portrait_percent(fighter.total_damage, total_damage)),
            ),
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

/// Un "groupe" nom + dégâts + barre de la colonne de droite — nom à gauche et dégâts chiffrés à
/// droite sur la MÊME ligne (retour utilisateur : le chiffre de dégâts avait disparu avec le
/// passage au pourcentage seul, régression à corriger), barre juste en dessous, quasiment collée
/// (`GROUP_NAME_BAR_GAP`) pour que l'œil lise l'ensemble comme un seul bloc — et un espace plus
/// large (`ROW_GAP`, voir l'appelant) entre deux groupes DIFFÉRENTS pour que cette distinction
/// reste lisible.
fn damage_bar_group(ui: &mut egui::Ui, name: &str, damage: i64, total_damage: i64) {
    let bar_width = ui.available_width().min(BAR_MAX_WIDTH);
    let name_font = text::label_font(ui.ctx(), NAME_FONT_SIZE);
    // Pas de rembourrage supplémentaire sous le texte (retour utilisateur, 6e retour : « l'écart
    // entre la barre et la ligne du dessus », déjà réduit une 1re fois via `GROUP_NAME_BAR_GAP` —
    // le reste venait de cette marge, retirée).
    let name_height = name_font.size;
    let (name_rect, _) =
        ui.allocate_exact_size(egui::vec2(bar_width, name_height), egui::Sense::hover());
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

// La doc de `show_tooltip_above` tenait ici : quatre retours utilisateur sur le placement d'une
// infobulle et l'ordre de ses replis. Elle est partie avec la fonction le 2026-09-11, dans
// `design::tooltip`, qui la porte intégralement — y compris le cas du switch ci-dessous, seul
// widget de l'interface à n'avoir RÉELLEMENT aucune place au-dessus de lui, et dont la réponse
// n'est pas un repli mais `render_content::COMBAT_TOP_MARGIN`.

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
    design::tooltip(&allies_response).text("Alliés (Ctrl+Shift+E)");
    let enemies_response = ui
        .interact(
            enemies_rect,
            ui.id().with("combat-side-enemies"),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    design::tooltip(&enemies_response).text("Ennemis (Ctrl+Shift+E)");
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
