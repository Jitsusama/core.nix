//! Says on the console what stopped the boot, before a failed boot's shell
//! asks for root, which is all there is to read on a machine whose root is
//! locked: each unit that failed, the last it said, and the way out
//! (summary.rs). A shell asked for with nothing failed says nothing.
//!
//! ```text
//! boot-stopped
//! ```
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit, in the theme its wrapper points it at
//! (../../nixos.nix), since root has no theme of its own. It asks
//! `systemctl` which units failed and `journalctl` what each said, both
//! from `PATH`. What a unit said is read through `--output=cat`, a line
//! each, so a unit's words can break no line but their own.
//!
//! The emergency shell runs it on the console, whose font draws only the
//! marks' stand-ins. systemd tells a program on `/dev/console` its `TERM`
//! by where the console's last output goes, which on a machine with a
//! serial console is the serial line rather than the screen, so a stream
//! on the console or a virtual terminal is drawn with the stand-ins
//! whatever `TERM` says.
//!
//! The shell waits for it, and systemd gives the shell's start only so
//! long, so every command it asks is held to a deadline (`Budget`): one
//! that stalls, as a journal on a failing disk can, is killed, and the
//! summary says what the others told it.

mod summary;

use std::fs;
use std::io::Read;
use std::process::{Command, ExitCode, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use terminal_kit::flags::Format;
use terminal_kit::{Context, Exit, Failure, Help, Ink, Pen, ink};

use summary::Failed;

fn help() -> Help {
    Help::new("boot-stopped", "says on the console what stopped the boot")
        .example(
            "boot-stopped",
            "each failed unit, the last it said, and the way out",
        )
        .exit(Exit::Unavailable, "systemd couldn't say which units failed")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), run)
}

fn run(context: &Context) -> Result<(), Failure> {
    if let Some(word) = context.args.first() {
        return Err(Failure::new(
            Exit::Usage,
            format!("boot-stopped takes no words, not {word}"),
        )
        .try_this("boot-stopped --help"));
    }
    // It says what it says for a person at the console, and has no record
    // to give a program, so a form for one is refused rather than met
    // with text.
    if context.shared.format != Format::Text {
        return Err(
            Failure::new(Exit::Usage, "boot-stopped says only text, for the console")
                .try_this("boot-stopped"),
        );
    }
    let budget = Budget::from_now();
    let names = failed(&budget)?;
    let units: Vec<Failed> = names
        .iter()
        .take(summary::SHOWN)
        .map(|name| Failed {
            about: described(&budget, name).unwrap_or_else(|| name.clone()),
            said: last(&budget, name),
            name: name.clone(),
        })
        .collect();
    let pen = on_the_console(context);
    context
        .say(&summary::text(&units, names.len(), &pen, context.room))
        .map_err(|error| {
            Failure::new(Exit::Failed, "the summary couldn't be said").because(error.to_string())
        })
}

/// The pen for stdout, drawing the marks' stand-ins where stdout is the
/// console or a virtual terminal, whose font draws nothing else.
fn on_the_console(context: &Context) -> Pen {
    let on_a_console =
        fs::read_link("/proc/self/fd/1").is_ok_and(|path| is_a_console(&path.to_string_lossy()));
    if !on_a_console || context.out.ink().console {
        return context.out.clone();
    }
    let ink = Ink {
        console: true,
        ..context.out.ink()
    };
    Pen::new(ink, ink::theme(&context.environment).as_ref())
}

/// Whether a stream's path is the console or a virtual terminal, such as
/// `/dev/tty1`, rather than a serial line, a pseudo-terminal or the
/// terminal a process calls its own.
fn is_a_console(path: &str) -> bool {
    path == "/dev/console"
        || path
            .strip_prefix("/dev/tty")
            .is_some_and(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
}

/// How long the summary may take, and each command it asks. systemd
/// gives the shell's start ninety seconds unless told otherwise, and the
/// console's font and Plymouth's leaving come first, so the summary takes
/// a small share. Each command has a share of its own, so one that stalls
/// leaves the rest their time.
struct Budget {
    by: Instant,
}

impl Budget {
    const WHOLE: Duration = Duration::from_secs(10);
    const EACH: Duration = Duration::from_secs(2);

    fn from_now() -> Self {
        Budget {
            by: Instant::now() + Self::WHOLE,
        }
    }

    /// What a command printed and how it ended, if it ends in its share
    /// of the time left; past that it's killed.
    fn ask(&self, command: &mut Command) -> Result<Output, String> {
        answered(command, self.by.min(Instant::now() + Self::EACH))
    }
}

/// What a command printed and how it ended, if it ends by `by`, or why
/// not: it couldn't run, or it was killed at `by`. What it prints is read
/// as it prints it, so a full pipe can't hold it.
fn answered(command: &mut Command, by: Instant) -> Result<Output, String> {
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("{program} couldn't run: {error}"))?;
    let stdout = child.stdout.take().map(read_apart);
    let stderr = child.stderr.take().map(read_apart);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < by => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                // Killed and reaped, and what it printed left unread: a
                // reader waits on no process that may still hold its pipe.
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{program} didn't answer in time, so it was stopped"
                ));
            }
            Err(error) => return Err(format!("{program} couldn't be waited on: {error}")),
        }
    };
    let all = |reader: Option<thread::JoinHandle<Vec<u8>>>| {
        reader
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default()
    };
    Ok(Output {
        status,
        stdout: all(stdout),
        stderr: all(stderr),
    })
}

/// A stream read to its end on a thread of its own.
fn read_apart(mut stream: impl Read + Send + 'static) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut read = Vec::new();
        let _ = stream.read_to_end(&mut read);
        read
    })
}

/// The units that failed, by name, as systemd lists them.
fn failed(budget: &Budget) -> Result<Vec<String>, Failure> {
    let unavailable = |why: String| {
        Failure::new(Exit::Unavailable, "systemd couldn't say which units failed").because(why)
    };
    let listed = budget
        .ask(Command::new("systemctl").args([
            "list-units",
            "--failed",
            "--plain",
            "--no-legend",
            "--full",
        ]))
        .map_err(unavailable)?;
    if !listed.status.success() {
        return Err(unavailable(
            String::from_utf8_lossy(&listed.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&listed.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect())
}

/// A unit's description, where systemd gives one.
fn described(budget: &Budget, unit: &str) -> Option<String> {
    let shown = budget
        .ask(Command::new("systemctl").args([
            "show",
            "--property=Description",
            "--value",
            "--",
            unit,
        ]))
        .ok()?;
    let about = String::from_utf8_lossy(&shown.stdout).trim().to_owned();
    (shown.status.success() && !about.is_empty()).then_some(about)
}

/// systemd's message for a unit's process exiting, with its code or
/// signal, as sd-messages.h names it.
const PROCESS_EXITED: &str = "MESSAGE_ID=98e322203f7a4ed290d09fe03c09fe15";

/// systemd's message for the result a unit failed with.
const FAILED_WITH_RESULT: &str = "MESSAGE_ID=d9b373ed55a64feb8242e02dbe79a49c";

/// The last three lines a unit's own programs said that show something,
/// or, when none of them does, as when one was killed, why systemd said
/// it failed. A line that shows nothing once cleaned, all blanks, escapes,
/// controls, marks or what draws no cell, says nothing, so the last twenty
/// are read for three that do. systemd's lines are asked for by its own
/// fields, since `--unit=` also gives the unit's own lines, and its last
/// may be one that draws nothing. Its last line of a unit is most often
/// what the unit cost or that it failed to start, which the heading says,
/// so the two that say why come first: the last time its process exited,
/// then the last result it failed with, each asked apart, since a unit
/// restarting into its start limit says two results and no exit between.
/// Any other of its lines only when a unit has neither.
fn last(budget: &Budget, unit: &str) -> Vec<String> {
    let own = journal(budget, &[&format!("_SYSTEMD_UNIT={unit}")], 20);
    let own = summary::last_shown(own, 3);
    if !own.is_empty() {
        return own;
    }
    let systemd = format!("UNIT={unit}");
    let mut why = journal(budget, &[&systemd, "_PID=1", PROCESS_EXITED], 1);
    why.extend(journal(
        budget,
        &[&systemd, "_PID=1", FAILED_WITH_RESULT],
        1,
    ));
    if why.is_empty() {
        journal(budget, &[&systemd, "_PID=1"], 1)
    } else {
        why
    }
}

/// The last `lines` this boot's journal holds matching every one of
/// `matching`, as they were said; nothing where the journal can't be read
/// in time.
fn journal(budget: &Budget, matching: &[&str], lines: usize) -> Vec<String> {
    budget
        .ask(
            Command::new("journalctl")
                .arg("--boot")
                .args(matching)
                .args([
                    &format!("--lines={lines}"),
                    "--output=cat",
                    "--no-pager",
                    "--quiet",
                ]),
        )
        .map(|said| {
            String::from_utf8_lossy(&said.stdout)
                .lines()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::{Duration, Instant};

    use super::{answered, is_a_console};

    #[test]
    fn a_command_that_answers_in_time_is_heard() {
        let said = answered(
            Command::new("echo").arg("disk gone"),
            Instant::now() + Duration::from_secs(5),
        )
        .expect("echo wasn't heard");
        assert!(said.status.success(), "echo was heard failing");
        assert_eq!(
            said.stdout, b"disk gone\n",
            "echo was heard saying something else"
        );
    }

    #[test]
    fn a_command_past_its_deadline_is_stopped() {
        let started = Instant::now();
        let said = answered(
            Command::new("sleep").arg("30"),
            started + Duration::from_millis(200),
        );
        assert!(said.is_err(), "a stalled command was heard: {said:?}");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "a stalled command held it {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn a_command_that_cant_run_says_so() {
        let said = answered(
            &mut Command::new("/nonexistent/journalctl"),
            Instant::now() + Duration::from_secs(5),
        );
        assert!(
            said.as_ref().is_err_and(|why| why.contains("couldn't run")),
            "a missing program was heard: {said:?}"
        );
    }

    #[test]
    fn only_the_console_and_a_virtual_terminal_are_the_console() {
        for path in ["/dev/console", "/dev/tty1", "/dev/tty12"] {
            assert!(is_a_console(path), "{path} wasn't taken for the console");
        }
        for path in [
            "/dev/tty",
            "/dev/ttyS0",
            "/dev/pts/0",
            "/dev/null",
            "pipe:[7]",
        ] {
            assert!(!is_a_console(path), "{path} was taken for the console");
        }
    }
}
