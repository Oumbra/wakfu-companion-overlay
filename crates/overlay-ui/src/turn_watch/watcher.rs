//! Machine d'états de la surveillance de tour — **sans OS** : la capture, l'horloge et l'état du
//! combat lui sont donnés à chaque tick, elle rend des événements. C'est ce qui la rend testable
//! sur les fixtures du spike S4 sans fenêtre de jeu.
//!
//! ## Le problème que ce module résout
//!
//! Le widget « Fin du tour » affiche le nom du combattant actif, mais l'overlay ne sait pas *lire*
//! un nom — il sait seulement comparer deux images de nom ([`vision::similarity`]). Il lui faut
//! donc, pour chaque personnage P, un **gabarit** : l'image de son nom telle que le jeu la peint.
//!
//! ## Apprendre le gabarit sans rien demander à l'utilisateur
//!
//! Le log dit quand P lance un sort (`FighterDamage::last_turn_casts` grandit). Un sort se lance
//! pendant son propre tour : à cet instant, le widget de la fenêtre de P affiche P — **si** il est
//! au repos (panneau doré, voir [`vision::find_gold_panel`]) et non sur la carte d'un combattant
//! survolé. Le module ouvre donc une courte fenêtre d'apprentissage à chaque sort, ne prend un
//! échantillon qu'au repos, et n'acquiert le gabarit qu'avec **deux échantillons concordants pris
//! à deux sorts différents** : un tour qui se termine pile après le sort (le nom bascule vers le
//! suivant) ne peut pas contaminer les deux.
//!
//! Tant que le gabarit manque, rien n'est notifié pour P — l'apprentissage se fait au fil du
//! premier combat, et vaut pour tous les suivants (persisté par l'appelant).
//!
//! ## Reconnaître, puis décider
//!
//! Gabarit acquis, chaque tick compare le nom affiché au gabarit : P est actif si la ressemblance
//! dépasse [`MATCH_THRESHOLD`]. C'est le **front montant** (P ne l'était pas, il l'est) qui vaut
//! « c'est à P de jouer » — et il n'est notifié que si la fenêtre de P n'est pas au premier plan,
//! au plus une fois par [`NOTIFY_COOLDOWN`].
//!
//! **Pas de notification en phase de placement** (demande du 2026-09-14) : le widget y affiche le
//! personnage local sous un bouton « Prêt », doré comme « Fin du tour », et son nom se reconnaît —
//! sans garde, chaque combat commencerait par un toast. Le combat est dit **engagé** dès que le
//! panneau porte « Fin du tour » ([`vision::panel_shows_end_turn`]) ou que le log a vu un sort ;
//! c'est acquis pour le reste du combat, et rien n'est notifié avant.
//!
//! **Une notification par tour, jamais de rafale** : le front montant seul ne suffit pas quand les
//! tours s'enchaînent en quelques secondes (observé en test contre un mannequin, tours passés à la
//! volée) — [`NOTIFY_COOLDOWN`] par personnage borne la cadence à ce qu'un vrai combat produit, où
//! un cycle complet dépasse largement la demi-minute.
//!
//! ## La géométrie, apprise elle aussi
//!
//! La bande du nom est déduite du panneau doré la première fois qu'il est vu, puis **mémorisée par
//! taille de bande** : deux clients à la même résolution partagent la même géométrie, et une
//! fenêtre qui reste sur une carte de stats (survol persistant, vu dans S4) se lit avec la
//! géométrie apprise sur l'autre.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::vision::{self, Band, Glyph, Rect};

/// Ressemblance au gabarit à partir de laquelle le nom affiché est celui du personnage.
/// Calibré sur S4 : même nom, repos contre carte de stats, ressemble à plus de 0,9 ; deux noms
/// différents, à moins de 0,2.
pub const MATCH_THRESHOLD: f64 = 0.85;
/// Concordance exigée entre les deux échantillons d'apprentissage.
const LEARN_THRESHOLD: f64 = 0.93;
/// Durée pendant laquelle un sort de P autorise à échantillonner le nom affiché.
const LEARN_WINDOW: Duration = Duration::from_millis(2500);
/// Deux notifications pour le même personnage ne peuvent pas être plus rapprochées que ça. Un tour
/// de Wakfu dure 30 s de base plus le report ; deux tours d'un même personnage sont séparés par
/// ceux de tous les autres combattants. 25 s coupe les rafales sans jamais rater un vrai tour.
pub const NOTIFY_COOLDOWN: Duration = Duration::from_secs(25);
/// Ticks consécutifs sans reconnaissance avant de considérer que le tour du personnage est fini.
/// Un parasite d'un tick (halo de l'étincelle animée, capture au milieu d'une transition) ne doit
/// pas produire un faux front descendant — puis un faux front montant, et un toast de trop.
const INACTIVE_TICKS: u32 = 2;

/// Ce que le moteur sait du combat du personnage, au moment du tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FightFacts {
    pub ongoing: bool,
    /// Le log a vu au moins un sort ou des dégâts dans ce combat : la phase de placement est
    /// passée, quoi que montre l'écran.
    pub engaged_by_log: bool,
    /// `FighterDamage::last_turn_casts.len()` du personnage — ses sorts du tour en cours. Toute
    /// variation vers une valeur non nulle signale un sort qui vient d'être lancé.
    pub own_cast_len: usize,
}

/// Ce que l'appelant fournit pour une fenêtre, à chaque tick.
pub struct TickInput<'a> {
    pub character: &'a str,
    /// `None` : pas de capture ce tick (fenêtre minimisée, `PrintWindow` en échec).
    pub band: Option<&'a Band>,
    /// `None` : aucun combat connu pour ce personnage.
    pub fight: Option<FightFacts>,
    pub foreground: bool,
    pub now: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// C'est au tour de ce personnage, et sa fenêtre n'est pas au premier plan.
    Notify { character: String },
    /// Gabarit acquis pour ce personnage — à persister par l'appelant.
    TemplateLearned { character: String, glyph: Glyph },
}

#[derive(Debug, Default)]
struct WindowState {
    prev_cast_len: usize,
    /// Nombre de sorts vus — identifie l'échantillon d'apprentissage à un sort.
    spell_seq: u64,
    learn_until: Option<Instant>,
    candidate: Option<(Glyph, u64)>,
    active: bool,
    /// Ticks consécutifs sans reconnaissance pendant que `active` — voir `INACTIVE_TICKS`.
    misses: u32,
    /// Le combat a quitté la phase de placement — acquis jusqu'à la fin du combat.
    engaged: bool,
    last_notified: Option<Instant>,
}

#[derive(Debug, Default)]
pub struct Watcher {
    templates: HashMap<String, Glyph>,
    geometry_by_size: HashMap<(u32, u32), Rect>,
    windows: HashMap<String, WindowState>,
}

impl Watcher {
    /// `templates` : les gabarits déjà appris (chargés du disque), par nom de personnage tel que
    /// le titre de fenêtre le donne.
    pub fn new(templates: HashMap<String, Glyph>) -> Self {
        Self {
            templates,
            ..Default::default()
        }
    }

    pub fn has_template(&self, character: &str) -> bool {
        self.templates.contains_key(character)
    }

    /// Un tick pour une fenêtre. Zéro, un ou deux événements (un gabarit peut être acquis au tick
    /// même où il reconnaît déjà le personnage).
    pub fn tick(&mut self, input: TickInput<'_>) -> Vec<Event> {
        let mut events = Vec::new();
        let state = self.windows.entry(input.character.to_string()).or_default();

        let Some(fight) = input.fight.filter(|f| f.ongoing) else {
            // Hors combat : tout repart de zéro au prochain, gabarit et géométrie exceptés.
            *state = WindowState::default();
            return events;
        };

        // Un sort de P ouvre la fenêtre d'apprentissage. La longueur peut aussi *baisser* (nouveau
        // tour, liste vidée puis premier sort) : c'est un sort aussi.
        if fight.own_cast_len != state.prev_cast_len && fight.own_cast_len > 0 {
            state.spell_seq += 1;
            state.learn_until = Some(input.now + LEARN_WINDOW);
        }
        state.prev_cast_len = fight.own_cast_len;

        let Some(band) = input.band else {
            return events;
        };

        // Géométrie : celle déjà connue pour cette taille, sinon apprise sur un panneau doré.
        let size = (band.width, band.height);
        let panel = vision::find_gold_panel(band);
        if !state.engaged
            && (fight.engaged_by_log
                || panel.is_some_and(|p| vision::panel_shows_end_turn(band, p)))
        {
            state.engaged = true;
            tracing::info!(
                "[tour] {} : combat engagé, placement terminé",
                input.character
            );
        }
        let area = match self.geometry_by_size.get(&size) {
            Some(area) => *area,
            None => match panel {
                Some(p) => {
                    let area = vision::name_area_above(p, band);
                    tracing::info!(
                        "[tour] {} : bande du nom localisée ({}x{} → {:?})",
                        input.character,
                        size.0,
                        size.1,
                        area
                    );
                    self.geometry_by_size.insert(size, area);
                    area
                }
                None => return events,
            },
        };

        let Some(glyph) = vision::extract_glyph(band, area) else {
            state.misses += 1;
            if state.misses >= INACTIVE_TICKS {
                state.active = false;
            }
            return events;
        };

        // Apprentissage — au repos seulement, dans la fenêtre ouverte par un sort.
        if !self.templates.contains_key(input.character)
            && panel.is_some()
            && state.learn_until.is_some_and(|until| input.now < until)
        {
            match &state.candidate {
                Some((first, seq)) if *seq != state.spell_seq => {
                    if vision::similarity(first, &glyph) >= LEARN_THRESHOLD {
                        tracing::info!(
                            "[tour] {} : gabarit du nom acquis ({}x{})",
                            input.character,
                            glyph.w,
                            glyph.h
                        );
                        self.templates
                            .insert(input.character.to_string(), glyph.clone());
                        events.push(Event::TemplateLearned {
                            character: input.character.to_string(),
                            glyph: glyph.clone(),
                        });
                        state.candidate = None;
                    } else {
                        // Deux sorts, deux noms : l'un des deux était un survol ou un
                        // changement de tour. On repart du plus récent.
                        state.candidate = Some((glyph.clone(), state.spell_seq));
                    }
                }
                Some(_) => {} // même sort que le premier échantillon : on attend le suivant
                None => state.candidate = Some((glyph.clone(), state.spell_seq)),
            }
        }

        // Reconnaissance.
        let Some(template) = self.templates.get(input.character) else {
            return events;
        };
        let score = vision::similarity(template, &glyph);
        let is_active = score >= MATCH_THRESHOLD;
        tracing::trace!(
            "[tour] {} : ressemblance {score:.2} ({}x{})",
            input.character,
            glyph.w,
            glyph.h
        );
        if is_active && !state.active {
            let cooled = state
                .last_notified
                .is_none_or(|t| input.now.duration_since(t) >= NOTIFY_COOLDOWN);
            if !state.engaged {
                tracing::debug!(
                    "[tour] {} : tour reconnu, pas de notification (phase de placement)",
                    input.character
                );
            } else if !input.foreground && cooled {
                state.last_notified = Some(input.now);
                events.push(Event::Notify {
                    character: input.character.to_string(),
                });
            } else {
                tracing::debug!(
                    "[tour] {} : tour reconnu, pas de notification ({})",
                    input.character,
                    if input.foreground {
                        "fenêtre au premier plan"
                    } else {
                        "trop tôt après la précédente"
                    }
                );
            }
        }
        // Front descendant retardé : `INACTIVE_TICKS` ticks sans reconnaissance.
        if is_active {
            state.active = true;
            state.misses = 0;
        } else {
            state.misses += 1;
            if state.misses >= INACTIVE_TICKS {
                state.active = false;
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Band {
        let path = format!(
            "{}/tests/fixtures/turn-watch/{name}.png",
            env!("CARGO_MANIFEST_DIR")
        );
        let img = image::open(&path).unwrap().into_rgba8();
        Band {
            width: img.width(),
            height: img.height(),
            rgba: img.into_raw(),
        }
    }

    fn fight(casts: usize) -> Option<FightFacts> {
        Some(FightFacts {
            ongoing: true,
            engaged_by_log: true,
            own_cast_len: casts,
        })
    }

    fn facts(engaged: bool) -> Option<FightFacts> {
        Some(FightFacts {
            ongoing: true,
            engaged_by_log: engaged,
            own_cast_len: 0,
        })
    }

    #[test]
    fn pas_de_notification_pendant_le_placement() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        learn_oumbra(&mut w, t0);
        // Fin du combat, puis un nouveau : la bande au repos porte « Prêt » et le nom du personnage
        // local, en arrière-plan, sans aucun sort au log — c'est la phase de placement.
        w.tick(TickInput {
            character: "Oumbra",
            band: None,
            fight: Some(FightFacts {
                ongoing: false,
                engaged_by_log: false,
                own_cast_len: 0,
            }),
            foreground: false,
            now: t0 + Duration::from_secs(60),
        });
        // La fixture « Prêt » porte le nom « Pugio Letalis » : on surveille donc Pugio ici, avec le
        // gabarit d'Oumbra rebaptisé — seul le mécanisme compte.
        let pret = fixture("repos-pugio-t18");
        let glyph = vision::extract_glyph(
            &pret,
            vision::name_area_above(vision::find_gold_panel(&pret).unwrap(), &pret),
        )
        .unwrap();
        w.templates.insert("Pugio Letalis".to_string(), glyph);
        for i in 0..4 {
            let ev = w.tick(TickInput {
                character: "Pugio Letalis",
                band: Some(&pret),
                fight: facts(false),
                foreground: false,
                now: t0 + Duration::from_secs(61 + i),
            });
            assert!(ev.is_empty(), "placement : {ev:?}");
        }
        // Le log voit un sort : engagé. Le tour est déjà reconnu (pas de front montant), donc pas
        // de notification rétroactive non plus ; il faut un vrai nouveau tour.
        let ev = w.tick(TickInput {
            character: "Pugio Letalis",
            band: Some(&pret),
            fight: facts(true),
            foreground: false,
            now: t0 + Duration::from_secs(66),
        });
        assert!(ev.is_empty(), "{ev:?}");
        let autre = fixture("repos-oumbra");
        for i in 0..2 {
            w.tick(TickInput {
                character: "Pugio Letalis",
                band: Some(&autre),
                fight: facts(true),
                foreground: false,
                now: t0 + Duration::from_secs(70 + i),
            });
        }
        let ev = w.tick(TickInput {
            character: "Pugio Letalis",
            band: Some(&pret),
            fight: facts(true),
            foreground: false,
            now: t0 + Duration::from_secs(100),
        });
        assert_eq!(
            ev,
            vec![Event::Notify {
                character: "Pugio Letalis".to_string()
            }]
        );
    }

    /// Joue les ticks qui font acquérir le gabarit d'« Oumbra » : deux sorts, la bande au repos.
    fn learn_oumbra(w: &mut Watcher, t0: Instant) -> Vec<Event> {
        let repos = fixture("repos-oumbra");
        let mut all = Vec::new();
        // Premier sort : échantillon.
        all.extend(w.tick(TickInput {
            character: "Oumbra",
            band: Some(&repos),
            fight: fight(1),
            foreground: true,
            now: t0,
        }));
        // Même sort, tick suivant : rien de plus.
        all.extend(w.tick(TickInput {
            character: "Oumbra",
            band: Some(&repos),
            fight: fight(1),
            foreground: true,
            now: t0 + Duration::from_millis(500),
        }));
        // Deuxième sort : confirmation.
        all.extend(w.tick(TickInput {
            character: "Oumbra",
            band: Some(&repos),
            fight: fight(2),
            foreground: true,
            now: t0 + Duration::from_secs(3),
        }));
        all
    }

    #[test]
    fn le_gabarit_s_apprend_sur_deux_sorts_au_repos() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        let events = learn_oumbra(&mut w, t0);
        assert!(
            matches!(events.as_slice(), [Event::TemplateLearned { character, .. }] if character == "Oumbra"),
            "{events:?}"
        );
        assert!(w.has_template("Oumbra"));
    }

    #[test]
    fn pas_d_apprentissage_sans_sort_ni_hors_repos() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        let repos = fixture("repos-oumbra");
        let carte = fixture("carte-pugio");
        // Aucun sort : la bande au repos ne suffit pas.
        for i in 0..4 {
            let ev = w.tick(TickInput {
                character: "Oumbra",
                band: Some(&repos),
                fight: fight(0),
                foreground: true,
                now: t0 + Duration::from_millis(500 * i),
            });
            assert!(ev.is_empty());
        }
        // Des sorts, mais la carte de stats à l'écran : pas d'échantillon.
        for i in 0..4 {
            let ev = w.tick(TickInput {
                character: "Oumbra",
                band: Some(&carte),
                fight: fight(i as usize + 1),
                foreground: true,
                now: t0 + Duration::from_secs(5 + 3 * i),
            });
            assert!(ev.is_empty(), "{ev:?}");
        }
        assert!(!w.has_template("Oumbra"));
    }

    #[test]
    fn le_tour_est_notifie_au_front_montant_hors_premier_plan() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        learn_oumbra(&mut w, t0);
        let repos = fixture("repos-oumbra");
        let autre = fixture("repos-pugio-t18"); // le widget affiche « Pugio Letalis »

        // Pendant que « Oumbra » est encore affiché : actif, mais déjà actif → rien.
        let ev = w.tick(TickInput {
            character: "Oumbra",
            band: Some(&repos),
            fight: fight(2),
            foreground: false,
            now: t0 + Duration::from_secs(4),
        });
        assert!(ev.is_empty(), "{ev:?}");
        // Le tour passe à un autre : plus actif — après deux ticks, un seul serait un parasite.
        for i in 0..2 {
            let ev = w.tick(TickInput {
                character: "Oumbra",
                band: Some(&autre),
                fight: fight(2),
                foreground: false,
                now: t0 + Duration::from_secs(10 + i),
            });
            assert!(ev.is_empty());
        }
        // Il revient à Oumbra, fenêtre en arrière-plan : notification.
        let ev = w.tick(TickInput {
            character: "Oumbra",
            band: Some(&repos),
            fight: fight(0),
            foreground: false,
            now: t0 + Duration::from_secs(40),
        });
        assert_eq!(
            ev,
            vec![Event::Notify {
                character: "Oumbra".to_string()
            }]
        );
        // Toujours actif : pas de seconde notification.
        let ev = w.tick(TickInput {
            character: "Oumbra",
            band: Some(&repos),
            fight: fight(0),
            foreground: false,
            now: t0 + Duration::from_secs(41),
        });
        assert!(ev.is_empty());
    }

    #[test]
    fn pas_de_notification_au_premier_plan() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        learn_oumbra(&mut w, t0);
        let repos = fixture("repos-oumbra");
        let autre = fixture("repos-pugio-t18");
        for i in 0..2 {
            w.tick(TickInput {
                character: "Oumbra",
                band: Some(&autre),
                fight: fight(2),
                foreground: true,
                now: t0 + Duration::from_secs(10 + i),
            });
        }
        let ev = w.tick(TickInput {
            character: "Oumbra",
            band: Some(&repos),
            fight: fight(0),
            foreground: true,
            now: t0 + Duration::from_secs(40),
        });
        assert!(ev.is_empty(), "{ev:?}");
    }

    #[test]
    fn la_carte_de_survol_se_reconnait_avec_la_geometrie_apprise() {
        // La fenêtre Pugio reste sur la carte de stats d'« Oumbra » (survol persistant, vu dans
        // S4) : sans panneau doré, la géométrie vient de la fenêtre Oumbra, de même taille.
        let mut w = Watcher::default();
        let t0 = Instant::now();
        learn_oumbra(&mut w, t0);
        let carte = fixture("carte-pugio");
        // Hypothèse du test : « Oumbra » est aussi surveillé par une fenêtre qui affiche la carte.
        let autre = fixture("repos-pugio-t18");
        for i in 0..2 {
            w.tick(TickInput {
                character: "Oumbra",
                band: Some(&autre),
                fight: fight(2),
                foreground: false,
                now: t0 + Duration::from_secs(10 + i),
            });
        }
        let ev = w.tick(TickInput {
            character: "Oumbra",
            band: Some(&carte),
            fight: fight(2),
            foreground: false,
            now: t0 + Duration::from_secs(40),
        });
        assert_eq!(
            ev,
            vec![Event::Notify {
                character: "Oumbra".to_string()
            }]
        );
    }

    #[test]
    fn la_fin_du_combat_remet_l_etat_a_zero_mais_garde_le_gabarit() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        learn_oumbra(&mut w, t0);
        let ev = w.tick(TickInput {
            character: "Oumbra",
            band: None,
            fight: Some(FightFacts {
                ongoing: false,
                engaged_by_log: false,
                own_cast_len: 0,
            }),
            foreground: false,
            now: t0 + Duration::from_secs(60),
        });
        assert!(ev.is_empty());
        assert!(w.has_template("Oumbra"));
        assert!(!w.windows["Oumbra"].active);
    }
}
