//! Style `egui` partagé entre les points de création d'`egui::Context` de cette UI — `main.rs`
//! (Windows), `bin/overlay-ui-x11.rs` (Linux, voir doc de `lib.rs`) et le harnais de rendu offscreen
//! (`overlay-testkit`, §17.1 du plan). Extrait ici le 2026-09-06 (refonte design system tooltip) :
//! auparavant dupliqué entre les deux binaires, et DÉJÀ divergent avant ce refactor (le binaire
//! Linux n'appliquait ni le curseur "main" ni le délai de tooltip complet — juste
//! `show_tooltips_only_when_still`/`tooltip_delay`) — exactement le risque qu'une fonction PARTAGÉE
//! élimine plutôt qu'un réglage recopié à chaque nouveau point d'entrée.

use crate::panels::tooltip;

/// Applique le style global de l'overlay à `ctx` : délai/persistance des tooltips, curseur "main"
/// au survol de tout élément cliquable, et le design system tooltip (fond/bordure/ombre, voir
/// `panels::tooltip::TOOLTIP_BG_FILL` et sa doc). Appliqué aux DEUX thèmes (`style_mut_of`,
/// egui 0.36) : seul le thème sombre est par ailleurs reproduit dans cette UI (voir
/// `panels::combat::ACCENT`), mais aucun de ces réglages n'a de contrepartie visuelle propre au
/// thème clair — autant ne pas dépendre de celui qu'egui choisit par défaut.
pub fn apply(ctx: &egui::Context) {
    // Polices du design system, AVANT tout le reste : `Context::set_fonts` reconstruit l'atlas de
    // glyphes, et `epaint` panique sur une famille nommée inconnue — un composant peint avec
    // `design::text::label_font` sur un contexte qui n'est pas passé par ici ne se rate pas
    // silencieusement, il s'arrête. Voir `design::fonts` pour le choix de la police.
    crate::design::fonts::install(ctx);

    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        ctx.style_mut_of(theme, |style| {
            // Délai de tooltip par défaut d'egui (0,5s, `show_tooltips_only_when_still=true` : le
            // minuteur repart de zéro à chaque micro-mouvement de la souris, pas seulement au
            // premier survol) — trop long/imprévisible dans cette architecture SANS boucle de
            // rendu continue (§6.1) : chaque redessin dépend du réveil `about_to_wait`/
            // `next_redraw_at`, qui n'apporte qu'une granularité de 50ms au mieux, jamais un vrai
            // 60Hz qui masquerait la latence. Retour utilisateur 2026-09-02 : « il faut bien
            // quasiment quatre, cinq secondes avant que la tooltip s'affiche » —
            // `show_tooltips_only_when_still` désactivé (affichée dès le survol, sans exiger une
            // souris parfaitement immobile) et le délai réduit à 150ms (perceptible comme quasi
            // immédiat, tout en évitant un flash sur un simple passage de souris).
            style.interaction.show_tooltips_only_when_still = false;
            style.interaction.tooltip_delay = 0.15;

            // Curseur "main" au survol de tout élément cliquable (retour utilisateur 2026-09-04 :
            // rien ne l'indiquait visuellement) — couvre automatiquement les widgets `Button`/
            // `small_button` (voir `render_content.rs`) ; un élément dessiné à la main via
            // `Ui::interact` brut (voir `panels::combat`/`panels::watchlist`) ne consulte PAS ce
            // réglage tout seul, d'où un `.on_hover_cursor(...)` explicite à chacun de ces
            // endroits en complément.
            style.visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);

            // Design system tooltip (retour utilisateur 2026-09-06, voir
            // `panels::tooltip::TOOLTIP_BG_FILL` et sa doc pour la mesure sur les captures de
            // référence) : fond translucide (~82 % d'opacité, PAS opaque comme un premier passage
            // l'avait posé) sans bordure ni ombre portée, à la place du thème PAR DÉFAUT d'egui
            // (gris `#1b1b1b` flouté sur 8px) jugé « pas très joli [...] ça rend flou ».
            // `window_fill`/`window_stroke`/`popup_shadow`/`menu_corner_radius`/`menu_margin` ne
            // servent QUE de socle aux tooltips dans cette UI (aucun `egui::Window` ni menu/combobox
            // ailleurs, voir la doc de `TOOLTIP_BG_FILL`) : ce réglage global couvre donc TOUS les
            // tooltips de l'overlay en un seul endroit, plutôt qu'un style dupliqué à chaque appel.
            style.visuals.window_fill = tooltip::TOOLTIP_BG_FILL;
            style.visuals.window_stroke = egui::Stroke::NONE;
            style.visuals.popup_shadow = egui::Shadow::NONE;
            style.visuals.menu_corner_radius = egui::CornerRadius::same(6);
            // Marge interne (retour utilisateur 2026-09-06, second passage : « plus de marge
            // latérale et surtout plus de marge top et bottom » que le 6px uniforme d'egui) — voir
            // la doc de `TOOLTIP_MARGIN` pour la mesure.
            style.spacing.menu_margin = tooltip::TOOLTIP_MARGIN;
        });
    }
}
