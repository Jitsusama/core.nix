//! How often a to-do's check runs, and what each run found. A check runs
//! as its to-do is first heard and at each signal the machine gives, and
//! between those after 30 s, doubling to 10 min while it keeps failing,
//! so a check that won't pass for a day costs a run each ten minutes.

use std::process::{Command, Stdio};
use std::time::Instant;

/// The first wait after a failing run, in seconds.
pub const FIRST: i64 = 30;
/// The longest wait, in seconds.
pub const LONGEST: i64 = 600;
/// How long a run may take before it's stopped, in seconds.
pub const TIMEOUT: u32 = 10;
/// How much of a run's output is kept, its end, in bytes.
const KEPT: usize = 4096;

/// What one run found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// Its exit status, or none when it was stopped or couldn't start.
    pub exit: Option<i32>,
    pub took_ms: u64,
    /// The last line it wrote, as written, its errors read after its
    /// output, since a failing check says why last.
    pub said: String,
}

impl Run {
    pub fn passed(&self) -> bool {
        self.exit == Some(0)
    }
}

/// Every run of one check, as the drawer says them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pace {
    /// The command, so a to-do whose check is changed starts again.
    pub check: String,
    pub ran: u32,
    pub first_at: Option<i64>,
    pub last_at: Option<i64>,
    pub last: Option<Run>,
    /// When it runs next.
    pub next_at: i64,
    /// How many runs in a row have failed.
    failing: u32,
}

impl Pace {
    /// A check not yet run, due at once.
    pub fn new(check: &str, now: i64) -> Self {
        Pace {
            check: check.to_owned(),
            ran: 0,
            first_at: None,
            last_at: None,
            last: None,
            next_at: now,
            failing: 0,
        }
    }

    pub fn due(&self, now: i64) -> bool {
        now >= self.next_at
    }

    /// Runs it again at once, as a signal may have changed what it checks.
    pub fn hurry(&mut self, now: i64) {
        self.next_at = self.next_at.min(now);
    }

    /// Takes in a run made at `at`, and when the next is due.
    pub fn ran(&mut self, run: Run, at: i64) {
        self.ran += 1;
        self.first_at.get_or_insert(at);
        self.last_at = Some(at);
        self.failing = if run.passed() { 0 } else { self.failing + 1 };
        self.next_at = at + wait(self.failing);
        self.last = Some(run);
    }
}

/// The wait after `failing` runs in a row have failed.
pub fn wait(failing: u32) -> i64 {
    let doubled = FIRST.saturating_mul(1 << failing.saturating_sub(1).min(16));
    doubled.min(LONGEST)
}

/// Runs `check` in `sh`, stopped after TIMEOUT seconds.
pub fn run(check: &str) -> Run {
    let started = Instant::now();
    let output = Command::new("timeout")
        .args([&TIMEOUT.to_string(), "sh", "-c", check])
        .stdin(Stdio::null())
        .output();
    let took_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    match output {
        Ok(output) => {
            // timeout says 124 when it stopped the check, which is no
            // exit of the check's own.
            let exit = output.status.code().filter(|code| *code != 124);
            let mut text = output.stdout;
            text.extend_from_slice(&output.stderr);
            Run {
                exit,
                took_ms,
                said: last_line(&text),
            }
        }
        Err(error) => Run {
            exit: None,
            took_ms,
            said: error.to_string(),
        },
    }
}

/// The last line with words in `text`, from its last KEPT bytes.
fn last_line(text: &[u8]) -> String {
    let kept = &text[text.len().saturating_sub(KEPT)..];
    String::from_utf8_lossy(kept)
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default()
        .trim_end()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failing_check_waits_longer_each_time_to_ten_minutes() {
        let waits: Vec<i64> = (1..=7).map(wait).collect();
        assert_eq!(waits, [30, 60, 120, 240, 480, 600, 600]);
        assert_eq!(wait(200), 600);
    }

    #[test]
    fn a_run_says_when_the_next_is_and_counts_each() {
        let mut pace = Pace::new("false", 1000);
        assert!(pace.due(1000));
        let failed = Run {
            exit: Some(1),
            took_ms: 5,
            said: String::new(),
        };
        pace.ran(failed.clone(), 1000);
        assert_eq!(pace.next_at, 1030);
        pace.ran(failed, 1030);
        assert_eq!(pace.next_at, 1090);
        assert_eq!(
            (pace.ran, pace.first_at, pace.last_at),
            (2, Some(1000), Some(1030))
        );
        assert!(!pace.due(1089));
        pace.hurry(1050);
        assert!(pace.due(1050));
    }

    #[test]
    fn a_check_says_its_exit_and_its_last_line() {
        let found = run("echo one; echo two >&2; echo; exit 3");
        assert_eq!(found.exit, Some(3));
        assert_eq!(found.said, "two");
        assert!(run("true").passed());
    }

    #[test]
    fn the_last_line_keeps_only_the_end() {
        let long = format!("{}\nend\n", "x".repeat(10_000));
        assert_eq!(last_line(long.as_bytes()), "end");
        assert_eq!(last_line(b""), "");
    }
}
