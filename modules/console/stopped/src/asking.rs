//! Asks the programs a failed boot's summary reads, each held to a
//! deadline: the shell waits for the summary, and systemd gives the
//! shell's start only so long (`Budget` says how much of it this takes).

use std::cell::RefCell;
use std::io::Read;
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// How long the summary may spend asking, and each command it asks.
/// systemd gives the shell's start, this included, ninety seconds, since
/// emergency.service sets no time of its own, and Plymouth's leaving
/// comes first, so the summary takes a small share. Each command has a
/// share of its own, so one that stalls leaves the rest their time, and a
/// program that stalled once isn't asked again: a journal that stalls on
/// one unit stalls on the next.
pub struct Budget {
    by: Instant,
    each: Duration,
    stalled: RefCell<Vec<String>>,
}

impl Budget {
    pub fn from_now() -> Self {
        Self::lasting(Duration::from_secs(10), Duration::from_secs(2))
    }

    fn lasting(whole: Duration, each: Duration) -> Self {
        Budget {
            by: Instant::now() + whole,
            each,
            stalled: RefCell::default(),
        }
    }

    /// What `program` printed given `args`, and how it ended, if it ends
    /// in its share of the time left, or why not. Nothing is asked once
    /// the whole budget is spent, and a program cut short by the whole
    /// budget rather than its own share isn't counted as stalled, since
    /// it was given less than its share.
    pub fn ask(&self, program: &str, args: &[&str]) -> Result<Output, String> {
        if self
            .stalled
            .borrow()
            .iter()
            .any(|stalled| stalled == program)
        {
            return Err(format!(
                "{program} didn't answer before, so it wasn't asked again"
            ));
        }
        let now = Instant::now();
        if now >= self.by {
            return Err(format!("no time was left to ask {program}"));
        }
        let own = now + self.each;
        answered(program, args, self.by.min(own)).map_err(|unanswered| {
            if unanswered.late && own <= self.by {
                self.stalled.borrow_mut().push(program.to_owned());
            }
            unanswered.why
        })
    }
}

/// Why a command gave no answer, and whether that was for want of time.
#[derive(Debug)]
struct Unanswered {
    why: String,
    late: bool,
}

/// A command as its thread and the asker both see it: the process once
/// started, and whether the asker has stopped waiting for it.
#[derive(Default)]
struct Asked {
    child: Option<Child>,
    given_up: bool,
}

fn lock(asked: &Mutex<Asked>) -> MutexGuard<'_, Asked> {
    asked.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What a command printed and how it ended, if it ends, and closes what
/// it prints to, by `by`. It runs on a thread of its own, and this waits
/// on that thread only until `by`, since starting a program, reading what
/// it prints and reaping it can each block on a failing disk, where even
/// a killed process ends only once the disk answers. Past `by` the
/// command is killed, and its thread reaps it whenever it ends.
fn answered(program: &str, args: &[&str], by: Instant) -> Result<Output, Unanswered> {
    let broken = |why: String| Unanswered { why, late: false };
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let asked = Arc::new(Mutex::new(Asked::default()));
    let (tell, heard) = mpsc::channel();
    let (name, theirs) = (program.to_owned(), Arc::clone(&asked));
    thread::Builder::new()
        .spawn(move || {
            let _ = tell.send(run(&name, command, &theirs));
        })
        .map_err(|error| broken(format!("{program} couldn't be asked: {error}")))?;
    let stop = || {
        let mut asked = lock(&asked);
        asked.given_up = true;
        if let Some(child) = asked.child.as_mut() {
            let _ = child.kill();
        }
    };
    match heard.recv_timeout(by.saturating_duration_since(Instant::now())) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => {
            stop();
            Err(Unanswered {
                why: format!("{program} didn't answer in time, so it was stopped"),
                late: true,
            })
        }
        Err(RecvTimeoutError::Disconnected) => {
            stop();
            Err(broken(format!("{program} couldn't be asked")))
        }
    }
}

/// A command run to its end on the thread `answered` gives it: started,
/// read, then reaped, with the process left where the asker can kill it.
fn run(program: &str, mut command: Command, asked: &Mutex<Asked>) -> Result<Output, Unanswered> {
    let broken = |why: String| Unanswered { why, late: false };
    let mut child = command
        .spawn()
        .map_err(|error| broken(format!("{program} couldn't run: {error}")))?;
    let (stdout, stderr) = (child.stdout.take(), child.stderr.take());
    {
        let mut asked = lock(asked);
        if asked.given_up {
            let _ = child.kill();
        }
        asked.child = Some(child);
    }
    // Both streams are read at once, so neither fills while the other is
    // waited on; what it says to stderr is let go if no thread is free.
    let errors = stderr.and_then(|stream| thread::Builder::new().spawn(|| all(stream)).ok());
    let printed = stdout.map(all).unwrap_or_default();
    let said = errors
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    loop {
        if let Some(child) = lock(asked).child.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return Ok(Output {
                        status,
                        stdout: printed,
                        stderr: said,
                    });
                }
                Ok(None) => {}
                Err(error) => {
                    let _ = child.kill();
                    return Err(broken(format!("{program} couldn't be waited on: {error}")));
                }
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// A stream read to its end.
fn all(mut stream: impl Read) -> Vec<u8> {
    let mut read = Vec::new();
    let _ = stream.read_to_end(&mut read);
    read
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::thread;
    use std::time::{Duration, Instant};

    use super::{Budget, answered};

    fn soon() -> Instant {
        Instant::now() + Duration::from_secs(5)
    }

    #[test]
    fn a_command_that_answers_in_time_is_heard() {
        let said = answered("echo", &["disk gone"], soon()).expect("echo wasn't heard");
        assert!(said.status.success(), "echo was heard failing");
        assert_eq!(
            said.stdout, b"disk gone\n",
            "echo was heard saying something else"
        );
    }

    #[test]
    fn a_command_past_its_deadline_is_stopped_killed_and_reaped() {
        let pid_file = std::env::temp_dir().join(format!("stalled-{}", std::process::id()));
        let script = format!("echo $$ > {}; exec sleep 30", pid_file.display());
        let started = Instant::now();
        let said = answered("sh", &["-c", &script], started + Duration::from_millis(500));
        assert!(
            said.as_ref().is_err_and(|unanswered| unanswered.late),
            "a stalled command wasn't stopped for time: {said:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "a stalled command held it {:?}",
            started.elapsed()
        );
        let pid = fs::read_to_string(&pid_file).expect("the stalled command left no pid");
        let _ = fs::remove_file(&pid_file);
        // A process killed but not reaped stays in /proc as a zombie, so
        // its going shows both.
        let proc = Path::new("/proc").join(pid.trim());
        let by = Instant::now() + Duration::from_secs(5);
        while proc.exists() && Instant::now() < by {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !proc.exists(),
            "the stalled command wasn't killed and reaped"
        );
    }

    #[test]
    fn a_process_left_holding_the_pipe_is_out_of_time_too() {
        let started = Instant::now();
        let said = answered(
            "sh",
            &["-c", "sleep 10 & echo started"],
            started + Duration::from_secs(1),
        );
        assert!(
            said.as_ref().is_err_and(|unanswered| unanswered.late),
            "what it left behind was waited on: {said:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "what it left behind held it {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn a_command_that_cant_run_says_so() {
        let said = answered("/nonexistent/journalctl", &[], soon());
        assert!(
            said.as_ref().is_err_and(
                |unanswered| !unanswered.late && unanswered.why.contains("couldn't run")
            ),
            "a missing program was heard: {said:?}"
        );
    }

    #[test]
    fn a_program_that_stalled_isnt_asked_again() {
        let budget = Budget::lasting(Duration::from_secs(10), Duration::from_millis(200));
        let first = budget.ask("sleep", &["30"]);
        assert!(first.is_err(), "a stalled command was heard: {first:?}");
        let started = Instant::now();
        let again = budget.ask("sleep", &["0"]);
        assert!(
            again
                .as_ref()
                .is_err_and(|why| why.contains("wasn't asked again")),
            "a program that stalled was asked again: {again:?}"
        );
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "asking again took {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn the_whole_budget_ends_every_ask_and_counts_no_one_stalled() {
        let started = Instant::now();
        let budget = Budget::lasting(Duration::from_millis(300), Duration::from_secs(10));
        let first = budget.ask("sleep", &["30"]);
        assert!(
            first.is_err(),
            "a command past the budget was heard: {first:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "a command past the budget held it {:?}",
            started.elapsed()
        );
        assert!(
            budget.stalled.borrow().is_empty(),
            "a program cut short by the budget was counted as stalled"
        );
        let after = budget.ask("echo", &["late"]);
        assert!(
            after
                .as_ref()
                .is_err_and(|why| why.contains("no time was left")),
            "a command was asked past the budget: {after:?}"
        );
    }
}
