//! `LogParser` vendu (voir `engine-js/`), embarqué en QuickJS (`rquickjs`) — validé par le spike
//! S2 (`spikes/s2-engine-quickjs/`, ~28 000 lignes/s sur un vrai `wakfu.log`).
//!
//! Volontairement mono-thread et non `Send` (QuickJS n'est pas thread-safe) : c'est cohérent avec
//! le modèle d'exécution du plan (§3), où l'Engine possède un thread dédié rien qu'à lui.

use std::sync::Arc;

use arc_swap::ArcSwap;
use rquickjs::{Context, Function, Object, Runtime};

use crate::catalog::CatalogIndex;
use crate::model::LogEntry;

const ENGINE_BUNDLE: &str = include_str!("../engine-js/dist/engine.bundle.js");

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("erreur QuickJS : {0}")]
    QuickJs(#[from] rquickjs::Error),
    /// Échec de désérialisation d'un lot analysé par le parser vendu.
    ///
    /// **Ni le JSON reçu ni le message brut de `serde_json` ne sont rendus par `Display`**
    /// (constat C6 de `docs/analyse-rgpd.md`) : le lot contient les entrées `Chat { author,
    /// message }` du moment, et le message de `serde_json` cite volontiers la valeur fautive
    /// (« invalid type: string "…" »). Recopier l'un ou l'autre dans un journal conservé 14 jours
    /// contredisait la promesse « le contenu du chat n'est ni transmis ni écrit sur disque » — et
    /// ce chemin-là est précisément celui qu'emprunte un incident, donc le plus susceptible d'être
    /// joint à un rapport de bug.
    ///
    /// Ce qu'il reste, [`DeserializeContext`], suffit à situer et à rejouer le défaut : nature de
    /// l'erreur, position, taille du lot, et les valeurs de `kind` qu'il portait. Le détail
    /// complet reste accessible à l'appelant par [`std::error::Error::source`], à ne journaliser
    /// qu'en `debug` (réglage « Journal détaillé »).
    #[error("désérialisation d'un LogEntry a échoué : {context}")]
    Deserialize {
        source: serde_json::Error,
        context: DeserializeContext,
    },
}

/// Ce qu'on garde d'un lot que `serde` a refusé : sa FORME, jamais son contenu — voir
/// [`EngineError::Deserialize`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeserializeContext {
    /// `data` (JSON valide mais forme inattendue), `syntax`, `eof` ou `io`.
    pub classification: &'static str,
    /// Position de l'erreur dans le JSON produit par le parser vendu — 1-based, `serde_json`.
    pub line: usize,
    pub column: usize,
    /// Taille du lot refusé, en octets : distingue une entrée isolée d'un rattrapage entier.
    pub bytes: usize,
    /// Les valeurs du champ discriminant `kind` portées par le lot, dédupliquées et ordonnées —
    /// vide si le JSON n'est même pas analysable en `serde_json::Value`. Ce sont des noms de
    /// VARIANTES (`chat`, `fighter-joined`…), pas des données de joueur.
    pub kinds: Vec<String>,
}

impl std::fmt::Display for DeserializeContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} à la ligne {} colonne {}, lot de {} octet(s)",
            self.classification, self.line, self.column, self.bytes
        )?;
        if self.kinds.is_empty() {
            f.write_str(", aucun `kind` lisible")
        } else {
            write!(f, ", kind(s) : {}", self.kinds.join(", "))
        }
    }
}

impl DeserializeContext {
    fn new(json: &str, source: &serde_json::Error) -> Self {
        use serde_json::error::Category;
        Self {
            classification: match source.classify() {
                Category::Io => "erreur d'entrée/sortie",
                Category::Syntax => "JSON mal formé",
                Category::Data => "forme inattendue",
                Category::Eof => "JSON tronqué",
            },
            line: source.line(),
            column: source.column(),
            bytes: json.len(),
            kinds: kinds_of(json),
        }
    }
}

/// Relit le lot uniquement pour en extraire les valeurs de `kind` — le seul champ dont on sait
/// qu'il ne porte aucune donnée de joueur (c'est le tag `#[serde(tag = "kind")]` de
/// [`LogEntry`], donc un ensemble fermé de noms de variantes). Tout le reste est laissé de côté.
fn kinds_of(json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let entries: Vec<&serde_json::Value> = match &value {
        serde_json::Value::Array(items) => items.iter().collect(),
        other => vec![other],
    };
    let mut kinds: Vec<String> = entries
        .iter()
        .filter_map(|entry| entry.get("kind"))
        .filter_map(serde_json::Value::as_str)
        // Une valeur de `kind` inattendue reste un nom de variante côté parser vendu, mais rien
        // ne le garantit si le JSON est corrompu : bornée, pour ne pas recopier une ligne entière.
        .map(|kind| kind.chars().take(32).collect::<String>())
        .collect();
    kinds.sort_unstable();
    kinds.dedup();
    kinds
}

fn deserialize<T: serde::de::DeserializeOwned>(json: &str) -> Result<T, EngineError> {
    serde_json::from_str(json).map_err(|source| EngineError::Deserialize {
        context: DeserializeContext::new(json, &source),
        source,
    })
}

/// Enveloppe QuickJS + `LogParser` vendu. Ne connaît **que** le parsing ligne à ligne — la
/// sémantique `is_initial_load` / reset (§5.3) est gérée par [`crate::session::Engine`], qui
/// enveloppe celui-ci ; voir sa documentation pour l'explication complète.
pub struct LogParserEngine {
    // Jamais relu directement après la construction : sa seule raison d'être est de garder le
    // runtime QuickJS vivant tant que `context` existe (le contexte emprunte le runtime).
    _runtime: Runtime,
    context: Context,
    /// Catalogue consulté par `hostIsKnownMonsterName` (voir `new`) — `Arc<ArcSwap<_>>` plutôt
    /// qu'une copie figée à la construction : `set_catalog` le met à jour EN PLACE (appelé par
    /// `crate::session::Engine::set_catalog`, lui-même relayé par l'hôte au fil du chargement du
    /// catalogue réseau/disque, voir `overlay-ui`), et la fermeture JS ci-dessous lit la version
    /// courante à CHAQUE appel — jamais besoin de re-enregistrer la fonction JS après un
    /// rafraîchissement.
    catalog: Arc<ArcSwap<CatalogIndex>>,
}

impl LogParserEngine {
    pub fn new() -> Result<Self, EngineError> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;
        let catalog = Arc::new(ArcSwap::from_pointee(CatalogIndex::default()));

        context.with(|ctx| -> Result<(), rquickjs::Error> {
            // Posé sur les globals AVANT d'évaluer le bundle : `entry.ts` le lit dès l'évaluation
            // du module (construction du singleton `LogParser`, voir sa doc) — l'ordre inverse
            // laisserait `globalThis.hostIsKnownMonsterName` indéfini pour ce premier appel.
            //
            // Miroir de `StatsStoreService` (`isKnownMonsterName: (name) =>
            // this.catalog.isKnownWakfuMonsterName(name)`) : seule façon de protéger un vrai
            // monstre qui se révèle en cours de combat (mimique, brèche) contre le repli
            // "invocation sans annonce" du parser vendu — sans ce garde-fou, N'IMPORTE QUEL
            // combattant qui rejoint dans la fenêtre `SUMMON_JOIN_WINDOW_MS` suivant une annonce
            // d'invocation SANS RAPPORT se voit à tort attribuer `summonedBy`, voir
            // `session.rs::upsert_fighter`. Retour utilisateur 2026-09-02 (capture d'écran à
            // l'appui) : une invocation de mécanisme apparaissait à tort côté ennemis, faute de
            // suivre `summonedBy` — corrigé côté Rust par `upsert_fighter`, mais qui dépend de ce
            // garde-fou pour ne jamais avaler à tort un vrai monstre.
            let catalog_for_closure = Arc::clone(&catalog);
            let is_known_monster_name = Function::new(ctx.clone(), move |name: String| -> bool {
                catalog_for_closure
                    .load()
                    .find_monster_icon(&name, None)
                    .is_some()
            })?;
            ctx.globals()
                .set("hostIsKnownMonsterName", is_known_monster_name)?;
            ctx.eval::<(), _>(ENGINE_BUNDLE)
        })?;

        Ok(Self {
            _runtime: runtime,
            context,
            catalog,
        })
    }

    /// Remplace le catalogue consulté par `hostIsKnownMonsterName` — appelé par
    /// `crate::session::Engine::set_catalog`, lui-même relayé par l'hôte (`overlay-ui`) à chaque
    /// (re)chargement du catalogue (cache disque puis réseau, voir `overlay-sync::catalog_cache`).
    /// Prend directement l'`Arc` déjà détenu par l'hôte (son propre `Arc<ArcSwap<CatalogIndex>>`,
    /// voir `main.rs`) plutôt qu'une valeur possédée : `CatalogIndex` n'a aucune raison d'être
    /// `Clone` (~11 700 entrées), un simple partage de référence suffit ici comme partout ailleurs
    /// dans l'overlay où ce catalogue circule. Un catalogue vide (`CatalogIndex::default`, valeur
    /// initiale de `new` avant tout appel) ne fait échouer aucun combat en cours : le garde-fou
    /// refuse simplement toute protection tant qu'il n'a rien à consulter, comportement historique
    /// du parser vendu (voir sa doc).
    pub fn set_catalog(&self, catalog: Arc<CatalogIndex>) {
        self.catalog.store(catalog);
    }

    /// Analyse un lot de lignes déjà lues (voir `overlay_ingest::LineBatch::lines`) et vide dans
    /// la foulée l'éventuel enregistrement multi-lignes en attente (`entry.ts::flush`, voir sa
    /// doc : c'est l'usage prévu, pas seulement une fin de fichier — sans effet de bord si rien
    /// n'était en attente).
    pub fn parse_lines(&self, lines: &[String]) -> Result<Vec<LogEntry>, EngineError> {
        self.context.with(|ctx| {
            let engine: Object = ctx.globals().get("wakfuEngine")?;

            let parse_batch: Function = engine.get("parseBatch")?;
            let joined = lines.join("\n");
            let batch_json: String = parse_batch.call((joined,))?;
            let mut entries: Vec<LogEntry> = deserialize(&batch_json)?;

            let flush: Function = engine.get("flush")?;
            let flushed_json: String = flush.call(())?;
            if !flushed_json.is_empty() {
                entries.push(deserialize(&flushed_json)?);
            }

            Ok(entries)
        })
    }

    /// Réinitialise tout l'état interne du parser — à appeler au début d'une nouvelle « session »
    /// de lecture (reconnexion ou rotation, jamais au milieu d'un rattrapage qui continuerait sur
    /// plusieurs lots, voir [`crate::session::Engine`]).
    pub fn reset(&self) -> Result<(), EngineError> {
        self.context.with(|ctx| {
            let engine: Object = ctx.globals().get("wakfuEngine")?;
            let reset: Function = engine.get("resetParser")?;
            reset.call::<(), ()>(())
        })?;
        Ok(())
    }

    /// Fait oublier au parser un combat qui n'aura jamais son `[FIGHT] End fight` (client fermé en
    /// plein combat) — voir `LogParser.closeFight` et [`crate::session::Engine::ingest_batch`], qui
    /// l'appelle pour chaque combat qu'il suspend. Sans cela, le parser continue de désigner ce
    /// combat fantôme comme seul combat actif et lui rattache toute ligne sans nom (butin, gain de
    /// kamas hors combat) — c'est le bug que le web a corrigé de la même façon, depuis son store.
    pub fn close_fight(&self, fight_id: i64) -> Result<(), EngineError> {
        self.context.with(|ctx| {
            let engine: Object = ctx.globals().get("wakfuEngine")?;
            let close: Function = engine.get("closeFight")?;
            close.call::<(i64,), ()>((fight_id,))
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LogEntry;

    /// Garde-fou du constat C6 : ce que l'erreur de désérialisation rend lisible ne doit JAMAIS
    /// contenir le lot — un message de chat s'y retrouverait, et de là dans le journal de session.
    #[test]
    fn l_erreur_de_deserialisation_ne_recopie_pas_le_lot() {
        // Forme volontairement fautive (`damage` attendu en nombre) sur une entrée de chat :
        // c'est exactement le cas que le constat décrit.
        let lot = r#"[{"kind":"chat","time":"10:00:00","channel":"commerce","author":"Oumbra","message":"WTS mon secret"},{"kind":"fight-start","fightId":"pas un nombre"}]"#;
        let err =
            deserialize::<Vec<LogEntry>>(lot).expect_err("ce lot ne doit pas se désérialiser");
        let rendu = err.to_string();
        for interdit in ["WTS mon secret", "Oumbra", "pas un nombre"] {
            assert!(
                !rendu.contains(interdit),
                "l'erreur rendue ne doit pas contenir {interdit:?} : {rendu}"
            );
        }
        // …tout en restant diagnostiquable : nature, position, taille et variantes du lot.
        let EngineError::Deserialize { context, .. } = &err else {
            panic!("variante inattendue : {err}");
        };
        assert_eq!(context.bytes, lot.len());
        assert_eq!(context.kinds, vec!["chat", "fight-start"]);
        assert!(
            rendu.contains("chat"),
            "les `kind` situent le lot : {rendu}"
        );
    }

    /// Un lot syntaxiquement cassé ne livre aucun `kind` — et surtout aucun fragment de ligne.
    #[test]
    fn l_erreur_sur_un_json_casse_reste_muette() {
        let lot = r#"[{"kind":"chat","author":"Oumbra","message":"WTS mon secr"#;
        let err = deserialize::<Vec<LogEntry>>(lot).expect_err("JSON tronqué");
        let rendu = err.to_string();
        assert!(!rendu.contains("Oumbra"), "{rendu}");
        assert!(!rendu.contains("WTS"), "{rendu}");
        let EngineError::Deserialize { context, .. } = &err else {
            panic!("variante inattendue : {err}");
        };
        assert!(context.kinds.is_empty(), "{:?}", context.kinds);
    }

    /// Lignes réelles (format vendu, mêmes textes que `log-parser.spec.ts` côté web — voir son test
    /// "identifie une invocation SANS aucune annonce 'Invoque' détectable") : "SorHon" fait tomber
    /// des "Rocher" au sol via `INVOCATION_INSTANTIATED_RE`, SANS annonce "X: Invoque ..." — le seul
    /// repli qu'`hostIsKnownMonsterName` doit pouvoir bloquer si "Rocher" est un vrai monstre
    /// catalogué (voir `LogParserEngine::new`).
    fn lignes_sorhon_rocher() -> Vec<String> {
        [
            " INFO 10:00:00,000 [T] (a:1) - [_FL_] fightId=1 Oumbra breed : 4 [1] isControlledByAI=false obstacleId : -1 join the fight at {P}",
            " INFO 10:00:00,001 [T] (a:1) - [_FL_] fightId=1 SorHon breed : 10 [-1] isControlledByAI=true obstacleId : -1 join the fight at {P}",
            " INFO 10:00:01,000 [T] (a:1) - [Information (combat)] SorHon lance le sort Effondrement",
            " INFO 10:00:01,090 [T] (eXG:105) - Instanciation d'une nouvelle invocation avec un id de 2",
            " INFO 10:00:01,091 [T] (a:1) - [_FL_] fightId=1 Rocher breed : 5875 [2] isControlledByAI=true obstacleId : 8 join the fight at {P}",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    fn rocher_summoned_by(entries: &[LogEntry]) -> Option<String> {
        for entry in entries {
            if let LogEntry::FighterJoined {
                name, summoned_by, ..
            } = entry
            {
                if name == "Rocher" {
                    return summoned_by.clone();
                }
            }
        }
        None
    }

    /// Lignes d'un combat à deux Grokoko et un allié du joueur, avec des relancers du même sort —
    /// les horodatages sont ceux du test, les textes ceux du log réel.
    fn lignes_relancers(delta_ms: u32) -> Vec<String> {
        let t = |ms: u32| format!("10:00:{:02},{:03}", ms / 1000, ms % 1000);
        [
            format!(" INFO {} [T] (a:1) - [_FL_] fightId=1 Oumbra breed : 15 [1] isControlledByAI=false obstacleId : -1 join the fight at {{P}}", t(0)),
            format!(" INFO {} [T] (a:1) - [_FL_] fightId=1 Grokoko breed : 4728 [-2] isControlledByAI=true obstacleId : -1 join the fight at {{P}}", t(1)),
            format!(" INFO {} [T] (a:1) - [Information (combat)] Oumbra lance le sort Croc-en-jambe", t(1000)),
            format!(" INFO {} [T] (a:1) - [Information (combat)] Grokoko: -105 PV (Terre)", t(1500)),
            format!(" INFO {} [T] (a:1) - [Information (combat)] Oumbra lance le sort Croc-en-jambe", t(1000 + delta_ms)),
            format!(" INFO {} [T] (a:1) - [Information (combat)] 61 secondes reportées pour le tour suivant.", t(5000)),
        ]
        .into_iter()
        .collect()
    }

    fn casts(entries: &[LogEntry]) -> usize {
        entries
            .iter()
            .filter(|e| matches!(e, LogEntry::SpellCast { .. }))
            .count()
    }

    /// Modification locale du parseur vendu (voir `SPELL_CAST_DEDUPE_WINDOW_MS` dans
    /// `log-parser.ts`) : un relancer réel du même sort à 724 ms (le plus rapide observé sur le log
    /// de parité) n'est plus avalé comme doublon multi-compte ; une copie d'un second client à
    /// 452 ms (la plus tardive observée) l'est toujours.
    #[test]
    fn un_relancer_rapide_du_meme_sort_nest_plus_deduplique() {
        let engine = LogParserEngine::new().expect("moteur QuickJS");
        let entries = engine.parse_lines(&lignes_relancers(724)).expect("parsing");
        assert_eq!(casts(&entries), 2, "relancer réel à 724 ms");

        engine.reset().expect("reset");
        let entries = engine.parse_lines(&lignes_relancers(452)).expect("parsing");
        assert_eq!(casts(&entries), 1, "copie multi-compte à 452 ms");
    }

    /// Ajout local : « N secondes reportées pour le tour suivant. » devient `TurnEnded`, rattaché
    /// au combat courant, avec les secondes reportées.
    #[test]
    fn la_fin_de_tour_du_joueur_est_un_evenement_rattache_au_combat() {
        let engine = LogParserEngine::new().expect("moteur QuickJS");
        let entries = engine.parse_lines(&lignes_relancers(724)).expect("parsing");
        let turn_ended = entries
            .iter()
            .find(|e| matches!(e, LogEntry::TurnEnded { .. }))
            .expect("un TurnEnded");
        assert_eq!(
            *turn_ended,
            LogEntry::TurnEnded {
                time: "10:00:05,000".to_string(),
                carried_seconds: 61,
                fight_id: Some(1),
            }
        );
    }

    /// Deux combats concurrents (multi-compte) contre des monstres du même nom : le combat 2
    /// démarre — rafale de jointures — pendant que le combat 1 est en cours. Un dégât du combat 1
    /// sur une cible ambiguë (« Grokoko » est dans les deux) doit rester dans le combat 1, crédité
    /// à son lanceur, et non partir dans le combat 2 où personne n'a encore agi — ce qui y créait
    /// un attaquant « Inconnu » (bug réel du 2026-09-22, voir `mostRecentlyActiveFight`).
    fn lignes_deux_combats_concurrents() -> Vec<String> {
        [
            " INFO 10:00:00,000 [T] (a:1) - [_FL_] fightId=1 Zoroark breed : 15 [1] isControlledByAI=false obstacleId : -1 join the fight at {P}",
            " INFO 10:00:00,001 [T] (a:1) - [_FL_] fightId=1 Grokoko breed : 4728 [-2] isControlledByAI=true obstacleId : -1 join the fight at {P}",
            " INFO 10:00:01,000 [T] (a:1) - [Information (combat)] Zoroark lance le sort Croc-en-jambe",
            " INFO 10:00:01,050 [T] (a:1) - [_FL_] fightId=2 Canis breed : 4 [3] isControlledByAI=false obstacleId : -1 join the fight at {P}",
            " INFO 10:00:01,051 [T] (a:1) - [_FL_] fightId=2 Grokoko breed : 4728 [-4] isControlledByAI=true obstacleId : -1 join the fight at {P}",
            " INFO 10:00:01,100 [T] (a:1) - [Information (combat)] Grokoko: -105 PV (Terre)",
            " INFO 10:00:02,000 [T] (a:1) - [Information (combat)] Canis lance le sort Morsure",
            " INFO 10:00:02,100 [T] (a:1) - [Information (combat)] Grokoko: -77 PV (Feu)",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    #[test]
    fn un_degat_sur_cible_ambigue_reste_dans_le_combat_ou_quelquun_a_agi() {
        let engine = LogParserEngine::new().expect("moteur QuickJS");
        let entries = engine
            .parse_lines(&lignes_deux_combats_concurrents())
            .expect("parsing");
        let damages: Vec<(Option<i64>, String, i64)> = entries
            .iter()
            .filter_map(|e| match e {
                LogEntry::Damage {
                    fight_id,
                    attacker,
                    amount,
                    ..
                } => Some((*fight_id, attacker.clone(), *amount)),
                _ => None,
            })
            .collect();
        assert_eq!(
            damages,
            vec![
                (Some(1), "Zoroark".to_string(), 105),
                (Some(2), "Canis".to_string(), 77),
            ]
        );
    }

    #[test]
    fn sans_catalogue_le_repli_dinvocation_avale_nimporte_quel_nouveau_venu() {
        let engine = LogParserEngine::new().expect("moteur QuickJS");
        let entries = engine
            .parse_lines(&lignes_sorhon_rocher())
            .expect("parsing");
        assert_eq!(
            rocher_summoned_by(&entries),
            Some("SorHon".to_string()),
            "comportement historique inchangé sans `set_catalog` (repli 'fallback' jamais protégé)"
        );
    }

    #[test]
    fn un_monstre_catalogue_nest_jamais_avale_par_le_repli_dinvocation() {
        let engine = LogParserEngine::new().expect("moteur QuickJS");
        engine.set_catalog(Arc::new(CatalogIndex::from_compact_json(
            &serde_json::json!({
                "monsters": [[1, "Rocher", "Rock", "Roca", "Rocha", "999", -1, 0, 0, 0]],
            }),
        )));
        let entries = engine
            .parse_lines(&lignes_sorhon_rocher())
            .expect("parsing");
        assert_eq!(
            rocher_summoned_by(&entries),
            None,
            "un vrai monstre catalogué (mimique/brèche qui se révèle) ne doit jamais être pris \
             pour une invocation, même via le repli 'fallback' sans annonce 'Invoque'"
        );
    }
}
