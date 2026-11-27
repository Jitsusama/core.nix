//! Running the machine's own programs for a stop, each to a limit, so a
//! machine that doesn't answer can't hold Joel's act up.

use std::io::{ErrorKind, Read};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, PoisonError};
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

/// How long a program that ended is given for its readers to take what
/// it printed before it's answered as unread, so one that ended just
/// before its limit isn't taken for one whose output is held open.
const GRACE: Duration = Duration::from_millis(200);

/// The most programs run at once, each holding two pipes and two threads
/// reading them, so a long list can't spend the daemon's own.
pub const AT_ONCE: usize = 16;

impl Runs for System {
    fn run(&mut self, program: &Path, args: &[String]) -> Result<String, String> {
        self.run_all(&[(program, args)])
            .pop()
            .unwrap_or_else(|| Err("nothing ran".to_owned()))
    }

    /// Starts the programs [`AT_ONCE`] at a time and waits for each lot
    /// together, each given the same `limit` from its lot's start, so one
    /// that doesn't answer holds none of the others up past it.
    fn run_all(&mut self, runs: &[(&Path, &[String])]) -> Vec<Result<String, String>> {
        runs.chunks(AT_ONCE)
            .flat_map(|lot| self.run_together(lot))
            .collect()
    }
}

impl System {
    /// Starts every program at once and waits for them together, each
    /// given `limit` from the start. One that ended is answered once what
    /// it printed is read, or as unknown at the limit once its readers
    /// had [`GRACE`] since it ended, as a program it started may still
    /// hold its output open.
    fn run_together(&self, runs: &[(&Path, &[String])]) -> Vec<Result<String, String>> {
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
                // Ended and read, or answered without its readers, which
                // are left to end with the pipes, as one of its own
                // children may hold them.
                let ended = match one.exited() {
                    Ok(Some(status)) if one.read(status) => Ok(status),
                    Ok(Some(status)) if late && one.ended_before(GRACE) => {
                        Err(Err(unread(status, &one.said_so_far())))
                    }
                    Ok(None) if late => {
                        // Killed at its limit; it may have ended meanwhile,
                        // which is as good.
                        let _ = one.child.kill();
                        let _ = one.child.wait();
                        Err(Err(format!("no answer in {} s", self.limit.as_secs())))
                    }
                    Ok(_) => continue,
                    Err(error) => {
                        let _ = one.child.kill();
                        let _ = one.child.wait();
                        Err(Err(error.to_string()))
                    }
                };
                if let Some(one) = slot.take() {
                    *answer = Some(match ended {
                        Ok(status) => one.answer(status),
                        Err(said) => said,
                    });
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
    out: Reader,
    err: Reader,
    /// When it was first seen ended.
    ended: Option<Instant>,
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
    match (drain(child.stdout.take()), drain(child.stderr.take())) {
        (Ok(out), Ok(err)) => Ok(Running {
            child,
            out,
            err,
            ended: None,
        }),
        (out, err) => {
            // Ended, so a reader that did start ends with the pipe.
            let _ = child.kill();
            let _ = child.wait();
            let why = out.err().or_else(|| err.err()).unwrap_or_default();
            Err(format!(
                "{} couldn't be read as it ran: {why}",
                program.to_string_lossy()
            ))
        }
    }
}

/// What a program that ended as `status` answers while what it printed is
/// still held open at the limit, as by a program it started: never a
/// success, since a caller reading its words would take none for nothing
/// printed, such as no agents' work running; and never ending in its
/// status, since a 255 so ending reads as the command never running there
/// (src/pause/reach.rs), when it may have begun. What it `said` before
/// comes first, as a failure's answer has it, so a journal keeping its
/// first line says why.
fn unread(status: ExitStatus, said: &str) -> String {
    let unknown = format!(
        "it ended ({status}), but what it printed was still held open at the \
         limit, so what it did there is unknown"
    );
    match said.trim() {
        "" => unknown,
        said => format!("{said}\n{unknown}"),
    }
}

impl Running {
    /// How it ended, once it has, noting when that was first seen.
    fn exited(&mut self) -> std::io::Result<Option<ExitStatus>> {
        let status = self.child.try_wait()?;
        if status.is_some() {
            self.ended.get_or_insert_with(Instant::now);
        }
        Ok(status)
    }

    /// Whether it was seen ended at least `grace` ago.
    fn ended_before(&self, grace: Duration) -> bool {
        self.ended.is_some_and(|ended| ended.elapsed() >= grace)
    }

    /// Whether all its answer as `status` takes is read, so answering it
    /// waits on nothing: what it printed, and what it said unless it
    /// exited 0, which answers its output alone.
    fn read(&self, status: ExitStatus) -> bool {
        self.out.finished() && (status.success() || self.err.finished())
    }

    /// What it said on its error output so far, or what it printed when
    /// it said nothing there.
    fn said_so_far(&self) -> String {
        let err = self.err.so_far();
        if err.trim().is_empty() {
            self.out.so_far()
        } else {
            err
        }
    }

    /// What it printed, once it exited 0 as `status` says, or what it said
    /// when it didn't.
    fn answer(self, status: ExitStatus) -> Result<String, String> {
        let out = self.out.all();
        if status.success() {
            return Ok(out);
        }
        let err = self.err.all();
        let said = if err.trim().is_empty() { out } else { err };
        Err(format!("{} ({status})", said.trim()))
    }
}

/// A pipe read on a thread of its own, what it gave kept as it comes, so
/// what was read is known before the pipe ends.
struct Reader {
    read: Arc<Mutex<Vec<u8>>>,
    thread: JoinHandle<()>,
}

impl Reader {
    /// Whether its pipe ended, so all it gives is read.
    fn finished(&self) -> bool {
        self.thread.is_finished()
    }

    /// What it read so far.
    fn so_far(&self) -> String {
        text(&self.read)
    }

    /// All it reads, once its pipe ends.
    fn all(self) -> String {
        // A reader that panicked read only what it kept.
        let _ = self.thread.join();
        text(&self.read)
    }
}

/// What `read` holds, as text.
fn text(read: &Mutex<Vec<u8>>) -> String {
    let read = read.lock().unwrap_or_else(PoisonError::into_inner);
    String::from_utf8_lossy(&read).into_owned()
}

/// Reads all a pipe gives, on a thread of its own, or why no thread could
/// be started.
fn drain(pipe: Option<impl Read + Send + 'static>) -> Result<Reader, String> {
    let read = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&read);
    let thread = std::thread::Builder::new()
        .spawn(move || {
            let Some(mut pipe) = pipe else { return };
            let mut chunk = [0; 8192];
            loop {
                match pipe.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(given) => kept
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .extend_from_slice(&chunk[..given]),
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    // What couldn't be read is said as nothing; the exit
                    // says whether it ran.
                    Err(_) => break,
                }
            }
        })
        .map_err(|error| error.to_string())?;
    Ok(Reader { read, thread })
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::super::reach::{BEGUN, didnt_run};
    use super::{AT_ONCE, Runs, System};

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

    #[test]
    fn more_programs_than_run_at_once_go_in_lots() {
        let mut system = System {
            limit: Duration::from_secs(4),
        };
        let each: Vec<Vec<String>> = (0..=AT_ONCE)
            .map(|at| sh(&format!("sleep 1; echo {at}")))
            .collect();
        let runs: Vec<(&Path, &[String])> = each
            .iter()
            .map(|one| (Path::new("sh"), one.as_slice()))
            .collect();
        let began = Instant::now();
        let answers = system.run_all(&runs);
        let took = began.elapsed();
        let said: Vec<Result<String, String>> =
            (0..=AT_ONCE).map(|at| Ok(format!("{at}\n"))).collect();
        assert_eq!(
            answers, said,
            "each answered in the order asked, across lots"
        );
        assert!(
            took >= Duration::from_secs(2),
            "the one past {AT_ONCE} waited for the lot before it: {took:?}"
        );
        // In smaller lots, or one by one, they'd take 3 seconds or more.
        assert!(
            took < Duration::from_secs(3),
            "the first {AT_ONCE} ran as one lot: {took:?}"
        );
    }

    #[test]
    fn each_lot_is_given_its_own_limit() {
        let mut system = System {
            limit: Duration::from_millis(1500),
        };
        let each: Vec<Vec<String>> = (0..=AT_ONCE)
            .map(|at| sh(&format!("sleep 1; echo {at}")))
            .collect();
        let runs: Vec<(&Path, &[String])> = each
            .iter()
            .map(|one| (Path::new("sh"), one.as_slice()))
            .collect();
        let answers = system.run_all(&runs);
        // The last lot starts a second in, so a limit from the first
        // lot's start would end it half a second before it answers.
        assert_eq!(
            answers.last(),
            Some(&Ok(format!("{AT_ONCE}\n"))),
            "the last lot had its limit from its own start: {answers:?}"
        );
    }

    #[test]
    fn one_ended_whose_output_is_held_briefly_is_waited_for_and_read() {
        let mut system = System {
            limit: Duration::from_secs(2),
        };
        let leaving = sh("(sleep 0.5; echo later) & echo now");
        let answers = system.run_all(&[(Path::new("sh"), &leaving)]);
        assert_eq!(
            answers[0],
            Ok("now\nlater\n".to_owned()),
            "what its child printed after it ended is read: {answers:?}"
        );
    }

    #[test]
    fn one_ended_whose_child_holds_its_output_is_answered_at_the_limit() {
        let mut system = System {
            limit: Duration::from_secs(1),
        };
        let leaving = sh("sleep 5 & echo done");
        let hung = sh("sleep 5");
        let began = Instant::now();
        let answers = system.run_all(&[(Path::new("sh"), &leaving), (Path::new("sh"), &hung)]);
        let took = began.elapsed();
        assert!(
            answers[0]
                .as_ref()
                .is_err_and(|why| why.contains("still held open")),
            "it ended well, but its words unread aren't taken for none: {answers:?}"
        );
        assert_eq!(
            answers[1],
            Err("no answer in 1 s".to_owned()),
            "the hung one was killed: {answers:?}"
        );
        assert!(
            took < Duration::from_secs(3),
            "neither waited on the output its child holds: {took:?}"
        );
    }

    #[test]
    fn one_ended_just_before_its_limit_is_read_past_it() {
        let mut system = System {
            limit: Duration::from_secs(1),
        };
        // It ends a tenth of a second before its limit, its child holding
        // what it printed a twentieth past it.
        let late = sh("sleep 0.9; sleep 0.15 & echo done");
        let answers = system.run_all(&[(Path::new("sh"), &late)]);
        assert_eq!(
            answers[0],
            Ok("done\n".to_owned()),
            "its readers had their time once it ended: {answers:?}"
        );
    }

    #[test]
    fn one_exited_0_with_its_output_whole_succeeds_while_its_errors_are_held() {
        let mut system = System {
            limit: Duration::from_secs(2),
        };
        let leaving = sh("echo done; sleep 5 >&2 &");
        let began = Instant::now();
        let answers = system.run_all(&[(Path::new("sh"), &leaving)]);
        let took = began.elapsed();
        assert_eq!(
            answers[0],
            Ok("done\n".to_owned()),
            "its output is whole, which is all a success answers: {answers:?}"
        );
        assert!(
            took < Duration::from_secs(1),
            "it was answered as it ended, not at its limit: {took:?}"
        );
    }

    #[test]
    fn one_answered_unread_keeps_what_it_said_first() {
        let mut system = System {
            limit: Duration::from_secs(1),
        };
        let held = sh("echo some; echo 'mycelium: refused there' >&2; sleep 5 &");
        let answers = system.run_all(&[(Path::new("sh"), &held)]);
        assert!(
            answers[0].as_ref().is_err_and(|why| {
                why.lines().next() == Some("mycelium: refused there")
                    && why.contains("still held open")
            }),
            "what it said is its answer's first line, its words unread after: {answers:?}"
        );
    }

    #[test]
    fn one_begun_that_failed_with_its_output_held_isnt_taken_for_never_run() {
        let mut system = System {
            limit: Duration::from_secs(1),
        };
        let begun = sh(&format!("echo '{BEGUN}' >&2; sleep 5 & exit 255"));
        let answers = system.run_all(&[(Path::new("sh"), &begun)]);
        assert!(
            answers[0].as_ref().is_err_and(|why| !didnt_run(why)),
            "its words unread, it may have begun, so it isn't read as never run: {answers:?}"
        );
    }
}
