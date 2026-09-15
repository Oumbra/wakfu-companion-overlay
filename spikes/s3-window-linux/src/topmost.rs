//! Décision « topmost/repli » — portage de `overlay_ui::main::App::sync_topmost` (Windows) vers une
//! fonction PURE, horloge injectable dès l'écriture (§17.6 du plan : réserve de l'expert X11 sur le
//! délai de grâce `TOPMOST_DEMOTE_GRACE`, ici traitée dès le départ plutôt qu'en correctif après
//! coup comme ce fut le cas côté Windows). Testable en microsecondes, sans Xvfb ni serveur X réel —
//! voir les tests en bas de fichier, seule couverture de ce fichier qui tourne réellement en CI
//! (le reste de ce spike a besoin d'un display X11).
//!
//! Même politique que Windows (§6.5 du plan) : rester `AlwaysOnTop` tant que la fenêtre de jeu (ou
//! l'overlay lui-même) a le focus ; sinon replier après un délai de grâce continu, pour absorber un
//! aléa de timing d'un seul tick sans revenir sur le principe « ne pas recouvrir durablement une
//! autre appli ».

use std::time::{Duration, Instant};

/// Délai de grâce avant repli — même valeur et même raisonnement que
/// `overlay_ui::main::TOPMOST_DEMOTE_GRACE` (1,5 s : absorbe un aléa d'un tick de sondage sans
/// tolérer un vrai changement de fenêtre prolongé).
pub const DEMOTE_GRACE: Duration = Duration::from_millis(1500);

/// État topmost de l'overlay entre deux appels à `decide` — équivalent des deux champs
/// `OverlayWindow::is_topmost`/`pending_demote_since` côté Windows, regroupés ici car ils n'ont de
/// sens que l'un avec l'autre (jamais `pending_demote_since.is_some()` alors que `is_topmost` est
/// déjà `false`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopmostState {
    /// Actuellement au-dessus. `pending_demote_since` : `Some` depuis quand `relevant` est retombé
    /// à faux EN CONTINU (voir `decide`), `None` tant que `relevant` est vrai ou vient tout juste
    /// de redevenir faux sans qu'aucun appel précédent ne l'ait encore noté.
    Above {
        pending_demote_since: Option<Instant>,
    },
    /// Actuellement en z-order normal (repli déjà appliqué).
    Normal,
}

/// Action que l'appelant doit exécuter sur la vraie fenêtre X11 (jamais décidée ici — cette
/// fonction ne touche à aucun état système, voir la doc de module).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopmostAction {
    /// Rien à changer côté fenêtre système (mais `state` peut quand même avoir changé — ex.
    /// `pending_demote_since` posé pour la première fois).
    None,
    /// Appliquer `_NET_WM_STATE_ABOVE` (ou `WindowLevel::AlwaysOnTop` côté winit).
    SetAbove,
    /// Retirer `_NET_WM_STATE_ABOVE` (ou `WindowLevel::Normal` côté winit) — le délai de grâce est
    /// écoulé sans que `relevant` ne soit redevenu vrai entre-temps.
    SetNormal,
}

/// Calcule la nouvelle transition à partir de l'état précédent — jamais d'écriture, jamais de
/// lecture d'horloge : `now` et `relevant` (le jeu ou l'overlay a-t-il le focus À CET INSTANT) sont
/// entièrement fournis par l'appelant (voir `main.rs::poll_topmost`, seul endroit qui lit
/// `_NET_ACTIVE_WINDOW` et `Instant::now()`).
pub fn decide(state: TopmostState, relevant: bool, now: Instant) -> (TopmostState, TopmostAction) {
    match state {
        TopmostState::Above {
            pending_demote_since,
        } => {
            if relevant {
                // Toute réaffirmation reste immédiate — jamais de délai à REDEVENIR pertinent,
                // seul le repli est temporisé (voir la doc de module).
                (
                    TopmostState::Above {
                        pending_demote_since: None,
                    },
                    TopmostAction::None,
                )
            } else {
                let demote_due_at = pending_demote_since.unwrap_or(now);
                if now.duration_since(demote_due_at) < DEMOTE_GRACE {
                    (
                        TopmostState::Above {
                            pending_demote_since: Some(demote_due_at),
                        },
                        TopmostAction::None,
                    )
                } else {
                    (TopmostState::Normal, TopmostAction::SetNormal)
                }
            }
        }
        TopmostState::Normal => {
            if relevant {
                (
                    TopmostState::Above {
                        pending_demote_since: None,
                    },
                    TopmostAction::SetAbove,
                )
            } else {
                (TopmostState::Normal, TopmostAction::None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(secs_from_epoch_like: u64) -> Instant {
        // `Instant` n'a pas de constructeur "à une valeur donnée" en API stable — on avance depuis
        // une base commune, ce qui suffit pour exprimer des écarts relatifs déterministes dans ces
        // tests (jamais besoin d'une date absolue).
        let base = Instant::now();
        base + Duration::from_secs(secs_from_epoch_like)
    }

    #[test]
    fn reste_above_tant_que_relevant_reste_vrai() {
        let (state, action) = decide(
            TopmostState::Above {
                pending_demote_since: None,
            },
            true,
            t(10),
        );
        assert_eq!(
            state,
            TopmostState::Above {
                pending_demote_since: None
            }
        );
        assert_eq!(action, TopmostAction::None);
    }

    #[test]
    fn passe_above_immediatement_depuis_normal_si_relevant() {
        let (state, action) = decide(TopmostState::Normal, true, t(0));
        assert_eq!(
            state,
            TopmostState::Above {
                pending_demote_since: None
            }
        );
        assert_eq!(action, TopmostAction::SetAbove);
    }

    #[test]
    fn ne_demote_pas_avant_lecheance_du_delai_de_grace() {
        // `t0` figé une seule fois : `t(0)` rappelé à chaque assertion capturerait un nouvel
        // `Instant::now()` à chaque fois (précision nanoseconde), jamais rigoureusement égal au
        // précédent — piège du helper `t()`, qui n'a de sens que pour exprimer un ÉCART depuis une
        // base commune, jamais pour reproduire « le même instant » par deux appels séparés.
        let t0 = t(0);
        let (state, action) = decide(
            TopmostState::Above {
                pending_demote_since: None,
            },
            false,
            t0,
        );
        assert_eq!(
            state,
            TopmostState::Above {
                pending_demote_since: Some(t0)
            }
        );
        assert_eq!(action, TopmostAction::None);

        // Toujours dans le délai (1,5 s) une seconde plus tard.
        let (state, action) = decide(state, false, t0 + Duration::from_secs(1));
        assert_eq!(
            state,
            TopmostState::Above {
                pending_demote_since: Some(t0)
            }
        );
        assert_eq!(action, TopmostAction::None);
    }

    #[test]
    fn demote_une_fois_le_delai_de_grace_ecoule() {
        let start = TopmostState::Above {
            pending_demote_since: Some(t(0)),
        };
        let (state, action) = decide(start, false, t(0) + DEMOTE_GRACE);
        assert_eq!(state, TopmostState::Normal);
        assert_eq!(action, TopmostAction::SetNormal);
    }

    #[test]
    fn un_retour_a_relevant_avant_lecheance_annule_la_demotion_sans_jamais_avoir_replie() {
        let pending = TopmostState::Above {
            pending_demote_since: Some(t(0)),
        };
        let (state, action) = decide(pending, true, t(0) + Duration::from_millis(500));
        assert_eq!(
            state,
            TopmostState::Above {
                pending_demote_since: None
            }
        );
        assert_eq!(action, TopmostAction::None); // déjà Above, rien à réappliquer côté fenêtre
    }

    #[test]
    fn un_seul_tick_dun_aleu_de_timing_nentraine_jamais_de_demotion_visible() {
        // Reproduction du bug corrigé côté Windows le 2026-09-02 (§6.5 du plan) : un aléa
        // d'ordonnancement d'un seul tick de sondage (~50 ms) ne doit jamais suffire à démoter.
        const POLL_TICK: Duration = Duration::from_millis(50);
        let mut state = TopmostState::Above {
            pending_demote_since: None,
        };
        let mut now = t(0);
        // `relevant` retombe à faux pendant UN SEUL tick, puis redevient vrai.
        let (next, action) = decide(state, false, now);
        state = next;
        assert_eq!(action, TopmostAction::None);
        now += POLL_TICK;
        let (next, action) = decide(state, true, now);
        state = next;
        assert_eq!(action, TopmostAction::None); // jamais démoté : aucune fenêtre n'a bougé
        assert_eq!(
            state,
            TopmostState::Above {
                pending_demote_since: None
            }
        );
    }

    #[test]
    fn reste_normal_tant_que_jamais_pertinent() {
        let (state, action) = decide(TopmostState::Normal, false, t(0));
        assert_eq!(state, TopmostState::Normal);
        assert_eq!(action, TopmostAction::None);
    }
}
