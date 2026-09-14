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
//! ## Une fenêtre, plusieurs personnages
//!
//! Un client Wakfu joue le titulaire de sa fenêtre **et ses héros** — jusqu'à trois personnages du
//! même compte, qui apparaissent tour à tour comme combattant actif dans la même fenêtre. La
//! première version ne cherchait que le titulaire : en test réel à six personnages, seuls deux
//! étaient notifiés (2026-09-14). Chaque fenêtre surveille donc **tous les personnages de son
//! compte**, que l'appelant tire du roster ([`overlay_engine::RosterIndex::account_mates`]) —
//! le titulaire seul quand il n'y a pas de roster.
//!
//! ## Apprendre le gabarit sans rien demander à l'utilisateur
//!
//! Le log dit quand P lance un sort (`FighterDamage::last_turn_casts` grandit). Un sort se lance
//! pendant son propre tour, depuis la fenêtre qui porte P : à cet instant, le widget de cette
//! fenêtre affiche P — **si** il est au repos (panneau doré, voir [`vision::find_gold_panel`]) et
//! non sur la carte d'un combattant survolé. Le module ouvre donc une courte fenêtre
//! d'apprentissage à chaque sort, ne prend un échantillon qu'au repos, et n'acquiert le gabarit
//! qu'avec **deux échantillons concordants pris à deux sorts différents** : un tour qui se termine
//! pile après le sort (le nom bascule vers le suivant) ne peut pas contaminer les deux.
//!
//! Tant que le gabarit manque, rien n'est notifié pour P — l'apprentissage se fait au fil du
//! premier combat, et vaut pour tous les suivants (persisté par l'appelant).
//!
//! **Et il se corrige tout seul** : si, à un sort de P au repos, le nom affiché ne ressemble pas
//! au gabarit connu, c'est le gabarit qui a tort (échelle d'interface changée, géométrie corrigée
//! par une mise à jour — vécu le 2026-09-14, un gabarit lu deux pixels trop court et plus jamais
//! reconnu). Le même protocole à deux sorts concordants le remplace alors.
//!
//! ## Reconnaître, puis décider
//!
//! Gabarits acquis, chaque tick compare le nom affiché à ceux des personnages de la fenêtre : le
//! mieux ressemblant, au-dessus de [`MATCH_THRESHOLD`], est l'actif. C'est le **changement
//! d'actif** vers un personnage (il ne l'était pas, il l'est) qui vaut « c'est à lui de jouer » —
//! notifié seulement si la fenêtre n'est pas au premier plan, au plus une fois par
//! [`NOTIFY_COOLDOWN`] et par personnage.
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

/// Ce que le moteur sait du combat de la fenêtre, au moment du tick.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FightFacts {
    pub ongoing: bool,
    /// Le log a vu au moins un sort ou des dégâts dans ce combat : la phase de placement est
    /// passée, quoi que montre l'écran.
    pub engaged_by_log: bool,
    /// `FighterDamage::last_turn_casts.len()` de chaque personnage surveillé, par nom tel que
    /// donné dans `TickInput::characters` (absent = 0). Toute variation vers une valeur non nulle
    /// signale un sort qui vient d'être lancé.
    pub cast_lens: HashMap<String, usize>,
}

/// Ce que l'appelant fournit pour une fenêtre, à chaque tick.
pub struct TickInput<'a> {
    /// Le titulaire de la fenêtre — la clé de son état.
    pub window: &'a str,
    /// Les personnages qui peuvent y jouer : le titulaire et ses héros (même compte au roster).
    /// Toujours au moins le titulaire.
    pub characters: &'a [String],
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
    /// Nombre de sorts vus — identifie l'échantillon d'apprentissage à un sort.
    spell_seq: u64,
    learn_until: Option<Instant>,
    candidate: Option<(Glyph, u64)>,
}

#[derive(Debug, Default)]
struct WindowState {
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
    /// le roster ou le titre de fenêtre le donne.
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

        let Some(fight) = input.fight.filter(|f| f.ongoing) else {
            // Hors combat : tout repart de zéro au prochain, gabarit et géométrie exceptés.
            *state = WindowState::default();
            return events;
        };

        // Un sort d'un personnage de la fenêtre ouvre sa fenêtre d'apprentissage. La longueur peut
        // aussi *baisser* (nouveau tour, liste vidée puis premier sort) : c'est un sort aussi.
        for name in input.characters {
            let len = fight.cast_lens.get(name).copied().unwrap_or(0);
            let learning = state.learning.entry(name.clone()).or_default();
            if len != learning.prev_cast_len && len > 0 {
                learning.spell_seq += 1;
                learning.learn_until = Some(input.now + LEARN_WINDOW);
            }
            learning.prev_cast_len = len;
        }

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
            tracing::info!("[tour] {} : combat engagé, placement terminé", input.window);
        }
        let area = match self.geometry_by_size.get(&size) {
            Some(area) => *area,
            None => match panel {
                Some(p) => {
                    let area = vision::name_area_above(p, band);
                    tracing::info!(
                        "[tour] {} : bande du nom localisée ({}x{} → {:?})",
                        input.window,
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
                state.active = None;
            }
            return events;
        };

        // Apprentissage — au repos seulement, dans la fenêtre ouverte par un sort du personnage.
        // Aussi quand un gabarit existe mais ne reconnaît pas ce que le jeu affiche au moment où
        // le personnage joue : il est obsolète, et se remplace par le même protocole.
        if panel.is_some() {
            for name in input.characters {
                let Some(learning) = state.learning.get_mut(name) else {
                    continue;
                };
                if !learning.learn_until.is_some_and(|until| input.now < until) {
                    continue;
                }
                let disagrees = self
                    .templates
                    .get(name)
                    .is_some_and(|t| vision::similarity(t, &glyph) < MATCH_THRESHOLD);
                if self.templates.contains_key(name) && !disagrees {
                    continue;
                }
                match &learning.candidate {
                    Some((first, seq)) if *seq != learning.spell_seq => {
                        if vision::similarity(first, &glyph) >= LEARN_THRESHOLD {
                            tracing::info!(
                                "[tour] {name} : gabarit du nom {} ({}x{}, fenêtre {})",
                                if disagrees {
                                    "remplacé — l'ancien ne reconnaissait plus"
                                } else {
                                    "acquis"
                                },
                                glyph.w,
                                glyph.h,
                                input.window
                            );
                            self.templates.insert(name.clone(), glyph.clone());
                            events.push(Event::TemplateLearned {
                                character: name.clone(),
                                glyph: glyph.clone(),
                            });
                            learning.candidate = None;
                        } else {
                            // Deux sorts, deux noms : l'un des deux était un survol ou un
                            // changement de tour. On repart du plus récent.
                            learning.candidate = Some((glyph.clone(), learning.spell_seq));
                        }
                    }
                    Some(_) => {} // même sort que le premier échantillon : on attend le suivant
                    None => learning.candidate = Some((glyph.clone(), learning.spell_seq)),
                }
            }
        }

        // Reconnaissance : le personnage de la fenêtre dont le gabarit ressemble le plus.
        let mut best: Option<(&str, f64)> = None;
        for name in input.characters {
            let Some(template) = self.templates.get(name) else {
                continue;
            };
            let score = vision::similarity(template, &glyph);
            tracing::trace!(
                "[tour] {} / {name} : ressemblance {score:.2} ({}x{})",
                input.window,
                glyph.w,
                glyph.h
            );
            if score >= MATCH_THRESHOLD && best.is_none_or(|(_, s)| score > s) {
                best = Some((name.as_str(), score));
            }
        }

        match best {
            Some((name, _)) => {
                if state.active.as_deref() != Some(name) {
                    let cooled = state
                        .last_notified
                        .get(name)
                        .is_none_or(|t| input.now.duration_since(*t) >= NOTIFY_COOLDOWN);
                    if !state.engaged {
                        tracing::debug!(
                            "[tour] {name} : tour reconnu, pas de notification (phase de placement)"
                        );
                    } else if !input.foreground && cooled {
                        state.last_notified.insert(name.to_string(), input.now);
                        events.push(Event::Notify {
                            character: name.to_string(),
                        });
                    } else {
                        tracing::debug!(
                            "[tour] {name} : tour reconnu, pas de notification ({})",
                            if input.foreground {
                                "fenêtre au premier plan"
                            } else {
                                "trop tôt après la précédente"
                            }
                        );
                    }
                }
                state.active = Some(name.to_string());
                state.misses = 0;
            }
            None => {
                // Front descendant retardé : `INACTIVE_TICKS` ticks sans reconnaissance.
                state.misses += 1;
                if state.misses >= INACTIVE_TICKS {
                    state.active = None;
                }
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

    fn facts(engaged: bool, casts: &[(&str, usize)]) -> FightFacts {
        FightFacts {
            ongoing: true,
            engaged_by_log: engaged,
            cast_lens: casts.iter().map(|(n, c)| (n.to_string(), *c)).collect(),
        }
    }

    fn one(name: &str) -> Vec<String> {
        vec![name.to_string()]
    }

    fn tick<'a>(
        w: &mut Watcher,
        window: &'a str,
        characters: &'a [String],
        band: Option<&'a Band>,
        fight: &'a FightFacts,
        foreground: bool,
        now: Instant,
    ) -> Vec<Event> {
        w.tick(TickInput {
            window,
            characters,
            band,
            fight: Some(fight),
            foreground,
            now,
        })
    }

    /// Joue les ticks qui font acquérir le gabarit d'« Oumbra » : deux sorts, la bande au repos.
    fn learn_oumbra(w: &mut Watcher, t0: Instant) -> Vec<Event> {
        let repos = fixture("repos-oumbra");
        let chars = one("Oumbra");
        let mut all = Vec::new();
        let f1 = facts(true, &[("Oumbra", 1)]);
        all.extend(tick(w, "Oumbra", &chars, Some(&repos), &f1, true, t0));
        all.extend(tick(
            w,
            "Oumbra",
            &chars,
            Some(&repos),
            &f1,
            true,
            t0 + Duration::from_millis(500),
        ));
        let f2 = facts(true, &[("Oumbra", 2)]);
        all.extend(tick(
            w,
            "Oumbra",
            &chars,
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
        let chars = one("Oumbra");
        let f0 = facts(true, &[]);
        for i in 0..4 {
            let ev = tick(
                &mut w,
                "Oumbra",
                &chars,
                Some(&repos),
                &f0,
                true,
                t0 + Duration::from_millis(500 * i),
            );
            assert!(ev.is_empty());
        }
        for i in 0..4u64 {
            let f = facts(true, &[("Oumbra", i as usize + 1)]);
            let ev = tick(
                &mut w,
                "Oumbra",
                &chars,
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
        let chars = one("Oumbra");
        let f = facts(true, &[("Oumbra", 2)]);
        // Déjà actif → rien.
        let ev = tick(
            &mut w,
            "Oumbra",
            &chars,
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
                &chars,
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
            &chars,
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
            &chars,
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
        let chars = one("Oumbra");
        let f = facts(true, &[("Oumbra", 2)]);
        for i in 0..2 {
            tick(
                &mut w,
                "Oumbra",
                &chars,
                Some(&autre),
                &f,
                true,
                t0 + Duration::from_secs(10 + i),
            );
        }
        let ev = tick(
            &mut w,
            "Oumbra",
            &chars,
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
        let chars = one("Oumbra");
        let f = facts(true, &[("Oumbra", 2)]);
        for i in 0..2 {
            tick(
                &mut w,
                "Oumbra",
                &chars,
                Some(&autre),
                &f,
                false,
                t0 + Duration::from_secs(10 + i),
            );
        }
        let ev = tick(
            &mut w,
            "Oumbra",
            &chars,
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
    fn la_fin_du_combat_remet_l_etat_a_zero_mais_garde_le_gabarit() {
        let mut w = Watcher::default();
        let t0 = Instant::now();
        learn_oumbra(&mut w, t0);
        let ended = FightFacts::default();
        let ev = tick(
            &mut w,
            "Oumbra",
            &one("Oumbra"),
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
        let chars = one("Pugio Letalis");
        let placement = facts(false, &[]);
        for i in 0..4 {
            let ev = tick(
                &mut w,
                "Pugio Letalis",
                &chars,
                Some(&pret),
                &placement,
                false,
                t0 + Duration::from_secs(i),
            );
            assert!(ev.is_empty(), "placement : {ev:?}");
        }
        // Le log voit un sort : engagé. Déjà reconnu, pas de front : rien de rétroactif.
        let engaged = facts(true, &[]);
        let ev = tick(
            &mut w,
            "Pugio Letalis",
            &chars,
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
                &chars,
                Some(&autre),
                &engaged,
                false,
                t0 + Duration::from_secs(10 + i),
            );
        }
        let ev = tick(
            &mut w,
            "Pugio Letalis",
            &chars,
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
    fn un_heros_est_appris_et_notifie_depuis_la_fenetre_de_son_compte() {
        // Fenêtre « Oumbra », compte à deux personnages : Oumbra et Pugio Letalis (héros). Pugio
        // lance deux sorts pendant que la fenêtre montre son nom sous un panneau doré : son gabarit
        // s'apprend depuis cette fenêtre. Ensuite, quand son tour revient et que la fenêtre est en
        // arrière-plan, c'est LUI qui est notifié.
        let mut w = Watcher::default();
        let t0 = Instant::now();
        learn_oumbra(&mut w, t0);
        let chars = vec!["Oumbra".to_string(), "Pugio Letalis".to_string()];
        let pret = fixture("repos-pugio-t18"); // le widget affiche « Pugio Letalis »
        let repos = fixture("repos-oumbra");
        let f1 = facts(true, &[("Oumbra", 2), ("Pugio Letalis", 1)]);
        tick(
            &mut w,
            "Oumbra",
            &chars,
            Some(&pret),
            &f1,
            true,
            t0 + Duration::from_secs(20),
        );
        let f2 = facts(true, &[("Oumbra", 2), ("Pugio Letalis", 2)]);
        let ev = tick(
            &mut w,
            "Oumbra",
            &chars,
            Some(&pret),
            &f2,
            true,
            t0 + Duration::from_secs(23),
        );
        assert!(
            matches!(ev.as_slice(), [Event::TemplateLearned { character, .. }] if character == "Pugio Letalis"),
            "{ev:?}"
        );
        // Oumbra joue (fenêtre au premier plan), puis le tour revient à Pugio, fenêtre en
        // arrière-plan.
        for i in 0..2 {
            tick(
                &mut w,
                "Oumbra",
                &chars,
                Some(&repos),
                &f2,
                true,
                t0 + Duration::from_secs(30 + i),
            );
        }
        let ev = tick(
            &mut w,
            "Oumbra",
            &chars,
            Some(&pret),
            &f2,
            false,
            t0 + Duration::from_secs(60),
        );
        assert_eq!(
            ev,
            vec![Event::Notify {
                character: "Pugio Letalis".to_string()
            }]
        );
    }
}
