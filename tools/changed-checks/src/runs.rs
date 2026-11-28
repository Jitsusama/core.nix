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
    phases: Vec<(&'static str, Duration)>,
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
            phases: Vec::new(),
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

    /// Note that getting a side's derivations, "base" or "tree", took this
    /// long however they came: kept, evaluated, or waited for while
    /// another run evaluated them, which no evaluation of this run counts.
    pub fn phase(&mut self, of: &'static str, took: Duration) {
        self.phases.push((of, took));
    }

    /// What each phase of the run took, as its line says it in the gate's
    /// words, so a wait is seen where it was paid: on 28 November a train's
    /// run spent 508 of its 1222 s outside every evaluation it counted.
    pub fn said(&self) -> String {
        let mut phases: Vec<String> = self
            .phases
            .iter()
            .map(|(of, took)| format!("the {of} {:.1} s", seconds(*took)))
            .collect();
        let built = self.built.as_ref().and_then(|built| built.get("seconds"));
        if let Some(built) = built.and_then(Value::as_f64) {
            phases.push(format!("the build {built:.1} s"));
        }
        let took = seconds(self.began.elapsed());
        if phases.is_empty() {
            return format!("the run took {took:.1} s");
        }
        format!("the run took {took:.1} s: {}", phases.join(", "))
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
            "phases": self
                .phases
                .iter()
                .map(|(of, took)| json!({ "of": of, "seconds": seconds(*took) }))
                .collect::<Vec<_>>(),
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

    #[test]
    fn a_run_says_and_logs_what_each_phase_took() {
        let mut run = Run::new("abc", "x86_64-linux");
        run.phase("base", Duration::from_millis(517_740));
        run.phase("tree", Duration::from_millis(714_400));
        run.built(27, Duration::from_secs(133), true);
        let said = run.said();
        assert!(
            said.ends_with(": the base 517.7 s, the tree 714.4 s, the build 133.0 s"),
            "{said}"
        );
        assert_eq!(
            run.line()["phases"],
            json!([{ "of": "base", "seconds": 517.7 }, { "of": "tree", "seconds": 714.4 }])
        );
    }

    #[test]
    fn a_run_with_no_phase_says_only_what_it_took() {
        let said = Run::new("abc", "x86_64-linux").said();
        assert!(said.starts_with("the run took "), "{said}");
        assert!(!said.contains(':'), "{said}");
    }
}
