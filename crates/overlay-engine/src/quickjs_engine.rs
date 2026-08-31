//! `LogParser` vendu (voir `engine-js/`), embarqué en QuickJS (`rquickjs`) — validé par le spike
//! S2 (`spikes/s2-engine-quickjs/`, ~28 000 lignes/s sur un vrai `wakfu.log`).
//!
//! Volontairement mono-thread et non `Send` (QuickJS n'est pas thread-safe) : c'est cohérent avec
//! le modèle d'exécution du plan (§3), où l'Engine possède un thread dédié rien qu'à lui.

use rquickjs::{Context, Function, Object, Runtime};

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
}

impl LogParserEngine {
    pub fn new() -> Result<Self, EngineError> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;
        context.with(|ctx| ctx.eval::<(), _>(ENGINE_BUNDLE))?;
        Ok(Self {
            _runtime: runtime,
            context,
        })
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
