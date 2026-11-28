//! Running the machine's own programs for a stop, each to a limit, so a
//! machine that doesn't answer can't hold Joel's act up.

use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Runs a program to its end: what it printed, or why it failed.
pub trait Runs {
    /// Runs `program` with `args`, answering its output when it exits 0.
    ///
    /// # Errors
    ///
    /// What it said, or why it couldn't run, when it didn't exit 0.
    fn run(&mut self, program: &Path, args: &[String]) -> Result<String, String>;

    /// Runs each program with its args, answering each as `run` would, in
    /// the order given: one after another, unless the runner can run them
    /// all at once.
    fn run_all(&mut self, runs: &[(&Path, &[String])]) -> Vec<Result<String, String>> {
        runs.iter()
            .map(|(program, args)| self.run(program, args))
            .collect()
    }
}

/// The machine's own programs, each given at most `limit` before it's
/// killed.
pub struct System {
    /// How long each program is given.
    pub limit: Duration,
}

/// How often a program running is looked at, to end it at its limit.
const LOOKED: Duration = Duration::from_millis(10);

impl Runs for System {
    fn run(&mut self, program: &Path, args: &[String]) -> Result<String, String> {
        self.run_all(&[(program, args)])
            .pop()
            .unwrap_or_else(|| Err("nothing ran".to_owned()))
    }

    /// Starts every program at once and waits for them together, each
    /// given the same `limit` from the start, so one that doesn't answer
    /// holds none of the others up past it.
    fn run_all(&mut self, runs: &[(&Path, &[String])]) -> Vec<Result<String, String>> {
        let began = Instant::now();
        let (mut running, mut answers): (Vec<_>, Vec<_>) = runs
            .iter()
            .map(|(program, args)| match start(program, args) {
                Ok(one) => (Some(one), None),
                Err(why) => (None, Some(Err(why))),
            })
            .unzip();
        while answers.iter().any(Option::is_none) {
            let late = began.elapsed() >= self.limit;
            for (slot, answer) in running.iter_mut().zip(&mut answers) {
                let Some(one) = slot else { continue };
                let status = match one.child.try_wait() {
                    Ok(Some(status)) => Ok(status),
                    Ok(None) if late => {
                        // Killed at its limit; it may have ended meanwhile,
                        // which is as good. Its readers are left to end with
                        // the pipes, which one of its own children may hold.
                        let _ = one.child.kill();
                        let _ = one.child.wait();
                        Err(format!("no answer in {} s", self.limit.as_secs()))
                    }
                    Ok(None) => continue,
                    Err(error) => Err(error.to_string()),
                };
                if let Some(one) = slot.take() {
                    *answer = Some(status.and_then(|status| one.answer(status)));
                }
            }
            if answers.iter().any(Option::is_none) {
                std::thread::sleep(LOOKED);
            }
        }
        answers.into_iter().flatten().collect()
    }
}

/// A program started, with the readers of what it prints.
struct Running {
    child: Child,
    out: JoinHandle<Vec<u8>>,
    err: JoinHandle<Vec<u8>>,
}

/// Starts `program` with `args`, reading what it prints as it runs, since
/// a program printing more than a pipe holds waits for its reader and
/// would otherwise never end.
fn start(program: &Path, args: &[String]) -> Result<Running, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("{} can't run: {error}", program.to_string_lossy()))?;
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    Ok(Running { child, out, err })
}

impl Running {
    /// What it printed, once it exited 0 as `status` says, or what it said
    /// when it didn't.
    fn answer(self, status: ExitStatus) -> Result<String, String> {
        let out = String::from_utf8_lossy(&self.out.join().unwrap_or_default()).into_owned();
        if status.success() {
            return Ok(out);
        }
        let err = String::from_utf8_lossy(&self.err.join().unwrap_or_default()).into_owned();
        let said = if err.trim().is_empty() { out } else { err };
        Err(format!("{} ({status})", said.trim()))
    }
}

/// Reads all a pipe gives, on a thread of its own.
fn drain(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut read = Vec::new();
        if let Some(mut pipe) = pipe {
            // What couldn't be read is said as nothing; the exit says
            // whether it ran.
            let _ = pipe.read_to_end(&mut read);
        }
        read
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::{Runs, System};

    /// `sh -c` running `line`.
    fn sh(line: &str) -> Vec<String> {
        vec!["-c".to_owned(), line.to_owned()]
    }

    #[test]
    fn programs_run_together_answer_each_in_turn_and_none_waits_on_another() {
        let mut system = System {
            limit: Duration::from_secs(4),
        };
        let first = sh("sleep 2; echo one");
        let hung = sh("sleep 30");
        let failing = sh("sleep 2; echo no >&2; exit 3");
        let began = Instant::now();
        let answers = system.run_all(&[
            (Path::new("sh"), &first),
            (Path::new("sh"), &hung),
            (Path::new("sh"), &failing),
            (Path::new("/nowhere/at/all"), &first),
        ]);
        let took = began.elapsed();
        assert_eq!(
            answers[0],
            Ok("one\n".to_owned()),
            "the first answered: {answers:?}"
        );
        assert_eq!(
            answers[1],
            Err("no answer in 4 s".to_owned()),
            "the hung one ended at its limit: {answers:?}"
        );
        assert!(
            answers[2]
                .as_ref()
                .is_err_and(|why| why.starts_with("no (") && why.contains('3')),
            "the failing one said why: {answers:?}"
        );
        assert!(
            answers[3]
                .as_ref()
                .is_err_and(|why| why.starts_with("/nowhere/at/all can't run")),
            "one that couldn't start said so: {answers:?}"
        );
        // One after another they'd take 2 + 4 + 2 seconds; together, the
        // limit's 4.
        assert!(
            took < Duration::from_secs(6),
            "all waited on together, within the one limit: {took:?}"
        );
    }
}
