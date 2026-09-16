//! **Registre des glyphes** du design system — le pendant de [`DsTexture`](super::DsTexture) pour
//! les icônes, séparé de lui le 2026-09-11.
//!
//! ## Pourquoi deux registres et non un
//!
//! Un fond de bouton et un glyphe n'ont pas les mêmes propriétés, et les tenir dans une seule
//! énumération obligeait chacun à porter celles de l'autre :
//!
//! - **Un glyphe n'a pas de 9-slice.** Les 38 icônes déclaraient toutes `ICON_SLICE`, c'est-à-dire
//!   « aucune marge figée, étirement uniforme » — un 9-slice dégénéré, présent uniquement parce que
//!   le champ était obligatoire. Il ne décrivait rien et cachait une vraie information : ce que la
//!   texture *est*.
//! - **Un fond n'a pas d'étalon d'encre.** `icon_content_size` énumérait à la main, dans un `match`
//!   de vingt lignes, les variantes qui sont des icônes normalisées — une liste qu'il fallait tenir
//!   à jour en parallèle de l'énumération, et que rien ne vérifiait.
//!
//! Le défaut que cette confusion a réellement produit est daté : le test de détourage
//! `les_glyphes_d_icone_sont_detoures_au_pixel_pres` sélectionnait ses cibles **par leur
//! découpage** (`insets == 0`), faute de mieux — et a donc attrapé `loader-sheet.png`, une planche
//! d'atlas qui n'est pas un glyphe. Il a fallu ajouter `DsTexture::grid` pour l'en exclure. Avec
//! deux registres, la question ne se pose plus : le test balaie `DsIcon::ALL`.
//!
//! ## La table est la source unique
//!
//! Nom de cache egui, chemin du fichier et présence d'un étalon viennent tous de la ligne
//! ci-dessous, dérivés d'un seul littéral. Il n'y a plus de façon d'écrire `ds-icon-eye` en face de
//! `icon-eye-off.png` — c'était possible tant que les trois étaient recopiés à la main dans
//! `spec()`.
//!
//! ## Les deux catégories d'étalon
//!
//! - **`socle`** : le glyphe a été détouré depuis un socle de bouton icône du jeu (`--from-button`,
//!   voir `references/recettes-icones.md` du skill `design-asset`). Sa taille d'encre est donc
//!   connue et comparable — [`tokens::ICON_BUTTON_CONTENT`] — ce qui permet de le normaliser.
//! - **`libre`** : détouré sans bouton porteur, ou sans mesure consignée. Le normaliser sur un
//!   étalon qu'il ne partage pas le rendrait faux ; il garde sa taille native.
//! - **`couleur`** (2026-09-16) : un glyphe **en couleurs**, pas un blanc à teinter — libre de
//!   taille, et peint tel quel. C'est l'exception à la règle « toutes les icônes sont
//!   monochromes » du skill `design-asset`, réservée aux glyphes dont la couleur porte le sens :
//!   les cinq du panneau Combat (vert = alliés, orange = ennemis ; dague, cœur vert d'armure, cœur
//!   rouge de soin). Passés au blanc par transfert de luminance (essai du 2026-09-16), les deux
//!   silhouettes ne se distinguaient plus que par la position des bras, et les deux cœurs
//!   devenaient des taches. Un composant qui teinte ses glyphes (`design::switch`) peint ceux-ci
//!   en blanc et les **atténue** hors sélection ([`tokens::ICON_NATIVE_DIM`]) au lieu de les
//!   colorer.

use crate::design::tokens;

/// Déclare les glyphes du design system.
///
/// Un seul littéral par icône : le nom du fichier. Le nom de cache egui s'en déduit (`ds-` en
/// préfixe), et le chemin aussi (`icons/<fichier>.png`) — `include_bytes!` exigeant un littéral,
/// c'est la macro qui les assemble plutôt que trois recopies manuelles.
macro_rules! ds_icons {
    ($(
        $(#[$doc:meta])*
        $variant:ident => $file:literal, $etalon:ident;
    )*) => {
        /// Un glyphe du design system. Voir la doc de module.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum DsIcon {
            $(
                $(#[$doc])*
                $variant,
            )*
        }

        impl DsIcon {
            /// Tous les glyphes, dans l'ordre de déclaration — **cet ordre fixe l'index de
            /// stockage** dans `DesignSystem`, il n'a pas d'autre sens.
            pub const ALL: &'static [DsIcon] = &[$(DsIcon::$variant,)*];

            /// Nom de cache egui et octets PNG embarqués.
            pub fn spec(self) -> DsIconSpec {
                match self {
                    $(
                        DsIcon::$variant => DsIconSpec {
                            name: concat!("ds-", $file),
                            bytes: include_bytes!(
                                concat!("../../../../assets/design-system/icons/", $file, ".png")
                            ),
                        },
                    )*
                }
            }

            /// Taille d'encre de référence, quand le glyphe en a une — voir « les deux catégories
            /// d'étalon » dans la doc de module.
            pub fn content_size(self) -> Option<f32> {
                match self {
                    $(DsIcon::$variant => ds_icons!(@etalon $etalon),)*
                }
            }

            /// Le glyphe est-il en couleurs natives (catégorie `couleur`, voir la doc de
            /// module) ? Un composant qui teinte ses glyphes ne doit pas teinter celui-ci.
            pub fn native_color(self) -> bool {
                match self {
                    $(DsIcon::$variant => ds_icons!(@couleur $etalon),)*
                }
            }
        }
    };
    (@etalon socle) => { Some(tokens::ICON_BUTTON_CONTENT) };
    (@etalon libre) => { None };
    (@etalon couleur) => { None };
    (@couleur couleur) => { true };
    (@couleur socle) => { false };
    (@couleur libre) => { false };
}

/// Description statique d'un glyphe : nom de cache egui et octets PNG embarqués.
pub struct DsIconSpec {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

impl DsIcon {
    /// Index de stockage dans `DesignSystem` — la position dans [`DsIcon::ALL`].
    pub fn index(self) -> usize {
        DsIcon::ALL
            .iter()
            .position(|&icon| icon == self)
            .expect("tout glyphe est dans DsIcon::ALL")
    }
}

ds_icons! {
        /// Pastille « i » du bloc d'information (`icons/icon-info.png`, 27 × 28).
        ///
        /// **La première icône entrée au design system**, et pendant un temps la seule — ce qui lui
        /// valait une doc expliquant pourquoi elle n'était pas un 9-slice ; c'est aujourd'hui la
        /// règle de tout ce registre. Blanche dans le fichier, elle prend sa couleur par teinte :
        /// c'est ce qui permet au ton `Alert` de `design::info_text` d'exister sans second fichier.
        Info => "icon-info", libre;

        /// Chevron `⌄` d'une liste déroulante (`icons/icon-chevron-down.png`, 14 × 8) — la taille à
        /// laquelle le jeu le peint, au pixel près.
        ChevronDown => "icon-chevron-down", libre;

        /// Glyphe du bouton Options du carré de contrôle.
        ///
        /// Cette doc portait jusqu'au 2026-09-11 une règle devenue caduque — « la table ne porte que
        /// les icônes réellement utilisées » — écrite quand le manifeste en comptait une poignée.
        /// Le lot du 2026-09-11 a versé les trente-huit du dossier, à la demande de l'utilisateur,
        /// et le coût mesuré a justifié le changement : **31 Ko** décodés en RGBA pour l'ensemble,
        /// sans effet sur le budget de 300 Mo (§8 du plan). Ce qui reste vrai, c'est que tout est
        /// chargé dès le premier composant peint (`DesignSystem::load`).
        Option => "icon-option", socle;

        ExternalLink => "icon-external-link", socle;

        Plus => "icon-plus", socle;

        Minus => "icon-minus", socle;

        /// Loupe d'un champ de recherche (`icons/icon-search.png`, 24 × 24) — le jeu la pose À
        /// L'INTÉRIEUR du champ, collée au bord gauche, jamais sur un socle de bouton
        /// (`interface-hdv-achat.png` x 27..39, `interface-personnage-equiement.png`). Elle n'a donc
        /// pas de `DsIcon::content_size` : sa taille est celle que lui donne le champ qui la porte.
        Search => "icon-search", libre;

        /// Croix de fermeture/retrait (`icons/icon-close.png`, 13 × 14) — le « × » de « Retirer tous
        /// les filtres ». **Pas celle qui ferme une fenêtre** : voir [`DsIcon::CloseWindow`].
        Close => "icon-close", socle;

        /// Bulle de message (`icons/icon-message.png`, 23 × 19) — fournie par l'utilisateur le
        /// 2026-09-14 pour la carte d'alerte de chat du Suivi (« répondre en privé »,
        /// `panels::watchlist::chat_toast_card`). `libre` : pas de socle, sa taille est celle que la
        /// carte lui donne.
        ///
        /// Le fichier est arrivé avec une marge transparente (canevas 28 × 28 pour une encre de
        /// 22 × 19), que `les_glyphes_d_icone_sont_detoures_au_pixel_pres` a signalée : `glyph_fit`
        /// mettant le CANEVAS à l'échelle, cette marge rétrécissait le glyphe en silence chez son
        /// appelant. Détouré le 2026-09-14 à sa boîte d'alpha non nul — frange d'antialiasing
        /// comprise, d'où le pixel d'écart entre canevas et encre que le test tolère — et
        /// `watchlist::CHAT_CARD_ICON_SIZE` recalée dans le même mouvement pour que la carte peigne
        /// exactement la même bulle qu'avant.
        Message => "icon-message", libre;

        /// Croix du bouton de fermeture d'une fenêtre (`icons/icon-close-window.png`, 12 × 12) —
        /// plus grasse que [`DsIcon::Close`] (traits de ~2,7 px contre 2), et légèrement
        /// asymétrique, comme le jeu la peint dans la bannière de toutes ses fenêtres
        /// (`interface-hdv-achat.png`, coin haut-droit, même dessin à une autre échelle).
        ///
        /// Démêlée des captures `window-close.png`/`window-close-hover.png` par
        /// `tools/design-system/build_window_close.py`, pas par `dsimg.py` (qui perd sa frange).
        /// `libre` : elle n'a jamais vécu sur un socle de bouton icône, sa taille d'encre est
        /// celle du contexte qui la porte ([`tokens::WINDOW_CLOSE_ICON_CONTENT`], 12 pour 32).
        CloseWindow => "icon-close-window", libre;

        /// Corbeille (`icons/icon-delete.png`, 12 × 14) — la suppression d'un élément d'une liste, sur
        /// socle de bouton icône (`interface-personnage-equiement.png`, barre d'outils du build).
        Delete => "icon-delete", socle;

        /// Crayon (`icons/icon-edit.png`, 14 × 14) — **modifier une entrée d'une liste**, le
        /// pendant de [`DsIcon::Delete`] avec qui il partage la barre d'outils d'une tuile.
        ///
        /// Détouré par l'utilisateur le 2026-09-16 (démélange direct fond → glyphe) pour l'onglet
        /// « Personnages », où le bouton de modification portait jusque-là [`DsIcon::Option`] faute
        /// de crayon au registre. Le trait diagonal est le crayon ; les trois tirets sous lui
        /// appartiennent au dessin, ce n'est pas une frange de détourage.
        Edit => "icon-edit", socle;

        /// Point d'interrogation (`icons/icon-help.png`, 12 × 12) — le bouton d'aide en tête de
        /// fenêtre (`interface-personnage-equiement.png`, coin haut-droit).
        Help => "icon-help", socle;

        /// Coche (`icons/icon-tick.png`, 12 × 9) — le marqueur « actif » du jeu, à côté d'un libellé
        /// (« ✓ Actif ») plutôt que sur un socle : pas de `DsIcon::content_size` pour la même raison que
        /// [`DsIcon::Search`].
        Tick => "icon-tick", libre;

        /// Flèche de réinitialisation (`icons/icon-undo.png`, 14 × 12) — le bouton « rétablir les
        /// valeurs par défaut » de la fenêtre Options (`interface-options-son.png`, coin haut-droit).
        Undo => "icon-undo", socle;

        /// Haut-parleur (`icons/icon-volume.png`, 26 × 22) — glyphe de volume actif, détouré le
        /// 2026-09-11 (skill `design-asset`) depuis une capture du jeu sans socle porteur : le glyphe
        /// est directement posé sur le décor, contrairement aux icônes prélevées sur un bouton.
        ///
        /// **Pas encore d'appelant** : aucun réglage de son n'est câblé dans l'overlay à ce jour —
        /// `alert_sound` ne fait que jouer les sons, jamais les couper. Entrée au manifeste comme
        /// [`DsTexture::ModalHeader`] avant sa bannière : préparée pour le futur bouton muet/actif de
        /// la fenêtre Options (`interface-options-son.png`), posée sur un socle de bouton icône —
        /// d'où `DsIcon::content_size`, comme les glyphes de la même famille.
        Volume => "icon-volume", socle;

        /// Haut-parleur barré (`icons/icon-volume-mute.png`, 26 × 26) — pendant coupé de
        /// [`DsIcon::Volume`], même provenance et même statut (pas encore d'appelant).
        VolumeMute => "icon-volume-mute", socle;

        /// Œil ouvert (`icons/icon-eye.png`, 16 × 14) — glyphe SOMBRE, détouré le 2026-09-11 (skill
        /// `design-asset`, `--polarity dark --keep center --floor 45`) depuis un crop du jeu sans
        /// socle porteur, contrairement aux icônes prélevées sur un bouton.
        ///
        /// **Pas encore d'appelant** : aucun bascule affiché/masqué n'existe dans l'overlay à ce jour.
        /// Entrée au manifeste comme [`DsIcon::Volume`] avant son futur bouton — préparée pour
        /// un toggle de visibilité (candidat naturel : masquer une entrée du panneau Suivi, ou un champ
        /// de jeton dans la fenêtre Options). Posée sur un socle de bouton icône, d'où
        /// `DsIcon::content_size`.
        Eye => "icon-eye", socle;

        /// Œil barré (`icons/icon-eye-off.png`, 16 × 14) — pendant coupé de [`DsIcon::Eye`],
        /// même provenance et même statut (pas encore d'appelant).
        EyeOff => "icon-eye-off", socle;

        /// Sac plein, flèche vers l'intérieur (`icons/icon-bag-in.png`, 10 × 16) — dépôt/réception
        /// d'objets. Détouré `--from-button` (voir `references/recettes-icones.md` du skill
        /// `design-asset`, commit `chore: renomme les captures d'icônes`) : vivait sur un socle de
        /// bouton icône dans le jeu, d'où `DsIcon::content_size`.
        ///
        /// **Pas encore d'appelant**, comme les 21 icônes suivantes de ce lot : détourées avant tout
        /// composant qui les consomme, à la différence des icônes plus anciennes du manifeste,
        /// entrées en même temps que leur premier usage. Rattachées au manifeste le 2026-09-11 en
        /// réponse à un écart relevé par l'utilisateur (« pas tous les icônes du répertoire dans le
        /// manifeste ») — voir le delta `assets/design-system/icons/` vs `DsTexture::ALL` qui a guidé
        /// cet ajout.
        BagIn => "icon-bag-in", socle;

        /// Sac, flèches de retrait (`icons/icon-bag-out.png`, 12 × 12) — retrait/échange d'objets.
        /// Détouré `--from-button`, même provenance et même statut que [`DsIcon::BagIn`].
        BagOut => "icon-bag-out", socle;

        /// Livre ouvert (`icons/icon-book.png`, 22 × 18) — encyclopédie/recette. Détouré SANS
        /// `--from-button` : posé sur le décor turquoise d'un panneau, pas sur un socle de bouton
        /// icône — pas de `DsIcon::content_size`, comme [`DsIcon::Search`]. Même statut que
        /// [`DsIcon::BagIn`] (pas encore d'appelant).
        Book => "icon-book", libre;

        /// Calendrier (`icons/icon-calendar.png`, 22 × 22) — grille à cases. Détouré SANS
        /// `--from-button`, sur socle sombre uni. Pas de `DsIcon::content_size`. Même statut que
        /// [`DsIcon::BagIn`].
        Calendar => "icon-calendar", libre;

        /// Emplacement de cartes (`icons/icon-cards.png`, 12 × 12) — deux rectangles superposés.
        /// Provenance : commit `feat: huit icones et variante hover du bouton valider` (2026-09-09),
        /// sans mesure `--from-button` consignée — pas de `DsIcon::content_size` par prudence, faute de
        /// certitude sur le socle porteur. Même statut que [`DsIcon::BagIn`].
        Cards => "icon-cards", libre;

        /// Trois silhouettes (`icons/icon-characters.png`, 20 × 14) — bascule de personnage.
        /// Provenance : commit `feat: icone des personnages (traitement retenu)` (2026-09-09), teinte
        /// blanche par luma retenue après comparatif de cinq traitements. Pas de `DsIcon::content_size`,
        /// même raison que [`DsIcon::Cards`]. Même statut que [`DsIcon::BagIn`].
        Characters => "icon-characters", libre;

        /// Entonnoir (`icons/icon-filter.png`, 12 × 12) — filtre. Détouré `--from-button`, d'où
        /// `DsIcon::content_size`. Même statut que [`DsIcon::BagIn`].
        Filter => "icon-filter", socle;

        /// Grille de neuf points (`icons/icon-grid.png`, 18 × 18) — vue en grille. Même provenance et
        /// la même prudence sur `DsIcon::content_size` que [`DsIcon::Cards`] (pas de mesure
        /// `--from-button` consignée). Même statut que [`DsIcon::BagIn`].
        Grid => "icon-grid", libre;

        /// Marteau (`icons/icon-hammer.png`, 16 × 16) — artisanat/métier. Provenance : commit `style:
        /// ajout d'assets pour le design system en vu de la modélisation de la modal d'options`
        /// (2026-09-08), sans mesure `--from-button` consignée — pas de `DsIcon::content_size` par la
        /// même prudence que [`DsIcon::Cards`]. Même statut que [`DsIcon::BagIn`].
        Hammer => "icon-hammer", libre;

        /// Pile de kamas (`icons/icon-kamas.png`, 14 × 12) — monnaie du jeu. Détouré SANS
        /// `--from-button`, sur socle sombre uni. Pas de `DsIcon::content_size`. Même statut que
        /// [`DsIcon::BagIn`].
        Kamas => "icon-kamas", libre;

        /// Cadenas fermé (`icons/icon-lock.png`, 12 × 14) — verrouillé. Détouré `--from-button`, d'où
        /// `DsIcon::content_size`. Même statut que [`DsIcon::BagIn`].
        Lock => "icon-lock", socle;

        /// Histogramme croissant (`icons/icon-order.png`, 14 × 14) — tri par valeur. Détouré
        /// `--from-button --floor 45` (ombre portée du glyphe sur aplat, voir
        /// `references/recettes-icones.md`), d'où `DsIcon::content_size`. Même statut que
        /// [`DsIcon::BagIn`].
        Order => "icon-order", socle;

        /// Masque à cornes (`icons/icon-pact.png`, 14 × 13) — pacte. Détouré `--from-button`, d'où
        /// `DsIcon::content_size`. Même statut que [`DsIcon::BagIn`].
        Pact => "icon-pact", socle;

        /// Punaise (`icons/icon-pin.png`, 14 × 14) — épingler/favori. Détouré SANS `--from-button` :
        /// glyphe posé à même le décor sombre, sans socle de bouton — pas de `DsIcon::content_size`.
        /// Même statut que [`DsIcon::BagIn`].
        Pin => "icon-pin", libre;

        /// Deux flèches en boucle (`icons/icon-repeat.png`, 22 × 22) — répétition/rafraîchissement.
        /// Détouré SANS `--from-button`, sur socle sombre uni. Pas de `DsIcon::content_size`. Même statut
        /// que [`DsIcon::BagIn`].
        Repeat => "icon-repeat", libre;

        /// Disquette (`icons/icon-save.png`, 12 × 12) — enregistrer. Détouré `--tol 4 --polarity dark
        /// --keep center --floor 45` `--from-button` (voir `references/recettes-icones.md`), d'où
        /// `DsIcon::content_size`. Même statut que [`DsIcon::BagIn`].
        Save => "icon-save", socle;

        /// Engrenage, première variante (`icons/icon-settings-1.png`, 14 × 14) — paramètres.
        /// Provenance : commit `feat: huit icones et variante hover du bouton valider` (2026-09-09,
        /// « deux variantes d'engrenage »), sans mesure `--from-button` consignée — pas de
        /// `DsIcon::content_size` par la même prudence que [`DsIcon::Cards`]. Même statut que
        /// [`DsIcon::BagIn`].
        Settings1 => "icon-settings-1", libre;

        /// Engrenage, seconde variante (`icons/icon-settings-2.png`, 14 × 14) — pendant de
        /// [`DsIcon::Settings1`], même provenance, même prudence, même statut.
        Settings2 => "icon-settings-2", libre;

        /// Flèches haut/bas (`icons/icon-sort.png`, 16 × 12) — tri. Détouré `--from-button`, d'où
        /// `DsIcon::content_size`. Même statut que [`DsIcon::BagIn`].
        Sort => "icon-sort", socle;

        /// Triangle (`icons/icon-triangle-right.png`, 7 × 10) — déplier/lire, et les deux flèches
        /// de la pagination. Détouré `--from-button --tol 4` (bouton sombre proche du décor, voir
        /// `references/recettes-icones.md`).
        ///
        /// **Il pointe vers la GAUCHE**, contrairement à ce que son nom laisse croire : la pointe
        /// est en x=0, la base en x=6 (vérifié sur le canal alpha). Le nom du fichier vient du
        /// jeu ; c'est la mesure qui fait foi, et `IconButton::mirrored` donne l'autre sens.
        ///
        /// **`libre` et non `socle`, corrigé le 2026-09-12.** L'étalon des glyphes de bouton vaut
        /// 18 px d'encre pour un socle de 36 ; celui-ci en fait 10. Mesure directe sur les flèches
        /// de pagination d'`interface-hdv-historique.png` : **8 × 10 px d'encre dans un socle de
        /// 36**, soit la taille native de l'asset à un pixel de détourage près. Le normaliser à 18
        /// l'agrandissait de 80 %, ce que la première capture du composant a montré sans
        /// ambiguïté. Un glyphe détouré depuis un bouton n'est donc pas automatiquement sur la
        /// grille de 18 — c'est ce que cette variante apprend au registre.
        TriangleRight => "icon-triangle-right", libre;

        /// Coupe (`icons/icon-trophy.png`, 22 × 22) — trophée/haut fait. Détouré SANS `--from-button`,
        /// sur socle sombre uni. Pas de `DsIcon::content_size`. Même statut que [`DsIcon::BagIn`].
        Trophy => "icon-trophy", libre;

        /// « Xp » barré d'une croix (`icons/icon-xp.png`, 16 × 11) — points d'expérience. Détouré SANS
        /// `--from-button`, sur socle sombre uni. Pas de `DsIcon::content_size`. Même statut que
        /// [`DsIcon::BagIn`].
        Xp => "icon-xp", libre;

        /// Épée (`icons/icon-sword.png`, 24 × 25) — combat. Détourée le 2026-09-16 par
        /// `demix_flat` (glyphe blanc à même le décor uni, sans bouton porteur — voir
        /// `references/recettes-icones.md` du skill `design-asset`), d'où `libre`. Premier
        /// appelant : la case « combats gagnés − perdus » de la bande Récap (`panels::recap`), où
        /// elle remplace la dague colorée [`DsIcon::MetricDamage`] par un glyphe blanc à teinter
        /// comme ses voisins.
        Sword => "icon-sword", libre;

        /// Horloge (`icons/icon-clock.png`, 19 × 19) — durée. Même provenance, même recette et même
        /// statut que [`DsIcon::Sword`]. Premier appelant : la case « durée de session » de la bande
        /// Récap, où elle remplace [`DsIcon::Calendar`] — le registre porte enfin un cadran, la
        /// bande n'a plus à emprunter une grille de calendrier pour dire une heure.
        Clock => "icon-clock", libre;

        /// Signe ♂ (`icons/icon-male.png`, 14 × 14) — genre masculin, case gauche du switch de
        /// genre du jeu. Détouré le 2026-09-16 SANS `--from-button` (glyphe posé sur le décor
        /// sombre de la capture, réglages par défaut, `luma-light`). Pas de `DsIcon::content_size` :
        /// le jeu le peint à sa taille native dans une case de 40px, c'est cette taille qui fait foi
        /// (voir `design::switch`).
        Male => "icon-male", libre;

        /// Signe ♀ (`icons/icon-female.png`, 10 × 16) — genre féminin, case droite du même switch.
        /// Même provenance, même réglage et même statut que [`DsIcon::Male`].
        Female => "icon-female", libre;

        /// Silhouette verte d'allié (`icons/icon-allies.png`, 18 × 22) — case gauche du switch
        /// de camp du panneau Combat. Le fichier du dépôt web (`header-allies.png`), rogné à sa
        /// boîte d'encre, entré au design system le 2026-09-16 avec la migration du switch ; il
        /// vivait jusque-là dans `ui_icons.rs`. En couleurs : voir la catégorie `couleur`.
        Allies => "icon-allies", couleur;

        /// Silhouette orange d'ennemi (`icons/icon-enemies.png`, 20 × 22) — case droite du même
        /// switch. Même provenance et même statut que [`DsIcon::Allies`].
        Enemies => "icon-enemies", couleur;

        /// Dague (`icons/icon-metric-damage.png`, 18 × 17) — dégâts infligés, première case du
        /// switch de grandeur du panneau Combat. Icône DU JEU, celle du sélecteur web
        /// `app-entity-stat-tabs` (`icons/di.png` du dépôt communautaire `Vertylo/wakassets`),
        /// embarquée depuis le 2026-09-14 (`ui_icons.rs`), au design system depuis le
        /// 2026-09-16. En couleurs : voir la catégorie `couleur`.
        MetricDamage => "icon-metric-damage", couleur;

        /// Cœur vert à flèche (`icons/icon-metric-armor.png`, 22 × 20) — armure donnée, case du
        /// milieu. `aptitudes/234.png` du même dépôt ; même statut que [`DsIcon::MetricDamage`].
        MetricArmor => "icon-metric-armor", couleur;

        /// Cœur rouge à croix (`icons/icon-metric-heal.png`, 22 × 20) — soins prodigués, dernière
        /// case. `aptitudes/12.png` du même dépôt ; même statut que [`DsIcon::MetricDamage`].
        MetricHeal => "icon-metric-heal", couleur;
}
