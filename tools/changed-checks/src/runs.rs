//! What each run took, kept as a line of JSON in `changed-checks/runs.jsonl`
//! under XDG's state folder, so the time from a change to its proof is
//! measured run after run rather than remembered: each side's evaluation
//! chunk by chunk, what the comparison named, and the build.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

/// One run, filled in as it goes.
pub struct Run {
    began: Instant,
    at: u64,
    base: String,
    system: String,
    evaluations: Vec<Value>,
    named: Option<usize>,
    built: Option<Value>,
}

/// One side's evaluation, chunk by chunk, as it goes.
pub struct Evaluation {
    of: &'static str,
    began: Instant,
    chunks: Vec<Value>,
}

impl Evaluation {
    /// Note that a chunk of this many checks took this long.
    pub fn chunk(&mut self, checks: usize, took: Duration) {
        self.chunks
            .push(json!({ "checks": checks, "seconds": seconds(took) }));
    }
}

impl Run {
    pub fn new(base: &str, system: &str) -> Self {
        Self {
            began: Instant::now(),
            at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_secs()),
            base: base.to_owned(),
            system: system.to_owned(),
            evaluations: Vec::new(),
            named: None,
            built: None,
        }
    }

    /// Begin evaluating a side, "base" or "tree".
    pub fn evaluating(of: &'static str) -> Evaluation {
        Evaluation {
            of,
            began: Instant::now(),
            chunks: Vec::new(),
        }
    }

    /// Note a side's evaluation, once it's done.
    pub fn evaluated(&mut self, evaluation: Evaluation) {
        self.evaluations.push(json!({
            "of": evaluation.of,
            "seconds": seconds(evaluation.began.elapsed()),
            "chunks": Value::Array(evaluation.chunks),
        }));
    }

    /// Note how many checks the comparison named.
    pub fn named(&mut self, checks: usize) {
        self.named = Some(checks);
    }

    /// Note a build of this many checks, how long it took and how it ended.
    pub fn built(&mut self, checks: usize, took: Duration, passed: bool) {
        self.built = Some(json!({
            "checks": checks,
            "seconds": seconds(took),
            "passed": passed,
        }));
    }

    /// The run as its line, with how long it took so far.
    pub fn line(&self) -> Value {
        json!({
            "at": self.at,
            "base": self.base,
            "system": self.system,
            "seconds": seconds(self.began.elapsed()),
            "evaluated": self.evaluations,
            "named": self.named,
            "built": self.built,
        })
    }

    /// Add the run's line to the log at the path, making its folder.
    pub fn record(&self, log: &Path) -> Result<(), String> {
        let failed = |error: std::io::Error| format!("{}: {error}", log.display());
        if let Some(folder) = log.parent() {
            std::fs::create_dir_all(folder).map_err(failed)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .map_err(failed)?;
        writeln!(file, "{}", self.line()).map_err(failed)
    }
}

/// A duration in seconds, to the tenth, as the log keeps it.
fn seconds(took: Duration) -> f64 {
    (took.as_secs_f64() * 10.0).round() / 10.0
}
