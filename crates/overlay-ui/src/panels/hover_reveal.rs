//! **Les pastilles révélées au survol restent un instant après le départ du pointeur**
//! (2026-09-28, demande utilisateur).
//!
//! Les rangées de boutons des overlays (cadenas et replacement du Récap, pastille et flèche de
//! repli du Combat, poignée du Suivi) ne sont peintes que pointeur posé sur le panneau. Depuis
//! que la fenêtre ne capte le clic que sur ce qu'elle a peint (`crate::hit_region`), l'air entre
//! le panneau et sa pastille — ou les vides transparents du panneau lui-même — rend la fenêtre
//! traversante : egui perd le pointeur (`PointerGone`), la pastille disparaît, et elle n'est plus
//! là quand le curseur arrive dessus. Demande, presque mot pour mot : « laisser afficher pendant
//! 500 millisecondes les petits groupes de boutons qui s'affichent lorsqu'on survole les
//! overlays, pour que l'utilisateur puisse interagir dessus ».
//!
//! [`reveal`] garde donc la pastille [`LINGER`] après le dernier survol. Une fois le curseur
//! arrivé sur elle, elle est peinte, donc cliquable : la fenêtre recapte le pointeur et le survol
//! reprend le relais. La durée est un point de départ, à ajuster à l'usage.

use std::time::Duration;

/// Combien de temps une pastille reste affichée après le départ du pointeur.
pub const LINGER: Duration = Duration::from_millis(500);

/// La pastille `id` doit-elle être peinte cette frame ? `hovered` est le survol mesuré par le
/// panneau. Vrai pendant le survol, puis encore [`LINGER`] après — et un redessin est alors
/// demandé pour l'instant où elle doit disparaître, cette architecture ne redessinant pas en
/// continu.
pub fn reveal(ctx: &egui::Context, id: egui::Id, hovered: bool) -> bool {
    let now = ctx.input(|i| i.time);
    if hovered {
        ctx.data_mut(|data| data.insert_temp(id, now));
        return true;
    }
    let Some(last) = ctx.data(|data| data.get_temp::<f64>(id)) else {
        return false;
    };
    let remaining = LINGER.as_secs_f64() - (now - last);
    if remaining > 0.0 {
        ctx.request_repaint_after(Duration::from_secs_f64(remaining));
        true
    } else {
        ctx.data_mut(|data| data.remove::<f64>(id));
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, time: f64, hovered: bool) -> bool {
        let id = egui::Id::new("pastille");
        let mut shown = false;
        let input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| shown = reveal(ui.ctx(), id, hovered));
        // Sans quoi epaint panique à la destruction : l'atlas de polices n'a pas été livré.
        output.textures_delta.clear();
        shown
    }

    #[test]
    fn la_pastille_reste_le_temps_du_maintien_puis_disparait() {
        let ctx = egui::Context::default();
        assert!(!frame(&ctx, 0.0, false), "jamais survolée");
        assert!(frame(&ctx, 1.0, true));
        assert!(frame(&ctx, 1.4, false), "encore dans le maintien");
        assert!(!frame(&ctx, 1.6, false), "maintien écoulé");
        assert!(!frame(&ctx, 1.61, false));
    }

    #[test]
    fn un_nouveau_survol_relance_le_maintien() {
        let ctx = egui::Context::default();
        assert!(frame(&ctx, 1.0, true));
        assert!(frame(&ctx, 1.4, true));
        assert!(frame(&ctx, 1.8, false));
        assert!(!frame(&ctx, 2.0, false));
    }
}
