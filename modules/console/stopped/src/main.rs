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
//! asked again, and the summary says what the others told it.

mod asking;
mod summary;

use std::fs;
use std::process::ExitCode;

use terminal_kit::flags::Format;
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
    let units: Vec<Failed> = shown
        .iter()
        .zip(abouts)
        .map(|(name, about)| Failed {
            about,
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

/// The units that failed, by name, as systemd lists them.
fn failed(budget: &Budget) -> Result<Vec<String>, Failure> {
    let unavailable = |why: String| {
        Failure::new(Exit::Unavailable, "systemd couldn't say which units failed").because(why)
    };
    let listed = budget
        .ask(
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
    let lines = format!("--lines={lines}");
    let mut args = vec!["--boot"];
    args.extend(matching);
    args.extend([lines.as_str(), "--output=cat", "--no-pager", "--quiet"]);
    budget
        .ask("journalctl", &args)
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
    use super::is_a_console;

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
