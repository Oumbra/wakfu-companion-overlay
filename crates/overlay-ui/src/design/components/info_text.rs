//! **Texte d'information** du design system Wakfu — une pastille et un texte, la façon dont le jeu
//! commente un réglage sans en faire un contrôle.
//!
//! ```ignore
//! use overlay_ui::design::{self, InfoTone};
//!
//! ui.add(design::info_text("Le thème sera appliqué au prochain démarrage."));
//!
//! ui.add(
//!     design::info_text("Le fichier sélectionné doit s'appeler wakfu.log.")
//!         .tone(InfoTone::Alert),
//! );
//! ```
//!
//! ## Ce qui a été mesuré
//!
//! Source unique : [`releve-options-interface.json`](../../../../../docs/design-system/releve-options-interface.json),
//! nœud `info` — le bloc d'information en bas de l'onglet Interface, **le seul de la fenêtre
//! Options du jeu**.
//!
//! | Grandeur | Valeur | Détail |
//! | --- | --- | --- |
//! | Pastille | **12 × 12 px** | boîte `[38, 438, 50, 450]` |
//! | Position de la pastille | centrée sur la **première ligne** | ligne 1 en `[57, 436, 628, 454]`, axe 445 ; pastille d'axe 444 |
//! | Écart pastille → texte | **7 px** | la pastille finit à x=50, le texte commence à x=57 |
//! | Lignes suivantes | alignées sur le **texte** | la seconde ligne est à x=57 elle aussi, pas sous la pastille |
//! | Interligne | **23 px** | haut d'encre à haut d'encre, y=436 puis y=459 |
//! | Corps | **17 px** (encre 13) | le même corps qu'un libellé de bouton |
//! | Graisse | **regular** (`design::fonts::LABEL`) | pas la Medium du pied de page |
//! | Couleur | **blanc pur `#ffffff`** | voir ci-dessous |
//!
//! **Le texte d'information est blanc pur, pas gris.** Note du relevé, à ne pas contourner :
//! « l'impression de gris vient du fond et de l'absence de graisse, pas de la couleur. Un portage
//! qui le grise s'écarte de la référence. »
//!
//! **La pastille n'est pas un rond plein** : c'est `icons/icon-info.png`, le disque doré cerclé
//! détouré de la capture du jeu, dont le « i » est creusé en alpha (recette
//! `.claude/skills/design-asset/references/recettes-icones.md`). Elle est blanche dans le fichier et
//! prend sa couleur par teinte, ce qui est précisément ce qui rend le ton `Alert` possible sans un
//! second fichier.
//!
//! ## Ce qui n'a PAS de référence, et est donc inventé
//!
//! - **Le ton `Alert`.** Le jeu n'a aucune variante d'alerte relevée : `Alert` est une **extension
//!   assumée**, au même titre que les états `Hovered`/`Disabled` de `design::input`. Seule sa
//!   *composition* est inventée — la couleur, elle, est le rouge relevé du design system
//!   (`tokens::INFO_ALERT`, le haut du bouton « Annuler »), pas un rouge choisi à l'œil.
//! - **Rien d'autre.** Ce composant n'est pas interactif : il n'a ni survol, ni état désactivé, ni
//!   clic. Les trois états du contrat de composant ne s'appliquent pas à un texte — inventer un
//!   survol ici serait inventer du design. Il garde `Sense::hover()` pour pouvoir porter une
//!   infobulle, rien de plus.

use egui::{Response, Sense, Ui, Vec2, Widget};

use crate::design::{assets::DsTexture, text, tokens, DesignSystem};

/// Registre du message. Le jeu n'a relevé que le premier — voir la doc de module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InfoTone {
    /// Pastille dorée, texte blanc : le ton du jeu, mesuré.
    #[default]
    Info,
    /// Même composition, teintée en rouge. **Alerte, pas erreur** — décision utilisateur du
    /// 2026-09-10 : ce ton signale « ce que vous venez de faire ne mène nulle part », pas « le
    /// programme est cassé ».
    Alert,
}

impl InfoTone {
    /// Couleur de la pastille ET du texte. Les deux la partagent : la pastille est le seul signal
    /// de registre que le jeu utilise, teinter l'une sans l'autre les ferait diverger.
    fn color(self) -> egui::Color32 {
        match self {
            InfoTone::Info => tokens::INFO_TEXT,
            InfoTone::Alert => tokens::INFO_ALERT,
        }
    }

    /// Couleur de la pastille. Distincte de celle du texte pour le seul ton mesuré : dans le jeu, le
    /// texte est blanc et la pastille dorée.
    fn dot_color(self) -> egui::Color32 {
        match self {
            InfoTone::Info => tokens::INFO_DOT,
            InfoTone::Alert => tokens::INFO_ALERT,
        }
    }
}

/// Construit un texte d'information. Point d'entrée unique — voir la doc de module.
pub fn info_text(text: impl Into<String>) -> InfoText {
    InfoText::new(text)
}

pub struct InfoText {
    text: String,
    tone: InfoTone,
    width: Option<f32>,
    tooltip: Option<String>,
    log_name: Option<String>,
}

impl InfoText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tone: InfoTone::default(),
            width: None,
            tooltip: None,
            log_name: None,
        }
    }

    pub fn tone(mut self, tone: InfoTone) -> Self {
        self.tone = tone;
        self
    }

    /// Largeur imposée. **Sans elle, le bloc prend toute la largeur disponible** — comme
    /// `design::input`, et pour la même raison : c'est la mise en page qui sait jusqu'où le texte a
    /// le droit d'aller, le composant ne le devine pas.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Nom d'instance pour la journalisation (défaut : les premiers mots du texte).
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }
}

/// Retrait du texte depuis le bord gauche du bloc : la pastille, puis la gouttière.
fn text_indent() -> f32 {
    tokens::INFO_DOT_SIZE + tokens::INFO_DOT_GAP
}

impl Widget for InfoText {
    fn ui(self, ui: &mut Ui) -> Response {
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let font = text::label_font(ui.ctx(), tokens::INFO_FONT_SIZE);
        let color = self.tone.color();

        // Le retour à la ligne se fait sur la largeur RESTANTE, pas sur la largeur du bloc : les
        // lignes suivantes s'alignent sur le texte (x=57 dans le relevé), jamais sous la pastille.
        // Plancher à zéro pour la même raison que partout ailleurs — un bloc plus étroit que sa
        // propre pastille produirait une largeur négative, qu'egui retournerait silencieusement.
        let wrap_width = (width - text_indent()).max(0.0);

        let mut job = egui::text::LayoutJob::single_section(
            self.text.clone(),
            egui::TextFormat {
                font_id: font,
                color,
                // L'interligne est une MESURE (23 px), pas la hauteur naturelle de la police : sans
                // ça, egui empile les lignes à ~20 px et le bloc se resserre par rapport au jeu.
                line_height: Some(tokens::INFO_LINE_HEIGHT),
                ..Default::default()
            },
        );
        job.wrap.max_width = wrap_width;
        let galley = ui.fonts_mut(|f| f.layout_job(job));

        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, galley.size().y), Sense::hover());

        if ui.is_rect_visible(rect) {
            // Pastille centrée sur la PREMIÈRE ligne, pas sur le bloc — c'est la cote la plus
            // facile à rater : sur un message de deux lignes, un centrage sur le bloc la
            // descendrait de 11 px.
            let dot_center_y = rect.top() + tokens::INFO_LINE_HEIGHT / 2.0;
            let dot_rect = egui::Rect::from_center_size(
                egui::pos2(
                    rect.left() + tokens::INFO_DOT_SIZE / 2.0,
                    dot_center_y.round(),
                ),
                Vec2::splat(tokens::INFO_DOT_SIZE),
            );
            // La texture fait 27 × 28 (le détourage a pris un pixel de plus en hauteur) et est
            // peinte dans un carré de 12 : l'écrasement d'un vingt-huitième est sous le pixel. La
            // cote du relevé est 12 × 12, c'est elle qui fait foi.
            DesignSystem::get(ui.ctx()).paint(
                ui.painter(),
                dot_rect,
                DsTexture::IconInfo,
                self.tone.dot_color(),
            );
            ui.painter()
                .with_clip_rect(rect.intersect(ui.clip_rect()))
                .galley(
                    egui::pos2(rect.left() + text_indent(), rect.top()),
                    galley,
                    color,
                );
        }

        let name = self.log_name.clone().unwrap_or_else(|| {
            self.text
                .split_whitespace()
                .take(4)
                .collect::<Vec<_>>()
                .join(" ")
        });

        // Bloc trop étroit pour porter ne serait-ce que sa pastille et sa gouttière : le texte n'a
        // alors plus aucune place et le composant ne peint qu'un rond. C'est un défaut de mise en
        // page, signalé UNE fois par instance comme le libellé écrêté d'un bouton — pas un flux à
        // 60 Hz.
        if wrap_width <= 0.0 {
            let warned_id = response.id.with("ds-info-text-overflow");
            let already = ui.data_mut(|d| {
                let seen = d.get_temp::<bool>(warned_id).unwrap_or(false);
                d.insert_temp(warned_id, true);
                seen
            });
            if !already {
                tracing::warn!(
                    component = "info_text",
                    name,
                    largeur = width,
                    requise = text_indent(),
                    "bloc d'information plus étroit que sa pastille : le texte n'a aucune place"
                );
            }
        }

        match self.tooltip {
            Some(tooltip) => response.on_hover_text(tooltip),
            None => response,
        }
    }
}
