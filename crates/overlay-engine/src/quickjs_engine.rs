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
    #[error("désérialisation d'un LogEntry a échoué : {source} — JSON reçu : {json}")]
    Deserialize {
        source: serde_json::Error,
        json: String,
    },
}

fn deserialize<T: serde::de::DeserializeOwned>(json: &str) -> Result<T, EngineError> {
    serde_json::from_str(json).map_err(|source| EngineError::Deserialize {
        source,
        json: json.to_string(),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LogEntry;

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
