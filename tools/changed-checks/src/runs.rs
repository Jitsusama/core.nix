//! What each run took, kept as a line of JSON in `changed-checks/runs.jsonl`
//! under XDG's state folder, so the time from a change to its proof is
//! measured run after run rather than remembered: each side's evaluation
//! chunk by chunk, the record of each check's reads it judged from and how
//! far main and the tree had moved from it, what the comparison named, and
//! the build: where it ran,
//! and, on a lent builder, what it sent up, so a build sent there is seen
//! to cost the upload nothing bulky.

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
    recorded: Option<Value>,
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
            recorded: None,
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

    /// Note the head of main whose record of each check's reads the run
    /// judged from, and how many files moved from it to the base, `main`,
    /// and to the tree, each when git could say: the further they moved,
    /// the more checks a change reaches.
    pub fn recorded(&mut self, head: &str, main: Option<usize>, tree: Option<usize>) {
        self.recorded = Some(json!({ "head": head, "main": main, "tree": tree }));
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

    /// Note that the build ran on the lent builder, sending it this many
    /// bytes, where ssh said.
    pub fn lent(&mut self, sent: Option<u64>) {
        if let Some(Value::Object(built)) = &mut self.built {
            built.insert("lent".to_owned(), Value::Bool(true));
            built.insert("sent".to_owned(), json!(sent));
        }
    }

    /// The run as its line, with how long it took so far.
    pub fn line(&self) -> Value {
        json!({
            "at": self.at,
            "base": self.base,
            "system": self.system,
            "seconds": seconds(self.began.elapsed()),
            "evaluated": self.evaluations,
            "recorded": self.recorded,
            "named": self.named,
            "built": self.built,
        })
    }

    /// Add the run's line to the log at the path, making its folder. The
    /// line goes in one write, which an appending file keeps whole beside
    /// another run's; `writeln!` writes it in pieces, which interleave.
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
        file.write_all(format!("{}\n", self.line()).as_bytes())
            .map_err(failed)
    }
}

/// A duration in seconds, to the tenth, as the log keeps it.
fn seconds(took: Duration) -> f64 {
    (took.as_secs_f64() * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch::Scratch;

    #[test]
    fn runs_logged_at_once_keep_their_lines_whole() {
        // On 12 November two gates on the lent builder logged at once, and
        // their lines came out mixed byte by byte, so neither could be read.
        let scratch = Scratch::new("runs-at-once");
        let log = scratch.join("runs.jsonl");
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    let mut run = Run::new(&"0".repeat(40), "x86_64-linux");
                    let mut evaluation = Run::evaluating("tree");
                    for _ in 0..20 {
                        evaluation.chunk(40, Duration::from_millis(1500));
                    }
                    run.evaluated(evaluation);
                    for _ in 0..50 {
                        run.record(&log).expect("logged");
                    }
                });
            }
        });
        let text = std::fs::read_to_string(&log).expect("the log");
        let broken = text
            .lines()
            .filter(|line| serde_json::from_str::<Value>(line).is_err())
            .count();
        let lines = text.lines().count();
        assert_eq!(broken, 0, "of {lines} lines, some were mixed");
    }
}
