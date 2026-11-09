//! How often a to-do's check runs, and what each run found. A check runs
//! as its to-do is first heard and at each signal the machine gives, and
//! between those after 30 s, doubling to 10 min while it keeps failing,
//! so a check that won't pass for a day costs a run each ten minutes.
//! Every due check runs at once, each on its own and for at most
//! TIMEOUT, its run told back as it ends, so one slow check holds up no
//! other.

use std::process::Command;
use std::time::Duration;

use crate::bounded;

/// The first wait after a failing run, in seconds.
pub const FIRST: i64 = 30;
/// The longest wait, in seconds.
pub const LONGEST: i64 = 600;
/// How long a run may take before it's asked to stop, in seconds: long
/// enough for a check that asks a service over the network, as one
/// bounding its own call at a minute does.
pub const TIMEOUT: u64 = 90;
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
    /// Whether a run has started that hasn't been told back yet.
    running: bool,
    /// Whether a signal came while it ran, so it runs again as it ends.
    again: bool,
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
            running: false,
            again: false,
        }
    }

    /// When it must next wake the daemon: never while it runs, since its
    /// run wakes it as it ends.
    pub fn next(&self) -> Option<i64> {
        (!self.running).then_some(self.next_at)
    }

    /// Whether it should start a run now, never while one goes.
    pub fn due(&self, now: i64) -> bool {
        !self.running && now >= self.next_at
    }

    /// Says a run has started, so none starts beside it.
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Runs it again at once, as a signal may have changed what it checks.
    pub fn hurry(&mut self, now: i64) {
        self.next_at = self.next_at.min(now);
        self.again |= self.running;
    }

    /// Takes in a run made at `at`, and when the next is due.
    pub fn ran(&mut self, run: Run, at: i64) {
        self.running = false;
        self.ran += 1;
        self.first_at.get_or_insert(at);
        self.last_at = Some(at);
        self.failing = if run.passed() { 0 } else { self.failing + 1 };
        self.next_at = if self.again {
            at
        } else {
            at + wait(self.failing)
        };
        self.again = false;
        self.last = Some(run);
    }
}

/// The wait after `failing` runs in a row have failed.
pub fn wait(failing: u32) -> i64 {
    let doubled = FIRST.saturating_mul(1 << failing.saturating_sub(1).min(16));
    doubled.min(LONGEST)
}

/// Runs `check` in `sh` with `session`'s variables set over the daemon's
/// own, stopped after TIMEOUT seconds.
pub fn run(check: &str, session: &[(String, String)]) -> Run {
    run_within(check, session, Duration::from_secs(TIMEOUT), bounded::GRACE)
}

/// Runs `check` in `sh` with `session`'s variables for at most `bound`,
/// then `grace` once asked to stop.
pub fn run_within(
    check: &str,
    session: &[(String, String)],
    bound: Duration,
    grace: Duration,
) -> Run {
    let mut command = Command::new("sh");
    command.args(["-c", check]).envs(session.iter().cloned());
    match bounded::run(command, bound, grace) {
        Ok(ended) => Run {
            exit: ended.status.and_then(|status| status.code()),
            took_ms: u64::try_from(ended.took.as_millis()).unwrap_or(u64::MAX),
            said: if ended.status.is_some() {
                last_line(&ended.output)
            } else {
                format!("stopped after {} s", bound.as_secs())
            },
        },
        Err(error) => Run {
            exit: None,
            took_ms: 0,
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
        assert!(pace.due(1000), "a new check is due at once");
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
        assert!(!pace.due(1089), "not due before its wait is over");
        pace.hurry(1050);
        assert!(pace.due(1050), "a signal makes it due at once");
    }

    #[test]
    fn a_check_says_its_exit_and_its_last_line() {
        let found = run("echo one; echo two >&2; echo; exit 3", &[]);
        assert_eq!(found.exit, Some(3));
        assert_eq!(found.said, "two");
        assert!(run("true", &[]).passed(), "true passes");
    }

    #[test]
    fn a_check_has_the_sessions_variables() {
        let session = [("FROM_SESSION".to_owned(), "here".to_owned())];
        assert_eq!(run("echo \"$FROM_SESSION\"", &session).said, "here");
    }

    #[test]
    fn a_check_that_hangs_is_stopped_and_says_so() {
        // The sleep stands for a check that never answers.
        let found = run_within(
            "sleep 30",
            &[],
            Duration::from_millis(200),
            Duration::from_millis(200),
        );
        assert_eq!(found.exit, None);
        assert!(!found.passed(), "a check stopped never passes");
        assert!(found.said.starts_with("stopped after "), "{}", found.said);
    }

    #[test]
    fn the_last_line_keeps_only_the_end() {
        let long = format!("{}\nend\n", "x".repeat(10_000));
        assert_eq!(last_line(long.as_bytes()), "end");
        assert_eq!(last_line(b""), "");
    }
}
