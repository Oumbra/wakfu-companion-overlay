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
        }
    };
    (@etalon socle) => { Some(tokens::ICON_BUTTON_CONTENT) };
    (@etalon libre) => { None };
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
        /// les filtres » et le bouton de fermeture d'une fenêtre.
        Close => "icon-close", socle;

        /// Corbeille (`icons/icon-delete.png`, 12 × 14) — la suppression d'un élément d'une liste, sur
        /// socle de bouton icône (`interface-personnage-equiement.png`, barre d'outils du build).
        Delete => "icon-delete", socle;

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

        /// Triangle vers la droite (`icons/icon-triangle-right.png`, 7 × 10) — déplier/lire. Détouré
        /// `--from-button --tol 4` (bouton sombre proche du décor, voir
        /// `references/recettes-icones.md`), d'où `DsIcon::content_size`. Même statut que
        /// [`DsIcon::BagIn`].
        TriangleRight => "icon-triangle-right", socle;

        /// Coupe (`icons/icon-trophy.png`, 22 × 22) — trophée/haut fait. Détouré SANS `--from-button`,
        /// sur socle sombre uni. Pas de `DsIcon::content_size`. Même statut que [`DsIcon::BagIn`].
        Trophy => "icon-trophy", libre;

        /// « Xp » barré d'une croix (`icons/icon-xp.png`, 16 × 11) — points d'expérience. Détouré SANS
        /// `--from-button`, sur socle sombre uni. Pas de `DsIcon::content_size`. Même statut que
        /// [`DsIcon::BagIn`].
        Xp => "icon-xp", libre;
}
