//! **Polices du design system** — enregistrement des familles nommées auprès d'`egui`.
//!
//! ## Pourquoi un fichier de police embarqué
//!
//! `egui` n'embarque, via `epaint_default_fonts`, que quatre fichiers : `Ubuntu-Light.ttf`
//! (l'unique proportionnelle), `Hack-Regular.ttf` (monospace) et deux fontes d'émoji. **Aucune
//! graisse** : ni Regular, ni Medium, ni Bold — et l'API de mise en forme n'expose pas davantage de
//! réglage de graisse, il n'y a donc rien à *demander*.
//!
//! La première tentative (2026-09-09, matin) contournait ce manque par une **graisse synthétique** :
//! la galley repeinte sur ses huit voisins immédiats à opacité réduite. Le balayage de l'après-midi
//! a montré ce que cette rustine coûte, et il se voit sans mesurer — *un halo est un contour, pas
//! une graisse*. Il épaissit le mot en dégradant son contraste : le libellé devient flou, alors que
//! celui du jeu est net. Une vraie graisse épaissit les fûts en gardant les arêtes franches.
//!
//! Or rien n'obligeait à se contenter de ce qu'`egui` embarque : `FontDefinitions` accepte
//! n'importe quel fichier que **nous** embarquons. La rustine n'était pas la seule option, c'était
//! la seule option *sans ajouter de fichier au dépôt* — et le fichier coûte moins cher que la
//! rustine.
//!
//! ## Pourquoi Ubuntu Medium
//!
//! Onze candidats ont été rendus par `egui` sur les textures réelles 338×36, puis comparés au pixel
//! aux libellés gravés du jeu (`assets/design-system/large-button-cancel.png` et
//! `large-button-validate.png`). Critères : boîte d'encre du mot, et recouvrement de forme (IoU des
//! pixels à plus de 50 % d'opacité, boîtes alignées) avec la référence.
//!
//! | Candidat | « Annuler » | « Valider » | recouvrement |
//! | --- | --- | --- | --- |
//! | jeu (référence) | 13 × 62 | 13 × 59 | — |
//! | Light 17 + halo 0,20 (l'ancienne rustine) | 13 × 60 | 14 × 53 | 0,43 / 0,36 |
//! | Regular 17 | 13 × 61 | 13 × 54 | 0,59 / 0,32 |
//! | **Medium 17** | **13 × 63** | **13 × 57** | **0,52 / 0,52** |
//! | Bold 17 | 13 × 65 | 13 × 59 | 0,38 / 0,75 |
//!
//! Le désaccord entre les deux mots n'est pas un défaut de mesure : **le jeu lui-même n'est pas
//! cohérent**, son « Valider » sombre sur or est nettement plus gras que son « Annuler » blanc sur
//! rouge (masse d'encre 268 contre 168 à hauteur d'encre identique, probablement une ombre cuite
//! dans la capture). Aucune police ne peut coller aux deux. Medium 17 est le seul candidat
//! au-dessus de 0,50 de recouvrement sur les **deux**, d'où le choix — arbitré à la publication de
//! l'artefact du 2026-09-09, l'autre option défendable étant Regular, plus léger.
//!
//! ## Pourquoi PT Serif Bold pour les titres
//!
//! Le jeu n'utilise **pas une seule police** : ses libellés de bouton sont dans une linéale, mais
//! ses titres (« Options » sur la banniere de modale, « Barres de raccourcis » en titre de section)
//! sont dans une **serif grasse**. Vingt-six serifs libres ou déjà présentes sur le poste ont été
//! rendues puis comparées aux deux échantillons du jeu (recouvrement de forme au meilleur
//! décalage, boîtes d'encre alignées). PT Serif Bold est la meilleure **libre** : l'écart de
//! chasse au mot « Options » n'est que de ~2 % (215 px mesurés dans le jeu, 220 px rendus).
//!
//! Publiée sous **SIL Open Font License 1.1**, texte joint dans `assets/fonts/OFL-PTSerif.txt`
//! comme la licence l'exige. 340 Ko, même ordre de grandeur qu'Ubuntu Medium.
//!
//! Le corps retenu est **21**, arbitré par l'utilisateur sur planche de comparaison le 2026-09-09.
//! À noter, parce que ce n'est pas une erreur mais un choix : le corps 22 colle **exactement** à
//! l'encre du jeu (21 × 82 px contre 21 × 82) là où le 21 rend 20 × 78. L'utilisateur préfère le
//! 21, jugé mieux proportionné à la bannière — ne pas le « corriger » à 22 en croyant rattraper un
//! écart de mesure.
//!
//! ## Licence
//!
//! Ubuntu est publiée sous **Ubuntu Font Licence 1.0**, libre et redistribuable ; le texte de la
//! licence accompagne le fichier dans `assets/fonts/UFL.txt`, comme elle l'exige. 340 Ko dans le
//! binaire, à comparer au budget de 300 Mo du §8 du plan : négligeable.
//!
//! ## Portée : les libellés du design system, pas toute l'application
//!
//! Seules les familles nommées [`LABEL`] et [`TITLE`] sont ajoutées. La proportionnelle par défaut d'`egui`
//! (Ubuntu Light) reste celle de tout le reste de l'interface — panneaux Combat et Suivi compris.
//! Basculer aussi le texte courant demanderait de mesurer la graisse du texte courant du jeu, ce
//! qui n'a pas été fait ; le faire au passage aurait changé toutes les captures de non-régression
//! sans qu'aucune mesure ne le justifie.
//!
//! ## Dépendance d'appel
//!
//! [`install`] est appelé par [`crate::style::apply`], comme il pose déjà le reste du style ; les
//! deux binaires et tous les harnais de test l'appellent. `epaint` **panique** sur une famille
//! nommée inconnue (« FontFamily::Name(…) is not bound to any fonts »), et `Context::set_fonts` ne
//! prend effet qu'à la passe suivante — [`super::text::label_font`] retombe donc sur la
//! proportionnelle par défaut tant que la famille n'est pas liée, ce qui couvre la toute première
//! passe des harnais, où `style::apply` est appelé depuis la fermeture d'interface elle-même.

use std::sync::Arc;

/// Nom de la famille des libellés du design system. Un identifiant `egui`, pas un nom de fichier :
/// changer la police se fait dans [`install`] seul, aucun composant ne cite un fichier.
pub const LABEL: &str = "ds-label";

/// Nom de la famille des titres du design system (bannière de modale, titre de section) — la
/// serif grasse du jeu, distincte de la linéale des libellés. Voir la doc de module.
pub const TITLE: &str = "ds-title";

/// Fichier embarqué — voir la doc de module pour le pourquoi de cette graisse précise.
const UBUNTU_MEDIUM: &[u8] = include_bytes!("../../../../assets/fonts/Ubuntu-Medium.ttf");

/// Fichier embarqué des titres — voir la doc de module pour le choix de cette serif.
const PT_SERIF_BOLD: &[u8] = include_bytes!("../../../../assets/fonts/PTSerif-Bold.ttf");

/// Enregistre les familles du design system sur `ctx`.
///
/// Part des définitions **par défaut** d'`egui` et n'y ajoute que des familles **nommées** : la
/// proportionnelle et la monospace d'origine restent intactes, aucun texte existant ne change
/// d'apparence tant qu'il ne demande pas explicitement l'une des familles du design system.
///
/// À appeler une fois au démarrage. `Context::set_fonts` reconstruit l'atlas de glyphes : l'appeler
/// à chaque frame le rebâtirait à chaque frame.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (nom, fichier) in [(LABEL, UBUNTU_MEDIUM), (TITLE, PT_SERIF_BOLD)] {
        fonts.font_data.insert(
            nom.to_owned(),
            Arc::new(egui::FontData::from_static(fichier)),
        );
        fonts
            .families
            .insert(egui::FontFamily::Name(nom.into()), vec![nom.to_owned()]);
    }
    ctx.set_fonts(fonts);
}
