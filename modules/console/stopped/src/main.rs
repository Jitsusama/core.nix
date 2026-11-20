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
//! long, so every command it asks is held to a deadline (asking.rs): one
//! that stalls, as a journal on a failing disk can, is stopped and not
//! asked again, and the summary says what the others told it, and under
//! each unit the journal couldn't speak for, why. A journal on a failing
//! disk can skip the files it can't read and say so, which the summary
//! says once, since any unit's lines may have been in them.

mod asking;
mod fit;
mod summary;

use std::fs;
use std::process::{ExitCode, Output};

use terminal_kit::flags::Format;
use terminal_kit::layout;
use terminal_kit::{Context, Exit, Failure, Help, Ink, Pen, ink};

use asking::Budget;
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
    let shown = &names[..names.len().min(summary::SHOWN)];
    // Every unit's description is asked before any journal is, so a slow
    // journal that spends the budget still leaves each unit named as
    // systemd describes it.
    let abouts: Vec<String> = shown
        .iter()
        .map(|name| described(&budget, name).unwrap_or_else(|| name.clone()))
        .collect();
    let mut skipped = Vec::new();
    let units: Vec<Failed> = shown
        .iter()
        .zip(abouts)
        .map(|(name, about)| {
            let (said, unread) = last(name, |matching, lines| {
                journal(&budget, matching, lines, &mut skipped)
            });
            Failed {
                about,
                said,
                unread,
                name: name.clone(),
            }
        })
        .collect();
    let pen = on_the_console(context);
    context
        .say(&summary::text(
            &units,
            names.len(),
            &skipped,
            &pen,
            context.room,
            layout::rows_of_stdout(&context.environment),
        ))
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

/// The units that failed, by name, as systemd lists them. It's asked
/// with all the time there is, since without it nothing can be said.
fn failed(budget: &Budget) -> Result<Vec<String>, Failure> {
    let unavailable = |why: String| {
        Failure::new(Exit::Unavailable, "systemd couldn't say which units failed").because(why)
    };
    let listed = budget
        .ask_with_all_left(
            "systemctl",
            &["list-units", "--failed", "--plain", "--no-legend", "--full"],
        )
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
        .ask(
            "systemctl",
            &["show", "--property=Description", "--value", "--", unit],
        )
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
/// Any other of its lines only when a unit has neither. Where none of it
/// could be read, the first reason comes with it, since what followed most
/// often only says the journal wasn't asked again. `ask` reads the journal
/// as `journal` does.
fn last(
    unit: &str,
    mut ask: impl FnMut(&[&str], usize) -> Result<Vec<String>, String>,
) -> (Vec<String>, Option<String>) {
    let mut unread = None;
    let mut read = |matching: &[&str], lines: usize| {
        ask(matching, lines).unwrap_or_else(|why| {
            unread.get_or_insert(why);
            Vec::new()
        })
    };
    let own = summary::last_shown(read(&[&format!("_SYSTEMD_UNIT={unit}")], 20), 3);
    if !own.is_empty() {
        return (own, None);
    }
    let systemd = format!("UNIT={unit}");
    let mut why = read(&[&systemd, "_PID=1", PROCESS_EXITED], 1);
    why.extend(read(&[&systemd, "_PID=1", FAILED_WITH_RESULT], 1));
    if why.is_empty() {
        why = read(&[&systemd, "_PID=1"], 1);
    }
    if why.is_empty() {
        (why, unread)
    } else {
        (why, None)
    }
}

/// The last `lines` this boot's journal holds matching every one of
/// `matching`, as they were said, or why the journal couldn't be read in
/// time or at all (`heard`). Each file it skipped is added
/// to `skipped`, once however many reads skip it.
fn journal(
    budget: &Budget,
    matching: &[&str],
    lines: usize,
    skipped: &mut Vec<String>,
) -> Result<Vec<String>, String> {
    let lines = format!("--lines={lines}");
    let mut args = vec!["--boot"];
    args.extend(matching);
    args.extend([lines.as_str(), "--output=cat", "--no-pager", "--quiet"]);
    let (said, skipping) = heard(&budget.ask("journalctl", &args)?);
    for file in skipping {
        if !skipped.contains(&file) {
            skipped.push(file);
        }
    }
    said
}

/// What journalctl says of each of its files it skipped, as
/// journal-util.c words every kind: truncated, corrupted, with a feature
/// it doesn't know, past its limit of files, or any other error opening
/// it. It says them whatever `--quiet` asks, and still ends well.
const SKIPPING: &str = ", ignoring file";

/// The line journalctl adds under a file with a feature it doesn't know,
/// saying how to look closer, which says nothing of why on its own. It's
/// found anywhere in a line, since systemd's logging may put words first.
const SKIPPING_HINT: &str = "Use SYSTEMD_LOG_LEVEL=debug journalctl --file=";

/// What a journal read printed, a line each, or, where it printed nothing
/// that shows, why: the last line it said on stderr that shows, or else
/// how it ended, where that was badly. Nothing matching ends well and
/// says nothing, so it's no reason. Beside it, what it said of each file
/// it skipped, however the read went, which is no unit's reason, since
/// the file may hold only an earlier boot's lines.
fn heard(said: &Output) -> (Result<Vec<String>, String>, Vec<String>) {
    let lines = |stream: &[u8]| -> Vec<String> {
        String::from_utf8_lossy(stream)
            .lines()
            .map(str::to_owned)
            .collect()
    };
    let (skipped, words): (Vec<String>, Vec<String>) = lines(&said.stderr)
        .into_iter()
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.contains(SKIPPING_HINT))
        .partition(|line| line.contains(SKIPPING));
    let printed = lines(&said.stdout);
    if !summary::last_shown(printed.clone(), 1).is_empty() {
        return (Ok(printed), skipped);
    }
    let why = match summary::last_shown(words, 1).pop() {
        Some(why) => Err(why),
        None if !said.status.success() => Err(format!("journalctl ended with {}", said.status)),
        None => Ok(printed),
    };
    (why, skipped)
}

#[cfg(test)]
mod tests {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};

    use super::{heard, is_a_console, last};

    /// A journal read that ended with `code`, printing `stdout` and saying
    /// `stderr`.
    fn read(code: i32, stdout: &str, stderr: &str) -> Output {
        Output {
            status: ExitStatus::from_raw(code << 8),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    fn lines(said: &[&str]) -> Vec<String> {
        said.iter().map(|line| (*line).to_owned()).collect()
    }

    #[test]
    fn a_read_that_printed_lines_gives_them_however_it_ended() {
        assert_eq!(
            heard(&read(
                1,
                "disk gone\n",
                "Failed to read: Input/output error\n"
            ))
            .0,
            Ok(lines(&["disk gone"])),
            "a read that printed a line that shows lost it"
        );
    }

    #[test]
    fn a_read_that_printed_nothing_that_shows_says_why_in_its_own_words() {
        assert_eq!(
            heard(&read(
                1,
                "\u{2800}\n",
                "Failed to open files: Input/output error\n\x1b[0m\n"
            ))
            .0,
            Err("Failed to open files: Input/output error".to_owned()),
            "a failed read gave other than the last of its words that shows"
        );
        assert_eq!(
            heard(&read(1, "", "")).0,
            Err("journalctl ended with exit status: 1".to_owned()),
            "a failed read that said nothing didn't say how it ended"
        );
    }

    #[test]
    fn a_read_that_matched_nothing_is_no_reason() {
        assert_eq!(
            heard(&read(0, "", "")),
            (Ok(Vec::new()), Vec::new()),
            "nothing matching was taken for a journal that couldn't be read"
        );
    }

    #[test]
    fn a_skipped_file_is_set_aside_and_is_no_reason() {
        // An earlier boot's journal left behind by a crash is skipped on
        // every read, whatever unit it asks of, so it's no unit's reason.
        assert_eq!(
            heard(&read(
                0,
                "",
                "Journal file /j/system@a.journal~ is truncated, ignoring file.\n"
            )),
            (
                Ok(Vec::new()),
                lines(&["Journal file /j/system@a.journal~ is truncated, ignoring file."])
            ),
            "a skipped file was taken for a unit's reason, or not set aside"
        );
        assert_eq!(
            heard(&read(
                0,
                "",
                "Journal file /j/b.journal uses an unsupported feature, ignoring file.\n\
                 Use SYSTEMD_LOG_LEVEL=debug journalctl --file=/j/b.journal to see the details.\n"
            )),
            (
                Ok(Vec::new()),
                lines(&["Journal file /j/b.journal uses an unsupported feature, ignoring file."])
            ),
            "the hint under an unsupported file was taken for a reason or a file"
        );
        assert_eq!(
            heard(&read(
                1,
                "",
                "Journal file /j/c.journal corrupted, ignoring file.\n\
                 Failed to open files: Input/output error\n"
            )),
            (
                Err("Failed to open files: Input/output error".to_owned()),
                lines(&["Journal file /j/c.journal corrupted, ignoring file."])
            ),
            "a failed read that skipped a file lost its reason or the file"
        );
        assert_eq!(
            heard(&read(
                1,
                "",
                "Journal file /j/c.journal corrupted, ignoring file.\n"
            )),
            (
                Err("journalctl ended with exit status: 1".to_owned()),
                lines(&["Journal file /j/c.journal corrupted, ignoring file."])
            ),
            "a failed read that only skipped a file didn't say how it ended"
        );
        assert_eq!(
            heard(&read(
                0,
                "disk gone\n",
                "Journal file /j/c.journal corrupted, ignoring file.\n"
            )),
            (
                Ok(lines(&["disk gone"])),
                lines(&["Journal file /j/c.journal corrupted, ignoring file."])
            ),
            "a read that printed lines lost the file it skipped"
        );
    }

    #[test]
    fn every_kind_of_skipped_file_is_set_aside() {
        for said in [
            "Too many journal files (limit is at 7168) in scope, ignoring file '/j/d.journal'.",
            "An error was encountered while opening journal file or directory /j/e.journal, \
             ignoring file: Input/output error",
            "journalctl[1]: Journal file /j/f.journal is truncated, ignoring file.",
        ] {
            assert_eq!(
                heard(&read(
                    0,
                    "",
                    &format!("{said}\njournalctl[1]: {}/j/f\n", super::SKIPPING_HINT)
                )),
                (Ok(Vec::new()), lines(&[said])),
                "a skipped file or its hint was taken for a reason"
            );
        }
    }

    #[test]
    fn a_unit_with_lines_to_show_shows_them_and_no_reason() {
        let mut replies = vec![
            Err("journalctl couldn't run".to_owned()),
            Ok(lines(&[
                "home.mount: Mount process exited, code=exited, status=32",
            ])),
        ]
        .into_iter();
        assert_eq!(
            last("home.mount", |_, _| replies
                .next()
                .unwrap_or(Ok(Vec::new()))),
            (
                lines(&["home.mount: Mount process exited, code=exited, status=32"]),
                None
            ),
            "a unit with a line from systemd was given a reason too"
        );
    }

    #[test]
    fn a_unit_with_nothing_read_gives_the_first_reason() {
        let mut replies = vec![
            Err("journalctl didn't answer in time, so it was stopped".to_owned()),
            Err(
                "journalctl didn't answer an earlier ask in time, so it wasn't asked again"
                    .to_owned(),
            ),
        ]
        .into_iter();
        assert_eq!(
            last("home.mount", |_, _| replies.next().unwrap_or_else(|| Err(
                "no time was left to ask journalctl".to_owned()
            ))),
            (
                Vec::new(),
                Some("journalctl didn't answer in time, so it was stopped".to_owned())
            ),
            "a unit with nothing read gave other than the first reason"
        );
        assert_eq!(
            last("quiet.service", |_, _| Ok(Vec::new())),
            (Vec::new(), None),
            "a unit whose journal held nothing was given a reason"
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
