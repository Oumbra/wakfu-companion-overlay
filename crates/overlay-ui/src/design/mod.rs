//! **Design system de l'overlay** — la couche qui transforme les captures préparées de
//! `assets/design-system/` en composants egui paramétrables, réutilisables et testés.
//!
//! Motivation (retour utilisateur 2026-09-09) : jusqu'ici chaque panneau redessinait ses propres
//! boutons à la main (`panels::options_modal` : sept constantes de couleur et un `chamfer` par
//! bouton), et chaque nouvelle taille de bouton demandait un **nouvel asset** découpé à la main,
//! libellé compris (`assets/design-system/large-button-cancel.png`, `large-button-validate.png`).
//! Résultat : refaire à chaque fois un travail déjà fait, et aucun endroit où corriger un défaut
//! une seule fois pour toute l'application.
//!
//! Répartition des rôles, à respecter :
//!
//! | Couche | Rôle | Ne fait PAS |
//! | --- | --- | --- |
//! | `design::assets` | manifeste : nom logique → fichier + découpage 9-slice | de la peinture |
//! | `design::nine_slice` | peindre une texture à n'importe quelle taille | connaître un composant |
//! | `design::text` | peindre un libellé (graisse synthétique) | choisir quoi écrire |
//! | `design::tokens` | couleurs et métriques mesurées, partagées | de la mise en page |
//! | `design::components` | un composant = une API paramétrable + son comportement | de la logique métier |
//! | `panels::*` | mise en page, état applicatif | dessiner un bouton à la main |
//!
//! Un panneau qui a besoin d'un bouton écrit `ui.add(design::button("Valider"))`. S'il lui faut
//! quelque chose qu'aucun composant ne sait faire, la réponse est **d'étendre le composant**, pas
//! de repeindre à côté — c'est exactement ce que le skill `.claude/skills/ui-component/` outille.
//!
//! Chargement des textures : paresseux et mémorisé par `egui::Context` (`DesignSystem::get`). Aucun
//! câblage à faire dans `render_content`/`main.rs`, et rien n'est téléversé sur le GPU tant qu'aucun
//! composant n'est utilisé — le budget mémoire (300 Mo, §8 du plan) ne paie que ce qui est affiché.

pub mod assets;
pub mod components;
pub mod nine_slice;
pub mod text;
pub mod tokens;

use std::sync::Arc;

pub use assets::DsTexture;
pub use components::button::{button, Button, ButtonSize, ButtonState, ButtonVariant};

/// Identifiant de mémorisation dans `egui::Context` — voir `DesignSystem::get`.
const MEMO_ID: &str = "wakfu-overlay-design-system";

struct Inner {
    /// Indexé par `DsTexture::index()` — un `Vec` plutôt qu'une `HashMap` : le manifeste est une
    /// table statique de taille connue, une table de hachage n'apporterait qu'un coût de hachage
    /// à chaque frame.
    textures: Vec<egui::TextureHandle>,
}

/// Poignée sur les textures du design system. `Clone` bon marché (un `Arc`) : les composants la
/// récupèrent par `DesignSystem::get(ui.ctx())` à chaque frame sans surcoût mesurable.
#[derive(Clone)]
pub struct DesignSystem {
    inner: Arc<Inner>,
}

impl DesignSystem {
    /// Récupère l'instance associée à `ctx`, en la chargeant au premier appel.
    ///
    /// La mémorisation passe par `Context::data` (données temporaires, jamais sérialisées). Le test
    /// « présent ? » et le chargement sont **séparés volontairement** : `Context::data_mut` prend le
    /// verrou d'écriture du contexte, et `Context::load_texture` le prend aussi — les imbriquer
    /// interbloquerait. Charger hors du verrou puis insérer coûte, dans le pire cas (deux appels
    /// concurrents au tout premier frame), un double chargement inoffensif.
    pub fn get(ctx: &egui::Context) -> Self {
        let id = egui::Id::new(MEMO_ID);
        if let Some(existing) = ctx.data(|d| d.get_temp::<DesignSystem>(id)) {
            return existing;
        }
        let loaded = Self::load(ctx);
        ctx.data_mut(|d| d.insert_temp(id, loaded.clone()));
        loaded
    }

    /// Téléverse toutes les textures du manifeste. Appelé par `get` ; public pour un harnais qui
    /// voudrait maîtriser le moment du chargement.
    pub fn load(ctx: &egui::Context) -> Self {
        let textures = DsTexture::ALL
            .iter()
            .map(|texture| {
                let spec = texture.spec();
                let decoded = image::load_from_memory(spec.bytes)
                    .expect("texture du design system invalide — asset corrompu au build")
                    .to_rgba8();
                let (width, height) = decoded.dimensions();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width as usize, height as usize],
                    decoded.as_raw(),
                );
                ctx.load_texture(spec.name, image, egui::TextureOptions::LINEAR)
            })
            .collect();
        tracing::debug!(
            textures = DsTexture::ALL.len(),
            "design system chargé (textures téléversées)"
        );
        Self {
            inner: Arc::new(Inner { textures }),
        }
    }

    pub fn texture(&self, texture: DsTexture) -> &egui::TextureHandle {
        &self.inner.textures[texture.index()]
    }

    /// Taille native de la texture, en pixels — la taille à laquelle elle a été capturée dans le
    /// jeu, donc la référence des gabarits (voir `components::button::ButtonSize`).
    pub fn native_size(&self, texture: DsTexture) -> egui::Vec2 {
        self.texture(texture).size_vec2()
    }

    /// Peint une texture du manifeste dans `rect`, en 9-slice, avec le découpage déclaré pour elle.
    /// C'est le seul chemin que doivent emprunter les composants — jamais `egui::Image::paint_at`,
    /// qui déformerait les coins.
    pub fn paint(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        texture: DsTexture,
        tint: egui::Color32,
    ) {
        nine_slice::paint(
            painter,
            rect,
            self.texture(texture),
            &texture.spec().slice,
            tint,
        );
    }
}
