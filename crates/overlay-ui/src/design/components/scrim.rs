//! **Voile modal** du design system — la teinte sombre qui couvre ce qu'une fenêtre interrompt, et
//! qui dit que le reste est inerte le temps qu'elle est ouverte.
//!
//! ```ignore
//! use overlay_ui::design::{self, ScrimLayer};
//!
//! // La boîte est posée dans un rectangle de `taille`, centré sur `fenetre` ; tout le reste de
//! // `fenetre` est voilé et n'accepte plus aucun clic.
//! let reponse = design::scrim(fenetre)
//!     .centered(taille)
//!     .log_name("suivi.recette")
//!     .show(ui, |ui| peindre_la_boite(ui));
//! ```
//!
//! ## D'où il vient
//!
//! Le 2026-09-17, ce voile existait en **quatre copies** : dans `design::confirm_dialog` (son
//! origine, 2026-09-12), dans `panels::recipe_dialog`, dans `panels::personnages_tab`
//! (`couche_modale`) et dans une maquette d'`overlay-testkit`. Les quatre faisaient la même chose
//! en douze lignes — une couche, un clip à `Rect::EVERYTHING`, un `rect_filled` noir à
//! [`tokens::SCRIM_ALPHA`], un `interact` qui avale les clics, un rectangle centré — et deux
//! d'entre elles avaient déjà divergé sur l'ordre de couche. Le jour où la fenêtre Options a eu
//! besoin du même voile sur la fenêtre de jeu entière (décision utilisateur, même jour : « un
//! voile qui recouvre toute la fenêtre du jeu et les overlays »), une cinquième copie n'était pas
//! une option : le comportement est remonté ici, et les trois appelants de production l'utilisent
//! (la maquette, figée, garde la sienne).
//!
//! ## Le voile n'est pas une teinte, c'est une information
//!
//! Tant que ce qu'il porte est ouvert, ce qu'il couvre est **inerte**. Il **avale** donc les
//! clics qui tombent à côté du contenu : sans ça il ne serait qu'une couleur, et ce qu'il couvre
//! resterait réellement cliquable — l'inverse de ce qu'il annonce. Ce qui est peint DANS le
//! contenu, lui, reste cliquable : egui donne la priorité au dernier widget posé sur une zone, et
//! le contenu est posé après le voile.
//!
//! C'est pour la même raison qu'il faut lui donner la **fenêtre entière**, pas le panneau d'où
//! l'on appelle : un voile rogné laisserait bannière, onglets et pied de page à pleine
//! luminosité, ce qui se lit comme « ils restent cliquables ».
//!
//! ## Trois couches, parce que trois cas
//!
//! Un voile s'interpose entre ce qui est déjà peint et ce qu'il porte. Il lui faut donc une
//! couche au-dessus de l'appelant — sauf quand il **est** le fond de la fenêtre. [`ScrimLayer`]
//! nomme les trois cas ; le mauvais choix ne se voit pas sur le voile lui-même mais sur ce qui
//! flotte au-dessus : un popup de sélecteur qui disparaît, ou une modale qui passe derrière un
//! panneau de suggestions.

use egui::{Color32, InnerResponse, Rect, Sense, Ui, Vec2};

use crate::design::tokens;

/// Dans quelle couche egui le voile — et le contenu qu'il porte — sont peints.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScrimLayer {
    /// **`Order::Foreground`**, le défaut : par-dessus tout ce que la fenêtre a peint, popups de
    /// sélecteur compris. C'est ce qu'il faut pour une question qui interrompt tout — la boîte
    /// de confirmation, la fenêtre de recette.
    #[default]
    Foreground,
    /// **`Order::Middle`** : au-dessus du contenu de base de la fenêtre, mais SOUS les popups
    /// (`design::autocomplete`, `design::select`, qui vivent en `Foreground`). Pour une modale
    /// qui porte elle-même un champ à suggestions : en `Foreground`, deux couches de même ordre
    /// se départagent par leur identifiant, et le panneau de suggestions passerait derrière la
    /// modale (`panels::personnages_tab`, 2026-09-16).
    Middle,
    /// **La couche de l'appelant**, sans en ouvrir de nouvelle : le voile est le FOND de la
    /// fenêtre, rien n'est peint dessous. C'est la fenêtre Options étendue à la fenêtre de jeu
    /// (2026-09-17) : le voile va bord à bord et la modale se centre dedans, dans la couche de
    /// base — ses sélecteurs, en `Foreground`, restent au-dessus d'elle.
    Current,
}

/// Construit un voile couvrant `over` — la fenêtre entière, voir la doc de module.
pub fn scrim(over: Rect) -> Scrim {
    Scrim {
        over,
        layer: ScrimLayer::default(),
        centered: None,
        log_name: None,
    }
}

/// Voir [`scrim`].
pub struct Scrim {
    over: Rect,
    layer: ScrimLayer,
    centered: Option<Vec2>,
    log_name: Option<String>,
}

impl Scrim {
    /// La couche dans laquelle voile et contenu sont peints — voir [`ScrimLayer`].
    pub fn layer(mut self, layer: ScrimLayer) -> Self {
        self.layer = layer;
        self
    }

    /// Le contenu est posé dans un rectangle de `size`, **centré** sur ce que le voile couvre.
    /// Sans cet appel, il reçoit le rectangle voilé entier et se place lui-même.
    pub fn centered(mut self, size: Vec2) -> Self {
        self.centered = Some(size);
        self
    }

    /// Nom d'instance — dans le journal, et dans les identifiants egui de la couche : deux voiles
    /// ouverts dans la même fenêtre (une modale et sa confirmation) doivent rester distincts.
    pub fn log_name(mut self, name: impl Into<String>) -> Self {
        self.log_name = Some(name.into());
        self
    }

    /// Le rectangle où le contenu sera posé, tel que `show` le calcule.
    pub fn content_rect(&self) -> Rect {
        match self.centered {
            Some(size) => Rect::from_center_size(self.over.center(), size),
            None => self.over,
        }
    }

    /// Peint le voile, avale les clics, puis appelle `add_contents` dans le rectangle du contenu.
    ///
    /// La `Response` rendue est celle du voile : `clicked()` y signifie « l'utilisateur a cliqué
    /// À CÔTÉ du contenu » — ce que le composant journalise et laisse l'appelant interpréter (ou
    /// ignorer : un vrai dialogue modal ne se ferme pas sur un clic à côté).
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let nom = self.log_name.clone().unwrap_or_else(|| "scrim".to_string());
        let over = self.over;
        let content = self.content_rect();

        // **`id_salt` autant que `layer_id`** (leçon de `panels::recipe_dialog`) : sans le sel,
        // le contenu est un `Ui` enfant de plus dans la même chaîne que le panneau qu'il
        // recouvre, ses widgets héritent des mêmes identifiants, et egui voit « le même widget a
        // changé de couche en cours de frame » — une assertion de debug qui panique.
        let mut builder = egui::UiBuilder::new()
            .max_rect(over)
            .id_salt(("ds-scrim", nom.clone()));
        let order = match self.layer {
            ScrimLayer::Foreground => Some(egui::Order::Foreground),
            ScrimLayer::Middle => Some(egui::Order::Middle),
            ScrimLayer::Current => None,
        };
        if let Some(order) = order {
            builder = builder.layer_id(egui::LayerId::new(
                order,
                egui::Id::new(("ds-scrim", nom.clone())),
            ));
        }
        let mut couche = ui.new_child(builder);
        // Le `Ui` d'où l'on vient peut être écrêté plus étroit que `over` (un panneau, une zone
        // défilable) : le voile, lui, doit aller jusqu'aux bords de ce qu'on lui a donné.
        couche.set_clip_rect(Rect::EVERYTHING);

        couche
            .painter()
            .rect_filled(over, 0, Color32::from_black_alpha(tokens::SCRIM_ALPHA));
        // Posé AVANT le contenu : c'est ce qui donne au contenu la priorité sur les clics qui
        // tombent dedans, et au voile ceux qui tombent à côté.
        let response = couche.interact(
            over,
            egui::Id::new(("ds-scrim-voile", nom.clone())),
            Sense::click(),
        );
        if response.clicked() {
            tracing::debug!(component = "scrim", name = %nom, "clic à côté du contenu, avalé");
        }

        let mut contenu = couche.new_child(egui::UiBuilder::new().max_rect(content));
        // Même règle que la couche : un ornement qui déborde du rectangle de contenu (la crête de
        // la boîte de confirmation) n'a pas à être rogné par lui.
        contenu.set_clip_rect(Rect::EVERYTHING);
        let inner = add_contents(&mut contenu);
        InnerResponse { inner, response }
    }
}
