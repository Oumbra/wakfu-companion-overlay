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
//! ## Licence
//!
//! Ubuntu est publiée sous **Ubuntu Font Licence 1.0**, libre et redistribuable ; le texte de la
//! licence accompagne le fichier dans `assets/fonts/UFL.txt`, comme elle l'exige. 340 Ko dans le
//! binaire, à comparer au budget de 300 Mo du §8 du plan : négligeable.
//!
//! ## Portée : les libellés du design system, pas toute l'application
//!
//! Seule la famille nommée [`LABEL`] est ajoutée. La proportionnelle par défaut d'`egui`
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

/// Fichier embarqué — voir la doc de module pour le pourquoi de cette graisse précise.
const UBUNTU_MEDIUM: &[u8] = include_bytes!("../../../../assets/fonts/Ubuntu-Medium.ttf");

/// Enregistre les familles du design system sur `ctx`.
///
/// Part des définitions **par défaut** d'`egui` et n'y ajoute qu'une famille : la proportionnelle
/// et la monospace d'origine restent intactes, aucun texte existant ne change d'apparence.
///
/// À appeler une fois au démarrage. `Context::set_fonts` reconstruit l'atlas de glyphes : l'appeler
/// à chaque frame le rebâtirait à chaque frame.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        LABEL.to_owned(),
        Arc::new(egui::FontData::from_static(UBUNTU_MEDIUM)),
    );
    fonts
        .families
        .insert(egui::FontFamily::Name(LABEL.into()), vec![LABEL.to_owned()]);
    ctx.set_fonts(fonts);
}
