//! **Pas numérique** du design system Wakfu — le « − valeur + » du jeu. Composant **feuille**
//! (§1 du contrat) : il ne prend aucun contenu, il mute une valeur.
//!
//! ```ignore
//! use overlay_ui::design;
//!
//! ui.add(design::stepper(&mut quantite).range(1..=999).log_name("hdv-quantite"));
//! ```
//!
//! ## Ce qu'il compose, et ce qu'il n'invente pas
//!
//! Trois composants existants, posés côte à côte : deux [`design::icon_button`](super::icon_button)
//! au contexte [`IconContext::Stepper`], et un [`design::input`](super::input) entre les deux. Rien
//! n'est repeint à la main, et le glyphe des boutons vient du manifeste — le socle aussi.
//!
//! **L'asset du jeu portait ses glyphes incrustés** (`button-plus.png`, `button-moins.png`, 34 × 34
//! chacun) : exactement l'« asset par libellé » que le skill `ui-component` interdit. Un seul socle
//! générique en a été tiré ([`DsTexture::ButtonStepper`], voir sa doc), et les deux glyphes sont
//! ceux, déjà au manifeste, du reste de l'interface.
//!
//! ## Mesures
//!
//! Tout vient de `large-input-number.png` (192 × 34), la seule capture dont le socle (32 px, y=1..32,
//! deux boutons symétriques) coïncide avec l'asset isolé `button-moins.png`. La petite capture
//! (`input-number.png`, 104 × 28) sert de contrôle : sa gouttière concorde à un pixel près, mais son
//! glyphe fait 12 px dans un socle de 24 — le même que dans un socle de 32. Les deux ne sont donc pas
//! le même composant à deux échelles (le jeu règle son interface de 67 % à 233 %), et une seule fait
//! foi. Voir [`tokens::STEPPER_GUTTER_RATIO`].
//!
//! | Grandeur | Rapport | Vérification |
//! | --- | --- | --- |
//! | Socle | carré, côté = hauteur du pas | 32 sur la grande capture (y=1..32), boutons symétriques |
//! | Gouttière | [`tokens::STEPPER_GUTTER_RATIO`] | 1 + 32 + **10** + 106 + **10** + 32 + 1 = 192 |
//! | Encre du glyphe | [`tokens::STEPPER_ICON_RATIO`] | 12 × 12 mesurés dans un socle de 32 |
//! | Hauteur du champ | [`tokens::STEPPER_FIELD_HEIGHT_RATIO`] | **1,0 — écart assumé** : le jeu met 26 pour 32 |
//!
//! ## La saisie au clavier (2026-09-16)
//!
//! **Le champ est éditable**, comme dans le jeu — demande utilisateur. Il a été en lecture seule
//! jusque-là, faute d'avoir tranché ce qu'une saisie partielle devait produire ; c'est ce que
//! cette section fixe, et le composant s'y tient seul (aucun appelant n'a de validation à écrire).
//!
//! | Ce qui est tapé | Ce que devient la valeur | Ce que montre le champ |
//! | --- | --- | --- |
//! | Un entier du domaine | la valeur saisie, tout de suite | la saisie |
//! | Un entier au-dessus du maximum | le maximum | le maximum, réécrit sous les doigts |
//! | Un entier sous le minimum (`0` sur un domaine `1..=9999`) | **inchangée tant qu'on tape** | la saisie, puis le minimum en sortant du champ |
//! | Un champ vidé, ou un `-` seul | **inchangée** | le vide, puis la valeur en sortant du champ |
//! | Tout le reste (lettres, espaces, `+`, `,`…) | inchangée | rien : le caractère n'entre pas |
//!
//! Trois principes derrière ce tableau :
//!
//! - **Écrêter par le haut pendant la frappe, par le bas seulement à la sortie.** Un `1` de trop
//!   ne peut que dépasser : le corriger tout de suite ne détruit aucune intention. En bas, c'est
//!   l'inverse — sur un domaine `1..=9999`, vider le champ pour retaper passe forcément par le
//!   vide et par des préfixes plus petits que le minimum ; y planter un `1` à chaque frappe rendrait
//!   le champ inutilisable.
//! - **Un brouillon, pas la valeur.** Ce qui est affiché pendant l'édition vit dans la mémoire
//!   d'egui ([`draft_id`]), pas dans le `&mut i64` de l'appelant : celui-ci ne voit jamais un état
//!   intermédiaire qui ne soit pas un entier valide de son domaine. C'est ce qui permet au champ
//!   d'être vide une frappe sans que la quantité suivie tombe à zéro.
//! - **Le brouillon ne survit pas au focus.** Il naît au premier caractère tapé, est repris par les
//!   boutons et les flèches (qui le réécrivent à la valeur qu'ils viennent de poser), et disparaît
//!   dès que le champ rend le focus — Entrée, Tab, ou un clic ailleurs.
//!
//! Le brouillon est rangé sous le [`Stepper::log_name`] du pas : **deux pas simultanés portant le
//! même nom partageraient leur saisie**. C'est déjà la condition pour que leurs traces de journal
//! se distinguent, et la galerie de contrôle (`overlay-testkit/tests/design_gallery.rs`) les nomme
//! un par un.
//!
//! ## Ce qu'il ne fait pas
//!
//! Il ne décide pas de ce qu'une valeur hors bornes doit devenir : il **écrête** au domaine donné
//! par [`Stepper::range`], et c'est tout. Un appelant qui veut refuser la saisie plutôt que
//! l'écrêter lit la valeur après coup — le composant ne produit aucun effet de bord, comme tous les
//! autres (§6 du contrat).

use std::ops::RangeInclusive;

use egui::{Response, Ui, Vec2, Widget};

use crate::design::{
    components::icon_button::{icon_button, IconContext},
    components::input::input,
    icons::DsIcon,
    tokens,
};

/// Construit un pas numérique sur `value`.
pub fn stepper(value: &mut i64) -> Stepper<'_> {
    Stepper {
        value,
        range: i64::MIN..=i64::MAX,
        step: 1,
        size: tokens::STEPPER_SIZE,
        field_width: None,
        enabled: true,
        log_name: None,
    }
}

/// Voir [`stepper`].
pub struct Stepper<'a> {
    value: &'a mut i64,
    range: RangeInclusive<i64>,
    step: i64,
    size: f32,
    field_width: Option<f32>,
    enabled: bool,
    log_name: Option<String>,
}

impl<'a> Stepper<'a> {
    /// Domaine autorisé. La valeur y est écrêtée à chaque frame — y compris celle que l'appelant
    /// fournit, qui peut venir d'une configuration hors bornes.
    pub fn range(mut self, range: RangeInclusive<i64>) -> Self {
        self.range = range;
        self
    }

    /// Incrément d'un clic. 1 par défaut.
    pub fn step(mut self, step: i64) -> Self {
        self.step = step;
        self
    }

    /// Côté des deux boutons, **et donc hauteur du pas entier** : le socle est carré dans le jeu.
    /// Par défaut [`tokens::STEPPER_SIZE`], la taille native de sa texture. La gouttière et l'encre
    /// des glyphes suivent dans le même rapport.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Largeur du champ central. Sans elle, le champ prend toute la place restante — ce que fait le
    /// jeu quand le pas occupe une ligne de formulaire.
    pub fn field_width(mut self, width: f32) -> Self {
        self.field_width = Some(width);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Nom d'instance dans `overlay-ui.<date>.log` — préfixe les deux boutons.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Taille que le pas occupera, sans le dessiner. `None` en largeur si le champ n'est pas
    /// dimensionné : elle dépend alors de la place disponible, que seul `ui` connaît.
    pub fn desired_size(&self) -> (Option<f32>, f32) {
        let width = self
            .field_width
            .map(|field| field + 2.0 * (self.size + gutter(self.size)));
        (width, self.size)
    }
}

/// Gouttière entre un bouton et le champ, pour un socle de côté `size` — voir la doc de module.
fn gutter(size: f32) -> f32 {
    size * tokens::STEPPER_GUTTER_RATIO
}

/// Clé du **brouillon de saisie** dans la mémoire d'egui, pour un pas nommé `prefix` — voir la
/// section « La saisie au clavier » de la doc de module. `Id::new` et non `Ui::make_persistent_id` :
/// le brouillon doit survivre au changement de `Ui` parent (une zone défilable en crée un par
/// passe), alors qu'il n'a besoin de rien d'autre que du nom du pas pour être retrouvé.
fn draft_id(prefix: &str) -> egui::Id {
    egui::Id::new(("ds-stepper-draft", prefix))
}

/// Clé sous laquelle le pas mémorise l'identifiant de son champ, d'une frame à l'autre.
///
/// Pourquoi mémoriser un identifiant qu'on obtient en peignant le champ : les flèches ↑/↓ doivent
/// être consommées **avant** que le champ ne les lise (voir la fin de [`Stepper::ui`]), donc avant
/// de le peindre — et savoir s'il faut les consommer demande de savoir s'il a le focus. L'état de
/// la frame précédente est la seule réponse disponible à ce moment-là, et elle suffit : un focus
/// ne change qu'entre deux frames.
fn field_id_key(prefix: &str) -> egui::Id {
    egui::Id::new(("ds-stepper-field", prefix))
}

/// Ne garde du texte saisi que ce qui peut composer un entier du domaine : les chiffres, et un
/// signe `-` en tête quand le domaine descend sous zéro.
///
/// **La longueur est bornée** par celle de la plus longue borne. Sans elle, une touche maintenue
/// produirait `99999999999999999999`, qui ne tient pas dans un `i64` : le parsing échouerait, la
/// valeur cesserait de suivre la saisie, et le champ afficherait un nombre qui n'est plus le sien.
/// Refuser le caractère en trop est plus honnête que de l'accepter puis de l'ignorer.
fn sanitize(text: &str, range: &RangeInclusive<i64>) -> String {
    let signed = *range.start() < 0;
    let digits = |n: i64| n.unsigned_abs().to_string().len();
    let max_digits = digits(*range.start()).max(digits(*range.end()));
    let mut out = String::new();
    let mut count = 0;
    for c in text.chars() {
        if c == '-' && signed && out.is_empty() {
            out.push(c);
        } else if c.is_ascii_digit() && count < max_digits {
            out.push(c);
            count += 1;
        }
    }
    out
}

/// Ce qu'une saisie **en cours** fait de la valeur, et ce que le champ doit alors afficher — voir
/// le tableau de la doc de module. Rendue libre et sans `Ui` pour être testable : c'est la seule
/// règle du composant qui ne se lise pas sur une capture.
///
/// Renvoie le texte à conserver comme brouillon ; `value` n'est touchée que si la saisie est un
/// entier qui atteint au moins le minimum (l'écrêtage par le bas attend la sortie du champ).
fn apply_draft(text: &str, range: &RangeInclusive<i64>, value: &mut i64) -> String {
    let text = sanitize(text, range);
    match text.parse::<i64>() {
        Ok(saisi) if saisi > *range.end() => {
            *value = *range.end();
            value.to_string()
        }
        Ok(saisi) if saisi >= *range.start() => {
            *value = saisi;
            text
        }
        // Sous le minimum, vide, ou `-` seul : la valeur ne bouge pas, le champ garde la frappe.
        _ => text,
    }
}

/// Ce que devient la valeur quand le champ rend le focus — l'écrêtage par le bas, enfin appliqué.
/// Une saisie vide ou réduite à `-` ne dit rien : la valeur reste ce qu'elle était.
fn commit_draft(text: &str, range: &RangeInclusive<i64>, value: &mut i64) {
    if let Ok(saisi) = text.parse::<i64>() {
        *value = saisi.clamp(*range.start(), *range.end());
    }
}

impl Widget for Stepper<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Stepper {
            value,
            range,
            step,
            size,
            field_width,
            enabled,
            log_name,
        } = self;

        let gutter = gutter(size);
        let prefix = log_name.as_deref().unwrap_or("pas");
        // Écrêtage d'entrée : la valeur fournie peut venir d'une configuration hors bornes, et un
        // pas qui afficherait 9999 sur un domaine 1..=99 mentirait sur ce que la validation fera.
        *value = (*value).clamp(*range.start(), *range.end());

        let field =
            field_width.unwrap_or_else(|| (ui.available_width() - 2.0 * (size + gutter)).max(0.0));
        let total = Vec2::new(field + 2.0 * (size + gutter), size);
        // **`click` et non `hover` : c'est ce qui rend le pas focusable.** Les flèches ↑/↓ pilotent
        // la valeur depuis le champ quand il est en cours de saisie, mais le pas doit aussi
        // répondre au `Tab` sans passer par lui — le focus va alors au pas entier, et ses gouttières
        // le prennent au clic (les boutons et le champ ont leur propre zone par-dessus).
        let (rect, response) = ui.allocate_exact_size(total, egui::Sense::click());

        let square = Vec2::splat(size);
        let minus_rect = egui::Rect::from_min_size(rect.min, square);
        let plus_rect =
            egui::Rect::from_min_size(egui::pos2(rect.right() - size, rect.top()), square);

        // **Le brouillon de saisie** (2026-09-16) — voir la section « La saisie au clavier » de la
        // doc de module pour ce qu'il porte et pourquoi il ne touche pas à `value`.
        let draft_id = draft_id(prefix);
        let field_key = field_id_key(prefix);
        let mut draft: Option<String> = ui.data(|d| d.get_temp(draft_id));
        // Le focus du champ TEL QU'IL ÉTAIT en fin de frame précédente — voir `field_id_key`.
        let editing = enabled
            && ui
                .data(|d| d.get_temp::<egui::Id>(field_key))
                .is_some_and(|id| ui.memory(|m| m.has_focus(id)));
        if !editing {
            // Le champ a rendu le focus (Entrée, Tab, clic ailleurs), ou le pas vient d'être
            // désactivé : la saisie est validée — écrêtée, cette fois par les deux bouts — et
            // l'affichage repart de la valeur.
            if let Some(texte) = draft.take() {
                commit_draft(&texte, &range, value);
                ui.data_mut(|d| d.remove::<String>(draft_id));
                tracing::debug!(
                    component = "stepper",
                    name = prefix,
                    valeur = *value,
                    "saisie validée"
                );
            }
        }

        // **Les flèches ↑ et ↓**, demandées explicitement le 2026-09-12 (« flèche du haut, flèche du
        // bas qui pourront augmenter la valeur de cet input number »). Seulement celles-là : ← et →
        // restent au déplacement de curseur — d'autant plus depuis que le champ est éditable, où
        // les leur reprendre serait un piège.
        //
        // `consume_key` plutôt qu'une simple lecture : sans ça la même touche servirait aussi à
        // déplacer le focus entre widgets, et un appui ferait les deux. **Avant le champ**, pour
        // qu'il ne les lise pas non plus.
        if response.clicked() {
            response.request_focus();
        }
        if enabled && (editing || response.has_focus()) {
            let (haut, bas) = ui.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                )
            });
            if haut {
                *value = (*value + step).min(*range.end());
            }
            if bas {
                *value = (*value - step).max(*range.start());
            }
            if haut || bas {
                // Une flèche pendant une saisie REMPLACE le brouillon : sans ça, la valeur qu'elle
                // vient de poser serait écrasée par la frappe en cours au moment de valider.
                if draft.is_some() {
                    draft = Some(value.to_string());
                }
                tracing::debug!(
                    component = "stepper",
                    name = prefix,
                    valeur = *value,
                    "flèche clavier"
                );
            }
        }

        let can_decrease = enabled && *value > *range.start();
        let can_increase = enabled && *value < *range.end();

        if ui
            .put(
                minus_rect,
                icon_button(DsIcon::Minus)
                    .context(IconContext::Stepper)
                    .size(size)
                    .enabled(can_decrease)
                    .log_name(format!("{prefix}-moins")),
            )
            .clicked()
        {
            *value = (*value - step).max(*range.start());
            draft = None;
        }

        if ui
            .put(
                plus_rect,
                icon_button(DsIcon::Plus)
                    .context(IconContext::Stepper)
                    .size(size)
                    .enabled(can_increase)
                    .log_name(format!("{prefix}-plus")),
            )
            .clicked()
        {
            *value = (*value + step).min(*range.end());
            draft = None;
        }

        // **La BOÎTE du champ fait la hauteur des boutons**, son texte non : le corps reste celui
        // du gabarit standard (voir l'appel plus bas). Décision utilisateur, et écart assumé avec le
        // jeu, qui met 26 px de champ pour 32 px de socle : voir
        // `tokens::STEPPER_FIELD_HEIGHT_RATIO`, qui porte la mesure et la raison.
        let field_height = size * tokens::STEPPER_FIELD_HEIGHT_RATIO;
        let field_rect =
            egui::Rect::from_center_size(rect.center(), Vec2::new(field, field_height));

        // Le champ est **éditable** depuis le 2026-09-16 (demande utilisateur, doc de module) : il
        // affiche le brouillon de saisie tant qu'il l'a, la valeur sinon.
        let mut texte = draft.clone().unwrap_or_else(|| value.to_string());
        // **Un enfant NOMMÉ plutôt que `Ui::put`** : `put` pose son enfant sur un identifiant
        // automatique, qui dépend du nombre de widgets déjà ajoutés au `Ui` parent — donc du
        // défilement, qui en escamote (voir la leçon du 2026-09-16 en tête de
        // `components::input`). Le focus du champ, et avec lui le brouillon, sautaient d'un pas à
        // l'autre. La place, elle, est déjà prise par `allocate_exact_size` : cet enfant ne fait
        // que peindre dedans.
        let mut field_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(field_rect)
                .layout(egui::Layout::centered_and_justified(
                    egui::Direction::TopDown,
                ))
                .id_salt((prefix, "stepper-champ")),
        );
        let field_response = field_ui.add(
            input(&mut texte)
                // `box_height` et NON `size(Height(...))` : la boîte prend la hauteur des boutons,
                // le texte garde le corps du gabarit standard. Mettre le champ à l'échelle ferait
                // grossir sa valeur d'un tiers (corps 22 au lieu de 17) — le jeu, lui, écrit 12 px
                // d'encre dans un champ de 26, et rien ne justifie de s'en écarter parce qu'on a
                // étiré la boîte. Retour utilisateur du 2026-09-10.
                .box_height(field_height)
                .width(field)
                .enabled(enabled)
                .log_name(format!("{prefix}-valeur")),
        );
        ui.data_mut(|d| d.insert_temp(field_key, field_response.id));

        // **Le brouillon rangé en mémoire est celui de cette frame, toujours** : un clic sur `+`
        // l'efface (il a posé la valeur, la frappe en cours n'a plus cours), et un brouillon laissé
        // derrière serait re-validé à la frame suivante, effaçant justement ce que le bouton vient
        // de faire.
        let retenu = if enabled && field_response.has_focus() {
            let avant = *value;
            let retenu = apply_draft(&texte, &range, value);
            if *value != avant {
                tracing::debug!(
                    component = "stepper",
                    name = prefix,
                    valeur = *value,
                    "valeur saisie"
                );
            }
            Some(retenu)
        } else {
            draft
        };
        ui.data_mut(|d| match retenu {
            Some(texte) => {
                d.insert_temp(draft_id, texte);
            }
            None => d.remove::<String>(draft_id),
        });

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tolérance de comparaison — voir `window::tests`.
    const EPS: f32 = 0.01;

    /// **Le découpage complet de `large-input-number.png`**, la capture de référence :
    /// 1 + 32 + 10 + 106 + 10 + 32 + 1 = 192.
    #[test]
    fn la_gouttiere_reproduit_la_capture_de_reference() {
        assert!(
            (gutter(32.0) - 10.0).abs() < EPS,
            "gouttière de {} au lieu de 10 pour un socle de 32",
            gutter(32.0),
        );
    }

    /// La petite capture n'est pas la source (voir la doc du jeton), mais elle doit rester
    /// cohérente à un pixel près — sans quoi le rapport serait à revoir.
    #[test]
    fn la_petite_capture_reste_coherente_a_un_pixel() {
        assert!(
            (gutter(24.0) - 7.0).abs() <= 1.0,
            "gouttière de {} pour un socle de 24, la capture en montre 7",
            gutter(24.0),
        );
    }

    /// Aux cotes du jeu : un socle de 32 et un champ de 106 donnent 190 px de large.
    ///
    /// La capture, elle, fait 192 : elle inclut **un pixel de fond de chaque côté** (le composant
    /// y commence à x=1 et finit à x=190). Un composant ne peint pas la marge de son panneau.
    #[test]
    fn la_largeur_desiree_somme_les_deux_boutons_leurs_gouttieres_et_le_champ() {
        let mut valeur = 1;
        let (width, height) = stepper(&mut valeur)
            .size(32.0)
            .field_width(106.0)
            .desired_size();
        assert!((height - 32.0).abs() < EPS);
        let width = width.expect("champ dimensionné");
        assert!(
            (width - 190.0).abs() < 1.0,
            "{width} au lieu des 190 px du composant dans large-input-number.png",
        );
    }

    /// **Le champ fait la hauteur de ses boutons** — décision utilisateur, écart assumé avec le jeu
    /// qui met 26 px de champ pour 32 px de socle. Un pas dont le champ serait plus court se
    /// remarque immédiatement, et c'est ce qui a motivé le changement.
    #[test]
    fn le_champ_fait_la_hauteur_des_boutons() {
        for cote in [24.0_f32, 32.0, 40.0] {
            assert!(
                (cote * tokens::STEPPER_FIELD_HEIGHT_RATIO - cote).abs() < EPS,
                "socle {cote} : champ {}",
                cote * tokens::STEPPER_FIELD_HEIGHT_RATIO,
            );
        }
    }

    /// **Ce que le champ refuse d'accueillir** : tout ce qui ne peut pas composer un entier du
    /// domaine. Le `-` n'entre que si le domaine descend sous zéro, et jamais ailleurs qu'en tête.
    #[test]
    fn la_saisie_ne_retient_que_de_quoi_faire_un_entier_du_domaine() {
        let positif = 1..=9999;
        assert_eq!(sanitize("12a3", &positif), "123");
        assert_eq!(sanitize("-12", &positif), "12");
        assert_eq!(sanitize(" 4 2 ", &positif), "42");
        assert_eq!(sanitize("+7", &positif), "7");

        let signe = -50..=50;
        assert_eq!(sanitize("-12", &signe), "-12");
        assert_eq!(sanitize("1-2", &signe), "12");
    }

    /// **La longueur est bornée par celle de la plus longue borne** — sans quoi une touche
    /// maintenue déborderait `i64` et la valeur cesserait de suivre le champ (voir `sanitize`).
    #[test]
    fn la_saisie_ne_depasse_pas_le_nombre_de_chiffres_du_domaine() {
        assert_eq!(sanitize("123456", &(1..=9999)), "1234");
        assert_eq!(sanitize("99999999999999999999", &(1..=100)), "999");
        // Le signe ne compte pas comme un chiffre : -100 en autorise trois.
        assert_eq!(sanitize("-99999", &(-100..=100)), "-999");
    }

    /// Le tableau de la doc de module, ligne par ligne — **la seule règle du composant qui ne se
    /// lise pas sur une capture**, donc la seule qu'un test doive figer.
    #[test]
    fn une_saisie_en_cours_ecrete_par_le_haut_mais_jamais_par_le_bas() {
        let range = 1..=9999;

        // Un entier du domaine : la valeur suit tout de suite.
        let mut valeur = 1;
        assert_eq!(apply_draft("42", &range, &mut valeur), "42");
        assert_eq!(valeur, 42);

        // Au-dessus du maximum : écrêté sous les doigts, champ réécrit.
        let mut valeur = 42;
        assert_eq!(apply_draft("99999", &range, &mut valeur), "9999");
        assert_eq!(valeur, 9999);

        // Sous le minimum : la valeur ne bouge pas, la frappe reste affichée — c'est ce qui permet
        // de vider le champ pour retaper sur un domaine qui commence à 1.
        let mut valeur = 42;
        assert_eq!(apply_draft("0", &range, &mut valeur), "0");
        assert_eq!(valeur, 42);

        // Champ vidé : même règle.
        let mut valeur = 42;
        assert_eq!(apply_draft("", &range, &mut valeur), "");
        assert_eq!(valeur, 42);
    }

    /// **En sortant du champ, l'écrêtage par le bas s'applique enfin**, et une saisie qui ne dit
    /// rien laisse la valeur intacte.
    #[test]
    fn la_sortie_du_champ_ecrete_par_les_deux_bouts() {
        let range = 1..=9999;

        let mut valeur = 42;
        commit_draft("0", &range, &mut valeur);
        assert_eq!(valeur, 1);

        let mut valeur = 42;
        commit_draft("", &range, &mut valeur);
        assert_eq!(valeur, 42, "un champ vidé puis quitté retrouve sa valeur");

        let mut valeur = 42;
        commit_draft("-", &(-50..=50), &mut valeur);
        assert_eq!(valeur, 42, "un signe seul n'est pas un nombre");

        let mut valeur = 42;
        commit_draft("7", &range, &mut valeur);
        assert_eq!(valeur, 7);
    }

    /// Sans largeur de champ, la largeur totale n'est pas connue avant le rendu — c'est ce que dit
    /// le `None`, et ce qui permet au pas d'occuper une ligne entière.
    #[test]
    fn sans_champ_dimensionne_la_largeur_n_est_pas_connue() {
        let mut valeur = 1;
        assert_eq!(stepper(&mut valeur).desired_size().0, None);
    }
}
