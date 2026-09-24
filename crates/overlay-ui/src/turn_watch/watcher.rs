//! Machine d'états de la surveillance de tour — **sans OS** : la capture, l'horloge, le titre de
//! la fenêtre et l'état du combat lui sont donnés à chaque tick, elle rend des événements. C'est
//! ce qui la rend testable sur les fixtures du spike S4 sans fenêtre de jeu.
//!
//! ## Le problème que ce module résout
//!
//! Le widget « Fin du tour » affiche le nom du combattant actif, mais l'overlay ne sait pas *lire*
//! un nom — il sait seulement comparer deux images de nom ([`vision::similarity`]). Il lui faut
//! donc, pour chaque personnage P, un **gabarit** : l'image de son nom telle que le jeu la peint.
//!
//! ## Une fenêtre, plusieurs personnages : le titre dit qui est aux commandes
//!
//! Un client Wakfu joue le titulaire de sa fenêtre **et ses héros** — jusqu'à trois personnages
//! du même compte. Et le jeu le dit lui-même : **le titre de la fenêtre** (`"<Nom> - WAKFU"`)
//! porte le personnage aux commandes, et **bascule sur le héros dont le tour commence** — relevé
//! au journal du 2026-09-14 (sondage du premier plan : « Sagittarius Caecus - WAKFU » à l'instant
//! de son sort, puis « Sagitta Lucis - WAKFU » au sien, « Magister Thesaurorum - WAKFU »…). Le
//! titre est donc la vérité de l'appelant, lue à chaque tick ([`TickInput::current`]) : c'est
//! **lui** que la fenêtre surveille, et une bascule de titre pendant un combat engagé vaut à elle
//! seule « c'est à lui de jouer » — sans capture, donc même fenêtre minimisée. La première
//! version demandait au roster les personnages du compte : dépendance à une déclaration
//! extérieure, et rien ne disait quel héros jouait dans quelle fenêtre — en test réel, un seul
//! personnage sur six notifié.
//!
//! Le widget reste nécessaire quand le titre ne bouge pas : fenêtre à un seul personnage, ou
//! premier tour d'un héros après le placement s'il en était déjà le titulaire.
//!
//! ## Apprendre le gabarit sans rien demander à l'utilisateur
//!
//! Deux instants où le widget, **s'il est au repos** (panneau doré, voir
//! [`vision::find_gold_panel`], et non la carte d'un combattant survolé), affiche à coup sûr le
//! personnage aux commandes P : quand le log dit que P lance un sort
//! (`FighterDamage::last_turn_casts` grandit — un sort se lance pendant son propre tour), et quand
//! le titre vient de basculer sur P. Chacun ouvre une courte fenêtre d'échantillonnage, et le
//! gabarit n'est acquis qu'avec **deux échantillons concordants pris à deux événements
//! différents** : un tour qui se termine pile après le sort (le nom bascule vers le suivant) ne
//! peut pas contaminer les deux.
//!
//! Tant que le gabarit manque, le widget ne notifie rien pour P — l'apprentissage se fait au fil
//! du premier combat, et vaut pour tous les suivants (persisté par l'appelant).
//!
//! **Et il se corrige tout seul** : si, à un sort de P au repos, le nom affiché ne ressemble pas
//! au gabarit connu, c'est le gabarit qui a tort (échelle d'interface changée, géométrie corrigée
//! par une mise à jour — vécu le 2026-09-14, un gabarit lu deux pixels trop court et plus jamais
//! reconnu). Le même protocole à deux échantillons concordants le remplace alors.
//!
//! ## Reconnaître, puis décider
//!
//! Gabarit acquis, chaque tick compare le nom affiché à celui du personnage aux commandes : au-
//! dessus de [`MATCH_THRESHOLD`], il est l'actif. C'est le **passage à actif** (il ne l'était pas,
//! il l'est — par le widget ou par le titre) qui vaut « c'est à lui de jouer » — notifié seulement
//! si la fenêtre n'est pas au premier plan, au plus une fois par [`NOTIFY_COOLDOWN`] et par
//! personnage.
//!
//! **Pas de notification en phase de placement** (demande du 2026-09-14) : le widget y affiche le
//! personnage aux commandes sous un bouton « Prêt », doré comme « Fin du tour », et son nom se
//! reconnaît — et le titre y bascule à chaque héros qu'on place. Sans garde, chaque combat
//! commencerait par des toasts. Le combat est dit **engagé** dès que le panneau porte « Fin du
//! tour » ([`vision::panel_shows_end_turn`]) ou que le log a vu un sort ou des dégâts ; c'est
//! acquis pour le reste du combat, et rien n'est notifié avant.
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
/// Durée pendant laquelle un sort de P, ou une bascule du titre sur P, autorise à échantillonner
/// le nom affiché.
const LEARN_WINDOW: Duration = Duration::from_millis(2500);
/// Deux notifications pour le même personnage ne peuvent pas être plus rapprochées que ça. Un tour
/// de Wakfu dure 30 s de base plus le report ; deux tours d'un même personnage sont séparés par
/// ceux de tous les autres combattants. 25 s coupe les rafales sans jamais rater un vrai tour.
pub const NOTIFY_COOLDOWN: Duration = Duration::from_secs(25);
/// Ticks consécutifs sans reconnaissance avant de considérer que le tour du personnage est fini.
/// Un parasite d'un tick (halo de l'étincelle animée, capture au milieu d'une transition) ne doit
/// pas produire un faux front descendant — puis un faux front montant, et un toast de trop.
const INACTIVE_TICKS: u32 = 2;

/// Ce que le moteur sait du combat de la fenêtre, au moment du tick.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FightFacts {
    pub ongoing: bool,
    /// Le log a vu au moins un sort ou des dégâts dans ce combat : la phase de placement est
    /// passée, quoi que montre l'écran.
    pub engaged_by_log: bool,
    /// `FighterDamage::last_turn_casts.len()` du personnage aux commandes
    /// ([`TickInput::current`]), 0 s'il n'est pas dans ce combat. Toute variation vers une valeur
    /// non nulle signale un sort qui vient d'être lancé.
    pub cast_len: usize,
    /// Le personnage aux commandes est un allié de ce combat — une bascule de titre vers un nom
    /// qui n'y combat pas n'est pas un tour.
    pub current_in_fight: bool,
}

/// Ce que l'appelant fournit pour une fenêtre, à chaque tick.
pub struct TickInput<'a> {
    /// La clé de la fenêtre — son titulaire à la création, stable pour toute sa vie.
    pub window: &'a str,
    /// Le personnage aux commandes de la fenêtre **maintenant** : son titre, lu à ce tick (voir
    /// la doc de module).
    pub current: &'a str,
    /// `None` : pas de capture ce tick (fenêtre minimisée, `PrintWindow` en échec).
    pub band: Option<&'a Band>,
    /// `None` : aucun combat connu pour cette fenêtre.
    pub fight: Option<&'a FightFacts>,
    pub foreground: bool,
    pub now: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// C'est au tour de ce personnage, joué depuis une fenêtre qui n'est pas au premier plan.
    Notify { character: String },
    /// Gabarit acquis pour ce personnage — à persister par l'appelant.
    TemplateLearned { character: String, glyph: Glyph },
}

/// Apprentissage en cours pour un personnage, dans une fenêtre.
#[derive(Debug, Default)]
struct Learning {
    prev_cast_len: usize,
    /// Nombre d'événements vus (sorts, bascules de titre) — identifie l'échantillon
    /// d'apprentissage à un événement.
    event_seq: u64,
    learn_until: Option<Instant>,
    candidate: Option<(Glyph, u64)>,
}

#[derive(Debug, Default)]
struct WindowState {
    /// Le personnage aux commandes au tick précédent — une différence avec `current` est une
    /// bascule de titre.
    current: Option<String>,
    learning: HashMap<String, Learning>,
    /// Le personnage reconnu comme actif au tick précédent.
    active: Option<String>,
    /// Ticks consécutifs sans reconnaissance pendant qu'un personnage est `active`.
    misses: u32,
    /// Le combat a quitté la phase de placement — acquis jusqu'à la fin du combat.
    engaged: bool,
    last_notified: HashMap<String, Instant>,
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

    /// Un tick pour une fenêtre. Zéro ou plusieurs événements.
    pub fn tick(&mut self, input: TickInput<'_>) -> Vec<Event> {
        let mut events = Vec::new();
        let state = self.windows.entry(input.window.to_string()).or_default();
        let current = input.current;

        let Some(fight) = input.fight.filter(|f| f.ongoing) else {
            // Hors combat : tout repart de zéro au prochain, gabarit et géométrie exceptés.
            *state = WindowState::default();
            return events;
        };

        let switched = state.current.as_deref().is_some_and(|prev| prev != current);
        state.current = Some(current.to_string());

        // Un sort du personnage aux commandes ouvre sa fenêtre d'échantillonnage — la longueur
        // peut aussi *baisser* (nouveau tour, liste vidée puis premier sort) : c'est un sort
        // aussi. Une bascule de titre sur lui l'ouvre de même.
        {
            let learning = state.learning.entry(current.to_string()).or_default();
            let len = fight.cast_len;
            let cast = len != learning.prev_cast_len && len > 0;
            learning.prev_cast_len = len;
            if cast || switched {
                learning.event_seq += 1;
                learning.learn_until = Some(input.now + LEARN_WINDOW);
            }
        }

        if !state.engaged && fight.engaged_by_log {
            state.engaged = true;
            tracing::info!("[tour] combat engagé, placement terminé");
            tracing::debug!(window = %input.window, "[tour] combat engagé, placement terminé");
        }

        // Le titre vient de basculer sur un combattant du combat engagé : c'est son tour, sans
        // rien lire à l'écran.
        if switched && fight.current_in_fight {
            if state.engaged {
                // Le titre de la fenêtre de jeu porte le nom du personnage, et `current` EST ce
                // nom (constat C6 de `docs/analyse-rgpd.md`) : la bascule se journalise, le
                // personnage part en `debug` (« Journal détaillé »).
                tracing::info!("[tour] la fenêtre passe aux commandes d'un autre personnage");
                tracing::debug!(
                    window = %input.window,
                    character = %current,
                    "[tour] la fenêtre passe aux commandes de ce personnage"
                );
                Self::activate(state, current, input.foreground, input.now, &mut events);
            } else {
                tracing::debug!(
                    "[tour] {current} : bascule de titre, pas de notification (phase de placement)"
                );
            }
        }

        let Some(band) = input.band else {
            return events;
        };

        // Géométrie : celle déjà connue pour cette taille, sinon apprise sur un panneau doré.
        let size = (band.width, band.height);
        let panel = vision::find_gold_panel(band);
        if !state.engaged && panel.is_some_and(|p| vision::panel_shows_end_turn(band, p)) {
            state.engaged = true;
            tracing::info!("[tour] combat engagé, placement terminé");
            tracing::debug!(window = %input.window, "[tour] combat engagé, placement terminé");
        }
        let area = match self.geometry_by_size.get(&size) {
            Some(area) => *area,
            None => match panel {
                Some(p) => {
                    let area = vision::name_area_above(p, band);
                    tracing::info!(
                        "[tour] bande du nom localisée ({}x{} → {:?})",
                        size.0,
                        size.1,
                        area
                    );
                    tracing::debug!(window = %input.window, "[tour] bande du nom localisée");
                    self.geometry_by_size.insert(size, area);
                    area
                }
                None => return events,
            },
        };

        let Some(glyph) = vision::extract_glyph(band, area) else {
            state.misses += 1;
            if state.misses >= INACTIVE_TICKS {
                state.active = None;
            }
            return events;
        };

        // Apprentissage — au repos seulement, dans la fenêtre ouverte par un sort du personnage
        // aux commandes ou une bascule sur lui. Aussi quand un gabarit existe mais ne reconnaît
        // pas ce que le jeu affiche à cet instant : il est obsolète, et se remplace par le même
        // protocole.
        if panel.is_some() {
            if let Some(learning) = state.learning.get_mut(current) {
                if learning.learn_until.is_some_and(|until| input.now < until) {
                    let disagrees = self
                        .templates
                        .get(current)
                        .is_some_and(|t| vision::similarity(t, &glyph) < MATCH_THRESHOLD);
                    if !self.templates.contains_key(current) || disagrees {
                        match &learning.candidate {
                            Some((first, seq)) if *seq != learning.event_seq => {
                                if vision::similarity(first, &glyph) >= LEARN_THRESHOLD {
                                    tracing::info!(
                                        "[tour] gabarit du nom {} ({}x{})",
                                        if disagrees {
                                            "remplacé — l'ancien ne reconnaissait plus"
                                        } else {
                                            "acquis"
                                        },
                                        glyph.w,
                                        glyph.h
                                    );
                                    tracing::debug!(
                                        character = %current,
                                        window = %input.window,
                                        "[tour] gabarit du nom appris"
                                    );
                                    self.templates.insert(current.to_string(), glyph.clone());
                                    events.push(Event::TemplateLearned {
                                        character: current.to_string(),
                                        glyph: glyph.clone(),
                                    });
                                    learning.candidate = None;
                                } else {
                                    // Deux événements, deux noms : l'un des deux était un survol
                                    // ou un changement de tour. On repart du plus récent.
                                    learning.candidate = Some((glyph.clone(), learning.event_seq));
                                }
                            }
                            // Même événement que le premier échantillon : on attend le suivant.
                            Some(_) => {}
                            None => learning.candidate = Some((glyph.clone(), learning.event_seq)),
                        }
                    }
                }
            }
        }

        // Reconnaissance : le nom affiché est-il celui du personnage aux commandes ?
        let recognized = self.templates.get(current).is_some_and(|template| {
            let score = vision::similarity(template, &glyph);
            tracing::trace!(
                "[tour] {} / {current} : ressemblance {score:.2} ({}x{})",
                input.window,
                glyph.w,
                glyph.h
            );
            score >= MATCH_THRESHOLD
        });

        if recognized {
            if state.active.as_deref() != Some(current) {
                if state.engaged {
                    Self::activate(state, current, input.foreground, input.now, &mut events);
                } else {
                    tracing::debug!(
                        "[tour] {current} : tour reconnu, pas de notification (phase de placement)"
                    );
                }
            }
            state.active = Some(current.to_string());
            state.misses = 0;
        } else {
            // Front descendant retardé : `INACTIVE_TICKS` ticks sans reconnaissance.
            state.misses += 1;
            if state.misses >= INACTIVE_TICKS {
                state.active = None;
            }
        }
        events
    }

    /// `character` vient de passer actif dans un combat engagé — notifié si la fenêtre n'est pas
    /// au premier plan et que sa dernière notification date d'assez longtemps.
    fn activate(
        state: &mut WindowState,
        character: &str,
        foreground: bool,
        now: Instant,
        events: &mut Vec<Event>,
    ) {
        if state.active.as_deref() != Some(character) {
            let cooled = state
                .last_notified
                .get(character)
                .is_none_or(|t| now.duration_since(*t) >= NOTIFY_COOLDOWN);
            if !foreground && cooled {
                state.last_notified.insert(character.to_string(), now);
                events.push(Event::Notify {
                    character: character.to_string(),
                });
            } else {
                tracing::debug!(
                    "[tour] {character} : tour reconnu, pas de notification ({})",
                    if foreground {
                        "fenêtre au premier plan"
                    } else {
                        "trop tôt après la précédente"
                    }
                );
            }
        }
        state.active = Some(character.to_string());
        state.misses = 0;
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

    fn facts(engaged: bool, cast_len: usize) -> FightFacts {
        FightFacts {
            ongoing: true,
            engaged_by_log: engaged,
            cast_len,
            current_in_fight: true,
        }
    }

    fn tick<'a>(
        w: &mut Watcher,
        window: &'a str,
        current: &'a str,
        band: Option<&'a Band>,
        fight: &'a FightFacts,
        foreground: bool,
        now: Instant,
    ) -> Vec<Event> {
        w.tick(TickInput {
            window,
            current,
            band,
            fight: Some(fight),
            foreground,
            now,
        })
    }

    /// Joue les ticks qui font acquérir le gabarit d'« Oumbra » : deux sorts, la bande au repos.
    fn learn_oumbra(w: &mut Watcher, t0: Instant) -> Vec<Event> {
        let repos = fixture("repos-oumbra");
        let mut all = Vec::new();
        let f1 = facts(true, 1);
        all.extend(tick(w, "Oumbra", "Oumbra", Some(&repos), &f1, true, t0));
        all.extend(tick(
            w,
            "Oumbra",
            "Oumbra",
            Some(&repos),
            &f1,
            true,
            t0 + Duration::from_millis(500),
        ));
        let f2 = facts(true, 2);
        all.extend(tick(
            w,
            "Oumbra",
            "Oumbra",
            Some(&repos),
            &f2,
            true,
            t0 + Duration::from_secs(3),
        ));
        all
    }

    #[test]
    fn le_gabarit_s_apprend_sur_deux_sorts_au_repos() {
        let mut w = Watcher::default();
        let events = learn_oumbra(&mut w, Instant::now());
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
        let f0 = facts(true, 0);
        for i in 0..4 {
            let ev = tick(
                &mut w,
                "Oumbra",
                "Oumbra",
                Some(&repos),
                &f0,
                true,
                t0 + Duration::from_millis(500 * i),
            );
            assert!(ev.is_empty());
        }
        for i in 0..4u64 {
            let f = facts(true, i as usize + 1);
            let ev = tick(
                &mut w,
                "Oumbra",
                "Oumbra",
                Some(&carte),
                &f,
                true,
                t0 + Duration::from_secs(5 + 3 * i),
            );
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
        let autre = fixture("repos-pugio-t18");
        let f = facts(true, 2);
        // Déjà actif → rien.
        let ev = tick(
            &mut w,
            "Oumbra",
            "Oumbra",
            Some(&repos),
            &f,
            false,
            t0 + Duration::from_secs(4),
        );
        assert!(ev.is_empty(), "{ev:?}");
        // Le tour passe à un autre : plus actif — après deux ticks, un seul serait un parasite.
        for i in 0..2 {
            let ev = tick(
                &mut w,
                "Oumbra",
                "Oumbra",
                Some(&autre),
                &f,
                false,
                t0 + Duration::from_secs(10 + i),
            );
            assert!(ev.is_empty());
        }
        // Il revient à Oumbra, fenêtre en arrière-plan : notification.
        let ev = tick(
            &mut w,
            "Oumbra",
            "Oumbra",
            Some(&repos),
            &f,
            false,
            t0 + Duration::from_secs(40),
        );
        assert_eq!(
            ev,
            vec![Event::Notify {
                character: "Oumbra".to_string()
            }]
        );
        // Toujours actif : pas de seconde notification.
        let ev = tick(
            &mut w,
            "Oumbra",
            "Oumbra",
            Some(&repos),
            &f,
            false,
            t0 + Duration::from_secs(41),
        );
        assert!(ev.is_empty());
    }

    #[test]
    fn pas_de_notification_au_premier_plan() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        learn_oumbra(&mut w, t0);
        let repos = fixture("repos-oumbra");
        let autre = fixture("repos-pugio-t18");
        let f = facts(true, 2);
        for i in 0..2 {
            tick(
                &mut w,
                "Oumbra",
                "Oumbra",
                Some(&autre),
                &f,
                true,
                t0 + Duration::from_secs(10 + i),
            );
        }
        let ev = tick(
            &mut w,
            "Oumbra",
            "Oumbra",
            Some(&repos),
            &f,
            true,
            t0 + Duration::from_secs(40),
        );
        assert!(ev.is_empty(), "{ev:?}");
    }

    #[test]
    fn la_carte_de_survol_se_reconnait_avec_la_geometrie_apprise() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        learn_oumbra(&mut w, t0);
        let carte = fixture("carte-pugio");
        let autre = fixture("repos-pugio-t18");
        let f = facts(true, 2);
        for i in 0..2 {
            tick(
                &mut w,
                "Oumbra",
                "Oumbra",
                Some(&autre),
                &f,
                false,
                t0 + Duration::from_secs(10 + i),
            );
        }
        let ev = tick(
            &mut w,
            "Oumbra",
            "Oumbra",
            Some(&carte),
            &f,
            false,
            t0 + Duration::from_secs(40),
        );
        assert_eq!(
            ev,
            vec![Event::Notify {
                character: "Oumbra".to_string()
            }]
        );
    }

    #[test]
    fn le_tour_est_notifie_sous_le_voile_des_bonus_de_tour() {
        // Vraies captures du 2026-09-17 : le tour de « Canis Furiosus » commence derrière la
        // sélection des bonus de tour — écran voilé, panneau plus doré. Le gabarit appris au
        // repos doit le reconnaître, sinon un joueur qui n'a pas la fenêtre sous les yeux n'est
        // jamais prévenu.
        let mut w = Watcher::default();
        let t0 = Instant::now();
        let canis = "Canis Furiosus";
        let repos = fixture("repos-canis");
        let voile = fixture("voile-canis");
        let autre = fixture("repos-pugio-t18");
        // Apprentissage : deux sorts, la bande au repos.
        let f1 = facts(true, 1);
        tick(&mut w, canis, canis, Some(&repos), &f1, true, t0);
        let f2 = facts(true, 2);
        let ev = tick(
            &mut w,
            canis,
            canis,
            Some(&repos),
            &f2,
            true,
            t0 + Duration::from_secs(3),
        );
        assert!(
            matches!(ev.as_slice(), [Event::TemplateLearned { character, .. }] if character == canis),
            "{ev:?}"
        );
        // Le tour passe à un autre, puis revient sous le voile, fenêtre en arrière-plan.
        for i in 0..2 {
            tick(
                &mut w,
                canis,
                canis,
                Some(&autre),
                &f2,
                false,
                t0 + Duration::from_secs(10 + i),
            );
        }
        let ev = tick(
            &mut w,
            canis,
            canis,
            Some(&voile),
            &f2,
            false,
            t0 + Duration::from_secs(40),
        );
        assert_eq!(
            ev,
            vec![Event::Notify {
                character: canis.to_string()
            }]
        );
    }

    #[test]
    fn la_fin_du_combat_remet_l_etat_a_zero_mais_garde_le_gabarit() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        learn_oumbra(&mut w, t0);
        let ended = FightFacts::default();
        let ev = tick(
            &mut w,
            "Oumbra",
            "Oumbra",
            None,
            &ended,
            false,
            t0 + Duration::from_secs(60),
        );
        assert!(ev.is_empty());
        assert!(w.has_template("Oumbra"));
        assert!(w.windows["Oumbra"].active.is_none());
    }

    #[test]
    fn un_gabarit_obsolete_est_remplace_au_sort_suivant() {
        let pret = fixture("repos-pugio-t18");
        let faux = vision::extract_glyph(
            &pret,
            vision::name_area_above(vision::find_gold_panel(&pret).unwrap(), &pret),
        )
        .unwrap();
        let mut w = Watcher::new(HashMap::from([("Oumbra".to_string(), faux)]));
        let events = learn_oumbra(&mut w, Instant::now());
        assert!(
            matches!(events.as_slice(), [Event::TemplateLearned { character, glyph }]
                if character == "Oumbra" && glyph.w > 50 && glyph.w < 90),
            "{events:?}"
        );
    }

    #[test]
    fn pas_de_notification_pendant_le_placement() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        let pret = fixture("repos-pugio-t18");
        let glyph = vision::extract_glyph(
            &pret,
            vision::name_area_above(vision::find_gold_panel(&pret).unwrap(), &pret),
        )
        .unwrap();
        w.templates.insert("Pugio Letalis".to_string(), glyph);
        let placement = facts(false, 0);
        for i in 0..4 {
            let ev = tick(
                &mut w,
                "Pugio Letalis",
                "Pugio Letalis",
                Some(&pret),
                &placement,
                false,
                t0 + Duration::from_secs(i),
            );
            assert!(ev.is_empty(), "placement : {ev:?}");
        }
        // Le log voit un sort : engagé. Déjà reconnu, pas de front : rien de rétroactif.
        let engaged = facts(true, 0);
        let ev = tick(
            &mut w,
            "Pugio Letalis",
            "Pugio Letalis",
            Some(&pret),
            &engaged,
            false,
            t0 + Duration::from_secs(6),
        );
        assert!(ev.is_empty(), "{ev:?}");
        let autre = fixture("repos-oumbra");
        for i in 0..2 {
            tick(
                &mut w,
                "Pugio Letalis",
                "Pugio Letalis",
                Some(&autre),
                &engaged,
                false,
                t0 + Duration::from_secs(10 + i),
            );
        }
        let ev = tick(
            &mut w,
            "Pugio Letalis",
            "Pugio Letalis",
            Some(&pret),
            &engaged,
            false,
            t0 + Duration::from_secs(40),
        );
        assert_eq!(
            ev,
            vec![Event::Notify {
                character: "Pugio Letalis".to_string()
            }]
        );
    }

    #[test]
    fn la_bascule_du_titre_sur_un_heros_notifie_sans_capture() {
        // Fenêtre « Oumbra », combat engagé, en arrière-plan. Le titre passe à « Pugio Letalis »
        // (son tour commence) : notifié aussitôt, sans gabarit ni capture — fenêtre minimisée.
        let mut w = Watcher::default();
        let t0 = Instant::now();
        let f = facts(true, 0);
        let ev = tick(&mut w, "Oumbra", "Oumbra", None, &f, false, t0);
        assert!(ev.is_empty(), "{ev:?}");
        let ev = tick(
            &mut w,
            "Oumbra",
            "Pugio Letalis",
            None,
            &f,
            false,
            t0 + Duration::from_secs(30),
        );
        assert_eq!(
            ev,
            vec![Event::Notify {
                character: "Pugio Letalis".to_string()
            }]
        );
        // Le titre ne bouge plus : rien de plus.
        let ev = tick(
            &mut w,
            "Oumbra",
            "Pugio Letalis",
            None,
            &f,
            false,
            t0 + Duration::from_secs(31),
        );
        assert!(ev.is_empty(), "{ev:?}");
        // Retour à Oumbra, une minute plus tard : son tour.
        let ev = tick(
            &mut w,
            "Oumbra",
            "Oumbra",
            None,
            &f,
            false,
            t0 + Duration::from_secs(90),
        );
        assert_eq!(
            ev,
            vec![Event::Notify {
                character: "Oumbra".to_string()
            }]
        );
    }

    #[test]
    fn la_bascule_du_titre_pendant_le_placement_ne_notifie_pas() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        let placement = facts(false, 0);
        tick(&mut w, "Oumbra", "Oumbra", None, &placement, false, t0);
        let ev = tick(
            &mut w,
            "Oumbra",
            "Pugio Letalis",
            None,
            &placement,
            false,
            t0 + Duration::from_secs(2),
        );
        assert!(ev.is_empty(), "{ev:?}");
        // Un nom qui ne combat pas ici (bascule hors combat, autre compte) : rien non plus.
        let mut etranger = facts(true, 0);
        etranger.current_in_fight = false;
        let ev = tick(
            &mut w,
            "Oumbra",
            "Inconnu",
            None,
            &etranger,
            false,
            t0 + Duration::from_secs(40),
        );
        assert!(ev.is_empty(), "{ev:?}");
    }

    #[test]
    fn le_titre_puis_le_widget_ne_notifient_qu_une_fois() {
        // Le titre bascule sur Pugio (notifié), puis le widget le reconnaît : pas de doublon. Et
        // la bascule ouvre l'apprentissage : avec un sort ensuite, le gabarit s'acquiert.
        let mut w = Watcher::default();
        let t0 = Instant::now();
        let pret = fixture("repos-pugio-t18");
        let repos = fixture("repos-oumbra");
        let f = facts(true, 0);
        tick(&mut w, "Oumbra", "Oumbra", Some(&repos), &f, false, t0);
        let ev = tick(
            &mut w,
            "Oumbra",
            "Pugio Letalis",
            Some(&pret),
            &f,
            false,
            t0 + Duration::from_secs(30),
        );
        assert_eq!(
            ev,
            vec![Event::Notify {
                character: "Pugio Letalis".to_string()
            }]
        );
        let f1 = facts(true, 1);
        let ev = tick(
            &mut w,
            "Oumbra",
            "Pugio Letalis",
            Some(&pret),
            &f1,
            false,
            t0 + Duration::from_secs(33),
        );
        assert!(
            matches!(ev.as_slice(), [Event::TemplateLearned { character, .. }] if character == "Pugio Letalis"),
            "{ev:?}"
        );
        assert!(w.has_template("Pugio Letalis"));
    }
}
